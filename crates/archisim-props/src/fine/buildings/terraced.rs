//! A row of terraced houses of a developer (adosados de promotora): two
//! floors and the roof, each house with its front garden behind a low
//! wall with railings and a gate, the garage's sectional door, the porch
//! with its little roof of tiles over the entrance, aluminium windows
//! with roller blinds in boxes, a balcony with a glass rail upstairs, the
//! roof of tiles falling to the street and to the back garden with the
//! fire walls standing a little over it, and a chimney each.

use super::*;
use crate::Params;

const WALL: f64 = 0.3;
const REVEAL: f64 = 0.08;

pub fn terraced_row(b: &mut Builder, p: &Params) {
    let houses = p.get("houses", 4.0).round().clamp(1.0, 10.0) as usize;
    let hw = p.get("house_width", 6.0).clamp(4.5, 9.0);
    let d = p.get("depth", 10.0).clamp(7.0, 16.0);
    let garden = p.get("garden", 3.0).clamp(0.0, 8.0);
    let seed = p.get("variant", 1.0).round().max(0.0) as u64;
    let w = hw * houses as f64;
    let ground = 3.0;
    let upper = 2.85;
    let top = ground + upper;
    let [front, right, back, left] = Face::sides(w, d);

    let mut holes = Vec::new();
    let mut back_holes = Vec::new();
    for i in 0..houses {
        let a = hw * i as f64;
        let mirror = i % 2 == 1;
        let at = |s: f64| if mirror { a + hw - s } else { a + s };
        let span = |s0: f64, s1: f64| {
            let (x, y) = (at(s0), at(s1));
            (x.min(y), x.max(y))
        };
        holes.push(Hole {
            s: span(0.5, 3.1),
            z: (0.0, 2.3),
        });
        holes.push(Hole {
            s: span(3.7, 4.7),
            z: (0.12, 2.3),
        });
        if hw > 5.8 {
            holes.push(Hole {
                s: span(5.0, hw - 0.3),
                z: (1.0, 2.2),
            });
        }
        holes.push(Hole {
            s: span(0.8, 2.6),
            z: (ground + 0.02, ground + 2.2),
        });
        holes.push(Hole {
            s: span(3.6, 4.8),
            z: (ground + 0.95, ground + 2.2),
        });
        back_holes.push(Hole {
            s: span(0.8, 3.2),
            z: (0.02, 2.2),
        });
        back_holes.push(Hole {
            s: span(4.0, 5.0),
            z: (1.0, 2.2),
        });
        back_holes.push(Hole {
            s: span(0.9, 2.1),
            z: (ground + 0.95, ground + 2.2),
        });
        back_holes.push(Hole {
            s: span(3.6, 4.8),
            z: (ground + 0.95, ground + 2.2),
        });
    }
    wall(b, "wall", &front, (0.0, top), WALL, &holes);
    wall(b, "wall", &back, (0.0, top), WALL, &back_holes);
    for f in [right, left] {
        wall(b, "wall", &f, (0.0, top), WALL, &[]);
    }
    core(b, w, d, WALL, (0.0, top));
    // The stone plinth and a band at the floor.
    for f in [&front, &back] {
        f.bx(b, "plinth", (0.0, f.len), (0.0, 0.6), (-0.01, 0.03));
        f.bx(
            b,
            "trim",
            (0.0, f.len),
            (ground - 0.15, ground + 0.02),
            (-0.01, 0.04),
        );
    }

    for (j, h) in holes.iter().enumerate() {
        let key = seed * 1000 + j as u64 * 7;
        let wide = h.s.1 - h.s.0;
        let up = h.z.0 > ground - 0.1;
        let is_garage = !up && wide > 2.0;
        let is_door = !up && h.z.0 > 0.05 && h.z.1 > 2.25 && wide < 1.2;
        if is_garage {
            garage(b, &front, *h, unit(seed, key) > 0.8);
            continue;
        }
        if is_door {
            entrance(b, &front, *h);
            continue;
        }
        let balcony_door = up && h.z.0 < ground + 0.1;
        let win = Win {
            leaves: 2,
            reveal: REVEAL,
            sliding: balcony_door,
            blind: [0.0, 0.3, 0.6, 1.0][(unit(seed, key + 1) * 4.0) as usize % 4],
            blind_box: true,
            curtain: unit(seed, key + 2) > 0.5,
            ..Default::default()
        };
        window(b, &front, *h, &win, "frame");
        if balcony_door {
            let s = (h.s.0 - 0.3, h.s.1 + 0.3);
            front.bx(b, "trim", s, (h.z.0 - 0.18, h.z.0), (-0.01, 0.7));
            glass_rail(b, &front, s, h.z.0, 0.7);
            if unit(seed, key + 3) > 0.4 {
                standing_pot(b, front.at(s.1 - 0.25, h.z.0, 0.4), 0.8, key);
            }
            if unit(seed, key + 4) > 0.55 {
                awning(b, &front, h.s, h.z.1 + 0.25, 0.9, 0.5);
            }
        } else {
            sill(b, &front, *h, "trim", 0.05, REVEAL);
        }
    }
    for (j, h) in back_holes.iter().enumerate() {
        let key = seed * 3000 + j as u64 * 5;
        let win = Win {
            leaves: 2,
            reveal: REVEAL,
            sliding: h.z.0 < 0.1,
            blind: 0.7 * unit(seed, key),
            blind_box: true,
            ..Default::default()
        };
        window(b, &back, *h, &win, "frame");
        if h.z.0 > 0.1 {
            sill(b, &back, *h, "trim", 0.05, REVEAL);
        }
        if h.z.0 > ground && unit(seed, key + 1) > 0.8 {
            ac_unit(b, &back, (h.s.0 + h.s.1) / 2.0, h.z.0 - 0.7);
        }
    }

    // The front gardens: the low wall with its railing, a gate to the
    // door and the drive to the garage; the dividing walls.
    if garden > 0.8 {
        let gf = Face {
            o: [-w / 2.0, d + garden],
            u: [1.0, 0.0],
            len: w,
        };
        for i in 0..houses {
            let a = hw * i as f64;
            let mirror = i % 2 == 1;
            let at = |s: f64| if mirror { a + hw - s } else { a + s };
            let span = |s0: f64, s1: f64| {
                let (x, y) = (at(s0), at(s1));
                (x.min(y), x.max(y))
            };
            let drive = span(0.4, 3.2);
            let gate = span(3.75, 4.65);
            let mut cuts = [drive, gate];
            cuts.sort_by(|p, q| p.0.total_cmp(&q.0));
            let mut s = a;
            for (c0, c1) in cuts
                .iter()
                .copied()
                .chain(std::iter::once((a + hw, a + hw)))
            {
                if c0 - s > 0.05 {
                    gf.bx(b, "plinth", (s, c0), (0.0, 0.6), (-0.2, 0.0));
                    gf.bx(b, "trim", (s - 0.02, c0 + 0.02), (0.6, 0.65), (-0.22, 0.02));
                    railing(
                        b,
                        &gf,
                        "iron",
                        (s + 0.05, c0 - 0.05),
                        0.65,
                        -0.1,
                        0.85,
                        0.12,
                    );
                }
                s = c1;
            }
            // The gate between two pillars.
            for x in [gate.0 - 0.15, gate.1] {
                gf.bx(b, "plinth", (x, x + 0.15), (0.0, 1.6), (-0.2, 0.0));
                gf.bx(b, "trim", (x - 0.02, x + 0.17), (1.6, 1.66), (-0.22, 0.02));
            }
            railing(
                b,
                &gf,
                "iron",
                (gate.0 + 0.02, gate.1 - 0.02),
                0.05,
                -0.1,
                1.35,
                0.11,
            );
            // The path to the door and the drive.
            b.rbox(
                "paving",
                [gf.o[0] + gate.0, d, 0.0],
                [gf.o[0] + gate.1, d + garden - 0.2, 0.03],
                0.0,
            );
            b.rbox(
                "paving",
                [gf.o[0] + drive.0, d, 0.0],
                [gf.o[0] + drive.1, d + garden, 0.03],
                0.0,
            );
            let lawn = if mirror {
                (a + 0.15, gate.0 - 0.05)
            } else {
                (gate.1 + 0.05, a + hw - 0.15)
            };
            let lawn = if lawn.1 - lawn.0 < 0.3 {
                (drive.1 + 0.05, gate.0 - 0.05)
            } else {
                lawn
            };
            if lawn.1 - lawn.0 > 0.3 {
                b.rbox(
                    "lawn",
                    [gf.o[0] + lawn.0, d, 0.0],
                    [gf.o[0] + lawn.1, d + garden - 0.2, 0.04],
                    0.0,
                );
                let c = [gf.o[0] + (lawn.0 + lawn.1) / 2.0, d + garden * 0.55, 0.04];
                if unit(seed, 50 + i as u64) > 0.3 {
                    b.blob(
                        "leaves",
                        [c[0], c[1], 0.45],
                        [0.45, 0.0, 0.0],
                        [0.0, 0.4, 0.0],
                        [0.0, 0.0, 0.42],
                    );
                }
            }
            // The wall between this garden and the next.
            if i + 1 < houses {
                let x = gf.o[0] + a + hw;
                b.rbox(
                    "plinth",
                    [x - 0.08, d, 0.0],
                    [x + 0.08, d + garden, 1.0],
                    0.0,
                );
            }
        }
    }

    // The roof, the fire walls over it and the chimneys.
    let pitch = 0.5;
    tile_roof(
        b,
        (-w / 2.0, w / 2.0),
        (0.0, d),
        top,
        pitch,
        (0.5, 0.35),
        (true, true),
    );
    let rise = d / 2.0 * pitch;
    for i in 1..houses {
        let x = -w / 2.0 + hw * i as f64;
        let f = Face {
            o: [x + 0.12, 0.0],
            u: [0.0, 1.0],
            len: d,
        };
        let pts = [
            (0.0, top),
            (d, top),
            (d / 2.0, top + rise + 0.28),
            (0.0, top + 0.28),
        ];
        let pts = [pts[0], pts[1], (d, top + 0.28), pts[2], pts[3]];
        f.prism(b, "wall", &pts, (0.0, 0.24));
    }
    for i in 0..houses {
        let x = -w / 2.0 + hw * (i as f64 + if i % 2 == 1 { 0.25 } else { 0.75 });
        let y = d * 0.3;
        let zr = top + y * pitch;
        chimney(b, [x, y], zr - 0.3, zr + 1.0);
    }
    downpipe(b, &front, 0.15, top);
    downpipe(b, &front, w - 0.15, top);
}

/// The sectional door of a garage: its panels with their grooves and the
/// handle; `open` lifted.
fn garage(b: &mut Builder, f: &Face, h: Hole, open: bool) {
    let fd = (-REVEAL - 0.05, -REVEAL);
    f.bx(b, "frame", (h.s.0, h.s.0 + 0.05), h.z, fd);
    f.bx(b, "frame", (h.s.1 - 0.05, h.s.1), h.z, fd);
    f.bx(b, "frame", h.s, (h.z.1 - 0.06, h.z.1), fd);
    let low = if open { h.z.1 - 0.6 } else { h.z.0 };
    let (a, e) = (h.s.0 + 0.05, h.s.1 - 0.05);
    f.bx(
        b,
        "garage",
        (a, e),
        (low, h.z.1 - 0.06),
        (-REVEAL - 0.04, -REVEAL - 0.01),
    );
    let mut z = h.z.1 - 0.06 - 0.5;
    while z > low + 0.05 {
        f.bx(
            b,
            "garage",
            (a, e),
            (z - 0.012, z + 0.012),
            (-REVEAL - 0.01, -REVEAL + 0.0),
        );
        z -= 0.5;
    }
    if !open {
        let c = (a + e) / 2.0;
        f.bx(
            b,
            "metal",
            (c - 0.12, c + 0.12),
            (0.95, 1.0),
            (-REVEAL - 0.01, -REVEAL + 0.02),
        );
    }
}

/// The entrance: a flush door with a side light, the step and the porch
/// with its little roof of tiles on two brackets.
fn entrance(b: &mut Builder, f: &Face, h: Hole) {
    panel_door(b, f, h, 1, 0.0, "door", "metal", REVEAL);
    f.bx(
        b,
        "trim",
        (h.s.0 - 0.25, h.s.1 + 0.25),
        (0.0, h.z.0),
        (-REVEAL, 0.9),
    );
    let z = h.z.1 + 0.25;
    let (s0, s1) = (h.s.0 - 0.45, h.s.1 + 0.45);
    f.leaning(b, "roof", (s0, s1), (z + 0.35, 0.0), (z, 1.0), 0.06);
    let n = ((s1 - s0) / 0.21).round().max(1.0) as usize;
    for i in 0..n {
        let x = s0 + (s1 - s0) * (i as f64 + 0.5) / n as f64;
        b.cone(
            "tiles",
            f.at(x, z + 0.42, 0.0),
            f.at(x, z + 0.09, 1.05),
            0.055,
            0.05,
            6,
        );
    }
    for x in [s0 + 0.1, s1 - 0.1] {
        let side = Face {
            o: f.at(x, 0.0, 0.0).map_xy(),
            u: f.out(),
            len: 1.0,
        };
        side.prism(
            b,
            "wood",
            &[(-0.01, z + 0.05), (0.85, z + 0.07), (-0.01, z - 0.5)],
            (-0.04, 0.04),
        );
    }
    wall_lantern(b, f, h.s.1 + 0.35, h.z.1 - 0.2);
}

/// A rail of glass on a balcony: glass in posts with a handrail.
fn glass_rail(b: &mut Builder, f: &Face, s: (f64, f64), z: f64, out: f64) {
    let d = out - 0.05;
    f.bx(
        b,
        "glass",
        (s.0 + 0.03, s.1 - 0.03),
        (z + 0.05, z + 0.95),
        (d - 0.01, d),
    );
    f.bx(b, "metal", s, (z + 0.95, z + 1.0), (d - 0.03, d + 0.02));
    for x in [s.0 + 0.03, s.1 - 0.03] {
        f.bx(
            b,
            "metal",
            (x - 0.025, x + 0.025),
            (z, z + 0.95),
            (d - 0.03, d + 0.02),
        );
        let side = Face {
            o: f.at(x, 0.0, 0.0).map_xy(),
            u: f.out(),
            len: d,
        };
        side.bx(b, "glass", (0.0, d), (z + 0.05, z + 0.95), (-0.005, 0.005));
        side.bx(b, "metal", (0.0, d), (z + 0.95, z + 1.0), (-0.025, 0.025));
    }
}
