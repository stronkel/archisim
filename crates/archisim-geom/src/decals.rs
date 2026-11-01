//! Decals in 3D (U8): the picture laid on its face as a grid of little
//! squares, each of the colour of its pixel, leaving out the transparent
//! ones; the 3D view, the render and the glTF draw them by their corners'
//! colours (`solid_vertex_colors`). The pictures are decoded once and kept.

use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock};

use archisim_core::{Decal, DecalId, MaterialId, Model};

use crate::solid::{Solid, SolidOwner, P3};

/// The most squares across the longer side of a decal.
pub const DECAL_CELLS: usize = 128;
/// Below this opacity a pixel is left out.
const ALPHA_CUT: u8 = 110;

/// A decoded picture: its pixels as RGBA, row by row from the top.
#[derive(Debug, Clone)]
pub struct Pixels {
    pub width: usize,
    pub height: usize,
    pub rgba: Vec<u8>,
}

impl Pixels {
    /// The pixel at `(u, v)`, both from 0 to 1, `v` from the bottom.
    pub fn at(&self, u: f64, v: f64) -> [u8; 4] {
        let x = ((u * self.width as f64) as usize).min(self.width - 1);
        let y = (((1.0 - v) * self.height as f64) as usize).min(self.height - 1);
        let i = (y * self.width + x) * 4;
        [
            self.rgba[i],
            self.rgba[i + 1],
            self.rgba[i + 2],
            self.rgba[i + 3],
        ]
    }
}

/// Decodes a PNG into RGBA (grey, palette and 16-bit ones too).
pub fn decode_png(png: &[u8]) -> Option<Pixels> {
    let mut d = png::Decoder::new(std::io::Cursor::new(png));
    d.set_transformations(png::Transformations::normalize_to_color8());
    let mut r = d.read_info().ok()?;
    let mut buf = vec![0; r.output_buffer_size()?];
    let info = r.next_frame(&mut buf).ok()?;
    let (w, h) = (info.width as usize, info.height as usize);
    let mut rgba = Vec::with_capacity(w * h * 4);
    let px = &buf[..info.buffer_size()];
    match info.color_type {
        png::ColorType::Rgba => rgba.extend_from_slice(px),
        png::ColorType::Rgb => {
            for c in px.chunks_exact(3) {
                rgba.extend_from_slice(&[c[0], c[1], c[2], 255]);
            }
        }
        png::ColorType::GrayscaleAlpha => {
            for c in px.chunks_exact(2) {
                rgba.extend_from_slice(&[c[0], c[0], c[0], c[1]]);
            }
        }
        png::ColorType::Grayscale => {
            for c in px {
                rgba.extend_from_slice(&[*c, *c, *c, 255]);
            }
        }
        png::ColorType::Indexed => return None,
    }
    (w > 0 && h > 0 && rgba.len() == w * h * 4).then_some(Pixels {
        width: w,
        height: h,
        rgba,
    })
}

/// The pictures decoded so far, by the address of their bytes (kept to
/// make sure the address is still theirs).
type Decoded = HashMap<usize, (Arc<Vec<u8>>, Option<Arc<Pixels>>)>;

/// The decoded picture of a PNG, kept by its bytes' address.
pub fn pixels_of(png: &Arc<Vec<u8>>) -> Option<Arc<Pixels>> {
    static CACHE: OnceLock<Mutex<Decoded>> = OnceLock::new();
    let cache = CACHE.get_or_init(Default::default);
    let key = Arc::as_ptr(png) as usize;
    if let Ok(c) = cache.lock() {
        if let Some((kept, p)) = c.get(&key) {
            if Arc::ptr_eq(kept, png) {
                return p.clone();
            }
        }
    }
    let p = decode_png(png).map(Arc::new);
    if let Ok(mut c) = cache.lock() {
        if c.len() > 256 {
            c.clear();
        }
        c.insert(key, (png.clone(), p.clone()));
    }
    p
}

/// The squares of a decal: for each, its two triangles and its colour.
pub fn decal_cells(model: &Model, d: &Decal) -> Vec<([[P3; 3]; 2], [u8; 3])> {
    let Some(px) = pixels_of(&d.image.png) else {
        return Vec::new();
    };
    let base = model.level(d.level).map_or(0.0, |l| l.elevation);
    let [p0, p1, _, p3] = d.corners(base);
    let along = [p1[0] - p0[0], p1[1] - p0[1], p1[2] - p0[2]];
    let up = [p3[0] - p0[0], p3[1] - p0[1], p3[2] - p0[2]];
    // As many squares as the picture has pixels, up to DECAL_CELLS.
    let (cols, rows) = {
        let k = (DECAL_CELLS as f64 / px.width.max(px.height) as f64).min(1.0);
        (
            ((px.width as f64 * k).round() as usize).max(1),
            ((px.height as f64 * k).round() as usize).max(1),
        )
    };
    let at = |u: f64, v: f64| -> P3 {
        [
            p0[0] + along[0] * u + up[0] * v,
            p0[1] + along[1] * u + up[1] * v,
            p0[2] + along[2] * u + up[2] * v,
        ]
    };
    let mut out = Vec::new();
    for j in 0..rows {
        let (v0, v1) = (j as f64 / rows as f64, (j + 1) as f64 / rows as f64);
        let vm = (v0 + v1) / 2.0;
        // Runs of the same colour along the row make one strip.
        let mut i = 0;
        while i < cols {
            let um = (i as f64 + 0.5) / cols as f64;
            let c = px.at(um, vm);
            if c[3] < ALPHA_CUT {
                i += 1;
                continue;
            }
            let mut k = i + 1;
            while k < cols {
                let n = px.at((k as f64 + 0.5) / cols as f64, vm);
                if n[3] < ALPHA_CUT || n[..3] != c[..3] {
                    break;
                }
                k += 1;
            }
            let (u0, u1) = (i as f64 / cols as f64, k as f64 / cols as f64);
            let (a, b, cc, dd) = (at(u0, v0), at(u1, v0), at(u1, v1), at(u0, v1));
            out.push(([[a, b, cc], [a, cc, dd]], [c[0], c[1], c[2]]));
            i = k;
        }
    }
    out
}

/// The material a decal's solid is listed under: the project's first (its
/// colours are its corners').
pub fn decal_material(model: &Model) -> MaterialId {
    model.materials().next().map_or(MaterialId(0), |m| m.id)
}

/// The solid of a decal.
pub fn decal_solids(model: &Model, id: DecalId) -> Vec<Solid> {
    let Some(d) = model.decal(id) else {
        return Vec::new();
    };
    let triangles: Vec<[P3; 3]> = decal_cells(model, d)
        .into_iter()
        .flat_map(|(t, _)| t)
        .collect();
    if triangles.is_empty() {
        return Vec::new();
    }
    vec![Solid {
        owner: SolidOwner::Decal(id),
        material: decal_material(model),
        triangles,
    }]
}

/// The colours of a decal's triangles' corners, in the order of
/// `decal_solids`.
pub fn decal_colors(model: &Model, id: DecalId) -> Option<Vec<[[u8; 3]; 3]>> {
    let d = model.decal(id)?;
    Some(
        decal_cells(model, d)
            .into_iter()
            .flat_map(|(_, c)| [[c; 3], [c; 3]])
            .collect(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use archisim_core::{decal_image, NewDecal};

    /// A PNG of 4 × 2: a red half, a transparent half.
    fn half_red() -> Vec<u8> {
        let mut out = Vec::new();
        {
            let mut e = png::Encoder::new(&mut out, 4, 2);
            e.set_color(png::ColorType::Rgba);
            e.set_depth(png::BitDepth::Eight);
            let mut w = e.write_header().unwrap();
            let mut px = Vec::new();
            for _ in 0..2 {
                for x in 0..4 {
                    px.extend_from_slice(if x < 2 {
                        &[220, 30, 30, 255]
                    } else {
                        &[0, 0, 0, 0]
                    });
                }
            }
            w.write_image_data(&px).unwrap();
        }
        out
    }

    #[test]
    fn a_decal_leaves_out_what_is_transparent() {
        let mut m = Model::new("T");
        let l = m.add_level("Planta baja", 1.0);
        m.add_material(
            "Pintura",
            "paint",
            [200, 200, 200],
            archisim_core::CutPattern::Solid,
        );
        let id = m
            .add_decal(NewDecal {
                name: "Cartel".into(),
                level: l,
                centre: [0.0, 0.0, 2.0],
                normal: [0.0, -1.0, 0.0],
                rotation: 0.0,
                width: 4.0,
                image: decal_image("rojo.png", half_red()).unwrap(),
                host: None,
            })
            .unwrap();
        let s = decal_solids(&m, id);
        assert_eq!(s.len(), 1);
        // Two rows, each one red strip of two pixels: four triangles.
        assert_eq!(s[0].triangles.len(), 4);
        for p in s[0].triangles.iter().flatten() {
            // Only the left half, at the level's height plus its own.
            assert!(p[0] <= 1e-9 && p[0] >= -2.0 - 1e-9, "{p:?}");
            assert!(p[2] >= 2.0 && p[2] <= 4.0, "{p:?}");
        }
        let colours = decal_colors(&m, id).unwrap();
        assert_eq!(colours.len(), 4);
        assert_eq!(colours[0][0], [220, 30, 30]);
    }
}
