//! The workshop's sofas, one for each collection (paquete 1a): the
//! Nordic one of `super::sofa` and seven more. All share the same values
//! (`width`, `depth`, `seat`, `height`, `places`, `pillows`) and lie as
//! the library's sofa: front towards `+y`, back along `y = 0`, centred on
//! `x = 0`.
//!
//! Roles: `mat` the upholstered body, `cushion` the loose cushions,
//! `frame` wood, `metal` steel or brass, `pillow` the scatter cushions.

use super::shape::{bar, turned};
use super::soft::{knife_pillow, Soft};
use crate::build::{Builder, P3};
use crate::Params;

/// The sizes every sofa reads.
pub struct Sizes {
    pub w: f64,
    pub d: f64,
    pub seat: f64,
    pub h: f64,
    pub places: usize,
    pub pillows: usize,
}

impl Sizes {
    pub fn of(p: &Params) -> Self {
        let seat = p.get("seat", 0.43).max(0.3);
        Sizes {
            w: p.get("width", 2.2).max(0.8),
            d: p.get("depth", 0.92).max(0.6),
            seat,
            h: p.get("height", 0.82).max(seat + 0.15),
            places: p.get("places", 3.0).round().clamp(1.0, 6.0) as usize,
            pillows: p.get("pillows", 2.0).round().clamp(0.0, 4.0) as usize,
        }
    }
}

/// A piece of upholstery with the usual fabric: a little swell, a little
/// irregularity.
pub(super) fn upholstered(lo: P3, hi: P3, r: f64, seed: u32) -> Soft {
    let mut s = Soft::new(lo, hi, r);
    s.crown = 0.008;
    s.side = 0.006;
    s.wobble = 0.0012;
    s.seed = seed;
    s
}

/// Seat cushions, one per place, from `x0` to `x1`, `y0` to `y1`, `z0` to
/// `z1`: stuffed, sat on, with or without piping.
#[allow(clippy::too_many_arguments)]
pub(super) fn seat_cushions(
    b: &mut Builder,
    places: usize,
    x0: f64,
    x1: f64,
    y0: f64,
    y1: f64,
    z0: f64,
    z1: f64,
    welt: f64,
    round: f64,
) {
    let pw = (x1 - x0) / places as f64;
    let gap = 0.004;
    for i in 0..places {
        let xa = x0 + i as f64 * pw + gap;
        let mut s = Soft::new([xa, y0, z0], [xa + pw - 2.0 * gap, y1, z1], round);
        s.crown = 0.022;
        s.side = 0.012;
        s.sag = 0.026;
        s.creases = 0.007;
        s.wobble = 0.002;
        s.welt = welt;
        s.seed = 10 + i as u32;
        s.add(b, "cushion");
    }
}

/// Back cushions, one per place, standing on `z0` against a back at
/// `back` (y), up to `z1`, `thick`, leant back `tilt` degrees.
#[allow(clippy::too_many_arguments)]
pub(super) fn back_cushions(
    b: &mut Builder,
    places: usize,
    x0: f64,
    x1: f64,
    back: f64,
    z0: f64,
    z1: f64,
    thick: f64,
    tilt: f64,
    welt: f64,
) {
    let pw = (x1 - x0) / places as f64;
    let gap = 0.004;
    for i in 0..places {
        let xa = x0 + i as f64 * pw + gap;
        let start = b.mark();
        let mut k = Soft::new(
            [xa, back, z0],
            [xa + pw - 2.0 * gap, back + thick, z1],
            (thick * 0.32).min(0.07),
        );
        k.crown = thick * 0.25;
        k.side = 0.014;
        k.creases = 0.009;
        k.wobble = 0.0028;
        k.welt = welt;
        k.seed = 20 + i as u32;
        k.add(b, "cushion");
        let (st, ct) = tilt.to_radians().sin_cos();
        b.transform_from(start, |q| {
            let (dy, dz) = (q[1] - back, q[2] - z0);
            [q[0], back + dy * ct - dz * st, z0 + dy * st + dz * ct]
        });
    }
}

/// Scatter cushions leaning on the back, at the ends first.
fn scatter(b: &mut Builder, s: &Sizes, x_in: f64, y: f64, z: f64) {
    let inner = 2.0 * x_in;
    let size = (inner / s.places as f64 * 0.75).clamp(0.38, 0.48);
    let at_end = -x_in + size * 0.62;
    let spots: Vec<f64> = match s.pillows {
        0 => vec![],
        1 => vec![at_end],
        2 => vec![at_end, -at_end],
        3 => vec![at_end, at_end + size * 0.85, -at_end],
        _ => vec![at_end, at_end + size * 0.85, -at_end - size * 0.85, -at_end],
    };
    for (i, x) in spots.into_iter().enumerate() {
        let yaw: f64 = [-0.16, 0.08, -0.06, 0.15][i];
        let lean = (62f64 + 4.0 * i as f64).to_radians();
        let sz = if i == 1 || i == 2 { size * 0.9 } else { size };
        let (sy, cy) = yaw.sin_cos();
        let (sl, cl) = lean.sin_cos();
        knife_pillow(
            b,
            "pillow",
            [x, y, z + sz * 0.46],
            [cy, sy, 0.0],
            [-sy * sl, cy * sl, cl],
            sz,
            sz,
            0.2,
            40 + i as u32,
        );
    }
}

/// A tapered wooden leg splayed out along `dir` (compass legs), from its
/// top at `at`, `top` high, with a brass shoe.
pub(super) fn compass_leg(b: &mut Builder, at: [f64; 2], top: f64, dir: [f64; 2], splay: f64) {
    let start = b.mark();
    turned(
        b,
        "frame",
        [0.0; 3],
        &[(0.0, 0.025), (0.011, 0.025), (0.019, top), (0.0, top)],
        16,
        0.0,
    );
    turned(
        b,
        "metal",
        [0.0; 3],
        &[(0.0, 0.0), (0.0105, 0.0), (0.0118, 0.03), (0.0, 0.03)],
        16,
        0.0,
    );
    let k = splay / top.max(1e-3);
    b.transform_from(start, |q| {
        let down = top - q[2];
        [
            at[0] + q[0] + dir[0] * k * down,
            at[1] + q[1] + dir[1] * k * down,
            q[2],
        ]
    });
}

/// A round turned foot (a bun) under a classic piece.
pub(super) fn bun_foot(b: &mut Builder, at: [f64; 2], top: f64) {
    turned(
        b,
        "frame",
        [at[0], at[1], 0.0],
        &[
            (0.0, 0.0),
            (0.022, 0.0),
            (0.03, top * 0.25),
            (0.034, top * 0.5),
            (0.03, top * 0.72),
            (0.024, top * 0.85),
            (0.03, top * 0.92),
            (0.03, top),
            (0.0, top),
        ],
        20,
        0.0,
    );
}

/// Mediterranean: a whitewashed wooden frame, slatted at the arms and the
/// back, carrying a tied linen mattress and loose back cushions.
pub fn mediterranean(b: &mut Builder, p: &Params) {
    let s = Sizes::of(p);
    let x0 = -s.w / 2.0;
    let post = 0.06;
    let deck = s.seat - 0.13;
    let arm_h = s.seat + 0.2;
    // Feet and the deck the mattress lies on.
    for (x, y) in [
        (x0 + 0.02, 0.02),
        (-x0 - 0.02 - post, 0.02),
        (x0 + 0.02, s.d - 0.02 - post),
        (-x0 - 0.02 - post, s.d - 0.02 - post),
    ] {
        b.rbox("frame", [x, y, 0.0], [x + post, y + post, 0.11], 0.0);
    }
    b.rbox("frame", [x0, 0.0, 0.1], [-x0, s.d, deck], 0.008);
    // The arms: a front and a back post, a top rail, three slats.
    for side in [-1.0, 1.0] {
        let xa = if side < 0.0 { x0 } else { -x0 - post };
        let (xm, xn) = (xa, xa + post);
        b.rbox("frame", [xm, 0.0, deck - 0.01], [xn, post, s.h], 0.006);
        b.rbox(
            "frame",
            [xm, s.d - post, deck - 0.01],
            [xn, s.d, arm_h],
            0.006,
        );
        b.rbox(
            "frame",
            [xm - 0.006, 0.0, arm_h - 0.035],
            [xn + 0.006, s.d + 0.01, arm_h + 0.004],
            0.01,
        );
        for k in 1..=3 {
            let y = s.d * k as f64 / 4.0;
            b.rbox(
                "frame",
                [xm + 0.012, y - 0.022, deck - 0.01],
                [xn - 0.012, y + 0.022, arm_h - 0.03],
                0.0,
            );
        }
    }
    // The back: a top rail and slats between the back posts.
    let (bx0, bx1) = (x0 + post, -x0 - post);
    b.rbox("frame", [x0, 0.0, s.h - 0.05], [-x0, post, s.h], 0.008);
    let slats = ((bx1 - bx0) / 0.16).round().max(3.0) as usize;
    for k in 0..slats {
        let x = bx0 + (bx1 - bx0) * (k as f64 + 0.5) / slats as f64;
        b.rbox(
            "frame",
            [x - 0.03, 0.008, deck],
            [x + 0.03, 0.03, s.h - 0.045],
            0.0,
        );
    }
    // The mattress, one long piece tied with a few buttons, piped.
    let mut m = Soft::new(
        [bx0 + 0.004, 0.035, deck],
        [bx1 - 0.004, s.d + 0.01, s.seat],
        0.05,
    );
    m.crown = 0.02;
    m.side = 0.012;
    m.sag = 0.02;
    m.creases = 0.006;
    m.wobble = 0.0025;
    m.welt = 0.006;
    m.tufts = 0.3;
    m.tuft_depth = 0.012;
    m.step = 0.08;
    m.seed = 5;
    m.add(b, "cushion");
    back_cushions(
        b,
        s.places,
        bx0,
        bx1,
        0.035,
        s.seat - 0.03,
        s.h + 0.04,
        0.18,
        14.0,
        0.0,
    );
    scatter(b, &s, -bx0, 0.3, s.seat);
}

/// Fifties: slim upholstered arms, one long bench seat, a buttoned back
/// leant a little, on splayed tapered legs with brass shoes.
pub fn fifties(b: &mut Builder, p: &Params) {
    let s = Sizes::of(p);
    let x0 = -s.w / 2.0;
    let arm = 0.075;
    let foot = 0.2;
    let seat_t = 0.12;
    let base_top = s.seat - seat_t;
    let arm_h = s.seat + 0.17;
    let inset = 0.08;
    for (x, y, dx, dy) in [
        (x0 + inset, inset, -1.0f64, -1.0f64),
        (-x0 - inset, inset, 1.0, -1.0),
        (x0 + inset, s.d - inset, -1.0, 1.0),
        (-x0 - inset, s.d - inset, 1.0, 1.0),
    ] {
        let l = (dx * dx + dy * dy).sqrt();
        compass_leg(b, [x, y], foot + 0.005, [dx / l, dy / l], 0.05);
    }
    // The slim base, the arms and the buttoned back.
    let mut base = upholstered([x0, 0.04, foot], [-x0, s.d, base_top], 0.025, 1);
    base.welt = 0.004;
    base.add(b, "mat");
    for (i, (a0, a1)) in [(x0, x0 + arm), (-x0 - arm, -x0)].into_iter().enumerate() {
        let mut a = upholstered([a0, 0.02, foot], [a1, s.d, arm_h], 0.034, 3 + i as u32);
        a.side = 0.004;
        a.add(b, "mat");
    }
    let start = b.mark();
    let mut back = upholstered(
        [x0 + arm - 0.01, 0.0, base_top - 0.02],
        [-x0 - arm + 0.01, 0.13, s.h],
        0.045,
        2,
    );
    back.crown = 0.018;
    back.welt = 0.005;
    back.tufts = 0.24;
    back.tuft_depth = 0.014;
    back.tuft_faces = [false, true];
    back.step = 0.04;
    back.add(b, "mat");
    let (st, ct) = 9f64.to_radians().sin_cos();
    let z0 = base_top - 0.02;
    b.transform_from(start, |q| {
        let (dy, dz) = (q[1], q[2] - z0);
        [q[0], dy * ct - dz * st + 0.03, z0 + dy * st + dz * ct]
    });
    // One long seat, piped.
    let mut seat = Soft::new(
        [x0 + arm, 0.14, base_top],
        [-x0 - arm, s.d + 0.01, s.seat],
        0.04,
    );
    seat.crown = 0.016;
    seat.side = 0.01;
    seat.sag = 0.018;
    seat.creases = 0.005;
    seat.wobble = 0.0018;
    seat.welt = 0.006;
    seat.seed = 9;
    seat.add(b, "cushion");
    scatter(b, &s, -x0 - arm, 0.3, s.seat);
}

/// Japandi: low, on a recessed wooden plinth, wooden boards for arms and
/// back, thick plain cushions.
pub fn japandi(b: &mut Builder, p: &Params) {
    let s = Sizes::of(p);
    let seat = s.seat - 0.04;
    let x0 = -s.w / 2.0;
    let board = 0.035;
    let deck = 0.2;
    b.rbox(
        "frame",
        [x0 + 0.05, 0.05, 0.0],
        [-x0 - 0.05, s.d - 0.05, 0.08],
        0.004,
    );
    b.rbox("frame", [x0, 0.0, 0.08], [-x0, s.d, deck], 0.01);
    for xa in [x0, -x0 - board] {
        b.rbox(
            "frame",
            [xa, 0.0, deck - 0.005],
            [xa + board, s.d, seat + 0.12],
            0.012,
        );
    }
    b.rbox(
        "frame",
        [x0 + board - 0.005, 0.0, deck - 0.005],
        [-x0 - board + 0.005, board, seat + 0.2],
        0.012,
    );
    let (xi0, xi1) = (x0 + board, -x0 - board);
    seat_cushions(b, s.places, xi0, xi1, board, s.d, deck, seat, 0.0, 0.05);
    back_cushions(
        b,
        s.places,
        xi0,
        xi1,
        board,
        seat - 0.03,
        s.h - 0.06,
        0.2,
        11.0,
        0.0,
    );
    scatter(b, &s, -xi0, 0.3, seat);
}

/// Contemporary: wide block arms, a recessed dark plinth, deep plain seat
/// cushions and thick back cushions.
pub fn contemporary(b: &mut Builder, p: &Params) {
    let s = Sizes::of(p);
    let x0 = -s.w / 2.0;
    let arm = (s.w * 0.11).clamp(0.18, 0.26);
    let plinth = 0.06;
    let cushion = 0.15;
    let base_top = s.seat - cushion;
    let arm_h = s.seat + 0.19;
    b.rbox(
        "metal",
        [x0 + 0.05, 0.05, 0.0],
        [-x0 - 0.05, s.d - 0.05, plinth + 0.005],
        0.004,
    );
    let mut base = upholstered(
        [x0 + arm - 0.02, 0.04, plinth],
        [-x0 - arm + 0.02, s.d, base_top],
        0.02,
        1,
    );
    base.crown = 0.003;
    base.add(b, "mat");
    for (i, (a0, a1)) in [(x0, x0 + arm), (-x0 - arm, -x0)].into_iter().enumerate() {
        let mut a = upholstered([a0, 0.0, plinth], [a1, s.d, arm_h], 0.035, 3 + i as u32);
        a.crown = 0.006;
        a.side = 0.01;
        a.creases = 0.003;
        a.add(b, "mat");
    }
    let back = upholstered(
        [x0 + arm - 0.02, 0.0, plinth],
        [-x0 - arm + 0.02, 0.17, s.h - 0.14],
        0.035,
        2,
    );
    back.add(b, "mat");
    let (xi0, xi1) = (x0 + arm, -x0 - arm);
    seat_cushions(
        b,
        s.places,
        xi0,
        xi1,
        0.17,
        s.d + 0.01,
        base_top,
        s.seat,
        0.0,
        0.035,
    );
    back_cushions(
        b,
        s.places,
        xi0,
        xi1,
        0.17,
        s.seat - 0.03,
        s.h + 0.02,
        0.22,
        10.0,
        0.0,
    );
    // Ahead of the back cushions, which bulge forwards.
    scatter(b, &s, -xi0, 0.47, s.seat);
}

/// Rustic: a frame of solid wood, square posts and thick rails, slats in
/// the arms, thick linen cushions.
pub fn rustic(b: &mut Builder, p: &Params) {
    let s = Sizes::of(p);
    let x0 = -s.w / 2.0;
    let post = 0.085;
    let arm_h = s.seat + 0.2;
    let deck = s.seat - 0.15;
    for xa in [x0, -x0 - post] {
        let (xm, xn) = (xa, xa + post);
        // Front post to the arm, back post up to the back rail.
        b.rbox(
            "frame",
            [xm, s.d - post, 0.0],
            [xn, s.d, arm_h - 0.02],
            0.01,
        );
        b.rbox("frame", [xm, 0.0, 0.0], [xn, post, s.h - 0.02], 0.01);
        // The arm rail, a little wider, and the low rail.
        b.rbox(
            "frame",
            [xm - 0.015, -0.01, arm_h - 0.05],
            [xn + 0.015, s.d + 0.025, arm_h],
            0.014,
        );
        b.rbox(
            "frame",
            [xm + 0.01, post, 0.12],
            [xn - 0.01, s.d - post, 0.2],
            0.008,
        );
        for k in 1..=2 {
            let y = s.d * k as f64 / 3.0;
            b.rbox(
                "frame",
                [xm + 0.018, y - 0.03, 0.2],
                [xn - 0.018, y + 0.03, arm_h - 0.05],
                0.0,
            );
        }
    }
    // The rails front and back, the back rail, the deck.
    b.rbox(
        "frame",
        [x0 + post, s.d - 0.07, 0.12],
        [-x0 - post, s.d - 0.01, deck],
        0.008,
    );
    b.rbox(
        "frame",
        [x0 + post, 0.0, 0.12],
        [-x0 - post, 0.06, deck],
        0.008,
    );
    b.rbox(
        "frame",
        [x0 + post, 0.0, s.h - 0.12],
        [-x0 - post, 0.05, s.h - 0.02],
        0.01,
    );
    b.rbox(
        "frame",
        [x0 + post, 0.05, deck - 0.03],
        [-x0 - post, s.d - 0.07, deck],
        0.0,
    );
    let (xi0, xi1) = (x0 + post, -x0 - post);
    seat_cushions(b, s.places, xi0, xi1, 0.05, s.d, deck, s.seat, 0.006, 0.05);
    back_cushions(
        b,
        s.places,
        xi0,
        xi1,
        0.05,
        s.seat - 0.03,
        s.h + 0.02,
        0.2,
        14.0,
        0.006,
    );
    scatter(b, &s, -xi0, 0.32, s.seat);
}

/// Industrial: a side frame of black steel at each end, leather on a
/// leather base, padded arm rests on the steel.
pub fn industrial(b: &mut Builder, p: &Params) {
    let s = Sizes::of(p);
    let x0 = -s.w / 2.0;
    let rod = 0.028;
    let arm_h = s.seat + 0.19;
    let base_z = 0.16;
    let base_top = s.seat - 0.14;
    for xa in [x0 + rod / 2.0, -x0 - rod / 2.0] {
        let along = [0.0, 1.0, 0.0];
        // A closed loop of square tube: runner, front post, arm rail,
        // back post.
        let lo_f = [xa, s.d - rod / 2.0, rod / 2.0];
        let lo_b = [xa, rod / 2.0, rod / 2.0];
        let hi_f = [xa, s.d - rod / 2.0, arm_h];
        let hi_b = [xa, rod / 2.0, arm_h];
        bar(
            b,
            "metal",
            [xa, -0.005, rod / 2.0],
            [xa, s.d + 0.005, rod / 2.0],
            rod,
            [1.0, 0.0, 0.0],
        );
        bar(
            b,
            "metal",
            [xa, -0.005, arm_h],
            [xa, s.d + 0.005, arm_h],
            rod,
            [1.0, 0.0, 0.0],
        );
        bar(b, "metal", lo_f, hi_f, rod, along);
        bar(b, "metal", lo_b, hi_b, rod, along);
        // The arm rest on the rail.
        let mut pad = upholstered(
            [xa - 0.05, 0.01, arm_h + rod / 2.0 - 0.003],
            [xa + 0.05, s.d - 0.01, arm_h + rod / 2.0 + 0.055],
            0.025,
            7,
        );
        pad.welt = 0.004;
        pad.add(b, "mat");
    }
    // Cross rails under the seat, front and back.
    for y in [0.04, s.d - 0.04] {
        bar(
            b,
            "metal",
            [x0, y, base_z - 0.02],
            [-x0, y, base_z - 0.02],
            rod,
            [0.0, 1.0, 0.0],
        );
    }
    let (xi0, xi1) = (x0 + rod + 0.004, -x0 - rod - 0.004);
    let mut base = upholstered(
        [xi0, 0.02, base_z - 0.006],
        [xi1, s.d - 0.02, base_top],
        0.02,
        1,
    );
    base.welt = 0.004;
    base.add(b, "mat");
    let back = upholstered([xi0, 0.0, base_z], [xi1, 0.12, s.h - 0.16], 0.03, 2);
    back.add(b, "mat");
    seat_cushions(
        b,
        s.places,
        xi0,
        xi1,
        0.12,
        s.d + 0.005,
        base_top,
        s.seat,
        0.005,
        0.04,
    );
    back_cushions(
        b,
        s.places,
        xi0,
        xi1,
        0.12,
        s.seat - 0.03,
        s.h + 0.02,
        0.18,
        12.0,
        0.005,
    );
    scatter(b, &s, -xi0, 0.32, s.seat);
}

/// Classic: buttoned all over, arms as high as the back and rolled over,
/// piped seat cushions, on turned bun feet.
pub fn classic(b: &mut Builder, p: &Params) {
    let s = Sizes::of(p);
    let x0 = -s.w / 2.0;
    let arm = (s.w * 0.1).clamp(0.17, 0.24);
    let foot = 0.09;
    let top = s.h - 0.06;
    let cushion = 0.13;
    let base_top = s.seat - cushion;
    for (x, y) in [
        (x0 + 0.06, 0.06),
        (-x0 - 0.06, 0.06),
        (x0 + 0.06, s.d - 0.06),
        (-x0 - 0.06, s.d - 0.06),
    ] {
        bun_foot(b, [x, y], foot + 0.004);
    }
    let mut base = upholstered(
        [x0 + arm - 0.02, 0.05, foot],
        [-x0 - arm + 0.02, s.d, base_top],
        0.03,
        1,
    );
    base.welt = 0.005;
    base.add(b, "mat");
    // Arms rolled over: a big radius on the top, buttons inside and out.
    for (i, (a0, a1)) in [(x0, x0 + arm), (-x0 - arm, -x0)].into_iter().enumerate() {
        let mut a = upholstered(
            [a0, 0.0, foot],
            [a1, s.d + 0.02, top],
            arm * 0.48,
            3 + i as u32,
        );
        a.crown = 0.012;
        a.side = 0.01;
        a.tufts = 0.11;
        a.tuft_depth = 0.03;
        // Buttoned inside only.
        a.tuft_faces = if i == 0 { [false, true] } else { [true, false] };
        a.step = 0.026;
        a.add(b, "mat");
    }
    let mut back = upholstered(
        [x0 + arm - 0.03, 0.0, foot],
        [-x0 - arm + 0.03, 0.2, top],
        0.07,
        2,
    );
    back.crown = 0.02;
    back.tufts = 0.11;
    back.tuft_depth = 0.034;
    back.tuft_faces = [false, true];
    back.step = 0.026;
    back.add(b, "mat");
    let (xi0, xi1) = (x0 + arm, -x0 - arm);
    seat_cushions(
        b,
        s.places,
        xi0,
        xi1,
        0.2,
        s.d + 0.015,
        base_top,
        s.seat,
        0.006,
        0.04,
    );
    scatter(b, &s, -xi0, 0.34, s.seat);
}

#[cfg(test)]
mod tests {
    use super::super::shape::signed_volume;

    /// Every sofa of the collections is made, closed and within its
    /// weight, at two and four places.
    #[test]
    fn the_collection_sofas_are_made() {
        for style in 1..=8 {
            for places in [2.0, 4.0] {
                let mut p = crate::Params::default();
                p.set("style", style as f64);
                p.set("places", places);
                p.set("width", places * 0.65 + 0.45);
                let m = crate::make("fine_sofa", &p).unwrap();
                let tris = m.triangle_count();
                assert!((4_000..40_000).contains(&tris), "{style}: {tris}");
                let v: f64 = m
                    .parts
                    .iter()
                    .map(|(_, t)| signed_volume(&m.positions, t))
                    .sum();
                assert!(v > 0.2, "{style}: {v}");
            }
        }
    }
}
