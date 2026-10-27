//! Lighter versions of a mesh for far views (levels of detail); when
//! collapsing edges cannot get there, by vertex clustering: the vertices in each cell of a grid become one (the
//! one farthest from the middle of the model, so that its size and
//! outline stay), with their average colour, and the triangles left flat
//! go away.

use std::collections::{HashMap, HashSet};

use crate::{Group, Mesh};

/// The mesh with its vertices clustered on a grid of `cells` along each
/// side of each material's part: a thin trunk keeps its own
/// coarse shape instead of vanishing into the canopy's cells.
fn cluster(mesh: &Mesh, cells: u32) -> Mesh {
    let mut out = Mesh {
        materials: mesh.materials.clone(),
        ..Default::default()
    };
    for g in &mesh.groups {
        let part = Mesh {
            positions: mesh.positions.clone(),
            colors: mesh.colors.clone(),
            materials: Vec::new(),
            groups: vec![g.clone()],
        };
        let (lo, hi) = part.bounds();
        // As many cells along each side (a thin part keeps its
        // thickness), the last cell closed at the far face.
        let step = [0, 1, 2].map(|k| ((hi[k] - lo[k]) / cells as f32).max(1e-6));
        let cell_of = |p: [f32; 3]| -> (i32, i32, i32) {
            let c = |k: usize| (((p[k] - lo[k]) / step[k]).floor() as i32).min(cells as i32 - 1);
            (c(0), c(1), c(2))
        };
        let mid = [0, 1, 2].map(|k| (lo[k] + hi[k]) / 2.0);
        let far = |p: [f32; 3]| (0..3).map(|k| (p[k] - mid[k]).powi(2)).sum::<f32>();
        // Per cell: the farthest vertex, the sum of colours and how many.
        let mut index: HashMap<(i32, i32, i32), u32> = HashMap::new();
        let mut sums: Vec<([f32; 3], [u64; 3], u64)> = Vec::new();
        let mut map: HashMap<u32, u32> = HashMap::new();
        for t in &g.triangles {
            for &v in t {
                if map.contains_key(&v) {
                    continue;
                }
                let p = mesh.positions[v as usize];
                let id = *index.entry(cell_of(p)).or_insert_with(|| {
                    sums.push((p, [0; 3], 0));
                    (sums.len() - 1) as u32
                });
                let s = &mut sums[id as usize];
                if far(p) > far(s.0) {
                    s.0 = p;
                }
                for k in 0..3 {
                    s.1[k] += mesh.colors[v as usize][k] as u64;
                }
                s.2 += 1;
                map.insert(v, id);
            }
        }
        let first = out.positions.len() as u32;
        out.positions.extend(sums.iter().map(|(p, _, _)| *p));
        out.colors
            .extend(sums.iter().map(|(_, c, n)| c.map(|v| (v / *n) as u8)));
        let mut seen = HashSet::new();
        let mut tris = Vec::new();
        for t in &g.triangles {
            let t = t.map(|v| map[&v]);
            if t[0] == t[1] || t[1] == t[2] || t[0] == t[2] {
                continue;
            }
            // The same triangle (either way round) once.
            let mut key = t;
            key.sort_unstable();
            if seen.insert(key) {
                tris.push(t.map(|v| first + v));
            }
        }
        if !tris.is_empty() {
            out.groups.push(Group {
                material: g.material,
                triangles: tris,
            });
        }
    }
    out
}

/// A version of `mesh` with at most `target` triangles (the mesh itself if
/// it has no more). Never fewer than a handful: a very light version is a
/// rough box.
pub fn simplify(mesh: &Mesh, target: usize) -> Mesh {
    if mesh.triangle_count() <= target {
        return mesh.clone();
    }
    let (mut lo, mut hi) = (2u32, 512u32);
    let mut best = cluster(mesh, lo);
    while lo < hi {
        let mid = (lo + hi).div_ceil(2);
        let m = cluster(mesh, mid);
        if m.triangle_count() <= target {
            best = m;
            lo = mid;
        } else {
            hi = mid - 1;
        }
    }
    best
}

/// Triangles of the levels of detail made for a model: the second and the
/// third, when the model has many more.
pub const LEVELS: [usize; 2] = [4_000, 600];

/// The model and its lighter versions, each clearly lighter than the one
/// before: the first is the model itself. Each is made from the one before
/// by collapsing edges ([`decimate`](crate::decimate)), which keeps the
/// shape; a model of loose pieces that will not go down that way (leaves
/// of a tree) is clustered on a grid.
pub fn levels_of_detail(mesh: &Mesh) -> Vec<Mesh> {
    let mut out = vec![mesh.clone()];
    for target in LEVELS {
        let last = out.last().map_or(0, |m| m.triangle_count());
        if last > target * 3 / 2 {
            let from = out.last().expect("the model is there");
            let m = crate::decimate(from, target)
                .filter(|m| m.triangle_count() < last)
                .unwrap_or_else(|| simplify(mesh, target));
            if m.triangle_count() < last {
                out.push(m);
            }
        }
    }
    out
}
