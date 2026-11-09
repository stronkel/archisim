//! The data of a building's water supply that the drawing does not give
//! (DB-HS 4): the pressure the network leaves at the meter and the pipe
//! the installation is made of. What the calculation needs from the plan
//! (the meter, the heater, the pipes, the risers and the appliances) is
//! drawn.

use super::*;

/// What the pipes are made of: their usual sizes, their roughness and the
/// fastest water the code allows in them (HS 4, 4.2.1).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PipeMaterial {
    /// Multilayer (PE-X/Al/PE-X): the usual pipe of a dwelling.
    #[default]
    Multilayer,
    /// Cross-linked polyethylene (PE-X), PN 10.
    Pex,
    /// Polypropylene (PP-R), PN 20, welded.
    Ppr,
    /// Copper, hard drawn.
    Copper,
}

impl PipeMaterial {
    pub const ALL: [PipeMaterial; 4] = [
        PipeMaterial::Multilayer,
        PipeMaterial::Pex,
        PipeMaterial::Ppr,
        PipeMaterial::Copper,
    ];

    pub fn name(self) -> &'static str {
        match self {
            PipeMaterial::Multilayer => "Multicapa PE-X/Al/PE-X",
            PipeMaterial::Pex => "Polietileno reticulado (PE-X)",
            PipeMaterial::Ppr => "Polipropileno (PP-R) PN 20",
            PipeMaterial::Copper => "Cobre",
        }
    }

    pub fn code(self) -> &'static str {
        match self {
            PipeMaterial::Multilayer => "multilayer",
            PipeMaterial::Pex => "pex",
            PipeMaterial::Ppr => "ppr",
            PipeMaterial::Copper => "copper",
        }
    }

    pub fn from_code(c: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|m| m.code() == c)
    }

    /// Whether it is a metal (HS 4, 4.2.1: 0,50 to 2,00 m/s; plastics,
    /// 0,50 to 3,50 m/s).
    pub fn is_metal(self) -> bool {
        self == PipeMaterial::Copper
    }

    /// The usual sizes: outside and inside diameter, metres.
    pub fn sizes(self) -> &'static [(f64, f64)] {
        match self {
            PipeMaterial::Multilayer => &[
                (0.016, 0.012),
                (0.020, 0.016),
                (0.026, 0.020),
                (0.032, 0.026),
                (0.040, 0.033),
                (0.050, 0.042),
                (0.063, 0.054),
                (0.075, 0.065),
            ],
            PipeMaterial::Pex => &[
                (0.016, 0.0124),
                (0.020, 0.0162),
                (0.025, 0.0204),
                (0.032, 0.0262),
                (0.040, 0.0326),
                (0.050, 0.0408),
                (0.063, 0.0514),
            ],
            PipeMaterial::Ppr => &[
                (0.020, 0.0132),
                (0.025, 0.0166),
                (0.032, 0.0212),
                (0.040, 0.0266),
                (0.050, 0.0332),
                (0.063, 0.0420),
                (0.075, 0.0500),
            ],
            PipeMaterial::Copper => &[
                (0.012, 0.010),
                (0.015, 0.013),
                (0.018, 0.016),
                (0.022, 0.020),
                (0.028, 0.026),
                (0.035, 0.033),
                (0.042, 0.040),
                (0.054, 0.051),
            ],
        }
    }

    /// Absolute roughness of its inside, metres.
    pub fn roughness(self) -> f64 {
        match self {
            PipeMaterial::Copper => 0.0015e-3,
            _ => 0.007e-3,
        }
    }

    /// The inside diameter of a pipe of this outside diameter: the usual
    /// size nearest, or 80 % of it.
    pub fn inside_of(self, outside: f64) -> f64 {
        self.sizes()
            .iter()
            .min_by(|a, b| (a.0 - outside).abs().total_cmp(&(b.0 - outside).abs()))
            .filter(|s| (s.0 - outside).abs() < 0.0015)
            .map_or(outside * 0.8, |s| s.1)
    }
}

/// Whether the roofs the downpipes go through are walked on (DB-HS 5
/// 3.3.3.1: the primary ventilation 2,00 m over a walked roof, 1,30 m over
/// one that is not).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum RoofAccess {
    /// By the drawing: a flat roof with rooms or a railing on its level is
    /// walked on; a pitched one is not.
    #[default]
    Auto,
    Walked,
    NotWalked,
}

impl RoofAccess {
    pub const ALL: [RoofAccess; 3] = [RoofAccess::Auto, RoofAccess::Walked, RoofAccess::NotWalked];

    pub fn code(self) -> &'static str {
        match self {
            RoofAccess::Auto => "auto",
            RoofAccess::Walked => "walked",
            RoofAccess::NotWalked => "not_walked",
        }
    }

    pub fn from_code(c: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|k| k.code() == c)
    }

    pub fn name(self) -> &'static str {
        match self {
            RoofAccess::Auto => "Según el dibujo",
            RoofAccess::Walked => "Transitables",
            RoofAccess::NotWalked => "No transitables",
        }
    }
}

/// What the water calculations (supply and drainage) need besides the
/// drawing.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WaterSite {
    /// The pressure the supply company guarantees at the meter, kPa
    /// (100 kPa ≈ 1 bar ≈ 10 m of water).
    pub pressure: f64,
    pub material: PipeMaterial,
    /// The rainfall intensity of the place, mm/h (DB-HS 5, anejo B, tabla
    /// B.1, by its zone and isohyet); the tables of rain water are for
    /// 100 mm/h and are scaled by i/100.
    pub rain_intensity: f64,
    /// What the meter, its filter and valves take from the pressure, kPa
    /// (the maker's figure; 0: not counted).
    pub meter_loss: f64,
    /// What the heater takes from the pressure of the hot water, kPa.
    pub heater_loss: f64,
    /// The water seal of the appliances' traps and trap boxes, metres
    /// (DB-HS 5 3.3.1.1: 50 to 100 mm).
    pub trap_seal: f64,
    /// Whether the roofs over the downpipes are walked on.
    pub roof_access: RoofAccess,
}

impl Default for WaterSite {
    fn default() -> Self {
        WaterSite {
            pressure: 300.0,
            material: PipeMaterial::Multilayer,
            rain_intensity: 100.0,
            meter_loss: 0.0,
            heater_loss: 0.0,
            trap_seal: 0.05,
            roof_access: RoofAccess::Auto,
        }
    }
}

impl WaterSite {
    pub fn is_valid(&self) -> bool {
        self.pressure.is_finite()
            && self.pressure > 0.0
            && self.rain_intensity.is_finite()
            && self.rain_intensity > 0.0
            && (0.0..self.pressure).contains(&self.meter_loss)
            && (0.0..1000.0).contains(&self.heater_loss)
            && self.trap_seal.is_finite()
            && (0.0..0.5).contains(&self.trap_seal)
    }
}

impl Model {
    pub fn water_site(&self) -> &WaterSite {
        &self.water_site
    }

    pub fn set_water_site(&mut self, site: WaterSite) -> Result<(), ModelError> {
        if !site.is_valid() {
            return Err(ModelError::NotFinite("water site"));
        }
        self.water_site = site;
        Ok(())
    }
}
