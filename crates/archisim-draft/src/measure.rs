//! Lengths, areas, bounds and distances of shapes (AutoCAD's DIST, AREA
//! and LIST, and picking with the mouse).

use std::f64::consts::{PI, TAU};

use archisim_core::{Point2, Polyline, Shape};

use crate::font::text_frame;
use crate::seg::Seg;
use crate::{ellipse_points, inside_loops, loop_polygon, polyline_segs, shape_segs, CURVE_TOL};

/// Length of a shape's outline (0 for points, texts and hatches; a closed
/// polyline counts its closing segment; a circle its circumference).
pub fn length(shape: &Shape) -> f64 {
    match shape {
        Shape::Ellipse {
            axis, ratio, sweep, ..
        } if *sweep >= TAU - 1e-9 => {
            // Ramanujan's approximation, exact enough for drawings.
            let a = axis.length();
            let b = a * ratio;
            let h = ((a - b) / (a + b)).powi(2);
            PI * (a + b) * (1.0 + 3.0 * h / (10.0 + (4.0 - 3.0 * h).sqrt()))
        }
        _ => shape_segs(shape).iter().map(Seg::length).sum(),
    }
}

/// Signed area enclosed by a closed polyline, arcs included (positive
/// counter-clockwise).
pub fn polyline_area(p: &Polyline) -> f64 {
    let segs = polyline_segs(p);
    let mut a = 0.0;
    for s in &segs {
        let (p0, p1) = (s.start(), s.end());
        a += p0.cross(p1) / 2.0;
        if let Seg::Arc { radius, sweep, .. } = s {
            // The circular segment between the chord and the arc.
            a += radius * radius / 2.0 * (sweep - sweep.sin());
        }
    }
    a
}

/// Area enclosed by a closed shape: closed polylines, circles, full
/// ellipses and hatches (holes taken out). `None` for open shapes.
pub fn area(shape: &Shape) -> Option<f64> {
    match shape {
        Shape::Polyline(p) if p.closed => Some(polyline_area(p).abs()),
        Shape::Circle { radius, .. } => Some(PI * radius * radius),
        Shape::Ellipse {
            axis, ratio, sweep, ..
        } if *sweep >= TAU - 1e-9 => {
            let a = axis.length();
            Some(PI * a * a * ratio)
        }
        Shape::Hatch(h) => {
            // Even-odd: a loop inside an odd number of others is a hole.
            let polys: Vec<Vec<Point2>> =
                h.loops.iter().map(|l| loop_polygon(l, CURVE_TOL)).collect();
            let mut total = 0.0;
            for (i, l) in h.loops.iter().enumerate() {
                let probe = polys[i][0];
                let depth = polys
                    .iter()
                    .enumerate()
                    .filter(|(j, q)| *j != i && inside_loops(std::slice::from_ref(*q), probe))
                    .count();
                let a = polyline_area(l).abs();
                total += if depth.is_multiple_of(2) { a } else { -a };
            }
            Some(total.abs())
        }
        _ => None,
    }
}

/// Lower-left and upper-right corners of a shape. `k`: drawing units per
/// paper millimetre, for texts.
pub fn bounds(shape: &Shape, k: f64) -> (Point2, Point2) {
    let pts: Vec<Point2> = match shape {
        Shape::Text(t) => text_frame(t, k).to_vec(),
        Shape::Point { at } => vec![*at],
        Shape::Ellipse {
            centre,
            axis,
            ratio,
            start,
            sweep,
        } => ellipse_points(*centre, *axis, *ratio, *start, *sweep, CURVE_TOL * 10.0),
        Shape::Hatch(h) => h
            .loops
            .iter()
            .flat_map(|l| loop_polygon(l, CURVE_TOL * 10.0))
            .collect(),
        _ => shape_segs(shape)
            .iter()
            .flat_map(|s| s.tessellate(CURVE_TOL * 10.0))
            .collect(),
    };
    let mut lo = Point2::new(f64::INFINITY, f64::INFINITY);
    let mut hi = Point2::new(f64::NEG_INFINITY, f64::NEG_INFINITY);
    for p in pts {
        lo = Point2::new(lo.x.min(p.x), lo.y.min(p.y));
        hi = Point2::new(hi.x.max(p.x), hi.y.max(p.y));
    }
    (lo, hi)
}

/// Distance from `p` to what is drawn of a shape: its lines, the box of a
/// text, and 0 inside a hatch.
pub fn distance(shape: &Shape, p: Point2, k: f64) -> f64 {
    match shape {
        Shape::Point { at } => at.distance(p),
        Shape::Text(t) => {
            let f = text_frame(t, k);
            if inside_loops(&[f.to_vec()], p) {
                0.0
            } else {
                (0..4)
                    .map(|i| {
                        Seg::Line {
                            a: f[i],
                            b: f[(i + 1) % 4],
                        }
                        .distance(p)
                    })
                    .fold(f64::INFINITY, f64::min)
            }
        }
        Shape::Hatch(h) => {
            let polys: Vec<Vec<Point2>> = h
                .loops
                .iter()
                .map(|l| loop_polygon(l, CURVE_TOL * 10.0))
                .collect();
            if inside_loops(&polys, p) {
                0.0
            } else {
                h.loops
                    .iter()
                    .flat_map(polyline_segs)
                    .map(|s| s.distance(p))
                    .fold(f64::INFINITY, f64::min)
            }
        }
        _ => shape_segs(shape)
            .iter()
            .map(|s| s.distance(p))
            .fold(f64::INFINITY, f64::min),
    }
}

/// Whether a shape lies entirely inside a window (selection from left to
/// right).
pub fn inside_window(shape: &Shape, lo: Point2, hi: Point2, k: f64) -> bool {
    let (a, b) = bounds(shape, k);
    a.x >= lo.x && a.y >= lo.y && b.x <= hi.x && b.y <= hi.y
}

/// Whether a shape touches a window at all (crossing selection, from right
/// to left).
pub fn crosses_window(shape: &Shape, lo: Point2, hi: Point2, k: f64) -> bool {
    let (a, b) = bounds(shape, k);
    if a.x > hi.x || a.y > hi.y || b.x < lo.x || b.y < lo.y {
        return false;
    }
    if inside_window(shape, lo, hi, k) {
        return true;
    }
    let rect = [lo, Point2::new(hi.x, lo.y), hi, Point2::new(lo.x, hi.y)];
    let inside = |p: Point2| p.x >= lo.x && p.x <= hi.x && p.y >= lo.y && p.y <= hi.y;
    match shape {
        Shape::Text(_) | Shape::Point { .. } => true,
        Shape::Hatch(h) => {
            // Inside the hatch, or an edge crossing the window.
            let polys: Vec<Vec<Point2>> = h
                .loops
                .iter()
                .map(|l| loop_polygon(l, CURVE_TOL * 10.0))
                .collect();
            if rect.iter().any(|c| inside_loops(&polys, *c)) {
                return true;
            }
            polys.iter().flatten().any(|p| inside(*p))
                || segs_cross_rect(
                    &h.loops.iter().flat_map(polyline_segs).collect::<Vec<_>>(),
                    &rect,
                )
        }
        _ => {
            let segs = shape_segs(shape);
            segs.iter()
                .any(|s| s.tessellate(CURVE_TOL * 10.0).into_iter().any(inside))
                || segs_cross_rect(&segs, &rect)
        }
    }
}

fn segs_cross_rect(segs: &[Seg], rect: &[Point2; 4]) -> bool {
    let edges: Vec<Seg> = (0..4)
        .map(|i| Seg::Line {
            a: rect[i],
            b: rect[(i + 1) % 4],
        })
        .collect();
    segs.iter().any(|s| {
        edges
            .iter()
            .any(|e| !crate::seg::intersect(s, e).is_empty())
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(x: f64, y: f64) -> Point2 {
        Point2::new(x, y)
    }

    #[test]
    fn areas_with_arcs_and_holes() {
        // A 4 × 2 rectangle with a half circle of radius 1 bulging out of
        // its top side.
        let r = Polyline {
            points: vec![
                p(0.0, 0.0),
                p(4.0, 0.0),
                p(4.0, 2.0),
                p(2.0, 2.0),
                p(0.0, 2.0),
            ],
            bulges: vec![0.0, 0.0, 0.0, 0.0, 0.0],
            closed: true,
            width: 0.0,
        };
        assert!((polyline_area(&r) - 8.0).abs() < 1e-12);
        let mut half = r.clone();
        // From (4,2) to (0,2) turning left (counter-clockwise): it bulges
        // up, out of the rectangle... with radius 2 here.
        half.points = vec![p(0.0, 0.0), p(4.0, 0.0), p(4.0, 2.0), p(0.0, 2.0)];
        half.bulges = vec![0.0, 0.0, 1.0, 0.0];
        assert!((polyline_area(&half) - (8.0 + PI * 2.0)).abs() < 1e-9);
        let hatch = Shape::Hatch(archisim_core::Hatch {
            loops: vec![
                r.clone(),
                Polyline::straight(
                    vec![p(1.0, 0.5), p(2.0, 0.5), p(2.0, 1.5), p(1.0, 1.5)],
                    true,
                ),
            ],
            pattern: archisim_core::HatchPattern::Solid,
            scale: 1.0,
            angle: 0.0,
            background: None,
        });
        assert!((area(&hatch).unwrap() - 7.0).abs() < 1e-9);
        assert!(
            (area(&Shape::Circle {
                centre: p(0.0, 0.0),
                radius: 2.0
            })
            .unwrap()
                - 4.0 * PI)
                .abs()
                < 1e-12
        );
    }

    #[test]
    fn lengths_and_distances() {
        let c = Shape::Circle {
            centre: p(0.0, 0.0),
            radius: 1.0,
        };
        assert!((length(&c) - TAU).abs() < 1e-12);
        assert!((distance(&c, p(3.0, 0.0), 0.05) - 2.0).abs() < 1e-12);
        let e = Shape::Ellipse {
            centre: p(0.0, 0.0),
            axis: p(2.0, 0.0),
            ratio: 1.0,
            start: 0.0,
            sweep: TAU,
        };
        assert!((length(&e) - 4.0 * PI).abs() < 1e-9);
        let l = Shape::Line {
            a: p(0.0, 0.0),
            b: p(2.0, 0.0),
        };
        assert!(crosses_window(&l, p(1.0, -1.0), p(1.5, 1.0), 0.05));
        assert!(!inside_window(&l, p(1.0, -1.0), p(1.5, 1.0), 0.05));
        assert!(inside_window(&l, p(-1.0, -1.0), p(3.0, 1.0), 0.05));
    }
}
