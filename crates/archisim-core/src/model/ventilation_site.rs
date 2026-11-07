//! The data of the ventilation calculation besides the drawing (DB-HS 3,
//! format version 91): whether the dwellings are ventilated by mechanical
//! extraction or by hybrid ventilation, and the thermal zone of the
//! table 4.4, which gives the draught of the hybrid ducts.

use super::{Model, ModelError};

/// How the dwellings are ventilated (HS 3, 3.1.1: hybrid or mechanical).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum VentSystem {
    /// A fan at the head of every extract duct; ducts by 4.2.2.
    #[default]
    Mechanical,
    /// The ducts draw by themselves and a hybrid aspirator helps when they
    /// do not; ducts by tables 4.2 and 4.3, outlets by 3.2.1.5.
    Hybrid,
}

impl VentSystem {
    pub const ALL: [VentSystem; 2] = [VentSystem::Mechanical, VentSystem::Hybrid];

    pub fn code(self) -> &'static str {
        match self {
            VentSystem::Mechanical => "mechanical",
            VentSystem::Hybrid => "hybrid",
        }
    }

    pub fn from_code(s: &str) -> Option<VentSystem> {
        VentSystem::ALL.into_iter().find(|v| v.code() == s)
    }

    pub fn name(self) -> &'static str {
        match self {
            VentSystem::Mechanical => "Mecánica",
            VentSystem::Hybrid => "Híbrida",
        }
    }
}

/// The thermal zone of the HS 3, table 4.4 (not the one of the HE).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default)]
pub enum DraftZone {
    W,
    X,
    Y,
    /// Huelva below 800 m, as the thermal data's default.
    #[default]
    Z,
}

impl DraftZone {
    pub const ALL: [DraftZone; 4] = [DraftZone::W, DraftZone::X, DraftZone::Y, DraftZone::Z];

    pub fn code(self) -> &'static str {
        match self {
            DraftZone::W => "W",
            DraftZone::X => "X",
            DraftZone::Y => "Y",
            DraftZone::Z => "Z",
        }
    }

    pub fn from_code(s: &str) -> Option<DraftZone> {
        DraftZone::ALL.into_iter().find(|z| z.code() == s)
    }
}

/// Table 4.4: the zone of each province, at 800 m or below and above
/// (Ceuta and Melilla have no height above).
pub const DRAFT_ZONES: [(&str, DraftZone, Option<DraftZone>); 52] = {
    use DraftZone::*;
    [
        ("Álava", W, Some(W)),
        ("Albacete", X, Some(W)),
        ("Alicante", Z, Some(Y)),
        ("Almería", Z, Some(Y)),
        ("Asturias", X, Some(W)),
        ("Ávila", W, Some(W)),
        ("Badajoz", Z, Some(Y)),
        ("Baleares", Z, Some(Y)),
        ("Barcelona", Z, Some(Y)),
        ("Burgos", W, Some(W)),
        ("Cáceres", Z, Some(Y)),
        ("Cádiz", Z, Some(Y)),
        ("Cantabria", X, Some(W)),
        ("Castellón", Z, Some(Y)),
        ("Ceuta", Z, None),
        ("Ciudad Real", Y, Some(X)),
        ("Córdoba", Z, Some(Y)),
        ("A Coruña", X, Some(W)),
        ("Cuenca", W, Some(W)),
        ("Girona", Y, Some(X)),
        ("Granada", Y, Some(X)),
        ("Guadalajara", X, Some(W)),
        ("Guipúzcoa", X, Some(W)),
        ("Huelva", Z, Some(Y)),
        ("Huesca", X, Some(W)),
        ("Jaén", Z, Some(Y)),
        ("Las Palmas", Z, Some(Y)),
        ("León", W, Some(W)),
        ("Lleida", Y, Some(X)),
        ("Lugo", W, Some(W)),
        ("Madrid", X, Some(W)),
        ("Málaga", Z, Some(Y)),
        ("Melilla", Z, None),
        ("Murcia", Z, Some(Y)),
        ("Navarra", X, Some(W)),
        ("Ourense", X, Some(W)),
        ("Palencia", W, Some(W)),
        ("Pontevedra", Y, Some(X)),
        ("La Rioja", Z, Some(Y)),
        ("Salamanca", Y, Some(X)),
        ("Santa Cruz de Tenerife", X, Some(W)),
        ("Segovia", W, Some(W)),
        ("Sevilla", Z, Some(Y)),
        ("Soria", W, Some(W)),
        ("Tarragona", Y, Some(X)),
        ("Teruel", W, Some(W)),
        ("Toledo", Y, Some(X)),
        ("Valencia", Z, Some(Y)),
        ("Valladolid", W, Some(W)),
        ("Vizcaya", X, Some(W)),
        ("Zamora", X, Some(W)),
        ("Zaragoza", Y, Some(X)),
    ]
};

/// The zone of a province (by its name in [`DRAFT_ZONES`]), above or below
/// 800 m.
pub fn draft_zone_of(province: &str, above_800: bool) -> Option<DraftZone> {
    DRAFT_ZONES
        .iter()
        .find(|(p, _, _)| p.eq_ignore_ascii_case(province))
        .and_then(|(_, low, high)| if above_800 { *high } else { Some(*low) })
}

/// The ventilation's data.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct VentilationSite {
    pub system: VentSystem,
    pub zone: DraftZone,
}

impl Model {
    pub fn ventilation_site(&self) -> &VentilationSite {
        &self.ventilation_site
    }

    pub fn set_ventilation_site(&mut self, site: VentilationSite) -> Result<(), ModelError> {
        self.ventilation_site = site;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_zones() {
        assert_eq!(draft_zone_of("Madrid", false), Some(DraftZone::X));
        assert_eq!(draft_zone_of("madrid", true), Some(DraftZone::W));
        assert_eq!(draft_zone_of("Ceuta", true), None);
        assert_eq!(draft_zone_of("Huelva", false), Some(DraftZone::default()));
        assert_eq!(VentSystem::from_code("hybrid"), Some(VentSystem::Hybrid));
    }
}
