//! Soft pieces for the workshop models: upholstery that looks stuffed and
//! sewn rather than carved.
//!
//! - [`Soft`]: a rounded box of fabric. Its faces swell (more the bigger
//!   face), its top sags where one sits, the fabric pulls into soft
//!   creases at the corners and nothing is perfectly regular.
//!   Its piping ([`Soft::welt`]) is part of that shape.
//! - [`knife_pillow`]: a scatter cushion, sewn round its edge, fat in the
//!   middle and with pointed corners.
//! - [`tube`]: a round section swept along a path (piping, a rail).
//!
//! Every shape is a function of the point: faces made separately meet
//! exactly, so the view smooths over them.

use crate::build::{cross, norm, sub, Builder, P3};
use std::f64::consts::{FRAC_PI_2, FRAC_PI_4, PI, TAU};

/// A rounded box of fabric from `lo` to `hi`. The *thin* axis (the
/// smallest side) is the one its two big faces look along: up for a seat
/// cushion, forward for a back cushion.
#[derive(Debug, Clone, Copy)]
pub struct Soft {
    pub lo: P3,
    pub hi: P3,
    /// Radius of the edges and corners.
    pub radius: f64,
    /// How much each big face swells at its middle (metres).
    pub crown: f64,
    /// How much the sides swell at their middle.
    pub side: f64,
    /// How deep the top sags where one sits (towards the front, `+y`).
    pub sag: f64,
    /// How deep the creases pulled from the corners of the big faces are.
    pub creases: f64,
    /// Small irregularities, as of hand-sewn fabric (metres).
    pub wobble: f64,
    /// The piping sewn round the two big faces: how much it stands proud
    /// (metres; 0: none). Part of the fabric's own shape, half way round
    /// the edge, so it shades as a seam instead of being drawn as a line.
    pub welt: f64,
    /// Which irregularities: two pieces with different seeds differ.
    pub seed: u32,
    /// The longest a facet of a straight run may be (metres).
    pub step: f64,
    /// Facets round each edge.
    pub round_steps: usize,
}

impl Soft {
    pub fn new(lo: P3, hi: P3, radius: f64) -> Self {
        Self {
            lo,
            hi,
            radius,
            crown: 0.0,
            side: 0.0,
            sag: 0.0,
            creases: 0.0,
            wobble: 0.0,
            welt: 0.0,
            seed: 0,
            step: 0.11,
            round_steps: 4,
        }
    }

    fn size(&self) -> P3 {
        [
            self.hi[0] - self.lo[0],
            self.hi[1] - self.lo[1],
            self.hi[2] - self.lo[2],
        ]
    }

    fn mid(&self) -> P3 {
        [
            (self.hi[0] + self.lo[0]) / 2.0,
            (self.hi[1] + self.lo[1]) / 2.0,
            (self.hi[2] + self.lo[2]) / 2.0,
        ]
    }

    /// The axis of its two big faces.
    pub fn thin(&self) -> usize {
        let s = self.size();
        (0..3).min_by(|a, b| s[*a].total_cmp(&s[*b])).unwrap_or(2)
    }

    fn r(&self) -> f64 {
        let s = self.size();
        self.radius
            .min(s[0] / 2.0)
            .min(s[1] / 2.0)
            .min(s[2] / 2.0)
            .max(0.0)
    }

    /// The point of the plain rounded box under `q` (a point of the sharp
    /// box), and its normal there.
    fn round(&self, q: P3) -> (P3, P3) {
        let r = self.r();
        let inner_lo = [self.lo[0] + r, self.lo[1] + r, self.lo[2] + r];
        let inner_hi = [
            (self.hi[0] - r).max(inner_lo[0]),
            (self.hi[1] - r).max(inner_lo[1]),
            (self.hi[2] - r).max(inner_lo[2]),
        ];
        let c = [
            q[0].clamp(inner_lo[0], inner_hi[0]),
            q[1].clamp(inner_lo[1], inner_hi[1]),
            q[2].clamp(inner_lo[2], inner_hi[2]),
        ];
        let d = sub(q, c);
        let l = (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).sqrt();
        if l < 1e-12 || r <= 1e-9 {
            return (q, [0.0, 0.0, 0.0]);
        }
        let n = [d[0] / l, d[1] / l, d[2] / l];
        ([c[0] + n[0] * r, c[1] + n[1] * r, c[2] + n[2] * r], n)
    }

    /// Where a point of the plain rounded box goes once the fabric is
    /// stuffed, sat on and pulled.
    pub fn shape(&self, p: P3, n: P3) -> P3 {
        let size = self.size();
        let mid = self.mid();
        let s = |k: usize| ((p[k] - mid[k]) / (size[k] / 2.0)).clamp(-1.0, 1.0);
        let sv = [s(0), s(1), s(2)];
        // How far from the edges across an axis: 1 in the middle, 0 at
        // the edge, soft (the fabric is held at the seams).
        let g = |x: f64| (1.0 - x * x).max(0.0).powf(0.85);
        let t = self.thin();
        let mut q = p;
        // The piping: a bead where the edge has turned half way from the
        // big face, all round (on the corners too).
        if self.welt > 0.0 {
            let turned = n[t].abs().clamp(0.0, 1.0).acos();
            let off = (turned - std::f64::consts::FRAC_PI_4) / 0.16;
            let bead = self.welt * (-off * off).exp();
            for k in 0..3 {
                q[k] += n[k] * bead;
            }
        }
        for k in 0..3 {
            let (u, v) = ((k + 1) % 3, (k + 2) % 3);
            let amount = if k == t { self.crown } else { self.side };
            q[k] += amount * sv[k] * g(sv[u]) * g(sv[v]);
        }
        let (u, v) = ((t + 1) % 3, (t + 2) % 3);
        // Sat on: the upper big face sinks, most a little in front of its
        // middle, nothing at its seams.
        if self.sag > 0.0 && sv[t] > 0.0 {
            let fwd = if t == 2 { sv[1] } else { 0.0 };
            let spot = (-((fwd - 0.25) / 0.7).powi(2)).exp();
            q[t] -= self.sag * sv[t] * g(sv[u]).powf(1.5) * g(sv[v]).powf(1.5) * spot;
        }
        // Creases pulled from the corners towards the middle: two or three
        // soft folds fanning out of each corner of the big faces.
        if self.creases > 0.0 {
            let mut c = 0.0;
            for (cu, cv) in [(-1.0, -1.0), (1.0, -1.0), (-1.0, 1.0), (1.0, 1.0)] {
                let du = -((sv[u] - cu) * cu); // 0 at the corner, inwards positive
                let dv = -((sv[v] - cv) * cv);
                let rho = (du * du + dv * dv).sqrt();
                if !(1e-6..=0.9).contains(&rho) {
                    continue;
                }
                // The angle off the diagonal into the face.
                let ang = dv.atan2(du) - FRAC_PI_4;
                let fan = (ang * 5.0).cos().max(0.0).powi(3);
                let reach = smooth(0.02, 0.12, rho) * (1.0 - smooth(0.25, 0.9, rho));
                c += fan * reach;
            }
            q[t] -= self.creases * sv[t] * c;
        }
        // Nothing sewn by hand is perfectly regular.
        if self.wobble > 0.0 {
            let n = wobble_at(p, self.seed);
            for k in 0..3 {
                q[k] += self.wobble * n[k] * (0.4 + 0.6 * sv[k].abs());
            }
        }
        q
    }

    /// The coordinates along an axis: the rounding at each end, the run
    /// between divided so that no facet is longer than `step`.
    fn coords(&self, k: usize) -> Vec<f64> {
        let r = self.r();
        let size = self.size()[k];
        let steps = if r > 1e-4 { self.round_steps.max(1) } else { 0 };
        let run = size - 2.0 * r;
        let runs = ((run / self.step).ceil() as usize).clamp(1, 24);
        // The angles round each edge, closer together at its middle when
        // the piping runs there.
        let angles: Vec<f64> = if self.welt > 0.0 && steps > 0 && k == self.thin() {
            [0.0, 25.0, 38.0, 45.0, 52.0, 65.0, 90.0]
                .iter()
                .map(|a: &f64| a.to_radians())
                .collect()
        } else {
            (0..=steps)
                .map(|i| FRAC_PI_2 * i as f64 / steps.max(1) as f64)
                .collect()
        };
        let mut v = vec![self.lo[k]];
        for a in angles.iter().skip(1) {
            v.push(self.lo[k] + r - r * a.cos());
        }
        for i in 1..runs {
            v.push(self.lo[k] + r + run * i as f64 / runs as f64);
        }
        for a in angles.iter().rev().skip(1) {
            v.push(self.hi[k] - r + r * a.cos());
        }
        v.push(self.hi[k]);
        v.dedup_by(|a, b| (*a - *b).abs() < 1e-9);
        v
    }

    /// Adds it to the model, with these faces (−x, +x, −y, +y, −z, +z).
    pub fn add_faces(&self, b: &mut Builder, role: &'static str, which: [bool; 6]) {
        let size = self.size();
        if size.iter().any(|s| *s <= 1e-6) {
            return;
        }
        for (f, make) in which.iter().enumerate() {
            if !make {
                continue;
            }
            let a = f / 2;
            let positive = f % 2 == 1;
            let (u, v) = ((a + 1) % 3, (a + 2) % 3);
            let (cu, cv) = (self.coords(u), self.coords(v));
            let mut ids = vec![vec![0u32; cv.len()]; cu.len()];
            for (i, x) in cu.iter().enumerate() {
                for (j, y) in cv.iter().enumerate() {
                    let mut q = [0.0; 3];
                    q[a] = if positive { self.hi[a] } else { self.lo[a] };
                    q[u] = *x;
                    q[v] = *y;
                    let (p, n) = self.round(q);
                    ids[i][j] = b.point(self.shape(p, n));
                }
            }
            for i in 0..cu.len() - 1 {
                for j in 0..cv.len() - 1 {
                    let (p, q, s, t) = (ids[i][j], ids[i + 1][j], ids[i + 1][j + 1], ids[i][j + 1]);
                    // Split along the shorter diagonal: smoother on the
                    // curved fabric.
                    let (pp, qq, ss, tt) = (
                        b.positions[p as usize],
                        b.positions[q as usize],
                        b.positions[s as usize],
                        b.positions[t as usize],
                    );
                    let d1 = dist2(pp, ss);
                    let d2 = dist2(qq, tt);
                    let quad = if positive { [p, q, s, t] } else { [p, t, s, q] };
                    if d1 <= d2 {
                        b.tri(role, quad[0], quad[1], quad[2]);
                        b.tri(role, quad[0], quad[2], quad[3]);
                    } else {
                        b.tri(role, quad[0], quad[1], quad[3]);
                        b.tri(role, quad[1], quad[2], quad[3]);
                    }
                }
            }
        }
    }

    /// Adds it whole. Its role is drawn as soft fabric: keep it for
    /// fabric only.
    pub fn add(&self, b: &mut Builder, role: &'static str) {
        self.add_faces(b, role, [true; 6]);
        b.soft.insert(role);
    }
}

/// A round section of radius `r` swept along `path` (closed: back to its
/// first point), its ends capped when open.
pub fn tube(b: &mut Builder, role: &'static str, path: &[P3], closed: bool, r: f64, seg: usize) {
    let n = path.len();
    if n < 2 {
        return;
    }
    let tangent = |i: usize| -> P3 {
        let (a, c) = if closed {
            (path[(i + n - 1) % n], path[(i + 1) % n])
        } else {
            (path[i.saturating_sub(1)], path[(i + 1).min(n - 1)])
        };
        norm(sub(c, a))
    };
    // Frames carried along without twisting.
    let t0 = tangent(0);
    let helper = if t0[2].abs() < 0.9 {
        [0.0, 0.0, 1.0]
    } else {
        [1.0, 0.0, 0.0]
    };
    // Carries `x` onto the plane across the tangent `t`.
    let carry = |x: P3, t: P3| -> P3 {
        let d = x[0] * t[0] + x[1] * t[1] + x[2] * t[2];
        norm([x[0] - t[0] * d, x[1] - t[1] * d, x[2] - t[2] * d])
    };
    let mut x = norm(cross(helper, t0));
    let mut frames: Vec<(P3, P3)> = Vec::with_capacity(n);
    for i in 0..n {
        let t = tangent(i);
        x = carry(x, t);
        frames.push((x, cross(t, x)));
    }
    // A closed loop: spread the turn it gathers so the ends meet.
    let twist = if closed {
        let (x0, y0) = frames[0];
        let xc = carry(frames[n - 1].0, t0);
        let cosang = x0[0] * xc[0] + x0[1] * xc[1] + x0[2] * xc[2];
        let sinang = y0[0] * xc[0] + y0[1] * xc[1] + y0[2] * xc[2];
        sinang.atan2(cosang)
    } else {
        0.0
    };
    let rings: Vec<Vec<u32>> = (0..n)
        .map(|i| {
            let (x, y) = frames[i];
            let back = -twist * i as f64 / n as f64;
            let c = path[i];
            (0..seg)
                .map(|k| {
                    let a = TAU * k as f64 / seg as f64 + back;
                    let (sa, ca) = a.sin_cos();
                    b.point([
                        c[0] + r * (x[0] * ca + y[0] * sa),
                        c[1] + r * (x[1] * ca + y[1] * sa),
                        c[2] + r * (x[2] * ca + y[2] * sa),
                    ])
                })
                .collect()
        })
        .collect();
    let last = if closed { n } else { n - 1 };
    for i in 0..last {
        let j = (i + 1) % n;
        for k in 0..seg {
            let k1 = (k + 1) % seg;
            b.quad(role, rings[i][k], rings[i][k1], rings[j][k1], rings[j][k]);
        }
    }
    if !closed {
        let a = b.point(path[0]);
        let z = b.point(path[n - 1]);
        for k in 0..seg {
            let k1 = (k + 1) % seg;
            b.tri(role, a, rings[0][k1], rings[0][k]);
            b.tri(role, z, rings[n - 1][k], rings[n - 1][k1]);
        }
    }
}

/// A scatter cushion: a square of fabric `w` × `h` sewn round its edge,
/// `thick` in the middle, its corners pointed. Centred at `c`, its face
/// looking along `az`, its width along `ax` (unit vectors at right
/// angles); `seed` makes each one a little different.
#[allow(clippy::too_many_arguments)]
pub fn knife_pillow(
    b: &mut Builder,
    role: &'static str,
    c: P3,
    ax: P3,
    az: P3,
    w: f64,
    h: f64,
    thick: f64,
    seed: u32,
) {
    b.soft.insert(role);
    let ay = cross(az, ax);
    let n = 12usize;
    // Closer together towards the edge, where it curves most.
    let at = |i: usize| -((PI * i as f64 / n as f64).cos());
    let place = |u: f64, v: f64, z: f64| -> P3 {
        // The edges drawn in between the corners, as a stuffed cushion's.
        let pull_u = 0.10 * (1.0 - v * v);
        let pull_v = 0.10 * (1.0 - u * u);
        let x = u * w / 2.0 * (1.0 - pull_u);
        let y = v * h / 2.0 * (1.0 - pull_v);
        // Squashed a little where it rests (towards +`ay`).
        let y = if v > 0.0 { y * (1.0 - 0.04 * v) } else { y };
        let mut p = [
            c[0] + ax[0] * x + ay[0] * y + az[0] * z,
            c[1] + ax[1] * x + ay[1] * y + az[1] * z,
            c[2] + ax[2] * x + ay[2] * y + az[2] * z,
        ];
        let wob = wobble_at(p, seed);
        for k in 0..3 {
            p[k] += 0.003 * wob[k] * (1.0 - u * u).sqrt() * (1.0 - v * v).sqrt();
        }
        p
    };
    let fat = |u: f64, v: f64| -> f64 {
        let f = ((1.0 - u * u).max(0.0) * (1.0 - v * v).max(0.0)).powf(0.42);
        // Creases running in from the corners.
        let mut crease = 0.0;
        for (cu, cv) in [(-1.0, -1.0), (1.0, -1.0), (-1.0, 1.0), (1.0, 1.0)] {
            let du = (u - cu) * -cu;
            let dv = (v - cv) * -cv;
            let rho = (du * du + dv * dv).sqrt();
            let ang = dv.atan2(du) - FRAC_PI_4;
            crease += (ang * 4.0).cos().max(0.0).powi(4)
                * smooth(0.05, 0.25, rho)
                * (1.0 - smooth(0.3, 0.8, rho));
        }
        thick / 2.0 * f * (1.0 - 0.18 * crease)
    };
    // The two faces share the points of their sewn edge.
    let mut edge = std::collections::HashMap::new();
    let mut ids = [
        vec![vec![0u32; n + 1]; n + 1],
        vec![vec![0u32; n + 1]; n + 1],
    ];
    for (face, sign) in ids.iter_mut().zip([1.0, -1.0]) {
        for (i, row) in face.iter_mut().enumerate() {
            for (j, id) in row.iter_mut().enumerate() {
                let (u, v) = (at(i), at(j));
                let on_edge = i == 0 || j == 0 || i == n || j == n;
                *id = if on_edge {
                    *edge
                        .entry((i, j))
                        .or_insert_with(|| b.point(place(u, v, 0.0)))
                } else {
                    b.point(place(u, v, sign * fat(u, v)))
                };
            }
        }
    }
    for i in 0..n {
        for j in 0..n {
            let f = &ids[0];
            b.quad(role, f[i][j], f[i + 1][j], f[i + 1][j + 1], f[i][j + 1]);
            let k = &ids[1];
            b.quad(role, k[i][j], k[i][j + 1], k[i + 1][j + 1], k[i + 1][j]);
        }
    }
}

/// 0 below `a`, 1 above `b`, smooth between.
pub fn smooth(a: f64, b: f64, x: f64) -> f64 {
    let t = ((x - a) / (b - a)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

fn dist2(a: [f32; 3], b: [f32; 3]) -> f32 {
    (a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2) + (a[2] - b[2]).powi(2)
}

/// A slow, smooth wobble (each component from −1 to 1) that depends on
/// the place and the seed: never the same twice, never jagged.
pub fn wobble_at(p: P3, seed: u32) -> P3 {
    let s = seed as f64 * 1.618_033_988;
    let f = |k: f64| -> f64 {
        let a = (p[0] * 9.1 + p[1] * 3.7 + s * 2.3 + k).sin();
        let b = (p[1] * 7.3 - p[2] * 5.1 + s * 1.1 + k * 2.0).sin();
        let c = (p[2] * 8.7 + p[0] * 4.3 - s * 0.7 + k * 3.0).sin();
        (a + b + c) / 3.0
    };
    [f(0.0), f(1.3), f(2.9)]
}
