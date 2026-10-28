//! The playground, calisthenics and the park's exercise machines
//! (paquete 8, tanda 8c): one good model each, its colours by its
//! materials.
//!
//! Swing (a flat seat and a cradle), slide on its tower, seesaw, spring
//! rider, calisthenics station (pull-up bars, horizontal ladder, parallel
//! bars and push-up bars on safety flooring) and a park elliptical.
//!
//! Each is centred on `x = 0` with its back at `y = 0`. Roles: `frame`
//! (posts and beams: wood or painted steel), `accent` (the plastic and the
//! painted details), `seat`, `slide`, `metal` (chains, bars and springs),
//! `dark` (feet and rubber) and `ground` (safety flooring).

use super::hospitality::rod;
use super::shape::{curve, sweep, Cap};
use super::sport::cord;
use crate::build::{Builder, P3};
use crate::Params;
use std::f64::consts::TAU;

/// A swing: a beam on two A-frames, a flat seat and a cradle.
pub fn swing(b: &mut Builder, _p: &Params) {
    let (hx, yc, h) = (1.7, 0.8, 2.4);
    for s in [-1.0, 1.0] {
        let x = s * hx;
        for y in [0.0, 1.6] {
            rod(b, "frame", [x, y, 0.02], [x, yc + (y - yc) * 0.03, h], 0.05);
        }
        // A brace across each frame, its feet anchored.
        b.rbox(
            "frame",
            [x - 0.04, 0.35, 0.75],
            [x + 0.04, 1.25, 0.83],
            0.01,
        );
        for y in [0.0, 1.6] {
            b.rbox(
                "dark",
                [x - 0.1, y - 0.1, 0.0],
                [x + 0.1, y + 0.1, 0.03],
                0.0,
            );
        }
    }
    b.rbox(
        "frame",
        [-hx - 0.15, yc - 0.06, h - 0.02],
        [hx + 0.15, yc + 0.06, h + 0.12],
        0.01,
    );
    // The flat seat, on its chains.
    let sx = -0.8;
    b.rbox(
        "seat",
        [sx - 0.22, yc - 0.09, 0.4],
        [sx + 0.22, yc + 0.09, 0.44],
        0.015,
    );
    for dx in [-0.2, 0.2] {
        cord(
            b,
            "metal",
            [sx + dx, yc, h - 0.02],
            [sx + dx, yc, 0.44],
            0.006,
        );
        b.rbox(
            "metal",
            [sx + dx - 0.02, yc - 0.02, h - 0.04],
            [sx + dx + 0.02, yc + 0.02, h - 0.01],
            0.0,
        );
    }
    // The cradle: a bucket with holes for the legs, on four chains.
    let cx = 0.8;
    let (z0, z1) = (0.4, 0.68);
    b.rbox(
        "accent",
        [cx - 0.22, yc - 0.15, z0],
        [cx + 0.22, yc + 0.15, z0 + 0.04],
        0.02,
    );
    b.rbox(
        "accent",
        [cx - 0.22, yc + 0.11, z0],
        [cx + 0.22, yc + 0.15, z1],
        0.015,
    );
    b.rbox(
        "accent",
        [cx - 0.22, yc - 0.15, z0 + 0.12],
        [cx + 0.22, yc - 0.11, z1 - 0.06],
        0.015,
    );
    for s in [-1.0, 1.0] {
        let x = cx + s * 0.2;
        b.rbox(
            "accent",
            [x - 0.02, yc - 0.15, z0],
            [x + 0.02, yc + 0.15, z1],
            0.015,
        );
    }
    for (dx, dy) in [(-0.2, -0.13), (0.2, -0.13), (-0.2, 0.13), (0.2, 0.13)] {
        cord(
            b,
            "metal",
            [cx + dx, yc, h - 0.02],
            [cx + dx, yc + dy, z1],
            0.006,
        );
    }
}

/// A slide from a tower: the ladder at the back, a platform at 1,2 m
/// under a gabled roof, the chute down to the front.
pub fn slide(b: &mut Builder, _p: &Params) {
    let st = b.mark();
    let (hx, d, zp) = (0.6, 1.2, 1.2);
    // The posts, the platform and the roof.
    for x in [-hx, hx] {
        for y in [0.0, d] {
            b.rbox(
                "frame",
                [x - 0.045, y - 0.045, 0.0],
                [x + 0.045, y + 0.045, 2.55],
                0.008,
            );
        }
    }
    b.rbox("frame", [-hx, 0.0, zp - 0.05], [hx, d, zp], 0.0);
    for s in [-1.0, 1.0] {
        let (cy, cz) = (d / 2.0, 2.72);
        let l = (0.75_f64.powi(2) + 0.3_f64.powi(2)).sqrt();
        b.obox(
            "accent",
            [s * 0.36, cy, cz - 0.15],
            [s * 0.75 / 2.0 * 1.04, 0.0, -0.3 / 2.0 * 1.04],
            [0.0, d / 2.0 + 0.15, 0.0],
            [s * 0.3 / l * 0.015, 0.0, 0.75 / l * 0.015],
        );
    }
    // Plastic panels on the sides.
    for x in [-hx, hx] {
        b.rbox(
            "accent",
            [x - 0.012, 0.05, zp + 0.1],
            [x + 0.012, d - 0.05, zp + 0.75],
            0.01,
        );
    }
    // The ladder at the back.
    let (ly, lz) = (-1.0, 0.02);
    for x in [-0.25, 0.25] {
        rod(b, "frame", [x, ly, lz], [x, 0.0, zp + 0.8], 0.03);
    }
    for k in 1..5 {
        let t = k as f64 / 5.0 * (zp / (zp + 0.8));
        let (y, z) = (ly + (0.0 - ly) * t, (zp + 0.8) * t);
        rod(b, "metal", [-0.25, y, z], [0.25, y, z], 0.016);
    }
    // The chute: down from the platform's front edge to its run-out.
    let path = curve(
        &[
            [0.0, d, zp],
            [0.0, d + 0.6, zp - 0.15],
            [0.0, d + 2.0, 0.55],
            [0.0, d + 2.7, 0.3],
            [0.0, d + 3.1, 0.3],
        ],
        4,
    );
    let w = 0.25;
    for s in path.windows(2) {
        let (a, c) = (s[0], s[1]);
        let (dy, dz) = (c[1] - a[1], c[2] - a[2]);
        let l = (dy * dy + dz * dz).sqrt();
        let (uy, uz) = (dy / l, dz / l);
        let half = l / 2.0 + 0.01;
        let m = [0.0, (a[1] + c[1]) / 2.0, (a[2] + c[2]) / 2.0];
        let along = [0.0, uy * half, uz * half];
        // The bed and its two walls.
        b.obox(
            "slide",
            m,
            [w, 0.0, 0.0],
            along,
            [0.0, -uz * 0.012, uy * 0.012],
        );
        for sx in [-1.0, 1.0] {
            b.obox(
                "slide",
                [sx * (w + 0.015), m[1] - uz * 0.09, m[2] + uy * 0.09],
                [0.015, 0.0, 0.0],
                along,
                [0.0, -uz * 0.1, uy * 0.1],
            );
        }
    }
    // The feet under the run-out.
    for y in [d + 2.4, d + 3.0] {
        for x in [-0.22, 0.22] {
            rod(b, "metal", [x, y, 0.0], [x, y, 0.29], 0.02);
        }
    }
    b.transform_from(st, |q| [q[0], q[1] + 1.05, q[2]]);
}

/// A seesaw on its pivot: a beam with a seat and a handle at each end,
/// rubber bumpers under its ends; one end down.
pub fn seesaw(b: &mut Builder, _p: &Params) {
    let yc = 0.3;
    let zp = 0.55;
    b.rbox("dark", [-0.3, yc - 0.25, 0.0], [0.3, yc + 0.25, 0.04], 0.01);
    b.rbox(
        "frame",
        [-0.12, yc - 0.12, 0.04],
        [0.12, yc + 0.12, zp - 0.05],
        0.02,
    );
    rod(
        b,
        "metal",
        [0.0, yc - 0.15, zp - 0.05],
        [0.0, yc + 0.15, zp - 0.05],
        0.03,
    );
    let st = b.mark();
    let l = 1.6;
    b.rbox(
        "frame",
        [-l, yc - 0.11, zp - 0.04],
        [l, yc + 0.11, zp + 0.04],
        0.015,
    );
    for s in [-1.0, 1.0] {
        let x = s * (l - 0.25);
        b.rbox(
            "seat",
            [x - 0.18, yc - 0.15, zp + 0.04],
            [x + 0.18, yc + 0.15, zp + 0.08],
            0.03,
        );
        let hx = s * (l - 0.55);
        sweep(
            b,
            "metal",
            &[
                [hx, yc - 0.12, zp + 0.04],
                [hx, yc - 0.12, zp + 0.3],
                [hx, yc + 0.12, zp + 0.3],
                [hx, yc + 0.12, zp + 0.04],
            ],
            |_| 0.014,
            8,
            Cap::Flat,
            Cap::Flat,
        );
    }
    // Tilted about the pivot, the left end down.
    let tilt: f64 = (zp - 0.17) / l;
    let a = tilt.asin();
    let (c, s) = (a.cos(), a.sin());
    b.transform_from(st, |q| {
        let (x, z) = (q[0], q[2] - (zp - 0.05));
        [x * c - z * s, q[1], zp - 0.05 + x * s + z * c]
    });
    // The bumpers where the ends land.
    for x in [-l + 0.15, l - 0.15] {
        b.ellipsoid("dark", [x * c, yc, 0.06], [0.14, 0.14, 0.06]);
    }
}

/// A spring rider: a seat on a body over a coiled spring, with its
/// handles and footrests.
pub fn spring_rider(b: &mut Builder, _p: &Params) {
    let yc = 0.4;
    b.rbox("dark", [-0.2, yc - 0.2, 0.0], [0.2, yc + 0.2, 0.03], 0.01);
    let coil: Vec<P3> = (0..=120)
        .map(|k| {
            let t = k as f64 / 120.0;
            let a = TAU * 6.0 * t;
            [0.09 * a.cos(), yc + 0.09 * a.sin(), 0.04 + 0.4 * t]
        })
        .collect();
    sweep(b, "metal", &coil, |_| 0.014, 8, Cap::Flat, Cap::Flat);
    b.rbox(
        "metal",
        [-0.12, yc - 0.12, 0.44],
        [0.12, yc + 0.12, 0.48],
        0.01,
    );
    b.ellipsoid("accent", [0.0, yc, 0.62], [0.16, 0.38, 0.15]);
    b.ellipsoid("accent", [0.0, yc + 0.3, 0.8], [0.12, 0.1, 0.16]);
    b.ellipsoid("seat", [0.0, yc - 0.08, 0.76], [0.13, 0.17, 0.05]);
    rod(
        b,
        "metal",
        [-0.17, yc + 0.28, 0.92],
        [0.17, yc + 0.28, 0.92],
        0.016,
    );
    for s in [-1.0, 1.0] {
        b.ellipsoid("dark", [s * 0.19, yc + 0.28, 0.92], [0.035, 0.035, 0.035]);
    }
    rod(
        b,
        "metal",
        [-0.24, yc + 0.08, 0.5],
        [0.24, yc + 0.08, 0.5],
        0.015,
    );
}

/// A calisthenics station on safety flooring: three pull-up bars stepping
/// up, a horizontal ladder, parallel bars and low push-up bars.
pub fn calisthenics(b: &mut Builder, _p: &Params) {
    let (x0, x1, d) = (-3.4, 3.8, 2.4);
    b.rbox("ground", [x0, 0.0, 0.0], [x1, d, 0.03], 0.005);
    let z = 0.03;
    let post = |b: &mut Builder, x: f64, y: f64, h: f64| {
        rod(b, "frame", [x, y, z], [x, y, h], 0.045);
        b.ellipsoid("accent", [x, y, h], [0.05, 0.05, 0.03]);
    };
    // The pull-up bars, 2,0, 2,3 and 2,6 m.
    let y = 0.5;
    let xs = [-3.1, -2.1, -1.1, -0.1];
    let hs: [f64; 3] = [2.0, 2.3, 2.6];
    for (i, x) in xs.into_iter().enumerate() {
        let h = hs[i.min(2)].max(if i > 0 { hs[i - 1] } else { 0.0 }) + 0.12;
        post(b, x, y, h);
    }
    for (i, h) in hs.into_iter().enumerate() {
        rod(
            b,
            "metal",
            [xs[i] + 0.04, y, h],
            [xs[i + 1] - 0.04, y, h],
            0.016,
        );
    }
    // The horizontal ladder at 2,3 m.
    let (la, lb) = (0.5, 3.5);
    for x in [la, lb] {
        for y in [0.2, 0.8] {
            post(b, x, y, 2.35);
        }
    }
    for y in [0.2, 0.8] {
        rod(b, "frame", [la, y, 2.3], [lb, y, 2.3], 0.025);
    }
    let n = ((lb - la) / 0.35).floor() as usize;
    for k in 1..n {
        let x = la + (lb - la) * k as f64 / n as f64;
        rod(b, "metal", [x, 0.2, 2.3], [x, 0.8, 2.3], 0.016);
    }
    // The parallel bars at 1,1 m.
    for y in [1.55, 2.05] {
        for x in [-2.9, -1.1] {
            rod(b, "frame", [x, y, z], [x, y, 1.1], 0.035);
        }
        rod(b, "metal", [-3.0, y, 1.1], [-1.0, y, 1.1], 0.022);
    }
    // The push-up bars, low.
    for y in [1.6, 2.0] {
        for x in [0.8, 1.8] {
            rod(b, "frame", [x, y, z], [x, y, 0.3], 0.03);
        }
        rod(b, "metal", [0.75, y, 0.3], [1.85, y, 0.3], 0.018);
    }
}

/// A park elliptical: a post with two handles and two pedals on swinging
/// arms.
pub fn park_elliptical(b: &mut Builder, _p: &Params) {
    let yc = 0.35;
    b.rbox(
        "dark",
        [-0.25, yc - 0.25, 0.0],
        [0.25, yc + 0.25, 0.03],
        0.01,
    );
    rod(b, "frame", [0.0, yc, 0.03], [0.0, yc, 1.45], 0.06);
    b.ellipsoid("accent", [0.0, yc, 1.45], [0.07, 0.07, 0.05]);
    rod(b, "frame", [-0.25, yc, 1.25], [0.25, yc, 1.25], 0.035);
    for s in [-1.0, 1.0] {
        let x = s * 0.28;
        // The handle, swinging from the cross bar.
        sweep(
            b,
            "accent",
            &[
                [x, yc + 0.35, 1.5],
                [x, yc + 0.05, 1.25],
                [x, yc + 0.25, 0.45],
            ],
            |_| 0.022,
            10,
            Cap::Round(1.0),
            Cap::Flat,
        );
        // The pedal arm from low on the post, and the pedal.
        let (py, pz) = (yc + 0.45 + s * 0.08, 0.28 + s * 0.06);
        rod(
            b,
            "frame",
            [s * 0.06, yc, 0.55],
            [x, yc + 0.25, 0.45],
            0.022,
        );
        rod(b, "frame", [x, yc + 0.25, 0.45], [x, py, pz], 0.022);
        b.rbox(
            "dark",
            [x - 0.08, py - 0.14, pz - 0.02],
            [x + 0.08, py + 0.14, pz + 0.02],
            0.01,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fine::shape::signed_volume;

    type Make = fn(&mut Builder, &Params);

    #[test]
    fn the_playground_is_made() {
        let pieces: [(&str, Make); 6] = [
            ("columpio", swing),
            ("tobogán", slide),
            ("balancín", seesaw),
            ("muelle", spring_rider),
            ("calistenia", calisthenics),
            ("elíptica", park_elliptical),
        ];
        for (name, make) in pieces {
            let mut b = Builder::default();
            make(&mut b, &Params::default());
            let tris = b.triangle_count();
            assert!((100..40_000).contains(&tris), "{name}: {tris}");
            for (role, t) in &b.parts {
                let v = signed_volume(&b.positions, t);
                assert!(v > -1e-7, "{name} {role}: {v}");
            }
            let low = b.positions.iter().map(|q| q[2]).fold(f32::MAX, f32::min);
            assert!(low > -0.001, "{name}: {low}");
            let near = b.positions.iter().map(|q| q[1]).fold(f32::MAX, f32::min);
            assert!(near > -0.2, "{name}: {near}");
        }
    }
}
