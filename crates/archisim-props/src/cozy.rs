//! The cosy pieces that make a house lived in (revisión de Antonio del
//! 23/10/2026, «biblioteca acogedora»): rugs, lamps, plants in their pots,
//! armchairs and poufs, side tables, sideboards, curtains and the little
//! things on tables and shelves. Soft shapes (cushions that swell, rounded
//! edges, turned feet) and few pieces each, so a room full of them stays
//! light. Axes as the furniture's: origin in the middle of the back, `x`
//! to the right, `y` forwards, `z` up; round pieces stand centred on
//! `(0, depth / 2)`.

use crate::build::{Builder, P3};
use crate::fronts::{self, Handle};
use crate::Params;

use std::f64::consts::{PI, TAU};

/// Rugs: 1 plain with a border, 2 round, 3 striped, 4 Berber with fringes.
pub fn rug(b: &mut Builder, p: &Params) {
    let w = p.get("width", 2.0);
    let d = p.get("depth", 1.4);
    let t = p.get("thickness", 0.012).clamp(0.004, 0.04);
    let x0 = -w / 2.0;
    match p.style() {
        2 => {
            let r = w.min(d) / 2.0;
            let c = [0.0, d / 2.0, 0.0];
            b.lathe(
                "border",
                c,
                &[(0.0, 0.0), (r, 0.0), (r, t * 0.8), (r - 0.04, t), (0.0, t)],
                1.0,
                1.0,
            );
            b.lathe(
                "mat",
                c,
                &[
                    (0.0, 0.0),
                    (r - 0.06, 0.0),
                    (r - 0.06, t * 1.15),
                    (0.0, t * 1.15),
                ],
                1.0,
                1.0,
            );
        }
        3 => {
            let n = ((d / 0.18).round() as usize).max(3);
            let s = d / n as f64;
            for i in 0..n {
                let role = if i % 2 == 0 { "mat" } else { "border" };
                b.rbox(
                    role,
                    [x0, i as f64 * s, 0.0],
                    [-x0, (i + 1) as f64 * s, t],
                    0.0,
                );
            }
        }
        4 => {
            b.pillow("mat", [x0, 0.0, t * 0.4], [-x0, d, t * 1.9], 0.01, t * 0.4);
            // Fringes at both ends.
            let n = ((w / 0.04).round() as usize).max(4);
            for i in 0..n {
                let x = x0 + (i as f64 + 0.5) * w / n as f64;
                for (y0, y1) in [(-0.07, 0.0), (d, d + 0.07)] {
                    b.rbox("border", [x - 0.006, y0, 0.0], [x + 0.006, y1, 0.004], 0.0);
                }
            }
        }
        _ => {
            b.rbox("border", [x0, 0.0, 0.0], [-x0, d, t], 0.004);
            b.rbox(
                "mat",
                [x0 + 0.07, 0.07, 0.0],
                [-x0 - 0.07, d - 0.07, t * 1.15],
                0.003,
            );
        }
    }
}

/// Lamps: 1 floor lamp with a drum shade, 2 table lamp with a ceramic
/// foot, 3 pendant hanging from the ceiling (its `height` is where it
/// hangs), 4 arc floor lamp.
pub fn lamp(b: &mut Builder, p: &Params) {
    let h = p.get("height", 1.6);
    let shade = p.get("diameter", 0.45).clamp(0.15, 1.2);
    let c = [0.0, shade / 2.0];
    let r = shade / 2.0;
    match p.style() {
        2 => {
            // A round ceramic foot, the drum on top.
            let foot = (h * 0.55).max(0.12);
            b.lathe(
                "frame",
                [c[0], c[1], 0.0],
                &[
                    (0.0, 0.0),
                    (r * 0.42, 0.0),
                    (r * 0.55, foot * 0.35),
                    (r * 0.45, foot * 0.8),
                    (r * 0.12, foot),
                    (0.0, foot),
                ],
                1.0,
                1.0,
            );
            b.cyl("metal", [c[0], c[1], foot], [c[0], c[1], h - 0.02], 0.006);
            drum(b, [c[0], c[1], h - (h - foot) * 0.9], r, (h - foot) * 0.85);
        }
        3 => {
            // From the ceiling (`drop` below it) a cord and a dome.
            let drop = p.get("drop", 0.6).clamp(0.2, 2.0);
            let top = h + drop;
            b.cyl("metal", [c[0], c[1], h + 0.1], [c[0], c[1], top], 0.003);
            b.lathe(
                "shade",
                [c[0], c[1], h - 0.02],
                &[
                    (r, 0.0),
                    (r * 0.97, 0.06),
                    (r * 0.8, 0.14),
                    (r * 0.45, 0.2),
                    (0.04, 0.12 + r * 0.25),
                    (0.0, 0.12 + r * 0.25),
                ],
                1.0,
                1.0,
            );
            b.ellipsoid("bulb", [c[0], c[1], h - 0.05], [0.045, 0.045, 0.04]);
            b.cyl("metal", [c[0], c[1], top - 0.02], [c[0], c[1], top], 0.05);
        }
        4 => {
            // A heavy base, a bowed arm and a hanging bowl.
            let reach = (shade * 2.6).clamp(0.8, 2.2);
            b.cone("frame", [0.0, 0.2, 0.0], [0.0, 0.2, 0.05], 0.16, 0.15, 24);
            let steps = 14;
            let mut last = [0.0, 0.2, 0.05];
            for i in 1..=steps {
                let a = PI / 2.0 * i as f64 / steps as f64;
                let q = [reach * (1.0 - a.cos()), 0.2, 0.05 + (h - 0.05) * a.sin()];
                b.cyl("metal", last, q, 0.012);
                last = q;
            }
            b.lathe(
                "shade",
                [reach, 0.2, h - 0.32],
                &[(r, 0.0), (r * 0.8, 0.16), (0.03, 0.26), (0.0, 0.26)],
                1.0,
                1.0,
            );
            b.ellipsoid("bulb", [reach, 0.2, h - 0.35], [0.05, 0.05, 0.04]);
        }
        _ => {
            b.cone(
                "frame",
                [c[0], c[1], 0.0],
                [c[0], c[1], 0.03],
                r * 0.6,
                r * 0.55,
                24,
            );
            b.cyl("metal", [c[0], c[1], 0.03], [c[0], c[1], h - 0.05], 0.011);
            drum(b, [c[0], c[1], h - 0.38], r, 0.36);
        }
    }
}

/// A lampshade: a fabric drum from `c` up `h`, a little narrower at the
/// top, with the bulb glowing inside.
fn drum(b: &mut Builder, c: P3, r: f64, h: f64) {
    b.cone("shade", c, [c[0], c[1], c[2] + h], r, r * 0.84, 28);
}

/// Plants in pots: 1 a big-leaved one (Monstera), 2 a small tree (Ficus),
/// 3 a cactus, 4 a hanging fern, 5 a snake plant (Sansevieria). Leaves
/// set by a fixed seed, so the same plant comes out every time.
pub fn potted_plant(b: &mut Builder, p: &Params) {
    let h = p.get("height", 1.0).clamp(0.15, 3.0);
    let pot = p.get("pot", 0.32).clamp(0.08, 1.0);
    let r = pot / 2.0;
    let c = [0.0, r];
    let pot_h = (pot * 0.9).min(h * 0.4);
    let style = p.style();
    // The pot, a little wider at the top, with its rim and the soil.
    let profile: Vec<(f64, f64)> = match style {
        3 => vec![
            (0.0, 0.0),
            (r * 0.8, 0.0),
            (r, pot_h * 0.8),
            (r, pot_h),
            (0.0, pot_h),
        ],
        4 => vec![
            (0.0, 0.0),
            (r * 0.5, 0.0),
            (r * 0.95, pot_h * 0.5),
            (r, pot_h),
            (0.0, pot_h),
        ],
        _ => vec![
            (0.0, 0.0),
            (r * 0.75, 0.0),
            (r * 0.95, pot_h * 0.9),
            (r, pot_h * 0.9),
            (r, pot_h),
            (0.0, pot_h),
        ],
    };
    b.lathe("pot", [c[0], c[1], 0.0], &profile, 1.0, 1.0);
    b.cone(
        "soil",
        [c[0], c[1], pot_h],
        [c[0], c[1], pot_h + 0.01],
        r * 0.92,
        r * 0.9,
        16,
    );
    let base = [c[0], c[1], pot_h];
    let mut seed = 0x9e37_79b9_u32;
    let mut rnd = || {
        seed ^= seed << 13;
        seed ^= seed >> 17;
        seed ^= seed << 5;
        f64::from(seed) / f64::from(u32::MAX)
    };
    let grow = h - pot_h;
    match style {
        2 => {
            // A trunk and a round crown of small leaves.
            let top = [base[0], base[1], base[2] + grow * 0.55];
            b.cone("stem", base, top, 0.02, 0.012, 8);
            let crown = grow * 0.28;
            for i in 0..26 {
                let a = rnd() * TAU;
                let e = rnd() * PI - PI / 2.0;
                let q = [
                    top[0] + crown * e.cos() * a.cos(),
                    top[1] + crown * e.cos() * a.sin(),
                    top[2] + crown * (1.0 + e.sin()),
                ];
                let s = crown * (0.28 + 0.12 * rnd());
                let _ = i;
                b.blob(
                    "leaves",
                    q,
                    [s, 0.0, 0.0],
                    [0.0, s * 0.8, 0.0],
                    [0.0, 0.0, s * 0.7],
                );
            }
        }
        3 => {
            // A ribbed column and two arms.
            let col = r * 0.55;
            b.lathe(
                "leaves",
                base,
                &[
                    (0.0, 0.0),
                    (col, 0.0),
                    (col, grow * 0.8),
                    (col * 0.6, grow * 0.97),
                    (0.0, grow),
                ],
                1.0,
                1.0,
            );
            for (side, z) in [(-1.0, 0.35), (1.0, 0.55)] {
                let a = [base[0] + side * col, base[1], base[2] + grow * z];
                let elbow = [a[0] + side * col * 1.4, a[1], a[2]];
                b.cone("leaves", a, elbow, col * 0.45, col * 0.45, 12);
                b.cone(
                    "leaves",
                    elbow,
                    [elbow[0], elbow[1], elbow[2] + grow * 0.25],
                    col * 0.45,
                    col * 0.3,
                    12,
                );
            }
        }
        4 => {
            // Fronds falling over the rim all round.
            for i in 0..14 {
                let a = TAU * i as f64 / 14.0 + rnd() * 0.3;
                let len = grow.max(0.3) * (0.7 + 0.3 * rnd());
                let (ca, sa) = (a.cos(), a.sin());
                // Up out of the pot, then arching over its rim.
                let rise = [
                    base[0] + ca * r * 0.5,
                    base[1] + sa * r * 0.5,
                    base[2] + len * 0.22,
                ];
                b.blob(
                    "leaves",
                    rise,
                    [ca * len * 0.2, sa * len * 0.2, len * 0.14],
                    [-sa * len * 0.07, ca * len * 0.07, 0.0],
                    [0.0, 0.0, 0.01],
                );
                let mid = [
                    base[0] + ca * (r + len * 0.3),
                    base[1] + sa * (r + len * 0.3),
                    base[2] + len * 0.2,
                ];
                b.blob(
                    "leaves",
                    mid,
                    [ca * len * 0.38, sa * len * 0.38, -len * 0.16],
                    [-sa * len * 0.09, ca * len * 0.09, 0.0],
                    [0.0, 0.0, 0.012],
                );
            }
        }
        5 => {
            // Upright sword leaves.
            for i in 0..9 {
                let a = TAU * i as f64 / 9.0 + rnd() * 0.4;
                let off = r * 0.5 * rnd();
                let tall = grow * (0.6 + 0.4 * rnd());
                let lean = 0.08 + 0.1 * rnd();
                let q = [
                    base[0] + a.cos() * off,
                    base[1] + a.sin() * off,
                    base[2] + tall / 2.0,
                ];
                b.blob(
                    "leaves",
                    q,
                    [a.cos() * 0.035, a.sin() * 0.035, 0.0],
                    [-a.sin() * 0.008, a.cos() * 0.008, 0.0],
                    [a.cos() * tall * lean, a.sin() * tall * lean, tall / 2.0],
                );
            }
        }
        _ => {
            // Stems up and out, a big split leaf on each.
            for i in 0..9 {
                let a = TAU * i as f64 / 9.0 + rnd() * 0.5;
                let up = grow * (0.45 + 0.5 * rnd());
                let out = grow * (0.2 + 0.25 * rnd());
                let (ca, sa) = (a.cos(), a.sin());
                let tip = [base[0] + ca * out, base[1] + sa * out, base[2] + up];
                b.cone("stem", base, tip, 0.008, 0.005, 6);
                let leaf = grow * (0.16 + 0.06 * rnd());
                let centre = [
                    tip[0] + ca * leaf * 0.6,
                    tip[1] + sa * leaf * 0.6,
                    tip[2] + leaf * 0.1,
                ];
                b.blob(
                    "leaves",
                    centre,
                    [ca * leaf, sa * leaf, -leaf * 0.35],
                    [-sa * leaf * 0.8, ca * leaf * 0.8, 0.0],
                    [ca * 0.01, sa * 0.01, 0.025],
                );
            }
        }
    }
}

/// Armchairs: 1 wingback, 2 a round tub chair, 3 rattan with a cushion,
/// 4 a lounge chair on slim legs.
pub fn armchair(b: &mut Builder, p: &Params) {
    let w = p.get("width", 0.8);
    let d = p.get("depth", 0.8);
    let seat = p.get("seat", 0.42);
    let h = p.get("height", 0.95);
    let x0 = -w / 2.0;
    let legs = |b: &mut Builder, foot: f64, inset: f64| {
        for (x, y) in [
            (x0 + inset, inset),
            (-x0 - inset, inset),
            (x0 + inset, d - inset),
            (-x0 - inset, d - inset),
        ] {
            let dx = if x < 0.0 { -0.02 } else { 0.02 };
            let dy = if y < d / 2.0 { -0.02 } else { 0.02 };
            b.cone(
                "frame",
                [x + dx, y + dy, 0.004],
                [x, y, foot],
                0.012,
                0.02,
                10,
            );
        }
    };
    match p.style() {
        2 => {
            // A round tub: a ring of back and arms, a cushion in it.
            let c = [0.0, d / 2.0, 0.0];
            legs(b, 0.12, 0.12);
            // A round cup up to the arms, the seat cushion in it and a
            // rounded back rising behind.
            let arms = seat + 0.2 - 0.12;
            b.lathe(
                "mat",
                [c[0], c[1], 0.12],
                &[
                    (0.0, 0.0),
                    (w * 0.47, 0.0),
                    (w * 0.5, arms * 0.55),
                    (w * 0.48, arms),
                    (w * 0.4, arms),
                    (w * 0.38, seat - 0.2),
                    (0.0, seat - 0.2),
                ],
                1.0,
                d / w,
            );
            b.pillow(
                "cushion",
                [x0 + w * 0.14, d * 0.2, seat - 0.09],
                [-x0 - w * 0.14, d * 0.86, seat + 0.02],
                0.05,
                0.025,
            );
            b.pillow(
                "mat",
                [x0 + w * 0.1, d * 0.02, seat - 0.05],
                [-x0 - w * 0.1, d * 0.2, h],
                0.08,
                0.03,
            );
        }
        3 => {
            // A rattan frame: a woven shell of rings, a seat cushion.
            legs(b, 0.1, 0.07);
            // Rattan canes round the back and the arms, and a ring under
            // the seat.
            let (rx, ry) = (w * 0.48, d * 0.48);
            let cane = |b: &mut Builder, z: f64, a0: f64, a1: f64| {
                let n = 16;
                let at = |t: f64| {
                    let a = a0 + (a1 - a0) * t;
                    [rx * a.cos(), d / 2.0 + ry * a.sin(), z]
                };
                for i in 0..n {
                    b.cyl(
                        "frame",
                        at(i as f64 / n as f64),
                        at((i + 1) as f64 / n as f64),
                        0.011,
                    );
                }
            };
            cane(b, 0.1, 0.0, TAU);
            cane(b, seat - 0.1, 0.0, TAU);
            let rows = 7;
            for i in 1..=rows {
                let z = seat - 0.1 + (h - seat + 0.08) * i as f64 / rows as f64;
                // Round the back (y < d/2) and, lower down, along the
                // sides as arms; higher up, only the back.
                let k = i as f64 / rows as f64;
                let wrap = 0.6 * (1.0 - k) - 0.2 * k;
                cane(b, z, -PI - wrap, wrap);
            }
            for i in 0..7 {
                let a = -PI + 0.35 + (PI - 0.7) * i as f64 / 6.0;
                let q = [rx * a.cos(), d / 2.0 + ry * a.sin()];
                b.cyl("frame", [q[0], q[1], 0.1], [q[0], q[1], h - 0.06], 0.009);
            }
            b.pillow(
                "cushion",
                [x0 + 0.05, 0.12, seat - 0.06],
                [-x0 - 0.05, d - 0.04, seat + 0.04],
                0.05,
                0.025,
            );
            b.pillow(
                "cushion",
                [x0 + 0.1, 0.04, seat + 0.02],
                [-x0 - 0.1, 0.2, h - 0.06],
                0.06,
                0.03,
            );
        }
        4 => {
            // Low and deep, on slim wooden legs: frame, seat and back
            // cushions.
            legs(b, 0.2, 0.06);
            b.rbox("mat", [x0, 0.0, 0.2], [-x0, d, seat - 0.12], 0.03);
            for side in [-1.0, 1.0] {
                let (a0, a1) = if side < 0.0 {
                    (x0, x0 + 0.08)
                } else {
                    (-x0 - 0.08, -x0)
                };
                b.rbox(
                    "frame",
                    [a0, 0.05, seat - 0.12],
                    [a1, d - 0.05, seat + 0.16],
                    0.03,
                );
            }
            b.pillow(
                "cushion",
                [x0 + 0.08, 0.15, seat - 0.12],
                [-x0 - 0.08, d - 0.01, seat],
                0.05,
                0.03,
            );
            b.pillow(
                "cushion",
                [x0 + 0.08, 0.02, seat - 0.02],
                [-x0 - 0.08, 0.2, h],
                0.06,
                0.04,
            );
        }
        _ => {
            // Wingback: tall back with wings, rolled arms, turned legs.
            legs(b, 0.14, 0.06);
            b.rbox("mat", [x0, 0.0, 0.14], [-x0, d, seat - 0.1], 0.03);
            b.pillow(
                "mat",
                [x0 + 0.04, 0.0, seat - 0.12],
                [-x0 - 0.04, 0.16, h],
                0.06,
                0.02,
            );
            for side in [-1.0, 1.0] {
                let (a0, a1) = if side < 0.0 {
                    (x0, x0 + 0.11)
                } else {
                    (-x0 - 0.11, -x0)
                };
                b.rbox("mat", [a0, 0.0, 0.14], [a1, d, seat + 0.18], 0.045);
                // The wing, from the arm to the top of the back.
                b.rbox("mat", [a0, 0.0, seat + 0.1], [a1, d * 0.38, h - 0.04], 0.05);
            }
            b.pillow(
                "cushion",
                [x0 + 0.11, 0.16, seat - 0.1],
                [-x0 - 0.11, d - 0.01, seat + 0.02],
                0.05,
                0.03,
            );
        }
    }
}

/// Poufs: 1 round, 2 knitted (ribbed), 3 a soft cube.
pub fn pouf(b: &mut Builder, p: &Params) {
    let w = p.get("width", 0.5);
    let h = p.get("height", 0.4);
    let r = w / 2.0;
    let c = [0.0, r, 0.0];
    match p.style() {
        2 => {
            let ribs = 16;
            let profile: Vec<(f64, f64)> = (0..=12)
                .map(|i| {
                    let t = i as f64 / 12.0;
                    let bulge = (PI * t).sin();
                    (r * (0.82 + 0.18 * bulge), h * t)
                })
                .collect();
            let mut prof = vec![(0.0, 0.0)];
            prof.extend(profile);
            prof.push((0.0, h));
            b.lathe("mat", c, &prof, 1.0, 1.0);
            // The ribs of the knit, as soft ridges.
            for i in 0..ribs {
                let a = TAU * i as f64 / ribs as f64;
                let q = [
                    c[0] + a.cos() * r * 0.98,
                    c[1] + a.sin() * r * 0.98,
                    h / 2.0,
                ];
                b.blob(
                    "mat",
                    q,
                    [a.cos() * 0.025, a.sin() * 0.025, 0.0],
                    [-a.sin() * 0.03, a.cos() * 0.03, 0.0],
                    [0.0, 0.0, h * 0.46],
                );
            }
        }
        3 => {
            b.pillow("mat", [-r, 0.0, 0.03], [r, w, h], 0.06, 0.03);
        }
        _ => {
            b.lathe(
                "mat",
                c,
                &[
                    (0.0, 0.0),
                    (r * 0.9, 0.0),
                    (r, h * 0.15),
                    (r, h * 0.8),
                    (r * 0.9, h * 0.98),
                    (r * 0.6, h),
                    (0.0, h),
                ],
                1.0,
                1.0,
            );
        }
    }
}

/// Side tables: 1 round on a central foot, 2 three legs and a tray, 3 a
/// pair of nesting tables.
pub fn side_table(b: &mut Builder, p: &Params) {
    let w = p.get("width", 0.5);
    let h = p.get("height", 0.55);
    let r = w / 2.0;
    let c = [0.0, r];
    match p.style() {
        2 => {
            b.lathe(
                "mat",
                [c[0], c[1], h - 0.04],
                &[
                    (0.0, 0.0),
                    (r, 0.0),
                    (r, 0.04),
                    (r - 0.012, 0.045),
                    (0.0, 0.03),
                ],
                1.0,
                1.0,
            );
            for i in 0..3 {
                let a = TAU * i as f64 / 3.0 + PI / 2.0;
                let top = [c[0] + a.cos() * r * 0.7, c[1] + a.sin() * r * 0.7, h - 0.04];
                let foot = [c[0] + a.cos() * r * 0.95, c[1] + a.sin() * r * 0.95, 0.004];
                b.cone("frame", foot, top, 0.009, 0.014, 10);
            }
        }
        3 => {
            let d = p.get("depth", w);
            for (k, (sx, sh)) in [(1.0, 1.0), (0.8, 0.85)].iter().enumerate() {
                let ww = w * sx;
                let hh = h * sh;
                let xo = if k == 0 { 0.0 } else { w * 0.15 };
                let x0 = -ww / 2.0 + xo;
                let x1 = ww / 2.0 + xo;
                let dd = d * sx;
                b.rbox("mat", [x0, 0.0, hh - 0.025], [x1, dd, hh], 0.005);
                for (x, y) in [
                    (x0 + 0.02, 0.02),
                    (x1 - 0.02, 0.02),
                    (x0 + 0.02, dd - 0.02),
                    (x1 - 0.02, dd - 0.02),
                ] {
                    b.cyl("frame", [x, y, 0.0], [x, y, hh - 0.025], 0.009);
                }
            }
        }
        _ => {
            b.cone(
                "frame",
                [c[0], c[1], 0.0],
                [c[0], c[1], 0.02],
                r * 0.55,
                r * 0.5,
                24,
            );
            b.cone(
                "frame",
                [c[0], c[1], 0.02],
                [c[0], c[1], h - 0.03],
                0.025,
                0.02,
                12,
            );
            b.lathe(
                "mat",
                [c[0], c[1], h - 0.03],
                &[(0.0, 0.0), (r, 0.0), (r, 0.03), (0.0, 0.03)],
                1.0,
                1.0,
            );
        }
    }
}

/// Sideboards: 1 Nordic on tapered legs, with doors and drawers; 2 slatted
/// doors on a plinth.
pub fn sideboard(b: &mut Builder, p: &Params) {
    let w = p.get("width", 1.6);
    let d = p.get("depth", 0.45);
    let h = p.get("height", 0.75);
    let x0 = -w / 2.0;
    let kind = Handle::of(p, Handle::Knob);
    let doors = ((w / 0.45).round() as usize).max(2);
    match p.style() {
        2 => {
            b.rbox(
                "frame",
                [x0 + 0.03, 0.03, 0.0],
                [-x0 - 0.03, d - 0.05, 0.08],
                0.0,
            );
            b.rbox("mat", [x0, 0.0, 0.08], [-x0, d - 0.02, h], 0.006);
            let dw = w / doors as f64;
            for i in 0..doors {
                let a = x0 + i as f64 * dw;
                let slats = ((dw / 0.035).round() as usize).max(4);
                for k in 0..slats {
                    let sx = a + 0.01 + k as f64 * (dw - 0.02) / slats as f64;
                    b.rbox(
                        "mat",
                        [sx, d - 0.02, 0.11],
                        [sx + (dw - 0.02) / slats as f64 * 0.7, d + 0.004, h - 0.04],
                        0.004,
                    );
                }
            }
        }
        _ => {
            let foot = 0.18;
            for (x, y) in [
                (x0 + 0.06, 0.06),
                (-x0 - 0.06, 0.06),
                (x0 + 0.06, d - 0.06),
                (-x0 - 0.06, d - 0.06),
            ] {
                let dx = if x < 0.0 { -0.02 } else { 0.02 };
                b.cone("frame", [x + dx, y, 0.004], [x, y, foot], 0.012, 0.02, 10);
            }
            b.rbox("mat", [x0, 0.0, foot], [-x0, d - 0.02, h], 0.008);
            // Drawers in the middle, doors either side.
            let dw = w / doors as f64;
            for i in 0..doors {
                let a = x0 + i as f64 * dw;
                let middle = doors > 2 && i == doors / 2;
                if middle {
                    fronts::drawers(b, a, a + dw, foot + 0.02, h - 0.02, d - 0.02, 3, kind);
                } else {
                    fronts::door(
                        b,
                        a,
                        a + dw,
                        foot + 0.02,
                        h - 0.02,
                        d - 0.02,
                        kind,
                        i < doors / 2,
                        (foot + h) / 2.0,
                    );
                }
            }
        }
    }
}

/// Curtains: a rod and soft folds down to the floor, drawn to both sides
/// (`open` 1) or closed (`open` 0). 1 long, 2 sheer (see-through), 3 to
/// the sill (`height` from the rod down).
pub fn curtain(b: &mut Builder, p: &Params) {
    let w = p.get("width", 1.6);
    let h = p.get("height", 2.5).clamp(0.4, 3.5);
    let open = p.get("open", 0.6).clamp(0.0, 1.0);
    let drop = if p.style() == 3 {
        (h * 0.45).max(0.6)
    } else {
        h - 0.02
    };
    let x0 = -w / 2.0;
    let rod = h;
    b.cyl(
        "metal",
        [x0 - 0.08, 0.08, rod],
        [-x0 + 0.08, 0.08, rod],
        0.012,
    );
    for x in [x0 - 0.08, -x0 + 0.08] {
        b.ellipsoid("metal", [x, 0.08, rod], [0.02, 0.02, 0.02]);
    }
    // Each half gathered towards its side as it opens.
    let half = w / 2.0;
    let gathered = half * (1.0 - 0.75 * open);
    let role = if p.style() == 2 { "sheer" } else { "mat" };
    for side in [-1.0, 1.0] {
        let outer = side * (half + 0.06);
        let inner = outer - side * gathered;
        let folds = ((gathered / 0.09).round() as usize).max(3);
        let step = gathered / folds as f64;
        let _ = inner;
        for k in 0..folds {
            // Rounded columns side by side, in and out: the folds.
            let a = outer - side * (k as f64 + 0.5) * step;
            let deep = if k % 2 == 0 { 0.05 } else { 0.035 };
            b.rbox(
                role,
                [a - step * 0.55, 0.06, rod - drop],
                [a + step * 0.55, 0.06 + deep, rod - 0.03],
                deep / 2.0 - 0.001,
            );
        }
    }
}

/// Things on tables and shelves: 1 a vase with branches, 2 a bowl of
/// fruit, 3 a stack of books with a candle, 4 ceramics (jugs and cups),
/// 5 a basket with a throw.
pub fn decor(b: &mut Builder, p: &Params) {
    let w = p.get("width", 0.3);
    let c = [0.0, w / 2.0];
    match p.style() {
        2 => {
            let r = w * 0.45;
            b.lathe(
                "mat",
                [c[0], c[1], 0.0],
                &[
                    (0.0, 0.0),
                    (r * 0.45, 0.0),
                    (r * 0.9, r * 0.35),
                    (r, r * 0.5),
                    (r * 0.96, r * 0.52),
                    (0.0, r * 0.2),
                ],
                1.0,
                1.0,
            );
            for (dx, dy, k) in [
                (-0.3, -0.1, 0),
                (0.25, 0.05, 1),
                (0.0, 0.3, 2),
                (0.05, -0.3, 0),
                (-0.15, 0.2, 1),
            ] {
                let role = ["fruit", "fruit2", "fruit"][k];
                let s = r * 0.32;
                b.ellipsoid(
                    role,
                    [c[0] + dx * r, c[1] + dy * r, r * 0.42 + s * 0.6],
                    [s, s, s * 0.9],
                );
            }
        }
        3 => {
            let mut z = 0.0;
            for (i, t) in [0.035, 0.025, 0.04, 0.03].iter().enumerate() {
                let role = if i % 2 == 0 { "mat" } else { "books2" };
                let ww = w * (0.95 - 0.06 * i as f64);
                let a = 0.05 * (i as f64 - 1.5);
                let (ca, sa) = (a.cos(), a.sin());
                let poly: Vec<[f64; 2]> = [(-1.0, -0.7), (1.0, -0.7), (1.0, 0.7), (-1.0, 0.7)]
                    .iter()
                    .map(|(x, y)| {
                        let (x, y) = (x * ww / 2.0, y * ww * 0.35);
                        [c[0] + x * ca - y * sa, c[1] + x * sa + y * ca]
                    })
                    .collect();
                b.slab(role, &poly, z, z + t);
                z += t;
            }
            b.cyl(
                "candle",
                [c[0] + w * 0.1, c[1], z],
                [c[0] + w * 0.1, c[1], z + 0.09],
                0.03,
            );
            b.ellipsoid(
                "flame",
                [c[0] + w * 0.1, c[1], z + 0.105],
                [0.007, 0.007, 0.016],
            );
        }
        4 => {
            let r = w * 0.18;
            b.lathe(
                "mat",
                [c[0] - w * 0.2, c[1], 0.0],
                &[
                    (0.0, 0.0),
                    (r * 0.8, 0.0),
                    (r, r * 1.2),
                    (r * 0.55, r * 2.2),
                    (r * 0.6, r * 2.6),
                    (0.0, r * 2.6),
                ],
                1.0,
                1.0,
            );
            for (dx, dy) in [(0.18, -0.1), (0.25, 0.18)] {
                b.lathe(
                    "glaze2",
                    [c[0] + dx * w, c[1] + dy * w, 0.0],
                    &[
                        (0.0, 0.0),
                        (r * 0.55, 0.0),
                        (r * 0.75, r * 0.9),
                        (0.0, r * 0.9),
                    ],
                    1.0,
                    1.0,
                );
            }
        }
        5 => {
            let r = w * 0.45;
            b.lathe(
                "frame",
                [c[0], c[1], 0.0],
                &[(0.0, 0.0), (r * 0.85, 0.0), (r, r * 1.1), (0.0, r * 1.1)],
                1.0,
                1.0,
            );
            b.blob(
                "mat",
                [c[0], c[1], r * 1.15],
                [r * 0.8, 0.0, 0.1 * r],
                [0.0, r * 0.7, 0.0],
                [-0.05 * r, 0.0, r * 0.3],
            );
        }
        _ => {
            let r = w * 0.3;
            let h = p.get("height", 0.5).max(0.15);
            b.lathe(
                "mat",
                [c[0], c[1], 0.0],
                &[
                    (0.0, 0.0),
                    (r * 0.6, 0.0),
                    (r, h * 0.25),
                    (r * 0.8, h * 0.45),
                    (r * 0.35, h * 0.55),
                    (r * 0.42, h * 0.6),
                    (0.0, h * 0.6),
                ],
                1.0,
                1.0,
            );
            // Branches out of the neck.
            for (i, a) in [0.4_f64, 2.1, 3.9, 5.2].iter().enumerate() {
                let top = [
                    c[0] + a.cos() * r * 1.2,
                    c[1] + a.sin() * r * 1.2,
                    h * (0.95 + 0.05 * i as f64),
                ];
                b.cone("stem", [c[0], c[1], h * 0.55], top, 0.004, 0.002, 5);
                b.ellipsoid("leaves", top, [r * 0.35, r * 0.35, r * 0.25]);
            }
        }
    }
}
