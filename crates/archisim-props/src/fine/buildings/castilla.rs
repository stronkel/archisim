//! The Castilian house on a main square (casa de plaza mayor con
//! soportales): the ground floor set back behind an arcade of stone
//! columns with their wooden corbels and the beam, two floors above with
//! balconies of wood running along the front, walls of red ochre render
//! (or stone), the eaves on carved rafters and a roof of tiles.

use super::more::*;
use super::*;
use crate::Params;

const WALL: f64 = 0.5;
const REVEAL: f64 = 0.16;

pub fn castilian_arcade(b: &mut Builder, p: &Params) {
    let w = p.get("width", 12.0).clamp(6.0, 24.0);
    let d = p.get("depth", 12.0).clamp(8.0, 20.0);
    let floors = p.get("floors", 3.0).round().clamp(2.0, 4.0) as usize;
    let seed = p.get("variant", 1.0).round().max(0.0) as u64;
    let arc = 3.0; // the arcade's depth
    let ground = 3.6;
    let upper = 3.0;
    let top = ground + upper * (floors - 1) as f64;
    let [front, right, back, left] = Face::sides(w, d);
    let inner = front.moved(-arc);

    // The arcade: the shops and doors behind, the columns and the beam.
    let n = (w / 3.4).round().max(2.0) as usize;
    let bay = w / n as f64;
    let mut gh = Vec::new();
    for i in 0..n {
        let c = bay * (i as f64 + 0.5);
        gh.push(Hole {
            s: (c - bay * 0.35, c + bay * 0.35),
            z: (0.0, 2.9),
        });
    }
    wall(b, "wall", &inner, (0.0, ground), WALL, &gh);
    for (i, h) in gh.iter().enumerate() {
        if i % 2 == 0 {
            shop_front(
                b,
                &inner,
                *h,
                if unit(seed, i as u64) > 0.6 { 0.4 } else { 0.0 },
                true,
                "wood",
                REVEAL,
            );
        } else {
            panel_door(b, &inner, *h, 2, 0.6, "wood", "iron", REVEAL);
        }
    }
    b.rbox("paving", [-w / 2.0, d - arc, 0.0], [w / 2.0, d, 0.03], 0.0);
    for i in 0..=n {
        let x = (bay * i as f64).clamp(0.3, w - 0.3);
        let c = front.at(x, 0.0, -0.35);
        b.rbox(
            "stone",
            [c[0] - 0.32, c[1] - 0.32, 0.0],
            [c[0] + 0.32, c[1] + 0.32, 0.45],
            0.0,
        );
        b.lathe(
            "stone",
            [c[0], c[1], 0.45],
            &[
                (0.24, 0.0),
                (0.2, 0.12),
                (0.18, 2.5),
                (0.22, 2.65),
                (0.3, 2.8),
                (0.0, 2.8),
            ],
            1.0,
            1.0,
        );
        // The wooden corbel (zapata) on its capital.
        front.bx(b, "wood", (x - 0.55, x + 0.55), (3.25, 3.42), (-0.6, -0.1));
        front.bx(
            b,
            "wood",
            (x - 0.3, x + 0.3),
            (3.25 - 0.17, 3.25),
            (-0.55, -0.15),
        );
    }
    front.bx(b, "wood", (0.0, w), (3.42, 3.7), (-0.62, -0.08));
    // The arcade's ceiling of beams.
    b.rbox(
        "wood",
        [-w / 2.0, d - arc - 0.1, ground - 0.15],
        [w / 2.0, d - 0.08, ground],
        0.0,
    );
    let mut x = -w / 2.0 + 0.3;
    while x < w / 2.0 {
        b.rbox(
            "wood",
            [x - 0.06, d - arc, ground - 0.3],
            [x + 0.06, d - 0.6, ground - 0.15],
            0.0,
        );
        x += 0.6;
    }

    // The floors above: a balcony door per bay, the balconies of wood.
    let mut uh = Vec::new();
    for k in 1..floors {
        let zf = ground + upper * (k - 1) as f64;
        for i in 0..n {
            let c = bay * (i as f64 + 0.5);
            uh.push((
                Hole {
                    s: (c - 0.55, c + 0.55),
                    z: (zf + 0.05, zf + 2.35),
                },
                k,
            ));
        }
    }
    let holes: Vec<Hole> = uh.iter().map(|h| h.0).collect();
    wall(b, "wall", &front, (ground, top), WALL, &holes);
    for f in [right, left] {
        wall(b, "party", &f, (0.0, top), WALL, &[]);
    }
    let mut bh = Vec::new();
    for k in 0..floors {
        let zf = if k == 0 {
            0.0
        } else {
            ground + upper * (k - 1) as f64
        };
        for i in 0..n {
            let c = bay * (i as f64 + 0.5);
            bh.push(Hole {
                s: (c - 0.45, c + 0.45),
                z: (zf + 1.0, zf + 2.2),
            });
        }
    }
    wall(b, "wall", &back, (0.0, top), WALL, &bh);
    b.rbox(
        "interior",
        [-w / 2.0 + WALL, WALL, 0.0],
        [w / 2.0 - WALL, d - arc - WALL, ground],
        0.0,
    );
    b.rbox(
        "interior",
        [-w / 2.0 + WALL, WALL, ground],
        [w / 2.0 - WALL, d - WALL, top],
        0.0,
    );
    for (j, (h, _)) in uh.iter().enumerate() {
        let win = Win {
            leaves: 2,
            reveal: REVEAL,
            bars: 3,
            curtain: unit(seed, 100 + j as u64) > 0.4,
            ..Default::default()
        };
        window(b, &front, *h, &win, "frame");
        surround(b, &front, *h, "trim", 0.14, 0.02);
    }
    for k in 1..floors {
        let zf = ground + upper * (k - 1) as f64;
        // The first floor has its balcony along the whole front; the
        // others, a balcony of iron to each door.
        if k == 1 {
            wooden_balcony(b, &front, (0.1, w - 0.1), zf + 0.05, 0.7, "wood", None);
            for i in 0..n {
                if unit(seed, 300 + i as u64) > 0.4 {
                    standing_pot(
                        b,
                        front.at(bay * (i as f64 + 0.5) + 0.75, zf + 0.05, 0.4),
                        0.7,
                        seed + i as u64,
                    );
                }
            }
        } else {
            for i in 0..n {
                let c = bay * (i as f64 + 0.5);
                balcony(
                    b,
                    &front,
                    (c - 0.75, c + 0.75),
                    zf + 0.05,
                    0.45,
                    "stone",
                    "iron",
                    false,
                );
            }
        }
    }
    for h in &bh {
        let win = Win {
            leaves: 2,
            reveal: 0.12,
            ..Default::default()
        };
        window(b, &back, *h, &win, "frame");
    }
    // The eaves on carved rafters, the roof.
    let mut x = 0.15;
    while x < w {
        front.bx(
            b,
            "wood",
            (x - 0.06, x + 0.06),
            (top - 0.12, top),
            (-0.01, 0.6),
        );
        x += 0.45;
    }
    front.bx(b, "wood", (0.0, w), (top, top + 0.05), (-0.01, 0.65));
    front.bx(
        b,
        "plinth",
        (0.0, w),
        (ground - 0.1, ground + 0.15),
        (-0.01, 0.05),
    );
    tile_roof(
        b,
        (-w / 2.0, w / 2.0),
        (0.0, d),
        top,
        0.45,
        (0.65, 0.0),
        (true, true),
    );
    let zc = top + d * 0.25 * 0.45;
    chimney(b, [w * 0.25, d * 0.25], zc - 0.3, zc + 1.0);
}
