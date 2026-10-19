//! Straight and circular segments: the pieces every drafting shape is made
//! of for editing (polylines, circles, arcs), with their intersections.

use std::f64::consts::{PI, TAU};

use archisim_core::{Point2, Vec2};

/// Lengths below this are zero, in metres.
pub const TOL: f64 = 1e-9;

/// A straight segment or a circular arc.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Seg {
    Line {
        a: Point2,
        b: Point2,
    },
    /// From angle `start` around `centre` through the signed `sweep`
    /// (positive counter-clockwise). A full circle sweeps ±2π.
    Arc {
        centre: Point2,
        radius: f64,
        start: f64,
        sweep: f64,
    },
}

/// The unit vector at angle `a`.
pub fn dir(a: f64) -> Vec2 {
    Vec2::new(a.cos(), a.sin())
}

/// The angle of a vector.
pub fn angle_of(v: Vec2) -> f64 {
    v.y.atan2(v.x)
}

/// `a` brought into `[0, 2π)`.
pub fn norm_angle(a: f64) -> f64 {
    let r = a.rem_euclid(TAU);
    if r >= TAU - 1e-15 {
        0.0
    } else {
        r
    }
}

impl Seg {
    /// The segment from `a` to `b` with a DXF bulge (0: straight).
    pub fn from_bulge(a: Point2, b: Point2, bulge: f64) -> Seg {
        match archisim_core::Arc::from_bulge(a, b, bulge) {
            Some(arc) => Seg::Arc {
                centre: arc.centre,
                radius: arc.radius,
                start: arc.start_angle,
                sweep: arc.sweep,
            },
            None => Seg::Line { a, b },
        }
    }

    /// The bulge of the segment (0 for a line).
    pub fn bulge(&self) -> f64 {
        match self {
            Seg::Line { .. } => 0.0,
            Seg::Arc { sweep, .. } => (sweep / 4.0).tan(),
        }
    }

    pub fn is_full_circle(&self) -> bool {
        matches!(self, Seg::Arc { sweep, .. } if sweep.abs() >= TAU - 1e-9)
    }

    pub fn start(&self) -> Point2 {
        self.point_at(0.0)
    }

    pub fn end(&self) -> Point2 {
        self.point_at(1.0)
    }

    /// The point at fraction `t` (0 start, 1 end; also outside, on the
    /// line or circle it belongs to).
    pub fn point_at(&self, t: f64) -> Point2 {
        match *self {
            Seg::Line { a, b } => a + (b - a) * t,
            Seg::Arc {
                centre,
                radius,
                start,
                sweep,
            } => centre + dir(start + sweep * t) * radius,
        }
    }

    /// Unit direction of travel at fraction `t`.
    pub fn tangent(&self, t: f64) -> Vec2 {
        match *self {
            Seg::Line { a, b } => (b - a).normalized().unwrap_or(Vec2::new(1.0, 0.0)),
            Seg::Arc { start, sweep, .. } => {
                let r = dir(start + sweep * t);
                if sweep >= 0.0 {
                    r.perp_left()
                } else {
                    -r.perp_left()
                }
            }
        }
    }

    pub fn length(&self) -> f64 {
        match *self {
            Seg::Line { a, b } => a.distance(b),
            Seg::Arc { radius, sweep, .. } => radius * sweep.abs(),
        }
    }

    pub fn reversed(&self) -> Seg {
        match *self {
            Seg::Line { a, b } => Seg::Line { a: b, b: a },
            Seg::Arc {
                centre,
                radius,
                start,
                sweep,
            } => Seg::Arc {
                centre,
                radius,
                start: start + sweep,
                sweep: -sweep,
            },
        }
    }

    /// The part from fraction `t0` to `t1` (also beyond the ends: an
    /// extension).
    pub fn sub(&self, t0: f64, t1: f64) -> Seg {
        match *self {
            Seg::Line { .. } => Seg::Line {
                a: self.point_at(t0),
                b: self.point_at(t1),
            },
            Seg::Arc {
                centre,
                radius,
                start,
                sweep,
            } => Seg::Arc {
                centre,
                radius,
                start: start + sweep * t0,
                sweep: sweep * (t1 - t0),
            },
        }
    }

    /// The fraction of the point of the (infinite) line or the circle
    /// nearest to `p`. For arcs it is the fraction along the sweep's
    /// direction, in `[0, 2π/|sweep|)`; a point just before the start
    /// gives a value near that upper bound (see [`Seg::param_near`]).
    pub fn param(&self, p: Point2) -> f64 {
        match *self {
            Seg::Line { a, b } => {
                let d = b - a;
                let l2 = d.dot(d);
                if l2 < TOL * TOL {
                    0.0
                } else {
                    (p - a).dot(d) / l2
                }
            }
            Seg::Arc {
                centre,
                start,
                sweep,
                ..
            } => {
                let a = angle_of(p - centre);
                let d = if sweep >= 0.0 {
                    (a - start).rem_euclid(TAU)
                } else {
                    -(start - a).rem_euclid(TAU)
                };
                d / sweep
            }
        }
    }

    /// [`Seg::param`], choosing for arcs the turn of the circle that is
    /// closest to `[0, 1]` (so a point just before the start gives a small
    /// negative value).
    pub fn param_near(&self, p: Point2) -> f64 {
        let t = self.param(p);
        match *self {
            Seg::Line { .. } => t,
            Seg::Arc { sweep, .. } => {
                let turn = TAU / sweep.abs();
                let alt = t - turn;
                let dist = |v: f64| {
                    if (0.0..=1.0).contains(&v) {
                        0.0
                    } else if v < 0.0 {
                        -v
                    } else {
                        v - 1.0
                    }
                };
                if dist(alt) < dist(t) {
                    alt
                } else {
                    t
                }
            }
        }
    }

    /// The point of the segment (between its ends) nearest to `p`, and
    /// its fraction.
    pub fn nearest(&self, p: Point2) -> (f64, Point2) {
        let t = match self {
            Seg::Line { .. } => self.param(p).clamp(0.0, 1.0),
            Seg::Arc { .. } => {
                let t = self.param(p);
                if t <= 1.0 {
                    t
                } else {
                    // Outside the arc: the nearer end.
                    let s = self.start().distance(p);
                    let e = self.end().distance(p);
                    if s < e {
                        0.0
                    } else {
                        1.0
                    }
                }
            }
        };
        (t, self.point_at(t))
    }

    pub fn distance(&self, p: Point2) -> f64 {
        self.nearest(p).1.distance(p)
    }

    /// The middle of the segment.
    pub fn midpoint(&self) -> Point2 {
        self.point_at(0.5)
    }

    /// Points along the segment, within `tol` of it, from start to end.
    pub fn tessellate(&self, tol: f64) -> Vec<Point2> {
        match *self {
            Seg::Line { a, b } => vec![a, b],
            Seg::Arc { radius, sweep, .. } => {
                let tol = tol.clamp(1e-7, radius.max(1e-7));
                let step = 2.0 * (1.0 - tol / radius).clamp(-1.0, 1.0).acos();
                let n = (sweep.abs() / step.max(1e-4)).ceil().clamp(2.0, 2048.0) as usize;
                (0..=n)
                    .map(|i| self.point_at(i as f64 / n as f64))
                    .collect()
            }
        }
    }

    /// The segment moved `d` to the left of its direction of travel
    /// (parallel line, concentric arc). `None` if an arc would shrink to
    /// nothing.
    pub fn offset(&self, d: f64) -> Option<Seg> {
        match *self {
            Seg::Line { a, b } => {
                let n = (b - a).normalized()?.perp_left();
                Some(Seg::Line {
                    a: a + n * d,
                    b: b + n * d,
                })
            }
            Seg::Arc {
                centre,
                radius,
                start,
                sweep,
            } => {
                // Left of a counter-clockwise arc is towards the centre.
                let r = radius - d * sweep.signum();
                (r > TOL).then_some(Seg::Arc {
                    centre,
                    radius: r,
                    start,
                    sweep,
                })
            }
        }
    }

    /// Whether `t` is on the segment (within its ends, with a little slack
    /// in length).
    pub fn contains_param(&self, t: f64) -> bool {
        let slack = 1e-7 / self.length().max(1e-9);
        t >= -slack && t <= 1.0 + slack
    }
}

/// Intersections of two infinite lines (or line and circle, or two
/// circles) the segments belong to, as `(t on a, t on b, point)`. Arcs use
/// [`Seg::param_near`] parameters, so their extension beyond either end is
/// reported with values outside `[0, 1]`.
pub fn intersect_extended(a: &Seg, b: &Seg) -> Vec<(f64, f64, Point2)> {
    let pts: Vec<Point2> = match (a, b) {
        (Seg::Line { a: p1, b: p2 }, Seg::Line { a: q1, b: q2 }) => {
            let d1 = *p2 - *p1;
            let d2 = *q2 - *q1;
            let den = d1.cross(d2);
            if den.abs() < 1e-14 * d1.length().max(1.0) * d2.length().max(1.0) {
                Vec::new()
            } else {
                let t = (*q1 - *p1).cross(d2) / den;
                vec![*p1 + d1 * t]
            }
        }
        (Seg::Line { a: p1, b: p2 }, Seg::Arc { centre, radius, .. })
        | (Seg::Arc { centre, radius, .. }, Seg::Line { a: p1, b: p2 }) => {
            line_circle(*p1, *p2, *centre, *radius)
        }
        (
            Seg::Arc {
                centre: c1,
                radius: r1,
                ..
            },
            Seg::Arc {
                centre: c2,
                radius: r2,
                ..
            },
        ) => circle_circle(*c1, *r1, *c2, *r2),
    };
    pts.into_iter()
        .map(|p| (a.param_near(p), b.param_near(p), p))
        .collect()
}

/// Points where the segments themselves cross (within both).
pub fn intersect(a: &Seg, b: &Seg) -> Vec<(f64, f64, Point2)> {
    intersect_extended(a, b)
        .into_iter()
        .filter(|(ta, tb, _)| a.contains_param(*ta) && b.contains_param(*tb))
        .collect()
}

/// Points where the line through `p1`, `p2` meets a circle.
pub fn line_circle(p1: Point2, p2: Point2, c: Point2, r: f64) -> Vec<Point2> {
    let d = p2 - p1;
    let len = d.length();
    if len < TOL {
        return Vec::new();
    }
    let u = d * (1.0 / len);
    let f = (c - p1).dot(u);
    let foot = p1 + u * f;
    let h2 = r * r - foot.distance(c).powi(2);
    if h2 < -1e-12 * r.max(1.0) {
        Vec::new()
    } else if h2.abs() <= 1e-12 * r.max(1.0) {
        vec![foot]
    } else {
        let h = h2.sqrt();
        vec![foot - u * h, foot + u * h]
    }
}

/// Points where two circles meet.
pub fn circle_circle(c1: Point2, r1: f64, c2: Point2, r2: f64) -> Vec<Point2> {
    let d = c1.distance(c2);
    if d < TOL || d > r1 + r2 + 1e-12 || d < (r1 - r2).abs() - 1e-12 {
        return Vec::new();
    }
    let a = (r1 * r1 - r2 * r2 + d * d) / (2.0 * d);
    let h2 = r1 * r1 - a * a;
    let u = (c2 - c1) * (1.0 / d);
    let m = c1 + u * a;
    if h2 <= 1e-14 {
        vec![m]
    } else {
        let h = h2.sqrt();
        let n = u.perp_left();
        vec![m + n * h, m - n * h]
    }
}

/// Tangent points on a circle from an outside point.
pub fn tangent_points(p: Point2, c: Point2, r: f64) -> Vec<Point2> {
    let d = p.distance(c);
    if d <= r + TOL {
        return Vec::new();
    }
    // Angle at the centre between the centre→point line and the radius to
    // the tangent point.
    let a = (r / d).acos();
    let base = angle_of(p - c);
    vec![c + dir(base + a) * r, c + dir(base - a) * r]
}

/// Smallest angle between two directions, in `[0, π]`.
pub fn angle_between(a: Vec2, b: Vec2) -> f64 {
    let c = a.cross(b);
    let d = a.dot(b);
    c.atan2(d).abs().min(PI)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(x: f64, y: f64) -> Point2 {
        Point2::new(x, y)
    }

    #[test]
    fn line_line_crossing_and_extension() {
        let a = Seg::Line {
            a: p(0.0, 0.0),
            b: p(4.0, 0.0),
        };
        let b = Seg::Line {
            a: p(2.0, -1.0),
            b: p(2.0, 1.0),
        };
        let x = intersect(&a, &b);
        assert_eq!(x.len(), 1);
        assert!(x[0].2.approx_eq(p(2.0, 0.0)));
        assert!((x[0].0 - 0.5).abs() < 1e-12);
        let c = Seg::Line {
            a: p(6.0, 1.0),
            b: p(6.0, 2.0),
        };
        assert!(intersect(&a, &c).is_empty());
        let e = intersect_extended(&a, &c);
        assert!((e[0].0 - 1.5).abs() < 1e-12 && (e[0].1 + 1.0).abs() < 1e-12);
    }

    #[test]
    fn arcs_params_and_crossings() {
        // Quarter arc from +x to +y around the origin, radius 2.
        let arc = Seg::Arc {
            centre: p(0.0, 0.0),
            radius: 2.0,
            start: 0.0,
            sweep: PI / 2.0,
        };
        assert!(arc.end().approx_eq(p(0.0, 2.0)));
        let diag = Seg::Line {
            a: p(0.0, 0.0),
            b: p(3.0, 3.0),
        };
        let x = intersect(&arc, &diag);
        assert_eq!(x.len(), 1);
        assert!((x[0].0 - 0.5).abs() < 1e-12);
        // A point just below the start is a small negative fraction.
        assert!(arc.param_near(p(2.0, -0.1)) < 0.0);
        // Clockwise arcs too.
        let cw = arc.reversed();
        assert!(cw.start().approx_eq(p(0.0, 2.0)));
        assert!((cw.param(p(2.0, 0.0)) - 1.0).abs() < 1e-12);
        // Two circles.
        let pts = circle_circle(p(0.0, 0.0), 2.0, p(2.0, 0.0), 2.0);
        assert_eq!(pts.len(), 2);
        // Tangents from (4,0) to the unit circle.
        let t = tangent_points(p(4.0, 0.0), p(0.0, 0.0), 1.0);
        for q in t {
            assert!(((q - p(4.0, 0.0)).dot(q)).abs() < 1e-12);
        }
    }

    #[test]
    fn bulge_round_trip_and_offset() {
        let s = Seg::from_bulge(p(0.0, 0.0), p(4.0, 0.0), 1.0);
        assert!((s.bulge() - 1.0).abs() < 1e-12);
        assert!(s.end().approx_eq(p(4.0, 0.0)));
        assert!((s.length() - 2.0 * PI).abs() < 1e-12);
        // Counter-clockwise: left is inwards.
        let o = s.offset(0.5).unwrap();
        assert!(matches!(o, Seg::Arc { radius, .. } if (radius - 1.5).abs() < 1e-12));
        let l = Seg::Line {
            a: p(0.0, 0.0),
            b: p(1.0, 0.0),
        };
        assert!(l.offset(1.0).unwrap().start().approx_eq(p(0.0, 1.0)));
    }
}
