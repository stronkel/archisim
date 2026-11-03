//! The Canarian house (casa canaria): two floors of coloured render with
//! corners and surrounds of dark volcanic stone, sash windows of wood with
//! their shutters, the door in its stone frame, and in the middle of the
//! upper floor the wooden balcony (balcón canario) with its turned
//! balusters, posts and little roof of tiles; a hipped roof of tiles, and
//! a palm and a dragon tree's cousin in the garden.

use super::more::*;
use super::*;
use crate::Params;

const WALL: f64 = 0.5;
const REVEAL: f64 = 0.15;

pub fn canarian_house(b: &mut Builder, p: &Params) {
    let w = p.get("width", 11.0).clamp(8.0, 18.0);
    let d = p.get("depth", 10.0).clamp(7.0, 16.0);
    let seed = p.get("variant", 1.0).round().max(0.0) as u64;
    let ground = 3.6;
    let top = ground + 3.2;
    let [front, right, back, left] = Face::sides(w, d);

    let door = Hole {
        s: (w / 2.0 - 0.7, w / 2.0 + 0.7),
        z: (0.2, 2.8),
    };
    let mut holes = vec![door];
    let mut wins = Vec::new();
    for x in [w * 0.18, w * 0.82] {
        for z in [(0.9, 2.6), (ground + 0.6, ground + 2.3)] {
            let h = Hole {
                s: (x - 0.5, x + 0.5),
                z,
            };
            holes.push(h);
            wins.push(h);
        }
    }
    let bal_door = Hole {
        s: (w / 2.0 - 0.6, w / 2.0 + 0.6),
        z: (ground + 0.05, ground + 2.4),
    };
    holes.push(bal_door);
    wall(b, "wall", &front, (0.0, top), WALL, &holes);
    let mut others = Vec::new();
    for (k, f) in [right, back, left].iter().enumerate() {
        let n = ((f.len - 2.0) / 3.0).floor().max(1.0) as usize;
        let mut hs = Vec::new();
        for i in 0..n {
            let c = 1.0 + (f.len - 2.0) * (i as f64 + 0.5) / n as f64;
            hs.push(Hole {
                s: (c - 0.45, c + 0.45),
                z: (1.0, 2.5),
            });
            hs.push(Hole {
                s: (c - 0.45, c + 0.45),
                z: (ground + 0.6, ground + 2.2),
            });
        }
        wall(b, "wall", f, (0.0, top), WALL, &hs);
        others.push((k, *f, hs));
    }
    core(b, w, d, WALL, (0.0, top));
    // Dark stone: the corners, the plinth, the band at the floor, the
    // cornice; every hole framed in it.
    for f in [&front, &right, &back, &left] {
        f.bx(b, "stone", (0.0, 0.45), (0.0, top), (-0.01, 0.05));
        f.bx(b, "stone", (f.len - 0.45, f.len), (0.0, top), (-0.01, 0.05));
        f.bx(b, "stone", (0.0, f.len), (0.0, 0.55), (-0.01, 0.04));
        f.bx(
            b,
            "stone",
            (0.0, f.len),
            (ground - 0.15, ground + 0.05),
            (-0.01, 0.06),
        );
        f.bx(
            b,
            "stone",
            (-0.05, f.len + 0.05),
            (top - 0.3, top),
            (-0.01, 0.15),
        );
    }
    let frame_hole = |b: &mut Builder, f: &Face, h: &Hole| {
        surround(b, f, *h, "stone", 0.18, 0.05);
        f.bx(
            b,
            "stone",
            (h.s.0 - 0.18, h.s.1 + 0.18),
            (h.z.0 - 0.12, h.z.0),
            (-REVEAL, 0.08),
        );
        f.bx(
            b,
            "stone",
            (h.s.0 - 0.25, h.s.1 + 0.25),
            (h.z.1 + 0.18, h.z.1 + 0.3),
            (-0.01, 0.1),
        );
    };
    // The windows: sashes of wood with small panes and shutters inside
    // the reveal, half open.
    for (j, h) in wins.iter().enumerate() {
        sash(b, &front, h, unit(seed, j as u64));
        frame_hole(b, &front, h);
    }
    for (_, f, hs) in &others {
        for (j, h) in hs.iter().enumerate() {
            sash(b, f, h, unit(seed, 50 + j as u64));
            frame_hole(b, f, h);
        }
    }
    panel_door(b, &front, door, 2, 0.0, "wood", "iron", REVEAL);
    frame_hole(b, &front, &door);
    front.bx(
        b,
        "stone",
        (door.s.0 - 0.3, door.s.1 + 0.3),
        (0.0, 0.2),
        (-REVEAL, 0.5),
    );
    // The balcony: its door, the wooden balcony with posts and a roof.
    let win = Win {
        leaves: 2,
        reveal: REVEAL,
        bars: 3,
        ..Default::default()
    };
    window(b, &front, bal_door, &win, "wood");
    frame_hole(b, &front, &bal_door);
    let (s0, s1) = (w / 2.0 - 2.2, w / 2.0 + 2.2);
    let out = 0.95;
    let zf = ground + 0.05;
    wooden_balcony(b, &front, (s0, s1), zf, out, "wood", Some(top - 0.3));
    // Carved brackets under it.
    for i in 0..5 {
        let x = s0 + 0.15 + (s1 - s0 - 0.3) * i as f64 / 4.0;
        let side = Face {
            o: front.at(x, 0.0, 0.0).map_xy(),
            u: front.out(),
            len: out,
        };
        side.prism(
            b,
            "wood",
            &[
                (-0.01, zf - 0.26),
                (out * 0.8, zf - 0.26),
                (out * 0.3, zf - 0.6),
                (-0.01, zf - 0.8),
            ],
            (-0.05, 0.05),
        );
    }
    front.bx(
        b,
        "wood",
        (s0 - 0.1, s1 + 0.1),
        (top - 0.32, top - 0.2),
        (-0.01, out + 0.1),
    );
    front.leaning(
        b,
        "roof",
        (s0 - 0.3, s1 + 0.3),
        (top + 0.15, -0.01),
        (top - 0.2, out + 0.3),
        0.05,
    );
    let m = ((s1 - s0 + 0.6) / 0.21) as usize;
    for k in 0..m {
        let x = s0 - 0.3 + (s1 - s0 + 0.6) * (k as f64 + 0.5) / m as f64;
        b.cone(
            "tiles",
            front.at(x, top + 0.22, -0.01),
            front.at(x, top - 0.13, out + 0.35),
            0.055,
            0.05,
            6,
        );
    }
    for k in 0..3 {
        standing_pot(
            b,
            front.at(s0 + 0.6 + 1.5 * k as f64, zf, out * 0.5),
            0.7,
            seed + k,
        );
    }
    // The hipped roof.
    hipped_roof(b, (-w / 2.0, w / 2.0), (0.0, d), top, 0.5, 0.45);
    // The garden: a palm and a dragon tree.
    b.rbox(
        "yard",
        [-w / 2.0 - 3.0, d, 0.0],
        [w / 2.0 + 3.0, d + 5.0, 0.02],
        0.0,
    );
    palm(b, [-w / 2.0 - 1.5, d + 2.5], 7.0, seed);
    dragon_tree(b, [w / 2.0 + 1.5, d + 2.5]);
}

/// A sash window of wood with small panes, its inner shutters half open.
fn sash(b: &mut Builder, f: &Face, h: &Hole, open: f64) {
    let win = Win {
        leaves: 1,
        reveal: REVEAL,
        bars: 2,
        ..Default::default()
    };
    window(b, f, *h, &win, "wood");
    let mid = (h.z.0 + h.z.1) / 2.0;
    f.bx(
        b,
        "wood",
        h.s,
        (mid - 0.03, mid + 0.03),
        (-REVEAL - 0.03, -REVEAL + 0.02),
    );
    // A shutter folded against each jamb, deeper in.
    let lw = (h.s.1 - h.s.0) * (0.25 + 0.2 * open);
    f.bx(
        b,
        "shutter_wood",
        (h.s.0 + 0.02, h.s.0 + lw),
        (h.z.0 + 0.05, h.z.1 - 0.05),
        (-0.42, -0.4),
    );
    f.bx(
        b,
        "shutter_wood",
        (h.s.1 - lw, h.s.1 - 0.02),
        (h.z.0 + 0.05, h.z.1 - 0.05),
        (-0.42, -0.4),
    );
    for x in [h.s.0, h.s.1 - 0.02] {
        f.bx(
            b,
            "shutter_wood",
            (x, x + 0.02),
            (h.z.0 + 0.05, h.z.1 - 0.05),
            (-0.5, -0.4),
        );
    }
}

/// A dragon tree (drago): a stout trunk that forks and forks again, with
/// tufts of stiff leaves at the tips.
fn dragon_tree(b: &mut Builder, c: [f64; 2]) {
    let top = 2.2;
    b.cone(
        "trunk",
        [c[0], c[1], 0.0],
        [c[0], c[1], top],
        0.45,
        0.35,
        12,
    );
    for k in 0..6 {
        let a = std::f64::consts::TAU * k as f64 / 6.0;
        let (s, co) = a.sin_cos();
        let mid = [c[0] + co * 0.9, c[1] + s * 0.9, top + 1.0];
        b.cone("trunk", [c[0], c[1], top - 0.1], mid, 0.25, 0.16, 8);
        for j in [-0.4f64, 0.4] {
            let a2 = a + j;
            let tip = [
                mid[0] + a2.cos() * 0.6,
                mid[1] + a2.sin() * 0.6,
                mid[2] + 0.6,
            ];
            b.cone("trunk", mid, tip, 0.14, 0.1, 8);
            b.blob(
                "leaves",
                [tip[0], tip[1], tip[2] + 0.25],
                [0.55, 0.0, 0.0],
                [0.0, 0.55, 0.0],
                [0.0, 0.0, 0.3],
            );
        }
    }
}
