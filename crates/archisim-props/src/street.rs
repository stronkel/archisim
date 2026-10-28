//! Street furniture (revisión de Antonio del 24/10/2026: «el mobiliario
//! urbano sigue siendo muy básico»): benches, streetlights, litter bins,
//! bollards, letter boxes, waste containers, drinking fountains, planters,
//! tree grates, bike racks, signs and bicycles, each in a few models.
//! Their axes are those their families always had (they keep the projects
//! made with them): see each function.

use crate::build::{Builder, P3};
use crate::Params;

use std::f64::consts::{PI, TAU};

/// A hoop of rod: the circle through `c` in the plane of the unit vectors
/// `u` and `v`, of radius `r`.
#[allow(clippy::too_many_arguments)]
fn hoop(b: &mut Builder, role: &'static str, c: P3, u: P3, v: P3, r: f64, rod: f64, n: usize) {
    let at = |a: f64| {
        let (s, co) = a.sin_cos();
        [
            c[0] + r * (u[0] * co + v[0] * s),
            c[1] + r * (u[1] * co + v[1] * s),
            c[2] + r * (u[2] * co + v[2] * s),
        ]
    };
    for i in 0..n {
        let a0 = TAU * i as f64 / n as f64;
        let a1 = TAU * (i + 1) as f64 / n as f64;
        b.cyl(role, at(a0), at(a1), rod);
    }
}

/// Benches, the back at `y` 0, the middle of the back at `x` 0: 1 of
/// slats on cast iron legs, 2 a block of concrete with a wooden seat, 3
/// of slats on a steel frame, without back if it has none.
pub fn bench(b: &mut Builder, p: &Params) {
    // A collection's (paquete 5, tanda 5b).
    if crate::fine::urban::bench(b, p) {
        return;
    }
    let w = p.get("width", 1.8).clamp(0.6, 6.0);
    let d = p.get("depth", 0.5).clamp(0.3, 0.8);
    let seat = p.get("seat", 0.45).clamp(0.3, 0.6);
    let back = p.flag("back", true);
    let top = p.get("height", 0.85).max(seat + 0.15);
    let x0 = -w / 2.0;
    let legs: Vec<f64> = {
        let n = ((w / 1.6).ceil() as usize).max(1) + 1;
        (0..n)
            .map(|i| x0 + 0.12 + (w - 0.24) * i as f64 / (n - 1) as f64)
            .collect()
    };
    match p.style() {
        2 => {
            b.rbox(
                "concrete",
                [x0, 0.04, 0.0],
                [-x0, d - 0.04, seat - 0.06],
                0.02,
            );
            let n = 4;
            for i in 0..n {
                let y = 0.02 + (d - 0.04) * i as f64 / n as f64;
                b.rbox(
                    "mat",
                    [x0 - 0.02, y + 0.006, seat - 0.06],
                    [-x0 + 0.02, y + (d - 0.04) / n as f64 - 0.006, seat],
                    0.008,
                );
            }
            if back {
                b.rbox("concrete", [x0, 0.0, seat - 0.06], [-x0, 0.06, top], 0.02);
            }
        }
        3 => {
            for &x in &legs {
                // A closed frame of flat bar: feet, seat and, with a back,
                // up behind.
                b.rbox(
                    "frame",
                    [x - 0.01, 0.02, 0.0],
                    [x + 0.01, d - 0.02, 0.025],
                    0.003,
                );
                b.rbox(
                    "frame",
                    [x - 0.01, d - 0.06, 0.0],
                    [x + 0.01, d - 0.02, seat - 0.04],
                    0.003,
                );
                b.rbox(
                    "frame",
                    [x - 0.01, 0.02, 0.0],
                    [x + 0.01, 0.06, seat - 0.04],
                    0.003,
                );
                b.rbox(
                    "frame",
                    [x - 0.01, 0.02, seat - 0.06],
                    [x + 0.01, d - 0.02, seat - 0.04],
                    0.003,
                );
                if back {
                    b.rbox(
                        "frame",
                        [x - 0.01, 0.0, seat - 0.06],
                        [x + 0.01, 0.04, top],
                        0.003,
                    );
                }
            }
            slats(b, x0, -x0, 0.02, d, seat - 0.04, 5);
            if back {
                back_slats(b, x0, -x0, seat + 0.06, top, 3);
            }
        }
        _ => {
            for &x in &legs {
                // A cast leg: a curved foot and the arm rest.
                b.rbox(
                    "frame",
                    [x - 0.025, 0.0, 0.0],
                    [x + 0.025, d - 0.02, 0.05],
                    0.012,
                );
                b.cone(
                    "frame",
                    [x, d - 0.08, 0.04],
                    [x, d - 0.1, seat - 0.04],
                    0.03,
                    0.022,
                    10,
                );
                b.cone(
                    "frame",
                    [x, 0.08, 0.04],
                    [x, 0.1, seat - 0.04],
                    0.03,
                    0.022,
                    10,
                );
                b.rbox(
                    "frame",
                    [x - 0.022, 0.04, seat - 0.07],
                    [x + 0.022, d - 0.04, seat - 0.04],
                    0.01,
                );
                if back {
                    b.cyl("frame", [x, 0.08, seat - 0.05], [x, 0.03, top], 0.022);
                }
                if x == legs[0] || x == *legs.last().expect("two legs") {
                    b.rbox(
                        "frame",
                        [x - 0.025, 0.05, seat + 0.18],
                        [x + 0.025, d - 0.02, seat + 0.22],
                        0.015,
                    );
                    b.cyl(
                        "frame",
                        [x, d - 0.05, seat - 0.04],
                        [x, d - 0.05, seat + 0.2],
                        0.018,
                    );
                }
            }
            slats(b, x0, -x0, 0.06, d - 0.02, seat - 0.04, 5);
            if back {
                back_slats(b, x0, -x0, seat + 0.08, top, 4);
            }
        }
    }
}

/// Slats along a bench's seat, from `y0` to `y1`, their top at `z`.
fn slats(b: &mut Builder, x0: f64, x1: f64, y0: f64, y1: f64, z: f64, n: usize) {
    let s = (y1 - y0) / n as f64;
    for i in 0..n {
        let y = y0 + i as f64 * s;
        b.rbox(
            "mat",
            [x0, y + 0.007, z],
            [x1, y + s - 0.007, z + 0.035],
            0.008,
        );
    }
}

/// Slats of a bench's back, leaning a little, from `z0` to `z1`.
fn back_slats(b: &mut Builder, x0: f64, x1: f64, z0: f64, z1: f64, n: usize) {
    let s = (z1 - z0) / n as f64;
    for i in 0..n {
        let z = z0 + i as f64 * s;
        let lean = 0.06 * (i as f64 + 0.5) / n as f64;
        b.rbox(
            "mat",
            [x0, 0.02 - lean, z + 0.01],
            [x1, 0.055 - lean, z + s - 0.01],
            0.008,
        );
    }
}

/// Streetlights, the foot of the post at the origin and the arm towards
/// `+y`: 1 a modern bent post with a flat head, 2 a classic lantern on a
/// fluted column (the Fernandina of Spanish squares), 3 a column with a
/// globe, 4 a low bollard light. The light glows at night (role `light`).
pub fn streetlight(b: &mut Builder, p: &Params) {
    // A collection's (paquete 5, tanda 5b).
    if crate::fine::urban::streetlight(b, p) {
        return;
    }
    let h = p.get("height", 4.5).clamp(0.4, 16.0);
    let arm = p.get("arm", 0.8).clamp(0.0, 3.0);
    let pole = p.get("pole", 0.12).clamp(0.05, 0.4) / 2.0;
    let lamp = p.get("lamp", 0.5).clamp(0.15, 1.5);
    match p.style() {
        2 => {
            // A stepped base, the fluted column with its rings, a lantern
            // of four panes under a cap.
            b.cone(
                "frame",
                [0.0, 0.0, 0.0],
                [0.0, 0.0, 0.25],
                pole * 2.4,
                pole * 2.0,
                16,
            );
            b.cone(
                "frame",
                [0.0, 0.0, 0.25],
                [0.0, 0.0, 0.5],
                pole * 1.7,
                pole * 1.4,
                16,
            );
            b.cone(
                "frame",
                [0.0, 0.0, 0.5],
                [0.0, 0.0, h - 0.7],
                pole * 1.15,
                pole * 0.85,
                12,
            );
            for z in [0.9, h * 0.55, h - 0.75] {
                b.cone(
                    "frame",
                    [0.0, 0.0, z - 0.03],
                    [0.0, 0.0, z + 0.03],
                    pole * 1.4,
                    pole * 1.4,
                    16,
                );
            }
            let s = (lamp * 0.45).max(0.12);
            let z0 = h - 0.65;
            b.cone(
                "frame",
                [0.0, 0.0, z0],
                [0.0, 0.0, z0 + 0.06],
                s * 0.5,
                s * 0.8,
                4,
            );
            b.cone(
                "light",
                [0.0, 0.0, z0 + 0.06],
                [0.0, 0.0, z0 + 0.5],
                s * 0.75,
                s,
                4,
            );
            for k in 0..4 {
                let a = PI / 4.0 + k as f64 * PI / 2.0;
                let (x, y) = (a.cos() * s * 0.98, a.sin() * s * 0.98);
                b.cyl(
                    "frame",
                    [x * 0.76, y * 0.76, z0 + 0.06],
                    [x, y, z0 + 0.5],
                    0.008,
                );
            }
            b.cone(
                "frame",
                [0.0, 0.0, z0 + 0.5],
                [0.0, 0.0, z0 + 0.62],
                s * 1.25,
                s * 0.25,
                4,
            );
            b.ellipsoid("frame", [0.0, 0.0, z0 + 0.66], [0.03, 0.03, 0.04]);
        }
        3 => {
            b.cone(
                "frame",
                [0.0, 0.0, 0.0],
                [0.0, 0.0, 0.08],
                pole * 2.0,
                pole * 1.8,
                16,
            );
            b.cyl(
                "frame",
                [0.0, 0.0, 0.08],
                [0.0, 0.0, h - lamp * 0.8],
                pole * 0.8,
            );
            b.cone(
                "frame",
                [0.0, 0.0, h - lamp * 0.85],
                [0.0, 0.0, h - lamp * 0.75],
                pole * 1.6,
                pole * 1.2,
                16,
            );
            let r = lamp * 0.4;
            b.ellipsoid("light", [0.0, 0.0, h - r], [r, r, r]);
        }
        4 => {
            // A low bollard light: a post with a band of light near the top.
            let top = h.min(1.2);
            let r = pole.max(0.08);
            b.cyl("frame", [0.0, 0.0, 0.0], [0.0, 0.0, top - 0.16], r);
            b.cyl(
                "light",
                [0.0, 0.0, top - 0.16],
                [0.0, 0.0, top - 0.04],
                r * 0.92,
            );
            b.cone(
                "frame",
                [0.0, 0.0, top - 0.04],
                [0.0, 0.0, top],
                r * 1.08,
                r * 1.0,
                20,
            );
        }
        _ => {
            b.cone(
                "frame",
                [0.0, 0.0, 0.0],
                [0.0, 0.0, 0.4],
                pole * 1.6,
                pole * 1.3,
                16,
            );
            let bend = (arm * 0.5).min(0.6);
            b.cone(
                "frame",
                [0.0, 0.0, 0.4],
                [0.0, 0.0, h - bend],
                pole * 1.1,
                pole * 0.8,
                16,
            );
            // The bend into the arm, and the flat head over the street.
            let steps = 6;
            let mut last = [0.0, 0.0, h - bend];
            for i in 1..=steps {
                let a = PI / 2.0 * i as f64 / steps as f64;
                let q = [0.0, bend * (1.0 - a.cos()), h - bend + bend * a.sin()];
                b.cyl("frame", last, q, pole * 0.75);
                last = q;
            }
            if arm > bend {
                b.cyl("frame", last, [0.0, arm, h], pole * 0.7);
            }
            let y = arm.max(bend);
            b.rbox(
                "frame",
                [-0.14, y - lamp * 0.2, h - 0.06],
                [0.14, y + lamp * 0.8, h + 0.03],
                0.03,
            );
            b.rbox(
                "light",
                [-0.11, y - lamp * 0.1, h - 0.075],
                [0.11, y + lamp * 0.7, h - 0.06],
                0.01,
            );
        }
    }
}

/// Litter bins, centred on the origin: 1 of perforated sheet on a post, 2
/// of wooden slats round a drum, 3 a pair for sorting with a lid each.
pub fn litter_bin(b: &mut Builder, p: &Params) {
    // A collection's (paquete 5, tanda 5b).
    if crate::fine::urban::litter_bin(b, p) {
        return;
    }
    let r = p.get("width", 0.4).clamp(0.2, 1.0) / 2.0;
    let h = p.get("height", 0.9).clamp(0.4, 1.4);
    match p.style() {
        2 => {
            b.cyl("dark", [0.0, 0.0, 0.0], [0.0, 0.0, h - 0.02], r * 0.9);
            let n = 16;
            for i in 0..n {
                let a = TAU * i as f64 / n as f64;
                let (c, s) = (a.cos(), a.sin());
                b.rbox(
                    "mat",
                    [c * r * 0.93 - 0.03, s * r * 0.93 - 0.03, 0.03],
                    [c * r * 0.93 + 0.03, s * r * 0.93 + 0.03, h],
                    0.01,
                );
            }
            for z in [0.12, h - 0.12] {
                b.cone(
                    "frame",
                    [0.0, 0.0, z - 0.015],
                    [0.0, 0.0, z + 0.015],
                    r + 0.012,
                    r + 0.012,
                    24,
                );
            }
        }
        3 => {
            for (k, role) in [(-1.0, "mat"), (1.0, "mat2")] {
                let x = k * r * 0.55;
                b.rbox(
                    role,
                    [x - r * 0.5, -r * 0.6, 0.02],
                    [x + r * 0.5, r * 0.6, h - 0.06],
                    0.03,
                );
                b.rbox(
                    "frame",
                    [x - r * 0.52, -r * 0.62, h - 0.06],
                    [x + r * 0.52, r * 0.62, h],
                    0.02,
                );
                b.rbox(
                    "dark",
                    [x - r * 0.2, r * 0.6, h - 0.2],
                    [x + r * 0.2, r * 0.62, h - 0.12],
                    0.0,
                );
            }
        }
        _ => {
            b.cyl(
                "frame",
                [0.0, -r - 0.05, 0.0],
                [0.0, -r - 0.05, h + 0.05],
                0.03,
            );
            b.lathe(
                "mat",
                [0.0, 0.0, 0.18],
                &[
                    (0.0, 0.0),
                    (r * 0.82, 0.0),
                    (r, h - 0.22),
                    (r * 1.02, h - 0.18),
                    (0.0, h - 0.18),
                ],
                1.0,
                1.0,
            );
            // Its holes: rings of dark dots round the drum.
            for row in 0..5 {
                let z = 0.3 + (h - 0.5) * row as f64 / 5.0;
                for k in 0..12 {
                    let a = TAU * (k as f64 + 0.5 * (row % 2) as f64) / 12.0;
                    let rr = r * 0.84 + (r - r * 0.82) * (z - 0.18) / (h - 0.22);
                    let q = [a.cos() * rr, a.sin() * rr, z];
                    b.cyl("dark", q, [q[0] * 1.03, q[1] * 1.03, z], 0.012);
                }
            }
        }
    }
}

/// Bollards, centred on the origin: 1 of cast iron with a ball on top, 2
/// a steel tube with a reflective band, 3 a cone of concrete.
pub fn bollard(b: &mut Builder, p: &Params) {
    // A collection's (paquete 5, tanda 5b).
    if crate::fine::urban::bollard(b, p) {
        return;
    }
    let r = p.get("width", 0.15).clamp(0.06, 0.6) / 2.0;
    let h = p.get("height", 0.8).clamp(0.3, 1.5);
    match p.style() {
        2 => {
            b.cyl("mat", [0.0, 0.0, 0.0], [0.0, 0.0, h - 0.01], r);
            b.cyl(
                "light",
                [0.0, 0.0, h - 0.14],
                [0.0, 0.0, h - 0.08],
                r * 1.01,
            );
            b.cone("mat", [0.0, 0.0, h - 0.01], [0.0, 0.0, h], r, r * 0.8, 20);
        }
        3 => {
            b.lathe(
                "concrete",
                [0.0, 0.0, 0.0],
                &[
                    (0.0, 0.0),
                    (r * 1.8, 0.0),
                    (r * 1.8, 0.05),
                    (r * 1.1, h * 0.92),
                    (r * 0.9, h),
                    (0.0, h),
                ],
                1.0,
                1.0,
            );
        }
        _ => {
            b.lathe(
                "mat",
                [0.0, 0.0, 0.0],
                &[
                    (0.0, 0.0),
                    (r * 1.35, 0.0),
                    (r * 1.35, 0.04),
                    (r * 1.0, 0.1),
                    (r * 0.85, h * 0.5),
                    (r * 0.95, h - 0.18),
                    (r * 1.15, h - 0.14),
                    (r * 0.6, h - 0.1),
                    (0.0, h - 0.1),
                ],
                1.0,
                1.0,
            );
            b.ellipsoid("mat", [0.0, 0.0, h - r * 0.8], [r * 0.8, r * 0.8, r * 0.8]);
        }
    }
}

/// Letter boxes, the back at `y` 0: 1 a pillar box on its foot, 2 a
/// rounded one on a post, 3 a block of letter boxes for a building.
pub fn letter_box(b: &mut Builder, p: &Params) {
    let w = p.get("width", 0.45).clamp(0.25, 1.2);
    let d = p.get("depth", 0.35).clamp(0.2, 0.8);
    let h = p.get("height", 1.1).clamp(0.5, 1.8);
    let x0 = -w / 2.0;
    match p.style() {
        2 => {
            b.cyl("frame", [0.0, d / 2.0, 0.0], [0.0, d / 2.0, h * 0.5], 0.035);
            b.rbox("mat", [x0, 0.0, h * 0.5], [-x0, d, h - d * 0.3], 0.03);
            b.lathe(
                "mat",
                [0.0, d / 2.0, h - d * 0.32],
                &[
                    (d * 0.5, 0.0),
                    (d * 0.45, d * 0.15),
                    (d * 0.25, d * 0.28),
                    (0.0, d * 0.32),
                ],
                w / d,
                1.0,
            );
            b.rbox(
                "dark",
                [x0 * 0.6, d, h - d * 0.45],
                [-x0 * 0.6, d + 0.008, h - d * 0.4],
                0.0,
            );
        }
        3 => {
            let cols = ((w / 0.3).round() as usize).max(1);
            let rows = ((h / 0.32).round() as usize).clamp(2, 6);
            let z0 = (h - rows as f64 * 0.3).max(0.6);
            b.rbox(
                "frame",
                [x0, 0.0, z0],
                [-x0, d.min(0.35), z0 + rows as f64 * 0.3],
                0.01,
            );
            for i in 0..cols {
                for j in 0..rows {
                    let a = x0 + w * i as f64 / cols as f64;
                    let z = z0 + 0.3 * j as f64;
                    let y = d.min(0.35);
                    b.rbox(
                        "mat",
                        [a + 0.01, y, z + 0.01],
                        [a + w / cols as f64 - 0.01, y + 0.012, z + 0.29],
                        0.003,
                    );
                    b.rbox(
                        "dark",
                        [a + 0.04, y + 0.012, z + 0.22],
                        [a + w / cols as f64 - 0.04, y + 0.016, z + 0.235],
                        0.0,
                    );
                }
            }
        }
        _ => {
            b.rbox(
                "frame",
                [x0 + 0.05, 0.05, 0.0],
                [-x0 - 0.05, d - 0.05, 0.08],
                0.01,
            );
            b.rbox("mat", [x0, 0.0, 0.08], [-x0, d, h - 0.06], 0.04);
            b.lathe(
                "mat",
                [0.0, d / 2.0, h - 0.07],
                &[(0.5, 0.0), (0.5, 0.03), (0.3, 0.07), (0.0, 0.08)],
                w,
                d,
            );
            b.rbox(
                "dark",
                [x0 * 0.55, d, h - 0.3],
                [-x0 * 0.55, d + 0.01, h - 0.26],
                0.0,
            );
        }
    }
}

/// Waste containers, the back at `y` 0: 1 for side loading with a lid
/// and pedal, 2 an igloo for glass, 3 the bin of a buried one, its mouth
/// on a steel slab.
pub fn container(b: &mut Builder, p: &Params) {
    let w = p.get("width", 1.4).clamp(0.5, 3.0);
    let d = p.get("depth", 0.9).clamp(0.4, 2.0);
    let h = p.get("height", 1.3).clamp(0.8, 2.0);
    let x0 = -w / 2.0;
    match p.style() {
        2 => {
            let r = w.min(d * 1.4) / 2.0;
            b.lathe(
                "mat",
                [0.0, d / 2.0, 0.0],
                &[
                    (0.0, 0.0),
                    (r, 0.0),
                    (r * 0.98, h * 0.4),
                    (r * 0.8, h * 0.8),
                    (r * 0.35, h),
                    (0.0, h),
                ],
                1.0,
                d / w.max(d),
            );
            for k in 0..2 {
                let a = PI * 0.35 + k as f64 * PI * 0.3;
                b.rbox(
                    "dark",
                    [a.cos() * r * 0.4 - 0.1, d / 2.0 - 0.06, h * 0.7],
                    [a.cos() * r * 0.4 + 0.1, d / 2.0 + 0.06, h * 0.72],
                    0.0,
                );
            }
            b.cyl("frame", [0.0, d / 2.0, h], [0.0, d / 2.0, h + 0.15], 0.05);
        }
        3 => {
            b.rbox("frame", [x0, 0.0, 0.0], [-x0, d, 0.03], 0.0);
            b.cyl(
                "mat",
                [0.0, d / 2.0, 0.03],
                [0.0, d / 2.0, h * 0.85],
                (w.min(d) * 0.28).min(0.35),
            );
            b.rbox(
                "mat",
                [-0.25, d / 2.0 - 0.25, h * 0.85],
                [0.25, d / 2.0 + 0.25, h],
                0.05,
            );
            b.rbox(
                "dark",
                [-0.18, d / 2.0 + 0.25, h * 0.88],
                [0.18, d / 2.0 + 0.26, h * 0.95],
                0.0,
            );
        }
        _ => {
            // Wheels, a body a little narrower at the foot, a lid.
            for x in [x0 + 0.12, -x0 - 0.12] {
                for y in [0.15, d - 0.15] {
                    b.cone(
                        "dark",
                        [x - 0.03, y, 0.06],
                        [x + 0.03, y, 0.06],
                        0.06,
                        0.06,
                        12,
                    );
                }
            }
            b.slab(
                "mat",
                &[
                    [x0 + 0.04, 0.04],
                    [-x0 - 0.04, 0.04],
                    [-x0 - 0.04, d - 0.04],
                    [x0 + 0.04, d - 0.04],
                ],
                0.1,
                0.12,
            );
            b.rbox(
                "mat",
                [x0 + 0.02, 0.02, 0.12],
                [-x0 - 0.02, d - 0.02, h - 0.08],
                0.04,
            );
            b.rbox("mat", [x0, 0.0, h - 0.1], [-x0, d + 0.03, h], 0.03);
            b.rbox(
                "frame",
                [x0 + 0.2, d - 0.02, 0.18],
                [x0 + 0.45, d + 0.08, 0.22],
                0.01,
            );
            for k in 0..4 {
                let x = x0 + w * (k as f64 + 0.5) / 4.0;
                b.rbox(
                    "dark",
                    [x - 0.006, d - 0.02, 0.3],
                    [x + 0.006, d - 0.018, h - 0.2],
                    0.0,
                );
            }
        }
    }
}

/// Drinking fountains, the back at `y` 0: 1 a cast iron column with its
/// basin and grate, 2 a stone block with a spout, 3 a round pool with a
/// jet (an ornamental fountain; its width is the pool's).
pub fn fountain(b: &mut Builder, p: &Params) {
    // A collection's (paquete 5, tanda 5b).
    if crate::fine::urban::fountain(b, p) {
        return;
    }
    let w = p.get("width", 0.35).clamp(0.2, 6.0);
    let d = p.get("depth", 0.35).clamp(0.2, 6.0);
    let h = p.get("height", 1.0).clamp(0.4, 2.0);
    let c = [0.0, d / 2.0];
    match p.style() {
        2 => {
            b.rbox("concrete", [-w / 2.0, 0.0, 0.0], [w / 2.0, d, h], 0.02);
            b.hollow(
                "water",
                [-w * 0.35, d * 0.55, h * 0.45],
                [w * 0.35, d, h * 0.55],
                0.02,
            );
            b.cyl(
                "frame",
                [0.0, d * 0.2, h * 0.8],
                [0.0, d * 0.75, h * 0.8],
                0.012,
            );
        }
        3 => {
            let r = w.min(d) / 2.0;
            b.lathe(
                "concrete",
                [c[0], c[1], 0.0],
                &[
                    (0.0, 0.0),
                    (r, 0.0),
                    (r, 0.45),
                    (r - 0.12, 0.45),
                    (r - 0.12, 0.25),
                    (0.0, 0.25),
                ],
                1.0,
                1.0,
            );
            b.cyl("water", [c[0], c[1], 0.25], [c[0], c[1], 0.38], r - 0.12);
            b.lathe(
                "concrete",
                [c[0], c[1], 0.38],
                &[
                    (0.0, 0.0),
                    (r * 0.12, 0.0),
                    (r * 0.08, h * 0.5),
                    (r * 0.3, h * 0.55),
                    (r * 0.3, h * 0.6),
                    (0.0, h * 0.6),
                ],
                1.0,
                1.0,
            );
            b.cone(
                "water",
                [c[0], c[1], 0.38 + h * 0.6],
                [c[0], c[1], 0.38 + h * 0.95],
                0.03,
                0.005,
                10,
            );
        }
        _ => {
            let r = w.min(d) / 2.0;
            b.rbox("frame", [-r * 1.2, 0.0, 0.0], [r * 1.2, d, 0.04], 0.0);
            b.lathe(
                "mat",
                [c[0], c[1], 0.04],
                &[
                    (0.0, 0.0),
                    (r * 0.6, 0.0),
                    (r * 0.45, 0.15),
                    (r * 0.35, h * 0.7),
                    (r * 0.5, h - 0.12),
                    (r * 0.25, h - 0.04),
                    (0.0, h),
                ],
                1.0,
                1.0,
            );
            b.cyl(
                "mat",
                [c[0], c[1], h * 0.78],
                [c[0], c[1] + r * 0.9, h * 0.78],
                0.02,
            );
            b.lathe(
                "mat",
                [c[0], c[1] + r * 0.95, h * 0.45],
                &[(0.0, 0.0), (r * 0.2, 0.0), (r * 0.45, 0.08), (0.0, 0.08)],
                1.0,
                1.0,
            );
            b.cyl(
                "water",
                [c[0], c[1] + r * 0.95, h * 0.45 + 0.07],
                [c[0], c[1] + r * 0.95, h * 0.45 + 0.08],
                r * 0.38,
            );
        }
    }
}

/// Planters, the back at `y` 0: 1 of concrete with shrubs, 2 of wooden
/// boards with flowers, 3 of weathering steel with tall grasses.
pub fn planter(b: &mut Builder, p: &Params) {
    // A collection's (paquete 5, tanda 5b).
    if crate::fine::urban::planter(b, p) {
        return;
    }
    let w = p.get("width", 1.2).clamp(0.4, 6.0);
    let d = p.get("depth", 0.5).clamp(0.3, 2.0);
    let h = p.get("height", 0.5).clamp(0.2, 1.2);
    let x0 = -w / 2.0;
    let style = p.style();
    match style {
        2 => {
            let rows = ((h / 0.12).round() as usize).max(2);
            for i in 0..rows {
                let z = h * i as f64 / rows as f64;
                let t = h / rows as f64;
                b.rbox(
                    "mat",
                    [x0, 0.0, z + 0.004],
                    [-x0, 0.025, z + t - 0.004],
                    0.004,
                );
                b.rbox(
                    "mat",
                    [x0, d - 0.025, z + 0.004],
                    [-x0, d, z + t - 0.004],
                    0.004,
                );
                b.rbox(
                    "mat",
                    [x0, 0.0, z + 0.004],
                    [x0 + 0.025, d, z + t - 0.004],
                    0.004,
                );
                b.rbox(
                    "mat",
                    [-x0 - 0.025, 0.0, z + 0.004],
                    [-x0, d, z + t - 0.004],
                    0.004,
                );
            }
            for (x, y) in [(x0, 0.0), (-x0, 0.0), (x0, d), (-x0, d)] {
                b.rbox(
                    "frame",
                    [x - 0.03, y - 0.03, 0.0],
                    [x + 0.03, y + 0.03, h + 0.03],
                    0.006,
                );
            }
        }
        3 => {
            b.rbox("mat", [x0, 0.0, 0.0], [-x0, d, 0.01], 0.0);
            for (lo, hi) in [
                ([x0, -0.006, 0.0], [-x0, 0.0, h]),
                ([x0, d, 0.0], [-x0, d + 0.006, h]),
                ([x0 - 0.006, 0.0, 0.0], [x0, d, h]),
                ([-x0, 0.0, 0.0], [-x0 + 0.006, d, h]),
            ] {
                b.rbox("mat", lo, hi, 0.0);
            }
        }
        _ => {
            b.rbox("mat", [x0, 0.0, 0.0], [-x0, d, h], 0.025);
        }
    }
    b.rbox(
        "soil",
        [x0 + 0.04, 0.04, h - 0.06],
        [-x0 - 0.04, d - 0.04, h - 0.02],
        0.0,
    );
    // What grows in it, by a fixed seed.
    let n = ((w / 0.3).round() as usize).clamp(2, 20);
    for i in 0..n {
        let x = x0 + 0.1 + (w - 0.2) * (i as f64 + 0.5) / n as f64;
        let y = d * (0.35 + 0.3 * ((i * 7 % 5) as f64 / 5.0));
        let s = 0.13 + 0.05 * ((i * 3 % 4) as f64 / 4.0);
        match style {
            2 => {
                b.ellipsoid("leaves", [x, y, h + s * 0.5], [s, s * 0.9, s * 0.7]);
                for k in 0..4 {
                    let a = TAU * k as f64 / 4.0 + i as f64;
                    b.ellipsoid(
                        "flowers",
                        [x + a.cos() * s * 0.6, y + a.sin() * s * 0.5, h + s * 0.95],
                        [0.03, 0.03, 0.02],
                    );
                }
            }
            3 => {
                for k in 0..6 {
                    let a = TAU * (k as f64 / 6.0 + i as f64 * 0.17);
                    let tall = 0.35 + 0.15 * ((k + i) % 3) as f64;
                    b.blob(
                        "leaves",
                        [x + a.cos() * 0.05, y + a.sin() * 0.05, h + tall * 0.5],
                        [-a.sin() * 0.012, a.cos() * 0.012, 0.0],
                        [a.cos() * 0.006, a.sin() * 0.006, 0.0],
                        [a.cos() * 0.08, a.sin() * 0.08, tall * 0.5],
                    );
                }
            }
            _ => b.ellipsoid("leaves", [x, y, h + s * 0.7], [s * 1.2, s * 1.1, s]),
        }
    }
}

/// Tree grates, centred on the origin: 1 of cast iron in rings, 2 of
/// steel bars, 3 a border of stone around loose gravel.
pub fn tree_grate(b: &mut Builder, p: &Params) {
    let w = p.get("width", 1.2).clamp(0.6, 3.0);
    let h = w / 2.0;
    match p.style() {
        2 => {
            b.rbox("frame", [-h, -h, 0.0], [h, -h + 0.04, 0.025], 0.0);
            b.rbox("frame", [-h, h - 0.04, 0.0], [h, h, 0.025], 0.0);
            let n = ((w / 0.05).round() as usize).max(8);
            for i in 0..n {
                let x = -h + w * (i as f64 + 0.5) / n as f64;
                if x.abs() < 0.18 {
                    continue;
                }
                b.rbox(
                    "mat",
                    [x - 0.008, -h + 0.04, 0.0],
                    [x + 0.008, h - 0.04, 0.02],
                    0.0,
                );
            }
        }
        3 => {
            for (lo, hi) in [
                ([-h, -h, 0.0], [h, -h + 0.1, 0.06]),
                ([-h, h - 0.1, 0.0], [h, h, 0.06]),
                ([-h, -h + 0.1, 0.0], [-h + 0.1, h - 0.1, 0.06]),
                ([h - 0.1, -h + 0.1, 0.0], [h, h - 0.1, 0.06]),
            ] {
                b.rbox("concrete", lo, hi, 0.01);
            }
            b.rbox(
                "soil",
                [-h + 0.1, -h + 0.1, 0.0],
                [h - 0.1, h - 0.1, 0.03],
                0.0,
            );
        }
        _ => {
            b.rbox("frame", [-h, -h, 0.0], [h, -h + 0.04, 0.02], 0.0);
            b.rbox("frame", [-h, h - 0.04, 0.0], [h, h, 0.02], 0.0);
            b.rbox("frame", [-h, -h, 0.0], [-h + 0.04, h, 0.02], 0.0);
            b.rbox("frame", [h - 0.04, -h, 0.0], [h, h, 0.02], 0.0);
            let rings = ((h / 0.08) as usize).max(3);
            for k in 1..rings {
                let r = 0.15 + (h - 0.2) * k as f64 / rings as f64;
                hoop(
                    b,
                    "mat",
                    [0.0, 0.0, 0.012],
                    [1.0, 0.0, 0.0],
                    [0.0, 1.0, 0.0],
                    r,
                    0.01,
                    32,
                );
            }
            for k in 0..8 {
                let a = TAU * k as f64 / 8.0;
                b.cyl(
                    "mat",
                    [a.cos() * 0.15, a.sin() * 0.15, 0.012],
                    [a.cos() * h * 0.98, a.sin() * h * 0.98, 0.012],
                    0.01,
                );
            }
        }
    }
}

/// Bike racks: `count` stands `step` apart along `x` from the origin,
/// `width` across `y` centred on it. 1 an inverted U, 2 a wave of tube, 3
/// a stand with a slot for the wheel.
pub fn bike_rack(b: &mut Builder, p: &Params) {
    // A collection's (paquete 5, tanda 5b).
    if crate::fine::urban::bike_rack(b, p) {
        return;
    }
    let n = p.get("count", 5.0).round().clamp(1.0, 20.0) as usize;
    let step = p.get("step", 0.8).clamp(0.5, 1.5);
    let w = p.get("width", 0.7).clamp(0.4, 1.2);
    let h = p.get("height", 0.8).clamp(0.5, 1.2);
    let r = 0.025;
    match p.style() {
        2 => {
            // One tube bent up and down along the row.
            let pts = n * 8;
            let len = step * n as f64 - step * 0.5;
            let at = |t: f64| {
                let x = -step * 0.25 + len * t;
                let z = 0.05 + (h - 0.05) * (0.5 - 0.5 * (TAU * t * n as f64).cos());
                [x, 0.0, z]
            };
            for i in 0..pts {
                b.cyl(
                    "mat",
                    at(i as f64 / pts as f64),
                    at((i + 1) as f64 / pts as f64),
                    r,
                );
            }
            for t in [0.0, 1.0] {
                let q = at(t);
                b.cyl("mat", [q[0], 0.0, 0.0], q, r);
            }
        }
        3 => {
            for i in 0..n {
                let x = i as f64 * step;
                b.rbox(
                    "mat",
                    [x - 0.06, -w / 2.0, 0.0],
                    [x + 0.06, w / 2.0, 0.05],
                    0.01,
                );
                for dx in [-0.035, 0.035] {
                    b.cyl(
                        "mat",
                        [x + dx, -w * 0.2, 0.05],
                        [x + dx, -w * 0.05, h * 0.55],
                        0.012,
                    );
                    b.cyl(
                        "mat",
                        [x + dx, -w * 0.05, h * 0.55],
                        [x + dx, w * 0.2, 0.05],
                        0.012,
                    );
                }
            }
        }
        _ => {
            for i in 0..n {
                let x = i as f64 * step;
                let rr = w * 0.25;
                b.cyl("mat", [x, -w / 2.0, 0.0], [x, -w / 2.0, h - rr], r);
                b.cyl("mat", [x, w / 2.0, 0.0], [x, w / 2.0, h - rr], r);
                let steps = 8;
                let at = |k: usize| {
                    let a = PI * k as f64 / steps as f64;
                    [x, -(w / 2.0 - rr) - rr * a.cos(), h - rr + rr * a.sin()]
                };
                b.cyl("mat", [x, -w / 2.0, h - rr], at(0), r);
                for k in 0..steps {
                    b.cyl("mat", at(k), at(k + 1), r);
                }
                b.cyl("mat", [x, -(w / 2.0 - rr), h], [x, w / 2.0 - rr, h], r);
                let at2 = |k: usize| {
                    let a = PI * k as f64 / steps as f64;
                    [x, (w / 2.0 - rr) + rr * a.cos(), h - rr + rr * a.sin()]
                };
                for k in 0..steps {
                    b.cyl("mat", at2(k), at2(k + 1), r);
                }
                // A crossbar and a footing plate.
                b.cyl(
                    "mat",
                    [x, -w / 2.0, h * 0.45],
                    [x, w / 2.0, h * 0.45],
                    r * 0.8,
                );
                for y in [-w / 2.0, w / 2.0] {
                    b.rbox(
                        "frame",
                        [x - 0.06, y - 0.06, 0.0],
                        [x + 0.06, y + 0.06, 0.01],
                        0.0,
                    );
                }
            }
        }
    }
}

/// Signs, the post at the origin and the plate towards `+y`: 1 round
/// (prohibition), 2 a triangle (warning), 3 a rectangle (information).
pub fn sign(b: &mut Builder, p: &Params) {
    let h = p.get("height", 2.6).clamp(1.5, 5.0);
    let s = p.get("plate", 0.6).clamp(0.3, 1.5);
    b.cyl("mat", [0.0, 0.0, 0.0], [0.0, 0.0, h - 0.05], 0.03);
    b.cone(
        "mat",
        [0.0, 0.0, h - 0.05],
        [0.0, 0.0, h - 0.04],
        0.03,
        0.02,
        10,
    );
    let y = 0.045;
    let c = h - s / 2.0 - 0.05;
    match p.style() {
        2 => {
            let tri = |r: f64, z: f64| -> Vec<[f64; 2]> {
                (0..3)
                    .map(|k| {
                        let a = PI / 2.0 + TAU * k as f64 / 3.0;
                        [a.cos() * r, z + a.sin() * r]
                    })
                    .collect()
            };
            plate(b, "face2", &tri(s * 0.6, c), y, y + 0.01);
            plate(b, "face", &tri(s * 0.45, c), y + 0.01, y + 0.012);
        }
        3 => {
            let rect = |a: f64, z0: f64, z1: f64| vec![[-a, z0], [a, z0], [a, z1], [-a, z1]];
            plate(
                b,
                "face",
                &rect(s / 2.0, h - s * 0.75 - 0.05, h - 0.05),
                y,
                y + 0.01,
            );
            plate(
                b,
                "face2",
                &rect(s / 2.0 - 0.03, h - s * 0.75 - 0.02, h - 0.08),
                y + 0.01,
                y + 0.012,
            );
        }
        _ => {
            let circle = |r: f64| -> Vec<[f64; 2]> {
                (0..24)
                    .map(|k| {
                        let a = TAU * k as f64 / 24.0;
                        [a.cos() * r, c + a.sin() * r]
                    })
                    .collect()
            };
            plate(b, "face2", &circle(s / 2.0), y, y + 0.01);
            plate(b, "face", &circle(s * 0.38), y + 0.01, y + 0.012);
        }
    }
}

/// A flat plate in the plane `x`–`z`: a convex polygon `(x, z)`,
/// counter-clockwise, from `y0` to `y1`.
fn plate(b: &mut Builder, role: &'static str, poly: &[[f64; 2]], y0: f64, y1: f64) {
    let n = poly.len();
    let back: Vec<u32> = poly.iter().map(|q| b.point([q[0], y0, q[1]])).collect();
    let front: Vec<u32> = poly.iter().map(|q| b.point([q[0], y1, q[1]])).collect();
    for i in 1..n - 1 {
        b.tri(role, front[0], front[i + 1], front[i]);
        b.tri(role, back[0], back[i], back[i + 1]);
    }
    for i in 0..n {
        let j = (i + 1) % n;
        b.quad(role, back[i], front[i], front[j], back[j]);
    }
}

/// Bicycles, along `+y` from their back wheel, centred on `x` 0: 1 a
/// town bike with a basket, 2 a road bike, 3 a cargo bike with a box in
/// front.
pub fn bicycle(b: &mut Builder, p: &Params) {
    let l = p.get("length", 1.75).clamp(1.2, 2.6);
    let d = p.get("wheel", 0.68).clamp(0.4, 0.75);
    let r = d / 2.0;
    let style = p.style();
    let rear = [0.0, r, r];
    let front = [0.0, l - r, r];
    for c in [rear, front] {
        hoop(
            b,
            "tyre",
            c,
            [0.0, 1.0, 0.0],
            [0.0, 0.0, 1.0],
            r - 0.02,
            0.02,
            24,
        );
        hoop(
            b,
            "metal",
            c,
            [0.0, 1.0, 0.0],
            [0.0, 0.0, 1.0],
            r - 0.045,
            0.008,
            24,
        );
        for k in 0..8 {
            let a = TAU * k as f64 / 8.0;
            b.cyl(
                "metal",
                c,
                [
                    0.0,
                    c[1] + a.cos() * (r - 0.05),
                    c[2] + a.sin() * (r - 0.05),
                ],
                0.0025,
            );
        }
        b.cyl("metal", [-0.04, c[1], c[2]], [0.04, c[1], c[2]], 0.012);
    }
    let tube = |b: &mut Builder, a: P3, e: P3| b.cyl("frame_mat", a, e, 0.016);
    let crank = [0.0, rear[1] + (front[1] - rear[1]) * 0.38, r * 0.85];
    let seat = [0.0, crank[1] - 0.12, r + 0.55];
    let head = [
        0.0,
        front[1] - 0.16,
        r + if style == 2 { 0.5 } else { 0.55 },
    ];
    tube(b, rear, crank);
    tube(b, rear, seat);
    tube(b, crank, seat);
    if style == 1 {
        // The step-through frame of a town bike.
        tube(b, crank, [0.0, head[1], head[2] - 0.2]);
    } else {
        tube(b, crank, [0.0, head[1], head[2] - 0.12]);
        tube(b, seat, [0.0, head[1], head[2] - 0.04]);
    }
    tube(b, [0.0, head[1], head[2] - 0.2], head);
    tube(b, [0.0, head[1], head[2] - 0.2], front);
    b.cyl("metal", seat, [0.0, seat[1] - 0.03, seat[2] + 0.12], 0.012);
    b.blob(
        "dark",
        [0.0, seat[1] - 0.03, seat[2] + 0.15],
        [0.08, 0.0, 0.0],
        [0.0, 0.13, 0.0],
        [0.0, 0.0, 0.03],
    );
    b.cyl("metal", head, [0.0, head[1], head[2] + 0.08], 0.013);
    let bar = head[2] + 0.08;
    if style == 2 {
        b.cyl(
            "metal",
            [-0.2, head[1] + 0.02, bar],
            [0.2, head[1] + 0.02, bar],
            0.012,
        );
        for x in [-0.2, 0.2] {
            b.cyl(
                "dark",
                [x, head[1] + 0.02, bar],
                [x, head[1] + 0.12, bar - 0.12],
                0.013,
            );
        }
    } else {
        b.cyl(
            "metal",
            [-0.28, head[1] - 0.06, bar],
            [0.28, head[1] - 0.06, bar],
            0.012,
        );
        for x in [-0.28, 0.28] {
            b.cyl(
                "dark",
                [x * 0.8, head[1] - 0.06, bar],
                [x, head[1] - 0.06, bar],
                0.016,
            );
        }
    }
    b.cyl(
        "metal",
        [-0.09, crank[1], crank[2]],
        [0.09, crank[1], crank[2]],
        0.01,
    );
    b.cone(
        "metal",
        [-0.04, crank[1], crank[2]],
        [-0.03, crank[1], crank[2]],
        0.09,
        0.09,
        16,
    );
    match style {
        1 => {
            b.rbox(
                "frame",
                [-0.16, head[1] + 0.02, bar - 0.25],
                [0.16, head[1] + 0.32, bar - 0.04],
                0.03,
            );
            b.rbox(
                "metal",
                [-0.1, rear[1] - 0.15, r + 0.28],
                [0.1, rear[1] + 0.12, r + 0.3],
                0.005,
            );
        }
        3 => {
            // The box over the front wheel, ahead of the handlebar.
            b.rbox(
                "frame",
                [-0.28, head[1] + 0.1, 2.0 * r + 0.02],
                [0.28, front[1] + r * 0.9, 2.0 * r + 0.38],
                0.02,
            );
        }
        _ => {}
    }
}
