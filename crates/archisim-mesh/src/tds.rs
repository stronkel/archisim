//! 3D Studio (`.3ds`), the oldest format still handed out by
//! manufacturers: objects of triangles already in place, with their
//! materials (diffuse colour, transparency, texture with its tiling). It
//! says nothing of units, and is `z` up.

use std::collections::HashMap;

use crate::texture::{from_unit, Texture};
use crate::{to_ours, Builder, Error, Material, Mesh, Options, Resources, Rgb, Up};

fn invalid(s: &str) -> Error {
    Error::Invalid(format!("3DS: {s}"))
}

/// The chunks inside `b`: (id, contents).
fn chunks(b: &[u8]) -> Vec<(u16, &[u8])> {
    let mut out = Vec::new();
    let mut at = 0;
    while at + 6 <= b.len() {
        let id = u16::from_le_bytes([b[at], b[at + 1]]);
        let len = u32::from_le_bytes(b[at + 2..at + 6].try_into().unwrap()) as usize;
        if len < 6 || at + len > b.len() {
            break;
        }
        out.push((id, &b[at + 6..at + len]));
        at += len;
    }
    out
}

/// A string ended by a zero, and what follows it.
fn cstr(b: &[u8]) -> (String, &[u8]) {
    let end = b.iter().position(|c| *c == 0).unwrap_or(b.len());
    let s = b[..end].iter().map(|c| *c as char).collect();
    (s, b.get(end + 1..).unwrap_or(&[]))
}

fn f32_at(b: &[u8], i: usize) -> f32 {
    b.get(i..i + 4)
        .map_or(0.0, |s| f32::from_le_bytes(s.try_into().unwrap()))
}

fn u16_at(b: &[u8], i: usize) -> usize {
    b.get(i..i + 2)
        .map_or(0, |s| u16::from_le_bytes([s[0], s[1]]) as usize)
}

/// A colour chunk (float or byte, linear or gamma) inside `b`.
fn colour(b: &[u8]) -> Option<Rgb> {
    let mut found = None;
    for (id, c) in chunks(b) {
        match id {
            0x0011 | 0x0012 if c.len() >= 3 => return Some([c[0], c[1], c[2]]),
            0x0010 | 0x0013 if c.len() >= 12 => {
                found = Some(from_unit([0, 4, 8].map(|i| f32_at(c, i) as f64)))
            }
            _ => {}
        }
    }
    found
}

fn percent(b: &[u8]) -> Option<f32> {
    chunks(b).into_iter().find_map(|(id, c)| match id {
        0x0030 => Some(u16_at(c, 0) as f32 / 100.0),
        0x0031 => Some(f32_at(c, 0)),
        _ => None,
    })
}

struct Mat {
    color: Rgb,
    alpha: f32,
    map: Option<String>,
    /// u scale, v scale, u offset, v offset.
    tiling: [f32; 4],
}

fn material(b: &[u8]) -> (String, Mat) {
    let mut name = String::new();
    let mut m = Mat {
        color: [200, 200, 200],
        alpha: 1.0,
        map: None,
        tiling: [1.0, 1.0, 0.0, 0.0],
    };
    for (id, c) in chunks(b) {
        match id {
            0xA000 => name = cstr(c).0,
            0xA020 => m.color = colour(c).unwrap_or(m.color),
            0xA050 => m.alpha = 1.0 - percent(c).unwrap_or(0.0).clamp(0.0, 1.0),
            0xA200 => {
                for (id, c) in chunks(c) {
                    match id {
                        0xA300 => m.map = Some(cstr(c).0).filter(|s| !s.is_empty()),
                        0xA354 => m.tiling[0] = f32_at(c, 0),
                        0xA356 => m.tiling[1] = f32_at(c, 0),
                        0xA358 => m.tiling[2] = f32_at(c, 0),
                        0xA35A => m.tiling[3] = f32_at(c, 0),
                        _ => {}
                    }
                }
            }
            _ => {}
        }
    }
    (name, m)
}

pub fn read(bytes: &[u8], files: &dyn Resources, options: Options) -> Result<Mesh, Error> {
    let top = chunks(bytes);
    let Some((0x4D4D, main)) = top.first().copied() else {
        return Err(invalid("no es un fichero de 3D Studio"));
    };
    let up = match options.up {
        Up::Auto => Up::Z,
        u => u,
    };
    let unit = options.unit.unwrap_or(1.0);
    let editor: Vec<&[u8]> = chunks(main)
        .into_iter()
        .filter(|(id, _)| *id == 0x3D3D)
        .map(|(_, c)| c)
        .collect();
    let mut mats: HashMap<String, Mat> = HashMap::new();
    for e in &editor {
        for (id, c) in chunks(e) {
            if id == 0xAFFF {
                let (n, m) = material(c);
                mats.insert(n, m);
            }
        }
    }
    let mut textures: HashMap<String, Option<Texture>> = HashMap::new();
    let mut b = Builder::default();
    for e in &editor {
        for (id, obj) in chunks(e) {
            if id != 0x4000 {
                continue;
            }
            let (_name, rest) = cstr(obj);
            for (id, tri) in chunks(rest) {
                if id != 0x4100 {
                    continue;
                }
                let mut points: Vec<[f32; 3]> = Vec::new();
                let mut uvs: Vec<[f32; 2]> = Vec::new();
                let mut faces: Vec<[usize; 3]> = Vec::new();
                let mut groups: Vec<(String, Vec<usize>)> = Vec::new();
                for (id, c) in chunks(tri) {
                    match id {
                        0x4110 => {
                            let n = u16_at(c, 0);
                            points = (0..n)
                                .map(|i| {
                                    let at = 2 + i * 12;
                                    [f32_at(c, at), f32_at(c, at + 4), f32_at(c, at + 8)]
                                })
                                .collect();
                        }
                        0x4140 => {
                            let n = u16_at(c, 0);
                            uvs = (0..n)
                                .map(|i| [f32_at(c, 2 + i * 8), f32_at(c, 6 + i * 8)])
                                .collect();
                        }
                        0x4120 => {
                            let n = u16_at(c, 0);
                            if c.len() < 2 + n * 8 {
                                return Err(invalid("lista de caras cortada"));
                            }
                            faces = (0..n)
                                .map(|i| {
                                    let at = 2 + i * 8;
                                    [u16_at(c, at), u16_at(c, at + 2), u16_at(c, at + 4)]
                                })
                                .collect();
                            for (id, g) in chunks(&c[2 + n * 8..]) {
                                if id == 0x4130 {
                                    let (name, rest) = cstr(g);
                                    let k = u16_at(rest, 0);
                                    let list = (0..k).map(|i| u16_at(rest, 2 + i * 2)).collect();
                                    groups.push((name, list));
                                }
                            }
                        }
                        _ => {}
                    }
                }
                // Faces of no material: the default.
                let mut owner: Vec<Option<usize>> = vec![None; faces.len()];
                for (gi, (_, list)) in groups.iter().enumerate() {
                    for &f in list {
                        if let Some(o) = owner.get_mut(f) {
                            *o = Some(gi);
                        }
                    }
                }
                let mut made: HashMap<(usize, usize), u32> = HashMap::new();
                for (fi, face) in faces.iter().enumerate() {
                    if face.iter().any(|i| *i >= points.len()) {
                        return Err(invalid("una cara usa un vértice que no existe"));
                    }
                    let name = owner[fi].map_or("Predeterminado", |g| groups[g].0.as_str());
                    let mat = mats.get(name);
                    let tex = mat.and_then(|m| m.map.as_ref()).and_then(|f| {
                        textures
                            .entry(f.clone())
                            .or_insert_with(|| files.get(f).and_then(|d| Texture::decode(&d)))
                            .as_ref()
                    });
                    let base = match (tex, mat) {
                        (Some(t), _) => t.average(),
                        (None, Some(m)) => m.color,
                        (None, None) => [200, 200, 200],
                    };
                    let material = b.material(name, || Material {
                        name: String::new(),
                        color: base,
                        alpha: mat.map_or(1.0, |m| m.alpha),
                        texture: mat.and_then(|m| m.map.clone()),
                    });
                    let ids = face.map(|p| {
                        *made.entry((p, material)).or_insert_with(|| {
                            let color = match (tex, uvs.get(p), mat) {
                                (Some(t), Some(uv), Some(m)) => t.sample(
                                    uv[0] * m.tiling[0] + m.tiling[2],
                                    uv[1] * m.tiling[1] + m.tiling[3],
                                    true,
                                ),
                                _ => base,
                            };
                            let q = points[p];
                            b.vertex(
                                to_ours(up, unit, [q[0] as f64, q[1] as f64, q[2] as f64]),
                                color,
                            )
                        })
                    });
                    b.triangle(material, ids);
                }
            }
        }
    }
    Ok(b.mesh)
}
