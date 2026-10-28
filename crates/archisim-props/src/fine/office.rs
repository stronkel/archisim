//! Offices of the eight collections (paquete 7, tanda 7b): the bench of
//! desks with its screen and monitors, the meeting table, the reception
//! desk, the office cabinet with its box files and the free-standing
//! acoustic screen, each as the collection makes it. The chairs are the
//! collections' own (`study::office_chair`, `chairs::chair`).
//!
//! Bench: `width`, `depth` (both desks), `height`, `places` (per side);
//! centred on `x = 0`, from `y = 0` to `depth`, one side each way.
//! Roles: `top`, `frame`, `metal`, `dark`, `screen`, `weave`, `accent`,
//! `display` (the monitors' screens).
//!
//! Meeting table: `width`, `depth`, `height`; centred on `x = 0`, from
//! `y = 0` to `depth`. Roles: `top`, `frame`, `metal`, `dark`, `accent`.
//!
//! Reception: `width`, `depth`, `height`; the staff at `y = 0`, the
//! visitors in front towards `+y`; one end lowered to 80 cm for a
//! wheelchair. Roles: `frame`, `top`, `carcass`, `masonry`, `accent`,
//! `metal`, `dark`, `mat`, `light`.
//!
//! Cabinet: `width`, `depth`, `height`; on the wall at `y = 0`. Roles:
//! `frame`, `front`, `top`, `metal`, `dark`, `binder`, `binder2`,
//! `binder3`.
//!
//! Screen: `width`, `height`; centred on `x = 0`, its middle on `y = 0`.
//! Roles: `frame`, `screen`, `weave`, `metal`, `dark`, `accent`.

use super::hospitality::{disc, look, rod, stone, turn_back, Rng};
use super::shape::{bar, sweep, turned, Cap};
use super::sofas::compass_leg;
use super::soft::Soft;
use super::storage::{doors, drawers, front, tapered_leg, toupie, Panel, Pull, T};
use super::tables::round_top;
use crate::build::{Builder, P3};
use crate::Params;
use std::f64::consts::{FRAC_PI_4, PI};

/// A monitor on its foot standing at `(x, y)` on a top at `z`, facing
/// `+y` (or `-y` with `back`), `w` wide.
fn monitor(b: &mut Builder, x: f64, y: f64, z: f64, w: f64, back: bool) {
    let start = b.mark();
    let h = w * 0.58;
    b.rbox("dark", [-0.1, -0.08, 0.0], [0.1, 0.08, 0.012], 0.006);
    b.rbox("dark", [-0.025, -0.05, 0.012], [0.025, -0.03, 0.2], 0.004);
    b.rbox(
        "dark",
        [-w / 2.0, -0.03, 0.12],
        [w / 2.0, -0.01, 0.12 + h],
        0.006,
    );
    b.rbox(
        "display",
        [-w / 2.0 + 0.008, -0.01, 0.13],
        [w / 2.0 - 0.008, -0.006, 0.11 + h],
        0.0,
    );
    // Keyboard.
    b.rbox("dark", [-0.21, 0.12, 0.0], [0.21, 0.26, 0.018], 0.004);
    if back {
        turn_back(b, start, 0.0);
    }
    b.transform_from(start, |q| [q[0] + x, q[1] + y, q[2] + z]);
}

/// Box files standing in a row from `x0` to `x1` on a shelf at `z`, their
/// backs against `yb`, `room` high; a ring hole and a label on their spines.
#[allow(clippy::too_many_arguments)]
fn binders(b: &mut Builder, x0: f64, x1: f64, yb: f64, depth: f64, z: f64, room: f64, seed: u32) {
    let mut rnd = Rng(0x68e3_1da4 ^ seed);
    let roles = ["binder", "binder2", "binder3"];
    let h = room.min(0.32);
    let mut x = x0 + 0.01;
    let mut k = 0usize;
    while x + 0.075 < x1 {
        if rnd.next() < 0.08 {
            x += 0.08;
            continue;
        }
        let w = 0.075;
        let role = roles[(k / 3 + (rnd.next() * 1.2) as usize) % 3];
        let yf = yb + depth.min(0.29);
        b.rbox(role, [x, yb + 0.01, z], [x + w - 0.004, yf, z + h], 0.003);
        b.rbox(
            "binder3",
            [x + 0.012, yf, z + h * 0.55],
            [x + w - 0.016, yf + 0.002, z + h * 0.75],
            0.0,
        );
        b.cyl(
            "dark",
            [x + w / 2.0 - 0.002, yf, z + h * 0.25],
            [x + w / 2.0 - 0.002, yf + 0.003, z + h * 0.25],
            0.012,
        );
        x += w;
        k += 1;
    }
}

/// A turned leg from the floor to `top` at `(x, y)`, `r` thick.
fn turned_leg(b: &mut Builder, x: f64, y: f64, top: f64, r: f64) {
    turned(
        b,
        "frame",
        [x, y, 0.0],
        &[
            (0.0, 0.0),
            (r * 0.75, 0.0),
            (r * 0.7, 0.05),
            (r * 1.1, top * 0.18),
            (r * 0.8, top * 0.24),
            (r, top * 0.88),
            (r * 1.15, top * 0.92),
            (r * 1.15, top),
            (0.0, top),
        ],
        16,
        0.0,
    );
}

/// A felt panel with soft rounded edges, from `lo` to `hi`.
fn felt(b: &mut Builder, role: &'static str, lo: P3, hi: P3) {
    let t = (hi[1] - lo[1]).min(hi[0] - lo[0]).min(hi[2] - lo[2]);
    let mut s = Soft::new(lo, hi, (t * 0.45).min(0.03));
    s.crown = t * 0.15;
    s.wobble = 0.0006;
    s.step = 0.15;
    s.add(b, role);
}

// ------------------------------------------------------------------- bench

/// Benches of desks of the collections.
pub fn bench_desk(b: &mut Builder, p: &Params) {
    let ix = look(p);
    let w = p.get("width", 2.8).clamp(1.2, 7.0);
    let d = p.get("depth", 1.6).clamp(1.2, 2.0);
    let h = p.get("height", 0.74).clamp(0.68, 0.8);
    let places = p.get("places", (w / 1.4).round()).round().clamp(1.0, 5.0) as usize;
    let (x0, x1) = (-w / 2.0, w / 2.0);
    let mid = d / 2.0;
    let t = [0.03, 0.035, 0.03, 0.04, 0.025, 0.05, 0.035, 0.035][ix];
    let zt = h - t;
    // The top, as two desks meeting under the screen.
    let top = |b: &mut Builder, role: &'static str, r: f64| {
        b.rbox(role, [x0, 0.0, zt], [x1, mid - 0.003, h], r);
        b.rbox(role, [x0, mid + 0.003, zt], [x1, d, h], r);
    };
    // Legs or frames at both ends (and between every two places).
    let ends: Vec<f64> = (0..=places)
        .map(|i| x0 + 0.06 + (w - 0.12) * i as f64 / places as f64)
        .collect();
    let screen_h = 0.4;
    match ix {
        // Nordic: oak on splayed tapered legs, a felt screen in oak.
        0 => {
            top(b, "top", 0.008);
            for x in [ends[0], ends[ends.len() - 1]] {
                let sx: f64 = if x < 0.0 { -0.5 } else { 0.5 };
                for (y, sy) in [(0.08, -0.5), (d - 0.08, 0.5)] {
                    tapered_leg(
                        b,
                        "frame",
                        [x + 0.02 * -sx.signum(), y],
                        zt,
                        [sx, sy],
                        0.04,
                        0.022,
                    );
                }
                b.rbox(
                    "frame",
                    [x - 0.015, 0.08, zt - 0.06],
                    [x + 0.015, d - 0.08, zt],
                    0.004,
                );
            }
            b.rbox(
                "frame",
                [x0 + 0.06, mid - 0.012, h],
                [x1 - 0.06, mid + 0.012, h + screen_h],
                0.006,
            );
            felt(
                b,
                "screen",
                [x0 + 0.08, mid - 0.016, h + 0.02],
                [x1 - 0.08, mid + 0.016, h + screen_h - 0.02],
            );
        }
        // Mediterranean: whitewashed wood on turned legs, a screen of cane in
        // a frame.
        1 => {
            top(b, "top", 0.005);
            for x in &ends {
                for y in [0.06, d - 0.06] {
                    turned_leg(b, *x, y, zt, 0.025);
                }
                b.rbox(
                    "frame",
                    [x - 0.015, 0.06, zt - 0.09],
                    [x + 0.015, d - 0.06, zt],
                    0.004,
                );
            }
            for y in [0.05, d - 0.07] {
                b.rbox(
                    "frame",
                    [x0 + 0.06, y, zt - 0.09],
                    [x1 - 0.06, y + 0.02, zt],
                    0.004,
                );
            }
            for x in [x0 + 0.06, x1 - 0.08] {
                b.rbox(
                    "frame",
                    [x, mid - 0.015, h],
                    [x + 0.02, mid + 0.015, h + screen_h],
                    0.003,
                );
            }
            b.rbox(
                "frame",
                [x0 + 0.06, mid - 0.015, h + screen_h - 0.025],
                [x1 - 0.06, mid + 0.015, h + screen_h],
                0.004,
            );
            b.rbox(
                "weave",
                [x0 + 0.08, mid - 0.006, h],
                [x1 - 0.08, mid + 0.006, h + screen_h - 0.025],
                0.0,
            );
        }
        // Fifties: teak on compass legs, a coloured screen with brass edges.
        2 => {
            top(b, "top", 0.01);
            for x in [ends[0], ends[ends.len() - 1]] {
                let sx: f64 = if x < 0.0 { -0.5 } else { 0.5 };
                for (y, sy) in [(0.1, -0.5), (d - 0.1, 0.5)] {
                    compass_leg(b, [x + 0.04 * -sx.signum(), y], zt, [sx, sy], 0.06);
                }
            }
            b.rbox(
                "frame",
                [x0 + 0.1, 0.1, zt - 0.05],
                [x1 - 0.1, d - 0.1, zt],
                0.006,
            );
            b.rbox(
                "accent",
                [x0 + 0.1, mid - 0.012, h],
                [x1 - 0.1, mid + 0.012, h + screen_h],
                0.01,
            );
            sweep(
                b,
                "metal",
                &[[x0 + 0.1, mid, h + screen_h], [x1 - 0.1, mid, h + screen_h]],
                |_| 0.014,
                10,
                Cap::Round(1.0),
                Cap::Round(1.0),
            );
        }
        // Japandi: smoked oak on slab ends, a screen of upright slats.
        3 => {
            top(b, "top", 0.004);
            for x in [x0 + 0.04, x1 - 0.08] {
                b.rbox("frame", [x, 0.06, 0.0], [x + 0.04, d - 0.06, zt], 0.004);
            }
            b.rbox(
                "frame",
                [x0 + 0.08, mid - 0.06, 0.12],
                [x1 - 0.08, mid + 0.06, 0.14],
                0.003,
            );
            let n = ((w - 0.2) / 0.05).floor() as usize;
            for i in 0..n {
                let x = x0 + 0.1 + (w - 0.2) * i as f64 / n as f64;
                b.rbox(
                    "frame",
                    [x, mid - 0.012, h],
                    [x + 0.022, mid + 0.012, h + screen_h],
                    0.0,
                );
            }
            b.rbox(
                "frame",
                [x0 + 0.1, mid - 0.015, h + screen_h],
                [x1 - 0.1, mid + 0.015, h + screen_h + 0.02],
                0.003,
            );
        }
        // Contemporary: white top on black steel frames, a cable tray and a
        // felt screen with round corners.
        4 => {
            top(b, "top", 0.003);
            let s = 0.04;
            for x in &ends {
                for (ya, yb) in [(0.05, mid - 0.05), (mid + 0.05, d - 0.05)] {
                    bar(
                        b,
                        "dark",
                        [*x, ya, 0.01],
                        [*x, (ya + yb) / 2.0, zt - 0.03],
                        s,
                        [1.0, 0.0, 0.0],
                    );
                    bar(
                        b,
                        "dark",
                        [*x, yb, 0.01],
                        [*x, (ya + yb) / 2.0, zt - 0.03],
                        s,
                        [1.0, 0.0, 0.0],
                    );
                }
                b.rbox(
                    "dark",
                    [x - s / 2.0, 0.04, zt - 0.03],
                    [x + s / 2.0, d - 0.04, zt],
                    0.0,
                );
                b.rbox(
                    "dark",
                    [x - s / 2.0, 0.03, 0.0],
                    [x + s / 2.0, 0.07, 0.012],
                    0.0,
                );
                b.rbox(
                    "dark",
                    [x - s / 2.0, d - 0.07, 0.0],
                    [x + s / 2.0, d - 0.03, 0.012],
                    0.0,
                );
            }
            b.rbox(
                "dark",
                [x0 + 0.1, mid - 0.12, zt - 0.14],
                [x1 - 0.1, mid + 0.12, zt - 0.12],
                0.0,
            );
            felt(
                b,
                "screen",
                [x0 + 0.06, mid - 0.02, h + 0.01],
                [x1 - 0.06, mid + 0.02, h + screen_h],
            );
        }
        // Rustic: thick planks on trestles, a linen screen in a wooden frame.
        5 => {
            top(b, "top", 0.008);
            for x in [ends[0] + 0.08, ends[ends.len() - 1] - 0.08] {
                for (ya, yb) in [(0.08, mid - 0.15), (d - 0.08, mid + 0.15)] {
                    bar(
                        b,
                        "frame",
                        [x, ya, 0.02],
                        [x, yb, zt - 0.06],
                        0.06,
                        [1.0, 0.0, 0.0],
                    );
                }
                b.rbox(
                    "frame",
                    [x - 0.03, 0.06, zt - 0.07],
                    [x + 0.03, d - 0.06, zt],
                    0.006,
                );
                b.rbox(
                    "frame",
                    [x - 0.04, 0.03, 0.0],
                    [x + 0.04, d - 0.03, 0.05],
                    0.006,
                );
            }
            b.rbox(
                "frame",
                [ends[0] + 0.08, mid - 0.04, 0.25],
                [ends[ends.len() - 1] - 0.08, mid + 0.04, 0.32],
                0.006,
            );
            for x in [x0 + 0.08, x1 - 0.12] {
                b.rbox(
                    "frame",
                    [x, mid - 0.02, h],
                    [x + 0.04, mid + 0.02, h + screen_h],
                    0.004,
                );
            }
            b.rbox(
                "frame",
                [x0 + 0.08, mid - 0.02, h + screen_h - 0.04],
                [x1 - 0.08, mid + 0.02, h + screen_h],
                0.004,
            );
            b.pillow(
                "weave",
                [x0 + 0.12, mid - 0.012, h],
                [x1 - 0.12, mid + 0.012, h + screen_h - 0.04],
                0.006,
                0.004,
            );
        }
        // Industrial: wood on a frame of black square tube, a screen of steel
        // mesh in a black frame.
        6 => {
            top(b, "top", 0.004);
            let s = 0.04;
            for x in &ends {
                for y in [0.04, mid - 0.06, mid + 0.06 - s, d - 0.04 - s] {
                    b.rbox("dark", [x - s / 2.0, y, 0.0], [x + s / 2.0, y + s, zt], 0.0);
                }
                b.rbox(
                    "dark",
                    [x - s / 2.0, 0.04, zt - s],
                    [x + s / 2.0, d - 0.04, zt],
                    0.0,
                );
                b.rbox(
                    "dark",
                    [x - s / 2.0, 0.04, 0.12],
                    [x + s / 2.0, d - 0.04, 0.12 + s],
                    0.0,
                );
            }
            for x in [x0 + 0.06, x1 - 0.08] {
                b.rbox(
                    "dark",
                    [x, mid - 0.01, h],
                    [x + 0.02, mid + 0.01, h + screen_h],
                    0.0,
                );
            }
            b.rbox(
                "dark",
                [x0 + 0.06, mid - 0.01, h + screen_h - 0.02],
                [x1 - 0.06, mid + 0.01, h + screen_h],
                0.0,
            );
            let n = ((w - 0.16) / 0.08).floor() as usize;
            for i in 1..n {
                let x = x0 + 0.08 + (w - 0.16) * i as f64 / n as f64;
                b.rbox(
                    "metal",
                    [x - 0.002, mid - 0.002, h],
                    [x + 0.002, mid + 0.002, h + screen_h - 0.02],
                    0.0,
                );
            }
            for k in 1..5 {
                let z = h + (screen_h - 0.02) * k as f64 / 5.0;
                b.rbox(
                    "metal",
                    [x0 + 0.08, mid - 0.002, z - 0.002],
                    [x1 - 0.08, mid + 0.002, z + 0.002],
                    0.0,
                );
            }
        }
        // Classic: a partners' desk, leather inlaid in walnut, moulded
        // aprons with drawers, fluted legs on brass, a brass gallery.
        _ => {
            b.rbox("top", [x0, 0.0, zt], [x1, d, h - 0.004], 0.006);
            b.rbox(
                "accent",
                [x0 + 0.05, 0.05, h - 0.004],
                [x1 - 0.05, d - 0.05, h],
                0.0,
            );
            let ap = zt - 0.1;
            b.rbox(
                "frame",
                [x0 + 0.05, 0.05, ap],
                [x1 - 0.05, d - 0.05, zt],
                0.004,
            );
            for i in 0..places {
                let a = x0 + 0.1 + (w - 0.2) * i as f64 / places as f64;
                let e = x0 + 0.1 + (w - 0.2) * (i + 1) as f64 / places as f64;
                let c = (a + e) / 2.0;
                front(
                    b,
                    Panel::Moulded,
                    "frame",
                    c - 0.25,
                    c + 0.25,
                    ap + 0.01,
                    zt - 0.01,
                    d - 0.05,
                );
                let start = b.mark();
                front(
                    b,
                    Panel::Moulded,
                    "frame",
                    -c - 0.25,
                    -c + 0.25,
                    ap + 0.01,
                    zt - 0.01,
                    d - 0.05,
                );
                turn_back(b, start, d / 2.0);
            }
            for x in [x0 + 0.08, x1 - 0.08] {
                for y in [0.08, d - 0.08] {
                    let r = 0.025 * std::f64::consts::SQRT_2;
                    turned(
                        b,
                        "frame",
                        [x, y, 0.05],
                        &[
                            (0.0, 0.0),
                            (r * 0.65, 0.0),
                            (r, ap - 0.05),
                            (0.0, ap - 0.05),
                        ],
                        4,
                        FRAC_PI_4,
                    );
                    turned(
                        b,
                        "metal",
                        [x, y, 0.0],
                        &[(0.0, 0.0), (r * 0.66, 0.0), (r * 0.66, 0.05), (0.0, 0.05)],
                        4,
                        FRAC_PI_4,
                    );
                }
            }
            sweep(
                b,
                "metal",
                &[[x0 + 0.1, mid, h + 0.12], [x1 - 0.1, mid, h + 0.12]],
                |_| 0.008,
                10,
                Cap::Round(1.0),
                Cap::Round(1.0),
            );
            let n = ((w - 0.2) / 0.18).round() as usize;
            for i in 0..=n {
                let x = x0 + 0.1 + (w - 0.2) * i as f64 / n as f64;
                rod(b, "metal", [x, mid, h], [x, mid, h + 0.12], 0.005);
            }
        }
    }
    // A monitor and a keyboard at every place, both sides.
    for i in 0..places {
        let x = x0 + w * (i as f64 + 0.5) / places as f64;
        monitor(b, x, mid - 0.12, h, 0.55, true);
        monitor(b, x, mid + 0.12, h, 0.55, false);
    }
}

// --------------------------------------------------------- meeting table

/// Meeting tables of the collections.
pub fn meeting_table(b: &mut Builder, p: &Params) {
    let ix = look(p);
    let w = p.get("width", 2.4).clamp(1.2, 6.0);
    let d = p.get("depth", 1.1).clamp(0.8, 1.6);
    let h = p.get("height", 0.74).clamp(0.68, 0.8);
    let (x0, x1) = (-w / 2.0, w / 2.0);
    let c = [0.0, d / 2.0];
    let t = 0.035;
    let zt = h - t;
    // The power box set in the middle of the top.
    let power = |b: &mut Builder| {
        b.rbox(
            "dark",
            [-0.16, c[1] - 0.06, h],
            [0.16, c[1] + 0.06, h + 0.004],
            0.002,
        );
    };
    match ix {
        // Nordic: an oak top with round corners on four splayed legs.
        0 => {
            b.rbox("top", [x0, 0.0, zt], [x1, d, h], 0.012);
            b.rbox(
                "frame",
                [x0 + 0.12, 0.12, zt - 0.07],
                [x1 - 0.12, d - 0.12, zt],
                0.006,
            );
            for (x, y, dx, dy) in [
                (x0 + 0.15, 0.15, -0.5, -0.5),
                (x1 - 0.15, 0.15, 0.5, -0.5),
                (x1 - 0.15, d - 0.15, 0.5, 0.5),
                (x0 + 0.15, d - 0.15, -0.5, 0.5),
            ] {
                tapered_leg(b, "frame", [x, y], zt, [dx, dy], 0.06, 0.026);
            }
        }
        // Mediterranean: planks of whitewashed wood on two turned trestles.
        1 => {
            let n = (d / 0.18).round().max(3.0) as usize;
            for i in 0..n {
                let y0 = d * i as f64 / n as f64;
                b.rbox(
                    "top",
                    [x0, y0 + 0.002, zt],
                    [x1, y0 + d / n as f64 - 0.002, h],
                    0.004,
                );
            }
            for x in [x0 + 0.25, x1 - 0.25] {
                for y in [0.15, d - 0.15] {
                    turned_leg(b, x, y, zt - 0.06, 0.035);
                }
                b.rbox(
                    "frame",
                    [x - 0.04, 0.08, zt - 0.08],
                    [x + 0.04, d - 0.08, zt],
                    0.006,
                );
                b.rbox(
                    "frame",
                    [x - 0.03, 0.12, 0.12],
                    [x + 0.03, d - 0.12, 0.17],
                    0.006,
                );
            }
            b.rbox(
                "frame",
                [x0 + 0.25, c[1] - 0.03, 0.13],
                [x1 - 0.25, c[1] + 0.03, 0.17],
                0.006,
            );
        }
        // Fifties: a boat-shaped teak top on two pedestals with brass feet.
        2 => {
            round_top(b, "top", c, w / 2.0, d / 2.0, h, t);
            for x in [x0 + w * 0.25, x1 - w * 0.25] {
                b.rbox(
                    "frame",
                    [x - 0.05, c[1] - 0.2, 0.06],
                    [x + 0.05, c[1] + 0.2, zt],
                    0.02,
                );
                for y in [c[1] - 0.24, c[1] + 0.24] {
                    b.ellipsoid("metal", [x, y, 0.03], [0.04, 0.03, 0.03]);
                }
                b.rbox(
                    "frame",
                    [x - 0.06, c[1] - 0.28, 0.05],
                    [x + 0.06, c[1] + 0.28, 0.08],
                    0.015,
                );
            }
        }
        // Japandi: a thick oak slab on two solid blocks.
        3 => {
            b.rbox("top", [x0, 0.0, zt - 0.015], [x1, d, h], 0.004);
            for x in [x0 + 0.3, x1 - 0.42] {
                b.rbox(
                    "frame",
                    [x, c[1] - d * 0.3, 0.0],
                    [x + 0.12, c[1] + d * 0.3, zt - 0.015],
                    0.004,
                );
            }
        }
        // Contemporary: a thin top with soft corners on a steel spine.
        4 => {
            b.rbox("top", [x0, 0.0, h - 0.02], [x1, d, h], 0.004);
            for x in [x0 + 0.35, x1 - 0.35] {
                b.rbox(
                    "dark",
                    [x - 0.25, c[1] - 0.04, 0.0],
                    [x + 0.25, c[1] + 0.04, 0.012],
                    0.0,
                );
                b.rbox(
                    "dark",
                    [x - 0.03, c[1] - 0.04, 0.012],
                    [x + 0.03, c[1] + 0.04, h - 0.02],
                    0.0,
                );
            }
            b.rbox(
                "dark",
                [x0 + 0.35, c[1] - 0.06, h - 0.07],
                [x1 - 0.35, c[1] + 0.06, h - 0.02],
                0.0,
            );
        }
        // Rustic: a long table of thick planks on square legs with
        // stretchers.
        5 => {
            b.rbox("top", [x0, 0.0, zt - 0.02], [x1, d, h], 0.008);
            for (x, y) in [
                (x0 + 0.12, 0.12),
                (x1 - 0.12, 0.12),
                (x1 - 0.12, d - 0.12),
                (x0 + 0.12, d - 0.12),
            ] {
                bar(
                    b,
                    "frame",
                    [x, y, 0.0],
                    [x, y, zt - 0.02],
                    0.08,
                    [1.0, 0.0, 0.0],
                );
            }
            for x in [x0 + 0.12, x1 - 0.12] {
                b.rbox(
                    "frame",
                    [x - 0.03, 0.12, 0.15],
                    [x + 0.03, d - 0.12, 0.21],
                    0.006,
                );
            }
            b.rbox(
                "frame",
                [x0 + 0.12, c[1] - 0.03, 0.15],
                [x1 - 0.12, c[1] + 0.03, 0.21],
                0.006,
            );
            b.rbox(
                "frame",
                [x0 + 0.12, 0.09, zt - 0.12],
                [x1 - 0.12, d - 0.09, zt - 0.02],
                0.006,
            );
        }
        // Industrial: wood on a black steel frame with a cross brace.
        6 => {
            b.rbox("top", [x0, 0.0, zt], [x1, d, h], 0.004);
            let s = 0.05;
            for (x, y) in [
                (x0 + 0.06, 0.06),
                (x1 - 0.06 - s, 0.06),
                (x1 - 0.06 - s, d - 0.06 - s),
                (x0 + 0.06, d - 0.06 - s),
            ] {
                b.rbox("dark", [x, y, 0.0], [x + s, y + s, zt], 0.0);
            }
            for y in [0.06, d - 0.06 - s] {
                b.rbox(
                    "dark",
                    [x0 + 0.06, y, zt - 0.05],
                    [x1 - 0.06, y + s, zt],
                    0.0,
                );
            }
            for x in [x0 + 0.06, x1 - 0.06 - s] {
                b.rbox("dark", [x, 0.06, zt - 0.05], [x + s, d - 0.06, zt], 0.0);
            }
            for (a, e) in [
                ([x0 + 0.1, c[1], 0.1], [x1 - 0.1, c[1], zt - 0.06]),
                ([x0 + 0.1, c[1], zt - 0.06], [x1 - 0.1, c[1], 0.1]),
            ] {
                bar(b, "dark", a, e, 0.03, [0.0, 1.0, 0.0]);
            }
        }
        // Classic: an oval walnut top on two pedestals carved as urns, with
        // splayed feet.
        _ => {
            round_top(b, "top", c, w / 2.0, d / 2.0, h, t);
            round_top(
                b,
                "accent",
                c,
                w / 2.0 - 0.06,
                d / 2.0 - 0.06,
                h + 0.002,
                0.004,
            );
            for x in [x0 + w * 0.27, x1 - w * 0.27] {
                turned(
                    b,
                    "frame",
                    [x, c[1], 0.12],
                    &[
                        (0.0, 0.0),
                        (0.07, 0.0),
                        (0.09, 0.05),
                        (0.06, 0.12),
                        (0.045, 0.3),
                        (0.07, 0.38),
                        (0.05, 0.45),
                        (0.08, zt - 0.12),
                        (0.0, zt - 0.12),
                    ],
                    24,
                    0.0,
                );
                for k in 0..4 {
                    let a = FRAC_PI_4 + PI / 2.0 * k as f64;
                    let (ca, sa) = (a.cos(), a.sin());
                    let at = |s: f64, z: f64| [x + ca * s, c[1] + sa * s, z];
                    let path = super::shape::curve(
                        &[
                            at(0.04, 0.18),
                            at(0.15, 0.12),
                            at(0.3, 0.05),
                            at(0.34, 0.035),
                        ],
                        5,
                    );
                    sweep(
                        b,
                        "frame",
                        &path,
                        |t| 0.035 - 0.015 * t,
                        10,
                        Cap::Round(1.0),
                        Cap::Round(1.0),
                    );
                    let f = at(0.34, 0.0);
                    b.cyl("metal", f, [f[0], f[1], 0.035], 0.022);
                }
            }
        }
    }
    power(b);
}

// --------------------------------------------------------------- reception

/// Reception desks of the collections.
pub fn reception(b: &mut Builder, p: &Params) {
    let ix = look(p);
    let w = p.get("width", 2.4).clamp(1.6, 6.0);
    let d = p.get("depth", 0.85).clamp(0.7, 1.1);
    let h = p.get("height", 1.1).clamp(1.0, 1.2);
    let (x0, x1) = (-w / 2.0, w / 2.0);
    // The front at `ds`, the visitors' ledge over it, the lowered end at
    // `xl`.
    let ledge = 0.3;
    let ds = d - ledge + 0.08;
    let low = 0.8;
    let xl = x1 - 0.9;
    let face: &'static str = match ix {
        1 | 5 => "masonry",
        2 => "accent",
        4 => "mat",
        _ => "frame",
    };
    // The staff side: a work surface at 74 cm on cupboards and drawers.
    b.rbox(
        "dark",
        [x0 + 0.05, 0.05, 0.0],
        [x1 - 0.05, ds - 0.1, 0.08],
        0.0,
    );
    b.rbox(
        "carcass",
        [x0 + 0.03, 0.03, 0.08],
        [x1 - 0.03, ds - 0.06, 0.72],
        0.0,
    );
    b.rbox(
        "top",
        [x0 + 0.03, -0.02, 0.72],
        [x1 - 0.03, ds - 0.06, 0.745],
        0.003,
    );
    let start = b.mark();
    drawers(
        b,
        Panel::Flat,
        "carcass",
        -(x0 + 0.5),
        -(x0 + 0.04),
        0.09,
        0.71,
        0.03,
        3,
        Pull::Bar(0.12),
    );
    turn_back(b, start, 0.03);
    // The front and the ends, up to the ledge: high on most of it, low at
    // the end for a wheelchair.
    let front_wall = |b: &mut Builder, role: &'static str, a: f64, e: f64, top: f64| {
        b.rbox(role, [a, ds - 0.06, 0.0], [e, ds, top], 0.004);
    };
    let th = 0.04;
    front_wall(b, face, x0, xl, h - th);
    front_wall(b, face, xl, x1, low - th);
    for (a, e, top) in [(x0, x0 + 0.03, h - th), (x1 - 0.03, x1, low - th)] {
        b.rbox(face, [a, 0.0, 0.0], [e, ds, top], 0.003);
    }
    // The visitors' ledges.
    let ledge_role = if ix == 1 { "accent" } else { "top" };
    b.rbox(
        ledge_role,
        [x0 - 0.02, ds - 0.12, h - th],
        [xl + 0.01, d, h],
        0.008,
    );
    b.rbox(
        ledge_role,
        [xl - 0.01, ds - 0.12, low - th],
        [x1 + 0.02, d, low],
        0.008,
    );
    b.rbox(
        face,
        [xl - 0.02, ds - 0.06, low],
        [xl + 0.02, ds, h - th],
        0.003,
    );
    // The collection's face.
    match ix {
        0 => front(
            b,
            Panel::Flutes,
            "frame",
            x0 + 0.03,
            xl - 0.02,
            0.06,
            h - th - 0.01,
            ds,
        ),
        1 => {
            let tile = 0.15;
            let k = ((xl - x0 - 0.06) / tile).floor() as usize;
            for i in 0..k {
                let role = if i % 2 == 0 { "accent" } else { "masonry" };
                let xa = x0 + 0.03 + tile * i as f64;
                b.rbox(
                    role,
                    [xa + 0.003, ds, h - th - 0.2],
                    [xa + tile - 0.003, ds + 0.008, h - th - 0.05],
                    0.0,
                );
            }
        }
        2 => {
            for z in [0.35, 0.6] {
                b.rbox(
                    "metal",
                    [x0 + 0.03, ds, z - 0.012],
                    [x1 - 0.03, ds + 0.006, z + 0.012],
                    0.003,
                );
            }
            b.rbox("metal", [x0, ds - 0.06, 0.0], [x1, ds + 0.004, 0.08], 0.003);
        }
        3 => front(
            b,
            Panel::Ribs,
            "frame",
            x0 + 0.03,
            x1 - 0.03,
            0.06,
            low - th - 0.01,
            ds,
        ),
        4 => {
            b.rbox(
                "light",
                [x0 + 0.05, ds, h - th - 0.008],
                [xl - 0.05, ds + 0.004, h - th],
                0.0,
            );
            b.rbox(
                "light",
                [x0 + 0.05, ds, 0.03],
                [x1 - 0.05, ds + 0.004, 0.036],
                0.0,
            );
        }
        5 => {
            let mut rnd = Rng(31);
            let course = 0.16;
            for r in 0..((h - th) / course).floor() as usize {
                let (za, ze) = (course * r as f64, course * (r + 1) as f64);
                let end = if ze > low - th { xl } else { x1 };
                let mut x = x0 + if r % 2 == 0 { 0.0 } else { -0.12 };
                while x < end {
                    let l = rnd.range(0.22, 0.42);
                    let (xa, xe) = (x.max(x0) + 0.008, (x + l).min(end) - 0.008);
                    if xe - xa > 0.05 {
                        stone(
                            b,
                            "masonry",
                            [xa, ds - 0.01, za + 0.008],
                            [xe, ds + rnd.range(0.004, 0.012), ze - 0.008],
                            0.18,
                        );
                    }
                    x += l;
                }
            }
        }
        6 => {
            let posts = ((w / 1.2).ceil() as usize + 1).max(2);
            for i in 0..posts {
                let x = x0 + 0.03 + (w - 0.06) * i as f64 / (posts - 1) as f64;
                let top = if x > xl { low - th } else { h - th };
                b.rbox(
                    "dark",
                    [x - 0.03, ds, 0.0],
                    [x + 0.03, ds + 0.008, top],
                    0.0,
                );
            }
            b.rbox("dark", [x0, ds, 0.0], [x1, ds + 0.006, 0.1], 0.0);
        }
        _ => {
            let bays = ((xl - x0) / 0.6).round().max(1.0) as usize;
            let bw = (xl - x0 - 0.06) / bays as f64;
            for i in 0..bays {
                let a = x0 + 0.03 + bw * i as f64;
                front(
                    b,
                    Panel::Raised,
                    "frame",
                    a + 0.05,
                    a + bw - 0.05,
                    0.16,
                    h - th - 0.12,
                    ds,
                );
            }
            b.rbox("frame", [x0, ds - 0.06, 0.0], [x1, ds + 0.02, 0.1], 0.004);
            b.rbox(
                "frame",
                [x0, ds - 0.06, h - th - 0.07],
                [xl, ds + 0.02, h - th],
                0.004,
            );
        }
    }
    // A monitor on the work surface.
    monitor(b, x0 + 0.8, ds - 0.35, 0.745, 0.55, true);
}

// ------------------------------------------------------------- cabinet

/// Office cabinets of the collections: doors below, box files on open
/// shelves above.
pub fn office_storage(b: &mut Builder, p: &Params) {
    let ix = look(p);
    let w = p.get("width", 1.6).clamp(0.8, 4.0);
    let d = p.get("depth", 0.45).clamp(0.35, 0.6);
    let h = p.get("height", 1.2).clamp(0.75, 2.2);
    let (x0, x1) = (-w / 2.0, w / 2.0);
    let (panel, pull) = match ix {
        0 => (Panel::Flat, Pull::Slot),
        1 => (Panel::Shaker, Pull::Knob),
        2 => (Panel::Flat, Pull::Brass),
        3 => (Panel::Ribs, Pull::Tab),
        4 => (Panel::Flat, Pull::Slot),
        5 => (Panel::Planks, Pull::Ring),
        6 => (Panel::Mesh, Pull::Bar(0.12)),
        _ => (Panel::Raised, Pull::Brass),
    };
    let base: f64 = match ix {
        2 => 0.18,
        0 | 3 | 4 | 6 => 0.08,
        _ => 0.1,
    };
    let t = 0.022;
    let low = (base + 0.62).min(h - 0.3);
    // The carcass: sides, top, back and a shelf.
    let carcass = |b: &mut Builder, role: &'static str| {
        b.rbox(role, [x0, 0.0, base], [x0 + t, d, h], 0.002);
        b.rbox(role, [x1 - t, 0.0, base], [x1, d, h], 0.002);
        b.rbox(role, [x0, 0.0, h - t], [x1, d, h], 0.004);
        b.rbox(role, [x0 + t, 0.0, base], [x1 - t, 0.012, h - t], 0.0);
        b.rbox(role, [x0 + t, 0.0, base], [x1 - t, d - T, base + t], 0.0);
        b.rbox(role, [x0 + t, 0.0, low], [x1 - t, d, low + t], 0.0);
    };
    let role = if ix == 6 { "dark" } else { "frame" };
    carcass(b, role);
    let n = (w / 0.45).round().max(1.0) as usize;
    doors(
        b,
        panel,
        "front",
        x0 + t,
        x1 - t,
        base + t,
        low,
        d - T,
        n,
        pull,
        low - 0.08,
    );
    // Shelves of box files above.
    let shelves = ((h - t - low - t) / 0.36).floor().max(1.0) as usize;
    let step = (h - t - low - t) / shelves as f64;
    for i in 0..shelves {
        let z = low + t + step * i as f64;
        if i > 0 {
            b.rbox(role, [x0 + t, 0.012, z - t], [x1 - t, d, z], 0.0);
        }
        binders(
            b,
            x0 + t,
            x1 - t,
            0.02,
            d - 0.06,
            z,
            step - 0.03,
            7 + i as u32,
        );
    }
    // The base, in the collection's way.
    match ix {
        0 => {
            for (x, y, dx, dy) in [
                (x0 + 0.07, 0.07, -0.5, -0.5),
                (x1 - 0.07, 0.07, 0.5, -0.5),
                (x1 - 0.07, d - 0.07, 0.5, 0.5),
                (x0 + 0.07, d - 0.07, -0.5, 0.5),
            ] {
                tapered_leg(b, "frame", [x, y], base, [dx, dy], 0.02, 0.018);
            }
        }
        2 => {
            for (x, y, dx, dy) in [
                (x0 + 0.08, 0.08, -0.5, -0.5),
                (x1 - 0.08, 0.08, 0.5, -0.5),
                (x1 - 0.08, d - 0.08, 0.5, 0.5),
                (x0 + 0.08, d - 0.08, -0.5, 0.5),
            ] {
                compass_leg(b, [x, y], base, [dx, dy], 0.04);
            }
        }
        3 | 4 => b.rbox(
            "dark",
            [x0 + 0.03, 0.03, 0.0],
            [x1 - 0.03, d - 0.05, base],
            0.0,
        ),
        6 => {
            for (x, y) in [
                (x0, 0.0),
                (x1 - 0.03, 0.0),
                (x1 - 0.03, d - 0.03),
                (x0, d - 0.03),
            ] {
                b.rbox("dark", [x, y, 0.0], [x + 0.03, y + 0.03, base], 0.0);
            }
            disc(b, "dark", [x0 + 0.02, d - 0.02], 0.025, 0.0, 0.01);
        }
        7 => {
            b.rbox(
                "frame",
                [x0 - 0.02, 0.0, 0.03],
                [x1 + 0.02, d + 0.02, base],
                0.006,
            );
            for (x, y) in [
                (x0 + 0.05, 0.05),
                (x1 - 0.05, 0.05),
                (x1 - 0.05, d - 0.03),
                (x0 + 0.05, d - 0.03),
            ] {
                toupie(b, [x, y], 0.03);
            }
            b.rbox(
                "frame",
                [x0 - 0.025, 0.0, h],
                [x1 + 0.025, d + 0.025, h + 0.03],
                0.008,
            );
        }
        _ => b.rbox("frame", [x0, 0.0, 0.0], [x1, d - 0.02, base], 0.004),
    }
}

// ------------------------------------------------------------------ screen

/// Free-standing acoustic screens of the collections.
pub fn acoustic_screen(b: &mut Builder, p: &Params) {
    let ix = look(p);
    let w = p.get("width", 1.2).clamp(0.6, 2.4);
    let h = p.get("height", 1.5).clamp(1.0, 2.0);
    let (x0, x1) = (-w / 2.0, w / 2.0);
    let feet = |b: &mut Builder, role: &'static str, at: &[f64], len: f64, z: f64| {
        for x in at {
            b.rbox(
                role,
                [x - 0.025, -len / 2.0, 0.0],
                [x + 0.025, len / 2.0, z],
                0.008,
            );
        }
    };
    match ix {
        // Nordic: felt in a thin oak frame on oak feet.
        0 => {
            feet(b, "frame", &[x0 + 0.1, x1 - 0.1], 0.4, 0.03);
            for x in [x0, x1 - 0.03] {
                b.rbox("frame", [x, -0.02, 0.03], [x + 0.03, 0.02, h], 0.004);
            }
            b.rbox("frame", [x0, -0.02, h - 0.03], [x1, 0.02, h], 0.004);
            b.rbox("frame", [x0, -0.02, 0.06], [x1, 0.02, 0.09], 0.004);
            felt(
                b,
                "screen",
                [x0 + 0.03, -0.018, 0.09],
                [x1 - 0.03, 0.018, h - 0.03],
            );
        }
        // Mediterranean: three leaves of cane in whitewashed frames.
        1 => three_leaves(b, "weave", x0, x1, h, 1),
        // Fifties: a coloured panel with rounded corners on brass feet.
        2 => {
            let mut s = Soft::new([x0, -0.02, 0.12], [x1, 0.02, h], 0.08);
            s.crown = 0.004;
            s.step = 0.2;
            s.add(b, "accent");
            for x in [x0 + 0.15, x1 - 0.15] {
                rod(b, "metal", [x, 0.0, 0.0], [x, 0.0, 0.14], 0.012);
                b.rbox(
                    "metal",
                    [x - 0.02, -0.18, 0.0],
                    [x + 0.02, 0.18, 0.015],
                    0.006,
                );
            }
        }
        // Japandi: upright slats of smoked oak between two rails.
        3 => {
            feet(b, "frame", &[x0 + 0.08, x1 - 0.08], 0.36, 0.04);
            for z in [0.04, h - 0.04] {
                b.rbox("frame", [x0, -0.02, z], [x1, 0.02, z + 0.04], 0.004);
            }
            let n = ((w - 0.02) / 0.045).floor() as usize;
            for i in 0..n {
                let x = x0 + 0.01 + (w - 0.02) * i as f64 / n as f64;
                b.rbox(
                    "frame",
                    [x, -0.012, 0.08],
                    [x + 0.025, 0.012, h - 0.04],
                    0.0,
                );
            }
        }
        // Contemporary: a thick panel of grey felt with round corners on
        // black steel feet.
        4 => {
            felt(b, "screen", [x0, -0.03, 0.04], [x1, 0.03, h]);
            for x in [x0 + 0.15, x1 - 0.15] {
                b.rbox("dark", [x - 0.02, -0.2, 0.0], [x + 0.02, 0.2, 0.01], 0.0);
                b.rbox(
                    "dark",
                    [x - 0.02, -0.034, 0.01],
                    [x + 0.02, 0.034, 0.12],
                    0.0,
                );
            }
        }
        // Rustic: three leaves of linen in pine frames.
        5 => three_leaves(b, "weave", x0, x1, h, 5),
        // Industrial: a black steel frame with a mesh and a felt panel.
        6 => {
            let s = 0.03;
            feet(b, "dark", &[x0 + s / 2.0, x1 - s / 2.0], 0.4, 0.02);
            for x in [x0, x1 - s] {
                b.rbox("dark", [x, -s / 2.0, 0.02], [x + s, s / 2.0, h], 0.0);
            }
            for z in [0.08, h - s] {
                b.rbox("dark", [x0, -s / 2.0, z], [x1, s / 2.0, z + s], 0.0);
            }
            felt(
                b,
                "screen",
                [x0 + s, -0.01, 0.11],
                [x1 - s, 0.012, h * 0.55],
            );
            let n = ((w - 2.0 * s) / 0.06).floor() as usize;
            for i in 1..n {
                let x = x0 + s + (w - 2.0 * s) * i as f64 / n as f64;
                b.rbox(
                    "metal",
                    [x - 0.002, -0.002, h * 0.55],
                    [x + 0.002, 0.002, h - s],
                    0.0,
                );
            }
            for k in 1..6 {
                let z = h * 0.55 + (h * 0.45 - s) * k as f64 / 6.0;
                b.rbox(
                    "metal",
                    [x0 + s, -0.002, z - 0.002],
                    [x1 - s, 0.002, z + 0.002],
                    0.0,
                );
            }
        }
        // Classic: a three-leaved folding screen of walnut and fabric with
        // arched heads.
        _ => three_leaves(b, "screen", x0, x1, h, 7),
    }
}

/// A folding screen of three leaves from `x0` to `x1`, `h` high, its
/// panels of `role`; the outer leaves turned a little forward. Arched heads
/// for the classic one (`ix` 7).
fn three_leaves(b: &mut Builder, role: &'static str, x0: f64, x1: f64, h: f64, ix: usize) {
    let lw = (x1 - x0) / 3.0;
    let fr = if ix == 7 { 0.035 } else { 0.03 };
    for k in 0..3 {
        let start = b.mark();
        let (a, e) = (-lw / 2.0, lw / 2.0);
        let top = if ix == 7 && k == 1 {
            h
        } else if ix == 7 {
            h - 0.08
        } else {
            h
        };
        for x in [a, e - fr] {
            b.rbox("frame", [x, -0.012, 0.0], [x + fr, 0.012, top], 0.004);
        }
        for z in [0.12, top - fr] {
            b.rbox("frame", [a, -0.012, z], [e, 0.012, z + fr], 0.004);
        }
        b.pillow(
            role,
            [a + fr, -0.008, 0.12 + fr],
            [e - fr, 0.008, top - fr],
            0.004,
            0.004,
        );
        if ix == 7 {
            b.ellipsoid("metal", [0.0, 0.0, top + 0.02], [0.02, 0.012, 0.02]);
            for x in [a + fr / 2.0, e - fr / 2.0] {
                b.cyl("metal", [x, 0.0, 0.0], [x, 0.0, 0.02], 0.016);
            }
        }
        // Turn the outer leaves about their inner edge.
        let (pivot, turn): (f64, f64) = match k {
            0 => (x0 + lw, 0.35),
            2 => (x1 - lw, -0.35),
            _ => (0.0, 0.0),
        };
        let cx = x0 + lw * (k as f64 + 0.5);
        let (s, c) = turn.sin_cos();
        b.transform_from(start, |q| {
            let (x, y) = (q[0] + cx - pivot, q[1]);
            [pivot + x * c - y * s, x * s + y * c, q[2]]
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fine::shape::signed_volume;

    #[test]
    fn the_offices_of_the_eight_collections() {
        type Make = fn(&mut Builder, &Params);
        let pieces: [(&str, Make); 5] = [
            ("puestos", bench_desk),
            ("reuniones", meeting_table),
            ("recepción", reception),
            ("armario", office_storage),
            ("mampara", acoustic_screen),
        ];
        for c in 0..8 {
            for (name, make) in pieces {
                let mut p = Params::default();
                p.set("style", (1 + c) as f64);
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
