//! The street furniture of the eight collections (paquete 5, tanda 5b):
//! benches, streetlights, litter bins, bollards, planters, bike racks and
//! drinking fountains, each as Nordic warm, Mediterranean, fifties,
//! japandi, contemporary, rustic, industrial and classic make it. They
//! are the families' models listed after their own (`look`), so the
//! projects made before see the same.
//!
//! Roles: `frame` (wood), `metal`, `mat` (the body: concrete, stone,
//! weathering steel, terrazzo), `masonry` (whitewashed work), `top`
//! (stone slabs and copings), `accent` (glazed tiles, a colour), `pot`
//! (the planters' trough), `soil`, the plants' `stem`, `leaves`,
//! `leaves2` and `flowers`, `light` (what glows at night), `water` and
//! `dark` (openings, grates).

use super::garden_plants;
use super::lamp;
use super::shape::{add, bar, curve, mul, rounded_rect, turned};
use super::sofas::compass_leg;
use super::soft::tube;
use super::storage::tapered_leg;
use crate::build::{Builder, P3};
use crate::Params;
use std::f64::consts::{FRAC_PI_4, PI, SQRT_2, TAU};

/// The collection when the model chosen is one of those listed after the
/// family's `first` own models.
fn look(p: &Params, first: i32) -> Option<usize> {
    let s = p.style() - first;
    (1..=8).contains(&s).then(|| (s - 1) as usize)
}

/// A closed ring of `n` points round `c`, radius `r`, level.
fn ring(c: P3, r: f64, n: usize) -> Vec<P3> {
    (0..n)
        .map(|k| {
            let a = TAU * k as f64 / n as f64;
            [c[0] + r * a.cos(), c[1] + r * a.sin(), c[2]]
        })
        .collect()
}

/// A slab whose section is the polygon `poly` (`(y, z)`, convex,
/// counter-clockwise seen from `+x`), from `x0` to `x1`.
fn side_slab(b: &mut Builder, role: &'static str, poly: &[[f64; 2]], x0: f64, x1: f64) {
    let start = b.mark();
    b.slab(role, poly, x0, x1);
    // (u, v, t) → (t, u, v): a turn, not a mirror.
    b.transform_from(start, |q| [q[2], q[0], q[1]]);
}

/// Points `(y, z)` at `x`.
fn yz(x: f64, pts: &[(f64, f64)]) -> Vec<P3> {
    pts.iter().map(|&(y, z)| [x, y, z]).collect()
}

/// `n` slats running along `x` from `x0` to `x1`, laid side by side
/// across the strip from `a` to `c` (`(y, z)`), `t` thick, their upper
/// faces on the strip.
#[allow(clippy::too_many_arguments)]
fn slats(
    b: &mut Builder,
    role: &'static str,
    x0: f64,
    x1: f64,
    a: (f64, f64),
    c: (f64, f64),
    n: usize,
    t: f64,
) {
    let (dy, dz) = (c.0 - a.0, c.1 - a.1);
    let l = (dy * dy + dz * dz).sqrt().max(1e-6);
    let (uy, uz) = (dy / l, dz / l);
    // Across the strip, up from its upper face.
    let (ny, nz) = (-uz, uy);
    let each = l / n as f64;
    for i in 0..n {
        let mid = (i as f64 + 0.5) * each;
        let (my, mz) = (a.0 + uy * mid - ny * t / 2.0, a.1 + uz * mid - nz * t / 2.0);
        b.obox(
            role,
            [(x0 + x1) / 2.0, my, mz],
            [(x1 - x0) / 2.0, 0.0, 0.0],
            [0.0, uy * each * 0.4, uz * each * 0.4],
            [0.0, ny * t / 2.0, nz * t / 2.0],
        );
    }
}

/// The supports along a bench or a planter `w` long: at `inset` from its
/// ends and no more than `span` apart.
fn supports(w: f64, inset: f64, span: f64) -> Vec<f64> {
    let free = (w - 2.0 * inset).max(0.0);
    let n = (free / span).ceil().max(1.0) as usize;
    (0..=n)
        .map(|i| -w / 2.0 + inset + free * i as f64 / n as f64)
        .collect()
}

/// A scroll of forged or cast iron in the plane through `c` across
/// `u` (in the plane) and up: `turns` of a spiral from radius `r` in.
#[allow(clippy::too_many_arguments)]
fn curl(b: &mut Builder, role: &'static str, c: P3, u: P3, r: f64, turns: f64, a0: f64, t: f64) {
    let n = (turns * 18.0).ceil() as usize + 2;
    let pts: Vec<P3> = (0..=n)
        .map(|k| {
            let s = k as f64 / n as f64;
            let a = a0 + s * turns * TAU;
            let rr = r * (1.0 - 0.75 * s);
            add(add(c, mul(u, rr * a.cos())), [0.0, 0.0, rr * a.sin()])
        })
        .collect();
    tube(b, role, &pts, false, t, 6);
}

/// A trough: its walls `t` thick round the box from `lo` to `hi` and its
/// floor; what fills it is put in after.
fn trough(b: &mut Builder, role: &'static str, lo: P3, hi: P3, t: f64) {
    let [x0, y0, z0] = lo;
    let [x1, y1, z1] = hi;
    b.rbox(role, [x0, y0, z0], [x1, y0 + t, z1], 0.0);
    b.rbox(role, [x0, y1 - t, z0], [x1, y1, z1], 0.0);
    b.rbox(role, [x0, y0 + t, z0], [x0 + t, y1 - t, z1], 0.0);
    b.rbox(role, [x1 - t, y0 + t, z0], [x1, y1 - t, z1], 0.0);
    b.rbox(role, [x0 + t, y0 + t, z0], [x1 - t, y1 - t, z0 + t], 0.0);
}

/// A lantern hanging from `hook`: `w` across its top, its glass `hb`
/// high, narrower below; a dish under it, corner posts, a pyramid roof
/// and a ring to hang it by. Its panes are `light`.
fn lantern(b: &mut Builder, frame: &'static str, hook: P3, w: f64, hb: f64) {
    let [x, y, top] = hook;
    let roof = w * 0.45;
    let z1 = top - 0.07 - roof;
    let z0 = z1 - hb;
    let (rt, rb) = (w / 2.0 * SQRT_2, w / 2.0 * SQRT_2 * 0.72);
    turned(
        b,
        "light",
        [x, y, z0],
        &[(0.0, 0.0), (rb * 0.97, 0.0), (rt * 0.97, hb), (0.0, hb)],
        4,
        FRAC_PI_4,
    );
    for k in 0..4 {
        let a = FRAC_PI_4 + k as f64 * PI / 2.0;
        b.cone(
            frame,
            [x + rb * a.cos(), y + rb * a.sin(), z0 - 0.005],
            [x + rt * a.cos(), y + rt * a.sin(), z1 + 0.005],
            0.008,
            0.008,
            6,
        );
    }
    turned(
        b,
        frame,
        [x, y, z0 - 0.05],
        &[
            (0.0, 0.0),
            (0.025, 0.0),
            (rb * 0.6, 0.03),
            (rb * 1.04, 0.05),
            (rb * 1.04, 0.06),
            (0.0, 0.06),
        ],
        4,
        FRAC_PI_4,
    );
    turned(
        b,
        frame,
        [x, y, z1],
        &[
            (0.0, 0.0),
            (rt * 1.14, 0.0),
            (rt * 1.14, 0.015),
            (0.03, roof),
            (0.0, roof),
        ],
        4,
        FRAC_PI_4,
    );
    b.cyl(frame, [x, y, z1 + roof - 0.01], [x, y, top - 0.03], 0.012);
    tube(
        b,
        frame,
        &yz(
            x,
            &[
                (y - 0.03, top - 0.03),
                (y, top + 0.005),
                (y + 0.03, top - 0.03),
            ],
        ),
        false,
        0.006,
        6,
    );
}

// ------------------------------------------------------------- benches

/// Benches of the collections (after the family's 3 own models): the
/// back at `y` 0, the middle at `x` 0; `width`, `depth`, `seat`,
/// `height` (of the back) and `back`.
pub fn bench(b: &mut Builder, p: &Params) -> bool {
    let Some(ix) = look(p, 3) else {
        return false;
    };
    let w = p.get("width", 1.8).clamp(0.6, 6.0);
    let d = p.get("depth", 0.5).clamp(0.3, 0.8);
    let seat = p.get("seat", 0.45).clamp(0.3, 0.6);
    let back = p.flag("back", true);
    let top = p.get("height", 0.85).max(seat + 0.2);
    let (x0, x1) = (-w / 2.0, w / 2.0);
    match ix {
        // Nordic: oak slats on loops of black tube, the back on two
        // uprights.
        0 => {
            for x in supports(w, 0.2, 1.4) {
                let loop_ = rounded_rect(
                    [(0.12 + d - 0.04) / 2.0, (0.016 + seat - 0.045) / 2.0],
                    (d - 0.16) / 2.0,
                    (seat - 0.061) / 2.0,
                    0.06,
                    0.0,
                    4,
                );
                let pts: Vec<P3> = loop_.iter().map(|q| [x, q[0], q[1]]).collect();
                tube(b, "metal", &pts, true, 0.014, 8);
                if back {
                    tube(
                        b,
                        "metal",
                        &curve(
                            &yz(
                                x,
                                &[(0.13, seat - 0.045), (0.11, seat + 0.1), (0.03, top - 0.02)],
                            ),
                            4,
                        ),
                        false,
                        0.012,
                        8,
                    );
                }
            }
            slats(b, "frame", x0, x1, (0.1, seat), (d, seat), 6, 0.03);
            if back {
                slats(
                    b,
                    "frame",
                    x0 + 0.03,
                    x1 - 0.03,
                    (0.1, seat + 0.14),
                    (0.02, top),
                    3,
                    0.025,
                );
            }
        }
        // Mediterranean: whitewashed work with a seat of glazed tiles,
        // rounded arms and a tiled back under a stone coping.
        1 => {
            let back_t = if back { 0.14 } else { 0.0 };
            b.rbox(
                "masonry",
                [x0 + 0.1, back_t, 0.0],
                [x1 - 0.1, d - 0.07, seat - 0.06],
                0.0,
            );
            b.rbox(
                "accent",
                [x0 + 0.1, back_t, seat - 0.06],
                [x1 - 0.1, d, seat],
                0.0,
            );
            for (a, c) in [(x0, x0 + 0.12), (x1 - 0.12, x1)] {
                b.rbox("masonry", [a, 0.0, 0.0], [c, d, seat + 0.16], 0.0);
                b.cone(
                    "masonry",
                    [(a + c) / 2.0, 0.0, seat + 0.16],
                    [(a + c) / 2.0, d, seat + 0.16],
                    0.06,
                    0.06,
                    16,
                );
            }
            if back {
                b.rbox("masonry", [x0, 0.0, 0.0], [x1, back_t, top - 0.05], 0.0);
                b.rbox(
                    "accent",
                    [x0 + 0.18, back_t, seat + 0.07],
                    [x1 - 0.18, back_t + 0.012, top - 0.12],
                    0.0,
                );
                b.rbox(
                    "top",
                    [x0 - 0.02, -0.02, top - 0.05],
                    [x1 + 0.02, back_t + 0.03, top],
                    0.0,
                );
            }
        }
        // Fifties: teak slats on a frame of tube, splayed compass legs,
        // the back of two wide slats on curved uprights.
        2 => {
            let z = seat - 0.05;
            let frame = rounded_rect(
                [0.0, (0.1 + d) / 2.0],
                w / 2.0 - 0.06,
                (d - 0.12) / 2.0,
                0.05,
                z,
                3,
            );
            tube(b, "metal", &frame, true, 0.012, 8);
            let legs = supports(w, 0.16, 1.6);
            for (i, x) in legs.iter().enumerate() {
                let end = i == 0 || i == legs.len() - 1;
                let sx = if !end {
                    0.0
                } else if *x < 0.0 {
                    -0.6
                } else {
                    0.6
                };
                for (y, sy) in [(0.16, -0.8), (d - 0.08, 0.8)] {
                    compass_leg(b, [*x, y], z, [sx, sy], 0.06);
                }
                if back && end {
                    tube(
                        b,
                        "metal",
                        &curve(
                            &yz(
                                *x - sx.signum() * 0.03,
                                &[(0.12, z), (0.1, seat + 0.08), (0.03, top - 0.03)],
                            ),
                            4,
                        ),
                        false,
                        0.012,
                        8,
                    );
                }
            }
            slats(
                b,
                "frame",
                x0 + 0.02,
                x1 - 0.02,
                (0.1, seat),
                (d, seat),
                5,
                0.025,
            );
            if back {
                slats(
                    b,
                    "frame",
                    x0 + 0.05,
                    x1 - 0.05,
                    (0.1, seat + 0.13),
                    (0.02, top),
                    2,
                    0.022,
                );
            }
        }
        // Japandi: a thick plank of smoked oak on blocks of stone; its
        // back, if any, a low board on two posts.
        3 => {
            let feet = supports(w, 0.32, 1.8);
            for x in &feet {
                b.rbox(
                    "mat",
                    [x - 0.15, 0.07, 0.0],
                    [x + 0.15, d - 0.07, seat - 0.08],
                    0.0,
                );
            }
            b.rbox("frame", [x0, 0.02, seat - 0.08], [x1, d, seat], 0.0);
            if back {
                for x in [feet[0], feet[feet.len() - 1]] {
                    b.rbox(
                        "frame",
                        [x - 0.025, 0.02, seat],
                        [x + 0.025, 0.07, seat + 0.36],
                        0.0,
                    );
                }
                b.rbox(
                    "frame",
                    [x0 + 0.1, 0.02, seat + 0.18],
                    [x1 - 0.1, 0.06, seat + 0.36],
                    0.0,
                );
            }
        }
        // Contemporary: a monolith of concrete with oak slats let into
        // its top, the back of slats on flat steel.
        4 => {
            b.rbox("mat", [x0, 0.06, 0.0], [x1, d, seat - 0.05], 0.0);
            slats(
                b,
                "frame",
                x0 + 0.03,
                x1 - 0.03,
                (0.08, seat),
                (d - 0.02, seat),
                7,
                0.05,
            );
            if back {
                for x in supports(w, 0.25, 1.4) {
                    bar(
                        b,
                        "metal",
                        [x, 0.09, seat - 0.05],
                        [x, 0.0, top],
                        0.02,
                        [1.0, 0.0, 0.0],
                    );
                }
                slats(
                    b,
                    "frame",
                    x0 + 0.02,
                    x1 - 0.02,
                    (0.075, seat + 0.1),
                    (0.005, top),
                    4,
                    0.03,
                );
            }
        }
        // Rustic: a thick plank on four splayed legs through it, the
        // back a board between two posts.
        5 => {
            b.rbox("frame", [x0, 0.08, seat - 0.09], [x1, d, seat], 0.0);
            for x in supports(w, 0.22, 1.6) {
                let sx = (x / w * 2.0).clamp(-1.0, 1.0) * 0.6;
                for (y, sy) in [(0.18, -0.6), (d - 0.1, 0.6)] {
                    tapered_leg(b, "frame", [x, y], seat - 0.08, [sx, sy], 0.08, 0.045);
                }
            }
            if back {
                for x in [x0 + 0.25, x1 - 0.25] {
                    b.rbox(
                        "frame",
                        [x - 0.035, 0.08, seat - 0.09],
                        [x + 0.035, 0.15, top],
                        0.0,
                    );
                }
                b.rbox(
                    "frame",
                    [x0 + 0.1, 0.05, top - 0.24],
                    [x1 - 0.1, 0.11, top - 0.03],
                    0.0,
                );
            }
        }
        // Industrial: plates of weathering steel, the seat and back of
        // wooden planks on flat bars.
        6 => {
            for x in supports(w, 0.08, 1.6) {
                side_slab(
                    b,
                    "mat",
                    &[
                        [0.1, 0.0],
                        [d - 0.02, 0.0],
                        [d - 0.02, seat - 0.06],
                        [0.1, seat - 0.06],
                    ],
                    x - 0.006,
                    x + 0.006,
                );
                if back {
                    side_slab(
                        b,
                        "mat",
                        &[
                            [0.02, seat - 0.06],
                            [0.14, seat - 0.06],
                            [0.14, top],
                            [0.02, top],
                        ],
                        x - 0.006,
                        x + 0.006,
                    );
                }
            }
            for y in [0.1, d - 0.05] {
                b.rbox(
                    "metal",
                    [x0, y, seat - 0.06],
                    [x1, y + 0.03, seat - 0.04],
                    0.0,
                );
            }
            slats(
                b,
                "frame",
                x0 - 0.02,
                x1 + 0.02,
                (0.1, seat),
                (d, seat),
                3,
                0.04,
            );
            if back {
                slats(
                    b,
                    "frame",
                    x0 - 0.02,
                    x1 + 0.02,
                    (0.14, seat + 0.12),
                    (0.14, top),
                    2,
                    0.035,
                );
            }
        }
        // Classic: cast iron ends with scrolled arms, slats curving over
        // the front and up the back.
        _ => {
            let ends = supports(w, 0.06, 1.5);
            for (i, &x) in ends.iter().enumerate() {
                bar(
                    b,
                    "metal",
                    [x, d - 0.06, 0.02],
                    [x, d - 0.09, seat - 0.02],
                    0.04,
                    [1.0, 0.0, 0.0],
                );
                bar(
                    b,
                    "metal",
                    [x, 0.04, 0.02],
                    [x, 0.13, seat - 0.02],
                    0.04,
                    [1.0, 0.0, 0.0],
                );
                bar(
                    b,
                    "metal",
                    [x, 0.12, seat - 0.04],
                    [x, d - 0.05, seat - 0.04],
                    0.035,
                    [1.0, 0.0, 0.0],
                );
                for k in [0.0, PI] {
                    turned(
                        b,
                        "metal",
                        [x, if k == 0.0 { 0.04 } else { d - 0.06 }, 0.0],
                        &[(0.0, 0.0), (0.04, 0.0), (0.03, 0.025), (0.0, 0.025)],
                        8,
                        0.0,
                    );
                }
                if back {
                    tube(
                        b,
                        "metal",
                        &curve(
                            &yz(x, &[(0.12, seat - 0.04), (0.1, seat + 0.15), (0.05, top)]),
                            4,
                        ),
                        false,
                        0.018,
                        8,
                    );
                }
                let end = i == 0 || i == ends.len() - 1;
                if end {
                    let arm = seat + 0.22;
                    tube(
                        b,
                        "metal",
                        &curve(
                            &yz(
                                x,
                                &[(0.1, arm + 0.02), (d * 0.5, arm), (d - 0.08, arm - 0.02)],
                            ),
                            4,
                        ),
                        false,
                        0.018,
                        8,
                    );
                    curl(
                        b,
                        "metal",
                        [x, d - 0.06, arm - 0.06],
                        [0.0, 1.0, 0.0],
                        0.045,
                        1.2,
                        PI / 2.0,
                        0.012,
                    );
                    tube(
                        b,
                        "metal",
                        &yz(x, &[(d - 0.08, arm - 0.04), (d - 0.08, seat - 0.02)]),
                        false,
                        0.014,
                        8,
                    );
                }
            }
            let xa = x0 - 0.03;
            let xb = x1 + 0.03;
            slats(b, "frame", xa, xb, (0.12, seat), (d - 0.06, seat), 5, 0.03);
            slats(
                b,
                "frame",
                xa,
                xb,
                (d - 0.06, seat),
                (d, seat - 0.05),
                1,
                0.03,
            );
            if back {
                slats(
                    b,
                    "frame",
                    xa,
                    xb,
                    (0.13, seat + 0.1),
                    (0.06, top - 0.01),
                    5,
                    0.025,
                );
            }
        }
    }
    true
}

// --------------------------------------------------------- streetlights

/// Streetlights of the collections (after the family's 4 own models):
/// the foot at the origin, the arm towards `+y`; `height`, `arm`,
/// `pole` (diameter) and `lamp` (size of the lantern or shade).
pub fn streetlight(b: &mut Builder, p: &Params) -> bool {
    let Some(ix) = look(p, 4) else {
        return false;
    };
    let h = p.get("height", 4.5).clamp(2.2, 12.0);
    let arm = p.get("arm", 0.8).clamp(0.3, 3.0);
    let r = (p.get("pole", 0.12).clamp(0.05, 0.4) / 2.0).max(0.03);
    let lamp = p.get("lamp", 0.5).clamp(0.2, 1.2);
    match ix {
        // Nordic: a black post clad in oak below, a short arm and a
        // conical shade.
        0 => {
            turned(
                b,
                "metal",
                [0.0; 3],
                &[(0.0, 0.0), (r * 1.6, 0.0), (r * 1.2, 0.25), (0.0, 0.25)],
                20,
                0.0,
            );
            b.cyl("metal", [0.0, 0.0, 0.2], [0.0, 0.0, h - 0.12], r * 0.8);
            b.cyl("frame", [0.0, 0.0, 0.3], [0.0, 0.0, 1.7], r * 1.1);
            let reach = (arm * 0.6).max(0.3);
            tube(
                b,
                "metal",
                &curve(
                    &yz(
                        0.0,
                        &[
                            (0.0, h - 0.3),
                            (0.0, h - 0.12),
                            (0.1, h - 0.05),
                            (reach, h - 0.06),
                        ],
                    ),
                    4,
                ),
                false,
                0.022,
                8,
            );
            let c = [0.0, reach, h - 0.06];
            turned(
                b,
                "metal",
                c,
                &[
                    (0.0, 0.03),
                    (0.04, 0.03),
                    (lamp * 0.5, -lamp * 0.32),
                    (lamp * 0.5 - 0.01, -lamp * 0.34),
                    (0.0, -lamp * 0.3),
                ],
                24,
                0.0,
            );
            turned(
                b,
                "light",
                c,
                &[
                    (0.0, -lamp * 0.3),
                    (lamp * 0.42, -lamp * 0.3),
                    (lamp * 0.4, -lamp * 0.33),
                    (0.0, -lamp * 0.33),
                ],
                24,
                0.0,
            );
        }
        // Mediterranean: a forged iron post, a scrolled bracket and a
        // lantern hanging from it.
        1 => {
            turned(
                b,
                "metal",
                [0.0; 3],
                &[
                    (0.0, 0.0),
                    (0.13, 0.0),
                    (0.12, 0.06),
                    (0.06, 0.16),
                    (0.045, 0.3),
                    (0.0, 0.3),
                ],
                12,
                0.0,
            );
            b.cyl("metal", [0.0, 0.0, 0.2], [0.0, 0.0, h - 0.1], 0.04);
            b.ellipsoid("metal", [0.0, 0.0, h - 0.06], [0.05, 0.05, 0.06]);
            let z = h - 0.3;
            tube(b, "metal", &yz(0.0, &[(0.0, z), (arm, z)]), false, 0.016, 8);
            tube(
                b,
                "metal",
                &curve(
                    &yz(
                        0.0,
                        &[
                            (0.0, z - 0.45),
                            (arm * 0.3, z - 0.25),
                            (arm * 0.65, z - 0.02),
                        ],
                    ),
                    4,
                ),
                false,
                0.012,
                6,
            );
            curl(
                b,
                "metal",
                [0.0, arm * 0.35, z - 0.12],
                [0.0, 1.0, 0.0],
                0.07,
                1.1,
                PI,
                0.01,
            );
            curl(
                b,
                "metal",
                [0.0, arm + 0.04, z + 0.04],
                [0.0, 1.0, 0.0],
                0.04,
                1.2,
                -PI / 2.0,
                0.01,
            );
            lantern(
                b,
                "metal",
                [0.0, arm - 0.06, z - 0.01],
                lamp * 0.6,
                lamp * 0.75,
            );
        }
        // Fifties: a tapering pole with three arms rising to globes.
        2 => {
            turned(
                b,
                "metal",
                [0.0; 3],
                &[(0.0, 0.0), (r * 1.8, 0.0), (r * 1.3, 0.12), (0.0, 0.12)],
                20,
                0.0,
            );
            b.cone(
                "metal",
                [0.0, 0.0, 0.1],
                [0.0, 0.0, h - 0.35],
                r * 1.1,
                r * 0.55,
                20,
            );
            for k in 0..3 {
                let a = PI / 2.0 + TAU * k as f64 / 3.0;
                let (ca, sa) = (a.cos(), a.sin());
                let reach = (arm * 0.5).max(0.3);
                let end = [ca * reach, sa * reach, h - lamp * 0.3 - 0.04];
                tube(
                    b,
                    "metal",
                    &curve(
                        &[
                            [0.0, 0.0, h - 0.45],
                            [ca * reach * 0.4, sa * reach * 0.4, h - 0.34],
                            end,
                        ],
                        4,
                    ),
                    false,
                    0.014,
                    8,
                );
                b.cone(
                    "metal",
                    add(end, [0.0, 0.0, -0.02]),
                    add(end, [0.0, 0.0, 0.04]),
                    0.03,
                    0.04,
                    12,
                );
                b.ellipsoid(
                    "light",
                    add(end, [0.0, 0.0, 0.04 + lamp * 0.28]),
                    [lamp * 0.28, lamp * 0.28, lamp * 0.28],
                );
            }
        }
        // Japandi: a square post of smoked oak with a lantern of slats
        // and light on its top.
        3 => {
            let hh = h.min(3.2);
            let s = 0.06;
            let lt = hh - 0.5;
            b.rbox("mat", [-0.16, -0.16, 0.0], [0.16, 0.16, 0.12], 0.0);
            b.rbox("frame", [-s, -s, 0.12], [s, s, lt], 0.0);
            b.rbox("frame", [-0.2, -0.2, lt], [0.2, 0.2, lt + 0.03], 0.0);
            b.rbox(
                "light",
                [-0.16, -0.16, lt + 0.03],
                [0.16, 0.16, hh - 0.1],
                0.0,
            );
            for (x, y) in [(-1.0, -1.0), (1.0, -1.0), (1.0, 1.0), (-1.0, 1.0)] {
                b.rbox(
                    "frame",
                    [x * 0.17 - 0.012, y * 0.17 - 0.012, lt + 0.03],
                    [x * 0.17 + 0.012, y * 0.17 + 0.012, hh - 0.1],
                    0.0,
                );
            }
            for z in [lt + 0.14, lt + 0.26] {
                for (lo, hi) in [
                    ([-0.17, -0.172, z], [0.17, -0.162, z + 0.012]),
                    ([-0.17, 0.162, z], [0.17, 0.172, z + 0.012]),
                    ([-0.172, -0.17, z], [-0.162, 0.17, z + 0.012]),
                    ([0.162, -0.17, z], [0.172, 0.17, z + 0.012]),
                ] {
                    b.rbox("frame", lo, hi, 0.0);
                }
            }
            b.rbox(
                "frame",
                [-0.24, -0.24, hh - 0.1],
                [0.24, 0.24, hh - 0.06],
                0.0,
            );
            b.rbox("frame", [-0.14, -0.14, hh - 0.06], [0.14, 0.14, hh], 0.0);
        }
        // Contemporary: a slim square column with a blade of light on
        // its top.
        4 => {
            let s = r * 0.8;
            b.rbox(
                "metal",
                [-s * 1.6, -s * 1.6, 0.0],
                [s * 1.6, s * 1.6, 0.02],
                0.0,
            );
            b.rbox("metal", [-s, -s, 0.02], [s, s, h - 0.06], 0.0);
            let reach = arm.max(0.5);
            b.rbox("metal", [-0.06, -s - 0.04, h - 0.06], [0.06, reach, h], 0.0);
            b.rbox(
                "light",
                [-0.045, s + 0.05, h - 0.068],
                [0.045, reach - 0.03, h - 0.06],
                0.0,
            );
        }
        // Rustic: a timber post, an iron bracket with its brace and a
        // coach lantern.
        5 => {
            let s = 0.08;
            b.rbox("mat", [-0.17, -0.17, 0.0], [0.17, 0.17, 0.25], 0.0);
            b.rbox("frame", [-s, -s, 0.25], [s, s, h], 0.0);
            b.rbox(
                "frame",
                [-s - 0.015, -s - 0.015, h - 0.02],
                [s + 0.015, s + 0.015, h + 0.03],
                0.0,
            );
            let z = h - 0.18;
            b.rbox(
                "metal",
                [-0.015, s, z - 0.015],
                [0.015, arm, z + 0.015],
                0.0,
            );
            bar(
                b,
                "metal",
                [0.0, s, z - 0.5],
                [0.0, arm * 0.6, z],
                0.025,
                [1.0, 0.0, 0.0],
            );
            lantern(
                b,
                "metal",
                [0.0, arm - 0.08, z - 0.015],
                lamp * 0.7,
                lamp * 0.9,
            );
        }
        // Industrial: a steel pole, a gooseneck arm and an enamelled
        // dish.
        6 => {
            b.rbox("metal", [-0.14, -0.14, 0.0], [0.14, 0.14, 0.015], 0.0);
            b.cyl("metal", [0.0, 0.0, 0.0], [0.0, 0.0, h - 0.3], r * 0.9);
            let reach = arm.max(0.5);
            let neck = curve(
                &yz(
                    0.0,
                    &[
                        (0.0, h - 0.4),
                        (0.0, h - 0.2),
                        (reach * 0.3, h - 0.02),
                        (reach * 0.75, h - 0.03),
                        (reach, h - 0.2),
                    ],
                ),
                5,
            );
            tube(b, "metal", &neck, false, 0.025, 8);
            let c = [0.0, reach, h - 0.2];
            turned(
                b,
                "dark",
                c,
                &[
                    (0.0, 0.02),
                    (0.05, 0.02),
                    (0.08, -0.04),
                    (lamp * 0.55, -lamp * 0.25),
                    (lamp * 0.55, -lamp * 0.27),
                    (0.0, -lamp * 0.14),
                ],
                24,
                0.0,
            );
            b.ellipsoid(
                "light",
                add(c, [0.0, 0.0, -lamp * 0.17]),
                [0.07, 0.07, 0.08],
            );
        }
        // Classic: the workshop's cast iron column with its lantern.
        _ => {
            let mut q = p.clone();
            q.set("height", h.min(6.0));
            q.set("pole", (r * 2.0).clamp(0.08, 0.3));
            q.set("lamp", (lamp * 0.85).clamp(0.25, 0.8));
            let mut own = Builder::default();
            lamp::streetlight(&mut own, &q);
            // Its iron is this family's `metal`.
            let iron = own.parts.remove("mat").unwrap_or_default();
            own.parts.entry("metal").or_default().extend(iron);
            let base = b.positions.len() as u32;
            b.positions.extend(own.positions);
            for (role, tris) in own.parts {
                b.parts
                    .entry(role)
                    .or_default()
                    .extend(tris.into_iter().map(|t| t.map(|v| v + base)));
            }
        }
    }
    true
}

// ---------------------------------------------------------- litter bins

/// Litter bins of the collections (after the family's 3 own models):
/// centred on the origin; `width` and `height`.
pub fn litter_bin(b: &mut Builder, p: &Params) -> bool {
    let Some(ix) = look(p, 3) else {
        return false;
    };
    let r = p.get("width", 0.4).clamp(0.2, 1.0) / 2.0;
    let h = p.get("height", 0.9).clamp(0.4, 1.4);
    match ix {
        // Nordic: a ring of oak slats bound by two black hoops, the
        // liner showing at its mouth.
        0 => {
            b.cone(
                "dark",
                [0.0, 0.0, 0.02],
                [0.0, 0.0, h - 0.03],
                r * 0.86,
                r * 0.86,
                20,
            );
            let n = 16;
            let half = TAU * r / n as f64 * 0.38;
            for k in 0..n {
                let a = TAU * k as f64 / n as f64;
                let (c, s) = (a.cos(), a.sin());
                b.obox(
                    "frame",
                    [r * 0.94 * c, r * 0.94 * s, h / 2.0],
                    [-s * half, c * half, 0.0],
                    [c * 0.011, s * 0.011, 0.0],
                    [0.0, 0.0, h / 2.0],
                );
            }
            for z in [0.12, h - 0.08] {
                tube(
                    b,
                    "metal",
                    &ring([0.0, 0.0, z], r * 0.96 + 0.006, 32),
                    true,
                    0.008,
                    6,
                );
            }
        }
        // Mediterranean: a whitewashed pedestal with a band of tiles and
        // a stone cap round its mouth.
        1 => {
            b.rbox("masonry", [-r, -r, 0.0], [r, r, h - 0.06], 0.0);
            b.rbox(
                "accent",
                [-r - 0.006, -r - 0.006, h - 0.24],
                [r + 0.006, r + 0.006, h - 0.14],
                0.0,
            );
            b.rbox(
                "top",
                [-r - 0.025, -r - 0.025, h - 0.06],
                [r + 0.025, r + 0.025, h],
                0.0,
            );
            b.cone(
                "dark",
                [0.0, 0.0, h - 0.01],
                [0.0, 0.0, h + 0.003],
                r * 0.62,
                r * 0.62,
                24,
            );
        }
        // Fifties: a coloured drum on three splayed legs under a domed
        // lid.
        2 => {
            for k in 0..3 {
                let a = PI / 2.0 + TAU * k as f64 / 3.0;
                tapered_leg(
                    b,
                    "metal",
                    [r * 0.5 * a.cos(), r * 0.5 * a.sin()],
                    0.34,
                    [a.cos(), a.sin()],
                    0.06,
                    0.012,
                );
            }
            turned(
                b,
                "accent",
                [0.0, 0.0, 0.3],
                &[
                    (0.0, 0.0),
                    (r * 0.72, 0.0),
                    (r, h - 0.45),
                    (r, h - 0.42),
                    (0.0, h - 0.42),
                ],
                28,
                0.0,
            );
            turned(
                b,
                "dark",
                [0.0, 0.0, h - 0.12],
                &[(0.0, 0.0), (r * 0.98, 0.0), (r * 0.98, 0.03), (0.0, 0.03)],
                28,
                0.0,
            );
            turned(
                b,
                "metal",
                [0.0, 0.0, h - 0.09],
                &[
                    (0.0, 0.0),
                    (r * 1.03, 0.0),
                    (r * 0.9, 0.05),
                    (r * 0.45, 0.09),
                    (0.03, 0.1),
                    (0.0, 0.13),
                ],
                28,
                0.0,
            );
        }
        // Japandi: a square box of smoked oak slats under a stone cap.
        3 => {
            b.rbox(
                "dark",
                [-r * 0.9, -r * 0.9, 0.02],
                [r * 0.9, r * 0.9, h - 0.05],
                0.0,
            );
            let n = 6;
            let step = 2.0 * r / n as f64;
            for k in 0..n {
                let u = -r + step * (k as f64 + 0.5);
                let hw = step * 0.38;
                for (lo, hi) in [
                    ([u - hw, -r, 0.0], [u + hw, -r + 0.018, h - 0.05]),
                    ([u - hw, r - 0.018, 0.0], [u + hw, r, h - 0.05]),
                    ([-r, u - hw, 0.0], [-r + 0.018, u + hw, h - 0.05]),
                    ([r - 0.018, u - hw, 0.0], [r, u + hw, h - 0.05]),
                ] {
                    b.rbox("frame", lo, hi, 0.0);
                }
            }
            b.rbox(
                "mat",
                [-r - 0.02, -r - 0.02, h - 0.05],
                [r + 0.02, r + 0.02, h],
                0.0,
            );
            b.cone(
                "dark",
                [0.0, 0.0, h - 0.01],
                [0.0, 0.0, h + 0.003],
                r * 0.55,
                r * 0.55,
                24,
            );
        }
        // Contemporary: a tall prism of concrete with a steel top and a
        // slot.
        4 => {
            let dy = r * 0.75;
            b.rbox("mat", [-r, -dy, 0.0], [r, dy, h - 0.025], 0.0);
            b.rbox(
                "metal",
                [-r - 0.005, -dy - 0.005, h - 0.025],
                [r + 0.005, dy + 0.005, h],
                0.0,
            );
            b.rbox(
                "dark",
                [-r * 0.65, dy, h - 0.28],
                [r * 0.65, dy + 0.004, h - 0.13],
                0.0,
            );
        }
        // Rustic: a barrel with its hoops and a wooden lid.
        5 => {
            turned(
                b,
                "frame",
                [0.0; 3],
                &[
                    (0.0, 0.0),
                    (r * 0.84, 0.0),
                    (r, h * 0.5),
                    (r * 0.84, h - 0.03),
                    (0.0, h - 0.03),
                ],
                24,
                0.0,
            );
            for (z, rr) in [
                (0.1, 0.88),
                (h * 0.36, 0.98),
                (h * 0.64, 0.98),
                (h - 0.12, 0.88),
            ] {
                tube(
                    b,
                    "metal",
                    &ring([0.0, 0.0, z], r * rr + 0.006, 32),
                    true,
                    0.009,
                    6,
                );
            }
            turned(
                b,
                "frame",
                [0.0, 0.0, h - 0.03],
                &[(0.0, 0.0), (r * 0.88, 0.0), (r * 0.88, 0.035), (0.0, 0.035)],
                24,
                0.0,
            );
            b.rbox(
                "metal",
                [-0.05, -0.012, h + 0.005],
                [0.05, 0.012, h + 0.04],
                0.0,
            );
        }
        // Industrial: a drum of weathering steel hung between two posts,
        // to tip.
        6 => {
            for x in [-r - 0.05, r + 0.05] {
                b.rbox(
                    "metal",
                    [x - 0.025, -0.025, 0.0],
                    [x + 0.025, 0.025, h],
                    0.0,
                );
                b.cone(
                    "metal",
                    [x * 0.7, 0.0, h - 0.18],
                    [x, 0.0, h - 0.18],
                    0.015,
                    0.015,
                    8,
                );
            }
            b.cone(
                "mat",
                [0.0, 0.0, 0.22],
                [0.0, 0.0, h - 0.04],
                r * 0.92,
                r,
                24,
            );
            tube(
                b,
                "metal",
                &ring([0.0, 0.0, h - 0.04], r + 0.004, 32),
                true,
                0.01,
                6,
            );
            b.cone(
                "dark",
                [0.0, 0.0, h - 0.05],
                [0.0, 0.0, h - 0.035],
                r * 0.92,
                r * 0.92,
                24,
            );
        }
        // Classic: a fluted urn of cast iron under a domed lid with its
        // finial.
        _ => {
            turned(
                b,
                "metal",
                [0.0; 3],
                &[
                    (0.0, 0.0),
                    (r * 0.75, 0.0),
                    (r * 0.75, 0.04),
                    (r * 0.55, 0.09),
                    (r * 0.7, 0.24),
                    (r, h - 0.22),
                    (r * 1.06, h - 0.14),
                    (r * 0.96, h - 0.1),
                    (0.0, h - 0.1),
                ],
                28,
                0.0,
            );
            for k in 0..14 {
                let a = TAU * k as f64 / 14.0;
                let lo = [r * 0.73 * a.cos(), r * 0.73 * a.sin(), 0.26];
                let hi = [r * 0.99 * a.cos(), r * 0.99 * a.sin(), h - 0.24];
                b.cone("metal", lo, hi, 0.009, 0.012, 6);
            }
            turned(
                b,
                "metal",
                [0.0, 0.0, h - 0.1],
                &[
                    (0.0, 0.0),
                    (r * 0.94, 0.0),
                    (r * 0.62, 0.07),
                    (0.05, 0.12),
                    (0.03, 0.16),
                    (0.0, 0.16),
                ],
                28,
                0.0,
            );
            b.ellipsoid("metal", [0.0, 0.0, h + 0.09], [0.035, 0.035, 0.035]);
        }
    }
    true
}

// ------------------------------------------------------------- bollards

/// Bollards of the collections (after the family's 3 own models):
/// centred on the origin; `width` and `height`.
pub fn bollard(b: &mut Builder, p: &Params) -> bool {
    let Some(ix) = look(p, 3) else {
        return false;
    };
    let r = p.get("width", 0.15).clamp(0.06, 0.6) / 2.0;
    let h = p.get("height", 0.8).clamp(0.3, 1.5);
    match ix {
        // Nordic: a round post of oak, its top rounded, a black band.
        0 => {
            turned(
                b,
                "frame",
                [0.0; 3],
                &[
                    (0.0, 0.0),
                    (r, 0.0),
                    (r, h - r * 0.7),
                    (r * 0.75, h - r * 0.2),
                    (0.0, h),
                ],
                20,
                0.0,
            );
            b.cone(
                "metal",
                [0.0, 0.0, h * 0.62],
                [0.0, 0.0, h * 0.62 + 0.035],
                r * 1.04,
                r * 1.04,
                20,
            );
        }
        // Mediterranean: a whitewashed cone with its rounded head, as
        // the guard stones on the corners of old streets.
        1 => turned(
            b,
            "masonry",
            [0.0; 3],
            &[
                (0.0, 0.0),
                (r * 1.5, 0.0),
                (r * 1.15, h * 0.82),
                (r * 0.85, h - 0.03),
                (0.0, h),
            ],
            20,
            0.0,
        ),
        // Fifties: a tapering tube under a coloured mushroom cap.
        2 => {
            b.cone(
                "metal",
                [0.0, 0.0, 0.0],
                [0.0, 0.0, h - 0.08],
                r * 0.62,
                r * 0.45,
                20,
            );
            turned(
                b,
                "accent",
                [0.0, 0.0, h - 0.1],
                &[
                    (0.0, 0.0),
                    (r * 0.5, 0.0),
                    (r * 1.25, 0.03),
                    (r * 1.2, 0.07),
                    (r * 0.5, 0.1),
                    (0.0, 0.1),
                ],
                20,
                0.0,
            );
        }
        // Japandi: a square post of stone with its top chamfered.
        3 => {
            let rr = r * SQRT_2;
            turned(
                b,
                "mat",
                [0.0; 3],
                &[
                    (0.0, 0.0),
                    (rr, 0.0),
                    (rr, h - 0.03),
                    (rr * 0.8, h),
                    (0.0, h),
                ],
                4,
                FRAC_PI_4,
            );
        }
        // Contemporary: a slim square of steel with a slot of light.
        4 => {
            let s = r * 0.7;
            b.rbox("metal", [-s, -s, 0.0], [s, s, h - 0.13], 0.0);
            b.rbox(
                "light",
                [-s * 0.96, -s * 0.96, h - 0.13],
                [s * 0.96, s * 0.96, h - 0.1],
                0.0,
            );
            b.rbox("metal", [-s, -s, h - 0.1], [s, s, h], 0.0);
        }
        // Rustic: a rough post of stone.
        5 => turned(
            b,
            "mat",
            [0.0; 3],
            &[
                (0.0, 0.0),
                (r * 1.4, 0.0),
                (r * 1.3, h * 0.6),
                (r * 1.15, h - 0.06),
                (r * 0.6, h),
                (0.0, h),
            ],
            7,
            0.3,
        ),
        // Industrial: a square of weathering steel on a plate, capped.
        6 => {
            let s = r * 0.7;
            b.rbox(
                "metal",
                [-s - 0.04, -s - 0.04, 0.0],
                [s + 0.04, s + 0.04, 0.012],
                0.0,
            );
            b.rbox("mat", [-s, -s, 0.012], [s, s, h - 0.012], 0.0);
            b.rbox(
                "metal",
                [-s - 0.006, -s - 0.006, h - 0.012],
                [s + 0.006, s + 0.006, h],
                0.0,
            );
        }
        // Classic: cast iron, turned with its rings and a ball on top.
        _ => {
            turned(
                b,
                "metal",
                [0.0; 3],
                &[
                    (0.0, 0.0),
                    (r * 1.4, 0.0),
                    (r * 1.4, 0.04),
                    (r * 1.05, 0.08),
                    (r * 0.9, 0.14),
                    (r * 0.78, h - r * 1.6 - 0.12),
                    (r * 1.0, h - r * 1.6 - 0.09),
                    (r * 1.0, h - r * 1.6 - 0.06),
                    (r * 0.6, h - r * 1.6 - 0.03),
                    (r * 0.45, h - r * 1.6),
                    (0.0, h - r * 1.6),
                ],
                20,
                0.0,
            );
            b.ellipsoid(
                "metal",
                [0.0, 0.0, h - r * 0.85],
                [r * 0.85, r * 0.85, r * 0.85],
            );
        }
    }
    true
}

// ------------------------------------------------------------- planters

/// Planters of the collections (after the family's 3 own models): the
/// back at `y` 0, the middle at `x` 0; `width`, `depth`, `height`. Their
/// plants are the terraces' (olive, grasses or lavender, as the
/// collection's big pot), a few along it.
pub fn planter(b: &mut Builder, p: &Params) -> bool {
    let Some(ix) = look(p, 3) else {
        return false;
    };
    let w = p.get("width", 1.2).clamp(0.4, 6.0);
    let d = p.get("depth", 0.5).clamp(0.3, 2.0);
    let h = p.get("height", 0.5).clamp(0.2, 1.2);
    let (x0, x1) = (-w / 2.0, w / 2.0);
    // The rim: how thick the walls are, and how high the earth comes.
    let (t, earth) = match ix {
        // Nordic: a trough on black legs, an oak rail round its rim.
        0 => {
            for (x, y) in [
                (x0 + 0.04, 0.04),
                (x1 - 0.04, 0.04),
                (x1 - 0.04, d - 0.04),
                (x0 + 0.04, d - 0.04),
            ] {
                b.rbox(
                    "metal",
                    [x - 0.015, y - 0.015, 0.0],
                    [x + 0.015, y + 0.015, 0.14],
                    0.0,
                );
            }
            trough(b, "pot", [x0, 0.0, 0.12], [x1, d, h - 0.03], 0.03);
            for (lo, hi) in [
                ([x0 - 0.01, -0.01, h - 0.03], [x1 + 0.01, 0.04, h]),
                ([x0 - 0.01, d - 0.04, h - 0.03], [x1 + 0.01, d + 0.01, h]),
                ([x0 - 0.01, 0.04, h - 0.03], [x0 + 0.04, d - 0.04, h]),
                ([x1 - 0.04, 0.04, h - 0.03], [x1 + 0.01, d - 0.04, h]),
            ] {
                b.rbox("frame", lo, hi, 0.0);
            }
            (0.04, h - 0.06)
        }
        // Mediterranean: whitewashed work with a coping of tiles.
        1 => {
            trough(b, "masonry", [x0, 0.0, 0.0], [x1, d, h - 0.03], 0.1);
            for (lo, hi) in [
                ([x0 - 0.02, -0.02, h - 0.03], [x1 + 0.02, 0.11, h]),
                ([x0 - 0.02, d - 0.11, h - 0.03], [x1 + 0.02, d + 0.02, h]),
                ([x0 - 0.02, 0.11, h - 0.03], [x0 + 0.11, d - 0.11, h]),
                ([x1 - 0.11, 0.11, h - 0.03], [x1 + 0.02, d - 0.11, h]),
            ] {
                b.rbox("accent", lo, hi, 0.0);
            }
            (0.11, h - 0.06)
        }
        // Fifties: a coloured trough on splayed legs.
        2 => {
            for (x, y, sx, sy) in [
                (x0 + 0.08, 0.08, -0.6, -0.6),
                (x1 - 0.08, 0.08, 0.6, -0.6),
                (x1 - 0.08, d - 0.08, 0.6, 0.6),
                (x0 + 0.08, d - 0.08, -0.6, 0.6),
            ] {
                tapered_leg(b, "metal", [x, y], 0.2, [sx, sy], 0.05, 0.012);
            }
            trough(b, "pot", [x0, 0.0, 0.18], [x1, d, h], 0.025);
            (0.025, h - 0.04)
        }
        // Japandi: a low trough of smoked oak boards.
        3 => {
            trough(b, "frame", [x0, 0.0, 0.0], [x1, d, h], 0.035);
            for x in [x0 + 0.02, x1 - 0.02] {
                b.rbox(
                    "frame",
                    [x - 0.022, -0.01, 0.0],
                    [x + 0.022, d + 0.01, h + 0.01],
                    0.0,
                );
            }
            (0.035, h - 0.05)
        }
        // Contemporary: a box of concrete with thin walls.
        4 => {
            trough(b, "pot", [x0, 0.0, 0.0], [x1, d, h], 0.05);
            (0.05, h - 0.05)
        }
        // Rustic: a trough of old boards bound with iron at its
        // corners.
        5 => {
            trough(b, "frame", [x0, 0.0, 0.0], [x1, d, h], 0.04);
            for (x, y) in [(x0, 0.0), (x1, 0.0), (x1, d), (x0, d)] {
                let sx = if x < 0.0 { -1.0 } else { 1.0 };
                let sy = if y < 0.01 { -1.0 } else { 1.0 };
                b.rbox(
                    "metal",
                    [x.min(x - sx * 0.004), y.min(y + sy * 0.004) - 0.0, 0.03],
                    [x.max(x - sx * 0.004) + 0.0, y.max(y + sy * 0.004), h - 0.03],
                    0.0,
                );
                b.rbox(
                    "metal",
                    [
                        x - 0.05 * (1.0 + sx) / 2.0 - 0.0,
                        y.min(y + sy * 0.005),
                        0.08,
                    ],
                    [x + 0.05 * (1.0 - sx) / 2.0, y.max(y + sy * 0.005), 0.12],
                    0.0,
                );
            }
            (0.04, h - 0.05)
        }
        // Industrial: a box of weathering steel with its lip folded out.
        6 => {
            trough(b, "mat", [x0, 0.0, 0.0], [x1, d, h], 0.01);
            for (lo, hi) in [
                ([x0 - 0.015, -0.015, h - 0.012], [x1 + 0.015, 0.0, h]),
                ([x0 - 0.015, d, h - 0.012], [x1 + 0.015, d + 0.015, h]),
                ([x0 - 0.015, 0.0, h - 0.012], [x0, d, h]),
                ([x1, 0.0, h - 0.012], [x1 + 0.015, d, h]),
            ] {
                b.rbox("mat", lo, hi, 0.0);
            }
            (0.01, h - 0.05)
        }
        // Classic: a trough of limestone on its plinth, a moulded rim.
        _ => {
            b.rbox(
                "mat",
                [x0 - 0.02, -0.02, 0.0],
                [x1 + 0.02, d + 0.02, 0.07],
                0.0,
            );
            trough(
                b,
                "mat",
                [x0 + 0.02, 0.02, 0.07],
                [x1 - 0.02, d - 0.02, h - 0.05],
                0.06,
            );
            for (lo, hi) in [
                ([x0 - 0.03, -0.03, h - 0.05], [x1 + 0.03, 0.1, h]),
                ([x0 - 0.03, d - 0.1, h - 0.05], [x1 + 0.03, d + 0.03, h]),
                ([x0 - 0.03, 0.1, h - 0.05], [x0 + 0.1, d - 0.1, h]),
                ([x1 - 0.1, 0.1, h - 0.05], [x1 + 0.03, d - 0.1, h]),
            ] {
                b.rbox("top", lo, hi, 0.0);
            }
            (0.1, h - 0.06)
        }
    };
    // The earth, below the rim.
    b.rbox(
        "soil",
        [x0 + t, t, earth - 0.012],
        [x1 - t, d - t, earth],
        0.0,
    );
    // Its plants along it, as the collection's big pot.
    let inside = (w - 2.0 * t).max(0.2);
    let (each, most) = match ix {
        1 | 4 | 7 => (1.6, 2),
        2 | 5 => (0.6, 2),
        _ => (0.6, 3),
    };
    let n = ((inside / each).round() as usize).clamp(1, most);
    for k in 0..n {
        let x = x0 + t + inside * (k as f64 + 0.5) / n as f64;
        let foot = [x, d / 2.0, earth];
        let room = (inside / n as f64).min(d * 1.4);
        match ix {
            1 | 4 | 7 => garden_plants::olive(b, foot, 1.3, room.clamp(0.7, 1.1)),
            2 | 5 => garden_plants::lavender(b, foot, 0.55, room.clamp(0.4, 0.65)),
            _ => garden_plants::grasses(b, foot, 0.6, room.clamp(0.45, 0.8)),
        }
    }
    true
}

// ------------------------------------------------------------ bike racks

/// Bike racks of the collections (after the family's 3 own models):
/// `count` stands `step` apart along `x` from the origin, `width` across
/// `y` centred on it, `height`.
pub fn bike_rack(b: &mut Builder, p: &Params) -> bool {
    let Some(ix) = look(p, 3) else {
        return false;
    };
    let n = p.get("count", 5.0).round().clamp(1.0, 20.0) as usize;
    let step = p.get("step", 0.8).clamp(0.5, 1.5);
    let w = p.get("width", 0.7).clamp(0.4, 1.2);
    let h = p.get("height", 0.8).clamp(0.5, 1.2);
    let hw = w / 2.0;
    // An inverted U across `y` at `x`.
    let hoop = |x: f64, k: f64, top: f64| -> Vec<P3> {
        let r = (hw * k).min(0.2);
        let mut pts = vec![[x, -hw, 0.0], [x, -hw, top - r]];
        for s in 1..6 {
            let a = PI - PI / 2.0 * s as f64 / 6.0;
            pts.push([x, -hw + r + r * a.cos(), top - r + r * a.sin()]);
        }
        pts.push([x, -hw + r, top]);
        pts.push([x, hw - r, top]);
        for s in 1..6 {
            let a = PI / 2.0 - PI / 2.0 * s as f64 / 6.0;
            pts.push([x, hw - r + r * a.cos(), top - r + r * a.sin()]);
        }
        pts.push([x, hw, top - r]);
        pts.push([x, hw, 0.0]);
        pts
    };
    if ix == 5 {
        // Rustic: a log rail on posts, with notches for the wheels.
        let end = step * (n - 1) as f64;
        let z = h * 0.55;
        b.cone(
            "frame",
            [-0.35, 0.0, z],
            [end + 0.35, 0.0, z],
            0.07,
            0.07,
            14,
        );
        let mut x = -0.25;
        while x <= end + 0.26 {
            b.cone("frame", [x, 0.0, 0.0], [x, 0.0, z + 0.05], 0.08, 0.075, 12);
            x += (step * 3.0).max(1.2);
        }
        for i in 0..n {
            let x = step * i as f64;
            b.rbox(
                "metal",
                [x - 0.035, -0.08, z + 0.05],
                [x + 0.035, 0.08, z + 0.075],
                0.0,
            );
        }
        return true;
    }
    for i in 0..n {
        let x = step * i as f64;
        match ix {
            // Nordic: black tube with an oak grip on its top.
            0 => {
                tube(b, "metal", &hoop(x, 0.4, h), false, 0.022, 10);
                b.cone(
                    "frame",
                    [x, -hw * 0.45, h],
                    [x, hw * 0.45, h],
                    0.034,
                    0.034,
                    14,
                );
            }
            // Mediterranean: forged iron with scrolls under its top.
            1 => {
                tube(b, "metal", &hoop(x, 0.35, h), false, 0.016, 8);
                for s in [-1.0, 1.0] {
                    curl(
                        b,
                        "metal",
                        [x, s * hw * 0.45, h - 0.09],
                        [0.0, -s, 0.0],
                        0.06,
                        1.1,
                        PI / 2.0,
                        0.009,
                    );
                }
                for y in [-hw, hw] {
                    turned(
                        b,
                        "metal",
                        [x, y, 0.0],
                        &[(0.0, 0.0), (0.05, 0.0), (0.03, 0.03), (0.0, 0.03)],
                        10,
                        0.0,
                    );
                }
            }
            // Fifties: a chromed hairpin on round feet.
            2 => {
                tube(b, "metal", &hoop(x, 0.9, h), false, 0.02, 10);
                for y in [-hw, hw] {
                    b.ellipsoid("accent", [x, y, 0.03], [0.045, 0.045, 0.03]);
                }
            }
            // Japandi: two posts of smoked oak and a beam.
            3 => {
                for y in [-hw + 0.03, hw - 0.03] {
                    b.rbox(
                        "frame",
                        [x - 0.03, y - 0.03, 0.0],
                        [x + 0.03, y + 0.03, h - 0.06],
                        0.0,
                    );
                }
                b.rbox(
                    "frame",
                    [x - 0.03, -hw - 0.03, h - 0.06],
                    [x + 0.03, hw + 0.03, h],
                    0.0,
                );
            }
            // Contemporary: a flat bar bent square.
            4 => {
                for y in [-hw + 0.03, hw - 0.03] {
                    b.rbox(
                        "metal",
                        [x - 0.006, y - 0.03, 0.0],
                        [x + 0.006, y + 0.03, h - 0.03],
                        0.0,
                    );
                }
                b.rbox("metal", [x - 0.006, -hw, h - 0.03], [x + 0.006, hw, h], 0.0);
            }
            // Industrial: square bar of weathering steel.
            6 => {
                for y in [-hw + 0.025, hw - 0.025] {
                    b.rbox(
                        "mat",
                        [x - 0.025, y - 0.025, 0.0],
                        [x + 0.025, y + 0.025, h - 0.05],
                        0.0,
                    );
                }
                b.rbox("mat", [x - 0.025, -hw, h - 0.05], [x + 0.025, hw, h], 0.0);
                b.rbox(
                    "metal",
                    [x - 0.06, -hw - 0.04, 0.0],
                    [x + 0.06, -hw + 0.09, 0.01],
                    0.0,
                );
                b.rbox(
                    "metal",
                    [x - 0.06, hw - 0.09, 0.0],
                    [x + 0.06, hw + 0.04, 0.01],
                    0.0,
                );
            }
            // Classic: cast iron with ball finials and rosettes.
            _ => {
                tube(b, "metal", &hoop(x, 0.3, h), false, 0.02, 10);
                for y in [-hw, hw] {
                    turned(
                        b,
                        "metal",
                        [x, y, 0.0],
                        &[
                            (0.0, 0.0),
                            (0.06, 0.0),
                            (0.05, 0.02),
                            (0.03, 0.05),
                            (0.0, 0.05),
                        ],
                        12,
                        0.0,
                    );
                }
                for y in [-hw * 0.7, hw * 0.7] {
                    b.ellipsoid("metal", [x, y, h + 0.03], [0.03, 0.03, 0.03]);
                }
            }
        }
    }
    true
}

// ------------------------------------------------------------- fountains

/// A bowl: its rim at `c`, `r` across, `depth` deep inside, `t` thick,
/// with its water a little below the rim.
fn bowl(b: &mut Builder, role: &'static str, c: P3, r: f64, depth: f64, t: f64) {
    turned(
        b,
        role,
        c,
        &[
            (0.0, -depth - t),
            (r * 0.5, -depth - t),
            (r, -t),
            (r, 0.0),
            (r - t, 0.0),
            (r * 0.45, -depth),
            (0.0, -depth),
        ],
        28,
        0.0,
    );
    let wz = 0.02;
    let k = wz / depth.max(0.02);
    let wr = (r - t) - ((r - t) - r * 0.45) * k - 0.004;
    turned(
        b,
        "water",
        add(c, [0.0, 0.0, -wz - 0.004]),
        &[(0.0, 0.0), (wr, 0.0), (wr, 0.004), (0.0, 0.004)],
        28,
        0.0,
    );
}

/// A spout from `a` out along `+y` by `reach` and down.
fn spout(b: &mut Builder, role: &'static str, a: P3, reach: f64) {
    tube(
        b,
        role,
        &curve(
            &[
                a,
                add(a, [0.0, reach * 0.7, 0.01]),
                add(a, [0.0, reach, -0.04]),
            ],
            4,
        ),
        false,
        0.012,
        8,
    );
}

/// Drinking fountains of the collections (after the family's 3 own
/// models): the back at `y` 0, the middle at `x` 0; `width`, `depth`,
/// `height` (of the basin).
pub fn fountain(b: &mut Builder, p: &Params) -> bool {
    let Some(ix) = look(p, 3) else {
        return false;
    };
    let w = p.get("width", 0.35).clamp(0.2, 1.2);
    let d = p.get("depth", 0.35).clamp(0.2, 1.2);
    let h = p.get("height", 1.0).clamp(0.6, 1.4);
    match ix {
        // Nordic: a column clad in oak with a steel bowl on its top.
        0 => {
            let a = w * 0.3;
            b.rbox(
                "frame",
                [-a, d / 2.0 - a, 0.0],
                [a, d / 2.0 + a, h - 0.1],
                0.0,
            );
            b.cone(
                "metal",
                [0.0, d / 2.0, h - 0.12],
                [0.0, d / 2.0, h - 0.06],
                a * 0.8,
                a * 0.5,
                20,
            );
            bowl(b, "metal", [0.0, d / 2.0, h], w / 2.0, 0.06, 0.006);
            b.cone(
                "metal",
                [0.0, d / 2.0, h - 0.06],
                [0.0, d / 2.0, h + 0.04],
                0.012,
                0.01,
                10,
            );
            b.cone(
                "metal",
                [0.0, d / 2.0 + a, 0.7],
                [0.0, d / 2.0 + a + 0.02, 0.7],
                0.02,
                0.02,
                12,
            );
        }
        // Mediterranean: a whitewashed wall with a panel of tiles, a
        // spout and a stone basin on its pier.
        1 => {
            let hw = (w * 1.3).max(0.45);
            b.rbox("masonry", [-hw, 0.0, 0.0], [hw, 0.2, h + 0.45], 0.0);
            b.rbox(
                "top",
                [-hw - 0.02, -0.02, h + 0.45],
                [hw + 0.02, 0.22, h + 0.5],
                0.0,
            );
            b.rbox(
                "accent",
                [-hw + 0.08, 0.2, h - 0.05],
                [hw - 0.08, 0.212, h + 0.35],
                0.0,
            );
            spout(b, "metal", [0.0, 0.2, h + 0.06], 0.14);
            let bd = d.max(0.3);
            b.rbox(
                "masonry",
                [-hw * 0.6, 0.2, 0.0],
                [hw * 0.6, 0.2 + bd * 0.7, 0.45],
                0.0,
            );
            trough(
                b,
                "top",
                [-hw * 0.85, 0.2, 0.45],
                [hw * 0.85, 0.2 + bd, 0.6],
                0.04,
            );
            b.rbox(
                "water",
                [-hw * 0.85 + 0.04, 0.24, 0.55],
                [hw * 0.85 - 0.04, 0.2 + bd - 0.04, 0.565],
                0.0,
            );
        }
        // Fifties: a mushroom of terrazzo, its bowl and a bubbler.
        2 => {
            let c = [0.0, d / 2.0, 0.0];
            turned(
                b,
                "mat",
                c,
                &[
                    (0.0, 0.0),
                    (w * 0.55, 0.0),
                    (w * 0.5, 0.05),
                    (w * 0.16, 0.14),
                    (w * 0.12, h - 0.22),
                    (w * 0.3, h - 0.1),
                    (w * 0.55, h - 0.02),
                    (w * 0.55, h),
                    (w * 0.5, h),
                    (w * 0.22, h - 0.06),
                    (0.0, h - 0.06),
                ],
                28,
                0.0,
            );
            turned(
                b,
                "water",
                add(c, [0.0, 0.0, h - 0.025]),
                &[(0.0, 0.0), (w * 0.43, 0.0), (w * 0.43, 0.004), (0.0, 0.004)],
                28,
                0.0,
            );
            b.cone(
                "metal",
                add(c, [0.0, 0.0, h - 0.06]),
                add(c, [0.0, 0.0, h + 0.03]),
                0.016,
                0.012,
                10,
            );
            b.ellipsoid(
                "metal",
                add(c, [0.0, 0.0, h + 0.035]),
                [0.018, 0.018, 0.012],
            );
        }
        // Japandi: a basin of stone low on the ground, fed by a bamboo
        // pipe from its post.
        3 => {
            let c = [0.0, 0.2 + w * 0.8, 0.0];
            let r = w * 0.8;
            turned(
                b,
                "mat",
                c,
                &[
                    (0.0, 0.0),
                    (r * 0.9, 0.0),
                    (r, 0.25),
                    (r * 0.92, 0.35),
                    (r * 0.6, 0.35),
                    (r * 0.55, 0.25),
                    (0.0, 0.25),
                ],
                9,
                0.2,
            );
            turned(
                b,
                "water",
                add(c, [0.0, 0.0, 0.31]),
                &[(0.0, 0.0), (r * 0.57, 0.0), (r * 0.57, 0.004), (0.0, 0.004)],
                18,
                0.0,
            );
            b.cone(
                "frame",
                [0.0, 0.05, 0.0],
                [0.0, 0.05, 0.75],
                0.035,
                0.032,
                12,
            );
            b.cone(
                "frame",
                [0.0, 0.05, 0.66],
                [0.0, c[1] - r * 0.2, 0.5],
                0.02,
                0.02,
                10,
            );
            for (dx, dy, s) in [(-0.35, 0.1, 0.12), (0.32, 0.25, 0.09), (-0.2, 0.55, 0.07)] {
                b.blob(
                    "mat",
                    [c[0] + dx, c[1] + dy, s * 0.5],
                    [s, 0.0, 0.0],
                    [0.0, s * 0.8, 0.0],
                    [0.0, 0.0, s * 0.5],
                );
            }
        }
        // Contemporary: a prism of concrete with a steel top, a spout
        // and a grate at its foot.
        4 => {
            b.rbox("mat", [-w / 2.0, 0.0, 0.0], [w / 2.0, d, h - 0.01], 0.0);
            b.rbox("metal", [-w / 2.0, 0.0, h - 0.01], [w / 2.0, d, h], 0.0);
            b.rbox(
                "dark",
                [-w / 2.0 + 0.05, d * 0.35, h],
                [w / 2.0 - 0.05, d - 0.05, h + 0.002],
                0.0,
            );
            b.cone(
                "metal",
                [0.0, d * 0.2, h],
                [0.0, d * 0.2, h + 0.06],
                0.01,
                0.01,
                10,
            );
            b.cone(
                "metal",
                [0.0, d, 0.75],
                [0.0, d + 0.015, 0.75],
                0.02,
                0.02,
                12,
            );
            b.rbox(
                "metal",
                [-w / 2.0 - 0.05, d, 0.0],
                [w / 2.0 + 0.05, d + 0.35, 0.012],
                0.0,
            );
            for k in 0..6 {
                let y = d + 0.04 + 0.05 * k as f64;
                b.rbox(
                    "dark",
                    [-w / 2.0 - 0.02, y, 0.012],
                    [w / 2.0 + 0.02, y + 0.012, 0.014],
                    0.0,
                );
            }
        }
        // Rustic: a pillar of stone, an iron spout and a stone trough
        // at its foot.
        5 => {
            let hw = (w * 0.7).max(0.18);
            b.rbox("mat", [-hw, 0.0, 0.0], [hw, 0.3, h + 0.2], 0.0);
            b.rbox(
                "mat",
                [-hw - 0.03, -0.03, h + 0.2],
                [hw + 0.03, 0.33, h + 0.27],
                0.0,
            );
            spout(b, "metal", [0.0, 0.3, h], 0.16);
            let bd = d.max(0.35);
            trough(
                b,
                "top",
                [-hw * 1.6, 0.3, 0.0],
                [hw * 1.6, 0.3 + bd, 0.32],
                0.06,
            );
            b.rbox(
                "water",
                [-hw * 1.6 + 0.06, 0.36, 0.25],
                [hw * 1.6 - 0.06, 0.3 + bd - 0.06, 0.262],
                0.0,
            );
        }
        // Industrial: a column of weathering steel, a tap and a grate.
        6 => {
            b.rbox("mat", [-0.09, 0.0, 0.0], [0.09, 0.18, h], 0.0);
            b.rbox("mat", [-0.1, -0.01, h], [0.1, 0.19, h + 0.01], 0.0);
            b.cone(
                "metal",
                [0.0, 0.18, h - 0.12],
                [0.0, 0.28, h - 0.12],
                0.015,
                0.015,
                10,
            );
            b.cone(
                "metal",
                [0.0, 0.28, h - 0.12],
                [0.0, 0.28, h - 0.17],
                0.013,
                0.011,
                10,
            );
            b.cone(
                "metal",
                [0.0, 0.22, h - 0.12],
                [0.0, 0.22, h - 0.06],
                0.008,
                0.008,
                8,
            );
            b.rbox(
                "metal",
                [-0.04, 0.215, h - 0.06],
                [0.04, 0.225, h - 0.05],
                0.0,
            );
            b.rbox("metal", [-0.22, 0.18, 0.0], [0.22, 0.5, 0.015], 0.0);
            for k in 0..6 {
                let y = 0.21 + 0.045 * k as f64;
                b.rbox("dark", [-0.19, y, 0.015], [0.19, y + 0.012, 0.017], 0.0);
            }
        }
        // Classic: a cast iron column with its spout and a shell basin.
        _ => {
            let c = [0.0, w * 0.25, 0.0];
            turned(
                b,
                "metal",
                c,
                &[
                    (0.0, 0.0),
                    (w * 0.55, 0.0),
                    (w * 0.55, 0.05),
                    (w * 0.4, 0.1),
                    (w * 0.22, 0.18),
                    (w * 0.17, h - 0.1),
                    (w * 0.24, h - 0.06),
                    (w * 0.22, h),
                    (w * 0.12, h + 0.06),
                    (0.0, h + 0.06),
                ],
                24,
                0.0,
            );
            for k in 0..10 {
                let a = TAU * k as f64 / 10.0;
                b.cone(
                    "metal",
                    [c[0] + w * 0.2 * a.cos(), c[1] + w * 0.2 * a.sin(), 0.2],
                    [
                        c[0] + w * 0.16 * a.cos(),
                        c[1] + w * 0.16 * a.sin(),
                        h - 0.12,
                    ],
                    0.008,
                    0.007,
                    6,
                );
            }
            b.ellipsoid("metal", add(c, [0.0, 0.0, h + 0.1]), [0.045, 0.045, 0.045]);
            b.blob(
                "metal",
                add(c, [0.0, w * 0.17, h - 0.22]),
                [0.05, 0.0, 0.0],
                [0.0, 0.035, 0.0],
                [0.0, 0.0, 0.055],
            );
            spout(b, "metal", add(c, [0.0, w * 0.2, h - 0.24]), 0.08);
            bowl(
                b,
                "metal",
                add(c, [0.0, w * 0.17 + 0.16, 0.5]),
                0.17,
                0.07,
                0.01,
            );
            b.cone(
                "metal",
                add(c, [0.0, w * 0.17 + 0.16, 0.0]),
                add(c, [0.0, w * 0.17 + 0.16, 0.42]),
                0.04,
                0.025,
                12,
            );
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fine::shape::signed_volume;

    #[test]
    fn the_street_furniture_of_the_eight_collections() {
        type Make = fn(&mut Builder, &Params) -> bool;
        let pieces: [(&str, Make, i32); 7] = [
            ("banco", bench, 3),
            ("farola", streetlight, 4),
            ("papelera", litter_bin, 3),
            ("bolardo", bollard, 3),
            ("jardinera", planter, 3),
            ("aparcabicis", bike_rack, 3),
            ("fuente", fountain, 3),
        ];
        for c in 0..8 {
            for (name, make, first) in pieces {
                let mut p = Params::default();
                p.set("style", (first + 1 + c) as f64);
                let mut b = Builder::default();
                assert!(make(&mut b, &p), "{name}");
                let tris = b.triangle_count();
                assert!((20..40_000).contains(&tris), "{c} {name}: {tris}");
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
