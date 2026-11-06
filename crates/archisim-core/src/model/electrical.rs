//! The data of a dwelling's electrical installation that the drawing does
//! not give (REBT): its degree of electrification, the length of its
//! feeder from the meter to the board («derivación individual») and
//! whether it is a single user (a house with its own meter, no
//! centralised meters). What the calculation needs from the plan (the
//! board, the mechanisms and their circuits) is drawn.

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

/// What the electrical calculation needs besides the drawing.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ElectricalSite {
    pub grade: Electrification,
    /// Length of the feeder from the meter to the board, metres.
    pub feeder_length: f64,
    /// A single user (its own meter by the façade): the feeder may drop
    /// 1,5 % (ITC-BT-15); with centralised meters, 1 %.
    pub single_user: bool,
}

impl Default for ElectricalSite {
    fn default() -> Self {
        ElectricalSite {
            grade: Electrification::Auto,
            feeder_length: 15.0,
            single_user: true,
        }
    }
}

impl ElectricalSite {
    pub fn is_valid(&self) -> bool {
        self.feeder_length.is_finite() && self.feeder_length >= 0.0
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
