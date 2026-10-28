//! Courts and grandstands (paquete 8, tanda 8c): one good model each, its
//! colours by its materials.
//!
//! Padel court (20 × 10 m inside its glass and mesh), tennis court
//! (36,57 × 18,29 m with its run-off), basketball court (32 × 19 m with its
//! hoops) and a grandstand of concrete steps with seats.
//!
//! Each is centred on `x = 0` (the length of a court along `x`) with its
//! near edge at `y = 0`. Roles: `ground` (the run-off), `court` (the
//! playing surface), `line`, `key` (the painted keys), `net`, `frame`
//! (posts and steel), `glass`, `dark`, `light` (floodlights, glowing at
//! night); the basketball hoops' `board` and `accent` (rims and pads); the
//! grandstand's `mass`, `seat` and `metal`.

use super::sport::{basketball_hoop, cord, net};
use crate::build::Builder;
use crate::Params;
use std::f64::consts::{FRAC_PI_2, PI};

/// A painted line `w` wide along a path in plan, a hair over `z`.
fn line_path(b: &mut Builder, role: &'static str, path: &[[f64; 2]], w: f64, z: f64) {
    for s in path.windows(2) {
        let (a, c) = (s[0], s[1]);
        let (dx, dy) = (c[0] - a[0], c[1] - a[1]);
        let l = (dx * dx + dy * dy).sqrt();
        if l < 1e-6 {
            continue;
        }
        let (ux, uy) = (dx / l, dy / l);
        // Each piece a little longer, so the bends close.
        let half = l / 2.0 + w * 0.2;
        b.obox(
            role,
            [(a[0] + c[0]) / 2.0, (a[1] + c[1]) / 2.0, z + 0.0004],
            [ux * half, uy * half, 0.0],
            [-uy * w / 2.0, ux * w / 2.0, 0.0],
            [0.0, 0.0, 0.0004],
        );
    }
}

/// A straight line from `a` to `c`.
fn line(b: &mut Builder, a: [f64; 2], c: [f64; 2], w: f64, z: f64) {
    line_path(b, "line", &[a, c], w, z);
}

/// The points of an arc round `c`, radius `r`, from `a0` to `a1`.
fn arc(c: [f64; 2], r: f64, a0: f64, a1: f64, n: usize) -> Vec<[f64; 2]> {
    (0..=n)
        .map(|k| {
            let a = a0 + (a1 - a0) * k as f64 / n as f64;
            [c[0] + r * a.cos(), c[1] + r * a.sin()]
        })
        .collect()
}

/// A floodlight mast at `(x, y)`, `h` tall, its head facing `toward`.
fn mast(b: &mut Builder, x: f64, y: f64, h: f64, toward: [f64; 2]) {
    b.rbox(
        "dark",
        [x - 0.15, y - 0.15, 0.0],
        [x + 0.15, y + 0.15, 0.05],
        0.0,
    );
    super::hospitality::rod(b, "frame", [x, y, 0.05], [x, y, h], 0.06);
    let (dx, dy) = (toward[0] - x, toward[1] - y);
    let l = (dx * dx + dy * dy).sqrt().max(1e-6);
    let (ux, uy) = (dx / l, dy / l);
    let c = [x + ux * 0.15, y + uy * 0.15, h];
    // A box leaning down toward the court, its lit face under it.
    let tilt: f64 = 0.5;
    let (ct, st) = (tilt.cos(), tilt.sin());
    let ax = [-uy * 0.3, ux * 0.3, 0.0];
    let ay = [ux * 0.2 * ct, uy * 0.2 * ct, -0.2 * st];
    let az = [ux * 0.06 * st, uy * 0.06 * st, 0.06 * ct];
    b.obox("frame", c, ax, ay, az);
    b.obox(
        "light",
        [
            c[0] - az[0] * 1.05,
            c[1] - az[1] * 1.05,
            c[2] - az[2] * 1.05,
        ],
        [ax[0] * 0.9, ax[1] * 0.9, 0.0],
        [ay[0] * 0.9, ay[1] * 0.9, ay[2] * 0.9],
        [az[0] * 0.05, az[1] * 0.05, az[2] * 0.05],
    );
}

/// A padel court: 20 × 10 m of turf inside glass walls and mesh, its net,
/// the doors by the net and four floodlights.
pub fn padel_court(b: &mut Builder, _p: &Params) {
    let (hl, w) = (10.0, 10.0);
    let z = 0.015;
    b.rbox(
        "ground",
        [-hl - 0.5, -0.5, 0.0],
        [hl + 0.5, w + 0.5, 0.01],
        0.0,
    );
    b.rbox("court", [-hl, 0.0, 0.0], [hl, w, z], 0.0);
    // The service lines and the centre line.
    for s in [-1.0, 1.0] {
        line(b, [s * 6.95, 0.0], [s * 6.95, w], 0.05, z);
    }
    line(b, [-6.95, w / 2.0], [6.95, w / 2.0], 0.05, z);
    // The net and its posts.
    b.rbox("net", [-0.003, 0.05, 0.03], [0.003, w - 0.05, 0.84], 0.0);
    b.rbox("line", [-0.006, 0.05, 0.84], [0.006, w - 0.05, 0.9], 0.0);
    for y in [0.04, w - 0.04] {
        b.rbox(
            "dark",
            [-0.04, y - 0.04, 0.0],
            [0.04, y + 0.04, 0.92],
            0.006,
        );
    }
    let gt = 0.006;
    let post = |b: &mut Builder, x: f64, y: f64, h: f64| {
        b.rbox(
            "frame",
            [x - 0.04, y - 0.04, 0.0],
            [x + 0.04, y + 0.04, h],
            0.004,
        );
    };
    // The ends: glass to 3 m, mesh above to 4 m.
    for s in [-1.0, 1.0] {
        let x = s * (hl + 0.04);
        b.rbox("glass", [x - gt, 0.0, z], [x + gt, w, 3.0], 0.0);
        let xm = x + s * 0.02;
        net(
            b,
            "net",
            [[xm, 0.0, 3.0], [xm, w, 3.0], [xm, w, 4.0], [xm, 0.0, 4.0]],
            0.25,
        );
        for k in 0..=5 {
            post(b, x + s * 0.05, w * k as f64 / 5.0, 4.0);
        }
        cord(b, "frame", [xm, 0.0, 4.0], [xm, w, 4.0], 0.02);
    }
    // The sides: 2 m of glass to 3 m, 2 m stepping down to 2 m, then mesh
    // to 3 m, open where the doors are.
    for (y, out) in [(-0.04, -1.0), (w + 0.04, 1.0)] {
        let ym = y + out * 0.02;
        for s in [-1.0, 1.0] {
            let (a, c) = (s * 8.0, s * hl);
            b.rbox("glass", [a.min(c), y - gt, z], [a.max(c), y + gt, 3.0], 0.0);
            let (a2, c2) = (s * 6.0, s * 8.0);
            b.rbox(
                "glass",
                [a2.min(c2), y - gt, z],
                [a2.max(c2), y + gt, 2.0],
                0.0,
            );
            net(
                b,
                "net",
                [[a2, ym, 2.0], [c2, ym, 2.0], [c2, ym, 3.0], [a2, ym, 3.0]],
                0.25,
            );
            // Mesh panels between the glass and the door, and inside it.
            for (p0, p1) in [(1.6, 6.0), (0.0, 0.6)] {
                let (u, v) = (s * p0, s * p1);
                net(
                    b,
                    "net",
                    [[u, ym, z], [v, ym, z], [v, ym, 3.0], [u, ym, 3.0]],
                    0.25,
                );
            }
            for x in [0.6, 1.6, 2.0, 4.0, 6.0, 8.0] {
                post(b, s * x, y + out * 0.05, 3.0);
            }
            cord(b, "frame", [0.0, ym, 3.0], [s * hl, ym, 3.0], 0.02);
        }
        post(b, 0.0, y + out * 0.05, 3.0);
    }
    // The floodlights, two on each side.
    for y in [-0.45, w + 0.45] {
        for x in [-5.0, 5.0] {
            mast(b, x, y, 6.0, [x, w / 2.0]);
        }
    }
}

/// A tennis court: the doubles court (23,77 × 10,97 m) with its lines and
/// net on its run-off (36,57 × 18,29 m), and the umpire's chair.
pub fn tennis_court(b: &mut Builder, _p: &Params) {
    let (gl, gw) = (36.57 / 2.0, 18.29);
    let yc = gw / 2.0;
    let (hl, hd, hs) = (23.77 / 2.0, 10.97 / 2.0, 8.23 / 2.0);
    let z = 0.015;
    b.rbox("ground", [-gl, 0.0, 0.0], [gl, gw, 0.01], 0.0);
    b.rbox(
        "court",
        [-hl - 0.1, yc - hd - 0.1, 0.0],
        [hl + 0.1, yc + hd + 0.1, z],
        0.0,
    );
    // Its lines: sidelines, baselines, service lines and the centre.
    for s in [-1.0, 1.0] {
        line(b, [-hl, yc + s * hd], [hl, yc + s * hd], 0.05, z);
        line(b, [-hl, yc + s * hs], [hl, yc + s * hs], 0.05, z);
        line(b, [s * hl, yc - hd], [s * hl, yc + hd], 0.1, z);
        line(b, [s * 6.4, yc - hs], [s * 6.4, yc + hs], 0.05, z);
        line(b, [s * hl, yc], [s * (hl - 0.1), yc], 0.05, z);
    }
    line(b, [-6.4, yc], [6.4, yc], 0.05, z);
    // The net, sagging from 1,07 m at its posts to 0,914 m in the middle.
    let yp = hd + 0.914;
    for s in [-1.0, 1.0] {
        let (y0, y1) = if s < 0.0 { (-yp, 0.0) } else { (0.0, yp) };
        let st = b.mark();
        // The mesh: down from its band to the ground.
        b.slab(
            "net",
            &if s < 0.0 {
                vec![[y0, 0.02], [y1, 0.02], [y1, 0.87], [y0, 1.03]]
            } else {
                vec![[y0, 0.02], [y1, 0.02], [y1, 1.03], [y0, 0.87]]
            },
            -0.003,
            0.003,
        );
        b.transform_from(st, |q| [q[2], yc + q[0], q[1]]);
        let st = b.mark();
        b.slab(
            "line",
            &if s < 0.0 {
                vec![[y0, 1.01], [y1, 0.855], [y1, 0.914], [y0, 1.07]]
            } else {
                vec![[y0, 0.855], [y1, 1.01], [y1, 1.07], [y0, 0.914]]
            },
            -0.006,
            0.006,
        );
        b.transform_from(st, |q| [q[2], yc + q[0], q[1]]);
        super::hospitality::rod(
            b,
            "dark",
            [0.0, yc + s * yp, 0.0],
            [0.0, yc + s * yp, 1.09],
            0.04,
        );
    }
    // The strap in the middle.
    b.rbox(
        "line",
        [-0.008, yc - 0.025, 0.02],
        [0.008, yc + 0.025, 0.914],
        0.0,
    );
    // The umpire's chair, beside a post.
    let (cx, cy) = (0.0, yc - yp - 1.0);
    for (dx, dy) in [(-0.35, -0.35), (0.35, -0.35), (-0.35, 0.35), (0.35, 0.35)] {
        super::hospitality::rod(
            b,
            "frame",
            [cx + dx, cy + dy, 0.02],
            [cx + dx * 0.6, cy + dy * 0.6, 1.8],
            0.025,
        );
    }
    for k in 1..6 {
        let zk = 0.3 * k as f64;
        let k6 = 1.0 - 0.4 * zk / 1.8;
        super::hospitality::rod(
            b,
            "frame",
            [cx - 0.35 * k6, cy - 0.35 * k6, zk],
            [cx + 0.35 * k6, cy - 0.35 * k6, zk],
            0.015,
        );
    }
    b.rbox(
        "frame",
        [cx - 0.3, cy - 0.3, 1.8],
        [cx + 0.3, cy + 0.25, 1.85],
        0.01,
    );
    b.rbox(
        "frame",
        [cx - 0.3, cy - 0.3, 1.85],
        [cx + 0.3, cy - 0.25, 2.3],
        0.01,
    );
}

/// A basketball court: 28 × 15 m with its lines and painted keys on its
/// run-off (32 × 19 m), a hoop at each end.
pub fn basketball_court(b: &mut Builder, _p: &Params) {
    let (gl, gw) = (16.0, 19.0);
    let yc = gw / 2.0;
    let (hl, hw) = (14.0, 7.5);
    let z = 0.015;
    b.rbox("ground", [-gl, 0.0, 0.0], [gl, gw, 0.01], 0.0);
    b.rbox(
        "court",
        [-hl - 0.05, yc - hw - 0.05, 0.0],
        [hl + 0.05, yc + hw + 0.05, z],
        0.0,
    );
    let w = 0.05;
    // The boundary, the centre line and circle.
    line_path(
        b,
        "line",
        &[
            [-hl, yc - hw],
            [hl, yc - hw],
            [hl, yc + hw],
            [-hl, yc + hw],
            [-hl, yc - hw],
        ],
        w,
        z,
    );
    line(b, [0.0, yc - hw], [0.0, yc + hw], w, z);
    line_path(b, "line", &arc([0.0, yc], 1.8, 0.0, 2.0 * PI, 48), w, z);
    for s in [-1.0, 1.0] {
        let base = s * hl;
        let rim = s * (hl - 1.575);
        let ft = s * (hl - 5.8);
        // The key, painted, and its lines.
        let (k0, k1) = (base.min(ft), base.max(ft));
        b.rbox("key", [k0, yc - 2.45, z], [k1, yc + 2.45, z + 0.0003], 0.0);
        line_path(
            b,
            "line",
            &[
                [base, yc - 2.45],
                [ft, yc - 2.45],
                [ft, yc + 2.45],
                [base, yc + 2.45],
            ],
            w,
            z + 0.0003,
        );
        // The free-throw circle, toward the middle.
        let (a0, a1) = if s > 0.0 {
            (FRAC_PI_2, 3.0 * FRAC_PI_2)
        } else {
            (-FRAC_PI_2, FRAC_PI_2)
        };
        line_path(b, "line", &arc([ft, yc], 1.8, a0, a1, 24), w, z + 0.0003);
        // The three-point line: straight from the baseline, then the arc.
        let a: f64 = (6.6_f64 / 6.75).asin();
        let (b0, b1) = if s > 0.0 { (PI + a, PI - a) } else { (-a, a) };
        let mut path = vec![[base, yc - 6.6]];
        path.extend(arc([rim, yc], 6.75, b0, b1, 40));
        path.push([base, yc + 6.6]);
        line_path(b, "line", &path, w, z);
        // The hoop on its stand, behind the baseline.
        let st = b.mark();
        let mut hp = Params::default();
        hp.set("rim_height", 3.05);
        hp.set("reach", 2.3);
        basketball_hoop(b, &hp);
        // In the hoop's own plan the rim is 2,7 m in front of its back.
        let x0 = rim + s * 2.7;
        b.transform_from(st, |q| [x0 - s * q[1], yc + s * q[0], q[2]]);
    }
}

/// A grandstand of concrete steps with a seat every half metre and a rail
/// at its back: `length` and `rows`; the front row at `y = rows × 0,8`,
/// facing `+y`.
pub fn grandstand(b: &mut Builder, p: &Params) {
    let l = p.get("length", 6.0).clamp(2.0, 30.0);
    let n = p.get("rows", 4.0).round().clamp(1.0, 10.0) as usize;
    let (tread, rise) = (0.8, 0.42);
    let d = tread * n as f64;
    let x = l / 2.0;
    for k in 0..n {
        let (y0, y1) = (d - tread * (k + 1) as f64, d - tread * k as f64);
        let top = rise * (k + 1) as f64;
        b.rbox("mass", [-x, y0, 0.0], [x, y1, top], 0.0);
        // The seats along the step, facing the front.
        let m = ((l - 0.3) / 0.5).floor().max(1.0) as usize;
        let gap = (l - 0.3) / m as f64;
        for i in 0..m {
            let sx = -x + 0.15 + gap * (i as f64 + 0.5);
            let sy = y0 + 0.45;
            b.rbox(
                "seat",
                [sx - 0.21, sy - 0.2, top],
                [sx + 0.21, sy + 0.2, top + 0.06],
                0.0,
            );
            let st = b.mark();
            b.rbox("seat", [-0.21, -0.02, 0.0], [0.21, 0.02, 0.36], 0.0);
            let tilt: f64 = 0.2;
            b.transform_from(st, |q| {
                [
                    sx + q[0],
                    sy - 0.2 + q[1] * tilt.cos() - q[2] * tilt.sin(),
                    top + 0.05 + q[1] * tilt.sin() + q[2] * tilt.cos(),
                ]
            });
        }
    }
    // The rail along the back, on posts.
    let top = rise * n as f64;
    let posts = (l / 2.0).ceil().max(1.0) as usize;
    for i in 0..=posts {
        let px = -x + 0.05 + (l - 0.1) * i as f64 / posts as f64;
        super::hospitality::rod(b, "metal", [px, 0.05, top], [px, 0.05, top + 1.1], 0.025);
    }
    super::hospitality::rod(
        b,
        "metal",
        [-x + 0.05, 0.05, top + 1.1],
        [x - 0.05, 0.05, top + 1.1],
        0.025,
    );
    super::hospitality::rod(
        b,
        "metal",
        [-x + 0.05, 0.05, top + 0.55],
        [x - 0.05, 0.05, top + 0.55],
        0.015,
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fine::shape::signed_volume;

    type Make = fn(&mut Builder, &Params);

    #[test]
    fn the_courts_are_made() {
        let pieces: [(&str, Make); 4] = [
            ("pádel", padel_court),
            ("tenis", tennis_court),
            ("baloncesto", basketball_court),
            ("grada", grandstand),
        ];
        for (name, make) in pieces {
            for (key, v) in [("", 0.0), ("length", 30.0), ("rows", 10.0)] {
                let mut p = Params::default();
                if !key.is_empty() {
                    p.set(key, v);
                }
                let mut b = Builder::default();
                make(&mut b, &p);
                let tris = b.triangle_count();
                assert!((100..40_000).contains(&tris), "{name}: {tris}");
                for (role, t) in &b.parts {
                    let v = signed_volume(&b.positions, t);
                    assert!(v > -1e-7, "{name} {role}: {v}");
                }
                let low = b.positions.iter().map(|q| q[2]).fold(f32::MAX, f32::min);
                assert!(low > -0.001, "{name}: {low}");
            }
        }
    }

    #[test]
    fn the_three_point_line_runs_round_the_rim() {
        let mut b = Builder::default();
        basketball_court(&mut b, &Params::default());
        // Nothing of the court outside its run-off but the hoops' stands.
        let (lo, hi) = b
            .positions
            .iter()
            .fold(([f32::MAX; 2], [f32::MIN; 2]), |(lo, hi), q| {
                (
                    [lo[0].min(q[0]), lo[1].min(q[1])],
                    [hi[0].max(q[0]), hi[1].max(q[1])],
                )
            });
        assert!(lo[0] > -16.01 && hi[0] < 16.01, "{lo:?} {hi:?}");
        assert!(lo[1] > -0.01 && hi[1] < 19.01, "{lo:?} {hi:?}");
    }
}
