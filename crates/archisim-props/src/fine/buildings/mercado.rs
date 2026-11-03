//! The market hall (mercado de abastos, regionalist, 1910–1930): a hall
//! of exposed brick with white bands, round-arched windows with iron fans
//! along its sides, a raised central nave with its clerestory of louvres
//! under a roof of tiles and the aisles under roofs of one slope; the
//! front a stepped gable with the tiled name, a clock and the great arched
//! doorway between two side doors, and outside two fruit stalls.

use super::ensanche::{arch_fill, voussoirs_of};
use super::more::*;
use super::*;
use crate::Params;

const WALL: f64 = 0.45;

pub fn market(b: &mut Builder, p: &Params) {
    let w = p.get("width", 24.0).clamp(16.0, 40.0);
    let d = p.get("depth", 36.0).clamp(20.0, 70.0);
    let seed = p.get("variant", 1.0).round().max(0.0) as u64;
    let h = 6.0; // the outer walls
    let cw = (w * 0.44).max(7.0); // the central nave
    let a = (w - cw) / 2.0; // each aisle
    let ap = 0.22; // the aisles' pitch
    let hi = h + a * ap; // where the aisles' roofs meet the clerestory
    let ch = hi + 2.6; // the clerestory's top
    let cp = 0.42;
    let ridge = ch + cw / 2.0 * cp;
    let [front, right, back, left] = Face::sides(w, d);

    // The sides: an arched window every bay, a door in the middle.
    let bay = 3.2;
    let side_holes = |len: f64| {
        let n = ((len - 1.0) / bay).floor().max(1.0) as usize;
        let step = (len - 1.0) / n as f64;
        (0..n)
            .map(|i| {
                let c = 0.5 + step * (i as f64 + 0.5);
                let door = i == n / 2;
                let hw = if door { 1.1 } else { 0.85 };
                (
                    Hole {
                        s: (c - hw, c + hw),
                        z: (if door { 0.1 } else { 1.0 }, 4.0 + hw),
                    },
                    door,
                )
            })
            .collect::<Vec<_>>()
    };
    for f in [right, left, back] {
        let hs = side_holes(f.len);
        let holes: Vec<Hole> = hs.iter().map(|x| x.0).collect();
        wall(b, "brick", &f, (0.0, h), WALL, &holes);
        for (hh, door) in &hs {
            arch_fill(b, &f, "brick", hh.s, 4.0, (-WALL, 0.0));
            voussoirs_of(b, &f, hh.s, 4.0, "trim", 0.16);
            fan_window(b, &f, hh, *door);
        }
        bands(b, &f, h);
    }
    // The front: the great doorway and two side doors, all arched.
    let main = Hole {
        s: (w / 2.0 - 2.0, w / 2.0 + 2.0),
        z: (0.15, 3.4 + 2.0),
    };
    let sides = [
        Hole {
            s: (a * 0.5 - 0.9, a * 0.5 + 0.9),
            z: (0.1, 3.0 + 0.9),
        },
        Hole {
            s: (w - a * 0.5 - 0.9, w - a * 0.5 + 0.9),
            z: (0.1, 3.0 + 0.9),
        },
    ];
    let mut fh = vec![main];
    fh.extend(sides);
    wall(b, "brick", &front, (0.0, h), WALL, &fh);
    arch_fill(b, &front, "brick", main.s, 3.4, (-WALL, 0.0));
    voussoirs_of(b, &front, main.s, 3.4, "trim", 0.25);
    fan_window(b, &front, &main, true);
    for s in &sides {
        arch_fill(b, &front, "brick", s.s, 3.0, (-WALL, 0.0));
        voussoirs_of(b, &front, s.s, 3.0, "trim", 0.16);
        fan_window(b, &front, s, true);
    }
    bands(b, &front, h);
    // Its stepped gable: the aisles' slopes and the nave's, and a parapet
    // stepped over them.
    for f in [&front, &back] {
        f.prism(b, "brick", &[(0.0, h), (a, h), (a, hi)], (-WALL, 0.0));
        f.prism(b, "brick", &[(w - a, h), (w, h), (w - a, hi)], (-WALL, 0.0));
        f.prism(
            b,
            "brick",
            &[(a, h), (w - a, h), (w - a, ch), (w / 2.0, ridge), (a, ch)],
            (-WALL, 0.0),
        );
    }
    let gp = 1.1; // how far the front's parapet rises over the roofs
    front.prism(
        b,
        "brick",
        &[
            (a, ch),
            (w - a, ch),
            (w - a, ch + gp),
            (w / 2.0, ridge + gp + 0.6),
            (a, ch + gp),
        ],
        (-WALL, 0.0),
    );
    for (s0, s1) in [(0.0, a), (w - a, w)] {
        front.bx(b, "brick", (s0, s1), (h, hi + gp * 0.6), (-WALL, 0.0));
        front.bx(
            b,
            "trim",
            (s0 - 0.05, s1 + 0.05),
            (hi + gp * 0.6, hi + gp * 0.6 + 0.12),
            (-WALL - 0.03, 0.08),
        );
    }
    // The coping of the gable, in two slopes.
    for (x0, x1) in [(a - 0.05, w / 2.0), (w - a + 0.05, w / 2.0)] {
        let z0 = ch + gp;
        let z1 = ridge + gp + 0.6;
        plate(
            b,
            "trim",
            &[
                front.at(x0, z0, -WALL - 0.03),
                front.at(x1, z1, -WALL - 0.03),
                front.at(x1, z1, 0.08),
                front.at(x0, z0, 0.08),
            ],
            0.1,
        );
    }
    // Pilasters at the corners and either side of the doorway, with
    // pinnacles.
    for s in [0.0, w - 0.6, a - 0.3, w - a - 0.3] {
        let top = if s == 0.0 || s == w - 0.6 {
            hi + gp * 0.6 + 0.12
        } else {
            ch + gp
        };
        front.bx(b, "trim", (s, s + 0.6), (0.0, top), (-0.01, 0.12));
        let c = front.at(s + 0.3, 0.0, 0.0);
        b.rbox(
            "trim",
            [c[0] - 0.22, c[1] - 0.1, top],
            [c[0] + 0.22, c[1] + 0.22, top + 0.35],
            0.0,
        );
        b.cone(
            "tile_panel",
            [c[0], c[1] + 0.06, top + 0.35],
            [c[0], c[1] + 0.06, top + 1.1],
            0.18,
            0.0,
            8,
        );
    }
    // The tiled name and the clock.
    front.bx(
        b,
        "tile_panel",
        (w / 2.0 - 3.0, w / 2.0 + 3.0),
        (h + 0.2, h + 1.0),
        (-0.01, 0.03),
    );
    front.bx(
        b,
        "trim",
        (w / 2.0 - 3.1, w / 2.0 + 3.1),
        (h + 0.12, h + 0.2),
        (-0.01, 0.06),
    );
    front.bx(
        b,
        "trim",
        (w / 2.0 - 3.1, w / 2.0 + 3.1),
        (h + 1.0, h + 1.08),
        (-0.01, 0.06),
    );
    let cz = (ch + ridge) / 2.0 + 0.2;
    let cc = front.at(w / 2.0, cz, 0.0);
    let o = front.out();
    b.cone(
        "trim",
        [cc[0], cc[1], cz],
        [cc[0] + o[0] * 0.1, cc[1] + o[1] * 0.1, cz],
        0.75,
        0.75,
        24,
    );
    b.cone(
        "clock",
        [cc[0] + o[0] * 0.1, cc[1] + o[1] * 0.1, cz],
        [cc[0] + o[0] * 0.13, cc[1] + o[1] * 0.13, cz],
        0.62,
        0.62,
        24,
    );
    for (len, ang) in [(0.42, 0.5f64), (0.3, 2.2)] {
        let (s, c) = ang.sin_cos();
        let tip = front.at(w / 2.0 + s * len, cz + c * len, 0.15);
        b.cone("iron", front.at(w / 2.0, cz, 0.15), tip, 0.018, 0.012, 4);
    }

    // The clerestory and the roofs.
    let cx = cw / 2.0;
    let cle = [
        Face {
            o: [cx, d],
            u: [0.0, -1.0],
            len: d,
        },
        Face {
            o: [-cx, 0.0],
            u: [0.0, 1.0],
            len: d,
        },
    ];
    for f in &cle {
        let n = ((d - 1.0) / 2.0).floor() as usize;
        let mut hs = Vec::new();
        for i in 0..n {
            let c = 0.5 + (d - 1.0) * (i as f64 + 0.5) / n as f64;
            hs.push(Hole {
                s: (c - 0.6, c + 0.6),
                z: (hi + 0.5, ch - 0.4),
            });
        }
        wall(b, "brick", f, (hi - 0.2, ch), 0.3, &hs);
        for hh in &hs {
            // Louvres of the market's air.
            let mut z = hh.z.0 + 0.08;
            while z < hh.z.1 - 0.05 {
                f.leaning(b, "louvre", hh.s, (z + 0.12, -0.2), (z, -0.05), 0.015);
                z += 0.15;
            }
        }
        f.bx(b, "trim", (0.0, d), (ch - 0.2, ch), (-0.3, 0.08));
        pent_roof(b, f, (0.0, d), (-0.02, hi), (a + 0.35, h - 0.35 * ap));
    }
    b.rbox(
        "interior",
        [-w / 2.0 + WALL, WALL, 0.0],
        [w / 2.0 - WALL, d - WALL, h],
        0.0,
    );
    b.rbox(
        "interior",
        [-cx + 0.3, WALL, h],
        [cx - 0.3, d - WALL, ch],
        0.0,
    );
    super::more::tile_roof_along_y(b, (-cx, cx), (0.0, d), ch, cp, (0.3, 0.0), (false, false));
    for k in 0..3 {
        let y = d * (k as f64 + 1.0) / 4.0;
        // Little ventilators on the ridge.
        b.rbox(
            "trim",
            [-0.35, y - 0.35, ridge - 0.1],
            [0.35, y + 0.35, ridge + 0.6],
            0.0,
        );
        hipped_roof(b, (-0.4, 0.4), (y - 0.4, y + 0.4), ridge + 0.6, 0.6, 0.1);
    }

    // Two fruit stalls outside.
    for (i, x) in [-w * 0.3, w * 0.3].iter().enumerate() {
        stall(b, [*x, d + 2.5], seed + i as u64);
    }
    b.rbox(
        "paving",
        [-w / 2.0 - 1.0, d, 0.0],
        [w / 2.0 + 1.0, d + 5.0, 0.03],
        0.0,
    );
}

/// The white bands of the brick: a plinth, the springing and the
/// cornice.
fn bands(b: &mut Builder, f: &Face, h: f64) {
    f.bx(b, "plinth", (0.0, f.len), (0.0, 0.8), (-0.01, 0.04));
    f.bx(b, "trim", (0.0, f.len), (h - 0.45, h - 0.3), (-0.01, 0.08));
    f.bx(
        b,
        "trim",
        (-0.05, f.len + 0.05),
        (h - 0.15, h),
        (-0.01, 0.2),
    );
}

/// The iron fan in a round arch and a door or a window under it.
fn fan_window(b: &mut Builder, f: &Face, h: &Hole, door: bool) {
    let r = (h.s.1 - h.s.0) / 2.0;
    let spring = h.z.1 - r;
    let c = (h.s.0 + h.s.1) / 2.0;
    let rv = 0.15;
    f.bx(b, "glass", h.s, (spring, h.z.1), (-rv - 0.04, -rv - 0.03));
    for k in 0..=6 {
        let a = std::f64::consts::PI * k as f64 / 6.0;
        f.bar(
            b,
            "iron",
            (c, spring),
            (c + r * a.cos(), spring + r * a.sin()),
            -rv - 0.02,
            0.015,
            4,
        );
    }
    f.bx(
        b,
        "iron",
        h.s,
        (spring - 0.03, spring + 0.03),
        (-rv - 0.04, -rv),
    );
    let low = Hole {
        s: h.s,
        z: (h.z.0, spring - 0.03),
    };
    if door {
        panel_door(b, f, low, 2, 0.0, "wood", "iron", rv);
    } else {
        let win = Win {
            leaves: 2,
            reveal: rv,
            bars: 2,
            ..Default::default()
        };
        window(b, f, low, &win, "frame");
        grille_plane(b, f, "iron", low.s, low.z, -0.04, 0.12, 3);
    }
}

/// A fruit stall: trestles, a board, crates of fruit and greens and its
/// awning on poles.
fn stall(b: &mut Builder, c: [f64; 2], seed: u64) {
    let (w, d) = (2.4, 1.0);
    for (dx, dy) in [(-1.0, -1.0), (1.0, -1.0), (1.0, 1.0), (-1.0, 1.0)] {
        let p = [c[0] + dx * (w / 2.0 - 0.1), c[1] + dy * (d / 2.0 - 0.1)];
        b.cyl("metal", [p[0], p[1], 0.03], [p[0], p[1], 2.3], 0.025);
    }
    b.rbox(
        "wood",
        [c[0] - w / 2.0, c[1] - d / 2.0, 0.8],
        [c[0] + w / 2.0, c[1] + d / 2.0, 0.86],
        0.0,
    );
    let crates = 4;
    for k in 0..crates {
        let x = c[0] - w / 2.0 + 0.1 + (w - 0.2) * k as f64 / crates as f64;
        let x1 = x + (w - 0.2) / crates as f64 - 0.05;
        b.rbox("wood", [x, c[1] - 0.4, 0.86], [x1, c[1] + 0.4, 1.06], 0.0);
        let role = ["fruit", "leaves", "fruit_b", "fruit"][(k + seed as usize) % 4];
        for i in 0..3 {
            for j in 0..2 {
                let px = x + 0.1 + (x1 - x - 0.2) * i as f64 / 2.0;
                let py = c[1] - 0.2 + 0.4 * j as f64;
                b.ellipsoid(role, [px, py, 1.1], [0.07, 0.07, 0.06]);
            }
        }
    }
    let n = 8;
    for k in 0..n {
        let x = c[0] - w / 2.0 + w * k as f64 / n as f64;
        let role = if k % 2 == 0 { "awning" } else { "awning_b" };
        plate(
            b,
            role,
            &[
                [x, c[1] - d / 2.0 - 0.2, 2.2],
                [x + w / n as f64, c[1] - d / 2.0 - 0.2, 2.2],
                [x + w / n as f64, c[1] + d / 2.0 + 0.3, 2.45],
                [x, c[1] + d / 2.0 + 0.3, 2.45],
            ],
            0.02,
        );
    }
}
