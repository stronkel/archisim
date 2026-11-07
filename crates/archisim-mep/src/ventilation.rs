//! The ventilation of the dwellings by the DB-HS 3 (2017 update, «Calidad
//! del aire interior»), on the model as drawn.
//!
//! - **The dwellings.** The rooms of a level joined by doors, leaving out
//!   the common ones (corridors and stairs of the building, the portal,
//!   the garage, storerooms, plant rooms, terraces). A dwelling has one
//!   dry room at least.
//! - **The rooms.** Dry («secos»: bedrooms, living and dining rooms,
//!   studies), wet («húmedos»: kitchens, bathrooms, toilets, laundry
//!   rooms) or passages. A living room with its kitchen («estar-cocina»)
//!   is both: air comes in by its living part and goes out by its kitchen.
//! - **The flows**, table 2.1, by the number of bedrooms (a study or an
//!   office counts as one): 8 l/s the main
//!   bedroom (the largest), 4 the others, 6, 8 or 10 each living or dining
//!   room; out of the wet rooms, 6, 7 or 8 l/s each and 12, 24 or 33 in
//!   all. What comes in must go out: the larger of the two is the
//!   dwelling's flow, and the other side is raised to it.
//! - **The openings**, table 4.1: air inlets of 4·q cm² in every dry
//!   room, extract valves of 4·q cm² in every wet one, and passages of
//!   8·q cm² (70 cm² at least) in the doors the air goes through on its
//!   way from the dry rooms to the wet ones; a door gives the gap under
//!   its leaf ([`UNDERCUT`]), an opening without a door its whole size.
//! - **The ducts.** The extract network is the system «Extracción»: its
//!   ducts, the extract shafts between floors and the fan at the head.
//!   Every stretch carries the flow of the valves after it. Mechanical
//!   extraction (4.2.2): 2,5·q cm² beside the rooms, 1,5·q cm² on the
//!   roof. Hybrid ventilation (4.2.1, [`VentSystem::Hybrid`]): the
//!   sections of table 4.2 by the draught class of table 4.3, from the
//!   floors that vent into the duct and the zone of table 4.4.
//! - **The outlets** (3.2.1): every duct's outlet (its fan, or the top of
//!   a duct without one) at 3 m at least from windows, outside doors, air
//!   inlets and terraces; with hybrid ventilation, also 1 m above the roof
//!   (2 m if it is walked on), above every obstacle at 2 to 10 m and 1,3
//!   times as high as any nearer.
//! - **The kitchen** also needs its own extraction for the hood (50 l/s,
//!   3.1.1), through a duct of its own.
//!
//! Storerooms and refuse rooms of the building (table 2.2) are given
//! their flow by their area, and garages 120 l/s a parking space (the cars
//! drawn in them, or one every 25 m²), with the openings of 3.1.4.
//! Buildings that are not dwellings are ventilated by the RITE
//! (IT 1.1.4.2): not worked out here.
//!
//! The figures of tables 2.1, 2.2, 4.1, 4.2, 4.3 and 4.4 and of 3.1.4,
//! 3.2.1 and 4.2.2 were checked against the text of the code (2022
//! edition); they are together below.

use std::collections::{BTreeMap, BTreeSet, VecDeque};

use archisim_core::{
    BuildingUse, DeviceId, DeviceKind, DraftZone, Hand, LevelId, Model, NewDevice, NewObject,
    NewOpening, NewRoom, NewRun, NewWall, ObjectId, ObjectTypeId, OpeningId, OpeningKind,
    OpeningTypeId, Point2, ReferenceLine, RoomId, RunId, ServiceSystem, VentSystem, WallSide,
    WallTop, WallTypeId,
};

use crate::water::graph;

/// The gap under a door's leaf the air goes through, metres.
pub const UNDERCUT: f64 = 0.01;
/// How near a valve must be to its duct, metres.
pub const JOIN_REACH: f64 = 0.45;
/// How near a device must be to a room to be in it (in its wall).
pub const ROOM_REACH: f64 = 0.35;
/// Table 4.1: an inlet's or a valve's area per l/s, cm².
pub const OPENING_PER_LS: f64 = 4.0;
/// Table 4.1: a passage's area per l/s, cm², and its least.
pub const PASSAGE_PER_LS: f64 = 8.0;
pub const PASSAGE_MIN: f64 = 70.0;
/// 4.2.2: a mechanical duct's section per l/s beside the rooms, cm²
/// (formula 4.1), and on the roof (formula 4.2).
pub const DUCT_PER_LS: f64 = 2.5;
pub const ROOF_DUCT_PER_LS: f64 = 1.5;
/// Table 2.2: a garage's flow per parking space, l/s, and the area taken
/// for a space when no car is drawn, m².
pub const GARAGE_PER_SPACE: f64 = 120.0;
pub const GARAGE_SPACE_AREA: f64 = 25.0;
/// 3.1.4: an admission and an extraction opening every 100 m²; the
/// extraction ones less than 10 m apart; two networks a floor from 15
/// spaces; detecting CO above 5 spaces or 100 m².
pub const GARAGE_OPENING_AREA: f64 = 100.0;
pub const GARAGE_VENT_SPACING: f64 = 10.0;
pub const GARAGE_TWO_NETWORKS: usize = 15;
pub const GARAGE_CO_SPACES: usize = 5;
/// 3.2.1.4: an outlet's distance to any air coming in, metres.
pub const OUTLET_GAP: f64 = 3.0;
/// 3.2.1.5 (hybrid): the outlet above the roof, metres (walked on, the
/// second), and the obstacles: higher than those at 2 to 10 m, 1,3 times
/// those nearer.
pub const HYBRID_ABOVE: f64 = 1.0;
pub const HYBRID_ABOVE_WALKED: f64 = 2.0;
pub const OBSTACLE_NEAR: f64 = 2.0;
pub const OBSTACLE_FAR: f64 = 10.0;
pub const OBSTACLE_FACTOR: f64 = 1.3;

/// Table 4.3: the draught class (1 to 4) of a hybrid duct by the floors
/// venting into it (from the lowest to the last, both in) and the zone of
/// table 4.4.
pub fn draft_class(floors: usize, zone: DraftZone) -> u8 {
    const T: [[u8; 4]; 8] = [
        [3, 3, 4, 4],
        [2, 3, 3, 4],
        [2, 2, 3, 3],
        [2, 2, 2, 3],
        [1, 2, 2, 3],
        [1, 1, 2, 3],
        [1, 1, 1, 2],
        [1, 1, 1, 1],
    ];
    let row = floors.clamp(1, 8) - 1;
    let col = match zone {
        DraftZone::W => 0,
        DraftZone::X => 1,
        DraftZone::Y => 2,
        DraftZone::Z => 3,
    };
    T[row][col]
}

/// Table 4.2: the ducts a hybrid stretch needs (their sections, cm²) by
/// its flow and draught class; `None` above 1 000 l/s, out of the table.
pub fn hybrid_ducts(flow: f64, class: u8) -> Option<&'static [f64]> {
    const T: [(f64, [&[f64]; 4]); 5] = [
        (100.0, [&[225.0], &[400.0], &[625.0], &[625.0]]),
        (300.0, [&[400.0], &[625.0], &[625.0], &[900.0]]),
        (500.0, [&[625.0], &[900.0], &[900.0], &[900.0, 900.0]]),
        (
            750.0,
            [&[625.0], &[900.0], &[900.0, 625.0], &[900.0, 900.0, 900.0]],
        ),
        (
            1000.0,
            [
                &[900.0],
                &[900.0, 625.0],
                &[900.0, 900.0],
                &[900.0, 900.0, 900.0, 625.0],
            ],
        ),
    ];
    let c = usize::from(class.clamp(1, 4)) - 1;
    T.iter()
        .find(|(top, _)| flow <= *top + 1e-9)
        .map(|(_, r)| r[c])
}

/// «1 × 625 cm²», «900 + 625 cm²».
pub fn ducts_text(ducts: &[f64]) -> String {
    let same = ducts.windows(2).all(|w| (w[0] - w[1]).abs() < 1e-9);
    if same && !ducts.is_empty() {
        format!("{} × {} cm²", ducts.len(), ducts[0].round())
    } else {
        let parts: Vec<String> = ducts.iter().map(|d| format!("{}", d.round())).collect();
        format!("{} cm²", parts.join(" + "))
    }
}
/// 3.1.1: the kitchen hood's own extraction, l/s.
pub const HOOD_FLOW: f64 = 50.0;
/// Table 2.2: storerooms, l/s per m²; refuse rooms, l/s per m².
pub const STORE_PER_M2: f64 = 0.7;
pub const REFUSE_PER_M2: f64 = 10.0;

/// Valve diameters, metres.
pub const VENT_SIZES: [f64; 6] = [0.1, 0.125, 0.16, 0.2, 0.25, 0.315];
/// Round shaft and duct diameters, metres.
pub const SHAFT_SIZES: [f64; 14] = [
    0.1, 0.125, 0.15, 0.16, 0.2, 0.25, 0.3, 0.315, 0.355, 0.4, 0.45, 0.5, 0.56, 0.63,
];

/// Table 2.1: the least flows of a dwelling, l/s, by its bedrooms.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Flows {
    pub main_bedroom: f64,
    pub bedroom: f64,
    pub living: f64,
    /// Out of all the wet rooms together.
    pub wet_total: f64,
    /// Out of each wet room.
    pub wet_room: f64,
}

pub fn flows(bedrooms: usize) -> Flows {
    match bedrooms {
        0 | 1 => Flows {
            main_bedroom: 8.0,
            bedroom: 4.0,
            living: 6.0,
            wet_total: 12.0,
            wet_room: 6.0,
        },
        2 => Flows {
            main_bedroom: 8.0,
            bedroom: 4.0,
            living: 8.0,
            wet_total: 24.0,
            wet_room: 7.0,
        },
        _ => Flows {
            main_bedroom: 8.0,
            bedroom: 4.0,
            living: 10.0,
            wet_total: 33.0,
            wet_room: 8.0,
        },
    }
}

/// What a dry room is, for its flow.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Dry {
    MainBedroom,
    Bedroom,
    /// A study or office: counted as a bedroom, for its flow and for the
    /// kind of dwelling.
    Study,
    Living,
}

impl Dry {
    pub fn name(self) -> &'static str {
        match self {
            Dry::MainBedroom => "dormitorio principal",
            Dry::Bedroom => "dormitorio",
            Dry::Study => "estudio",
            Dry::Living => "estar o comedor",
        }
    }
}

/// What a room is, by its use and its name.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Class {
    pub bedroom: bool,
    pub study: bool,
    pub living: bool,
    pub wet: bool,
    pub kitchen: bool,
    pub passage: bool,
    /// Of the building, not of a dwelling.
    pub common: bool,
    pub store: bool,
    pub refuse: bool,
    pub garage: bool,
    /// A terrace, balcony or gallery: people stay there (3.2.1.4).
    pub terrace: bool,
}

impl Class {
    pub fn of(usage: &str, name: &str) -> Class {
        let t = format!("{usage} {name}").to_lowercase();
        let any = |words: &[&str]| words.iter().any(|w| t.contains(w));
        let kitchen = any(&["cocina", "office"]);
        Class {
            bedroom: any(&["dormitorio", "habitación", "habitacion"]),
            study: any(&["estudio", "despacho"]),
            living: any(&["estar", "salón", "salon", "comedor"]),
            wet: kitchen || any(&["baño", "bano", "aseo", "lavadero", "wc", "colada"]),
            kitchen,
            passage: any(&[
                "pasillo",
                "distribuidor",
                "vestíbulo",
                "vestibulo",
                "recibidor",
                "hall",
                "paso",
                "entrada",
                "vestidor",
                "despensa",
            ]),
            common: any(&[
                "común",
                "comun",
                "portal",
                "escalera",
                "garaje",
                "aparcamiento",
                "local",
                "instalaciones",
                "trastero",
                "basura",
                "residuos",
                "cubierta",
                "ascensor",
                "terraza",
                "balcón",
                "balcon",
                "porche",
                "patio",
            ]),
            store: any(&["trastero", "almacén", "almacen"]),
            refuse: any(&["basura", "residuos"]),
            // Of cars: not one of bicycles.
            garage: any(&["garaje", "aparcamiento", "parking"])
                && !any(&["bicicleta", "bicis", "motos"]),
            terrace: any(&[
                "terraza",
                "balcón",
                "balcon",
                "galería",
                "galeria",
                "solárium",
                "solarium",
                "azotea",
            ]),
        }
    }

    fn dry(&self) -> bool {
        self.bedroom || self.study || self.living
    }

    /// A room lived in: a bedroom, a study or a living or dining room.
    pub fn dry_room(&self) -> bool {
        self.dry()
    }

    fn of_dwelling(&self) -> bool {
        !self.common && (self.dry() || self.wet || self.passage)
    }
}

/// A room of a dwelling.
#[derive(Debug, Clone, PartialEq)]
pub struct VentRoom {
    pub room: RoomId,
    pub name: String,
    pub area: f64,
    pub dry: Option<Dry>,
    pub wet: bool,
    pub kitchen: bool,
    /// Air coming in, l/s.
    pub admission: f64,
    /// Air going out, l/s.
    pub extraction: f64,
    /// The inlets drawn in it and their free area, cm².
    pub inlets: Vec<DeviceId>,
    pub inlet_area: f64,
    /// The extract valves drawn in it and their area, cm².
    pub vents: Vec<DeviceId>,
    pub vent_area: f64,
}

impl VentRoom {
    /// The inlets' area it needs, cm².
    pub fn inlet_needed(&self) -> f64 {
        OPENING_PER_LS * self.admission
    }

    /// The valves' area it needs, cm².
    pub fn vent_needed(&self) -> f64 {
        OPENING_PER_LS * self.extraction
    }

    pub fn inlet_short(&self) -> bool {
        self.admission > 0.0 && self.inlet_area + 0.5 < self.inlet_needed()
    }

    pub fn vent_short(&self) -> bool {
        self.extraction > 0.0 && self.vent_area + 0.5 < self.vent_needed()
    }
}

/// A door the air goes through.
#[derive(Debug, Clone, PartialEq)]
pub struct Passage {
    pub opening: OpeningId,
    /// The rooms it joins, the way the air goes.
    pub from: RoomId,
    pub to: RoomId,
    pub flow: f64,
    /// Its clear width, metres; `None` for an opening without a door.
    pub door: Option<f64>,
    /// The area it gives, cm²: the gap under the leaf, or the opening.
    pub area: f64,
}

impl Passage {
    pub fn needed(&self) -> f64 {
        (PASSAGE_PER_LS * self.flow).max(PASSAGE_MIN)
    }

    pub fn short(&self) -> bool {
        self.area + 0.5 < self.needed()
    }

    /// The gap under the leaf that would be enough, millimetres.
    pub fn gap_needed(&self) -> Option<f64> {
        self.door
            .filter(|w| *w > 0.1)
            .map(|w| (self.needed() / (w * 1e4) * 1000.0).ceil())
    }
}

/// A dwelling.
#[derive(Debug, Clone, PartialEq)]
pub struct Dwelling {
    pub name: String,
    pub level: LevelId,
    pub bedrooms: usize,
    pub rooms: Vec<VentRoom>,
    pub passages: Vec<Passage>,
    /// Its flow: what comes in and goes out, l/s.
    pub flow: f64,
    /// What the dry rooms ask, and the wet ones, l/s.
    pub dry_flow: f64,
    pub wet_flow: f64,
    /// Kitchens: each needs its hood's extraction.
    pub kitchens: usize,
}

impl Dwelling {
    pub fn fails(&self) -> usize {
        self.rooms
            .iter()
            .map(|r| r.inlet_short() as usize + r.vent_short() as usize)
            .sum::<usize>()
            + self.passages.iter().filter(|p| p.short()).count()
    }
}

/// A stretch of the extract network.
#[derive(Debug, Clone, PartialEq)]
pub struct Stretch {
    /// «V1», «V2»…
    pub name: String,
    pub run: Option<RunId>,
    pub shaft: Option<DeviceId>,
    pub level: LevelId,
    /// The stretch it flows into, by index; `None` the one at the fan.
    pub parent: Option<usize>,
    pub length: f64,
    pub flow: f64,
    /// Valves after it.
    pub vents: usize,
    /// Kitchen hoods after it.
    pub hoods: usize,
    /// Its section as drawn, cm², and its size («200 × 150», «Ø200»).
    pub area: f64,
    pub size: String,
    /// What it needs, cm², and the size that gives it.
    pub needed: f64,
    pub sized: (f64, f64),
    pub sized_text: String,
    /// Hybrid: the ducts of table 4.2 it needs (more than one: the size
    /// is not enlarged by itself, ducts are to be added).
    pub ducts: Vec<f64>,
    /// On the roof (mechanical: 1,5·q, formula 4.2).
    pub on_roof: bool,
}

impl Stretch {
    pub fn short(&self) -> bool {
        self.area + 0.5 < self.needed
    }
}

/// Where an extract duct comes out (3.2.1).
#[derive(Debug, Clone, PartialEq)]
pub struct Outlet {
    pub level: LevelId,
    pub at: Point2,
    /// Absolute height of the mouth, metres.
    pub z: f64,
    pub fan: Option<DeviceId>,
    /// The nearest air coming in (a window, an outside door, an air inlet,
    /// a terrace) within 10 m, and how far, metres.
    pub intake: Option<(String, f64)>,
    /// Hybrid: the height rules apply.
    pub hybrid: bool,
    /// Its height above the roof it stands on, metres.
    pub above_roof: f64,
    /// The roof is walked on (rooms or a railing on it).
    pub walked: bool,
    /// The obstacle that asks the most (name, distance, height above the
    /// roof), and what the outlet must reach above the roof.
    pub obstacle: Option<(String, f64, f64)>,
    pub required: f64,
}

impl Outlet {
    pub fn too_close(&self) -> bool {
        self.intake
            .as_ref()
            .is_some_and(|(_, d)| *d + 1e-6 < OUTLET_GAP)
    }

    pub fn too_low(&self) -> bool {
        self.hybrid && self.above_roof + 1e-6 < self.required
    }

    pub fn fails(&self) -> bool {
        self.too_close() || self.too_low()
    }
}

/// An extract network: from the valves (or the kitchen hoods) to its fan
/// or its outlet.
#[derive(Debug, Clone, PartialEq)]
pub struct Network {
    pub stretches: Vec<Stretch>,
    pub fan: Option<DeviceId>,
    pub vents: Vec<DeviceId>,
    /// The stretch each valve hangs on, by index (as `vents`).
    pub vent_at: Vec<Option<usize>>,
    /// The kitchen hoods on it, and the stretch of each.
    pub hoods: Vec<ObjectId>,
    pub hood_at: Vec<Option<usize>>,
    pub flow: f64,
    /// Hybrid (dwellings' valves only, with [`VentSystem::Hybrid`]): the
    /// floors venting into it and its draught class (table 4.3).
    pub hybrid: Option<(usize, u8)>,
    /// Garage valves on it, and whether it serves anything else.
    pub garage_vents: usize,
    pub outlet: Option<Outlet>,
}

impl Network {
    /// A hoods' duct: only hoods on it (no fan needed, each hood has its
    /// own).
    pub fn is_hood_duct(&self) -> bool {
        !self.hoods.is_empty() && self.vents.is_empty()
    }

    /// Hoods and valves on one duct: the hoods' extraction must be
    /// independent (3.1.1).
    pub fn shared(&self) -> bool {
        !self.hoods.is_empty() && !self.vents.is_empty()
    }

    /// A garage's network that serves something else too (3.1.4.2: only
    /// the garage's).
    pub fn garage_shared(&self) -> bool {
        self.garage_vents > 0 && (self.garage_vents < self.vents.len() || !self.hoods.is_empty())
    }
}

/// A kitchen's own extraction for its hood (3.1.1): 50 l/s through a
/// duct of its own, not shared with the dwelling's ventilation.
#[derive(Debug, Clone, PartialEq)]
pub struct Hood {
    pub kitchen: RoomId,
    pub kitchen_name: String,
    pub dwelling: String,
    pub level: LevelId,
    /// The hood drawn in the kitchen (an object of the family «Campana
    /// extractora»).
    pub object: Option<ObjectId>,
    /// Its duct's network, by index in `networks`.
    pub network: Option<usize>,
}

/// A room of the building with its own flow (table 2.2).
#[derive(Debug, Clone, PartialEq)]
pub struct Other {
    pub room: RoomId,
    pub name: String,
    pub level: LevelId,
    pub area: f64,
    pub flow: f64,
    pub what: &'static str,
    pub garage: bool,
    /// A garage's spaces, and whether they are the cars drawn (or one
    /// every 25 m²).
    pub spaces: usize,
    pub cars_drawn: bool,
    pub inlets: Vec<DeviceId>,
    pub inlet_area: f64,
    pub vents: Vec<DeviceId>,
    pub vent_area: f64,
    /// A garage: the extract networks of its valves, and the farthest any
    /// valve is from the next one, metres.
    pub networks: usize,
    pub vent_gap: f64,
}

impl Other {
    pub fn drawn(&self) -> bool {
        !self.inlets.is_empty() || !self.vents.is_empty()
    }

    pub fn vent_needed(&self) -> f64 {
        OPENING_PER_LS * self.flow
    }

    pub fn inlet_needed(&self) -> f64 {
        OPENING_PER_LS * self.flow
    }

    /// Its valves give less than table 4.1 (when drawn).
    pub fn vent_short(&self) -> bool {
        !self.vents.is_empty() && self.vent_area + 0.5 < self.vent_needed()
    }

    pub fn inlet_short(&self) -> bool {
        !self.inlets.is_empty() && self.inlet_area + 0.5 < self.inlet_needed()
    }

    /// A garage: the openings of each kind it needs (one every 100 m²).
    pub fn openings_needed(&self) -> usize {
        ((self.area / GARAGE_OPENING_AREA) - 1e-9).ceil().max(1.0) as usize
    }

    /// A garage that fails 3.1.4: too few extraction openings, too far
    /// apart, or one network where two are needed.
    pub fn garage_fails(&self) -> usize {
        // Without valves drawn it may be ventilated naturally, by mixed
        // openings on two opposite façades: not checked.
        if !self.garage || self.vents.is_empty() {
            return 0;
        }
        (self.vents.len() < self.openings_needed()) as usize
            + (self.vent_gap > GARAGE_VENT_SPACING + 1e-6) as usize
            + (self.spaces >= GARAGE_TWO_NETWORKS && self.networks < 2) as usize
    }

    /// A garage that must detect carbon monoxide.
    pub fn needs_co(&self) -> bool {
        self.garage && (self.spaces > GARAGE_CO_SPACES || self.area > GARAGE_OPENING_AREA)
    }
}

/// The whole calculation.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Ventilation {
    /// Whether the building is a dwellings' one (the HS 3 applies).
    pub applies: bool,
    pub dwellings: Vec<Dwelling>,
    pub networks: Vec<Network>,
    pub others: Vec<Other>,
    /// Valves far from every duct.
    pub loose: Vec<DeviceId>,
    /// Wet rooms of no dwelling.
    pub lone_wet: Vec<RoomId>,
    /// Every kitchen of a dwelling and its hood.
    pub hoods: Vec<Hood>,
}

impl Ventilation {
    /// The ducts that are short and the width that is enough (their
    /// height kept): only to enlarge.
    pub fn sized_runs(&self, model: &Model) -> Vec<(RunId, f64)> {
        let mut out: BTreeMap<RunId, f64> = BTreeMap::new();
        for s in self.networks.iter().flat_map(|n| &n.stretches) {
            if let (Some(r), true) = (s.run, s.short() && s.ducts.len() <= 1) {
                if model.run(r).is_some() {
                    let e = out.entry(r).or_insert(0.0);
                    *e = e.max(s.sized.0);
                }
            }
        }
        out.into_iter().collect()
    }

    /// The devices that are short (inlets, valves and shafts) and the
    /// width and depth that are enough.
    pub fn sized_devices(&self, model: &Model) -> Vec<(DeviceId, f64, f64)> {
        let mut out: BTreeMap<DeviceId, (f64, f64)> = BTreeMap::new();
        for s in self.networks.iter().flat_map(|n| &n.stretches) {
            if let (Some(d), true) = (s.shaft, s.short() && s.ducts.len() <= 1) {
                let e = out.entry(d).or_insert((0.0, 0.0));
                e.0 = e.0.max(s.sized.0);
                e.1 = e.1.max(s.sized.1);
            }
        }
        for r in self.dwellings.iter().flat_map(|d| &d.rooms) {
            if r.inlet_short() {
                // The missing area shared among its inlets, longer.
                let n = r.inlets.len().max(1) as f64;
                for id in &r.inlets {
                    if let Some(d) = model.device(*id) {
                        let slot = if d.height > 0.0 { d.height } else { 0.02 };
                        let w = step_up(r.inlet_needed() / n / (slot * 1e4), 0.05).max(d.width);
                        out.insert(*id, (w, d.depth));
                    }
                }
            }
            if r.vent_short() {
                let n = r.vents.len().max(1) as f64;
                for id in &r.vents {
                    let dia = vent_size(r.vent_needed() / n);
                    out.insert(*id, (dia, dia));
                }
            }
        }
        for o in &self.others {
            if o.vent_short() {
                let n = o.vents.len().max(1) as f64;
                for id in &o.vents {
                    let dia = vent_size(o.vent_needed() / n);
                    out.insert(*id, (dia, dia));
                }
            }
        }
        out.into_iter()
            .filter(|(id, (w, d))| {
                model
                    .device(*id)
                    .is_some_and(|x| *w > x.width + 1e-6 || *d > x.depth + 1e-6)
            })
            .map(|(id, (w, d))| (id, w, d))
            .collect()
    }

    pub fn fails(&self) -> usize {
        self.dwellings.iter().map(|d| d.fails()).sum::<usize>()
            + self
                .networks
                .iter()
                .flat_map(|n| &n.stretches)
                .filter(|s| s.short())
                .count()
            + self.hoods.iter().filter(|h| self.hood_fails(h)).count()
            + self
                .networks
                .iter()
                .filter(|n| n.garage_shared() || n.outlet.as_ref().is_some_and(|o| o.fails()))
                .count()
            + self
                .others
                .iter()
                .map(|o| o.vent_short() as usize + o.garage_fails())
                .sum::<usize>()
    }

    /// A hood drawn without its duct, or on a duct with valves.
    pub fn hood_fails(&self, h: &Hood) -> bool {
        h.object.is_some()
            && match h.network.and_then(|i| self.networks.get(i)) {
                None => true,
                Some(n) => n.shared(),
            }
    }
}

fn step_up(v: f64, step: f64) -> f64 {
    ((v / step) - 1e-9).ceil().max(1.0) * step
}

/// The diameter of a valve that gives this area, metres.
pub fn vent_size(area: f64) -> f64 {
    VENT_SIZES
        .into_iter()
        .find(|d| round_area(*d) >= area - 0.5)
        .unwrap_or(VENT_SIZES[VENT_SIZES.len() - 1])
}

/// A round section's area, cm², for a diameter in metres.
pub fn round_area(d: f64) -> f64 {
    std::f64::consts::PI / 4.0 * d * d * 1e4
}

/// A device's free area, cm²: an inlet's face; a valve's or a shaft's
/// section (round when its width and depth are the same).
pub fn device_area(kind: DeviceKind, w: f64, d: f64, h: f64) -> f64 {
    match kind {
        DeviceKind::AirInlet => w * h * 1e4,
        _ if (w - d).abs() < 1e-6 => round_area(w),
        _ => w * d * 1e4,
    }
}

fn mm(v: f64) -> i64 {
    (v * 1000.0).round() as i64
}

/// A door between two rooms of a level.
struct Door {
    id: OpeningId,
    rooms: [RoomId; 2],
    /// Clear width; `None`, without a door.
    leaf: Option<f64>,
    area: f64,
}

fn doors(model: &Model) -> Vec<(LevelId, Door)> {
    let mut out = Vec::new();
    for o in model.openings() {
        let Some(t) = model.opening_type(o.opening_type) else {
            continue;
        };
        if o.sill > 0.05 || matches!(t.kind, OpeningKind::Window(_)) {
            continue;
        }
        let (Some(g), Some(wall)) = (
            archisim_geom::opening_geometry(model, o.id),
            model.wall(o.wall),
        ) else {
            continue;
        };
        let tm = (g.t0 + g.t1) / 2.0;
        let sign = if g.exterior_offset >= g.interior_offset {
            1.0
        } else {
            -1.0
        };
        let probe = [
            g.point(tm, g.exterior_offset + sign * 0.25),
            g.point(tm, g.interior_offset - sign * 0.25),
        ];
        let room_at = |p: Point2| {
            model
                .rooms_on_level(wall.level)
                .find(|r| r.contains(p))
                .map(|r| r.id)
        };
        let (Some(a), Some(b)) = (room_at(probe[0]), room_at(probe[1])) else {
            continue;
        };
        if a == b {
            continue;
        }
        let clear = model
            .opening_clear_width(t.id)
            .unwrap_or(t.width - 2.0 * t.frame_width);
        let (leaf, area) = match t.kind {
            OpeningKind::Void => (None, t.width * t.height * 1e4),
            _ => (Some(clear), clear * UNDERCUT * 1e4),
        };
        out.push((
            wall.level,
            Door {
                id: o.id,
                rooms: [a, b],
                leaf,
                area,
            },
        ));
    }
    out
}

/// The room of a level a device is in, or the nearest within reach (an
/// inlet in its wall).
fn room_of(model: &Model, level: LevelId, p: Point2, rooms: &BTreeSet<RoomId>) -> Option<RoomId> {
    let mut best: Option<(RoomId, f64)> = None;
    for r in model
        .rooms_on_level(level)
        .filter(|r| rooms.contains(&r.id))
    {
        if r.contains(p) {
            return Some(r.id);
        }
        let n = r.outline.len();
        for i in 0..n {
            let (a, b) = (r.outline[i], r.outline[(i + 1) % n]);
            let ab = b - a;
            let l2 = ab.dot(ab);
            if l2 < 1e-12 {
                continue;
            }
            let t = ((p - a).dot(ab) / l2).clamp(0.0, 1.0);
            let d = (a + ab * t).distance(p);
            if d <= ROOM_REACH && best.is_none_or(|(_, bd)| d < bd) {
                best = Some((r.id, d));
            }
        }
    }
    best.map(|(r, _)| r)
}

/// The dwellings' name: the last word all their rooms' names share, if
/// short («Dormitorio E», «Baño E» → «Vivienda E»).
fn dwelling_name(names: &[&str], i: usize, level: &str) -> String {
    let last: BTreeSet<&str> = names
        .iter()
        .filter_map(|n| n.split_whitespace().last())
        .collect();
    match (last.len(), last.iter().next()) {
        (1, Some(w)) if names.len() > 1 && w.chars().count() <= 3 => {
            format!("Vivienda {w} ({level})")
        }
        _ => format!("Vivienda {} ({level})", i + 1),
    }
}

/// Works out the ventilation of the dwellings.
pub fn ventilation(model: &Model) -> Ventilation {
    let mut v = Ventilation {
        applies: model.code_settings().building_use == BuildingUse::Dwelling,
        ..Default::default()
    };
    if !v.applies {
        return v;
    }
    let all_doors = doors(model);
    let devices: Vec<_> = model.devices().collect();
    for level in model.levels() {
        // Dwelling rooms and the others.
        let mut ours: BTreeMap<RoomId, Class> = BTreeMap::new();
        for r in model.rooms_on_level(level.id) {
            let c = Class::of(&r.usage, &r.name);
            if c.of_dwelling() {
                ours.insert(r.id, c);
            } else if c.store || c.refuse || c.garage {
                let area = r.area();
                let cars = model
                    .objects()
                    .filter(|o| o.level == level.id && r.contains(o.position))
                    .filter(|o| is_car(model, o.id))
                    .count();
                let spaces = if cars > 0 {
                    cars
                } else {
                    (area / GARAGE_SPACE_AREA).round().max(1.0) as usize
                };
                let (flow, what) = if c.garage {
                    (GARAGE_PER_SPACE * spaces as f64, "aparcamiento")
                } else if c.refuse {
                    (REFUSE_PER_M2 * area, "almacén de residuos")
                } else {
                    (STORE_PER_M2 * area, "trastero")
                };
                let one: BTreeSet<RoomId> = BTreeSet::from([r.id]);
                let (mut inlets, mut inlet_area, mut vents, mut vent_area) =
                    (Vec::new(), 0.0, Vec::new(), 0.0);
                for d in devices.iter().filter(|d| d.level == level.id) {
                    if !matches!(d.kind, DeviceKind::AirInlet | DeviceKind::ExhaustVent)
                        || room_of(model, level.id, d.position, &one).is_none()
                    {
                        continue;
                    }
                    let a = device_area(d.kind, d.width, d.depth, d.height);
                    if d.kind == DeviceKind::AirInlet {
                        inlets.push(d.id);
                        inlet_area += a;
                    } else {
                        vents.push(d.id);
                        vent_area += a;
                    }
                }
                // How far a valve is from the nearest other one: the
                // largest of these.
                let at: Vec<Point2> = vents
                    .iter()
                    .filter_map(|id| model.device(*id))
                    .map(|d| d.position)
                    .collect();
                let vent_gap = if at.len() < 2 {
                    0.0
                } else {
                    at.iter()
                        .map(|p| {
                            at.iter()
                                .filter(|q| q.distance(*p) > 1e-9)
                                .map(|q| q.distance(*p))
                                .fold(f64::MAX, f64::min)
                        })
                        .fold(0.0, f64::max)
                };
                v.others.push(Other {
                    room: r.id,
                    name: r.name.clone(),
                    level: level.id,
                    area,
                    flow,
                    what,
                    garage: c.garage,
                    spaces: if c.garage { spaces } else { 0 },
                    cars_drawn: c.garage && cars > 0,
                    inlets,
                    inlet_area,
                    vents,
                    vent_area,
                    networks: 0,
                    vent_gap,
                });
            }
        }
        // Joined by doors.
        let mut parent: BTreeMap<RoomId, RoomId> = ours.keys().map(|r| (*r, *r)).collect();
        fn find(p: &mut BTreeMap<RoomId, RoomId>, r: RoomId) -> RoomId {
            let up = p[&r];
            if up == r {
                return r;
            }
            let top = find(p, up);
            p.insert(r, top);
            top
        }
        let level_doors: Vec<&Door> = all_doors
            .iter()
            .filter(|(l, d)| {
                *l == level.id && ours.contains_key(&d.rooms[0]) && ours.contains_key(&d.rooms[1])
            })
            .map(|(_, d)| d)
            .collect();
        for d in &level_doors {
            let (a, b) = (find(&mut parent, d.rooms[0]), find(&mut parent, d.rooms[1]));
            if a != b {
                parent.insert(a, b);
            }
        }
        let mut groups: BTreeMap<RoomId, Vec<RoomId>> = BTreeMap::new();
        for r in ours.keys().copied().collect::<Vec<_>>() {
            let top = find(&mut parent, r);
            groups.entry(top).or_default().push(r);
        }
        // In order across the plan, for their numbers.
        let mut groups: Vec<Vec<RoomId>> = groups.into_values().collect();
        let at = |rs: &[RoomId]| {
            rs.iter()
                .filter_map(|r| model.room(*r))
                .map(|r| (r.label.x, r.label.y))
                .fold((f64::MAX, f64::MAX), |a, b| (a.0.min(b.0), a.1.min(b.1)))
        };
        groups.sort_by(|a, b| {
            let (pa, pb) = (at(a), at(b));
            pa.0.total_cmp(&pb.0).then(pa.1.total_cmp(&pb.1))
        });
        let mut n = 0;
        for g in groups {
            if !g.iter().any(|r| ours[r].dry()) {
                v.lone_wet
                    .extend(g.iter().filter(|r| ours[*r].wet).copied());
                continue;
            }
            let rooms: BTreeSet<RoomId> = g.iter().copied().collect();
            let names: Vec<&str> = g
                .iter()
                .filter_map(|r| model.room(*r))
                .map(|r| r.name.as_str())
                .collect();
            let name = dwelling_name(&names, n, &level.name);
            n += 1;
            // A study or office counts as a bedroom (HS 3, table 2.1),
            // also for the kind of dwelling.
            let sleeps = |r: &RoomId| ours[r].bedroom || ours[r].study;
            let bedrooms = g.iter().filter(|r| sleeps(r)).count();
            let f = flows(bedrooms);
            let area = |r: &RoomId| model.room(*r).map_or(0.0, |x| x.area());
            // The main bedroom is the largest bedroom, or the largest
            // study if there is no bedroom.
            let main = g
                .iter()
                .filter(|r| ours[*r].bedroom)
                .max_by(|a, b| area(a).total_cmp(&area(b)))
                .or_else(|| {
                    g.iter()
                        .filter(|r| sleeps(r))
                        .max_by(|a, b| area(a).total_cmp(&area(b)))
                })
                .copied();
            let mut vr: Vec<VentRoom> = g
                .iter()
                .filter_map(|r| model.room(*r))
                .map(|r| {
                    let c = ours[&r.id];
                    let dry = if Some(r.id) == main {
                        Some(Dry::MainBedroom)
                    } else if c.bedroom {
                        Some(Dry::Bedroom)
                    } else if c.living {
                        Some(Dry::Living)
                    } else if c.study {
                        Some(Dry::Study)
                    } else {
                        None
                    };
                    let admission = match dry {
                        Some(Dry::MainBedroom) => f.main_bedroom,
                        Some(Dry::Bedroom | Dry::Study) => f.bedroom,
                        Some(Dry::Living) => f.living,
                        None => 0.0,
                    };
                    VentRoom {
                        room: r.id,
                        name: r.name.clone(),
                        area: r.area(),
                        dry,
                        wet: c.wet,
                        kitchen: c.kitchen,
                        admission,
                        extraction: if c.wet { f.wet_room } else { 0.0 },
                        inlets: Vec::new(),
                        inlet_area: 0.0,
                        vents: Vec::new(),
                        vent_area: 0.0,
                    }
                })
                .collect();
            // What comes in goes out: the larger side rules.
            let dry_flow: f64 = vr.iter().map(|r| r.admission).sum();
            let wet_rooms = vr.iter().filter(|r| r.wet).count();
            let wet_sum: f64 = vr.iter().map(|r| r.extraction).sum();
            let wet_flow = if wet_rooms > 0 {
                wet_sum.max(f.wet_total)
            } else {
                0.0
            };
            let flow = dry_flow.max(wet_flow);
            if wet_rooms > 0 && wet_sum > 0.0 {
                let k = flow / wet_sum;
                for r in vr.iter_mut().filter(|r| r.wet) {
                    r.extraction *= k;
                }
            }
            if dry_flow > 0.0 && dry_flow < flow - 1e-9 && wet_rooms > 0 {
                let k = flow / dry_flow;
                for r in vr.iter_mut().filter(|r| r.dry.is_some()) {
                    r.admission *= k;
                }
            }
            // The inlets and valves drawn.
            for d in devices.iter().filter(|d| d.level == level.id) {
                if !matches!(d.kind, DeviceKind::AirInlet | DeviceKind::ExhaustVent) {
                    continue;
                }
                let Some(r) = room_of(model, level.id, d.position, &rooms) else {
                    continue;
                };
                let Some(x) = vr.iter_mut().find(|x| x.room == r) else {
                    continue;
                };
                let area = device_area(d.kind, d.width, d.depth, d.height);
                if d.kind == DeviceKind::AirInlet {
                    x.inlets.push(d.id);
                    x.inlet_area += area;
                } else {
                    x.vents.push(d.id);
                    x.vent_area += area;
                }
            }
            let passages = route_air(&vr, &level_doors);
            v.dwellings.push(Dwelling {
                name,
                level: level.id,
                bedrooms,
                kitchens: vr.iter().filter(|r| r.kitchen).count(),
                rooms: vr,
                passages,
                flow,
                dry_flow,
                wet_flow,
            });
        }
    }
    // The extract ducts, with the flow of each valve.
    let mut vent_flow: BTreeMap<DeviceId, f64> = BTreeMap::new();
    for r in v.dwellings.iter().flat_map(|d| &d.rooms) {
        let n = r.vents.len().max(1) as f64;
        for id in &r.vents {
            vent_flow.insert(*id, r.extraction / n);
        }
    }
    let mut garage_vents: BTreeSet<DeviceId> = BTreeSet::new();
    for o in &v.others {
        let n = o.vents.len().max(1) as f64;
        for id in &o.vents {
            vent_flow.insert(*id, o.flow / n);
            if o.garage {
                garage_vents.insert(*id);
            }
        }
    }
    // The dwellings' valves, for the hybrid ducts, and the highest floor
    // with dwellings: above it, the roof.
    let dwelling_vents: BTreeSet<DeviceId> = v
        .dwellings
        .iter()
        .flat_map(|d| &d.rooms)
        .flat_map(|r| r.vents.iter().copied())
        .collect();
    let top_dwelling = v
        .dwellings
        .iter()
        .filter_map(|d| model.level(d.level))
        .map(|l| l.elevation)
        .fold(f64::NEG_INFINITY, f64::max);
    // The kitchens' hoods.
    let mut hoods: Vec<(ObjectId, LevelId, Point2)> = Vec::new();
    for d in &v.dwellings {
        for r in d.rooms.iter().filter(|r| r.kitchen) {
            let Some(room) = model.room(r.room) else {
                continue;
            };
            let object = model
                .objects()
                .filter(|o| {
                    // A hood stands against the wall: on the room's edge.
                    let inward = (room.label - o.position).normalized().unwrap_or_default();
                    o.level == d.level
                        && (room.contains(o.position) || room.contains(o.position + inward * 0.3))
                })
                .find(|o| is_hood(model, o.id))
                .map(|o| o.id);
            if let Some(id) = object {
                if let Some(o) = model.objects().find(|o| o.id == id) {
                    hoods.push((id, o.level, o.position));
                }
            }
            v.hoods.push(Hood {
                kitchen: r.room,
                kitchen_name: r.name.clone(),
                dwelling: d.name.clone(),
                level: d.level,
                object,
                network: None,
            });
        }
    }
    let (mut networks, loose) = extract_networks(
        model,
        &Sources {
            vent_flow: &vent_flow,
            hoods: &hoods,
            dwelling_vents: &dwelling_vents,
            garage_vents: &garage_vents,
            top_dwelling,
        },
    );
    for o in v.others.iter_mut().filter(|o| o.garage) {
        o.networks = networks
            .iter()
            .filter(|n| n.vents.iter().any(|d| o.vents.contains(d)))
            .count();
    }
    // Named across the building: «V1»… the general ducts, «VC1»… the
    // hoods' ones.
    let (mut general, mut hood) = (0, 0);
    for n in &mut networks {
        let hoods_only = n.is_hood_duct();
        for st in &mut n.stretches {
            st.name = if hoods_only {
                hood += 1;
                format!("VC{hood}")
            } else {
                general += 1;
                format!("V{general}")
            };
        }
    }
    for h in &mut v.hoods {
        h.network = h
            .object
            .and_then(|o| networks.iter().position(|n| n.hoods.contains(&o)));
    }
    v.networks = networks;
    v.loose = loose;
    v
}

/// Whether an object is a kitchen hood: of the family that makes hoods
/// (`props:hood`) or named so.
pub fn is_hood(model: &Model, id: ObjectId) -> bool {
    let Some(o) = model.objects().find(|o| o.id == id) else {
        return false;
    };
    let Some(t) = model.object_type(o.object_type) else {
        return false;
    };
    model.family_prop(t.family).as_deref() == Some("hood")
        || t.name.to_lowercase().contains("campana")
}

/// Whether an object is a car (of the family that makes cars, `props:car`,
/// or named so): a garage's space.
pub fn is_car(model: &Model, id: ObjectId) -> bool {
    let Some(o) = model.objects().find(|o| o.id == id) else {
        return false;
    };
    let Some(t) = model.object_type(o.object_type) else {
        return false;
    };
    model.family_prop(t.family).as_deref() == Some("car") || {
        let n = t.name.to_lowercase();
        n.contains("coche") || n.contains("turismo")
    }
}

/// How near a hood must be to its duct, metres (a hood is up to 1,2 m
/// wide; its duct leaves from its middle).
pub const HOOD_REACH: f64 = 0.6;

/// The air's way through the dwelling: what each dry room takes in goes,
/// door by door, to the nearest wet rooms that still lack it.
fn route_air(rooms: &[VentRoom], doors: &[&Door]) -> Vec<Passage> {
    let index: BTreeMap<RoomId, usize> =
        rooms.iter().enumerate().map(|(i, r)| (r.room, i)).collect();
    let mut adj: Vec<Vec<(usize, usize)>> = vec![Vec::new(); rooms.len()];
    for (k, d) in doors.iter().enumerate() {
        if let (Some(&a), Some(&b)) = (index.get(&d.rooms[0]), index.get(&d.rooms[1])) {
            adj[a].push((b, k));
            adj[b].push((a, k));
        }
    }
    // Each room's own balance: a living-kitchen sends its air to itself.
    let mut surplus: Vec<f64> = rooms.iter().map(|r| r.admission - r.extraction).collect();
    // Door k: flow from rooms[0] to rooms[1] (negative, the other way).
    let mut through: BTreeMap<usize, f64> = BTreeMap::new();
    for s in 0..rooms.len() {
        let mut guard = 0;
        while surplus[s] > 1e-6 && guard < rooms.len() {
            guard += 1;
            // Nearest room that lacks air.
            let mut prev: Vec<Option<(usize, usize)>> = vec![None; rooms.len()];
            let mut seen = vec![false; rooms.len()];
            seen[s] = true;
            let mut q = VecDeque::from([s]);
            let mut found = None;
            while let Some(i) = q.pop_front() {
                if i != s && surplus[i] < -1e-6 {
                    found = Some(i);
                    break;
                }
                for &(j, k) in &adj[i] {
                    if !seen[j] {
                        seen[j] = true;
                        prev[j] = Some((i, k));
                        q.push_back(j);
                    }
                }
            }
            let Some(t) = found else {
                break;
            };
            let f = surplus[s].min(-surplus[t]);
            surplus[s] -= f;
            surplus[t] += f;
            let mut at = t;
            while let Some((from, k)) = prev[at] {
                let forward = index.get(&doors[k].rooms[0]) == Some(&from);
                *through.entry(k).or_insert(0.0) += if forward { f } else { -f };
                at = from;
            }
        }
    }
    through
        .into_iter()
        .filter(|(_, f)| f.abs() > 1e-6)
        .map(|(k, f)| {
            let d = doors[k];
            let (from, to) = if f > 0.0 {
                (d.rooms[0], d.rooms[1])
            } else {
                (d.rooms[1], d.rooms[0])
            };
            Passage {
                opening: d.id,
                from,
                to,
                flow: f.abs(),
                door: d.leaf,
                area: d.area,
            }
        })
        .collect()
}

/// What the extract networks are worked out from.
struct Sources<'a> {
    vent_flow: &'a BTreeMap<DeviceId, f64>,
    hoods: &'a [(ObjectId, LevelId, Point2)],
    dwelling_vents: &'a BTreeSet<DeviceId>,
    garage_vents: &'a BTreeSet<DeviceId>,
    /// The highest floor with dwellings (above it, the roof).
    top_dwelling: f64,
}

/// The extract networks: each part of the ducts with valves, from them
/// to its fan.
fn extract_networks(model: &Model, src: &Sources) -> (Vec<Network>, Vec<DeviceId>) {
    let (vent_flow, hoods) = (src.vent_flow, src.hoods);
    let site = *model.ventilation_site();
    let g = graph(model, ServiceSystem::ExhaustAir);
    let mut at_node: BTreeMap<usize, Vec<DeviceId>> = BTreeMap::new();
    let mut loose = Vec::new();
    for d in model
        .devices()
        .filter(|d| d.kind == DeviceKind::ExhaustVent)
    {
        match g.nearest(d.level, d.position, JOIN_REACH) {
            Some(n) => at_node.entry(n).or_default().push(d.id),
            None => loose.push(d.id),
        }
    }
    let mut at_hood: BTreeMap<usize, Vec<ObjectId>> = BTreeMap::new();
    for (id, level, p) in hoods {
        if let Some(n) = g.nearest(*level, *p, HOOD_REACH) {
            at_hood.entry(n).or_default().push(*id);
        }
    }
    let starts: BTreeSet<usize> = at_node.keys().chain(at_hood.keys()).copied().collect();
    let fans: Vec<_> = model
        .devices()
        .filter(|d| d.kind == DeviceKind::Fan)
        .filter_map(|d| g.nearest(d.level, d.position, 1.0).map(|n| (n, d.id)))
        .collect();
    let mut done = vec![false; g.nodes.len()];
    let mut out = Vec::new();
    for &start in &starts {
        if done[start] {
            continue;
        }
        // The part of the graph it is in.
        let mut part = Vec::new();
        let mut q = VecDeque::from([start]);
        done[start] = true;
        while let Some(i) = q.pop_front() {
            part.push(i);
            for &e in &g.adj[i] {
                let ed = g.edges[e];
                let j = if ed.a == i { ed.b } else { ed.a };
                if !done[j] {
                    done[j] = true;
                    q.push_back(j);
                }
            }
        }
        // A fan only for the valves: a hoods' duct goes up to its outlet.
        let with_vents = part.iter().any(|n| at_node.contains_key(n));
        let fan = fans
            .iter()
            .find(|(n, _)| with_vents && part.contains(n))
            .copied();
        // Without a fan, the mouth is the highest free end that is not a
        // valve's or a hood's (or the highest point).
        let root = fan.map(|(n, _)| n).unwrap_or_else(|| {
            let free: Vec<usize> = part
                .iter()
                .copied()
                .filter(|n| g.adj[*n].len() == 1 && !starts.contains(n))
                .collect();
            let pool = if free.is_empty() { &part } else { &free };
            *pool
                .iter()
                .max_by(|a, b| g.nodes[**a].z.total_cmp(&g.nodes[**b].z))
                .unwrap_or(&start)
        });
        let part_vents: Vec<DeviceId> = part
            .iter()
            .filter_map(|n| at_node.get(n))
            .flatten()
            .copied()
            .collect();
        let garage_vents = part_vents
            .iter()
            .filter(|d| src.garage_vents.contains(d))
            .count();
        // Hybrid: only the dwellings' valves on it (a garage's and a
        // hood's extraction are mechanical).
        let hybrid = (site.system == VentSystem::Hybrid
            && !part_vents.is_empty()
            && part_vents.iter().all(|d| src.dwelling_vents.contains(d))
            && !part.iter().any(|n| at_hood.contains_key(n)))
        .then(|| {
            let floors = venting_floors(model, &part_vents);
            (floors, draft_class(floors, site.zone))
        });
        // A tree from the fan.
        let mut up: BTreeMap<usize, usize> = BTreeMap::new(); // node → edge to its parent
        let mut order = vec![root];
        let mut seen: BTreeSet<usize> = BTreeSet::from([root]);
        let mut k = 0;
        while k < order.len() {
            let i = order[k];
            k += 1;
            for &e in &g.adj[i] {
                let ed = g.edges[e];
                let j = if ed.a == i { ed.b } else { ed.a };
                if seen.insert(j) {
                    up.insert(j, e);
                    order.push(j);
                }
            }
        }
        let other = |e: usize, i: usize| {
            let ed = g.edges[e];
            if ed.a == i {
                ed.b
            } else {
                ed.a
            }
        };
        // Flow, valves and hoods after each node.
        let mut flow: BTreeMap<usize, (f64, usize, usize)> = BTreeMap::new();
        for &i in order.iter().rev() {
            let mut own: (f64, usize, usize) = at_node.get(&i).map_or((0.0, 0, 0), |vs| {
                (
                    vs.iter()
                        .map(|v| vent_flow.get(v).copied().unwrap_or(0.0))
                        .sum(),
                    vs.len(),
                    0,
                )
            });
            if let Some(hs) = at_hood.get(&i) {
                own.0 += HOOD_FLOW * hs.len() as f64;
                own.2 += hs.len();
            }
            let e = flow.entry(i).or_insert((0.0, 0, 0));
            e.0 += own.0;
            e.1 += own.1;
            e.2 += own.2;
            let mine = *e;
            if let Some(&pe) = up.get(&i) {
                let p = other(pe, i);
                let pf = flow.entry(p).or_insert((0.0, 0, 0));
                pf.0 += mine.0;
                pf.1 += mine.1;
                pf.2 += mine.2;
            }
        }
        let children = |i: usize| up.iter().filter(move |(c, e)| other(**e, **c) == i).count();
        // Stretches: chains between branches, of one run or shaft.
        let key = |e: usize| (g.edges[e].run, g.edges[e].riser);
        let mut of_edge: BTreeMap<usize, usize> = BTreeMap::new();
        let mut stretches: Vec<Stretch> = Vec::new();
        for &c in &order[1..] {
            let e = up[&c];
            let p = other(e, c);
            let carry = (flow.get(&c).copied().unwrap_or((0.0, 0, 0))).0 > 1e-9;
            if !carry {
                continue;
            }
            let goes_on = up.get(&p).filter(|pe| {
                children(p) == 1
                    && !at_node.contains_key(&p)
                    && !at_hood.contains_key(&p)
                    && key(**pe) == key(e)
            });
            let length = g.edges[e].length;
            if let Some(s) = goes_on.and_then(|pe| of_edge.get(pe)).copied() {
                stretches[s].length += length;
                of_edge.insert(e, s);
                continue;
            }
            let (fl, nv, nh) = flow[&c];
            let parent = up.get(&p).and_then(|pe| of_edge.get(pe)).copied();
            let (run, shaft) = key(e);
            let level = if g.nodes[c].z <= g.nodes[p].z {
                g.nodes[c].level
            } else {
                g.nodes[p].level
            };
            let on_roof = model
                .level(level)
                .is_some_and(|l| l.elevation > src.top_dwelling + 1e-6);
            let ducts: Vec<f64> = match hybrid {
                Some((_, class)) => hybrid_ducts(fl, class).map_or_else(Vec::new, |d| d.to_vec()),
                None => Vec::new(),
            };
            let needed = match hybrid {
                Some(_) if !ducts.is_empty() => ducts.iter().sum(),
                // Above the table: as many 900 cm² ducts as 1 000 l/s.
                Some(_) => 900.0 * (fl / 1000.0).ceil(),
                None if on_roof => ROOF_DUCT_PER_LS * fl,
                None => DUCT_PER_LS * fl,
            };
            let (area, size, sized, sized_text) = if let Some(r) = run.and_then(|r| model.run(r)) {
                let area = r.width * r.height * 1e4;
                let h = if r.height > 0.0 { r.height } else { 0.1 };
                let w = step_up(needed / (h * 1e4), 0.05).max(r.width).max(0.1);
                (
                    area,
                    format!("{} × {}", mm(r.width), mm(r.height)),
                    (w, h),
                    format!("{} × {}", mm(w), mm(h)),
                )
            } else if let Some(d) = shaft.and_then(|d| model.device(d)) {
                let area = device_area(d.kind, d.width, d.depth, d.height);
                if (d.width - d.depth).abs() < 1e-6 {
                    let dia = SHAFT_SIZES
                        .into_iter()
                        .find(|x| round_area(*x) >= needed - 0.5)
                        .unwrap_or(SHAFT_SIZES[SHAFT_SIZES.len() - 1])
                        .max(d.width);
                    (
                        area,
                        format!("Ø{}", mm(d.width)),
                        (dia, dia),
                        format!("Ø{}", mm(dia)),
                    )
                } else {
                    let w = step_up(needed / (d.depth * 1e4), 0.05).max(d.width);
                    (
                        area,
                        format!("{} × {}", mm(d.width), mm(d.depth)),
                        (w, d.depth),
                        format!("{} × {}", mm(w), mm(d.depth)),
                    )
                }
            } else {
                (0.0, "—".into(), (0.0, 0.0), "—".into())
            };
            stretches.push(Stretch {
                name: String::new(),
                run,
                shaft,
                level,
                parent,
                length,
                flow: fl,
                vents: nv,
                hoods: nh,
                area,
                size,
                needed,
                sized_text: if ducts.len() > 1 {
                    format!("{} (añadir conductos)", ducts_text(&ducts))
                } else {
                    sized_text
                },
                sized,
                ducts,
                on_roof,
            });
            of_edge.insert(e, stretches.len() - 1);
        }
        let vents: Vec<DeviceId> = part
            .iter()
            .filter_map(|n| at_node.get(n))
            .flatten()
            .copied()
            .collect();
        let at_of = |n: usize| up.get(&n).and_then(|e| of_edge.get(e)).copied();
        let vent_at: Vec<Option<usize>> = part
            .iter()
            .filter_map(|n| at_node.get(n).map(|vs| (n, vs)))
            .flat_map(|(n, vs)| vs.iter().map(move |_| at_of(*n)))
            .collect();
        let mut net_hoods = Vec::new();
        let mut hood_at = Vec::new();
        for n in &part {
            for h in at_hood.get(n).into_iter().flatten() {
                net_hoods.push(*h);
                hood_at.push(at_of(*n));
            }
        }
        let total = vents
            .iter()
            .map(|v| vent_flow.get(v).copied().unwrap_or(0.0))
            .sum::<f64>()
            + HOOD_FLOW * net_hoods.len() as f64;
        // The mouth: the fan's top, or the duct's highest end.
        let outlet = match fan.and_then(|(_, d)| model.device(d)) {
            Some(f) => Some((
                f.level,
                f.position,
                crate::water::level_z(model, f.level) + f.elevation + f.height,
                Some(f.id),
            )),
            None => {
                let n = g.nodes[root];
                Some((n.level, n.p, n.z, None))
            }
        }
        .map(|(level, at, z, fan)| outlet(model, level, at, z, fan, hybrid.is_some()));
        out.push(Network {
            stretches,
            fan: fan.map(|(_, d)| d),
            vents,
            vent_at,
            hoods: net_hoods,
            hood_at,
            flow: total,
            hybrid,
            garage_vents,
            outlet,
        });
    }
    (out, loose)
}

/// The floors venting into a hybrid duct: from the lowest with a valve on
/// it to the highest, both in (4.2.1).
fn venting_floors(model: &Model, vents: &[DeviceId]) -> usize {
    let z: Vec<f64> = vents
        .iter()
        .filter_map(|d| model.device(*d))
        .filter_map(|d| model.level(d.level))
        .map(|l| l.elevation)
        .collect();
    let (lo, hi) = (
        z.iter().copied().fold(f64::MAX, f64::min),
        z.iter().copied().fold(f64::MIN, f64::max),
    );
    model
        .levels()
        .iter()
        .filter(|l| l.elevation >= lo - 1e-6 && l.elevation <= hi + 1e-6)
        .count()
        .max(1)
}

/// The distance in plan from a point to a segment.
fn to_segment(p: Point2, a: Point2, b: Point2) -> f64 {
    let ab = b - a;
    let l2 = ab.dot(ab);
    if l2 < 1e-12 {
        return p.distance(a);
    }
    let t = ((p - a).dot(ab) / l2).clamp(0.0, 1.0);
    (a + ab * t).distance(p)
}

/// The distance from a height to a band of heights.
fn to_band(z: f64, lo: f64, hi: f64) -> f64 {
    (lo - z).max(z - hi).max(0.0)
}

/// The nearest air coming in to a point (a window, an outside door, an
/// air inlet, a terrace), within 10 m, and how far, metres (3.2.1.4).
pub fn intake_near(model: &Model, at: Point2, z: f64) -> Option<(String, f64)> {
    let lz = |l: LevelId| crate::water::level_z(model, l);
    let level_name = |l: LevelId| model.level(l).map_or("—".to_string(), |l| l.name.clone());
    let mut intake: Option<(String, f64)> = None;
    let mut near = |what: String, d: f64| {
        if d <= OBSTACLE_FAR && intake.as_ref().is_none_or(|(_, b)| d < *b) {
            intake = Some((what, d));
        }
    };
    // Air inlets.
    for d in model.devices().filter(|d| d.kind == DeviceKind::AirInlet) {
        let dz = z - (lz(d.level) + d.elevation + d.height / 2.0);
        near(
            format!("aireador ({})", level_name(d.level)),
            (d.position.distance(at).powi(2) + dz * dz).sqrt(),
        );
    }
    // Windows and outside doors.
    for o in model.openings() {
        let (Some(t), Some(wall)) = (model.opening_type(o.opening_type), model.wall(o.wall)) else {
            continue;
        };
        if matches!(t.kind, OpeningKind::Void) {
            continue;
        }
        let Some(g) = archisim_geom::opening_geometry(model, o.id) else {
            continue;
        };
        let mid = (g.exterior_offset + g.interior_offset) / 2.0;
        let window = matches!(t.kind, OpeningKind::Window(_));
        if !window {
            // A door with no room on one side.
            let tm = (g.t0 + g.t1) / 2.0;
            let sign = if g.exterior_offset >= g.interior_offset {
                1.0
            } else {
                -1.0
            };
            let inside = |p: Point2| model.rooms_on_level(wall.level).any(|r| r.contains(p));
            if inside(g.point(tm, g.exterior_offset + sign * 0.25))
                && inside(g.point(tm, g.interior_offset - sign * 0.25))
            {
                continue;
            }
        }
        let (a, b) = (g.point(g.t0, mid), g.point(g.t1, mid));
        let lo = lz(wall.level) + o.sill;
        let d = to_segment(at, a, b).hypot(to_band(z, lo, lo + t.height));
        near(
            format!(
                "{} «{}» ({})",
                if window { "ventana" } else { "puerta exterior" },
                t.name,
                level_name(wall.level)
            ),
            d,
        );
    }
    // Terraces, balconies and galleries: people stand there.
    for r in model.rooms() {
        let c = Class::of(&r.usage, &r.name);
        if !c.terrace {
            continue;
        }
        let n = r.outline.len();
        let plan = if r.contains(at) {
            0.0
        } else {
            (0..n)
                .map(|i| to_segment(at, r.outline[i], r.outline[(i + 1) % n]))
                .fold(f64::MAX, f64::min)
        };
        let floor = lz(r.level);
        near(
            format!("terraza «{}» ({})", r.name, level_name(r.level)),
            plan.hypot(to_band(z, floor, floor + 2.0)),
        );
    }
    intake
}

/// An outlet and what is around it (3.2.1).
fn outlet(
    model: &Model,
    level: LevelId,
    at: Point2,
    z: f64,
    fan: Option<DeviceId>,
    hybrid: bool,
) -> Outlet {
    let lz = |l: LevelId| crate::water::level_z(model, l);
    let level_name = |l: LevelId| model.level(l).map_or("—".to_string(), |l| l.name.clone());
    let intake = intake_near(model, at, z);
    let roof = lz(level);
    let walked = model.railings_on_level(level).next().is_some()
        || model.rooms_on_level(level).next().is_some();
    let mut required = if walked {
        HYBRID_ABOVE_WALKED
    } else {
        HYBRID_ABOVE
    };
    let mut obstacle = None;
    if hybrid {
        // The walls standing above the roof around it.
        for w in model.walls() {
            let base = lz(w.level) + w.base_offset;
            let top = base + w.height;
            if top <= roof + 0.05 || base > z {
                continue;
            }
            let d = to_segment(at, w.start, w.end);
            let h = top - roof;
            let asks = if d <= OBSTACLE_NEAR {
                OBSTACLE_FACTOR * h
            } else if d <= OBSTACLE_FAR {
                h
            } else {
                continue;
            };
            if asks > required {
                required = asks;
                obstacle = Some((format!("muro ({})", level_name(w.level)), d, h));
            }
        }
    }
    Outlet {
        level,
        at,
        z,
        fan,
        intake,
        hybrid,
        above_roof: z - roof,
        walked,
        obstacle,
        required,
    }
}

/// What an outlet is, in words.
pub fn outlet_text(o: &Outlet) -> String {
    let mut s = match &o.intake {
        Some((what, d)) => format!(
            "a {} m de {}{}",
            dec(*d, 2),
            what,
            if o.too_close() {
                format!(" — NO CUMPLE: a {} m al menos (3.2.1)", dec(OUTLET_GAP, 0))
            } else {
                String::new()
            }
        ),
        None => format!(
            "nada por donde entre el aire a menos de {} m",
            dec(OBSTACLE_FAR, 0)
        ),
    };
    if o.hybrid {
        s.push_str(&format!(
            "; {} m sobre la cubierta{} (≥ {} m{})",
            dec(o.above_roof, 2),
            if o.walked { " transitable" } else { "" },
            dec(o.required, 2),
            match &o.obstacle {
                Some((what, d, h)) =>
                    format!(" por un {what} de {} m a {} m", dec(*h, 2), dec(*d, 1)),
                None => String::new(),
            }
        ));
        if o.too_low() {
            s.push_str(" — NO CUMPLE (3.2.1)");
        }
    }
    s
}

fn dec(v: f64, n: usize) -> String {
    format!("{v:.n$}").replace('.', ",")
}

/// What a kitchen's hood is, in words.
pub fn hood_text(v: &Ventilation, h: &Hood) -> String {
    match (h.object, h.network.and_then(|i| v.networks.get(i))) {
        (None, _) => format!(
            "{}: sin campana dibujada; necesita extracción propia de {} l/s con su conducto (3.1.1)",
            h.kitchen_name,
            dec(HOOD_FLOW, 0)
        ),
        (Some(_), None) => format!(
            "{}: campana sin conducto — NO CUMPLE: necesita {} l/s por un conducto propio (3.1.1)",
            h.kitchen_name,
            dec(HOOD_FLOW, 0)
        ),
        (Some(_), Some(n)) if n.shared() => format!(
            "{}: la campana va al conducto de la ventilación general — NO CUMPLE: conducto propio (3.1.1)",
            h.kitchen_name
        ),
        (Some(_), Some(_)) => format!(
            "{}: campana de {} l/s por su conducto propio (red {}, 3.1.1)",
            h.kitchen_name,
            dec(HOOD_FLOW, 0),
            h.network.map_or(0, |i| i + 1)
        ),
    }
}

/// What a garage, a storeroom or a refuse room has, line by line.
pub fn other_lines(o: &Other) -> Vec<String> {
    let mut out = Vec::new();
    if !o.drawn() {
        out.push(if o.garage {
            "sin aireadores ni bocas de extracción dibujados: ventilación natural por aberturas mixtas en dos fachadas opuestas, a comprobar (3.1.4), o mecánica".into()
        } else {
            "sin aireadores ni bocas de extracción dibujados".into()
        });
        return out;
    }
    let no = |b: bool| if b { " — NO CUMPLE" } else { "" };
    if !o.vents.is_empty() {
        out.push(format!(
            "bocas de extracción {} cm² (≥ {}, tabla 4.1){}",
            dec(o.vent_area, 0),
            dec(o.vent_needed(), 0),
            no(o.vent_short())
        ));
    }
    if !o.inlets.is_empty() {
        out.push(format!(
            "aberturas de admisión {} cm² (≥ {}){}",
            dec(o.inlet_area, 0),
            dec(o.inlet_needed(), 0),
            if o.inlet_short() {
                " — a completar con las rejillas de la puerta"
            } else {
                ""
            }
        ));
    }
    if o.garage {
        let need = o.openings_needed();
        out.push(format!(
            "{} boca{} de extracción y {} de admisión (≥ {} de cada una, una cada 100 m², 3.1.4){}",
            o.vents.len(),
            if o.vents.len() == 1 { "" } else { "s" },
            o.inlets.len(),
            need,
            no(o.vents.len() < need)
        ));
        if o.vents.len() > 1 {
            out.push(format!(
                "bocas de extracción a {} m como mucho de la siguiente (< {} m){}",
                dec(o.vent_gap, 1),
                dec(GARAGE_VENT_SPACING, 0),
                no(o.vent_gap > GARAGE_VENT_SPACING + 1e-6)
            ));
        }
        if o.spaces >= GARAGE_TWO_NETWORKS {
            out.push(format!(
                "{} red{} de extracción en la planta (≥ 2 con {} plazas o más){}",
                o.networks,
                if o.networks == 1 { "" } else { "es" },
                GARAGE_TWO_NETWORKS,
                no(o.networks < 2)
            ));
        }
        if o.needs_co() {
            out.push(
                "detección de monóxido de carbono que ponga en marcha los extractores a 50 ppm (100 ppm si no hay personal), a prever (3.1.4)"
                    .into(),
            );
        }
    }
    out
}

/// The calculation in words.
pub fn report(model: &Model, v: &Ventilation) -> String {
    use std::fmt::Write as _;
    let mut s = String::new();
    let _ = writeln!(s, "VENTILACIÓN DE LAS VIVIENDAS (DB-HS 3)");
    if !v.applies {
        let _ = writeln!(
            s,
            "\nEl edificio no es de viviendas: su ventilación es la del RITE (IT 1.1.4.2), que no se calcula aquí."
        );
        return s;
    }
    let level_name = |l: LevelId| model.level(l).map_or("—".to_string(), |l| l.name.clone());
    for d in &v.dwellings {
        let _ = writeln!(
            s,
            "\n{} — {} dormitorio{}: caudal {} l/s ({} m³/h; locales secos {} l/s, húmedos {} l/s; tabla 2.1)",
            d.name,
            d.bedrooms,
            if d.bedrooms == 1 { "" } else { "s" },
            dec(d.flow, 1),
            dec(d.flow * 3.6, 0),
            dec(d.dry_flow, 1),
            dec(d.wet_flow, 1),
        );
        for r in &d.rooms {
            if r.admission > 0.0 {
                let _ = writeln!(
                    s,
                    "  {} ({}): admisión {} l/s, aireadores {} cm² (≥ {}){}",
                    r.name,
                    r.dry.map_or("seco", |x| x.name()),
                    dec(r.admission, 1),
                    dec(r.inlet_area, 0),
                    dec(r.inlet_needed(), 0),
                    if r.inlet_short() {
                        " — NO CUMPLE"
                    } else {
                        ""
                    }
                );
            }
            if r.extraction > 0.0 {
                let _ = writeln!(
                    s,
                    "  {} (húmedo): extracción {} l/s, bocas {} cm² (≥ {}){}",
                    r.name,
                    dec(r.extraction, 1),
                    dec(r.vent_area, 0),
                    dec(r.vent_needed(), 0),
                    if r.vent_short() { " — NO CUMPLE" } else { "" }
                );
            }
        }
        let room_name = |id: RoomId| model.room(id).map_or("—".to_string(), |r| r.name.clone());
        for p in &d.passages {
            let _ = writeln!(
                s,
                "  Paso {} → {}: {} l/s, {} {} cm² (≥ {}){}",
                room_name(p.from),
                room_name(p.to),
                dec(p.flow, 1),
                if p.door.is_some() {
                    "holgura bajo la hoja"
                } else {
                    "hueco"
                },
                dec(p.area, 0),
                dec(p.needed(), 0),
                match (p.short(), p.gap_needed()) {
                    (true, Some(g)) => format!(" — NO CUMPLE: holgura de {g} mm o rejilla de paso"),
                    (true, None) => " — NO CUMPLE".to_string(),
                    _ => String::new(),
                }
            );
        }
        for h in v
            .hoods
            .iter()
            .filter(|h| h.dwelling == d.name && h.level == d.level)
        {
            let _ = writeln!(s, "  {}", hood_text(v, h));
        }
    }
    for (i, n) in v.networks.iter().enumerate() {
        if n.is_hood_duct() {
            let _ = writeln!(
                s,
                "\nRed de extracción {} (campanas): {} campana{}, {} l/s ({} m³/h), salida por su extremo más alto",
                i + 1,
                n.hoods.len(),
                if n.hoods.len() == 1 { "" } else { "s" },
                dec(n.flow, 1),
                dec(n.flow * 3.6, 0),
            );
        } else {
            let _ = writeln!(
                s,
                "\nRed de extracción {}: {} bocas{}, {} l/s ({} m³/h). Extractor: {}",
                i + 1,
                n.vents.len(),
                if n.shared() {
                    format!(
                        " y {} campana{} — NO CUMPLE: la campana lleva conducto propio (3.1.1)",
                        n.hoods.len(),
                        if n.hoods.len() == 1 { "" } else { "s" }
                    )
                } else {
                    String::new()
                },
                dec(n.flow, 1),
                dec(n.flow * 3.6, 0),
                match n.fan.and_then(|f| model.device(f)) {
                    Some(f) => format!("«{}», {} m³/h", f.name, dec(n.flow * 3.6, 0)),
                    None => "no está dibujado".into(),
                }
            );
        }
        if let Some((floors, class)) = n.hybrid {
            let _ = writeln!(
                s,
                "  Ventilación híbrida: {} planta{} vierten al conducto, zona {}: tiro T-{} (tablas 4.3 y 4.4); secciones de la tabla 4.2",
                floors,
                if floors == 1 { "" } else { "s" },
                model.ventilation_site().zone.code(),
                class
            );
        }
        if n.garage_shared() {
            let _ = writeln!(
                s,
                "  — NO CUMPLE: la red del aparcamiento sirve también a otros locales (3.1.4)"
            );
        }
        if let Some(o) = &n.outlet {
            let _ = writeln!(
                s,
                "  Salida ({}, a {} m): {}",
                level_name(o.level),
                dec(o.z, 2),
                outlet_text(o)
            );
        }
        for st in &n.stretches {
            let _ = writeln!(
                s,
                "  {} ({}, {}): {} m, {} l/s, {} = {} cm² (≥ {}, {}){}",
                st.name,
                if st.shaft.is_some() {
                    "conducto vertical"
                } else {
                    "conducto"
                },
                level_name(st.level),
                dec(st.length, 1),
                dec(st.flow, 1),
                st.size,
                dec(st.area, 0),
                dec(st.needed, 0),
                st.sized_text,
                if st.short() { " — NO CUMPLE" } else { "" }
            );
        }
    }
    for o in &v.others {
        let _ = writeln!(
            s,
            "\n{} ({}, {} m²{}): {} l/s (tabla 2.2)",
            o.name,
            o.what,
            dec(o.area, 1),
            if o.garage {
                format!(
                    ", {} plaza{}{}",
                    o.spaces,
                    if o.spaces == 1 { "" } else { "s" },
                    if o.cars_drawn {
                        ""
                    } else {
                        " a razón de una cada 25 m²"
                    }
                )
            } else {
                String::new()
            },
            dec(o.flow, 1)
        );
        for t in other_lines(o) {
            let _ = writeln!(s, "  {t}");
        }
    }
    if !v.loose.is_empty() {
        let _ = writeln!(s, "\n{} bocas de extracción sin conducto.", v.loose.len());
    }
    if !v.lone_wet.is_empty() {
        let names: Vec<String> = v
            .lone_wet
            .iter()
            .filter_map(|r| model.room(*r))
            .map(|r| format!("{} ({})", r.name, level_name(r.level)))
            .collect();
        let _ = writeln!(
            s,
            "\nLocales húmedos fuera de las viviendas (sin puerta a un local seco), no se calculan: {}.",
            names.join(", ")
        );
    }
    s
}

/// An example (for the tests and `archisim ventilacion --ejemplo`): a
/// flat of two bedrooms: living room, kitchen, bathroom and a
/// corridor between them; its inlets, valves, ducts to a shaft, and the
/// fan on the roof.
pub fn example_flat(
    m: &mut Model,
    l0: LevelId,
    l1: LevelId,
    partition: WallTypeId,
    facade: WallTypeId,
    door_type: OpeningTypeId,
) {
    let wall = |m: &mut Model, a: (f64, f64), b: (f64, f64)| {
        m.add_wall(NewWall {
            wall_type: partition,
            level: l0,
            start: Point2::new(a.0, a.1),
            end: Point2::new(b.0, b.1),
            bulge: 0.0,
            reference: ReferenceLine::Center,
            height: 2.7,
            base_offset: 0.0,
            top: WallTop::Unconnected,
        })
        .expect("example flat")
    };
    // 10 × 6: the corridor along y 2..3; south the living (0..5) and
    // the kitchen (5..10); north the bedrooms (0..4, 4..7) and the
    // bathroom (7..10).
    for (a, b) in [
        ((0.0, 0.0), (10.0, 0.0)),
        ((10.0, 0.0), (10.0, 6.0)),
        ((10.0, 6.0), (0.0, 6.0)),
        ((0.0, 6.0), (0.0, 0.0)),
    ] {
        let w = wall(m, a, b);
        m.set_wall_type(w, facade).expect("example flat");
    }
    let corridor_s = wall(m, (0.0, 2.0), (10.0, 2.0));
    let corridor_n = wall(m, (0.0, 3.0), (10.0, 3.0));
    wall(m, (5.0, 0.0), (5.0, 2.0));
    wall(m, (4.0, 3.0), (4.0, 6.0));
    wall(m, (7.0, 3.0), (7.0, 6.0));
    let room = |m: &mut Model, n: &str, name: &str, usage: &str, a: (f64, f64), b: (f64, f64)| {
        let id = m
            .add_room(NewRoom {
                level: l0,
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
            .expect("example flat");
        let mut r = m.room(id).expect("example flat").clone();
        r.usage = usage.into();
        m.set_room(r).expect("example flat");
    };
    let h = 0.05;
    room(m, "1", "Estar", "Estar", (h, h), (5.0 - h, 2.0 - h));
    room(
        m,
        "2",
        "Cocina",
        "Cocina",
        (5.0 + h, h),
        (10.0 - h, 2.0 - h),
    );
    room(
        m,
        "3",
        "Pasillo",
        "Pasillo",
        (h, 2.0 + h),
        (10.0 - h, 3.0 - h),
    );
    room(
        m,
        "4",
        "Dormitorio 1",
        "Dormitorio",
        (h, 3.0 + h),
        (4.0 - h, 6.0 - h),
    );
    room(
        m,
        "5",
        "Dormitorio 2",
        "Dormitorio",
        (4.0 + h, 3.0 + h),
        (7.0 - h, 6.0 - h),
    );
    room(
        m,
        "6",
        "Baño",
        "Baño",
        (7.0 + h, 3.0 + h),
        (10.0 - h, 6.0 - h),
    );
    let door = |m: &mut Model, w, along| {
        m.add_opening(NewOpening {
            opening_type: door_type,
            wall: w,
            along,
            sill: 0.0,
            opens_to: WallSide::Interior,
            hand: Hand::Left,
            setback: None,
        })
        .expect("example flat");
    };
    door(m, corridor_s, 2.5);
    door(m, corridor_s, 7.5);
    door(m, corridor_n, 2.0);
    door(m, corridor_n, 5.5);
    door(m, corridor_n, 8.5);
    let dev = |m: &mut Model, level, kind: DeviceKind, x: f64, y: f64, w: f64, d: f64, hgt: f64| {
        m.add_device(NewDevice {
            level,
            kind,
            name: kind.name().into(),
            position: Point2::new(x, y),
            rotation: 0.0,
            elevation: match kind {
                DeviceKind::AirInlet | DeviceKind::ExhaustVent => kind.defaults().3,
                _ => 0.0,
            },
            width: w,
            depth: d,
            height: hgt,
        })
        .expect("example flat")
    };
    // Inlets on the outer walls: 0,4 × 0,02 = 80 cm² each.
    dev(m, l0, DeviceKind::AirInlet, 2.5, 0.0, 0.4, 0.06, 0.02);
    dev(m, l0, DeviceKind::AirInlet, 2.0, 6.0, 0.4, 0.06, 0.02);
    // The second bedroom's inlet too small: 0,1 × 0,015 = 15 cm², of
    // 4 · 4,8 = 19,2.
    dev(m, l0, DeviceKind::AirInlet, 5.5, 6.0, 0.1, 0.06, 0.015);
    // Valves: Ø125 in the kitchen and the bathroom.
    dev(m, l0, DeviceKind::ExhaustVent, 9.0, 1.0, 0.125, 0.125, 0.05);
    dev(m, l0, DeviceKind::ExhaustVent, 9.0, 4.5, 0.125, 0.125, 0.05);
    // The shaft at (9.5, 2.5), to the roof; the fan beside it.
    dev(m, l0, DeviceKind::AirShaft, 9.5, 2.5, 0.15, 0.15, 3.0);
    dev(m, l1, DeviceKind::Fan, 9.5, 3.1, 0.6, 0.6, 0.55);
    let run = |m: &mut Model, level, path: Vec<Point2>, w: f64, hh: f64| {
        m.add_run(NewRun {
            level,
            system: ServiceSystem::ExhaustAir,
            path,
            // On the roof, out of the shaft's head into the fan.
            elevation: if level == l1 { 0.3 } else { 2.5 },
            width: w,
            height: hh,
            slope: 0.0,
        })
        .expect("example flat");
    };
    run(
        m,
        l0,
        vec![
            Point2::new(9.0, 1.0),
            Point2::new(9.0, 2.5),
            Point2::new(9.5, 2.5),
        ],
        0.1,
        0.1,
    );
    run(
        m,
        l0,
        vec![Point2::new(9.0, 4.5), Point2::new(9.0, 2.5)],
        0.1,
        0.1,
    );
    run(
        m,
        l1,
        vec![Point2::new(9.5, 2.5), Point2::new(9.5, 3.1)],
        0.15,
        0.15,
    );
}

/// The kitchen of [`example_flat`] with its hood (an object of
/// `hood_type`, of the family «Campana extractora») and the hood's own
/// duct: along the ceiling to a shaft Ø150 of its own, up to the roof,
/// where it comes out by a short duct.
pub fn example_hood(m: &mut Model, l0: LevelId, l1: LevelId, hood_type: ObjectTypeId) {
    m.add_object(NewObject {
        level: l0,
        object_type: hood_type,
        position: Point2::new(7.0, 0.35),
        rotation: 0.0,
        base_offset: 0.0,
    })
    .expect("example hood");
    m.add_device(NewDevice {
        level: l0,
        kind: DeviceKind::AirShaft,
        name: "Conducto de la campana".into(),
        position: Point2::new(5.6, 0.35),
        rotation: 0.0,
        elevation: 0.0,
        width: 0.15,
        depth: 0.15,
        height: 3.0,
    })
    .expect("example hood");
    for (level, path, z) in [
        (
            l0,
            vec![Point2::new(7.0, 0.35), Point2::new(5.6, 0.35)],
            2.5,
        ),
        (l1, vec![Point2::new(5.6, 0.35), Point2::new(5.6, 0.7)], 0.3),
    ] {
        m.add_run(NewRun {
            level,
            system: ServiceSystem::ExhaustAir,
            path,
            elevation: z,
            width: 0.15,
            height: 0.15,
            slope: 0.0,
        })
        .expect("example hood");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_table() {
        assert_eq!(flows(1).wet_total, 12.0);
        assert_eq!(flows(2).living, 8.0);
        assert_eq!(flows(4).wet_total, 33.0);
        let c = Class::of("Estar", "Estar-cocina E");
        assert!(c.living && c.kitchen && c.wet && !c.common);
        assert!(Class::of("Pasillo común", "Pasillo común").common);
        assert!(Class::of("Baño", "Baño A").wet);
        assert_eq!(vent_size(40.0), 0.1);
        assert_eq!(vent_size(100.0), 0.125);
    }

    fn flat() -> Model {
        let mut m = Model::new("Piso");
        let l0 = m.add_level("Planta baja", 0.0);
        let l1 = m.add_level("Cubierta", 3.0);
        let lib = archisim_library::add_library(&mut m).unwrap();
        example_flat(&mut m, l0, l1, lib.partition, lib.facade, lib.interior_door);
        m
    }

    #[test]
    fn the_kitchen_hood_has_its_own_duct() {
        // Without a hood drawn: the kitchen is told, nothing fails.
        let m = flat();
        let v = ventilation(&m);
        assert_eq!(v.hoods.len(), 1);
        assert!(v.hoods[0].object.is_none());
        assert!(!v.hood_fails(&v.hoods[0]));
        // With its hood and its own duct to the roof: 50 l/s, Ø150 is
        // 177 cm² against 125.
        let mut m = flat();
        let l0 = m.levels()[0].id;
        let l1 = m.levels()[1].id;
        let hood = archisim_library::add_object_entry(&mut m, "Campana decorativa 60").unwrap();
        example_hood(&mut m, l0, l1, hood);
        let v = ventilation(&m);
        let h = &v.hoods[0];
        assert!(h.object.is_some());
        let n = &v.networks[h.network.expect("its duct")];
        assert!(n.is_hood_duct());
        assert!(n.fan.is_none());
        assert!((n.flow - HOOD_FLOW).abs() < 1e-9);
        assert_eq!(n.hood_at.len(), 1);
        assert!(n.hood_at[0].is_some());
        assert!(n.stretches.iter().all(|s| !s.short()), "{:?}", n.stretches);
        assert!(!v.hood_fails(h));
        // The general network is still the same, with its fan.
        assert_eq!(v.networks.len(), 2);
        let general = v.networks.iter().find(|n| !n.vents.is_empty()).unwrap();
        assert!(general.fan.is_some());
        assert_eq!(general.vent_at.len(), general.vents.len());
        assert!(general.vent_at.iter().all(|s| s.is_some()));
        // Its duct taken to the general shaft: shared, it fails.
        let duct = m
            .runs()
            .find(|r| r.level == l0 && r.path[0].distance(Point2::new(7.0, 0.35)) < 1e-9)
            .map(|r| r.id)
            .unwrap();
        m.edit_run(duct, |r| {
            r.path = vec![
                Point2::new(7.0, 0.35),
                Point2::new(9.0, 0.35),
                Point2::new(9.0, 1.0),
            ];
        })
        .unwrap();
        let v = ventilation(&m);
        let h = &v.hoods[0];
        assert!(v.networks[h.network.unwrap()].shared());
        assert!(v.hood_fails(h));
        assert!(report(&m, &v).contains("conducto propio"));
    }

    #[test]
    fn the_hybrid_tables() {
        // Table 4.3: one floor in Madrid (X) draws T-3; eight anywhere T-1.
        assert_eq!(draft_class(1, DraftZone::X), 3);
        assert_eq!(draft_class(9, DraftZone::Z), 1);
        assert_eq!(draft_class(5, DraftZone::W), 1);
        assert_eq!(draft_class(7, DraftZone::Z), 2);
        // Table 4.2.
        assert_eq!(hybrid_ducts(80.0, 1), Some(&[225.0][..]));
        assert_eq!(hybrid_ducts(300.0, 4), Some(&[900.0][..]));
        assert_eq!(hybrid_ducts(600.0, 3), Some(&[900.0, 625.0][..]));
        assert_eq!(hybrid_ducts(1200.0, 1), None);
        assert_eq!(ducts_text(&[900.0, 900.0]), "2 × 900 cm²");
        assert_eq!(ducts_text(&[900.0, 625.0]), "900 + 625 cm²");
    }

    #[test]
    fn where_the_ducts_come_out() {
        let mut m = flat();
        let l1 = m.levels()[1].id;
        let v = ventilation(&m);
        let n = &v.networks[0];
        let o = n.outlet.as_ref().expect("its outlet");
        // The fan's top: on the roof at 3 m, 0,55 m high.
        assert!(o.fan.is_some() && (o.z - 3.55).abs() < 1e-9, "{o:?}");
        assert!(!o.hybrid && !o.fails(), "{o:?}");
        // Mechanical: on the roof the duct needs 1,5·q.
        let roof = n.stretches.iter().find(|s| s.level == l1).unwrap();
        assert!(roof.on_roof && (roof.needed - 1.5 * roof.flow).abs() < 1e-9);
        // An inlet 1,4 m away: too close.
        let inlet = m
            .add_device(NewDevice {
                level: l1,
                kind: DeviceKind::AirInlet,
                name: "Toma".into(),
                position: Point2::new(9.5, 4.5),
                rotation: 0.0,
                elevation: 0.5,
                width: 0.4,
                depth: 0.06,
                height: 0.02,
            })
            .unwrap();
        let v = ventilation(&m);
        let o = v.networks[0].outlet.as_ref().unwrap();
        assert!(o.too_close(), "{o:?}");
        assert!(v.fails() > 0);
        assert!(report(&m, &v).contains("a 3 m al menos"));
        m.remove_device(inlet).unwrap();

        // Hybrid in Madrid: one floor, T-3; 24 l/s want 625 cm², and the
        // shaft Ø150 (177) is short; the fan 0,55 m over the roof, short
        // of 1 m.
        m.set_ventilation_site(archisim_core::VentilationSite {
            system: VentSystem::Hybrid,
            zone: DraftZone::X,
        })
        .unwrap();
        let v = ventilation(&m);
        let n = &v.networks[0];
        assert_eq!(n.hybrid, Some((1, 3)));
        let shaft = n.stretches.iter().find(|s| s.shaft.is_some()).unwrap();
        assert!((shaft.needed - 625.0).abs() < 1e-9 && shaft.short());
        assert!(n.outlet.as_ref().unwrap().too_low());
        // Raised to 1,55 m it passes; a parapet 1,2 m high 1 m away asks
        // 1,3 × 1,2 = 1,56 m.
        let fan = n.fan.unwrap();
        m.edit_device(fan, |d| d.elevation = 1.0).unwrap();
        let v = ventilation(&m);
        let o = v.networks[0].outlet.as_ref().unwrap();
        assert!(!o.too_low(), "{o:?}");
        let lib_wall = m.walls().next().unwrap().wall_type;
        m.add_wall(NewWall {
            wall_type: lib_wall,
            level: l1,
            start: Point2::new(8.5, 2.0),
            end: Point2::new(8.5, 5.0),
            bulge: 0.0,
            reference: ReferenceLine::Center,
            height: 1.2,
            base_offset: 0.0,
            top: WallTop::Unconnected,
        })
        .unwrap();
        let v = ventilation(&m);
        let o = v.networks[0].outlet.as_ref().unwrap();
        assert!((o.required - 1.56).abs() < 1e-9 && o.too_low(), "{o:?}");
        assert!(outlet_text(o).contains("NO CUMPLE"));
    }

    #[test]
    fn a_garage() {
        let mut m = flat();
        let below = m.add_level("Sótano", -3.0);
        let id = m
            .add_room(NewRoom {
                level: below,
                number: "S1".into(),
                name: "Garaje".into(),
                outline: vec![
                    Point2::new(0.0, 0.0),
                    Point2::new(20.0, 0.0),
                    Point2::new(20.0, 10.0),
                    Point2::new(0.0, 10.0),
                ],
                holes: Vec::new(),
                label: Point2::new(10.0, 5.0),
                bounded: false,
            })
            .unwrap();
        // 200 m², no cars: 8 spaces, 960 l/s; nothing drawn, a note.
        let v = ventilation(&m);
        let g = v.others.iter().find(|o| o.room == id).unwrap();
        assert!(g.garage && g.spaces == 8 && !g.cars_drawn);
        assert!((g.flow - 960.0).abs() < 1e-9);
        assert_eq!(g.garage_fails(), 0);
        assert!(g.needs_co());
        // Three cars drawn: 360 l/s.
        let car = archisim_library::add_object_entry(&mut m, "Turismo").unwrap();
        for x in [3.0, 6.0, 9.0] {
            m.add_object(NewObject {
                level: below,
                object_type: car,
                position: Point2::new(x, 3.0),
                rotation: 0.0,
                base_offset: 0.0,
            })
            .unwrap();
        }
        // One valve Ø250 on a duct of its own: 2 needed, and short.
        m.add_device(NewDevice {
            level: below,
            kind: DeviceKind::ExhaustVent,
            name: "Boca".into(),
            position: Point2::new(15.0, 9.0),
            rotation: 0.0,
            elevation: 2.4,
            width: 0.25,
            depth: 0.25,
            height: 0.05,
        })
        .unwrap();
        let v = ventilation(&m);
        let g = v.others.iter().find(|o| o.room == id).unwrap();
        assert!(g.cars_drawn && g.spaces == 3, "{g:?}");
        assert!((g.flow - 360.0).abs() < 1e-9);
        assert!(g.vent_short());
        assert_eq!(g.garage_fails(), 1);
        let text = report(&m, &v);
        assert!(
            text.contains("aparcamiento") && text.contains("monóxido"),
            "{text}"
        );
        // Its valve joined to the dwelling's network: shared, it fails.
        m.add_run(NewRun {
            level: below,
            system: ServiceSystem::ExhaustAir,
            path: vec![Point2::new(15.0, 9.0), Point2::new(9.5, 2.5)],
            elevation: 2.5,
            width: 0.4,
            height: 0.3,
            slope: 0.0,
        })
        .unwrap();
        m.add_device(NewDevice {
            level: below,
            kind: DeviceKind::AirShaft,
            name: "Conducto".into(),
            position: Point2::new(9.5, 2.5),
            rotation: 0.0,
            elevation: 0.0,
            width: 0.4,
            depth: 0.4,
            height: 3.0,
        })
        .unwrap();
        let v = ventilation(&m);
        assert!(
            v.networks.iter().any(|n| n.garage_shared()),
            "{:?}",
            v.networks
        );
    }

    #[test]
    fn a_study_counts_as_a_bedroom() {
        let mut m = flat();
        let mut room = m
            .rooms()
            .find(|r| r.name == "Dormitorio 2")
            .cloned()
            .unwrap();
        room.name = "Estudio".into();
        room.usage = String::new();
        m.set_room(room).unwrap();
        let v = ventilation(&m);
        let d = &v.dwellings[0];
        // Still a flat of two bedrooms: 8 l/s the living room, not 6.
        assert_eq!(d.bedrooms, 2);
        let study = d.rooms.iter().find(|r| r.name == "Estudio").unwrap();
        assert_eq!(study.dry, Some(Dry::Study));
        let main = d
            .rooms
            .iter()
            .find(|r| r.dry == Some(Dry::MainBedroom))
            .unwrap();
        assert_eq!(main.name, "Dormitorio 1");
        assert!((d.dry_flow - 20.0).abs() < 1e-9, "{}", d.dry_flow);
    }

    #[test]
    fn a_flat_breathes() {
        let m = flat();
        let v = ventilation(&m);
        assert!(v.applies);
        assert_eq!(v.dwellings.len(), 1, "{:?}", v.dwellings);
        let d = &v.dwellings[0];
        assert_eq!(d.bedrooms, 2);
        // Dry: 8 + 4 + 8 = 20; wet: 7 + 7 = 14, at least 24: 24 rules.
        assert!((d.dry_flow - 20.0).abs() < 1e-9, "{}", d.dry_flow);
        assert!((d.flow - 24.0).abs() < 1e-9, "{}", d.flow);
        let kitchen = d.rooms.iter().find(|r| r.kitchen).unwrap();
        assert!((kitchen.extraction - 12.0).abs() < 1e-9);
        // The main bedroom is the larger one, with 8 l/s raised by 24/20.
        let main = d
            .rooms
            .iter()
            .find(|r| r.dry == Some(Dry::MainBedroom))
            .unwrap();
        assert_eq!(main.name, "Dormitorio 1");
        assert!((main.admission - 9.6).abs() < 1e-9, "{}", main.admission);
        // The second bedroom's inlet is short; the first one's is enough.
        let second = d.rooms.iter().find(|r| r.name == "Dormitorio 2").unwrap();
        assert!(second.inlet_short());
        assert!(!main.inlet_short());
        // Ø125 valves give 123 cm², 48 needed.
        assert!(!kitchen.vent_short());
        // The air goes through the doors: four passages at least, the
        // living room's to the kitchen by the corridor.
        assert!(d.passages.len() >= 4, "{:?}", d.passages);
        let total_in: f64 = d.rooms.iter().map(|r| r.admission).sum();
        assert!((total_in - 24.0).abs() < 1e-9);
        // An undercut of 1 cm under a 72 cm leaf: 72 cm², short of 8·q
        // for the larger passages.
        assert!(d.passages.iter().any(|p| p.short()));
        // The ducts: from both valves to the shaft and up to the fan.
        assert_eq!(v.networks.len(), 1, "{:?}", v.networks);
        let n = &v.networks[0];
        assert!(n.fan.is_some());
        assert_eq!(n.vents.len(), 2);
        assert!((n.flow - 24.0).abs() < 1e-9);
        let shaft = n
            .stretches
            .iter()
            .find(|s| s.shaft.is_some())
            .expect("shaft");
        assert!((shaft.flow - 24.0).abs() < 1e-9);
        // Ø150: 177 cm², 60 needed.
        assert!(!shaft.short());
        // 10 × 10 duct: 100 cm²; enough for one valve (30), not short.
        assert!(n.stretches.iter().all(|s| !s.short()), "{:?}", n.stretches);
        let text = report(&m, &v);
        if std::env::var("ARCHISIM_VENTILACION_TEXTO").is_ok() {
            println!("{text}");
        }
        assert!(text.contains("tabla 2.1"), "{text}");
        let sized = v.sized_devices(&m);
        assert!(sized
            .iter()
            .any(|(id, w, _)| second.inlets.contains(id) && *w > 0.1));
    }
}
