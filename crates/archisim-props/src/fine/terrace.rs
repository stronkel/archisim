//! The workshop's garden and terrace, one of each piece for each
//! collection (paquete 5, tanda 5a): the outdoor table with its chairs,
//! the chair, the outdoor sofa, and the library's sun lounger, parasol,
//! big pot and pergola made the way each collection makes them.
//!
//! Axes as every piece's: the back at `y = 0`, the front looking to `+y`,
//! centred on `x = 0`; round pieces stand centred on `(0, width / 2)`.
//!
//! Roles: `frame` the wood (or the iron) of the structure, `metal`,
//! `weave` rope, wicker or cord, `cushion` and `pillow` the cushions,
//! `mat` canvas (a sling, a parasol's cloth), `accent` its stripes, `top`
//! a table top of another material, `masonry` a built bench, `pot`
//! (parameter `pot_mat`), `soil`, `stem`, `leaves`, `flowers`, `dark`.

use super::garden_plants;
use super::plant;
use super::shape::{bar, curve, sweep, turned, Cap};
use super::sofas::compass_leg;
use super::soft::{knife_pillow, tube, Soft};
use super::storage::{corners, tapered_leg, OUT};
use crate::build::{Builder, P3};
use crate::Params;
use std::f64::consts::{FRAC_PI_2, PI, TAU};

/// The collection of a piece that only has the collections' models.
fn own(p: &Params) -> usize {
    (p.style() - 1).clamp(0, 7) as usize
}

/// The collection when the model chosen is one of those listed after the
/// family's `first` own models.
fn look(p: &Params, first: i32) -> Option<usize> {
    let s = p.style() - first;
    (1..=8).contains(&s).then(|| (s - 1) as usize)
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

/// A seat or back cushion of outdoor fabric.
#[allow(clippy::too_many_arguments)]
fn cushion(b: &mut Builder, role: &'static str, lo: P3, hi: P3, r: f64, crown: f64, seed: u32) {
    let mut s = Soft::new(lo, hi, r);
    s.crown = crown;
    s.side = crown * 0.5;
    s.creases = 0.004;
    s.wobble = 0.0015;
    s.welt = 0.004;
    s.seed = seed;
    s.add(b, role);
}

/// A curl of wrought iron in the plane `y`: a spiral from `c` out to
/// radius `r`, `turns` round.
fn curl(b: &mut Builder, role: &'static str, c: P3, r: f64, turns: f64, a0: f64) {
    let n = (turns * 16.0).ceil() as usize;
    let pts: Vec<P3> = (0..=n)
        .map(|k| {
            let t = k as f64 / n as f64;
            let a = a0 + TAU * turns * t;
            let rr = r * (0.25 + 0.75 * t);
            [c[0] + rr * a.cos(), c[1], c[2] + rr * a.sin()]
        })
        .collect();
    tube(b, role, &pts, false, 0.006, 6);
}

// ---------------------------------------------------------- chairs

/// The outdoor chair of collection `ix`, its seat `h` high, front to
/// `+y`, centred on `(0, 0.25)`.
fn chair_of(b: &mut Builder, ix: usize, h: f64) {
    let (w, d) = (0.48, 0.5);
    let (x0, x1) = (-w / 2.0, w / 2.0);
    let back_h = h + 0.42;
    match ix {
        // Nordic: slatted wood, the back leaning.
        0 => {
            for q in corners(x0, x1, d, 0.02) {
                let top = if q[1] < 0.1 { back_h } else { h };
                b.rbox(
                    "frame",
                    [q[0] - 0.016, q[1] - 0.016, 0.0],
                    [q[0] + 0.016, q[1] + 0.016, top],
                    0.0,
                );
            }
            for k in 0..5 {
                let y = 0.05 + (d - 0.08) * k as f64 / 5.0;
                b.rbox(
                    "frame",
                    [x0, y + 0.004, h - 0.02],
                    [x1, y + (d - 0.08) / 5.0 - 0.004, h],
                    0.0,
                );
            }
            for k in 0..4 {
                let z = h + 0.12 + 0.07 * k as f64;
                b.rbox(
                    "frame",
                    [x0 + 0.02, 0.0, z],
                    [x1 - 0.02, 0.022, z + 0.05],
                    0.0,
                );
            }
        }
        // Mediterranean: wrought iron with a seat and back of rope.
        1 => {
            for q in corners(x0, x1, d, 0.02) {
                let top = if q[1] < 0.1 { back_h } else { h };
                b.cyl("metal", [q[0], q[1], 0.0], [q[0], q[1], top], 0.01);
            }
            let ring = [
                [x0 + 0.02, 0.02, h],
                [x1 - 0.02, 0.02, h],
                [x1 - 0.02, d - 0.02, h],
                [x0 + 0.02, d - 0.02, h],
            ];
            tube(b, "metal", &ring, true, 0.009, 6);
            let mut seat = Soft::new(
                [x0 + 0.03, 0.03, h - 0.01],
                [x1 - 0.03, d - 0.03, h + 0.012],
                0.01,
            );
            seat.crown = 0.003;
            seat.sag = 0.008;
            seat.seed = 71;
            seat.add_faces(b, "weave", [true; 6]);
            b.rbox(
                "weave",
                [x0 + 0.03, 0.01, h + 0.14],
                [x1 - 0.03, 0.03, back_h - 0.03],
                0.0,
            );
            tube(
                b,
                "metal",
                &[[x0 + 0.02, 0.02, back_h], [x1 - 0.02, 0.02, back_h]],
                false,
                0.009,
                6,
            );
            for s in [-1.0, 1.0] {
                curl(
                    b,
                    "metal",
                    [s * 0.1, 0.035, h - 0.1],
                    0.05,
                    1.3,
                    if s < 0.0 { 0.0 } else { PI },
                );
            }
        }
        // Fifties: a steel tube frame on splayed legs, a seat and back of
        // coloured cords.
        2 => {
            for (q, o) in corners(x0, x1, d, 0.04).iter().zip(OUT) {
                let foot = [q[0] + o[0] * 0.04, q[1] + o[1] * 0.04, 0.01];
                b.cyl("metal", foot, [q[0], q[1], h - 0.02], 0.009);
            }
            let ring = [[x0, 0.0, h], [x1, 0.0, h], [x1, d, h], [x0, d, h]];
            tube(b, "metal", &ring, true, 0.01, 6);
            let n = 14;
            for k in 0..n {
                let x = x0 + 0.02 + (w - 0.04) * (k as f64 + 0.5) / n as f64;
                b.cyl("weave", [x, 0.0, h], [x, d, h], 0.006);
                b.cyl("weave", [x, 0.0, h], [x, -0.06, back_h], 0.006);
            }
            tube(
                b,
                "metal",
                &[[x0, -0.06, back_h], [x1, -0.06, back_h]],
                false,
                0.01,
                6,
            );
            for x in [x0, x1] {
                b.cyl("metal", [x, 0.0, h], [x, -0.06, back_h], 0.01);
            }
        }
        // Japandi: low, a wide wooden seat, a back of rope between posts.
        3 => {
            let h = h - 0.05;
            for q in corners(x0, x1, d, 0.025) {
                let top = if q[1] < 0.1 { h + 0.32 } else { h };
                b.rbox(
                    "frame",
                    [q[0] - 0.022, q[1] - 0.022, 0.0],
                    [q[0] + 0.022, q[1] + 0.022, top],
                    0.0,
                );
            }
            b.rbox(
                "frame",
                [x0 - 0.01, 0.0, h - 0.03],
                [x1 + 0.01, d + 0.01, h],
                0.004,
            );
            b.rbox("frame", [x0, 0.0, h + 0.28], [x1, 0.045, h + 0.32], 0.0);
            for k in 0..6 {
                let z = h + 0.06 + 0.035 * k as f64;
                b.cyl("weave", [x0 + 0.02, 0.022, z], [x1 - 0.02, 0.022, z], 0.007);
            }
        }
        // Contemporary: a slim aluminium frame, a sling of canvas.
        4 => {
            for x in [x0, x1] {
                let pts = [
                    [x, d, 0.0],
                    [x, d - 0.04, h],
                    [x, 0.04, h - 0.03],
                    [x, -0.04, back_h],
                    [x, 0.0, 0.0],
                ];
                tube(b, "metal", &pts, false, 0.011, 8);
            }
            let mut sling = Soft::new(
                [x0 + 0.01, 0.03, h - 0.03],
                [x1 - 0.01, d - 0.03, h - 0.01],
                0.008,
            );
            sling.sag = 0.015;
            sling.seed = 72;
            sling.add_faces(b, "mat", [true; 6]);
            b.obox(
                "mat",
                [0.0, -0.005, (h + back_h) / 2.0],
                [w / 2.0 - 0.01, 0.0, 0.0],
                [0.0, 0.006, 0.0],
                [0.0, -0.035, (back_h - h) / 2.0 - 0.02],
            );
        }
        // Rustic: heavy pine, a plank seat and three upright slats.
        5 => {
            for q in corners(x0, x1, d, 0.025) {
                let top = if q[1] < 0.1 { back_h } else { h };
                b.rbox(
                    "frame",
                    [q[0] - 0.024, q[1] - 0.024, 0.0],
                    [q[0] + 0.024, q[1] + 0.024, top],
                    0.006,
                );
            }
            b.rbox(
                "frame",
                [x0 - 0.01, 0.0, h - 0.035],
                [x1 + 0.01, d + 0.01, h],
                0.006,
            );
            for z in [h + 0.32, back_h - 0.04] {
                b.rbox("frame", [x0, 0.0, z], [x1, 0.045, z + 0.06], 0.004);
            }
            for x in [-0.1, 0.0, 0.1] {
                b.rbox(
                    "frame",
                    [x - 0.03, 0.01, h],
                    [x + 0.03, 0.035, h + 0.33],
                    0.0,
                );
            }
            for y in [0.06, d - 0.06] {
                b.rbox("frame", [x0, y - 0.015, 0.12], [x1, y + 0.015, 0.16], 0.0);
            }
        }
        // Industrial: square steel tube, wooden slats.
        6 => {
            let t = 0.022;
            for q in corners(x0, x1, d, t / 2.0) {
                let top = if q[1] < 0.1 { back_h } else { h };
                bar(
                    b,
                    "metal",
                    [q[0], q[1], 0.0],
                    [q[0], q[1], top],
                    t,
                    [1.0, 0.0, 0.0],
                );
            }
            for k in 0..5 {
                let y = 0.02 + (d - 0.04) * k as f64 / 5.0;
                b.rbox(
                    "frame",
                    [x0, y + 0.004, h - 0.02],
                    [x1, y + (d - 0.04) / 5.0 - 0.004, h],
                    0.0,
                );
            }
            for z in [h + 0.16, h + 0.28] {
                b.rbox("frame", [x0 + t, 0.0, z], [x1 - t, 0.022, z + 0.08], 0.0);
            }
            bar(
                b,
                "metal",
                [x0, d / 2.0, 0.12],
                [x1, d / 2.0, 0.12],
                t * 0.7,
                [0.0, 0.0, 1.0],
            );
        }
        // Classic: cast iron, a curved back of scrolls, a round cushion.
        _ => {
            for (q, o) in corners(x0, x1, d, 0.04).iter().zip(OUT) {
                let pts = curve(
                    &[
                        [q[0] + o[0] * 0.05, q[1] + o[1] * 0.05, 0.013],
                        [q[0] + o[0] * 0.01, q[1] + o[1] * 0.01, h * 0.4],
                        [q[0], q[1], h],
                    ],
                    6,
                );
                sweep(
                    b,
                    "metal",
                    &pts,
                    |t| 0.012 - 0.004 * t,
                    8,
                    Cap::Round(1.0),
                    Cap::Flat,
                );
            }
            turned(
                b,
                "metal",
                [0.0, d / 2.0, h - 0.025],
                &[(0.0, 0.0), (0.24, 0.0), (0.24, 0.025), (0.0, 0.025)],
                32,
                0.0,
            );
            let back: Vec<P3> = (0..=12)
                .map(|k| {
                    let a = PI * k as f64 / 12.0;
                    [-0.22 * a.cos(), 0.02, h + 0.05 + 0.35 * a.sin()]
                })
                .collect();
            tube(b, "metal", &back, false, 0.01, 6);
            for k in 0..3 {
                curl(
                    b,
                    "metal",
                    [-0.1 + 0.1 * k as f64, 0.02, h + 0.2],
                    0.06,
                    1.2,
                    k as f64,
                );
            }
            let mut c = Soft::new(
                [-0.22, d / 2.0 - 0.22, h],
                [0.22, d / 2.0 + 0.22, h + 0.05],
                0.025,
            );
            c.crown = 0.012;
            c.seed = 73;
            c.add(b, "cushion");
        }
    }
}

/// The outdoor chair of each collection (styles 1 to 8): `height` of its
/// seat.
pub fn garden_chair(b: &mut Builder, p: &Params) {
    let h = p.get("height", 0.45).clamp(0.38, 0.5);
    chair_of(b, own(p), h);
}

// ---------------------------------------------------------- tables

/// The outdoor table of each collection (styles 1 to 8), with `chairs`
/// round it: `width`, `depth` (round ones take the smaller as their
/// diameter) and `height`.
pub fn garden_table(b: &mut Builder, p: &Params) {
    let ix = own(p);
    let w = p.get("width", 1.6).clamp(0.6, 3.0);
    let d = p.get("depth", 0.9).clamp(0.6, 1.4);
    let h = p.get("height", 0.75).clamp(0.6, 0.8);
    let (x0, x1) = (-w / 2.0, w / 2.0);
    let round = matches!(ix, 1 | 2 | 7);
    let r = w.min(d) / 2.0;
    match ix {
        // Nordic: a slatted top on tapered legs.
        0 => {
            for (q, o) in corners(x0, x1, d, 0.07).iter().zip(OUT) {
                tapered_leg(b, "frame", *q, h - 0.03, o, 0.02, 0.025);
            }
            b.rbox(
                "frame",
                [x0 + 0.05, 0.05, h - 0.09],
                [x1 - 0.05, d - 0.05, h - 0.03],
                0.0,
            );
            let n = ((d / 0.09).round() as usize).max(5);
            for k in 0..n {
                let y = d * k as f64 / n as f64;
                b.rbox(
                    "frame",
                    [x0, y + 0.004, h - 0.03],
                    [x1, y + d / n as f64 - 0.004, h],
                    0.003,
                );
            }
        }
        // Mediterranean: a round top of tiles on a wrought iron stand.
        1 => {
            turned(
                b,
                "top",
                [0.0, r, h - 0.025],
                &[(0.0, 0.0), (r, 0.0), (r, 0.025), (0.0, 0.025)],
                48,
                0.0,
            );
            tube(
                b,
                "metal",
                &ring_at([0.0, r, h - 0.012], r + 0.008, 48),
                true,
                0.012,
                6,
            );
            for k in 0..4 {
                let a = TAU * k as f64 / 4.0 + PI / 4.0;
                let (s, c) = a.sin_cos();
                let pts = curve(
                    &[
                        [0.0, r, h - 0.03],
                        [c * r * 0.4, r + s * r * 0.4, h * 0.55],
                        [c * r * 0.55, r + s * r * 0.55, 0.08],
                        [c * r * 0.72, r + s * r * 0.72, 0.013],
                    ],
                    6,
                );
                tube(b, "metal", &pts, false, 0.012, 6);
            }
            b.cyl("metal", [0.0, r, 0.2], [0.0, r, h - 0.03], 0.02);
        }
        // Fifties: a round teak top on three splayed legs with brass
        // shoes.
        2 => {
            turned(
                b,
                "frame",
                [0.0, r, h - 0.03],
                &[
                    (0.0, 0.0),
                    (r - 0.01, 0.0),
                    (r, 0.015),
                    (r - 0.01, 0.03),
                    (0.0, 0.03),
                ],
                48,
                0.0,
            );
            for k in 0..3 {
                let a = PI / 2.0 + TAU * k as f64 / 3.0;
                compass_leg(
                    b,
                    [0.25 * r * a.cos(), r + 0.25 * r * a.sin()],
                    h - 0.03,
                    [a.cos(), a.sin()],
                    0.18,
                );
            }
        }
        // Japandi: thick planks on two trestles with a stretcher.
        3 => {
            let n = ((d / 0.22).round() as usize).max(3);
            for k in 0..n {
                let y = d * k as f64 / n as f64;
                b.rbox(
                    "frame",
                    [x0, y + 0.003, h - 0.045],
                    [x1, y + d / n as f64 - 0.003, h],
                    0.004,
                );
            }
            for x in [x0 + 0.2, x1 - 0.26] {
                b.rbox(
                    "frame",
                    [x, 0.08, 0.0],
                    [x + 0.06, d - 0.08, h - 0.045],
                    0.0,
                );
            }
            b.rbox(
                "frame",
                [x0 + 0.2, d / 2.0 - 0.025, 0.25],
                [x1 - 0.2, d / 2.0 + 0.025, 0.31],
                0.0,
            );
        }
        // Contemporary: a slab of stone on a slim black frame.
        4 => {
            b.rbox("top", [x0, 0.0, h - 0.03], [x1, d, h], 0.004);
            let t = 0.03;
            for q in corners(x0, x1, d, 0.05) {
                bar(
                    b,
                    "metal",
                    [q[0], q[1], 0.0],
                    [q[0], q[1], h - 0.03],
                    t,
                    [1.0, 0.0, 0.0],
                );
            }
            for (a, c) in [
                ([x0 + 0.05, 0.05], [x1 - 0.05, 0.05]),
                ([x0 + 0.05, d - 0.05], [x1 - 0.05, d - 0.05]),
            ] {
                bar(
                    b,
                    "metal",
                    [a[0], a[1], h - 0.05],
                    [c[0], c[1], h - 0.05],
                    t * 0.8,
                    [0.0, 0.0, 1.0],
                );
            }
        }
        // Rustic: a thick top of boards on crossed trestles.
        5 => {
            let n = ((d / 0.15).round() as usize).max(4);
            for k in 0..n {
                let y = d * k as f64 / n as f64;
                b.rbox(
                    "frame",
                    [x0, y + 0.003, h - 0.06],
                    [x1, y + d / n as f64 - 0.003, h],
                    0.008,
                );
            }
            for x in [x0 + 0.22, x1 - 0.22] {
                for (ya, yb) in [(0.1, d - 0.1), (d - 0.1, 0.1)] {
                    bar(
                        b,
                        "frame",
                        [x, ya, 0.04],
                        [x, yb, h - 0.09],
                        0.07,
                        [1.0, 0.0, 0.0],
                    );
                }
                b.rbox(
                    "frame",
                    [x - 0.035, 0.05, h - 0.12],
                    [x + 0.035, d - 0.05, h - 0.06],
                    0.0,
                );
            }
            b.rbox(
                "frame",
                [x0 + 0.22, d / 2.0 - 0.03, 0.3],
                [x1 - 0.22, d / 2.0 + 0.03, 0.36],
                0.0,
            );
        }
        // Industrial: planks on a steel frame of square tube.
        6 => {
            let t = 0.04;
            for q in corners(x0, x1, d, t / 2.0 + 0.03) {
                bar(
                    b,
                    "metal",
                    [q[0], q[1], 0.0],
                    [q[0], q[1], h - 0.04],
                    t,
                    [1.0, 0.0, 0.0],
                );
            }
            for y in [0.05, d - 0.05] {
                bar(
                    b,
                    "metal",
                    [x0 + 0.03, y, h - 0.06],
                    [x1 - 0.03, y, h - 0.06],
                    t,
                    [0.0, 0.0, 1.0],
                );
                bar(
                    b,
                    "metal",
                    [x0 + 0.03, y, 0.12],
                    [x1 - 0.03, y, 0.12],
                    t * 0.7,
                    [0.0, 0.0, 1.0],
                );
            }
            let n = ((d / 0.13).round() as usize).max(4);
            for k in 0..n {
                let y = d * k as f64 / n as f64;
                b.rbox(
                    "frame",
                    [x0, y + 0.004, h - 0.04],
                    [x1, y + d / n as f64 - 0.004, h],
                    0.0,
                );
            }
        }
        // Classic: a round marble top on a cast iron pedestal of three
        // curved feet.
        _ => {
            turned(
                b,
                "top",
                [0.0, r, h - 0.03],
                &[(0.0, 0.0), (r, 0.0), (r, 0.03), (0.0, 0.03)],
                48,
                0.0,
            );
            turned(
                b,
                "metal",
                [0.0, r, 0.1],
                &[
                    (0.0, 0.0),
                    (0.06, 0.0),
                    (0.035, 0.06),
                    (0.04, 0.15),
                    (0.025, 0.25),
                    (0.028, h - 0.2),
                    (0.07, h - 0.14),
                    (0.0, h - 0.13),
                ],
                20,
                0.0,
            );
            for k in 0..3 {
                let a = TAU * k as f64 / 3.0;
                let (s, c) = a.sin_cos();
                let pts = curve(
                    &[
                        [0.0, r, 0.18],
                        [c * 0.12, r + s * 0.12, 0.14],
                        [c * 0.26, r + s * 0.26, 0.04],
                        [c * 0.33, r + s * 0.33, 0.012],
                    ],
                    6,
                );
                sweep(
                    b,
                    "metal",
                    &pts,
                    |t| 0.02 - 0.008 * t,
                    8,
                    Cap::Flat,
                    Cap::Round(1.0),
                );
            }
        }
    }
    // The chairs round it.
    let n = p.get("chairs", 0.0).round().clamp(0.0, 12.0) as usize;
    if n == 0 {
        return;
    }
    let seat = 0.45;
    let mut spots: Vec<(f64, f64, f64)> = Vec::new();
    if round {
        for k in 0..n {
            let a = TAU * k as f64 / n as f64 - FRAC_PI_2;
            let reach = r + 0.3;
            spots.push((reach * a.cos(), r + reach * a.sin(), a + FRAC_PI_2));
        }
    } else {
        let front = n.div_ceil(2);
        for (count, y, turn) in [(front, d + 0.3, PI), (n - front, -0.3, 0.0)] {
            for i in 0..count {
                let x = -w / 2.0 + w * (i as f64 + 0.5) / count as f64;
                spots.push((x, y, turn));
            }
        }
    }
    for (x, y, turn) in spots {
        // The chair looks along +y with its back at y = 0: back it off
        // so its seat's front is under the table's edge.
        placed(b, [x, y, 0.0], turn, |b| {
            let start = b.mark();
            chair_of(b, ix, seat);
            b.transform_from(start, |q| [q[0], q[1] - 0.25, q[2]]);
        });
    }
}

/// A circle of `n` points of radius `r` about `c`, level.
fn ring_at(c: P3, r: f64, n: usize) -> Vec<P3> {
    (0..n)
        .map(|k| {
            let a = TAU * k as f64 / n as f64;
            [c[0] + r * a.cos(), c[1] + r * a.sin(), c[2]]
        })
        .collect()
}

// ---------------------------------------------------------- sofas

/// The outdoor sofa of each collection (styles 1 to 8): `width`,
/// `depth`, `seat` height, `places` and scatter `pillows`.
pub fn garden_sofa(b: &mut Builder, p: &Params) {
    let ix = own(p);
    let w = p.get("width", 2.0).clamp(0.8, 3.5);
    let d = p.get("depth", 0.85).clamp(0.6, 1.1);
    let seat = p.get("seat", 0.42).clamp(0.3, 0.5);
    let places = p.get("places", 3.0).round().clamp(1.0, 5.0) as usize;
    let (x0, x1) = (-w / 2.0, w / 2.0);
    let arm = 0.12;
    let back = 0.14;
    let cush = 0.13;
    let base = seat - cush;
    let back_top = seat + 0.38;
    match ix {
        // Nordic: a teak frame with slatted sides on splayed legs.
        0 => {
            for (q, o) in corners(x0, x1, d, 0.06).iter().zip(OUT) {
                tapered_leg(b, "frame", *q, base - 0.06, o, 0.02, 0.022);
            }
            b.rbox("frame", [x0, 0.0, base - 0.06], [x1, d, base], 0.004);
            for x in [x0, x1 - arm * 0.5] {
                for k in 0..4 {
                    let z = base + 0.06 * k as f64;
                    b.rbox("frame", [x, 0.0, z], [x + arm * 0.5, d, z + 0.04], 0.0);
                }
            }
            b.rbox("frame", [x0, 0.0, base], [x1, 0.04, back_top - 0.05], 0.0);
        }
        // Mediterranean: a built bench of whitewashed masonry.
        1 => {
            b.rbox("masonry", [x0, 0.0, 0.0], [x1, d, base], 0.03);
            for x in [x0, x1 - arm] {
                b.rbox(
                    "masonry",
                    [x, 0.0, base - 0.01],
                    [x + arm, d, seat + 0.12],
                    0.04,
                );
            }
            b.rbox(
                "masonry",
                [x0, 0.0, base - 0.01],
                [x1, back * 0.9, back_top - 0.02],
                0.04,
            );
        }
        // Fifties: a low steel frame on splayed legs.
        2 => {
            for (q, o) in corners(x0, x1, d, 0.08).iter().zip(OUT) {
                let foot = [q[0] + o[0] * 0.06, q[1] + o[1] * 0.06, 0.013];
                b.cyl("metal", foot, [q[0], q[1], base], 0.012);
            }
            let ring = [
                [x0, 0.0, base],
                [x1, 0.0, base],
                [x1, d, base],
                [x0, d, base],
            ];
            tube(b, "metal", &ring, true, 0.014, 8);
            b.rbox(
                "frame",
                [x0 + 0.02, 0.02, base - 0.02],
                [x1 - 0.02, d - 0.02, base],
                0.0,
            );
            for x in [x0, x1] {
                tube(
                    b,
                    "metal",
                    &[
                        [x, d - 0.05, base],
                        [x, d - 0.08, seat + 0.14],
                        [x, 0.03, seat + 0.16],
                        [x, 0.0, back_top],
                    ],
                    false,
                    0.012,
                    8,
                );
            }
            tube(
                b,
                "metal",
                &[[x0, 0.0, back_top], [x1, 0.0, back_top]],
                false,
                0.012,
                8,
            );
        }
        // Japandi: a low wooden platform.
        3 => {
            b.rbox(
                "dark",
                [x0 + 0.05, 0.05, 0.0],
                [x1 - 0.05, d - 0.05, 0.06],
                0.0,
            );
            b.rbox("frame", [x0, 0.0, 0.06], [x1, d, base], 0.006);
            b.rbox("frame", [x0, 0.0, base], [x1, 0.05, back_top - 0.1], 0.004);
        }
        // Contemporary: an aluminium frame, thin, its arms flat.
        4 => {
            for x in [x0, x1 - 0.04] {
                b.rbox("metal", [x, 0.0, 0.0], [x + 0.04, d, 0.04], 0.0);
                b.rbox("metal", [x, d - 0.04, 0.0], [x + 0.04, d, seat + 0.16], 0.0);
                b.rbox("metal", [x, 0.0, 0.0], [x + 0.04, 0.04, back_top], 0.0);
                b.rbox(
                    "metal",
                    [x - 0.03, 0.0, seat + 0.16],
                    [x + 0.07, d, seat + 0.18],
                    0.0,
                );
            }
            b.rbox("metal", [x0, 0.04, base - 0.03], [x1, d - 0.04, base], 0.0);
            b.rbox("metal", [x0, 0.0, base], [x1, 0.03, back_top - 0.04], 0.0);
        }
        // Rustic: a thick frame of old timber.
        5 => {
            for q in corners(x0, x1, d, 0.05) {
                b.rbox(
                    "frame",
                    [q[0] - 0.05, q[1] - 0.05, 0.0],
                    [q[0] + 0.05, q[1] + 0.05, base + 0.2],
                    0.01,
                );
            }
            b.rbox("frame", [x0, 0.0, base - 0.08], [x1, d, base], 0.008);
            for x in [x0, x1 - 0.1] {
                b.rbox(
                    "frame",
                    [x, 0.0, base + 0.2],
                    [x + 0.1, d, base + 0.26],
                    0.008,
                );
            }
            b.rbox("frame", [x0, 0.0, base], [x1, 0.08, back_top], 0.01);
        }
        // Industrial: square steel tube.
        6 => {
            let t = 0.035;
            for q in corners(x0, x1, d, t / 2.0) {
                let top = if q[1] < 0.1 { back_top } else { seat + 0.14 };
                bar(
                    b,
                    "metal",
                    [q[0], q[1], 0.0],
                    [q[0], q[1], top],
                    t,
                    [1.0, 0.0, 0.0],
                );
            }
            for (a, c) in [([x0, 0.0], [x0, d]), ([x1, 0.0], [x1, d])] {
                let a = [
                    a[0] + if a[0] < 0.0 { t / 2.0 } else { -t / 2.0 },
                    a[1] + t / 2.0,
                ];
                let c = [
                    c[0] + if c[0] < 0.0 { t / 2.0 } else { -t / 2.0 },
                    c[1] - t / 2.0,
                ];
                bar(
                    b,
                    "metal",
                    [a[0], a[1], seat + 0.14],
                    [c[0], c[1], seat + 0.14],
                    t,
                    [0.0, 0.0, 1.0],
                );
            }
            b.rbox(
                "frame",
                [x0 + t, t, base - 0.025],
                [x1 - t, d - t, base],
                0.0,
            );
            b.rbox(
                "frame",
                [x0 + t, 0.0, base],
                [x1 - t, 0.025, back_top - 0.03],
                0.0,
            );
        }
        // Classic: wicker with rolled arms.
        _ => {
            b.rbox("weave", [x0, 0.0, 0.05], [x1, d, base], 0.03);
            for x in [x0, x1 - arm] {
                let mut a = Soft::new([x, 0.0, 0.05], [x + arm, d, seat + 0.16], 0.05);
                a.crown = 0.01;
                a.side = 0.01;
                a.seed = 74;
                a.add_faces(b, "weave", [true; 6]);
            }
            let mut bk = Soft::new([x0, 0.0, base], [x1, back, back_top + 0.04], 0.05);
            bk.crown = 0.01;
            bk.seed = 75;
            bk.add_faces(b, "weave", [true; 6]);
            for q in corners(x0, x1, d, 0.06) {
                turned(
                    b,
                    "frame",
                    [q[0], q[1], 0.0],
                    &[
                        (0.0, 0.0),
                        (0.03, 0.0),
                        (0.035, 0.02),
                        (0.03, 0.05),
                        (0.0, 0.05),
                    ],
                    12,
                    0.0,
                );
            }
        }
    }
    // Seat and back cushions, one per place.
    let (ix0, ix1) = match ix {
        1 | 7 => (x0 + arm, x1 - arm),
        0 => (x0 + arm * 0.5, x1 - arm * 0.5),
        5 => (x0 + 0.1, x1 - 0.1),
        _ => (x0 + 0.04, x1 - 0.04),
    };
    let bk_y = match ix {
        1 | 7 => back * 0.9,
        3 => 0.05,
        5 => 0.08,
        _ => 0.04,
    };
    let pw = (ix1 - ix0) / places as f64;
    for i in 0..places {
        let a = ix0 + pw * i as f64 + 0.004;
        let e = a + pw - 0.008;
        cushion(
            b,
            "cushion",
            [a, bk_y, base],
            [e, d + 0.01, seat],
            0.04,
            0.02,
            80 + i as u32,
        );
        let start = b.mark();
        cushion(
            b,
            "cushion",
            [a, bk_y, seat - 0.02],
            [e, bk_y + 0.16, back_top + 0.06],
            0.05,
            0.035,
            90 + i as u32,
        );
        let tilt: f64 = 0.2;
        let (st, ct) = tilt.sin_cos();
        b.transform_from(start, |q| {
            let (dy, dz) = (q[1] - bk_y, q[2] - (seat - 0.02));
            [
                q[0],
                bk_y + dy * ct - dz * st + 0.03,
                seat - 0.02 + dy * st + dz * ct,
            ]
        });
    }
    let pillows = p.get("pillows", 2.0).round().clamp(0.0, 4.0) as usize;
    let size = 0.42;
    let xs: Vec<f64> = match pillows {
        0 => vec![],
        1 => vec![ix0 + size * 0.6],
        2 => vec![ix0 + size * 0.6, ix1 - size * 0.6],
        3 => vec![ix0 + size * 0.6, ix0 + size * 1.4, ix1 - size * 0.6],
        _ => vec![
            ix0 + size * 0.6,
            ix0 + size * 1.4,
            ix1 - size * 1.4,
            ix1 - size * 0.6,
        ],
    };
    for (i, x) in xs.into_iter().enumerate() {
        let yaw: f64 = [-0.15, 0.08, -0.06, 0.14][i];
        let lean = 1.05_f64;
        let (sy, cy) = yaw.sin_cos();
        let (sl, cl) = lean.sin_cos();
        let az = [-sy * sl, cy * sl, cl];
        knife_pillow(
            b,
            "pillow",
            [x, bk_y + 0.3, seat + size * 0.45],
            [cy, sy, 0.0],
            az,
            size,
            size,
            0.17,
            100 + i as u32,
        );
    }
}

// ---------------------------------------------------------- loungers

/// Sun loungers of the collections (after the family's 3 own models):
/// `width`, `depth`, the head at the back; `pad` lays a cushion.
pub fn lounger(b: &mut Builder, p: &Params) -> bool {
    let Some(ix) = look(p, 3) else {
        return false;
    };
    let w = p.get("width", 0.7).clamp(0.5, 2.0);
    let l = p.get("depth", 2.0).clamp(1.6, 2.4);
    let (x0, x1) = (-w / 2.0, w / 2.0);
    let seat = if ix == 3 { 0.25 } else { 0.34 };
    // The back rises from 0.62 m along, leaning back 35°.
    let hinge = 0.62;
    let lean: f64 = 0.6;
    let (sl, cl) = lean.sin_cos();
    let back_len = 0.62;
    let top = [hinge - back_len * cl, seat + back_len * sl];
    match ix {
        0 | 5 => {
            // Wood: two rails on legs, slats, the back on its own frame.
            let (role, s) = ("frame", if ix == 5 { 0.05 } else { 0.04 });
            for x in [x0, x1 - s] {
                b.rbox(role, [x, 0.05, seat - 0.08], [x + s, l, seat - 0.02], 0.004);
                for y in [0.1, l - 0.12] {
                    b.rbox(role, [x, y, 0.0], [x + s, y + s, seat - 0.02], 0.0);
                }
            }
            if ix == 5 {
                for x in [x0 - 0.03, x1 + 0.03] {
                    turned_wheel(b, [x, 0.13, 0.11], 0.11);
                }
            }
            let n = (((l - hinge) / 0.075).floor() as usize).max(8);
            for k in 0..n {
                let y = hinge + (l - hinge) * k as f64 / n as f64;
                b.rbox(
                    role,
                    [x0, y + 0.005, seat - 0.02],
                    [x1, y + (l - hinge) / n as f64 - 0.005, seat],
                    0.0,
                );
            }
            let m = 8;
            for k in 0..m {
                let t0 = k as f64 / m as f64 + 0.01;
                let t1 = (k + 1) as f64 / m as f64 - 0.01;
                let a = [0.0, hinge - back_len * cl * t0, seat + back_len * sl * t0];
                let c = [0.0, hinge - back_len * cl * t1, seat + back_len * sl * t1];
                let mid = [0.0, (a[1] + c[1]) / 2.0, (a[2] + c[2]) / 2.0];
                b.obox(
                    role,
                    mid,
                    [w / 2.0 - 0.01, 0.0, 0.0],
                    [0.0, (c[1] - a[1]) / 2.0, (c[2] - a[2]) / 2.0],
                    [0.0, -sl * 0.01, cl * 0.01],
                );
            }
            for x in [x0 + 0.03, x1 - 0.03] {
                b.cyl(
                    role,
                    [x, hinge - 0.35, seat - 0.05],
                    [x, top[0] + 0.15, top[1] - 0.12],
                    0.012,
                );
            }
        }
        // Mediterranean: whitewashed frame laced with rope.
        1 => {
            for x in [x0, x1 - 0.045] {
                b.rbox("frame", [x, 0.0, seat - 0.06], [x + 0.045, l, seat], 0.004);
                for y in [0.05, l - 0.1] {
                    b.rbox("frame", [x, y, 0.0], [x + 0.045, y + 0.045, seat], 0.0);
                }
            }
            let n = ((l - hinge) / 0.04) as usize;
            for k in 0..n {
                let y = hinge + 0.02 + (l - hinge - 0.04) * k as f64 / n as f64;
                b.cyl(
                    "weave",
                    [x0 + 0.04, y, seat - 0.01],
                    [x1 - 0.04, y, seat - 0.01],
                    0.007,
                );
            }
            let m = 14;
            for k in 0..m {
                let t = (k as f64 + 0.5) / m as f64;
                let q = [hinge - back_len * cl * t, seat + back_len * sl * t];
                b.cyl(
                    "weave",
                    [x0 + 0.03, q[0], q[1]],
                    [x1 - 0.03, q[0], q[1]],
                    0.007,
                );
            }
            for x in [x0 + 0.01, x1 - 0.01] {
                b.cyl("frame", [x, hinge, seat], [x, top[0], top[1]], 0.02);
            }
        }
        // Fifties: a steel frame with cords, splayed legs.
        2 | 6 => {
            for x in [x0, x1] {
                let pts = [[x, l, seat], [x, hinge, seat], [x, top[0], top[1]]];
                tube(b, "metal", &pts, false, 0.013, 8);
                for (y, s) in [(l - 0.12, 1.0), (hinge - 0.1, -1.0)] {
                    b.cyl(
                        "metal",
                        [x + x.signum() * 0.03, y + s * 0.04, 0.011],
                        [x, y, seat],
                        0.011,
                    );
                }
            }
            tube(b, "metal", &[[x0, l, seat], [x1, l, seat]], false, 0.013, 8);
            tube(
                b,
                "metal",
                &[[x0, top[0], top[1]], [x1, top[0], top[1]]],
                false,
                0.013,
                8,
            );
            if ix == 6 {
                // Industrial: a mesh of steel.
                b.rbox(
                    "dark",
                    [x0 + 0.01, hinge, seat - 0.006],
                    [x1 - 0.01, l - 0.01, seat + 0.002],
                    0.0,
                );
                b.obox(
                    "dark",
                    [0.0, (hinge + top[0]) / 2.0, (seat + top[1]) / 2.0],
                    [w / 2.0 - 0.01, 0.0, 0.0],
                    [0.0, (top[0] - hinge) / 2.0, (top[1] - seat) / 2.0],
                    [0.0, -sl * 0.003, cl * 0.003],
                );
            } else {
                let n = 12;
                for k in 0..n {
                    let x = x0 + 0.02 + (w - 0.04) * (k as f64 + 0.5) / n as f64;
                    b.cyl("weave", [x, l, seat], [x, hinge, seat], 0.006);
                    b.cyl("weave", [x, hinge, seat], [x, top[0], top[1]], 0.006);
                }
            }
        }
        // Japandi: a low daybed of wood, a thick mattress, a bolster.
        3 => {
            b.rbox(
                "dark",
                [x0 + 0.05, 0.05, 0.0],
                [x1 - 0.05, l - 0.05, 0.05],
                0.0,
            );
            b.rbox("frame", [x0, 0.0, 0.05], [x1, l, seat - 0.1], 0.006);
            cushion(
                b,
                "cushion",
                [x0 + 0.02, 0.02, seat - 0.1],
                [x1 - 0.02, l - 0.02, seat + 0.05],
                0.04,
                0.012,
                76,
            );
            let start = b.mark();
            turned(
                b,
                "pillow",
                [0.0, 0.0, 0.0],
                &[
                    (0.0, 0.0),
                    (0.09, 0.0),
                    (0.1, 0.03),
                    (0.1, w - 0.1),
                    (0.09, w - 0.07),
                    (0.0, w - 0.07),
                ],
                16,
                0.0,
            );
            b.transform_from(start, |q| {
                [x0 + 0.035 + q[2], 0.15 + q[1], seat + 0.15 - q[0]]
            });
            b.soft.insert("pillow");
            return true;
        }
        // Contemporary: an aluminium frame, a sling.
        4 => {
            for x in [x0, x1 - 0.03] {
                b.rbox("metal", [x, 0.08, 0.0], [x + 0.03, l - 0.08, 0.03], 0.0);
                for y in [0.1, l - 0.16] {
                    b.rbox("metal", [x, y, 0.0], [x + 0.03, y + 0.06, seat], 0.0);
                }
                // The side rail at seat height, from head to foot: the
                // legs under it, the back's hinge on it.
                b.rbox("metal", [x, 0.08, seat - 0.04], [x + 0.03, l, seat], 0.0);
                // A prop from the rail up under the back.
                let t = 0.55;
                b.cyl(
                    "metal",
                    [x + 0.015, 0.3, seat - 0.01],
                    [
                        x + 0.015,
                        hinge - t * back_len * cl,
                        seat + t * back_len * sl,
                    ],
                    0.01,
                );
            }
            b.rbox(
                "mat",
                [x0 + 0.03, hinge, seat - 0.02],
                [x1 - 0.03, l - 0.01, seat - 0.008],
                0.0,
            );
            b.obox(
                "mat",
                [0.0, (hinge + top[0]) / 2.0, (seat + top[1]) / 2.0],
                [w / 2.0 - 0.01, 0.0, 0.0],
                [0.0, (top[0] - hinge) / 2.0, (top[1] - seat) / 2.0],
                [0.0, -sl * 0.006, cl * 0.006],
            );
            for x in [x0 + 0.015, x1 - 0.015] {
                b.cyl("metal", [x, hinge, seat], [x, top[0], top[1]], 0.015);
            }
        }
        // Classic: a wicker chaise with a rolled head.
        _ => {
            b.rbox("weave", [x0, hinge, 0.06], [x1, l, seat], 0.03);
            let n = 8;
            for k in 0..n {
                let t0 = k as f64 / n as f64;
                let t1 = (k + 1) as f64 / n as f64;
                let a = [hinge - back_len * cl * t0, seat + back_len * sl * t0];
                let c = [hinge - back_len * cl * t1, seat + back_len * sl * t1];
                b.obox(
                    "weave",
                    [0.0, (a[0] + c[0]) / 2.0, (a[1] + c[1]) / 2.0 - 0.1],
                    [w / 2.0, 0.0, 0.0],
                    [0.0, (c[0] - a[0]) / 2.0, (c[1] - a[1]) / 2.0],
                    [0.0, -sl * 0.1, cl * 0.1],
                );
            }
            for q in corners(x0, x1, l, 0.06) {
                turned(
                    b,
                    "frame",
                    [q[0], q[1], 0.0],
                    &[
                        (0.0, 0.0),
                        (0.03, 0.0),
                        (0.035, 0.03),
                        (0.028, 0.06),
                        (0.0, 0.06),
                    ],
                    12,
                    0.0,
                );
            }
        }
    }
    if p.flag("pad", true) {
        cushion(
            b,
            "cushion",
            [x0 + 0.02, hinge, seat + 0.002],
            [x1 - 0.02, l - 0.02, seat + 0.05],
            0.025,
            0.008,
            77,
        );
        let start = b.mark();
        cushion(
            b,
            "cushion",
            [x0 + 0.02, 0.0, 0.0],
            [x1 - 0.02, 0.045, back_len - 0.02],
            0.02,
            0.008,
            78,
        );
        // Laid up the back: along it `s`, through it `t` (a rotation).
        b.transform_from(start, |q| {
            let (t, s) = (q[1], q[2]);
            [q[0], hinge - s * cl + t * sl, seat + 0.01 + s * sl + t * cl]
        });
    }
    true
}

/// A wooden wheel standing at `c`, `r` across, its axle along `x`.
fn turned_wheel(b: &mut Builder, c: P3, r: f64) {
    let start = b.mark();
    turned(
        b,
        "frame",
        [0.0; 3],
        &[(0.0, -0.02), (r, -0.02), (r, 0.02), (0.0, 0.02)],
        20,
        0.0,
    );
    // Its axis turned from z to x (a rotation).
    b.transform_from(start, |q| [c[0] + q[2], c[1] + q[1], c[2] - q[0]]);
}

// ---------------------------------------------------------- parasols

/// The cloth of a parasol: `n` panels from its edge at `c` (radius `r`)
/// up `rise` to the top, alternate panels in `accent` when `stripes`.
#[allow(clippy::too_many_arguments)]
fn canopy(
    b: &mut Builder,
    c: P3,
    r: f64,
    rise: f64,
    n: usize,
    turn: f64,
    stripes: bool,
    valance: f64,
) {
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
        let role = if stripes && k % 2 == 1 {
            "accent"
        } else {
            "mat"
        };
        b.tri(role, ids[k], ids[k1], top);
        b.tri(role, ids[k1], ids[k], mid);
        b.cyl("frame", edge[k], [c[0], c[1], c[2] + rise * 0.5], 0.008);
        if valance > 0.0 {
            let (a, e) = (edge[k], edge[k1]);
            let (dx, dy) = (e[0] - a[0], e[1] - a[1]);
            let len = (dx * dx + dy * dy).sqrt();
            let m = [
                (a[0] + e[0]) / 2.0,
                (a[1] + e[1]) / 2.0,
                a[2] - valance / 2.0,
            ];
            b.obox(
                role,
                m,
                [dx / 2.0, dy / 2.0, 0.0],
                [-dy / len * 0.004, dx / len * 0.004, 0.0],
                [0.0, 0.0, valance / 2.0],
            );
        }
    }
}

/// Parasols of the collections (after the family's 3 own models):
/// `width` across, `height`.
pub fn parasol(b: &mut Builder, p: &Params) -> bool {
    let Some(ix) = look(p, 3) else {
        return false;
    };
    let s = p.get("width", 2.5).clamp(1.2, 4.0);
    let h = p.get("height", 2.4).clamp(1.8, 2.9);
    let r = s / 2.0;
    let c = [0.0, r];
    let pole = |b: &mut Builder, role: &'static str, rr: f64| {
        b.cyl(role, [c[0], c[1], 0.06], [c[0], c[1], h + 0.06], rr);
    };
    let disc_base = |b: &mut Builder, role: &'static str, rr: f64| {
        turned(
            b,
            role,
            [c[0], c[1], 0.0],
            &[
                (0.0, 0.0),
                (rr, 0.0),
                (rr * 0.95, 0.06),
                (0.05, 0.08),
                (0.0, 0.08),
            ],
            32,
            0.0,
        );
    };
    match ix {
        0 => {
            disc_base(b, "dark", 0.25);
            pole(b, "frame", 0.024);
            canopy(b, [c[0], c[1], h - 0.4], r, 0.38, 8, 0.0, false, 0.05);
            b.ellipsoid("frame", [c[0], c[1], h + 0.07], [0.03, 0.03, 0.03]);
        }
        1 => {
            // Straw on a whitewashed pole, set in a terracotta base.
            disc_base(b, "pot", 0.24);
            pole(b, "frame", 0.03);
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
        2 => {
            // Fifties: striped, scalloped, on a chrome pole.
            disc_base(b, "metal", 0.24);
            pole(b, "metal", 0.02);
            canopy(b, [c[0], c[1], h - 0.38], r, 0.34, 12, 0.0, true, 0.09);
            b.ellipsoid("metal", [c[0], c[1], h + 0.07], [0.035, 0.035, 0.035]);
        }
        3 => {
            disc_base(b, "dark", 0.25);
            pole(b, "frame", 0.026);
            canopy(
                b,
                [c[0], c[1], h - 0.3],
                r / std::f64::consts::SQRT_2,
                0.28,
                4,
                PI / 4.0,
                false,
                0.0,
            );
        }
        4 | 6 => {
            // A cantilever: the pole beside, an arm over, a square cloth.
            let px = -r - 0.25;
            b.rbox(
                "dark",
                [px - 0.45, c[1] - 0.45, 0.0],
                [px + 0.45, c[1] + 0.45, 0.06],
                0.0,
            );
            b.cyl("metal", [px, c[1], 0.06], [px, c[1], h + 0.1], 0.035);
            let pts = curve(
                &[
                    [px, c[1], h + 0.1],
                    [px + (r + 0.25) * 0.5, c[1], h + 0.25],
                    [0.0, c[1], h + 0.15],
                ],
                8,
            );
            sweep(b, "metal", &pts, |_| 0.025, 10, Cap::Flat, Cap::Flat);
            b.cyl("metal", [0.0, c[1], h + 0.15], [0.0, c[1], h - 0.15], 0.02);
            canopy(
                b,
                [0.0, c[1], h - 0.3],
                r / std::f64::consts::SQRT_2,
                0.36,
                4,
                PI / 4.0,
                false,
                if ix == 6 { 0.0 } else { 0.03 },
            );
        }
        5 => {
            // Rustic: canvas on a rough pole, a fringe, a stone base.
            disc_base(b, "dark", 0.27);
            pole(b, "frame", 0.035);
            canopy(b, [c[0], c[1], h - 0.42], r, 0.4, 8, 0.0, false, 0.06);
            let n = ((TAU * r / 0.05).round() as usize).max(24);
            for i in 0..n {
                let a = TAU * i as f64 / n as f64;
                let rr = r * (PI / 8.0).cos() / ((a % (TAU / 8.0)) - PI / 8.0).cos();
                let q = [c[0] + a.cos() * rr, c[1] + a.sin() * rr, h - 0.48];
                b.cyl("accent", q, [q[0], q[1], q[2] - 0.08], 0.005);
            }
        }
        // Classic: octagonal with a deep valance, a turned finial, a cast
        // iron base.
        _ => {
            turned(
                b,
                "metal",
                [c[0], c[1], 0.0],
                &[
                    (0.0, 0.0),
                    (0.3, 0.0),
                    (0.28, 0.04),
                    (0.12, 0.08),
                    (0.08, 0.15),
                    (0.04, 0.2),
                    (0.0, 0.2),
                ],
                24,
                0.0,
            );
            pole(b, "frame", 0.025);
            canopy(b, [c[0], c[1], h - 0.4], r, 0.42, 8, PI / 8.0, true, 0.14);
            turned(
                b,
                "metal",
                [c[0], c[1], h + 0.02],
                &[(0.03, 0.0), (0.04, 0.04), (0.012, 0.09), (0.0, 0.12)],
                16,
                0.0,
            );
        }
    }
    true
}

// ---------------------------------------------------------- big pots

/// Big pots of the collections (after the family's 4 own models): the

/// How far below the rim of a big pot its earth is, metres.
const SUNK: f64 = 0.04;

/// The rim walls of a square planter `2r` by `w`, `t` thick, from its
/// earth up to `top`.
fn rim(b: &mut Builder, r: f64, w: f64, top: f64, t: f64) {
    let lo = top - SUNK;
    b.rbox("pot", [-r, 0.0, lo], [r, t, top], 0.0);
    b.rbox("pot", [-r, w - t, lo], [r, w, top], 0.0);
    b.rbox("pot", [-r, t, lo], [-r + t, w - t, top], 0.0);
    b.rbox("pot", [r - t, t, lo], [r, w - t, top], 0.0);
}

/// collection's pot with a plant of the library's pots in it.
pub fn big_pot(b: &mut Builder, p: &Params) -> bool {
    let Some(ix) = look(p, 4) else {
        return false;
    };
    let w = p.get("width", 0.6).clamp(0.25, 1.4);
    let h = p.get("height", 1.4).clamp(0.3, 2.8);
    let r = w / 2.0;
    let c = [0.0, r];
    let mine = match ix {
        0 => (h * 0.45).min(0.8),
        1 => (w * 0.85).min(h * 0.4),
        2 => (w * 0.45).min(h * 0.5) + 0.22,
        3 => (h * 0.3).clamp(0.2, 0.45),
        4 => (h * 0.5).min(0.9),
        5 => (w * 0.55).min(h * 0.5),
        6 => (h * 0.4).clamp(0.25, 0.8),
        _ => (h * 0.42).min(0.9),
    };
    // The pot.
    match ix {
        // A tall cylinder of pale concrete.
        0 => turned(
            b,
            "pot",
            [c[0], c[1], 0.0],
            &[
                (0.0, 0.0),
                (r * 0.92, 0.0),
                (r, 0.01),
                (r, mine),
                (r * 0.9, mine),
                (r * 0.9, mine - SUNK),
                (0.0, mine - SUNK),
            ],
            40,
            0.0,
        ),
        // A terracotta pot with its rim.
        1 => turned(
            b,
            "pot",
            [c[0], c[1], 0.0],
            &[
                (0.0, 0.0),
                (r * 0.7, 0.0),
                (r * 0.95, mine * 0.85),
                (r * 1.05, mine * 0.88),
                (r * 1.05, mine),
                (r * 0.9, mine),
                (r * 0.9, mine - SUNK),
                (0.0, mine - SUNK),
            ],
            40,
            0.0,
        ),
        // A glazed bowl on three splayed legs.
        2 => {
            for k in 0..3 {
                let a = PI / 2.0 + TAU * k as f64 / 3.0;
                tapered_leg(
                    b,
                    "frame",
                    [c[0] + r * 0.45 * a.cos(), c[1] + r * 0.45 * a.sin()],
                    0.24,
                    [a.cos(), a.sin()],
                    0.06,
                    0.014,
                );
            }
            turned(
                b,
                "pot",
                [c[0], c[1], 0.22],
                &[
                    (0.0, 0.0),
                    (r * 0.5, 0.0),
                    (r * 0.95, (mine - 0.22) * 0.8),
                    (r, mine - 0.22),
                    (r * 0.88, mine - 0.22),
                    (r * 0.88, mine - 0.22 - SUNK),
                    (0.0, mine - 0.22 - SUNK),
                ],
                40,
                0.0,
            );
        }
        // A low dark stoneware bowl.
        3 => turned(
            b,
            "pot",
            [c[0], c[1], 0.0],
            &[
                (0.0, 0.0),
                (r * 0.6, 0.0),
                (r * 0.95, mine * 0.6),
                (r, mine),
                (r * 0.9, mine),
                (r * 0.9, mine - SUNK),
                (0.0, mine - SUNK),
            ],
            40,
            0.0,
        ),
        // A tall square planter of fibre cement.
        4 => {
            b.rbox("pot", [-r, 0.0, 0.0], [r, w, mine - SUNK], 0.0);
            rim(b, r, w, mine, 0.03);
        }
        // Half a wine barrel: staves and iron hoops.
        5 => {
            turned(
                b,
                "pot",
                [c[0], c[1], 0.0],
                &[
                    (0.0, 0.0),
                    (r * 0.88, 0.0),
                    (r, mine * 0.5),
                    (r * 0.94, mine),
                    (r * 0.86, mine),
                    (r * 0.86, mine - SUNK),
                    (0.0, mine - SUNK),
                ],
                24,
                0.0,
            );
            for z in [mine * 0.18, mine * 0.82] {
                let rr = r * (0.88 + 0.12 * (1.0 - ((z / mine - 0.5) * 2.0).abs())) + 0.004;
                tube(
                    b,
                    "metal",
                    &ring_at([c[0], c[1], z], rr, 32),
                    true,
                    0.006,
                    4,
                );
            }
        }
        // A box of weathered steel.
        6 => {
            b.rbox("pot", [-r, 0.0, 0.0], [r, w, mine - SUNK], 0.0);
            rim(b, r, w, mine, 0.008);
            // Its lip, folded out.
            for (lo, hi) in [
                ([-r - 0.01, -0.01], [r + 0.01, 0.0]),
                ([-r - 0.01, w], [r + 0.01, w + 0.01]),
                ([-r - 0.01, 0.0], [-r, w]),
                ([r, 0.0], [r + 0.01, w]),
            ] {
                b.rbox(
                    "pot",
                    [lo[0], lo[1], mine - 0.015],
                    [hi[0], hi[1], mine],
                    0.0,
                );
            }
        }
        // Classic: a stone urn on a plinth.
        _ => {
            b.rbox(
                "pot",
                [-r * 0.6, c[1] - r * 0.6, 0.0],
                [r * 0.6, c[1] + r * 0.6, 0.1],
                0.004,
            );
            turned(
                b,
                "pot",
                [c[0], c[1], 0.1],
                &[
                    (0.0, 0.0),
                    (r * 0.35, 0.0),
                    (r * 0.2, (mine - 0.1) * 0.2),
                    (r * 0.25, (mine - 0.1) * 0.28),
                    (r * 0.85, (mine - 0.1) * 0.7),
                    (r, (mine - 0.1) * 0.9),
                    (r * 1.08, mine - 0.1),
                    (r * 0.92, mine - 0.1),
                    (r * 0.92, mine - 0.1 - SUNK),
                    (0.0, mine - 0.1 - SUNK),
                ],
                40,
                0.0,
            );
        }
    }
    // Its earth, a little below the rim (domed, with a few crumbs; flat
    // in the square ones), and its plant: an olive tree, a tuft of grass
    // or a lavender bush, made leaf by leaf.
    let earth = mine - SUNK;
    match ix {
        4 | 6 => {
            let t = if ix == 4 { 0.03 } else { 0.008 };
            b.rbox(
                "soil",
                [-r + t, t, earth - 0.006],
                [r - t, w - t, earth + 0.004],
                0.0,
            );
        }
        _ => {
            let inner = match ix {
                2 => 0.88,
                5 => 0.86,
                7 => 0.92,
                _ => 0.9,
            };
            plant::soil_in(b, [c[0], c[1], earth], r * inner, 11 + ix as u32);
        }
    }
    let foot = [c[0], c[1], earth];
    let tall = (h - mine).max(0.2);
    match ix {
        1 | 4 | 7 => garden_plants::olive(b, foot, tall, (w * 1.5).max(tall * 0.75)),
        2 | 5 => garden_plants::lavender(b, foot, tall.min(0.75), w * 1.05),
        _ => garden_plants::grasses(b, foot, tall, w * 1.4),
    }
    true
}

// ---------------------------------------------------------- pergolas

/// Pergolas of the collections (after the family's 3 own models):
/// `width`, `depth`, `height` of its top.
pub fn pergola(b: &mut Builder, p: &Params) -> bool {
    let Some(ix) = look(p, 3) else {
        return false;
    };
    let w = p.get("width", 3.0).clamp(1.5, 8.0);
    let d = p.get("depth", 3.0).clamp(1.5, 8.0);
    let h = p.get("height", 2.5).clamp(2.1, 2.95);
    let (x0, x1) = (-w / 2.0, w / 2.0);
    let (post, role, beam) = match ix {
        2 | 4 | 6 => (0.08, "metal", 0.12),
        5 => (0.16, "frame", 0.22),
        7 => (0.22, "masonry", 0.18),
        _ => (0.12, "frame", 0.18),
    };
    let posts = corners(x0, x1, d, post / 2.0);
    for q in posts {
        match ix {
            // Classic: round columns with base and capital.
            7 => turned(
                b,
                "masonry",
                [q[0], q[1], 0.0],
                &[
                    (0.0, 0.0),
                    (post * 0.65, 0.0),
                    (post * 0.65, 0.05),
                    (post * 0.5, 0.08),
                    (post * 0.45, h - beam - 0.12),
                    (post * 0.6, h - beam - 0.05),
                    (post * 0.65, h - beam),
                    (0.0, h - beam),
                ],
                24,
                0.0,
            ),
            6 => {
                // A steel H post: two flanges and a web.
                b.rbox(
                    "metal",
                    [q[0] - post / 2.0, q[1] - post / 2.0, 0.0],
                    [q[0] + post / 2.0, q[1] - post / 2.0 + 0.008, h],
                    0.0,
                );
                b.rbox(
                    "metal",
                    [q[0] - post / 2.0, q[1] + post / 2.0 - 0.008, 0.0],
                    [q[0] + post / 2.0, q[1] + post / 2.0, h],
                    0.0,
                );
                b.rbox(
                    "metal",
                    [q[0] - 0.004, q[1] - post / 2.0, 0.0],
                    [q[0] + 0.004, q[1] + post / 2.0, h],
                    0.0,
                );
            }
            _ => b.rbox(
                role,
                [q[0] - post / 2.0, q[1] - post / 2.0, 0.0],
                [q[0] + post / 2.0, q[1] + post / 2.0, h],
                if ix == 1 { 0.02 } else { 0.004 },
            ),
        }
    }
    // Beams along the front and the back, projecting a little.
    let over = if ix == 4 { 0.0 } else { 0.2 };
    let brole = if ix == 7 { "frame" } else { role };
    for y in [post / 2.0, d - post / 2.0] {
        b.rbox(
            brole,
            [x0 - over, y - 0.04, h - beam],
            [x1 + over, y + 0.04, h],
            0.004,
        );
    }
    if ix == 4 {
        for x in [x0, x1 - 0.08] {
            b.rbox("metal", [x, 0.0, h - beam], [x + 0.08, d, h], 0.0);
        }
    }
    // What goes across: rafters, slats, louvres or reeds.
    let (step, depth, thick) = match ix {
        3 => (0.15, 0.06, 0.03),
        4 => (0.2, 0.16, 0.03),
        2 => (0.25, 0.06, 0.04),
        6 => (0.6, 0.12, 0.06),
        _ => (0.5, beam * 0.8, 0.06),
    };
    let n = ((w / step).round() as usize).max(3);
    for k in 0..=n {
        let x = x0 + w * k as f64 / n as f64;
        let x = x.clamp(x0 + thick / 2.0, x1 - thick / 2.0);
        if ix == 4 {
            // Louvres, tilted open.
            let a: f64 = 0.7;
            b.obox(
                "metal",
                [x, d / 2.0, h - beam / 2.0],
                [thick / 2.0 * a.cos(), 0.0, -thick / 2.0 * a.sin()],
                [0.0, d / 2.0 - 0.08, 0.0],
                [depth / 2.0 * a.sin(), 0.0, depth / 2.0 * a.cos()],
            );
        } else {
            b.rbox(
                brole,
                [x - thick / 2.0, -over, h],
                [x + thick / 2.0, d + over, h + depth],
                0.003,
            );
        }
    }
    match ix {
        // Reeds over it (Mediterranean).
        1 => b.rbox(
            "weave",
            [x0 - over, -over, h + beam * 0.8],
            [x1 + over, d + over, h + beam * 0.8 + 0.02],
            0.0,
        ),
        // A bougainvillea up a post and along the beam over it
        // (rustic, industrial, classic).
        5..=7 => {
            let q = posts[3];
            let around = post / 2.0 + 0.03;
            let mut path: Vec<P3> = (0..=30)
                .map(|k| {
                    let t = k as f64 / 30.0;
                    let a = t * TAU * 1.5;
                    [
                        q[0] + around * a.cos(),
                        q[1] + around * a.sin(),
                        0.015 + t * (h + 0.02 - 0.015),
                    ]
                })
                .collect();
            // Then along the outer face of the back beam, nearly to its
            // far end.
            let top = path[path.len() - 1];
            let face = q[1] + 0.08;
            let reach = x1 - 0.35 - top[0];
            for k in 1..=24 {
                let u = k as f64 / 24.0;
                path.push([
                    top[0] + reach * u,
                    face + 0.03 * (u * 13.0).sin(),
                    h - beam * 0.35 + 0.04 * (u * 9.0).sin(),
                ]);
            }
            garden_plants::climber(b, &path, 0.0, 7 + ix as u32);
            // And an arm over the top of the rafters, towards the front.
            let over: Vec<P3> = (0..=14)
                .map(|k| {
                    let u = k as f64 / 14.0;
                    [
                        top[0] + 0.25 + 0.6 * u + 0.05 * (u * 7.0).sin(),
                        q[1] - (q[1] - d * 0.3) * u,
                        h + depth + 0.025,
                    ]
                })
                .collect();
            let mut arm = vec![top];
            arm.extend(over);
            garden_plants::climber(b, &arm, 0.0, 31 + ix as u32);
        }
        _ => {}
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fine::bedroom::COLLECTIONS;
    use crate::fine::shape::signed_volume;

    #[test]
    fn the_terraces_of_the_eight_collections() {
        type Make = fn(&mut Builder, &Params) -> bool;
        let chair: Make = |b, p| {
            garden_chair(b, p);
            true
        };
        let table: Make = |b, p| {
            garden_table(b, p);
            true
        };
        let sofa: Make = |b, p| {
            garden_sofa(b, p);
            true
        };
        let pieces: [(&str, Make, i32); 7] = [
            ("silla", chair, 0),
            ("mesa", table, 0),
            ("sofá", sofa, 0),
            ("tumbona", lounger, 3),
            ("sombrilla", parasol, 3),
            ("macetón", big_pot, 4),
            ("pérgola", pergola, 3),
        ];
        for (c, col) in COLLECTIONS.iter().enumerate() {
            for (name, make, first) in pieces {
                let mut p = Params::default();
                p.set("style", (first + 1 + c as i32) as f64);
                p.set("chairs", 4.0);
                let mut b = Builder::default();
                assert!(make(&mut b, &p), "{name}");
                let tris = b.triangle_count();
                assert!((50..40_000).contains(&tris), "{col} {name}: {tris}");
                for (role, t) in &b.parts {
                    let v = signed_volume(&b.positions, t);
                    assert!(v > -1e-7, "{col} {name} {role}: {v}");
                }
                for q in &b.positions {
                    assert!(q.iter().all(|v| v.is_finite()), "{col} {name}");
                    assert!(q[2] > -0.001, "{col} {name}: {q:?}");
                }
            }
        }
    }
}
