//! The workshop's chairs and stools, one for each collection (paquete 1b).
//!
//! Chairs: `width`, `depth`, `seat` (height of the seat), `height` (of
//! the back); front towards `+y`, back at `y = 0`, centred on `x = 0`.
//! Stools: `width` (the seat across), `seat` (its height); centred on
//! `x = 0`, `y = width / 2`.
//!
//! Roles: `frame` wood, `metal` steel or brass, `mat` upholstery,
//! `weave` rush or cord, `cushion` a loose pad.

use super::armchairs::wrap;
use super::shape::{curve, sweep, turned, Cap};
use super::sofas::{compass_leg, upholstered};
use super::soft::Soft;
use crate::build::{Builder, P3};
use crate::Params;
use std::f64::consts::{FRAC_PI_4, PI, TAU};

struct Chair {
    w: f64,
    d: f64,
    seat: f64,
    h: f64,
}

impl Chair {
    fn of(p: &Params) -> Self {
        let seat = p.get("seat", 0.45).clamp(0.38, 0.5);
        Chair {
            w: p.get("width", 0.46).clamp(0.38, 0.65),
            d: p.get("depth", 0.52).clamp(0.4, 0.65),
            seat,
            h: p.get("height", 0.82).max(seat + 0.25),
        }
    }
}

/// A straight round rod from `a` to `c`.
fn rod(b: &mut Builder, role: &'static str, a: P3, c: P3, r: f64) {
    sweep(b, role, &[a, c], |_| r, 12, Cap::Flat, Cap::Flat);
}

/// A turned leg from the floor at `at` up to `top`: thinner at the foot,
/// with a ring a third of the way up.
fn turned_leg(b: &mut Builder, at: [f64; 2], top: f64, r: f64) {
    turned(
        b,
        "frame",
        [at[0], at[1], 0.0],
        &[
            (0.0, 0.0),
            (r * 0.7, 0.0),
            (r * 0.75, top * 0.3),
            (r * 1.15, top * 0.34),
            (r * 0.85, top * 0.38),
            (r, top * 0.9),
            (r, top),
            (0.0, top),
        ],
        14,
        0.0,
    );
}

/// A board bent round the vertical through `c` (`r` metres): its length
/// along `x` becomes the arc; for a curved chair back or rail.
#[allow(clippy::too_many_arguments)]
fn bent_board(
    b: &mut Builder,
    role: &'static str,
    c: [f64; 2],
    r: f64,
    len: f64,
    thick: f64,
    z0: f64,
    z1: f64,
    round: f64,
) {
    let start = b.mark();
    b.rbox(role, [-len / 2.0, 0.0, z0], [len / 2.0, thick, z1], round);
    wrap(b, start, c, r, 10.0, z0, 0.0);
}

/// Splays what was made since `start` (a leg standing upright at `at`,
/// `top` high): its foot goes `splay` metres out along `dir`.
fn splay(b: &mut Builder, start: usize, top: f64, dir: [f64; 2], amount: f64) {
    let k = amount / top.max(1e-3);
    b.transform_from(start, |q| {
        let down = top - q[2];
        [q[0] + dir[0] * k * down, q[1] + dir[1] * k * down, q[2]]
    });
}

/// A chair of a collection.
pub fn chair(b: &mut Builder, p: &Params) {
    let c = Chair::of(p);
    let x0 = -c.w / 2.0;
    let (w, d, seat, h) = (c.w, c.d, c.seat, c.h);
    match p.style() {
        // Mediterranean: the Andalusian rush chair — turned posts, the back
        // ones tall with knobs, three curved back slats, turned stretchers,
        // a rush seat.
        2 => {
            let r = 0.017;
            let fy = d - 0.03;
            for x in [x0 + 0.03, -x0 - 0.03] {
                turned_leg(b, [x, fy], seat - 0.01, r);
                turned(
                    b,
                    "frame",
                    [x, 0.03, 0.0],
                    &[
                        (0.0, 0.0),
                        (r * 0.75, 0.0),
                        (r, h * 0.4),
                        (r * 0.9, h - 0.04),
                        (r * 1.3, h - 0.02),
                        (r * 0.8, h),
                        (r * 1.1, h + 0.02),
                        (0.0, h + 0.035),
                    ],
                    14,
                    0.0,
                );
            }
            for k in 0..3 {
                let z = seat + 0.08 + (h - seat - 0.14) * k as f64 / 2.0;
                bent_board(
                    b,
                    "frame",
                    [0.0, 0.9],
                    0.88,
                    w - 0.04,
                    0.014,
                    z,
                    z + 0.045,
                    0.004,
                );
            }
            for (z, ya, yb) in [(0.12, fy, fy), (0.2, fy, fy), (0.14, 0.03, 0.03)] {
                rod(b, "frame", [x0 + 0.03, ya, z], [-x0 - 0.03, yb, z], 0.009);
            }
            for x in [x0 + 0.03, -x0 - 0.03] {
                rod(b, "frame", [x, 0.03, 0.17], [x, fy, 0.17], 0.009);
            }
            let mut s = Soft::new(
                [x0 + 0.012, 0.012, seat - 0.035],
                [-x0 - 0.012, d - 0.01, seat],
                0.015,
            );
            s.crown = -0.006;
            s.wobble = 0.0012;
            s.seed = 51;
            s.add(b, "weave");
        }
        // Fifties: a small upholstered shell on compass legs.
        3 => {
            let foot = seat - 0.06;
            for (dx, dy) in [(-1.0f64, -1.0f64), (1.0, -1.0), (-1.0, 1.0), (1.0, 1.0)] {
                let l = (dx * dx + dy * dy).sqrt();
                compass_leg(
                    b,
                    [dx * (w / 2.0 - 0.07), d / 2.0 + dy * (d / 2.0 - 0.08)],
                    foot + 0.004,
                    [dx / l, dy / l],
                    0.035,
                );
            }
            let mut base = upholstered([x0 + 0.01, 0.06, foot], [-x0 - 0.01, d, seat], 0.03, 52);
            base.crown = 0.012;
            base.welt = 0.004;
            base.add(b, "mat");
            let r = w * 0.62;
            let start = b.mark();
            let mut shell = Soft::new([-r * 1.25, 0.0, foot], [r * 1.25, 0.04, h], 0.018);
            shell.crown = 0.004;
            shell.welt = 0.004;
            shell.step = 0.04;
            shell.seed = 53;
            shell.add(b, "mat");
            wrap(b, start, [0.0, r + 0.02], r, 1.25, foot, 0.45);
        }
        // Japandi: square softened legs, a curved back board, a woven cord
        // seat in a wooden frame.
        4 => {
            let leg = 0.032;
            for (x, y, top) in [
                (x0 + 0.02, d - 0.02 - leg, seat - 0.005),
                (-x0 - 0.02 - leg, d - 0.02 - leg, seat - 0.005),
                (x0 + 0.02, 0.03, h - 0.02),
                (-x0 - 0.02 - leg, 0.03, h - 0.02),
            ] {
                b.rbox("frame", [x, y, 0.0], [x + leg, y + leg, top], 0.008);
            }
            bent_board(
                b,
                "frame",
                [0.0, 0.85],
                0.83,
                w - 0.02,
                0.02,
                h - 0.14,
                h,
                0.008,
            );
            for (ya, yb) in [(d - 0.035, d - 0.035), (0.045, 0.045)] {
                b.rbox(
                    "frame",
                    [x0 + 0.02, ya - 0.012, seat - 0.05],
                    [-x0 - 0.02, yb + 0.012, seat - 0.015],
                    0.004,
                );
            }
            for x in [x0 + 0.02, -x0 - 0.02 - leg] {
                b.rbox(
                    "frame",
                    [x, 0.03, seat - 0.05],
                    [x + leg, d - 0.02, seat - 0.015],
                    0.004,
                );
                b.rbox(
                    "frame",
                    [x + 0.006, 0.05, 0.12],
                    [x + leg - 0.006, d - 0.05, 0.14],
                    0.0,
                );
            }
            let mut s = Soft::new(
                [x0 + 0.045, 0.05, seat - 0.03],
                [-x0 - 0.045, d - 0.05, seat],
                0.008,
            );
            s.crown = -0.004;
            s.seed = 54;
            s.add(b, "weave");
        }
        // Contemporary: a thin black steel frame, the seat and back
        // upholstered pads.
        5 => {
            let r = 0.009;
            for x in [x0 + 0.03, -x0 - 0.03] {
                let path = curve(
                    &[
                        [x, d - 0.03, 0.0],
                        [x, d - 0.03, seat - 0.03],
                        [x, 0.06, seat - 0.03],
                        [x, 0.03, seat + 0.05],
                        [x, 0.0, h - 0.02],
                    ],
                    6,
                );
                sweep(b, "metal", &path, |_| r, 10, Cap::Flat, Cap::Round(1.0));
                rod(b, "metal", [x, 0.06, 0.0], [x, 0.06, seat - 0.03], r);
            }
            rod(
                b,
                "metal",
                [x0 + 0.03, d - 0.03, seat - 0.03],
                [-x0 - 0.03, d - 0.03, seat - 0.03],
                r * 0.8,
            );
            let mut s = upholstered(
                [x0 + 0.01, 0.04, seat - 0.025],
                [-x0 - 0.01, d + 0.01, seat + 0.02],
                0.025,
                55,
            );
            s.crown = 0.012;
            s.welt = 0.004;
            s.add(b, "mat");
            // The back pad, fixed on the front of the posts and leaning
            // with them.
            let (z0, z1) = (seat + 0.05, h - 0.02);
            let post_y = |z: f64| 0.03 * (1.0 - ((z - z0) / (z1 - z0)).clamp(0.0, 1.0));
            let start = b.mark();
            let mut k = upholstered(
                [x0 + 0.01, r * 0.6, seat + 0.14],
                [-x0 - 0.01, r * 0.6 + 0.045, h],
                0.02,
                56,
            );
            k.crown = 0.01;
            k.add(b, "mat");
            b.transform_from(start, |q| [q[0], q[1] + post_y(q[2]), q[2]]);
        }
        // Rustic: solid square legs, a board seat, slatted back.
        6 => {
            let leg = 0.042;
            for (x, y, top) in [
                (x0 + 0.01, d - 0.01 - leg, seat - 0.03),
                (-x0 - 0.01 - leg, d - 0.01 - leg, seat - 0.03),
                (x0 + 0.01, 0.02, h),
                (-x0 - 0.01 - leg, 0.02, h),
            ] {
                b.rbox("frame", [x, y, 0.0], [x + leg, y + leg, top], 0.006);
            }
            b.rbox("frame", [x0, 0.0, seat - 0.035], [-x0, d, seat], 0.01);
            b.rbox(
                "frame",
                [x0 + 0.01 + leg, 0.025, h - 0.09],
                [-x0 - 0.01 - leg, 0.055, h - 0.01],
                0.008,
            );
            for k in 0..3 {
                let x = x0 + 0.01 + leg + (w - 0.02 - 2.0 * leg) * (k as f64 + 0.5) / 3.0;
                b.rbox(
                    "frame",
                    [x - 0.025, 0.03, seat - 0.005],
                    [x + 0.025, 0.05, h - 0.085],
                    0.0,
                );
            }
            for (z, ya) in [(0.12, d - 0.01 - leg / 2.0), (0.15, 0.02 + leg / 2.0)] {
                b.rbox(
                    "frame",
                    [x0 + 0.01 + leg, ya - 0.012, z],
                    [-x0 - 0.01 - leg, ya + 0.012, z + 0.04],
                    0.0,
                );
            }
        }
        // Industrial: pressed steel sheet — the seat and a curved back —
        // on splayed legs with a stretcher.
        7 => {
            let r = 0.011;
            for (x, y, dx, dy) in [
                (x0 + 0.04, d - 0.05, -1.0f64, 1.0f64),
                (-x0 - 0.04, d - 0.05, 1.0, 1.0),
                (x0 + 0.04, 0.06, -1.0, -1.0),
                (-x0 - 0.04, 0.06, 1.0, -1.0),
            ] {
                let s = 0.035;
                rod(
                    b,
                    "metal",
                    [x + dx * s, y + dy * s, 0.0],
                    [x, y, seat - 0.01],
                    r,
                );
            }
            {
                // The stretcher runs through the legs where they are at its
                // height (they lean out towards the foot).
                let z = 0.16;
                let out = 0.035 * (seat - 0.01 - z) / (seat - 0.01);
                let (xa, xb) = (x0 + 0.04 - out, -x0 - 0.04 + out);
                let (ya, yb) = (0.06 - out, d - 0.05 + out);
                let pts = [[xa, yb, z], [xb, yb, z], [xb, ya, z], [xa, ya, z]];
                super::soft::tube(b, "metal", &pts, true, 0.006, 8);
            }
            b.rbox("metal", [x0, 0.04, seat - 0.012], [-x0, d, seat], 0.008);
            b.rbox(
                "metal",
                [x0 + 0.004, d - 0.02, seat - 0.05],
                [-x0 - 0.004, d, seat - 0.005],
                0.004,
            );
            let start = b.mark();
            let r0 = w * 0.75;
            b.rbox(
                "metal",
                [-w * 0.5, 0.0, seat + 0.03],
                [w * 0.5, 0.004, h],
                0.01,
            );
            wrap(b, start, [0.0, r0 + 0.02], r0, 10.0, seat, 0.0);
            // Two straps from the seat up to the back, where its curve
            // has brought the sheet forward.
            for side in [-1.0, 1.0] {
                let a = (w * 0.5 - 0.035) / r0;
                let (x, y) = (side * r0 * a.sin(), r0 + 0.02 - r0 * a.cos());
                let start = b.mark();
                b.rbox(
                    "metal",
                    [-0.015, 0.002, seat - 0.01],
                    [0.015, 0.016, seat + 0.1],
                    0.004,
                );
                let (sa, ca) = (side * a).sin_cos();
                b.transform_from(start, |q| {
                    [x + q[0] * ca - q[1] * sa, y + q[0] * sa + q[1] * ca, q[2]]
                });
            }
        }
        // Classic: Louis XVI — turned tapered legs, a piped upholstered
        // seat, an oval medallion back in a moulded frame.
        8 => {
            for (x, y) in [
                (x0 + 0.03, d - 0.03),
                (-x0 - 0.03, d - 0.03),
                (x0 + 0.03, 0.03),
                (-x0 - 0.03, 0.03),
            ] {
                turned(
                    b,
                    "frame",
                    [x, y, 0.0],
                    &[
                        (0.0, 0.0),
                        (0.011, 0.0),
                        (0.013, 0.03),
                        (0.011, 0.04),
                        (0.017, seat - 0.12),
                        (0.022, seat - 0.11),
                        (0.018, seat - 0.1),
                        (0.022, seat - 0.05),
                        (0.0, seat - 0.05),
                    ],
                    16,
                    0.0,
                );
            }
            b.rbox(
                "frame",
                [x0 + 0.01, 0.01, seat - 0.11],
                [-x0 - 0.01, d - 0.01, seat - 0.05],
                0.008,
            );
            let mut s = upholstered([x0, 0.0, seat - 0.06], [-x0, d + 0.01, seat], 0.03, 57);
            s.crown = 0.018;
            s.welt = 0.005;
            s.add(b, "mat");
            // The oval medallion, held at its sides by the back posts,
            // which rise from the back legs and curve in to meet it, and
            // on a short turned spindle over the seat.
            let (cz, rx, rz) = (
                seat + 0.08 + (h - seat - 0.08) / 2.0,
                w / 2.0 - 0.04,
                (h - seat - 0.1) / 2.0,
            );
            for side in [-1.0, 1.0] {
                let path = curve(
                    &[
                        [side * (-x0 - 0.03), 0.03, seat - 0.02],
                        [side * (-x0 - 0.03), 0.025, seat + 0.06],
                        [side * (rx + 0.004), 0.02, cz - rz * 0.35],
                        [side * rx, 0.02, cz],
                    ],
                    8,
                );
                sweep(
                    b,
                    "frame",
                    &path,
                    |t| 0.015 - 0.003 * t,
                    10,
                    Cap::Flat,
                    Cap::Round(1.0),
                );
            }
            turned(
                b,
                "frame",
                [0.0, 0.02, seat - 0.01],
                &[
                    (0.0, 0.0),
                    (0.016, 0.0),
                    (0.01, 0.03),
                    (0.015, 0.06),
                    (0.011, cz - rz - seat + 0.02),
                    (0.0, cz - rz - seat + 0.02),
                ],
                12,
                0.0,
            );
            let ring: Vec<P3> = (0..40)
                .map(|k| {
                    let a = TAU * k as f64 / 40.0;
                    [rx * a.cos(), 0.02, cz + rz * a.sin()]
                })
                .collect();
            super::soft::tube(b, "frame", &ring, true, 0.014, 10);
            let start = b.mark();
            b.lathe(
                "mat",
                [0.0, 0.0, 0.0],
                &[
                    (0.0, -0.01),
                    (0.9, -0.01),
                    (1.0, 0.0),
                    (0.9, 0.012),
                    (0.0, 0.02),
                ],
                rx - 0.008,
                rz - 0.008,
            );
            // Stood up facing forward (a turn, not a mirror: x flips too).
            b.transform_from(start, |q| [-q[0], 0.02 + q[2], cz + q[1]]);
        }
        // Nordic: oak, round tapered legs, a curved top rail on the back
        // posts and a linen pad on the seat.
        _ => {
            let r = 0.016;
            for (x, y, top, dir) in [
                (x0 + 0.035, d - 0.035, seat - 0.02, [-0.6, 0.8]),
                (-x0 - 0.035, d - 0.035, seat - 0.02, [0.6, 0.8]),
            ] {
                let start = b.mark();
                turned(
                    b,
                    "frame",
                    [x, y, 0.0],
                    &[(0.0, 0.0), (r * 0.7, 0.0), (r, top), (0.0, top)],
                    14,
                    0.0,
                );
                splay(b, start, top, dir, 0.02);
            }
            for x in [x0 + 0.035, -x0 - 0.035] {
                let path = curve(&[[x, 0.06, 0.0], [x, 0.05, seat], [x, 0.02, h - 0.06]], 6);
                sweep(
                    b,
                    "frame",
                    &path,
                    |t| r * (0.7 + 0.3 * t.min(0.5) * 2.0),
                    14,
                    Cap::Flat,
                    Cap::Round(1.0),
                );
            }
            // The top rail bowed through the back posts' tops.
            let xp = -x0 - 0.035;
            let cy = (xp * xp + 0.000816) / 0.048;
            bent_board(
                b,
                "frame",
                [0.0, cy],
                cy - 0.005,
                w + 0.02,
                0.018,
                h - 0.1,
                h - 0.02,
                0.008,
            );
            b.rbox(
                "frame",
                [x0 + 0.01, 0.02, seat - 0.05],
                [-x0 - 0.01, d - 0.01, seat - 0.018],
                0.01,
            );
            let mut s = upholstered(
                [x0 + 0.015, 0.03, seat - 0.02],
                [-x0 - 0.015, d - 0.01, seat + 0.015],
                0.015,
                58,
            );
            s.crown = 0.008;
            s.welt = 0.003;
            s.add(b, "mat");
        }
    }
}

/// A stool of a collection.
pub fn stool(b: &mut Builder, p: &Params) {
    let w = p.get("width", 0.36).clamp(0.25, 1.0);
    let seat = p.get("height", p.get("seat", 0.65)).clamp(0.25, 0.85);
    let c = [0.0, w / 2.0];
    let tall = seat > 0.55;
    let ring = |b: &mut Builder, role: &'static str, z: f64, r: f64, rr: f64| {
        let pts: Vec<P3> = (0..32)
            .map(|k| {
                let a = TAU * k as f64 / 32.0;
                [c[0] + r * a.cos(), c[1] + r * a.sin(), z]
            })
            .collect();
        super::soft::tube(b, role, &pts, true, rr, 8);
    };
    let disc = |b: &mut Builder, role: &'static str, z: f64, r: f64, t: f64| {
        turned(
            b,
            role,
            [c[0], c[1], z],
            &[
                (0.0, 0.0),
                (r - 0.006, 0.0),
                (r, 0.006),
                (r, t - 0.008),
                (r - 0.01, t),
                (0.0, t + 0.002),
            ],
            36,
            0.0,
        );
    };
    let legs4 = |b: &mut Builder, role: &'static str, top: f64, at: f64, out: f64, r: f64| {
        for k in 0..4 {
            let a = FRAC_PI_4 + k as f64 * PI / 2.0;
            let (sa, ca) = a.sin_cos();
            rod(
                b,
                role,
                [c[0] + ca * (at + out), c[1] + sa * (at + out), 0.0],
                [c[0] + ca * at, c[1] + sa * at, top],
                r,
            );
        }
    };
    let r = w / 2.0;
    match p.style() {
        // Mediterranean: low, four turned legs and a rush seat.
        2 => {
            let top = seat - 0.03;
            for k in 0..4 {
                let a = FRAC_PI_4 + k as f64 * PI / 2.0;
                let (sa, ca) = a.sin_cos();
                turned_leg(b, [c[0] + ca * r * 1.05, c[1] + sa * r * 1.05], top, 0.016);
            }
            for k in 0..4 {
                let a0 = FRAC_PI_4 + k as f64 * PI / 2.0;
                let a1 = a0 + PI / 2.0;
                let z = 0.1 + 0.03 * (k % 2) as f64;
                rod(
                    b,
                    "frame",
                    [c[0] + a0.cos() * r * 1.05, c[1] + a0.sin() * r * 1.05, z],
                    [c[0] + a1.cos() * r * 1.05, c[1] + a1.sin() * r * 1.05, z],
                    0.008,
                );
            }
            let h = r * 0.74;
            let mut s = Soft::new(
                [c[0] - h - 0.01, c[1] - h - 0.01, top - 0.01],
                [c[0] + h + 0.01, c[1] + h + 0.01, seat],
                0.012,
            );
            s.crown = -0.005;
            s.seed = 61;
            s.add(b, "weave");
        }
        // Fifties: a round upholstered seat on compass legs, a brass ring.
        3 => {
            for k in 0..4 {
                let a = FRAC_PI_4 + k as f64 * PI / 2.0;
                let (sa, ca) = a.sin_cos();
                compass_leg(
                    b,
                    [c[0] + ca * r * 0.55, c[1] + sa * r * 0.55],
                    seat - 0.06,
                    [ca, sa],
                    seat * 0.12,
                );
            }
            if tall {
                ring(
                    b,
                    "metal",
                    seat * 0.35,
                    r * 0.55 + seat * 0.12 * 0.65,
                    0.007,
                );
            }
            disc(b, "mat", seat - 0.075, r, 0.075);
            b.soft.insert("mat");
        }
        // Japandi: a block of solid wood, softened, with a hand slot.
        4 => {
            b.rbox("frame", [-r, 0.0, 0.0], [r, w, seat], 0.03);
            b.rbox(
                "dark",
                [-r * 0.5, w * 0.2, seat - 0.002],
                [r * 0.5, w * 0.24, seat + 0.0015],
                0.0,
            );
        }
        // Contemporary: a black tube frame, a padded square seat, a
        // footrest.
        5 => {
            let top = seat - 0.04;
            legs4(b, "metal", top, r * 0.85, 0.02, 0.009);
            if tall {
                let z = seat * 0.3;
                let rr = r * 0.85 + 0.02 * (1.0 - z / top);
                let pts: Vec<P3> = (0..4)
                    .map(|k| {
                        let a = FRAC_PI_4 + k as f64 * PI / 2.0;
                        [c[0] + rr * a.cos(), c[1] + rr * a.sin(), z]
                    })
                    .collect();
                super::soft::tube(b, "metal", &pts, true, 0.008, 8);
            }
            b.rbox(
                "metal",
                [c[0] - r * 0.65, c[1] - r * 0.65, top - 0.006],
                [c[0] + r * 0.65, c[1] + r * 0.65, top + 0.004],
                0.004,
            );
            let mut s = upholstered(
                [c[0] - r, c[1] - r, top],
                [c[0] + r, c[1] + r, seat],
                0.02,
                62,
            );
            s.crown = 0.01;
            s.welt = 0.004;
            s.add(b, "mat");
        }
        // Rustic: a milking stool — a thick round seat on three splayed legs.
        6 => {
            let t = 0.045;
            disc(b, "frame", seat - t, r, t);
            for k in 0..3 {
                let a = TAU * k as f64 / 3.0 + PI / 2.0;
                let (sa, ca) = a.sin_cos();
                let top = [c[0] + ca * r * 0.55, c[1] + sa * r * 0.55, seat - t + 0.01];
                let foot = [c[0] + ca * r * 1.05, c[1] + sa * r * 1.05, 0.0];
                sweep(
                    b,
                    "frame",
                    &[foot, top],
                    |q| 0.017 + 0.004 * q,
                    10,
                    Cap::Flat,
                    Cap::Flat,
                );
            }
        }
        // Industrial: a round wooden seat on a steel tripod with a ring.
        7 => {
            let t = 0.035;
            disc(b, "frame", seat - t, r, t);
            b.rbox(
                "metal",
                [c[0] - r * 0.5, c[1] - r * 0.5, seat - t - 0.012],
                [c[0] + r * 0.5, c[1] + r * 0.5, seat - t + 0.002],
                0.004,
            );
            for k in 0..3 {
                let a = TAU * k as f64 / 3.0 + PI / 2.0;
                let (sa, ca) = a.sin_cos();
                rod(
                    b,
                    "metal",
                    [c[0] + ca * r * 1.0, c[1] + sa * r * 1.0, 0.0],
                    [c[0] + ca * r * 0.4, c[1] + sa * r * 0.4, seat - t - 0.006],
                    0.011,
                );
            }
            if tall {
                let z = seat * 0.32;
                ring(
                    b,
                    "metal",
                    z,
                    r * 0.4 + r * 0.6 * (1.0 - z / (seat - t)),
                    0.007,
                );
            }
        }
        // Classic: a piano bench — buttoned top on four turned legs with an
        // apron.
        8 => {
            let (bw, bd) = (w.max(0.7), 0.36);
            let x0 = -bw / 2.0;
            for (x, y) in [
                (x0 + 0.04, 0.04),
                (-x0 - 0.04, 0.04),
                (x0 + 0.04, bd - 0.04),
                (-x0 - 0.04, bd - 0.04),
            ] {
                turned_leg(b, [x, y], seat - 0.08, 0.02);
            }
            b.rbox(
                "frame",
                [x0 + 0.02, 0.02, seat - 0.13],
                [-x0 - 0.02, bd - 0.02, seat - 0.07],
                0.006,
            );
            let mut s = upholstered([x0, 0.0, seat - 0.075], [-x0, bd, seat], 0.025, 63);
            s.crown = 0.014;
            s.welt = 0.005;
            s.tufts = 0.12;
            s.tuft_depth = 0.02;
            s.tuft_faces = [false, true];
            s.step = 0.025;
            s.add(b, "mat");
        }
        // Nordic: a round wooden seat on four splayed legs with a footrest.
        _ => {
            let t = 0.03;
            disc(b, "frame", seat - t, r, t);
            legs4(b, "frame", seat - t + 0.004, r * 0.6, seat * 0.1, 0.015);
            if tall {
                let z = seat * 0.3;
                let rr = r * 0.6 + seat * 0.1 * (1.0 - z / (seat - t));
                for k in 0..4 {
                    let a0 = FRAC_PI_4 + k as f64 * PI / 2.0;
                    let a1 = a0 + PI / 2.0;
                    let zz = z + 0.03 * (k % 2) as f64;
                    rod(
                        b,
                        "frame",
                        [c[0] + a0.cos() * rr, c[1] + a0.sin() * rr, zz],
                        [c[0] + a1.cos() * rr, c[1] + a1.sin() * rr, zz],
                        0.01,
                    );
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::shape::signed_volume;

    #[test]
    fn the_collection_chairs_and_stools_are_made() {
        for kind in ["fine_chair", "fine_stool"] {
            for style in 1..=8 {
                for seat in [0.45, 0.66] {
                    let mut p = crate::Params::default();
                    p.set("style", style as f64);
                    p.set("seat", seat);
                    p.set("height", if kind == "fine_stool" { seat } else { 0.85 });
                    let m = crate::make(kind, &p).unwrap();
                    let tris = m.triangle_count();
                    assert!((200..20_000).contains(&tris), "{kind} {style}: {tris}");
                    let v: f64 = m
                        .parts
                        .iter()
                        .map(|(_, t)| signed_volume(&m.positions, t))
                        .sum();
                    assert!(v > 0.0005, "{kind} {style}: {v}");
                }
            }
        }
    }
}
