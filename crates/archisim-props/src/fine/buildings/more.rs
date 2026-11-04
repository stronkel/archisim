//! More pieces for the buildings (tanda 2): roofs of one slope and hipped
//! roofs with their courses of tiles, plates in any plane, turning what
//! was made, low walls with their coping, palms and fruit trees, wells and
//! troughs.

use super::{out_tri, unit, Face, Xy};
use crate::build::{Builder, P3};

/// Turns what was made since `start` by `a` round the vertical through
/// `c`.
pub fn turn_from(b: &mut Builder, start: usize, c: [f64; 2], a: f64) {
    let (s, co) = a.sin_cos();
    b.transform_from(start, |q| {
        let (x, y) = (q[0] - c[0], q[1] - c[1]);
        [c[0] + x * co - y * s, c[1] + x * s + y * co, q[2]]
    });
}

/// A flat plate on a convex polygon of points in one plane, `th` thick
/// along its normal (upwards if it lies or leans).
pub fn plate(b: &mut Builder, role: &'static str, pts: &[P3], th: f64) {
    let n = pts.len();
    if n < 3 {
        return;
    }
    let e1 = sub(pts[1], pts[0]);
    let e2 = sub(pts[2], pts[0]);
    let mut nr = crate::build::cross(e1, e2);
    let l = (nr[0] * nr[0] + nr[1] * nr[1] + nr[2] * nr[2])
        .sqrt()
        .max(1e-9);
    nr = [nr[0] / l, nr[1] / l, nr[2] / l];
    if nr[2] < 0.0 {
        nr = [-nr[0], -nr[1], -nr[2]];
    }
    let top: Vec<P3> = pts
        .iter()
        .map(|p| [p[0] + nr[0] * th, p[1] + nr[1] * th, p[2] + nr[2] * th])
        .collect();
    let mut c = [0.0; 3];
    for p in pts.iter().chain(&top) {
        for k in 0..3 {
            c[k] += p[k] / (2 * n) as f64;
        }
    }
    for i in 1..n - 1 {
        out_tri(b, role, [pts[0], pts[i], pts[i + 1]], c);
        out_tri(b, role, [top[0], top[i], top[i + 1]], c);
    }
    for i in 0..n {
        let j = (i + 1) % n;
        out_tri(b, role, [pts[i], pts[j], top[j]], c);
        out_tri(b, role, [pts[i], top[j], top[i]], c);
    }
}

fn sub(a: P3, b: P3) -> P3 {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

/// A roof of one slope of curved tiles leaning out of a face, from the
/// line `d0` at `z0` (its high side) to `d1` at `z1`, from `s0` to `s1`.
pub fn pent_roof(
    b: &mut Builder,
    f: &Face,
    s: (f64, f64),
    (d0, z0): (f64, f64),
    (d1, z1): (f64, f64),
) {
    let th = 0.08;
    f.leaning(b, "roof", s, (z0, d0), (z1, d1), th);
    let n = ((s.1 - s.0) / 0.21).round().max(1.0) as usize;
    let r = 0.065;
    let slope = (z0 - z1) / (d1 - d0).abs().max(0.01);
    let past = 0.06;
    let sgn = (d1 - d0).signum();
    for i in 0..n {
        let x = s.0 + (s.1 - s.0) * (i as f64 + 0.5) / n as f64;
        let a = f.at(x, z0 + th + r * 0.5, d0);
        let e = f.at(x, z1 + th + r * 0.5 - past * slope, d1 + sgn * past);
        b.cone("tiles", a, e, r, r * 0.9, 6);
    }
    b.fine.insert("tiles");
}

/// A hipped roof of curved tiles over `x0`–`x1` by `y0`–`y1` with its
/// eaves at the top of the walls `z`, `eaves` out: four slopes at `pitch`,
/// the courses cut at the hips, ridge and hip tiles. A square plan is a
/// pyramid.
pub fn hipped_roof(
    b: &mut Builder,
    (x0, x1): (f64, f64),
    (y0, y1): (f64, f64),
    z: f64,
    pitch: f64,
    eaves: f64,
) {
    if y1 - y0 > x1 - x0 {
        // Made with the ridge along x, then turned a quarter.
        let c = [(x0 + x1) / 2.0, (y0 + y1) / 2.0];
        let (hx, hy) = ((x1 - x0) / 2.0, (y1 - y0) / 2.0);
        let start = b.mark();
        hipped_roof(
            b,
            (c[0] - hy, c[0] + hy),
            (c[1] - hx, c[1] + hx),
            z,
            pitch,
            eaves,
        );
        turn_from(b, start, c, std::f64::consts::FRAC_PI_2);
        return;
    }
    let (ax, bx, ay, by) = (x0 - eaves, x1 + eaves, y0 - eaves, y1 + eaves);
    let ze = z - eaves * pitch;
    let h = (by - ay) / 2.0;
    let ym = (ay + by) / 2.0;
    let zr = ze + h * pitch;
    let (rx0, rx1) = (ax + h, bx - h);
    let th = 0.07;
    plate(
        b,
        "roof",
        &[[ax, by, ze], [bx, by, ze], [rx1, ym, zr], [rx0, ym, zr]],
        th,
    );
    plate(
        b,
        "roof",
        &[[bx, ay, ze], [ax, ay, ze], [rx0, ym, zr], [rx1, ym, zr]],
        th,
    );
    plate(b, "roof", &[[ax, ay, ze], [ax, by, ze], [rx0, ym, zr]], th);
    plate(b, "roof", &[[bx, by, ze], [bx, ay, ze], [rx1, ym, zr]], th);
    let r = 0.06;
    let lift = th + r * 0.6;
    let past = 0.05;
    // The long slopes: courses across x.
    let n = ((bx - ax) / 0.21).round().max(1.0) as usize;
    for i in 0..n {
        let x = ax + (bx - ax) * (i as f64 + 0.5) / n as f64;
        let l = (x - ax).min(bx - x).min(h) - 0.03;
        if l < 0.1 {
            continue;
        }
        for (yy, sgn) in [(by, -1.0), (ay, 1.0)] {
            let a = [x, yy - sgn * past, ze - past * pitch + lift];
            let e = [x, yy + sgn * l, ze + l * pitch + lift];
            b.cone("tiles", a, e, r, r * 0.9, 6);
        }
    }
    // The ends: courses across y.
    let m = ((by - ay) / 0.21).round().max(1.0) as usize;
    for i in 0..m {
        let y = ay + (by - ay) * (i as f64 + 0.5) / m as f64;
        let l = (y - ay).min(by - y).min(h) - 0.03;
        if l < 0.1 {
            continue;
        }
        for (xx, sgn) in [(ax, 1.0), (bx, -1.0)] {
            let a = [xx - sgn * past, y, ze - past * pitch + lift];
            let e = [xx + sgn * l, y, ze + l * pitch + lift];
            b.cone("tiles", a, e, r, r * 0.9, 6);
        }
    }
    // Ridge and hips.
    let top = th + 0.05;
    if rx1 - rx0 > 0.05 {
        b.cone(
            "tiles",
            [rx0, ym, zr + top],
            [rx1, ym, zr + top],
            0.1,
            0.1,
            8,
        );
    }
    for (cx, cy, rx) in [(ax, ay, rx0), (ax, by, rx0), (bx, ay, rx1), (bx, by, rx1)] {
        b.cone(
            "tiles",
            [cx, cy, ze + top],
            [rx, ym, zr + top],
            0.09,
            0.09,
            8,
        );
    }
    b.fine.insert("tiles");
}

/// A gable roof with its ridge along `y` (as [`super::tile_roof`] turned).
#[allow(clippy::too_many_arguments)]
pub fn tile_roof_along_y(
    b: &mut Builder,
    (x0, x1): (f64, f64),
    (y0, y1): (f64, f64),
    z: f64,
    pitch: f64,
    eaves: (f64, f64),
    gables: (bool, bool),
) {
    let c = [(x0 + x1) / 2.0, (y0 + y1) / 2.0];
    let (hx, hy) = ((x1 - x0) / 2.0, (y1 - y0) / 2.0);
    let start = b.mark();
    super::tile_roof(
        b,
        (c[0] - hy, c[0] + hy),
        (c[1] - hx, c[1] + hx),
        z,
        pitch,
        eaves,
        gables,
    );
    turn_from(b, start, c, std::f64::consts::FRAC_PI_2);
}

/// A low wall (tapia) along a face from `s0` to `s1`, `high`, with its
/// coping of tiles.
pub fn tapia(b: &mut Builder, f: &Face, s: (f64, f64), high: f64, role: &'static str) {
    f.bx(b, role, s, (0.0, high), (-0.35, 0.0));
    f.leaning(b, "tiles", s, (high + 0.12, -0.175), (high, 0.05), 0.04);
    f.leaning(b, "tiles", s, (high + 0.12, -0.175), (high, -0.4), 0.04);
}

/// A palm: the trunk ringed, a little bent, and its crown of fronds.
pub fn palm(b: &mut Builder, base: [f64; 2], h: f64, seed: u64) {
    let lean = 0.06 + 0.06 * unit(seed, 1);
    let a = unit(seed, 2) * std::f64::consts::TAU;
    let (dx, dy) = (a.cos() * lean, a.sin() * lean);
    let n = 8;
    let at = |k: usize| {
        let t = k as f64 / n as f64;
        [base[0] + dx * h * t * t, base[1] + dy * h * t * t, h * t]
    };
    for k in 0..n {
        let r = 0.2 - 0.06 * k as f64 / n as f64;
        b.cone("trunk", at(k), at(k + 1), r + 0.02, r, 10);
    }
    let top = at(n);
    b.ellipsoid("trunk", [top[0], top[1], top[2] + 0.1], [0.32, 0.32, 0.4]);
    let fronds = 14;
    for i in 0..fronds {
        let ang = std::f64::consts::TAU * (i as f64 + 0.3 * unit(seed, 10 + i)) / fronds as f64;
        let (s, c) = ang.sin_cos();
        let up = 0.5 - 0.9 * unit(seed, 40 + i);
        let len = 1.9 + 0.6 * unit(seed, 70 + i);
        let mut p = [top[0], top[1], top[2] + 0.2];
        for seg in 0..3 {
            let droop = up - 0.55 * seg as f64;
            let l = len / 3.0;
            let q = [p[0] + c * l, p[1] + s * l, p[2] + droop * l];
            let mid = [
                (p[0] + q[0]) / 2.0,
                (p[1] + q[1]) / 2.0,
                (p[2] + q[2]) / 2.0,
            ];
            let wide = 0.32 - 0.08 * seg as f64;
            let ax = [
                (q[0] - p[0]) / 2.0,
                (q[1] - p[1]) / 2.0,
                (q[2] - p[2]) / 2.0,
            ];
            b.obox(
                "palm",
                mid,
                ax,
                [-s * wide, c * wide, 0.0],
                [0.0, 0.0, 0.012],
            );
            p = q;
        }
    }
    b.fine.insert("palm");
}

/// A fruit tree (orange or olive): trunk, a crown of clumps, its fruit.
pub fn fruit_tree(b: &mut Builder, base: [f64; 2], h: f64, seed: u64, fruit: bool) {
    let c = [base[0], base[1]];
    b.cone(
        "trunk",
        [c[0], c[1], 0.0],
        [c[0], c[1], h * 0.45],
        0.12,
        0.08,
        10,
    );
    let r = h * 0.32;
    let zc = h * 0.62;
    for k in 0..5 {
        let a = std::f64::consts::TAU * (k as f64 + unit(seed, k)) / 5.0;
        let off = if k == 0 { 0.0 } else { r * 0.45 };
        let cc = [
            c[0] + off * a.cos(),
            c[1] + off * a.sin(),
            zc + 0.25 * r * unit(seed, 10 + k),
        ];
        let rr = r * (0.75 + 0.2 * unit(seed, 20 + k));
        b.blob(
            "leaves",
            cc,
            [rr, 0.0, 0.0],
            [0.0, rr, 0.0],
            [0.0, 0.0, rr * 0.85],
        );
        if fruit {
            for j in 0..4 {
                let t = std::f64::consts::TAU * (j as f64 + unit(seed, 30 + k * 4 + j)) / 4.0;
                b.ellipsoid_n(
                    "fruit",
                    [
                        cc[0] + rr * 0.92 * t.cos(),
                        cc[1] + rr * 0.92 * t.sin(),
                        cc[2] + rr * 0.2 * (unit(seed, 50 + j) - 0.5),
                    ],
                    [0.06, 0.06, 0.06],
                    8,
                );
            }
        }
    }
}

/// A well: its round curb of whitewashed brick, the iron arch and the
/// pulley.
pub fn well(b: &mut Builder, c: [f64; 2]) {
    b.lathe(
        "wall",
        [c[0], c[1], 0.0],
        &[(0.75, 0.0), (0.75, 0.8), (0.62, 0.8), (0.62, 0.0)],
        1.0,
        1.0,
    );
    b.cone(
        "stone",
        [c[0], c[1], 0.8],
        [c[0], c[1], 0.88],
        0.78,
        0.78,
        20,
    );
    for dx in [-0.68, 0.68] {
        b.cyl(
            "iron",
            [c[0] + dx, c[1], 0.8],
            [c[0] + dx, c[1], 2.0],
            0.025,
        );
    }
    let n = 8;
    for k in 0..n {
        let a0 = std::f64::consts::PI * k as f64 / n as f64;
        let a1 = std::f64::consts::PI * (k + 1) as f64 / n as f64;
        b.cyl(
            "iron",
            [c[0] + 0.68 * a0.cos(), c[1], 2.0 + 0.45 * a0.sin()],
            [c[0] + 0.68 * a1.cos(), c[1], 2.0 + 0.45 * a1.sin()],
            0.022,
        );
    }
    b.cone(
        "iron",
        [c[0], c[1] - 0.04, 2.3],
        [c[0], c[1] + 0.04, 2.3],
        0.12,
        0.12,
        14,
    );
    b.cyl("iron", [c[0], c[1], 2.3], [c[0], c[1], 2.45], 0.015);
}

/// A trough of stone on the ground along `x`.
pub fn trough(b: &mut Builder, c: [f64; 2], len: f64) {
    let (x0, x1) = (c[0] - len / 2.0, c[0] + len / 2.0);
    b.rbox(
        "stone",
        [x0, c[1] - 0.35, 0.0],
        [x1, c[1] + 0.35, 0.12],
        0.0,
    );
    b.rbox(
        "stone",
        [x0, c[1] - 0.35, 0.12],
        [x1, c[1] - 0.25, 0.6],
        0.0,
    );
    b.rbox(
        "stone",
        [x0, c[1] + 0.25, 0.12],
        [x1, c[1] + 0.35, 0.6],
        0.0,
    );
    b.rbox(
        "stone",
        [x0, c[1] - 0.25, 0.12],
        [x0 + 0.1, c[1] + 0.25, 0.6],
        0.0,
    );
    b.rbox(
        "stone",
        [x1 - 0.1, c[1] - 0.25, 0.12],
        [x1, c[1] + 0.25, 0.6],
        0.0,
    );
    b.rbox(
        "water",
        [x0 + 0.1, c[1] - 0.25, 0.12],
        [x1 - 0.1, c[1] + 0.25, 0.5],
        0.0,
    );
}

/// A flag pole with its flag (`role`) hung from the top.
pub fn flag_pole(b: &mut Builder, base: [f64; 2], h: f64, role: &'static str) {
    b.cyl(
        "metal",
        [base[0], base[1], 0.0],
        [base[0], base[1], h],
        0.04,
    );
    b.ellipsoid("metal", [base[0], base[1], h + 0.04], [0.07, 0.07, 0.07]);
    b.rbox(
        role,
        [base[0] + 0.03, base[1] - 0.008, h - 1.05],
        [base[0] + 1.55, base[1] + 0.008, h - 0.05],
        0.0,
    );
}

/// A bell hung from a beam at `top` (its crown there), `size` across.
pub fn bell(b: &mut Builder, c: [f64; 2], top: f64, size: f64) {
    let s = size / 0.5;
    b.lathe(
        "bell",
        [c[0], c[1], top - 0.7 * s],
        &[
            (0.0, 0.0),
            (0.25 * s, 0.02 * s),
            (0.2 * s, 0.2 * s),
            (0.12 * s, 0.62 * s),
            (0.03 * s, 0.7 * s),
            (0.0, 0.7 * s),
        ],
        1.0,
        1.0,
    );
}

/// A balcony of wood running along a face (balcón corrido): its floor of
/// boards on beams, turned balusters, the handrail, and posts to the eaves
/// if `posts_to` is given.
#[allow(clippy::too_many_arguments)]
pub fn wooden_balcony(
    b: &mut Builder,
    f: &Face,
    s: (f64, f64),
    floor: f64,
    out: f64,
    role: &'static str,
    posts_to: Option<f64>,
) {
    let (s0, s1) = s;
    f.bx(b, role, (s0, s1), (floor - 0.08, floor), (-0.01, out));
    let n = ((s1 - s0) / 1.2).ceil().max(1.0) as usize;
    for i in 0..=n {
        let x = (s0 + (s1 - s0) * i as f64 / n as f64).clamp(s0 + 0.06, s1 - 0.06);
        f.bx(
            b,
            role,
            (x - 0.06, x + 0.06),
            (floor - 0.26, floor - 0.08),
            (-0.01, out + 0.05),
        );
    }
    let d = out - 0.06;
    let mut x = s0 + 0.08;
    while x < s1 - 0.05 {
        let c = f.at(x, 0.0, d);
        b.lathe(
            role,
            [c[0], c[1], floor],
            &[
                (0.035, 0.0),
                (0.03, 0.1),
                (0.05, 0.3),
                (0.025, 0.65),
                (0.035, 0.85),
                (0.0, 0.86),
            ],
            1.0,
            1.0,
        );
        x += 0.14;
    }
    f.bx(
        b,
        role,
        (s0, s1),
        (floor + 0.86, floor + 0.95),
        (d - 0.05, d + 0.05),
    );
    for side in [s0, s1] {
        let sf = Face {
            o: f.at(side, 0.0, 0.0).map_xy(),
            u: f.out(),
            len: d,
        };
        sf.bx(
            b,
            role,
            (0.0, d + 0.05),
            (floor + 0.86, floor + 0.95),
            (-0.05, 0.05),
        );
    }
    if let Some(top) = posts_to {
        for i in 0..=n {
            let x = (s0 + (s1 - s0) * i as f64 / n as f64).clamp(s0 + 0.07, s1 - 0.07);
            f.bx(
                b,
                role,
                (x - 0.07, x + 0.07),
                (floor, top),
                (d - 0.07, d + 0.07),
            );
        }
    }
}

/// Timber framing showing on a face (entramado): posts every `bay`, a
/// rail at mid height and at the top, braces in every other bay.
pub fn timber_frame(
    b: &mut Builder,
    f: &Face,
    s: (f64, f64),
    z: (f64, f64),
    bay: f64,
    role: &'static str,
) {
    let n = ((s.1 - s.0) / bay).round().max(1.0) as usize;
    let t = 0.09;
    let d = (-0.01, 0.04);
    for i in 0..=n {
        let x = (s.0 + (s.1 - s.0) * i as f64 / n as f64).clamp(s.0 + t, s.1 - t);
        f.bx(b, role, (x - t, x + t), z, d);
    }
    let zm = (z.0 + z.1) / 2.0;
    for zz in [z.0 + t, zm, z.1 - t] {
        f.bx(b, role, s, (zz - t, zz + t), d);
    }
    for i in 0..n {
        if i % 2 == 1 {
            continue;
        }
        let a = s.0 + (s.1 - s.0) * i as f64 / n as f64;
        let e = s.0 + (s.1 - s.0) * (i + 1) as f64 / n as f64;
        let (p, q) = if (i / 2) % 2 == 0 {
            ((a, z.0), (e, zm))
        } else {
            ((a, zm), (e, z.0))
        };
        f.bar(b, role, p, q, 0.015, 0.075, 4);
        f.bar(
            b,
            role,
            (p.0, p.1 + (zm - z.0)),
            (q.0, q.1 + (zm - z.0)),
            0.015,
            0.075,
            4,
        );
    }
}

/// A café set on a pavement: a round table on its foot and chairs round
/// it, and a parasol over it if `parasol`.
pub fn cafe_set(b: &mut Builder, c: [f64; 2], chairs: usize, parasol: bool, seed: u64) {
    b.cone(
        "furniture",
        [c[0], c[1], 0.0],
        [c[0], c[1], 0.03],
        0.25,
        0.25,
        12,
    );
    b.cyl("furniture", [c[0], c[1], 0.03], [c[0], c[1], 0.72], 0.03);
    b.cone(
        "table_top",
        [c[0], c[1], 0.72],
        [c[0], c[1], 0.75],
        0.35,
        0.35,
        16,
    );
    for k in 0..chairs {
        let a = std::f64::consts::TAU * (k as f64 + 0.15 * unit(seed, k as u64)) / chairs as f64;
        let (s, co) = a.sin_cos();
        let p = [c[0] + co * 0.62, c[1] + s * 0.62];
        let start = b.mark();
        for (dx, dy) in [(-0.18, -0.18), (0.18, -0.18), (0.18, 0.18), (-0.18, 0.18)] {
            b.cyl("furniture", [dx, dy, 0.0], [dx, dy, 0.45], 0.012);
        }
        b.rbox("seat", [-0.21, -0.21, 0.45], [0.21, 0.21, 0.48], 0.0);
        b.rbox("seat", [0.17, -0.21, 0.48], [0.21, 0.21, 0.88], 0.0);
        turn_from(b, start, [0.0, 0.0], a);
        b.transform_from(start, |q| [q[0] + p[0], q[1] + p[1], q[2]]);
    }
    if parasol {
        b.cyl("furniture", [c[0], c[1], 0.75], [c[0], c[1], 2.35], 0.025);
        b.cone(
            "awning",
            [c[0], c[1], 2.05],
            [c[0], c[1], 2.45],
            1.25,
            0.04,
            8,
        );
    }
}

/// A curtain wall on a face from `s0` to `s1` and `z0` to `z1`: mullions
/// every `bay`, transoms at every `floor`, spandrel bands of opaque glass,
/// the glass between.
#[allow(clippy::too_many_arguments)]
pub fn curtain_wall(
    b: &mut Builder,
    f: &Face,
    s: (f64, f64),
    z: (f64, f64),
    bay: f64,
    floor: f64,
    out: f64,
) {
    let n = ((s.1 - s.0) / bay).round().max(1.0) as usize;
    let d = (out - 0.14, out);
    for i in 0..=n {
        let x = s.0 + (s.1 - s.0) * i as f64 / n as f64;
        f.bx(b, "mullion", (x - 0.035, x + 0.035), z, d);
    }
    let m = ((z.1 - z.0) / floor).round().max(1.0) as usize;
    for k in 0..=m {
        let zz = z.0 + (z.1 - z.0) * k as f64 / m as f64;
        f.bx(b, "mullion", s, (zz - 0.04, zz + 0.04), d);
        if k < m {
            f.bx(b, "spandrel", s, (zz, zz + 0.9), (out - 0.1, out - 0.08));
        }
    }
    f.bx(b, "curtain_glass", s, z, (out - 0.1, out - 0.09));
}
