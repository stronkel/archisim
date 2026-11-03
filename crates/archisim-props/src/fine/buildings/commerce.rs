//! Shops and cafés (tanda 5): the corner building with its café on the
//! ground floor and its terrace on the pavement, the building of flats
//! over a shop of several kinds (chemist's, bank, bakery, fashion), the
//! urban hotel and the supermarket with its car park.

use super::more::*;
use super::offices::car;
use super::*;
use crate::Params;

const WALL: f64 = 0.3;
const REVEAL: f64 = 0.1;

/// The four renders of the flats over a shop.
const FACADES: [&str; 4] = ["wall", "wall_ochre", "wall_cream", "wall_white"];

/// Flats over a ground floor: the floors' windows with blinds, balconies,
/// air conditioners; the ground floor's openings are the caller's.
#[allow(clippy::too_many_arguments)]
fn flats_over(
    b: &mut Builder,
    f: &Face,
    ground: f64,
    fl: f64,
    floors: usize,
    seed: u64,
    bal: bool,
    role: &'static str,
) {
    let n = ((f.len - 0.8) / 2.6).floor().max(1.0) as usize;
    let step = (f.len - 0.8) / n as f64;
    let mut hs = Vec::new();
    for k in 1..floors {
        let zf = ground + fl * (k - 1) as f64;
        for i in 0..n {
            let c = 0.4 + step * (i as f64 + 0.5);
            let door = bal && i % 2 == 0;
            hs.push((
                Hole {
                    s: (c - 0.6, c + 0.6),
                    z: (zf + if door { 0.05 } else { 0.9 }, zf + 2.2),
                },
                door,
                k,
            ));
        }
    }
    let holes: Vec<Hole> = hs.iter().map(|h| h.0).collect();
    wall(
        b,
        role,
        f,
        (ground, ground + fl * (floors - 1) as f64),
        WALL,
        &holes,
    );
    for (j, (h, door, _)) in hs.iter().enumerate() {
        let key = seed * 31 + j as u64;
        let win = Win {
            leaves: 2,
            reveal: REVEAL,
            sliding: unit(key, 1) > 0.5,
            blind: [0.0, 0.3, 0.6, 0.9][(unit(key, 2) * 4.0) as usize % 4],
            blind_box: true,
            curtain: unit(key, 3) > 0.5,
            ..Default::default()
        };
        window(b, f, *h, &win, "frame");
        if *door {
            balcony(
                b,
                f,
                (h.s.0 - 0.3, h.s.1 + 0.3),
                h.z.0,
                0.6,
                "trim",
                "iron",
                false,
            );
            if unit(key, 4) > 0.5 {
                standing_pot(b, f.at(h.s.1 + 0.1, h.z.0, 0.3), 0.7, key);
            }
        } else {
            sill(b, f, *h, "trim", 0.05, REVEAL);
            if unit(key, 5) > 0.8 {
                ac_unit(b, f, (h.s.0 + h.s.1) / 2.0, h.z.0 - 0.75);
            }
        }
    }
}

/// The corner building with its café: three floors of flats over the café
/// that turns the corner with its glazed fronts, the awnings, the terrace
/// of tables and parasols on the pavement and the café's sign.
pub fn corner_cafe(b: &mut Builder, p: &Params) {
    let w = p.get("width", 14.0).clamp(8.0, 30.0);
    let d = p.get("depth", 12.0).clamp(8.0, 24.0);
    let floors = p.get("floors", 4.0).round().clamp(2.0, 8.0) as usize;
    let seed = p.get("variant", 1.0).round().max(0.0) as u64;
    let ground = 4.0;
    let fl = 3.0;
    let top = ground + fl * (floors - 1) as f64;
    let [front, right, back, left] = Face::sides(w, d);
    // The café: glazed on the front and the right (the corner).
    let cafe_f = Hole {
        s: (0.6, w - 0.6),
        z: (0.0, 3.2),
    };
    let cafe_r = Hole {
        s: (0.6, d - 3.0),
        z: (0.0, 3.2),
    };
    wall(b, "plinth", &front, (0.0, ground), WALL, &[cafe_f]);
    wall(b, "plinth", &right, (0.0, ground), WALL, &[cafe_r]);
    for (f, h) in [(&front, cafe_f), (&right, cafe_r)] {
        let n = ((h.s.1 - h.s.0) / 1.8).round().max(1.0) as usize;
        let fd = (-REVEAL - 0.06, -REVEAL);
        for i in 0..=n {
            let x = h.s.0 + (h.s.1 - h.s.0) * i as f64 / n as f64;
            f.bx(
                b,
                "shop_frame",
                ((x - 0.04).max(h.s.0), (x + 0.04).min(h.s.1)),
                h.z,
                fd,
            );
        }
        f.bx(b, "shop_frame", h.s, (0.0, 0.4), fd);
        f.bx(b, "shop_frame", h.s, (2.6, 2.7), fd);
        f.bx(b, "shop_frame", h.s, (h.z.1 - 0.06, h.z.1), fd);
        f.bx(b, "glass", h.s, h.z, (-REVEAL - 0.04, -REVEAL - 0.03));
        f.bx(
            b,
            "sign",
            (h.s.0, h.s.1),
            (h.z.1 + 0.15, ground - 0.15),
            (-0.01, 0.08),
        );
        // Awnings in pieces along the front.
        let m = ((h.s.1 - h.s.0) / 3.6).round().max(1.0) as usize;
        for i in 0..m {
            let a = h.s.0 + (h.s.1 - h.s.0) * i as f64 / m as f64 + 0.15;
            let e = h.s.0 + (h.s.1 - h.s.0) * (i + 1) as f64 / m as f64 - 0.15;
            awning(b, f, (a, e), 3.35, 2.2, 0.6);
        }
    }
    // The café's inside seen through the glass: a counter and lamps.
    b.rbox(
        "interior",
        [-w / 2.0 + WALL, WALL, 0.0],
        [w / 2.0 - 1.4, d - 1.4, ground],
        0.0,
    );
    b.rbox(
        "counter",
        [-w / 2.0 + 1.0, d - 2.4, 0.0],
        [w / 2.0 - 4.0, d - 1.4, 1.1],
        0.0,
    );
    for i in 0..4 {
        let x = -w / 2.0 + 2.0 + i as f64 * (w - 4.0) / 3.0;
        b.cyl(
            "furniture",
            [x, d - 1.0, ground - 0.5],
            [x, d - 1.0, ground],
            0.01,
        );
        b.ellipsoid("lamp", [x, d - 1.0, ground - 0.6], [0.18, 0.18, 0.12]);
    }
    b.rbox(
        "interior_floor",
        [-w / 2.0 + WALL, WALL, 0.0],
        [w / 2.0 - WALL, d - WALL, 0.02],
        0.0,
    );
    // The flats above.
    flats_over(b, &front, ground, fl, floors, seed, true, "wall");
    flats_over(b, &right, ground, fl, floors, seed + 7, false, "wall");
    for f in [&back, &left] {
        wall(b, "party", f, (0.0, top), WALL, &[]);
    }
    b.rbox(
        "interior",
        [-w / 2.0 + WALL, WALL, ground],
        [w / 2.0 - WALL, d - WALL, top],
        0.0,
    );
    for f in [&front, &right] {
        f.bx(
            b,
            "trim",
            (0.0, f.len),
            (ground - 0.15, ground + 0.05),
            (-0.01, 0.12),
        );
        f.bx(
            b,
            "trim",
            (-0.05, f.len + 0.05),
            (top - 0.3, top),
            (-0.01, 0.2),
        );
    }
    parapet(b, w, d, top, 0.9, "wall", "trim");
    b.rbox(
        "roof_floor",
        [-w / 2.0 + 0.2, 0.2, top - 0.2],
        [w / 2.0 - 0.2, d - 0.2, top],
        0.0,
    );
    aerial(b, [-w * 0.2, d * 0.4], top, seed);
    // The terrace on the pavement: tables, chairs, parasols, planters.
    b.rbox(
        "paving",
        [-w / 2.0, d, 0.0],
        [w / 2.0 + 5.0, d + 5.0, 0.03],
        0.0,
    );
    b.rbox(
        "paving",
        [w / 2.0, -0.0, 0.0],
        [w / 2.0 + 5.0, d, 0.03],
        0.0,
    );
    for i in 0..(w / 2.8) as usize {
        let x = -w / 2.0 + 1.6 + 2.8 * i as f64;
        cafe_set(b, [x, d + 3.0], 4, i % 2 == 0, seed + i as u64);
    }
    for k in 0..2 {
        let y = d - 4.0 - 2.8 * k as f64;
        cafe_set(b, [w / 2.0 + 2.8, y], 3, k == 0, seed + 20 + k);
    }
    for x in [-w / 2.0 + 0.4, w / 2.0 - 0.4] {
        b.rbox(
            "planter",
            [x - 0.4, d + 4.2, 0.03],
            [x + 0.4, d + 4.9, 0.6],
            0.0,
        );
        b.blob(
            "leaves",
            [x, d + 4.55, 0.85],
            [0.45, 0.0, 0.0],
            [0.0, 0.35, 0.0],
            [0.0, 0.0, 0.35],
        );
    }
}

/// Flats over a shop of a kind (`shop`: 0 chemist's with its green cross,
/// 1 bank with its cash machine, 2 bakery with its awning, 3 fashion with
/// its window of mannequins).
pub fn shop_and_flats(b: &mut Builder, p: &Params) {
    let w = p.get("width", 10.0).clamp(6.0, 24.0);
    let d = p.get("depth", 12.0).clamp(8.0, 24.0);
    let floors = p.get("floors", 4.0).round().clamp(2.0, 9.0) as usize;
    let seed = p.get("variant", 1.0).round().max(0.0) as u64;
    let shop = p.get("shop", 0.0).round().clamp(0.0, 3.0) as usize;
    // The flats' render: one of four colours, by default by the kind of shop.
    let facade = p.get("facade", -1.0).round();
    let role = FACADES[if facade < 0.0 {
        shop
    } else {
        facade.min(3.0) as usize
    }];
    let ground = 3.8;
    let fl = 3.0;
    let top = ground + fl * (floors - 1) as f64;
    let [front, right, back, left] = Face::sides(w, d);
    let portal = Hole {
        s: (0.5, 1.7),
        z: (0.1, 2.7),
    };
    let window_h = Hole {
        s: (2.3, w - 0.5),
        z: (0.0, 3.0),
    };
    wall(
        b,
        "plinth",
        &front,
        (0.0, ground),
        WALL,
        &[portal, window_h],
    );
    panel_door(b, &front, portal, 1, 0.4, "wood", "iron", REVEAL);
    shop_front(b, &front, window_h, 0.0, true, "shop_frame", REVEAL);
    flats_over(b, &front, ground, fl, floors, seed, true, role);
    for f in [&right, &left] {
        wall(b, "party", f, (0.0, top), WALL, &[]);
    }
    let mut bh = Vec::new();
    for k in 1..floors {
        for x in [w * 0.3, w * 0.7] {
            let z = ground + fl * (k - 1) as f64;
            bh.push(Hole {
                s: (x - 0.5, x + 0.5),
                z: (z + 1.0, z + 2.2),
            });
        }
    }
    wall(b, role, &back, (0.0, top), WALL, &bh);
    for h in &bh {
        let win = Win {
            leaves: 2,
            reveal: REVEAL,
            blind: 0.5,
            blind_box: true,
            ..Default::default()
        };
        window(b, &back, *h, &win, "frame");
    }
    core(b, w, d, WALL, (0.0, top));
    front.bx(
        b,
        "trim",
        (0.0, w),
        (ground - 0.15, ground + 0.05),
        (-0.01, 0.12),
    );
    front.bx(b, "trim", (-0.05, w + 0.05), (top - 0.3, top), (-0.01, 0.2));
    parapet(b, w, d, top, 0.9, role, "trim");
    b.rbox(
        "roof_floor",
        [-w / 2.0 + 0.2, 0.2, top - 0.2],
        [w / 2.0 - 0.2, d - 0.2, top],
        0.0,
    );
    water_tank(b, [w * 0.2, d * 0.4], top);
    // What tells the shop.
    let (s0, s1) = window_h.s;
    match shop {
        0 => {
            // The chemist's green cross, standing out of the wall.
            let x = s1 - 0.5;
            let side = Face {
                o: front.at(x, 0.0, 0.0).map_xy(),
                u: front.out(),
                len: 0.8,
            };
            side.bx(b, "iron", (0.0, 0.25), (3.3, 3.36), (-0.03, 0.03));
            side.bx(b, "cross", (0.25, 0.45), (2.85, 3.85), (-0.06, 0.06));
            side.bx(b, "cross", (0.0 + 0.05, 0.65), (3.25, 3.45), (-0.06, 0.06));
        }
        1 => {
            // The bank's cash machine in its niche.
            let x = s0 + 1.0;
            front.bx(b, "steel", (x - 0.4, x + 0.4), (0.9, 1.9), (-REVEAL, 0.05));
            front.bx(b, "screen", (x - 0.2, x + 0.2), (1.35, 1.65), (0.05, 0.06));
            front.bx(b, "steel", (x - 0.25, x + 0.25), (1.1, 1.25), (0.05, 0.15));
        }
        2 => {
            awning(b, &front, (s0, s1), 3.3, 1.5, 0.5);
            for k in 0..4 {
                let x = s0 + 0.6 + k as f64 * (s1 - s0 - 1.2) / 3.0;
                b.ellipsoid(
                    "bread",
                    front.at(x, 1.15, -REVEAL - 0.25),
                    [0.18, 0.08, 0.06],
                );
                b.rbox(
                    "counter",
                    front.at(x - 0.3, 1.0, -REVEAL - 0.4).map_xy3(0.0),
                    front.at(x + 0.3, 1.1, -REVEAL - 0.1).map_xy3(1.1),
                    0.0,
                );
            }
        }
        _ => {
            for k in 0..3 {
                let x = s0 + 1.0 + k as f64 * (s1 - s0 - 2.0) / 2.0;
                let c = front.at(x, 0.0, -REVEAL - 0.5);
                b.cyl("steel", [c[0], c[1], 0.0], [c[0], c[1], 0.9], 0.02);
                b.cone(
                    "mannequin",
                    [c[0], c[1], 0.9],
                    [c[0], c[1], 1.6],
                    0.17,
                    0.2,
                    12,
                );
                b.ellipsoid("mannequin", [c[0], c[1], 1.75], [0.1, 0.1, 0.12]);
            }
        }
    }
    b.rbox("paving", [-w / 2.0, d, 0.0], [w / 2.0, d + 3.0, 0.03], 0.0);
}

trait Xy3 {
    fn map_xy3(self, z: f64) -> P3;
}

impl Xy3 for P3 {
    fn map_xy3(self, z: f64) -> P3 {
        [self[0], self[1], z]
    }
}

/// The urban hotel: a stone ground floor with its canopy, flags and the
/// revolving door, seven floors of rooms with French windows and little
/// balconies, the name on top and a roof terrace with a pergola.
pub fn urban_hotel(b: &mut Builder, p: &Params) {
    let w = p.get("width", 22.0).clamp(12.0, 50.0);
    let d = p.get("depth", 16.0).clamp(10.0, 30.0);
    let floors = p.get("floors", 8.0).round().clamp(3.0, 20.0) as usize;
    let seed = p.get("variant", 1.0).round().max(0.0) as u64;
    let ground = 4.5;
    let fl = 3.1;
    let top = ground + fl * (floors - 1) as f64;
    let faces = Face::sides(w, d);
    let front = faces[0];
    let entry = Hole {
        s: (w / 2.0 - 2.0, w / 2.0 + 2.0),
        z: (0.0, 3.6),
    };
    let mut gh = vec![entry];
    for x in [w * 0.15, w * 0.32, w * 0.68, w * 0.85] {
        gh.push(Hole {
            s: (x - 0.9, x + 0.9),
            z: (0.6, 3.6),
        });
    }
    wall(b, "stone", &front, (0.0, ground), WALL, &gh);
    for h in gh.iter().skip(1) {
        let win = Win {
            leaves: 2,
            reveal: 0.15,
            fanlight: 0.8,
            curtain: true,
            ..Default::default()
        };
        window(b, &front, *h, &win, "frame");
    }
    // The revolving door.
    let c = front.at(w / 2.0, 0.0, -0.4);
    b.cone("frame", [c[0], c[1], 0.0], [c[0], c[1], 0.05], 1.1, 1.1, 24);
    b.cone(
        "frame",
        [c[0], c[1], 2.7],
        [c[0], c[1], 2.9],
        1.15,
        1.15,
        24,
    );
    b.lathe(
        "glass",
        [c[0], c[1], 0.05],
        &[(1.05, 0.0), (1.05, 2.65), (1.0, 2.65), (1.0, 0.0)],
        1.0,
        1.0,
    );
    for k in 0..4 {
        let a = std::f64::consts::FRAC_PI_4 + std::f64::consts::FRAC_PI_2 * k as f64;
        b.rbox(
            "frame",
            [
                c[0] + a.cos() * 0.5 - 0.03,
                c[1] + a.sin() * 0.5 - 0.03,
                0.05,
            ],
            [
                c[0] + a.cos() * 0.5 + 0.03,
                c[1] + a.sin() * 0.5 + 0.03,
                2.7,
            ],
            0.0,
        );
    }
    front.bx(
        b,
        "glass",
        (entry.s.0, w / 2.0 - 1.15),
        (0.0, 3.6),
        (-0.15, -0.14),
    );
    front.bx(
        b,
        "glass",
        (w / 2.0 + 1.15, entry.s.1),
        (0.0, 3.6),
        (-0.15, -0.14),
    );
    // The canopy, the flags.
    front.bx(
        b,
        "canopy",
        (entry.s.0 - 1.5, entry.s.1 + 1.5),
        (3.8, 4.1),
        (-0.01, 4.0),
    );
    for x in [entry.s.0 - 1.2, entry.s.1 + 1.2] {
        front.bar(b, "frame", (x, 0.0), (x, 3.8), 3.8, 0.06, 10);
    }
    let fz = 4.1;
    for (k, x) in [w * 0.15, w * 0.32, w * 0.68].iter().enumerate() {
        let q = front.at(*x, 0.0, 0.0);
        let o = front.out();
        b.cone(
            "frame",
            [q[0], q[1], fz + 0.5],
            [q[0] + o[0] * 2.2, q[1] + o[1] * 2.2, fz + 1.6],
            0.025,
            0.025,
            6,
        );
        b.rbox(
            ["flag", "flag_b", "flag_c"][k],
            [q[0] + o[0] * 1.2 - 0.01, q[1] + o[1] * 1.2, fz - 0.4],
            [q[0] + o[0] * 1.2 + 0.01, q[1] + o[1] * 2.1, fz + 1.15],
            0.0,
        );
    }
    // The rooms.
    for (k, f) in faces.iter().enumerate() {
        if k == 0 {
            flats_over(b, f, ground, fl, floors, seed, true, "wall");
        } else {
            flats_over(b, f, ground, fl, floors, seed + k as u64, false, "wall");
            f.bx(b, "stone", (0.0, f.len), (0.0, ground), (-WALL, 0.0));
        }
        f.bx(
            b,
            "stone",
            (0.0, f.len),
            (ground - 0.25, ground + 0.05),
            (-0.01, 0.12),
        );
        f.bx(
            b,
            "trim",
            (-0.05, f.len + 0.05),
            (top - 0.3, top),
            (-0.01, 0.25),
        );
    }
    b.rbox(
        "interior",
        [-w / 2.0 + WALL, WALL, 0.0],
        [w / 2.0 - WALL, d - WALL, top],
        0.0,
    );
    // The name on the roof and the terrace with its pergola.
    parapet(b, w, d, top, 1.1, "wall", "trim");
    b.rbox(
        "roof_floor",
        [-w / 2.0 + 0.2, 0.2, top - 0.2],
        [w / 2.0 - 0.2, d - 0.2, top],
        0.0,
    );
    front.bx(
        b,
        "sign",
        (w * 0.25, w * 0.75),
        (top + 1.15, top + 2.2),
        (-0.4, -0.3),
    );
    for x in [w * 0.27, w * 0.73] {
        front.bx(
            b,
            "frame",
            (x - 0.05, x + 0.05),
            (top + 1.1, top + 1.2),
            (-0.6, -0.3),
        );
    }
    let (px0, px1, py0, py1) = (-w / 2.0 + 1.5, w / 2.0 - 8.0, d - 7.0, d - 1.0);
    for (x, y) in [(px0, py0), (px1, py0), (px1, py1), (px0, py1)] {
        b.rbox(
            "frame",
            [x - 0.08, y - 0.08, top],
            [x + 0.08, y + 0.08, top + 2.6],
            0.0,
        );
    }
    let mut x = px0;
    while x <= px1 {
        b.rbox(
            "pergola",
            [x - 0.04, py0 - 0.2, top + 2.6],
            [x + 0.04, py1 + 0.2, top + 2.75],
            0.0,
        );
        x += 0.5;
    }
    for y in [py0, py1] {
        b.rbox(
            "frame",
            [px0 - 0.2, y - 0.08, top + 2.45],
            [px1 + 0.2, y + 0.08, top + 2.6],
            0.0,
        );
    }
    for k in 0..3 {
        cafe_set(
            b,
            [px0 + 1.5 + k as f64 * 2.6, (py0 + py1) / 2.0],
            4,
            false,
            seed + k,
        );
    }
    b.rbox("paving", [-w / 2.0, d, 0.0], [w / 2.0, d + 5.0, 0.03], 0.0);
}

/// The supermarket: a big low box of panel with its glazed entrance under
/// a canopy, the sign, the trolleys' shelter, the car park with its lines,
/// trees and cars.
pub fn supermarket(b: &mut Builder, p: &Params) {
    let w = p.get("width", 40.0).clamp(20.0, 90.0);
    let d = p.get("depth", 30.0).clamp(15.0, 70.0);
    let seed = p.get("variant", 1.0).round().max(0.0) as u64;
    let h = 6.5;
    let faces = Face::sides(w, d);
    let front = faces[0];
    let glass = Hole {
        s: (w * 0.55, w - 1.0),
        z: (0.0, 4.2),
    };
    wall(b, "panel", &front, (0.0, h), WALL, &[glass]);
    for f in &faces[1..] {
        wall(b, "panel", f, (0.0, h), WALL, &[]);
    }
    for f in &faces {
        f.bx(b, "plinth", (0.0, f.len), (0.0, 1.0), (-0.01, 0.04));
        let mut z = 1.6;
        while z < h - 0.3 {
            f.bx(
                b,
                "panel",
                (0.0, f.len),
                (z - 0.02, z + 0.02),
                (-0.01, 0.03),
            );
            z += 1.2;
        }
        f.bx(
            b,
            "accent",
            (-0.05, f.len + 0.05),
            (h - 0.6, h),
            (-0.01, 0.1),
        );
    }
    curtain_wall(b, &front, glass.s, glass.z, 2.0, 4.2, -0.1);
    let c = (glass.s.0 + glass.s.1) / 2.0;
    front.bx(
        b,
        "door_frame",
        (c - 1.8, c + 1.8),
        (0.0, 2.6),
        (-0.06, 0.02),
    );
    front.bx(
        b,
        "canopy",
        (glass.s.0 - 1.0, glass.s.1 + 0.5),
        (4.3, 4.6),
        (-0.01, 3.5),
    );
    for x in [glass.s.0 - 0.6, c, glass.s.1] {
        front.bar(b, "mullion", (x, 0.0), (x, 4.3), 3.3, 0.1, 12);
    }
    front.bx(
        b,
        "sign",
        (w * 0.1, w * 0.45),
        (h - 2.4, h - 0.8),
        (-0.01, 0.15),
    );
    front.bx(
        b,
        "sign_text",
        (w * 0.12, w * 0.43),
        (h - 2.1, h - 1.1),
        (0.15, 0.17),
    );
    // The trolleys' shelter.
    let sx = glass.s.0 - 6.0;
    let sf = Face {
        o: [-w / 2.0 + sx, d + 1.5],
        u: [1.0, 0.0],
        len: 4.0,
    };
    sf.bx(b, "canopy", (0.0, 4.0), (2.3, 2.45), (-0.1, 2.2));
    for (x, y) in [(0.1, 0.0), (3.9, 0.0), (0.1, 2.0), (3.9, 2.0)] {
        sf.bar(b, "mullion", (x, 0.0), (x, 2.3), y, 0.05, 8);
    }
    for k in 0..6 {
        let y0 = 0.2 + 0.25 * k as f64;
        for row in 0..2 {
            let x = 0.4 + 1.6 * row as f64;
            sf.bx(b, "trolley", (x, x + 1.0), (0.15, 0.95), (y0, y0 + 0.2));
            sf.bar(
                b,
                "trolley",
                (x + 0.1, 0.0),
                (x + 0.1, 0.15),
                y0 + 0.1,
                0.02,
                4,
            );
        }
    }
    b.rbox(
        "interior",
        [-w / 2.0 + WALL, WALL, 0.0],
        [w / 2.0 - WALL, d - WALL, h - 0.4],
        0.0,
    );
    b.rbox(
        "roof_floor",
        [-w / 2.0 + 0.2, 0.2, h - 0.4],
        [w / 2.0 - 0.2, d - 0.2, h - 0.3],
        0.0,
    );
    for i in 0..4 {
        b.rbox(
            "appliance",
            [-w / 2.0 + 4.0 + 5.0 * i as f64, d * 0.3, h - 0.3],
            [-w / 2.0 + 7.0 + 5.0 * i as f64, d * 0.3 + 1.5, h + 1.2],
            0.0,
        );
    }
    // The car park.
    let py = d + 4.0;
    b.rbox("paving", [-w / 2.0, d, 0.0], [w / 2.0, py, 0.03], 0.0);
    b.rbox(
        "yard",
        [-w / 2.0 - 2.0, py, 0.0],
        [w / 2.0 + 2.0, py + 18.0, 0.02],
        0.0,
    );
    let n = (w / 2.5) as usize;
    for row in 0..2 {
        let y0 = py + 1.0 + row as f64 * 11.0;
        for i in 0..=n {
            let x = -w / 2.0 + 2.5 * i as f64;
            b.rbox("line", [x, y0, 0.02], [x + 0.1, y0 + 5.0, 0.025], 0.0);
        }
        for i in 0..n {
            if unit(seed, (row * 200 + i) as u64) > 0.5 {
                car(
                    b,
                    [-w / 2.0 + 2.5 * i as f64 + 1.25, y0 + 2.5],
                    unit(seed, (row * 200 + i + 77) as u64),
                );
            }
        }
    }
    for k in 0..6 {
        let x = -w / 2.0 + 2.0 + k as f64 * (w - 4.0) / 5.0;
        b.rbox(
            "stone",
            [x - 0.8, py + 7.0, 0.0],
            [x + 0.8, py + 9.0, 0.15],
            0.0,
        );
        fruit_tree(b, [x, py + 8.0], 5.0, seed + k, false);
    }
}
