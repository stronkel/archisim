//! Shops of the eight collections (paquete 7, tanda 7c): the counter with
//! its till, the wall shelving with its goods, the display table for the
//! middle of the shop, the clothes rail with its garments and the fitting
//! room, each as the collection makes it.
//!
//! Counter: `width`, `depth`, `height`; the staff at `y = 0`, the
//! customers towards `+y`. Shelving and fitting room: `width`, `depth`,
//! `height`; on the wall at `y = 0`, open towards `+y`. Display table and
//! clothes rail: `width`, `depth`, `height`; centred on `x = 0`, from
//! `y = 0` to `depth`.
//!
//! Roles: `frame`, `top`, `front`, `masonry`, `accent`, `metal`, `dark`,
//! `glass`, `mirror`, `light`, `display`, the goods `goods`, `goods2` and
//! `goods3`, the clothes `cloth`, `cloth2` and `cloth3`, and `curtain`.

use super::hospitality::{arch_head, disc, look, rod, stone, turn_back, Rng};
use super::office::{felt, turned_leg};
use super::shape::{bar, plate, sweep, turned, Cap};
use super::sofas::compass_leg;
use super::soft::Soft;
use super::storage::{front, tapered_leg, Panel};
use crate::build::{Builder, P3};
use crate::Params;
use std::f64::consts::{FRAC_PI_4, PI};

// -------------------------------------------------------------- the goods

/// Goods standing in a row from `x0` to `x1` on a shelf at `z`, from the
/// back `yb` forward `depth`, none taller than `room`: boxes, jars and
/// stacks of folded clothes, as `kind` says (0 mixed, 1 clothes).
#[allow(clippy::too_many_arguments)]
fn goods(
    b: &mut Builder,
    x0: f64,
    x1: f64,
    yb: f64,
    depth: f64,
    z: f64,
    room: f64,
    kind: u8,
    seed: u32,
) {
    let mut rnd = Rng(0x7a3c_51e9 ^ seed);
    let boxes = ["goods", "goods2", "goods3"];
    let cloths = ["cloth", "cloth2", "cloth3"];
    let mut x = x0 + rnd.range(0.01, 0.04);
    while x < x1 - 0.08 {
        let pick = if kind == 1 { 0.0 } else { rnd.next() };
        if pick < 0.35 {
            // A stack of folded clothes.
            let w = rnd.range(0.26, 0.32).min(x1 - x - 0.01);
            if w < 0.15 {
                break;
            }
            let d = (depth * 0.8).min(0.3);
            let n = ((room * 0.7) / 0.045).floor().clamp(2.0, 6.0) as usize;
            let role = cloths[(rnd.next() * 3.0) as usize % 3];
            let mut zz = z;
            for _ in 0..n {
                let dx = rnd.range(-0.006, 0.006);
                stone(
                    b,
                    role,
                    [x + dx, yb + 0.02, zz],
                    [x + dx + w, yb + 0.02 + d, zz + 0.04],
                    0.25,
                );
                zz += 0.042;
            }
            x += w + rnd.range(0.03, 0.06);
        } else if pick < 0.7 {
            // A row of boxes.
            let n = 2 + (rnd.next() * 3.0) as usize;
            let (w, h) = (rnd.range(0.08, 0.16), rnd.range(0.1, 0.26).min(room - 0.02));
            let role = boxes[(rnd.next() * 3.0) as usize % 3];
            for _ in 0..n {
                if x + w > x1 - 0.01 {
                    break;
                }
                b.rbox(
                    role,
                    [x, yb + 0.02, z],
                    [x + w - 0.004, yb + 0.02 + depth.min(0.25), z + h],
                    0.0,
                );
                b.rbox(
                    "goods3",
                    [x + w * 0.2, yb + 0.02 + depth.min(0.25), z + h * 0.45],
                    [
                        x + w * 0.8 - 0.004,
                        yb + 0.022 + depth.min(0.25),
                        z + h * 0.75,
                    ],
                    0.0,
                );
                x += w;
            }
            x += rnd.range(0.03, 0.06);
        } else {
            // Jars.
            let n = 2 + (rnd.next() * 3.0) as usize;
            let (r, h) = (rnd.range(0.035, 0.05), rnd.range(0.1, 0.2).min(room - 0.02));
            let role = boxes[(rnd.next() * 3.0) as usize % 3];
            for _ in 0..n {
                if x + 2.0 * r > x1 - 0.01 {
                    break;
                }
                let c = [x + r, yb + 0.02 + depth.min(0.25) * 0.6, z];
                turned(
                    b,
                    "glass",
                    c,
                    &[
                        (0.0, 0.0),
                        (r, 0.0),
                        (r, h * 0.85),
                        (r * 0.8, h * 0.88),
                        (0.0, h * 0.88),
                    ],
                    12,
                    0.0,
                );
                turned(
                    b,
                    role,
                    [c[0], c[1], c[2] + h * 0.88],
                    &[
                        (0.0, 0.0),
                        (r * 0.85, 0.0),
                        (r * 0.85, h * 0.12),
                        (0.0, h * 0.12),
                    ],
                    12,
                    0.0,
                );
                x += 2.0 * r + 0.01;
            }
            x += rnd.range(0.03, 0.06);
        }
    }
}

/// A garment on its hanger, hanging from a rail at `(x, y, z)`, its face
/// across the rail (towards `±y`), `long` long.
fn garment(b: &mut Builder, role: &'static str, x: f64, y: f64, z: f64, long: f64, wide: f64) {
    // The hanger: a hook over the rail and a bar across.
    let hook: Vec<P3> = (0..=8)
        .map(|k| {
            let a = PI * k as f64 / 8.0;
            [x, y - 0.012 * a.cos(), z + 0.014 * a.sin() + 0.002]
        })
        .collect();
    sweep(b, "metal", &hook, |_| 0.0025, 5, Cap::Flat, Cap::Flat);
    let zh = z - 0.05;
    rod(b, "metal", [x, y - 0.012, z], [x, y, zh + 0.01], 0.0025);
    rod(
        b,
        "frame",
        [x, y - wide / 2.0, zh],
        [x, y + wide / 2.0, zh],
        0.006,
    );
    // The garment: a soft panel, its shoulders rounded.
    let mut s = Soft::new(
        [x - 0.018, y - wide / 2.0 - 0.01, zh - long],
        [x + 0.018, y + wide / 2.0 + 0.01, zh + 0.012],
        0.016,
    );
    s.crown = 0.008;
    s.side = 0.01;
    s.wobble = 0.002;
    s.seed = (x * 1000.0) as u32;
    s.step = 0.12;
    s.add(b, role);
}

/// Garments on hangers along a rail from `x0` to `x1` at `(y, z)`.
fn garments(b: &mut Builder, x0: f64, x1: f64, y: f64, z: f64, room: f64, seed: u32) {
    let mut rnd = Rng(0x15a4_e35d ^ seed);
    let roles = ["cloth", "cloth2", "cloth3"];
    let mut x = x0 + 0.06;
    while x < x1 - 0.06 {
        let role = roles[(rnd.next() * 3.0) as usize % 3];
        let long = rnd.range(0.55, 0.95).min(room - 0.1);
        garment(b, role, x, y, z, long, rnd.range(0.38, 0.46));
        x += rnd.range(0.05, 0.075);
    }
}

/// A till: a screen on its stand and a card reader on a counter top at
/// `(x, y, z)`, the screen facing the staff (`-y`).
fn till(b: &mut Builder, x: f64, y: f64, z: f64) {
    let start = b.mark();
    b.rbox("dark", [-0.1, -0.08, 0.0], [0.1, 0.08, 0.02], 0.006);
    b.rbox("dark", [-0.02, -0.02, 0.02], [0.02, 0.02, 0.16], 0.004);
    let s = b.mark();
    b.rbox("dark", [-0.17, -0.01, 0.0], [0.17, 0.01, 0.24], 0.008);
    b.rbox("display", [-0.155, 0.01, 0.015], [0.155, 0.013, 0.225], 0.0);
    // Tilted back, facing `+y` before the whole is turned to the staff.
    b.transform_from(s, |q| {
        let (sn, cs) = (0.3_f64).sin_cos();
        [q[0], q[1] * cs - q[2] * sn, 0.12 + q[1] * sn + q[2] * cs]
    });
    b.rbox("dark", [0.22, -0.03, 0.0], [0.3, 0.1, 0.04], 0.008);
    b.rbox("display", [0.235, 0.02, 0.04], [0.285, 0.06, 0.042], 0.0);
    turn_back(b, start, 0.0);
    b.transform_from(start, |q| [q[0] + x, q[1] + y, q[2] + z]);
}

/// A straight curtain, pleated, from `x0` to `x1` along `y`, from the floor
/// to `z`, drawn in its pleats `k` times its width.
fn curtain(b: &mut Builder, x0: f64, x1: f64, y: f64, z0: f64, z1: f64) {
    let n = (((x1 - x0) / 0.06).round() as usize).max(2);
    let pts: Vec<[f64; 2]> = (0..=n)
        .map(|i| {
            let x = x0 + (x1 - x0) * i as f64 / n as f64;
            let dy = if i % 2 == 0 { 0.03 } else { -0.03 };
            [x, y + dy]
        })
        .collect();
    for w in pts.windows(2) {
        let (a, e) = (w[0], w[1]);
        plate(
            b,
            "curtain",
            [
                [a[0], a[1], z0],
                [e[0], e[1], z0],
                [e[0], e[1], z1],
                [a[0], a[1], z1],
            ],
            0.004,
        );
    }
}

// --------------------------------------------------------------- counter

/// Shop counters of the collections.
pub fn shop_counter(b: &mut Builder, p: &Params) {
    let ix = look(p);
    let w = p.get("width", 1.8).clamp(1.0, 5.0);
    let d = p.get("depth", 0.65).clamp(0.5, 0.9);
    let h = p.get("height", 0.95).clamp(0.85, 1.1);
    let (x0, x1) = (-w / 2.0, w / 2.0);
    let t = [0.03, 0.04, 0.03, 0.035, 0.02, 0.06, 0.04, 0.04][ix];
    let zt = h - t;
    let face: &'static str = match ix {
        1 | 5 => "masonry",
        2 => "accent",
        4 => "front",
        6 => "dark",
        _ => "frame",
    };
    // The body: the front towards the customers, the staff side open with
    // two shelves.
    let ds = d - 0.03;
    b.rbox(face, [x0, ds - 0.05, 0.0], [x1, ds, zt], 0.004);
    for (a, e) in [(x0, x0 + 0.03), (x1 - 0.03, x1)] {
        b.rbox(face, [a, 0.0, 0.0], [e, ds, zt], 0.003);
    }
    b.rbox(
        "dark",
        [x0 + 0.03, 0.04, 0.0],
        [x1 - 0.03, ds - 0.05, 0.08],
        0.0,
    );
    for z in [0.08, zt * 0.55] {
        b.rbox(
            "frame",
            [x0 + 0.03, 0.02, z],
            [x1 - 0.03, ds - 0.05, z + 0.02],
            0.0,
        );
    }
    b.rbox("top", [x0 - 0.01, -0.02, zt], [x1 + 0.01, d, h], 0.006);
    match ix {
        0 => front(
            b,
            Panel::Flutes,
            "frame",
            x0 + 0.03,
            x1 - 0.03,
            0.04,
            zt - 0.01,
            ds,
        ),
        1 => {
            let tile = 0.15;
            let k = ((w - 0.06) / tile).floor() as usize;
            for i in 0..k {
                let role = if i % 2 == 0 { "accent" } else { "masonry" };
                let xa = -(k as f64) * tile / 2.0 + tile * i as f64;
                b.rbox(
                    role,
                    [xa + 0.003, ds, zt - 0.18],
                    [xa + tile - 0.003, ds + 0.008, zt - 0.03],
                    0.0,
                );
            }
        }
        2 => {
            for z in [0.3, 0.55] {
                b.rbox(
                    "metal",
                    [x0 + 0.03, ds, z - 0.012],
                    [x1 - 0.03, ds + 0.006, z + 0.012],
                    0.003,
                );
            }
        }
        3 => front(
            b,
            Panel::Ribs,
            "frame",
            x0 + 0.03,
            x1 - 0.03,
            0.06,
            zt - 0.01,
            ds,
        ),
        4 => {
            b.rbox(
                "light",
                [x0 + 0.05, ds, zt - 0.008],
                [x1 - 0.05, ds + 0.004, zt],
                0.0,
            );
            b.rbox("dark", [x0, ds - 0.07, 0.0], [x1, ds - 0.045, 0.05], 0.0);
        }
        5 => {
            let mut rnd = Rng(53);
            let course = 0.16;
            for r in 0..(zt / course).floor() as usize {
                let (za, ze) = (course * r as f64, course * (r + 1) as f64);
                let mut x = x0 + if r % 2 == 0 { 0.0 } else { -0.12 };
                while x < x1 {
                    let l = rnd.range(0.22, 0.42);
                    let (xa, xe) = (x.max(x0) + 0.008, (x + l).min(x1) - 0.008);
                    if xe - xa > 0.05 {
                        stone(
                            b,
                            "masonry",
                            [xa, ds - 0.01, za + 0.008],
                            [xe, ds + rnd.range(0.004, 0.012), ze - 0.008],
                            0.18,
                        );
                    }
                    x += l;
                }
            }
        }
        6 => {
            let rows = (zt / 0.14).floor() as usize;
            for r in 0..rows {
                let z = 0.0 + zt * r as f64 / rows as f64;
                b.rbox(
                    "frame",
                    [x0 + 0.04, ds, z + 0.004],
                    [x1 - 0.04, ds + 0.012, z + zt / rows as f64 - 0.004],
                    0.003,
                );
            }
            for x in [x0, x1 - 0.04] {
                b.rbox("dark", [x, ds - 0.01, 0.0], [x + 0.04, ds + 0.016, zt], 0.0);
            }
        }
        _ => {
            let bays = (w / 0.6).round().max(1.0) as usize;
            let bw = (w - 0.06) / bays as f64;
            for i in 0..bays {
                let a = x0 + 0.03 + bw * i as f64;
                front(
                    b,
                    Panel::Raised,
                    "frame",
                    a + 0.05,
                    a + bw - 0.05,
                    0.14,
                    zt - 0.08,
                    ds,
                );
            }
            b.rbox("frame", [x0, ds - 0.05, 0.0], [x1, ds + 0.02, 0.1], 0.004);
            b.rbox(
                "frame",
                [x0 - 0.01, ds - 0.05, zt - 0.05],
                [x1 + 0.01, ds + 0.02, zt],
                0.004,
            );
        }
    }
    till(b, x0 + 0.45, d * 0.45, h);
    // A small stand of goods at the other end.
    goods(b, x1 - 0.5, x1 - 0.05, d * 0.25, d * 0.5, h, 0.2, 0, 3);
}

// -------------------------------------------------------------- shelving

/// Wall shelving of the collections.
pub fn shop_shelving(b: &mut Builder, p: &Params) {
    let ix = look(p);
    let w = p.get("width", 1.2).clamp(0.6, 4.0);
    let d = p.get("depth", 0.4).clamp(0.3, 0.6);
    let h = p.get("height", 2.2).clamp(1.4, 3.0);
    let (x0, x1) = (-w / 2.0, w / 2.0);
    let first = 0.12;
    let levels = ((h - first - 0.1) / 0.38).floor().clamp(2.0, 7.0) as usize;
    let step = (h - first - 0.1) / levels as f64;
    let zs: Vec<f64> = (0..levels).map(|i| first + step * i as f64).collect();
    let room = step - 0.05;
    let mut seed = 0u32;
    let mut stock = |b: &mut Builder, xa: f64, xe: f64, z: f64| {
        seed += 1;
        goods(
            b,
            xa,
            xe,
            0.02,
            d - 0.06,
            z,
            room,
            if seed % 3 == 0 { 1 } else { 0 },
            seed,
        );
    };
    match ix {
        // Nordic: oak sides and shelves, a pale back.
        0 => {
            b.rbox("front", [x0, 0.0, 0.0], [x1, 0.015, h], 0.0);
            for x in [x0, x1 - 0.025] {
                b.rbox("frame", [x, 0.0, 0.0], [x + 0.025, d, h], 0.004);
            }
            for z in &zs {
                b.rbox(
                    "frame",
                    [x0 + 0.025, 0.015, z - 0.025],
                    [x1 - 0.025, d, *z],
                    0.003,
                );
                stock(b, x0 + 0.03, x1 - 0.03, *z);
            }
            b.rbox("frame", [x0, 0.0, h - 0.025], [x1, d, h], 0.004);
        }
        // Mediterranean: whitewashed niches with round heads.
        1 => {
            b.rbox("masonry", [x0, 0.0, 0.0], [x1, 0.03, h], 0.0);
            let pier = 0.1;
            let k = ((w - pier) / 0.7).round().max(1.0) as usize;
            let nw = (w - pier * (k + 1) as f64) / k as f64;
            let zs_top = h - 0.12 - nw / 2.0;
            for i in 0..=k {
                let x = x0 + (pier + nw) * i as f64;
                b.rbox("masonry", [x, 0.03, 0.0], [x + pier, d, h], 0.01);
            }
            b.rbox("masonry", [x0, 0.03, 0.0], [x1, d, first], 0.01);
            for i in 0..k {
                let xa = x0 + pier + (pier + nw) * i as f64;
                arch_head(b, "masonry", xa, xa + nw, zs_top, h, 0.03, d);
                for z in zs.iter().filter(|z| **z < zs_top - 0.15) {
                    b.rbox(
                        "frame",
                        [xa, 0.03, z - 0.035],
                        [xa + nw, d - 0.02, *z],
                        0.006,
                    );
                    stock(b, xa + 0.01, xa + nw - 0.01, *z);
                }
            }
        }
        // Fifties: teak shelves on brass posts, a coloured back.
        2 => {
            b.rbox("accent", [x0, 0.0, 0.1], [x1, 0.015, h], 0.0);
            for x in [x0 + 0.04, x1 - 0.04] {
                for y in [0.06, d - 0.04] {
                    b.cyl("metal", [x, y, 0.0], [x, y, h], 0.012);
                }
            }
            for z in &zs {
                b.rbox("frame", [x0, 0.015, z - 0.025], [x1, d, *z], 0.008);
                stock(b, x0 + 0.06, x1 - 0.06, *z);
            }
        }
        // Japandi: a grid of cubbies of smoked oak.
        3 => {
            b.rbox("dark", [x0, 0.0, 0.0], [x1, 0.015, h], 0.0);
            let cols = (w / 0.45).round().max(1.0) as usize;
            for i in 0..=cols {
                let x = x0 + (w - 0.025) * i as f64 / cols as f64;
                b.rbox("frame", [x, 0.015, 0.0], [x + 0.025, d, h], 0.0);
            }
            for z in zs.iter().chain([0.0, h - 0.025].iter()) {
                let z = if *z > 0.0 && *z < h - 0.03 {
                    z - 0.025
                } else {
                    *z
                };
                b.rbox("frame", [x0, 0.015, z], [x1, d, z + 0.025], 0.0);
            }
            for (k, z) in zs.iter().enumerate() {
                for i in 0..cols {
                    if (i + k) % 2 == 1 {
                        continue;
                    }
                    let xa = x0 + 0.025 + (w - 0.025) * i as f64 / cols as f64;
                    let xe = x0 + (w - 0.025) * (i + 1) as f64 / cols as f64;
                    stock(b, xa, xe, *z);
                }
            }
        }
        // Contemporary: white shelves, a light under each.
        4 => {
            b.rbox("front", [x0, 0.0, 0.0], [x1, 0.02, h], 0.0);
            for x in [x0, x1 - 0.02] {
                b.rbox("front", [x, 0.0, 0.0], [x + 0.02, d, h], 0.0);
            }
            b.rbox(
                "dark",
                [x0 + 0.02, 0.02, 0.0],
                [x1 - 0.02, d - 0.04, first - 0.02],
                0.0,
            );
            for z in &zs {
                b.rbox(
                    "front",
                    [x0 + 0.02, 0.02, z - 0.02],
                    [x1 - 0.02, d, *z],
                    0.0,
                );
                b.rbox(
                    "light",
                    [x0 + 0.04, d - 0.03, z - 0.023],
                    [x1 - 0.04, d - 0.015, z - 0.02],
                    0.0,
                );
                stock(b, x0 + 0.03, x1 - 0.03, *z);
            }
            b.rbox("front", [x0, 0.0, h - 0.02], [x1, d, h], 0.0);
        }
        // Rustic: thick planks on iron brackets against lime.
        5 => {
            b.rbox("masonry", [x0, 0.0, 0.0], [x1, 0.02, h], 0.0);
            let k = ((w / 0.8).ceil() as usize + 1).max(2);
            for z in &zs {
                b.rbox(
                    "frame",
                    [x0 + 0.02, 0.02, z - 0.05],
                    [x1 - 0.02, d, *z],
                    0.008,
                );
                if *z < 0.3 {
                    b.rbox(
                        "frame",
                        [x0 + 0.04, 0.02, 0.0],
                        [x1 - 0.04, d - 0.04, z - 0.05],
                        0.004,
                    );
                }
                for i in (0..k).filter(|_| *z >= 0.3) {
                    let x = x0 + 0.12 + (w - 0.24) * i as f64 / (k - 1) as f64;
                    b.rbox(
                        "metal",
                        [x - 0.006, 0.02, z - 0.06],
                        [x + 0.006, d - 0.04, z - 0.05],
                        0.0,
                    );
                    b.rbox(
                        "metal",
                        [x - 0.006, 0.02, z - 0.24],
                        [x + 0.006, 0.03, z - 0.05],
                        0.0,
                    );
                    bar(
                        b,
                        "metal",
                        [x, 0.03, z - 0.21],
                        [x, d - 0.08, z - 0.055],
                        0.01,
                        [1.0, 0.0, 0.0],
                    );
                }
                stock(b, x0 + 0.04, x1 - 0.04, *z);
            }
        }
        // Industrial: planks on posts of black pipe.
        6 => {
            let k = ((w / 0.9).ceil() as usize + 1).max(2);
            for i in 0..k {
                let x = x0 + 0.04 + (w - 0.08) * i as f64 / (k - 1) as f64;
                for y in [0.04, d - 0.04] {
                    b.cyl("dark", [x, y, 0.0], [x, y, h], 0.016);
                    disc(b, "dark", [x, y], 0.035, 0.0, 0.01);
                }
            }
            for z in &zs {
                b.rbox("frame", [x0, 0.0, z - 0.04], [x1, d, *z], 0.004);
                stock(b, x0 + 0.06, x1 - 0.06, *z);
            }
            b.rbox("frame", [x0, 0.0, h - 0.04], [x1, d, h], 0.004);
        }
        // Classic: walnut with pilasters, a mirrored back and a cornice.
        _ => {
            b.rbox("frame", [x0, 0.0, 0.0], [x1, 0.02, h], 0.0);
            b.rbox(
                "mirror",
                [x0 + 0.1, 0.02, first],
                [x1 - 0.1, 0.024, h - 0.2],
                0.0,
            );
            for x in [x0, x1 - 0.08] {
                b.rbox(
                    "frame",
                    [x, 0.0, 0.0],
                    [x + 0.08, d + 0.02, h - 0.12],
                    0.004,
                );
            }
            b.rbox("frame", [x0, 0.0, 0.0], [x1, d + 0.02, first], 0.004);
            for z in zs.iter().filter(|z| **z < h - 0.45) {
                b.rbox(
                    "frame",
                    [x0 + 0.08, 0.02, z - 0.03],
                    [x1 - 0.08, d, *z],
                    0.006,
                );
                stock(b, x0 + 0.1, x1 - 0.1, *z);
            }
            for (k, (dz, o)) in [(0.0, 0.05), (0.04, 0.035), (0.07, 0.015)]
                .into_iter()
                .enumerate()
            {
                let t = if k == 0 { 0.05 } else { 0.03 };
                b.rbox(
                    "frame",
                    [x0 - o, 0.0, h - dz - t],
                    [x1 + o, d + 0.02 + o, h - dz],
                    0.006,
                );
            }
        }
    }
}

// ---------------------------------------------------------- display table

/// Display tables of the collections, with folded clothes and boxes on
/// them and on the shelf below.
pub fn display_table(b: &mut Builder, p: &Params) {
    let ix = look(p);
    let w = p.get("width", 1.4).clamp(0.8, 3.0);
    let d = p.get("depth", 0.8).clamp(0.5, 1.2);
    let h = p.get("height", 0.8).clamp(0.6, 1.0);
    let (x0, x1) = (-w / 2.0, w / 2.0);
    let t = 0.035;
    let zt = h - t;
    let shelf = 0.2;
    match ix {
        0 => {
            b.rbox("top", [x0, 0.0, zt], [x1, d, h], 0.01);
            for (x, y, dx, dy) in [
                (x0 + 0.08, 0.08, -0.5, -0.5),
                (x1 - 0.08, 0.08, 0.5, -0.5),
                (x1 - 0.08, d - 0.08, 0.5, 0.5),
                (x0 + 0.08, d - 0.08, -0.5, 0.5),
            ] {
                tapered_leg(b, "frame", [x, y], zt, [dx, dy], 0.04, 0.022);
            }
            b.rbox(
                "frame",
                [x0 + 0.1, 0.1, shelf],
                [x1 - 0.1, d - 0.1, shelf + 0.02],
                0.004,
            );
        }
        1 => {
            b.rbox("top", [x0, 0.0, zt], [x1, d, h], 0.006);
            for (x, y) in [
                (x0 + 0.05, 0.05),
                (x1 - 0.05, 0.05),
                (x1 - 0.05, d - 0.05),
                (x0 + 0.05, d - 0.05),
            ] {
                turned_leg(b, x, y, zt, 0.028);
            }
            b.rbox(
                "frame",
                [x0 + 0.05, 0.05, shelf],
                [x1 - 0.05, d - 0.05, shelf + 0.025],
                0.004,
            );
        }
        2 => {
            b.rbox("top", [x0, 0.0, zt], [x1, d, h], 0.015);
            for (x, y, dx, dy) in [
                (x0 + 0.12, 0.1, -0.5, -0.5),
                (x1 - 0.12, 0.1, 0.5, -0.5),
                (x1 - 0.12, d - 0.1, 0.5, 0.5),
                (x0 + 0.12, d - 0.1, -0.5, 0.5),
            ] {
                compass_leg(b, [x, y], zt, [dx, dy], 0.06);
            }
            b.rbox(
                "frame",
                [x0 + 0.12, 0.12, shelf],
                [x1 - 0.12, d - 0.12, shelf + 0.02],
                0.006,
            );
        }
        3 => {
            b.rbox("top", [x0, 0.0, zt], [x1, d, h], 0.004);
            for x in [x0 + 0.06, x1 - 0.1] {
                b.rbox("frame", [x, 0.04, 0.0], [x + 0.04, d - 0.04, zt], 0.004);
            }
            b.rbox(
                "frame",
                [x0 + 0.1, 0.06, shelf],
                [x1 - 0.1, d - 0.06, shelf + 0.025],
                0.003,
            );
        }
        4 => {
            // A white block, a second lower table nested beside it.
            b.rbox("front", [x0, 0.0, 0.0], [x1 - 0.25, d, h], 0.004);
            b.rbox(
                "front",
                [x1 - 0.6, 0.12, 0.0],
                [x1, d - 0.12, h * 0.65],
                0.004,
            );
            goods(
                b,
                x1 - 0.55,
                x1 - 0.05,
                0.12,
                d - 0.3,
                h * 0.65,
                0.25,
                1,
                21,
            );
        }
        5 => {
            b.rbox("top", [x0, 0.0, zt - 0.02], [x1, d, h], 0.008);
            for x in [x0 + 0.1, x1 - 0.18] {
                bar(
                    b,
                    "frame",
                    [x + 0.04, 0.06, 0.02],
                    [x + 0.04, d / 2.0 - 0.05, zt - 0.06],
                    0.06,
                    [1.0, 0.0, 0.0],
                );
                bar(
                    b,
                    "frame",
                    [x + 0.04, d - 0.06, 0.02],
                    [x + 0.04, d / 2.0 + 0.05, zt - 0.06],
                    0.06,
                    [1.0, 0.0, 0.0],
                );
                b.rbox(
                    "frame",
                    [x, 0.06, zt - 0.08],
                    [x + 0.08, d - 0.06, zt - 0.02],
                    0.006,
                );
                b.rbox(
                    "frame",
                    [x - 0.01, 0.03, 0.0],
                    [x + 0.09, d - 0.03, 0.05],
                    0.006,
                );
            }
            b.rbox(
                "frame",
                [x0 + 0.1, d / 2.0 - 0.03, 0.2],
                [x1 - 0.1, d / 2.0 + 0.03, 0.27],
                0.006,
            );
        }
        6 => {
            b.rbox("top", [x0, 0.0, zt], [x1, d, h], 0.004);
            let s = 0.04;
            for (x, y) in [
                (x0 + 0.02, 0.02),
                (x1 - 0.02 - s, 0.02),
                (x1 - 0.02 - s, d - 0.02 - s),
                (x0 + 0.02, d - 0.02 - s),
            ] {
                b.rbox("dark", [x, y, 0.0], [x + s, y + s, zt], 0.0);
            }
            b.rbox(
                "dark",
                [x0 + 0.02, 0.02, shelf],
                [x1 - 0.02, d - 0.02, shelf + 0.02],
                0.0,
            );
            b.rbox(
                "dark",
                [x0 + 0.02, 0.02, zt - 0.04],
                [x1 - 0.02, d - 0.02, zt],
                0.0,
            );
        }
        _ => {
            b.rbox("top", [x0, 0.0, zt], [x1, d, h - 0.004], 0.006);
            b.rbox(
                "accent",
                [x0 + 0.04, 0.04, h - 0.004],
                [x1 - 0.04, d - 0.04, h],
                0.0,
            );
            b.rbox(
                "frame",
                [x0 + 0.04, 0.04, zt - 0.1],
                [x1 - 0.04, d - 0.04, zt],
                0.004,
            );
            for (x, y) in [
                (x0 + 0.07, 0.07),
                (x1 - 0.07, 0.07),
                (x1 - 0.07, d - 0.07),
                (x0 + 0.07, d - 0.07),
            ] {
                let r = 0.025 * std::f64::consts::SQRT_2;
                turned(
                    b,
                    "frame",
                    [x, y, 0.05],
                    &[
                        (0.0, 0.0),
                        (r * 0.65, 0.0),
                        (r, zt - 0.15),
                        (0.0, zt - 0.15),
                    ],
                    4,
                    FRAC_PI_4,
                );
                turned(
                    b,
                    "metal",
                    [x, y, 0.0],
                    &[(0.0, 0.0), (r * 0.66, 0.0), (r * 0.66, 0.05), (0.0, 0.05)],
                    4,
                    FRAC_PI_4,
                );
            }
        }
    }
    // The goods on top, and on the shelf below where there is one.
    goods(
        b,
        x0 + 0.05,
        if ix == 4 { x1 - 0.3 } else { x1 - 0.05 },
        0.08,
        d - 0.16,
        h,
        0.25,
        1,
        11 + ix as u32,
    );
    if matches!(ix, 0 | 1 | 2 | 3 | 6) {
        goods(
            b,
            x0 + 0.15,
            x1 - 0.15,
            0.1,
            d - 0.2,
            shelf + 0.025,
            0.3,
            0,
            31 + ix as u32,
        );
    }
}

// ------------------------------------------------------------ clothes rail

/// Clothes rails of the collections, with their garments.
pub fn clothes_rail(b: &mut Builder, p: &Params) {
    let ix = look(p);
    let w = p.get("width", 1.2).clamp(0.6, 3.0);
    let d = p.get("depth", 0.5).clamp(0.35, 0.8);
    let h = p.get("height", 1.6).clamp(1.2, 2.0);
    let (x0, x1) = (-w / 2.0, w / 2.0);
    let y = d / 2.0;
    let rail = h - 0.03;
    let bar_rod = |b: &mut Builder, role: &'static str, r: f64| {
        sweep(
            b,
            role,
            &[[x0 + 0.02, y, rail], [x1 - 0.02, y, rail]],
            |_| r,
            12,
            Cap::Round(1.0),
            Cap::Round(1.0),
        );
    };
    match ix {
        // Nordic: an oak A-frame at each end and an oak rail.
        0 => {
            for x in [x0 + 0.04, x1 - 0.04] {
                for yf in [0.02, d - 0.02] {
                    bar(
                        b,
                        "frame",
                        [x, yf, 0.03],
                        [x, y, rail + 0.03],
                        0.035,
                        [1.0, 0.0, 0.0],
                    );
                    b.rbox(
                        "frame",
                        [x - 0.0175, yf - 0.03, 0.0],
                        [x + 0.0175, yf + 0.03, 0.03],
                        0.004,
                    );
                }
            }
            bar_rod(b, "frame", 0.016);
        }
        // Mediterranean: wrought iron with curls at the top.
        1 => {
            for x in [x0 + 0.04, x1 - 0.04] {
                rod(b, "metal", [x, y, 0.03], [x, y, rail], 0.01);
                b.rbox(
                    "metal",
                    [x - 0.01, 0.02, 0.0],
                    [x + 0.01, d - 0.02, 0.03],
                    0.004,
                );
                let s = if x < 0.0 { -1.0 } else { 1.0 };
                let curl: Vec<P3> = (0..=16)
                    .map(|k| {
                        let a = PI * 1.6 * k as f64 / 16.0;
                        let r = 0.06 * (1.0 - 0.6 * k as f64 / 16.0);
                        [x + s * (0.06 - r * a.cos()), y, rail + r * a.sin()]
                    })
                    .collect();
                sweep(
                    b,
                    "metal",
                    &curl,
                    |_| 0.007,
                    6,
                    Cap::Round(1.0),
                    Cap::Round(1.0),
                );
            }
            bar_rod(b, "metal", 0.011);
        }
        // Fifties: brass tubes and a teak shelf below for shoes.
        2 => {
            for x in [x0 + 0.04, x1 - 0.04] {
                rod(b, "metal", [x, y, 0.04], [x, y, rail], 0.012);
                b.rbox(
                    "metal",
                    [x - 0.012, 0.02, 0.0],
                    [x + 0.012, d - 0.02, 0.04],
                    0.006,
                );
            }
            b.rbox(
                "frame",
                [x0 + 0.04, 0.05, 0.2],
                [x1 - 0.04, d - 0.05, 0.225],
                0.006,
            );
            bar_rod(b, "metal", 0.013);
            goods(b, x0 + 0.08, x1 - 0.08, 0.06, d - 0.12, 0.225, 0.15, 0, 41);
        }
        // Japandi: a gate of smoked oak, the top beam reaching over.
        3 => {
            for x in [x0 + 0.06, x1 - 0.06] {
                b.rbox(
                    "frame",
                    [x - 0.02, y - 0.02, 0.0],
                    [x + 0.02, y + 0.02, rail + 0.05],
                    0.004,
                );
                b.rbox(
                    "frame",
                    [x - 0.02, 0.04, 0.0],
                    [x + 0.02, d - 0.04, 0.04],
                    0.004,
                );
            }
            b.rbox(
                "frame",
                [x0, y - 0.025, rail + 0.05],
                [x1, y + 0.025, rail + 0.09],
                0.004,
            );
            bar_rod(b, "frame", 0.014);
        }
        // Contemporary: thin black steel, a rail hung from a single frame.
        4 => {
            for x in [x0 + 0.02, x1 - 0.02] {
                b.rbox(
                    "dark",
                    [x - 0.01, y - 0.01, 0.0],
                    [x + 0.01, y + 0.01, rail + 0.02],
                    0.0,
                );
                b.rbox(
                    "dark",
                    [x - 0.01, 0.02, 0.0],
                    [x + 0.01, d - 0.02, 0.012],
                    0.0,
                );
            }
            b.rbox(
                "dark",
                [x0 + 0.02, y - 0.01, rail - 0.01],
                [x1 - 0.02, y + 0.01, rail + 0.01],
                0.0,
            );
        }
        // Rustic: two trunks of wood and a pole between them.
        5 => {
            for x in [x0 + 0.05, x1 - 0.05] {
                turned(
                    b,
                    "frame",
                    [x, y, 0.0],
                    &[
                        (0.0, 0.0),
                        (0.04, 0.0),
                        (0.035, rail * 0.5),
                        (0.032, rail + 0.05),
                        (0.0, rail + 0.05),
                    ],
                    10,
                    0.0,
                );
                b.rbox(
                    "frame",
                    [x - 0.04, 0.03, 0.0],
                    [x + 0.04, d - 0.03, 0.06],
                    0.01,
                );
            }
            bar_rod(b, "frame", 0.022);
        }
        // Industrial: black pipe on castors.
        6 => {
            for x in [x0 + 0.03, x1 - 0.03] {
                b.cyl("dark", [x, y, 0.08], [x, y, rail], 0.016);
                b.cyl("dark", [x, 0.04, 0.08], [x, d - 0.04, 0.08], 0.016);
                for yy in [0.04, d - 0.04] {
                    b.ellipsoid("dark", [x, yy, 0.035], [0.02, 0.035, 0.035]);
                }
            }
            bar_rod(b, "dark", 0.016);
        }
        // Classic: brass with turned finials on a walnut base.
        _ => {
            b.rbox("frame", [x0, 0.02, 0.0], [x1, d - 0.02, 0.08], 0.01);
            for x in [x0 + 0.05, x1 - 0.05] {
                rod(b, "metal", [x, y, 0.08], [x, y, rail + 0.02], 0.011);
                b.ellipsoid("metal", [x, y, rail + 0.05], [0.025, 0.025, 0.03]);
            }
            for (x, yy) in [
                (x0 + 0.06, 0.06),
                (x1 - 0.06, 0.06),
                (x1 - 0.06, d - 0.06),
                (x0 + 0.06, d - 0.06),
            ] {
                b.ellipsoid("frame", [x, yy, 0.0 + 0.02], [0.02, 0.02, 0.02]);
            }
            bar_rod(b, "metal", 0.012);
        }
    }
    garments(
        b,
        x0 + 0.06,
        x1 - 0.06,
        y,
        rail,
        rail - 0.25,
        50 + ix as u32,
    );
}

// ------------------------------------------------------------ fitting room

/// Fitting rooms of the collections: a back with its mirror and hooks, two
/// sides, a curtain half drawn on the front and a stool inside.
pub fn fitting_room(b: &mut Builder, p: &Params) {
    let ix = look(p);
    let w = p.get("width", 1.0).clamp(0.8, 2.0);
    let d = p.get("depth", 1.0).clamp(0.8, 2.0);
    let h = p.get("height", 2.1).clamp(1.9, 2.6);
    let (x0, x1) = (-w / 2.0, w / 2.0);
    let wall = 0.04;
    let side: &'static str = match ix {
        1 => "masonry",
        4 => "front",
        6 => "dark",
        _ => "frame",
    };
    // The back and the sides.
    b.rbox(side, [x0, 0.0, 0.0], [x1, wall, h], 0.0);
    for (a, e) in [(x0, x0 + wall), (x1 - wall, x1)] {
        b.rbox(side, [a, 0.0, 0.0], [e, d, h], 0.004);
    }
    // The mirror and the hooks.
    b.rbox(
        "mirror",
        [-0.25, wall, 0.3],
        [0.25, wall + 0.006, 1.9_f64.min(h - 0.1)],
        0.0,
    );
    for x in [x0 + 0.15, x1 - 0.15] {
        let start = b.mark();
        turned(
            b,
            "metal",
            [0.0; 3],
            &[
                (0.0, 0.0),
                (0.012, 0.0),
                (0.008, 0.05),
                (0.012, 0.06),
                (0.0, 0.06),
            ],
            10,
            0.0,
        );
        b.transform_from(start, |q| [x + q[0], wall + q[2], 1.6 - q[1]]);
    }
    // A stool against a side.
    disc(b, "top", [x1 - 0.22, d * 0.45], 0.16, 0.42, 0.04);
    b.cyl(
        "metal",
        [x1 - 0.22, d * 0.45, 0.0],
        [x1 - 0.22, d * 0.45, 0.42],
        0.02,
    );
    disc(b, "metal", [x1 - 0.22, d * 0.45], 0.14, 0.0, 0.012);
    // The curtain rod and the curtain, half drawn to one side.
    let rod_z = h - 0.08;
    let rod_role = if matches!(ix, 2 | 7) { "metal" } else { "dark" };
    rod(
        b,
        rod_role,
        [x0 + wall, d - 0.05, rod_z],
        [x1 - wall, d - 0.05, rod_z],
        0.012,
    );
    curtain(
        b,
        x0 + wall + 0.02,
        x0 + wall + (w - 2.0 * wall) * 0.45,
        d - 0.05,
        0.04,
        rod_z - 0.02,
    );
    // The collection's way with its walls.
    match ix {
        0 => {
            for x in [x0, x1 - wall] {
                b.rbox(
                    "frame",
                    [x - 0.005, d - 0.04, 0.0],
                    [x + wall + 0.005, d, h],
                    0.004,
                );
            }
            b.rbox("frame", [x0, d - 0.04, h - 0.06], [x1, d, h], 0.004);
        }
        1 => {
            let tile = 0.1;
            let k = ((w - 2.0 * wall) / tile).floor() as usize;
            for i in 0..k {
                let role = if i % 2 == 0 { "accent" } else { "masonry" };
                let xa = -(k as f64) * tile / 2.0 + tile * i as f64;
                b.rbox(
                    role,
                    [xa + 0.002, wall, 0.0],
                    [xa + tile - 0.002, wall + 0.008, tile],
                    0.0,
                );
            }
        }
        2 => {
            for x in [x0, x1 - wall] {
                b.rbox(
                    "metal",
                    [x, d - 0.01, 0.0],
                    [x + wall, d + 0.004, 0.1],
                    0.003,
                );
            }
            b.rbox(
                "accent",
                [-0.3, wall, 0.25],
                [0.3, wall + 0.004, 1.95_f64.min(h - 0.05)],
                0.02,
            );
        }
        3 => {
            for s in [x0, x1 - wall] {
                let n = ((d - 0.04) / 0.06).floor() as usize;
                for i in 0..n {
                    let yy = 0.03 + (d - 0.04) * i as f64 / n as f64;
                    let x = if s < 0.0 { s + wall } else { s - 0.012 };
                    b.rbox("frame", [x, yy, 0.0], [x + 0.012, yy + 0.03, h], 0.0);
                }
            }
        }
        4 => {
            b.rbox(
                "light",
                [-0.27, wall + 0.006, 1.9_f64.min(h - 0.1)],
                [0.27, wall + 0.012, 1.92_f64.min(h - 0.08)],
                0.0,
            );
        }
        5 => {
            for s in [x0, x1 - wall] {
                let n = ((d - 0.02) / 0.18).round() as usize;
                for i in 0..n {
                    let yy = (d - 0.02) * i as f64 / n as f64;
                    let x = if s < 0.0 { s - 0.008 } else { s + wall };
                    b.rbox(
                        "frame",
                        [x, yy + 0.003, 0.0],
                        [x + 0.008, yy + (d - 0.02) / n as f64 - 0.003, h],
                        0.002,
                    );
                }
            }
        }
        6 => {
            for s in [x0, x1 - wall] {
                felt(
                    b,
                    "front",
                    [s + 0.004, 0.06, 0.08],
                    [s + wall - 0.004, d - 0.06, h - 0.08],
                );
            }
        }
        _ => {
            // Raised panels on the insides of the two sides: made facing
            // `+y`, then turned a quarter to face across the room.
            let start = b.mark();
            front(b, Panel::Raised, "frame", 0.08 - d, -0.08, 0.15, 0.95, 0.0);
            front(
                b,
                Panel::Raised,
                "frame",
                0.08 - d,
                -0.08,
                1.05,
                h - 0.15,
                0.0,
            );
            b.transform_from(start, |q| [x0 + wall + q[1], -q[0], q[2]]);
            let start = b.mark();
            front(b, Panel::Raised, "frame", 0.08, d - 0.08, 0.15, 0.95, 0.0);
            front(
                b,
                Panel::Raised,
                "frame",
                0.08,
                d - 0.08,
                1.05,
                h - 0.15,
                0.0,
            );
            b.transform_from(start, |q| [x1 - wall - q[1], q[0], q[2]]);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fine::shape::signed_volume;

    #[test]
    fn the_shops_of_the_eight_collections() {
        type Make = fn(&mut Builder, &Params);
        let pieces: [(&str, Make); 5] = [
            ("mostrador", shop_counter),
            ("estantería", shop_shelving),
            ("mesa", display_table),
            ("burro", clothes_rail),
            ("probador", fitting_room),
        ];
        for c in 0..8 {
            for (name, make) in pieces {
                let mut p = Params::default();
                p.set("style", (1 + c) as f64);
                let mut b = Builder::default();
                make(&mut b, &p);
                let tris = b.triangle_count();
                assert!((100..40_000).contains(&tris), "{c} {name}: {tris}");
                for (role, t) in &b.parts {
                    let v = signed_volume(&b.positions, t);
                    assert!(v > -1e-7, "{c} {name} {role}: {v}");
                }
                let low = b.positions.iter().map(|q| q[2]).fold(f32::MAX, f32::min);
                assert!(low > -0.001, "{c} {name}: {low}");
            }
        }
    }
}
