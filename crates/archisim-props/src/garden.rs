//! The garden and the terrace (colecciones, revisión de Antonio del
//! 23/10/2026): sun loungers, parasols, big pots with what grows in them
//! and pergolas. Axes as the furniture's: origin in the middle of the
//! back, `x` to the right, `y` forwards, `z` up; round pieces stand
//! centred on `(0, width / 2)`.

use crate::build::{norm, Builder, P3};
use crate::Params;

use std::f64::consts::{PI, TAU};

/// A leaning slat or back: from `a` to `b` (the middle of its two ends),
/// `w` wide across `x`, `t` thick.
fn lean(b: &mut Builder, role: &'static str, a: P3, e: P3, w: f64, t: f64) {
    let c = [
        (a[0] + e[0]) / 2.0,
        (a[1] + e[1]) / 2.0,
        (a[2] + e[2]) / 2.0,
    ];
    let along = [
        (e[0] - a[0]) / 2.0,
        (e[1] - a[1]) / 2.0,
        (e[2] - a[2]) / 2.0,
    ];
    let n = norm([0.0, -along[2], along[1]]);
    b.obox(
        role,
        c,
        [w / 2.0, 0.0, 0.0],
        along,
        [n[0] * t / 2.0, n[1] * t / 2.0, n[2] * t / 2.0],
    );
}

/// Sun loungers: 1 of teak slats with its back raised, 2 an aluminium
/// frame with a sling, 3 a daybed with a thick mattress and cushions. The
/// head is at the back (`y` 0); `cushion` lays a pad on 1 and 2.
pub fn lounger(b: &mut Builder, p: &Params) {
    let w = p.get("width", 0.7).clamp(0.5, 2.0);
    let l = p.get("depth", 2.0).clamp(1.6, 2.4);
    let x0 = -w / 2.0;
    let seat = 0.32;
    let back_len = 0.62;
    let back_top = [0.0, 0.12, seat + back_len * 0.75];
    let back_foot = [0.0, 0.12 + back_len * 0.66, seat + 0.02];
    match p.style() {
        2 => {
            // A tube frame on four feet, the sling over it.
            for x in [x0 + 0.02, -x0 - 0.02] {
                b.cyl("metal", [x, back_foot[1], seat], [x, l - 0.05, seat], 0.014);
                b.cyl(
                    "metal",
                    [x, back_foot[1], seat],
                    [x, back_top[1], back_top[2]],
                    0.014,
                );
                for y in [0.3, l - 0.1] {
                    b.cyl("metal", [x, y, 0.004], [x, y, seat], 0.014);
                }
            }
            for y in [0.3, l - 0.1] {
                b.cyl("metal", [x0 + 0.02, y, 0.1], [-x0 - 0.02, y, 0.1], 0.01);
            }
            b.rbox(
                "mat",
                [x0 + 0.03, back_foot[1], seat - 0.012],
                [-x0 - 0.03, l - 0.06, seat + 0.006],
                0.004,
            );
            lean(
                b,
                "mat",
                [0.0, back_foot[1], seat],
                [0.0, back_top[1], back_top[2]],
                w - 0.06,
                0.016,
            );
            if p.flag("pad", false) {
                pad(b, w - 0.06, back_foot, back_top, seat + 0.006, l - 0.06);
            }
        }
        3 => {
            // A low platform, a thick mattress and cushions at the back.
            let base = 0.28;
            b.rbox("frame", [x0, 0.0, 0.0], [-x0, l, 0.06], 0.01);
            b.rbox(
                "frame",
                [x0 + 0.03, 0.03, 0.06],
                [-x0 - 0.03, l - 0.03, base],
                0.01,
            );
            b.pillow(
                "mat",
                [x0 + 0.04, 0.04, base],
                [-x0 - 0.04, l - 0.04, base + 0.16],
                0.06,
                0.015,
            );
            let n = ((w / 0.5).round() as usize).max(2);
            let cw = (w - 0.1) / n as f64;
            for i in 0..n {
                let a = x0 + 0.05 + i as f64 * cw;
                lean(
                    b,
                    "cushion",
                    [a + cw / 2.0, 0.18, base + 0.16],
                    [a + cw / 2.0, 0.08, base + 0.6],
                    cw - 0.02,
                    0.16,
                );
            }
            let s = 0.2;
            b.pillow(
                "accent",
                [-s, 0.28, base + 0.16],
                [s, 0.42, base + 0.16 + s * 1.6],
                0.05,
                0.035,
            );
        }
        _ => {
            // Two side rails on legs, the slats across, the back raised.
            for x in [x0, -x0 - 0.04] {
                b.rbox("frame", [x, 0.08, seat - 0.08], [x + 0.04, l, seat], 0.006);
                for y in [0.1, l - 0.1] {
                    b.rbox(
                        "frame",
                        [x, y - 0.025, 0.0],
                        [x + 0.04, y + 0.025, seat - 0.08],
                        0.005,
                    );
                }
            }
            // Wheels at the head.
            for x in [x0 - 0.02, -x0 + 0.02] {
                b.cone(
                    "dark",
                    [x - 0.015, 0.1, 0.07],
                    [x + 0.015, 0.1, 0.07],
                    0.07,
                    0.07,
                    16,
                );
            }
            let n = ((l - back_foot[1] - 0.05) / 0.09).floor().max(4.0) as usize;
            for i in 0..n {
                let y = back_foot[1] + 0.03 + i as f64 * (l - back_foot[1] - 0.05) / n as f64;
                b.rbox(
                    "frame",
                    [x0 + 0.04, y, seat - 0.02],
                    [-x0 - 0.04, y + 0.065, seat],
                    0.004,
                );
            }
            let rows = 6;
            for i in 0..rows {
                let t0 = i as f64 / rows as f64;
                let t1 = (i as f64 + 0.8) / rows as f64;
                let at = |t: f64| {
                    [
                        0.0,
                        back_foot[1] + (back_top[1] - back_foot[1]) * t,
                        seat + (back_top[2] - seat) * t,
                    ]
                };
                lean(b, "frame", at(t0), at(t1), w - 0.1, 0.02);
            }
            if p.flag("pad", false) {
                pad(b, w - 0.08, back_foot, back_top, seat, l - 0.02);
            }
        }
    }
}

/// The pad of a lounger: one part on the seat, one on the back.
fn pad(b: &mut Builder, w: f64, back_foot: P3, back_top: P3, seat: f64, end: f64) {
    b.pillow(
        "cushion",
        [-w / 2.0, back_foot[1], seat + 0.01],
        [w / 2.0, end, seat + 0.07],
        0.03,
        0.012,
    );
    let n = norm([0.0, back_top[2] - seat, -(back_top[1] - back_foot[1])]);
    let off = |q: P3| [q[0], q[1] + n[1] * 0.045, q[2] + n[2] * 0.045];
    lean(
        b,
        "cushion",
        off([0.0, back_foot[1], seat]),
        off(back_top),
        w,
        0.06,
    );
}

/// Parasols: 1 round on a central pole, 2 square hanging from an arm
/// beside it (cantilever), 3 of straw with a fringe (as on the beach).
pub fn parasol(b: &mut Builder, p: &Params) {
    let s = p.get("width", 2.5).clamp(1.2, 4.0);
    let h = p.get("height", 2.4).clamp(1.8, 2.9);
    let r = s / 2.0;
    let c = [0.0, r];
    match p.style() {
        2 => {
            // The pole and its cross base beside the canopy, the arm over.
            let px = -r - 0.25;
            for (dx, dy) in [(0.45_f64, 0.0_f64), (-0.45, 0.0), (0.0, 0.45), (0.0, -0.45)] {
                b.rbox(
                    "dark",
                    [px.min(px + dx) - 0.04, c[1] + dy.min(0.0) - 0.04, 0.0],
                    [px.max(px + dx) + 0.04, c[1] + dy.max(0.0) + 0.04, 0.08],
                    0.01,
                );
            }
            b.cyl("metal", [px, c[1], 0.08], [px, c[1], h], 0.035);
            let steps = 8;
            let mut last = [px, c[1], h];
            for i in 1..=steps {
                let t = i as f64 / steps as f64;
                let q = [px + (r + 0.25) * t, c[1], h + 0.15 * (PI * t).sin()];
                b.cyl("metal", last, q, 0.025);
                last = q;
            }
            b.cyl("metal", [0.0, c[1], h], [0.0, c[1], h - 0.25], 0.02);
            canopy(b, [0.0, c[1], h - 0.45], r, 0.4, 4, PI / 4.0);
        }
        3 => {
            b.cyl("frame", [c[0], c[1], 0.0], [c[0], c[1], h - 0.1], 0.03);
            b.lathe(
                "mat",
                [c[0], c[1], h - 0.55],
                &[
                    (r, 0.0),
                    (r * 0.98, 0.04),
                    (r * 0.6, 0.32),
                    (0.06, 0.55),
                    (0.0, 0.56),
                ],
                1.0,
                1.0,
            );
            // The fringe of straw round the edge.
            let n = ((TAU * r / 0.06).round() as usize).max(24);
            for i in 0..n {
                let a = TAU * i as f64 / n as f64;
                let (ca, sa) = (a.cos(), a.sin());
                let q = [c[0] + ca * r * 0.99, c[1] + sa * r * 0.99, h - 0.53];
                b.cyl(
                    "mat",
                    q,
                    [q[0] + ca * 0.03, q[1] + sa * 0.03, h - 0.72],
                    0.012,
                );
            }
        }
        _ => {
            b.cone(
                "dark",
                [c[0], c[1], 0.0],
                [c[0], c[1], 0.06],
                0.25,
                0.23,
                24,
            );
            b.cyl("frame", [c[0], c[1], 0.06], [c[0], c[1], h + 0.08], 0.022);
            canopy(b, [c[0], c[1], h - 0.4], r, 0.4, 8, 0.0);
            b.ellipsoid("frame", [c[0], c[1], h + 0.09], [0.03, 0.03, 0.03]);
        }
    }
}

/// The cloth of a parasol: a pyramid of `n` sides from its edge at `c`
/// (radius `r`) up `rise` to the top, its ribs under it and a valance
/// hanging from the edge.
fn canopy(b: &mut Builder, c: P3, r: f64, rise: f64, n: usize, turn: f64) {
    let top = b.point([c[0], c[1], c[2] + rise]);
    let mid = b.point([c[0], c[1], c[2] + rise * 0.55]);
    let edge: Vec<P3> = (0..n)
        .map(|k| {
            let a = turn + TAU * k as f64 / n as f64;
            let rr = if n == 4 {
                r * std::f64::consts::SQRT_2
            } else {
                r
            };
            [c[0] + rr * a.cos(), c[1] + rr * a.sin(), c[2]]
        })
        .collect();
    let ids: Vec<u32> = edge.iter().map(|q| b.point(*q)).collect();
    for k in 0..n {
        let k1 = (k + 1) % n;
        b.tri("mat", ids[k], ids[k1], top);
        b.tri("mat", ids[k1], ids[k], mid);
        b.cyl("frame", edge[k], [c[0], c[1], c[2] + rise * 0.5], 0.008);
        // The valance.
        let (a, e) = (edge[k], edge[k1]);
        let (dx, dy) = (e[0] - a[0], e[1] - a[1]);
        let len = (dx * dx + dy * dy).sqrt();
        let mid = [(a[0] + e[0]) / 2.0, (a[1] + e[1]) / 2.0, a[2] - 0.06];
        b.obox(
            "mat",
            mid,
            [dx / 2.0, dy / 2.0, 0.0],
            [-dy / len * 0.004, dx / len * 0.004, 0.0],
            [0.0, 0.0, 0.06],
        );
    }
}

/// Big pots for a terrace: 1 terracotta with a small olive tree, 2 a clay
/// amphora, 3 a fibre-cement cube with tall grasses, 4 a low bowl of
/// lavender. `height` is of the whole (pot and plant).
pub fn big_pot(b: &mut Builder, p: &Params) {
    let w = p.get("width", 0.6).clamp(0.25, 1.4);
    let h = p.get("height", 1.4).clamp(0.3, 2.8);
    let r = w / 2.0;
    let c = [0.0, r];
    match p.style() {
        2 => {
            let ph = h.min(1.2);
            b.lathe(
                "pot",
                [c[0], c[1], 0.0],
                &[
                    (0.0, 0.0),
                    (r * 0.35, 0.0),
                    (r * 0.8, ph * 0.25),
                    (r, ph * 0.5),
                    (r * 0.75, ph * 0.78),
                    (r * 0.4, ph * 0.9),
                    (r * 0.45, ph),
                    (0.0, ph),
                ],
                1.0,
                1.0,
            );
        }
        3 => {
            let ph = (h * 0.4).clamp(0.25, 0.9);
            b.rbox("pot", [-r, 0.0, 0.0], [r, w, ph], 0.015);
            b.rbox(
                "soil",
                [-r + 0.03, 0.03, ph - 0.04],
                [r - 0.03, w - 0.03, ph - 0.01],
                0.0,
            );
            let blades = 34;
            for i in 0..blades {
                let a = TAU * (i as f64 * 0.618_034).fract();
                let rr = r * 0.5 * ((i * 7 % 10) as f64 / 10.0);
                let (ca, sa) = (a.cos(), a.sin());
                let base = [c[0] + ca * rr, c[1] + sa * rr, ph - 0.02];
                let tall = (h - ph) * (0.7 + 0.3 * ((i * 3 % 7) as f64 / 7.0));
                let out = 0.25 + 0.35 * ((i * 5 % 9) as f64 / 9.0);
                b.blob(
                    "leaves",
                    [
                        base[0] + ca * out * tall * 0.25,
                        base[1] + sa * out * tall * 0.25,
                        base[2] + tall * 0.5,
                    ],
                    [-sa * 0.012, ca * 0.012, 0.0],
                    [ca * 0.006, sa * 0.006, 0.0],
                    [ca * out * tall * 0.25, sa * out * tall * 0.25, tall * 0.5],
                );
            }
        }
        4 => {
            let ph = (w * 0.45).min(h * 0.5);
            b.lathe(
                "pot",
                [c[0], c[1], 0.0],
                &[
                    (0.0, 0.0),
                    (r * 0.6, 0.0),
                    (r, ph * 0.85),
                    (r * 1.03, ph),
                    (0.0, ph),
                ],
                1.0,
                1.0,
            );
            let n = 5;
            for i in 0..n {
                let a = TAU * i as f64 / n as f64 + 0.3;
                let rr = if i == 0 { 0.0 } else { r * 0.5 };
                let (x, y) = (c[0] + a.cos() * rr, c[1] + a.sin() * rr);
                let bush = (h - ph).min(r * 0.9) * 0.45;
                b.ellipsoid(
                    "leaves",
                    [x, y, ph + bush * 0.6],
                    [bush * 0.9, bush * 0.9, bush * 0.6],
                );
                for k in 0..7 {
                    let t = TAU * k as f64 / 7.0 + i as f64;
                    let q = [
                        x + t.cos() * bush * 0.7,
                        y + t.sin() * bush * 0.7,
                        ph + bush * 0.8,
                    ];
                    b.cyl(
                        "stem",
                        [q[0], q[1], q[2] - 0.02],
                        [q[0], q[1], q[2] + bush * 0.6],
                        0.003,
                    );
                    b.ellipsoid(
                        "flowers",
                        [q[0], q[1], q[2] + bush * 0.7],
                        [0.008, 0.008, 0.035],
                    );
                }
            }
        }
        _ => {
            let ph = (w * 0.85).min(h * 0.4);
            b.lathe(
                "pot",
                [c[0], c[1], 0.0],
                &[
                    (0.0, 0.0),
                    (r * 0.7, 0.0),
                    (r * 0.95, ph * 0.85),
                    (r * 1.05, ph * 0.9),
                    (r * 1.05, ph),
                    (0.0, ph),
                ],
                1.0,
                1.0,
            );
            // A crooked trunk and a crown of grey-green tufts.
            let crown = (h - ph) * 0.45;
            let top = [c[0] + 0.05, c[1] - 0.03, h - crown * 0.9];
            b.cone(
                "stem",
                [c[0], c[1], ph - 0.02],
                [c[0] - 0.03, c[1], ph + (top[2] - ph) * 0.5],
                0.04,
                0.03,
                10,
            );
            b.cone(
                "stem",
                [c[0] - 0.03, c[1], ph + (top[2] - ph) * 0.5],
                top,
                0.03,
                0.02,
                10,
            );
            let n = 9;
            for i in 0..n {
                let a = TAU * (i as f64 * 0.618_034).fract();
                let up = ((i * 5 % 7) as f64 / 7.0 - 0.3) * crown * 0.6;
                let out = crown * (0.45 + 0.3 * ((i * 3 % 4) as f64 / 4.0));
                let q = [top[0] + a.cos() * out, top[1] + a.sin() * out, top[2] + up];
                b.cyl("stem", top, q, 0.01);
                b.ellipsoid("leaves", q, [crown * 0.38, crown * 0.38, crown * 0.28]);
            }
        }
    }
}

/// Pergolas: 1 of wood, posts, beams and rafters; 2 of aluminium with
/// louvres; 3 of wood with a reed mat on top and a climber up a post.
/// `height` is of its top.
pub fn pergola(b: &mut Builder, p: &Params) {
    let w = p.get("width", 3.0).clamp(1.5, 8.0);
    let d = p.get("depth", 3.0).clamp(1.5, 8.0);
    let h = p.get("height", 2.5).clamp(2.1, 2.95);
    let x0 = -w / 2.0;
    let post = 0.12;
    let posts = [
        [x0 + post / 2.0, post / 2.0],
        [-x0 - post / 2.0, post / 2.0],
        [x0 + post / 2.0, d - post / 2.0],
        [-x0 - post / 2.0, d - post / 2.0],
    ];
    match p.style() {
        2 => {
            let f = 0.14;
            for q in posts {
                b.rbox(
                    "metal",
                    [q[0] - f / 2.0, q[1] - f / 2.0, 0.0],
                    [q[0] + f / 2.0, q[1] + f / 2.0, h],
                    0.004,
                );
            }
            for (lo, hi) in [
                ([x0, 0.0, h - 0.2], [-x0, f, h]),
                ([x0, d - f, h - 0.2], [-x0, d, h]),
                ([x0, 0.0, h - 0.2], [x0 + f, d, h]),
                ([-x0 - f, 0.0, h - 0.2], [-x0, d, h]),
            ] {
                b.rbox("metal", lo, hi, 0.004);
            }
            // The louvres, a little open.
            let n = ((w - 2.0 * f) / 0.2).floor().max(3.0) as usize;
            let step = (w - 2.0 * f) / n as f64;
            for i in 0..n {
                let x = x0 + f + (i as f64 + 0.5) * step;
                let a = 0.5_f64;
                b.obox(
                    "metal",
                    [x, d / 2.0, h - 0.08],
                    [a.cos() * step * 0.48, 0.0, a.sin() * step * 0.48],
                    [0.0, (d - 2.0 * f) / 2.0, 0.0],
                    [-a.sin() * 0.008, 0.0, a.cos() * 0.008],
                );
            }
        }
        style => {
            for q in posts {
                b.rbox(
                    "frame",
                    [q[0] - post / 2.0, q[1] - post / 2.0, 0.0],
                    [q[0] + post / 2.0, q[1] + post / 2.0, h - 0.2],
                    0.008,
                );
            }
            // Two beams along x over the posts, rafters across them.
            for y in [post / 2.0, d - post / 2.0] {
                b.rbox(
                    "frame",
                    [x0 - 0.2, y - 0.04, h - 0.38],
                    [-x0 + 0.2, y + 0.04, h - 0.2],
                    0.006,
                );
            }
            let n = ((w + 0.2) / 0.45).floor().max(3.0) as usize;
            for i in 0..=n {
                let x = x0 - 0.1 + i as f64 * (w + 0.2) / n as f64;
                b.rbox(
                    "frame",
                    [x - 0.03, -0.2, h - 0.2],
                    [x + 0.03, d + 0.2, h - 0.04],
                    0.006,
                );
            }
            if style == 3 {
                b.rbox(
                    "mat",
                    [x0 - 0.1, -0.1, h - 0.04],
                    [-x0 + 0.1, d + 0.1, h - 0.01],
                    0.0,
                );
                // A climber up the front left post, spreading over the top.
                let q = posts[2];
                for i in 0..16 {
                    let t = i as f64 / 15.0;
                    let a = t * TAU * 2.2;
                    let z = 0.3 + (h - 0.6) * t;
                    let rr = 0.12 + 0.05 * (i % 3) as f64;
                    b.ellipsoid(
                        "leaves",
                        [q[0] + a.cos() * rr, q[1] + a.sin() * rr, z],
                        [0.16, 0.16, 0.14],
                    );
                }
                for i in 0..10 {
                    let t = i as f64 / 9.0;
                    let x = q[0] + (w * 0.6) * t;
                    let y = q[1] - 0.2 - (d * 0.35) * (t * PI).sin() * 0.5;
                    b.ellipsoid("leaves", [x, y, h + 0.02], [0.22, 0.2, 0.1]);
                }
            }
        }
    }
}
