//! The Andalusian farmstead (cortijo): the owner's house of two floors at
//! the back of a yard of albero, the long barns of one floor on both sides
//! with their great doors, the oil mill's counterweight tower (torre de
//! contrapeso) with its pyramid of tiles, and the front wall with the
//! gateway under its bell gable (espadaña); in the yard the well, the
//! trough, olive trees and a palm.

use super::more::*;
use super::*;
use crate::Params;

const WALL: f64 = 0.5;
const REVEAL: f64 = 0.18;

pub fn cortijo(b: &mut Builder, p: &Params) {
    let w = p.get("width", 32.0).clamp(22.0, 50.0);
    let d = p.get("depth", 28.0).clamp(20.0, 50.0);
    let seed = p.get("variant", 1.0).round().max(0.0) as u64;
    let tower = p.flag("tower", true);
    let hd = 8.0; // the house's depth
    let nw = 7.0; // the barns' width
    let ground = 3.4;
    let top = ground + 3.0;
    let barn = 4.3;

    // The house at the back, its front to the yard.
    let hf = Face {
        o: [-w / 2.0, hd],
        u: [1.0, 0.0],
        len: w,
    };
    let hb = Face {
        o: [w / 2.0, 0.0],
        u: [-1.0, 0.0],
        len: w,
    };
    let hl = Face {
        o: [-w / 2.0, 0.0],
        u: [0.0, 1.0],
        len: hd,
    };
    let hr = Face {
        o: [w / 2.0, hd],
        u: [0.0, -1.0],
        len: hd,
    };
    // Its bays over the yard (between the barns).
    let (ya0, ya1) = (nw + 0.5, w - nw - 0.5);
    let n = ((ya1 - ya0) / 2.6).floor().max(3.0) as usize;
    let n = if n.is_multiple_of(2) { n - 1 } else { n };
    let bay = (ya1 - ya0) / n as f64;
    let axis = |i: usize| ya0 + bay * (i as f64 + 0.5);
    let mid = n / 2;
    let door = Hole {
        s: (axis(mid) - 0.8, axis(mid) + 0.8),
        z: (0.15, 2.9),
    };
    let mut holes = vec![door];
    let mut wins = Vec::new();
    for i in 0..n {
        if i != mid {
            let h = Hole {
                s: (axis(i) - 0.55, axis(i) + 0.55),
                z: (0.9, 2.5),
            };
            holes.push(h);
            wins.push((h, 0, i));
        }
        let h = Hole {
            s: (axis(i) - 0.5, axis(i) + 0.5),
            z: (ground + 0.05, ground + 2.3),
        };
        holes.push(h);
        wins.push((h, 1, i));
    }
    wall(b, "wall", &hf, (0.0, top), WALL, &holes);
    let mut bh = Vec::new();
    for i in 0..(w / 4.0) as usize {
        let x = 2.0 + 4.0 * i as f64;
        bh.push(Hole {
            s: (x, x + 0.8),
            z: (ground + 1.0, ground + 2.0),
        });
        bh.push(Hole {
            s: (x + 0.1, x + 0.7),
            z: (1.3, 2.1),
        });
    }
    wall(b, "wall", &hb, (0.0, top), WALL, &bh);
    for f in [hl, hr] {
        wall(
            b,
            "wall",
            &f,
            (0.0, top),
            WALL,
            &[Hole {
                s: (hd / 2.0 - 0.4, hd / 2.0 + 0.4),
                z: (ground + 1.0, ground + 2.0),
            }],
        );
    }
    b.rbox(
        "interior",
        [-w / 2.0 + WALL, WALL, 0.0],
        [w / 2.0 - WALL, hd - WALL, top],
        0.0,
    );
    for f in [&hf, &hb] {
        f.bx(b, "plinth", (0.0, f.len), (0.0, 0.9), (-0.01, 0.03));
        f.bx(b, "trim", (0.0, f.len), (top - 0.3, top), (-0.01, 0.12));
    }
    // The door with its stone surround and the balcony over it.
    panel_door(b, &hf, door, 2, 0.0, "wood", "iron", REVEAL);
    surround(b, &hf, door, "stone", 0.25, 0.06);
    hf.bx(
        b,
        "stone",
        (door.s.0 - 0.35, door.s.1 + 0.35),
        (door.z.1 + 0.25, door.z.1 + 0.4),
        (-0.01, 0.12),
    );
    hf.bx(
        b,
        "step",
        (door.s.0 - 0.2, door.s.1 + 0.2),
        (0.0, 0.15),
        (-REVEAL, 0.45),
    );
    for (j, (h, fl, i)) in wins.iter().enumerate() {
        let key = seed * 11 + j as u64;
        let win = Win {
            leaves: 2,
            reveal: REVEAL,
            bars: 2,
            curtain: unit(key, 1) > 0.4,
            ..Default::default()
        };
        window(b, &hf, *h, &win, "frame");
        surround(b, &hf, *h, "trim", 0.16, 0.015);
        if *fl == 0 {
            sill(b, &hf, *h, "step", 0.06, REVEAL);
            projecting_grille(b, &hf, *h, "iron", 0.18);
        } else if *i == mid {
            balcony(
                b,
                &hf,
                (h.s.0 - 0.4, h.s.1 + 0.4),
                h.z.0,
                0.6,
                "stone",
                "iron",
                true,
            );
        } else {
            sill(b, &hf, *h, "step", 0.05, REVEAL);
            grille_plane(b, &hf, "iron", h.s, (h.z.0, h.z.0 + 0.95), 0.03, 0.11, 2);
        }
    }
    for h in &bh {
        let win = Win {
            leaves: if h.s.1 - h.s.0 > 0.7 { 2 } else { 1 },
            reveal: 0.15,
            ..Default::default()
        };
        window(b, &hb, *h, &win, "frame");
        grille_plane(b, &hb, "iron", h.s, h.z, 0.02, 0.12, 2);
    }
    wall_lantern(b, &hf, door.s.1 + 0.6, 3.05);
    tile_roof(
        b,
        (-w / 2.0, w / 2.0),
        (0.0, hd),
        top,
        0.42,
        (0.4, 0.15),
        (true, true),
    );
    chimney(
        b,
        [-w * 0.3, hd * 0.3],
        top + hd * 0.3 * 0.42 - 0.3,
        top + hd * 0.3 * 0.42 + 0.9,
    );
    chimney(
        b,
        [w * 0.18, hd * 0.3],
        top + hd * 0.3 * 0.42 - 0.3,
        top + hd * 0.3 * 0.42 + 0.9,
    );

    // The barns.
    for east in [false, true] {
        let x0 = if east { w / 2.0 - nw } else { -w / 2.0 };
        let x1 = x0 + nw;
        let len = d - hd;
        let (yard, out, end) = if east {
            (
                Face {
                    o: [x0, hd],
                    u: [0.0, 1.0],
                    len,
                },
                Face {
                    o: [x1, d],
                    u: [0.0, -1.0],
                    len,
                },
                Face {
                    o: [x0, d],
                    u: [1.0, 0.0],
                    len: nw,
                },
            )
        } else {
            (
                Face {
                    o: [x1, d],
                    u: [0.0, -1.0],
                    len,
                },
                Face {
                    o: [x0, hd],
                    u: [0.0, 1.0],
                    len,
                },
                Face {
                    o: [x0, d],
                    u: [1.0, 0.0],
                    len: nw,
                },
            )
        };
        let k = ((len - 2.0) / 7.0).floor().max(1.0) as usize;
        let mut doors = Vec::new();
        let mut hs = Vec::new();
        for i in 0..k {
            let c = 1.0 + (len - 2.0) * (i as f64 + 0.5) / k as f64;
            let h = Hole {
                s: (c - 1.5, c + 1.5),
                z: (0.0, 3.3),
            };
            doors.push(h);
            hs.push(h);
            for dx in [-2.6, 2.6] {
                if c + dx > 0.8 && c + dx < len - 0.8 {
                    hs.push(Hole {
                        s: (c + dx - 0.35, c + dx + 0.35),
                        z: (2.3, 3.0),
                    });
                }
            }
        }
        wall(b, "wall", &yard, (0.0, barn), WALL, &hs);
        let mut oh = Vec::new();
        let m = (len / 3.5) as usize;
        for i in 0..m {
            let c = 1.75 + 3.5 * i as f64;
            oh.push(Hole {
                s: (c - 0.3, c + 0.3),
                z: (2.6, 3.4),
            });
        }
        wall(b, "wall", &out, (0.0, barn), WALL, &oh);
        let gate = Hole {
            s: (nw / 2.0 - 1.6, nw / 2.0 + 1.6),
            z: (0.0, 3.4),
        };
        wall(b, "wall", &end, (0.0, barn), WALL, &[gate]);
        b.rbox(
            "interior",
            [x0 + WALL, hd, 0.0],
            [x1 - WALL, d - WALL, barn],
            0.0,
        );
        for f in [&yard, &out, &end] {
            f.bx(b, "plinth", (0.0, f.len), (0.0, 0.7), (-0.01, 0.03));
        }
        for h in doors.iter().chain(std::iter::once(&gate)) {
            let f = if h.s == gate.s && h.z == gate.z {
                &end
            } else {
                &yard
            };
            panel_door(b, f, *h, 2, 0.0, "wood", "iron", 0.25);
            f.bx(
                b,
                "wood",
                (h.s.0 - 0.3, h.s.1 + 0.3),
                (h.z.1, h.z.1 + 0.3),
                (-0.01, 0.06),
            );
            surround(b, f, *h, "trim", 0.18, 0.012);
        }
        for h in hs.iter().filter(|h| h.z.0 > 1.0).chain(oh.iter()) {
            let f = if oh.iter().any(|o| o.s == h.s && o.z == h.z) {
                &out
            } else {
                &yard
            };
            let win = Win {
                leaves: 1,
                reveal: 0.2,
                ..Default::default()
            };
            window(b, f, *h, &win, "frame");
            grille_plane(b, f, "iron", h.s, h.z, -0.05, 0.1, 2);
        }
        tile_roof_along_y(b, (x0, x1), (hd, d), barn, 0.4, (0.35, 0.2), (true, true));
    }

    // The counterweight tower, at the house's corner of the east barn.
    if tower {
        let s = 4.6;
        let c = [w / 2.0 - nw / 2.0, hd + s / 2.0 + 0.3];
        let th = 12.5;
        let tf = Face::sides(s, s);
        for (k, f) in tf.iter().enumerate() {
            let f = Face {
                o: [f.o[0] + c[0], f.o[1] + c[1] - s / 2.0],
                ..*f
            };
            let mut hs = Vec::new();
            for (i, z) in [5.5, 8.5, 11.0].iter().enumerate() {
                if (k + i) % 2 == 0 {
                    hs.push(Hole {
                        s: (s / 2.0 - 0.35, s / 2.0 + 0.35),
                        z: (*z, *z + 0.9),
                    });
                }
            }
            wall(b, "wall", &f, (0.0, th), 0.6, &hs);
            for h in &hs {
                grille_plane(b, &f, "iron", h.s, h.z, -0.1, 0.1, 2);
                let win = Win {
                    leaves: 1,
                    reveal: 0.25,
                    ..Default::default()
                };
                window(b, &f, *h, &win, "frame");
            }
            f.bx(b, "trim", (-0.1, s + 0.1), (th - 0.35, th), (-0.01, 0.14));
            f.bx(b, "plinth", (0.0, s), (0.0, 0.9), (-0.01, 0.03));
            // Buttresses at the corners, leaning in as they rise.
            for sx in [0.0, s - 0.7] {
                let side = Face {
                    o: f.at(sx + 0.35, 0.0, 0.0).map_xy(),
                    u: f.out(),
                    len: 0.6,
                };
                side.prism(
                    b,
                    "wall",
                    &[(-0.02, 0.0), (0.6, 0.0), (0.6, 0.8), (-0.02, 6.0)],
                    (-0.3, 0.3),
                );
            }
        }
        b.rbox(
            "interior",
            [c[0] - s / 2.0 + 0.6, c[1] - s / 2.0 + 0.6, 0.0],
            [c[0] + s / 2.0 - 0.6, c[1] + s / 2.0 - 0.6, th],
            0.0,
        );
        hipped_roof(
            b,
            (c[0] - s / 2.0, c[0] + s / 2.0),
            (c[1] - s / 2.0, c[1] + s / 2.0),
            th,
            0.95,
            0.3,
        );
        let tip = th + (s / 2.0 + 0.3) * 0.95 - 0.3 * 0.95 + 0.05;
        b.cyl("iron", [c[0], c[1], tip], [c[0], c[1], tip + 1.0], 0.02);
        b.cyl(
            "iron",
            [c[0] - 0.35, c[1], tip + 0.75],
            [c[0] + 0.35, c[1], tip + 0.75],
            0.015,
        );
        b.ellipsoid("iron", [c[0], c[1], tip + 1.02], [0.05, 0.05, 0.05]);
    }

    // The front wall with the gateway and its bell gable.
    let fw = Face {
        o: [-w / 2.0 + nw, d],
        u: [1.0, 0.0],
        len: w - 2.0 * nw,
    };
    let gw = 4.0;
    let gc = fw.len / 2.0;
    let wall_h = 3.2;
    tapia(b, &fw, (0.0, gc - gw / 2.0 - 0.6), wall_h, "wall");
    tapia(b, &fw, (gc + gw / 2.0 + 0.6, fw.len), wall_h, "wall");
    let spring = 3.4;
    let gh = Hole {
        s: (gc - gw / 2.0, gc + gw / 2.0),
        z: (0.0, spring + gw / 2.0),
    };
    let ptop = spring + gw / 2.0 + 0.8;
    wall(
        b,
        "wall",
        &Face {
            o: fw.at(gc - gw / 2.0 - 0.6, 0.0, 0.2).map_xy(),
            u: fw.u,
            len: gw + 1.2,
        },
        (0.0, ptop),
        0.7,
        &[Hole {
            s: (0.6, 0.6 + gw),
            z: gh.z,
        }],
    );
    super::ensanche::arch_fill(b, &fw.moved(0.2), "wall", gh.s, spring, (-0.7, 0.0));
    for (a, e) in [(gh.s.0 - 0.6, gh.s.0 - 0.05), (gh.s.1 + 0.05, gh.s.1 + 0.6)] {
        fw.bx(b, "trim", (a, e), (0.0, ptop), (0.15, 0.32));
    }
    fw.bx(
        b,
        "trim",
        (gh.s.0 - 0.7, gh.s.1 + 0.7),
        (ptop, ptop + 0.2),
        (0.1, 0.36),
    );
    fw.bx(
        b,
        "tile_panel",
        (gc - 0.6, gc + 0.6),
        (spring + gw / 2.0 + 0.15, ptop - 0.12),
        (0.2, 0.23),
    );
    // The bell gable.
    let eg = Face {
        o: fw.at(gc - 1.3, 0.0, 0.2).map_xy(),
        u: fw.u,
        len: 2.6,
    };
    let bz = ptop + 0.2;
    let bell_hole = Hole {
        s: (0.8, 1.8),
        z: (bz + 0.4, bz + 1.4),
    };
    wall(b, "wall", &eg, (bz, bz + 1.9), 0.45, &[bell_hole]);
    super::ensanche::arch_fill(b, &eg, "wall", bell_hole.s, bz + 1.4 - 0.5, (-0.45, 0.0));
    eg.prism(
        b,
        "wall",
        &[(0.0, bz + 1.9), (2.6, bz + 1.9), (1.3, bz + 2.6)],
        (-0.45, 0.0),
    );
    for (a, e) in [(-0.06, 1.3), (2.66, 1.3)] {
        plate(
            b,
            "tiles",
            &[
                eg.at(a, bz + 1.88, -0.5),
                eg.at(e, bz + 2.6, -0.5),
                eg.at(e, bz + 2.6, 0.05),
                eg.at(a, bz + 1.88, 0.05),
            ],
            0.06,
        );
    }
    let bc = eg.at(1.3, bz + 1.0, -0.225);
    b.cyl(
        "iron",
        eg.at(0.8, bz + 1.32, -0.225),
        eg.at(1.8, bz + 1.32, -0.225),
        0.02,
    );
    b.lathe(
        "brass",
        [bc[0], bc[1], bz + 0.62],
        &[
            (0.0, 0.0),
            (0.24, 0.02),
            (0.2, 0.2),
            (0.12, 0.62),
            (0.03, 0.7),
            (0.0, 0.7),
        ],
        1.0,
        1.0,
    );
    b.cyl(
        "iron",
        eg.at(1.3, bz + 2.6, -0.225),
        eg.at(1.3, bz + 3.1, -0.225),
        0.02,
    );
    b.cyl(
        "iron",
        eg.at(1.12, bz + 2.95, -0.225),
        eg.at(1.48, bz + 2.95, -0.225),
        0.018,
    );
    // The gates, open into the yard.
    for (x, sgn) in [(gh.s.0, 1.0), (gh.s.1, -1.0)] {
        let side = Face {
            o: fw.at(x, 0.0, -0.5).map_xy(),
            u: [0.0, -1.0],
            len: 1.9,
        };
        side.bx(b, "wood", (0.0, 1.9), (0.05, spring), (0.0, 0.08 * sgn));
    }

    // The yard: albero, the well, the trough, olive trees, a palm, pots.
    b.rbox(
        "yard",
        [-w / 2.0 + nw, hd, 0.0],
        [w / 2.0 - nw, d, 0.02],
        0.0,
    );
    let yc = [0.0, (hd + d) / 2.0];
    well(b, [yc[0] - (w / 2.0 - nw) * 0.45, yc[1] + 1.0]);
    trough(b, [yc[0] + (w / 2.0 - nw) * 0.4, d - 2.5], 3.0);
    for (i, (dx, dy)) in [(-0.25, 0.25), (0.3, -0.1)].iter().enumerate() {
        fruit_tree(
            b,
            [yc[0] + dx * (w - 2.0 * nw), yc[1] + dy * (d - hd)],
            4.2,
            seed + i as u64,
            false,
        );
    }
    palm(b, [door.s.1 + 2.5 - w / 2.0, hd + 2.0], 7.5, seed);
    for k in 0..6 {
        let x = -w / 2.0 + ya0 + 1.0 + k as f64 * (ya1 - ya0 - 2.0) / 5.0;
        if (x - (axis(mid) - w / 2.0)).abs() < 1.4 {
            continue;
        }
        standing_pot(b, [x, hd + 0.5, 0.02], 1.0, seed + 40 + k);
    }
}
