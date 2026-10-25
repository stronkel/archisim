//! 2D drafting geometry, as AutoCAD users expect it: the segments drafting
//! shapes are made of, moving, turning, scaling and mirroring them, the
//! editing commands (offset, trim, extend, fillet, chamfer, break, join,
//! explode, arrays, stretch), measurements, object snap points, hatches
//! and closed boundaries found from a point inside them.
//!
//! Everything here works on [`archisim_core::Shape`]s and knows nothing of
//! the model, views or the screen. Text and hatch sizes are given on paper
//! (millimetres) and turned into model metres with the view's scale: `k` is
//! always metres of the drawing per millimetre of paper (`scale / 1000`).

pub mod dims;
pub mod font;
pub mod grips;
pub mod hatch;
pub mod measure;
pub mod ops;
pub mod outline;
pub mod region;
pub mod seg;
pub mod snaps;
pub mod textures;
pub mod transform;

pub use measure::*;
pub use ops::*;
pub use seg::{intersect, intersect_extended, Seg, TOL};
pub use snaps::{snap_points, SnapPoint};
pub use transform::{transform_shape, Transform};

use std::f64::consts::TAU;

use archisim_core::{Point2, Polyline, Shape};

/// Tolerance for turning curves into straight pieces when an exact curve
/// is not needed (ellipses, boundaries): a tenth of a millimetre.
pub const CURVE_TOL: f64 = 1e-4;

/// The segments of a polyline, in order.
pub fn polyline_segs(p: &Polyline) -> Vec<Seg> {
    (0..p.segment_count())
        .map(|i| {
            let (a, b, bulge) = p.segment(i);
            Seg::from_bulge(a, b, bulge)
        })
        .filter(|s| s.length() > TOL)
        .collect()
}

/// A polyline made of segments that follow one another (each starts where
/// the previous one ends).
pub fn polyline_from_segs(segs: &[Seg], closed: bool, width: f64) -> Option<Polyline> {
    let first = segs.first()?;
    let mut points = vec![first.start()];
    let mut bulges = Vec::new();
    for s in segs {
        bulges.push(s.bulge());
        points.push(s.end());
    }
    if closed {
        points.pop();
    } else {
        bulges.push(0.0);
    }
    Some(Polyline {
        points,
        bulges,
        closed,
        width,
    })
}

/// Points of a full or partial ellipse.
pub fn ellipse_points(
    centre: Point2,
    axis: Point2,
    ratio: f64,
    start: f64,
    sweep: f64,
    tol: f64,
) -> Vec<Point2> {
    let a = axis.length();
    let u = axis * (1.0 / a.max(1e-12));
    let v = u.perp_left();
    let b = a * ratio;
    let step = 2.0 * (1.0 - tol.clamp(1e-7, a) / a).clamp(-1.0, 1.0).acos();
    let n = (sweep.abs() / step.max(1e-3)).ceil().clamp(8.0, 4096.0) as usize;
    (0..=n)
        .map(|i| {
            let t = start + sweep * i as f64 / n as f64;
            centre + u * (a * t.cos()) + v * (b * t.sin())
        })
        .collect()
}

/// The point of an ellipse at parameter `t`.
pub fn ellipse_point(centre: Point2, axis: Point2, ratio: f64, t: f64) -> Point2 {
    let a = axis.length();
    let u = axis * (1.0 / a.max(1e-12));
    let v = u.perp_left();
    centre + u * (a * t.cos()) + v * (a * ratio * t.sin())
}

/// The parameter of an ellipse at the point `p` (exact for points on it).
pub fn ellipse_param(centre: Point2, axis: Point2, ratio: f64, p: Point2) -> f64 {
    let a = axis.length();
    let u = axis * (1.0 / a.max(1e-12));
    let v = u.perp_left();
    let d = p - centre;
    let x = d.dot(u) / a;
    let y = d.dot(v) / (a * ratio);
    y.atan2(x)
}

/// The segments a shape is drawn with: exact lines and arcs, and straight
/// pieces along ellipses. Points, texts and hatch fills have none (a
/// hatch's boundary is given by [`boundary_segs`]).
pub fn shape_segs(shape: &Shape) -> Vec<Seg> {
    match shape {
        Shape::Line { a, b } => vec![Seg::Line { a: *a, b: *b }],
        Shape::Polyline(p) => polyline_segs(p),
        Shape::Circle { centre, radius } => vec![Seg::Arc {
            centre: *centre,
            radius: *radius,
            start: 0.0,
            sweep: TAU,
        }],
        Shape::Arc {
            centre,
            radius,
            start,
            sweep,
        } => vec![Seg::Arc {
            centre: *centre,
            radius: *radius,
            start: *start,
            sweep: *sweep,
        }],
        Shape::Ellipse {
            centre,
            axis,
            ratio,
            start,
            sweep,
        } => {
            let pts = ellipse_points(*centre, *axis, *ratio, *start, *sweep, CURVE_TOL);
            pts.windows(2)
                .map(|w| Seg::Line { a: w[0], b: w[1] })
                .collect()
        }
        Shape::Point { .. } | Shape::Text(_) | Shape::Hatch(_) => Vec::new(),
        Shape::Dim(m) => dims::main_segs(m),
    }
}

/// The boundary segments of a hatch.
pub fn boundary_segs(shape: &Shape) -> Vec<Seg> {
    match shape {
        Shape::Hatch(h) => h.loops.iter().flat_map(polyline_segs).collect(),
        other => shape_segs(other),
    }
}

/// A closed loop as a polygon (curves in straight pieces).
pub fn loop_polygon(p: &Polyline, tol: f64) -> Vec<Point2> {
    let mut out: Vec<Point2> = Vec::new();
    for s in polyline_segs(p) {
        let pts = s.tessellate(tol);
        out.extend_from_slice(&pts[..pts.len() - 1]);
    }
    if !p.closed {
        if let Some(last) = p.points.last() {
            out.push(*last);
        }
    }
    out
}

/// Whether a point is inside a set of loops (even-odd).
pub fn inside_loops(loops: &[Vec<Point2>], p: Point2) -> bool {
    let mut inside = false;
    for poly in loops {
        let n = poly.len();
        for i in 0..n {
            let (a, b) = (poly[i], poly[(i + 1) % n]);
            if (a.y > p.y) != (b.y > p.y) {
                let x = a.x + (p.y - a.y) / (b.y - a.y) * (b.x - a.x);
                if p.x < x {
                    inside = !inside;
                }
            }
        }
    }
    inside
}
