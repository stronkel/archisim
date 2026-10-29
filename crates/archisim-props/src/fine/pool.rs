//! Pools of the eight collections (paquete 8, tanda 8a): the pool as an
//! object, its shell lined below the ground, the water, the coping, the
//! deck round it, the ladder and the lights under water; and the outdoor
//! shower beside it.
//!
//! Pool: `length` (along `x`), `width`, `water_depth`, `deck_width` (the
//! paving round it, 0 for none) and `form` (1 rectangular, 2 with Roman
//! steps, 3 with an infinity edge all round); the fifties pool is kidney
//! shaped and the classic one ends in a half round. Its coping sits on the
//! ground (`z = 0`), the shell goes down below it; centred on `x = 0`, from
//! `y = 0` to the far edge of its deck. Roles: `lining`, `water`, `coping`,
//! `deck`, `shell` (the concrete outside the lining), `accent` (the band
//! at the water line), `metal` (the ladder),
//! `dark` (the overflow grate) and `light` (glowing at night).
//!
//! Shower: `height`; centred on `x = 0`, its back at `y = 0`. Roles:
//! `frame`, `metal`, `dark`, `masonry`, `accent`, `deck` (its tray).

use super::hospitality::{disc, look, rod, stone, Rng};
use super::shape::{curve, sweep, turned, Cap};
use super::soft::tube;
use crate::build::{Builder, P3};
use crate::Params;
use std::f64::consts::{FRAC_PI_2, PI, TAU};

/// The forms of a pool (`form`, from 1).
pub const FORMS: &[&str] = &["Rectangular", "Con escalera romana", "Desbordante"];

type Loop = Vec<[f64; 2]>;

/// Twice the signed area of a loop (positive: counter-clockwise).
fn area2(p: &[[f64; 2]]) -> f64 {
    (0..p.len())
        .map(|i| {
            let (a, b) = (p[i], p[(i + 1) % p.len()]);
            a[0] * b[1] - b[0] * a[1]
        })
        .sum()
}

/// The loop moved `d` outwards (inwards if negative), each corner along
/// the mean of its two sides' normals.
fn offset(p: &[[f64; 2]], d: f64) -> Loop {
    let n = p.len();
    (0..n)
        .map(|i| {
            let (a, b, c) = (p[(i + n - 1) % n], p[i], p[(i + 1) % n]);
            let n1 = norm2([b[1] - a[1], a[0] - b[0]]);
            let n2 = norm2([c[1] - b[1], b[0] - c[0]]);
            let m = norm2([n1[0] + n2[0], n1[1] + n2[1]]);
            let cos = (m[0] * n1[0] + m[1] * n1[1]).max(0.4);
            [b[0] + m[0] * d / cos, b[1] + m[1] * d / cos]
        })
        .collect()
}

fn norm2(v: [f64; 2]) -> [f64; 2] {
    let l = (v[0] * v[0] + v[1] * v[1]).sqrt().max(1e-12);
    [v[0] / l, v[1] / l]
}

/// The triangles of a simple counter-clockwise loop, by clipping ears.
fn triangulate(p: &[[f64; 2]]) -> Vec<[usize; 3]> {
    let mut idx: Vec<usize> = (0..p.len()).collect();
    let mut out = Vec::new();
    let cross = |o: [f64; 2], a: [f64; 2], b: [f64; 2]| {
        (a[0] - o[0]) * (b[1] - o[1]) - (a[1] - o[1]) * (b[0] - o[0])
    };
    let mut guard = 0;
    while idx.len() > 3 && guard < 10_000 {
        guard += 1;
        let n = idx.len();
        let mut cut = None;
        for i in 0..n {
            let (ia, ib, ic) = (idx[(i + n - 1) % n], idx[i], idx[(i + 1) % n]);
            let (a, b, c) = (p[ia], p[ib], p[ic]);
            if cross(a, b, c) <= 1e-12 {
                continue;
            }
            let inside = idx.iter().any(|&j| {
                j != ia
                    && j != ib
                    && j != ic
                    && cross(a, b, p[j]) > 0.0
                    && cross(b, c, p[j]) > 0.0
                    && cross(c, a, p[j]) > 0.0
            });
            if !inside {
                cut = Some(i);
                out.push([ia, ib, ic]);
                break;
            }
        }
        match cut {
            Some(i) => {
                idx.remove(i);
            }
            None => break,
        }
    }
    if idx.len() == 3 {
        out.push([idx[0], idx[1], idx[2]]);
    }
    out
}

/// A solid of the loop's plan from `z0` to `z1`.
fn prism(b: &mut Builder, role: &'static str, p: &[[f64; 2]], z0: f64, z1: f64) {
    let bot: Vec<u32> = p.iter().map(|q| b.point([q[0], q[1], z0])).collect();
    let top: Vec<u32> = p.iter().map(|q| b.point([q[0], q[1], z1])).collect();
    for [i, j, k] in triangulate(p) {
        b.tri(role, top[i], top[j], top[k]);
        b.tri(role, bot[k], bot[j], bot[i]);
    }
    let n = p.len();
    for i in 0..n {
        let j = (i + 1) % n;
        b.quad(role, bot[i], bot[j], top[j], top[i]);
    }
}

/// A band between two loops of as many corners (`outer` round `inner`),
/// from `z0` to `z1`: a closed solid.
fn band(
    b: &mut Builder,
    role: &'static str,
    inner: &[[f64; 2]],
    outer: &[[f64; 2]],
    z0: f64,
    z1: f64,
) {
    let n = inner.len();
    let at = |b: &mut Builder, q: [f64; 2], z: f64| b.point([q[0], q[1], z]);
    let ib: Vec<u32> = inner.iter().map(|q| at(b, *q, z0)).collect();
    let it: Vec<u32> = inner.iter().map(|q| at(b, *q, z1)).collect();
    let ob: Vec<u32> = outer.iter().map(|q| at(b, *q, z0)).collect();
    let ot: Vec<u32> = outer.iter().map(|q| at(b, *q, z1)).collect();
    for i in 0..n {
        let j = (i + 1) % n;
        b.quad(role, it[i], ot[i], ot[j], it[j]);
        b.quad(role, ib[j], ob[j], ob[i], ib[i]);
        b.quad(role, ob[i], ob[j], ot[j], ot[i]);
        b.quad(role, ib[j], ib[i], it[i], it[j]);
    }
}

/// The plan of the water's edge: counter-clockwise round `(0, cy)`.
fn outline(ix: usize, l: f64, w: f64, cy: f64) -> Loop {
    let (a, h) = (l / 2.0, w / 2.0);
    match ix {
        // Fifties: a kidney.
        2 => (0..72)
            .map(|k| {
                let t = TAU * k as f64 / 72.0;
                let (x, y) = (a * t.cos(), h * t.sin());
                // The waist pinched in on one side.
                let pinch = 0.32 * h * (-(x / (0.38 * a)).powi(2)).exp();
                let y = if y > 0.0 {
                    y - pinch * (y / h).powf(0.6)
                } else {
                    y
                };
                [x, cy + y]
            })
            .collect(),
        // Classic: a rectangle ending in a half round to the west.
        7 => {
            let r = h;
            let mut p = vec![[-a + r, cy - h], [a - 0.08, cy - h]];
            for k in 0..=4 {
                let t = -FRAC_PI_2 + FRAC_PI_2 * k as f64 / 4.0;
                p.push([a - 0.08 + 0.08 * t.cos(), cy - h + 0.08 + 0.08 * t.sin()]);
            }
            p.push([a, cy + h - 0.08]);
            for k in 0..=4 {
                let t = FRAC_PI_2 * k as f64 / 4.0;
                p.push([a - 0.08 + 0.08 * t.cos(), cy + h - 0.08 + 0.08 * t.sin()]);
            }
            p.push([-a + r, cy + h]);
            for k in 1..24 {
                let t = FRAC_PI_2 + PI * k as f64 / 24.0;
                p.push([-a + r + r * t.cos(), cy + r * t.sin()]);
            }
            dedup(p)
        }
        // A rectangle with small round corners.
        _ => {
            let r = 0.08;
            let mut p = Vec::new();
            for (cx, cy2, a0) in [
                (a - r, cy - h + r, -FRAC_PI_2),
                (a - r, cy + h - r, 0.0),
                (-a + r, cy + h - r, FRAC_PI_2),
                (-a + r, cy - h + r, PI),
            ] {
                for k in 0..=3 {
                    let t = a0 + FRAC_PI_2 * k as f64 / 3.0;
                    p.push([cx + r * t.cos(), cy2 + r * t.sin()]);
                }
            }
            p
        }
    }
}

/// The loop without corners closer than a millimetre.
fn dedup(p: Loop) -> Loop {
    let mut out: Loop = Vec::with_capacity(p.len());
    for q in p {
        if out
            .last()
            .is_none_or(|l: &[f64; 2]| (l[0] - q[0]).hypot(l[1] - q[1]) > 1e-3)
        {
            out.push(q);
        }
    }
    if out.len() > 2 {
        let (f, l) = (out[0], out[out.len() - 1]);
        if (f[0] - l[0]).hypot(f[1] - l[1]) < 1e-3 {
            out.pop();
        }
    }
    out
}

/// What a pool takes out of the ground: the outside of its shell in plan
/// (counter-clockwise, the pool's own axes) and the level of its bottom
/// (negative, metres under its coping).
pub fn excavation(p: &Params) -> (Vec<[f64; 2]>, f64) {
    let ix = look(p);
    let l = p.get("length", 8.0).clamp(2.0, 25.0);
    let w = p.get("width", 4.0).clamp(1.5, 12.0);
    let depth = p.get("water_depth", 1.4).clamp(0.4, 2.5);
    let deck = p.get("deck_width", 1.0).clamp(0.0, 4.0);
    let form = p.get("form", 1.0).round() as i32;
    let infinity = form == 3 && !matches!(ix, 2 | 7);
    let coping = if infinity { 0.5 } else { 0.32 };
    let cy = coping + deck + w / 2.0;
    let edge = outline(ix, l, w, cy);
    let edge: Loop = if area2(&edge) < 0.0 {
        edge.into_iter().rev().collect()
    } else {
        edge
    };
    let zw = if infinity { -0.015 } else { -0.12 };
    let wall = 0.2;
    (offset(&edge, wall), zw - depth - wall)
}

/// Pools of the collections.
pub fn pool(b: &mut Builder, p: &Params) {
    let ix = look(p);
    let l = p.get("length", 8.0).clamp(2.0, 25.0);
    let w = p.get("width", 4.0).clamp(1.5, 12.0);
    let depth = p.get("water_depth", 1.4).clamp(0.4, 2.5);
    let deck = p.get("deck_width", 1.0).clamp(0.0, 4.0);
    let form = p.get("form", 1.0).round() as i32;
    let rect = !matches!(ix, 2 | 7);
    let infinity = form == 3 && rect;
    let roman = (form == 2 && rect) || ix == 7;
    let coping = if infinity { 0.5 } else { 0.32 };
    let margin = coping + deck;
    let cy = margin + w / 2.0;
    let edge = outline(ix, l, w, cy);
    let edge = if area2(&edge) < 0.0 {
        edge.into_iter().rev().collect()
    } else {
        edge
    };
    // Levels: the water's face, the floor of the shell.
    let zw = if infinity { -0.015 } else { -0.12 };
    let zf = zw - depth;
    let wall = 0.2;
    // The shell, its lining seen from the water.
    let top = if infinity { -0.03 } else { 0.0 };
    let skin = offset(&edge, 0.03);
    band(b, "lining", &edge, &skin, zf - 0.03, top);
    prism(b, "lining", &skin, zf - 0.03, zf);
    // The concrete shell round it, seen only where the ground is cut away.
    let outer_wall = offset(&edge, wall);
    band(b, "shell", &skin, &outer_wall, zf - wall, top);
    prism(b, "shell", &outer_wall, zf - wall, zf - 0.03);
    // Steps where the shell has them.
    if roman {
        let (corner, a0) = if ix == 7 {
            // In the half round end, as a whole half round of steps.
            ([-l / 2.0 + w / 2.0, cy], FRAC_PI_2)
        } else {
            ([-l / 2.0, cy - w / 2.0], 0.0)
        };
        let sweep_angle = if ix == 7 { PI } else { FRAC_PI_2 };
        let radii: [f64; 3] = if ix == 7 {
            [w / 2.0, w / 2.0 - 0.4, w / 2.0 - 0.8]
        } else {
            [1.6, 1.2, 0.8]
        };
        for (k, r) in radii.into_iter().enumerate() {
            let top = zw - 0.25 - (2 - k) as f64 * 0.25;
            let top = top.max(zf + 0.05);
            let n = 16;
            let mut pts = vec![[corner[0], corner[1]]];
            for s in 0..=n {
                let t = a0 + sweep_angle * s as f64 / n as f64;
                pts.push([corner[0] + r * t.cos(), corner[1] + r * t.sin()]);
            }
            if ix == 7 {
                // The half round is centred: keep its diameter as the loop.
                pts.remove(0);
            }
            let pts = if area2(&pts) < 0.0 {
                pts.into_iter().rev().collect()
            } else {
                pts
            };
            prism(b, "lining", &pts, zf, top);
        }
    }
    // The band of tiles at the water line.
    if matches!(ix, 1 | 2 | 7) {
        band(
            b,
            "accent",
            &offset(&edge, -0.006),
            &edge,
            zw - 0.18,
            -0.002,
        );
    }
    // The water.
    prism(b, "water", &offset(&edge, -0.004), zf + 0.002, zw);
    // The coping and the deck, or the overflow grate round an infinity
    // edge.
    if infinity {
        let g = offset(&edge, 0.2);
        band(b, "dark", &edge, &g, -0.04, -0.012);
        band(b, "coping", &g, &offset(&edge, coping), -0.04, 0.0);
    } else {
        let lip = offset(&edge, -0.03);
        band(b, "coping", &lip, &offset(&edge, coping), 0.0, 0.05);
    }
    if deck > 0.01 {
        let inner = offset(&edge, coping);
        let outer = offset(&edge, coping + deck);
        band(b, "deck", &inner, &outer, 0.0, 0.03);
    }
    // The lights under water, along the long sides.
    let n = ((l / 3.0).round() as usize).max(1);
    for i in 0..n {
        let x = -l / 2.0 + l * (i as f64 + 0.5) / n as f64;
        if ix == 7 && x < -l / 2.0 + w / 2.0 {
            continue;
        }
        for (y, s) in [(cy - w / 2.0, 1.0), (cy + w / 2.0, -1.0)] {
            if ix == 2 {
                continue;
            }
            b.ellipsoid("light", [x, y + s * 0.006, zw - 0.45], [0.11, 0.012, 0.11]);
        }
    }
    if ix == 2 {
        for k in 0..4 {
            let t = TAU * (k as f64 + 0.5) / 4.0;
            let q = [l / 2.0 * 0.92 * t.cos(), cy + w / 2.0 * 0.85 * t.sin()];
            b.ellipsoid("light", [q[0], q[1], zf + 0.012], [0.12, 0.12, 0.01]);
        }
    }
    // The ladder, on the far long side towards its east end.
    let lx = if ix == 2 { l * 0.22 } else { l / 2.0 - 1.2 };
    let ly = if ix == 2 {
        cy - w / 2.0 * 0.82
    } else {
        cy - w / 2.0
    };
    let ladder_side = -1.0;
    let r = 0.021;
    let top = if infinity { 0.75 } else { 0.8 };
    for dx in [-0.25, 0.25] {
        let x = lx + dx;
        let out = ladder_side;
        let path = curve(
            &[
                [x, ly + out * 0.45, 0.0],
                [x, ly + out * 0.45, top - 0.25],
                [x, ly + out * 0.3, top],
                [x, ly + out * 0.02, top - 0.05],
                [x, ly - out * 0.12, top - 0.35],
                [x, ly - out * 0.12, zw - 0.5],
                [x, ly - out * 0.05, zw - 1.0_f64.min(depth - 0.1)],
            ],
            5,
        );
        sweep(
            b,
            "metal",
            &path,
            |_| r,
            12,
            Cap::Round(1.0),
            Cap::Round(1.0),
        );
        b.cyl(
            "metal",
            [x, ly + out * 0.45, 0.0],
            [x, ly + out * 0.45, 0.012],
            0.04,
        );
    }
    for k in 0..3 {
        let z = zw - 0.1 - 0.28 * k as f64;
        if z < zf + 0.1 {
            break;
        }
        b.rbox(
            "metal",
            [lx - 0.25, ly - ladder_side * 0.06, z - 0.01],
            [lx + 0.25, ly - ladder_side * 0.16, z + 0.01],
            0.004,
        );
    }
}

// ------------------------------------------------------------------ shower

/// Outdoor showers of the collections.
pub fn pool_shower(b: &mut Builder, p: &Params) {
    let ix = look(p);
    let h = p.get("height", 2.2).clamp(1.8, 2.6);
    let c = [0.0, 0.2];
    // The tray under it.
    match ix {
        0 => {
            for k in 0..6 {
                let x = -0.4 + 0.8 * k as f64 / 6.0;
                b.rbox(
                    "frame",
                    [x + 0.008, -0.2, 0.0],
                    [x + 0.8 / 6.0 - 0.008, 0.6, 0.04],
                    0.004,
                );
            }
        }
        1 | 5 => {
            let mut rnd = Rng(5);
            b.rbox("deck", [-0.45, -0.25, 0.0], [0.45, 0.65, 0.03], 0.01);
            for _ in 0..40 {
                let q = [rnd.range(-0.4, 0.4), rnd.range(-0.2, 0.6)];
                let s = rnd.range(0.025, 0.045);
                b.ellipsoid("masonry", [q[0], q[1], 0.03], [s, s * 0.8, s * 0.4]);
            }
        }
        2 => disc(b, "deck", [c[0], c[1] + 0.15], 0.45, 0.0, 0.03),
        3 => {
            stone(b, "deck", [-0.4, -0.2, 0.0], [0.4, 0.6, 0.05], 0.2);
        }
        4 | 6 => {
            b.rbox("dark", [-0.4, -0.2, 0.0], [0.4, 0.6, 0.015], 0.0);
            for k in 1..16 {
                let x = -0.4 + 0.8 * k as f64 / 16.0;
                b.rbox(
                    "metal",
                    [x - 0.003, -0.19, 0.015],
                    [x + 0.003, 0.59, 0.02],
                    0.0,
                );
            }
        }
        _ => b.rbox("deck", [-0.45, -0.25, 0.0], [0.45, 0.65, 0.04], 0.015),
    }
    let base = 0.04;
    // The column and its head, in the collection's way.
    let arm = 0.45;
    let head_at = [c[0], c[1] + arm, h - 0.05];
    match ix {
        // Nordic: a square post of oak with a steel arm and a round head.
        0 => {
            b.rbox("frame", [-0.05, 0.0, base], [0.05, 0.1, h], 0.006);
            rod(b, "metal", [0.0, 0.1, h - 0.05], head_at, 0.012);
            disc(b, "metal", [head_at[0], head_at[1]], 0.1, h - 0.07, 0.015);
            b.cyl("metal", [0.0, 0.1, 1.0], [0.0, 0.14, 1.0], 0.025);
        }
        // Mediterranean: a whitewashed pillar with a band of tiles and a
        // brass spout.
        1 => {
            b.rbox("masonry", [-0.15, 0.0, base], [0.15, 0.2, h], 0.02);
            b.rbox("accent", [-0.15, 0.2, 1.1], [0.15, 0.206, 1.4], 0.0);
            sweep(
                b,
                "metal",
                &curve(
                    &[
                        [0.0, 0.2, h - 0.15],
                        [0.0, 0.35, h - 0.12],
                        [0.0, 0.5, h - 0.2],
                    ],
                    5,
                ),
                |_| 0.014,
                10,
                Cap::Flat,
                Cap::Flat,
            );
            turned(
                b,
                "metal",
                [0.0, 0.5, h - 0.28],
                &[(0.0, 0.0), (0.09, 0.0), (0.04, 0.08), (0.0, 0.08)],
                18,
                0.0,
            );
            b.cyl("metal", [0.0, 0.2, 1.0], [0.0, 0.24, 1.0], 0.03);
        }
        // Fifties: a chromed column bent over at the top.
        2 => {
            let path = curve(
                &[
                    [0.0, c[1], base],
                    [0.0, c[1], h - 0.3],
                    [0.0, c[1] + 0.05, h],
                    [0.0, c[1] + 0.3, h + 0.02],
                    [0.0, head_at[1], h - 0.08],
                ],
                6,
            );
            sweep(b, "metal", &path, |_| 0.025, 14, Cap::Flat, Cap::Flat);
            turned(
                b,
                "metal",
                [head_at[0], head_at[1], h - 0.22],
                &[(0.0, 0.0), (0.12, 0.0), (0.03, 0.12), (0.0, 0.14)],
                20,
                0.0,
            );
            disc(b, "metal", c, 0.08, base, 0.03);
        }
        // Japandi: a post of wood with a bamboo spout pouring into the tray.
        3 => {
            b.rbox("frame", [-0.06, 0.0, base], [0.06, 0.12, h], 0.004);
            sweep(
                b,
                "accent",
                &[[0.0, 0.12, h - 0.2], [0.0, 0.5, h - 0.25]],
                |_| 0.025,
                10,
                Cap::Flat,
                Cap::Flat,
            );
            b.rbox("frame", [-0.08, 0.0, h], [0.08, 0.14, h + 0.03], 0.004);
        }
        // Contemporary: a slim column of steel with a flat square head.
        4 => {
            b.rbox("metal", [-0.04, 0.0, base], [0.04, 0.06, h], 0.004);
            b.rbox(
                "metal",
                [-0.02, 0.06, h - 0.04],
                [0.02, head_at[1], h - 0.01],
                0.003,
            );
            b.rbox(
                "metal",
                [-0.12, head_at[1] - 0.12, h - 0.06],
                [0.12, head_at[1] + 0.12, h - 0.04],
                0.004,
            );
        }
        // Rustic: a wooden post, an iron pipe and a watering-can head.
        5 => {
            b.rbox("frame", [-0.06, 0.0, base], [0.06, 0.12, h], 0.01);
            sweep(
                b,
                "dark",
                &curve(
                    &[
                        [0.08, 0.13, 0.9],
                        [0.08, 0.13, h - 0.1],
                        [0.08, 0.3, h - 0.02],
                        [0.08, head_at[1], h - 0.1],
                    ],
                    5,
                ),
                |_| 0.016,
                10,
                Cap::Flat,
                Cap::Flat,
            );
            turned(
                b,
                "dark",
                [0.08, head_at[1], h - 0.24],
                &[(0.0, 0.0), (0.1, 0.0), (0.05, 0.12), (0.0, 0.14)],
                16,
                0.0,
            );
            b.cyl("dark", [0.08, 0.13, 1.0], [0.08, 0.2, 1.0], 0.02);
        }
        // Industrial: black pipe with its elbows and a valve wheel.
        6 => {
            let path = vec![[0.0, c[1], base], [0.0, c[1], h], [0.0, head_at[1], h]];
            sweep(b, "dark", &path, |_| 0.021, 12, Cap::Flat, Cap::Flat);
            b.ellipsoid("dark", [0.0, c[1], h], [0.03, 0.03, 0.03]);
            disc(b, "dark", c, 0.07, base, 0.015);
            turned(
                b,
                "metal",
                [head_at[0], head_at[1], h - 0.16],
                &[
                    (0.0, 0.0),
                    (0.11, 0.0),
                    (0.11, 0.02),
                    (0.02, 0.16),
                    (0.0, 0.16),
                ],
                16,
                0.0,
            );
            tube(
                b,
                "accent",
                &super::hospitality::ring([0.0, c[1] + 0.07], 0.06, 1.1, 16)
                    .iter()
                    .map(|q| [q[0], c[1] + 0.07, 1.1 + (q[1] - c[1] - 0.07)])
                    .collect::<Vec<P3>>(),
                true,
                0.008,
                6,
            );
        }
        // Classic: a fluted brass column with a large rose.
        _ => {
            turned(
                b,
                "metal",
                [c[0], c[1], base],
                &[
                    (0.0, 0.0),
                    (0.07, 0.0),
                    (0.07, 0.04),
                    (0.035, 0.1),
                    (0.028, h - 0.3),
                    (0.04, h - 0.25),
                    (0.03, h - 0.2),
                    (0.0, h - 0.2),
                ],
                16,
                0.0,
            );
            sweep(
                b,
                "metal",
                &curve(
                    &[
                        [0.0, c[1], h - 0.22],
                        [0.0, c[1], h - 0.05],
                        [0.0, c[1] + 0.15, h],
                        [0.0, head_at[1], h - 0.1],
                    ],
                    6,
                ),
                |_| 0.016,
                12,
                Cap::Flat,
                Cap::Flat,
            );
            turned(
                b,
                "metal",
                [head_at[0], head_at[1], h - 0.26],
                &[
                    (0.0, 0.0),
                    (0.15, 0.0),
                    (0.15, 0.015),
                    (0.03, 0.14),
                    (0.0, 0.16),
                ],
                24,
                0.0,
            );
            for z in [0.95, 1.05] {
                b.ellipsoid("metal", [0.0, c[1] + 0.03, z], [0.035, 0.03, 0.02]);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fine::shape::signed_volume;

    #[test]
    fn the_pools_of_the_eight_collections() {
        for c in 0..8 {
            for form in [1.0, 2.0, 3.0] {
                let mut p = Params::default();
                p.set("style", (1 + c) as f64);
                p.set("form", form);
                let mut b = Builder::default();
                pool(&mut b, &p);
                let tris = b.triangle_count();
                assert!((100..40_000).contains(&tris), "{c} {form}: {tris}");
                for (role, t) in &b.parts {
                    let v = signed_volume(&b.positions, t);
                    assert!(v > -1e-7, "{c} {form} {role}: {v}");
                }
                let low = b.positions.iter().map(|q| q[2]).fold(f32::MAX, f32::min);
                assert!(low > -1.85, "{c} {form}: {low}");
            }
            let mut p = Params::default();
            p.set("style", (1 + c) as f64);
            let mut b = Builder::default();
            pool_shower(&mut b, &p);
            assert!((50..40_000).contains(&b.triangle_count()), "{c} ducha");
            for (role, t) in &b.parts {
                let v = signed_volume(&b.positions, t);
                assert!(v > -1e-7, "{c} ducha {role}: {v}");
            }
            let low = b.positions.iter().map(|q| q[2]).fold(f32::MAX, f32::min);
            assert!(low > -0.001, "{c} ducha: {low}");
        }
    }

    #[test]
    fn the_excavation_holds_the_shell() {
        for c in 0..8 {
            for form in [1.0, 2.0, 3.0] {
                let mut p = Params::default();
                p.set("style", (1 + c) as f64);
                p.set("form", form);
                let (hole, bottom) = excavation(&p);
                assert!(hole.len() >= 4 && area2(&hole) > 0.0, "{c}");
                let mut b = Builder::default();
                pool(&mut b, &p);
                let low = b
                    .positions
                    .iter()
                    .map(|q| q[2] as f64)
                    .fold(f64::MAX, f64::min);
                assert!((low - bottom).abs() < 0.01, "{c} {form}: {low} {bottom}");
            }
        }
    }

    #[test]
    fn the_forms_are_chosen_by_name() {
        assert_eq!(crate::choice_number("form", "Desbordante"), Some(3.0));
        assert_eq!(
            crate::choice_number("form", "Con escalera romana"),
            Some(2.0)
        );
    }

    #[test]
    fn a_kidney_is_cut_into_triangles_whole() {
        let edge = outline(2, 8.0, 4.0, 3.0);
        let edge: Loop = if area2(&edge) < 0.0 {
            edge.into_iter().rev().collect()
        } else {
            edge
        };
        let tris = triangulate(&edge);
        assert_eq!(tris.len(), edge.len() - 2);
        let sum: f64 = tris
            .iter()
            .map(|[a, b, c]| area2(&[edge[*a], edge[*b], edge[*c]]))
            .sum();
        assert!((sum - area2(&edge)).abs() < 1e-6);
    }
}
