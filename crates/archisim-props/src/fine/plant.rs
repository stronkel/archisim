//! Potted plants of the workshop: leaves with their outline, a midrib,
//! veins pressed into them, bending down by the tip and a little twisted,
//! each on a stalk that grows out of the soil and runs into its midrib;
//! a pot turned with its wall, rim and inside, and soil in it.
//!
//! Models: 1 a peace lily (Spathiphyllum) in a glazed bellied pot, with
//! its white flowers; 2 a Chinese money plant (Pilea) in a terracotta
//! pot on its saucer.

use super::shape::{add, curve, leaf, mul, rotate, sweep, turned, Cap, Leaf, Outline};
use super::soft::wobble_at;
use crate::build::{norm, Builder, P3};
use crate::Params;
use std::f64::consts::{PI, TAU};

/// Sides of a pot.
const POT_SIDES: usize = 28;

/// A small random number generator (the same plant every time).
pub(super) struct Rand(pub(super) u32);

impl Rand {
    pub(super) fn next(&mut self) -> f64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 17;
        self.0 ^= self.0 << 5;
        f64::from(self.0) / f64::from(u32::MAX)
    }

    pub(super) fn range(&mut self, a: f64, b: f64) -> f64 {
        a + (b - a) * self.next()
    }
}

/// The plant. Values: `height` (all of it, pot included) and `pot`
/// (diameter of the pot); centred on `x = 0`, its back at `y = 0`, as the
/// library's.
pub fn potted_plant(b: &mut Builder, p: &Params) {
    let h = p.get("height", 0.75).clamp(0.2, 2.5);
    let pot = p.get("pot", 0.3).clamp(0.1, 0.9);
    let r = pot / 2.0;
    let c = [0.0, r, 0.0];
    match p.style() {
        2 => {
            let saucer = 0.009;
            let pot_h = (pot * 0.86).min(h * 0.42);
            terracotta(b, c, r, pot_h, saucer);
            let soil = saucer + pot_h * 0.86;
            soil_in(b, [c[0], c[1], soil], r * 0.84, 7);
            pilea(b, [c[0], c[1], soil], r * 0.84, h - soil);
        }
        _ => {
            let pot_h = (pot * 0.82).min(h * 0.36);
            bellied(b, c, r, pot_h);
            let soil = pot_h * 0.9;
            soil_in(b, [c[0], c[1], soil], r * 0.8, 3);
            peace_lily(b, [c[0], c[1], soil], r * 0.8, h - soil);
        }
    }
    // Leaves and stalks are smooth all over (outlined only against what is
    // behind them), as the fabric.
    for role in ["leaves", "leaves2", "stem", "flowers"] {
        b.soft.insert(role);
    }
}

/// A glazed pot with a round belly on a small foot, its rim rolled over
/// and its inside down to the soil.
fn bellied(b: &mut Builder, c: P3, r: f64, h: f64) {
    let profile = [
        (r * 0.52, 0.0),
        (r * 0.57, 0.003),
        (r * 0.59, h * 0.035),
        (r * 0.66, h * 0.07),
        (r * 0.86, h * 0.22),
        (r * 0.97, h * 0.42),
        (r * 1.0, h * 0.58),
        (r * 0.97, h * 0.74),
        (r * 0.89, h * 0.87),
        (r * 0.86, h * 0.92),
        (r * 0.88, h * 0.955),
        (r * 0.885, h * 0.99),
        (r * 0.87, h),
        (r * 0.84, h * 0.998),
        (r * 0.825, h * 0.975),
        (r * 0.81, h * 0.94),
        (r * 0.8, h * 0.88),
    ];
    turned(b, "pot", c, &profile, POT_SIDES, 0.0);
}

/// A terracotta flower pot: tapered, with its thick band at the top, on a
/// saucer.
fn terracotta(b: &mut Builder, c: P3, r: f64, h: f64, saucer: f64) {
    // The saucer, a shallow dish a little wider than the pot's foot.
    turned(
        b,
        "pot",
        c,
        &[
            (r * 0.62, 0.0),
            (r * 0.8, 0.0),
            (r * 0.86, 0.004),
            (r * 0.93, saucer + 0.016),
            (r * 0.9, saucer + 0.018),
            (r * 0.84, saucer + 0.004),
            (r * 0.78, saucer),
        ],
        POT_SIDES,
        0.0,
    );
    let z = [c[0], c[1], c[2] + saucer - 0.001];
    let profile = [
        (r * 0.7, 0.0),
        (r * 0.725, 0.004),
        (r * 0.86, h * 0.76),
        (r * 0.865, h * 0.775),
        (r * 0.985, h * 0.79),
        (r * 1.0, h * 0.81),
        (r * 1.0, h * 0.985),
        (r * 0.985, h),
        (r * 0.89, h),
        (r * 0.875, h * 0.985),
        (r * 0.86, h * 0.94),
        (r * 0.84, h * 0.86),
    ];
    turned(b, "pot", z, &profile, POT_SIDES, 0.0);
}

/// The soil in a pot, `ri` inside its wall at `c`: domed a little, with a
/// few crumbs on it.
pub(super) fn soil_in(b: &mut Builder, c: P3, ri: f64, seed: u32) {
    turned(
        b,
        "soil",
        c,
        &[
            (0.0, 0.007),
            (ri * 0.55, 0.005),
            (ri * 0.9, 0.002),
            (ri + 0.002, -0.001),
            (ri + 0.002, -0.012),
        ],
        POT_SIDES,
        0.0,
    );
    let mut rnd = Rand(0x51ed_270b ^ seed.wrapping_mul(2_654_435_761));
    for _ in 0..4 {
        let a = rnd.range(0.0, TAU);
        let d = ri * rnd.range(0.35, 0.85);
        let s = ri * rnd.range(0.035, 0.07);
        let q = [c[0] + a.cos() * d, c[1] + a.sin() * d, c[2] + 0.004];
        b.blob(
            "soil",
            q,
            [s * a.cos(), s * a.sin(), 0.0],
            [-s * 0.8 * a.sin(), s * 0.8 * a.cos(), 0.0],
            [0.0, 0.0, s * 0.45],
        );
    }
}

/// A stalk from `from` growing up and leaning out towards `out` until it
/// heads `lean` radians off the vertical, `length` long: its points.
pub(super) fn stalk(from: P3, out: [f64; 2], lean: f64, length: f64, wobble: u32) -> Vec<P3> {
    let n = 14;
    let o = [out[0], out[1], 0.0];
    let up = [0.0, 0.0, 1.0];
    let mut pts = vec![from];
    let mut q = from;
    for i in 1..=n {
        let t = i as f64 / n as f64;
        // It bends most in its upper part, as a stalk reaching for light.
        let a = lean * t.powf(1.6);
        let d = add(mul(up, a.cos()), mul(o, a.sin()));
        q = add(q, mul(d, length / n as f64));
        let w = wobble_at(mul(q, 3.0), wobble);
        pts.push(add(q, mul([w[0], w[1], 0.0], length * 0.008 * t)));
    }
    pts
}

/// The direction of the stalk's tip.
pub(super) fn heading(pts: &[P3]) -> P3 {
    let n = pts.len();
    norm(crate::build::sub(pts[n - 1], pts[n - 2]))
}

/// The peace lily: a dozen long pointed leaves on stalks from the middle
/// of the pot, the inner ones upright and young, the outer ones lying out
/// and bending over, and two white flowers above them.
fn peace_lily(b: &mut Builder, soil: P3, ri: f64, grow: f64) {
    let mut rnd = Rand(0x2545_f491);
    let n = 14;
    let golden = PI * (3.0 - 5f64.sqrt());
    for i in 0..n {
        // From the outside in: the oldest leaves lie lowest.
        let k = i as f64 / (n - 1) as f64;
        let az = i as f64 * golden + rnd.range(-0.2, 0.2);
        let out = [az.cos(), az.sin()];
        let off = ri * 0.32 * (1.0 - k * 0.6) * rnd.range(0.5, 1.0);
        let from = [
            soil[0] + out[0] * off,
            soil[1] + out[1] * off,
            soil[2] - 0.01,
        ];
        let lean = (1.45 - 1.1 * k + rnd.range(-0.12, 0.12)).clamp(0.15, 1.55);
        let stalk_len = grow * (0.34 + 0.16 * k) * rnd.range(0.9, 1.08);
        let pts = stalk(from, out, lean, stalk_len, 100 + i);
        let end = pts[pts.len() - 1];
        let dir = heading(&pts);
        // The leaf faces up and in.
        let o3 = [out[0], out[1], 0.0];
        let up = add(mul([0.0, 0.0, 1.0], lean.sin()), mul(o3, -lean.cos()));
        let blade = grow * rnd.range(0.4, 0.48) * (0.9 + 0.1 * k);
        let mut l = Leaf::new(blade, blade * rnd.range(0.36, 0.42), Outline::Lance);
        l.fold = 0.2;
        l.droop = (0.3 + 0.8 * (1.0 - k)) * rnd.range(0.85, 1.15);
        l.twist = rnd.range(-0.25, 0.25);
        l.wave = 0.06;
        l.veins = 6;
        l.vein_depth = blade * 0.016;
        l.thick = 0.0022;
        l.rows = 22;
        l.cols = 4;
        let role = if k > 0.7 { "leaves2" } else { "leaves" };
        let rib = leaf(b, role, end, dir, up, &l, 7 + i);
        // The stalk runs on as the midrib, thinning to the tip.
        let mut path = pts.clone();
        let reach = (rib.len() as f64 * 0.8) as usize;
        path.extend(rib.iter().skip(2).step_by(2).take(reach / 2));
        let stalk_share = stalk_len / (stalk_len + blade * 0.8);
        sweep(
            b,
            "stem",
            &path,
            |t| {
                if t < stalk_share {
                    0.0034 - 0.0008 * t / stalk_share
                } else {
                    let s = (t - stalk_share) / (1.0 - stalk_share);
                    0.0024 * (1.0 - s) + 0.0005 * s
                }
            },
            5,
            Cap::Flat,
            Cap::Round(1.0),
        );
    }
    // Two flowers: a tall stalk, the white spathe cupped round its
    // cream spike.
    for (i, (az, tall)) in [(1.15, 1.08), (2.2, 0.94)].into_iter().enumerate() {
        let out = [f64::cos(az), f64::sin(az)];
        let from = [
            soil[0] + out[0] * ri * 0.15,
            soil[1] + out[1] * ri * 0.15,
            soil[2] - 0.01,
        ];
        let pts = stalk(from, out, 0.22, grow * 0.78 * tall, 300 + i as u32);
        sweep(
            b,
            "stem",
            &pts,
            |t| 0.0032 - 0.001 * t,
            6,
            Cap::Flat,
            Cap::Round(1.0),
        );
        let top = pts[pts.len() - 1];
        let dir = heading(&pts);
        let o3 = [out[0], out[1], 0.0];
        // The spathe stands up behind the spike, facing out.
        let up = norm(add(o3, mul(dir, -0.2)));
        let size = grow * 0.17;
        let mut s = Leaf::new(size, size * 0.82, Outline::Heart);
        s.fold = 0.4;
        s.droop = -0.35;
        s.twist = 0.15;
        s.thick = 0.0016;
        s.rows = 14;
        s.cols = 4;
        let spathe_dir = norm(add(dir, mul(o3, -0.15)));
        let back = add(top, mul(o3, -0.004));
        let _ = leaf(b, "flowers", back, spathe_dir, up, &s, 50 + i as u32);
        // The spike, cream, a little in front.
        let base = add(top, mul(o3, 0.006));
        let tip = add(base, mul(norm(add(dir, mul(o3, 0.3))), size * 0.5));
        let spike = curve(&[base, super::shape::lerp(base, tip, 0.5), tip], 4);
        sweep(
            b,
            "spadix",
            &spike,
            |t| 0.0062 * (1.0 - 0.4 * t),
            8,
            Cap::Round(0.6),
            Cap::Round(1.0),
        );
    }
}

/// The Chinese money plant: two or three stems out of the soil, round
/// leaves held flat from beneath on long thin stalks all round them.
fn pilea(b: &mut Builder, soil: P3, ri: f64, grow: f64) {
    let mut rnd = Rand(0x6a09_e667);
    let stems = [
        (0.0, 0.0, 0.62, 0.0),
        (2.4, 0.35, 0.45, 0.35),
        (4.6, 0.4, 0.34, 0.55),
    ];
    let mut count = 0u32;
    for (si, (az, off, tall, lean)) in stems.into_iter().enumerate() {
        let out = [f64::cos(az), f64::sin(az)];
        let from = [
            soil[0] + out[0] * ri * off,
            soil[1] + out[1] * ri * off,
            soil[2] - 0.012,
        ];
        let stem_len = grow * tall;
        let pts = stalk(from, out, lean, stem_len, 500 + si as u32);
        sweep(
            b,
            "stem",
            &pts,
            |t| 0.0048 - 0.0022 * t,
            7,
            Cap::Flat,
            Cap::Round(1.0),
        );
        // Leaves from the nodes of its upper part, more near the top.
        let leaves = [11, 7, 5][si];
        for j in 0..leaves {
            let k = j as f64 / (leaves - 1).max(1) as f64;
            let at = ((0.32 + 0.68 * k.sqrt()) * (pts.len() - 1) as f64).round() as usize;
            let node = pts[at.min(pts.len() - 1)];
            let a = az + j as f64 * 2.399_963 + rnd.range(-0.25, 0.25);
            let o = [a.cos(), a.sin(), 0.0];
            // Young leaves at the top stand up; old ones lie out.
            let rise = (0.25 + 0.9 * k + rnd.range(-0.15, 0.15)).clamp(0.05, 1.2);
            let pet_len = grow * rnd.range(0.22, 0.3) * (1.15 - 0.45 * k);
            let d0 = norm(add(mul(o, rise.cos()), mul([0.0, 0.0, 1.0], rise.sin())));
            // The petiole arches up and out, then bends level under the
            // leaf it holds.
            let mid = add(
                node,
                add(mul(d0, pet_len * 0.55), [0.0, 0.0, pet_len * 0.12]),
            );
            let end = add(
                node,
                add(
                    mul(o, pet_len * 0.95),
                    [0.0, 0.0, pet_len * rise.sin() * 0.7 + pet_len * 0.12],
                ),
            );
            let path = curve(&[node, mid, end], 5);
            sweep(
                b,
                "stem",
                &path,
                |t| 0.0019 - 0.0006 * t,
                5,
                Cap::Flat,
                Cap::Flat,
            );
            let size = grow * rnd.range(0.17, 0.22) * (1.1 - 0.35 * k);
            let mut l = Leaf::new(size, size * rnd.range(0.92, 1.0), Outline::Round);
            l.attach = 0.36;
            l.fold = 0.04;
            l.dome = 0.05;
            l.droop = rnd.range(0.05, 0.35);
            l.twist = rnd.range(-0.25, 0.25);
            l.wave = 0.03;
            l.thick = 0.0024;
            l.rows = 14;
            l.cols = 4;
            // Held out from the stalk, its face turned up and a little out.
            let tilt = rnd.range(0.15, 0.5) * (1.0 - k * 0.5);
            let dir = norm(add(o, [0.0, 0.0, 0.12 + 0.3 * k]));
            let up = rotate([0.0, 0.0, 1.0], norm([-o[1], o[0], 0.0]), tilt);
            let role = if k > 0.75 { "leaves2" } else { "leaves" };
            let top = add(end, [0.0, 0.0, 0.0012]);
            let _ = leaf(b, role, top, dir, up, &l, 900 + count);
            count += 1;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn plant(style: f64) -> Builder {
        let mut b = Builder::default();
        let mut p = Params::default();
        p.set("style", style);
        p.set("height", 0.75);
        p.set("pot", 0.3);
        potted_plant(&mut b, &p);
        b
    }

    #[test]
    fn the_plants_are_light_and_in_their_box() {
        for style in [1.0, 2.0] {
            let b = plant(style);
            let tris = b.triangle_count();
            assert!((4_000..18_000).contains(&tris), "{style}: {tris}");
            let top = b.positions.iter().map(|p| p[2]).fold(0.0f32, f32::max);
            assert!((0.55..0.95).contains(&top), "{style}: {top}");
            // Every piece closed: the volume of the whole is positive.
            let v: f64 = b
                .parts
                .values()
                .map(|t| super::super::shape::signed_volume(&b.positions, t))
                .sum();
            assert!(v > 0.0, "{style}");
        }
    }
}
