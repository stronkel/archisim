//! The workshop's shapes beyond stuffed fabric: tubes swept along smooth
//! curves (stems, a tap's neck), turned profiles of any number of sides
//! (a lamp post and its lantern), thin plates set any way (a lantern's
//! panes), shells lofted through rounded rectangles (a sink's basin) and
//! leaves with their thickness, fold, droop and veins.
//!
//! Every piece is closed, its triangles counter-clockwise from outside,
//! as the rest of the models; pieces that meet overlap a little, so
//! nothing floats.

use crate::build::{cross, norm, sub, Builder, P3};
use std::f64::consts::{PI, TAU};

pub fn add(a: P3, b: P3) -> P3 {
    [a[0] + b[0], a[1] + b[1], a[2] + b[2]]
}

pub fn mul(a: P3, k: f64) -> P3 {
    [a[0] * k, a[1] * k, a[2] * k]
}

pub fn dot(a: P3, b: P3) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

pub fn len(a: P3) -> f64 {
    dot(a, a).sqrt()
}

pub fn lerp(a: P3, b: P3, t: f64) -> P3 {
    add(a, mul(sub(b, a), t))
}

/// `v` turned `angle` about the unit axis `k` (Rodrigues).
pub fn rotate(v: P3, k: P3, angle: f64) -> P3 {
    let (s, c) = angle.sin_cos();
    add(
        add(mul(v, c), mul(cross(k, v), s)),
        mul(k, dot(k, v) * (1.0 - c)),
    )
}

/// A smooth curve through the points (Catmull–Rom), `per` points for
/// each stretch between two of them, the ends included.
pub fn curve(points: &[P3], per: usize) -> Vec<P3> {
    let n = points.len();
    if n < 3 || per < 2 {
        return points.to_vec();
    }
    let at = |i: isize| -> P3 {
        if i < 0 {
            // Mirrored before the first point: the curve leaves it
            // heading for the second.
            sub(mul(points[0], 2.0), points[1])
        } else if i as usize >= n {
            sub(mul(points[n - 1], 2.0), points[n - 2])
        } else {
            points[i as usize]
        }
    };
    let mut out = Vec::with_capacity((n - 1) * per + 1);
    for i in 0..n - 1 {
        let (p0, p1, p2, p3) = (
            at(i as isize - 1),
            at(i as isize),
            at(i as isize + 1),
            at(i as isize + 2),
        );
        for s in 0..per {
            let t = s as f64 / per as f64;
            let (t2, t3) = (t * t, t * t * t);
            let q = [0, 1, 2].map(|k| {
                0.5 * (2.0 * p1[k]
                    + (-p0[k] + p2[k]) * t
                    + (2.0 * p0[k] - 5.0 * p1[k] + 4.0 * p2[k] - p3[k]) * t2
                    + (-p0[k] + 3.0 * p1[k] - 3.0 * p2[k] + p3[k]) * t3)
            });
            out.push(q);
        }
    }
    out.push(points[n - 1]);
    out
}

/// How a swept tube ends.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Cap {
    /// Cut square.
    Flat,
    /// Rounded over, as a half ball of its radius (`k` times as long).
    Round(f64),
}

/// A tube along `path`, its radius `radius(t)` for `t` from 0 at the start
/// to 1 at the end (by length), `seg` sides, closed at both ends. The
/// rings are carried along without twisting, so a bent tube stays smooth.
pub fn sweep(
    b: &mut Builder,
    role: &'static str,
    path: &[P3],
    radius: impl Fn(f64) -> f64,
    seg: usize,
    start: Cap,
    end: Cap,
) {
    let mut path: Vec<P3> = path.to_vec();
    path.dedup_by(|a, b| len(sub(*a, *b)) < 1e-7);
    let n = path.len();
    if n < 2 {
        return;
    }
    let seg = seg.max(3);
    // Length along the path, for the radius.
    let mut at = vec![0.0; n];
    for i in 1..n {
        at[i] = at[i - 1] + len(sub(path[i], path[i - 1]));
    }
    let total = at[n - 1].max(1e-9);
    let tangent = |i: usize| -> P3 {
        let (a, c) = (path[i.saturating_sub(1)], path[(i + 1).min(n - 1)]);
        norm(sub(c, a))
    };
    let t0 = tangent(0);
    let helper = if t0[2].abs() < 0.9 {
        [0.0, 0.0, 1.0]
    } else {
        [1.0, 0.0, 0.0]
    };
    let mut x = norm(cross(helper, t0));
    let mut frames: Vec<(P3, P3, P3)> = Vec::with_capacity(n);
    for i in 0..n {
        let t = tangent(i);
        // Carried onto the plane across the tangent.
        x = norm(sub(x, mul(t, dot(x, t))));
        frames.push((t, x, cross(t, x)));
    }
    let ring = |b: &mut Builder, c: P3, x: P3, y: P3, r: f64| -> Vec<u32> {
        (0..seg)
            .map(|k| {
                let a = TAU * k as f64 / seg as f64;
                let (s, co) = a.sin_cos();
                b.point(add(c, add(mul(x, r * co), mul(y, r * s))))
            })
            .collect()
    };
    let mut rings: Vec<Vec<u32>> = Vec::new();
    // The rounded start, from its tip.
    let r0 = radius(0.0).max(1e-5);
    let mut tip_start = None;
    if let Cap::Round(k) = start {
        let (t, x, y) = frames[0];
        tip_start = Some(b.point(sub(path[0], mul(t, r0 * k))));
        for s in 1..4 {
            let phi = PI / 2.0 * s as f64 / 4.0;
            let c = sub(path[0], mul(t, r0 * k * phi.cos()));
            rings.push(ring(b, c, x, y, r0 * phi.sin()));
        }
    }
    for i in 0..n {
        let (_, x, y) = frames[i];
        rings.push(ring(b, path[i], x, y, radius(at[i] / total).max(1e-5)));
    }
    let r1 = radius(1.0).max(1e-5);
    let mut tip_end = None;
    if let Cap::Round(k) = end {
        let (t, x, y) = frames[n - 1];
        for s in (1..4).rev() {
            let phi = PI / 2.0 * s as f64 / 4.0;
            let c = add(path[n - 1], mul(t, r1 * k * phi.cos()));
            rings.push(ring(b, c, x, y, r1 * phi.sin()));
        }
        tip_end = Some(b.point(add(path[n - 1], mul(t, r1 * k))));
    }
    for j in 0..rings.len() - 1 {
        for k in 0..seg {
            let k1 = (k + 1) % seg;
            b.quad(
                role,
                rings[j][k],
                rings[j][k1],
                rings[j + 1][k1],
                rings[j + 1][k],
            );
        }
    }
    let first = &rings[0];
    let last = &rings[rings.len() - 1];
    let ca = tip_start.unwrap_or_else(|| b.point(path[0]));
    let cb = tip_end.unwrap_or_else(|| b.point(path[n - 1]));
    for k in 0..seg {
        let k1 = (k + 1) % seg;
        b.tri(role, ca, first[k1], first[k]);
        b.tri(role, cb, last[k], last[k1]);
    }
}

/// A profile `(radius, height)` turned about the vertical through `c`
/// with `sides` sides (4: square, 8: octagonal, 24 or more: round), the
/// first corner at `phase` radians; radii to the corners. Closed at its
/// ends where they are off the axis.
pub fn turned(
    b: &mut Builder,
    role: &'static str,
    c: P3,
    profile: &[(f64, f64)],
    sides: usize,
    phase: f64,
) {
    let mut prof: Vec<(f64, f64)> = profile.to_vec();
    if prof.len() < 2 {
        return;
    }
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
    let sides = sides.max(3);
    let rings: Vec<Vec<u32>> = prof
        .iter()
        .map(|(r, z)| {
            (0..sides)
                .map(|k| {
                    let t = phase + TAU * k as f64 / sides as f64;
                    b.point([c[0] + r * t.cos(), c[1] + r * t.sin(), c[2] + z])
                })
                .collect()
        })
        .collect();
    for j in 0..rings.len() - 1 {
        for k in 0..sides {
            let k1 = (k + 1) % sides;
            b.quad(
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
        let m = b.point([c[0], c[1], c[2] + z0]);
        for k in 0..sides {
            b.tri(role, m, rings[0][(k + 1) % sides], rings[0][k]);
        }
    }
    let (rn, zn) = prof[prof.len() - 1];
    let last = rings.len() - 1;
    if rn > 1e-6 {
        let m = b.point([c[0], c[1], c[2] + zn]);
        for k in 0..sides {
            b.tri(role, m, rings[last][k], rings[last][(k + 1) % sides]);
        }
    }
}

/// A thin plate on the quadrilateral `q` (corners counter-clockwise seen
/// from its front), `t` thick, centred on it.
pub fn plate(b: &mut Builder, role: &'static str, q: [P3; 4], t: f64) {
    let n = norm(cross(sub(q[1], q[0]), sub(q[3], q[0])));
    let top: Vec<u32> = q
        .iter()
        .map(|p| b.point(add(*p, mul(n, t / 2.0))))
        .collect();
    let bot: Vec<u32> = q
        .iter()
        .map(|p| b.point(sub(*p, mul(n, t / 2.0))))
        .collect();
    b.quad(role, top[0], top[1], top[2], top[3]);
    b.quad(role, bot[3], bot[2], bot[1], bot[0]);
    for i in 0..4 {
        let j = (i + 1) % 4;
        b.quad(role, bot[i], bot[j], top[j], top[i]);
    }
}

/// A rod of square section from `a` to `b`, `w` wide, its faces turned
/// to `side` (any direction across it).
pub fn bar(b: &mut Builder, role: &'static str, a: P3, c: P3, w: f64, side: P3) {
    let t = norm(sub(c, a));
    let x = norm(sub(side, mul(t, dot(side, t))));
    let y = cross(t, x);
    let h = w / 2.0;
    let mid = lerp(a, c, 0.5);
    let half = len(sub(c, a)) / 2.0;
    b.obox(role, mid, mul(x, h), mul(y, h), mul(t, half));
}

/// A rounded rectangle across `(cx, cy)` at height `z`: half sides `hx`,
/// `hy`, corners of radius `r`, `per` points to a corner,
/// counter-clockwise seen from above.
pub fn rounded_rect(c: [f64; 2], hx: f64, hy: f64, r: f64, z: f64, per: usize) -> Vec<P3> {
    let r = r.min(hx).min(hy).max(1e-4);
    let corners = [
        (hx - r, -hy + r, -PI / 2.0),
        (hx - r, hy - r, 0.0),
        (-hx + r, hy - r, PI / 2.0),
        (-hx + r, -hy + r, PI),
    ];
    let mut out = Vec::with_capacity(4 * per);
    for (x, y, a0) in corners {
        for s in 0..per {
            let a = a0 + PI / 2.0 * s as f64 / (per - 1).max(1) as f64;
            out.push([c[0] + x + r * a.cos(), c[1] + y + r * a.sin(), z]);
        }
    }
    out
}

/// A shell through rounded rectangles `(hx, hy, r, z)` about `c`, the
/// last joined back to the first: the section of the shell goes round its
/// wall once (a sink's basin with its rim, inside and outside).
pub fn loft_loop(b: &mut Builder, role: &'static str, c: [f64; 2], rings: &[(f64, f64, f64, f64)]) {
    let n = rings.len();
    if n < 3 {
        return;
    }
    // Counter-clockwise in (size, height), as a turned profile.
    let area: f64 = (0..n)
        .map(|i| {
            let (a, b) = (rings[i], rings[(i + 1) % n]);
            let (ra, rb) = ((a.0 + a.1) / 2.0, (b.0 + b.1) / 2.0);
            ra * b.3 - rb * a.3
        })
        .sum();
    let mut rings = rings.to_vec();
    if area < 0.0 {
        rings.reverse();
    }
    let per = 6;
    let ids: Vec<Vec<u32>> = rings
        .iter()
        .map(|(hx, hy, r, z)| {
            rounded_rect(c, *hx, *hy, *r, *z, per)
                .into_iter()
                .map(|p| b.point(p))
                .collect()
        })
        .collect();
    let m = ids[0].len();
    for j in 0..n {
        let j1 = (j + 1) % n;
        for k in 0..m {
            let k1 = (k + 1) % m;
            b.quad(role, ids[j][k], ids[j][k1], ids[j1][k1], ids[j1][k]);
        }
    }
}

/// The outline of a leaf: its half width at `u` (0 at the stalk, 1 at
/// the tip), as a share of the widest.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Outline {
    /// Long and pointed at both ends, widest a little below the middle
    /// (a peace lily, a calathea).
    Lance,
    /// Round (a Chinese money plant).
    Round,
    /// Heart-shaped: wide, rounded at the stalk, pointed at the tip.
    Heart,
}

impl Outline {
    fn width(self, u: f64) -> f64 {
        let u = u.clamp(0.0, 1.0);
        match self {
            Outline::Lance => {
                // Pointed at the tip, narrowing at the stalk.
                let s = (PI * u.powf(0.78)).sin().max(0.0);
                s.powf(0.85)
            }
            Outline::Round => (1.0 - (2.0 * u - 1.0).powi(2)).max(0.0).sqrt(),
            Outline::Heart => {
                let s = (PI * u.powf(0.62)).sin().max(0.0);
                s.powf(0.6) * (1.0 - 0.15 * u)
            }
        }
    }
}

/// A leaf's shape: sizes, how it bends and what it is like.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Leaf {
    pub length: f64,
    pub width: f64,
    pub outline: Outline,
    /// The two halves raised along the midrib (a share of the half
    /// width): a V, 0 flat.
    pub fold: f64,
    /// How much it bends down by its tip (radians turned from the stalk
    /// to the tip); negative curls it up.
    pub droop: f64,
    /// Twisted about its midrib from stalk to tip (radians).
    pub twist: f64,
    /// Swell of the whole blade (a share of the length): positive domes
    /// it up, as a round leaf held out flat.
    pub dome: f64,
    /// A wavy margin (a share of the half width).
    pub wave: f64,
    /// Side veins pressed into the blade: how many to a side, and how
    /// deep (metres).
    pub veins: usize,
    pub vein_depth: f64,
    /// Where the stalk meets the blade (a share of the length from its
    /// base: 0 at the edge; a round leaf held from beneath, about 0.3).
    pub attach: f64,
    /// Thickness in the middle.
    pub thick: f64,
    /// Points along and across (each half).
    pub rows: usize,
    pub cols: usize,
}

impl Leaf {
    /// A plain leaf of these sizes, to be adjusted.
    pub fn new(length: f64, width: f64, outline: Outline) -> Self {
        Leaf {
            length,
            width,
            outline,
            fold: 0.12,
            droop: 0.5,
            twist: 0.0,
            dome: 0.0,
            wave: 0.0,
            veins: 0,
            vein_depth: 0.0,
            attach: 0.0,
            thick: 0.0015,
            rows: 16,
            cols: 4,
        }
    }
}

/// Makes a leaf whose stalk meets it at `base`, heading along `dir`, its
/// upper face towards `up` (across `dir`). Returns where its midrib runs
/// (points from the stalk to the tip, on its upper face), for a rib to be
/// laid along it.
pub fn leaf(
    b: &mut Builder,
    role: &'static str,
    base: P3,
    dir: P3,
    up: P3,
    l: &Leaf,
    seed: u32,
) -> Vec<P3> {
    let rows = l.rows.max(4);
    let cols = l.cols.max(2);
    let dir = norm(dir);
    let up = norm(sub(up, mul(dir, dot(up, dir))));
    // The midrib, bent down by its tip and twisted, with the frame of the
    // blade at each row.
    let mut centre = Vec::with_capacity(rows + 1);
    let mut frames = Vec::with_capacity(rows + 1);
    let step = l.length / rows as f64;
    let mut p = [0.0; 3];
    for i in 0..=rows {
        let u = i as f64 / rows as f64;
        // The droop grows towards the tip.
        let phi = l.droop * u * u;
        let t = add(mul(dir, phi.cos()), mul(up, -phi.sin()));
        let n0 = add(mul(up, phi.cos()), mul(dir, phi.sin()));
        let n = rotate(n0, t, l.twist * u);
        let s = cross(n, t);
        if i > 0 {
            p = add(p, mul(t, step));
        }
        centre.push(p);
        frames.push((t, s, n));
    }
    // Held at `attach`: the whole leaf moved so that point is at `base`.
    let ai = ((l.attach * rows as f64).round() as usize).min(rows);
    let shift = sub(base, centre[ai]);
    let hw0 = l.width / 2.0;
    let wob = |u: f64| -> f64 {
        let s = seed as f64 * 0.731;
        ((u * 9.0 + s).sin() * 0.6 + (u * 17.0 + s * 2.1).sin() * 0.4) * l.wave
    };
    // The surface, with the vein grooves and the swelling.
    let surface = |i: usize, v: f64| -> (P3, P3) {
        let u = i as f64 / rows as f64;
        let (_, s, n) = frames[i];
        let hw = hw0 * l.outline.width(u);
        let av = v.abs();
        // A V raised along the midrib, rounded at the rib.
        let fold = l.fold * hw * (av * av + 0.02).sqrt();
        let dome = if l.dome != 0.0 {
            let r2 = ((u - 0.5) * 2.0).powi(2) + v * v;
            l.dome * l.length * (1.0 - r2).max(0.0)
        } else {
            0.0
        };
        let wave = wob(u) * hw * av * av;
        let mut groove = 0.0;
        if l.veins > 0 && l.vein_depth > 0.0 {
            // Veins leave the midrib towards the tip and curve out.
            let k = (u - 0.32 * av.powf(0.8)) * l.veins as f64;
            let f = k - k.floor() - 0.5;
            let along = (-(f * f) / 0.02).exp();
            let fade = (av * 6.0).min(1.0) * (1.0 - av).max(0.0).powf(0.4) * (u * 4.0).min(1.0);
            groove = l.vein_depth * along * fade;
        }
        let q = add(
            add(centre[i], mul(s, v * hw)),
            mul(n, fold + dome + wave - groove),
        );
        (add(q, shift), n)
    };
    let tip_row = |i: usize| i == 0 || i == rows;
    // Points: shared at the two ends of the midrib and along the margin,
    // where the two faces meet.
    let half_t = |v: f64| l.thick / 2.0 * (1.0 - v * v).max(0.0).sqrt();
    let mut top = vec![vec![0u32; 2 * cols + 1]; rows + 1];
    let mut bot = vec![vec![0u32; 2 * cols + 1]; rows + 1];
    for i in 0..=rows {
        if tip_row(i) && l.outline.width(i as f64 / rows as f64) < 1e-3 {
            let (q, _) = surface(i, 0.0);
            let id = b.point(q);
            top[i].fill(id);
            bot[i].fill(id);
            continue;
        }
        for j in 0..=2 * cols {
            let v = j as f64 / cols as f64 - 1.0;
            let (q, n) = surface(i, v);
            if j == 0 || j == 2 * cols {
                let id = b.point(q);
                top[i][j] = id;
                bot[i][j] = id;
            } else {
                let h = half_t(v);
                top[i][j] = b.point(add(q, mul(n, h)));
                bot[i][j] = b.point(sub(q, mul(n, h)));
            }
        }
    }
    for i in 0..rows {
        for j in 0..2 * cols {
            // Upper face: along × across = up.
            b.quad(
                role,
                top[i][j],
                top[i + 1][j],
                top[i + 1][j + 1],
                top[i][j + 1],
            );
            b.quad(
                role,
                bot[i][j + 1],
                bot[i + 1][j + 1],
                bot[i + 1][j],
                bot[i][j],
            );
        }
    }
    // A blade with no width at its base (a round leaf) is closed there by
    // the shared margin points; one cut square (none of the outlines) is
    // left open, so close it.
    for i in [0, rows] {
        if l.outline.width(i as f64 / rows as f64) >= 1e-3 {
            for j in 0..2 * cols {
                if i == 0 {
                    b.quad(role, bot[0][j], bot[0][j + 1], top[0][j + 1], top[0][j]);
                } else {
                    b.quad(role, top[i][j], top[i][j + 1], bot[i][j + 1], bot[i][j]);
                }
            }
        }
    }
    (0..=rows)
        .filter(|i| *i >= ai)
        .map(|i| {
            let (q, n) = surface(i, 0.0);
            add(q, mul(n, half_t(0.0)))
        })
        .collect()
}

/// The volume a closed piece holds (positive when its triangles turn
/// counter-clockwise from outside): for the tests.
pub fn signed_volume(positions: &[[f32; 3]], tris: &[[u32; 3]]) -> f64 {
    tris.iter()
        .map(|t| {
            let p = t.map(|i| {
                let q = positions[i as usize];
                [q[0] as f64, q[1] as f64, q[2] as f64]
            });
            dot(p[0], cross(p[1], p[2])) / 6.0
        })
        .sum()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn volume(b: &Builder) -> f64 {
        b.parts
            .values()
            .map(|t| signed_volume(&b.positions, t))
            .sum()
    }

    #[test]
    fn a_swept_tube_is_closed_and_turned_out() {
        let mut b = Builder::default();
        let path = curve(
            &[
                [0.0, 0.0, 0.0],
                [0.0, 0.0, 0.3],
                [0.1, 0.0, 0.4],
                [0.2, 0.0, 0.3],
            ],
            8,
        );
        sweep(
            &mut b,
            "metal",
            &path,
            |_| 0.01,
            16,
            Cap::Flat,
            Cap::Round(1.0),
        );
        let v = volume(&b);
        // A tube of radius 1 cm about 55 cm long: about 1.7e-4 m³.
        assert!(v > 1.2e-4 && v < 2.2e-4, "{v}");
    }

    #[test]
    fn a_turned_square_is_a_box() {
        let mut b = Builder::default();
        let r = 0.5 * 2f64.sqrt();
        turned(
            &mut b,
            "frame",
            [0.0; 3],
            &[(r, 0.0), (r, 1.0)],
            4,
            PI / 4.0,
        );
        assert!((volume(&b) - 1.0).abs() < 1e-9, "{}", volume(&b));
    }

    #[test]
    fn a_plate_and_a_bar_are_turned_out() {
        let mut b = Builder::default();
        plate(
            &mut b,
            "glass",
            [
                [0.0, 0.0, 0.0],
                [1.0, 0.0, 0.0],
                [1.0, 0.0, 1.0],
                [0.0, 0.0, 1.0],
            ],
            0.01,
        );
        assert!((volume(&b) - 0.01).abs() < 1e-6);
        let mut b = Builder::default();
        bar(
            &mut b,
            "frame",
            [0.0; 3],
            [0.0, 0.0, 2.0],
            0.1,
            [1.0, 1.0, 0.0],
        );
        assert!((volume(&b) - 0.02).abs() < 1e-6, "{}", volume(&b));
    }

    #[test]
    fn a_lofted_shell_holds_its_wall() {
        let mut b = Builder::default();
        // A square ring wall 1 cm thick and 10 cm high.
        loft_loop(
            &mut b,
            "metal",
            [0.0, 0.0],
            &[
                (0.5, 0.5, 0.0, 0.0),
                (0.51, 0.51, 0.0, 0.0),
                (0.51, 0.51, 0.0, 0.1),
                (0.5, 0.5, 0.0, 0.1),
            ],
        );
        let v = volume(&b);
        // (1.02² − 1²) × 0.1, the corners being slightly rounded.
        assert!(v > 0.0038 && v < 0.0041, "{v}");
    }

    #[test]
    fn a_leaf_is_closed_and_thin() {
        for outline in [Outline::Lance, Outline::Round, Outline::Heart] {
            let mut b = Builder::default();
            let mut l = Leaf::new(0.2, 0.08, outline);
            l.veins = 6;
            l.vein_depth = 0.001;
            l.thick = 0.002;
            let rib = leaf(
                &mut b,
                "leaves",
                [0.0; 3],
                [1.0, 0.0, 0.0],
                [0.0, 0.0, 1.0],
                &l,
                3,
            );
            assert!(rib.len() > 4);
            let v = volume(&b);
            // About two thirds of length × width × half the thickness.
            assert!(v > 0.0 && v < 0.2 * 0.08 * 0.002, "{outline:?} {v}");
            // Every edge is shared by exactly two triangles: closed.
            let mut edges = std::collections::HashMap::new();
            for t in b.parts.values().flatten() {
                for k in 0..3 {
                    let (a, c) = (t[k], t[(k + 1) % 3]);
                    *edges.entry((a.min(c), a.max(c))).or_insert(0) += 1;
                }
            }
            assert!(edges.values().all(|n| *n == 2), "{outline:?}");
        }
    }
}
