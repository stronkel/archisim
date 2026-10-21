//! 3D models brought from other programs (version 24): their triangles,
//! vertex colours and materials, model and lighter versions, as base64 of
//! little-endian numbers so that a model of thousands of triangles does
//! not take a line per number.

use archisim_core as core;
use serde::{Deserialize, Serialize};

use crate::FileError;

/// A 3D model kept in the project.
#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Mesh {
    pub id: u64,
    pub name: String,
    /// The file it came from (its name).
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub source: String,
    /// The model, then lighter versions.
    pub levels: Vec<MeshLevel>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct MeshLevel {
    /// x, y, z of each vertex: 32-bit floats, metres from the insertion
    /// point, z up.
    pub positions: String,
    /// R, G, B of each vertex: one byte each.
    pub colors: String,
    pub groups: Vec<MeshGroup>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct MeshGroup {
    pub material: u64,
    /// Three 32-bit vertex indices per triangle.
    pub triangles: String,
}

const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

pub(crate) fn encode(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for c in bytes.chunks(3) {
        let n = (c[0] as u32) << 16
            | (*c.get(1).unwrap_or(&0) as u32) << 8
            | *c.get(2).unwrap_or(&0) as u32;
        for k in 0..4 {
            if k <= c.len() {
                out.push(ALPHABET[(n >> (18 - 6 * k)) as usize & 63] as char);
            } else {
                out.push('=');
            }
        }
    }
    out
}

pub(crate) fn decode(s: &str) -> Option<Vec<u8>> {
    let mut out = Vec::with_capacity(s.len() * 3 / 4);
    let (mut acc, mut bits) = (0u32, 0);
    for c in s.bytes() {
        let v = match c {
            b'A'..=b'Z' => c - b'A',
            b'a'..=b'z' => c - b'a' + 26,
            b'0'..=b'9' => c - b'0' + 52,
            b'+' => 62,
            b'/' => 63,
            b'=' => continue,
            _ => return None,
        } as u32;
        acc = (acc << 6) | v;
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push((acc >> bits) as u8);
            acc &= (1 << bits) - 1;
        }
    }
    Some(out)
}

pub(crate) fn mesh_out(m: &core::MeshModel) -> Mesh {
    Mesh {
        id: m.id.0,
        name: m.name.clone(),
        source: m.source.clone(),
        levels: m
            .levels
            .iter()
            .map(|l| MeshLevel {
                positions: encode(
                    &l.positions
                        .iter()
                        .flatten()
                        .flat_map(|c| c.to_le_bytes())
                        .collect::<Vec<u8>>(),
                ),
                colors: encode(&l.colors.iter().flatten().copied().collect::<Vec<u8>>()),
                groups: l
                    .groups
                    .iter()
                    .map(|g| MeshGroup {
                        material: g.material.0,
                        triangles: encode(
                            &g.triangles
                                .iter()
                                .flatten()
                                .flat_map(|i| i.to_le_bytes())
                                .collect::<Vec<u8>>(),
                        ),
                    })
                    .collect(),
            })
            .collect(),
    }
}

pub(crate) fn mesh_in(m: Mesh) -> Result<core::MeshModel, FileError> {
    let element = format!("3D model {} «{}»", m.id, m.name);
    let bad = |what: &str| FileError::Value {
        element: element.clone(),
        message: format!("{what} no válidos"),
    };
    let mut levels = Vec::new();
    for l in m.levels {
        let p = decode(&l.positions).ok_or_else(|| bad("vértices"))?;
        if p.len() % 12 != 0 {
            return Err(bad("vértices"));
        }
        let positions: Vec<[f32; 3]> = p
            .chunks_exact(12)
            .map(|c| {
                let f = |k: usize| f32::from_le_bytes([c[k], c[k + 1], c[k + 2], c[k + 3]]);
                [f(0), f(4), f(8)]
            })
            .collect();
        let c = decode(&l.colors).ok_or_else(|| bad("colores"))?;
        if c.len() % 3 != 0 {
            return Err(bad("colores"));
        }
        let colors = c.chunks_exact(3).map(|c| [c[0], c[1], c[2]]).collect();
        let mut groups = Vec::new();
        for g in l.groups {
            let t = decode(&g.triangles).ok_or_else(|| bad("triángulos"))?;
            if t.len() % 12 != 0 {
                return Err(bad("triángulos"));
            }
            let triangles = t
                .chunks_exact(12)
                .map(|c| {
                    let i = |k: usize| u32::from_le_bytes([c[k], c[k + 1], c[k + 2], c[k + 3]]);
                    [i(0), i(4), i(8)]
                })
                .collect();
            groups.push(core::MeshGroup {
                material: core::MaterialId(g.material),
                triangles,
            });
        }
        levels.push(core::MeshLevel {
            positions,
            colors,
            groups,
        });
    }
    Ok(core::MeshModel {
        id: core::MeshId(m.id),
        name: m.name,
        source: m.source,
        levels,
    })
}

#[cfg(test)]
mod tests {
    #[test]
    fn base64_round_trip() {
        for n in 0..20u8 {
            let bytes: Vec<u8> = (0..n).map(|i| i.wrapping_mul(37)).collect();
            assert_eq!(super::decode(&super::encode(&bytes)).unwrap(), bytes);
        }
        assert_eq!(super::encode(b"Man"), "TWFu");
        assert_eq!(super::encode(b"Ma"), "TWE=");
    }
}
