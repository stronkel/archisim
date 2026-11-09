//! The second round of DB-HE 1 over the envelope worked out by
//! [`crate::thermal`]:
//!
//! - **Linear thermal bridges**: a ψ by class of junction
//!   ([`Junction`]) and by where the façade's insulation is
//!   ([`insulation_of`], from the layers of the wall type), with the
//!   lengths measured on the drawing. They enter K = (Σ U·A + Σ ψ·L) / Σ A
//!   and the heating load.
//! - **The ground** by UNE-EN ISO 13370 ([`slab_on_ground`],
//!   [`basement_wall`]): the equivalent U of a slab on the ground or of a
//!   basement floor by its characteristic dimension B' = A / (0,5·P), the
//!   equivalent thickness and the edge insulation, and the U of the walls
//!   against the ground by their depth.
//! - **Solar control** of July, q_sol;jul (HE 1, 3.1.2): the July
//!   radiation by orientation, the glass's solar factor with its movable
//!   shading drawn, the frame and what the slabs, balconies, eaves and the
//!   reveal above each window shade ([`shade_factor`]).
//! - **Air permeability** (HE 1, 3.1.3): the class of the windows against
//!   table 3.1.3.a, and n50 by the simplified method of anejo H against
//!   table 3.1.3.b.
//! - **Surface condensation**: fRsi = 1 − 0,25·U of every opaque type and
//!   the fRsi of every class of bridge against fRsi,min by the hygrometry
//!   class and the winter zone.
//!
//! **Written from memory, to be checked by Antonio**: every table below
//! (the default ψ and fRsi of the bridges, the July radiation, the limits
//! of q_sol;jul, of the permeability and of n50, the coefficients of
//! anejo H and fRsi,min). They are together here.

use archisim_core::{EdgeInsulation, Insulation, Junction, Layer, LayerFunction, Model};

use crate::thermal::Facing;

/// Default ψ of a linear thermal bridge, W/mK, measured with the inside
/// dimensions of the rooms (as the areas are): one row per [`Junction`]
/// (slab front, pillar, opening perimeter, outer corner, inner corner,
/// roof, ground) and one column per [`Insulation`] (outside, inside, in
/// the cavity, none). Usual values of DA DB-HE/3 and the catalogue of
/// thermal bridges, from memory.
pub const PSI: [[f64; 4]; 7] = [
    [0.10, 0.85, 0.80, 0.90],
    [0.05, 0.90, 0.80, 0.90],
    [0.25, 0.15, 0.20, 0.30],
    [0.10, 0.05, 0.08, 0.15],
    [-0.05, -0.05, -0.05, -0.10],
    [0.40, 0.60, 0.55, 0.70],
    [0.15, 0.55, 0.50, 0.70],
];

/// The inside surface temperature factor fRsi of each class of bridge,
/// same rows and columns as [`PSI`] (from memory).
pub const F_RSI: [[f64; 4]; 7] = [
    [0.90, 0.65, 0.70, 0.60],
    [0.90, 0.60, 0.65, 0.55],
    [0.75, 0.75, 0.75, 0.65],
    [0.85, 0.80, 0.80, 0.70],
    [0.95, 0.95, 0.95, 0.90],
    [0.80, 0.70, 0.70, 0.60],
    [0.85, 0.70, 0.70, 0.65],
];

/// The inside surface resistance for condensation, m²K/W (UNE-EN ISO
/// 13788).
pub const RSI_CONDENSATION: f64 = 0.25;
/// fRsi,min (DA DB-HE/2, table 1) for hygrometry class 3 or less by winter
/// zone (α, A, B, C, D, E); class 4 and class 5.
pub const F_RSI_MIN: [f64; 6] = [0.50, 0.50, 0.52, 0.56, 0.61, 0.64];
pub const F_RSI_MIN_4: f64 = 0.85;
pub const F_RSI_MIN_5: f64 = 0.90;

/// UNE-EN ISO 13370: the surface resistances of a floor (heat going down)
/// and of a wall, and the outside one, m²K/W.
pub const GROUND_RSI: f64 = 0.17;
pub const GROUND_RSI_WALL: f64 = 0.13;
pub const GROUND_RSE: f64 = 0.04;

/// July radiation on a surface, kWh/m²·month (DA DB-HE/1), by the summer
/// number of the zone (1 to 4) and the orientation (N, E, S, W and
/// horizontal). From memory.
pub const H_SOL_JUL: [[f64; 5]; 4] = [
    [50.0, 115.0, 85.0, 115.0, 200.0],
    [52.0, 125.0, 88.0, 125.0, 215.0],
    [55.0, 135.0, 92.0, 135.0, 230.0],
    [58.0, 145.0, 95.0, 145.0, 245.0],
];
/// Limit of q_sol;jul, kWh/m²·month (HE 1, table 3.1.2): private
/// residential, and every other use.
pub const Q_SOL_LIMIT_DWELLING: f64 = 2.0;
pub const Q_SOL_LIMIT_OTHER: f64 = 4.0;
/// The part of the July radiation on a façade that comes straight from
/// the sun (the part an overhang can stop): N, E, S, W, horizontal.
pub const DIRECT_SHARE: [f64; 5] = [0.3, 0.7, 0.6, 0.7, 0.7];
/// The tangent of the sun's profile angle, in July, on an east or west
/// façade (mid-morning and mid-afternoon) and on a north one (early and
/// late); on the south it is worked out from the latitude at noon.
pub const PROFILE_EAST_WEST: f64 = 1.2;
pub const PROFILE_NORTH: f64 = 3.0;
/// The sun's declination in mid-July, degrees.
pub const JULY_DECLINATION: f64 = 21.5;

/// Air permeability of a window at 100 Pa, m³/h·m², by its class (UNE-EN
/// 12207, 1 to 4).
pub const Q100: [f64; 4] = [50.0, 27.0, 9.0, 3.0];
/// Limit of the windows' permeability at 100 Pa (HE 1, table 3.1.3.a), by
/// winter zone (α, A, B, C, D, E).
pub const Q100_LIMIT: [f64; 6] = [27.0, 27.0, 27.0, 9.0, 9.0, 9.0];
/// HE 1, table 3.1.3.b: the limit of n50, h⁻¹, at V/A ≤ 2 and at V/A ≥ 4
/// (straight between), for dwellings of more than this useful area, m².
pub const N50_COMPACT_LOW: f64 = 6.0;
pub const N50_COMPACT_HIGH: f64 = 3.0;
pub const N50_FROM_AREA: f64 = 120.0;
/// Anejo H: n50 = 0,629 · (Co·Ao + Σ Ch·Ah) / V (the flows at 100 Pa
/// taken to 50 Pa with the exponent 0,67).
pub const N50_FACTOR: f64 = 0.629;

/// The default ψ of a class of bridge.
pub fn usual_psi(j: Junction, i: Insulation) -> f64 {
    PSI[j.index()][i.index()]
}

/// The ψ of a class: the one set for the project, or the usual one.
pub fn psi(model: &Model, j: Junction, i: Insulation) -> f64 {
    model
        .thermal_site()
        .psi_set(j, i)
        .unwrap_or_else(|| usual_psi(j, i))
}

pub fn f_rsi_bridge(j: Junction, i: Insulation) -> f64 {
    F_RSI[j.index()][i.index()]
}

/// fRsi of a plain element of transmittance `u`.
pub fn f_rsi(u: f64) -> f64 {
    1.0 - RSI_CONDENSATION * u
}

/// fRsi,min by the winter zone's column and the hygrometry class.
pub fn f_rsi_min(zone: usize, hygrometry: u8) -> f64 {
    match hygrometry {
        5 => F_RSI_MIN_5,
        4 => F_RSI_MIN_4,
        _ => F_RSI_MIN[zone],
    }
}

/// Whether a layer insulates: its function says so, or it is at least
/// 2 cm of a material under 0,06 W/mK.
fn insulates(model: &Model, l: &Layer) -> bool {
    l.function == LayerFunction::Insulation
        || (l.function != LayerFunction::AirGap
            && l.thickness >= 0.02
            && model.conductivity(l.material) <= 0.06)
}

/// Where the insulation of a wall is, from its layers (outside first): a
/// leaf (4 cm or more of a material that does not insulate) on both sides
/// is a cavity, only inside is outside, only outside is inside.
pub fn insulation_of(model: &Model, layers: &[Layer]) -> Insulation {
    let ins: Vec<usize> = (0..layers.len())
        .filter(|i| insulates(model, &layers[*i]))
        .collect();
    let (Some(first), Some(last)) = (ins.first(), ins.last()) else {
        return Insulation::None;
    };
    let leaf = |l: &Layer| {
        !insulates(model, l)
            && l.function != LayerFunction::AirGap
            && l.thickness >= 0.04
            && model.conductivity(l.material) >= 0.15
    };
    let outside = layers[..*first].iter().any(leaf);
    let inside = layers[last + 1..].iter().any(leaf);
    match (outside, inside) {
        (true, true) => Insulation::Middle,
        (true, false) => Insulation::Inside,
        _ => Insulation::Outside,
    }
}

/// A floor on the ground (or a basement floor) by UNE-EN ISO 13370.
#[derive(Debug, Clone, PartialEq)]
pub struct GroundFloor {
    /// Its area, m², and its exposed perimeter, m (the length of façade
    /// around it: party walls are left out).
    pub area: f64,
    pub perimeter: f64,
    /// B' = A / (0,5·P), m.
    pub b: f64,
    /// The thickness of the walls around it, m, and the equivalent
    /// thickness dt = w + λ·(Rsi + Rf + Rse), m.
    pub w: f64,
    pub dt: f64,
    /// How deep it is under the ground outside, m (0 for a slab on the
    /// ground).
    pub depth: f64,
    /// U of its layers alone and the equivalent U, W/m²K.
    pub u_layers: f64,
    pub u: f64,
    /// The ψ of the edge insulation, W/mK (0 without).
    pub edge_psi: f64,
}

/// The equivalent U of a floor on the ground by UNE-EN ISO 13370: area,
/// exposed perimeter, wall thickness, resistance of its layers, depth
/// under the ground and edge insulation (only for a slab on the ground).
pub fn slab_on_ground(
    lambda: f64,
    area: f64,
    perimeter: f64,
    w: f64,
    rf: f64,
    depth: f64,
    edge: Option<EdgeInsulation>,
) -> GroundFloor {
    use std::f64::consts::PI;
    let b = if perimeter > 1e-6 {
        area / (0.5 * perimeter)
    } else {
        // No façade around it: a very wide floor.
        1000.0
    };
    let dt = w + lambda * (GROUND_RSI + rf + GROUND_RSE);
    let d = dt + 0.5 * depth;
    let u0 = if d < b {
        2.0 * lambda / (PI * b + d) * (PI * b / d + 1.0).ln()
    } else {
        lambda / (0.457 * b + d)
    };
    let mut edge_psi = 0.0;
    if let Some(e) = edge.filter(|_| depth < 0.05) {
        // The extra thickness of the insulation: its resistance less that
        // of the soil it takes the place of.
        let r = (e.resistance - e.thickness / lambda).max(0.0);
        let dd = r * lambda;
        let s = if e.vertical { 2.0 * e.size } else { e.size };
        edge_psi = -lambda / PI * ((s / dt + 1.0).ln() - (s / (dt + dd) + 1.0).ln());
    }
    let u = (u0 + 2.0 * edge_psi / b).max(0.0);
    GroundFloor {
        area,
        perimeter,
        b,
        w,
        dt,
        depth,
        u_layers: 1.0 / (GROUND_RSI + rf),
        u,
        edge_psi,
    }
}

/// The U of a basement wall `depth` m into the ground (UNE-EN ISO
/// 13370): `dt` is that of the floor, `rw` the resistance of its layers.
pub fn basement_wall(lambda: f64, dt: f64, rw: f64, depth: f64) -> f64 {
    use std::f64::consts::PI;
    let z = depth.max(0.05);
    let dw = lambda * (GROUND_RSI_WALL + rw + GROUND_RSE);
    2.0 * lambda / (PI * z) * (1.0 + 0.5 * dt / (dt + z)) * (z / dw + 1.0).ln()
}

/// The July radiation on a window, kWh/m²·month.
pub fn july_radiation(summer: u8, facing: Facing) -> f64 {
    let z = (summer.clamp(1, 4) - 1) as usize;
    H_SOL_JUL[z][facing_index(facing)]
}

fn facing_index(f: Facing) -> usize {
    match f {
        Facing::North => 0,
        Facing::East => 1,
        Facing::South => 2,
        Facing::West => 3,
        Facing::Up => 4,
    }
}

/// The tangent of the sun's profile angle on a façade in July.
pub fn july_profile(facing: Facing, latitude: f64) -> f64 {
    match facing {
        Facing::South => (90.0 - latitude.abs() + JULY_DECLINATION)
            .clamp(10.0, 89.0)
            .to_radians()
            .tan(),
        Facing::East | Facing::West => PROFILE_EAST_WEST,
        Facing::North => PROFILE_NORTH,
        Facing::Up => 0.0,
    }
}

/// F_sh,obst of a window `height` m high under something `depth` m deep
/// out from the glass (its reveal and what flies above) whose underside is
/// `gap` m over its head; the reveal alone (`reveal`, with no gap) counts
/// too. 1 is no shade.
pub fn shade_factor(
    facing: Facing,
    latitude: f64,
    height: f64,
    reveal: f64,
    depth: f64,
    gap: f64,
) -> f64 {
    if facing == Facing::Up || height <= 0.0 {
        return 1.0;
    }
    let p = july_profile(facing, latitude);
    let shaded = (reveal * p)
        .max(depth * p - gap.max(0.0))
        .clamp(0.0, height);
    1.0 - shaded / height * DIRECT_SHARE[facing_index(facing)]
}

/// The limit of n50 by the compactness.
pub fn n50_limit(compactness: f64) -> f64 {
    let t = ((compactness - 2.0) / 2.0).clamp(0.0, 1.0);
    N50_COMPACT_LOW + (N50_COMPACT_HIGH - N50_COMPACT_LOW) * t
}

/// n50 by anejo H: opaque flow at 100 Pa and area, the windows' (flow,
/// area) and the inside volume.
pub fn n50(opaque_air: f64, opaque_area: f64, windows: &[(f64, f64)], volume: f64) -> f64 {
    if volume <= 0.0 {
        return 0.0;
    }
    let w: f64 = windows.iter().map(|(c, a)| c * a).sum();
    N50_FACTOR * (opaque_air * opaque_area + w) / volume
}

/// The permeability of a window class at 100 Pa.
pub fn q100(class: u8) -> f64 {
    Q100[(class.clamp(1, 4) - 1) as usize]
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A slab of 10 × 10 m on the ground, all of it façade, 30 cm walls,
    /// no insulation: B' = 5 m, dt = 0,30 + 2·0,21 = 0,72 m and
    /// U = 2·2/(π·5 + 0,72)·ln(π·5/0,72 + 1) = 0,762 W/m²K.
    #[test]
    fn a_slab_on_the_ground_by_iso_13370() {
        let g = slab_on_ground(2.0, 100.0, 40.0, 0.3, 0.0, 0.0, None);
        assert!((g.b - 5.0).abs() < 1e-9);
        assert!((g.dt - 0.72).abs() < 1e-9);
        assert!((g.u - 0.7616).abs() < 1e-3, "{}", g.u);
        // Insulated with R = 2: dt = 0,30 + 2·2,21 = 4,72 and B' > dt,
        // so 4/(15,708 + 4,72)·ln(15,708/4,72 + 1) = 0,2869.
        let g = slab_on_ground(2.0, 100.0, 40.0, 0.3, 2.0, 0.0, None);
        assert!((g.u - 0.2869).abs() < 1e-3, "{}", g.u);
        // A wide floor with B' < dt: λ/(0,457·B' + dt).
        let g = slab_on_ground(2.0, 10.0, 40.0, 0.3, 2.0, 0.0, None);
        assert!((g.u - 2.0 / (0.457 * 0.5 + 4.72)).abs() < 1e-9);
        // Edge insulation lowers it.
        let edge = EdgeInsulation {
            vertical: true,
            size: 1.0,
            resistance: 2.0,
            thickness: 0.06,
        };
        let plain = slab_on_ground(2.0, 100.0, 40.0, 0.3, 0.0, 0.0, None);
        let with = slab_on_ground(2.0, 100.0, 40.0, 0.3, 0.0, 0.0, Some(edge));
        assert!(with.edge_psi < 0.0 && with.u < plain.u);
        // ψ = −2/π·[ln(2/0,72 + 1) − ln(2/(0,72 + 3,94) + 1)]
        let d = (2.0 - 0.06 / 2.0) * 2.0;
        let psi = -2.0 / std::f64::consts::PI
            * ((2.0f64 / 0.72 + 1.0).ln() - (2.0f64 / (0.72 + d) + 1.0).ln());
        assert!((with.edge_psi - psi).abs() < 1e-9);
        // A basement floor 3 m down loses less.
        let deep = slab_on_ground(2.0, 100.0, 40.0, 0.3, 0.0, 3.0, None);
        assert!(deep.u < plain.u);
    }

    #[test]
    fn a_basement_wall() {
        // z = 3, dt = 0,72, Rw = 0: dw = 2·0,17 = 0,34;
        // U = 2·2/(3π)·(1 + 0,36/3,72)·ln(3/0,34 + 1) = 1,0636.
        let u = basement_wall(2.0, 0.72, 0.0, 3.0);
        assert!((u - 1.0636).abs() < 1e-3, "{u}");
        assert!(basement_wall(2.0, 0.72, 2.0, 3.0) < u);
    }

    #[test]
    fn shade_and_air() {
        // No overhang, no reveal: no shade.
        assert_eq!(shade_factor(Facing::South, 37.0, 1.2, 0.0, 0.0, 0.3), 1.0);
        // A 1 m balcony 30 cm over a south window at 37°: the profile is
        // tan 74,5° = 3,6, so it shades it all; the direct part is 0,6.
        let f = shade_factor(Facing::South, 37.0, 1.2, 0.0, 1.0, 0.3);
        assert!((f - 0.4).abs() < 1e-9, "{f}");
        // On the east (profile 1,2): 1,2 − 0,3 = 0,9 of 1,2 m shaded.
        let f = shade_factor(Facing::East, 37.0, 1.2, 0.0, 1.0, 0.3);
        assert!((f - (1.0 - 0.75 * 0.7)).abs() < 1e-9, "{f}");
        assert_eq!(q100(3), 9.0);
        assert!((n50_limit(1.0) - 6.0).abs() < 1e-9);
        assert!((n50_limit(3.0) - 4.5).abs() < 1e-9);
        // 300 m² opaque at 16, 20 m² of class 3 windows, 400 m³:
        // 0,629·(4800 + 180)/400 = 7,831.
        let n = n50(16.0, 300.0, &[(9.0, 20.0)], 400.0);
        assert!((n - 7.831).abs() < 1e-3, "{n}");
        assert!((f_rsi(1.0) - 0.75).abs() < 1e-9);
        assert_eq!(f_rsi_min(1, 3), 0.50);
        assert_eq!(f_rsi_min(5, 4), 0.85);
    }
}
