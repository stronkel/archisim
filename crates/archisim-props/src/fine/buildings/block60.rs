//! The block of flats of the 1960s and 70s (bloque desarrollista): brick
//! walls between white slab edges, terraces standing out with their brick
//! parapets and iron tubes (some closed in aluminium, some with the green
//! awning down), roller blinds at every height, the staircase's windows
//! between floors, air conditioners and clothes lines, the shops on the
//! ground floor with their shutters and signs, the doorway in the middle,
//! and on the flat roof the stair's hut, water tanks and a crowd of
//! aerials.

use super::*;
use crate::Params;

const WALL: f64 = 0.3;
const REVEAL: f64 = 0.1;

#[derive(Clone, Copy, PartialEq)]
enum Slot {
    Small,
    Window,
    Terrace,
}

pub fn block_1960s(b: &mut Builder, p: &Params) {
    let w = p.get("width", 18.0).clamp(9.0, 40.0);
    let d = p.get("depth", 12.0).clamp(8.0, 24.0);
    let floors = p.get("floors", 6.0).round().clamp(3.0, 12.0) as usize;
    let seed = p.get("variant", 1.0).round().max(0.0) as u64;
    let shops = p.flag("shops", true);
    let ground = 3.6;
    let upper = 2.85;
    let level = |k: usize| {
        if k == 0 {
            0.0
        } else {
            ground + upper * (k - 1) as f64
        }
    };
    let roof_z = level(floors);
    let [front, right, back, left] = Face::sides(w, d);

    // The front's slots, from the staircase outwards, mirrored.
    let stair = 1.8;
    let half = (w - stair) / 2.0;
    let mut slots: Vec<(Slot, f64)> = Vec::new();
    let mut used = 0.0;
    let wanted = [
        (Slot::Small, 1.1),
        (Slot::Window, 1.9),
        (Slot::Terrace, 3.4),
        (Slot::Window, 1.9),
        (Slot::Window, 1.9),
        (Slot::Terrace, 3.4),
        (Slot::Small, 1.1),
        (Slot::Window, 1.9),
    ];
    for (k, l) in wanted.iter().cycle().take(40) {
        if used + l > half - 0.3 {
            break;
        }
        slots.push((*k, *l));
        used += l;
    }
    let spare = (half - used) / (slots.len() as f64 + 1.0);
    // The slots on the front: (kind, s0, s1) on both halves.
    let mut placed: Vec<(Slot, f64, f64)> = Vec::new();
    let mut s = w / 2.0 + stair / 2.0 + spare;
    for (k, l) in &slots {
        placed.push((*k, s, s + l));
        placed.push((*k, w - s - l, w - s));
        s += l + spare;
    }

    // The holes of the floors above.
    let mut holes = Vec::new();
    let mut parts: Vec<(usize, Slot, Hole)> = Vec::new();
    for k in 1..floors {
        let zf = level(k);
        for (kind, s0, s1) in &placed {
            let c = (s0 + s1) / 2.0;
            let h = match kind {
                Slot::Small => Hole {
                    s: (c - 0.32, c + 0.32),
                    z: (zf + 1.35, zf + 2.15),
                },
                Slot::Window => Hole {
                    s: (c - 0.62, c + 0.62),
                    z: (zf + 0.95, zf + 2.15),
                },
                Slot::Terrace => Hole {
                    s: (c - 0.8, c + 0.8),
                    z: (zf + 0.02, zf + 2.15),
                },
            };
            holes.push(h);
            parts.push((k, *kind, h));
        }
        // The staircase window, half a floor up.
        let zs = zf - upper / 2.0 + 1.1;
        if k > 1 {
            holes.push(Hole {
                s: (w / 2.0 - 0.45, w / 2.0 + 0.45),
                z: (zs, zs + 1.0),
            });
        }
    }
    // The ground floor: the doorway and the shops.
    let portal = Hole {
        s: (w / 2.0 - 0.95, w / 2.0 + 0.95),
        z: (0.15, 2.9),
    };
    holes.push(portal);
    let mut shop_holes = Vec::new();
    if shops {
        for (a, e) in [(0.6, w / 2.0 - 1.6), (w / 2.0 + 1.6, w - 0.6)] {
            if e - a < 2.0 {
                continue;
            }
            let n = ((e - a) / 5.5).ceil() as usize;
            let l = (e - a) / n as f64;
            for i in 0..n {
                let h = Hole {
                    s: (a + l * i as f64 + 0.3, a + l * (i + 1) as f64 - 0.3),
                    z: (0.0, 3.2),
                };
                shop_holes.push(h);
                holes.push(h);
            }
        }
    } else {
        for (kind, s0, s1) in &placed {
            if *kind == Slot::Window {
                let c = (s0 + s1) / 2.0;
                let h = Hole {
                    s: (c - 0.62, c + 0.62),
                    z: (1.2, 2.5),
                };
                holes.push(h);
                parts.push((0, Slot::Window, h));
            }
        }
    }

    // The walls: the ground floor clad, brick above.
    let ground_holes: Vec<Hole> = holes.iter().copied().filter(|h| h.z.0 < ground).collect();
    let upper_holes: Vec<Hole> = holes
        .iter()
        .copied()
        .filter(|h| h.z.0 >= ground - 0.01)
        .collect();
    wall(b, "ground_clad", &front, (0.0, ground), WALL, &ground_holes);
    wall(b, "brick", &front, (ground, roof_z), WALL, &upper_holes);
    let mut back_holes = Vec::new();
    let nb = ((w - 1.0) / 2.4).floor().max(2.0) as usize;
    for k in 0..floors {
        let zf = level(k);
        for i in 0..nb {
            let c = 0.5 + (w - 1.0) * (i as f64 + 0.5) / nb as f64;
            let small = (i + k) % 3 == 1;
            back_holes.push(if small {
                Hole {
                    s: (c - 0.32, c + 0.32),
                    z: (zf + 1.35, zf + 2.15),
                }
            } else {
                Hole {
                    s: (c - 0.6, c + 0.6),
                    z: (zf + 0.95, zf + 2.15),
                }
            });
        }
    }
    wall(b, "brick", &back, (0.0, roof_z), WALL, &back_holes);
    for f in [right, left] {
        wall(b, "party", &f, (0.0, roof_z), WALL, &[]);
    }
    core(b, w, d, WALL, (0.0, roof_z));

    // The slab edges, white, on the front and the back.
    for k in 1..=floors {
        let z = level(k);
        for f in [&front, &back] {
            f.bx(b, "slab_edge", (0.0, f.len), (z - 0.3, z), (-0.01, 0.025));
        }
    }

    // The carpentry and what hangs on the front.
    for (j, (k, kind, h)) in parts.iter().enumerate() {
        let key = 100 + j as u64 * 11;
        let r = unit(seed, key);
        let blind = if r < 0.25 {
            0.0
        } else if r < 0.5 {
            0.25
        } else if r < 0.8 {
            0.55
        } else {
            0.95
        };
        match kind {
            Slot::Small => {
                let win = Win {
                    leaves: 1,
                    reveal: REVEAL,
                    ..Default::default()
                };
                window(b, &front, *h, &win, "frame");
                sill(b, &front, *h, "slab_edge", 0.04, REVEAL);
                if unit(seed, key + 1) > 0.55 {
                    clothes_line(
                        b,
                        &front,
                        (h.s.0 - 0.35, h.s.1 + 0.35),
                        h.z.0 - 0.05,
                        seed + key,
                    );
                }
            }
            Slot::Window => {
                let win = Win {
                    leaves: 2,
                    reveal: REVEAL,
                    blind,
                    curtain: unit(seed, key + 2) > 0.5,
                    sliding: unit(seed, key + 3) > 0.6,
                    ..Default::default()
                };
                window(b, &front, *h, &win, "frame");
                sill(b, &front, *h, "slab_edge", 0.05, REVEAL);
                let r = unit(seed, key + 4);
                if r > 0.7 && *k > 0 {
                    awning(b, &front, h.s, h.z.1 + 0.18, 0.75, 0.55);
                } else if r < 0.25 && *k > 0 {
                    ac_unit(b, &front, (h.s.0 + h.s.1) / 2.0 + 0.1, h.z.0 - 0.75);
                }
            }
            Slot::Terrace => {
                let win = Win {
                    leaves: 2,
                    reveal: REVEAL,
                    blind: blind * 0.8,
                    sliding: true,
                    ..Default::default()
                };
                window(b, &front, *h, &win, "frame");
                terrace(b, &front, h, *k, floors, level(*k), upper, seed + key);
            }
        }
    }
    // The staircase's windows: fixed, with a lattice.
    for k in 2..floors {
        let zs = level(k) - upper / 2.0 + 1.1;
        let h = Hole {
            s: (w / 2.0 - 0.45, w / 2.0 + 0.45),
            z: (zs, zs + 1.0),
        };
        let win = Win {
            leaves: 1,
            reveal: REVEAL,
            bars: 2,
            ..Default::default()
        };
        window(b, &front, h, &win, "frame");
        sill(b, &front, h, "slab_edge", 0.04, REVEAL);
    }

    // The doorway: aluminium and glass, its steps, the canopy.
    let fd = (-REVEAL - 0.06, -REVEAL);
    let (s0, s1) = portal.s;
    for x in [s0, s0 + 0.45, s1 - 0.45, s1] {
        let (a, e) = if x == s1 {
            (x - 0.05, x)
        } else {
            (x, x + 0.05)
        };
        front.bx(b, "frame", (a, e), portal.z, fd);
    }
    front.bx(b, "frame", (s0, s1), (portal.z.1 - 0.05, portal.z.1), fd);
    front.bx(b, "frame", (s0, s1), (2.25, 2.3), fd);
    front.bx(
        b,
        "glass",
        (s0, s1),
        (portal.z.0, portal.z.1),
        (-REVEAL - 0.04, -REVEAL - 0.03),
    );
    front.bx(
        b,
        "brass",
        (w / 2.0 - 0.05, w / 2.0 - 0.02),
        (0.95, 1.25),
        (-REVEAL - 0.03, -REVEAL + 0.0),
    );
    front.bx(
        b,
        "step",
        (s0 - 0.2, s1 + 0.2),
        (0.0, 0.15),
        (-REVEAL, 0.35),
    );
    front.bx(
        b,
        "slab_edge",
        (s0 - 0.5, s1 + 0.5),
        (portal.z.1 + 0.15, portal.z.1 + 0.3),
        (-0.01, 1.1),
    );
    for (i, h) in shop_holes.iter().enumerate() {
        let r = unit(seed, 300 + i as u64);
        shop_front(
            b,
            &front,
            *h,
            if r < 0.4 {
                0.0
            } else if r < 0.75 {
                0.3
            } else {
                1.0
            },
            true,
            "frame",
            REVEAL,
        );
        if unit(seed, 310 + i as u64) > 0.5 {
            awning(b, &front, (h.s.0 + 0.2, h.s.1 - 0.2), 3.35, 1.3, 0.75);
        }
    }

    // The back: blinds, clothes lines and air conditioners.
    for (j, h) in back_holes.iter().enumerate() {
        let key = 500 + j as u64 * 5;
        let small = h.s.1 - h.s.0 < 1.0;
        let win = Win {
            leaves: if small { 1 } else { 2 },
            reveal: REVEAL,
            blind: if small { 0.0 } else { 0.6 * unit(seed, key) },
            sliding: !small,
            ..Default::default()
        };
        window(b, &back, *h, &win, "frame");
        sill(b, &back, *h, "slab_edge", 0.04, REVEAL);
        if small && unit(seed, key + 1) > 0.25 && h.z.0 > 1.5 {
            clothes_line(
                b,
                &back,
                (h.s.0 - 0.5, h.s.1 + 0.5),
                h.z.0 - 0.05,
                seed + key,
            );
        } else if !small && unit(seed, key + 2) > 0.75 && h.z.0 > 1.5 {
            ac_unit(b, &back, (h.s.0 + h.s.1) / 2.0, h.z.0 - 0.75);
        }
    }
    downpipe(b, &back, 0.3, roof_z);
    downpipe(b, &back, w - 0.3, roof_z);

    // The flat roof: its floor, the parapet, the stair's hut, tanks,
    // aerials and washing.
    let z = roof_z;
    b.rbox(
        "roof_floor",
        [-w / 2.0 + 0.1, 0.1, z - 0.2],
        [w / 2.0 - 0.1, d - 0.1, z + 0.03],
        0.0,
    );
    parapet(b, w, d, z, 1.0, "brick", "slab_edge");
    let hut = [3.2, 4.2, 2.6];
    let hc = [0.0, d * 0.45];
    let hf = Face::sides(hut[0], hut[1]);
    let tr = |f: &Face| Face {
        o: [f.o[0] + hc[0], f.o[1] + hc[1] - hut[1] / 2.0],
        ..*f
    };
    let door_h = Hole {
        s: (1.0, 1.9),
        z: (z + 0.03, z + 2.1),
    };
    for (k, f) in hf.iter().enumerate() {
        let f = tr(f);
        if k == 0 {
            wall(b, "party", &f, (z, z + hut[2]), 0.2, &[door_h]);
            panel_door(b, &f, door_h, 1, 0.0, "shutter", "metal", 0.06);
        } else {
            wall(b, "party", &f, (z, z + hut[2]), 0.2, &[]);
        }
        f.bx(
            b,
            "slab_edge",
            (-0.05, f.len + 0.05),
            (z + hut[2], z + hut[2] + 0.12),
            (-0.2, 0.06),
        );
    }
    b.rbox(
        "interior",
        [hc[0] - hut[0] / 2.0 + 0.2, hc[1] - hut[1] / 2.0 + 0.2, z],
        [
            hc[0] + hut[0] / 2.0 - 0.2,
            hc[1] + hut[1] / 2.0 - 0.2,
            z + hut[2],
        ],
        0.0,
    );
    water_tank(b, [hc[0] + 0.6, hc[1] - 0.8], z + hut[2] + 0.12);
    let n_aer = (w / 3.0).round().max(2.0) as usize;
    for i in 0..n_aer {
        let x = -w / 2.0
            + 1.0
            + (w - 2.0) * (i as f64 + unit(seed, 600 + i as u64) * 0.6) / n_aer as f64;
        let y = d * (0.15 + 0.7 * unit(seed, 620 + i as u64));
        if (x - hc[0]).abs() < hut[0] / 2.0 + 0.6 && (y - hc[1]).abs() < hut[1] / 2.0 + 0.6 {
            continue;
        }
        aerial(b, [x, y], z + 0.03, seed + i as u64 * 13);
    }
    roof_clothes(
        b,
        [-w / 2.0 + 1.2, d * 0.75],
        [-w / 2.0 + 1.2 + (w / 2.0 - 3.0).min(6.0), d * 0.75],
        z + 0.03,
        seed + 7,
    );
    roof_clothes(
        b,
        [w / 2.0 - 1.2 - (w / 2.0 - 3.0).min(6.0), d * 0.2],
        [w / 2.0 - 1.2, d * 0.2],
        z + 0.03,
        seed + 8,
    );
}

/// A terrace standing out of the front: its slab, the brick parapet with
/// the iron tube, closed in aluminium or with the green awning down.
#[allow(clippy::too_many_arguments)]
fn terrace(
    b: &mut Builder,
    f: &Face,
    h: &Hole,
    k: usize,
    floors: usize,
    zf: f64,
    upper: f64,
    seed: u64,
) {
    let out = 1.05;
    let (s0, s1) = (h.s.0 - 0.75, h.s.1 + 0.75);
    f.bx(
        b,
        "slab_edge",
        (s0, s1),
        (zf - 0.22, zf + 0.02),
        (-0.01, out),
    );
    let ph = 0.62;
    let pt = 0.14;
    f.bx(b, "brick", (s0, s1), (zf + 0.02, zf + ph), (out - pt, out));
    for x in [s0, s1 - pt] {
        f.bx(
            b,
            "brick",
            (x, x + pt),
            (zf + 0.02, zf + ph),
            (-0.01, out - pt),
        );
    }
    f.bx(
        b,
        "slab_edge",
        (s0 - 0.02, s1 + 0.02),
        (zf + ph, zf + ph + 0.04),
        (out - pt - 0.02, out + 0.02),
    );
    let top = zf + upper - 0.22;
    let r = unit(seed, 1);
    let closed = k + 1 < floors && r > 0.62;
    if closed {
        // Closed in aluminium: posts, a rail and sliding glass to the slab
        // above.
        let n = ((s1 - s0) / 0.8).round().max(2.0) as usize;
        let z0 = zf + ph + 0.04;
        for i in 0..=n {
            let x = s0 + 0.03 + (s1 - s0 - 0.06) * i as f64 / n as f64;
            f.bx(
                b,
                "alu",
                (x - 0.025, x + 0.025),
                (z0, top),
                (out - 0.09, out - 0.04),
            );
        }
        f.bx(
            b,
            "alu",
            (s0, s1),
            (top - 0.05, top),
            (out - 0.09, out - 0.04),
        );
        f.bx(
            b,
            "glass",
            (s0 + 0.03, s1 - 0.03),
            (z0, top - 0.05),
            (out - 0.07, out - 0.06),
        );
        for x in [s0, s1] {
            let side = Face {
                o: f.at(x, 0.0, 0.0).map_xy(),
                u: f.out(),
                len: out,
            };
            side.bx(
                b,
                "alu",
                (0.0, out - 0.04),
                (top - 0.05, top),
                (-0.03, 0.03),
            );
            side.bx(
                b,
                "glass",
                (0.0, out - 0.07),
                (z0, top - 0.05),
                (-0.006, 0.006),
            );
        }
        if unit(seed, 2) > 0.5 {
            f.bx(
                b,
                "curtain",
                (s0 + 0.1, s0 + 0.9),
                (z0, top - 0.06),
                (out - 0.2, out - 0.1),
            );
            f.bx(
                b,
                "curtain",
                (s0 + 0.1, s0 + 0.9),
                (top - 0.08, top - 0.05),
                (out - 0.1, out - 0.07),
            );
        }
    } else {
        // The iron tube on the parapet.
        let z0 = zf + ph + 0.04;
        f.bx(
            b,
            "iron",
            (s0 + 0.04, s1 - 0.04),
            (z0 + 0.32, z0 + 0.36),
            (out - 0.09, out - 0.05),
        );
        let n = ((s1 - s0) / 0.9).round().max(2.0) as usize;
        for i in 0..=n {
            let x = s0 + 0.06 + (s1 - s0 - 0.12) * i as f64 / n as f64;
            f.bx(
                b,
                "iron",
                (x - 0.015, x + 0.015),
                (z0, z0 + 0.34),
                (out - 0.085, out - 0.055),
            );
        }
        // The green awning down from the slab above, or a plant and a
        // chair.
        if k + 1 < floors && r > 0.3 {
            let drop = 0.8 + 0.9 * unit(seed, 3);
            let nn = ((s1 - s0) / 0.25).round().max(1.0) as usize;
            let wst = (s1 - s0) / nn as f64;
            for i in 0..nn {
                let role = if i % 2 == 0 { "awning" } else { "awning_b" };
                let a = s0 + wst * i as f64;
                f.leaning(
                    b,
                    role,
                    (a, a + wst),
                    (top + 0.0, out - 0.02),
                    (top - drop, out + 0.25),
                    0.01,
                );
            }
            f.bx(
                b,
                "awning",
                (s0, s1),
                (top - drop - 0.05, top - drop + 0.01),
                (out + 0.2, out + 0.27),
            );
        }
        standing_pot(b, f.at(s1 - 0.4, zf + 0.02, out - 0.4), 0.9, seed + 5);
        if unit(seed, 6) > 0.5 {
            standing_pot(b, f.at(s0 + 0.4, zf + 0.02, out - 0.4), 0.7, seed + 6);
        }
    }
}
