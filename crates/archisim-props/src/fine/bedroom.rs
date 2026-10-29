//! The workshop's bedrooms, one for each collection (paquete 3): the
//! library's beds, bedside tables, wardrobes and chests of drawers, with
//! the same sizes, made the way each collection makes them, and two new
//! pieces: the bench at the foot of the bed and the dressing table.
//!
//! Axes as every piece's: the back on the wall at `y = 0`, the front
//! looking to `+y`, centred on `x = 0`. A bed's head is on the wall.
//!
//! Roles: `frame` the wood (or the steel) of the frame and the carcass,
//! `front` doors and drawers, `top` a top of another wood, `stone` a
//! marble top, `plinth` what it stands on, `metal` pulls, legs and tubes,
//! `glass`, `weave` cane or rush, `dark` shadow gaps, `cushion` the
//! upholstery, `linen` sheets and pillows, `duvet`, `spread` the throw or
//! bedspread at the foot, `pillow` the scatter cushions, `shade` a lamp's
//! shade, `ceramic` its base, `bulb` a bare bulb, `mirror_glass` a
//! mirror's glass.

use super::bedding::{bed_pillow, bolster, standing_cushion, Cover};
use super::shape::{bar, curve, sweep, turned, Cap};
use super::sofas::compass_leg;
use super::soft::{tube, Soft};
use super::storage::{self, corners, front, pull, tapered_leg, Panel, Pull, OUT, T};
use crate::build::{cross, norm, sub, Builder, P3};
use crate::Params;
use std::f64::consts::{FRAC_PI_2, PI, TAU};

pub use super::kitchen::COLLECTIONS;

/// The collection of a model chosen among the collections, listed after
/// the family's `first` own models (0 to 7).
pub(super) fn look(p: &Params, first: i32) -> Option<usize> {
    let s = p.style() - first;
    (1..=8).contains(&s).then(|| (s - 1) as usize)
}

// ---------------------------------------------------------- helpers

/// A quadrilateral facing `out` (its corners in either order).
pub(super) fn oquad(b: &mut Builder, role: &'static str, q: [P3; 4], out: P3) {
    let n = cross(sub(q[1], q[0]), sub(q[3], q[0]));
    let ids: Vec<u32> = q.iter().map(|p| b.point(*p)).collect();
    if n[0] * out[0] + n[1] * out[1] + n[2] * out[2] >= 0.0 {
        b.quad(role, ids[0], ids[1], ids[2], ids[3]);
    } else {
        b.quad(role, ids[3], ids[2], ids[1], ids[0]);
    }
}

/// The top of an arch from `x0` to `x1`: half an ellipse springing at
/// `zs`, its crown at `zt`.
pub(super) fn arch_z(x: f64, x0: f64, x1: f64, zs: f64, zt: f64) -> f64 {
    let u = ((x - x0) / (x1 - x0) * 2.0 - 1.0).clamp(-1.0, 1.0);
    zs + (zt - zs) * (1.0 - u * u).max(0.0).sqrt()
}

/// A board from `z0` up to an arched top (see [`arch_z`]), from `y0` to
/// `y1` thick.
#[allow(clippy::too_many_arguments)]
pub(super) fn arched_board(
    b: &mut Builder,
    role: &'static str,
    x0: f64,
    x1: f64,
    z0: f64,
    zs: f64,
    zt: f64,
    y0: f64,
    y1: f64,
) {
    let n = 20;
    let xs: Vec<f64> = (0..=n)
        .map(|k| x0 + (x1 - x0) * (1.0 - (PI * k as f64 / n as f64).cos()) / 2.0)
        .collect();
    let zt_at = |x: f64| arch_z(x, x0, x1, zs, zt);
    for k in 0..n {
        let (a, c) = (xs[k], xs[k + 1]);
        let (za, zc) = (zt_at(a), zt_at(c));
        oquad(
            b,
            role,
            [[a, y1, z0], [c, y1, z0], [c, y1, zc], [a, y1, za]],
            [0.0, 1.0, 0.0],
        );
        oquad(
            b,
            role,
            [[a, y0, z0], [c, y0, z0], [c, y0, zc], [a, y0, za]],
            [0.0, -1.0, 0.0],
        );
        let up = norm([-(zc - za), 0.0, c - a]);
        oquad(
            b,
            role,
            [[a, y0, za], [c, y0, zc], [c, y1, zc], [a, y1, za]],
            up,
        );
        oquad(
            b,
            role,
            [[a, y0, z0], [c, y0, z0], [c, y1, z0], [a, y1, z0]],
            [0.0, 0.0, -1.0],
        );
    }
    for (x, s) in [(x0, -1.0), (x1, 1.0)] {
        let z = zt_at(x);
        oquad(
            b,
            role,
            [[x, y0, z0], [x, y1, z0], [x, y1, z], [x, y0, z]],
            [s, 0.0, 0.0],
        );
    }
}

/// A band round an arch (see [`arch_z`]) `depth` deep, from `y0` to `y1`:
/// the frame of an arched headboard or mirror.
#[allow(clippy::too_many_arguments)]
pub(super) fn arch_band(
    b: &mut Builder,
    role: &'static str,
    x0: f64,
    x1: f64,
    zs: f64,
    zt: f64,
    depth: f64,
    y0: f64,
    y1: f64,
) {
    let (cx, a, h) = ((x0 + x1) / 2.0, (x1 - x0) / 2.0, zt - zs);
    let n = 24;
    let at = |k: usize, inner: bool| -> [f64; 2] {
        let t = PI * k as f64 / n as f64;
        let (ra, rh) = if inner {
            ((a - depth).max(0.01), (h - depth).max(0.01))
        } else {
            (a, h)
        };
        [cx + ra * t.cos(), zs + rh * t.sin()]
    };
    for k in 0..n {
        let (o0, o1, i0, i1) = (at(k, false), at(k + 1, false), at(k, true), at(k + 1, true));
        let p = |q: [f64; 2], y: f64| [q[0], y, q[1]];
        oquad(
            b,
            role,
            [p(o0, y1), p(o1, y1), p(i1, y1), p(i0, y1)],
            [0.0, 1.0, 0.0],
        );
        oquad(
            b,
            role,
            [p(o0, y0), p(o1, y0), p(i1, y0), p(i0, y0)],
            [0.0, -1.0, 0.0],
        );
        let mid = |q: [f64; 2], r: [f64; 2]| [(q[0] + r[0]) / 2.0 - cx, (q[1] + r[1]) / 2.0 - zs];
        let m = mid(o0, o1);
        oquad(
            b,
            role,
            [p(o0, y0), p(o1, y0), p(o1, y1), p(o0, y1)],
            [m[0], 0.0, m[1]],
        );
        let m = mid(i0, i1);
        oquad(
            b,
            role,
            [p(i0, y0), p(i1, y0), p(i1, y1), p(i0, y1)],
            [-m[0], 0.0, -m[1]],
        );
    }
    for (k, s) in [(0, -1.0), (n, -1.0)] {
        let (o, i) = (at(k, false), at(k, true));
        let p = |q: [f64; 2], y: f64| [q[0], y, q[1]];
        oquad(
            b,
            role,
            [p(o, y0), p(i, y0), p(i, y1), p(o, y1)],
            [0.0, 0.0, s],
        );
    }
}

/// A turned post from `z0` to `z1` at `at`: a pine bed's or a table's.
pub(super) fn turned_post(
    b: &mut Builder,
    role: &'static str,
    at: [f64; 2],
    z0: f64,
    z1: f64,
    r: f64,
) {
    let h = z1 - z0;
    let prof = [
        (0.0, 0.0),
        (r * 0.8, 0.0),
        (r * 0.85, h * 0.05),
        (r * 1.05, h * 0.09),
        (r * 0.75, h * 0.13),
        (r * 0.62, h * 0.2),
        (r * 0.8, h * 0.45),
        (r * 0.62, h * 0.72),
        (r * 0.95, h * 0.82),
        (r * 1.05, h * 0.9),
        (r, h),
        (0.0, h),
    ];
    turned(b, role, [at[0], at[1], z0], &prof, 16, 0.0);
}

/// A fluted, tapering leg of the classic pieces, standing on a toupie
/// foot, its top at `top`.
pub(super) fn louis_leg(b: &mut Builder, role: &'static str, at: [f64; 2], top: f64, r: f64) {
    let foot = (top * 0.18).min(0.07);
    turned(
        b,
        role,
        [at[0], at[1], 0.0],
        &[
            (0.0, 0.0),
            (r * 0.55, 0.0),
            (r * 0.9, foot * 0.55),
            (r * 0.7, foot),
            (r * 0.62, foot + 0.005),
            (r * 0.95, top - 0.06),
            (r * 1.15, top - 0.05),
            (r * 1.15, top),
            (0.0, top),
        ],
        16,
        0.0,
    );
}

/// Four square legs under the corners of `x0`…`x1` by `0`…`d`, `s` wide.
pub(super) fn square_legs(
    b: &mut Builder,
    role: &'static str,
    x0: f64,
    x1: f64,
    d: f64,
    top: f64,
    s: f64,
) {
    for q in corners(x0, x1, d, s / 2.0) {
        b.rbox(
            role,
            [q[0] - s / 2.0, q[1] - s / 2.0, 0.0],
            [q[0] + s / 2.0, q[1] + s / 2.0, top],
            0.004,
        );
    }
}

/// A box open at the front: sides, top, bottom and back, `t` thick.
#[allow(clippy::too_many_arguments)]
fn case(b: &mut Builder, role: &'static str, x0: f64, x1: f64, d: f64, z0: f64, z1: f64, t: f64) {
    b.rbox(role, [x0, 0.0, z0], [x0 + t, d, z1], 0.0);
    b.rbox(role, [x1 - t, 0.0, z0], [x1, d, z1], 0.0);
    b.rbox(role, [x0 + t, 0.0, z1 - t], [x1 - t, d, z1], 0.0);
    b.rbox(role, [x0 + t, 0.0, z0], [x1 - t, d, z0 + t], 0.0);
    b.rbox(role, [x0 + t, 0.0, z0 + t], [x1 - t, 0.012, z1 - t], 0.0);
}

/// Drawers stacked from `z0` to `z1` in a front at `y`, with one pull in
/// the middle or, wider than 60 cm, two.
#[allow(clippy::too_many_arguments)]
fn drawer_stack(
    b: &mut Builder,
    panel: Panel,
    x0: f64,
    x1: f64,
    z0: f64,
    z1: f64,
    y: f64,
    n: usize,
    kind: Pull,
) {
    let n = n.max(1);
    let gap = 0.003;
    let step = (z1 - z0) / n as f64;
    let wide = x1 - x0 > 0.6;
    for i in 0..n {
        let (a, e) = (
            z0 + step * i as f64 + gap / 2.0,
            z0 + step * (i + 1) as f64 - gap / 2.0,
        );
        front(b, panel, "front", x0 + gap / 2.0, x1 - gap / 2.0, a, e, y);
        let pz = match kind {
            Pull::Slot => e - 0.001,
            _ => (a + e) / 2.0,
        };
        let xs: Vec<f64> = if wide && kind != Pull::Slot && kind != Pull::None {
            let q = (x1 - x0) * 0.25;
            vec![x0 + q, x1 - q]
        } else {
            vec![(x0 + x1) / 2.0]
        };
        for x in xs {
            pull(b, kind, x, pz, y + T);
        }
    }
}

/// Upright planks from `x0` to `x1` and `z0` to `z1` on a face at `y`,
/// their joints dark (lighter than a door of planks).
fn planks(b: &mut Builder, role: &'static str, x0: f64, x1: f64, z0: f64, z1: f64, y: f64) {
    let n = (((x1 - x0) / 0.12).round() as usize).max(2);
    let step = (x1 - x0) / n as f64;
    for i in 0..n {
        let a = x0 + step * i as f64;
        b.rbox(role, [a + 0.002, y, z0], [a + step - 0.002, y + T, z1], 0.0);
    }
    b.rbox(
        "dark",
        [x0 + 0.004, y + 0.004, z0 + 0.004],
        [x1 - 0.004, y + T * 0.5, z1 - 0.004],
        0.0,
    );
}

/// Handleless drawers: each front with a dark groove over it (a gola).
fn gola_stack(b: &mut Builder, x0: f64, x1: f64, z0: f64, z1: f64, y: f64, n: usize) {
    let n = n.max(1);
    let step = (z1 - z0) / n as f64;
    for i in 0..n {
        let (a, e) = (
            z0 + step * i as f64 + 0.002,
            z0 + step * (i + 1) as f64 - 0.028,
        );
        front(b, Panel::Flat, "front", x0 + 0.002, x1 - 0.002, a, e, y);
        b.rbox(
            "dark",
            [x0 + 0.002, y - 0.02, e],
            [x1 - 0.002, y - 0.016, e + 0.026],
            0.0,
        );
    }
}

// ---------------------------------------------------------- the bed

/// How the bed is made up.
pub(super) struct Making {
    /// The mattress: from `x0` to `x1`, `y0` to `y1`, its foot at
    /// `base` and its top at `top`.
    pub(super) x0: f64,
    pub(super) x1: f64,
    pub(super) y0: f64,
    pub(super) y1: f64,
    pub(super) base: f64,
    pub(super) top: f64,
    /// The lowest the bed clothes may hang.
    pub(super) floor: f64,
    /// The duvet's foot hangs (false: tucked against a footboard).
    pub(super) foot: bool,
    /// Its sides hang (false: tucked inside the rails of a bunk or a
    /// cot).
    pub(super) sides: bool,
}

/// The bed clothes of collection `ix`: the mattress in its sheet, the
/// duvet with the sheet turned down over it, the pillows, a throw across
/// the foot (`spread`) and scatter cushions (`cushions`).
pub(super) fn make_bed(b: &mut Builder, p: &Params, ix: usize, m: &Making) {
    let w = m.x1 - m.x0;
    let mut mattress = Soft::new([m.x0, m.y0, m.base], [m.x1, m.y1, m.top], 0.05);
    mattress.crown = 0.006;
    mattress.side = 0.008;
    mattress.wobble = 0.0012;
    mattress.seed = 71;
    mattress.step = 0.18;
    mattress.add(b, "linen");

    // The duvet: from below the pillows to the foot, over the sides.
    let (puff, folds, thick) = match ix {
        3 => (0.014, 0.01, 0.035),
        5 => (0.02, 0.016, 0.04),
        7 => (0.03, 0.014, 0.05),
        _ => (0.026, 0.018, 0.05),
    };
    let ys = m.y0 + 0.55;
    let side = if m.sides {
        (m.top - m.floor) * 0.9 + 0.08
    } else {
        0.1
    };
    // Where it is tucked (inside rails or against a footboard) its turn
    // stays over the mattress, so nothing goes through the bars.
    let round = 0.06;
    let tx = if m.sides { 0.0 } else { round + 0.01 };
    let ty = if m.foot { 0.0 } else { round + 0.01 };
    let mut duvet = Cover::new(
        [m.x0 + tx, m.x1 - tx],
        [ys, m.y1 - ty],
        m.top + thick,
        thick,
    );
    duvet.round = round;
    duvet.falls = [side, side, 0.07, if m.foot { side } else { 0.1 }];
    duvet.puff = puff;
    duvet.folds = folds;
    duvet.flare = 0.022;
    duvet.floor = m.floor;
    duvet.seed = 3 + ix as u32;
    if ix == 5 {
        duvet.quilt = 0.16;
    }
    duvet.add(b, "duvet");

    // The top sheet turned down over the duvet's head.
    let mut sheet = Cover::new(
        [m.x0 - 0.002, m.x1 + 0.002],
        [ys - 0.002, ys + 0.24],
        duvet.z + puff + 0.008,
        0.008,
    );
    sheet.round = duvet.round + 0.012;
    sheet.falls = [
        sheet.round * FRAC_PI_2 + 0.04,
        sheet.round * FRAC_PI_2 + 0.04,
        1.0,
        0.0,
    ];
    sheet.floor = m.top + 0.004;
    sheet.seed = duvet.seed;
    sheet.add(b, "linen");

    // Pillows against the headboard.
    let n = if w > 1.2 { 2 } else { 1 };
    let pw = ((w - 0.1) / n as f64 - 0.04).min(0.74);
    for i in 0..n {
        let x = m.x0 + w * (i as f64 + 0.5) / n as f64;
        if ix == 7 && w > 1.2 {
            // Shams standing behind the pillows, as a classic bed has.
            standing_cushion(
                b,
                "linen",
                [x, m.y0 + 0.1, m.top + 0.3],
                pw.min(0.65),
                0.3,
                0.0,
                90 + i as u32,
            );
        }
        let start = b.mark();
        bed_pillow(
            b,
            "linen",
            [x, m.y0 + 0.3, m.top + 0.07],
            pw,
            0.46,
            0.42,
            80 + i as u32,
        );
        // Resting on the mattress, a little sunk in it.
        b.settle(start, m.top - 0.004);
    }
    if ix == 7 && w > 1.2 {
        bolster(b, "pillow", [0.0, m.y0 + 0.62, m.top + 0.1], w - 0.3, 0.085);
    }

    // The throw across the foot.
    if p.flag("spread", false) {
        let mut spread = Cover::new(
            [m.x0, m.x1],
            [m.y1 - 0.58, m.y1],
            duvet.z + puff * 0.9 + 0.02,
            0.018,
        );
        spread.round = duvet.round + thick + 0.012;
        spread.falls = [
            side * 0.8,
            side * 0.8,
            0.0,
            if m.foot { side * 0.8 } else { 0.12 },
        ];
        spread.puff = 0.004;
        spread.folds = folds;
        spread.flare = duvet.flare + 0.012;
        spread.floor = m.floor + 0.06;
        spread.seed = duvet.seed;
        spread.add(b, "spread");
    }

    // Scatter cushions leaning on the pillows.
    let k = p.get("cushions", 0.0).round().clamp(0.0, 4.0) as usize;
    if k > 0 {
        let size = (0.42_f64).min((w - 0.2) / k as f64);
        for i in 0..k {
            let x = m.x0 + w * (i as f64 + 0.5) / k as f64;
            let yaw = [0.08, -0.05, 0.06, -0.09][i];
            let start = b.mark();
            standing_cushion(
                b,
                "pillow",
                [x, m.y0 + 0.5, m.top + 0.08 + size * 0.42],
                size,
                0.42,
                yaw,
                60 + i as u32,
            );
            b.settle(start, m.top - 0.004);
        }
    }
}

/// Beds of the collections (after the family's 5 own models): the frame,
/// the headboard and the bed clothes.
pub fn bed(b: &mut Builder, p: &Params) -> bool {
    let Some(ix) = look(p, 5) else {
        return false;
    };
    let w = p.get("width", 1.5).clamp(0.8, 2.2);
    let l = p.get("depth", 2.0).clamp(1.8, 2.4);
    let top = p.get("height", 0.5).clamp(0.3, 0.8);
    let head = p.get("head", 1.0).clamp(0.5, 1.6).max(top + 0.15);
    let m = bed_frame(b, ix, w, l, top, head, 0.22);
    make_bed(b, p, ix, &m);
    true
}

/// The frame and headboard of collection `ix`'s bed for a mattress `w`
/// by about `l`, its top at `top` (`mattress` thick), the headboard
/// `head` high; how the bed is then made up.
pub(super) fn bed_frame(
    b: &mut Builder,
    ix: usize,
    w: f64,
    l: f64,
    top: f64,
    head: f64,
    mattress: f64,
) -> Making {
    let (x0, x1) = (-w / 2.0, w / 2.0);
    let base = top - mattress;
    let mut m = Making {
        x0,
        x1,
        y0: 0.1,
        y1: l - 0.02,
        base,
        top,
        floor: 0.05,
        foot: true,
        sides: true,
    };
    match ix {
        // Nordic: an upholstered headboard with soft edges and piping,
        // an upholstered base on splayed oak legs.
        0 => {
            let mut hb = Soft::new([x0 - 0.06, 0.0, 0.12], [x1 + 0.06, 0.1, head], 0.05);
            hb.crown = 0.014;
            hb.side = 0.008;
            hb.wobble = 0.0012;
            hb.welt = 0.005;
            hb.seed = 5;
            hb.step = 0.12;
            hb.add(b, "cushion");
            let mut bs = Soft::new([x0 - 0.01, 0.1, 0.14], [x1 + 0.01, l, base], 0.03);
            bs.side = 0.005;
            bs.wobble = 0.001;
            bs.seed = 6;
            bs.step = 0.25;
            bs.add(b, "cushion");
            for (q, o) in corners(x0 + 0.02, x1 - 0.02, l, 0.08).iter().zip(OUT) {
                tapered_leg(b, "frame", *q, 0.145, o, 0.02, 0.02);
            }
        }
        // Mediterranean: whitewashed wood, a headboard of cane in an
        // arched frame on two posts.
        1 => {
            let rail = base - 0.18;
            for x in [x0 - 0.04, x1] {
                b.rbox("frame", [x, 0.06, rail], [x + 0.04, l, base], 0.0);
            }
            b.rbox("frame", [x0, l - 0.04, rail], [x1, l, base], 0.0);
            square_legs(b, "frame", x0 - 0.04, x1 + 0.04, l, rail, 0.06);
            let (px0, px1) = (x0 - 0.1, x1 + 0.1);
            let zs = head - (w * 0.18).clamp(0.18, 0.3);
            for x in [px0, px1 - 0.06] {
                b.rbox("frame", [x, 0.0, 0.0], [x + 0.06, 0.06, zs], 0.0);
            }
            arch_band(b, "frame", px0, px1, zs, head, 0.06, 0.0, 0.06);
            b.rbox(
                "frame",
                [px0 + 0.06, 0.005, base - 0.02],
                [px1 - 0.06, 0.055, base + 0.05],
                0.0,
            );
            arched_board(
                b,
                "weave",
                px0 + 0.055,
                px1 - 0.055,
                base + 0.045,
                zs,
                head - 0.055,
                0.022,
                0.036,
            );
            m.y0 = 0.07;
            m.floor = 0.04;
        }
        // Fifties: teak, a wide headboard with grooves, the base on
        // compass legs with brass shoes.
        2 => {
            let rail = base - 0.16;
            b.rbox(
                "frame",
                [x0 - 0.025, 0.04, rail],
                [x1 + 0.025, l, base],
                0.012,
            );
            for (q, o) in corners(x0 + 0.04, x1 - 0.04, l, 0.1).iter().zip(OUT) {
                compass_leg(b, *q, rail + 0.005, o, 0.035);
            }
            b.rbox(
                "frame",
                [x0 - 0.12, 0.0, rail],
                [x1 + 0.12, 0.045, head],
                0.02,
            );
            for k in 1..4 {
                let z = rail + (head - rail) * k as f64 / 4.0;
                b.rbox(
                    "dark",
                    [x0 - 0.1, 0.044, z - 0.004],
                    [x1 + 0.1, 0.047, z + 0.004],
                    0.0,
                );
            }
            // Its two ends turn forward as small wings.
            for (xa, xb) in [(x0 - 0.12, x0 - 0.1), (x1 + 0.1, x1 + 0.12)] {
                b.rbox(
                    "frame",
                    [xa, 0.0, rail + 0.05],
                    [xb, 0.16, head - 0.05],
                    0.008,
                );
            }
            m.y0 = 0.05;
            m.y1 = l - 0.015;
        }
        // Japandi: a low oak platform wider than the mattress on a dark
        // recessed plinth, a long low headboard of slats.
        3 => {
            let p_top = 0.17;
            b.rbox(
                "dark",
                [x0 + 0.04, 0.14, 0.0],
                [x1 - 0.04, l - 0.12, 0.05],
                0.0,
            );
            b.rbox(
                "frame",
                [x0 - 0.14, 0.03, 0.05],
                [x1 + 0.14, l, p_top],
                0.008,
            );
            let hh = head.min(0.9);
            let (hx0, hx1) = (x0 - 0.14, x1 + 0.14);
            b.rbox("dark", [hx0, 0.0, p_top], [hx1, 0.012, hh], 0.0);
            let n = (((hx1 - hx0) / 0.055).round() as usize).max(8);
            let step = (hx1 - hx0) / n as f64;
            for i in 0..n {
                let a = hx0 + step * i as f64;
                b.rbox(
                    "frame",
                    [a + 0.008, 0.012, p_top],
                    [a + step - 0.008, 0.035, hh],
                    0.0,
                );
            }
            b.rbox("frame", [hx0, 0.0, hh], [hx1, 0.04, hh + 0.025], 0.0);
            m.base = p_top;
            m.top = p_top + mattress;
            m.y0 = 0.05;
            m.y1 = l - 0.08;
            m.floor = p_top + 0.012;
        }
        // Contemporary: a tall headboard of upholstered channels wider
        // than the bed, the base upholstered down to a shadow gap.
        4 => {
            let hw = w + 0.5;
            let n = ((hw / 0.2).round() as usize).max(5);
            let cw = hw / n as f64;
            for i in 0..n {
                let a = -hw / 2.0 + cw * i as f64;
                let mut c = Soft::new([a + 0.002, 0.0, 0.05], [a + cw - 0.002, 0.11, head], 0.045);
                c.crown = 0.022;
                c.side = 0.01;
                c.wobble = 0.001;
                c.seed = 20 + i as u32;
                c.step = 0.14;
                c.add(b, "cushion");
            }
            b.rbox(
                "dark",
                [-hw / 2.0 + 0.02, 0.0, 0.0],
                [hw / 2.0 - 0.02, 0.1, 0.05],
                0.0,
            );
            let mut bs = Soft::new([x0 - 0.02, 0.11, 0.05], [x1 + 0.02, l, base], 0.03);
            bs.side = 0.005;
            bs.wobble = 0.001;
            bs.seed = 7;
            bs.step = 0.25;
            bs.add(b, "cushion");
            b.rbox(
                "dark",
                [x0 + 0.06, 0.2, 0.0],
                [x1 - 0.06, l - 0.08, 0.05],
                0.0,
            );
            m.y0 = 0.12;
            m.floor = 0.06;
        }
        // Rustic: square pine posts with turned caps, headboard and
        // footboard of planks, the duvet tucked in at the foot.
        5 => {
            let s = 0.085;
            let rail = base - 0.18;
            let (px0, px1) = (x0 - s, x1);
            let foot_top = base + 0.28;
            for (x, y, z) in [
                (px0, 0.0, head + 0.04),
                (px1, 0.0, head + 0.04),
                (px0, l - s, foot_top),
                (px1, l - s, foot_top),
            ] {
                b.rbox("frame", [x, y, 0.0], [x + s, y + s, z], 0.01);
                turned(
                    b,
                    "frame",
                    [x + s / 2.0, y + s / 2.0, z],
                    &[
                        (s * 0.42, 0.0),
                        (s * 0.48, 0.012),
                        (s * 0.3, 0.03),
                        (s * 0.42, 0.055),
                        (s * 0.18, 0.085),
                        (0.0, 0.09),
                    ],
                    16,
                    0.0,
                );
            }
            for x in [px0 + 0.01, x1 + 0.01] {
                b.rbox(
                    "frame",
                    [x - 0.005, s, rail],
                    [x + s - 0.045, l - s, base - 0.02],
                    0.006,
                );
            }
            // Headboard: planks under a top rail.
            planks(b, "frame", x0, x1, rail, head - 0.08, 0.025);
            b.rbox(
                "frame",
                [x0, 0.01, head - 0.08],
                [x1, s - 0.01, head],
                0.008,
            );
            // Footboard.
            planks(b, "frame", x0, x1, rail, foot_top - 0.08, l - s + 0.02);
            b.rbox(
                "frame",
                [x0, l - s + 0.01, foot_top - 0.08],
                [x1, l - 0.01, foot_top],
                0.008,
            );
            m.y0 = 0.07;
            m.y1 = l - s - 0.075;
            m.foot = false;
            m.floor = rail + 0.04;
        }
        // Industrial: black steel pipe, a headboard of bars between two
        // posts, the base on square tube.
        6 => {
            let r = 0.021;
            let (px0, px1) = (x0 - 0.04, x1 + 0.04);
            let yh = 0.03;
            for x in [px0, px1] {
                b.cyl("metal", [x, yh, 0.0], [x, yh, head], r);
                b.cyl("metal", [x, yh, 0.0], [x, yh, 0.012], r * 1.6);
                b.ellipsoid("metal", [x, yh, head + 0.01], [r * 1.25, r * 1.25, r * 1.1]);
            }
            for z in [base + 0.04, head - 0.04] {
                b.cyl("metal", [px0, yh, z], [px1, yh, z], r * 0.85);
                for x in [px0, px1] {
                    b.cyl("metal", [x, yh, z - 0.03], [x, yh, z + 0.03], r * 1.3);
                }
            }
            let n = ((w / 0.16).round() as usize).max(4);
            for i in 1..n {
                let x = px0 + (px1 - px0) * i as f64 / n as f64;
                b.cyl("metal", [x, yh, base + 0.04], [x, yh, head - 0.04], r * 0.6);
            }
            let rail = base - 0.12;
            let t = 0.04;
            for x in [x0 - 0.02, x1 + 0.02] {
                bar(
                    b,
                    "metal",
                    [x, 0.05, rail],
                    [x, l - t / 2.0, rail],
                    t,
                    [1.0, 0.0, 0.0],
                );
            }
            bar(
                b,
                "metal",
                [x0 - 0.02, l - t / 2.0, rail],
                [x1 + 0.02, l - t / 2.0, rail],
                t,
                [0.0, 1.0, 0.0],
            );
            for x in [x0 - 0.02, x1 + 0.02] {
                bar(
                    b,
                    "metal",
                    [x, l - t / 2.0, 0.0],
                    [x, l - t / 2.0, rail + t / 2.0],
                    t,
                    [1.0, 0.0, 0.0],
                );
            }
            b.rbox(
                "frame",
                [x0, 0.06, rail + t / 2.0],
                [x1, l - 0.01, base],
                0.0,
            );
            m.y0 = 0.06;
            m.floor = 0.04;
        }
        // Classic: a buttoned headboard in a moulded frame with a crest,
        // fluted posts with finials, the base on toupie legs.
        _ => {
            let (px0, px1) = (x0 - 0.12, x1 + 0.12);
            for x in [px0 + 0.045, px1 - 0.045] {
                louis_leg(b, "frame", [x, 0.05], head - 0.05, 0.034);
                turned(
                    b,
                    "frame",
                    [x, 0.05, head - 0.05],
                    &[
                        (0.04, 0.0),
                        (0.03, 0.02),
                        (0.036, 0.05),
                        (0.014, 0.075),
                        (0.022, 0.09),
                        (0.0, 0.11),
                    ],
                    16,
                    0.0,
                );
            }
            let crest = 0.12;
            let zr = head - 0.1;
            let pts: Vec<P3> = curve(
                &[
                    [px0 + 0.08, 0.05, zr],
                    [px0 + 0.3, 0.05, zr + 0.01],
                    [0.0, 0.05, zr + crest],
                    [px1 - 0.3, 0.05, zr + 0.01],
                    [px1 - 0.08, 0.05, zr],
                ],
                8,
            );
            sweep(b, "frame", &pts, |_| 0.026, 12, Cap::Flat, Cap::Flat);
            let low = base - 0.02;
            b.rbox(
                "frame",
                [px0 + 0.08, 0.02, low - 0.06],
                [px1 - 0.08, 0.08, low],
                0.012,
            );
            for x in [px0 + 0.08, px1 - 0.08] {
                b.cyl("frame", [x, 0.05, low - 0.03], [x, 0.05, zr], 0.022);
            }
            let mut panel = Soft::new([px0 + 0.1, 0.025, low], [px1 - 0.1, 0.085, zr - 0.02], 0.03);
            panel.crown = 0.012;
            panel.tufts = 0.16;
            panel.tuft_depth = 0.014;
            panel.tuft_faces = [false, true];
            panel.step = 0.026;
            panel.seed = 9;
            panel.wobble = 0.001;
            panel.add(b, "cushion");
            // The crest's carving: a rosette over the middle.
            let start = b.mark();
            turned(
                b,
                "frame",
                [0.0, 0.0, 0.0],
                &[
                    (0.0, 0.0),
                    (0.045, 0.0),
                    (0.04, 0.012),
                    (0.02, 0.022),
                    (0.0, 0.026),
                ],
                16,
                0.0,
            );
            // Turned to look forward (a rotation: it stays closed).
            let zc = zr + crest - 0.035;
            b.transform_from(start, |q| [q[0], 0.07 + q[2], zc - q[1]]);
            let rail = base - 0.2;
            b.rbox(
                "frame",
                [x0 - 0.035, 0.08, rail],
                [x1 + 0.035, l, base],
                0.012,
            );
            b.rbox(
                "frame",
                [x0 - 0.045, 0.08, rail + 0.02],
                [x1 + 0.045, l + 0.01, rail + 0.045],
                0.008,
            );
            for x in [x0 + 0.02, x1 - 0.02] {
                louis_leg(b, "frame", [x, l - 0.05], rail + 0.005, 0.028);
            }
            m.y0 = 0.09;
            m.floor = 0.06;
        }
    }
    m
}

// ---------------------------------------------------------- lamps

/// The bedside lamp of collection `ix`, standing at `(x, y, z)`.
pub(super) fn bedside_lamp(b: &mut Builder, ix: usize, x: f64, y: f64, z: f64) {
    let at = [x, y, z];
    // A shade turned about the vertical: radius at its foot and head.
    let shade = |b: &mut Builder, z0: f64, h: f64, r0: f64, r1: f64| {
        turned(
            b,
            "shade",
            [x, y, z0],
            &[(0.0, 0.0), (r0, 0.0), (r1, h), (0.0, h)],
            32,
            0.0,
        );
    };
    match ix {
        // A round glazed base and a linen drum.
        0 => {
            turned(
                b,
                "ceramic",
                at,
                &[
                    (0.0, 0.0),
                    (0.045, 0.0),
                    (0.075, 0.06),
                    (0.07, 0.13),
                    (0.03, 0.18),
                    (0.014, 0.19),
                    (0.0, 0.19),
                ],
                28,
                0.0,
            );
            b.cyl("metal", [x, y, z + 0.19], [x, y, z + 0.27], 0.006);
            shade(b, z + 0.23, 0.17, 0.15, 0.145);
        }
        // A terracotta gourd under an empire shade.
        1 => {
            turned(
                b,
                "ceramic",
                at,
                &[
                    (0.0, 0.0),
                    (0.05, 0.0),
                    (0.085, 0.07),
                    (0.06, 0.14),
                    (0.05, 0.17),
                    (0.065, 0.2),
                    (0.02, 0.24),
                    (0.0, 0.24),
                ],
                28,
                0.0,
            );
            b.cyl("metal", [x, y, z + 0.24], [x, y, z + 0.31], 0.006);
            shade(b, z + 0.26, 0.17, 0.16, 0.09);
        }
        // Fifties: a turquoise base with a long neck, a tapered shade.
        2 => {
            turned(
                b,
                "ceramic",
                at,
                &[
                    (0.0, 0.0),
                    (0.04, 0.0),
                    (0.07, 0.08),
                    (0.05, 0.15),
                    (0.022, 0.2),
                    (0.018, 0.28),
                    (0.0, 0.28),
                ],
                28,
                0.0,
            );
            b.cyl("metal", [x, y, z + 0.28], [x, y, z + 0.32], 0.007);
            shade(b, z + 0.27, 0.2, 0.15, 0.07);
        }
        // A paper lantern on a little wooden foot.
        3 => {
            turned(
                b,
                "frame",
                at,
                &[(0.0, 0.0), (0.08, 0.0), (0.08, 0.02), (0.0, 0.02)],
                24,
                0.0,
            );
            let n = 10;
            let prof: Vec<(f64, f64)> = (0..=n)
                .map(|i| {
                    let t = i as f64 / n as f64;
                    (0.1 + 0.025 * (PI * t).sin(), 0.32 * t)
                })
                .collect();
            let mut full = vec![(0.0, 0.0)];
            full.extend(prof);
            full.push((0.0, 0.32));
            turned(b, "shade", [x, y, z + 0.02], &full, 32, 0.0);
        }
        // Contemporary: a mushroom lamp, a dome on a slim stem.
        4 => {
            turned(
                b,
                "ceramic",
                at,
                &[
                    (0.0, 0.0),
                    (0.08, 0.0),
                    (0.08, 0.012),
                    (0.018, 0.02),
                    (0.016, 0.26),
                    (0.0, 0.26),
                ],
                28,
                0.0,
            );
            let n = 10;
            let mut dome: Vec<(f64, f64)> = vec![(0.0, 0.0)];
            for i in 0..=n {
                let t = FRAC_PI_2 * i as f64 / n as f64;
                dome.push((0.14 * t.cos(), 0.11 * t.sin()));
            }
            turned(b, "shade", [x, y, z + 0.24], &dome, 32, 0.0);
        }
        // Rustic: an iron candlestick, an empire shade.
        5 => {
            turned(
                b,
                "metal",
                at,
                &[
                    (0.0, 0.0),
                    (0.07, 0.0),
                    (0.06, 0.015),
                    (0.02, 0.03),
                    (0.03, 0.05),
                    (0.014, 0.08),
                    (0.012, 0.24),
                    (0.028, 0.26),
                    (0.012, 0.28),
                    (0.0, 0.3),
                ],
                16,
                0.0,
            );
            shade(b, z + 0.25, 0.18, 0.15, 0.1);
        }
        // Industrial: a black tube on a disc, a bare bulb in a cage.
        6 => {
            turned(
                b,
                "metal",
                at,
                &[(0.0, 0.0), (0.075, 0.0), (0.075, 0.015), (0.0, 0.015)],
                24,
                0.0,
            );
            b.cyl("metal", [x, y, z], [x, y, z + 0.26], 0.011);
            b.cyl("metal", [x, y, z + 0.25], [x, y, z + 0.29], 0.02);
            b.ellipsoid("bulb", [x, y, z + 0.34], [0.04, 0.04, 0.055]);
            for k in 0..6 {
                let a = TAU * k as f64 / 6.0;
                let pts: Vec<P3> = (0..=8)
                    .map(|i| {
                        let t = PI * i as f64 / 8.0;
                        let r = 0.065 * t.sin() + 0.02;
                        [
                            x + r * a.cos(),
                            y + r * a.sin(),
                            z + 0.29 + 0.13 * (1.0 - t.cos()) / 2.0,
                        ]
                    })
                    .collect();
                tube(b, "metal", &pts, false, 0.0022, 5);
            }
            let ring: Vec<P3> = (0..18)
                .map(|k| {
                    let a = TAU * k as f64 / 18.0;
                    [x + 0.085 * a.cos(), y + 0.085 * a.sin(), z + 0.355]
                })
                .collect();
            tube(b, "metal", &ring, true, 0.0025, 5);
        }
        // Classic: a turned brass column, a drum shade.
        _ => {
            turned(
                b,
                "metal",
                at,
                &[
                    (0.0, 0.0),
                    (0.07, 0.0),
                    (0.07, 0.012),
                    (0.05, 0.03),
                    (0.024, 0.045),
                    (0.034, 0.07),
                    (0.018, 0.1),
                    (0.015, 0.25),
                    (0.03, 0.27),
                    (0.012, 0.29),
                    (0.0, 0.3),
                ],
                24,
                0.0,
            );
            shade(b, z + 0.27, 0.19, 0.16, 0.13);
        }
    }
}

// ---------------------------------------------------------- bedside tables

/// Bedside tables of the collections (after the family's 4 own models),
/// each with its lamp on it.
pub fn nightstand(b: &mut Builder, p: &Params) -> bool {
    let Some(ix) = look(p, 4) else {
        return false;
    };
    let w = p.get("width", 0.45).clamp(0.3, 0.8);
    let d = p.get("depth", 0.4).clamp(0.3, 0.6);
    let h = p.get("height", 0.5).clamp(0.35, 0.75);
    let (x0, x1) = (-w / 2.0, w / 2.0);
    let fy = d - T;
    let mut top_z = h;
    let mut lamp_y = d * 0.45;
    match ix {
        // Nordic: oak on splayed legs, a drawer over an open shelf.
        0 => {
            let z0 = 0.2;
            for (q, o) in corners(x0, x1, d, 0.05).iter().zip(OUT) {
                tapered_leg(b, "frame", *q, z0 + 0.005, o, 0.018, 0.015);
            }
            case(b, "frame", x0, x1, fy, z0, h - 0.02, 0.018);
            b.rbox(
                "top",
                [x0 - 0.005, -0.005, h - 0.02],
                [x1 + 0.005, d + 0.005, h],
                0.008,
            );
            let zd = h - 0.02 - 0.12;
            b.rbox(
                "frame",
                [x0 + 0.018, 0.0, zd - 0.018],
                [x1 - 0.018, fy, zd],
                0.0,
            );
            front(
                b,
                Panel::Flat,
                "front",
                x0 + 0.003,
                x1 - 0.003,
                zd,
                h - 0.023,
                fy,
            );
            pull(b, Pull::Slot, 0.0, h - 0.024, fy + T);
        }
        // Mediterranean: whitewashed on square legs, a cane door.
        1 => {
            let z0 = 0.12;
            square_legs(b, "frame", x0, x1, fy, h - 0.03, 0.04);
            b.rbox(
                "frame",
                [x0 + 0.01, 0.0, z0],
                [x1 - 0.01, fy, h - 0.03],
                0.0,
            );
            b.rbox("frame", [x0 + 0.04, fy, z0 - 0.03], [x1 - 0.04, d, z0], 0.0);
            b.rbox(
                "frame",
                [x0 - 0.012, -0.005, h - 0.03],
                [x1 + 0.012, d + 0.012, h],
                0.006,
            );
            front(
                b,
                Panel::Cane,
                "front",
                x0 + 0.045,
                x1 - 0.045,
                z0 + 0.005,
                h - 0.035,
                fy,
            );
            pull(b, Pull::Knob, x1 - 0.085, (z0 + h) / 2.0, fy + T);
        }
        // Fifties: teak on compass legs, a drawer with a brass bar over
        // an open niche.
        2 => {
            let z0 = 0.22;
            for (q, o) in corners(x0, x1, d, 0.07).iter().zip(OUT) {
                compass_leg(b, *q, z0 + 0.005, o, 0.03);
            }
            case(b, "frame", x0, x1, fy, z0, h, 0.02);
            let zd = h - 0.13;
            b.rbox(
                "frame",
                [x0 + 0.02, 0.0, zd - 0.018],
                [x1 - 0.02, fy, zd],
                0.0,
            );
            front(
                b,
                Panel::Flat,
                "front",
                x0 + 0.003,
                x1 - 0.003,
                zd,
                h - 0.003,
                fy,
            );
            pull(b, Pull::Bar(0.1), 0.0, (zd + h) / 2.0, fy + T);
        }
        // Japandi: a low oak box on a dark plinth, open below a drawer
        // with a finger cut.
        3 => {
            let z0 = 0.06;
            b.rbox(
                "dark",
                [x0 + 0.04, 0.04, 0.0],
                [x1 - 0.04, d - 0.06, z0],
                0.0,
            );
            case(b, "frame", x0, x1, fy, z0, h, 0.022);
            let zd = h - 0.12;
            b.rbox(
                "frame",
                [x0 + 0.022, 0.0, zd - 0.02],
                [x1 - 0.022, fy, zd],
                0.0,
            );
            front(
                b,
                Panel::Flat,
                "front",
                x0 + 0.003,
                x1 - 0.003,
                zd,
                h - 0.003,
                fy,
            );
            pull(b, Pull::Slot, 0.0, h - 0.004, fy + T);
            // A book or two in the niche.
            b.rbox(
                "books",
                [x0 + 0.04, 0.05, z0 + 0.022],
                [x0 + 0.26_f64.min(w * 0.6), fy - 0.03, z0 + 0.05],
                0.0,
            );
            b.rbox(
                "books2",
                [x0 + 0.05, 0.06, z0 + 0.05],
                [x0 + 0.24_f64.min(w * 0.55), fy - 0.04, z0 + 0.072],
                0.0,
            );
        }
        // Contemporary: a lacquered box of two handleless drawers on a
        // slim steel sled.
        4 => {
            let z0 = 0.1;
            for x in [x0 + 0.02, x1 - 0.02] {
                let t = 0.012;
                bar(
                    b,
                    "metal",
                    [x, 0.03, t / 2.0],
                    [x, d - 0.03, t / 2.0],
                    t,
                    [1.0, 0.0, 0.0],
                );
                for y in [0.03, d - 0.03] {
                    bar(
                        b,
                        "metal",
                        [x, y, 0.0],
                        [x, y, z0 + 0.005],
                        t,
                        [1.0, 0.0, 0.0],
                    );
                }
            }
            b.rbox("frame", [x0, 0.0, z0], [x1, fy, h], 0.0);
            gola_stack(b, x0, x1, z0 + 0.003, h, fy, 2);
        }
        // Rustic: pine on turned legs, a drawer with an iron ring, a
        // shelf of planks below.
        5 => {
            let z0 = 0.18;
            for q in corners(x0, x1, d, 0.03) {
                turned_post(b, "frame", q, 0.0, h - 0.03, 0.022);
            }
            b.rbox(
                "frame",
                [x0 + 0.01, 0.01, z0],
                [x1 - 0.01, fy, h - 0.03],
                0.0,
            );
            b.rbox(
                "frame",
                [x0 + 0.02, 0.02, 0.07],
                [x1 - 0.02, d - 0.02, 0.09],
                0.0,
            );
            b.rbox(
                "frame",
                [x0 - 0.015, -0.005, h - 0.03],
                [x1 + 0.015, d + 0.015, h],
                0.006,
            );
            let zd = z0 + 0.03;
            front(
                b,
                Panel::Planks,
                "front",
                x0 + 0.04,
                x1 - 0.04,
                zd,
                h - 0.045,
                fy,
            );
            pull(b, Pull::Ring, 0.0, (zd + h) / 2.0 + 0.01, fy + T);
        }
        // Industrial: a frame of steel tube, a wooden top and drawer, a
        // shelf below.
        6 => {
            storage::steel_frame(
                b,
                x0,
                x1,
                d,
                h - 0.025,
                &[0.06, h - 0.17, h - 0.04],
                0.022,
                false,
            );
            b.rbox(
                "top",
                [x0 - 0.005, -0.005, h - 0.025],
                [x1 + 0.005, d + 0.005, h],
                0.0,
            );
            b.rbox(
                "top",
                [x0 + 0.02, 0.02, 0.07],
                [x1 - 0.02, d - 0.02, 0.09],
                0.0,
            );
            b.rbox(
                "frame",
                [x0 + 0.03, 0.02, h - 0.165],
                [x1 - 0.03, fy - 0.01, h - 0.045],
                0.0,
            );
            front(
                b,
                Panel::Flat,
                "front",
                x0 + 0.03,
                x1 - 0.03,
                h - 0.165,
                h - 0.045,
                fy - 0.01,
            );
            pull(b, Pull::Cup, 0.0, h - 0.105, fy - 0.01 + T);
        }
        // Classic: Louis XVI, on fluted legs, two panelled drawers with
        // brass knobs, a marble top.
        _ => {
            let z0 = 0.2;
            for q in corners(x0, x1, d, 0.03) {
                louis_leg(b, "frame", q, z0 + 0.03, 0.024);
            }
            b.rbox("frame", [x0, 0.0, z0], [x1, fy, h - 0.03], 0.0);
            drawer_stack(
                b,
                Panel::Moulded,
                x0 + 0.02,
                x1 - 0.02,
                z0 + 0.02,
                h - 0.05,
                fy,
                2,
                Pull::Brass,
            );
            b.rbox(
                "stone",
                [x0 - 0.015, -0.005, h - 0.03],
                [x1 + 0.015, d + 0.015, h],
                0.008,
            );
            top_z = h;
            lamp_y = d * 0.42;
        }
    }
    if p.flag("lamp", true) {
        bedside_lamp(b, ix, x0 + w * 0.56, lamp_y, top_z);
    }
    true
}

// ---------------------------------------------------------- wardrobes

/// Wardrobes of the collections (after the family's own model): the
/// carcass on what it stands, `doors` doors over `drawers` rows of
/// drawers.
pub fn wardrobe(b: &mut Builder, p: &Params) -> bool {
    let Some(ix) = look(p, 1) else {
        return false;
    };
    let w = p.get("width", 1.2).clamp(0.4, 6.0);
    let d = p.get("depth", 0.6).clamp(0.3, 1.0);
    let h = p.get("height", 2.4).clamp(0.8, 3.0);
    let doors = p.get("doors", 2.0).round().clamp(1.0, 12.0) as usize;
    let rows = p.get("drawers", 0.0).round().clamp(0.0, 4.0) as usize;
    let (x0, x1) = (-w / 2.0, w / 2.0);
    let fy = d - T;
    // (panel, pull, foot of the carcass, top of the carcass)
    let (panel, kind, z0, zt) = match ix {
        0 => (Panel::Flat, Pull::Tab, 0.08, h),
        1 => (Panel::Cane, Pull::Knob, 0.12, h - 0.035),
        2 => (Panel::Flat, Pull::Bar(0.3), 0.24, h),
        3 => (Panel::Ribs, Pull::None, 0.08, h),
        4 => (Panel::Flat, Pull::None, 0.06, h),
        5 => (Panel::Planks, Pull::Ring, 0.1, h - 0.035),
        6 => (Panel::Flat, Pull::Cup, 0.05, h - 0.03),
        _ => (Panel::Raised, Pull::Brass, 0.12, h - 0.035),
    };
    // What it stands on.
    match ix {
        0 => b.rbox(
            "plinth",
            [x0 + 0.02, 0.02, 0.0],
            [x1 - 0.02, d - 0.05, z0],
            0.0,
        ),
        1 => {
            square_legs(b, "frame", x0, x1, fy, z0 + 0.01, 0.05);
            storage::cornice(b, "frame", x0, x1, fy, h, 0.025);
        }
        2 => {
            for (q, o) in corners(x0, x1, d, 0.1).iter().zip(OUT) {
                compass_leg(b, *q, z0 + 0.005, o, 0.035);
            }
            if w > 1.6 {
                for q in [[0.0, 0.1], [0.0, d - 0.1]] {
                    compass_leg(
                        b,
                        q,
                        z0 + 0.005,
                        [0.0, if q[1] < 0.2 { -1.0 } else { 1.0 }],
                        0.03,
                    );
                }
            }
        }
        3 | 4 => b.rbox(
            "dark",
            [x0 + 0.03, 0.03, 0.0],
            [x1 - 0.03, d - 0.06, z0],
            0.0,
        ),
        5 => {
            storage::base_moulding(b, "frame", x0, x1, fy, 0.0, 0.015);
            b.rbox("frame", [x0, 0.0, 0.0], [x1, fy, z0], 0.0);
            storage::cornice(b, "frame", x0, x1, fy, h, 0.04);
        }
        6 => {
            let rails = [0.05, zt - 0.01];
            storage::steel_frame(b, x0 - 0.03, x1 + 0.03, d + 0.02, h, &rails, 0.035, true);
        }
        _ => {
            storage::base_moulding(b, "frame", x0, x1, fy, 0.0, 0.02);
            b.rbox("frame", [x0, 0.0, 0.0], [x1, fy, z0], 0.0);
            storage::cornice(b, "frame", x0, x1, fy, h, 0.05);
        }
    }
    // The carcass.
    b.rbox(
        "frame",
        [x0, 0.0, z0],
        [x1, fy, zt],
        if ix == 2 { 0.02 } else { 0.004 },
    );
    // Drawers under the doors, a column under each pair of doors.
    let row_h = 0.21;
    let zd = z0 + rows as f64 * row_h;
    let (dx0, dx1) = (x0 + 0.003, x1 - 0.003);
    if rows > 0 {
        let cols = doors.div_ceil(2);
        let cw = (dx1 - dx0) / cols as f64;
        for c in 0..cols {
            let a = dx0 + cw * c as f64;
            if ix == 4 {
                gola_stack(b, a + 0.002, a + cw - 0.002, z0 + 0.003, zd, fy, rows);
            } else {
                drawer_stack(
                    b,
                    panel_for_drawers(panel),
                    a + 0.002,
                    a + cw - 0.002,
                    z0 + 0.003,
                    zd,
                    fy,
                    rows,
                    drawer_pull(kind),
                );
            }
        }
    }
    let pz = (1.05_f64).clamp(zd + 0.15, zt - 0.15);
    let top = zt - 0.003;
    match ix {
        // Japandi: sliding doors of slats, one before the other.
        3 => {
            let n = doors.max(2);
            let step = (dx1 - dx0) / n as f64;
            for i in 0..n {
                let (a, e) = (
                    dx0 + step * i as f64 - 0.01,
                    dx0 + step * (i + 1) as f64 + 0.01,
                );
                let (a, e) = (a.max(dx0), e.min(dx1));
                let y = if i.is_multiple_of(2) {
                    fy - 0.004
                } else {
                    fy + 0.016
                };
                front(b, Panel::Ribs, "front", a, e, zd + 0.003, top, y);
                // A recessed finger pull at its leading edge.
                let xe = if i.is_multiple_of(2) {
                    e - 0.03
                } else {
                    a + 0.03
                };
                b.rbox(
                    "dark",
                    [xe - 0.008, y + T - 0.001, pz - 0.15],
                    [xe + 0.008, y + T + 0.002, pz + 0.15],
                    0.0,
                );
            }
        }
        // Contemporary: handleless, a dark recess down where two doors
        // meet.
        4 => {
            storage::doors(
                b,
                Panel::Flat,
                "front",
                dx0,
                dx1,
                zd + 0.003,
                top,
                fy,
                doors,
                Pull::None,
                pz,
            );
            let step = (dx1 - dx0) / doors as f64;
            for i in 0..doors {
                if i.is_multiple_of(2) && i + 1 < doors {
                    let x = dx0 + step * (i + 1) as f64;
                    b.rbox(
                        "dark",
                        [x - 0.012, fy - 0.01, zd + 0.003],
                        [x + 0.012, fy + 0.001, top],
                        0.0,
                    );
                }
            }
        }
        _ => {
            storage::doors(
                b,
                panel,
                "front",
                dx0,
                dx1,
                zd + 0.003,
                top,
                fy,
                doors,
                kind,
                pz,
            );
            if ix == 5 {
                let step = (dx1 - dx0) / doors as f64;
                for i in 0..doors {
                    let (a, e) = (dx0 + step * i as f64, dx0 + step * (i + 1) as f64);
                    let towards_right = if doors.is_multiple_of(2) {
                        i < doors / 2
                    } else {
                        i.is_multiple_of(2)
                    };
                    let (x, toward) = if towards_right {
                        (a + 0.01, 1.0)
                    } else {
                        (e - 0.01, -1.0)
                    };
                    storage::straps(b, x, toward, zd, top, fy + T);
                }
            }
            if ix == 7 {
                // Pilasters at its two ends.
                for x in [x0 - 0.01, x1 - 0.03] {
                    b.rbox(
                        "frame",
                        [x, fy, z0 + 0.02],
                        [x + 0.04, fy + 0.025, zt - 0.02],
                        0.0,
                    );
                }
            }
        }
    }
    true
}

/// The panel of the drawers under a wardrobe's doors.
fn panel_for_drawers(doors: Panel) -> Panel {
    match doors {
        Panel::Cane | Panel::Glass | Panel::Grid(..) => Panel::Flat,
        p => p,
    }
}

/// The pull of a drawer under doors with `kind`.
fn drawer_pull(kind: Pull) -> Pull {
    match kind {
        Pull::Bar(_) => Pull::Bar(0.12),
        Pull::Tab => Pull::Slot,
        k => k,
    }
}

// ---------------------------------------------------------- chests

/// Chests of drawers of the collections (after the family's 2 own
/// models): `drawers` rows in `columns` columns.
pub fn chest(b: &mut Builder, p: &Params) -> bool {
    let Some(ix) = look(p, 2) else {
        return false;
    };
    let w = p.get("width", 1.0).clamp(0.4, 2.4);
    let d = p.get("depth", 0.5).clamp(0.3, 0.7);
    let h = p.get("height", 0.85).clamp(0.4, 1.4);
    let rows = p.get("drawers", 4.0).round().clamp(1.0, 10.0) as usize;
    let cols = p.get("columns", 1.0).round().clamp(1.0, 4.0) as usize;
    let (x0, x1) = (-w / 2.0, w / 2.0);
    let fy = d - T;
    let (panel, kind, z0, ztop, top_role) = match ix {
        0 => (Panel::Flat, Pull::Slot, 0.18, h - 0.025, "top"),
        1 => (Panel::Flat, Pull::Knob, 0.1, h - 0.03, "frame"),
        2 => (Panel::Flat, Pull::Bar(0.12), 0.22, h, ""),
        3 => (Panel::Flat, Pull::Slot, 0.06, h, ""),
        4 => (Panel::Flat, Pull::None, 0.06, h, ""),
        5 => (Panel::Flat, Pull::Cup, 0.08, h - 0.035, "frame"),
        6 => (Panel::Flat, Pull::Cup, 0.06, h - 0.025, "top"),
        _ => (Panel::Moulded, Pull::Brass, 0.16, h - 0.03, "stone"),
    };
    match ix {
        0 => {
            for (q, o) in corners(x0, x1, d, 0.07).iter().zip(OUT) {
                tapered_leg(b, "frame", *q, z0 + 0.005, o, 0.025, 0.018);
            }
        }
        1 => square_legs(b, "frame", x0, x1, fy, z0 + 0.01, 0.045),
        2 => {
            for (q, o) in corners(x0, x1, d, 0.09).iter().zip(OUT) {
                compass_leg(b, *q, z0 + 0.005, o, 0.035);
            }
        }
        3 | 4 => b.rbox(
            "dark",
            [x0 + 0.04, 0.04, 0.0],
            [x1 - 0.04, d - 0.06, z0],
            0.0,
        ),
        5 => {
            for q in corners(x0, x1, fy, 0.05) {
                turned(
                    b,
                    "frame",
                    [q[0], q[1], 0.0],
                    &[
                        (0.0, 0.0),
                        (0.03, 0.0),
                        (0.045, 0.03),
                        (0.04, 0.06),
                        (0.03, z0),
                        (0.0, z0),
                    ],
                    16,
                    0.0,
                );
            }
            storage::base_moulding(b, "frame", x0, x1, fy, z0, 0.012);
        }
        6 => storage::steel_frame(
            b,
            x0 - 0.025,
            x1 + 0.025,
            d + 0.02,
            ztop,
            &[0.04, ztop - 0.015],
            0.03,
            false,
        ),
        _ => {
            for q in corners(x0, x1, d, 0.035) {
                louis_leg(b, "frame", q, z0 + 0.03, 0.026);
            }
        }
    }
    b.rbox(
        "frame",
        [x0, 0.0, z0],
        [x1, fy, ztop],
        if ix == 2 { 0.018 } else { 0.004 },
    );
    if !top_role.is_empty() {
        let o = if ix == 7 { 0.02 } else { 0.01 };
        b.rbox(top_role, [x0 - o, -0.005, ztop], [x1 + o, d + o, h], 0.008);
    }
    let (dz0, dz1) = (z0 + 0.012, ztop - 0.012);
    let cw = (w - 0.02) / cols as f64;
    for c in 0..cols {
        let a = x0 + 0.01 + c as f64 * cw;
        if ix == 4 {
            gola_stack(b, a + 0.002, a + cw - 0.002, dz0, ztop - 0.003, fy, rows);
        } else {
            drawer_stack(
                b,
                panel,
                a + 0.002,
                a + cw - 0.002,
                dz0,
                dz1,
                fy,
                rows,
                kind,
            );
        }
    }
    if ix == 7 {
        // An apron with a carved rosette under the drawers.
        b.rbox(
            "frame",
            [x0 + 0.03, fy, z0 - 0.04],
            [x1 - 0.03, d - 0.005, z0],
            0.006,
        );
    }
    true
}

// ---------------------------------------------------------- benches

/// The bench at the foot of the bed of each collection (styles 1 to 8):
/// `width`, `depth`, `height` of its seat.
pub fn bed_bench(b: &mut Builder, p: &Params) {
    let ix = (p.style() - 1).clamp(0, 7) as usize;
    let w = p.get("width", 1.2).clamp(0.5, 2.2);
    let d = p.get("depth", 0.42).clamp(0.3, 0.7);
    let h = p.get("height", 0.46).clamp(0.3, 0.6);
    let (x0, x1) = (-w / 2.0, w / 2.0);
    let seat = |b: &mut Builder, lo: P3, hi: P3, r: f64, seed: u32| {
        let mut s = Soft::new(lo, hi, r);
        s.crown = 0.016;
        s.side = 0.008;
        s.sag = 0.008;
        s.creases = 0.004;
        s.wobble = 0.0015;
        s.welt = 0.005;
        s.seed = seed;
        s.add(b, "cushion");
    };
    match ix {
        // Nordic: an upholstered seat on splayed oak legs.
        0 => {
            let z0 = h - 0.12;
            for (q, o) in corners(x0, x1, d, 0.06).iter().zip(OUT) {
                tapered_leg(b, "frame", *q, z0 + 0.01, o, 0.02, 0.017);
            }
            b.rbox(
                "frame",
                [x0 + 0.03, 0.03, z0 - 0.02],
                [x1 - 0.03, d - 0.03, z0 + 0.005],
                0.0,
            );
            seat(b, [x0, 0.0, z0], [x1, d, h], 0.04, 31);
        }
        // Mediterranean: whitewashed with a rush seat and stretchers.
        1 => {
            let s = 0.045;
            square_legs(b, "frame", x0, x1, d, h, s);
            for (ya, yb) in [(0.0, s), (d - s, d)] {
                b.rbox(
                    "frame",
                    [x0 + s, ya + 0.005, h - 0.05],
                    [x1 - s, yb - 0.005, h - 0.01],
                    0.0,
                );
                b.rbox(
                    "frame",
                    [x0 + s, ya + 0.01, 0.1],
                    [x1 - s, yb - 0.01, 0.13],
                    0.0,
                );
            }
            for xa in [x0, x1 - s] {
                b.rbox(
                    "frame",
                    [xa + 0.005, s, h - 0.05],
                    [xa + s - 0.005, d - s, h - 0.01],
                    0.0,
                );
            }
            // The rush seat, woven over the rails.
            let mut rush = Soft::new(
                [x0 + s * 0.6, s * 0.6, h - 0.03],
                [x1 - s * 0.6, d - s * 0.6, h + 0.004],
                0.012,
            );
            rush.crown = 0.004;
            rush.sag = 0.006;
            rush.seed = 32;
            rush.add_faces(b, "weave", [true; 6]);
        }
        // Fifties: teak slats on compass legs with brass shoes.
        2 => {
            let z0 = h - 0.05;
            for (q, o) in corners(x0, x1, d, 0.08).iter().zip(OUT) {
                compass_leg(b, *q, z0 + 0.005, o, 0.03);
            }
            // Two bearers across, on the legs, under the slats.
            for x in [x0 + 0.08, x1 - 0.08] {
                b.rbox(
                    "frame",
                    [x - 0.02, 0.03, z0 - 0.04],
                    [x + 0.02, d - 0.03, z0],
                    0.0,
                );
            }
            let n = ((d / 0.06).round() as usize).max(4);
            let step = d / n as f64;
            for i in 0..n {
                let y = step * i as f64;
                b.rbox(
                    "frame",
                    [x0, y + 0.006, z0],
                    [x1, y + step - 0.006, h],
                    0.008,
                );
            }
        }
        // Japandi: a low slab of oak on two slab legs, a cushion on it.
        3 => {
            let t = 0.04;
            let hh = h - 0.06;
            for x in [x0 + 0.08, x1 - 0.08 - t] {
                b.rbox("frame", [x, 0.03, 0.0], [x + t, d - 0.03, hh - t], 0.0);
            }
            b.rbox("frame", [x0, 0.0, hh - t], [x1, d, hh], 0.006);
            seat(b, [x0 + 0.04, 0.03, hh], [x1 - 0.04, d - 0.03, h], 0.03, 33);
        }
        // Contemporary: an upholstered block on a shadow gap.
        4 => {
            b.rbox(
                "dark",
                [x0 + 0.04, 0.04, 0.0],
                [x1 - 0.04, d - 0.04, 0.04],
                0.0,
            );
            let mut s = Soft::new([x0, 0.0, 0.04], [x1, d, h], 0.03);
            s.crown = 0.01;
            s.side = 0.006;
            s.wobble = 0.001;
            s.seed = 34;
            s.add(b, "cushion");
        }
        // Rustic: a pine chest with iron straps and handles.
        5 => {
            b.rbox("frame", [x0, 0.0, 0.0], [x1, d, h - 0.04], 0.006);
            front(
                b,
                Panel::Planks,
                "frame",
                x0 + 0.01,
                x1 - 0.01,
                0.04,
                h - 0.05,
                d - 0.004,
            );
            b.rbox(
                "frame",
                [x0 - 0.01, -0.01, h - 0.045],
                [x1 + 0.01, d + 0.01, h],
                0.008,
            );
            for x in [x0 + 0.12, x1 - 0.12] {
                b.rbox(
                    "metal",
                    [x - 0.022, d + 0.012, 0.0],
                    [x + 0.022, d + 0.018, h],
                    0.0,
                );
                b.rbox(
                    "metal",
                    [x - 0.022, -0.018, 0.0],
                    [x + 0.022, -0.012, h],
                    0.0,
                );
                b.rbox(
                    "metal",
                    [x - 0.022, -0.018, h - 0.006],
                    [x + 0.022, d + 0.018, h + 0.002],
                    0.0,
                );
            }
            for (x, s) in [(x0, -1.0), (x1, 1.0)] {
                let pts: Vec<P3> = (0..=8)
                    .map(|k| {
                        let a = PI * k as f64 / 8.0;
                        [
                            // Its ends set into the chest's side.
                            x + s * (-0.004 + 0.04 * a.sin()),
                            d / 2.0 + 0.06 * a.cos(),
                            h - 0.12,
                        ]
                    })
                    .collect();
                tube(b, "metal", &pts, false, 0.006, 6);
            }
            pull(b, Pull::Ring, 0.0, h - 0.07, d + 0.004);
        }
        // Industrial: a frame of square steel tube, a leather cushion on
        // a wooden board.
        6 => {
            let t = 0.03;
            let z = h - 0.09;
            for q in corners(x0, x1, d, t / 2.0) {
                bar(
                    b,
                    "metal",
                    [q[0], q[1], 0.0],
                    [q[0], q[1], z],
                    t,
                    [1.0, 0.0, 0.0],
                );
            }
            for (a, c) in [
                ([x0 + t / 2.0, t / 2.0], [x1 - t / 2.0, t / 2.0]),
                ([x0 + t / 2.0, d - t / 2.0], [x1 - t / 2.0, d - t / 2.0]),
                ([x0 + t / 2.0, t / 2.0], [x0 + t / 2.0, d - t / 2.0]),
                ([x1 - t / 2.0, t / 2.0], [x1 - t / 2.0, d - t / 2.0]),
            ] {
                for zz in [0.08, z - t / 2.0] {
                    bar(
                        b,
                        "metal",
                        [a[0], a[1], zz],
                        [c[0], c[1], zz],
                        t,
                        [0.0, 0.0, 1.0],
                    );
                }
            }
            b.rbox(
                "frame",
                [x0 + 0.005, 0.005, z],
                [x1 - 0.005, d - 0.005, z + 0.025],
                0.0,
            );
            seat(
                b,
                [x0 + 0.02, 0.02, z + 0.025],
                [x1 - 0.02, d - 0.02, h],
                0.03,
                36,
            );
        }
        // Classic: a buttoned seat on fluted legs, a moulded apron.
        _ => {
            let z0 = h - 0.13;
            for q in corners(x0, x1, d, 0.035) {
                louis_leg(b, "frame", q, z0 + 0.06, 0.022);
            }
            b.rbox(
                "frame",
                [x0 + 0.01, 0.01, z0 - 0.03],
                [x1 - 0.01, d - 0.01, z0 + 0.03],
                0.008,
            );
            let mut s = Soft::new(
                [x0 - 0.01, -0.01, z0 + 0.03],
                [x1 + 0.01, d + 0.01, h],
                0.035,
            );
            s.crown = 0.018;
            s.side = 0.006;
            s.tufts = 0.15;
            s.tuft_depth = 0.012;
            s.step = 0.03;
            s.seed = 37;
            s.welt = 0.004;
            s.add(b, "cushion");
        }
    }
}

// ---------------------------------------------------------- dressing tables

/// The dressing table of each collection (styles 1 to 8): the table with
/// its drawer, the mirror over it (`mirror`) and its stool (`stool`).
/// `width`, `depth`, `height` of the table; the mirror hangs at
/// `mirror_height` over it.
pub fn dressing_table(b: &mut Builder, p: &Params) {
    let ix = (p.style() - 1).clamp(0, 7) as usize;
    let w = p.get("width", 1.0).clamp(0.6, 1.8);
    let d = p.get("depth", 0.45).clamp(0.3, 0.7);
    let h = p.get("height", 0.76).clamp(0.6, 0.9);
    let (x0, x1) = (-w / 2.0, w / 2.0);
    let apron = 0.13;
    let za = h - 0.025 - apron;
    let fy = d - T;
    match ix {
        0 => {
            for (q, o) in corners(x0, x1, d, 0.05).iter().zip(OUT) {
                tapered_leg(b, "frame", *q, za + 0.01, o, 0.02, 0.018);
            }
            b.rbox(
                "frame",
                [x0 + 0.02, 0.02, za],
                [x1 - 0.02, fy, h - 0.025],
                0.0,
            );
            b.rbox("top", [x0, 0.0, h - 0.025], [x1, d, h], 0.008);
            front(
                b,
                Panel::Flat,
                "front",
                -w * 0.3,
                w * 0.3,
                za + 0.012,
                h - 0.03,
                fy,
            );
            pull(b, Pull::Slot, 0.0, h - 0.031, fy + T);
        }
        1 => {
            square_legs(b, "frame", x0, x1, d, h - 0.03, 0.045);
            b.rbox(
                "frame",
                [x0 + 0.02, 0.02, za],
                [x1 - 0.02, fy, h - 0.03],
                0.0,
            );
            b.rbox(
                "frame",
                [x0 - 0.012, -0.005, h - 0.03],
                [x1 + 0.012, d + 0.012, h],
                0.006,
            );
            front(
                b,
                Panel::Cane,
                "front",
                -w * 0.28,
                w * 0.28,
                za + 0.01,
                h - 0.035,
                fy,
            );
            // On the cane, which is set back from the frame.
            pull(b, Pull::Knob, 0.0, za + apron / 2.0, fy + T - 0.01);
        }
        2 => {
            for (q, o) in corners(x0, x1, d, 0.07).iter().zip(OUT) {
                compass_leg(b, *q, za + 0.005, o, 0.035);
            }
            b.rbox("frame", [x0, 0.0, za], [x1, fy, h], 0.015);
            for (a, e) in [(x0 + 0.01, -0.005), (0.005, x1 - 0.01)] {
                front(b, Panel::Flat, "front", a, e, za + 0.01, h - 0.01, fy);
                pull(
                    b,
                    Pull::Bar(0.08),
                    (a + e) / 2.0,
                    za + apron / 2.0 + 0.012,
                    fy + T,
                );
            }
        }
        3 => {
            // Hung on the wall: a slim oak box, a drawer with a finger cut.
            b.rbox("frame", [x0, 0.0, za], [x1, fy, h], 0.006);
            front(
                b,
                Panel::Flat,
                "front",
                x0 + 0.004,
                x1 - 0.004,
                za + 0.004,
                h - 0.004,
                fy,
            );
            pull(b, Pull::Slot, 0.0, h - 0.005, fy + T);
        }
        4 => {
            for x in [x0 + 0.03, x1 - 0.03] {
                let t = 0.014;
                bar(
                    b,
                    "metal",
                    [x, 0.04, 0.0],
                    [x, 0.04, za + 0.005],
                    t,
                    [1.0, 0.0, 0.0],
                );
                bar(
                    b,
                    "metal",
                    [x, d - 0.04, 0.0],
                    [x, d - 0.04, za + 0.005],
                    t,
                    [1.0, 0.0, 0.0],
                );
                bar(
                    b,
                    "metal",
                    [x, 0.04, t / 2.0],
                    [x, d - 0.04, t / 2.0],
                    t,
                    [1.0, 0.0, 0.0],
                );
            }
            b.rbox("frame", [x0, 0.0, za], [x1, fy, h], 0.0);
            gola_stack(b, x0, x1, za + 0.003, h, fy, 1);
        }
        5 => {
            for q in corners(x0, x1, d, 0.03) {
                turned_post(b, "frame", q, 0.0, h - 0.03, 0.022);
            }
            b.rbox(
                "frame",
                [x0 + 0.01, 0.01, za],
                [x1 - 0.01, fy, h - 0.03],
                0.0,
            );
            b.rbox(
                "frame",
                [x0 - 0.015, -0.005, h - 0.03],
                [x1 + 0.015, d + 0.015, h],
                0.006,
            );
            front(
                b,
                Panel::Planks,
                "front",
                -w * 0.3,
                w * 0.3,
                za + 0.012,
                h - 0.04,
                fy,
            );
            pull(b, Pull::Ring, 0.0, za + apron / 2.0 + 0.01, fy + T);
        }
        6 => {
            storage::steel_frame(b, x0, x1, d, h - 0.03, &[0.08, h - 0.04], 0.025, false);
            b.rbox(
                "top",
                [x0 - 0.005, -0.005, h - 0.03],
                [x1 + 0.005, d + 0.005, h],
                0.0,
            );
            b.rbox(
                "frame",
                [-w * 0.3, 0.03, za + 0.01],
                [w * 0.3, fy - 0.01, h - 0.04],
                0.0,
            );
            front(
                b,
                Panel::Flat,
                "front",
                -w * 0.3,
                w * 0.3,
                za + 0.01,
                h - 0.04,
                fy - 0.01,
            );
            pull(b, Pull::Cup, 0.0, za + apron / 2.0, fy - 0.01 + T);
        }
        _ => {
            for q in corners(x0, x1, d, 0.03) {
                louis_leg(b, "frame", q, za + 0.03, 0.024);
            }
            b.rbox("frame", [x0, 0.0, za], [x1, fy, h - 0.03], 0.0);
            drawer_stack(
                b,
                Panel::Moulded,
                -w * 0.32,
                w * 0.32,
                za + 0.012,
                h - 0.04,
                fy,
                1,
                Pull::Brass,
            );
            b.rbox(
                "stone",
                [x0 - 0.015, -0.005, h - 0.03],
                [x1 + 0.015, d + 0.015, h],
                0.008,
            );
        }
    }
    if p.flag("mirror", true) {
        let mh = p.get("mirror_height", 0.75).clamp(0.3, 1.2);
        wall_mirror(b, ix, (w * 0.7).min(0.8), mh, h + 0.22);
    }
    if p.flag("stool", true) {
        let start = b.mark();
        stool(
            b,
            ix,
            (w * 0.45).clamp(0.36, 0.5),
            (h - 0.3).clamp(0.4, 0.5),
        );
        // Pulled out a little in front of the table.
        let y = d * 0.55;
        b.transform_from(start, |q| [q[0], y + q[1], q[2]]);
    }
}

/// The mirror of a dressing table of collection `ix`: `w` wide, `h`
/// high, its foot at `z`, on the wall.
fn wall_mirror(b: &mut Builder, ix: usize, w: f64, h: f64, z: f64) {
    let (x0, x1) = (-w / 2.0, w / 2.0);
    let t = 0.02;
    match ix {
        // Round, in an oak ring.
        0 | 2 => {
            let r = w.min(h) / 2.0;
            let c = [0.0, z + r];
            let ring: Vec<P3> = (0..40)
                .map(|k| {
                    let a = TAU * k as f64 / 40.0;
                    [c[0] + r * a.cos(), 0.02, c[1] + r * a.sin()]
                })
                .collect();
            let role = if ix == 2 { "metal" } else { "frame" };
            tube(b, role, &ring, true, if ix == 2 { 0.012 } else { 0.02 }, 10);
            turned_disc_y(b, "mirror_glass", [c[0], 0.0, c[1]], r - 0.005, 0.016);
        }
        // Arched, in a whitewashed frame.
        1 => {
            let zs = z + h - w / 2.0;
            for x in [x0, x1 - 0.04] {
                b.rbox("frame", [x, 0.0, z], [x + 0.04, 0.03, zs], 0.0);
            }
            b.rbox("frame", [x0, 0.0, z - 0.04], [x1, 0.03, z], 0.0);
            arch_band(b, "frame", x0, x1, zs, z + h, 0.04, 0.0, 0.03);
            arched_board(
                b,
                "mirror_glass",
                x0 + 0.035,
                x1 - 0.035,
                z,
                zs,
                z + h - 0.035,
                0.008,
                0.018,
            );
        }
        // A rounded rectangle without a frame (japandi, contemporary).
        3 | 4 => {
            let r = if ix == 3 { w / 2.0 - 0.01 } else { 0.04 };
            let r = r.min(h / 2.0 - 0.01);
            let pts = super::shape::rounded_rect([0.0, z + h / 2.0], w / 2.0, h / 2.0, r, 0.0, 8);
            let poly: Vec<[f64; 2]> = pts.iter().map(|q| [q[0], q[1]]).collect();
            // A slab in plan, turned up to stand on the wall.
            let start = b.mark();
            b.slab("mirror_glass", &poly, 0.0, 0.012);
            b.transform_from(start, |q| [q[0], q[2], q[1]]);
            // That swap is a mirror: turned back the right way round.
            mirror_fix(b, start);
            if ix == 3 {
                b.rbox(
                    "frame",
                    [x0 + 0.05, 0.0, z - 0.03],
                    [x1 - 0.05, 0.12, z - 0.01],
                    0.0,
                );
            }
        }
        // A square frame of pine, of steel, or a gilt frame with a crest.
        _ => {
            let (role, f) = match ix {
                5 => ("frame", 0.055),
                6 => ("metal", 0.025),
                _ => ("frame", 0.06),
            };
            for (lo, hi) in [
                ([x0, 0.0, z], [x1, t + 0.01, z + f]),
                ([x0, 0.0, z + h - f], [x1, t + 0.01, z + h]),
                ([x0, 0.0, z + f], [x0 + f, t + 0.01, z + h - f]),
                ([x1 - f, 0.0, z + f], [x1, t + 0.01, z + h - f]),
            ] {
                b.rbox(role, lo, hi, 0.0);
            }
            b.rbox(
                "mirror_glass",
                [x0 + f - 0.005, 0.004, z + f - 0.005],
                [x1 - f + 0.005, 0.014, z + h - f + 0.005],
                0.0,
            );
            if ix == 7 {
                let pts = curve(
                    &[
                        [x0 + 0.02, 0.02, z + h],
                        [-w * 0.2, 0.02, z + h + 0.03],
                        [0.0, 0.02, z + h + 0.12],
                        [w * 0.2, 0.02, z + h + 0.03],
                        [x1 - 0.02, 0.02, z + h],
                    ],
                    6,
                );
                sweep(
                    b,
                    "frame",
                    &pts,
                    |_| 0.018,
                    10,
                    Cap::Round(1.0),
                    Cap::Round(1.0),
                );
                for x in [x0, x1] {
                    b.ellipsoid("frame", [x, 0.025, z + h], [0.025, 0.02, 0.025]);
                }
            }
            if ix == 6 {
                // Hung from a bar on two chains of steel.
                b.cyl(
                    "metal",
                    [x0 - 0.05, 0.03, z + h + 0.25],
                    [x1 + 0.05, 0.03, z + h + 0.25],
                    0.01,
                );
                for x in [x0 + 0.05, x1 - 0.05] {
                    b.cyl("metal", [x, 0.012, z + h], [x, 0.03, z + h + 0.25], 0.003);
                }
            }
        }
    }
}

/// Turns a piece made by swapping `y` and `z` (a reflection) back to
/// facing outwards: its triangles reversed.
pub(super) fn mirror_fix(b: &mut Builder, start: usize) {
    let start = start as u32;
    for tris in b.parts.values_mut() {
        for t in tris.iter_mut() {
            if t.iter().all(|i| *i >= start) {
                t.swap(1, 2);
            }
        }
    }
}

/// A disc of radius `r` standing on the wall (its face to `+y`), `t`
/// thick, centred at `c`.
pub(super) fn turned_disc_y(b: &mut Builder, role: &'static str, c: P3, r: f64, t: f64) {
    let start = b.mark();
    turned(
        b,
        role,
        [0.0; 3],
        &[(0.0, 0.0), (r, 0.0), (r, t), (0.0, t)],
        40,
        0.0,
    );
    // The vertical turned to `+y` (a rotation).
    b.transform_from(start, |q| [c[0] + q[0], c[1] + q[2], c[2] - q[1]]);
}

/// The stool of a dressing table of collection `ix`: `w` across, its
/// seat `h` high, centred on `(0, 0)`.
fn stool(b: &mut Builder, ix: usize, w: f64, h: f64) {
    let r = w / 2.0;
    let (x0, x1) = (-r, r);
    let soft = |b: &mut Builder, lo: P3, hi: P3, rad: f64, seed: u32| {
        let mut s = Soft::new(lo, hi, rad);
        s.crown = 0.012;
        s.side = 0.006;
        s.wobble = 0.001;
        s.welt = 0.004;
        s.seed = seed;
        s.add(b, "cushion");
    };
    match ix {
        0 | 2 => {
            let z0 = h - 0.08;
            for k in 0..4 {
                let a = PI / 4.0 + FRAC_PI_2 * k as f64;
                let (s, c) = a.sin_cos();
                let at = [r * 0.62 * c, r * 0.62 * s];
                if ix == 2 {
                    compass_leg(b, at, z0 + 0.005, [c, s], 0.03);
                } else {
                    tapered_leg(b, "frame", at, z0 + 0.005, [c, s], 0.02, 0.016);
                }
            }
            turned(
                b,
                "frame",
                [0.0, 0.0, z0 - 0.02],
                &[(0.0, 0.0), (r * 0.8, 0.0), (r * 0.8, 0.025), (0.0, 0.025)],
                32,
                0.0,
            );
            // A round cushion.
            let n = 12;
            let mut prof = vec![(0.0, 0.0)];
            for i in 0..=n {
                let t = PI * i as f64 / n as f64;
                prof.push((r - 0.03 + 0.03 * t.sin(), 0.04 - 0.04 * t.cos()));
            }
            prof.push((0.0, 0.08));
            turned(b, "cushion", [0.0, 0.0, z0], &prof, 32, 0.0);
            b.soft.insert("cushion");
        }
        1 => {
            let start = b.mark();
            let s = 0.035;
            square_legs(b, "frame", x0, x1, w, h, s);
            for k in [0.0, w - s] {
                b.rbox(
                    "frame",
                    [x0 + s, k, h - 0.04],
                    [x1 - s, k + s, h - 0.01],
                    0.0,
                );
                b.rbox(
                    "frame",
                    [x0 + s, k + 0.005, 0.12],
                    [x1 - s, k + s - 0.005, 0.14],
                    0.0,
                );
            }
            let mut rush = Soft::new(
                [x0 + s * 0.6, s * 0.6, h - 0.03],
                [x1 - s * 0.6, w - s * 0.6, h + 0.004],
                0.01,
            );
            rush.crown = 0.004;
            rush.sag = 0.005;
            rush.seed = 42;
            rush.add_faces(b, "weave", [true; 6]);
            b.transform_from(start, |q| [q[0], q[1] - r, q[2]]);
        }
        3 => {
            b.rbox("frame", [x0, -r * 0.6, h - 0.03], [x1, r * 0.6, h], 0.006);
            for x in [x0 + 0.03, x1 - 0.06] {
                b.rbox(
                    "frame",
                    [x, -r * 0.5, 0.0],
                    [x + 0.03, r * 0.5, h - 0.03],
                    0.0,
                );
            }
        }
        4 => {
            b.rbox(
                "dark",
                [x0 + 0.03, -r + 0.03, 0.0],
                [x1 - 0.03, r - 0.03, 0.03],
                0.0,
            );
            soft(b, [x0, -r, 0.03], [x1, r, h], 0.04, 41);
        }
        5 => {
            turned(
                b,
                "frame",
                [0.0, 0.0, h - 0.04],
                &[(0.0, 0.0), (r * 0.85, 0.0), (r * 0.85, 0.04), (0.0, 0.04)],
                24,
                0.0,
            );
            for k in 0..3 {
                let a = TAU * k as f64 / 3.0;
                let (s, c) = a.sin_cos();
                let top = [r * 0.5 * c, r * 0.5 * s, h - 0.04];
                let foot = [r * 0.75 * c, r * 0.75 * s, 0.0025];
                b.cone("frame", foot, top, 0.017, 0.02, 10);
            }
        }
        6 => {
            let t = 0.022;
            for q in corners(x0, x1, w, t / 2.0) {
                bar(
                    b,
                    "metal",
                    [q[0], q[1] - r, 0.0],
                    [q[0], q[1] - r, h - 0.03],
                    t,
                    [1.0, 0.0, 0.0],
                );
            }
            for zz in [0.15] {
                for (a, c) in [
                    ([x0 + t / 2.0, -r + t / 2.0], [x1 - t / 2.0, -r + t / 2.0]),
                    ([x0 + t / 2.0, r - t / 2.0], [x1 - t / 2.0, r - t / 2.0]),
                ] {
                    bar(
                        b,
                        "metal",
                        [a[0], a[1], zz],
                        [c[0], c[1], zz],
                        t,
                        [0.0, 0.0, 1.0],
                    );
                }
            }
            b.rbox(
                "frame",
                [x0 - 0.005, -r - 0.005, h - 0.03],
                [x1 + 0.005, r + 0.005, h],
                0.0,
            );
        }
        _ => {
            let z0 = h - 0.1;
            for q in corners(x0, x1, w, 0.03) {
                louis_leg(b, "frame", [q[0], q[1] - r], z0 + 0.04, 0.02);
            }
            b.rbox(
                "frame",
                [x0 + 0.01, -r + 0.01, z0 - 0.02],
                [x1 - 0.01, r - 0.01, z0 + 0.04],
                0.006,
            );
            let mut s = Soft::new(
                [x0 - 0.01, -r - 0.01, z0 + 0.04],
                [x1 + 0.01, r + 0.01, h],
                0.03,
            );
            s.crown = 0.016;
            s.tufts = 0.12;
            s.tuft_depth = 0.01;
            s.step = 0.03;
            s.seed = 43;
            s.add(b, "cushion");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fine::shape::signed_volume;

    fn params(style: i32, values: &[(&str, f64)]) -> Params {
        let mut p = Params::default();
        p.set("style", style as f64);
        for (k, v) in values {
            p.set(k, *v);
        }
        p
    }

    /// Every piece of every collection is made, closed outwards and light.
    #[test]
    fn the_bedrooms_of_the_eight_collections() {
        type Make = fn(&mut Builder, &Params) -> bool;
        let both = |b: &mut Builder, p: &Params| {
            bed_bench(b, p);
            true
        };
        let table = |b: &mut Builder, p: &Params| {
            dressing_table(b, p);
            true
        };
        type Values = &'static [(&'static str, f64)];
        let pieces: [(&str, Make, i32, Values); 6] = [
            ("cama", bed, 5, &[("spread", 1.0), ("cushions", 2.0)]),
            ("mesita", nightstand, 4, &[]),
            ("armario", wardrobe, 1, &[("drawers", 1.0), ("doors", 3.0)]),
            ("cómoda", chest, 2, &[("columns", 2.0)]),
            ("banco", both, 0, &[]),
            ("tocador", table, 0, &[]),
        ];
        for c in 0..8 {
            for (name, make, first, values) in pieces {
                let mut b = Builder::default();
                assert!(make(&mut b, &params(first + 1 + c, values)), "{name}");
                let tris = b.triangle_count();
                assert!(
                    (50..40_000).contains(&tris),
                    "{} {name}: {tris}",
                    COLLECTIONS[c as usize]
                );
                for (role, t) in &b.parts {
                    let v = signed_volume(&b.positions, t);
                    assert!(v > -1e-7, "{} {name} {role}: {v}", COLLECTIONS[c as usize]);
                }
                for p in &b.positions {
                    assert!(
                        p.iter().all(|v| v.is_finite()),
                        "{} {name}",
                        COLLECTIONS[c as usize]
                    );
                    assert!(p[2] > -0.001, "{} {name}: {p:?}", COLLECTIONS[c as usize]);
                }
            }
        }
        let mut b = Builder::default();
        assert!(!bed(&mut b, &params(5, &[])));
        assert!(!wardrobe(&mut b, &params(1, &[])));
    }
}
