//! Bars and cafés of the eight collections (paquete 7, tanda 7a): the bar
//! counter, the back bar with its bottles, the café table, the café chair
//! and the banquette, each as the collection makes it. The bar stools are
//! the collections' own (`chairs::stool`).
//!
//! Bar counter: `width`, `depth`, `height`; the staff behind at `y = 0`,
//! the customers in front, towards `+y`. Roles: `frame` (its front),
//! `masonry`, `accent` (tiles, laminate), `top` (the bar top), `carcass`
//! (the staff side), `metal` (foot rail, staff worktop), `dark`, `light`
//! (a lit strip, glowing at night) and `mat` (concrete).
//!
//! Back bar: `width`, `depth`, `height`; against the wall at `y = 0`.
//! Roles: `frame`, `front` (doors), `top`, `masonry`, `accent`, `metal`,
//! `dark`, `glass`, `mirror`, `light` and the bottles `bottle`, `bottle2`
//! and `bottle3`.
//!
//! Café table: `width` (its top), `height`; centred on `(0, width / 2)`.
//! Café chair: `width`, `depth`, `seat`, `height`; facing `+y`, its back
//! at `y = 0`. Banquette: `width`, `depth`, `seat`, `height`; its back on
//! the wall at `y = 0`.

use super::shape::{bar, curve, sweep, turned, Cap};
use super::sofas::{back_cushions, compass_leg, upholstered};
use super::soft::{knife_pillow, tube, Soft};
use super::storage::{doors, front, tapered_leg, toupie, Panel, Pull, T};
use super::study::band;
use crate::build::{Builder, P3};
use crate::Params;
use std::f64::consts::{FRAC_PI_4, PI, TAU};

/// The collection of a workshop model (its style, 1 to 8).
fn look(p: &Params) -> usize {
    (p.style() - 1).clamp(0, 7) as usize
}

/// A small repeatable random sequence.
struct Rng(u32);

impl Rng {
    fn next(&mut self) -> f64 {
        self.0 = self.0.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        (self.0 >> 8) as f64 / (1u32 << 24) as f64
    }
    fn range(&mut self, a: f64, b: f64) -> f64 {
        a + (b - a) * self.next()
    }
}

/// A round rod from `a` to `c`.
fn rod(b: &mut Builder, role: &'static str, a: P3, c: P3, r: f64) {
    sweep(b, role, &[a, c], |_| r, 12, Cap::Flat, Cap::Flat);
}

/// A closed ring of `n` points round `(cx, cy)` at `z`.
fn ring(c: [f64; 2], r: f64, z: f64, n: usize) -> Vec<P3> {
    (0..n)
        .map(|k| {
            let a = TAU * k as f64 / n as f64;
            [c[0] + r * a.cos(), c[1] + r * a.sin(), z]
        })
        .collect()
}

/// A round disc, `r` across and `t` thick, its underside at `z`, its edge
/// eased.
fn disc(b: &mut Builder, role: &'static str, c: [f64; 2], r: f64, z: f64, t: f64) {
    turned(
        b,
        role,
        [c[0], c[1], z],
        &[
            (0.0, 0.0),
            (r - 0.004, 0.0),
            (r, 0.004),
            (r, t - 0.004),
            (r - 0.004, t),
            (0.0, t),
        ],
        40,
        0.0,
    );
}

/// Turns what was made since `start` half round about the upright line
/// through `(0, y)`: built facing `+y`, it faces `-y` (a rotation, not a
/// mirror, so it stays closed outwards).
fn turn_back(b: &mut Builder, start: usize, y: f64) {
    b.transform_from(start, |q| [-q[0], 2.0 * y - q[1], q[2]]);
}

/// Bottles standing in a row from `x0` to `x1` on a shelf at `z`, their
/// middles at `y`, none taller than `tall`.
fn bottles(b: &mut Builder, x0: f64, x1: f64, y: f64, z: f64, tall: f64, seed: u32) {
    let mut rnd = Rng(0x2545_f491 ^ seed);
    let roles = ["bottle", "bottle2", "bottle3"];
    let mut x = x0 + 0.05;
    while x < x1 - 0.05 {
        let r = rnd.range(0.03, 0.04);
        let h = rnd.range(0.22, 0.32).min(tall).max(0.12);
        let role = roles[(rnd.next() * 3.0) as usize % 3];
        turned(
            b,
            role,
            [x, y + rnd.range(-0.02, 0.02), z],
            &[
                (0.0, 0.0),
                (r, 0.0),
                (r, h * 0.62),
                (r * 0.45, h * 0.78),
                (r * 0.28, h * 0.84),
                (r * 0.28, h),
                (0.0, h),
            ],
            10,
            0.0,
        );
        x += 2.0 * r + rnd.range(0.008, 0.03);
    }
}

/// A foot rail along the front of a bar at `y`, `out` in front of it and
/// `z` high, from `x0` to `x1`, held by brackets about every metre; with
/// rounded ends.
#[allow(clippy::too_many_arguments)]
fn foot_rail(
    b: &mut Builder,
    role: &'static str,
    x0: f64,
    x1: f64,
    y: f64,
    out: f64,
    z: f64,
    r: f64,
) {
    let yr = y + out;
    sweep(
        b,
        role,
        &[[x0, yr, z], [x1, yr, z]],
        |_| r,
        14,
        Cap::Round(1.0),
        Cap::Round(1.0),
    );
    let n = ((x1 - x0) / 1.0).ceil().max(1.0) as usize + 1;
    for i in 0..n {
        let x = x0 + 0.08 + (x1 - x0 - 0.16) * i as f64 / (n - 1) as f64;
        rod(b, role, [x, y, z + r * 0.4], [x, yr, z], r * 0.55);
        b.cyl(
            role,
            [x, y, z + r * 0.4],
            [x, y + 0.006, z + r * 0.4],
            r * 1.3,
        );
    }
}

/// A block from `lo` to `hi` whose front (`+y`) is bevelled all round
/// `k` of its size: a dressed stone, light on triangles.
fn stone(b: &mut Builder, role: &'static str, lo: P3, hi: P3, k: f64) {
    let start = b.mark();
    let r = std::f64::consts::SQRT_2 / 2.0;
    turned(
        b,
        role,
        [0.0; 3],
        &[
            (0.0, 0.0),
            (r, 0.0),
            (r, 1.0 - k),
            (r * (1.0 - 2.0 * k), 1.0),
            (0.0, 1.0),
        ],
        4,
        FRAC_PI_4,
    );
    let c = [(lo[0] + hi[0]) / 2.0, (lo[2] + hi[2]) / 2.0];
    let (sx, sz, sy) = (hi[0] - lo[0], hi[2] - lo[2], hi[1] - lo[1]);
    // (across, up, out) to (x, y, z): a turn, not a mirror.
    b.transform_from(start, |q| {
        [c[0] + q[0] * sx, lo[1] + q[2] * sy, c[1] - q[1] * sz]
    });
}

/// A barrel standing at `c`, `r` round at its belly and `h` high: staves
/// with their joints and four iron hoops.
fn barrel(b: &mut Builder, c: [f64; 2], r: f64, h: f64) {
    let at = |z: f64| r * (0.84 + 0.16 * (PI * z / h).sin());
    let prof: Vec<(f64, f64)> = (0..=8)
        .map(|k| {
            let z = h * k as f64 / 8.0;
            (at(z), z)
        })
        .collect();
    let mut p = vec![(0.0, 0.0)];
    p.extend(prof);
    p.push((0.0, h));
    turned(b, "frame", [c[0], c[1], 0.0], &p, 24, 0.0);
    for z in [0.1, 0.27, 0.73, 0.9] {
        let zz = h * z;
        tube(b, "metal", &ring(c, at(zz) + 0.002, zz, 36), true, 0.007, 5);
    }
}

// ------------------------------------------------------------ bar counter

/// Bar counters of the collections.
pub fn bar_counter(b: &mut Builder, p: &Params) {
    let ix = look(p);
    let w = p.get("width", 2.4).clamp(1.0, 8.0);
    let d = p.get("depth", 0.8).clamp(0.6, 1.1);
    let h = p.get("height", 1.1).clamp(1.0, 1.2);
    let (x0, x1) = (-w / 2.0, w / 2.0);
    let over = 0.25;
    // The customers' face of the front.
    let ds = d - over;
    let t = [0.035, 0.04, 0.035, 0.03, 0.02, 0.07, 0.045, 0.04][ix];
    let zt = h - t;
    let face: &'static str = match ix {
        1 | 5 => "masonry",
        2 => "accent",
        4 => "mat",
        6 => "dark",
        _ => "frame",
    };
    // The staff side: a carcass of cupboards under a steel worktop.
    let side = 0.03;
    b.rbox(
        "dark",
        [x0 + 0.05, 0.06, 0.0],
        [x1 - 0.05, ds - 0.1, 0.1],
        0.0,
    );
    b.rbox(
        "carcass",
        [x0 + side, 0.03, 0.1],
        [x1 - side, ds - 0.06, 0.88],
        0.0,
    );
    b.rbox(
        "metal",
        [x0 + side, 0.0, 0.88],
        [x1 - side, ds - 0.06, 0.9],
        0.002,
    );
    let n = ((w - 2.0 * side) / 0.6).round().max(1.0) as usize;
    let start = b.mark();
    doors(
        b,
        Panel::Flat,
        "carcass",
        x0 + side + 0.005,
        x1 - side - 0.005,
        0.11,
        0.86,
        0.03,
        n,
        Pull::Bar(0.14),
        0.7,
    );
    turn_back(b, start, 0.03);
    // A glass shelf under the top for the staff.
    b.rbox(
        "glass",
        [x0 + side, 0.12, zt - 0.12],
        [x1 - side, ds - 0.06, zt - 0.11],
        0.0,
    );
    // The ends of the bar.
    for (a, e) in [(x0, x0 + side), (x1 - side, x1)] {
        b.rbox(face, [a, 0.0, 0.0], [e, ds, zt], 0.003);
    }
    // The front, in the collection's way.
    let fy = ds - 0.06;
    match ix {
        // Nordic: fluted oak, an oak top, a black rail.
        0 => {
            b.rbox(
                "dark",
                [x0 + side, fy + 0.01, 0.0],
                [x1 - side, fy + 0.03, 0.1],
                0.0,
            );
            b.rbox("frame", [x0 + side, fy, 0.1], [x1 - side, ds - T, zt], 0.0);
            front(
                b,
                Panel::Flutes,
                "frame",
                x0 + side,
                x1 - side,
                0.1,
                zt,
                ds - T,
            );
            b.rbox("top", [x0 - 0.02, fy - 0.06, zt], [x1 + 0.02, d, h], 0.01);
            foot_rail(b, "dark", x0 + 0.1, x1 - 0.1, ds, 0.17, 0.22, 0.02);
        }
        // Mediterranean: whitewashed masonry with a band of tiles under a
        // wooden top, an iron rail.
        1 => {
            b.rbox("masonry", [x0, fy - 0.04, 0.0], [x1, ds, zt], 0.02);
            let tile = 0.15;
            let k = ((w - 0.06) / tile).floor().max(1.0) as usize;
            let x = -(k as f64) * tile / 2.0;
            for i in 0..k {
                let xa = x + tile * i as f64;
                for (j, z) in [zt - 0.33, zt - 0.18].into_iter().enumerate() {
                    let role = if (i + j) % 2 == 0 {
                        "accent"
                    } else {
                        "masonry"
                    };
                    b.rbox(
                        role,
                        [xa + 0.003, ds, z + 0.003],
                        [xa + tile - 0.003, ds + 0.008, z + tile - 0.003],
                        0.0,
                    );
                }
            }
            b.rbox("top", [x0 - 0.03, fy - 0.08, zt], [x1 + 0.03, d, h], 0.006);
            foot_rail(b, "metal", x0 + 0.1, x1 - 0.1, ds, 0.17, 0.2, 0.017);
        }
        // Fifties: coloured laminate with chrome bands, a teak top with a
        // chrome edge, a chrome rail.
        2 => {
            b.rbox(
                "metal",
                [x0 + side, fy, 0.0],
                [x1 - side, ds + 0.004, 0.1],
                0.003,
            );
            b.rbox("accent", [x0 + side, fy, 0.1], [x1 - side, ds, zt], 0.0);
            for z in [0.32, 0.56, 0.8] {
                let z = z * zt;
                b.rbox(
                    "metal",
                    [x0 + side, ds, z - 0.012],
                    [x1 - side, ds + 0.006, z + 0.012],
                    0.003,
                );
            }
            b.rbox("top", [x0 - 0.02, fy - 0.06, zt], [x1 + 0.02, d, h], 0.012);
            sweep(
                b,
                "metal",
                &[
                    [x0 - 0.02, d + 0.004, zt + t / 2.0],
                    [x1 + 0.02, d + 0.004, zt + t / 2.0],
                ],
                |_| t * 0.55,
                12,
                Cap::Round(1.0),
                Cap::Round(1.0),
            );
            foot_rail(b, "metal", x0 + 0.1, x1 - 0.1, ds, 0.17, 0.22, 0.022);
        }
        // Japandi: upright battens of smoked oak on a dark ground, a pale
        // stone top, a foot rest of oak.
        3 => {
            b.rbox("dark", [x0 + side, fy, 0.0], [x1 - side, ds - T, zt], 0.0);
            front(
                b,
                Panel::Ribs,
                "frame",
                x0 + side,
                x1 - side,
                0.08,
                zt,
                ds - T,
            );
            b.rbox("top", [x0 - 0.01, fy - 0.06, zt], [x1 + 0.01, d, h], 0.004);
            let n = ((w / 1.0).ceil() as usize + 1).max(2);
            for i in 0..n {
                let x = x0 + 0.15 + (w - 0.3) * i as f64 / (n - 1) as f64;
                b.rbox(
                    "frame",
                    [x - 0.02, ds, 0.0],
                    [x + 0.02, ds + 0.2, 0.17],
                    0.003,
                );
            }
            b.rbox(
                "frame",
                [x0 + 0.1, ds + 0.02, 0.17],
                [x1 - 0.1, ds + 0.22, 0.2],
                0.006,
            );
        }
        // Contemporary: a monolith of microcement flush with a thin top, a
        // lit strip under the top and at the floor, a flat steel rail.
        4 => {
            b.rbox(
                "dark",
                [x0 + side, fy - 0.02, 0.0],
                [x1 - side, ds - 0.03, 0.06],
                0.0,
            );
            b.rbox(
                "light",
                [x0 + side, ds - 0.03, 0.05],
                [x1 - side, ds - 0.025, 0.06],
                0.0,
            );
            b.rbox("mat", [x0, fy - 0.02, 0.06], [x1, ds, zt], 0.004);
            b.rbox("top", [x0, fy - 0.06, zt], [x1, d, h], 0.003);
            b.rbox(
                "light",
                [x0 + 0.05, ds + 0.002, zt - 0.006],
                [x1 - 0.05, ds + 0.012, zt],
                0.0,
            );
            b.rbox(
                "dark",
                [x0 + 0.1, ds + 0.15, 0.2],
                [x1 - 0.1, ds + 0.19, 0.212],
                0.002,
            );
            let n = ((w / 1.0).ceil() as usize + 1).max(2);
            for i in 0..n {
                let x = x0 + 0.15 + (w - 0.3) * i as f64 / (n - 1) as f64;
                b.rbox(
                    "dark",
                    [x - 0.004, ds, 0.19],
                    [x + 0.004, ds + 0.17, 0.212],
                    0.0,
                );
            }
        }
        // Rustic: stone in courses, a thick plank on it, an iron rail.
        5 => {
            b.rbox("masonry", [x0, fy - 0.02, 0.0], [x1, ds - 0.01, zt], 0.01);
            let mut rnd = Rng(77);
            let course = 0.16;
            let rows = (zt / course).floor().max(1.0) as usize;
            for r in 0..rows {
                let (za, ze) = (course * r as f64, (course * (r + 1) as f64).min(zt));
                let mut x = x0 + if r % 2 == 0 { 0.0 } else { -0.12 };
                while x < x1 {
                    let l = rnd.range(0.22, 0.42);
                    let (xa, xe) = (x.max(x0) + 0.008, (x + l).min(x1) - 0.008);
                    if xe - xa > 0.05 {
                        let out = rnd.range(0.004, 0.014);
                        stone(
                            b,
                            "masonry",
                            [xa, ds - 0.02, za + 0.008],
                            [xe, ds - 0.01 + out, ze - 0.008],
                            0.18,
                        );
                    }
                    x += l;
                }
            }
            b.rbox("top", [x0 - 0.03, fy - 0.08, zt], [x1 + 0.03, d, h], 0.012);
            foot_rail(b, "metal", x0 + 0.1, x1 - 0.1, ds, 0.17, 0.2, 0.018);
        }
        // Industrial: planks laid across in a frame of black steel with its
        // rivets, a wooden top, a rail of pipe.
        6 => {
            b.rbox("dark", [x0 + side, fy, 0.0], [x1 - side, ds, 0.12], 0.0);
            b.rbox("dark", [x0 + side, fy, zt - 0.06], [x1 - side, ds, zt], 0.0);
            b.rbox(
                "frame",
                [x0 + side, fy, 0.12],
                [x1 - side, ds - 0.01, zt - 0.06],
                0.0,
            );
            let rows = ((zt - 0.18) / 0.14).floor().max(1.0) as usize;
            let step = (zt - 0.18) / rows as f64;
            for r in 0..rows {
                let z = 0.12 + step * r as f64;
                b.rbox(
                    "frame",
                    [x0 + side, ds - 0.01, z + 0.004],
                    [x1 - side, ds + 0.004, z + step - 0.004],
                    0.003,
                );
            }
            let posts = ((w / 1.2).ceil() as usize + 1).max(2);
            for i in 0..posts {
                let x = x0 + side + 0.03 + (w - 2.0 * side - 0.06) * i as f64 / (posts - 1) as f64;
                b.rbox("dark", [x - 0.03, ds, 0.0], [x + 0.03, ds + 0.008, zt], 0.0);
                for k in 0..6 {
                    let z = 0.06 + (zt - 0.12) * k as f64 / 5.0;
                    b.ellipsoid("dark", [x, ds + 0.009, z], [0.006, 0.004, 0.006]);
                }
            }
            b.rbox("top", [x0 - 0.02, fy - 0.06, zt], [x1 + 0.02, d, h], 0.004);
            let (xa, xb, yr, z) = (x0 + 0.12, x1 - 0.12, ds + 0.17, 0.2);
            sweep(
                b,
                "metal",
                &[[xa, yr, z], [xb, yr, z]],
                |_| 0.021,
                14,
                Cap::Flat,
                Cap::Flat,
            );
            for x in [xa, xb] {
                b.ellipsoid("metal", [x, yr, z], [0.026, 0.026, 0.026]);
                rod(b, "metal", [x, ds, z], [x, yr, z], 0.021);
                b.cyl("metal", [x, ds, z], [x, ds + 0.012, z], 0.04);
            }
        }
        // Classic: raised panels of wood between pilasters, a moulded base,
        // a marble top, a brass rail with its brackets.
        _ => {
            b.rbox("frame", [x0 + side, fy, 0.0], [x1 - side, ds - T, zt], 0.0);
            b.rbox(
                "frame",
                [x0 + side, ds - T, 0.0],
                [x1 - side, ds + 0.02, 0.1],
                0.004,
            );
            b.rbox(
                "frame",
                [x0 + side, ds - T, 0.1],
                [x1 - side, ds + 0.01, 0.13],
                0.004,
            );
            b.rbox(
                "frame",
                [x0 + side, ds - T, zt - 0.08],
                [x1 - side, ds + 0.02, zt],
                0.004,
            );
            b.rbox(
                "frame",
                [x0 + side, ds - T, zt - 0.11],
                [x1 - side, ds + 0.01, zt - 0.08],
                0.004,
            );
            let bays = ((w - 2.0 * side) / 0.65).round().max(1.0) as usize;
            let bw = (w - 2.0 * side) / bays as f64;
            for i in 0..bays {
                let xa = x0 + side + bw * i as f64;
                front(
                    b,
                    Panel::Raised,
                    "frame",
                    xa + 0.06,
                    xa + bw - 0.06,
                    0.18,
                    zt - 0.16,
                    ds - T,
                );
            }
            for i in 0..=bays {
                let x = x0 + side + bw * i as f64;
                let (xa, xe) = ((x - 0.035).max(x0 + side), (x + 0.035).min(x1 - side));
                b.rbox(
                    "frame",
                    [xa, ds - T, 0.13],
                    [xe, ds + 0.015, zt - 0.11],
                    0.004,
                );
            }
            b.rbox("top", [x0 - 0.03, fy - 0.06, zt], [x1 + 0.03, d, h], 0.012);
            let (yr, z, r) = (ds + 0.17, 0.2, 0.02);
            foot_rail(b, "metal", x0 + 0.1, x1 - 0.1, ds, 0.17, z, r);
            for x in [x0 + 0.1, x1 - 0.1] {
                b.ellipsoid("metal", [x, yr, z], [0.028, 0.028, 0.028]);
            }
        }
    }
}

// --------------------------------------------------------------- back bar

/// A round-headed niche's arch: the wall between the top of the opening
/// at `zs` (where the arch springs) and `zt`, from `xa` to `xe`, `y0` to
/// `y1` deep. Made of thin convex slices.
#[allow(clippy::too_many_arguments)]
fn arch_head(
    b: &mut Builder,
    role: &'static str,
    xa: f64,
    xe: f64,
    zs: f64,
    zt: f64,
    y0: f64,
    y1: f64,
) {
    let r = (xe - xa) / 2.0;
    let cx = (xa + xe) / 2.0;
    let n = 12;
    let start = b.mark();
    for k in 0..n {
        let (a0, a1) = (
            PI - PI * k as f64 / n as f64,
            PI - PI * (k + 1) as f64 / n as f64,
        );
        let (p0, p1) = (
            [cx + r * a0.cos(), zs + r * a0.sin()],
            [cx + r * a1.cos(), zs + r * a1.sin()],
        );
        // In (x, height), counter-clockwise: the arc and up to the top.
        b.slab(role, &[p0, p1, [p1[0], zt], [p0[0], zt]], -y1, -y0);
    }
    // From (x, height, -depth) to (x, depth, height): a quarter turn about x.
    b.transform_from(start, |q| [q[0], -q[2], q[1]]);
}

/// Back bars of the collections.
pub fn back_bar(b: &mut Builder, p: &Params) {
    let ix = look(p);
    let w = p.get("width", 2.4).clamp(1.0, 6.0);
    let d = p.get("depth", 0.5).clamp(0.35, 0.7);
    let h = p.get("height", 2.4).clamp(1.6, 3.0);
    let (x0, x1) = (-w / 2.0, w / 2.0);
    let low = 0.9;
    let top_t = 0.03;
    let zw = low + top_t;
    // The cupboards under the worktop.
    let (panel, pk, frole) = match ix {
        0 => (Panel::Flat, Pull::Slot, "front"),
        1 => (Panel::Planks, Pull::Ring, "front"),
        2 => (Panel::Flat, Pull::Brass, "front"),
        3 => (Panel::Ribs, Pull::Tab, "front"),
        4 => (Panel::Flat, Pull::Slot, "front"),
        5 => (Panel::Planks, Pull::Ring, "front"),
        6 => (Panel::Mesh, Pull::Bar(0.12), "front"),
        _ => (Panel::Raised, Pull::Brass, "front"),
    };
    let carcass = if ix == 1 { "masonry" } else { "frame" };
    let plinth = if ix == 1 { 0.0 } else { 0.1 };
    if plinth > 0.0 {
        let role = if matches!(ix, 0 | 4) { "dark" } else { carcass };
        b.rbox(
            role,
            [x0 + 0.02, 0.0, 0.0],
            [x1 - 0.02, d - T - 0.05, plinth],
            0.0,
        );
    }
    b.rbox(carcass, [x0, 0.0, plinth], [x1, d - T, low], 0.0);
    let n = (w / 0.6).round().max(1.0) as usize;
    doors(
        b,
        panel,
        frole,
        x0 + 0.003,
        x1 - 0.003,
        plinth + 0.005,
        low - 0.005,
        d - T,
        n,
        pk,
        low - 0.12,
    );
    let top_role = if ix == 1 { "accent" } else { "top" };
    b.rbox(
        top_role,
        [x0 - 0.01, 0.0, low],
        [x1 + 0.01, d + 0.02, zw],
        0.004,
    );
    // Above: shelves for the bottles, up to `h`.
    let up = 0.26;
    let levels = ((h - zw - 0.35) / 0.42).floor().clamp(1.0, 4.0) as usize;
    let first = zw + 0.32;
    let gap = ((h - 0.15) - first) / levels as f64;
    let shelves: Vec<f64> = (0..levels).map(|i| first + gap * i as f64).collect();
    let tall = gap - 0.06;
    let mut seed = 0u32;
    let mut stock = |b: &mut Builder, xa: f64, xe: f64, y: f64, z: f64| {
        seed += 1;
        bottles(b, xa, xe, y, z, tall, seed);
    };
    match ix {
        // Nordic: an oak back and floating oak shelves.
        0 => {
            b.rbox("frame", [x0, 0.0, zw], [x1, 0.02, h], 0.0);
            for z in &shelves {
                b.rbox(
                    "frame",
                    [x0 + 0.05, 0.02, z - 0.03],
                    [x1 - 0.05, up, *z],
                    0.004,
                );
                stock(b, x0 + 0.05, x1 - 0.05, up * 0.55, *z);
            }
        }
        // Mediterranean: whitewashed niches with round heads, a band of
        // tiles over the worktop, wooden shelves in the niches.
        1 => {
            b.rbox("masonry", [x0, 0.0, zw], [x1, 0.03, h], 0.0);
            let k = (w / 0.9).round().max(1.0) as usize;
            let pier = 0.1;
            let nw = (w - pier * (k + 1) as f64) / k as f64;
            let zs = h - 0.15 - nw / 2.0 - 0.08;
            for i in 0..=k {
                let x = x0 + (pier + nw) * i as f64;
                b.rbox(
                    "masonry",
                    [x, 0.03, zw + 0.25],
                    [x + pier, up + 0.04, h],
                    0.01,
                );
            }
            b.rbox(
                "masonry",
                [x0, 0.03, zw + 0.2],
                [x1, up + 0.04, zw + 0.25],
                0.006,
            );
            for i in 0..k {
                let xa = x0 + pier + (pier + nw) * i as f64;
                arch_head(b, "masonry", xa, xa + nw, zs, h, 0.03, up + 0.04);
                for z in shelves.iter().filter(|z| **z < zs - 0.1) {
                    b.rbox("frame", [xa, 0.03, z - 0.035], [xa + nw, up, *z], 0.006);
                    stock(b, xa, xa + nw, up * 0.55, *z);
                }
            }
            let tile = 0.1;
            let cols = (w / tile).floor() as usize;
            for c in 0..cols {
                for r in 0..2 {
                    let xa = x0 + (w - tile * cols as f64) / 2.0 + tile * c as f64;
                    let za = zw + tile * r as f64;
                    let role = if (c + r) % 2 == 0 {
                        "accent"
                    } else {
                        "masonry"
                    };
                    b.rbox(
                        role,
                        [xa + 0.002, 0.03, za + 0.002],
                        [xa + tile - 0.002, 0.036, za + tile - 0.002],
                        0.0,
                    );
                }
            }
        }
        // Fifties: a mirror behind teak shelves on brass posts.
        2 => {
            b.rbox("frame", [x0, 0.0, zw], [x1, 0.015, h], 0.0);
            b.rbox(
                "mirror",
                [x0 + 0.06, 0.015, zw + 0.04],
                [x1 - 0.06, 0.02, h - 0.06],
                0.0,
            );
            for z in &shelves {
                b.rbox(
                    "frame",
                    [x0 + 0.02, 0.02, z - 0.025],
                    [x1 - 0.02, up, *z],
                    0.008,
                );
                stock(b, x0 + 0.06, x1 - 0.06, up * 0.55, *z);
            }
            for x in [x0 + 0.05, x1 - 0.05] {
                b.cyl("metal", [x, up - 0.03, zw], [x, up - 0.03, h - 0.06], 0.009);
            }
            b.rbox("frame", [x0, 0.0, h - 0.06], [x1, up, h], 0.008);
        }
        // Japandi: a grid of oak cubbies, a few bottles and bowls.
        3 => {
            b.rbox("dark", [x0, 0.0, zw], [x1, 0.015, h], 0.0);
            let cols = (w / 0.6).round().max(1.0) as usize;
            for i in 0..=cols {
                let x = x0 + (w - 0.025) * i as f64 / cols as f64;
                b.rbox("frame", [x, 0.015, zw + 0.2], [x + 0.025, up, h], 0.0);
            }
            let mut zs = vec![zw + 0.2];
            zs.extend(shelves.iter().copied());
            zs.push(h - 0.025);
            for z in &zs {
                b.rbox("frame", [x0, 0.015, *z], [x1, up, z + 0.025], 0.0);
            }
            for (k, z) in zs.iter().take(zs.len() - 1).enumerate() {
                for i in 0..cols {
                    if (i + k) % 2 == 1 {
                        continue;
                    }
                    let xa = x0 + 0.025 + (w - 0.025) * i as f64 / cols as f64;
                    let xe = x0 + (w - 0.025) * (i + 1) as f64 / cols as f64;
                    let room = zs[k + 1] - z - 0.04;
                    seed += 1;
                    bottles(b, xa, xe, up * 0.55, z + 0.025, room, seed);
                }
            }
        }
        // Contemporary: a lit panel behind glass shelves.
        4 => {
            b.rbox("dark", [x0, 0.0, zw], [x1, 0.02, h], 0.0);
            b.rbox(
                "light",
                [x0 + 0.04, 0.02, zw + 0.06],
                [x1 - 0.04, 0.026, h - 0.06],
                0.0,
            );
            for z in &shelves {
                b.rbox(
                    "glass",
                    [x0 + 0.04, 0.03, z - 0.012],
                    [x1 - 0.04, up, *z],
                    0.002,
                );
                stock(b, x0 + 0.04, x1 - 0.04, up * 0.55, *z);
            }
        }
        // Rustic: thick shelves on iron brackets against lime, two small
        // barrels on the worktop.
        5 => {
            b.rbox("masonry", [x0, 0.0, zw], [x1, 0.02, h], 0.0);
            for z in &shelves {
                b.rbox(
                    "frame",
                    [x0 + 0.04, 0.02, z - 0.05],
                    [x1 - 0.04, up + 0.02, *z],
                    0.008,
                );
                let k = ((w / 0.9).ceil() as usize + 1).max(2);
                for i in 0..k {
                    let x = x0 + 0.15 + (w - 0.3) * i as f64 / (k - 1) as f64;
                    b.rbox(
                        "metal",
                        [x - 0.006, 0.02, z - 0.06],
                        [x + 0.006, up, z - 0.05],
                        0.0,
                    );
                    b.rbox(
                        "metal",
                        [x - 0.006, 0.02, z - 0.25],
                        [x + 0.006, 0.03, z - 0.05],
                        0.0,
                    );
                    bar(
                        b,
                        "metal",
                        [x, 0.03, z - 0.22],
                        [x, up - 0.04, z - 0.055],
                        0.01,
                        [1.0, 0.0, 0.0],
                    );
                }
                stock(b, x0 + 0.06, x1 - 0.06, up * 0.55, *z);
            }
            for x in [x0 + 0.25, x0 + 0.62] {
                let start = b.mark();
                barrel(b, [x, d * 0.45], 0.15, 0.3);
                b.transform_from(start, |q| [q[0], q[1], q[2] + zw]);
                b.cyl(
                    "metal",
                    [x, d * 0.45 + 0.15, zw + 0.09],
                    [x, d * 0.45 + 0.21, zw + 0.09],
                    0.012,
                );
            }
        }
        // Industrial: shelves of planks on posts of black pipe.
        6 => {
            let k = ((w / 1.0).ceil() as usize + 1).max(2);
            for i in 0..k {
                let x = x0 + 0.06 + (w - 0.12) * i as f64 / (k - 1) as f64;
                for y in [0.05, up - 0.04] {
                    b.cyl("dark", [x, y, zw], [x, y, h], 0.016);
                    b.cyl("dark", [x, y, zw], [x, y, zw + 0.01], 0.035);
                }
            }
            for z in &shelves {
                b.rbox(
                    "frame",
                    [x0 + 0.02, 0.01, z - 0.04],
                    [x1 - 0.02, up + 0.02, *z],
                    0.004,
                );
                stock(b, x0 + 0.1, x1 - 0.1, up * 0.5, *z);
            }
            b.rbox(
                "frame",
                [x0 + 0.02, 0.01, h - 0.04],
                [x1 - 0.02, up + 0.02, h],
                0.004,
            );
        }
        // Classic: a framed mirror between pilasters, wooden shelves, a
        // cornice.
        _ => {
            b.rbox("frame", [x0, 0.0, zw], [x1, 0.02, h], 0.0);
            b.rbox(
                "mirror",
                [x0 + 0.12, 0.02, zw + 0.06],
                [x1 - 0.12, 0.024, h - 0.2],
                0.0,
            );
            for (lo, hi) in [
                ([x0 + 0.1, 0.02, zw + 0.04], [x1 - 0.1, 0.035, zw + 0.06]),
                ([x0 + 0.1, 0.02, h - 0.2], [x1 - 0.1, 0.035, h - 0.18]),
            ] {
                b.rbox("metal", lo, hi, 0.004);
            }
            for x in [x0, x1 - 0.1] {
                b.rbox(
                    "frame",
                    [x, 0.02, zw],
                    [x + 0.1, up + 0.02, h - 0.12],
                    0.004,
                );
                b.rbox(
                    "frame",
                    [x - 0.01, 0.02, h - 0.2],
                    [x + 0.11, up + 0.04, h - 0.12],
                    0.004,
                );
            }
            for z in shelves.iter().filter(|z| **z < h - 0.45) {
                b.rbox(
                    "frame",
                    [x0 + 0.1, 0.02, z - 0.03],
                    [x1 - 0.1, up, *z],
                    0.006,
                );
                stock(b, x0 + 0.12, x1 - 0.12, up * 0.55, *z);
            }
            for (k, (dz, o)) in [(0.0, 0.05), (0.04, 0.035), (0.07, 0.015)]
                .into_iter()
                .enumerate()
            {
                let t = if k == 0 { 0.05 } else { 0.03 };
                b.rbox(
                    "frame",
                    [x0 - o, 0.0, h - dz - t],
                    [x1 + o, up + 0.02 + o, h - dz],
                    0.006,
                );
            }
        }
    }
}

// -------------------------------------------------------------- café table

/// Café tables of the collections.
pub fn bar_table(b: &mut Builder, p: &Params) {
    let ix = look(p);
    let w = p.get("width", 0.6).clamp(0.45, 1.2);
    let h = p.get("height", 0.75).clamp(0.7, 1.15);
    let c = [0.0, w / 2.0];
    let r = w / 2.0;
    let tall = h > 0.9;
    let square = |b: &mut Builder, role: &'static str, z: f64, t: f64, round: f64| {
        b.rbox(
            role,
            [c[0] - r, c[1] - r, z],
            [c[0] + r, c[1] + r, z + t],
            round,
        );
    };
    match ix {
        // Nordic: a round oak top on three splayed legs, a ring for the
        // feet when it is high.
        0 => {
            let t = 0.03;
            disc(b, "top", c, r, h - t, t);
            for k in 0..3 {
                let a = PI / 2.0 + TAU * k as f64 / 3.0;
                let (ca, sa) = (a.cos(), a.sin());
                tapered_leg(
                    b,
                    "frame",
                    [c[0] + ca * r * 0.55, c[1] + sa * r * 0.55],
                    h - t,
                    [ca, sa],
                    0.06 + h * 0.04,
                    0.02,
                );
            }
            if tall {
                let z = 0.3;
                let k = 0.06 + h * 0.04;
                let rr = r * 0.55 + k * (h - t - z) / (h - t);
                tube(b, "dark", &ring(c, rr, z, 36), true, 0.01, 8);
            }
        }
        // Mediterranean: tiles in an iron frame on an iron column with four
        // curled feet.
        1 => {
            let t = 0.025;
            square(b, "masonry", h - t, t - 0.004, 0.0);
            let k = ((2.0 * r - 0.04) / 0.1).floor().max(1.0) as usize;
            let tile = (2.0 * r - 0.04) / k as f64;
            for i in 0..k {
                for j in 0..k {
                    let (xa, ya) = (
                        c[0] - r + 0.02 + tile * i as f64,
                        c[1] - r + 0.02 + tile * j as f64,
                    );
                    let role = if (i + j) % 2 == 0 {
                        "accent"
                    } else {
                        "masonry"
                    };
                    b.rbox(
                        role,
                        [xa + 0.002, ya + 0.002, h - 0.006],
                        [xa + tile - 0.002, ya + tile - 0.002, h],
                        0.0,
                    );
                }
            }
            for (lo, hi) in [
                (
                    [c[0] - r, c[1] - r, h - t],
                    [c[0] + r, c[1] - r + 0.02, h + 0.002],
                ),
                (
                    [c[0] - r, c[1] + r - 0.02, h - t],
                    [c[0] + r, c[1] + r, h + 0.002],
                ),
                (
                    [c[0] - r, c[1] - r + 0.02, h - t],
                    [c[0] - r + 0.02, c[1] + r - 0.02, h + 0.002],
                ),
                (
                    [c[0] + r - 0.02, c[1] - r + 0.02, h - t],
                    [c[0] + r, c[1] + r - 0.02, h + 0.002],
                ),
            ] {
                b.rbox("metal", lo, hi, 0.002);
            }
            b.cyl("metal", [c[0], c[1], 0.12], [c[0], c[1], h - t], 0.022);
            for k in 0..4 {
                let a = FRAC_PI_4 + PI / 2.0 * k as f64;
                let (ca, sa) = (a.cos(), a.sin());
                let at = |s: f64, z: f64| [c[0] + ca * s, c[1] + sa * s, z];
                let reach = 0.24_f64.max(r * 0.75);
                let path = curve(
                    &[
                        at(0.0, 0.2),
                        at(0.03, 0.12),
                        at(reach * 0.6, 0.05),
                        at(reach, 0.025),
                        at(reach + 0.03, 0.05),
                        at(reach - 0.005, 0.07),
                    ],
                    5,
                );
                sweep(
                    b,
                    "metal",
                    &path,
                    |t| 0.011 - 0.004 * t,
                    8,
                    Cap::Round(1.0),
                    Cap::Round(1.0),
                );
                b.ellipsoid("metal", at(reach, 0.012), [0.016, 0.016, 0.012]);
            }
            tube(b, "metal", &ring(c, 0.03, 0.18, 16), true, 0.008, 6);
        }
        // Fifties: a round laminate top with a chrome edge on a chrome
        // column and a domed base.
        2 => {
            let t = 0.03;
            disc(b, "accent", c, r - 0.006, h - t, t);
            tube(
                b,
                "metal",
                &ring(c, r - 0.004, h - t / 2.0, 48),
                true,
                t * 0.5,
                8,
            );
            b.cyl("metal", [c[0], c[1], 0.04], [c[0], c[1], h - t], 0.03);
            let rb = 0.21_f64.max(r * 0.7);
            turned(
                b,
                "metal",
                [c[0], c[1], 0.0],
                &[
                    (0.0, 0.0),
                    (rb, 0.0),
                    (rb, 0.008),
                    (rb * 0.6, 0.035),
                    (0.05, 0.05),
                    (0.0, 0.05),
                ],
                36,
                0.0,
            );
        }
        // Japandi: a square oak top on two crossed slabs.
        3 => {
            let t = 0.035;
            square(b, "top", h - t, t, 0.006);
            let s = r * 0.62;
            b.rbox(
                "frame",
                [c[0] - s, c[1] - 0.02, 0.0],
                [c[0] + s, c[1] + 0.02, h - t],
                0.003,
            );
            b.rbox(
                "frame",
                [c[0] - 0.02, c[1] - s, 0.0],
                [c[0] + 0.02, c[1] - 0.02, h - t],
                0.003,
            );
            b.rbox(
                "frame",
                [c[0] - 0.02, c[1] + 0.02, 0.0],
                [c[0] + 0.02, c[1] + s, h - t],
                0.003,
            );
        }
        // Contemporary: a thin round top on a steel column and a flat disc.
        4 => {
            let t = 0.02;
            disc(b, "top", c, r, h - t, t);
            b.cyl("dark", [c[0], c[1], 0.012], [c[0], c[1], h - t], 0.03);
            b.cyl(
                "dark",
                [c[0], c[1], h - t - 0.012],
                [c[0], c[1], h - t],
                0.12,
            );
            disc(b, "dark", c, 0.22_f64.max(r * 0.65), 0.0, 0.014);
        }
        // Rustic: a barrel with a round plank top.
        5 => {
            let t = 0.04;
            barrel(b, c, (r * 0.75).min(0.3), h - t);
            disc(b, "top", c, r, h - t, t);
        }
        // Industrial: a square wooden top on a cast X base.
        6 => {
            let t = 0.04;
            square(b, "top", h - t, t, 0.004);
            b.rbox(
                "dark",
                [c[0] - r * 0.6, c[1] - r * 0.6, h - t - 0.03],
                [c[0] + r * 0.6, c[1] + r * 0.6, h - t],
                0.003,
            );
            b.cyl(
                "dark",
                [c[0], c[1], 0.06],
                [c[0], c[1], h - t - 0.03],
                0.035,
            );
            let reach = 0.28_f64.max(r * 0.8);
            for k in 0..4 {
                let a = FRAC_PI_4 + PI / 2.0 * k as f64;
                let (ca, sa) = (a.cos(), a.sin());
                bar(
                    b,
                    "dark",
                    [c[0], c[1], 0.06],
                    [c[0] + ca * reach, c[1] + sa * reach, 0.03],
                    0.045,
                    [0.0, 0.0, 1.0],
                );
                let f = [c[0] + ca * reach, c[1] + sa * reach];
                b.cyl("dark", [f[0], f[1], 0.0], [f[0], f[1], 0.02], 0.03);
            }
        }
        // Classic: a round marble top with a brass band on an iron tripod
        // of scrolls.
        _ => {
            let t = 0.03;
            disc(b, "top", c, r - 0.005, h - t, t);
            tube(
                b,
                "metal",
                &ring(c, r - 0.003, h - t / 2.0, 48),
                true,
                t * 0.45,
                8,
            );
            let neck = h - t - 0.05;
            turned(
                b,
                "dark",
                [c[0], c[1], 0.22],
                &[
                    (0.0, 0.0),
                    (0.04, 0.0),
                    (0.05, 0.03),
                    (0.022, 0.07),
                    (0.02, neck - 0.32),
                    (0.035, neck - 0.27),
                    (0.025, neck - 0.24),
                    (0.06, neck - 0.22),
                    (0.0, neck - 0.22),
                ],
                24,
                0.0,
            );
            for k in 0..3 {
                let a = PI / 2.0 + TAU * k as f64 / 3.0;
                let (ca, sa) = (a.cos(), a.sin());
                let at = |s: f64, z: f64| [c[0] + ca * s, c[1] + sa * s, z];
                let reach = 0.26_f64.max(r * 0.7);
                let path = curve(
                    &[
                        at(0.03, 0.26),
                        at(0.1, 0.2),
                        at(reach * 0.7, 0.07),
                        at(reach, 0.03),
                        at(reach + 0.035, 0.06),
                        at(reach + 0.01, 0.08),
                    ],
                    5,
                );
                sweep(
                    b,
                    "dark",
                    &path,
                    |t| 0.016 - 0.006 * t,
                    8,
                    Cap::Round(1.0),
                    Cap::Round(1.0),
                );
                b.ellipsoid("dark", at(reach, 0.015), [0.02, 0.02, 0.015]);
            }
        }
    }
}

// --------------------------------------------------------------- café chair

/// Café chairs of the collections.
pub fn cafe_chair(b: &mut Builder, p: &Params) {
    let ix = look(p);
    let w = p.get("width", 0.44).clamp(0.38, 0.6);
    let d = p.get("depth", 0.48).clamp(0.4, 0.6);
    let seat = p.get("seat", 0.46).clamp(0.42, 0.5);
    let h = p.get("height", 0.82).clamp(seat + 0.25, 1.1);
    let (hx, x) = (w / 2.0, w / 2.0 - 0.03);
    match ix {
        // Nordic: oak, a padded round seat, a bent back on four splayed
        // legs.
        0 => {
            let top = seat - 0.03;
            for (px, py, dx, dy) in [
                (-x, 0.06, -0.5, -0.6),
                (x, 0.06, 0.5, -0.6),
                (x, d - 0.05, 0.5, 0.5),
                (-x, d - 0.05, -0.5, 0.5),
            ] {
                tapered_leg(b, "frame", [px, py], top, [dx, dy], 0.035, 0.016);
            }
            b.rbox("frame", [-hx, 0.02, top], [hx, d, seat - 0.008], 0.012);
            upholstered(
                [-hx + 0.015, 0.035, seat - 0.012],
                [hx - 0.015, d - 0.015, seat + 0.03],
                0.02,
                31,
            )
            .add(b, "cushion");
            let cy = d * 0.6;
            band(
                b,
                "frame",
                [0.0, cy],
                cy - 0.02,
                -PI * 0.8,
                -PI * 0.2,
                h - 0.17,
                h,
                0.016,
                14,
            );
            for s in [-1.0, 1.0] {
                let a: f64 = -PI / 2.0 + s * PI * 0.22;
                let q = [(cy - 0.012) * a.cos(), cy + (cy - 0.012) * a.sin()];
                bar(
                    b,
                    "frame",
                    [q[0], q[1] + 0.02, seat - 0.01],
                    [q[0], q[1], h - 0.15],
                    0.024,
                    [1.0, 0.0, 0.0],
                );
            }
        }
        // Mediterranean: a painted tavern chair, a rush seat, two slats.
        1 => {
            let s = 0.03;
            for (px, py) in [(-x, d - 0.04), (x, d - 0.04)] {
                b.rbox(
                    "frame",
                    [px - s / 2.0, py - s / 2.0, 0.0],
                    [px + s / 2.0, py + s / 2.0, seat],
                    0.004,
                );
            }
            for px in [-x, x] {
                b.rbox(
                    "frame",
                    [px - s / 2.0, 0.02, 0.0],
                    [px + s / 2.0, 0.02 + s, h],
                    0.004,
                );
                b.ellipsoid(
                    "frame",
                    [px, 0.02 + s / 2.0, h + 0.012],
                    [0.02, 0.02, 0.018],
                );
            }
            for z in [0.12, 0.26] {
                b.rbox("frame", [-x, d - 0.05, z], [x, d - 0.03, z + 0.022], 0.003);
                b.rbox("frame", [-x, 0.03, z], [x, 0.05, z + 0.022], 0.003);
                for px in [-x, x] {
                    b.rbox(
                        "frame",
                        [px - 0.01, 0.04, z + 0.06],
                        [px + 0.01, d - 0.04, z + 0.08],
                        0.003,
                    );
                }
            }
            b.rbox(
                "frame",
                [-x - s / 2.0, 0.02, seat - 0.05],
                [x + s / 2.0, d - 0.02, seat - 0.02],
                0.004,
            );
            b.pillow(
                "weave",
                [-x, 0.04, seat - 0.035],
                [x, d - 0.03, seat + 0.01],
                0.012,
                0.01,
            );
            for z in [h - 0.2, h - 0.08] {
                b.rbox("frame", [-x, 0.02, z - 0.04], [x, 0.04, z], 0.006);
            }
        }
        // Fifties: chromed tube, a seat and a back of coloured plywood.
        2 => {
            for (px, py, dx, dy) in [
                (-x, 0.08, -0.4, -0.6),
                (x, 0.08, 0.4, -0.6),
                (x, d - 0.07, 0.4, 0.6),
                (-x, d - 0.07, -0.4, 0.6),
            ] {
                let k = 0.05;
                rod(
                    b,
                    "metal",
                    [px + dx * k, py + dy * k, 0.012],
                    [px, py, seat - 0.03],
                    0.011,
                );
                b.cyl(
                    "dark",
                    [px + dx * k, py + dy * k, 0.0],
                    [px + dx * k, py + dy * k, 0.012],
                    0.013,
                );
            }
            for py in [0.08, d - 0.07] {
                rod(
                    b,
                    "metal",
                    [-x, py, seat - 0.035],
                    [x, py, seat - 0.035],
                    0.009,
                );
            }
            b.rbox("accent", [-hx, 0.03, seat - 0.025], [hx, d, seat], 0.012);
            let cy = d * 0.7;
            band(
                b,
                "accent",
                [0.0, cy],
                cy - 0.02,
                -PI * 0.78,
                -PI * 0.22,
                h - 0.2,
                h - 0.02,
                0.012,
                14,
            );
            for s in [-1.0, 1.0] {
                rod(
                    b,
                    "metal",
                    [s * x * 0.7, 0.07, seat - 0.03],
                    [s * x * 0.7, 0.03, h - 0.12],
                    0.009,
                );
            }
        }
        // Japandi: oak, a woven cord seat, a low curved back rail.
        3 => {
            let s = 0.028;
            let rail = seat + 0.24;
            for (px, py, top) in [
                (-x, 0.05, rail),
                (x, 0.05, rail),
                (x, d - 0.04, seat),
                (-x, d - 0.04, seat),
            ] {
                bar(b, "frame", [px, py, 0.0], [px, py, top], s, [1.0, 0.0, 0.0]);
            }
            for (a, e) in [
                ([-x, 0.05, 0.18], [x, 0.05, 0.18]),
                ([-x, d - 0.04, 0.12], [x, d - 0.04, 0.12]),
            ] {
                bar(b, "frame", a, e, 0.02, [0.0, 0.0, 1.0]);
            }
            for px in [-x, x] {
                bar(
                    b,
                    "frame",
                    [px, 0.05, 0.15],
                    [px, d - 0.04, 0.15],
                    0.02,
                    [0.0, 0.0, 1.0],
                );
            }
            b.rbox(
                "frame",
                [-x - s / 2.0, 0.035, seat - 0.04],
                [x + s / 2.0, d - 0.025, seat - 0.01],
                0.004,
            );
            b.pillow(
                "weave",
                [-x + 0.01, 0.05, seat - 0.03],
                [x - 0.01, d - 0.04, seat + 0.005],
                0.008,
                0.006,
            );
            let cy = d * 0.75;
            band(
                b,
                "frame",
                [0.0, cy],
                cy - 0.05,
                -PI * 0.82,
                -PI * 0.18,
                rail - 0.05,
                rail + 0.02,
                0.025,
                14,
            );
        }
        // Contemporary: a stacking shell of coloured polypropylene on thin
        // steel legs.
        4 => {
            for (px, py, dx, dy) in [
                (-x, 0.07, -0.5, -0.5),
                (x, 0.07, 0.5, -0.5),
                (x, d - 0.06, 0.5, 0.5),
                (-x, d - 0.06, -0.5, 0.5),
            ] {
                let k = 0.045;
                rod(
                    b,
                    "dark",
                    [px + dx * k, py + dy * k, 0.01],
                    [px * 0.85, py, seat - 0.03],
                    0.008,
                );
                b.cyl(
                    "dark",
                    [px + dx * k, py + dy * k, 0.0],
                    [px + dx * k, py + dy * k, 0.012],
                    0.01,
                );
            }
            b.rbox(
                "dark",
                [-x * 0.85, 0.07, seat - 0.035],
                [x * 0.85, d - 0.06, seat - 0.025],
                0.0,
            );
            b.rbox("accent", [-hx, 0.04, seat - 0.025], [hx, d, seat], 0.014);
            let cy = d * 0.62;
            band(
                b,
                "accent",
                [0.0, cy],
                cy - 0.035,
                -PI * 0.88,
                -PI * 0.12,
                seat - 0.01,
                h,
                0.012,
                18,
            );
        }
        // Rustic: a thick seat, four splayed turned legs with stretchers and
        // a back of spindles under a crest rail.
        5 => {
            let t = 0.045;
            let top = seat - t;
            let legs = [
                (-x, 0.07, -0.5, -0.6),
                (x, 0.07, 0.5, -0.6),
                (x, d - 0.06, 0.5, 0.6),
                (-x, d - 0.06, -0.5, 0.6),
            ];
            for (px, py, dx, dy) in legs {
                tapered_leg(b, "frame", [px, py], top, [dx, dy], 0.045, 0.02);
            }
            let k = 0.045 * (top - 0.16) / top;
            for (i, j) in [(0, 3), (1, 2), (0, 1)] {
                let (a, e) = (legs[i], legs[j]);
                let at = |l: (f64, f64, f64, f64)| {
                    [l.0 + l.2 * (0.045 - k), l.1 + l.3 * (0.045 - k), 0.16]
                };
                rod(b, "frame", at(a), at(e), 0.011);
            }
            b.rbox("frame", [-hx, 0.02, top], [hx, d, seat], 0.012);
            let cy = d * 0.6;
            let rr = cy - 0.03;
            band(
                b,
                "frame",
                [0.0, cy],
                rr,
                -PI * 0.8,
                -PI * 0.2,
                h - 0.08,
                h,
                0.03,
                14,
            );
            for i in 0..5 {
                let a = -PI * 0.75 + PI * 0.5 * i as f64 / 4.0;
                let q = [rr * a.cos(), cy + (rr + 0.012) * a.sin()];
                rod(
                    b,
                    "frame",
                    [q[0], q[1] + 0.02, seat],
                    [q[0], q[1], h - 0.07],
                    0.009,
                );
            }
        }
        // Industrial: steel sheet seat and back, square legs, rivets.
        6 => {
            let s = 0.022;
            for (px, py, dx, dy) in [
                (-x, 0.06, -0.5, -0.5),
                (x, 0.06, 0.5, -0.5),
                (x, d - 0.05, 0.5, 0.5),
                (-x, d - 0.05, -0.5, 0.5),
            ] {
                let k = 0.05;
                bar(
                    b,
                    "dark",
                    [px + dx * k, py + dy * k, 0.012],
                    [px, py, seat - 0.01],
                    s,
                    [1.0, 0.0, 0.0],
                );
                let f = [px + dx * k, py + dy * k];
                b.rbox(
                    "dark",
                    [f[0] - 0.014, f[1] - 0.014, 0.0],
                    [f[0] + 0.014, f[1] + 0.014, 0.014],
                    0.003,
                );
            }
            b.rbox("dark", [-hx, 0.03, seat - 0.035], [hx, d, seat], 0.006);
            for (px, py) in [(-x, 0.06), (x, 0.06), (x, d - 0.05), (-x, d - 0.05)] {
                b.ellipsoid("metal", [px, py, seat + 0.001], [0.006, 0.006, 0.003]);
            }
            for px in [-x, x] {
                bar(
                    b,
                    "dark",
                    [px, 0.06, seat],
                    [px, 0.025, h - 0.03],
                    s * 0.8,
                    [1.0, 0.0, 0.0],
                );
            }
            let start = b.mark();
            b.rbox(
                "dark",
                [-hx + 0.02, -0.006, h - 0.25],
                [hx - 0.02, 0.006, h],
                0.004,
            );
            b.transform_from(start, |q| {
                let lean = 0.12 * (q[2] - (h - 0.25)) / 0.25;
                [q[0], q[1] + 0.035 - lean * 0.15, q[2]]
            });
            for k in 0..3 {
                let z = h - 0.22 + 0.08 * k as f64;
                b.rbox(
                    "dark",
                    [-hx * 0.5, 0.045 - 0.12 * 0.15 * (z - (h - 0.25)) / 0.25, z],
                    [hx * 0.5, 0.046, z + 0.012],
                    0.002,
                );
            }
        }
        // Classic: a Parisian bistro chair, its frame of rattan with rings
        // and a woven seat and back.
        _ => {
            let r = 0.014;
            let top = seat - 0.03;
            let legs = [
                (-x, 0.06, -0.4, -0.5),
                (x, 0.06, 0.4, -0.5),
                (x, d - 0.06, 0.4, 0.5),
                (-x, d - 0.06, -0.4, 0.5),
            ];
            for (i, (px, py, dx, dy)) in legs.into_iter().enumerate() {
                let k = 0.04;
                let up = if i < 2 { h - 0.05 } else { top };
                rod(
                    b,
                    "frame",
                    [px + dx * k, py + dy * k, 0.012],
                    [px, py, up.min(top)],
                    r,
                );
                b.cyl(
                    "dark",
                    [px + dx * k, py + dy * k, 0.0],
                    [px + dx * k, py + dy * k, 0.014],
                    r + 0.002,
                );
                for z in [0.2, top - 0.05] {
                    let f = (top - z) / top;
                    let q = [px + dx * k * f, py + dy * k * f, z];
                    tube(
                        b,
                        "frame",
                        &ring([q[0], q[1]], r + 0.003, z, 10),
                        true,
                        0.004,
                        5,
                    );
                }
            }
            // The back: a hoop of rattan from the back legs.
            let cy = d * 0.55;
            let hoop: Vec<P3> = (0..=14)
                .map(|k| {
                    let t = k as f64 / 14.0;
                    let a = PI * t;
                    [
                        -x * a.cos(),
                        0.06 - 0.03 * a.sin(),
                        top + (h - top - 0.02) * (0.25 + 0.75 * a.sin().powf(0.6)),
                    ]
                })
                .collect();
            let mut back = vec![[-x, 0.06, top]];
            back.extend(hoop);
            back.push([x, 0.06, top]);
            sweep(
                b,
                "frame",
                &curve(&back, 3),
                |_| r,
                10,
                Cap::Flat,
                Cap::Flat,
            );
            band(
                b,
                "weave",
                [0.0, cy],
                cy - 0.03,
                -PI * 0.78,
                -PI * 0.22,
                top + 0.12,
                h - 0.08,
                0.01,
                12,
            );
            tube(
                b,
                "frame",
                &super::shape::rounded_rect(
                    [0.0, d / 2.0 + 0.02],
                    hx,
                    d / 2.0 - 0.02,
                    0.06,
                    seat - 0.015,
                    4,
                ),
                true,
                0.012,
                8,
            );
            b.pillow(
                "weave",
                [-hx + 0.02, 0.05, seat - 0.035],
                [hx - 0.02, d - 0.01, seat - 0.005],
                0.02,
                0.006,
            );
            for y in [0.06, d - 0.06] {
                rod(b, "frame", [-x, y, top - 0.04], [x, y, top - 0.04], 0.01);
            }
        }
    }
}

// --------------------------------------------------------------- banquette

/// Banquettes of the collections.
pub fn banquette(b: &mut Builder, p: &Params) {
    let ix = look(p);
    let w = p.get("width", 1.8).clamp(0.9, 6.0);
    let d = p.get("depth", 0.6).clamp(0.45, 0.8);
    let seat = p.get("seat", 0.46).clamp(0.4, 0.5);
    let h = p.get("height", 1.0).clamp(0.75, 1.4);
    let (x0, x1) = (-w / 2.0, w / 2.0);
    let places = (w / 0.6).round().max(1.0) as usize;
    let seat_pad = |b: &mut Builder, y0: f64, z0: f64, t: f64, welt: f64, seed: u32| {
        let mut s = upholstered([x0 + 0.02, y0, z0], [x1 - 0.02, d, z0 + t], 0.03, seed);
        s.sag = 0.008;
        s.welt = welt;
        s.add(b, "cushion");
    };
    match ix {
        // Nordic: an oak box, a seat pad and loose cushions against an oak
        // panel.
        0 => {
            b.rbox(
                "dark",
                [x0 + 0.03, 0.03, 0.0],
                [x1 - 0.03, d - 0.06, 0.08],
                0.0,
            );
            b.rbox("frame", [x0, 0.0, 0.08], [x1, d, seat - 0.08], 0.006);
            b.rbox("frame", [x0, 0.0, seat - 0.08], [x1, 0.03, h], 0.006);
            seat_pad(b, 0.03, seat - 0.08, 0.08, 0.0, 41);
            back_cushions(
                b,
                places,
                x0 + 0.03,
                x1 - 0.03,
                0.03,
                seat,
                h - 0.06,
                0.13,
                8.0,
                0.0,
            );
        }
        // Mediterranean: built of lime, a thin pad and scattered pillows.
        1 => {
            b.rbox("masonry", [x0, 0.0, 0.0], [x1, d, seat - 0.07], 0.03);
            b.rbox("masonry", [x0, 0.0, seat - 0.07], [x1, 0.12, h], 0.04);
            seat_pad(b, 0.12, seat - 0.07, 0.07, 0.004, 42);
            let n = places.min(4);
            for i in 0..n {
                let x = x0 + w * (i as f64 + 0.5) / n as f64;
                let lean = 1.08_f64;
                let yaw: f64 = [-0.12, 0.08, -0.05, 0.1][i % 4];
                let (sy, cy) = yaw.sin_cos();
                let (sl, cl) = lean.sin_cos();
                let role = if i % 2 == 0 { "pillow" } else { "accent" };
                knife_pillow(
                    b,
                    role,
                    [x, 0.22, seat + 0.2],
                    [cy, sy, 0.0],
                    [-sy * cl, cy * cl, sl],
                    0.42,
                    0.42,
                    0.2,
                    60 + i as u32,
                );
            }
        }
        // Fifties: channels down a high back, a seat with piping, a teak
        // plinth on compass legs.
        2 => {
            let leg = 0.14;
            b.rbox("frame", [x0, 0.02, leg], [x1, d, seat - 0.12], 0.008);
            for (x, y, dx) in [
                (x0 + 0.08, 0.08, -0.4),
                (x1 - 0.08, 0.08, 0.4),
                (x1 - 0.08, d - 0.08, 0.4),
                (x0 + 0.08, d - 0.08, -0.4),
            ] {
                compass_leg(
                    b,
                    [x, y],
                    leg + 0.01,
                    [dx, if y < 0.2 { -0.4 } else { 0.4 }],
                    0.04,
                );
            }
            seat_pad(b, 0.14, seat - 0.12, 0.12, 0.005, 43);
            b.rbox("frame", [x0, 0.0, leg], [x1, 0.04, h], 0.006);
            let ch = (w / 0.14).round().max(2.0) as usize;
            let cw = (w - 0.04) / ch as f64;
            for i in 0..ch {
                let xa = x0 + 0.02 + cw * i as f64;
                let mut s = Soft::new(
                    [xa + 0.002, 0.04, seat],
                    [xa + cw - 0.002, 0.14, h - 0.02],
                    0.04,
                );
                s.crown = 0.02;
                s.wobble = 0.001;
                s.seed = 70 + i as u32;
                s.step = 0.06;
                s.add(b, "cushion");
            }
        }
        // Japandi: a slatted oak bench on blocks, a thin pad, a slatted back
        // with a long cushion.
        3 => {
            for x in [x0 + 0.05, x1 - 0.15] {
                b.rbox(
                    "frame",
                    [x, 0.04, 0.0],
                    [x + 0.1, d - 0.04, seat - 0.1],
                    0.004,
                );
            }
            let k = ((d - 0.02) / 0.075).floor() as usize;
            for i in 0..k {
                let y = 0.01 + (d - 0.02) * i as f64 / k as f64;
                b.rbox(
                    "frame",
                    [x0, y + 0.004, seat - 0.1],
                    [x1, y + (d - 0.02) / k as f64 - 0.004, seat - 0.07],
                    0.003,
                );
            }
            seat_pad(b, 0.06, seat - 0.07, 0.05, 0.0, 44);
            let n = ((w - 0.04) / 0.06).floor() as usize;
            for i in 0..n {
                let x = x0 + 0.02 + (w - 0.04) * i as f64 / n as f64;
                b.rbox(
                    "frame",
                    [x + 0.006, 0.0, seat - 0.07],
                    [x + (w - 0.04) / n as f64 - 0.006, 0.025, h],
                    0.003,
                );
            }
            let mut s = upholstered(
                [x0 + 0.06, 0.025, seat + 0.1],
                [x1 - 0.06, 0.12, seat + 0.32],
                0.05,
                45,
            );
            s.crown = 0.02;
            s.add(b, "cushion");
        }
        // Contemporary: an upholstered block on a dark recess, a back in
        // three channels across.
        4 => {
            b.rbox(
                "dark",
                [x0 + 0.04, 0.04, 0.0],
                [x1 - 0.04, d - 0.04, 0.06],
                0.0,
            );
            let mut base = upholstered([x0, 0.0, 0.06], [x1, d, seat - 0.1], 0.02, 46);
            base.crown = 0.003;
            base.add(b, "cushion");
            seat_pad(b, 0.16, seat - 0.1, 0.1, 0.0, 47);
            let rows = 3;
            let zh = (h - seat) / rows as f64;
            for i in 0..rows {
                let z = seat + zh * i as f64;
                let mut s = Soft::new([x0, 0.0, z], [x1, 0.16 - 0.02 * i as f64, z + zh], 0.05);
                s.crown = 0.025;
                s.wobble = 0.001;
                s.seed = 80 + i;
                s.add(b, "cushion");
            }
            b.rbox("dark", [x0, 0.0, seat - 0.1], [x1, 0.02, h], 0.0);
        }
        // Rustic: a settle of planks with its arms, cushions and pillows.
        5 => {
            let t = 0.04;
            b.rbox("frame", [x0, 0.0, 0.0], [x1, d, seat - 0.06], 0.004);
            front(
                b,
                Panel::Planks,
                "frame",
                x0 + t,
                x1 - t,
                0.04,
                seat - 0.1,
                d,
            );
            b.rbox(
                "frame",
                [x0 - 0.01, 0.0, seat - 0.1],
                [x1 + 0.01, d + 0.02, seat - 0.06],
                0.006,
            );
            let n = ((w - 2.0 * t) / 0.18).round().max(1.0) as usize;
            let pw = (w - 2.0 * t) / n as f64;
            for i in 0..n {
                let xa = x0 + t + pw * i as f64;
                b.rbox(
                    "frame",
                    [xa + 0.002, 0.0, seat - 0.06],
                    [xa + pw - 0.002, 0.03, h],
                    0.004,
                );
            }
            b.rbox("frame", [x0, 0.0, h - 0.04], [x1, 0.06, h + 0.02], 0.008);
            for x in [x0, x1 - t] {
                b.rbox("frame", [x, 0.0, 0.0], [x + t, d, seat + 0.22], 0.006);
                b.rbox(
                    "frame",
                    [x - 0.01, 0.0, seat + 0.2],
                    [x + t + 0.01, d + 0.03, seat + 0.24],
                    0.008,
                );
            }
            let mut s = upholstered(
                [x0 + t + 0.01, 0.04, seat - 0.06],
                [x1 - t - 0.01, d, seat + 0.02],
                0.03,
                48,
            );
            s.sag = 0.01;
            s.add(b, "cushion");
            back_cushions(
                b,
                places.min(3),
                x0 + t + 0.02,
                x1 - t - 0.02,
                0.03,
                seat + 0.02,
                seat + 0.42,
                0.14,
                10.0,
                0.004,
            );
        }
        // Industrial: a frame of black steel, a leather seat and back.
        6 => {
            let s = 0.035;
            for x in [x0, x1 - s] {
                for y in [0.0, d - s] {
                    b.rbox("dark", [x, y, 0.0], [x + s, y + s, seat - 0.1], 0.0);
                }
                b.rbox("dark", [x, 0.0, 0.0], [x + s, s, h], 0.0);
                b.rbox("dark", [x, 0.0, seat - 0.13], [x + s, d, seat - 0.1], 0.0);
            }
            for y in [0.0, d - s] {
                b.rbox("dark", [x0, y, seat - 0.13], [x1, y + s, seat - 0.1], 0.0);
                b.rbox("dark", [x0, y, 0.12], [x1, y + s, 0.12 + s], 0.0);
            }
            b.rbox("dark", [x0, 0.0, h - s], [x1, s, h], 0.0);
            b.rbox(
                "frame",
                [x0 + s, 0.0, seat - 0.1],
                [x1 - s, d, seat - 0.08],
                0.0,
            );
            seat_pad(b, 0.04, seat - 0.08, 0.08, 0.004, 49);
            let mut back = Soft::new([x0 + s, s, seat + 0.04], [x1 - s, s + 0.08, h - 0.06], 0.03);
            back.crown = 0.012;
            back.tufts = 0.22;
            back.tuft_depth = 0.012;
            back.tuft_faces = [false, true];
            back.wobble = 0.001;
            back.seed = 50;
            back.add(b, "cushion");
        }
        // Classic: a buttoned velvet back, a seat with piping, a moulded
        // wooden base on turned feet.
        _ => {
            let feet = 0.1;
            for (x, y) in [
                (x0 + 0.06, 0.06),
                (x1 - 0.06, 0.06),
                (x1 - 0.06, d - 0.06),
                (x0 + 0.06, d - 0.06),
            ] {
                toupie(b, [x, y], feet);
            }
            b.rbox("frame", [x0, 0.0, feet], [x1, d, seat - 0.14], 0.006);
            b.rbox(
                "frame",
                [x0 - 0.01, 0.0, seat - 0.16],
                [x1 + 0.01, d + 0.015, seat - 0.13],
                0.008,
            );
            b.rbox(
                "frame",
                [x0 - 0.01, 0.0, feet],
                [x1 + 0.01, d + 0.01, feet + 0.03],
                0.008,
            );
            seat_pad(b, 0.16, seat - 0.13, 0.13, 0.006, 51);
            let mut back = Soft::new([x0, 0.0, seat - 0.13], [x1, 0.16, h], 0.05);
            back.crown = 0.015;
            back.tufts = 0.16;
            back.tuft_depth = 0.02;
            back.tuft_faces = [false, true];
            back.welt = 0.004;
            back.wobble = 0.0012;
            back.seed = 52;
            back.add(b, "cushion");
            b.rbox(
                "frame",
                [x0 - 0.005, 0.0, h],
                [x1 + 0.005, 0.17, h + 0.035],
                0.012,
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fine::shape::signed_volume;

    #[test]
    fn the_bars_of_the_eight_collections() {
        type Make = fn(&mut Builder, &Params);
        let pieces: [(&str, Make); 5] = [
            ("barra", bar_counter),
            ("contrabarra", back_bar),
            ("mesa", bar_table),
            ("silla", cafe_chair),
            ("banco", banquette),
        ];
        for c in 0..8 {
            for (name, make) in pieces {
                for tall in [false, true] {
                    let mut p = Params::default();
                    p.set("style", (1 + c) as f64);
                    if tall {
                        p.set("height", if name == "mesa" { 1.05 } else { 1.2 });
                    }
                    let mut b = Builder::default();
                    make(&mut b, &p);
                    let tris = b.triangle_count();
                    assert!((100..40_000).contains(&tris), "{c} {name}: {tris}");
                    for (role, t) in &b.parts {
                        let v = signed_volume(&b.positions, t);
                        assert!(v > -1e-7, "{c} {name} {role}: {v}");
                    }
                    let low = b.positions.iter().map(|q| q[2]).fold(f32::MAX, f32::min);
                    assert!(low > -0.001, "{c} {name}: {low}");
                }
            }
        }
    }
}
