//! More of what a house has (revisión de Antonio del 24/10/2026):
//! fireplaces and wood stoves, coat stands and hall benches, office
//! chairs, garden tables with their chairs, hanging chairs and hammocks,
//! radiators, air conditioning, ceiling fans, pianos and children's beds.
//! Axes as the furniture's: origin in the middle of the back (on the
//! wall), `x` to the right, `y` forwards, `z` up.

use crate::build::{Builder, P3};
use crate::Params;

use std::f64::consts::{PI, TAU};

/// Logs and flames in a hearth from `x0` to `x1` at depth `y`, on `z`.
fn fire(b: &mut Builder, x0: f64, x1: f64, y: f64, z: f64) {
    let w = x1 - x0;
    for (k, dy) in [(0.0, -0.04), (0.5, 0.04), (0.25, 0.0)] {
        let a = [x0 + w * (0.15 + k * 0.5), y + dy, z + 0.045 + k * 0.08];
        let e = [x0 + w * (0.85 - k * 0.3), y - dy, z + 0.045 + k * 0.08];
        b.cyl("wood", a, e, 0.045);
    }
    let n = ((w / 0.12).round() as usize).max(3);
    for i in 0..n {
        let x = x0 + w * (i as f64 + 0.5) / n as f64;
        let tall = 0.12 + 0.12 * (((i * 7) % 5) as f64 / 5.0);
        b.blob(
            "flame",
            [x, y, z + 0.12 + tall * 0.5],
            [0.035, 0.0, 0.0],
            [0.0, 0.03, 0.0],
            [0.0, 0.0, tall * 0.5],
        );
    }
}

/// Fireplaces: 1 a chimney breast of masonry with its hearth, mantel and
/// fire, 2 a cast iron wood stove with its flue up to `height`, 3 a long
/// modern one set in a plastered wall. The fire glows at night.
pub fn fireplace(b: &mut Builder, p: &Params) {
    let w = p.get("width", 1.4).clamp(0.6, 4.0);
    let d = p.get("depth", 0.5).clamp(0.3, 1.2);
    let h = p.get("height", 2.5).clamp(1.0, 4.0);
    let x0 = -w / 2.0;
    match p.style() {
        2 => {
            let r = (w.min(0.9) * 0.3).max(0.2);
            let c = [0.0, d.max(0.5) * 0.6];
            let foot = 0.15;
            for k in 0..4 {
                let a = PI / 4.0 + k as f64 * PI / 2.0;
                let q = [c[0] + a.cos() * r * 0.75, c[1] + a.sin() * r * 0.75];
                b.cone(
                    "frame",
                    [q[0], q[1], 0.004],
                    [q[0], q[1], foot],
                    0.025,
                    0.02,
                    8,
                );
            }
            b.cyl("frame", [c[0], c[1], foot], [c[0], c[1], foot + 0.65], r);
            b.cone(
                "frame",
                [c[0], c[1], foot + 0.65],
                [c[0], c[1], foot + 0.75],
                r * 1.05,
                r * 0.6,
                24,
            );
            // The door with its glass and the fire behind it.
            let gy = c[1] + r * 0.98;
            b.rbox(
                "dark",
                [-r * 0.55, c[1], foot + 0.12],
                [r * 0.55, gy - 0.01, foot + 0.5],
                0.0,
            );
            fire(b, -r * 0.45, r * 0.45, gy - 0.08, foot + 0.12);
            b.rbox(
                "glass",
                [-r * 0.5, gy, foot + 0.15],
                [r * 0.5, gy + 0.008, foot + 0.48],
                0.0,
            );
            b.cyl(
                "metal",
                [r * 0.6, gy + 0.03, foot + 0.3],
                [r * 0.6, gy + 0.03, foot + 0.4],
                0.01,
            );
            b.cyl(
                "frame",
                [c[0], c[1], foot + 0.75],
                [c[0], c[1], h - 0.4],
                0.075,
            );
            b.cyl("frame", [c[0], c[1], h - 0.4], [c[0], 0.02, h - 0.2], 0.075);
            b.rbox(
                "stone",
                [-0.6_f64.max(x0), 0.0, 0.0],
                [0.6_f64.min(-x0), c[1] + r + 0.3, 0.03],
                0.01,
            );
        }
        3 => {
            b.rbox("mat", [x0, 0.0, 0.0], [-x0, d.min(0.35), h], 0.0);
            let (ox0, ox1) = (x0 + 0.15, -x0 - 0.15);
            let (z0, z1) = (0.45, 0.85);
            let y = d.min(0.35);
            b.rbox(
                "frame",
                [ox0 - 0.02, y, z0 - 0.02],
                [ox1 + 0.02, y + 0.01, z1 + 0.02],
                0.004,
            );
            b.rbox("dark", [ox0, y - 0.2, z0], [ox1, y + 0.002, z0 + 0.01], 0.0);
            let n = ((ox1 - ox0) / 0.08).round().max(4.0) as usize;
            for i in 0..n {
                let x = ox0 + (ox1 - ox0) * (i as f64 + 0.5) / n as f64;
                b.ellipsoid("stone", [x, y - 0.08, z0 + 0.02], [0.035, 0.03, 0.02]);
                let tall = 0.1 + 0.12 * (((i * 5) % 4) as f64 / 4.0);
                b.blob(
                    "flame",
                    [x, y - 0.08, z0 + 0.04 + tall * 0.5],
                    [0.03, 0.0, 0.0],
                    [0.0, 0.02, 0.0],
                    [0.0, 0.0, tall * 0.5],
                );
            }
            b.rbox("glass", [ox0, y + 0.01, z0], [ox1, y + 0.016, z1], 0.0);
        }
        _ => {
            // Two piers, the lintel and the breast up to the ceiling.
            let ow = (w * 0.55).min(0.9);
            let (ox0, ox1) = (-ow / 2.0, ow / 2.0);
            let (z0, z1) = (0.2, 0.9);
            b.rbox("mat", [x0, 0.0, 0.0], [ox0, d, 1.1], 0.01);
            b.rbox("mat", [ox1, 0.0, 0.0], [-x0, d, 1.1], 0.01);
            b.rbox("mat", [ox0, 0.0, z1], [ox1, d, 1.1], 0.0);
            b.rbox("mat", [ox0, 0.0, 0.0], [ox1, d, z0], 0.0);
            b.rbox("dark", [ox0, 0.0, z0], [ox1, d * 0.5, z1], 0.0);
            fire(b, ox0 + 0.06, ox1 - 0.06, d * 0.65, z0);
            // The mantel shelf and the breast above it, a little narrower.
            b.rbox(
                "frame",
                [x0 - 0.06, 0.0, 1.1],
                [-x0 + 0.06, d + 0.06, 1.16],
                0.01,
            );
            b.rbox("mat", [x0 + 0.1, 0.0, 1.16], [-x0 - 0.1, d - 0.12, h], 0.0);
            b.rbox(
                "stone",
                [x0 - 0.1, 0.0, 0.0],
                [-x0 + 0.1, d + 0.45, 0.06],
                0.01,
            );
        }
    }
}

/// Coat stands: 1 a pole with hooks and two coats on them, 2 a hall
/// bench with a board of hooks, coats and baskets under it.
pub fn coat_stand(b: &mut Builder, p: &Params) {
    let w = p.get("width", 0.5).clamp(0.3, 2.0);
    let h = p.get("height", 1.8).clamp(1.2, 2.2);
    match p.style() {
        2 => {
            let x0 = -w / 2.0;
            let d = 0.4;
            b.rbox("mat", [x0, 0.0, 0.42], [-x0, d, 0.46], 0.01);
            for x in [x0 + 0.03, -x0 - 0.06] {
                for y in [0.03, d - 0.06] {
                    b.rbox("mat", [x, y, 0.0], [x + 0.03, y + 0.03, 0.42], 0.004);
                }
            }
            b.rbox(
                "mat",
                [x0 + 0.03, 0.03, 0.1],
                [-x0 - 0.03, d - 0.03, 0.12],
                0.0,
            );
            let n = ((w / 0.4).floor() as usize).max(1);
            for i in 0..n {
                let a = x0 + 0.05 + (w - 0.1) * i as f64 / n as f64;
                b.rbox(
                    "weave",
                    [a + 0.02, 0.06, 0.12],
                    [a + (w - 0.1) / n as f64 - 0.02, d - 0.06, 0.38],
                    0.03,
                );
            }
            b.rbox("mat", [x0, 0.0, 1.2], [-x0, 0.02, h], 0.004);
            let hooks = ((w / 0.25).round() as usize).max(2);
            for i in 0..hooks {
                let x = x0 + w * (i as f64 + 0.5) / hooks as f64;
                b.cyl("metal", [x, 0.02, h - 0.15], [x, 0.09, h - 0.12], 0.008);
                if i % 2 == 0 {
                    coat(
                        b,
                        [x, 0.09, h - 0.14],
                        if i % 4 == 0 { "cushion" } else { "cushion2" },
                    );
                }
            }
        }
        _ => {
            let c = [0.0, 0.25];
            for k in 0..3 {
                let a = PI / 2.0 + TAU * k as f64 / 3.0;
                b.cyl(
                    "mat",
                    [c[0], c[1], 0.25],
                    [c[0] + a.cos() * 0.25, c[1] + a.sin() * 0.25, 0.016],
                    0.014,
                );
            }
            b.cyl("mat", [c[0], c[1], 0.0], [c[0], c[1], h], 0.018);
            b.ellipsoid("mat", [c[0], c[1], h + 0.02], [0.03, 0.03, 0.03]);
            for k in 0..4 {
                let a = TAU * k as f64 / 4.0 + 0.4;
                let tip = [c[0] + a.cos() * 0.13, c[1] + a.sin() * 0.13, h - 0.05];
                b.cyl("mat", [c[0], c[1], h - 0.15], tip, 0.01);
            }
            coat(b, [c[0] + 0.12, c[1] + 0.03, h - 0.07], "cushion");
            coat(b, [c[0] - 0.1, c[1] - 0.06, h - 0.07], "cushion2");
        }
    }
}

/// A coat hanging from a hook at `top`.
fn coat(b: &mut Builder, top: P3, role: &'static str) {
    let l = 0.85;
    b.blob(
        role,
        [top[0], top[1] + 0.06, top[2] - l * 0.5],
        [0.2, 0.0, 0.0],
        [0.0, 0.08, 0.0],
        [0.0, 0.0, l * 0.5],
    );
}

/// Office chairs: 1 a task chair on five castors, 2 one with a mesh back
/// and a headrest. The seat's middle at `(0, width / 2)`.
pub fn office_chair(b: &mut Builder, p: &Params) {
    // A collection's (paquete 6, tanda 6b).
    if crate::fine::study::office_chair(b, p) {
        return;
    }
    let w = p.get("width", 0.62).clamp(0.45, 0.8);
    let seat = p.get("seat", 0.47).clamp(0.38, 0.56);
    let c = [0.0, w / 2.0];
    for k in 0..5 {
        let a = PI / 2.0 + TAU * k as f64 / 5.0;
        let q = [c[0] + a.cos() * w * 0.45, c[1] + a.sin() * w * 0.45];
        b.rbox(
            "dark",
            [q[0].min(c[0]) - 0.02, q[1].min(c[1]) - 0.02, 0.07],
            [q[0].max(c[0]) + 0.02, q[1].max(c[1]) + 0.02, 0.1],
            0.01,
        );
        b.ellipsoid("dark", [q[0], q[1], 0.035], [0.03, 0.025, 0.035]);
    }
    b.cyl("metal", [c[0], c[1], 0.1], [c[0], c[1], seat - 0.08], 0.025);
    b.pillow(
        "cushion",
        [-w * 0.4, c[1] - w * 0.36, seat - 0.08],
        [w * 0.4, c[1] + w * 0.38, seat],
        0.04,
        0.012,
    );
    for x in [-w * 0.45, w * 0.45] {
        b.cyl(
            "dark",
            [x, c[1] - 0.12, seat - 0.05],
            [x, c[1] - 0.12, seat + 0.2],
            0.012,
        );
        b.rbox(
            "dark",
            [x - 0.03, c[1] - 0.22, seat + 0.19],
            [x + 0.03, c[1] + 0.08, seat + 0.23],
            0.012,
        );
    }
    let back_y = c[1] - w * 0.38;
    b.rbox(
        "dark",
        [-0.02, back_y - 0.04, seat - 0.06],
        [0.02, back_y, seat + 0.25],
        0.008,
    );
    match p.style() {
        2 => {
            b.rbox(
                "frame",
                [-w * 0.36, back_y - 0.06, seat + 0.12],
                [w * 0.36, back_y - 0.03, seat + 0.62],
                0.04,
            );
            b.rbox(
                "mesh",
                [-w * 0.33, back_y - 0.065, seat + 0.15],
                [w * 0.33, back_y - 0.025, seat + 0.59],
                0.02,
            );
            b.rbox(
                "cushion",
                [-w * 0.2, back_y - 0.1, seat + 0.66],
                [w * 0.2, back_y - 0.04, seat + 0.78],
                0.03,
            );
        }
        _ => {
            b.pillow(
                "cushion",
                [-w * 0.36, back_y - 0.08, seat + 0.1],
                [w * 0.36, back_y - 0.02, seat + 0.55],
                0.05,
                0.015,
            );
        }
    }
}

/// Garden tables with their chairs: 1 a round bistro table with two
/// folding chairs, 2 a teak table for six, 3 a low outdoor sofa with its
/// coffee table. `width` is the table's.
pub fn outdoor_set(b: &mut Builder, p: &Params) {
    let w = p.get("width", 0.6).clamp(0.5, 3.0);
    let d = p.get("depth", 0.6).clamp(0.5, 1.4);
    match p.style() {
        2 => {
            let (cy, h) = (d / 2.0 + 0.55, 0.75);
            b.rbox(
                "mat",
                [-w / 2.0, cy - d / 2.0, h - 0.04],
                [w / 2.0, cy + d / 2.0, h],
                0.01,
            );
            for (x, y) in [
                (-w / 2.0 + 0.08, cy - d / 2.0 + 0.08),
                (w / 2.0 - 0.08, cy - d / 2.0 + 0.08),
                (-w / 2.0 + 0.08, cy + d / 2.0 - 0.08),
                (w / 2.0 - 0.08, cy + d / 2.0 - 0.08),
            ] {
                b.rbox(
                    "mat",
                    [x - 0.03, y - 0.03, 0.0],
                    [x + 0.03, y + 0.03, h - 0.04],
                    0.005,
                );
            }
            let side = ((w / 0.65).floor() as usize).max(1);
            for i in 0..side {
                let x = -w / 2.0 + w * (i as f64 + 0.5) / side as f64;
                garden_chair(b, [x, cy - d / 2.0 - 0.25], 0.0);
                garden_chair(b, [x, cy + d / 2.0 + 0.25], PI);
            }
            garden_chair(b, [-w / 2.0 - 0.3, cy], PI / 2.0);
            garden_chair(b, [w / 2.0 + 0.3, cy], -PI / 2.0);
        }
        3 => {
            let sw = w.max(1.6);
            b.rbox("mat", [-sw / 2.0, 0.0, 0.08], [sw / 2.0, 0.8, 0.38], 0.02);
            b.rbox("mat", [-sw / 2.0, 0.0, 0.08], [sw / 2.0, 0.15, 0.7], 0.02);
            for x in [-sw / 2.0, sw / 2.0 - 0.12] {
                b.rbox("mat", [x, 0.0, 0.08], [x + 0.12, 0.8, 0.58], 0.02);
            }
            for (x, y) in [
                (-sw / 2.0 + 0.05, 0.05),
                (sw / 2.0 - 0.05, 0.05),
                (-sw / 2.0 + 0.05, 0.75),
                (sw / 2.0 - 0.05, 0.75),
            ] {
                b.rbox(
                    "frame",
                    [x - 0.025, y - 0.025, 0.0],
                    [x + 0.025, y + 0.025, 0.08],
                    0.004,
                );
            }
            b.pillow(
                "cushion",
                [-sw / 2.0 + 0.12, 0.15, 0.38],
                [sw / 2.0 - 0.12, 0.78, 0.5],
                0.04,
                0.02,
            );
            let n = ((sw / 0.6).round() as usize).max(2);
            for i in 0..n {
                let a = -sw / 2.0 + 0.12 + (sw - 0.24) * i as f64 / n as f64;
                b.pillow(
                    "cushion",
                    [a + 0.01, 0.15, 0.48],
                    [a + (sw - 0.24) / n as f64 - 0.01, 0.3, 0.85],
                    0.05,
                    0.03,
                );
            }
            b.rbox("frame", [-0.45, 1.05, 0.0], [0.45, 1.55, 0.35], 0.02);
        }
        _ => {
            let c = [0.0, d / 2.0 + 0.45];
            let r = w / 2.0;
            b.cone(
                "frame",
                [c[0], c[1], 0.0],
                [c[0], c[1], 0.02],
                0.22,
                0.2,
                20,
            );
            b.cyl("frame", [c[0], c[1], 0.02], [c[0], c[1], 0.72], 0.02);
            b.cone("mat", [c[0], c[1], 0.72], [c[0], c[1], 0.74], r, r, 28);
            garden_chair(b, [c[0], c[1] - r - 0.25], 0.0);
            garden_chair(b, [c[0], c[1] + r + 0.25], PI);
        }
    }
}

/// A garden chair at `c`, its seat facing `+y` turned `turn`.
fn garden_chair(b: &mut Builder, c: [f64; 2], turn: f64) {
    let (s, co) = turn.sin_cos();
    let at = |x: f64, y: f64, z: f64| [c[0] + x * co - y * s, c[1] + x * s + y * co, z];
    for (x, y) in [(-0.2, -0.2), (0.2, -0.2), (-0.2, 0.18), (0.2, 0.18)] {
        b.cyl(
            "frame",
            at(x, y, 0.004),
            at(x * 0.95, y * 0.95, 0.45),
            0.012,
        );
    }
    let seat = [
        at(-0.21, -0.21, 0.45),
        at(0.21, -0.21, 0.45),
        at(0.21, 0.2, 0.45),
        at(-0.21, 0.2, 0.45),
    ];
    let poly: Vec<[f64; 2]> = seat.iter().map(|q| [q[0], q[1]]).collect();
    b.slab("mat", &poly, 0.45, 0.47);
    // The back: two posts and three slats.
    for x in [-0.2, 0.2] {
        b.cyl("frame", at(x, -0.2, 0.45), at(x, -0.24, 0.85), 0.012);
    }
    for k in 0..3 {
        let z = 0.6 + 0.09 * k as f64;
        b.cyl(
            "mat",
            at(-0.2, -0.215 - 0.01 * k as f64, z),
            at(0.2, -0.215 - 0.01 * k as f64, z),
            0.018,
        );
    }
}

/// Hanging chairs: 1 an egg of rattan on its stand, with a cushion, 2 a
/// hammock of cloth on a wooden frame, `width` long.
pub fn hanging_chair(b: &mut Builder, p: &Params) {
    let w = p.get("width", 1.0).clamp(0.8, 3.5);
    match p.style() {
        2 => {
            let l = w.max(2.6);
            let y = 0.6;
            b.rbox(
                "frame",
                [-l / 2.0, y - 0.05, 0.0],
                [l / 2.0, y + 0.05, 0.08],
                0.01,
            );
            for s in [-1.0, 1.0] {
                b.cyl(
                    "frame",
                    [s * l / 2.0, y, 0.04],
                    [s * (l / 2.0 - 0.35), y, 1.2],
                    0.04,
                );
            }
            // The cloth sagging between the two ends.
            let n = 12;
            let at = |t: f64| {
                let x = -(l / 2.0 - 0.42) + (l - 0.84) * t;
                let z = 1.15 - 0.6 * (PI * t).sin();
                [x, y, z]
            };
            for i in 0..n {
                let a = at(i as f64 / n as f64);
                let e = at((i + 1) as f64 / n as f64);
                b.obox(
                    "cushion",
                    [(a[0] + e[0]) / 2.0, y, (a[2] + e[2]) / 2.0],
                    [(e[0] - a[0]) / 2.0, 0.0, (e[2] - a[2]) / 2.0],
                    [0.0, 0.38, 0.0],
                    [0.0, 0.0, 0.012],
                );
            }
        }
        _ => {
            let c = [0.0, 0.55];
            b.cone(
                "frame",
                [c[0], c[1], 0.0],
                [c[0], c[1], 0.05],
                0.4,
                0.38,
                24,
            );
            b.cyl(
                "frame",
                [c[0], c[1] - 0.3, 0.05],
                [c[0], c[1] - 0.45, 1.6],
                0.025,
            );
            let top = [c[0], c[1] - 0.45, 1.6];
            let mut last = top;
            for i in 1..=6 {
                let a = PI / 2.0 * i as f64 / 6.0;
                let q = [
                    c[0],
                    c[1] - 0.45 + 0.45 * a.sin(),
                    1.6 + 0.35 * a.cos() * 0.4 + 0.12 * a.sin(),
                ];
                b.cyl("frame", last, q, 0.022);
                last = q;
            }
            b.cyl("metal", last, [c[0], c[1], 1.5], 0.006);
            // The egg: an open shell of rattan with a cushion in it.
            b.blob(
                "weave",
                [c[0], c[1], 0.95],
                [0.45, 0.0, 0.0],
                [0.0, 0.38, 0.0],
                [0.0, 0.0, 0.55],
            );
            b.blob(
                "dark",
                [c[0], c[1] + 0.2, 0.92],
                [0.35, 0.0, 0.0],
                [0.0, 0.2, 0.0],
                [0.0, 0.0, 0.42],
            );
            b.pillow(
                "cushion",
                [-0.3, c[1] - 0.1, 0.62],
                [0.3, c[1] + 0.3, 0.72],
                0.05,
                0.03,
            );
        }
    }
}

/// Radiators on the wall, from `elevation`: 1 of aluminium sections, 2 a
/// panel of steel.
pub fn radiator(b: &mut Builder, p: &Params) {
    let w = p.get("width", 0.8).clamp(0.3, 2.5);
    let h = p.get("height", 0.6).clamp(0.3, 2.0);
    let z = p.get("elevation", 0.15).clamp(0.0, 1.5);
    let x0 = -w / 2.0;
    for x in [x0 + 0.08, -x0 - 0.08] {
        b.cyl(
            "metal",
            [x, 0.0, z + h * 0.7],
            [x, 0.05, z + h * 0.7],
            0.008,
        );
    }
    match p.style() {
        2 => {
            b.rbox("mat", [x0, 0.04, z], [-x0, 0.1, z + h], 0.01);
            for k in 0..((w / 0.05) as usize) {
                let x = x0 + 0.025 + k as f64 * 0.05;
                b.rbox(
                    "mat",
                    [x - 0.008, 0.1, z + 0.02],
                    [x + 0.008, 0.104, z + h - 0.02],
                    0.0,
                );
            }
        }
        _ => {
            let n = ((w / 0.08).round() as usize).max(3);
            let s = w / n as f64;
            for i in 0..n {
                let a = x0 + i as f64 * s;
                b.rbox(
                    "mat",
                    [a + 0.004, 0.03, z],
                    [a + s - 0.004, 0.11, z + h],
                    0.012,
                );
            }
        }
    }
    // The pipe from the floor.
    if z > 0.05 {
        b.cyl(
            "metal",
            [x0 + 0.03, 0.08, 0.004],
            [x0 + 0.03, 0.08, z],
            0.009,
        );
    }
}

/// Air conditioning: the unit of a split on the wall, high up.
pub fn ac_unit(b: &mut Builder, p: &Params) {
    let w = p.get("width", 0.85).clamp(0.6, 1.4);
    let z = p.get("elevation", 2.1).clamp(1.5, 2.8);
    let x0 = -w / 2.0;
    b.rbox("mat", [x0, 0.0, z], [-x0, 0.2, z + 0.29], 0.06);
    b.rbox(
        "dark",
        [x0 + 0.04, 0.14, z + 0.01],
        [-x0 - 0.04, 0.205, z + 0.05],
        0.01,
    );
    b.rbox(
        "light",
        [-x0 - 0.12, 0.2, z + 0.12],
        [-x0 - 0.08, 0.205, z + 0.14],
        0.0,
    );
}

/// Ceiling fans, hanging from `height` (the ceiling), centred on
/// `(0, width / 2)`: 1 of wooden blades with a light, 2 of rattan blades.
pub fn ceiling_fan(b: &mut Builder, p: &Params) {
    let w = p.get("width", 1.3).clamp(0.8, 1.8);
    let h = p.get("height", 2.6).clamp(2.2, 4.0);
    let c = [0.0, w / 2.0];
    let z = h - 0.45;
    b.cone(
        "frame",
        [c[0], c[1], h - 0.04],
        [c[0], c[1], h],
        0.07,
        0.08,
        16,
    );
    b.cyl(
        "frame",
        [c[0], c[1], z + 0.1],
        [c[0], c[1], h - 0.04],
        0.012,
    );
    b.lathe(
        "frame",
        [c[0], c[1], z - 0.04],
        &[
            (0.0, 0.0),
            (0.1, 0.02),
            (0.11, 0.1),
            (0.05, 0.14),
            (0.0, 0.14),
        ],
        1.0,
        1.0,
    );
    let role = if p.style() == 2 { "weave" } else { "mat" };
    for k in 0..5 {
        let a = TAU * k as f64 / 5.0;
        let (co, s) = (a.cos(), a.sin());
        let mid = [c[0] + co * w * 0.28, c[1] + s * w * 0.28, z + 0.03];
        b.obox(
            role,
            mid,
            [co * w * 0.2, s * w * 0.2, 0.0],
            [-s * 0.065, co * 0.065, 0.0],
            [0.0, 0.0, 0.006],
        );
    }
    if p.style() != 2 {
        b.ellipsoid("bulb", [c[0], c[1], z - 0.07], [0.08, 0.08, 0.06]);
    }
}

/// Pianos: 1 an upright against the wall with its stool, 2 a grand.
pub fn piano(b: &mut Builder, p: &Params) {
    match p.style() {
        2 => {
            let (w, l) = (1.5, 1.85);
            let x0 = -w / 2.0;
            b.slab(
                "mat",
                &[[x0, 0.1], [-x0, 0.1], [-x0, 0.9], [x0, 0.9]],
                0.62,
                0.98,
            );
            b.slab(
                "mat",
                &[
                    [x0, 0.9],
                    [-x0 - 0.1, 0.9],
                    [x0 + w * 0.55, 0.1 + l * 0.95],
                    [x0, 0.1 + l],
                ],
                0.62,
                0.98,
            );
            for (x, y) in [(x0 + 0.1, 0.25), (-x0 - 0.1, 0.25), (x0 + 0.2, l - 0.1)] {
                b.cone("mat", [x, y, 0.0], [x, y, 0.62], 0.04, 0.05, 10);
            }
            b.rbox(
                "light",
                [x0 + 0.08, 0.0, 0.7],
                [-x0 - 0.08, 0.15, 0.74],
                0.003,
            );
            b.obox(
                "mat",
                [x0 + 0.6, 0.9, 1.25],
                [0.55, 0.0, 0.0],
                [0.0, 0.35, 0.28],
                [0.0, -0.02, 0.025],
            );
        }
        _ => {
            let w = 1.48;
            let x0 = -w / 2.0;
            b.rbox("mat", [x0, 0.0, 0.0], [-x0, 0.32, 1.22], 0.01);
            b.rbox("mat", [x0, 0.0, 0.66], [-x0, 0.58, 0.74], 0.01);
            for x in [x0, -x0 - 0.06] {
                b.rbox("mat", [x, 0.32, 0.0], [x + 0.06, 0.58, 0.74], 0.008);
            }
            b.rbox(
                "light",
                [x0 + 0.08, 0.32, 0.72],
                [-x0 - 0.08, 0.5, 0.75],
                0.002,
            );
            for k in 0..36 {
                let x = x0 + 0.1 + (w - 0.2) * k as f64 / 36.0;
                if [1, 2, 4, 5, 6].contains(&(k % 7)) {
                    b.rbox(
                        "dark",
                        [x - 0.006, 0.38, 0.75],
                        [x + 0.006, 0.47, 0.765],
                        0.0,
                    );
                }
            }
            b.rbox("frame", [-0.28, 0.85, 0.0], [0.28, 1.2, 0.5], 0.02);
        }
    }
}

/// Children's beds: 1 a cabin bed under a frame like a little house, 2
/// bunk beds with their ladder. `width` and `depth` are the mattress's.
pub fn kids_bed(b: &mut Builder, p: &Params) {
    // A collection's (paquete 3).
    if crate::fine::nursery::kids_bed(b, p) {
        return;
    }
    let w = p.get("width", 0.9).clamp(0.7, 1.4);
    let l = p.get("depth", 1.9).clamp(1.4, 2.1);
    let x0 = -w / 2.0;
    let mattress = |b: &mut Builder, z: f64| {
        b.rbox(
            "linen",
            [x0 + 0.03, 0.03, z],
            [-x0 - 0.03, l - 0.03, z + 0.14],
            0.04,
        );
        b.pillow(
            "duvet",
            [x0 + 0.02, l * 0.35, z + 0.08],
            [-x0 - 0.02, l - 0.02, z + 0.2],
            0.04,
            0.02,
        );
        b.pillow(
            "linen",
            [x0 + 0.1, 0.06, z + 0.13],
            [-x0 - 0.1, 0.4, z + 0.24],
            0.04,
            0.03,
        );
    };
    match p.style() {
        2 => {
            for (x, y) in [
                (x0, 0.0),
                (-x0 - 0.06, 0.0),
                (x0, l - 0.06),
                (-x0 - 0.06, l - 0.06),
            ] {
                b.rbox("mat", [x, y, 0.0], [x + 0.06, y + 0.06, 1.75], 0.008);
            }
            for z in [0.2, 1.2] {
                b.rbox("mat", [x0, 0.0, z], [-x0, l, z + 0.06], 0.006);
                mattress(b, z + 0.06);
            }
            b.rbox("mat", [x0, 0.0, 1.45], [-x0, 0.04, 1.6], 0.006);
            b.rbox("mat", [x0, l - 0.04, 1.45], [-x0, l, 1.6], 0.006);
            b.rbox("mat", [-x0 - 0.04, 0.0, 1.45], [-x0, l * 0.65, 1.6], 0.006);
            // The ladder at the foot.
            for y in [l - 0.4, l - 0.08] {
                b.rbox(
                    "mat",
                    [-x0, y - 0.02, 0.0],
                    [-x0 + 0.04, y + 0.02, 1.6],
                    0.006,
                );
            }
            for k in 1..5 {
                let z = 0.3 * k as f64;
                b.cyl(
                    "mat",
                    [-x0 + 0.02, l - 0.4, z],
                    [-x0 + 0.02, l - 0.08, z],
                    0.014,
                );
            }
        }
        _ => {
            b.rbox("mat", [x0 - 0.04, 0.0, 0.08], [-x0 + 0.04, l, 0.2], 0.01);
            mattress(b, 0.2);
            // The posts and the roof of the little house.
            let top = 1.45;
            let ridge = top + w * 0.5;
            for y in [0.0, l - 0.05] {
                for x in [x0 - 0.04, -x0] {
                    b.rbox("mat", [x, y, 0.0], [x + 0.04, y + 0.05, top], 0.006);
                }
                let leg = |s: f64| [s * (w / 2.0 + 0.02), y + 0.025, top];
                b.cyl("mat", leg(-1.0), [0.0, y + 0.025, ridge], 0.025);
                b.cyl("mat", leg(1.0), [0.0, y + 0.025, ridge], 0.025);
            }
            b.cyl("mat", [0.0, 0.0, ridge], [0.0, l, ridge], 0.025);
            for x in [x0 - 0.02, -x0 + 0.02] {
                b.cyl("mat", [x, 0.0, top], [x, l, top], 0.022);
            }
            // A garland of little lights along the ridge.
            for k in 0..12 {
                let y = l * (k as f64 + 0.5) / 12.0;
                let sag = 0.06 * (PI * (k as f64 + 0.5) / 12.0).sin();
                b.ellipsoid(
                    "bulb",
                    [w * 0.15, y, ridge - 0.12 - sag],
                    [0.015, 0.015, 0.02],
                );
            }
        }
    }
}
