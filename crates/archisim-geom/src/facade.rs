//! The pieces of a façade (E9): the bricks, blocks, panels or slabs of the
//! exterior finish of a wall whose type has a [`Facade`], laid by its bond
//! on what is left of the face — round the doors and windows (each stretch
//! between them starts its course again at the jamb), under the roofs and
//! profiles that cut the wall — with lintels of bricks on end over the
//! openings, bonded corners with the walls of the same kind, and a band of
//! its own piece over the edge of every slab the face passes (the «frente
//! de forjado»).
//!
//! Each piece stands a little out of the face of its layer ([`RELIEF`]),
//! so the face behind shows in the joints, in the colour of the mortar
//! ([`joint_material`]).

use archisim_core::{Coursing, MaterialId, Model, Point2, WallId};

use crate::junctions::{difference, intersection, union, Shape};
use crate::solid::{Solid, SolidOwner, P3};
use crate::{signed_area, wall_geometry_in, EndCut, WallIndex};

/// How far the pieces stand out of the face of their layer (the depth of
/// the joints).
pub const RELIEF: f64 = 0.006;

/// Tolerance for the face of the layer.
const ON_FACE: f64 = 1e-4;

/// One piece of a façade, in the face's own axes: `x` along the wall from
/// its start, `y` the height in the model.
#[derive(Debug, Clone, PartialEq)]
pub struct FacadePiece {
    pub material: MaterialId,
    pub outline: Vec<Point2>,
    /// How far its front stands out of the face of the layer.
    pub out: f64,
    /// Part of the band over a slab edge.
    pub slab_front: bool,
    /// A whole piece (not cut by an edge, a jamb or a roof).
    pub whole: bool,
}

/// The pieces of a wall's façade, with the frame of its face.
#[derive(Debug, Clone)]
pub struct FacadeLayout {
    pub wall: WallId,
    /// The start of the wall's line, the direction along it and the normal
    /// towards the exterior.
    pub origin: Point2,
    pub along: Point2,
    pub normal: Point2,
    /// Offset of the exterior face from the wall's line.
    pub face: f64,
    pub pieces: Vec<FacadePiece>,
}

impl FacadeLayout {
    /// A point of the face (`x` along, at height `z`, `d` out of it) in the
    /// model.
    pub fn point(&self, x: f64, z: f64, d: f64) -> P3 {
        let p = self.origin + self.along * x + self.normal * (self.face + d);
        [p.x, p.y, z]
    }
}

/// The mortar of a façade's joints, if its bond names one.
pub fn joint_material(model: &Model, wall: WallId) -> Option<MaterialId> {
    model
        .wall_facade(wall)?
        .coursing
        .joint_material
        .filter(|m| model.material(*m).is_some())
}

/// Whether a point with that normal is on the exterior face of a wall with
/// a façade: the face behind the pieces, which shows in the joints.
pub fn on_facade_face(model: &Model, wall: WallId, at: P3, normal: P3) -> bool {
    let Some(w) = model.wall(wall) else {
        return false;
    };
    let Some(t) = model.wall_type(w.wall_type) else {
        return false;
    };
    if t.facade.is_none() || w.is_curved() {
        return false;
    }
    let Some(dir) = (w.end - w.start).normalized() else {
        return false;
    };
    let n = dir.perp_left();
    let len = (normal[0] * normal[0] + normal[1] * normal[1] + normal[2] * normal[2]).sqrt();
    if len < 1e-12 || (normal[0] * n.x + normal[1] * n.y) / len < 0.99 {
        return false;
    }
    let (ext, _) = w.face_offsets(t.thickness());
    let d = (Point2::new(at[0], at[1]) - w.start).dot(n);
    (d - ext).abs() < 1e-3
}

fn rect(x0: f64, x1: f64, y0: f64, y1: f64) -> Vec<Point2> {
    vec![
        Point2::new(x0, y0),
        Point2::new(x1, y0),
        Point2::new(x1, y1),
        Point2::new(x0, y1),
    ]
}

fn bounds(shapes: &[Shape]) -> Option<(Point2, Point2)> {
    let mut it = shapes.iter().flat_map(|(o, _)| o.iter());
    let first = *it.next()?;
    Some(it.fold((first, first), |(lo, hi), p| {
        (
            Point2::new(lo.x.min(p.x), lo.y.min(p.y)),
            Point2::new(hi.x.max(p.x), hi.y.max(p.y)),
        )
    }))
}

/// The rectangle a shape is, if it is one square to the axes and without
/// holes.
fn as_rect(shape: &Shape) -> Option<(f64, f64, f64, f64)> {
    let (o, holes) = shape;
    if !holes.is_empty() {
        return None;
    }
    let (lo, hi) = bounds(std::slice::from_ref(shape))?;
    let area = (hi.x - lo.x) * (hi.y - lo.y);
    ((signed_area(o).abs() - area).abs() < 1e-8).then_some((lo.x, hi.x, lo.y, hi.y))
}

/// The pieces of a wall's façade. For many walls, build a [`WallIndex`]
/// once; `solids` are the wall's own (from [`crate::solid::wall_solids_in`]),
/// whose exterior face is where the pieces go.
pub fn facade_layout(idx: &WallIndex, id: WallId, solids: &[Solid]) -> Option<FacadeLayout> {
    let model = idx.model;
    let facade = *model.wall_facade(id)?;
    let wall = model.wall(id)?;
    let wtype = model.wall_type(wall.wall_type)?;
    let first = wtype.layers.first()?;
    let g = wall_geometry_in(idx, id)?;
    let along = (wall.end - wall.start).normalized()?;
    let normal = along.perp_left();
    let origin = wall.start;
    let face = g.exterior_offset;
    let level = model.level(wall.level).map_or(0.0, |l| l.elevation);
    let flat = |p: &P3| Point2::new((Point2::new(p[0], p[1]) - origin).dot(along), p[2]);
    // What is left of the face: the triangles of the wall's solids on it.
    let mut tris: Vec<Shape> = Vec::new();
    for s in solids {
        for t in &s.triangles {
            let u = [t[1][0] - t[0][0], t[1][1] - t[0][1], t[1][2] - t[0][2]];
            let v = [t[2][0] - t[0][0], t[2][1] - t[0][1], t[2][2] - t[0][2]];
            let nz = [
                u[1] * v[2] - u[2] * v[1],
                u[2] * v[0] - u[0] * v[2],
                u[0] * v[1] - u[1] * v[0],
            ];
            let len = (nz[0] * nz[0] + nz[1] * nz[1] + nz[2] * nz[2]).sqrt();
            if len < 1e-12 || (nz[0] * normal.x + nz[1] * normal.y) / len < 0.99 {
                continue;
            }
            let on = t
                .iter()
                .all(|p| ((Point2::new(p[0], p[1]) - origin).dot(normal) - face).abs() < ON_FACE);
            if !on {
                continue;
            }
            let poly: Vec<Point2> = t.iter().map(flat).collect();
            if signed_area(&poly).abs() > 1e-10 {
                tris.push((crate::ensure_ccw(poly), Vec::new()));
            }
        }
    }
    if tris.is_empty() {
        return None;
    }
    let region = union(&tris, &[]);
    let (lo, hi) = bounds(&region)?;
    let c = facade.coursing;
    let mut pieces = Vec::new();
    let mut taken: Vec<Shape> = Vec::new();

    // Bands over the edges of the slabs the face passes.
    if let Some(front) = facade.slab_front.filter(|f| f.coursing.is_valid()) {
        let inside = face - first.thickness - 0.01;
        let (a, b) = (
            origin + normal * inside + along * lo.x,
            origin + normal * inside + along * hi.x,
        );
        let (blo, bhi) = (
            Point2::new(a.x.min(b.x) - 0.05, a.y.min(b.y) - 0.05),
            Point2::new(a.x.max(b.x) + 0.05, a.y.max(b.y) + 0.05),
        );
        let material = front.material.unwrap_or(first.material);
        for s in idx.slabs_in_box(blo, bhi) {
            if s.slope.is_some() {
                continue;
            }
            let Some(st) = model.slab_type(s.slab_type) else {
                continue;
            };
            let top = model.level(s.level).map_or(0.0, |l| l.elevation) + s.top_offset;
            let bottom = top - st.thickness();
            let mid = (top + bottom) / 2.0;
            // The band is this wall's if the middle of the edge is on its
            // face (the wall below a floor, not the one standing on it).
            if mid <= lo.y + 1e-6 || mid >= hi.y - 1e-6 {
                continue;
            }
            for (x0, x1) in segment_inside(a, b, &s.outline, &s.holes) {
                let (x0, x1) = (lo.x + x0 * (hi.x - lo.x), lo.x + x1 * (hi.x - lo.x));
                let band = vec![(
                    rect(x0, x1, bottom - front.over, top + front.over),
                    Vec::new(),
                )];
                // Not over the doors and windows.
                let band = difference(&band, &openings(idx, id, level));
                let band = intersection(&band, &[(rect(lo.x, hi.x, lo.y, hi.y), Vec::new())]);
                if band.is_empty() {
                    continue;
                }
                lay(
                    &front.coursing,
                    &band,
                    material,
                    RELIEF + front.projection.max(0.0),
                    true,
                    &mut pieces,
                );
                taken.extend(band);
            }
        }
    }

    // Lintels of bricks on end over the doors and windows.
    if c.soldier_heads {
        let soldier = Coursing {
            length: c.height,
            height: c.length,
            bond: archisim_core::Bond::Stack,
            pattern: archisim_core::Pattern::Plain,
            ..c
        };
        for o in idx.openings_in_wall(id) {
            if !o.shape.is_rect() {
                continue;
            }
            let (t0, t1) = o.cut_span();
            let head = level + o.head;
            let reach = c.height + c.joint;
            let band = vec![(
                rect(t0 - reach, t1 + reach, head, head + c.length + c.joint),
                Vec::new(),
            )];
            let band = difference(&intersection(&band, &region), &taken);
            if band.is_empty() {
                continue;
            }
            // The lintel's own joints: the band less its top joint.
            let laid = intersection(
                &band,
                &[(
                    rect(t0 - reach, t1 + reach, head, head + c.length),
                    Vec::new(),
                )],
            );
            lay(&soldier, &laid, first.material, RELIEF, false, &mut pieces);
            taken.extend(band);
        }
    }

    // The courses of the rest.
    let rest = difference(&region, &taken);
    let turns = (
        corner_turns(idx, id, &g, true),
        corner_turns(idx, id, &g, false),
    );
    let (accent, base) = (c.accent.unwrap_or(first.material), first.material);
    let ch = c.course_height();
    let (k0, k1) = ((lo.y / ch).floor() as i64, (hi.y / ch).ceil() as i64);
    for k in k0..=k1 {
        let (y0, y1) = (k as f64 * ch, k as f64 * ch + c.height);
        if y1 <= lo.y || y0 >= hi.y {
            continue;
        }
        let strip = intersection(&rest, &[(rect(lo.x - 1.0, hi.x + 1.0, y0, y1), Vec::new())]);
        for shape in &strip {
            let Some((bl, bh)) = bounds(std::slice::from_ref(shape)) else {
                continue;
            };
            let at_start = (bl.x - lo.x).abs() < 1e-3;
            let at_end = (bh.x - hi.x).abs() < 1e-3;
            let turn_left = turns.0.as_ref().filter(|_| at_start).map(|f| f(k));
            let turn_right = turns.1.as_ref().filter(|_| at_end).map(|f| f(k));
            let rect_shape = as_rect(shape);
            for u in course_with_ends(&c, k, bh.x - bl.x, turn_left, turn_right) {
                let (x0, x1) = (bl.x + u.start, bl.x + u.end);
                let r = rect(x0, x1, y0, y1);
                let pick = c
                    .accent_by_place((x0 + x1) / 2.0 - lo.x, k)
                    .unwrap_or(u.accent);
                let material = if pick { accent } else { base };
                let whole_len = if u.header { c.depth } else { c.length };
                match rect_shape {
                    Some((_, _, sy0, sy1)) => {
                        let r = rect(x0, x1, y0.max(sy0), y1.min(sy1));
                        let whole = (x1 - x0 - whole_len).abs() < 1e-6
                            && (y1.min(sy1) - y0.max(sy0) - c.height).abs() < 1e-6;
                        pieces.push(FacadePiece {
                            material,
                            outline: r,
                            out: RELIEF,
                            slab_front: false,
                            whole,
                        });
                    }
                    None => {
                        for (o, _) in intersection(&[(r, Vec::new())], std::slice::from_ref(shape))
                        {
                            pieces.push(FacadePiece {
                                material,
                                outline: o,
                                out: RELIEF,
                                slab_front: false,
                                whole: false,
                            });
                        }
                    }
                }
            }
        }
    }
    Some(FacadeLayout {
        wall: id,
        origin,
        along,
        normal,
        face,
        pieces,
    })
}

/// The course with the end of the other wall's piece at a bonded corner
/// at its start (`turn_left`) or its end (`turn_right`).
fn course_with_ends(
    c: &Coursing,
    k: i64,
    len: f64,
    left: Option<bool>,
    right: Option<bool>,
) -> Vec<archisim_core::Unit> {
    use archisim_core::Start;
    let corners = c.bonds_at_corners();
    let start = match left.filter(|_| corners) {
        Some(true) => Start::Turn,
        Some(false) => Start::Corner,
        None => Start::Free,
    };
    let mut units = c.course_from(k, len, start);
    if right.filter(|_| corners) == Some(true) && len > 2.0 * c.depth + c.joint {
        let cut = len - c.depth - c.joint;
        units.retain(|u| u.start < cut - 1e-6);
        if let Some(last) = units.last_mut() {
            last.end = last.end.min(cut);
        }
        units.push(archisim_core::Unit {
            start: len - c.depth,
            end: len,
            header: true,
            accent: false,
        });
    }
    units
}

/// Lays the courses of `c` on `shapes` (each course from the left of each
/// stretch of it).
fn lay(
    c: &Coursing,
    shapes: &[Shape],
    material: MaterialId,
    out: f64,
    slab_front: bool,
    pieces: &mut Vec<FacadePiece>,
) {
    let Some((lo, hi)) = bounds(shapes) else {
        return;
    };
    let ch = c.course_height();
    // The courses of a band start at its bottom.
    let base = lo.y;
    let n = ((hi.y - base) / ch).ceil() as i64;
    for k in 0..n.max(1) {
        let (y0, y1) = (base + k as f64 * ch, base + k as f64 * ch + c.height);
        let strip = intersection(
            shapes,
            &[(rect(lo.x - 1.0, hi.x + 1.0, y0, y1), Vec::new())],
        );
        for shape in &strip {
            let Some((bl, bh)) = bounds(std::slice::from_ref(shape)) else {
                continue;
            };
            for u in course_with_ends(c, k, bh.x - bl.x, None, None) {
                let r = rect(bl.x + u.start, bl.x + u.end, y0, y1);
                for (o, _) in intersection(&[(r, Vec::new())], std::slice::from_ref(shape)) {
                    pieces.push(FacadePiece {
                        material,
                        outline: o,
                        out,
                        slab_front,
                        whole: false,
                    });
                }
            }
        }
    }
}

/// The doors and windows of a wall in its face's axes.
fn openings(idx: &WallIndex, id: WallId, level: f64) -> Vec<Shape> {
    idx.openings_in_wall(id)
        .iter()
        .map(|o| {
            let (t0, t1) = o.cut_span();
            (rect(t0, t1, level + o.sill, level + o.head), Vec::new())
        })
        .collect()
}

/// Which courses show the end of the other wall's piece at a corner (the
/// start or the end of the wall), if that wall has a façade too: the two
/// walls take turns, course by course.
#[allow(clippy::type_complexity)]
fn corner_turns(
    idx: &WallIndex,
    id: WallId,
    g: &crate::WallGeometry,
    start: bool,
) -> Option<Box<dyn Fn(i64) -> bool>> {
    let model = idx.model;
    let wall = model.wall(id)?;
    let cut = if start { &g.start_cut } else { &g.end_cut };
    if !matches!(cut, EndCut::Along(_)) {
        return None;
    }
    let at = if start { wall.start } else { wall.end };
    let other = idx
        .at_end(wall.level, at)
        .into_iter()
        .find(|o| o.id != id && model.wall_facade(o.id).is_some())?;
    let first = (id.0 < other.id.0) as i64;
    Some(Box::new(move |k: i64| (k + first).rem_euclid(2) == 0))
}

/// The stretches of the segment `a`–`b` inside a polygon (and not in its
/// holes), as fractions of its length.
fn segment_inside(
    a: Point2,
    b: Point2,
    outline: &[Point2],
    holes: &[Vec<Point2>],
) -> Vec<(f64, f64)> {
    let mut ts = vec![0.0, 1.0];
    let d = b - a;
    for ring in std::iter::once(outline).chain(holes.iter().map(|h| h.as_slice())) {
        for i in 0..ring.len() {
            let (p, q) = (ring[i], ring[(i + 1) % ring.len()]);
            let e = q - p;
            let den = d.x * e.y - d.y * e.x;
            if den.abs() < 1e-12 {
                continue;
            }
            let w = p - a;
            let t = (w.x * e.y - w.y * e.x) / den;
            let s = (w.x * d.y - w.y * d.x) / den;
            if (0.0..=1.0).contains(&t) && (-1e-9..=1.0 + 1e-9).contains(&s) {
                ts.push(t);
            }
        }
    }
    ts.sort_by(|x, y| x.total_cmp(y));
    ts.dedup_by(|x, y| (*x - *y).abs() < 1e-9);
    let inside = |p: Point2| {
        archisim_core::point_in_polygon(p, outline)
            && !holes.iter().any(|h| archisim_core::point_in_polygon(p, h))
    };
    let mut out: Vec<(f64, f64)> = Vec::new();
    for w in ts.windows(2) {
        let m = a + d * ((w[0] + w[1]) / 2.0);
        if inside(m) {
            match out.last_mut() {
                Some(last) if (last.1 - w[0]).abs() < 1e-9 => last.1 = w[1],
                _ => out.push((w[0], w[1])),
            }
        }
    }
    out
}

/// The pieces of a façade as solids: each piece from the face of its layer
/// out to its front, without the back (against the face).
pub fn facade_solids(layout: &FacadeLayout) -> Vec<Solid> {
    let mut by_material: std::collections::BTreeMap<u64, Vec<[P3; 3]>> = Default::default();
    for p in &layout.pieces {
        let tris = by_material.entry(p.material.0).or_default();
        piece_triangles(layout, p, tris);
    }
    by_material
        .into_iter()
        .map(|(m, triangles)| Solid {
            owner: SolidOwner::Facade(layout.wall),
            material: MaterialId(m),
            triangles,
        })
        .collect()
}

/// The solids of a wall's façade, from its own solids. Empty without one.
pub fn facade_solids_in(idx: &WallIndex, id: WallId, solids: &[Solid]) -> Vec<Solid> {
    facade_layout(idx, id, solids).map_or_else(Vec::new, |l| facade_solids(&l))
}

fn piece_triangles(layout: &FacadeLayout, p: &FacadePiece, out: &mut Vec<[P3; 3]>) {
    let poly = crate::ensure_ccw(p.outline.clone());
    if poly.len() < 3 {
        return;
    }
    let front = |q: Point2| layout.point(q.x, q.y, p.out);
    let back = |q: Point2| layout.point(q.x, q.y, 0.0);
    // Counter-clockwise in (along, up) looks out of the face when the
    // normal is to the left of the wall's line: along × up = −normal,
    // so the front is wound the other way.
    for t in crate::solid::triangulate(&poly) {
        out.push([front(poly[t[0]]), front(poly[t[2]]), front(poly[t[1]])]);
    }
    for i in 0..poly.len() {
        let (a, b) = (poly[i], poly[(i + 1) % poly.len()]);
        out.push([back(a), front(a), front(b)]);
        out.push([back(a), front(b), back(b)]);
    }
}

/// How many whole and cut pieces of each material a façade has.
pub fn piece_counts(layout: &FacadeLayout) -> Vec<(MaterialId, bool, usize, usize)> {
    let mut map: std::collections::BTreeMap<(u64, bool), (usize, usize)> = Default::default();
    for p in &layout.pieces {
        let e = map.entry((p.material.0, p.slab_front)).or_default();
        if p.whole {
            e.0 += 1;
        } else {
            e.1 += 1;
        }
    }
    map.into_iter()
        .map(|((m, front), (w, c))| (MaterialId(m), front, w, c))
        .collect()
}

/// The joints between the pieces of a façade cut at height `z` (in the
/// model), along the wall's line, in the stretch `from`..`to` of it
/// (`None`: from the end of the face), for the plan at a scale of detail.
/// The same courses as [`facade_layout`], without what a roof or a
/// profile cuts.
pub fn plan_joints(
    model: &Model,
    g: &crate::WallGeometry,
    from: Option<f64>,
    to: Option<f64>,
    z: f64,
) -> Vec<f64> {
    let Some(facade) = model.wall_facade(g.wall) else {
        return Vec::new();
    };
    let Some(wall) = model.wall(g.wall) else {
        return Vec::new();
    };
    let Some(along) = (wall.end - wall.start).normalized() else {
        return Vec::new();
    };
    let Some(band) = g.layers.first() else {
        return Vec::new();
    };
    // Where the face starts and ends.
    let (lo, hi) = band
        .polygon
        .iter()
        .fold((f64::MAX, f64::MIN), |(lo, hi), p| {
            let t = (*p - wall.start).dot(along);
            (lo.min(t), hi.max(t))
        });
    let c = facade.coursing;
    let k = (z / c.course_height()).floor() as i64;
    let (s, e) = (from.unwrap_or(lo), to.unwrap_or(hi));
    let corner = |start: bool| -> Option<bool> {
        let cut = if start { &g.start_cut } else { &g.end_cut };
        if !matches!(cut, EndCut::Along(_)) {
            return None;
        }
        let at = if start { wall.start } else { wall.end };
        let other = model.walls_on_level(wall.level).find(|o| {
            o.id != wall.id
                && (o.start.approx_eq(at) || o.end.approx_eq(at))
                && model.wall_facade(o.id).is_some()
        })?;
        let first = (wall.id.0 < other.id.0) as i64;
        Some((k + first).rem_euclid(2) == 0)
    };
    let left = if from.is_none() { corner(true) } else { None };
    let right = if to.is_none() { corner(false) } else { None };
    let mut out = Vec::new();
    for u in course_with_ends(&c, k, e - s, left, right) {
        for t in [u.start, u.end] {
            if t > 1e-6 && t < e - s - 1e-6 {
                out.push(s + t);
            }
        }
    }
    out
}

/// Whether a wall's type has a façade to lay (used to skip the work).
pub fn has_facade(model: &Model, id: WallId) -> bool {
    model.wall_facade(id).is_some()
}
