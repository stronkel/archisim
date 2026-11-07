//! What the thermal calculations need besides the drawing (DB-HE 1 and
//! the heating and cooling loads): the climate zone of the place, the
//! design temperatures, the thermal conductivity of the materials and
//! the transmittance of the windows and doors.
//!
//! The envelope itself (walls, roofs, floors and openings, and which of
//! their faces are outside) comes from the drawing.

use super::*;

/// The winter letter of a climate zone (CTE DB-HE, anejo B).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, PartialOrd, Ord)]
pub enum WinterZone {
    Alpha,
    #[default]
    A,
    B,
    C,
    D,
    E,
}

impl WinterZone {
    pub const ALL: [WinterZone; 6] = [
        WinterZone::Alpha,
        WinterZone::A,
        WinterZone::B,
        WinterZone::C,
        WinterZone::D,
        WinterZone::E,
    ];

    pub fn letter(self) -> &'static str {
        match self {
            WinterZone::Alpha => "α",
            WinterZone::A => "A",
            WinterZone::B => "B",
            WinterZone::C => "C",
            WinterZone::D => "D",
            WinterZone::E => "E",
        }
    }

    pub fn code(self) -> &'static str {
        match self {
            WinterZone::Alpha => "alpha",
            WinterZone::A => "a",
            WinterZone::B => "b",
            WinterZone::C => "c",
            WinterZone::D => "d",
            WinterZone::E => "e",
        }
    }

    pub fn from_code(c: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|z| z.code() == c)
    }

    /// Its column in the tables of DB-HE 1 (α = 0 … E = 5).
    pub fn index(self) -> usize {
        self as usize
    }

    /// A usual outdoor temperature for the heating loads, °C (to be set
    /// from the place's data).
    pub fn usual_winter_air(self) -> f64 {
        match self {
            WinterZone::Alpha => 8.0,
            WinterZone::A => 4.0,
            WinterZone::B => 2.0,
            WinterZone::C => 0.0,
            WinterZone::D => -3.0,
            WinterZone::E => -5.0,
        }
    }
}

/// A usual outdoor temperature for the cooling loads by the summer
/// number of the zone (1 to 4), °C.
pub fn usual_summer_air(summer: u8) -> f64 {
    match summer {
        1 => 30.0,
        2 => 32.0,
        3 => 34.0,
        _ => 37.0,
    }
}

/// What the thermal calculations need besides the drawing.
#[derive(Debug, Clone, PartialEq)]
pub struct ThermalSite {
    pub winter: WinterZone,
    /// 1 to 4.
    pub summer: u8,
    /// Design temperatures, °C: outside in winter and summer, inside
    /// heated and cooled.
    pub winter_air: f64,
    pub summer_air: f64,
    pub heated: f64,
    pub cooled: f64,
    /// Transmittance of the windows (frame and glass together) and of the
    /// outside doors, W/m²K.
    pub window_u: f64,
    pub door_u: f64,
    /// Solar factor of the glass (with its shading).
    pub glass_g: f64,
    /// Thermal conductivity of a material, W/mK, where the usual one of
    /// its name or kind is not right.
    pub conductivity: BTreeMap<MaterialId, f64>,
}

impl Default for ThermalSite {
    /// Huelva: zone A4.
    fn default() -> Self {
        ThermalSite {
            winter: WinterZone::A,
            summer: 4,
            winter_air: WinterZone::A.usual_winter_air(),
            summer_air: usual_summer_air(4),
            heated: 21.0,
            cooled: 25.0,
            window_u: 2.7,
            door_u: 2.2,
            glass_g: 0.6,
            conductivity: BTreeMap::new(),
        }
    }
}

impl ThermalSite {
    /// «A4».
    pub fn zone(&self) -> String {
        format!("{}{}", self.winter.letter(), self.summer)
    }

    pub fn is_valid(&self) -> bool {
        let fin = [
            self.winter_air,
            self.summer_air,
            self.heated,
            self.cooled,
            self.window_u,
            self.door_u,
            self.glass_g,
        ]
        .iter()
        .all(|v| v.is_finite());
        fin && (1..=4).contains(&self.summer)
            && self.window_u > 0.0
            && self.door_u > 0.0
            && (0.0..=1.0).contains(&self.glass_g)
            && self
                .conductivity
                .values()
                .all(|v| v.is_finite() && *v > 0.0)
    }
}

/// The usual thermal conductivity of a material, W/mK, by its name and
/// its kind (values of the CTE's catalogue of construction elements, to
/// be checked against the product's data).
pub fn usual_conductivity(name: &str, category: &str) -> f64 {
    let n = name.to_lowercase();
    let has = |w: &[&str]| w.iter().any(|x| n.contains(x));
    if has(&["poliuretano", "pir", "pur "]) {
        0.028
    } else if has(&["extruido", "xps"]) {
        0.034
    } else if has(&["expandido", "eps"]) {
        0.037
    } else if has(&["lana", "fibra de vidrio"]) {
        0.040
    } else if has(&["corcho"]) {
        0.045
    } else if has(&["celular"]) {
        0.18
    } else if has(&["hormigón armado", "hormigon armado"]) {
        2.5
    } else if has(&["hormigón", "hormigon"]) {
        2.0
    } else if has(&["perforado", "macizo"]) {
        0.50
    } else if has(&["hueco"]) {
        0.43
    } else if has(&["placa de yeso", "cartón yeso", "carton yeso", "pladur"]) {
        0.25
    } else if has(&["yeso"]) {
        0.57
    } else if has(&["mortero", "monocapa", "revoco"]) {
        1.3
    } else if has(&["teja"]) {
        1.0
    } else if has(&["baldosa", "gres", "cerámic", "ceramic"]) {
        1.3
    } else if has(&["granito", "mármol", "marmol", "piedra", "caliza"]) {
        2.8
    } else if has(&["grava"]) {
        2.0
    } else if has(&["madera", "tablero"]) {
        0.15
    } else if has(&["aluminio"]) {
        160.0
    } else if has(&["acero", "hierro"]) {
        50.0
    } else if has(&["vidrio"]) {
        1.0
    } else if has(&["lámina", "lamina", "betún", "betun", "impermeab"]) {
        0.23
    } else if has(&["asfált", "asfalt"]) {
        0.70
    } else {
        match category {
            "concrete" => 2.0,
            "brick" => 0.5,
            "insulation" => 0.04,
            "plaster" => 0.57,
            "ceramic" | "mortar" => 1.3,
            "stone" => 2.8,
            "gravel" => 2.0,
            "wood" => 0.15,
            "aluminium" => 160.0,
            "steel" | "metal" => 50.0,
            "glass" => 1.0,
            "membrane" => 0.23,
            "asphalt" => 0.70,
            _ => 1.0,
        }
    }
}

/// Thermal resistance of an air gap that is not ventilated, m²K/W.
pub const AIR_GAP_R: f64 = 0.17;

impl Model {
    pub fn thermal_site(&self) -> &ThermalSite {
        &self.thermal_site
    }

    pub fn set_thermal_site(&mut self, site: ThermalSite) -> Result<(), ModelError> {
        if !site.is_valid() {
            return Err(ModelError::NotFinite("thermal site"));
        }
        self.thermal_site = site;
        Ok(())
    }

    /// The thermal conductivity of a material, W/mK: the one set for the
    /// project, or the usual one.
    pub fn conductivity(&self, m: MaterialId) -> f64 {
        if let Some(v) = self.thermal_site.conductivity.get(&m) {
            return *v;
        }
        self.material(m)
            .map_or(1.0, |x| usual_conductivity(&x.name, &x.category))
    }

    /// The thermal resistance of some layers, m²K/W (an air gap, 0,17).
    pub fn layers_resistance(&self, layers: &[Layer]) -> f64 {
        layers
            .iter()
            .map(|l| {
                if l.function == LayerFunction::AirGap {
                    AIR_GAP_R
                } else {
                    l.thickness / self.conductivity(l.material)
                }
            })
            .sum()
    }
}
