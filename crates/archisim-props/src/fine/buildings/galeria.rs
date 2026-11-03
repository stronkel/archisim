//! The Galician town house with glazed galleries (casa de galerías, A
//! Coruña, Ferrol): a narrow house between party walls with its ground
//! floor of granite, granite pilasters up the sides, and the whole front
//! of every floor above closed by a gallery of white wood and small panes
//! standing out of the wall, with its panels underneath and its cornice;
//! a roof of tiles on wide eaves.

use super::*;
use crate::Params;

const WALL: f64 = 0.5;

pub fn galician_galleries(b: &mut Builder, p: &Params) {
    let w = p.get("width", 7.0).clamp(4.5, 14.0);
    let d = p.get("depth", 12.0).clamp(8.0, 20.0);
    let floors = p.get("floors", 5.0).round().clamp(3.0, 7.0) as usize;
    let seed = p.get("variant", 1.0).round().max(0.0) as u64;
    let ground = 3.8;
    let upper = 3.0;
    let top = ground + upper * (floors - 1) as f64;
    let [front, right, back, left] = Face::sides(w, d);
    let pil = 0.55;

    // The ground floor: the door and the shop, granite.
    let door = Hole {
        s: (pil + 0.3, pil + 1.4),
        z: (0.15, 2.9),
    };
    let shop = Hole {
        s: (pil + 2.0, w - pil - 0.3),
        z: (0.0, 3.0),
    };
    wall(b, "stone", &front, (0.0, ground), WALL, &[door, shop]);
    panel_door(b, &front, door, 1, 0.5, "wood", "iron", 0.2);
    shop_front(
        b,
        &front,
        shop,
        if unit(seed, 1) > 0.5 { 0.3 } else { 0.0 },
        true,
        "wood",
        0.2,
    );
    front.bx(
        b,
        "step",
        (door.s.0 - 0.1, door.s.1 + 0.1),
        (0.0, 0.15),
        (-0.2, 0.25),
    );
    front.bx(b, "stone", (0.0, w), (ground - 0.3, ground), (-0.01, 0.2));
    // The floors above: the wall open behind each gallery.
    let mut holes = Vec::new();
    for k in 1..floors {
        let zf = ground + upper * (k - 1) as f64;
        holes.push(Hole {
            s: (pil + 0.1, w - pil - 0.1),
            z: (zf + 0.1, zf + upper - 0.25),
        });
    }
    wall(b, "wall", &front, (ground, top), WALL, &holes);
    for s in [(0.0, pil), (w - pil, w)] {
        front.bx(b, "stone", s, (0.0, top), (-0.01, 0.08));
    }
    for f in [right, left] {
        wall(b, "party", &f, (0.0, top), WALL, &[]);
    }
    let mut bh = Vec::new();
    for k in 0..floors {
        let zf = if k == 0 {
            0.0
        } else {
            ground + upper * (k - 1) as f64
        };
        for x in [w * 0.3, w * 0.7] {
            bh.push(Hole {
                s: (x - 0.45, x + 0.45),
                z: (zf + 1.0, zf + 2.2),
            });
        }
    }
    wall(b, "wall", &back, (0.0, top), WALL, &bh);
    for h in &bh {
        let win = Win {
            leaves: 2,
            reveal: 0.12,
            blind: 0.4 * unit(seed, (h.z.0 * 3.0) as u64 + (h.s.0 * 10.0) as u64),
            ..Default::default()
        };
        window(b, &back, *h, &win, "frame");
        sill(b, &back, *h, "stone", 0.05, 0.12);
    }
    core(b, w, d, WALL, (0.0, top));

    // The galleries.
    let out = 0.6;
    for k in 1..floors {
        let zf = ground + upper * (k - 1) as f64;
        let (s0, s1) = (pil + 0.05, w - pil - 0.05);
        let z0 = zf;
        let z1 = zf + upper - 0.1;
        let pz = z0 + 0.95; // the top of the panels
                            // Its floor and its head.
        front.bx(
            b,
            "gallery",
            (s0 - 0.05, s1 + 0.05),
            (z0 - 0.15, z0 + 0.05),
            (-0.01, out + 0.05),
        );
        front.bx(
            b,
            "gallery",
            (s0 - 0.08, s1 + 0.08),
            (z1 - 0.05, z1 + 0.15),
            (-0.01, out + 0.1),
        );
        // The panels under the glass.
        front.bx(b, "gallery", (s0, s1), (z0 + 0.05, pz), (out - 0.06, out));
        let m = ((s1 - s0) / 0.55).round().max(2.0) as usize;
        let bay = (s1 - s0) / m as f64;
        for i in 0..m {
            let a = s0 + bay * i as f64 + 0.07;
            let e = a + bay - 0.14;
            front.bx(
                b,
                "gallery",
                (a, e),
                (z0 + 0.2, pz - 0.12),
                (out, out + 0.015),
            );
        }
        // The frames: posts at each bay, a rail low and one under the
        // little top panes.
        for i in 0..=m {
            let x = s0 + bay * i as f64;
            front.bx(
                b,
                "gallery",
                (x - 0.03, x + 0.03),
                (pz, z1),
                (out - 0.08, out),
            );
        }
        let tr = z1 - 0.45;
        for z in [pz, tr] {
            front.bx(b, "gallery", (s0, s1), (z, z + 0.05), (out - 0.08, out));
        }
        for i in 0..m {
            let xm = s0 + bay * (i as f64 + 0.5);
            front.bx(
                b,
                "gallery",
                (xm - 0.015, xm + 0.015),
                (tr, z1),
                (out - 0.07, out - 0.01),
            );
            front.bx(
                b,
                "gallery",
                (s0 + bay * i as f64, s0 + bay * (i + 1) as f64),
                ((pz + tr) / 2.0 - 0.015, (pz + tr) / 2.0 + 0.015),
                (out - 0.07, out - 0.01),
            );
        }
        front.bx(b, "glass", (s0, s1), (pz, z1), (out - 0.045, out - 0.035));
        // Its glazed ends.
        for x in [s0, s1] {
            let side = Face {
                o: front.at(x, 0.0, 0.0).map_xy(),
                u: front.out(),
                len: out,
            };
            side.bx(b, "gallery", (0.0, out), (z0 + 0.05, pz), (-0.03, 0.03));
            side.bx(b, "glass", (0.0, out - 0.06), (pz, z1), (-0.006, 0.006));
            side.bx(b, "gallery", (out - 0.08, out), (pz, z1), (-0.035, 0.035));
        }
        // A curtain or a plant behind some panes.
        if unit(seed, 10 + k as u64) > 0.4 {
            front.bx(
                b,
                "curtain",
                (s0 + 0.1, s0 + 0.9),
                (pz + 0.05, z1 - 0.1),
                (0.1, 0.14),
            );
            front.bx(
                b,
                "curtain",
                (s0 + 0.1, s0 + 0.9),
                (z1 - 0.12, z1 - 0.05),
                (0.14, out - 0.04),
            );
        }
        if unit(seed, 20 + k as u64) > 0.5 {
            standing_pot(
                b,
                front.at(s1 - 0.4, z0 + 0.05, out * 0.45),
                0.7,
                seed + k as u64,
            );
        }
    }
    // The cornice on wide eaves and the roof.
    front.bx(
        b,
        "stone",
        (-0.05, w + 0.05),
        (top - 0.1, top + 0.15),
        (-0.01, 0.75),
    );
    tile_roof(
        b,
        (-w / 2.0, w / 2.0),
        (0.0, d),
        top + 0.15,
        0.42,
        (0.8, 0.0),
        (true, true),
    );
    let zc = top + 0.15 + d * 0.3 * 0.42;
    chimney(b, [-w * 0.2, d * 0.3], zc - 0.3, zc + 1.0);
    downpipe(b, &front, 0.25, top);
}
