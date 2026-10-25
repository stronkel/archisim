//! Cars: the centre of the rear at the origin, looking towards +y.
//! 1 a saloon; 2 a hatchback; 3 an SUV. In the style of the cosy library
//! (C1, 24/10/2026): soft, rounded bodies like a toy's, round lights and
//! mirrors, chunky wheels.

use crate::build::Builder;
use crate::Params;

/// A solid between a bottom rectangle and a top one (both level), as the
/// cabin of a car: its sides are glass, its top the roof.
fn cabin(b: &mut Builder, lo: [[f64; 2]; 2], z0: f64, hi: [[f64; 2]; 2], z1: f64, roof: f64) {
    // Corners: bottom (x0,y0) (x1,y0) (x1,y1) (x0,y1), top the same.
    let bot = [
        [lo[0][0], lo[0][1]],
        [lo[1][0], lo[0][1]],
        [lo[1][0], lo[1][1]],
        [lo[0][0], lo[1][1]],
    ];
    let top = [
        [hi[0][0], hi[0][1]],
        [hi[1][0], hi[0][1]],
        [hi[1][0], hi[1][1]],
        [hi[0][0], hi[1][1]],
    ];
    let mid = z1 - roof;
    // The glass band from the waist to the roof's edge.
    let g: Vec<u32> = bot.iter().map(|p| b.point([p[0], p[1], z0])).collect();
    let t: Vec<u32> = top.iter().map(|p| b.point([p[0], p[1], mid])).collect();
    for i in 0..4 {
        let j = (i + 1) % 4;
        b.quad("glass", g[i], g[j], t[j], t[i]);
    }
    b.tri("glass", g[0], g[2], g[1]);
    b.tri("glass", g[0], g[3], g[2]);
    // The roof, a slab over it.
    let r0 = [hi[0][0] - 0.03, hi[0][1] - 0.03];
    let r1 = [hi[1][0] + 0.03, hi[1][1] + 0.03];
    b.rbox("body", [r0[0], r0[1], mid - 0.02], [r1[0], r1[1], z1], 0.08);
    // Pillars at the corners, round and thick.
    for i in 0..4 {
        let (a, c) = (bot[i], top[i]);
        b.cone("body", [a[0], a[1], z0], [c[0], c[1], mid], 0.05, 0.045, 12);
    }
}

pub fn car(b: &mut Builder, p: &Params) {
    let l = p.get("length", 4.4);
    let w = p.get("width", 1.8);
    let h = p.get("height", 1.45);
    let style = p.style();
    let x = w / 2.0;
    let wheel = (h * 0.46).clamp(0.58, 0.84);
    let clearance = 0.15;
    let waist = if style == 3 { h * 0.6 } else { h * 0.55 };
    // The lower body, with its bonnet sloping a little towards the front.
    b.rbox(
        "body",
        [-x, 0.02, clearance + 0.05],
        [x, l - 0.02, waist],
        0.22,
    );
    b.rbox(
        "body",
        [-x + 0.05, l * 0.72, waist - 0.12],
        [x - 0.05, l - 0.1, waist + 0.04],
        0.14,
    );
    // Bumpers and lights.
    b.rbox(
        "trim",
        [-x + 0.04, -0.03, clearance],
        [x - 0.04, 0.16, clearance + 0.24],
        0.11,
    );
    b.rbox(
        "trim",
        [-x + 0.04, l - 0.16, clearance],
        [x - 0.04, l + 0.03, clearance + 0.24],
        0.11,
    );
    // Round lights, a little proud of the body.
    for side in [-1.0, 1.0] {
        let lx = side * (x - 0.26);
        b.ellipsoid("light", [lx, l - 0.03, waist - 0.12], [0.1, 0.05, 0.085]);
        b.ellipsoid("rear", [lx, 0.03, waist - 0.13], [0.09, 0.05, 0.07]);
    }
    // The cabin: windscreen and rear window lean, the roof is shorter.
    let (y0, y1, ry0, ry1) = match style {
        2 => (0.12 * l, 0.68 * l, 0.14 * l, 0.5 * l),
        3 => (0.1 * l, 0.72 * l, 0.12 * l, 0.6 * l),
        _ => (0.22 * l, 0.68 * l, 0.32 * l, 0.58 * l),
    };
    let inset = 0.08;
    cabin(
        b,
        [[-x + inset, y0], [x - inset, y1]],
        waist,
        [[-x + inset + 0.08, ry0], [x - inset - 0.08, ry1]],
        h,
        0.05,
    );
    // Wheels with their hubs, and mirrors.
    for yc in [0.18 * l + wheel / 2.0 - 0.12, 0.8 * l - wheel / 2.0 + 0.08] {
        for side in [-1.0, 1.0] {
            let xo = side * x;
            let xi = side * (x - 0.22);
            // A fat tyre with its round edge, and a domed hub.
            b.cone(
                "tyre",
                [xi, yc, wheel / 2.0],
                [xo + side * 0.03, yc, wheel / 2.0],
                wheel / 2.0,
                wheel / 2.0,
                28,
            );
            b.ellipsoid(
                "tyre",
                [xo + side * 0.03, yc, wheel / 2.0],
                [0.05, wheel * 0.46, wheel * 0.46],
            );
            b.ellipsoid(
                "hub",
                [xo + side * 0.07, yc, wheel / 2.0],
                [0.035, wheel * 0.24, wheel * 0.24],
            );
        }
    }
    // Round mirrors on short arms.
    for side in [-1.0, 1.0] {
        b.cone(
            "body",
            [side * (x - 0.05), y1 - 0.08, waist + 0.08],
            [side * (x + 0.06), y1 - 0.08, waist + 0.1],
            0.02,
            0.02,
            8,
        );
        b.ellipsoid(
            "body",
            [side * (x + 0.1), y1 - 0.08, waist + 0.11],
            [0.06, 0.04, 0.05],
        );
    }
}
