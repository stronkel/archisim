//! The terrace's plants, made as the workshop's potted plants are (leaf
//! by leaf with [`leaf`], stalks and branches swept along their paths):
//! a small olive tree, a tuft of ornamental grass with its plumes, a
//! lavender bush in flower, and a climber that grows along a path (up a
//! post and along a beam) with its leaves and flowers.
//!
//! Roles: `stem` (trunk, branches, stalks), `leaves` and `leaves2` (two
//! greens), `flowers`; the climber's are `vine_stem`, `vine`, `vine2`
//! and `bracts`, so that it can grow beside a pot plant of other
//! colours. They are drawn soft.

use super::plant::{heading, stalk, Rand};
use super::shape::{add, curve, leaf, lerp, mul, rotate, sweep, Cap, Leaf, Outline};
use super::soft::wobble_at;
use crate::build::{cross, norm, sub, Builder, P3};
use std::f64::consts::{PI, TAU};

/// The roles of the plants are smooth all over.
fn soft(b: &mut Builder) {
    for role in [
        "leaves",
        "leaves2",
        "stem",
        "flowers",
        "vine",
        "vine2",
        "vine_stem",
        "bracts",
    ] {
        b.soft.insert(role);
    }
}

/// A unit vector across `d` (any), for a leaf's upper face to start from.
fn across(d: P3) -> P3 {
    let h = if d[2].abs() < 0.9 {
        [0.0, 0.0, 1.0]
    } else {
        [1.0, 0.0, 0.0]
    };
    norm(cross(cross(d, h), d))
}

/// A wandering path from `a` heading along `dir` for `length`, bending
/// `bend` towards `towards` as it goes: its points.
fn wander(a: P3, dir: P3, towards: P3, length: f64, bend: f64, n: usize, seed: u32) -> Vec<P3> {
    let mut pts = vec![a];
    let mut q = a;
    for i in 1..=n {
        let t = i as f64 / n as f64;
        let d = norm(add(
            mul(norm(dir), 1.0 - bend * t),
            mul(norm(towards), bend * t),
        ));
        q = add(q, mul(d, length / n as f64));
        let w = wobble_at(mul(q, 4.0), seed);
        pts.push(add(q, mul(w, length * 0.025)));
    }
    pts
}

/// A small olive tree from `base` (its foot, in the soil), `tall` high
/// with a crown `spread` across: a crooked trunk of two stems that fork
/// into branches and twigs, narrow grey-green leaves along the twigs.
pub fn olive(b: &mut Builder, base: P3, tall: f64, spread: f64) {
    let mut rnd = Rand(0x3c6e_f372);
    let fork = base[2] + tall * 0.42;
    // The trunk: two stems twisting round each other up to the fork.
    let mut tops = Vec::new();
    for s in 0..2 {
        let a0 = s as f64 * PI + 0.4;
        let pts: Vec<P3> = (0..=10)
            .map(|k| {
                let t = k as f64 / 10.0;
                let a = a0 + t * 2.2;
                let r = 0.018 * (1.0 - t * 0.5) + 0.004;
                [
                    base[0] + r * a.cos() + 0.03 * (t * 3.0).sin(),
                    base[1] + r * a.sin(),
                    base[2] - 0.02 + (fork - base[2] + 0.02) * t,
                ]
            })
            .collect();
        sweep(
            b,
            "stem",
            &pts,
            |t| 0.022 * (1.0 - 0.3 * t),
            9,
            Cap::Flat,
            Cap::Round(0.8),
        );
        tops.push(pts[pts.len() - 1]);
    }
    let centre = [base[0], base[1], fork];
    // Main branches out of the fork, up and out to the crown's edge.
    let mains = 5;
    for m in 0..mains {
        let az = TAU * m as f64 / mains as f64 + rnd.range(-0.3, 0.3);
        let out = [az.cos(), az.sin(), 0.0];
        let reach = spread / 2.0 * rnd.range(0.75, 1.0);
        let rise = (base[2] + tall - fork) * rnd.range(0.55, 0.85);
        let from = tops[m % 2];
        let pts = wander(
            from,
            add(mul(out, 0.5), [0.0, 0.0, 1.0]),
            add(out, [0.0, 0.0, 0.25]),
            (reach * reach + rise * rise).sqrt(),
            0.5,
            8,
            40 + m as u32,
        );
        sweep(
            b,
            "stem",
            &pts,
            |t| 0.012 * (1.0 - 0.75 * t) + 0.002,
            7,
            Cap::Flat,
            Cap::Round(1.0),
        );
        // Twigs from the outer half, each with its leaves; leaves at the
        // branch's tip too.
        let twigs = 7;
        for k in 0..=twigs {
            let along = if k == twigs {
                pts.len() - 1
            } else {
                pts.len() / 3 + (pts.len() * 2 / 3 - 1) * k / twigs
            };
            let at = pts[along.min(pts.len() - 1)];
            let side = rotate(out, [0.0, 0.0, 1.0], rnd.range(-1.2, 1.2));
            let up = rnd.range(0.0, 0.6);
            let dir = norm(add(side, [0.0, 0.0, up]));
            let len = spread * rnd.range(0.12, 0.22);
            let twig = wander(
                at,
                dir,
                add(dir, [0.0, 0.0, -0.4]),
                len,
                0.5,
                5,
                90 + (m * 10 + k) as u32,
            );
            sweep(
                b,
                "stem",
                &twig,
                |t| 0.0035 * (1.0 - 0.7 * t) + 0.0008,
                4,
                Cap::Flat,
                Cap::Round(1.0),
            );
            let n = 9;
            for j in 0..n {
                let t = (j as f64 + 1.0) / (n as f64 + 1.0);
                let i = ((twig.len() - 1) as f64 * t) as usize;
                let q = lerp(twig[i], twig[(i + 1).min(twig.len() - 1)], 0.5);
                let along_dir = norm(sub(twig[(i + 1).min(twig.len() - 1)], twig[i]));
                let s = if j % 2 == 0 { 1.0 } else { -1.0 };
                let off = rotate(across(along_dir), along_dir, s * 1.1 + rnd.range(-0.3, 0.3));
                let ldir = norm(add(mul(along_dir, 0.55), mul(off, 0.8)));
                let mut l = Leaf::new(0.06 * rnd.range(0.85, 1.15), 0.012, Outline::Lance);
                l.fold = 0.18;
                l.droop = rnd.range(0.1, 0.4);
                l.twist = rnd.range(-0.4, 0.4);
                l.thick = 0.0009;
                l.rows = 5;
                l.cols = 2;
                let role = if rnd.next() < 0.45 {
                    "leaves2"
                } else {
                    "leaves"
                };
                let _ = leaf(
                    b,
                    role,
                    q,
                    ldir,
                    [0.0, 0.0, 1.0],
                    &l,
                    300 + (m * 200 + k * 20 + j) as u32,
                );
            }
        }
    }
    let _ = centre;
    soft(b);
}

/// A tuft of ornamental grass from `base`, `tall` high and `spread`
/// across: long narrow blades arching out from the middle, and plumes on
/// thin stalks above them.
pub fn grasses(b: &mut Builder, base: P3, tall: f64, spread: f64) {
    let mut rnd = Rand(0x1f83_d9ab);
    let n = 64;
    let golden = PI * (3.0 - 5f64.sqrt());
    for i in 0..n {
        let k = i as f64 / (n - 1) as f64;
        let az = i as f64 * golden;
        let out = [az.cos(), az.sin()];
        let off = 0.05 * k.sqrt();
        let from = [
            base[0] + out[0] * off,
            base[1] + out[1] * off,
            base[2] - 0.01,
        ];
        // Inner blades upright, outer ones arching out and over.
        let lean = 0.08 + 0.55 * k + rnd.range(-0.08, 0.08);
        let dir = norm([out[0] * lean.sin(), out[1] * lean.sin(), lean.cos()]);
        let len = tall * rnd.range(0.65, 0.95) * (1.0 - 0.25 * k);
        let mut l = Leaf::new(len, rnd.range(0.007, 0.011), Outline::Lance);
        l.fold = 0.3;
        l.droop = (0.4 + 1.6 * k * spread / tall.max(0.1)).min(2.2) * rnd.range(0.8, 1.2);
        l.twist = rnd.range(-0.8, 0.8);
        l.thick = 0.0008;
        l.rows = 10;
        l.cols = 2;
        let up = [-out[0] * lean.cos(), -out[1] * lean.cos(), lean.sin()];
        let role = if rnd.next() < 0.3 {
            "leaves2"
        } else {
            "leaves"
        };
        let _ = leaf(b, role, from, dir, up, &l, 700 + i as u32);
    }
    // The plumes: a thin stalk and a soft spike nodding at its top.
    for i in 0..12 {
        let az = TAU * i as f64 / 12.0 + rnd.range(-0.2, 0.2);
        let out = [az.cos(), az.sin()];
        let from = [base[0] + out[0] * 0.02, base[1] + out[1] * 0.02, base[2]];
        let pts = stalk(
            from,
            out,
            rnd.range(0.15, 0.5),
            tall * rnd.range(1.0, 1.2),
            800 + i,
        );
        sweep(
            b,
            "stem",
            &pts,
            |t| 0.0018 - 0.0008 * t,
            4,
            Cap::Flat,
            Cap::Flat,
        );
        let top = pts[pts.len() - 1];
        let dir = heading(&pts);
        let nod = norm(add(dir, [out[0] * 0.5, out[1] * 0.5, -0.2]));
        let len = tall * 0.18;
        let spike = curve(
            &[
                top,
                add(top, mul(dir, len * 0.5)),
                add(top, add(mul(dir, len * 0.6), mul(nod, len * 0.5))),
            ],
            5,
        );
        sweep(
            b,
            "flowers",
            &spike,
            |t| 0.014 * (PI * (0.15 + 0.85 * t)).sin().max(0.15),
            7,
            Cap::Round(1.0),
            Cap::Round(1.0),
        );
    }
    soft(b);
}

/// A lavender bush from `base`, `tall` high and `spread` across: a mound
/// of narrow grey leaves and, above it, thin stalks each ending in a
/// spike of flowers.
pub fn lavender(b: &mut Builder, base: P3, tall: f64, spread: f64) {
    let mut rnd = Rand(0x5be0_cd19);
    let r = spread / 2.0;
    // The mound of leaves.
    for i in 0..90 {
        let az = rnd.range(0.0, TAU);
        let d = r * 0.6 * rnd.next().sqrt();
        let from = [
            base[0] + az.cos() * d,
            base[1] + az.sin() * d,
            base[2] + tall * 0.05 * rnd.next(),
        ];
        let lean = 0.3 + 0.9 * d / (r * 0.6);
        let dir = norm([az.cos() * lean.sin(), az.sin() * lean.sin(), lean.cos()]);
        let mut l = Leaf::new(0.05 * rnd.range(0.8, 1.2), 0.006, Outline::Lance);
        l.fold = 0.3;
        l.droop = 0.3;
        l.thick = 0.0008;
        l.rows = 4;
        l.cols = 2;
        let _ = leaf(b, "leaves", from, dir, [0.0, 0.0, 1.0], &l, 1000 + i);
    }
    // The flower stalks.
    for i in 0..56 {
        let az = rnd.range(0.0, TAU);
        let d = r * 0.5 * rnd.next().sqrt();
        let out = [az.cos(), az.sin()];
        let from = [base[0] + out[0] * d, base[1] + out[1] * d, base[2]];
        let lean = 0.1 + 0.6 * d / (r * 0.5);
        let len = tall * rnd.range(0.8, 1.0);
        let pts = stalk(from, out, lean, len, 1200 + i);
        sweep(b, "stem", &pts, |_| 0.0013, 4, Cap::Flat, Cap::Flat);
        let top = pts[pts.len() - 1];
        let dir = heading(&pts);
        let spike_len = tall * 0.14;
        let spike = [
            top,
            add(top, mul(dir, spike_len * 0.5)),
            add(top, mul(dir, spike_len)),
        ];
        // Whorls of flowers: a bumpy spike, fatter in its middle.
        sweep(
            b,
            "flowers",
            &spike,
            |t| 0.0055 * (0.6 + 0.4 * (PI * t).sin()) * (1.0 + 0.25 * (t * 30.0).sin()),
            6,
            Cap::Round(1.0),
            Cap::Round(1.0),
        );
    }
    soft(b);
}

/// A climber (a bougainvillea) growing along `path`: its woody stem, a
/// node every 10 cm with leaves on short stalks, side shoots hanging
/// from where the path runs level, and clusters of magenta bracts.
pub fn climber(b: &mut Builder, path: &[P3], seed: u32) {
    if path.len() < 2 {
        return;
    }
    let mut rnd = Rand(0x9e37_79b9 ^ seed);
    sweep(
        b,
        "vine_stem",
        path,
        |t| 0.011 * (1.0 - 0.6 * t) + 0.003,
        6,
        Cap::Flat,
        Cap::Round(1.0),
    );
    // Nodes along it.
    let mut dist = 0.0;
    let mut next = 0.3;
    let mut node = 0u32;
    for w in path.windows(2) {
        let seg = sub(w[1], w[0]);
        let l = (seg[0] * seg[0] + seg[1] * seg[1] + seg[2] * seg[2]).sqrt();
        while next < dist + l {
            let t = (next - dist) / l.max(1e-9);
            let at = lerp(w[0], w[1], t);
            let along = norm(seg);
            node += 1;
            let level = along[2].abs() < 0.5;
            // A shoot: up along a post, hanging down from a beam.
            let side = rotate(across(along), along, rnd.range(0.0, TAU));
            let shoot_dir = if level {
                norm(add(side, [0.0, 0.0, -1.2]))
            } else {
                norm(add(side, mul(along, 0.6)))
            };
            let shoot_len = if level {
                rnd.range(0.12, 0.4)
            } else {
                rnd.range(0.08, 0.18)
            };
            let shoot = wander(
                at,
                shoot_dir,
                if level { [0.0, 0.0, -1.0] } else { along },
                shoot_len,
                0.3,
                4,
                2000 + node,
            );
            sweep(
                b,
                "vine_stem",
                &shoot,
                |t| 0.0035 * (1.0 - 0.6 * t),
                4,
                Cap::Flat,
                Cap::Round(1.0),
            );
            // Leaves along the shoot.
            for j in 0..4 {
                let u = (j as f64 + 1.0) / 5.0;
                let i = ((shoot.len() - 1) as f64 * u) as usize;
                let q = shoot[i];
                let d = norm(sub(
                    shoot[(i + 1).min(shoot.len() - 1)],
                    shoot[i.saturating_sub(1)],
                ));
                let s = if j % 2 == 0 { 1.0 } else { -1.0 };
                let off = rotate(across(d), d, s * 1.3 + rnd.range(-0.3, 0.3));
                let ldir = norm(add(mul(d, 0.4), off));
                let size = rnd.range(0.045, 0.07);
                let mut lf = Leaf::new(size, size * 0.62, Outline::Heart);
                lf.fold = 0.15;
                lf.droop = rnd.range(0.3, 0.8);
                lf.twist = rnd.range(-0.3, 0.3);
                lf.veins = 3;
                lf.vein_depth = size * 0.012;
                lf.thick = 0.001;
                lf.rows = 6;
                lf.cols = 2;
                let role = if rnd.next() < 0.35 { "vine2" } else { "vine" };
                let _ = leaf(b, role, q, ldir, [0.0, 0.0, 1.0], &lf, 3000 + node * 10 + j);
            }
            // Bracts at the shoot's end, now and then.
            if rnd.next() < 0.6 {
                let tip = shoot[shoot.len() - 1];
                for f in 0..6 {
                    let a = TAU * f as f64 / 6.0 + rnd.range(-0.3, 0.3);
                    let dir = norm([a.cos(), a.sin(), rnd.range(-0.6, 0.4)]);
                    let size = rnd.range(0.022, 0.032);
                    let mut br = Leaf::new(size, size * 0.85, Outline::Heart);
                    br.fold = 0.35;
                    br.droop = -0.3;
                    br.thick = 0.0007;
                    br.rows = 5;
                    br.cols = 2;
                    let _ = leaf(
                        b,
                        "bracts",
                        tip,
                        dir,
                        [0.0, 0.0, 1.0],
                        &br,
                        5000 + node * 10 + f,
                    );
                }
            }
            next += rnd.range(0.1, 0.16);
        }
        dist += l;
    }
    soft(b);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_terrace_plants_are_made_and_light() {
        for (name, make) in [
            ("olivo", olive as fn(&mut Builder, P3, f64, f64)),
            ("gramíneas", grasses),
            ("lavanda", lavender),
        ] {
            let mut b = Builder::default();
            make(&mut b, [0.0, 0.3, 0.5], 1.0, 0.8);
            let n = b.triangle_count();
            assert!((2_000..32_000).contains(&n), "{name}: {n}");
            assert!(b.positions.iter().all(|p| p.iter().all(|v| v.is_finite())));
        }
        let mut b = Builder::default();
        let path: Vec<P3> = (0..=20)
            .map(|k| {
                [
                    0.05 * (k as f64).cos(),
                    0.05 * (k as f64).sin(),
                    0.12 * k as f64,
                ]
            })
            .collect();
        climber(&mut b, &path, 1);
        let n = b.triangle_count();
        assert!((2_000..40_000).contains(&n), "trepadora: {n}");
    }
}
