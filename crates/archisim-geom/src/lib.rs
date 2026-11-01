//! Plan geometry of building elements.
//!
//! The main entry point is [`wall_geometry`], which turns a wall (an axis
//! plus a layered type) into closed polygons: one for the whole wall and one
//! per material layer, with corners resolved against connected walls.
//!
//! Current join rules (MVP):
//! - Two walls sharing an end point on the same level meet with a mitre: the
//!   exterior faces meet each other and so do the interior faces, and every
//!   layer is cut by the line joining those two corners.
//! - An end inside another wall butts against it (a T-junction): each layer
//!   goes in through the other wall's layers of lower priority and stops at
//!   its own line; the other wall loses that part (see [`joins`]).
//! - An end against a curved wall, or a curved wall's end inside another
//!   wall, stops at that wall's face (no layer goes through).
//! - Any other end (free, three or more walls) is cut square.
//!
//! Curved walls follow circular arcs ([`Curve`]): their faces and layer
//! boundaries are drawn as polylines close enough to the arc
//! ([`archisim_core::CURVE_TOLERANCE`]); [`WallGeometry::outline_path`]
//! gives the exact arcs for the IFC.
//!
//! Doors and windows are resolved by [`opening_geometry`]: their span along
//! the wall, the faces of their frame and their height. [`clip_span`] cuts a
//! wall (or one of its layers) into the solid pieces between openings.

pub mod anchors;
pub mod cover;
pub mod crossroads;
pub mod csg;
pub mod elevators;
pub mod env_buildings;
pub mod facade;
pub mod forms;
pub mod growth;
pub mod index;
pub mod joins;
pub mod junctions;
pub mod pools;
pub mod rebar;
pub mod roof_parts;
pub mod room;
pub mod sculpt;
pub mod shadow;
pub mod slice;
pub mod solid;
pub mod stairs;
pub mod sweeps;

use archisim_core::{
    Arc, Hand, LayerFunction, LevelId, MaterialId, Model, OpeningId, OpeningKind, Point2, Vec2,
    Wall, WallId, WallSide, CURVE_TOLERANCE, EPSILON,
};

/// An infinite line through `p` with direction `d` (not necessarily unit).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Line {
    pub p: Point2,
    pub d: Vec2,
}

impl Line {
    pub fn new(p: Point2, d: Vec2) -> Self {
        Self { p, d }
    }

    /// Intersection point, or `None` if the lines are parallel.
    pub fn intersect(&self, other: &Line) -> Option<Point2> {
        let den = self.d.cross(other.d);
        if den.abs() < 1e-12 {
            return None;
        }
        let t = (other.p - self.p).cross(other.d) / den;
        Some(self.p + self.d * t)
    }
}

/// A wall face or layer boundary in plan: a straight line, or the circle
/// a curved wall's face follows.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Curve {
    Line(Line),
    Circle { centre: Point2, radius: f64 },
}

/// How far apart a line and a circle (or two circles) may be and still be
/// taken as touching, in metres.
const TOUCH: f64 = 1e-4;

impl Curve {
    /// The intersection nearest to `near`, if they meet. Curves that just
    /// touch (a straight wall running on tangent into a curved one) meet at
    /// the point of contact.
    pub fn intersect_near(&self, other: &Curve, near: Point2) -> Option<Point2> {
        let found = match (self, other) {
            (Curve::Line(a), Curve::Line(b)) => return a.intersect(b),
            (Curve::Line(l), Curve::Circle { centre, radius })
            | (Curve::Circle { centre, radius }, Curve::Line(l)) => {
                line_circle(*l, *centre, *radius)
            }
            (
                Curve::Circle {
                    centre: c1,
                    radius: r1,
                },
                Curve::Circle {
                    centre: c2,
                    radius: r2,
                },
            ) => circle_circle(*c1, *r1, *c2, *r2),
        };
        found
            .into_iter()
            .min_by(|a, b| a.distance(near).total_cmp(&b.distance(near)))
    }
}

fn line_circle(l: Line, c: Point2, r: f64) -> Vec<Point2> {
    let Some(u) = l.d.normalized() else {
        return Vec::new();
    };
    let foot = l.p + u * (c - l.p).dot(u);
    let d = foot.distance(c);
    if d > r + TOUCH {
        return Vec::new();
    }
    let h = (r * r - d * d).max(0.0).sqrt();
    vec![foot + u * h, foot - u * h]
}

fn circle_circle(c1: Point2, r1: f64, c2: Point2, r2: f64) -> Vec<Point2> {
    let d = c1.distance(c2);
    if d < 1e-9 || d > r1 + r2 + TOUCH || d < (r1 - r2).abs() - TOUCH {
        return Vec::new();
    }
    let a = (r1 * r1 - r2 * r2 + d * d) / (2.0 * d);
    let h = (r1 * r1 - a * a).max(0.0).sqrt();
    let u = (c2 - c1) * (1.0 / d);
    let p = c1 + u * a;
    vec![p + u.perp_left() * h, p - u.perp_left() * h]
}

/// The line or circle `offset` to the left of a wall's drawn line.
pub fn wall_curve(wall: &Wall, offset: f64) -> Option<Curve> {
    Some(match wall.arc() {
        Some(a) => Curve::Circle {
            centre: a.centre,
            radius: a.radius_at(offset),
        },
        None => {
            let dir = wall.end - wall.start;
            let n = dir.normalized()?.perp_left();
            Curve::Line(Line::new(wall.start + n * offset, dir))
        }
    })
}

/// Direction of travel (start → end) of a wall's drawn line at one end.
fn end_tangent(wall: &Wall, end: End) -> Option<Vec2> {
    match wall.arc() {
        Some(a) => Some(a.tangent(if end == End::Start { 0.0 } else { 1.0 })),
        None => (wall.end - wall.start).normalized(),
    }
}

/// The point `offset` to the left of a wall's drawn line at one end.
fn end_point(wall: &Wall, end: End, offset: f64) -> Point2 {
    wall.point_at(if end == End::Start { 0.0 } else { 1.0 }, offset)
}

/// One piece of a wall's outline, for exporting it exactly.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PathSeg {
    /// A straight piece from the first point to the second.
    Line(Point2, Point2),
    /// A circular arc from the first point through the second to the third.
    Arc(Point2, Point2, Point2),
}

/// Signed area (counter-clockwise positive) of a closed outline made of
/// straight pieces and arcs: exact, not that of the drawn polyline.
pub fn path_area(path: &[PathSeg]) -> f64 {
    let mut area = 0.0;
    for seg in path {
        match *seg {
            PathSeg::Line(a, b) => area += a.cross(b) / 2.0,
            PathSeg::Arc(a, b, c) => {
                area += a.cross(c) / 2.0;
                let chord = c - a;
                let len = chord.length();
                let (ba, bc) = (a - b, c - b);
                let cross = ba.cross(bc);
                if cross.abs() < 1e-15 || len < 1e-12 {
                    continue;
                }
                // The angle at the middle point is half of what the rest
                // of the circle sweeps (inscribed angle).
                let beta = cross.abs().atan2(ba.dot(bc));
                let phi = 2.0 * (std::f64::consts::PI - beta);
                let r = len / (2.0 * beta.sin());
                let segment = r * r / 2.0 * (phi - phi.sin());
                // Bulging to the right of a → c adds to the area.
                area += if chord.cross(b - a) < 0.0 {
                    segment
                } else {
                    -segment
                };
            }
        }
    }
    area
}

/// Number of straight pieces for every face and layer boundary of a
/// curved wall: the same for all, so the layers share their vertices.
fn arc_segments(arc: &Arc, ext: f64, int: f64) -> usize {
    arc.segments(ext, CURVE_TOLERANCE)
        .max(arc.segments(int, CURVE_TOLERANCE))
}

/// The points of the curve `offset` from the arc strictly between the
/// points `from` and `to` (which lie on it), in that order.
fn arc_points(arc: &Arc, nseg: usize, from: Point2, to: Point2, offset: f64) -> Vec<Point2> {
    let (fa, fb) = (arc.project(from).0, arc.project(to).0);
    let margin = 0.25 / nseg as f64;
    let mut pts: Vec<Point2> = (1..nseg)
        .map(|i| i as f64 / nseg as f64)
        .filter(|f| *f > fa.min(fb) + margin && *f < fa.max(fb) - margin)
        .map(|f| arc.point(f, offset))
        .collect();
    if fa > fb {
        pts.reverse();
    }
    pts
}

pub use archisim_core::signed_area;
pub use index::WallIndex;
pub use joins::{Notch, NotchRegion, TeeEnd};
pub use room::{room_outline, Contour, RoomError};

/// Returns the polygon in counter-clockwise order.
pub fn ensure_ccw(mut poly: Vec<Point2>) -> Vec<Point2> {
    if signed_area(&poly) < 0.0 {
        poly.reverse();
    }
    poly
}

/// How one end of a wall is cut.
#[derive(Debug, Clone, PartialEq)]
pub enum EndCut {
    /// Perpendicular to the axis, at the end point.
    Square,
    /// Along the given line (a mitre with a connected wall).
    Along(Line),
    /// Butting against another wall (a T-junction): each layer stops at its
    /// own line, parallel to the other wall (see [`joins`]).
    Tee(TeeEnd),
    /// Stopping at the face of another wall, every layer alike: a wall
    /// butting against a curved one, or a curved wall's end inside another.
    Face(Curve),
}

impl EndCut {
    /// The line where layer `layer` ends, if not square.
    pub fn line(&self, layer: usize) -> Option<Line> {
        match self {
            EndCut::Square => None,
            EndCut::Along(l) => Some(*l),
            EndCut::Tee(t) => Some(t.layer_line(layer)),
            EndCut::Face(Curve::Line(l)) => Some(*l),
            EndCut::Face(Curve::Circle { .. }) => None,
        }
    }

    /// The line or circle where layer `layer` ends, if not square.
    pub fn curve(&self, layer: usize) -> Option<Curve> {
        match self {
            EndCut::Square => None,
            EndCut::Along(l) => Some(Curve::Line(*l)),
            EndCut::Tee(t) => Some(Curve::Line(t.layer_line(layer))),
            EndCut::Face(c) => Some(*c),
        }
    }

    /// Whether the end is closed by another wall (no end face of its own).
    pub fn is_mitre(&self) -> bool {
        matches!(self, EndCut::Along(_))
    }
}

/// The cut polygon of one material layer.
#[derive(Debug, Clone, PartialEq)]
pub struct LayerBand {
    pub material: MaterialId,
    pub function: LayerFunction,
    /// Closed polygon, counter-clockwise, without repeating the first point.
    /// Parts taken by other walls butting against this one are not
    /// removed here: see [`WallGeometry::layer_pieces`].
    pub polygon: Vec<Point2>,
}

/// Plan geometry of one wall.
#[derive(Debug, Clone, PartialEq)]
pub struct WallGeometry {
    pub wall: WallId,
    /// The ends of the drawn line (reference line) of the wall.
    pub axis: (Point2, Point2),
    /// The arc the drawn line follows, for a curved wall.
    pub arc: Option<Arc>,
    /// Offset of the exterior face from the drawn line (left positive).
    pub exterior_offset: f64,
    /// Offset of the interior face from the drawn line (left positive).
    pub interior_offset: f64,
    pub start_cut: EndCut,
    pub end_cut: EndCut,
    /// Outline of the whole wall, counter-clockwise, with the parts taken
    /// by walls butting against it removed.
    pub outline: Vec<Point2>,
    /// One band per layer, from exterior to interior.
    pub layers: Vec<LayerBand>,
    /// Where other walls butt into this one.
    pub notches: Vec<Notch>,
    /// The two ends as point lists from the exterior face to the interior
    /// one (one point per layer boundary at a T-junction).
    ends: (Vec<Point2>, Vec<Point2>),
}

impl WallGeometry {
    /// The pieces of layer `li` left by the walls butting into this one at
    /// height `z` above the level (`None`: wherever they are).
    pub fn layer_pieces(&self, li: usize, z: Option<f64>) -> Vec<Vec<Point2>> {
        let mut pieces = vec![self.layers[li].polygon.clone()];
        for notch in self.notches.iter().filter(|n| notch_at(n, z)) {
            for region in notch.regions.iter().filter(|r| r.layers.contains(&li)) {
                let (f, lo, hi) = region.strip();
                pieces = pieces
                    .iter()
                    .flat_map(|poly| {
                        [
                            clip_half_plane(poly, |p| lo - f(p)),
                            clip_half_plane(poly, |p| f(p) - hi),
                        ]
                    })
                    .filter(|p| p.len() >= 3 && signed_area(p).abs() > EPSILON * EPSILON)
                    .collect();
            }
        }
        pieces
    }

    /// The outline at height `z` above the level: only the walls butting in
    /// at that height take their part.
    pub fn outline_at(&self, z: Option<f64>) -> Vec<Point2> {
        build_outline(self, z)
    }

    /// The points of each end, from the exterior face to the interior one
    /// (one per layer boundary at a T-junction): the corners of the wall.
    pub fn end_points(&self) -> (&[Point2], &[Point2]) {
        (&self.ends.0, &self.ends.1)
    }

    /// The outline of a curved wall as straight pieces and exact arcs
    /// (`None` for a straight wall, whose outline is already exact).
    pub fn outline_path(&self) -> Option<Vec<PathSeg>> {
        let arc = self.arc.as_ref()?;
        let (s, e) = (&self.ends.0, &self.ends.1);
        let (s_ext, s_int) = (*s.first()?, *s.last()?);
        let (e_ext, e_int) = (*e.first()?, *e.last()?);
        let mid = |a: Point2, b: Point2, off: f64| {
            let (fa, fb) = (arc.project(a).0, arc.project(b).0);
            arc.point((fa + fb) / 2.0, off)
        };
        let mut path = Vec::new();
        for w in s.windows(2) {
            if !w[0].approx_eq(w[1]) {
                path.push(PathSeg::Line(w[0], w[1]));
            }
        }
        path.push(PathSeg::Arc(
            s_int,
            mid(s_int, e_int, self.interior_offset),
            e_int,
        ));
        let back: Vec<Point2> = e.iter().rev().copied().collect();
        for w in back.windows(2) {
            if !w[0].approx_eq(w[1]) {
                path.push(PathSeg::Line(w[0], w[1]));
            }
        }
        path.push(PathSeg::Arc(
            e_ext,
            mid(e_ext, s_ext, self.exterior_offset),
            s_ext,
        ));
        Some(path)
    }

    /// Heights above the level where the outline changes (the bottom and
    /// top of the walls butting in), sorted.
    pub fn notch_heights(&self) -> Vec<f64> {
        let mut v: Vec<f64> = self.notches.iter().flat_map(|n| [n.z.0, n.z.1]).collect();
        v.sort_by(f64::total_cmp);
        v.dedup_by(|a, b| (*a - *b).abs() < EPSILON);
        v
    }
}

fn notch_at(n: &Notch, z: Option<f64>) -> bool {
    z.is_none_or(|z| z > n.z.0 - EPSILON && z < n.z.1 + EPSILON)
}

/// Which end of a wall touches a junction.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum End {
    Start,
    End,
}

/// Geometry of every wall on a level.
pub fn wall_geometries(model: &Model, level: LevelId) -> Vec<WallGeometry> {
    let idx = WallIndex::new(model);
    model
        .walls_on_level(level)
        .filter_map(|w| wall_geometry_in(&idx, w.id))
        .collect()
}

/// Geometry of one wall, with joins against walls on the same level.
pub fn wall_geometry(model: &Model, id: WallId) -> Option<WallGeometry> {
    wall_geometry_in(&WallIndex::new(model), id)
}

/// [`wall_geometry`] with the walls around found through an index (build
/// one for many walls).
pub fn wall_geometry_in(idx: &WallIndex, id: WallId) -> Option<WallGeometry> {
    let model = idx.model;
    let wall = model.wall(id)?;
    let wtype = model.wall_type(wall.wall_type)?;
    let (ext, int) = wall.face_offsets(wtype.thickness());

    let start_cut = end_cut(idx, wall, End::Start);
    let end_cut_ = end_cut(idx, wall, End::End);

    (wall.end - wall.start).normalized()?;
    let arc = wall.arc();
    let point_at = |offset: f64, end: End, layer: usize| -> Point2 {
        let cut = match end {
            End::Start => &start_cut,
            End::End => &end_cut_,
        };
        let square = end_point(wall, end, offset);
        match (cut.curve(layer), wall_curve(wall, offset)) {
            (Some(c), Some(own)) => own.intersect_near(&c, square).unwrap_or(square),
            _ => square,
        }
    };
    let nseg = arc.map_or(0, |a| arc_segments(&a, ext, int));

    let mut tops = vec![ext];
    for layer in &wtype.layers {
        let last = *tops.last()?;
        tops.push(last - layer.thickness);
    }
    let layers: Vec<LayerBand> = wtype
        .layers
        .iter()
        .enumerate()
        .map(|(li, layer)| {
            let (high, low) = (tops[li], tops[li + 1]);
            let (ls, le) = (point_at(low, End::Start, li), point_at(low, End::End, li));
            let (hs, he) = (point_at(high, End::Start, li), point_at(high, End::End, li));
            let mut polygon = vec![ls];
            if let Some(a) = &arc {
                polygon.extend(arc_points(a, nseg, ls, le, low));
            }
            polygon.extend([le, he]);
            if let Some(a) = &arc {
                polygon.extend(arc_points(a, nseg, he, hs, high));
            }
            polygon.push(hs);
            LayerBand {
                material: layer.material,
                function: layer.function,
                polygon: ensure_ccw(polygon),
            }
        })
        .collect();
    let profile = |end: End| -> Vec<Point2> {
        let tee = match end {
            End::Start => matches!(start_cut, EndCut::Tee(_)),
            End::End => matches!(end_cut_, EndCut::Tee(_)),
        };
        if !tee {
            let last = wtype.layers.len().saturating_sub(1);
            return vec![point_at(ext, end, 0), point_at(int, end, last)];
        }
        let mut v: Vec<Point2> = Vec::new();
        for li in 0..wtype.layers.len() {
            for off in [tops[li], tops[li + 1]] {
                let p = point_at(off, end, li);
                if v.last().is_none_or(|q| !q.approx_eq(p)) {
                    v.push(p);
                }
            }
        }
        v
    };

    let mut g = WallGeometry {
        wall: id,
        axis: (wall.start, wall.end),
        arc,
        exterior_offset: ext,
        interior_offset: int,
        ends: (profile(End::Start), profile(End::End)),
        start_cut,
        end_cut: end_cut_,
        outline: Vec::new(),
        layers,
        notches: joins::notches_in(idx, wall),
    };
    g.outline = build_outline(&g, None);
    Some(g)
}

/// The outline: start end (exterior to interior), the interior face with
/// the notches on it, the end (interior to exterior), the exterior face
/// back with its notches.
fn build_outline(g: &WallGeometry, z: Option<f64>) -> Vec<Point2> {
    if let Some(arc) = &g.arc {
        // Curved walls take no notches: other walls stop at their faces.
        let nseg = arc_segments(arc, g.exterior_offset, g.interior_offset);
        let (s, e) = (&g.ends.0, &g.ends.1);
        let (Some(s_int), Some(e_int), Some(s_ext), Some(e_ext)) =
            (s.last(), e.last(), s.first(), e.first())
        else {
            return Vec::new();
        };
        let mut ring: Vec<Point2> = s.clone();
        ring.extend(arc_points(arc, nseg, *s_int, *e_int, g.interior_offset));
        ring.extend(e.iter().rev());
        ring.extend(arc_points(arc, nseg, *e_ext, *s_ext, g.exterior_offset));
        ring.dedup_by(|a, b| a.approx_eq(*b));
        return ensure_ccw(ring);
    }
    let (a, b) = g.axis;
    let Some(u) = (b - a).normalized() else {
        return Vec::new();
    };
    let n = u.perp_left();
    let centre = (g.exterior_offset + g.interior_offset) / 2.0;
    let on_face = |face: f64, ascending: bool| -> Vec<Point2> {
        let mut found: Vec<(f64, Vec<Point2>)> = g
            .notches
            .iter()
            .filter(|nt| notch_at(nt, z) && (nt.face_offset - face).abs() < 1e-9)
            .map(|nt| {
                let pts = notch_profile(nt, a, u, n, centre);
                let t = pts.first().map_or(0.0, |p| (*p - a).dot(u));
                (t, pts)
            })
            .collect();
        found.sort_by(|x, y| x.0.total_cmp(&y.0));
        let mut pts: Vec<Point2> = found.into_iter().flat_map(|(_, p)| p).collect();
        if !ascending {
            pts.reverse();
        }
        pts
    };
    let mut ring: Vec<Point2> = Vec::new();
    let mut push = |p: Point2| {
        if ring.last().is_none_or(|q: &Point2| !q.approx_eq(p)) {
            ring.push(p);
        }
    };
    for p in &g.ends.0 {
        push(*p);
    }
    for p in on_face(g.interior_offset, true) {
        push(p);
    }
    for p in g.ends.1.iter().rev() {
        push(*p);
    }
    for p in on_face(g.exterior_offset, false) {
        push(p);
    }
    if ring.len() > 1 && ring[0].approx_eq(ring[ring.len() - 1]) {
        ring.pop();
    }
    ensure_ccw(ring)
}

/// The notch as a path along the host's face, in increasing `t`: down each
/// region's first side, along its bottom, and up its second side, joining
/// neighbouring regions along their shared side.
fn notch_profile(nt: &Notch, a: Point2, u: Vec2, n: Vec2, centre: f64) -> Vec<Point2> {
    let face = Line::new(a + n * nt.face_offset, u);
    let inward = if centre > nt.face_offset { n } else { -n };
    let at = |side: Line, depth: f64| {
        Line::new(face.p + inward * depth, face.d)
            .intersect(&side)
            .unwrap_or(side.p)
    };
    let t = |p: Point2| (p - a).dot(u);
    let mut regions: Vec<(Line, Line, f64)> = nt
        .regions
        .iter()
        .map(|r| {
            let (s1, s2) = r.sides;
            if t(at(s1, 0.0)) <= t(at(s2, 0.0)) {
                (s1, s2, r.depth)
            } else {
                (s2, s1, r.depth)
            }
        })
        .collect();
    regions.sort_by(|x, y| t(at(x.0, 0.0)).total_cmp(&t(at(y.0, 0.0))));
    let mut pts = Vec::new();
    for (k, (s1, s2, depth)) in regions.iter().enumerate() {
        let joined_before = k > 0 && at(regions[k - 1].1, 0.0).approx_eq(at(*s1, 0.0));
        let joined_after =
            k + 1 < regions.len() && at(*s2, 0.0).approx_eq(at(regions[k + 1].0, 0.0));
        if !joined_before {
            pts.push(at(*s1, 0.0));
        }
        pts.push(at(*s1, *depth));
        pts.push(at(*s2, *depth));
        if !joined_after {
            pts.push(at(*s2, 0.0));
        }
    }
    pts
}

/// The two face lines of a wall, as (left, right) when walking in direction
/// `towards`: `true` walks start → end, `false` walks end → start.
fn face_curves(model: &Model, wall: &Wall, forward: bool) -> Option<(Curve, Curve)> {
    let t = model.wall_type(wall.wall_type)?.thickness();
    let (ext, int) = wall.face_offsets(t);
    let (e, i) = (wall_curve(wall, ext)?, wall_curve(wall, int)?);
    Some(if forward { (e, i) } else { (i, e) })
}

fn faces(model: &Model, wall: &Wall, forward: bool) -> Option<(Line, Line)> {
    let t = model.wall_type(wall.wall_type)?.thickness();
    let (ext, int) = wall.face_offsets(t);
    let dir = wall.end - wall.start;
    let n = dir.normalized()?.perp_left();
    let ext_line = Line::new(wall.start + n * ext, dir);
    let int_line = Line::new(wall.start + n * int, dir);
    // The exterior face is on the left when walking start → end.
    Some(if forward {
        (ext_line, int_line)
    } else {
        (int_line, ext_line)
    })
}

fn end_cut(idx: &WallIndex, wall: &Wall, end: End) -> EndCut {
    let model = idx.model;
    let p = match end {
        End::Start => wall.start,
        End::End => wall.end,
    };
    // «No permitir unión» at this end (on this wall or on a wall that
    // ends here): a free, square end; the walls only touch.
    if !wall.joins_at(p) {
        return EndCut::Square;
    }
    let near = idx.at_end(wall.level, p);
    if near.iter().any(|o| o.id != wall.id && !o.joins_at(p)) {
        return EndCut::Square;
    }
    let mut others = near.into_iter().filter_map(|o| {
        if o.id == wall.id {
            None
        } else if o.start.approx_eq(p) {
            Some((o, End::Start))
        } else if o.end.approx_eq(p) {
            Some((o, End::End))
        } else {
            None
        }
    });
    let (other, other_end) = match (others.next(), others.next()) {
        (Some(one), None) => one,
        (None, _) => {
            return joins::tee_end_in(idx, wall, end == End::Start)
                .map(EndCut::Tee)
                .or_else(|| face_stop(idx, wall, end).map(EndCut::Face))
                .unwrap_or(EndCut::Square)
        }
        _ => return EndCut::Square,
    };
    mitre(model, wall, end, other, other_end).unwrap_or(EndCut::Square)
}

/// The face of another wall that an end of `wall` stops at, when a curved
/// wall is involved (straight walls butt with [`joins::tee_end`]): the end
/// lies inside the other wall, away from its ends, and comes in at a clear
/// angle.
fn face_stop(idx: &WallIndex, wall: &Wall, end: End) -> Option<Curve> {
    let model = idx.model;
    let p = end_point(wall, end, 0.0);
    let t = end_tangent(wall, end)?;
    // Into the wall, away from the end.
    let away = if end == End::Start { t } else { -t };
    idx.near(wall.level, p, 0.0)
        .into_iter()
        .filter(|h| h.id != wall.id && (h.is_curved() || wall.is_curved()))
        .find_map(|h| {
            let th = model.wall_type(h.wall_type)?.thickness();
            let (ext, int) = h.face_offsets(th);
            let (lo, hi) = (ext.min(int), ext.max(int));
            let (inside, off, normal) = match h.arc() {
                Some(a) => {
                    let (f, off) = a.project(p);
                    let s = f * a.length();
                    let inside = s > EPSILON && s < a.length() - EPSILON;
                    (inside, off, a.tangent(f).perp_left())
                }
                None => {
                    let u = (h.end - h.start).normalized()?;
                    let s = (p - h.start).dot(u);
                    let inside = s > EPSILON && s < h.length() - EPSILON;
                    (inside, (p - h.start).dot(u.perp_left()), u.perp_left())
                }
            };
            if !inside || off < lo - 1e-6 || off > hi + 1e-6 {
                return None;
            }
            let side = away.dot(normal);
            if side.abs() < 0.26 {
                return None;
            }
            wall_curve(h, if side > 0.0 { hi } else { lo })
        })
}

/// Mitre between `wall` arriving at the junction and `other` leaving it.
fn mitre(model: &Model, wall: &Wall, end: End, other: &Wall, other_end: End) -> Option<EndCut> {
    if wall.is_curved() || other.is_curved() {
        return curved_mitre(model, wall, end, other, other_end);
    }
    // Walk `wall` towards the junction and `other` away from it: the left
    // faces of both then lie on the same side of the corner.
    let (w_left, w_right) = faces(model, wall, end == End::End)?;
    let (o_left, o_right) = faces(model, other, other_end == End::Start)?;
    let p = match end {
        End::Start => wall.start,
        End::End => wall.end,
    };

    let (l, r) = match (w_left.intersect(&o_left), w_right.intersect(&o_right)) {
        (Some(l), Some(r)) => (l, r),
        // Collinear walls: cut perpendicular to the axis at the junction.
        _ => {
            let n = (wall.end - wall.start).normalized()?.perp_left();
            return Some(EndCut::Along(Line::new(p, n)));
        }
    };

    // Very sharp angles push the mitre far away; fall back to a square cut.
    let tw = model.wall_type(wall.wall_type)?.thickness();
    let to = model.wall_type(other.wall_type)?.thickness();
    let limit = 4.0 * (tw + to);
    if l.distance(p) > limit || r.distance(p) > limit || l.approx_eq(r) {
        return None;
    }
    if (r - l).length() < EPSILON {
        return None;
    }
    Some(EndCut::Along(Line::new(l, r - l)))
}

/// [`mitre`] when one of the walls (or both) is curved: the faces meet as
/// curves, and the cut is the straight line through both corners.
fn curved_mitre(
    model: &Model,
    wall: &Wall,
    end: End,
    other: &Wall,
    other_end: End,
) -> Option<EndCut> {
    let (w_left, w_right) = face_curves(model, wall, end == End::End)?;
    let (o_left, o_right) = face_curves(model, other, other_end == End::Start)?;
    let p = end_point(wall, end, 0.0);
    let tw = model.wall_type(wall.wall_type)?.thickness();
    let to = model.wall_type(other.wall_type)?.thickness();
    let across = Line::new(p, end_tangent(wall, end)?.perp_left());
    let (l, r) = match (
        w_left.intersect_near(&o_left, p),
        w_right.intersect_near(&o_right, p),
    ) {
        (Some(l), Some(r)) if !l.approx_eq(r) => (l, r),
        // Running on smoothly (two halves of a circle, a straight wall on
        // tangent): cut across at the junction.
        _ => return Some(EndCut::Along(across)),
    };
    let limit = 4.0 * (tw + to);
    if l.distance(p) > limit || r.distance(p) > limit {
        // Tangent curves meet far away, or nowhere near: across if they
        // run on smoothly, square otherwise.
        let (ta, tb) = (end_tangent(wall, end)?, end_tangent(other, other_end)?);
        let tb = if other_end == End::Start { tb } else { -tb };
        let ta = if end == End::End { ta } else { -ta };
        return (ta.dot(tb) > 0.999).then_some(EndCut::Along(across));
    }
    Some(EndCut::Along(Line::new(l, r - l)))
}

/// A door or window resolved against its host wall.
///
/// Positions across the wall are signed offsets from the wall's drawn line,
/// positive towards the exterior face (the left of the axis), as in
/// [`WallGeometry`]. Heights are measured from the level.
#[derive(Debug, Clone, PartialEq)]
pub struct OpeningGeometry {
    pub opening: OpeningId,
    pub wall: WallId,
    pub kind: OpeningKind,
    /// Start of the wall's drawn line, origin of the `t` coordinates (in
    /// a curved wall, the point of the tangent at the opening's centre as
    /// far back as the centre is along the arc).
    pub origin: Point2,
    /// Unit vector along the wall, start → end (in a curved wall, the
    /// tangent at the opening's centre).
    pub u: Vec2,
    /// Unit normal towards the exterior face.
    pub n: Vec2,
    /// Start and end of the opening along the drawn line.
    pub t0: f64,
    pub t1: f64,
    pub exterior_offset: f64,
    pub interior_offset: f64,
    /// Offsets of the exterior and interior faces of the frame.
    pub frame_exterior: f64,
    pub frame_interior: f64,
    /// Visible width of the frame profile.
    pub frame_width: f64,
    /// Bottom and top of the opening above the level.
    pub sill: f64,
    pub head: f64,
    pub opens_to: WallSide,
    pub hand: Hand,
    /// A corner window: it opens the wall through its start (`Some(false)`)
    /// or its end (`Some(true)`), past its jamb there.
    pub corner: Option<bool>,
    /// The shape of the rough opening in elevation (square, arched, an
    /// oculus), within its box `t0`..`t1` by `sill`..`head`.
    pub shape: archisim_core::OpeningShape,
}

impl OpeningGeometry {
    pub fn width(&self) -> f64 {
        self.t1 - self.t0
    }

    /// Where the opening cuts its wall along it: its span, or, on the
    /// corner side of a corner window, on to the end of the wall (its
    /// joint with the next one included).
    pub fn cut_span(&self) -> (f64, f64) {
        match self.corner {
            Some(false) => (-1e6, self.t1),
            Some(true) => (self.t0, 1e6),
            None => (self.t0, self.t1),
        }
    }

    pub fn height(&self) -> f64 {
        self.head - self.sill
    }

    /// The area of the opening in elevation: its box, or less under an
    /// arch or round an oculus.
    pub fn face_area(&self) -> f64 {
        let (w, h) = (self.width(), self.height());
        if self.shape.is_rect() {
            w * h
        } else {
            archisim_core::shape::area(&self.shape.polygon(w, h)).abs()
        }
    }

    /// The plan point at distance `t` along the wall and `offset` across it.
    pub fn point(&self, t: f64, offset: f64) -> Point2 {
        self.origin + self.u * t + self.n * offset
    }

    /// Offset of the wall face on the side the leaves open to.
    pub fn swing_face(&self) -> f64 {
        match self.opens_to {
            WallSide::Exterior => self.exterior_offset,
            WallSide::Interior => self.interior_offset,
        }
    }

    /// Offset of the frame face on the side the leaves open to.
    pub fn swing_frame_face(&self) -> f64 {
        match self.opens_to {
            WallSide::Exterior => self.frame_exterior,
            WallSide::Interior => self.frame_interior,
        }
    }

    /// Unit vector pointing out of the wall on the side the leaves open to.
    pub fn swing_dir(&self) -> Vec2 {
        match self.opens_to {
            WallSide::Exterior => self.n,
            WallSide::Interior => -self.n,
        }
    }

    /// Whether the hinges (or the parked sliding leaf) are at `t0`.
    ///
    /// The hand is defined looking at the wall from the side the door opens
    /// to; that viewer's left points along `perp_left(-swing_dir)`.
    pub fn hinge_at_start(&self) -> bool {
        let viewer_left = (-self.swing_dir()).perp_left();
        let left_is_start = viewer_left.dot(self.u) < 0.0;
        left_is_start == (self.hand == Hand::Left)
    }

    /// The rough opening in plan: the span of the opening across the whole
    /// wall thickness, counter-clockwise.
    pub fn footprint(&self) -> Vec<Point2> {
        ensure_ccw(vec![
            self.point(self.t0, self.interior_offset),
            self.point(self.t1, self.interior_offset),
            self.point(self.t1, self.exterior_offset),
            self.point(self.t0, self.exterior_offset),
        ])
    }
}

/// The points to dimension one face of a wall with its doors and windows
/// (ArchiCAD's automatic dimensioning of a wall): the ends of that face
/// (with the mitred corners) and the jambs of every opening, in order along
/// the wall.
#[derive(Debug, Clone, PartialEq)]
pub struct FaceChain {
    pub points: Vec<Point2>,
    /// Unit vector along the wall, start → end.
    pub direction: Vec2,
    /// Unit vector out of the wall through that face.
    pub outward: Vec2,
}

/// [`FaceChain`] of the exterior (`true`) or interior face of a wall.
pub fn wall_face_chain(model: &Model, wall: WallId, exterior: bool) -> Option<FaceChain> {
    let g = wall_geometry(model, wall)?;
    if g.arc.is_some() {
        // A curved face is not measured with straight dimensions.
        return None;
    }
    let (a, b) = g.axis;
    let u = (b - a).normalized()?;
    let n = u.perp_left();
    let (face, other) = if exterior {
        (g.exterior_offset, g.interior_offset)
    } else {
        (g.interior_offset, g.exterior_offset)
    };
    let outward = if face >= other { n } else { -n };
    let on_face: Vec<f64> = g
        .outline
        .iter()
        .filter(|p| ((**p - a).dot(n) - face).abs() < 1e-6)
        .map(|p| (*p - a).dot(u))
        .collect();
    let lo = on_face.iter().copied().fold(f64::INFINITY, f64::min);
    let hi = on_face.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    if !lo.is_finite() || !hi.is_finite() {
        return None;
    }
    let mut ts = vec![lo, hi];
    for o in openings_in_wall(model, wall) {
        ts.push(o.t0);
        ts.push(o.t1);
    }
    ts.sort_by(f64::total_cmp);
    ts.dedup_by(|x, y| (*x - *y).abs() < EPSILON);
    Some(FaceChain {
        points: ts.iter().map(|t| a + u * *t + n * face).collect(),
        direction: u,
        outward,
    })
}

/// Geometry of one door or window, or `None` if it or its wall is missing.
pub fn opening_geometry(model: &Model, id: OpeningId) -> Option<OpeningGeometry> {
    let o = model.opening(id)?;
    let otype = model.opening_type(o.opening_type)?;
    let wall = model.wall(o.wall)?;
    let thickness = model.wall_type(wall.wall_type)?.thickness();
    let (ext, int) = wall.face_offsets(thickness);
    let setback = o.frame_setback(thickness, otype);
    let (t0, t1) = o.span(otype.width);
    // In a curved wall, a straight door or window square to the wall at
    // its centre: its frame is the tangent there, with `t` measured as
    // along the arc at its centre.
    let (origin, u) = match wall.is_curved() {
        true => {
            let mid = (t0 + t1) / 2.0;
            let (c, u) = wall.opening_frame(mid, otype.width)?;
            (c - u * mid, u)
        }
        false => (wall.start, (wall.end - wall.start).normalized()?),
    };
    Some(OpeningGeometry {
        opening: id,
        wall: wall.id,
        kind: otype.kind,
        origin,
        u,
        n: u.perp_left(),
        t0,
        t1,
        exterior_offset: ext,
        interior_offset: int,
        frame_exterior: ext - setback,
        frame_interior: ext - setback - otype.frame_depth,
        frame_width: otype.frame_width,
        sill: o.sill,
        head: o.sill + otype.height,
        opens_to: o.opens_to,
        hand: o.hand,
        corner: o.corner_end(),
        shape: otype.shape,
    })
}

/// Geometry of every door and window in a wall, sorted along it.
pub fn openings_in_wall(model: &Model, wall: WallId) -> Vec<OpeningGeometry> {
    let mut v: Vec<_> = model
        .openings_in_wall(wall)
        .filter_map(|o| opening_geometry(model, o.id))
        .collect();
    v.sort_by(|a, b| a.t0.total_cmp(&b.t0));
    v
}

/// Clips a convex polygon to the part between `lo` and `hi` along the
/// direction `u` from `origin` (either bound may be open). Returns an empty
/// vector if nothing is left.
pub fn clip_span(
    poly: &[Point2],
    origin: Point2,
    u: Vec2,
    lo: Option<f64>,
    hi: Option<f64>,
) -> Vec<Point2> {
    let mut out = poly.to_vec();
    if let Some(lo) = lo {
        out = clip_half_plane(&out, |p| (p - origin).dot(u) - lo);
    }
    if let Some(hi) = hi {
        out = clip_half_plane(&out, |p| hi - (p - origin).dot(u));
    }
    if out.len() < 3 || signed_area(&out).abs() < EPSILON * EPSILON {
        Vec::new()
    } else {
        out
    }
}

/// Sutherland–Hodgman: keeps the part of a polygon where `side(p) >= 0`.
fn clip_half_plane(poly: &[Point2], side: impl Fn(Point2) -> f64) -> Vec<Point2> {
    let mut out: Vec<Point2> = Vec::with_capacity(poly.len() + 2);
    let mut push = |p: Point2| {
        if out.last().is_none_or(|q: &Point2| !q.approx_eq(p)) {
            out.push(p);
        }
    };
    for i in 0..poly.len() {
        let (a, b) = (poly[i], poly[(i + 1) % poly.len()]);
        let (sa, sb) = (side(a), side(b));
        if sa >= 0.0 {
            push(a);
        }
        if (sa >= 0.0) != (sb >= 0.0) {
            push(a + (b - a) * (sa / (sa - sb)));
        }
    }
    if out.len() > 1 && out[0].approx_eq(out[out.len() - 1]) {
        out.pop();
    }
    out
}

/// [`clip_span`] for a curved wall: the part of `poly` between `lo` and
/// `hi` metres along `arc` (either open), cut square to the arc (along
/// the radius). The polygon's edges must be short (as the curve's).
pub fn clip_arc_span(
    poly: &[Point2],
    arc: &archisim_core::Arc,
    lo: Option<f64>,
    hi: Option<f64>,
) -> Vec<Point2> {
    let len = arc.length().max(EPSILON);
    let along = |p: Point2| arc.project(p).0 * len;
    let mut out = poly.to_vec();
    if let Some(lo) = lo {
        out = clip_half_plane(&out, |p| along(p) - lo);
    }
    if let Some(hi) = hi {
        out = clip_half_plane(&out, |p| hi - along(p));
    }
    if out.len() < 3 || signed_area(&out).abs() < EPSILON * EPSILON {
        Vec::new()
    } else {
        out
    }
}

/// One solid stretch of a wall between openings (or the wall ends). `None`
/// means the stretch reaches the wall end there.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WallPiece {
    pub from: Option<f64>,
    pub to: Option<f64>,
}

/// The solid pieces left after cutting the given spans out of a wall, in
/// order along it. Spans may touch or overlap; they are merged first.
pub fn wall_pieces(spans: &[(f64, f64)]) -> Vec<WallPiece> {
    let mut sorted: Vec<(f64, f64)> = spans.to_vec();
    sorted.sort_by(|a, b| a.0.total_cmp(&b.0));
    let mut merged: Vec<(f64, f64)> = Vec::new();
    for (a, b) in sorted {
        match merged.last_mut() {
            Some(last) if a <= last.1 + EPSILON => last.1 = last.1.max(b),
            _ => merged.push((a, b)),
        }
    }
    let mut pieces = Vec::with_capacity(merged.len() + 1);
    let mut from = None;
    for (a, b) in merged {
        pieces.push(WallPiece { from, to: Some(a) });
        from = Some(b);
    }
    pieces.push(WallPiece { from, to: None });
    pieces
}

/// Plan area of the wall removed by an opening: the wall outline clipped to
/// the opening's span. Times the opening height, it is the volume the
/// opening takes out of the wall.
pub fn removed_area(wall: &WallGeometry, opening: &OpeningGeometry) -> f64 {
    let piece = clip_span(
        &wall.outline,
        opening.origin,
        opening.u,
        Some(opening.t0),
        Some(opening.t1),
    );
    signed_area(&piece).abs()
}

#[cfg(test)]
mod tests {
    use super::*;
    use archisim_core::{CutPattern, Layer, NewWall, ReferenceLine, WallFunction};

    fn approx(a: f64, b: f64) -> bool {
        (a - b).abs() < 1e-9
    }

    /// A 10 × 8 m rectangle drawn clockwise along its exterior faces with a
    /// two-layer 0.3 m wall.
    fn rectangle() -> (Model, LevelId, Vec<WallId>) {
        let mut m = Model::new("T");
        let lvl = m.add_level("PB", 0.0);
        let a = m.add_material("A", "brick", [0, 0, 0], CutPattern::Solid);
        let b = m.add_material("B", "plaster", [0, 0, 0], CutPattern::Solid);
        let t = m
            .add_wall_type(
                "Fachada",
                WallFunction::Exterior,
                vec![
                    Layer {
                        material: a,
                        thickness: 0.2,
                        function: LayerFunction::Structure,
                    },
                    Layer {
                        material: b,
                        thickness: 0.1,
                        function: LayerFunction::Finish,
                    },
                ],
            )
            .unwrap();
        let corners = [
            Point2::new(0.0, 0.0),
            Point2::new(0.0, 8.0),
            Point2::new(10.0, 8.0),
            Point2::new(10.0, 0.0),
        ];
        let ids = (0..4)
            .map(|i| {
                m.add_wall(NewWall {
                    wall_type: t,
                    level: lvl,
                    start: corners[i],
                    end: corners[(i + 1) % 4],
                    reference: ReferenceLine::ExteriorFace,
                    height: 3.0,
                    base_offset: 0.0,
                    bulge: 0.0,
                    top: archisim_core::WallTop::Unconnected,
                })
                .unwrap()
            })
            .collect();
        (m, lvl, ids)
    }

    #[test]
    fn line_intersection() {
        let a = Line::new(Point2::new(0.0, 0.0), Point2::new(1.0, 0.0));
        let b = Line::new(Point2::new(2.0, -1.0), Point2::new(0.0, 3.0));
        assert!(a.intersect(&b).unwrap().approx_eq(Point2::new(2.0, 0.0)));
        assert!(a.intersect(&a).is_none());
    }

    #[test]
    fn rectangle_corners_close_exactly() {
        let (m, lvl, ids) = rectangle();
        let geos = wall_geometries(&m, lvl);
        assert_eq!(geos.len(), 4);
        for g in &geos {
            assert!(matches!(g.start_cut, EndCut::Along(_)));
            assert!(matches!(g.end_cut, EndCut::Along(_)));
            assert!(signed_area(&g.outline) > 0.0);
        }
        // Total wall area = outer rectangle minus inner rectangle.
        let total: f64 = geos.iter().map(|g| signed_area(&g.outline)).sum();
        assert!(approx(total, 10.0 * 8.0 - 9.4 * 7.4), "area {total}");
        // Layers add up to the whole wall.
        for g in &geos {
            let layers: f64 = g.layers.iter().map(|b| signed_area(&b.polygon)).sum();
            assert!(approx(layers, signed_area(&g.outline)));
        }
        // The first (west) wall reaches the building corners.
        let west = wall_geometry(&m, ids[0]).unwrap();
        for corner in [Point2::new(0.0, 0.0), Point2::new(0.0, 8.0)] {
            assert!(west.outline.iter().any(|p| p.approx_eq(corner)));
        }
        for corner in [Point2::new(0.3, 0.3), Point2::new(0.3, 7.7)] {
            assert!(west.outline.iter().any(|p| p.approx_eq(corner)));
        }
    }

    /// The rectangle with a wall of the same type (structure 0.2 + plaster
    /// 0.1) starting at the inner face of the west wall (x = 0.3).
    fn with_tee() -> (Model, Vec<WallId>, WallId) {
        let (mut m, lvl, ids) = rectangle();
        let t = m.wall_types().next().unwrap().id;
        let p = m
            .add_wall(NewWall {
                wall_type: t,
                level: lvl,
                start: Point2::new(0.3, 4.0),
                end: Point2::new(5.0, 4.0),
                reference: ReferenceLine::Center,
                height: 3.0,
                base_offset: 0.0,
                bulge: 0.0,
                top: archisim_core::WallTop::Unconnected,
            })
            .unwrap();
        (m, ids, p)
    }

    #[test]
    fn t_junction_structure_goes_through_the_plaster() {
        let (m, ids, p) = with_tee();
        let g = wall_geometry(&m, p).unwrap();
        // Its brick goes 0.1 m into the west wall, through its plaster, up
        // to its brick; its plaster stops at the face. The other end is free.
        let EndCut::Tee(tee) = &g.start_cut else {
            panic!("{:?}", g.start_cut)
        };
        assert_eq!(tee.host, ids[0]);
        assert!(approx(tee.reach[0], 0.1) && approx(tee.reach[1], 0.0));
        assert_eq!(g.end_cut, EndCut::Square);
        assert!(approx(signed_area(&g.outline), 4.8 * 0.2 + 4.7 * 0.1));
        let layers: f64 = g.layers.iter().map(|b| signed_area(&b.polygon)).sum();
        assert!(approx(layers, signed_area(&g.outline)));

        // The west wall loses that bit of its plaster, nothing else.
        let west = wall_geometry(&m, ids[0]).unwrap();
        assert_eq!(west.notches.len(), 1);
        assert_eq!(west.notches[0].regions.len(), 1);
        assert_eq!(west.notches[0].regions[0].layers, vec![1]);
        let plain = {
            let mut m2 = m.clone();
            m2.remove_wall(p).unwrap();
            wall_geometry(&m2, ids[0]).unwrap()
        };
        let lost = signed_area(&plain.outline) - signed_area(&west.outline);
        assert!(approx(lost, 0.2 * 0.1), "{lost}");
        let plaster = west.layer_pieces(1, Some(1.0));
        assert_eq!(plaster.len(), 2);
        let left: f64 = plaster.iter().map(|q| signed_area(q)).sum();
        assert!(approx(left, signed_area(&plain.layers[1].polygon) - 0.02));
        assert_eq!(west.layer_pieces(0, Some(1.0)).len(), 1);
        // Above the butting wall nothing is taken (it is 3 m high too).
        assert_eq!(west.layer_pieces(1, Some(3.5)).len(), 1);
    }

    #[test]
    fn material_priority_overrides_the_layer_function() {
        let (mut m, _, p) = with_tee();
        let t = m.wall_types().next().unwrap().clone();
        let plaster = t.layers[1].material;
        // A plaster stronger than brick: nothing goes through it.
        m.set_material_priority(plaster, Some(950)).unwrap();
        let g = wall_geometry(&m, p).unwrap();
        let EndCut::Tee(tee) = &g.start_cut else {
            panic!()
        };
        assert!(tee.reach.iter().all(|r| *r == 0.0), "{:?}", tee.reach);
        // Back to the functions: the brick goes through the plaster again.
        m.set_material_priority(plaster, None).unwrap();
        let g = wall_geometry(&m, p).unwrap();
        let EndCut::Tee(tee) = &g.start_cut else {
            panic!()
        };
        assert!(approx(tee.reach[0], 0.1));
        assert_eq!(
            m.set_material_priority(plaster, Some(1000)),
            Err(archisim_core::ModelError::InvalidPriority)
        );
    }

    #[test]
    fn a_lower_or_cornered_wall_just_butts() {
        let (mut m, ids, p) = with_tee();
        // Taller than the host: it stops at the face.
        m.set_wall_height(p, 3.5, 0.0).unwrap();
        let g = wall_geometry(&m, p).unwrap();
        let EndCut::Tee(tee) = &g.start_cut else {
            panic!()
        };
        assert!(tee.reach.iter().all(|r| *r == 0.0));
        let host = wall_geometry(&m, ids[0]).unwrap();
        assert_eq!(host.notches.len(), 1);
        assert!(host.notches[0].regions.is_empty(), "it takes nothing");
        // Too close to a corner: also just butting.
        m.set_wall_height(p, 3.0, 0.0).unwrap();
        m.move_wall(p, Point2::new(0.3, 0.35), Point2::new(5.0, 0.35))
            .unwrap();
        let g = wall_geometry(&m, p).unwrap();
        let EndCut::Tee(tee) = &g.start_cut else {
            panic!()
        };
        assert!(tee.reach.iter().all(|r| *r == 0.0));
        // Drawn up to the host's axis or exterior face, it still butts.
        m.move_wall(p, Point2::new(0.1, 4.0), Point2::new(5.0, 4.0))
            .unwrap();
        let g = wall_geometry(&m, p).unwrap();
        assert!(matches!(g.start_cut, EndCut::Tee(_)));
        assert!(approx(signed_area(&g.outline), 4.8 * 0.2 + 4.7 * 0.1));
    }

    #[test]
    fn pieces_between_openings() {
        let p = wall_pieces(&[(3.0, 4.0), (1.0, 2.0), (2.0, 2.5)]);
        assert_eq!(
            p,
            vec![
                WallPiece {
                    from: None,
                    to: Some(1.0)
                },
                WallPiece {
                    from: Some(2.5),
                    to: Some(3.0)
                },
                WallPiece {
                    from: Some(4.0),
                    to: None
                },
            ]
        );
        assert_eq!(wall_pieces(&[]).len(), 1);
    }

    #[test]
    fn openings_cut_mitred_walls_exactly() {
        use archisim_core::{
            DoorOperation, Hand, NewOpening, NewOpeningType, OpeningKind, WallSide,
        };
        let (mut m, _, ids) = rectangle();
        let mat = m.materials().next().unwrap().id;
        let t = m
            .add_opening_type(NewOpeningType {
                name: "P".into(),
                kind: OpeningKind::Door(DoorOperation::SingleSwing),
                width: 1.0,
                height: 2.1,
                frame_width: 0.05,
                frame_depth: 0.07,
                frame_material: mat,
                panel_material: mat,
            })
            .unwrap();
        // West wall runs (0,0) → (0,8); exterior on its left (−x side).
        let o = m
            .add_opening(NewOpening {
                opening_type: t,
                wall: ids[0],
                along: 4.0,
                sill: 0.0,
                opens_to: WallSide::Interior,
                hand: Hand::Left,
                setback: Some(0.1),
            })
            .unwrap();
        let og = opening_geometry(&m, o).unwrap();
        assert!(approx(og.width(), 1.0));
        assert!(approx(og.frame_exterior, -0.1));
        assert!(approx(og.frame_interior, -0.17));
        // Seen from inside the room (x > 0) looking west, left is south,
        // which is the wall start.
        assert!(og.hinge_at_start());
        let wg = wall_geometry(&m, ids[0]).unwrap();
        assert!(approx(removed_area(&wg, &og), 0.3));
        // An opening touching the mitred start only removes the trapezoid.
        let o2 = m
            .add_opening(NewOpening {
                opening_type: t,
                wall: ids[0],
                along: 0.5,
                sill: 0.0,
                opens_to: WallSide::Interior,
                hand: Hand::Right,
                setback: None,
            })
            .unwrap();
        let og2 = opening_geometry(&m, o2).unwrap();
        assert!(!og2.hinge_at_start());
        // Outer edge 1.0 m, inner edge 0.7 m (mitre), 0.3 m thick.
        assert!(approx(removed_area(&wg, &og2), (1.0 + 0.7) / 2.0 * 0.3));
        // The pieces left over add up to the rest of the wall.
        let pieces: f64 = wall_pieces(&[(og2.t0, og2.t1), (og.t0, og.t1)])
            .iter()
            .map(|p| signed_area(&clip_span(&wg.outline, og.origin, og.u, p.from, p.to)))
            .sum();
        let total = signed_area(&wg.outline);
        assert!(approx(pieces, total - 0.3 - 0.255), "{pieces} {total}");
    }

    #[test]
    fn collinear_walls_cut_perpendicular() {
        let (mut m, lvl, ids) = rectangle();
        let t = m.wall_types().next().unwrap().id;
        for id in ids {
            m.remove_wall(id).unwrap();
        }
        let spec = |a: f64, b: f64| NewWall {
            wall_type: t,
            level: lvl,
            start: Point2::new(a, 0.0),
            end: Point2::new(b, 0.0),
            reference: ReferenceLine::Center,
            height: 3.0,
            base_offset: 0.0,
            bulge: 0.0,
            top: archisim_core::WallTop::Unconnected,
        };
        let w1 = m.add_wall(spec(0.0, 3.0)).unwrap();
        m.add_wall(spec(3.0, 6.0)).unwrap();
        let g = wall_geometry(&m, w1).unwrap();
        assert!(approx(signed_area(&g.outline), 3.0 * 0.3));
    }
}

#[cfg(test)]
mod curved_tests;
