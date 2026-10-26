//! The workshop's fireplaces and stoves, one for each collection
//! (paquete 1c). The wall at `y = 0`, the front looking to `+y`, centred
//! on `x = 0`; `width`, `height` (of the chimney breast or the flue),
//! `depth`.
//!
//! Roles: `mat` the surround (plaster, brick, microcement), `stone`
//! stone or marble, `frame` a wooden mantel, `metal` steel, cast iron or
//! brass, `dark` the inside of the firebox, `glass`, `logs`, `flame`
//! (it glows).

use super::shape::{bar, turned};
use crate::build::{Builder, P3};
use crate::Params;
use std::f64::consts::TAU;

/// A solid between a rectangle at `z0` and another at `z1` (each `[x0,
/// y0, x1, y1]`), as a hood narrowing upwards.
fn frustum(b: &mut Builder, role: &'static str, lo: [f64; 4], hi: [f64; 4], z0: f64, z1: f64) {
    let ring = |b: &mut Builder, r: [f64; 4], z: f64| -> [u32; 4] {
        [
            b.point([r[0], r[1], z]),
            b.point([r[2], r[1], z]),
            b.point([r[2], r[3], z]),
            b.point([r[0], r[3], z]),
        ]
    };
    let a = ring(b, lo, z0);
    let c = ring(b, hi, z1);
    b.quad(role, a[3], a[2], a[1], a[0]);
    b.quad(role, c[0], c[1], c[2], c[3]);
    for i in 0..4 {
        let j = (i + 1) % 4;
        b.quad(role, a[i], a[j], c[j], c[i]);
    }
}

/// A block from `(x0, y0, z0)` to `(x1, y1, z1)` with rectangular holes
/// through it (`[xa, xb, za, zb]` each), built in upright strips so that
/// what is behind a hole shows.
#[allow(clippy::too_many_arguments)]
fn holed(
    b: &mut Builder,
    role: &'static str,
    x0: f64,
    x1: f64,
    y0: f64,
    y1: f64,
    z0: f64,
    z1: f64,
    holes: &[[f64; 4]],
) {
    let mut xs = vec![x0, x1];
    for h in holes {
        xs.push(h[0].clamp(x0, x1));
        xs.push(h[1].clamp(x0, x1));
    }
    xs.sort_by(|a, c| a.total_cmp(c));
    xs.dedup_by(|a, c| (*a - *c).abs() < 1e-6);
    for k in 0..xs.len() - 1 {
        let (xa, xb) = (xs[k], xs[k + 1]);
        let xm = (xa + xb) / 2.0;
        let mut cuts: Vec<[f64; 2]> = holes
            .iter()
            .filter(|h| h[0] < xm && xm < h[1])
            .map(|h| [h[2].max(z0), h[3].min(z1)])
            .collect();
        cuts.sort_by(|a, c| a[0].total_cmp(&c[0]));
        let mut z = z0;
        for c in cuts.iter().chain(std::iter::once(&[z1, z1])) {
            if c[0] > z + 1e-6 {
                b.rbox(role, [xa, y0, z], [xb, y1, c[0]], 0.0);
            }
            z = z.max(c[1]);
        }
    }
}

/// Tongues of fire along `x0`…`x1` rising from `z` at depth `y`.
fn flames(b: &mut Builder, x0: f64, x1: f64, y: f64, z: f64, tall: f64) {
    let n = (((x1 - x0) / 0.06).round() as usize).max(2);
    for k in 0..n {
        let u = (k as f64 + 0.5) / n as f64;
        let x = x0 + (x1 - x0) * u;
        // Taller in the middle, never two alike.
        let h = tall
            * (0.55 + 0.45 * (std::f64::consts::PI * u).sin())
            * (0.8 + 0.25 * ((k * 37 % 11) as f64 / 11.0));
        let r = (x1 - x0) / n as f64 * 0.62;
        let start = b.mark();
        turned(
            b,
            "flame",
            [0.0; 3],
            &[
                (0.0, 0.0),
                (r * 0.7, 0.0),
                (r, h * 0.18),
                (r * 0.75, h * 0.45),
                (r * 0.35, h * 0.75),
                (0.0, h),
            ],
            10,
            0.0,
        );
        let lean = 0.12 * ((k * 53 % 7) as f64 / 7.0 - 0.5);
        b.transform_from(start, |q| {
            [x + q[0] + lean * q[2], y + q[1] * 0.6, z + q[2]]
        });
    }
}

/// Logs laid across at `(cx, y)` on `z`, `w` across.
fn logs(b: &mut Builder, cx: f64, y: f64, z: f64, w: f64) {
    let r = (w * 0.07).clamp(0.035, 0.055);
    b.cyl(
        "logs",
        [cx - w * 0.45, y - 0.04, z + r],
        [cx + w * 0.4, y + 0.02, z + r],
        r,
    );
    b.cyl(
        "logs",
        [cx - w * 0.38, y + 0.06, z + r],
        [cx + w * 0.45, y + 0.02, z + r * 0.95],
        r * 0.9,
    );
    b.cyl(
        "logs",
        [cx - w * 0.3, y + 0.02, z + r * 2.6],
        [cx + w * 0.28, y - 0.01, z + r * 2.4],
        r * 0.85,
    );
}

/// A dark firebox opening from `x0` to `x1`, `z0` to `z1`, `deep` into
/// the breast whose face is at `y`: the back and sides inside, the fire.
#[allow(clippy::too_many_arguments)]
fn firebox(b: &mut Builder, x0: f64, x1: f64, z0: f64, z1: f64, y: f64, deep: f64, fire: bool) {
    let back = (y - deep).max(0.02);
    b.rbox("dark", [x0, back - 0.02, z0], [x1, back, z1], 0.0);
    b.rbox("dark", [x0 - 0.02, back, z0], [x0, y, z1], 0.0);
    b.rbox("dark", [x1, back, z0], [x1 + 0.02, y, z1], 0.0);
    b.rbox("dark", [x0, back, z1], [x1, y, z1 + 0.02], 0.0);
    b.rbox("dark", [x0, back, z0 - 0.02], [x1, y, z0], 0.0);
    if fire {
        let w = x1 - x0;
        let cy = (back + y) / 2.0;
        logs(b, (x0 + x1) / 2.0, cy, z0, w * 0.75);
        flames(
            b,
            x0 + w * 0.2,
            x1 - w * 0.2,
            cy,
            z0 + 0.06,
            (z1 - z0) * 0.55,
        );
    }
}

/// A fireplace of a collection.
pub fn fireplace(b: &mut Builder, p: &Params) {
    let w = p.get("width", 1.4).clamp(0.5, 4.0);
    let h = p.get("height", 1.2).clamp(0.6, 3.5);
    let d = p.get("depth", 0.4).clamp(0.2, 1.0);
    let fire = p.flag("fire", true);
    let (x0, x1) = (-w / 2.0, w / 2.0);
    match p.style() {
        // Mediterranean: a whitewashed chimney with its hood narrowing up
        // to the ceiling, a ledge where they meet, a tiled hearth.
        2 => {
            let (open_w, open_h) = (w * 0.62, 0.7_f64.min(h * 0.45));
            let (oa, ob) = (-open_w / 2.0, open_w / 2.0);
            let ledge = open_h + 0.25;
            b.rbox(
                "stone",
                [x0 - 0.05, 0.0, 0.0],
                [x1 + 0.05, d + 0.15, 0.06],
                0.006,
            );
            b.rbox("mat", [x0, 0.0, 0.06], [oa, d, ledge], 0.02);
            b.rbox("mat", [ob, 0.0, 0.06], [x1, d, ledge], 0.02);
            b.rbox("mat", [oa, 0.0, open_h + 0.06], [ob, d, ledge], 0.0);
            b.rbox(
                "mat",
                [x0 - 0.04, 0.0, ledge],
                [x1 + 0.04, d + 0.05, ledge + 0.05],
                0.015,
            );
            if h > ledge + 0.1 {
                frustum(
                    b,
                    "mat",
                    [x0 + 0.04, 0.0, x1 - 0.04, d],
                    [x0 * 0.45, 0.0, x1 * 0.45, d * 0.6],
                    ledge + 0.05,
                    h,
                );
            }
            firebox(b, oa, ob, 0.06, open_h + 0.06, d, d - 0.06, fire);
        }
        // Fifties: a brick surround, a long hearth bench of stone and a
        // slim wooden mantel shelf, a brass screen.
        3 => {
            let bench = 0.35;
            b.rbox(
                "stone",
                [x0 - w * 0.25, 0.0, 0.32],
                [x1 + 0.05, d + 0.12, bench],
                0.004,
            );
            b.rbox(
                "mat",
                [x0 - w * 0.25 + 0.02, 0.0, 0.0],
                [x1, d + 0.08, 0.32],
                0.004,
            );
            let (oa, ob, oz) = (x0 + w * 0.25, x1 - w * 0.25, bench + 0.5_f64.min(h * 0.5));
            b.rbox("mat", [x0, 0.0, bench], [oa, d, h], 0.004);
            b.rbox("mat", [ob, 0.0, bench], [x1, d, h], 0.004);
            b.rbox("mat", [oa, 0.0, oz], [ob, d, h], 0.0);
            // The brick courses, a dark joint every few centimetres.
            let mut z = 0.07;
            while z < h - 0.02 {
                b.rbox(
                    "dark",
                    [x0 - 0.001, d, z],
                    [x1 + 0.001, d + 0.0015, z + 0.006],
                    0.0,
                );
                z += 0.072;
            }
            b.rbox(
                "frame",
                [x0 - 0.1, 0.0, oz + 0.12],
                [x1 + 0.1, d + 0.1, oz + 0.16],
                0.006,
            );
            firebox(b, oa, ob, bench, oz, d, d - 0.05, fire);
            for k in 0..((ob - oa) / 0.03) as usize {
                let x = oa + 0.015 + 0.03 * k as f64;
                b.cyl(
                    "metal",
                    [x, d + 0.03, bench],
                    [x, d + 0.03, oz - 0.02],
                    0.0012,
                );
            }
            b.cyl(
                "metal",
                [oa, d + 0.03, oz - 0.02],
                [ob, d + 0.03, oz - 0.02],
                0.006,
            );
            b.cyl(
                "metal",
                [oa, d + 0.03, bench + 0.01],
                [ob, d + 0.03, bench + 0.01],
                0.006,
            );
        }
        // Japandi: a block of microcement with a long low fire on pebbles
        // and a niche of stacked logs.
        4 => {
            let (oa, ob) = (x0 + 0.08, x0 + w * 0.68);
            let (za, zb) = (0.32, 0.6_f64.min(h - 0.2));
            let (na, nb) = (x0 + w * 0.74, x1 - 0.08);
            let (nz0, nz1) = (0.08, h - 0.12);
            holed(
                b,
                "mat",
                x0,
                x1,
                0.0,
                d,
                0.0,
                h,
                &[[oa, ob, za, zb], [na, nb, nz0, nz1]],
            );
            firebox(b, oa, ob, za, zb, d, d - 0.08, false);
            if fire {
                for k in 0..(((ob - oa) / 0.05) as usize) {
                    let x = oa + 0.03 + 0.05 * k as f64;
                    b.ellipsoid("stone", [x, d * 0.55, za + 0.012], [0.024, 0.02, 0.014]);
                    b.ellipsoid(
                        "stone",
                        [x + 0.024, d * 0.38, za + 0.01],
                        [0.02, 0.018, 0.012],
                    );
                }
                flames(
                    b,
                    oa + 0.05,
                    ob - 0.05,
                    d * 0.47,
                    za + 0.015,
                    (zb - za) * 0.6,
                );
            }

            b.rbox("dark", [na, 0.04, nz0], [nb, 0.06, nz1], 0.0);
            b.rbox("mat", [na, 0.08, nz0 - 0.001], [nb, d + 0.001, nz0], 0.0);
            let r = 0.04;
            let cols = (((nb - na) / (2.0 * r)).floor() as usize).max(1);
            let rows = (((nz1 - nz0) / (2.0 * r)).floor() as usize).min(12);
            for j in 0..rows {
                for i in 0..cols {
                    let off = if j % 2 == 0 { 0.0 } else { r * 0.5 };
                    let x = na + r + off + 2.0 * r * i as f64;
                    if x + r > nb {
                        continue;
                    }
                    let z = nz0 + r + 2.0 * r * j as f64 * 0.88;
                    b.cyl("logs", [x, 0.07, z], [x, d - 0.02, z], r * 0.95);
                }
            }
        }
        // Contemporary: a wide gas fire behind glass in a white wall unit,
        // a slim black frame.
        5 => {
            let (oa, ob) = (x0 + 0.12, x1 - 0.12);
            let (za, zb) = (0.4, (0.4_f64 + 0.42).min(h - 0.15));
            holed(b, "mat", x0, x1, 0.0, d, 0.0, h, &[[oa, ob, za, zb]]);
            firebox(b, oa, ob, za, zb, d, d - 0.1, false);
            for (lo, hi) in [
                ([oa - 0.02, d, za - 0.02], [ob + 0.02, d + 0.006, za]),
                ([oa - 0.02, d, zb], [ob + 0.02, d + 0.006, zb + 0.02]),
                ([oa - 0.02, d, za], [oa, d + 0.006, zb]),
                ([ob, d, za], [ob + 0.02, d + 0.006, zb]),
            ] {
                b.rbox("metal", lo, hi, 0.0);
            }
            b.rbox("glass", [oa, d - 0.004, za], [ob, d - 0.001, zb], 0.0);
            if fire {
                for k in 0..(((ob - oa) / 0.04) as usize) {
                    let x = oa + 0.025 + 0.04 * k as f64;
                    b.ellipsoid("dark", [x, d * 0.5, za + 0.01], [0.022, 0.03, 0.012]);
                }
                flames(
                    b,
                    oa + 0.06,
                    ob - 0.06,
                    d * 0.5,
                    za + 0.012,
                    (zb - za) * 0.55,
                );
            }
        }
        // Rustic: rough stone jambs, a heavy wooden beam for a mantel, iron
        // firedogs and a stone hearth.
        6 => {
            b.rbox(
                "stone",
                [x0 - 0.1, 0.0, 0.0],
                [x1 + 0.1, d + 0.3, 0.08],
                0.01,
            );
            let jamb = w * 0.18;
            let beam_z = 0.95_f64.min(h - 0.2);
            for (xa, xb) in [(x0, x0 + jamb), (x1 - jamb, x1)] {
                // Blocks of stone, one over another, each a little
                // different.
                let mut z = 0.08;
                let mut k = 0;
                while z < beam_z - 0.02 {
                    let bh = [0.2, 0.16, 0.22, 0.18][k % 4];
                    let zt = (z + bh).min(beam_z);
                    let o = [0.0, 0.012, 0.005, 0.016][k % 4];
                    b.rbox(
                        "stone",
                        [xa - o, 0.0, z + 0.004],
                        [xb + o * 0.5, d + o, zt - 0.004],
                        0.02,
                    );
                    z = zt;
                    k += 1;
                }
            }
            b.rbox(
                "frame",
                [x0 - 0.15, 0.0, beam_z],
                [x1 + 0.15, d + 0.06, beam_z + 0.2],
                0.012,
            );
            if h > beam_z + 0.25 {
                b.rbox("mat", [x0, 0.0, beam_z + 0.2], [x1, d - 0.05, h], 0.01);
            }
            let (oa, ob) = (x0 + jamb, x1 - jamb);
            firebox(b, oa, ob, 0.08, beam_z, d, d - 0.04, fire);
            for x in [oa + 0.12, ob - 0.12] {
                bar(
                    b,
                    "metal",
                    [x, d * 0.2, 0.08],
                    [x, d * 0.85, 0.08 + 0.02],
                    0.02,
                    [1.0, 0.0, 0.0],
                );
                b.cyl("metal", [x, d * 0.85, 0.08], [x, d * 0.85, 0.32], 0.012);
                b.ellipsoid("metal", [x, d * 0.85, 0.34], [0.022, 0.022, 0.026]);
            }
        }
        // Industrial: a black steel box stove on legs with its flue, on a
        // steel floor plate, logs in a rack beside it.
        7 => {
            let sw = (w * 0.45).clamp(0.45, 0.7);
            let (sa, sb) = (-sw / 2.0 - w * 0.12, sw / 2.0 - w * 0.12);
            let (sy0, sy1) = (d * 0.4, d * 0.4 + 0.42);
            b.rbox("metal", [x0, 0.0, 0.0], [x1, sy1 + 0.25, 0.006], 0.002);
            for (x, y) in [
                (sa + 0.03, sy0 + 0.03),
                (sb - 0.03, sy0 + 0.03),
                (sb - 0.03, sy1 - 0.03),
                (sa + 0.03, sy1 - 0.03),
            ] {
                bar(
                    b,
                    "metal",
                    [x, y, 0.006],
                    [x, y, 0.2],
                    0.03,
                    [1.0, 0.0, 0.0],
                );
            }
            let top = 0.2 + 0.55;

            let (ga, gb, gz0, gz1) = (sa + 0.06, sb - 0.06, 0.28, top - 0.1);
            let t = 0.012;
            b.rbox("metal", [sa, sy0, 0.2], [sb, sy0 + t, top], 0.004);
            b.rbox("metal", [sa, sy0, 0.2], [sa + t, sy1, top], 0.004);
            b.rbox("metal", [sb - t, sy0, 0.2], [sb, sy1, top], 0.004);
            b.rbox("metal", [sa, sy0, 0.2], [sb, sy1, 0.2 + t], 0.004);
            b.rbox("metal", [sa, sy0, top - t], [sb, sy1, top], 0.004);
            holed(
                b,
                "metal",
                sa,
                sb,
                sy1 - t,
                sy1,
                0.2,
                top,
                &[[ga, gb, gz0, gz1]],
            );
            b.rbox(
                "dark",
                [sa + t, sy0 + t, 0.2 + t],
                [sb - t, sy0 + t + 0.004, top - t],
                0.0,
            );
            b.rbox(
                "dark",
                [sa + t, sy0 + t, 0.2 + t],
                [sb - t, sy1 - t, 0.2 + t + 0.004],
                0.0,
            );
            b.rbox(
                "metal",
                [sa - 0.01, sy0 - 0.01, top],
                [sb + 0.01, sy1 + 0.01, top + 0.012],
                0.004,
            );

            if fire {
                logs(b, (ga + gb) / 2.0, sy1 - 0.08, gz0, gb - ga);
                flames(
                    b,
                    ga + 0.05,
                    gb - 0.05,
                    sy1 - 0.06,
                    gz0 + 0.05,
                    (gz1 - gz0) * 0.6,
                );
            }
            b.rbox("glass", [ga, sy1 + 0.002, gz0], [gb, sy1 + 0.006, gz1], 0.0);
            b.cyl(
                "metal",
                [gb + 0.02, sy1 + 0.02, gz0 + 0.05],
                [gb + 0.02, sy1 + 0.02, gz1 - 0.05],
                0.008,
            );
            let fx = (sa + sb) / 2.0;
            let fy = (sy0 + sy1) / 2.0;
            b.cyl("metal", [fx, fy, top], [fx, fy, h.max(top + 0.3)], 0.075);
            b.cyl("metal", [fx, fy, top + 0.1], [fx, fy, top + 0.12], 0.082);
            // The log rack.
            let (ra, rb) = (sb + 0.12, x1 - 0.02);
            if rb - ra > 0.2 {
                for x in [ra, rb] {
                    bar(
                        b,
                        "metal",
                        [x, 0.08, 0.006],
                        [x, 0.08, 0.7],
                        0.02,
                        [1.0, 0.0, 0.0],
                    );
                    bar(
                        b,
                        "metal",
                        [x, 0.32, 0.006],
                        [x, 0.32, 0.7],
                        0.02,
                        [1.0, 0.0, 0.0],
                    );
                }
                b.rbox("metal", [ra, 0.07, 0.04], [rb, 0.33, 0.06], 0.0);
                let r = 0.045;
                let cols = (((rb - ra - 0.02) / (2.0 * r)).floor() as usize).max(1);
                for j in 0..6 {
                    for i in 0..cols {
                        let x = ra
                            + 0.01
                            + r
                            + 2.0 * r * i as f64
                            + if j % 2 == 0 { 0.0 } else { r * 0.4 };
                        if x + r > rb - 0.01 {
                            continue;
                        }
                        let z = 0.06 + r + 1.75 * r * j as f64;
                        b.cyl("logs", [x, 0.09, z], [x, 0.31, z], r * 0.95);
                    }
                }
            }
        }
        // Classic: a Louis XVI marble mantel — fluted pilasters, a frieze
        // with roundels, a cornice shelf; a cast iron insert, a brass
        // fender.
        8 => {
            b.rbox(
                "stone",
                [x0 - 0.06, 0.0, 0.0],
                [x1 + 0.06, d + 0.2, 0.03],
                0.004,
            );
            let (oa, ob) = (x0 + w * 0.2, x1 - w * 0.2);
            let oz = 0.78_f64.min(h * 0.7);
            let shelf = oz + 0.2;

            holed(
                b,
                "mat",
                x0 + 0.06,
                x1 - 0.06,
                0.0,
                d - 0.06,
                0.03,
                shelf,
                &[[oa, ob, 0.03, oz]],
            );
            for (xa, xb) in [(x0, oa), (ob, x1)] {
                b.rbox("stone", [xa, d - 0.06, 0.03], [xb, d, oz], 0.004);
                b.rbox(
                    "stone",
                    [xa - 0.01, d - 0.06, 0.03],
                    [xb + 0.01, d + 0.012, 0.1],
                    0.004,
                );
                let n = 5;
                for k in 0..n {
                    let x = xa + 0.02 + (xb - xa - 0.04) * (k as f64 + 0.5) / n as f64;
                    b.rbox(
                        "dark",
                        [x - 0.004, d - 0.001, 0.12],
                        [x + 0.004, d + 0.0015, oz - 0.04],
                        0.002,
                    );
                }
            }
            b.rbox(
                "stone",
                [x0, d - 0.06, oz],
                [x1, d + 0.004, shelf - 0.03],
                0.004,
            );
            for x in [x0 + (oa - x0) / 2.0, x1 - (x1 - ob) / 2.0, 0.0] {
                b.ellipsoid(
                    "stone",
                    [x, d + 0.004, (oz + shelf - 0.03) / 2.0],
                    [0.04, 0.012, 0.04],
                );
            }
            b.rbox(
                "stone",
                [x0 - 0.05, 0.0, shelf - 0.03],
                [x1 + 0.05, d + 0.06, shelf],
                0.008,
            );
            b.rbox(
                "stone",
                [x0 - 0.03, 0.0, shelf - 0.05],
                [x1 + 0.03, d + 0.035, shelf - 0.03],
                0.006,
            );
            firebox(b, oa, ob, 0.03, oz, d - 0.06, d - 0.16, fire);

            // The cast iron insert: a frame round the opening.
            holed(
                b,
                "metal",
                oa - 0.04,
                ob + 0.04,
                d - 0.075,
                d - 0.055,
                0.03,
                oz + 0.03,
                &[[oa, ob, 0.0, oz]],
            );
            let fy = d + 0.14;
            for (a, c) in [
                ([oa - 0.06, fy, 0.06], [ob + 0.06, fy, 0.06]),
                ([oa - 0.06, d - 0.02, 0.06], [oa - 0.06, fy, 0.06]),
                ([ob + 0.06, d - 0.02, 0.06], [ob + 0.06, fy, 0.06]),
            ] {
                b.cyl("metal", a, c, 0.012);
            }
            for x in [oa - 0.06, ob + 0.06] {
                b.ellipsoid("metal", [x, fy, 0.085], [0.02, 0.02, 0.025]);
            }
        }
        // Nordic: a round black stove on a stone plate, a glass door, its
        // flue up to the ceiling.
        _ => {
            let r = (w * 0.2).clamp(0.2, 0.3);
            let c: P3 = [0.0, d.max(r + 0.08) - 0.0, 0.0];
            b.rbox(
                "stone",
                [-r - 0.2, c[1] - r - 0.12, 0.0],
                [r + 0.2, c[1] + r + 0.3, 0.02],
                0.004,
            );
            let top = 1.0_f64.min(h - 0.2);
            let (z0, z1) = (0.3, top - 0.25);
            // Below and above the door, solid.
            turned(
                b,
                "metal",
                [c[0], c[1], 0.02],
                &[
                    (0.0, 0.0),
                    (r * 0.7, 0.0),
                    (r * 0.75, 0.12),
                    (r, 0.14),
                    (r, z0 - 0.02),
                    (0.0, z0 - 0.02),
                ],
                36,
                0.0,
            );
            turned(
                b,
                "metal",
                [c[0], c[1], z1],
                &[
                    (0.0, 0.0),
                    (r, 0.0),
                    (r, top - z1 - 0.04),
                    (r * 0.95, top - z1),
                    (r * 0.4, top - z1 + 0.02),
                    (0.0, top - z1 + 0.02),
                ],
                36,
                0.0,
            );
            // Between them the wall, open at the front behind a curved
            // pane, black inside.
            let n = 36;
            let (wa, wb) = (TAU / 4.0 - 0.6, TAU / 4.0 + 0.6);
            let at =
                |a: f64, rr: f64, z: f64| -> P3 { [c[0] + rr * a.cos(), c[1] + rr * a.sin(), z] };
            for k in 0..n {
                let (a0, a1) = (TAU * k as f64 / n as f64, TAU * (k + 1) as f64 / n as f64);
                let mid = (a0 + a1) / 2.0;
                let window = mid > wa && mid < wb;
                let (za, zb) = (z0 - 0.02, z1);
                let outer = [at(a0, r, za), at(a1, r, za), at(a1, r, zb), at(a0, r, zb)];
                let inner = [
                    at(a0, r - 0.01, za),
                    at(a1, r - 0.01, za),
                    at(a1, r - 0.01, zb),
                    at(a0, r - 0.01, zb),
                ];
                let o: Vec<u32> = outer.iter().map(|q| b.point(*q)).collect();
                let i: Vec<u32> = inner.iter().map(|q| b.point(*q)).collect();
                if window {
                    let g: Vec<u32> = [
                        at(a0, r - 0.004, za),
                        at(a1, r - 0.004, za),
                        at(a1, r - 0.004, zb),
                        at(a0, r - 0.004, zb),
                    ]
                    .iter()
                    .map(|q| b.point(*q))
                    .collect();
                    b.quad("glass", g[0], g[1], g[2], g[3]);
                } else {
                    b.quad("metal", o[0], o[1], o[2], o[3]);
                    b.quad("dark", i[3], i[2], i[1], i[0]);
                }
            }
            if fire {
                logs(b, c[0], c[1] + r * 0.1, z0 - 0.02, r * 1.2);
                flames(
                    b,
                    c[0] - r * 0.4,
                    c[0] + r * 0.4,
                    c[1] + r * 0.15,
                    z0 + 0.04,
                    (z1 - z0) * 0.6,
                );
            }
            b.cyl(
                "metal",
                [c[0] + r * 0.62, c[1] + r * 0.62, z0 + 0.05],
                [c[0] + r * 0.62, c[1] + r * 0.62, z1 - 0.05],
                0.007,
            );
            b.cyl("metal", [c[0], c[1], 0.02 + top], [c[0], c[1], h], 0.075);
            b.cyl("metal", [c[0], c[1], h - 0.02], [c[0], c[1], h], 0.11);
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn the_collection_fireplaces_are_made() {
        for style in 1..=8 {
            let mut p = crate::Params::default();
            p.set("style", style as f64);
            p.set("height", 2.5);
            let m = crate::make("fine_fireplace", &p).unwrap();
            let n = m.triangle_count();
            println!("fine_fireplace {style}: {n}");
            assert!((100..=60_000).contains(&n), "{style}: {n}");
        }
    }
}
