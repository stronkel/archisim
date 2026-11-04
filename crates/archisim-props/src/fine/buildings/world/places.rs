//! The squares of each place and the parks, the promenades, the avenues
//! and the gardens (tanda 13). Each square is the place's own: the plaza
//! mayor under its arcades in Castile, the square of orange trees in the
//! south, the piazza with its fountain, the market square of the north,
//! the plaza de armas of the colonies, the forecourt of a temple in the
//! East, the green of America and the square of the medina.

use super::super::*;
use super::civic::{bench, dressed, lamp_post, tree, Ctx};
use super::plan::{Plan, Side};
use super::roofs;
use super::skins::*;
use super::{look, Look};
use crate::build::Builder;
use std::f64::consts::PI;

/// How a place makes its square.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum SquareKind {
    /// Under arcades on its four sides (Castile, La Mancha, the north).
    Mayor,
    /// Orange trees in their pits, tiled benches, a low fountain.
    Oranges,
    /// Stone paving and a great fountain (Italy, Greece, Lisbon).
    Piazza,
    /// Cobbles, the market cross and a few stalls (the north of Europe).
    Market,
    /// Lawns in a cross round a bandstand (the colonies, the tropics).
    Arms,
    /// Gravel, a gate, stone lanterns and pines (the East).
    Temple,
    /// A lawn, a statue and the flag (America).
    Green,
    /// Palms round a tiled fountain (the Maghreb).
    Medina,
}

pub fn square_kind(sk: &Skin) -> SquareKind {
    use SquareKind::*;
    match sk.key {
        "castilla" | "mancha" | "extremadura" | "colgadas" | "norte" | "montana" => Mayor,
        "andalucia" | "valencia" | "baleares" | "algarve" | "mediterraneo" => Oranges,
        "italia" | "grecia" | "lisboa" => Piazza,
        "paris" | "reino_unido" | "centroeuropa" | "paises_bajos" | "nordico" | "este" => Market,
        "china" | "japon" | "corea" => Temple,
        "nueva_york" | "eeuu" | "san_francisco" => Green,
        "magreb" => Medina,
        _ => Arms,
    }
}

/// The square of the place.
pub(super) fn square(b: &mut Builder, c: &Ctx) -> Look {
    match square_kind(c.sk) {
        SquareKind::Mayor => mayor(b, c),
        SquareKind::Oranges => oranges(b, c, false),
        SquareKind::Medina => oranges(b, c, true),
        SquareKind::Piazza => piazza(b, c),
        SquareKind::Market => market(b, c),
        SquareKind::Arms => super::civic::bandstand_square(b, c),
        SquareKind::Temple => temple_court(b, c),
        SquareKind::Green => green(b, c),
    }
}

// ------------------------------------------------------------ helpers

/// The paving over the plot and its kerb.
fn ground(b: &mut Builder, w: f64, d: f64, role: &'static str) {
    b.rbox(role, [-w / 2.0, 0.0, 0.0], [w / 2.0, d, 0.03], 0.0);
    for f in roofs::faces((-w / 2.0, w / 2.0), (0.0, d)) {
        f.bx(b, "c_concrete", (0.0, f.len), (0.0, 0.15), (-0.3, 0.0));
    }
}

/// A round basin with its water.
fn basin(b: &mut Builder, c: [f64; 2], r: f64, h: f64, role: &'static str, sides: usize) {
    b.cone(
        role,
        [c[0], c[1], 0.0],
        [c[0], c[1], h],
        r + 0.25,
        r + 0.2,
        sides,
    );
    b.cone(
        "water",
        [c[0], c[1], h - 0.15],
        [c[0], c[1], h + 0.01],
        r,
        r,
        sides,
    );
}

/// A tree with fruit among its leaves (an orange tree).
fn fruit_tree(b: &mut Builder, c: [f64; 2], h: f64, seed: u64, light: bool) {
    let r = h * 0.3;
    b.cone(
        "trunk",
        [c[0], c[1], 0.0],
        [c[0], c[1], h - r],
        0.11,
        0.07,
        7,
    );
    let zc = h - r * 0.9;
    b.ellipsoid("leaves", [c[0], c[1], zc], [r, r, r * 0.9]);
    if !light {
        for k in 0..6 {
            let a = 2.0 * PI * (k as f64 + unit(seed, k)) / 6.0;
            let up = (unit(seed, k + 10) - 0.5) * r;
            let rr = (r * r - up * up).max(0.0).sqrt() * 0.98;
            b.ellipsoid(
                "fruit",
                [c[0] + rr * a.cos(), c[1] + rr * a.sin(), zc + up * 0.9],
                [0.06, 0.06, 0.06],
            );
        }
    }
}

/// A palm: its trunk and a crown of fronds.
pub(super) fn palm(b: &mut Builder, c: [f64; 2], h: f64, seed: u64, light: bool) {
    let lean = (unit(seed, 1) - 0.5) * 0.6;
    let top = [c[0] + lean, c[1] + lean * 0.5, h];
    b.cone("trunk", [c[0], c[1], 0.0], top, 0.2, 0.14, 8);
    b.ellipsoid("leaves", top, [0.35, 0.35, 0.3]);
    if light {
        b.ellipsoid("leaves", [top[0], top[1], h + 0.2], [2.0, 2.0, 0.6]);
        return;
    }
    for k in 0..9 {
        let a = 2.0 * PI * (k as f64 + unit(seed, k + 3) * 0.4) / 9.0;
        let len = 2.4 + unit(seed, k + 20) * 0.6;
        let e = [
            top[0] + len * a.cos(),
            top[1] + len * a.sin(),
            h - 0.6 - unit(seed, k + 30) * 0.5,
        ];
        let m = [
            (top[0] + e[0]) / 2.0,
            (top[1] + e[1]) / 2.0,
            (h + e[2]) / 2.0 + 0.35,
        ];
        for (p, q) in [(top, m), (m, e)] {
            let mid = [
                (p[0] + q[0]) / 2.0,
                (p[1] + q[1]) / 2.0,
                (p[2] + q[2]) / 2.0,
            ];
            let half = [
                (q[0] - p[0]) / 2.0,
                (q[1] - p[1]) / 2.0,
                (q[2] - p[2]) / 2.0,
            ];
            b.obox(
                "leaves",
                mid,
                half,
                [-a.sin() * 0.35, a.cos() * 0.35, 0.0],
                [0.0, 0.0, 0.025],
            );
        }
    }
}

/// A statue on its plinth.
fn statue(b: &mut Builder, c: [f64; 2], stone: &'static str, bronze: &'static str, h: f64) {
    b.rbox(
        stone,
        [c[0] - 0.9, c[1] - 0.9, 0.0],
        [c[0] + 0.9, c[1] + 0.9, 0.5],
        0.0,
    );
    b.rbox(
        stone,
        [c[0] - 0.6, c[1] - 0.6, 0.5],
        [c[0] + 0.6, c[1] + 0.6, h],
        0.0,
    );
    // The figure: legs, body and head.
    b.cone(bronze, [c[0], c[1], h], [c[0], c[1], h + 1.1], 0.3, 0.24, 8);
    b.ellipsoid(bronze, [c[0], c[1], h + 1.45], [0.28, 0.2, 0.45]);
    b.ellipsoid(bronze, [c[0], c[1], h + 2.0], [0.14, 0.14, 0.16]);
}

/// Lamp posts at the four corners of a rectangle, inset `m`.
fn corner_lamps(b: &mut Builder, w: f64, d: f64, m: f64, h: f64) {
    for (x, y) in [
        (-w / 2.0 + m, m),
        (w / 2.0 - m, m),
        (w / 2.0 - m, d - m),
        (-w / 2.0 + m, d - m),
    ] {
        lamp_post(b, [x, y], 0.0, h);
    }
}

/// A market stall: four posts, its counter and a canvas.
pub(super) fn stall(b: &mut Builder, c: [f64; 2], w: f64, d: f64, cloth: &'static str) {
    let (x0, x1, y0, y1) = (
        c[0] - w / 2.0,
        c[0] + w / 2.0,
        c[1] - d / 2.0,
        c[1] + d / 2.0,
    );
    for (x, y) in [(x0, y0), (x1, y0), (x1, y1), (x0, y1)] {
        b.cyl("metal", [x, y, 0.0], [x, y, 2.3], 0.03);
    }
    b.rbox("b_cedar", [x0, y1 - 0.8, 0.0], [x1, y1, 0.85], 0.0);
    b.ellipsoid("fruit", [c[0], y1 - 0.4, 0.95], [w * 0.4, 0.3, 0.12]);
    b.rbox(
        cloth,
        [x0 - 0.1, y0 - 0.1, 2.3],
        [x1 + 0.1, y1 + 0.3, 2.38],
        0.0,
    );
}

// ------------------------------------------------------------ squares

/// The plaza mayor: four rows of houses under arcades round a paved
/// square, a statue in the middle.
fn mayor(b: &mut Builder, c: &Ctx) -> Look {
    let mut sk = *c.sk;
    sk.ground = Ground::Arcade;
    if sk.balc == Balc::None {
        sk.balc = Balc::Iron;
    }
    sk.extras &= !(AC | LAUNDRY | SIGNS | SOLAR | TANK | WATER_TOWER);
    let (w, d) = (c.w, c.d);
    let dp = 9.0_f64.min(w / 4.0).min(d / 4.0);
    let (x0, x1) = (-w / 2.0, w / 2.0);
    // Each row is built as a house of the street, its front to +y, then
    // turned and set on its side of the square, the front inwards.
    let rows: [(f64, f64, [f64; 2]); 4] = [
        (w, 0.0, [0.0, 0.0]),
        (w, PI, [0.0, d]),
        (d - 2.0 * dp, -PI / 2.0, [x0, d / 2.0]),
        (d - 2.0 * dp, PI / 2.0, [x1, d / 2.0]),
    ];
    let mut l = look(&sk, c.seed, false);
    for (k, (len, a, at)) in rows.iter().enumerate() {
        let start = b.mark();
        let plan = Plan::rect(
            *len,
            dp,
            [Side::Back, Side::Party, Side::Street, Side::Party],
        );
        let (_, lk) = dressed(b, &plan, &sk, c.floors.max(3), c.seed + k as u64, c.light);
        let (sn, cs) = a.sin_cos();
        let at = *at;
        b.transform_from(start, |q| {
            [
                at[0] + q[0] * cs - q[1] * sn,
                at[1] + q[0] * sn + q[1] * cs,
                q[2],
            ]
        });
        if k == 0 {
            l = lk;
        }
    }
    b.rbox("paving", [x0 + dp, dp, 0.0], [x1 - dp, d - dp, 0.03], 0.0);
    let cy = d / 2.0;
    statue(b, [0.0, cy], l.trim, "iron", 2.2);
    if !c.light {
        let m = dp + 2.0;
        corner_lamps(b, w, d, m, 3.4);
    }
    l
}

/// The square of orange trees: a grid of trees in their pits, benches of
/// tiles and a low fountain; with palms in the medina.
fn oranges(b: &mut Builder, c: &Ctx, palms: bool) -> Look {
    let l = look(c.sk, c.seed, false);
    ground(b, c.w, c.d, "paving");
    let cy = c.d / 2.0;
    // The fountain: an octagon of tiles, its jet.
    let tile = if palms { "c_azulejo" } else { l.trim };
    basin(b, [0.0, cy], 2.2, 0.6, tile, 8);
    b.cone(tile, [0.0, cy, 0.45], [0.0, cy, 1.2], 0.3, 0.22, 8);
    b.cone("water", [0.0, cy, 1.2], [0.0, cy, 1.3], 0.5, 0.5, 8);
    // The trees in rows, leaving the middle free.
    let step = if palms { 7.5 } else { 5.0 };
    let nx = ((c.w - 4.0) / step).floor() as i32;
    let ny = ((c.d - 4.0) / step).floor() as i32;
    let mut k = 0;
    for i in 0..=nx {
        for j in 0..=ny {
            let x = -(nx as f64) * step / 2.0 + i as f64 * step;
            let y = cy - (ny as f64) * step / 2.0 + j as f64 * step;
            if x.abs() < 4.5 && (y - cy).abs() < 4.5 {
                continue;
            }
            k += 1;
            b.rbox(
                "trunk",
                [x - 0.6, y - 0.6, 0.0],
                [x + 0.6, y + 0.6, 0.05],
                0.0,
            );
            if palms {
                palm(b, [x, y], 6.5, c.seed + k, c.light);
            } else {
                fruit_tree(b, [x, y], 3.6, c.seed + k, c.light);
            }
        }
    }
    // Benches of tiles round the fountain.
    if !c.light {
        for (dx, dy) in [(4.0, 0.0), (-4.0, 0.0), (0.0, 4.0), (0.0, -4.0)] {
            let p = [dx, cy + dy];
            let u: [f64; 2] = if dx != 0.0 { [0.0, 1.0] } else { [1.0, 0.0] };
            let n = [-u[1], u[0]];
            b.obox(
                "c_azulejo",
                [p[0], p[1], 0.22],
                [u[0] * 1.2, u[1] * 1.2, 0.0],
                [n[0] * 0.25, n[1] * 0.25, 0.0],
                [0.0, 0.0, 0.22],
            );
            b.obox(
                "c_azulejo",
                [p[0] + n[0] * 0.25, p[1] + n[1] * 0.25, 0.65],
                [u[0] * 1.2, u[1] * 1.2, 0.0],
                [n[0] * 0.05, n[1] * 0.05, 0.0],
                [0.0, 0.0, 0.43],
            );
        }
        corner_lamps(b, c.w, c.d, 1.5, 3.4);
    }
    l
}

/// The piazza: stone paving drawn in bands to the middle and a fountain
/// with its figures and an obelisk.
fn piazza(b: &mut Builder, c: &Ctx) -> Look {
    let l = look(c.sk, c.seed, false);
    ground(b, c.w, c.d, "s_ashlar");
    let cy = c.d / 2.0;
    if !c.light {
        // Bands of white stone radiating from the fountain.
        for k in 0..12 {
            let a = 2.0 * PI * k as f64 / 12.0;
            let r = c.w.min(c.d) * 0.45;
            let m = [0.5 * r * a.cos(), cy + 0.5 * r * a.sin()];
            b.obox(
                "w_white",
                [m[0], m[1], 0.04],
                [0.5 * r * a.cos(), 0.5 * r * a.sin(), 0.0],
                [-a.sin() * 0.2, a.cos() * 0.2, 0.0],
                [0.0, 0.0, 0.01],
            );
        }
    }
    let r = (c.w.min(c.d) * 0.16).clamp(3.0, 6.0);
    basin(b, [0.0, cy], r, 0.8, "s_lime", 24);
    // The rock with its figures and the obelisk over it.
    b.cone(
        "s_lime",
        [0.0, cy, 0.6],
        [0.0, cy, 2.6],
        r * 0.45,
        r * 0.2,
        9,
    );
    for k in 0..4 {
        let a = PI / 4.0 + PI / 2.0 * k as f64;
        let p = [r * 0.45 * a.cos(), cy + r * 0.45 * a.sin()];
        b.ellipsoid("w_white", [p[0], p[1], 1.4], [0.35, 0.35, 0.75]);
        b.ellipsoid("w_white", [p[0], p[1], 2.25], [0.17, 0.17, 0.2]);
    }
    b.cone("s_granite", [0.0, cy, 2.6], [0.0, cy, 9.0], 0.6, 0.3, 4);
    b.cone("s_granite", [0.0, cy, 9.0], [0.0, cy, 9.6], 0.3, 0.0, 4);
    if !c.light {
        corner_lamps(b, c.w, c.d, 1.2, 3.8);
    }
    l
}

/// The market square: cobbles, a market cross on its steps and stalls.
fn market(b: &mut Builder, c: &Ctx) -> Look {
    let l = look(c.sk, c.seed, false);
    ground(b, c.w, c.d, "s_granite");
    let cy = c.d / 2.0;
    for (k, r) in [2.4, 1.8, 1.2].iter().enumerate() {
        b.cone(
            "s_ashlar",
            [0.0, cy, 0.2 * k as f64],
            [0.0, cy, 0.2 * (k + 1) as f64],
            *r,
            *r,
            8,
        );
    }
    b.cone("s_ashlar", [0.0, cy, 0.6], [0.0, cy, 6.5], 0.3, 0.24, 8);
    b.cone("s_ashlar", [0.0, cy, 6.5], [0.0, cy, 6.8], 0.45, 0.45, 8);
    b.ellipsoid("brass", [0.0, cy, 7.25], [0.3, 0.3, 0.5]);
    // The stalls, in a row on each side.
    if !c.light {
        let n = ((c.w - 8.0) / 4.0).floor().max(1.0) as usize;
        for side in [-1.0, 1.0] {
            for k in 0..n {
                let x = -(n as f64 - 1.0) * 2.0 + 4.0 * k as f64;
                if x.abs() < 3.5 {
                    continue;
                }
                let cloth = ["cloth_a", "cloth_b", "cloth_c"][k % 3];
                stall(b, [x, cy + side * (c.d * 0.3)], 2.8, 2.0, cloth);
            }
        }
        corner_lamps(b, c.w, c.d, 1.2, 3.6);
    }
    l
}

/// The forecourt of a temple: raked gravel, a gate, stone lanterns, pines
/// and the basin for the hands.
fn temple_court(b: &mut Builder, c: &Ctx) -> Look {
    let l = look(c.sk, c.seed, false);
    ground(b, c.w, c.d, "r_gravel");
    let cy = c.d / 2.0;
    // The path of stones from the street.
    b.rbox("s_granite", [-1.5, 0.0, 0.0], [1.5, c.d, 0.06], 0.0);
    // The gate near the street.
    let gy = c.d - 4.0;
    let red = if c.sk.key == "japon" {
        "f_red"
    } else {
        "b_dark"
    };
    for x in [-2.6, 2.6] {
        b.cyl(red, [x, gy, 0.0], [x, gy, 5.0], 0.22);
    }
    b.rbox(red, [-3.5, gy - 0.25, 4.2], [3.5, gy + 0.25, 4.5], 0.0);
    b.rbox("b_dark", [-4.2, gy - 0.35, 5.0], [4.2, gy + 0.35, 5.4], 0.0);
    b.rbox(red, [-0.15, gy - 0.2, 4.5], [0.15, gy + 0.2, 5.0], 0.0);
    // Stone lanterns along the path.
    for y in [cy - 6.0, cy, cy + 6.0] {
        for x in [-3.0, 3.0] {
            if y > gy - 2.0 {
                continue;
            }
            b.rbox(
                "s_granite",
                [x - 0.3, y - 0.3, 0.0],
                [x + 0.3, y + 0.3, 0.3],
                0.0,
            );
            b.cyl("s_granite", [x, y, 0.3], [x, y, 1.3], 0.12);
            b.rbox(
                "s_granite",
                [x - 0.3, y - 0.3, 1.3],
                [x + 0.3, y + 0.3, 1.75],
                0.0,
            );
            b.cone("s_granite", [x, y, 1.75], [x, y, 2.1], 0.5, 0.05, 4);
        }
    }
    // The basin under its roof.
    let p = [c.w / 2.0 - 5.0, cy + 2.0];
    b.rbox(
        "s_granite",
        [p[0] - 1.0, p[1] - 0.6, 0.0],
        [p[0] + 1.0, p[1] + 0.6, 0.7],
        0.0,
    );
    b.rbox(
        "water",
        [p[0] - 0.85, p[1] - 0.45, 0.6],
        [p[0] + 0.85, p[1] + 0.45, 0.68],
        0.0,
    );
    for (x, y) in [(-1.3, -0.9), (1.3, -0.9), (1.3, 0.9), (-1.3, 0.9)] {
        b.cyl(
            "b_dark",
            [p[0] + x, p[1] + y, 0.0],
            [p[0] + x, p[1] + y, 2.5],
            0.1,
        );
    }
    b.cone("roof", [p[0], p[1], 2.5], [p[0], p[1], 3.6], 2.3, 0.2, 4);
    // The hall at the back, on its platform under a deep roof.
    let (hx0, hx1) = (-6.0_f64.min(c.w * 0.25), 6.0_f64.min(c.w * 0.25));
    let (hy0, hy1) = (1.5, 8.5_f64.min(cy - 4.0));
    b.rbox(
        "s_granite",
        [hx0 - 0.8, hy0 - 0.8, 0.0],
        [hx1 + 0.8, hy1 + 0.8, 0.6],
        0.0,
    );
    let door = Hole {
        s: ((hx1 - hx0) / 2.0 - 1.6, (hx1 - hx0) / 2.0 + 1.6),
        z: (0.6, 3.0),
    };
    let fs = super::civic::walls_box(
        b,
        (hx0, hx1),
        (hy0, hy1),
        (0.6, 4.2),
        "w_white",
        [&[door], &[], &[], &[]],
        true,
    );
    fs[0].bx(b, "b_dark", door.s, door.z, (-0.22, -0.16));
    for f in &fs {
        let n = (f.len / 2.0).round().max(1.0) as usize;
        for i in 0..=n {
            let s = f.len * i as f64 / n as f64;
            f.bx(b, "b_dark", (s - 0.12, s + 0.12), (0.6, 4.2), (0.0, 0.06));
        }
        f.bx(b, "b_dark", (0.0, f.len), (3.0, 3.25), (0.0, 0.06));
    }
    super::roofs::roof(
        b,
        &super::roofs::RoofPlan {
            x: (hx0, hx1),
            y: (hy0, hy1),
            z: 4.2,
            form: Roof::Asian,
            cover: Cover::Courses,
            pitch: 0.5,
            eaves: 1.0,
            party: false,
            dormers: Vec::new(),
            gable: Gable::Plain,
            seed: 1,
            extras: 0,
            top: Top::None,
            block: false,
            light: c.light,
        },
        &l,
    );
    // Pines with flat crowns.
    for (k, (x, y)) in [
        (-c.w / 2.0 + 4.0, 4.0),
        (-c.w / 2.0 + 5.0, c.d - 8.0),
        (c.w / 2.0 - 4.0, 5.0),
        (-c.w / 2.0 + 4.0, cy),
    ]
    .iter()
    .enumerate()
    {
        b.cone("trunk", [*x, *y, 0.0], [*x + 0.6, *y, 4.0], 0.2, 0.12, 7);
        b.ellipsoid("leaves", [*x + 0.6, *y, 4.2], [2.4, 2.0, 0.7]);
        if !c.light {
            b.ellipsoid(
                "leaves",
                [*x + 0.3, *y + 0.4, 3.0 + unit(c.seed, k as u64) * 0.3],
                [1.6, 1.4, 0.5],
            );
        }
    }
    l
}

/// The green: a lawn crossed by paths, a statue, trees and the flag.
fn green(b: &mut Builder, c: &Ctx) -> Look {
    let l = look(c.sk, c.seed, false);
    ground(b, c.w, c.d, "paving");
    let cy = c.d / 2.0;
    b.rbox(
        "lawn",
        [-c.w / 2.0 + 2.0, 2.0, 0.03],
        [c.w / 2.0 - 2.0, c.d - 2.0, 0.08],
        0.0,
    );
    // Two paths across, from corner to corner.
    for s in [1.0, -1.0] {
        let a: f64 = (c.d - 4.0).atan2(s * (c.w - 4.0));
        let half = ((c.w - 4.0).powi(2) + (c.d - 4.0).powi(2)).sqrt() / 2.0;
        b.obox(
            "paving",
            [0.0, cy, 0.09],
            [half * a.cos(), half * a.sin(), 0.0],
            [-a.sin() * 1.0, a.cos() * 1.0, 0.0],
            [0.0, 0.0, 0.01],
        );
    }
    statue(b, [0.0, cy], "s_granite", "iron", 2.6);
    super::civic::flagpole(b, [3.5, cy + 3.5], 0.0, 9.0, "cloth_b", [1.0, 0.0]);
    let mut k = 0;
    for (x, y) in [
        (-c.w / 2.0 + 5.0, 5.0),
        (c.w / 2.0 - 5.0, 5.0),
        (-c.w / 2.0 + 5.0, c.d - 5.0),
        (c.w / 2.0 - 5.0, c.d - 5.0),
        (-c.w / 2.0 + 5.0, cy),
        (c.w / 2.0 - 5.0, cy),
    ] {
        k += 1;
        tree(b, [x, y], 0.0, 8.0, c.seed + k);
    }
    if !c.light {
        for (x, y, u) in [
            (-3.0, cy - 4.0, [1.0, 0.0]),
            (3.0, cy - 4.0, [1.0, 0.0]),
            (-3.0, cy + 4.0, [1.0, 0.0]),
        ] {
            bench(b, [x, y], u, "b_cedar");
        }
        corner_lamps(b, c.w, c.d, 1.2, 3.6);
    }
    l
}

// -------------------------------------------------- parks and gardens

/// The playground: swings, a slide and a sandpit.
fn playground(b: &mut Builder, c: [f64; 2], light: bool) {
    let (x0, y0) = (c[0] - 5.0, c[1] - 4.0);
    b.rbox("sand", [x0, y0, 0.0], [x0 + 10.0, y0 + 8.0, 0.05], 0.0);
    if light {
        b.rbox(
            "f_red",
            [x0 + 1.0, y0 + 1.0, 0.05],
            [x0 + 3.0, y0 + 3.0, 2.0],
            0.0,
        );
        return;
    }
    // The swings: two A frames and their bar.
    let sy = y0 + 2.0;
    let (sx0, sx1) = (x0 + 1.0, x0 + 4.5);
    for x in [sx0, sx1] {
        for dy in [-0.8, 0.8] {
            b.cyl("f_red", [x, sy + dy, 0.05], [x, sy, 2.4], 0.05);
        }
    }
    b.cyl("f_red", [sx0, sy, 2.4], [sx1, sy, 2.4], 0.06);
    for x in [sx0 + 1.0, sx1 - 1.0] {
        for dx in [-0.2, 0.2] {
            b.cyl("metal", [x + dx, sy, 2.38], [x + dx, sy, 0.48], 0.01);
        }
        b.rbox(
            "tyre",
            [x - 0.25, sy - 0.12, 0.42],
            [x + 0.25, sy + 0.12, 0.5],
            0.0,
        );
    }
    // The slide: its tower, ladder and chute.
    let (tx, ty) = (x0 + 7.5, y0 + 5.5);
    for (dx, dy) in [(-0.6, -0.6), (0.6, -0.6), (0.6, 0.6), (-0.6, 0.6)] {
        b.cyl(
            "f_yellow",
            [tx + dx, ty + dy, 0.05],
            [tx + dx, ty + dy, 2.6],
            0.05,
        );
    }
    b.rbox(
        "b_cedar",
        [tx - 0.65, ty - 0.65, 1.4],
        [tx + 0.65, ty + 0.65, 1.5],
        0.0,
    );
    b.cone("f_blue", [tx, ty, 2.6], [tx, ty, 3.4], 1.4, 0.05, 4);
    for k in 0..5 {
        let z = 0.3 * (k + 1) as f64;
        b.cyl(
            "metal",
            [tx - 0.6, ty - 0.6, z],
            [tx + 0.6, ty - 0.6, z],
            0.02,
        );
    }
    b.obox(
        "f_blue",
        [tx, ty + 0.65 + 1.3, 0.75],
        [0.0, 1.3, -0.72],
        [0.3, 0.0, 0.0],
        [0.0, 0.03, 0.05],
    );
    // A see-saw.
    let (qx, qy) = (x0 + 2.5, y0 + 6.0);
    b.rbox(
        "f_yellow",
        [qx - 0.15, qy - 0.2, 0.05],
        [qx + 0.15, qy + 0.2, 0.53],
        0.0,
    );
    b.obox(
        "f_green",
        [qx, qy, 0.55],
        [1.6, 0.0, 0.25],
        [0.0, 0.15, 0.0],
        [0.0, 0.0, 0.04],
    );
}

/// The park of a neighbourhood: lawns, paths, trees, a playground,
/// benches, a fountain to drink from and its hedge.
pub(super) fn park(b: &mut Builder, c: &Ctx) -> Look {
    let l = look(c.sk, c.seed, false);
    let (w, d) = (c.w, c.d);
    b.rbox("lawn", [-w / 2.0, 0.0, 0.0], [w / 2.0, d, 0.06], 0.0);
    // The paths: one round and one across.
    b.rbox("sand", [-1.5, 0.0, 0.06], [1.5, d, 0.09], 0.0);
    b.rbox(
        "sand",
        [-w / 2.0, d / 2.0 - 1.5, 0.06],
        [w / 2.0, d / 2.0 + 1.5, 0.09],
        0.0,
    );
    playground(b, [w / 4.0 + 0.5, d / 4.0 + 0.5], c.light);
    let palms = matches!(c.sk.key, "tropico" | "magreb" | "california" | "latam");
    let mut k = 0;
    for (fx, fy) in [
        (-0.35, 0.2),
        (-0.15, 0.3),
        (-0.4, 0.75),
        (-0.2, 0.85),
        (0.3, 0.7),
        (0.4, 0.9),
        (0.15, 0.62),
    ] {
        k += 1;
        let p = [w * fx, d * fy];
        if palms && k % 2 == 0 {
            palm(b, p, 7.0, c.seed + k, c.light);
        } else {
            tree(b, p, 0.06, 7.0 + unit(c.seed, k) * 2.0, c.seed + k);
        }
    }
    if !c.light {
        for (x, y, u) in [
            (-4.0, d / 2.0 + 2.2, [1.0, 0.0]),
            (4.0, d / 2.0 - 2.2, [-1.0, 0.0]),
            (2.4, d * 0.75, [0.0, 1.0]),
        ] {
            bench(b, [x, y], u, "b_cedar");
        }
        // The drinking fountain.
        b.cyl(
            "iron",
            [-2.2, d / 2.0 + 2.2, 0.06],
            [-2.2, d / 2.0 + 2.2, 1.0],
            0.12,
        );
        // The hedge round it, its gates on the paths.
        for f in roofs::faces((-w / 2.0, w / 2.0), (0.0, d)) {
            let mid = f.len / 2.0;
            for (s0, s1) in [(0.0, mid - 2.0), (mid + 2.0, f.len)] {
                f.bx(b, "hedge", (s0, s1), (0.06, 1.0), (-0.7, 0.0));
            }
        }
        corner_lamps(b, w, d, 1.5, 3.6);
    }
    l
}

/// The promenade by the sea: the walk with its rail over the beach, palms,
/// benches and lamps; the sand and the edge of the water.
pub(super) fn promenade(b: &mut Builder, c: &Ctx) -> Look {
    let l = look(c.sk, c.seed, false);
    let (w, d) = (c.w, c.d);
    let ys = d * 0.45;
    let top = 1.4;
    // The sea, the sand and the wall.
    b.rbox("water", [-w / 2.0, 0.0, 0.0], [w / 2.0, 3.0, 0.2], 0.0);
    b.rbox("sand", [-w / 2.0, 3.0, 0.0], [w / 2.0, ys, 0.3], 0.0);
    b.rbox(l.trim, [-w / 2.0, ys, 0.0], [w / 2.0, d, top], 0.0);
    b.rbox("paving", [-w / 2.0, ys, top], [w / 2.0, d, top + 0.04], 0.0);
    // The rail along the edge.
    let f = Face {
        o: [w / 2.0, ys + 0.3],
        u: [-1.0, 0.0],
        len: w,
    };
    let n = (w / 2.0).ceil() as usize;
    for i in 0..=n {
        let s = w * i as f64 / n as f64;
        f.bx(
            b,
            "w_white",
            (s - 0.08, s + 0.08),
            (top, top + 1.0),
            (-0.08, 0.08),
        );
    }
    f.bx(
        b,
        "w_white",
        (0.0, w),
        (top + 1.0, top + 1.12),
        (-0.12, 0.12),
    );
    if !c.light {
        let m = ((w / 0.35).floor()) as usize;
        for i in 0..m {
            let s = 0.175 + 0.35 * i as f64;
            f.bar(b, "w_white", (s, top + 0.04), (s, top + 1.0), 0.0, 0.03, 6);
        }
    }
    // Stairs down to the beach.
    let sw = 2.4;
    let steps = 7;
    for k in 0..steps {
        let z = top * (k + 1) as f64 / (steps + 1) as f64;
        let y = ys - 0.35 * (steps - k) as f64;
        b.rbox(l.trim, [-sw / 2.0, y, 0.0], [sw / 2.0, ys, z], 0.0);
    }
    // Palms, benches and lamps along the walk.
    let n = (w / 8.0).floor().max(1.0) as usize;
    for i in 0..n {
        let x = -w / 2.0 + 4.0 + 8.0 * i as f64;
        let y = d - 2.0;
        let start = b.mark();
        palm(b, [x, y], 7.5, c.seed + i as u64, c.light);
        lift(b, start, top);
        if !c.light {
            let s = b.mark();
            lamp_post(b, [x + 4.0, ys + 1.0], 0.0, 3.6);
            bench(b, [x, ys + 1.2], [1.0, 0.0], "b_cedar");
            lift(b, s, top + 0.04);
        }
    }
    l
}

/// The avenue (alameda): a walk of sand between two rows of trees, benches
/// and lamps, a small fountain in the middle.
pub(super) fn alameda(b: &mut Builder, c: &Ctx) -> Look {
    let l = look(c.sk, c.seed, false);
    let (w, d) = (c.w, c.d);
    ground(b, w, d, "paving");
    let cy = d / 2.0;
    b.rbox(
        "sand",
        [-w / 2.0 + 1.0, cy - 4.0, 0.03],
        [w / 2.0 - 1.0, cy + 4.0, 0.06],
        0.0,
    );
    let n = ((w - 4.0) / 6.0).floor().max(2.0) as usize;
    for i in 0..n {
        let x = -w / 2.0 + 3.0 + 6.0 * i as f64;
        for (j, y) in [cy - 5.5, cy + 5.5].iter().enumerate() {
            tree(b, [x, *y], 0.0, 9.0, c.seed + (i * 2 + j) as u64);
        }
        if !c.light && x.abs() > 3.5 {
            let y = if i % 2 == 0 { cy - 3.4 } else { cy + 3.4 };
            let u = if i % 2 == 0 { [1.0, 0.0] } else { [-1.0, 0.0] };
            bench(b, [x + 3.0, y], u, "b_cedar");
            if i % 2 == 0 {
                lamp_post(b, [x + 3.0, cy + 3.6], 0.0, 3.8);
            }
        }
    }
    basin(b, [0.0, cy], 1.8, 0.5, l.trim, 16);
    b.cone(l.trim, [0.0, cy, 0.4], [0.0, cy, 1.4], 0.25, 0.18, 8);
    b.ellipsoid(l.trim, [0.0, cy, 1.5], [0.2, 0.2, 0.15]);
    l
}

/// The garden: parterres of hedges with roses, a pergola, a pond and a
/// glasshouse.
pub(super) fn garden(b: &mut Builder, c: &Ctx) -> Look {
    let l = look(c.sk, c.seed, false);
    let (w, d) = (c.w, c.d);
    b.rbox("sand", [-w / 2.0, 0.0, 0.0], [w / 2.0, d, 0.04], 0.0);
    // The parterres: four squares edged with box, roses inside.
    let pw = (w * 0.18).min(7.0);
    for (fx, fy) in [(-0.25, 0.62), (0.25, 0.62), (-0.25, 0.88), (0.25, 0.88)] {
        let p = [w * fx, d * fy - 1.5];
        let (x0, x1, y0, y1) = (
            p[0] - pw / 2.0,
            p[0] + pw / 2.0,
            p[1] - pw / 2.5,
            p[1] + pw / 2.5,
        );
        b.rbox("lawn", [x0, y0, 0.04], [x1, y1, 0.08], 0.0);
        for f in roofs::faces((x0, x1), (y0, y1)) {
            f.bx(b, "hedge", (0.0, f.len), (0.08, 0.5), (-0.35, 0.0));
        }
        if !c.light {
            for k in 0..6 {
                let q = [
                    x0 + 1.0 + (x1 - x0 - 2.0) * unit(c.seed, k * 7),
                    y0 + 1.0 + (y1 - y0 - 2.0) * unit(c.seed, k * 7 + 1),
                ];
                b.ellipsoid("hedge", [q[0], q[1], 0.35], [0.4, 0.4, 0.3]);
                b.ellipsoid("flowers", [q[0], q[1], 0.6], [0.3, 0.3, 0.12]);
            }
        }
    }
    // The pond in the middle.
    let cy = d * 0.5;
    b.cone(l.trim, [0.0, cy, 0.0], [0.0, cy, 0.4], 3.2, 3.2, 20);
    b.cone("water", [0.0, cy, 0.3], [0.0, cy, 0.41], 3.0, 3.0, 20);
    // The pergola along the back.
    let (px0, px1, py) = (-w / 2.0 + 3.0, -2.0, 4.0);
    let n = ((px1 - px0) / 2.5).floor().max(1.0) as usize;
    for i in 0..=n {
        let x = px0 + (px1 - px0) * i as f64 / n as f64;
        for y in [py - 1.2, py + 1.2] {
            b.rbox(
                l.trim,
                [x - 0.15, y - 0.15, 0.04],
                [x + 0.15, y + 0.15, 2.6],
                0.0,
            );
        }
        b.rbox(
            "b_cedar",
            [x - 0.06, py - 1.6, 2.6],
            [x + 0.06, py + 1.6, 2.8],
            0.0,
        );
    }
    b.rbox(
        "leaves",
        [px0 - 0.2, py - 1.5, 2.8],
        [px1 + 0.2, py + 1.5, 3.1],
        0.2,
    );
    // The glasshouse.
    let (gx0, gx1, gy0, gy1) = (w / 2.0 - 12.0, w / 2.0 - 2.0, 2.0, 8.0);
    b.rbox(l.trim, [gx0, gy0, 0.04], [gx1, gy1, 0.6], 0.0);
    b.rbox(
        "glass",
        [gx0 + 0.05, gy0 + 0.05, 0.6],
        [gx1 - 0.05, gy1 - 0.05, 3.2],
        0.0,
    );
    // Its pitched glass roof: the gable's section drawn on the side.
    let gd = gy1 - gy0;
    let side = Face {
        o: [gx0, gy0],
        u: [0.0, 1.0],
        len: gd,
    };
    side.prism(
        b,
        "glass",
        &[(0.0, 3.2), (gd, 3.2), (gd / 2.0, 3.2 + gd * 0.35)],
        (-(gx1 - gx0) + 0.05, -0.05),
    );
    if !c.light {
        let m = ((gx1 - gx0) / 1.2).round() as usize;
        for i in 0..=m {
            let x = gx0 + (gx1 - gx0) * i as f64 / m as f64;
            b.cyl("f_white", [x, gy1, 0.6], [x, gy1, 3.2], 0.03);
            b.cyl("f_white", [x, gy0, 0.6], [x, gy0, 3.2], 0.03);
        }
        corner_lamps(b, w, d, 1.2, 3.4);
    }
    l
}

/// Moves what was drawn since `start` up by `dz`.
pub(super) fn lift(b: &mut Builder, start: usize, dz: f64) {
    for p in b.positions[start..].iter_mut() {
        p[2] += dz as f32;
    }
}
