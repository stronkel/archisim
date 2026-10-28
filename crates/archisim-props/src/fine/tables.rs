//! The workshop's tables, one of each kind for each collection
//! (paquete 1b): dining, coffee and side tables.
//!
//! Dining and coffee tables: `width`, `depth`, `height`; side tables:
//! `width` (across) and `height`. Centred on `x = 0`, from `y = 0` to their
//! depth.
//!
//! Roles: `top` the tabletop, `frame` wood, `metal` steel or brass,
//! `stone` marble, `ceramic` glazed earthenware.

use super::shape::{curve, plate, sweep, turned, Cap};
use super::sofas::compass_leg;
use crate::build::{Builder, P3};
use crate::Params;
use std::f64::consts::{FRAC_PI_4, PI, TAU};

/// A straight round rod from `a` to `c`.
fn rod(b: &mut Builder, role: &'static str, a: P3, c: P3, r: f64) {
    sweep(b, role, &[a, c], |_| r, 12, Cap::Flat, Cap::Flat);
}

/// A tapered round leg from the floor at `foot` to `top` (its top
/// centre), `r0` at the foot, `r1` at the top.
fn leg(b: &mut Builder, role: &'static str, foot: P3, top: P3, r0: f64, r1: f64) {
    sweep(
        b,
        role,
        &[foot, top],
        |t| r0 + (r1 - r0) * t,
        14,
        Cap::Flat,
        Cap::Flat,
    );
}

/// A round or oval top: radius `rx` × `ry` about `c`, `t` thick, its top
/// at `z`, its edge eased.
pub(super) fn round_top(
    b: &mut Builder,
    role: &'static str,
    c: [f64; 2],
    rx: f64,
    ry: f64,
    z: f64,
    t: f64,
) {
    b.lathe(
        role,
        [c[0], c[1], z - t],
        &[
            (0.0, 0.0),
            (0.96, 0.0),
            (0.995, t * 0.25),
            (1.0, t * 0.6),
            (0.99, t * 0.92),
            (0.97, t),
            (0.0, t),
        ],
        rx,
        ry,
    );
}

/// A top of planks along `x`, a little gap between them.
#[allow(clippy::too_many_arguments)]
fn planks(
    b: &mut Builder,
    role: &'static str,
    x0: f64,
    x1: f64,
    y0: f64,
    y1: f64,
    z0: f64,
    z1: f64,
    n: usize,
) {
    let gap = 0.003;
    let pw = (y1 - y0) / n as f64;
    for i in 0..n {
        let ya = y0 + i as f64 * pw;
        b.rbox(
            role,
            [x0, ya + gap / 2.0, z0],
            [x1, ya + pw - gap / 2.0, z1],
            0.004,
        );
    }
}

/// A dining table of a collection.
pub fn dining(b: &mut Builder, p: &Params) {
    let w = p.get("width", 1.6).clamp(0.6, 3.6);
    let d = p.get("depth", 0.9).clamp(0.6, 1.4);
    let h = p.get("height", 0.75).clamp(0.6, 1.1);
    let x0 = -w / 2.0;
    let c = [0.0, d / 2.0];
    match p.style() {
        // Mediterranean: whitewashed, a thick top over a deep apron, square
        // tapered legs.
        2 => {
            let t = 0.04;
            b.rbox("top", [x0, 0.0, h - t], [-x0, d, h], 0.008);
            let leg_w = 0.065;
            for (x, y) in [
                (x0 + 0.05, 0.05),
                (-x0 - 0.05 - leg_w, 0.05),
                (x0 + 0.05, d - 0.05 - leg_w),
                (-x0 - 0.05 - leg_w, d - 0.05 - leg_w),
            ] {
                let start = b.mark();
                b.rbox("frame", [x, y, 0.0], [x + leg_w, y + leg_w, h - t], 0.004);
                let (cx, cy) = (x + leg_w / 2.0, y + leg_w / 2.0);
                b.transform_from(start, |q| {
                    let k = 0.65 + 0.35 * (q[2] / (h - t));
                    [cx + (q[0] - cx) * k, cy + (q[1] - cy) * k, q[2]]
                });
            }
            let (ax0, ax1, ay0, ay1) = (x0 + 0.065, -x0 - 0.065, 0.065, d - 0.065);
            b.rbox(
                "frame",
                [ax0, ay0, h - t - 0.11],
                [ax1, ay0 + 0.022, h - t],
                0.0,
            );
            b.rbox(
                "frame",
                [ax0, ay1 - 0.022, h - t - 0.11],
                [ax1, ay1, h - t],
                0.0,
            );
            b.rbox(
                "frame",
                [ax0, ay0, h - t - 0.11],
                [ax0 + 0.022, ay1, h - t],
                0.0,
            );
            b.rbox(
                "frame",
                [ax1 - 0.022, ay0, h - t - 0.11],
                [ax1, ay1, h - t],
                0.0,
            );
        }
        // Fifties: an oval top on splayed compass legs.
        3 => {
            let t = 0.03;
            round_top(b, "top", c, w / 2.0, d / 2.0, h, t);
            b.rbox(
                "frame",
                [x0 * 0.6, c[1] - d * 0.28, h - t - 0.05],
                [-x0 * 0.6, c[1] + d * 0.28, h - t],
                0.01,
            );
            for (dx, dy) in [(-1.0f64, -1.0f64), (1.0, -1.0), (-1.0, 1.0), (1.0, 1.0)] {
                let l = (dx * dx + dy * dy).sqrt();
                compass_leg(
                    b,
                    [dx * (-x0 * 0.55), c[1] + dy * d * 0.24],
                    h - t - 0.005,
                    [dx / l, dy / l],
                    0.07,
                );
            }
        }
        // Japandi: a thick top on two plank feet joined by a rail.
        4 => {
            let t = 0.05;
            b.rbox("top", [x0, 0.0, h - t], [-x0, d, h], 0.012);
            let inset = (w * 0.16).min(0.35);
            for x in [x0 + inset, -x0 - inset - 0.05] {
                b.rbox("frame", [x, 0.12, 0.0], [x + 0.05, d - 0.12, h - t], 0.008);
                b.rbox(
                    "frame",
                    [x - 0.03, 0.08, 0.0],
                    [x + 0.08, d - 0.08, 0.04],
                    0.008,
                );
            }
            b.rbox(
                "frame",
                [x0 + inset, c[1] - 0.025, 0.24],
                [-x0 - inset, c[1] + 0.025, 0.3],
                0.006,
            );
        }
        // Contemporary: a thin top on two V legs of black steel.
        5 => {
            let t = 0.022;
            b.rbox("top", [x0, 0.0, h - t], [-x0, d, h], 0.004);
            for x in [x0 + 0.12, -x0 - 0.12] {
                for y in [0.06, d - 0.06] {
                    rod(b, "metal", [x, c[1], 0.006], [x, y, h - t], 0.012);
                }
                rod(
                    b,
                    "metal",
                    [x, 0.06, h - t - 0.012],
                    [x, d - 0.06, h - t - 0.012],
                    0.012,
                );
                b.rbox(
                    "metal",
                    [x - 0.03, c[1] - 0.04, 0.0],
                    [x + 0.03, c[1] + 0.04, 0.008],
                    0.002,
                );
            }
        }
        // Rustic: a farmhouse table — a thick top of planks, chunky legs,
        // a deep apron, the edges chamfered.
        6 => {
            let t = 0.055;
            planks(b, "top", x0, -x0, 0.0, d, h - t, h, 4);
            let leg_w = 0.09;
            for (x, y) in [
                (x0 + 0.06, 0.06),
                (-x0 - 0.06 - leg_w, 0.06),
                (x0 + 0.06, d - 0.06 - leg_w),
                (-x0 - 0.06 - leg_w, d - 0.06 - leg_w),
            ] {
                b.rbox("frame", [x, y, 0.0], [x + leg_w, y + leg_w, h - t], 0.012);
            }
            b.rbox(
                "frame",
                [x0 + 0.08, 0.08, h - t - 0.12],
                [-x0 - 0.08, 0.11, h - t],
                0.006,
            );
            b.rbox(
                "frame",
                [x0 + 0.08, d - 0.11, h - t - 0.12],
                [-x0 - 0.08, d - 0.08, h - t],
                0.006,
            );
            b.rbox(
                "frame",
                [x0 + 0.08, 0.08, 0.1],
                [-x0 - 0.08, d - 0.08, 0.14],
                0.0,
            );
        }
        // Industrial: reclaimed planks on two rectangular frames of black
        // square tube and a long rail under the top joining them.
        7 => {
            let t = 0.045;
            planks(b, "top", x0, -x0, 0.0, d, h - t, h, 3);
            let tube = 0.04;
            for x in [x0 + 0.1, -x0 - 0.1] {
                let pts = [
                    [x, 0.06, tube / 2.0],
                    [x, d - 0.06, tube / 2.0],
                    [x, d - 0.06, h - t - tube / 2.0],
                    [x, 0.06, h - t - tube / 2.0],
                ];
                for i in 0..4 {
                    let (a, e) = (pts[i], pts[(i + 1) % 4]);
                    super::shape::bar(b, "metal", a, e, tube, [1.0, 0.0, 0.0]);
                }
            }
            super::shape::bar(
                b,
                "metal",
                [x0 + 0.1, c[1], h - t - 0.015],
                [-x0 - 0.1, c[1], h - t - 0.015],
                0.03,
                [0.0, 1.0, 0.0],
            );
        }
        // Classic: a round top on a turned pedestal with three curved feet.
        8 => {
            let r = (w.min(d) / 2.0).max(0.45);
            let c = [0.0, r];
            round_top(b, "top", c, r, r, h, 0.035);
            pedestal(b, c, h - 0.035, r * 0.42, 0.06);
        }
        // Nordic: oak, round tapered legs splayed a little, a slim apron.
        _ => {
            let t = 0.032;
            b.rbox("top", [x0, 0.0, h - t], [-x0, d, h], 0.012);
            for (dx, dy) in [(-1.0f64, -1.0f64), (1.0, -1.0), (-1.0, 1.0), (1.0, 1.0)] {
                let (x, y) = (dx * (-x0 - 0.1), c[1] + dy * (d / 2.0 - 0.1));
                leg(
                    b,
                    "frame",
                    [x + dx * 0.02, y + dy * 0.02, 0.0],
                    [x, y, h - t],
                    0.016,
                    0.026,
                );
            }
            b.rbox(
                "frame",
                [x0 + 0.1, 0.09, h - t - 0.07],
                [-x0 - 0.1, d - 0.09, h - t],
                0.0,
            );
        }
    }
}

/// A turned pedestal: column from the floor to `top`, three feet curving
/// out `reach`, `r` the column's radius.
fn pedestal(b: &mut Builder, c: [f64; 2], top: f64, reach: f64, r: f64) {
    turned(
        b,
        "frame",
        [c[0], c[1], 0.0],
        &[
            (0.0, 0.08),
            (r * 1.3, 0.08),
            (r * 1.5, 0.12),
            (r * 1.1, 0.18),
            (r * 0.7, top * 0.35),
            (r * 0.9, top * 0.45),
            (r * 0.6, top * 0.55),
            (r * 0.55, top * 0.85),
            (r * 0.9, top * 0.92),
            (r * 1.4, top),
            (0.0, top),
        ],
        24,
        0.0,
    );
    for k in 0..3 {
        let a = TAU * k as f64 / 3.0 + PI / 2.0;
        let (sa, ca) = a.sin_cos();
        let path = curve(
            &[
                [c[0] + ca * r * 0.7, c[1] + sa * r * 0.7, 0.2],
                [c[0] + ca * reach * 0.3, c[1] + sa * reach * 0.3, 0.17],
                [c[0] + ca * reach * 0.62, c[1] + sa * reach * 0.62, 0.08],
                [c[0] + ca * reach * 0.9, c[1] + sa * reach * 0.9, 0.025],
                [c[0] + ca * reach * 1.04, c[1] + sa * reach * 1.04, 0.035],
            ],
            8,
        );
        sweep(
            b,
            "frame",
            &path,
            |t| 0.034 - 0.02 * t + 0.008 * (t * PI).sin(),
            10,
            Cap::Flat,
            Cap::Round(1.0),
        );
        let pad = [c[0] + ca * reach * 0.92, c[1] + sa * reach * 0.92];
        b.lathe(
            "frame",
            [pad[0], pad[1], 0.0],
            &[
                (0.0, 0.0),
                (0.02, 0.0),
                (0.022, 0.012),
                (0.012, 0.03),
                (0.0, 0.03),
            ],
            1.0,
            1.0,
        );
    }
}

/// A coffee table of a collection.
pub fn coffee(b: &mut Builder, p: &Params) {
    let w = p.get("width", 1.1).clamp(0.4, 2.0);
    let d = p.get("depth", 0.6).clamp(0.35, 1.2);
    let h = p.get("height", 0.4).clamp(0.25, 0.55);
    let x0 = -w / 2.0;
    let c = [0.0, d / 2.0];
    match p.style() {
        // Mediterranean: whitewashed, chunky legs and a slatted shelf.
        2 => {
            b.rbox("top", [x0, 0.0, h - 0.04], [-x0, d, h], 0.008);
            let lw = 0.06;
            for (x, y) in [
                (x0 + 0.03, 0.03),
                (-x0 - 0.03 - lw, 0.03),
                (x0 + 0.03, d - 0.03 - lw),
                (-x0 - 0.03 - lw, d - 0.03 - lw),
            ] {
                b.rbox("frame", [x, y, 0.0], [x + lw, y + lw, h - 0.04], 0.006);
            }
            let n = ((d - 0.12) / 0.07).round().max(3.0) as usize;
            for i in 0..n {
                let y = 0.06 + (d - 0.12) * (i as f64 + 0.5) / n as f64;
                b.rbox(
                    "frame",
                    [x0 + 0.06, y - 0.025, 0.08],
                    [-x0 - 0.06, y + 0.025, 0.1],
                    0.0,
                );
            }
            for y in [0.06, d - 0.06] {
                b.rbox(
                    "frame",
                    [x0 + 0.06, y - 0.012, 0.06],
                    [-x0 - 0.06, y + 0.012, 0.08],
                    0.0,
                );
            }
        }
        // Fifties: an oval teak top on splayed compass legs.
        3 => {
            round_top(b, "top", c, w / 2.0, d / 2.0, h, 0.03);
            for (dx, dy) in [(-1.0f64, -1.0f64), (1.0, -1.0), (-1.0, 1.0), (1.0, 1.0)] {
                let l = (dx * dx + dy * dy).sqrt();
                compass_leg(
                    b,
                    [dx * w * 0.3, c[1] + dy * d * 0.22],
                    h - 0.034,
                    [dx / l, dy / l],
                    0.05,
                );
            }
        }
        // Japandi: a low box of wood open at the ends.
        4 => {
            let t = 0.035;
            b.rbox("top", [x0, 0.0, h - t], [-x0, d, h], 0.008);
            b.rbox("frame", [x0, 0.0, 0.0], [-x0, d, t], 0.008);
            for x in [x0, -x0 - t] {
                b.rbox(
                    "frame",
                    [x, 0.0, t - 0.002],
                    [x + t, d, h - t + 0.002],
                    0.004,
                );
            }
            b.rbox(
                "frame",
                [x0 + w * 0.45, 0.0, t - 0.002],
                [x0 + w * 0.45 + t, d, h - t + 0.002],
                0.004,
            );
        }
        // Contemporary: a round marble top on a ring of black steel.
        5 => {
            let r = w.min(d) / 2.0;
            let c = [0.0, r];
            round_top(b, "stone", c, r, r, h, 0.025);
            let ring: Vec<P3> = (0..40)
                .map(|k| {
                    let a = TAU * k as f64 / 40.0;
                    [c[0] + r * 0.82 * a.cos(), c[1] + r * 0.82 * a.sin(), 0.008]
                })
                .collect();
            super::soft::tube(b, "metal", &ring, true, 0.008, 8);
            for k in 0..4 {
                let a = FRAC_PI_4 + k as f64 * PI / 2.0;
                let (sa, ca) = a.sin_cos();
                rod(
                    b,
                    "metal",
                    [c[0] + ca * r * 0.82, c[1] + sa * r * 0.82, 0.004],
                    [c[0] + ca * r * 0.82, c[1] + sa * r * 0.82, h - 0.025],
                    0.008,
                );
            }
        }
        // Rustic: a wooden chest with iron straps.
        6 => {
            let lid = 0.05;
            b.rbox("frame", [x0, 0.0, 0.03], [-x0, d, h - lid], 0.008);
            b.rbox(
                "top",
                [x0 - 0.01, -0.01, h - lid],
                [-x0 + 0.01, d + 0.01, h],
                0.012,
            );
            for x in [x0 + 0.04, -x0 - 0.04] {
                b.rbox("frame", [x - 0.04, 0.0, 0.0], [x + 0.04, d, 0.035], 0.004);
            }
            for x in [x0 + w * 0.2, -x0 - w * 0.2] {
                b.rbox(
                    "metal",
                    [x - 0.025, -0.004, 0.03],
                    [x + 0.025, d + 0.004, h - lid],
                    0.002,
                );
                b.rbox(
                    "metal",
                    [x - 0.025, -0.014, h - lid],
                    [x + 0.025, d + 0.014, h + 0.004],
                    0.002,
                );
            }
            b.rbox(
                "metal",
                [-0.04, d - 0.002, h - lid - 0.08],
                [0.04, d + 0.012, h - lid + 0.01],
                0.003,
            );
        }
        // Industrial: planks on a black steel frame, on four casters.
        7 => {
            let t = 0.04;
            let wheel = 0.08;
            planks(b, "top", x0, -x0, 0.0, d, h - t, h, 3);
            let tube = 0.03;
            let z0 = wheel + 0.01;
            let pts = [
                [x0 + 0.04, 0.04],
                [-x0 - 0.04, 0.04],
                [-x0 - 0.04, d - 0.04],
                [x0 + 0.04, d - 0.04],
            ];
            for (i, q) in pts.iter().enumerate() {
                super::shape::bar(
                    b,
                    "metal",
                    [q[0], q[1], z0],
                    [q[0], q[1], h - t],
                    tube,
                    [1.0, 0.0, 0.0],
                );
                let e = pts[(i + 1) % 4];
                for z in [z0 + tube / 2.0, h - t - tube / 2.0] {
                    super::shape::bar(
                        b,
                        "metal",
                        [q[0], q[1], z],
                        [e[0], e[1], z],
                        tube,
                        [0.0, 0.0, 1.0],
                    );
                }
                // The caster: a bracket and its wheel, stood on its edge.
                b.rbox(
                    "metal",
                    [q[0] - 0.02, q[1] - 0.02, wheel - 0.01],
                    [q[0] + 0.02, q[1] + 0.02, z0],
                    0.003,
                );
                let start = b.mark();
                turned(
                    b,
                    "dark",
                    [0.0; 3],
                    &[
                        (0.0, -0.012),
                        (wheel / 2.0 - 0.006, -0.012),
                        (wheel / 2.0, 0.0),
                        (wheel / 2.0 - 0.006, 0.012),
                        (0.0, 0.012),
                    ],
                    16,
                    0.0,
                );
                let (qx, qy) = (q[0], q[1]);
                b.transform_from(start, |v| [qx + v[2], qy - v[1], wheel / 2.0 + v[0]]);
            }
            // The lower shelf.
            planks(
                b,
                "top",
                x0 + 0.055,
                -x0 - 0.055,
                0.055,
                d - 0.055,
                z0 + tube,
                z0 + tube + 0.025,
                2,
            );
        }
        // Classic: a marble top with a moulded edge on turned legs and a
        // shelf.
        8 => {
            b.rbox(
                "frame",
                [x0 + 0.02, 0.02, h - 0.1],
                [-x0 - 0.02, d - 0.02, h - 0.03],
                0.008,
            );
            b.rbox("stone", [x0, 0.0, h - 0.03], [-x0, d, h], 0.012);
            for (x, y) in [
                (x0 + 0.05, 0.05),
                (-x0 - 0.05, 0.05),
                (x0 + 0.05, d - 0.05),
                (-x0 - 0.05, d - 0.05),
            ] {
                turned(
                    b,
                    "frame",
                    [x, y, 0.0],
                    &[
                        (0.0, 0.0),
                        (0.013, 0.0),
                        (0.016, 0.03),
                        (0.012, 0.05),
                        (0.02, h * 0.35),
                        (0.014, h * 0.45),
                        (0.022, h - 0.1),
                        (0.0, h - 0.1),
                    ],
                    16,
                    0.0,
                );
            }
            b.rbox(
                "frame",
                [x0 + 0.04, 0.04, 0.09],
                [-x0 - 0.04, d - 0.04, 0.11],
                0.004,
            );
        }
        // Nordic: round oak with a shelf below.
        _ => {
            let r = w.min(d) / 2.0;
            let r = r.max(0.35);
            let c = [0.0, r];
            round_top(b, "top", c, r, r, h, 0.028);
            round_top(b, "frame", c, r * 0.82, r * 0.82, 0.12, 0.02);
            for k in 0..4 {
                let a = FRAC_PI_4 + k as f64 * PI / 2.0;
                let (sa, ca) = a.sin_cos();
                let at = [c[0] + ca * r * 0.7, c[1] + sa * r * 0.7];
                leg(
                    b,
                    "frame",
                    [at[0] + ca * 0.02, at[1] + sa * 0.02, 0.0],
                    [at[0], at[1], h - 0.028],
                    0.015,
                    0.022,
                );
            }
        }
    }
}

/// A side table of a collection.
pub fn side(b: &mut Builder, p: &Params) {
    let w = p.get("width", 0.45).clamp(0.25, 0.8);
    let h = p.get("height", 0.55).clamp(0.3, 0.8);
    let r = w / 2.0;
    let c = [0.0, r];
    match p.style() {
        // Mediterranean: a glazed earthenware drum, waisted.
        2 => {
            turned(
                b,
                "ceramic",
                [c[0], c[1], 0.0],
                &[
                    (0.0, 0.0),
                    (r * 0.8, 0.0),
                    (r * 0.85, 0.02),
                    (r * 0.6, h * 0.35),
                    (r * 0.5, h * 0.5),
                    (r * 0.6, h * 0.65),
                    (r * 0.95, h - 0.03),
                    (r, h - 0.012),
                    (r * 0.96, h),
                    (0.0, h),
                ],
                36,
                0.0,
            );
        }
        // Fifties: two round tiers on three splayed legs.
        3 => {
            round_top(b, "top", c, r, r, h, 0.022);
            round_top(b, "top", c, r * 0.8, r * 0.8, h * 0.45, 0.018);
            for k in 0..3 {
                let a = TAU * k as f64 / 3.0 + PI / 2.0;
                let (sa, ca) = a.sin_cos();
                compass_leg(
                    b,
                    [c[0] + ca * r * 0.6, c[1] + sa * r * 0.6],
                    h - 0.022,
                    [ca, sa],
                    0.05,
                );
            }
        }
        // Japandi: a round top on two boards slotted into a cross.
        4 => {
            round_top(b, "top", c, r, r, h, 0.024);
            let (t, a) = (0.024, r * 0.72);
            b.rbox(
                "frame",
                [c[0] - a, c[1] - t / 2.0, 0.0],
                [c[0] + a, c[1] + t / 2.0, h - 0.024],
                0.003,
            );
            b.rbox(
                "frame",
                [c[0] - t / 2.0, c[1] - a, 0.0],
                [c[0] + t / 2.0, c[1] - t / 2.0, h - 0.024],
                0.003,
            );
            b.rbox(
                "frame",
                [c[0] - t / 2.0, c[1] + t / 2.0, 0.0],
                [c[0] + t / 2.0, c[1] + a, h - 0.024],
                0.003,
            );
        }
        // Contemporary: a sheet of steel folded into a C.
        5 => {
            let t = 0.006;
            let (x0, x1) = (-r, r);
            plate(
                b,
                "metal",
                [
                    [x0, 0.0, h - t / 2.0],
                    [x1, 0.0, h - t / 2.0],
                    [x1, w, h - t / 2.0],
                    [x0, w, h - t / 2.0],
                ],
                t,
            );
            plate(
                b,
                "metal",
                [
                    [x0, 0.0, t / 2.0],
                    [x1, 0.0, t / 2.0],
                    [x1, w * 0.78, t / 2.0],
                    [x0, w * 0.78, t / 2.0],
                ],
                t,
            );
            plate(
                b,
                "metal",
                [
                    [x0, t / 2.0, 0.0],
                    [x1, t / 2.0, 0.0],
                    [x1, t / 2.0, h],
                    [x0, t / 2.0, h],
                ],
                t,
            );
            // The folds, rounded.
            for z in [t / 2.0, h - t / 2.0] {
                rod(b, "metal", [x0, t / 2.0, z], [x1, t / 2.0, z], t * 0.7);
            }
            // A board laid into the top, its edge set back from the steel.
            b.rbox(
                "top",
                [x0 + 0.012, 0.012, h],
                [x1 - 0.012, w - 0.012, h + 0.014],
                0.004,
            );
            // Felt pads under the base.
            for q in [
                [x0 + 0.03, 0.03],
                [x1 - 0.03, 0.03],
                [x0 + 0.03, w * 0.74],
                [x1 - 0.03, w * 0.74],
            ] {
                b.rbox(
                    "dark",
                    [q[0] - 0.01, q[1] - 0.01, -0.004],
                    [q[0] + 0.01, q[1] + 0.01, 0.0],
                    0.0,
                );
            }
        }
        // Rustic: a slice of trunk with its bark ring and a top.
        6 => {
            turned(
                b,
                "frame",
                [c[0], c[1], 0.0],
                &[
                    (0.0, 0.0),
                    (r * 1.02, 0.0),
                    (r * 0.98, 0.03),
                    (r * 0.86, 0.12),
                    (r * 0.84, h * 0.5),
                    (r * 0.88, h - 0.05),
                    (r * 0.9, h - 0.02),
                    (0.0, h - 0.02),
                ],
                30,
                0.12,
            );
            round_top(b, "top", c, r * 0.88, r * 0.88, h, 0.022);
        }
        // Industrial: a steel cube frame with a wooden drawer box.
        7 => {
            let tube = 0.022;
            let x0 = -r;
            let corners = [[x0, 0.0], [-x0, 0.0], [-x0, w], [x0, w]];
            for (i, q) in corners.iter().enumerate() {
                let (qx, qy) = (
                    q[0] + (if q[0] < 0.0 { 1.0 } else { -1.0 }) * tube / 2.0,
                    q[1] + (if q[1] < 0.01 { 1.0 } else { -1.0 }) * tube / 2.0,
                );
                super::shape::bar(
                    b,
                    "metal",
                    [qx, qy, 0.0],
                    [qx, qy, h - 0.02],
                    tube,
                    [1.0, 0.0, 0.0],
                );
                let e = corners[(i + 1) % 4];
                let (ex, ey) = (
                    e[0] + (if e[0] < 0.0 { 1.0 } else { -1.0 }) * tube / 2.0,
                    e[1] + (if e[1] < 0.01 { 1.0 } else { -1.0 }) * tube / 2.0,
                );
                for z in [0.08, h - 0.02 - tube / 2.0] {
                    super::shape::bar(b, "metal", [qx, qy, z], [ex, ey, z], tube, [0.0, 0.0, 1.0]);
                }
            }
            b.rbox("top", [x0, 0.0, h - 0.02], [-x0, w, h + 0.01], 0.004);
            b.rbox(
                "frame",
                [x0 + tube, tube, h - 0.17],
                [-x0 - tube, w - tube + 0.004, h - 0.02],
                0.004,
            );
            b.rbox(
                "metal",
                [-0.04, w - tube + 0.004, h - 0.1],
                [0.04, w - tube + 0.02, h - 0.09],
                0.003,
            );
            b.rbox(
                "top",
                [x0 + tube, tube, 0.08 + tube / 2.0],
                [-x0 - tube, w - tube, 0.1 + tube / 2.0],
                0.0,
            );
        }
        // Classic: a tripod gueridon — a round top on a turned column.
        8 => {
            round_top(b, "top", c, r, r, h, 0.025);
            pedestal(b, c, h - 0.025, r * 0.75, 0.03);
        }
        // Nordic: three legs and a tray top with its rim.
        _ => {
            b.lathe(
                "top",
                [c[0], c[1], h - 0.03],
                &[
                    (0.0, 0.0),
                    (r - 0.004, 0.0),
                    (r, 0.006),
                    (r, 0.03),
                    (r - 0.012, 0.03),
                    (r - 0.014, 0.012),
                    (0.0, 0.012),
                ],
                1.0,
                1.0,
            );
            for k in 0..3 {
                let a = TAU * k as f64 / 3.0 + PI / 2.0;
                let (sa, ca) = a.sin_cos();
                let at = [c[0] + ca * r * 0.62, c[1] + sa * r * 0.62];
                leg(
                    b,
                    "frame",
                    [at[0] + ca * 0.03, at[1] + sa * 0.03, 0.0],
                    [at[0], at[1], h - 0.028],
                    0.012,
                    0.016,
                );
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::shape::signed_volume;

    #[test]
    fn the_collection_tables_are_made() {
        for kind in ["fine_dining_table", "fine_coffee_table", "fine_side_table"] {
            for style in 1..=8 {
                let mut p = crate::Params::default();
                p.set("style", style as f64);
                let m = crate::make(kind, &p).unwrap();
                let tris = m.triangle_count();
                assert!((30..20_000).contains(&tris), "{kind} {style}: {tris}");
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
