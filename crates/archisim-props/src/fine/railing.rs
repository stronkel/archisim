//! Railings (E12, barandillas): one straight run of a railing at a time,
//! along `x` from 0 to `run_len`, its axis at `y` = 0, its foot at `z` =
//! 0, `height` to the top of the handrail. ArchiSim puts a run on each
//! segment of a railing's line; at a corner the run that starts there
//! draws the corner post (`start` 1) and the one that ends there does not
//! (`end` 1).
//!
//! What it is made of, by collection («Modelo 3D») or chosen («A tu
//! gusto»): the posts, the handrail, what fills between them (bars,
//! turned balusters, glass, sheet, slats, cables, mesh, scrolls), the
//! post at a free end and how it is fixed. Bars, balusters and slats keep
//! the gaps under `bar_gap` (10 cm, CTE DB-SUA 1).

use super::shape::{sweep, turned, Cap};
use crate::build::Builder;
use crate::Params;

const STRUCT: &str = "structure_material";
const RAIL: &str = "rail_material";
const FILL: &str = "infill_material";
const HW: &str = "hardware_material";

/// The collections a railing type can take its look from.
pub const COLLECTIONS: [&str; 8] = super::door::COLLECTIONS;

/// The posts, by name (`rail_post`).
pub const POSTS: &[&str] = &[
    "Como la colección",
    "Cuadrado",
    "Redondo",
    "Pletina",
    "Sin postes",
];

/// The handrails, by name (`rail_top`).
pub const RAILS: &[&str] = &[
    "Como la colección",
    "Rectangular",
    "Redondo",
    "Pletina",
    "Madera moldurado",
    "Sin pasamanos",
];

/// What fills between the posts, by name (`rail_infill`).
pub const INFILLS: &[&str] = &[
    "Como la colección",
    "Barrotes",
    "Barrotes y pletina",
    "Balaustres torneados",
    "Vidrio",
    "Vidrio con pinzas",
    "Chapa perforada",
    "Chapa lisa",
    "Lamas horizontales",
    "Cables",
    "Malla",
    "Forja con volutas",
];

/// The post at a free end (arranque), by name (`rail_end`).
pub const ENDS: &[&str] = &[
    "Como la colección",
    "Igual que los demás",
    "Más grueso",
    "Con bola",
    "Pilastra",
];

/// How it is fixed, by name (`rail_anchor`).
pub const ANCHORS: &[&str] = &[
    "Como la colección",
    "Placa al suelo",
    "Al canto",
    "Empotrado",
];

/// Each collection's post, handrail, infill, end post and anchor (numbers
/// in the lists above).
const LOOKS: [(usize, usize, usize, usize, usize); 8] = [
    (2, 2, 2, 2, 2),  // Nórdico cálido: square posts, wooden rail, bars.
    (2, 4, 12, 4, 2), // Mediterráneo: wrought iron with scrolls, balls.
    (4, 4, 3, 2, 2),  // Años 50: flat bars with a rail.
    (2, 2, 9, 2, 2),  // Japandi: horizontal slats.
    (5, 3, 5, 2, 3),  // Contemporáneo: glass in a shoe, round rail.
    (2, 5, 4, 3, 2),  // Rústica: turned balusters, thicker end posts.
    (3, 3, 10, 2, 2), // Industrial: round posts and cables.
    (2, 5, 4, 5, 2),  // Clásica: balusters and a newel.
];

#[derive(Clone, Copy)]
struct Look {
    post: usize,
    rail: usize,
    infill: usize,
    end: usize,
    anchor: usize,
}

fn look(p: &Params) -> Look {
    let s = (p.style() as usize).clamp(1, 8);
    let (post, rail, infill, end, anchor) = LOOKS[s - 1];
    let mut l = Look {
        post,
        rail,
        infill,
        end,
        anchor,
    };
    let pick = |key: &str| {
        let n = p.get(key, 1.0).round() as usize;
        (n >= 2).then_some(n)
    };
    if let Some(n) = pick("rail_post") {
        l.post = n;
    }
    if let Some(n) = pick("rail_top") {
        l.rail = n;
    }
    if let Some(n) = pick("rail_infill") {
        l.infill = n;
    }
    if let Some(n) = pick("rail_end") {
        l.end = n;
    }
    if let Some(n) = pick("rail_anchor") {
        l.anchor = n;
    }
    // Glass in a shoe needs no posts; anything else without posts would
    // have nothing to stand on.
    if l.post == 5 && !matches!(l.infill, 5 | 6) {
        l.post = 2;
    }
    l
}

/// One straight run of a railing.
pub fn railing_run(b: &mut Builder, p: &Params) {
    let l = look(p);
    let len = p.get("run_len", 2.0).max(0.1);
    let h = p.get("height", 1.1).clamp(0.3, 3.0);
    let spacing = p.get("post_spacing", 1.5).clamp(0.3, 5.0);
    let gap = p.get("bar_gap", 0.10).clamp(0.03, 0.25);
    let start = p.get("start", 0.0).round() as i32;
    let end = p.get("end", 0.0).round() as i32;
    let far = p.flag("lod", false);
    let posts = l.post != 5;
    // The handrail and the posts' sizes.
    let (rail_w, rail_h) = match l.rail {
        3 => (0.045, 0.045),
        4 => (0.05, 0.010),
        5 => (0.075, 0.055),
        6 => (0.0, 0.0),
        _ => (0.06, 0.045),
    };
    let ps = match l.post {
        3 => 0.045,
        4 => 0.05,
        _ => 0.04,
    };
    // Where the posts go: a bay no longer than the spacing.
    let bays = ((len / spacing) - 1e-9).ceil().max(1.0) as usize;
    let at = |k: usize| len * k as f64 / bays as f64;
    let top = h - rail_h;
    // The rail runs a little past a corner, into the next run's.
    let (x0, x1) = (
        if start == 0 { -ps / 2.0 } else { -rail_w / 2.0 },
        if end == 0 {
            len + ps / 2.0
        } else {
            len + rail_w / 2.0
        },
    );
    // ---- Posts.
    if posts {
        for k in 0..=bays {
            if (k == 0 && start == 2) || (k == bays && end != 0) {
                continue;
            }
            let free_end = (k == 0 && start == 0) || (k == bays && end == 0);
            let x = at(k);
            post(b, &l, x, ps, top, free_end, far);
            if !far {
                anchor(b, &l, x, ps);
            }
        }
    } else {
        // A shoe the glass stands in, along the whole run.
        let (sw, sh) = (0.06, 0.11);
        b.rbox(
            STRUCT,
            [x0.min(0.0), -sw / 2.0, 0.0],
            [x1.max(len), sw / 2.0, sh],
            0.003,
        );
        if l.anchor == 3 && !far {
            // On the edge: a plate down the slab's face.
            b.rbox(
                STRUCT,
                [x0.min(0.0), -sw / 2.0 - 0.012, -0.16],
                [x1.max(len), -sw / 2.0, sh],
                0.002,
            );
        }
    }
    // ---- Handrail.
    if rail_h > 0.0 {
        handrail(b, &l, x0, x1, top, rail_w, rail_h, start, end, far);
    }
    // ---- What fills each bay.
    let zb = if posts { 0.10 } else { 0.11 };
    let zt = if rail_h > 0.0 { top } else { h };
    if !posts {
        // Glass panels in the shoe, a joint every 1.2 m or so.
        let n = ((x1 - x0) / 1.25).ceil().max(1.0) as usize;
        let w = (x1 - x0) / n as f64;
        for i in 0..n {
            let (a, c) = (x0 + i as f64 * w + 0.004, x0 + (i + 1) as f64 * w - 0.004);
            b.rbox(FILL, [a, -0.006, 0.03], [c, 0.006, zt - 0.02], 0.0);
        }
        // The glass's top edge in a channel the handrail sits on (or is
        // the top itself), so that the glass reads from far away.
        b.rbox(
            RAIL,
            [x0, -0.013, zt - 0.03],
            [x1, 0.013, zt + 0.001],
            0.002,
        );
        return;
    }
    // (A flat-bar post is thin along the run.)
    let px = if l.post == 4 { 0.012 } else { ps };
    for k in 0..bays {
        let (a, c) = (at(k) + px / 2.0, at(k + 1) - px / 2.0);
        if c - a < 0.02 {
            continue;
        }
        infill(b, &l, a, c, zb, zt, gap, far);
    }
}

/// A post at `x`, `ps` across, up to `top`; a thicker, crowned or newel
/// one at a free end when the look says.
fn post(b: &mut Builder, l: &Look, x: f64, ps: f64, top: f64, free_end: bool, far: bool) {
    let foot = match l.anchor {
        3 => -0.16,
        4 => -0.06,
        _ => 0.0,
    };
    let kind = if free_end { l.end } else { 2 };
    match kind {
        3 => {
            // Thicker.
            let s = ps * 1.7;
            b.rbox(
                STRUCT,
                [x - s / 2.0, -s / 2.0, foot],
                [x + s / 2.0, s / 2.0, top],
                0.004,
            );
            return;
        }
        5 => {
            // A newel: a square shaft, a base and a cap.
            let s = 0.10_f64.max(ps * 2.2);
            b.rbox(
                STRUCT,
                [x - s / 2.0 - 0.012, -s / 2.0 - 0.012, 0.0],
                [x + s / 2.0 + 0.012, s / 2.0 + 0.012, 0.12],
                0.004,
            );
            b.rbox(
                STRUCT,
                [x - s / 2.0, -s / 2.0, 0.12],
                [x + s / 2.0, s / 2.0, top + 0.06],
                0.004,
            );
            b.rbox(
                STRUCT,
                [x - s / 2.0 - 0.015, -s / 2.0 - 0.015, top + 0.06],
                [x + s / 2.0 + 0.015, s / 2.0 + 0.015, top + 0.095],
                0.006,
            );
            if !far {
                b.ellipsoid(STRUCT, [x, 0.0, top + 0.13], [0.04, 0.04, 0.04]);
            }
            return;
        }
        _ => {}
    }
    match l.post {
        3 => b.cyl(STRUCT, [x, 0.0, foot], [x, 0.0, top], ps / 2.0),
        4 => b.rbox(
            STRUCT,
            [x - 0.006, -ps / 2.0, foot],
            [x + 0.006, ps / 2.0, top],
            0.001,
        ),
        _ => b.rbox(
            STRUCT,
            [x - ps / 2.0, -ps / 2.0, foot],
            [x + ps / 2.0, ps / 2.0, top],
            0.003,
        ),
    }
    if kind == 4 && !far {
        // A ball on top, standing on the rail.
        b.ellipsoid(STRUCT, [x, 0.0, top + 0.1], [0.035, 0.035, 0.035]);
        b.cyl(STRUCT, [x, 0.0, top - 0.01], [x, 0.0, top + 0.07], 0.012);
    }
}

/// How a post is fixed: a plate on the floor, a plate on the slab's edge
/// (the post runs down past it) or a collar where it goes into the floor.
fn anchor(b: &mut Builder, l: &Look, x: f64, ps: f64) {
    match l.anchor {
        3 => {
            b.rbox(
                HW,
                [x - 0.05, -ps / 2.0 - 0.01, -0.15],
                [x + 0.05, -ps / 2.0, -0.02],
                0.002,
            );
        }
        4 => {
            b.cyl(HW, [x, 0.0, 0.0], [x, 0.0, 0.006], ps * 0.9);
        }
        _ => {
            b.rbox(HW, [x - 0.06, -0.06, 0.0], [x + 0.06, 0.06, 0.008], 0.002);
            for (dx, dy) in [(-0.04, -0.04), (0.04, -0.04), (-0.04, 0.04), (0.04, 0.04)] {
                b.cyl(HW, [x + dx, dy, 0.008], [x + dx, dy, 0.014], 0.006);
            }
        }
    }
}

/// The handrail from `x0` to `x1`, its underside at `top`.
#[allow(clippy::too_many_arguments)]
fn handrail(
    b: &mut Builder,
    l: &Look,
    x0: f64,
    x1: f64,
    top: f64,
    w: f64,
    hgt: f64,
    start: i32,
    end: i32,
    far: bool,
) {
    match l.rail {
        3 => {
            let r = w / 2.0;
            let z = top + r;
            sweep(
                b,
                RAIL,
                &[[x0, 0.0, z], [x1, 0.0, z]],
                |_| r,
                if far { 8 } else { 18 },
                if start == 0 {
                    Cap::Round(0.6)
                } else {
                    Cap::Flat
                },
                if end == 0 { Cap::Round(0.6) } else { Cap::Flat },
            );
            // A ball over a corner so that the two runs meet round.
            if end != 0 && !far {
                b.ellipsoid(RAIL, [x1 - w / 2.0, 0.0, z], [r, r, r]);
            }
        }
        4 => {
            b.rbox(RAIL, [x0, -w / 2.0, top], [x1, w / 2.0, top + hgt], 0.002);
        }
        5 => {
            // Moulded: a wide rounded top over a narrower neck.
            b.rbox(
                RAIL,
                [x0, -w / 2.0, top + hgt * 0.35],
                [x1, w / 2.0, top + hgt],
                0.018,
            );
            b.rbox(
                RAIL,
                [x0, -w * 0.32, top],
                [x1, w * 0.32, top + hgt * 0.4],
                0.004,
            );
        }
        _ => {
            b.rbox(RAIL, [x0, -w / 2.0, top], [x1, w / 2.0, top + hgt], 0.008);
        }
    }
}

/// What fills a bay from `a` to `c`, between `zb` and `zt`.
#[allow(clippy::too_many_arguments)]
fn infill(b: &mut Builder, l: &Look, a: f64, c: f64, zb: f64, zt: f64, gap: f64, far: bool) {
    let rail = |b: &mut Builder, z: f64, t: f64, s: f64| {
        b.rbox(
            STRUCT,
            [a - 0.004, -s / 2.0, z],
            [c + 0.004, s / 2.0, z + t],
            0.001,
        );
    };
    // Bars spread so that no gap is over `gap`: `n` bars of `w` across.
    let spread = |w: f64| {
        let clear = c - a;
        let n = ((clear - gap) / (gap + w)).ceil().max(0.0) as usize;
        let step = clear / (n + 1) as f64;
        (1..=n).map(move |i| a + step * i as f64)
    };
    match l.infill {
        3 | 12 => {
            // Bars with a flat bar along the top (and scrolls).
            rail(b, zb - 0.02, 0.02, 0.03);
            rail(b, zt - 0.03, 0.03, 0.012);
            let bw = 0.016;
            let xs: Vec<f64> = spread(bw).collect();
            for &x in &xs {
                b.rbox(
                    STRUCT,
                    [x - bw / 2.0, -bw / 2.0, zb],
                    [x + bw / 2.0, bw / 2.0, zt - 0.03],
                    0.0,
                );
            }
            if l.infill == 12 && !far {
                // A pair of C scrolls between every two bars, top and bottom.
                let mut edges = vec![a];
                edges.extend(xs.iter().copied());
                edges.push(c);
                for w in edges.windows(2) {
                    let (p, q) = (w[0], w[1]);
                    let r = ((q - p) / 2.0 - 0.006).min(0.05);
                    if r < 0.02 {
                        continue;
                    }
                    let m = (p + q) / 2.0;
                    scroll(b, [m, 0.0, zt - 0.03 - r], r, -1.0, false);
                    scroll(b, [m, 0.0, zb + r], r, 1.0, true);
                }
            }
        }
        4 => {
            // Turned balusters on a base rail and under a top rail.
            rail(b, zb - 0.04, 0.06, 0.07);
            rail(b, zt - 0.05, 0.05, 0.07);
            let r = 0.032;
            let (z0, z1) = (zb + 0.02, zt - 0.05);
            let hh = z1 - z0;
            for x in spread(2.0 * r) {
                if far {
                    b.rbox(
                        FILL,
                        [x - r * 0.7, -r * 0.7, z0],
                        [x + r * 0.7, r * 0.7, z1],
                        0.0,
                    );
                    continue;
                }
                let prof = [
                    (0.0, 0.0),
                    (r * 0.95, 0.0),
                    (r * 0.95, hh * 0.08),
                    (r * 0.55, hh * 0.12),
                    (r * 0.6, hh * 0.2),
                    (r, hh * 0.42),
                    (r * 0.85, hh * 0.6),
                    (r * 0.45, hh * 0.78),
                    (r * 0.6, hh * 0.86),
                    (r * 0.6, hh * 0.9),
                    (r * 0.95, hh * 0.93),
                    (r * 0.95, hh),
                    (0.0, hh),
                ];
                turned(b, FILL, [x, 0.0, z0], &prof, 16, 0.0);
            }
        }
        5 | 6 => {
            // Glass: into the posts, or held by clamps.
            rail(b, zb - 0.02, 0.02, 0.03);
            let clamps = l.infill == 6 && !far;
            let (ga, gc) = if clamps {
                (a + 0.03, c - 0.03)
            } else {
                (a - 0.006, c + 0.006)
            };
            let (gb, gt) = if clamps {
                (zb + 0.05, zt - 0.06)
            } else {
                (zb, zt)
            };
            if clamps {
                b.rbox(FILL, [ga, -0.006, gb], [gc, 0.006, gt], 0.0);
            } else {
                // Into the posts, its top in a channel under the rail.
                b.rbox(FILL, [ga, -0.006, gb], [gc, 0.006, gt - 0.02], 0.0);
                b.rbox(
                    STRUCT,
                    [a - 0.004, -0.012, gt - 0.03],
                    [c + 0.004, 0.012, gt],
                    0.001,
                );
            }
            if clamps && !far {
                for x in [a, c] {
                    let s = if x == a { 1.0 } else { -1.0 };
                    for z in [gb + 0.12, gt - 0.12] {
                        b.rbox(
                            HW,
                            [x.min(x + s * 0.05), -0.014, z - 0.03],
                            [x.max(x + s * 0.05), 0.014, z + 0.03],
                            0.004,
                        );
                    }
                }
            }
        }
        7 | 8 => {
            // A sheet in a frame of flat bars.
            let f = 0.03;
            rail(b, zb - 0.02, f, 0.025);
            rail(b, zt - f, f, 0.025);
            b.rbox(STRUCT, [a - 0.004, -0.0125, zb], [a + f, 0.0125, zt], 0.001);
            b.rbox(STRUCT, [c - f, -0.0125, zb], [c + 0.004, 0.0125, zt], 0.001);
            b.rbox(
                FILL,
                [a + f - 0.002, -0.002, zb + f - 0.02 - 0.002],
                [c - f + 0.002, 0.002, zt - f + 0.002],
                0.0,
            );
        }
        9 => {
            // Horizontal slats.
            let (t, sh) = (0.02, 0.09);
            let n = ((zt - zb + gap) / (sh + gap)).floor().max(1.0) as usize;
            let step = if n > 1 {
                (zt - zb - sh) / (n - 1) as f64
            } else {
                0.0
            };
            for i in 0..n {
                let z = zb + step * i as f64;
                b.rbox(
                    FILL,
                    [a - 0.004, -t / 2.0, z],
                    [c + 0.004, t / 2.0, z + sh],
                    0.003,
                );
            }
        }
        10 => {
            // Cables every 8 cm, through the posts.
            rail(b, zb - 0.02, 0.02, 0.03);
            let mut z = zb + 0.08;
            while z < zt - 0.04 {
                b.cyl(HW, [a - 0.004, 0.0, z], [c + 0.004, 0.0, z], 0.003);
                z += 0.08;
            }
        }
        11 => {
            // A welded mesh in a frame.
            let f = 0.025;
            rail(b, zb - 0.02, f, 0.025);
            rail(b, zt - f, f, 0.025);
            b.rbox(STRUCT, [a - 0.004, -0.0125, zb], [a + f, 0.0125, zt], 0.001);
            b.rbox(STRUCT, [c - f, -0.0125, zb], [c + 0.004, 0.0125, zt], 0.001);
            if far {
                b.rbox(
                    FILL,
                    [a + f, -0.001, zb + 0.005],
                    [c - f, 0.001, zt - f],
                    0.0,
                );
            } else {
                let m = 0.05;
                let mut x = a + f + m;
                while x < c - f - 0.01 {
                    b.rbox(
                        FILL,
                        [x - 0.002, -0.002, zb + 0.005],
                        [x + 0.002, 0.002, zt - f],
                        0.0,
                    );
                    x += m;
                }
                let mut z = zb + f + m;
                while z < zt - f - 0.01 {
                    b.rbox(
                        FILL,
                        [a + f, -0.004, z - 0.002],
                        [c - f, 0.0, z + 0.002],
                        0.0,
                    );
                    z += m;
                }
            }
        }
        _ => {
            // Plain bars on a bottom rail.
            rail(b, zb - 0.02, 0.02, 0.03);
            let bw = 0.018;
            for x in spread(bw) {
                b.rbox(
                    FILL,
                    [x - bw / 2.0, -bw / 2.0, zb],
                    [x + bw / 2.0, bw / 2.0, zt + 0.001],
                    0.0,
                );
            }
        }
    }
}

/// A C scroll of radius `rad` in the railing's plane at `c`, open
/// towards `s` (±x), curling down (`flip`) or up.
fn scroll(b: &mut Builder, c: [f64; 3], rad: f64, s: f64, flip: bool) {
    let turns = 1.3;
    let k = 24;
    let path: Vec<[f64; 3]> = (0..=k)
        .map(|i| {
            let t = i as f64 / k as f64;
            let a = t * turns * std::f64::consts::TAU;
            let rr = rad * (1.0 - 0.7 * t);
            let dz = if flip { -1.0 } else { 1.0 };
            [c[0] + s * rr * a.cos(), c[1], c[2] + dz * rr * a.sin()]
        })
        .collect();
    sweep(b, STRUCT, &path, |_| 0.005, 6, Cap::Flat, Cap::Flat);
}
