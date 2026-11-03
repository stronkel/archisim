//! The «corral de vecinos» (Seville, Huelva, Cádiz): two floors of small
//! dwellings round a long courtyard, reached from the street through a
//! passage behind a gate; each dwelling with its door and its window to
//! the courtyard, the upper ones on an iron gallery round three sides under
//! the roofs of one slope, a stair up to it, the washing hung across, the
//! orange tree, the trough and pots everywhere.

use super::more::*;
use super::*;
use crate::Params;

const WALL: f64 = 0.4;
const REVEAL: f64 = 0.14;
const BAY: f64 = 3.3;

/// A hole on the courtyard: its place, whether it is a door, its floor.
type CourtHole = (Hole, bool, usize);

pub fn corral(b: &mut Builder, p: &Params) {
    let w = p.get("width", 16.0).clamp(12.0, 28.0);
    let d = p.get("depth", 24.0).clamp(15.0, 44.0);
    let seed = p.get("variant", 1.0).round().max(0.0) as u64;
    let c = 3.6; // the depth of the ranges of rooms (crujías)
    let ground = 3.0;
    let top = ground + 2.8;
    let gal = 1.3;
    let (px0, px1, py0, py1) = (-w / 2.0 + c, w / 2.0 - c, c, d - c);
    let (wp, lp) = (px1 - px0, py1 - py0);
    let [front, right, back, left] = Face::sides(w, d);
    // The courtyard's faces, seen from the courtyard: towards the street,
    // east, back and west.
    let pf = Face {
        o: [px1, py1],
        u: [-1.0, 0.0],
        len: wp,
    };
    let pe = Face {
        o: [px1, py0],
        u: [0.0, 1.0],
        len: lp,
    };
    let pb = Face {
        o: [px0, py0],
        u: [1.0, 0.0],
        len: wp,
    };
    let pw = Face {
        o: [px0, py1],
        u: [0.0, -1.0],
        len: lp,
    };

    // The street front: the gate in the middle, a window each side, and
    // upstairs a balcony over the gate and windows.
    let gate = Hole {
        s: (w / 2.0 - 0.95, w / 2.0 + 0.95),
        z: (0.12, 3.0),
    };
    let mut fh = vec![gate];
    let mut fw = Vec::new();
    for x in [w * 0.2, w * 0.8] {
        let h = Hole {
            s: (x - 0.55, x + 0.55),
            z: (1.0, 2.3),
        };
        fh.push(h);
        fw.push((h, 0));
    }
    let up_door = Hole {
        s: (w / 2.0 - 0.5, w / 2.0 + 0.5),
        z: (ground + 0.05, ground + 2.3),
    };
    fh.push(up_door);
    for x in [w * 0.2, w * 0.8] {
        let h = Hole {
            s: (x - 0.45, x + 0.45),
            z: (ground + 0.9, ground + 2.1),
        };
        fh.push(h);
        fw.push((h, 1));
    }
    wall(b, "wall", &front, (0.0, top), WALL, &fh);
    wall(b, "wall", &back, (0.0, top), WALL, &[]);
    for f in [right, left] {
        wall(b, "party", &f, (0.0, top), WALL, &[]);
    }
    front.bx(
        b,
        "plinth",
        (0.0, gate.s.0 - 0.2),
        (0.0, 0.8),
        (-0.01, 0.025),
    );
    front.bx(b, "plinth", (gate.s.1 + 0.2, w), (0.0, 0.8), (-0.01, 0.025));
    front.bx(b, "trim", (0.0, w), (top - 0.25, top), (-0.01, 0.12));

    // The courtyard's walls: a door and a window for each dwelling, two
    // floors; the passage from the gate.
    let passage = Hole {
        s: (wp / 2.0 - 0.9, wp / 2.0 + 0.9),
        z: (0.0, 2.9),
    };
    let mut courtyard: Vec<(Face, Vec<CourtHole>)> = Vec::new();
    for (i, f) in [pf, pe, pb, pw].iter().enumerate() {
        let n = ((f.len - 0.6) / BAY).floor().max(1.0) as usize;
        let bay = (f.len - 0.6) / n as f64;
        let mut hs = Vec::new();
        for k in 0..n {
            let a = 0.3 + bay * k as f64;
            for fl in 0..2 {
                let z = if fl == 0 { 0.0 } else { ground };
                if i == 0 && fl == 0 {
                    continue;
                }
                hs.push((
                    Hole {
                        s: (a + 0.35, a + 1.2),
                        z: (z + 0.05, z + 2.2),
                    },
                    true,
                    fl,
                ));
                if bay > 2.4 {
                    hs.push((
                        Hole {
                            s: (a + 1.65, a + 2.45),
                            z: (z + 1.0, z + 2.1),
                        },
                        false,
                        fl,
                    ));
                }
            }
        }
        if i == 0 {
            hs.push((passage, true, 9));
        }
        let holes: Vec<Hole> = hs.iter().map(|h| h.0).collect();
        wall(b, "wall", f, (0.0, top), WALL, &holes);
        f.bx(b, "plinth", (0.0, f.len), (0.0, 0.6), (-0.01, 0.02));
        courtyard.push((*f, hs));
    }
    // The rooms inside: the ranges round the courtyard, the passage open.
    let t = WALL;
    let (gx0, gx1) = (-0.9, 0.9);
    for (x0, x1, z0) in [
        (-w / 2.0 + t, gx0, 0.0),
        (gx1, w / 2.0 - t, 0.0),
        (gx0, gx1, 2.9),
    ] {
        b.rbox("interior", [x0, py1 + t, z0], [x1, d - t, top], 0.0);
    }
    b.rbox(
        "interior",
        [-w / 2.0 + t, t, 0.0],
        [w / 2.0 - t, py0 - t, top],
        0.0,
    );
    b.rbox(
        "interior",
        [-w / 2.0 + t, py0 - t, 0.0],
        [px0 - t, py1 + t, top],
        0.0,
    );
    b.rbox(
        "interior",
        [px1 + t, py0 - t, 0.0],
        [w / 2.0 - t, py1 + t, top],
        0.0,
    );
    // The passage's own walls and ceiling, whitewashed.
    b.rbox(
        "wall",
        [gx0 - 0.08, py1 - 0.05, 0.0],
        [gx0, d - 0.05, 2.9],
        0.0,
    );
    b.rbox(
        "wall",
        [gx1, py1 - 0.05, 0.0],
        [gx1 + 0.08, d - 0.05, 2.9],
        0.0,
    );
    b.rbox("wall", [gx0, py1 - 0.05, 2.82], [gx1, d - 0.05, 2.9], 0.0);
    // The floors of the passage and the courtyard.
    b.rbox("patio_floor", [px0, py0, 0.0], [px1, py1, 0.03], 0.0);
    b.rbox("patio_floor", [gx0, py1 - 0.01, 0.0], [gx1, d, 0.03], 0.0);

    // The gate: stone surround, the leaves open in the passage, the iron
    // gate (cancela) at the end of it, the tiled panel with its name.
    surround(b, &front, gate, "stone", 0.22, 0.05);
    front.bx(b, "step", (gate.s.0, gate.s.1), (0.0, 0.12), (-WALL, 0.25));
    for (x, sgn) in [(gate.s.0, 1.0), (gate.s.1, -1.0)] {
        let side = Face {
            o: front.at(x, 0.0, -0.06).map_xy(),
            u: [0.0, -1.0],
            len: 0.95,
        };
        side.bx(b, "wood", (0.0, 0.92), (0.12, 2.9), (0.0, 0.07 * sgn));
    }
    grille_plane(b, &pf, "iron", passage.s, (0.02, 2.4), 0.0, 0.11, 4);
    {
        // Its crest of scrolls.
        let (s0, s1) = passage.s;
        let n = 7;
        for k in 0..n {
            let x = s0 + (s1 - s0) * (k as f64 + 0.5) / n as f64;
            let hgt = 0.25 + 0.2 * (1.0 - ((k as f64 - 3.0) / 3.0).abs());
            pf.bar(b, "iron", (x, 2.4), (x, 2.4 + hgt), 0.0, 0.012, 5);
            b.ellipsoid(
                "iron",
                pf.at(x, 2.4 + hgt + 0.03, 0.0),
                [0.035, 0.035, 0.035],
            );
        }
    }
    front.bx(
        b,
        "tile_panel",
        (gate.s.0 + 0.2, gate.s.1 - 0.2),
        (gate.z.1 + 0.32, gate.z.1 + 0.82),
        (-0.01, 0.02),
    );
    front.bx(
        b,
        "stone",
        (gate.s.0 + 0.15, gate.s.1 - 0.15),
        (gate.z.1 + 0.28, gate.z.1 + 0.32),
        (-0.01, 0.05),
    );
    front.bx(
        b,
        "stone",
        (gate.s.0 + 0.15, gate.s.1 - 0.15),
        (gate.z.1 + 0.82, gate.z.1 + 0.86),
        (-0.01, 0.05),
    );
    // Its street windows and the balcony.
    for (i, (h, fl)) in fw.iter().enumerate() {
        let win = Win {
            leaves: 2,
            reveal: REVEAL,
            bars: 2,
            curtain: true,
            blind: if unit(seed, 300 + i as u64) > 0.5 {
                0.3
            } else {
                0.0
            },
            ..Default::default()
        };
        window(b, &front, *h, &win, "frame");
        surround(b, &front, *h, "trim", 0.14, 0.012);
        sill(b, &front, *h, "step", 0.05, REVEAL);
        if *fl == 0 {
            projecting_grille(b, &front, *h, "iron", 0.16);
        } else {
            grille_plane(b, &front, "iron", h.s, h.z, 0.03, 0.12, 3);
        }
    }
    let win = Win {
        leaves: 2,
        reveal: REVEAL,
        bars: 3,
        curtain: true,
        ..Default::default()
    };
    window(b, &front, up_door, &win, "frame");
    surround(b, &front, up_door, "trim", 0.14, 0.012);
    balcony(
        b,
        &front,
        (up_door.s.0 - 0.3, up_door.s.1 + 0.3),
        up_door.z.0,
        0.45,
        "step",
        "iron",
        true,
    );
    standing_pot(
        b,
        front.at(up_door.s.1 + 0.1, up_door.z.0, 0.22),
        0.8,
        seed + 5,
    );
    for (k, x) in [w * 0.35, w * 0.65].iter().enumerate() {
        hung_pot(b, &front, *x, 2.0, seed + 20 + k as u64);
    }

    // The dwellings' doors and windows on the courtyard.
    let roles = ["wood", "wood_b"];
    for (fi, (f, hs)) in courtyard.iter().enumerate() {
        for (j, (h, is_door, fl)) in hs.iter().enumerate() {
            if *fl == 9 {
                continue;
            }
            let key = seed * 7 + (fi * 100 + j) as u64;
            if *is_door {
                let role = roles[(unit(key, 1) * 2.0) as usize % 2];
                panel_door(b, f, *h, 1, 0.0, role, "iron", 0.1);
                if *fl == 0 && unit(key, 2) > 0.3 {
                    standing_pot(b, f.at(h.s.1 + 0.3, 0.03, 0.3), 0.85, key);
                }
            } else {
                let win = Win {
                    leaves: 2,
                    reveal: 0.1,
                    bars: 1,
                    curtain: unit(key, 3) > 0.3,
                    ..Default::default()
                };
                window(b, f, *h, &win, "frame");
                grille_plane(b, f, "iron", h.s, h.z, -0.03, 0.12, 3);
                sill(b, f, *h, "step", 0.04, 0.1);
                if unit(key, 4) > 0.5 {
                    hung_pot(b, f, h.s.1 + 0.25, h.z.1 - 0.4, key);
                }
            }
        }
    }

    // The gallery round the back and the sides: its floor, the iron
    // posts below and above, the railing; and the roofs over it.
    let pitch = 0.36;
    for (fi, f) in [pe, pb, pw].iter().enumerate() {
        let (s0, s1) = match fi {
            0 => (gal - 0.05, f.len),
            1 => (0.0, f.len),
            _ => (0.0, f.len - gal + 0.05),
        };
        f.bx(
            b,
            "gallery",
            (s0, s1),
            (ground - 0.18, ground + 0.02),
            (-0.01, gal),
        );
        f.bx(
            b,
            "gallery_edge",
            (s0, s1),
            (ground - 0.25, ground - 0.18),
            (gal - 0.12, gal),
        );
        let n = ((s1 - s0) / BAY).round().max(1.0) as usize;
        for k in 0..=n {
            let x = (s0 + (s1 - s0) * k as f64 / n as f64).clamp(s0 + 0.08, s1 - 0.08);
            let zr = top - (gal - 0.1) * pitch;
            f.bar(
                b,
                "iron",
                (x, 0.03),
                (x, ground - 0.25),
                gal - 0.1,
                0.05,
                10,
            );
            f.bar(
                b,
                "iron",
                (x, ground + 0.02),
                (x, zr + 0.02),
                gal - 0.1,
                0.04,
                10,
            );
            // Brackets of the posts.
            f.bar(
                b,
                "iron",
                (x, ground - 0.25),
                (x, ground - 0.45),
                gal - 0.1,
                0.07,
                10,
            );
        }
        // The railing, open where the stair arrives.
        let (r0, r1) = if fi == 0 { (s0, f.len - 2.0) } else { (s0, s1) };
        railing(
            b,
            f,
            "iron",
            (r0 + 0.05, r1 - 0.05),
            ground + 0.02,
            gal - 0.06,
            0.95,
            0.12,
        );
        // Pots on the gallery.
        let m = ((s1 - s0) / 1.4) as usize;
        for k in 0..m {
            let key = seed * 13 + (fi * 50 + k) as u64;
            if unit(key, 1) > 0.45 {
                let x = s0 + 0.5 + 1.4 * k as f64;
                standing_pot(b, f.at(x, ground + 0.02, gal - 0.3), 0.65, key);
            }
        }
    }
    // The roofs of one slope over the gallery, and the street range's
    // roof of two.
    for (fi, f) in [pe, pb, pw].iter().enumerate() {
        let s = if fi == 1 {
            (-c, f.len + c)
        } else {
            (0.0, f.len)
        };
        pent_roof(
            b,
            f,
            s,
            (-c - 0.02, top + c * pitch),
            (gal + 0.3, top - (gal + 0.3) * pitch),
        );
    }
    // The outer walls rise to the high side of the roofs of one slope.
    let up = (top, top + c * pitch + 0.02);
    back.bx(b, "wall", (0.0, w), up, (-WALL, 0.0));
    right.bx(b, "party", (c, d), up, (-WALL, 0.0));
    left.bx(b, "party", (0.0, py1), up, (-WALL, 0.0));
    tile_roof(
        b,
        (-w / 2.0, w / 2.0),
        (py1, d),
        top,
        0.42,
        (0.35, 0.0),
        (true, true),
    );
    let zc = top + 1.0 * 0.42;
    chimney(b, [-w / 2.0 + 1.3, py1 + 1.0], zc - 0.3, zc + 0.9);

    // The stair up to the gallery, along the east side, out of it.
    let steps = 16;
    let rise = (ground + 0.02) / steps as f64;
    let tread = 0.27;
    let top_s = pe.len - 0.3;
    for k in 0..steps {
        let s1 = top_s - tread * k as f64;
        let z = ground + 0.02 - rise * k as f64;
        pe.bx(
            b,
            "step",
            (s1 - tread, s1),
            (0.0, z),
            (gal + 0.02, gal + 1.0),
        );
    }
    let foot = top_s - tread * steps as f64;
    pe.bx(
        b,
        "wall",
        (foot, top_s),
        (0.0, 0.25),
        (gal + 0.97, gal + 1.1),
    );
    for k in 0..=4 {
        let s = foot + (top_s - foot) * k as f64 / 4.0;
        let z = rise * (s - foot) / tread;
        pe.bar(
            b,
            "iron",
            (s, z.max(0.25)),
            (s, z + 0.95),
            gal + 1.04,
            0.015,
            6,
        );
    }
    pe.bar(
        b,
        "iron",
        (foot, 0.95 + 0.25),
        (top_s, ground + 0.97),
        gal + 1.04,
        0.025,
        6,
    );

    // In the courtyard: the orange tree in its bed, the trough, the
    // washing across from gallery to gallery.
    let ctr = [(px0 + px1) / 2.0, (py0 + py1) / 2.0];
    b.rbox(
        "stone",
        [ctr[0] - 0.6, ctr[1] - 0.6, 0.0],
        [ctr[0] + 0.6, ctr[1] + 0.6, 0.35],
        0.0,
    );
    fruit_tree(b, ctr, 3.6, seed, true);
    trough(b, [(px0 + px1) / 2.0, py0 + 0.6], 1.6);
    for k in 0..3 {
        let y = py0 + lp * (0.25 + 0.2 * k as f64);
        washing(
            b,
            [px0 + gal - 0.06, y],
            [px1 - gal + 0.06, y],
            ground + 0.97,
            seed + k,
        );
    }
}

/// Clothes lines from one railing to the other at `z`, with the washing.
fn washing(b: &mut Builder, a: [f64; 2], e: [f64; 2], z: f64, seed: u64) {
    let len = ((e[0] - a[0]).powi(2) + (e[1] - a[1]).powi(2)).sqrt();
    let f = Face {
        o: a,
        u: [(e[0] - a[0]) / len, (e[1] - a[1]) / len],
        len,
    };
    for dd in [-0.15, 0.15] {
        f.bar(b, "line", (0.0, z), (len, z), dd, 0.005, 4);
    }
    let roles = ["cloth_a", "cloth_b", "cloth_c"];
    let mut x = 0.3;
    let mut k = 0;
    while x < len - 0.6 {
        let wdt = 0.3 + 0.45 * unit(seed, 20 + k);
        let end = (x + wdt).min(len - 0.3);
        let dd = if unit(seed, 40 + k) > 0.5 {
            0.15
        } else {
            -0.15
        };
        let hang = 0.35 + 0.45 * unit(seed, 60 + k);
        let role = roles[(unit(seed, 80 + k) * 3.0) as usize % 3];
        f.bx(b, role, (x, end), (z - hang, z), (dd - 0.006, dd + 0.006));
        x = end + 0.15 + 0.4 * unit(seed, 100 + k);
        k += 1;
    }
}
