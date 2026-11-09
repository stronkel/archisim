//! The indoor air of the rooms that are not of dwellings, by the RITE
//! (IT 1.1.4.2, «Exigencia de calidad del aire interior»), on the model as
//! drawn: offices, shops, schools, clinics, bars and restaurants, and the
//! service rooms beside them (toilets, changing rooms, cleaning rooms).
//!
//! - **The indoor air class** (IDA 1 to 4, IT 1.1.4.2.2) by the room's use
//!   ([`ida_of`]), or set by hand on the room
//!   (`VentilationSite::ida`).
//! - **The outdoor air** (IT 1.1.4.2.3): by person (method A, table
//!   1.4.2.1), with the people of the room by the occupancy of the DB-SI 3
//!   ([`archisim_core::occupancy`]); by floor area (method D, table
//!   1.4.2.4) in the rooms not occupied for long (stores, archives); and
//!   2,5 l/s per m² in heated pools.
//! - **The service rooms** (IT 1.1.4.2.5): extraction of 2 l/s per m².
//! - **The filters** (IT 1.1.4.2.4, table 1.4.2.5), from the outdoor air's
//!   class (ODA) and the IDA: shown, not checked.
//! - **Heat recovery** (IT 1.2.4.5.2): needed when the air thrown out by
//!   fans passes [`RECOVERY_ABOVE`]; its least efficiency is the one of the
//!   ecodesign of the ventilation units.
//!
//! The ducts of these rooms are sized by the air's speed (the site's
//! `duct_speed`), in `ventilation`.
//!
//! **Written from memory, to be checked** («para revisar»): every figure
//! below, together.

use archisim_core::occupancy::{density, occupants, plain, Density};
use archisim_core::{BuildingUse, DeviceId, DeviceKind, LevelId, Model, Room, RoomId};

/// Para revisar. Table 1.4.2.1: outdoor air per person, l/s, IDA 1 to 4.
pub const PER_PERSON: [f64; 4] = [20.0, 12.5, 8.0, 5.0];
/// Para revisar. Table 1.4.2.4: outdoor air per m² of the rooms not
/// occupied for long, l/s, IDA 1 to 4 (IDA 1: not allowed; the one of
/// IDA 2 is taken).
pub const PER_M2: [Option<f64>; 4] = [None, Some(0.83), Some(0.55), Some(0.28)];
/// Para revisar. IT 1.1.4.2.5: the extraction of the service rooms, l/s
/// per m² of floor.
pub const SERVICE_PER_M2: f64 = 2.0;
/// Para revisar. IT 1.1.4.2.3: a heated pool, l/s per m² of its water and
/// beach (the room's whole area is taken).
pub const POOL_PER_M2: f64 = 2.5;
/// Para revisar. IT 1.2.4.5.2: the air thrown out by fans above which
/// its heat is recovered, m³/s (0,28 since the 2021 amendment; 0,5 before).
pub const RECOVERY_ABOVE: f64 = 0.28;
/// Para revisar. The least efficiency of the heat recovery of a
/// non-residential ventilation unit (ecodesign, Regulation (EU) 1253/2014,
/// from 2018), and with a run-around coil.
pub const RECOVERY_EFFICIENCY: f64 = 0.73;
pub const RECOVERY_EFFICIENCY_COIL: f64 = 0.68;
/// Para revisar. Table 1.4.2.5: the least filters, by ODA (rows 1 to 3)
/// and IDA (columns 1 to 4), in the classes of EN 779 (the 2021 text may
/// give them by ISO 16890).
pub const FILTERS: [[&str; 4]; 3] = [
    ["F9", "F8", "F7", "F5"],
    ["F7 + F9", "F6 + F8", "F5 + F7", "F5 + F6"],
    ["F7 + GF + F9", "F7 + GF + F9", "F5 + F7", "F5 + F6"],
];
/// The branches to a single terminal: this part of the main ducts' speed.
pub const BRANCH_SPEED: f64 = 0.6;

/// How a room's air is worked out.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Method {
    /// Method A: by its people (table 1.4.2.1).
    Person,
    /// Method D: by its floor (table 1.4.2.4), not occupied for long.
    Area,
    /// A service room: extraction by its floor (IT 1.1.4.2.5).
    Service,
    /// A heated pool (IT 1.1.4.2.3).
    Pool,
}

impl Method {
    pub fn name(self) -> &'static str {
        match self {
            Method::Person => "por persona (tabla 1.4.2.1)",
            Method::Area => "por superficie (tabla 1.4.2.4)",
            Method::Service => "local de servicio, extracción (IT 1.1.4.2.5)",
            Method::Pool => "piscina (IT 1.1.4.2.3)",
        }
    }
}

/// A room ventilated by the RITE.
#[derive(Debug, Clone, PartialEq)]
pub struct RiteRoom {
    pub room: RoomId,
    pub name: String,
    pub level: LevelId,
    pub area: f64,
    /// IDA 1 to 4, and whether set by hand; why (its use).
    pub ida: u8,
    pub by_hand: bool,
    pub why: &'static str,
    pub method: Method,
    /// Its people and where their density comes from (DB-SI 3).
    pub people: u32,
    pub density: String,
    /// Outdoor air brought in, l/s.
    pub outdoor: f64,
    /// Air taken out by its own valves, l/s (the service rooms; the others
    /// give back what comes in, where they have valves).
    pub extraction: f64,
    /// Supply terminals drawn in it (diffusers, grilles, inlets) and
    /// extract valves.
    pub supply: Vec<DeviceId>,
    pub vents: Vec<DeviceId>,
}

impl RiteRoom {
    /// It needs outdoor air and nothing brings it in.
    pub fn no_supply(&self) -> bool {
        self.outdoor > 0.0 && self.supply.is_empty()
    }

    /// A service room without extraction.
    pub fn no_vent(&self) -> bool {
        self.extraction > 0.0 && self.vents.is_empty()
    }

    /// The flow each of its supply terminals gives, l/s.
    pub fn supply_flow(&self) -> f64 {
        self.outdoor / self.supply.len().max(1) as f64
    }

    /// The flow each of its valves takes: a service room its extraction;
    /// another its outdoor air back.
    pub fn vent_flow(&self) -> f64 {
        let q = if self.extraction > 0.0 {
            self.extraction
        } else {
            self.outdoor
        };
        q / self.vents.len().max(1) as f64
    }
}

/// The rooms of the RITE in the building.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Rite {
    pub rooms: Vec<RiteRoom>,
    /// Rooms of these uses left out, and why (no people, circulation).
    pub left: Vec<(RoomId, String)>,
    /// Outdoor air, extraction of the service rooms, and what the fans
    /// throw out (the larger: the outdoor air goes out somewhere), l/s.
    pub outdoor: f64,
    pub extraction: f64,
    pub expelled: f64,
    /// The outdoor air's class (1 to 3).
    pub oda: u8,
    /// Heat recovery is needed, and the unit drawn (equipment named
    /// «recuperador»).
    pub recovery_needed: bool,
    pub recovery: Option<DeviceId>,
}

impl Rite {
    pub fn applies(&self) -> bool {
        !self.rooms.is_empty()
    }

    /// The classes used, with their filters.
    pub fn filters(&self) -> Vec<(u8, &'static str)> {
        let mut used: Vec<u8> = self
            .rooms
            .iter()
            .filter(|r| r.outdoor > 0.0)
            .map(|r| r.ida)
            .collect();
        used.sort_unstable();
        used.dedup();
        used.into_iter()
            .map(|i| (i, filters(self.oda, i)))
            .collect()
    }
}

/// The filters of table 1.4.2.5.
pub fn filters(oda: u8, ida: u8) -> &'static str {
    FILTERS[usize::from(oda.clamp(1, 3)) - 1][usize::from(ida.clamp(1, 4)) - 1]
}

fn has(t: &str, words: &[&str]) -> bool {
    words.iter().any(|w| t.contains(w))
}

fn text(r: &Room) -> String {
    plain(&format!("{} {}", r.usage, r.name))
}

/// A service room: toilets, changing rooms, cleaning rooms, kitchens.
pub fn is_service(t: &str) -> bool {
    has(
        t,
        &[
            "aseo",
            "bano",
            "wc",
            "vestuario",
            "limpieza",
            "cocina",
            "office",
        ],
    )
}

/// A room of a use that is not a dwelling's (in a building of dwellings:
/// the shops and offices of its ground floor).
pub fn is_non_residential(t: &str) -> bool {
    has(
        t,
        &[
            "comercial",
            "comercio",
            "tienda",
            "local comercial",
            "venta",
            "oficina",
            "administrativo",
            "cafe",
            "bar ",
            "restaurante",
            "cafeteria",
            "aula",
            "docente",
            "consulta",
            "clinica",
            "gimnasio",
            "sala de reuniones",
            "recepcion",
        ],
    ) || t.ends_with(" bar")
}

/// The building use a room is of, for its people: the building's, or in a
/// building of dwellings what its name says.
fn use_of(t: &str, building: BuildingUse) -> BuildingUse {
    if building != BuildingUse::Dwelling {
        return building;
    }
    if has(t, &["cafe", "bar", "restaurante", "cafeteria", "gimnasio"]) {
        BuildingUse::Assembly
    } else if has(t, &["oficina", "administrativo", "despacho", "reuniones"]) {
        BuildingUse::Administrative
    } else if has(t, &["aula", "docente"]) {
        BuildingUse::Teaching
    } else if has(t, &["consulta", "clinica"]) {
        BuildingUse::Hospital
    } else {
        BuildingUse::Commercial
    }
}

/// IT 1.1.4.2.2: the indoor air class of a room by its use, and why.
pub fn ida_of(t: &str, building: BuildingUse) -> (u8, &'static str) {
    if has(
        t,
        &[
            "hospital",
            "clinica",
            "laboratorio",
            "guarderia",
            "infantil",
            "quirofano",
        ],
    ) {
        return (1, "hospitales, clínicas, laboratorios y guarderías");
    }
    if has(
        t,
        &[
            "oficina",
            "administrativo",
            "despacho",
            "reuniones",
            "aula",
            "lectura",
            "biblioteca",
            "museo",
            "tribunal",
            "piscina",
            "residencia",
        ],
    ) {
        return (
            2,
            "oficinas, residencias, salas de lectura, museos, aulas y piscinas",
        );
    }
    if has(
        t,
        &[
            "comercial",
            "comercio",
            "tienda",
            "venta",
            "cine",
            "teatro",
            "salon de actos",
            "habitacion",
            "restaurante",
            "cafe",
            "bar",
            "gimnasio",
            "deporte",
            "ordenadores",
        ],
    ) {
        return (
            3,
            "comercios, cines, teatros, restaurantes, bares, gimnasios y habitaciones de hotel",
        );
    }
    match building {
        BuildingUse::Hospital => (1, "uso hospitalario"),
        BuildingUse::Administrative | BuildingUse::Teaching | BuildingUse::PublicResidential => (
            2,
            "uso del edificio: oficinas, docente o residencial público",
        ),
        _ => (
            3,
            "uso del edificio: comercial, pública concurrencia u otros",
        ),
    }
}

/// A room of the RITE, or why it is left out. `building` is the building's
/// use.
pub fn rite_room(model: &Model, building: BuildingUse, r: &Room) -> Result<RiteRoom, String> {
    let t = text(r);
    let area = r.area();
    let (auto, why) = ida_of(&t, building);
    let by_hand = model.room_ida(r.id);
    let ida = by_hand.unwrap_or(auto);
    let make = |method, people, density: String, outdoor, extraction| RiteRoom {
        room: r.id,
        name: r.name.clone(),
        level: r.level,
        area,
        ida,
        by_hand: by_hand.is_some(),
        why: if by_hand.is_some() {
            "puesta a mano"
        } else {
            why
        },
        method,
        people,
        density,
        outdoor,
        extraction,
        supply: Vec::new(),
        vents: Vec::new(),
    };
    if is_service(&t) {
        let mut x = make(
            Method::Service,
            0,
            String::new(),
            0.0,
            SERVICE_PER_M2 * area,
        );
        if !x.by_hand {
            x.why = "local de servicio: su aire de extracción es AE 2";
        }
        return Ok(x);
    }
    if has(&t, &["piscina"]) {
        return Ok(make(
            Method::Pool,
            0,
            String::new(),
            POOL_PER_M2 * area,
            0.0,
        ));
    }
    let per_m2 = PER_M2[usize::from(ida) - 1].or(PER_M2[1]).unwrap_or(0.83);
    if has(
        &t,
        &["almacen", "archivo", "trastienda", "deposito", "cuarto de"],
    ) {
        return Ok(make(Method::Area, 0, String::new(), per_m2 * area, 0.0));
    }
    let d: Density = density(model, use_of(&t, building), r);
    if d.per_person.is_none() {
        return Err(d.source);
    }
    let people = occupants(area, &d);
    if people == 0 {
        return Err("sin ocupación".into());
    }
    Ok(make(
        Method::Person,
        people,
        d.source,
        PER_PERSON[usize::from(ida) - 1] * f64::from(people),
        0.0,
    ))
}

/// Fills the totals and the heat recovery of the rooms found.
pub fn finish(model: &Model, rite: &mut Rite) {
    rite.oda = model.ventilation_site().oda;
    rite.outdoor = rite.rooms.iter().map(|r| r.outdoor).sum();
    rite.extraction = rite.rooms.iter().map(|r| r.extraction).sum();
    set_expelled(rite, 0.0);
    rite.recovery = model
        .devices()
        .filter(|d| d.kind == DeviceKind::Equipment)
        .find(|d| plain(&d.name).contains("recuperador"))
        .map(|d| d.id);
}

fn dec(v: f64, n: usize) -> String {
    format!("{v:.n$}").replace('.', ",")
}

/// What the RITE asks of a room, in words.
pub fn room_text(r: &RiteRoom) -> String {
    let flow = match r.method {
        Method::Person => format!(
            "{} persona{} × {} l/s = {} l/s de aire exterior",
            r.people,
            if r.people == 1 { "" } else { "s" },
            dec(PER_PERSON[usize::from(r.ida) - 1], 1),
            dec(r.outdoor, 1)
        ),
        Method::Area | Method::Pool => format!(
            "{} m² × {} l/s·m² = {} l/s de aire exterior",
            dec(r.area, 1),
            dec(r.outdoor / r.area.max(1e-9), 2),
            dec(r.outdoor, 1)
        ),
        Method::Service => format!(
            "{} m² × {} l/s·m² = {} l/s de extracción",
            dec(r.area, 1),
            dec(SERVICE_PER_M2, 0),
            dec(r.extraction, 1)
        ),
    };
    let mut s = format!(
        "{}: IDA {} ({}), {}: {}",
        r.name,
        r.ida,
        r.why,
        r.method.name(),
        flow
    );
    if r.no_supply() {
        s.push_str(" — sin impulsión dibujada (difusores, rejillas o aireadores)");
    } else if r.outdoor > 0.0 {
        s.push_str(&format!(
            "; {} terminal{} de impulsión de {} l/s",
            r.supply.len(),
            if r.supply.len() == 1 { "" } else { "es" },
            dec(r.supply_flow(), 1)
        ));
    }
    if r.no_vent() {
        s.push_str(" — sin boca de extracción dibujada");
    }
    s
}

/// What the fans throw out, with the extract networks drawn (their flow,
/// l/s), and so whether the heat must be recovered.
pub fn set_expelled(rite: &mut Rite, drawn: f64) {
    rite.expelled = rite.outdoor.max(rite.extraction).max(drawn);
    rite.recovery_needed = rite.expelled / 1000.0 > RECOVERY_ABOVE + 1e-9;
}

/// The heat recovery, in words.
pub fn recovery_text(rite: &Rite) -> String {
    let m3 = rite.expelled / 1000.0;
    if !rite.recovery_needed {
        return format!(
            "Aire expulsado {} m³/s ≤ {} m³/s: la recuperación de calor no es obligatoria (IT 1.2.4.5.2)",
            dec(m3, 3),
            dec(RECOVERY_ABOVE, 2)
        );
    }
    format!(
        "Aire expulsado {} m³/s > {} m³/s: recuperación de calor obligatoria, eficiencia ≥ {} % ({} % con batería de doble circuito; IT 1.2.4.5.2 y diseño ecológico) — {}",
        dec(m3, 3),
        dec(RECOVERY_ABOVE, 2),
        dec(RECOVERY_EFFICIENCY * 100.0, 0),
        dec(RECOVERY_EFFICIENCY_COIL * 100.0, 0),
        match rite.recovery {
            Some(_) => "recuperador dibujado".to_string(),
            None => "a prever (no hay ningún equipo llamado «recuperador»)".to_string(),
        }
    )
}

/// An example (for the tests and `archisim ventilacion --ejemplo-terciario`):
/// an office building. On `ground`, an office of 12 × 8 m (an open office
/// of 64 m², a meeting room and a toilet), with a heat recovery unit that
/// supplies three diffusers and takes the air back from the office and the
/// toilet; on `basement`, a garage of 25 × 10 m (ten spaces) ventilated
/// naturally by two openings without a door in its long walls.
#[allow(clippy::too_many_arguments)]
pub fn example_office_garage(
    m: &mut Model,
    basement: archisim_core::LevelId,
    ground: archisim_core::LevelId,
    partition: archisim_core::WallTypeId,
    facade: archisim_core::WallTypeId,
    door_type: archisim_core::OpeningTypeId,
    void_type: archisim_core::OpeningTypeId,
) {
    use archisim_core::{
        Hand, NewDevice, NewOpening, NewRoom, NewRun, NewWall, Point2, ReferenceLine,
        ServiceSystem, WallSide, WallTop,
    };
    let mut cs = m.code_settings();
    cs.building_use = BuildingUse::Administrative;
    m.set_code_settings(cs);
    let wall = |m: &mut Model, level, t, a: (f64, f64), b: (f64, f64)| {
        m.add_wall(NewWall {
            wall_type: t,
            level,
            start: Point2::new(a.0, a.1),
            end: Point2::new(b.0, b.1),
            bulge: 0.0,
            reference: ReferenceLine::Center,
            height: 2.8,
            base_offset: 0.0,
            top: WallTop::Unconnected,
        })
        .expect("example office")
    };
    let room =
        |m: &mut Model, level, n: &str, name: &str, usage: &str, a: (f64, f64), b: (f64, f64)| {
            let id = m
                .add_room(NewRoom {
                    level,
                    number: n.into(),
                    name: name.into(),
                    outline: vec![
                        Point2::new(a.0, a.1),
                        Point2::new(b.0, a.1),
                        Point2::new(b.0, b.1),
                        Point2::new(a.0, b.1),
                    ],
                    holes: Vec::new(),
                    label: Point2::new((a.0 + b.0) / 2.0, (a.1 + b.1) / 2.0),
                    bounded: false,
                })
                .expect("example office");
            let mut r = m.room(id).expect("example office").clone();
            r.usage = usage.into();
            m.set_room(r).expect("example office");
        };
    let opening = |m: &mut Model, w, t, along| {
        m.add_opening(NewOpening {
            opening_type: t,
            wall: w,
            along,
            sill: 0.0,
            opens_to: WallSide::Interior,
            hand: Hand::Left,
            setback: None,
        })
        .expect("example office");
    };
    // The office.
    for (a, b) in [
        ((0.0, 0.0), (12.0, 0.0)),
        ((12.0, 0.0), (12.0, 8.0)),
        ((12.0, 8.0), (0.0, 8.0)),
        ((0.0, 8.0), (0.0, 0.0)),
    ] {
        wall(m, ground, facade, a, b);
    }
    let middle = wall(m, ground, partition, (8.0, 0.0), (8.0, 8.0));
    wall(m, ground, partition, (8.0, 4.0), (12.0, 4.0));
    opening(m, middle, door_type, 2.0);
    opening(m, middle, door_type, 6.0);
    let h = 0.05;
    room(
        m,
        ground,
        "1",
        "Oficina",
        "Oficinas",
        (h, h),
        (8.0 - h, 8.0 - h),
    );
    room(
        m,
        ground,
        "2",
        "Aseo",
        "Aseo",
        (8.0 + h, h),
        (12.0 - h, 4.0 - h),
    );
    room(
        m,
        ground,
        "3",
        "Sala de reuniones",
        "Oficinas",
        (8.0 + h, 4.0 + h),
        (12.0 - h, 8.0 - h),
    );
    let dev = |m: &mut Model,
               kind: DeviceKind,
               name: &str,
               x: f64,
               y: f64,
               w: f64,
               d: f64,
               z: f64,
               hh: f64| {
        m.add_device(NewDevice {
            level: ground,
            kind,
            name: name.into(),
            position: Point2::new(x, y),
            rotation: 0.0,
            elevation: z,
            width: w,
            depth: d,
            height: hh,
        })
        .expect("example office")
    };
    dev(
        m,
        DeviceKind::Equipment,
        "Recuperador de calor",
        6.0,
        7.5,
        1.0,
        0.6,
        2.5,
        0.3,
    );
    for (x, y) in [(2.0, 4.0), (6.0, 4.0), (10.0, 6.0)] {
        dev(
            m,
            DeviceKind::Diffuser,
            "Difusor",
            x,
            y,
            0.6,
            0.6,
            2.7,
            0.05,
        );
    }
    dev(
        m,
        DeviceKind::ExhaustVent,
        "Boca de extracción",
        10.0,
        2.0,
        0.16,
        0.16,
        2.7,
        0.05,
    );
    dev(
        m,
        DeviceKind::ExhaustVent,
        "Rejilla de retorno",
        2.0,
        7.0,
        0.2,
        0.2,
        2.7,
        0.05,
    );
    let run = |m: &mut Model, system, path: &[(f64, f64)], w: f64, hh: f64| {
        m.add_run(NewRun {
            level: ground,
            system,
            path: path.iter().map(|p| Point2::new(p.0, p.1)).collect(),
            elevation: 2.75,
            width: w,
            height: hh,
            slope: 0.0,
        })
        .expect("example office");
    };
    // Supply: 112,5 l/s out of the unit (≥ 225 cm² at 5 m/s).
    run(
        m,
        ServiceSystem::SupplyAir,
        &[(6.0, 7.5), (6.0, 4.0)],
        0.25,
        0.15,
    );
    run(
        m,
        ServiceSystem::SupplyAir,
        &[(6.0, 4.0), (2.0, 4.0)],
        0.15,
        0.1,
    );
    run(
        m,
        ServiceSystem::SupplyAir,
        &[(6.0, 7.5), (10.0, 7.5), (10.0, 6.0)],
        0.15,
        0.1,
    );
    // Extraction: the toilet's 32 l/s and the office's air back.
    run(
        m,
        ServiceSystem::ExhaustAir,
        &[(10.0, 2.0), (10.0, 3.5), (7.0, 3.5), (7.0, 7.0), (5.5, 7.0)],
        0.2,
        0.15,
    );
    run(
        m,
        ServiceSystem::ExhaustAir,
        &[(2.0, 7.0), (5.5, 7.0)],
        0.2,
        0.15,
    );
    // The garage, under it.
    let mut long = Vec::new();
    for (a, b) in [
        ((0.0, 0.0), (25.0, 0.0)),
        ((25.0, 0.0), (25.0, 10.0)),
        ((25.0, 10.0), (0.0, 10.0)),
        ((0.0, 10.0), (0.0, 0.0)),
    ] {
        long.push(wall(m, basement, facade, a, b));
    }
    opening(m, long[0], void_type, 12.5);
    opening(m, long[2], void_type, 12.5);
    room(
        m,
        basement,
        "S1",
        "Garaje",
        "Aparcamiento",
        (h, h),
        (25.0 - h, 10.0 - h),
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_classes_and_filters() {
        let b = BuildingUse::Dwelling;
        assert_eq!(ida_of("administrativo oficina", b).0, 2);
        assert_eq!(ida_of("comercial cafe", b).0, 3);
        assert_eq!(ida_of("consulta clinica", b).0, 1);
        assert_eq!(ida_of("sala", BuildingUse::Hospital).0, 1);
        assert_eq!(ida_of("sala", BuildingUse::Commercial).0, 3);
        assert_eq!(filters(1, 2), "F8");
        assert_eq!(filters(2, 3), "F5 + F7");
        assert!(is_service("aseo aseo accesible") && !is_service("oficina"));
        assert!(
            is_non_residential("comercial cafe") && !is_non_residential("estar estar-cocina a")
        );
    }

    fn office() -> Model {
        let mut m = Model::new("Oficina");
        let s = m.add_level("Sótano", -3.0);
        let l0 = m.add_level("Planta baja", 0.0);
        m.add_level("Cubierta", 3.0);
        let lib = archisim_library::add_library(&mut m).unwrap();
        example_office_garage(
            &mut m,
            s,
            l0,
            lib.partition,
            lib.facade,
            lib.interior_door,
            lib.void,
        );
        m
    }

    fn room_named(m: &Model, name: &str) -> RoomId {
        m.rooms().find(|r| r.name == name).unwrap().id
    }

    #[test]
    fn an_office_by_the_rite() {
        let m = office();
        let v = crate::ventilation::ventilation(&m);
        assert!(!v.applies, "not a building of dwellings");
        assert_eq!(v.rite.rooms.len(), 3, "{:?}", v.rite);
        let office = v.rite.rooms.iter().find(|r| r.name == "Oficina").unwrap();
        // 64 m² of offices at 10 m² a person: 7 people × 12,5 l/s.
        assert_eq!(
            (office.ida, office.method, office.people),
            (2, Method::Person, 7)
        );
        assert!((office.outdoor - 87.5).abs() < 1e-9);
        assert_eq!(office.supply.len(), 2);
        let toilet = v.rite.rooms.iter().find(|r| r.name == "Aseo").unwrap();
        assert_eq!(toilet.method, Method::Service);
        assert!((toilet.extraction - SERVICE_PER_M2 * toilet.area).abs() < 1e-9);
        assert!((v.rite.outdoor - 112.5).abs() < 1e-9);
        assert!(!v.rite.recovery_needed);
        // One supply network from the unit to the three diffusers and one
        // extract network, both sized by the speed.
        let supply: Vec<_> = v.networks.iter().filter(|n| n.supply).collect();
        assert_eq!(supply.len(), 1);
        assert_eq!(supply[0].vents.len(), 3);
        assert!(supply[0].fan.is_some() && supply[0].rite);
        assert!((supply[0].flow - 112.5).abs() < 1e-9);
        let extract: Vec<_> = v.networks.iter().filter(|n| !n.supply).collect();
        assert_eq!(extract.len(), 1);
        assert!(extract[0].rite && extract[0].outlet.is_none());
        assert!(v
            .networks
            .iter()
            .flat_map(|n| &n.stretches)
            .all(|s| s.speed.is_some()));
        assert!(v
            .networks
            .iter()
            .flat_map(|n| &n.stretches)
            .all(|s| !s.short()));
        // The garage under it passes naturally.
        let g = v.others.iter().find(|o| o.garage).unwrap();
        assert_eq!(g.spaces, 10);
        let n = g.natural.as_ref().expect("natural");
        assert_eq!(n.openings.len(), 2);
        assert!(
            n.opposite.is_some() && n.zones_pass() && n.passes(),
            "{n:?}"
        );
        assert!(n.farthest < 14.0);
        assert_eq!(v.fails(), 0, "{}", crate::ventilation::report(&m, &v));
    }

    #[test]
    fn the_class_by_hand_and_the_heat_recovery() {
        let mut m = office();
        let id = room_named(&m, "Oficina");
        m.set_room_ida(id, Some(1)).unwrap();
        let v = crate::ventilation::ventilation(&m);
        let office = v.rite.rooms.iter().find(|r| r.room == id).unwrap();
        assert!(office.by_hand && (office.outdoor - 140.0).abs() < 1e-9);
        assert!(m.set_room_ida(id, Some(5)).is_err());
        m.set_room_ida(id, None).unwrap();
        // Crowded: 2 m² a person, 32 people, 400 l/s > 0,28 m³/s.
        let mut r = m.room(id).unwrap().clone();
        r.occupancy = Some(2.0);
        m.set_room(r).unwrap();
        let v = crate::ventilation::ventilation(&m);
        assert!(v.rite.recovery_needed, "{:?}", v.rite.expelled);
        assert!(v.rite.recovery.is_some(), "the unit named «Recuperador»");
        // Its ducts are now short.
        assert!(v
            .networks
            .iter()
            .flat_map(|n| &n.stretches)
            .any(|s| s.short()));
        assert!(recovery_text(&v.rite).contains("obligatoria"));
    }

    #[test]
    fn a_garage_open_on_one_side_needs_fans() {
        let mut m = office();
        let north = m
            .openings()
            .filter(|o| {
                m.wall(o.wall)
                    .is_some_and(|w| w.start.y > 9.0 && w.end.y > 9.0)
            })
            .map(|o| o.id)
            .next()
            .unwrap();
        m.remove_opening(north).unwrap();
        let v = crate::ventilation::ventilation(&m);
        let g = v.others.iter().find(|o| o.garage).unwrap();
        let n = g.natural.as_ref().unwrap();
        assert!(n.opposite.is_none() && !n.passes());
        assert_eq!(g.garage_fails(), 1);
        let lines = crate::ventilation::other_lines(g).join("\n");
        assert!(lines.contains("necesita ventilación mecánica"), "{lines}");
        // Mechanical by choice, without valves: fails too.
        let mut site = m.ventilation_site().clone();
        site.garage = archisim_core::GarageVent::Mechanical;
        m.set_ventilation_site(site).unwrap();
        let v = crate::ventilation::ventilation(&m);
        let g = v.others.iter().find(|o| o.garage).unwrap();
        assert!(g.natural.is_none() && g.mechanical);
        assert_eq!(g.garage_fails(), 1);
    }

    #[test]
    fn a_shop_under_the_flats() {
        let mut m = Model::new("Piso y local");
        let l0 = m.add_level("Planta baja", 0.0);
        let l1 = m.add_level("Cubierta", 3.0);
        let lib = archisim_library::add_library(&mut m).unwrap();
        crate::ventilation::example_flat(
            &mut m,
            l0,
            l1,
            lib.partition,
            lib.facade,
            lib.interior_door,
        );
        let o = |x: f64, y: f64| archisim_core::Point2::new(x, y);
        m.add_room(archisim_core::NewRoom {
            level: l0,
            number: "L".into(),
            name: "Local comercial".into(),
            outline: vec![o(20.0, 0.0), o(30.0, 0.0), o(30.0, 6.0), o(20.0, 6.0)],
            holes: Vec::new(),
            label: o(25.0, 3.0),
            bounded: false,
        })
        .unwrap();
        let v = crate::ventilation::ventilation(&m);
        assert_eq!(v.dwellings.len(), 1);
        assert_eq!(v.rite.rooms.len(), 1);
        let shop = &v.rite.rooms[0];
        // 60 m² of shop on the ground floor, 2 m² a person: 30 × 8 l/s.
        assert_eq!((shop.ida, shop.people), (3, 30));
        assert!((shop.outdoor - 240.0).abs() < 1e-9);
        assert!(shop.no_supply());
    }
}
