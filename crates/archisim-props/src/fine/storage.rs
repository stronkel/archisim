//! The workshop's storage, one piece for each collection (paquete 1c):
//! sideboards, bookcases, television units and glass cabinets.
//!
//! Axes as every piece of furniture's: the back on the wall at `y = 0`,
//! the front looking to `+y`, centred on `x = 0`. `width`, `depth`,
//! `height` (of the carcass; the television stands on top of its unit).
//!
//! Roles: `frame` the carcass, `front` doors and drawers, `top` a top of
//! another material, `stone` a stone top, `metal` steel, brass or iron,
//! `glass`, `weave` cane or rush, `dark` shadow gaps and leather,
//! `books`, `books2`, `books3` the books, `ceramic` vases and bowls,
//! `screen` a television's glass.

use super::shape::{bar, sweep, turned, Cap};
use super::sofas::compass_leg;
use crate::build::{Builder, P3};
use crate::Params;
use std::f64::consts::PI;

/// How thick a door or a drawer front is.
pub(super) const T: f64 = 0.02;

/// A small repeatable random sequence, for books and their gaps.
struct Rng(u32);

impl Rng {
    fn next(&mut self) -> f64 {
        self.0 = self.0.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        (self.0 >> 8) as f64 / (1u32 << 24) as f64
    }
    fn range(&mut self, a: f64, b: f64) -> f64 {
        a + (b - a) * self.next()
    }
}

/// The sizes asked for, with the piece's defaults.
fn dims(p: &Params, w: f64, d: f64, h: f64) -> (f64, f64, f64) {
    (
        p.get("width", w).clamp(0.4, 4.0),
        p.get("depth", d).clamp(0.25, 0.8),
        p.get("height", h).clamp(0.3, 2.8),
    )
}

/// How a door or a drawer front is made.
#[derive(Clone, Copy, PartialEq)]
pub(super) enum Panel {
    /// A plain board.
    Flat,
    /// A frame round a sunk panel.
    Shaker,
    /// A frame round a raised and bevelled panel, with a moulding.
    Raised,
    /// A frame round a panel of cane.
    Cane,
    /// A frame round glass.
    Glass,
    /// Glass in a frame divided by bars: columns and rows.
    Grid(usize, usize),
    /// Square slats standing out of a board.
    Ribs,
    /// Half-round flutes standing out of a board.
    Flutes,
    /// Upright planks with their joints.
    Planks,
    /// A steel frame round a wire mesh.
    Mesh,
    /// A plain board with a moulding set in round its face (a light
    /// panelled drawer).
    Moulded,
}

/// A door or drawer front from `x0` to `x1` and `z0` to `z1` on a face at
/// `y`, `role` its wood.
#[allow(clippy::too_many_arguments)]
pub(super) fn front(
    b: &mut Builder,
    panel: Panel,
    role: &'static str,
    x0: f64,
    x1: f64,
    z0: f64,
    z1: f64,
    y: f64,
) {
    let (w, h) = (x1 - x0, z1 - z0);
    let rail = 0.06_f64.min(w * 0.18).min(h * 0.18).max(0.018);
    let frame = |b: &mut Builder, role: &'static str, depth: f64| {
        for (lo, hi) in [
            ([x0, y, z0], [x1, y + depth, z0 + rail]),
            ([x0, y, z1 - rail], [x1, y + depth, z1]),
            ([x0, y, z0 + rail], [x0 + rail, y + depth, z1 - rail]),
            ([x1 - rail, y, z0 + rail], [x1, y + depth, z1 - rail]),
        ] {
            b.rbox(role, lo, hi, 0.002);
        }
    };
    let (ix0, ix1, iz0, iz1) = (x0 + rail, x1 - rail, z0 + rail, z1 - rail);
    match panel {
        Panel::Flat => b.rbox(role, [x0, y, z0], [x1, y + T, z1], 0.003),
        Panel::Moulded => {
            b.rbox(role, [x0, y, z0], [x1, y + T, z1], 0.003);
            let k = (0.035_f64).min(w * 0.15).min(h * 0.22);
            let ring = [
                [x0 + k, y + T, z0 + k],
                [x1 - k, y + T, z0 + k],
                [x1 - k, y + T, z1 - k],
                [x0 + k, y + T, z1 - k],
            ];
            super::soft::tube(b, role, &ring, true, 0.004, 6);
        }
        Panel::Shaker => {
            frame(b, role, T);
            b.rbox(role, [ix0, y, iz0], [ix1, y + T * 0.45, iz1], 0.0);
        }
        Panel::Raised => {
            frame(b, role, T);
            b.rbox(role, [ix0, y, iz0], [ix1, y + T * 0.5, iz1], 0.0);
            let k = (0.03_f64).min((ix1 - ix0) * 0.2).min((iz1 - iz0) * 0.2);
            b.rbox(
                role,
                [ix0 + k * 0.5, y, iz0 + k * 0.5],
                [ix1 - k * 0.5, y + T * 0.75, iz1 - k * 0.5],
                0.006,
            );
            b.rbox(
                role,
                [ix0 + k, y, iz0 + k],
                [ix1 - k, y + T * 0.95, iz1 - k],
                0.004,
            );
            // The moulding round the inside of the frame.
            let ring = [
                [ix0, y + T, iz0],
                [ix1, y + T, iz0],
                [ix1, y + T, iz1],
                [ix0, y + T, iz1],
            ];
            super::soft::tube(b, role, &ring, true, 0.0045, 6);
        }
        Panel::Cane => {
            frame(b, role, T);
            b.rbox(
                "weave",
                [ix0, y + T * 0.3, iz0],
                [ix1, y + T * 0.5, iz1],
                0.0,
            );
        }
        Panel::Glass => {
            frame(b, role, T);
            b.rbox(
                "glass",
                [ix0, y + T * 0.4, iz0],
                [ix1, y + T * 0.6, iz1],
                0.0,
            );
        }
        Panel::Grid(cols, rows) => {
            frame(b, role, T);
            b.rbox(
                "glass",
                [ix0, y + T * 0.35, iz0],
                [ix1, y + T * 0.5, iz1],
                0.0,
            );
            let m = 0.012;
            for i in 1..cols.max(1) {
                let x = ix0 + (ix1 - ix0) * i as f64 / cols as f64;
                b.rbox(
                    role,
                    [x - m / 2.0, y + T * 0.2, iz0],
                    [x + m / 2.0, y + T * 0.85, iz1],
                    0.0,
                );
            }
            for j in 1..rows.max(1) {
                let z = iz0 + (iz1 - iz0) * j as f64 / rows as f64;
                b.rbox(
                    role,
                    [ix0, y + T * 0.2, z - m / 2.0],
                    [ix1, y + T * 0.85, z + m / 2.0],
                    0.0,
                );
            }
        }
        Panel::Ribs => {
            b.rbox(role, [x0, y, z0], [x1, y + T * 0.55, z1], 0.002);
            let n = ((w / 0.032).round() as usize).max(3);
            let step = w / n as f64;
            for i in 0..n {
                let x = x0 + step * (i as f64 + 0.5);
                b.rbox(
                    role,
                    [x - step * 0.36, y + T * 0.5, z0],
                    [x + step * 0.36, y + T, z1],
                    0.0,
                );
            }
        }
        Panel::Flutes => {
            b.rbox(role, [x0, y, z0], [x1, y + T * 0.5, z1], 0.002);
            let n = ((w / 0.026).round() as usize).max(3);
            let step = w / n as f64;
            for i in 0..n {
                let x = x0 + step * (i as f64 + 0.5);
                b.cone(
                    role,
                    [x, y + T * 0.5, z0 + 0.004],
                    [x, y + T * 0.5, z1 - 0.004],
                    step * 0.48,
                    step * 0.48,
                    10,
                );
            }
        }
        Panel::Planks => {
            let n = ((w / 0.11).round() as usize).max(2);
            let step = w / n as f64;
            for i in 0..n {
                let xa = x0 + step * i as f64;
                b.rbox(
                    role,
                    [xa + 0.0015, y, z0],
                    [xa + step - 0.0015, y + T, z1],
                    0.003,
                );
            }
            b.rbox(
                "dark",
                [x0 + 0.004, y, z0 + 0.004],
                [x1 - 0.004, y + T * 0.3, z1 - 0.004],
                0.0,
            );
        }
        Panel::Mesh => {
            frame(b, "metal", T * 0.8);
            let s = 0.035;
            let (nx, nz) = (
                ((ix1 - ix0) / s).floor() as usize,
                ((iz1 - iz0) / s).floor() as usize,
            );
            for i in 1..=nx {
                let x = ix0 + (ix1 - ix0) * i as f64 / (nx + 1) as f64;
                b.rbox(
                    "metal",
                    [x - 0.0012, y + T * 0.35, iz0],
                    [x + 0.0012, y + T * 0.45, iz1],
                    0.0,
                );
            }
            for j in 1..=nz {
                let z = iz0 + (iz1 - iz0) * j as f64 / (nz + 1) as f64;
                b.rbox(
                    "metal",
                    [ix0, y + T * 0.45, z - 0.0012],
                    [ix1, y + T * 0.55, z + 0.0012],
                    0.0,
                );
            }
        }
    }
}

/// The pulls a front can have.
#[derive(Clone, Copy, PartialEq)]
pub(super) enum Pull {
    None,
    /// A turned wooden knob.
    Knob,
    /// A small brass knob.
    Brass,
    /// A ring hanging from a round rosette.
    Ring,
    /// An iron cup pull.
    Cup,
    /// A bar on two posts, `len` long.
    Bar(f64),
    /// A loop of leather.
    Tab,
    /// A finger cut in the top edge.
    Slot,
}

/// A pull centred at `(x, z)` on a front whose face is at `y`.
pub(super) fn pull(b: &mut Builder, kind: Pull, x: f64, z: f64, y: f64) {
    // Something turned round an axis looking out of the front.
    let out = |b: &mut Builder, role: &'static str, prof: &[(f64, f64)]| {
        let start = b.mark();
        turned(b, role, [0.0; 3], prof, 14, 0.0);
        // Turned on its side (a rotation, not a mirror: it stays closed
        // outwards).
        b.transform_from(start, |q| [x + q[0], y + q[2], z - q[1]]);
    };
    match kind {
        Pull::None => {}
        Pull::Knob => out(
            b,
            "front",
            &[
                (0.0, 0.0),
                (0.009, 0.0),
                (0.006, 0.01),
                (0.014, 0.022),
                (0.012, 0.03),
                (0.0, 0.032),
            ],
        ),
        Pull::Brass => out(
            b,
            "metal",
            &[
                (0.0, 0.0),
                (0.008, 0.0),
                (0.004, 0.008),
                (0.011, 0.018),
                (0.0, 0.024),
            ],
        ),
        Pull::Ring => {
            out(
                b,
                "metal",
                &[
                    (0.0, 0.0),
                    (0.018, 0.0),
                    (0.014, 0.006),
                    (0.006, 0.012),
                    (0.0, 0.014),
                ],
            );
            let ring: Vec<P3> = (0..18)
                .map(|k| {
                    let a = 2.0 * PI * k as f64 / 18.0;
                    [x + 0.022 * a.sin(), y + 0.012, z - 0.022 + 0.022 * a.cos()]
                })
                .collect();
            super::soft::tube(b, "metal", &ring, true, 0.0035, 6);
        }
        Pull::Cup => {
            b.rbox(
                "metal",
                [x - 0.05, y, z - 0.012],
                [x + 0.05, y + 0.003, z + 0.02],
                0.002,
            );
            b.ellipsoid("metal", [x, y + 0.004, z + 0.004], [0.04, 0.014, 0.018]);
            b.rbox(
                "dark",
                [x - 0.034, y + 0.012, z - 0.012],
                [x + 0.034, y + 0.016, z - 0.004],
                0.0,
            );
        }
        Pull::Bar(len) => {
            b.cyl(
                "metal",
                [x - len / 2.0, y + 0.028, z],
                [x + len / 2.0, y + 0.028, z],
                0.006,
            );
            // Its posts set into the front (a mesh or cane is a little
            // behind the frame's face).
            for px in [x - len / 2.0 + 0.012, x + len / 2.0 - 0.012] {
                b.cyl("metal", [px, y - 0.008, z], [px, y + 0.028, z], 0.0045);
            }
        }
        Pull::Tab => {
            b.rbox(
                "dark",
                [x - 0.014, y - 0.002, z - 0.03],
                [x + 0.014, y + 0.004, z + 0.012],
                0.002,
            );
            b.cyl(
                "metal",
                [x, y, z + 0.004],
                [x, y + 0.007, z + 0.004],
                0.0035,
            );
        }
        Pull::Slot => {
            b.rbox(
                "dark",
                [x - 0.05, y - T * 0.6, z - 0.012],
                [x + 0.05, y + 0.001, z + 0.001],
                0.006,
            );
        }
    }
}

/// `n` doors side by side from `x0` to `x1`, pulls at the edges where two
/// meet (or near the right of a single one), at `pz`.
#[allow(clippy::too_many_arguments)]
pub(super) fn doors(
    b: &mut Builder,
    panel: Panel,
    role: &'static str,
    x0: f64,
    x1: f64,
    z0: f64,
    z1: f64,
    y: f64,
    n: usize,
    kind: Pull,
    pz: f64,
) {
    let n = n.max(1);
    let gap = 0.003;
    let step = (x1 - x0) / n as f64;
    for i in 0..n {
        let (a, e) = (
            x0 + step * i as f64 + gap / 2.0,
            x0 + step * (i + 1) as f64 - gap / 2.0,
        );
        front(b, panel, role, a, e, z0, z1, y);
        // Paired doors meet in the middle; the rest open from the right.
        let towards_right = if n.is_multiple_of(2) {
            i < n / 2
        } else {
            i.is_multiple_of(2)
        };
        let inset = match kind {
            Pull::Bar(_) | Pull::Cup => 0.03,
            Pull::Slot => 0.06,
            _ => 0.04,
        };
        let px = if towards_right { e - inset } else { a + inset };
        let pz = if kind == Pull::Slot { z1 - 0.001 } else { pz };
        if let Pull::Bar(l) = kind {
            // Upright on a door: made across, then turned on its end.
            let l = l.min((z1 - z0) * 0.6);
            let start = b.mark();
            pull(b, Pull::Bar(l), 0.0, 0.0, 0.0);
            b.transform_from(start, |q| [px - q[2], y + T + q[1], pz + q[0]]);
        } else {
            pull(b, kind, px, pz, y + T);
        }
    }
}

/// `n` drawers stacked from `z0` to `z1`, pulls in their middles.
#[allow(clippy::too_many_arguments)]
pub(super) fn drawers(
    b: &mut Builder,
    panel: Panel,
    role: &'static str,
    x0: f64,
    x1: f64,
    z0: f64,
    z1: f64,
    y: f64,
    n: usize,
    kind: Pull,
) {
    let n = n.max(1);
    let gap = 0.003;
    let step = (z1 - z0) / n as f64;
    for i in 0..n {
        let (a, e) = (
            z0 + step * i as f64 + gap / 2.0,
            z0 + step * (i + 1) as f64 - gap / 2.0,
        );
        front(b, panel, role, x0 + gap / 2.0, x1 - gap / 2.0, a, e, y);
        let pz = match kind {
            Pull::Slot => e - 0.001,
            _ => (a + e) / 2.0,
        };
        pull(b, kind, (x0 + x1) / 2.0, pz, y + T);
    }
}

/// A round leg tapering to its foot, splayed out along `dir`; `at` is
/// its top, `top` high.
pub(super) fn tapered_leg(
    b: &mut Builder,
    role: &'static str,
    at: [f64; 2],
    top: f64,
    dir: [f64; 2],
    splay: f64,
    r: f64,
) {
    let start = b.mark();
    turned(
        b,
        role,
        [0.0; 3],
        &[
            (0.0, 0.0),
            (r * 0.6, 0.0),
            (r, top - 0.01),
            (r, top),
            (0.0, top),
        ],
        14,
        0.0,
    );
    let k = splay / top.max(1e-3);
    b.transform_from(start, |q| {
        let down = top - q[2];
        [
            at[0] + q[0] + dir[0] * k * down,
            at[1] + q[1] + dir[1] * k * down,
            q[2],
        ]
    });
}

/// A turned "toupie" foot of the Louis XVI pieces, `top` high.
pub(super) fn toupie(b: &mut Builder, at: [f64; 2], top: f64) {
    turned(
        b,
        "frame",
        [at[0], at[1], 0.0],
        &[
            (0.0, 0.0),
            (0.012, 0.0),
            (0.03, top * 0.45),
            (0.026, top * 0.6),
            (0.034, top * 0.75),
            (0.03, top),
            (0.0, top),
        ],
        16,
        0.0,
    );
}

/// The four corners of a carcass `x0`…`x1` by `0`…`d`, inset `k`.
pub(super) fn corners(x0: f64, x1: f64, d: f64, k: f64) -> [[f64; 2]; 4] {
    [[x0 + k, k], [x1 - k, k], [x1 - k, d - k], [x0 + k, d - k]]
}

/// The direction out of each corner of [`corners`].
pub(super) const OUT: [[f64; 2]; 4] = [[-0.7, -0.7], [0.7, -0.7], [0.7, 0.7], [-0.7, 0.7]];

/// An open box: sides, top, bottom and back, `t` thick, open at the front.
#[allow(clippy::too_many_arguments)]
fn open_carcass(
    b: &mut Builder,
    role: &'static str,
    x0: f64,
    x1: f64,
    d: f64,
    z0: f64,
    z1: f64,
    t: f64,
) {
    b.rbox(role, [x0, 0.0, z0], [x0 + t, d, z1], 0.003);
    b.rbox(role, [x1 - t, 0.0, z0], [x1, d, z1], 0.003);
    b.rbox(role, [x0 + t, 0.0, z1 - t], [x1 - t, d, z1], 0.002);
    b.rbox(role, [x0 + t, 0.0, z0], [x1 - t, d, z0 + t], 0.002);
    b.rbox(role, [x0 + t, 0.0, z0 + t], [x1 - t, 0.01, z1 - t], 0.0);
}

/// A shelf from `x0` to `x1`, its top at `z`.
fn shelf(b: &mut Builder, role: &'static str, x0: f64, x1: f64, d: f64, z: f64, t: f64) {
    b.rbox(role, [x0, 0.01, z - t], [x1, d, z], 0.002);
}

/// A run of books on a shelf from `x0` to `x1` standing at `z`, no taller
/// than `room`, their spines lined up near the front at `yf`; now and
/// then a gap, a lying stack, a vase or a leaning book.
#[allow(clippy::too_many_arguments)]
fn books(b: &mut Builder, rng: &mut Rng, x0: f64, x1: f64, yf: f64, z: f64, room: f64, depth: f64) {
    let roles = ["books", "books2", "books3", "books", "books2"];
    let mut x = x0 + rng.range(0.0, 0.04);
    let tall = room.min(0.32);
    while x < x1 - 0.03 {
        let pick = rng.next();
        if pick < 0.12 && x1 - x > 0.2 {
            // A stack lying flat, a little bowl on it.
            let (w, n) = (rng.range(0.15, 0.22), 2 + (rng.next() * 3.0) as usize);
            let mut zz = z;
            for i in 0..n {
                let t = rng.range(0.022, 0.04);
                let dx = rng.range(-0.01, 0.01);
                let role = roles[(i + (pick * 100.0) as usize) % roles.len()];
                b.rbox(
                    role,
                    [x + dx, yf - depth * 0.9, zz],
                    [x + dx + w, yf, zz + t],
                    0.002,
                );
                zz += t;
            }
            if room - (zz - z) > 0.08 {
                bowl(b, x + w / 2.0, yf - depth * 0.45, zz, 0.06);
            }
            x += w + rng.range(0.02, 0.05);
            continue;
        }
        if pick < 0.2 && x1 - x > 0.12 && room > 0.16 {
            vase(b, x + 0.045, yf - depth * 0.5, z, (room * 0.8).min(0.24));
            x += 0.1;
            continue;
        }
        // A run of upright books.
        let n = 4 + (rng.next() * 8.0) as usize;
        for _ in 0..n {
            let w = rng.range(0.016, 0.045);
            if x + w > x1 - 0.005 {
                break;
            }
            let h = rng.range(0.62, 0.98) * tall;
            let dp = rng.range(0.78, 1.0) * depth;
            let role = roles[(rng.next() * roles.len() as f64) as usize % roles.len()];
            b.rbox(role, [x, yf - dp, z], [x + w, yf, z + h], 0.0);
            x += w + 0.001;
        }
        // The last one leans on the run.
        if rng.next() < 0.5 && x + 0.06 < x1 {
            let (w, h) = (rng.range(0.02, 0.035), rng.range(0.7, 0.95) * tall);
            let lean: f64 = 0.32;
            let start = b.mark();
            b.rbox("books2", [0.0, -depth * 0.9, 0.0], [w, 0.0, h], 0.0);
            let (s, c) = lean.sin_cos();
            let at = x + h * s + 0.003;
            b.transform_from(start, |q| {
                [at + q[0] * c - q[2] * s, yf + q[1], z + q[0] * s + q[2] * c]
            });
            x = at + w + 0.01;
        }
        x += rng.range(0.03, 0.12);
    }
}

/// A turned vase `h` tall standing at `(x, y, z)`.
fn vase(b: &mut Builder, x: f64, y: f64, z: f64, h: f64) {
    let r = h * 0.28;
    turned(
        b,
        "ceramic",
        [x, y, z],
        &[
            (0.0, 0.0),
            (r * 0.6, 0.0),
            (r, h * 0.35),
            (r * 0.9, h * 0.6),
            (r * 0.4, h * 0.85),
            (r * 0.45, h),
            (0.0, h),
        ],
        20,
        0.0,
    );
}

/// A shallow bowl `w` across at `(x, y, z)`.
fn bowl(b: &mut Builder, x: f64, y: f64, z: f64, w: f64) {
    turned(
        b,
        "ceramic",
        [x, y, z],
        &[
            (0.0, 0.0),
            (w * 0.22, 0.0),
            (w * 0.45, w * 0.22),
            (w * 0.5, w * 0.3),
            (w * 0.44, w * 0.3),
            (0.0, w * 0.1),
        ],
        20,
        0.0,
    );
}

/// A plate standing up on its rim, `r` in radius, facing forward.
fn plate_up(b: &mut Builder, x: f64, y: f64, z: f64, r: f64) {
    let start = b.mark();
    b.lathe(
        "ceramic",
        [0.0; 3],
        &[
            (0.0, 0.0),
            (0.55, 0.0),
            (1.0, 0.12),
            (0.92, 0.14),
            (0.0, 0.03),
        ],
        r,
        r,
    );
    b.transform_from(start, |q| [x + q[0], y - q[2] * r * 0.6, z + r + q[1]]);
}

/// Things on the glass shelves of a cabinet: vases, bowls, plates.
fn display(b: &mut Builder, rng: &mut Rng, x0: f64, x1: f64, d: f64, z: f64, room: f64) {
    let mut x = x0 + 0.03;
    loop {
        let k = rng.next();
        // How wide the next thing is; it must fit before the side.
        let (w, kind) = if k < 0.35 {
            let h = (room * rng.range(0.55, 0.85)).min(0.3);
            (h * 0.6, (0, h))
        } else if k < 0.65 {
            let r = (room * 0.42).min(0.12);
            (2.0 * r, (1, r))
        } else {
            let w = (room * 0.5).min(0.16);
            (w, (2, w))
        };
        if x + w > x1 - 0.03 {
            break;
        }
        match kind {
            (0, h) => vase(b, x + w / 2.0, d * 0.5, z, h),
            (1, r) => plate_up(b, x + r, d * 0.25, z, r),
            (_, w) => bowl(b, x + w / 2.0, d * 0.5, z, w),
        }
        x += w + rng.range(0.03, 0.07);
    }
}

/// A television standing on its foot at the middle of `(y, z)`, about
/// `w` across.
fn television(b: &mut Builder, w: f64, y: f64, z: f64) {
    let (h, t) = (w * 0.5625, 0.028);
    let (x0, x1) = (-w / 2.0, w / 2.0);
    let zb = z + 0.07;
    b.rbox(
        "dark",
        [x0, y - t / 2.0, zb],
        [x1, y + t / 2.0, zb + h],
        0.004,
    );
    b.rbox(
        "screen",
        [x0 + 0.008, y + t / 2.0 - 0.001, zb + 0.008],
        [x1 - 0.008, y + t / 2.0 + 0.001, zb + h - 0.008],
        0.0,
    );
    b.rbox(
        "dark",
        [-0.03, y - 0.012, z + 0.006],
        [0.03, y + 0.002, zb + 0.05],
        0.004,
    );
    b.rbox(
        "dark",
        [-w * 0.16, y - 0.09, z],
        [w * 0.16, y + 0.07, z + 0.008],
        0.004,
    );
}

/// A basket of woven rush, open at the top.
fn basket(b: &mut Builder, x0: f64, x1: f64, y0: f64, y1: f64, z0: f64, z1: f64) {
    b.rbox("weave", [x0, y0, z0], [x1, y1, z1 - 0.01], 0.02);
    let ring = [[x0, y0, z1], [x1, y0, z1], [x1, y1, z1], [x0, y1, z1]];
    super::soft::tube(b, "weave", &ring, true, 0.009, 8);
    b.rbox(
        "dark",
        [x0 + 0.015, y0 + 0.015, z1 - 0.012],
        [x1 - 0.015, y1 - 0.015, z1 - 0.008],
        0.0,
    );
}

/// A moulded cornice along the front and the sides of a top at `z`,
/// standing out `out`.
pub(super) fn cornice(
    b: &mut Builder,
    role: &'static str,
    x0: f64,
    x1: f64,
    d: f64,
    z: f64,
    out: f64,
) {
    // Three steps, each on the one below.
    for (k, (dz, o)) in [(0.0, out), (0.016, out * 0.6), (0.03, out * 0.25)]
        .into_iter()
        .enumerate()
    {
        let (zz, t) = (z - dz, if k == 0 { 0.016 } else { 0.014 });
        b.rbox(role, [x0 - o, 0.0, zz - t], [x1 + o, d + o, zz], 0.004);
    }
}

/// A plinth moulding round the foot of a carcass at `z`.
pub(super) fn base_moulding(
    b: &mut Builder,
    role: &'static str,
    x0: f64,
    x1: f64,
    d: f64,
    z: f64,
    out: f64,
) {
    b.rbox(
        role,
        [x0 - out, 0.0, z],
        [x1 + out, d + out, z + 0.04],
        0.006,
    );
    b.rbox(
        role,
        [x0 - out * 0.5, 0.0, z + 0.04],
        [x1 + out * 0.5, d + out * 0.5, z + 0.055],
        0.004,
    );
}

/// Two iron strap hinges on the hinge side of a door.
pub(super) fn straps(b: &mut Builder, x: f64, toward: f64, z0: f64, z1: f64, y: f64) {
    for z in [z0 + 0.07, z1 - 0.07] {
        let len = 0.14;
        let xe = x + toward * len;
        b.rbox(
            "metal",
            [x.min(xe), y, z - 0.012],
            [x.max(xe), y + 0.004, z + 0.012],
            0.002,
        );
        b.cyl("metal", [xe, y, z], [xe, y + 0.005, z], 0.016);
        for t in [0.3, 0.7] {
            let xn = x + toward * len * t;
            b.cyl("metal", [xn, y + 0.004, z], [xn, y + 0.008, z], 0.004);
        }
    }
}

/// Rivets in a row from `a` to `c` on a steel face looking forward.
fn rivets(b: &mut Builder, a: P3, c: P3, n: usize) {
    for i in 0..n {
        let t = (i as f64 + 0.5) / n as f64;
        let q = [a[0] + (c[0] - a[0]) * t, a[1], a[2] + (c[2] - a[2]) * t];
        b.ellipsoid("metal", q, [0.004, 0.003, 0.004]);
    }
}

/// A steel frame of square tube round a box: four posts from the floor
/// and rails at the given heights.
#[allow(clippy::too_many_arguments)]
pub(super) fn steel_frame(
    b: &mut Builder,
    x0: f64,
    x1: f64,
    d: f64,
    top: f64,
    rails: &[f64],
    tube: f64,
    braces: bool,
) {
    let h = tube / 2.0;
    let pts = [[x0 + h, h], [x1 - h, h], [x1 - h, d - h], [x0 + h, d - h]];
    for q in pts {
        bar(
            b,
            "metal",
            [q[0], q[1], 0.0],
            [q[0], q[1], top],
            tube,
            [1.0, 0.0, 0.0],
        );
    }
    for &z in rails {
        for i in 0..4 {
            let (a, e) = (pts[i], pts[(i + 1) % 4]);
            bar(
                b,
                "metal",
                [a[0], a[1], z],
                [e[0], e[1], z],
                tube,
                [0.0, 0.0, 1.0],
            );
        }
    }
    if braces && rails.len() >= 2 {
        // An X on each side between the lowest and the highest rail.
        let (za, zb) = (rails[0], rails[rails.len() - 1]);
        for x in [x0 + h, x1 - h] {
            for (ya, yb) in [(h, d - h), (d - h, h)] {
                bar(
                    b,
                    "metal",
                    [x, ya, za],
                    [x, yb, zb],
                    tube * 0.35,
                    [1.0, 0.0, 0.0],
                );
            }
        }
    }
}

/// The arch of a niche: a band of `n` pieces round a half circle from
/// `x0` to `x1` springing at `z`, `t` deep at the front.
#[allow(clippy::too_many_arguments)]
fn arch(
    b: &mut Builder,
    role: &'static str,
    x0: f64,
    x1: f64,
    z: f64,
    d: f64,
    band: f64,
    top: f64,
) {
    // The spandrels: the board above the arch, cut round it, each a fan
    // of triangles from its top corner (the arch's tangents are the side
    // and the top, so every point of it is seen from there).
    let r = (x1 - x0) / 2.0;
    let cx = (x0 + x1) / 2.0;
    let n = 24;
    let (yf, yb) = (d, d - band);
    let tri = |b: &mut Builder, p: [[f64; 2]; 3], y: f64, out: f64| {
        let [a, c, e] = p;
        // Its normal along y: (c - a) × (e - a).
        let ny = (c[1] - a[1]) * (e[0] - a[0]) - (c[0] - a[0]) * (e[1] - a[1]);
        let (c, e) = if ny * out >= 0.0 { (c, e) } else { (e, c) };
        let ia = b.point([a[0], y, a[1]]);
        let ic = b.point([c[0], y, c[1]]);
        let ie = b.point([e[0], y, e[1]]);
        b.tri(role, ia, ic, ie);
    };
    for side in [-1.0f64, 1.0] {
        let corner = [cx + side * r, top];
        let arc: Vec<[f64; 2]> = (0..=n)
            .map(|k| {
                let a = PI / 2.0 * k as f64 / n as f64;
                [cx + side * r * a.cos(), z + r * a.sin()]
            })
            .collect();
        for k in 0..n {
            let (p, q) = (arc[k], arc[k + 1]);
            tri(b, [corner, p, q], yf, 1.0);
            tri(b, [corner, p, q], yb, -1.0);
            // The soffit of the arch between the faces, looking in.
            let m = [(p[0] + q[0]) / 2.0, (p[1] + q[1]) / 2.0];
            let inward = [cx - m[0], z - m[1]];
            let ids = [
                b.point([p[0], yf, p[1]]),
                b.point([q[0], yf, q[1]]),
                b.point([q[0], yb, q[1]]),
                b.point([p[0], yb, p[1]]),
            ];
            // (q - p) × (0, -1, 0) looks to (−dz, 0, dx)·…: check the side.
            let (dx, dz) = (q[0] - p[0], q[1] - p[1]);
            let normal = [dz, -dx];
            if normal[0] * inward[0] + normal[1] * inward[1] >= 0.0 {
                b.quad(role, ids[0], ids[1], ids[2], ids[3]);
            } else {
                b.quad(role, ids[3], ids[2], ids[1], ids[0]);
            }
        }
    }
}

/// A sideboard of a collection.
pub fn sideboard(b: &mut Builder, p: &Params) {
    let (w, d, h) = dims(p, 1.6, 0.45, 0.78);
    let (x0, x1) = (-w / 2.0, w / 2.0);
    let fy = d - T;
    match p.style() {
        // Mediterranean: whitewashed, on its own square legs, cane doors
        // with turned knobs, a wooden top.
        2 => {
            let z0 = 0.14;
            for q in corners(x0, x1, d, 0.0) {
                let (xa, ya) = (
                    q[0].min(q[0] + if q[0] < 0.0 { 0.05 } else { -0.05 }),
                    q[1].min(q[1] + if q[1] < 0.01 { 0.05 } else { -0.05 }),
                );
                b.rbox(
                    "frame",
                    [xa, ya, 0.0],
                    [xa + 0.05, ya + 0.05, h - 0.03],
                    0.004,
                );
            }
            b.rbox(
                "frame",
                [x0 + 0.01, 0.0, z0],
                [x1 - 0.01, fy, h - 0.03],
                0.003,
            );
            // An apron under the doors, gently scalloped.
            b.rbox(
                "frame",
                [x0 + 0.05, fy, z0 - 0.04],
                [x1 - 0.05, d, z0],
                0.004,
            );
            b.rbox(
                "top",
                [x0 - 0.015, -0.01, h - 0.03],
                [x1 + 0.015, d + 0.015, h],
                0.008,
            );
            let n = if w > 1.3 { 3 } else { 2 };
            doors(
                b,
                Panel::Cane,
                "front",
                x0 + 0.05,
                x1 - 0.05,
                z0 + 0.005,
                h - 0.035,
                fy,
                n,
                Pull::Knob,
                (z0 + h) / 2.0,
            );
        }
        // Fifties: walnut on compass legs with brass shoes, fluted doors
        // either side of a pair of drawers, brass pulls.
        3 => {
            let z0 = 0.24;
            for (q, o) in corners(x0, x1, d, 0.1).iter().zip(OUT) {
                compass_leg(b, *q, z0 + 0.01, o, 0.04);
            }
            b.rbox(
                "frame",
                [x0 + 0.06, 0.06, z0 - 0.03],
                [x1 - 0.06, d - 0.06, z0],
                0.004,
            );
            b.rbox("frame", [x0, 0.0, z0], [x1, fy, h], 0.02);
            let side = w * 0.34;
            doors(
                b,
                Panel::Flutes,
                "front",
                x0 + 0.02,
                x0 + side,
                z0 + 0.02,
                h - 0.02,
                fy,
                1,
                Pull::Brass,
                (z0 + h) / 2.0,
            );
            drawers(
                b,
                Panel::Flat,
                "front",
                x0 + side + 0.004,
                x1 - side - 0.004,
                z0 + 0.02,
                h - 0.02,
                fy,
                2,
                Pull::Bar(0.1),
            );
            doors(
                b,
                Panel::Flutes,
                "front",
                x1 - side,
                x1 - 0.02,
                z0 + 0.02,
                h - 0.02,
                fy,
                1,
                Pull::Brass,
                (z0 + h) / 2.0,
            );
            b.rbox(
                "dark",
                [x0 + 0.02, fy - 0.002, z0 + 0.008],
                [x1 - 0.02, fy + 0.001, z0 + 0.018],
                0.0,
            );
        }
        // Japandi: long and low on a recessed plinth, sliding doors of
        // slats with leather pulls.
        4 => {
            let z0 = 0.09;
            b.rbox(
                "dark",
                [x0 + 0.05, 0.04, 0.0],
                [x1 - 0.05, d - 0.06, z0],
                0.0,
            );
            b.rbox("frame", [x0, 0.0, z0], [x1, fy, h], 0.006);
            // Two sliding doors, one behind the other.
            let half = w / 2.0;
            front(
                b,
                Panel::Ribs,
                "front",
                x0 + 0.02,
                0.01,
                z0 + 0.02,
                h - 0.02,
                fy - 0.004,
            );
            front(
                b,
                Panel::Ribs,
                "front",
                -0.01,
                x1 - 0.02,
                z0 + 0.02,
                h - 0.02,
                fy + 0.012,
            );
            pull(
                b,
                Pull::Tab,
                x0 + 0.02 + half * 0.08,
                h - 0.05,
                fy + T - 0.004,
            );
            pull(
                b,
                Pull::Tab,
                x1 - 0.02 - half * 0.08,
                h - 0.05,
                fy + T + 0.012,
            );
        }
        // Contemporary: lacquered, push to open, a stone top, floating on
        // a dark recessed plinth.
        5 => {
            let z0 = 0.1;
            b.rbox(
                "dark",
                [x0 + 0.06, 0.06, 0.0],
                [x1 - 0.06, d - 0.08, z0],
                0.0,
            );
            b.rbox("front", [x0, 0.0, z0], [x1, fy, h - 0.03], 0.003);
            b.rbox("stone", [x0, 0.0, h - 0.03], [x1, d, h], 0.004);
            let n = ((w / 0.42).round() as usize).max(2);
            doors(
                b,
                Panel::Flat,
                "front",
                x0,
                x1,
                z0,
                h - 0.032,
                fy,
                n,
                Pull::None,
                0.0,
            );
        }
        // Rustic: thick solid pine, posts to the floor, plank doors with
        // iron strap hinges and ring pulls, a heavy top.
        6 => {
            let leg = 0.07;
            for q in [
                [x0, 0.0],
                [x1 - leg, 0.0],
                [x1 - leg, fy - leg],
                [x0, fy - leg],
            ] {
                b.rbox(
                    "frame",
                    [q[0], q[1], 0.0],
                    [q[0] + leg, q[1] + leg, h - 0.05],
                    0.006,
                );
            }
            b.rbox(
                "frame",
                [x0 + 0.01, 0.01, 0.12],
                [x1 - 0.01, fy, h - 0.05],
                0.004,
            );
            b.rbox(
                "frame",
                [x0 + leg, fy - 0.01, 0.08],
                [x1 - leg, fy + 0.012, 0.13],
                0.004,
            );
            b.rbox(
                "top",
                [x0 - 0.03, -0.01, h - 0.05],
                [x1 + 0.03, d + 0.03, h],
                0.012,
            );
            let n = if w > 1.3 { 3 } else { 2 };
            let (za, zb) = (0.14, h - 0.07);
            let step = (w - 2.0 * leg) / n as f64;
            for i in 0..n {
                let a = x0 + leg + step * i as f64 + 0.004;
                let e = a + step - 0.008;
                front(b, Panel::Planks, "front", a, e, za, zb, fy);
                let hinge_left = i % 2 == 0;
                let (hx, tw, px) = if hinge_left {
                    (a + 0.01, 1.0, e - 0.05)
                } else {
                    (e - 0.01, -1.0, a + 0.05)
                };
                straps(b, hx, tw, za, zb, fy + T);
                pull(b, Pull::Ring, px, (za + zb) / 2.0 + 0.02, fy + T);
            }
        }
        // Industrial: a steel frame, wooden drawers with iron cup pulls,
        // mesh doors either side, a plank top.
        7 => {
            let tube = 0.03;
            steel_frame(
                b,
                x0,
                x1,
                d,
                h - 0.035,
                &[0.12, h - 0.035 - tube / 2.0],
                tube,
                false,
            );
            b.rbox(
                "frame",
                [x0 + tube, tube, 0.12],
                [x1 - tube, fy, h - 0.05],
                0.002,
            );
            for i in 0..4 {
                let xa = x0 + w * i as f64 / 4.0;
                b.rbox(
                    "top",
                    [xa + 0.002, 0.0, h - 0.035],
                    [xa + w / 4.0 - 0.002, d, h],
                    0.003,
                );
            }
            let side = (w - 2.0 * tube) * 0.3;
            let (za, zb) = (0.12 + tube / 2.0 + 0.004, h - 0.05 - tube / 2.0);
            doors(
                b,
                Panel::Mesh,
                "front",
                x0 + tube,
                x0 + tube + side,
                za,
                zb,
                fy,
                1,
                Pull::Cup,
                zb - 0.06,
            );
            drawers(
                b,
                Panel::Flat,
                "front",
                x0 + tube + side + 0.004,
                x1 - tube - side - 0.004,
                za,
                zb,
                fy,
                3,
                Pull::Cup,
            );
            doors(
                b,
                Panel::Mesh,
                "front",
                x1 - tube - side,
                x1 - tube,
                za,
                zb,
                fy,
                1,
                Pull::Cup,
                zb - 0.06,
            );
            for x in [x0 + tube / 2.0, x1 - tube / 2.0] {
                rivets(b, [x, d + 0.001, 0.2], [x, d + 0.001, h - 0.1], 5);
            }
        }
        // Classic: Louis XVI — toupie feet, a moulded base, raised panel
        // doors with brass knobs, fluted pilasters, a marble top.
        8 => {
            let z0 = 0.12;
            for q in corners(x0, x1, d, 0.05) {
                toupie(b, q, z0);
            }
            base_moulding(b, "frame", x0, x1, fy, z0, 0.012);
            b.rbox("frame", [x0, 0.0, z0], [x1, fy, h - 0.03], 0.004);
            for x in [x0 + 0.035, x1 - 0.035] {
                front(
                    b,
                    Panel::Flutes,
                    "frame",
                    x - 0.025,
                    x + 0.025,
                    z0 + 0.07,
                    h - 0.06,
                    fy,
                );
            }
            cornice(b, "frame", x0, x1, fy, h - 0.03, 0.012);
            b.rbox(
                "stone",
                [x0 - 0.02, 0.0, h - 0.03],
                [x1 + 0.02, d + 0.01, h],
                0.006,
            );
            let (xa, xb) = (x0 + 0.07, x1 - 0.07);
            let mid = (xb - xa) * 0.3;
            let cx = (xa + xb) / 2.0;
            doors(
                b,
                Panel::Raised,
                "front",
                xa,
                cx - mid / 2.0,
                z0 + 0.07,
                h - 0.08,
                fy,
                1,
                Pull::Brass,
                (z0 + h) / 2.0,
            );
            drawers(
                b,
                Panel::Raised,
                "front",
                cx - mid / 2.0 + 0.004,
                cx + mid / 2.0 - 0.004,
                z0 + 0.07,
                h - 0.08,
                fy,
                2,
                Pull::Brass,
            );
            doors(
                b,
                Panel::Raised,
                "front",
                cx + mid / 2.0,
                xb,
                z0 + 0.07,
                h - 0.08,
                fy,
                1,
                Pull::Brass,
                (z0 + h) / 2.0,
            );
        }
        // Nordic: oak on four splayed tapered legs, two doors and three
        // drawers with finger cuts, a top with a soft edge.
        _ => {
            let z0 = 0.2;
            for (q, o) in corners(x0, x1, d, 0.09).iter().zip(OUT) {
                tapered_leg(b, "frame", *q, z0 + 0.005, o, 0.03, 0.018);
            }
            b.rbox("frame", [x0, 0.0, z0], [x1, fy, h - 0.025], 0.006);
            b.rbox(
                "top",
                [x0 - 0.005, -0.005, h - 0.025],
                [x1 + 0.005, d + 0.005, h],
                0.01,
            );
            let split = x0 + w * 0.62;
            doors(
                b,
                Panel::Flat,
                "front",
                x0 + 0.015,
                split,
                z0 + 0.015,
                h - 0.04,
                fy,
                2,
                Pull::Slot,
                0.0,
            );
            drawers(
                b,
                Panel::Flat,
                "front",
                split + 0.004,
                x1 - 0.015,
                z0 + 0.015,
                h - 0.04,
                fy,
                3,
                Pull::Slot,
            );
        }
    }
}

/// A bookcase of a collection, its shelves dressed with books and things.
pub fn bookcase(b: &mut Builder, p: &Params) {
    let (w, d, h) = dims(p, 1.0, 0.36, 2.0);
    let (x0, x1) = (-w / 2.0, w / 2.0);
    let mut rng = Rng(p.get("seed", 7.0) as u32 + 11);
    let fill = p.flag("with_books", true);
    // Shelves between top and bottom («Baldas intermedias»), if asked.
    let asked = p.get("shelves", 0.0);
    // Shelves evenly from `z0` to `z1`, `n` spaces, dressed.
    let dress = |b: &mut Builder,
                 rng: &mut Rng,
                 xa: f64,
                 xb: f64,
                 za: f64,
                 zb: f64,
                 n: usize,
                 t: f64,
                 role: &'static str| {
        // The shelves asked for (spaces one more), or the style's own.
        let n = if asked > 0.0 {
            (asked.round() as usize + 1).clamp(1, 10)
        } else {
            n
        };
        let step = (zb - za) / n as f64;
        for i in 1..n {
            shelf(b, role, xa, xb, d - 0.01, za + step * i as f64, t);
        }
        if fill {
            for i in 0..n {
                let z = za + step * i as f64;
                books(
                    b,
                    rng,
                    xa + 0.01,
                    xb - 0.01,
                    d - 0.03,
                    z,
                    step - t - 0.02,
                    d - 0.08,
                );
            }
        }
    };
    match p.style() {
        // Mediterranean: whitewashed, an arched niche over the shelves,
        // cane doors below.
        2 => {
            let t = 0.03;
            let low = 0.8;
            b.rbox("frame", [x0, 0.0, 0.0], [x1, d, low], 0.004);
            b.rbox(
                "dark",
                [x0 + 0.03, d - 0.002, 0.0],
                [x1 - 0.03, d + 0.001, 0.06],
                0.0,
            );
            doors(
                b,
                Panel::Cane,
                "front",
                x0 + 0.03,
                x1 - 0.03,
                0.08,
                low - 0.03,
                d,
                2,
                Pull::Knob,
                low - 0.15,
            );
            b.rbox(
                "top",
                [x0 - 0.015, 0.0, low],
                [x1 + 0.015, d + 0.02, low + 0.03],
                0.006,
            );
            b.rbox("frame", [x0, 0.0, low + 0.03], [x0 + t, d - 0.04, h], 0.003);
            b.rbox("frame", [x1 - t, 0.0, low + 0.03], [x1, d - 0.04, h], 0.003);
            b.rbox("frame", [x0 + t, 0.0, low + 0.03], [x1 - t, 0.012, h], 0.0);
            let spring = h - (w / 2.0 - t) - 0.04;
            b.rbox("frame", [x0, 0.0, h - 0.04], [x1, d - 0.04, h], 0.003);
            arch(b, "frame", x0 + t, x1 - t, spring, d - 0.04, 0.03, h - 0.04);
            dress(
                b,
                &mut rng,
                x0 + t,
                x1 - t,
                low + 0.03,
                spring,
                3,
                0.03,
                "frame",
            );
        }
        // Fifties: walnut shelves on thin black uprights over a low cabinet
        // with fluted sliding doors, on compass legs.
        3 => {
            let (z0, low) = (0.22, 0.62);
            for (q, o) in corners(x0, x1, d, 0.07).iter().zip(OUT) {
                compass_leg(b, *q, z0 + 0.01, o, 0.035);
            }
            b.rbox("frame", [x0, 0.0, z0], [x1, d - T, low], 0.015);
            front(
                b,
                Panel::Flutes,
                "front",
                x0 + 0.02,
                0.006,
                z0 + 0.02,
                low - 0.02,
                d - T - 0.004,
            );
            front(
                b,
                Panel::Flutes,
                "front",
                -0.006,
                x1 - 0.02,
                z0 + 0.02,
                low - 0.02,
                d - T + 0.012,
            );
            pull(b, Pull::Brass, x0 + 0.06, (z0 + low) / 2.0, d - 0.004);
            pull(b, Pull::Brass, x1 - 0.06, (z0 + low) / 2.0, d + 0.012);
            for x in [x0 + 0.02, x1 - 0.02] {
                for y in [0.03, d - 0.05] {
                    b.cyl("metal", [x, y, low], [x, y, h], 0.007);
                }
            }
            dress(b, &mut rng, x0, x1, low, h + 0.02, 4, 0.022, "frame");
            shelf(b, "frame", x0, x1, d - 0.01, h, 0.022);
        }
        // Japandi: dark ash cubbies in three staggered columns, two of
        // them closed with slatted fronts.
        4 => {
            let t = 0.025;
            open_carcass(b, "frame", x0, x1, d, 0.06, h, t);
            b.rbox(
                "dark",
                [x0 + 0.03, 0.03, 0.0],
                [x1 - 0.03, d - 0.04, 0.06],
                0.0,
            );
            let cols = 3;
            let cw = (w - 2.0 * t) / cols as f64;
            for c in 1..cols {
                let x = x0 + t + cw * c as f64;
                b.rbox(
                    "frame",
                    [x - t / 2.0, 0.01, 0.06 + t],
                    [x + t / 2.0, d, h - t],
                    0.0,
                );
            }
            for c in 0..cols {
                let (xa, xb) = (
                    x0 + t + cw * c as f64 + if c > 0 { t / 2.0 } else { 0.0 },
                    x0 + t + cw * (c + 1) as f64 - if c + 1 < cols { t / 2.0 } else { 0.0 },
                );
                let rows = 4 + c % 2;
                let off = [0.0, 0.18, 0.09][c % 3];
                let mut zs: Vec<f64> = (1..rows)
                    .map(|i| {
                        0.06 + t
                            + (h - 0.06 - 2.0 * t) * i as f64 / rows as f64
                            + off * if i == 1 { 1.0 } else { 0.3 }
                    })
                    .collect();
                zs.retain(|z| *z < h - 0.2);
                let mut bottom = 0.06 + t;
                for (i, z) in zs.iter().chain(std::iter::once(&(h - t + t))).enumerate() {
                    let top = *z - t;
                    if *z < h - 0.01 {
                        shelf(b, "frame", xa, xb, d, *z, t);
                    }
                    if (c + i) % 4 == 1 {
                        front(
                            b,
                            Panel::Ribs,
                            "front",
                            xa + 0.003,
                            xb - 0.003,
                            bottom + 0.003,
                            top - 0.003,
                            d - T,
                        );
                    } else if fill {
                        books(
                            b,
                            &mut rng,
                            xa + 0.01,
                            xb - 0.01,
                            d - 0.03,
                            bottom,
                            top - bottom - 0.02,
                            d - 0.08,
                        );
                    }
                    bottom = *z;
                }
            }
        }
        // Contemporary: a lacquered grid of compartments of different
        // sizes, a few closed with push doors.
        5 => {
            let t = 0.02;
            open_carcass(b, "front", x0, x1, d, 0.0, h, t);
            let rows = 5;
            let rh = (h - t) / rows as f64;
            for r in 0..rows {
                let z = t / 2.0 + rh * r as f64;
                if r > 0 {
                    shelf(b, "front", x0 + t, x1 - t, d, z + t / 2.0, t);
                }
                // An upright in each row, alternating sides.
                let split = if r % 2 == 0 { 0.36 } else { 0.62 };
                let xs = x0 + w * split;
                b.rbox(
                    "front",
                    [xs - t / 2.0, 0.01, z + t / 2.0],
                    [xs + t / 2.0, d, z + rh - t / 2.0],
                    0.0,
                );
                for (k, (xa, xb)) in [(x0 + t, xs - t / 2.0), (xs + t / 2.0, x1 - t)]
                    .into_iter()
                    .enumerate()
                {
                    let (za, zb) = (z + t / 2.0, z + rh - t / 2.0);
                    if (r + k) % 3 == 0 {
                        front(
                            b,
                            Panel::Flat,
                            "front",
                            xa + 0.002,
                            xb - 0.002,
                            za + 0.002,
                            zb - 0.002,
                            d - T,
                        );
                    } else if fill {
                        books(
                            b,
                            &mut rng,
                            xa + 0.01,
                            xb - 0.01,
                            d - 0.03,
                            za,
                            zb - za - 0.02,
                            d - 0.08,
                        );
                    }
                }
            }
        }
        // Rustic: thick pine planks, open above, plank doors below with
        // iron hinges.
        6 => {
            let t = 0.04;
            let low = 0.75;
            open_carcass(b, "frame", x0, x1, d, 0.0, h, t);
            b.rbox("frame", [x0 + t, 0.0, low - t], [x1 - t, d, low], 0.004);
            b.rbox(
                "frame",
                [x0 - 0.01, 0.0, h - 0.04],
                [x1 + 0.01, d + 0.02, h],
                0.006,
            );
            let n = 2;
            let step = (w - 2.0 * t) / n as f64;
            for i in 0..n {
                let a = x0 + t + step * i as f64 + 0.003;
                let e = a + step - 0.006;
                front(
                    b,
                    Panel::Planks,
                    "front",
                    a,
                    e,
                    t + 0.04,
                    low - t - 0.004,
                    d - T,
                );
                let left = i == 0;
                straps(
                    b,
                    if left { a + 0.01 } else { e - 0.01 },
                    if left { 1.0 } else { -1.0 },
                    t + 0.04,
                    low - t,
                    d,
                );
                pull(
                    b,
                    Pull::Ring,
                    if left { e - 0.05 } else { a + 0.05 },
                    low - 0.15,
                    d,
                );
            }
            b.rbox(
                "frame",
                [x0 + t, d - 0.03, 0.0],
                [x1 - t, d, t + 0.04],
                0.003,
            );
            dress(b, &mut rng, x0 + t, x1 - t, low, h - t, 3, 0.035, "frame");
        }
        // Industrial: steel uprights braced in an X at the sides, plank
        // shelves.
        7 => {
            let tube = 0.03;
            let n = if asked > 0.0 {
                (asked.round() as usize + 1).clamp(1, 10)
            } else {
                5
            };
            let rails: Vec<f64> = (0..=n)
                .map(|i| 0.06 + (h - 0.08) * i as f64 / n as f64)
                .collect();
            steel_frame(b, x0, x1, d, h, &rails, tube, true);
            for (i, z) in rails.iter().enumerate() {
                for k in 0..3 {
                    let ya = tube + (d - 2.0 * tube) * k as f64 / 3.0;
                    b.rbox(
                        "top",
                        [x0 + tube, ya + 0.002, z + tube / 2.0],
                        [
                            x1 - tube,
                            ya + (d - 2.0 * tube) / 3.0 - 0.002,
                            z + tube / 2.0 + 0.03,
                        ],
                        0.003,
                    );
                }
                if fill && i + 1 < rails.len() {
                    let room = rails[i + 1] - z - tube - 0.05;
                    books(
                        b,
                        &mut rng,
                        x0 + tube + 0.01,
                        x1 - tube - 0.01,
                        d - 0.04,
                        z + tube / 2.0 + 0.03,
                        room,
                        d - 0.09,
                    );
                }
            }
        }
        // Classic: walnut with a cornice, pilasters and a moulded base,
        // open above, raised panel doors below.
        8 => {
            let t = 0.03;
            let low = 0.85;
            base_moulding(b, "frame", x0, x1, d, 0.0, 0.015);
            b.rbox("frame", [x0, 0.0, 0.0], [x1, d, low], 0.004);
            doors(
                b,
                Panel::Raised,
                "front",
                x0 + 0.06,
                x1 - 0.06,
                0.1,
                low - 0.04,
                d,
                2,
                Pull::Brass,
                low - 0.18,
            );
            b.rbox(
                "frame",
                [x0 - 0.02, 0.0, low],
                [x1 + 0.02, d + 0.03, low + 0.035],
                0.008,
            );
            open_carcass(
                b,
                "frame",
                x0 + 0.02,
                x1 - 0.02,
                d - 0.03,
                low + 0.035,
                h - 0.08,
                t,
            );
            for x in [x0 + 0.02 + t / 2.0, x1 - 0.02 - t / 2.0] {
                front(
                    b,
                    Panel::Flutes,
                    "frame",
                    x - 0.025,
                    x + 0.025,
                    low + 0.06,
                    h - 0.1,
                    d - 0.03,
                );
            }
            // The top board, the cornice's steps on it.
            b.rbox(
                "frame",
                [x0 + 0.02, 0.0, h - 0.08],
                [x1 - 0.02, d - 0.03, h - 0.044],
                0.0,
            );
            cornice(b, "frame", x0 + 0.02, x1 - 0.02, d - 0.03, h, 0.025);
            dress(
                b,
                &mut rng,
                x0 + 0.02 + t,
                x1 - 0.02 - t,
                low + 0.035 + t,
                h - 0.08 - t,
                4,
                0.025,
                "frame",
            );
        }
        // Nordic: open oak, five spaces on a low plinth.
        _ => {
            let t = 0.024;
            open_carcass(b, "frame", x0, x1, d, 0.06, h, t);
            b.rbox(
                "frame",
                [x0 + 0.02, 0.02, 0.0],
                [x1 - 0.02, d - 0.03, 0.06],
                0.004,
            );
            dress(b, &mut rng, x0 + t, x1 - t, 0.06 + t, h - t, 5, t, "frame");
        }
    }
}

/// A television unit of a collection, its television standing on it.
pub fn tv_unit(b: &mut Builder, p: &Params) {
    let (w, d, h) = dims(p, 1.8, 0.42, 0.5);
    let (x0, x1) = (-w / 2.0, w / 2.0);
    let fy = d - T;
    let mut rng = Rng(p.get("seed", 3.0) as u32 + 31);
    let tv = p.get("screen", (w * 0.7).min(1.45));
    match p.style() {
        // Mediterranean: whitewashed, cane doors either side of an open
        // middle with a basket.
        2 => {
            for q in corners(x0, x1, d, 0.03) {
                b.rbox(
                    "frame",
                    [q[0] - 0.025, q[1] - 0.025, 0.0],
                    [q[0] + 0.025, q[1] + 0.025, 0.12],
                    0.004,
                );
            }
            open_carcass(b, "frame", x0, x1, d - 0.002, 0.1, h - 0.03, 0.025);
            b.rbox(
                "top",
                [x0 - 0.01, 0.0, h - 0.03],
                [x1 + 0.01, d + 0.01, h],
                0.006,
            );
            let side = w * 0.32;
            for (xa, xb) in [(x0 + 0.025, x0 + side), (x1 - side, x1 - 0.025)] {
                b.rbox("frame", [xa, 0.0, 0.125], [xb, d - T, h - 0.055], 0.0);
                front(
                    b,
                    Panel::Cane,
                    "front",
                    xa + 0.002,
                    xb - 0.002,
                    0.127,
                    h - 0.057,
                    d - T,
                );
                pull(
                    b,
                    Pull::Knob,
                    if xa < 0.0 { xb - 0.04 } else { xa + 0.04 },
                    (h + 0.1) / 2.0,
                    d,
                );
            }
            basket(b, -0.18, 0.18, 0.06, d - 0.05, 0.125, h - 0.12);
        }
        // Fifties: walnut on compass legs, fluted doors and an open middle.
        3 => {
            let z0 = 0.2;
            for (q, o) in corners(x0, x1, d, 0.1).iter().zip(OUT) {
                compass_leg(b, *q, z0 + 0.01, o, 0.04);
            }
            open_carcass(b, "frame", x0, x1, d, z0, h, 0.022);
            let side = w * 0.3;
            for (xa, xb) in [(x0 + 0.022, x0 + side), (x1 - side, x1 - 0.022)] {
                b.rbox("frame", [xa, 0.0, z0 + 0.022], [xb, fy, h - 0.022], 0.0);
                doors(
                    b,
                    Panel::Flutes,
                    "front",
                    xa + 0.002,
                    xb - 0.002,
                    z0 + 0.024,
                    h - 0.024,
                    fy,
                    1,
                    Pull::Brass,
                    (z0 + h) / 2.0,
                );
            }
            books(
                b,
                &mut rng,
                x0 + side + 0.02,
                x1 - side - 0.02,
                d - 0.03,
                z0 + 0.022,
                h - z0 - 0.06,
                d - 0.1,
            );
        }
        // Japandi: a low platform on a recessed plinth, open, one sliding
        // slatted door.
        4 => {
            b.rbox(
                "dark",
                [x0 + 0.04, 0.04, 0.0],
                [x1 - 0.04, d - 0.05, 0.05],
                0.0,
            );
            open_carcass(b, "frame", x0, x1, d, 0.05, h, 0.03);
            let mid = x0 + w * 0.55;
            b.rbox(
                "frame",
                [mid - 0.012, 0.01, 0.08],
                [mid + 0.012, d, h - 0.03],
                0.0,
            );
            front(
                b,
                Panel::Ribs,
                "front",
                mid + 0.015,
                x1 - 0.035,
                0.083,
                h - 0.033,
                d - T,
            );
            pull(b, Pull::Tab, mid + 0.06, h - 0.08, d);
            books(
                b,
                &mut rng,
                x0 + 0.06,
                mid - 0.04,
                d - 0.03,
                0.08,
                h - 0.13,
                d - 0.1,
            );
        }
        // Contemporary: lacquered drawers that seem to float, a dark
        // recessed plinth, push to open.
        5 => {
            b.rbox(
                "dark",
                [x0 + 0.08, 0.06, 0.0],
                [x1 - 0.08, d - 0.1, 0.12],
                0.0,
            );
            b.rbox("front", [x0, 0.0, 0.12], [x1, fy, h], 0.003);
            let n = ((w / 0.6).round() as usize).max(2);
            let step = w / n as f64;
            for i in 0..n {
                let a = x0 + step * i as f64 + 0.0015;
                front(b, Panel::Flat, "front", a, a + step - 0.003, 0.12, h, fy);
            }
            b.rbox("dark", [x0, fy - 0.001, 0.12], [x1, fy + 0.002, 0.125], 0.0);
        }
        // Rustic: a plank bench with an open shelf and baskets.
        6 => {
            let t = 0.045;
            for x in [x0, x1 - t] {
                b.rbox("frame", [x, 0.0, 0.0], [x + t, d, h - t], 0.006);
            }
            b.rbox(
                "top",
                [x0 - 0.02, 0.0, h - t],
                [x1 + 0.02, d + 0.01, h],
                0.008,
            );
            shelf(b, "frame", x0 + t, x1 - t, d, 0.12 + 0.035, 0.035);
            let n = ((w - 2.0 * t) / 0.42).floor().max(1.0) as usize;
            let step = (w - 2.0 * t) / n as f64;
            for i in 0..n {
                let a = x0 + t + step * i as f64;
                basket(
                    b,
                    a + 0.03,
                    a + step - 0.03,
                    0.04,
                    d - 0.03,
                    0.155,
                    h - t - 0.05,
                );
            }
            b.rbox(
                "frame",
                [x0 + t, d - 0.04, 0.03],
                [x1 - t, d - 0.01, 0.12],
                0.003,
            );
        }
        // Industrial: a steel frame with plank shelves and a mesh door.
        7 => {
            let tube = 0.03;
            steel_frame(
                b,
                x0,
                x1,
                d,
                h - 0.035,
                &[0.08, h - 0.035 - tube / 2.0],
                tube,
                false,
            );
            b.rbox("top", [x0, 0.0, h - 0.035], [x1, d, h], 0.003);
            b.rbox(
                "top",
                [x0 + tube, tube, 0.08 + tube / 2.0],
                [x1 - tube, d - tube, 0.08 + tube / 2.0 + 0.03],
                0.003,
            );
            let side = w * 0.3;
            b.rbox(
                "frame",
                [x1 - tube - side, tube, 0.08 + tube / 2.0 + 0.03],
                [x1 - tube, fy, h - 0.035 - tube],
                0.0,
            );
            front(
                b,
                Panel::Mesh,
                "front",
                x1 - tube - side + 0.003,
                x1 - tube - 0.003,
                0.08 + tube / 2.0 + 0.033,
                h - 0.035 - tube - 0.003,
                fy,
            );
            pull(b, Pull::Cup, x1 - tube - side + 0.07, h - 0.12, d);
            books(
                b,
                &mut rng,
                x0 + tube + 0.03,
                x1 - tube - side - 0.03,
                d - 0.04,
                0.08 + tube / 2.0 + 0.03,
                h - 0.2,
                d - 0.1,
            );
        }
        // Classic: raised panel doors and a drawer, toupie feet, a moulded
        // top.
        8 => {
            let z0 = 0.1;
            for q in corners(x0, x1, d, 0.05) {
                toupie(b, q, z0);
            }
            base_moulding(b, "frame", x0, x1, fy, z0, 0.01);
            b.rbox("frame", [x0, 0.0, z0], [x1, fy, h - 0.03], 0.004);
            cornice(b, "frame", x0, x1, fy, h - 0.03, 0.01);
            b.rbox(
                "top",
                [x0 - 0.015, 0.0, h - 0.03],
                [x1 + 0.015, d + 0.01, h],
                0.006,
            );
            let mid = w * 0.3;
            doors(
                b,
                Panel::Raised,
                "front",
                x0 + 0.04,
                -mid / 2.0,
                z0 + 0.06,
                h - 0.06,
                fy,
                1,
                Pull::Brass,
                (z0 + h) / 2.0,
            );
            drawers(
                b,
                Panel::Raised,
                "front",
                -mid / 2.0 + 0.004,
                mid / 2.0 - 0.004,
                z0 + 0.06,
                h - 0.06,
                fy,
                1,
                Pull::Brass,
            );
            doors(
                b,
                Panel::Raised,
                "front",
                mid / 2.0,
                x1 - 0.04,
                z0 + 0.06,
                h - 0.06,
                fy,
                1,
                Pull::Brass,
                (z0 + h) / 2.0,
            );
        }
        // Nordic: a low oak bench on tapered legs, two drawers and an open
        // space.
        _ => {
            let z0 = 0.17;
            for (q, o) in corners(x0, x1, d, 0.08).iter().zip(OUT) {
                tapered_leg(b, "frame", *q, z0 + 0.005, o, 0.025, 0.017);
            }
            open_carcass(b, "frame", x0, x1, d, z0, h, 0.02);
            let split = x0 + w * 0.45;
            b.rbox(
                "frame",
                [split - 0.01, 0.0, z0 + 0.02],
                [split + 0.01, d, h - 0.02],
                0.0,
            );
            b.rbox(
                "frame",
                [split + 0.01, 0.0, z0 + 0.02],
                [x1 - 0.02, fy, h - 0.02],
                0.0,
            );
            let n = 2;
            let step = (x1 - 0.02 - split - 0.01) / n as f64;
            for i in 0..n {
                let a = split + 0.01 + step * i as f64;
                drawers(
                    b,
                    Panel::Flat,
                    "front",
                    a,
                    a + step,
                    z0 + 0.022,
                    h - 0.022,
                    fy,
                    1,
                    Pull::Slot,
                );
            }
            books(
                b,
                &mut rng,
                x0 + 0.04,
                split - 0.03,
                d - 0.03,
                z0 + 0.02,
                h - z0 - 0.06,
                d - 0.1,
            );
        }
    }
    if p.flag("television", true) {
        television(b, tv, d * 0.4, h);
    }
}

/// A glass cabinet of a collection, its shelves dressed with ceramics.
pub fn vitrine(b: &mut Builder, p: &Params) {
    let (w, d, h) = dims(p, 0.9, 0.42, 1.85);
    let (x0, x1) = (-w / 2.0, w / 2.0);
    let fy = d - T;
    let mut rng = Rng(p.get("seed", 5.0) as u32 + 51);
    // Glass shelves evenly between `za` and `zb`, dressed.
    let glass_shelves =
        |b: &mut Builder, rng: &mut Rng, xa: f64, xb: f64, za: f64, zb: f64, n: usize| {
            let step = (zb - za) / n as f64;
            for i in 0..n {
                let z = za + step * i as f64;
                if i > 0 {
                    b.rbox("glass", [xa, 0.02, z - 0.006], [xb, fy - 0.01, z], 0.0);
                }
                display(b, rng, xa, xb, fy - 0.02, z, step - 0.03);
            }
        };
    match p.style() {
        // Mediterranean: whitewashed, glazed doors in small panes above,
        // cane doors below, a cornice.
        2 => {
            let low = 0.75;
            b.rbox("frame", [x0, 0.0, 0.0], [x1, fy, low], 0.004);
            b.rbox(
                "dark",
                [x0 + 0.03, fy - 0.002, 0.0],
                [x1 - 0.03, fy + 0.001, 0.06],
                0.0,
            );
            doors(
                b,
                Panel::Cane,
                "front",
                x0 + 0.03,
                x1 - 0.03,
                0.08,
                low - 0.03,
                fy,
                2,
                Pull::Knob,
                low - 0.15,
            );
            b.rbox(
                "top",
                [x0 - 0.01, 0.0, low],
                [x1 + 0.01, d, low + 0.03],
                0.006,
            );
            open_carcass(
                b,
                "frame",
                x0 + 0.02,
                x1 - 0.02,
                fy,
                low + 0.03,
                h - 0.04,
                0.025,
            );
            glass_shelves(
                b,
                &mut rng,
                x0 + 0.045,
                x1 - 0.045,
                low + 0.055,
                h - 0.065,
                3,
            );
            doors(
                b,
                Panel::Grid(2, 3),
                "front",
                x0 + 0.02,
                x1 - 0.02,
                low + 0.03,
                h - 0.04,
                fy,
                2,
                Pull::Knob,
                low + 0.25,
            );
            cornice(b, "frame", x0 + 0.02, x1 - 0.02, fy, h, 0.02);
        }
        // Fifties: walnut, sliding glass over a fluted cupboard, on compass
        // legs.
        3 => {
            let (z0, low) = (0.22, 0.75);
            for (q, o) in corners(x0, x1, d, 0.07).iter().zip(OUT) {
                compass_leg(b, *q, z0 + 0.01, o, 0.035);
            }
            b.rbox("frame", [x0, 0.0, z0], [x1, fy, low], 0.012);
            front(
                b,
                Panel::Flutes,
                "front",
                x0 + 0.02,
                0.006,
                z0 + 0.02,
                low - 0.02,
                fy - 0.004,
            );
            front(
                b,
                Panel::Flutes,
                "front",
                -0.006,
                x1 - 0.02,
                z0 + 0.02,
                low - 0.02,
                fy + 0.012,
            );
            pull(b, Pull::Brass, x0 + 0.06, (z0 + low) / 2.0, fy + T - 0.004);
            pull(b, Pull::Brass, x1 - 0.06, (z0 + low) / 2.0, fy + T + 0.012);
            open_carcass(b, "frame", x0, x1, fy, low, h, 0.022);
            glass_shelves(
                b,
                &mut rng,
                x0 + 0.022,
                x1 - 0.022,
                low + 0.022,
                h - 0.022,
                3,
            );
            b.rbox(
                "glass",
                [x0 + 0.02, fy, low + 0.022],
                [0.01, fy + 0.006, h - 0.022],
                0.0,
            );
            b.rbox(
                "glass",
                [-0.01, fy + 0.008, low + 0.022],
                [x1 - 0.02, fy + 0.014, h - 0.022],
                0.0,
            );
            for x in [0.0, x1 - 0.04] {
                b.rbox(
                    "metal",
                    [x - 0.002, fy + 0.014, (low + h) / 2.0 - 0.05],
                    [x + 0.002, fy + 0.018, (low + h) / 2.0 + 0.05],
                    0.001,
                );
            }
        }
        // Japandi: dark ash, tall and narrow, one glass door, slatted
        // sides, on a plinth.
        4 => {
            b.rbox(
                "dark",
                [x0 + 0.03, 0.03, 0.0],
                [x1 - 0.03, d - 0.04, 0.06],
                0.0,
            );
            open_carcass(b, "frame", x0, x1, fy, 0.06, h, 0.025);
            for x in [x0, x1 - 0.004] {
                for k in 0..((fy / 0.03) as usize) {
                    let y = 0.015 + 0.03 * k as f64;
                    b.rbox(
                        "frame",
                        [x - 0.008, y, 0.07],
                        [x + 0.012, y + 0.018, h - 0.01],
                        0.002,
                    );
                }
            }
            glass_shelves(b, &mut rng, x0 + 0.025, x1 - 0.025, 0.085, h - 0.025, 4);
            doors(
                b,
                Panel::Glass,
                "front",
                x0,
                x1,
                0.06,
                h,
                fy,
                1,
                Pull::Tab,
                h * 0.55,
            );
        }
        // Contemporary: a thin black steel frame round glass on every side.
        5 => {
            let tube = 0.016;
            steel_frame(b, x0, x1, d, h, &[0.06, h - tube / 2.0], tube, false);
            b.rbox(
                "metal",
                [x0 + tube, tube, 0.06],
                [x1 - tube, d - tube, 0.06 + 0.012],
                0.0,
            );
            for (lo, hi) in [
                ([x0 + 0.002, tube, 0.06], [x0 + 0.008, d - tube, h - tube]),
                ([x1 - 0.008, tube, 0.06], [x1 - 0.002, d - tube, h - tube]),
                ([x0 + tube, 0.002, 0.06], [x1 - tube, 0.008, h - tube]),
                (
                    [x0 + tube, d - 0.008, 0.06],
                    [x1 - tube, d - 0.002, h - tube],
                ),
            ] {
                b.rbox("glass", lo, hi, 0.0);
            }
            glass_shelves(b, &mut rng, x0 + tube, x1 - tube, 0.072, h - tube, 4);
            b.rbox(
                "metal",
                [-0.002, d, h * 0.45],
                [0.002, d + 0.012, h * 0.6],
                0.001,
            );
        }
        // Rustic: pine, glazed doors in panes above, plank doors below, a
        // cornice.
        6 => {
            let low = 0.8;
            b.rbox("frame", [x0, 0.0, 0.0], [x1, fy, low], 0.006);
            doors(
                b,
                Panel::Planks,
                "front",
                x0 + 0.03,
                x1 - 0.03,
                0.08,
                low - 0.03,
                fy,
                2,
                Pull::Ring,
                low - 0.14,
            );
            b.rbox("frame", [x0 + 0.03, fy, 0.0], [x1 - 0.03, d, 0.07], 0.003);
            b.rbox(
                "top",
                [x0 - 0.02, 0.0, low],
                [x1 + 0.02, d + 0.02, low + 0.035],
                0.008,
            );
            open_carcass(
                b,
                "frame",
                x0 + 0.03,
                x1 - 0.03,
                fy,
                low + 0.035,
                h - 0.05,
                0.03,
            );
            glass_shelves(b, &mut rng, x0 + 0.06, x1 - 0.06, low + 0.065, h - 0.08, 3);
            doors(
                b,
                Panel::Grid(1, 3),
                "front",
                x0 + 0.03,
                x1 - 0.03,
                low + 0.035,
                h - 0.05,
                fy,
                2,
                Pull::Knob,
                low + 0.3,
            );
            b.rbox(
                "frame",
                [x0 - 0.01, 0.0, h - 0.05],
                [x1 + 0.01, d + 0.02, h],
                0.008,
            );
        }
        // Industrial: a riveted steel frame, glass doors in steel bars,
        // wooden shelves and top.
        7 => {
            let tube = 0.03;
            steel_frame(
                b,
                x0,
                x1,
                d,
                h - 0.035,
                &[0.08, h - 0.035 - tube / 2.0],
                tube,
                false,
            );
            b.rbox("top", [x0, 0.0, h - 0.035], [x1, d, h], 0.003);
            b.rbox(
                "top",
                [x0 + tube, tube, 0.08],
                [x1 - tube, d - tube, 0.11],
                0.003,
            );
            for x in [x0 + 0.002, x1 - 0.008] {
                b.rbox(
                    "glass",
                    [x, tube, 0.11],
                    [x + 0.006, d - tube, h - 0.035 - tube],
                    0.0,
                );
            }
            b.rbox(
                "glass",
                [x0 + tube, 0.002, 0.11],
                [x1 - tube, 0.008, h - 0.035 - tube],
                0.0,
            );
            glass_shelves(b, &mut rng, x0 + tube, x1 - tube, 0.11, h - 0.035 - tube, 4);
            let step = w / 2.0 - tube;
            for i in 0..2 {
                let a = x0 + tube + step * i as f64 + 0.002;
                front(
                    b,
                    Panel::Grid(1, 4),
                    "metal",
                    a,
                    a + step - 0.004,
                    0.11,
                    h - 0.035 - tube - 0.003,
                    fy,
                );
                pull(
                    b,
                    Pull::Bar(0.18),
                    if i == 0 { a + step - 0.03 } else { a + 0.03 },
                    h * 0.5,
                    d,
                );
            }
            for x in [x0 + tube / 2.0, x1 - tube / 2.0] {
                rivets(b, [x, d + 0.001, 0.15], [x, d + 0.001, h - 0.1], 8);
            }
        }
        // Classic: walnut, glazed doors with a round head of bars, raised
        // panels below, toupie feet, a cornice.
        8 => {
            let (z0, low) = (0.1, 0.8);
            for q in corners(x0, x1, d, 0.05) {
                toupie(b, q, z0);
            }
            base_moulding(b, "frame", x0, x1, fy, z0, 0.012);
            b.rbox("frame", [x0, 0.0, z0], [x1, fy, low], 0.004);
            doors(
                b,
                Panel::Raised,
                "front",
                x0 + 0.04,
                x1 - 0.04,
                z0 + 0.07,
                low - 0.04,
                fy,
                2,
                Pull::Brass,
                low - 0.14,
            );
            b.rbox(
                "frame",
                [x0 - 0.015, 0.0, low],
                [x1 + 0.015, d + 0.01, low + 0.03],
                0.006,
            );
            open_carcass(
                b,
                "frame",
                x0 + 0.02,
                x1 - 0.02,
                fy,
                low + 0.03,
                h - 0.06,
                0.025,
            );
            glass_shelves(
                b,
                &mut rng,
                x0 + 0.045,
                x1 - 0.045,
                low + 0.055,
                h - 0.085,
                3,
            );
            let (za, zb) = (low + 0.03, h - 0.06);
            doors(
                b,
                Panel::Grid(2, 4),
                "front",
                x0 + 0.02,
                x1 - 0.02,
                za,
                zb,
                fy,
                2,
                Pull::Brass,
                za + 0.3,
            );
            // A half-round bar at the head of each door.
            let half = (w - 0.04) / 2.0;
            for i in 0..2 {
                let cx = x0 + 0.02 + half * (i as f64 + 0.5);
                let r = half / 2.0 - 0.05;
                let path: Vec<P3> = (0..=12)
                    .map(|k| {
                        let a = PI * k as f64 / 12.0;
                        [cx - r * a.cos(), fy + T * 0.6, zb - 0.07 - r + r * a.sin()]
                    })
                    .collect();
                sweep(b, "front", &path, |_| 0.006, 6, Cap::Flat, Cap::Flat);
            }
            for x in [x0 + 0.02, x1 - 0.02] {
                front(
                    b,
                    Panel::Flutes,
                    "frame",
                    x - 0.02,
                    x + 0.02,
                    low + 0.05,
                    h - 0.08,
                    fy + T,
                );
            }
            cornice(b, "frame", x0 + 0.02, x1 - 0.02, fy, h, 0.025);
        }
        // Nordic: oak on tapered legs, two glass doors in a thin frame.
        _ => {
            let z0 = 0.2;
            for (q, o) in corners(x0, x1, d, 0.06).iter().zip(OUT) {
                tapered_leg(b, "frame", *q, z0 + 0.005, o, 0.02, 0.017);
            }
            open_carcass(b, "frame", x0, x1, fy, z0, h, 0.022);
            glass_shelves(
                b,
                &mut rng,
                x0 + 0.022,
                x1 - 0.022,
                z0 + 0.022,
                h - 0.022,
                4,
            );
            doors(
                b,
                Panel::Glass,
                "front",
                x0,
                x1,
                z0,
                h,
                fy,
                2,
                Pull::Slot,
                0.0,
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::Params;

    #[test]
    fn the_collection_storage_is_made() {
        for kind in [
            "fine_sideboard",
            "fine_bookcase",
            "fine_tv_unit",
            "fine_vitrine",
        ] {
            for style in 1..=8 {
                let mut p = Params::default();
                p.set("style", style as f64);
                let m = crate::make(kind, &p).unwrap();
                let n = m.triangle_count();
                println!("{kind} {style}: {n}");
                assert!((200..=60_000).contains(&n), "{kind} {style}: {n}");
            }
        }
    }
}
