//! Minimal 2D math shared by every crate. Units are metres.

use std::ops::{Add, Mul, Neg, Sub};

/// Geometric tolerance in metres (0.01 mm).
pub const EPSILON: f64 = 1e-5;

/// A point or vector in the plan (XY) plane.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Point2 {
    pub x: f64,
    pub y: f64,
}

/// Alias used where a value is a direction or displacement, not a position.
pub type Vec2 = Point2;

impl Point2 {
    pub const fn new(x: f64, y: f64) -> Self {
        Self { x, y }
    }

    pub fn dot(self, o: Self) -> f64 {
        self.x * o.x + self.y * o.y
    }

    /// Z component of the 3D cross product.
    pub fn cross(self, o: Self) -> f64 {
        self.x * o.y - self.y * o.x
    }

    pub fn length(self) -> f64 {
        self.dot(self).sqrt()
    }

    pub fn distance(self, o: Self) -> f64 {
        (self - o).length()
    }

    /// Unit vector in the same direction, or `None` for a zero vector.
    pub fn normalized(self) -> Option<Self> {
        let l = self.length();
        (l > EPSILON).then(|| self * (1.0 / l))
    }

    /// The vector rotated 90° counter-clockwise (the "left" normal).
    pub fn perp_left(self) -> Self {
        Self::new(-self.y, self.x)
    }

    pub fn approx_eq(self, o: Self) -> bool {
        self.distance(o) <= EPSILON
    }
}

impl Add for Point2 {
    type Output = Self;
    fn add(self, o: Self) -> Self {
        Self::new(self.x + o.x, self.y + o.y)
    }
}

impl Sub for Point2 {
    type Output = Self;
    fn sub(self, o: Self) -> Self {
        Self::new(self.x - o.x, self.y - o.y)
    }
}

impl Mul<f64> for Point2 {
    type Output = Self;
    fn mul(self, k: f64) -> Self {
        Self::new(self.x * k, self.y * k)
    }
}

impl Neg for Point2 {
    type Output = Self;
    fn neg(self) -> Self {
        Self::new(-self.x, -self.y)
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

/// Perimeter of a closed polygon.
pub fn perimeter(poly: &[Point2]) -> f64 {
    let n = poly.len();
    (0..n).map(|i| poly[i].distance(poly[(i + 1) % n])).sum()
}

/// Whether two closed segments `a1-a2` and `b1-b2` touch or cross.
fn segments_touch(a1: Point2, a2: Point2, b1: Point2, b2: Point2) -> bool {
    let side = |p: Point2, q: Point2, r: Point2| (q - p).cross(r - p);
    let on_segment = |p: Point2, q: Point2, r: Point2| {
        r.x >= p.x.min(q.x) - EPSILON
            && r.x <= p.x.max(q.x) + EPSILON
            && r.y >= p.y.min(q.y) - EPSILON
            && r.y <= p.y.max(q.y) + EPSILON
    };
    let d1 = side(b1, b2, a1);
    let d2 = side(b1, b2, a2);
    let d3 = side(a1, a2, b1);
    let d4 = side(a1, a2, b2);
    let eps = EPSILON * EPSILON;
    if ((d1 > eps && d2 < -eps) || (d1 < -eps && d2 > eps))
        && ((d3 > eps && d4 < -eps) || (d3 < -eps && d4 > eps))
    {
        return true;
    }
    (d1.abs() <= eps && on_segment(b1, b2, a1))
        || (d2.abs() <= eps && on_segment(b1, b2, a2))
        || (d3.abs() <= eps && on_segment(a1, a2, b1))
        || (d4.abs() <= eps && on_segment(a1, a2, b2))
}

/// Whether a closed polygon is simple: no repeated points and no edge
/// touching or crossing a non-adjacent edge.
pub fn is_simple_polygon(poly: &[Point2]) -> bool {
    let n = poly.len();
    if n < 3 {
        return false;
    }
    for i in 0..n {
        if poly[i].approx_eq(poly[(i + 1) % n]) {
            return false;
        }
    }
    for i in 0..n {
        let (a1, a2) = (poly[i], poly[(i + 1) % n]);
        for j in (i + 1)..n {
            // Skip the edge itself and its neighbours (they share a vertex).
            if j == i || (j + 1) % n == i || (i + 1) % n == j {
                continue;
            }
            let (b1, b2) = (poly[j], poly[(j + 1) % n]);
            if segments_touch(a1, a2, b1, b2) {
                return false;
            }
        }
    }
    true
}

/// A circular arc from `start` to `end`, the shape of a curved wall's
/// drawn line.
///
/// It is described, as in DXF polylines, by its **bulge**: `tan(θ/4)`, where
/// `θ` is the signed angle it sweeps (positive counter-clockwise: the
/// centre is on the left of `start → end` and the arc bows out to the
/// right, as when walking a room anticlockwise). A bulge of 1 is a
/// half circle; 0 is a straight line. The bulge does not change when the
/// arc is moved, turned or scaled, which is what keeps a curved wall's
/// shape when one of its ends is stretched.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Arc {
    pub start: Point2,
    pub end: Point2,
    pub centre: Point2,
    pub radius: f64,
    /// Angle of `start` seen from the centre, in radians.
    pub start_angle: f64,
    /// Signed sweep, positive counter-clockwise; never zero.
    pub sweep: f64,
}

/// Bulges smaller than this are straight lines.
pub const MIN_BULGE: f64 = 1e-6;

impl Arc {
    /// The arc with that bulge, or `None` for a straight line (or two
    /// equal points).
    pub fn from_bulge(start: Point2, end: Point2, bulge: f64) -> Option<Self> {
        if !bulge.is_finite() || bulge.abs() < MIN_BULGE || start.approx_eq(end) {
            return None;
        }
        let sweep = 4.0 * bulge.atan();
        let chord = end - start;
        let c = chord.length();
        let radius = c / (2.0 * (sweep / 2.0).sin().abs());
        // The centre is on the perpendicular bisector, on the left for a
        // counter-clockwise sweep smaller than half a turn.
        let mid = (start + end) * 0.5;
        let n = chord.perp_left() * (1.0 / c);
        let d = radius * (sweep / 2.0).cos() * sweep.signum();
        let centre = mid + n * d;
        let v = start - centre;
        Some(Self {
            start,
            end,
            centre,
            radius,
            start_angle: v.y.atan2(v.x),
            sweep,
        })
    }

    /// The arc from `start` through `through` to `end`, as a bulge, or
    /// `None` if the three points are in line.
    pub fn bulge_through(start: Point2, through: Point2, end: Point2) -> Option<f64> {
        let a = through - start;
        let b = end - through;
        let cross = a.cross(b);
        if cross.abs() < EPSILON * EPSILON || start.approx_eq(end) {
            return None;
        }
        // Inscribed angles: the turn between the two chords at `through`
        // is half the angle the arc sweeps.
        let turn = cross.atan2(a.dot(b));
        let sweep = 2.0 * turn;
        Some((sweep / 4.0).tan())
    }

    /// Arc length.
    pub fn length(&self) -> f64 {
        self.radius * self.sweep.abs()
    }

    /// Angle (from the centre) of the point at fraction `f` of the arc.
    pub fn angle_at(&self, f: f64) -> f64 {
        self.start_angle + self.sweep * f
    }

    /// Unit vector from the centre towards angle `a`.
    pub fn radial(a: f64) -> Vec2 {
        Vec2::new(a.cos(), a.sin())
    }

    /// The point at fraction `f` of the arc (0 = start, 1 = end), moved
    /// `offset` to the left of the direction of travel (as wall faces are).
    pub fn point(&self, f: f64, offset: f64) -> Point2 {
        self.centre + Self::radial(self.angle_at(f)) * self.radius_at(offset)
    }

    /// Radius of the curve `offset` to the left of the arc.
    pub fn radius_at(&self, offset: f64) -> f64 {
        // Turning left, the left side is towards the centre.
        self.radius - offset * self.sweep.signum()
    }

    /// Unit tangent (direction of travel) at fraction `f`.
    pub fn tangent(&self, f: f64) -> Vec2 {
        let r = Self::radial(self.angle_at(f));
        if self.sweep > 0.0 {
            r.perp_left()
        } else {
            -r.perp_left()
        }
    }

    /// The fraction of the arc nearest to `p` (clamped to the arc), and the
    /// signed offset of `p` from it, left positive.
    pub fn project(&self, p: Point2) -> (f64, f64) {
        let v = p - self.centre;
        let a = v.y.atan2(v.x);
        let mut d = a - self.start_angle;
        // Bring the angle into the sweep's turning direction.
        let tau = std::f64::consts::TAU;
        if self.sweep > 0.0 {
            d = d.rem_euclid(tau);
        } else {
            d = -(-d).rem_euclid(tau);
        }
        let mut f = d / self.sweep;
        if !(0.0..=1.0).contains(&f) {
            // Outside the arc: the nearer end.
            let gap_end = (f - 1.0).abs();
            let other = (tau - d.abs()) / self.sweep.abs();
            f = if gap_end <= other { 1.0 } else { 0.0 };
        }
        let r = v.length();
        let offset = (self.radius - r) * self.sweep.signum();
        (f, offset)
    }

    /// Number of straight pieces that follow the curve `offset` from the
    /// arc to within `tolerance` metres.
    pub fn segments(&self, offset: f64, tolerance: f64) -> usize {
        let r = self.radius_at(offset).abs().max(EPSILON);
        let tol = tolerance.clamp(1e-6, r);
        // Sagitta of one piece: r(1 − cos(φ/2)) ≤ tol.
        let step = 2.0 * (1.0 - tol / r).clamp(-1.0, 1.0).acos();
        let n = (self.sweep.abs() / step.max(1e-3)).ceil() as usize;
        n.clamp(2, 720)
    }

    /// Points along the curve `offset` from the arc, from start to end,
    /// close enough to draw or extrude it.
    pub fn polyline(&self, offset: f64, tolerance: f64) -> Vec<Point2> {
        let n = self.segments(offset, tolerance);
        (0..=n)
            .map(|i| self.point(i as f64 / n as f64, offset))
            .collect()
    }
}

/// Tolerance for drawing curves with straight pieces: half a millimetre,
/// below what any plan or 3D view shows.
pub const CURVE_TOLERANCE: f64 = 0.0005;

#[cfg(test)]
mod tests {
    use super::*;

    fn p(x: f64, y: f64) -> Point2 {
        Point2::new(x, y)
    }

    fn close(a: f64, b: f64) -> bool {
        (a - b).abs() < 1e-9
    }

    #[test]
    fn half_circle_from_bulge() {
        // Bulge 1 turning left: centre midway, radius 2, bending to +y.
        let a = Arc::from_bulge(p(0.0, 0.0), p(4.0, 0.0), 1.0).unwrap();
        assert!(a.centre.approx_eq(p(2.0, 0.0)));
        assert!(close(a.radius, 2.0));
        assert!(close(a.length(), 2.0 * std::f64::consts::PI));
        // Counter-clockwise from (0,0) seen from (2,0) goes through (2,-2).
        assert!(a.point(0.5, 0.0).approx_eq(p(2.0, -2.0)));
        // Left of travel is towards the centre: a smaller radius.
        assert!(a.point(0.5, 0.5).approx_eq(p(2.0, -1.5)));
        assert!(a.end.approx_eq(a.point(1.0, 0.0)));
        assert!(a.tangent(0.0).approx_eq(p(0.0, -1.0)));
        // Bulge −1 is the mirror image.
        let b = Arc::from_bulge(p(0.0, 0.0), p(4.0, 0.0), -1.0).unwrap();
        assert!(b.point(0.5, 0.0).approx_eq(p(2.0, 2.0)));
        assert!(b.point(0.5, 0.5).approx_eq(p(2.0, 2.5)));
        assert!(Arc::from_bulge(p(0.0, 0.0), p(4.0, 0.0), 0.0).is_none());
    }

    #[test]
    fn bulge_through_three_points() {
        // A quarter circle of radius 1 around the origin.
        let s = std::f64::consts::FRAC_1_SQRT_2;
        let b = Arc::bulge_through(p(1.0, 0.0), p(s, s), p(0.0, 1.0)).unwrap();
        let a = Arc::from_bulge(p(1.0, 0.0), p(0.0, 1.0), b).unwrap();
        assert!(a.centre.approx_eq(p(0.0, 0.0)));
        assert!(close(a.sweep, std::f64::consts::FRAC_PI_2));
        // Three quarters, through the far side.
        let b = Arc::bulge_through(p(1.0, 0.0), p(-1.0, 0.0), p(0.0, -1.0)).unwrap();
        let a = Arc::from_bulge(p(1.0, 0.0), p(0.0, -1.0), b).unwrap();
        assert!(a.centre.approx_eq(p(0.0, 0.0)));
        assert!(close(a.sweep, 1.5 * std::f64::consts::PI));
        assert!(Arc::bulge_through(p(0.0, 0.0), p(1.0, 0.0), p(2.0, 0.0)).is_none());
    }

    #[test]
    fn project_onto_an_arc() {
        let a = Arc::from_bulge(p(0.0, 0.0), p(4.0, 0.0), -1.0).unwrap();
        let (f, off) = a.project(p(2.0, 3.0));
        assert!(close(f, 0.5));
        assert!(close(off, 1.0)); // outside a clockwise arc is on its left
        let (f, _) = a.project(p(-1.0, -0.5));
        assert!(close(f, 0.0));
        let (f, _) = a.project(p(5.0, -0.5));
        assert!(close(f, 1.0));
        let pts = a.polyline(0.0, CURVE_TOLERANCE);
        assert!(pts.len() > 20);
        for q in &pts {
            assert!(close(q.distance(a.centre), 2.0));
        }
    }

    #[test]
    fn area_and_perimeter() {
        let sq = [p(0.0, 0.0), p(4.0, 0.0), p(4.0, 3.0), p(0.0, 3.0)];
        assert_eq!(signed_area(&sq), 12.0);
        assert_eq!(perimeter(&sq), 14.0);
        let mut cw = sq;
        cw.reverse();
        assert_eq!(signed_area(&cw), -12.0);
    }

    #[test]
    fn simple_polygons() {
        let l_shape = [
            p(0.0, 0.0),
            p(6.0, 0.0),
            p(6.0, 3.0),
            p(3.0, 3.0),
            p(3.0, 6.0),
            p(0.0, 6.0),
        ];
        assert!(is_simple_polygon(&l_shape));
        // A bow tie crosses itself.
        let bow = [p(0.0, 0.0), p(2.0, 2.0), p(2.0, 0.0), p(0.0, 2.0)];
        assert!(!is_simple_polygon(&bow));
        // A spike that folds back onto an earlier edge.
        let folded = [p(0.0, 0.0), p(4.0, 0.0), p(4.0, 2.0), p(2.0, 0.0)];
        assert!(!is_simple_polygon(&folded));
        assert!(!is_simple_polygon(&[p(0.0, 0.0), p(1.0, 0.0)]));
        assert!(!is_simple_polygon(&[p(0.0, 0.0), p(0.0, 0.0), p(1.0, 1.0)]));
    }
}
