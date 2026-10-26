//! The workshop's lamp post: a cast-iron column in the Spanish manner,
//! one turned profile from the base to the capital (plinth, torus, bell,
//! collars, a vase, the slim shaft with its ring, the capital that opens
//! to hold the lantern), the bar for the ladder across it, four scrolls
//! that hold the lantern's dish, and the lantern: four panes between
//! corner posts, banded top and bottom, under a curved roof with its
//! finial. Every piece runs into the next.

use super::shape::{add, bar, curve, mul, plate, sweep, turned, Cap};
use crate::build::{Builder, P3};
use crate::Params;
use std::f64::consts::{FRAC_PI_4, PI, SQRT_2};

/// Sides of the round parts.
const ROUND: usize = 28;

/// Values: `height` (to the top of the finial), `pole` (diameter of the
/// shaft at its foot), `lamp` (width of the lantern at its top). Centred
/// on the origin.
pub fn streetlight(b: &mut Builder, p: &Params) {
    let h = p.get("height", 3.5).clamp(2.2, 6.0);
    let k = (p.get("pole", 0.14).clamp(0.08, 0.3)) / 0.14;
    let lamp = p.get("lamp", 0.42).clamp(0.25, 0.8);
    // The lantern's sizes.
    let wt = lamp / 2.0;
    let wb = wt * 0.66;
    let body = lamp * 1.05;
    let dish = 0.07;
    let roof = lamp * 0.5;
    let finial = 0.13;
    let zc = h - (dish + body + roof + finial);
    let cap_h = 0.2 * k;
    let shaft_top = zc - cap_h;

    // The plinth, octagonal, its top chamfered.
    turned(
        b,
        "mat",
        [0.0; 3],
        &[
            (0.0, 0.0),
            (0.185 * k, 0.0),
            (0.185 * k, 0.055 * k),
            (0.17 * k, 0.072 * k),
            (0.0, 0.072 * k),
        ],
        8,
        PI / 8.0,
    );
    // The column, one profile from the plinth to the capital.
    let r = |x: f64| x * k;
    let mut col: Vec<(f64, f64)> = vec![
        (0.0, r(0.06)),
        (r(0.135), r(0.06)),
        (r(0.135), r(0.085)),
        // The torus.
        (r(0.142), r(0.095)),
        (r(0.144), r(0.108)),
        (r(0.138), r(0.12)),
        (r(0.122), r(0.128)),
        (r(0.118), r(0.14)),
        // The bell, curving in.
        (r(0.112), r(0.2)),
        (r(0.1), r(0.32)),
        (r(0.09), r(0.46)),
        (r(0.086), r(0.56)),
        // A collar.
        (r(0.098), r(0.57)),
        (r(0.101), r(0.585)),
        (r(0.098), r(0.6)),
        (r(0.084), r(0.61)),
        // The vase.
        (r(0.092), r(0.68)),
        (r(0.094), r(0.75)),
        (r(0.086), r(0.83)),
        (r(0.068), r(0.9)),
        (r(0.06), r(0.93)),
        // Another collar.
        (r(0.074), r(0.94)),
        (r(0.077), r(0.955)),
        (r(0.074), r(0.97)),
        (r(0.058), r(0.98)),
    ];
    // The shaft, slim and slightly tapered, with a ring halfway up.
    let s0 = r(0.98);
    let ring_z = s0 + (shaft_top - s0) * 0.55;
    let shaft_r = |z: f64| r(0.056) - r(0.012) * ((z - s0) / (shaft_top - s0)).clamp(0.0, 1.0);
    col.extend([
        (shaft_r(ring_z - 0.03), ring_z - 0.03),
        (shaft_r(ring_z) + r(0.014), ring_z - 0.012),
        (shaft_r(ring_z) + r(0.016), ring_z),
        (shaft_r(ring_z) + r(0.014), ring_z + 0.012),
        (shaft_r(ring_z + 0.03), ring_z + 0.03),
        (shaft_r(shaft_top), shaft_top),
    ]);
    // The capital: a collar, a neck, and a cup that opens under the dish.
    let c = shaft_top;
    let sr = shaft_r(shaft_top);
    col.extend([
        (sr + r(0.016), c + cap_h * 0.06),
        (sr + r(0.018), c + cap_h * 0.14),
        (sr + r(0.004), c + cap_h * 0.2),
        (sr, c + cap_h * 0.3),
        (sr + r(0.008), c + cap_h * 0.55),
        (sr + r(0.03), c + cap_h * 0.8),
        (sr + r(0.05), c + cap_h * 0.95),
        (sr + r(0.05), c + cap_h + 0.006),
        (0.0, c + cap_h + 0.006),
    ]);
    turned(b, "mat", [0.0; 3], &col, ROUND, 0.0);

    // The ladder bar, through the shaft below the capital, with a knob at
    // each end.
    let zb = shaft_top - 0.12;
    let half = r(0.26);
    sweep(
        b,
        "mat",
        &[[-half, 0.0, zb], [half, 0.0, zb]],
        |_| r(0.011),
        12,
        Cap::Flat,
        Cap::Flat,
    );
    for x in [-half, half] {
        b.ellipsoid("mat", [x, 0.0, zb], [r(0.022), r(0.022), r(0.022)]);
    }

    // The dish under the lantern: square, flaring from the capital.
    let z0 = zc;
    let wd = wb * SQRT_2;
    turned(
        b,
        "mat",
        [0.0, 0.0, z0],
        &[
            (sr + r(0.03), -0.004),
            (wd * 0.7, dish * 0.4),
            (wd * 1.12, dish * 0.8),
            (wd * 1.12, dish),
            (wd * 0.8, dish + 0.004),
        ],
        4,
        FRAC_PI_4,
    );
    // Four scrolls from the neck of the capital out under the corners of
    // the dish.
    for i in 0..4 {
        let a = FRAC_PI_4 + i as f64 * PI / 2.0;
        let d = [a.cos(), a.sin(), 0.0];
        // An S: out of the neck, curling under, and up to the corner.
        let from = add(mul(d, sr + r(0.005)), [0.0, 0.0, c + cap_h * 0.32]);
        let pts = curve(
            &[
                from,
                add(mul(d, sr + r(0.045)), [0.0, 0.0, c + cap_h * 0.3]),
                add(mul(d, wd * 0.62), [0.0, 0.0, c + cap_h * 0.55]),
                add(mul(d, wd * 0.95), [0.0, 0.0, z0 - 0.01]),
                add(mul(d, wd * 1.04), [0.0, 0.0, z0 + dish * 0.7]),
            ],
            6,
        );
        sweep(
            b,
            "mat",
            &pts,
            |t| r(0.009) - r(0.003) * t,
            7,
            Cap::Round(1.0),
            Cap::Flat,
        );
    }

    // The lantern: corner posts leaning out, panes between them, a band
    // at the foot and at the top.
    let zl = z0 + dish;
    let zt = zl + body;
    let corner = |w: f64, i: usize, z: f64| -> P3 {
        let a = FRAC_PI_4 + i as f64 * PI / 2.0;
        [w * SQRT_2 * a.cos(), w * SQRT_2 * a.sin(), z]
    };
    for i in 0..4 {
        let (a, t) = (corner(wb, i, zl - 0.005), corner(wt, i, zt + 0.005));
        bar(b, "mat", a, t, 0.02, [a[0], a[1], 0.0]);
    }
    let inset = 0.97;
    for i in 0..4 {
        let j = (i + 1) % 4;
        let q = [
            corner(wb * inset, i, zl),
            corner(wb * inset, j, zl),
            corner(wt * inset, j, zt),
            corner(wt * inset, i, zt),
        ];
        // Counter-clockwise seen from outside: the pane's front faces out.
        plate(b, "light", q, 0.004);
        // A thin glazing bar across the middle.
        let m0 = super::shape::lerp(q[0], q[3], 0.5);
        let m1 = super::shape::lerp(q[1], q[2], 0.5);
        let out = mul(add(m0, m1), 0.5);
        bar(b, "mat", m0, m1, 0.009, [out[0], out[1], 0.0]);
    }
    // The bands: square frames round the foot and the top of the panes.
    let band = |b: &mut Builder, w: f64, z: f64, t: f64| {
        let wd = w * SQRT_2;
        turned(
            b,
            "mat",
            [0.0, 0.0, z],
            &[
                (wd * 0.86, -t / 2.0),
                (wd * 1.07, -t / 2.0),
                (wd * 1.07, t / 2.0),
                (wd * 0.86, t / 2.0),
            ],
            4,
            FRAC_PI_4,
        );
    };
    band(b, wb, zl + 0.008, 0.024);
    band(b, wt, zt - 0.006, 0.03);
    // The roof: a wide eave, then curving up to the crown.
    let wr = wt * SQRT_2;
    turned(
        b,
        "mat",
        [0.0, 0.0, zt],
        &[
            (wr * 0.9, 0.0),
            (wr * 1.2, 0.006),
            (wr * 1.22, 0.02),
            (wr * 1.0, roof * 0.2),
            (wr * 0.7, roof * 0.42),
            (wr * 0.42, roof * 0.66),
            (wr * 0.22, roof * 0.86),
            (r(0.035), roof),
            (0.0, roof),
        ],
        4,
        FRAC_PI_4,
    );
    // The finial: a neck, a collar, a ball and a point.
    let zf = zt + roof - 0.004;
    turned(
        b,
        "mat",
        [0.0, 0.0, zf],
        &[
            (0.0, 0.0),
            (r(0.03), 0.0),
            (r(0.018), finial * 0.12),
            (r(0.026), finial * 0.2),
            (r(0.026), finial * 0.26),
            (r(0.014), finial * 0.32),
            (r(0.032), finial * 0.45),
            (r(0.036), finial * 0.6),
            (r(0.028), finial * 0.74),
            (r(0.008), finial * 0.86),
            (0.0, finial),
        ],
        ROUND,
        0.0,
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_lamp_post_is_its_height_and_closed() {
        let mut b = Builder::default();
        let mut p = Params::default();
        p.set("height", 3.5);
        streetlight(&mut b, &p);
        let top = b.positions.iter().map(|p| p[2]).fold(0.0f32, f32::max);
        assert!((top - 3.5).abs() < 0.01, "{top}");
        for (role, t) in &b.parts {
            let v = super::super::shape::signed_volume(&b.positions, t);
            assert!(v > 0.0, "{role}: {v}");
        }
        let tris = b.triangle_count();
        assert!((2_000..12_000).contains(&tris), "{tris}");
    }
}
