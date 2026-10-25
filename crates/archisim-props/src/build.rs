//! The pieces models are made of: boxes with rounded edges, cylinders and
//! cones between two points, solids of revolution, ellipsoids, and the
//! hollow of a basin. Every piece is a closed mesh (the hollows, open at
//! the top, are seen from inside), its triangles counter-clockwise from
//! outside, in a role (what material it takes).

use std::collections::BTreeMap;

pub type P3 = [f64; 3];

/// Segments of a round piece.
pub const ROUND: usize = 20;

#[derive(Default)]
pub struct Builder {
    pub positions: Vec<[f32; 3]>,
    pub parts: BTreeMap<&'static str, Vec<[u32; 3]>>,
}

pub fn sub(a: P3, b: P3) -> P3 {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

pub fn cross(a: P3, b: P3) -> P3 {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

pub fn norm(a: P3) -> P3 {
    let l = (a[0] * a[0] + a[1] * a[1] + a[2] * a[2]).sqrt();
    if l < 1e-12 {
        [0.0, 0.0, 1.0]
    } else {
        [a[0] / l, a[1] / l, a[2] / l]
    }
}

impl Builder {
    pub fn point(&mut self, p: P3) -> u32 {
        self.positions.push([p[0] as f32, p[1] as f32, p[2] as f32]);
        (self.positions.len() - 1) as u32
    }

    pub fn tri(&mut self, role: &'static str, a: u32, b: u32, c: u32) {
        if a != b && b != c && a != c {
            self.parts.entry(role).or_default().push([a, b, c]);
        }
    }

    pub fn quad(&mut self, role: &'static str, a: u32, b: u32, c: u32, d: u32) {
        self.tri(role, a, b, c);
        self.tri(role, a, c, d);
    }

    /// A box from `lo` to `hi` with its edges rounded `r` (0: sharp).
    pub fn rbox(&mut self, role: &'static str, lo: P3, hi: P3, r: f64) {
        self.rbox_faces(role, lo, hi, r, [true; 6], false, None);
    }

    /// A cushion: a rounded box whose two big faces swell `puff` in the
    /// middle, as a stuffed cushion does (its sides stay straight).
    pub fn pillow(&mut self, role: &'static str, lo: P3, hi: P3, r: f64, puff: f64) {
        self.rbox_faces(role, lo, hi, r, [true; 6], false, Some(puff));
    }

    /// A box without its top face: for what always has something over it
    /// (a kitchen module's body under its worktop), so no view that paints
    /// faces far to near shows it through.
    pub fn open_box(&mut self, role: &'static str, lo: P3, hi: P3) {
        self.rbox_faces(
            role,
            lo,
            hi,
            0.0,
            [true, true, true, true, true, false],
            false,
            None,
        );
    }

    /// The hollow of a basin: a rounded box without its top, seen from
    /// inside (its faces looking in).
    pub fn hollow(&mut self, role: &'static str, lo: P3, hi: P3, r: f64) {
        self.rbox_faces(
            role,
            lo,
            hi,
            r,
            [true, true, true, true, true, false],
            true,
            None,
        );
    }

    /// The faces of a rounded box: −x, +x, −y, +y, −z, +z, which to make;
    /// with `puff`, swollen as a cushion along its thinnest side.
    #[allow(clippy::too_many_arguments)]
    fn rbox_faces(
        &mut self,
        role: &'static str,
        lo: P3,
        hi: P3,
        r: f64,
        which: [bool; 6],
        inside: bool,
        puff: Option<f64>,
    ) {
        let size = [hi[0] - lo[0], hi[1] - lo[1], hi[2] - lo[2]];
        if size.iter().any(|s| *s <= 1e-6) {
            return;
        }
        let r = r
            .min(size[0] / 2.0)
            .min(size[1] / 2.0)
            .min(size[2] / 2.0)
            .max(0.0);
        let steps = if r > 1e-4 { 4 } else { 0 };
        // A cushion's straight runs are divided, for it to swell.
        let runs = if puff.is_some() { 4 } else { 1 };
        // Coordinates along each axis: the rounding at each end, then the
        // straight run.
        let coords = |k: usize| -> Vec<f64> {
            let mut v = vec![lo[k]];
            for s in 1..=steps {
                let t = std::f64::consts::FRAC_PI_2 * s as f64 / steps as f64;
                v.push(lo[k] + r - r * t.cos());
            }
            for s in 1..runs {
                v.push(lo[k] + r + (size[k] - 2.0 * r) * s as f64 / runs as f64);
            }
            for s in (0..steps).rev() {
                let t = std::f64::consts::FRAC_PI_2 * s as f64 / steps as f64;
                v.push(hi[k] - r + r * t.cos());
            }
            v.push(hi[k]);
            v.dedup_by(|a, b| (*a - *b).abs() < 1e-9);
            v
        };
        // (When the rounding takes all of a side, both ends meet: kept in
        // order against rounding errors.)
        let inner_lo = [lo[0] + r, lo[1] + r, lo[2] + r];
        let inner_hi = [
            (hi[0] - r).max(inner_lo[0]),
            (hi[1] - r).max(inner_lo[1]),
            (hi[2] - r).max(inner_lo[2]),
        ];
        let round = |q: P3| -> P3 {
            if r <= 1e-9 {
                return q;
            }
            let c = [
                q[0].clamp(inner_lo[0], inner_hi[0]),
                q[1].clamp(inner_lo[1], inner_hi[1]),
                q[2].clamp(inner_lo[2], inner_hi[2]),
            ];
            let d = sub(q, c);
            let l = (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).sqrt();
            if l < 1e-12 {
                q
            } else {
                [
                    c[0] + d[0] / l * r,
                    c[1] + d[1] / l * r,
                    c[2] + d[2] / l * r,
                ]
            }
        };
        // The swelling: along the thinnest side, most in the middle of
        // the big faces, nothing at their edges.
        let thin = (0..3)
            .min_by(|a, b| size[*a].total_cmp(&size[*b]))
            .unwrap_or(2);
        let swell = |mut q: P3| -> P3 {
            let Some(puff) = puff else {
                return q;
            };
            let n = |k: usize| ((q[k] - (lo[k] + hi[k]) / 2.0) / (size[k] / 2.0)).clamp(-1.0, 1.0);
            let (u, v) = ((thin + 1) % 3, (thin + 2) % 3);
            let f = (1.0 - n(u) * n(u)) * (1.0 - n(v) * n(v));
            q[thin] += puff * n(thin) * f;
            q
        };
        for (f, make) in which.iter().enumerate() {
            if !make {
                continue;
            }
            let a = f / 2;
            let positive = f % 2 == 1;
            let (u, v) = ((a + 1) % 3, (a + 2) % 3);
            let (cu, cv) = (coords(u), coords(v));
            let mut ids = vec![vec![0u32; cv.len()]; cu.len()];
            for (i, x) in cu.iter().enumerate() {
                for (j, y) in cv.iter().enumerate() {
                    let mut q = [0.0; 3];
                    q[a] = if positive { hi[a] } else { lo[a] };
                    q[u] = *x;
                    q[v] = *y;
                    ids[i][j] = self.point(swell(round(q)));
                }
            }
            // u × v = a: counter-clockwise from +a.
            let flip = !positive ^ inside;
            for i in 0..cu.len() - 1 {
                for j in 0..cv.len() - 1 {
                    let (p, q, s, t) = (ids[i][j], ids[i + 1][j], ids[i + 1][j + 1], ids[i][j + 1]);
                    if flip {
                        self.quad(role, p, t, s, q);
                    } else {
                        self.quad(role, p, q, s, t);
                    }
                }
            }
        }
    }

    /// A frustum (a cylinder if `ra == rb`) from `a` to `b`, closed.
    pub fn cone(&mut self, role: &'static str, a: P3, b: P3, ra: f64, rb: f64, seg: usize) {
        let axis = sub(b, a);
        let w = norm(axis);
        let helper = if w[2].abs() < 0.9 {
            [0.0, 0.0, 1.0]
        } else {
            [1.0, 0.0, 0.0]
        };
        let u = norm(cross(helper, w));
        let v = cross(w, u);
        let ring = |b: &mut Builder, c: P3, r: f64| -> Vec<u32> {
            (0..seg)
                .map(|k| {
                    let t = std::f64::consts::TAU * k as f64 / seg as f64;
                    let (s, co) = t.sin_cos();
                    b.point([
                        c[0] + r * (u[0] * co + v[0] * s),
                        c[1] + r * (u[1] * co + v[1] * s),
                        c[2] + r * (u[2] * co + v[2] * s),
                    ])
                })
                .collect()
        };
        let ra_ = ring(self, a, ra.max(1e-5));
        let rb_ = ring(self, b, rb.max(1e-5));
        for k in 0..seg {
            let k1 = (k + 1) % seg;
            self.quad(role, ra_[k], ra_[k1], rb_[k1], rb_[k]);
        }
        let ca = self.point(a);
        let cb = self.point(b);
        for k in 0..seg {
            let k1 = (k + 1) % seg;
            self.tri(role, ca, ra_[k1], ra_[k]);
            self.tri(role, cb, rb_[k], rb_[k1]);
        }
    }

    pub fn cyl(&mut self, role: &'static str, a: P3, b: P3, r: f64) {
        self.cone(role, a, b, r, r, ROUND);
    }

    /// A profile (radius, height) turned round the vertical through `c`,
    /// stretched `sx`, `sy` across (an oval bowl), closed at its ends.
    pub fn lathe(&mut self, role: &'static str, c: P3, profile: &[(f64, f64)], sx: f64, sy: f64) {
        let mut prof: Vec<(f64, f64)> = profile.to_vec();
        // Up the outside: counter-clockwise with the axis.
        let area: f64 = {
            let mut q = vec![(0.0, prof[0].1)];
            q.extend_from_slice(&prof);
            q.push((0.0, prof[prof.len() - 1].1));
            (0..q.len())
                .map(|i| {
                    let (a, b) = (q[i], q[(i + 1) % q.len()]);
                    a.0 * b.1 - b.0 * a.1
                })
                .sum()
        };
        if area < 0.0 {
            prof.reverse();
        }
        let seg = ROUND + 4;
        let rings: Vec<Vec<u32>> = prof
            .iter()
            .map(|(r, z)| {
                (0..seg)
                    .map(|k| {
                        let t = std::f64::consts::TAU * k as f64 / seg as f64;
                        self.point([c[0] + r * sx * t.cos(), c[1] + r * sy * t.sin(), c[2] + z])
                    })
                    .collect()
            })
            .collect();
        for j in 0..rings.len() - 1 {
            for k in 0..seg {
                let k1 = (k + 1) % seg;
                self.quad(
                    role,
                    rings[j][k],
                    rings[j][k1],
                    rings[j + 1][k1],
                    rings[j + 1][k],
                );
            }
        }
        let (r0, z0) = prof[0];
        if r0 > 1e-6 {
            let m = self.point([c[0], c[1], c[2] + z0]);
            for k in 0..seg {
                self.tri(role, m, rings[0][(k + 1) % seg], rings[0][k]);
            }
        }
        let (rn, zn) = prof[prof.len() - 1];
        let last = rings.len() - 1;
        if rn > 1e-6 {
            let m = self.point([c[0], c[1], c[2] + zn]);
            for k in 0..seg {
                self.tri(role, m, rings[last][k], rings[last][(k + 1) % seg]);
            }
        }
    }

    /// An ellipsoid with these half-axes.
    pub fn ellipsoid(&mut self, role: &'static str, c: P3, r: P3) {
        let seg = ROUND;
        let rows = seg / 2;
        let profile: Vec<(f64, f64)> = (0..=rows)
            .map(|i| {
                let phi =
                    -std::f64::consts::FRAC_PI_2 + std::f64::consts::PI * i as f64 / rows as f64;
                (phi.cos(), r[2] + r[2] * phi.sin())
            })
            .collect();
        self.lathe(role, [c[0], c[1], c[2] - r[2]], &profile, r[0], r[1]);
    }

    /// An ellipsoid whose half-axes are the vectors `ax`, `ay`, `az`
    /// (at right angles): a leaf, a pebble, a cushion lying askew.
    pub fn blob(&mut self, role: &'static str, c: P3, ax: P3, ay: P3, az: P3) {
        let seg = 12;
        let rows = 6;
        // Outwards whatever the hand of the axes.
        let det = {
            let x = cross(ax, ay);
            x[0] * az[0] + x[1] * az[1] + x[2] * az[2]
        };
        let at = |b: &mut Builder, phi: f64, t: f64| {
            let (sp, cp) = phi.sin_cos();
            let (st, ct) = t.sin_cos();
            let (x, y, z) = (cp * ct, cp * st, sp);
            b.point([
                c[0] + ax[0] * x + ay[0] * y + az[0] * z,
                c[1] + ax[1] * x + ay[1] * y + az[1] * z,
                c[2] + ax[2] * x + ay[2] * y + az[2] * z,
            ])
        };
        let half = std::f64::consts::FRAC_PI_2;
        let bottom = at(self, -half, 0.0);
        let top = at(self, half, 0.0);
        let rings: Vec<Vec<u32>> = (1..rows)
            .map(|i| {
                let phi = -half + std::f64::consts::PI * i as f64 / rows as f64;
                (0..seg)
                    .map(|k| at(self, phi, std::f64::consts::TAU * k as f64 / seg as f64))
                    .collect()
            })
            .collect();
        let tri = |b: &mut Builder, p: u32, q: u32, r: u32| {
            if det >= 0.0 {
                b.tri(role, p, q, r);
            } else {
                b.tri(role, p, r, q);
            }
        };
        for k in 0..seg {
            let k1 = (k + 1) % seg;
            tri(self, bottom, rings[0][k1], rings[0][k]);
            let last = rings.len() - 1;
            tri(self, top, rings[last][k], rings[last][k1]);
            for j in 0..rings.len() - 1 {
                tri(self, rings[j][k], rings[j][k1], rings[j + 1][k1]);
                tri(self, rings[j][k], rings[j + 1][k1], rings[j + 1][k]);
            }
        }
    }

    /// A box turned any way: centred at `c`, its half-sides the vectors
    /// `ax`, `ay`, `az` (at right angles): a slat or a back that leans.
    pub fn obox(&mut self, role: &'static str, c: P3, ax: P3, ay: P3, az: P3) {
        let det = {
            let x = cross(ax, ay);
            x[0] * az[0] + x[1] * az[1] + x[2] * az[2]
        };
        let corner = |b: &mut Builder, i: usize| {
            let s = |bit: usize| if i & bit != 0 { 1.0 } else { -1.0 };
            let (sx, sy, sz) = (s(1), s(2), s(4));
            b.point([
                c[0] + ax[0] * sx + ay[0] * sy + az[0] * sz,
                c[1] + ax[1] * sx + ay[1] * sy + az[1] * sz,
                c[2] + ax[2] * sx + ay[2] * sy + az[2] * sz,
            ])
        };
        let v: Vec<u32> = (0..8).map(|i| corner(self, i)).collect();
        // The faces as for an axis box (x, y, z by bits 1, 2, 4).
        let faces = [
            [0, 4, 6, 2], // −x
            [1, 3, 7, 5], // +x
            [0, 1, 5, 4], // −y
            [2, 6, 7, 3], // +y
            [0, 2, 3, 1], // −z
            [4, 5, 7, 6], // +z
        ];
        for f in faces {
            if det >= 0.0 {
                self.quad(role, v[f[0]], v[f[1]], v[f[2]], v[f[3]]);
            } else {
                self.quad(role, v[f[3]], v[f[2]], v[f[1]], v[f[0]]);
            }
        }
    }

    /// A slab whose plan is a polygon (counter-clockwise, convex), from
    /// `z0` to `z1`.
    pub fn slab(&mut self, role: &'static str, poly: &[[f64; 2]], z0: f64, z1: f64) {
        let n = poly.len();
        if n < 3 {
            return;
        }
        let bot: Vec<u32> = poly.iter().map(|p| self.point([p[0], p[1], z0])).collect();
        let top: Vec<u32> = poly.iter().map(|p| self.point([p[0], p[1], z1])).collect();
        for i in 1..n - 1 {
            self.tri(role, top[0], top[i], top[i + 1]);
            self.tri(role, bot[0], bot[i + 1], bot[i]);
        }
        for i in 0..n {
            let j = (i + 1) % n;
            self.quad(role, bot[i], bot[j], top[j], top[i]);
        }
    }

    /// A box from `lo` to `hi` with the top face only in the frame round
    /// a hole (`hole_lo`…`hole_hi` in plan): a worktop with its sink cut.
    pub fn framed_top(
        &mut self,
        role: &'static str,
        lo: P3,
        hi: P3,
        hole_lo: [f64; 2],
        hole_hi: [f64; 2],
    ) {
        // Four boxes round the hole.
        self.rbox(role, lo, [hole_lo[0], hi[1], hi[2]], 0.0);
        self.rbox(role, [hole_hi[0], lo[1], lo[2]], hi, 0.0);
        self.rbox(
            role,
            [hole_lo[0], lo[1], lo[2]],
            [hole_hi[0], hole_lo[1], hi[2]],
            0.0,
        );
        self.rbox(
            role,
            [hole_lo[0], hole_hi[1], lo[2]],
            [hole_hi[0], hi[1], hi[2]],
            0.0,
        );
    }

    pub fn triangle_count(&self) -> usize {
        self.parts.values().map(|t| t.len()).sum()
    }
}
