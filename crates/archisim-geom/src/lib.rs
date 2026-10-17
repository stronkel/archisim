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

use archisim_core::{
    LayerFunction, LevelId, MaterialId, Model, Point2, Vec2, Wall, WallId, EPSILON,
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

/// Signed area of a polygon (positive when counter-clockwise).
pub fn signed_area(poly: &[Point2]) -> f64 {
    let n = poly.len();
    (0..n)
        .map(|i| poly[i].cross(poly[(i + 1) % n]))
        .sum::<f64>()
        / 2.0
}

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
