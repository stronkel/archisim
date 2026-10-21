//! Wavefront OBJ with its MTL: positions, texture coordinates, faces of
//! any number of sides (fanned into triangles), `usemtl` materials with
//! their diffuse colour (`Kd`), transparency (`d`, `Tr`) and texture
//! (`map_Kd`).

use std::collections::HashMap;

use crate::texture::{from_unit, tint, Texture};
use crate::{to_ours, Builder, Error, Material, Mesh, Options, Resources, Rgb, Up};

#[derive(Default, Clone)]
struct Mtl {
    color: Option<[f64; 3]>,
    alpha: f32,
    map: Option<String>,
}

fn parse_mtl(text: &str) -> HashMap<String, Mtl> {
    let mut out: HashMap<String, Mtl> = HashMap::new();
    let mut current: Option<String> = None;
    for line in text.lines() {
        let line = line.trim();
        let mut it = line.split_whitespace();
        let Some(key) = it.next() else { continue };
        let rest: Vec<&str> = it.collect();
        match key {
            "newmtl" => {
                let name = rest.join(" ");
                out.insert(
                    name.clone(),
                    Mtl {
                        alpha: 1.0,
                        ..Default::default()
                    },
                );
                current = Some(name);
            }
            _ => {
                let Some(m) = current.as_ref().and_then(|c| out.get_mut(c)) else {
                    continue;
                };
                let nums: Vec<f64> = rest.iter().filter_map(|s| s.parse().ok()).collect();
                match key {
                    "Kd" if nums.len() >= 3 => m.color = Some([nums[0], nums[1], nums[2]]),
                    "d" if !nums.is_empty() => m.alpha = nums[0] as f32,
                    "Tr" if !nums.is_empty() => m.alpha = 1.0 - nums[0] as f32,
                    "map_Kd" => {
                        // Options (-s 1 1 1, -o …) come before the file.
                        let file = rest
                            .iter()
                            .rposition(|s| !s.starts_with('-') && s.parse::<f64>().is_err())
                            .map(|i| rest[i..].join(" "));
                        m.map = file;
                    }
                    _ => {}
                }
            }
        }
    }
    out
}

/// An index of OBJ (1-based, or negative from the end) into a list of `n`.
fn index(s: &str, n: usize) -> Option<usize> {
    let i: i64 = s.parse().ok()?;
    if i > 0 && (i as usize) <= n {
        Some(i as usize - 1)
    } else if i < 0 && (-i as usize) <= n {
        Some(n - (-i) as usize)
    } else {
        None
    }
}

pub fn read(bytes: &[u8], files: &dyn Resources, options: Options) -> Result<Mesh, Error> {
    let text = String::from_utf8_lossy(bytes);
    let up = match options.up {
        Up::Auto => Up::Y,
        u => u,
    };
    let unit = options.unit.unwrap_or(1.0);
    let mut v: Vec<[f64; 3]> = Vec::new();
    let mut vc: Vec<Option<Rgb>> = Vec::new();
    let mut vt: Vec<[f32; 2]> = Vec::new();
    let mut mtls: HashMap<String, Mtl> = HashMap::new();
    let mut textures: HashMap<String, Option<Texture>> = HashMap::new();
    let mut b = Builder::default();
    let mut current = "Predeterminado".to_string();
    // Vertices already made, by (position, uv, material).
    let mut made: HashMap<(usize, Option<usize>, usize), u32> = HashMap::new();
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let (key, rest) = line.split_once(char::is_whitespace).unwrap_or((line, ""));
        let rest = rest.trim();
        match key {
            "v" => {
                let n: Vec<f64> = rest
                    .split_whitespace()
                    .filter_map(|s| s.parse().ok())
                    .collect();
                if n.len() < 3 {
                    return Err(Error::Invalid(format!("vértice «{line}»")));
                }
                v.push([n[0], n[1], n[2]]);
                vc.push((n.len() >= 6).then(|| from_unit([n[3], n[4], n[5]])));
            }
            "vt" => {
                let n: Vec<f32> = rest
                    .split_whitespace()
                    .filter_map(|s| s.parse().ok())
                    .collect();
                vt.push([
                    n.first().copied().unwrap_or(0.0),
                    n.get(1).copied().unwrap_or(0.0),
                ]);
            }
            "mtllib" => {
                if let Some(data) = files.get(rest) {
                    mtls.extend(parse_mtl(&String::from_utf8_lossy(&data)));
                }
            }
            "usemtl" => current = rest.to_string(),
            "f" => {
                let mtl = mtls.get(&current).cloned().unwrap_or(Mtl {
                    alpha: 1.0,
                    ..Default::default()
                });
                if let Some(map) = &mtl.map {
                    textures
                        .entry(map.clone())
                        .or_insert_with(|| files.get(map).and_then(|d| Texture::decode(&d)));
                }
                let tex = mtl
                    .map
                    .as_ref()
                    .and_then(|m| textures.get(m))
                    .and_then(|t| t.as_ref());
                // With a texture, its own colours (Kd is often a leftover
                // default grey that most programs ignore then).
                let factor = [255, 255, 255];
                let base = match tex {
                    Some(t) => tint(t.average(), factor),
                    None => mtl.color.map_or([200, 200, 200], from_unit),
                };
                let material = b.material(&current, || Material {
                    name: String::new(),
                    color: base,
                    alpha: mtl.alpha,
                    texture: mtl.map.clone(),
                });
                let mut corners = Vec::new();
                for c in rest.split_whitespace() {
                    let mut parts = c.split('/');
                    let Some(pi) = parts.next().and_then(|s| index(s, v.len())) else {
                        return Err(Error::Invalid(format!("cara «{line}»")));
                    };
                    let ti = parts.next().and_then(|s| index(s, vt.len()));
                    let key = (pi, ti, material);
                    let id = match made.get(&key) {
                        Some(id) => *id,
                        None => {
                            let color = match (tex, ti) {
                                (Some(t), Some(ti)) => {
                                    tint(t.sample(vt[ti][0], vt[ti][1], true), factor)
                                }
                                _ => vc[pi].unwrap_or(base),
                            };
                            let id = b.vertex(to_ours(up, unit, v[pi]), color);
                            made.insert(key, id);
                            id
                        }
                    };
                    corners.push(id);
                }
                for k in 1..corners.len().saturating_sub(1) {
                    b.triangle(material, [corners[0], corners[k], corners[k + 1]]);
                }
            }
            _ => {}
        }
    }
    Ok(b.mesh)
}
