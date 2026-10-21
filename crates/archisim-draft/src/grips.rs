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
        Shape::Dim(m) => {
            let [c, h] = crate::dims::handles(m);
            vec![(c, GripKind::Move), (h, GripKind::Size)]
        }
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
        Shape::Dim(m) => match index {
            0 => translate(shape),
            _ => Shape::Dim(crate::dims::place(m, to)),
        },
    }
}

// ---- Vertices: added on an edge, removed one by one (as ArchiCAD) --------

/// A polyline with a new vertex on its segment nearest to `at`: the
/// polyline, the new vertex's index and how far `at` was from the segment.
/// `None` when `at` is on an existing vertex (nothing to add).
fn insert_in(p: &Polyline, at: Point2) -> Option<(Polyline, usize, f64)> {
    let segs = polyline_segs(p);
    let (i, (t, q), d) = segs
        .iter()
        .enumerate()
        .map(|(i, s)| {
            let n = s.nearest(at);
            (i, n, n.1.distance(at))
        })
        .min_by(|a, b| a.2.total_cmp(&b.2))?;
    let s = segs[i];
    let len = s.length();
    if len < 1e-9 || t * len < 1e-6 || (1.0 - t) * len < 1e-6 {
        return None;
    }
    let mut out = p.clone();
    out.bulges.resize(out.points.len(), 0.0);
    out.bulges[i] = s.sub(0.0, t).bulge();
    out.points.insert(i + 1, q);
    out.bulges.insert(i + 1, s.sub(t, 1.0).bulge());
    Some((out, i + 1, d))
}

/// A polyline without vertex `i` (the two segments that met there become
/// one; two arcs, one arc of both sweeps). `None` if it would be left with
/// too few points (two on an open one, three on a closed one).
fn remove_in(p: &Polyline, i: usize) -> Option<Polyline> {
    let n = p.points.len();
    if i >= n || n <= if p.closed { 3 } else { 2 } {
        return None;
    }
    let mut out = p.clone();
    out.bulges.resize(n, 0.0);
    let sweep = |b: f64| 4.0 * b.atan();
    if !p.closed && i == 0 {
        // The first segment goes with it.
    } else if !p.closed && i == n - 1 {
        out.bulges[i - 1] = 0.0;
    } else {
        let prev = (i + n - 1) % n;
        let joined = sweep(out.bulges[prev]) + sweep(out.bulges[i]);
        out.bulges[prev] = if out.bulges[prev].abs() < 1e-12 && out.bulges[i].abs() < 1e-12 {
            0.0
        } else {
            (joined / 4.0).tan()
        };
    }
    out.points.remove(i);
    out.bulges.remove(i);
    Some(out)
}

/// `shape` with a new vertex on its edge nearest to `at`, within `reach`:
/// the shape and the index of the new vertex's grip (to drag it at once).
/// A line becomes a polyline of two segments. Circles, arcs, ellipses,
/// texts and dimensions have no vertices to add.
pub fn insert_vertex(shape: &Shape, at: Point2, reach: f64) -> Option<(Shape, usize)> {
    match shape {
        Shape::Line { a, b } => {
            let p = Polyline::straight(vec![*a, *b], false);
            let (q, i, d) = insert_in(&p, at)?;
            (d <= reach).then_some((Shape::Polyline(q), i))
        }
        Shape::Polyline(p) => {
            let (q, i, d) = insert_in(p, at)?;
            (d <= reach).then_some((Shape::Polyline(q), i))
        }
        Shape::Hatch(h) => {
            let mut offset = 0;
            let mut best: Option<(usize, Polyline, usize, f64)> = None;
            for (k, l) in h.loops.iter().enumerate() {
                if let Some((q, i, d)) = insert_in(l, at) {
                    if best.as_ref().is_none_or(|b| d < b.3) {
                        best = Some((k, q, offset + i, d));
                    }
                }
                offset += l.points.len();
            }
            let (k, q, index, d) = best?;
            if d > reach {
                return None;
            }
            let mut out = h.clone();
            out.loops[k] = q;
            Some((Shape::Hatch(out), index))
        }
        _ => None,
    }
}

/// Whether grip `index` of `shape` is a vertex that can be removed.
pub fn is_vertex(shape: &Shape, index: usize) -> bool {
    match shape {
        Shape::Polyline(p) => index < p.points.len(),
        Shape::Hatch(h) => index < h.loops.iter().map(|l| l.points.len()).sum(),
        _ => false,
    }
}

/// `shape` without the vertex of grip `index`; `None` if it is not a
/// vertex or the shape would be left with too few (an open polyline of two
/// points becomes a line).
pub fn remove_vertex(shape: &Shape, index: usize) -> Option<Shape> {
    match shape {
        Shape::Polyline(p) => {
            if !p.closed && p.points.len() == 2 {
                return None;
            }
            let q = remove_in(p, index)?;
            if !q.closed && q.points.len() == 2 && q.is_straight() && q.width == 0.0 {
                return Some(Shape::Line {
                    a: q.points[0],
                    b: q.points[1],
                });
            }
            Some(Shape::Polyline(q))
        }
        Shape::Hatch(h) => {
            let mut i = index;
            for (k, l) in h.loops.iter().enumerate() {
                if i < l.points.len() {
                    let mut out = h.clone();
                    out.loops[k] = remove_in(l, i)?;
                    return Some(Shape::Hatch(out));
                }
                i -= l.points.len();
            }
            None
        }
        _ => None,
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

    #[test]
    fn vertices_are_added_on_edges_and_removed() {
        // A line becomes a polyline with the new vertex in the middle.
        let line = Shape::Line {
            a: p(0.0, 0.0),
            b: p(4.0, 0.0),
        };
        let (s, i) = insert_vertex(&line, p(1.0, 0.05), 0.1).unwrap();
        let Shape::Polyline(q) = &s else { panic!() };
        assert_eq!(i, 1);
        assert_eq!(q.points.len(), 3);
        assert!(q.points[1].approx_eq(p(1.0, 0.0)));
        // Too far from the edge: nothing.
        assert!(insert_vertex(&line, p(1.0, 1.0), 0.1).is_none());
        // Dragging it makes a corner; removing it gives the line back.
        let moved = move_grip(&s, 1, p(1.0, 2.0));
        assert!(is_vertex(&moved, 1));
        assert_eq!(remove_vertex(&moved, 1), Some(line.clone()));
        // A closed hatch: the new corner on the side clicked, in order.
        let h = Shape::Hatch(archisim_core::Hatch {
            loops: vec![Polyline::straight(
                vec![p(0.0, 0.0), p(4.0, 0.0), p(4.0, 3.0), p(0.0, 3.0)],
                true,
            )],
            pattern: archisim_core::HatchPattern::Solid,
            scale: 1.0,
            angle: 0.0,
            background: None,
            opacity: 1.0,
        });
        let (h2, i) = insert_vertex(&h, p(2.0, 3.0), 0.1).unwrap();
        assert_eq!(i, 3);
        let a0 = crate::area(&h).unwrap();
        let h3 = move_grip(&h2, i, p(2.0, 4.0));
        assert!((crate::area(&h3).unwrap() - (a0 + 2.0)).abs() < 1e-9);
        let back = remove_vertex(&h3, i).unwrap();
        assert!((crate::area(&back).unwrap() - a0).abs() < 1e-9);
        // A triangle keeps its three corners.
        let Shape::Hatch(t) = &back else { panic!() };
        let tri = Shape::Hatch(archisim_core::Hatch {
            loops: vec![Polyline::straight(t.loops[0].points[..3].to_vec(), true)],
            ..t.clone()
        });
        assert!(remove_vertex(&tri, 0).is_none());
        // A vertex on an arc splits it into two arcs of the same circle.
        let arc = Shape::Polyline(Polyline {
            points: vec![p(1.0, 0.0), p(-1.0, 0.0)],
            bulges: vec![1.0, 0.0],
            closed: false,
            width: 0.0,
        });
        let (s, i) = insert_vertex(&arc, p(0.0, 1.0), 0.1).unwrap();
        let Shape::Polyline(q) = &s else { panic!() };
        assert!(q.points[i].approx_eq(p(0.0, 1.0)));
        let quarter = (std::f64::consts::FRAC_PI_2 / 4.0).tan();
        assert!((q.bulges[0] - quarter).abs() < 1e-9 && (q.bulges[1] - quarter).abs() < 1e-9);
        let Some(Shape::Polyline(r)) = remove_vertex(&s, i) else {
            panic!()
        };
        assert!((r.bulges[0] - 1.0).abs() < 1e-9);
    }
}
