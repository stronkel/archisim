//! Object snap points of drafting shapes (AutoCAD's "referencias a
//! objetos"): end, middle, centre, quadrant, node, insertion; and the ones
//! that depend on the cursor or on the point drawn before: nearest,
//! perpendicular, tangent and intersection.

use std::f64::consts::{FRAC_PI_2, TAU};

use archisim_core::{Point2, Shape};

use crate::seg::{tangent_points, Seg};
use crate::{ellipse_point, polyline_segs, shape_segs};

/// Kinds of object snap, in order of preference when two are equally
/// close.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum SnapPoint {
    Endpoint,
    Intersection,
    Midpoint,
    Centre,
    Quadrant,
    Node,
    Insertion,
    Perpendicular,
    Tangent,
    Nearest,
}

impl SnapPoint {
    /// The name shown next to the cursor.
    pub fn label(self) -> &'static str {
        match self {
            SnapPoint::Endpoint => "Punto final",
            SnapPoint::Intersection => "Intersección",
            SnapPoint::Midpoint => "Punto medio",
            SnapPoint::Centre => "Centro",
            SnapPoint::Quadrant => "Cuadrante",
            SnapPoint::Node => "Punto",
            SnapPoint::Insertion => "Inserción",
            SnapPoint::Perpendicular => "Perpendicular",
            SnapPoint::Tangent => "Tangente",
            SnapPoint::Nearest => "Cercano",
        }
    }
}

fn arc_quadrants(
    centre: Point2,
    radius: f64,
    start: f64,
    sweep: f64,
    out: &mut Vec<(Point2, SnapPoint)>,
) {
    for q in 0..4 {
        let a = q as f64 * FRAC_PI_2;
        let s = Seg::Arc {
            centre,
            radius,
            start,
            sweep,
        };
        let t = s.param(centre + crate::seg::dir(a) * radius);
        if sweep.abs() >= TAU - 1e-9 || s.contains_param(t) {
            out.push((centre + crate::seg::dir(a) * radius, SnapPoint::Quadrant));
        }
    }
}

/// The fixed snap points of a shape.
pub fn snap_points(shape: &Shape) -> Vec<(Point2, SnapPoint)> {
    let mut out = Vec::new();
    match shape {
        Shape::Line { a, b } => {
            out.push((*a, SnapPoint::Endpoint));
            out.push((*b, SnapPoint::Endpoint));
            out.push(((*a + *b) * 0.5, SnapPoint::Midpoint));
        }
        Shape::Polyline(p) => {
            for q in &p.points {
                out.push((*q, SnapPoint::Endpoint));
            }
            for s in polyline_segs(p) {
                out.push((s.midpoint(), SnapPoint::Midpoint));
                if let Seg::Arc { centre, .. } = s {
                    out.push((centre, SnapPoint::Centre));
                }
            }
        }
        Shape::Circle { centre, radius } => {
            out.push((*centre, SnapPoint::Centre));
            arc_quadrants(*centre, *radius, 0.0, TAU, &mut out);
        }
        Shape::Arc {
            centre,
            radius,
            start,
            sweep,
        } => {
            let s = Seg::Arc {
                centre: *centre,
                radius: *radius,
                start: *start,
                sweep: *sweep,
            };
            out.push((s.start(), SnapPoint::Endpoint));
            out.push((s.end(), SnapPoint::Endpoint));
            out.push((s.midpoint(), SnapPoint::Midpoint));
            out.push((*centre, SnapPoint::Centre));
            arc_quadrants(*centre, *radius, *start, *sweep, &mut out);
        }
        Shape::Ellipse {
            centre,
            axis,
            ratio,
            start,
            sweep,
        } => {
            out.push((*centre, SnapPoint::Centre));
            let full = *sweep >= TAU - 1e-9;
            for q in 0..4 {
                let t = q as f64 * FRAC_PI_2;
                let rel = (t - start).rem_euclid(TAU);
                if full || rel <= *sweep {
                    out.push((
                        ellipse_point(*centre, *axis, *ratio, t),
                        SnapPoint::Quadrant,
                    ));
                }
            }
            if !full {
                out.push((
                    ellipse_point(*centre, *axis, *ratio, *start),
                    SnapPoint::Endpoint,
                ));
                out.push((
                    ellipse_point(*centre, *axis, *ratio, start + sweep),
                    SnapPoint::Endpoint,
                ));
            }
        }
        Shape::Point { at } => out.push((*at, SnapPoint::Node)),
        Shape::Text(t) => out.push((t.at, SnapPoint::Insertion)),
        Shape::Hatch(h) => {
            for l in &h.loops {
                for q in &l.points {
                    out.push((*q, SnapPoint::Endpoint));
                }
            }
        }
    }
    out
}

/// The segments a shape can be snapped onto (its lines, and the boundary
/// of a hatch).
pub fn snap_segs(shape: &Shape) -> Vec<Seg> {
    crate::boundary_segs(shape)
}

/// The nearest point on the segments to the cursor.
pub fn nearest(segs: &[Seg], cursor: Point2) -> Option<Point2> {
    segs.iter()
        .map(|s| s.nearest(cursor).1)
        .min_by(|a, b| a.distance(cursor).total_cmp(&b.distance(cursor)))
}

/// Feet of the perpendiculars from `from` onto the segments (on the
/// segment itself), the one nearest to the cursor.
pub fn perpendicular(segs: &[Seg], from: Point2, cursor: Point2, reach: f64) -> Option<Point2> {
    segs.iter()
        .filter_map(|s| {
            let foot = match s {
                Seg::Line { .. } => s.point_at(s.param(from)),
                Seg::Arc { centre, radius, .. } => {
                    let d = from - *centre;
                    let n = d.normalized()?;
                    // Of the two feet, the one nearer the cursor.
                    let a = *centre + n * *radius;
                    let b = *centre - n * *radius;
                    if a.distance(cursor) < b.distance(cursor) {
                        a
                    } else {
                        b
                    }
                }
            };
            let t = s.param_near(foot);
            (s.contains_param(t) && foot.distance(cursor) <= reach).then_some(foot)
        })
        .min_by(|a, b| a.distance(cursor).total_cmp(&b.distance(cursor)))
}

/// Tangent points from `from` onto the arcs and circles among the
/// segments, the one nearest to the cursor.
pub fn tangent(segs: &[Seg], from: Point2, cursor: Point2, reach: f64) -> Option<Point2> {
    segs.iter()
        .filter_map(|s| match s {
            Seg::Arc { centre, radius, .. } => Some(
                tangent_points(from, *centre, *radius)
                    .into_iter()
                    .filter(|p| s.contains_param(s.param_near(*p)))
                    .collect::<Vec<_>>(),
            ),
            _ => None,
        })
        .flatten()
        .filter(|p| p.distance(cursor) <= reach)
        .min_by(|a, b| a.distance(cursor).total_cmp(&b.distance(cursor)))
}

/// Intersections between the segments near the cursor (within `reach`).
pub fn intersections(segs: &[Seg], cursor: Point2, reach: f64) -> Vec<Point2> {
    let near: Vec<&Seg> = segs
        .iter()
        .filter(|s| s.distance(cursor) <= reach)
        .collect();
    let mut out = Vec::new();
    for i in 0..near.len() {
        for j in i + 1..near.len() {
            for (_, _, p) in crate::seg::intersect(near[i], near[j]) {
                if p.distance(cursor) <= reach {
                    out.push(p);
                }
            }
        }
    }
    out
}

/// All the segments of several shapes.
pub fn segs_of<'a>(shapes: impl IntoIterator<Item = &'a Shape>) -> Vec<Seg> {
    shapes.into_iter().flat_map(shape_segs).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(x: f64, y: f64) -> Point2 {
        Point2::new(x, y)
    }

    #[test]
    fn points_of_shapes() {
        let arc = Shape::Arc {
            centre: p(0.0, 0.0),
            radius: 1.0,
            start: 0.0,
            sweep: FRAC_PI_2,
        };
        let pts = snap_points(&arc);
        // Two ends, middle, centre and the two quadrants it reaches
        // (which are also its ends).
        assert_eq!(pts.iter().filter(|p| p.1 == SnapPoint::Quadrant).count(), 2);
        assert!(pts.contains(&(p(0.0, 0.0), SnapPoint::Centre)));
        let c = Shape::Circle {
            centre: p(1.0, 1.0),
            radius: 1.0,
        };
        assert_eq!(snap_points(&c).len(), 5);
    }

    #[test]
    fn perpendicular_tangent_and_crossings() {
        let segs = vec![
            Seg::Line {
                a: p(0.0, 0.0),
                b: p(10.0, 0.0),
            },
            Seg::Arc {
                centre: p(5.0, 5.0),
                radius: 1.0,
                start: 0.0,
                sweep: TAU,
            },
            Seg::Line {
                a: p(3.0, -1.0),
                b: p(3.0, 1.0),
            },
        ];
        let f = perpendicular(&segs, p(2.0, 3.0), p(2.1, 0.1), 0.5).unwrap();
        assert!(f.approx_eq(p(2.0, 0.0)));
        let t = tangent(&segs, p(5.0, 0.0), p(4.0, 5.0), 0.5).unwrap();
        assert!(((t - p(5.0, 0.0)).dot(t - p(5.0, 5.0))).abs() < 1e-9);
        let x = intersections(&segs, p(3.05, 0.05), 0.2);
        assert_eq!(x.len(), 1);
        assert!(x[0].approx_eq(p(3.0, 0.0)));
    }
}
