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

#[cfg(test)]
mod tests {
    use super::*;

    fn p(x: f64, y: f64) -> Point2 {
        Point2::new(x, y)
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
