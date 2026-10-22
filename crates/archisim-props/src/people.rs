//! People, as architects draw them: figures of one colour, for scale.
//! Centred on their feet, looking towards +y. 1 a man standing; 2 a woman
//! in a skirt; 3 someone walking.

use crate::build::Builder;
use crate::Params;

pub fn person(b: &mut Builder, p: &Params) {
    let h = p.get("height", 1.75);
    let style = p.style();
    let k = h / 1.75;
    let s = |v: f64| v * k;
    let walking = style == 3;
    // Legs (and feet).
    for side in [-1.0, 1.0] {
        let x = side * s(0.09);
        let stride = if walking { side * s(0.16) } else { 0.0 };
        let knee = [x, stride * 0.5, s(0.5)];
        let foot = [x, stride, s(0.08)];
        if style == 2 {
            b.cone(
                "mat",
                [x * 0.9, stride * 0.3, s(0.08)],
                [x * 0.8, 0.0, s(0.55)],
                s(0.045),
                s(0.055),
                12,
            );
        } else {
            b.cone("mat", foot, knee, s(0.045), s(0.055), 12);
            b.cone("mat", knee, [x, 0.0, s(0.93)], s(0.055), s(0.075), 12);
        }
        b.rbox(
            "mat",
            [x - s(0.045), stride - s(0.06), 0.0],
            [x + s(0.045), stride + s(0.17), s(0.09)],
            s(0.035),
        );
    }
    // Hips (a skirt for 2), torso, shoulders.
    if style == 2 {
        b.cone(
            "mat",
            [0.0, 0.0, s(0.52)],
            [0.0, 0.0, s(0.98)],
            s(0.24),
            s(0.15),
            20,
        );
    } else {
        b.rbox(
            "mat",
            [-s(0.16), -s(0.1), s(0.86)],
            [s(0.16), s(0.1), s(1.0)],
            s(0.06),
        );
    }
    b.rbox(
        "mat",
        [-s(0.18), -s(0.11), s(0.98)],
        [s(0.18), s(0.12), s(1.42)],
        s(0.09),
    );
    b.ellipsoid("mat", [0.0, 0.0, s(1.4)], [s(0.2), s(0.12), s(0.06)]);
    // Arms, slightly away from the body (swinging when walking).
    for side in [-1.0, 1.0] {
        let x = side * s(0.21);
        let swing = if walking { -side * s(0.14) } else { 0.0 };
        let elbow = [x + side * s(0.02), swing * 0.5, s(1.12)];
        let hand = [x + side * s(0.03), swing, s(0.86)];
        b.cone("mat", [x, 0.0, s(1.38)], elbow, s(0.05), s(0.042), 10);
        b.cone("mat", elbow, hand, s(0.042), s(0.034), 10);
        b.ellipsoid(
            "mat",
            [hand[0], hand[1], hand[2] - s(0.04)],
            [s(0.035), s(0.025), s(0.06)],
        );
    }
    // Neck and head.
    b.cyl("mat", [0.0, 0.0, s(1.42)], [0.0, 0.0, s(1.52)], s(0.05));
    b.ellipsoid("mat", [0.0, s(0.01), s(1.63)], [s(0.085), s(0.1), s(0.12)]);
    if style == 2 {
        // Hair gathered at the back.
        b.ellipsoid("mat", [0.0, -s(0.09), s(1.62)], [s(0.06), s(0.05), s(0.07)]);
    }
}
