//! Small plan geometry for the load paths.

use archisim_core::{Point2, Vec2, Wall};

pub fn inside(p: Point2, poly: &[Point2]) -> bool {
    let n = poly.len();
    if n < 3 {
        return false;
    }
    let mut inside = false;
    for i in 0..n {
        let (a, b) = (poly[i], poly[(i + 1) % n]);
        if (a.y > p.y) != (b.y > p.y) && p.x < a.x + (b.x - a.x) * (p.y - a.y) / (b.y - a.y) {
            inside = !inside;
        }
    }
    inside
}

/// The distance from `p` to the segment `a`–`b` and where along it the
/// nearest point is (0 at `a`, 1 at `b`).
pub fn to_segment(p: Point2, a: Point2, b: Point2) -> (f64, f64) {
    let d = b - a;
    let l2 = d.dot(d);
    if l2 < 1e-12 {
        return (p.distance(a), 0.0);
    }
    let t = ((p - a).dot(d) / l2).clamp(0.0, 1.0);
    (p.distance(a + d * t), t)
}

pub fn near_polygon(p: Point2, poly: &[Point2], tol: f64) -> bool {
    inside(p, poly) || edge_distance(p, poly) <= tol
}

pub fn edge_distance(p: Point2, poly: &[Point2]) -> f64 {
    let n = poly.len();
    (0..n)
        .map(|i| to_segment(p, poly[i], poly[(i + 1) % n]).0)
        .fold(f64::INFINITY, f64::min)
}

pub fn area(poly: &[Point2]) -> f64 {
    let n = poly.len();
    (0..n)
        .map(|i| {
            let (a, b) = (poly[i], poly[(i + 1) % n]);
            a.x * b.y - b.x * a.y
        })
        .sum::<f64>()
        .abs()
        / 2.0
}

pub fn bounds(poly: &[Point2]) -> (Point2, Point2) {
    poly.iter().fold(
        (
            Point2::new(f64::INFINITY, f64::INFINITY),
            Point2::new(f64::NEG_INFINITY, f64::NEG_INFINITY),
        ),
        |(lo, hi), p| {
            (
                Point2::new(lo.x.min(p.x), lo.y.min(p.y)),
                Point2::new(hi.x.max(p.x), hi.y.max(p.y)),
            )
        },
    )
}

/// Points on a regular grid of `step` inside `outline` and out of
/// `holes`, each standing for a square of `step` × `step`.
pub fn grid(outline: &[Point2], holes: &[Vec<Point2>], step: f64) -> Vec<Point2> {
    let (lo, hi) = bounds(outline);
    let mut out = Vec::new();
    if !(lo.x.is_finite() && hi.x.is_finite()) {
        return out;
    }
    let nx = ((hi.x - lo.x) / step).ceil().max(1.0) as usize;
    let ny = ((hi.y - lo.y) / step).ceil().max(1.0) as usize;
    if nx * ny > 4_000_000 {
        return out;
    }
    for i in 0..nx {
        for j in 0..ny {
            let p = Point2::new(
                lo.x + (i as f64 + 0.5) * step,
                lo.y + (j as f64 + 0.5) * step,
            );
            if inside(p, outline) && !holes.iter().any(|h| inside(p, h)) {
                out.push(p);
            }
        }
    }
    out
}

/// The line of a wall in plan as a polyline (an arc in short chords).
pub fn wall_path(w: &Wall) -> Vec<Point2> {
    if w.bulge.abs() < 1e-9 {
        return vec![w.start, w.end];
    }
    let chord = w.end - w.start;
    let c = chord.length();
    if c < 1e-9 {
        return vec![w.start, w.end];
    }
    let theta = 4.0 * w.bulge.atan();
    let r = c / (2.0 * (theta / 2.0).sin().abs());
    // The centre is to the left of the chord for a left turn under half a
    // circle; the tangent's sign takes care of the rest.
    let mid = w.start + chord * 0.5;
    let left = Vec2::new(-chord.y / c, chord.x / c);
    let centre = mid + left * ((c / 2.0) / (theta / 2.0).tan());
    let a0 = (w.start.y - centre.y).atan2(w.start.x - centre.x);
    let n = ((theta.abs() * r) / 0.25).ceil().clamp(4.0, 200.0) as usize;
    (0..=n)
        .map(|i| {
            let a = a0 + theta * i as f64 / n as f64;
            Point2::new(centre.x + r * a.cos(), centre.y + r * a.sin())
        })
        .collect()
}

/// A polyline's length.
pub fn path_length(path: &[Point2]) -> f64 {
    path.windows(2).map(|w| w[0].distance(w[1])).sum()
}

/// The distance from `p` to a polyline and how far along it the nearest
/// point is.
pub fn to_path(p: Point2, path: &[Point2]) -> (f64, f64) {
    let mut best = (f64::INFINITY, 0.0);
    let mut run = 0.0;
    for w in path.windows(2) {
        let (d, t) = to_segment(p, w[0], w[1]);
        let l = w[0].distance(w[1]);
        if d < best.0 {
            best = (d, run + t * l);
        }
        run += l;
    }
    best
}

/// The point `s` metres along a polyline.
pub fn along_path(path: &[Point2], s: f64) -> Point2 {
    let mut run = 0.0;
    for w in path.windows(2) {
        let l = w[0].distance(w[1]);
        if run + l >= s && l > 0.0 {
            let t = ((s - run) / l).clamp(0.0, 1.0);
            return w[0] + (w[1] - w[0]) * t;
        }
        run += l;
    }
    *path.last().unwrap_or(&Point2::new(0.0, 0.0))
}

/// Where a ray from `p` along `d` first crosses the polyline, as the
/// distance along the ray and along the polyline.
pub fn ray_hit(p: Point2, d: Vec2, path: &[Point2], max: f64) -> Option<(f64, f64)> {
    let mut best: Option<(f64, f64)> = None;
    let mut run = 0.0;
    for w in path.windows(2) {
        let (a, b) = (w[0], w[1]);
        let e = b - a;
        let l = e.length();
        let den = d.x * e.y - d.y * e.x;
        if den.abs() > 1e-12 {
            let ap = a - p;
            let t = (ap.x * e.y - ap.y * e.x) / den;
            let u = (ap.x * d.y - ap.y * d.x) / den;
            if t > 1e-6
                && t <= max
                && (-1e-9..=1.0 + 1e-9).contains(&u)
                && best.is_none_or(|(bt, _)| t < bt)
            {
                best = Some((t, run + u.clamp(0.0, 1.0) * l));
            }
        }
        run += l;
    }
    best
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rays_and_grids() {
        let sq = vec![
            Point2::new(0.0, 0.0),
            Point2::new(2.0, 0.0),
            Point2::new(2.0, 2.0),
            Point2::new(0.0, 2.0),
        ];
        assert_eq!(grid(&sq, &[], 0.5).len(), 16);
        assert!((area(&sq) - 4.0).abs() < 1e-12);
        let line = vec![Point2::new(3.0, -1.0), Point2::new(3.0, 3.0)];
        let hit = ray_hit(Point2::new(1.0, 1.0), Vec2::new(1.0, 0.0), &line, 10.0).unwrap();
        assert!((hit.0 - 2.0).abs() < 1e-9 && (hit.1 - 2.0).abs() < 1e-9);
        assert!(ray_hit(Point2::new(1.0, 1.0), Vec2::new(-1.0, 0.0), &line, 10.0).is_none());
    }
}
