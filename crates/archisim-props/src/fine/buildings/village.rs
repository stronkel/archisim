//! The Andalusian village house between party walls (casa de pueblo):
//! thick whitewashed walls, a painted plinth, the panelled door with its
//! fanlight and step, grilles standing out of the ground floor windows
//! (or a «cierro»), a balcony of wrought iron over the door, pots hung on
//! the wall, a lantern, a cornice of two rows of tiles and the roof of
//! curved tiles falling to the street and to the yard.

use super::*;
use crate::Params;

const WALL: f64 = 0.4;
const REVEAL: f64 = 0.16;

pub fn village_house(b: &mut Builder, p: &Params) {
    let w = p.get("width", 7.0).clamp(4.5, 14.0);
    let d = p.get("depth", 10.0).clamp(6.0, 20.0);
    let floors = p.get("floors", 2.0).round().clamp(1.0, 3.0) as usize;
    let seed = p.get("variant", 1.0).round().max(0.0) as u64;
    let cierro = p.flag("cierro", true);
    let ground = 3.1;
    let upper = 2.85;
    let top = ground + upper * (floors - 1) as f64 + 0.35;
    let [front, _right, back, _left] = Face::sides(w, d);

    // Where things go on the front: the door, and a window each side
    // where there is room.
    let door_c = if w < 6.0 {
        w * 0.32
    } else if seed.is_multiple_of(2) {
        w * 0.5
    } else {
        w * 0.36
    };
    let door = Hole {
        s: (door_c - 0.62, door_c + 0.62),
        z: (0.14, 2.65),
    };
    let mut ground_windows: Vec<(Hole, bool)> = Vec::new();
    let left_room = door.s.0 - 0.5;
    let right_room = w - door.s.1 - 0.5;
    let big_right = right_room >= left_room;
    for (room, at_left) in [(left_room, true), (right_room, false)] {
        if room < 1.3 {
            continue;
        }
        let c = if at_left {
            door.s.0 - room / 2.0 - 0.15
        } else {
            door.s.1 + room / 2.0 + 0.15
        };
        let is_cierro = cierro && (at_left != big_right) && room > 1.8;
        let hw = if is_cierro { 0.7 } else { 0.55 };
        let z = if is_cierro { (0.55, 2.45) } else { (0.95, 2.3) };
        ground_windows.push((
            Hole {
                s: (c - hw, c + hw),
                z,
            },
            is_cierro,
        ));
    }
    // The floors above: a balcony door over the door, windows over the
    // ground windows.
    let mut upper_holes: Vec<(Hole, bool)> = Vec::new();
    for k in 1..floors {
        let zf = ground + upper * (k - 1) as f64;
        let attic = k == 2;
        if attic {
            for c in std::iter::once(door_c)
                .chain(ground_windows.iter().map(|(h, _)| (h.s.0 + h.s.1) / 2.0))
            {
                upper_holes.push((
                    Hole {
                        s: (c - 0.32, c + 0.32),
                        z: (zf + 1.0, zf + 1.65),
                    },
                    false,
                ));
            }
            continue;
        }
        upper_holes.push((
            Hole {
                s: (door_c - 0.5, door_c + 0.5),
                z: (zf + 0.02, zf + 2.25),
            },
            true,
        ));
        for (h, _) in &ground_windows {
            let c = (h.s.0 + h.s.1) / 2.0;
            upper_holes.push((
                Hole {
                    s: (c - 0.48, c + 0.48),
                    z: (zf + 0.85, zf + 2.1),
                },
                false,
            ));
        }
    }

    // The walls.
    let mut holes: Vec<Hole> = vec![door];
    holes.extend(ground_windows.iter().map(|(h, _)| *h));
    holes.extend(upper_holes.iter().map(|(h, _)| *h));
    wall(b, "wall", &front, (0.0, top), WALL, &holes);
    // The yard: a small window each floor and the back door.
    let mut back_holes = vec![Hole {
        s: (w * 0.62, w * 0.62 + 0.9),
        z: (0.0, 2.2),
    }];
    for k in 0..floors {
        let zf = if k == 0 {
            0.0
        } else {
            ground + upper * (k - 1) as f64
        };
        back_holes.push(Hole {
            s: (w * 0.25, w * 0.25 + 0.8),
            z: (zf + 1.1, zf + 2.1),
        });
    }
    wall(b, "wall", &back, (0.0, top), WALL, &back_holes);
    for f in [Face::sides(w, d)[1], Face::sides(w, d)[3]] {
        wall(b, "party", &f, (0.0, top), WALL, &[]);
    }
    core(b, w, d, WALL, (0.0, top));

    // The plinth, the cornice of tiles.
    let pl = 0.85 + 0.15 * unit(seed, 1);
    front.bx(b, "plinth", (0.0, door.s.0), (0.0, pl), (-0.01, 0.025));
    front.bx(b, "plinth", (door.s.1, w), (0.0, pl), (-0.01, 0.025));
    front.bx(b, "trim", (0.0, w), (top - 0.32, top - 0.2), (-0.01, 0.07));
    front.bx(b, "tiles", (0.0, w), (top - 0.2, top - 0.1), (-0.01, 0.15));
    front.bx(b, "trim", (0.0, w), (top - 0.1, top), (-0.01, 0.2));
    back.bx(b, "trim", (0.0, w), (top - 0.15, top), (-0.01, 0.12));

    // The door, its step and its surround.
    panel_door(b, &front, door, 2, 0.45, "wood", "iron", REVEAL);
    front.bx(
        b,
        "step",
        (door.s.0 - 0.1, door.s.1 + 0.1),
        (0.0, 0.14),
        (-REVEAL, 0.3),
    );
    let painted = seed % 3 != 1;
    if painted {
        surround(b, &front, door, "trim", 0.16, 0.012);
    }
    wall_lantern(b, &front, door.s.1 + 0.45, 2.75);

    // The ground floor windows: grilles out of the wall, or the «cierro».
    for (i, (h, is_cierro)) in ground_windows.iter().enumerate() {
        let win = Win {
            leaves: 2,
            reveal: REVEAL,
            bars: 2,
            curtain: unit(seed, 10 + i as u64) > 0.4,
            blind: if unit(seed, 20 + i as u64) > 0.6 {
                0.35
            } else {
                0.0
            },
            ..Default::default()
        };
        window(b, &front, *h, &win, "frame");
        sill(b, &front, *h, "step", 0.06, REVEAL);
        if painted {
            surround(b, &front, *h, "trim", 0.14, 0.012);
        }
        if *is_cierro {
            cierro_box(b, &front, *h);
        } else {
            projecting_grille(b, &front, *h, "iron", 0.16);
        }
    }

    // The floors above.
    for (i, (h, is_balcony)) in upper_holes.iter().enumerate() {
        let k = 40 + i as u64;
        let win = Win {
            leaves: 2,
            reveal: REVEAL,
            bars: if *is_balcony { 3 } else { 2 },
            curtain: unit(seed, k) > 0.35,
            blind: if unit(seed, k + 1) > 0.5 {
                0.2 + 0.5 * unit(seed, k + 2)
            } else {
                0.0
            },
            ..Default::default()
        };
        window(b, &front, *h, &win, "frame");
        if painted {
            surround(b, &front, *h, "trim", 0.13, 0.012);
        }
        if *is_balcony {
            let s = (h.s.0 - 0.35, h.s.1 + 0.35);
            balcony(b, &front, s, h.z.0, 0.5, "step", "iron", true);
            standing_pot(b, front.at(s.0 + 0.22, h.z.0, 0.24), 0.8, seed + k);
            if unit(seed, k + 3) > 0.4 {
                standing_pot(b, front.at(s.1 - 0.22, h.z.0, 0.24), 0.7, seed + k + 9);
            }
        } else if h.z.1 - h.z.0 > 0.9 {
            sill(b, &front, *h, "step", 0.06, REVEAL);
            grille_plane(
                b,
                &front,
                "iron",
                (h.s.0 - 0.04, h.s.1 + 0.04),
                (h.z.0 - 0.02, h.z.1),
                0.03,
                0.12,
                3,
            );
        } else {
            sill(b, &front, *h, "step", 0.04, REVEAL);
        }
    }
    // The yard's carpentry.
    for (i, h) in back_holes.iter().enumerate() {
        if i == 0 {
            panel_door(b, &back, *h, 1, 0.0, "wood", "iron", 0.1);
        } else {
            let win = Win {
                leaves: 1,
                reveal: 0.1,
                ..Default::default()
            };
            window(b, &back, *h, &win, "frame");
            sill(b, &back, *h, "step", 0.04, 0.1);
        }
    }

    // Pots hung between the holes of the ground floor, a few.
    let mut spots: Vec<f64> = Vec::new();
    let mut edges: Vec<(f64, f64)> = vec![door.s];
    edges.extend(
        ground_windows
            .iter()
            .map(|(h, _)| (h.s.0 - 0.2, h.s.1 + 0.2)),
    );
    edges.sort_by(|a, b| a.0.total_cmp(&b.0));
    let mut s = 0.2;
    for (a, e) in edges {
        if a - s > 0.6 {
            spots.push((s + a) / 2.0);
        }
        s = e;
    }
    if w - 0.2 - s > 0.6 {
        spots.push((s + w - 0.2) / 2.0);
    }
    for (i, x) in spots.iter().enumerate() {
        let k = 70 + i as u64;
        hung_pot(b, &front, *x, 1.9 + 0.3 * unit(seed, k), seed + k);
        if unit(seed, k + 5) > 0.3 {
            hung_pot(b, &front, *x + 0.05, 1.35, seed + k + 3);
        }
    }

    // The roof: to the street and to the yard, a chimney on the back.
    tile_roof(
        b,
        (-w / 2.0, w / 2.0),
        (0.0, d),
        top,
        0.42,
        (0.32, 0.0),
        (true, true),
    );
    let zc = top + d * 0.22 * 0.42;
    chimney(b, [w * 0.18, d * 0.22], zc - 0.3, zc + 0.9);
}

/// A «cierro»: the glazed box of wood and iron out of a ground floor
/// window, on its stone base, with a small roof.
fn cierro_box(b: &mut Builder, f: &Face, h: Hole) {
    let out = 0.38;
    let (s0, s1) = (h.s.0 - 0.06, h.s.1 + 0.06);
    let (z0, z1) = (h.z.0, h.z.1 + 0.1);
    f.bx(
        b,
        "step",
        (s0 - 0.05, s1 + 0.05),
        (z0 - 0.12, z0),
        (-0.01, out + 0.05),
    );
    f.bx(
        b,
        "step",
        (s0 - 0.05, s1 + 0.05),
        (z1, z1 + 0.08),
        (-0.01, out + 0.06),
    );
    f.leaning(
        b,
        "tiles",
        (s0 - 0.08, s1 + 0.08),
        (z1 + 0.32, 0.0),
        (z1 + 0.08, out + 0.14),
        0.05,
    );
    // The posts and the lower bars.
    let n = ((s1 - s0) / 0.45).round().max(2.0) as usize;
    for i in 0..=n {
        let x = s0 + (s1 - s0) * i as f64 / n as f64;
        f.bx(
            b,
            "iron",
            (x - 0.025, x + 0.025),
            (z0, z1),
            (out - 0.05, out),
        );
    }
    for x in [s0, s1] {
        let side = Face {
            o: f.at(x, 0.0, 0.0).map_xy(),
            u: f.out(),
            len: out,
        };
        side.bx(b, "iron", (0.0, out), (z0, z0 + 0.04), (-0.02, 0.02));
        side.bx(b, "iron", (0.0, out), (z0 + 0.85, z0 + 0.89), (-0.02, 0.02));
        side.bx(
            b,
            "glass",
            (0.0, out - 0.05),
            (z0 + 0.89, z1),
            (-0.006, 0.006),
        );
        grille_plane(
            b,
            &side,
            "iron",
            (0.02, out - 0.04),
            (z0 + 0.04, z0 + 0.85),
            0.0,
            0.11,
            2,
        );
    }
    f.bx(
        b,
        "iron",
        (s0, s1),
        (z0 + 0.85, z0 + 0.89),
        (out - 0.05, out),
    );
    f.bx(
        b,
        "glass",
        (s0, s1),
        (z0 + 0.89, z1),
        (out - 0.03, out - 0.02),
    );
    grille_plane(
        b,
        f,
        "iron",
        (s0, s1),
        (z0 + 0.04, z0 + 0.85),
        out - 0.025,
        0.11,
        2,
    );
    // Glazing bars on the upper glass.
    for k in 1..3 {
        let zz = z0 + 0.89 + (z1 - z0 - 0.89) * k as f64 / 3.0;
        f.bx(
            b,
            "iron",
            (s0, s1),
            (zz - 0.01, zz + 0.01),
            (out - 0.045, out - 0.005),
        );
    }
}
