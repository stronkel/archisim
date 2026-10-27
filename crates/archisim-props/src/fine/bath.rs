//! The workshop's bathrooms, one for each collection (paquete 2b): the
//! library's washbasins, toilets, bidets, showers, baths, mirrors and
//! towel rails, with the same sizes, made the way each collection makes a
//! bathroom.
//!
//! Axes as every piece's: the back on the wall at `y = 0`, the front
//! looking to `+y`, centred on `x = 0`.
//!
//! Roles: `mat` the vitreous china (basins, bowls, tubs), `unit` a
//! vanity's body and fronts or a built base, `top` a counter or a basin of
//! stone, `metal` taps, legs, frames and rails, `seat` a toilet's seat,
//! `glass` a screen, `frame` a mirror's frame, `wood` slats, ladders and
//! tubs of wood, `towel` towels and a shower's curtain, `dark` drains and
//! shadows, `light` a mirror's light.

use super::kitchen::{tap_scaled, Tap};
use super::shape::{bar, curve, loft_loop, sweep, turned, Cap};
use super::storage::{self, Panel, Pull, T};
use crate::build::{Builder, P3};
use crate::Params;
use std::f64::consts::PI;

pub use super::kitchen::COLLECTIONS;

/// What the washbasin stands on.
#[derive(Clone, Copy, PartialEq)]
enum Vanity {
    /// A cabinet hung on the wall, the basin made in its top.
    Hung,
    /// A built base of microcement with an arched niche, a bowl on it.
    Built,
    /// An oak shelf hung on the wall, another below, a bowl on top.
    Shelf,
    /// A thick old plank on iron brackets, a stone bowl on it.
    Plank,
    /// A pedestal of china.
    Pedestal,
    /// A steel frame with a wooden shelf, a concrete trough on it.
    Frame,
    /// A marble top on turned legs, the basin under it.
    Washstand,
}

/// The toilet.
#[derive(Clone, Copy, PartialEq)]
enum Wc {
    /// Hung on the wall, a flush plate.
    Hung,
    /// On the floor with its cistern.
    Floor,
    /// On the floor, against the wall, the cistern in it.
    BackToWall,
    /// On the floor, the cistern high on the wall with its pipe and chain.
    HighTank,
}

/// What closes a shower.
#[derive(Clone, Copy, PartialEq)]
enum Screen {
    /// A fixed glass panel, its edge in a thin profile.
    Glass,
    /// A fixed panel in a black frame.
    Black,
    /// A black frame with glazing bars, as a steel window.
    Grid,
    /// Two sliding panels of reeded glass in a chrome frame.
    Sliding,
    /// A curtain on a rail.
    Curtain,
    /// A panel in a brass frame.
    Brass,
}

/// The shower's head and its taps.
#[derive(Clone, Copy, PartialEq)]
enum Head {
    /// A round rain head on an arm from the wall.
    Round,
    /// A square rain head from the ceiling.
    Square,
    /// An exposed riser from a bar mixer, a big rose.
    Exposed,
    /// A bell on an arm, the fifties'.
    Bell,
}

/// The bath.
#[derive(Clone, Copy, PartialEq)]
enum Tub {
    /// A freestanding oval tub, a tall filler beside it.
    Oval,
    /// A roll-top tub on four claw feet.
    Claw,
    /// A tub set in a built base.
    Built,
    /// A deep square tub of wood.
    Ofuro,
}

/// The mirror.
#[derive(Clone, Copy, PartialEq)]
enum Mirror {
    RoundWood,
    Arched,
    RoundChrome,
    Capsule,
    Lit,
    RectWood,
    RectSteel,
    Ornate,
}

/// The towel rail.
#[derive(Clone, Copy, PartialEq)]
enum Rail {
    /// A wooden ladder leaning on the wall.
    Ladder,
    /// A bar on two posts.
    Bar,
    /// A ring of iron.
    Ring,
    /// A ladder of black pipe.
    Pipes,
    /// A heated rail of columns, brass.
    Columns,
}

/// How a collection makes a bathroom.
#[derive(Clone, Copy)]
struct Look {
    vanity: Vanity,
    panel: Panel,
    pull: Pull,
    /// The washbasin's tap: on the deck (`Some`) or from the wall.
    tap: Option<Tap>,
    wc: Wc,
    screen: Screen,
    head: Head,
    tub: Tub,
    mirror: Mirror,
    rail: Rail,
}

const LOOKS: [Look; 8] = [
    // Nórdico cálido.
    Look {
        vanity: Vanity::Hung,
        panel: Panel::Flat,
        pull: Pull::Tab,
        tap: Some(Tap::Gooseneck),
        wc: Wc::Hung,
        screen: Screen::Glass,
        head: Head::Round,
        tub: Tub::Oval,
        mirror: Mirror::RoundWood,
        rail: Rail::Ladder,
    },
    // Mediterráneo.
    Look {
        vanity: Vanity::Built,
        panel: Panel::Flat,
        pull: Pull::None,
        tap: Some(Tap::Swan),
        wc: Wc::Floor,
        screen: Screen::Curtain,
        head: Head::Exposed,
        tub: Tub::Built,
        mirror: Mirror::Arched,
        rail: Rail::Ring,
    },
    // Años 50.
    Look {
        vanity: Vanity::Pedestal,
        panel: Panel::Flat,
        pull: Pull::None,
        tap: Some(Tap::Twin),
        wc: Wc::Floor,
        screen: Screen::Sliding,
        head: Head::Bell,
        tub: Tub::Built,
        mirror: Mirror::RoundChrome,
        rail: Rail::Bar,
    },
    // Japandi.
    Look {
        vanity: Vanity::Shelf,
        panel: Panel::Flat,
        pull: Pull::None,
        tap: None,
        wc: Wc::Hung,
        screen: Screen::Black,
        head: Head::Round,
        tub: Tub::Ofuro,
        mirror: Mirror::Capsule,
        rail: Rail::Ladder,
    },
    // Contemporáneo.
    Look {
        vanity: Vanity::Hung,
        panel: Panel::Flat,
        pull: Pull::None,
        tap: None,
        wc: Wc::Hung,
        screen: Screen::Glass,
        head: Head::Square,
        tub: Tub::Oval,
        mirror: Mirror::Lit,
        rail: Rail::Bar,
    },
    // Rústica.
    Look {
        vanity: Vanity::Plank,
        panel: Panel::Planks,
        pull: Pull::Cup,
        tap: Some(Tap::Bridge),
        wc: Wc::HighTank,
        screen: Screen::Curtain,
        head: Head::Exposed,
        tub: Tub::Claw,
        mirror: Mirror::RectWood,
        rail: Rail::Ring,
    },
    // Industrial.
    Look {
        vanity: Vanity::Frame,
        panel: Panel::Flat,
        pull: Pull::None,
        tap: None,
        wc: Wc::BackToWall,
        screen: Screen::Grid,
        head: Head::Exposed,
        tub: Tub::Claw,
        mirror: Mirror::RectSteel,
        rail: Rail::Pipes,
    },
    // Clásica.
    Look {
        vanity: Vanity::Washstand,
        panel: Panel::Raised,
        pull: Pull::Brass,
        tap: Some(Tap::Bridge),
        wc: Wc::HighTank,
        screen: Screen::Brass,
        head: Head::Exposed,
        tub: Tub::Claw,
        mirror: Mirror::Ornate,
        rail: Rail::Columns,
    },
];

/// The collection's look when the model chosen is one of the collections,
/// listed after the family's `first` own models.
fn look(p: &Params, first: i32) -> Option<(usize, Look)> {
    let s = p.style() - first;
    (1..=8)
        .contains(&s)
        .then(|| ((s - 1) as usize, LOOKS[(s - 1) as usize]))
}

// ---------------------------------------------------------- pieces

/// A bowl under an opening of half sides `a`, `bb` (corners of radius
/// `r`) about `c`, from `zt` down `deep`, its wall `wall` thick.
#[allow(clippy::too_many_arguments)]
fn under_bowl(
    b: &mut Builder,
    role: &'static str,
    c: [f64; 2],
    a: f64,
    bb: f64,
    r: f64,
    zt: f64,
    deep: f64,
    wall: f64,
) {
    let r = r.min(a).min(bb);
    let k = (r * 0.7).min(0.04);
    let rings = [
        (a + wall, bb + wall, r + wall, zt - 0.003),
        (a + wall, bb + wall, r + wall, zt),
        (a, bb, r, zt),
        (a, bb, r, zt - 0.01),
        (a - k * 0.3, bb - k * 0.3, r, zt - deep + k),
        (a - k, bb - k, r * 0.8, zt - deep),
        (0.03, 0.03, 0.03, zt - deep - 0.003),
        (0.03, 0.03, 0.03, zt - deep - 0.006 - wall * 0.5),
        (
            a - k + wall,
            bb - k + wall,
            r * 0.8 + wall,
            zt - deep - wall,
        ),
        (
            a - k * 0.3 + wall,
            bb - k * 0.3 + wall,
            r + wall,
            zt - deep + k,
        ),
        (a + wall, bb + wall, r + wall, zt - 0.01),
    ];
    loft_loop(b, role, c, &rings);
    turned(
        b,
        "metal",
        [c[0], c[1], zt - deep - 0.003],
        &[(0.0, -0.002), (0.028, -0.002), (0.03, 0.002), (0.0, 0.003)],
        18,
        0.0,
    );
}

/// A bowl standing on a surface at `z`: round (`sy` = 1) or oval, `r`
/// across and `h` high, its wall `t` thick.
#[allow(clippy::too_many_arguments)]
fn vessel(
    b: &mut Builder,
    role: &'static str,
    c: [f64; 2],
    r: f64,
    h: f64,
    t: f64,
    sx: f64,
    sy: f64,
    flat: f64,
) {
    // `flat` near 1: a bowl with straight sides; near 0: a rounded one.
    let mid = 0.75 + 0.2 * flat;
    let prof = [
        (0.0, 0.0),
        (r * (0.45 + 0.4 * flat), 0.0),
        (r * mid, h * 0.25),
        (r, h),
        (r - t, h),
        (r * mid - t, h * 0.28),
        (r * (0.42 + 0.4 * flat) - t, t),
        (0.0, t),
    ];
    b.lathe(role, [c[0], c[1], 0.0], &prof, sx, sy);
}

/// A bowl standing at height `z` (see [`vessel`]).
#[allow(clippy::too_many_arguments)]
fn vessel_at(
    b: &mut Builder,
    role: &'static str,
    c: [f64; 2],
    z: f64,
    r: f64,
    h: f64,
    t: f64,
    sx: f64,
    sy: f64,
    flat: f64,
) {
    let start = b.mark();
    vessel(b, role, [0.0, 0.0], r, h, t, sx, sy, flat);
    b.transform_from(start, |q| [c[0] + q[0], c[1] + q[1], z + q[2]]);
}

/// A tap coming out of the wall at height `z` over `x`, reaching `reach`.
fn wall_tap(b: &mut Builder, look_ix: usize, x: f64, z: f64, reach: f64) {
    match look_ix {
        // Contemporáneo: a square plate, a square spout, a flat lever.
        4 => {
            b.rbox(
                "metal",
                [x - 0.09, 0.0, z - 0.03],
                [x + 0.09, 0.008, z + 0.11],
                0.002,
            );
            bar(
                b,
                "metal",
                [x - 0.04, 0.0, z],
                [x - 0.04, reach, z],
                0.022,
                [0.0, 0.0, 1.0],
            );
            b.rbox(
                "metal",
                [x + 0.04, 0.008, z + 0.05],
                [x + 0.06, 0.06, z + 0.058],
                0.001,
            );
        }
        // Industrial: a pipe from the wall with an elbow, a cross handle
        // on a tee.
        6 => {
            turned_y(b, "metal", [x, 0.0, z + 0.08], 0.025, 0.012);
            sweep(
                b,
                "metal",
                &[[x, 0.012, z + 0.08], [x, 0.07, z + 0.08]],
                |_| 0.012,
                12,
                Cap::Flat,
                Cap::Flat,
            );
            turned(
                b,
                "metal",
                [x, 0.07, z + 0.05],
                &[(0.0, 0.0), (0.017, 0.0), (0.017, 0.06), (0.0, 0.06)],
                12,
                0.0,
            );
            for k in 0..4 {
                let a = PI / 4.0 + PI / 2.0 * k as f64;
                let e = [x + 0.03 * a.cos(), 0.07 + 0.03 * a.sin(), z + 0.125];
                sweep(
                    b,
                    "metal",
                    &[[x, 0.07, z + 0.115], e],
                    |_| 0.004,
                    8,
                    Cap::Flat,
                    Cap::Round(1.2),
                );
            }
            let spout = curve(
                &[
                    [x, 0.07, z + 0.05],
                    [x, 0.07 + reach * 0.4, z + 0.05],
                    [x, reach, z + 0.02],
                ],
                4,
            );
            sweep(b, "metal", &spout, |_| 0.011, 12, Cap::Flat, Cap::Flat);
        }
        // Japandi and others: a slim round spout and a round plate with a
        // small lever.
        _ => {
            turned_y(b, "metal", [x, 0.0, z], 0.03, 0.006);
            sweep(
                b,
                "metal",
                &[[x, 0.006, z], [x, reach, z]],
                |_| 0.009,
                14,
                Cap::Flat,
                Cap::Flat,
            );
            turned_y(b, "metal", [x, 0.0, z + 0.11], 0.03, 0.006);
            sweep(
                b,
                "metal",
                &[[x, 0.006, z + 0.11], [x, 0.05, z + 0.11]],
                |_| 0.006,
                10,
                Cap::Flat,
                Cap::Round(1.0),
            );
        }
    }
}

/// A disc on the wall looking out of it (`+y`).
fn turned_y(b: &mut Builder, role: &'static str, c: P3, r: f64, t: f64) {
    sweep(
        b,
        role,
        &[c, [c[0], c[1] + t, c[2]]],
        |_| r,
        18,
        Cap::Flat,
        Cap::Flat,
    );
}

/// A towel hanging over a bar at `(y, z)` from `x0` to `x1`, `fall`
/// down each side.
fn towel_over(b: &mut Builder, x0: f64, x1: f64, y: f64, z: f64, fall: f64) {
    let t = 0.008;
    b.rbox(
        "towel",
        [x0, y - 0.022, z - fall],
        [x1, y - 0.022 + t, z],
        0.003,
    );
    b.rbox(
        "towel",
        [x0, y + 0.014, z - fall * 0.8],
        [x1, y + 0.014 + t, z],
        0.003,
    );
    b.rbox(
        "towel",
        [x0, y - 0.022, z],
        [x1, y + 0.022, z + 0.016],
        0.008,
    );
}

// -------------------------------------------------------- washbasin

/// A washbasin made in a collection (see `sanitary::washbasin`): `height`
/// is its rim's.
pub fn washbasin(b: &mut Builder, p: &Params) -> bool {
    let Some((ix, look)) = look(p, 3) else {
        return false;
    };
    let w = p.get("width", 0.6);
    let d = p.get("depth", 0.45);
    let h = p.get("height", 0.85);
    let (x0, x1) = (-w / 2.0, w / 2.0);
    // Where the water falls: the middle of the basin, and where the tap
    // stands behind it.
    let cy = d * 0.55;
    match look.vanity {
        Vanity::Hung => {
            // The cabinet, its top the basin.
            let top = 0.03;
            let z0 = (h - 0.5).max(0.2);
            b.rbox("unit", [x0, 0.0, z0], [x1, d - 0.04 - T, h - top], 0.003);
            let yc = d - 0.04 - T;
            let gola = look.pull == Pull::None;
            let zt = if gola {
                h - top - 0.03
            } else {
                h - top - 0.004
            };
            storage::front(
                b,
                look.panel,
                "unit",
                x0 + 0.002,
                x1 - 0.002,
                z0 + 0.003,
                zt,
                yc,
            );
            if gola {
                b.rbox("dark", [x0, yc - 0.02, zt], [x1, yc - 0.016, h - top], 0.0);
            } else {
                storage::pull(b, look.pull, 0.0, zt - 0.04, yc + T);
            }
            let (a, bb) = ((w / 2.0 - 0.07).max(0.12), (d - 0.15) / 2.0);
            let c = [0.0, 0.06 + bb + 0.02];
            b.framed_top(
                "mat",
                [x0, 0.0, h - top],
                [x1, d, h],
                [c[0] - a, c[1] - bb],
                [c[0] + a, c[1] + bb],
            );
            under_bowl(b, "mat", c, a, bb, 0.06, h - top, 0.12, 0.012);
            match look.tap {
                Some(t) => tap_scaled(b, t, [0.0, 0.035, h], 0.25, 0.55),
                None => wall_tap(b, ix, 0.0, h + 0.2, c[1] - 0.02),
            }
        }
        Vanity::Built => {
            // A block of microcement with an arched niche, its top a slab.
            let slab = 0.06;
            let zt = h - 0.14;
            b.rbox("unit", [x0, 0.0, 0.0], [x1, d, zt - slab], 0.02);
            b.rbox(
                "unit",
                [x0 - 0.01, 0.0, zt - slab],
                [x1 + 0.01, d + 0.01, zt],
                0.02,
            );
            // The niche, dark, in its front.
            let (nx0, nx1) = (x0 + 0.08, x1 - 0.08);
            let nz = zt - slab - 0.12;
            b.rbox(
                "dark",
                [nx0, d - 0.25, 0.12],
                [nx1, d + 0.001, nz - (nx1 - nx0) / 2.0],
                0.0,
            );
            let arc: Vec<[f64; 2]> = (0..=12)
                .map(|i| {
                    let a = PI * i as f64 / 12.0;
                    [-(nx1 - nx0) / 2.0 * a.cos(), (nx1 - nx0) / 2.0 * a.sin()]
                })
                .collect();
            let start = b.mark();
            let pts: Vec<[f64; 2]> = arc.iter().map(|q| [q[0], q[1]]).collect();
            front_profile(b, "dark", &pts, d - 0.25, d + 0.001);
            b.transform_from(start, |q| [q[0], q[1], q[2] + nz - (nx1 - nx0) / 2.0]);
            // Towels rolled in the niche.
            for k in 0..3 {
                let x = nx0 + 0.09 + k as f64 * ((nx1 - nx0 - 0.18) / 2.0);
                b.cyl("towel", [x, d - 0.22, 0.2], [x, d - 0.04, 0.2], 0.075);
            }
            vessel_at(b, "mat", [0.0, cy], zt, 0.2, 0.14, 0.012, 1.0, 1.0, 0.2);
            tap_scaled(b, look.tap.unwrap_or(Tap::Swan), [0.0, 0.06, zt], 0.3, 0.7);
        }
        Vanity::Shelf => {
            let zt = h - 0.12;
            b.rbox("unit", [x0, 0.0, zt - 0.04], [x1, d, zt], 0.004);
            b.rbox(
                "unit",
                [x0, 0.0, zt - 0.42],
                [x1, d * 0.9, zt - 0.39],
                0.004,
            );
            for x in [x0 + 0.02, x1 - 0.04] {
                b.rbox(
                    "unit",
                    [x, 0.0, zt - 0.42],
                    [x + 0.02, d * 0.9, zt - 0.04],
                    0.002,
                );
            }
            for k in 0..2 {
                let x = x0 + 0.12 + k as f64 * 0.12;
                b.rbox(
                    "towel",
                    [x - 0.05, 0.04, zt - 0.39],
                    [x + 0.05, d * 0.75, zt - 0.31],
                    0.012,
                );
            }
            vessel_at(b, "mat", [0.0, cy], zt, 0.19, 0.12, 0.01, 1.0, 1.0, 0.6);
            wall_tap(b, ix, 0.0, zt + 0.25, cy - 0.04);
        }
        Vanity::Plank => {
            let zt = h - 0.13;
            b.rbox("unit", [x0, 0.0, zt - 0.07], [x1, d, zt], 0.008);
            for x in [x0 + 0.1, x1 - 0.1] {
                b.rbox(
                    "metal",
                    [x - 0.008, 0.0, zt - 0.3],
                    [x + 0.008, 0.012, zt - 0.07],
                    0.0,
                );
                b.rbox(
                    "metal",
                    [x - 0.008, 0.0, zt - 0.082],
                    [x + 0.008, d * 0.8, zt - 0.07],
                    0.0,
                );
                let arc: Vec<P3> = (0..=10)
                    .map(|i| {
                        let a = PI / 2.0 * i as f64 / 10.0;
                        [x, 0.012 + 0.2 * (1.0 - a.cos()), zt - 0.3 + 0.21 * a.sin()]
                    })
                    .collect();
                sweep(b, "metal", &arc, |_| 0.005, 6, Cap::Flat, Cap::Flat);
            }
            // A basket underneath.
            b.rbox(
                "wood",
                [x0 + 0.15, 0.05, 0.0],
                [x1 - 0.15, d * 0.85, 0.28],
                0.02,
            );
            vessel_at(b, "top", [0.0, cy], zt, 0.2, 0.13, 0.025, 1.25, 1.0, 0.8);
            tap_scaled(
                b,
                look.tap.unwrap_or(Tap::Bridge),
                [0.0, 0.05, zt],
                0.3,
                0.65,
            );
        }
        Vanity::Pedestal => {
            // The basin, round in front, and its pedestal.
            let rim = 0.17;
            turned(
                b,
                "mat",
                [0.0, d * 0.4, 0.0],
                &[
                    (0.0, 0.0),
                    (0.13, 0.0),
                    (0.12, 0.02),
                    (0.09, 0.06),
                    (0.075, h * 0.5),
                    (0.1, h - rim - 0.02),
                    (0.0, h - rim - 0.02),
                ],
                28,
                0.0,
            );
            // An oval reaching from the wall to the front.
            let r = w / 2.0;
            let sy = (d / 2.0) / r;
            vessel_at(
                b,
                "mat",
                [0.0, d / 2.0],
                h - rim,
                r,
                rim,
                0.02,
                1.0,
                sy,
                0.3,
            );
            // A back to the wall.
            b.rbox(
                "mat",
                [x0 + 0.04, 0.0, h - rim],
                [x1 - 0.04, 0.06, h + 0.03],
                0.02,
            );
            tap_scaled(
                b,
                look.tap.unwrap_or(Tap::Twin),
                [0.0, 0.035, h + 0.03],
                0.24,
                0.6,
            );
        }
        Vanity::Frame => {
            let zt = h - 0.14;
            storage::steel_frame(b, x0, x1, d, zt, &[0.2, zt], 0.025, false);
            b.rbox(
                "wood",
                [x0 + 0.03, 0.02, 0.2],
                [x1 - 0.03, d - 0.02, 0.23],
                0.003,
            );
            for k in 0..2 {
                let x = x0 + 0.15 + k as f64 * 0.15;
                b.rbox(
                    "towel",
                    [x - 0.06, 0.06, 0.23],
                    [x + 0.06, d * 0.8, 0.32],
                    0.012,
                );
            }
            // The concrete trough, square-sided.
            let (tw, td) = (w - 0.06, d - 0.06);
            let start = b.mark();
            turned(
                b,
                "top",
                [0.0, 0.0, 0.0],
                &[
                    (0.0, 0.0),
                    (0.7, 0.0),
                    (0.7, 1.0),
                    (0.62, 1.0),
                    (0.62, 0.3),
                    (0.0, 0.3),
                ],
                4,
                PI / 4.0,
            );
            let (kx, ky) = (
                tw / 2.0 / (0.7 * (PI / 4.0).cos()),
                td / 2.0 / (0.7 * (PI / 4.0).cos()),
            );
            b.transform_from(start, |q| {
                [q[0] * kx, d / 2.0 + q[1] * ky, zt + q[2] * 0.14]
            });
            wall_tap(b, ix, 0.0, zt + 0.25, d * 0.55);
        }
        Vanity::Washstand => {
            let top = 0.03;
            let zt = h;
            b.rbox("top", [x0, 0.0, zt - top], [x1, d, zt], 0.006);
            b.rbox("top", [x0, 0.0, zt], [x1, 0.02, zt + 0.12], 0.004);
            for (x, y) in [
                (x0 + 0.05, d - 0.05),
                (x1 - 0.05, d - 0.05),
                (x0 + 0.05, 0.05),
                (x1 - 0.05, 0.05),
            ] {
                turned(
                    b,
                    "metal",
                    [x, y, 0.0],
                    &[
                        (0.0, 0.0),
                        (0.02, 0.0),
                        (0.014, 0.05),
                        (0.018, 0.1),
                        (0.012, 0.3),
                        (0.012, zt - top - 0.12),
                        (0.017, zt - top - 0.06),
                        (0.012, zt - top),
                        (0.0, zt - top),
                    ],
                    16,
                    0.0,
                );
            }
            // A towel bar between the front legs.
            sweep(
                b,
                "metal",
                &[[x0 + 0.05, d - 0.05, 0.62], [x1 - 0.05, d - 0.05, 0.62]],
                |_| 0.008,
                12,
                Cap::Flat,
                Cap::Flat,
            );
            towel_over(b, x0 + 0.1, x0 + 0.38, d - 0.05, 0.62, 0.3);
            let (a, bb) = ((w / 2.0 - 0.1).max(0.12), (d - 0.18) / 2.0);
            let c = [0.0, 0.08 + bb];
            b.framed_top(
                "top",
                [x0, 0.0, zt - top],
                [x1, d, zt],
                [c[0] - a, c[1] - bb],
                [c[0] + a, c[1] + bb],
            );
            under_bowl(b, "mat", c, a, bb, bb, zt - top, 0.14, 0.012);
            tap_scaled(
                b,
                look.tap.unwrap_or(Tap::Bridge),
                [0.0, 0.05, zt],
                0.3,
                0.6,
            );
        }
    }
    true
}

/// A flat shape in `(x, z)` (convex, counter-clockwise seen from the
/// front), from `y0` to `y1`.
fn front_profile(b: &mut Builder, role: &'static str, pts: &[[f64; 2]], y0: f64, y1: f64) {
    let n = pts.len();
    // Seen from +y: x to the left looking back towards the wall, so a
    // counter-clockwise outline (x right, z up from the front) is drawn
    // with the front face's corners in reverse.
    let a: Vec<u32> = pts.iter().map(|q| b.point([q[0], y0, q[1]])).collect();
    let c: Vec<u32> = pts.iter().map(|q| b.point([q[0], y1, q[1]])).collect();
    let area: f64 = (0..n)
        .map(|i| {
            let (p, q) = (pts[i], pts[(i + 1) % n]);
            p[0] * q[1] - q[0] * p[1]
        })
        .sum();
    let (a, c) = if area > 0.0 {
        (a, c)
    } else {
        (
            a.into_iter().rev().collect(),
            c.into_iter().rev().collect::<Vec<_>>(),
        )
    };
    for i in 1..n - 1 {
        b.tri(role, c[0], c[i], c[i + 1]);
        b.tri(role, a[0], a[i + 1], a[i]);
    }
    for i in 0..n {
        let j = (i + 1) % n;
        b.quad(role, a[i], c[i], c[j], a[j]);
    }
}

// -------------------------------------------------------- toilets

/// The bowl of a toilet or a bidet: an oval tapering to its foot (or to
/// the wall when hung), `w` across, its front at `front`, its rim at
/// `rim`, standing from `base`.
fn bowl(b: &mut Builder, w: f64, front: f64, base: f64, rim: f64, hung: bool, square: bool) {
    let r = w / 2.0;
    let cy = front - r * 1.35;
    let sy = 1.35;
    let prof: Vec<(f64, f64)> = if hung {
        vec![
            (0.0, 0.0),
            (0.6, 0.0),
            (0.85, 0.06),
            (1.0, rim - base - 0.035),
            (0.98, rim - base),
            (0.8, rim - base),
            (0.72, rim - base - 0.1),
            (0.0, rim - base - 0.14),
        ]
    } else {
        vec![
            (0.0, 0.0),
            (0.5, 0.0),
            (0.48, 0.15),
            (0.8, 0.28),
            (1.0, rim - 0.035),
            (0.98, rim),
            (0.8, rim),
            (0.7, rim - 0.12),
            (0.0, rim - 0.16),
        ]
    }
    .into_iter()
    .map(|(k, z)| (k * r, z))
    .collect();
    if square {
        let start = b.mark();
        turned(b, "mat", [0.0, 0.0, 0.0], &prof, 4, PI / 4.0);
        let k = std::f64::consts::SQRT_2 * 0.92;
        b.transform_from(start, |q| [q[0] * k, cy + q[1] * k * sy, base + q[2]]);
    } else {
        b.lathe("mat", [0.0, cy, base], &prof, 1.0, sy);
    }
    // Back to the wall.
    b.rbox(
        "mat",
        [-r * 0.62, 0.0, base],
        [r * 0.62, cy, rim - 0.03],
        0.04,
    );
}

/// A seat and its lid, closed, over a bowl of `w` whose front is at
/// `front`, its rim at `rim`.
fn seat(b: &mut Builder, w: f64, front: f64, rim: f64, square: bool) {
    let r = w / 2.0;
    let cy = front - r * 1.35;
    if square {
        b.rbox(
            "seat",
            [-r * 0.95, cy - r * 1.25, rim],
            [r * 0.95, front, rim + 0.035],
            0.012,
        );
    } else {
        b.lathe(
            "seat",
            [0.0, cy, rim],
            &[(0.0, 0.0), (r * 1.0, 0.0), (r * 0.98, 0.03), (0.0, 0.04)],
            1.0,
            1.35,
        );
    }
    b.rbox(
        "seat",
        [-0.07, cy - r * 1.35 - 0.02, rim],
        [0.07, cy - r * 1.2, rim + 0.04],
        0.01,
    );
}

/// A toilet made in a collection (see `sanitary::wc`).
pub fn wc(b: &mut Builder, p: &Params) -> bool {
    let Some((ix, look)) = look(p, 2) else {
        return false;
    };
    let w = p.get("width", 0.38);
    let d = p.get("depth", 0.68);
    let rim = 0.4;
    let square = ix == 4;
    match look.wc {
        Wc::Hung => {
            bowl(b, w, d.min(0.56), 0.22, rim, true, square);
            seat(b, w, d.min(0.56), rim, square);
            // The flush plate.
            if square {
                b.rbox("metal", [-0.12, 0.0, 1.0], [0.12, 0.008, 1.16], 0.002);
                b.rbox("dark", [-0.002, 0.008, 1.01], [0.002, 0.009, 1.15], 0.0);
            } else {
                b.rbox("metal", [-0.11, 0.0, 1.0], [0.11, 0.008, 1.15], 0.012);
                for x in [-0.045, 0.045] {
                    b.cyl("mat", [x, 0.008, 1.075], [x, 0.0095, 1.075], 0.035);
                }
            }
        }
        Wc::Floor | Wc::BackToWall => {
            let back = if look.wc == Wc::BackToWall { 0.0 } else { 0.16 };
            bowl(b, w, d, 0.0, rim, false, false);
            seat(b, w, d, rim, false);
            if look.wc == Wc::Floor {
                // The cistern on the bowl, rounded, its lever.
                b.rbox(
                    "mat",
                    [-w * 0.48, 0.0, rim - 0.02],
                    [w * 0.48, back + 0.02, 0.8],
                    0.04,
                );
                b.rbox(
                    "mat",
                    [-w * 0.5, -0.004, 0.79],
                    [w * 0.5, back + 0.03, 0.83],
                    0.02,
                );
                sweep(
                    b,
                    "metal",
                    &[
                        [-w * 0.4, back + 0.03, 0.74],
                        [-w * 0.4 + 0.07, back + 0.05, 0.74],
                    ],
                    |_| 0.006,
                    10,
                    Cap::Round(1.0),
                    Cap::Round(1.0),
                );
            } else {
                b.rbox("metal", [-0.1, 0.0, 0.9], [0.1, 0.01, 1.03], 0.004);
                b.rbox(
                    "mat",
                    [-w * 0.45, 0.0, 0.0],
                    [w * 0.45, 0.2, rim - 0.03],
                    0.03,
                );
            }
        }
        Wc::HighTank => {
            bowl(b, w, d, 0.0, rim, false, false);
            seat(b, w, d, rim, false);
            // The cistern high on the wall on two brackets, its pipe down
            // to the bowl and the chain with its pull.
            let zc = 1.95;
            b.rbox("unit", [-0.24, 0.0, zc], [0.24, 0.2, zc + 0.26], 0.015);
            b.rbox(
                "unit",
                [-0.25, 0.0, zc + 0.24],
                [0.25, 0.21, zc + 0.27],
                0.01,
            );
            for x in [-0.18, 0.18] {
                b.rbox(
                    "metal",
                    [x - 0.008, 0.0, zc - 0.12],
                    [x + 0.008, 0.16, zc],
                    0.0,
                );
            }
            let pipe = curve(
                &[
                    [-0.12, 0.08, zc],
                    [-0.12, 0.08, rim + 0.25],
                    [0.0, 0.09, rim + 0.08],
                    [0.0, 0.1, rim - 0.02],
                ],
                5,
            );
            sweep(b, "metal", &pipe, |_| 0.016, 12, Cap::Flat, Cap::Flat);
            sweep(
                b,
                "metal",
                &[[0.2, 0.15, zc], [0.2, 0.15, zc - 0.55]],
                |_| 0.003,
                6,
                Cap::Flat,
                Cap::Flat,
            );
            turned(
                b,
                "unit",
                [0.2, 0.15, zc - 0.64],
                &[
                    (0.0, 0.0),
                    (0.014, 0.0),
                    (0.02, 0.05),
                    (0.012, 0.09),
                    (0.0, 0.09),
                ],
                14,
                0.0,
            );
        }
    }
    true
}

/// A bidet made in a collection (see `sanitary::bidet`): hung or on the
/// floor as the collection's toilet, its tap as its washbasin's.
pub fn bidet(b: &mut Builder, p: &Params) -> bool {
    let Some((ix, look)) = look(p, 2) else {
        return false;
    };
    let w = p.get("width", 0.36);
    let d = p.get("depth", 0.55);
    let rim = 0.4;
    let hung = look.wc == Wc::Hung;
    let front = if hung { d.min(0.54) } else { d };
    bowl(
        b,
        w,
        front,
        if hung { 0.2 } else { 0.0 },
        rim,
        hung,
        ix == 4,
    );
    // The deck behind the bowl, its tap.
    let tap = look.tap.unwrap_or(match ix {
        4 => Tap::Square,
        6 => Tap::Spring,
        _ => Tap::Slim,
    });
    let k = if tap == Tap::Spring { 0.35 } else { 0.45 };
    tap_scaled(b, tap, [0.0, 0.06, rim], 0.18, k);
    true
}

// -------------------------------------------------------- showers

/// A shower made in a collection (see `sanitary::shower`): its tray, what
/// closes it on the open sides (the first model of the family: none; the
/// second: one side; the third: two) and its head and taps on the back
/// wall.
pub fn shower(b: &mut Builder, p: &Params) -> bool {
    let Some((ix, look)) = look(p, 3) else {
        return false;
    };
    let w = p.get("width", 0.9);
    let d = p.get("depth", 0.9);
    let h = p.get("height", 0.04).max(0.01);
    let (x0, x1) = (-w / 2.0, w / 2.0);
    // The tray, its waste: a linear channel along the back for the
    // contemporary and japandi, a round one in the others.
    b.rbox("mat", [x0, 0.0, 0.0], [x1, d, h], 0.006);
    if matches!(ix, 3 | 4) {
        b.rbox(
            "metal",
            [x0 + 0.08, 0.06, h],
            [x1 - 0.08, 0.13, h + 0.002],
            0.0,
        );
        let n = ((w - 0.16) / 0.02).floor() as usize;
        for k in 0..n {
            let x = x0 + 0.09 + k as f64 * 0.02;
            b.rbox(
                "dark",
                [x, 0.07, h + 0.002],
                [x + 0.008, 0.12, h + 0.0025],
                0.0,
            );
        }
    } else {
        turned(
            b,
            "metal",
            [0.0, d / 2.0, h],
            &[(0.0, 0.0), (0.06, 0.0), (0.06, 0.002), (0.0, 0.003)],
            24,
            0.0,
        );
    }
    // What closes it: along the front, and on the right side.
    let top = 2.0;
    match look.screen {
        Screen::Glass | Screen::Black | Screen::Brass => {
            let frame = match look.screen {
                Screen::Glass => 0.0,
                _ => 0.025,
            };
            let role_f = "metal";
            // A fixed panel along the right side, a bar to the wall.
            b.rbox("glass", [x1 - 0.012, 0.0, h], [x1 - 0.004, d, top], 0.0);
            if frame > 0.0 {
                for (lo, hi) in [
                    ([x1 - 0.016, 0.0, h], [x1, frame, top]),
                    ([x1 - 0.016, d - frame, h], [x1, d, top]),
                    ([x1 - 0.016, 0.0, top - frame], [x1, d, top]),
                    ([x1 - 0.016, 0.0, h], [x1, d, h + frame]),
                ] {
                    b.rbox(role_f, lo, hi, 0.0);
                }
                if look.screen == Screen::Black {
                    b.rbox(
                        role_f,
                        [x1 - 0.016, 0.0, 1.0],
                        [x1, d, 1.0 + frame * 0.6],
                        0.0,
                    );
                }
            } else {
                b.rbox(role_f, [x1 - 0.016, 0.0, h], [x1, 0.025, top], 0.0);
                b.rbox(role_f, [x1 - 0.016, d - 0.02, h], [x1, d, h + 0.025], 0.0);
            }
            sweep(
                b,
                role_f,
                &[
                    [x1 - 0.008, d - 0.02, top - 0.02],
                    [x1 - 0.008, 0.0, top - 0.02],
                ],
                |_| 0.008,
                10,
                Cap::Flat,
                Cap::Flat,
            );
            // And half the front.
            let xm = x0 + w * 0.45;
            b.rbox(
                "glass",
                [xm, d - 0.012, h],
                [x1 - 0.012, d - 0.004, top],
                0.0,
            );
            if frame > 0.0 {
                b.rbox(role_f, [xm, d - 0.016, h], [xm + frame, d, top], 0.0);
                b.rbox(
                    role_f,
                    [xm, d - 0.016, top - frame],
                    [x1 - 0.012, d, top],
                    0.0,
                );
                b.rbox(role_f, [xm, d - 0.016, h], [x1 - 0.012, d, h + frame], 0.0);
            }
        }
        Screen::Grid => {
            // A steel window: the side and half the front, a frame and
            // glazing bars in squares.
            let xm = x0 + w * 0.45;
            for (side, a, e) in [(true, 0.0, d), (false, xm, x1)] {
                let s = 0.024;
                let (rows, cols) = (
                    5usize,
                    if side {
                        ((d / 0.3).round() as usize).max(2)
                    } else {
                        (((x1 - xm) / 0.3).round() as usize).max(1)
                    },
                );
                let put = |b: &mut Builder, lo: P3, hi: P3| b.rbox("metal", lo, hi, 0.0);
                if side {
                    b.rbox("glass", [x1 - 0.012, a, h], [x1 - 0.006, e, top], 0.0);
                    for r in 0..=rows {
                        let z = h + (top - h) * r as f64 / rows as f64;
                        put(
                            b,
                            [x1 - 0.016, a, z - s / 2.0],
                            [x1 - 0.002, e, z + s / 2.0],
                        );
                    }
                    for c in 0..=cols {
                        let y = a + (e - a) * c as f64 / cols as f64;
                        put(
                            b,
                            [x1 - 0.016, (y - s / 2.0).max(0.0), h],
                            [x1 - 0.002, (y + s / 2.0).min(d), top],
                        );
                    }
                } else {
                    b.rbox("glass", [a, d - 0.012, h], [e, d - 0.006, top], 0.0);
                    for r in 0..=rows {
                        let z = h + (top - h) * r as f64 / rows as f64;
                        put(b, [a, d - 0.016, z - s / 2.0], [e, d - 0.002, z + s / 2.0]);
                    }
                    for c in 0..=cols {
                        let x = a + (e - a) * c as f64 / cols as f64;
                        put(
                            b,
                            [(x - s / 2.0).max(a), d - 0.016, h],
                            [(x + s / 2.0).min(e), d - 0.002, top],
                        );
                    }
                }
            }
        }
        Screen::Sliding => {
            // Two panels of reeded glass in a chrome frame across the
            // front, overlapping in the middle.
            let s = 0.02;
            b.rbox("metal", [x0, d - 0.05, top - 0.04], [x1, d, top], 0.003);
            b.rbox("metal", [x0, d - 0.05, h], [x1, d, h + 0.025], 0.002);
            let m = w / 2.0 + 0.05;
            for (k, (a, e)) in [(x0 + 0.01, x0 + m), (x1 - m, x1 - 0.01)]
                .into_iter()
                .enumerate()
            {
                let y = d - 0.045 + k as f64 * 0.022;
                b.rbox("glass", [a, y, h + 0.025], [e, y + 0.008, top - 0.04], 0.0);
                for (lo, hi) in [
                    ([a, y - 0.004, h + 0.025], [a + s, y + 0.012, top - 0.04]),
                    ([e - s, y - 0.004, h + 0.025], [e, y + 0.012, top - 0.04]),
                ] {
                    b.rbox("metal", lo, hi, 0.002);
                }
                // The reeds.
                let n = ((e - a - 2.0 * s) / 0.03).floor() as usize;
                for i in 0..n {
                    let x = a + s + 0.015 + i as f64 * 0.03;
                    b.rbox(
                        "glass",
                        [x - 0.006, y + 0.008, h + 0.05],
                        [x + 0.006, y + 0.011, top - 0.065],
                        0.0,
                    );
                }
                sweep(
                    b,
                    "metal",
                    &[
                        [if k == 0 { e - 0.05 } else { a + 0.05 }, y + 0.012, 1.0],
                        [if k == 0 { e - 0.05 } else { a + 0.05 }, y + 0.012, 1.3],
                    ],
                    |_| 0.008,
                    10,
                    Cap::Round(1.0),
                    Cap::Round(1.0),
                );
            }
        }
        Screen::Curtain => {
            // A rail across the front and down the side, the curtain
            // gathered at one end.
            let zr = 2.0;
            let path = [
                [x0, d - 0.03, zr],
                [x1 - 0.03, d - 0.03, zr],
                [x1 - 0.03, 0.0, zr],
            ];
            sweep(b, "metal", &path, |_| 0.01, 12, Cap::Round(1.0), Cap::Flat);
            let n = 40;
            let len = w * 0.45;
            let pts: Vec<[f64; 2]> = (0..=n)
                .map(|i| {
                    let t = i as f64 / n as f64;
                    let x = x1 - 0.03 - len * t;
                    let y = d - 0.03 + 0.04 * (t * PI * 9.0).sin();
                    [x, y]
                })
                .collect();
            curtain(b, &pts, 0.08, zr - 0.02);
        }
    }
    // The head and the taps on the back wall.
    let xc = x0 + w * 0.35;
    match look.head {
        Head::Round | Head::Bell => {
            turned_y(b, "metal", [xc, 0.0, 1.1], 0.06, 0.008);
            sweep(
                b,
                "metal",
                &[[xc + 0.02, 0.008, 1.1], [xc + 0.02, 0.06, 1.1]],
                |_| 0.008,
                10,
                Cap::Flat,
                Cap::Round(1.0),
            );
            let arm = curve(&[[xc, 0.0, 2.08], [xc, 0.2, 2.1], [xc, 0.36, 2.08]], 4);
            sweep(b, "metal", &arm, |_| 0.011, 12, Cap::Flat, Cap::Flat);
            if look.head == Head::Bell {
                turned(
                    b,
                    "metal",
                    [xc, 0.36, 1.98],
                    &[
                        (0.0, 0.0),
                        (0.075, 0.0),
                        (0.06, 0.04),
                        (0.02, 0.09),
                        (0.012, 0.1),
                        (0.0, 0.1),
                    ],
                    24,
                    0.0,
                );
            } else {
                turned(
                    b,
                    "metal",
                    [xc, 0.36, 2.04],
                    &[(0.0, 0.0), (0.12, 0.0), (0.12, 0.012), (0.0, 0.02)],
                    32,
                    0.0,
                );
            }
        }
        Head::Square => {
            b.rbox(
                "metal",
                [xc - 0.15, 0.25, 2.3],
                [xc + 0.15, 0.55, 2.31],
                0.002,
            );
            bar(
                b,
                "metal",
                [xc, 0.4, 2.31],
                [xc, 0.4, 2.5],
                0.025,
                [1.0, 0.0, 0.0],
            );
            b.rbox(
                "metal",
                [xc - 0.08, 0.0, 1.05],
                [xc + 0.08, 0.008, 1.2],
                0.002,
            );
            b.rbox(
                "metal",
                [xc + 0.03, 0.008, 1.12],
                [xc + 0.05, 0.05, 1.128],
                0.001,
            );
        }
        Head::Exposed => {
            // A bar mixer across the wall with two cross handles, the
            // riser to the rose.
            sweep(
                b,
                "metal",
                &[[xc - 0.16, 0.06, 1.1], [xc + 0.16, 0.06, 1.1]],
                |_| 0.02,
                16,
                Cap::Round(1.0),
                Cap::Round(1.0),
            );
            for x in [xc - 0.15, xc + 0.15] {
                sweep(
                    b,
                    "metal",
                    &[[x, 0.0, 1.1], [x, 0.06, 1.1]],
                    |_| 0.012,
                    10,
                    Cap::Flat,
                    Cap::Flat,
                );
                for k in 0..4 {
                    let a = PI / 4.0 + PI / 2.0 * k as f64;
                    let e = [x + 0.035 * a.cos(), 0.075, 1.1 + 0.035 * a.sin()];
                    sweep(
                        b,
                        "metal",
                        &[[x, 0.075, 1.1], e],
                        |_| 0.005,
                        8,
                        Cap::Flat,
                        Cap::Round(1.2),
                    );
                }
            }
            sweep(
                b,
                "metal",
                &[[xc, 0.06, 1.12], [xc, 0.06, 2.05]],
                |_| 0.012,
                12,
                Cap::Flat,
                Cap::Flat,
            );
            let arm = curve(
                &[
                    [xc, 0.06, 2.05],
                    [xc, 0.06, 2.12],
                    [xc, 0.15, 2.15],
                    [xc, 0.3, 2.12],
                ],
                4,
            );
            sweep(b, "metal", &arm, |_| 0.011, 12, Cap::Flat, Cap::Flat);
            turned(
                b,
                "metal",
                [xc, 0.3, 2.04],
                &[
                    (0.0, 0.0),
                    (0.11, 0.0),
                    (0.1, 0.03),
                    (0.02, 0.07),
                    (0.0, 0.08),
                ],
                28,
                0.0,
            );
            // A hand shower on its hook.
            sweep(
                b,
                "metal",
                &[[xc + 0.06, 0.05, 1.4], [xc + 0.06, 0.05, 1.62]],
                |_| 0.012,
                10,
                Cap::Round(1.0),
                Cap::Round(1.0),
            );
        }
    }
    true
}

/// A curtain hanging along `path` (in plan, from its rail at `zr`) to
/// `z0`, of the shower curtain's cloth.
fn curtain(b: &mut Builder, path: &[[f64; 2]], z0: f64, zr: f64) {
    let t = 0.004;
    let n = path.len();
    let normal = |i: usize| {
        let (p, q) = (path[i.saturating_sub(1)], path[(i + 1).min(n - 1)]);
        let (dx, dy) = (q[0] - p[0], q[1] - p[1]);
        let l = (dx * dx + dy * dy).sqrt().max(1e-9);
        [-dy / l, dx / l]
    };
    let side = |b: &mut Builder, s: f64, z: f64| -> Vec<u32> {
        (0..n)
            .map(|i| {
                let m = normal(i);
                b.point([path[i][0] + m[0] * t * s, path[i][1] + m[1] * t * s, z])
            })
            .collect()
    };
    let (o0, o1, i0, i1) = (
        side(b, -1.0, z0),
        side(b, -1.0, zr),
        side(b, 1.0, z0),
        side(b, 1.0, zr),
    );
    for k in 0..n - 1 {
        b.quad("towel", o0[k], o0[k + 1], o1[k + 1], o1[k]);
        b.quad("towel", i0[k + 1], i0[k], i1[k], i1[k + 1]);
        b.quad("towel", o1[k], o1[k + 1], i1[k + 1], i1[k]);
        b.quad("towel", i0[k], i0[k + 1], o0[k + 1], o0[k]);
    }
    b.quad("towel", i0[0], o0[0], o1[0], i1[0]);
    b.quad("towel", o0[n - 1], i0[n - 1], i1[n - 1], o1[n - 1]);
}

// -------------------------------------------------------------- baths

/// A bath made in a collection (see `sanitary::bath`): `width` its
/// length, `depth` across, `height` its rim's.
pub fn bath(b: &mut Builder, p: &Params) -> bool {
    let Some((ix, look)) = look(p, 1) else {
        return false;
    };
    let l = p.get("width", 1.7);
    let wd = p.get("depth", 0.7);
    let h = p.get("height", 0.55);
    let (x0, x1) = (-l / 2.0, l / 2.0);
    match look.tub {
        Tub::Oval | Tub::Claw => {
            let claw = look.tub == Tub::Claw;
            let base = if claw { 0.14 } else { 0.0 };
            let hh = h + if claw { 0.12 } else { 0.0 } - base;
            // A shell turned on a unit radius and stretched to the tub.
            let prof: Vec<(f64, f64)> = if claw {
                vec![
                    (0.0, 0.0),
                    (0.62, 0.0),
                    (0.82, hh * 0.25),
                    (0.95, hh * 0.8),
                    (1.0, hh),
                    (0.985, hh + 0.025),
                    (0.95, hh + 0.02),
                    (0.93, hh * 0.85),
                    (0.83, hh * 0.3),
                    (0.6, 0.04),
                    (0.0, 0.04),
                ]
            } else {
                vec![
                    (0.0, 0.0),
                    (0.7, 0.0),
                    (0.88, hh * 0.2),
                    (0.98, hh * 0.75),
                    (1.0, hh),
                    (0.965, hh),
                    (0.94, hh * 0.75),
                    (0.84, hh * 0.25),
                    (0.66, 0.05),
                    (0.0, 0.05),
                ]
            };
            b.lathe("mat", [0.0, wd / 2.0, base], &prof, l / 2.0, wd / 2.0);
            if claw {
                // Four claw feet, from inside the shell's underside down to
                // the floor, splaying out.
                for (sx, sy) in [(-1.0, -1.0), (1.0, -1.0), (-1.0, 1.0), (1.0, 1.0)] {
                    let (x, y) = (sx * l * 0.3, wd / 2.0 + sy * wd * 0.22);
                    let leg = curve(
                        &[
                            [x, y, base + 0.12],
                            [x + sx * 0.01, y + sy * 0.015, base * 0.6],
                            [x + sx * 0.03, y + sy * 0.05, 0.035],
                        ],
                        4,
                    );
                    sweep(
                        b,
                        "metal",
                        &leg,
                        |t| 0.034 - 0.014 * t,
                        12,
                        Cap::Flat,
                        Cap::Flat,
                    );
                    b.ellipsoid(
                        "metal",
                        [x + sx * 0.035, y + sy * 0.055, 0.025],
                        [0.04, 0.045, 0.026],
                    );
                }
                // The tap on the rim at the wall end: a bridge, or a pipe.
                let tap = if ix == 6 { Tap::Spring } else { Tap::Bridge };
                tap_scaled(b, tap, [x0 + 0.12, wd / 2.0, base + hh + 0.02], 0.2, 0.6);
            } else {
                // A tall filler on the floor at its end.
                floor_filler(b, x0 - 0.1, wd / 2.0, ix == 4);
            }
        }
        Tub::Built => {
            // A base built round the tub, the tub set in it.
            let rim = 0.08;
            let (hx0, hx1, hy0, hy1) = (x0 + rim + 0.04, x1 - rim, rim, wd - rim);
            b.framed_top("unit", [x0, 0.0, 0.0], [x1, wd, h], [hx0, hy0], [hx1, hy1]);
            let c = [(hx0 + hx1) / 2.0, (hy0 + hy1) / 2.0];
            under_bowl(
                b,
                "mat",
                c,
                (hx1 - hx0) / 2.0,
                (hy1 - hy0) / 2.0,
                0.12,
                h,
                h - 0.06,
                0.01,
            );
            if ix == 2 {
                // The fifties: a band of tiles along the front panel.
                for k in 0..((l / 0.15) as usize) {
                    let x = x0 + 0.075 + k as f64 * 0.15;
                    b.rbox(
                        "dark",
                        [x - 0.001, wd, 0.02],
                        [x + 0.001, wd + 0.001, h - 0.02],
                        0.0,
                    );
                }
            }
            wall_tap(b, if ix == 2 { 0 } else { ix }, x0 + 0.25, h + 0.2, 0.18);
            if look.tap.is_some() && ix == 2 {
                tap_scaled(b, Tap::Twin, [x0 + 0.06, wd / 2.0, h], 0.2, 0.7);
            }
        }
        Tub::Ofuro => {
            // A deep square tub of wood, its staves and two bands, and a
            // stool beside it.
            let (tx0, tx1) = (x0 + 0.02, x0 + 0.02 + (l * 0.65).min(1.2));
            let deep = h.max(0.6);
            let t = 0.04;
            for (lo, hi) in [
                ([tx0, 0.0, 0.0], [tx1, t, deep]),
                ([tx0, wd - t, 0.0], [tx1, wd, deep]),
                ([tx0, t, 0.0], [tx0 + t, wd - t, deep]),
                ([tx1 - t, t, 0.0], [tx1, wd - t, deep]),
                ([tx0 + t, t, 0.0], [tx1 - t, wd - t, 0.05]),
            ] {
                b.rbox("wood", lo, hi, 0.004);
            }
            let n = ((tx1 - tx0) / 0.09).floor() as usize;
            for k in 1..n {
                let x = tx0 + k as f64 * (tx1 - tx0) / n as f64;
                b.rbox(
                    "dark",
                    [x - 0.001, wd, 0.0],
                    [x + 0.001, wd + 0.001, deep],
                    0.0,
                );
            }
            for z in [0.1, deep - 0.12] {
                b.rbox(
                    "metal",
                    [tx0 - 0.004, wd, z],
                    [tx1 + 0.004, wd + 0.004, z + 0.025],
                    0.0,
                );
            }
            // The stool.
            let sx = tx1 + 0.25;
            b.rbox(
                "wood",
                [sx - 0.18, wd * 0.3, 0.38],
                [sx + 0.18, wd * 0.75, 0.42],
                0.006,
            );
            for x in [sx - 0.15, sx + 0.12] {
                b.rbox(
                    "wood",
                    [x, wd * 0.32, 0.0],
                    [x + 0.03, wd * 0.73, 0.38],
                    0.004,
                );
            }
            wall_tap(b, ix, (tx0 + tx1) / 2.0, deep + 0.2, 0.16);
        }
    }
    true
}

/// A bath filler standing on the floor at `(x, y)`, its spout reaching
/// over the tub (towards `+x`): a column, its spout and lever.
fn floor_filler(b: &mut Builder, x: f64, y: f64, square: bool) {
    let top = 0.95;
    if square {
        b.rbox(
            "metal",
            [x - 0.05, y - 0.05, 0.0],
            [x + 0.05, y + 0.05, 0.01],
            0.002,
        );
        bar(b, "metal", [x, y, 0.0], [x, y, top], 0.04, [1.0, 0.0, 0.0]);
        bar(
            b,
            "metal",
            [x - 0.02, y, top - 0.02],
            [x + 0.25, y, top - 0.02],
            0.04,
            [0.0, 1.0, 0.0],
        );
        b.rbox(
            "metal",
            [x - 0.006, y + 0.02, top - 0.25],
            [x + 0.006, y + 0.08, top - 0.24],
            0.001,
        );
        return;
    }
    turned(
        b,
        "metal",
        [x, y, 0.0],
        &[
            (0.0, 0.0),
            (0.06, 0.0),
            (0.06, 0.008),
            (0.022, 0.014),
            (0.02, top - 0.05),
            (0.0, top - 0.05),
        ],
        24,
        0.0,
    );
    let spout = curve(
        &[
            [x, y, top - 0.06],
            [x, y, top],
            [x + 0.08, y, top + 0.03],
            [x + 0.2, y, top - 0.04],
        ],
        6,
    );
    sweep(b, "metal", &spout, |_| 0.016, 16, Cap::Flat, Cap::Flat);
    sweep(
        b,
        "metal",
        &[[x, y, top - 0.3], [x - 0.05, y + 0.03, top - 0.22]],
        |_| 0.007,
        10,
        Cap::Flat,
        Cap::Round(1.0),
    );
    // The hand shower on its cradle.
    sweep(
        b,
        "metal",
        &[
            [x - 0.03, y - 0.02, top - 0.55],
            [x - 0.03, y - 0.02, top - 0.32],
        ],
        |_| 0.016,
        12,
        Cap::Round(1.0),
        Cap::Round(1.0),
    );
}

// ------------------------------------------------------------ mirrors

/// A mirror made in a collection (see `accessories::mirror`).
pub fn mirror(b: &mut Builder, p: &Params) -> bool {
    let Some((_, look)) = look(p, 3) else {
        return false;
    };
    let w = p.get("width", 0.8);
    let hh = p.get("height", 1.0);
    let z0 = p.get("elevation", 1.0);
    let (x0, x1) = (-w / 2.0, w / 2.0);
    let disc =
        |b: &mut Builder, role: &'static str, r: f64, c: P3, y0: f64, y1: f64, seg: usize| {
            let pts: Vec<[f64; 2]> = (0..seg)
                .map(|k| {
                    let a = 2.0 * PI * k as f64 / seg as f64;
                    [c[0] + r * a.cos(), c[2] + r * a.sin()]
                })
                .collect();
            front_profile(b, role, &pts, y0, y1);
        };
    match look.mirror {
        Mirror::RoundWood | Mirror::RoundChrome => {
            let r = w.min(hh) / 2.0;
            let c = [0.0, 0.0, z0 + r];
            let rim = if look.mirror == Mirror::RoundWood {
                0.04
            } else {
                0.015
            };
            let role = if look.mirror == Mirror::RoundWood {
                "frame"
            } else {
                "metal"
            };
            let ring: Vec<P3> = (0..48)
                .map(|k| {
                    let a = 2.0 * PI * k as f64 / 48.0;
                    [r * a.cos(), 0.02, c[2] + r * a.sin()]
                })
                .collect();
            super::soft::tube(b, role, &ring, true, rim / 2.0 + 0.004, 10);
            disc(b, "glass", r - rim * 0.3, c, 0.0, 0.016, 48);
            if look.mirror == Mirror::RoundWood {
                // Hung on a leather strap from a peg.
                b.rbox(
                    "dark",
                    [-0.012, 0.004, c[2] + r],
                    [0.012, 0.01, c[2] + r + 0.25],
                    0.002,
                );
                sweep(
                    b,
                    "frame",
                    &[[0.0, 0.0, c[2] + r + 0.26], [0.0, 0.05, c[2] + r + 0.26]],
                    |_| 0.012,
                    10,
                    Cap::Flat,
                    Cap::Round(1.0),
                );
            } else {
                // A light over it.
                b.rbox(
                    "metal",
                    [-0.18, 0.0, c[2] + r + 0.05],
                    [0.18, 0.08, c[2] + r + 0.09],
                    0.02,
                );
                b.rbox(
                    "light",
                    [-0.17, 0.02, c[2] + r + 0.045],
                    [0.17, 0.075, c[2] + r + 0.05],
                    0.0,
                );
            }
        }
        Mirror::Capsule => {
            let r = w / 2.0;
            let mut pts: Vec<[f64; 2]> = Vec::new();
            for k in 0..=16 {
                let a = PI * k as f64 / 16.0;
                pts.push([r * a.cos(), z0 + hh - r + r * a.sin()]);
            }
            for k in 0..=16 {
                let a = PI + PI * k as f64 / 16.0;
                pts.push([r * a.cos(), z0 + r + r * a.sin()]);
            }
            front_profile(b, "frame", &pts, 0.0, 0.012);
            let inner: Vec<[f64; 2]> = pts
                .iter()
                .map(|q| [q[0] * 0.96, (q[1] - (z0 + hh / 2.0)) * 0.98 + z0 + hh / 2.0])
                .collect();
            front_profile(b, "glass", &inner, 0.012, 0.016);
        }
        Mirror::Arched => {
            let rr = w / 2.0;
            let zs = z0 + hh - rr;
            let mut pts: Vec<[f64; 2]> = vec![[x0, z0], [x1, z0]];
            for k in 0..=20 {
                let a = PI * k as f64 / 20.0;
                pts.push([rr * a.cos(), zs + rr * a.sin()]);
            }
            front_profile(b, "frame", &pts, 0.0, 0.025);
            let inner: Vec<[f64; 2]> = vec![[x0 + 0.05, z0 + 0.05], [x1 - 0.05, z0 + 0.05]]
                .into_iter()
                .chain((0..=20).map(|k| {
                    let a = PI * k as f64 / 20.0;
                    [(rr - 0.05) * a.cos(), zs + (rr - 0.05) * a.sin()]
                }))
                .collect();
            front_profile(b, "glass", &inner, 0.025, 0.029);
        }
        Mirror::Lit => {
            b.rbox("glass", [x0, 0.02, z0], [x1, 0.026, z0 + hh], 0.004);
            b.rbox(
                "dark",
                [x0 + 0.02, 0.0, z0 + 0.02],
                [x1 - 0.02, 0.02, z0 + hh - 0.02],
                0.0,
            );
            // A band of light round its edge, behind.
            b.rbox(
                "light",
                [x0 + 0.005, 0.018, z0 + 0.005],
                [x1 - 0.005, 0.02, z0 + hh - 0.005],
                0.0,
            );
        }
        Mirror::RectWood | Mirror::RectSteel | Mirror::Ornate => {
            let (f, depth, role) = match look.mirror {
                Mirror::RectWood => (0.07, 0.035, "frame"),
                Mirror::RectSteel => (0.018, 0.02, "metal"),
                _ => (0.08, 0.04, "frame"),
            };
            for (lo, hi) in [
                ([x0, 0.0, z0], [x1, depth, z0 + f]),
                ([x0, 0.0, z0 + hh - f], [x1, depth, z0 + hh]),
                ([x0, 0.0, z0 + f], [x0 + f, depth, z0 + hh - f]),
                ([x1 - f, 0.0, z0 + f], [x1, depth, z0 + hh - f]),
            ] {
                b.rbox(
                    role,
                    lo,
                    hi,
                    if look.mirror == Mirror::RectSteel {
                        0.0
                    } else {
                        0.006
                    },
                );
            }
            b.rbox(
                "glass",
                [x0 + f, 0.006, z0 + f],
                [x1 - f, 0.012, z0 + hh - f],
                0.0,
            );
            if look.mirror == Mirror::Ornate {
                // An inner bead and a crest over it.
                let ring = [
                    [x0 + f, depth, z0 + f],
                    [x1 - f, depth, z0 + f],
                    [x1 - f, depth, z0 + hh - f],
                    [x0 + f, depth, z0 + hh - f],
                ];
                super::soft::tube(b, "frame", &ring, true, 0.008, 6);
                let crest: Vec<[f64; 2]> = (0..=16)
                    .map(|k| {
                        let a = PI * k as f64 / 16.0;
                        [w * 0.28 * a.cos(), z0 + hh + 0.12 * a.sin()]
                    })
                    .collect();
                front_profile(b, "frame", &crest, 0.0, depth * 0.7);
                b.ellipsoid(
                    "frame",
                    [0.0, depth * 0.7, z0 + hh + 0.13],
                    [0.035, 0.02, 0.035],
                );
            }
            if look.mirror == Mirror::RectSteel {
                // Hung from a rod by two straps.
                sweep(
                    b,
                    "metal",
                    &[
                        [x0 - 0.05, 0.05, z0 + hh + 0.3],
                        [x1 + 0.05, 0.05, z0 + hh + 0.3],
                    ],
                    |_| 0.01,
                    10,
                    Cap::Round(1.0),
                    Cap::Round(1.0),
                );
                for x in [x0 + 0.1, x1 - 0.1] {
                    b.rbox(
                        "dark",
                        [x - 0.01, 0.03, z0 + hh],
                        [x + 0.01, 0.04, z0 + hh + 0.3],
                        0.0,
                    );
                }
            }
        }
    }
    true
}

// -------------------------------------------------------- towel rails

/// A towel rail made in a collection (see `accessories::towel_rail`),
/// with a towel on it.
pub fn towel_rail(b: &mut Builder, p: &Params) -> bool {
    let Some((_, look)) = look(p, 2) else {
        return false;
    };
    let w = p.get("width", 0.6);
    let z = p.get("elevation", 1.1);
    let hh = p.get("height", 0.9);
    let (x0, x1) = (-w / 2.0, w / 2.0);
    match look.rail {
        Rail::Ladder => {
            // Leaning on the wall from the floor: two rails, rungs.
            let top = (z + 0.4).max(1.5);
            let foot = 0.32;
            for x in [x0 + 0.02, x1 - 0.02] {
                bar(
                    b,
                    "wood",
                    // Its foot's corner on the floor, not under it.
                    [x, foot, 0.006],
                    [x, 0.03, top],
                    0.035,
                    [1.0, 0.0, 0.0],
                );
            }
            let n = 5;
            for k in 0..n {
                let t = (k as f64 + 0.6) / n as f64;
                let (y, zz) = (foot + (0.03 - foot) * t, top * t);
                sweep(
                    b,
                    "wood",
                    &[[x0 + 0.02, y, zz], [x1 - 0.02, y, zz]],
                    |_| 0.012,
                    10,
                    Cap::Flat,
                    Cap::Flat,
                );
                if k == 2 {
                    towel_over(b, x0 + 0.06, x1 - 0.06, y, zz, 0.35);
                }
            }
        }
        Rail::Bar => {
            for x in [x0 + 0.02, x1 - 0.02] {
                turned_y(b, "metal", [x, 0.0, z], 0.022, 0.008);
                sweep(
                    b,
                    "metal",
                    &[[x, 0.008, z], [x, 0.08, z]],
                    |_| 0.009,
                    12,
                    Cap::Flat,
                    Cap::Flat,
                );
            }
            sweep(
                b,
                "metal",
                &[[x0 + 0.01, 0.08, z], [x1 - 0.01, 0.08, z]],
                |_| 0.011,
                14,
                Cap::Round(1.0),
                Cap::Round(1.0),
            );
            towel_over(b, x0 + 0.08, x1 - 0.08, 0.08, z, 0.42);
        }
        Rail::Ring => {
            turned_y(b, "metal", [0.0, 0.0, z], 0.03, 0.012);
            sweep(
                b,
                "metal",
                &[[0.0, 0.012, z], [0.0, 0.05, z]],
                |_| 0.008,
                10,
                Cap::Flat,
                Cap::Flat,
            );
            let r = 0.09;
            let ring: Vec<P3> = (0..28)
                .map(|k| {
                    let a = 2.0 * PI * k as f64 / 28.0;
                    [r * a.sin(), 0.055, z - r + r * a.cos()]
                })
                .collect();
            super::soft::tube(b, "metal", &ring, true, 0.006, 8);
            towel_over(b, -0.12, 0.12, 0.055, z - 2.0 * r + 0.006, 0.3);
        }
        Rail::Pipes => {
            let top = z + hh * 0.5;
            for x in [x0 + 0.02, x1 - 0.02] {
                for zz in [z - hh * 0.4, top] {
                    turned_y(b, "metal", [x, 0.0, zz], 0.025, 0.012);
                    sweep(
                        b,
                        "metal",
                        &[[x, 0.012, zz], [x, 0.09, zz]],
                        |_| 0.012,
                        12,
                        Cap::Flat,
                        Cap::Flat,
                    );
                }
                sweep(
                    b,
                    "metal",
                    &[[x, 0.09, z - hh * 0.4], [x, 0.09, top]],
                    |_| 0.012,
                    12,
                    Cap::Flat,
                    Cap::Flat,
                );
            }
            for k in 0..4 {
                let zz = z - hh * 0.4 + (k as f64 + 0.5) * (hh * 0.9) / 4.0;
                sweep(
                    b,
                    "metal",
                    &[[x0 + 0.02, 0.09, zz], [x1 - 0.02, 0.09, zz]],
                    |_| 0.01,
                    12,
                    Cap::Flat,
                    Cap::Flat,
                );
                if k == 2 {
                    towel_over(b, x0 + 0.06, x1 - 0.06, 0.09, zz, 0.3);
                }
            }
        }
        Rail::Columns => {
            // A heated rail: columns between two headers, a top rail.
            let (za, zb) = (z - hh * 0.45, z + hh * 0.45);
            for zz in [za, zb] {
                sweep(
                    b,
                    "metal",
                    &[[x0, 0.06, zz], [x1, 0.06, zz]],
                    |_| 0.02,
                    14,
                    Cap::Round(1.0),
                    Cap::Round(1.0),
                );
            }
            let n = ((w - 0.04) / 0.05).floor() as usize;
            for k in 0..n {
                let x = x0 + 0.03 + k as f64 * (w - 0.06) / (n - 1).max(1) as f64;
                sweep(
                    b,
                    "metal",
                    &[[x, 0.06, za], [x, 0.06, zb]],
                    |_| 0.012,
                    10,
                    Cap::Flat,
                    Cap::Flat,
                );
            }
            sweep(
                b,
                "metal",
                &[[x0, 0.12, zb + 0.06], [x1, 0.12, zb + 0.06]],
                |_| 0.012,
                12,
                Cap::Round(1.0),
                Cap::Round(1.0),
            );
            for x in [x0 + 0.03, x1 - 0.03] {
                sweep(
                    b,
                    "metal",
                    &[[x, 0.06, zb], [x, 0.12, zb + 0.06]],
                    |_| 0.008,
                    10,
                    Cap::Flat,
                    Cap::Flat,
                );
                sweep(
                    b,
                    "metal",
                    &[[x, 0.0, za + 0.05], [x, 0.06, za + 0.05]],
                    |_| 0.008,
                    10,
                    Cap::Flat,
                    Cap::Flat,
                );
            }
            towel_over(b, x0 + 0.06, x1 - 0.06, 0.12, zb + 0.06, 0.36);
        }
    }
    true
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

    /// Every piece of every collection is made, closed outwards and light.
    #[test]
    fn the_bathrooms_of_the_eight_collections() {
        type Make = fn(&mut Builder, &Params) -> bool;
        let pieces: [(&str, Make, i32); 7] = [
            ("lavabo", washbasin, 3),
            ("inodoro", wc, 2),
            ("bidé", bidet, 2),
            ("ducha", shower, 3),
            ("bañera", bath, 1),
            ("espejo", mirror, 3),
            ("toallero", towel_rail, 2),
        ];
        for c in 0..8 {
            for (name, make, first) in pieces {
                let mut b = Builder::default();
                assert!(make(&mut b, &params(first + 1 + c, &[])), "{name}");
                let tris = b.triangle_count();
                assert!(
                    (50..30_000).contains(&tris),
                    "{} {name}: {tris}",
                    COLLECTIONS[c as usize]
                );
                for (role, t) in &b.parts {
                    let v = signed_volume(&b.positions, t);
                    assert!(v > -1e-7, "{} {name} {role}: {v}", COLLECTIONS[c as usize]);
                }
            }
        }
        let mut b = Builder::default();
        assert!(!washbasin(&mut b, &params(3, &[])));
        assert!(!bath(&mut b, &params(1, &[])));
    }
}
