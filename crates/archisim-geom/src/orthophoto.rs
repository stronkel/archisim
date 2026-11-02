//! The orthophoto in 3D (V2): a grid draped a few centimetres over the
//! top of the terrain (and of the streets, if asked), each corner of the
//! colour of the picture around it; the 3D view, the render and the glTF
//! draw it by its corners' colours (`solid_vertex_colors`), like the
//! decals.

use archisim_core::{Model, Point2};

use crate::decals::{decal_material, pixels_of, Pixels};
use crate::solid::{Solid, SolidOwner, P3};

/// The most squares across the longer side of the picture.
pub const ORTHO_CELLS: usize = 256;
/// How far over the ground the picture floats, metres.
const LIFT: f64 = 0.03;

/// The grid: its nodes' heights (none where there is no ground) and
/// colours, and how many squares each way.
struct Grid {
    nx: usize,
    ny: usize,
    z: Vec<Option<f64>>,
    colour: Vec<[u8; 3]>,
}

/// The upward faces of the ground under the picture: the terrains' and,
/// if asked, the streets'.
fn ground_triangles(model: &Model, on_roads: bool) -> Vec<[P3; 3]> {
    let mut out = Vec::new();
    let mut take = |s: Solid| {
        for t in s.triangles {
            let (a, b, c) = (t[0], t[1], t[2]);
            let nz = (b[0] - a[0]) * (c[1] - a[1]) - (b[1] - a[1]) * (c[0] - a[0]);
            let ab = [b[0] - a[0], b[1] - a[1], b[2] - a[2]];
            let ac = [c[0] - a[0], c[1] - a[1], c[2] - a[2]];
            let n = [
                ab[1] * ac[2] - ab[2] * ac[1],
                ab[2] * ac[0] - ab[0] * ac[2],
                nz,
            ];
            let len = (n[0] * n[0] + n[1] * n[1] + n[2] * n[2]).sqrt();
            if len > 1e-12 && nz / len > 0.3 {
                out.push(t);
            }
        }
    };
    for t in model.terrains() {
        for s in crate::solid::terrain_solids(model, t.id) {
            take(s);
        }
    }
    if on_roads {
        for l in model.levels() {
            for s in crate::solid::level_road_solids(model, l.id) {
                take(s);
            }
        }
    }
    out
}

/// The mean colour of the picture over a window round `(u, v)`.
fn mean(px: &Pixels, u: f64, v: f64, du: f64, dv: f64) -> [u8; 3] {
    let x0 = ((u - du / 2.0) * px.width as f64).floor().max(0.0) as usize;
    let x1 = (((u + du / 2.0) * px.width as f64).ceil() as usize).clamp(x0 + 1, px.width);
    let y0 = (((1.0 - v - dv / 2.0) * px.height as f64).floor()).max(0.0) as usize;
    let y1 = ((((1.0 - v + dv / 2.0) * px.height as f64).ceil()) as usize).clamp(y0 + 1, px.height);
    let x0 = x0.min(px.width - 1);
    let y0 = y0.min(px.height - 1);
    // At most 8 × 8 samples a window.
    let sx = ((x1 - x0) / 8).max(1);
    let sy = ((y1 - y0) / 8).max(1);
    let (mut r, mut g, mut b, mut n) = (0u64, 0u64, 0u64, 0u64);
    let mut y = y0;
    while y < y1 {
        let mut x = x0;
        while x < x1 {
            let i = (y * px.width + x) * 4;
            r += px.rgba[i] as u64;
            g += px.rgba[i + 1] as u64;
            b += px.rgba[i + 2] as u64;
            n += 1;
            x += sx;
        }
        y += sy;
    }
    let n = n.max(1);
    [(r / n) as u8, (g / n) as u8, (b / n) as u8]
}

fn grid(model: &Model) -> Option<(Grid, [Point2; 4])> {
    let o = model.orthophoto()?;
    let px = pixels_of(&o.image.png)?;
    let (w, h) = o.size();
    let k = ORTHO_CELLS as f64 / w.max(h);
    let nx = ((w * k).round() as usize).clamp(1, ORTHO_CELLS);
    let ny = ((h * k).round() as usize).clamp(1, ORTHO_CELLS);
    let node = |i: usize, j: usize| -> Point2 {
        let (a, b) = (i as f64 / nx as f64, j as f64 / ny as f64);
        Point2::new(
            o.origin[0] + o.u[0] * a + o.v[0] * b,
            o.origin[1] + o.u[1] * a + o.v[1] * b,
        )
    };
    let (cols, rows) = (nx + 1, ny + 1);
    let mut z: Vec<Option<f64>> = vec![None; cols * rows];
    let tris = ground_triangles(model, o.on_roads);
    // Each triangle onto the nodes under it, keeping the highest.
    let inv = {
        let det = o.u[0] * o.v[1] - o.u[1] * o.v[0];
        move |p: [f64; 2]| -> (f64, f64) {
            let (dx, dy) = (p[0] - o.origin[0], p[1] - o.origin[1]);
            (
                (dx * o.v[1] - dy * o.v[0]) / det * nx as f64,
                (o.u[0] * dy - o.u[1] * dx) / det * ny as f64,
            )
        }
    };
    for t in &tris {
        let g = [
            inv([t[0][0], t[0][1]]),
            inv([t[1][0], t[1][1]]),
            inv([t[2][0], t[2][1]]),
        ];
        let (i0, i1) = (
            g.iter()
                .map(|p| p.0)
                .fold(f64::MAX, f64::min)
                .ceil()
                .max(0.0) as usize,
            (g.iter().map(|p| p.0).fold(f64::MIN, f64::max).floor()).min(nx as f64),
        );
        let (j0, j1) = (
            g.iter()
                .map(|p| p.1)
                .fold(f64::MAX, f64::min)
                .ceil()
                .max(0.0) as usize,
            (g.iter().map(|p| p.1).fold(f64::MIN, f64::max).floor()).min(ny as f64),
        );
        if i1 < 0.0 || j1 < 0.0 {
            continue;
        }
        let d = (g[1].1 - g[2].1) * (g[0].0 - g[2].0) + (g[2].0 - g[1].0) * (g[0].1 - g[2].1);
        if d.abs() < 1e-12 {
            continue;
        }
        for j in j0..=(j1 as usize) {
            for i in i0..=(i1 as usize) {
                let (x, y) = (i as f64, j as f64);
                let l0 = ((g[1].1 - g[2].1) * (x - g[2].0) + (g[2].0 - g[1].0) * (y - g[2].1)) / d;
                let l1 = ((g[2].1 - g[0].1) * (x - g[2].0) + (g[0].0 - g[2].0) * (y - g[2].1)) / d;
                let l2 = 1.0 - l0 - l1;
                if l0 < -1e-9 || l1 < -1e-9 || l2 < -1e-9 {
                    continue;
                }
                let zz = l0 * t[0][2] + l1 * t[1][2] + l2 * t[2][2];
                let slot = &mut z[j * cols + i];
                if slot.is_none_or(|old| zz > old) {
                    *slot = Some(zz);
                }
            }
        }
    }
    // With no ground at all, the picture lies at the project's ±0,00.
    if tris.is_empty() {
        z.iter_mut().for_each(|v| *v = Some(0.0));
    }
    let (du, dv) = (1.0 / nx as f64, 1.0 / ny as f64);
    let mut colour = Vec::with_capacity(cols * rows);
    for j in 0..rows {
        for i in 0..cols {
            colour.push(mean(&px, i as f64 * du, j as f64 * dv, du, dv));
        }
    }
    let corners = [node(0, 0), node(nx, 0), node(nx, ny), node(0, ny)];
    Some((Grid { nx, ny, z, colour }, corners))
}

/// The squares of the picture with their corners' colours.
fn cells(model: &Model) -> Vec<([P3; 3], [[u8; 3]; 3])> {
    let Some((g, c)) = grid(model) else {
        return Vec::new();
    };
    let cols = g.nx + 1;
    let at = |i: usize, j: usize| -> Option<(P3, [u8; 3])> {
        let z = g.z[j * cols + i]?;
        let (a, b) = (i as f64 / g.nx as f64, j as f64 / g.ny as f64);
        let x = c[0].x + (c[1].x - c[0].x) * a + (c[3].x - c[0].x) * b;
        let y = c[0].y + (c[1].y - c[0].y) * a + (c[3].y - c[0].y) * b;
        Some(([x, y, z + LIFT], g.colour[j * cols + i]))
    };
    let mut out = Vec::with_capacity(g.nx * g.ny * 2);
    for j in 0..g.ny {
        for i in 0..g.nx {
            let (Some(a), Some(b), Some(cc), Some(d)) =
                (at(i, j), at(i + 1, j), at(i + 1, j + 1), at(i, j + 1))
            else {
                continue;
            };
            out.push(([a.0, b.0, cc.0], [a.1, b.1, cc.1]));
            out.push(([a.0, cc.0, d.0], [a.1, cc.1, d.1]));
        }
    }
    out
}

/// The solid of the orthophoto, if the project has one.
pub fn orthophoto_solids(model: &Model) -> Vec<Solid> {
    let triangles: Vec<[P3; 3]> = cells(model).into_iter().map(|(t, _)| t).collect();
    if triangles.is_empty() {
        return Vec::new();
    }
    vec![Solid {
        owner: SolidOwner::Orthophoto,
        material: decal_material(model),
        triangles,
    }]
}

/// The colours of its triangles' corners, in the order of
/// `orthophoto_solids`.
pub fn orthophoto_colors(model: &Model) -> Option<Vec<[[u8; 3]; 3]>> {
    model.orthophoto()?;
    Some(cells(model).into_iter().map(|(_, c)| c).collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use archisim_core::{decal_image, NewTerrain, Orthophoto, SurveyPoint};

    /// A PNG of 2 × 2: red on the left, blue on the right.
    fn halves() -> Vec<u8> {
        let mut out = Vec::new();
        {
            let mut e = png::Encoder::new(&mut out, 2, 2);
            e.set_color(png::ColorType::Rgb);
            e.set_depth(png::BitDepth::Eight);
            let mut w = e.write_header().unwrap();
            w.write_image_data(&[200, 0, 0, 0, 0, 200, 200, 0, 0, 0, 0, 200])
                .unwrap();
        }
        out
    }

    #[test]
    fn the_picture_lies_on_the_ground() {
        let mut m = Model::new("T");
        let l = m.add_level("Planta baja", 0.0);
        let earth = m.add_material(
            "Tierra",
            "earth",
            [120, 100, 80],
            archisim_core::CutPattern::Solid,
        );
        let sp = |x: f64, y: f64, z: f64| SurveyPoint {
            at: Point2::new(x, y),
            z,
        };
        // A terrain rising to the east, 2 m in 40 m.
        m.add_terrain(NewTerrain {
            level: l,
            points: vec![
                sp(-20.0, -20.0, 0.0),
                sp(20.0, -20.0, 2.0),
                sp(20.0, 20.0, 2.0),
                sp(-20.0, 20.0, 0.0),
            ],
            material: earth,
            depth: 1.0,
            contour: 0.5,
            lines: Vec::new(),
        })
        .unwrap();
        m.set_orthophoto(Some(Orthophoto {
            image: decal_image("o.png", halves()).unwrap(),
            origin: [-10.0, -10.0],
            u: [20.0, 0.0],
            v: [0.0, 20.0],
            on_roads: false,
        }))
        .unwrap();
        let s = orthophoto_solids(&m);
        assert_eq!(s.len(), 1);
        let t = &s[0].triangles;
        assert!(!t.is_empty());
        for p in t.iter().flatten() {
            assert!(p[0] >= -10.0 - 1e-9 && p[0] <= 10.0 + 1e-9);
            // Over the slope, by its lift.
            let ground = (p[0] + 20.0) / 40.0 * 2.0;
            assert!((p[2] - ground - LIFT).abs() < 0.02, "{p:?}");
        }
        let c = orthophoto_colors(&m).unwrap();
        assert_eq!(c.len(), t.len());
        // The west reddish, the east bluish.
        let west = t.iter().zip(&c).find(|(t, _)| t[0][0] < -9.0).unwrap().1[0];
        let east = t.iter().zip(&c).find(|(t, _)| t[0][0] > 9.0).unwrap().1[0];
        assert!(west[0] > west[2] && east[2] > east[0], "{west:?} {east:?}");
    }
}
