//! The Basque farmhouse (caserío, baserri): a big square block under a
//! wide roof of two low slopes with its gable to the front, the ground
//! floor of rough stone with the great arched door for the carts, the
//! gable above in timber framing over white infill, a balcony of wood
//! across it, windows with red or green shutters, the woodpile and an
//! apple tree.

use super::chalet80::shutters;
use super::ensanche::{arch_fill, voussoirs_of};
use super::more::*;
use super::*;
use crate::Params;

const WALL: f64 = 0.6;
const REVEAL: f64 = 0.18;

pub fn caserio(b: &mut Builder, p: &Params) {
    let w = p.get("width", 15.0).clamp(10.0, 22.0);
    let d = p.get("depth", 14.0).clamp(10.0, 22.0);
    let seed = p.get("variant", 1.0).round().max(0.0) as u64;
    let ground = 3.4;
    let first = 3.0;
    let top = ground + first; // the eaves on the sides
    let pitch = 0.42;
    let rise = w / 2.0 * pitch;
    let [front, right, back, left] = Face::sides(w, d);

    // The front: the arched door, windows downstairs; upstairs the
    // balcony's doors and windows; the gable's windows.
    let door = Hole {
        s: (w / 2.0 - 1.6, w / 2.0 + 1.6),
        z: (0.0, 2.2 + 1.6),
    };
    let mut fh = vec![door];
    for x in [w * 0.15, w * 0.85] {
        fh.push(Hole {
            s: (x - 0.45, x + 0.45),
            z: (1.1, 2.3),
        });
    }
    let mut up = Vec::new();
    for x in [w * 0.22, w * 0.42, w * 0.58, w * 0.78] {
        let h = Hole {
            s: (x - 0.5, x + 0.5),
            z: (ground + 0.05, ground + 2.2),
        };
        up.push(h);
        fh.push(h);
    }
    let gable_wins: Vec<Hole> = [w * 0.38, w * 0.62]
        .iter()
        .map(|x| Hole {
            s: (x - 0.4, x + 0.4),
            z: (top + 0.5, top + 1.5),
        })
        .collect();
    // The stone ground floor; the framed walls above.
    wall(
        b,
        "stone_wall",
        &front,
        (0.0, ground),
        WALL,
        &fh.iter()
            .copied()
            .filter(|h| h.z.0 < ground)
            .collect::<Vec<_>>(),
    );
    wall(b, "infill", &front, (ground, top), WALL, &up);
    // The gable: a band with its windows, triangles at its ends and over
    // it.
    let band = 1.6;
    let a = band * w / (2.0 * rise);
    wall(
        b,
        "infill",
        &Face {
            o: front.at(a, 0.0, 0.0).map_xy(),
            len: w - 2.0 * a,
            ..front
        },
        (top, top + band),
        WALL,
        &gable_wins
            .iter()
            .map(|h| Hole {
                s: (h.s.0 - a, h.s.1 - a),
                z: h.z,
            })
            .collect::<Vec<_>>(),
    );
    front.prism(
        b,
        "infill",
        &[(0.0, top), (a, top), (a, top + band)],
        (-WALL, 0.0),
    );
    front.prism(
        b,
        "infill",
        &[(w - a, top), (w, top), (w - a, top + band)],
        (-WALL, 0.0),
    );
    front.prism(
        b,
        "infill",
        &[(a, top + band), (w - a, top + band), (w / 2.0, top + rise)],
        (-WALL, 0.0),
    );
    for (k, f) in [right, back, left].iter().enumerate() {
        let n = ((f.len - 2.0) / 3.0).floor().max(1.0) as usize;
        let mut hs = Vec::new();
        for i in 0..n {
            let c = 1.0 + (f.len - 2.0) * (i as f64 + 0.5) / n as f64;
            hs.push(Hole {
                s: (c - 0.35, c + 0.35),
                z: (1.3, 2.2),
            });
            hs.push(Hole {
                s: (c - 0.45, c + 0.45),
                z: (ground + 0.9, ground + 2.1),
            });
        }
        wall(b, "stone_wall", f, (0.0, top), WALL, &hs);
        if k == 1 {
            f.prism(
                b,
                "stone_wall",
                &[(0.0, top), (w, top), (w / 2.0, top + rise)],
                (-WALL, 0.0),
            );
        }
        for h in &hs {
            let win = Win {
                leaves: if h.z.0 > ground { 2 } else { 1 },
                reveal: REVEAL,
                ..Default::default()
            };
            window(b, f, *h, &win, "frame");
            surround(b, f, *h, "quoin", 0.18, 0.03);
            sill(b, f, *h, "quoin", 0.06, REVEAL);
            if h.z.0 > ground {
                shutters(b, f, h, 1.0);
            }
        }
        // Stone quoins at the corners.
        let mut z = 0.0;
        let mut i = 0;
        while z < top - 0.2 {
            let wide = if i % 2 == 0 { 0.6 } else { 0.35 };
            f.bx(b, "quoin", (0.0, wide), (z, z + 0.38), (-0.01, 0.04));
            f.bx(
                b,
                "quoin",
                (f.len - wide, f.len),
                (z, z + 0.38),
                (-0.01, 0.04),
            );
            z += 0.4;
            i += 1;
        }
    }
    core(b, w, d, WALL, (0.0, top));
    // The door: its stone arch, the wooden leaves.
    let spring = 2.2;
    arch_fill(b, &front, "stone_wall", door.s, spring, (-WALL, 0.0));
    voussoirs_of(b, &front, door.s, spring, "quoin", 0.4);
    panel_door(
        b,
        &front,
        Hole {
            s: door.s,
            z: (0.0, spring),
        },
        2,
        0.0,
        "wood",
        "iron",
        0.3,
    );
    front.bx(b, "wood", door.s, (spring, door.z.1), (-0.32, -0.25));
    for h in fh.iter().filter(|h| h.z.0 < ground && h.s != door.s) {
        let win = Win {
            leaves: 1,
            reveal: REVEAL,
            ..Default::default()
        };
        window(b, &front, *h, &win, "frame");
        surround(b, &front, *h, "quoin", 0.18, 0.03);
        grille_plane(b, &front, "iron", h.s, h.z, -0.06, 0.12, 2);
    }
    // The framing over the white infill, the balcony across, the doors
    // and the gable's windows.
    timber_frame(b, &front, (0.0, w), (ground, top), 1.2, "timber");
    let n = 12;
    for i in 0..n {
        let x0 = w * i as f64 / n as f64;
        let x1 = w * (i + 1) as f64 / n as f64;
        let xm = (x0 + x1) / 2.0;
        let zt = top + rise * (1.0 - (2.0 * xm / w - 1.0).abs()) - 0.15;
        if zt > top + 0.3 {
            front.bx(
                b,
                "timber",
                (xm - 0.08, xm + 0.08),
                (top, zt),
                (-0.01, 0.04),
            );
        }
    }
    front.bx(
        b,
        "timber",
        (0.0, w),
        (top - 0.09, top + 0.09),
        (-0.01, 0.05),
    );
    for (j, h) in up.iter().enumerate() {
        let win = Win {
            leaves: 2,
            reveal: 0.12,
            bars: 2,
            curtain: unit(seed, j as u64) > 0.4,
            ..Default::default()
        };
        window(b, &front, *h, &win, "frame");
        shutters(b, &front, h, 1.0);
    }
    for h in &gable_wins {
        let win = Win {
            leaves: 2,
            reveal: 0.12,
            ..Default::default()
        };
        window(b, &front, *h, &win, "frame");
        shutters(b, &front, h, 1.0);
    }
    wooden_balcony(
        b,
        &front,
        (w * 0.12, w * 0.88),
        ground + 0.05,
        0.9,
        "timber",
        None,
    );
    for k in 0..4 {
        if unit(seed, 30 + k) > 0.3 {
            standing_pot(
                b,
                front.at(w * (0.2 + 0.18 * k as f64), ground + 0.05, 0.5),
                0.75,
                seed + k,
            );
        }
    }
    // The roof: two wide slopes, the gable to the front.
    tile_roof_along_y(
        b,
        (-w / 2.0, w / 2.0),
        (0.0, d),
        top,
        pitch,
        (0.9, 0.9),
        (false, false),
    );
    // The woodpile under the eaves and an apple tree.
    let lf = left;
    for row in 0..5 {
        let z = 0.2 * row as f64;
        lf.bx(b, "logs", (2.0, 6.0), (z, z + 0.2), (0.0, 0.6));
    }
    let mut x = 2.1;
    while x < 6.0 {
        for row in 0..5 {
            let c = lf.at(x, 0.1 + 0.2 * row as f64, 0.62);
            let o = lf.out();
            b.cone(
                "log_ends",
                [c[0] - o[0] * 0.02, c[1] - o[1] * 0.02, c[2]],
                [c[0] + o[0] * 0.01, c[1] + o[1] * 0.01, c[2]],
                0.08,
                0.08,
                8,
            );
        }
        x += 0.19;
    }
    fruit_tree(b, [w / 2.0 + 4.0, d + 2.5], 4.0, seed, true);
    b.rbox(
        "lawn",
        [-w / 2.0 - 6.0, -4.0, 0.0],
        [w / 2.0 + 7.0, d + 6.0, 0.02],
        0.0,
    );
    b.rbox("paving", [-2.0, d, 0.0], [2.0, d + 6.0, 0.03], 0.0);
}
