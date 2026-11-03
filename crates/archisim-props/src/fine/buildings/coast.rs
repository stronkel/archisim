//! The tower of holiday flats on the coast (torre de apartamentos): a
//! white tower on its arcade, with a terrace running the whole front of
//! every floor, glass or white balustrades with a coloured band, the
//! party fins between flats, sliding windows behind, striped awnings and
//! towels hung out, and on the top the lift's hut, the aerials and the
//! solar panels.

use super::*;
use crate::Params;

const WALL: f64 = 0.25;
const REVEAL: f64 = 0.06;

pub fn coastal_tower(b: &mut Builder, p: &Params) {
    let w = p.get("width", 20.0).clamp(10.0, 40.0);
    let d = p.get("depth", 13.0).clamp(8.0, 24.0);
    let floors = p.get("floors", 10.0).round().clamp(4.0, 16.0) as usize;
    let seed = p.get("variant", 1.0).round().max(0.0) as u64;
    let glass = p.flag("glass_rail", false);
    let ground = 4.0;
    let upper = 2.9;
    let level = |k: usize| {
        if k == 0 {
            0.0
        } else {
            ground + upper * (k - 1) as f64
        }
    };
    let roof_z = level(floors);
    let faces = Face::sides(w, d);
    let [front, right, back, left] = faces;
    let out = 1.6;

    // The flats across the front, each with its window and its door.
    let n = ((w - 1.0) / 6.5).round().max(2.0) as usize;
    let fl = (w - 1.0) / n as f64;
    let mut holes = Vec::new();
    let mut parts = Vec::new();
    for k in 1..floors {
        let zf = level(k);
        for i in 0..n {
            let a = 0.5 + fl * i as f64;
            let door = Hole {
                s: (a + 0.5, a + 0.5 + 2.4),
                z: (zf + 0.02, zf + 2.2),
            };
            let win = Hole {
                s: (a + fl - 2.3, a + fl - 0.6),
                z: (zf + 0.9, zf + 2.2),
            };
            holes.push(door);
            holes.push(win);
            parts.push((k, i, door, true));
            parts.push((k, i, win, false));
        }
    }
    // The arcade: the ground floor set back behind columns.
    let set = 2.0;
    wall(b, "wall", &front, (ground, roof_z), WALL, &holes);
    let inner = front.moved(-set);
    let cafe = Hole {
        s: (1.0, w * 0.55),
        z: (0.0, 3.3),
    };
    let entry = Hole {
        s: (w * 0.62, w * 0.62 + 2.0),
        z: (0.1, 2.8),
    };
    wall(
        b,
        "ground_clad",
        &inner,
        (0.0, ground),
        WALL,
        &[cafe, entry],
    );
    front.bx(
        b,
        "slab_edge",
        (0.0, w),
        (ground - 0.45, ground),
        (-set, 0.02),
    );
    let cols = (w / 4.5).round().max(2.0) as usize;
    for i in 0..=cols {
        let x = 0.3 + (w - 0.6) * i as f64 / cols as f64;
        front.bx(
            b,
            "wall",
            (x - 0.25, x + 0.25),
            (0.0, ground - 0.45),
            (-0.5, 0.0),
        );
    }
    shop_front(b, &inner, cafe, 0.0, true, "alu", REVEAL);
    awning(b, &front, (1.4, w * 0.5), ground - 0.6, 1.6, 0.5);
    let ef = (-REVEAL - 0.06, -REVEAL);
    for x in [entry.s.0, entry.s.0 + 1.0, entry.s.1 - 0.05] {
        inner.bx(b, "alu", (x, x + 0.05), entry.z, ef);
    }
    inner.bx(b, "alu", entry.s, (entry.z.1 - 0.05, entry.z.1), ef);
    inner.bx(
        b,
        "glass",
        entry.s,
        entry.z,
        (-REVEAL - 0.04, -REVEAL - 0.03),
    );
    inner.bx(
        b,
        "step",
        (entry.s.0 - 0.2, entry.s.1 + 0.2),
        (0.0, 0.1),
        (-REVEAL, 0.6),
    );

    // The other faces: windows in rows.
    for (fi, f) in [right, back, left].iter().enumerate() {
        let m = ((f.len - 1.0) / 3.2).floor().max(1.0) as usize;
        let mut hs = Vec::new();
        for k in 1..floors {
            let zf = level(k);
            for i in 0..m {
                let c = 0.5 + (f.len - 1.0) * (i as f64 + 0.5) / m as f64;
                let small = fi != 1 && i % 2 == 1;
                hs.push(Hole {
                    s: if small {
                        (c - 0.35, c + 0.35)
                    } else {
                        (c - 0.7, c + 0.7)
                    },
                    z: if small {
                        (zf + 1.4, zf + 2.2)
                    } else {
                        (zf + 0.95, zf + 2.2)
                    },
                });
            }
        }
        wall(b, "wall", f, (ground, roof_z), WALL, &hs);
        let start = if fi == 0 { set } else { 0.0 };
        let end = if fi == 2 { f.len - set } else { f.len };
        f.bx(b, "ground_clad", (start, end), (0.0, ground), (-WALL, 0.0));
        for (j, h) in hs.iter().enumerate() {
            let key = 700 + (fi * 1000 + j) as u64;
            let win = Win {
                leaves: 2,
                reveal: REVEAL,
                sliding: true,
                blind: 0.7 * unit(seed, key),
                blind_box: true,
                ..Default::default()
            };
            window(b, f, *h, &win, "alu");
            if unit(seed, key + 1) > 0.85 {
                ac_unit(b, f, (h.s.0 + h.s.1) / 2.0, h.z.0 - 0.7);
            }
        }
    }
    b.rbox(
        "interior",
        [-w / 2.0 + WALL, WALL, 0.0],
        [w / 2.0 - WALL, d - set - WALL, ground],
        0.0,
    );
    b.rbox(
        "interior",
        [-w / 2.0 + WALL, WALL, ground],
        [w / 2.0 - WALL, d - WALL, roof_z],
        0.0,
    );

    // The terraces: a slab the whole front, fins between the flats, the
    // balustrade.
    for k in 1..=floors {
        let zf = level(k);
        let top = k == floors;
        front.bx(
            b,
            "slab_edge",
            (-0.05, w + 0.05),
            (zf - 0.25, zf + 0.02),
            (-0.01, out),
        );
        if top {
            continue;
        }
        for i in 0..=n {
            let x = 0.5 + fl * i as f64;
            let x = x.clamp(0.1, w - 0.1);
            front.bx(
                b,
                "wall",
                (x - 0.1, x + 0.1),
                (zf + 0.02, zf + upper - 0.25),
                (-0.01, out - 0.05),
            );
            if i < n {
                let a = x + 0.1;
                let e = 0.5 + fl * (i + 1) as f64 - 0.1;
                balustrade(b, &front, (a, e), zf + 0.02, out, glass);
            }
        }
    }
    for (j, (k, i, h, is_door)) in parts.iter().enumerate() {
        let key = 900 + j as u64 * 7;
        let win = Win {
            leaves: if *is_door { 3 } else { 2 },
            reveal: REVEAL,
            sliding: true,
            blind: if *is_door {
                0.5 * unit(seed, key)
            } else {
                0.85 * unit(seed, key)
            },
            blind_box: true,
            curtain: unit(seed, key + 1) > 0.5,
            ..Default::default()
        };
        window(b, &front, *h, &win, "alu");
        if !*is_door {
            continue;
        }
        let zf = level(*k);
        let a = 0.5 + fl * *i as f64 + 0.1;
        let e = a + fl - 0.2;
        let r = unit(seed, key + 2);
        // A striped awning under the slab above, more or less down.
        if r > 0.45 {
            let ztop = zf + upper - 0.25;
            let drop = 0.6 + 0.9 * unit(seed, key + 3);
            let nn = ((e - a) / 0.3).round().max(1.0) as usize;
            let wst = (e - a) / nn as f64;
            for t in 0..nn {
                let role = if t % 2 == 0 { "awning" } else { "awning_b" };
                let x = a + wst * t as f64;
                front.leaning(
                    b,
                    role,
                    (x, x + wst),
                    (ztop, out - 0.08),
                    (ztop - drop, out + 0.15),
                    0.01,
                );
            }
            front.bx(
                b,
                "awning",
                (a, e),
                (ztop - drop - 0.05, ztop - drop + 0.01),
                (out + 0.1, out + 0.17),
            );
        }
        // Towels on the balustrade.
        if unit(seed, key + 4) > 0.55 {
            let x = a + 0.6 + (e - a - 2.0) * unit(seed, key + 5);
            let role = ["cloth_a", "cloth_b", "cloth_c"][(unit(seed, key + 6) * 3.0) as usize % 3];
            front.bx(
                b,
                role,
                (x, x + 0.7),
                (zf + 0.45, zf + 1.07),
                (out - 0.02, out + 0.01),
            );
        }
        if unit(seed, key + 7) > 0.5 {
            standing_pot(b, front.at(e - 0.4, zf + 0.02, out - 0.45), 0.9, seed + key);
        }
    }

    // The top: the lift's hut, the parapet, aerials and solar panels.
    let z = roof_z;
    b.rbox(
        "roof_floor",
        [-w / 2.0 + 0.1, 0.1, z - 0.2],
        [w / 2.0 - 0.1, d - 0.1, z + 0.03],
        0.0,
    );
    parapet(b, w, d, z, 1.0, "wall", "slab_edge");
    b.rbox(
        "wall",
        [-2.0, d * 0.35, z],
        [2.0, d * 0.35 + 4.5, z + 3.0],
        0.0,
    );
    b.rbox(
        "slab_edge",
        [-2.1, d * 0.35 - 0.1, z + 3.0],
        [2.1, d * 0.35 + 4.6, z + 3.15],
        0.0,
    );
    water_tank(b, [3.2, d * 0.7], z + 0.03);
    for i in 0..3 {
        aerial(
            b,
            [-w / 2.0 + 2.0 + i as f64 * 2.5, d * 0.25],
            z + 0.03,
            seed + i,
        );
    }
    let pw = (w / 2.0 - 3.5).max(2.0);
    let n_p = (pw / 1.1).floor().max(1.0) as usize;
    for i in 0..n_p {
        let x = 2.5 + i as f64 * 1.1;
        if x + 1.0 > w / 2.0 - 0.5 {
            break;
        }
        for row in 0..2 {
            let y = 1.0 + row as f64 * 2.2;
            b.rbox(
                "metal",
                [x + 0.4, y + 1.5, z + 0.03],
                [x + 0.46, y + 1.56, z + 0.95],
                0.0,
            );
            let c = [x + 0.5, y + 0.85, z + 0.55];
            let t: f64 = 0.5;
            b.obox(
                "panel",
                c,
                [0.5, 0.0, 0.0],
                [0.0, 0.85 * t.cos(), 0.85 * t.sin()],
                [0.0, -0.02 * t.sin(), 0.02 * t.cos()],
            );
            b.rbox(
                "metal",
                [x + 0.4, y + 0.1, z + 0.03],
                [x + 0.46, y + 0.16, z + 0.2],
                0.0,
            );
        }
    }
}

/// The balustrade of a terrace from `s0` to `s1`: glass in an aluminium
/// handrail, or white with a band of colour.
fn balustrade(b: &mut Builder, f: &Face, s: (f64, f64), z: f64, out: f64, glass: bool) {
    let d = out - 0.05;
    if glass {
        f.bx(b, "alu", s, (z, z + 0.06), (d - 0.04, d + 0.03));
        f.bx(b, "glass", s, (z + 0.06, z + 0.98), (d - 0.012, d + 0.0));
        f.bx(b, "alu", s, (z + 0.98, z + 1.04), (d - 0.04, d + 0.03));
        let n = ((s.1 - s.0) / 1.2).round().max(1.0) as usize;
        for i in 0..=n {
            let x = s.0 + (s.1 - s.0) * i as f64 / n as f64;
            f.bx(
                b,
                "alu",
                (x - 0.02, x + 0.02),
                (z, z + 0.98),
                (d - 0.03, d + 0.01),
            );
        }
    } else {
        f.bx(b, "wall", s, (z, z + 0.62), (d - 0.12, d + 0.04));
        f.bx(b, "band", s, (z + 0.62, z + 0.72), (d - 0.12, d + 0.05));
        f.bx(
            b,
            "alu",
            (s.0 + 0.03, s.1 - 0.03),
            (z + 0.98, z + 1.02),
            (d - 0.08, d - 0.04),
        );
        let n = ((s.1 - s.0) / 1.0).round().max(1.0) as usize;
        for i in 0..=n {
            let x = s.0 + 0.04 + (s.1 - s.0 - 0.08) * i as f64 / n as f64;
            f.bx(
                b,
                "alu",
                (x - 0.015, x + 0.015),
                (z + 0.72, z + 0.98),
                (d - 0.075, d - 0.045),
            );
        }
    }
}
