//! The workshop's armchairs, one for each collection (paquete 1a).
//!
//! Values: `width`, `depth`, `seat`, `height` (of the back), `pillows`
//! (0 or 1). Front towards `+y`, back along `y = 0`, centred on `x = 0`.
//! The contemporary, Japandi and industrial ones are their sofas at one
//! place; the others are their own shapes: a round tub (Nordic), a rattan
//! frame (Mediterranean), a shell on compass legs (fifties), a friar's
//! chair of wood and leather (rustic) and a wingback (classic).
//!
//! Roles as the sofas', and `weave` for rattan.

use super::shape::{curve, sweep, turned, Cap};
use super::sofas::{back_cushions, bun_foot, compass_leg, seat_cushions, upholstered, Sizes};
use super::soft::{knife_pillow, smooth, Soft};
use crate::build::{Builder, P3};
use crate::Params;
use std::f64::consts::{PI, TAU};

/// The sizes, with an armchair's defaults.
fn sizes(p: &Params) -> Sizes {
    let mut q = p.clone();
    for (k, v) in [
        ("width", 0.82),
        ("depth", 0.82),
        ("height", 0.8),
        ("pillows", 1.0),
    ] {
        if p.get(k, f64::NAN).is_nan() {
            q.set(k, v);
        }
    }
    q.set("places", 1.0);
    let mut s = Sizes::of(&q);
    s.pillows = s.pillows.min(1);
    s
}

/// One scatter cushion in the middle, leaning on the back.
fn pillow(b: &mut Builder, s: &Sizes, y: f64, z: f64) {
    if s.pillows == 0 {
        return;
    }
    let size = (s.w * 0.5).clamp(0.34, 0.44);
    leaning_pillow(b, size, y, z, 64.0, -0.1);
}

/// A scatter cushion `size` square, its middle at `y`, sitting on `z`,
/// leaning back `lean` degrees from flat and turned `yaw` radians.
fn leaning_pillow(b: &mut Builder, size: f64, y: f64, z: f64, lean: f64, yaw: f64) {
    let (sl, cl) = lean.to_radians().sin_cos();
    let (sy, cy) = yaw.sin_cos();
    knife_pillow(
        b,
        "pillow",
        [0.0, y, z + size * 0.46],
        [cy, sy, 0.0],
        [-sy * sl, cy * sl, cl],
        size,
        size,
        0.19,
        44,
    );
}

/// Bends what was made since `start`, a straight slab along `x` with its
/// outer face on `y = 0`, round the vertical through `c`: `x` becomes the
/// length along the circle of radius `r` (0 at the back), `y` the depth
/// in from it; the top comes down towards the front by `drop`.
pub(super) fn wrap(
    b: &mut Builder,
    start: usize,
    c: [f64; 2],
    r: f64,
    span: f64,
    z0: f64,
    drop: f64,
) {
    b.transform_from(start, |q| {
        let a = q[0] / r;
        let rr = r - q[1];
        let k = smooth(0.15, 1.0, a.abs() / span);
        let z = z0 + (q[2] - z0) * (1.0 - drop * k);
        [c[0] + rr * a.sin(), c[1] - rr * a.cos(), z]
    });
}

/// Nordic: a round tub, its back wrapping round and coming down into the
/// arms, a round seat cushion, on four splayed legs.
pub fn nordic(b: &mut Builder, p: &Params) {
    let s = sizes(p);
    let r = (s.w / 2.0).min(s.d / 2.0);
    let c = [0.0, r];
    let foot = 0.17;
    let seat_t = 0.12;
    let base_top = s.seat - seat_t;
    for k in 0..4 {
        let a = PI / 4.0 + k as f64 * PI / 2.0;
        let (sa, ca) = a.sin_cos();
        compass_leg(
            b,
            [c[0] + ca * r * 0.62, c[1] + sa * r * 0.62],
            foot + 0.005,
            [ca, sa],
            0.035,
        );
    }
    // The base: a round drum.
    turned(
        b,
        "mat",
        [c[0], c[1], foot],
        &[
            (0.0, 0.0),
            (r * 0.86, 0.0),
            (r * 0.9, 0.015),
            (r * 0.9, base_top - foot - 0.015),
            (r * 0.86, base_top - foot),
            (0.0, base_top - foot),
        ],
        40,
        0.0,
    );
    // The tub: a slab bent round, 240° of it.
    let span = 2.1;
    let start = b.mark();
    let mut tub = Soft::new([-r * span, 0.0, foot + 0.02], [r * span, 0.13, s.h], 0.05);
    tub.crown = 0.012;
    tub.side = 0.008;
    tub.creases = 0.004;
    tub.wobble = 0.0012;
    tub.welt = 0.005;
    tub.step = 0.05;
    tub.seed = 3;
    tub.add(b, "mat");
    wrap(b, start, c, r, span, foot, 0.32);
    // The round seat cushion.
    let ri = r - 0.12;
    let mut seat = Soft::new(
        [c[0] - ri, c[1] - ri, base_top],
        [c[0] + ri, c[1] + ri + 0.04, s.seat],
        ri * 0.9,
    );
    seat.crown = 0.018;
    seat.side = 0.01;
    seat.sag = 0.018;
    seat.wobble = 0.002;
    seat.welt = 0.005;
    seat.seed = 12;
    seat.add(b, "cushion");
    // The cushion stands nearly upright in the tub, its back corners
    // clear of the curved inside of the back.
    if s.pillows > 0 {
        let size = (ri * 1.2).clamp(0.3, 0.38);
        let lean = 76f64;
        let inside = r - 0.13;
        let half = size / 2.0;
        let corner = c[1] - (inside * inside - half * half).max(0.0).sqrt();
        let top_back = size * lean.to_radians().cos() * 0.5;
        let y = (corner + top_back).max(c[1] - inside + 0.095) + 0.015;
        leaning_pillow(b, size, y, s.seat, lean, 0.0);
    }
}

/// Mediterranean: a rattan frame — legs, seat ring, arms looping back and
/// a hooped back with spindles — and linen cushions.
pub fn mediterranean(b: &mut Builder, p: &Params) {
    let s = sizes(p);
    let x0 = -s.w / 2.0;
    let cane = 0.014;
    let arm_h = s.seat + 0.2;
    let ring_z = s.seat - 0.1;
    let tube = |b: &mut Builder, pts: &[P3], r: f64| {
        let path = curve(pts, 6);
        sweep(
            b,
            "weave",
            &path,
            |_| r,
            10,
            Cap::Round(1.0),
            Cap::Round(1.0),
        );
    };
    // Front posts up to the arms, back posts up to the back hoop.
    for x in [x0 + 0.05, -x0 - 0.05] {
        tube(
            b,
            &[[x, s.d - 0.06, 0.0], [x, s.d - 0.06, arm_h]],
            cane * 1.25,
        );
        tube(
            b,
            &[
                [x * 0.98, 0.07, 0.0],
                [x, 0.06, ring_z],
                [x * 0.97, 0.03, s.h - 0.06],
            ],
            cane * 1.25,
        );
        // The arm: from the front post, round and back to the back post.
        tube(
            b,
            &[
                [x, s.d - 0.06, arm_h - 0.01],
                [x * 1.02, s.d - 0.16, arm_h + 0.02],
                [x * 1.01, s.d * 0.45, arm_h + 0.03],
                [x * 0.98, 0.06, arm_h + 0.06],
            ],
            cane,
        );
        // A second cane under the arm, as rattan chairs have.
        tube(
            b,
            &[
                [x, s.d - 0.06, arm_h - 0.08],
                [x * 1.01, s.d * 0.45, arm_h - 0.06],
                [x * 0.99, 0.06, arm_h - 0.04],
            ],
            cane * 0.7,
        );
    }
    // The seat ring and its cross canes.
    let (xa, xb) = (x0 + 0.05, -x0 - 0.05);
    let ring = [
        [xa, s.d - 0.06, ring_z],
        [0.0, s.d - 0.045, ring_z],
        [xb, s.d - 0.06, ring_z],
        [xb * 0.99, s.d * 0.5, ring_z],
        [xb, 0.06, ring_z],
        [0.0, 0.05, ring_z],
        [xa, 0.06, ring_z],
        [xa * 0.99, s.d * 0.5, ring_z],
        [xa, s.d - 0.06, ring_z],
    ];
    tube(b, &ring, cane);
    for k in 1..=4 {
        let y = 0.06 + (s.d - 0.12) * k as f64 / 5.0;
        tube(
            b,
            &[[xa, y, ring_z - 0.01], [xb, y, ring_z - 0.01]],
            cane * 0.55,
        );
    }
    // The back: a hoop over the back posts and spindles down to the ring.
    let hoop = [
        [xa * 0.97, 0.03, s.h - 0.08],
        [xa * 0.6, 0.0, s.h - 0.01],
        [0.0, -0.01, s.h + 0.01],
        [xb * 0.6, 0.0, s.h - 0.01],
        [xb * 0.97, 0.03, s.h - 0.08],
    ];
    tube(b, &hoop, cane * 1.1);
    for k in 1..=5 {
        let x = xa + (xb - xa) * k as f64 / 6.0;
        let top = s.h - 0.02 - 0.06 * (x / xb).powi(2);
        tube(b, &[[x, 0.06, ring_z], [x, 0.02, top]], cane * 0.6);
    }
    // Cushions: the seat on the ring, one leaning on the spindles.
    seat_cushions(
        b,
        1,
        xa + 0.01,
        xb - 0.01,
        0.05,
        s.d - 0.03,
        ring_z + cane,
        s.seat + 0.02,
        0.006,
        0.05,
    );
    back_cushions(
        b,
        1,
        xa + 0.04,
        xb - 0.04,
        0.05,
        s.seat,
        s.h - 0.03,
        0.12,
        12.0,
        0.006,
    );
    pillow(b, &s, 0.24, s.seat + 0.02);
}

/// Fifties: an upholstered shell, thin and wide, on splayed compass legs,
/// with its seat cushion.
pub fn fifties(b: &mut Builder, p: &Params) {
    let s = sizes(p);
    let r = (s.w / 2.0).min(s.d / 2.0) * 1.08;
    let c = [0.0, r * 0.92];
    let foot = 0.22;
    let base_top = s.seat - 0.1;
    for (dx, dy) in [(-1.0f64, -1.0f64), (1.0, -1.0), (-1.0, 1.0), (1.0, 1.0)] {
        let l = (dx * dx + dy * dy).sqrt();
        compass_leg(
            b,
            [c[0] + dx * r * 0.45, c[1] + dy * r * 0.45],
            foot + 0.005,
            [dx / l, dy / l],
            0.06,
        );
    }
    let mut seat_base = upholstered(
        [c[0] - r * 0.78, c[1] - r * 0.7, foot],
        [c[0] + r * 0.78, c[1] + r * 0.8, base_top],
        0.03,
        1,
    );
    seat_base.welt = 0.004;
    seat_base.add(b, "mat");
    let span = 1.65;
    let start = b.mark();
    let mut shell = Soft::new([-r * span, 0.0, foot], [r * span, 0.06, s.h], 0.028);
    shell.crown = 0.006;
    shell.side = 0.004;
    shell.wobble = 0.001;
    shell.welt = 0.004;
    shell.step = 0.05;
    shell.seed = 4;
    shell.add(b, "mat");
    wrap(b, start, c, r, span, foot, 0.38);
    let ri = r * 0.74;
    let mut seat = Soft::new(
        [c[0] - ri, c[1] - r * 0.62, base_top],
        [c[0] + ri, c[1] + r * 0.82, s.seat],
        0.04,
    );
    seat.crown = 0.014;
    seat.side = 0.008;
    seat.sag = 0.016;
    seat.welt = 0.005;
    seat.wobble = 0.0015;
    seat.seed = 13;
    seat.add(b, "cushion");
    pillow(b, &s, c[1] - r * 0.55, s.seat);
}

/// Rustic: a Spanish friar's chair — square wooden legs, flat arms, a
/// wide carved stretcher in front, leather stretched over the seat and
/// the back, knobs on the back posts.
pub fn friar(b: &mut Builder, p: &Params) {
    let s = sizes(p);
    let w = s.w.min(0.72);
    let d = s.d.min(0.62);
    let h = s.h.max(1.0);
    let x0 = -w / 2.0;
    let leg = 0.05;
    let arm_h = s.seat + 0.22;
    // Back legs, a little raked, up to the knobs.
    for x in [x0, -x0 - leg] {
        let start = b.mark();
        b.rbox("frame", [x, 0.0, 0.0], [x + leg, leg, h], 0.006);
        let rake = 0.06;
        b.transform_from(start, |q| [q[0], q[1] - rake * (q[2] / h), q[2]]);
        turned(
            b,
            "frame",
            [x + leg / 2.0, leg / 2.0 - rake, h - 0.004],
            &[
                (0.0, 0.0),
                (0.024, 0.0),
                (0.03, 0.025),
                (0.024, 0.05),
                (0.008, 0.07),
                (0.0, 0.075),
            ],
            16,
            0.0,
        );
        // Front legs up to the arm.
        b.rbox("frame", [x, d - leg, 0.0], [x + leg, d, arm_h], 0.006);
        // The flat arm, from the back post to beyond the front leg.
        b.rbox(
            "frame",
            [x - 0.02, -0.03, arm_h],
            [x + leg + 0.02, d + 0.05, arm_h + 0.03],
            0.012,
        );
        // Side stretchers.
        b.rbox(
            "frame",
            [x + 0.008, leg, 0.1],
            [x + leg - 0.008, d - leg, 0.14],
            0.0,
        );
    }
    // The wide front stretcher, with a moulding along its top.
    let (xa, xb) = (x0 + leg, -x0 - leg);
    b.rbox("frame", [xa, d - 0.035, 0.1], [xb, d - 0.012, 0.24], 0.004);
    b.rbox("frame", [xa, d - 0.04, 0.22], [xb, d - 0.006, 0.245], 0.006);
    b.rbox("frame", [xa, 0.01, 0.1], [xb, 0.035, 0.14], 0.0);
    // Seat rails and the leather stretched over them, sagging a little.
    b.rbox(
        "frame",
        [xa, leg, s.seat - 0.05],
        [xb, d - leg, s.seat - 0.03],
        0.0,
    );
    let mut seat = Soft::new(
        [xa - 0.004, 0.02, s.seat - 0.035],
        [xb + 0.004, d + 0.005, s.seat],
        0.012,
    );
    seat.crown = -0.008;
    seat.wobble = 0.001;
    seat.seed = 31;
    seat.add(b, "mat");
    // The leather back between the back posts, high up, raked as they are.
    let start = b.mark();
    let mut back = Soft::new(
        [xa - 0.004, 0.012, s.seat + 0.18],
        [xb + 0.004, 0.032, h - 0.06],
        0.008,
    );
    back.crown = -0.006;
    back.wobble = 0.001;
    back.seed = 32;
    back.add(b, "mat");
    b.transform_from(start, |q| [q[0], q[1] - 0.06 * (q[2] / h), q[2]]);
}

/// Classic: a wingback — tall buttoned back, wings, rolled arms, a piped
/// seat cushion, on turned feet.
pub fn wingback(b: &mut Builder, p: &Params) {
    let s = sizes(p);
    let h = s.h.max(1.0);
    let x0 = -s.w / 2.0;
    let foot = 0.1;
    let arm = 0.13;
    let arm_h = s.seat + 0.2;
    let base_top = s.seat - 0.12;
    for (x, y) in [
        (x0 + 0.06, 0.06),
        (-x0 - 0.06, 0.06),
        (x0 + 0.06, s.d - 0.06),
        (-x0 - 0.06, s.d - 0.06),
    ] {
        bun_foot(b, [x, y], foot + 0.004);
    }
    let mut base = upholstered(
        [x0 + 0.02, 0.06, foot],
        [-x0 - 0.02, s.d, base_top],
        0.03,
        1,
    );
    base.welt = 0.005;
    base.add(b, "mat");
    // Rolled arms.
    for (i, (a0, a1)) in [(x0, x0 + arm), (-x0 - arm, -x0)].into_iter().enumerate() {
        let mut a = upholstered(
            [a0, 0.12, foot],
            [a1, s.d + 0.02, arm_h],
            arm * 0.48,
            3 + i as u32,
        );
        a.welt = 0.005;
        a.add(b, "mat");
    }
    // The tall back, buttoned, leant a little.
    let start = b.mark();
    let mut back = upholstered(
        [x0 + arm - 0.02, 0.0, foot],
        [-x0 - arm + 0.02, 0.17, h],
        0.05,
        2,
    );
    back.crown = 0.02;
    back.tufts = 0.12;
    back.tuft_depth = 0.03;
    back.tuft_faces = [false, true];
    back.step = 0.028;
    back.add(b, "mat");
    let (st, ct) = 6f64.to_radians().sin_cos();
    b.transform_from(start, |q| {
        let dz = q[2] - foot;
        [q[0], q[1] * ct - dz * st + 0.04, foot + q[1] * st + dz * ct]
    });
    // The wings: from the back's edges forward over the arms, turned out a
    // little, rounded at the front.
    for (i, side) in [-1.0f64, 1.0].into_iter().enumerate() {
        let start = b.mark();
        let xw = side * (s.w / 2.0 - 0.06);
        let mut wing = upholstered(
            [xw - 0.045, 0.0, arm_h - 0.02],
            [xw + 0.045, s.d * 0.42, h - 0.04],
            0.04,
            40 + i as u32,
        );
        wing.welt = 0.005;
        wing.add(b, "mat");
        // Turned out about its back end: the front swings outwards, the
        // top more than the foot.
        b.transform_from(start, |q| {
            let (dx, dy) = (q[0] - xw, q[1]);
            let lift = ((q[2] - arm_h) / (h - arm_h)).clamp(0.0, 1.0);
            let a = -side * (0.12 + 0.14 * lift);
            let (sa, ca) = a.sin_cos();
            [xw + dx * ca - dy * sa, dx * sa + dy * ca + 0.03, q[2]]
        });
    }
    seat_cushions(
        b,
        1,
        x0 + arm,
        -x0 - arm,
        0.17,
        s.d + 0.015,
        base_top,
        s.seat,
        0.006,
        0.04,
    );
    pillow(b, &s, 0.32, s.seat);
}

/// Pufs and footstools, one per collection (paquete 1a). Values: `width`,
/// `depth` (the diameter of the round ones is the smaller), `height`.
/// Centred on `x = 0`, its back at `y = 0`.
pub fn pouf(b: &mut Builder, p: &Params) {
    let w = p.get("width", 0.5).clamp(0.3, 1.4);
    let d = p.get("depth", 0.5).clamp(0.3, 1.0);
    let h = p.get("height", 0.42).clamp(0.15, 0.6);
    let c = [0.0, d / 2.0];
    // A round pouf: a drum, its edges rounded and piped, its top domed.
    let drum = |b: &mut Builder, role: &'static str, z0: f64, r: f64, top: f64| {
        let hh = top - z0;
        turned(
            b,
            role,
            [c[0], c[1], z0],
            &[
                (0.0, -0.004),
                (r - 0.05, 0.0),
                (r - 0.012, 0.006),
                (r, 0.03),
                (r + 0.006, hh * 0.5),
                (r, hh - 0.03),
                (r - 0.012, hh - 0.006),
                (r - 0.05, hh),
                (0.0, hh + 0.012),
            ],
            48,
            0.0,
        );
        // The piping round the top and the foot.
        for z in [z0 + hh - 0.012, z0 + 0.012] {
            let ring: Vec<P3> = (0..48)
                .map(|j| {
                    let a = TAU * j as f64 / 48.0;
                    let rr = r - 0.006;
                    [c[0] + rr * a.cos(), c[1] + rr * a.sin(), z]
                })
                .collect();
            super::soft::tube(b, role, &ring, true, 0.0055, 6);
        }
        b.soft.insert(role);
    };
    match p.style() {
        // Mediterranean: esparto braided in rings, a coiled lid.
        2 => {
            let r = w.min(d) / 2.0;
            let rope = 0.013;
            let rings = ((h - 0.02) / (rope * 1.75)).floor() as usize;
            for i in 0..rings {
                let z = rope + i as f64 * rope * 1.75;
                // A little wider in the middle, as a soft basket.
                let k = (i as f64 / rings as f64 - 0.5) * 2.0;
                let rr = r * (1.0 - 0.04 * k * k) - rope;
                let path: Vec<P3> = (0..32)
                    .map(|j| {
                        let a = TAU * j as f64 / 32.0 + i as f64 * 0.3;
                        [c[0] + rr * a.cos(), c[1] + rr * a.sin(), z]
                    })
                    .collect();
                super::soft::tube(b, "weave", &path, true, rope, 7);
            }
            // The lid: a spiral from the middle out.
            let top = rope + (rings - 1) as f64 * rope * 1.75 + rope * 0.6;
            let turns = ((r - rope) / (rope * 1.9)).floor().max(1.0);
            let n = (turns * 28.0) as usize;
            let path: Vec<P3> = (0..=n)
                .map(|j| {
                    let t = j as f64 / n as f64;
                    let a = TAU * turns * t;
                    let rr = (r - 1.5 * rope) * t + 0.004;
                    [
                        c[0] + rr * a.cos(),
                        c[1] + rr * a.sin(),
                        top + 0.004 * (1.0 - t),
                    ]
                })
                .collect();
            sweep(
                b,
                "weave",
                &path,
                |_| rope,
                7,
                Cap::Round(1.0),
                Cap::Round(1.0),
            );
            // A disc under the coils so nothing shows through.
            turned(
                b,
                "weave",
                [c[0], c[1], 0.0],
                &[
                    (0.0, 0.004),
                    (r - rope * 1.5, 0.004),
                    (r - rope * 1.5, top - 0.004),
                    (0.0, top - 0.004),
                ],
                24,
                0.0,
            );
        }
        // Fifties: an upholstered drum on short splayed legs.
        3 => {
            let foot = 0.13;
            let r = w.min(d) / 2.0;
            for k in 0..4 {
                let a = PI / 4.0 + k as f64 * PI / 2.0;
                let (sa, ca) = a.sin_cos();
                compass_leg(
                    b,
                    [c[0] + ca * r * 0.6, c[1] + sa * r * 0.6],
                    foot + 0.005,
                    [ca, sa],
                    0.03,
                );
            }
            drum(b, "mat", foot, r, h);
        }
        // Japandi: a thick floor cushion, square, with one button.
        4 => {
            let mut s = Soft::new([-w / 2.0, 0.0, 0.0], [w / 2.0, d, h.min(0.24)], 0.06);
            s.crown = 0.03;
            s.side = 0.014;
            s.sag = 0.012;
            s.creases = 0.01;
            s.wobble = 0.002;
            s.tufts = w.max(d) * 1.2;
            s.tuft_depth = 0.035;
            s.tuft_faces = [false, true];
            s.step = 0.04;
            s.seed = 32;
            s.add(b, "cushion");
        }
        // Contemporary: a cube, piped, on a recessed plinth.
        5 => {
            b.rbox(
                "metal",
                [-w / 2.0 + 0.03, 0.03, 0.0],
                [w / 2.0 - 0.03, d - 0.03, 0.035],
                0.003,
            );
            let mut s = upholstered([-w / 2.0, 0.0, 0.03], [w / 2.0, d, h], 0.03, 33);
            s.crown = 0.012;
            s.welt = 0.005;
            s.add(b, "mat");
        }
        // Rustic: a stool of turned legs and stretchers, a rush seat.
        6 => {
            let leg_top = h - 0.04;
            for (x, y) in [
                (-w / 2.0 + 0.04, 0.04),
                (w / 2.0 - 0.04, 0.04),
                (-w / 2.0 + 0.04, d - 0.04),
                (w / 2.0 - 0.04, d - 0.04),
            ] {
                turned(
                    b,
                    "frame",
                    [x, y, 0.0],
                    &[
                        (0.0, 0.0),
                        (0.018, 0.0),
                        (0.02, leg_top * 0.3),
                        (0.026, leg_top * 0.4),
                        (0.018, leg_top * 0.5),
                        (0.022, leg_top),
                        (0.0, leg_top),
                    ],
                    14,
                    0.0,
                );
            }
            for (a, z) in [(0usize, 0.1), (1, 0.14)] {
                let (x0, x1, y0, y1) = (-w / 2.0 + 0.04, w / 2.0 - 0.04, 0.04, d - 0.04);
                let pairs: [(P3, P3); 2] = if a == 0 {
                    [([x0, y0, z], [x1, y0, z]), ([x0, y1, z], [x1, y1, z])]
                } else {
                    [([x0, y0, z], [x0, y1, z]), ([x1, y0, z], [x1, y1, z])]
                };
                for (p0, p1) in pairs {
                    sweep(b, "frame", &[p0, p1], |_| 0.011, 10, Cap::Flat, Cap::Flat);
                }
            }
            b.rbox(
                "frame",
                [-w / 2.0 + 0.015, 0.015, leg_top - 0.03],
                [w / 2.0 - 0.015, d - 0.015, leg_top],
                0.008,
            );
            let mut seat = Soft::new(
                [-w / 2.0 + 0.01, 0.01, leg_top - 0.01],
                [w / 2.0 - 0.01, d - 0.01, h],
                0.02,
            );
            seat.crown = 0.008;
            seat.wobble = 0.0015;
            seat.seed = 34;
            seat.add(b, "weave");
        }
        // Industrial: round leather on a black steel ring.
        7 => {
            let r = w.min(d) / 2.0;
            let ring: Vec<P3> = (0..32)
                .map(|j| {
                    let a = TAU * j as f64 / 32.0;
                    [c[0] + r * 0.8 * a.cos(), c[1] + r * 0.8 * a.sin(), 0.012]
                })
                .collect();
            super::soft::tube(b, "metal", &ring, true, 0.012, 8);
            for k in 0..3 {
                let a = TAU * k as f64 / 3.0;
                let (sa, ca) = a.sin_cos();
                sweep(
                    b,
                    "metal",
                    &[
                        [c[0] + ca * r * 0.8, c[1] + sa * r * 0.8, 0.012],
                        [c[0] + ca * r * 0.7, c[1] + sa * r * 0.7, 0.07],
                    ],
                    |_| 0.01,
                    8,
                    Cap::Flat,
                    Cap::Flat,
                );
            }
            drum(b, "mat", 0.06, r, h);
        }
        // Classic: buttoned on top, on four turned legs.
        8 => {
            let foot = 0.12;
            for (x, y) in [
                (-w / 2.0 + 0.05, 0.05),
                (w / 2.0 - 0.05, 0.05),
                (-w / 2.0 + 0.05, d - 0.05),
                (w / 2.0 - 0.05, d - 0.05),
            ] {
                bun_foot(b, [x, y], foot + 0.004);
            }
            let mut s = upholstered([-w / 2.0, 0.0, foot], [w / 2.0, d, h], 0.04, 36);
            s.crown = 0.02;
            s.welt = 0.005;
            s.tufts = 0.12;
            s.tuft_depth = 0.04;
            s.tuft_faces = [false, true];
            s.step = 0.025;
            s.add(b, "mat");
        }
        // Nordic: round, in bouclé, piped.
        _ => {
            let r = w.min(d) / 2.0;
            drum(b, "mat", 0.0, r, h);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::shape::signed_volume;

    /// Every armchair and pouf of the collections is made, closed and light.
    #[test]
    fn the_collection_armchairs_and_pufs_are_made() {
        for kind in ["fine_armchair", "fine_pouf"] {
            for style in 1..=8 {
                let mut p = crate::Params::default();
                p.set("style", style as f64);
                let m = crate::make(kind, &p).unwrap();
                let tris = m.triangle_count();
                assert!((800..30_000).contains(&tris), "{kind} {style}: {tris}");
                let v: f64 = m
                    .parts
                    .iter()
                    .map(|(_, t)| signed_volume(&m.positions, t))
                    .sum();
                assert!(v > 0.005, "{kind} {style}: {v}");
            }
        }
    }
}
