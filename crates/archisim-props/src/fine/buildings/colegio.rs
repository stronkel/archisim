//! The primary school (colegio público, CEIP): the classrooms' block of
//! two floors in brick and white bands, with long windows and coloured
//! blinds, a covered porch on steel columns along its front, the entrance
//! with its canopy, the school's tiled name and three flags; the gym to
//! one side; the playground with its basketball court, swings and slide,
//! trees, benches and the fence with its gate to the street.

use super::more::*;
use super::*;
use crate::Params;

const WALL: f64 = 0.3;
const REVEAL: f64 = 0.1;

pub fn school(b: &mut Builder, p: &Params) {
    let seed = p.get("variant", 1.0).round().max(0.0) as u64;
    let bw = p.get("width", 40.0).clamp(24.0, 70.0);
    let bd = 10.0;
    let fl = 3.5;
    let top = 2.0 * fl;
    let yard = 24.0;
    // The block at the back, centred at x = −gw/2 so the gym fits.
    let gw = 18.0;
    let gd = 26.0;
    let total = bw + 2.0 + gw;
    let bx0 = -total / 2.0;
    let bc = bx0 + bw / 2.0;
    let sides = Face::sides(bw, bd).map(|f| Face {
        o: [f.o[0] + bc, f.o[1]],
        ..f
    });
    let [front, right, back, left] = sides;

    // The front: classrooms' windows in a row, the entrance in the middle.
    let ent = Hole {
        s: (bw / 2.0 - 1.6, bw / 2.0 + 1.6),
        z: (0.1, 2.8),
    };
    let mut holes = vec![ent];
    let mut wins = Vec::new();
    let n = ((bw - 2.0) / 3.6).floor() as usize;
    let step = (bw - 2.0) / n as f64;
    for k in 0..2 {
        for i in 0..n {
            let c = 1.0 + step * (i as f64 + 0.5);
            if k == 0 && (c - bw / 2.0).abs() < 2.6 {
                continue;
            }
            let h = Hole {
                s: (c - 1.35, c + 1.35),
                z: (fl * k as f64 + 0.95, fl * k as f64 + 2.75),
            };
            holes.push(h);
            wins.push(h);
        }
    }
    wall(b, "brick", &front, (0.0, top + 1.0), WALL, &holes);
    let mut bh = Vec::new();
    for k in 0..2 {
        for i in 0..n {
            let c = 1.0 + step * (i as f64 + 0.5);
            bh.push(Hole {
                s: (c - 0.9, c + 0.9),
                z: (fl * k as f64 + 1.2, fl * k as f64 + 2.6),
            });
        }
    }
    wall(b, "brick", &back, (0.0, top + 1.0), WALL, &bh);
    for f in [right, left] {
        wall(
            b,
            "brick",
            &f,
            (0.0, top + 1.0),
            WALL,
            &[Hole {
                s: (bd / 2.0 - 0.6, bd / 2.0 + 0.6),
                z: (fl + 1.0, fl + 2.6),
            }],
        );
    }
    b.rbox(
        "interior",
        [bx0 + WALL, WALL, 0.0],
        [bx0 + bw - WALL, bd - WALL, top + 0.8],
        0.0,
    );
    for f in [&front, &right, &back, &left] {
        f.bx(b, "plinth", (0.0, f.len), (0.0, 0.6), (-0.01, 0.03));
        f.bx(
            b,
            "band",
            (0.0, f.len),
            (fl - 0.25, fl + 0.05),
            (-0.01, 0.04),
        );
        f.bx(
            b,
            "band",
            (-0.05, f.len + 0.05),
            (top + 0.8, top + 1.05),
            (-0.01, 0.08),
        );
    }
    for (j, h) in wins.iter().chain(bh.iter()).enumerate() {
        let f = if j < wins.len() { &front } else { &back };
        let win = Win {
            leaves: 3,
            reveal: REVEAL,
            sliding: true,
            blind: [0.0, 0.3, 0.5, 0.0, 0.8][(unit(seed, j as u64) * 5.0) as usize % 5],
            blind_box: true,
            ..Default::default()
        };
        window(b, f, *h, &win, "frame");
        f.bx(
            b,
            "accent",
            (h.s.0 - 0.08, h.s.1 + 0.08),
            (h.z.0 - 0.1, h.z.0),
            (-REVEAL, 0.06),
        );
    }
    // The entrance: glass doors, a canopy, the name and the flags.
    let fd = (-REVEAL - 0.06, -REVEAL);
    for x in [ent.s.0, ent.s.0 + 0.8, ent.s.1 - 0.85, ent.s.1 - 0.05] {
        front.bx(b, "frame", (x, x + 0.05), ent.z, fd);
    }
    front.bx(b, "frame", ent.s, (ent.z.1 - 0.05, ent.z.1), fd);
    front.bx(b, "glass", ent.s, ent.z, (-REVEAL - 0.04, -REVEAL - 0.03));
    front.bx(
        b,
        "accent",
        (ent.s.0 - 1.0, ent.s.1 + 1.0),
        (3.0, 3.25),
        (-0.01, 2.2),
    );
    front.bx(
        b,
        "tile_panel",
        (bw / 2.0 - 2.5, bw / 2.0 + 2.5),
        (fl + 0.3, fl + 0.85),
        (-0.01, 0.04),
    );
    for (k, dx) in [-1.5, 0.0, 1.5].iter().enumerate() {
        let q = front.at(bw / 2.0 + dx * 1.2 - 4.5, 0.0, 3.5);
        flag_pole(b, [q[0], q[1]], 8.0, ["flag", "flag_b", "flag_c"][k]);
    }
    // The covered porch along the front (but the entrance).
    let pz = 3.0;
    let pd = 3.0;
    for (s0, s1) in [(0.4, ent.s.0 - 1.2), (ent.s.1 + 1.2, bw - 0.4)] {
        front.bx(b, "canopy", (s0, s1), (pz, pz + 0.2), (-0.01, pd));
        let m = ((s1 - s0) / 4.0).ceil() as usize;
        for i in 0..=m {
            let x = s0 + 0.15 + (s1 - s0 - 0.3) * i as f64 / m as f64;
            front.bar(b, "frame", (x, 0.0), (x, pz), pd - 0.2, 0.08, 10);
        }
    }
    // The flat roof: its floor, solar panels and a water tank.
    b.rbox(
        "roof_floor",
        [bx0 + 0.2, 0.2, top + 0.6],
        [bx0 + bw - 0.2, bd - 0.2, top + 0.8],
        0.0,
    );
    let np = ((bw - 4.0) / 1.2) as usize;
    for i in 0..np {
        let x = bx0 + 2.0 + 1.2 * i as f64;
        let c = [x + 0.5, 4.0, top + 1.15];
        let t: f64 = 0.45;
        b.obox(
            "panel",
            c,
            [0.5, 0.0, 0.0],
            [0.0, 0.85 * t.cos(), 0.85 * t.sin()],
            [0.0, -0.02 * t.sin(), 0.02 * t.cos()],
        );
        b.rbox(
            "metal",
            [x + 0.45, 3.25, top + 0.8],
            [x + 0.55, 3.3, top + 0.9],
            0.0,
        );
        b.rbox(
            "metal",
            [x + 0.45, 4.7, top + 0.8],
            [x + 0.55, 4.75, top + 1.55],
            0.0,
        );
    }

    // The gym: brick to three metres, panel above with a strip of glass,
    // a roof of low slope.
    let gx = bx0 + bw + 2.0 + gw / 2.0;
    let gh = 8.0;
    let gf = Face::sides(gw, gd).map(|f| Face {
        o: [f.o[0] + gx, f.o[1]],
        ..f
    });
    for (k, f) in gf.iter().enumerate() {
        let hs = if k == 0 {
            vec![Hole {
                s: (f.len / 2.0 - 1.5, f.len / 2.0 + 1.5),
                z: (0.0, 3.0),
            }]
        } else {
            Vec::new()
        };
        wall(b, "brick", f, (0.0, 3.0), WALL, &hs);
        let strip = Hole {
            s: (1.0, f.len - 1.0),
            z: (gh - 2.0, gh - 0.6),
        };
        wall(b, "panel_wall", f, (3.0, gh), WALL, &[strip]);
        f.bx(b, "glass", strip.s, strip.z, (-WALL * 0.5, -WALL * 0.4));
        let m = ((strip.s.1 - strip.s.0) / 1.5) as usize;
        for i in 0..=m {
            let x = strip.s.0 + (strip.s.1 - strip.s.0) * i as f64 / m as f64;
            f.bx(b, "frame", (x - 0.04, x + 0.04), strip.z, (-WALL, 0.03));
        }
        f.bx(b, "band", (0.0, f.len), (2.9, 3.1), (-0.01, 0.05));
        for hh in &hs {
            panel_door(b, f, *hh, 2, 0.0, "accent", "metal", 0.1);
        }
    }
    b.rbox(
        "interior",
        [gx - gw / 2.0 + WALL, WALL, 0.0],
        [gx + gw / 2.0 - WALL, gd - WALL, gh],
        0.0,
    );
    for sgn in [-1.0, 1.0] {
        let xe = gx + sgn * (gw / 2.0 + 0.3);
        plate(
            b,
            "metal_roof",
            &[
                [xe, -0.3, gh - 0.03],
                [gx, -0.3, gh + gw / 2.0 * 0.12],
                [gx, gd + 0.3, gh + gw / 2.0 * 0.12],
                [xe, gd + 0.3, gh - 0.03],
            ],
            0.08,
        );
    }
    for f in [&gf[0], &gf[2]] {
        f.prism(
            b,
            "panel_wall",
            &[(0.0, gh), (gw, gh), (gw / 2.0, gh + gw / 2.0 * 0.12)],
            (-WALL, 0.0),
        );
    }

    // The playground: its paving, the court, swings and slide, trees and
    // benches, and the fence with the gate.
    let py = bd;
    b.rbox(
        "yard",
        [-total / 2.0 - 1.0, py, 0.0],
        [total / 2.0 + 1.0, py + yard, 0.02],
        0.0,
    );
    let start = b.mark();
    super::super::courts::basketball_court(b, p);
    let cxc = bx0 + bw * 0.42;
    b.transform_from(start, |q| [q[0] + cxc, q[1] + py + 2.5, q[2] + 0.02]);
    for (k, x) in [bx0 + bw - 6.0, bx0 + bw - 2.0].iter().enumerate() {
        let start = b.mark();
        if k == 0 {
            super::super::playground::swing(b, p);
        } else {
            super::super::playground::slide(b, p);
        }
        b.transform_from(start, |q| [q[0] + x, q[1] + py + 4.0, q[2] + 0.02]);
    }
    for k in 0..4 {
        let x = gx - gw / 2.0 + 2.0 + k as f64 * 4.5;
        fruit_tree(b, [x, py + yard - 2.5], 4.5, seed + k, false);
        b.rbox(
            "stone",
            [x - 1.0, py + yard - 4.6, 0.0],
            [x + 1.0, py + yard - 4.2, 0.45],
            0.0,
        );
    }
    let fence = Face {
        o: [-total / 2.0 - 1.0, py + yard],
        u: [1.0, 0.0],
        len: total + 2.0,
    };
    let gate = (bc + total / 2.0 + 1.0 - 2.0, bc + total / 2.0 + 1.0 + 2.0);
    for (a, e) in [(0.0, gate.0), (gate.1, fence.len)] {
        fence.bx(b, "brick", (a, e), (0.0, 0.8), (-0.3, 0.0));
        fence.bx(b, "band", (a - 0.02, e + 0.02), (0.8, 0.86), (-0.32, 0.02));
        railing(
            b,
            &fence,
            "metal",
            (a + 0.1, e - 0.1),
            0.86,
            -0.15,
            1.2,
            0.12,
        );
    }
    for x in [gate.0 - 0.4, gate.1] {
        fence.bx(b, "brick", (x, x + 0.4), (0.0, 2.4), (-0.4, 0.05));
    }
    railing(
        b,
        &fence,
        "accent",
        (gate.0 + 0.05, gate.1 - 0.05),
        0.05,
        -0.15,
        1.9,
        0.11,
    );
    fence.bx(
        b,
        "tile_panel",
        (gate.0 - 0.4, gate.1 + 0.4),
        (2.4, 3.0),
        (-0.3, -0.25),
    );
}
