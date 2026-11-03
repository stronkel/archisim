//! The health centre (centro de salud): a white block of two floors with
//! bands of windows, vertical fins shading the upper floor, a lobby of
//! glass two floors high at the entrance under a cantilevered canopy with
//! its sign and green cross, the ramp with its handrails and the steps,
//! the ambulance's bay under its own canopy at the side, the machines on
//! the roof, parking lines and trees.

use super::more::*;
use super::*;
use crate::Params;

const WALL: f64 = 0.3;
const REVEAL: f64 = 0.08;

pub fn health_centre(b: &mut Builder, p: &Params) {
    let w = p.get("width", 32.0).clamp(18.0, 60.0);
    let d = p.get("depth", 14.0).clamp(10.0, 24.0);
    let seed = p.get("variant", 1.0).round().max(0.0) as u64;
    let fl = 3.6;
    let top = 2.0 * fl + 0.6;
    let [front, right, back, left] = Face::sides(w, d);
    let lobby = (w * 0.3, w * 0.3 + 6.0);

    // Bands of windows: the ground floor's and the upper floor's, cut by
    // the lobby.
    let mut holes = vec![Hole {
        s: lobby,
        z: (0.0, 2.0 * fl - 0.2),
    }];
    let mut bands_h = Vec::new();
    for k in 0..2 {
        let z = (fl * k as f64 + 1.0, fl * k as f64 + 2.7);
        for (a, e) in [(1.0, lobby.0 - 1.0), (lobby.1 + 1.0, w - 1.0)] {
            if e - a > 1.5 {
                let h = Hole { s: (a, e), z };
                holes.push(h);
                bands_h.push((h, k));
            }
        }
    }
    wall(b, "wall", &front, (0.0, top), WALL, &holes);
    for f in [right, back, left] {
        let mut hs = Vec::new();
        for k in 0..2 {
            let z = (fl * k as f64 + 1.0, fl * k as f64 + 2.7);
            let m = ((f.len - 2.0) / 4.0).floor().max(1.0) as usize;
            for i in 0..m {
                let c = 1.0 + (f.len - 2.0) * (i as f64 + 0.5) / m as f64;
                hs.push(Hole {
                    s: (c - 1.2, c + 1.2),
                    z,
                });
            }
        }
        wall(b, "wall", &f, (0.0, top), WALL, &hs);
        for h in &hs {
            let win = Win {
                leaves: 2,
                reveal: REVEAL,
                sliding: true,
                blind: 0.4 * unit(seed, (h.s.0 * 7.0 + h.z.0) as u64),
                blind_box: true,
                ..Default::default()
            };
            window(b, &f, *h, &win, "frame");
            f.bx(
                b,
                "frame",
                (h.s.0 - 0.05, h.s.1 + 0.05),
                (h.z.0 - 0.06, h.z.0),
                (-REVEAL, 0.06),
            );
        }
        f.bx(b, "plinth", (0.0, f.len), (0.0, 0.5), (-0.01, 0.03));
        f.bx(
            b,
            "band",
            (-0.05, f.len + 0.05),
            (top - 0.35, top),
            (-0.01, 0.1),
        );
    }
    core(b, w, d, WALL, (0.0, top - 0.2));
    front.bx(b, "plinth", (0.0, lobby.0), (0.0, 0.5), (-0.01, 0.03));
    front.bx(b, "plinth", (lobby.1, w), (0.0, 0.5), (-0.01, 0.03));
    front.bx(
        b,
        "band",
        (-0.05, w + 0.05),
        (top - 0.35, top),
        (-0.01, 0.1),
    );
    // The front's bands of windows in panes, the upper one behind fins.
    for (h, k) in &bands_h {
        let n = ((h.s.1 - h.s.0) / 1.6).round().max(1.0) as usize;
        let l = (h.s.1 - h.s.0) / n as f64;
        for i in 0..n {
            let hh = Hole {
                s: (h.s.0 + l * i as f64, h.s.0 + l * (i + 1) as f64),
                z: h.z,
            };
            let win = Win {
                leaves: 1,
                reveal: REVEAL,
                blind: if unit(seed, 50 + i as u64 + 20 * *k as u64) > 0.6 {
                    0.4
                } else {
                    0.0
                },
                ..Default::default()
            };
            window(b, &front, hh, &win, "frame");
        }
        front.bx(
            b,
            "band",
            (h.s.0 - 0.1, h.s.1 + 0.1),
            (h.z.0 - 0.25, h.z.0),
            (-0.01, 0.12),
        );
        if *k == 1 {
            let mut x = h.s.0 - 0.1;
            while x <= h.s.1 + 0.1 {
                front.bx(
                    b,
                    "fins",
                    (x - 0.04, x + 0.04),
                    (h.z.0 - 0.25, h.z.1 + 0.35),
                    (0.12, 0.62),
                );
                x += 0.55;
            }
            front.bx(
                b,
                "band",
                (h.s.0 - 0.1, h.s.1 + 0.1),
                (h.z.1 + 0.35, h.z.1 + 0.5),
                (-0.01, 0.65),
            );
        }
    }
    // The lobby: a wall of glass two floors high, its doors.
    let (l0, l1) = lobby;
    let lz = (0.0, 2.0 * fl - 0.2);
    let m = ((l1 - l0) / 1.5).round() as usize;
    for i in 0..=m {
        let x = l0 + (l1 - l0) * i as f64 / m as f64;
        front.bx(b, "frame", (x - 0.04, x + 0.04), lz, (-0.18, -0.06));
    }
    for z in [2.6, fl, fl + 2.6, lz.1 - 0.06] {
        front.bx(b, "frame", lobby, (z, z + 0.06), (-0.18, -0.06));
    }
    front.bx(b, "glass", lobby, lz, (-0.14, -0.13));
    front.bx(b, "frame", (l0 + 2.2, l0 + 3.8), (0.0, 2.6), (-0.13, -0.06));
    // The canopy, its sign and the green cross.
    let cz = 3.3;
    front.bx(
        b,
        "canopy",
        (l0 - 1.0, l1 + 1.0),
        (cz, cz + 0.35),
        (-0.01, 3.5),
    );
    front.bx(
        b,
        "sign",
        (l0 - 0.6, l1 - 1.5),
        (cz + 0.35, cz + 0.8),
        (2.9, 3.45),
    );
    let cx = l1 - 0.6;
    front.bx(
        b,
        "cross",
        (cx - 0.15, cx + 0.15),
        (cz + 0.35, cz + 1.25),
        (3.1, 3.3),
    );
    front.bx(
        b,
        "cross",
        (cx - 0.45, cx + 0.45),
        (cz + 0.65, cz + 0.95),
        (3.1, 3.3),
    );
    // Steps and the ramp with its handrails.
    front.bx(b, "step", (l0 - 0.5, l1 + 0.5), (0.0, 0.15), (-0.01, 3.0));
    for k in 0..2 {
        front.bx(
            b,
            "step",
            (l0 + 0.5 - 0.0, l0 + 3.0),
            (0.0, 0.15 - 0.075 * k as f64),
            (3.0 + 0.3 * k as f64, 3.3 + 0.3 * k as f64),
        );
    }
    let ramp = (l1 + 0.6, l1 + 2.1);
    let rl = 6.0;
    plate(
        b,
        "step",
        &[
            front.at(ramp.0, 0.15, 0.0),
            front.at(ramp.1, 0.15, 0.0),
            front.at(ramp.1, 0.0, rl),
            front.at(ramp.0, 0.0, rl),
        ],
        0.05,
    );
    for x in [ramp.0, ramp.1] {
        for t in 0..=4 {
            let dd = rl * t as f64 / 4.0;
            let z = 0.15 * (1.0 - dd / rl);
            front.bar(b, "metal", (x, z), (x, z + 0.95), dd.max(0.06), 0.022, 8);
        }
        b.cone(
            "metal",
            front.at(x, 0.15 + 0.95, 0.06),
            front.at(x, 0.95, rl),
            0.025,
            0.025,
            8,
        );
    }

    // The ambulance's bay at the right end: a canopy on two columns.
    let (a0, a1) = (w - 7.0, w - 0.5);
    front.bx(b, "canopy", (a0, a1), (4.2, 4.5), (-0.01, 6.0));
    for x in [a0 + 0.3, a1 - 0.3] {
        front.bar(b, "frame", (x, 0.0), (x, 4.2), 5.7, 0.12, 12);
    }
    front.bx(b, "line", (a0 + 0.5, a1 - 0.5), (0.0, 0.02), (1.0, 1.1));
    front.bx(b, "line", (a0 + 0.5, a0 + 0.6), (0.0, 0.02), (1.0, 5.8));
    front.bx(b, "line", (a1 - 0.6, a1 - 0.5), (0.0, 0.02), (1.0, 5.8));

    // On the roof: the parapet, the machines and their ducts.
    b.rbox(
        "roof_floor",
        [-w / 2.0 + 0.2, 0.2, top - 0.6],
        [w / 2.0 - 0.2, d - 0.2, top - 0.15],
        0.0,
    );
    for i in 0..4 {
        let x = -w / 2.0 + 3.0 + i as f64 * 3.0;
        b.rbox(
            "appliance",
            [x, d * 0.4, top - 0.15],
            [x + 2.2, d * 0.4 + 1.1, top + 0.9],
            0.0,
        );
        for j in 0..2 {
            let c = [x + 0.55 + 1.1 * j as f64, d * 0.4 + 0.55];
            b.cone(
                "metal",
                [c[0], c[1], top + 0.9],
                [c[0], c[1], top + 0.95],
                0.45,
                0.45,
                16,
            );
        }
    }
    b.rbox(
        "metal",
        [w / 2.0 - 8.0, d * 0.3, top - 0.15],
        [w / 2.0 - 2.0, d * 0.3 + 0.8, top + 0.4],
        0.0,
    );
    b.rbox(
        "metal",
        [w / 2.0 - 2.8, d * 0.3, top - 0.15],
        [w / 2.0 - 2.0, d * 0.7, top + 0.4],
        0.0,
    );

    // In front: paving, parking lines, trees in their pits.
    let fy = d;
    b.rbox(
        "paving",
        [-w / 2.0, fy, 0.0],
        [w / 2.0, fy + 3.5, 0.03],
        0.0,
    );
    b.rbox(
        "yard",
        [-w / 2.0, fy + 3.5, 0.0],
        [w / 2.0, fy + 14.0, 0.02],
        0.0,
    );
    for k in 0..((w - 10.0) / 2.6) as usize {
        let x = -w / 2.0 + 1.0 + 2.6 * k as f64;
        if x > l0 - w / 2.0 - 3.0 && x < l1 - w / 2.0 + 3.0 {
            continue;
        }
        b.rbox(
            "line",
            [x, fy + 8.5, 0.02],
            [x + 0.1, fy + 13.5, 0.025],
            0.0,
        );
    }
    for k in 0..4 {
        let x = -w / 2.0 + 2.5 + k as f64 * (w - 5.0) / 3.0;
        b.rbox(
            "stone",
            [x - 0.6, fy + 5.0, 0.0],
            [x + 0.6, fy + 6.2, 0.15],
            0.0,
        );
        fruit_tree(b, [x, fy + 5.6], 5.0, seed + k as u64, false);
    }
}
