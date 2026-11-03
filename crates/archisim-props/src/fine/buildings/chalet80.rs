//! The chalet of the 1980s (chalet de los 80): a house of two floors
//! standing alone in its plot, render with corners and surrounds of
//! exposed brick, a hipped roof of tiles with a big brick chimney, the
//! porch of three brick arches with the terrace and its concrete
//! balusters over it, green louvred shutters (mallorquinas) open against
//! the wall, iron grilles downstairs, the garage to one side, and round it
//! the garden: a low wall with brick piers and railings, the gate, lawn, a
//! palm and the swimming pool.

use super::ensanche::{arch_fill, voussoirs_of};
use super::more::*;
use super::*;
use crate::Params;

const WALL: f64 = 0.3;
const REVEAL: f64 = 0.1;

pub fn chalet_80s(b: &mut Builder, p: &Params) {
    let w = p.get("width", 12.0).clamp(9.0, 18.0);
    let d = p.get("depth", 10.0).clamp(8.0, 14.0);
    let seed = p.get("variant", 1.0).round().max(0.0) as u64;
    let pool = p.flag("pool", true);
    let ground = 3.0;
    let top = ground + 2.8;
    let [front, right, back, left] = Face::sides(w, d);

    // The porch in the middle of the front, its terrace over it.
    let pw = (w * 0.55).clamp(5.5, 8.0);
    let pd = 2.4;
    let px0 = w / 2.0 - pw / 2.0;

    // The holes: the front door in the porch, windows each side; upstairs
    // the door to the terrace and windows.
    let mut holes = Vec::new();
    let door = Hole {
        s: (w / 2.0 - 0.55, w / 2.0 + 0.55),
        z: (0.15, 2.3),
    };
    holes.push((door, 0));
    for x in [px0 + 1.1, px0 + pw - 1.1] {
        holes.push((
            Hole {
                s: (x - 0.6, x + 0.6),
                z: (0.9, 2.2),
            },
            1,
        ));
    }
    for x in [px0 * 0.5, w - px0 * 0.5] {
        if px0 > 1.6 {
            holes.push((
                Hole {
                    s: (x - 0.55, x + 0.55),
                    z: (0.9, 2.2),
                },
                1,
            ));
            holes.push((
                Hole {
                    s: (x - 0.55, x + 0.55),
                    z: (ground + 0.9, ground + 2.1),
                },
                2,
            ));
        }
    }
    holes.push((
        Hole {
            s: (w / 2.0 - 0.8, w / 2.0 + 0.8),
            z: (ground + 0.05, ground + 2.2),
        },
        3,
    ));
    for x in [px0 + 1.0, px0 + pw - 1.0] {
        holes.push((
            Hole {
                s: (x - 0.5, x + 0.5),
                z: (ground + 0.9, ground + 2.1),
            },
            2,
        ));
    }
    let fh: Vec<Hole> = holes.iter().map(|h| h.0).collect();
    wall(b, "wall", &front, (0.0, top), WALL, &fh);
    let mut faces = vec![(front, holes.clone())];
    for (fi, f) in [right, back, left].iter().enumerate() {
        let m = ((f.len - 1.0) / 2.8).floor().max(1.0) as usize;
        let mut hs = Vec::new();
        for i in 0..m {
            let c = 0.5 + (f.len - 1.0) * (i as f64 + 0.5) / m as f64;
            if fi == 1 && i == m / 2 {
                hs.push((
                    Hole {
                        s: (c - 0.9, c + 0.9),
                        z: (0.05, 2.2),
                    },
                    4,
                ));
            } else {
                hs.push((
                    Hole {
                        s: (c - 0.5, c + 0.5),
                        z: (1.0, 2.2),
                    },
                    1,
                ));
            }
            hs.push((
                Hole {
                    s: (c - 0.5, c + 0.5),
                    z: (ground + 0.9, ground + 2.1),
                },
                2,
            ));
        }
        let only: Vec<Hole> = hs.iter().map(|h| h.0).collect();
        wall(b, "wall", f, (0.0, top), WALL, &only);
        faces.push((*f, hs));
    }
    core(b, w, d, WALL, (0.0, top));

    // Brick: the corners, the plinth, the band at the floor.
    for (f, _) in &faces {
        f.bx(b, "brick", (0.0, f.len), (0.0, 0.55), (-0.01, 0.03));
        f.bx(
            b,
            "brick",
            (0.0, f.len),
            (ground - 0.12, ground + 0.08),
            (-0.01, 0.03),
        );
        let mut z = 0.55;
        let mut k = 0;
        while z < top - 0.1 {
            let wide = if k % 2 == 0 { 0.45 } else { 0.25 };
            f.bx(
                b,
                "brick",
                (0.0, wide),
                (z, (z + 0.3).min(top)),
                (-0.01, 0.025),
            );
            f.bx(
                b,
                "brick",
                (f.len - wide, f.len),
                (z, (z + 0.3).min(top)),
                (-0.01, 0.025),
            );
            z += 0.3;
            k += 1;
        }
    }
    // The carpentry: shutters, grilles, the door.
    for (fi, (f, hs)) in faces.iter().enumerate() {
        for (j, (h, kind)) in hs.iter().enumerate() {
            let key = seed * 31 + (fi * 40 + j) as u64;
            match kind {
                0 => {
                    panel_door(b, f, *h, 1, 0.0, "wood", "iron", REVEAL);
                    f.bx(
                        b,
                        "brick",
                        (h.s.0 - 0.2, h.s.1 + 0.2),
                        (h.z.1, h.z.1 + 0.2),
                        (-0.01, 0.03),
                    );
                }
                4 => {
                    let win = Win {
                        leaves: 2,
                        reveal: REVEAL,
                        sliding: true,
                        blind: 0.4,
                        ..Default::default()
                    };
                    window(b, f, *h, &win, "frame");
                }
                _ => {
                    let win = Win {
                        leaves: 2,
                        reveal: REVEAL,
                        curtain: unit(key, 1) > 0.4,
                        ..Default::default()
                    };
                    window(b, f, *h, &win, "frame");
                    // A brick surround with its lintel.
                    f.bx(
                        b,
                        "brick",
                        (h.s.0 - 0.12, h.s.1 + 0.12),
                        (h.z.1, h.z.1 + 0.2),
                        (-0.01, 0.03),
                    );
                    if *kind != 3 {
                        f.bx(
                            b,
                            "brick",
                            (h.s.0 - 0.08, h.s.1 + 0.08),
                            (h.z.0 - 0.06, h.z.0),
                            (-REVEAL, 0.06),
                        );
                    }
                    if *kind == 1 {
                        grille_plane(b, f, "iron", h.s, h.z, 0.04, 0.12, 3);
                    }
                    if *kind != 3 {
                        let open = unit(key, 2);
                        shutters(b, f, h, open);
                    }
                }
            }
        }
    }

    // The porch: the brick arches on the front and the sides, its floor,
    // the terrace on it with balusters.
    let pf = Face {
        o: [-pw / 2.0, d + pd],
        u: [1.0, 0.0],
        len: pw,
    };
    let ps = [
        Face {
            o: [pw / 2.0, d + pd],
            u: [0.0, -1.0],
            len: pd,
        },
        Face {
            o: [-pw / 2.0, d],
            u: [0.0, 1.0],
            len: pd,
        },
    ];
    let spring = 2.0;
    let pier = 0.45;
    let span = (pw - 4.0 * pier) / 3.0;
    let r = span / 2.0;
    let mut arches = Vec::new();
    for i in 0..3 {
        let a = pier + (span + pier) * i as f64;
        arches.push(Hole {
            s: (a, a + span),
            z: (0.0, spring + r),
        });
    }
    wall(b, "brick", &pf, (0.0, ground), pier, &arches);
    for h in &arches {
        arch_fill(b, &pf, "brick", h.s, spring, (-pier, 0.0));
        voussoirs_of(b, &pf, h.s, spring, "trim", 0.12);
    }
    for f in &ps {
        let sp = pd - 2.0 * pier;
        let h = Hole {
            s: (pier, pd - pier),
            z: (0.0, spring + sp / 2.0),
        };
        wall(b, "brick", f, (0.0, ground), 0.35, &[h]);
        arch_fill(b, f, "brick", h.s, spring, (-0.35, 0.0));
    }
    b.rbox("step", [-pw / 2.0, d, 0.0], [pw / 2.0, d + pd, 0.15], 0.0);
    b.rbox(
        "trim",
        [-pw / 2.0 - 0.08, d - 0.02, ground - 0.02],
        [pw / 2.0 + 0.08, d + pd + 0.08, ground + 0.18],
        0.0,
    );
    // The balusters round the terrace.
    let bz = ground + 0.18;
    for (f, len) in [(pf, pw), (ps[0], pd), (ps[1], pd)] {
        let n = ((len - 0.4) / 0.2).floor() as usize;
        for k in 0..n {
            let s = 0.3 + (len - 0.6) * k as f64 / (n.max(2) - 1) as f64;
            let c = f.at(s, 0.0, -0.15);
            b.lathe(
                "trim",
                [c[0], c[1], bz],
                &[
                    (0.07, 0.0),
                    (0.05, 0.1),
                    (0.09, 0.3),
                    (0.035, 0.6),
                    (0.06, 0.68),
                    (0.0, 0.7),
                ],
                1.0,
                1.0,
            );
        }
        f.bx(b, "trim", (0.0, len), (bz + 0.68, bz + 0.8), (-0.27, -0.03));
        for s in [0.0, len - 0.25] {
            f.bx(b, "trim", (s, s + 0.25), (bz, bz + 0.85), (-0.28, -0.02));
        }
    }
    standing_pot(b, [pw / 2.0 - 0.7, d + 0.8, ground + 0.18], 0.9, seed + 3);
    standing_pot(b, [-pw / 2.0 + 0.5, d + pd - 0.5, 0.15], 1.1, seed + 4);

    // The hipped roof and the brick chimney.
    hipped_roof(b, (-w / 2.0, w / 2.0), (0.0, d), top, 0.45, 0.55);
    let cx = -w / 2.0 + w * 0.28;
    let cy = d * 0.35;
    let z_roof = top + (cy).min(cx + w / 2.0).min(d / 2.0) * 0.45;
    b.rbox(
        "brick",
        [cx - 0.45, cy - 0.35, z_roof - 0.5],
        [cx + 0.45, cy + 0.35, z_roof + 1.5],
        0.0,
    );
    b.rbox(
        "trim",
        [cx - 0.52, cy - 0.42, z_roof + 1.5],
        [cx + 0.52, cy + 0.42, z_roof + 1.58],
        0.0,
    );
    for (dx, dy) in [(-0.38, -0.28), (0.38, -0.28), (0.38, 0.28), (-0.38, 0.28)] {
        b.rbox(
            "brick",
            [cx + dx - 0.07, cy + dy - 0.07, z_roof + 1.58],
            [cx + dx + 0.07, cy + dy + 0.07, z_roof + 1.95],
            0.0,
        );
    }
    hipped_roof(
        b,
        (cx - 0.5, cx + 0.5),
        (cy - 0.4, cy + 0.4),
        z_roof + 1.95,
        0.5,
        0.08,
    );

    // The garage on the right: one floor, its tilting door, a roof of
    // one slope.
    let gw = 3.6;
    let gx0 = w / 2.0;
    let gy0 = d - 6.0;
    let gf = Face {
        o: [gx0, d],
        u: [1.0, 0.0],
        len: gw,
    };
    let gd = Hole {
        s: (0.4, gw - 0.4),
        z: (0.0, 2.3),
    };
    wall(b, "wall", &gf, (0.0, 2.8), 0.25, &[gd]);
    let gr = Face {
        o: [gx0 + gw, d],
        u: [0.0, -1.0],
        len: 6.0,
    };
    wall(b, "wall", &gr, (0.0, 2.8), 0.25, &[]);
    let gbk = Face {
        o: [gx0 + gw, gy0],
        u: [-1.0, 0.0],
        len: gw,
    };
    wall(b, "wall", &gbk, (0.0, 2.8), 0.25, &[]);
    b.rbox(
        "interior",
        [gx0, gy0 + 0.25, 0.0],
        [gx0 + gw - 0.25, d - 0.25, 2.8],
        0.0,
    );
    gf.bx(b, "brick", (0.0, gw), (0.0, 0.55), (-0.01, 0.03));
    gf.bx(b, "garage", (gd.s.0, gd.s.1), (0.0, gd.z.1), (-0.12, -0.08));
    for k in 1..12 {
        let x = gd.s.0 + (gd.s.1 - gd.s.0) * k as f64 / 12.0;
        gf.bx(
            b,
            "garage",
            (x - 0.012, x + 0.012),
            (0.05, gd.z.1 - 0.05),
            (-0.08, -0.06),
        );
    }
    gf.bx(
        b,
        "brick",
        (gd.s.0 - 0.15, gd.s.1 + 0.15),
        (gd.z.1, gd.z.1 + 0.2),
        (-0.01, 0.03),
    );
    let side = Face {
        o: [gx0, d + 0.3],
        u: [0.0, -1.0],
        len: 6.6,
    };
    pent_roof(
        b,
        &side,
        (0.0, side.len),
        (-0.02, 3.3),
        (gw + 0.3, 3.3 - (gw + 0.32) * 0.3),
    );
    b.rbox("paving", [gx0, d, 0.0], [gx0 + gw, d + 6.0, 0.03], 0.0);

    // The garden: lawn, the low wall with piers and railings, the gate,
    // a palm, the pool behind.
    let (fx0, fx1, fy0, fy1) = (-w / 2.0 - 3.0, w / 2.0 + gw + 1.0, -6.0, d + 6.0);
    b.rbox("lawn", [fx0, fy0, 0.0], [fx1, fy1, 0.02], 0.0);
    b.rbox("paving", [-0.7, d + pd, 0.0], [0.7, fy1, 0.04], 0.0);
    let fence = Face::sides(fx1 - fx0, fy1 - fy0).map(|f| Face {
        o: [f.o[0] + (fx0 + fx1) / 2.0, f.o[1] + fy0],
        ..f
    });
    for (k, f) in fence.iter().enumerate() {
        let gaps: Vec<(f64, f64)> = if k == 0 {
            let gate_c = -fx0;
            vec![
                (gate_c - 0.7, gate_c + 0.7),
                (gx0 - fx0 + 0.1, gx0 - fx0 + gw - 0.1),
            ]
        } else {
            Vec::new()
        };
        let mut cuts = gaps.clone();
        cuts.push((f.len, f.len));
        let mut s = 0.0;
        for (c0, c1) in cuts {
            if c0 - s > 0.4 {
                f.bx(b, "wall", (s, c0), (0.0, 0.7), (-0.25, 0.0));
                f.bx(
                    b,
                    "brick",
                    (s - 0.02, c0 + 0.02),
                    (0.7, 0.76),
                    (-0.27, 0.02),
                );
                let n = ((c0 - s) / 2.5).ceil().max(1.0) as usize;
                for i in 0..=n {
                    let x = (s + (c0 - s) * i as f64 / n as f64).clamp(s + 0.2, c0 - 0.2);
                    f.bx(b, "brick", (x - 0.2, x + 0.2), (0.0, 1.7), (-0.33, 0.07));
                    f.bx(b, "trim", (x - 0.24, x + 0.24), (1.7, 1.78), (-0.37, 0.11));
                }
                railing(b, f, "iron", (s + 0.1, c0 - 0.1), 0.76, -0.12, 0.85, 0.12);
            }
            s = c1;
        }
        if k == 0 {
            let (g0, g1) = gaps[0];
            railing(b, f, "iron", (g0, g1), 0.05, -0.12, 1.5, 0.11);
        }
    }
    palm(b, [w / 2.0 + 1.5, d + 4.2], 6.5, seed);
    fruit_tree(b, [-w / 2.0 - 1.6, d + 3.5], 3.2, seed + 1, true);
    if pool {
        let (px0, px1, py0, py1) = (-w / 2.0 + 0.5, -w / 2.0 + 6.5, -5.0, -1.8);
        b.rbox(
            "coping",
            [px0 - 0.35, py0 - 0.35, 0.0],
            [px1 + 0.35, py1 + 0.35, 0.06],
            0.0,
        );
        b.rbox("pool_tile", [px0, py0, 0.0], [px1, py1, 0.065], 0.0);
        b.rbox(
            "water",
            [px0 + 0.02, py0 + 0.02, 0.065],
            [px1 - 0.02, py1 - 0.02, 0.07],
            0.0,
        );
        for x in [px1 - 0.6, px1 - 0.2] {
            b.cyl("metal", [x, py1 - 0.1, 0.06], [x, py1 + 0.15, 1.0], 0.02);
        }
    }
}

/// Louvred shutters (mallorquinas) open against the wall either side of a
/// hole; `open` 0 to 1 says how open the right one is (some are half
/// shut).
pub fn shutters(b: &mut Builder, f: &Face, h: &Hole, open: f64) {
    let lw = (h.s.1 - h.s.0) / 2.0 + 0.02;
    let (z0, z1) = (h.z.0 - 0.02, h.z.1 + 0.02);
    for (a, e) in [
        (h.s.0 - lw - 0.03, h.s.0 - 0.03),
        (h.s.1 + 0.03, h.s.1 + lw + 0.03),
    ] {
        f.bx(b, "shutter_wood", (a, e), (z0, z1), (0.0, 0.035));
        let mut z = z0 + 0.12;
        while z < z1 - 0.12 {
            f.bx(
                b,
                "shutter_wood",
                (a + 0.05, e - 0.05),
                (z, z + 0.025),
                (0.035, 0.045),
            );
            z += 0.07;
        }
        // The hinges.
        let x = if a < h.s.0 { e + 0.015 } else { a - 0.015 };
        for zz in [z0 + 0.2, z1 - 0.2] {
            f.bx(
                b,
                "iron",
                (x - 0.02, x + 0.02),
                (zz, zz + 0.05),
                (-0.01, 0.05),
            );
        }
    }
    let _ = open;
}
