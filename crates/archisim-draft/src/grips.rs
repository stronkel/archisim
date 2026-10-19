//! Grips of drafting shapes (AutoCAD's "pinzamientos"): the points shown on
//! a selected shape, and what dragging one of them does.

use std::f64::consts::{FRAC_PI_2, TAU};

use archisim_core::{Point2, Polyline, Shape};

use crate::seg::{angle_of, Seg};
use crate::transform::{transform_shape, Transform};
use crate::{ellipse_point, polyline_segs};

/// What dragging a grip does.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GripKind {
    /// Moves that point (an end, a corner).
    Stretch,
    /// Moves the whole shape (a centre, the middle of a line, a text).
    Move,
    /// Changes a size keeping the centre (radius, axis).
    Size,
    /// Bends an arc through the point.
    Bend,
}

/// The grips of a shape, in a fixed order.
pub fn grips(shape: &Shape) -> Vec<(Point2, GripKind)> {
    match shape {
        Shape::Line { a, b } => vec![
            (*a, GripKind::Stretch),
            (*b, GripKind::Stretch),
            ((*a + *b) * 0.5, GripKind::Move),
        ],
        Shape::Polyline(p) => {
            let mut v: Vec<(Point2, GripKind)> =
                p.points.iter().map(|q| (*q, GripKind::Stretch)).collect();
            // Middles of the segments bend arcs and move straight sides.
            for s in polyline_segs(p) {
                v.push((s.midpoint(), GripKind::Bend));
            }
            v
        }
        Shape::Circle { centre, radius } => {
            let mut v = vec![(*centre, GripKind::Move)];
            for q in 0..4 {
                v.push((
                    *centre + crate::seg::dir(q as f64 * FRAC_PI_2) * *radius,
                    GripKind::Size,
                ));
            }
            v
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
            vec![
                (s.start(), GripKind::Stretch),
                (s.end(), GripKind::Stretch),
                (s.midpoint(), GripKind::Bend),
                (*centre, GripKind::Move),
            ]
        }
        Shape::Ellipse {
            centre,
            axis,
            ratio,
            ..
        } => vec![
            (*centre, GripKind::Move),
            (ellipse_point(*centre, *axis, *ratio, 0.0), GripKind::Size),
            (
                ellipse_point(*centre, *axis, *ratio, FRAC_PI_2),
                GripKind::Size,
            ),
        ],
        Shape::Point { at } => vec![(*at, GripKind::Move)],
        Shape::Text(t) => vec![(t.at, GripKind::Move)],
        Shape::Hatch(h) => h
            .loops
            .iter()
            .flat_map(|l| l.points.iter().map(|q| (*q, GripKind::Stretch)))
            .collect(),
    }
}

fn moved_polyline(p: &Polyline, index: usize, to: Point2) -> Polyline {
    let n = p.points.len();
    let mut q = p.clone();
    if index < n {
        q.points[index] = to;
        return q;
    }
    // The middle of segment i.
    let i = index - n;
    let (a, b, bulge) = p.segment(i);
    if bulge.abs() > 1e-12 {
        // An arc passes through the new point.
        if let Some(bl) = archisim_core::Arc::bulge_through(a, to, b) {
            q.bulges[i] = bl;
        }
    } else {
        // A straight side moves parallel to itself.
        let m = (a + b) * 0.5;
        let d = to - m;
        q.points[i] = a + d;
        q.points[(i + 1) % n] = b + d;
    }
    q
}

/// The shape after dragging grip `index` to `to`.
pub fn move_grip(shape: &Shape, index: usize, to: Point2) -> Shape {
    let Some((from, _)) = grips(shape).get(index).copied() else {
        return shape.clone();
    };
    let translate = |s: &Shape| transform_shape(s, &Transform::translate(to - from));
    match shape {
        Shape::Line { a, b } => match index {
            0 => Shape::Line { a: to, b: *b },
            1 => Shape::Line { a: *a, b: to },
            _ => translate(shape),
        },
        Shape::Polyline(p) => Shape::Polyline(moved_polyline(p, index, to)),
        Shape::Circle { centre, .. } => match index {
            0 => translate(shape),
            _ => Shape::Circle {
                centre: *centre,
                radius: centre.distance(to).max(1e-6),
            },
        },
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
            let (a, b, m) = (s.start(), s.end(), s.midpoint());
            let rebuilt = |a: Point2, through: Point2, b: Point2| {
                archisim_core::Arc::bulge_through(a, through, b)
                    .map(|bl| crate::ops::seg_shape(&Seg::from_bulge(a, b, bl)))
                    .unwrap_or_else(|| shape.clone())
            };
            match index {
                // Moving an end keeps the arc's bulge.
                0 => crate::ops::seg_shape(&Seg::from_bulge(to, b, s.bulge())),
                1 => crate::ops::seg_shape(&Seg::from_bulge(a, to, s.bulge())),
                2 => rebuilt(a, to, b),
                _ => {
                    let _ = m;
                    translate(shape)
                }
            }
        }
        Shape::Ellipse {
            centre,
            axis,
            ratio,
            start,
            sweep,
        } => match index {
            0 => translate(shape),
            1 => {
                let new_axis = to - *centre;
                let a = axis.length();
                let b = a * ratio;
                let na = new_axis.length().max(1e-6);
                // Keep the other axis; swap if the new one is shorter.
                if na >= b {
                    Shape::Ellipse {
                        centre: *centre,
                        axis: new_axis,
                        ratio: (b / na).min(1.0),
                        start: *start,
                        sweep: *sweep,
                    }
                } else {
                    Shape::Ellipse {
                        centre: *centre,
                        axis: new_axis.perp_left() * (b / na),
                        ratio: na / b,
                        start: start - FRAC_PI_2,
                        sweep: *sweep,
                    }
                }
            }
            _ => {
                let a = axis.length();
                let nb = to.distance(*centre).max(1e-6);
                if nb <= a {
                    Shape::Ellipse {
                        centre: *centre,
                        axis: *axis,
                        ratio: nb / a,
                        start: *start,
                        sweep: *sweep,
                    }
                } else {
                    let u = axis.perp_left() * (1.0 / a.max(1e-12));
                    Shape::Ellipse {
                        centre: *centre,
                        axis: u * nb,
                        ratio: a / nb,
                        start: (start - FRAC_PI_2).rem_euclid(TAU),
                        sweep: *sweep,
                    }
                }
            }
        },
        Shape::Point { .. } | Shape::Text(_) => translate(shape),
        Shape::Hatch(h) => {
            let mut h = h.clone();
            let mut i = index;
            for l in &mut h.loops {
                if i < l.points.len() {
                    l.points[i] = to;
                    break;
                }
                i -= l.points.len();
            }
            Shape::Hatch(h)
        }
    }
}

/// The angle of the line from `a` to `b` (helper for rotation grips).
pub fn angle(a: Point2, b: Point2) -> f64 {
    angle_of(b - a)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(x: f64, y: f64) -> Point2 {
        Point2::new(x, y)
    }

    #[test]
    fn line_and_polyline_grips() {
        let l = Shape::Line {
            a: p(0.0, 0.0),
            b: p(2.0, 0.0),
        };
        assert_eq!(grips(&l).len(), 3);
        assert_eq!(
            move_grip(&l, 1, p(3.0, 1.0)),
            Shape::Line {
                a: p(0.0, 0.0),
                b: p(3.0, 1.0)
            }
        );
        assert_eq!(
            move_grip(&l, 2, p(1.0, 1.0)),
            Shape::Line {
                a: p(0.0, 1.0),
                b: p(2.0, 1.0)
            }
        );
        let pl = Shape::Polyline(Polyline::straight(
            vec![p(0.0, 0.0), p(2.0, 0.0), p(2.0, 2.0)],
            false,
        ));
        // The middle of the first side moves it up.
        let Shape::Polyline(q) = move_grip(&pl, 3, p(1.0, 0.5)) else {
            panic!()
        };
        assert_eq!(q.points[0], p(0.0, 0.5));
        assert_eq!(q.points[1], p(2.0, 0.5));
    }

    #[test]
    fn circle_and_arc_grips() {
        let c = Shape::Circle {
            centre: p(0.0, 0.0),
            radius: 1.0,
        };
        assert!(
            matches!(move_grip(&c, 1, p(3.0, 0.0)), Shape::Circle { radius, .. } if (radius - 3.0).abs() < 1e-12)
        );
        let arc = Shape::Arc {
            centre: p(0.0, 0.0),
            radius: 1.0,
            start: 0.0,
            sweep: std::f64::consts::PI,
        };
        // Bending the half circle through (0, 2).
        let Shape::Arc { centre, .. } = move_grip(&arc, 2, p(0.0, 2.0)) else {
            panic!()
        };
        assert!(centre.y > 0.0);
    }
}
