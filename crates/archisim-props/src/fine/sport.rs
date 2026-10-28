//! Sport at home and in the gym (paquete 8, tanda 8b): one good model of
//! each piece, its colours by its materials (no collections).
//!
//! Garden and home: basketball hoop, football goal, table tennis table and
//! trampoline. Gym: treadmill, exercise bike, weight bench with its rack
//! and bar, dumbbell rack, wall bars and a yoga mat with its blocks.
//!
//! Each is centred on `x = 0` with its back at `y = 0` (the goal's mouth,
//! the treadmill's console, the bench's foot face `+y`). Roles: `frame`
//! (the painted steel or the wood), `dark` (black steel and rubber),
//! `metal` (bright steel), `mat` (upholstery, the trampoline's bed, the
//! belt), `accent` (the colour: the hoop's rim, the trampoline's pads,
//! the blocks), `plate` (rubber weights), `board` and `line` (the
//! backboard, the table's top and its white lines), `net` and `display`.

use super::hospitality::{disc, ring, rod};
use super::shape::{curve, sweep, Cap};
use super::soft::{tube, Soft};
use crate::build::{Builder, P3};
use crate::Params;
use std::f64::consts::{PI, TAU};

/// A thin strand of four sides from `a` to `c` (a net's cord).
pub(super) fn cord(b: &mut Builder, role: &'static str, a: P3, c: P3, r: f64) {
    sweep(b, role, &[a, c], |_| r, 4, Cap::Flat, Cap::Flat);
}

pub(super) fn lerp(a: P3, c: P3, t: f64) -> P3 {
    [
        a[0] + (c[0] - a[0]) * t,
        a[1] + (c[1] - a[1]) * t,
        a[2] + (c[2] - a[2]) * t,
    ]
}

/// A net over the four-sided patch `p00 p10 p11 p01`: cords from side to
/// side both ways, `mesh` apart (at most).
pub(super) fn net(b: &mut Builder, role: &'static str, q: [P3; 4], mesh: f64) {
    let len = |a: P3, c: P3| {
        ((a[0] - c[0]).powi(2) + (a[1] - c[1]).powi(2) + (a[2] - c[2]).powi(2)).sqrt()
    };
    let [p00, p10, p11, p01] = q;
    let nu = (len(p00, p10).max(len(p01, p11)) / mesh).ceil().max(1.0) as usize;
    let nv = (len(p00, p01).max(len(p10, p11)) / mesh).ceil().max(1.0) as usize;
    for i in 1..nu {
        let t = i as f64 / nu as f64;
        cord(b, role, lerp(p00, p10, t), lerp(p01, p11, t), 0.003);
    }
    for j in 1..nv {
        let t = j as f64 / nv as f64;
        cord(b, role, lerp(p00, p01, t), lerp(p10, p11, t), 0.003);
    }
}

/// A basketball hoop on a weighted stand: `rim_height` (3.05 m, lower
/// for children) and `reach`, from the pole to the board (1,15 m; a
/// court's stand reaches further, behind its baseline).
pub fn basketball_hoop(b: &mut Builder, p: &Params) {
    let h = p.get("rim_height", 3.05).clamp(2.0, 3.05);
    // The base, filled with water or sand, and its wheels at the back.
    b.rbox("dark", [-0.42, 0.0, 0.0], [0.42, 1.05, 0.26], 0.06);
    for x in [-0.3, 0.3] {
        let s = b.mark();
        disc(b, "dark", [0.0, 0.0], 0.07, -0.025, 0.05);
        b.transform_from(s, |q| [x + q[2], 0.02 - q[1], 0.07 + q[0]]);
    }
    // The pole in three sections, each narrower.
    let top = h + 0.25;
    let (py, sec) = (0.45, (top - 0.26) / 3.0);
    for k in 0..3 {
        let r = 0.055 - 0.008 * k as f64;
        let z0 = 0.26 + sec * k as f64 - if k > 0 { 0.08 } else { 0.0 };
        b.rbox(
            "frame",
            [-r, py - r, z0],
            [r, py + r, 0.26 + sec * (k + 1) as f64],
            0.008,
        );
    }
    // The pad round the foot of the pole.
    b.rbox(
        "accent",
        [-0.09, py - 0.09, 0.26],
        [0.09, py + 0.09, 1.2],
        0.03,
    );
    // Two arms out to the board, and a brace.
    let by = p.get("reach", 1.15).clamp(0.9, 2.5);
    for x in [-0.22, 0.22] {
        rod(b, "frame", [x, py, top - 0.05], [x, by, top - 0.05], 0.018);
        rod(b, "frame", [x, py, top - 0.45], [x, by, top - 0.15], 0.018);
    }
    rod(
        b,
        "frame",
        [-0.24, py, top - 0.05],
        [0.24, py, top - 0.05],
        0.02,
    );
    rod(
        b,
        "frame",
        [-0.24, py, top - 0.45],
        [0.24, py, top - 0.45],
        0.02,
    );
    // The board, its frame and its lines.
    let (bx, bz0, bz1) = (0.6, h - 0.15, h + 0.6);
    b.rbox("board", [-bx, by, bz0], [bx, by + 0.02, bz1], 0.005);
    for (lo, hi) in [
        ([-bx - 0.02, bz0 - 0.02], [bx + 0.02, bz0 + 0.02]),
        ([-bx - 0.02, bz1 - 0.02], [bx + 0.02, bz1 + 0.02]),
        ([-bx - 0.02, bz0], [-bx + 0.02, bz1]),
        ([bx - 0.02, bz0], [bx + 0.02, bz1]),
    ] {
        b.rbox(
            "frame",
            [lo[0], by - 0.01, lo[1]],
            [hi[0], by + 0.03, hi[1]],
            0.0,
        );
    }
    let fy = by + 0.0205;
    let (sx, sz0, sz1) = (0.295, h, h + 0.45);
    for (lo, hi) in [
        ([-sx, sz0], [sx, sz0 + 0.05]),
        ([-sx, sz1 - 0.05], [sx, sz1]),
        ([-sx, sz0], [-sx + 0.05, sz1]),
        ([sx - 0.05, sz0], [sx, sz1]),
    ] {
        b.rbox("line", [lo[0], fy, lo[1]], [hi[0], fy + 0.003, hi[1]], 0.0);
    }
    // The rim, its bracket and the net.
    let (rr, cy) = (0.23, by + 0.02 + 0.15 + 0.23);
    tube(b, "accent", &ring([0.0, cy], rr, h, 20), true, 0.009, 8);
    b.rbox(
        "accent",
        [-0.08, by + 0.02, h - 0.1],
        [0.08, cy - rr + 0.01, h],
        0.005,
    );
    let n = 12;
    let (low, rl) = (h - 0.42, 0.14);
    let mid = h - 0.2;
    for k in 0..n {
        let a = TAU * k as f64 / n as f64;
        let at = |r: f64, z: f64, da: f64| [(r * (a + da).cos()), cy + r * (a + da).sin(), z];
        let half = PI / n as f64;
        cord(b, "net", at(rr, h - 0.005, 0.0), at(0.18, mid, half), 0.004);
        cord(
            b,
            "net",
            at(rr, h - 0.005, 0.0),
            at(0.18, mid, -half),
            0.004,
        );
        cord(b, "net", at(0.18, mid, half), at(rl, low, 0.0), 0.004);
        cord(
            b,
            "net",
            at(0.18, mid, half),
            at(rl, low, 2.0 * half),
            0.004,
        );
    }
    tube(b, "net", &ring([0.0, cy], rl, low, 16), true, 0.004, 4);
}

/// A football goal: `width`, `height` and `depth`; its mouth faces `+y`
/// (the posts at `y = depth`), the net back to the ground at `y = 0`.
pub fn goal(b: &mut Builder, p: &Params) {
    let w = p.get("width", 3.0).clamp(1.2, 7.32);
    let h = p.get("height", 2.0).clamp(0.8, 2.44);
    let d = p.get("depth", 1.2).clamp(0.6, 2.0);
    let r = if w > 5.0 { 0.06 } else { 0.04 };
    let x = w / 2.0 + r;
    let top = 0.6_f64.min(d * 0.5);
    let yt = d - top;
    // The posts and crossbar, round and white.
    sweep(
        b,
        "frame",
        &[[-x, d, 0.0], [-x, d, h + r], [x, d, h + r], [x, d, 0.0]],
        |_| r,
        12,
        Cap::Flat,
        Cap::Flat,
    );
    // The frame behind: on the ground, up the back and across the top.
    let t = 0.02;
    for s in [-1.0, 1.0] {
        let xs = s * x;
        sweep(
            b,
            "frame",
            &[[xs, d - r, t], [xs, 0.0, t], [xs, yt, h], [xs, d - r, h]],
            |_| t,
            8,
            Cap::Flat,
            Cap::Flat,
        );
    }
    rod(b, "frame", [-x, 0.0, t], [x, 0.0, t], t);
    rod(b, "frame", [-x, yt, h], [x, yt, h], t);
    // The net: back, top and sides.
    let m = 0.12;
    let (g0, g1) = (0.004, 0.006);
    net(
        b,
        "net",
        [[-x, 0.0, g0], [x, 0.0, g0], [x, yt, h], [-x, yt, h]],
        m,
    );
    net(
        b,
        "net",
        [
            [-x, yt, h + g1],
            [x, yt, h + g1],
            [x, d, h + g1],
            [-x, d, h + g1],
        ],
        m,
    );
    for s in [-1.0, 1.0] {
        let xs = s * (x - 0.004);
        net(
            b,
            "net",
            [[xs, 0.0, g0], [xs, d, g0], [xs, d, h], [xs, yt, h]],
            m,
        );
    }
}

/// A table tennis table, folding, on wheels: 2.74 × 1.525 m at 76 cm.
pub fn ping_pong(b: &mut Builder, _p: &Params) {
    let (l, w, h) = (2.74, 1.525, 0.76);
    let (x1, y1) = (l / 2.0, w);
    let t = 0.025;
    // The two halves of the top, its white edges and middle line.
    for s in [-1.0, 1.0] {
        let (a, c) = if s < 0.0 { (-x1, -0.004) } else { (0.004, x1) };
        b.rbox("board", [a, 0.0, h - t], [c, y1, h], 0.003);
    }
    let z = h + 0.0006;
    let e = 0.02;
    for (lo, hi) in [
        ([-x1, 0.0], [x1, e]),
        ([-x1, y1 - e], [x1, y1]),
        ([-x1, 0.0], [-x1 + e, y1]),
        ([x1 - e, 0.0], [x1, y1]),
        ([-x1, w / 2.0 - 0.0015], [x1, w / 2.0 + 0.0015]),
    ] {
        b.rbox("line", [lo[0], lo[1], h - 0.001], [hi[0], hi[1], z], 0.0);
    }
    // The net on its two posts, its top a white band.
    let nh = 0.1525;
    b.rbox(
        "net",
        [-0.002, -0.14, h + 0.01],
        [0.002, y1 + 0.14, h + nh - 0.012],
        0.0,
    );
    b.rbox(
        "line",
        [-0.004, -0.14, h + nh - 0.014],
        [0.004, y1 + 0.14, h + nh],
        0.0,
    );
    for y in [-0.16, y1 + 0.16] {
        b.rbox(
            "dark",
            [-0.012, y - 0.012, h - 0.04],
            [0.012, y + 0.012, h + nh],
            0.004,
        );
        b.rbox(
            "dark",
            [-0.03, y - 0.03, h - 0.06],
            [0.03, y + 0.03, h - 0.02],
            0.004,
        );
    }
    // Under each half: an apron, two leg frames, a bar between, wheels.
    for s in [-1.0, 1.0] {
        let xin = s * 0.12;
        let xout = s * (x1 - 0.25);
        b.rbox(
            "frame",
            [xin.min(xout), 0.06, h - t - 0.05],
            [xin.max(xout), 0.09, h - t],
            0.0,
        );
        b.rbox(
            "frame",
            [xin.min(xout), y1 - 0.09, h - t - 0.05],
            [xin.max(xout), y1 - 0.06, h - t],
            0.0,
        );
        for y in [0.25, y1 - 0.25] {
            rod(b, "frame", [xout, y, 0.1], [xout, y, h - t - 0.04], 0.022);
            rod(b, "frame", [xin, y, 0.1], [xin, y, h - t - 0.04], 0.022);
            // The wheel under each leg.
            for xl in [xout, xin] {
                let st = b.mark();
                disc(b, "dark", [0.0, 0.0], 0.05, -0.015, 0.03);
                b.transform_from(st, |q| [xl + q[2], y - q[1], 0.05 + q[0]]);
                b.rbox(
                    "frame",
                    [xl - 0.025, y - 0.02, 0.05],
                    [xl + 0.025, y + 0.02, 0.11],
                    0.005,
                );
            }
        }
        rod(b, "frame", [xout, 0.25, 0.3], [xout, y1 - 0.25, 0.3], 0.016);
        rod(b, "frame", [xin, 0.25, 0.3], [xin, y1 - 0.25, 0.3], 0.016);
        rod(b, "frame", [xin, w / 2.0, 0.3], [xout, w / 2.0, 0.3], 0.016);
    }
}

/// A round trampoline with its safety net: `diameter`.
pub fn trampoline(b: &mut Builder, p: &Params) {
    let dia = p.get("diameter", 3.05).clamp(1.5, 4.3);
    let rf = dia / 2.0;
    let c = [0.0, rf];
    let zf = (0.45 + 0.1 * rf).min(0.85);
    // The frame ring and the bed hung inside it on its springs.
    tube(b, "frame", &ring(c, rf, zf, 48), true, 0.022, 8);
    let rm = rf - 0.2;
    disc(b, "mat", c, rm, zf - 0.03, 0.01);
    let n = (rf * 22.0).round() as usize;
    for k in 0..n {
        let a = TAU * (k as f64 + 0.5) / n as f64;
        let (co, si) = (a.cos(), a.sin());
        cord(
            b,
            "metal",
            [c[0] + rm * co, c[1] + rm * si, zf - 0.025],
            [c[0] + (rf - 0.02) * co, c[1] + (rf - 0.02) * si, zf - 0.005],
            0.006,
        );
    }
    // The pad over the springs.
    let pad = (rf - 0.1, 0.11);
    let st = b.mark();
    tube(b, "accent", &ring(c, pad.0, 0.0, 48), true, pad.1, 8);
    b.transform_from(st, |q| [q[0], q[1], zf + 0.035 + (q[2] - 0.0) * 0.3]);
    // Legs: W-shaped, a pair of posts on a foot rail each.
    let legs = if rf > 1.6 { 6 } else { 4 };
    for k in 0..legs {
        let a = TAU * k as f64 / legs as f64;
        let at =
            |r: f64, da: f64, z: f64| [c[0] + r * (a + da).cos(), c[1] + r * (a + da).sin(), z];
        let da = 0.16 / rf * 1.5;
        let foot = curve(
            &[
                at(rf, -da, 0.03),
                at(rf + 0.06, 0.0, 0.03),
                at(rf, da, 0.03),
            ],
            4,
        );
        sweep(
            b,
            "frame",
            &foot,
            |_| 0.02,
            8,
            Cap::Round(1.0),
            Cap::Round(1.0),
        );
        for s in [-da, da] {
            rod(b, "frame", at(rf, s, 0.03), at(rf, s, zf - 0.02), 0.02);
        }
        // The net's pole from the leg, curving out and up, padded.
        let top = zf + 1.8;
        let pole = curve(
            &[
                at(rf + 0.02, 0.0, zf - 0.05),
                at(rf + 0.05, 0.0, zf + 0.4),
                at(rf + 0.02, 0.0, top - 0.3),
                at(rf - 0.08, 0.0, top),
            ],
            6,
        );
        sweep(
            b,
            "accent",
            &pole,
            |_| 0.03,
            10,
            Cap::Round(1.0),
            Cap::Round(1.0),
        );
    }
    // The net: a cylinder of cords from the bed's edge to the top.
    let top = zf + 1.75;
    let rn = rf - 0.04;
    let m = 48;
    for k in 0..m {
        let a = TAU * k as f64 / m as f64;
        let (co, si) = (a.cos(), a.sin());
        cord(
            b,
            "net",
            [c[0] + rn * co, c[1] + rn * si, zf + 0.06],
            [c[0] + rn * co, c[1] + rn * si, top],
            0.003,
        );
    }
    for j in 1..10 {
        let z = zf + 0.06 + (top - zf - 0.06) * j as f64 / 10.0;
        tube(b, "net", &ring(c, rn, z, m), true, 0.003, 4);
    }
    tube(b, "dark", &ring(c, rn, top, m), true, 0.012, 6);
}

/// A treadmill: its back roller at `y = 0`, the console at the front.
pub fn treadmill(b: &mut Builder, _p: &Params) {
    let (w, l) = (0.86, 1.9);
    let x = w / 2.0;
    // The deck, its side rails and the belt.
    b.rbox("dark", [-x + 0.05, 0.02, 0.06], [x - 0.05, 1.62, 0.2], 0.03);
    for s in [-1.0, 1.0] {
        let (a, c) = if s < 0.0 {
            (-x, -x + 0.09)
        } else {
            (x - 0.09, x)
        };
        b.rbox("frame", [a, 0.0, 0.04], [c, 1.66, 0.22], 0.03);
        // Its feet.
        for y in [0.06, 1.5] {
            b.rbox("dark", [a + 0.01, y, 0.0], [c - 0.01, y + 0.1, 0.045], 0.01);
        }
    }
    b.rbox("mat", [-x + 0.1, 0.01, 0.2], [x - 0.1, 1.6, 0.215], 0.006);
    // The motor's hood at the front.
    b.rbox("frame", [-x, 1.56, 0.0], [x, l - 0.02, 0.3], 0.08);
    // The uprights, leaning back to the console.
    for s in [-1.0, 1.0] {
        let xs = s * (x - 0.04);
        sweep(
            b,
            "frame",
            &curve(&[[xs, 1.82, 0.25], [xs, 1.76, 0.75], [xs, 1.66, 1.2]], 5),
            |_| 0.032,
            10,
            Cap::Flat,
            Cap::Round(1.0),
        );
        // The handrail along the side, its grip.
        sweep(
            b,
            "dark",
            &curve(&[[xs, 1.72, 1.02], [xs, 1.45, 1.0], [xs, 1.2, 0.95]], 5),
            |_| 0.02,
            10,
            Cap::Round(1.0),
            Cap::Round(1.0),
        );
    }
    // The console: a tilted panel with its screen and a bar in front.
    let tilt: f64 = 0.55;
    let (co, si) = (tilt.cos(), tilt.sin());
    let cc = [0.0, 1.66, 1.3];
    b.obox(
        "frame",
        cc,
        [x - 0.02, 0.0, 0.0],
        [0.0, 0.17 * co, -0.17 * si],
        [0.0, 0.03 * si, 0.03 * co],
    );
    b.obox(
        "display",
        [cc[0], cc[1] - 0.031 * si, cc[2] + 0.031 * co],
        [0.15, 0.0, 0.0],
        [0.0, 0.09 * co, -0.09 * si],
        [0.0, 0.002 * si, 0.002 * co],
    );
    rod(
        b,
        "dark",
        [-x + 0.06, 1.5, 1.12],
        [x - 0.06, 1.5, 1.12],
        0.018,
    );
    b.rbox(
        "dark",
        [-x + 0.04, 1.49, 1.06],
        [-x + 0.08, 1.66, 1.13],
        0.01,
    );
    b.rbox("dark", [x - 0.08, 1.49, 1.06], [x - 0.04, 1.66, 1.13], 0.01);
}

/// An exercise bike: its back foot at `y = 0`, the handlebar at the front.
pub fn exercise_bike(b: &mut Builder, _p: &Params) {
    // The two feet, with their levelling caps.
    for y in [0.08, 1.02] {
        b.rbox(
            "dark",
            [-0.28, y - 0.045, 0.0],
            [0.28, y + 0.045, 0.065],
            0.025,
        );
    }
    // The frame: a bent tube from the back foot up under the seat and
    // down to the front foot.
    sweep(
        b,
        "frame",
        &curve(
            &[
                [0.0, 0.1, 0.06],
                [0.0, 0.3, 0.42],
                [0.0, 0.55, 0.4],
                [0.0, 0.85, 0.22],
                [0.0, 1.0, 0.06],
            ],
            6,
        ),
        |_| 0.038,
        12,
        Cap::Flat,
        Cap::Flat,
    );
    // The flywheel under its covers.
    let fw = [0.0, 0.83, 0.36];
    for (x0, x1, role, r) in [
        (-0.055, -0.03, "frame", 0.25),
        (-0.03, 0.03, "metal", 0.235),
        (0.03, 0.055, "frame", 0.25),
    ] {
        let st = b.mark();
        disc(b, role, [0.0, 0.0], r, x0, x1 - x0);
        b.transform_from(st, |q| [q[2], fw[1] - q[1], fw[2] + q[0]]);
    }
    // The seat on its post, the post on its slider.
    let seat = [0.0, 0.24, 0.92];
    b.rbox("frame", [-0.04, 0.27, 0.4], [0.04, 0.35, 0.55], 0.01);
    rod(
        b,
        "metal",
        [0.0, 0.31, 0.5],
        [0.0, seat[1] + 0.02, seat[2] - 0.04],
        0.022,
    );
    let st = b.mark();
    b.ellipsoid("mat", [0.0, 0.0, 0.0], [0.13, 0.15, 0.045]);
    b.transform_from(st, |q| {
        let k = 1.0 - 0.45 * (q[1] / 0.15).clamp(0.0, 1.0);
        [seat[0] + q[0] * k, seat[1] + q[1], seat[2] + q[2]]
    });
    // The handlebar's post and its bars with the console.
    let hb = [0.0, 0.95, 1.08];
    rod(
        b,
        "frame",
        [0.0, 0.72, 0.38],
        [0.0, hb[1] - 0.04, hb[2] - 0.06],
        0.03,
    );
    sweep(
        b,
        "dark",
        &curve(
            &[
                [-0.26, hb[1] - 0.12, hb[2] - 0.04],
                [-0.2, hb[1], hb[2]],
                [0.2, hb[1], hb[2]],
                [0.26, hb[1] - 0.12, hb[2] - 0.04],
            ],
            5,
        ),
        |_| 0.017,
        10,
        Cap::Round(1.0),
        Cap::Round(1.0),
    );
    b.rbox(
        "frame",
        [-0.1, hb[1] - 0.06, hb[2] + 0.02],
        [0.1, hb[1] + 0.02, hb[2] + 0.1],
        0.015,
    );
    b.rbox(
        "display",
        [-0.07, hb[1] - 0.061, hb[2] + 0.035],
        [0.07, hb[1] - 0.059, hb[2] + 0.09],
        0.0,
    );
    // The cranks and pedals.
    let ax = [0.0, 0.62, 0.36];
    for s in [-1.0, 1.0] {
        let a = if s < 0.0 { 0.9 } else { 0.9 + PI };
        let end = [s * 0.1, ax[1] + 0.17 * a.cos(), ax[2] + 0.17 * a.sin()];
        rod(
            b,
            "metal",
            [s * 0.05, ax[1], ax[2]],
            [s * 0.06, ax[1], ax[2]],
            0.04,
        );
        rod(
            b,
            "metal",
            [s * 0.06, ax[1], ax[2]],
            [s * 0.07, end[1], end[2]],
            0.012,
        );
        b.rbox(
            "dark",
            [s * 0.075 - 0.05, end[1] - 0.05, end[2] - 0.015],
            [s * 0.075 + 0.05, end[1] + 0.05, end[2] + 0.015],
            0.006,
        );
    }
}

/// A weight bench under its rack, the bar on the hooks with its plates.
pub fn weight_bench(b: &mut Builder, _p: &Params) {
    // The rack's two uprights on their feet, at the head of the bench.
    let (ux, uy, uh) = (0.55, 0.32, 1.55);
    for s in [-1.0, 1.0] {
        let x = s * ux;
        b.rbox(
            "dark",
            [x - 0.035, uy - 0.3, 0.0],
            [x + 0.035, uy + 0.35, 0.06],
            0.01,
        );
        b.rbox(
            "frame",
            [x - 0.03, uy - 0.03, 0.06],
            [x + 0.03, uy + 0.03, uh],
            0.006,
        );
        rod(b, "frame", [x, uy - 0.25, 0.06], [x, uy - 0.02, 0.5], 0.014);
        // The holes up its front, and the hook.
        for k in 0..14 {
            let z = 0.6 + 0.06 * k as f64;
            b.rbox(
                "dark",
                [x - 0.01, uy + 0.0295, z],
                [x + 0.01, uy + 0.031, z + 0.025],
                0.0,
            );
        }
        b.rbox(
            "metal",
            [x - 0.02, uy + 0.03, 1.1],
            [x + 0.02, uy + 0.1, 1.13],
            0.005,
        );
        b.rbox(
            "metal",
            [x - 0.02, uy + 0.08, 1.13],
            [x + 0.02, uy + 0.1, 1.18],
            0.005,
        );
    }
    rod(
        b,
        "frame",
        [-ux, uy - 0.28, 0.04],
        [ux, uy - 0.28, 0.04],
        0.02,
    );
    // The bar on the hooks, with a pair of plates and collars each side.
    let (by, bz) = (uy + 0.07, 1.15);
    rod(b, "metal", [-1.05, by, bz], [1.05, by, bz], 0.014);
    for s in [-1.0, 1.0] {
        rod(b, "metal", [s * 0.66, by, bz], [s * 1.0, by, bz], 0.025);
        for (x0, x1, r) in [
            (0.7, 0.735, 0.225),
            (0.735, 0.765, 0.225),
            (0.765, 0.785, 0.16),
        ] {
            let st = b.mark();
            disc(b, "plate", [0.0, 0.0], r, x0, x1 - x0);
            b.transform_from(st, |q| [s * q[2], by - s * q[1], bz + q[0]]);
        }
        rod(b, "metal", [s * 0.79, by, bz], [s * 0.82, by, bz], 0.032);
    }
    // The bench: its pad on a steel spine with two feet.
    let (y0, y1, z) = (0.12, 1.3, 0.43);
    Soft::new([-0.14, y0, z], [0.14, y1, z + 0.07], 0.02).add(b, "mat");
    b.rbox(
        "dark",
        [-0.12, y0 + 0.02, z - 0.03],
        [0.12, y1 - 0.02, z],
        0.0,
    );
    b.rbox(
        "frame",
        [-0.03, y0 + 0.08, z - 0.08],
        [0.03, y1 - 0.08, z - 0.03],
        0.006,
    );
    for y in [y0 + 0.12, y1 - 0.12] {
        b.rbox(
            "frame",
            [-0.03, y - 0.03, 0.06],
            [0.03, y + 0.03, z - 0.06],
            0.006,
        );
        b.rbox(
            "dark",
            [-0.25, y - 0.035, 0.0],
            [0.25, y + 0.035, 0.06],
            0.015,
        );
    }
}

/// A rack of dumbbells in two tiers: `width`.
pub fn dumbbell_rack(b: &mut Builder, p: &Params) {
    let w = p.get("width", 1.2).clamp(0.6, 2.4);
    let x = w / 2.0;
    // The two end frames, A-shaped, and the tiers between.
    for s in [-1.0, 1.0] {
        let xs = s * (x - 0.025);
        rod(b, "frame", [xs, 0.05, 0.03], [xs, 0.22, 0.85], 0.022);
        rod(b, "frame", [xs, 0.55, 0.03], [xs, 0.32, 0.85], 0.022);
        b.rbox("dark", [xs - 0.03, 0.0, 0.0], [xs + 0.03, 0.6, 0.04], 0.01);
    }
    let tiers = [(0.36, 0.42, 0.58), (0.72, 0.2, 0.38)];
    for (z, ya, yb) in tiers {
        b.rbox("frame", [-x, ya, z - 0.03], [x, ya + 0.025, z], 0.004);
        b.rbox("frame", [-x, yb - 0.025, z - 0.03], [x, yb, z], 0.004);
    }
    // The dumbbells, lying across, lighter on the top tier.
    let n = ((w - 0.1) / 0.17).floor().max(1.0) as usize;
    let gap = (w - 0.1) / n as f64;
    for (t, (z, ya, yb)) in tiers.into_iter().enumerate() {
        let yc = (ya + yb) / 2.0;
        for i in 0..n {
            let k = (t * n + i) as f64 / (2 * n) as f64;
            let heavy = 1.0 - k;
            let r = 0.045 + 0.04 * heavy;
            let head = 0.05 + 0.04 * heavy;
            let xc = -x + 0.05 + gap * (i as f64 + 0.5);
            let zc = z + r;
            let half = 0.07;
            for s in [-1.0, 1.0] {
                let (a, c) = (yc + s * half, yc + s * (half + head));
                let st = b.mark();
                disc(b, "plate", [0.0, 0.0], r, a.min(c), (c - a).abs());
                b.transform_from(st, |q| [xc - q[0], q[2], zc + q[1]]);
            }
            rod(b, "metal", [xc, yc - half, zc], [xc, yc + half, zc], 0.016);
        }
    }
}

/// Wall bars: `sections` of 90 cm side by side, `height`; standing off
/// the wall at `y = 0`.
pub fn wall_bars(b: &mut Builder, p: &Params) {
    let n = p.get("sections", 1.0).round().clamp(1.0, 4.0) as usize;
    let h = p.get("height", 2.4).clamp(2.0, 2.8);
    let sw = 0.9;
    let x0 = -sw * n as f64 / 2.0;
    for i in 0..=n {
        let x = x0 + sw * i as f64;
        b.rbox("frame", [x - 0.022, 0.0, 0.0], [x + 0.022, 0.1, h], 0.006);
        // The brackets to the wall.
        for z in [0.3, h - 0.2] {
            b.rbox("dark", [x - 0.03, 0.0, z], [x + 0.03, 0.02, z + 0.1], 0.004);
        }
    }
    let rungs = ((h - 0.15) / 0.152).floor() as usize;
    for i in 0..n {
        let (a, c) = (x0 + sw * i as f64 + 0.022, x0 + sw * (i + 1) as f64 - 0.022);
        for k in 0..rungs {
            let z = 0.15 + 0.152 * k as f64;
            // The top rung stands out from the others.
            let y = if k + 1 == rungs { 0.16 } else { 0.06 };
            if k + 1 == rungs {
                for xe in [a, c] {
                    b.rbox(
                        "frame",
                        [xe - 0.02, 0.05, z - 0.04],
                        [xe + 0.02, 0.19, z + 0.04],
                        0.006,
                    );
                }
            }
            rod(b, "frame", [a, y, z], [c, y, z], 0.017);
        }
    }
}

/// A yoga mat unrolled, a second one rolled at its head, two blocks and
/// a kettlebell.
pub fn yoga_mat(b: &mut Builder, _p: &Params) {
    let (w, l) = (0.61, 1.83);
    let x = w / 2.0;
    b.rbox("mat", [-x, 0.12, 0.0], [x, 0.12 + l, 0.006], 0.003);
    // The rolled one at its head.
    let st = b.mark();
    disc(b, "accent", [0.0, 0.0], 0.065, -x, w);
    b.transform_from(st, |q| [q[2], 0.06 - q[1], 0.065 + q[0]]);
    // The blocks.
    b.rbox(
        "plate",
        [x + 0.12, 0.3, 0.0],
        [x + 0.35, 0.45, 0.076],
        0.012,
    );
    b.rbox(
        "plate",
        [x + 0.14, 0.5, 0.0],
        [x + 0.29, 0.73, 0.076],
        0.012,
    );
    // The kettlebell.
    let k = [x + 0.25, 1.1];
    b.ellipsoid("dark", [k[0], k[1], 0.1], [0.1, 0.1, 0.095]);
    b.rbox(
        "dark",
        [k[0] - 0.055, k[1] - 0.055, 0.0],
        [k[0] + 0.055, k[1] + 0.055, 0.03],
        0.01,
    );
    sweep(
        b,
        "dark",
        &curve(
            &[
                [k[0] - 0.06, k[1], 0.17],
                [k[0] - 0.06, k[1], 0.26],
                [k[0], k[1], 0.29],
                [k[0] + 0.06, k[1], 0.26],
                [k[0] + 0.06, k[1], 0.17],
            ],
            4,
        ),
        |_| 0.015,
        8,
        Cap::Flat,
        Cap::Flat,
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fine::shape::signed_volume;

    type Make = fn(&mut Builder, &Params);

    #[test]
    fn the_sport_pieces_are_made() {
        let pieces: [(&str, Make); 10] = [
            ("canasta", basketball_hoop),
            ("portería", goal),
            ("ping-pong", ping_pong),
            ("cama elástica", trampoline),
            ("cinta", treadmill),
            ("bicicleta", exercise_bike),
            ("banco", weight_bench),
            ("mancuernero", dumbbell_rack),
            ("espaldera", wall_bars),
            ("esterilla", yoga_mat),
        ];
        for (name, make) in pieces {
            for (key, v) in [
                ("", 0.0),
                ("width", 7.32),
                ("sections", 3.0),
                ("diameter", 4.3),
            ] {
                let mut p = Params::default();
                if !key.is_empty() {
                    p.set(key, v);
                }
                let mut b = Builder::default();
                make(&mut b, &p);
                let tris = b.triangle_count();
                assert!((100..40_000).contains(&tris), "{name}: {tris}");
                for (role, t) in &b.parts {
                    let v = signed_volume(&b.positions, t);
                    assert!(v > -1e-7, "{name} {role}: {v}");
                }
                let low = b.positions.iter().map(|q| q[2]).fold(f32::MAX, f32::min);
                assert!(low > -0.001, "{name}: {low}");
            }
        }
    }
}
