//! The terrace's plants, made as the workshop's potted plants are (leaf
//! by leaf, stalks and branches swept along their paths) but with the
//! many small leaves a garden plant has: a small olive tree with its
//! round crown, a tuft of ornamental grass with its plumes, a lavender
//! bush in flower, and a bougainvillea that grows along a path (up a
//! post and along a beam) and hangs from it in flower.
//!
//! So many leaves cannot each be the workshop's [`super::shape::leaf`]
//! (64 triangles or more): they are [`blade`]s, a closed leaf of a few
//! rows, 16 to 56 triangles. What reads is the mass of leaves, dense
//! enough that the 3D view outlines the crown and not every leaf.
//!
//! Roles: `stem` (trunk, branches, stalks), `leaves` and `leaves2` (two
//! greens), `flowers`; the climber's are `vine_stem`, `vine`, `vine2`
//! and `bracts`, so that it can grow beside a pot plant of other
//! colours. They are drawn soft.

use super::plant::Rand;
use super::shape::{add, curve, lerp, mul, rotate, sweep, Cap};
use super::soft::wobble_at;
use crate::build::{cross, norm, sub, Builder, P3};
use std::f64::consts::{FRAC_PI_2, PI, TAU};

/// The roles of the plants are smooth all over and drawn without
/// contour lines (`fine`).
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
        b.fine.insert(role);
    }
}

fn dot(a: P3, b: P3) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

fn length(a: P3) -> f64 {
    dot(a, a).sqrt()
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

/// The width of a leaf along it (0 at its base, 1 at its tip), at most 1.
#[derive(Clone, Copy)]
enum Form {
    /// Narrow, widest in the middle (olive, lavender).
    Lance,
    /// Widest near the base, rounded there (bougainvillea).
    Heart,
    /// A grass blade: as wide as it is from its base, then tapering.
    Strap,
}

impl Form {
    fn width(self, u: f64) -> f64 {
        match self {
            Form::Lance => (PI * u).sin().max(0.0).powf(0.7),
            Form::Heart => (u.sqrt() * (1.0 - u) / 0.385).min(1.0),
            Form::Strap => ((1.0 - u).powf(0.55) * (u * 10.0 + 0.35).min(1.0)).max(0.0),
        }
    }
}

/// A leaf as it grows on a garden plant: its shape.
#[derive(Clone, Copy)]
struct Blade {
    length: f64,
    width: f64,
    form: Form,
    /// How much it bends down towards its tip, radians.
    droop: f64,
    /// How much it turns about its midrib by its tip, radians.
    twist: f64,
    /// How much it folds up along the midrib (of its half width).
    fold: f64,
    /// Rows from its base to its tip (at least 2).
    rows: usize,
}

impl Blade {
    fn new(length: f64, width: f64, form: Form) -> Self {
        Blade {
            length,
            width,
            form,
            droop: 0.3,
            twist: 0.0,
            fold: 0.25,
            rows: 4,
        }
    }
}

/// A closed leaf from `base` along `dir`, its upper face towards `up`:
/// a midrib with a margin point on each side at every row, the two faces
/// meeting at the margin and a little apart along the midrib. Its
/// triangles: 8 per row between its two ends and 8 for the ends
/// (24 for 4 rows).
fn blade(b: &mut Builder, role: &'static str, base: P3, dir: P3, up: P3, l: &Blade) {
    let rows = l.rows.max(2);
    let dir = norm(dir);
    let up = {
        let u = sub(up, mul(dir, dot(up, dir)));
        if length(u) < 1e-6 {
            across(dir)
        } else {
            norm(u)
        }
    };
    let thick = (l.width * 0.06).clamp(0.0004, 0.0012);
    let step = l.length / rows as f64;
    let mut p = base;
    let mut ring: Vec<[u32; 4]> = Vec::with_capacity(rows);
    let start = b.point(base);
    let mut tip = start;
    for i in 1..=rows {
        let u = i as f64 / rows as f64;
        let phi = l.droop * u * u;
        let t = add(mul(dir, phi.cos()), mul(up, -phi.sin()));
        let n0 = add(mul(up, phi.cos()), mul(dir, phi.sin()));
        let n = rotate(n0, t, l.twist * u);
        let s = cross(n, t);
        p = add(p, mul(t, step));
        if i == rows {
            tip = b.point(p);
            break;
        }
        let hw = l.width / 2.0 * l.form.width(u);
        let lift = mul(n, l.fold * hw);
        // Left margin, top of the midrib, right margin, its underside.
        ring.push([
            b.point(add(add(p, mul(s, hw)), lift)),
            b.point(add(p, mul(n, thick / 2.0))),
            b.point(sub(add(p, lift), mul(s, hw))),
            b.point(sub(p, mul(n, thick / 2.0))),
        ]);
    }
    let [l1, m1, r1, d1] = ring[0];
    b.tri(role, start, m1, l1);
    b.tri(role, start, r1, m1);
    b.tri(role, start, l1, d1);
    b.tri(role, start, d1, r1);
    for w in ring.windows(2) {
        let [la, ma, ra, da] = w[0];
        let [lb, mb, rb, db] = w[1];
        b.quad(role, ma, mb, lb, la);
        b.quad(role, ra, rb, mb, ma);
        b.quad(role, la, lb, db, da);
        b.quad(role, da, db, rb, ra);
    }
    let [ln, mn, rn, dn] = ring[ring.len() - 1];
    b.tri(role, ln, mn, tip);
    b.tri(role, mn, rn, tip);
    b.tri(role, dn, ln, tip);
    b.tri(role, rn, dn, tip);
}

/// A wandering path from `a` heading along `dir` for `length`, bending
/// towards `towards` as it goes: its points.
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
        pts.push(add(q, mul(w, length * 0.03)));
    }
    pts
}

/// The point at `t` (0 to 1) of the way along a path, and the way it
/// goes there.
fn at_along(path: &[P3], t: f64) -> (P3, P3) {
    let total: f64 = path.windows(2).map(|w| length(sub(w[1], w[0]))).sum();
    let mut left = t.clamp(0.0, 1.0) * total;
    for w in path.windows(2) {
        let l = length(sub(w[1], w[0]));
        if left <= l || l <= 0.0 {
            let k = if l > 0.0 { left / l } else { 0.0 };
            return (lerp(w[0], w[1], k), norm(sub(w[1], w[0])));
        }
        left -= l;
    }
    let n = path.len();
    (path[n - 1], norm(sub(path[n - 1], path[n - 2])))
}

/// Leaves in opposite pairs along `path` from `from` (0 to 1) to its end,
/// `pairs` of them, leaning forward and out, with one at the tip; a
/// share `second` of them in the second role.
#[allow(clippy::too_many_arguments)]
fn leafy(
    b: &mut Builder,
    rnd: &mut Rand,
    path: &[P3],
    from: f64,
    pairs: usize,
    shape: Blade,
    roles: [&'static str; 2],
    second: f64,
) {
    let put = |b: &mut Builder, rnd: &mut Rand, q: P3, d: P3| {
        let mut l = shape;
        l.length *= rnd.range(0.8, 1.15);
        l.width *= rnd.range(0.85, 1.1);
        l.droop = shape.droop * rnd.range(0.5, 1.4);
        l.twist = rnd.range(-0.5, 0.5);
        let role = if rnd.next() < second {
            roles[1]
        } else {
            roles[0]
        };
        let up = norm(add([0.0, 0.0, 1.0], mul(d, -0.3)));
        blade(b, role, q, d, up, &l);
    };
    let mut turn = rnd.range(0.0, PI);
    for k in 0..pairs {
        let t = from + (1.0 - from) * (k as f64 + 0.5) / pairs as f64;
        let (q, along) = at_along(path, t);
        // Each pair a quarter turn from the one before.
        turn += FRAC_PI_2 + rnd.range(-0.3, 0.3);
        let side = rotate(across(along), along, turn);
        for s in [1.0, -1.0] {
            let d = norm(add(mul(along, 0.75), mul(side, s)));
            put(b, rnd, q, d);
        }
    }
    let (q, along) = at_along(path, 1.0);
    put(b, rnd, q, along);
}

/// A small olive tree from `base` (its foot, in the soil), `tall` high
/// with a crown `spread` across: a crooked trunk of two stems twisting
/// round each other, a fork of five branches and, out to the surface of a
/// round crown, twigs dense with narrow grey-green leaves in opposite
/// pairs (their silver undersides, `leaves2`, showing here and there).
pub fn olive(b: &mut Builder, base: P3, tall: f64, spread: f64) {
    let mut rnd = Rand(0x3c6e_f372);
    let top = base[2] + tall;
    let fork = base[2] + tall * 0.4;
    // The crown: a ball a little flattened.
    let crown_z = fork + (top - fork) * 0.55;
    let (rx, rz) = (spread / 2.0, (top - fork) * 0.5);
    // The trunk: two stems twisting round each other up to the fork.
    let mut tops = Vec::new();
    let girth = (tall * 0.02).clamp(0.012, 0.03);
    for s in 0..2 {
        let a0 = s as f64 * PI + 0.4;
        let pts: Vec<P3> = (0..=8)
            .map(|k| {
                let t = k as f64 / 8.0;
                let a = a0 + t * 2.4;
                let r = girth * (0.9 - t * 0.4);
                [
                    base[0] + r * a.cos() + girth * 1.2 * (t * 3.0).sin(),
                    base[1] + r * a.sin(),
                    base[2] - 0.02 + (fork - base[2] + 0.02) * t,
                ]
            })
            .collect();
        sweep(
            b,
            "stem",
            &pts,
            |t| girth * (1.0 - 0.3 * t),
            8,
            Cap::Flat,
            Cap::Flat,
        );
        tops.push(pts[pts.len() - 1]);
    }
    // The main branches, from the fork up and out into the crown.
    let mains = 5;
    let mut branches: Vec<Vec<P3>> = Vec::new();
    for m in 0..mains {
        let az = TAU * m as f64 / mains as f64 + rnd.range(-0.3, 0.3);
        let out = [az.cos(), az.sin(), 0.0];
        let end = [
            base[0] + out[0] * rx * 0.55,
            base[1] + out[1] * rx * 0.55,
            crown_z + rz * rnd.range(-0.1, 0.4),
        ];
        let from = tops[m % 2];
        let d = sub(end, from);
        let pts = wander(
            from,
            add(mul(out, 0.3), [0.0, 0.0, 1.0]),
            d,
            length(d),
            0.8,
            6,
            40 + m as u32,
        );
        sweep(
            b,
            "stem",
            &pts,
            |t| girth * 0.6 * (1.0 - 0.6 * t) + 0.002,
            6,
            Cap::Flat,
            Cap::Round(1.0),
        );
        branches.push(pts);
    }
    // Twigs out to the crown's surface, from the branch nearest each.
    let twigs = 40;
    let golden = PI * (3.0 - 5f64.sqrt());
    let leaf = {
        let mut l = Blade::new(0.065, 0.013, Form::Lance);
        l.droop = 0.25;
        l.fold = 0.2;
        l.rows = 3;
        l
    };
    for k in 0..twigs {
        // Over the crown but its very bottom, evenly.
        let zz = 1.0 - 1.75 * (k as f64 + 0.5) / twigs as f64;
        let rr = (1.0 - zz * zz).max(0.0).sqrt();
        let a = k as f64 * golden;
        let tip = [
            base[0] + rx * rr * a.cos() * rnd.range(0.9, 1.05),
            base[1] + rx * rr * a.sin() * rnd.range(0.9, 1.05),
            crown_z + rz * zz * rnd.range(0.9, 1.05),
        ];
        let near = branches
            .iter()
            .min_by(|p, q| {
                let dp = length(sub(p[p.len() - 1], tip));
                let dq = length(sub(q[q.len() - 1], tip));
                dp.total_cmp(&dq)
            })
            .unwrap();
        let (from, _) = at_along(near, rnd.range(0.55, 0.95));
        let d = sub(tip, from);
        let twig = wander(from, d, d, length(d), 0.0, 4, 100 + k as u32);
        sweep(
            b,
            "stem",
            &twig,
            |t| 0.0045 * (1.0 - 0.7 * t) + 0.001,
            4,
            Cap::Flat,
            Cap::Flat,
        );
        leafy(
            b,
            &mut rnd,
            &twig,
            0.25,
            5,
            leaf,
            ["leaves", "leaves2"],
            0.35,
        );
        // Two sprays off it, out towards the surface beside it.
        for s in 0..2 {
            let (q, along) = at_along(&twig, rnd.range(0.35, 0.8));
            let side = rotate(across(along), along, rnd.range(0.0, TAU));
            let dir = norm(add(along, mul(side, 0.9)));
            let spray = wander(
                q,
                dir,
                along,
                rx * rnd.range(0.18, 0.3),
                0.4,
                3,
                300 + (k * 2 + s) as u32,
            );
            sweep(
                b,
                "stem",
                &spray,
                |t| 0.0025 * (1.0 - 0.6 * t),
                3,
                Cap::Flat,
                Cap::Flat,
            );
            leafy(
                b,
                &mut rnd,
                &spray,
                0.1,
                4,
                leaf,
                ["leaves", "leaves2"],
                0.35,
            );
        }
    }
    soft(b);
}

/// A tuft of ornamental grass from `base`, `tall` high and `spread`
/// across: a hundred and more long blades arching out from the middle,
/// the inner ones upright, and plumes on thin stalks above them.
pub fn grasses(b: &mut Builder, base: P3, tall: f64, spread: f64) {
    let mut rnd = Rand(0x1f83_d9ab);
    let n = 190;
    let golden = PI * (3.0 - 5f64.sqrt());
    for i in 0..n {
        let k = i as f64 / (n - 1) as f64;
        let az = i as f64 * golden;
        let out = [az.cos(), az.sin(), 0.0];
        let off = 0.06 * k.sqrt();
        let from = [base[0] + out[0] * off, base[1] + out[1] * off, base[2]];
        // Inner blades upright, outer ones leaning out and arching over.
        let lean = 0.05 + 0.75 * k + rnd.range(-0.1, 0.1);
        let dir = add(mul(out, lean.sin()), [0.0, 0.0, lean.cos()]);
        let mut l = Blade::new(
            tall * rnd.range(0.7, 1.0) * (1.0 - 0.3 * k),
            rnd.range(0.018, 0.026),
            Form::Strap,
        );
        l.droop = (0.3 + 2.2 * k * (spread / tall).min(1.5)) * rnd.range(0.8, 1.2);
        l.twist = rnd.range(-0.9, 0.9);
        l.fold = 0.35;
        l.rows = 6;
        // The upper face looks in, towards the middle: it bends out.
        let up = add(mul(out, -lean.cos()), [0.0, 0.0, lean.sin()]);
        let role = if rnd.next() < 0.3 {
            "leaves2"
        } else {
            "leaves"
        };
        blade(b, role, from, dir, up, &l);
    }
    // The plumes: a thin stalk and a soft spike nodding at its top.
    for i in 0..16 {
        let az = TAU * i as f64 / 16.0 + rnd.range(-0.2, 0.2);
        let out = [az.cos(), az.sin(), 0.0];
        let from = [base[0] + out[0] * 0.025, base[1] + out[1] * 0.025, base[2]];
        let lean = rnd.range(0.1, 0.4);
        let len = tall * rnd.range(1.0, 1.25);
        let dir = add(mul(out, lean.sin()), [0.0, 0.0, lean.cos()]);
        let pts = wander(from, dir, add(out, [0.0, 0.0, 1.0]), len, 0.25, 4, 800 + i);
        sweep(
            b,
            "stem",
            &pts,
            |t| 0.0022 - 0.001 * t,
            4,
            Cap::Flat,
            Cap::Flat,
        );
        let top = pts[pts.len() - 1];
        let (_, d) = at_along(&pts, 1.0);
        let nod = norm(add(d, add(mul(out, 0.6), [0.0, 0.0, -0.3])));
        let l = tall * 0.2;
        let spike = curve(
            &[
                top,
                add(top, mul(d, l * 0.45)),
                add(top, add(mul(d, l * 0.55), mul(nod, l * 0.5))),
            ],
            4,
        );
        sweep(
            b,
            "flowers",
            &spike,
            |t| 0.016 * (PI * (0.12 + 0.88 * t)).sin().max(0.12),
            6,
            Cap::Round(1.0),
            Cap::Round(1.0),
        );
    }
    soft(b);
}

/// A lavender bush from `base`, `tall` high and `spread` across: a round
/// mound of hundreds of narrow grey leaves, deeper ones too, and, above it, stalks each
/// ending in a spike of whorled flowers.
pub fn lavender(b: &mut Builder, base: P3, tall: f64, spread: f64) {
    let mut rnd = Rand(0x5be0_cd19);
    let r = spread / 2.0;
    let mound = tall * 0.45;
    // The mound: leaves all over a dome, pointing out of it.
    let n = 700;
    let golden = PI * (3.0 - 5f64.sqrt());
    let leaf = {
        let mut l = Blade::new(0.055, 0.008, Form::Lance);
        l.droop = 0.2;
        l.fold = 0.3;
        l.rows = 3;
        l
    };
    for i in 0..n {
        let k = (i as f64 + 0.5) / n as f64;
        // From its top to near its foot, deeper in now and then.
        let zz = 1.0 - 0.95 * k;
        let rr = (1.0 - zz * zz).max(0.0).sqrt();
        let a = i as f64 * golden;
        let depth = rnd.range(0.5, 1.0);
        let normal = norm([rr * a.cos() / r, rr * a.sin() / r, zz / mound]);
        let q = [
            base[0] + r * rr * a.cos() * depth,
            base[1] + r * rr * a.sin() * depth,
            base[2] + mound * zz * depth,
        ];
        let d = norm(add(normal, [0.0, 0.0, 0.5 + rnd.range(-0.3, 0.3)]));
        let mut l = leaf;
        l.length *= rnd.range(0.8, 1.2);
        l.twist = rnd.range(-0.6, 0.6);
        let role = if rnd.next() < 0.3 {
            "leaves2"
        } else {
            "leaves"
        };
        blade(b, role, sub(q, mul(d, 0.02)), d, [0.0, 0.0, 1.0], &l);
    }
    // The flower stalks, from inside the mound, fanning out over it.
    // The flower stalks, from inside the mound, fanning out over it.
    // Inside, the mound itself, under its leaves.
    b.blob(
        "leaves",
        [base[0], base[1], base[2] + mound * 0.2],
        [r * 0.85, 0.0, 0.0],
        [0.0, r * 0.85, 0.0],
        [0.0, 0.0, mound * 0.72],
    );
    for i in 0..60 {
        let a = i as f64 * golden;
        let k = ((i as f64 + 0.5) / 60.0).sqrt();
        let out = [a.cos(), a.sin(), 0.0];
        let from = [
            base[0] + out[0] * r * 0.45 * k,
            base[1] + out[1] * r * 0.45 * k,
            base[2] + mound * 0.3,
        ];
        let lean = 0.08 + 0.55 * k + rnd.range(-0.08, 0.08);
        let len = (tall - mound * 0.3) * rnd.range(0.82, 1.0) * (1.0 - 0.15 * k);
        let dir = add(mul(out, lean.sin()), [0.0, 0.0, lean.cos()]);
        let pts = wander(from, dir, dir, len, 0.0, 3, 1200 + i);
        sweep(b, "stem", &pts, |_| 0.002, 4, Cap::Flat, Cap::Flat);
        let top = pts[pts.len() - 1];
        let (_, d) = at_along(&pts, 1.0);
        let spike_len = tall * 0.17;
        let spike: Vec<P3> = (0..=8)
            .map(|j| add(top, mul(d, spike_len * j as f64 / 8.0)))
            .collect();
        // Whorls of flowers: a beaded spike, thinner at its tip.
        sweep(
            b,
            "flowers",
            &spike,
            |t| 0.0095 * (1.0 - 0.45 * t) * (0.75 + 0.25 * (t * 8.0 * PI).cos().abs()),
            6,
            Cap::Round(1.0),
            Cap::Round(1.0),
        );
    }
    soft(b);
}

/// A bougainvillea growing along `path` from the ground at `ground`: its woody stem, bare and
/// twisting near the ground; leaves on it thickening as it climbs; and,
/// where it runs level (along a beam), dense foliage with shoots hanging
/// from it and clusters of magenta bracts.
pub fn climber(b: &mut Builder, path: &[P3], ground: f64, seed: u32) {
    if path.len() < 2 {
        return;
    }
    let mut rnd = Rand(0x9e37_79b9 ^ seed.wrapping_mul(2_654_435_761));
    sweep(
        b,
        "vine_stem",
        path,
        |t| 0.014 * (1.0 - 0.6 * t) + 0.004,
        6,
        Cap::Flat,
        Cap::Round(1.0),
    );
    let leaf = {
        let mut l = Blade::new(0.095, 0.065, Form::Heart);
        l.droop = 0.5;
        l.fold = 0.2;
        l.rows = 4;
        l
    };
    let bract = {
        let mut l = Blade::new(0.04, 0.034, Form::Heart);
        l.droop = -0.4;
        l.fold = 0.5;
        l.rows = 3;
        l
    };
    let cluster = |b: &mut Builder, rnd: &mut Rand, at: P3| {
        for f in 0..8 {
            let a = TAU * f as f64 / 8.0 + rnd.range(-0.3, 0.3);
            let d = norm([a.cos(), a.sin(), rnd.range(-0.7, 0.5)]);
            let c = add(at, mul(d, 0.012));
            let mut l = bract;
            l.length *= rnd.range(0.8, 1.2);
            l.twist = rnd.range(-0.5, 0.5);
            blade(b, "bracts", c, d, [0.0, 0.0, 1.0], &l);
        }
    };
    let total: f64 = path.windows(2).map(|w| length(sub(w[1], w[0]))).sum();
    let mut s = 0.35;
    let mut node = 0u32;
    while s < total {
        let (at, along) = at_along(path, s / total);
        let level = along[2].abs() < 0.5;
        let height = at[2] - ground;
        node += 1;
        // Leaves at the node, in twos and threes.
        let leaves = if level { 3 } else { 2 };
        for j in 0..leaves {
            let side = rotate(across(along), along, rnd.range(0.0, TAU) + j as f64 * 2.1);
            let d = norm(add(mul(along, 0.35), side));
            let q = add(at, mul(side, 0.012));
            let mut l = leaf;
            l.length *= rnd.range(0.75, 1.15);
            l.width = l.length * 0.68;
            l.twist = rnd.range(-0.5, 0.5);
            let role = if rnd.next() < 0.35 { "vine2" } else { "vine" };
            blade(b, role, q, d, [0.0, 0.0, 1.0], &l);
        }
        // A shoot now and then: up the post short, hanging from the beam
        // long, with leaves along it and bracts at its end.
        let shoot_every = if level {
            3
        } else if height > 0.9 {
            4
        } else {
            9
        };
        if node.is_multiple_of(shoot_every) {
            let side = rotate(across(along), along, rnd.range(0.0, TAU));
            let (dir, towards, len) = if level {
                (
                    norm(add(side, [0.0, 0.0, -0.8])),
                    [0.0, 0.0, -1.0],
                    rnd.range(0.2, 0.55),
                )
            } else {
                (
                    norm(add(side, mul(along, 0.8))),
                    along,
                    rnd.range(0.12, 0.3),
                )
            };
            let shoot = wander(at, dir, towards, len, 0.6, 4, 2000 + node);
            sweep(
                b,
                "vine_stem",
                &shoot,
                |t| 0.004 * (1.0 - 0.6 * t),
                4,
                Cap::Flat,
                Cap::Flat,
            );
            let pairs = ((len / 0.07) as usize).clamp(2, 7);
            leafy(
                b,
                &mut rnd,
                &shoot,
                0.15,
                pairs,
                leaf,
                ["vine", "vine2"],
                0.35,
            );
            if height > 0.8 && rnd.next() < 0.85 {
                let tip = shoot[shoot.len() - 1];
                cluster(b, &mut rnd, tip);
                if level && rnd.next() < 0.5 {
                    let (mid, _) = at_along(&shoot, 0.55);
                    cluster(b, &mut rnd, mid);
                }
            }
        }
        // Bracts along the beam too.
        if level && rnd.next() < 0.3 {
            let side = rotate(across(along), along, rnd.range(0.0, TAU));
            cluster(b, &mut rnd, add(at, mul(side, 0.04)));
        }
        // Closer leaves the higher it climbs; bare low down.
        s += if level {
            rnd.range(0.05, 0.07)
        } else if height > 0.9 {
            rnd.range(0.07, 0.1)
        } else {
            rnd.range(0.14, 0.2)
        };
    }
    soft(b);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fine::shape::signed_volume;

    #[test]
    fn a_blade_is_closed_and_light() {
        let mut b = Builder::default();
        let l = Blade::new(0.07, 0.015, Form::Lance);
        blade(
            &mut b,
            "leaves",
            [0.0, 0.0, 0.5],
            [1.0, 0.0, 0.2],
            [0.0, 0.0, 1.0],
            &l,
        );
        assert_eq!(b.triangle_count(), 8 + 8 * (l.rows - 2));
        let v = signed_volume(&b.positions, &b.parts["leaves"]);
        assert!(v > 0.0, "{v}");
    }

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
            assert!((4_000..34_000).contains(&n), "{name}: {n}");
            assert!(b.positions.iter().all(|p| p.iter().all(|v| v.is_finite())));
            for (role, t) in &b.parts {
                let v = signed_volume(&b.positions, t);
                assert!(v > -1e-7, "{name} {role}: {v}");
            }
        }
        let mut b = Builder::default();
        let mut path: Vec<P3> = (0..=20)
            .map(|k| {
                let a = k as f64 * 0.5;
                [0.05 * a.cos(), 0.05 * a.sin(), 0.12 * k as f64]
            })
            .collect();
        path.extend((1..=10).map(|k| [-0.1 * k as f64, 0.0, 2.45]));
        climber(&mut b, &path, 0.0, 1);
        let n = b.triangle_count();
        assert!((4_000..30_000).contains(&n), "trepadora: {n}");
    }
}
