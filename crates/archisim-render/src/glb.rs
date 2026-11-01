//! The whole project as a `.glb` (glTF 2.0 in one file), to take it to D5,
//! Twinmotion, Blender or any program that reads glTF (C4).
//!
//! The same solids as the 3D view and the IFC, one node per element
//! (named by what it is and its id) and in it one part per material. The
//! materials keep their names and colours; those with a texture carry it
//! as a picture (made by [`crate::textures`] in the material's colour)
//! with its coordinates on each face, as the 3D view lays it; glass is
//! see-through. Metres, `y` up, as glTF wants.

use std::collections::BTreeMap;

use archisim_core::{MaterialId, Model};
use archisim_geom::solid::{painted_material, solid_vertex_colors, SolidOwner};

use crate::textures;

/// How a part is seen: its material (or the paint over it), with the
/// colours of its vertices when it comes from another program.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct Look {
    material: MaterialId,
    vertex_colours: bool,
}

/// The triangles of a part: positions, flat normals, texture points and
/// vertex colours, three a triangle.
#[derive(Default)]
struct Part {
    pos: Vec<[f32; 3]>,
    normal: Vec<[f32; 3]>,
    uv: Vec<[f32; 2]>,
    colour: Vec<[f32; 3]>,
    /// The corners of each triangle (once indexed).
    index: Vec<u32>,
}

/// The part with each different corner once and its triangles as
/// indices.
fn indexed(part: &Part) -> Part {
    let mut out = Part::default();
    let mut seen: std::collections::HashMap<[i32; 11], u32> = Default::default();
    let q = |v: f32, k: f32| (v * k).round() as i32;
    for i in 0..part.pos.len() {
        let (p, n, uv, c) = (part.pos[i], part.normal[i], part.uv[i], part.colour[i]);
        let key = [
            q(p[0], 1e4),
            q(p[1], 1e4),
            q(p[2], 1e4),
            q(n[0], 1e3),
            q(n[1], 1e3),
            q(n[2], 1e3),
            q(uv[0], 1e4),
            q(uv[1], 1e4),
            q(c[0], 255.0),
            q(c[1], 255.0),
            q(c[2], 255.0),
        ];
        let at = *seen.entry(key).or_insert_with(|| {
            out.pos.push(p);
            out.normal.push(n);
            out.uv.push(uv);
            out.colour.push(c);
            (out.pos.len() - 1) as u32
        });
        out.index.push(at);
    }
    out
}

fn linear(c: u8) -> f32 {
    let s = c as f32 / 255.0;
    if s <= 0.04045 {
        s / 12.92
    } else {
        ((s + 0.055) / 1.055).powf(2.4)
    }
}

/// glTF is `y` up: our (x, y, z) is its (x, z, −y).
fn gl(p: [f64; 3]) -> [f32; 3] {
    [p[0] as f32, p[2] as f32, -p[1] as f32]
}

/// The name of an element in the file.
fn element_name(model: &Model, owner: SolidOwner) -> String {
    let what = match owner {
        SolidOwner::Wall(_) => "Muro",
        SolidOwner::Facade(_) => "Fachada",
        SolidOwner::Slab(_) => "Forjado",
        SolidOwner::Opening(_) => "Carpintería",
        SolidOwner::Column(_) => "Pilar",
        SolidOwner::Beam(_) => "Viga",
        SolidOwner::Roof(_) => "Cubierta",
        SolidOwner::Dormer(..) => "Buhardilla",
        SolidOwner::Stair(_) => "Escalera",
        SolidOwner::Terrain(_) => "Terreno",
        SolidOwner::Growth(_) => "Hierba",
        SolidOwner::CurtainWall(_) => "Muro cortina",
        SolidOwner::Railing(_) => "Barandilla",
        SolidOwner::Object(id) => {
            return model
                .object(id)
                .and_then(|o| model.object_type(o.object_type))
                .map_or_else(
                    || format!("Objeto {}", id.0),
                    |t| format!("{} {}", t.name, id.0),
                );
        }
        SolidOwner::Footing(_) => "Cimentación",
        SolidOwner::Pool(_) => "Piscina",
        SolidOwner::Elevator(_) => "Ascensor",
        SolidOwner::Mass(_) => "Volumen",
        SolidOwner::Road(_) => "Vial",
        SolidOwner::Marking(_) => "Marca vial",
        SolidOwner::Decal(_) => "Pegatina",
        SolidOwner::Run(_) => "Tubería",
        SolidOwner::Device(_) => "Aparato",
    };
    format!("{what} {}", owner.id())
}

/// The project as a `.glb`.
pub fn project_glb(model: &Model) -> Vec<u8> {
    // Element by element, in the order the solids come.
    let mut nodes: Vec<(String, BTreeMap<Look, Part>)> = Vec::new();
    let mut index: BTreeMap<(String, u64), usize> = BTreeMap::new();
    // The furniture ArchiSim makes: each model once, placed by every
    // object of it (as the 3D view does): a building with hundreds of
    // chairs carries one chair.
    let mut skip = std::collections::HashSet::new();
    let mut shared: Vec<(String, BTreeMap<Look, Part>)> = Vec::new();
    let mut shared_index: BTreeMap<(usize, Vec<MaterialId>), usize> = BTreeMap::new();
    let mut placed: Vec<(String, usize, [f32; 3], f64)> = Vec::new();
    for o in model.objects() {
        let Some(inst) = archisim_geom::solid::object_instance(model, o.id) else {
            continue;
        };
        skip.insert(o.id);
        let key = (
            std::sync::Arc::as_ptr(&inst.mesh) as usize,
            inst.parts.iter().map(|p| p.0).collect::<Vec<_>>(),
        );
        let mesh = *shared_index.entry(key).or_insert_with(|| {
            let name = model
                .object_type(o.object_type)
                .map_or_else(|| "Objeto".to_string(), |t| t.name.clone());
            let mut parts: BTreeMap<Look, Part> = BTreeMap::new();
            for (material, tris) in &inst.parts {
                let solid = archisim_geom::solid::Solid {
                    owner: SolidOwner::Object(o.id),
                    material: *material,
                    triangles: tris
                        .iter()
                        .map(|t| t.map(|i| inst.mesh.positions[i as usize].map(|c| c as f64)))
                        .collect(),
                };
                add_triangles(model, &solid, false, &mut parts);
            }
            shared.push((name, parts));
            shared.len() - 1
        });
        placed.push((
            element_name(model, SolidOwner::Object(o.id)),
            mesh,
            gl(inst.origin),
            inst.rotation,
        ));
    }
    // The painted zones of the faces too, in their own material.
    let solids = archisim_geom::solid::model_solids_except(model, &skip);
    let zones = archisim_geom::solid::face_zone_solids(model, &solids);
    let first_zone = solids.len();
    for (i, s) in solids.into_iter().chain(zones).enumerate() {
        let zone = i >= first_zone;
        let name = element_name(model, s.owner);
        let key = (
            format!("{:?}", std::mem::discriminant(&s.owner)),
            s.owner.id(),
        );
        let at = *index.entry(key).or_insert_with(|| {
            nodes.push((name, BTreeMap::new()));
            nodes.len() - 1
        });
        add_triangles(model, &s, zone, &mut nodes[at].1);
    }
    write(model, &nodes, &shared, &placed)
}

/// Adds the triangles of a solid to the parts of a mesh, by look.
fn add_triangles(
    model: &Model,
    s: &archisim_geom::solid::Solid,
    zone: bool,
    parts: &mut BTreeMap<Look, Part>,
) {
    {
        let corners = solid_vertex_colors(model, s);
        let is_object = matches!(s.owner, SolidOwner::Object(_));
        for (i, t) in s.triangles.iter().enumerate() {
            let material = if zone {
                s.material
            } else {
                painted_material(model, s, t).unwrap_or(s.material)
            };
            let vc = corners.as_ref().and_then(|c| c.get(i));
            let look = Look {
                material,
                vertex_colours: vc.is_some(),
            };
            let e1 = [t[1][0] - t[0][0], t[1][1] - t[0][1], t[1][2] - t[0][2]];
            let e2 = [t[2][0] - t[0][0], t[2][1] - t[0][1], t[2][2] - t[0][2]];
            let n = [
                e1[1] * e2[2] - e1[2] * e2[1],
                e1[2] * e2[0] - e1[0] * e2[2],
                e1[0] * e2[1] - e1[1] * e2[0],
            ];
            let len = (n[0] * n[0] + n[1] * n[1] + n[2] * n[2]).sqrt();
            if len < 1e-12 {
                continue;
            }
            let n = n.map(|v| v / len);
            let part = parts.entry(look).or_default();
            // The pieces of a façade are each a brick: no brick texture.
            let texture = if matches!(s.owner, SolidOwner::Facade(_)) && material == s.material {
                None
            } else {
                crate::texture_of(model, material, is_object)
            };
            for (k, p) in t.iter().enumerate() {
                part.pos.push(gl(*p));
                part.normal.push(gl(n));
                let uv = match texture {
                    Some((kind, scale)) => {
                        let q = textures::texture_point(*p, n, scale);
                        let per = textures::period(kind);
                        [(q[0] / per) as f32, (-q[1] / per) as f32]
                    }
                    None => [0.0, 0.0],
                };
                part.uv.push(uv);
                let c = vc.map_or([1.0; 3], |c| c[k].map(linear));
                part.colour.push(c);
            }
        }
    }
}

/// Writes the nodes as a `.glb`: the elements each with its own mesh, and
/// the shared meshes placed (name, mesh, where, turned about the
/// vertical).
fn write(
    model: &Model,
    nodes: &[(String, BTreeMap<Look, Part>)],
    shared: &[(String, BTreeMap<Look, Part>)],
    placed: &[(String, usize, [f32; 3], f64)],
) -> Vec<u8> {
    use serde_json::json;
    let mut bin: Vec<u8> = Vec::new();
    let pad = |b: &mut Vec<u8>| {
        while !b.len().is_multiple_of(4) {
            b.push(0);
        }
    };
    let mut views = Vec::new();
    let mut accessors = Vec::new();
    let mut meshes = Vec::new();
    let mut gl_nodes = Vec::new();
    let mut materials = Vec::new();
    let mut images = Vec::new();
    let mut gl_textures = Vec::new();
    let mut material_index: BTreeMap<Look, usize> = BTreeMap::new();
    // A float attribute: its view and accessor.
    let attribute = |bin: &mut Vec<u8>,
                     views: &mut Vec<serde_json::Value>,
                     accessors: &mut Vec<serde_json::Value>,
                     data: &[f32],
                     width: usize,
                     bounds: bool| {
        let off = bin.len();
        for v in data {
            bin.extend(v.to_le_bytes());
        }
        views.push(
            json!({"buffer": 0, "byteOffset": off, "byteLength": bin.len() - off, "target": 34962}),
        );
        pad(bin);
        let kind = match width {
            2 => "VEC2",
            _ => "VEC3",
        };
        let mut a = json!({"bufferView": views.len() - 1, "componentType": 5126,
            "count": data.len() / width, "type": kind});
        if bounds {
            let mut lo = vec![f32::INFINITY; width];
            let mut hi = vec![f32::NEG_INFINITY; width];
            for c in data.chunks(width) {
                for k in 0..width {
                    lo[k] = lo[k].min(c[k]);
                    hi[k] = hi[k].max(c[k]);
                }
            }
            a["min"] = json!(lo);
            a["max"] = json!(hi);
        }
        accessors.push(a);
        accessors.len() - 1
    };
    // Where each shared mesh went (None: empty).
    let mut shared_mesh: Vec<Option<usize>> = Vec::new();
    for (k, (name, parts)) in nodes.iter().chain(shared).enumerate() {
        let is_shared = k >= nodes.len();
        let mut primitives = Vec::new();
        for (look, part) in parts {
            if part.pos.is_empty() {
                continue;
            }
            // Corners shared by the triangles of a face (same point,
            // normal, texture point and colour) are written once.
            let part = &indexed(part);
            let flat = |v: &[[f32; 3]]| v.iter().flatten().copied().collect::<Vec<f32>>();
            let pos = attribute(
                &mut bin,
                &mut views,
                &mut accessors,
                &flat(&part.pos),
                3,
                true,
            );
            let nor = attribute(
                &mut bin,
                &mut views,
                &mut accessors,
                &flat(&part.normal),
                3,
                false,
            );
            let off = bin.len();
            for i in &part.index {
                bin.extend(i.to_le_bytes());
            }
            views.push(json!({"buffer": 0, "byteOffset": off, "byteLength": bin.len() - off, "target": 34963}));
            pad(&mut bin);
            accessors.push(json!({"bufferView": views.len() - 1, "componentType": 5125,
                "count": part.index.len(), "type": "SCALAR"}));
            let indices = accessors.len() - 1;
            let mut attrs = json!({"POSITION": pos, "NORMAL": nor});
            let mat = *material_index.entry(*look).or_insert_with(|| {
                let m = material_json(
                    model,
                    *look,
                    &mut images,
                    &mut gl_textures,
                    &mut bin,
                    &mut views,
                );
                materials.push(m);
                materials.len() - 1
            });
            if materials[mat]
                .pointer("/pbrMetallicRoughness/baseColorTexture")
                .is_some()
            {
                let uv: Vec<f32> = part.uv.iter().flatten().copied().collect();
                attrs["TEXCOORD_0"] = json!(attribute(
                    &mut bin,
                    &mut views,
                    &mut accessors,
                    &uv,
                    2,
                    false
                ));
            }
            if look.vertex_colours {
                attrs["COLOR_0"] = json!(attribute(
                    &mut bin,
                    &mut views,
                    &mut accessors,
                    &flat(&part.colour),
                    3,
                    false
                ));
            }
            primitives.push(json!({"attributes": attrs, "indices": indices, "material": mat}));
        }
        if primitives.is_empty() {
            if is_shared {
                shared_mesh.push(None);
            }
            continue;
        }
        meshes.push(json!({"name": name, "primitives": primitives}));
        if is_shared {
            shared_mesh.push(Some(meshes.len() - 1));
        } else {
            gl_nodes.push(json!({"name": name, "mesh": meshes.len() - 1}));
        }
    }
    for (name, mesh, at, turn) in placed {
        let Some(Some(mesh)) = shared_mesh.get(*mesh) else {
            continue;
        };
        // A turn about our z is one about glTF's y.
        let h = (*turn / 2.0) as f32;
        gl_nodes.push(json!({"name": name, "mesh": mesh, "translation": at,
            "rotation": [0.0, h.sin(), 0.0, h.cos()]}));
    }
    let scene_nodes: Vec<usize> = (0..gl_nodes.len()).collect();
    let mut doc = json!({
        "asset": {"version": "2.0", "generator": "ArchiSim"},
        "scene": 0,
        "scenes": [{"name": model.project_name, "nodes": scene_nodes}],
        "nodes": gl_nodes,
        "meshes": meshes,
        "materials": materials,
        "accessors": accessors,
        "bufferViews": views,
        "buffers": [{"byteLength": bin.len()}],
    });
    if !images.is_empty() {
        doc["images"] = json!(images);
        doc["textures"] = json!(gl_textures);
        doc["samplers"] =
            json!([{"magFilter": 9729, "minFilter": 9987, "wrapS": 10497, "wrapT": 10497}]);
    }
    let mut text = serde_json::to_vec(&doc).unwrap_or_default();
    while !text.len().is_multiple_of(4) {
        text.push(b' ');
    }
    pad(&mut bin);
    let total = 12 + 8 + text.len() + 8 + bin.len();
    let mut out = Vec::with_capacity(total);
    out.extend(b"glTF");
    out.extend(2u32.to_le_bytes());
    out.extend((total as u32).to_le_bytes());
    out.extend((text.len() as u32).to_le_bytes());
    out.extend(b"JSON");
    out.extend(&text);
    out.extend((bin.len() as u32).to_le_bytes());
    out.extend(b"BIN\0");
    out.extend(&bin);
    out
}

/// A material of the file: its name and colour, its texture as a picture
/// in that colour, glass see-through.
fn material_json(
    model: &Model,
    look: Look,
    images: &mut Vec<serde_json::Value>,
    textures_out: &mut Vec<serde_json::Value>,
    bin: &mut Vec<u8>,
    views: &mut Vec<serde_json::Value>,
) -> serde_json::Value {
    use serde_json::json;
    let m = model.material(look.material);
    let name = m.map_or_else(|| "Material".to_string(), |m| m.name.clone());
    let glass = m.is_some_and(|m| m.category == "glass");
    let water = m.is_some_and(|m| m.category == "water");
    let rgb = model.surface_color(look.material);
    let mut colour = [linear(rgb[0]), linear(rgb[1]), linear(rgb[2]), 1.0];
    if look.vertex_colours {
        colour = [1.0, 1.0, 1.0, 1.0];
    }
    let mut pbr = json!({"metallicFactor": 0.0, "roughnessFactor": 0.85});
    let metal = m.is_some_and(|m| m.category == "metal");
    if metal {
        pbr["metallicFactor"] = json!(0.6);
        pbr["roughnessFactor"] = json!(0.4);
    }
    let mut out = json!({"name": name, "doubleSided": false});
    if water {
        // See-through, so the floor of a pool shows.
        colour[3] = 0.55;
        pbr["roughnessFactor"] = json!(0.05);
        out["alphaMode"] = json!("BLEND");
    }
    if glass {
        colour[3] = 0.25;
        pbr["roughnessFactor"] = json!(0.05);
        out["alphaMode"] = json!("BLEND");
    }
    let texture = (!glass && !look.vertex_colours)
        .then(|| crate::texture_of(model, look.material, true))
        .flatten();
    match texture {
        Some((kind, _)) => {
            let png = tile_png(kind, rgb);
            let off = bin.len();
            bin.extend(&png);
            views.push(json!({"buffer": 0, "byteOffset": off, "byteLength": png.len()}));
            while !bin.len().is_multiple_of(4) {
                bin.push(0);
            }
            images.push(json!({"bufferView": views.len() - 1, "mimeType": "image/png", "name": format!("{name} ({})", kind.label())}));
            textures_out.push(json!({"source": images.len() - 1, "sampler": 0}));
            pbr["baseColorTexture"] = json!({"index": textures_out.len() - 1});
            pbr["baseColorFactor"] = json!([1.0, 1.0, 1.0, colour[3]]);
        }
        None => pbr["baseColorFactor"] = json!(colour),
    }
    out["pbrMetallicRoughness"] = pbr;
    out
}

/// A texture's picture in a colour, as a PNG (rows from the top).
pub fn tile_png(kind: archisim_core::TextureKind, rgb: [u8; 3]) -> Vec<u8> {
    let t = textures::tile(kind);
    // Half the size of the 3D view's (four pixels in one): lighter files.
    let big = t.size;
    let n = (big / 2).max(1);
    let base = rgb.map(|c| c as f64 / 255.0);
    let mut pixels = vec![0u8; n * n * 3];
    for j in 0..n {
        for i in 0..n {
            let mut v = [0.0; 3];
            for (di, dj) in [(0, 0), (1, 0), (0, 1), (1, 1)] {
                let o = ((2 * j + dj) * big + 2 * i + di) * 4;
                let a = t.rgba[o + 3] as f64 / 255.0 * textures::FACTOR as f64;
                for k in 0..3 {
                    v[k] += (base[k] * a + t.rgba[o + k] as f64 / 255.0) / 4.0;
                }
            }
            let row = n - 1 - j;
            for k in 0..3 {
                pixels[(row * n + i) * 3 + k] = (v[k].clamp(0.0, 1.0) * 255.0).round() as u8;
            }
        }
    }
    let mut out = Vec::new();
    {
        let mut e = png::Encoder::new(&mut out, n as u32, n as u32);
        e.set_color(png::ColorType::Rgb);
        e.set_depth(png::BitDepth::Eight);
        if let Ok(mut w) = e.write_header() {
            let _ = w.write_image_data(&pixels);
        }
    }
    out
}
