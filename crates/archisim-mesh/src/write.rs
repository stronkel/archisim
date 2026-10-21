//! Writing a mesh as a binary glTF (`.glb`): positions, the colour of
//! each vertex and one primitive per material with its transparency. What
//! ArchiSim keeps of a model (it has no textures, only their colours at
//! the vertices) goes to the library this way, and any program that reads
//! glTF opens it.

use crate::Mesh;

/// sRGB 0–255 to glTF's linear 0–1.
fn linear(c: u8) -> f64 {
    let s = c as f64 / 255.0;
    if s <= 0.040_45 {
        s / 12.92
    } else {
        ((s + 0.055) / 1.055).powf(2.4)
    }
}

/// The mesh as a `.glb`, `y` up as glTF wants.
pub fn write_glb(mesh: &Mesh, name: &str) -> Vec<u8> {
    let mut bin: Vec<u8> = Vec::new();
    let pad = |b: &mut Vec<u8>| {
        while !b.len().is_multiple_of(4) {
            b.push(0);
        }
    };
    let mut views = Vec::new();
    let mut accessors = Vec::new();
    let mut primitives = Vec::new();
    // Each material with its own vertices.
    for g in &mesh.groups {
        let mut index = std::collections::HashMap::new();
        let mut used: Vec<u32> = Vec::new();
        let tris: Vec<[u32; 3]> = g
            .triangles
            .iter()
            .map(|t| {
                t.map(|i| {
                    *index.entry(i).or_insert_with(|| {
                        used.push(i);
                        (used.len() - 1) as u32
                    })
                })
            })
            .collect();
        // Positions (ours z up → glTF y up) and their bounds.
        let mut lo = [f32::INFINITY; 3];
        let mut hi = [f32::NEG_INFINITY; 3];
        let off = bin.len();
        for &i in &used {
            let p = mesh.positions[i as usize];
            let q = [p[0], p[2], -p[1]];
            for k in 0..3 {
                lo[k] = lo[k].min(q[k]);
                hi[k] = hi[k].max(q[k]);
                bin.extend(q[k].to_le_bytes());
            }
        }
        views.push(serde_json::json!({"buffer": 0, "byteOffset": off, "byteLength": bin.len() - off, "target": 34962}));
        pad(&mut bin);
        accessors.push(
            serde_json::json!({"bufferView": views.len() - 1, "componentType": 5126,
            "count": used.len(), "type": "VEC3", "min": lo, "max": hi}),
        );
        let pos = accessors.len() - 1;
        // Vertex colours are linear in glTF: 16 bits each so that dark
        // ones keep their tone.
        let off = bin.len();
        for &i in &used {
            for x in mesh.colors[i as usize] {
                bin.extend(((linear(x) * 65535.0).round() as u16).to_le_bytes());
            }
        }
        views.push(serde_json::json!({"buffer": 0, "byteOffset": off, "byteLength": bin.len() - off, "target": 34962}));
        pad(&mut bin);
        accessors.push(
            serde_json::json!({"bufferView": views.len() - 1, "componentType": 5123,
            "normalized": true, "count": used.len(), "type": "VEC3"}),
        );
        let col = accessors.len() - 1;
        let off = bin.len();
        for t in &tris {
            for i in t {
                bin.extend(i.to_le_bytes());
            }
        }
        views.push(serde_json::json!({"buffer": 0, "byteOffset": off, "byteLength": bin.len() - off, "target": 34963}));
        pad(&mut bin);
        accessors.push(
            serde_json::json!({"bufferView": views.len() - 1, "componentType": 5125,
            "count": tris.len() * 3, "type": "SCALAR"}),
        );
        primitives.push(serde_json::json!({
            "attributes": {"POSITION": pos, "COLOR_0": col},
            "indices": accessors.len() - 1,
            "material": g.material,
        }));
    }
    let materials: Vec<serde_json::Value> = mesh
        .materials
        .iter()
        .map(|m| {
            let mut v = serde_json::json!({
                "name": m.name,
                "pbrMetallicRoughness": {"baseColorFactor": [1.0, 1.0, 1.0, m.alpha as f64],
                    "metallicFactor": 0.0, "roughnessFactor": 0.9},
                "doubleSided": true,
            });
            if m.alpha < 0.999 {
                v["alphaMode"] = "BLEND".into();
            }
            v
        })
        .collect();
    let json = serde_json::json!({
        "asset": {"version": "2.0", "generator": "ArchiSim"},
        "scene": 0,
        "scenes": [{"nodes": [0]}],
        "nodes": [{"mesh": 0, "name": name}],
        "meshes": [{"name": name, "primitives": primitives}],
        "materials": materials,
        "accessors": accessors,
        "bufferViews": views,
        "buffers": [{"byteLength": bin.len()}],
    });
    let mut text = serde_json::to_vec(&json).unwrap_or_default();
    while !text.len().is_multiple_of(4) {
        text.push(b' ');
    }
    let total = 12 + 8 + text.len() + 8 + bin.len();
    let mut out = Vec::with_capacity(total);
    out.extend(0x4654_6C67u32.to_le_bytes());
    out.extend(2u32.to_le_bytes());
    out.extend((total as u32).to_le_bytes());
    out.extend((text.len() as u32).to_le_bytes());
    out.extend(0x4E4F_534Au32.to_le_bytes());
    out.extend(text);
    out.extend((bin.len() as u32).to_le_bytes());
    out.extend(0x004E_4942u32.to_le_bytes());
    out.extend(bin);
    out
}
