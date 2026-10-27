//! The rest of the workshop's bedrooms (paquete 3, segunda vuelta): the
//! child's bed, the cot, the bunk bed, the valet stand and the standing
//! mirror, one for each collection, with the same bed clothes as the
//! big beds.
//!
//! Axes as every piece's: the back on the wall at `y = 0`, the front
//! looking to `+y`, centred on `x = 0`. Roles as in [`super::bedroom`],
//! and `clothes` for what hangs on a valet stand, `toy` for a cot's soft
//! toy.

use super::bedding::Cover;
use super::bedroom::{
    arch_band, arched_board, bed_frame, look, louis_leg, make_bed, mirror_fix, square_legs,
    turned_post,
};
use super::shape::{curve, sweep, turned, Cap};
use super::sofas::compass_leg;
use super::soft::{tube, Soft};
use super::storage::{corners, tapered_leg, OUT};
use crate::build::{Builder, P3};
use crate::Params;
use std::f64::consts::{FRAC_PI_2, PI, TAU};

/// The collection of a piece that only has the collections' models.
fn own(p: &Params) -> usize {
    (p.style() - 1).clamp(0, 7) as usize
}

/// Runs `make` and moves what it made: turned `turn` radians about the
/// vertical, then by `at`.
fn placed(b: &mut Builder, at: P3, turn: f64, make: impl FnOnce(&mut Builder)) {
    let start = b.mark();
    make(b);
    let (s, c) = turn.sin_cos();
    b.transform_from(start, |q| {
        [
            at[0] + q[0] * c - q[1] * s,
            at[1] + q[0] * s + q[1] * c,
            at[2] + q[2],
        ]
    });
}

/// A rail of collection `ix` from `a` to `c` (in plan), from `z0` to
/// `z1`: its top and bottom rails and what fills between them
/// (spindles, cane, slats, a panel or steel bars), `t` thick.
pub(super) fn fence(
    b: &mut Builder,
    ix: usize,
    a: [f64; 2],
    c: [f64; 2],
    z0: f64,
    z1: f64,
    t: f64,
) {
    let len = ((c[0] - a[0]).powi(2) + (c[1] - a[1]).powi(2)).sqrt();
    if len < 0.05 || z1 - z0 < 0.04 {
        return;
    }
    let turn = (c[1] - a[1]).atan2(c[0] - a[0]);
    placed(b, [a[0], a[1], 0.0], turn, |b| {
        let h = t / 2.0;
        let role = if ix == 6 { "metal" } else { "frame" };
        let rail = 0.035_f64.min((z1 - z0) / 4.0);
        // Along x from 0 to `len`, centred on y = 0.
        match ix {
            6 => {
                for z in [z0 + 0.015, z1 - 0.015] {
                    b.cyl("metal", [0.0, 0.0, z], [len, 0.0, z], 0.015);
                }
            }
            7 => {
                b.rbox(role, [0.0, -h, z0], [len, h, z0 + rail], 0.004);
                b.cyl(role, [0.0, 0.0, z1 - 0.018], [len, 0.0, z1 - 0.018], 0.02);
            }
            _ => {
                b.rbox(role, [0.0, -h, z0], [len, h, z0 + rail], 0.004);
                b.rbox(role, [0.0, -h, z1 - rail], [len, h, z1], 0.004);
            }
        }
        let (fz0, fz1) = (z0 + rail, z1 - rail);
        let spaced = |step: f64| -> Vec<f64> {
            let n = ((len / step).round() as usize).max(1);
            (1..n).map(|i| len * i as f64 / n as f64).collect()
        };
        match ix {
            // Round spindles.
            0 => {
                for x in spaced(0.075) {
                    b.cyl("frame", [x, 0.0, fz0], [x, 0.0, fz1], 0.011);
                }
            }
            // A cane panel.
            1 => b.rbox(
                "weave",
                [0.01, -h * 0.4, fz0],
                [len - 0.01, h * 0.4, fz1],
                0.0,
            ),
            // Flat upright slats.
            2 => {
                for x in spaced(0.07) {
                    b.rbox(
                        "frame",
                        [x - 0.016, -h * 0.7, fz0],
                        [x + 0.016, h * 0.7, fz1],
                        0.0,
                    );
                }
            }
            // Slats across, with gaps.
            3 => {
                let n = (((fz1 - fz0) / 0.09).round() as usize).max(1);
                for k in 1..n {
                    let z = fz0 + (fz1 - fz0) * k as f64 / n as f64;
                    b.rbox("frame", [0.0, -h, z - 0.018], [len, h, z + 0.018], 0.0);
                }
            }
            // A lacquered panel.
            4 => b.rbox("frame", [0.0, -h * 0.8, fz0], [len, h * 0.8, fz1], 0.004),
            // Turned spindles.
            5 | 7 => {
                for x in spaced(0.085) {
                    let hh = fz1 - fz0;
                    turned(
                        b,
                        "frame",
                        [x, 0.0, fz0],
                        &[
                            (0.0, 0.0),
                            (0.012, 0.0),
                            (0.014, hh * 0.15),
                            (0.009, hh * 0.25),
                            (0.016, hh * 0.5),
                            (0.009, hh * 0.75),
                            (0.014, hh * 0.85),
                            (0.012, hh),
                            (0.0, hh),
                        ],
                        10,
                        0.0,
                    );
                }
            }
            // Steel bars.
            _ => {
                for x in spaced(0.1) {
                    b.cyl("metal", [x, 0.0, fz0 - 0.015], [x, 0.0, fz1 + 0.015], 0.008);
                }
            }
        }
    });
}

/// A corner post of collection `ix` from the floor to `top`, `s` across.
fn post(b: &mut Builder, ix: usize, at: [f64; 2], top: f64, s: f64) {
    match ix {
        5 => {
            b.rbox(
                "frame",
                [at[0] - s / 2.0, at[1] - s / 2.0, 0.0],
                [at[0] + s / 2.0, at[1] + s / 2.0, top],
                0.006,
            );
            turned(
                b,
                "frame",
                [at[0], at[1], top],
                &[
                    (s * 0.42, 0.0),
                    (s * 0.3, 0.025),
                    (s * 0.4, 0.045),
                    (s * 0.15, 0.07),
                    (0.0, 0.075),
                ],
                16,
                0.0,
            );
        }
        6 => {
            b.cyl("metal", [at[0], at[1], 0.0], [at[0], at[1], top], 0.022);
            b.cyl("metal", [at[0], at[1], 0.0], [at[0], at[1], 0.012], 0.035);
            b.ellipsoid("metal", [at[0], at[1], top + 0.008], [0.027, 0.027, 0.022]);
        }
        7 => {
            louis_leg(b, "frame", at, top, s / 2.0);
            turned(
                b,
                "frame",
                [at[0], at[1], top],
                &[
                    (s * 0.5, 0.0),
                    (s * 0.3, 0.02),
                    (s * 0.42, 0.045),
                    (s * 0.12, 0.07),
                    (0.0, 0.085),
                ],
                16,
                0.0,
            );
        }
        2 => {
            // Teak, rounded, on a brass shoe.
            b.rbox(
                "frame",
                [at[0] - s / 2.0, at[1] - s / 2.0, 0.03],
                [at[0] + s / 2.0, at[1] + s / 2.0, top],
                0.012,
            );
            turned(
                b,
                "metal",
                [at[0], at[1], 0.0],
                &[(0.0, 0.0), (s * 0.4, 0.0), (s * 0.42, 0.03), (0.0, 0.03)],
                16,
                0.0,
            );
        }
        _ => {
            b.rbox(
                "frame",
                [at[0] - s / 2.0, at[1] - s / 2.0, 0.0],
                [at[0] + s / 2.0, at[1] + s / 2.0, top],
                if ix == 0 || ix == 4 { 0.012 } else { 0.004 },
            );
        }
    }
}

// ---------------------------------------------------------- the child's bed

/// Children's beds of the collections (after the family's 2 own models):
/// the collection's bed made smaller and lower, a guard along the head
/// half of its side, bed clothes and a soft toy. `width` and `depth` of
/// the mattress.
pub fn kids_bed(b: &mut Builder, p: &Params) -> bool {
    let Some(ix) = look(p, 2) else {
        return false;
    };
    let w = p.get("width", 0.9).clamp(0.7, 1.4);
    let l = p.get("depth", 1.9).clamp(1.4, 2.1);
    let m = bed_frame(b, ix, w, l + 0.12, 0.44, 0.85, 0.16);
    // The guard on the right, standing off the frame on two posts,
    // clear of the duvet hanging down.
    let x = m.x1 + 0.13;
    let (ya, yc) = (m.y0 + 0.05, m.y0 + (m.y1 - m.y0) * 0.55);
    fence(b, ix, [x, ya], [x, yc], m.top - 0.02, m.top + 0.2, 0.025);
    for y in [ya + 0.03, yc - 0.03] {
        let role = if ix == 6 { "metal" } else { "frame" };
        b.rbox(
            role,
            [x - 0.012, y - 0.012, m.base - 0.02],
            [x + 0.012, y + 0.012, m.top],
            0.0,
        );
        b.rbox(
            role,
            [m.x1 - 0.01, y - 0.012, m.base - 0.02],
            [x, y + 0.012, m.base + 0.005],
            0.0,
        );
    }
    make_bed(b, p, ix, &m);
    if p.flag("toy", true) {
        let wx = m.x0 + (m.x1 - m.x0) * 0.72;
        toy(b, [wx, m.y0 + 0.45, m.top + 0.07]);
    }
    true
}

/// A soft toy sitting up: a round body, a head with its ears and arms.
fn toy(b: &mut Builder, at: P3) {
    let [x, y, z] = at;
    b.ellipsoid("toy", [x, y, z + 0.07], [0.075, 0.06, 0.085]);
    b.ellipsoid("toy", [x, y + 0.01, z + 0.2], [0.06, 0.055, 0.055]);
    for s in [-1.0, 1.0] {
        b.ellipsoid(
            "toy",
            [x + s * 0.032, y + 0.005, z + 0.29],
            [0.016, 0.01, 0.05],
        );
        b.ellipsoid(
            "toy",
            [x + s * 0.075, y + 0.03, z + 0.08],
            [0.02, 0.035, 0.022],
        );
        b.ellipsoid(
            "toy",
            [x + s * 0.04, y + 0.06, z + 0.012],
            [0.025, 0.045, 0.02],
        );
    }
    b.soft.insert("toy");
}

// ---------------------------------------------------------- the cot

/// The cot of each collection (styles 1 to 8): rails all round a 60 by
/// 120 mattress (`width` across, `depth` long), its long side on the
/// wall, a little quilt and a soft toy.
pub fn crib(b: &mut Builder, p: &Params) {
    let ix = own(p);
    let w = p.get("width", 0.6).clamp(0.5, 0.8);
    let l = p.get("depth", 1.2).clamp(0.9, 1.5);
    let (x0, x1) = (-l / 2.0, l / 2.0);
    let (y0, y1) = (0.04, 0.04 + w);
    let base = 0.34;
    let top = base + 0.12;
    let height = 0.92;
    let s = 0.045;
    // The four posts and the base under the mattress.
    let (px0, px1, py0, py1) = (
        x0 - s / 2.0,
        x1 + s / 2.0,
        y0 - s / 2.0 + 0.01,
        y1 + s / 2.0,
    );
    let raised = matches!(ix, 2 | 7);
    for (q, o) in [[px0, py0], [px1, py0], [px1, py1], [px0, py1]]
        .iter()
        .zip(OUT)
    {
        if raised {
            // Short posts on legs: compass legs or Louis XVI.
            let leg = 0.2;
            if ix == 2 {
                compass_leg(b, *q, leg + 0.005, o, 0.03);
            } else {
                louis_leg(b, "frame", *q, leg + 0.02, 0.024);
            }
            b.rbox(
                "frame",
                [q[0] - s / 2.0, q[1] - s / 2.0, leg],
                [q[0] + s / 2.0, q[1] + s / 2.0, height],
                0.008,
            );
        } else {
            post(b, ix, *q, height, s);
        }
    }
    let role = if ix == 6 { "metal" } else { "frame" };
    b.rbox(role, [x0, y0, base - 0.03], [x1, y1, base], 0.0);
    // Rails round it; the front one lower, to lift the baby out.
    let low = height - 0.12;
    fence(
        b,
        ix,
        [x0 + s / 2.0, py0],
        [x1 - s / 2.0, py0],
        base - 0.04,
        height - 0.02,
        0.022,
    );
    fence(
        b,
        ix,
        [x0 + s / 2.0, py1],
        [x1 - s / 2.0, py1],
        base - 0.04,
        low,
        0.022,
    );
    fence(
        b,
        ix,
        [px0, py0 + s / 2.0],
        [px0, py1 - s / 2.0],
        base - 0.04,
        height - 0.02,
        0.022,
    );
    fence(
        b,
        ix,
        [px1, py0 + s / 2.0],
        [px1, py1 - s / 2.0],
        base - 0.04,
        height - 0.02,
        0.022,
    );
    // The mattress in its sheet, a quilt over its foot half.
    let mut mattress = Soft::new(
        [x0 + 0.01, y0 + 0.01, base],
        [x1 - 0.01, y1 - 0.01, top],
        0.035,
    );
    mattress.crown = 0.004;
    mattress.side = 0.005;
    mattress.seed = 81;
    mattress.add(b, "linen");
    let mut quilt = Cover::new(
        [x0 + 0.42 * l, x1 - 0.03],
        [y0 + 0.03, y1 - 0.03],
        top + 0.025,
        0.025,
    );
    quilt.round = 0.03;
    quilt.falls = [0.06, 0.04, 0.04, 0.04];
    quilt.puff = 0.012;
    quilt.floor = top - 0.03;
    quilt.seed = 82;
    quilt.add(b, "duvet");
    if p.flag("toy", true) {
        toy(b, [x0 + 0.2, (y0 + y1) / 2.0 - 0.02, top]);
    }
}

// ---------------------------------------------------------- the bunk bed

/// The bunk bed of each collection (styles 1 to 8): two beds of `width`
/// by `depth` one over the other on four posts, a guard round the top,
/// the ladder on the right near the foot, both made up.
pub fn bunk_bed(b: &mut Builder, p: &Params) {
    let ix = own(p);
    let w = p.get("width", 0.9).clamp(0.7, 1.2);
    let l = p.get("depth", 1.9).clamp(1.6, 2.1);
    let tall = p.get("height", 1.75).clamp(1.5, 2.0);
    let (x0, x1) = (-w / 2.0, w / 2.0);
    let s = if ix == 6 { 0.05 } else { 0.07 };
    let (px0, px1) = (x0 - s / 2.0 - 0.01, x1 + s / 2.0 + 0.01);
    let (py0, py1) = (s / 2.0, l + 0.07 - s / 2.0);
    for q in [[px0, py0], [px1, py0], [px1, py1], [px0, py1]] {
        post(b, ix, q, tall, s);
    }
    let role = if ix == 6 { "metal" } else { "frame" };
    let mattress = 0.15;
    let lower = 0.22;
    let upper = (tall - 0.55).max(lower + 0.85);
    for z in [lower, upper] {
        // The side rails and the base boards.
        for x in [px0, px1] {
            b.rbox(
                role,
                [x - 0.018, py0 + s / 2.0, z - 0.1],
                [x + 0.018, py1 - s / 2.0, z],
                0.0,
            );
        }
        b.rbox(role, [x0, py0, z - 0.03], [x1, py1, z], 0.0);
        // Head and foot.
        for y in [py0, py1] {
            fence(
                b,
                ix,
                [px0 + s / 2.0, y],
                [px1 - s / 2.0, y],
                z - 0.1,
                z + 0.32,
                0.025,
            );
        }
    }
    // The guard round the top: all along the left, the right but for
    // the ladder's opening.
    let gz0 = upper + mattress - 0.02;
    let gz1 = upper + mattress + 0.2;
    let open = 0.45;
    fence(
        b,
        ix,
        [px0, py0 + s / 2.0],
        [px0, py1 - s / 2.0],
        gz0,
        gz1,
        0.025,
    );
    fence(
        b,
        ix,
        [px1, py0 + s / 2.0],
        [px1, py1 - s / 2.0 - open - 0.04],
        gz0,
        gz1,
        0.025,
    );
    // The ladder, against the right side near the foot.
    let (la, lc) = (py1 - s / 2.0 - open, py1 - s / 2.0 - 0.04);
    let lx = px1 + 0.03;
    for y in [la, lc] {
        match ix {
            6 => b.cyl("metal", [lx, y, 0.0], [lx, y, gz1], 0.018),
            _ => b.rbox(
                role,
                [lx - 0.02, y - 0.02, 0.0],
                [lx + 0.02, y + 0.02, gz1],
                0.004,
            ),
        }
    }
    let n = ((gz0 / 0.28).round() as usize).max(3);
    for k in 1..=n {
        let z = gz0 * k as f64 / (n + 1) as f64 + 0.05;
        match ix {
            1 | 5 => b.rbox(
                role,
                [lx - 0.02, la, z - 0.015],
                [lx + 0.02, lc, z + 0.015],
                0.0,
            ),
            6 => b.cyl("metal", [lx, la, z], [lx, lc, z], 0.012),
            _ => b.cyl(role, [lx, la, z], [lx, lc, z], 0.014),
        }
    }
    // Both beds made up.
    let mut q = p.clone();
    q.set("spread", 0.0);
    for (z, k) in [(lower, 1.0), (upper, 0.0)] {
        q.set("cushions", k);
        let m = super::bedroom::Making {
            x0: x0 + 0.01,
            x1: x1 - 0.01,
            y0: py0 + s / 2.0 + 0.005,
            y1: py1 - s / 2.0 - 0.005,
            base: z,
            top: z + mattress,
            floor: z - 0.05,
            foot: false,
            sides: false,
        };
        make_bed(b, &q, ix, &m);
    }
}

// ---------------------------------------------------------- the valet stand

/// The valet stand of each collection (styles 1 to 8): a shoulder hanger,
/// a bar for the trousers and something to stand or sit on; trousers
/// over its bar (`clothes`).
pub fn valet(b: &mut Builder, p: &Params) {
    let ix = own(p);
    let h = p.get("height", 1.1).clamp(0.9, 1.3);
    let hw = 0.22;
    let bar_z = h * 0.6;
    // The hanger at the top: a curved bar.
    let hanger = |b: &mut Builder, role: &'static str, y: f64| {
        let pts = curve(
            &[
                [-hw, y, h - 0.06],
                [-hw * 0.5, y, h - 0.02],
                [0.0, y, h],
                [hw * 0.5, y, h - 0.02],
                [hw, y, h - 0.06],
            ],
            6,
        );
        sweep(
            b,
            role,
            &pts,
            |_| 0.014,
            10,
            Cap::Round(1.0),
            Cap::Round(1.0),
        );
    };
    let y = 0.2;
    match ix {
        // A post on three splayed legs (oak, teak with brass shoes).
        0 | 2 => {
            let foot = 0.32;
            for k in 0..3 {
                let a = PI / 2.0 + TAU * k as f64 / 3.0;
                let (s, c) = a.sin_cos();
                if ix == 2 {
                    compass_leg(b, [0.0, y], foot, [c, s], 0.2);
                } else {
                    tapered_leg(b, "frame", [0.0, y], foot, [c, s], 0.2, 0.017);
                }
            }
            turned(
                b,
                "frame",
                [0.0, y, foot - 0.03],
                &[
                    (0.0, 0.0),
                    (0.03, 0.0),
                    (0.02, 0.03),
                    (0.018, h - foot),
                    (0.0, h - foot + 0.01),
                ],
                16,
                0.0,
            );
            hanger(b, "frame", y);
            b.cyl("frame", [-hw * 0.9, y, bar_z], [hw * 0.9, y, bar_z], 0.012);
            // A little round tray for the watch and the keys.
            turned(
                b,
                "frame",
                [0.0, y, foot + 0.08],
                &[
                    (0.0, 0.0),
                    (0.12, 0.0),
                    (0.12, 0.02),
                    (0.11, 0.022),
                    (0.11, 0.008),
                    (0.0, 0.008),
                ],
                24,
                0.0,
            );
        }
        // A chair to sit on and leave the clothes: whitewashed with rush
        // (mediterranean), or Louis XVI, buttoned (classic).
        1 | 7 => {
            let (sw, sd, sz) = (0.42, 0.38, 0.45);
            let (x0, x1) = (-sw / 2.0, sw / 2.0);
            if ix == 7 {
                for q in corners(x0, x1, sd, 0.025) {
                    louis_leg(b, "frame", [q[0], q[1] + y - sd / 2.0], sz - 0.06, 0.02);
                }
                let mut s = Soft::new([x0, y - sd / 2.0, sz - 0.06], [x1, y + sd / 2.0, sz], 0.025);
                s.crown = 0.012;
                s.tufts = 0.12;
                s.tuft_depth = 0.008;
                s.step = 0.03;
                s.seed = 51;
                s.add(b, "cushion");
            } else {
                let start = b.mark();
                square_legs(b, "frame", x0, x1, sd, sz, 0.035);
                b.transform_from(start, |q| [q[0], q[1] + y - sd / 2.0, q[2]]);
                let mut rush = Soft::new(
                    [x0 + 0.02, y - sd / 2.0 + 0.02, sz - 0.03],
                    [x1 - 0.02, y + sd / 2.0 - 0.02, sz + 0.004],
                    0.01,
                );
                rush.crown = 0.004;
                rush.seed = 52;
                rush.add_faces(b, "weave", [true; 6]);
            }
            // Its back: two uprights up to the hanger.
            for x in [x0 + 0.02, x1 - 0.02] {
                if ix == 7 {
                    b.cyl(
                        "frame",
                        [x, y - sd / 2.0 + 0.02, sz],
                        [x, y - sd / 2.0 + 0.02, h - 0.04],
                        0.016,
                    );
                } else {
                    b.rbox(
                        "frame",
                        [x - 0.017, y - sd / 2.0, sz],
                        [x + 0.017, y - sd / 2.0 + 0.035, h - 0.04],
                        0.0,
                    );
                }
            }
            hanger(b, "frame", y - sd / 2.0 + 0.02);
            b.cyl(
                "frame",
                [x0 + 0.02, y - sd / 2.0 + 0.02, bar_z + 0.1],
                [x1 - 0.02, y - sd / 2.0 + 0.02, bar_z + 0.1],
                0.012,
            );
        }
        // Japandi: an oak frame, two uprights on feet and rails.
        3 => {
            for x in [-hw, hw] {
                b.rbox(
                    "frame",
                    [x - 0.016, y - 0.012, 0.03],
                    [x + 0.016, y + 0.012, h - 0.06],
                    0.0,
                );
                b.rbox(
                    "frame",
                    [x - 0.016, y - 0.17, 0.0],
                    [x + 0.016, y + 0.17, 0.03],
                    0.0,
                );
            }
            for z in [bar_z, h - 0.06] {
                b.rbox(
                    "frame",
                    [-hw - 0.016, y - 0.012, z - 0.03],
                    [hw + 0.016, y + 0.012, z],
                    0.0,
                );
            }
            b.rbox("frame", [-hw, y - 0.12, 0.28], [hw, y + 0.12, 0.3], 0.0);
        }
        // Contemporary: a black steel rod bent into a frame, on a disc.
        4 => {
            turned(
                b,
                "metal",
                [0.0, y, 0.0],
                &[(0.0, 0.0), (0.17, 0.0), (0.17, 0.012), (0.0, 0.016)],
                32,
                0.0,
            );
            b.cyl("metal", [0.0, y, 0.0], [0.0, y, h - 0.04], 0.012);
            hanger(b, "metal", y);
            b.cyl("metal", [-hw * 0.8, y, bar_z], [hw * 0.8, y, bar_z], 0.009);
        }
        // Rustic: a turned pine post on a cross of feet, iron hooks.
        5 => {
            for (dx, dy) in [(1.0, 0.0), (0.0, 1.0)] {
                b.rbox(
                    "frame",
                    [-0.18 * dx - 0.025 * dy, y - 0.18 * dy - 0.025 * dx, 0.0],
                    [0.18 * dx + 0.025 * dy, y + 0.18 * dy + 0.025 * dx, 0.05],
                    0.004,
                );
            }
            turned_post(b, "frame", [0.0, y], 0.05, h - 0.02, 0.03);
            b.cyl(
                "frame",
                [-hw * 0.9, y, h - 0.08],
                [hw * 0.9, y, h - 0.08],
                0.018,
            );
            b.cyl("frame", [-hw * 0.9, y, bar_z], [hw * 0.9, y, bar_z], 0.015);
            for s in [-1.0, 1.0] {
                let pts: Vec<P3> = (0..=6)
                    .map(|k| {
                        let t = PI * k as f64 / 6.0;
                        [
                            s * 0.035,
                            y + 0.03 - 0.03 * t.cos(),
                            h - 0.25 + 0.03 * t.sin(),
                        ]
                    })
                    .collect();
                tube(b, "metal", &pts, false, 0.004, 6);
            }
        }
        // Industrial: steel pipe on flanges, a shelf of wood.
        _ => {
            for x in [-hw, hw] {
                b.cyl("metal", [x, y, 0.0], [x, y, 0.012], 0.04);
                b.cyl("metal", [x, y, 0.0], [x, y, h - 0.04], 0.014);
                b.ellipsoid("metal", [x, y, h - 0.04], [0.022, 0.022, 0.022]);
            }
            b.cyl("metal", [-hw, y, h - 0.04], [hw, y, h - 0.04], 0.014);
            b.cyl("metal", [-hw, y, bar_z], [hw, y, bar_z], 0.012);
            b.rbox(
                "frame",
                [-hw - 0.02, y - 0.15, 0.25],
                [hw + 0.02, y + 0.15, 0.275],
                0.003,
            );
            for x in [-hw, hw] {
                b.cyl("metal", [x, y - 0.13, 0.25], [x, y + 0.13, 0.25], 0.008);
            }
        }
    }
    // Trousers folded over the bar.
    if p.flag("clothes", true) {
        let (by, bz) = match ix {
            1 | 7 => (y - 0.19 + 0.02, bar_z + 0.1),
            _ => (y, bar_z),
        };
        let mut t = Cover::new([-0.17, 0.17], [by - 0.012, by + 0.012], bz + 0.024, 0.012);
        t.round = 0.022;
        t.falls = [0.0, 0.0, 0.38, 0.32];
        t.folds = 0.006;
        t.flare = 0.01;
        t.floor = 0.1;
        t.seed = 55 + ix as u32;
        t.add(b, "clothes");
    }
}

// ---------------------------------------------------------- the standing mirror

/// The standing mirror of each collection (styles 1 to 8): most lean on
/// the wall; the fifties' stands on its easel and the classic one swings
/// between two posts. `width` and `height`.
pub fn floor_mirror(b: &mut Builder, p: &Params) {
    let ix = own(p);
    let w = p.get("width", 0.6).clamp(0.35, 1.2);
    let h = p.get("height", 1.7).clamp(1.0, 2.2);
    let (x0, x1) = (-w / 2.0, w / 2.0);
    // Built standing up on the wall's face, then leant `tilt`.
    let tilt: f64 = 0.09;
    let start = b.mark();
    let t = 0.025;
    match ix {
        1 => {
            let zs = h - w / 2.0;
            for x in [x0, x1 - 0.05] {
                b.rbox("frame", [x, 0.0, 0.0], [x + 0.05, t, zs], 0.0);
            }
            b.rbox("frame", [x0, 0.0, 0.0], [x1, t, 0.06], 0.0);
            arch_band(b, "frame", x0, x1, zs, h, 0.05, 0.0, t);
            arched_board(
                b,
                "mirror_glass",
                x0 + 0.045,
                x1 - 0.045,
                0.055,
                zs,
                h - 0.045,
                0.006,
                0.016,
            );
        }
        2 => {
            // An oval in a teak ring, on an easel of three legs.
            let (a, c) = (w / 2.0, h * 0.42);
            let zc = h - c - 0.05;
            let ring: Vec<P3> = (0..48)
                .map(|k| {
                    let u = TAU * k as f64 / 48.0;
                    [a * u.cos(), 0.35, zc + c * u.sin()]
                })
                .collect();
            tube(b, "frame", &ring, true, 0.018, 10);
            let start = b.mark();
            turned(
                b,
                "mirror_glass",
                [0.0; 3],
                &[(0.0, 0.0), (1.0, 0.0), (1.0, 0.012), (0.0, 0.012)],
                48,
                0.0,
            );
            b.transform_from(start, |q| {
                [q[0] * (a - 0.01), 0.342 + q[2], zc - q[1] * (c - 0.01)]
            });
            for x in [x0 + 0.08, x1 - 0.08] {
                compass_leg(b, [x * 0.6, 0.36], zc - c * 0.3, [x.signum(), 0.4], 0.12);
            }
            b.cyl(
                "frame",
                [0.0, 0.33, zc + c * 0.5],
                [0.0, 0.05, 0.006],
                0.012,
            );
            // The easel stands: not leant.
            return;
        }
        3 => {
            let f = 0.03;
            frame_rect(b, "frame", x0, x1, 0.0, h, f, t);
            b.rbox(
                "mirror_glass",
                [x0 + f - 0.004, 0.004, f - 0.004],
                [x1 - f + 0.004, 0.014, h - f + 0.004],
                0.0,
            );
        }
        4 => {
            // A thin black frame, its top corners rounded.
            let f = 0.012;
            let r = 0.12_f64.min(w / 2.0);
            let pts: Vec<P3> = {
                let mut v = vec![[x1, t / 2.0, f], [x1, t / 2.0, h - r]];
                for k in 1..8 {
                    let a = FRAC_PI_2 * k as f64 / 8.0;
                    v.push([x1 - r + r * a.cos(), t / 2.0, h - r + r * a.sin()]);
                }
                v.push([x1 - r, t / 2.0, h]);
                v.push([x0 + r, t / 2.0, h]);
                for k in 1..8 {
                    let a = FRAC_PI_2 + FRAC_PI_2 * k as f64 / 8.0;
                    v.push([x0 + r + r * a.cos(), t / 2.0, h - r + r * a.sin()]);
                }
                v.push([x0, t / 2.0, h - r]);
                v.push([x0, t / 2.0, f]);
                v
            };
            tube(b, "metal", &pts, true, f, 6);
            let poly =
                super::shape::rounded_rect([0.0, h / 2.0], w / 2.0 - f, h / 2.0 - f, r - f, 0.0, 8);
            let mut q: Vec<[f64; 2]> = poly.iter().map(|q| [q[0], q[1]]).collect();
            // Its foot is square: no rounding below.
            for p in q.iter_mut() {
                if p[1] < h / 2.0 {
                    p[0] = p[0].signum() * (w / 2.0 - f);
                    p[1] = p[1].max(f);
                }
            }
            q.dedup_by(|a, b| (a[0] - b[0]).abs() < 1e-9 && (a[1] - b[1]).abs() < 1e-9);
            let s = b.mark();
            b.slab("mirror_glass", &q, 0.006, 0.016);
            b.transform_from(s, |q| [q[0], q[2], q[1]]);
            mirror_fix(b, s);
        }
        5 => {
            let f = 0.075;
            frame_rect(b, "frame", x0, x1, 0.0, h, f, 0.035);
            b.rbox(
                "mirror_glass",
                [x0 + f - 0.005, 0.006, f - 0.005],
                [x1 - f + 0.005, 0.016, h - f + 0.005],
                0.0,
            );
        }
        6 => {
            let f = 0.03;
            frame_rect(b, "metal", x0, x1, 0.0, h, f, t);
            b.rbox(
                "mirror_glass",
                [x0 + f - 0.004, 0.004, f - 0.004],
                [x1 - f + 0.004, 0.014, h - f + 0.004],
                0.0,
            );
            for x in [x0 + f / 2.0, x1 - f / 2.0] {
                for k in 0..6 {
                    let z = 0.1 + (h - 0.2) * k as f64 / 5.0;
                    b.ellipsoid("metal", [x, t + 0.002, z], [0.005, 0.003, 0.005]);
                }
            }
        }
        7 => {
            // A cheval glass: the mirror swings between two posts on
            // splayed feet.
            let (mw, mh) = (w - 0.12, h - 0.45);
            let z0 = 0.3;
            let y = 0.25;
            let f = 0.045;
            let start2 = b.mark();
            frame_rect(b, "frame", -mw / 2.0, mw / 2.0, 0.0, mh, f, 0.03);
            b.rbox(
                "mirror_glass",
                [-mw / 2.0 + f - 0.004, 0.005, f - 0.004],
                [mw / 2.0 - f + 0.004, 0.015, mh - f + 0.004],
                0.0,
            );
            let pts = curve(
                &[
                    [-mw / 2.0 + 0.02, 0.02, mh],
                    [-mw * 0.2, 0.02, mh + 0.03],
                    [0.0, 0.02, mh + 0.11],
                    [mw * 0.2, 0.02, mh + 0.03],
                    [mw / 2.0 - 0.02, 0.02, mh],
                ],
                6,
            );
            sweep(
                b,
                "frame",
                &pts,
                |_| 0.016,
                10,
                Cap::Round(1.0),
                Cap::Round(1.0),
            );
            // Swung back a little, about its pivots.
            let a: f64 = 0.06;
            let (sa, ca) = a.sin_cos();
            let pivot = mh * 0.55;
            b.transform_from(start2, |q| {
                let (yy, zz) = (q[1], q[2] - pivot);
                [q[0], y + yy * ca - zz * sa, z0 + pivot + yy * sa + zz * ca]
            });
            for x in [x0 + 0.02, x1 - 0.02] {
                turned_post(b, "frame", [x, y + 0.015], 0.05, z0 + mh * 0.75, 0.022);
                b.rbox(
                    "frame",
                    [x - 0.025, y - 0.2, 0.0],
                    [x + 0.025, y + 0.22, 0.05],
                    0.01,
                );
                b.cyl(
                    "metal",
                    [x, y + 0.015, z0 + pivot],
                    [x * 0.85, y + 0.015, z0 + pivot],
                    0.01,
                );
            }
            b.cyl(
                "frame",
                [x0 + 0.02, y + 0.015, 0.2],
                [x1 - 0.02, y + 0.015, 0.2],
                0.015,
            );
            return;
        }
        // Nordic: an oak frame with soft corners and a leather strap.
        _ => {
            let f = 0.04;
            frame_rect(b, "frame", x0, x1, 0.0, h, f, t);
            b.rbox(
                "mirror_glass",
                [x0 + f - 0.004, 0.004, f - 0.004],
                [x1 - f + 0.004, 0.014, h - f + 0.004],
                0.0,
            );
            b.rbox(
                "dark",
                [-0.02, t, h - 0.25],
                [0.02, t + 0.004, h - 0.02],
                0.0,
            );
        }
    }
    // Leant against the wall: its foot comes forward.
    let (st, ct) = tilt.sin_cos();
    b.transform_from(start, |q| {
        [q[0], q[1] * ct - q[2] * st + h * st, q[1] * st + q[2] * ct]
    });
}

/// A rectangular frame from `x0` to `x1` and `z0` to `z1` of bars `f`
/// wide and `t` deep (its back on `y = 0`).
#[allow(clippy::too_many_arguments)]
fn frame_rect(
    b: &mut Builder,
    role: &'static str,
    x0: f64,
    x1: f64,
    z0: f64,
    z1: f64,
    f: f64,
    t: f64,
) {
    for (lo, hi) in [
        ([x0, 0.0, z0], [x1, t, z0 + f]),
        ([x0, 0.0, z1 - f], [x1, t, z1]),
        ([x0, 0.0, z0 + f], [x0 + f, t, z1 - f]),
        ([x1 - f, 0.0, z0 + f], [x1, t, z1 - f]),
    ] {
        b.rbox(role, lo, hi, 0.003);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fine::bedroom::COLLECTIONS;
    use crate::fine::shape::signed_volume;

    #[test]
    fn the_childrens_pieces_of_the_eight_collections() {
        type Make = Box<dyn Fn(&mut Builder, &Params)>;
        for c in 0..8 {
            let pieces: [(&str, Make, i32); 5] = [
                ("cama infantil", Box::new(|b, p| assert!(kids_bed(b, p))), 2),
                ("cuna", Box::new(crib), 0),
                ("litera", Box::new(bunk_bed), 0),
                ("galán", Box::new(valet), 0),
                ("espejo de pie", Box::new(floor_mirror), 0),
            ];
            for (name, make, first) in pieces {
                let mut p = Params::default();
                p.set("style", (first + 1 + c) as f64);
                let mut b = Builder::default();
                make(&mut b, &p);
                let tris = b.triangle_count();
                let col = COLLECTIONS[c as usize];
                assert!((50..40_000).contains(&tris), "{col} {name}: {tris}");
                for (role, t) in &b.parts {
                    let v = signed_volume(&b.positions, t);
                    assert!(v > -1e-7, "{col} {name} {role}: {v}");
                }
                for q in &b.positions {
                    assert!(q.iter().all(|v| v.is_finite()), "{col} {name}");
                    assert!(q[2] > -0.003, "{col} {name}: {q:?}");
                }
            }
        }
    }
}
