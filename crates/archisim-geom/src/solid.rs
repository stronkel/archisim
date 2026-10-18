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
    LevelId, MaterialId, Model, OpeningId, Point2, SlabId, WallId, WallSide, EPSILON,
};

use crate::{ensure_ccw, opening_geometry, openings_in_wall, wall_geometry, EndCut, Line};

/// A point in model space: metres, Z up.
pub type P3 = [f64; 3];

/// The element a solid belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum SolidOwner {
    Wall(WallId),
    Slab(SlabId),
    Opening(OpeningId),
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
    let (Some(wall), Some(g)) = (model.wall(id), wall_geometry(model, id)) else {
        return Vec::new();
    };
    let Some(wtype) = model.wall_type(wall.wall_type) else {
        return Vec::new();
    };
    let level = model.level(wall.level).map_or(0.0, |l| l.elevation);
    let z_base = level + wall.base_offset;
    let z_top = z_base + wall.height;
    let (a, b) = g.axis;
    let len = a.distance(b);
    let Some(u) = (b - a).normalized() else {
        return Vec::new();
    };
    let n = u.perp_left();
    let openings = openings_in_wall(model, id);
    let layers = wtype.layers.len();

    // Layer boundaries across the wall, from the exterior face inwards.
    let mut offsets = vec![g.exterior_offset];
    for layer in &wtype.layers {
        let last = *offsets.last().expect("not empty");
        offsets.push(last - layer.thickness);
    }
    let along = |line: &Line, off: f64, fallback: f64| -> f64 {
        Line::new(a + n * off, b - a)
            .intersect(line)
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
        .flat_map(|o| [o.t0, o.t1])
        .filter(|t| *t > EPSILON && *t < len - EPSILON)
        .map(|t| Line::new(a + u * t, n))
        .collect();
    for notch in &g.notches {
        inner.extend(notch.boundaries.iter().copied());
    }
    let mid = (offsets[0] + offsets[layers]) / 2.0;
    let mut keyed: Vec<(f64, Line)> = inner
        .into_iter()
        .map(|l| (along(&l, mid, -1.0), l))
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
            Col::Start => g.start_cut.line(k).map_or(0.0, |l| along(&l, off, 0.0)),
            Col::End => g.end_cut.line(k).map_or(len, |l| along(&l, off, len)),
            Col::At(l) => along(l, off, 0.0),
        }
    };

    // Rows: base, top and every sill and head in between, also those of
    // the walls joined at the ends or butting into this one, so both sides
    // of a junction share their vertices.
    let mut zs: Vec<f64> = vec![z_base, z_top];
    let mut heights: Vec<(f64, f64)> = openings.iter().map(|o| (o.sill, o.head)).collect();
    for other in model.walls_on_level(wall.level) {
        let touches = [other.start, other.end]
            .iter()
            .any(|p| p.approx_eq(wall.start) || p.approx_eq(wall.end));
        if other.id != id && touches {
            heights.extend(
                openings_in_wall(model, other.id)
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
        let in_opening = openings
            .iter()
            .any(|o| t > o.t0 && t < o.t1 && z > level + o.sill && z < level + o.head);
        let p = a + u * t + n * off;
        let in_notch = g.notches.iter().any(|nt| {
            z > level + nt.z.0 && z < level + nt.z.1 && {
                nt.regions.iter().any(|reg| {
                    let (f, lo, hi) = reg.strip();
                    reg.layers.contains(&k) && f(p) > lo + EPSILON && f(p) < hi - EPSILON
                })
            }
        });
        !in_opening && !in_notch
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
    for c in 0..ncols - 1 {
        for r in 0..rows {
            for (k, out) in tris.iter_mut().enumerate() {
                if !solid(c, r, k) {
                    continue;
                }
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
                let open_before = c == 0 && !joined_start || c > 0 && !solid(c - 1, r, k);
                let open_after =
                    c + 2 == ncols && !joined_end || c + 2 < ncols && !solid(c + 1, r, k);
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
                let open_below = r == 0 || !solid(c, r - 1, k);
                let open_above = r + 1 == rows || !solid(c, r + 1, k);
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
    wtype
        .layers
        .iter()
        .zip(tris)
        .map(|(layer, triangles)| Solid {
            owner: SolidOwner::Wall(id),
            material: layer.material,
            triangles,
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
    let Some(slab) = model.slab(id) else {
        return Vec::new();
    };
    let Some(stype) = model.slab_type(slab.slab_type) else {
        return Vec::new();
    };
    let level = model.level(slab.level).map_or(0.0, |l| l.elevation);
    let poly = ensure_ccw(slab.outline.clone());
    let caps = triangulate(&poly);
    let at = |p: Point2, z: f64| [p.x, p.y, z];
    let mut top = level + slab.top_offset;
    let count = stype.layers.len();
    stype
        .layers
        .iter()
        .enumerate()
        .map(|(i, layer)| {
            let bottom = top - layer.thickness;
            let mut tris = Vec::new();
            if i == 0 {
                for [x, y, z] in &caps {
                    tris.push([at(poly[*x], top), at(poly[*y], top), at(poly[*z], top)]);
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
            for k in 0..poly.len() {
                let (p, q) = (poly[k], poly[(k + 1) % poly.len()]);
                tris.push([at(p, bottom), at(q, bottom), at(q, top)]);
                tris.push([at(p, bottom), at(q, top), at(p, top)]);
            }
            top = bottom;
            Solid {
                owner: SolidOwner::Slab(id),
                material: layer.material,
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
    let (Some(o), Some(og)) = (model.opening(id), opening_geometry(model, id)) else {
        return Vec::new();
    };
    let Ok(ev) = model.evaluate_opening(id) else {
        return Vec::new();
    };
    let Some(wall) = model.wall(o.wall) else {
        return Vec::new();
    };
    let level = model.level(wall.level).map_or(0.0, |l| l.elevation);
    let z_sill = level + og.sill;
    let mid = (og.t0 + og.t1) / 2.0;
    let y_mid = (og.frame_exterior + og.frame_interior) / 2.0;
    // The normal `n` points to the exterior. Seen from the exterior, the
    // wall's direction runs to the left; seen from the interior, to the
    // right.
    let (sx, sy) = match og.opens_to {
        WallSide::Exterior => (-1.0, 1.0),
        WallSide::Interior => (1.0, -1.0),
    };

    let mut by_material: Vec<(MaterialId, Vec<[P3; 3]>)> = Vec::new();
    for part in &ev.parts {
        let (ta, tb) = sorted(mid + sx * part.min[0], mid + sx * part.max[0]);
        let (ya, yb) = sorted(y_mid + sy * part.min[1], y_mid + sy * part.max[1]);
        let rect = [
            og.point(ta, ya),
            og.point(tb, ya),
            og.point(tb, yb),
            og.point(ta, yb),
        ];
        let tris = prism(&rect, z_sill + part.min[2], z_sill + part.max[2]);
        let material = MaterialId(part.material);
        match by_material.iter_mut().find(|(m, _)| *m == material) {
            Some((_, list)) => list.extend(tris),
            None => by_material.push((material, tris)),
        }
    }
    by_material
        .into_iter()
        .map(|(material, triangles)| Solid {
            owner: SolidOwner::Opening(id),
            material,
            triangles,
        })
        .collect()
}

fn sorted(a: f64, b: f64) -> (f64, f64) {
    if a <= b {
        (a, b)
    } else {
        (b, a)
    }
}

/// Every solid of the model, on every level.
pub fn model_solids(model: &Model) -> Vec<Solid> {
    let mut out = Vec::new();
    for w in model.walls() {
        out.extend(wall_solids(model, w.id));
    }
    for s in model.slabs() {
        out.extend(slab_solids(model, s.id));
    }
    for o in model.openings() {
        out.extend(opening_solids(model, o.id));
    }
    out.retain(|s| !s.triangles.is_empty());
    out
}

/// Solids of the elements on one level only.
pub fn level_solids(model: &Model, level: LevelId) -> Vec<Solid> {
    let on_level = |owner: SolidOwner| match owner {
        SolidOwner::Wall(w) => model.wall(w).is_some_and(|w| w.level == level),
        SolidOwner::Slab(s) => model.slab(s).is_some_and(|s| s.level == level),
        SolidOwner::Opening(o) => model
            .opening(o)
            .and_then(|o| model.wall(o.wall))
            .is_some_and(|w| w.level == level),
    };
    model_solids(model)
        .into_iter()
        .filter(|s| on_level(s.owner))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

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
                through: None,
                reference: ReferenceLine::Center,
                height: 3.0,
                base_offset: 0.0,
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

    #[test]
    fn wall_volume_matches_the_ifc_net_volume() {
        let (m, w) = model();
        let solids = wall_solids(&m, w);
        assert_eq!(solids.len(), 2, "one solid per layer");
        let volume: f64 = solids.iter().map(Solid::volume).sum();
        // The same computation as Qto_WallBaseQuantities.NetVolume.
        let g = wall_geometry(&m, w).unwrap();
        let gross = crate::signed_area(&g.outline).abs() * 3.0;
        let cut: f64 = openings_in_wall(&m, w)
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
                through: None,
                reference: ReferenceLine::ExteriorFace,
                height: 3.0,
                base_offset: 0.0,
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
                through: None,
                reference: ReferenceLine::Center,
                height: 3.0,
                base_offset: 0.0,
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
                through: None,
                reference: ReferenceLine::ExteriorFace,
                height: 3.1,
                base_offset: 0.0,
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
