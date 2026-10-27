//! The workshop's kitchens, one for each collection (paquete 2a): the
//! library's modular kitchen — base modules, tall units, wall units and
//! hoods with the same sizes, the same «what it holds» and the same
//! worktop that runs on from module to module — made the way each
//! collection makes a kitchen: its fronts and pulls, what it stands on,
//! its sink and tap, its hob, its hood, and cupboards or open shelves on
//! the wall.
//!
//! Axes as every piece of furniture's: the back on the wall at `y = 0`,
//! the front looking to `+y`, centred on `x = 0`.
//!
//! Roles: `mat` fronts and carcass, `plinth` the kick board, `top` the
//! worktop, `metal` pulls, taps, legs and steel, `sink` the basin, `dark`
//! shadow gaps and leather, `black` a hob's top, an oven's and a
//! microwave's face, `ring` the marks of a hob, `glass`, `ceramic` jars
//! and jugs, `hood` a hood's body, `wood` legs, shelves, beams and
//! brackets of wood.

use super::shape::{bar, curve, loft_loop, sweep, turned, Cap};
use super::storage::{self, Panel, Pull, T};
use crate::build::{Builder, P3};
use crate::Params;
use std::f64::consts::PI;

/// The collections, in the order the kitchen families list them after
/// their own first model.
pub const COLLECTIONS: [&str; 8] = [
    "Nórdico cálido",
    "Mediterráneo",
    "Años 50",
    "Japandi",
    "Contemporáneo",
    "Rústica",
    "Industrial",
    "Clásica",
];

/// What a kitchen stands on.
#[derive(Clone, Copy, PartialEq)]
enum Base {
    /// A kick board set back under the fronts.
    Plinth,
    /// A board flush with the fronts, with a moulding (classic).
    Skirting,
    /// Tapered legs with brass shoes (the fifties).
    Legs,
    /// A frame of steel tube on the floor (industrial).
    Steel,
}

/// The sink under the worktop's opening.
#[derive(Clone, Copy, PartialEq)]
enum Sink {
    /// A steel basin set in, its rim on the worktop.
    Inset,
    /// A basin under the worktop, the worktop's edge round it.
    Under,
    /// A thick fireclay basin under the worktop, its front showing.
    Ceramic,
}

/// The tap.
#[derive(Clone, Copy, PartialEq)]
pub(super) enum Tap {
    /// A gooseneck mixer of one piece.
    Gooseneck,
    /// A curved spout and a lever on its top.
    Swan,
    /// Two cross handles on a bridge, the spout from its middle.
    Bridge,
    /// Two knobs on a deck plate and a swivel spout (the fifties).
    Twin,
    /// Square tube bent at right angles.
    Square,
    /// A slim tube bent forward, a pin lever.
    Slim,
    /// A pull-down spray in a coil spring.
    Spring,
}

/// The hob.
#[derive(Clone, Copy, PartialEq)]
enum Hob {
    Induction,
    Gas,
}

/// The hood.
#[derive(Clone, Copy, PartialEq)]
enum Hood {
    /// A box canopy and a chimney.
    Chimney,
    /// A plastered hood, sloping in to its chimney, a ledge at its lip.
    Mantel,
    /// The plastered hood on an oak beam.
    Beam,
    /// The plastered hood with a cornice.
    Cornice,
    /// A curved canopy with a chrome band.
    Curved,
    /// A box clad as the fronts, between the wall units.
    Built,
    /// A slim flat canopy on a slim chimney.
    Slim,
    /// A steel cone on a round flue.
    Tube,
}

/// What hangs on the wall over the worktop.
#[derive(Clone, Copy, PartialEq)]
enum Upper {
    /// Cupboards with the collection's fronts.
    Cupboards,
    /// Glazed cupboards, glazing bars and a cornice.
    Vitrine,
    /// Shelves on plaster corbels.
    Corbels,
    /// Shelves with nothing seen holding them.
    Floating,
    /// Planks on iron brackets with a rail.
    Brackets,
    /// Boards on steel pipes.
    Pipes,
}

/// How a collection makes a kitchen.
#[derive(Clone, Copy)]
struct Look {
    panel: Panel,
    pull: Pull,
    /// Handleless, with a recessed channel above each front.
    gola: bool,
    /// Fronts with their edges well rounded (the fifties).
    rounded: bool,
    base: Base,
    sink: Sink,
    tap: Tap,
    hob: Hob,
    hood: Hood,
    upper: Upper,
    /// A cornice on the tall and wall units.
    cornice: bool,
}

/// The eight looks, in the order of [`COLLECTIONS`].
const LOOKS: [Look; 8] = [
    // Nórdico cálido.
    Look {
        panel: Panel::Flat,
        pull: Pull::Tab,
        gola: false,
        rounded: false,
        base: Base::Plinth,
        sink: Sink::Inset,
        tap: Tap::Gooseneck,
        hob: Hob::Induction,
        hood: Hood::Chimney,
        upper: Upper::Cupboards,
        cornice: false,
    },
    // Mediterráneo.
    Look {
        panel: Panel::Planks,
        pull: Pull::Ring,
        gola: false,
        rounded: false,
        base: Base::Skirting,
        sink: Sink::Ceramic,
        tap: Tap::Swan,
        hob: Hob::Gas,
        hood: Hood::Mantel,
        upper: Upper::Corbels,
        cornice: false,
    },
    // Años 50.
    Look {
        panel: Panel::Flat,
        pull: Pull::Bar(0.3),
        gola: false,
        rounded: true,
        base: Base::Legs,
        sink: Sink::Inset,
        tap: Tap::Twin,
        hob: Hob::Gas,
        hood: Hood::Curved,
        upper: Upper::Cupboards,
        cornice: false,
    },
    // Japandi.
    Look {
        panel: Panel::Flat,
        pull: Pull::Slot,
        gola: false,
        rounded: false,
        base: Base::Plinth,
        sink: Sink::Under,
        tap: Tap::Slim,
        hob: Hob::Induction,
        hood: Hood::Built,
        upper: Upper::Floating,
        cornice: false,
    },
    // Contemporáneo.
    Look {
        panel: Panel::Flat,
        pull: Pull::None,
        gola: true,
        rounded: false,
        base: Base::Plinth,
        sink: Sink::Under,
        tap: Tap::Square,
        hob: Hob::Induction,
        hood: Hood::Slim,
        upper: Upper::Cupboards,
        cornice: false,
    },
    // Rústica.
    Look {
        panel: Panel::Shaker,
        pull: Pull::Cup,
        gola: false,
        rounded: false,
        base: Base::Skirting,
        sink: Sink::Ceramic,
        tap: Tap::Bridge,
        hob: Hob::Gas,
        hood: Hood::Beam,
        upper: Upper::Brackets,
        cornice: false,
    },
    // Industrial.
    Look {
        panel: Panel::Flat,
        pull: Pull::Bar(0.22),
        gola: false,
        rounded: false,
        base: Base::Steel,
        sink: Sink::Under,
        tap: Tap::Spring,
        hob: Hob::Gas,
        hood: Hood::Tube,
        upper: Upper::Pipes,
        cornice: false,
    },
    // Clásica.
    Look {
        panel: Panel::Raised,
        pull: Pull::Brass,
        gola: false,
        rounded: false,
        base: Base::Skirting,
        sink: Sink::Ceramic,
        tap: Tap::Bridge,
        hob: Hob::Gas,
        hood: Hood::Cornice,
        upper: Upper::Vitrine,
        cornice: true,
    },
];

/// The collection's look when the model chosen (`style`) is one of the
/// collections, which a kitchen family lists after its `first` own
/// models.
fn look(p: &Params, first: i32) -> Option<Look> {
    let s = p.style() - first;
    (1..=8).contains(&s).then(|| LOOKS[(s - 1) as usize])
}

/// The worktop's thickness (the library's).
const TOP: f64 = crate::WORKTOP;

/// How high the carcass starts, over what it stands on.
fn foot(look: &Look) -> f64 {
    match look.base {
        Base::Legs => 0.15,
        Base::Steel => 0.12,
        _ => 0.1,
    }
}

/// What a carcass from `x0` to `x1`, its face at `yc`, stands on.
fn stand(b: &mut Builder, look: &Look, x0: f64, x1: f64, yc: f64) {
    let z = foot(look);
    match look.base {
        Base::Plinth => b.rbox(
            "plinth",
            [x0 + 0.002, 0.0, 0.0],
            [x1 - 0.002, yc - 0.06, z],
            0.0,
        ),
        Base::Skirting => {
            b.rbox(
                "mat",
                [x0 + 0.001, 0.0, 0.0],
                [x1 - 0.001, yc + T - 0.004, z],
                0.002,
            );
            b.rbox(
                "mat",
                [x0 + 0.001, yc, z - 0.022],
                [x1 - 0.001, yc + T + 0.006, z - 0.006],
                0.004,
            );
        }
        Base::Legs => {
            // A board set well back, tapered legs at the front corners and
            // plain ones behind.
            b.rbox(
                "dark",
                [x0 + 0.04, 0.02, 0.0],
                [x1 - 0.04, yc - 0.12, z],
                0.0,
            );
            for (x, dir) in [(x0 + 0.05, -1.0), (x1 - 0.05, 1.0)] {
                storage::tapered_leg(b, "wood", [x, yc - 0.03], z, [dir * 0.3, 0.7], 0.012, 0.017);
                // The brass shoe.
                turned(
                    b,
                    "metal",
                    [x + dir * 0.0036, yc - 0.03 + 0.0084, 0.0],
                    &[(0.0, 0.0), (0.0115, 0.0), (0.0118, 0.028), (0.0, 0.028)],
                    14,
                    0.0,
                );
            }
        }
        Base::Steel => {
            let s = 0.03;
            for x in [x0 + s / 2.0 + 0.004, x1 - s / 2.0 - 0.004] {
                for y in [0.04, yc - 0.03] {
                    bar(b, "metal", [x, y, 0.0], [x, y, z], s, [1.0, 0.0, 0.0]);
                }
                bar(
                    b,
                    "metal",
                    [x, 0.04 - s / 2.0, z - s / 2.0],
                    [x, yc - 0.03 + s / 2.0, z - s / 2.0],
                    s,
                    [1.0, 0.0, 0.0],
                );
            }
            bar(
                b,
                "metal",
                [x0 + 0.004, yc - 0.03, z - s / 2.0],
                [x1 - 0.004, yc - 0.03, z - s / 2.0],
                s,
                [0.0, 0.0, 1.0],
            );
        }
    }
}

/// One front from `x0` to `x1` and `z0` to `z1` on a face at `y`.
#[allow(clippy::too_many_arguments)]
fn front(b: &mut Builder, look: &Look, x0: f64, x1: f64, z0: f64, z1: f64, y: f64) {
    if look.rounded {
        let r = 0.016_f64.min((x1 - x0) * 0.2).min((z1 - z0) * 0.2);
        b.rbox("mat", [x0, y, z0], [x1, y + T, z1], r);
        // The fifties' chrome edging: a thin band round the face.
        return;
    }
    if look.base == Base::Steel {
        // Industrial: a board in a frame of flat steel.
        b.rbox("mat", [x0, y, z0], [x1, y + T, z1], 0.002);
        let k = 0.012;
        for (lo, hi) in [
            ([x0, y + T, z0], [x1, y + T + 0.004, z0 + k]),
            ([x0, y + T, z1 - k], [x1, y + T + 0.004, z1]),
            ([x0, y + T, z0 + k], [x0 + k, y + T + 0.004, z1 - k]),
            ([x1 - k, y + T, z0 + k], [x1, y + T + 0.004, z1 - k]),
        ] {
            b.rbox("metal", lo, hi, 0.0);
        }
        return;
    }
    storage::front(b, look.panel, "mat", x0, x1, z0, z1, y);
}

/// The pull of a front at `(x, z)`, its face at `y`; `upright` on a door.
fn pull(b: &mut Builder, look: &Look, x: f64, z: f64, y: f64, upright: bool) {
    let face = y + T + if look.base == Base::Steel { 0.004 } else { 0.0 };
    match look.pull {
        Pull::Bar(l) if upright => {
            let start = b.mark();
            storage::pull(b, Pull::Bar(l), 0.0, 0.0, 0.0);
            b.transform_from(start, |q| [x - q[2], face + q[1], z + q[0]]);
        }
        kind => storage::pull(b, kind, x, z, face),
    }
}

/// A recessed channel along the top of fronts from `x0` to `x1`, from `z`
/// up `h`, behind a face at `y` (the handleless kitchen's gola).
fn channel(b: &mut Builder, x0: f64, x1: f64, z: f64, h: f64, y: f64) {
    b.rbox("metal", [x0, y - 0.03, z], [x1, y - 0.026, z + h], 0.0);
    b.rbox(
        "metal",
        [x0, y - 0.03, z],
        [x1, y + T * 0.5, z + 0.003],
        0.0,
    );
}

/// Drawers stacked from `z0` to `z1`, the top one shallow when there are
/// three or more, as a kitchen's.
#[allow(clippy::too_many_arguments)]
fn drawers(b: &mut Builder, look: &Look, x0: f64, x1: f64, z0: f64, z1: f64, y: f64, n: usize) {
    let n = n.max(1);
    let gap = if look.gola { 0.035 } else { 0.003 };
    let total = z1 - z0 - gap * (n - 1) as f64;
    let first = if n >= 3 {
        total * 0.2
    } else {
        total / n as f64
    };
    let rest = if n >= 3 {
        (total - first) / (n - 1) as f64
    } else {
        first
    };
    let mut top = z1;
    for i in 0..n {
        let h = if i == 0 { first } else { rest };
        let (a, e) = (top - h, top);
        front(b, look, x0 + 0.0015, x1 - 0.0015, a, e, y);
        if look.gola && i > 0 {
            channel(b, x0, x1, e, gap, y);
        }
        let pz = match look.pull {
            Pull::Slot => e - 0.001,
            Pull::Bar(_) if look.rounded => e - 0.035,
            _ => e - (h * 0.5).min(0.06),
        };
        if !look.gola {
            pull(b, look, (x0 + x1) / 2.0, pz, y, false);
        }
        top = a - gap;
    }
}

/// `n` doors side by side from `x0` to `x1`, their pulls near the top
/// (`high`) or the bottom.
#[allow(clippy::too_many_arguments)]
fn doors(
    b: &mut Builder,
    look: &Look,
    x0: f64,
    x1: f64,
    z0: f64,
    z1: f64,
    y: f64,
    n: usize,
    high: bool,
) {
    let n = n.max(1);
    let step = (x1 - x0) / n as f64;
    for i in 0..n {
        let (a, e) = (
            x0 + step * i as f64 + 0.0015,
            x0 + step * (i + 1) as f64 - 0.0015,
        );
        if look.upper == Upper::Vitrine && !high && z1 - z0 < 1.2 && z0 > 1.0 {
            storage::front(b, Panel::Grid(2, 3), "mat", a, e, z0, z1, y);
        } else {
            front(b, look, a, e, z0, z1, y);
        }
        if look.gola {
            continue;
        }
        let right = if n.is_multiple_of(2) {
            i < n / 2
        } else {
            i.is_multiple_of(2)
        };
        let inset = match look.pull {
            Pull::Bar(_) if look.rounded => 0.0,
            Pull::Bar(_) | Pull::Cup => 0.032,
            Pull::Slot => 0.06,
            _ => 0.042,
        };
        let px = if right { e - inset } else { a + inset };
        let pz = if high { z1 - 0.07 } else { z0 + 0.07 };
        match look.pull {
            // The fifties: a long chrome bar across the top of each door.
            Pull::Bar(_) if look.rounded => {
                let l = ((e - a) * 0.55).min(0.32);
                storage::pull(b, Pull::Bar(l), (a + e) / 2.0, z1 - 0.04, y + T);
            }
            Pull::Slot => {
                let z = if high { z1 - 0.001 } else { z0 + 0.012 };
                pull(b, look, px, z, y, true);
            }
            _ => pull(b, look, px, pz, y, true),
        }
    }
}

// -------------------------------------------------------------- sinks

/// The basin under an opening of half sides `a`, `bb` about `c` in a
/// worktop whose top is at `h`.
fn basin(b: &mut Builder, look: &Look, c: [f64; 2], a: f64, bb: f64, h: f64) {
    match look.sink {
        Sink::Inset => super::sink::basin(b, c, a, bb, h),
        Sink::Under | Sink::Ceramic => {
            let ceramic = look.sink == Sink::Ceramic;
            let zt = h - TOP;
            let (r, wall, deep) = if ceramic {
                (0.045, 0.022, 0.24)
            } else {
                (0.012, 0.004, 0.21)
            };
            let fl = if ceramic { 0.0 } else { 0.014 };
            // Round the section of the basin's wall: from under the
            // worktop, in at the opening, down the inside, across the floor
            // to the waste, back under and up the outside.
            let rings = [
                (a + wall + fl, bb + wall + fl, r + wall + fl, zt - 0.003),
                (a + wall + fl, bb + wall + fl, r + wall + fl, zt),
                (a, bb, r, zt),
                (a, bb, r, zt - 0.012),
                (a - 0.004, bb - 0.004, r, zt - deep + 0.02),
                (a - 0.02, bb - 0.02, r * 0.7, zt - deep),
                (0.045, 0.045, 0.045, zt - deep - 0.004),
                (0.045, 0.045, 0.045, zt - deep - 0.007 - wall * 0.5),
                (
                    a - 0.02 + wall,
                    bb - 0.02 + wall,
                    r * 0.7 + wall,
                    zt - deep - wall,
                ),
                (
                    a - 0.004 + wall,
                    bb - 0.004 + wall,
                    r + wall,
                    zt - deep + 0.02,
                ),
                (a + wall, bb + wall, r + wall, zt - 0.012),
            ];
            loft_loop(b, "sink", c, &rings);
            // The waste.
            turned(
                b,
                "metal",
                [c[0], c[1], zt - deep - 0.004],
                &[(0.0, -0.002), (0.042, -0.002), (0.044, 0.002), (0.0, 0.003)],
                24,
                0.0,
            );
        }
    }
}

/// The tap standing at `at` on the worktop behind the basin, its spout
/// reaching `reach` forward.
fn tap(b: &mut Builder, look: &Look, at: P3, reach: f64) {
    tap_kind(b, look.tap, at, reach);
}

/// A tap of `kind` standing at `at`, scaled `k` (1: a kitchen's; less: a
/// washbasin's), its spout reaching `reach` forward (before scaling).
pub(super) fn tap_scaled(b: &mut Builder, kind: Tap, at: P3, reach: f64, k: f64) {
    let start = b.mark();
    tap_kind(b, kind, [0.0; 3], reach);
    b.transform_from(start, |q| {
        [at[0] + q[0] * k, at[1] + q[1] * k, at[2] + q[2] * k]
    });
}

/// A tap of `kind` standing at `at`.
fn tap_kind(b: &mut Builder, kind: Tap, at: P3, reach: f64) {
    let [x, y, z] = at;
    match kind {
        Tap::Gooseneck => super::sink::tap_in(b, "metal", at, reach),
        Tap::Swan => {
            // A turned body, the spout rising out of it in an S and
            // dropping over the bowl, the lever on top.
            turned(
                b,
                "metal",
                at,
                &[
                    (0.0, 0.0),
                    (0.027, 0.0),
                    (0.027, 0.006),
                    (0.022, 0.012),
                    (0.02, 0.09),
                    (0.023, 0.1),
                    (0.0, 0.104),
                ],
                24,
                0.0,
            );
            let path = curve(
                &[
                    [x, y, z + 0.085],
                    [x, y + 0.01, z + 0.16],
                    [x, y + 0.07, z + 0.26],
                    [x, y + reach * 0.75, z + 0.27],
                    [x, y + reach, z + 0.21],
                ],
                5,
            );
            sweep(
                b,
                "metal",
                &path,
                |t| 0.012 - 0.003 * t,
                16,
                Cap::Flat,
                Cap::Flat,
            );
            let lever = curve(
                &[
                    [x, y, z + 0.1],
                    [x, y - 0.02, z + 0.115],
                    [x, y - 0.07, z + 0.125],
                ],
                4,
            );
            sweep(
                b,
                "metal",
                &lever,
                |t| 0.006 - 0.002 * t,
                12,
                Cap::Flat,
                Cap::Round(1.0),
            );
        }
        Tap::Bridge | Tap::Twin => {
            let bridge = kind == Tap::Bridge;
            let s = if bridge { 0.1 } else { 0.08 };
            if !bridge {
                // The deck plate.
                b.rbox(
                    "metal",
                    [x - s - 0.035, y - 0.03, z],
                    [x + s + 0.035, y + 0.03, z + 0.008],
                    0.004,
                );
            }
            for sx in [x - s, x + s] {
                let base = [sx, y, z + if bridge { 0.0 } else { 0.008 }];
                let hgt = if bridge { 0.1 } else { 0.035 };
                turned(
                    b,
                    "metal",
                    base,
                    &[
                        (0.0, 0.0),
                        (0.022, 0.0),
                        (0.022, 0.005),
                        (0.014, 0.012),
                        (0.013, hgt),
                        (0.018, hgt + 0.006),
                        (0.0, hgt + 0.01),
                    ],
                    20,
                    0.0,
                );
                let top = [sx, y, base[2] + hgt + 0.008];
                if bridge {
                    // A cross handle: four arms with round ends.
                    for k in 0..4 {
                        let ang = PI / 4.0 + PI / 2.0 * k as f64;
                        let e = [
                            top[0] + 0.03 * ang.cos(),
                            top[1] + 0.03 * ang.sin(),
                            top[2] + 0.012,
                        ];
                        sweep(
                            b,
                            "metal",
                            &[top, e],
                            |_| 0.0045,
                            10,
                            Cap::Flat,
                            Cap::Round(1.4),
                        );
                    }
                } else {
                    // A round knob, fluted.
                    turned(
                        b,
                        "metal",
                        top,
                        &[
                            (0.0, 0.0),
                            (0.02, 0.0),
                            (0.022, 0.012),
                            (0.017, 0.022),
                            (0.0, 0.024),
                        ],
                        8,
                        0.0,
                    );
                }
            }
            let zb = if bridge { z + 0.07 } else { z + 0.008 };
            if bridge {
                sweep(
                    b,
                    "metal",
                    &[[x - s, y, zb], [x + s, y, zb]],
                    |_| 0.011,
                    16,
                    Cap::Flat,
                    Cap::Flat,
                );
            }
            // The spout from the middle, over the bowl.
            let path = if bridge {
                curve(
                    &[
                        [x, y, zb],
                        [x, y, zb + 0.14],
                        [x, y + reach * 0.4, zb + 0.24],
                        [x, y + reach * 0.85, zb + 0.2],
                        [x, y + reach, zb + 0.13],
                    ],
                    5,
                )
            } else {
                curve(
                    &[
                        [x, y, zb],
                        [x, y, zb + 0.12],
                        [x, y + 0.03, zb + 0.16],
                        [x, y + reach * 0.8, zb + 0.17],
                        [x, y + reach, zb + 0.15],
                    ],
                    5,
                )
            };
            sweep(b, "metal", &path, |_| 0.0105, 16, Cap::Flat, Cap::Flat);
        }
        Tap::Square => {
            let w = 0.022;
            b.rbox("metal", [x - w, y - w, z], [x + w, y + w, z + 0.004], 0.001);
            bar(b, "metal", [x, y, z], [x, y, z + 0.3], w, [1.0, 0.0, 0.0]);
            bar(
                b,
                "metal",
                [x, y - w / 2.0, z + 0.3 - w / 2.0],
                [x, y + reach, z + 0.3 - w / 2.0],
                w,
                [1.0, 0.0, 0.0],
            );
            b.rbox(
                "black",
                [x - 0.008, y + reach - 0.03, z + 0.3 - w - 0.001],
                [x + 0.008, y + reach - 0.006, z + 0.3 - w],
                0.0,
            );
            // The lever: a flat blade on its side.
            b.rbox(
                "metal",
                [x + w / 2.0, y - 0.006, z + 0.12],
                [x + w / 2.0 + 0.06, y + 0.006, z + 0.128],
                0.001,
            );
        }
        Tap::Slim => {
            let r = 0.011;
            turned(
                b,
                "metal",
                at,
                &[
                    (0.0, 0.0),
                    (0.02, 0.0),
                    (0.02, 0.004),
                    (r, 0.006),
                    (r, 0.01),
                    (0.0, 0.012),
                ],
                20,
                0.0,
            );
            let k = 0.05;
            let mut path = vec![[x, y, z + 0.01], [x, y, z + 0.3 - k]];
            for i in 1..=8 {
                let t = PI / 2.0 * i as f64 / 8.0;
                path.push([x, y + k - k * t.cos(), z + 0.3 - k + k * t.sin()]);
            }
            path.push([x, y + reach, z + 0.3]);
            sweep(b, "metal", &path, |_| r, 16, Cap::Flat, Cap::Flat);
            // A pin lever on its side.
            sweep(
                b,
                "metal",
                &[[x + r, y, z + 0.17], [x + 0.06, y, z + 0.17]],
                |_| 0.0035,
                10,
                Cap::Flat,
                Cap::Round(1.0),
            );
        }
        Tap::Spring => {
            // A column, a coil round the hose up to a hook, the spray
            // hanging forward.
            turned(
                b,
                "metal",
                at,
                &[
                    (0.0, 0.0),
                    (0.026, 0.0),
                    (0.026, 0.008),
                    (0.018, 0.014),
                    (0.018, 0.12),
                    (0.014, 0.13),
                    (0.0, 0.132),
                ],
                24,
                0.0,
            );
            let h0 = z + 0.13;
            let h1 = z + 0.42;
            let hose = curve(
                &[
                    [x, y, h0],
                    [x, y, h1 - 0.06],
                    [x, y + 0.02, h1],
                    [x, y + reach * 0.7, h1 + 0.01],
                    [x, y + reach, h1 - 0.06],
                ],
                6,
            );
            sweep(b, "dark", &hose, |_| 0.008, 12, Cap::Flat, Cap::Flat);
            let turns = 9.0;
            let coil: Vec<P3> = (0..=150)
                .map(|i| {
                    let t = i as f64 / 150.0;
                    let a = 2.0 * PI * turns * t;
                    [
                        x + 0.022 * a.cos(),
                        y + 0.022 * a.sin(),
                        h0 + (h1 - 0.08 - h0) * t,
                    ]
                })
                .collect();
            sweep(b, "metal", &coil, |_| 0.003, 8, Cap::Flat, Cap::Flat);
            // The arm that holds the spray.
            sweep(
                b,
                "metal",
                &[
                    [x, y, h1 - 0.08],
                    [x, y, h1 + 0.015],
                    [x, y + reach, h1 + 0.015],
                ],
                |_| 0.006,
                12,
                Cap::Flat,
                Cap::Round(1.0),
            );
            turned(
                b,
                "metal",
                [x, y + reach, h1 - 0.13],
                &[
                    (0.0, 0.0),
                    (0.02, 0.0),
                    (0.02, 0.05),
                    (0.012, 0.07),
                    (0.0, 0.07),
                ],
                20,
                0.0,
            );
        }
    }
}

/// The hob in the middle of a module `w` wide and `d` deep, its top at
/// `h`.
fn hob(b: &mut Builder, look: &Look, w: f64, d: f64, h: f64) {
    let (cx, cy) = (0.0, d / 2.0);
    let hw = (w / 2.0 - 0.04).min(0.38);
    let wide = hw > 0.33;
    match look.hob {
        Hob::Induction => {
            b.rbox(
                "black",
                [cx - hw, cy - 0.25, h],
                [cx + hw, cy + 0.25, h + 0.005],
                0.003,
            );
            let zones: &[(f64, f64, f64)] = if wide {
                &[
                    (-0.2, -0.1, 0.1),
                    (0.0, 0.1, 0.11),
                    (0.2, -0.1, 0.08),
                    (-0.2, 0.13, 0.07),
                    (0.2, 0.13, 0.07),
                ]
            } else {
                &[
                    (-0.13, -0.11, 0.095),
                    (0.13, -0.11, 0.075),
                    (-0.13, 0.12, 0.075),
                    (0.13, 0.12, 0.095),
                ]
            };
            for &(dx, dy, r) in zones {
                // A thin ring marking each zone.
                let ring: Vec<P3> = (0..32)
                    .map(|k| {
                        let a = 2.0 * PI * k as f64 / 32.0;
                        [cx + dx + r * a.cos(), cy + dy + r * a.sin(), h + 0.0052]
                    })
                    .collect();
                super::soft::tube(b, "ring", &ring, true, 0.0011, 4);
            }
            // The touch controls along the front.
            b.rbox(
                "ring",
                [cx - 0.12, cy - 0.235, h + 0.005],
                [cx + 0.12, cy - 0.215, h + 0.0056],
                0.0,
            );
        }
        Hob::Gas => {
            b.rbox(
                "black",
                [cx - hw, cy - 0.25, h],
                [cx + hw, cy + 0.25, h + 0.008],
                0.004,
            );
            let burners: &[(f64, f64, f64)] = if wide {
                &[
                    (-0.21, -0.11, 0.045),
                    (0.0, 0.0, 0.06),
                    (0.21, -0.11, 0.035),
                    (-0.21, 0.13, 0.035),
                    (0.21, 0.13, 0.04),
                ]
            } else {
                &[
                    (-0.13, -0.11, 0.045),
                    (0.13, -0.11, 0.035),
                    (-0.13, 0.12, 0.035),
                    (0.13, 0.12, 0.045),
                ]
            };
            for &(dx, dy, r) in burners {
                let c = [cx + dx, cy + dy, h + 0.008];
                turned(
                    b,
                    "metal",
                    c,
                    &[
                        (0.0, 0.0),
                        (r * 1.25, 0.0),
                        (r * 1.2, 0.012),
                        (r, 0.016),
                        (0.0, 0.018),
                    ],
                    20,
                    0.0,
                );
                turned(
                    b,
                    "black",
                    c,
                    &[
                        (0.0, 0.017),
                        (r * 0.7, 0.017),
                        (r * 0.6, 0.024),
                        (0.0, 0.025),
                    ],
                    16,
                    0.0,
                );
                // The cast iron pan support: four arms on a ring.
                let rr = r * 1.9 + 0.02;
                let ring: Vec<P3> = (0..20)
                    .map(|k| {
                        let a = 2.0 * PI * k as f64 / 20.0;
                        [c[0] + rr * a.cos(), c[1] + rr * a.sin(), c[2] + 0.004]
                    })
                    .collect();
                super::soft::tube(b, "black", &ring, true, 0.004, 5);
                for k in 0..4 {
                    let a = PI / 4.0 + PI / 2.0 * k as f64;
                    let o = [c[0] + rr * a.cos(), c[1] + rr * a.sin(), c[2] + 0.004];
                    let i = [
                        c[0] + r * 1.15 * a.cos(),
                        c[1] + r * 1.15 * a.sin(),
                        c[2] + 0.03,
                    ];
                    sweep(
                        b,
                        "black",
                        &[o, [o[0], o[1], c[2] + 0.03], i],
                        |_| 0.004,
                        6,
                        Cap::Flat,
                        Cap::Flat,
                    );
                }
            }
            // The knobs on the front edge.
            let n = if wide { 5 } else { 4 };
            for k in 0..n {
                let kx = cx - hw + 0.05 + (2.0 * hw - 0.1) * k as f64 / (n - 1) as f64;
                turned(
                    b,
                    "metal",
                    [kx, cy - 0.22, h + 0.008],
                    &[(0.0, 0.0), (0.016, 0.0), (0.014, 0.014), (0.0, 0.016)],
                    14,
                    0.0,
                );
            }
        }
    }
}

// -------------------------------------------------------- base unit

/// A base module of the library's modular kitchen made in a collection
/// (see `kitchen::base_unit`; same values).
pub fn base_unit(b: &mut Builder, p: &Params) -> bool {
    let Some(look) = look(p, 1) else {
        return false;
    };
    let look = &look;
    let w = p.get("width", 0.6);
    let d = p.get("depth", 0.6);
    let h = p.get("height", 0.9);
    let unit = p.get("unit", 1.0).round() as i32;
    let (x0, x1) = (-w / 2.0, w / 2.0);
    let yc = d - 0.04;
    stand(b, look, x0, x1, yc);
    let zf = foot(look);
    b.open_box("mat", [x0, 0.0, zf], [x1, yc, h - TOP]);
    let z0 = zf + 0.004;
    let mut z1 = h - TOP - 0.004;
    if look.gola {
        z1 -= 0.035;
        channel(b, x0, x1, z1 + 0.003, 0.035 - 0.003, yc);
    }
    let n_drawers = p.get("drawers", 3.0).round().clamp(0.0, 4.0) as usize;
    let n_doors = p
        .get("doors", if w > 0.6 { 2.0 } else { 1.0 })
        .round()
        .clamp(1.0, 2.0) as usize;
    let sink = unit == crate::SINK_UNIT as i32;
    let hole = crate::sink_hole(w, d);
    match unit {
        1 if n_drawers > 0 => drawers(b, look, x0, x1, z0, z1, yc, n_drawers),
        1 | 2 => doors(b, look, x0, x1, z0, z1, yc, n_doors, true),
        3 => {
            // A fireclay basin shows its front above the doors.
            if look.sink == Sink::Ceramic && hole.is_some() {
                let za = h - TOP - 0.22;
                let (sx0, sx1) = (x0 + 0.05, x1 - 0.05);
                b.rbox(
                    "sink",
                    [sx0, yc - 0.01, za],
                    [sx1, yc + T + 0.012, h - TOP],
                    0.02,
                );
                if sx0 > x0 + 0.01 {
                    b.rbox(
                        "mat",
                        [x0 + 0.0015, yc, za],
                        [sx0 - 0.002, yc + T, h - TOP - 0.004],
                        0.002,
                    );
                    b.rbox(
                        "mat",
                        [sx1 + 0.002, yc, za],
                        [x1 - 0.0015, yc + T, h - TOP - 0.004],
                        0.002,
                    );
                }
                doors(b, look, x0, x1, z0, za - 0.004, yc, n_doors, true);
            } else {
                doors(b, look, x0, x1, z0, z1, yc, n_doors, true);
            }
        }
        4 => {
            // A drawer over the oven.
            let zd = z1 - 0.14;
            drawers(b, look, x0, x1, zd, z1, yc, 1);
            oven(b, look, x0, x1, z0, zd - 0.006, yc);
        }
        // An integrated dishwasher: one front, its pull at the top.
        5 => {
            front(b, look, x0 + 0.0015, x1 - 0.0015, z0, z1, yc);
            if !look.gola {
                let pz = if look.pull == Pull::Slot {
                    z1 - 0.001
                } else {
                    z1 - 0.06
                };
                pull(b, look, 0.0, pz, yc, false);
            }
        }
        6 => drawers(b, look, x0, x1, z0, z1, yc, n_drawers.max(2)),
        7 => {
            // An open bottle rack.
            b.rbox(
                "dark",
                [x0 + 0.018, 0.01, z0],
                [x1 - 0.018, yc - 0.01, z1],
                0.0,
            );
            let n = ((z1 - z0) / 0.13).floor().max(1.0) as usize;
            for i in 0..=n {
                let z = z0 + i as f64 * (z1 - z0) / n as f64;
                b.rbox(
                    "mat",
                    [x0 + 0.01, 0.02, z],
                    [x1 - 0.01, yc + 0.012, z + 0.016],
                    0.002,
                );
            }
        }
        // A blind corner: a fixed panel and a door.
        _ => {
            let blind = x0 + w * 0.45;
            front(b, look, x0 + 0.0015, blind - 0.0015, z0, z1, yc);
            doors(b, look, blind, x1, z0, z1, yc, 1, true);
        }
    }
    // The worktop (left out of a run, which draws it whole), with the
    // sink's opening; the basin, its tap, and the hob.
    let front_over = p.get("overhang", 0.0).max(0.0);
    let back = p.get("overhang_back", 0.0).max(0.0);
    let up = p.get("upstand", 0.0).max(0.0);
    let (tlo, thi) = ([x0, -back, h - TOP], [x1, d + front_over, h]);
    if up > 1e-3 {
        b.rbox(
            "top",
            [x0, -back, h],
            [x1, -back + crate::UPSTAND, h + up],
            0.0,
        );
    }
    match (sink, hole) {
        (true, Some((lo, hi))) => {
            b.framed_top("top", tlo, thi, lo, hi);
            let c = [(lo[0] + hi[0]) / 2.0, (lo[1] + hi[1]) / 2.0];
            let (a, bb) = ((hi[0] - lo[0]) / 2.0, (hi[1] - lo[1]) / 2.0);
            basin(b, look, c, a, bb, h);
            let reach = (c[1] - lo[1] * 0.5).clamp(0.12, 0.26);
            tap(b, look, [c[0], lo[1] * 0.5, h], reach);
        }
        _ => b.rbox("top", tlo, thi, 0.0),
    }
    if unit == 6 {
        hob(b, look, w, d, h);
    }
    true
}

/// An oven's door from `x0` to `x1` and `z0` to `z1` on a face at `y`:
/// its glass, its handle, its control strip.
#[allow(clippy::too_many_arguments)]
fn oven(b: &mut Builder, look: &Look, x0: f64, x1: f64, z0: f64, z1: f64, y: f64) {
    let classic = matches!(look.hob, Hob::Gas) && !look.rounded && look.base != Base::Steel;
    b.rbox(
        "black",
        [x0 + 0.006, y, z0],
        [x1 - 0.006, y + 0.022, z1],
        0.004,
    );
    b.rbox(
        "glass",
        [x0 + 0.06, y + 0.02, z0 + 0.1],
        [x1 - 0.06, y + 0.024, z1 - 0.13],
        0.006,
    );
    // The control strip and the handle.
    b.rbox(
        "black",
        [x0 + 0.04, y + 0.022, z1 - 0.06],
        [x1 - 0.04, y + 0.024, z1 - 0.02],
        0.0,
    );
    if classic {
        for k in 0..2 {
            let kx = x0 + 0.1 + k as f64 * (x1 - x0 - 0.2);
            sweep(
                b,
                "metal",
                &[[kx, y + 0.024, z1 - 0.04], [kx, y + 0.045, z1 - 0.04]],
                |_| 0.014,
                14,
                Cap::Flat,
                Cap::Round(0.5),
            );
        }
    }
    b.cyl(
        "metal",
        [x0 + 0.08, y + 0.05, z1 - 0.1],
        [x1 - 0.08, y + 0.05, z1 - 0.1],
        0.008,
    );
    for x in [x0 + 0.1, x1 - 0.1] {
        b.cyl(
            "metal",
            [x, y + 0.022, z1 - 0.1],
            [x, y + 0.05, z1 - 0.1],
            0.006,
        );
    }
}

// -------------------------------------------------------- tall unit

/// A tall unit made in a collection (see `kitchen::tall_unit`).
pub fn tall_unit(b: &mut Builder, p: &Params) -> bool {
    let Some(look) = look(p, 1) else {
        return false;
    };
    let look = &look;
    let w = p.get("width", 0.6);
    let d = p.get("depth", 0.6);
    let h = p.get("height", 2.1);
    let tall = p.get("tall", 1.0).round() as i32;
    let (x0, x1) = (-w / 2.0, w / 2.0);
    let yc = d - 0.04;
    stand(b, look, x0, x1, yc);
    let zf = foot(look);
    let top = if look.cornice { h - 0.06 } else { h };
    b.rbox("mat", [x0, 0.0, zf], [x1, yc, top], 0.002);
    if look.cornice {
        storage::cornice(b, "mat", x0, x1, yc + T, h, 0.025);
    }
    let z0 = zf + 0.004;
    let z1 = top - 0.004;
    if look.gola {
        channel(b, x0, x1, z1 - 0.035, 0.035, yc);
    }
    let z1f = if look.gola { z1 - 0.035 } else { z1 };
    match tall {
        2 => {
            // A door under the oven, the oven, the microwave, a door over.
            let (oa, ob) = (0.8, 1.38);
            doors(b, look, x0, x1, z0, oa - 0.005, yc, 1, true);
            oven(b, look, x0, x1, oa, ob, yc);
            let (ma, mb) = (ob + 0.01, ob + 0.39);
            b.rbox(
                "black",
                [x0 + 0.006, yc, ma],
                [x1 - 0.006, yc + 0.022, mb],
                0.004,
            );
            b.rbox(
                "glass",
                [x0 + 0.05, yc + 0.02, ma + 0.05],
                [x1 - 0.16, yc + 0.024, mb - 0.05],
                0.004,
            );
            b.rbox(
                "black",
                [x1 - 0.12, yc + 0.022, ma + 0.08],
                [x1 - 0.04, yc + 0.024, mb - 0.08],
                0.0,
            );
            doors(b, look, x0, x1, mb + 0.005, z1f, yc, 1, false);
        }
        3 => {
            // An integrated fridge: two doors, the fridge's higher.
            let split = zf + (h - zf) * 0.4;
            doors(b, look, x0, x1, z0, split - 0.002, yc, 1, true);
            doors(b, look, x0, x1, split + 0.002, z1f, yc, 1, false);
        }
        _ => {
            let split = zf + (h - zf) * 0.6;
            let n = if w > 0.6 { 2 } else { 1 };
            doors(b, look, x0, x1, z0, split - 0.002, yc, n, true);
            doors(b, look, x0, x1, split + 0.002, z1f, yc, n, false);
        }
    }
    true
}

// -------------------------------------------------------- wall units

/// Wall units made in a collection (see `kitchen::wall_cabinet`):
/// cupboards, or open shelves where the collection has them.
pub fn wall_cabinet(b: &mut Builder, p: &Params) -> bool {
    let Some(look) = look(p, 1) else {
        return false;
    };
    let look = &look;
    let w = p.get("width", 1.2);
    let d = p.get("depth", 0.35);
    let h = p.get("height", 0.7);
    let z0 = p.get("elevation", 1.45);
    let (x0, x1) = (-w / 2.0, w / 2.0);
    let doors_n = p.get("doors", 0.0).round() as usize;
    let n = if doors_n > 0 {
        doors_n
    } else {
        (w / 0.6).round().max(1.0) as usize
    };
    match look.upper {
        Upper::Cupboards | Upper::Vitrine => {
            let yc = d - T;
            let top = if look.cornice { z0 + h - 0.05 } else { z0 + h };
            let r = if look.rounded { 0.02 } else { 0.002 };
            b.rbox("mat", [x0, 0.0, z0], [x1, yc, top], r);
            if look.cornice {
                storage::cornice(b, "mat", x0, x1, yc + T, z0 + h, 0.022);
            }
            let zb = if look.gola { z0 + 0.03 } else { z0 + 0.003 };
            doors(b, look, x0, x1, zb, top - 0.003, yc, n, false);
            if look.gola {
                // Push to open: their lower edges stand clear of the box.
                b.rbox("metal", [x0, yc - 0.02, z0], [x1, yc - 0.016, zb], 0.0);
            }
        }
        Upper::Corbels => {
            // Two shelves of plaster on corbels, as built.
            for (k, z) in [z0, z0 + h * 0.55].into_iter().enumerate() {
                b.rbox("hood", [x0, 0.0, z], [x1, d * 0.8, z + 0.04], 0.012);
                if k == 0 {
                    for x in [x0 + 0.12, x1 - 0.12] {
                        let prof = [
                            (0.0, 0.0),
                            (d * 0.75, 0.0),
                            (d * 0.75, -0.03),
                            (0.04, -0.16),
                            (0.0, -0.16),
                        ];
                        let pts: Vec<[f64; 2]> = prof.iter().map(|q| [q.0, q.1]).collect();
                        side_profile(b, "hood", x, 0.035, &pts, z);
                    }
                }
            }
            shelf_things(b, x0, x1, z0 + 0.04, d * 0.7);
            shelf_things(b, x0, x1, z0 + h * 0.55 + 0.04, d * 0.7);
        }
        Upper::Floating => {
            for z in [z0, z0 + h * 0.5] {
                b.rbox("wood", [x0, 0.0, z], [x1, d * 0.75, z + 0.035], 0.003);
                shelf_things(b, x0, x1, z + 0.035, d * 0.65);
            }
        }
        Upper::Brackets => {
            for (k, z) in [z0, z0 + h * 0.55].into_iter().enumerate() {
                b.rbox("wood", [x0, 0.0, z], [x1, d * 0.8, z + 0.03], 0.004);
                for x in [x0 + 0.1, x1 - 0.1] {
                    // An iron bracket: an L with a scroll.
                    b.rbox(
                        "metal",
                        [x - 0.006, 0.0, z - 0.2],
                        [x + 0.006, 0.012, z],
                        0.0,
                    );
                    b.rbox(
                        "metal",
                        [x - 0.006, 0.0, z - 0.012],
                        [x + 0.006, d * 0.7, z],
                        0.0,
                    );
                    let arc: Vec<P3> = (0..=10)
                        .map(|i| {
                            let a = PI / 2.0 * i as f64 / 10.0;
                            [x, 0.012 + 0.16 * (1.0 - a.cos()), z - 0.2 + 0.16 * a.sin()]
                        })
                        .collect();
                    sweep(b, "metal", &arc, |_| 0.004, 6, Cap::Flat, Cap::Flat);
                }
                if k == 0 {
                    // A rail with hooks under the lower shelf.
                    sweep(
                        b,
                        "metal",
                        &[[x0 + 0.15, 0.04, z - 0.08], [x1 - 0.15, 0.04, z - 0.08]],
                        |_| 0.006,
                        10,
                        Cap::Round(1.0),
                        Cap::Round(1.0),
                    );
                    let n = ((w - 0.3) / 0.12).floor().max(1.0) as usize;
                    for i in 0..n {
                        let x = x0 + 0.2 + i as f64 * 0.12;
                        let hook = curve(
                            &[
                                [x, 0.04, z - 0.08],
                                [x, 0.06, z - 0.12],
                                [x, 0.045, z - 0.15],
                            ],
                            3,
                        );
                        sweep(b, "metal", &hook, |_| 0.0025, 6, Cap::Flat, Cap::Flat);
                    }
                }
                shelf_things(b, x0, x1, z + 0.03, d * 0.7);
            }
        }
        Upper::Pipes => {
            // Boards on black pipes from the wall: flanges, elbows, posts.
            for z in [z0, z0 + h * 0.55] {
                b.rbox("wood", [x0, 0.0, z], [x1, d * 0.85, z + 0.04], 0.003);
                for x in [x0 + 0.12, x1 - 0.12] {
                    turned_y(b, "metal", [x, 0.0, z - 0.03], 0.03, 0.008);
                    sweep(
                        b,
                        "metal",
                        &[[x, 0.008, z - 0.03], [x, d * 0.6, z - 0.03]],
                        |_| 0.011,
                        12,
                        Cap::Flat,
                        Cap::Round(1.0),
                    );
                }
                shelf_things(b, x0, x1, z + 0.04, d * 0.7);
            }
        }
    }
    true
}

/// A flange on the wall: a disc looking out of it (`+y`).
fn turned_y(b: &mut Builder, role: &'static str, c: P3, r: f64, t: f64) {
    sweep(
        b,
        role,
        &[c, [c[0], c[1] + t, c[2]]],
        |_| r,
        16,
        Cap::Flat,
        Cap::Flat,
    );
}

/// A plate standing in the plane `x`, its outline `(y, z)` (from `z`
/// at its top), `t` thick.
fn side_profile(b: &mut Builder, role: &'static str, x: f64, t: f64, pts: &[[f64; 2]], z: f64) {
    // Counter-clockwise seen from `+x`, whichever way it was given.
    let area: f64 = (0..pts.len())
        .map(|i| {
            let (p, q) = (pts[i], pts[(i + 1) % pts.len()]);
            p[0] * q[1] - q[0] * p[1]
        })
        .sum();
    let mut pts = pts.to_vec();
    if area < 0.0 {
        pts.reverse();
    }
    let a: Vec<u32> = pts
        .iter()
        .map(|q| b.point([x - t / 2.0, q[0], z + q[1]]))
        .collect();
    let c: Vec<u32> = pts
        .iter()
        .map(|q| b.point([x + t / 2.0, q[0], z + q[1]]))
        .collect();
    let n = pts.len();
    // A convex outline, as the corbels': fanned from its first corner.
    for i in 1..n - 1 {
        b.tri(role, a[0], a[i + 1], a[i]);
        b.tri(role, c[0], c[i], c[i + 1]);
    }
    for i in 0..n {
        let j = (i + 1) % n;
        b.quad(role, a[i], a[j], c[j], c[i]);
    }
}

/// A few things on a shelf from `x0` to `x1`, its top at `z`: jars,
/// bowls standing up, a jug.
fn shelf_things(b: &mut Builder, x0: f64, x1: f64, z: f64, d: f64) {
    let w = x1 - x0;
    let y = d * 0.5;
    let mut x = x0 + w * 0.15;
    // Jars of three heights.
    for (k, hgt) in [0.14, 0.11, 0.16].into_iter().enumerate() {
        if x > x1 - 0.1 {
            break;
        }
        let role = if k == 1 { "ceramic" } else { "glass" };
        turned(
            b,
            role,
            [x, y, z],
            &[
                (0.0, 0.0),
                (0.04, 0.0),
                (0.042, 0.01),
                (0.042, hgt - 0.02),
                (0.03, hgt),
                (0.0, hgt),
            ],
            18,
            0.0,
        );
        turned(
            b,
            "wood",
            [x, y, z + hgt],
            &[(0.0, 0.0), (0.033, 0.0), (0.033, 0.018), (0.0, 0.02)],
            18,
            0.0,
        );
        x += 0.11;
    }
    // A jug at the other end.
    let xj = x1 - w * 0.18;
    if xj > x + 0.05 {
        turned(
            b,
            "ceramic",
            [xj, y, z],
            &[
                (0.0, 0.0),
                (0.045, 0.0),
                (0.06, 0.06),
                (0.045, 0.15),
                (0.05, 0.19),
                (0.0, 0.19),
            ],
            20,
            0.0,
        );
        let handle = curve(
            &[
                [xj + 0.045, y, z + 0.15],
                [xj + 0.09, y, z + 0.13],
                [xj + 0.065, y, z + 0.05],
            ],
            4,
        );
        sweep(b, "ceramic", &handle, |_| 0.007, 8, Cap::Flat, Cap::Flat);
    }
}

// ------------------------------------------------------------ hoods

/// A hood made in a collection (see `accessories::hood`), its canopy's
/// lip at `elevation`, its chimney `height` over it.
pub fn hood(b: &mut Builder, p: &Params) -> bool {
    let Some(look) = look(p, 2) else {
        return false;
    };
    let w = p.get("width", 0.6);
    let d = p.get("depth", 0.5);
    let z0 = p.get("elevation", 1.6);
    let top = z0 + p.get("height", 0.9);
    let (x0, x1) = (-w / 2.0, w / 2.0);
    // The filter under it.
    let filter = |b: &mut Builder, inset: f64| {
        b.rbox(
            "black",
            [x0 + inset, 0.04, z0 - 0.002],
            [x1 - inset, d - 0.05, z0],
            0.0,
        );
    };
    match look.hood {
        Hood::Chimney => {
            b.rbox("hood", [x0, 0.0, z0], [x1, d, z0 + 0.09], 0.006);
            let cw = (w * 0.45).min(0.3);
            b.rbox(
                "hood",
                [-cw / 2.0, 0.0, z0 + 0.09],
                [cw / 2.0, 0.26, top],
                0.003,
            );
            filter(b, 0.03);
        }
        Hood::Slim => {
            b.rbox("hood", [x0, 0.0, z0], [x1, d, z0 + 0.04], 0.003);
            b.rbox(
                "black",
                [x0 + 0.004, d - 0.004, z0 + 0.006],
                [x1 - 0.004, d + 0.001, z0 + 0.034],
                0.0,
            );
            let cw = (w * 0.35).min(0.26);
            b.rbox(
                "hood",
                [-cw / 2.0, 0.0, z0 + 0.04],
                [cw / 2.0, 0.2, top],
                0.002,
            );
            filter(b, 0.04);
        }
        Hood::Built => {
            // A box as the fronts, from the lip to the wall units' top.
            b.rbox("mat", [x0, 0.0, z0], [x1, d, top], 0.002);
            b.rbox(
                "black",
                [x0 + 0.02, 0.02, z0 - 0.003],
                [x1 - 0.02, d - 0.02, z0],
                0.0,
            );
        }
        Hood::Curved => {
            // A quarter round canopy, a chrome band at its lip, a chimney.
            let r = (d * 0.75).min(0.3);
            let steps = 10;
            // The outline in (y, z): from the wall, round the front in a
            // quarter circle, back to the wall at its top.
            let outline: Vec<[f64; 2]> = vec![[0.0, 0.0]]
                .into_iter()
                .chain((0..=steps).map(|i| {
                    let a = PI / 2.0 * i as f64 / steps as f64;
                    [d - r + r * a.cos(), r * a.sin()]
                }))
                .chain(std::iter::once([0.0, r]))
                .collect();
            prism_x(b, "hood", x0, x1, &outline, z0);
            b.rbox(
                "metal",
                [x0 - 0.002, d - 0.012, z0],
                [x1 + 0.002, d + 0.004, z0 + 0.025],
                0.004,
            );
            let cw = (w * 0.4).min(0.28);
            b.rbox(
                "hood",
                [-cw / 2.0, 0.0, z0 + r],
                [cw / 2.0, 0.24, top],
                0.01,
            );
            filter(b, 0.04);
        }
        Hood::Tube => {
            // A steel cone on a round flue.
            let r0 = (w * 0.5).min(d * 0.55);
            let c = [0.0, d * 0.55, z0];
            turned(
                b,
                "hood",
                c,
                &[
                    (0.0, 0.0),
                    (r0, 0.0),
                    (r0, 0.08),
                    (0.1, 0.36),
                    (0.1, top - z0),
                    (0.0, top - z0),
                ],
                32,
                0.0,
            );
            // Rivets round the lip.
            for k in 0..16 {
                let a = 2.0 * PI * k as f64 / 16.0;
                b.ellipsoid(
                    "metal",
                    [
                        c[0] + (r0 + 0.001) * a.cos(),
                        c[1] + (r0 + 0.001) * a.sin(),
                        z0 + 0.04,
                    ],
                    [0.005, 0.005, 0.005],
                );
            }
            // Its bracket to the wall.
            bar(
                b,
                "metal",
                [0.0, 0.0, top - 0.25],
                [0.0, c[1], top - 0.25],
                0.025,
                [1.0, 0.0, 0.0],
            );
            b.rbox(
                "black",
                [-r0 * 0.7, c[1] - r0 * 0.7, z0 - 0.002],
                [r0 * 0.7, c[1] + r0 * 0.7, z0],
                0.0,
            );
        }
        Hood::Mantel | Hood::Beam | Hood::Cornice => {
            // A plastered hood: a ledge at its lip, its faces sloping in
            // to a chimney as wide as half of it.
            let lip = 0.12;
            let slope_top = z0 + lip + 0.4;
            let (cw, cd) = (w * 0.55, d * 0.55);
            b.rbox("hood", [x0, 0.0, z0], [x1, d, z0 + lip], 0.01);
            // The sloping faces: a frustum from the lip to the chimney.
            let lower = [
                [x0 + 0.01, 0.0],
                [x1 - 0.01, 0.0],
                [x1 - 0.01, d - 0.01],
                [x0 + 0.01, d - 0.01],
            ];
            let upper = [
                [-cw / 2.0, 0.0],
                [cw / 2.0, 0.0],
                [cw / 2.0, cd],
                [-cw / 2.0, cd],
            ];
            frustum(b, "hood", lower, z0 + lip, upper, slope_top);
            b.rbox(
                "hood",
                [-cw / 2.0, 0.0, slope_top],
                [cw / 2.0, cd, top],
                0.008,
            );
            match look.hood {
                Hood::Beam => {
                    // An oak beam along its front, under the ledge.
                    b.rbox(
                        "wood",
                        [x0 - 0.04, d - 0.06, z0 - 0.02],
                        [x1 + 0.04, d + 0.04, z0 + 0.1],
                        0.008,
                    );
                }
                Hood::Cornice => {
                    storage::cornice(b, "hood", x0, x1, d, z0 + lip, 0.02);
                    storage::cornice(b, "hood", -cw / 2.0, cw / 2.0, cd, top, 0.015);
                }
                _ => {
                    // The ledge rounded as a bullnose.
                    b.rbox(
                        "hood",
                        [x0 - 0.02, 0.0, z0 + lip - 0.035],
                        [x1 + 0.02, d + 0.02, z0 + lip],
                        0.016,
                    );
                }
            }
            filter(b, 0.06);
        }
    }
    true
}

/// A prism across `x` from `x0` to `x1` whose outline in `(y, z)` (from
/// `z`) is `outline`, convex, counter-clockwise seen from `+x`.
fn prism_x(b: &mut Builder, role: &'static str, x0: f64, x1: f64, outline: &[[f64; 2]], z: f64) {
    let a: Vec<u32> = outline
        .iter()
        .map(|q| b.point([x0, q[0], z + q[1]]))
        .collect();
    let c: Vec<u32> = outline
        .iter()
        .map(|q| b.point([x1, q[0], z + q[1]]))
        .collect();
    let n = outline.len();
    for i in 1..n - 1 {
        b.tri(role, c[0], c[i], c[i + 1]);
        b.tri(role, a[0], a[i + 1], a[i]);
    }
    for i in 0..n {
        let j = (i + 1) % n;
        b.quad(role, a[i], a[j], c[j], c[i]);
    }
}

/// A closed frustum between a quadrilateral at `z0` and another at `z1`
/// (both counter-clockwise seen from above).
fn frustum(
    b: &mut Builder,
    role: &'static str,
    lo: [[f64; 2]; 4],
    z0: f64,
    hi: [[f64; 2]; 4],
    z1: f64,
) {
    let a: Vec<u32> = lo.iter().map(|q| b.point([q[0], q[1], z0])).collect();
    let c: Vec<u32> = hi.iter().map(|q| b.point([q[0], q[1], z1])).collect();
    b.quad(role, a[3], a[2], a[1], a[0]);
    b.quad(role, c[0], c[1], c[2], c[3]);
    for i in 0..4 {
        let j = (i + 1) % 4;
        b.quad(role, a[i], a[j], c[j], c[i]);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fine::shape::signed_volume;

    fn params(model: i32, pairs: &[(&str, f64)]) -> Params {
        let mut p = Params::default();
        p.set("style", model as f64);
        for (k, v) in pairs {
            p.set(k, *v);
        }
        p
    }

    /// Every module of every collection is made, closed (no part turned
    /// inside out) and light enough for a whole kitchen.
    #[test]
    fn the_kitchens_of_the_eight_collections() {
        for c in 0..8 {
            for unit in 1..=8 {
                let mut b = Builder::default();
                let p = params(c + 2, &[("unit", unit as f64), ("width", 0.8)]);
                assert!(base_unit(&mut b, &p));
                let tris = b.triangle_count();
                assert!(
                    (100..25_000).contains(&tris),
                    "{} unit {unit}: {tris}",
                    COLLECTIONS[c as usize]
                );
                for (role, t) in &b.parts {
                    let v = signed_volume(&b.positions, t);
                    assert!(
                        v > -1e-9,
                        "{} unit {unit} {role}: {v}",
                        COLLECTIONS[c as usize]
                    );
                }
            }
            for tall in 1..=3 {
                let mut b = Builder::default();
                assert!(tall_unit(&mut b, &params(c + 2, &[("tall", tall as f64)])));
                assert!(b.triangle_count() < 25_000);
            }
            let mut b = Builder::default();
            assert!(wall_cabinet(&mut b, &params(c + 2, &[])));
            assert!(b.triangle_count() < 25_000);
            let mut b = Builder::default();
            assert!(hood(&mut b, &params(c + 3, &[])));
            for (role, t) in &b.parts {
                let v = signed_volume(&b.positions, t);
                assert!(v > -1e-9, "{} hood {role}: {v}", COLLECTIONS[c as usize]);
            }
        }
        // The library's own models are not the collections'.
        let mut b = Builder::default();
        assert!(!base_unit(&mut b, &params(1, &[])));
        assert!(!hood(&mut b, &params(2, &[])));
    }
}
