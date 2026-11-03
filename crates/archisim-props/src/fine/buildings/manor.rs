//! The town house of the old centre (casa señorial): three floors in
//! bays, a stone doorway with pilasters, entablature and coat of arms, the
//! main balcony over it, grilles in the ground floor holes, balconies of
//! wrought iron on the main floor with their hoods, white surrounds on a
//! coloured render, a cornice on brackets, a parapet with balls and, on
//! the flat roof, a lookout tower (torre mirador) as in Cádiz.

use super::*;
use crate::Params;

const WALL: f64 = 0.5;
const REVEAL: f64 = 0.18;

pub fn manor_house(b: &mut Builder, p: &Params) {
    let w = p.get("width", 12.0).clamp(7.0, 24.0);
    let d = p.get("depth", 14.0).clamp(8.0, 30.0);
    let floors = p.get("floors", 3.0).round().clamp(2.0, 4.0) as usize;
    let seed = p.get("variant", 1.0).round().max(0.0) as u64;
    let tower = p.flag("tower", true);
    let ground = 4.2;
    let noble = 3.8;
    let other = 3.1;
    let level = |k: usize| -> f64 {
        match k {
            0 => 0.0,
            1 => ground,
            _ => ground + noble + other * (k - 2) as f64,
        }
    };
    let roof_z = level(floors);
    let [front, right, back, left] = Face::sides(w, d);

    // The bays: an odd number, the door in the middle one.
    let mut n = ((w - 1.0) / 2.6).floor().max(3.0) as usize;
    if n.is_multiple_of(2) {
        n -= 1;
    }
    let bay = (w - 0.6) / n as f64;
    let axis = |i: usize| 0.3 + bay * (i as f64 + 0.5);
    let mid = n / 2;

    let door = Hole {
        s: (axis(mid) - 0.9, axis(mid) + 0.9),
        z: (0.18, 3.1),
    };
    let mut holes = vec![door];
    let mut ground_w = Vec::new();
    let mut floor_w: Vec<(usize, usize, Hole)> = Vec::new();
    for i in 0..n {
        let c = axis(i);
        if i != mid {
            let h = Hole {
                s: (c - 0.6, c + 0.6),
                z: (0.9, 3.2),
            };
            ground_w.push(h);
            holes.push(h);
        }
        for k in 1..floors {
            let zf = level(k);
            let h = if k == 1 {
                let hw = if i == mid { 0.7 } else { 0.6 };
                Hole {
                    s: (c - hw, c + hw),
                    z: (zf + 0.02, zf + 2.9),
                }
            } else {
                Hole {
                    s: (c - 0.5, c + 0.5),
                    z: (zf + 0.02, zf + 2.3),
                }
            };
            floor_w.push((i, k, h));
            holes.push(h);
        }
    }
    wall(b, "wall", &front, (0.0, roof_z), WALL, &holes);
    let mut back_holes = Vec::new();
    for i in 0..n {
        for k in 0..floors {
            let zf = level(k);
            back_holes.push(Hole {
                s: (axis(i) - 0.45, axis(i) + 0.45),
                z: (zf + 1.0, zf + 2.4),
            });
        }
    }
    wall(b, "wall", &back, (0.0, roof_z), WALL, &back_holes);
    for f in [right, left] {
        wall(b, "party", &f, (0.0, roof_z), WALL, &[]);
    }
    core(b, w, d, WALL, (0.0, roof_z));

    // The plinth and the string courses.
    let pl = 0.75;
    front.bx(
        b,
        "plinth",
        (0.0, door.s.0 - 0.45),
        (0.0, pl),
        (-0.01, 0.04),
    );
    front.bx(b, "plinth", (door.s.1 + 0.45, w), (0.0, pl), (-0.01, 0.04));
    for k in 1..floors {
        let z = level(k);
        front.bx(b, "trim", (0.0, w), (z - 0.32, z - 0.12), (-0.01, 0.05));
    }
    // The corners in stone quoins.
    for x in [0.0, w - 0.45] {
        let mut z = pl;
        let mut k = 0;
        while z < roof_z - 0.4 {
            let long = k % 2 == 0;
            let (a, e) = if x == 0.0 {
                (0.0, if long { 0.45 } else { 0.3 })
            } else {
                (if long { w - 0.45 } else { w - 0.3 }, w)
            };
            front.bx(b, "trim", (a, e), (z, z + 0.32), (-0.01, 0.025));
            z += 0.36;
            k += 1;
        }
    }

    // The doorway: pilasters, entablature, the coat of arms.
    let (ps0, ps1) = (door.s.0 - 0.45, door.s.1 + 0.45);
    for (a, e) in [(ps0, door.s.0 - 0.05), (door.s.1 + 0.05, ps1)] {
        front.bx(b, "stone", (a, e), (0.0, 0.45), (-0.01, 0.18));
        front.bx(
            b,
            "stone",
            (a + 0.04, e - 0.04),
            (0.45, 3.65),
            (-0.01, 0.12),
        );
        front.bx(b, "stone", (a, e), (3.65, 3.8), (-0.01, 0.16));
    }
    front.bx(
        b,
        "stone",
        (ps0 - 0.05, ps1 + 0.05),
        (3.8, 4.05),
        (-0.01, 0.2),
    );
    front.bx(
        b,
        "stone",
        (door.s.0 - 0.05, door.s.1 + 0.05),
        (door.z.1, 3.8),
        (-0.01, 0.06),
    );
    let u = front.u;
    let o = front.out();
    b.blob(
        "stone",
        front.at(axis(mid), 3.45, 0.06),
        [u[0] * 0.2, u[1] * 0.2, 0.0],
        [o[0] * 0.07, o[1] * 0.07, 0.0],
        [0.0, 0.0, 0.26],
    );
    panel_door(b, &front, door, 2, 0.0, "wood", "iron", REVEAL);
    front.bx(
        b,
        "stone",
        (door.s.0 - 0.15, door.s.1 + 0.15),
        (0.0, 0.18),
        (-REVEAL, 0.35),
    );
    // Studs on the leaves, three rows.
    for row in 0..3 {
        let z = 0.9 + 0.9 * row as f64;
        let mut x = door.s.0 + 0.25;
        while x < door.s.1 - 0.2 {
            b.ellipsoid(
                "brass",
                front.at(x, z, -REVEAL - 0.01),
                [0.025, 0.025, 0.025],
            );
            x += 0.3;
        }
    }

    // The ground floor windows: the grille in the hole, white surround.
    for (i, h) in ground_w.iter().enumerate() {
        let win = Win {
            leaves: 2,
            reveal: REVEAL,
            bars: 3,
            curtain: unit(seed, 5 + i as u64) > 0.5,
            ..Default::default()
        };
        window(b, &front, *h, &win, "frame");
        grille_plane(b, &front, "iron", h.s, h.z, -0.06, 0.12, 4);
        surround(b, &front, *h, "trim", 0.2, 0.03);
        sill(b, &front, *h, "stone", 0.08, REVEAL);
    }

    // The floors above.
    for (j, (i, k, h)) in floor_w.iter().enumerate() {
        let key = 30 + j as u64 * 7;
        let win = Win {
            leaves: 2,
            reveal: REVEAL,
            bars: 3,
            fanlight: if *k == 1 { 0.6 } else { 0.0 },
            curtain: unit(seed, key) > 0.35,
            blind: if unit(seed, key + 1) > 0.65 { 0.3 } else { 0.0 },
            ..Default::default()
        };
        window(b, &front, *h, &win, "frame");
        surround(b, &front, *h, "trim", 0.18, 0.03);
        // The ears of the surround and its hood on the main floor.
        if *k == 1 {
            front.bx(
                b,
                "trim",
                (h.s.0 - 0.3, h.s.1 + 0.3),
                (h.z.1 + 0.18, h.z.1 + 0.3),
                (-0.01, 0.1),
            );
            front.bx(
                b,
                "trim",
                (h.s.0 - 0.36, h.s.1 + 0.36),
                (h.z.1 + 0.3, h.z.1 + 0.38),
                (-0.01, 0.16),
            );
            for x in [h.s.0 - 0.18, h.s.1] {
                front.bx(
                    b,
                    "trim",
                    (x - 0.06, x + 0.24),
                    (h.z.1 - 0.12, h.z.1),
                    (-0.01, 0.03),
                );
            }
            let wide = if *i == mid { 0.45 } else { 0.25 };
            balcony(
                b,
                &front,
                (h.s.0 - wide, h.s.1 + wide),
                h.z.0,
                0.62,
                "stone",
                "iron",
                true,
            );
            if unit(seed, key + 2) > 0.3 {
                standing_pot(
                    b,
                    front.at(h.s.0 - wide + 0.25, h.z.0, 0.3),
                    0.85,
                    seed + key,
                );
            }
        } else if unit(seed, key + 3) > 0.25 {
            balcony(
                b,
                &front,
                (h.s.0 - 0.15, h.s.1 + 0.15),
                h.z.0,
                0.4,
                "stone",
                "iron",
                false,
            );
        } else {
            sill(b, &front, *h, "stone", 0.06, REVEAL);
            grille_plane(b, &front, "iron", h.s, (h.z.0, h.z.0 + 0.9), 0.02, 0.11, 2);
        }
    }
    for h in &back_holes {
        let win = Win {
            leaves: 2,
            reveal: 0.12,
            blind: 0.2,
            ..Default::default()
        };
        window(b, &back, *h, &win, "frame");
        sill(b, &back, *h, "trim", 0.04, 0.12);
    }

    // The cornice on brackets and the parapet with its balls.
    let z = roof_z;
    front.bx(b, "trim", (0.0, w), (z - 0.5, z - 0.3), (-0.01, 0.08));
    let mut x = 0.25;
    while x < w - 0.2 {
        front.bx(
            b,
            "trim",
            (x - 0.06, x + 0.06),
            (z - 0.3, z - 0.1),
            (-0.01, 0.3),
        );
        x += 0.55;
    }
    front.bx(
        b,
        "trim",
        (-0.05, w + 0.05),
        (z - 0.1, z + 0.05),
        (-0.01, 0.42),
    );
    front.bx(
        b,
        "trim",
        (-0.05, w + 0.05),
        (z + 0.05, z + 0.12),
        (-0.01, 0.36),
    );
    let pz = z + 0.12;
    parapet_on(b, &front, (0.0, w), pz, 0.9);
    for f in [right, back, left] {
        parapet_on(b, &f, (0.0, f.len), z, 1.0);
    }
    // Pedestals and balls on the axes, and the balusters between them.
    for i in 0..=n {
        let s = 0.3 + bay * i as f64;
        front.bx(
            b,
            "trim",
            (s - 0.18, s + 0.18),
            (pz, pz + 1.05),
            (-0.2, 0.05),
        );
        let c = front.at(s, pz + 1.27, -0.07);
        b.rbox(
            "stone",
            [c[0] - 0.12, c[1] - 0.12, pz + 1.05],
            [c[0] + 0.12, c[1] + 0.12, pz + 1.12],
            0.0,
        );
        b.ellipsoid("stone", c, [0.15, 0.15, 0.15]);
    }
    // The flat roof.
    b.rbox(
        "roof_floor",
        [-w / 2.0 + WALL * 0.5, WALL * 0.5, z - 0.2],
        [w / 2.0 - WALL * 0.5, d - WALL * 0.5, z + 0.02],
        0.0,
    );
    if tower {
        let (tw, td, th) = (3.0, 3.0, 3.2);
        let c = [w / 2.0 - tw / 2.0 - 0.5, d * 0.35];
        let tf = Face::sides(tw, td);
        let t = |f: &Face| Face {
            o: [f.o[0] + c[0], f.o[1] + c[1] - td / 2.0],
            ..*f
        };
        let z0 = z + 0.02;
        for (k, f) in tf.iter().enumerate() {
            let f = t(f);
            let h = Hole {
                s: (0.8, tw - 0.8),
                z: (z0 + 0.9, z0 + 2.5),
            };
            wall(b, "wall", &f, (z0, z0 + th), 0.25, &[h]);
            let win = Win {
                leaves: 2,
                reveal: 0.1,
                bars: 2,
                blind: if k == 2 { 0.5 } else { 0.0 },
                ..Default::default()
            };
            window(b, &f, h, &win, "frame");
            sill(b, &f, h, "stone", 0.05, 0.1);
            f.bx(
                b,
                "trim",
                (-0.05, tw + 0.05),
                (z0 + th, z0 + th + 0.12),
                (-0.25, 0.1),
            );
            parapet_on(b, &f, (0.0, tw), z0 + th + 0.12, 0.7);
        }
        b.rbox(
            "interior",
            [c[0] - tw / 2.0 + 0.25, c[1] - td / 2.0 + 0.25, z0],
            [c[0] + tw / 2.0 - 0.25, c[1] + td / 2.0 - 0.25, z0 + th],
            0.0,
        );
        for (dx, dy) in [(-1.0, -1.0), (1.0, -1.0), (1.0, 1.0), (-1.0, 1.0)] {
            let p = [
                c[0] + dx * (tw / 2.0 - 0.1),
                c[1] + dy * (td / 2.0 - 0.1),
                z0 + th + 0.82,
            ];
            b.ellipsoid("stone", [p[0], p[1], p[2] + 0.1], [0.1, 0.1, 0.1]);
        }
    }
    aerial(b, [-w / 2.0 + 1.5, d * 0.3], z + 0.02, seed);
    roof_clothes(
        b,
        [-w / 2.0 + 1.0, d * 0.65],
        [-w / 2.0 + 5.0, d * 0.65],
        z + 0.02,
        seed + 3,
    );
}

/// A parapet along a face from `s0` to `s1`, `high` over `z`, with its
/// coping.
fn parapet_on(b: &mut Builder, f: &Face, s: (f64, f64), z: f64, high: f64) {
    f.bx(b, "wall", s, (z, z + high), (-0.2, 0.0));
    f.bx(
        b,
        "trim",
        (s.0 - 0.02, s.1 + 0.02),
        (z + high, z + high + 0.06),
        (-0.23, 0.04),
    );
}
