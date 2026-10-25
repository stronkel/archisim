//! Moving things near walls (C3): what is dragged is pulled to touch the
//! face of a wall when it comes near it (and then to a second face, in a
//! corner), and while it moves its clear distances to the walls around it
//! are measured, as the temporary dimensions of the plan. No screen here:
//! the 3D view and the plan draw what these give.

use std::collections::BTreeSet;

use archisim_core::{LevelId, Point2};

use crate::geo::Geo;
use crate::pick::Element;

/// How near a face what is dragged is pulled to touch it, in metres.
pub const PULL: f64 = 0.15;

/// Farther than this, a clear distance is not shown.
pub const REACH: f64 = 12.0;

/// A wall face: a straight side of a wall's outline, from `a` to `b`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Face {
    pub a: Point2,
    pub b: Point2,
}

/// The faces of the straight walls of `level`, but those in `skip`.
pub fn faces<G: Geo + ?Sized>(geo: &G, level: LevelId, skip: &BTreeSet<Element>) -> Vec<Face> {
    let model = geo.model();
    let mut out = Vec::new();
    for w in model.walls_on_level(level) {
        if w.is_curved() || skip.contains(&Element::Wall(w.id)) {
            continue;
        }
        let Some(g) = geo.wall_geometry(w.id) else {
            continue;
        };
        let n = g.outline.len();
        for i in 0..n {
            let (a, b) = (g.outline[i], g.outline[(i + 1) % n]);
            if a.distance(b) > 0.02 {
                out.push(Face { a, b });
            }
        }
    }
    out
}

/// How far `pts` are from the line of `f` along its normal, if they are
/// all on one side of it and face it (their shadow on the face overlaps
/// it): the gap and the normal pointing to them.
fn gap_to(f: &Face, pts: &[Point2]) -> Option<(f64, Point2)> {
    let len = f.a.distance(f.b);
    let d = (f.b - f.a) * (1.0 / len);
    let n = d.perp_left();
    let (mut lo, mut hi) = (f64::MAX, f64::MIN);
    let (mut s_lo, mut s_hi) = (f64::MAX, f64::MIN);
    for p in pts {
        let t = (*p - f.a).dot(d);
        let s = (*p - f.a).dot(n);
        lo = lo.min(t);
        hi = hi.max(t);
        s_lo = s_lo.min(s);
        s_hi = s_hi.max(s);
    }
    // They must face the face, by a centimetre at least.
    if hi.min(len) - lo.max(0.0) < 0.01 {
        return None;
    }
    const INSIDE: f64 = 0.02;
    if s_lo >= -INSIDE {
        Some((s_lo, n))
    } else if s_hi <= INSIDE {
        Some((-s_hi, -n))
    } else {
        None
    }
}

/// The move `delta` of the plan points `pts`, changed so that they touch
/// the faces they come within [`PULL`] of: the nearest, and then one
/// across it (a corner).
pub fn snap_delta(faces: &[Face], pts: &[Point2], delta: Point2) -> Point2 {
    if pts.is_empty() {
        return delta;
    }
    let moved = |d: Point2| -> Vec<Point2> { pts.iter().map(|p| *p + d).collect() };
    let best = |d: Point2, across: Option<Point2>| -> Option<(f64, Point2)> {
        let m = moved(d);
        faces
            .iter()
            .filter_map(|f| gap_to(f, &m))
            .filter(|(g, n)| *g <= PULL && across.is_none_or(|a| a.dot(*n).abs() < 0.5))
            .min_by(|a, b| a.0.abs().total_cmp(&b.0.abs()))
    };
    let Some((g1, n1)) = best(delta, None) else {
        return delta;
    };
    let d1 = delta - n1 * g1;
    match best(d1, Some(n1)) {
        Some((g2, n2)) => d1 - n2 * g2,
        None => d1,
    }
}

/// A clear distance from what is moved to a wall face.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Clearance {
    /// On the outline of what is moved.
    pub from: Point2,
    /// On the face.
    pub to: Point2,
}

impl Clearance {
    pub fn value(&self) -> f64 {
        self.from.distance(self.to)
    }
}

/// The clear distances from the plan outline `pts` to the nearest face in
/// each of the four directions of its axes `u` and `u.perp_left()`,
/// measured from the middle of its box.
pub fn clearances(faces: &[Face], pts: &[Point2], u: Point2) -> Vec<Clearance> {
    if pts.len() < 2 {
        return Vec::new();
    }
    let v = u.perp_left();
    let (mut lo, mut hi) = ([f64::MAX; 2], [f64::MIN; 2]);
    for p in pts {
        let (x, y) = (p.dot(u), p.dot(v));
        lo = [lo[0].min(x), lo[1].min(y)];
        hi = [hi[0].max(x), hi[1].max(y)];
    }
    let c = u * ((lo[0] + hi[0]) / 2.0) + v * ((lo[1] + hi[1]) / 2.0);
    let half = [(hi[0] - lo[0]) / 2.0, (hi[1] - lo[1]) / 2.0];
    let mut out = Vec::new();
    for (dir, e) in [(u, half[0]), (-u, half[0]), (v, half[1]), (-v, half[1])] {
        let near = faces
            .iter()
            .filter_map(|f| ray_hits(c, dir, f.a, f.b))
            .filter(|t| *t >= e - 0.02)
            .min_by(f64::total_cmp);
        if let Some(t) = near.filter(|t| t - e < REACH) {
            out.push(Clearance {
                from: c + dir * e,
                to: c + dir * t,
            });
        }
    }
    out
}

/// Where the ray from `p` along `dir` crosses the segment `a`–`b`.
fn ray_hits(p: Point2, dir: Point2, a: Point2, b: Point2) -> Option<f64> {
    let e = b - a;
    let den = dir.x * e.y - dir.y * e.x;
    if den.abs() < 1e-12 {
        return None;
    }
    let w = a - p;
    let t = (w.x * e.y - w.y * e.x) / den;
    let s = (w.x * dir.y - w.y * dir.x) / den;
    ((0.0..=1.0).contains(&s) && t > 0.0).then_some(t)
}

/// The plan points of what is selected (to pull and measure them), or
/// none if it is something that moves with its own rules (walls and
/// what is in them).
pub fn selection_points<G: Geo + ?Sized>(geo: &G, sel: &BTreeSet<Element>) -> Vec<Point2> {
    if sel
        .iter()
        .any(|e| matches!(e, Element::Wall(_) | Element::Opening(_)))
    {
        return Vec::new();
    }
    sel.iter()
        .filter_map(|e| crate::pick::outline(geo, *e))
        .flatten()
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pick::tests::room;

    /// A 1 × 0.5 m box with its lower left corner at `at`.
    fn box_at(at: Point2) -> Vec<Point2> {
        vec![
            at,
            at + Point2::new(1.0, 0.0),
            at + Point2::new(1.0, 0.5),
            at + Point2::new(0.0, 0.5),
        ]
    }

    #[test]
    fn pulled_to_a_face_and_into_a_corner() {
        // Inside the 6 × 4 m room: the inner faces at x 0.265 and 5.735,
        // y 0.265 and 3.735.
        let (m, lvl, _) = room();
        let f = faces(&m, lvl, &BTreeSet::new());
        let pts = box_at(Point2::new(2.0, 2.0));
        // Near the north face (its top 10 cm from it): it touches it.
        let d = snap_delta(&f, &pts, Point2::new(0.0, 1.135));
        assert!((d.y - 1.235).abs() < 1e-9 && d.x.abs() < 1e-9, "{d:?}");
        // Near the north face and the west one: into the corner.
        let d = snap_delta(&f, &pts, Point2::new(-1.65, 1.135));
        assert!(
            (d.x + 1.735).abs() < 1e-9 && (d.y - 1.235).abs() < 1e-9,
            "{d:?}"
        );
        // Far from both: as it is.
        let d = snap_delta(&f, &pts, Point2::new(0.5, 0.5));
        assert_eq!(d, Point2::new(0.5, 0.5));
        // Its clear distances to the four walls.
        let c = clearances(&f, &pts, Point2::new(1.0, 0.0));
        let mut v: Vec<f64> = c
            .iter()
            .map(|c| (c.value() * 1000.0).round() / 1000.0)
            .collect();
        v.sort_by(f64::total_cmp);
        assert_eq!(v, [1.235, 1.735, 1.735, 2.735]);
    }
}
