//! 3D solids of the model as closed triangle meshes, for the 3D view.
//!
//! The shapes are the same ones written to IFC, so the 3D view shows what
//! Revit and ArchiCAD will receive:
//! - walls: their layers between base and top, with the rough openings of
//!   their doors and windows taken out, as one skin per wall;
//! - slabs: their layers hanging down from the top face, as one skin;
//! - doors and windows: the 3D body of their family (for the system
//!   families: lining or frame, closed leaves, mullions and glazing).
//!
//! Triangles are wound counter-clockwise seen from outside, so normals
//! point out. The solids of one element together enclose its volume (walls
//! joined at a mitre leave the shared corner face to each other).

use archisim_core::{
    BeamId, ColumnId, LevelId, MaterialId, Model, OpeningId, Point2, SlabId, WallId, WallSide,
    EPSILON,
};

use crate::{
    ensure_ccw, opening_geometry, wall_geometry_in, Curve, EndCut, Line, WallGeometry, WallIndex,
};

pub use crate::stairs::{
    railing_bars, railing_material, railing_pieces, stair_parts, stair_solids,
};

/// A point in model space: metres, Z up.
pub type P3 = [f64; 3];

/// The element a solid belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum SolidOwner {
    Wall(WallId),
    Slab(SlabId),
    Opening(OpeningId),
    Column(ColumnId),
    Beam(BeamId),
    Roof(archisim_core::RoofId),
    /// A dormer or roof window: its roof and its index there.
    Dormer(archisim_core::RoofId, u16),
    Stair(archisim_core::StairId),
    Terrain(archisim_core::TerrainId),
    CurtainWall(archisim_core::CurtainWallId),
    Railing(archisim_core::RailingId),
    Object(archisim_core::ObjectId),
    Footing(archisim_core::FootingId),
    /// A volume of the concept phase.
    Mass(archisim_core::MassId),
    Road(archisim_core::RoadId),
    /// A pipe or duct.
    Run(archisim_core::RunId),
    Device(archisim_core::DeviceId),
    /// The grass and flowers on a terrain's painted areas.
    Growth(archisim_core::TerrainId),
    /// A pool built in the ground.
    Pool(archisim_core::PoolId),
    /// A lift: its shaft, car, rails and doors (H4).
    Elevator(archisim_core::ElevatorId),
    /// The pieces of a wall's façade (E9): bricks, blocks, panels.
    Facade(WallId),
}

impl SolidOwner {
    /// The id of the element.
    pub fn id(self) -> u64 {
        match self {
            SolidOwner::Wall(x) | SolidOwner::Facade(x) => x.0,
            SolidOwner::Slab(x) => x.0,
            SolidOwner::Opening(x) => x.0,
            SolidOwner::Column(x) => x.0,
            SolidOwner::Beam(x) => x.0,
            SolidOwner::Roof(x) => x.0,
            SolidOwner::Dormer(x, _) => x.0,
            SolidOwner::Stair(x) => x.0,
            SolidOwner::Terrain(x) => x.0,
            SolidOwner::Growth(x) => x.0,
            SolidOwner::CurtainWall(x) => x.0,
            SolidOwner::Railing(x) => x.0,
            SolidOwner::Object(x) => x.0,
            SolidOwner::Footing(x) => x.0,
            SolidOwner::Pool(x) => x.0,
            SolidOwner::Elevator(x) => x.0,
            SolidOwner::Mass(x) => x.0,
            SolidOwner::Road(x) => x.0,
            SolidOwner::Run(x) => x.0,
            SolidOwner::Device(x) => x.0,
        }
    }
}

/// One closed piece of one material.
#[derive(Debug, Clone, PartialEq)]
pub struct Solid {
    pub owner: SolidOwner,
    pub material: MaterialId,
    pub triangles: Vec<[P3; 3]>,
}

impl Solid {
    /// Enclosed volume (divergence theorem); exact for closed meshes.
    pub fn volume(&self) -> f64 {
        self.triangles
            .iter()
            .map(|[a, b, c]| dot(*a, cross(*b, *c)) / 6.0)
            .sum()
    }
}

fn cross(a: P3, b: P3) -> P3 {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

fn dot(a: P3, b: P3) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

/// Ear-clipping triangulation of a simple counter-clockwise polygon.
/// Returns index triples, counter-clockwise.
pub fn triangulate(poly: &[Point2]) -> Vec<[usize; 3]> {
    let n = poly.len();
    if n < 3 {
        return Vec::new();
    }
    let mut idx: Vec<usize> = (0..n).collect();
    let mut out = Vec::with_capacity(n - 2);
    let area2 = |a: Point2, b: Point2, c: Point2| (b - a).cross(c - a);
    let inside = |p: Point2, a: Point2, b: Point2, c: Point2| {
        area2(a, b, p) >= -1e-12 && area2(b, c, p) >= -1e-12 && area2(c, a, p) >= -1e-12
    };
    let mut guard = 0;
    while idx.len() > 3 && guard < 10 * n * n {
        guard += 1;
        let m = idx.len();
        let mut clipped = false;
        for i in 0..m {
            let (ia, ib, ic) = (idx[(i + m - 1) % m], idx[i], idx[(i + 1) % m]);
            let (a, b, c) = (poly[ia], poly[ib], poly[ic]);
            if area2(a, b, c) <= 1e-14 {
                // Reflex or degenerate corner: not an ear. A collinear point
                // can simply be dropped.
                if area2(a, b, c).abs() <= 1e-14 {
                    idx.remove(i);
                    clipped = true;
                    break;
                }
                continue;
            }
            let blocked = idx.iter().any(|&j| {
                j != ia && j != ib && j != ic && {
                    let p = poly[j];
                    !p.approx_eq(a) && !p.approx_eq(b) && !p.approx_eq(c) && inside(p, a, b, c)
                }
            });
            if !blocked {
                out.push([ia, ib, ic]);
                idx.remove(i);
                clipped = true;
                break;
            }
        }
        if !clipped {
            break;
        }
    }
    if idx.len() == 3 {
        let (a, b, c) = (poly[idx[0]], poly[idx[1]], poly[idx[2]]);
        if area2(a, b, c).abs() > 1e-14 {
            out.push([idx[0], idx[1], idx[2]]);
        }
    }
    out
}

/// One polygon from an outline and the holes inside it, for
/// [`triangulate`]: each hole is joined to the outline by a cut going there
/// and back (a "bridge") from its rightmost corner to a corner it can see.
pub fn with_holes(outer: &[Point2], holes: &[Vec<Point2>]) -> Vec<Point2> {
    let mut poly = ensure_ccw(outer.to_vec());
    let mut holes: Vec<Vec<Point2>> = holes
        .iter()
        .map(|h| {
            let mut h = ensure_ccw(h.clone());
            h.reverse();
            h
        })
        .collect();
    // Rightmost holes first, so later bridges see the earlier ones.
    let right = |h: &Vec<Point2>| h.iter().map(|p| p.x).fold(f64::NEG_INFINITY, f64::max);
    holes.sort_by(|a, b| right(b).total_cmp(&right(a)));
    for (k, hole) in holes.iter().enumerate() {
        let Some(mi) = (0..hole.len()).max_by(|a, b| hole[*a].x.total_cmp(&hole[*b].x)) else {
            continue;
        };
        let m = hole[mi];
        let edges = |pts: &[Point2]| -> Vec<(Point2, Point2)> {
            (0..pts.len())
                .map(|i| (pts[i], pts[(i + 1) % pts.len()]))
                .collect()
        };
        let mut all_edges = edges(&poly);
        for h in &holes[k..] {
            all_edges.extend(edges(h));
        }
        let crosses = |a: Point2, b: Point2| {
            all_edges.iter().any(|(p, q)| {
                let touches_end = |x: Point2| x.approx_eq(a) || x.approx_eq(b);
                if touches_end(*p) || touches_end(*q) {
                    return false;
                }
                let d1 = (b - a).cross(*p - a);
                let d2 = (b - a).cross(*q - a);
                let d3 = (*q - *p).cross(a - *p);
                let d4 = (*q - *p).cross(b - *p);
                d1 * d2 < 0.0 && d3 * d4 < 0.0
            })
        };
        let mut candidates: Vec<usize> = (0..poly.len()).collect();
        candidates.sort_by(|a, b| poly[*a].distance(m).total_cmp(&poly[*b].distance(m)));
        let Some(pi) = candidates.into_iter().find(|&i| !crosses(m, poly[i])) else {
            continue;
        };
        let mut joined = Vec::with_capacity(poly.len() + hole.len() + 2);
        joined.extend_from_slice(&poly[..=pi]);
        for j in 0..=hole.len() {
            joined.push(hole[(mi + j) % hole.len()]);
        }
        joined.push(poly[pi]);
        joined.extend_from_slice(&poly[pi + 1..]);
        poly = joined;
    }
    poly
}

/// A vertical prism: a plan polygon (any direction) between two heights.
pub fn prism(poly: &[Point2], z0: f64, z1: f64) -> Vec<[P3; 3]> {
    if poly.len() < 3 || z1 - z0 <= EPSILON {
        return Vec::new();
    }
    let poly = ensure_ccw(poly.to_vec());
    let n = poly.len();
    let at = |p: Point2, z: f64| [p.x, p.y, z];
    let mut tris = Vec::new();
    for [a, b, c] in triangulate(&poly) {
        // Top seen from above: counter-clockwise. Bottom: reversed.
        tris.push([at(poly[a], z1), at(poly[b], z1), at(poly[c], z1)]);
        tris.push([at(poly[a], z0), at(poly[c], z0), at(poly[b], z0)]);
    }
    for i in 0..n {
        let (p, q) = (poly[i], poly[(i + 1) % n]);
        if p.approx_eq(q) {
            continue;
        }
        // Outside is to the right of p → q for a counter-clockwise polygon.
        tris.push([at(p, z0), at(q, z0), at(q, z1)]);
        tris.push([at(p, z0), at(q, z1), at(p, z1)]);
    }
    tris
}

/// A prism whose top and bottom follow the ground: each point of the plan
/// polygon between its own two heights, `z(p) = (bottom, top)`.
pub fn draped_prism(poly: &[Point2], z: impl Fn(Point2) -> (f64, f64)) -> Vec<[P3; 3]> {
    if poly.len() < 3 {
        return Vec::new();
    }
    let poly = ensure_ccw(poly.to_vec());
    let n = poly.len();
    let zs: Vec<(f64, f64)> = poly.iter().map(|p| z(*p)).collect();
    if zs.iter().all(|(a, b)| b - a <= EPSILON) {
        return Vec::new();
    }
    let at = |i: usize, top: bool| {
        let p = poly[i];
        [p.x, p.y, if top { zs[i].1 } else { zs[i].0 }]
    };
    let mut tris = Vec::new();
    for [a, b, c] in triangulate(&poly) {
        tris.push([at(a, true), at(b, true), at(c, true)]);
        tris.push([at(a, false), at(c, false), at(b, false)]);
    }
    for i in 0..n {
        let j = (i + 1) % n;
        if poly[i].approx_eq(poly[j]) {
            continue;
        }
        tris.push([at(i, false), at(j, false), at(j, true)]);
        tris.push([at(i, false), at(j, true), at(i, true)]);
    }
    tris
}

/// The wall filled back in over an arch (or round an oculus), across one
/// layer from `y0` to `y1`: its outline in `x` along the wall and `z` up,
/// each point marked when it is on the shape (not on the box). Faces on
/// the sides `faces` asks for (at `y0`, at `y1`: those that are faces of
/// the wall) and along the shape only: along the box it meets the rest of
/// the wall, which leaves no face there either.
#[allow(clippy::too_many_arguments)]
fn infill_prism(
    poly: &[([f64; 2], bool)],
    y0: f64,
    y1: f64,
    faces: (bool, bool),
    at: impl Fn(f64, f64, f64) -> P3,
    n: archisim_core::Vec2,
) -> Vec<[P3; 3]> {
    let pts: Vec<Point2> = poly.iter().map(|(q, _)| Point2::new(q[0], q[1])).collect();
    let ccw = archisim_core::shape::area(&poly.iter().map(|(q, _)| *q).collect::<Vec<_>>()) > 0.0;
    let world = |q: [f64; 2], y: f64| at(q[0], y, q[1]);
    let normal = |t: &[P3; 3]| {
        let (a, b) = (sub3(t[1], t[0]), sub3(t[2], t[0]));
        [
            a[1] * b[2] - a[2] * b[1],
            a[2] * b[0] - a[0] * b[2],
            a[0] * b[1] - a[1] * b[0],
        ]
    };
    let facing = |t: [P3; 3], want: [f64; 3]| {
        let m = normal(&t);
        if m[0] * want[0] + m[1] * want[1] + m[2] * want[2] < 0.0 {
            [t[0], t[2], t[1]]
        } else {
            t
        }
    };
    let mut out = Vec::new();
    let nn = [n.x, n.y, 0.0];
    for [i, j, k] in triangulate(&pts) {
        let (p, q, r) = (poly[i].0, poly[j].0, poly[k].0);
        // Only on the faces of the wall (between layers, the cells leave
        // none either).
        if faces.1 {
            out.push(facing([world(p, y1), world(q, y1), world(r, y1)], nn));
        }
        if faces.0 {
            out.push(facing(
                [world(p, y0), world(q, y0), world(r, y0)],
                [-nn[0], -nn[1], 0.0],
            ));
        }
    }
    let m = poly.len();
    for i in 0..m {
        let ((p, on_p), (q, on_q)) = (poly[i], poly[(i + 1) % m]);
        if !(on_p && on_q) || (i + 1) % m == 0 {
            continue;
        }
        // Outwards from the outline, in the plane of the wall.
        let (dx, dz) = (q[0] - p[0], q[1] - p[1]);
        let (ox, oz) = if ccw { (dz, -dx) } else { (-dz, dx) };
        let a = world([0.0, 0.0], y0);
        let ux = sub3(world([1.0, 0.0], y0), a);
        let want = [ux[0] * ox, ux[1] * ox, oz];
        let (a0, b0, a1, b1) = (world(p, y0), world(q, y0), world(p, y1), world(q, y1));
        out.push(facing([a0, b0, b1], want));
        out.push(facing([a0, b1, a1], want));
    }
    out
}

fn sub3(a: P3, b: P3) -> P3 {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

/// Convex pieces of one outline (in `x` across an opening and `z` up),
/// side by side, as one prism from `y0` to `y1`: their faces on both
/// sides and round the outside of the whole, none between two pieces.
/// Each point is put in place by `at(x, y, z)`; `det` is the sign of that
/// placing.
pub fn profile_group(
    pieces: &[&Vec<[f64; 2]>],
    y0: f64,
    y1: f64,
    det: f64,
    at: impl Fn(f64, f64, f64) -> P3,
) -> Vec<[P3; 3]> {
    let key = |q: [f64; 2]| ((q[0] * 1e6).round() as i64, (q[1] * 1e6).round() as i64);
    let ccw: Vec<Vec<[f64; 2]>> = pieces
        .iter()
        .filter(|p| p.len() >= 3)
        .map(|p| {
            if archisim_core::shape::area(p) < 0.0 {
                p.iter().rev().copied().collect()
            } else {
                p.to_vec()
            }
        })
        .collect();
    type Edge = ((i64, i64), (i64, i64));
    let mut count: std::collections::HashMap<Edge, usize> = Default::default();
    for p in &ccw {
        for i in 0..p.len() {
            let (a, b) = (key(p[i]), key(p[(i + 1) % p.len()]));
            *count
                .entry(if a <= b { (a, b) } else { (b, a) })
                .or_default() += 1;
        }
    }
    let (ya, yb) = (y0.min(y1), y0.max(y1));
    // In the family's axes: (x, y, z); a face looking along `want`.
    let local = |q: [f64; 2], y: f64| [q[0], y, q[1]];
    let facing = |t: [P3; 3], want: [f64; 3]| {
        let (a, b) = (sub3(t[1], t[0]), sub3(t[2], t[0]));
        let m = [
            a[1] * b[2] - a[2] * b[1],
            a[2] * b[0] - a[0] * b[2],
            a[0] * b[1] - a[1] * b[0],
        ];
        if m[0] * want[0] + m[1] * want[1] + m[2] * want[2] < 0.0 {
            [t[0], t[2], t[1]]
        } else {
            t
        }
    };
    let mut out = Vec::new();
    for p in &ccw {
        for i in 1..p.len() - 1 {
            let (q0, q1, q2) = (p[0], p[i], p[i + 1]);
            out.push(facing(
                [local(q0, yb), local(q1, yb), local(q2, yb)],
                [0.0, 1.0, 0.0],
            ));
            out.push(facing(
                [local(q0, ya), local(q1, ya), local(q2, ya)],
                [0.0, -1.0, 0.0],
            ));
        }
        for i in 0..p.len() {
            let (q0, q1) = (p[i], p[(i + 1) % p.len()]);
            let (a, b) = (key(q0), key(q1));
            if count[&(if a <= b { (a, b) } else { (b, a) })] > 1 {
                continue;
            }
            let want = [q1[1] - q0[1], 0.0, -(q1[0] - q0[0])];
            let (a0, b0, a1, b1) = (local(q0, ya), local(q1, ya), local(q0, yb), local(q1, yb));
            out.push(facing([a0, b0, b1], want));
            out.push(facing([a0, b1, a1], want));
        }
    }
    out.into_iter()
        .map(|t| {
            let m = t.map(|p| at(p[0], p[1], p[2]));
            if det < 0.0 {
                [m[0], m[2], m[1]]
            } else {
                m
            }
        })
        .collect()
}

/// A prism of the convex `profile` (in `x` across an opening and `z` up)
/// from `y0` to `y1` across it, each point put in place by `at(x, y, z)`;
/// `det` is the sign of that placing (negative when it mirrors), so that
/// the faces still look out.
pub fn profile_prism(
    profile: &[[f64; 2]],
    y0: f64,
    y1: f64,
    det: f64,
    at: impl Fn(f64, f64, f64) -> P3,
) -> Vec<[P3; 3]> {
    let poly: Vec<Point2> = profile.iter().map(|q| Point2::new(q[0], q[1])).collect();
    // Built as a vertical prism of (x, z) along y, which swaps two axes.
    let flip = det > 0.0;
    prism(&poly, y0.min(y1), y0.max(y1))
        .into_iter()
        .map(|t| {
            let m = t.map(|p| at(p[0], p[2], p[1]));
            if flip {
                [m[0], m[2], m[1]]
            } else {
                m
            }
        })
        .collect()
}

/// A vertical prism with a hole through it (a tube): caps triangulated
/// round the hole, sides outside and inside.
pub fn hollow_prism(outer: &[Point2], hole: Option<&[Point2]>, z0: f64, z1: f64) -> Vec<[P3; 3]> {
    let Some(hole) = hole else {
        return prism(outer, z0, z1);
    };
    if outer.len() < 3 || hole.len() < 3 || z1 - z0 <= EPSILON {
        return Vec::new();
    }
    let at = |p: Point2, z: f64| [p.x, p.y, z];
    let cap = with_holes(outer, &[hole.to_vec()]);
    let mut tris = Vec::new();
    for [a, b, c] in triangulate(&cap) {
        tris.push([at(cap[a], z1), at(cap[b], z1), at(cap[c], z1)]);
        tris.push([at(cap[a], z0), at(cap[c], z0), at(cap[b], z0)]);
    }
    let outer = ensure_ccw(outer.to_vec());
    let mut inner = ensure_ccw(hole.to_vec());
    inner.reverse();
    for ring in [&outer, &inner] {
        let n = ring.len();
        for i in 0..n {
            let (p, q) = (ring[i], ring[(i + 1) % n]);
            if p.approx_eq(q) {
                continue;
            }
            tris.push([at(p, z0), at(q, z0), at(q, z1)]);
            tris.push([at(p, z0), at(q, z1), at(p, z1)]);
        }
    }
    tris
}

/// Solids of one wall, one per layer material, together forming a single
/// skin: the wall is cut into a grid of cells (columns at the jambs of its
/// doors and windows, rows at their sills and heads, slices at its layer
/// boundaries) and only the faces between solid and empty cells are kept.
/// Layers and pieces touching each other inside the wall have no faces, so
/// every visible edge is shared by exactly two faces and nothing flickers.
///
/// Ends joined to another wall with a mitre have no end face either: the
/// other wall closes the corner. A wall with free ends is a closed solid.
pub fn wall_solids(model: &Model, id: WallId) -> Vec<Solid> {
    wall_solids_in(&WallIndex::new(model), id)
}

/// [`wall_solids`] with the walls around found through an index.
pub fn wall_solids_in(idx: &WallIndex, id: WallId) -> Vec<Solid> {
    wall_parts_in(idx, id, false)
}

/// The faces where the layers of a wall touch each other, one solid per
/// layer, each face looking out of its own layer. They are not part of the
/// wall's skin (which is one closed solid): the 3D view adds them so that
/// a cut through the wall shows each layer in its own cut colour instead
/// of the far side of the wall (it draws them only seen from inside, as
/// the cut).
pub fn wall_inner_faces(idx: &WallIndex, id: WallId) -> Vec<Solid> {
    wall_parts_in(idx, id, true)
}

use archisim_core::MassMesh;

fn wall_parts_in(idx: &WallIndex, id: WallId, between: bool) -> Vec<Solid> {
    let parts = wall_parts_plain(idx, id, between);
    let model = idx.model;
    let Some(wall) = model.wall(id) else {
        return parts;
    };
    let Some(profile) = wall.profile.as_ref().filter(|_| !wall.is_curved()) else {
        return parts;
    };
    let level = model.level(wall.level).map_or(0.0, |l| l.elevation);
    let frame = ProfileFrame {
        origin: wall.start,
        u: (wall.end - wall.start)
            .normalized()
            .unwrap_or(Point2::new(1.0, 0.0)),
        z0: level + wall.base_offset,
    };
    if between {
        // Faces between layers: those whose middle is in the wall.
        return parts
            .into_iter()
            .map(|mut s| {
                s.triangles.retain(|t| {
                    let c = [
                        (t[0][0] + t[1][0] + t[2][0]) / 3.0,
                        (t[0][1] + t[1][1] + t[2][1]) / 3.0,
                        (t[0][2] + t[1][2] + t[2][2]) / 3.0,
                    ];
                    frame.inside(profile, c)
                });
                s
            })
            .filter(|s| !s.triangles.is_empty())
            .collect();
    }
    let Some(wtype) = model.wall_type(wall.wall_type) else {
        return parts;
    };
    let Some(g) = wall_geometry_in(idx, id) else {
        return parts;
    };
    let Some(owner) = parts.first().map(|s| s.owner) else {
        return parts;
    };
    let reach = wtype.thickness() + 2.0;
    let top = frame.z0 + wall.height.max(profile.top());
    // Each layer as a closed solid (its band of the wall's outline, up its
    // height), cut by the profile, its holes and the doors and windows.
    // (The joins with slabs and roofs are left aside.)
    let keep = frame.prism(
        &crate::junctions::profile_with_ends(profile, wall.length(), reach),
        reach,
    );
    let mut cutters: Vec<MassMesh> = profile
        .holes
        .iter()
        .map(|h| frame.prism(h, reach))
        .collect();
    for o in idx.openings_in_wall(id) {
        let (t0, t1) = o.cut_span();
        let (w, h) = (t1 - t0, o.head - o.sill);
        let sill = o.sill - wall.base_offset;
        let poly: Vec<Point2> = if o.shape.is_rect() {
            vec![
                Point2::new(t0, sill),
                Point2::new(t1, sill),
                Point2::new(t1, sill + h),
                Point2::new(t0, sill + h),
            ]
        } else {
            o.shape
                .polygon(w, h)
                .into_iter()
                .map(|q| Point2::new(t0 + w / 2.0 + q[0], sill + q[1]))
                .collect()
        };
        cutters.push(frame.prism(&poly, reach));
    }
    let n = frame.u.perp_left();
    let mut out = Vec::new();
    let mut at = g.exterior_offset;
    for l in &wtype.layers {
        let (lo, hi) = (at - l.thickness, at);
        at = lo;
        let band = band_of(&g.outline, frame.origin, n, lo, hi);
        if band.len() < 3 {
            continue;
        }
        let solid = mass_mesh_of(&prism(&band, frame.z0, top));
        let Some(mut piece) = crate::csg::combine(&solid, &keep, 0.0, 1, crate::csg::Op::Intersect)
        else {
            continue;
        };
        let mut gone = false;
        for c in &cutters {
            match crate::csg::combine(&piece, c, 0.0, 1, crate::csg::Op::Subtract) {
                Some(p) => piece = p,
                None => {
                    gone = true;
                    break;
                }
            }
        }
        if gone {
            continue;
        }
        let triangles: Vec<[P3; 3]> = piece
            .triangles
            .iter()
            .map(|t| t.map(|i| piece.positions[i as usize]))
            .collect();
        if !triangles.is_empty() {
            out.push(Solid {
                owner,
                material: l.material,
                triangles,
            });
        }
    }
    out
}

/// The part of `outline` between `lo` and `hi` across the line through
/// `origin` with normal `n`.
fn band_of(outline: &[Point2], origin: Point2, n: Point2, lo: f64, hi: f64) -> Vec<Point2> {
    let mut band = outline.to_vec();
    for (limit, keep_above) in [(lo, true), (hi, false)] {
        let g = |p: Point2| {
            let d = (p - origin).dot(n) - limit;
            if keep_above {
                d
            } else {
                -d
            }
        };
        let m = band.len();
        let mut next = Vec::new();
        for i in 0..m {
            let (p, q) = (band[i], band[(i + 1) % m]);
            let (gp, gq) = (g(p), g(q));
            if gp >= -1e-9 {
                next.push(p);
            }
            if (gp > 1e-9 && gq < -1e-9) || (gp < -1e-9 && gq > 1e-9) {
                next.push(p + (q - p) * (gp / (gp - gq)));
            }
        }
        band = next;
        if band.len() < 3 {
            return Vec::new();
        }
    }
    band.dedup_by(|a, b| a.approx_eq(*b));
    band
}

/// A wall's axes for its elevation profile: along it from its start, and
/// up from its base.
struct ProfileFrame {
    origin: Point2,
    u: Point2,
    z0: f64,
}

impl ProfileFrame {
    fn to_profile(&self, p: P3) -> Point2 {
        let d = Point2::new(p[0], p[1]) - self.origin;
        Point2::new(d.dot(self.u), p[2] - self.z0)
    }

    fn inside(&self, profile: &archisim_core::WallProfile, p: P3) -> bool {
        let q = self.to_profile(p);
        point_in(q, &profile.outline) && !profile.holes.iter().any(|h| point_in(q, h))
    }

    /// `poly` (in the profile's axes) as a solid through the wall, `reach`
    /// to each side of its line.
    fn prism(&self, poly: &[Point2], reach: f64) -> MassMesh {
        let n = self.u.perp_left();
        let world = |q: Point2, w: f64| -> [f64; 3] {
            let p = self.origin + self.u * q.x + n * w;
            [p.x, p.y, self.z0 + q.y]
        };
        let mut poly = poly.to_vec();
        if archisim_core::signed_area(&poly) < 0.0 {
            poly.reverse();
        }
        let m = poly.len() as u32;
        let mut mesh = MassMesh::default();
        for w in [-reach, reach] {
            for q in &poly {
                mesh.positions.push(world(*q, w));
            }
        }
        for [i, j, k] in triangulate(&poly) {
            let (i, j, k) = (i as u32, j as u32, k as u32);
            // Outward: at -reach towards -w, at +reach towards +w.
            mesh.triangles.push([i, j, k]);
            mesh.triangles.push([m + i, m + k, m + j]);
            mesh.faces.extend([0, 0]);
        }
        for i in 0..m {
            let j = (i + 1) % m;
            mesh.triangles.push([i, m + j, j]);
            mesh.triangles.push([i, m + i, m + j]);
            mesh.faces.extend([0, 0]);
        }
        mesh
    }
}

fn point_in(p: Point2, poly: &[Point2]) -> bool {
    let n = poly.len();
    let mut inside = false;
    for i in 0..n {
        let (a, b) = (poly[i], poly[(i + 1) % n]);
        if (a.y > p.y) != (b.y > p.y) && p.x < a.x + (p.y - a.y) / (b.y - a.y) * (b.x - a.x) {
            inside = !inside;
        }
    }
    inside
}

/// A closed triangle soup as a mesh with shared corners.
fn mass_mesh_of(triangles: &[[P3; 3]]) -> MassMesh {
    let mut mesh = MassMesh::default();
    let mut index: std::collections::HashMap<[i64; 3], u32> = std::collections::HashMap::new();
    for t in triangles {
        let ids = t.map(|p| {
            let key = p.map(|c| (c * 1e7).round() as i64);
            *index.entry(key).or_insert_with(|| {
                mesh.positions.push(p);
                (mesh.positions.len() - 1) as u32
            })
        });
        if ids[0] != ids[1] && ids[1] != ids[2] && ids[0] != ids[2] {
            mesh.triangles.push(ids);
            mesh.faces.push(0);
        }
    }
    mesh
}

fn wall_parts_plain(idx: &WallIndex, id: WallId, between: bool) -> Vec<Solid> {
    let model = idx.model;
    let (Some(wall), Some(g)) = (model.wall(id), wall_geometry_in(idx, id)) else {
        return Vec::new();
    };
    let Some(wtype) = model.wall_type(wall.wall_type) else {
        return Vec::new();
    };
    let level = model.level(wall.level).map_or(0.0, |l| l.elevation);
    let z_base = level + wall.base_offset;
    // Under a roof, built up to its ridge and then trimmed.
    let planes = if g.arc.is_none() {
        model.wall_trim_planes(id)
    } else {
        Vec::new()
    };
    let z_top = if planes.is_empty() {
        z_base + wall.height
    } else {
        model.wall_top_z(wall)
    };
    if g.arc.is_some() {
        let spans: Vec<(f64, f64, f64, f64)> = idx
            .openings_in_wall(id)
            .iter()
            .map(|o| (o.t0, o.t1, level + o.sill, level + o.head))
            .collect();
        return curved_wall_solids(&g, &offsets_of(&g, wtype), wtype, z_base, z_top, &spans);
    }
    let (a, b) = g.axis;
    let len = a.distance(b);
    let Some(u) = (b - a).normalized() else {
        return Vec::new();
    };
    let n = u.perp_left();
    let openings = idx.openings_in_wall(id);
    let layers = wtype.layers.len();

    // Layer boundaries across the wall, from the exterior face inwards.
    let mut offsets = vec![g.exterior_offset];
    for layer in &wtype.layers {
        let last = *offsets.last().expect("not empty");
        offsets.push(last - layer.thickness);
    }
    let along = |cut: &Curve, off: f64, fallback: f64| -> f64 {
        Curve::Line(Line::new(a + n * off, b - a))
            .intersect_near(cut, a + n * off + u * fallback)
            .map_or(fallback, |p| (p - a).dot(u))
    };

    // Columns: lines across the wall. The ends (a T-junction end has one
    // line per layer), the face of the wall butted against, every jamb and
    // every layer boundary of the walls butting into this one.
    #[derive(Clone)]
    enum Col {
        Start,
        End,
        At(Line),
    }
    let mut inner: Vec<Line> = openings
        .iter()
        .flat_map(|o| {
            let (t0, t1) = o.cut_span();
            [t0, t1]
        })
        .filter(|t| *t > EPSILON && *t < len - EPSILON)
        .map(|t| Line::new(a + u * t, n))
        .collect();
    // An arch or an oculus: a line down its middle too, where the two
    // halves of the wall filled back in over it meet.
    for o in openings
        .iter()
        .filter(|o| !o.shape.is_rect() && o.corner.is_none())
    {
        let t = (o.t0 + o.t1) / 2.0;
        if t > EPSILON && t < len - EPSILON {
            inner.push(Line::new(a + u * t, n));
        }
    }
    for notch in &g.notches {
        inner.extend(notch.boundaries.iter().copied());
    }
    // Wrapping (Revit's «Envolver»): the finish that turns round a free
    // end or into a reveal takes a strip as wide as itself; a line across
    // the wall where that strip ends.
    let (ends_ext, ends_int) = wtype.wrapping_layers(wtype.wrap_ends);
    let (ins_ext, ins_int) = wtype.wrapping_layers(wtype.wrap_inserts);
    let width_of = |l: Option<usize>| l.map_or(0.0, |l| wtype.layers[l].thickness);
    let end_strip = width_of(ends_ext.or(ends_int));
    if end_strip > 0.0 {
        if g.start_cut == EndCut::Square {
            inner.push(Line::new(a + u * end_strip, n));
        }
        if g.end_cut == EndCut::Square {
            inner.push(Line::new(a + u * (len - end_strip), n));
        }
    }
    let reveal_strips: Vec<f64> = [ins_ext, ins_int]
        .into_iter()
        .flatten()
        .map(|l| wtype.layers[l].thickness)
        .collect();
    for o in &openings {
        let (t0, t1) = o.cut_span();
        for f in &reveal_strips {
            inner.push(Line::new(a + u * (t0 - f), n));
            inner.push(Line::new(a + u * (t1 + f), n));
        }
    }
    // Slabs met (see `junctions`): a line across the wall where an edge of
    // the slab crosses it (square to the wall: slabs are drawn square to
    // their walls), and a row at each face of their layers.
    let slab_cuts = crate::junctions::wall_slab_cuts(idx, id);
    let mut slabs_seen: Vec<archisim_core::SlabId> = Vec::new();
    for cut in &slab_cuts {
        if slabs_seen.contains(&cut.slab) {
            continue;
        }
        slabs_seen.push(cut.slab);
        for ring in std::iter::once(&cut.outline).chain(&cut.holes) {
            let m = ring.len();
            for i in 0..m {
                let (p, q) = (ring[i], ring[(i + 1) % m]);
                for k in 0..layers {
                    let off = (offsets[k] + offsets[k + 1]) / 2.0;
                    let o = a + n * off;
                    // Where the edge crosses the line of the layer.
                    let (sp, sq) = ((p - o).cross(u), (q - o).cross(u));
                    if (sp > 0.0) == (sq > 0.0) || (sp - sq).abs() < 1e-12 {
                        continue;
                    }
                    let x = p + (q - p) * (sp / (sp - sq));
                    let t = (x - a).dot(u);
                    if t > EPSILON && t < len - EPSILON {
                        inner.push(Line::new(a + u * t, n));
                    }
                }
            }
        }
    }
    let mid = (offsets[0] + offsets[layers]) / 2.0;
    let mut keyed: Vec<(f64, Line)> = inner
        .into_iter()
        .map(|l| (along(&Curve::Line(l), mid, -1.0), l))
        .filter(|(t, _)| *t > EPSILON && *t < len - EPSILON)
        .collect();
    keyed.sort_by(|x, y| x.0.total_cmp(&y.0));
    keyed.dedup_by(|x, y| (x.0 - y.0).abs() < EPSILON);
    // At a T-junction, the face of the other wall right after the end, so
    // the layers that stop there and those that go in share it.
    let mut cols: Vec<Col> = vec![Col::Start];
    if let EndCut::Tee(t) = &g.start_cut {
        cols.push(Col::At(t.face));
    }
    cols.extend(keyed.into_iter().map(|(_, l)| Col::At(l)));
    if let EndCut::Tee(t) = &g.end_cut {
        cols.push(Col::At(t.face));
    }
    cols.push(Col::End);
    let ncols = cols.len();
    // Position along the wall of column `c` on boundary offset `off`, for
    // layer `k` (only the ends depend on the layer).
    let col_t = |c: usize, off: f64, k: usize| -> f64 {
        match &cols[c] {
            Col::Start => g.start_cut.curve(k).map_or(0.0, |c| along(&c, off, 0.0)),
            Col::End => g.end_cut.curve(k).map_or(len, |c| along(&c, off, len)),
            Col::At(l) => along(&Curve::Line(*l), off, 0.0),
        }
    };

    // Rows: base, top and every sill and head in between, also those of
    // the walls joined at the ends or butting into this one, so both sides
    // of a junction share their vertices.
    let mut zs: Vec<f64> = vec![z_base, z_top];
    let mut heights: Vec<(f64, f64)> = openings.iter().map(|o| (o.sill, o.head)).collect();
    // And a row at the spring of an arch (the middle of an oculus).
    for o in openings
        .iter()
        .filter(|o| !o.shape.is_rect() && o.corner.is_none())
    {
        let z = o.sill + o.shape.spring(o.width(), o.height());
        heights.push((z, z));
    }
    let mut joined = idx.at_end(wall.level, wall.start);
    joined.extend(idx.at_end(wall.level, wall.end));
    joined.sort_by_key(|w| w.id);
    joined.dedup_by_key(|w| w.id);
    for other in joined {
        if other.id != id {
            heights.extend(
                idx.openings_in_wall(other.id)
                    .iter()
                    .map(|o| (o.sill, o.head)),
            );
            for z in [other.base_offset, other.base_offset + other.height] {
                heights.push((z, z));
            }
        }
    }
    for z in g.notch_heights() {
        heights.push((z, z));
    }
    for o in &openings {
        for f in &reveal_strips {
            heights.push((o.sill - f, o.head + f));
        }
    }
    for cut in &slab_cuts {
        for z in [cut.z.0, cut.z.1] {
            heights.push((z - level, z - level));
        }
    }
    for cut in [&g.start_cut, &g.end_cut] {
        if let EndCut::Tee(t) = cut {
            if let Some(h) = model.wall(t.host) {
                heights.push((h.base_offset, h.base_offset + h.height));
            }
        }
    }
    for (sill, head) in heights {
        for z in [level + sill, level + head] {
            if z > z_base + EPSILON && z < z_top - EPSILON {
                zs.push(z);
            }
        }
    }
    zs.sort_by(f64::total_cmp);
    zs.dedup_by(|x, y| (*x - *y).abs() < EPSILON);
    let rows = zs.len() - 1;

    // A cell of one layer is solid unless it has no length, lies inside a
    // door or window, or inside the part another wall takes.
    let solid = |c: usize, r: usize, k: usize| -> bool {
        if c + 1 >= ncols || r >= rows || k >= layers {
            return false;
        }
        let off = (offsets[k] + offsets[k + 1]) / 2.0;
        let (t0, t1) = (col_t(c, off, k), col_t(c + 1, off, k));
        if t1 - t0 <= EPSILON {
            return false;
        }
        let t = (t0 + t1) / 2.0;
        let z = (zs[r] + zs[r + 1]) / 2.0;
        let in_opening = openings.iter().any(|o| {
            let (t0, t1) = o.cut_span();
            t > t0 && t < t1 && z > level + o.sill && z < level + o.head
        });
        let p = a + u * t + n * off;
        let in_notch = g.notches.iter().any(|nt| {
            z > level + nt.z.0 && z < level + nt.z.1 && {
                nt.regions.iter().any(|reg| {
                    let (f, lo, hi) = reg.strip();
                    reg.layers.contains(&k) && f(p) > lo + EPSILON && f(p) < hi - EPSILON
                })
            }
        });
        let in_slab = slab_cuts
            .iter()
            .any(|cut| z > cut.z.0 && z < cut.z.1 && cut.taken.contains(&k) && cut.contains(p));
        !in_opening && !in_notch && !in_slab
    };
    // A cell of a shaped opening's box the wall partly fills back in (over
    // an arch's spring, or round an oculus): its neighbours leave no face
    // towards it (the infill has its own).
    let filled = |c: usize, r: usize, k: usize| -> bool {
        if c + 1 >= ncols || r >= rows || k >= layers {
            return false;
        }
        let off = (offsets[k] + offsets[k + 1]) / 2.0;
        let t = (col_t(c, off, k) + col_t(c + 1, off, k)) / 2.0;
        let z = (zs[r] + zs[r + 1]) / 2.0;
        openings.iter().any(|o| {
            if o.shape.is_rect() || o.corner.is_some() {
                return false;
            }
            let spring = level + o.sill + o.shape.spring(o.width(), o.height());
            let oculus = o.shape.kind == archisim_core::ShapeKind::Oculus;
            t > o.t0
                && t < o.t1
                && z > level + o.sill
                && z < level + o.head
                && (z > spring || oculus)
        })
    };
    // The layer whose material a cell takes: its own, or the finish that
    // wraps over it at a free end or in the reveal of a door or window.
    let wrap_into = |k: usize, ext: Option<usize>, int: Option<usize>| -> Option<usize> {
        match (ext, int) {
            (Some(e), Some(i)) => (k != e && k != i).then_some(e),
            (Some(e), None) => (k != e).then_some(e),
            (None, Some(i)) => (k != i).then_some(i),
            (None, None) => None,
        }
    };
    let owner = |c: usize, r: usize, k: usize| -> usize {
        let off = (offsets[k] + offsets[k + 1]) / 2.0;
        let t = (col_t(c, off, k) + col_t(c + 1, off, k)) / 2.0;
        let z = (zs[r] + zs[r + 1]) / 2.0;
        let at_start = g.start_cut == EndCut::Square && t < end_strip;
        let at_end = g.end_cut == EndCut::Square && t > len - end_strip;
        if end_strip > 0.0 && (at_start || at_end) {
            if let Some(o) = wrap_into(k, ends_ext, ends_int) {
                return o;
            }
        }
        // (Not round an arch or an oculus: the wall filled back in over
        // it keeps its layers.)
        for o in openings.iter().filter(|o| o.shape.is_rect()) {
            let (t0, t1) = o.cut_span();
            let (sill, head) = (level + o.sill, level + o.head);
            // Exterior finish: the layers outside the frame; interior
            // finish: those inside it.
            for (l, outside) in [
                (ins_ext, off > o.frame_exterior + EPSILON),
                (ins_int, off < o.frame_interior - EPSILON),
            ] {
                let Some(l) = l else { continue };
                if !outside || k == l {
                    continue;
                }
                let f = wtype.layers[l].thickness;
                let beside = (t > t0 - f && t < t0) || (t > t1 && t < t1 + f);
                let within = t > t0 - f && t < t1 + f;
                let jamb = beside && z > sill - f && z < head + f;
                let above = within && z > head && z < head + f;
                let below = o.sill > EPSILON && within && z > sill - f && z < sill;
                if jamb || above || below {
                    return l;
                }
            }
        }
        k
    };
    let pt = |t: f64, off: f64, z: f64| -> P3 {
        let p = a + u * t + n * off;
        [p.x, p.y, z]
    };
    // Corner `(c, r)` of layer `k` on boundary `bnd` (k or k + 1).
    let v = |c: usize, r: usize, bnd: usize, k: usize| -> P3 {
        let off = offsets[bnd];
        pt(col_t(c, off, k), off, zs[r])
    };
    let centre = |c: usize, r: usize, k: usize| -> P3 {
        let pts = [
            v(c, r, k, k),
            v(c + 1, r, k, k),
            v(c, r + 1, k + 1, k),
            v(c + 1, r + 1, k + 1, k),
        ];
        let mut m = [0.0; 3];
        for p in pts {
            for i in 0..3 {
                m[i] += p[i] / 4.0;
            }
        }
        m
    };
    let joined_start = g.start_cut.is_mitre();
    let joined_end = g.end_cut.is_mitre();

    let mut tris: Vec<Vec<[P3; 3]>> = vec![Vec::new(); layers];
    let mut inner_tris: Vec<Vec<[P3; 3]>> = vec![Vec::new(); layers];
    for c in 0..ncols - 1 {
        for r in 0..rows {
            for k in 0..layers {
                if !solid(c, r, k) {
                    continue;
                }
                let mine = owner(c, r, k);
                let out = &mut tris[mine];
                let inside = centre(c, r, k);
                // Towards the neighbouring layers (or the faces): the part
                // of this cell's side the neighbour does not cover.
                for (bnd, nb) in [(k, k.checked_sub(1)), (k + 1, Some(k + 1))] {
                    let off = offsets[bnd];
                    let own = (col_t(c, off, k), col_t(c + 1, off, k));
                    let theirs = match nb {
                        Some(j) if j < layers && solid(c, r, j) => {
                            Some((col_t(c, off, j), col_t(c + 1, off, j)))
                        }
                        _ => None,
                    };
                    // The part the neighbour covers: a face between two
                    // layers (only for the cut in the 3D view).
                    let same = nb.is_some_and(|j| j < layers && owner(c, r, j) == mine);
                    if between && !same {
                        if let Some((t0, t1)) = theirs {
                            let (a0, a1) = (own.0.max(t0), own.1.min(t1));
                            if a1 > a0 + EPSILON {
                                quad(
                                    &mut inner_tris[mine],
                                    [
                                        pt(a0, off, zs[r]),
                                        pt(a1, off, zs[r]),
                                        pt(a1, off, zs[r + 1]),
                                        pt(a0, off, zs[r + 1]),
                                    ],
                                    inside,
                                );
                            }
                        }
                    }
                    let exposed: Vec<(f64, f64)> = match theirs {
                        None => vec![own],
                        Some((t0, t1)) => {
                            let mut v = Vec::new();
                            if t0 > own.0 + EPSILON {
                                v.push((own.0, t0.min(own.1)));
                            }
                            if t1 < own.1 - EPSILON {
                                v.push((t1.max(own.0), own.1));
                            }
                            v
                        }
                    };
                    for (t0, t1) in exposed {
                        quad(
                            out,
                            [
                                pt(t0, off, zs[r]),
                                pt(t1, off, zs[r]),
                                pt(t1, off, zs[r + 1]),
                                pt(t0, off, zs[r + 1]),
                            ],
                            inside,
                        );
                    }
                }
                // Towards the previous and next column.
                let open_before =
                    c == 0 && !joined_start || c > 0 && !solid(c - 1, r, k) && !filled(c - 1, r, k);
                let open_after = c + 2 == ncols && !joined_end
                    || c + 2 < ncols && !solid(c + 1, r, k) && !filled(c + 1, r, k);
                if between {
                    // Where a wrapping finish meets the layer it covers.
                    let before = c > 0 && solid(c - 1, r, k) && owner(c - 1, r, k) != mine;
                    let after = c + 2 < ncols && solid(c + 1, r, k) && owner(c + 1, r, k) != mine;
                    for (cc, differs) in [(c, before), (c + 1, after)] {
                        if differs {
                            quad(
                                &mut inner_tris[mine],
                                [
                                    v(cc, r, k, k),
                                    v(cc, r, k + 1, k),
                                    v(cc, r + 1, k + 1, k),
                                    v(cc, r + 1, k, k),
                                ],
                                inside,
                            );
                        }
                    }
                }
                for (cc, open) in [(c, open_before), (c + 1, open_after)] {
                    if open {
                        quad(
                            out,
                            [
                                v(cc, r, k, k),
                                v(cc, r, k + 1, k),
                                v(cc, r + 1, k + 1, k),
                                v(cc, r + 1, k, k),
                            ],
                            inside,
                        );
                    }
                }
                // Below and above.
                let open_below = r == 0 || !solid(c, r - 1, k) && !filled(c, r - 1, k);
                let open_above = r + 1 == rows || !solid(c, r + 1, k) && !filled(c, r + 1, k);
                for (rr, open) in [(r, open_below), (r + 1, open_above)] {
                    if open {
                        quad(
                            out,
                            [
                                v(c, rr, k, k),
                                v(c + 1, rr, k, k),
                                v(c + 1, rr, k + 1, k),
                                v(c, rr, k + 1, k),
                            ],
                            inside,
                        );
                    }
                }
            }
        }
    }
    // Over an arch (or round an oculus), the wall fills its box back in
    // between the shape and the box's top and bottom, layer by layer.
    if !between {
        for o in &openings {
            if o.shape.is_rect() || o.corner.is_some() {
                continue;
            }
            let (w, h) = (o.t1 - o.t0, o.head - o.sill);
            let mid = (o.t0 + o.t1) / 2.0;
            let z0 = level + o.sill;
            for (line, edge) in [(o.shape.upper(w, h), h), (o.shape.lower(w, h), 0.0)] {
                // One piece each side of the middle (the crown touches the
                // box), the shape's edge closed along the box.
                let halves = [
                    line.iter()
                        .filter(|q| q[0] <= 1e-9)
                        .copied()
                        .collect::<Vec<_>>(),
                    line.iter()
                        .filter(|q| q[0] >= -1e-9)
                        .copied()
                        .collect::<Vec<_>>(),
                ];
                for half in halves {
                    if half.len() < 2 {
                        continue;
                    }
                    let (first, last) = (half[0], half[half.len() - 1]);
                    for (k, list) in tris.iter_mut().enumerate() {
                        // The shape's points, then back along the box
                        // through every corner of the wall's cells on
                        // it, so that the infill and the cells meet
                        // point to point.
                        let off = (offsets[k] + offsets[k + 1]) / 2.0;
                        let xs: Vec<f64> = (0..ncols).map(|c| col_t(c, off, k) - mid).collect();
                        let rows_z: Vec<f64> = zs.iter().map(|z| z - z0).collect();
                        let mut poly: Vec<([f64; 2], bool)> =
                            half.iter().map(|q| (*q, true)).collect();
                        let corners = [[last[0], edge], [first[0], edge], first];
                        let mut from = last;
                        for to in corners {
                            // Grid points strictly between `from` and `to`
                            // (the path runs along x or along z).
                            let mut along: Vec<[f64; 2]> = if (from[1] - to[1]).abs() < 1e-9 {
                                xs.iter()
                                    .filter(|x| (**x - from[0]) * (**x - to[0]) < -1e-12)
                                    .map(|x| [*x, from[1]])
                                    .collect()
                            } else {
                                rows_z
                                    .iter()
                                    .filter(|z| (**z - from[1]) * (**z - to[1]) < -1e-12)
                                    .map(|z| [from[0], *z])
                                    .collect()
                            };
                            along.sort_by(|a, b| {
                                let da = (a[0] - from[0]).abs() + (a[1] - from[1]).abs();
                                let db = (b[0] - from[0]).abs() + (b[1] - from[1]).abs();
                                da.total_cmp(&db)
                            });
                            along.push(to);
                            for q in along {
                                let near = |p: &[f64; 2]| {
                                    (p[0] - q[0]).abs() < 1e-9 && (p[1] - q[1]).abs() < 1e-9
                                };
                                if poly.last().is_some_and(|(p, _)| near(p)) || near(&first) {
                                    continue;
                                }
                                poly.push((q, false));
                            }
                            from = to;
                        }
                        let pts: Vec<[f64; 2]> = poly.iter().map(|(q, _)| *q).collect();
                        if archisim_core::shape::area(&pts).abs() < 1e-9 {
                            continue;
                        }
                        list.extend(infill_prism(
                            &poly,
                            offsets[k + 1],
                            offsets[k],
                            (k + 1 == layers, k == 0),
                            |x, y, z| pt(mid + x, y, z0 + z),
                            n,
                        ));
                    }
                }
            }
        }
    }
    if between {
        return wtype
            .layers
            .iter()
            .zip(inner_tris)
            .filter(|(_, t)| !t.is_empty())
            .map(|(layer, triangles)| Solid {
                owner: SolidOwner::Wall(id),
                material: layer.material,
                triangles,
            })
            .collect();
    }
    let mut solids: Vec<Solid> = wtype
        .layers
        .iter()
        .zip(tris)
        .map(|(layer, triangles)| Solid {
            owner: SolidOwner::Wall(id),
            material: layer.material,
            triangles,
        })
        .collect();
    if !planes.is_empty() {
        let bands: Vec<(f64, f64)> = (0..layers).map(|k| (offsets[k + 1], offsets[k])).collect();
        for plane in &planes {
            clip_under(&mut solids, *plane, a, n, &bands);
        }
    }
    solids
}

/// Cuts the layers of a wall under the plane `z = a·x + b·y + c`:
/// whatever is above goes, and the cut is closed, each layer over its own
/// band across the wall (`bands[k]`: the offsets of layer `k` from the
/// wall's axis through `origin`, along `n`).
fn clip_under(
    solids: &mut [Solid],
    plane: [f64; 3],
    origin: Point2,
    n: archisim_core::Vec2,
    bands: &[(f64, f64)],
) {
    let f = |p: P3| plane[0] * p[0] + plane[1] * p[1] + plane[2] - p[2];
    let lerp3 = |a: P3, b: P3, t: f64| {
        [
            a[0] + (b[0] - a[0]) * t,
            a[1] + (b[1] - a[1]) * t,
            a[2] + (b[2] - a[2]) * t,
        ]
    };
    let mut segments: Vec<(P3, P3)> = Vec::new();
    for solid in solids.iter_mut() {
        let mut out: Vec<[P3; 3]> = Vec::new();
        for t in &solid.triangles {
            let d = [f(t[0]), f(t[1]), f(t[2])];
            if d.iter().all(|v| *v >= -1e-9) {
                out.push(*t);
                continue;
            }
            if d.iter().all(|v| *v <= 1e-9) {
                continue;
            }
            // Keep the part below: a triangle or a quadrilateral.
            let mut poly: Vec<P3> = Vec::new();
            let mut cut: Vec<P3> = Vec::new();
            for i in 0..3 {
                let (p, q) = (t[i], t[(i + 1) % 3]);
                let (dp, dq) = (d[i], d[(i + 1) % 3]);
                if dp >= -1e-9 {
                    poly.push(p);
                }
                if (dp > 1e-9 && dq < -1e-9) || (dp < -1e-9 && dq > 1e-9) {
                    let x = lerp3(p, q, dp / (dp - dq));
                    poly.push(x);
                    cut.push(x);
                }
            }
            for k in 1..poly.len().saturating_sub(1) {
                out.push([poly[0], poly[k], poly[k + 1]]);
            }
            if cut.len() == 2 {
                segments.push((cut[0], cut[1]));
            }
        }
        solid.triangles = out;
    }
    // The cut's outline(s), walked from the pieces.
    let key = |p: P3| {
        (
            (p[0] * 1e7).round() as i64,
            (p[1] * 1e7).round() as i64,
            (p[2] * 1e7).round() as i64,
        )
    };
    type K = (i64, i64, i64);
    let mut adj: std::collections::HashMap<K, Vec<(K, P3)>> = Default::default();
    let mut at: std::collections::HashMap<K, P3> = Default::default();
    for (p, q) in &segments {
        let (kp, kq) = (key(*p), key(*q));
        if kp == kq {
            continue;
        }
        adj.entry(kp).or_default().push((kq, *q));
        adj.entry(kq).or_default().push((kp, *p));
        at.insert(kp, *p);
        at.insert(kq, *q);
    }
    let mut loops: Vec<Vec<Point2>> = Vec::new();
    let mut starts: Vec<K> = adj.keys().copied().collect();
    starts.sort_unstable();
    for start in starts {
        if adj.get(&start).is_none_or(|v| v.is_empty()) {
            continue;
        }
        let mut lp = vec![at[&start]];
        let mut prev = start;
        let Some((mut cur, _)) = adj.get_mut(&start).and_then(|v| v.pop()) else {
            continue;
        };
        if let Some(v) = adj.get_mut(&cur) {
            if let Some(i) = v.iter().position(|x| x.0 == prev) {
                v.remove(i);
            }
        }
        let mut guard = 0;
        while cur != start && guard < 100_000 {
            guard += 1;
            lp.push(at[&cur]);
            let Some((next, _)) = adj.get_mut(&cur).and_then(|v| v.pop()) else {
                break;
            };
            if let Some(v) = adj.get_mut(&next) {
                if let Some(i) = v.iter().position(|x| x.0 == cur) {
                    v.remove(i);
                }
            }
            prev = cur;
            cur = next;
        }
        let _ = prev;
        if lp.len() >= 3 {
            let mut poly: Vec<Point2> = lp.iter().map(|p| Point2::new(p[0], p[1])).collect();
            poly.dedup_by(|a, b| a.approx_eq(*b));
            if archisim_core::signed_area(&poly) < 0.0 {
                poly.reverse();
            }
            loops.push(poly);
        }
    }
    // Each layer's part of the cap, on the plane, facing up.
    let z = |p: Point2| plane[0] * p.x + plane[1] * p.y + plane[2];
    for (k, solid) in solids.iter_mut().enumerate() {
        let Some(&(lo, hi)) = bands.get(k) else {
            continue;
        };
        for poly in &loops {
            let mut band = poly.clone();
            for (limit, keep_above) in [(lo, true), (hi, false)] {
                let g = |p: Point2| {
                    let d = (p - origin).dot(n) - limit;
                    if keep_above {
                        d
                    } else {
                        -d
                    }
                };
                let m = band.len();
                let mut next = Vec::new();
                for i in 0..m {
                    let (p, q) = (band[i], band[(i + 1) % m]);
                    let (gp, gq) = (g(p), g(q));
                    if gp >= -1e-9 {
                        next.push(p);
                    }
                    if (gp > 1e-9 && gq < -1e-9) || (gp < -1e-9 && gq > 1e-9) {
                        next.push(p + (q - p) * (gp / (gp - gq)));
                    }
                }
                band = next;
                if band.len() < 3 {
                    break;
                }
            }
            band.dedup_by(|a, b| a.approx_eq(*b));
            if band.len() < 3 || archisim_core::signed_area(&band).abs() < 1e-12 {
                continue;
            }
            for [i, j, l] in triangulate(&band) {
                let (p, q, r) = (band[i], band[j], band[l]);
                solid
                    .triangles
                    .push([[p.x, p.y, z(p)], [q.x, q.y, z(q)], [r.x, r.y, z(r)]]);
            }
        }
    }
}

/// Offsets of every layer boundary of a wall, from the exterior face in.
fn offsets_of(g: &WallGeometry, wtype: &archisim_core::WallType) -> Vec<f64> {
    let mut offsets = vec![g.exterior_offset];
    for layer in &wtype.layers {
        let last = *offsets.last().expect("not empty");
        offsets.push(last - layer.thickness);
    }
    offsets
}

/// Solids of a curved wall (no walls butting in): each layer is its plan
/// band extruded, with no faces where layers touch or where a mitred end
/// meets the next wall. Doors and windows cut it square to the curve: the
/// band is full height between them and, across each, only below its sill
/// and above its head.
fn curved_wall_solids(
    g: &WallGeometry,
    offsets: &[f64],
    wtype: &archisim_core::WallType,
    z0: f64,
    z1: f64,
    openings: &[(f64, f64, f64, f64)],
) -> Vec<Solid> {
    let Some(arc) = &g.arc else {
        return Vec::new();
    };
    let (outer_a, outer_b) = (offsets[0], offsets[offsets.len() - 1]);
    let at = |p: Point2, z: f64| [p.x, p.y, z];
    // One piece extruded from `za` to `zb`; `jambs`: whether its cut ends
    // (at doors and windows) show.
    let extrude = |poly: &[Point2], za: f64, zb: f64, jambs: bool, tris: &mut Vec<[P3; 3]>| {
        if zb - za < 1e-9 || poly.len() < 3 {
            return;
        }
        let n = poly.len();
        for [a, b, c] in triangulate(poly) {
            tris.push([at(poly[a], zb), at(poly[b], zb), at(poly[c], zb)]);
            tris.push([at(poly[a], za), at(poly[c], za), at(poly[b], za)]);
        }
        for i in 0..n {
            let (p, q) = (poly[i], poly[(i + 1) % n]);
            if p.approx_eq(q) {
                continue;
            }
            let (fp, op) = arc.project(p);
            let (fq, oq) = arc.project(q);
            let exposed = if (op - oq).abs() < 1e-6 {
                // Along a face or a layer boundary: only the faces.
                (op - outer_a).abs() < 1e-6 || (op - outer_b).abs() < 1e-6
            } else if fp.min(fq) > 1e-6 && fp.max(fq) < 1.0 - 1e-6 {
                // Across the wall, inside it: a jamb.
                jambs
            } else {
                // Across an end: open unless another wall closes it.
                let cut = if fp + fq < 1.0 {
                    &g.start_cut
                } else {
                    &g.end_cut
                };
                !cut.is_mitre()
            };
            if exposed {
                tris.push([at(p, za), at(q, za), at(q, zb)]);
                tris.push([at(p, za), at(q, zb), at(p, zb)]);
            }
        }
    };
    let spans: Vec<(f64, f64)> = openings.iter().map(|o| (o.0, o.1)).collect();
    let full = crate::wall_pieces(&spans);
    wtype
        .layers
        .iter()
        .zip(&g.layers)
        .map(|(layer, band)| {
            let mut tris = Vec::new();
            for piece in &full {
                let poly = crate::clip_arc_span(&band.polygon, arc, piece.from, piece.to);
                extrude(&poly, z0, z1, true, &mut tris);
            }
            for &(t0, t1, sill, head) in openings {
                let poly = crate::clip_arc_span(&band.polygon, arc, Some(t0), Some(t1));
                extrude(&poly, z0, sill.min(z1), false, &mut tris);
                extrude(&poly, head.max(z0), z1, false, &mut tris);
            }
            Solid {
                owner: SolidOwner::Wall(g.wall),
                material: layer.material,
                triangles: tris,
            }
        })
        .collect()
}

/// Adds a planar quad as two triangles facing away from `inside`. Quads
/// with no area (a slanted end meeting a jamb) are skipped.
fn quad(out: &mut Vec<[P3; 3]>, q: [P3; 4], inside: P3) {
    let sub = |a: P3, b: P3| [a[0] - b[0], a[1] - b[1], a[2] - b[2]];
    let normal = cross(sub(q[2], q[0]), sub(q[3], q[1]));
    if dot(normal, normal) < 1e-18 {
        return;
    }
    let centre = [
        (q[0][0] + q[1][0] + q[2][0] + q[3][0]) / 4.0,
        (q[0][1] + q[1][1] + q[2][1] + q[3][1]) / 4.0,
        (q[0][2] + q[1][2] + q[2][2] + q[3][2]) / 4.0,
    ];
    let q = if dot(normal, sub(centre, inside)) < 0.0 {
        [q[3], q[2], q[1], q[0]]
    } else {
        q
    };
    let area = |a: P3, b: P3, c: P3| {
        let n = cross(sub(b, a), sub(c, a));
        dot(n, n)
    };
    if area(q[0], q[1], q[2]) > 1e-24 {
        out.push([q[0], q[1], q[2]]);
    }
    if area(q[0], q[2], q[3]) > 1e-24 {
        out.push([q[0], q[2], q[3]]);
    }
}

/// Solids of one slab: one skin with the top face in the top layer's
/// material, the soffit in the bottom layer's, and the edges banded by
/// layer. Layers touching inside the slab have no faces.
pub fn slab_solids(model: &Model, id: SlabId) -> Vec<Solid> {
    slab_solids_in(&WallIndex::new(model), id)
}

/// [`slab_solids`] with the walls around found through an index: the
/// layers that walls go through lose their room (see [`crate::junctions`]).
pub fn slab_solids_in(idx: &WallIndex, id: SlabId) -> Vec<Solid> {
    let model = idx.model;
    let Some(slab) = model.slab(id) else {
        return Vec::new();
    };
    let Some(stype) = model.slab_type(slab.slab_type) else {
        return Vec::new();
    };
    let level = model.level(slab.level).map_or(0.0, |l| l.elevation);
    let cuts = crate::junctions::slab_wall_cuts(idx, id);
    if cuts.iter().any(|c| !c.is_empty()) {
        let regions = crate::junctions::slab_layer_regions(&slab.outline, &slab.holes, &cuts);
        return layered_regions(
            SolidOwner::Slab(id),
            &regions,
            &|p| level + slab.top_at(p),
            &|t| slab.vertical(t),
            &stype.layers,
        );
    }
    layered_plate(
        SolidOwner::Slab(id),
        &slab.outline,
        &slab.holes,
        &|p| level + slab.top_at(p),
        &|t| slab.vertical(t),
        &stype.layers,
    )
}

/// The faces between the layers of a slab, seen only through a cut (as
/// [`wall_inner_faces`]): each boundary twice, facing up in the colour of
/// the layer under it and down in the colour of the layer over it, so a
/// cut slab shows each of its layers solid.
pub fn slab_inner_faces(idx: &WallIndex, id: SlabId) -> Vec<Solid> {
    let model = idx.model;
    let (Some(slab), Some(stype)) = (
        model.slab(id),
        model.slab(id).and_then(|s| model.slab_type(s.slab_type)),
    ) else {
        return Vec::new();
    };
    let level = model.level(slab.level).map_or(0.0, |l| l.elevation);
    let cuts = crate::junctions::slab_wall_cuts(idx, id);
    let whole: Vec<crate::junctions::Shape> = vec![(slab.outline.clone(), slab.holes.clone())];
    let regions = if cuts.iter().any(|c| !c.is_empty()) {
        crate::junctions::slab_layer_regions(&slab.outline, &slab.holes, &cuts)
    } else {
        vec![whole; stype.layers.len()]
    };
    plate_between(
        SolidOwner::Slab(id),
        &regions,
        &|p| level + slab.top_at(p),
        &|t| slab.vertical(t),
        &stype.layers,
    )
}

/// The faces between the layers of a roof's planes, as
/// [`slab_inner_faces`].
pub fn roof_inner_faces(model: &Model, id: archisim_core::RoofId) -> Vec<Solid> {
    let Some(roof) = model.roof(id) else {
        return Vec::new();
    };
    let Some(stype) = model.slab_type(roof.roof_type) else {
        return Vec::new();
    };
    let level = model.level(roof.level).map_or(0.0, |l| l.elevation);
    let mut out = Vec::new();
    for plane in roof.planes() {
        let slope = plane.slope;
        for (outline, holes) in crate::roof_parts::roof_plane_shapes(roof, &plane) {
            let regions = vec![vec![(outline, holes)]; stype.layers.len()];
            out.extend(plate_between(
                SolidOwner::Roof(id),
                &regions,
                &|p| level + roof.base_offset + slope.rise_at(p),
                &|t| slope.vertical(t),
                &stype.layers,
            ));
        }
    }
    out
}

/// The faces between the layers of a plate whose layers have the shapes
/// `regions` (top layer first): where two layers meet, a face down in the
/// upper one's material and a face up in the lower one's.
fn plate_between(
    owner: SolidOwner,
    regions: &[Vec<crate::junctions::Shape>],
    top: &dyn Fn(Point2) -> f64,
    vertical: &dyn Fn(f64) -> f64,
    layers: &[archisim_core::Layer],
) -> Vec<Solid> {
    let at = |p: Point2, depth: f64| [p.x, p.y, top(p) - vertical(depth)];
    let count = layers.len().min(regions.len());
    let mut out = Vec::new();
    let mut depth = 0.0;
    for i in 0..count.saturating_sub(1) {
        depth += layers[i].thickness;
        let (upper, lower) = (&layers[i], &layers[i + 1]);
        if upper.material == lower.material {
            continue;
        }
        let shared = crate::junctions::intersection(&regions[i], &regions[i + 1]);
        let (mut down, mut up) = (Vec::new(), Vec::new());
        for (outer, holes) in &shared {
            let poly = with_holes(outer, holes);
            for [x, y, z] in triangulate(&poly) {
                let (a, b, c) = (at(poly[x], depth), at(poly[y], depth), at(poly[z], depth));
                up.push([a, b, c]);
                down.push([a, c, b]);
            }
        }
        out.push(Solid {
            owner,
            material: upper.material,
            triangles: down,
        });
        out.push(Solid {
            owner,
            material: lower.material,
            triangles: up,
        });
    }
    out
}

/// Solids of a roof: each plane as an inclined slab over the part of the
/// roof it covers, its layers square to it, its edges plumb (two planes
/// meet on the plumb plane through their hip or ridge).
pub fn roof_solids(model: &Model, id: archisim_core::RoofId) -> Vec<Solid> {
    let Some(roof) = model.roof(id) else {
        return Vec::new();
    };
    let Some(stype) = model.slab_type(roof.roof_type) else {
        return Vec::new();
    };
    let level = model.level(roof.level).map_or(0.0, |l| l.elevation);
    let mut out = Vec::new();
    for plane in roof.planes() {
        let slope = plane.slope;
        // The dormers on this plane (on this piece of it) cut it.
        for (outline, holes) in crate::roof_parts::roof_plane_shapes(roof, &plane) {
            out.extend(layered_plate(
                SolidOwner::Roof(id),
                &outline,
                &holes,
                &|p| level + roof.base_offset + slope.rise_at(p),
                &|t| slope.vertical(t),
                &stype.layers,
            ));
        }
    }
    out
}

/// The ground of a terrain: its surface on top, flat underneath `depth`
/// below its lowest point, plumb at its edges.
/// Solids of a curtain wall: one per material (frames, each kind of
/// panel, caps, louvres), each piece a closed box.
pub fn curtain_wall_solids(model: &Model, id: archisim_core::CurtainWallId) -> Vec<Solid> {
    let Some(c) = model.curtain_wall(id) else {
        return Vec::new();
    };
    let Some(level) = model.level(c.level) else {
        return Vec::new();
    };
    let z = c.base_z(level.elevation);
    let mut by_material: Vec<(archisim_core::MaterialId, Vec<[P3; 3]>)> = Vec::new();
    for p in model.curtain_wall_pieces(id) {
        let tris = prism(&c.piece_footprint(&p), z + p.up.0, z + p.up.1);
        match by_material.iter_mut().find(|(m, _)| *m == p.material) {
            Some((_, t)) => t.extend(tris),
            None => by_material.push((p.material, tris)),
        }
    }
    // Its doors, in the cells they take.
    for (material, tris) in curtain_door_solids(model, id) {
        match by_material.iter_mut().find(|(m, _)| *m == material) {
            Some((_, t)) => t.extend(tris),
            None => by_material.push((material, tris)),
        }
    }
    by_material
        .into_iter()
        .map(|(material, triangles)| Solid {
            owner: SolidOwner::CurtainWall(id),
            material,
            triangles,
        })
        .collect()
}

/// The bars of a loose railing, in model space: its frame (posts and
/// rails) and its glass panels, apart.
pub fn loose_railing_parts(model: &Model, id: archisim_core::RailingId) -> (Vec<Bar>, Vec<Bar>) {
    let Some(r) = model.railing(id) else {
        return (Vec::new(), Vec::new());
    };
    let panel = |p: &archisim_core::RailingPiece| p.kind == archisim_core::RailingPieceKind::Panel;
    let pieces = r.pieces();
    let bars = loose_railing_bars(model, id);
    pieces
        .iter()
        .zip(bars)
        .fold((Vec::new(), Vec::new()), |(mut f, mut g), (p, b)| {
            if panel(p) {
                g.push(b);
            } else {
                f.push(b);
            }
            (f, g)
        })
}

/// The bars of a loose railing, in model space, one per piece.
pub fn loose_railing_bars(model: &Model, id: archisim_core::RailingId) -> Vec<Bar> {
    let Some(r) = model.railing(id) else {
        return Vec::new();
    };
    let z0 = model.level(r.level).map_or(0.0, |l| l.elevation) + r.base_offset;
    r.pieces()
        .into_iter()
        .filter_map(|p| {
            let (a, b) = (p.a.0, p.b.0);
            let vertical = a.approx_eq(b);
            let u = if vertical {
                // A post or bar faces along its segment: any direction
                // will do for a square.
                archisim_core::Vec2::new(1.0, 0.0)
            } else {
                (b - a).normalized()?
            };
            let n = u.perp_left();
            Some(Bar {
                a: [a.x, a.y, z0 + p.a.1],
                b: [b.x, b.y, z0 + p.b.1],
                // As the stair railings: wound so faces point out.
                side: if vertical {
                    [u.x, u.y, 0.0]
                } else {
                    [n.x, n.y, 0.0]
                },
                up: if vertical {
                    [n.x, n.y, 0.0]
                } else {
                    [0.0, 0.0, 1.0]
                },
                w: p.size,
                d: p.tall,
            })
        })
        .collect()
}

/// Solids of an object (furniture, sanitary ware…): the 3D body of its
/// family, one solid per material, turned and placed.
pub fn object_solids(model: &Model, id: archisim_core::ObjectId) -> Vec<Solid> {
    if let Some(solids) = plant_solids(model, id) {
        return solids;
    }
    if let Some(solids) = object_mesh_solids(model, id) {
        return solids;
    }
    if let Some(solids) = prop_solids(model, id) {
        return solids;
    }
    let (Some(o), Ok(ev)) = (model.object(id), model.evaluate_object(id)) else {
        return Vec::new();
    };
    let z0 = model.level(o.level).map_or(0.0, |l| l.elevation) + o.base_offset;
    let mut by_material: Vec<(MaterialId, Vec<[P3; 3]>)> = Vec::new();
    for part in &ev.parts {
        let rect = [
            o.to_plan(part.min[0], part.min[1]),
            o.to_plan(part.max[0], part.min[1]),
            o.to_plan(part.max[0], part.max[1]),
            o.to_plan(part.min[0], part.max[1]),
        ];
        let tris = prism(&rect, z0 + part.min[2], z0 + part.max[2]);
        let material = MaterialId(part.material);
        match by_material.iter_mut().find(|(m, _)| *m == material) {
            Some((_, list)) => list.extend(tris),
            None => by_material.push((material, tris)),
        }
    }
    by_material
        .into_iter()
        .map(|(material, triangles)| Solid {
            owner: SolidOwner::Object(id),
            material,
            triangles,
        })
        .collect()
}

/// Solids of an object whose type shows a 3D model: one per material of
/// the version the views take ([`Model::object_mesh`]), scaled to the
/// type's height, turned and placed. `None` for an object without a model.
pub fn object_mesh_solids(model: &Model, id: archisim_core::ObjectId) -> Option<Vec<Solid>> {
    let o = model.object(id)?;
    let (_, k, level) = model.object_mesh(id)?;
    let z0 = model.level(o.level).map_or(0.0, |l| l.elevation) + o.base_offset;
    let place = |p: [f32; 3]| -> P3 {
        let q = o.to_plan(p[0] as f64 * k, p[1] as f64 * k);
        [q.x, q.y, z0 + p[2] as f64 * k]
    };
    Some(
        level
            .groups
            .iter()
            .map(|g| Solid {
                owner: SolidOwner::Object(id),
                material: g.material,
                triangles: g
                    .triangles
                    .iter()
                    .map(|t| t.map(|i| place(level.positions[i as usize])))
                    .collect(),
            })
            .collect(),
    )
}

thread_local! {
    /// The most detail drawings ask of plants (sections, axonometries and
    /// shadows do not need every leaf).
    static PLANT_CAP: std::cell::Cell<Option<archisim_core::flora::Lod>> =
        const { std::cell::Cell::new(None) };
}

fn coarseness(l: archisim_core::flora::Lod) -> usize {
    archisim_core::flora::Lod::ALL
        .iter()
        .position(|x| *x == l)
        .unwrap_or(0)
}

/// The level of detail of the plants: the views', no finer than asked by
/// [`with_plant_detail`].
fn plant_lod(model: &Model) -> archisim_core::flora::Lod {
    let base = model.plant_detail();
    match PLANT_CAP.with(|c| c.get()) {
        Some(cap) if coarseness(cap) > coarseness(base) => cap,
        _ => base,
    }
}

/// Runs `f` with the plants at most this detailed (a section or a shadow
/// needs their masses, not their leaves).
pub fn with_plant_detail<T>(lod: archisim_core::flora::Lod, f: impl FnOnce() -> T) -> T {
    let old = PLANT_CAP.with(|c| c.replace(Some(lod)));
    let out = f();
    PLANT_CAP.with(|c| c.set(old));
    out
}

thread_local! {
    static NO_PLANTS: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

/// Runs `f` with the plants left without solids: drawings that draw them
/// as figures (elevations, sections, axonometries) need not grow their
/// meshes.
pub fn without_plants<T>(f: impl FnOnce() -> T) -> T {
    let old = NO_PLANTS.with(|c| c.replace(true));
    let out = f();
    NO_PLANTS.with(|c| c.set(old));
    out
}

/// Whether the grass on painted areas is grown: not for the drawings that
/// leave the plants out or take them less detailed (sections, shadows).
pub fn growth_wanted() -> bool {
    !NO_PLANTS.with(|c| c.get()) && PLANT_CAP.with(|c| c.get()).is_none()
}

thread_local! {
    static PIECES: std::cell::Cell<bool> = const { std::cell::Cell::new(true) };
}

/// Runs `f` without the pieces of the façades (E9) and the bars of the
/// reinforcement (E11): walls show their plain finish and concrete its
/// outside (for drawings at a scale where they would be a smudge, and for
/// clash detection).
pub fn without_detail<T>(f: impl FnOnce() -> T) -> T {
    let before = PIECES.with(|p| p.replace(false));
    let out = f();
    PIECES.with(|p| p.set(before));
    out
}

/// Whether [`model_solids`] lays the pieces of the façades and the bars.
pub fn facade_pieces_wanted() -> bool {
    PIECES.with(|p| p.get())
}

thread_local! {
    static COARSE_PROPS: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

/// Runs `f` with the furniture ArchiSim makes simplified for drawings
/// ([`COARSE_CELL`]): the folds of a cushion or the slats of a chair
/// are below a tenth of a millimetre at 1:50, and a building full of
/// furniture has millions of their triangles (Revit's "coarse" detail
/// level in the views that need no more).
pub fn with_coarse_props<T>(f: impl FnOnce() -> T) -> T {
    let old = COARSE_PROPS.with(|c| c.replace(true));
    let out = f();
    COARSE_PROPS.with(|c| c.set(old));
    out
}

/// The size of the cells the furniture's points are merged in for
/// drawings (m).
pub const COARSE_CELL: f64 = 0.03;

/// A furniture model simplified: each of its points moved to the middle
/// of the points in its cell, with which point each was.
struct Coarse {
    _mesh: std::sync::Arc<archisim_core::props::PropMesh>,
    map: Vec<u32>,
    positions: Vec<[f32; 3]>,
}

/// The simplified version of a model, made once (by the model it is of).
fn coarse_of(mesh: &std::sync::Arc<archisim_core::props::PropMesh>) -> std::sync::Arc<Coarse> {
    use std::collections::HashMap;
    use std::sync::{Arc, Mutex, OnceLock};
    static CACHE: OnceLock<Mutex<HashMap<usize, Arc<Coarse>>>> = OnceLock::new();
    let key = Arc::as_ptr(mesh) as usize;
    let cache = CACHE.get_or_init(Default::default);
    if let Some(c) = cache.lock().ok().and_then(|c| c.get(&key).cloned()) {
        return c;
    }
    let cell = COARSE_CELL as f32;
    let mut cells: HashMap<[i32; 3], u32> = HashMap::new();
    let mut sums: Vec<([f64; 3], u32)> = Vec::new();
    let map: Vec<u32> = mesh
        .positions
        .iter()
        .map(|p| {
            let k = p.map(|v| (v / cell).floor() as i32);
            *cells.entry(k).or_insert_with(|| {
                sums.push(([0.0; 3], 0));
                (sums.len() - 1) as u32
            })
        })
        .collect();
    for (p, &c) in mesh.positions.iter().zip(&map) {
        let s = &mut sums[c as usize];
        for (sum, v) in s.0.iter_mut().zip(p) {
            *sum += *v as f64;
        }
        s.1 += 1;
    }
    let positions = sums
        .iter()
        .map(|(s, n)| s.map(|v| (v / *n as f64) as f32))
        .collect();
    let c = Arc::new(Coarse {
        _mesh: mesh.clone(),
        map,
        positions,
    });
    if let Ok(mut cache) = cache.lock() {
        // The furniture of a project is a few hundred models at most.
        if cache.len() > 4096 {
            cache.clear();
        }
        cache.insert(key, c.clone());
    }
    c
}

/// Solids of a plant grown by ArchiSim: one per material, at the level of
/// detail the views take ([`Model::plant_detail`]), turned and placed.
/// `None` for an object that is not a plant.
pub fn plant_solids(model: &Model, id: archisim_core::ObjectId) -> Option<Vec<Solid>> {
    let o = model.object(id)?;
    if NO_PLANTS.with(|c| c.get()) {
        return model.type_grows(o.object_type).then(Vec::new);
    }
    let (mesh, parts) = model.plant_mesh(id, plant_lod(model))?;
    let z0 = model.level(o.level).map_or(0.0, |l| l.elevation) + o.base_offset;
    let place = |p: [f32; 3]| -> P3 {
        let q = o.to_plan(p[0] as f64, p[1] as f64);
        [q.x, q.y, z0 + p[2] as f64]
    };
    Some(
        parts
            .iter()
            .map(|(material, tris)| Solid {
                owner: SolidOwner::Object(id),
                material: *material,
                triangles: tris
                    .iter()
                    .map(|t| t.map(|i| place(mesh.positions[i as usize])))
                    .collect(),
            })
            .collect(),
    )
}

/// Solids of an object whose model ArchiSim makes from its measurements
/// (`archisim-props`): one per material, turned and placed. `None` for
/// any other object.
pub fn prop_solids(model: &Model, id: archisim_core::ObjectId) -> Option<Vec<Solid>> {
    let o = model.object(id)?;
    let (mesh, parts) = model.object_prop(id)?;
    let z0 = model.level(o.level).map_or(0.0, |l| l.elevation) + o.base_offset;
    let place = |p: [f32; 3]| -> P3 {
        let q = o.to_plan(p[0] as f64, p[1] as f64);
        [q.x, q.y, z0 + p[2] as f64]
    };
    let mut out: Vec<Solid> = if COARSE_PROPS.with(|c| c.get()) {
        let c = coarse_of(&mesh);
        parts
            .iter()
            .map(|(material, tris)| {
                let mut seen = std::collections::HashSet::new();
                Solid {
                    owner: SolidOwner::Object(id),
                    material: *material,
                    triangles: tris
                        .iter()
                        .map(|t| t.map(|i| c.map[i as usize]))
                        .filter(|t| t[0] != t[1] && t[1] != t[2] && t[2] != t[0])
                        .filter(|t| {
                            let mut k = *t;
                            k.sort_unstable();
                            seen.insert(k)
                        })
                        .map(|t| t.map(|i| place(c.positions[i as usize])))
                        .collect(),
                }
            })
            .collect()
    } else {
        parts
            .iter()
            .map(|(material, tris)| Solid {
                owner: SolidOwner::Object(id),
                material: *material,
                triangles: tris
                    .iter()
                    .map(|t| t.map(|i| place(mesh.positions[i as usize])))
                    .collect(),
            })
            .collect()
    };
    // The first module of a worktop run carries the run's worktop.
    if let Some(run) = model.worktop_run_of(id).filter(|r| r.members[0] == id) {
        out.extend(worktop_solids(model, &run, SolidOwner::Object(id)));
    }
    Some(out)
}

/// The solid of a worktop run: one slab over all its modules. Its top and
/// bottom faces go module by module (no long thin triangles: views that
/// paint faces far to near order them well), its edges only round the
/// whole outline and its sinks (no joints).
pub fn worktop_solids(
    model: &Model,
    run: &archisim_core::WorktopRun,
    owner: SolidOwner,
) -> Vec<Solid> {
    let z1 = model.level(run.level).map_or(0.0, |l| l.elevation) + run.top;
    let z0 = z1 - run.thickness;
    let at = |p: Point2, z: f64| [p.x, p.y, z];
    let shape = |r: &[Point2; 4]| (ensure_ccw(r.to_vec()), Vec::new());
    let sinks: Vec<crate::junctions::Shape> = run.sinks.iter().map(shape).collect();
    let mut tris: Vec<[P3; 3]> = Vec::new();
    // Faces: each module's part, not already covered by another's.
    let mut covered: Vec<crate::junctions::Shape> = Vec::new();
    for piece in &run.pieces {
        let overlaps =
            crate::junctions::area(&crate::junctions::intersection(&[shape(piece)], &covered))
                > 1e-6;
        let own = if overlaps {
            // Modules overlapping (rare): what is left of this one.
            let own = crate::junctions::difference(&[shape(piece)], &covered);
            crate::junctions::difference(&own, &sinks)
        } else {
            // Side by side, the usual: the piece exactly as it is, with
            // the openings of its sinks, so it meets its neighbours'
            // corners with no crack between them.
            let inside = |q: &[Point2; 4]| {
                let c = q.iter().fold(Point2::new(0.0, 0.0), |a, p| {
                    Point2::new(a.x + p.x / 4.0, a.y + p.y / 4.0)
                });
                crate::junctions::inside(c, piece)
            };
            let holes: Vec<Vec<Point2>> = run
                .sinks
                .iter()
                .filter(|q| inside(q))
                .map(|q| q.to_vec())
                .collect();
            vec![(ensure_ccw(piece.to_vec()), holes)]
        };
        covered = crate::junctions::union(&covered, &[shape(piece)]);
        for (outer, holes) in own {
            let poly = with_holes(&outer, &holes);
            for [a, b, c] in triangulate(&poly) {
                // Top seen from above: counter-clockwise. Bottom: reversed.
                tris.push([at(poly[a], z1), at(poly[b], z1), at(poly[c], z1)]);
                tris.push([at(poly[a], z0), at(poly[c], z0), at(poly[b], z0)]);
            }
        }
    }
    // Edges: the sides of each module's piece that no other piece
    // touches (no joints, and on the same corners as the faces), and
    // round each sink; outside to the right of each edge.
    let mut side = |p: Point2, q: Point2| {
        tris.push([at(p, z0), at(q, z0), at(q, z1)]);
        tris.push([at(p, z0), at(q, z1), at(p, z1)]);
    };
    let rings: Vec<Vec<Point2>> = run.pieces.iter().map(|p| ensure_ccw(p.to_vec())).collect();
    for (k, ring) in rings.iter().enumerate() {
        for i in 0..ring.len() {
            let (a, b) = (ring[i], ring[(i + 1) % ring.len()]);
            for (p, q) in free_parts(a, b, &rings, k) {
                side(p, q);
            }
        }
    }
    for sink in &run.sinks {
        let mut h = ensure_ccw(sink.to_vec());
        h.reverse();
        for i in 0..h.len() {
            side(h[i], h[(i + 1) % h.len()]);
        }
    }
    // The upstands: a strip standing on each back edge, towards the front.
    for ([a, b], height) in &run.upstands {
        let Some(u) = (*b - *a).normalized() else {
            continue;
        };
        let n = u.perp_left() * archisim_core::props::UPSTAND;
        let ring = [*a, *b, *b + n, *a + n];
        let (lo, hi) = (z1, z1 + height);
        tris.push([at(ring[0], hi), at(ring[1], hi), at(ring[2], hi)]);
        tris.push([at(ring[0], hi), at(ring[2], hi), at(ring[3], hi)]);
        for i in 0..4 {
            // Outside to the right of each edge, as the ring runs CCW.
            let (p, q) = (ring[i], ring[(i + 1) % 4]);
            tris.push([at(p, lo), at(q, lo), at(q, hi)]);
            tris.push([at(p, lo), at(q, hi), at(p, hi)]);
        }
    }
    vec![Solid {
        owner,
        material: run.material,
        triangles: tris,
    }]
}

/// The parts of the edge `a`–`b` of piece `own` that no other piece's
/// edge lies on (where two pieces of a worktop touch, no side is made).
fn free_parts(a: Point2, b: Point2, rings: &[Vec<Point2>], own: usize) -> Vec<(Point2, Point2)> {
    let ab = b - a;
    let len2 = ab.dot(ab);
    if len2 < 1e-12 {
        return Vec::new();
    }
    let t_of = |p: Point2| (p - a).dot(ab) / len2;
    let on_line = |p: Point2| ((p - a).cross(ab)).abs() / len2.sqrt() < 1e-6;
    let mut covered: Vec<(f64, f64)> = Vec::new();
    for (k, ring) in rings.iter().enumerate() {
        if k == own {
            continue;
        }
        for i in 0..ring.len() {
            let (p, q) = (ring[i], ring[(i + 1) % ring.len()]);
            if on_line(p) && on_line(q) {
                let (t0, t1) = (t_of(p), t_of(q));
                let (lo, hi) = (t0.min(t1).max(0.0), t0.max(t1).min(1.0));
                if hi - lo > 1e-9 {
                    covered.push((lo, hi));
                }
            }
        }
    }
    covered.sort_by(|x, y| x.0.total_cmp(&y.0));
    let mut out = Vec::new();
    let mut t = 0.0;
    for (lo, hi) in covered {
        if lo > t + 1e-9 {
            out.push((a + ab * t, a + ab * lo));
        }
        t = t.max(hi);
    }
    if t < 1.0 - 1e-9 {
        out.push((a + ab * t, b));
    }
    out
}

/// The colours of the corners of each triangle of a solid of
/// [`object_mesh_solids`] (its textures) or of [`plant_solids`], in the
/// same order; `None` for any other solid.
pub fn solid_vertex_colors(model: &Model, s: &Solid) -> Option<Vec<[[u8; 3]; 3]>> {
    if let SolidOwner::Growth(_) = s.owner {
        return crate::growth::growth_colors(model, s);
    }
    let SolidOwner::Object(id) = s.owner else {
        return None;
    };
    if let Some((mesh, parts)) = model.plant_mesh(id, plant_lod(model)) {
        let (_, tris) = parts.iter().find(|(m, _)| *m == s.material)?;
        return Some(
            tris.iter()
                .map(|t| t.map(|i| mesh.colors[i as usize]))
                .collect(),
        );
    }
    let (_, _, level) = model.object_mesh(id)?;
    let g = level.groups.iter().find(|g| g.material == s.material)?;
    Some(
        g.triangles
            .iter()
            .map(|t| t.map(|i| level.colors[i as usize]))
            .collect(),
    )
}

/// Solids of a railing with a type (E12): each of its runs made by the
/// type's generator, laid along its segment, one solid per material.
fn typed_railing_solids(
    model: &Model,
    id: archisim_core::RailingId,
    t: archisim_core::ObjectTypeId,
) -> Vec<Solid> {
    let Some(r) = model.railing(id) else {
        return Vec::new();
    };
    let z0 = model.level(r.level).map_or(0.0, |l| l.elevation) + r.base_offset;
    let mut by_material: Vec<(MaterialId, Vec<[P3; 3]>)> = Vec::new();
    for run in r.runs() {
        let Some((kind, params)) = model.railing_run_params(id, &run) else {
            continue;
        };
        let Some(mesh) = archisim_core::props::mesh_of(&kind, &params) else {
            continue;
        };
        let n = run.dir.perp_left();
        let place = |q: [f32; 3]| -> P3 {
            let p = run.start + run.dir * q[0] as f64 + n * q[1] as f64;
            [p.x, p.y, z0 + q[2] as f64]
        };
        for (role, tris) in &mesh.parts {
            let Some(material) = model.type_role_material(t, role) else {
                continue;
            };
            let list: Vec<[P3; 3]> = tris
                .iter()
                .map(|tri| tri.map(|i| place(mesh.positions[i as usize])))
                .collect();
            match by_material.iter_mut().find(|(m, _)| *m == material) {
                Some((_, v)) => v.extend(list),
                None => by_material.push((material, list)),
            }
        }
    }
    by_material
        .into_iter()
        .map(|(material, triangles)| Solid {
            owner: SolidOwner::Railing(id),
            material,
            triangles,
        })
        .collect()
}

/// Solids of a loose railing: its bars, each a closed box, as one solid.
pub fn loose_railing_solids(model: &Model, id: archisim_core::RailingId) -> Vec<Solid> {
    let Some(r) = model.railing(id) else {
        return Vec::new();
    };
    if let Some(t) = r.railing_type {
        return typed_railing_solids(model, id, t);
    }
    let (bars, panels) = loose_railing_parts(model, id);
    let solid = |material, bars: Vec<Bar>| Solid {
        owner: SolidOwner::Railing(id),
        material,
        triangles: bars.iter().flat_map(|b| b.triangles()).collect(),
    };
    let mut out = vec![solid(r.material, bars)];
    if let Some(glass) = r.glass {
        out.push(solid(glass, panels));
    }
    out
}

pub fn terrain_solids(model: &Model, id: archisim_core::TerrainId) -> Vec<Solid> {
    let Some(t) = model.terrain(id) else {
        return Vec::new();
    };
    let tris = t.triangles();
    if tris.is_empty() {
        return Vec::new();
    }
    let p3 = |i: usize| [t.points[i].at.x, t.points[i].at.y, t.points[i].z];
    let floor = t.range().0 - t.depth;
    let low = |i: usize| [t.points[i].at.x, t.points[i].at.y, floor];
    let ring = t.boundary();
    let poly: Vec<Point2> = ring.iter().map(|i| t.points[*i].at).collect();
    // The ground's top, less the room the pools take out of it.
    let whole: Vec<[P3; 3]> = tris
        .iter()
        .map(|[a, b, c]| [p3(*a), p3(*b), p3(*c)])
        .collect();
    let holes = terrain_holes(model, &poly);
    let surface = crate::pools::cut_surface(&whole, &holes);
    let mut out = surface.clone();
    out.extend(crate::pools::hole_faces(&surface, &holes, &poly));
    for [a, b, c] in triangulate(&poly) {
        out.push([low(ring[a]), low(ring[c]), low(ring[b])]);
    }
    let n = ring.len();
    for k in 0..n {
        let (a, b) = (ring[k], ring[(k + 1) % n]);
        out.push([low(a), low(b), p3(b)]);
        out.push([low(a), p3(b), p3(a)]);
    }
    let mut solids = vec![Solid {
        owner: SolidOwner::Terrain(id),
        material: t.material,
        triangles: out,
    }];
    // The areas painted over it: a thin closed layer each, on the ground.
    for patch in &t.patches {
        let regions = if holes.is_empty() {
            patch.regions.clone()
        } else {
            let cut: Vec<crate::junctions::Shape> = holes
                .iter()
                .map(|h| (h.outline.clone(), Vec::new()))
                .collect();
            crate::junctions::difference(&patch.regions, &cut)
        };
        let layer = patch_layer(&surface, &regions, PATCH_THICKNESS);
        if !layer.is_empty() {
            solids.push(Solid {
                owner: SolidOwner::Terrain(id),
                material: patch.material,
                triangles: layer,
            });
        }
    }
    solids
}

/// The excavations of the model's pools that reach into a terrain's
/// outline.
pub fn terrain_holes(model: &Model, outline: &[Point2]) -> Vec<archisim_core::Excavation> {
    let (lo, hi) = outline.iter().fold(
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
    );
    let mut holes = model.excavations();
    // The roads sit in the ground (U1): their whole width, down to under
    // their pavement where it is lowest.
    for r in model.roads() {
        let outline = r.outline();
        if outline.len() < 3 {
            continue;
        }
        let base = model.level(r.level).map_or(0.0, |l| l.elevation);
        let len = r.length();
        let n = (len / 5.0).ceil().max(1.0) as usize;
        let low = (0..=n)
            .map(|k| r.top_at(len * k as f64 / n as f64))
            .fold(f64::MAX, f64::min);
        holes.push(archisim_core::Excavation {
            outline,
            bottom: base + low - r.section.depth,
        });
    }
    holes
        .into_iter()
        .filter(|e| {
            e.outline
                .iter()
                .any(|p| p.x >= lo.x && p.x <= hi.x && p.y >= lo.y && p.y <= hi.y)
        })
        .collect()
}

/// How thick the layer of a painted area of a terrain is (m): enough not
/// to flicker with the ground under it.
pub const PATCH_THICKNESS: f64 = 0.02;

/// A layer `thick` deep over the parts of `surface` (triangles, counter-
/// clockwise seen from above) inside `regions`: its top, its bottom on the
/// surface and its edges where the regions end.
pub fn patch_layer(
    surface: &[[P3; 3]],
    regions: &[(Vec<Point2>, Vec<Vec<Point2>>)],
    thick: f64,
) -> Vec<[P3; 3]> {
    let shapes: Vec<crate::junctions::Shape> = regions
        .iter()
        .filter(|(o, _)| o.len() >= 3)
        .map(|(o, h)| (ensure_ccw(o.clone()), h.clone()))
        .collect();
    if shapes.is_empty() {
        return Vec::new();
    }
    // The edges of the regions, to tell an edge of a piece that is the
    // region's (it gets a side) from one inside it.
    let mut borders: Vec<(Point2, Point2)> = Vec::new();
    for (o, holes) in &shapes {
        for ring in std::iter::once(o).chain(holes.iter()) {
            for i in 0..ring.len() {
                borders.push((ring[i], ring[(i + 1) % ring.len()]));
            }
        }
    }
    let on_border = |a: Point2, b: Point2| {
        let m = (a + b) * 0.5;
        borders.iter().any(|(p, q)| {
            let pq = *q - *p;
            let len = pq.length();
            if len < 1e-9 {
                return false;
            }
            let t = (m - *p).dot(pq) / (len * len);
            (-1e-6..=1.0 + 1e-6).contains(&t) && ((m - *p).cross(pq)).abs() / len < 1e-5
        })
    };
    // The pieces of each triangle inside the regions, each with the
    // triangle's plane: z = c + gx·(x − x0) + gy·(y − y0).
    struct Piece {
        shape: crate::junctions::Shape,
        plane: [f64; 5],
    }
    let mut pieces: Vec<Piece> = Vec::new();
    for tri in surface {
        let flat: Vec<Point2> = tri.iter().map(|p| Point2::new(p[0], p[1])).collect();
        let e1 = [
            tri[1][0] - tri[0][0],
            tri[1][1] - tri[0][1],
            tri[1][2] - tri[0][2],
        ];
        let e2 = [
            tri[2][0] - tri[0][0],
            tri[2][1] - tri[0][1],
            tri[2][2] - tri[0][2],
        ];
        let n = [
            e1[1] * e2[2] - e1[2] * e2[1],
            e1[2] * e2[0] - e1[0] * e2[2],
            e1[0] * e2[1] - e1[1] * e2[0],
        ];
        if n[2].abs() < 1e-12 {
            continue;
        }
        let plane = [tri[0][2], -n[0] / n[2], -n[1] / n[2], tri[0][0], tri[0][1]];
        for shape in crate::junctions::intersection(&[(ensure_ccw(flat), Vec::new())], &shapes) {
            pieces.push(Piece { shape, plane });
        }
    }
    // Every corner of every piece splits the edges of the others that pass
    // through it (two triangles share an edge, and a region's corner may
    // lie on it): no T-junctions, so the layer is closed.
    let corners: Vec<Point2> = pieces
        .iter()
        .flat_map(|p| p.shape.0.iter().chain(p.shape.1.iter().flatten()).copied())
        .collect();
    let split = |ring: &[Point2]| -> Vec<Point2> {
        let mut v = Vec::new();
        for i in 0..ring.len() {
            let (a, b) = (ring[i], ring[(i + 1) % ring.len()]);
            v.push(a);
            let ab = b - a;
            let len2 = ab.dot(ab);
            if len2 < 1e-18 {
                continue;
            }
            let mut on: Vec<(f64, Point2)> = corners
                .iter()
                .filter_map(|c| {
                    let t = (*c - a).dot(ab) / len2;
                    let off = (*c - a).cross(ab).abs() / len2.sqrt();
                    (t > 1e-9 && t < 1.0 - 1e-9 && off < 1e-7).then_some((t, *c))
                })
                .collect();
            on.sort_by(|x, y| x.0.total_cmp(&y.0));
            on.dedup_by(|x, y| (x.0 - y.0).abs() < 1e-12);
            v.extend(on.into_iter().map(|(_, c)| c));
        }
        v
    };
    let mut out = Vec::new();
    for piece in &pieces {
        let [c, gx, gy, x0, y0] = piece.plane;
        let z = |p: Point2| c + gx * (p.x - x0) + gy * (p.y - y0);
        let outer = split(&ensure_ccw(piece.shape.0.clone()));
        let holes: Vec<Vec<Point2>> = piece
            .shape
            .1
            .iter()
            .map(|h| {
                let mut h = ensure_ccw(h.clone());
                h.reverse();
                split(&h)
            })
            .collect();
        let poly = with_holes(&outer, &holes);
        for [a, b, cc] in triangulate(&poly) {
            let (pa, pb, pc) = (poly[a], poly[b], poly[cc]);
            let up = |p: Point2| [p.x, p.y, z(p) + thick];
            let down = |p: Point2| [p.x, p.y, z(p)];
            out.push([up(pa), up(pb), up(pc)]);
            out.push([down(pa), down(pc), down(pb)]);
        }
        // Sides where the region ends: outside to the right of each edge
        // of the counter-clockwise outline (holes run the other way).
        for ring in std::iter::once(&outer).chain(holes.iter()) {
            for i in 0..ring.len() {
                let (a, b) = (ring[i], ring[(i + 1) % ring.len()]);
                if !on_border(a, b) {
                    continue;
                }
                let (za, zb) = (z(a), z(b));
                out.push([[a.x, a.y, za], [b.x, b.y, zb], [b.x, b.y, zb + thick]]);
                out.push([
                    [a.x, a.y, za],
                    [b.x, b.y, zb + thick],
                    [a.x, a.y, za + thick],
                ]);
            }
        }
    }
    out
}

/// A straight bar of a railing: from `a` to `b` (the middle of its end
/// faces), `w` wide across `side` (a horizontal unit vector) and `d` deep
/// along `up`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Bar {
    pub a: P3,
    pub b: P3,
    pub side: [f64; 3],
    pub up: [f64; 3],
    pub w: f64,
    pub d: f64,
}

impl Bar {
    /// Its eight corners: the face at `a`, then the face at `b`.
    pub fn corners(&self) -> [P3; 8] {
        let c = |p: P3, i: f64, j: f64| {
            [
                p[0] + self.side[0] * i * self.w / 2.0 + self.up[0] * j * self.d / 2.0,
                p[1] + self.side[1] * i * self.w / 2.0 + self.up[1] * j * self.d / 2.0,
                p[2] + self.side[2] * i * self.w / 2.0 + self.up[2] * j * self.d / 2.0,
            ]
        };
        [
            c(self.a, -1.0, -1.0),
            c(self.a, 1.0, -1.0),
            c(self.a, 1.0, 1.0),
            c(self.a, -1.0, 1.0),
            c(self.b, -1.0, -1.0),
            c(self.b, 1.0, -1.0),
            c(self.b, 1.0, 1.0),
            c(self.b, -1.0, 1.0),
        ]
    }

    /// Its twelve triangles, facing out.
    pub fn triangles(&self) -> Vec<[P3; 3]> {
        let v = self.corners();
        let quads = [
            [0, 3, 2, 1],
            [4, 5, 6, 7],
            [0, 1, 5, 4],
            [1, 2, 6, 5],
            [2, 3, 7, 6],
            [3, 0, 4, 7],
        ];
        let mut t: Vec<[P3; 3]> = Vec::new();
        for q in quads {
            t.push([v[q[0]], v[q[1]], v[q[2]]]);
            t.push([v[q[0]], v[q[2]], v[q[3]]]);
        }
        let solid = Solid {
            owner: SolidOwner::Wall(archisim_core::WallId(0)),
            material: archisim_core::MaterialId(0),
            triangles: t.clone(),
        };
        if solid.volume() < 0.0 {
            for tri in &mut t {
                tri.swap(1, 2);
            }
        }
        t
    }
}

/// The side view of a flight: `x` along it from its first riser, `y` the
/// height; counter-clockwise. The steps on top, the slab under them
/// parallel to the line of the nosings.
pub fn flight_profile(
    f: &archisim_core::Flight,
    riser: f64,
    waist: f64,
    level: f64,
) -> Vec<Point2> {
    let z0 = level + f.base;
    let g = f.going;
    let t = f.treads;
    let len = f.length();
    let mut pts = vec![Point2::new(0.0, z0)];
    for i in 0..t {
        let z = z0 + (i + 1) as f64 * riser;
        pts.push(Point2::new(i as f64 * g, z));
        pts.push(Point2::new((i + 1) as f64 * g, z));
    }
    // The last riser is the front of the landing (or of the floor above).
    let last = z0 + t as f64 * riser;
    // Under the steps: the slab ("losa"), `waist` thick square to the
    // line of the inner corners of the steps (the "garganta").
    let k = riser / g;
    let wv = waist * (1.0 + k * k).sqrt();
    let under = |s: f64| z0 + k * s - wv;
    let end = under(len).min(last - 0.01);
    if end > z0 + 1e-9 {
        pts.push(Point2::new(len, end));
        // Where the slab meets the floor.
        pts.push(Point2::new(wv / k, z0));
    } else {
        pts.push(Point2::new(len, z0));
    }
    pts.dedup_by(|a, b| a.approx_eq(*b));
    if archisim_core::signed_area(&pts) < 0.0 {
        pts.reverse();
    }
    pts
}

/// A prism from a profile in the vertical plane through `origin` along
/// `u` (`x` along `u`, `y` the height), swept `half` to each side. The
/// profile must be counter-clockwise and x-monotone (any vertical line
/// crosses it once), as a flight seen from the side: it is cut into
/// vertical strips at its corners, each convex, so that its many square
/// corners triangulate cleanly.
fn vertical_prism(
    owner: SolidOwner,
    material: archisim_core::MaterialId,
    profile: &[Point2],
    origin: Point2,
    u: archisim_core::Vec2,
    half: f64,
) -> Solid {
    let n = u.perp_left();
    let at = |p: Point2, side: f64| {
        let q = origin + u * p.x + n * (side * half);
        [q.x, q.y, p.y]
    };
    let mut xs: Vec<f64> = profile.iter().map(|p| p.x).collect();
    xs.sort_by(f64::total_cmp);
    xs.dedup_by(|a, b| (*a - *b).abs() < 1e-9);
    let key = |p: Point2| ((p.x * 1e7).round() as i64, (p.y * 1e7).round() as i64);
    let mut strips: Vec<Vec<Point2>> = Vec::new();
    for w in xs.windows(2) {
        let mut poly = clip_x(profile, w[0], true);
        poly = clip_x(&poly, w[1], false);
        poly.dedup_by(|a, b| a.approx_eq(*b));
        while poly.len() > 1 && poly[0].approx_eq(poly[poly.len() - 1]) {
            poly.pop();
        }
        // Points of the profile on the cutting line beyond the strip
        // (the next riser) leave a spike: out and back along the line.
        loop {
            let m = poly.len();
            if m < 3 {
                break;
            }
            let spike = (0..m).find(|&i| {
                let (a, b, c) = (poly[(i + m - 1) % m], poly[i], poly[(i + 1) % m]);
                let (u, v) = (b - a, c - b);
                u.cross(v).abs() < 1e-12 && u.dot(v) < 0.0
            });
            match spike {
                Some(i) => {
                    poly.remove(i);
                }
                None => break,
            }
        }
        if poly.len() >= 3 && archisim_core::signed_area(&poly).abs() > 1e-12 {
            strips.push(poly);
        }
    }
    let mut tris = Vec::new();
    type Key = ((i64, i64), (i64, i64));
    let mut edges: std::collections::HashMap<Key, (Point2, Point2)> = Default::default();
    for poly in &strips {
        // A fan from the middle: every side of the strip gets its own
        // triangle, whatever points lie in line.
        let c = poly.iter().fold(Point2::new(0.0, 0.0), |a, p| a + *p) * (1.0 / poly.len() as f64);
        let m = poly.len();
        for i in 0..m {
            let (a, b) = (poly[i], poly[(i + 1) % m]);
            // Counter-clockwise seen from the right of `u`: as is there,
            // reversed on the left.
            tris.push([at(c, -1.0), at(a, -1.0), at(b, -1.0)]);
            tris.push([at(c, 1.0), at(b, 1.0), at(a, 1.0)]);
            edges.insert((key(a), key(b)), (a, b));
        }
    }
    // The sides: the strips' edges that no other strip shares.
    for (&(ka, kb), &(p, q)) in &edges {
        if edges.contains_key(&(kb, ka)) {
            continue;
        }
        tris.push([at(p, -1.0), at(p, 1.0), at(q, 1.0)]);
        tris.push([at(p, -1.0), at(q, 1.0), at(q, -1.0)]);
    }
    Solid {
        owner,
        material,
        triangles: tris,
    }
}

/// The part of a polygon on one side of the vertical line `x = at`
/// (`right`: where x ≥ at).
fn clip_x(poly: &[Point2], at: f64, right: bool) -> Vec<Point2> {
    let f = |p: Point2| if right { p.x - at } else { at - p.x };
    let n = poly.len();
    let mut out = Vec::new();
    for i in 0..n {
        let (a, b) = (poly[i], poly[(i + 1) % n]);
        let (fa, fb) = (f(a), f(b));
        if fa >= -1e-12 {
            out.push(a);
        }
        if (fa < -1e-12 && fb > 1e-12) || (fa > 1e-12 && fb < -1e-12) {
            let t = fa / (fa - fb);
            out.push(a + (b - a) * t);
        }
    }
    out
}

/// [`layered_plate`] for the other modules of this crate.
pub(crate) fn layered_plate_pub(
    owner: SolidOwner,
    outline: &[Point2],
    holes: &[Vec<Point2>],
    top: &dyn Fn(Point2) -> f64,
    vertical: &dyn Fn(f64) -> f64,
    layers: &[archisim_core::Layer],
) -> Vec<Solid> {
    layered_plate(owner, outline, holes, top, vertical, layers)
}

/// [`vertical_prism`] for the other modules of this crate.
pub(crate) fn vertical_prism_pub(
    owner: SolidOwner,
    material: archisim_core::MaterialId,
    profile: &[Point2],
    origin: Point2,
    u: archisim_core::Vec2,
    half: f64,
) -> Solid {
    vertical_prism(owner, material, profile, origin, u, half)
}

/// A plate of layers under a (level or inclined) top face: one skin with
/// the top face in the top layer's material, the soffit in the bottom
/// layer's, and the edges banded by layer. Layers touching inside have no
/// faces. `top` is the height of the top face at a point; `vertical` the
/// vertical height of a thickness.
fn layered_plate(
    owner: SolidOwner,
    outline: &[Point2],
    holes: &[Vec<Point2>],
    top: &dyn Fn(Point2) -> f64,
    vertical: &dyn Fn(f64) -> f64,
    layers: &[archisim_core::Layer],
) -> Vec<Solid> {
    let outer = ensure_ccw(outline.to_vec());
    // The faces round the outline and round each hole (walked clockwise,
    // so they face into the hole).
    let mut rings = vec![outer.clone()];
    rings.extend(holes.iter().map(|h| {
        let mut h = ensure_ccw(h.clone());
        h.reverse();
        h
    }));
    let poly = with_holes(&outer, holes);
    let caps = triangulate(&poly);
    // Heights of the faces between layers at a point: the top face, then
    // each layer down (square to an inclined face, so vertically a little
    // more).
    let at = |p: Point2, depth: f64| [p.x, p.y, top(p) - vertical(depth)];
    let mut upper = 0.0;
    let count = layers.len();
    layers
        .iter()
        .enumerate()
        .map(|(i, layer)| {
            let bottom = upper + layer.thickness;
            let mut tris = Vec::new();
            if i == 0 {
                for [x, y, z] in &caps {
                    tris.push([
                        at(poly[*x], upper),
                        at(poly[*y], upper),
                        at(poly[*z], upper),
                    ]);
                }
            }
            if i + 1 == count {
                for [x, y, z] in &caps {
                    tris.push([
                        at(poly[*x], bottom),
                        at(poly[*z], bottom),
                        at(poly[*y], bottom),
                    ]);
                }
            }
            for ring in &rings {
                for k in 0..ring.len() {
                    let (p, q) = (ring[k], ring[(k + 1) % ring.len()]);
                    tris.push([at(p, bottom), at(q, bottom), at(q, upper)]);
                    tris.push([at(p, bottom), at(q, upper), at(p, upper)]);
                }
            }
            upper = bottom;
            Solid {
                owner,
                material: layer.material,
                triangles: tris,
            }
        })
        .collect()
}

/// A plate whose layers have each their own shape (`regions[i]`, top
/// layer first), as one skin: the top of the first layer, the bottom of the
/// last, the edges of each, and between two layers only where one of them
/// is and the other is not. Every edge is split at the corners of the other
/// faces lying on it, so faces meet corner to corner (no cracks, no lines
/// drawn along a flat face in sections and elevations).
fn layered_regions(
    owner: SolidOwner,
    regions: &[Vec<crate::junctions::Shape>],
    top: &dyn Fn(Point2) -> f64,
    vertical: &dyn Fn(f64) -> f64,
    layers: &[archisim_core::Layer],
) -> Vec<Solid> {
    use crate::junctions::{difference, Shape};
    let at = |p: Point2, depth: f64| [p.x, p.y, top(p) - vertical(depth)];
    let count = layers.len().min(regions.len());
    let mut depths = vec![0.0];
    for l in &layers[..count] {
        depths.push(depths.last().copied().unwrap_or(0.0) + l.thickness);
    }
    // The flat faces of each layer: above (the top face, or where the layer
    // above is not) and below.
    let above: Vec<Vec<Shape>> = (0..count)
        .map(|i| match i.checked_sub(1) {
            None => regions[i].clone(),
            Some(j) => difference(&regions[i], &regions[j]),
        })
        .collect();
    let below: Vec<Vec<Shape>> = (0..count)
        .map(|i| {
            if i + 1 == count {
                regions[i].clone()
            } else {
                difference(&regions[i], &regions[i + 1])
            }
        })
        .collect();
    // Every corner of every face, to split the edges at.
    let mut corners: Vec<Point2> = Vec::new();
    for list in regions[..count].iter().chain(&above).chain(&below) {
        for (outer, holes) in list {
            corners.extend(outer.iter().copied());
            for h in holes {
                corners.extend(h.iter().copied());
            }
        }
    }
    corners.sort_by(|a, b| a.x.total_cmp(&b.x).then(a.y.total_cmp(&b.y)));
    corners.dedup_by(|a, b| a.approx_eq(*b));
    // The corners strictly inside the segment `p q`, in order from `p`.
    let on = |p: Point2, q: Point2| -> Vec<Point2> {
        let d = q - p;
        let len2 = d.dot(d);
        if len2 < 1e-18 {
            return Vec::new();
        }
        let (lo, hi) = (
            Point2::new(p.x.min(q.x) - 1e-7, p.y.min(q.y) - 1e-7),
            Point2::new(p.x.max(q.x) + 1e-7, p.y.max(q.y) + 1e-7),
        );
        let mut found: Vec<(f64, Point2)> = corners
            .iter()
            .filter(|c| c.x >= lo.x && c.x <= hi.x && c.y >= lo.y && c.y <= hi.y)
            .filter_map(|c| {
                let t = (*c - p).dot(d) / len2;
                let off = (*c - p).cross(d).abs() / len2.sqrt();
                (t > 1e-9 && t < 1.0 - 1e-9 && off < 1e-7).then_some((t, *c))
            })
            .collect();
        found.sort_by(|a, b| a.0.total_cmp(&b.0));
        found.into_iter().map(|(_, c)| c).collect()
    };
    // A plan triangle split at the corners on its edges (fans from the
    // opposite corner).
    fn split(
        tri: [Point2; 3],
        on: &dyn Fn(Point2, Point2) -> Vec<Point2>,
        out: &mut Vec<[Point2; 3]>,
        depth: usize,
    ) {
        for e in 0..3 {
            let (a, b, c) = (tri[e], tri[(e + 1) % 3], tri[(e + 2) % 3]);
            if let Some(m) = on(a, b).first().copied().filter(|_| depth < 64) {
                split([a, m, c], on, out, depth + 1);
                split([m, b, c], on, out, depth + 1);
                return;
            }
        }
        out.push(tri);
    }
    let caps = |tris: &mut Vec<[P3; 3]>, shapes: &[Shape], depth: f64, up: bool| {
        for (outer, holes) in shapes {
            let poly = with_holes(outer, holes);
            for [x, y, z] in triangulate(&poly) {
                let mut pieces = Vec::new();
                split([poly[x], poly[y], poly[z]], &on, &mut pieces, 0);
                for [a, b, c] in pieces {
                    let (a, b, c) = (at(a, depth), at(b, depth), at(c, depth));
                    tris.push(if up { [a, b, c] } else { [a, c, b] });
                }
            }
        }
    };
    (0..count)
        .map(|i| {
            let (upper, bottom) = (depths[i], depths[i + 1]);
            let mut tris = Vec::new();
            caps(&mut tris, &above[i], upper, true);
            caps(&mut tris, &below[i], bottom, false);
            // Edges: round each outline and each hole.
            for (outer, holes) in &regions[i] {
                let mut rings = vec![ensure_ccw(outer.clone())];
                rings.extend(holes.iter().map(|h| {
                    let mut h = ensure_ccw(h.clone());
                    h.reverse();
                    h
                }));
                for ring in &rings {
                    for k in 0..ring.len() {
                        let (p0, q0) = (ring[k], ring[(k + 1) % ring.len()]);
                        let mut pts = vec![p0];
                        pts.extend(on(p0, q0));
                        pts.push(q0);
                        for w in pts.windows(2) {
                            let (p, q) = (w[0], w[1]);
                            tris.push([at(p, bottom), at(q, bottom), at(q, upper)]);
                            tris.push([at(p, bottom), at(q, upper), at(p, upper)]);
                        }
                    }
                }
            }
            Solid {
                owner,
                material: layers[i].material,
                triangles: tris,
            }
        })
        .collect()
}

/// Solids of a door or window: the 3D body of its family, one solid per
/// material (frame or lining, leaves or glazing…).
///
/// The family works in its own axes: `x` along the wall from the centre of
/// the opening, to the right seen from the side the leaves open to; `y`
/// across the wall from the middle of the frame, towards that side; `z` up
/// from the sill.
pub fn opening_solids(model: &Model, id: OpeningId) -> Vec<Solid> {
    match model.evaluate_opening(id) {
        Ok(ev) => opening_body_solids(model, id, &ev),
        Err(_) => Vec::new(),
    }
}

/// The solids of a door or window drawn with its detailed model (see
/// [`opening_detail`]): its body without what the model makes (the
/// leaves), or the whole body when it has no detailed model.
pub fn opening_solids_detailed(model: &Model, id: OpeningId) -> Vec<Solid> {
    if opening_detail(model, id).is_none() {
        return opening_solids(model, id);
    }
    match model.evaluate_opening_detailed(id) {
        Ok(ev) => opening_body_solids(model, id, &ev),
        Err(_) => Vec::new(),
    }
}

/// A door or window's detailed model (paquete 9: leaves with their
/// panels, casings, hardware), made once per type and hand and drawn
/// where it stands: the model in the family's axes, mirrored when the
/// door is, its material by part, turned about the vertical and moved to
/// the middle of the frame at the sill.
pub struct OpeningDetail {
    pub mesh: std::sync::Arc<archisim_core::props::PropMesh>,
    pub parts: Vec<(MaterialId, Vec<[u32; 3]>)>,
    pub rotation: f64,
    pub origin: P3,
    /// The wall it stands in.
    pub wall: archisim_core::WallId,
}

/// See [`OpeningDetail`]; `None` for a family that makes none.
pub fn opening_detail(model: &Model, id: OpeningId) -> Option<OpeningDetail> {
    opening_detail_with(model, id, false)
}

/// [`opening_detail`], or with `simple` its plain version for far away:
/// the frame, the leaves and the glass, without hardware, beads, glazing
/// bars or scrolls (the generator's `lod`).
pub fn opening_detail_with(model: &Model, id: OpeningId, simple: bool) -> Option<OpeningDetail> {
    opening_detail_of(model, id, simple, 0.0)
}

/// What goes outside a window, made as an element of its own.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outside {
    /// The grille (`IfcRailing`).
    Grille,
    /// The shutters (`IfcShadingDevice`).
    Shutters,
}

/// What goes outside a window (its grille, its shutters), as solids where
/// they are, in their plain version (the elevations, the IFC): nothing for
/// a window without them or a door.
pub fn opening_outside_solids(model: &Model, id: OpeningId, which: Outside) -> Vec<Solid> {
    let Some(t) = model
        .opening(id)
        .and_then(|o| model.opening_type(o.opening_type))
    else {
        return Vec::new();
    };
    if !matches!(
        model.family_prop(t.family).as_deref(),
        Some("window" | "slider" | "sash_window")
    ) {
        return Vec::new();
    }
    let only = match which {
        Outside::Grille => 1.0,
        Outside::Shutters => 2.0,
    };
    let Some(d) = opening_detail_of(model, id, true, only) else {
        return Vec::new();
    };
    let (s, c) = d.rotation.sin_cos();
    let place = |q: [f32; 3]| -> P3 {
        let (x, y, z) = (q[0] as f64, q[1] as f64, q[2] as f64);
        [
            d.origin[0] + c * x - s * y,
            d.origin[1] + s * x + c * y,
            d.origin[2] + z,
        ]
    };
    d.parts
        .iter()
        .map(|(material, tris)| Solid {
            owner: SolidOwner::Opening(id),
            material: *material,
            triangles: tris
                .iter()
                .map(|t| t.map(|i| place(d.mesh.positions[i as usize])))
                .collect(),
        })
        .filter(|s| !s.triangles.is_empty())
        .collect()
}

/// The weight of a window's grille, kg: wrought iron, 7 850 kg/m³, by the
/// volume of its bars (in their plain version, all closed pieces); 0
/// without one.
pub fn grille_weight(model: &Model, id: OpeningId) -> f64 {
    let volume: f64 = opening_outside_solids(model, id, Outside::Grille)
        .iter()
        .map(|s| {
            s.triangles
                .iter()
                .map(|[a, b, c]| {
                    (a[0] * (b[1] * c[2] - b[2] * c[1]) - a[1] * (b[0] * c[2] - b[2] * c[0])
                        + a[2] * (b[0] * c[1] - b[1] * c[0]))
                        / 6.0
                })
                .sum::<f64>()
                .abs()
        })
        .sum();
    (volume * 7850.0 * 10.0).round() / 10.0
}

fn opening_detail_of(
    model: &Model,
    id: OpeningId,
    simple: bool,
    only: f64,
) -> Option<OpeningDetail> {
    let o = model.opening(id)?;
    let t = model.opening_type(o.opening_type)?;
    model.family_prop(t.family)?;
    let og = opening_geometry(model, id)?;
    let wall = model.wall(o.wall)?;
    let level = model.level(wall.level).map_or(0.0, |l| l.elevation);
    let mid = (og.t0 + og.t1) / 2.0;
    let y_mid = (og.frame_exterior + og.frame_interior) / 2.0;
    let (sx, sy) = match og.opens_to {
        WallSide::Exterior => (-1.0, 1.0),
        WallSide::Interior => (1.0, -1.0),
    };
    // The wall's faces in the family's axes (`y` = sy · (offset − y_mid)).
    let ext = sy * (og.exterior_offset - y_mid);
    let int = sy * (og.interior_offset - y_mid);
    let (face_pos, face_neg) = (ext.max(int), ext.min(int));
    // A façade takes casings on its inside only; a partition, on both.
    let exterior = model
        .wall_type(wall.wall_type)
        .is_some_and(|wt| wt.function == archisim_core::WallFunction::Exterior);
    let ext_is_pos = ext > int;
    let (casing_pos, casing_neg) = if exterior {
        (!ext_is_pos, ext_is_pos)
    } else {
        (true, true)
    };
    // The family's x and y in the model: x along `sx · u`, y along
    // `sy · n`; when that turns the model over, it is made mirrored.
    let ax = [sx * og.u.x, sx * og.u.y];
    let ay = [sy * og.n.x, sy * og.n.y];
    let mirror = ax[0] * ay[1] - ax[1] * ay[0] < 0.0;
    let b = |v: bool| if v { 1.0 } else { 0.0 };
    let (mesh, parts) = model.opening_prop(
        id,
        &[
            ("face_pos", face_pos),
            ("face_neg", face_neg),
            ("face_ext", ext),
            ("casing_pos", b(casing_pos)),
            ("casing_neg", b(casing_neg)),
            ("mirror", b(mirror)),
            ("lod", b(simple)),
            ("only", only),
        ],
    )?;
    let x_axis = if mirror { [-ax[0], -ax[1]] } else { ax };
    let origin = og.point(mid, y_mid);
    Some(OpeningDetail {
        mesh,
        parts,
        rotation: x_axis[1].atan2(x_axis[0]),
        origin: [origin.x, origin.y, level + og.sill],
        wall: o.wall,
    })
}
/// The pieces of a door or window family evaluated, placed: `point(t, y)`
/// is the plan point `t` along its host and `y` towards its exterior;
/// `at` is the middle of its frame along, across and the height of its
/// sill; its leaves open to `opens_to`. By material.
#[allow(clippy::type_complexity)]
fn family_body(
    ev: &archisim_core::family::Evaluation,
    point: &dyn Fn(f64, f64) -> Point2,
    at: (f64, f64, f64),
    opens_to: WallSide,
) -> Vec<(MaterialId, Vec<[P3; 3]>)> {
    let (mid, y_mid, z_sill) = at;
    // The normal points to the exterior. Seen from the exterior, the
    // host's direction runs to the left; seen from the interior, to the
    // right.
    let (sx, sy) = match opens_to {
        WallSide::Exterior => (-1.0, 1.0),
        WallSide::Interior => (1.0, -1.0),
    };
    let mut by_material: Vec<(MaterialId, Vec<[P3; 3]>)> = Vec::new();
    // The pieces of one `outline` (a frame round an arch, a surround) as
    // one body: no faces between them.
    type Group<'a> = ((usize, u64, i64, i64), Vec<&'a Vec<[f64; 2]>>, f64, f64);
    let mut groups: Vec<Group> = Vec::new();
    for part in &ev.parts {
        if let Some(profile) = &part.profile {
            let key = (
                part.line,
                part.material,
                (part.min[1] * 1e6).round() as i64,
                (part.max[1] * 1e6).round() as i64,
            );
            match groups.iter_mut().find(|g| g.0 == key) {
                Some(g) => g.1.push(profile),
                None => groups.push((key, vec![profile], part.min[1], part.max[1])),
            }
        }
    }
    for (key, pieces, y0, y1) in &groups {
        let tris = profile_group(pieces, *y0, *y1, sx * sy, |x, y, z| {
            let p = point(mid + sx * x, y_mid + sy * y);
            [p.x, p.y, z_sill + z]
        });
        let material = MaterialId(key.1);
        match by_material.iter_mut().find(|(m, _)| *m == material) {
            Some((_, list)) => list.extend(tris),
            None => by_material.push((material, tris)),
        }
    }
    for part in ev.parts.iter().filter(|p| p.profile.is_none()) {
        let tris = {
            let (ta, tb) = sorted(mid + sx * part.min[0], mid + sx * part.max[0]);
            let (ya, yb) = sorted(y_mid + sy * part.min[1], y_mid + sy * part.max[1]);
            let rect = [point(ta, ya), point(tb, ya), point(tb, yb), point(ta, yb)];
            prism(&rect, z_sill + part.min[2], z_sill + part.max[2])
        };
        let material = MaterialId(part.material);
        match by_material.iter_mut().find(|(m, _)| *m == material) {
            Some((_, list)) => list.extend(tris),
            None => by_material.push((material, tris)),
        }
    }
    by_material
}

/// The doors of a curtain wall, each in its cell, by material.
pub fn curtain_door_solids(
    model: &Model,
    id: archisim_core::CurtainWallId,
) -> Vec<(MaterialId, Vec<[P3; 3]>)> {
    let Some(c) = model.curtain_wall(id) else {
        return Vec::new();
    };
    let mut out: Vec<(MaterialId, Vec<[P3; 3]>)> = Vec::new();
    for i in 0..c.doors.len() {
        for (m, tris) in curtain_door_body(model, id, i) {
            match out.iter_mut().find(|(k, _)| *k == m) {
                Some((_, list)) => list.extend(tris),
                None => out.push((m, tris)),
            }
        }
    }
    out
}

/// One door of a curtain wall in its cell, by material (empty if it does
/// not evaluate).
pub fn curtain_door_body(
    model: &Model,
    id: archisim_core::CurtainWallId,
    index: usize,
) -> Vec<(MaterialId, Vec<[P3; 3]>)> {
    let Some(c) = model.curtain_wall(id) else {
        return Vec::new();
    };
    let Some(door) = c.doors.get(index) else {
        return Vec::new();
    };
    let Ok((ev, cell)) = model.evaluate_curtain_door(id, index) else {
        return Vec::new();
    };
    let z = model.level(c.level).map_or(0.0, |l| l.elevation) + c.base_offset;
    let mid = (cell.along.0 + cell.along.1) / 2.0;
    family_body(
        &ev,
        &|t, y| c.point_at(t, y),
        (mid, 0.0, z + cell.up.0),
        door.opens_to,
    )
}

/// The solids of the recipe's body evaluated as `ev`.
fn opening_body_solids(
    model: &Model,
    id: OpeningId,
    ev: &archisim_core::family::Evaluation,
) -> Vec<Solid> {
    let (Some(o), Some(og)) = (model.opening(id), opening_geometry(model, id)) else {
        return Vec::new();
    };
    let Some(wall) = model.wall(o.wall) else {
        return Vec::new();
    };
    let level = model.level(wall.level).map_or(0.0, |l| l.elevation);
    let z_sill = level + og.sill;
    let mid = (og.t0 + og.t1) / 2.0;
    let y_mid = (og.frame_exterior + og.frame_interior) / 2.0;
    let by_material = family_body(
        ev,
        &|t, y| og.point(t, y),
        (mid, y_mid, z_sill),
        og.opens_to,
    );
    by_material
        .into_iter()
        .map(|(material, triangles)| Solid {
            owner: SolidOwner::Opening(id),
            material,
            triangles,
        })
        .collect()
}

/// The opening lines a door or window's recipe draws in elevation, in the
/// model (none when its type hides them there,
/// [`Model::opening_lines_in_views`]): each segment twice, on the face of the element its leaves open
/// to and on the other, and whether its line style is not continuous. The
/// last value is the horizontal direction the leaves open to (a viewer on
/// that side sees the lines as drawn; from the other side, they open away).
pub fn opening_elevation_lines(model: &Model, id: OpeningId) -> (Vec<ElevationLine>, [f64; 2]) {
    let (Some(o), Some(og)) = (model.opening(id), opening_geometry(model, id)) else {
        return (Vec::new(), [0.0, 0.0]);
    };
    let (Ok(ev), Ok(marks)) = (
        model.evaluate_opening(id),
        model.opening_elevation_marks(id),
    ) else {
        return (Vec::new(), [0.0, 0.0]);
    };
    let Some(wall) = model.wall(o.wall) else {
        return (Vec::new(), [0.0, 0.0]);
    };
    let level = model.level(wall.level).map_or(0.0, |l| l.elevation);
    let z_sill = level + og.sill;
    let mid = (og.t0 + og.t1) / 2.0;
    let y_mid = (og.frame_exterior + og.frame_interior) / 2.0;
    let (sx, sy) = match og.opens_to {
        WallSide::Exterior => (-1.0, 1.0),
        WallSide::Interior => (1.0, -1.0),
    };
    // Just outside the body on each side.
    let front = ev.parts.iter().map(|p| p.max[1]).fold(0.0_f64, f64::max) + 0.002;
    let back = ev.parts.iter().map(|p| p.min[1]).fold(0.0_f64, f64::min) - 0.002;
    let at = |x: f64, y: f64, z: f64| {
        let p = og.point(mid + sx * x, y_mid + sy * y);
        [p.x, p.y, z_sill + z]
    };
    // Windows draw them only when asked (`Model::opening_lines_in_views`).
    let marks = if model.opening_lines_in_views(id) {
        marks
    } else {
        Vec::new()
    };
    let mut out = Vec::new();
    let mut push = |a: [f64; 2], b: [f64; 2], dashed: bool| {
        out.push(ElevationLine {
            front: (at(a[0], front, a[1]), at(b[0], front, b[1])),
            back: (at(a[0], back, a[1]), at(b[0], back, b[1])),
            dashed,
        });
    };
    for m in &marks {
        match m {
            archisim_core::Mark::Line {
                from, to, style, ..
            } => push(*from, *to, *style != archisim_core::LineStyle::Continuous),
            archisim_core::Mark::Polygon { points, style, .. } => {
                for i in 0..points.len() {
                    push(
                        points[i],
                        points[(i + 1) % points.len()],
                        *style != archisim_core::LineStyle::Continuous,
                    );
                }
            }
            archisim_core::Mark::Arc { .. } => {}
        }
    }
    let n = og.n;
    (out, [n.x * sy, n.y * sy])
}

/// One opening line of a door or window in elevation (see
/// [`opening_elevation_lines`]).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ElevationLine {
    pub front: (P3, P3),
    pub back: (P3, P3),
    pub dashed: bool,
}

fn sorted(a: f64, b: f64) -> (f64, f64) {
    if a <= b {
        (a, b)
    } else {
        (b, a)
    }
}

/// Segments used for round sections in 3D and IFC tessellations.
pub const ROUND_SEGMENTS: usize = 32;

/// The solid of a column: its section extruded from base to top.
pub fn column_solids(model: &Model, id: ColumnId) -> Vec<Solid> {
    let Some(c) = model.column(id) else {
        return Vec::new();
    };
    let Some(t) = model.member_type(c.member_type) else {
        return Vec::new();
    };
    let base = model.level(c.level).map_or(0.0, |l| l.elevation) + c.base_offset;
    let outline = c.plan_outline(t.profile, ROUND_SEGMENTS);
    vec![Solid {
        owner: SolidOwner::Column(id),
        material: t.material,
        triangles: hollow_prism(
            &outline,
            c.plan_hole(t.profile, ROUND_SEGMENTS).as_deref(),
            base,
            base + c.height,
        ),
    }]
}

/// The solid of a beam: its section swept from start to end, with the
/// top of the section at the beam's top face.
pub fn beam_solids(model: &Model, id: BeamId) -> Vec<Solid> {
    let Some(b) = model.beam(id) else {
        return Vec::new();
    };
    let Some(t) = model.member_type(b.member_type) else {
        return Vec::new();
    };
    let Some(u) = (b.end - b.start).normalized() else {
        return Vec::new();
    };
    let n = u.perp_left();
    let top = model.level(b.level).map_or(0.0, |l| l.elevation) + b.top_offset;
    let (_, depth) = t.profile.size();
    let z_mid = top - depth / 2.0;
    // Section `x` → across the beam (left), section `y` → up, extrusion →
    // along the beam: a right-handed frame, so windings stay outward.
    let local = hollow_prism(
        &t.profile.outline(ROUND_SEGMENTS),
        t.profile.hole(ROUND_SEGMENTS).as_deref(),
        0.0,
        b.length(),
    );
    let map = |[x, y, s]: P3| {
        let p = b.start + n * x + u * s;
        [p.x, p.y, z_mid + y]
    };
    vec![Solid {
        owner: SolidOwner::Beam(id),
        material: t.material,
        triangles: local
            .into_iter()
            .map(|[a, b2, c]| [map(a), map(b2), map(c)])
            .collect(),
    }]
}

/// Every solid of the model, on every level.
/// The triangles of a mesh, as points.
pub fn mesh_triangles(m: &archisim_core::MassMesh) -> Vec<[P3; 3]> {
    m.triangles
        .iter()
        .map(|t| t.map(|i| m.positions[i as usize]))
        .collect()
}

pub fn model_solids(model: &Model) -> Vec<Solid> {
    model_solids_except(model, &Default::default())
}

/// An object whose model is the same for every object of its type (the
/// furniture ArchiSim makes): the model in the object's own axes, with
/// the material of each part, and where the object stands. The 3D view
/// sends each model to the graphics card once and draws it wherever it
/// stands, instead of a copy per object.
pub struct ObjectInstance {
    pub mesh: std::sync::Arc<archisim_core::props::PropMesh>,
    pub parts: Vec<(MaterialId, Vec<[u32; 3]>)>,
    /// Turned this much about the vertical (radians), then moved here.
    pub rotation: f64,
    pub origin: P3,
}

/// The object as an [`ObjectInstance`], if it is one (not a plant, which
/// grows differently each time, nor a model brought from elsewhere).
pub fn object_instance(model: &Model, id: archisim_core::ObjectId) -> Option<ObjectInstance> {
    let o = model.object(id)?;
    if model.type_grows(o.object_type) || model.object_mesh(id).is_some() {
        return None;
    }
    let (mesh, parts) = model.object_prop(id)?;
    let z0 = model.level(o.level).map_or(0.0, |l| l.elevation) + o.base_offset;
    Some(ObjectInstance {
        mesh,
        parts,
        rotation: o.rotation,
        origin: [o.position.x, o.position.y, z0],
    })
}

/// The solids of the model but those of the objects in `skip` (drawn as
/// instances); a skipped object that carries the worktop of its run
/// still gives the worktop.
pub fn model_solids_except(
    model: &Model,
    skip: &std::collections::HashSet<archisim_core::ObjectId>,
) -> Vec<Solid> {
    model_solids_except_detailed(model, skip, &Default::default())
}

/// [`model_solids_except`], the doors and windows in `detailed` without
/// what their detailed model makes (drawn as instances too).
pub fn model_solids_except_detailed(
    model: &Model,
    skip: &std::collections::HashSet<archisim_core::ObjectId>,
    detailed: &std::collections::HashSet<OpeningId>,
) -> Vec<Solid> {
    let mut out = Vec::new();
    let idx = WallIndex::new(model);
    let pieces = facade_pieces_wanted();
    for w in model.walls() {
        let solids = wall_solids_in(&idx, w.id);
        // The bricks, blocks or panels of its façade (E9).
        if pieces && crate::facade::has_facade(model, w.id) {
            out.extend(crate::facade::facade_solids_in(&idx, w.id, &solids));
        }
        // The leaves of its green faces (H5).
        if pieces && growth_wanted() {
            out.extend(crate::growth::green_wall_solids(model, w.id, &solids));
        }
        out.extend(solids);
    }
    for s in model.slabs() {
        out.extend(slab_solids_in(&idx, s.id));
    }
    // The accessories of their types: skirtings, bands, cornices, copings
    // and slab edges (bloque 5).
    for w in model.walls() {
        for p in crate::sweeps::wall_sweep_pieces_in(&idx, w.id) {
            out.push(Solid {
                owner: SolidOwner::Wall(w.id),
                material: p.material,
                triangles: mesh_triangles(&p.mesh),
            });
        }
    }
    for s in model.slabs() {
        if let Some(p) = crate::sweeps::slab_edge_piece(model, s.id) {
            out.push(Solid {
                owner: SolidOwner::Slab(s.id),
                material: p.material,
                triangles: mesh_triangles(&p.mesh),
            });
        }
    }
    for o in model.openings() {
        if detailed.contains(&o.id) {
            out.extend(opening_solids_detailed(model, o.id));
        } else {
            out.extend(opening_solids(model, o.id));
            // What goes outside a window: its grille and its shutters.
            for which in [Outside::Grille, Outside::Shutters] {
                out.extend(opening_outside_solids(model, o.id, which));
            }
        }
    }
    for c in model.columns() {
        out.extend(column_solids(model, c.id));
    }
    for b in model.beams() {
        out.extend(beam_solids(model, b.id));
    }
    for r in model.roofs() {
        out.extend(roof_solids(model, r.id));
        // What it carries: dormers, tiles, gutters (bloque K).
        for i in 0..r.dormers.len() {
            out.extend(crate::roof_parts::dormer_solids(model, r.id, i));
        }
        out.extend(crate::roof_parts::tile_solids(model, r.id));
        out.extend(crate::roof_parts::gutter_solids(model, r.id));
    }
    for s in model.stairs() {
        out.extend(stair_solids(model, s.id));
    }
    for t in model.terrains() {
        out.extend(terrain_solids(model, t.id));
        if growth_wanted() {
            out.extend(crate::growth::growth_solids(model, t.id));
        }
    }
    for c in model.curtain_walls() {
        out.extend(curtain_wall_solids(model, c.id));
    }
    for r in model.railings() {
        out.extend(loose_railing_solids(model, r.id));
    }
    for o in model.objects() {
        if skip.contains(&o.id) {
            if let Some(run) = model.worktop_run_of(o.id).filter(|r| r.members[0] == o.id) {
                out.extend(worktop_solids(model, &run, SolidOwner::Object(o.id)));
            }
            continue;
        }
        out.extend(object_solids(model, o.id));
    }
    for f in model.footings() {
        out.extend(footing_solids(model, f.id));
    }
    // The bars of the reinforcement (E11).
    if pieces {
        for (id, _) in model.reinforcements() {
            out.extend(crate::rebar::rebar_solids(model, id));
        }
    }
    for p in model.pools() {
        out.extend(crate::pools::pool_solids(model, p.id));
    }
    for e in model.elevators() {
        out.extend(crate::elevators::elevator_solids(model, e.id));
    }
    for m in model.masses() {
        out.extend(mass_solids(model, m.id));
    }
    for l in model.levels() {
        out.extend(level_road_solids(model, l.id));
    }
    for r in model.runs() {
        out.extend(run_solids(model, r.id));
    }
    out.extend(fitting_solids(model));
    for d in model.devices() {
        out.extend(device_solids(model, d.id));
    }
    out.retain(|s| !s.triangles.is_empty());
    out
}

/// The solid of a footing: its outline from its top face down its depth.
pub fn footing_solids(model: &Model, id: archisim_core::FootingId) -> Vec<Solid> {
    let Some(f) = model.footing(id) else {
        return Vec::new();
    };
    let outline = f.outline();
    if outline.len() < 3 {
        return Vec::new();
    }
    let top = model.level(f.level).map_or(0.0, |l| l.elevation) + f.top_offset;
    vec![Solid {
        owner: SolidOwner::Footing(id),
        material: f.material,
        triangles: prism(&outline, top - f.depth, top),
    }]
}

/// The solids of a mass: its solid (a contour pushed up, a round form, or
/// what joining and cutting volumes gave), from its base, one solid per
/// material: its own, and the paint of the faces painted. A face (height
/// zero) is a centimetre thick so that it is seen. None once it is a
/// building.
pub fn mass_solids(model: &Model, id: archisim_core::MassId) -> Vec<Solid> {
    let Some(m) = model.mass(id) else {
        return Vec::new();
    };
    // Turned into a building: the building shows it.
    if m.building.is_some() {
        return Vec::new();
    }
    let body = crate::forms::mass_body(m);
    let base = model.level(m.level).map_or(0.0, |l| l.elevation) + m.base_offset;
    let mut by: std::collections::BTreeMap<archisim_core::MaterialId, Vec<[P3; 3]>> =
        Default::default();
    let mut paint: std::collections::BTreeMap<u32, archisim_core::MaterialId> = Default::default();
    for (t, f) in body.triangles.iter().zip(&body.faces) {
        let mat = *paint
            .entry(*f)
            .or_insert_with(|| model.face_paint(id.0, m.face_ref(*f)).unwrap_or(m.material));
        let tri = t.map(|i| {
            let p = body.positions[i as usize];
            [p[0], p[1], p[2] + base]
        });
        by.entry(mat).or_default().push(tri);
    }
    by.into_iter()
        .map(|(material, triangles)| Solid {
            owner: SolidOwner::Mass(id),
            material,
            triangles,
        })
        .collect()
}

/// The solids of a road: its pavement from the carriageway's top down
/// its depth, the kerbs standing their height over it, the sidewalks,
/// planting strips and medians on top of their fill (H6), and its road
/// markings a few millimetres over the carriageway when the road has their
/// material.
pub fn road_solids(model: &Model, id: archisim_core::RoadId) -> Vec<Solid> {
    let Some(r) = model.road(id) else {
        return Vec::new();
    };
    let junctions = crate::crossroads::junctions(model, r.level);
    road_solids_with(model, id, &junctions)
}

/// The solids of the roads of a level, their junctions worked out once.
pub fn level_road_solids(model: &Model, level: archisim_core::LevelId) -> Vec<Solid> {
    let junctions = crate::crossroads::junctions(model, level);
    model
        .roads_on_level(level)
        .flat_map(|r| road_solids_with(model, r.id, &junctions))
        .collect()
}

/// A road's pieces (but where its junctions are), following its profile,
/// its markings, and the junctions it owns (U1).
pub fn road_solids_with(
    model: &Model,
    id: archisim_core::RoadId,
    junctions: &[crate::crossroads::Junction],
) -> Vec<Solid> {
    use archisim_core::RoadBand;
    let Some(r) = model.road(id) else {
        return Vec::new();
    };
    let base = model.level(r.level).map_or(0.0, |l| l.elevation);
    let s = &r.section;
    let kh = if s.single_level { 0.0 } else { s.kerb_height };
    let gaps = crate::crossroads::road_gaps(junctions, id);
    let mut out: Vec<Solid> = r
        .pieces_except(&gaps)
        .into_iter()
        .map(|p| {
            let (d0, d1) = match (p.band, p.raised) {
                (RoadBand::Kerb, true) => (-s.depth, kh),
                (_, true) => (kh - 0.2, kh),
                _ => (-s.depth, 0.0),
            };
            Solid {
                owner: SolidOwner::Road(id),
                material: p.material,
                triangles: draped_prism(&p.outline, |q| {
                    let top = base + r.top_under(q);
                    (top + d0, top + d1)
                }),
            }
        })
        .collect();
    if let Some(m) = s.markings {
        let mut tris = Vec::new();
        for poly in r.markings_except(&gaps) {
            tris.extend(draped_prism(&poly, |q| {
                let top = base + r.top_under(q);
                (top, top + 0.004)
            }));
        }
        if !tris.is_empty() {
            out.push(Solid {
                owner: SolidOwner::Road(id),
                material: m,
                triangles: tris,
            });
        }
    }
    for j in junctions.iter().filter(|j| j.owner == id) {
        let top = base + j.top;
        let kh = j.kerb_height;
        out.push(Solid {
            owner: SolidOwner::Road(id),
            material: j.pavement,
            triangles: prism(&j.plate, top - j.depth, top),
        });
        for k in &j.kerbs {
            out.push(Solid {
                owner: SolidOwner::Road(id),
                material: j.kerb_material,
                triangles: prism(k, top - j.depth, top + kh),
            });
        }
        for w in &j.sidewalks {
            out.push(Solid {
                owner: SolidOwner::Road(id),
                material: j.sidewalk_material,
                triangles: prism(w, top + kh - 0.2, top + kh),
            });
        }
    }
    out
}

/// A cross-section (counter-clockwise, `x` across to the left, `y` up)
/// swept along a straight piece from `a` to `b`, with its two ends closed.
pub fn sweep_segment(section: &[(f64, f64)], a: P3, b: P3) -> Vec<[P3; 3]> {
    let d = [b[0] - a[0], b[1] - a[1], b[2] - a[2]];
    let len = (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).sqrt();
    let h = (d[0] * d[0] + d[1] * d[1]).sqrt();
    if len < EPSILON || h < EPSILON || section.len() < 3 {
        return Vec::new();
    }
    // Across (horizontal, to the left) and up.
    let u = [-d[1] / h, d[0] / h, 0.0];
    let v = [0.0, 0.0, 1.0];
    let at = |o: P3, (x, y): (f64, f64)| {
        [
            o[0] + u[0] * x + v[0] * y,
            o[1] + u[1] * x + v[1] * y,
            o[2] + u[2] * x + v[2] * y,
        ]
    };
    let n = section.len();
    let mut tris = Vec::new();
    for i in 1..n - 1 {
        // The end faces along the piece; the start, back.
        tris.push([at(b, section[0]), at(b, section[i]), at(b, section[i + 1])]);
        tris.push([at(a, section[0]), at(a, section[i + 1]), at(a, section[i])]);
    }
    for i in 0..n {
        let (p, q) = (section[i], section[(i + 1) % n]);
        tris.push([at(a, p), at(a, q), at(b, q)]);
        tris.push([at(a, p), at(b, q), at(b, p)]);
    }
    tris
}

/// The solids of a pipe or duct: one piece along each stretch of its
/// path, round (twelve sides) or rectangular.
/// Facets of a round pipe or light.
const ROUND_FACETS: usize = 24;

pub fn run_solids(model: &Model, id: archisim_core::RunId) -> Vec<Solid> {
    let Some(r) = model.run(id) else {
        return Vec::new();
    };
    let z0 = model.level(r.level).map_or(0.0, |l| l.elevation);
    let zs = r.point_elevations();
    let section: Vec<(f64, f64)> = if r.is_duct() {
        let (w, h) = (r.width / 2.0, r.height / 2.0);
        vec![(-w, -h), (w, -h), (w, h), (-w, h)]
    } else {
        // Facets that turn less than the 3D view's crease angle: a round
        // pipe, not a ribbed one.
        (0..ROUND_FACETS)
            .map(|i| {
                let a = i as f64 * std::f64::consts::TAU / ROUND_FACETS as f64;
                (r.width / 2.0 * a.cos(), r.width / 2.0 * a.sin())
            })
            .collect()
    };
    // Each stretch stops where the bend of its elbows starts (E13).
    let n = r.path.len();
    let cut = |i: usize| -> f64 {
        if i == 0 || i + 1 >= n {
            return 0.0;
        }
        elbow_cut(r, i)
    };
    let tris: Vec<[P3; 3]> = r
        .path
        .windows(2)
        .zip(zs.windows(2))
        .enumerate()
        .flat_map(|(i, (w, z))| {
            let len = w[0].distance(w[1]);
            let (a, b) = (cut(i).min(len / 2.0), cut(i + 1).min(len / 2.0));
            let at = |t: f64| {
                let f = if len > 1e-12 { t / len } else { 0.0 };
                let p = w[0] + (w[1] - w[0]) * f;
                [p.x, p.y, z0 + z[0] + (z[1] - z[0]) * f]
            };
            let mut out = sweep_segment(&section, at(a), at(len - b));
            // Its details (G4): a duct's flanges each 1.2 m, a pipe's
            // couplings each 3 m and its clamps each 1.5 m between them.
            let (step, wide, long) = if r.is_duct() {
                (1.2, 1.08, 0.03)
            } else {
                (1.5, 1.22, 0.02)
            };
            let collar = |k: f64, l: f64, t: f64| -> Vec<[P3; 3]> {
                let ring: Vec<(f64, f64)> = section.iter().map(|(x, y)| (x * k, y * k)).collect();
                sweep_segment(&ring, at(t - l / 2.0), at(t + l / 2.0))
            };
            let (from, to) = (a + 0.1, len - b - 0.1);
            let mut t = a + step;
            let mut i_step = 1;
            while t < to {
                if t > from {
                    let coupling = !r.is_duct() && i_step % 2 == 0;
                    if coupling {
                        out.extend(collar(1.12, 0.06, t));
                    } else {
                        out.extend(collar(wide, long, t));
                    }
                }
                t += step;
                i_step += 1;
            }
            // The ends of a duct: a flange each.
            if r.is_duct() && len - a - b > 0.2 {
                if i == 0 {
                    out.extend(collar(wide, long, a + long / 2.0));
                }
                if i + 2 == n {
                    out.extend(collar(wide, long, len - b - long / 2.0));
                }
            }
            out
        })
        .collect();
    vec![Solid {
        owner: SolidOwner::Run(id),
        material: archisim_core::MaterialId(0),
        triangles: tris,
    }]
}

/// The elbows and tees of the pipes and ducts (`Model::run_fittings`):
/// a sleeve a little wider than the run along each way out of them, with
/// the colour of its run.
/// The radius of the bends of a run: one and a half times a pipe's
/// diameter, a duct's width (E13).
pub fn bend_radius(r: &archisim_core::ServiceRun) -> f64 {
    if r.is_duct() {
        r.width
    } else {
        1.5 * r.width
    }
}

/// How far back from corner `i` of a run its bend starts, along each
/// stretch: the radius by the tangent of half the turn.
fn elbow_cut(r: &archisim_core::ServiceRun, i: usize) -> f64 {
    if i == 0 || i + 1 >= r.path.len() {
        return 0.0;
    }
    let (Some(a), Some(b)) = (
        (r.path[i] - r.path[i - 1]).normalized(),
        (r.path[i + 1] - r.path[i]).normalized(),
    ) else {
        return 0.0;
    };
    let turn = a.dot(b).clamp(-1.0, 1.0).acos();
    if !(0.05..=3.0).contains(&turn) {
        return 0.0;
    }
    bend_radius(r) * (turn / 2.0).tan()
}

pub fn fitting_solids(model: &Model) -> Vec<Solid> {
    let mut out = Vec::new();
    for f in model.run_fittings() {
        let Some(r) = model.run(f.run) else {
            continue;
        };
        let z = model.level(r.level).map_or(0.0, |l| l.elevation) + f.elevation;
        let grow = 0.01_f64.max(r.width * 0.15);
        let section: Vec<(f64, f64)> = if r.is_duct() {
            let (w, h) = ((r.width + grow) / 2.0, (r.height + grow) / 2.0);
            vec![(-w, -h), (w, -h), (w, h), (-w, h)]
        } else {
            (0..ROUND_FACETS)
                .map(|i| {
                    let a = i as f64 * std::f64::consts::TAU / ROUND_FACETS as f64;
                    let rad = (r.width + grow) / 2.0;
                    (rad * a.cos(), rad * a.sin())
                })
                .collect()
        };
        // An elbow: a bend of its radius from where one stretch stops to
        // where the next starts, and a sleeve at each end.
        if f.kind == archisim_core::FittingKind::Elbow {
            let i = r
                .path
                .iter()
                .position(|p| p.distance(f.position) < 1e-6)
                .unwrap_or(0);
            let t = elbow_cut(r, i);
            if t > 1e-6 {
                let a = [f.position.x - f.from.x * t, f.position.y - f.from.y * t, z];
                let b = [f.position.x + f.to.x * t, f.position.y + f.to.y * t, z];
                let c = [f.position.x, f.position.y, z];
                let pipe: Vec<(f64, f64)> = section
                    .iter()
                    .map(|(x, y)| {
                        (
                            x / (1.0 + grow / r.width.max(1e-6)),
                            *y / (1.0 + grow / r.width.max(1e-6)),
                        )
                    })
                    .collect();
                let mut tris = Vec::new();
                let steps = 8;
                let q = |u: f64| -> P3 {
                    let v = 1.0 - u;
                    [
                        v * v * a[0] + 2.0 * v * u * c[0] + u * u * b[0],
                        v * v * a[1] + 2.0 * v * u * c[1] + u * u * b[1],
                        z,
                    ]
                };
                for k in 0..steps {
                    let (u0, u1) = (k as f64 / steps as f64, (k + 1) as f64 / steps as f64);
                    tris.extend(sweep_segment(&pipe, q(u0), q(u1)));
                }
                let sleeve = (r.width.max(r.height) * 0.4).max(0.015);
                for (end, d) in [(a, -f.from), (b, f.to)] {
                    tris.extend(sweep_segment(
                        &section,
                        [end[0] - d.x * sleeve * 0.5, end[1] - d.y * sleeve * 0.5, z],
                        [end[0] + d.x * sleeve * 0.5, end[1] + d.y * sleeve * 0.5, z],
                    ));
                }
                out.push(Solid {
                    owner: SolidOwner::Run(f.run),
                    material: archisim_core::MaterialId(0),
                    triangles: tris,
                });
                continue;
            }
        }
        let leg = (r.width.max(r.height) * 0.75).max(0.03);
        let mut ways = vec![-f.from, f.to];
        if f.kind == archisim_core::FittingKind::Tee {
            ways.push(f.from);
        }
        let c = [f.position.x, f.position.y, z];
        let tris: Vec<[P3; 3]> = ways
            .into_iter()
            .flat_map(|d| sweep_segment(&section, c, [c[0] + d.x * leg, c[1] + d.y * leg, z]))
            .collect();
        out.push(Solid {
            owner: SolidOwner::Run(f.run),
            material: archisim_core::MaterialId(0),
            triangles: tris,
        });
    }
    // Where a run ends at a riser or downpipe: a bend from the run into
    // it, up or down towards the longer part of it (E13).
    for d in model.devices().filter(|d| d.kind.is_vertical()) {
        let base = model.level(d.level).map_or(0.0, |l| l.elevation) + d.elevation;
        let top = base + d.height;
        for r in model.runs() {
            if r.is_duct() || r.path.len() < 2 {
                continue;
            }
            let z0 = model.level(r.level).map_or(0.0, |l| l.elevation);
            let n = r.path.len();
            for (p, q, z) in [
                (r.path[0], r.path[1], z0 + r.elevation),
                (r.path[n - 1], r.path[n - 2], z0 + r.end_elevation()),
            ] {
                if p.distance(d.position) > d.width / 2.0 + r.width / 2.0 + 0.02
                    || z < base - 0.01
                    || z > top + 0.01
                {
                    continue;
                }
                let Some(away) = (q - p).normalized() else {
                    continue;
                };
                let rad = bend_radius(r).max(d.width / 2.0);
                let down = if top - z > z - base { 1.0 } else { -1.0 };
                let c = [d.position.x, d.position.y, z];
                let a = [c[0] + away.x * rad, c[1] + away.y * rad, z];
                let b = [c[0], c[1], z + down * rad];
                let section: Vec<(f64, f64)> = (0..ROUND_FACETS)
                    .map(|i| {
                        let t = i as f64 * std::f64::consts::TAU / ROUND_FACETS as f64;
                        (
                            r.width / 2.0 * 1.08 * t.cos(),
                            r.width / 2.0 * 1.08 * t.sin(),
                        )
                    })
                    .collect();
                let bez = |u: f64| -> P3 {
                    let v = 1.0 - u;
                    [
                        v * v * a[0] + 2.0 * v * u * c[0] + u * u * b[0],
                        v * v * a[1] + 2.0 * v * u * c[1] + u * u * b[1],
                        v * v * a[2] + 2.0 * v * u * c[2] + u * u * b[2],
                    ]
                };
                let mut tris = Vec::new();
                for k in 0..8 {
                    tris.extend(sweep_segment(
                        &section,
                        bez(k as f64 / 8.0),
                        bez((k + 1) as f64 / 8.0),
                    ));
                }
                out.push(Solid {
                    owner: SolidOwner::Run(r.id),
                    material: archisim_core::MaterialId(0),
                    triangles: tris,
                });
            }
        }
    }
    out
}

/// The solid of a device: its box (a light, a disc).
pub fn device_solids(model: &Model, id: archisim_core::DeviceId) -> Vec<Solid> {
    let Some(d) = model.device(id) else {
        return Vec::new();
    };
    let z = model.level(d.level).map_or(0.0, |l| l.elevation) + d.elevation;
    let (c, s) = (d.rotation.cos(), d.rotation.sin());
    let place = |p: [f64; 3]| -> P3 {
        [
            d.position.x + p[0] * c - p[1] * s,
            d.position.y + p[0] * s + p[1] * c,
            z + p[2],
        ]
    };
    // A light point with its fitting: the lamp, centred in the box, its
    // bottom on the box's (its top on the ceiling).
    if let Some((mesh, parts)) = d.fixture.and_then(|t| model.type_prop(t)) {
        let mut lo = [f64::MAX; 3];
        let mut hi = [f64::MIN; 3];
        for p in &mesh.positions {
            for k in 0..3 {
                lo[k] = lo[k].min(p[k] as f64);
                hi[k] = hi[k].max(p[k] as f64);
            }
        }
        if lo[0] < hi[0] {
            let mid = [(lo[0] + hi[0]) / 2.0, (lo[1] + hi[1]) / 2.0, lo[2]];
            let at = |i: u32| {
                let p = mesh.positions[i as usize];
                place([
                    p[0] as f64 - mid[0],
                    p[1] as f64 - mid[1],
                    p[2] as f64 - mid[2],
                ])
            };
            return parts
                .iter()
                .map(|(material, tris)| Solid {
                    owner: SolidOwner::Device(id),
                    material: *material,
                    triangles: tris
                        .iter()
                        .map(|t| [at(t[0]), at(t[1]), at(t[2])])
                        .collect(),
                })
                .collect();
        }
    }
    // Its own model, with the detail of the other pieces (G4).
    if let Some(mesh) = archisim_core::props::fine::services::device_model(
        d.kind.code(),
        d.width,
        d.depth,
        d.height,
    ) {
        let at = |i: u32| {
            let p = mesh.positions[i as usize];
            place([p[0] as f64, p[1] as f64, p[2] as f64])
        };
        return mesh
            .parts
            .iter()
            .filter_map(|(role, tris)| {
                let k = archisim_core::props::fine::services::ROLES
                    .iter()
                    .position(|r| r == role)?;
                Some(Solid {
                    owner: SolidOwner::Device(id),
                    material: archisim_core::MaterialId(DEVICE_ROLE_BASE + k as u64),
                    triangles: tris
                        .iter()
                        .map(|t| [at(t[0]), at(t[1]), at(t[2])])
                        .collect(),
                })
            })
            .collect();
    }
    let outline = if d.kind == archisim_core::DeviceKind::Light || d.kind.is_vertical() {
        (0..ROUND_FACETS)
            .map(|i| {
                let a = i as f64 * std::f64::consts::TAU / ROUND_FACETS as f64;
                d.position + Point2::new(a.cos(), a.sin()) * (d.width / 2.0)
            })
            .collect()
    } else {
        d.outline()
    };
    vec![Solid {
        owner: SolidOwner::Device(id),
        material: archisim_core::MaterialId(0),
        triangles: prism(&outline, z, z + d.height),
    }]
}

/// The pieces of a device's own model are not of a material of the
/// project: each role of `archisim_props::fine::services::ROLES` is
/// marked with this number plus its place, and [`solid_color`] gives its
/// colour.
pub const DEVICE_ROLE_BASE: u64 = u64::MAX - 63;

/// The solids of a stair, a railing or a curtain wall (what Pintar
/// paints by the material of its pieces).
pub fn piece_solids(model: &Model, owner: SolidOwner) -> Vec<Solid> {
    match owner {
        SolidOwner::Stair(id) => crate::stairs::stair_solids(model, id),
        SolidOwner::Railing(id) => loose_railing_solids(model, id),
        SolidOwner::CurtainWall(id) => curtain_wall_solids(model, id),
        _ => Vec::new(),
    }
}

/// The material of the piece of a solid nearest a point on it (the one
/// clicked).
pub fn material_at(solids: &[Solid], at: P3) -> Option<archisim_core::MaterialId> {
    let dist = |t: &[P3; 3]| -> f64 {
        // To the triangle: its plane if the point falls on it, else its
        // nearest corner.
        let u = [t[1][0] - t[0][0], t[1][1] - t[0][1], t[1][2] - t[0][2]];
        let v = [t[2][0] - t[0][0], t[2][1] - t[0][1], t[2][2] - t[0][2]];
        let w = [at[0] - t[0][0], at[1] - t[0][1], at[2] - t[0][2]];
        let d = |a: [f64; 3], b: [f64; 3]| a[0] * b[0] + a[1] * b[1] + a[2] * b[2];
        let (uu, uv, vv, wu, wv) = (d(u, u), d(u, v), d(v, v), d(w, u), d(w, v));
        let den = uu * vv - uv * uv;
        if den.abs() > 1e-18 {
            let a = (vv * wu - uv * wv) / den;
            let b = (uu * wv - uv * wu) / den;
            if a >= -1e-3 && b >= -1e-3 && a + b <= 1.0 + 1e-3 {
                let p = [
                    t[0][0] + u[0] * a + v[0] * b,
                    t[0][1] + u[1] * a + v[1] * b,
                    t[0][2] + u[2] * a + v[2] * b,
                ];
                let e = [at[0] - p[0], at[1] - p[1], at[2] - p[2]];
                return d(e, e).sqrt();
            }
        }
        t.iter()
            .map(|p| {
                let e = [at[0] - p[0], at[1] - p[1], at[2] - p[2]];
                d(e, e).sqrt()
            })
            .fold(f64::MAX, f64::min)
    };
    solids
        .iter()
        .flat_map(|s| s.triangles.iter().map(move |t| (s.material, t)))
        .min_by(|a, b| dist(a.1).total_cmp(&dist(b.1)))
        .map(|(m, _)| m)
}

/// The colour of a role of a device's own model.
fn device_role_color(d: &archisim_core::ServiceDevice, role: usize) -> [u8; 3] {
    match archisim_core::props::fine::services::ROLES
        .get(role)
        .copied()
    {
        Some("plate") => [243, 243, 239],
        Some("key") => [250, 250, 247],
        Some("metal") => [178, 181, 186],
        Some("dark") => [48, 48, 52],
        Some("glass") => [96, 104, 112],
        Some("body") => [238, 238, 234],
        Some("pipe") => d.kind.system().map_or([170, 140, 110], |s| s.color()),
        Some("shade") => [255, 244, 214],
        Some("red") => [205, 50, 45],
        Some("sign") => [30, 160, 90],
        Some("cell") => [29, 52, 98],
        _ => [235, 235, 230],
    }
}

/// How far a lamp's light reaches at night, metres; a streetlight's.
pub const LAMP_REACH: f64 = 4.5;
pub const STREET_REACH: f64 = 11.0;
/// The glow of an emergency light or an exit sign.
pub const SIGN_REACH: f64 = 1.5;

/// The materials of an object that glow at night: the shade and the bulb
/// of a lamp (also of a bedside table's), a candle's flame, the fire of a
/// fireplace, a streetlight's lantern; and how far its light reaches.
pub fn glow_materials(
    model: &Model,
    o: archisim_core::ObjectId,
) -> (Vec<archisim_core::MaterialId>, f64) {
    match model.object(o) {
        Some(ob) => type_glow_materials(model, ob.object_type),
        None => (Vec::new(), 0.0),
    }
}

/// Whether a solid glows at night, and how far its light reaches: a
/// lampshade or a bulb of an object or of a light point's lamp, or the
/// lens of a downlight.
pub fn solid_glow(model: &Model, s: &Solid) -> Option<f64> {
    match s.owner {
        SolidOwner::Object(o) => {
            let (mats, reach) = glow_materials(model, o);
            mats.contains(&s.material).then_some(reach)
        }
        SolidOwner::Device(id) => {
            let d = model.device(id)?;
            if s.material.0 >= DEVICE_ROLE_BASE {
                let role = archisim_core::props::fine::services::ROLES
                    .get((s.material.0 - DEVICE_ROLE_BASE) as usize)?;
                return match *role {
                    "shade" if d.kind.is_fire() => Some(SIGN_REACH),
                    "shade" => Some(LAMP_REACH),
                    "sign" => Some(SIGN_REACH),
                    _ => None,
                };
            }
            let (mats, reach) = type_glow_materials(model, d.fixture?);
            mats.contains(&s.material).then_some(reach)
        }
        // A lift's ceiling light.
        SolidOwner::Elevator(_) => {
            let k = s
                .material
                .0
                .checked_sub(crate::elevators::ELEVATOR_ROLE_BASE)?;
            (archisim_core::props::fine::lift::ROLES.get(k as usize) == Some(&"light"))
                .then_some(SIGN_REACH)
        }
        _ => None,
    }
}

/// [`glow_materials`] of a type of object.
pub fn type_glow_materials(
    model: &Model,
    t: archisim_core::ObjectTypeId,
) -> (Vec<archisim_core::MaterialId>, f64) {
    let none = (Vec::new(), 0.0);
    let Some(t) = model.object_type(t) else {
        return none;
    };
    let Some(kind) = model.family_prop(t.family) else {
        return none;
    };
    let (keys, reach): (&[&str], f64) = match kind.as_str() {
        "lamp" | "nightstand" | "decor" => (&["shade", "bulb", "flame"], LAMP_REACH),
        // The lamps of the collections (paquete 4).
        "pendant" | "ceiling_light" | "wall_light" | "floor_lamp" | "table_lamp" => {
            (&["shade", "bulb"], LAMP_REACH)
        }
        "fireplace" => (&["flame"], LAMP_REACH),
        "fine_fireplace" => (&["flame", "flame2"], LAMP_REACH),
        "streetlight" | "fine_streetlight" => (&["light"], STREET_REACH),
        // The lit strips of the bars (paquete 7).
        "fine_bar_counter" | "fine_back_bar" => (&["light"], LAMP_REACH),
        // The lights under water of the pools (paquete 8).
        "fine_pool" => (&["light"], LAMP_REACH),
        "fine_padel_court" => (&["light"], LAMP_REACH),
        _ => return none,
    };
    let Ok(defaults) = model.family_defaults(t.family, archisim_core::ParamScope::Type) else {
        return none;
    };
    let mats = keys
        .iter()
        .filter_map(|k| match t.values.get(*k).or(defaults.get(*k)) {
            Some(archisim_core::Value::Material(m)) => Some(archisim_core::MaterialId(*m)),
            _ => None,
        })
        .collect();
    (mats, reach)
}

/// The texture a material is seen with and the size of its piece
/// (metres): its appearance's, or, for furniture (`is_object`) whose
/// material has no appearance of its own, the one its name suggests
/// (linen, wool, rattan…). The 3D view, the rendered pictures and the
/// `.glb` lay it the same way.
pub fn texture_of(
    model: &Model,
    material: archisim_core::MaterialId,
    is_object: bool,
) -> Option<(archisim_core::TextureKind, f64)> {
    let a = model.appearance(material);
    let t = a.texture.or_else(|| {
        let x = model.material(material)?;
        (is_object && a.is_plain())
            .then(|| archisim_core::TextureKind::suggested(&x.name, &x.category))
            .flatten()
    })?;
    Some((t, a.scale.unwrap_or_else(|| t.default_scale())))
}

/// The colour a solid is shown with: a pipe's or duct's that of its
/// system, a device's a light grey, the rest their material's.
pub fn solid_color(model: &Model, s: &Solid) -> [u8; 3] {
    match s.owner {
        SolidOwner::Run(id) => model.run(id).map_or([150, 150, 150], |r| r.system.color()),
        // A piece of a device's own model, or of its lamp.
        SolidOwner::Device(id)
            if s.material.0 >= DEVICE_ROLE_BASE && model.device(id).is_some() =>
        {
            let d = model.device(id).expect("checked");
            device_role_color(d, (s.material.0 - DEVICE_ROLE_BASE) as usize)
        }
        SolidOwner::Device(id)
            if model.device(id).is_some_and(|d| d.fixture.is_some())
                && model.material(s.material).is_some() =>
        {
            model.surface_color(s.material)
        }
        SolidOwner::Elevator(_)
            if s.material.0 >= crate::elevators::ELEVATOR_ROLE_BASE
                && s.material.0 < DEVICE_ROLE_BASE =>
        {
            crate::elevators::role_color(
                (s.material.0 - crate::elevators::ELEVATOR_ROLE_BASE) as usize,
            )
        }
        SolidOwner::Device(id) => match model.device(id).map(|d| d.kind) {
            Some(archisim_core::DeviceKind::Light) => [250, 230, 150],
            Some(archisim_core::DeviceKind::Radiator) => [245, 245, 240],
            Some(archisim_core::DeviceKind::Grille | archisim_core::DeviceKind::Diffuser) => {
                [200, 205, 210]
            }
            // Risers and downpipes, in the colour of their system.
            Some(k @ (archisim_core::DeviceKind::Riser | archisim_core::DeviceKind::Downpipe)) => {
                k.system().map_or([150, 150, 150], |s| s.color())
            }
            Some(archisim_core::DeviceKind::Panel) => [210, 214, 220],
            _ => [235, 235, 230],
        },
        _ => model.surface_color(s.material),
    }
}

/// The face of an element a point with that normal is on: a side of a
/// wall (faces looking up or down, and the ends, are none); the top, the
/// bottom or a side of a volume.
pub fn face_of(
    model: &Model,
    owner: SolidOwner,
    at: P3,
    normal: P3,
) -> Option<archisim_core::FaceRef> {
    use archisim_core::{FaceRef, FaceSide};
    let len = (normal[0] * normal[0] + normal[1] * normal[1] + normal[2] * normal[2]).sqrt();
    if len < 1e-12 {
        return None;
    }
    let n = [normal[0] / len, normal[1] / len, normal[2] / len];
    let c = Point2::new(at[0], at[1]);
    let flat = Point2::new(n[0], n[1]);
    match owner {
        SolidOwner::Wall(id) | SolidOwner::Facade(id) => {
            if n[2].abs() > 0.5 {
                return None;
            }
            let w = model.wall(id)?;
            let left = match w.arc() {
                Some(a) => {
                    let r = (c - a.centre).normalized()?;
                    if a.sweep > 0.0 {
                        r * -1.0
                    } else {
                        r
                    }
                }
                None => (w.end - w.start).normalized()?.perp_left(),
            };
            let d = flat.x * left.x + flat.y * left.y;
            if d > 0.7 {
                Some(FaceRef::Wall(FaceSide::Exterior))
            } else if d < -0.7 {
                Some(FaceRef::Wall(FaceSide::Interior))
            } else {
                None
            }
        }
        SolidOwner::Mass(id) => {
            let m = model.mass(id)?;
            let base = model.level(m.level).map_or(0.0, |l| l.elevation) + m.base_offset;
            crate::forms::face_at(m, [at[0], at[1], at[2] - base], n).map(|f| m.face_ref(f))
        }
        // G3: the top, the bottom and each edge's side of a slab.
        SolidOwner::Slab(id) => {
            let sl = model.slab(id)?;
            if n[2] > 0.7 {
                return Some(FaceRef::Top);
            }
            if n[2] < -0.7 {
                return Some(FaceRef::Bottom);
            }
            let mut edges = Vec::new();
            for lp in std::iter::once(&sl.outline).chain(sl.holes.iter()) {
                let k = lp.len();
                for i in 0..k {
                    edges.push((lp[i], lp[(i + 1) % k]));
                }
            }
            nearest_edge(&edges, c).map(FaceRef::Side)
        }
        // A roof: each slope by the edge it falls to, its soffit, and the
        // fascia on each edge.
        SolidOwner::Roof(id) => {
            let r = model.roof(id)?;
            let k = r.outline.len();
            let edges: Vec<(Point2, Point2)> = (0..k)
                .map(|i| (r.outline[i], r.outline[(i + 1) % k]))
                .collect();
            if n[2] < -0.7 {
                return Some(FaceRef::Bottom);
            }
            if n[2] > 0.97 {
                return Some(FaceRef::Top);
            }
            if n[2] > 0.1 {
                // The edge whose outward side the slope looks to.
                let h = flat.normalized()?;
                let best = (0..k).max_by(|&a, &b| {
                    let out = |i: usize| {
                        let (p, q) = edges[i];
                        let d = q - p;
                        Point2::new(d.y, -d.x)
                            .normalized()
                            .map_or(-2.0, |o| o.x * h.x + o.y * h.y)
                    };
                    out(a).total_cmp(&out(b))
                })?;
                return Some(FaceRef::Side(best));
            }
            nearest_edge(&edges, c).map(|i| FaceRef::Part(i as u32))
        }
        // Columns, beams and footings: top, bottom and their sides by the
        // way they look (every 22,5°).
        SolidOwner::Column(_) | SolidOwner::Beam(_) | SolidOwner::Footing(_) => {
            if n[2] > 0.7 {
                return Some(FaceRef::Top);
            }
            if n[2] < -0.7 {
                return Some(FaceRef::Bottom);
            }
            let k = archisim_core::MEMBER_SIDES as f64;
            let a = flat.y.atan2(flat.x).rem_euclid(std::f64::consts::TAU);
            let i =
                ((a / std::f64::consts::TAU * k).round() as usize) % archisim_core::MEMBER_SIDES;
            Some(FaceRef::Side(i))
        }
        _ => None,
    }
}

/// The edge nearest a point.
fn nearest_edge(edges: &[(Point2, Point2)], c: Point2) -> Option<usize> {
    let d = |(a, b): (Point2, Point2)| {
        let ab = b - a;
        let l2 = ab.dot(ab);
        if l2 < 1e-18 {
            return c.distance(a);
        }
        let t = ((c - a).dot(ab) / l2).clamp(0.0, 1.0);
        c.distance(a + ab * t)
    };
    (0..edges.len()).min_by(|&i, &j| d(edges[i]).total_cmp(&d(edges[j])))
}

/// The paint on a triangle of a wall or a volume: the colour of the
/// material painted on the face it is part of, if that face is painted.
pub fn painted_color(model: &Model, s: &Solid, tri: &[P3; 3]) -> Option<[u8; 3]> {
    painted_material(model, s, tri).map(|m| model.surface_color(m))
}

/// The material painted on the face a triangle of a wall is on, if it is
/// painted. (A volume's painted faces come as solids of their own
/// material.)
pub fn painted_material(
    model: &Model,
    s: &Solid,
    tri: &[P3; 3],
) -> Option<archisim_core::MaterialId> {
    let element = match s.owner {
        // What is made of pieces: by the material of each.
        SolidOwner::Stair(_) | SolidOwner::Railing(_) | SolidOwner::CurtainWall(_) => {
            let e = s.owner.id();
            if !model.has_face_paint(e) {
                return None;
            }
            return model
                .face_paint(e, archisim_core::FaceRef::Material(s.material.0))
                .filter(|m| model.material(*m).is_some());
        }
        SolidOwner::Wall(id) | SolidOwner::Facade(id) => id.0,
        SolidOwner::Slab(_)
        | SolidOwner::Roof(_)
        | SolidOwner::Column(_)
        | SolidOwner::Beam(_)
        | SolidOwner::Footing(_) => {
            let e = s.owner.id();
            // Most are not painted: nothing to work out.
            if !model.has_face_paint(e) {
                return None;
            }
            e
        }
        _ => return None,
    };
    let u = [
        tri[1][0] - tri[0][0],
        tri[1][1] - tri[0][1],
        tri[1][2] - tri[0][2],
    ];
    let v = [
        tri[2][0] - tri[0][0],
        tri[2][1] - tri[0][1],
        tri[2][2] - tri[0][2],
    ];
    let n = [
        u[1] * v[2] - u[2] * v[1],
        u[2] * v[0] - u[0] * v[2],
        u[0] * v[1] - u[1] * v[0],
    ];
    let c = [
        (tri[0][0] + tri[1][0] + tri[2][0]) / 3.0,
        (tri[0][1] + tri[1][1] + tri[2][1]) / 3.0,
        (tri[0][2] + tri[1][2] + tri[2][2]) / 3.0,
    ];
    let face = face_of(model, s.owner, c, n)?;
    // (The zones of a face are drawn over it: `face_zone_triangles`.)
    let painted = model
        .face_paint(element, face)
        .filter(|m| model.material(*m).is_some());
    if painted.is_some() {
        return painted;
    }
    // The face behind the pieces of a façade shows in the joints: the
    // mortar's colour.
    match s.owner {
        SolidOwner::Wall(w) if crate::facade::on_facade_face(model, w, c, n) => {
            crate::facade::joint_material(model, w)
        }
        _ => None,
    }
}

/// How far a painted zone stands off its wall's face, so it is seen over it.
const ZONE_OFFSET: f64 = 0.002;

/// The triangles of the painted zones of walls' faces, just off the face,
/// with the wall they are on and their colour.
pub fn paint_zone_triangles(model: &Model) -> Vec<(archisim_core::WallId, [P3; 3], [u8; 3])> {
    let mut out = Vec::new();
    for z in model.paint_zones() {
        let Some(w) = model.wall(z.wall) else {
            continue;
        };
        let Some(t) = model.wall_type(w.wall_type) else {
            continue;
        };
        let Some(colour) = model.material(z.material).map(|m| m.cut_color) else {
            continue;
        };
        let (ext, int) = w.face_offsets(t.thickness());
        let off = match z.side {
            archisim_core::FaceSide::Exterior => ext + ZONE_OFFSET,
            archisim_core::FaceSide::Interior => int - ZONE_OFFSET,
        };
        let len = w.length().max(1e-9);
        let base = model.level(w.level).map_or(0.0, |l| l.elevation) + w.base_offset;
        let at = |p: Point2| -> P3 {
            let q = w.point_at((p.x / len).clamp(0.0, 1.0), off);
            [q.x, q.y, base + p.y]
        };
        let sign = match z.side {
            archisim_core::FaceSide::Exterior => 1.0,
            archisim_core::FaceSide::Interior => -1.0,
        };
        for [a, b, c] in triangulate(&z.outline) {
            let (pa, pb, pc) = (at(z.outline[a]), at(z.outline[b]), at(z.outline[c]));
            // Out of the wall on its side, where the triangle is.
            let f =
                ((z.outline[a].x + z.outline[b].x + z.outline[c].x) / 3.0 / len).clamp(0.0, 1.0);
            let out_dir = (w.point_at(f, 1.0) - w.point_at(f, 0.0)) * sign;
            let u = [pb[0] - pa[0], pb[1] - pa[1], pb[2] - pa[2]];
            let v = [pc[0] - pa[0], pc[1] - pa[1], pc[2] - pa[2]];
            let nx = u[1] * v[2] - u[2] * v[1];
            let ny = u[2] * v[0] - u[0] * v[2];
            let tri = if nx * out_dir.x + ny * out_dir.y < 0.0 {
                [pa, pc, pb]
            } else {
                [pa, pb, pc]
            };
            out.push((z.wall, tri, colour));
        }
    }
    out
}

/// The owner of an element's solids, by its id (the elements whose faces
/// are painted face by face).
fn plain_owner(model: &Model, id: u64) -> Option<SolidOwner> {
    use archisim_core::{BeamId, ColumnId, FootingId, RoofId, SlabId};
    if model.slab(SlabId(id)).is_some() {
        Some(SolidOwner::Slab(SlabId(id)))
    } else if model.roof(RoofId(id)).is_some() {
        Some(SolidOwner::Roof(RoofId(id)))
    } else if model.column(ColumnId(id)).is_some() {
        Some(SolidOwner::Column(ColumnId(id)))
    } else if model.beam(BeamId(id)).is_some() {
        Some(SolidOwner::Beam(BeamId(id)))
    } else if model.footing(FootingId(id)).is_some() {
        Some(SolidOwner::Footing(FootingId(id)))
    } else {
        None
    }
}

/// The triangles of the painted zones of faces looking up or down (G3),
/// just off the face, with their element and colour. The height of each
/// corner is the face's under it.
pub fn face_zone_triangles(model: &Model, solids: &[Solid]) -> Vec<(SolidOwner, [P3; 3], [u8; 3])> {
    face_zone_pieces(model, solids)
        .into_iter()
        .map(|(o, t, m)| (o, t, model.surface_color(m)))
        .collect()
}

/// The painted zones of the faces as solids of their element, one per
/// element and material, a hair over the face: what the rendered pictures
/// and the `.glb` add to [`model_solids`] (the 3D view draws them with
/// [`face_zone_triangles`]). Their material is the zone's: no face paint
/// applies to them.
pub fn face_zone_solids(model: &Model, solids: &[Solid]) -> Vec<Solid> {
    let mut out: Vec<Solid> = Vec::new();
    for (owner, tri, material) in face_zone_pieces(model, solids) {
        match out
            .iter_mut()
            .find(|s| s.owner == owner && s.material == material)
        {
            Some(s) => s.triangles.push(tri),
            None => out.push(Solid {
                owner,
                material,
                triangles: vec![tri],
            }),
        }
    }
    out
}

fn face_zone_pieces(
    model: &Model,
    solids: &[Solid],
) -> Vec<(SolidOwner, [P3; 3], archisim_core::MaterialId)> {
    let mut out = Vec::new();
    let zones: Vec<_> = model
        .face_zones()
        .filter(|z| z.material.is_some())
        .collect();
    for z in zones {
        let Some(owner) = plain_owner(model, z.element) else {
            continue;
        };
        let Some(m) = z.material.filter(|m| model.material(*m).is_some()) else {
            continue;
        };
        let Some(frame) = face_frame(model, z.element, z.face, Some(solids)) else {
            continue;
        };
        // Counter-clockwise, as the triangulation wants it.
        let mut outline = z.outline.clone();
        if archisim_core::signed_area(&outline) < 0.0 {
            outline.reverse();
        }
        for [a, b, c] in triangulate(&outline) {
            let (pa, pb, pc) = (
                frame.from_face(outline[a]),
                frame.from_face(outline[b]),
                frame.from_face(outline[c]),
            );
            // Facing the way the face looks.
            let u = [pb[0] - pa[0], pb[1] - pa[1], pb[2] - pa[2]];
            let v = [pc[0] - pa[0], pc[1] - pa[1], pc[2] - pa[2]];
            let nn = [
                u[1] * v[2] - u[2] * v[1],
                u[2] * v[0] - u[0] * v[2],
                u[0] * v[1] - u[1] * v[0],
            ];
            let dot = nn[0] * frame.normal[0] + nn[1] * frame.normal[1] + nn[2] * frame.normal[2];
            let tri = if dot >= 0.0 {
                [pa, pb, pc]
            } else {
                [pa, pc, pb]
            };
            out.push((owner, tri, m));
        }
    }
    out
}

/// A flat face of an element as «Dividir cara» draws on it (G3): its
/// triangles, the way it looks, and, for a face standing up, its own
/// axes in elevation.
#[derive(Debug, Clone)]
pub struct FaceFrame {
    pub triangles: Vec<[P3; 3]>,
    /// Unit, outwards.
    pub normal: P3,
    /// A face standing up: the corner its `x` starts from (in plan), the
    /// direction along it (left to right seen from outside) and the height
    /// of the element's level, its `y = 0`.
    pub vertical: Option<(Point2, Point2, f64)>,
}

impl FaceFrame {
    /// A point of the face in its coordinates: in plan, or along it and
    /// up from the level.
    pub fn to_face(&self, p: P3) -> Point2 {
        match self.vertical {
            Some((o, u, z0)) => Point2::new((p[0] - o.x) * u.x + (p[1] - o.y) * u.y, p[2] - z0),
            None => Point2::new(p[0], p[1]),
        }
    }

    /// The point of the face (a hair out of it) at its coordinates.
    pub fn from_face(&self, q: Point2) -> P3 {
        let n = self.normal;
        match self.vertical {
            Some((o, u, z0)) => [
                o.x + u.x * q.x + n[0] * ZONE_OFFSET,
                o.y + u.y * q.x + n[1] * ZONE_OFFSET,
                z0 + q.y + n[2] * ZONE_OFFSET,
            ],
            None => {
                // On the face over that point of the plan.
                let mut z = None;
                for t in &self.triangles {
                    let (a, b, c) = (t[0], t[1], t[2]);
                    let den = (b[1] - c[1]) * (a[0] - c[0]) + (c[0] - b[0]) * (a[1] - c[1]);
                    if den.abs() < 1e-12 {
                        continue;
                    }
                    let l1 = ((b[1] - c[1]) * (q.x - c[0]) + (c[0] - b[0]) * (q.y - c[1])) / den;
                    let l2 = ((c[1] - a[1]) * (q.x - c[0]) + (a[0] - c[0]) * (q.y - c[1])) / den;
                    let l3 = 1.0 - l1 - l2;
                    if l1 >= -1e-6 && l2 >= -1e-6 && l3 >= -1e-6 {
                        z = Some(l1 * a[2] + l2 * b[2] + l3 * c[2]);
                        break;
                    }
                }
                let z = z.unwrap_or_else(|| {
                    // Off the face: the height of its nearest corner.
                    self.triangles
                        .iter()
                        .flat_map(|t| t.iter())
                        .min_by(|a, b| {
                            let d = |p: &&P3| (p[0] - q.x).powi(2) + (p[1] - q.y).powi(2);
                            d(a).total_cmp(&d(b))
                        })
                        .map_or(0.0, |p| p[2])
                });
                [
                    q.x + n[0] * ZONE_OFFSET,
                    q.y + n[1] * ZONE_OFFSET,
                    z + n[2] * ZONE_OFFSET,
                ]
            }
        }
    }

    /// Its outline in its own coordinates, as a box (what a sketch in
    /// elevation shows as reference).
    pub fn extent(&self) -> Option<(Point2, Point2)> {
        let pts: Vec<Point2> = self
            .triangles
            .iter()
            .flat_map(|t| t.iter())
            .map(|p| self.to_face(*p))
            .collect();
        let lo = pts.iter().fold(Point2::new(f64::MAX, f64::MAX), |a, p| {
            Point2::new(a.x.min(p.x), a.y.min(p.y))
        });
        let hi = pts.iter().fold(Point2::new(f64::MIN, f64::MIN), |a, p| {
            Point2::new(a.x.max(p.x), a.y.max(p.y))
        });
        (lo.x <= hi.x).then_some((lo, hi))
    }
}

/// The solids of an element whose faces are painted and divided.
fn plain_solids(model: &Model, owner: SolidOwner) -> Vec<Solid> {
    match owner {
        SolidOwner::Slab(id) => slab_solids(model, id),
        SolidOwner::Roof(id) => roof_solids(model, id),
        SolidOwner::Column(id) => column_solids(model, id),
        SolidOwner::Beam(id) => beam_solids(model, id),
        SolidOwner::Footing(id) => footing_solids(model, id),
        _ => Vec::new(),
    }
}

/// The frame of a face of a slab, a roof, a column, a beam or a footing
/// (from `solids`, if given, or the element's own).
pub fn face_frame(
    model: &Model,
    element: u64,
    face: archisim_core::FaceRef,
    solids: Option<&[Solid]>,
) -> Option<FaceFrame> {
    let owner = plain_owner(model, element)?;
    let own;
    let solids = match solids {
        Some(s) => s,
        None => {
            own = plain_solids(model, owner);
            &own[..]
        }
    };
    let mut triangles = Vec::new();
    let mut normal = [0.0f64; 3];
    for t in solids
        .iter()
        .filter(|s| s.owner == owner)
        .flat_map(|s| s.triangles.iter())
    {
        let u = [t[1][0] - t[0][0], t[1][1] - t[0][1], t[1][2] - t[0][2]];
        let v = [t[2][0] - t[0][0], t[2][1] - t[0][1], t[2][2] - t[0][2]];
        let n = [
            u[1] * v[2] - u[2] * v[1],
            u[2] * v[0] - u[0] * v[2],
            u[0] * v[1] - u[1] * v[0],
        ];
        let c = [
            (t[0][0] + t[1][0] + t[2][0]) / 3.0,
            (t[0][1] + t[1][1] + t[2][1]) / 3.0,
            (t[0][2] + t[1][2] + t[2][2]) / 3.0,
        ];
        if face_of(model, owner, c, n) == Some(face) {
            triangles.push(*t);
            for k in 0..3 {
                normal[k] += n[k];
            }
        }
    }
    let len = (normal[0].powi(2) + normal[1].powi(2) + normal[2].powi(2)).sqrt();
    if triangles.is_empty() || len < 1e-12 {
        return None;
    }
    let normal = normal.map(|v| v / len);
    let in_plan = model.zone_in_plan(element, face) || normal[2].abs() > 0.6;
    let vertical = if in_plan {
        None
    } else {
        let h = (normal[0].powi(2) + normal[1].powi(2)).sqrt().max(1e-12);
        let u = Point2::new(-normal[1] / h, normal[0] / h);
        let o = triangles
            .iter()
            .flat_map(|t| t.iter())
            .min_by(|a, b| (a[0] * u.x + a[1] * u.y).total_cmp(&(b[0] * u.x + b[1] * u.y)))
            .map(|p| Point2::new(p[0], p[1]))?;
        let z0 = model
            .plain_level(element)
            .and_then(|l| model.level(l))
            .map_or(0.0, |l| l.elevation);
        Some((o, u, z0))
    };
    Some(FaceFrame {
        triangles,
        normal,
        vertical,
    })
}

/// Solids of the elements on one level only.
pub fn level_solids(model: &Model, level: LevelId) -> Vec<Solid> {
    let on_level = |owner: SolidOwner| match owner {
        SolidOwner::Wall(w) | SolidOwner::Facade(w) => {
            model.wall(w).is_some_and(|w| w.level == level)
        }
        SolidOwner::Slab(s) => model.slab(s).is_some_and(|s| s.level == level),
        SolidOwner::Opening(o) => model
            .opening(o)
            .and_then(|o| model.wall(o.wall))
            .is_some_and(|w| w.level == level),
        SolidOwner::Column(c) => model.column(c).is_some_and(|c| c.level == level),
        SolidOwner::Beam(b) => model.beam(b).is_some_and(|b| b.level == level),
        SolidOwner::Roof(r) | SolidOwner::Dormer(r, _) => {
            model.roof(r).is_some_and(|r| r.level == level)
        }
        SolidOwner::Stair(s) => model.stair(s).is_some_and(|s| s.level == level),
        SolidOwner::Terrain(t) | SolidOwner::Growth(t) => {
            model.terrain(t).is_some_and(|t| t.level == level)
        }
        SolidOwner::CurtainWall(c) => model.curtain_wall(c).is_some_and(|c| c.level == level),
        SolidOwner::Railing(r) => model.railing(r).is_some_and(|r| r.level == level),
        SolidOwner::Object(o) => model.object(o).is_some_and(|o| o.level == level),
        SolidOwner::Footing(f) => model.footing(f).is_some_and(|f| f.level == level),
        SolidOwner::Pool(p) => model.pool(p).is_some_and(|p| p.level == level),
        SolidOwner::Elevator(e) => model.elevator(e).is_some_and(|e| e.level == level),
        SolidOwner::Mass(m) => model.mass(m).is_some_and(|m| m.level == level),
        SolidOwner::Road(r) => model.road(r).is_some_and(|r| r.level == level),
        SolidOwner::Run(r) => model.run(r).is_some_and(|r| r.level == level),
        SolidOwner::Device(d) => model.device(d).is_some_and(|d| d.level == level),
    };
    model_solids(model)
        .into_iter()
        .filter(|s| on_level(s.owner))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::wall_geometry;

    /// R19: painting one face of a volume colours that face only.
    #[test]
    fn one_face_of_a_volume_is_painted() {
        use archisim_core::{FaceRef, MassShape, NewMass};
        let mut m = Model::new("T");
        let level = m.add_level("P1", 0.0);
        let mat = m.add_material("Hormigón", "concrete", [180, 180, 180], CutPattern::Solid);
        let red = m.add_material("Ladrillo", "brick", [200, 100, 80], CutPattern::Diagonal);
        let id = m
            .add_mass(NewMass {
                name: "V".into(),
                level,
                shape: MassShape::Prism(vec![
                    Point2::new(0.0, 0.0),
                    Point2::new(4.0, 0.0),
                    Point2::new(4.0, 3.0),
                    Point2::new(0.0, 3.0),
                ]),
                base_offset: 0.0,
                height: 2.0,
                floor_height: None,
                material: mat,
            })
            .unwrap();
        // The south side, hit from outside.
        let face = face_of(&m, SolidOwner::Mass(id), [2.0, 0.0, 1.0], [0.0, -1.0, 0.0]);
        assert_eq!(face, Some(FaceRef::Side(0)));
        m.set_face_paint(id.0, FaceRef::Side(0), Some(red)).unwrap();
        // The painted face comes as a solid of its own, in the paint.
        let solids = model_solids(&m);
        let of_mass: Vec<&Solid> = solids
            .iter()
            .filter(|s| s.owner == SolidOwner::Mass(id))
            .collect();
        assert_eq!(of_mass.len(), 2);
        let painted = of_mass.iter().find(|s| s.material == red).unwrap();
        assert_eq!(painted.triangles.len(), 2);
        // Every painted triangle lies on y = 0.
        for t in &painted.triangles {
            assert!(t.iter().all(|p| p[1].abs() < 1e-9), "{t:?}");
        }
        let plain = of_mass.iter().find(|s| s.material == mat).unwrap();
        assert_eq!(plain.triangles.len(), 10);
        assert_eq!(solid_color(&m, painted), [200, 100, 80]);
    }

    /// The door and window solids as ArchiSim built them before families,
    /// kept to check that the system families reproduce them exactly.
    pub(crate) fn legacy_opening_solids(model: &Model, id: OpeningId) -> Vec<Solid> {
        let (Some(o), Some(og)) = (model.opening(id), opening_geometry(model, id)) else {
            return Vec::new();
        };
        let Some(t) = model.opening_type(o.opening_type) else {
            return Vec::new();
        };
        let Some(wall) = model.wall(o.wall) else {
            return Vec::new();
        };
        let level = model.level(wall.level).map_or(0.0, |l| l.elevation);
        let z_sill = level + og.sill;
        let (w, h) = (og.width(), og.height());
        let mid = (og.t0 + og.t1) / 2.0;
        let y_mid = (og.frame_exterior + og.frame_interior) / 2.0;
        let f = t.frame_width;
        let (x0, x1) = (-w / 2.0, w / 2.0);

        // A box in the frame's own coordinates: along the wall from the centre
        // of the opening, across the wall from the middle of the frame, and up
        // from the sill.
        let block = |xa: f64, xb: f64, depth: f64, za: f64, zb: f64| -> Vec<[P3; 3]> {
            let (ya, yb) = (y_mid - depth / 2.0, y_mid + depth / 2.0);
            let rect = [
                og.point(mid + xa, ya),
                og.point(mid + xb, ya),
                og.point(mid + xb, yb),
                og.point(mid + xa, yb),
            ];
            prism(&rect, z_sill + za, z_sill + zb)
        };

        let mut frame = Vec::new();
        let mut panels = Vec::new();
        match t.kind {
            OpeningKind::Void => {}
            OpeningKind::Door(op) => {
                let d = t.frame_depth;
                frame.extend(block(x0, x0 + f, d, 0.0, h));
                frame.extend(block(x1 - f, x1, d, 0.0, h));
                frame.extend(block(x0 + f, x1 - f, d, h - f, h));
                let leaf = 0.04_f64.min(d);
                let leaves: Vec<(f64, f64)> = match op {
                    DoorOperation::DoubleSwing => vec![(x0 + f, 0.0), (0.0, x1 - f)],
                    _ => vec![(x0 + f, x1 - f)],
                };
                for (a, b) in leaves {
                    panels.extend(block(a, b, leaf, 0.0, h - f));
                }
            }
            OpeningKind::Window(op) => {
                let d = t.frame_depth;
                let n = op.panels();
                let clear = (w - 2.0 * f - (n as f64 - 1.0) * f) / n as f64;
                frame.extend(block(x0, x0 + f, d, 0.0, h));
                frame.extend(block(x1 - f, x1, d, 0.0, h));
                frame.extend(block(x0 + f, x1 - f, d, 0.0, f));
                frame.extend(block(x0 + f, x1 - f, d, h - f, h));
                for i in 0..n {
                    let a = x0 + f + i as f64 * (clear + f);
                    if i > 0 {
                        // Mullion between panes.
                        frame.extend(block(a - f, a, d, f, h - f));
                    }
                    panels.extend(block(a, a + clear, 0.024_f64.min(d), f, h - f));
                }
            }
        }
        vec![
            Solid {
                owner: SolidOwner::Opening(id),
                material: t.frame_material,
                triangles: frame,
            },
            Solid {
                owner: SolidOwner::Opening(id),
                material: t.panel_material,
                triangles: panels,
            },
        ]
    }

    use crate::removed_area;
    use archisim_core::{
        CutPattern, DoorOperation, Hand, Layer, LayerFunction, NewOpening, NewOpeningType, NewSlab,
        NewWall, OpeningKind, ReferenceLine, SlabFunction, WallFunction, WallSide, WindowOperation,
    };

    fn p(x: f64, y: f64) -> Point2 {
        Point2::new(x, y)
    }

    #[test]
    fn triangulates_concave_outlines() {
        let l = [
            p(0.0, 0.0),
            p(4.0, 0.0),
            p(4.0, 2.0),
            p(2.0, 2.0),
            p(2.0, 4.0),
            p(0.0, 4.0),
        ];
        let tris = triangulate(&l);
        assert_eq!(tris.len(), 4);
        let area: f64 = tris
            .iter()
            .map(|[a, b, c]| (l[*b] - l[*a]).cross(l[*c] - l[*a]) / 2.0)
            .sum();
        assert!((area - 12.0).abs() < 1e-12);
        assert!(tris
            .iter()
            .all(|[a, b, c]| (l[*b] - l[*a]).cross(l[*c] - l[*a]) > 0.0));
    }

    #[test]
    fn prisms_are_closed_and_outward() {
        let l = [
            p(0.0, 0.0),
            p(4.0, 0.0),
            p(4.0, 2.0),
            p(2.0, 2.0),
            p(2.0, 4.0),
            p(0.0, 4.0),
        ];
        // Drawn clockwise on purpose.
        let mut cw = l.to_vec();
        cw.reverse();
        let s = Solid {
            owner: SolidOwner::Slab(SlabId(1)),
            material: MaterialId(1),
            triangles: prism(&cw, -0.3, 0.0),
        };
        assert!((s.volume() - 3.6).abs() < 1e-12);
        // Every edge is shared by exactly two triangles in opposite
        // directions: the mesh is closed and consistently wound.
        let mut edges = std::collections::HashMap::new();
        let key = |a: P3| [a[0].to_bits(), a[1].to_bits(), a[2].to_bits()];
        for t in &s.triangles {
            for i in 0..3 {
                *edges.entry((key(t[i]), key(t[(i + 1) % 3]))).or_insert(0) += 1;
            }
        }
        for ((a, b), n) in &edges {
            assert_eq!(*n, 1);
            assert_eq!(edges.get(&(*b, *a)), Some(&1), "edge without its twin");
        }
    }

    /// A 6 m wall, 0.3 m thick in two layers, 3 m high on a level at +1,
    /// with a 1.2 × 1.2 window at 0.9 and a door touching the floor.
    fn model() -> (Model, WallId) {
        let mut m = Model::new("T");
        let lvl = m.add_level("P1", 1.0);
        let brick = m.add_material("Ladrillo", "brick", [200, 100, 80], CutPattern::Diagonal);
        let glass = m.add_material("Vidrio", "glass", [190, 220, 240], CutPattern::Solid);
        let layer = |t| Layer {
            material: brick,
            thickness: t,
            function: LayerFunction::Structure,
        };
        let wt = m
            .add_wall_type("M", WallFunction::Exterior, vec![layer(0.1), layer(0.2)])
            .unwrap();
        let w = m
            .add_wall(NewWall {
                wall_type: wt,
                level: lvl,
                start: p(0.0, 0.0),
                end: p(6.0, 0.0),
                reference: ReferenceLine::Center,
                height: 3.0,
                base_offset: 0.0,
                bulge: 0.0,
                top: archisim_core::WallTop::Unconnected,
            })
            .unwrap();
        let mut kind = |kind, width, height| {
            m.add_opening_type(NewOpeningType {
                name: "T".into(),
                kind,
                width,
                height,
                frame_width: 0.06,
                frame_depth: 0.07,
                frame_material: brick,
                panel_material: glass,
            })
            .unwrap()
        };
        let win = kind(
            OpeningKind::Window(WindowOperation::DoubleCasement),
            1.2,
            1.2,
        );
        let door = kind(OpeningKind::Door(DoorOperation::SingleSwing), 0.9, 2.1);
        for (t, along, sill) in [(win, 1.5, 0.9), (door, 4.0, 0.0)] {
            m.add_opening(NewOpening {
                opening_type: t,
                wall: w,
                along,
                sill,
                opens_to: WallSide::Interior,
                hand: Hand::Left,
                setback: None,
            })
            .unwrap();
        }
        let st = m
            .add_slab_type(
                "Losa",
                SlabFunction::Floor,
                vec![
                    Layer {
                        material: brick,
                        thickness: 0.05,
                        function: LayerFunction::Finish,
                    },
                    Layer {
                        material: brick,
                        thickness: 0.25,
                        function: LayerFunction::Structure,
                    },
                ],
            )
            .unwrap();
        m.add_slab(NewSlab {
            slab_type: st,
            level: lvl,
            outline: vec![p(0.0, 0.0), p(6.0, 0.0), p(6.0, 4.0), p(0.0, 4.0)],
            top_offset: 0.0,
        })
        .unwrap();
        (m, w)
    }

    /// Holes go through every layer of a slab, with faces round them.
    #[test]
    fn slab_holes_go_through() {
        let (mut m, _) = model();
        let slab = m.slabs().next().unwrap().id;
        m.add_slab_hole(
            slab,
            vec![p(1.0, 1.0), p(2.0, 1.0), p(2.0, 3.0), p(1.0, 3.0)],
        )
        .unwrap();
        m.add_slab_hole(slab, vec![p(4.0, 0.5), p(5.5, 0.5), p(5.5, 2.0)])
            .unwrap();
        let solids = slab_solids(&m, slab);
        let volume: f64 = solids.iter().map(Solid::volume).sum();
        let area = m.slab(slab).unwrap().area();
        assert!((area - (24.0 - 2.0 - 1.125)).abs() < 1e-9);
        assert!((volume - area * 0.3).abs() < 1e-9, "{volume}");
        // A closed skin: every edge shared by two triangles.
        use std::collections::HashMap;
        let key = |a: P3| {
            (
                (a[0] * 1e6).round() as i64,
                (a[1] * 1e6).round() as i64,
                (a[2] * 1e6).round() as i64,
            )
        };
        let mut edges: HashMap<_, i32> = HashMap::new();
        for s in &solids {
            for t in &s.triangles {
                for i in 0..3 {
                    let (a, b) = (key(t[i]), key(t[(i + 1) % 3]));
                    *edges
                        .entry(if a < b { (a, b) } else { (b, a) })
                        .or_default() += 1;
                }
            }
        }
        // Edges between layers are shared by the two solids' side faces.
        assert!(
            edges.values().all(|n| n % 2 == 0),
            "{:?}",
            edges.values().filter(|n| *n % 2 != 0).count()
        );
    }

    /// E3: a wall with a sloping top and a round hole in its profile
    /// loses that much of its volume.
    #[test]
    fn wall_profile_cuts_the_wall() {
        let (mut m, w) = model();
        let before: f64 = wall_solids(&m, w).iter().map(Solid::volume).sum();
        let wall = m.wall(w).unwrap().clone();
        let (len, h) = (wall.length(), wall.height);
        let t = m.wall_type(wall.wall_type).unwrap().thickness();
        let mut p = archisim_core::WallProfile::sloping(len, h, h - 0.3);
        let hole: Vec<Point2> = (0..24)
            .map(|i| {
                let a = i as f64 * std::f64::consts::TAU / 24.0;
                Point2::new(len * 0.5 + 0.2 * a.cos(), 2.75 + 0.2 * a.sin())
            })
            .collect();
        let hole_area = crate::signed_area(&hole).abs();
        p.holes.push(hole);
        m.set_wall_profile(w, Some(p)).unwrap();
        // (The cut leaves corners on edges, so the skin is not checked
        // edge by edge; its volume is.)
        let solids = wall_solids(&m, w);
        let after: f64 = solids.iter().map(Solid::volume).sum();
        let expected = (len * 0.3 / 2.0 + hole_area) * t;
        assert!(
            ((before - after) - expected).abs() < expected * 0.08,
            "{} vs {expected}",
            before - after
        );
    }

    #[test]
    fn wall_volume_matches_the_ifc_net_volume() {
        let (m, w) = model();
        let solids = wall_solids(&m, w);
        assert_eq!(solids.len(), 2, "one solid per layer");
        let volume: f64 = solids.iter().map(Solid::volume).sum();
        // The same computation as Qto_WallBaseQuantities.NetVolume.
        let g = wall_geometry(&m, w).unwrap();
        let gross = crate::signed_area(&g.outline).abs() * 3.0;
        let cut: f64 = crate::openings_in_wall(&m, w)
            .iter()
            .map(|o| removed_area(&g, o) * o.height())
            .sum();
        assert!(
            (volume - (gross - cut)).abs() < 1e-9,
            "{volume} vs {}",
            gross - cut
        );
        // Heights follow the level (+1) and the wall top.
        let zs: Vec<f64> = solids
            .iter()
            .flat_map(|s| s.triangles.iter().flatten().map(|p| p[2]))
            .collect();
        let (lo, hi) = zs
            .iter()
            .fold((f64::MAX, f64::MIN), |(a, b), z| (a.min(*z), b.max(*z)));
        assert_eq!((lo, hi), (1.0, 4.0));
    }

    #[test]
    fn slabs_hang_from_their_top_face_layer_by_layer() {
        let (m, _) = model();
        let s = m.slabs().next().unwrap().id;
        let solids = slab_solids(&m, s);
        assert_eq!(solids.len(), 2, "one part per layer");
        let volume: f64 = solids.iter().map(Solid::volume).sum();
        assert!((volume - 24.0 * 0.3).abs() < 1e-9);
        assert_closed(&solids);
        let top = solids[0]
            .triangles
            .iter()
            .flatten()
            .map(|p| p[2])
            .fold(f64::MIN, f64::max);
        assert_eq!(top, 1.0);
    }

    /// An inclined slab: the top face follows the slope, the layers keep
    /// their thickness square to it, the edges stay vertical.
    #[test]
    fn inclined_slabs_follow_their_slope() {
        let (mut m, _) = model();
        let s = m.slabs().next().unwrap().clone();
        let lo = s.outline.iter().fold(f64::MAX, |a, p| a.min(p.x));
        let hi = s.outline.iter().fold(f64::MIN, |a, p| a.max(p.x));
        let flat: f64 = slab_solids(&m, s.id).iter().map(Solid::volume).sum();
        m.set_slab_slope(
            s.id,
            Some(archisim_core::SlabSlope {
                from: Point2::new(lo, 0.0),
                to: Point2::new(hi, 0.0),
                slope: 0.1,
            }),
        )
        .unwrap();
        let solids = slab_solids(&m, s.id);
        assert_closed(&solids);
        let volume: f64 = solids.iter().map(Solid::volume).sum();
        assert!((volume - flat * (1.01f64).sqrt()).abs() < 1e-9, "{volume}");
        let top = solids[0]
            .triangles
            .iter()
            .flatten()
            .map(|p| p[2])
            .fold(f64::MIN, f64::max);
        assert!((top - (1.0 + 0.1 * (hi - lo))).abs() < 1e-9, "{top}");
    }

    /// A hip roof: one plate per plane, each closed, the volume that of
    /// the planes' plan areas by their vertical thickness.
    #[test]
    fn roofs_are_plates_per_plane() {
        let (mut m, w) = model();
        let l = m.wall(w).unwrap().level;
        let t = m.slabs().next().unwrap().slab_type;
        let r = m
            .add_roof(archisim_core::NewRoof {
                roof_type: t,
                level: l,
                outline: vec![p(0.0, 0.0), p(6.0, 0.0), p(6.0, 4.0), p(0.0, 4.0)],
                slopes: vec![Some(0.5)],
                base_offset: 0.0,
                overhang: 0.5,
            })
            .unwrap();
        let solids = roof_solids(&m, r);
        let layers = m.slab_type(t).unwrap().layers.len();
        assert_eq!(solids.len(), 4 * layers);
        for plane in solids.chunks(layers) {
            assert_closed(plane);
        }
        let thickness = m.slab_type(t).unwrap().thickness();
        let volume: f64 = solids.iter().map(Solid::volume).sum();
        let expected = 7.0 * 5.0 * thickness * (1.25f64).sqrt();
        assert!((volume - expected).abs() < 1e-9, "{volume} {expected}");
        assert!(model_solids(&m)
            .iter()
            .any(|s| s.owner == SolidOwner::Roof(r)));
    }

    /// A straight stair: one stepped flight, closed, its volume that of
    /// its profile by its width; an L stair adds a landing.
    #[test]
    fn stairs_are_flights_and_landings() {
        let (mut m, w) = model();
        let l = m.wall(w).unwrap().level;
        let mat = m.materials().next().unwrap().id;
        let id = m
            .add_stair(archisim_core::NewStair {
                level: l,
                path: vec![p(0.0, 0.0), p(2.8, 0.0), p(2.8, 2.0)],
                width: 1.0,
                going: 0.28,
                base_offset: 0.0,
                top: archisim_core::WallTop::Unconnected,
                height: 2.8,
                waist: 0.15,
                material: mat,
                usage: archisim_core::StairUse::General,
                design: Default::default(),
            })
            .unwrap();
        let solids = stair_solids(&m, id);
        let layout = m.stair(id).unwrap().layout();
        // Flights, landings and the railings.
        assert_eq!(
            solids.len(),
            layout.flights.len() + layout.landings.len() + 1
        );
        for s in &solids[..solids.len() - 1] {
            assert_closed(std::slice::from_ref(s));
            assert!(s.volume() > 0.0);
        }
        let bars = railing_bars(&m, id);
        // Both sides, round the outer corner of the landing too.
        assert!(bars.iter().filter(|b| b.w == 0.05).count() >= 6);
        for b in &bars {
            let t = Solid {
                owner: SolidOwner::Stair(id),
                material: mat,
                triangles: b.triangles(),
            };
            assert_closed(std::slice::from_ref(&t));
            assert!(t.volume() > 0.0);
        }
        m.edit_stair(id, |s| s.railings = archisim_core::Railings::None)
            .unwrap();
        assert!(railing_bars(&m, id).is_empty());
        assert!(m
            .stair(id)
            .unwrap()
            .issues()
            .contains(&archisim_core::StairIssue::NoHandrail));
        m.edit_stair(id, |s| s.railings = archisim_core::Railings::Both)
            .unwrap();
        let f = layout.flights[0];
        let profile = flight_profile(&f, layout.riser, 0.15, 1.0);
        assert!((solids[0].volume() - archisim_core::signed_area(&profile)).abs() < 1e-9);
        // The first riser stands on the floor; the last tread is a riser
        // under the landing.
        let lo = profile.iter().map(|q| q.y).fold(f64::MAX, f64::min);
        let hi = profile.iter().map(|q| q.y).fold(f64::MIN, f64::max);
        assert!((lo - 1.0).abs() < 1e-9);
        assert!((hi - 1.0 - layout.landings[0].top + layout.riser).abs() < 1e-9);
    }

    /// A terrain is one closed solid; its volume, that of the ground down
    /// to its floor.
    #[test]
    fn terrain_is_closed() {
        let (mut m, w) = model();
        let l = m.wall(w).unwrap().level;
        let mat = m.materials().next().unwrap().id;
        let sp = |x: f64, y: f64, z: f64| archisim_core::SurveyPoint { at: p(x, y), z };
        let id = m
            .add_terrain(archisim_core::NewTerrain {
                level: l,
                points: vec![
                    sp(0.0, 0.0, 0.0),
                    sp(10.0, 0.0, 0.0),
                    sp(10.0, 10.0, 1.0),
                    sp(0.0, 10.0, 1.0),
                    sp(5.0, 5.0, 2.0),
                ],
                material: mat,
                depth: 1.0,
                contour: 0.5,
                lines: Vec::new(),
            })
            .unwrap();
        let solids = terrain_solids(&m, id);
        assert_closed(&solids);
        assert!(solids[0].volume() > 100.0);
        // A lawn painted on it, across the triangles, with a hole: a thin
        // closed layer, as large as the lawn.
        m.edit_terrain(id, |t| {
            t.patches = vec![archisim_core::TerrainPatch {
                material: mat,
                regions: vec![(
                    vec![p(2.0, 2.0), p(8.0, 2.0), p(8.0, 7.0), p(2.0, 7.0)],
                    vec![vec![p(4.0, 3.0), p(5.0, 3.0), p(5.0, 4.0), p(4.0, 4.0)]],
                )],
            }]
        })
        .unwrap();
        let solids = terrain_solids(&m, id);
        assert_eq!(solids.len(), 2);
        assert_closed(&solids);
        // Its volume: area (30 − 1) by its thickness, sloping a little.
        let v = solids[1].volume();
        assert!((v - 29.0 * PATCH_THICKNESS).abs() < 0.05, "{v}");
        // A pool dug in it: the ground loses the pool's room, closed round
        // the hole, and so does the lawn.
        let before = terrain_solids(&m, id)[0].volume();
        let pool = m
            .add_pool(archisim_core::NewPool::new(
                l,
                vec![p(3.0, 3.5), p(7.0, 3.5), p(7.0, 5.5), p(3.0, 5.5)],
                mat,
            ))
            .unwrap();
        let solids = terrain_solids(&m, id);
        let after = solids[0].volume();
        let hole = m.excavations()[0].clone();
        let footprint = archisim_core::signed_area(&hole.outline);
        assert!(after < before - footprint, "{before} {after}");
        assert!(solids[1].volume() < 29.0 * PATCH_THICKNESS - 0.1);
        assert!(!crate::pools::pool_solids(&m, pool).is_empty());
    }

    /// A gable wall under a two-slope roof: built up to the ridge and
    /// trimmed under both planes; its volume is its thickness by the area
    /// under the roof's underside.
    #[test]
    fn tubes_are_hollow_and_closed() {
        for p in [
            archisim_core::Profile::RectHollow {
                width: 0.12,
                depth: 0.08,
                wall: 0.005,
            },
            archisim_core::Profile::CircleHollow {
                diameter: 0.1143,
                wall: 0.005,
            },
        ] {
            let outer = p.outline(ROUND_SEGMENTS);
            let hole = p.hole(ROUND_SEGMENTS).unwrap();
            let s = Solid {
                owner: SolidOwner::Column(ColumnId(1)),
                material: MaterialId(1),
                triangles: hollow_prism(&outer, Some(&hole), 0.0, 2.0),
            };
            let area = crate::signed_area(&outer) - crate::signed_area(&hole);
            assert!((s.volume() - 2.0 * area).abs() < 1e-9, "{p:?}");
        }
    }

    #[test]
    fn loose_railing_bars_are_closed_outward() {
        let mut m = Model::new("t");
        let pb = m.add_level("PB", 0.0);
        let steel = m.add_material(
            "Acero",
            "steel",
            [120, 126, 134],
            archisim_core::CutPattern::Solid,
        );
        let id = m
            .add_railing(archisim_core::NewRailing {
                level: pb,
                path: vec![Point2::new(0.0, 0.0), Point2::new(2.0, 1.0)],
                base_offset: 0.5,
                height: 1.0,
                post_spacing: 1.5,
                material: steel,
                glass: None,
                railing_type: None,
            })
            .unwrap();
        let bars = loose_railing_bars(&m, id);
        for b in &bars {
            let s = Solid {
                owner: SolidOwner::Railing(id),
                material: steel,
                triangles: b.triangles(),
            };
            let len =
                ((b.b[0] - b.a[0]).powi(2) + (b.b[1] - b.a[1]).powi(2) + (b.b[2] - b.a[2]).powi(2))
                    .sqrt();
            assert!((s.volume() - len * b.w * b.d).abs() < 1e-9, "{b:?}");
        }
        let solids = loose_railing_solids(&m, id);
        assert_eq!(solids.len(), 1);
    }

    #[test]
    fn walls_trimmed_under_a_roof() {
        let (mut m, w) = model();
        let l = m.wall(w).unwrap().level;
        let wt = m.wall(w).unwrap().wall_type;
        let st = m.slabs().next().unwrap().slab_type;
        let g = m
            .add_wall(NewWall {
                wall_type: wt,
                level: l,
                start: p(3.0, -2.5),
                end: p(3.0, 2.5),
                reference: ReferenceLine::Center,
                height: 2.0,
                base_offset: 0.0,
                bulge: 0.0,
                top: archisim_core::WallTop::Unconnected,
            })
            .unwrap();
        let r = m
            .add_roof(archisim_core::NewRoof {
                roof_type: st,
                level: l,
                outline: vec![p(-1.0, -3.0), p(7.0, -3.0), p(7.0, 3.0), p(-1.0, 3.0)],
                slopes: vec![Some(0.5), None, Some(0.5), None],
                base_offset: 2.0,
                overhang: 0.0,
            })
            .unwrap();
        let before: f64 = wall_solids(&m, g).iter().map(Solid::volume).sum();
        m.set_wall_roof(g, Some(r)).unwrap();
        let solids = wall_solids(&m, g);
        let after: f64 = solids.iter().map(Solid::volume).sum();
        // Under the soffit: level 1 + base 2 + 0,5·(3 − |y|) − the slab's
        // vertical thickness, over y in −2,5…2,5, from the wall's base (1).
        let t = m.slab_type(st).unwrap().thickness() * (1.25f64).sqrt();
        let thickness = 0.3;
        // ∫ 0,5·|y| dy over −2,5…2,5 = 0,5·2,5² = 3,125.
        let expected = thickness * (5.0 * (2.0 + 1.5 - t) - 3.125);
        assert!(
            (after - expected).abs() < 1e-6,
            "{after} vs {expected} (was {before})"
        );
        let top = solids
            .iter()
            .flat_map(|s| s.triangles.iter().flatten())
            .map(|q| q[2])
            .fold(f64::MIN, f64::max);
        assert!((top - (1.0 + 2.0 + 1.5 - t)).abs() < 1e-9, "{top}");
        // Freed, it is its own height again.
        m.set_wall_roof(g, None).unwrap();
        let again: f64 = wall_solids(&m, g).iter().map(Solid::volume).sum();
        assert!((again - before).abs() < 1e-9);
    }

    /// Every edge of the solids together is shared by exactly two
    /// triangles, in opposite directions: closed, consistently wound, and
    /// without hidden faces between parts.
    fn assert_closed(solids: &[Solid]) {
        // Points closer than 0.1 µm are the same vertex.
        let key = |a: P3| a.map(|v| (v * 1e7).round() as i64);
        let mut edges = std::collections::HashMap::new();
        for t in solids.iter().flat_map(|s| &s.triangles) {
            for i in 0..3 {
                *edges.entry((key(t[i]), key(t[(i + 1) % 3]))).or_insert(0) += 1;
            }
        }
        for ((a, b), n) in &edges {
            assert_eq!(*n, 1, "edge used twice in the same direction");
            assert_eq!(edges.get(&(*b, *a)), Some(&1), "edge without its twin");
        }
    }

    /// A façade (render, brick, insulation, inner leaf, plaster) and two
    /// partitions (plaster, inner leaf, plaster) butting against its inner
    /// face: one square, one at 45°, both lower than the façade.
    fn tee_model() -> (Model, WallId, Vec<WallId>) {
        let mut m = Model::new("T");
        let lvl = m.add_level("P1", 0.0);
        let mat = m.add_material("M", "brick", [200, 100, 80], CutPattern::Solid);
        let l = |t, function| Layer {
            material: mat,
            thickness: t,
            function,
        };
        use LayerFunction as F;
        let facade = m
            .add_wall_type(
                "Fachada",
                WallFunction::Exterior,
                vec![
                    l(0.015, F::Finish),
                    l(0.115, F::Structure),
                    l(0.05, F::Insulation),
                    l(0.07, F::Substrate),
                    l(0.015, F::Finish),
                ],
            )
            .unwrap();
        let partition = m
            .add_wall_type(
                "Tabique",
                WallFunction::Interior,
                vec![
                    l(0.015, F::Finish),
                    l(0.07, F::Substrate),
                    l(0.015, F::Finish),
                ],
            )
            .unwrap();
        // The façade along y = 0, exterior face on it, interior towards +y.
        let host = m
            .add_wall(NewWall {
                wall_type: facade,
                level: lvl,
                start: p(8.0, 0.0),
                end: p(0.0, 0.0),
                reference: ReferenceLine::ExteriorFace,
                height: 3.0,
                base_offset: 0.0,
                bulge: 0.0,
                top: archisim_core::WallTop::Unconnected,
            })
            .unwrap();
        let mut parts = Vec::new();
        for (start, end) in [(p(2.0, 0.265), p(2.0, 3.0)), (p(6.0, 0.265), p(4.0, 2.265))] {
            parts.push(
                m.add_wall(NewWall {
                    wall_type: partition,
                    level: lvl,
                    start,
                    end,
                    reference: ReferenceLine::Center,
                    height: 2.5,
                    base_offset: 0.0,
                    bulge: 0.0,
                    top: archisim_core::WallTop::Unconnected,
                })
                .unwrap(),
            );
        }
        (m, host, parts)
    }

    #[test]
    fn t_junctions_are_closed_and_their_volumes_add_up() {
        let (m, host, parts) = tee_model();
        // The partitions' inner leaf goes through the façade's plaster.
        let g = wall_geometry(&m, host).unwrap();
        assert_eq!(g.notches.len(), 2);
        // Host: its outline changes at the partitions' top (2.5 m).
        let banded = |g: &crate::WallGeometry, h: f64| -> f64 {
            let mut zs = vec![0.0, h];
            zs.extend(g.notch_heights().into_iter().filter(|z| *z > 0.0 && *z < h));
            zs.sort_by(f64::total_cmp);
            zs.windows(2)
                .map(|w| {
                    crate::signed_area(&g.outline_at(Some((w[0] + w[1]) / 2.0))) * (w[1] - w[0])
                })
                .sum()
        };
        for (id, h) in [(host, 3.0), (parts[0], 2.5), (parts[1], 2.5)] {
            let solids = wall_solids(&m, id);
            assert_closed(&solids);
            let vol: f64 = solids.iter().map(Solid::volume).sum();
            let g = wall_geometry(&m, id).unwrap();
            let want = banded(&g, h);
            assert!((vol - want).abs() < 1e-9, "wall {id}: {vol} != {want}");
        }
        // Less than the plain façade by the two bits of plaster taken.
        let plain = {
            let mut m2 = m.clone();
            for w in &parts {
                m2.remove_wall(*w).unwrap();
            }
            wall_solids(&m2, host)
                .iter()
                .map(Solid::volume)
                .sum::<f64>()
        };
        let host_vol: f64 = wall_solids(&m, host).iter().map(Solid::volume).sum();
        let square = 0.07 * 0.015 * 2.5;
        let slanted = 0.07 * std::f64::consts::SQRT_2 * 0.015 * 2.5;
        assert!(
            (plain - host_vol - square - slanted).abs() < 1e-9,
            "{}",
            plain - host_vol
        );
    }

    #[test]
    fn a_wall_with_openings_is_one_closed_skin() {
        let (m, w) = model();
        assert_closed(&wall_solids(&m, w));
    }

    #[test]
    fn mitred_corners_leave_the_shared_face_out() {
        let (mut m, w) = model();
        let wt = m.wall(w).unwrap().wall_type;
        let lvl = m.wall(w).unwrap().level;
        let other = m
            .add_wall(NewWall {
                wall_type: wt,
                level: lvl,
                start: p(6.0, 0.0),
                end: p(6.0, 4.0),
                reference: ReferenceLine::Center,
                height: 3.0,
                base_offset: 0.0,
                bulge: 0.0,
                top: archisim_core::WallTop::Unconnected,
            })
            .unwrap();
        // Together the two walls are closed: each leaves its mitre face to
        // the other, and the corner edge has the same vertices on both sides.
        let mut both = wall_solids(&m, w);
        both.extend(wall_solids(&m, other));
        assert_closed(&both);
    }

    #[test]
    fn doors_and_windows_fill_their_openings() {
        let (m, w) = model();
        let ids: Vec<_> = m.openings_in_wall(w).map(|o| o.id).collect();
        for id in ids {
            let og = opening_geometry(&m, id).unwrap();
            let solids = opening_solids(&m, id);
            assert_eq!(solids.len(), 2, "frame and leaves/glass");
            assert!(solids.iter().all(|s| s.volume() > 0.0));
            // Everything stays inside the rough opening.
            for p3 in solids.iter().flat_map(|s| s.triangles.iter().flatten()) {
                let along = (Point2::new(p3[0], p3[1]) - og.origin).dot(og.u);
                assert!(along >= og.t0 - 1e-9 && along <= og.t1 + 1e-9);
                assert!(p3[2] >= 1.0 + og.sill - 1e-9 && p3[2] <= 1.0 + og.head + 1e-9);
            }
        }
        // The whole model: 2 wall layers, 2 slab layers, 2 × 2 opening parts.
        assert_eq!(model_solids(&m).len(), 8);
        assert_eq!(level_solids(&m, m.levels()[0].id).len(), 8);
    }

    /// Doors draw their opening lines in the model's sections and
    /// elevations, dashed; windows only when their type asks for them.
    #[test]
    fn windows_draw_opening_lines_only_when_asked() {
        let (mut m, w) = model();
        let ops: Vec<_> = m
            .openings_in_wall(w)
            .map(|o| (o.id, o.opening_type))
            .collect();
        let mut window = None;
        for (id, t) in &ops {
            let ty = m.opening_type(*t).unwrap();
            let (lines, _) = opening_elevation_lines(&m, *id);
            if ty.is_door() {
                assert!(!lines.is_empty());
                assert!(lines.iter().all(|l| l.dashed), "doors: dashed");
            } else {
                assert!(lines.is_empty(), "windows: none by default");
                window = Some((*id, *t));
            }
        }
        let (id, t) = window.unwrap();
        let ty = m.opening_type(t).unwrap();
        let mut values = ty.values.clone();
        values.insert(
            archisim_core::OPENING_LINES_IN_VIEWS.into(),
            archisim_core::Value::Bool(true),
        );
        let family = ty.family;
        m.edit_family_type(t, family, values).unwrap();
        let (lines, _) = opening_elevation_lines(&m, id);
        assert!(!lines.is_empty() && lines.iter().all(|l| l.dashed));
    }

    /// Revit's «Envolver en extremos»: the exterior finish turns round the
    /// free ends and covers the other layers' ends, a strip as wide as
    /// itself; the wall keeps its volume. «Envolver en huecos»: it turns
    /// into the reveals of a window up to the frame.
    #[test]
    fn finishes_wrap_at_ends_and_inserts() {
        let mut m = Model::new("Envolver");
        let lvl = m.add_level("P1", 0.0);
        let render = m.add_material("Mortero", "plaster", [230, 230, 220], CutPattern::Solid);
        let brick = m.add_material("Ladrillo", "brick", [200, 100, 80], CutPattern::Diagonal);
        let plaster = m.add_material("Yeso", "plaster", [245, 245, 245], CutPattern::Solid);
        let layer = |material, thickness, function| Layer {
            material,
            thickness,
            function,
        };
        let wt = m
            .add_wall_type(
                "Fachada",
                WallFunction::Exterior,
                vec![
                    layer(render, 0.02, LayerFunction::Finish),
                    layer(brick, 0.2, LayerFunction::Structure),
                    layer(plaster, 0.015, LayerFunction::Finish),
                ],
            )
            .unwrap();
        // The reveals of a façade wrap by default, as they were drawn.
        assert_eq!(
            m.wall_type(wt).unwrap().wrap_inserts,
            archisim_core::Wrap::Exterior
        );
        m.set_wall_type_wrap(wt, archisim_core::Wrap::Exterior, archisim_core::Wrap::None)
            .unwrap();
        let w = m
            .add_wall(NewWall {
                wall_type: wt,
                level: lvl,
                start: Point2::new(0.0, 0.0),
                end: Point2::new(4.0, 0.0),
                reference: ReferenceLine::Center,
                height: 3.0,
                base_offset: 0.0,
                bulge: 0.0,
                top: archisim_core::WallTop::Unconnected,
            })
            .unwrap();
        // The area of the faces of one material lying on the plane x = at.
        let area_at = |m: &Model, mat, at: f64| -> f64 {
            wall_solids(m, w)
                .iter()
                .filter(|s| s.material == mat)
                .flat_map(|s| s.triangles.iter())
                .filter(|t| t.iter().all(|p| (p[0] - at).abs() < 1e-9))
                .map(|[p, q, r]| {
                    let c = cross(
                        [q[0] - p[0], q[1] - p[1], q[2] - p[2]],
                        [r[0] - p[0], r[1] - p[1], r[2] - p[2]],
                    );
                    dot(c, c).sqrt() / 2.0
                })
                .sum()
        };
        let total = |m: &Model| -> f64 { wall_solids(m, w).iter().map(|s| s.volume()).sum() };
        // The whole end face (0.235 × 3 m) is render; the brick shows none.
        assert!((area_at(&m, render, 0.0) - 0.235 * 3.0).abs() < 1e-9);
        assert!(area_at(&m, brick, 0.0) < 1e-12);
        assert!((area_at(&m, render, 4.0) - 0.235 * 3.0).abs() < 1e-9);
        assert!((total(&m) - 0.235 * 4.0 * 3.0).abs() < 1e-9);
        // Each layer's cut shows its own colour: faces between the render
        // strip and the brick it covers.
        let idx = crate::WallIndex::new(&m);
        assert!(!wall_inner_faces(&idx, w).is_empty());
        // Without wrapping, the brick shows at the end.
        m.set_wall_type_wrap(wt, archisim_core::Wrap::None, archisim_core::Wrap::None)
            .unwrap();
        assert!((area_at(&m, brick, 0.0) - 0.2 * 3.0).abs() < 1e-9);
        // Into the reveals of a window: the jamb face outside the frame is
        // render, inside it the brick and plaster as before.
        m.set_wall_type_wrap(wt, archisim_core::Wrap::None, archisim_core::Wrap::Exterior)
            .unwrap();
        let glass = m.add_material("Vidrio", "glass", [190, 220, 240], CutPattern::Solid);
        let t = m
            .add_opening_type(archisim_core::NewOpeningType {
                name: "V".into(),
                kind: archisim_core::OpeningKind::Window(archisim_core::WindowOperation::Fixed),
                width: 1.0,
                height: 1.0,
                frame_width: 0.05,
                frame_depth: 0.07,
                frame_material: brick,
                panel_material: glass,
            })
            .unwrap();
        m.add_opening(archisim_core::NewOpening {
            opening_type: t,
            wall: w,
            along: 2.0,
            sill: 1.0,
            opens_to: archisim_core::WallSide::Interior,
            hand: archisim_core::Hand::Left,
            // The frame 15 cm in, past the middle of the brick.
            setback: Some(0.15),
        })
        .unwrap();
        let jamb = 1.5;
        let render_jamb = area_at(&m, render, jamb);
        let brick_jamb = area_at(&m, brick, jamb);
        assert!(render_jamb > 0.02 * 1.0 + 1e-6, "{render_jamb}");
        assert!(brick_jamb < 0.2 * 1.0 - 1e-6, "{brick_jamb}");
        assert!((total(&m) - (0.235 * 4.0 * 3.0 - 0.235 * 1.0 * 1.0)).abs() < 1e-6);
    }

    /// The system families build the same doors and windows ArchiSim built
    /// before families: every operation, several sizes, both sides and both
    /// hands, in a wall at an odd angle. The triangles may differ in the
    /// last digit (around 1e-15 m): the old code measured the opening back
    /// from its jambs and head instead of using the type's exact size, and
    /// a family opening to the exterior is mirrored.
    #[test]
    fn system_families_reproduce_the_previous_doors_and_windows() {
        let mut m = Model::new("Regresión");
        let lvl = m.add_level("P1", 0.35);
        let brick = m.add_material("Ladrillo", "brick", [200, 100, 80], CutPattern::Diagonal);
        let wood = m.add_material("Madera", "wood", [190, 150, 100], CutPattern::Solid);
        let glass = m.add_material("Vidrio", "glass", [190, 220, 240], CutPattern::Solid);
        let wt = m
            .add_wall_type(
                "M",
                WallFunction::Exterior,
                vec![Layer {
                    material: brick,
                    thickness: 0.31,
                    function: LayerFunction::Structure,
                }],
            )
            .unwrap();
        let w = m
            .add_wall(NewWall {
                wall_type: wt,
                level: lvl,
                start: p(1.3, 2.7),
                end: p(1.3 + 300.0 * 0.6, 2.7 + 300.0 * 0.8 + 0.123),
                reference: ReferenceLine::ExteriorFace,
                height: 3.1,
                base_offset: 0.0,
                bulge: 0.0,
                top: archisim_core::WallTop::Unconnected,
            })
            .unwrap();
        let kinds = [
            OpeningKind::Door(DoorOperation::SingleSwing),
            OpeningKind::Door(DoorOperation::DoubleSwing),
            OpeningKind::Door(DoorOperation::Sliding),
            OpeningKind::Window(WindowOperation::Fixed),
            OpeningKind::Window(WindowOperation::SingleCasement),
            OpeningKind::Window(WindowOperation::DoubleCasement),
            OpeningKind::Window(WindowOperation::Sliding),
        ];
        let sizes = [(0.83, 2.07, 0.053, 0.071), (1.457, 1.913, 0.067, 0.031)];
        let mut along = 1.0;
        let mut count = 0;
        for kind in kinds {
            for (size, (width, height, fw, fd)) in sizes.into_iter().enumerate() {
                // Same material for frame and leaf in half of them.
                let panel = if size == 0 { wood } else { glass };
                let t = m
                    .add_opening_type(NewOpeningType {
                        name: format!("{kind:?} {size}"),
                        kind,
                        width,
                        height,
                        frame_width: fw,
                        frame_depth: fd,
                        frame_material: wood,
                        panel_material: panel,
                    })
                    .unwrap();
                for opens_to in [WallSide::Interior, WallSide::Exterior] {
                    for hand in [Hand::Left, Hand::Right] {
                        for setback in [None, Some(0.05)] {
                            let o = m
                                .add_opening(NewOpening {
                                    opening_type: t,
                                    wall: w,
                                    along,
                                    sill: 0.21,
                                    opens_to,
                                    hand,
                                    setback,
                                })
                                .unwrap();
                            along += 2.0;
                            count += 1;
                            compare(&m, o);
                        }
                    }
                }
            }
        }
        assert_eq!(count, 7 * 2 * 8);
    }

    fn compare(m: &Model, id: OpeningId) {
        let old = legacy_opening_solids(m, id);
        let new = opening_solids(m, id);
        let materials: Vec<MaterialId> = new.iter().map(|s| s.material).collect();
        let mut expected_materials: Vec<MaterialId> = Vec::new();
        for s in &old {
            if !expected_materials.contains(&s.material) {
                expected_materials.push(s.material);
            }
        }
        assert_eq!(materials, expected_materials, "same materials, frame first");
        for s in &new {
            let want: Vec<[P3; 3]> = old
                .iter()
                .filter(|o| o.material == s.material)
                .flat_map(|o| o.triangles.iter().copied())
                .collect();
            let (a, b) = (sorted_tris(&s.triangles), sorted_tris(&want));
            assert_eq!(a.len(), b.len());
            for (x, y) in a.iter().zip(&b) {
                for k in 0..9 {
                    assert!((x[k] - y[k]).abs() < 1e-12, "opening {id}");
                }
            }
            assert!((s.volume() - want_volume(&want)).abs() < 1e-9);
        }
    }

    fn want_volume(tris: &[[P3; 3]]) -> f64 {
        Solid {
            owner: SolidOwner::Opening(OpeningId(0)),
            material: MaterialId(0),
            triangles: tris.to_vec(),
        }
        .volume()
    }

    fn sorted_tris(tris: &[[P3; 3]]) -> Vec<[f64; 9]> {
        let mut v: Vec<[f64; 9]> = tris
            .iter()
            .map(|t| {
                let mut a = [0.0; 9];
                for (i, p3) in t.iter().enumerate() {
                    a[3 * i..3 * i + 3].copy_from_slice(p3);
                }
                a
            })
            .collect();
        // Sort on rounded keys so that last-digit differences keep the order.
        let key = |a: &[f64; 9]| a.map(|x| (x * 1e6).round() as i64);
        v.sort_by_key(key);
        v
    }
}

#[cfg(test)]
mod member_tests {
    use super::*;
    use archisim_core::{CutPattern, MemberUsage, NewBeam, NewColumn, Profile, WallTop};

    #[test]
    fn column_and_beam_volumes() {
        let mut m = Model::new("t");
        let l0 = m.add_level("PB", 0.0);
        let l1 = m.add_level("P1", 3.0);
        let mat = m.add_material("HA", "concrete", [200, 200, 200], CutPattern::Cross);
        let profiles = [
            Profile::Rectangle {
                width: 0.3,
                depth: 0.4,
            },
            Profile::IShape {
                width: 0.15,
                depth: 0.3,
                web: 0.0071,
                flange: 0.0107,
            },
        ];
        for (k, profile) in profiles.into_iter().enumerate() {
            let ct = m
                .add_member_type(format!("c{k}"), MemberUsage::Column, mat, profile)
                .unwrap();
            let bt = m
                .add_member_type(format!("b{k}"), MemberUsage::Beam, mat, profile)
                .unwrap();
            let c = m
                .add_column(NewColumn {
                    member_type: ct,
                    level: l0,
                    position: Point2::new(1.0, 1.0),
                    rotation: 0.7,
                    base_offset: 0.1,
                    top: WallTop::Level {
                        level: l1,
                        offset: -0.3,
                    },
                    height: 0.0,
                })
                .unwrap();
            let b = m
                .add_beam(NewBeam {
                    member_type: bt,
                    level: l1,
                    start: Point2::new(0.0, 0.0),
                    end: Point2::new(3.0, 4.0),
                    top_offset: 0.0,
                })
                .unwrap();
            let cs = column_solids(&m, c);
            assert!((cs[0].volume() - profile.area() * 2.6).abs() < 1e-9);
            let bs = beam_solids(&m, b);
            assert!((bs[0].volume() - profile.area() * 5.0).abs() < 1e-9);
            // The beam hangs from the level: its top is at 3.0.
            let top = bs[0]
                .triangles
                .iter()
                .flatten()
                .map(|p| p[2])
                .fold(f64::MIN, f64::max);
            assert!((top - 3.0).abs() < 1e-9);
        }
    }
}

#[cfg(test)]
mod junction_tests {
    use super::*;
    use archisim_core::{
        CutPattern, Layer, LayerFunction, NewSlab, NewWall, ReferenceLine, SlabFunction,
        WallFunction, WallTop,
    };

    fn p(x: f64, y: f64) -> Point2 {
        Point2::new(x, y)
    }

    /// Every edge shared by exactly two triangles (a closed skin).
    fn closed(solids: &[Solid]) -> bool {
        use std::collections::HashMap;
        let key = |a: P3| {
            (
                (a[0] * 1e5).round() as i64,
                (a[1] * 1e5).round() as i64,
                (a[2] * 1e5).round() as i64,
            )
        };
        let mut edges: HashMap<_, i32> = HashMap::new();
        for s in solids {
            for t in &s.triangles {
                for i in 0..3 {
                    let (a, b) = (key(t[i]), key(t[(i + 1) % 3]));
                    if a != b {
                        *edges.entry((a, b)).or_default() += 1;
                        *edges.entry((b, a)).or_default() -= 1;
                    }
                }
            }
        }
        edges.values().all(|v| *v == 0)
    }

    /// A partition up to the top of the slab of the floor above: the slab's
    /// structure cuts it (it stops under the slab and starts again over
    /// it), its brick goes through the screed, the floor and the ceiling
    /// plaster (they stop at its faces), its own plaster stops under the
    /// ceiling plaster. Nothing is counted twice.
    #[test]
    fn walls_and_slabs_share_their_room_by_priority() {
        let mut m = Model::new("t");
        let pb = m.add_level("PB", 0.0);
        let p1 = m.add_level("P1", 3.0);
        let mat = |m: &mut Model, n: &str| m.add_material(n, "x", [1, 2, 3], CutPattern::Solid);
        let (plaster, brick, floor, screed, concrete) = (
            mat(&mut m, "Yeso"),
            mat(&mut m, "Ladrillo"),
            mat(&mut m, "Solado"),
            mat(&mut m, "Mortero"),
            mat(&mut m, "Hormigón"),
        );
        let layer = |material, thickness, function| Layer {
            material,
            thickness,
            function,
        };
        let wt = m
            .add_wall_type(
                "Tabique",
                WallFunction::Interior,
                vec![
                    layer(plaster, 0.015, LayerFunction::Finish),
                    layer(brick, 0.115, LayerFunction::Structure),
                    layer(plaster, 0.015, LayerFunction::Finish),
                ],
            )
            .unwrap();
        let st = m
            .add_slab_type(
                "Forjado",
                SlabFunction::Floor,
                vec![
                    layer(floor, 0.02, LayerFunction::Finish),
                    layer(screed, 0.05, LayerFunction::Substrate),
                    layer(concrete, 0.25, LayerFunction::Structure),
                    layer(plaster, 0.015, LayerFunction::Finish),
                ],
            )
            .unwrap();
        let w = m
            .add_wall(NewWall {
                wall_type: wt,
                level: pb,
                start: p(0.0, 5.0),
                end: p(10.0, 5.0),
                reference: ReferenceLine::Center,
                height: 3.0,
                base_offset: 0.0,
                bulge: 0.0,
                top: WallTop::Unconnected,
            })
            .unwrap();
        let s = m
            .add_slab(NewSlab {
                slab_type: st,
                level: p1,
                outline: vec![p(0.0, 0.0), p(10.0, 0.0), p(10.0, 10.0), p(0.0, 10.0)],
                top_offset: 0.0,
            })
            .unwrap();
        // One skin per element: volumes of the whole (a layer alone is
        // not closed).
        let vol = |list: &[Solid]| -> f64 { list.iter().map(Solid::volume).sum() };
        let ws = wall_solids(&m, w);
        // Plaster up to the ceiling plaster (2,665), brick through it up
        // to the concrete (2,68). The wall does not go on above the slab,
        // so its brick does not come out again through the screed.
        let plaster_v = 2.0 * 0.015 * 10.0 * 2.665;
        assert!(
            (vol(&ws) - plaster_v - 0.115 * 10.0 * 2.68).abs() < 1e-9,
            "{}",
            vol(&ws)
        );
        assert!(closed(&ws));
        // The slab loses the brick's room in its ceiling plaster only.
        let ss = slab_solids(&m, s);
        let left = 100.0 - 10.0 * 0.115;
        assert!(
            (vol(&ss) - 100.0 * 0.32 - left * 0.015).abs() < 1e-6,
            "{}",
            vol(&ss)
        );
        assert!(closed(&ss), "the slab is not a closed skin");
        // Two storeys high: from above, the brick goes down through the
        // floor and the screed to the concrete; the plaster stops on the
        // floor.
        m.set_wall_height(w, 6.0, 0.0).unwrap();
        let ws = wall_solids(&m, w);
        let expected = 0.03 * 10.0 * (2.665 + 3.0) + 0.115 * 10.0 * (2.68 + 3.07);
        assert!((vol(&ws) - expected).abs() < 1e-9, "{}", vol(&ws));
        assert!(closed(&ws));
        let ss = slab_solids(&m, s);
        assert!(
            (vol(&ss) - 100.0 * 0.25 - left * 0.085).abs() < 1e-6,
            "{}",
            vol(&ss)
        );
        assert!(closed(&ss));
        // Its floor layer, alone, has the brick's strip taken out.
        let floor_tris = &ss.iter().find(|x| x.material == floor).unwrap().triangles;
        assert!(floor_tris.iter().all(|t| t
            .iter()
            .all(|q| q[1] <= 5.0 - 0.0575 + 1e-6 || q[1] >= 5.0 + 0.0575 - 1e-6)));
        let _ = (screed, concrete);
        // Up to 2,95, inside the screed: it does not cover the screed, so
        // the screed keeps its room; the brick stops at the concrete.
        m.set_wall_height(w, 2.95, 0.0).unwrap();
        let ws = wall_solids(&m, w);
        assert!(
            (vol(&ws) - plaster_v - 0.115 * 10.0 * 2.68).abs() < 1e-9,
            "{}",
            vol(&ws)
        );
        let ss = slab_solids(&m, s);
        assert!(
            (vol(&ss) - 100.0 * 0.32 - left * 0.015).abs() < 1e-6,
            "{}",
            vol(&ss)
        );
        // A slab edge across the wall: only the part under the slab is cut.
        m.set_slab_outline(
            s,
            vec![p(4.0, 0.0), p(10.0, 0.0), p(10.0, 10.0), p(4.0, 10.0)],
        )
        .unwrap();
        let ws = wall_solids(&m, w);
        let full = 0.145 * 4.0 * 2.95;
        let under = 6.0 * (0.03 * 2.665 + 0.115 * 2.68);
        assert!((vol(&ws) - full - under).abs() < 1e-9, "{}", vol(&ws));
        assert!(closed(&ws));
    }
}

#[cfg(test)]
mod corner_tests {
    use super::*;
    use archisim_core::{Hand, NewOpening, NewWall, Point2, ReferenceLine, Value, WallSide};

    /// Two walls in an L, a window at the corner of each: with «esquina»
    /// set they reach the corner and the wall is opened through it, so the
    /// two windows meet with no pier between them.
    #[test]
    fn corner_windows_open_the_corner() {
        use archisim_core::{
            CutPattern, Layer, LayerFunction, NewOpeningType, OpeningKind, WallFunction,
            WindowOperation,
        };
        let mut m = Model::new("T");
        let level = m.add_level("P1", 0.0);
        let brick = m.add_material("Ladrillo", "brick", [200, 100, 80], CutPattern::Diagonal);
        let glass = m.add_material("Vidrio", "glass", [190, 220, 240], CutPattern::Solid);
        let wt = m
            .add_wall_type(
                "M",
                WallFunction::Exterior,
                vec![Layer {
                    material: brick,
                    thickness: 0.3,
                    function: LayerFunction::Structure,
                }],
            )
            .unwrap();
        let wall = |m: &mut Model, a: Point2, b: Point2| {
            m.add_wall(NewWall {
                wall_type: wt,
                level,
                start: a,
                end: b,
                reference: ReferenceLine::Center,
                height: 3.0,
                base_offset: 0.0,
                bulge: 0.0,
                top: archisim_core::WallTop::Unconnected,
            })
            .unwrap()
        };
        let a = wall(&mut m, Point2::new(0.0, 0.0), Point2::new(4.0, 0.0));
        let b = wall(&mut m, Point2::new(4.0, 0.0), Point2::new(4.0, 4.0));
        let wid = m
            .add_opening_type(NewOpeningType {
                name: "V".into(),
                kind: OpeningKind::Window(WindowOperation::DoubleCasement),
                width: 1.2,
                height: 1.2,
                frame_width: 0.06,
                frame_depth: 0.07,
                frame_material: brick,
                panel_material: glass,
            })
            .unwrap();
        let window = m.opening_type(wid).unwrap().clone();
        let place = |m: &mut Model, w, along| {
            m.add_opening(NewOpening {
                opening_type: window.id,
                wall: w,
                along,
                sill: 0.9,
                opens_to: WallSide::Interior,
                hand: Hand::Left,
                setback: None,
            })
            .unwrap()
        };
        let before: f64 = wall_solids(&m, a).iter().map(Solid::volume).sum::<f64>()
            + wall_solids(&m, b).iter().map(Solid::volume).sum::<f64>();
        let oa = place(&mut m, a, 2.5);
        let ob = place(&mut m, b, 1.5);
        let plain: f64 = wall_solids(&m, a).iter().map(Solid::volume).sum::<f64>()
            + wall_solids(&m, b).iter().map(Solid::volume).sum::<f64>();
        // The window of the first wall at its end; the second at its start.
        // Seen from inside (where they open), the end of «a» is on the
        // right and the start of «b» on the left.
        let mut va = m.opening(oa).unwrap().values.clone();
        va.insert("corner".into(), Value::Text("derecha".into()));
        m.set_opening_values(oa, va).unwrap();
        let mut vb = m.opening(ob).unwrap().values.clone();
        vb.insert("corner".into(), Value::Text("izquierda".into()));
        m.set_opening_values(ob, vb).unwrap();
        let ga = crate::opening_geometry(&m, oa).unwrap();
        let gb = crate::opening_geometry(&m, ob).unwrap();
        assert_eq!((ga.corner, gb.corner), (Some(true), Some(false)));
        // Snapped: the jamb at the meeting of the walls' middles.
        assert!((ga.t1 - 4.0).abs() < 1e-9, "{}", ga.t1);
        assert!(gb.t0.abs() < 1e-9, "{}", gb.t0);
        // Moving it along the wall keeps it at the corner.
        m.move_opening(oa, 1.0, 0.9).unwrap();
        assert!((crate::opening_geometry(&m, oa).unwrap().t1 - 4.0).abs() < 1e-9);
        // The corner is open: more wall removed than two plain windows,
        // by the corner's own square (thickness² × window height).
        let open: f64 = wall_solids(&m, a).iter().map(Solid::volume).sum::<f64>()
            + wall_solids(&m, b).iter().map(Solid::volume).sum::<f64>();
        // Each wall is opened from the window's other jamb to its end,
        // its share of the corner included: between them, the whole
        // corner is open, with no pier.
        let t = m.wall_type(wt).unwrap().thickness();
        let h = window.height;
        let removed = before - open;
        assert!(
            (removed - 2.0 * 1.2 * t * h).abs() < 1e-6,
            "removed {removed}"
        );
        // No jamb on the corner side: no frame piece reaches its edge.
        let w = window.width;
        let parts = m.evaluate_opening(oa).unwrap().parts;
        assert!(parts.iter().all(|p| p.max[0] < w / 2.0 - 1e-9
            || p.min[2] > 0.01
            || p.max[2] - p.min[2] < h - 0.01));
        let _ = plain;
    }
}

#[cfg(test)]
mod sweep_tests {
    use super::*;

    #[test]
    fn swept_pieces_are_closed_and_outward() {
        // A 1 × 1 square section along 4 m: 4 m³.
        let sq = [(-0.5, -0.5), (0.5, -0.5), (0.5, 0.5), (-0.5, 0.5)];
        let s = Solid {
            owner: SolidOwner::Run(archisim_core::RunId(1)),
            material: archisim_core::MaterialId(0),
            triangles: sweep_segment(&sq, [0.0, 0.0, 1.0], [0.0, 4.0, 1.0]),
        };
        assert!((s.volume() - 4.0).abs() < 1e-9, "{}", s.volume());
    }
}

#[cfg(test)]
mod worktop_tests {
    use super::*;
    use archisim_core::{CutPattern, NewObject, ParamScope, Value};

    /// Four base modules along a wall, the second a sink, and one turned
    /// a quarter up the next wall (an L): one worktop over all of them,
    /// whose top face is their tops less the sink's opening, and modules
    /// without a top of their own.
    #[test]
    fn a_run_has_one_worktop() {
        let mut m = Model::new("cocina");
        let l = m.add_level("PB", 0.0);
        let stone = m.add_material("Encimera", "stone", [230, 230, 225], CutPattern::Solid);
        let wood = m.add_material("Pino", "wood", [200, 170, 120], CutPattern::Solid);
        let fam = m.system_family("Módulo bajo de cocina").unwrap();
        let mut values = m.family_defaults(fam, ParamScope::Type).unwrap();
        values.insert("top".into(), Value::Material(stone.0));
        values.insert("mat".into(), Value::Material(wood.0));
        let drawers = m.add_object_type("Cajonera", fam, values.clone()).unwrap();
        values.insert("unit".into(), Value::Text("Fregadero".into()));
        values.insert("width".into(), Value::Number(0.8));
        let sink = m.add_object_type("Fregadero", fam, values).unwrap();
        let mut place = |t, x: f64, y: f64, rotation: f64| {
            m.add_object(NewObject {
                object_type: t,
                level: l,
                position: Point2::new(x, y),
                rotation,
                base_offset: 0.0,
            })
            .unwrap()
        };
        let a = place(drawers, 0.3, 0.0, 0.0);
        let b = place(sink, 1.0, 0.0, 0.0);
        let c = place(drawers, 1.7, 0.0, 0.0);
        // The corner, then up the right-hand wall (x = 2.6), facing left.
        let e = place(drawers, 2.3, 0.0, 0.0);
        let d = place(drawers, 2.6, 0.9, std::f64::consts::FRAC_PI_2);
        // An island: three each way, backs together.
        let mut island = Vec::new();
        for x in [10.3, 10.9, 11.5] {
            island.push(place(drawers, x, 5.0, std::f64::consts::PI));
            island.push(place(drawers, x, 5.0, 0.0));
        }
        let run = m.worktop_run_of(a).expect("a run");
        assert_eq!(run.members, vec![a, b, c, e, d]);
        // The top face of the worktop, at 0.90.
        let top_area = |solids: &[Solid]| -> f64 {
            solids
                .iter()
                .filter(|s| s.material == stone)
                .flat_map(|s| s.triangles.iter())
                .filter(|t| t.iter().all(|p| (p[2] - 0.9).abs() < 1e-6))
                .map(|t| {
                    let u = [t[1][0] - t[0][0], t[1][1] - t[0][1]];
                    let v = [t[2][0] - t[0][0], t[2][1] - t[0][1]];
                    (u[0] * v[1] - u[1] * v[0]).abs() / 2.0
                })
                .sum()
        };
        let first = object_solids(&m, a);
        let want = run.area;
        let got = top_area(&first);
        assert!((got - want).abs() < 1e-6, "{got} m² against {want}");
        // The others carry no worktop.
        // (A sink keeps the rim of its basin level with it: a few cm², or
        // the flange of the nordic inset sink, some 450 cm².)
        for o in [b, c, e, d] {
            let left = top_area(&object_solids(&m, o));
            let rim = if o == b { 0.05 } else { 0.01 };
            assert!(left < rim, "{o:?}: {left}");
        }
        let run = m.worktop_run_of(island[0]).expect("an island");
        assert_eq!(run.members.len(), 6);
        let got = top_area(&object_solids(&m, run.members[0]));
        assert!((got - 6.0 * 0.36).abs() < 1e-6, "island: {got} m²");
        // Taken out of the run, a module has its own top again.
        m.edit_object(d, |o| o.position = Point2::new(6.0, 6.0))
            .unwrap();
        assert!(top_area(&object_solids(&m, d)) > 0.3);
    }
}
