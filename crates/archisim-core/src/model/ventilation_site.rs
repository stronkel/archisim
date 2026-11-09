//! The data of the ventilation calculation besides the drawing (DB-HS 3,
//! format version 91): whether the dwellings are ventilated by mechanical
//! extraction or by hybrid ventilation, and the thermal zone of the
//! table 4.4, which gives the draught of the hybrid ducts. Format version
//! 99 adds how the garages are ventilated (natural or mechanical, HS 3
//! 3.1.4), and, for the rooms that are not of dwellings (RITE IT 1.1.4.2),
//! the outdoor air's class (ODA), the air speed of the ducts and the indoor
//! air class (IDA) set by hand on a room.

use std::collections::BTreeMap;

use super::{Model, ModelError, RoomId};

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

/// How the garages are ventilated (HS 3, 3.1.4).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum GarageVent {
    /// By the drawing: mechanical if extract valves are drawn in it,
    /// natural if only openings to the outside.
    #[default]
    Auto,
    /// Mixed openings on the façades (3.1.4.1).
    Natural,
    /// Extraction by fans, 120 l/s a space (3.1.4.2).
    Mechanical,
}

impl GarageVent {
    pub const ALL: [GarageVent; 3] = [
        GarageVent::Auto,
        GarageVent::Natural,
        GarageVent::Mechanical,
    ];

    pub fn code(self) -> &'static str {
        match self {
            GarageVent::Auto => "auto",
            GarageVent::Natural => "natural",
            GarageVent::Mechanical => "mechanical",
        }
    }

    pub fn from_code(s: &str) -> Option<GarageVent> {
        GarageVent::ALL.into_iter().find(|v| v.code() == s)
    }

    pub fn name(self) -> &'static str {
        match self {
            GarageVent::Auto => "Según el dibujo",
            GarageVent::Natural => "Natural",
            GarageVent::Mechanical => "Mecánica",
        }
    }
}

/// The outdoor air's class of the RITE (IT 1.1.4.2.4) taken when none is
/// given: ODA 2, a town.
pub const DEFAULT_ODA: u8 = 2;
/// The air speed of the main ducts of the RITE, m/s, taken when none is
/// given.
pub const DEFAULT_DUCT_SPEED: f64 = 5.0;

/// The ventilation's data.
#[derive(Debug, Clone, PartialEq)]
pub struct VentilationSite {
    pub system: VentSystem,
    pub zone: DraftZone,
    /// The garages (version 99).
    pub garage: GarageVent,
    /// The outdoor air's class, 1 to 3 (RITE IT 1.1.4.2.4, for the
    /// filters; version 99).
    pub oda: u8,
    /// The air speed of the main ducts of the rooms ventilated by the
    /// RITE, m/s (the branches to a single terminal take 60 % of it;
    /// version 99).
    pub duct_speed: f64,
    /// The indoor air class (IDA 1 to 4) set by hand on a room; the others
    /// take the one of their use (RITE IT 1.1.4.2.1; version 99).
    pub ida: BTreeMap<RoomId, u8>,
}

impl Default for VentilationSite {
    fn default() -> Self {
        VentilationSite {
            system: VentSystem::default(),
            zone: DraftZone::default(),
            garage: GarageVent::default(),
            oda: DEFAULT_ODA,
            duct_speed: DEFAULT_DUCT_SPEED,
            ida: BTreeMap::new(),
        }
    }
}

impl Model {
    pub fn ventilation_site(&self) -> &VentilationSite {
        &self.ventilation_site
    }

    pub fn set_ventilation_site(&mut self, site: VentilationSite) -> Result<(), ModelError> {
        if !(1..=3).contains(&site.oda) {
            return Err(ModelError::InvalidParameter(
                "la calidad del aire exterior va de ODA 1 a ODA 3".into(),
            ));
        }
        if !(site.duct_speed > 0.5 && site.duct_speed <= 20.0) {
            return Err(ModelError::InvalidParameter(
                "la velocidad de los conductos va de 0,5 a 20 m/s".into(),
            ));
        }
        if site.ida.values().any(|c| !(1..=4).contains(c)) {
            return Err(ModelError::InvalidParameter(
                "la calidad del aire interior va de IDA 1 a IDA 4".into(),
            ));
        }
        self.ventilation_site = site;
        Ok(())
    }

    /// Sets (or, with `None`, takes away) the indoor air class of a room.
    pub fn set_room_ida(&mut self, room: RoomId, ida: Option<u8>) -> Result<(), ModelError> {
        let mut site = self.ventilation_site.clone();
        match ida {
            Some(c) => {
                site.ida.insert(room, c);
            }
            None => {
                site.ida.remove(&room);
            }
        }
        self.set_ventilation_site(site)
    }

    /// The indoor air class set by hand on a room, if any (and the room is
    /// still there).
    pub fn room_ida(&self, room: RoomId) -> Option<u8> {
        self.ventilation_site
            .ida
            .get(&room)
            .copied()
            .filter(|_| self.room(room).is_some())
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
        assert_eq!(GarageVent::from_code("natural"), Some(GarageVent::Natural));
        let s = VentilationSite::default();
        assert_eq!((s.oda, s.duct_speed), (DEFAULT_ODA, DEFAULT_DUCT_SPEED));
    }
}
