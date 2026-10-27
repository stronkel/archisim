//! The workshop's kitchen sink: a stainless basin set into the worktop,
//! its rim lying on it and rolled over into the bowl, the bowl with its
//! rounded corners and the floor falling to the waste, the strainer; and
//! a gooseneck mixer tap all of one piece, its lever growing out of its
//! body.

use super::shape::{add, loft_loop, sweep, turned, Cap};
use crate::build::{Builder, P3};
use crate::Params;
use std::f64::consts::PI;

/// A base module with the sink: the library's module (its carcass, doors
/// and fronts) with the workshop's worktop, basin and tap. Same values as
/// the library's «Módulo bajo de cocina».
pub fn sink_unit(b: &mut Builder, p: &Params) {
    let mut q = p.clone();
    q.set("unit", crate::SINK_UNIT);
    q.set("fine_sink", 1.0);
    crate::kitchen::base_unit(b, &q);
    let w = p.get("width", 0.8);
    let d = p.get("depth", 0.6);
    let h = p.get("height", 0.9);
    let front = p.get("overhang", 0.0).max(0.0);
    let back = p.get("overhang_back", 0.0).max(0.0);
    let (x0, x1) = (-w / 2.0, w / 2.0);
    let tlo = [x0, -back, h - crate::WORKTOP];
    let thi = [x1, d + front, h];
    match crate::sink_hole(w, d) {
        Some((lo, hi)) => {
            b.framed_top("top", tlo, thi, lo, hi);
            let c = [(lo[0] + hi[0]) / 2.0, (lo[1] + hi[1]) / 2.0];
            let (a, bb) = ((hi[0] - lo[0]) / 2.0, (hi[1] - lo[1]) / 2.0);
            basin(b, c, a, bb, h);
            // The tap behind the bowl, on the worktop, its spout over the
            // middle of the bowl.
            let reach = (c[1] - lo[1] * 0.5).clamp(0.12, 0.26);
            tap(b, [c[0], lo[1] * 0.5, h], reach);
        }
        None => b.rbox("top", tlo, thi, 0.0),
    }
}

/// The basin under a hole of half sides `a`, `bb` about `c` in a worktop
/// whose top is at `h`.
pub fn basin(b: &mut Builder, c: [f64; 2], a: f64, bb: f64, h: f64) {
    let deep = 0.2;
    let r = 0.032;
    // The section of the steel, all round: the rim on the worktop, rolled
    // into the bowl, down its wall, across the floor to the waste, and
    // back under it and up the outside.
    let rings = [
        (a + 0.012, bb + 0.012, r + 0.012, h),
        (a + 0.012, bb + 0.012, r + 0.012, h + 0.0022),
        (a + 0.008, bb + 0.008, r + 0.008, h + 0.0042),
        (a - 0.004, bb - 0.004, r, h + 0.0048),
        (a - 0.009, bb - 0.009, r, h + 0.0032),
        (a - 0.011, bb - 0.011, r, h - 0.006),
        (a - 0.016, bb - 0.016, r, h - deep + 0.016),
        (a - 0.03, bb - 0.03, r * 0.7, h - deep + 0.002),
        (0.046, 0.046, 0.046, h - deep - 0.004),
        (0.046, 0.046, 0.046, h - deep - 0.007),
        (a - 0.028, bb - 0.028, r * 0.7, h - deep - 0.0005),
        (a - 0.014, bb - 0.014, r, h - deep + 0.015),
        (a - 0.009, bb - 0.009, r, h - 0.006),
        (a - 0.008, bb - 0.008, r, h),
    ];
    loft_loop(b, "sink", c, &rings);
    // The strainer over the waste, a little proud of the floor, with its
    // darker basket in the middle.
    let z = h - deep - 0.004;
    let at = [c[0], c[1], z];
    turned(
        b,
        "sink",
        at,
        &[
            (0.0, -0.006),
            (0.04, -0.006),
            (0.04, 0.0),
            (0.056, 0.0012),
            (0.056, 0.0022),
            (0.05, 0.0028),
            (0.0, 0.0028),
        ],
        32,
        0.0,
    );
    turned(
        b,
        "dark",
        at,
        &[
            (0.0, 0.0024),
            (0.028, 0.0024),
            (0.03, 0.0034),
            (0.0, 0.0034),
        ],
        24,
        0.0,
    );
}

/// A gooseneck mixer standing at `at` (on the worktop), its spout
/// `reach` in front (towards `+y`).
pub fn tap(b: &mut Builder, at: P3, reach: f64) {
    tap_in(b, "sink", at, reach);
}

/// [`tap`] made of `role`.
pub fn tap_in(b: &mut Builder, role: &'static str, at: P3, reach: f64) {
    let [x, y, z] = at;
    let rb = 0.0215;
    // The body, turned: a foot on the worktop and a slim column, its top
    // rounded into the neck.
    turned(
        b,
        role,
        at,
        &[
            (0.0, 0.0),
            (rb + 0.004, 0.0),
            (rb + 0.004, 0.003),
            (rb + 0.001, 0.007),
            (rb, 0.012),
            (rb - 0.0015, 0.17),
            (rb - 0.004, 0.178),
            (0.012, 0.183),
            (0.0, 0.185),
        ],
        28,
        0.0,
    );
    // The neck: up out of the body, over in a half circle, and down to
    // the spray head.
    let r = reach / 2.0;
    let top = 0.33;
    let mut path: Vec<P3> = vec![[x, y, z + 0.15], [x, y, z + 0.2], [x, y, z + top - 0.03]];
    for i in 0..=14 {
        let t = PI * i as f64 / 14.0;
        path.push([x, y + r - r * t.cos(), z + top + r * t.sin()]);
    }
    path.push([x, y + reach, z + top - 0.02]);
    let rn = 0.0108;
    sweep(b, role, &path, |_| rn, 20, Cap::Flat, Cap::Flat);
    // The spray head, a little wider, its nozzle dark underneath.
    let head = [x, y + reach, z + top - 0.085];
    turned(
        b,
        role,
        head,
        &[
            (0.0, 0.0),
            (0.0115, 0.0),
            (0.0135, 0.004),
            (0.0138, 0.05),
            (rn, 0.068),
            (0.0, 0.07),
        ],
        24,
        0.0,
    );
    turned(
        b,
        "dark",
        head,
        &[
            (0.0, -0.0012),
            (0.009, -0.0012),
            (0.009, 0.0005),
            (0.0, 0.0005),
        ],
        20,
        0.0,
    );
    // The lever on the right of the body: its boss grows out of the
    // column, and the handle rises from it towards the back.
    let lz = z + 0.105;
    turned_x(b, role, [x, y, lz], rb - 0.006, rb + 0.016, 0.0125);
    let handle = vec![
        [x + rb + 0.012, y, lz],
        [x + rb + 0.024, y - 0.004, lz + 0.012],
        [x + rb + 0.032, y - 0.012, lz + 0.04],
        [x + rb + 0.035, y - 0.018, lz + 0.068],
    ];
    let handle = super::shape::curve(&handle, 4);
    sweep(
        b,
        role,
        &handle,
        |t| 0.0068 - 0.0012 * t,
        14,
        Cap::Flat,
        Cap::Round(0.8),
    );
}

/// A short round boss across the vertical, along `+x` from `x0` to `x1`
/// off the axis at `c`.
fn turned_x(b: &mut Builder, role: &'static str, c: P3, x0: f64, x1: f64, r: f64) {
    let a = add(c, [x0, 0.0, 0.0]);
    let e = add(c, [x1, 0.0, 0.0]);
    sweep(b, role, &[a, e], |_| r, 22, Cap::Flat, Cap::Round(0.35));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_sink_unit_is_closed_and_light() {
        let mut b = Builder::default();
        let mut p = Params::default();
        p.set("width", 0.8);
        sink_unit(&mut b, &p);
        let tris = b.triangle_count();
        assert!((2_000..14_000).contains(&tris), "{tris}");
        for (role, t) in &b.parts {
            let v = super::super::shape::signed_volume(&b.positions, t);
            assert!(v > 0.0, "{role}: {v}");
        }
    }
}
