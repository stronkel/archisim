//! What the building is for, as the Spanish building code (CTE) asks: its
//! main use (DB-SI, which sets occupancies and evacuation), what
//! accessibility it must offer (DB-SUA 9) and whether it has automatic
//! extinction (sprinklers, which lengthen the evacuation routes by 25 %).
//!
//! The checks themselves are in `archisim-cte`.

use crate::model::Model;

/// Main use of the building (CTE, Anejo SI A).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, PartialOrd, Ord, Hash)]
pub enum BuildingUse {
    /// Residencial Vivienda.
    #[default]
    Dwelling,
    /// Residencial Público (hotels, residences…).
    PublicResidential,
    /// Administrativo (offices).
    Administrative,
    /// Docente (schools).
    Teaching,
    /// Hospitalario.
    Hospital,
    /// Comercial (shops).
    Commercial,
    /// Pública Concurrencia (restaurants, theatres, gyms…).
    Assembly,
    /// Aparcamiento.
    Parking,
}

impl BuildingUse {
    pub const ALL: [BuildingUse; 8] = [
        BuildingUse::Dwelling,
        BuildingUse::PublicResidential,
        BuildingUse::Administrative,
        BuildingUse::Teaching,
        BuildingUse::Hospital,
        BuildingUse::Commercial,
        BuildingUse::Assembly,
        BuildingUse::Parking,
    ];

    /// Its code in files.
    pub fn code(self) -> &'static str {
        match self {
            BuildingUse::Dwelling => "dwelling",
            BuildingUse::PublicResidential => "public_residential",
            BuildingUse::Administrative => "administrative",
            BuildingUse::Teaching => "teaching",
            BuildingUse::Hospital => "hospital",
            BuildingUse::Commercial => "commercial",
            BuildingUse::Assembly => "assembly",
            BuildingUse::Parking => "parking",
        }
    }

    pub fn from_code(code: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|u| u.code() == code)
    }
}

/// What accessibility the building must offer (CTE DB-SUA 9).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, PartialOrd, Ord, Hash)]
pub enum Accessibility {
    /// Nothing is checked.
    #[default]
    None,
    /// Accessible routes (itinerario accesible) in the areas of public use,
    /// with accessible toilets.
    Routes,
    /// A dwelling for wheelchair users (vivienda accesible).
    WheelchairDwelling,
}

impl Accessibility {
    pub const ALL: [Accessibility; 3] = [
        Accessibility::None,
        Accessibility::Routes,
        Accessibility::WheelchairDwelling,
    ];

    pub fn code(self) -> &'static str {
        match self {
            Accessibility::None => "none",
            Accessibility::Routes => "routes",
            Accessibility::WheelchairDwelling => "wheelchair_dwelling",
        }
    }

    pub fn from_code(code: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|u| u.code() == code)
    }
}

/// The building's settings for the code checks.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct CodeSettings {
    pub building_use: BuildingUse,
    pub accessibility: Accessibility,
    /// Automatic extinction (sprinklers) throughout.
    pub sprinklers: bool,
}

impl Model {
    pub fn code_settings(&self) -> CodeSettings {
        self.code
    }

    pub fn set_code_settings(&mut self, settings: CodeSettings) {
        self.code = settings;
    }
}
