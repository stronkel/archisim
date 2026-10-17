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
//! - Any other end (free, T-junction, three or more walls) is cut square.
//!
//! Doors and windows are resolved by [`opening_geometry`]: their span along
//! the wall, the faces of their frame and their height. [`clip_span`] cuts a
//! wall (or one of its layers) into the solid pieces between openings.

use archisim_core::{
    Hand, LayerFunction, LevelId, MaterialId, Model, OpeningId, OpeningKind, Point2, Vec2, Wall,
    WallId, WallSide, EPSILON,
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

pub use archisim_core::signed_area;

/// Returns the polygon in counter-clockwise order.
pub fn ensure_ccw(mut poly: Vec<Point2>) -> Vec<Point2> {
    if signed_area(&poly) < 0.0 {
        poly.reverse();
    }
    poly
}

/// How one end of a wall is cut.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum EndCut {
    /// Perpendicular to the axis, at the end point.
    Square,
    /// Along the given line (a mitre with a connected wall).
    Along(Line),
}

/// The cut polygon of one material layer.
#[derive(Debug, Clone, PartialEq)]
pub struct LayerBand {
    pub material: MaterialId,
    pub function: LayerFunction,
    /// Closed polygon, counter-clockwise, without repeating the first point.
    pub polygon: Vec<Point2>,
}

/// Plan geometry of one wall.
#[derive(Debug, Clone, PartialEq)]
pub struct WallGeometry {
    pub wall: WallId,
    /// The drawn line (reference line) of the wall.
    pub axis: (Point2, Point2),
    /// Offset of the exterior face from the drawn line (left positive).
    pub exterior_offset: f64,
    /// Offset of the interior face from the drawn line (left positive).
    pub interior_offset: f64,
    pub start_cut: EndCut,
    pub end_cut: EndCut,
    /// Outline of the whole wall, counter-clockwise.
    pub outline: Vec<Point2>,
    /// One band per layer, from exterior to interior.
    pub layers: Vec<LayerBand>,
}

/// Which end of a wall touches a junction.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum End {
    Start,
    End,
}

/// Geometry of every wall on a level.
pub fn wall_geometries(model: &Model, level: LevelId) -> Vec<WallGeometry> {
    model
        .walls_on_level(level)
        .filter_map(|w| wall_geometry(model, w.id))
        .collect()
}

/// Geometry of one wall, with joins against walls on the same level.
pub fn wall_geometry(model: &Model, id: WallId) -> Option<WallGeometry> {
    let wall = model.wall(id)?;
    let wtype = model.wall_type(wall.wall_type)?;
    let (ext, int) = wall.face_offsets(wtype.thickness());

    let start_cut = end_cut(model, wall, End::Start);
    let end_cut_ = end_cut(model, wall, End::End);

    let dir = wall.end - wall.start;
    let normal = dir.normalized()?.perp_left();
    let point_at = |offset: f64, end: End| -> Point2 {
        let (base, cut) = match end {
            End::Start => (wall.start, start_cut),
            End::End => (wall.end, end_cut_),
        };
        let square = base + normal * offset;
        match cut {
            EndCut::Square => square,
            EndCut::Along(line) => Line::new(wall.start + normal * offset, dir)
                .intersect(&line)
                .unwrap_or(square),
        }
    };
    let band = |high: f64, low: f64| -> Vec<Point2> {
        ensure_ccw(vec![
            point_at(low, End::Start),
            point_at(low, End::End),
            point_at(high, End::End),
            point_at(high, End::Start),
        ])
    };

    let mut layers = Vec::with_capacity(wtype.layers.len());
    let mut high = ext;
    for layer in &wtype.layers {
        let low = high - layer.thickness;
        layers.push(LayerBand {
            material: layer.material,
            function: layer.function,
            polygon: band(high, low),
        });
        high = low;
    }

    Some(WallGeometry {
        wall: id,
        axis: (wall.start, wall.end),
        exterior_offset: ext,
        interior_offset: int,
        start_cut,
        end_cut: end_cut_,
        outline: band(ext, int),
        layers,
    })
}

/// The two face lines of a wall, as (left, right) when walking in direction
/// `towards`: `true` walks start → end, `false` walks end → start.
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

fn end_cut(model: &Model, wall: &Wall, end: End) -> EndCut {
    let p = match end {
        End::Start => wall.start,
        End::End => wall.end,
    };
    let mut others = model.walls_on_level(wall.level).filter_map(|o| {
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
        _ => return EndCut::Square,
    };
    mitre(model, wall, end, other, other_end).unwrap_or(EndCut::Square)
}

/// Mitre between `wall` arriving at the junction and `other` leaving it.
fn mitre(model: &Model, wall: &Wall, end: End, other: &Wall, other_end: End) -> Option<EndCut> {
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
    /// Start of the wall's drawn line, origin of the `t` coordinates.
    pub origin: Point2,
    /// Unit vector along the wall, start → end.
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
}

impl OpeningGeometry {
    pub fn width(&self) -> f64 {
        self.t1 - self.t0
    }

    pub fn height(&self) -> f64 {
        self.head - self.sill
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

/// Geometry of one door or window, or `None` if it or its wall is missing.
pub fn opening_geometry(model: &Model, id: OpeningId) -> Option<OpeningGeometry> {
    let o = model.opening(id)?;
    let otype = model.opening_type(o.opening_type)?;
    let wall = model.wall(o.wall)?;
    let thickness = model.wall_type(wall.wall_type)?.thickness();
    let (ext, int) = wall.face_offsets(thickness);
    let u = (wall.end - wall.start).normalized()?;
    let setback = o.frame_setback(thickness, otype.frame_depth);
    let (t0, t1) = o.span(otype.width);
    Some(OpeningGeometry {
        opening: id,
        wall: wall.id,
        kind: otype.kind,
        origin: wall.start,
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

    #[test]
    fn free_and_t_ends_are_square() {
        let (mut m, lvl, _) = rectangle();
        let t = m.wall_types().next().unwrap().id;
        // A partition starting at the inner face of the west wall (T-junction).
        let p = m
            .add_wall(NewWall {
                wall_type: t,
                level: lvl,
                start: Point2::new(0.3, 4.0),
                end: Point2::new(5.0, 4.0),
                reference: ReferenceLine::Center,
                height: 3.0,
                base_offset: 0.0,
            })
            .unwrap();
        let g = wall_geometry(&m, p).unwrap();
        assert_eq!(g.start_cut, EndCut::Square);
        assert_eq!(g.end_cut, EndCut::Square);
        assert!(approx(signed_area(&g.outline), 4.7 * 0.3));
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
        };
        let w1 = m.add_wall(spec(0.0, 3.0)).unwrap();
        m.add_wall(spec(3.0, 6.0)).unwrap();
        let g = wall_geometry(&m, w1).unwrap();
        assert!(approx(signed_area(&g.outline), 3.0 * 0.3));
    }
}
