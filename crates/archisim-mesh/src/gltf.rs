//! glTF 2.0, as `.gltf` (JSON with its buffers in `.bin` files or `data:`
//! URIs) or `.glb` (all in one): the meshes of the default scene with the
//! transforms of their nodes, triangles (also strips and fans), the base
//! colour of each material (factor, texture and vertex colours) and its
//! transparency.

use serde_json::Value;

use crate::texture::{from_linear, tint, Texture};
use crate::{
    apply, mirrors, mul, to_ours, Builder, Error, Mat4, Material, Mesh, Options, Resources, Rgb,
    Up, IDENTITY,
};

const GLB_MAGIC: u32 = 0x4654_6C67;
const CHUNK_JSON: u32 = 0x4E4F_534A;
const CHUNK_BIN: u32 = 0x004E_4942;

fn u32_at(b: &[u8], i: usize) -> Option<u32> {
    Some(u32::from_le_bytes(b.get(i..i + 4)?.try_into().ok()?))
}

fn invalid(s: impl Into<String>) -> Error {
    Error::Invalid(s.into())
}

/// The JSON and the embedded binary chunk of a `.glb`, or the JSON of a
/// `.gltf`.
fn split(bytes: &[u8]) -> Result<(Value, Option<Vec<u8>>), Error> {
    if u32_at(bytes, 0) == Some(GLB_MAGIC) {
        let mut i = 12;
        let mut json = None;
        let mut bin = None;
        while i + 8 <= bytes.len() {
            let len = u32_at(bytes, i).unwrap_or(0) as usize;
            let kind = u32_at(bytes, i + 4).unwrap_or(0);
            let data = bytes
                .get(i + 8..i + 8 + len)
                .ok_or_else(|| invalid("GLB cortado"))?;
            match kind {
                CHUNK_JSON => json = Some(data),
                CHUNK_BIN => bin = Some(data.to_vec()),
                _ => {}
            }
            i += 8 + len;
        }
        let json = json.ok_or_else(|| invalid("GLB sin JSON"))?;
        let v = serde_json::from_slice(json).map_err(|e| invalid(format!("JSON: {e}")))?;
        Ok((v, bin))
    } else {
        let v = serde_json::from_slice(bytes).map_err(|e| invalid(format!("JSON: {e}")))?;
        Ok((v, None))
    }
}

struct Doc<'a> {
    json: Value,
    buffers: Vec<Option<Vec<u8>>>,
    files: &'a dyn Resources,
}

fn arr<'v>(v: &'v Value, key: &str) -> &'v [Value] {
    v.get(key)
        .and_then(|x| x.as_array())
        .map_or(&[], |a| a.as_slice())
}

fn idx(v: &Value, key: &str) -> Option<usize> {
    v.get(key).and_then(|x| x.as_u64()).map(|x| x as usize)
}

impl Doc<'_> {
    fn view(&self, i: usize) -> Option<(&[u8], usize)> {
        let view = arr(&self.json, "bufferViews").get(i)?;
        let buffer = self.buffers.get(idx(view, "buffer")?)?.as_ref()?;
        let offset = idx(view, "byteOffset").unwrap_or(0);
        let len = idx(view, "byteLength")?;
        let stride = idx(view, "byteStride").unwrap_or(0);
        Some((buffer.get(offset..offset + len)?, stride))
    }

    /// The values of accessor `i`, as numbers, `n` per element.
    fn accessor(&self, i: usize) -> Option<(Vec<f64>, usize)> {
        let a = arr(&self.json, "accessors").get(i)?;
        let count = idx(a, "count")?;
        let n = match a.get("type")?.as_str()? {
            "SCALAR" => 1,
            "VEC2" => 2,
            "VEC3" => 3,
            "VEC4" => 4,
            _ => return None,
        };
        let ct = idx(a, "componentType")?;
        let size = match ct {
            5120 | 5121 => 1,
            5122 | 5123 => 2,
            5125 | 5126 => 4,
            _ => return None,
        };
        let normalized = a
            .get("normalized")
            .and_then(|x| x.as_bool())
            .unwrap_or(false);
        let mut out = vec![0.0; count * n];
        let Some(view) = idx(a, "bufferView") else {
            // No data: zeros (sparse accessors are not read).
            return Some((out, n));
        };
        let (data, stride) = self.view(view)?;
        let offset = idx(a, "byteOffset").unwrap_or(0);
        let stride = if stride == 0 { size * n } else { stride };
        for e in 0..count {
            for k in 0..n {
                let at = offset + e * stride + k * size;
                let b = data.get(at..at + size)?;
                let x = match ct {
                    5120 => {
                        let v = b[0] as i8 as f64;
                        if normalized {
                            (v / 127.0).max(-1.0)
                        } else {
                            v
                        }
                    }
                    5121 => {
                        let v = b[0] as f64;
                        if normalized {
                            v / 255.0
                        } else {
                            v
                        }
                    }
                    5122 => {
                        let v = i16::from_le_bytes([b[0], b[1]]) as f64;
                        if normalized {
                            (v / 32767.0).max(-1.0)
                        } else {
                            v
                        }
                    }
                    5123 => {
                        let v = u16::from_le_bytes([b[0], b[1]]) as f64;
                        if normalized {
                            v / 65535.0
                        } else {
                            v
                        }
                    }
                    5125 => u32::from_le_bytes([b[0], b[1], b[2], b[3]]) as f64,
                    _ => f32::from_le_bytes([b[0], b[1], b[2], b[3]]) as f64,
                };
                out[e * n + k] = x;
            }
        }
        Some((out, n))
    }

    fn image(&self, texture: usize) -> Option<Texture> {
        let t = arr(&self.json, "textures").get(texture)?;
        let img = arr(&self.json, "images").get(idx(t, "source")?)?;
        let bytes = if let Some(view) = idx(img, "bufferView") {
            self.view(view)?.0.to_vec()
        } else {
            let uri = img.get("uri")?.as_str()?;
            crate::base64::data_uri(uri).or_else(|| self.files.get(uri))?
        };
        Texture::decode(&bytes)
    }
}

fn node_matrix(n: &Value) -> Mat4 {
    if let Some(m) = n.get("matrix").and_then(|m| m.as_array()) {
        if m.len() == 16 {
            let mut out = IDENTITY;
            for (i, v) in m.iter().enumerate() {
                out[i / 4][i % 4] = v.as_f64().unwrap_or(0.0);
            }
            return out;
        }
    }
    let get = |k: &str, d: &[f64]| -> Vec<f64> {
        n.get(k)
            .and_then(|v| v.as_array())
            .map(|a| a.iter().map(|x| x.as_f64().unwrap_or(0.0)).collect())
            .unwrap_or_else(|| d.to_vec())
    };
    let t = get("translation", &[0.0, 0.0, 0.0]);
    let r = get("rotation", &[0.0, 0.0, 0.0, 1.0]);
    let s = get("scale", &[1.0, 1.0, 1.0]);
    let (x, y, z, w) = (r[0], r[1], r[2], r[3]);
    let rot = [
        [
            1.0 - 2.0 * (y * y + z * z),
            2.0 * (x * y + z * w),
            2.0 * (x * z - y * w),
            0.0,
        ],
        [
            2.0 * (x * y - z * w),
            1.0 - 2.0 * (x * x + z * z),
            2.0 * (y * z + x * w),
            0.0,
        ],
        [
            2.0 * (x * z + y * w),
            2.0 * (y * z - x * w),
            1.0 - 2.0 * (x * x + y * y),
            0.0,
        ],
        [0.0, 0.0, 0.0, 1.0],
    ];
    let mut m = rot;
    for (col, k) in m.iter_mut().zip(&s).take(3) {
        for v in col.iter_mut().take(3) {
            *v *= k;
        }
    }
    m[3] = [t[0], t[1], t[2], 1.0];
    m
}

pub fn read(bytes: &[u8], files: &dyn Resources, options: Options) -> Result<Mesh, Error> {
    let (json, bin) = split(bytes)?;
    let mut buffers = Vec::new();
    for (i, b) in arr(&json, "buffers").iter().enumerate() {
        let data = match b.get("uri").and_then(|u| u.as_str()) {
            Some(uri) => crate::base64::data_uri(uri).or_else(|| files.get(uri)),
            // The GLB's own chunk is the first buffer without a URI.
            None if i == 0 => bin.clone(),
            None => None,
        };
        buffers.push(data);
    }
    let doc = Doc {
        json,
        buffers,
        files,
    };
    let up = match options.up {
        Up::Auto => Up::Y,
        u => u,
    };
    let unit = options.unit.unwrap_or(1.0);
    let mut b = Builder::default();
    let mut textures: Vec<(usize, Option<Texture>)> = Vec::new();
    // The nodes of the scene, depth first, with their transforms.
    let scene = idx(&doc.json, "scene").unwrap_or(0);
    let roots: Vec<usize> = match arr(&doc.json, "scenes").get(scene) {
        Some(s) => arr(s, "nodes")
            .iter()
            .filter_map(|n| n.as_u64())
            .map(|n| n as usize)
            .collect(),
        None => (0..arr(&doc.json, "nodes").len()).collect(),
    };
    let nodes = arr(&doc.json, "nodes");
    let mut stack: Vec<(usize, Mat4, usize)> =
        roots.into_iter().map(|n| (n, IDENTITY, 0)).collect();
    let mut placed = Vec::new();
    while let Some((n, parent, depth)) = stack.pop() {
        let Some(node) = nodes.get(n) else { continue };
        if depth > 64 {
            continue;
        }
        let m = mul(&parent, &node_matrix(node));
        if let Some(mesh) = idx(node, "mesh") {
            placed.push((mesh, m));
        }
        for c in arr(node, "children").iter().filter_map(|c| c.as_u64()) {
            stack.push((c as usize, m, depth + 1));
        }
    }
    if placed.is_empty() && nodes.is_empty() {
        // Meshes without nodes: as they are.
        placed = (0..arr(&doc.json, "meshes").len())
            .map(|i| (i, IDENTITY))
            .collect();
    }
    let materials = arr(&doc.json, "materials").to_vec();
    for (mesh, m) in placed {
        let Some(mesh) = arr(&doc.json, "meshes").get(mesh) else {
            continue;
        };
        let flip = mirrors(&m);
        for prim in arr(mesh, "primitives") {
            let mode = idx(prim, "mode").unwrap_or(4);
            if !(4..=6).contains(&mode) {
                continue; // points and lines
            }
            let attrs = prim.get("attributes");
            let Some((pos, 3)) = attrs
                .and_then(|a| idx(a, "POSITION"))
                .and_then(|i| doc.accessor(i))
            else {
                continue;
            };
            let count = pos.len() / 3;
            let uv = attrs
                .and_then(|a| idx(a, "TEXCOORD_0"))
                .and_then(|i| doc.accessor(i));
            let vcol = attrs
                .and_then(|a| idx(a, "COLOR_0"))
                .and_then(|i| doc.accessor(i));
            let mat_index = idx(prim, "material");
            let mat = mat_index.and_then(|i| materials.get(i));
            let pbr = mat.and_then(|m| m.get("pbrMetallicRoughness"));
            let factor = pbr
                .and_then(|p| p.get("baseColorFactor"))
                .and_then(|f| f.as_array())
                .map(|f| {
                    let g = |i: usize| f.get(i).and_then(|x| x.as_f64()).unwrap_or(1.0);
                    [g(0), g(1), g(2), g(3)]
                })
                .unwrap_or([1.0; 4]);
            let tex_index = pbr
                .and_then(|p| p.get("baseColorTexture"))
                .and_then(|t| idx(t, "index"));
            if let Some(t) = tex_index {
                if !textures.iter().any(|(i, _)| *i == t) {
                    textures.push((t, doc.image(t)));
                }
            }
            let tex = tex_index
                .and_then(|t| textures.iter().find(|(i, _)| *i == t))
                .and_then(|(_, x)| x.as_ref());
            let tint_rgb = from_linear([factor[0], factor[1], factor[2]]);
            let blend = mat
                .and_then(|m| m.get("alphaMode"))
                .and_then(|a| a.as_str())
                .is_some_and(|a| a == "BLEND");
            let alpha = if blend { factor[3] as f32 } else { 1.0 };
            let name = mat
                .and_then(|m| m.get("name"))
                .and_then(|n| n.as_str())
                .map(String::from)
                .unwrap_or_else(|| match mat_index {
                    Some(i) => format!("Material {}", i + 1),
                    None => "Predeterminado".into(),
                });
            // With vertex colours and no texture: their average (what the
            // material looks like from afar).
            let vertex_average =
                vcol.as_ref()
                    .filter(|(v, n)| *n >= 3 && !v.is_empty())
                    .map(|(v, n)| {
                        let count = v.len() / n;
                        let sum = (0..count).fold([0.0; 3], |a, i| {
                            [a[0] + v[n * i], a[1] + v[n * i + 1], a[2] + v[n * i + 2]]
                        });
                        from_linear(sum.map(|x| x / count as f64))
                    });
            let base = match (tex, vertex_average) {
                (Some(t), _) => tint(t.average(), tint_rgb),
                (None, Some(v)) => tint(v, tint_rgb),
                (None, None) => tint_rgb,
            };
            let material = b.material(&name, || Material {
                name: String::new(),
                color: base,
                alpha,
                texture: tex_index.map(|t| format!("textura {t}")),
            });
            let first = b.mesh.positions.len() as u32;
            for i in 0..count {
                let p = apply(&m, [pos[3 * i], pos[3 * i + 1], pos[3 * i + 2]]);
                let mut c: Rgb = base;
                if let (Some(t), Some((uv, 2))) = (tex, uv.as_ref()) {
                    c = tint(
                        t.sample(uv[2 * i] as f32, uv[2 * i + 1] as f32, false),
                        tint_rgb,
                    );
                }
                if let Some((vc, n)) = vcol.as_ref() {
                    if *n >= 3 {
                        let v = from_linear([vc[n * i], vc[n * i + 1], vc[n * i + 2]]);
                        c = match tex {
                            Some(_) => tint(c, v),
                            None => tint(v, tint_rgb),
                        };
                    }
                }
                b.vertex(to_ours(up, unit, p), c);
            }
            let indices: Vec<u32> = match idx(prim, "indices").and_then(|i| doc.accessor(i)) {
                Some((ix, 1)) => ix.into_iter().map(|x| x as u32).collect(),
                _ => (0..count as u32).collect(),
            };
            let tris: Vec<[u32; 3]> = match mode {
                4 => indices
                    .chunks_exact(3)
                    .map(|t| [t[0], t[1], t[2]])
                    .collect(),
                5 => (0..indices.len().saturating_sub(2))
                    .map(|k| {
                        if k % 2 == 0 {
                            [indices[k], indices[k + 1], indices[k + 2]]
                        } else {
                            [indices[k + 1], indices[k], indices[k + 2]]
                        }
                    })
                    .collect(),
                _ => (1..indices.len().saturating_sub(1))
                    .map(|k| [indices[0], indices[k], indices[k + 1]])
                    .collect(),
            };
            for t in tris {
                if t.iter().any(|&i| i as usize >= count) {
                    return Err(invalid("índice fuera de los vértices"));
                }
                let t = t.map(|i| first + i);
                b.triangle(material, if flip { [t[0], t[2], t[1]] } else { t });
            }
        }
    }
    Ok(b.mesh)
}
