//! The crown's envelope: the space the branches fill and the leaves
//! clothe, given by the crown shape, the plant's size and a few waves
//! that make each plant lopsided in its own way.

use crate::rng::Rng;
use crate::CrownShape;
use std::f64::consts::{PI, TAU};

#[derive(Debug, Clone)]
pub struct Envelope {
    /// The crown's axis in plan.
    pub center: [f64; 2],
    /// Bottom and top of the crown.
    pub z0: f64,
    pub z1: f64,
    /// Widest radius.
    pub r: f64,
    shape: CrownShape,
    /// Largest value of the profile, to make it 1.
    peak: f64,
    /// Waves around the axis: (order, amount, phase).
    waves: Vec<(f64, f64, f64)>,
    /// Waves up the axis, so the lumps are not columns.
    vwaves: Vec<(f64, f64, f64)>,
}

/// The crown's radius at a height, from 0 (bottom) to 1 (top), as a share
/// of its widest radius (before making the peak 1).
fn profile(shape: CrownShape, t: f64) -> f64 {
    if !(0.0..=1.0).contains(&t) {
        return 0.0;
    }
    let ell = |c: f64, lo: f64, hi: f64| {
        // Half ellipse with its widest at `c`, ending at 0 and 1.
        let u = if t < c { (c - t) / lo } else { (t - c) / hi };
        (1.0 - u * u).max(0.0).sqrt()
    };
    match shape {
        CrownShape::Round | CrownShape::Irregular => ell(0.5, 0.5, 0.5),
        CrownShape::Oval => ell(0.45, 0.45, 0.55),
        CrownShape::Weeping => ell(0.42, 0.42, 0.58),
        CrownShape::Umbrella => {
            // A flat underside that opens fast, then a low dome.
            let base = (t / 0.22).min(1.0).powf(0.45);
            base * (1.0 - t.powf(2.4)).max(0.0).sqrt()
        }
        CrownShape::Spreading => ell(0.32, 0.32, 0.68),
        CrownShape::Columnar => {
            let base = (t / 0.18).min(1.0).sqrt();
            base * (1.0 - t).max(0.0).powf(0.75)
        }
        CrownShape::Conical => {
            let base = (t / 0.08).min(1.0).sqrt();
            base * (1.0 - t).max(0.0).powf(0.95)
        }
        CrownShape::Vase => (0.32 + 0.68 * t.sqrt()) * (1.0 - t.powi(5)).max(0.0).sqrt(),
        CrownShape::Mound => {
            let base = 0.78 + 0.22 * (t / 0.2).min(1.0);
            base * (1.0 - t * t).max(0.0).sqrt()
        }
        CrownShape::Cushion => {
            let base = 0.85 + 0.15 * (t / 0.3).min(1.0);
            base * (1.0 - t.powi(4)).max(0.0).sqrt()
        }
    }
}

impl Envelope {
    pub fn new(
        shape: CrownShape,
        center: [f64; 2],
        z0: f64,
        z1: f64,
        r: f64,
        variation: f64,
        rng: &mut Rng,
    ) -> Self {
        let peak = (0..=200)
            .map(|i| profile(shape, i as f64 / 200.0))
            .fold(0.0_f64, f64::max)
            .max(1e-6);
        let lumpy = match shape {
            CrownShape::Irregular => 2.6,
            CrownShape::Columnar | CrownShape::Conical => 0.45,
            CrownShape::Cushion => 0.6,
            _ => 1.0,
        };
        let a = variation * lumpy;
        let waves = vec![
            (1.0, a * 0.10 * rng.f(), rng.range(0.0, TAU)),
            (2.0, a * 0.09 * rng.f(), rng.range(0.0, TAU)),
            (3.0, a * 0.07 * rng.f(), rng.range(0.0, TAU)),
            (5.0, a * 0.05 * rng.f(), rng.range(0.0, TAU)),
        ];
        let vwaves = vec![
            (2.0, a * 0.06 * rng.f(), rng.range(0.0, TAU)),
            (3.0, a * 0.05 * rng.f(), rng.range(0.0, TAU)),
        ];
        Envelope {
            center,
            z0,
            z1: z1.max(z0 + 1e-3),
            r: r.max(1e-3),
            shape,
            peak,
            waves,
            vwaves,
        }
    }

    /// Share of the crown's height of a point, from 0 (bottom) to 1.
    pub fn t(&self, z: f64) -> f64 {
        (z - self.z0) / (self.z1 - self.z0)
    }

    /// Radius of the crown at a height and an azimuth.
    pub fn radius(&self, z: f64, az: f64) -> f64 {
        let t = self.t(z);
        let p = profile(self.shape, t) / self.peak;
        if p <= 0.0 {
            return 0.0;
        }
        let mut k = 1.0;
        for &(n, a, ph) in &self.waves {
            k += a * (n * az + ph).cos();
        }
        for &(n, a, ph) in &self.vwaves {
            k += a * (n * PI * t + ph + az).sin();
        }
        self.r * p * k.max(0.35)
    }

    fn polar(&self, p: [f64; 3]) -> (f64, f64) {
        let dx = p[0] - self.center[0];
        let dy = p[1] - self.center[1];
        ((dx * dx + dy * dy).sqrt(), dy.atan2(dx))
    }

    /// How deep in the crown a point is: 0 on its skin (or outside), 1 on
    /// its axis.
    pub fn depth(&self, p: [f64; 3]) -> f64 {
        let (d, az) = self.polar(p);
        let r = self.radius(p[2], az);
        if r <= 1e-9 {
            return 0.0;
        }
        (1.0 - d / r).clamp(0.0, 1.0)
    }

    pub fn inside(&self, p: [f64; 3]) -> bool {
        let (d, az) = self.polar(p);
        d <= self.radius(p[2], az)
    }

    /// How far a ray from a point inside the crown goes before leaving it.
    pub fn ray(&self, p: [f64; 3], dir: [f64; 3]) -> f64 {
        let span = (self.r * 2.0).max(self.z1 - self.z0) * 1.2;
        let steps = 48;
        let step = span / steps as f64;
        let mut last = 0.0;
        for i in 1..=steps {
            let s = step * i as f64;
            let q = [p[0] + dir[0] * s, p[1] + dir[1] * s, p[2] + dir[2] * s];
            if !self.inside(q) {
                // Halve the last step to find the skin.
                let (mut a, mut b) = (last, s);
                for _ in 0..6 {
                    let m = (a + b) / 2.0;
                    let q = [p[0] + dir[0] * m, p[1] + dir[1] * m, p[2] + dir[2] * m];
                    if self.inside(q) {
                        a = m;
                    } else {
                        b = m;
                    }
                }
                return a;
            }
            last = s;
        }
        span
    }

    /// The point of the crown's skin at a height share and azimuth.
    pub fn skin(&self, t: f64, az: f64) -> [f64; 3] {
        let z = self.z0 + t * (self.z1 - self.z0);
        let r = self.radius(z, az);
        [
            self.center[0] + r * az.cos(),
            self.center[1] + r * az.sin(),
            z,
        ]
    }

    /// Area of the crown's skin, roughly.
    pub fn area(&self) -> f64 {
        let n = 24;
        let mut a = 0.0;
        let h = (self.z1 - self.z0) / n as f64;
        for i in 0..n {
            let z0 = self.z0 + h * i as f64;
            let z1 = z0 + h;
            let r0 = self.radius(z0, 0.0);
            let r1 = self.radius(z1, 0.0);
            let slant = ((r1 - r0).powi(2) + h * h).sqrt();
            a += PI * (r0 + r1) * slant;
        }
        // The top and bottom caps where the profile does not close.
        a + PI * self.radius(self.z0 + 1e-6, 0.0).powi(2)
    }
}
