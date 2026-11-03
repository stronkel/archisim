//! The petrol station (gasolinera, of no brand): the canopy on four
//! columns with its coloured fascia and its lights underneath, two islands
//! with two pumps each (their screens, holsters and hoses), the shop of
//! glass with its sign, the tall price sign, the car wash with its
//! brushes, the post of air and water, the forecourt with its lines and
//! planters.

use super::*;
use crate::Params;

pub fn petrol_station(b: &mut Builder, p: &Params) {
    let seed = p.get("variant", 1.0).round().max(0.0) as u64;
    let _ = seed;
    let (cw, cd, ch) = (16.0, 11.0, 5.2);
    let cy = 14.0; // the canopy's centre from the back of the plot
                   // The forecourt.
    b.rbox("yard", [-16.0, 0.0, 0.0], [16.0, 24.0, 0.02], 0.0);
    // The canopy: columns, the slab, the fascia in two colours, lights.
    for (dx, dy) in [(-1.0, -1.0), (1.0, -1.0), (1.0, 1.0), (-1.0, 1.0)] {
        let c = [dx * (cw / 2.0 - 3.0), cy + dy * (cd / 2.0 - 2.5)];
        b.rbox(
            "brand",
            [c[0] - 0.25, c[1] - 0.25, 0.02],
            [c[0] + 0.25, c[1] + 0.25, ch],
            0.0,
        );
        b.rbox(
            "trim",
            [c[0] - 0.3, c[1] - 0.3, 0.02],
            [c[0] + 0.3, c[1] + 0.3, 0.9],
            0.0,
        );
    }
    let (x0, x1, y0, y1) = (-cw / 2.0, cw / 2.0, cy - cd / 2.0, cy + cd / 2.0);
    b.rbox("canopy", [x0, y0, ch], [x1, y1, ch + 0.25], 0.0);
    b.rbox(
        "brand",
        [x0 - 0.05, y0 - 0.05, ch + 0.25],
        [x1 + 0.05, y1 + 0.05, ch + 1.0],
        0.0,
    );
    b.rbox(
        "brand_b",
        [x0 - 0.08, y0 - 0.08, ch + 0.35],
        [x1 + 0.08, y1 + 0.08, ch + 0.55],
        0.0,
    );
    b.rbox(
        "trim",
        [x0 - 0.06, y0 - 0.06, ch + 1.0],
        [x1 + 0.06, y1 + 0.06, ch + 1.1],
        0.0,
    );
    for i in 0..4 {
        for j in 0..3 {
            let x = x0 + 2.0 + i as f64 * (cw - 4.0) / 3.0;
            let y = y0 + 2.0 + j as f64 * (cd - 4.0) / 2.0;
            b.rbox(
                "lamp",
                [x - 0.6, y - 0.3, ch - 0.04],
                [x + 0.6, y + 0.3, ch],
                0.0,
            );
        }
    }
    // The islands and their pumps.
    for dx in [-1.0, 1.0] {
        let ix = dx * (cw / 2.0 - 3.0);
        b.rbox(
            "trim",
            [ix - 0.7, cy - 3.2, 0.02],
            [ix + 0.7, cy + 3.2, 0.2],
            0.0,
        );
        b.rbox(
            "stripe",
            [ix - 0.72, cy - 3.22, 0.02],
            [ix + 0.72, cy - 3.0, 0.21],
            0.0,
        );
        b.rbox(
            "stripe",
            [ix - 0.72, cy + 3.0, 0.02],
            [ix + 0.72, cy + 3.22, 0.21],
            0.0,
        );
        for dy in [-1.4, 1.4] {
            pump(b, [ix, cy + dy]);
        }
        // The bollards at the islands' ends.
        for e in [-3.6, 3.6] {
            b.cyl("stripe", [ix, cy + e, 0.02], [ix, cy + e, 1.0], 0.08);
        }
    }
    // The shop at the back: glass to the forecourt, its sign.
    let (sw, sd, sh) = (13.0, 8.0, 3.8);
    let sx = -2.0;
    let sf = Face::sides(sw, sd).map(|f| Face {
        o: [f.o[0] + sx, f.o[1]],
        ..f
    });
    let front = sf[0];
    let door = Hole {
        s: (sw * 0.3, sw * 0.3 + 2.0),
        z: (0.05, 2.6),
    };
    let glass = Hole {
        s: (0.6, sw - 0.6),
        z: (0.05, 2.9),
    };
    wall(b, "wall", &front, (0.0, sh), 0.25, &[glass]);
    for f in &sf[1..] {
        wall(b, "wall", f, (0.0, sh), 0.25, &[]);
    }
    let m = ((glass.s.1 - glass.s.0) / 1.5).round() as usize;
    for i in 0..=m {
        let x = glass.s.0 + (glass.s.1 - glass.s.0) * i as f64 / m as f64;
        front.bx(b, "frame", (x - 0.04, x + 0.04), glass.z, (-0.15, -0.05));
    }
    front.bx(
        b,
        "frame",
        glass.s,
        (glass.z.1 - 0.06, glass.z.1),
        (-0.15, -0.05),
    );
    front.bx(
        b,
        "frame",
        glass.s,
        (door.z.1, door.z.1 + 0.06),
        (-0.15, -0.05),
    );
    front.bx(b, "glass", glass.s, glass.z, (-0.12, -0.11));
    front.bx(
        b,
        "frame",
        (door.s.0 + 0.98, door.s.0 + 1.02),
        (0.05, door.z.1),
        (-0.11, -0.04),
    );
    front.bx(
        b,
        "brand",
        (-0.05, sw + 0.05),
        (sh - 0.15, sh + 0.75),
        (-0.01, 0.15),
    );
    front.bx(
        b,
        "sign_text",
        (sw * 0.25, sw * 0.75),
        (sh + 0.05, sh + 0.55),
        (0.15, 0.17),
    );
    b.rbox(
        "interior",
        [sx - sw / 2.0 + 0.25, 0.25, 0.0],
        [sx + sw / 2.0 - 0.25, sd - 0.25, sh - 0.1],
        0.0,
    );
    b.rbox(
        "roof_floor",
        [sx - sw / 2.0, 0.0, sh - 0.1],
        [sx + sw / 2.0, sd, sh],
        0.0,
    );
    ac_unit(b, &sf[1], 2.0, 2.4);
    // A display of bottles and gas bottles by the door.
    for k in 0..4 {
        let c = front.at(door.s.1 + 0.6 + 0.4 * k as f64, 0.02, 0.4);
        b.cyl("gas", [c[0], c[1], 0.02], [c[0], c[1], 0.6], 0.15);
    }
    b.rbox(
        "metal",
        [sx + sw / 2.0 - 1.5, sd, 0.02],
        [sx + sw / 2.0 - 0.3, sd + 0.6, 1.4],
        0.0,
    );

    // The car wash beside the shop.
    let wx = sx + sw / 2.0 + 4.5;
    let wf = Face::sides(5.0, 8.0).map(|f| Face {
        o: [f.o[0] + wx, f.o[1]],
        ..f
    });
    for (k, f) in wf.iter().enumerate() {
        if k == 0 {
            continue;
        }
        let hs = if k == 2 {
            vec![Hole {
                s: (0.8, 4.2),
                z: (0.0, 3.4),
            }]
        } else {
            Vec::new()
        };
        wall(b, "wall", f, (0.0, 4.2), 0.2, &hs);
        f.bx(b, "brand", (-0.05, f.len + 0.05), (3.7, 4.2), (-0.01, 0.06));
    }
    wf[0].bx(b, "brand", (-0.05, 5.05), (3.7, 4.2), (-0.21, 0.06));
    b.rbox(
        "roof_floor",
        [wx - 2.5, 0.0, 4.2],
        [wx + 2.5, 8.0, 4.3],
        0.0,
    );
    // The gantry with its brushes.
    for dx in [-1.6, 1.6] {
        b.rbox(
            "metal",
            [wx + dx - 0.15, 3.6, 0.02],
            [wx + dx + 0.15, 4.4, 3.4],
            0.0,
        );
        b.cyl(
            "brush",
            [wx + dx * 0.6, 4.0, 0.3],
            [wx + dx * 0.6, 4.0, 3.0],
            0.45,
        );
    }
    b.rbox("metal", [wx - 1.75, 3.6, 3.4], [wx + 1.75, 4.4, 3.7], 0.0);
    b.cyl("brush", [wx - 1.4, 4.0, 3.05], [wx + 1.4, 4.0, 3.05], 0.32);
    b.rbox("stripe", [wx - 2.3, 0.2, 0.02], [wx + 2.3, 7.9, 0.025], 0.0);

    // The tall price sign by the road.
    let tp = [cw / 2.0 + 4.0, 22.0];
    b.rbox(
        "trim",
        [tp[0] - 0.9, tp[1] - 0.4, 0.02],
        [tp[0] + 0.9, tp[1] + 0.4, 0.7],
        0.0,
    );
    b.rbox(
        "brand",
        [tp[0] - 0.25, tp[1] - 0.15, 0.7],
        [tp[0] + 0.25, tp[1] + 0.15, 4.0],
        0.0,
    );
    b.rbox(
        "brand",
        [tp[0] - 1.0, tp[1] - 0.25, 4.0],
        [tp[0] + 1.0, tp[1] + 0.25, 8.0],
        0.0,
    );
    b.rbox(
        "brand_b",
        [tp[0] - 1.02, tp[1] - 0.27, 7.0],
        [tp[0] + 1.02, tp[1] + 0.27, 7.4],
        0.0,
    );
    for k in 0..4 {
        let z = 4.3 + 0.62 * k as f64;
        for s in [-1.0, 1.0] {
            b.rbox(
                "sign_text",
                [tp[0] - 0.85, tp[1] + s * 0.25 - 0.01, z],
                [tp[0] + 0.85, tp[1] + s * 0.25 + 0.01, z + 0.42],
                0.0,
            );
            b.rbox(
                "digits",
                [tp[0] + 0.05, tp[1] + s * 0.26 - 0.01, z + 0.06],
                [tp[0] + 0.78, tp[1] + s * 0.26 + 0.01, z + 0.36],
                0.0,
            );
        }
    }
    // The post of air and water, and planters.
    let ap = [-cw / 2.0 - 3.0, 20.0];
    b.rbox(
        "appliance",
        [ap[0] - 0.25, ap[1] - 0.2, 0.02],
        [ap[0] + 0.25, ap[1] + 0.2, 1.3],
        0.0,
    );
    b.rbox(
        "brand",
        [ap[0] - 0.27, ap[1] - 0.22, 1.3],
        [ap[0] + 0.27, ap[1] + 0.22, 1.5],
        0.0,
    );
    b.cone(
        "dark",
        [ap[0] + 0.25, ap[1], 1.0],
        [ap[0] + 0.6, ap[1], 0.5],
        0.015,
        0.015,
        6,
    );
    for x in [-14.0, -10.0, 10.0, 14.0] {
        b.rbox("plinth", [x - 1.2, 23.0, 0.02], [x + 1.2, 23.8, 0.5], 0.0);
        b.blob(
            "leaves",
            [x, 23.4, 0.75],
            [1.1, 0.0, 0.0],
            [0.0, 0.35, 0.0],
            [0.0, 0.0, 0.35],
        );
    }
    // The lines on the ground: lanes and the way out.
    for dx in [-1.0, 1.0] {
        b.rbox(
            "line",
            [dx * (cw / 2.0 - 3.0) - 2.8, cy - 5.5, 0.02],
            [dx * (cw / 2.0 - 3.0) - 2.7, cy + 5.5, 0.025],
            0.0,
        );
    }
}

/// A pump: its body, the screens on both faces, the price display, the
/// holsters with their nozzles and the hoses.
fn pump(b: &mut Builder, c: [f64; 2]) {
    let (hw, hd, h) = (0.55, 0.3, 2.1);
    b.rbox(
        "appliance",
        [c[0] - hw, c[1] - hd, 0.2],
        [c[0] + hw, c[1] + hd, h],
        0.0,
    );
    b.rbox(
        "brand",
        [c[0] - hw - 0.02, c[1] - hd - 0.02, h - 0.35],
        [c[0] + hw + 0.02, c[1] + hd + 0.02, h],
        0.0,
    );
    for s in [-1.0, 1.0] {
        let x = c[0] + s * (hw + 0.005);
        b.rbox(
            "screen",
            [x - 0.01, c[1] - 0.2, 1.25],
            [x + 0.01, c[1] + 0.2, 1.6],
            0.0,
        );
        b.rbox(
            "digits",
            [x - 0.012, c[1] - 0.18, 1.45],
            [x + 0.012, c[1] + 0.18, 1.56],
            0.0,
        );
        for (k, dy) in [-0.2, 0.0, 0.2].iter().enumerate() {
            let y = c[1] + dy;
            let role = ["nozzle_a", "nozzle_b", "dark"][k];
            b.rbox(
                "dark",
                [x - 0.01, y - 0.05, 0.85],
                [x + s * 0.08, y + 0.05, 1.05],
                0.0,
            );
            b.rbox(
                role,
                [x + s * 0.06, y - 0.03, 0.82],
                [x + s * 0.12, y + 0.03, 1.08],
                0.0,
            );
            // The hose down in a loop and back.
            b.cone(
                "dark",
                [x + s * 0.09, y, 0.82],
                [x + s * 0.3, y, 0.45],
                0.018,
                0.018,
                6,
            );
            b.cone(
                "dark",
                [x + s * 0.3, y, 0.45],
                [x + s * 0.02, y, 0.35],
                0.018,
                0.018,
                6,
            );
        }
    }
}
