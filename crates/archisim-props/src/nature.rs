//! Rocks and other natural things of the site (C8): a rock (rounded,
//! angular, a flat slab, or with moss on top), a group of rocks, a patch of
//! pebbles, a fallen log and a tree stump. Each model is made from its
//! measures and a `variant` number, so the same type is always the same
//! rock and several types of one family are different ones. Centred on the
//! insertion point, flat underneath, standing on the ground.

use std::f64::consts::TAU;

use crate::build::{norm, Builder, P3};
use crate::Params;

/// A small, repeatable random number generator.
struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Rng {
        Rng(seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) ^ 0x2545_F491_4F6C_DD1D | 1)
    }
    fn next(&mut self) -> f64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        (self.0 >> 11) as f64 / (1u64 << 53) as f64
    }
    fn dir(&mut self) -> P3 {
        let z = self.next() * 2.0 - 1.0;
        let a = self.next() * TAU;
        let r = (1.0 - z * z).sqrt();
        [r * a.cos(), r * a.sin(), z]
    }
}

/// The unit sphere as an icosahedron divided `n` times: its corners and
/// triangles (counter-clockwise from outside).
fn icosphere(n: usize) -> (Vec<P3>, Vec<[usize; 3]>) {
    let t = (1.0 + 5f64.sqrt()) / 2.0;
    let mut v: Vec<P3> = [
        [-1.0, t, 0.0],
        [1.0, t, 0.0],
        [-1.0, -t, 0.0],
        [1.0, -t, 0.0],
        [0.0, -1.0, t],
        [0.0, 1.0, t],
        [0.0, -1.0, -t],
        [0.0, 1.0, -t],
        [t, 0.0, -1.0],
        [t, 0.0, 1.0],
        [-t, 0.0, -1.0],
        [-t, 0.0, 1.0],
    ]
    .iter()
    .map(|p| norm(*p))
    .collect();
    let mut f: Vec<[usize; 3]> = vec![
        [0, 11, 5],
        [0, 5, 1],
        [0, 1, 7],
        [0, 7, 10],
        [0, 10, 11],
        [1, 5, 9],
        [5, 11, 4],
        [11, 10, 2],
        [10, 7, 6],
        [7, 1, 8],
        [3, 9, 4],
        [3, 4, 2],
        [3, 2, 6],
        [3, 6, 8],
        [3, 8, 9],
        [4, 9, 5],
        [2, 4, 11],
        [6, 2, 10],
        [8, 6, 7],
        [9, 8, 1],
    ];
    for _ in 0..n {
        let mut mids = std::collections::HashMap::new();
        let mut mid = |a: usize, b: usize, v: &mut Vec<P3>| -> usize {
            let k = (a.min(b), a.max(b));
            *mids.entry(k).or_insert_with(|| {
                let (p, q) = (v[a], v[b]);
                v.push(norm([p[0] + q[0], p[1] + q[1], p[2] + q[2]]));
                v.len() - 1
            })
        };
        let mut next = Vec::with_capacity(f.len() * 4);
        for [a, b, c] in f {
            let ab = mid(a, b, &mut v);
            let bc = mid(b, c, &mut v);
            let ca = mid(c, a, &mut v);
            next.extend([[a, ab, ca], [b, bc, ab], [c, ca, bc], [ab, bc, ca]]);
        }
        f = next;
    }
    (v, f)
}

/// How a rock is shaped.
#[derive(Clone, Copy, PartialEq)]
enum Shape {
    /// Worn round by water ("canto rodado").
    Rounded,
    /// Broken faces ("roca angulosa").
    Angular,
}

/// A rock centred at `c` (on the ground), `half` its half-width, -depth
/// and height, of shape `shape`; `moss` grows on its top faces.
fn rock(b: &mut Builder, c: P3, half: P3, shape: Shape, seed: u64, moss: bool) {
    let mut rng = Rng::new(seed);
    let (dirs, faces) = icosphere(2);
    // Broken: the sphere cut by planes at random; worn: bumps.
    let cuts: Vec<(P3, f64)> = (0..9)
        .map(|_| (rng.dir(), 0.62 + 0.3 * rng.next()))
        .collect();
    let bumps: Vec<(P3, f64)> = (0..6)
        .map(|_| (rng.dir(), (rng.next() - 0.5) * 0.22))
        .collect();
    let lean = (rng.next() - 0.5) * 0.3;
    let pts: Vec<P3> = dirs
        .iter()
        .map(|v| {
            let mut r = match shape {
                Shape::Angular => cuts
                    .iter()
                    .filter_map(|(n, o)| {
                        let d = v[0] * n[0] + v[1] * n[1] + v[2] * n[2];
                        (d > 1e-6).then(|| o / d)
                    })
                    .fold(1.0, f64::min),
                Shape::Rounded => {
                    1.0 + bumps
                        .iter()
                        .map(|(n, a)| {
                            a * (v[0] * n[0] + v[1] * n[1] + v[2] * n[2]).max(0.0).powi(2)
                        })
                        .sum::<f64>()
                }
            };
            r = r.clamp(0.35, 1.3);
            // Flat underneath, a little into the ground.
            let z = (v[2] * r).max(-0.35);
            let x = v[0] * r + lean * (z + 0.35);
            [
                c[0] + x * half[0],
                c[1] + v[1] * r * half[1],
                c[2] + (z + 0.35) / 1.35 * half[2],
            ]
        })
        .collect();
    let idx: Vec<u32> = pts.iter().map(|p| b.point(*p)).collect();
    for [p, q, r] in faces {
        let role = if moss {
            let (a, bb, cc) = (pts[p], pts[q], pts[r]);
            let u = [bb[0] - a[0], bb[1] - a[1], bb[2] - a[2]];
            let w = [cc[0] - a[0], cc[1] - a[1], cc[2] - a[2]];
            let n = norm(crate::build::cross(u, w));
            let high = (a[2] + bb[2] + cc[2]) / 3.0 > c[2] + half[2] * 0.45;
            if n[2] > 0.55 && high {
                "moss"
            } else {
                "mat"
            }
        } else {
            "mat"
        };
        b.tri(role, idx[p], idx[q], idx[r]);
    }
}

/// A rock: «Canto rodado», «Roca angulosa», «Laja» (a flat slab) or
/// «Roca con musgo».
pub fn rock_prop(b: &mut Builder, p: &Params) {
    let w = p.get("width", 0.8).clamp(0.1, 6.0);
    let d = p.get("depth", 0.6).clamp(0.1, 6.0);
    let h = p.get("height", 0.5).clamp(0.05, 4.0);
    let seed = p.get("variant", 1.0).round().max(0.0) as u64;
    let half = [w / 2.0, d / 2.0, h];
    match p.style() {
        1 => rock(b, [0.0, 0.0, 0.0], half, Shape::Rounded, seed, false),
        3 => rock(
            b,
            [0.0, 0.0, 0.0],
            [half[0], half[1], h.min(w.min(d) * 0.3)],
            Shape::Angular,
            seed + 31,
            false,
        ),
        4 => rock(b, [0.0, 0.0, 0.0], half, Shape::Angular, seed, true),
        _ => rock(b, [0.0, 0.0, 0.0], half, Shape::Angular, seed, false),
    }
}

/// A group of rocks in the footprint: a big one and smaller ones round it,
/// with pebbles.
pub fn rock_group(b: &mut Builder, p: &Params) {
    let w = p.get("width", 2.0).clamp(0.4, 12.0);
    let d = p.get("depth", 1.6).clamp(0.4, 12.0);
    let h = p.get("height", 0.9).clamp(0.1, 4.0);
    let seed = p.get("variant", 1.0).round().max(0.0) as u64;
    let mut rng = Rng::new(seed + 7);
    let shape = if p.style() == 2 {
        Shape::Rounded
    } else {
        Shape::Angular
    };
    let moss = p.style() == 3;
    // The big one, off centre.
    let big = [w * 0.27, d * 0.3, h];
    rock(b, [-w * 0.12, d * 0.05, 0.0], big, shape, seed, moss);
    for k in 0..4 {
        let a = TAU * (k as f64 / 4.0 + rng.next() * 0.2);
        let s = 0.35 + 0.3 * rng.next();
        let c = [a.cos() * w * 0.3 + w * 0.05, a.sin() * d * 0.3, 0.0];
        rock(
            b,
            c,
            [w * 0.2 * s, d * 0.2 * s, h * 0.6 * s],
            shape,
            seed * 13 + k,
            moss && k % 2 == 0,
        );
    }
    for k in 0..10 {
        let c = [
            (rng.next() - 0.5) * w * 0.9,
            (rng.next() - 0.5) * d * 0.9,
            0.0,
        ];
        let s = 0.04 + 0.05 * rng.next();
        rock(
            b,
            c,
            [s * 1.3, s, s * 0.6],
            Shape::Rounded,
            seed * 7 + k,
            false,
        );
    }
}

/// Pebbles over the footprint ("cantos rodados").
pub fn pebbles(b: &mut Builder, p: &Params) {
    let w = p.get("width", 1.2).clamp(0.2, 8.0);
    let d = p.get("depth", 1.2).clamp(0.2, 8.0);
    let size = p.get("height", 0.08).clamp(0.02, 0.4);
    let seed = p.get("variant", 1.0).round().max(0.0) as u64;
    let mut rng = Rng::new(seed + 3);
    let n = ((w * d) / (size * size * 6.0)).round().clamp(6.0, 220.0) as usize;
    for k in 0..n {
        let (u, v) = (rng.next() - 0.5, rng.next() - 0.5);
        // Within the ellipse of the footprint.
        if u * u + v * v > 0.25 {
            continue;
        }
        let s = size * (0.5 + rng.next());
        rock(
            b,
            [u * w, v * d, 0.0],
            [s * (0.9 + 0.5 * rng.next()), s, s * 0.55],
            Shape::Rounded,
            seed * 31 + k as u64,
            false,
        );
    }
}

/// A fallen log along its width, with its bark, its cut ends and a branch
/// or two.
pub fn log(b: &mut Builder, p: &Params) {
    let len = p.get("width", 2.4).clamp(0.4, 12.0);
    let dia = p.get("height", 0.45).clamp(0.08, 2.0);
    let r = dia / 2.0;
    let (a, z) = ([-len / 2.0, 0.0, r], [len / 2.0, 0.0, r]);
    b.cone("bark", a, z, r, r * 0.88, 14);
    // The cut ends, the wood showing.
    b.cone(
        "wood",
        [a[0] - 0.012, a[1], a[2]],
        [a[0] + 0.002, a[1], a[2]],
        r * 0.9,
        r * 0.9,
        14,
    );
    b.cone(
        "wood",
        [z[0] - 0.002, z[1], z[2]],
        [z[0] + 0.012, z[1], z[2]],
        r * 0.8,
        r * 0.8,
        14,
    );
    // Stubs of branches.
    let stub = |b: &mut Builder, at: f64, ang: f64| {
        let base = [at, 0.0, r * 0.9];
        let tip = [
            at + r * 0.6,
            ang.sin() * r * 1.6,
            r * 0.9 + ang.cos() * r * 1.6,
        ];
        b.cone("bark", base, tip, r * 0.28, r * 0.12, 8);
    };
    stub(b, -len * 0.18, 0.7);
    if len > 1.2 {
        stub(b, len * 0.22, -0.5);
    }
}

/// A tree stump with roots reaching into the ground and its cut top.
pub fn stump(b: &mut Builder, p: &Params) {
    let dia = p.get("width", 0.6).clamp(0.15, 3.0);
    let h = p.get("height", 0.45).clamp(0.08, 2.0);
    let r = dia / 2.0;
    b.lathe(
        "bark",
        [0.0, 0.0, 0.0],
        &[
            (0.0, 0.0),
            (r * 1.45, 0.0),
            (r * 1.15, 0.12),
            (r * 1.02, h * 0.45),
            (r, h),
            (0.0, h),
        ],
        1.0,
        1.0,
    );
    b.lathe(
        "wood",
        [0.0, 0.0, h - 0.04],
        &[(0.0, 0.0), (r * 0.94, 0.0), (r * 0.94, 0.05), (0.0, 0.05)],
        1.0,
        1.0,
    );
    // Roots.
    for k in 0..5 {
        let a = TAU * (k as f64 / 5.0 + 0.07 * (k % 2) as f64);
        let (ca, sa) = (a.cos(), a.sin());
        b.cone(
            "bark",
            [ca * r * 0.8, sa * r * 0.8, 0.12],
            [ca * r * 2.0, sa * r * 2.0, r * 0.1],
            r * 0.25,
            r * 0.08,
            8,
        );
    }
}
