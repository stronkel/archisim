//! People in the style of the cosy library (C1, 24/10/2026): soft,
//! rounded figures like the ones of a model, with their clothes, skin and
//! hair each in its own colour (all of one colour if the type gives only
//! one), for scale. Centred on their feet, looking towards +y. 1 a man
//! standing; 2 a woman in a dress; 3 someone walking.
//!
//! Roles: `mat` the top (shirt, jacket, dress), `mat2` trousers and shoes,
//! `skin` face and hands, `hair`.

use crate::build::Builder;
use crate::Params;

/// A limb: a tapering tube with rounded ends (a ball at each joint).
fn limb(b: &mut Builder, role: &'static str, a: [f64; 3], c: [f64; 3], ra: f64, rc: f64) {
    b.cone(role, a, c, ra, rc, 14);
    b.ellipsoid(role, a, [ra, ra, ra]);
    b.ellipsoid(role, c, [rc, rc, rc]);
}

pub fn person(b: &mut Builder, p: &Params) {
    let h = p.get("height", 1.75);
    let style = p.style();
    let k = h / 1.75;
    let s = |v: f64| v * k;
    let walking = style == 3;
    let dress = style == 2;
    // Legs and shoes: one forward, one back when walking.
    for side in [-1.0, 1.0] {
        let x = side * s(0.085);
        let stride = if walking { side * s(0.17) } else { 0.0 };
        let hip = [x, 0.0, s(0.9)];
        let knee = [x * 1.02, stride * 0.55, s(0.5)];
        let ankle = [x, stride, s(0.09)];
        if !dress {
            limb(b, "mat2", hip, knee, s(0.075), s(0.058));
        }
        limb(
            b,
            if dress { "skin" } else { "mat2" },
            knee,
            ankle,
            s(0.055),
            s(0.042),
        );
        // A shoe: a rounded block, longer forwards.
        b.rbox(
            "mat2",
            [x - s(0.048), stride - s(0.07), 0.0],
            [x + s(0.048), stride + s(0.16), s(0.085)],
            s(0.04),
        );
    }
    // The body: trousers' waist and the top, or a dress to the knees,
    // turned as a soft profile (radius, height above the floor).
    if dress {
        b.lathe(
            "mat",
            [0.0, 0.0, 0.0],
            &[
                (s(0.25), s(0.5)),
                (s(0.235), s(0.62)),
                (s(0.19), s(0.82)),
                (s(0.155), s(0.98)),
                (s(0.165), s(1.18)),
                (s(0.185), s(1.36)),
                (s(0.15), s(1.44)),
                (s(0.06), s(1.47)),
            ],
            1.0,
            0.62,
        );
    } else {
        b.lathe(
            "mat2",
            [0.0, 0.0, 0.0],
            &[(s(0.155), s(0.8)), (s(0.165), s(0.92)), (s(0.16), s(1.0))],
            1.0,
            0.66,
        );
        b.lathe(
            "mat",
            [0.0, 0.0, 0.0],
            &[
                (s(0.165), s(0.94)),
                (s(0.17), s(1.08)),
                (s(0.19), s(1.3)),
                (s(0.2), s(1.38)),
                (s(0.15), s(1.45)),
                (s(0.06), s(1.48)),
            ],
            1.0,
            0.64,
        );
    }
    // Arms in sleeves, hands bare; swinging when walking.
    for side in [-1.0, 1.0] {
        let x = side * s(0.2);
        let swing = if walking { -side * s(0.15) } else { 0.0 };
        let shoulder = [x, 0.0, s(1.37)];
        let elbow = [x + side * s(0.03), swing * 0.5, s(1.1)];
        let wrist = [x + side * s(0.04), swing, s(0.86)];
        limb(b, "mat", shoulder, elbow, s(0.055), s(0.047));
        limb(b, "mat", elbow, wrist, s(0.047), s(0.04));
        b.ellipsoid(
            "skin",
            [wrist[0], wrist[1], wrist[2] - s(0.05)],
            [s(0.035), s(0.028), s(0.055)],
        );
    }
    // Neck, head and hair.
    b.cyl("skin", [0.0, 0.0, s(1.44)], [0.0, 0.0, s(1.54)], s(0.048));
    b.ellipsoid(
        "skin",
        [0.0, s(0.01), s(1.64)],
        [s(0.088), s(0.1), s(0.115)],
    );
    if dress {
        // Long hair over the back and a bun.
        b.ellipsoid(
            "hair",
            [0.0, -s(0.035), s(1.67)],
            [s(0.096), s(0.1), s(0.11)],
        );
        b.ellipsoid(
            "hair",
            [0.0, -s(0.06), s(1.53)],
            [s(0.085), s(0.05), s(0.11)],
        );
        b.ellipsoid(
            "hair",
            [0.0, -s(0.1), s(1.69)],
            [s(0.05), s(0.045), s(0.05)],
        );
    } else {
        // Short hair: a cap over the top and the back.
        b.ellipsoid(
            "hair",
            [0.0, -s(0.03), s(1.69)],
            [s(0.093), s(0.1), s(0.08)],
        );
    }
}
