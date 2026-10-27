//! STL, binary or in text: triangles and nothing else (no materials, no
//! units), what 3D printing and some catalogues use. Taken as `z` up.

use std::collections::HashMap;

use crate::{to_ours, Builder, Error, Material, Mesh, Options, Up};

fn triangles(bytes: &[u8]) -> Result<Vec<[[f32; 3]; 3]>, Error> {
    let binary_len = bytes
        .get(80..84)
        .map(|n| 84 + 50 * u32::from_le_bytes(n.try_into().unwrap()) as usize);
    if binary_len == Some(bytes.len()) {
        let n = (bytes.len() - 84) / 50;
        let f = |at: usize| f32::from_le_bytes(bytes[at..at + 4].try_into().unwrap());
        return Ok((0..n)
            .map(|i| {
                let at = 84 + i * 50 + 12;
                [0, 1, 2].map(|v| [0, 1, 2].map(|k| f(at + v * 12 + k * 4)))
            })
            .collect());
    }
    let text = String::from_utf8_lossy(bytes);
    if !text.trim_start().starts_with("solid") {
        return Err(Error::Invalid("STL: no es un STL".into()));
    }
    let mut out = Vec::new();
    let mut corners: Vec<[f32; 3]> = Vec::new();
    for line in text.lines() {
        let mut it = line.split_whitespace();
        match it.next() {
            Some("vertex") => {
                let n: Vec<f32> = it.filter_map(|s| s.parse().ok()).collect();
                if n.len() == 3 {
                    corners.push([n[0], n[1], n[2]]);
                }
            }
            Some("endfacet") => {
                if corners.len() >= 3 {
                    for k in 1..corners.len() - 1 {
                        out.push([corners[0], corners[k], corners[k + 1]]);
                    }
                }
                corners.clear();
            }
            _ => {}
        }
    }
    Ok(out)
}

pub fn read(bytes: &[u8], options: Options) -> Result<Mesh, Error> {
    let up = match options.up {
        Up::Auto => Up::Z,
        u => u,
    };
    let unit = options.unit.unwrap_or(1.0);
    let mut b = Builder::default();
    let material = b.material("STL", || Material {
        name: String::new(),
        color: [200, 200, 200],
        alpha: 1.0,
        texture: None,
    });
    let mut made: HashMap<[u32; 3], u32> = HashMap::new();
    for t in triangles(bytes)? {
        let ids = t.map(|p| {
            *made.entry(p.map(f32::to_bits)).or_insert_with(|| {
                b.vertex(to_ours(up, unit, p.map(|c| c as f64)), [200, 200, 200])
            })
        });
        b.triangle(material, ids);
    }
    Ok(b.mesh)
}
