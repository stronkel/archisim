//! The landmark of each place (tanda 11): the bell tower of the south and
//! of Italy, the windmills of La Mancha and of the Low Countries, the
//! church of two towers (Galicia, Lisbon, colonial America, Central
//! Europe with spires, the East with onions), the mission of California,
//! the dome over its portico (Paris, Washington, San Francisco), the clock
//! tower (London), the temple (Greece), the pagoda (China, Japan, Korea),
//! the mosque and its minaret (the Maghreb), the stave church (the North)
//! and the skyscraper of New York.

use super::super::ensanche::arch_fill;
use super::super::more::plate;
use super::super::*;
use super::civic::{tree, walls_box};
use super::plan::{Plan, Side};
use super::roofs::{self, RoofPlan};
use super::skins::*;
use super::{look, Look};
use crate::build::{Builder, P3};
use std::f64::consts::PI;

/// The landmarks there are.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Kind {
    BellTower(TowerTop),
    Church(TowerTop, usize),
    Mission,
    Windmills,
    DutchMill,
    Dome,
    ClockTower,
    Temple,
    Pagoda,
    Mosque,
    StaveChurch,
    Skyscraper,
}

/// How a tower ends.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum TowerTop {
    /// A pyramid of tiles.
    Pyramid,
    /// A tall spire.
    Spire,
    /// Stages of a lantern up to a small dome (the Giralda).
    Lantern,
    /// A dome on a drum.
    Dome,
    /// An onion.
    Onion,
}

/// The landmark of a place.
pub fn kind_of(sk: &Skin) -> Kind {
    match sk.key {
        "andalucia" => Kind::BellTower(TowerTop::Lantern),
        "italia" | "mediterraneo" | "valencia" | "baleares" => Kind::BellTower(TowerTop::Pyramid),
        "castilla" | "extremadura" | "colgadas" | "montana" | "algarve" => {
            Kind::BellTower(TowerTop::Spire)
        }
        "norte" | "lisboa" | "latam" => Kind::Church(TowerTop::Dome, 2),
        "centroeuropa" => Kind::Church(TowerTop::Spire, 2),
        "este" => Kind::Church(TowerTop::Onion, 2),
        "california" => Kind::Mission,
        "mancha" => Kind::Windmills,
        "paises_bajos" => Kind::DutchMill,
        "paris" | "eeuu" | "san_francisco" => Kind::Dome,
        "reino_unido" | "tropico" => Kind::ClockTower,
        "grecia" => Kind::Temple,
        "china" | "japon" | "corea" => Kind::Pagoda,
        "magreb" => Kind::Mosque,
        "nordico" => Kind::StaveChurch,
        "nueva_york" => Kind::Skyscraper,
        _ => Kind::BellTower(TowerTop::Pyramid),
    }
}

/// The landmark of `sk` over a `w` × `d` plot, its front to y = `d`.
pub fn landmark(b: &mut Builder, sk: &Skin, w: f64, d: f64, seed: u64, light: bool) -> Look {
    let l = look(sk, seed, false);
    match kind_of(sk) {
        Kind::BellTower(top) => {
            church(b, sk, &l, w, d, light, top, 1);
        }
        Kind::Church(top, n) => {
            church(b, sk, &l, w, d, light, top, n);
        }
        Kind::Mission => mission(b, sk, &l, w, d, light),
        Kind::Windmills => {
            for (k, x) in [-w * 0.3, 0.0, w * 0.3].iter().enumerate() {
                mill_round(
                    b,
                    [*x, d * 0.5 + (k as f64 - 1.0).abs() * 2.0],
                    light,
                    seed + k as u64,
                );
            }
        }
        Kind::DutchMill => mill_dutch(b, [0.0, d * 0.5], light),
        Kind::Dome => dome_hall(b, &l, w, d, light),
        Kind::ClockTower => clock_tower(b, &l, w, d, light),
        Kind::Temple => temple(b, w, d, light),
        Kind::Pagoda => pagoda(b, sk, &l, w, d, light, seed),
        Kind::Mosque => mosque(b, &l, w, d, light),
        Kind::StaveChurch => stave(b, w, d, light),
        Kind::Skyscraper => {
            let mut s = *sk;
            s.tower.style = TowerStyle::Deco;
            let tw = w.min(d).min(30.0);
            let plan = Plan::rect(tw, tw, [Side::Rear, Side::Free, Side::Street, Side::Free]);
            return super::towers::tower_on(b, &plan, &s, 45, seed, light);
        }
    }
    l
}

// ----------------------------------------------------------- churches

/// A church: its nave along y with the front to the street, a gable with
/// a rose window and a door, and one tower at the corner or two framing
/// the front.
#[allow(clippy::too_many_arguments)]
fn church(
    b: &mut Builder,
    sk: &Skin,
    l: &Look,
    w: f64,
    d: f64,
    light: bool,
    top: TowerTop,
    towers: usize,
) {
    let brick = matches!(top, TowerTop::Lantern) || sk.key == "italia";
    let stone = if brick { "k_red" } else { l.trim };
    let walls = if brick { l.wall } else { stone };
    let ts = (w * 0.28).clamp(5.0, 8.0);
    let nw = if towers == 2 {
        (w - 2.0 * ts).max(8.0)
    } else {
        (w - ts - 1.0).clamp(8.0, 14.0)
    };
    let nx = if towers == 2 {
        (-nw / 2.0, nw / 2.0)
    } else {
        (-w / 2.0 + ts + 0.01, -w / 2.0 + ts + nw)
    };
    let (y0, y1) = (1.0, d - 1.0);
    let nh = (nw * 0.9).clamp(9.0, 14.0);
    // The nave and its tall windows, the door in the front.
    let side_holes: Vec<Hole> = {
        let n = ((y1 - y0) / 5.0).floor().max(2.0) as usize;
        (0..n)
            .map(|i| {
                let c = (y1 - y0) * (i as f64 + 0.5) / n as f64;
                Hole {
                    s: (c - 0.7, c + 0.7),
                    z: (nh * 0.4, nh * 0.4 + 3.2),
                }
            })
            .collect()
    };
    let door = Hole {
        s: (nw / 2.0 - 1.4, nw / 2.0 + 1.4),
        z: (0.0, 4.2),
    };
    let fs = walls_box(
        b,
        nx,
        (y0, y1),
        (0.0, nh),
        walls,
        [&[door], &side_holes, &[], &side_holes],
        true,
    );
    for k in [1, 3] {
        for h in &side_holes {
            arch_fill(b, &fs[k], walls, h.s, h.z.1 - 0.7, (-0.3, 0.0));
            fs[k].bx(b, "glass", h.s, h.z, (-0.2, -0.16));
        }
    }
    arch_fill(b, &fs[0], walls, door.s, door.z.1 - 1.4, (-0.3, 0.0));
    fs[0].bx(b, l.door, door.s, door.z, (-0.22, -0.16));
    // The roof along the nave and the front's gable with its rose window.
    roofs::roof(
        b,
        &RoofPlan {
            x: nx,
            y: (y0, y1),
            z: nh,
            form: Roof::GableToStreet,
            cover: if sk.cover == Cover::Seam {
                Cover::Courses
            } else {
                sk.cover
            },
            pitch: if sk.roof == Roof::Flat {
                0.45
            } else {
                sk.pitch.clamp(0.4, 1.0)
            },
            eaves: 0.3,
            party: false,
            dormers: Vec::new(),
            gable: Gable::Plain,
            seed: 1,
            extras: 0,
            top: Top::None,
            block: false,
            light,
        },
        l,
    );
    let f = fs[0];
    let rose = f.at(nw / 2.0, nh + 0.8, 0.05);
    let o = f.out();
    b.cone(
        "glass",
        rose,
        [rose[0] + o[0] * 0.04, rose[1] + o[1] * 0.04, rose[2]],
        1.3,
        1.3,
        16,
    );
    b.cone(
        stone,
        [rose[0] - o[0] * 0.45, rose[1] - o[1] * 0.45, rose[2]],
        rose,
        1.5,
        1.5,
        16,
    );
    // The towers.
    let spots: Vec<f64> = if towers == 2 {
        vec![-w / 2.0, w / 2.0 - ts]
    } else {
        vec![-w / 2.0]
    };
    let th = if towers == 2 {
        nh * 2.2
    } else {
        (nh * 2.6).max(28.0)
    };
    for x in spots {
        bell_tower(
            b,
            (x, x + ts),
            (y1 - ts, y1),
            th,
            stone,
            walls,
            top,
            l,
            light,
        );
    }
}

/// A bell tower: its shaft with slits, the belfry with its arches and
/// bells, a cornice and its top.
#[allow(clippy::too_many_arguments)]
fn bell_tower(
    b: &mut Builder,
    x: (f64, f64),
    y: (f64, f64),
    h: f64,
    stone: &'static str,
    shaft: &'static str,
    top: TowerTop,
    l: &Look,
    light: bool,
) {
    let s = x.1 - x.0;
    let bz = h - s * 1.2;
    let slits: Vec<Hole> = (1..((bz / 6.0) as usize))
        .map(|k| Hole {
            s: (s / 2.0 - 0.25, s / 2.0 + 0.25),
            z: (6.0 * k as f64, 6.0 * k as f64 + 1.4),
        })
        .collect();
    let fs = walls_box(
        b,
        x,
        y,
        (0.0, bz),
        shaft,
        [&slits, &slits, &slits, &slits],
        true,
    );
    for f in &fs {
        for h in &slits {
            f.bx(b, "interior", h.s, h.z, (-0.25, -0.2));
        }
        // The band under the belfry.
        f.bx(b, stone, (-0.15, s + 0.15), (bz - 0.4, bz), (-0.3, 0.12));
    }
    // The belfry: an arch on each face with its bell.
    let ah = s * 1.2 - 0.5;
    let aw = s * 0.45;
    let arch = Hole {
        s: (s / 2.0 - aw / 2.0, s / 2.0 + aw / 2.0),
        z: (bz + 0.3, bz + ah - 0.3),
    };
    let bf = walls_box(
        b,
        x,
        y,
        (bz, bz + ah),
        stone,
        [&[arch], &[arch], &[arch], &[arch]],
        false,
    );
    for f in &bf {
        arch_fill(b, f, stone, arch.s, arch.z.1 - aw / 2.0, (-0.3, 0.0));
    }
    let c = [(x.0 + x.1) / 2.0, (y.0 + y.1) / 2.0];
    b.rbox(
        stone,
        [x.0 + 0.3, y.0 + 0.3, bz - 0.1],
        [x.1 - 0.3, y.1 - 0.3, bz + 0.3],
        0.0,
    );
    if !light {
        b.cyl(
            "b_dark",
            [c[0] - s / 2.0 + 0.3, c[1], bz + ah - 0.15],
            [c[0] + s / 2.0 - 0.3, c[1], bz + ah - 0.15],
            0.08,
        );
        b.cone(
            "brass",
            [c[0], c[1], bz + ah - 0.155],
            [c[0], c[1], bz + ah - 1.3],
            0.15,
            0.55,
            12,
        );
    }
    let zt = bz + ah;
    b.rbox(
        stone,
        [x.0 - 0.2, y.0 - 0.2, zt],
        [x.1 + 0.2, y.1 + 0.2, zt + 0.5],
        0.0,
    );
    let zt = zt + 0.5;
    match top {
        TowerTop::Pyramid => b.cone(
            "roof",
            [c[0], c[1], zt],
            [c[0], c[1], zt + s * 0.6],
            s * 0.72,
            0.05,
            4,
        ),
        TowerTop::Spire => {
            b.cone(
                stone,
                [c[0], c[1], zt],
                [c[0], c[1], zt + s * 2.4],
                s * 0.62,
                0.05,
                8,
            );
            if !light {
                for (dx, dy) in [(-1.0, -1.0), (1.0, -1.0), (1.0, 1.0), (-1.0, 1.0)] {
                    let p = [c[0] + dx * (s / 2.0 - 0.1), c[1] + dy * (s / 2.0 - 0.1)];
                    b.cone(
                        stone,
                        [p[0], p[1], zt],
                        [p[0], p[1], zt + 1.8],
                        0.25,
                        0.02,
                        4,
                    );
                }
            }
        }
        TowerTop::Lantern => {
            // Two stages, each smaller, a small dome and its figure.
            let mut z = zt;
            let mut half = s * 0.36;
            for stage in 0..2 {
                let hh = s * (0.75 - 0.15 * stage as f64);
                let hole = Hole {
                    s: (half - 0.5, half + 0.5),
                    z: (z + 0.5, z + hh - 0.6),
                };
                let sfs = walls_box(
                    b,
                    (c[0] - half, c[0] + half),
                    (c[1] - half, c[1] + half),
                    (z, z + hh),
                    l.wall,
                    [&[hole], &[hole], &[hole], &[hole]],
                    false,
                );
                for f in &sfs {
                    arch_fill(b, f, l.wall, hole.s, hole.z.1 - 0.5, (-0.3, 0.0));
                }
                b.rbox(
                    stone,
                    [c[0] - half - 0.12, c[1] - half - 0.12, z + hh],
                    [c[0] + half + 0.12, c[1] + half + 0.12, z + hh + 0.3],
                    0.0,
                );
                z += hh + 0.3;
                half *= 0.75;
            }
            dome(b, [c[0], c[1]], z, half * 1.05, "brass", light);
            b.cyl(
                "brass",
                [c[0], c[1], z + half],
                [c[0], c[1], z + half + 1.6],
                0.06,
            );
            b.ellipsoid("brass", [c[0], c[1], z + half + 1.8], [0.2, 0.2, 0.35]);
        }
        TowerTop::Dome => {
            let r = s * 0.4;
            b.cone(
                stone,
                [c[0], c[1], zt],
                [c[0], c[1], zt + 1.2],
                r + 0.2,
                r + 0.2,
                12,
            );
            dome(b, c, zt + 1.2, r, "roof", light);
            b.cone(
                stone,
                [c[0], c[1], zt + 1.2 + r * 0.95],
                [c[0], c[1], zt + 2.6 + r],
                0.35,
                0.2,
                8,
            );
            b.cyl(
                "iron",
                [c[0], c[1], zt + 2.6 + r],
                [c[0], c[1], zt + 3.6 + r],
                0.05,
            );
        }
        TowerTop::Onion => {
            let r = s * 0.42;
            b.cone(
                stone,
                [c[0], c[1], zt],
                [c[0], c[1], zt + 1.5],
                r * 0.8,
                r * 0.8,
                12,
            );
            onion(b, c, zt + 1.5, r, "brass", light);
        }
    }
}

/// A dome over a circle of radius `r` standing at `z`, as stacked rings.
pub(super) fn dome(b: &mut Builder, c: [f64; 2], z: f64, r: f64, role: &'static str, light: bool) {
    let n = if light { 4 } else { 8 };
    for k in 0..n {
        let a0 = PI / 2.0 * k as f64 / n as f64;
        let a1 = PI / 2.0 * (k + 1) as f64 / n as f64;
        b.cone(
            role,
            [c[0], c[1], z + r * a0.sin()],
            [c[0], c[1], z + r * a1.sin()],
            r * a0.cos(),
            (r * a1.cos()).max(0.02),
            16,
        );
    }
}

/// An onion dome.
fn onion(b: &mut Builder, c: [f64; 2], z: f64, r: f64, role: &'static str, light: bool) {
    let prof: &[(f64, f64)] = &[
        (0.0, 0.7),
        (0.25, 1.0),
        (0.6, 0.95),
        (0.95, 0.6),
        (1.3, 0.25),
        (1.6, 0.05),
    ];
    let _ = light;
    for w in prof.windows(2) {
        b.cone(
            role,
            [c[0], c[1], z + w[0].0 * r],
            [c[0], c[1], z + w[1].0 * r],
            w[0].1 * r,
            w[1].1 * r,
            12,
        );
    }
    b.cyl(
        "brass",
        [c[0], c[1], z + 1.6 * r],
        [c[0], c[1], z + 1.6 * r + 1.4],
        0.05,
    );
    b.rbox(
        "brass",
        [c[0] - 0.35, c[1] - 0.03, z + 1.6 * r + 0.9],
        [c[0] + 0.35, c[1] + 0.03, z + 1.6 * r + 0.98],
        0.0,
    );
}

/// The mission of California: a white front with its bell gable and
/// three bells, the nave behind and an arcade along it.
fn mission(b: &mut Builder, sk: &Skin, l: &Look, w: f64, d: f64, light: bool) {
    let nw = (w * 0.45).clamp(8.0, 12.0);
    let nx = (-w / 2.0, -w / 2.0 + nw);
    let (y0, y1) = (1.0, d - 1.0);
    let nh = 8.5;
    let door = Hole {
        s: (nw / 2.0 - 1.3, nw / 2.0 + 1.3),
        z: (0.0, 3.8),
    };
    let wh = "w_white";
    let fs = walls_box(
        b,
        nx,
        (y0, y1),
        (0.0, nh),
        wh,
        [&[door], &[], &[], &[]],
        true,
    );
    arch_fill(b, &fs[0], wh, door.s, door.z.1 - 1.3, (-0.3, 0.0));
    fs[0].bx(b, l.door, door.s, door.z, (-0.22, -0.16));
    roofs::roof(
        b,
        &RoofPlan {
            x: nx,
            y: (y0, y1),
            z: nh,
            form: Roof::GableToStreet,
            cover: Cover::Curved,
            pitch: 0.4,
            eaves: 0.3,
            party: false,
            dormers: Vec::new(),
            gable: Gable::Plain,
            seed: 1,
            extras: 0,
            top: Top::None,
            block: false,
            light,
        },
        &Look {
            roof: "r_curved",
            ..*l
        },
    );
    // The bell wall rising over the front, with three arches.
    let f = fs[0];
    let gw = nw + 2.0;
    let gz = nh + nw * 0.2;
    let gf = f.moved(0.02);
    let pts = [
        (-1.0, nh - 0.5),
        (nw + 1.0, nh - 0.5),
        (nw + 1.0, gz + 2.8),
        (nw / 2.0 + 1.2, gz + 4.2),
        (nw / 2.0 - 1.2, gz + 4.2),
        (-1.0, gz + 2.8),
    ];
    gf.prism(b, wh, &pts, (-0.6, 0.0));
    for (k, s) in [nw / 2.0 - 2.4, nw / 2.0, nw / 2.0 + 2.4]
        .iter()
        .enumerate()
    {
        let z = gz + if k == 1 { 1.4 } else { 0.4 };
        let bell = gf.at(*s, z + 0.9, 0.05);
        // A niche (dark) and the bell in it.
        gf.bx(b, "interior", (s - 0.5, s + 0.5), (z, z + 1.6), (0.0, 0.03));
        if !light {
            b.cone(
                "brass",
                [bell[0], bell[1], bell[2] + 0.3],
                bell,
                0.08,
                0.32,
                10,
            );
        }
    }
    let _ = gw;
    gf.bx(
        b,
        "iron",
        (nw / 2.0 - 0.04, nw / 2.0 + 0.04),
        (gz + 4.2, gz + 5.1),
        (-0.32, -0.25),
    );
    gf.bx(
        b,
        "iron",
        (nw / 2.0 - 0.3, nw / 2.0 + 0.3),
        (gz + 4.75, gz + 4.83),
        (-0.32, -0.25),
    );
    // The arcade along the nave.
    let ax = nx.1 + 3.0;
    let n = ((y1 - y0) / 3.0).floor() as usize;
    for i in 0..=n {
        let y = y0 + (y1 - y0) * i as f64 / n as f64;
        b.rbox(
            wh,
            [ax - 0.35, y - 0.35, 0.0],
            [ax + 0.35, y + 0.35, 3.0],
            0.0,
        );
    }
    b.rbox(
        wh,
        [ax - 0.4, y0 - 0.35, 3.0],
        [ax + 0.4, y1 + 0.35, 3.6],
        0.0,
    );
    let pts: [P3; 4] = [
        [nx.1, y0 - 0.5, 4.6],
        [nx.1, y1 + 0.5, 4.6],
        [ax + 0.8, y1 + 0.5, 3.55],
        [ax + 0.8, y0 - 0.5, 3.55],
    ];
    plate(b, "r_curved", &pts, 0.12);
    let _ = sk;
}

// ------------------------------------------------------------ windmills

/// A windmill of La Mancha: a white round body, its dark conical cap and
/// four sails of lattice.
fn mill_round(b: &mut Builder, c: [f64; 2], light: bool, seed: u64) {
    let (r, h) = (2.6, 8.0);
    b.cone(
        "w_white",
        [c[0], c[1], 0.0],
        [c[0], c[1], h],
        r,
        r * 0.9,
        20,
    );
    b.cone(
        "r_black",
        [c[0], c[1], h],
        [c[0], c[1], h + 2.6],
        r * 1.0,
        0.1,
        16,
    );
    // The door and two small windows.
    let f = Face {
        o: [c[0] - 0.5, c[1] + r * 0.97],
        u: [1.0, 0.0],
        len: 1.0,
    };
    f.bx(b, "f_blue", (0.0, 1.0), (0.0, 2.0), (-0.1, 0.05));
    f.bx(b, "interior", (0.3, 0.7), (4.0, 4.7), (-0.1, 0.05));
    sails(
        b,
        [c[0], c[1] + r * 0.95],
        h - 0.3,
        6.5,
        light,
        unit(seed, 1) * 0.6,
    );
}

/// A windmill of the Low Countries: a brick base, an octagonal body of
/// thatch, its stage round, the cap and the sails.
fn mill_dutch(b: &mut Builder, c: [f64; 2], light: bool) {
    b.cone("k_red", [c[0], c[1], 0.0], [c[0], c[1], 5.0], 4.4, 4.0, 8);
    b.cone(
        "r_thatch",
        [c[0], c[1], 5.0],
        [c[0], c[1], 16.0],
        3.9,
        2.4,
        8,
    );
    // The stage at the top of the brick, with its railing.
    b.cone("b_cedar", [c[0], c[1], 4.9], [c[0], c[1], 5.1], 6.5, 6.5, 8);
    if !light {
        for k in 0..16 {
            let a = 2.0 * PI * k as f64 / 16.0;
            let p = [c[0] + 6.3 * a.cos(), c[1] + 6.3 * a.sin()];
            b.cyl("b_cedar", [p[0], p[1], 5.1], [p[0], p[1], 6.1], 0.04);
            let pb = [c[0] + 6.3 * (a - 0.4).cos(), c[1] + 6.3 * (a - 0.4).sin()];
            b.cyl(
                "b_cedar",
                [p[0], p[1], 0.0],
                [pb[0] * 0.4 + c[0] * 0.6, pb[1] * 0.4 + c[1] * 0.6, 4.95],
                0.05,
            );
        }
        b.cone(
            "b_cedar",
            [c[0], c[1], 6.05],
            [c[0], c[1], 6.15],
            6.4,
            6.4,
            16,
        );
    }
    b.rbox(
        "b_dark",
        [c[0] - 2.4, c[1] - 2.6, 16.0],
        [c[0] + 2.4, c[1] + 2.6, 17.4],
        0.0,
    );
    let pts: [P3; 4] = [
        [c[0] - 2.6, c[1] - 2.8, 17.4],
        [c[0] + 2.6, c[1] - 2.8, 17.4],
        [c[0] + 2.6, c[1] + 2.8, 17.4],
        [c[0] - 2.6, c[1] + 2.8, 17.4],
    ];
    plate(b, "r_thatch", &pts, 0.2);
    b.cone(
        "r_thatch",
        [c[0], c[1], 17.6],
        [c[0], c[1], 19.0],
        2.4,
        0.2,
        4,
    );
    sails(b, [c[0], c[1] + 2.7], 16.7, 11.0, light, 0.3);
}

/// Four sails turning round an axle at `c` (on the front, y up), `len`
/// long.
fn sails(b: &mut Builder, c: [f64; 2], z: f64, len: f64, light: bool, turn: f64) {
    b.cyl("b_dark", [c[0], c[1] - 0.6, z], [c[0], c[1] + 0.4, z], 0.22);
    for k in 0..4 {
        let a = turn + PI / 2.0 * k as f64;
        let (s, co) = a.sin_cos();
        let tip = [c[0] + co * len, c[1] + 0.35, z + s * len];
        b.cone("b_dark", [c[0], c[1] + 0.35, z], tip, 0.12, 0.06, 6);
        // The lattice of the sail beside its stock.
        let n = if light { 1 } else { 7 };
        let (nx, nz) = (-s, co);
        let wide = len * 0.2;
        for j in 0..=n {
            let t = 0.25 + 0.75 * j as f64 / n as f64;
            let p = [c[0] + co * len * t, c[1] + 0.35, z + s * len * t];
            let q = [p[0] + nx * wide, p[1], p[2] + nz * wide];
            b.cone("f_white", p, q, 0.03, 0.03, 4);
        }
        let a0 = [
            c[0] + co * len * 0.25 + nx * wide,
            c[1] + 0.35,
            z + s * len * 0.25 + nz * wide,
        ];
        let a1 = [tip[0] + nx * wide, tip[1], tip[2] + nz * wide];
        b.cone("f_white", a0, a1, 0.035, 0.035, 4);
        if light {
            let pts: [P3; 4] = [
                [c[0] + co * len * 0.25, c[1] + 0.3, z + s * len * 0.25],
                tip,
                [a1[0], a1[1] - 0.05, a1[2]],
                [a0[0], a0[1] - 0.05, a0[2]],
            ];
            plate(b, "f_white", &pts, 0.02);
        }
    }
}

// ---------------------------------------------------------- monuments

/// A hall with a dome on a drum and a portico of columns with its
/// pediment (the Panthéon, a capitol, a city hall).
fn dome_hall(b: &mut Builder, l: &Look, w: f64, d: f64, light: bool) {
    let stone = if l.wall.starts_with("w_") {
        "s_ashlar"
    } else {
        l.trim
    };
    let (x0, x1) = (-w / 2.0 + 1.0, w / 2.0 - 1.0);
    let pd = 5.0;
    let (y0, y1) = (1.0, d - pd - 1.0);
    let h = 12.0;
    let win = |len: f64| -> Vec<Hole> {
        let n = (len / 4.0).floor().max(1.0) as usize;
        (0..n)
            .map(|i| {
                let c = len * (i as f64 + 0.5) / n as f64;
                Hole {
                    s: (c - 0.7, c + 0.7),
                    z: (5.0, 8.0),
                }
            })
            .collect()
    };
    let (hw, hd) = (win(x1 - x0), win(y1 - y0));
    let door = Hole {
        s: ((x1 - x0) / 2.0 - 1.5, (x1 - x0) / 2.0 + 1.5),
        z: (1.2, 5.6),
    };
    let fs = walls_box(
        b,
        (x0, x1),
        (y0, y1),
        (0.0, h),
        stone,
        [&[door], &hd, &hw, &hd],
        true,
    );
    for (k, hs) in [(1, &hd), (2, &hw), (3, &hd)] {
        for hh in hs.iter() {
            fs[k].bx(b, "glass", hh.s, hh.z, (-0.2, -0.16));
            fs[k].bx(
                b,
                stone,
                (hh.s.0 - 0.2, hh.s.1 + 0.2),
                (hh.z.1, hh.z.1 + 0.3),
                (0.0, 0.1),
            );
        }
    }
    fs[0].bx(b, l.door, door.s, door.z, (-0.22, -0.16));
    b.rbox(stone, [x0, y0, 0.0], [x1, y1, 1.2], 0.0);
    // The flat roof with its balustrade.
    b.rbox(
        "r_gravel",
        [x0 + 0.3, y0 + 0.3, h - 0.3],
        [x1 - 0.3, y1 - 0.3, h + 0.05],
        0.0,
    );
    for f in roofs::faces((x0, x1), (y0, y1)) {
        f.bx(b, stone, (-0.2, f.len + 0.2), (h, h + 0.6), (-0.3, 0.15));
        f.bx(b, stone, (0.0, f.len), (h + 0.6, h + 1.3), (-0.3, 0.0));
    }
    // The portico: steps, columns, entablature and pediment.
    let pw = ((x1 - x0) * 0.55).max(10.0).min(x1 - x0);
    let px = (-pw / 2.0, pw / 2.0);
    let (py0, py1) = (y1, y1 + pd);
    for k in 0..4 {
        let o = 0.4 * (3 - k) as f64;
        b.rbox(
            stone,
            [px.0 - o, py0, 0.3 * k as f64],
            [px.1 + o, py1 + o, 0.3 * (k + 1) as f64],
            0.0,
        );
    }
    let zc = 1.2;
    let ch = h - 1.2 - zc;
    let cols = 6;
    for i in 0..cols {
        let x = px.0 + 0.7 + (pw - 1.4) * i as f64 / (cols - 1) as f64;
        let y = py1 - 0.8;
        b.cone(
            stone,
            [x, y, zc],
            [x, y, zc + ch],
            0.5,
            0.42,
            if light { 8 } else { 16 },
        );
        b.rbox(
            stone,
            [x - 0.6, y - 0.6, zc + ch],
            [x + 0.6, y + 0.6, zc + ch + 0.25],
            0.0,
        );
    }
    let ze = zc + ch + 0.25;
    b.rbox(
        stone,
        [px.0 - 0.2, py0, ze],
        [px.1 + 0.2, py1 - 0.1, ze + 1.0],
        0.0,
    );
    let pf = Face {
        o: [px.0 - 0.2, py1 - 0.1],
        u: [1.0, 0.0],
        len: pw + 0.4,
    };
    let pts = [
        (0.0, ze + 1.0),
        (pf.len, ze + 1.0),
        (pf.len / 2.0, ze + 1.0 + pw * 0.18),
    ];
    pf.prism(b, stone, &pts, (-(pd - 0.1), 0.0));
    let rpts: [P3; 4] = [
        [px.0 - 0.3, py0, ze + 1.0],
        [px.0 - 0.3, py1, ze + 1.0],
        [0.0, py1, ze + 1.05 + pw * 0.18],
        [0.0, py0, ze + 1.05 + pw * 0.18],
    ];
    plate(b, "r_grey", &rpts, 0.1);
    let rpts: [P3; 4] = [
        [0.0, py0, ze + 1.05 + pw * 0.18],
        [0.0, py1, ze + 1.05 + pw * 0.18],
        [px.1 + 0.3, py1, ze + 1.0],
        [px.1 + 0.3, py0, ze + 1.0],
    ];
    plate(b, "r_grey", &rpts, 0.1);
    // The drum with its columns and the dome with its lantern.
    let c = [0.0, (y0 + y1) / 2.0];
    let r = ((x1 - x0).min(y1 - y0) * 0.3).clamp(4.0, 9.0);
    b.cone(stone, [c[0], c[1], h], [c[0], c[1], h + r * 0.9], r, r, 24);
    if !light {
        for k in 0..16 {
            let a = 2.0 * PI * k as f64 / 16.0;
            let p = [c[0] + (r + 0.35) * a.cos(), c[1] + (r + 0.35) * a.sin()];
            b.cyl(stone, [p[0], p[1], h], [p[0], p[1], h + r * 0.9], 0.25);
        }
    }
    b.cone(
        stone,
        [c[0], c[1], h + r * 0.9],
        [c[0], c[1], h + r * 0.9 + 0.5],
        r + 0.7,
        r + 0.7,
        24,
    );
    dome(b, c, h + r * 0.9 + 0.5, r, "r_grey", light);
    let zl = h + r * 1.85 + 0.5;
    b.cone(
        stone,
        [c[0], c[1], zl - 0.3],
        [c[0], c[1], zl + 2.0],
        r * 0.2,
        r * 0.2,
        12,
    );
    dome(b, c, zl + 2.0, r * 0.22, "r_grey", light);
    b.cyl(
        "brass",
        [c[0], c[1], zl + 2.0 + r * 0.2],
        [c[0], c[1], zl + 3.4 + r * 0.2],
        0.08,
    );
    // The forecourt.
    b.rbox("paving", [-w / 2.0, py1, 0.0], [w / 2.0, d, 0.02], 0.0);
}

/// A clock tower over a hall (the parliament's): the shaft with its
/// panels, the clock on each face, the belfry and the spire.
fn clock_tower(b: &mut Builder, l: &Look, w: f64, d: f64, light: bool) {
    let stone = "s_ashlar";
    // The hall.
    let (hx, hy) = ((-w / 2.0 + 1.0, w / 2.0 - 1.0), (1.0, d * 0.6));
    let n = ((hx.1 - hx.0) / 2.5) as usize;
    let holes: Vec<Hole> = (0..n)
        .map(|i| {
            let c = (hx.1 - hx.0) * (i as f64 + 0.5) / n as f64;
            Hole {
                s: (c - 0.6, c + 0.6),
                z: (2.0, 5.5),
            }
        })
        .collect();
    let fs = walls_box(
        b,
        hx,
        hy,
        (0.0, 9.0),
        stone,
        [&holes, &[], &holes, &[]],
        true,
    );
    for k in [0, 2] {
        for h in &holes {
            arch_fill(b, &fs[k], stone, h.s, h.z.1 - 0.6, (-0.3, 0.0));
            fs[k].bx(b, "glass", h.s, h.z, (-0.2, -0.16));
        }
    }
    roofs::roof(
        b,
        &RoofPlan {
            x: hx,
            y: hy,
            z: 9.0,
            form: Roof::Gable,
            cover: Cover::Courses,
            pitch: 1.0,
            eaves: 0.2,
            party: false,
            dormers: Vec::new(),
            gable: Gable::Plain,
            seed: 1,
            extras: 0,
            top: Top::None,
            block: false,
            light,
        },
        &Look {
            roof: "r_slate",
            ..*l
        },
    );
    // The tower at the front corner.
    let s = 8.0;
    let tx = (hx.1 - s, hx.1);
    let ty = (hy.1 - 0.01, hy.1 + s);
    let sh = 38.0;
    let panels: Vec<Hole> = (0..3)
        .map(|k| Hole {
            s: (1.0 + 2.2 * k as f64, 2.4 + 2.2 * k as f64),
            z: (6.0, sh - 4.0),
        })
        .collect();
    let tf = walls_box(
        b,
        tx,
        ty,
        (0.0, sh),
        stone,
        [&panels, &panels, &panels, &panels],
        true,
    );
    b.rbox(
        stone,
        [tx.0 - 0.3, ty.0 - 0.3, sh],
        [tx.1 + 0.3, ty.1 + 0.3, sh + 6.0],
        0.0,
    );
    for f in &tf {
        for h in &panels {
            f.bx(b, "s_brown", h.s, h.z, (-0.3, -0.12));
            if !light {
                let m = ((h.z.1 - h.z.0) / 3.0) as usize;
                for k in 1..m {
                    let z = h.z.0 + 3.0 * k as f64;
                    f.bx(b, stone, h.s, (z - 0.12, z + 0.12), (-0.3, -0.05));
                }
            }
        }
        // The clock.
        let p = f.at(s / 2.0, sh + 3.0, 0.0);
        let o = f.out();
        let cpos = [p[0] + o[0] * 0.3, p[1] + o[1] * 0.3, p[2]];
        b.cone(
            "brass",
            cpos,
            [cpos[0] + o[0] * 0.08, cpos[1] + o[1] * 0.08, cpos[2]],
            3.0,
            3.0,
            20,
        );
        b.cone(
            "f_white",
            [cpos[0] + o[0] * 0.08, cpos[1] + o[1] * 0.08, cpos[2]],
            [cpos[0] + o[0] * 0.12, cpos[1] + o[1] * 0.12, cpos[2]],
            2.6,
            2.6,
            20,
        );
        if !light {
            let q = [cpos[0] + o[0] * 0.14, cpos[1] + o[1] * 0.14, cpos[2]];
            b.obox(
                "f_black",
                [q[0], q[1], q[2] + 0.9],
                [o[0] * 0.02, o[1] * 0.02, 0.0],
                [f.u[0] * 0.06, f.u[1] * 0.06, 0.0],
                [0.0, 0.0, 0.9],
            );
            b.obox(
                "f_black",
                [q[0] + f.u[0] * 0.6, q[1] + f.u[1] * 0.6, q[2]],
                [o[0] * 0.02, o[1] * 0.02, 0.0],
                [f.u[0] * 0.6, f.u[1] * 0.6, 0.0],
                [0.0, 0.0, 0.05],
            );
        }
    }
    // The belfry and the spire with its pinnacles.
    let c = [(tx.0 + tx.1) / 2.0, (ty.0 + ty.1) / 2.0];
    let zb = sh + 6.0;
    let arch = Hole {
        s: (1.5, s - 1.5),
        z: (zb + 0.5, zb + 4.0),
    };
    let bf = walls_box(
        b,
        (tx.0 + 0.5, tx.1 - 0.5),
        (ty.0 + 0.5, ty.1 - 0.5),
        (zb, zb + 5.0),
        stone,
        [&[arch], &[arch], &[arch], &[arch]],
        false,
    );
    let _ = bf;
    b.rbox(
        "interior",
        [tx.0 + 1.0, ty.0 + 1.0, zb],
        [tx.1 - 1.0, ty.1 - 1.0, zb + 5.0],
        0.0,
    );
    b.cone(
        "r_slate",
        [c[0], c[1], zb + 5.0],
        [c[0], c[1], zb + 18.0],
        s * 0.58,
        0.08,
        4,
    );
    if !light {
        for (dx, dy) in [(-1.0, -1.0), (1.0, -1.0), (1.0, 1.0), (-1.0, 1.0)] {
            let p = [c[0] + dx * (s / 2.0 - 0.6), c[1] + dy * (s / 2.0 - 0.6)];
            b.cone(
                stone,
                [p[0], p[1], zb + 5.0],
                [p[0], p[1], zb + 8.5],
                0.4,
                0.03,
                4,
            );
        }
        b.cyl(
            "brass",
            [c[0], c[1], zb + 18.0],
            [c[0], c[1], zb + 20.0],
            0.06,
        );
    }
}

/// A Greek temple: its stepped base, the peristyle, the entablature and
/// the pediments with the roof.
fn temple(b: &mut Builder, w: f64, d: f64, light: bool) {
    let marble = "s_lime";
    let (tw, td) = ((w - 2.0).min(d * 0.48), d - 2.0);
    let (x0, x1, y0, y1) = (-tw / 2.0, tw / 2.0, 1.0, 1.0 + td);
    for k in 0..3 {
        let o = 0.5 * (2 - k) as f64;
        b.rbox(
            marble,
            [x0 - o, y0 - o, 0.4 * k as f64],
            [x1 + o, y1 + o, 0.4 * (k + 1) as f64],
            0.0,
        );
    }
    let zc = 1.2;
    let ch = tw * 0.55;
    let cols_x = 6;
    let cols_y = 13;
    let r = tw / cols_x as f64 * 0.22;
    let seg = if light { 8 } else { 16 };
    let mut col = |x: f64, y: f64| {
        b.cone(marble, [x, y, zc], [x, y, zc + ch], r, r * 0.8, seg);
        b.rbox(
            marble,
            [x - r * 1.25, y - r * 1.25, zc + ch],
            [x + r * 1.25, y + r * 1.25, zc + ch + r * 0.5],
            0.0,
        );
    };
    for i in 0..cols_x {
        let x = x0 + r * 1.5 + (tw - 3.0 * r) * i as f64 / (cols_x - 1) as f64;
        col(x, y0 + r * 1.5);
        col(x, y1 - r * 1.5);
    }
    for j in 1..cols_y - 1 {
        let y = y0 + r * 1.5 + (td - 3.0 * r) * j as f64 / (cols_y - 1) as f64;
        col(x0 + r * 1.5, y);
        col(x1 - r * 1.5, y);
    }
    // The cella inside.
    b.rbox(
        marble,
        [x0 + r * 4.0, y0 + r * 6.0, zc],
        [x1 - r * 4.0, y1 - r * 6.0, zc + ch],
        0.0,
    );
    let ze = zc + ch + r * 0.5;
    let eh = ch * 0.28;
    b.rbox(marble, [x0, y0, ze], [x1, y1, ze + eh], 0.0);
    if !light {
        // The triglyphs as a band of darker blocks.
        b.rbox(
            "s_ashlar",
            [x0 - 0.02, y0 - 0.02, ze + eh * 0.45],
            [x1 + 0.02, y1 + 0.02, ze + eh * 0.85],
            0.0,
        );
    }
    let rise = tw * 0.13;
    let z = ze + eh;
    for (y, sgn) in [(y1, 1.0), (y0, -1.0)] {
        let f = if sgn > 0.0 {
            Face {
                o: [x0, y],
                u: [1.0, 0.0],
                len: tw,
            }
        } else {
            Face {
                o: [x1, y],
                u: [-1.0, 0.0],
                len: tw,
            }
        };
        f.prism(
            b,
            marble,
            &[(0.0, z), (tw, z), (tw / 2.0, z + rise)],
            (-0.6, 0.0),
        );
    }
    for (xa, xb) in [(x0 - 0.3, 0.0), (0.0, x1 + 0.3)] {
        let (za, zb) = if xa < 0.0 && xb == 0.0 {
            (z, z + rise)
        } else {
            (z + rise, z)
        };
        let pts: [P3; 4] = [
            [xa, y0 - 0.3, za],
            [xa, y1 + 0.3, za],
            [xb, y1 + 0.3, zb],
            [xb, y0 - 0.3, zb],
        ];
        plate(b, "r_tile", &pts, 0.12);
    }
}

/// A pagoda: a platform and five storeys, each smaller, under deep roofs,
/// and its finial.
fn pagoda(b: &mut Builder, sk: &Skin, l: &Look, w: f64, d: f64, light: bool, seed: u64) {
    let c = [0.0, d / 2.0];
    let base = (w.min(d) * 0.4).clamp(7.0, 12.0);
    b.rbox(
        "s_granite",
        [c[0] - base * 0.75, c[1] - base * 0.75, 0.0],
        [c[0] + base * 0.75, c[1] + base * 0.75, 1.0],
        0.0,
    );
    let tiers = 5;
    let wall = if sk.key == "japon" { "b_dark" } else { "f_red" };
    let mut z = 1.0;
    let mut half = base / 2.0;
    for t in 0..tiers {
        let h = if t == 0 { 3.6 } else { 2.6 };
        let door = Hole {
            s: (half - 0.6, half + 0.6),
            z: (z + 0.2, z + h - 0.6),
        };
        let fs = walls_box(
            b,
            (c[0] - half, c[0] + half),
            (c[1] - half, c[1] + half),
            (z, z + h),
            wall,
            [&[door], &[door], &[door], &[door]],
            true,
        );
        for f in &fs {
            f.bx(b, "f_white", door.s, door.z, (-0.2, -0.15));
            if !light {
                for s in [0.15, f.len - 0.15] {
                    f.bx(b, wall, (s - 0.12, s + 0.12), (z, z + h), (0.0, 0.12));
                }
            }
        }
        roofs::roof(
            b,
            &RoofPlan {
                x: (c[0] - half, c[0] + half),
                y: (c[1] - half, c[1] + half),
                z: z + h,
                form: Roof::Asian,
                cover: Cover::Curved,
                pitch: 0.5,
                eaves: 1.4 - 0.12 * t as f64,
                party: false,
                dormers: Vec::new(),
                gable: Gable::Plain,
                seed: seed + t as u64,
                extras: 0,
                top: Top::None,
                block: false,
                light,
            },
            &Look {
                roof: if sk.roof_mat.is_empty() {
                    "r_black"
                } else {
                    l.roof
                },
                ..*l
            },
        );
        z += h + half * 0.16 * 0.5 - 0.05;
        half *= 0.84;
    }
    // The finial: rings on a mast.
    b.cyl(
        "brass",
        [c[0], c[1], z - half * 0.3],
        [c[0], c[1], z + 5.0],
        0.12,
    );
    if !light {
        for k in 0..7 {
            let zz = z + 0.6 + 0.5 * k as f64;
            b.cone(
                "brass",
                [c[0], c[1], zz],
                [c[0], c[1], zz + 0.12],
                0.55 - 0.04 * k as f64,
                0.55 - 0.04 * k as f64,
                12,
            );
        }
    }
    b.ellipsoid("brass", [c[0], c[1], z + 5.1], [0.3, 0.3, 0.45]);
    if !light {
        for (k, p) in [[-w * 0.38, d * 0.15], [w * 0.38, d * 0.15]]
            .iter()
            .enumerate()
        {
            tree(b, *p, 0.0, 7.0, seed + k as u64);
        }
    }
}

/// A mosque of the Maghreb: the prayer hall with its battlements and its
/// green tiled roofs, a dome and the square minaret with its panels.
fn mosque(b: &mut Builder, l: &Look, w: f64, d: f64, light: bool) {
    let wall = l.wall;
    let (x0, x1, y0, y1) = (-w / 2.0 + 1.0, w / 2.0 - 1.0, 1.0, d * 0.62);
    let h = 8.0;
    let n = ((x1 - x0) / 3.5) as usize;
    let arches: Vec<Hole> = (0..n)
        .map(|i| {
            let c = (x1 - x0) * (i as f64 + 0.5) / n as f64;
            Hole {
                s: (c - 1.1, c + 1.1),
                z: (0.0, 4.6),
            }
        })
        .collect();
    let fs = walls_box(
        b,
        (x0, x1),
        (y0, y1),
        (0.0, h),
        wall,
        [&arches, &[], &[], &[]],
        true,
    );
    for hh in &arches {
        arch_fill(b, &fs[0], wall, hh.s, hh.z.1 - 1.1, (-0.3, 0.0));
        fs[0].bx(b, "f_green", hh.s, (0.0, hh.z.1), (-0.25, -0.2));
    }
    b.rbox(
        "r_gravel",
        [x0 + 0.3, y0 + 0.3, h - 0.3],
        [x1 - 0.3, y1 - 0.3, h + 0.05],
        0.0,
    );
    for f in &fs {
        f.bx(b, wall, (0.0, f.len), (h, h + 0.8), (-0.3, 0.0));
        if !light {
            let m = (f.len / 0.9) as usize;
            for k in 0..m {
                let s = f.len * (k as f64 + 0.5) / m as f64;
                f.bx(
                    b,
                    wall,
                    (s - 0.25, s + 0.25),
                    (h + 0.8, h + 1.4),
                    (-0.3, 0.0),
                );
                f.leaning(
                    b,
                    "c_azulejo",
                    (s - 0.25, s + 0.25),
                    (h + 1.4, -0.3),
                    (h + 1.65, -0.15),
                    0.04,
                );
            }
        }
    }
    // A green tiled roof over the middle, the dome by the mihrab.
    let pts: [P3; 4] = [
        [-3.0, y0 + 1.0, h + 0.05],
        [3.0, y0 + 1.0, h + 0.05],
        [3.0, y1 - 1.0, h + 2.0],
        [-3.0, y1 - 1.0, h + 2.0],
    ];
    let _ = pts;
    let c = [0.0, y0 + (y1 - y0) * 0.35];
    b.rbox(
        wall,
        [c[0] - 2.8, c[1] - 2.8, h],
        [c[0] + 2.8, c[1] + 2.8, h + 1.6],
        0.0,
    );
    dome(b, c, h + 1.6, 2.6, "c_azulejo", light);
    // The minaret at a front corner.
    let s = 4.2;
    let mx = (x1 - s, x1);
    let my = (y1 - 0.01, y1 + s);
    let mh = 26.0;
    let panels = [Hole {
        s: (0.9, s - 0.9),
        z: (12.0, mh - 2.0),
    }];
    let mf = walls_box(
        b,
        mx,
        my,
        (0.0, mh),
        wall,
        [&panels, &panels, &panels, &panels],
        true,
    );
    for f in &mf {
        f.bx(b, "c_azulejo", panels[0].s, panels[0].z, (-0.3, -0.08));
        if !light {
            let m = ((panels[0].z.1 - panels[0].z.0) / 1.2) as usize;
            for k in 0..m {
                let z = panels[0].z.0 + 1.2 * k as f64;
                f.bx(b, wall, (0.9, s - 0.9), (z, z + 0.4), (-0.3, -0.05));
            }
        }
        f.bx(b, "c_azulejo", (0.0, s), (mh - 1.4, mh - 0.6), (0.0, 0.05));
        if !light {
            for k in 0..4 {
                let x = s * (k as f64 + 0.5) / 4.0;
                f.bx(b, wall, (x - 0.3, x + 0.3), (mh, mh + 0.7), (-0.3, 0.0));
            }
        }
    }
    let c = [(mx.0 + mx.1) / 2.0, (my.0 + my.1) / 2.0];
    b.rbox(
        wall,
        [c[0] - s * 0.3, c[1] - s * 0.3, mh - 0.1],
        [c[0] + s * 0.3, c[1] + s * 0.3, mh + 3.6],
        0.0,
    );
    dome(b, c, mh + 3.6, s * 0.3, "c_azulejo", light);
    for k in 0..3 {
        b.ellipsoid(
            "brass",
            [c[0], c[1], mh + 3.6 + s * 0.3 + 0.12 + 0.38 * k as f64],
            [0.2, 0.2, 0.2],
        );
    }
    b.rbox("paving", [-w / 2.0, y1, 0.0], [w / 2.0, d, 0.02], 0.0);
}

/// A stave church: dark wood, stacked steep roofs of shingles, the ridge
/// turret with its spire.
fn stave(b: &mut Builder, w: f64, d: f64, light: bool) {
    let wood = "b_dark";
    let c = [0.0, d / 2.0];
    let (hw, hd) = ((w * 0.3).clamp(4.0, 6.0), (d * 0.3).clamp(5.0, 8.0));
    let mut z = 0.0;
    let mut sc = 1.0;
    for t in 0..3 {
        let h = if t == 0 { 3.2 } else { 2.6 };
        let (x, y) = (
            (c[0] - hw * sc, c[0] + hw * sc),
            (c[1] - hd * sc, c[1] + hd * sc),
        );
        let door = Hole {
            s: (hw * sc - 0.6, hw * sc + 0.6),
            z: (0.0, 2.3),
        };
        let doors = [door];
        let front: &[Hole] = if t == 0 { &doors } else { &[] };
        let fs = walls_box(b, x, y, (z, z + h), wood, [front, &[], &[], &[]], true);
        if t == 0 {
            fs[0].bx(b, "f_red", door.s, door.z, (-0.2, -0.15));
        }
        if !light {
            for f in &fs {
                let m = (f.len / 0.25) as usize;
                for k in 0..m {
                    let s = f.len * (k as f64 + 0.5) / m as f64;
                    f.bx(b, wood, (s - 0.04, s + 0.04), (z, z + h), (0.0, 0.04));
                }
            }
        }
        roofs::roof(
            b,
            &RoofPlan {
                x,
                y,
                z: z + h,
                form: Roof::GableToStreet,
                cover: Cover::Courses,
                pitch: 1.5,
                eaves: 0.5,
                party: false,
                dormers: Vec::new(),
                gable: Gable::Plain,
                seed: t as u64,
                extras: 0,
                top: Top::None,
                block: false,
                light,
            },
            &Look {
                wall: wood,
                base: wood,
                trim: wood,
                frame: wood,
                shutter: wood,
                door: wood,
                roof: "r_shingle",
                accent: wood,
            },
        );
        z += h + hw * sc * 0.38 * 1.5 - 0.05;
        sc *= 0.62;
    }
    b.rbox(
        wood,
        [c[0] - 0.9, c[1] - 0.9, z - 0.8],
        [c[0] + 0.9, c[1] + 0.9, z + 1.8],
        0.0,
    );
    b.cone(
        "r_shingle",
        [c[0], c[1], z + 1.8],
        [c[0], c[1], z + 6.5],
        1.3,
        0.05,
        4,
    );
}
