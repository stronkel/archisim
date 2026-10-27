//! The workshop's lamps, one of each for each collection (paquete 4):
//! pendants, ceiling lights, wall lights, floor lamps and table lamps.
//! What glows at night is the role `shade` (opal glass, paper, linen,
//! rattan) and `bulb` (a bare bulb, a candle's flame).
//!
//! Axes as every piece's: the back on the wall at `y = 0`, the front
//! looking to `+y`, centred on `x = 0`. A pendant or a ceiling light hangs
//! from the ceiling at `ceiling`, centred on `(0, diameter / 2)`; a wall
//! light's plate is on the wall, its middle at `elevation`.
//!
//! Roles: `shade` and `bulb` (they glow), `body_mat` the painted or
//! enamelled body of a lamp (role `body`), `frame` wood, `metal` brass,
//! iron or steel, `ceramic` a base, `weave` rattan that does not glow,
//! `glass` clear glass, `stone` a marble foot, `dark` cords.

use super::bedroom::bedside_lamp;
use super::shape::{curve, sweep, turned, Cap};
use super::sofas::compass_leg;
use super::soft::tube;
use super::storage::tapered_leg;
use crate::build::{cross, norm, Builder, P3};
use crate::Params;
use std::f64::consts::{FRAC_PI_2, PI, TAU};

/// The collection of a piece that only has the collections' models.
fn own(p: &Params) -> usize {
    (p.style() - 1).clamp(0, 7) as usize
}

/// Runs `make` (built about the origin, looking up `+z`) and turns what
/// it made to look along `dir`, then moves it to `at`.
fn aimed(b: &mut Builder, at: P3, dir: P3, make: impl FnOnce(&mut Builder)) {
    let z = norm(dir);
    let helper = if z[2].abs() < 0.9 {
        [0.0, 0.0, 1.0]
    } else {
        [1.0, 0.0, 0.0]
    };
    let x = norm(cross(helper, z));
    let y = cross(z, x);
    let start = b.mark();
    make(b);
    b.transform_from(start, |q| {
        [
            at[0] + x[0] * q[0] + y[0] * q[1] + z[0] * q[2],
            at[1] + x[1] * q[0] + y[1] * q[1] + z[1] * q[2],
            at[2] + x[2] * q[0] + y[2] * q[1] + z[2] * q[2],
        ]
    });
}

/// A circle of `n` points of radius `r` about `c`, level.
fn ring(c: P3, r: f64, n: usize) -> Vec<P3> {
    (0..n)
        .map(|k| {
            let a = TAU * k as f64 / n as f64;
            [c[0] + r * a.cos(), c[1] + r * a.sin(), c[2]]
        })
        .collect()
}

/// A turned solid closed at both ends from its profile `(r, z)`.
fn solid(b: &mut Builder, role: &'static str, c: P3, prof: &[(f64, f64)], sides: usize) {
    let mut v = vec![(0.0, prof[0].1)];
    v.extend_from_slice(prof);
    v.push((0.0, prof[prof.len() - 1].1));
    turned(b, role, c, &v, sides, 0.0);
}

/// A ceiling rose at the ceiling `z` over `(x, y)`.
fn canopy(b: &mut Builder, role: &'static str, x: f64, y: f64, z: f64, r: f64) {
    solid(
        b,
        role,
        [x, y, z - 0.03],
        &[(r * 0.8, 0.0), (r, 0.025), (r, 0.03)],
        24,
    );
}

/// A candle sleeve with its flame bulb, standing at `at`.
fn candle(b: &mut Builder, sleeve: &'static str, at: P3, h: f64) {
    b.cyl(sleeve, at, [at[0], at[1], at[2] + h], 0.012);
    b.ellipsoid(
        "bulb",
        [at[0], at[1], at[2] + h + 0.03],
        [0.011, 0.011, 0.028],
    );
}

// ---------------------------------------------------------- pendants

/// The pendant of each collection (styles 1 to 8): `diameter`, hanging
/// `drop` below the ceiling at `ceiling`.
pub fn pendant(b: &mut Builder, p: &Params) {
    let ix = own(p);
    let d = p.get("diameter", 0.45).clamp(0.2, 1.2);
    let drop = p.get("drop", 0.8).clamp(0.3, 2.0);
    let ceil = p.get("ceiling", 2.6).clamp(2.2, 5.0);
    let r = d / 2.0;
    let (cx, cy) = (0.0, r);
    let z_low = ceil - drop;
    let cord = |b: &mut Builder, z_top_of_lamp: f64| {
        b.cyl(
            "dark",
            [cx, cy, z_top_of_lamp],
            [cx, cy, ceil - 0.025],
            0.004,
        );
    };
    match ix {
        // A lacquered bell with an oak cap, an opal disc below.
        0 => {
            let h = d * 0.62;
            solid(
                b,
                "body",
                [cx, cy, z_low],
                &[
                    (r, 0.0),
                    (r * 0.97, 0.015),
                    (r * 0.7, h * 0.35),
                    (r * 0.32, h * 0.8),
                    (r * 0.18, h),
                ],
                40,
            );
            solid(
                b,
                "frame",
                [cx, cy, z_low + h - 0.005],
                &[(r * 0.2, 0.0), (r * 0.2, 0.06)],
                24,
            );
            solid(
                b,
                "shade",
                [cx, cy, z_low - 0.004],
                &[(r * 0.94, 0.0), (r * 0.94, 0.006)],
                40,
            );
            cord(b, z_low + h + 0.05);
            canopy(b, "body", cx, cy, ceil, 0.06);
        }
        // A wide dome of woven rattan.
        1 => {
            let h = d * 0.4;
            let n = 14;
            let prof: Vec<(f64, f64)> = (0..=n)
                .map(|i| {
                    let t = FRAC_PI_2 * i as f64 / n as f64;
                    (r * t.cos().max(0.08), h * t.sin())
                })
                .collect();
            solid(b, "shade", [cx, cy, z_low], &prof, 40);
            // Its rim, a thicker coil.
            tube(b, "weave", &ring([cx, cy, z_low], r, 40), true, 0.008, 6);
            cord(b, z_low + h);
            canopy(b, "weave", cx, cy, ceil, 0.06);
        }
        // Fifties: three opal globes on brass stems from a brass hub.
        2 => {
            let hub = z_low + d * 0.5;
            solid(
                b,
                "metal",
                [cx, cy, hub - 0.03],
                &[(0.035, 0.0), (0.04, 0.03), (0.02, 0.06)],
                16,
            );
            for k in 0..3 {
                let a = TAU * k as f64 / 3.0 + 0.3;
                let reach = r * 0.75;
                let drop_k = [0.0, 0.12, 0.24][k];
                let end = [
                    cx + reach * a.cos(),
                    cy + reach * a.sin(),
                    hub - 0.12 - drop_k,
                ];
                let pts = curve(
                    &[
                        [cx, cy, hub],
                        [
                            cx + reach * 0.6 * a.cos(),
                            cy + reach * 0.6 * a.sin(),
                            hub + 0.02,
                        ],
                        [end[0], end[1], end[2] + 0.12],
                        end,
                    ],
                    5,
                );
                sweep(b, "metal", &pts, |_| 0.006, 8, Cap::Flat, Cap::Flat);
                b.ellipsoid(
                    "shade",
                    [end[0], end[1], end[2] - 0.08],
                    [0.085, 0.085, 0.085],
                );
                solid(
                    b,
                    "metal",
                    [end[0], end[1], end[2] - 0.015],
                    &[(0.025, 0.0), (0.018, 0.02)],
                    16,
                );
            }
            cord(b, hub + 0.03);
            canopy(b, "metal", cx, cy, ceil, 0.06);
        }
        // A round paper lantern on its wire ribs, a wooden cap.
        3 => {
            let n = 16;
            let prof: Vec<(f64, f64)> = (0..=n)
                .map(|i| {
                    let t = PI * i as f64 / n as f64;
                    (r * t.sin().max(0.12), r - r * t.cos())
                })
                .collect();
            solid(b, "shade", [cx, cy, z_low], &prof, 40);
            for k in 1..8 {
                let z = z_low + 2.0 * r * k as f64 / 8.0;
                let rr = (r * r - (z - z_low - r).powi(2)).max(0.0).sqrt() + 0.0015;
                tube(b, "frame", &ring([cx, cy, z], rr, 40), true, 0.0015, 4);
            }
            solid(
                b,
                "frame",
                [cx, cy, z_low + 2.0 * r - 0.01],
                &[(0.04, 0.0), (0.035, 0.03)],
                20,
            );
            cord(b, z_low + 2.0 * r + 0.02);
            canopy(b, "frame", cx, cy, ceil, 0.05);
        }
        // Contemporary: a glowing ring in a dark band, on three wires.
        4 => {
            let z = z_low + 0.03;
            tube(
                b,
                "body",
                &ring([cx, cy, z + 0.012], r, 64),
                true,
                0.016,
                10,
            );
            tube(
                b,
                "shade",
                &ring([cx, cy, z - 0.006], r, 64),
                true,
                0.011,
                10,
            );
            for k in 0..3 {
                let a = TAU * k as f64 / 3.0;
                let q = [cx + r * a.cos(), cy + r * a.sin(), z + 0.025];
                b.cyl("dark", q, [cx, cy, ceil - 0.02], 0.0015);
            }
            canopy(b, "body", cx, cy, ceil, 0.07);
        }
        // Rustic: a wheel of wrought iron with candles, on chains.
        5 => {
            let z = z_low + 0.08;
            tube(b, "metal", &ring([cx, cy, z], r, 40), true, 0.012, 8);
            tube(
                b,
                "metal",
                &ring([cx, cy, z - 0.02], r * 0.35, 24),
                true,
                0.01,
                8,
            );
            let n = 6;
            for k in 0..n {
                let a = TAU * k as f64 / n as f64;
                let (s, c) = a.sin_cos();
                b.cyl(
                    "metal",
                    [cx + r * 0.35 * c, cy + r * 0.35 * s, z - 0.02],
                    [cx + r * c, cy + r * s, z],
                    0.007,
                );
                let at = [cx + r * c, cy + r * s, z + 0.01];
                solid(
                    b,
                    "metal",
                    [at[0], at[1], at[2] - 0.012],
                    &[(0.03, 0.0), (0.035, 0.012)],
                    16,
                );
                candle(b, "ceramic", at, 0.09);
                // The chain to the hook.
                if k % 2 == 0 {
                    b.cyl(
                        "metal",
                        [cx + r * 0.8 * c, cy + r * 0.8 * s, z],
                        [cx, cy, ceil - 0.25],
                        0.004,
                    );
                }
            }
            b.cyl("metal", [cx, cy, ceil - 0.25], [cx, cy, ceil - 0.02], 0.006);
            solid(
                b,
                "metal",
                [cx, cy, z - 0.1],
                &[(0.02, 0.0), (0.035, 0.05), (0.015, 0.09)],
                16,
            );
            canopy(b, "metal", cx, cy, ceil, 0.06);
        }
        // Industrial: an enamelled dome, a bare bulb under it.
        6 => {
            let h = d * 0.45;
            let n = 12;
            let mut prof: Vec<(f64, f64)> = (0..=n)
                .map(|i| {
                    let t = FRAC_PI_2 * i as f64 / n as f64;
                    (r * t.cos(), h * t.sin())
                })
                .collect();
            prof[n].0 = 0.03;
            prof.insert(0, (r * 1.02, -0.01));
            solid(b, "body", [cx, cy, z_low + 0.04], &prof, 40);
            solid(
                b,
                "metal",
                [cx, cy, z_low + 0.04 + h - 0.01],
                &[(0.03, 0.0), (0.022, 0.06)],
                16,
            );
            b.ellipsoid("bulb", [cx, cy, z_low + 0.06], [0.04, 0.04, 0.055]);
            cord(b, z_low + 0.04 + h + 0.05);
            canopy(b, "body", cx, cy, ceil, 0.06);
        }
        // Classic: a brass chandelier, six curved arms with candles and
        // little shades.
        _ => {
            let hub = z_low + 0.18;
            solid(
                b,
                "metal",
                [cx, cy, z_low],
                &[
                    (0.01, 0.0),
                    (0.04, 0.05),
                    (0.06, 0.14),
                    (0.04, 0.2),
                    (0.025, 0.26),
                    (0.035, 0.3),
                    (0.015, 0.36),
                ],
                24,
            );
            let n = 6;
            for k in 0..n {
                let a = TAU * k as f64 / n as f64;
                let (s, c) = a.sin_cos();
                let pts = curve(
                    &[
                        [cx + 0.05 * c, cy + 0.05 * s, hub],
                        [cx + r * 0.5 * c, cy + r * 0.5 * s, hub - 0.08],
                        [cx + r * 0.85 * c, cy + r * 0.85 * s, hub - 0.02],
                        [cx + r * c, cy + r * s, hub + 0.08],
                    ],
                    6,
                );
                sweep(b, "metal", &pts, |_| 0.008, 8, Cap::Round(1.0), Cap::Flat);
                let at = [cx + r * c, cy + r * s, hub + 0.08];
                solid(
                    b,
                    "metal",
                    at,
                    &[(0.02, 0.0), (0.035, 0.015), (0.03, 0.02)],
                    16,
                );
                candle(b, "ceramic", [at[0], at[1], at[2] + 0.02], 0.07);
                solid(
                    b,
                    "shade",
                    [at[0], at[1], at[2] + 0.07],
                    &[(0.07, 0.0), (0.04, 0.1)],
                    24,
                );
            }
            b.cyl(
                "metal",
                [cx, cy, z_low + 0.36],
                [cx, cy, ceil - 0.02],
                0.006,
            );
            canopy(b, "metal", cx, cy, ceil, 0.07);
        }
    }
}

// ---------------------------------------------------------- ceiling lights

/// The ceiling light of each collection (styles 1 to 8): `diameter`, on
/// the ceiling at `ceiling`.
pub fn ceiling_light(b: &mut Builder, p: &Params) {
    let ix = own(p);
    let d = p.get("diameter", 0.4).clamp(0.2, 1.2);
    let ceil = p.get("ceiling", 2.6).clamp(2.2, 5.0);
    let r = d / 2.0;
    let (cx, cy) = (0.0, r);
    let dome = |b: &mut Builder, role: &'static str, z: f64, h: f64, rr: f64| {
        let n = 12;
        let prof: Vec<(f64, f64)> = (0..=n)
            .map(|i| {
                let t = FRAC_PI_2 * i as f64 / n as f64;
                (rr * t.sin().max(0.0), z - h * t.cos())
            })
            .map(|(a, zz)| (a, zz - z + h))
            .collect();
        // From its low point up to the rim, closed flat at the rim.
        turned(b, role, [cx, cy, z - h], &prof, 40, 0.0);
    };
    match ix {
        0 => {
            dome(b, "shade", ceil - 0.025, r * 0.45, r * 0.95);
            solid(
                b,
                "frame",
                [cx, cy, ceil - 0.03],
                &[(r, 0.0), (r, 0.03)],
                40,
            );
        }
        1 => {
            solid(
                b,
                "shade",
                [cx, cy, ceil - r * 0.6],
                &[(r * 0.9, 0.0), (r, r * 0.1), (r, r * 0.6)],
                40,
            );
            for k in [1, 3] {
                let z = ceil - r * 0.6 + r * 0.15 * k as f64;
                tube(
                    b,
                    "weave",
                    &ring([cx, cy, z], r + 0.002, 40),
                    true,
                    0.004,
                    5,
                );
            }
        }
        2 => {
            dome(b, "shade", ceil - 0.03, r * 0.4, r * 0.9);
            tube(
                b,
                "metal",
                &ring([cx, cy, ceil - 0.03], r * 0.92, 48),
                true,
                0.008,
                6,
            );
            solid(
                b,
                "metal",
                [cx, cy, ceil - 0.03 - r * 0.4 - 0.025],
                &[(0.012, 0.0), (0.018, 0.02), (0.008, 0.03)],
                12,
            );
            solid(
                b,
                "metal",
                [cx, cy, ceil - 0.03],
                &[(r * 0.95, 0.0), (r * 0.95, 0.03)],
                40,
            );
        }
        3 => {
            let s = r * 0.95;
            b.rbox(
                "shade",
                [cx - s, cy - s, ceil - 0.12],
                [cx + s, cy + s, ceil - 0.01],
                0.0,
            );
            for k in 0..=4 {
                let t = -s + 2.0 * s * k as f64 / 4.0;
                b.rbox(
                    "frame",
                    [cx + t - 0.006, cy - s - 0.004, ceil - 0.125],
                    [cx + t + 0.006, cy + s + 0.004, ceil - 0.11],
                    0.0,
                );
                b.rbox(
                    "frame",
                    [cx - s - 0.004, cy + t - 0.006, ceil - 0.125],
                    [cx + s + 0.004, cy + t + 0.006, ceil - 0.11],
                    0.0,
                );
            }
            b.rbox(
                "frame",
                [cx - s - 0.01, cy - s - 0.01, ceil - 0.02],
                [cx + s + 0.01, cy + s + 0.01, ceil],
                0.0,
            );
        }
        4 => {
            solid(
                b,
                "body",
                [cx, cy, ceil - 0.05],
                &[(r * 0.98, 0.0), (r, 0.005), (r, 0.05)],
                64,
            );
            solid(
                b,
                "shade",
                [cx, cy, ceil - 0.054],
                &[(r * 0.94, 0.0), (r * 0.94, 0.005)],
                64,
            );
        }
        5 => {
            dome(b, "shade", ceil - 0.06, r * 0.5, r * 0.85);
            for k in 0..4 {
                let a = TAU * k as f64 / 4.0 + PI / 4.0;
                let pts: Vec<P3> = (0..=8)
                    .map(|i| {
                        let t = FRAC_PI_2 * i as f64 / 8.0;
                        let rr = r * 0.87 * t.sin();
                        [
                            cx + rr * a.cos(),
                            cy + rr * a.sin(),
                            ceil - 0.06 - r * 0.52 * t.cos(),
                        ]
                    })
                    .collect();
                tube(b, "metal", &pts, false, 0.006, 6);
            }
            tube(
                b,
                "metal",
                &ring([cx, cy, ceil - 0.06], r * 0.87, 40),
                true,
                0.01,
                6,
            );
            solid(
                b,
                "metal",
                [cx, cy, ceil - 0.06],
                &[(r * 0.6, 0.0), (r * 0.65, 0.06)],
                32,
            );
            solid(
                b,
                "metal",
                [cx, cy, ceil - 0.06 - r * 0.52 - 0.04],
                &[(0.01, 0.0), (0.02, 0.025), (0.012, 0.045)],
                12,
            );
        }
        6 => {
            dome(b, "shade", ceil - 0.05, r * 0.55, r * 0.75);
            solid(
                b,
                "metal",
                [cx, cy, ceil - 0.05],
                &[(r * 0.9, 0.0), (r * 0.9, 0.05)],
                40,
            );
            for k in 0..6 {
                let a = TAU * k as f64 / 6.0;
                let pts: Vec<P3> = (0..=8)
                    .map(|i| {
                        let t = FRAC_PI_2 * i as f64 / 8.0;
                        let rr = r * 0.8 * t.sin();
                        [
                            cx + rr * a.cos(),
                            cy + rr * a.sin(),
                            ceil - 0.05 - r * 0.6 * t.cos(),
                        ]
                    })
                    .collect();
                tube(b, "metal", &pts, false, 0.005, 5);
            }
            tube(
                b,
                "metal",
                &ring([cx, cy, ceil - 0.05 - r * 0.3], r * 0.7, 32),
                true,
                0.005,
                5,
            );
        }
        _ => {
            let z = ceil - 0.42;
            solid(
                b,
                "shade",
                [cx, cy, z - r * 0.35],
                &[
                    (0.03, 0.0),
                    (r * 0.6, r * 0.1),
                    (r * 0.9, r * 0.27),
                    (r, r * 0.35),
                ],
                48,
            );
            tube(b, "metal", &ring([cx, cy, z], r, 48), true, 0.008, 6);
            for k in 0..3 {
                let a = TAU * k as f64 / 3.0;
                b.cyl(
                    "metal",
                    [cx + r * a.cos(), cy + r * a.sin(), z],
                    [cx, cy, ceil - 0.06],
                    0.004,
                );
            }
            solid(
                b,
                "metal",
                [cx, cy, z - r * 0.35 - 0.05],
                &[(0.01, 0.0), (0.025, 0.03), (0.012, 0.05)],
                12,
            );
            solid(
                b,
                "metal",
                [cx, cy, ceil - 0.06],
                &[(0.04, 0.0), (0.07, 0.04), (0.07, 0.06)],
                24,
            );
        }
    }
}

// ---------------------------------------------------------- wall lights

/// The wall light of each collection (styles 1 to 8): its plate on the
/// wall, its middle at `elevation`.
pub fn wall_light(b: &mut Builder, p: &Params) {
    let ix = own(p);
    let z = p.get("elevation", 1.7).clamp(0.3, 3.0);
    let plate = |b: &mut Builder, role: &'static str, r: f64| {
        aimed(b, [0.0, 0.0, z], [0.0, 1.0, 0.0], |b| {
            solid(b, role, [0.0; 3], &[(r, 0.0), (r * 0.9, 0.02)], 24);
        });
    };
    match ix {
        // An oak arm and a linen drum.
        0 => {
            plate(b, "frame", 0.05);
            b.rbox(
                "frame",
                [-0.012, 0.0, z - 0.012],
                [0.012, 0.28, z + 0.012],
                0.004,
            );
            b.cyl("metal", [0.0, 0.27, z], [0.0, 0.27, z + 0.08], 0.006);
            solid(
                b,
                "shade",
                [0.0, 0.27, z + 0.03],
                &[(0.11, 0.0), (0.11, 0.15)],
                32,
            );
        }
        // A half dome of rattan over the bulb.
        1 => {
            let r = 0.15;
            let start = b.mark();
            let n = 10;
            let prof: Vec<(f64, f64)> = (0..=n)
                .map(|i| {
                    let t = FRAC_PI_2 * i as f64 / n as f64;
                    (r * t.cos().max(0.06), 0.3 * t.sin())
                })
                .collect();
            solid(b, "shade", [0.0, 0.0, 0.0], &prof, 32);
            // Cut in half: only the half before the wall is kept, squashed
            // against it.
            b.transform_from(start, |q| {
                [q[0], q[1].max(0.0) * 0.9 + 0.012, z - 0.15 + q[2]]
            });
            b.ellipsoid("bulb", [0.0, 0.06, z - 0.08], [0.03, 0.03, 0.045]);
        }
        // Fifties: brass cones up and down on a little arm.
        2 => {
            plate(b, "metal", 0.04);
            b.cyl("metal", [0.0, 0.0, z], [0.0, 0.12, z], 0.008);
            for up in [1.0, -1.0] {
                aimed(b, [0.0, 0.12, z + up * 0.02], [0.0, 0.0, up], |b| {
                    solid(b, "metal", [0.0; 3], &[(0.02, 0.0), (0.08, 0.13)], 32);
                    solid(
                        b,
                        "shade",
                        [0.0, 0.0, 0.13],
                        &[(0.075, 0.0), (0.075, 0.004)],
                        32,
                    );
                });
            }
        }
        // Japandi: a box of paper on an oak back.
        3 => {
            b.rbox("frame", [-0.09, 0.0, z - 0.17], [0.09, 0.02, z + 0.17], 0.0);
            b.rbox(
                "shade",
                [-0.075, 0.02, z - 0.15],
                [0.075, 0.1, z + 0.15],
                0.0,
            );
            for k in 0..4 {
                let zz = z - 0.15 + 0.3 * k as f64 / 3.0;
                b.rbox(
                    "frame",
                    [-0.078, 0.018, zz - 0.004],
                    [0.078, 0.103, zz + 0.004],
                    0.0,
                );
            }
        }
        // Contemporary: a dark disc standing off the wall, its halo
        // glowing behind.
        4 => {
            aimed(b, [0.0, 0.0, z], [0.0, 1.0, 0.0], |b| {
                solid(
                    b,
                    "shade",
                    [0.0, 0.0, 0.004],
                    &[(0.12, 0.0), (0.12, 0.012)],
                    48,
                );
                solid(
                    b,
                    "body",
                    [0.0, 0.0, 0.03],
                    &[(0.14, 0.0), (0.14, 0.012)],
                    48,
                );
                b.cyl("body", [0.0; 3], [0.0, 0.0, 0.03], 0.02);
            });
        }
        // Rustic: an iron lantern on its bracket, glass panes, a candle.
        5 => {
            plate(b, "metal", 0.04);
            let (lx, ly) = (0.0, 0.2);
            let pts = curve(
                &[
                    [0.0, 0.0, z + 0.06],
                    [0.0, 0.1, z + 0.12],
                    [lx, ly, z + 0.16],
                ],
                6,
            );
            tube(b, "metal", &pts, false, 0.008, 6);
            let s = 0.07;
            let (z0, z1) = (z - 0.12, z + 0.1);
            for (dx, dy) in [(-1.0, -1.0), (1.0, -1.0), (1.0, 1.0), (-1.0, 1.0)] {
                b.rbox(
                    "metal",
                    [lx + dx * s - 0.005, ly + dy * s - 0.005, z0],
                    [lx + dx * s + 0.005, ly + dy * s + 0.005, z1],
                    0.0,
                );
            }
            b.rbox(
                "glass",
                [lx - s, ly - s, z0 + 0.01],
                [lx + s, ly + s, z1 - 0.01],
                0.0,
            );
            solid(
                b,
                "metal",
                [lx, ly, z0 - 0.02],
                &[(s + 0.01, 0.0), (s + 0.01, 0.02)],
                4,
            );
            solid(b, "metal", [lx, ly, z1], &[(s * 1.5, 0.0), (0.02, 0.06)], 4);
            candle(b, "ceramic", [lx, ly, z0], 0.08);
        }
        // Industrial: a pipe arm, a cage round a bare bulb.
        6 => {
            plate(b, "metal", 0.045);
            b.cyl("metal", [0.0, 0.0, z], [0.0, 0.18, z], 0.012);
            b.cyl("metal", [0.0, 0.18, z], [0.0, 0.18, z - 0.06], 0.012);
            let c = [0.0, 0.18, z - 0.14];
            b.ellipsoid("bulb", c, [0.04, 0.04, 0.055]);
            for k in 0..6 {
                let a = TAU * k as f64 / 6.0;
                let pts: Vec<P3> = (0..=8)
                    .map(|i| {
                        let t = PI * i as f64 / 8.0;
                        let rr = 0.065 * t.sin() + 0.015;
                        [
                            c[0] + rr * a.cos(),
                            c[1] + rr * a.sin(),
                            c[2] + 0.08 - 0.16 * t / PI,
                        ]
                    })
                    .collect();
                tube(b, "metal", &pts, false, 0.0022, 5);
            }
        }
        // Classic: a brass back plate, two curved arms with candles and
        // little shades.
        _ => {
            b.rbox(
                "metal",
                [-0.04, 0.0, z - 0.12],
                [0.04, 0.02, z + 0.12],
                0.008,
            );
            for s in [-1.0, 1.0] {
                let pts = curve(
                    &[
                        [0.0, 0.02, z - 0.05],
                        [s * 0.06, 0.1, z - 0.1],
                        [s * 0.14, 0.15, z - 0.02],
                        [s * 0.16, 0.16, z + 0.04],
                    ],
                    6,
                );
                sweep(b, "metal", &pts, |_| 0.007, 8, Cap::Flat, Cap::Flat);
                let at = [s * 0.16, 0.16, z + 0.04];
                solid(b, "metal", at, &[(0.02, 0.0), (0.03, 0.015)], 16);
                candle(b, "ceramic", [at[0], at[1], at[2] + 0.015], 0.07);
                solid(
                    b,
                    "shade",
                    [at[0], at[1], at[2] + 0.06],
                    &[(0.065, 0.0), (0.04, 0.09)],
                    24,
                );
            }
        }
    }
}

// ---------------------------------------------------------- floor lamps

/// The floor lamp of each collection (styles 1 to 8): `height`, its shade
/// `diameter` across, centred on `(0, diameter / 2)`.
pub fn floor_lamp(b: &mut Builder, p: &Params) {
    let ix = own(p);
    let h = p.get("height", 1.6).clamp(1.0, 2.2);
    let d = p.get("diameter", 0.45).clamp(0.2, 0.9);
    let r = d / 2.0;
    let (cx, cy) = (0.0, r.max(0.2));
    match ix {
        // Oak tripod, a linen drum.
        0 => {
            let top = h - 0.25;
            for k in 0..3 {
                let a = PI / 2.0 + TAU * k as f64 / 3.0;
                tapered_leg(b, "frame", [cx, cy], top, [a.cos(), a.sin()], 0.28, 0.016);
            }
            b.cyl("metal", [cx, cy, top], [cx, cy, h - 0.1], 0.008);
            solid(
                b,
                "shade",
                [cx, cy, h - 0.3],
                &[(r, 0.0), (r * 0.95, 0.3)],
                40,
            );
        }
        // A turned whitewashed column, a tall rattan cone.
        1 => {
            solid(
                b,
                "frame",
                [cx, cy, 0.0],
                &[(0.14, 0.0), (0.14, 0.03), (0.06, 0.06), (0.03, 0.1)],
                24,
            );
            turned(
                b,
                "frame",
                [cx, cy, 0.08],
                &[
                    (0.0, 0.0),
                    (0.025, 0.0),
                    (0.02, h * 0.3),
                    (0.03, h * 0.35),
                    (0.018, h * 0.6),
                    (0.0, h * 0.6),
                ],
                16,
                0.0,
            );
            b.cyl("metal", [cx, cy, 0.08 + h * 0.6], [cx, cy, h - 0.2], 0.007);
            solid(
                b,
                "shade",
                [cx, cy, h - 0.42],
                &[(r, 0.0), (r * 0.55, 0.42)],
                40,
            );
        }
        // Fifties: a brass stem with three cones turned three ways.
        2 => {
            for k in 0..3 {
                let a = PI / 2.0 + TAU * k as f64 / 3.0;
                compass_leg(b, [cx, cy], 0.4, [a.cos(), a.sin()], 0.22);
            }
            b.cyl("metal", [cx, cy, 0.38], [cx, cy, h - 0.2], 0.009);
            for (k, (zz, tilt)) in [(h - 0.2, 0.9), (h - 0.42, -0.5), (h - 0.65, 0.3)]
                .iter()
                .enumerate()
            {
                let a = TAU * k as f64 / 3.0 + 0.4;
                let arm = [cx + 0.18 * a.cos(), cy + 0.18 * a.sin(), *zz];
                b.cyl("metal", [cx, cy, *zz], arm, 0.006);
                let dir = [a.cos() * 0.6, a.sin() * 0.6, *tilt];
                aimed(b, arm, dir, |b| {
                    solid(b, "body", [0.0; 3], &[(0.02, 0.0), (0.09, 0.16)], 32);
                    solid(
                        b,
                        "shade",
                        [0.0, 0.0, 0.16],
                        &[(0.085, 0.0), (0.085, 0.004)],
                        32,
                    );
                });
            }
        }
        // Japandi: a tall paper column in a slim oak frame.
        3 => {
            let s = 0.12;
            for (dx, dy) in [(-1.0, -1.0), (1.0, -1.0), (1.0, 1.0), (-1.0, 1.0)] {
                b.rbox(
                    "frame",
                    [cx + dx * s - 0.01, cy + dy * s - 0.01, 0.0],
                    [cx + dx * s + 0.01, cy + dy * s + 0.01, h],
                    0.0,
                );
            }
            b.rbox(
                "shade",
                [cx - s + 0.01, cy - s + 0.01, 0.15],
                [cx + s - 0.01, cy + s - 0.01, h - 0.03],
                0.0,
            );
            for zz in [0.15, h - 0.03] {
                b.rbox(
                    "frame",
                    [cx - s - 0.01, cy - s - 0.01, zz - 0.015],
                    [cx + s + 0.01, cy + s + 0.01, zz],
                    0.0,
                );
            }
        }
        // Contemporary: an arc of steel from a marble foot, a dome at its
        // end.
        4 => {
            solid(
                b,
                "stone",
                [cx - 0.5, cy, 0.0],
                &[(0.16, 0.0), (0.16, 0.08)],
                32,
            );
            let pts = curve(
                &[
                    [cx - 0.5, cy, 0.08],
                    [cx - 0.48, cy, h * 0.6],
                    [cx - 0.3, cy, h + 0.2],
                    [cx + 0.05, cy, h + 0.05],
                    [cx + 0.15, cy, h - 0.35 + r * 0.6],
                ],
                8,
            );
            sweep(b, "metal", &pts, |_| 0.011, 10, Cap::Flat, Cap::Flat);
            let n = 10;
            let prof: Vec<(f64, f64)> = (0..=n)
                .map(|i| {
                    let t = FRAC_PI_2 * i as f64 / n as f64;
                    (r * t.cos().max(0.05), r * 0.6 * t.sin())
                })
                .collect();
            solid(b, "body", [cx + 0.15, cy, h - 0.35], &prof, 40);
            solid(
                b,
                "shade",
                [cx + 0.15, cy, h - 0.356],
                &[(r * 0.92, 0.0), (r * 0.92, 0.006)],
                40,
            );
        }
        // Rustic: a wrought iron candlestick, an empire shade.
        5 => {
            for k in 0..3 {
                let a = PI / 2.0 + TAU * k as f64 / 3.0;
                let pts: Vec<P3> = (0..=6)
                    .map(|i| {
                        let t = i as f64 / 6.0;
                        [
                            cx + 0.2 * t * a.cos(),
                            cy + 0.2 * t * a.sin(),
                            0.18 * (1.0 - t).powi(2) + 0.012,
                        ]
                    })
                    .collect();
                tube(b, "metal", &pts, false, 0.011, 6);
            }
            turned(
                b,
                "metal",
                [cx, cy, 0.17],
                &[
                    (0.0, 0.0),
                    (0.035, 0.0),
                    (0.012, 0.05),
                    (0.012, h - 0.6),
                    (0.04, h - 0.55),
                    (0.014, h - 0.5),
                    (0.0, h - 0.5),
                ],
                12,
                0.0,
            );
            b.cyl("metal", [cx, cy, h - 0.35], [cx, cy, h - 0.12], 0.006);
            solid(
                b,
                "shade",
                [cx, cy, h - 0.36],
                &[(r, 0.0), (r * 0.6, 0.3)],
                40,
            );
        }
        // Industrial: a steel tripod, a spot on top.
        6 => {
            let top = h - 0.3;
            for k in 0..3 {
                let a = PI / 2.0 + TAU * k as f64 / 3.0;
                b.cyl(
                    "metal",
                    [cx + 0.35 * a.cos(), cy + 0.35 * a.sin(), 0.014],
                    [cx, cy, top],
                    0.012,
                );
            }
            b.cyl("metal", [cx, cy, top], [cx, cy, h - 0.12], 0.012);
            aimed(b, [cx, cy, h - 0.1], [0.0, 0.7, -0.4], |b| {
                solid(b, "body", [0.0; 3], &[(0.06, -0.08), (0.12, 0.12)], 32);
                solid(
                    b,
                    "shade",
                    [0.0, 0.0, 0.12],
                    &[(0.115, 0.0), (0.115, 0.004)],
                    32,
                );
            });
        }
        // Classic: a turned brass column, a pleated drum.
        _ => {
            solid(
                b,
                "metal",
                [cx, cy, 0.0],
                &[(0.15, 0.0), (0.15, 0.02), (0.1, 0.04), (0.05, 0.07)],
                32,
            );
            turned(
                b,
                "metal",
                [cx, cy, 0.06],
                &[
                    (0.0, 0.0),
                    (0.035, 0.0),
                    (0.02, 0.05),
                    (0.016, h * 0.4),
                    (0.03, h * 0.43),
                    (0.016, h * 0.46),
                    (0.016, h - 0.42),
                    (0.0, h - 0.42),
                ],
                24,
                0.0,
            );
            b.cyl("metal", [cx, cy, h - 0.4], [cx, cy, h - 0.12], 0.007);
            solid(
                b,
                "shade",
                [cx, cy, h - 0.36],
                &[(r, 0.0), (r * 0.82, 0.32)],
                48,
            );
        }
    }
}

// ---------------------------------------------------------- table lamps

/// The table lamp of each collection (styles 1 to 8): its bedside lamp,
/// `height` high (from about 45 cm).
pub fn table_lamp(b: &mut Builder, p: &Params) {
    let ix = own(p);
    let h = p.get("height", 0.55).clamp(0.25, 1.0);
    let k = h / 0.47;
    let start = b.mark();
    bedside_lamp(b, ix, 0.0, 0.0, 0.0);
    b.transform_from(start, |q| [q[0] * k, 0.18 * k + q[1] * k, q[2] * k]);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fine::bedroom::COLLECTIONS;
    use crate::fine::shape::signed_volume;

    #[test]
    fn the_lamps_of_the_eight_collections() {
        type Make = fn(&mut Builder, &Params);
        let pieces: [(&str, Make); 5] = [
            ("colgante", pendant),
            ("plafón", ceiling_light),
            ("aplique", wall_light),
            ("de pie", floor_lamp),
            ("de mesa", table_lamp),
        ];
        for (c, col) in COLLECTIONS.iter().enumerate() {
            for (name, make) in pieces {
                let mut p = Params::default();
                p.set("style", (c + 1) as f64);
                let mut b = Builder::default();
                make(&mut b, &p);
                let tris = b.triangle_count();
                assert!((50..30_000).contains(&tris), "{col} {name}: {tris}");
                assert!(
                    b.parts.contains_key("shade") || b.parts.contains_key("bulb"),
                    "{col} {name}: nothing glows"
                );
                for (role, t) in &b.parts {
                    let v = signed_volume(&b.positions, t);
                    assert!(v > -1e-7, "{col} {name} {role}: {v}");
                }
                for q in &b.positions {
                    assert!(q.iter().all(|v| v.is_finite()), "{col} {name}");
                    assert!(q[2] > -0.003 && q[2] < 2.61, "{col} {name}: {q:?}");
                }
            }
        }
    }
}
