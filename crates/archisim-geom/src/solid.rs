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

pub use crate::stairs::{railing_bars, railing_material, stair_parts, stair_solids};

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
}

impl SolidOwner {
    /// The id of the element.
    pub fn id(self) -> u64 {
        match self {
            SolidOwner::Wall(x) => x.0,
            SolidOwner::Slab(x) => x.0,
            SolidOwner::Opening(x) => x.0,
            SolidOwner::Column(x) => x.0,
            SolidOwner::Beam(x) => x.0,
            SolidOwner::Roof(x) => x.0,
            SolidOwner::Dormer(x, _) => x.0,
            SolidOwner::Stair(x) => x.0,
            SolidOwner::Terrain(x) => x.0,
            SolidOwner::CurtainWall(x) => x.0,
            SolidOwner::Railing(x) => x.0,
            SolidOwner::Object(x) => x.0,
            SolidOwner::Footing(x) => x.0,
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
    for notch in &g.notches {
        inner.extend(notch.boundaries.iter().copied());
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
        let holes: Vec<Vec<Point2>> = roof
            .plane_holes(plane.side)
            .into_iter()
            .filter(|h| {
                h.iter()
                    .all(|q| archisim_core::point_in_polygon(*q, &plane.outline))
            })
            .collect();
        out.extend(layered_plate(
            SolidOwner::Roof(id),
            &plane.outline,
            &holes,
            &|p| level + roof.base_offset + slope.rise_at(p),
            &|t| slope.vertical(t),
            &stype.layers,
        ));
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
    let mut out: Vec<Solid> = parts
        .iter()
        .map(|(material, tris)| Solid {
            owner: SolidOwner::Object(id),
            material: *material,
            triangles: tris
                .iter()
                .map(|t| t.map(|i| place(mesh.positions[i as usize])))
                .collect(),
        })
        .collect();
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

/// Solids of a loose railing: its bars, each a closed box, as one solid.
pub fn loose_railing_solids(model: &Model, id: archisim_core::RailingId) -> Vec<Solid> {
    let Some(r) = model.railing(id) else {
        return Vec::new();
    };
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
    let mut out: Vec<[P3; 3]> = tris
        .iter()
        .map(|[a, b, c]| [p3(*a), p3(*b), p3(*c)])
        .collect();
    let ring = t.boundary();
    let poly: Vec<Point2> = ring.iter().map(|i| t.points[*i].at).collect();
    for [a, b, c] in triangulate(&poly) {
        out.push([low(ring[a]), low(ring[c]), low(ring[b])]);
    }
    let n = ring.len();
    for k in 0..n {
        let (a, b) = (ring[k], ring[(k + 1) % n]);
        out.push([low(a), low(b), p3(b)]);
        out.push([low(a), p3(b), p3(a)]);
    }
    vec![Solid {
        owner: SolidOwner::Terrain(id),
        material: t.material,
        triangles: out,
    }]
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

/// The opening lines a door or window's recipe draws in elevation, in the
/// model: each segment twice, on the face of the element its leaves open
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
    let mut out = Vec::new();
    let idx = WallIndex::new(model);
    for w in model.walls() {
        out.extend(wall_solids_in(&idx, w.id));
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
        out.extend(opening_solids(model, o.id));
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
    }
    for c in model.curtain_walls() {
        out.extend(curtain_wall_solids(model, c.id));
    }
    for r in model.railings() {
        out.extend(loose_railing_solids(model, r.id));
    }
    for o in model.objects() {
        out.extend(object_solids(model, o.id));
    }
    for f in model.footings() {
        out.extend(footing_solids(model, f.id));
    }
    for m in model.masses() {
        out.extend(mass_solids(model, m.id));
    }
    for r in model.roads() {
        out.extend(road_solids(model, r.id));
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
/// its depth, and the kerbs and sidewalks standing their height over it.
pub fn road_solids(model: &Model, id: archisim_core::RoadId) -> Vec<Solid> {
    use archisim_core::RoadBand;
    let Some(r) = model.road(id) else {
        return Vec::new();
    };
    let top = model.level(r.level).map_or(0.0, |l| l.elevation) + r.top_offset;
    let s = r.section;
    r.bands()
        .into_iter()
        .map(|(band, outline)| {
            let (material, z0, z1) = match band {
                RoadBand::Carriageway | RoadBand::Parking => (r.pavement, top - s.depth, top),
                RoadBand::Kerb => (r.kerb_material, top - s.depth, top + s.kerb_height),
                RoadBand::Sidewalk => (
                    r.sidewalk_material,
                    top + s.kerb_height - 0.2,
                    top + s.kerb_height,
                ),
            };
            Solid {
                owner: SolidOwner::Road(id),
                material,
                triangles: prism(&outline, z0, z1),
            }
        })
        .collect()
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
    let tris: Vec<[P3; 3]> = r
        .path
        .windows(2)
        .zip(zs.windows(2))
        .flat_map(|(w, z)| {
            sweep_segment(
                &section,
                [w[0].x, w[0].y, z0 + z[0]],
                [w[1].x, w[1].y, z0 + z[1]],
            )
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
    out
}

/// The solid of a device: its box (a light, a disc).
pub fn device_solids(model: &Model, id: archisim_core::DeviceId) -> Vec<Solid> {
    let Some(d) = model.device(id) else {
        return Vec::new();
    };
    let z = model.level(d.level).map_or(0.0, |l| l.elevation) + d.elevation;
    let outline = if d.kind == archisim_core::DeviceKind::Light {
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

/// The colour a solid is shown with: a pipe's or duct's that of its
/// system, a device's a light grey, the rest their material's.
pub fn solid_color(model: &Model, s: &Solid) -> [u8; 3] {
    match s.owner {
        SolidOwner::Run(id) => model.run(id).map_or([150, 150, 150], |r| r.system.color()),
        SolidOwner::Device(id) => match model.device(id).map(|d| d.kind) {
            Some(archisim_core::DeviceKind::Light) => [250, 230, 150],
            Some(archisim_core::DeviceKind::Radiator) => [245, 245, 240],
            Some(archisim_core::DeviceKind::Grille | archisim_core::DeviceKind::Diffuser) => {
                [200, 205, 210]
            }
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
        SolidOwner::Wall(id) => {
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
        _ => None,
    }
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
        SolidOwner::Wall(id) => id.0,
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
    model
        .face_paint(element, face)
        .filter(|m| model.material(*m).is_some())
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

/// Solids of the elements on one level only.
pub fn level_solids(model: &Model, level: LevelId) -> Vec<Solid> {
    let on_level = |owner: SolidOwner| match owner {
        SolidOwner::Wall(w) => model.wall(w).is_some_and(|w| w.level == level),
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
        SolidOwner::Terrain(t) => model.terrain(t).is_some_and(|t| t.level == level),
        SolidOwner::CurtainWall(c) => model.curtain_wall(c).is_some_and(|c| c.level == level),
        SolidOwner::Railing(r) => model.railing(r).is_some_and(|r| r.level == level),
        SolidOwner::Object(o) => model.object(o).is_some_and(|o| o.level == level),
        SolidOwner::Footing(f) => model.footing(f).is_some_and(|f| f.level == level),
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
        // (A sink keeps the rim of its basin, a few cm², level with it.)
        for o in [b, c, e, d] {
            let left = top_area(&object_solids(&m, o));
            assert!(left < 0.01, "{o:?}: {left}");
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
