//! A pair of semi-detached houses (pareadas): two houses of two floors
//! mirrored about their party wall under one hipped roof of tiles, each
//! with its porch on two columns over the door, a bay window downstairs,
//! a balcony upstairs, roller blinds and an awning; round them the
//! garden, split down the middle, with hedges, the gates and the drives.

use super::more::*;
use super::*;
use crate::Params;

const WALL: f64 = 0.3;
const REVEAL: f64 = 0.08;

pub fn semi_detached(b: &mut Builder, p: &Params) {
    let hw = p.get("house_width", 8.0).clamp(6.0, 12.0);
    let d = p.get("depth", 10.0).clamp(8.0, 14.0);
    let seed = p.get("variant", 1.0).round().max(0.0) as u64;
    let w = 2.0 * hw;
    let ground = 3.0;
    let top = ground + 2.85;
    let [front, right, back, left] = Face::sides(w, d);

    let mut fh = Vec::new();
    for i in 0..2 {
        let mirror = i == 1;
        let at = |s: f64| if mirror { w - s } else { s };
        let span = |a: f64, e: f64| {
            let (x, y) = (at(a), at(e));
            (x.min(y), x.max(y))
        };
        fh.push((
            Hole {
                s: span(0.9, 2.9),
                z: (0.5, 2.3),
            },
            0,
            i,
        )); // bay window
        fh.push((
            Hole {
                s: span(hw - 2.1, hw - 1.1),
                z: (0.15, 2.35),
            },
            1,
            i,
        )); // door
        fh.push((
            Hole {
                s: span(0.9, 2.5),
                z: (ground + 0.05, ground + 2.2),
            },
            2,
            i,
        )); // balcony door
        fh.push((
            Hole {
                s: span(hw - 2.3, hw - 1.0),
                z: (ground + 0.95, ground + 2.2),
            },
            3,
            i,
        ));
        fh.push((
            Hole {
                s: span(3.6, 4.4),
                z: (1.3, 2.1),
            },
            3,
            i,
        ));
    }
    let holes: Vec<Hole> = fh.iter().map(|h| h.0).collect();
    wall(b, "wall", &front, (0.0, top), WALL, &holes);
    let mut bh = Vec::new();
    for x in [1.5, hw - 2.5, hw + 1.5, w - 2.5] {
        bh.push(Hole {
            s: (x, x + 1.2),
            z: (0.05, 2.2),
        });
        bh.push(Hole {
            s: (x, x + 1.2),
            z: (ground + 0.95, ground + 2.2),
        });
    }
    wall(b, "wall", &back, (0.0, top), WALL, &bh);
    let mut sh = Vec::new();
    for x in [2.0, d - 3.0] {
        sh.push(Hole {
            s: (x, x + 1.0),
            z: (1.0, 2.2),
        });
        sh.push(Hole {
            s: (x, x + 1.0),
            z: (ground + 0.95, ground + 2.2),
        });
    }
    for f in [right, left] {
        wall(b, "wall", &f, (0.0, top), WALL, &sh);
    }
    core(b, w, d, WALL, (0.0, top));
    for f in [&front, &right, &back, &left] {
        f.bx(b, "plinth", (0.0, f.len), (0.0, 0.7), (-0.01, 0.03));
        f.bx(
            b,
            "trim",
            (0.0, f.len),
            (ground - 0.12, ground + 0.05),
            (-0.01, 0.04),
        );
        f.bx(b, "trim", (0.0, f.len), (top - 0.25, top), (-0.01, 0.06));
    }
    // The party wall marked down the front.
    front.bx(b, "trim", (hw - 0.1, hw + 0.1), (0.7, top), (-0.01, 0.05));

    for (j, (h, kind, i)) in fh.iter().enumerate() {
        let key = seed * 17 + j as u64;
        match kind {
            0 => bay_window(b, &front, h),
            1 => {
                panel_door(b, &front, *h, 1, 0.0, "door", "metal", REVEAL);
                porch(b, &front, h);
                wall_lantern(
                    b,
                    &front,
                    if *i == 0 { h.s.0 - 0.35 } else { h.s.1 + 0.35 },
                    h.z.1 - 0.1,
                );
            }
            2 => {
                let win = Win {
                    leaves: 2,
                    reveal: REVEAL,
                    sliding: true,
                    blind: 0.4 * unit(key, 1),
                    blind_box: true,
                    ..Default::default()
                };
                window(b, &front, *h, &win, "frame");
                let s = (h.s.0 - 0.3, h.s.1 + 0.3);
                balcony(b, &front, s, h.z.0, 0.6, "trim", "iron", false);
                if unit(key, 2) > 0.4 {
                    awning(b, &front, h.s, h.z.1 + 0.25, 1.0, 0.45);
                }
                standing_pot(b, front.at(s.1 - 0.25, h.z.0, 0.32), 0.75, key);
            }
            _ => {
                let win = Win {
                    leaves: 2,
                    reveal: REVEAL,
                    blind: [0.0, 0.3, 0.7][(unit(key, 3) * 3.0) as usize % 3],
                    blind_box: true,
                    curtain: true,
                    ..Default::default()
                };
                window(b, &front, *h, &win, "frame");
                sill(b, &front, *h, "trim", 0.05, REVEAL);
            }
        }
    }
    for (j, h) in bh.iter().enumerate() {
        let win = Win {
            leaves: 2,
            reveal: REVEAL,
            sliding: h.z.0 < 0.1,
            blind: 0.6 * unit(seed, 200 + j as u64),
            blind_box: true,
            ..Default::default()
        };
        window(b, &back, *h, &win, "frame");
    }
    for f in [&right, &left] {
        for (j, h) in sh.iter().enumerate() {
            let win = Win {
                leaves: 1,
                reveal: REVEAL,
                blind: 0.5 * unit(seed, 300 + j as u64),
                blind_box: true,
                ..Default::default()
            };
            window(b, f, *h, &win, "frame");
            sill(b, f, *h, "trim", 0.05, REVEAL);
            if h.z.0 > ground && j == 1 {
                ac_unit(b, f, (h.s.0 + h.s.1) / 2.0, h.z.0 - 0.75);
            }
        }
    }

    // The roof over both, the chimneys and the party wall's cap.
    hipped_roof(b, (-w / 2.0, w / 2.0), (0.0, d), top, 0.45, 0.6);
    for x in [-hw * 0.4, hw * 0.4] {
        let y = d * 0.3;
        let z = top + y.min(d / 2.0) * 0.45;
        chimney(b, [x, y], z - 0.3, z + 1.0);
    }

    // The gardens: lawn, the hedge between the two and round the front,
    // the gates, the paths and the drives.
    let gd = 4.5;
    b.rbox(
        "lawn",
        [-w / 2.0 - 2.5, -3.0, 0.0],
        [w / 2.0 + 2.5, d + gd, 0.02],
        0.0,
    );
    b.rbox("hedge", [-0.3, d + 0.2, 0.0], [0.3, d + gd - 0.1, 1.3], 0.0);
    for i in 0..2 {
        let sgn = if i == 0 { -1.0 } else { 1.0 };
        let door_x = sgn * 1.6;
        let drive_x = sgn * (hw - 1.9);
        b.rbox(
            "paving",
            [door_x - 0.6, d, 0.0],
            [door_x + 0.6, d + gd, 0.04],
            0.0,
        );
        b.rbox(
            "paving",
            [drive_x - 1.5, d, 0.0],
            [drive_x + 1.5, d + gd, 0.04],
            0.0,
        );
        // The front hedge, open at the gate and the drive.
        let edges = [
            (sgn * 0.3, door_x - sgn * 0.7),
            (door_x + sgn * 0.7, drive_x - sgn * 1.6),
            (drive_x + sgn * 1.6, sgn * (w / 2.0 + 2.5)),
        ];
        for (a, e) in edges {
            let (a, e) = (a.min(e), a.max(e));
            if e - a > 0.2 {
                b.rbox("hedge", [a, d + gd - 0.6, 0.0], [e, d + gd, 1.1], 0.0);
            }
        }
        for x in [door_x - 0.7, door_x + 0.55] {
            b.rbox(
                "plinth",
                [x, d + gd - 0.65, 0.0],
                [x + 0.15, d + gd + 0.05, 1.4],
                0.0,
            );
        }
        let gf = Face {
            o: [door_x - 0.55, d + gd - 0.3],
            u: [1.0, 0.0],
            len: 1.1,
        };
        railing(b, &gf, "iron", (0.0, 1.1), 0.05, 0.0, 1.1, 0.11);
        fruit_tree(
            b,
            [sgn * (w / 2.0 + 1.0), d + 2.0],
            3.0,
            seed + i as u64,
            false,
        );
    }
}

/// A bay window: a box out of the hole with glass on three sides, its
/// panel under the glass and a little roof of tiles.
fn bay_window(b: &mut Builder, f: &Face, h: &Hole) {
    let out = 0.55;
    let (s0, s1) = (h.s.0, h.s.1);
    let (z0, z1) = h.z;
    f.bx(
        b,
        "trim",
        (s0 - 0.05, s1 + 0.05),
        (z0 - 0.1, z0 + 0.4),
        (-0.01, out),
    );
    f.bx(
        b,
        "frame",
        (s0, s1),
        (z0 + 0.4, z0 + 0.46),
        (out - 0.07, out),
    );
    f.bx(b, "frame", (s0, s1), (z1 - 0.06, z1), (out - 0.07, out));
    for k in 0..=3 {
        let x = s0 + (s1 - s0) * k as f64 / 3.0;
        f.bx(
            b,
            "frame",
            ((x - 0.03).max(s0), (x + 0.03).min(s1)),
            (z0 + 0.4, z1),
            (out - 0.07, out),
        );
    }
    f.bx(
        b,
        "glass",
        (s0, s1),
        (z0 + 0.46, z1 - 0.06),
        (out - 0.045, out - 0.035),
    );
    for x in [s0, s1] {
        let side = Face {
            o: f.at(x, 0.0, 0.0).map_xy(),
            u: f.out(),
            len: out,
        };
        side.bx(
            b,
            "glass",
            (0.0, out - 0.05),
            (z0 + 0.46, z1 - 0.06),
            (-0.006, 0.006),
        );
        side.bx(b, "frame", (0.0, out), (z1 - 0.06, z1), (-0.03, 0.03));
    }
    f.leaning(
        b,
        "roof",
        (s0 - 0.1, s1 + 0.1),
        (z1 + 0.35, -0.01),
        (z1, out + 0.12),
        0.05,
    );
    let n = ((s1 - s0 + 0.2) / 0.21).round() as usize;
    for k in 0..n {
        let x = s0 - 0.1 + (s1 - s0 + 0.2) * (k as f64 + 0.5) / n as f64;
        b.cone(
            "tiles",
            f.at(x, z1 + 0.43, -0.01),
            f.at(x, z1 + 0.08, out + 0.16),
            0.055,
            0.05,
            6,
        );
    }
    b.fine.insert("tiles");
}

/// A porch over a door: a slab on two columns with a roof of tiles.
fn porch(b: &mut Builder, f: &Face, h: &Hole) {
    let out = 1.4;
    let (s0, s1) = (h.s.0 - 0.6, h.s.1 + 0.6);
    let z = h.z.1 + 0.25;
    f.bx(b, "step", (s0, s1), (0.0, 0.15), (-0.01, out));
    for x in [s0 + 0.15, s1 - 0.15] {
        let c = f.at(x, 0.0, out - 0.15);
        b.rbox(
            "trim",
            [c[0] - 0.13, c[1] - 0.13, 0.15],
            [c[0] + 0.13, c[1] + 0.13, 0.35],
            0.0,
        );
        b.cyl("trim", [c[0], c[1], 0.35], [c[0], c[1], z - 0.15], 0.1);
        b.rbox(
            "trim",
            [c[0] - 0.14, c[1] - 0.14, z - 0.15],
            [c[0] + 0.14, c[1] + 0.14, z],
            0.0,
        );
    }
    f.bx(
        b,
        "trim",
        (s0 - 0.05, s1 + 0.05),
        (z, z + 0.2),
        (-0.01, out + 0.05),
    );
    f.leaning(
        b,
        "roof",
        (s0 - 0.1, s1 + 0.1),
        (z + 0.6, -0.01),
        (z + 0.2, out + 0.15),
        0.05,
    );
    let n = ((s1 - s0 + 0.2) / 0.21).round() as usize;
    for k in 0..n {
        let x = s0 - 0.1 + (s1 - s0 + 0.2) * (k as f64 + 0.5) / n as f64;
        b.cone(
            "tiles",
            f.at(x, z + 0.68, -0.01),
            f.at(x, z + 0.27, out + 0.2),
            0.055,
            0.05,
            6,
        );
    }
    b.fine.insert("tiles");
}
