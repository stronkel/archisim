//! Pools built in the ground (paquete 8, tanda 8d): their parts as closed
//! meshes, and the terrains with the room the pools take out of them.
//!
//! A pool is made of its shell (floor and walls, of concrete), the lining
//! on its inside faces, the coping on top of its walls and the water. Its
//! floor slopes as [`archisim_core::Pool::depth_at`] says; every level
//! below the water follows the floor at each corner of the outline.

use archisim_core::{Excavation, MaterialId, Model, Point2, PoolId};

use crate::junctions::{difference, intersection, Shape};
use crate::solid::{triangulate, with_holes, Solid, SolidOwner, P3};

/// What a part of a pool is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PoolPartKind {
    /// The concrete floor of the shell.
    Floor,
    /// The concrete walls of the shell.
    Walls,
    /// The tiles, stone or paint on its inside faces.
    Lining,
    /// The stone on top of its walls.
    Coping,
    /// The water.
    Water,
}

/// A part of a pool: a closed mesh of one material.
#[derive(Debug, Clone)]
pub struct PoolPart {
    pub kind: PoolPartKind,
    pub material: MaterialId,
    pub triangles: Vec<[P3; 3]>,
}

/// How thick the lining is.
const LINING: f64 = 0.02;
/// How thick the coping is, and how far it reaches over the water.
const COPING_THICK: f64 = 0.05;
const COPING_OVER: f64 = 0.03;

/// A prism over a polygon (counter-clockwise), its bottom and top at each
/// corner as the functions say.
fn prism(
    poly: &[Point2],
    bottom: &dyn Fn(Point2) -> f64,
    top: &dyn Fn(Point2) -> f64,
) -> Vec<[P3; 3]> {
    let lo = |p: Point2| [p.x, p.y, bottom(p)];
    let hi = |p: Point2| [p.x, p.y, top(p)];
    let mut out = Vec::new();
    for [a, b, c] in triangulate(poly) {
        out.push([hi(poly[a]), hi(poly[b]), hi(poly[c])]);
        out.push([lo(poly[a]), lo(poly[c]), lo(poly[b])]);
    }
    let n = poly.len();
    for i in 0..n {
        let (a, b) = (poly[i], poly[(i + 1) % n]);
        out.push([lo(a), lo(b), hi(b)]);
        out.push([lo(a), hi(b), hi(a)]);
    }
    out
}

/// A ring between two loops of the same corners (`inner` inside
/// `outer`, both counter-clockwise), its bottom and top at each corner as
/// the functions say.
fn ring(
    inner: &[Point2],
    outer: &[Point2],
    bottom: &dyn Fn(Point2) -> f64,
    top: &dyn Fn(Point2) -> f64,
) -> Vec<[P3; 3]> {
    let lo = |p: Point2| [p.x, p.y, bottom(p)];
    let hi = |p: Point2| [p.x, p.y, top(p)];
    let n = inner.len().min(outer.len());
    let mut out = Vec::new();
    for i in 0..n {
        let j = (i + 1) % n;
        let (a, b, c, d) = (inner[i], outer[i], outer[j], inner[j]);
        // Top and bottom.
        out.push([hi(a), hi(b), hi(c)]);
        out.push([hi(a), hi(c), hi(d)]);
        out.push([lo(a), lo(c), lo(b)]);
        out.push([lo(a), lo(d), lo(c)]);
        // The outside face, looking out.
        out.push([lo(b), lo(c), hi(c)]);
        out.push([lo(b), hi(c), hi(b)]);
        // The inside face, looking in.
        out.push([lo(d), lo(a), hi(a)]);
        out.push([lo(d), hi(a), hi(d)]);
    }
    out
}

/// The parts of a pool, in model space.
pub fn pool_parts(model: &Model, id: PoolId) -> Vec<PoolPart> {
    let Some(p) = model.pool(id) else {
        return Vec::new();
    };
    let z0 = model.level(p.level).map_or(0.0, |l| l.elevation);
    let top = z0 + p.top_offset;
    let water = z0 + p.water_level();
    let floor = |q: Point2| z0 + p.floor_at(q);
    let l0 = p.outline.clone();
    let lining = LINING.min(p.wall / 2.0);
    let off = |d: f64| archisim_core::offset_contour(&l0, d);
    let (Some(l1), Some(l2)) = (off(lining), off(p.wall)) else {
        return Vec::new();
    };
    // The walls go up to the coping, or to the top without one.
    let wall_top = if p.coping > 0.0 {
        top - COPING_THICK
    } else {
        top
    };
    let mut out = Vec::new();
    let part = |kind, material, triangles| PoolPart {
        kind,
        material,
        triangles,
    };
    out.push(part(
        PoolPartKind::Floor,
        p.shell,
        prism(&l1, &|q| floor(q) - p.wall, &|q| floor(q) - lining),
    ));
    out.push(part(
        PoolPartKind::Walls,
        p.shell,
        ring(&l1, &l2, &|q| floor(q) - p.wall, &|_| wall_top),
    ));
    let mut skin = ring(&l0, &l1, &|q| floor(q) - lining, &|_| wall_top);
    skin.extend(prism(&l0, &|q| floor(q) - lining, &floor));
    out.push(part(PoolPartKind::Lining, p.lining, skin));
    if p.coping > 0.0 {
        let cin = off(-COPING_OVER).unwrap_or_else(|| l0.clone());
        if let Some(cout) = off(p.coping.max(p.wall)) {
            out.push(part(
                PoolPartKind::Coping,
                p.coping_material,
                ring(&cin, &cout, &|_| top - COPING_THICK, &|_| top),
            ));
        }
    }
    out.push(part(
        PoolPartKind::Water,
        p.water,
        prism(&l0, &floor, &|_| water),
    ));
    out
}

/// The solids of a pool, for the 3D view and the drawings.
pub fn pool_solids(model: &Model, id: PoolId) -> Vec<Solid> {
    pool_parts(model, id)
        .into_iter()
        .map(|part| Solid {
            owner: SolidOwner::Pool(id),
            material: part.material,
            triangles: part.triangles,
        })
        .collect()
}

/// The box round a polygon in plan.
fn bounds(poly: &[Point2]) -> (Point2, Point2) {
    poly.iter().fold(
        (
            Point2::new(f64::MAX, f64::MAX),
            Point2::new(f64::MIN, f64::MIN),
        ),
        |(lo, hi), p| {
            (
                Point2::new(lo.x.min(p.x), lo.y.min(p.y)),
                Point2::new(hi.x.max(p.x), hi.y.max(p.y)),
            )
        },
    )
}

/// Whether two boxes overlap.
fn overlap(a: (Point2, Point2), b: (Point2, Point2)) -> bool {
    a.0.x <= b.1.x && b.0.x <= a.1.x && a.0.y <= b.1.y && b.0.y <= a.1.y
}

/// A surface of triangles (counter-clockwise from above) less the
/// excavations: its triangles that do not touch them as they were, the
/// others cut, each piece still on its triangle's plane.
pub fn cut_surface(surface: &[[P3; 3]], holes: &[Excavation]) -> Vec<[P3; 3]> {
    if holes.is_empty() {
        return surface.to_vec();
    }
    let shapes: Vec<Shape> = holes
        .iter()
        .map(|h| (h.outline.clone(), Vec::new()))
        .collect();
    let boxes: Vec<_> = holes.iter().map(|h| bounds(&h.outline)).collect();
    let mut out = Vec::new();
    for t in surface {
        let poly: Vec<Point2> = t.iter().map(|q| Point2::new(q[0], q[1])).collect();
        let b = bounds(&poly);
        let near: Vec<Shape> = boxes
            .iter()
            .zip(&shapes)
            .filter(|(hb, _)| overlap(b, **hb))
            .map(|(_, s)| s.clone())
            .collect();
        if near.is_empty() {
            out.push(*t);
            continue;
        }
        // The plane of the triangle, to put the pieces' corners back on it.
        let [a, bb, c] = *t;
        let (u, v) = (
            [bb[0] - a[0], bb[1] - a[1], bb[2] - a[2]],
            [c[0] - a[0], c[1] - a[1], c[2] - a[2]],
        );
        let det = u[0] * v[1] - u[1] * v[0];
        if det.abs() < 1e-12 {
            continue;
        }
        let z_at = |p: Point2| {
            let (dx, dy) = (p.x - a[0], p.y - a[1]);
            let s = (dx * v[1] - dy * v[0]) / det;
            let r = (u[0] * dy - u[1] * dx) / det;
            a[2] + s * u[2] + r * v[2]
        };
        for (outer, inner) in difference(&[(poly.clone(), Vec::new())], &near) {
            let piece = with_holes(&outer, &inner);
            for [i, j, k] in triangulate(&piece) {
                let lift = |p: Point2| [p.x, p.y, z_at(p)];
                out.push([lift(piece[i]), lift(piece[j]), lift(piece[k])]);
            }
        }
    }
    out
}

/// The faces that close a cut surface over the holes it was cut by: down
/// from its open edges to each hole's bottom, and the hole's floor (the
/// part of it inside `boundary`, the ground's outline).
pub fn hole_faces(cut: &[[P3; 3]], holes: &[Excavation], boundary: &[Point2]) -> Vec<[P3; 3]> {
    use std::collections::HashMap;
    let key = |p: P3| ((p[0] * 1e5).round() as i64, (p[1] * 1e5).round() as i64);
    type Key = (i64, i64);
    let mut edges: HashMap<(Key, Key), (P3, P3)> = HashMap::new();
    for t in cut {
        for k in 0..3 {
            let (a, b) = (t[k], t[(k + 1) % 3]);
            let (ka, kb) = (key(a), key(b));
            // An edge and its reverse cancel: inside edges are shared.
            if edges.remove(&(kb, ka)).is_none() {
                edges.insert((ka, kb), (a, b));
            }
        }
    }
    let on = |p: Point2, poly: &[Point2]| {
        let n = poly.len();
        (0..n).any(|i| {
            let (a, b) = (poly[i], poly[(i + 1) % n]);
            let ab = b - a;
            let l2 = ab.x * ab.x + ab.y * ab.y;
            if l2 < 1e-18 {
                return false;
            }
            let t = (((p - a).x * ab.x + (p - a).y * ab.y) / l2).clamp(0.0, 1.0);
            p.distance(a + ab * t) < 1e-4
        })
    };
    let mut out = Vec::new();
    for (a, b) in edges.values() {
        let mid = Point2::new((a[0] + b[0]) / 2.0, (a[1] + b[1]) / 2.0);
        let Some(h) = holes.iter().find(|h| on(mid, &h.outline)) else {
            continue;
        };
        let low = |p: P3| [p[0], p[1], h.bottom.min(p[2])];
        // The earth is to the left of a → b: the face looks into the hole.
        out.push([low(*a), low(*b), *b]);
        out.push([low(*a), *b, *a]);
    }
    let ground: Vec<Shape> = vec![(boundary.to_vec(), Vec::new())];
    for h in holes {
        for (outer, inner) in intersection(&[(h.outline.clone(), Vec::new())], &ground) {
            let piece = with_holes(&outer, &inner);
            let at = |p: Point2| [p.x, p.y, h.bottom];
            for [i, j, k] in triangulate(&piece) {
                out.push([at(piece[i]), at(piece[j]), at(piece[k])]);
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use archisim_core::{CutPattern, NewPool};

    fn model_with_pool() -> (Model, PoolId) {
        let mut m = Model::new("Piscina");
        let level = m.add_level("Planta baja", 0.0);
        let mat = m.add_material("Gresite", "ceramic", [80, 160, 200], CutPattern::Solid);
        let id = m
            .add_pool(NewPool::new(
                level,
                vec![
                    Point2::new(0.0, 0.0),
                    Point2::new(8.0, 0.0),
                    Point2::new(8.0, 4.0),
                    Point2::new(0.0, 4.0),
                ],
                mat,
            ))
            .unwrap();
        (m, id)
    }

    fn volume(t: &[[P3; 3]]) -> f64 {
        t.iter()
            .map(|[a, b, c]| {
                let cr = [
                    b[1] * c[2] - b[2] * c[1],
                    b[2] * c[0] - b[0] * c[2],
                    b[0] * c[1] - b[1] * c[0],
                ];
                (a[0] * cr[0] + a[1] * cr[1] + a[2] * cr[2]) / 6.0
            })
            .sum()
    }

    #[test]
    fn the_water_fills_the_sloped_shell() {
        let (m, id) = model_with_pool();
        let parts = pool_parts(&m, id);
        assert_eq!(parts.len(), 5);
        let water = parts
            .iter()
            .find(|p| p.kind == PoolPartKind::Water)
            .unwrap();
        // 32 m² from 1,0 to 2,0 m deep.
        assert!((volume(&water.triangles) - 48.0).abs() < 1e-6);
        for p in &parts {
            assert!(volume(&p.triangles) > 0.0, "{:?}", p.kind);
        }
        // The shell's floor under the deep end.
        let low = parts
            .iter()
            .flat_map(|p| p.triangles.iter().flatten())
            .map(|q| q[2])
            .fold(f64::MAX, f64::min);
        assert!((low - (-0.12 - 2.0 - 0.25)).abs() < 1e-9);
    }

    #[test]
    fn the_ground_is_cut_and_closed_round_the_hole() {
        // A flat square of ground, 20 × 20 m, with a hole of 4 × 2 m.
        let a = [-10.0, -10.0, 0.0];
        let b = [10.0, -10.0, 0.0];
        let c = [10.0, 10.0, 0.0];
        let d = [-10.0, 10.0, 0.0];
        let surface = vec![[a, b, c], [a, c, d]];
        let hole = Excavation {
            outline: vec![
                Point2::new(-2.0, -1.0),
                Point2::new(2.0, -1.0),
                Point2::new(2.0, 1.0),
                Point2::new(-2.0, 1.0),
            ],
            bottom: -2.0,
        };
        let cut = cut_surface(&surface, std::slice::from_ref(&hole));
        let area: f64 = cut
            .iter()
            .map(|[p, q, r]| ((q[0] - p[0]) * (r[1] - p[1]) - (q[1] - p[1]) * (r[0] - p[0])) / 2.0)
            .sum();
        assert!((area - (400.0 - 8.0)).abs() < 1e-6, "{area}");
        let boundary = vec![
            Point2::new(-10.0, -10.0),
            Point2::new(10.0, -10.0),
            Point2::new(10.0, 10.0),
            Point2::new(-10.0, 10.0),
        ];
        let faces = hole_faces(&cut, &[hole], &boundary);
        // Four walls of two triangles (the cut may split them) and a floor.
        assert!(faces.len() >= 10, "{}", faces.len());
        // With the hole's faces the cut surface over a box encloses the
        // hole's volume, taken away: the faces add −(8 m² × 2 m).
        let mut closed = cut.clone();
        closed.extend(faces);
        // The flat surface at z = 0 adds nothing; the hole adds its volume
        // with a minus sign (its faces look into it).
        assert!((volume(&closed) + 16.0).abs() < 1e-6, "{}", volume(&closed));
    }
}
