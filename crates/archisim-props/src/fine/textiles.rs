//! The workshop's rugs and curtains, one for each collection (paquete 1c).
//!
//! Rugs: `width` × `depth`, centred on `x = 0` from `y = 0` to `depth`,
//! lying on the floor. Roles: `mat` the ground, `mat2` and `accent` the
//! pattern, `fringe`, `weave` jute.
//!
//! Curtains: `width` the window they cover, `height` up to the rod, `open`
//! from 0 (drawn) to 1 (gathered at the sides); the wall at `y = 0`.
//! Roles: `mat` the curtain, `sheer` a voile, `metal` or `frame` the rod,
//! `accent` the tie-backs and trimmings.

use super::shape::{sweep, turned, Cap};
use crate::build::{Builder, P3};
use crate::Params;
use std::f64::consts::{PI, TAU};

/// A flat piece of the pattern lying on the rug, `z0` to `z1`.
fn inlay(b: &mut Builder, role: &'static str, poly: &[[f64; 2]], z0: f64, z1: f64) {
    // The slab wants its polygon counter-clockwise.
    let n = poly.len();
    let area: f64 = (0..n)
        .map(|i| {
            let (a, c) = (poly[i], poly[(i + 1) % n]);
            a[0] * c[1] - c[0] * a[1]
        })
        .sum();
    if area < 0.0 {
        let rev: Vec<[f64; 2]> = poly.iter().rev().copied().collect();
        b.slab(role, &rev, z0, z1);
    } else {
        b.slab(role, poly, z0, z1);
    }
}

/// A rectangle as a polygon.
fn rect(x0: f64, y0: f64, x1: f64, y1: f64) -> Vec<[f64; 2]> {
    vec![[x0, y0], [x1, y0], [x1, y1], [x0, y1]]
}

/// An ellipse as a polygon of `n` sides.
fn ellipse(c: [f64; 2], rx: f64, ry: f64, n: usize) -> Vec<[f64; 2]> {
    (0..n)
        .map(|k| {
            let a = TAU * k as f64 / n as f64;
            [c[0] + rx * a.cos(), c[1] + ry * a.sin()]
        })
        .collect()
}

/// A diamond.
fn diamond(c: [f64; 2], rx: f64, ry: f64) -> Vec<[f64; 2]> {
    vec![
        [c[0], c[1] - ry],
        [c[0] + rx, c[1]],
        [c[0], c[1] + ry],
        [c[0] - rx, c[1]],
    ]
}

/// A band round the rug `inset` from its edge and `w` wide, as four
/// strips.
#[allow(clippy::too_many_arguments)]
fn border(
    b: &mut Builder,
    role: &'static str,
    w: f64,
    d: f64,
    inset: f64,
    band: f64,
    z0: f64,
    z1: f64,
) {
    let (x0, x1, y0, y1) = (-w / 2.0 + inset, w / 2.0 - inset, inset, d - inset);
    for poly in [
        rect(x0, y0, x1, y0 + band),
        rect(x0, y1 - band, x1, y1),
        rect(x0, y0 + band, x0 + band, y1 - band),
        rect(x1 - band, y0 + band, x1, y1 - band),
    ] {
        inlay(b, role, &poly, z0, z1);
    }
}

/// Knotted fringes along both short ends (at `x = ±w/2`), `len` long.
fn fringes(b: &mut Builder, w: f64, d: f64, len: f64) {
    for side in [-1.0f64, 1.0] {
        let x = side * w / 2.0;
        // The loose threads as one soft flat strip, a little ragged.
        b.rbox(
            "fringe",
            [x.min(x + side * len), 0.01, 0.0],
            [x.max(x + side * len), d - 0.01, 0.003],
            0.0,
        );
        // The knotted band where they start.
        b.rbox(
            "fringe",
            [x_min(side, w), 0.0, 0.0],
            [x_max(side, w), d, 0.011],
            0.003,
        );
    }
}

fn x_min(side: f64, w: f64) -> f64 {
    if side < 0.0 {
        -w / 2.0 - 0.012
    } else {
        w / 2.0 - 0.004
    }
}

fn x_max(side: f64, w: f64) -> f64 {
    if side < 0.0 {
        -w / 2.0 + 0.004
    } else {
        w / 2.0 + 0.012
    }
}

/// A rug of a collection.
pub fn rug(b: &mut Builder, p: &Params) {
    let w = p.get("width", 2.0).clamp(0.6, 5.0);
    let d = p.get("depth", 1.4).clamp(0.4, 4.0);
    let t = 0.012;
    let (top, up) = (t, t + 0.0006);
    let c = [0.0, d / 2.0];
    match p.style() {
        // Mediterranean: a round rug of braided jute, coil on coil: a
        // disc whose top rises and falls with the coils, two of them dyed.
        2 => {
            let r = w.min(d) / 2.0;
            let coil = 0.024;
            let n = ((r / coil).floor() as usize).max(2);
            let mut prof = vec![(0.0, 0.0), (r, 0.0), (r, t * 0.6)];
            for k in 0..n {
                let rr = r - coil * k as f64;
                prof.push((rr - coil * 0.5, t * 0.6 + 0.003));
                prof.push((rr - coil, t * 0.6));
            }
            prof.push((0.0, t * 0.6));
            prof.retain(|q| q.0 >= 0.0);
            b.lathe("weave", [c[0], c[1], 0.0], &prof, 1.0, 1.0);
            for k in [3usize, 5] {
                let rr = r - coil * (k as f64 + 0.5);
                if rr < coil {
                    continue;
                }
                let sides = ((rr * 60.0) as usize).clamp(16, 72);
                let ring: Vec<P3> = (0..sides)
                    .map(|i| {
                        let a = TAU * i as f64 / sides as f64;
                        [c[0] + rr * a.cos(), c[1] + rr * a.sin(), t * 0.6 + 0.001]
                    })
                    .collect();
                super::soft::tube(b, "mat2", &ring, true, coil * 0.42, 6);
            }
        }
        // Fifties: discs of two colours drifting across the ground in a
        // loose diagonal, large and small, none touching.
        3 => {
            b.rbox("mat", [-w / 2.0, 0.0, 0.0], [w / 2.0, d, t], 0.006);
            let m = w.min(d);
            // (x, y) as fractions of the rug, radius as a fraction of its
            // shorter side, colour.
            let discs: [(f64, f64, f64, &str); 11] = [
                (0.2, 0.28, 0.13, "mat2"),
                (0.36, 0.62, 0.09, "accent"),
                (0.5, 0.36, 0.06, "accent"),
                (0.62, 0.7, 0.12, "mat2"),
                (0.8, 0.4, 0.08, "accent"),
                (0.1, 0.7, 0.05, "accent"),
                (0.9, 0.78, 0.04, "mat2"),
                (0.46, 0.14, 0.035, "mat2"),
                (0.74, 0.16, 0.05, "mat2"),
                (0.28, 0.88, 0.035, "mat2"),
                (0.55, 0.9, 0.03, "accent"),
            ];
            for (fx, fy, fr, role) in discs {
                let at = [-w / 2.0 + w * fx, d * fy];
                let r = m * fr;
                inlay(b, role, &ellipse(at, r, r, 40), top - 0.001, up);
                // A ring of the ground colour inside the largest ones.
                if fr > 0.1 {
                    inlay(
                        b,
                        "mat",
                        &ellipse(at, r * 0.45, r * 0.45, 32),
                        top - 0.001,
                        up + 0.0006,
                    );
                }
            }
        }
        // Japandi: woven straw in a fine grid with a dark cloth binding.
        4 => {
            b.rbox("weave", [-w / 2.0, 0.0, 0.0], [w / 2.0, d, t * 0.8], 0.002);
            let band = 0.035;
            border(b, "mat2", w, d, 0.0, band, t * 0.8 - 0.002, t * 0.8 + 0.002);
            let n = ((d / 0.06) as usize).max(2);
            for k in 1..n {
                let y = band + (d - 2.0 * band) * k as f64 / n as f64;
                inlay(
                    b,
                    "mat",
                    &rect(-w / 2.0 + band, y - 0.0015, w / 2.0 - band, y + 0.0015),
                    t * 0.8 - 0.001,
                    t * 0.8 + 0.0008,
                );
            }
        }
        // Contemporary: a pebble, two levels of pile, a smaller pebble
        // set in it in the other colour.
        5 => {
            let pebble = |c: [f64; 2], rx: f64, ry: f64, seed: f64| -> Vec<[f64; 2]> {
                (0..48)
                    .map(|k| {
                        let a = TAU * k as f64 / 48.0;
                        let wob = 1.0
                            + 0.05 * (2.0 * a + seed).sin()
                            + 0.03 * (3.0 * a + 2.0 * seed).cos();
                        [c[0] + rx * wob * a.cos(), c[1] + ry * wob * a.sin()]
                    })
                    .collect()
            };
            inlay(
                b,
                "mat",
                &pebble(c, w / 2.0 * 0.94, d / 2.0 * 0.94, 0.4),
                0.0,
                t,
            );
            inlay(
                b,
                "mat2",
                &pebble([c[0] + w * 0.12, c[1] - d * 0.06], w * 0.22, d * 0.24, 1.7),
                0.0,
                t + 0.006,
            );
        }
        // Rustic: a rag rug of stripes in three colours, fringed.
        6 => {
            b.rbox("mat", [-w / 2.0, 0.0, 0.0], [w / 2.0, d, t], 0.004);
            let mut x = -w / 2.0 + 0.04;
            let roles = ["mat2", "accent", "mat2", "mat", "accent"];
            let mut k = 0;
            while x < w / 2.0 - 0.06 {
                let band = [0.03, 0.06, 0.02, 0.09, 0.04][k % 5];
                let role = roles[k % roles.len()];
                if role != "mat" {
                    inlay(
                        b,
                        role,
                        &rect(x, 0.0, (x + band).min(w / 2.0 - 0.04), d),
                        top - 0.002,
                        up,
                    );
                }
                x += band + 0.02;
                k += 1;
            }
            fringes(b, w, d, 0.06);
        }
        // Industrial: a worn kilim, its rows of diamonds faded.
        7 => {
            b.rbox("mat", [-w / 2.0, 0.0, 0.0], [w / 2.0, d, t * 0.7], 0.002);
            border(
                b,
                "mat2",
                w,
                d,
                0.06,
                0.05,
                t * 0.7 - 0.001,
                t * 0.7 + 0.001,
            );
            let rows = 3;
            let s = (d - 0.32) / rows as f64;
            let n = (((w - 0.32) / s).floor() as usize).max(1);
            for i in 0..n {
                for j in 0..rows {
                    let at = [
                        -w / 2.0 + 0.16 + (w - 0.32) * (i as f64 + 0.5) / n as f64,
                        0.16 + s * (j as f64 + 0.5),
                    ];
                    let role = if (i + j) % 2 == 0 { "accent" } else { "mat2" };
                    inlay(
                        b,
                        role,
                        &diamond(at, s * 0.42, s * 0.42),
                        t * 0.7 - 0.001,
                        t * 0.7 + 0.0012,
                    );
                    inlay(
                        b,
                        "mat",
                        &diamond(at, s * 0.2, s * 0.2),
                        t * 0.7 - 0.001,
                        t * 0.7 + 0.0016,
                    );
                }
            }
            fringes(b, w, d, 0.05);
        }
        // Classic: an oriental carpet — borders, a medallion, corners,
        // fringed.
        8 => {
            b.rbox("mat", [-w / 2.0, 0.0, 0.0], [w / 2.0, d, t], 0.003);
            let m = w.min(d);
            border(b, "accent", w, d, 0.0, m * 0.1, top - 0.001, up);
            border(b, "mat2", w, d, m * 0.115, m * 0.025, top - 0.001, up);
            border(b, "mat2", w, d, m * 0.03, m * 0.012, top, up + 0.0004);
            let (cx, cy) = (c[0], c[1]);
            inlay(
                b,
                "accent",
                &diamond([cx, cy], w * 0.2, d * 0.22),
                top - 0.001,
                up,
            );
            inlay(
                b,
                "mat2",
                &ellipse([cx, cy], w * 0.11, d * 0.13, 32),
                top - 0.001,
                up + 0.0004,
            );
            inlay(
                b,
                "mat",
                &ellipse([cx, cy], w * 0.05, d * 0.06, 24),
                top - 0.001,
                up + 0.0008,
            );
            for (sx, sy) in [(-1.0, -1.0), (1.0, -1.0), (1.0, 1.0), (-1.0, 1.0)] {
                let k = m * 0.14;
                let (ax, ay) = (sx * (w / 2.0 - k), cy + sy * (d / 2.0 - k));
                let tri = [[ax, ay], [ax - sx * w * 0.14, ay], [ax, ay - sy * d * 0.18]];
                inlay(b, "mat2", &tri, top - 0.001, up);
            }
            fringes(b, w, d, 0.07);
        }
        // Nordic: thick wool, a band in another colour near each end.
        _ => {
            b.rbox("mat", [-w / 2.0, 0.0, 0.0], [w / 2.0, d, t * 1.4], 0.008);
            for side in [-1.0, 1.0] {
                for (o, bw, role) in [(0.12, 0.05, "mat2"), (0.2, 0.015, "accent")] {
                    let x = side * (w / 2.0 - o);
                    inlay(
                        b,
                        role,
                        &rect(x - bw / 2.0, 0.02, x + bw / 2.0, d - 0.02),
                        t * 1.4 - 0.002,
                        t * 1.4 + 0.0015,
                    );
                }
            }
        }
    }
}

/// How a curtain hangs from its rod.
#[derive(Clone, Copy, PartialEq)]
enum Heading {
    /// Soft even waves.
    Wave,
    /// Pinched in threes at the top, the folds opening below.
    Pinch,
    /// Gathered on rings, small irregular folds.
    Gathered,
}

/// One curtain from `xa` to `xb` (its width when drawn; it is hung
/// `width` × `gather` of fabric), from `top` down to `bottom`, in front of
/// the wall at `y`. `tie`: a tie-back at height `z` pulling it to its outer
/// side `xs`.
#[allow(clippy::too_many_arguments)]
fn drape(
    b: &mut Builder,
    role: &'static str,
    xa: f64,
    xb: f64,
    top: f64,
    bottom: f64,
    y: f64,
    heading: Heading,
    tie: Option<(f64, f64)>,
    seed: f64,
) {
    b.soft.insert(role);
    let w = (xb - xa).abs().max(0.05);
    let fold = match heading {
        Heading::Wave => 0.12,
        Heading::Pinch => 0.14,
        Heading::Gathered => 0.08,
    };
    let folds = (w / fold).round().max(1.0);
    let amp = match heading {
        Heading::Wave => 0.045,
        Heading::Pinch => 0.05,
        Heading::Gathered => 0.03,
    };
    let nx = (folds as usize * 8).max(4);
    let nz = 18usize;
    let h = top - bottom;
    let point = |i: usize, j: usize, back: bool| -> P3 {
        let u = i as f64 / nx as f64;
        let v = j as f64 / nz as f64; // 0 at the top
        let z = top - h * v;
        let mut x = xa + (xb - xa) * u;
        // The folds: deeper towards the hem; a pinch pleat's are drawn
        // together at the top.
        let phase = TAU * folds * u;
        let mut a = amp * (0.75 + 0.45 * v) * (1.0 + 0.12 * (phase * 0.37 + seed).sin());
        if heading == Heading::Pinch {
            a *= 0.35 + 0.65 * (v / 0.12).min(1.0);
        }
        let mut dy = a * 0.5 * (1.0 - phase.cos());
        if let Some((zt, xs)) = tie {
            // Pulled to the side at the tie, falling free below it.
            let k = if z >= zt {
                let s = ((top - z) / (top - zt)).clamp(0.0, 1.0);
                1.0 - 0.7 * s * s
            } else {
                let s = ((zt - z) / (zt - bottom).max(0.01)).clamp(0.0, 1.0);
                0.3 + 0.35 * s
            };
            x = xs + (x - xs) * k;
            dy *= (1.0 / k).sqrt().min(2.2);
        }
        let t = if back { 0.0 } else { 0.004 };
        [x, y + dy + t, z]
    };
    let mut front = Vec::new();
    let mut rear = Vec::new();
    for j in 0..=nz {
        for i in 0..=nx {
            front.push(b.point(point(i, j, false)));
            rear.push(b.point(point(i, j, true)));
        }
    }
    let at = |i: usize, j: usize| j * (nx + 1) + i;
    // Which way is "out" depends on whether the curtain runs left to right.
    let flip = xb < xa;
    for j in 0..nz {
        for i in 0..nx {
            let (a, bb, c, d) = (at(i, j), at(i + 1, j), at(i + 1, j + 1), at(i, j + 1));
            if flip {
                b.quad(role, front[d], front[c], front[bb], front[a]);
                b.quad(role, rear[a], rear[bb], rear[c], rear[d]);
            } else {
                b.quad(role, front[a], front[bb], front[c], front[d]);
                b.quad(role, rear[d], rear[c], rear[bb], rear[a]);
            }
        }
    }
    // The edges: the hem and the sides.
    for i in 0..nx {
        let (a, e) = (at(i, nz), at(i + 1, nz));
        if flip {
            b.quad(role, front[a], front[e], rear[e], rear[a]);
        } else {
            b.quad(role, front[e], front[a], rear[a], rear[e]);
        }
    }
    for (i, sign) in [(0usize, 1.0), (nx, -1.0)] {
        for j in 0..nz {
            let (a, e) = (at(i, j), at(i, j + 1));
            let s = if flip { -sign } else { sign };
            if s > 0.0 {
                b.quad(role, front[a], front[e], rear[e], rear[a]);
            } else {
                b.quad(role, front[e], front[a], rear[a], rear[e]);
            }
        }
    }
}

/// A pair of curtains drawn from the middle, open by `open`, each hung
/// from `top` to `bottom` at `y`.
#[allow(clippy::too_many_arguments)]
fn pair(
    b: &mut Builder,
    role: &'static str,
    w: f64,
    open: f64,
    top: f64,
    bottom: f64,
    y: f64,
    heading: Heading,
    tie: Option<f64>,
    over: f64,
) {
    let half = w / 2.0 + over;
    let span = half * (1.0 - 0.72 * open);
    for (k, side) in [-1.0f64, 1.0].into_iter().enumerate() {
        let outer = side * half;
        let inner = outer - side * span;
        let t = tie.map(|z| (z, outer));
        drape(
            b,
            role,
            outer,
            inner,
            top,
            bottom,
            y,
            heading,
            t,
            k as f64 * 1.7,
        );
    }
}

/// A rod from `x0` to `x1` at `(y, z)`, `r` thick, with turned finials and
/// brackets back to the wall.
#[allow(clippy::too_many_arguments)]
fn rod(b: &mut Builder, role: &'static str, x0: f64, x1: f64, y: f64, z: f64, r: f64, finial: u8) {
    b.cyl(role, [x0, y, z], [x1, y, z], r);
    for (x, s) in [(x0, -1.0f64), (x1, 1.0)] {
        // The finial, turned along the rod.
        let prof: Vec<(f64, f64)> = match finial {
            // A ball.
            0 => vec![
                (0.0, 0.0),
                (r * 1.1, 0.0),
                (r * 2.0, r * 1.2),
                (r * 1.6, r * 3.2),
                (0.0, r * 3.6),
            ],
            // An acorn with a collar.
            1 => vec![
                (0.0, 0.0),
                (r * 1.4, 0.0),
                (r * 1.4, r * 0.8),
                (r * 1.0, r * 1.2),
                (r * 2.0, r * 2.6),
                (r * 1.4, r * 4.6),
                (r * 0.4, r * 5.6),
                (0.0, r * 5.8),
            ],
            // A plain end cap.
            _ => vec![
                (0.0, 0.0),
                (r * 1.3, 0.0),
                (r * 1.3, r * 1.2),
                (0.0, r * 1.2),
            ],
        };
        let start = b.mark();
        turned(b, role, [0.0; 3], &prof, 14, 0.0);
        // Laid along the rod at either end (a rotation, never a mirror:
        // it stays closed outwards).
        b.transform_from(start, |q| [x + s * q[2], y - s * q[1], z + q[0]]);
        // The bracket a little in from the end.
        let xb = x - s * 0.1;
        b.cyl(role, [xb, 0.0, z], [xb, y, z], r * 0.6);
        b.cyl(role, [xb, 0.0, z], [xb, 0.006, z], r * 2.2);
    }
}

/// Rings on a rod at `(y, z)` along `x0`…`x1`, `n` of them.
#[allow(clippy::too_many_arguments)]
fn rings(b: &mut Builder, role: &'static str, x0: f64, x1: f64, y: f64, z: f64, rr: f64, n: usize) {
    for k in 0..n {
        let x = x0 + (x1 - x0) * k as f64 / (n.max(2) - 1) as f64;
        let ring: Vec<P3> = (0..12)
            .map(|i| {
                let a = TAU * i as f64 / 12.0;
                [x, y + rr * a.cos(), z + rr * a.sin()]
            })
            .collect();
        super::soft::tube(b, role, &ring, true, rr * 0.18, 5);
    }
}

/// A rope or cord tie-back round a gathered curtain at `(x, z)`.
fn tieback(b: &mut Builder, role: &'static str, x: f64, y: f64, z: f64, tassel: bool) {
    let loop_: Vec<P3> = (0..=16)
        .map(|i| {
            let a = PI * i as f64 / 16.0;
            [
                x - 0.06 * a.cos(),
                y + 0.06 * a.sin() - 0.005,
                z - 0.01 * a.sin(),
            ]
        })
        .collect();
    sweep(
        b,
        role,
        &loop_,
        |_| 0.007,
        6,
        Cap::Round(1.0),
        Cap::Round(1.0),
    );
    b.cyl("metal", [x - 0.06, 0.0, z], [x - 0.06, y, z], 0.006);
    if tassel {
        let c = [x + 0.03, y + 0.06, z - 0.02];
        turned(
            b,
            role,
            [c[0], c[1], c[2] - 0.13],
            &[
                (0.0, 0.0),
                (0.03, 0.0),
                (0.022, 0.07),
                (0.012, 0.09),
                (0.018, 0.11),
                (0.006, 0.13),
                (0.0, 0.13),
            ],
            14,
            0.0,
        );
    }
}

/// A curtain of a collection.
pub fn curtain(b: &mut Builder, p: &Params) {
    let w = p.get("width", 1.6).clamp(0.4, 6.0);
    let h = p.get("height", 2.5).clamp(0.6, 4.0);
    let open = p.get("open", 0.6).clamp(0.0, 1.0);
    let y = 0.11;
    let tie = if open > 0.3 { Some(h * 0.42) } else { None };
    match p.style() {
        // Mediterranean: light linen gathered on rings, a wooden pole, rope
        // tie-backs.
        2 => {
            rod(b, "frame", -w / 2.0 - 0.2, w / 2.0 + 0.2, y, h, 0.016, 0);
            pair(
                b,
                "mat",
                w,
                open,
                h - 0.03,
                0.005,
                y - 0.012,
                Heading::Gathered,
                tie,
                0.14,
            );
            let n = ((w / 0.1) as usize).max(6);
            rings(b, "frame", -w / 2.0 - 0.12, w / 2.0 + 0.12, y, h, 0.024, n);
            if let Some(z) = tie {
                for s in [-1.0, 1.0] {
                    tieback(b, "accent", s * (w / 2.0 + 0.1), y, z, false);
                }
            }
        }
        // Fifties: pinch pleats on a slim brass rod.
        3 => {
            rod(b, "metal", -w / 2.0 - 0.18, w / 2.0 + 0.18, y, h, 0.01, 1);
            pair(
                b,
                "mat",
                w,
                open,
                h - 0.02,
                0.01,
                y - 0.01,
                Heading::Pinch,
                tie,
                0.12,
            );
            if let Some(z) = tie {
                for s in [-1.0, 1.0] {
                    tieback(b, "metal", s * (w / 2.0 + 0.09), y, z, false);
                }
            }
        }
        // Japandi: flat panels on a ceiling track, overlapping.
        4 => {
            let n = ((w / 0.6).ceil() as usize).max(2);
            let pw = w / n as f64 + 0.06;
            b.rbox(
                "frame",
                [-w / 2.0 - 0.08, 0.02, h - 0.03],
                [w / 2.0 + 0.08, 0.2, h],
                0.004,
            );
            for k in 0..n {
                let slide = open
                    * (w / 2.0)
                    * (if (k as f64) < n as f64 / 2.0 {
                        -1.0
                    } else {
                        1.0
                    })
                    * (1.0 - (k as f64 - (n as f64 - 1.0) / 2.0).abs() / n as f64);
                let x = -w / 2.0 + w * (k as f64 + 0.5) / n as f64 + slide;
                let yy = 0.05 + 0.04 * (k % 3) as f64;
                b.rbox(
                    "mat",
                    [x - pw / 2.0, yy, 0.02],
                    [x + pw / 2.0, yy + 0.004, h - 0.035],
                    0.0,
                );
                b.rbox(
                    "frame",
                    [x - pw / 2.0, yy - 0.004, 0.0],
                    [x + pw / 2.0, yy + 0.008, 0.035],
                    0.003,
                );
                b.rbox(
                    "frame",
                    [x - pw / 2.0, yy - 0.004, h - 0.07],
                    [x + pw / 2.0, yy + 0.008, h - 0.035],
                    0.003,
                );
            }
        }
        // Contemporary: soft waves from a ceiling track, a sheer behind.
        5 => {
            b.rbox(
                "metal",
                [-w / 2.0 - 0.15, 0.04, h - 0.02],
                [w / 2.0 + 0.15, 0.18, h],
                0.003,
            );
            pair(
                b,
                "sheer",
                w,
                0.0,
                h - 0.025,
                0.01,
                0.07,
                Heading::Wave,
                None,
                0.0,
            );
            pair(
                b,
                "mat",
                w,
                open,
                h - 0.025,
                0.01,
                0.13,
                Heading::Wave,
                None,
                0.12,
            );
        }
        // Rustic: heavy linen on an iron rod with ring clips, tied back.
        6 => {
            rod(b, "metal", -w / 2.0 - 0.22, w / 2.0 + 0.22, y, h, 0.012, 0);
            pair(
                b,
                "mat",
                w,
                open,
                h - 0.045,
                0.0,
                y - 0.014,
                Heading::Gathered,
                tie,
                0.15,
            );
            let n = ((w / 0.12) as usize).max(6);
            rings(b, "metal", -w / 2.0 - 0.14, w / 2.0 + 0.14, y, h, 0.02, n);
            if let Some(z) = tie {
                for s in [-1.0, 1.0] {
                    tieback(b, "accent", s * (w / 2.0 + 0.12), y, z, false);
                }
            }
        }
        // Industrial: a black iron pipe on elbow brackets, eyelet panels.
        7 => {
            let r = 0.014;
            b.cyl("metal", [-w / 2.0 - 0.18, y, h], [w / 2.0 + 0.18, y, h], r);
            for x in [-w / 2.0 - 0.18, w / 2.0 + 0.18] {
                b.ellipsoid("metal", [x, y, h], [r * 1.5, r * 1.5, r * 1.5]);
                b.cyl("metal", [x, 0.0, h], [x, y, h], r);
                b.cyl("metal", [x, 0.0, h], [x, 0.008, h], r * 3.0);
            }
            pair(
                b,
                "mat",
                w,
                open,
                h + 0.02,
                0.01,
                y - 0.004,
                Heading::Wave,
                None,
                0.14,
            );
            let n = ((w / 0.15) as usize).max(6);
            rings(b, "metal", -w / 2.0 - 0.12, w / 2.0 + 0.12, y, h, 0.022, n);
        }
        // Classic: velvet drapes under a swagged pelmet, tasselled
        // tie-backs, a gilded rod.
        8 => {
            rod(
                b,
                "metal",
                -w / 2.0 - 0.22,
                w / 2.0 + 0.22,
                y + 0.02,
                h,
                0.016,
                1,
            );
            let tie = Some(h * 0.4);
            pair(
                b,
                "mat",
                w,
                open.max(0.5),
                h - 0.05,
                -0.02,
                y - 0.004,
                Heading::Pinch,
                tie,
                0.16,
            );
            // The pelmet: swags hanging between the ends.
            let n = ((w / 0.6).round() as usize).max(2);
            let sw = (w + 0.36) / n as f64;
            for k in 0..n {
                let x0 = -w / 2.0 - 0.18 + sw * k as f64;
                let start = b.mark();
                drape(
                    b,
                    "mat",
                    x0,
                    x0 + sw,
                    h + 0.01,
                    h - 0.3,
                    y + 0.05,
                    Heading::Gathered,
                    None,
                    k as f64,
                );
                // Each swag dips in its middle.
                b.transform_from(start, |q| {
                    let u = ((q[0] - x0) / sw).clamp(0.0, 1.0);
                    let dip = 0.12 * (PI * u).sin() * ((h + 0.01 - q[2]) / 0.31).clamp(0.0, 1.0);
                    [q[0], q[1] + 0.02 * (PI * u).sin(), q[2] - dip]
                });
            }
            for s in [-1.0, 1.0] {
                drape(
                    b,
                    "mat",
                    s * (w / 2.0 + 0.2),
                    s * (w / 2.0 + 0.02),
                    h + 0.01,
                    h - 0.75,
                    y + 0.06,
                    Heading::Pinch,
                    None,
                    3.0,
                );
                if let Some(z) = tie {
                    tieback(b, "accent", s * (w / 2.0 + 0.12), y, z, true);
                }
            }
        }
        // Nordic: linen in soft waves on an oak pole with ball finials.
        _ => {
            rod(b, "frame", -w / 2.0 - 0.18, w / 2.0 + 0.18, y, h, 0.014, 0);
            pair(
                b,
                "mat",
                w,
                open,
                h - 0.015,
                0.008,
                y - 0.002,
                Heading::Wave,
                None,
                0.12,
            );
            let n = ((w / 0.12) as usize).max(6);
            rings(b, "frame", -w / 2.0 - 0.12, w / 2.0 + 0.12, y, h, 0.02, n);
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn the_collection_textiles_are_made() {
        for kind in ["fine_rug", "fine_curtain"] {
            for style in 1..=8 {
                for open in [0.0, 1.0] {
                    let mut p = crate::Params::default();
                    p.set("style", style as f64);
                    p.set("open", open);
                    let m = crate::make(kind, &p).unwrap();
                    let n = m.triangle_count();
                    println!("{kind} {style} {open}: {n}");
                    assert!((12..=60_000).contains(&n), "{kind} {style}: {n}");
                }
            }
        }
    }
}
