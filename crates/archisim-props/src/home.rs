//! The pieces of each room that go with the cosy ones (colecciones,
//! revisión de Antonio del 23/10/2026): kitchen stools and an island with
//! things on the worktop, towels and baskets for the bathroom, pictures,
//! framed mirrors and clocks on the walls, floor cushions and folded
//! blankets. Axes as the furniture's: origin in the middle of the back,
//! `x` to the right, `y` forwards, `z` up; round pieces stand centred on
//! `(0, depth / 2)`. What hangs on a wall rises `elevation` from the floor.

use crate::build::{Builder, P3};
use crate::fronts::{self, Handle};
use crate::kitchen::tap;
use crate::Params;

use std::f64::consts::{PI, TAU};

/// Kitchen stools: 1 wooden with a round seat, 2 upholstered with a low
/// back on metal legs, 3 with a rush seat (as the Mediterranean chairs).
/// `height` is the seat's.
pub fn stool(b: &mut Builder, p: &Params) {
    let h = p.get("height", 0.65).clamp(0.4, 0.85);
    let s = p.get("width", 0.38).clamp(0.28, 0.55);
    let r = s / 2.0;
    let c = [0.0, r];
    let rung = h * 0.38;
    match p.style() {
        2 => {
            for k in 0..4 {
                let a = PI / 4.0 + k as f64 * PI / 2.0;
                let (sx, sy) = (a.cos(), a.sin());
                let top = [c[0] + sx * r * 0.62, c[1] + sy * r * 0.62, h - 0.05];
                let foot = [c[0] + sx * r * 0.85, c[1] + sy * r * 0.85, 0.004];
                b.cyl("metal", foot, top, 0.011);
            }
            ring(b, "metal", [c[0], c[1], rung], r * 0.76, 0.008);
            b.cone(
                "metal",
                [c[0], c[1], h - 0.06],
                [c[0], c[1], h - 0.045],
                r * 0.7,
                r * 0.7,
                20,
            );
            b.lathe(
                "cushion",
                [c[0], c[1], h - 0.05],
                &[
                    (0.0, 0.0),
                    (r, 0.0),
                    (r, 0.045),
                    (r * 0.9, 0.065),
                    (0.0, 0.07),
                ],
                1.0,
                1.0,
            );
            // The low back, curved round the seat's back.
            let n = 8;
            for i in 0..n {
                let a0 = PI * 0.15 + PI * 0.7 * i as f64 / n as f64;
                let a1 = PI * 0.15 + PI * 0.7 * (i + 1) as f64 / n as f64;
                let p0 = [c[0] + r * 0.92 * a0.cos(), c[1] - r * 0.92 * a0.sin()];
                let p1 = [c[0] + r * 0.92 * a1.cos(), c[1] - r * 0.92 * a1.sin()];
                b.pillow(
                    "cushion",
                    [p0[0].min(p1[0]), p0[1].min(p1[1]) - 0.025, h + 0.005],
                    [p0[0].max(p1[0]), p0[1].max(p1[1]) + 0.025, h + 0.2],
                    0.02,
                    0.01,
                );
            }
        }
        3 => {
            // A square frame of turned legs and rungs, the seat of rush.
            let half = r * 0.82;
            let corners = [
                [-half, c[1] - half],
                [half, c[1] - half],
                [half, c[1] + half],
                [-half, c[1] + half],
            ];
            for q in corners {
                b.lathe(
                    "mat",
                    [q[0], q[1], 0.004],
                    &[
                        (0.0, 0.0),
                        (0.016, 0.0),
                        (0.019, h * 0.5),
                        (0.016, h * 0.8),
                        (0.02, h - 0.02),
                        (0.0, h - 0.02),
                    ],
                    1.0,
                    1.0,
                );
            }
            for z in [rung, rung * 0.55] {
                for i in 0..4 {
                    let (a, e) = (corners[i], corners[(i + 1) % 4]);
                    b.cyl("mat", [a[0], a[1], z], [e[0], e[1], z], 0.009);
                }
            }
            b.pillow(
                "seat",
                [-half - 0.01, c[1] - half - 0.01, h - 0.045],
                [half + 0.01, c[1] + half + 0.01, h],
                0.012,
                0.008,
            );
        }
        _ => {
            for k in 0..4 {
                let a = PI / 4.0 + k as f64 * PI / 2.0;
                let (sx, sy) = (a.cos(), a.sin());
                let top = [c[0] + sx * r * 0.55, c[1] + sy * r * 0.55, h - 0.03];
                let foot = [c[0] + sx * r * 0.95, c[1] + sy * r * 0.95, 0.004];
                b.cone("mat", foot, top, 0.014, 0.018, 10);
            }
            ring(b, "mat", [c[0], c[1], rung], r * 0.78, 0.011);
            b.lathe(
                "mat",
                [c[0], c[1], h - 0.035],
                &[
                    (0.0, 0.025),
                    (r * 0.8, 0.02),
                    (r, 0.03),
                    (r, 0.0),
                    (0.0, 0.0),
                ],
                1.0,
                1.0,
            );
        }
    }
}

/// A ring of rod round the vertical through `c`.
fn ring(b: &mut Builder, role: &'static str, c: P3, r: f64, rod: f64) {
    let n = 16;
    for i in 0..n {
        let a0 = TAU * i as f64 / n as f64;
        let a1 = TAU * (i + 1) as f64 / n as f64;
        b.cyl(
            role,
            [c[0] + r * a0.cos(), c[1] + r * a0.sin(), c[2]],
            [c[0] + r * a1.cos(), c[1] + r * a1.sin(), c[2]],
            rod,
        );
    }
}

/// Kitchen islands: 1 units with doors and drawers and the worktop over
/// the back (`overhang`) for stools, 2 a worktable on legs with a shelf
/// and baskets under it, 3 units with a sink and the overhang. The fronts
/// look to `+y`; the overhang is on the back, `y` from 0.
pub fn island(b: &mut Builder, p: &Params) {
    let w = p.get("width", 1.8);
    let d = p.get("depth", 0.95);
    let h = p.get("height", 0.9);
    let over = p.get("overhang", 0.3).clamp(0.0, d * 0.5);
    let x0 = -w / 2.0;
    let top = 0.035;
    let kind = Handle::of(p, Handle::Bar);
    match p.style() {
        2 => {
            for (x, y) in [
                (x0 + 0.04, 0.04),
                (-x0 - 0.04, 0.04),
                (x0 + 0.04, d - 0.04),
                (-x0 - 0.04, d - 0.04),
            ] {
                b.rbox(
                    "mat",
                    [x - 0.03, y - 0.03, 0.0],
                    [x + 0.03, y + 0.03, h - top],
                    0.006,
                );
            }
            b.rbox(
                "mat",
                [x0 + 0.04, 0.04, h - top - 0.08],
                [-x0 - 0.04, d - 0.04, h - top],
                0.0,
            );
            b.rbox(
                "mat",
                [x0 + 0.02, 0.03, 0.14],
                [-x0 - 0.02, d - 0.03, 0.17],
                0.004,
            );
            // Baskets on the shelf.
            let n = ((w - 0.1) / 0.42).floor().max(1.0) as usize;
            let bw = (w - 0.1) / n as f64;
            for i in 0..n {
                let a = x0 + 0.05 + i as f64 * bw;
                b.rbox(
                    "weave",
                    [a + 0.03, 0.12, 0.17],
                    [a + bw - 0.03, d - 0.12, 0.42],
                    0.03,
                );
            }
            b.rbox("top", [x0, 0.0, h - top], [-x0, d, h], 0.006);
        }
        style => {
            let y0 = over;
            b.rbox(
                "plinth",
                [x0 + 0.05, y0 + 0.05, 0.0],
                [-x0 - 0.05, d - 0.06, 0.1],
                0.0,
            );
            b.rbox("mat", [x0, y0, 0.1], [-x0, d - 0.04, h - top], 0.0);
            let unit = 0.6_f64.min(w * 0.4);
            fronts::drawers(
                b,
                -x0 - unit,
                -x0,
                0.105,
                h - top - 0.005,
                d - 0.04,
                3,
                kind,
            );
            let n = ((w - unit) / 0.6).round().max(1.0) as usize;
            let fw = (w - unit) / n as f64;
            for i in 0..n {
                let a = x0 + i as f64 * fw;
                fronts::door(
                    b,
                    a,
                    a + fw,
                    0.105,
                    h - top - 0.005,
                    d - 0.04,
                    kind,
                    i % 2 == 0,
                    h - 0.12,
                );
            }
            if style == 3 {
                let (sx0, sx1) = (x0 + 0.2, x0 + 0.2 + 0.5_f64.min(w * 0.35));
                let (sy0, sy1) = (y0 + 0.12, d - 0.1);
                b.framed_top(
                    "top",
                    [x0, 0.0, h - top],
                    [-x0, d, h],
                    [sx0, sy0],
                    [sx1, sy1],
                );
                b.hollow("metal", [sx0, sy0, h - 0.2], [sx1, sy1, h], 0.03);
                tap(b, (sx0 + sx1) / 2.0, y0 + 0.06, h, 0.16);
            } else {
                b.rbox("top", [x0, 0.0, h - top], [-x0, d, h], 0.004);
            }
        }
    }
}

/// Things on a worktop: 1 chopping boards and a bottle of oil, 2 glass
/// jars, 3 a coffee pot and cups, 4 herbs in small pots. Standing at the
/// back of the surface they are put on (its `z` 0).
pub fn counter_decor(b: &mut Builder, p: &Params) {
    let w = p.get("width", 0.4).clamp(0.15, 1.0);
    let x0 = -w / 2.0;
    match p.style() {
        2 => {
            let n = ((w / 0.12).floor() as usize).clamp(2, 6);
            for i in 0..n {
                let x = x0 + (i as f64 + 0.5) * w / n as f64;
                let tall = 0.14 + 0.05 * ((i * 7 % 3) as f64);
                let r = 0.045;
                b.lathe(
                    "glass",
                    [x, 0.08, 0.0],
                    &[
                        (0.0, 0.0),
                        (r, 0.0),
                        (r, tall),
                        (r * 0.9, tall + 0.01),
                        (0.0, tall + 0.01),
                    ],
                    1.0,
                    1.0,
                );
                // What is in each jar, and its wooden lid.
                b.lathe(
                    "mat",
                    [x, 0.08, 0.004],
                    &[
                        (0.0, 0.0),
                        (r * 0.9, 0.0),
                        (r * 0.9, tall * 0.6),
                        (0.0, tall * 0.6),
                    ],
                    1.0,
                    1.0,
                );
                b.cone(
                    "frame",
                    [x, 0.08, tall + 0.01],
                    [x, 0.08, tall + 0.03],
                    r * 0.92,
                    r * 0.88,
                    16,
                );
            }
        }
        3 => {
            // A moka pot, two cups on saucers.
            let x = x0 + 0.08;
            b.lathe(
                "metal",
                [x, 0.08, 0.0],
                &[
                    (0.0, 0.0),
                    (0.05, 0.0),
                    (0.04, 0.09),
                    (0.03, 0.1),
                    (0.045, 0.19),
                    (0.02, 0.21),
                    (0.0, 0.215),
                ],
                1.0,
                1.0,
            );
            b.cyl(
                "dark",
                [x + 0.04, 0.08, 0.17],
                [x + 0.08, 0.08, 0.12],
                0.008,
            );
            for i in 0..2 {
                let cx = x + 0.13 + i as f64 * 0.11;
                b.cone("mat", [cx, 0.1, 0.0], [cx, 0.1, 0.008], 0.06, 0.065, 18);
                b.lathe(
                    "mat",
                    [cx, 0.1, 0.008],
                    &[
                        (0.0, 0.0),
                        (0.028, 0.0),
                        (0.04, 0.05),
                        (0.036, 0.055),
                        (0.0, 0.055),
                    ],
                    1.0,
                    1.0,
                );
            }
        }
        4 => {
            let n = ((w / 0.14).floor() as usize).clamp(1, 5);
            for i in 0..n {
                let x = x0 + (i as f64 + 0.5) * w / n as f64;
                let r = 0.055;
                b.lathe(
                    "pot",
                    [x, 0.09, 0.0],
                    &[
                        (0.0, 0.0),
                        (r * 0.8, 0.0),
                        (r, 0.1),
                        (r * 1.05, 0.11),
                        (0.0, 0.11),
                    ],
                    1.0,
                    1.0,
                );
                for k in 0..7 {
                    let a = TAU * (k as f64 / 7.0 + i as f64 * 0.13);
                    let lean = 0.35 + 0.1 * (k % 3) as f64;
                    let (ca, sa) = (a.cos(), a.sin());
                    let up = 0.06 + 0.02 * (k % 2) as f64;
                    b.blob(
                        "leaves",
                        [x + ca * 0.03, 0.09 + sa * 0.03, 0.14 + up * 0.5],
                        [ca * 0.012, sa * 0.012, 0.0],
                        [-sa * 0.025, ca * 0.025, 0.0],
                        [ca * lean * 0.04, sa * lean * 0.04, up],
                    );
                }
            }
        }
        _ => {
            // Two boards leant against the wall and a bottle.
            for (i, (bw, bh)) in [(0.3_f64.min(w * 0.7), 0.42), (0.22_f64.min(w * 0.5), 0.32)]
                .into_iter()
                .enumerate()
            {
                let x = x0 + bw / 2.0 + i as f64 * 0.06;
                let y = 0.02 + i as f64 * 0.025;
                let (lo, hi) = ([x - bw / 2.0, y, 0.0], [x + bw / 2.0, y + 0.02, bh]);
                b.rbox("mat", lo, hi, 0.012);
            }
            let x = -x0 - 0.05;
            b.lathe(
                "glass",
                [x, 0.07, 0.0],
                &[
                    (0.0, 0.0),
                    (0.035, 0.0),
                    (0.035, 0.18),
                    (0.012, 0.23),
                    (0.012, 0.27),
                    (0.0, 0.27),
                ],
                1.0,
                1.0,
            );
            b.lathe(
                "fruit",
                [x, 0.07, 0.004],
                &[(0.0, 0.0), (0.031, 0.0), (0.031, 0.12), (0.0, 0.12)],
                1.0,
                1.0,
            );
            b.cyl("frame", [x, 0.07, 0.27], [x, 0.07, 0.295], 0.013);
        }
    }
}

/// Towels: 1 a pile folded, 2 hung over a bar on the wall (at
/// `elevation`), 3 rolled up in a tray.
pub fn towels(b: &mut Builder, p: &Params) {
    let w = p.get("width", 0.4).clamp(0.2, 1.2);
    let x0 = -w / 2.0;
    match p.style() {
        2 => {
            let z = p.get("elevation", 1.1).clamp(0.3, 2.0);
            let y = 0.07;
            b.cyl("metal", [x0, y, z], [-x0, y, z], 0.01);
            for x in [x0 + 0.01, -x0 - 0.01] {
                b.cyl("metal", [x, 0.0, z], [x, y, z], 0.008);
            }
            // The towel over it, its two halves hanging.
            let tw = w - 0.08;
            b.pillow(
                "mat",
                [-tw / 2.0, y - 0.035, z - 0.55],
                [tw / 2.0, y - 0.012, z + 0.012],
                0.01,
                0.004,
            );
            b.pillow(
                "mat",
                [-tw / 2.0, y + 0.012, z - 0.45],
                [tw / 2.0, y + 0.035, z + 0.012],
                0.01,
                0.004,
            );
            b.cyl(
                "mat",
                [-tw / 2.0, y, z + 0.006],
                [tw / 2.0, y, z + 0.006],
                0.022,
            );
        }
        3 => {
            b.rbox("frame", [x0, 0.02, 0.0], [-x0, 0.3, 0.03], 0.008);
            let n = ((w / 0.12).floor() as usize).clamp(2, 8);
            for i in 0..n {
                let x = x0 + (i as f64 + 0.5) * w / n as f64;
                let role = if i % 2 == 0 { "mat" } else { "mat2" };
                b.cyl(role, [x, 0.04, 0.079], [x, 0.28, 0.079], 0.05);
            }
        }
        _ => {
            let n = p.get("count", 3.0).round().clamp(1.0, 6.0) as usize;
            let th = 0.055;
            for i in 0..n {
                let role = if i % 2 == 0 { "mat" } else { "mat2" };
                let z = i as f64 * th;
                let shift = if i % 2 == 0 { 0.0 } else { 0.008 };
                b.pillow(
                    role,
                    [x0 + shift, 0.02, z + 0.007],
                    [-x0 + shift, 0.3, z + th - 0.005],
                    0.02,
                    0.006,
                );
            }
        }
    }
}

/// Baskets: 1 a tall laundry basket with its lid, 2 a low one with rolls
/// of paper, 3 a wicker one with handles and a towel in it.
pub fn basket(b: &mut Builder, p: &Params) {
    let w = p.get("width", 0.4).clamp(0.15, 0.9);
    let r = w / 2.0;
    let c = [0.0, r];
    match p.style() {
        2 => {
            let h = p.get("height", 0.2).clamp(0.08, 0.5);
            weave(b, c, r, r * 0.92, h);
            for (dx, dy) in [(-0.5, -0.25), (0.5, -0.25), (0.0, 0.35)] {
                let x = c[0] + dx * r * 0.9;
                let y = c[1] + dy * r * 0.9;
                b.cyl(
                    "mat",
                    [x, y, 0.02],
                    [x, y, h + 0.03],
                    0.055_f64.min(r * 0.42),
                );
            }
        }
        3 => {
            let h = p.get("height", 0.3).clamp(0.1, 0.7);
            weave(b, c, r, r * 0.85, h);
            for s in [-1.0, 1.0] {
                b.rbox(
                    "weave",
                    [s * r * 0.98 - 0.015, c[1] - 0.05, h - 0.06],
                    [s * r * 0.98 + 0.015, c[1] + 0.05, h + 0.05],
                    0.012,
                );
            }
            b.pillow(
                "mat",
                [-r * 0.7, c[1] - r * 0.55, h * 0.7],
                [r * 0.7, c[1] + r * 0.6, h + 0.06],
                0.04,
                0.03,
            );
        }
        _ => {
            let h = p.get("height", 0.6).clamp(0.2, 1.0);
            weave(b, c, r, r * 0.88, h);
            b.lathe(
                "weave",
                [c[0], c[1], h],
                &[
                    (0.0, 0.0),
                    (r * 1.02, 0.0),
                    (r * 1.02, 0.025),
                    (r * 0.6, 0.05),
                    (0.0, 0.055),
                ],
                1.0,
                1.0,
            );
        }
    }
}

/// A woven basket, a little narrower at the foot, with ribs round it.
fn weave(b: &mut Builder, c: [f64; 2], r: f64, foot: f64, h: f64) {
    b.lathe(
        "weave",
        [c[0], c[1], 0.0],
        &[
            (0.0, 0.0),
            (foot, 0.0),
            (r, h * 0.92),
            (r * 1.02, h),
            (0.0, h),
        ],
        1.0,
        1.0,
    );
    let rows = ((h / 0.06).round() as usize).max(2);
    for i in 1..rows {
        let z = h * i as f64 / rows as f64;
        let rr = foot + (r - foot) * (z / (h * 0.92)).min(1.0);
        b.cone(
            "weave",
            [c[0], c[1], z - 0.006],
            [c[0], c[1], z + 0.006],
            rr + 0.005,
            rr + 0.005,
            24,
        );
    }
}

/// On the walls, at `elevation` (the bottom of the piece): 1 a picture in
/// a frame with a pale mount, 2 three pictures, 3 an arched mirror with a
/// frame, 4 a round mirror with a woven sun frame, 5 a clock.
pub fn wall_decor(b: &mut Builder, p: &Params) {
    // A collection's (paquete 6, tanda 6a).
    if crate::fine::indoor::wall_decor(b, p) {
        return;
    }
    let w = p.get("width", 0.6).clamp(0.15, 2.4);
    let h = p.get("height", 0.8).clamp(0.15, 2.0);
    let z = p.get("elevation", 1.2).clamp(0.0, 2.4);
    let x0 = -w / 2.0;
    let depth = 0.03;
    match p.style() {
        2 => {
            let gap = 0.05;
            let pw = (w - 2.0 * gap) / 3.0;
            for i in 0..3 {
                let a = x0 + i as f64 * (pw + gap);
                picture(b, a, a + pw, z, z + h, depth, i);
            }
        }
        3 => {
            // An arch: a straight lower part and a half circle on top.
            let r = w / 2.0;
            let straight = (h - r).max(0.05);
            b.rbox("frame", [x0, 0.0, z], [-x0, depth, z + straight], 0.006);
            b.rbox(
                "glass",
                [x0 + 0.03, depth, z + 0.03],
                [-x0 - 0.03, depth + 0.004, z + straight],
                0.0,
            );
            let n = 16;
            for i in 0..n {
                let a0 = PI * i as f64 / n as f64;
                let a1 = PI * (i + 1) as f64 / n as f64;
                let pt = |a: f64, rr: f64| [rr * a.cos(), z + straight + rr * a.sin()];
                let (p0, p1) = (pt(a0, r), pt(a1, r));
                let (q0, q1) = (pt(a0, r - 0.03), pt(a1, r - 0.03));
                fan(b, "frame", [p0, p1, q1, q0], 0.0, depth);
                fan(
                    b,
                    "glass",
                    [q0, q1, [0.0, z + straight], [0.0, z + straight]],
                    depth,
                    depth + 0.004,
                );
            }
        }
        4 => {
            let r = w.min(h) / 2.0;
            let cz = z + r;
            let n = 24;
            for i in 0..n {
                let a = TAU * i as f64 / n as f64;
                let long = if i % 2 == 0 { 1.0 } else { 0.78 };
                let (ca, sa) = (a.cos(), a.sin());
                b.cyl(
                    "frame",
                    [ca * r * 0.5, 0.012, cz + sa * r * 0.5],
                    [ca * r * long, 0.012, cz + sa * r * long],
                    0.006,
                );
            }
            b.cone(
                "frame",
                [0.0, 0.0, cz],
                [0.0, 0.022, cz],
                r * 0.55,
                r * 0.55,
                28,
            );
            b.cone(
                "glass",
                [0.0, 0.022, cz],
                [0.0, 0.026, cz],
                r * 0.48,
                r * 0.48,
                28,
            );
        }
        5 => {
            let r = w.min(h) / 2.0;
            let cz = z + r;
            b.cone("frame", [0.0, 0.0, cz], [0.0, 0.04, cz], r, r, 32);
            b.cone(
                "mat",
                [0.0, 0.04, cz],
                [0.0, 0.043, cz],
                r * 0.9,
                r * 0.9,
                32,
            );
            for k in 0..12 {
                let a = TAU * k as f64 / 12.0;
                let (ca, sa) = (a.cos(), a.sin());
                let s = if k % 3 == 0 { 0.011 } else { 0.005 };
                b.rbox(
                    "dark",
                    [ca * r * 0.8 - s, 0.043, cz + sa * r * 0.8 - s],
                    [ca * r * 0.8 + s, 0.046, cz + sa * r * 0.8 + s],
                    0.0,
                );
            }
            // The hands at ten past ten.
            for (a, l) in [(PI * 0.5 + PI / 3.0, 0.5), (PI * 0.5 - PI / 3.0, 0.72)] {
                b.cyl(
                    "dark",
                    [0.0, 0.048, cz],
                    [a.cos() * r * l, 0.048, cz + a.sin() * r * l],
                    0.004,
                );
            }
        }
        _ => picture(b, x0, -x0, z, z + h, depth, 0),
    }
}

/// A picture from `x0` to `x1` and `z0` to `z1`: its frame, a pale mount
/// and a composition of a few shapes in two colours (`n` varies it).
fn picture(b: &mut Builder, x0: f64, x1: f64, z0: f64, z1: f64, depth: f64, n: usize) {
    let f = 0.025_f64.min((x1 - x0) * 0.1);
    b.rbox("frame", [x0, 0.0, z0], [x0 + f, depth, z1], 0.002);
    b.rbox("frame", [x1 - f, 0.0, z0], [x1, depth, z1], 0.002);
    b.rbox("frame", [x0 + f, 0.0, z0], [x1 - f, depth, z0 + f], 0.002);
    b.rbox("frame", [x0 + f, 0.0, z1 - f], [x1 - f, depth, z1], 0.002);
    b.rbox(
        "mount",
        [x0 + f, 0.0, z0 + f],
        [x1 - f, depth * 0.6, z1 - f],
        0.0,
    );
    let (w, h) = (x1 - x0, z1 - z0);
    let m = 0.18 * w.min(h);
    let (ax0, ax1, az0, az1) = (x0 + m, x1 - m, z0 + m, z1 - m);
    let y = depth * 0.6;
    b.rbox("art", [ax0, y, az0], [ax1, y + 0.002, az1], 0.0);
    // A sun or a hill, a band, by turns.
    let cx = ax0 + (ax1 - ax0) * [0.62, 0.38, 0.5][n % 3];
    let cz = az0 + (az1 - az0) * [0.62, 0.45, 0.7][n % 3];
    let r = (ax1 - ax0).min(az1 - az0) * [0.26, 0.34, 0.22][n % 3];
    b.cone("art2", [cx, y + 0.002, cz], [cx, y + 0.004, cz], r, r, 24);
    let band = (az1 - az0) * 0.22;
    b.rbox(
        "art3",
        [ax0, y + 0.002, az0],
        [ax1, y + 0.005, az0 + band],
        0.0,
    );
}

/// A flat piece in the wall's plane: a quadrilateral `(x, z)` from `y0`
/// to `y1`.
fn fan(b: &mut Builder, role: &'static str, q: [[f64; 2]; 4], y0: f64, y1: f64) {
    let back: Vec<u32> = q.iter().map(|p| b.point([p[0], y0, p[1]])).collect();
    let front: Vec<u32> = q.iter().map(|p| b.point([p[0], y1, p[1]])).collect();
    // Outwards: the front looks to +y, the back to −y.
    b.quad(role, front[0], front[3], front[2], front[1]);
    b.quad(role, back[0], back[1], back[2], back[3]);
    for i in 0..4 {
        let j = (i + 1) % 4;
        b.quad(role, back[i], front[i], front[j], back[j]);
    }
}

/// Floor cushions: 1 a big square one, tufted, 2 a round one (a zafu), 3 a
/// pile of two with a little one on top.
pub fn floor_cushion(b: &mut Builder, p: &Params) {
    let w = p.get("width", 0.65).clamp(0.3, 1.2);
    let t = p.get("height", 0.16).clamp(0.06, 0.4);
    let x0 = -w / 2.0;
    match p.style() {
        2 => {
            let r = w / 2.0;
            b.lathe(
                "mat",
                [0.0, r, 0.0],
                &[
                    (0.0, 0.0),
                    (r * 0.9, 0.0),
                    (r, t * 0.3),
                    (r, t * 0.7),
                    (r * 0.9, t),
                    (0.0, t * 1.05),
                ],
                1.0,
                1.0,
            );
            for k in 0..12 {
                let a = TAU * k as f64 / 12.0;
                b.cyl(
                    "mat2",
                    [a.cos() * r * 0.6, r + a.sin() * r * 0.6, t * 0.98],
                    [a.cos() * r * 0.98, r + a.sin() * r * 0.98, t * 0.7],
                    0.006,
                );
            }
        }
        3 => {
            b.pillow(
                "mat",
                [x0, 0.0, t * 0.25],
                [-x0, w, t * 0.85],
                0.05,
                t * 0.25,
            );
            b.pillow(
                "mat2",
                [x0 + 0.04, 0.03, t * 0.9],
                [-x0 - 0.02, w - 0.02, t * 1.9],
                0.05,
                t * 0.25,
            );
            let s = w * 0.3;
            b.pillow(
                "accent",
                [-s, w * 0.35 - s, t * 1.8],
                [s, w * 0.35 + s, t * 1.8 + 0.12],
                0.04,
                0.03,
            );
        }
        _ => {
            b.pillow(
                "mat",
                [x0, 0.0, t * 0.25],
                [-x0, w, t * 0.85],
                0.05,
                t * 0.25,
            );
            for (dx, dy) in [
                (-0.25, -0.25),
                (0.25, -0.25),
                (-0.25, 0.25),
                (0.25, 0.25),
                (0.0, 0.0),
            ] {
                b.cone(
                    "mat2",
                    // Buttons sunk in the top.
                    [dx * w, w / 2.0 + dy * w, t * 0.8],
                    [dx * w, w / 2.0 + dy * w, t * 1.2],
                    0.012,
                    0.012,
                    8,
                );
            }
        }
    }
}

/// Blankets: 1 one folded, its fringe hanging, 2 a pile of three, 3 two
/// over the rungs of a wooden ladder leant on the wall.
pub fn blanket(b: &mut Builder, p: &Params) {
    let w = p.get("width", 0.45).clamp(0.2, 1.0);
    let x0 = -w / 2.0;
    let d = w * 0.75;
    match p.style() {
        2 => {
            for (i, role) in ["mat", "mat2", "accent"].into_iter().enumerate() {
                let z = i as f64 * 0.06;
                let s = [0.0, 0.012, -0.008][i];
                b.pillow(
                    role,
                    [x0 + s, 0.02 + s, z + 0.009],
                    [-x0 + s, 0.02 + d + s, z + 0.052],
                    0.025,
                    0.008,
                );
            }
        }
        3 => {
            let h = p.get("height", 1.6).clamp(0.8, 2.2);
            let lean = 0.32;
            // Two rails leant on the wall, the rungs between.
            for x in [x0, -x0] {
                b.cyl("frame", [x, lean, 0.004], [x, 0.02, h], 0.018);
            }
            let rungs = 5;
            for i in 1..=rungs {
                let t = i as f64 / (rungs + 1) as f64;
                let (y, z) = (lean + (0.02 - lean) * t, h * t);
                b.cyl("frame", [x0, y, z], [-x0, y, z], 0.012);
            }
            // Blankets hung over two rungs.
            for (i, role) in [(2, "mat"), (4, "mat2")] {
                let t = i as f64 / (rungs + 1) as f64;
                let (y, z) = (lean + (0.02 - lean) * t, h * t);
                let bw = w - 0.08;
                b.pillow(
                    role,
                    [-bw / 2.0, y - 0.03, z - 0.45],
                    [bw / 2.0, y - 0.01, z + 0.015],
                    0.01,
                    0.004,
                );
                b.pillow(
                    role,
                    [-bw / 2.0, y + 0.01, z - 0.35],
                    [bw / 2.0, y + 0.03, z + 0.015],
                    0.01,
                    0.004,
                );
                b.cyl(
                    role,
                    [-bw / 2.0, y, z + 0.008],
                    [bw / 2.0, y, z + 0.008],
                    0.024,
                );
            }
        }
        _ => {
            b.pillow("mat", [x0, 0.02, 0.011], [-x0, 0.02 + d, 0.07], 0.03, 0.01);
            // The fold, a roll along the front.
            b.cyl(
                "mat",
                [x0 + 0.003, 0.02 + d, 0.036],
                [-x0 - 0.003, 0.02 + d, 0.036],
                0.034,
            );
            let n = ((w / 0.025).round() as usize).max(4);
            for i in 0..n {
                let x = x0 + (i as f64 + 0.5) * w / n as f64;
                b.rbox(
                    "mat2",
                    [x - 0.004, 0.0, 0.003],
                    [x + 0.004, 0.025, 0.008],
                    0.0,
                );
            }
        }
    }
}
