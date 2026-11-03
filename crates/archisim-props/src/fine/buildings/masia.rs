//! The Catalan farmhouse (masia de tipus basilical): a tall central body
//! of three floors with its gable to the front and two lower side bodies
//! under roofs of one slope; rough stone walls with dressed quoins and
//! window surrounds, the great doorway of long voussoirs, a balcony of
//! iron over it, the three little arches of the attic, a sundial, the
//! eaves of several courses of tiles, and before it the threshing floor
//! and two cypresses.

use super::ensanche::{arch_fill, voussoirs_of};
use super::more::*;
use super::*;
use crate::Params;

const WALL: f64 = 0.65;
const REVEAL: f64 = 0.2;

pub fn masia(b: &mut Builder, p: &Params) {
    let cw = p.get("width", 9.0).clamp(7.0, 14.0); // the central body
    let sw = p.get("side_width", 5.0).clamp(3.5, 8.0); // each side body
    let d = p.get("depth", 14.0).clamp(10.0, 22.0);
    let seed = p.get("variant", 1.0).round().max(0.0) as u64;
    let w = cw + 2.0 * sw;
    let fl = 3.2;
    let side_top = 2.0 * fl; // the side bodies' eaves at the outside
    let side_pitch = 0.32;
    let c_top = 3.0 * fl; // the central body's eaves
    let c_pitch = 0.45;
    let c_rise = cw / 2.0 * c_pitch;
    let [front, right, back, left] = Face::sides(w, d);

    // The front's holes: the doorway, a window each side, the balcony
    // door and windows, the three arches of the attic.
    let door = Hole {
        s: (w / 2.0 - 1.3, w / 2.0 + 1.3),
        z: (0.1, 2.4 + 1.3),
    };
    let mut hs = vec![door];
    let mut wins = Vec::new();
    for x in [sw / 2.0, w - sw / 2.0] {
        for (k, z) in [(1.1, 2.2), (fl + 1.0, fl + 2.3)].iter().enumerate() {
            let h = Hole {
                s: (x - 0.45, x + 0.45),
                z: (z.0, z.1),
            };
            hs.push(h);
            wins.push((h, k));
        }
    }
    for x in [w / 2.0 - 2.4, w / 2.0 + 2.4] {
        let h = Hole {
            s: (x - 0.45, x + 0.45),
            z: (1.2, 2.2),
        };
        hs.push(h);
        wins.push((h, 0));
    }
    let bal = Hole {
        s: (w / 2.0 - 0.6, w / 2.0 + 0.6),
        z: (fl + 0.05, fl + 2.4),
    };
    hs.push(bal);
    for x in [w / 2.0 - 2.4, w / 2.0 + 2.4] {
        let h = Hole {
            s: (x - 0.45, x + 0.45),
            z: (fl + 1.0, fl + 2.3),
        };
        hs.push(h);
        wins.push((h, 1));
    }
    let attic: Vec<Hole> = (0..3)
        .map(|i| {
            let x = w / 2.0 + (i as f64 - 1.0) * 1.2;
            Hole {
                s: (x - 0.4, x + 0.4),
                z: (2.0 * fl + 0.9, 2.0 * fl + 2.2),
            }
        })
        .collect();
    hs.extend(&attic);
    // The front wall: the side bodies' parts to their slope, the central
    // body to its gable.
    let lower: Vec<Hole> = hs
        .iter()
        .copied()
        .filter(|h| h.z.1 <= side_top + 0.01)
        .collect();
    wall(b, "stone_wall", &front, (0.0, side_top), WALL, &lower);
    let cf = Face {
        o: front.at(sw, 0.0, 0.0).map_xy(),
        len: cw,
        ..front
    };
    let upper: Vec<Hole> = attic
        .iter()
        .map(|h| Hole {
            s: (h.s.0 - sw, h.s.1 - sw),
            z: h.z,
        })
        .collect();
    wall(b, "stone_wall", &cf, (side_top, c_top), WALL, &upper);
    front.prism(
        b,
        "stone_wall",
        &[(sw, c_top), (sw + cw, c_top), (w / 2.0, c_top + c_rise)],
        (-WALL, 0.0),
    );
    for (s0, s1) in [(0.0, sw), (w - sw, w)] {
        // The side bodies' fronts rise to their roof at the central body.
        let pts = if s0 == 0.0 {
            [
                (s0, side_top),
                (s1, side_top),
                (s1, side_top + sw * side_pitch),
            ]
        } else {
            [
                (s0, side_top),
                (s1, side_top),
                (s0, side_top + sw * side_pitch),
            ]
        };
        front.prism(b, "stone_wall", &pts, (-WALL, 0.0));
    }
    // The other faces.
    let mut bh = Vec::new();
    for x in [sw / 2.0, w / 2.0, w - sw / 2.0] {
        bh.push(Hole {
            s: (x - 0.4, x + 0.4),
            z: (fl + 1.0, fl + 2.1),
        });
        bh.push(Hole {
            s: (x - 0.35, x + 0.35),
            z: (1.3, 2.1),
        });
    }
    wall(b, "stone_wall", &back, (0.0, side_top), WALL, &bh);
    let bf = Face {
        o: back.at(sw, 0.0, 0.0).map_xy(),
        len: cw,
        ..back
    };
    wall(b, "stone_wall", &bf, (side_top, c_top), WALL, &[]);
    back.prism(
        b,
        "stone_wall",
        &[(sw, c_top), (sw + cw, c_top), (w / 2.0, c_top + c_rise)],
        (-WALL, 0.0),
    );
    for (s0, s1, first) in [(0.0, sw, true), (w - sw, w, false)] {
        let pts = if first {
            [
                (s0, side_top),
                (s1, side_top),
                (s1, side_top + sw * side_pitch),
            ]
        } else {
            [
                (s0, side_top),
                (s1, side_top),
                (s0, side_top + sw * side_pitch),
            ]
        };
        back.prism(b, "stone_wall", &pts, (-WALL, 0.0));
    }
    for f in [right, left] {
        let n = ((d - 2.0) / 3.5).floor().max(1.0) as usize;
        let mut sh = Vec::new();
        for i in 0..n {
            let c = 1.0 + (d - 2.0) * (i as f64 + 0.5) / n as f64;
            sh.push(Hole {
                s: (c - 0.35, c + 0.35),
                z: (fl + 1.0, fl + 2.0),
            });
        }
        wall(b, "stone_wall", &f, (0.0, side_top), WALL, &sh);
        for h in &sh {
            let win = Win {
                leaves: 1,
                reveal: REVEAL,
                ..Default::default()
            };
            window(b, &f, *h, &win, "frame");
            surround(b, &f, *h, "quoin", 0.16, 0.03);
        }
    }
    // The central body's walls over the side roofs, on the sides.
    for (x, u) in [(-cw / 2.0, [0.0, 1.0]), (cw / 2.0, [0.0, -1.0])] {
        let o = if u[1] > 0.0 { [x, 0.0] } else { [x, d] };
        let f = Face { o, u, len: d };
        let n = ((d - 2.0) / 3.5).floor().max(1.0) as usize;
        let mut ch = Vec::new();
        for i in 0..n {
            let c = 1.0 + (d - 2.0) * (i as f64 + 0.5) / n as f64;
            ch.push(Hole {
                s: (c - 0.3, c + 0.3),
                z: (2.0 * fl + 1.2, 2.0 * fl + 2.0),
            });
        }
        wall(b, "stone_wall", &f, (side_top, c_top), 0.5, &ch);
        for h in &ch {
            let win = Win {
                leaves: 1,
                reveal: 0.15,
                ..Default::default()
            };
            window(b, &f, *h, &win, "frame");
        }
    }
    b.rbox(
        "interior",
        [-w / 2.0 + WALL, WALL, 0.0],
        [w / 2.0 - WALL, d - WALL, side_top],
        0.0,
    );
    b.rbox(
        "interior",
        [-cw / 2.0 + 0.5, WALL, side_top],
        [cw / 2.0 - 0.5, d - WALL, c_top],
        0.0,
    );

    // Quoins at every corner of the front and the back.
    for f in [&front, &back] {
        for (s, top) in [(0.0, side_top), (w, side_top), (sw, c_top), (w - sw, c_top)] {
            let mut z = 0.0;
            let mut i = 0;
            while z < top - 0.2 {
                let wide = if i % 2 == 0 { 0.55 } else { 0.32 };
                let (a, e) = if s < w / 2.0 {
                    (s, s + wide)
                } else {
                    (s - wide, s)
                };
                f.bx(b, "quoin", (a, e), (z, z + 0.36), (-0.01, 0.04));
                z += 0.38;
                i += 1;
            }
        }
    }
    // The doorway of long voussoirs, the leaves.
    let spring = 2.4;
    arch_fill(b, &front, "stone_wall", door.s, spring, (-WALL, 0.0));
    voussoirs_of(b, &front, door.s, spring, "quoin", 0.75);
    panel_door(
        b,
        &front,
        Hole {
            s: door.s,
            z: (0.1, spring),
        },
        2,
        0.0,
        "wood",
        "iron",
        0.35,
    );
    front.bx(b, "wood", door.s, (spring, door.z.1), (-0.37, -0.3));
    front.bx(
        b,
        "quoin",
        (door.s.0 - 0.3, door.s.1 + 0.3),
        (0.0, 0.1),
        (-0.35, 0.35),
    );
    // Windows with stone surrounds, the balcony, the attic's arches.
    for (j, (h, _)) in wins.iter().enumerate() {
        let win = Win {
            leaves: 2,
            reveal: REVEAL,
            bars: 2,
            curtain: unit(seed, j as u64) > 0.5,
            ..Default::default()
        };
        window(b, &front, *h, &win, "frame");
        surround(b, &front, *h, "quoin", 0.2, 0.04);
        sill(b, &front, *h, "quoin", 0.08, REVEAL);
        if h.z.0 < 1.5 {
            grille_plane(b, &front, "iron", h.s, h.z, -0.06, 0.12, 3);
        }
    }
    let win = Win {
        leaves: 2,
        reveal: REVEAL,
        bars: 3,
        ..Default::default()
    };
    window(b, &front, bal, &win, "frame");
    surround(b, &front, bal, "quoin", 0.22, 0.04);
    balcony(
        b,
        &front,
        (bal.s.0 - 0.35, bal.s.1 + 0.35),
        bal.z.0,
        0.55,
        "quoin",
        "iron",
        true,
    );
    for h in &attic {
        let r = (h.s.1 - h.s.0) / 2.0;
        arch_fill(
            b,
            &cf,
            "stone_wall",
            (h.s.0 - sw, h.s.1 - sw),
            h.z.1 - r,
            (-WALL, 0.0),
        );
        cf.bx(b, "glass", (h.s.0 - sw, h.s.1 - sw), h.z, (-0.35, -0.34));
        cf.bx(
            b,
            "quoin",
            (h.s.0 - sw - 0.05, h.s.1 - sw + 0.05),
            (h.z.0 - 0.1, h.z.0),
            (-WALL, 0.06),
        );
    }
    for h in &bh {
        let win = Win {
            leaves: if h.s.1 - h.s.0 > 0.75 { 2 } else { 1 },
            reveal: REVEAL,
            ..Default::default()
        };
        window(b, &back, *h, &win, "frame");
        surround(b, &back, *h, "quoin", 0.16, 0.03);
    }
    // The sundial on the front, beside the balcony.
    let sx = w / 2.0 + 3.6;
    let sz = 2.0 * fl + 0.6;
    front.bx(
        b,
        "sundial",
        (sx - 0.6, sx + 0.6),
        (sz, sz + 1.1),
        (-0.01, 0.02),
    );
    for k in 0..7 {
        let a = std::f64::consts::PI * (0.15 + 0.7 * k as f64 / 6.0);
        let (s, c) = a.sin_cos();
        front.bar(
            b,
            "iron",
            (sx, sz + 0.9),
            (sx + c * 0.5, sz + 0.9 - s * 0.75),
            0.022,
            0.006,
            4,
        );
    }
    front.bar(b, "iron", (sx, sz + 0.9), (sx, sz + 0.65), 0.02, 0.012, 4);
    b.cone(
        "iron",
        front.at(sx, sz + 0.9, 0.02),
        front.at(sx, sz + 0.6, 0.3),
        0.012,
        0.008,
        4,
    );

    // The roofs: the central one with its gable to the front, the side
    // ones of one slope; eaves of several courses.
    tile_roof_along_y(
        b,
        (-cw / 2.0, cw / 2.0),
        (0.0, d),
        c_top,
        c_pitch,
        (0.7, 0.45),
        (false, false),
    );
    for (x0, sgn) in [(-cw / 2.0, -1.0), (cw / 2.0, 1.0)] {
        let f = if sgn < 0.0 {
            Face {
                o: [x0, 0.0],
                u: [0.0, 1.0],
                len: d,
            }
        } else {
            Face {
                o: [x0, d],
                u: [0.0, -1.0],
                len: d,
            }
        };
        pent_roof(
            b,
            &f,
            (-0.45, d + 0.45),
            (-0.02, side_top + sw * side_pitch),
            (sw + 0.6, side_top - 0.6 * side_pitch),
        );
    }
    for (k, out) in [0.15, 0.3, 0.45].iter().enumerate() {
        let z = c_top - 0.1 - 0.12 * k as f64;
        front.bx(
            b,
            "tiles_eave",
            (sw - 0.1, sw + cw + 0.1),
            (z - 0.1, z),
            (-0.01, *out),
        );
    }
    let zc = c_top + (cw / 2.0 - 1.2) * c_pitch;
    chimney(b, [-cw / 2.0 + 1.2, d * 0.7], zc - 0.3, zc + 1.2);

    // The threshing floor and two cypresses.
    let era: Vec<P3> = (0..24)
        .map(|i| {
            let a = std::f64::consts::TAU * i as f64 / 24.0;
            [w * 0.32 + 4.5 * a.cos(), d + 7.0 + 4.5 * a.sin(), 0.0]
        })
        .collect();
    plate(b, "paving", &era, 0.08);
    for x in [-w / 2.0 - 2.0, w / 2.0 + 2.0] {
        b.cyl("trunk", [x, d + 1.5, 0.0], [x, d + 1.5, 1.0], 0.15);
        b.blob(
            "leaves",
            [x, d + 1.5, 5.0],
            [0.9, 0.0, 0.0],
            [0.0, 0.9, 0.0],
            [0.0, 0.0, 4.4],
        );
    }
    b.rbox(
        "yard",
        [-w / 2.0 - 4.0, d, 0.0],
        [w / 2.0 + 4.0, d + 12.0, 0.02],
        0.0,
    );
}
