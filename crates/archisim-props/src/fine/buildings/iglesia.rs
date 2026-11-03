//! The parish church of an Andalusian town (iglesia parroquial): a
//! whitewashed nave under a roof of tiles with its buttresses, the apse
//! behind, the stone doorway with its round arch, pilasters, niche and
//! round window, a side door under a little roof, and the bell tower at
//! the front's corner: the white shaft, the belfry with its arches and
//! bells, and the spire of glazed tiles with its pinnacles and its cross;
//! before it the small square with orange trees.

use super::ensanche::{arch_fill, voussoirs_of};
use super::more::*;
use super::*;
use crate::Params;

const WALL: f64 = 0.8;

pub fn church(b: &mut Builder, p: &Params) {
    let w = p.get("width", 11.0).clamp(8.0, 20.0);
    let d = p.get("depth", 28.0).clamp(16.0, 50.0);
    let seed = p.get("variant", 1.0).round().max(0.0) as u64;
    let tower_h = p.get("tower_height", 26.0).clamp(14.0, 40.0);
    let h = 9.5;
    let pitch = 0.5;
    let [front, right, back, left] = Face::sides(w, d);

    // The front: the doorway and the round window.
    let spring = 3.4;
    let door = Hole {
        s: (w / 2.0 - 1.2, w / 2.0 + 1.2),
        z: (0.3, spring + 1.2),
    };
    wall(b, "wall", &front, (0.0, h), WALL, &[door]);
    arch_fill(b, &front, "wall", door.s, spring, (-WALL, 0.0));
    let rv = 0.35;
    let low = Hole {
        s: door.s,
        z: (door.z.0, spring),
    };
    panel_door(b, &front, low, 2, 0.0, "wood", "iron", rv);
    front.bx(
        b,
        "glass",
        door.s,
        (spring, door.z.1),
        (-rv - 0.06, -rv - 0.05),
    );
    front.bx(
        b,
        "wood",
        door.s,
        (spring - 0.03, spring + 0.08),
        (-rv - 0.1, -rv),
    );
    // The stone doorway.
    voussoirs_of(b, &front, door.s, spring, "stone", 0.35);
    for (s0, s1) in [
        (door.s.0 - 0.75, door.s.0 - 0.05),
        (door.s.1 + 0.05, door.s.1 + 0.75),
    ] {
        front.bx(b, "stone", (s0, s1), (0.0, 0.6), (-0.01, 0.3));
        front.bx(
            b,
            "stone",
            (s0 + 0.08, s1 - 0.08),
            (0.6, 5.6),
            (-0.01, 0.22),
        );
        front.bx(b, "stone", (s0, s1), (5.6, 5.85), (-0.01, 0.3));
    }
    front.bx(
        b,
        "stone",
        (door.s.0 - 0.9, door.s.1 + 0.9),
        (5.85, 6.25),
        (-0.01, 0.35),
    );
    // The niche over it with a figure, and its little pediment.
    let nz = 6.25;
    front.bx(
        b,
        "stone",
        (w / 2.0 - 0.75, w / 2.0 + 0.75),
        (nz, nz + 2.0),
        (-0.01, 0.2),
    );
    front.bx(
        b,
        "niche",
        (w / 2.0 - 0.45, w / 2.0 + 0.45),
        (nz + 0.2, nz + 1.7),
        (0.15, 0.21),
    );
    let fc = front.at(w / 2.0, nz + 0.2, 0.3);
    b.cone(
        "stone",
        [fc[0], fc[1], nz + 0.2],
        [fc[0], fc[1], nz + 1.25],
        0.2,
        0.13,
        12,
    );
    b.ellipsoid("stone", [fc[0], fc[1], nz + 1.38], [0.11, 0.11, 0.13]);
    front.prism(
        b,
        "stone",
        &[
            (w / 2.0 - 0.9, nz + 2.0),
            (w / 2.0 + 0.9, nz + 2.0),
            (w / 2.0, nz + 2.6),
        ],
        (-0.01, 0.25),
    );
    // The round window in the gable.
    let oz = h + 1.3;
    let oc = front.at(w / 2.0, oz, 0.0);
    let o = front.out();
    let ring = |b: &mut Builder, role: &'static str, r: f64, out: f64| {
        b.cone(
            role,
            [oc[0] - o[0] * 0.2, oc[1] - o[1] * 0.2, oz],
            [oc[0] + o[0] * out, oc[1] + o[1] * out, oz],
            r,
            r,
            24,
        );
    };
    ring(b, "stone", 1.05, 0.12);
    ring(b, "glass", 0.8, 0.13);
    for k in 0..4 {
        let a = std::f64::consts::PI * k as f64 / 4.0;
        let (s, c) = a.sin_cos();
        b.cone(
            "iron",
            front.at(w / 2.0 - c * 0.8, oz - s * 0.8, 0.14),
            front.at(w / 2.0 + c * 0.8, oz + s * 0.8, 0.14),
            0.02,
            0.02,
            4,
        );
    }
    front.bx(
        b,
        "plinth",
        (0.0, door.s.0 - 0.8),
        (0.0, 0.9),
        (-0.01, 0.04),
    );
    front.bx(b, "plinth", (door.s.1 + 0.8, w), (0.0, 0.9), (-0.01, 0.04));
    front.bx(
        b,
        "step",
        (door.s.0 - 1.2, door.s.1 + 1.2),
        (0.0, 0.15),
        (-0.01, 1.2),
    );
    front.bx(
        b,
        "step",
        (door.s.0 - 1.2, door.s.1 + 1.2),
        (0.15, 0.3),
        (-0.01, 0.8),
    );

    // The sides: high round-headed windows between buttresses; a door on
    // the right with its little roof.
    let n = ((d - 2.0) / 5.0).floor().max(2.0) as usize;
    let step = (d - 2.0) / n as f64;
    for (k, f) in [right, left].iter().enumerate() {
        let mut hs = Vec::new();
        for i in 0..n {
            let c = 1.0 + step * (i as f64 + 0.5);
            hs.push(Hole {
                s: (c - 0.5, c + 0.5),
                z: (5.5, 7.5 + 0.5),
            });
        }
        let side_door = Hole {
            s: (d * 0.35 - 0.8, d * 0.35 + 0.8),
            z: (0.2, 2.6 + 0.8),
        };
        if k == 0 {
            hs.push(side_door);
        }
        wall(b, "wall", f, (0.0, h), WALL, &hs);
        for hh in &hs {
            let r = (hh.s.1 - hh.s.0) / 2.0;
            let sp = hh.z.1 - r;
            arch_fill(b, f, "wall", hh.s, sp, (-WALL, 0.0));
            if hh.z.0 < 1.0 {
                panel_door(
                    b,
                    f,
                    Hole {
                        s: hh.s,
                        z: (hh.z.0, sp),
                    },
                    2,
                    0.0,
                    "wood",
                    "iron",
                    0.3,
                );
                surround(
                    b,
                    f,
                    Hole {
                        s: hh.s,
                        z: (hh.z.0, hh.z.1),
                    },
                    "trim",
                    0.2,
                    0.04,
                );
                f.leaning(
                    b,
                    "roof",
                    (hh.s.0 - 0.6, hh.s.1 + 0.6),
                    (hh.z.1 + 0.9, -0.01),
                    (hh.z.1 + 0.35, 1.0),
                    0.06,
                );
                let m = ((hh.s.1 - hh.s.0 + 1.2) / 0.21) as usize;
                for t in 0..m {
                    let x = hh.s.0 - 0.6 + (hh.s.1 - hh.s.0 + 1.2) * (t as f64 + 0.5) / m as f64;
                    b.cone(
                        "tiles",
                        f.at(x, hh.z.1 + 0.98, -0.01),
                        f.at(x, hh.z.1 + 0.42, 1.05),
                        0.055,
                        0.05,
                        6,
                    );
                }
            } else {
                f.bx(b, "glass", hh.s, hh.z, (-0.45, -0.44));
                grille_plane(b, f, "iron", hh.s, (hh.z.0, sp), -0.3, 0.12, 3);
                f.bx(
                    b,
                    "trim",
                    (hh.s.0 - 0.1, hh.s.1 + 0.1),
                    (hh.z.0 - 0.1, hh.z.0),
                    (-0.3, 0.05),
                );
            }
        }
        // Buttresses, leaning in as they rise.
        for i in 0..=n {
            let c = 1.0 + step * i as f64;
            let side = Face {
                o: f.at(c, 0.0, 0.0).map_xy(),
                u: f.out(),
                len: 1.2,
            };
            side.prism(
                b,
                "wall",
                &[
                    (-0.02, 0.0),
                    (1.2, 0.0),
                    (1.2, 1.2),
                    (0.3, h - 0.6),
                    (-0.02, h - 0.6),
                ],
                (-0.35, 0.35),
            );
        }
        f.bx(b, "plinth", (0.0, f.len), (0.0, 0.9), (-0.01, 0.04));
        f.bx(
            b,
            "trim",
            (-0.05, f.len + 0.05),
            (h - 0.35, h),
            (-0.01, 0.18),
        );
    }
    wall(b, "wall", &back, (0.0, h), WALL, &[]);
    core(b, w, d, WALL, (0.0, h));
    tile_roof_along_y(
        b,
        (-w / 2.0, w / 2.0),
        (0.0, d),
        h,
        pitch,
        (0.4, 0.15),
        (true, true),
    );
    // The cross on the front's gable.
    let gz = h + w / 2.0 * pitch + 0.1;
    let gc = front.at(w / 2.0, gz, -0.4);
    b.rbox(
        "stone",
        [gc[0] - 0.25, gc[1] - 0.25, gz],
        [gc[0] + 0.25, gc[1] + 0.25, gz + 0.4],
        0.0,
    );
    b.cyl(
        "iron",
        [gc[0], gc[1], gz + 0.4],
        [gc[0], gc[1], gz + 1.8],
        0.04,
    );
    b.cyl(
        "iron",
        [gc[0] - 0.4, gc[1], gz + 1.4],
        [gc[0] + 0.4, gc[1], gz + 1.4],
        0.035,
    );

    // The apse behind: lower, square, with a hipped roof.
    let aw = w * 0.7;
    let ad = 4.0;
    let ah = h - 1.2;
    let af = Face::sides(aw, ad).map(|f| Face {
        o: [f.o[0], f.o[1] - ad],
        ..f
    });
    for (k, f) in af.iter().enumerate() {
        if k == 0 {
            continue;
        }
        let hs = if k == 2 {
            vec![Hole {
                s: (aw / 2.0 - 0.4, aw / 2.0 + 0.4),
                z: (5.0, 6.8),
            }]
        } else {
            Vec::new()
        };
        wall(b, "wall", f, (0.0, ah), WALL, &hs);
        f.bx(b, "plinth", (0.0, f.len), (0.0, 0.9), (-0.01, 0.04));
        f.bx(
            b,
            "trim",
            (-0.05, f.len + 0.05),
            (ah - 0.3, ah),
            (-0.01, 0.15),
        );
        for hh in &hs {
            f.bx(b, "glass", hh.s, hh.z, (-0.45, -0.44));
        }
    }
    b.rbox(
        "interior",
        [-aw / 2.0 + WALL, -ad + WALL, 0.0],
        [aw / 2.0 - WALL, 0.0, ah],
        0.0,
    );
    hipped_roof(b, (-aw / 2.0, aw / 2.0), (-ad, 0.6), ah, 0.5, 0.35);

    // The bell tower at the front's left corner.
    let tw = 5.0;
    let tx = -w / 2.0 - tw / 2.0 + 0.3;
    let ty = d - tw / 2.0;
    let shaft = tower_h - 6.0;
    let tf = Face::sides(tw, tw).map(|f| Face {
        o: [f.o[0] + tx, f.o[1] + ty - tw / 2.0],
        ..f
    });
    for (k, f) in tf.iter().enumerate() {
        let small = if k % 2 == 0 {
            vec![Hole {
                s: (tw / 2.0 - 0.3, tw / 2.0 + 0.3),
                z: (shaft * 0.55, shaft * 0.55 + 1.0),
            }]
        } else {
            Vec::new()
        };
        wall(b, "wall", f, (0.0, shaft), 0.9, &small);
        for hh in &small {
            f.bx(b, "glass", hh.s, hh.z, (-0.5, -0.49));
            grille_plane(b, f, "iron", hh.s, hh.z, -0.2, 0.1, 2);
        }
        f.bx(b, "plinth", (0.0, tw), (0.0, 1.2), (-0.01, 0.05));
        // Bands of albero up the shaft and its cornice.
        f.bx(
            b,
            "trim",
            (0.0, tw),
            (shaft * 0.45, shaft * 0.45 + 0.25),
            (-0.01, 0.06),
        );
        f.bx(
            b,
            "trim",
            (-0.15, tw + 0.15),
            (shaft - 0.45, shaft),
            (-0.01, 0.25),
        );
        // The belfry: an arch on each side.
        let bz = shaft;
        let bh = Hole {
            s: (tw / 2.0 - 1.0, tw / 2.0 + 1.0),
            z: (bz + 0.9, bz + 3.6 + 1.0),
        };
        wall(b, "wall", f, (bz, bz + 5.4), 0.7, &[bh]);
        arch_fill(b, f, "wall", bh.s, bz + 3.6, (-0.7, 0.0));
        voussoirs_of(b, f, bh.s, bz + 3.6, "trim", 0.22);
        f.bx(
            b,
            "trim",
            (bh.s.0 - 0.1, bh.s.1 + 0.1),
            (bz + 0.75, bz + 0.9),
            (-0.7, 0.1),
        );
        railing(
            b,
            f,
            "iron",
            (bh.s.0 + 0.05, bh.s.1 - 0.05),
            bz + 0.9,
            -0.35,
            0.9,
            0.12,
        );
        for s in [0.0, tw - 0.45] {
            f.bx(b, "trim", (s, s + 0.45), (bz, bz + 5.4), (-0.01, 0.1));
        }
        f.bx(
            b,
            "trim",
            (-0.2, tw + 0.2),
            (bz + 5.4, bz + 5.8),
            (-0.01, 0.3),
        );
    }
    let top = shaft + 5.8;
    b.rbox(
        "interior",
        [tx - tw / 2.0 + 0.9, ty - tw / 2.0 + 0.9, 0.0],
        [tx + tw / 2.0 - 0.9, ty + tw / 2.0 - 0.9, shaft],
        0.0,
    );
    b.rbox(
        "trim",
        [tx - 1.8, ty - 1.8, shaft],
        [tx + 1.8, ty + 1.8, shaft + 0.9],
        0.0,
    );
    // The bells, hung in the arches.
    for (k, (dx, dy)) in [
        (0.0, tw / 2.0 - 0.6),
        (tw / 2.0 - 0.6, 0.0),
        (0.0, -tw / 2.0 + 0.6),
        (-tw / 2.0 + 0.6, 0.0),
    ]
    .iter()
    .enumerate()
    {
        let c = [tx + dx, ty + dy];
        let beam = shaft + 3.4;
        let along = if *dy == 0.0 { [0.0, 1.15] } else { [1.15, 0.0] };
        b.cyl(
            "wood",
            [c[0] - along[0], c[1] - along[1], beam],
            [c[0] + along[0], c[1] + along[1], beam],
            0.08,
        );
        bell(b, c, beam - 0.06, if k == 0 { 0.9 } else { 0.6 });
    }
    // The spire of glazed tiles, its pinnacles and its weathervane cross.
    b.rbox(
        "trim",
        [tx - 2.2, ty - 2.2, top],
        [tx + 2.2, ty + 2.2, top + 0.5],
        0.0,
    );
    let start = b.mark();
    b.cone(
        "glaze",
        [tx, ty, top + 0.5],
        [tx, ty, top + 5.0],
        2.0 * std::f64::consts::SQRT_2,
        0.0,
        4,
    );
    turn_from(b, start, [tx, ty], std::f64::consts::FRAC_PI_4);
    for (dx, dy) in [(-1.0, -1.0), (1.0, -1.0), (1.0, 1.0), (-1.0, 1.0)] {
        let c = [tx + dx * 2.0, ty + dy * 2.0];
        b.rbox(
            "trim",
            [c[0] - 0.2, c[1] - 0.2, top + 0.5],
            [c[0] + 0.2, c[1] + 0.2, top + 0.9],
            0.0,
        );
        b.cone(
            "glaze",
            [c[0], c[1], top + 0.9],
            [c[0], c[1], top + 1.9],
            0.18,
            0.0,
            8,
        );
    }
    b.ellipsoid("brass", [tx, ty, top + 5.1], [0.15, 0.15, 0.15]);
    b.cyl("iron", [tx, ty, top + 5.0], [tx, ty, top + 6.6], 0.035);
    b.cyl(
        "iron",
        [tx - 0.45, ty, top + 6.1],
        [tx + 0.45, ty, top + 6.1],
        0.03,
    );
    b.cone(
        "iron",
        [tx, ty, top + 5.6],
        [tx + 0.6, ty, top + 5.6],
        0.02,
        0.02,
        4,
    );

    // The square before it: paving, benches, orange trees.
    let sq = 9.0;
    b.rbox(
        "paving",
        [-w / 2.0 - 5.5, d, 0.0],
        [w / 2.0 + 3.0, d + sq, 0.03],
        0.0,
    );
    for (i, x) in [-w / 2.0 - 2.5, w / 2.0].iter().enumerate() {
        let c = [*x, d + sq * 0.6];
        b.rbox(
            "stone",
            [c[0] - 0.7, c[1] - 0.7, 0.0],
            [c[0] + 0.7, c[1] + 0.7, 0.3],
            0.0,
        );
        fruit_tree(b, c, 3.6, seed + i as u64, true);
        b.rbox(
            "stone",
            [c[0] - 1.0, c[1] + 1.4, 0.0],
            [c[0] + 1.0, c[1] + 1.85, 0.45],
            0.0,
        );
    }
}
