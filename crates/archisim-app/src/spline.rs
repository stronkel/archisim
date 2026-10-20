//! Free curves for the drawing tools (the «Spline» method) and the lines
//! picked from the 2D drawing («Elegir línea»), as the points and bulges
//! the tools are built with. No screen: what the curve is.

use archisim_core::{Point2, Shape};

/// A Catmull-Rom curve through the control points: `per_span` points from
/// each one to the next (open: from the first to the last; closed: back to
/// the first).
pub fn catmull_rom(points: &[Point2], closed: bool, per_span: usize) -> Vec<Point2> {
    let n = points.len();
    if n < 2 {
        return points.to_vec();
    }
    let at = |i: isize| -> Point2 {
        if closed {
            points[i.rem_euclid(n as isize) as usize]
        } else {
            points[i.clamp(0, n as isize - 1) as usize]
        }
    };
    let spans = if closed { n } else { n - 1 };
    let mut out = Vec::with_capacity(spans * per_span + 1);
    for i in 0..spans as isize {
        let (p0, p1, p2, p3) = (at(i - 1), at(i), at(i + 1), at(i + 2));
        for k in 0..per_span {
            let t = k as f64 / per_span as f64;
            out.push(point(p0, p1, p2, p3, t));
        }
    }
    if !closed {
        out.push(points[n - 1]);
    }
    out
}

/// The uniform Catmull-Rom point at `t` (0–1) of the span from `p1` to
/// `p2`.
fn point(p0: Point2, p1: Point2, p2: Point2, p3: Point2, t: f64) -> Point2 {
    let (t2, t3) = (t * t, t * t * t);
    let c = |a: f64, b: f64, c: f64, d: f64| {
        0.5 * ((2.0 * b)
            + (-a + c) * t
            + (2.0 * a - 5.0 * b + 4.0 * c - d) * t2
            + (-a + 3.0 * b - 3.0 * c + d) * t3)
    };
    Point2::new(c(p0.x, p1.x, p2.x, p3.x), c(p0.y, p1.y, p2.y, p3.y))
}

/// The curve as arcs, one from each control point to the next through the
/// curve's middle there: `(start, end, bulge)`, as walls are drawn.
pub fn spline_arcs(points: &[Point2], closed: bool) -> Vec<(Point2, Point2, f64)> {
    let n = points.len();
    if n < 2 {
        return Vec::new();
    }
    let samples = catmull_rom(points, closed, 2);
    let spans = if closed { n } else { n - 1 };
    (0..spans)
        .map(|i| {
            let (a, b) = (points[i], points[(i + 1) % n]);
            (a, b, bulge_through(a, samples[2 * i + 1], b))
        })
        .collect()
}

/// The bulge of the arc from `a` to `b` through `m` (DXF: positive
/// counter-clockwise): the angle the chord is seen with from `m` gives the
/// arc's sweep (inscribed angle).
pub fn bulge_through(a: Point2, m: Point2, b: Point2) -> f64 {
    let (u, v) = (a - m, b - m);
    let cross = u.x * v.y - u.y * v.x;
    let dot = u.x * v.x + u.y * v.y;
    if cross.abs() < 1e-12 || (b - a).length() < 1e-9 {
        return 0.0;
    }
    // The angle at m, 0–π; the sweep is 2(π − φ).
    let phi = cross.abs().atan2(dot);
    let b = ((std::f64::consts::PI - phi) / 2.0).tan();
    // Seen from m, a to b turning clockwise: the arc goes counter-clockwise.
    if cross < 0.0 {
        b
    } else {
        -b
    }
}

/// A line of the 2D drawing as the tools take it: its pieces (start, end,
/// bulge), whether it closes, and its points (arcs tessellated).
pub struct PickedLine {
    pub pieces: Vec<(Point2, Point2, f64)>,
    pub closed: bool,
    pub points: Vec<Point2>,
}

/// A line, polyline, arc or circle of the 2D drawing; `None` for texts,
/// hatches and dimensions.
pub fn picked_line(shape: &Shape) -> Option<PickedLine> {
    let closed = match shape {
        Shape::Line { .. } | Shape::Arc { .. } => false,
        Shape::Polyline(p) => p.closed,
        Shape::Circle { .. } => true,
        _ => return None,
    };
    let segs = match shape {
        // A circle as four quarter arcs, so that each piece is an arc.
        Shape::Circle { centre, radius } => {
            let q: Vec<Point2> = (0..4)
                .map(|i| {
                    let a = i as f64 * std::f64::consts::FRAC_PI_2;
                    *centre + Point2::new(a.cos(), a.sin()) * *radius
                })
                .collect();
            let b = (std::f64::consts::FRAC_PI_2 / 4.0).tan();
            (0..4)
                .map(|i| archisim_draft::Seg::from_bulge(q[i], q[(i + 1) % 4], b))
                .collect()
        }
        _ => archisim_draft::shape_segs(shape),
    };
    if segs.is_empty() {
        return None;
    }
    let pieces: Vec<(Point2, Point2, f64)> = segs
        .iter()
        .map(|s| (s.start(), s.end(), s.bulge()))
        .collect();
    let mut points = Vec::new();
    for s in &segs {
        let t = s.tessellate(0.01);
        let skip = usize::from(!points.is_empty());
        points.extend(t.into_iter().skip(skip));
    }
    if closed
        && points.len() > 2
        && points
            .first()
            .zip(points.last())
            .is_some_and(|(a, b)| a.approx_eq(*b))
    {
        points.pop();
    }
    Some(PickedLine {
        pieces,
        closed,
        points,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(x: f64, y: f64) -> Point2 {
        Point2::new(x, y)
    }

    #[test]
    fn curves_pass_through_their_points() {
        let pts = [p(0.0, 0.0), p(5.0, 3.0), p(10.0, 0.0)];
        let c = catmull_rom(&pts, false, 8);
        assert_eq!(c.len(), 17);
        assert!(c[0].approx_eq(pts[0]) && c[8].approx_eq(pts[1]) && c[16].approx_eq(pts[2]));
        let closed = catmull_rom(&pts, true, 4);
        assert_eq!(closed.len(), 12);
    }

    #[test]
    fn three_points_give_the_bulge_of_their_arc() {
        // A half circle counter-clockwise from (1,0) to (-1,0) through (0,1).
        assert!((bulge_through(p(1.0, 0.0), p(0.0, 1.0), p(-1.0, 0.0)) - 1.0).abs() < 1e-9);
        // The other way round: clockwise.
        assert!((bulge_through(p(-1.0, 0.0), p(0.0, 1.0), p(1.0, 0.0)) + 1.0).abs() < 1e-9);
        // In line: straight.
        assert_eq!(bulge_through(p(0.0, 0.0), p(1.0, 0.0), p(2.0, 0.0)), 0.0);
        let arcs = spline_arcs(&[p(0.0, 0.0), p(5.0, 3.0), p(10.0, 0.0)], false);
        assert_eq!(arcs.len(), 2);
        assert!(arcs.iter().all(|a| a.2.abs() > 0.0 && a.2.abs() < 1.0));
    }

    #[test]
    fn drawing_lines_are_picked_with_their_arcs() {
        let circle = picked_line(&Shape::Circle {
            centre: p(0.0, 0.0),
            radius: 2.0,
        })
        .unwrap();
        assert!(circle.closed);
        assert_eq!(circle.pieces.len(), 4);
        let area = archisim_core::signed_area(&circle.points).abs();
        assert!((area - std::f64::consts::PI * 4.0).abs() < 0.2, "{area}");
        let line = picked_line(&Shape::Line {
            a: p(0.0, 0.0),
            b: p(3.0, 0.0),
        })
        .unwrap();
        assert!(!line.closed);
        assert_eq!(line.pieces, [(p(0.0, 0.0), p(3.0, 0.0), 0.0)]);
        assert!(picked_line(&Shape::Point { at: p(0.0, 0.0) }).is_none());
    }
}
