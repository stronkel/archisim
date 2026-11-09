//! The data of a dwelling's electrical installation that the drawing does
//! not give (REBT): its degree of electrification, the length of its
//! feeder from the meter to the board («derivación individual») and
//! whether it is a single user (a house with its own meter, no
//! centralised meters). What the calculation needs from the plan (the
//! board, the mechanisms and their circuits) is drawn.
//!
//! Version 98 adds the supply (single or three phase, or by the forecast),
//! the meters centralised in more than one place, the general feeder's
//! length («línea general de alimentación»), the short circuit at the
//! supply (given, or from the line to the transformer), the breaking
//! capacity of the breakers and their curve.

use super::*;

/// The degree of electrification of a dwelling (ITC-BT-10).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Electrification {
    /// Worked out from the dwelling: high with more than 160 m², more
    /// points than the basic circuits take, or a circuit of the high ones.
    #[default]
    Auto,
    /// Basic: 5 750 W at 230 V (25 A).
    Basic,
    /// High: 9 200 W at 230 V (40 A).
    High,
}

impl Electrification {
    pub const ALL: [Electrification; 3] = [
        Electrification::Auto,
        Electrification::Basic,
        Electrification::High,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Electrification::Auto => "Según la vivienda",
            Electrification::Basic => "Básica (5 750 W)",
            Electrification::High => "Elevada (9 200 W)",
        }
    }

    pub fn code(self) -> &'static str {
        match self {
            Electrification::Auto => "auto",
            Electrification::Basic => "basic",
            Electrification::High => "high",
        }
    }

    pub fn from_code(c: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|e| e.code() == c)
    }
}

/// How the installation is supplied.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Supply {
    /// Single phase up to 14 490 W, three phase over it.
    #[default]
    Auto,
    /// 230 V.
    Single,
    /// 3 × 400/230 V.
    Three,
}

impl Supply {
    pub const ALL: [Supply; 3] = [Supply::Auto, Supply::Single, Supply::Three];

    pub fn name(self) -> &'static str {
        match self {
            Supply::Auto => "Según la potencia",
            Supply::Single => "Monofásico 230 V",
            Supply::Three => "Trifásico 3×400/230 V",
        }
    }

    pub fn code(self) -> &'static str {
        match self {
            Supply::Auto => "auto",
            Supply::Single => "single",
            Supply::Three => "three",
        }
    }

    pub fn from_code(c: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|e| e.code() == c)
    }
}

/// The tripping curve of the breakers: the magnetic trip at so many times
/// their rating.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TripCurve {
    B,
    #[default]
    C,
    D,
}

impl TripCurve {
    pub const ALL: [TripCurve; 3] = [TripCurve::B, TripCurve::C, TripCurve::D];

    pub fn name(self) -> &'static str {
        match self {
            TripCurve::B => "B",
            TripCurve::C => "C",
            TripCurve::D => "D",
        }
    }

    pub fn from_code(c: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|e| e.name() == c)
    }
}

/// What the electrical calculation needs besides the drawing.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ElectricalSite {
    pub grade: Electrification,
    /// Length of the feeder from the meter to the board, metres (with the
    /// meters centralised, it is measured on the drawing).
    pub feeder_length: f64,
    /// A single user (its own meter by the façade): the feeder may drop
    /// 1,5 % (ITC-BT-15); with centralised meters, 1 %.
    pub single_user: bool,
    pub supply: Supply,
    /// The meters are centralised in more than one place (the general
    /// feeder may then drop 1 % and each feeder 0,5 %).
    pub partly_centralised: bool,
    /// The general feeder from the main fuse box to the meters, metres.
    pub general_length: f64,
    /// The short circuit current at the supply, kA, given by the company;
    /// 0: worked out from the line to the transformer.
    pub supply_fault: f64,
    /// The length of the company's line from the transformer to the
    /// supply, metres (when `supply_fault` is not given).
    pub network_length: f64,
    /// The breaking capacity of the breakers, kA (4,5, 6, 10…).
    pub breaking_capacity: f64,
    pub curve: TripCurve,
}

impl Default for ElectricalSite {
    fn default() -> Self {
        ElectricalSite {
            grade: Electrification::Auto,
            feeder_length: 15.0,
            single_user: true,
            supply: Supply::Auto,
            partly_centralised: false,
            general_length: 10.0,
            supply_fault: 0.0,
            network_length: 100.0,
            breaking_capacity: 6.0,
            curve: TripCurve::C,
        }
    }
}

impl ElectricalSite {
    pub fn is_valid(&self) -> bool {
        let fine = |v: f64| v.is_finite() && v >= 0.0;
        fine(self.feeder_length)
            && fine(self.general_length)
            && fine(self.supply_fault)
            && fine(self.network_length)
            && self.breaking_capacity.is_finite()
            && self.breaking_capacity > 0.0
    }
}

impl Model {
    pub fn electrical_site(&self) -> &ElectricalSite {
        &self.electrical_site
    }

    pub fn set_electrical_site(&mut self, site: ElectricalSite) -> Result<(), ModelError> {
        if !site.is_valid() {
            return Err(ModelError::NotFinite("electrical site"));
        }
        self.electrical_site = site;
        Ok(())
    }
}
