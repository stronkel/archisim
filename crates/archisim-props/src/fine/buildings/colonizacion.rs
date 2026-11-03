//! The houses of a settlement village (pueblo de colonización, 1950s–60s):
//! a row of whitewashed houses of plain volumes, each with a two-floor
//! piece and a one-floor piece under roofs of one slope that fall to
//! opposite sides, square windows set deep, the door in a painted frame,
//! a lattice of brick, the tall chimney with its pierced top, and next to
//! each house the yard (corral) behind its wall with the cart gate under
//! a little roof, its shed and its fig tree.

use super::more::*;
use super::*;
use crate::Params;

const WALL: f64 = 0.35;
const REVEAL: f64 = 0.2;

fn moved(f: &Face, dx: f64, dy: f64) -> Face {
    Face {
        o: [f.o[0] + dx, f.o[1] + dy],
        ..*f
    }
}

pub fn colonization_row(b: &mut Builder, p: &Params) {
    let houses = p.get("houses", 3.0).round().clamp(1.0, 8.0) as usize;
    let hw = p.get("house_width", 7.0).clamp(5.5, 10.0);
    let yw = p.get("yard_width", 4.0).clamp(3.0, 8.0);
    let d = p.get("depth", 10.0).clamp(7.0, 16.0);
    let seed = p.get("variant", 1.0).round().max(0.0) as u64;
    let unit_w = hw + yw;
    let w = unit_w * houses as f64;
    let low = 3.3;
    let high = 6.0;
    let pitch = 0.15;
    for i in 0..houses {
        let mirror = i % 2 == 1;
        let a = -w / 2.0 + unit_w * i as f64;
        // The house's span and the yard's along x.
        let (hx0, hx1, yx0, yx1) = if mirror {
            (a + yw, a + unit_w, a, a + yw)
        } else {
            (a, a + hw, a + hw, a + unit_w)
        };
        // The two-floor piece on the yard's side.
        let tw = hw * 0.45;
        let (tx0, tx1) = if mirror {
            (hx0, hx0 + tw)
        } else {
            (hx1 - tw, hx1)
        };
        let (lx0, lx1) = if mirror {
            (hx0 + tw, hx1)
        } else {
            (hx0, hx1 - tw)
        };
        let key = seed * 100 + i as u64;
        // The tall piece.
        let tall = Face::sides(tx1 - tx0, d).map(|f| moved(&f, (tx0 + tx1) / 2.0, 0.0));
        let tfront = tall[0];
        let door_on_tall = !mirror;
        let tw_ = tx1 - tx0;
        let mut th = vec![Hole {
            s: (tw_ / 2.0 - 0.45, tw_ / 2.0 + 0.45),
            z: (3.5, 4.4),
        }];
        let door = Hole {
            s: (tw_ / 2.0 - 0.5, tw_ / 2.0 + 0.5),
            z: (0.15, 2.4),
        };
        if door_on_tall {
            th.push(door);
        } else {
            th.push(Hole {
                s: (tw_ / 2.0 - 0.45, tw_ / 2.0 + 0.45),
                z: (1.0, 1.9),
            });
        }
        wall(b, "wall", &tfront, (0.0, high), WALL, &th);
        let back_h = [Hole {
            s: (tw_ / 2.0 - 0.35, tw_ / 2.0 + 0.35),
            z: (4.0, 4.7),
        }];
        wall(b, "wall", &tall[2], (0.0, high), WALL, &back_h);
        // The side to the yard has the lattice and a window.
        let (side, other) = if mirror {
            (tall[3], tall[1])
        } else {
            (tall[1], tall[3])
        };
        let ls = (d * 0.55 - 0.7, d * 0.55 + 0.7);
        let lz = (high - 1.9, high - 0.5);
        let side_win = Hole {
            s: (d * 0.2 - 0.4, d * 0.2 + 0.4),
            z: (1.0, 1.9),
        };
        wall(
            b,
            "wall",
            &side,
            (0.0, high),
            WALL,
            &[Hole { s: ls, z: lz }, side_win],
        );
        wall(b, "wall", &other, (0.0, high), WALL, &[]);
        // The low piece.
        let lowp = Face::sides(lx1 - lx0, d).map(|f| moved(&f, (lx0 + lx1) / 2.0, 0.0));
        let lw = lx1 - lx0;
        let mut lh = vec![Hole {
            s: (lw * 0.5 - 0.45, lw * 0.5 + 0.45),
            z: (1.0, 1.9),
        }];
        if !door_on_tall {
            lh[0] = Hole {
                s: (lw * 0.3 - 0.45, lw * 0.3 + 0.45),
                z: (1.0, 1.9),
            };
            lh.push(Hole {
                s: (lw * 0.72 - 0.5, lw * 0.72 + 0.5),
                z: (0.15, 2.4),
            });
        }
        wall(b, "wall", &lowp[0], (0.0, low), WALL, &lh);
        wall(
            b,
            "wall",
            &lowp[2],
            (0.0, low),
            WALL,
            &[Hole {
                s: (lw * 0.5 - 0.4, lw * 0.5 + 0.4),
                z: (1.1, 1.9),
            }],
        );
        let end = if mirror { lowp[1] } else { lowp[3] };
        wall(b, "party", &end, (0.0, low), WALL, &[]);
        b.rbox(
            "interior",
            [tx0 + WALL, WALL, 0.0],
            [tx1 - WALL, d - WALL, high],
            0.0,
        );
        b.rbox("interior", [lx0, WALL, 0.0], [lx1, d - WALL, low], 0.0);
        // Plinths.
        for f in [&tfront, &lowp[0]] {
            f.bx(b, "plinth", (0.0, f.len), (0.0, 0.5), (-0.01, 0.04));
        }
        // The roofs: the tall one falls to the back, the low one to the
        // street; the walls rise to them (their sides in triangles).
        let tall_rise = d * pitch;
        tfront.bx(
            b,
            "wall",
            (0.0, tfront.len),
            (high, high + tall_rise),
            (-WALL, 0.0),
        );
        pent_roof(
            b,
            &tfront,
            (-0.25, tfront.len + 0.25),
            (0.25, high + tall_rise + 0.04),
            (-d - 0.45, high - 0.45 * pitch),
        );
        for (k, f) in [tall[1], tall[3]].iter().enumerate() {
            let pts = if k == 0 {
                [(0.0, high), (d, high), (0.0, high + tall_rise)]
            } else {
                [(0.0, high), (d, high), (d, high + tall_rise)]
            };
            f.prism(b, "wall", &pts, (-WALL, 0.0));
        }
        let lf = lowp[0];
        lowp[2].bx(
            b,
            "wall",
            (0.0, lowp[2].len),
            (low, low + tall_rise),
            (-WALL, 0.0),
        );
        pent_roof(
            b,
            &lf,
            (-0.05, lf.len + 0.05),
            (-d - 0.25, low + tall_rise + 0.08),
            (0.45, low - 0.45 * pitch),
        );
        for (k, f) in [lowp[1], lowp[3]].iter().enumerate() {
            let pts = if k == 0 {
                [(0.0, low), (d, low), (d, low + tall_rise)]
            } else {
                [(0.0, low), (d, low), (0.0, low + tall_rise)]
            };
            f.prism(b, "wall", &pts, (-WALL, 0.0));
        }
        // The carpentry: deep square windows, the door in its frame.
        let frames = ["frame", "frame_b"];
        let fr = frames[(unit(key, 1) * 2.0) as usize % 2];
        for (f, hs) in [(&tfront, &th), (&lowp[0], &lh)] {
            for h in hs.iter() {
                if h.z.0 < 0.5 {
                    panel_door(b, f, *h, 1, 0.0, fr, "iron", REVEAL);
                    // The painted frame round the door, standing out.
                    f.bx(
                        b,
                        fr,
                        (h.s.0 - 0.18, h.s.0),
                        (0.0, h.z.1 + 0.18),
                        (-0.01, 0.08),
                    );
                    f.bx(
                        b,
                        fr,
                        (h.s.1, h.s.1 + 0.18),
                        (0.0, h.z.1 + 0.18),
                        (-0.01, 0.08),
                    );
                    f.bx(b, fr, (h.s.0, h.s.1), (h.z.1, h.z.1 + 0.18), (-0.01, 0.08));
                    f.bx(
                        b,
                        "step",
                        (h.s.0 - 0.1, h.s.1 + 0.1),
                        (0.0, 0.15),
                        (-REVEAL, 0.3),
                    );
                    standing_pot(b, f.at(h.s.1 + 0.45, 0.0, 0.25), 0.9, key + 3);
                } else {
                    let win = Win {
                        leaves: if h.s.1 - h.s.0 > 0.8 { 2 } else { 1 },
                        reveal: REVEAL,
                        curtain: unit(key, 4) > 0.3,
                        blind: if unit(key, 5) > 0.6 { 0.3 } else { 0.0 },
                        ..Default::default()
                    };
                    window(b, f, *h, &win, fr);
                    sill(b, f, *h, "plinth", 0.07, REVEAL);
                    if h.z.0 < 1.5 {
                        grille_plane(b, f, "iron", h.s, h.z, -0.04, 0.12, 2);
                    }
                }
            }
        }
        // The lattice of brick in its hole.
        let win = Win {
            leaves: 1,
            reveal: REVEAL,
            ..Default::default()
        };
        window(b, &side, side_win, &win, "frame");
        grille_plane(b, &side, "iron", side_win.s, side_win.z, -0.04, 0.12, 2);
        let lat = side.moved(-0.1);
        let rows = 7;
        for r in 0..rows {
            let z0 = lz.0 + (lz.1 - lz.0) * r as f64 / rows as f64;
            let z1 = z0 + (lz.1 - lz.0) / rows as f64;
            let off = if r % 2 == 0 { 0.0 } else { 0.12 };
            let mut x = ls.0 + off;
            while x < ls.1 - 0.1 {
                lat.bx(
                    b,
                    "brick",
                    (x, (x + 0.16).min(ls.1)),
                    (z0, z1),
                    (-0.02, 0.03),
                );
                x += 0.24;
            }
        }
        lat.bx(
            b,
            "plinth",
            (ls.0 - 0.06, ls.1 + 0.06),
            (lz.0 - 0.06, lz.0),
            (-0.02, 0.06),
        );
        // The chimney, tall, with its pierced top.
        let cx = (tx0 + tx1) / 2.0;
        let cy = d * 0.3;
        let roof_z = high + cy * pitch;
        b.rbox(
            "wall",
            [cx - 0.35, cy - 0.35, roof_z - 0.4],
            [cx + 0.35, cy + 0.35, roof_z + 1.5],
            0.0,
        );
        for (dx, dy) in [(-0.25, -0.25), (0.25, -0.25), (0.25, 0.25), (-0.25, 0.25)] {
            b.rbox(
                "wall",
                [cx + dx - 0.1, cy + dy - 0.1, roof_z + 1.5],
                [cx + dx + 0.1, cy + dy + 0.1, roof_z + 1.9],
                0.0,
            );
        }
        b.rbox(
            "plinth",
            [cx - 0.42, cy - 0.42, roof_z + 1.9],
            [cx + 0.42, cy + 0.42, roof_z + 2.0],
            0.0,
        );

        // The yard: its wall with the cart gate, the shed, the fig tree.
        let yf = Face {
            o: [yx0, d],
            u: [1.0, 0.0],
            len: yx1 - yx0,
        };
        let gate = Hole {
            s: (0.4, yf.len - 0.4),
            z: (0.0, 2.6),
        };
        wall(b, "wall", &yf, (0.0, 3.0), 0.3, &[gate]);
        yf.bx(b, "plinth", (0.0, yf.len), (0.0, 0.5), (-0.01, 0.03));
        panel_door(b, &yf, gate, 2, 0.0, "wood_b", "iron", 0.1);
        yf.leaning(
            b,
            "roof",
            (-0.1, yf.len + 0.1),
            (3.35, -0.3),
            (2.95, 0.6),
            0.06,
        );
        let n = ((yf.len + 0.2) / 0.21).round() as usize;
        for k in 0..n {
            let x = -0.1 + (yf.len + 0.2) * (k as f64 + 0.5) / n as f64;
            b.cone(
                "tiles",
                yf.at(x, 3.45, -0.3),
                yf.at(x, 3.05, 0.65),
                0.06,
                0.055,
                6,
            );
        }
        let yb = Face {
            o: [yx1, 0.0],
            u: [-1.0, 0.0],
            len: yx1 - yx0,
        };
        wall(b, "wall", &yb, (0.0, 2.5), 0.3, &[]);
        if i + 1 == houses && !mirror || i == 0 && mirror {
            let edge = if mirror {
                Face {
                    o: [yx0, 0.0],
                    u: [0.0, 1.0],
                    len: d,
                }
            } else {
                Face {
                    o: [yx1, d],
                    u: [0.0, -1.0],
                    len: d,
                }
            };
            wall(b, "wall", &edge, (0.0, 2.5), 0.3, &[]);
        }
        b.rbox("yard", [yx0, 0.3, 0.0], [yx1, d - 0.3, 0.02], 0.0);
        // The shed against the back wall.
        let shed = Face {
            o: [yx0, 0.3],
            u: [1.0, 0.0],
            len: yx1 - yx0,
        };
        pent_roof(b, &shed, (0.0, shed.len), (-0.02, 2.6), (2.0, 2.1));
        for x in [0.3, shed.len - 0.3] {
            shed.bar(b, "wood", (x, 0.02), (x, 2.12), 1.85, 0.07, 8);
        }
        fruit_tree(b, [(yx0 + yx1) / 2.0, d * 0.62], 3.4, key, false);
    }
}
