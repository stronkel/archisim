//! What finishes a town (tanda 14): the cemetery, the hermitage, the
//! lighthouse, the fish market on its quay, the railway and the bus
//! stations, the hospital, the hotel, the stone bridge and the barracks of
//! the Guardia Civil (casa cuartel). Those that are a building of the
//! street wear the place's skin; the rest are drawn with its colours.

use super::super::ensanche::arch_fill;
use super::super::*;
use super::civic::{
    bench, canopy, car_at, civic, dressed, lamp_post, sign, street_front, tree, walls_box, Ctx,
};
use super::plan::{Plan, Side};
use super::roofs::{self, RoofPlan};
use super::skins::*;
use super::types::Typology;
use super::{look, Look};
use crate::build::Builder;
use std::f64::consts::PI;

// ------------------------------------------------------------ helpers

/// A plain roof plan over a rectangle.
fn plain_roof(
    x: (f64, f64),
    y: (f64, f64),
    z: f64,
    form: Roof,
    cover: Cover,
    pitch: f64,
    light: bool,
) -> RoofPlan {
    RoofPlan {
        x,
        y,
        z,
        form,
        cover,
        pitch,
        eaves: 0.3,
        party: false,
        dormers: Vec::new(),
        gable: Gable::Plain,
        seed: 1,
        extras: 0,
        top: Top::None,
        block: false,
        light,
    }
}

/// A small chapel: whitewashed nave with its door to y1, a window on each
/// side, a roof of curved tiles and the bell gable (espadaña) over the
/// front.
pub(super) fn chapel(b: &mut Builder, l: &Look, x: (f64, f64), y: (f64, f64), light: bool) {
    let (w, d) = (x.1 - x.0, y.1 - y.0);
    let h = 4.2;
    let door = Hole {
        s: (w / 2.0 - 0.8, w / 2.0 + 0.8),
        z: (0.0, 2.8),
    };
    let win = [Hole {
        s: (d * 0.5 - 0.35, d * 0.5 + 0.35),
        z: (2.2, 3.4),
    }];
    let fs = walls_box(b, x, y, (0.0, h), l.wall, [&[door], &win, &[], &win], true);
    arch_fill(b, &fs[0], l.wall, door.s, door.z.1 - 0.8, (-0.3, 0.0));
    fs[0].bx(b, l.door, door.s, door.z, (-0.22, -0.16));
    for k in [1, 3] {
        fs[k].bx(b, "glass", win[0].s, win[0].z, (-0.2, -0.16));
    }
    // The stone round the door.
    fs[0].bx(
        b,
        l.trim,
        (door.s.0 - 0.3, door.s.0),
        (0.0, door.z.1),
        (0.0, 0.08),
    );
    fs[0].bx(
        b,
        l.trim,
        (door.s.1, door.s.1 + 0.3),
        (0.0, door.z.1),
        (0.0, 0.08),
    );
    fs[0].bx(
        b,
        l.trim,
        (door.s.0 - 0.3, door.s.1 + 0.3),
        (door.z.1, door.z.1 + 0.3),
        (0.0, 0.08),
    );
    let pitch = 0.42;
    roofs::roof(
        b,
        &plain_roof(x, y, h, Roof::GableToStreet, Cover::Curved, pitch, light),
        l,
    );
    // The bell gable on the front, its bell in the arch.
    let f = fs[0];
    let apex = h + w / 2.0 * pitch;
    let sc = w / 2.0;
    let z0 = apex - 0.6;
    let dd = (-0.3, 0.0);
    f.bx(b, l.wall, (sc - 1.1, sc - 0.45), (z0, apex + 1.6), dd);
    f.bx(b, l.wall, (sc + 0.45, sc + 1.1), (z0, apex + 1.6), dd);
    f.bx(b, l.wall, (sc - 0.45, sc + 0.45), (z0, apex + 0.2), dd);
    f.prism(
        b,
        l.wall,
        &[
            (sc - 1.2, apex + 1.6),
            (sc + 1.2, apex + 1.6),
            (sc, apex + 2.4),
        ],
        dd,
    );
    let c = f.at(sc, 0.0, -0.15);
    b.cone(
        "brass",
        [c[0], c[1], apex + 0.6],
        [c[0], c[1], apex + 1.6],
        0.36,
        0.12,
        10,
    );
    if !light {
        let t = f.at(sc, apex + 2.4, -0.15);
        b.cyl("iron", t, [t[0], t[1], t[2] + 0.9], 0.03);
        b.cyl(
            "iron",
            [t[0] - f.u[0] * 0.25, t[1] - f.u[1] * 0.25, t[2] + 0.6],
            [t[0] + f.u[0] * 0.25, t[1] + f.u[1] * 0.25, t[2] + 0.6],
            0.03,
        );
    }
}

/// A cypress.
fn cypress(b: &mut Builder, c: [f64; 2], h: f64) {
    b.cyl("trunk", [c[0], c[1], 0.0], [c[0], c[1], 0.8], 0.12);
    b.cone(
        "leaves",
        [c[0], c[1], 0.6],
        [c[0], c[1], h * 0.45],
        0.55,
        0.6,
        9,
    );
    b.cone(
        "leaves",
        [c[0], c[1], h * 0.45],
        [c[0], c[1], h],
        0.6,
        0.05,
        9,
    );
}

/// A flag hung out of a face on its pole.
fn wall_flag(b: &mut Builder, f: &Face, s: f64, z: f64, role: &'static str) {
    let a = f.at(s, z, -0.2);
    let e = f.at(s, z + 0.9, 1.6);
    b.cone("metal", a, e, 0.025, 0.025, 6);
    let m = f.at(s, z + 0.1, 1.2);
    b.obox(
        role,
        [m[0], m[1], m[2] - 0.15],
        [f.u[0] * 0.02, f.u[1] * 0.02, 0.0],
        [f.out()[0] * 0.5, f.out()[1] * 0.5, 0.0],
        [0.0, 0.0, 0.55],
    );
}

// ----------------------------------------------------------- the town

/// The cemetery: the whitewashed wall with its gate, an avenue of
/// cypresses, rows of niches, the graves and the chapel at the back.
pub(super) fn cemetery(b: &mut Builder, c: &Ctx) -> Look {
    let l0 = look(c.sk, c.seed, false);
    let l = Look {
        wall: "w_white",
        roof: "r_curved",
        ..l0
    };
    let (w, d) = (c.w, c.d);
    let (x0, x1) = (-w / 2.0, w / 2.0);
    b.rbox("r_gravel", [x0, 0.0, 0.0], [x1, d, 0.03], 0.0);
    let gate = Hole {
        s: (w / 2.0 - 1.8, w / 2.0 + 1.8),
        z: (0.0, 2.6),
    };
    let fs = walls_box(
        b,
        (x0, x1),
        (0.0, d),
        (0.0, 2.6),
        l.wall,
        [&[gate], &[], &[], &[]],
        false,
    );
    for f in &fs {
        f.bx(b, l.trim, (0.0, f.len), (2.6, 2.72), (-0.35, 0.05));
    }
    // The gate: two pillars, their arch with a cross, the iron leaves.
    let f = fs[0];
    for s in [gate.s.0 - 0.6, gate.s.1] {
        f.bx(b, l.trim, (s, s + 0.6), (0.0, 3.4), (-0.4, 0.15));
    }
    f.bx(
        b,
        l.trim,
        (gate.s.0 - 0.6, gate.s.1 + 0.6),
        (3.4, 3.8),
        (-0.4, 0.15),
    );
    f.bx(
        b,
        "iron",
        (w / 2.0 - 0.05, w / 2.0 + 0.05),
        (3.8, 4.8),
        (-0.15, -0.05),
    );
    f.bx(
        b,
        "iron",
        (w / 2.0 - 0.35, w / 2.0 + 0.35),
        (4.35, 4.45),
        (-0.15, -0.05),
    );
    if !c.light {
        let n = 24;
        for i in 0..=n {
            let s = gate.s.0 + (gate.s.1 - gate.s.0) * i as f64 / n as f64;
            f.bx(
                b,
                "iron",
                (s - 0.015, s + 0.015),
                (0.0, 2.4),
                (-0.22, -0.18),
            );
        }
        f.bx(b, "iron", gate.s, (2.3, 2.4), (-0.22, -0.18));
        f.bx(b, "iron", gate.s, (0.1, 0.18), (-0.22, -0.18));
    }
    // The avenue of cypresses to the chapel.
    let cw = 8.0_f64.min(w * 0.3);
    let cd = 11.0_f64.min(d * 0.3);
    let chapel_y0 = 1.0;
    let mut y = d - 3.0;
    while y > chapel_y0 + cd + 2.0 {
        for x in [-2.5, 2.5] {
            cypress(b, [x, y], 9.0);
        }
        y -= 4.5;
    }
    chapel(
        b,
        &l,
        (-cw / 2.0, cw / 2.0),
        (chapel_y0, chapel_y0 + cd),
        c.light,
    );
    // The rows of niches along the side walls.
    for side in [-1.0, 1.0] {
        let xa = if side < 0.0 { x0 + 0.3 } else { x1 - 2.8 };
        let (ya, yb) = (2.0, d - 3.0);
        b.rbox(l.wall, [xa, ya, 0.0], [xa + 2.5, yb, 3.2], 0.0);
        b.rbox(
            l.trim,
            [xa - 0.1, ya - 0.1, 3.2],
            [xa + 2.6, yb + 0.1, 3.35],
            0.0,
        );
        if !c.light {
            // The face of the niches, out into the cemetery.
            let face = if side < 0.0 {
                Face {
                    o: [xa + 2.5, yb],
                    u: [0.0, -1.0],
                    len: yb - ya,
                }
            } else {
                Face {
                    o: [xa, ya],
                    u: [0.0, 1.0],
                    len: yb - ya,
                }
            };
            let cols = ((yb - ya) / 0.85).floor() as usize;
            for i in 0..cols {
                for j in 0..4 {
                    let s = 0.1 + 0.85 * i as f64;
                    let z = 0.1 + 0.75 * j as f64;
                    face.bx(b, "s_lime", (s, s + 0.75), (z, z + 0.65), (0.0, 0.03));
                }
            }
        }
    }
    // The graves in rows on each side of the avenue.
    if !c.light {
        let mut k = 0;
        for side in [-1.0, 1.0] {
            let xs: [f64; 3] = [5.0, 7.5, 10.0];
            for x in xs {
                let x = side * x;
                if x.abs() + 1.0 > w / 2.0 - 3.0 {
                    continue;
                }
                let mut y = chapel_y0 + cd + 2.0;
                while y < d - 4.0 {
                    k += 1;
                    b.rbox(
                        "s_granite",
                        [x - 0.5, y, 0.0],
                        [x + 0.5, y + 2.1, 0.35],
                        0.0,
                    );
                    if k % 3 == 0 {
                        b.rbox(
                            "w_white",
                            [x - 0.06, y + 1.95, 0.35],
                            [x + 0.06, y + 2.05, 1.4],
                            0.0,
                        );
                        b.rbox(
                            "w_white",
                            [x - 0.35, y + 1.95, 1.0],
                            [x + 0.35, y + 2.05, 1.1],
                            0.0,
                        );
                    } else {
                        b.rbox(
                            "s_granite",
                            [x - 0.45, y + 1.9, 0.35],
                            [x + 0.45, y + 2.05, 1.0],
                            0.0,
                        );
                        b.ellipsoid("flowers", [x, y + 1.6, 0.42], [0.25, 0.15, 0.1]);
                    }
                    y += 3.0;
                }
            }
        }
    }
    l
}

/// The hermitage in the fields: the chapel with its porch, a cross on its
/// steps, two trees and a bench.
pub(super) fn hermitage(b: &mut Builder, c: &Ctx) -> Look {
    let l0 = look(c.sk, c.seed, false);
    let l = Look {
        wall: "w_white",
        roof: "r_curved",
        ..l0
    };
    let (w, d) = (c.w, c.d);
    b.rbox("sand", [-w / 2.0, 0.0, 0.0], [w / 2.0, d, 0.03], 0.0);
    let cw = (w - 4.0).clamp(6.0, 9.0);
    let cd = (d - 6.0).clamp(8.0, 14.0);
    let y1 = 1.0 + cd;
    chapel(b, &l, (-cw / 2.0, cw / 2.0), (1.0, y1), c.light);
    // The porch on four posts before the door.
    let pd = 2.6;
    for x in [-cw / 2.0 + 0.3, cw / 2.0 - 0.3] {
        b.rbox(
            l.wall,
            [x - 0.25, y1 + pd - 0.5, 0.0],
            [x + 0.25, y1 + pd, 2.6],
            0.0,
        );
    }
    let f = Face {
        o: [-cw / 2.0, y1],
        u: [1.0, 0.0],
        len: cw,
    };
    f.leaning(
        b,
        "r_curved",
        (-0.2, cw + 0.2),
        (3.2, -0.02),
        (2.6, pd + 0.2),
        0.12,
    );
    // The cross on its steps.
    if !c.light {
        let p = [0.0, (y1 + pd + d) / 2.0 + 0.5];
        for (k, r) in [1.0, 0.7].iter().enumerate() {
            b.rbox(
                l.trim,
                [p[0] - r, p[1] - r, 0.2 * k as f64],
                [p[0] + r, p[1] + r, 0.2 * (k + 1) as f64],
                0.0,
            );
        }
        b.rbox(
            l.trim,
            [p[0] - 0.12, p[1] - 0.12, 0.4],
            [p[0] + 0.12, p[1] + 0.12, 3.0],
            0.0,
        );
        b.rbox(
            l.trim,
            [p[0] - 0.6, p[1] - 0.1, 2.2],
            [p[0] + 0.6, p[1] + 0.1, 2.4],
            0.0,
        );
        tree(b, [-w / 2.0 + 1.5, y1 - 2.0], 0.0, 7.0, c.seed);
        bench(b, [cw / 2.0 + 0.8, y1 - 3.0], [0.0, 1.0], "b_cedar");
    }
    l
}

/// The lighthouse: on its rock, the tower in bands, the gallery and the
/// lantern; the keeper's house at its foot.
pub(super) fn lighthouse(b: &mut Builder, c: &Ctx) -> Look {
    let l = look(c.sk, c.seed, false);
    let (w, d) = (c.w, c.d);
    let (rb, rt) = (w * 0.4, w * 0.25);
    let p = [0.0, rb + 1.0];
    b.cone("s_granite", [p[0], p[1], 0.0], [p[0], p[1], 1.6], rb, rt, 9);
    let h = 22.0;
    let (r0, r1) = (2.4, 1.6);
    b.cone(
        "w_white",
        [p[0], p[1], 1.6],
        [p[0], p[1], 1.6 + h],
        r0,
        r1,
        20,
    );
    // Its bands.
    let bands = 3;
    for k in 0..bands {
        let za = 1.6 + h * (0.25 + 0.25 * k as f64);
        let t = (za - 1.6) / h;
        let r = r0 + (r1 - r0) * t + 0.03;
        b.cone(
            "f_red",
            [p[0], p[1], za],
            [p[0], p[1], za + 2.0],
            r,
            r - 0.15,
            20,
        );
    }
    let zt = 1.6 + h;
    b.cone(
        "w_white",
        [p[0], p[1], zt],
        [p[0], p[1], zt + 0.3],
        r1 + 0.7,
        r1 + 0.7,
        20,
    );
    if !c.light {
        for k in 0..16 {
            let a = 2.0 * PI * k as f64 / 16.0;
            let q = [p[0] + (r1 + 0.6) * a.cos(), p[1] + (r1 + 0.6) * a.sin()];
            b.cyl(
                "iron",
                [q[0], q[1], zt + 0.3],
                [q[0], q[1], zt + 1.3],
                0.025,
            );
        }
        b.cone(
            "iron",
            [p[0], p[1], zt + 1.25],
            [p[0], p[1], zt + 1.32],
            r1 + 0.63,
            r1 + 0.63,
            20,
        );
        b.cone(
            "iron",
            [p[0], p[1], zt + 1.2],
            [p[0], p[1], zt + 1.32],
            r1 + 0.57,
            r1 + 0.57,
            20,
        );
    }
    b.cone(
        "f_red",
        [p[0], p[1], zt + 0.3],
        [p[0], p[1], zt + 0.8],
        1.3,
        1.3,
        16,
    );
    b.cone(
        "glass",
        [p[0], p[1], zt + 0.8],
        [p[0], p[1], zt + 3.0],
        1.2,
        1.2,
        16,
    );
    b.ellipsoid("lamp", [p[0], p[1], zt + 1.9], [0.5, 0.5, 0.6]);
    b.cone(
        "f_red",
        [p[0], p[1], zt + 3.0],
        [p[0], p[1], zt + 4.2],
        1.4,
        0.15,
        16,
    );
    b.cyl("iron", [p[0], p[1], zt + 4.2], [p[0], p[1], zt + 4.8], 0.05);
    b.ellipsoid("iron", [p[0], p[1], zt + 4.85], [0.12, 0.12, 0.12]);
    // The keeper's house before the tower.
    let (hx0, hx1) = (-w * 0.3, w * 0.3);
    let hy0 = p[1] + rb + 0.5;
    let hy1 = (hy0 + 6.5).min(d - 0.5).max(hy0 + 4.0);
    let door = Hole {
        s: ((hx1 - hx0) / 2.0 - 0.5, (hx1 - hx0) / 2.0 + 0.5),
        z: (0.0, 2.2),
    };
    let wins = [
        Hole {
            s: (1.0, 2.0),
            z: (1.0, 2.2),
        },
        Hole {
            s: (hx1 - hx0 - 2.0, hx1 - hx0 - 1.0),
            z: (1.0, 2.2),
        },
    ];
    let holes: Vec<Hole> = std::iter::once(door).chain(wins).collect();
    let fs = walls_box(
        b,
        (hx0, hx1),
        (hy0, hy1),
        (0.0, 3.2),
        "w_white",
        [&holes, &[], &[], &[]],
        true,
    );
    fs[0].bx(b, "f_greyblue", door.s, door.z, (-0.22, -0.16));
    for h in &wins {
        fs[0].bx(b, "glass", h.s, h.z, (-0.2, -0.16));
    }
    roofs::roof(
        b,
        &plain_roof(
            (hx0, hx1),
            (hy0, hy1),
            3.2,
            Roof::Hip,
            Cover::Curved,
            0.4,
            c.light,
        ),
        &Look {
            roof: "r_curved",
            wall: "w_white",
            ..l
        },
    );
    Look {
        roof: "r_curved",
        wall: "w_white",
        ..l
    }
}

/// A fishing boat on the water.
fn boat(b: &mut Builder, c: [f64; 2], len: f64, role: &'static str, water: f64, light: bool) {
    let hw = len * 0.18;
    let poly = vec![
        [c[0] - len / 2.0, c[1] - hw * 0.8],
        [c[0] + len * 0.25, c[1] - hw],
        [c[0] + len / 2.0, c[1]],
        [c[0] + len * 0.25, c[1] + hw],
        [c[0] - len / 2.0, c[1] + hw * 0.8],
    ];
    b.slab(role, &poly, water - 0.3, water + 0.7);
    b.slab("w_white", &poly, water + 0.7, water + 0.8);
    b.rbox(
        "w_white",
        [c[0] - len * 0.3, c[1] - hw * 0.5, water + 0.8],
        [c[0] - len * 0.05, c[1] + hw * 0.5, water + 2.0],
        0.0,
    );
    if !light {
        b.cyl(
            "metal",
            [c[0] + len * 0.1, c[1], water + 0.8],
            [c[0] + len * 0.1, c[1], water + 5.0],
            0.05,
        );
        b.cyl(
            "metal",
            [c[0] + len * 0.1, c[1], water + 3.0],
            [c[0] + len * 0.4, c[1], water + 1.0],
            0.03,
        );
    }
}

/// The fish market on its quay: the hall open on its sides under a
/// pitched roof, boxes of fish, bollards and the boats at the quay.
pub(super) fn harbour(b: &mut Builder, c: &Ctx) -> Look {
    let l = look(c.sk, c.seed, false);
    let (w, d) = (c.w, c.d);
    let ys = d * 0.4;
    let zq = 1.6;
    let wl = 1.0;
    b.rbox("water", [-w / 2.0, 0.0, 0.0], [w / 2.0, ys, wl], 0.0);
    b.rbox("s_granite", [-w / 2.0, ys, 0.0], [w / 2.0, d, zq], 0.0);
    b.rbox(
        "paving",
        [-w / 2.0, ys + 0.6, zq],
        [w / 2.0, d, zq + 0.03],
        0.0,
    );
    b.rbox(
        l.trim,
        [-w / 2.0, ys, zq],
        [w / 2.0, ys + 0.6, zq + 0.1],
        0.0,
    );
    // The bollards along the edge.
    let n = (w / 6.0).floor() as usize;
    for i in 0..n {
        let x = -w / 2.0 + 3.0 + 6.0 * i as f64;
        b.cone(
            "iron",
            [x, ys + 0.3, zq + 0.1],
            [x, ys + 0.3, zq + 0.6],
            0.16,
            0.13,
            10,
        );
        b.ellipsoid("iron", [x, ys + 0.3, zq + 0.62], [0.2, 0.2, 0.08]);
    }
    // The hall.
    let (hx0, hx1) = (-w * 0.35, w * 0.35);
    let (hy0, hy1) = (ys + 3.0, d - 2.0);
    let hh = 5.0;
    let m = ((hx1 - hx0) / 5.0).round().max(2.0) as usize;
    for i in 0..=m {
        let x = hx0 + (hx1 - hx0) * i as f64 / m as f64;
        for y in [hy0, hy1] {
            b.rbox(
                l.wall,
                [x - 0.25, y - 0.25, zq],
                [x + 0.25, y + 0.25, zq + hh],
                0.0,
            );
        }
    }
    b.rbox(
        l.trim,
        [hx0 - 0.25, hy0 - 0.25, zq + hh - 0.6],
        [hx1 + 0.25, hy1 + 0.25, zq + hh],
        0.0,
    );
    roofs::roof(
        b,
        &plain_roof(
            (hx0 - 0.25, hx1 + 0.25),
            (hy0 - 0.25, hy1 + 0.25),
            zq + hh,
            Roof::Gable,
            Cover::Seam,
            0.35,
            c.light,
        ),
        &Look {
            roof: "r_galv",
            ..l
        },
    );
    // A blind wall at the back, the boards over the tables.
    b.rbox(
        l.wall,
        [hx0, hy1 - 0.25, zq],
        [hx1, hy1, zq + hh - 0.6],
        0.0,
    );
    if !c.light {
        let f = Face {
            o: [hx1, hy0 - 0.25],
            u: [-1.0, 0.0],
            len: hx1 - hx0,
        };
        sign(
            b,
            &f,
            (f.len / 2.0 - 3.0, f.len / 2.0 + 3.0),
            (zq + hh - 0.55, zq + hh - 0.05),
            "f_blue",
            "f_white",
            5,
            false,
        );
        let t = ((hx1 - hx0 - 4.0) / 3.5).floor().max(1.0) as usize;
        for i in 0..t {
            let x = hx0 + 2.0 + 3.5 * i as f64 + 1.2;
            let y = (hy0 + hy1) / 2.0;
            b.rbox(
                "w_white",
                [x - 1.2, y - 0.6, zq],
                [x + 1.2, y + 0.6, zq + 0.85],
                0.0,
            );
            for k in 0..3 {
                let xb = x - 0.8 + 0.8 * k as f64;
                b.rbox(
                    "f_blue",
                    [xb - 0.35, y - 0.25, zq + 0.85],
                    [xb + 0.35, y + 0.25, zq + 1.05],
                    0.0,
                );
                b.ellipsoid("s_granite", [xb, y, zq + 1.05], [0.28, 0.18, 0.06]);
            }
        }
        // Crates on the quay.
        for k in 0..4 {
            let x = hx1 + 2.0 + (k % 2) as f64 * 0.7;
            let z = zq + 0.4 * (k / 2) as f64;
            b.rbox(
                "b_cedar",
                [x, ys + 2.0, z],
                [x + 0.6, ys + 2.6, z + 0.4],
                0.0,
            );
        }
    }
    // The boats.
    let roles = ["b_blue", "f_red", "b_green", "b_white"];
    let nb = ((w - 6.0) / 9.0).floor().max(1.0) as usize;
    for i in 0..nb {
        let x = -w / 2.0 + 6.0 + 9.0 * i as f64;
        boat(b, [x, ys - 2.4], 8.0, roles[i % 4], wl, c.light);
    }
    l
}

/// Tracks with their sleepers on the ballast along x at `y`.
fn track(b: &mut Builder, x: (f64, f64), y: f64, light: bool) {
    b.rbox("r_gravel", [x.0, y - 1.6, 0.0], [x.1, y + 1.6, 0.25], 0.0);
    if !light {
        let n = ((x.1 - x.0) / 0.6).floor() as usize;
        for i in 0..n {
            let xs = x.0 + 0.3 + 0.6 * i as f64;
            b.rbox(
                "b_dark",
                [xs - 0.12, y - 1.25, 0.25],
                [xs + 0.12, y + 1.25, 0.37],
                0.0,
            );
        }
    } else {
        b.rbox("b_dark", [x.0, y - 1.25, 0.25], [x.1, y + 1.25, 0.37], 0.0);
    }
    for r in [-0.72, 0.72] {
        b.rbox(
            "metal",
            [x.0, y + r - 0.04, 0.37],
            [x.1, y + r + 0.04, 0.52],
            0.0,
        );
    }
}

/// The railway station: the building on the street, the platform with
/// its canopy and clock, two tracks and a train.
pub(super) fn train_station(b: &mut Builder, c: &Ctx) -> Look {
    let (w, d) = (c.w, c.d);
    let mut sk = civic(c.sk, Typology::Townhouse);
    sk.storey = (sk.storey.0.max(4.2), sk.storey.1.max(3.6));
    let bw = (w * 0.5).clamp(16.0, 40.0);
    let bd = 11.0;
    let yb = d - bd;
    let plan = Plan {
        poly: vec![
            [-bw / 2.0, yb],
            [bw / 2.0, yb],
            [bw / 2.0, d],
            [-bw / 2.0, d],
        ],
        sides: vec![Side::Rear, Side::Free, Side::Street, Side::Free],
    };
    let (built, l) = dressed(b, &plan, &sk, c.floors.max(1), c.seed, c.light);
    let f = built.fronts[2].map(|fr| fr.f).unwrap_or(Face {
        o: [-bw / 2.0, d],
        u: [1.0, 0.0],
        len: bw,
    });
    let z1 = built.fronts[2].map_or(4.2, |fr| fr.z(1));
    sign(
        b,
        &f,
        (f.len / 2.0 - 4.0, f.len / 2.0 + 4.0),
        (z1 - 0.55, z1 - 0.1),
        "f_black",
        "f_white",
        8,
        c.light,
    );
    // The platform behind it with its canopy.
    let (py0, py1) = (yb - 6.0, yb);
    b.rbox(l.trim, [-w / 2.0, py0, 0.0], [w / 2.0, py1, 0.9], 0.0);
    b.rbox("paving", [-w / 2.0, py0, 0.9], [w / 2.0, py1, 0.93], 0.0);
    let n = (w / 6.0).floor().max(2.0) as usize;
    for i in 0..=n {
        let x = -w / 2.0 + 1.0 + (w - 2.0) * i as f64 / n as f64;
        b.cyl("iron", [x, py1 - 1.5, 0.93], [x, py1 - 1.5, 4.6], 0.1);
    }
    b.rbox(
        "iron",
        [-w / 2.0, py1 - 1.7, 4.4],
        [w / 2.0, py1 - 1.3, 4.6],
        0.0,
    );
    b.rbox(
        "r_galv",
        [-w / 2.0, py0 - 0.4, 4.6],
        [w / 2.0, py1 - 0.2, 4.75],
        0.0,
    );
    if !c.light {
        let x = 1.0 + (w - 2.0) / n as f64 - w / 2.0;
        b.cone(
            "w_white",
            [x, py1 - 1.5 + 0.1, 3.4],
            [x, py1 - 1.5 + 0.18, 3.4],
            0.35,
            0.35,
            14,
        );
        for k in 0..3 {
            let xb = -w / 4.0 + w / 4.0 * k as f64;
            let s = b.mark();
            bench(b, [xb, py1 - 0.6], [-1.0, 0.0], "b_cedar");
            super::places::lift(b, s, 0.93);
        }
    }
    // The tracks and a train at the platform.
    let t1 = py0 - 2.2;
    let t2 = t1 - 4.6;
    track(b, (-w / 2.0, w / 2.0), t1, c.light);
    if t2 - 1.6 > 0.0 {
        track(b, (-w / 2.0, w / 2.0), t2, c.light);
    }
    let cars = ((w - 4.0) / 18.0).floor().max(1.0) as usize;
    for k in 0..cars {
        let x0 = -w / 2.0 + 2.0 + 18.0 * k as f64;
        let x1 = x0 + 17.0;
        for xw in [x0 + 2.0, x0 + 3.6, x1 - 3.6, x1 - 2.0] {
            for r in [-0.72, 0.72] {
                b.cone(
                    "iron",
                    [xw, t1 + r - 0.05, 0.92],
                    [xw, t1 + r + 0.05, 0.92],
                    0.42,
                    0.42,
                    10,
                );
            }
        }
        b.rbox(
            "metal",
            [x0 + 1.2, t1 - 0.9, 0.9],
            [x1 - 1.2, t1 + 0.9, 1.25],
            0.0,
        );
        b.rbox("f_red", [x0, t1 - 1.45, 1.25], [x1, t1 + 1.45, 2.3], 0.0);
        b.rbox(
            "glass",
            [x0 + 0.5, t1 - 1.46, 2.3],
            [x1 - 0.5, t1 + 1.46, 3.1],
            0.0,
        );
        b.rbox(
            "w_white",
            [x0, t1 - 1.45, 2.3],
            [x0 + 0.5, t1 + 1.45, 3.1],
            0.0,
        );
        b.rbox(
            "w_white",
            [x1 - 0.5, t1 - 1.45, 2.3],
            [x1, t1 + 1.45, 3.1],
            0.0,
        );
        b.rbox("w_white", [x0, t1 - 1.45, 3.1], [x1, t1 + 1.45, 3.8], 0.15);
    }
    l
}

/// A bus.
fn bus(b: &mut Builder, c: [f64; 2], along_y: bool) {
    let (hx, hy) = if along_y { (1.25, 6.0) } else { (6.0, 1.25) };
    for (sx, sy) in [(-1.0, -1.0), (1.0, -1.0), (1.0, 1.0), (-1.0, 1.0)] {
        let p = if along_y {
            [c[0] + sx * hx, c[1] + sy * (hy - 1.8)]
        } else {
            [c[0] + sx * (hx - 1.8), c[1] + sy * hy]
        };
        let n = if along_y { [1.0, 0.0] } else { [0.0, 1.0] };
        b.cone(
            "f_black",
            [p[0] - n[0] * 0.15, p[1] - n[1] * 0.15, 0.5],
            [p[0] + n[0] * 0.15, p[1] + n[1] * 0.15, 0.5],
            0.5,
            0.5,
            10,
        );
    }
    b.rbox(
        "f_white",
        [c[0] - hx, c[1] - hy, 0.5],
        [c[0] + hx, c[1] + hy, 1.6],
        0.1,
    );
    b.rbox(
        "glass",
        [c[0] - hx, c[1] - hy, 1.6],
        [c[0] + hx, c[1] + hy, 2.6],
        0.0,
    );
    b.rbox(
        "f_greyblue",
        [c[0] - hx, c[1] - hy, 2.6],
        [c[0] + hx, c[1] + hy, 3.1],
        0.15,
    );
}

/// The bus station: the waiting room on the street, the canopy over the
/// bays and the buses in them.
pub(super) fn bus_station(b: &mut Builder, c: &Ctx) -> Look {
    let (w, d) = (c.w, c.d);
    let sk = civic(c.sk, Typology::Townhouse);
    b.rbox("c_concrete", [-w / 2.0, 0.0, 0.0], [w / 2.0, d, 0.03], 0.0);
    let bw = (w * 0.4).clamp(12.0, 24.0);
    let plan = Plan {
        poly: vec![
            [-w / 2.0, d - 10.0],
            [-w / 2.0 + bw, d - 10.0],
            [-w / 2.0 + bw, d],
            [-w / 2.0, d],
        ],
        sides: vec![Side::Back, Side::Free, Side::Street, Side::Free],
    };
    let (built, l) = dressed(b, &plan, &sk, 1, c.seed, c.light);
    let (f, _) = street_front(&built, bw, 10.0);
    let z1 = built.fronts[2].map_or(4.0, |fr| fr.z(1));
    sign(
        b,
        &f,
        (1.0, f.len - 1.0),
        (z1 - 0.5, z1 - 0.1),
        "f_blue",
        "f_white",
        8,
        c.light,
    );
    // The canopy along the bays.
    let (cx0, cx1) = (-w / 2.0 + bw + 2.0, w / 2.0 - 1.0);
    let (cy0, cy1) = (d - 9.0, d - 5.0);
    let n = ((cx1 - cx0) / 6.0).floor().max(1.0) as usize;
    for i in 0..=n {
        let x = cx0 + (cx1 - cx0) * i as f64 / n as f64;
        b.cyl("metal", [x, cy1 - 0.5, 0.03], [x, cy1 - 0.5, 4.2], 0.12);
    }
    b.rbox(
        "r_galv",
        [cx0 - 0.5, cy0 - 1.0, 4.2],
        [cx1 + 0.5, cy1, 4.45],
        0.0,
    );
    if !c.light {
        for k in 0..n {
            let x = cx0 + (cx1 - cx0) * (k as f64 + 0.5) / n as f64;
            bench(b, [x, cy1 - 1.2], [-1.0, 0.0], "b_cedar");
        }
    }
    // The bays and their buses, across.
    let nb = ((cx1 - cx0) / 4.2).floor().max(1.0) as usize;
    for k in 0..nb {
        let x = cx0 + 2.1 + 4.2 * k as f64;
        b.rbox(
            "w_white",
            [x + 2.0, 2.0, 0.03],
            [x + 2.15, cy0 - 1.5, 0.05],
            0.0,
        );
        if k % 3 != 2 {
            bus(b, [x, cy0 - 7.8], true);
        }
    }
    l
}

/// The hospital: a long block set back behind its forecourt, its entrance
/// under a canopy with the cross, and the ambulances.
pub(super) fn hospital(b: &mut Builder, c: &Ctx) -> Look {
    let (w, d) = (c.w, c.d);
    let mut sk = civic(c.sk, Typology::Block);
    sk.ground = Ground::Door;
    sk.balc = Balc::None;
    sk.roof = Roof::Flat;
    sk.roof_mat = "r_gravel";
    sk.top = Top::Band;
    sk.block.roof = Roof::Flat;
    let hd = (d - 8.0).clamp(12.0, 20.0);
    let plan = Plan {
        poly: vec![
            [-w / 2.0, 0.0],
            [w / 2.0, 0.0],
            [w / 2.0, hd],
            [-w / 2.0, hd],
        ],
        sides: vec![Side::Rear, Side::Free, Side::Street, Side::Free],
    };
    let (built, l) = dressed(b, &plan, &sk, c.floors.max(3), c.seed, c.light);
    b.rbox("c_concrete", [-w / 2.0, hd, 0.0], [w / 2.0, d, 0.03], 0.0);
    let f = built.fronts[2].map(|fr| fr.f).unwrap_or(Face {
        o: [-w / 2.0, hd],
        u: [1.0, 0.0],
        len: w,
    });
    let mid = f.len / 2.0;
    canopy(b, &f, (mid - 4.0, mid + 4.0), 3.4, 5.0, "w_white", true);
    let z1 = built.fronts[2].map_or(4.0, |fr| fr.z(1));
    sign(
        b,
        &f,
        (mid - 5.0, mid + 5.0),
        (z1 + 0.2, z1 + 0.8),
        "w_white",
        "f_blue",
        8,
        c.light,
    );
    // The cross high on the front.
    let zc = built.top - 2.0;
    f.bx(
        b,
        "f_red",
        (f.len - 3.5, f.len - 1.5),
        (zc - 0.3, zc + 0.3),
        (0.0, 0.12),
    );
    f.bx(
        b,
        "f_red",
        (f.len - 2.8, f.len - 2.2),
        (zc - 1.0, zc + 1.0),
        (0.0, 0.12),
    );
    if !c.light {
        for k in 0..2 {
            let x = -w / 2.0 + 4.0 + 3.0 * k as f64;
            car_at(b, [x, hd + 4.0], 0.0, 0.1);
        }
        for x in [-w / 2.0 + 1.5, w / 2.0 - 1.5] {
            tree(b, [x, d - 1.5], 0.0, 6.0, c.seed);
        }
    }
    l
}

/// The hotel: a block of the place with its canopy over the door, flags
/// and its name high on the front.
pub(super) fn hotel(b: &mut Builder, c: &Ctx) -> Look {
    let (w, d) = (c.w, c.d);
    let mut sk = super::types::adapted(c.sk, Typology::Block);
    sk.ground = Ground::Door;
    sk.extras &= !(LAUNDRY | AC);
    let plan = Plan::rect(w, d, [Side::Back, Side::Free, Side::Street, Side::Free]);
    let (built, l) = dressed(b, &plan, &sk, c.floors, c.seed, c.light);
    let (f, top) = street_front(&built, w, d);
    let axes = &built.axes[2];
    let door = axes.get(axes.len() / 2).copied().unwrap_or(f.len / 2.0);
    canopy(b, &f, (door - 2.0, door + 2.0), 3.1, 3.2, "f_black", true);
    let zf = built.fronts[2].map_or(top - 3.0, |fr| fr.z(fr.floors.saturating_sub(1)));
    let zs = (zf - 0.7).max(4.0);
    sign(
        b,
        &f,
        (f.len / 2.0 - 4.0, f.len / 2.0 + 4.0),
        (zs, zs + 0.6),
        "f_black",
        "brass",
        5,
        c.light,
    );
    if !c.light {
        for (k, s) in [door - 3.0, door + 3.0, door + 4.2].iter().enumerate() {
            let role = ["cloth_a", "cloth_b", "cloth_c"][k];
            wall_flag(b, &f, *s, sk.storey.0 + 0.15, role);
        }
    }
    l
}

/// The stone bridge over a river: arches between piers, its parapets and
/// the banks.
pub(super) fn bridge(b: &mut Builder, c: &Ctx) -> Look {
    let l = look(c.sk, c.seed, false);
    let (w, d) = (c.w, c.d);
    let stone = "s_ashlar";
    let (ya, yb) = (d * 0.3, d * 0.7);
    let deck = 3.0;
    let wl = 1.0;
    // The river and its banks.
    b.rbox("sand", [-w / 2.0, ya, 0.0], [w / 2.0, yb, 0.1], 0.0);
    b.rbox("water", [-w / 2.0, ya, 0.1], [w / 2.0, yb, wl], 0.0);
    for (y0, y1) in [(0.0, ya), (yb, d)] {
        b.rbox(
            "s_rubble",
            [-w / 2.0, y0, 0.0],
            [w / 2.0, y1, deck - 0.05],
            0.0,
        );
        b.rbox(
            "lawn",
            [-w / 2.0, y0, deck - 0.05],
            [w / 2.0, y1, deck],
            0.0,
        );
    }
    // The road over the banks and the bridge.
    let bw = 7.0_f64.min(w * 0.5);
    for (y0, y1) in [(0.0, ya), (yb, d)] {
        b.rbox(
            "s_granite",
            [-bw / 2.0, y0, deck],
            [bw / 2.0, y1, deck + 0.04],
            0.0,
        );
    }
    let span = yb - ya;
    let narches = 4;
    let pier = 1.2;
    let s = (span - (narches - 1) as f64 * pier) / narches as f64;
    let r = s / 2.0;
    let spring = deck - 0.4 - r;
    let f = Face {
        o: [bw / 2.0, ya],
        u: [0.0, 1.0],
        len: span,
    };
    // `out` of this face is −x: the bridge from x = bw/2 back to −bw/2.
    let dd = (0.0, bw);
    for k in 0..narches {
        let s0 = k as f64 * (s + pier);
        if k > 0 {
            f.bx(b, stone, (s0 - pier, s0), (0.0, deck), (0.0, bw));
        }
        let seg = 10;
        for i in 0..seg {
            let a0 = PI * i as f64 / seg as f64;
            let a1 = PI * (i + 1) as f64 / seg as f64;
            let p0 = (s0 + r - r * a0.cos(), spring + r * a0.sin());
            let p1 = (s0 + r - r * a1.cos(), spring + r * a1.sin());
            f.prism(b, stone, &[p0, p1, (p1.0, deck), (p0.0, deck)], dd);
        }
        // Below the springing, the face down to the bed.
        if spring > 0.0 {
            f.bx(b, stone, (s0 - 0.01, s0 + 0.01), (0.0, spring), (0.0, bw));
            f.bx(
                b,
                stone,
                (s0 + s - 0.01, s0 + s + 0.01),
                (0.0, spring),
                (0.0, bw),
            );
        }
    }
    // The paving of the deck and its parapets.
    let top = Face {
        o: [bw / 2.0, ya - 1.0],
        u: [0.0, 1.0],
        len: span + 2.0,
    };
    top.bx(
        b,
        "s_granite",
        (0.0, top.len),
        (deck, deck + 0.04),
        (0.0, bw),
    );
    for (d0, d1) in [(0.0, 0.45), (bw - 0.45, bw)] {
        top.bx(b, stone, (0.0, top.len), (deck, deck + 1.0), (d0, d1));
        top.bx(
            b,
            l.trim,
            (-0.05, top.len + 0.05),
            (deck + 1.0, deck + 1.12),
            (d0 - 0.05, d1 + 0.05),
        );
    }
    if !c.light {
        for (x, y) in [(-bw / 2.0 - 1.0, ya - 2.0), (bw / 2.0 + 1.0, yb + 2.0)] {
            lamp_post(b, [x, y], deck, 3.4);
        }
        for (k, (x, y)) in [
            (-w / 2.0 + 3.0, ya - 3.0),
            (w / 2.0 - 3.0, yb + 3.5),
            (w / 2.0 - 4.0, ya - 4.0),
        ]
        .iter()
        .enumerate()
        {
            tree(b, [*x, *y], deck, 7.0, c.seed + k as u64);
        }
    }
    l
}

/// The barracks of the Guardia Civil: a house round a courtyard, its name
/// over the door, the flag and the boundary wall.
pub(super) fn guard_house(b: &mut Builder, c: &Ctx) -> Look {
    let (w, d) = (c.w, c.d);
    let sk = civic(c.sk, Typology::Courtyard);
    let plan = Plan::rect(w, d, [Side::Back, Side::Free, Side::Street, Side::Free]);
    let l = super::towers::courtyard_on(b, &plan, &sk, c.floors.max(2), c.seed, c.light);
    let f = Face::sides(w, d)[0];
    let zs = (sk.storey.0 - 0.2).max(3.0);
    sign(
        b,
        &f,
        (w / 2.0 - 3.5, w / 2.0 + 3.5),
        (zs - 0.45, zs),
        "w_white",
        "f_green",
        12,
        c.light,
    );
    if !c.light {
        wall_flag(b, &f, w / 2.0 + 4.5, sk.storey.0 + 0.15, "cloth_a");
    }
    l
}
