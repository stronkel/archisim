//! The actions of CTE DB-SE-AE: live loads by use (table 3.1), the
//! combination coefficients of DB-SE (table 4.2), the snow (3.5 and annex
//! E) and the wind (3.3 and annex D).

use archisim_core::{BuildingUse, Model, Room, StructuralSite};

/// The categories of use of table 3.1 of DB-SE-AE.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum UseCategory {
    /// Dwellings, hotel and hospital rooms.
    A1,
    /// Storerooms.
    A2,
    /// Offices.
    B,
    /// With tables and chairs.
    C1,
    /// With fixed seats.
    C2,
    /// Without obstacles to people moving (halls, corridors of public
    /// buildings).
    C3,
    /// Gyms.
    C4,
    /// Crowds (concert halls, stands).
    C5,
    /// Shops.
    D1,
    /// Supermarkets and large stores.
    D2,
    /// Traffic and parking of light vehicles.
    E,
    /// Roofs walked on, reached only privately.
    F,
    /// Roofs reached only for upkeep, pitch under 20°.
    G1,
    /// Roofs reached only for upkeep, pitch over 40°.
    G2,
}

impl UseCategory {
    pub const ALL: [UseCategory; 14] = [
        UseCategory::A1,
        UseCategory::A2,
        UseCategory::B,
        UseCategory::C1,
        UseCategory::C2,
        UseCategory::C3,
        UseCategory::C4,
        UseCategory::C5,
        UseCategory::D1,
        UseCategory::D2,
        UseCategory::E,
        UseCategory::F,
        UseCategory::G1,
        UseCategory::G2,
    ];

    /// Uniform load, kN/m².
    pub fn uniform(self) -> f64 {
        use UseCategory::*;
        match self {
            A1 | B | E => 2.0,
            A2 => 3.0,
            C1 => 3.0,
            C2 => 4.0,
            C3 | C4 | C5 | D1 | D2 => 5.0,
            F | G1 => 1.0,
            G2 => 0.0,
        }
    }

    /// Concentrated load, kN (checked on its own, not added to the
    /// uniform one).
    pub fn concentrated(self) -> f64 {
        use UseCategory::*;
        match self {
            A1 | A2 | B | F | G1 | G2 => 2.0,
            C1 | C2 | C3 | C5 | D1 => 4.0,
            C4 | D2 => 7.0,
            E => 20.0,
        }
    }

    /// The combination coefficients ψ0, ψ1, ψ2 (DB-SE, table 4.2).
    pub fn psi(self) -> [f64; 3] {
        use UseCategory::*;
        match self {
            A1 | A2 | B | F => [0.7, 0.5, 0.3],
            C1 | C2 | C3 | C4 | C5 | D1 | D2 | E => [0.7, 0.7, 0.6],
            G1 | G2 => [0.0, 0.0, 0.0],
        }
    }

    pub fn code(self) -> &'static str {
        use UseCategory::*;
        match self {
            A1 => "A1",
            A2 => "A2",
            B => "B",
            C1 => "C1",
            C2 => "C2",
            C3 => "C3",
            C4 => "C4",
            C5 => "C5",
            D1 => "D1",
            D2 => "D2",
            E => "E",
            F => "F",
            G1 => "G1",
            G2 => "G2",
        }
    }

    pub fn name(self) -> &'static str {
        use UseCategory::*;
        match self {
            A1 => "Viviendas y habitaciones",
            A2 => "Trasteros",
            B => "Zonas administrativas",
            C1 => "Zonas con mesas y sillas",
            C2 => "Zonas con asientos fijos",
            C3 => "Zonas sin obstáculos",
            C4 => "Gimnasios",
            C5 => "Zonas de aglomeración",
            D1 => "Locales comerciales",
            D2 => "Supermercados y grandes superficies",
            E => "Tráfico y aparcamiento de vehículos ligeros",
            F => "Cubiertas transitables de acceso privado",
            G1 => "Cubiertas solo para conservación",
            G2 => "Cubiertas solo para conservación (más de 40°)",
        }
    }
}

/// The category of a building use, for the rooms that say nothing else.
pub fn category_of_use(u: BuildingUse) -> UseCategory {
    match u {
        BuildingUse::Dwelling | BuildingUse::PublicResidential | BuildingUse::Hospital => {
            UseCategory::A1
        }
        BuildingUse::Administrative => UseCategory::B,
        BuildingUse::Teaching => UseCategory::C1,
        BuildingUse::Commercial => UseCategory::D1,
        BuildingUse::Assembly => UseCategory::C3,
        BuildingUse::Parking => UseCategory::E,
    }
}

fn plain(s: &str) -> String {
    s.to_lowercase()
        .chars()
        .map(|c| match c {
            'á' | 'à' | 'ä' => 'a',
            'é' | 'è' | 'ë' => 'e',
            'í' | 'ì' | 'ï' => 'i',
            'ó' | 'ò' | 'ö' => 'o',
            'ú' | 'ù' | 'ü' => 'u',
            c => c,
        })
        .collect()
}

/// The category of a room, from its use or its name, in a building of
/// `building`'s use.
pub fn room_category(r: &Room, building: BuildingUse) -> UseCategory {
    let text = plain(if r.usage.trim().is_empty() {
        &r.name
    } else {
        &r.usage
    });
    let has = |w: &[&str]| w.iter().any(|x| text.contains(x));
    if has(&["trastero", "almacen", "archivo", "despensa"]) {
        UseCategory::A2
    } else if has(&["garaje", "aparcamiento", "parking", "cochera"]) {
        UseCategory::E
    } else if has(&["gimnasio"]) {
        UseCategory::C4
    } else if has(&["supermercado", "hipermercado"]) {
        UseCategory::D2
    } else if has(&["tienda", "comercio", "local comercial"]) {
        UseCategory::D1
    } else if has(&["oficina", "despacho"]) {
        UseCategory::B
    } else if has(&["aula", "comedor", "restaurante", "cafeteria", "bar"]) {
        UseCategory::C1
    } else if has(&["auditorio", "salon de actos", "graderio", "grada"]) {
        UseCategory::C5
    } else if building != BuildingUse::Dwelling
        && has(&["vestibulo", "hall", "pasillo", "distribuidor", "portal"])
    {
        UseCategory::C3
    } else {
        category_of_use(building)
    }
}

/// The ψ of the snow (DB-SE, table 4.2): over 1 000 m it stays longer.
pub fn snow_psi(altitude: f64) -> [f64; 3] {
    if altitude > 1000.0 {
        [0.7, 0.5, 0.2]
    } else {
        [0.5, 0.2, 0.0]
    }
}

/// The ψ of the wind.
pub const WIND_PSI: [f64; 3] = [0.6, 0.5, 0.0];

/// Table E.2: snow on level ground (kN/m²) by altitude, for zones 1 to 7.
const SNOW_TABLE: [(f64, [f64; 7]); 14] = [
    (0.0, [0.3, 0.4, 0.2, 0.2, 0.2, 0.2, 0.2]),
    (200.0, [0.5, 0.5, 0.2, 0.2, 0.3, 0.2, 0.2]),
    (400.0, [0.6, 0.6, 0.2, 0.3, 0.4, 0.2, 0.2]),
    (500.0, [0.7, 0.7, 0.3, 0.4, 0.4, 0.3, 0.2]),
    (600.0, [0.9, 0.9, 0.3, 0.5, 0.5, 0.4, 0.2]),
    (700.0, [1.0, 1.0, 0.4, 0.6, 0.6, 0.5, 0.2]),
    (800.0, [1.2, 1.1, 0.5, 0.8, 0.7, 0.7, 0.2]),
    (900.0, [1.4, 1.3, 0.6, 1.0, 0.8, 0.9, 0.2]),
    (1000.0, [1.7, 1.5, 0.7, 1.2, 0.9, 1.2, 0.2]),
    (1200.0, [2.3, 2.0, 1.1, 1.9, 1.3, 2.0, 0.2]),
    (1400.0, [3.2, 2.6, 1.7, 3.0, 1.8, 3.3, 0.2]),
    (1600.0, [4.3, 3.5, 2.6, 4.6, 2.5, 5.5, 0.2]),
    // Beyond the table some zones have no value: the last one is kept
    // (above, the code sends to the municipal ordinance).
    (1800.0, [4.3, 4.6, 4.0, 4.6, 2.5, 9.3, 0.2]),
    (2200.0, [4.3, 8.0, 4.0, 4.6, 2.5, 9.3, 0.2]),
];

/// The snow on level ground sk (kN/m²) for a winter zone (1–7) and an
/// altitude, interpolating table E.2 (the code allows reading it by
/// altitude; between rows, the straight line is on the safe side of the
/// lower one).
pub fn ground_snow(zone: u8, altitude: f64) -> f64 {
    let z = (zone.clamp(1, 7) - 1) as usize;
    let a = altitude.max(0.0);
    let rows = &SNOW_TABLE;
    if a >= rows[rows.len() - 1].0 {
        return rows[rows.len() - 1].1[z];
    }
    for w in rows.windows(2) {
        let (a0, v0) = (w[0].0, w[0].1[z]);
        let (a1, v1) = (w[1].0, w[1].1[z]);
        if a <= a1 {
            let t = (a - a0) / (a1 - a0);
            return v0 + (v1 - v0) * t;
        }
    }
    rows[0].1[z]
}

/// The shape coefficient μ of a roof of `pitch` (radians) with nothing to
/// stop the snow sliding off (3.5.3): 1 up to 30°, 0 from 60°.
pub fn snow_shape(pitch: f64) -> f64 {
    let deg = pitch.to_degrees().abs();
    if deg <= 30.0 {
        1.0
    } else if deg >= 60.0 {
        0.0
    } else {
        (60.0 - deg) / 30.0
    }
}

/// The snow on a roof per m² of plan: μ · sk.
pub fn roof_snow(site: &StructuralSite, altitude: f64, pitch: f64) -> f64 {
    snow_shape(pitch) * ground_snow(site.snow_zone, altitude)
}

/// k, L, Z of table D.2 for roughness 1 (I) to 5 (V).
pub fn roughness_parameters(r: u8) -> (f64, f64, f64) {
    match r.clamp(1, 5) {
        1 => (0.156, 0.003, 1.0),
        2 => (0.17, 0.01, 1.0),
        3 => (0.19, 0.05, 2.0),
        4 => (0.22, 0.3, 5.0),
        _ => (0.24, 1.0, 10.0),
    }
}

pub const ROUGHNESS_NAMES: [&str; 5] = [
    "I · Borde del mar o de un lago",
    "II · Terreno rural llano sin obstáculos",
    "III · Zona rural accidentada o con obstáculos aislados",
    "IV · Zona urbana, industrial o forestal",
    "V · Centro de ciudad con edificios en altura",
];

/// The exposure coefficient ce at `z` metres above the ground (D.2).
pub fn exposure(roughness: u8, z: f64) -> f64 {
    let (k, l, zmin) = roughness_parameters(roughness);
    let f = k * (z.max(zmin) / l).ln();
    f * (f + 7.0 * k)
}

/// The pressure coefficients of a building's facades for the global
/// action (table D.3, areas of 10 m² or more): windward (D, pressure) and
/// leeward (E, suction), by its slenderness h/d in the wind's direction.
pub fn facade_coefficients(h_over_d: f64) -> (f64, f64) {
    let r = h_over_d.max(0.0);
    let windward = if r <= 0.25 {
        0.7
    } else if r >= 1.0 {
        0.8
    } else {
        0.7 + 0.1 * (r - 0.25) / 0.75
    };
    let leeward = if r <= 0.25 {
        -0.3
    } else if r <= 1.0 {
        -0.3 - 0.2 * (r - 0.25) / 0.75
    } else if r >= 5.0 {
        -0.7
    } else {
        -0.5 - 0.2 * (r - 1.0) / 4.0
    };
    (windward, leeward)
}

/// The wind pressure qe = qb · ce at height `z` (kN/m², before the
/// pressure coefficient).
pub fn wind_pressure(site: &StructuralSite, z: f64) -> f64 {
    site.wind_zone.basic_pressure() * exposure(site.roughness, z)
}

/// What the site gives, written out for the report.
#[derive(Debug, Clone, PartialEq)]
pub struct SiteActions {
    pub altitude: f64,
    pub ground_snow: f64,
    pub basic_wind: f64,
    pub snow_psi: [f64; 3],
}

pub fn site_actions(model: &Model) -> SiteActions {
    let site = model.structural_site();
    let altitude = model.site_altitude();
    SiteActions {
        altitude,
        ground_snow: ground_snow(site.snow_zone, altitude),
        basic_wind: site.wind_zone.basic_pressure(),
        snow_psi: snow_psi(altitude),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn snow_reads_the_table() {
        // Huelva: zone 6, at the sea.
        assert!((ground_snow(6, 0.0) - 0.2).abs() < 1e-9);
        // Halfway between 600 and 700 m in zone 1.
        assert!((ground_snow(1, 650.0) - 0.95).abs() < 1e-9);
        assert!((ground_snow(2, 2200.0) - 8.0).abs() < 1e-9);
        assert_eq!(snow_shape(20f64.to_radians()), 1.0);
        assert!((snow_shape(45f64.to_radians()) - 0.5).abs() < 1e-9);
        assert_eq!(snow_shape(70f64.to_radians()), 0.0);
    }

    #[test]
    fn wind_follows_annex_d() {
        // Roughness II at 3 m: F = 0,17 ln(3/0,01) = 0,9696, ce = F (F + 1,19).
        let ce = exposure(2, 3.0);
        assert!((ce - 0.9696 * (0.9696 + 1.19)).abs() < 1e-3, "{ce}");
        // Below Z the value of Z is used.
        assert_eq!(exposure(5, 3.0), exposure(5, 10.0));
        let (d, e) = facade_coefficients(1.0);
        assert!((d - 0.8).abs() < 1e-9 && (e + 0.5).abs() < 1e-9);
        let (d, e) = facade_coefficients(0.1);
        assert!((d - 0.7).abs() < 1e-9 && (e + 0.3).abs() < 1e-9);
    }

    #[test]
    fn uses_have_their_loads() {
        assert_eq!(UseCategory::A1.uniform(), 2.0);
        assert_eq!(UseCategory::E.concentrated(), 20.0);
        assert_eq!(UseCategory::C3.psi(), [0.7, 0.7, 0.6]);
        assert_eq!(UseCategory::G1.psi(), [0.0; 3]);
    }
}
