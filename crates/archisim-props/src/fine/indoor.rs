//! The house plants of the eight collections (paquete 6, tanda 6a): each
//! collection's pot (on its stand where it has one) with its own plant,
//! leaf by leaf as the workshop's peace lily:
//!
//! - Nordic warm: a fiddle-leaf fig in a white pot on an oak stand;
//! - Mediterranean: a lemon tree in a terracotta pot on its saucer;
//! - fifties: a Swiss cheese plant in a tapered pot on splayed legs;
//! - japandi: a snake plant in a stone pot;
//! - contemporary: a bird of paradise in a tall concrete pot;
//! - rustic: a Boston fern in a wicker basket;
//! - industrial: a pothos trailing from a steel bucket on its stand;
//! - classic: a kentia palm in a glazed jardinière.
//!
//! Values: `height` (all of it, pot and stand included) and `pot` (its
//! diameter); centred on `x = 0`, its back at `y = 0`, as the library's.
//! Roles: `pot`, `frame` (wood), `metal`, `weave`, `soil`, `stem`,
//! `leaves`, `leaves2`, `flowers` and `fruit`.

use super::garden_plants::{self, across, at_along, blade, leafy, wander, Blade, Form};
use super::plant::{bellied, heading, soil_in, stalk, terracotta, Rand};
use super::shape::{add, bar, curve, leaf, mul, rotate, sweep, turned, Cap, Leaf, Outline};
use super::soft::tube;
use super::storage::tapered_leg;
use crate::build::{norm, Builder, P3};
use crate::Params;
use std::f64::consts::{PI, TAU};

/// Sides of a pot.
const SIDES: usize = 28;

/// The collection when the model chosen is one of those listed after the
/// family's `first` own models.
fn look(p: &Params, first: i32) -> Option<usize> {
    let s = p.style() - first;
    (1..=8).contains(&s).then(|| (s - 1) as usize)
}

/// A closed ring of `n` points round `c`, radius `r`, level.
fn ring(c: P3, r: f64, n: usize) -> Vec<P3> {
    (0..n)
        .map(|k| {
            let a = TAU * k as f64 / n as f64;
            [c[0] + r * a.cos(), c[1] + r * a.sin(), c[2]]
        })
        .collect()
}

/// The house plants of the collections (after the family's 5 own models).
pub fn potted_plant(b: &mut Builder, p: &Params) -> bool {
    let Some(ix) = look(p, 5) else {
        return false;
    };
    let h = p.get("height", 1.2).clamp(0.4, 3.0);
    let pot = p.get("pot", 0.36).clamp(0.12, 1.0);
    let r = pot / 2.0;
    let c = [0.0, r, 0.0];
    // The pot, and where its earth is: its height and the radius inside.
    let (soil, ri) = match ix {
        // Nordic: a white pot on a tripod of oak with its ring.
        0 => {
            let stand = (h * 0.22).clamp(0.12, 0.4);
            for k in 0..3 {
                let a = PI / 2.0 + TAU * k as f64 / 3.0;
                tapered_leg(
                    b,
                    "frame",
                    [r * 0.62 * a.cos(), r + r * 0.62 * a.sin()],
                    stand,
                    [a.cos(), a.sin()],
                    r * 0.35,
                    0.013,
                );
            }
            tube(
                b,
                "frame",
                &ring([0.0, r, stand - 0.03], r * 0.66, 32),
                true,
                0.01,
                6,
            );
            let ph = (pot * 0.85).min(h * 0.3);
            turned(
                b,
                "pot",
                [0.0, r, stand - 0.04],
                &[
                    (r * 0.7, 0.0),
                    (r * 0.9, 0.012),
                    (r * 0.98, 0.05),
                    (r, ph * 0.3),
                    (r, ph),
                    (r * 0.9, ph),
                    (r * 0.9, ph - 0.03),
                ],
                SIDES,
                0.0,
            );
            (stand - 0.04 + ph - 0.03, r * 0.9)
        }
        // Mediterranean: terracotta on its saucer.
        1 => {
            let ph = (pot * 0.9).min(h * 0.35);
            terracotta(b, c, r, ph, 0.009);
            (0.009 + ph * 0.9, r * 0.86)
        }
        // Fifties: a tapered pot on three splayed legs.
        2 => {
            let lift = 0.16;
            for k in 0..3 {
                let a = PI / 2.0 + TAU * k as f64 / 3.0;
                tapered_leg(
                    b,
                    "metal",
                    [r * 0.5 * a.cos(), r + r * 0.5 * a.sin()],
                    lift + 0.02,
                    [a.cos(), a.sin()],
                    r * 0.45,
                    0.009,
                );
            }
            let ph = (pot * 0.8).min(h * 0.3);
            turned(
                b,
                "pot",
                [0.0, r, lift],
                &[
                    (r * 0.55, 0.0),
                    (r, ph),
                    (r * 0.92, ph),
                    (r * 0.9, ph - 0.03),
                ],
                SIDES,
                0.0,
            );
            (lift + ph - 0.03, r * 0.88)
        }
        // Japandi: a pot of stone, straight and heavy.
        3 => {
            let ph = (pot * 0.75).min(h * 0.35);
            turned(
                b,
                "pot",
                c,
                &[
                    (r * 0.96, 0.0),
                    (r, 0.015),
                    (r, ph),
                    (r * 0.84, ph),
                    (r * 0.84, ph - 0.03),
                ],
                SIDES,
                0.0,
            );
            (ph - 0.03, r * 0.82)
        }
        // Contemporary: a tall cylinder of concrete.
        4 => {
            let ph = (pot * 1.3).min(h * 0.42);
            turned(
                b,
                "pot",
                c,
                &[(r, 0.0), (r, ph), (r * 0.88, ph), (r * 0.88, ph - 0.035)],
                SIDES,
                0.0,
            );
            (ph - 0.035, r * 0.86)
        }
        // Rustic: a wicker basket with two rope handles, its liner.
        5 => {
            let ph = (pot * 0.8).min(h * 0.4);
            turned(
                b,
                "weave",
                c,
                &[
                    (r * 0.82, 0.0),
                    (r * 0.95, ph * 0.3),
                    (r, ph),
                    (r * 0.92, ph),
                    (r * 0.9, ph - 0.03),
                ],
                SIDES,
                0.0,
            );
            for z in [ph * 0.35, ph * 0.7] {
                let rr = r * (0.84 + 0.13 * z / ph);
                tube(
                    b,
                    "weave",
                    &ring([0.0, r, z], rr + 0.004, 36),
                    true,
                    0.006,
                    5,
                );
            }
            tube(
                b,
                "weave",
                &ring([0.0, r, ph], r + 0.002, 36),
                true,
                0.012,
                6,
            );
            for s in [-1.0, 1.0] {
                let x = s * (r + 0.005);
                let loop_: Vec<P3> = (0..=10)
                    .map(|k| {
                        let a = PI * k as f64 / 10.0;
                        [x + s * 0.035 * a.sin(), r + 0.05 * a.cos(), ph - 0.06]
                    })
                    .collect();
                tube(b, "weave", &loop_, false, 0.007, 6);
            }
            (ph - 0.035, r * 0.88)
        }
        // Industrial: a steel bucket with its ribs and handle, on a stand
        // for its plant to hang from.
        6 => {
            let stand = (h * 0.3).clamp(0.25, 0.6);
            for (x, y) in [(-1.0, -1.0), (1.0, -1.0), (1.0, 1.0), (-1.0, 1.0)] {
                let (px, py) = (x * r * 0.62, r + y * r * 0.62);
                b.rbox(
                    "metal",
                    [px - 0.012, py - 0.012, 0.0],
                    [px + 0.012, py + 0.012, stand],
                    0.0,
                );
            }
            b.rbox(
                "metal",
                [-r * 0.75, r * 0.25, stand - 0.015],
                [r * 0.75, r * 1.75, stand],
                0.0,
            );
            let ph = (pot * 0.75).min(h * 0.25);
            turned(
                b,
                "pot",
                [0.0, r, stand],
                &[
                    (r * 0.8, 0.0),
                    (r, ph),
                    (r * 0.95, ph),
                    (r * 0.93, ph - 0.03),
                ],
                SIDES,
                0.0,
            );
            for k in [0.25, 0.6, 0.98] {
                tube(
                    b,
                    "metal",
                    &ring([0.0, r, stand + ph * k], r * (0.8 + 0.2 * k) + 0.004, 36),
                    true,
                    0.006,
                    5,
                );
            }
            let handle: Vec<P3> = (0..=14)
                .map(|k| {
                    let a = PI * k as f64 / 14.0;
                    [-(r + 0.01) * a.cos(), r, stand + ph * 0.9 + 0.1 * a.sin()]
                })
                .collect();
            tube(b, "metal", &handle, false, 0.004, 5);
            (stand + ph - 0.03, r * 0.9)
        }
        // Classic: a glazed jardinière on its foot, moulded.
        _ => {
            let ph = (pot * 0.85).min(h * 0.35);
            bellied(b, c, r, ph);
            (ph * 0.88, r * 0.8)
        }
    };
    let ground = [0.0, r, soil];
    soil_in(b, ground, ri, 17 + ix as u32);
    let grow = (h - soil).max(0.2);
    match ix {
        0 => fiddle_leaf_fig(b, ground, grow),
        1 => {
            garden_plants::lemon_tree(b, ground, grow, (pot * 2.2).max(grow * 0.6), 9);
        }
        2 => monstera(b, ground, ri, grow),
        3 => snake_plant(b, ground, ri, grow),
        4 => bird_of_paradise(b, ground, ri, grow),
        5 => fern(b, ground, ri, grow),
        6 => pothos(b, ground, ri, grow, soil),
        _ => kentia(b, ground, ri, grow),
    }
    for role in ["leaves", "leaves2", "stem", "flowers", "fruit"] {
        b.soft.insert(role);
    }
    // The plants of many small leaves are drawn without contours.
    if matches!(ix, 1 | 5 | 7) {
        for role in ["leaves", "leaves2", "stem", "fruit"] {
            b.fine.insert(role);
        }
    }
    true
}

/// A fiddle-leaf fig: a slender trunk with a couple of branches, big
/// wavy leaves broad at their tip on short stalks, the young ones up.
fn fiddle_leaf_fig(b: &mut Builder, soil: P3, grow: f64) {
    let mut rnd = Rand(0x1b87_3593);
    let trunk = stalk(
        add(soil, [0.0, 0.0, -0.02]),
        [1.0, 0.0],
        0.12,
        grow * 0.82,
        41,
    );
    sweep(
        b,
        "stem",
        &trunk,
        |t| 0.014 * (1.0 - 0.55 * t) + 0.004,
        8,
        Cap::Flat,
        Cap::Round(1.0),
    );
    let mut stems = vec![trunk.clone()];
    for (k, az) in [(0.55, 2.2), (0.68, -1.0)] {
        let (from, _) = at_along(&trunk, k);
        let out = [f64::cos(az), f64::sin(az)];
        let br = stalk(from, out, 0.55, grow * 0.3, 60 + (az * 10.0) as u32);
        sweep(
            b,
            "stem",
            &br,
            |t| 0.007 * (1.0 - 0.5 * t) + 0.003,
            6,
            Cap::Flat,
            Cap::Round(1.0),
        );
        stems.push(br);
    }
    let mut n = 0u32;
    for (si, st) in stems.iter().enumerate() {
        let leaves = [12, 5, 5][si];
        for j in 0..leaves {
            let k = j as f64 / (leaves - 1) as f64;
            let from = if si == 0 { 0.45 } else { 0.2 };
            let (node, along) = at_along(st, from + (1.0 - from) * k);
            let a = j as f64 * 2.399_963 + si as f64;
            let o = [a.cos(), a.sin(), 0.0];
            let rise = 0.15 + 0.8 * k + rnd.range(-0.15, 0.15);
            let dir = norm(add(
                mul(o, rise.cos()),
                add([0.0, 0.0, rise.sin()], mul(along, 0.3)),
            ));
            let pet = add(node, mul(dir, 0.025));
            sweep(b, "stem", &[node, pet], |_| 0.003, 5, Cap::Flat, Cap::Flat);
            let size = grow * rnd.range(0.2, 0.26) * (1.0 - 0.3 * k);
            let mut l = Leaf::new(size, size * 0.62, Outline::Round);
            l.fold = 0.12;
            l.droop = 0.55 - 0.3 * k;
            l.wave = 0.06;
            l.veins = 7;
            l.vein_depth = size * 0.006;
            l.dome = 0.02;
            l.thick = 0.002;
            l.rows = 10;
            l.cols = 3;
            let role = if k > 0.75 { "leaves2" } else { "leaves" };
            leaf(b, role, pet, dir, [0.0, 0.0, 1.0], &l, 900 + n);
            n += 1;
        }
    }
}

/// A Swiss cheese plant: long stalks arching out of the soil, each
/// holding a big leaf split into lobes from its midrib to its edge.
fn monstera(b: &mut Builder, soil: P3, ri: f64, grow: f64) {
    let mut rnd = Rand(0x2c1b_3c6d);
    let n = 9;
    for i in 0..n {
        let k = i as f64 / (n - 1) as f64;
        let a = i as f64 * 2.399_963;
        let out = [a.cos(), a.sin()];
        let from = [
            soil[0] + out[0] * ri * 0.3,
            soil[1] + out[1] * ri * 0.3,
            soil[2] - 0.01,
        ];
        let lean = 0.25 + 0.7 * k + rnd.range(-0.1, 0.1);
        let len = grow * rnd.range(0.45, 0.6) * (1.0 - 0.25 * k);
        let pts = stalk(from, out, lean, len, 300 + i);
        sweep(
            b,
            "stem",
            &pts,
            |t| 0.006 - 0.002 * t,
            6,
            Cap::Flat,
            Cap::Flat,
        );
        let top = pts[pts.len() - 1];
        let o = [out[0], out[1], 0.0];
        // The blade held out and a little down, from the stalk's end.
        let tilt = 0.35 + 0.3 * k;
        let dir = norm(add(mul(o, tilt.cos()), [0.0, 0.0, -tilt.sin() * 0.4]));
        let size = grow * rnd.range(0.32, 0.4) * (1.0 - 0.25 * (1.0 - k));
        split_leaf(b, top, dir, size, rnd.range(-0.3, 0.3), 700 + i * 20);
    }
}

/// A big leaf split into lobes: a midrib of `size` along `dir`, a lobe
/// either side at each of its nodes reaching out and forward, a rounded
/// one at the tip and two at the base that make its heart.
fn split_leaf(b: &mut Builder, base: P3, dir: P3, size: f64, twist: f64, seed: u32) {
    let side0 = rotate(across(dir), dir, twist);
    let up = norm(crate::build::cross(dir, side0));
    let up = if up[2] < 0.0 { mul(up, -1.0) } else { up };
    let side = norm(crate::build::cross(up, dir));
    let rib: Vec<P3> = (0..=6)
        .map(|k| {
            let u = k as f64 / 6.0;
            add(add(base, mul(dir, size * u)), mul(up, -size * 0.18 * u * u))
        })
        .collect();
    sweep(
        b,
        "stem",
        &rib,
        |t| 0.004 * (1.0 - 0.6 * t),
        5,
        Cap::Flat,
        Cap::Round(1.0),
    );
    let lobes = 5;
    let mut s = 0u32;
    for k in 0..=lobes {
        let u = k as f64 / lobes as f64;
        let (q, along) = at_along(&rib, (0.05 + 0.9 * u).min(1.0));
        // Longer in the middle of the leaf, shorter at its ends.
        let reach = size * (0.22 + 0.36 * (PI * (0.15 + 0.8 * u)).sin());
        for sgn in [1.0, -1.0] {
            let lean = if k == 0 { -0.3 } else { 0.55 + 0.25 * u };
            let d = norm(add(mul(side, sgn), mul(along, lean)));
            let mut l = Leaf::new(reach, size * 0.17, Outline::Lance);
            l.fold = 0.1;
            l.droop = 0.35;
            l.wave = 0.03;
            l.thick = 0.0016;
            l.rows = 6;
            l.cols = 2;
            leaf(b, "leaves", q, d, up, &l, seed + s);
            s += 1;
        }
    }
    let (tip, along) = at_along(&rib, 1.0);
    let mut l = Leaf::new(size * 0.22, size * 0.2, Outline::Round);
    l.droop = 0.3;
    l.rows = 5;
    l.cols = 2;
    leaf(
        b,
        "leaves",
        add(tip, mul(along, -size * 0.06)),
        along,
        up,
        &l,
        seed + s,
    );
}

/// A snake plant: stiff sword leaves standing up from the soil, a little
/// twisted, the young ones paler.
fn snake_plant(b: &mut Builder, soil: P3, ri: f64, grow: f64) {
    let mut rnd = Rand(0x7f4a_7c15);
    let n = 15;
    for i in 0..n {
        let k = i as f64 / (n - 1) as f64;
        let a = i as f64 * 2.399_963;
        let d = ri * 0.6 * k.sqrt();
        let from = [soil[0] + a.cos() * d, soil[1] + a.sin() * d, soil[2] - 0.02];
        let lean = 0.04 + 0.28 * k + rnd.range(-0.05, 0.05);
        let dir = norm([a.cos() * lean.sin(), a.sin() * lean.sin(), lean.cos()]);
        let len = grow * rnd.range(0.7, 1.0) * (1.0 - 0.35 * k);
        let mut l = Leaf::new(len, len * rnd.range(0.09, 0.12), Outline::Lance);
        l.fold = 0.35;
        l.droop = rnd.range(-0.05, 0.15);
        l.twist = rnd.range(-0.6, 0.6);
        l.wave = 0.04;
        l.thick = 0.003;
        l.rows = 10;
        l.cols = 2;
        let role = if rnd.next() < 0.35 {
            "leaves2"
        } else {
            "leaves"
        };
        let face = [-a.sin(), a.cos(), 0.0];
        leaf(b, role, from, dir, face, &l, 1100 + i);
    }
}

/// A bird of paradise: long stalks rising from the soil, each with a
/// broad paddle leaf held up and out.
fn bird_of_paradise(b: &mut Builder, soil: P3, ri: f64, grow: f64) {
    let mut rnd = Rand(0x3c37_8f02);
    let n = 9;
    for i in 0..n {
        let k = i as f64 / (n - 1) as f64;
        let a = i as f64 * 2.399_963;
        let out = [a.cos(), a.sin()];
        let from = [
            soil[0] + out[0] * ri * 0.25,
            soil[1] + out[1] * ri * 0.25,
            soil[2] - 0.01,
        ];
        let lean = 0.08 + 0.35 * k + rnd.range(-0.06, 0.06);
        let len = grow * rnd.range(0.5, 0.62) * (1.0 - 0.2 * k);
        let pts = stalk(from, out, lean, len, 400 + i);
        sweep(
            b,
            "stem",
            &pts,
            |t| 0.008 - 0.003 * t,
            6,
            Cap::Flat,
            Cap::Flat,
        );
        let top = pts[pts.len() - 1];
        let dir = heading(&pts);
        let size = grow * rnd.range(0.36, 0.44);
        let mut l = Leaf::new(size, size * 0.34, Outline::Lance);
        l.fold = 0.18;
        l.droop = 0.25 + 0.3 * k;
        l.twist = rnd.range(-0.4, 0.4);
        l.wave = 0.05;
        l.veins = 12;
        l.vein_depth = size * 0.004;
        l.thick = 0.002;
        l.rows = 12;
        l.cols = 3;
        let role = if k > 0.8 { "leaves2" } else { "leaves" };
        leaf(b, role, top, dir, [-out[1], out[0], 0.0], &l, 1300 + i);
    }
}

/// A Boston fern: fronds arching out of the basket and down over its
/// rim, each a stalk with small leaflets in pairs along it.
fn fern(b: &mut Builder, soil: P3, ri: f64, grow: f64) {
    let mut rnd = Rand(0x510e_527f);
    let n = 24;
    let pinna = {
        let mut l = Blade::new(0.05, 0.012, Form::Lance);
        l.droop = 0.2;
        l.fold = 0.15;
        l.rows = 3;
        l
    };
    for i in 0..n {
        let k = i as f64 / (n - 1) as f64;
        let a = i as f64 * 2.399_963;
        let out = [a.cos(), a.sin(), 0.0];
        let from = add(soil, [out[0] * ri * 0.3 * k, out[1] * ri * 0.3 * k, -0.01]);
        let rise = 1.2 - 0.9 * k;
        let len = grow * rnd.range(0.55, 0.75) * (0.85 + 0.3 * k);
        let start = norm(add(mul(out, rise.cos()), [0.0, 0.0, rise.sin()]));
        let frond: Vec<P3> = wander(
            from,
            start,
            add(out, [0.0, 0.0, -1.2]),
            len,
            0.85,
            8,
            500 + i,
        )
        .into_iter()
        .map(|q| [q[0], q[1], q[2].max(0.04)])
        .collect();
        sweep(
            b,
            "stem",
            &frond,
            |t| 0.0025 * (1.0 - 0.7 * t),
            4,
            Cap::Flat,
            Cap::Flat,
        );
        let mut leafl = pinna;
        leafl.length = (0.03 + 0.03 * grow.min(1.0)).min(0.06);
        leafy(
            b,
            &mut rnd,
            &frond,
            0.12,
            18,
            leafl,
            ["leaves", "leaves2"],
            0.25,
        );
    }
}

/// A pothos: a few leaves standing in the middle and vines trailing over
/// the rim and down the stand, heart leaves along them.
fn pothos(b: &mut Builder, soil: P3, ri: f64, grow: f64, above: f64) {
    let mut rnd = Rand(0x6b2f_ae81);
    let heart = {
        let mut l = Blade::new(0.075, 0.055, Form::Heart);
        l.droop = 0.4;
        l.fold = 0.2;
        l.rows = 4;
        l
    };
    // The cluster in the middle.
    for i in 0..10 {
        let a = i as f64 * 2.399_963;
        let out = [a.cos(), a.sin()];
        let from = add(soil, [out[0] * ri * 0.3, out[1] * ri * 0.3, -0.01]);
        let pts = stalk(from, out, 0.6, grow * rnd.range(0.12, 0.2), 600 + i);
        sweep(b, "stem", &pts, |_| 0.0025, 4, Cap::Flat, Cap::Flat);
        let top = pts[pts.len() - 1];
        let mut l = heart;
        l.length *= rnd.range(0.9, 1.3);
        l.width = l.length * 0.75;
        let role = if rnd.next() < 0.4 {
            "leaves2"
        } else {
            "leaves"
        };
        blade(
            b,
            role,
            top,
            norm(add(heading(&pts), [0.0, 0.0, -0.3])),
            [0.0, 0.0, 1.0],
            &l,
        );
    }
    // The vines, over the rim and down.
    let fall = (above - 0.05).max(0.15);
    for i in 0..7 {
        let a = TAU * i as f64 / 7.0 + rnd.range(-0.3, 0.3);
        let out = [a.cos(), a.sin(), 0.0];
        let rim = add(soil, [out[0] * (ri + 0.03), out[1] * (ri + 0.03), 0.04]);
        let down = fall * rnd.range(0.5, 1.0);
        let path = curve(
            &[
                add(soil, [out[0] * ri * 0.4, out[1] * ri * 0.4, -0.005]),
                add(rim, [0.0, 0.0, 0.01]),
                add(rim, [out[0] * 0.06, out[1] * 0.06, -down * 0.3]),
                add(rim, [out[0] * 0.09, out[1] * 0.09, -down]),
            ],
            6,
        );
        sweep(b, "stem", &path, |_| 0.0028, 4, Cap::Flat, Cap::Flat);
        let leaves = (down / 0.07).ceil() as usize + 2;
        for j in 0..leaves {
            let (q, along) = at_along(&path, 0.15 + 0.85 * j as f64 / leaves as f64);
            let s = if j % 2 == 0 { 1.0 } else { -1.0 };
            let side = rotate(across(along), along, s * 1.2);
            let d = norm(add(mul(out, 0.9), add(mul(side, 0.6), [0.0, 0.0, -0.2])));
            let mut l = heart;
            l.length *= rnd.range(0.8, 1.15);
            l.width = l.length * 0.75;
            let role = if rnd.next() < 0.4 {
                "leaves2"
            } else {
                "leaves"
            };
            blade(b, role, q, d, [0.0, 0.0, 1.0], &l);
        }
    }
}

/// A kentia palm: slender stalks rising and arching, each frond with
/// narrow leaflets in pairs drooping from it.
fn kentia(b: &mut Builder, soil: P3, ri: f64, grow: f64) {
    let mut rnd = Rand(0x1f83_d9ab);
    let leaflet = {
        let mut l = Blade::new(0.22, 0.022, Form::Lance);
        l.droop = 0.9;
        l.fold = 0.3;
        l.rows = 5;
        l
    };
    let n = 8;
    for i in 0..n {
        let k = i as f64 / (n - 1) as f64;
        let a = i as f64 * 2.399_963;
        let out = [a.cos(), a.sin(), 0.0];
        let from = add(soil, [out[0] * ri * 0.25, out[1] * ri * 0.25, -0.01]);
        let rise = 1.35 - 0.6 * k + rnd.range(-0.08, 0.08);
        let len = grow * rnd.range(0.85, 1.0);
        let start = norm(add(mul(out, rise.cos()), [0.0, 0.0, rise.sin()]));
        let frond = wander(
            from,
            start,
            add(out, [0.0, 0.0, -0.6]),
            len,
            0.6,
            9,
            800 + i,
        );
        sweep(
            b,
            "stem",
            &frond,
            |t| 0.006 * (1.0 - 0.7 * t) + 0.0015,
            5,
            Cap::Flat,
            Cap::Flat,
        );
        let mut l = leaflet;
        l.length = (grow * 0.2).clamp(0.12, 0.32);
        l.width = l.length * 0.1;
        leafy(b, &mut rnd, &frond, 0.4, 16, l, ["leaves", "leaves2"], 0.2);
    }
}

// ---------------------------------------------------------------- decor

/// A shape turned round the vertical through `c`: `prof` from its foot
/// up, as `(radius, height)`.
fn pot_shape(b: &mut Builder, role: &'static str, c: P3, prof: &[(f64, f64)]) {
    turned(b, role, c, prof, 24, 0.0);
}

/// A candle with its flame, `r` across and `h` high, standing at `c`.
fn candle(b: &mut Builder, c: P3, r: f64, h: f64) {
    b.cone("candle", c, add(c, [0.0, 0.0, h]), r, r, 16);
    b.cone(
        "dark",
        add(c, [0.0, 0.0, h]),
        add(c, [0.0, 0.0, h + 0.008]),
        0.0012,
        0.0012,
        4,
    );
    b.ellipsoid("flame", add(c, [0.0, 0.0, h + 0.02]), [0.006, 0.006, 0.014]);
}

/// A book lying flat: its corner at `lo`, `w` by `d` and `t` thick,
/// turned `a` about the vertical through its middle; pages pale, covers
/// `role`.
fn book(b: &mut Builder, role: &'static str, c: P3, w: f64, d: f64, t: f64, a: f64) {
    let (s, co) = a.sin_cos();
    b.obox(
        role,
        add(c, [0.0, 0.0, t / 2.0]),
        [co * w / 2.0, s * w / 2.0, 0.0],
        [-s * d / 2.0, co * d / 2.0, 0.0],
        [0.0, 0.0, t / 2.0],
    );
    b.obox(
        "books",
        add(c, add([co * 0.004, s * 0.004, 0.0], [0.0, 0.0, t / 2.0])),
        [co * (w / 2.0 - 0.002), s * (w / 2.0 - 0.002), 0.0],
        [-s * (d / 2.0 - 0.006), co * (d / 2.0 - 0.006), 0.0],
        [0.0, 0.0, t / 2.0 - 0.003],
    );
}

/// Branches standing in a vase at `c` (its mouth), `h` high: a few
/// stalks with small round leaves along them.
fn branches(b: &mut Builder, c: P3, h: f64, n: usize, round: bool, seed: u32) {
    let mut rnd = Rand(0x9b05_688c ^ seed);
    let leafl = {
        let mut l = Blade::new(
            0.03,
            if round { 0.026 } else { 0.012 },
            if round { Form::Heart } else { Form::Lance },
        );
        l.droop = 0.3;
        l.rows = 3;
        l
    };
    for i in 0..n {
        let a = TAU * i as f64 / n as f64 + rnd.range(-0.3, 0.3);
        let out = [a.cos(), a.sin()];
        let pts = stalk(
            add(c, [0.0, 0.0, -0.05]),
            out,
            rnd.range(0.2, 0.5),
            h * rnd.range(0.75, 1.0),
            40 + i as u32,
        );
        sweep(b, "stem", &pts, |_| 0.002, 4, Cap::Flat, Cap::Flat);
        leafy(b, &mut rnd, &pts, 0.35, 7, leafl, ["leaves", "leaves"], 0.0);
    }
}

/// Things on tables and shelves of the collections (after the family's 5
/// own models): standing on the surface they are put on (`z` 0), `width`
/// across, centred on `x` 0, their back at `y` 0; `height` is the tallest
/// piece's.
pub fn decor(b: &mut Builder, p: &Params) -> bool {
    let Some(ix) = look(p, 5) else {
        return false;
    };
    let w = p.get("width", 0.4).clamp(0.2, 1.0);
    let h = p.get("height", 0.45).clamp(0.15, 1.2);
    let d = (w * 0.55).clamp(0.14, 0.35);
    let y = d / 2.0;
    match ix {
        // Nordic: two books, a white vase with eucalyptus, a candle.
        0 => {
            book(b, "books2", [-w * 0.28, y, 0.0], 0.24, 0.17, 0.03, 0.1);
            book(b, "books3", [-w * 0.28, y, 0.03], 0.2, 0.15, 0.025, -0.15);
            let vh = h * 0.45;
            let c = [w * 0.12, y, 0.0];
            pot_shape(
                b,
                "mat",
                c,
                &[
                    (0.0, 0.0),
                    (0.04, 0.0),
                    (0.06, vh * 0.4),
                    (0.045, vh * 0.85),
                    (0.022, vh * 0.95),
                    (0.025, vh),
                    (0.0, vh),
                ],
            );
            branches(b, add(c, [0.0, 0.0, vh]), h - vh, 5, true, 1);
            candle(b, [w * 0.38, y * 0.8, 0.0], 0.025, 0.08);
        }
        // Mediterranean: a clay jug with its spout and handle, and a
        // glazed bowl of lemons.
        1 => {
            let jh = h * 0.7;
            let c = [-w * 0.22, y, 0.0];
            pot_shape(
                b,
                "mat",
                c,
                &[
                    (0.0, 0.0),
                    (0.05, 0.0),
                    (0.085, jh * 0.4),
                    (0.07, jh * 0.75),
                    (0.025, jh * 0.9),
                    (0.03, jh),
                    (0.0, jh),
                ],
            );
            tube(
                b,
                "mat",
                &curve(
                    &[
                        add(c, [0.06, 0.0, jh * 0.7]),
                        add(c, [0.11, 0.0, jh * 0.95]),
                        add(c, [0.02, 0.0, jh * 0.93]),
                    ],
                    5,
                ),
                false,
                0.008,
                6,
            );
            b.cone(
                "mat",
                add(c, [-0.06, 0.0, jh * 0.7]),
                add(c, [-0.1, 0.0, jh * 0.85]),
                0.012,
                0.006,
                8,
            );
            let bc = [w * 0.2, y, 0.0];
            pot_shape(
                b,
                "glaze2",
                bc,
                &[
                    (0.0, 0.0),
                    (0.05, 0.0),
                    (0.12, 0.07),
                    (0.115, 0.075),
                    (0.045, 0.012),
                    (0.0, 0.012),
                ],
            );
            for (dx, dy, dz) in [
                (-0.04, 0.0, 0.04),
                (0.04, 0.02, 0.04),
                (0.0, -0.04, 0.04),
                (0.01, 0.01, 0.085),
            ] {
                b.ellipsoid("fruit", add(bc, [dx, dy, dz]), [0.032, 0.032, 0.04]);
            }
        }
        // Fifties: a teak bowl of fruit and two tall coloured bottles.
        2 => {
            let bc = [-w * 0.18, y, 0.0];
            pot_shape(
                b,
                "frame",
                bc,
                &[
                    (0.0, 0.0),
                    (0.04, 0.0),
                    (0.13, 0.06),
                    (0.125, 0.065),
                    (0.035, 0.01),
                    (0.0, 0.01),
                ],
            );
            for (k, (dx, dy)) in [(-0.05, 0.0), (0.04, 0.03), (0.02, -0.05), (-0.01, 0.05)]
                .into_iter()
                .enumerate()
            {
                let role = if k % 2 == 0 { "fruit" } else { "fruit2" };
                b.ellipsoid(role, add(bc, [dx, dy, 0.045]), [0.036, 0.036, 0.034]);
            }
            for (k, (x, bh)) in [(w * 0.18, h), (w * 0.32, h * 0.75)]
                .into_iter()
                .enumerate()
            {
                let role = if k == 0 { "mat" } else { "glaze2" };
                pot_shape(
                    b,
                    role,
                    [x, y, 0.0],
                    &[
                        (0.0, 0.0),
                        (0.035, 0.0),
                        (0.04, bh * 0.45),
                        (0.012, bh * 0.85),
                        (0.012, bh),
                        (0.0, bh),
                    ],
                );
            }
        }
        // Japandi: a tray with a round teapot and two tea bowls.
        3 => {
            b.rbox(
                "frame",
                [-w * 0.45, y - d * 0.42, 0.0],
                [w * 0.45, y + d * 0.42, 0.015],
                0.0,
            );
            let c = [-w * 0.1, y, 0.015];
            b.ellipsoid("mat", add(c, [0.0, 0.0, 0.055]), [0.07, 0.07, 0.055]);
            b.cone(
                "mat",
                add(c, [0.06, 0.0, 0.05]),
                add(c, [0.11, 0.0, 0.09]),
                0.012,
                0.006,
                8,
            );
            b.ellipsoid("mat", add(c, [0.0, 0.0, 0.112]), [0.012, 0.012, 0.01]);
            tube(
                b,
                "frame",
                &curve(
                    &[
                        add(c, [-0.05, 0.0, 0.1]),
                        add(c, [0.0, 0.0, 0.17]),
                        add(c, [0.05, 0.0, 0.1]),
                    ],
                    5,
                ),
                false,
                0.004,
                5,
            );
            for x in [w * 0.18, w * 0.32] {
                pot_shape(
                    b,
                    "glaze2",
                    [x, y, 0.015],
                    &[
                        (0.0, 0.0),
                        (0.022, 0.0),
                        (0.035, 0.05),
                        (0.03, 0.05),
                        (0.018, 0.008),
                        (0.0, 0.008),
                    ],
                );
            }
        }
        // Contemporary: a stack of books with a ceramic sphere on it and
        // a ring standing beside.
        4 => {
            book(b, "books2", [-w * 0.15, y, 0.0], 0.28, 0.2, 0.035, 0.0);
            book(b, "books3", [-w * 0.15, y, 0.035], 0.24, 0.18, 0.03, 0.08);
            book(b, "books", [-w * 0.15, y, 0.065], 0.22, 0.16, 0.025, -0.06);
            b.ellipsoid("mat", [-w * 0.15, y, 0.09 + 0.05], [0.05, 0.05, 0.05]);
            let rr = (h * 0.28).clamp(0.06, 0.14);
            let ring: Vec<P3> = (0..40)
                .map(|k| {
                    let a = TAU * k as f64 / 40.0;
                    [w * 0.3 + rr * a.cos(), y, rr * 1.2 + 0.012 + rr * a.sin()]
                })
                .collect();
            tube(b, "glaze2", &ring, true, rr * 0.18, 10);
            b.rbox(
                "glaze2",
                [w * 0.3 - 0.05, y - 0.03, 0.0],
                [w * 0.3 + 0.05, y + 0.03, 0.012],
                0.0,
            );
        }
        // Rustic: a board with an iron lantern and its candle, and a clay
        // jug.
        5 => {
            b.rbox(
                "frame",
                [-w * 0.48, y - d * 0.45, 0.0],
                [w * 0.48, y + d * 0.45, 0.025],
                0.0,
            );
            let lc = [-w * 0.18, y, 0.025];
            let lh = (h * 0.55).clamp(0.15, 0.35);
            let s = 0.07;
            b.rbox("metal", add(lc, [-s, -s, 0.0]), add(lc, [s, s, 0.012]), 0.0);
            b.rbox(
                "glass",
                add(lc, [-s * 0.92, -s * 0.92, 0.012]),
                add(lc, [s * 0.92, s * 0.92, lh * 0.8]),
                0.0,
            );
            for (x, yy) in [(-1.0, -1.0), (1.0, -1.0), (1.0, 1.0), (-1.0, 1.0)] {
                b.rbox(
                    "metal",
                    add(lc, [x * s - 0.005, yy * s - 0.005, 0.0]),
                    add(lc, [x * s + 0.005, yy * s + 0.005, lh * 0.8]),
                    0.0,
                );
            }
            turned(
                b,
                "metal",
                add(lc, [0.0, 0.0, lh * 0.8]),
                &[
                    (0.0, 0.0),
                    (s * 1.5, 0.0),
                    (0.02, lh * 0.15),
                    (0.0, lh * 0.15),
                ],
                4,
                PI / 4.0,
            );
            tube(
                b,
                "metal",
                &(0..=10)
                    .map(|k| {
                        let a = PI * k as f64 / 10.0;
                        add(lc, [0.03 * a.cos(), 0.0, lh * 0.95 + 0.03 * a.sin()])
                    })
                    .collect::<Vec<_>>(),
                false,
                0.003,
                5,
            );
            candle(b, add(lc, [0.0, 0.0, 0.012]), 0.025, 0.07);
            let jc = [w * 0.24, y, 0.025];
            let jh = h * 0.5;
            pot_shape(
                b,
                "mat",
                jc,
                &[
                    (0.0, 0.0),
                    (0.045, 0.0),
                    (0.07, jh * 0.45),
                    (0.04, jh * 0.85),
                    (0.045, jh),
                    (0.0, jh),
                ],
            );
            tube(
                b,
                "mat",
                &curve(
                    &[
                        add(jc, [0.055, 0.0, jh * 0.55]),
                        add(jc, [0.1, 0.0, jh * 0.75]),
                        add(jc, [0.04, 0.0, jh * 0.92]),
                    ],
                    5,
                ),
                false,
                0.008,
                6,
            );
        }
        // Industrial: a steel tray with three jars of candles, and books.
        6 => {
            b.rbox(
                "metal",
                [-w * 0.45, y - d * 0.4, 0.0],
                [w * 0.1, y + d * 0.4, 0.008],
                0.0,
            );
            b.rbox(
                "metal",
                [-w * 0.45, y - d * 0.4, 0.008],
                [w * 0.1, y - d * 0.4 + 0.004, 0.03],
                0.0,
            );
            b.rbox(
                "metal",
                [-w * 0.45, y + d * 0.4 - 0.004, 0.008],
                [w * 0.1, y + d * 0.4, 0.03],
                0.0,
            );
            for (k, x) in [-w * 0.35, -w * 0.18, -w * 0.02].into_iter().enumerate() {
                let jh = [0.11, 0.08, 0.095][k];
                pot_shape(
                    b,
                    "glass",
                    [x, y, 0.008],
                    &[(0.0, 0.0), (0.035, 0.0), (0.036, jh), (0.0, jh)],
                );
                candle(b, [x, y, 0.012], 0.028, jh * 0.6);
            }
            book(b, "books2", [w * 0.3, y, 0.0], 0.22, 0.16, 0.03, 0.2);
            book(b, "books", [w * 0.3, y, 0.03], 0.2, 0.15, 0.03, 0.05);
        }
        // Classic: a pair of silver candlesticks and a porcelain vase of
        // flowers between them.
        _ => {
            let ch = (h * 0.6).clamp(0.15, 0.4);
            for x in [-w * 0.35, w * 0.35] {
                pot_shape(
                    b,
                    "metal",
                    [x, y, 0.0],
                    &[
                        (0.0, 0.0),
                        (0.05, 0.0),
                        (0.05, 0.01),
                        (0.02, 0.04),
                        (0.012, ch * 0.4),
                        (0.022, ch * 0.45),
                        (0.01, ch * 0.5),
                        (0.01, ch * 0.9),
                        (0.03, ch * 0.95),
                        (0.03, ch),
                        (0.0, ch),
                    ],
                );
                candle(b, [x, y, ch], 0.011, 0.13);
            }
            let vh = h * 0.45;
            let c = [0.0, y, 0.0];
            pot_shape(
                b,
                "mat",
                c,
                &[
                    (0.0, 0.0),
                    (0.04, 0.0),
                    (0.03, 0.02),
                    (0.07, vh * 0.45),
                    (0.06, vh * 0.75),
                    (0.035, vh * 0.9),
                    (0.05, vh),
                    (0.0, vh),
                ],
            );
            branches(b, add(c, [0.0, 0.0, vh]), (h - vh) * 0.8, 5, false, 3);
            let mut rnd = Rand(0x3141_5926);
            for i in 0..7 {
                let a = TAU * i as f64 / 7.0;
                let q = add(
                    c,
                    [
                        0.05 * a.cos(),
                        0.05 * a.sin(),
                        vh + (h - vh) * rnd.range(0.4, 0.75),
                    ],
                );
                b.ellipsoid("flowers", q, [0.03, 0.03, 0.024]);
            }
        }
    }
    for role in ["leaves", "stem", "flowers"] {
        b.soft.insert(role);
        b.fine.insert(role);
    }
    true
}

// ----------------------------------------------------------- wall decor

/// Moves what `make` builds as a thing lying face up (its front towards
/// `+z`) onto the wall: its front towards `+y`, its middle at `c`.
fn on_wall(b: &mut Builder, c: P3, make: impl FnOnce(&mut Builder)) {
    let start = b.mark();
    make(b);
    // (x, y, z) → (−x, z, y): its up stays up, its front comes out of
    // the wall (a half turn about the diagonal of y and z, not a mirror).
    b.transform_from(start, |q| [c[0] - q[0], c[1] + q[2], c[2] + q[1]]);
}

/// A frame of four bars round the rectangle `w` by `h` (outside), `bar`
/// wide and `t` deep, lying face up from `z` 0, centred on the origin.
fn flat_frame(b: &mut Builder, role: &'static str, w: f64, h: f64, bar: f64, t: f64) {
    let (x, y) = (w / 2.0, h / 2.0);
    b.rbox(role, [-x, -y, 0.0], [x, -y + bar, t], 0.0);
    b.rbox(role, [-x, y - bar, 0.0], [x, y, t], 0.0);
    b.rbox(role, [-x, -y + bar, 0.0], [-x + bar, y - bar, t], 0.0);
    b.rbox(role, [x - bar, -y + bar, 0.0], [x, y - bar, t], 0.0);
}

/// Pictures, mirrors and clocks of the collections (after the family's 5
/// own models), on the wall at `y` 0 facing `+y`: `width`, `height`, and
/// `elevation` (their bottom from the floor).
pub fn wall_decor(b: &mut Builder, p: &Params) -> bool {
    let Some(ix) = look(p, 5) else {
        return false;
    };
    let w = p.get("width", 0.6).clamp(0.15, 2.4);
    let h = p.get("height", 0.8).clamp(0.15, 2.0);
    let z = p.get("elevation", 1.2).clamp(0.0, 2.4);
    let mid = [0.0, 0.0, z + h / 2.0];
    match ix {
        // Nordic: a print in a thin oak frame with its mount: a pale
        // ground, a sun and a hill.
        0 => on_wall(b, mid, |b| {
            flat_frame(b, "frame", w, h, 0.02, 0.025);
            b.rbox(
                "mount",
                [-w / 2.0 + 0.02, -h / 2.0 + 0.02, 0.0],
                [w / 2.0 - 0.02, h / 2.0 - 0.02, 0.012],
                0.0,
            );
            let (aw, ah) = (w * 0.6, h * 0.6);
            b.rbox(
                "art",
                [-aw / 2.0, -ah / 2.0, 0.0],
                [aw / 2.0, ah / 2.0, 0.014],
                0.0,
            );
            turned(
                b,
                "art2",
                [aw * 0.15, ah * 0.12, 0.0],
                &[
                    (0.0, 0.0),
                    (aw * 0.18, 0.0),
                    (aw * 0.18, 0.017),
                    (0.0, 0.017),
                ],
                32,
                0.0,
            );
            b.slab(
                "art3",
                &[
                    [-aw / 2.0, -ah / 2.0],
                    [aw / 2.0, -ah / 2.0],
                    [aw / 2.0, -ah * 0.15],
                    [0.0, ah * 0.02],
                    [-aw / 2.0, -ah * 0.2],
                ],
                0.0,
                0.018,
            );
        }),
        // Mediterranean: three glazed plates, the middle one larger.
        1 => {
            for (k, (dx, dz, r)) in [
                (-w * 0.32, -h * 0.12, 0.12),
                (0.0, h * 0.1, 0.17),
                (w * 0.32, -h * 0.12, 0.12),
            ]
            .into_iter()
            .enumerate()
            {
                let r = r * (w / 0.6).clamp(0.6, 2.0);
                on_wall(b, add(mid, [dx, 0.0, dz]), |b| {
                    turned(
                        b,
                        "mat",
                        [0.0; 3],
                        &[
                            (0.0, 0.0),
                            (r * 0.6, 0.0),
                            (r, 0.02),
                            (r * 0.97, 0.026),
                            (r * 0.6, 0.008),
                            (0.0, 0.008),
                        ],
                        32,
                        0.0,
                    );
                    let band = if k == 1 { "art3" } else { "art2" };
                    turned(
                        b,
                        band,
                        [0.0, 0.0, 0.0],
                        &[
                            (r * 0.7, 0.012),
                            (r * 0.9, 0.019),
                            (r * 0.9, 0.021),
                            (r * 0.7, 0.014),
                        ],
                        32,
                        0.0,
                    );
                    turned(
                        b,
                        band,
                        [0.0, 0.0, 0.0],
                        &[
                            (0.0, 0.0085),
                            (r * 0.25, 0.0085),
                            (r * 0.25, 0.0105),
                            (0.0, 0.0105),
                        ],
                        24,
                        0.0,
                    );
                });
            }
        }
        // Fifties: a sunburst mirror, gilded rods round a round glass.
        2 => {
            let r = (w.min(h) / 2.0) * 0.45;
            on_wall(b, mid, |b| {
                turned(
                    b,
                    "glass",
                    [0.0; 3],
                    &[(0.0, 0.0), (r, 0.0), (r, 0.012), (0.0, 0.012)],
                    40,
                    0.0,
                );
                turned(
                    b,
                    "metal",
                    [0.0; 3],
                    &[(r, 0.0), (r * 1.12, 0.0), (r * 1.12, 0.02), (r, 0.02)],
                    40,
                    0.0,
                );
                let reach = w.min(h) / 2.0;
                for k in 0..24 {
                    let a = TAU * k as f64 / 24.0;
                    let long = if k % 2 == 0 { 1.0 } else { 0.78 };
                    let q0 = [r * 1.1 * a.cos(), r * 1.1 * a.sin(), 0.01];
                    let q1 = [reach * long * a.cos(), reach * long * a.sin(), 0.01];
                    b.cone("metal", q0, q1, 0.004, 0.003, 6);
                    b.ellipsoid("metal", q1, [0.009, 0.009, 0.009]);
                }
            });
        }
        // Japandi: a hanging scroll, its paper between two rods, an ink
        // stroke on it, hung from a cord.
        3 => on_wall(b, mid, |b| {
            let sw = w * 0.6;
            b.rbox(
                "art",
                [-sw / 2.0, -h / 2.0 + 0.03, 0.0],
                [sw / 2.0, h / 2.0 - 0.03, 0.004],
                0.0,
            );
            b.rbox(
                "mount",
                [-sw / 2.0 + 0.03, -h / 2.0 + 0.12, 0.0],
                [sw / 2.0 - 0.03, h / 2.0 - 0.12, 0.006],
                0.0,
            );
            for y in [-h / 2.0 + 0.03, h / 2.0 - 0.03] {
                b.cone(
                    "frame",
                    [-sw / 2.0 - 0.03, y, 0.012],
                    [sw / 2.0 + 0.03, y, 0.012],
                    0.012,
                    0.012,
                    12,
                );
            }
            let stroke: Vec<P3> = (0..=12)
                .map(|k| {
                    let t = k as f64 / 12.0;
                    [
                        -sw * 0.15 + sw * 0.3 * (t * 2.0).sin() * 0.5,
                        h * 0.25 - h * 0.5 * t,
                        0.008,
                    ]
                })
                .collect();
            tube(b, "art3", &stroke, false, 0.008, 5);
            tube(
                b,
                "dark",
                &[
                    [-sw * 0.3, h / 2.0 - 0.03, 0.012],
                    [0.0, h / 2.0 + 0.08, 0.012],
                    [sw * 0.3, h / 2.0 - 0.03, 0.012],
                ],
                false,
                0.002,
                4,
            );
        }),
        // Contemporary: a large canvas of colour blocks in a slim black
        // frame.
        4 => on_wall(b, mid, |b| {
            flat_frame(b, "dark", w, h, 0.012, 0.035);
            b.rbox(
                "art",
                [-w / 2.0 + 0.012, -h / 2.0 + 0.012, 0.0],
                [w / 2.0 - 0.012, h / 2.0 - 0.012, 0.03],
                0.0,
            );
            b.rbox(
                "art2",
                [-w * 0.35, -h * 0.3, 0.0],
                [w * 0.05, h * 0.35, 0.032],
                0.0,
            );
            b.rbox(
                "art3",
                [w * 0.0, -h * 0.4, 0.0],
                [w * 0.38, -h * 0.05, 0.034],
                0.0,
            );
        }),
        // Rustic: a mirror in a thick frame of old wood with iron
        // corners, hung on a rope.
        5 => on_wall(b, mid, |b| {
            flat_frame(b, "frame", w, h, 0.07, 0.04);
            b.rbox(
                "glass",
                [-w / 2.0 + 0.06, -h / 2.0 + 0.06, 0.0],
                [w / 2.0 - 0.06, h / 2.0 - 0.06, 0.02],
                0.0,
            );
            for (x, y) in [(-1.0, -1.0), (1.0, -1.0), (1.0, 1.0), (-1.0, 1.0)] {
                b.rbox(
                    "dark",
                    [
                        x * w / 2.0 - 0.06 * (x + 1.0) / 2.0 - 0.0,
                        y * h / 2.0 - 0.06 * (y + 1.0) / 2.0,
                        0.04,
                    ],
                    [
                        x * w / 2.0 + 0.06 * (1.0 - x) / 2.0,
                        y * h / 2.0 + 0.06 * (1.0 - y) / 2.0,
                        0.045,
                    ],
                    0.0,
                );
            }
            tube(
                b,
                "weave",
                &[
                    [-w * 0.35, h / 2.0 - 0.02, 0.03],
                    [0.0, h / 2.0 + 0.15, 0.01],
                    [w * 0.35, h / 2.0 - 0.02, 0.03],
                ],
                false,
                0.007,
                6,
            );
        }),
        // Industrial: a large station clock, its steel ring, face, marks
        // and hands.
        6 => {
            let r = w.min(h) / 2.0;
            on_wall(b, mid, |b| {
                turned(
                    b,
                    "metal",
                    [0.0; 3],
                    &[
                        (0.0, 0.0),
                        (r, 0.0),
                        (r, 0.05),
                        (r * 0.92, 0.05),
                        (r * 0.92, 0.02),
                        (0.0, 0.02),
                    ],
                    48,
                    0.0,
                );
                turned(
                    b,
                    "mat",
                    [0.0; 3],
                    &[(0.0, 0.0), (r * 0.93, 0.0), (r * 0.93, 0.021), (0.0, 0.021)],
                    48,
                    0.0,
                );
                for k in 0..12 {
                    let a = TAU * k as f64 / 12.0;
                    let (l0, l1) = if k % 3 == 0 {
                        (0.68, 0.86)
                    } else {
                        (0.76, 0.86)
                    };
                    let wd = if k % 3 == 0 { 0.02 } else { 0.01 };
                    bar(
                        b,
                        "dark",
                        [r * l0 * a.cos(), r * l0 * a.sin(), 0.024],
                        [r * l1 * a.cos(), r * l1 * a.sin(), 0.024],
                        wd,
                        [0.0, 0.0, 1.0],
                    );
                }
                bar(
                    b,
                    "dark",
                    [0.0, -r * 0.08, 0.03],
                    [r * 0.38, r * 0.3, 0.03],
                    0.016,
                    [0.0, 0.0, 1.0],
                );
                bar(
                    b,
                    "dark",
                    [0.0, -r * 0.1, 0.035],
                    [-r * 0.12, r * 0.7, 0.035],
                    0.011,
                    [0.0, 0.0, 1.0],
                );
                b.cone(
                    "dark",
                    [0.0, 0.0, 0.02],
                    [0.0, 0.0, 0.042],
                    0.018,
                    0.018,
                    12,
                );
            });
        }
        // Classic: an oil painting in a gilded moulded frame with its
        // corner ornaments.
        _ => on_wall(b, mid, |b| {
            flat_frame(b, "metal", w, h, 0.09, 0.03);
            flat_frame(b, "metal", w - 0.03, h - 0.03, 0.03, 0.05);
            flat_frame(b, "metal", w - 0.14, h - 0.14, 0.02, 0.04);
            let (aw, ah) = (w - 0.16, h - 0.16);
            b.rbox(
                "art",
                [-aw / 2.0, -ah / 2.0, 0.0],
                [aw / 2.0, ah / 2.0, 0.025],
                0.0,
            );
            b.rbox(
                "art2",
                [-aw / 2.0, -ah / 2.0, 0.0],
                [aw / 2.0, -ah * 0.1, 0.027],
                0.0,
            );
            b.slab(
                "art3",
                &[
                    [-aw / 2.0, -ah * 0.1],
                    [-aw * 0.1, -ah * 0.1],
                    [-aw * 0.25, ah * 0.15],
                    [-aw / 2.0, ah * 0.05],
                ],
                0.0,
                0.029,
            );
            for (x, y) in [(-1.0, -1.0), (1.0, -1.0), (1.0, 1.0), (-1.0, 1.0)] {
                b.ellipsoid(
                    "metal",
                    [x * (w / 2.0 - 0.045), y * (h / 2.0 - 0.045), 0.04],
                    [0.04, 0.04, 0.025],
                );
            }
        }),
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fine::shape::signed_volume;

    #[test]
    fn the_house_plants_and_decor_of_the_eight_collections() {
        type Make = fn(&mut Builder, &Params) -> bool;
        let pieces: [(&str, Make, f32); 3] = [
            ("planta", potted_plant, 1.6),
            ("decoración", decor, 1.3),
            ("cuadro", wall_decor, 2.2),
        ];
        for c in 0..8 {
            for (name, make, top) in pieces {
                let mut p = Params::default();
                p.set("style", (6 + c) as f64);
                let mut b = Builder::default();
                assert!(make(&mut b, &p), "{name}");
                let tris = b.triangle_count();
                assert!((20..40_000).contains(&tris), "{c} {name}: {tris}");
                for (role, t) in &b.parts {
                    let v = signed_volume(&b.positions, t);
                    assert!(v > -1e-7, "{c} {name} {role}: {v}");
                }
                let low = b.positions.iter().map(|q| q[2]).fold(f32::MAX, f32::min);
                assert!(low > -0.001, "{c} {name}: {low}");
                let high = b.positions.iter().map(|q| q[2]).fold(f32::MIN, f32::max);
                assert!(high < top, "{c} {name}: {high}");
            }
        }
    }
}
