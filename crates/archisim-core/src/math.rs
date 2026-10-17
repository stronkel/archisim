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
