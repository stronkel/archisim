//! Bed clothes for the workshop's beds (paquete 3): a cover that lies on
//! a mattress and falls over its edges ([`Cover`]: a duvet, a sheet
//! turned down over it, a bedspread or a throw across the foot), the
//! pillows and the scatter cushions.
//!
//! A cover is a cloth laid flat over a rectangle and folded down round
//! its edges: each point of the flat cloth goes over the edge's rounded
//! turn and hangs, flaring out a little and in soft folds towards its
//! hem. Its corners hang as a tablecloth's, their hem rounded.

use super::soft::{knife_pillow, smooth, wobble_at};
use crate::build::{Builder, P3};
use std::collections::HashMap;
use std::f64::consts::{FRAC_PI_2, TAU};

/// A cloth over the rectangle `x` × `y` (in plan), its top face at `z`.
#[derive(Debug, Clone, Copy)]
pub struct Cover {
    pub x: [f64; 2],
    pub y: [f64; 2],
    pub z: f64,
    /// How thick it is (a duvet several centimetres, a sheet one).
    pub thick: f64,
    /// The radius of its turn over the edge (outside).
    pub round: f64,
    /// How much cloth falls beyond each edge (−x, +x, −y, +y), measured
    /// along the cloth from the edge; 0: it ends at that edge.
    pub falls: [f64; 4],
    /// How much its top swells in the middle.
    pub puff: f64,
    /// How deep the folds of the hanging cloth are at its hem.
    pub folds: f64,
    /// How far the hem stands off the edge.
    pub flare: f64,
    /// Quilted channels across the top, their spacing (0: none).
    pub quilt: f64,
    /// The lowest its hem may come.
    pub floor: f64,
    pub seed: u32,
}

impl Cover {
    pub fn new(x: [f64; 2], y: [f64; 2], z: f64, thick: f64) -> Self {
        Self {
            x,
            y,
            z,
            thick,
            round: (thick * 1.2).max(0.02),
            falls: [0.0; 4],
            puff: 0.0,
            folds: 0.0,
            flare: 0.0,
            quilt: 0.0,
            floor: 0.02,
            seed: 0,
        }
    }

    /// The cloth beyond the turn reaches the floor here.
    fn reach(&self) -> f64 {
        self.round * FRAC_PI_2 + (self.z - self.round - self.floor).max(0.0)
    }

    /// How far the cloth may reach out from the edge along `dir` (a plan
    /// direction out of the rectangle).
    fn limit(&self, dir: [f64; 2]) -> f64 {
        let fx = if dir[0] < 0.0 {
            self.falls[0]
        } else {
            self.falls[1]
        };
        let fy = if dir[1] < 0.0 {
            self.falls[2]
        } else {
            self.falls[3]
        };
        let (fx, fy) = (fx.min(self.reach()), fy.min(self.reach()));
        let ax = if dir[0].abs() < 1e-9 {
            0.0
        } else if fx <= 1e-9 {
            return 0.0;
        } else {
            dir[0] / fx
        };
        let ay = if dir[1].abs() < 1e-9 {
            0.0
        } else if fy <= 1e-9 {
            return 0.0;
        } else {
            dir[1] / fy
        };
        let k = (ax * ax + ay * ay).sqrt();
        if k < 1e-12 {
            0.0
        } else {
            1.0 / k
        }
    }

    /// Where the point `(fx, fy)` of the flat cloth lies, on its top
    /// (`inner` false) or under face. `None`: beyond its hem (the place on
    /// the hem it is pulled to comes back as `Some` with `true`).
    fn place(&self, fx: f64, fy: f64, inner: bool) -> (P3, bool) {
        let q = [
            fx.clamp(self.x[0], self.x[1]),
            fy.clamp(self.y[0], self.y[1]),
        ];
        let d = [fx - q[0], fy - q[1]];
        let mut s = (d[0] * d[0] + d[1] * d[1]).sqrt();
        let dir = if s > 1e-12 {
            [d[0] / s, d[1] / s]
        } else {
            [0.0, 0.0]
        };
        let mut clamped = false;
        if s > 1e-12 {
            let lim = self.limit(dir);
            if s > lim + 1e-9 {
                s = lim;
                clamped = true;
            }
        }
        let t = if inner { self.thick } else { 0.0 };
        if s <= 1e-12 {
            // On the top: it swells in the middle, wrinkles a little and,
            // quilted, dips along its channels.
            let (w, l) = (self.x[1] - self.x[0], self.y[1] - self.y[0]);
            let u = ((q[0] - self.x[0]) / w.max(1e-6)) * 2.0 - 1.0;
            let v = ((q[1] - self.y[0]) / l.max(1e-6)) * 2.0 - 1.0;
            let fall_x = self.falls[0].max(self.falls[1]) > 0.0;
            let fall_y0 = self.falls[2] > 0.0;
            let fall_y1 = self.falls[3] > 0.0;
            // Towards an edge it falls over, the swell eases out.
            let ex = if fall_x { 1.0 - u.abs().powi(6) } else { 1.0 };
            let ey = match (v < 0.0, fall_y0, fall_y1) {
                (true, true, _) | (false, _, true) => 1.0 - v.abs().powi(6),
                _ => 1.0 - 0.6 * v.abs().powi(6),
            };
            let bump = ex * ey;
            let mut z = self.z - t + self.puff * bump * if inner { 0.3 } else { 1.0 };
            if !inner {
                let wob = wobble_at([q[0] * 1.7, q[1] * 1.7, 0.0], self.seed);
                z += 0.004 * wob[2] * bump;
                if self.quilt > 0.0 {
                    let c = (q[1] - self.y[0]) / self.quilt * TAU;
                    z -= self.thick * 0.18 * (0.5 + 0.5 * c.cos()).powi(3) * bump;
                }
            }
            return ([q[0], q[1], z], false);
        }
        // Over the turn and down.
        let r = (self.round - t).max(0.002);
        let zc = self.z - self.round;
        let theta = s / self.round;
        let (mut h, z) = if theta < FRAC_PI_2 {
            (r * theta.sin(), zc + r * theta.cos())
        } else {
            (r, zc - (s - self.round * FRAC_PI_2))
        };
        // Hanging: it flares and folds towards its hem.
        let hang = (s - self.round * FRAC_PI_2).max(0.0);
        let k = (hang / 0.35).min(1.0);
        let along = self.along(q, dir, s);
        let phase = along / 0.21 * TAU + self.seed as f64 * 1.3;
        let fold = (phase.sin() + 0.45 * (phase * 2.3 + 1.1).sin()) / 1.45;
        h += self.flare * k.powf(1.5) + self.folds * k * fold;
        let wob = wobble_at([fx * 2.1, fy * 2.1, z * 2.1], self.seed + 7);
        let z = z + 0.006 * k * wob[2];
        ([q[0] + dir[0] * h, q[1] + dir[1] * h, z], clamped)
    }

    /// How far round the hem a hanging point is (metres along the cloth
    /// at its distance `s` from the edge), going round the rectangle from
    /// its corner at `(x1, y0)`: the folds run on round the corners.
    fn along(&self, q: [f64; 2], dir: [f64; 2], s: f64) -> f64 {
        let (w, l) = (self.x[1] - self.x[0], self.y[1] - self.y[0]);
        let e = 1e-9;
        let on_x1 = (q[0] - self.x[1]).abs() < e;
        let on_x0 = (q[0] - self.x[0]).abs() < e;
        let on_y1 = (q[1] - self.y[1]).abs() < e;
        let on_y0 = (q[1] - self.y[0]).abs() < e;
        let mut a = dir[1].atan2(dir[0]);
        if a < -FRAC_PI_2 {
            a += TAU;
        }
        let q4 = FRAC_PI_2;
        match (on_x0, on_x1, on_y0, on_y1) {
            (_, true, _, true) => l + s * a,
            (true, _, _, true) => l + w + s * a,
            (true, _, true, _) => 2.0 * l + w + s * a,
            (_, true, true, _) => -s * a.abs(),
            (_, true, _, _) => q[1] - self.y[0],
            (_, _, _, true) => l + s * q4 + (self.x[1] - q[0]),
            (true, _, _, _) => l + w + 2.0 * s * q4 + (self.y[1] - q[1]),
            _ => 2.0 * l + w + 3.0 * s * q4 + (q[0] - self.x[0]),
        }
    }

    /// The cloth's samples along one axis: fine where it turns and
    /// hangs, coarser across its flat middle.
    fn samples(lo: f64, hi: f64, fall_lo: f64, fall_hi: f64, near: f64) -> Vec<f64> {
        let fine = 0.04;
        let coarse = 0.11;
        let mut v = Vec::new();
        let push_run = |a: f64, b: f64, step: f64, v: &mut Vec<f64>| {
            if b - a <= 1e-9 {
                return;
            }
            let n = ((b - a) / step).ceil().max(1.0) as usize;
            for i in 0..n {
                v.push(a + (b - a) * i as f64 / n as f64);
            }
        };
        let near = near.min((hi - lo) / 3.0);
        push_run(lo - fall_lo, lo, fine, &mut v);
        push_run(lo, lo + near, fine, &mut v);
        push_run(lo + near, hi - near, coarse, &mut v);
        push_run(hi - near, hi, fine, &mut v);
        push_run(hi, hi + fall_hi, fine, &mut v);
        v.push(hi + fall_hi);
        v
    }

    /// Adds it: a closed skin of `role`, drawn soft.
    pub fn add(&self, b: &mut Builder, role: &'static str) {
        let (lx, hx) = (self.limit([-1.0, 0.0]), self.limit([1.0, 0.0]));
        let (ly, hy) = (self.limit([0.0, -1.0]), self.limit([0.0, 1.0]));
        let near = self.round * 2.5;
        let xs = Self::samples(self.x[0], self.x[1], lx, hx, near);
        let ys = Self::samples(self.y[0], self.y[1], ly, hy, near);
        let (nx, ny) = (xs.len(), ys.len());
        let mut outer = vec![vec![0u32; ny]; nx];
        let mut inner = vec![vec![0u32; ny]; nx];
        let mut out = vec![vec![false; ny]; nx];
        for (i, &fx) in xs.iter().enumerate() {
            for (j, &fy) in ys.iter().enumerate() {
                let (p, c) = self.place(fx, fy, false);
                let (pi, _) = self.place(fx, fy, true);
                outer[i][j] = b.point(p);
                inner[i][j] = b.point(pi);
                out[i][j] = c;
            }
        }
        // The cells of the cloth (not those wholly beyond its hem), and
        // which of their edges are its border.
        let mut cells = Vec::new();
        for i in 0..nx - 1 {
            for j in 0..ny - 1 {
                if out[i][j] && out[i + 1][j] && out[i + 1][j + 1] && out[i][j + 1] {
                    continue;
                }
                cells.push((i, j));
            }
        }
        let mut edges: HashMap<(usize, usize, usize, usize), u32> = HashMap::new();
        let mut tris: Vec<[(usize, usize); 3]> = Vec::new();
        for &(i, j) in &cells {
            let quad = [(i, j), (i + 1, j), (i + 1, j + 1), (i, j + 1)];
            // Split along the shorter diagonal of the hung cloth.
            let pos = |n: (usize, usize)| b.positions[outer[n.0][n.1] as usize];
            let d = |a: [f32; 3], c: [f32; 3]| {
                (a[0] - c[0]).powi(2) + (a[1] - c[1]).powi(2) + (a[2] - c[2]).powi(2)
            };
            let pair = if d(pos(quad[0]), pos(quad[2])) <= d(pos(quad[1]), pos(quad[3])) {
                [[quad[0], quad[1], quad[2]], [quad[0], quad[2], quad[3]]]
            } else {
                [[quad[0], quad[1], quad[3]], [quad[1], quad[2], quad[3]]]
            };
            for t in pair {
                // Two corners pulled to one place on the hem: no triangle.
                let same = |a: (usize, usize), c: (usize, usize)| d(pos(a), pos(c)) < 1e-12;
                if same(t[0], t[1]) || same(t[1], t[2]) || same(t[0], t[2]) {
                    continue;
                }
                tris.push(t);
                for k in 0..3 {
                    let (a, c) = (t[k], t[(k + 1) % 3]);
                    *edges.entry((a.0, a.1, c.0, c.1)).or_default() += 1;
                }
            }
        }
        for t in &tris {
            let o = |n: (usize, usize)| outer[n.0][n.1];
            let n = |n: (usize, usize)| inner[n.0][n.1];
            b.tri(role, o(t[0]), o(t[1]), o(t[2]));
            b.tri(role, n(t[0]), n(t[2]), n(t[1]));
        }
        // The hem and the cut edges: the two faces sewn together.
        for t in &tris {
            for k in 0..3 {
                let (a, c) = (t[k], t[(k + 1) % 3]);
                if edges.contains_key(&(c.0, c.1, a.0, a.1)) {
                    continue;
                }
                let (oa, oc) = (outer[a.0][a.1], outer[c.0][c.1]);
                let (ia, ic) = (inner[a.0][a.1], inner[c.0][c.1]);
                b.quad(role, oc, oa, ia, ic);
            }
        }
        b.soft.insert(role);
    }
}

/// A bed pillow, `w` across and `h` deep, lying on its back against the
/// headboard: centred at `c`, propped up `lean` radians from lying flat.
pub fn bed_pillow(
    b: &mut Builder,
    role: &'static str,
    c: P3,
    w: f64,
    h: f64,
    lean: f64,
    seed: u32,
) {
    let (s, co) = lean.sin_cos();
    // Its face looks up and to the front.
    let az = [0.0, s, co];
    knife_pillow(b, role, c, [1.0, 0.0, 0.0], az, w, h, h * 0.36, seed);
}

/// A scatter cushion standing up before the pillows at `c`, `size`
/// across, leaning back `lean` and turned `yaw` about the vertical.
pub fn standing_cushion(
    b: &mut Builder,
    role: &'static str,
    c: P3,
    size: f64,
    lean: f64,
    yaw: f64,
    seed: u32,
) {
    let (sy, cy) = yaw.sin_cos();
    let (sl, cl) = lean.sin_cos();
    let az = [-sy * cl, cy * cl, sl];
    let ax = [cy, sy, 0.0];
    knife_pillow(b, role, c, ax, az, size, size, size * 0.42, seed);
}

/// A long round bolster across the bed at `c`, `len` long.
pub fn bolster(b: &mut Builder, role: &'static str, c: P3, len: f64, r: f64) {
    let start = b.mark();
    let n = 14;
    let prof: Vec<(f64, f64)> = (0..=n)
        .map(|i| {
            let t = i as f64 / n as f64;
            // Full along its length, gathered at its two ends.
            let e = smooth(0.0, 0.08, t) * smooth(0.0, 0.08, 1.0 - t);
            (r * (0.35 + 0.65 * e.sqrt()), len * t)
        })
        .collect();
    let mut prof = prof;
    prof.insert(0, (0.0, 0.0));
    prof.push((0.0, len));
    super::shape::turned(b, role, [0.0; 3], &prof, 16, 0.0);
    // Laid along x.
    b.transform_from(start, |q| {
        [c[0] - len / 2.0 + q[2], c[1] + q[1], c[2] - q[0]]
    });
    b.soft.insert(role);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fine::shape::signed_volume;

    #[test]
    fn a_duvet_is_closed_and_hangs_over_its_edges() {
        let mut b = Builder::default();
        let mut c = Cover::new([-0.8, 0.8], [0.6, 2.0], 0.6, 0.05);
        c.falls = [0.3, 0.3, 0.0, 0.3];
        c.puff = 0.03;
        c.folds = 0.015;
        c.flare = 0.02;
        c.add(&mut b, "duvet");
        let t = &b.parts["duvet"];
        assert!(signed_volume(&b.positions, t) > 0.0);
        // Its hem below the top, its sides beyond the mattress.
        let zmin = b.positions.iter().map(|p| p[2]).fold(f32::MAX, f32::min);
        let xmax = b.positions.iter().map(|p| p[0]).fold(f32::MIN, f32::max);
        assert!(zmin < 0.4 && zmin > 0.0, "{zmin}");
        assert!(xmax > 0.8, "{xmax}");
        // Every edge is shared by two triangles: closed.
        let mut count: HashMap<(u32, u32), i32> = HashMap::new();
        for tri in t {
            for k in 0..3 {
                let (a, c) = (tri[k], tri[(k + 1) % 3]);
                *count.entry((a.min(c), a.max(c))).or_default() += 1;
            }
        }
        assert!(count.values().all(|n| *n == 2));
    }

    #[test]
    fn a_bolster_is_closed() {
        let mut b = Builder::default();
        bolster(&mut b, "linen", [0.0, 0.3, 0.7], 1.4, 0.1);
        assert!(signed_volume(&b.positions, &b.parts["linen"]) > 0.0);
    }
}
