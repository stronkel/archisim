//! Climbers on a wall (H5): a climbing plant over a stretch of a wall's
//! face, `width` × `height`, on its support — steel cables, a steel mesh,
//! a timber trellis, or none for the self-clinging ones — with its stems
//! from the ground fanning up and across, its leaves and flowers.
//!
//! Axes: the wall's face on `y = 0`, the plant in front of it towards
//! `+y`; from the ground (`z = 0`) up. Roles: `frame` the support, and the
//! garden plants' `vine_stem`, `vine`, `vine2` and `bracts` (flowers).

use super::garden_plants::{across, at_along, blade, leafy, soft, wander, Blade, Form};
use super::plant::Rand;
use super::shape::{add, mul, rotate, sweep, Cap};
use crate::build::{norm, Builder, P3};
use crate::Params;
use std::f64::consts::TAU;

/// The supports a climber may grow on.
pub const SUPPORTS: &[&str] = &[
    "Cables de acero",
    "Malla de acero",
    "Celosía de madera",
    "Sin soporte",
];

/// The climbers: their names.
pub const PLANTS: &[&str] = &[
    "Hiedra",
    "Parra virgen",
    "Jazmín estrellado",
    "Buganvilla",
    "Glicinia",
];

/// How a climber grows: its leaf, how close its leaves are, its flowers.
struct Habit {
    leaf: Blade,
    /// Metres between nodes along a stem.
    node: f64,
    /// Leaves at each node.
    per_node: usize,
    /// Flower clusters per metre of stem (0: none) and the length of a
    /// cluster (a raceme hangs).
    flowers: f64,
    raceme: f64,
    /// Width of a flower's petal.
    petal: f64,
}

fn habit(plant: usize) -> Habit {
    let leaf = |len: f64, w: f64, form: Form| {
        let mut l = Blade::new(len, w, form);
        l.droop = 0.4;
        l.fold = 0.2;
        l.rows = 3;
        l
    };
    match plant {
        // Parra virgen: big leaves, fewer.
        1 => Habit {
            leaf: leaf(0.13, 0.11, Form::Heart),
            node: 0.16,
            per_node: 2,
            flowers: 0.0,
            raceme: 0.0,
            petal: 0.0,
        },
        // Jazmín: small glossy leaves, white stars.
        2 => Habit {
            leaf: leaf(0.06, 0.025, Form::Lance),
            node: 0.07,
            per_node: 2,
            flowers: 6.0,
            raceme: 0.0,
            petal: 0.012,
        },
        // Buganvilla: heart leaves and bracts.
        3 => Habit {
            leaf: leaf(0.09, 0.06, Form::Heart),
            node: 0.1,
            per_node: 3,
            flowers: 5.0,
            raceme: 0.0,
            petal: 0.03,
        },
        // Glicinia: long leaflets and hanging racemes.
        4 => Habit {
            leaf: leaf(0.08, 0.03, Form::Lance),
            node: 0.09,
            per_node: 3,
            flowers: 1.6,
            raceme: 0.25,
            petal: 0.012,
        },
        // Hiedra: small leaves, close.
        _ => Habit {
            leaf: leaf(0.06, 0.055, Form::Heart),
            node: 0.06,
            per_node: 2,
            flowers: 0.0,
            raceme: 0.0,
            petal: 0.0,
        },
    }
}

/// The support, from the wall out to `y` (where the plant holds on).
fn support(b: &mut Builder, kind: usize, w: f64, h: f64) -> f64 {
    let hw = w / 2.0;
    match kind {
        // Steel cables, upright every half metre, with their anchors.
        1 => {
            let y = 0.06;
            let n = ((w / 0.5).round() as usize).max(1);
            for i in 0..=n {
                let x = -hw + w * i as f64 / n as f64;
                b.cyl("frame", [x, y, 0.15], [x, y, h - 0.05], 0.002);
                let mut z = 0.15;
                while z <= h - 0.05 + 1e-6 {
                    b.cyl("frame", [x, 0.0, z], [x, y + 0.006, z], 0.006);
                    z += ((h - 0.2) / ((h - 0.2) / 1.2).ceil().max(1.0)).max(0.3);
                }
            }
            y
        }
        // A steel mesh of 30 cm on standoffs.
        2 => {
            let y = 0.05;
            let cols = ((w / 0.3).round() as usize).max(1);
            let rows = ((h / 0.3).round() as usize).max(1);
            for i in 0..=cols {
                let x = -hw + w * i as f64 / cols as f64;
                b.cyl("frame", [x, y, 0.1], [x, y, h], 0.003);
            }
            for j in 0..=rows {
                let z = 0.1 + (h - 0.1) * j as f64 / rows as f64;
                b.cyl("frame", [-hw, y, z], [hw, y, z], 0.003);
            }
            for i in [0, cols] {
                let x = -hw + w * i as f64 / cols as f64;
                for z in [0.15, h - 0.05] {
                    b.cyl("frame", [x, 0.0, z], [x, y, z], 0.008);
                }
            }
            y
        }
        // A timber trellis: a frame and slats at 45°.
        3 => {
            let y0 = 0.03;
            let t = 0.012;
            let s = 0.025;
            b.rbox("frame", [-hw, y0, 0.0], [-hw + 0.04, y0 + 0.03, h], 0.0);
            b.rbox("frame", [hw - 0.04, y0, 0.0], [hw, y0 + 0.03, h], 0.0);
            b.rbox("frame", [-hw, y0, h - 0.04], [hw, y0 + 0.03, h], 0.0);
            b.rbox("frame", [-hw, y0, 0.15], [hw, y0 + 0.03, 0.19], 0.0);
            let step = 0.3;
            let span = w + h;
            let mut k = -span;
            while k < span {
                for (dir, yy) in [(1.0, y0), (-1.0, y0 + t)] {
                    // The slat along x = k + dir·(z − 0.19), clipped to
                    // the frame.
                    let z_at = |x: f64| 0.19 + (x - k) * dir;
                    let mut pts: Vec<[f64; 2]> = Vec::new();
                    for x in [-hw + 0.04, hw - 0.04] {
                        let z = z_at(x);
                        if (0.19..=h - 0.04).contains(&z) {
                            pts.push([x, z]);
                        }
                    }
                    for z in [0.19, h - 0.04] {
                        let x = k + (z - 0.19) * dir;
                        if (-hw + 0.04..=hw - 0.04).contains(&x) {
                            pts.push([x, z]);
                        }
                    }
                    if pts.len() >= 2 {
                        let (a, c) = (pts[0], pts[1]);
                        let mid = [(a[0] + c[0]) / 2.0, yy + t / 2.0, (a[1] + c[1]) / 2.0];
                        let half = [(c[0] - a[0]) / 2.0, 0.0, (c[1] - a[1]) / 2.0];
                        let l = (half[0] * half[0] + half[2] * half[2]).sqrt();
                        if l > 0.02 {
                            let across = [-half[2] / l * s / 2.0, 0.0, half[0] / l * s / 2.0];
                            b.obox("frame", mid, half, [0.0, t / 2.0, 0.0], across);
                        }
                    }
                }
                k += step;
            }
            y0 + 0.03
        }
        _ => 0.02,
    }
}

/// The climber of the «Trepadora» family.
pub fn wall_climber(b: &mut Builder, p: &Params) {
    let w = p.get("width", 2.0).clamp(0.4, 12.0);
    let h = p.get("height", 2.5).clamp(0.5, 12.0);
    let cover = p.get("cover", 0.8).clamp(0.1, 1.0);
    let kind = p.get("support", 1.0).round() as usize;
    let plant = (p.get("plant", 1.0).round() as usize).saturating_sub(1);
    let seed = p.get("variant", 1.0).round() as u32;
    let y = support(b, kind, w, h);
    let habit = habit(plant);
    let mut rnd = Rand(0x51_7cc1 ^ seed.wrapping_mul(2_654_435_761));
    // The stems: from the ground near the middle, fanning out over the
    // width and up to the top, so many as the cover asks.
    let stems = (((w / 0.35) * cover).round() as usize).clamp(2, 36);
    let root = |rnd: &mut Rand| -> P3 { [rnd.range(-0.15, 0.15) * w.min(1.0), y + 0.04, 0.03] };
    let mut paths: Vec<Vec<P3>> = Vec::new();
    for i in 0..stems {
        let t = if stems == 1 {
            0.5
        } else {
            i as f64 / (stems - 1) as f64
        };
        let x_top = -w / 2.0 + w * (0.1 + 0.8 * t) + rnd.range(-0.1, 0.1);
        let z_top = h * rnd.range(0.75, 1.0);
        let start = root(&mut rnd);
        let n = ((z_top / 0.25).ceil() as usize).max(3);
        let mut path = vec![start];
        for k in 1..=n {
            let u = k as f64 / n as f64;
            // Up first, then across: the way stems climb and spread.
            let x = start[0] + (x_top - start[0]) * u.powf(0.7);
            let z = z_top * u;
            let wob = rnd.range(-0.04, 0.04);
            path.push([x + wob, y + 0.02 + rnd.range(0.0, 0.03), z]);
        }
        paths.push(path);
    }
    // Side shoots across, every so often up each stem.
    let mut shoots: Vec<Vec<P3>> = Vec::new();
    for (i, path) in paths.iter().enumerate() {
        let total = path.last().map_or(0.0, |q| q[2]);
        let mut z = 0.6;
        while z < total - 0.2 {
            let (at, _) = at_along(path, z / total);
            let side = if (i + shoots.len()).is_multiple_of(2) {
                1.0
            } else {
                -1.0
            };
            let len = rnd.range(0.3, 0.7) * cover.sqrt();
            let reach = (w / 2.0 - at[0].abs()).max(0.1);
            let dir = [side, 0.0, rnd.range(-0.2, 0.5)];
            let s = wander(
                at,
                dir,
                [side, 0.0, 0.3],
                len.min(reach),
                0.4,
                4,
                seed + 97 * shoots.len() as u32,
            );
            let s: Vec<P3> = s
                .into_iter()
                .map(|q| [q[0].clamp(-w / 2.0, w / 2.0), y + 0.03, q[2].clamp(0.0, h)])
                .collect();
            shoots.push(s);
            z += rnd.range(0.18, 0.35) / cover;
        }
    }
    for path in &paths {
        sweep(
            b,
            "vine_stem",
            path,
            |t| 0.012 * (1.0 - 0.6 * t) + 0.003,
            5,
            Cap::Flat,
            Cap::Round(1.0),
        );
    }
    for s in &shoots {
        sweep(
            b,
            "vine_stem",
            s,
            |t| 0.004 * (1.0 - 0.6 * t) + 0.001,
            4,
            Cap::Flat,
            Cap::Flat,
        );
    }
    // Leaves along all of them, facing out from the wall.
    let out = [0.0, 1.0, 0.0];
    for path in paths.iter().chain(shoots.iter()) {
        let total: f64 = path
            .windows(2)
            .map(|q| {
                let d = [q[1][0] - q[0][0], q[1][1] - q[0][1], q[1][2] - q[0][2]];
                (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).sqrt()
            })
            .sum();
        if total < 0.05 {
            continue;
        }
        let nodes = ((total / habit.node * cover.sqrt() * 1.4) as usize).max(1);
        let from = if path[0][2] < 0.05 { 0.12 } else { 0.0 };
        for k in 0..nodes {
            let t = from + (1.0 - from) * (k as f64 + 0.5) / nodes as f64;
            let (at, along) = at_along(path, t);
            for j in 0..habit.per_node {
                let side = rotate(across(along), along, rnd.range(0.0, TAU) + j as f64 * 2.1);
                // Leaves turn out from the wall, towards the light.
                let d = norm(add(add(mul(side, 0.8), mul(along, 0.3)), mul(out, 0.6)));
                let mut l = habit.leaf;
                l.length *= rnd.range(0.75, 1.2);
                l.width *= rnd.range(0.8, 1.1);
                l.twist = rnd.range(-0.5, 0.5);
                let role = if rnd.next() < 0.35 { "vine2" } else { "vine" };
                // Near the ground the stem is bare.
                if at[2] < 0.15 {
                    continue;
                }
                blade(b, role, at, d, out, &l);
            }
        }
        // Flowers.
        if habit.flowers > 0.0 && cover > 0.2 {
            let count = (total * habit.flowers * cover) as usize;
            for _ in 0..count {
                let (at, _) = at_along(path, rnd.range(0.2, 1.0));
                if at[2] < 0.4 {
                    continue;
                }
                let base = at;
                if habit.raceme > 0.0 {
                    // A hanging cluster: flowers along a short stalk down.
                    let len = habit.raceme * rnd.range(0.7, 1.2);
                    let stalk = vec![base, [base[0], base[1] + 0.03, (base[2] - len).max(0.05)]];
                    leafy(
                        b,
                        &mut rnd,
                        &stalk,
                        0.0,
                        5,
                        {
                            let mut f =
                                Blade::new(habit.petal * 2.5, habit.petal * 2.0, Form::Heart);
                            f.rows = 2;
                            f
                        },
                        ["bracts", "bracts"],
                        0.0,
                    );
                } else {
                    for f in 0..5 {
                        let a = TAU * f as f64 / 5.0 + rnd.range(-0.2, 0.2);
                        let d = norm([a.cos(), 0.5, a.sin()]);
                        let mut l = Blade::new(habit.petal * 1.6, habit.petal, Form::Heart);
                        l.rows = 2;
                        l.droop = -0.2;
                        blade(b, "bracts", add(base, mul(d, 0.004)), d, out, &l);
                    }
                }
            }
        }
    }
    soft(b);
}
