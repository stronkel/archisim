//! The envelope by the DB-HE 1 (2019) and the heating and cooling loads
//! of every room, with the equipment that covers them.
//!
//! - **The envelope** of each room comes from its outline. Each edge
//!   whose other side, 60 cm out, is in no room of the level is outside:
//!   its wall (the nearest one along it), the windows and doors in that
//!   wall within the edge, or a curtain wall. Above it, the floor of the
//!   level above or a roof, unless a room of that level is over it; below
//!   it, its slab on the ground floor (against the ground). Edges and
//!   floors against another room (heated or not), interior walls without
//!   windows (to a stairwell or a shaft; one with a window is on a light
//!   well) and party walls are left out.
//! - **Transmittance** U = 1/(Rsi + Σ e/λ + Rse) with the layers of the
//!   wall, slab or roof type and the conductivity of each material
//!   ([`Model::conductivity`]); windows and doors by the figures of the
//!   project (`ThermalSite::window_u`, `door_u`).
//! - **The ground**: the slab on the ground with its equivalent U by
//!   UNE-EN ISO 13370 (its area and the façade around it, level by
//!   level), and on levels under the ground the walls against it by their
//!   depth ([`crate::envelope`]).
//! - **Linear thermal bridges** measured on the drawing: slab fronts at
//!   the façades, columns in them, the perimeter of every window and
//!   outside door, outward and inward corners, and the façade against the
//!   roof and the ground slab, with a ψ by class and by where the wall's
//!   insulation is.
//! - **DB-HE 1** (table 3.1.1.a-HE1): every element against its limit by
//!   the winter zone, and the global coefficient K = (Σ U·A + Σ ψ·L) / Σ A
//!   of the heated rooms (table 3.1.1.b-HE1, for new dwellings) by the
//!   compactness V/A; the solar control of July (3.1.2), the air
//!   permeability of the windows and n50 (3.1.3), and the surface
//!   condensation of the opaque types and the bridges.
//! - **Heating load** of each room: U·A·ΔT of what is outside (half the
//!   difference against the ground), ψ·L·ΔT of its bridges and the
//!   ventilation, 1,2·q·ΔT with the inflow of DB-HS 3 (or half a renewal
//!   an hour).
//! - **Cooling load**: the same transmission with the summer difference,
//!   the sun through the glass (by orientation), people (75 W sensible
//!   and 55 W latent each), light and appliances, and the ventilation.
//! - **Equipment**: the indoor unit of each dry room and the heat pump of
//!   each dwelling (or room) from the usual sizes; the radiators drawn,
//!   against the heating load of their room.
//!
//! **To be checked by Antonio**: the limits of DB-HE 1, the default
//! conductivities, the solar gains and the internal gains are written as
//! remembered (they are together below).

use std::collections::BTreeMap;

use std::collections::BTreeSet;

use archisim_core::{
    point_in_polygon, BuildingUse, ColumnId, DeviceId, DeviceKind, Insulation, Junction, LevelId,
    Model, OpeningKind, OpeningTypeId, Point2, RoomId, ThermalSite, Vec2,
};

use crate::envelope as env;

/// Surface resistances, m²K/W (DB-HE, documento de apoyo 1): inside a
/// wall, outside; inside a roof (heat going up), inside a floor (going
/// down).
pub const RSI_WALL: f64 = 0.13;
pub const RSE: f64 = 0.04;
pub const RSI_UP: f64 = 0.10;
pub const RSI_DOWN: f64 = 0.17;

/// Table 3.1.1.a-HE1: limits of transmittance, W/m²K, by winter zone
/// (α, A, B, C, D, E).
pub const WALL_LIMIT: [f64; 6] = [0.80, 0.70, 0.56, 0.49, 0.41, 0.37];
pub const ROOF_LIMIT: [f64; 6] = [0.55, 0.50, 0.44, 0.40, 0.35, 0.33];
pub const GROUND_LIMIT: [f64; 6] = [0.90, 0.80, 0.75, 0.70, 0.65, 0.59];
pub const OPENING_LIMIT: [f64; 6] = [3.2, 2.7, 2.3, 2.1, 1.8, 1.8];
pub const DOOR_LIMIT: f64 = 5.7;
/// Table 3.1.1.b-HE1, new dwellings: K at V/A ≤ 1 and at V/A ≥ 4, W/m²K.
pub const K_COMPACT_LOW: [f64; 6] = [0.67, 0.60, 0.58, 0.53, 0.48, 0.43];
pub const K_COMPACT_HIGH: [f64; 6] = [0.86, 0.80, 0.77, 0.72, 0.67, 0.62];

/// Peak sun through glass by orientation, W/m² (summer): north, east,
/// south, west, and a roof light.
pub const SUN_N: f64 = 100.0;
pub const SUN_E: f64 = 450.0;
pub const SUN_S: f64 = 350.0;
pub const SUN_W: f64 = 500.0;
pub const SUN_UP: f64 = 750.0;
/// People, W each: sensible and latent.
pub const PERSON_SENSIBLE: f64 = 75.0;
pub const PERSON_LATENT: f64 = 55.0;
/// Light and appliances, W/m²; a kitchen's appliances.
pub const LIGHT_W_M2: f64 = 5.0;
pub const APPLIANCES_W_M2: f64 = 3.0;
pub const KITCHEN_W_M2: f64 = 20.0;
/// The depth of a curtain wall for the equivalent thickness of a floor on
/// the ground, m.
pub const CURTAIN_DEPTH: f64 = 0.15;
/// What a radiator gives, W per m² of its front (aluminium, ΔT 50).
pub const RADIATOR_W_M2: f64 = 2000.0;
/// Usual sizes, kW: indoor units and heat pumps.
pub const INDOOR_UNITS: [f64; 7] = [2.0, 2.5, 3.5, 4.2, 5.0, 6.0, 7.1];
pub const HEAT_PUMPS: [f64; 10] = [3.5, 5.0, 6.0, 7.1, 8.0, 10.0, 12.0, 14.0, 16.0, 20.0];

/// What a piece of envelope is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Kind {
    Wall,
    Window,
    Door,
    /// A curtain wall: glass.
    Curtain,
    Roof,
    /// The floor of a room under the open air (over a porch).
    ExposedFloor,
    Ground,
    /// A wall against the ground (a basement's).
    GroundWall,
}

impl Kind {
    pub fn name(self) -> &'static str {
        match self {
            Kind::Wall => "muro de fachada",
            Kind::Window => "ventana",
            Kind::Door => "puerta exterior",
            Kind::Curtain => "muro cortina",
            Kind::Roof => "cubierta",
            Kind::ExposedFloor => "suelo sobre el exterior",
            Kind::Ground => "suelo sobre el terreno",
            Kind::GroundWall => "muro contra el terreno",
        }
    }

    pub fn glazed(self) -> bool {
        matches!(self, Kind::Window | Kind::Curtain)
    }

    /// Against the ground: half the difference in winter, none in summer.
    pub fn on_ground(self) -> bool {
        matches!(self, Kind::Ground | Kind::GroundWall)
    }

    /// Opaque and against the outside air: what surface condensation and
    /// the air flow of anejo H look at.
    pub fn opaque_to_air(self) -> bool {
        matches!(self, Kind::Wall | Kind::Roof | Kind::ExposedFloor)
    }

    /// Its limit in DB-HE 1 for a winter zone (by its column).
    pub fn limit(self, zone: usize) -> f64 {
        match self {
            Kind::Wall | Kind::ExposedFloor => WALL_LIMIT[zone],
            Kind::Roof => ROOF_LIMIT[zone],
            Kind::Ground | Kind::GroundWall => GROUND_LIMIT[zone],
            Kind::Window | Kind::Curtain => OPENING_LIMIT[zone],
            Kind::Door => DOOR_LIMIT,
        }
    }
}

/// The four orientations.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Facing {
    North,
    East,
    South,
    West,
    Up,
}

impl Facing {
    pub fn name(self) -> &'static str {
        match self {
            Facing::North => "N",
            Facing::East => "E",
            Facing::South => "S",
            Facing::West => "O",
            Facing::Up => "cenital",
        }
    }

    fn sun(self) -> f64 {
        match self {
            Facing::North => SUN_N,
            Facing::East => SUN_E,
            Facing::South => SUN_S,
            Facing::West => SUN_W,
            Facing::Up => SUN_UP,
        }
    }
}

/// A piece of a room's envelope.
#[derive(Debug, Clone, PartialEq)]
pub struct Piece {
    pub kind: Kind,
    /// What it is made of: the wall, slab, roof or opening type.
    pub type_name: String,
    pub area: f64,
    pub u: f64,
    pub facing: Facing,
    /// U of its layers alone (it differs from `u` against the ground).
    pub u_layers: f64,
    /// A window's type, its F_sh,obst in July (1, no shade) and how far
    /// what is over it flies out of the façade, m.
    pub opening_type: Option<OpeningTypeId>,
    pub shade: f64,
    pub overhang: f64,
}

impl Piece {
    fn plain(kind: Kind, type_name: String, area: f64, u: f64, facing: Facing) -> Self {
        Piece {
            kind,
            type_name,
            area,
            u,
            facing,
            u_layers: u,
            opening_type: None,
            shade: 1.0,
            overhang: 0.0,
        }
    }
}

/// A length of linear thermal bridge.
#[derive(Debug, Clone, PartialEq)]
pub struct Bridge {
    pub junction: Junction,
    pub insulation: Insulation,
    pub length: f64,
    pub psi: f64,
}

/// A class of bridge over the whole envelope.
#[derive(Debug, Clone, PartialEq)]
pub struct BridgeSum {
    pub junction: Junction,
    pub insulation: Insulation,
    pub length: f64,
    pub psi: f64,
    /// ψ·L, W/K.
    pub h: f64,
    /// Its fRsi, against `f_min`.
    pub f_rsi: f64,
    pub f_min: f64,
}

impl BridgeSum {
    pub fn condenses(&self) -> bool {
        self.f_rsi < self.f_min - 1e-9
    }
}

/// The solar control of July (HE 1, 3.1.2).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct SolarControl {
    /// By orientation (N, E, S, O, cenital): the glazed area, the mean
    /// shade factor and the solar gain of the month, kWh.
    pub by_facing: Vec<(Facing, f64, f64, f64)>,
    /// The useful area of the conditioned rooms, m².
    pub useful: f64,
    /// q_sol;jul and its limit, kWh/m²·month.
    pub q: f64,
    pub limit: f64,
    pub residential: bool,
    /// Windows with something drawn over them (a slab, a balcony, an
    /// eave), of all the windows (every one has its reveal too).
    pub overhung: usize,
    pub windows: usize,
}

impl SolarControl {
    pub fn fails(&self) -> bool {
        self.useful > 0.0 && self.q > self.limit + 1e-9
    }
}

/// A window type's air permeability against table 3.1.3.a.
#[derive(Debug, Clone, PartialEq)]
pub struct AirWindow {
    pub opening_type: OpeningTypeId,
    pub type_name: String,
    pub class: u8,
    pub q100: f64,
    pub limit: f64,
    pub area: f64,
}

impl AirWindow {
    pub fn fails(&self) -> bool {
        self.q100 > self.limit + 1e-9
    }
}

/// n50 (HE 1, 3.1.3.b and anejo H).
#[derive(Debug, Clone, PartialEq)]
pub struct Airtightness {
    pub windows: Vec<AirWindow>,
    /// Whether 3.1.3.b applies: dwellings of more than 120 m² useful.
    pub applies: bool,
    pub n50: f64,
    /// Measured (given) rather than worked out by anejo H.
    pub measured: bool,
    pub limit: f64,
    /// Opaque area of anejo H and its flow at 100 Pa, m² and m³/h·m².
    pub opaque_area: f64,
    pub opaque_air: f64,
}

impl Airtightness {
    pub fn n50_fails(&self) -> bool {
        self.applies && self.n50 > self.limit + 1e-9
    }

    pub fn fails(&self) -> usize {
        self.windows.iter().filter(|w| w.fails()).count() + self.n50_fails() as usize
    }
}

/// Surface condensation of an opaque type.
#[derive(Debug, Clone, PartialEq)]
pub struct Condensation {
    pub kind: Kind,
    pub type_name: String,
    pub f_rsi: f64,
    pub f_min: f64,
}

impl Condensation {
    pub fn fails(&self) -> bool {
        self.f_rsi < self.f_min - 1e-9
    }
}

/// A room's loads.
#[derive(Debug, Clone, PartialEq)]
pub struct RoomLoad {
    pub room: RoomId,
    pub name: String,
    pub level: LevelId,
    pub area: f64,
    pub volume: f64,
    pub pieces: Vec<Piece>,
    /// Its thermal bridges.
    pub bridges: Vec<Bridge>,
    /// Ventilation inflow, l/s.
    pub ventilation: f64,
    /// Heating: transmission (with the bridges), ventilation and total, W;
    /// what the bridges are of the transmission.
    pub heat_transmission: f64,
    pub heat_bridges: f64,
    pub heat_ventilation: f64,
    pub heating: f64,
    /// Cooling: transmission, sun, internal, ventilation, latent, total, W.
    pub cool_transmission: f64,
    pub cool_sun: f64,
    pub cool_internal: f64,
    pub cool_ventilation: f64,
    pub cool_latent: f64,
    pub cooling: f64,
    /// Whether it is lived in (a dry room): it gets an indoor unit.
    pub dry: bool,
    /// The indoor unit that covers it, kW.
    pub unit: Option<f64>,
    /// The radiators drawn in it and what they give, W.
    pub radiators: Vec<DeviceId>,
    pub radiator_power: f64,
}

impl RoomLoad {
    /// Radiators drawn that do not reach its heating load.
    pub fn radiators_short(&self) -> bool {
        !self.radiators.is_empty() && self.radiator_power + 1.0 < self.heating
    }
}

/// A group of rooms on one machine: a dwelling, or a room on its own.
#[derive(Debug, Clone, PartialEq)]
pub struct Group {
    pub name: String,
    pub rooms: Vec<RoomId>,
    pub heating: f64,
    pub cooling: f64,
    /// The heat pump, kW.
    pub heat_pump: f64,
}

/// A type of the envelope against its DB-HE 1 limit.
#[derive(Debug, Clone, PartialEq)]
pub struct Element {
    pub kind: Kind,
    pub type_name: String,
    pub u: f64,
    pub limit: f64,
    pub area: f64,
    /// U of its layers alone (it differs against the ground).
    pub u_layers: f64,
}

impl Element {
    pub fn fails(&self) -> bool {
        self.u > self.limit + 1e-6
    }
}

/// The whole calculation.
#[derive(Debug, Clone, PartialEq)]
pub struct Thermal {
    pub site: ThermalSite,
    pub rooms: Vec<RoomLoad>,
    pub groups: Vec<Group>,
    pub elements: Vec<Element>,
    /// The thermal bridges by class, and Σ ψ·L, W/K.
    pub bridges: Vec<BridgeSum>,
    pub psi_l: f64,
    /// Σ U·A of the envelope, W/K.
    pub ua: f64,
    /// The floors on the ground by UNE-EN ISO 13370, one per level.
    pub ground: Vec<(LevelId, env::GroundFloor)>,
    pub solar: SolarControl,
    pub air: Airtightness,
    pub condensation: Vec<Condensation>,
    /// (Σ U·A + Σ ψ·L) / Σ A of the envelope, the heated volume, the
    /// envelope's area and the compactness V/A.
    pub k: f64,
    pub volume: f64,
    pub envelope: f64,
    pub compactness: f64,
    pub k_limit: f64,
}

impl Thermal {
    pub fn heating(&self) -> f64 {
        self.rooms.iter().map(|r| r.heating).sum()
    }

    pub fn cooling(&self) -> f64 {
        self.rooms.iter().map(|r| r.cooling).sum()
    }

    pub fn k_fails(&self) -> bool {
        self.envelope > 0.0 && self.k > self.k_limit + 1e-6
    }

    pub fn fails(&self) -> usize {
        self.elements.iter().filter(|e| e.fails()).count()
            + self.k_fails() as usize
            + self.rooms.iter().filter(|r| r.radiators_short()).count()
            + self.solar.fails() as usize
            + self.air.fails()
            + self.condensation.iter().filter(|c| c.fails()).count()
            + self.bridges.iter().filter(|b| b.condenses()).count()
    }
}

/// The transmittance of some layers between two surface resistances.
pub fn u_value(model: &Model, layers: &[archisim_core::Layer], rsi: f64, rse: f64) -> f64 {
    1.0 / (rsi + model.layers_resistance(layers) + rse)
}

/// K's limit for new dwellings by compactness (straight between V/A 1
/// and 4).
pub fn k_limit(zone: usize, compactness: f64) -> f64 {
    let t = ((compactness - 1.0) / 3.0).clamp(0.0, 1.0);
    K_COMPACT_LOW[zone] + (K_COMPACT_HIGH[zone] - K_COMPACT_LOW[zone]) * t
}

fn pick(sizes: &[f64], kw: f64) -> f64 {
    sizes
        .iter()
        .copied()
        .find(|s| *s * 1000.0 >= kw - 1e-6)
        .unwrap_or(sizes[sizes.len() - 1])
}

/// The orientation of an outward normal in plan.
fn facing(model: &Model, n: Vec2) -> Facing {
    // A bearing in plan (clockwise from up) less true north is from north.
    let plan = n.x.atan2(n.y);
    let az = (plan - model.true_north())
        .rem_euclid(std::f64::consts::TAU)
        .to_degrees();
    match az {
        a if !(45.0..315.0).contains(&a) => Facing::North,
        a if a < 135.0 => Facing::East,
        a if a < 225.0 => Facing::South,
        _ => Facing::West,
    }
}

fn segment_distance(p: Point2, a: Point2, b: Point2) -> (f64, f64) {
    let ab = b - a;
    let l2 = ab.dot(ab);
    if l2 < 1e-12 {
        return (p.distance(a), 0.0);
    }
    let t = ((p - a).dot(ab) / l2).clamp(0.0, 1.0);
    ((a + ab * t).distance(p), t)
}

fn level_after(model: &Model, level: LevelId) -> Option<(LevelId, f64)> {
    let levels = model.levels();
    let i = levels.iter().position(|l| l.id == level)?;
    levels.get(i + 1).map(|l| (l.id, l.elevation))
}

/// The shortest distance from a point to the edges of a polygon.
fn polygon_distance(p: Point2, outline: &[Point2]) -> f64 {
    let n = outline.len();
    (0..n)
        .map(|i| segment_distance(p, outline[i], outline[(i + 1) % n]).0)
        .fold(f64::INFINITY, f64::min)
}

/// What shades a window in July: whatever flies over it (a slab or
/// balcony of the level above, the eaves of a roof), probed out from the
/// façade every 10 cm up to 3 m, and its reveal. Gives F_sh,obst and how
/// far that flies out, m.
#[allow(clippy::too_many_arguments)]
fn window_shade(
    model: &Model,
    above: Option<(LevelId, f64)>,
    face_point: Point2,
    out: Vec2,
    thick: f64,
    reveal: f64,
    top_z: f64,
    height: f64,
    facing: Facing,
    latitude: f64,
) -> (f64, f64) {
    let mut depth = 0.0;
    let mut gap = f64::INFINITY;
    for k in 1..=30 {
        let dist = k as f64 * 0.1;
        let p = face_point + out * (thick + dist);
        let mut over: Option<f64> = None;
        if let Some((up, z_up)) = above {
            for s in model.slabs_on_level(up) {
                if point_in_polygon(p, &s.outline) {
                    let t = model.slab_type(s.slab_type).map_or(0.3, |t| t.thickness());
                    let z = z_up + s.top_offset - t;
                    if z > top_z - 0.01 {
                        over = Some(over.map_or(z, |o: f64| o.min(z)));
                    }
                }
            }
        }
        for r in model.roofs() {
            let z = model.level(r.level).map_or(0.0, |l| l.elevation) + r.base_offset;
            if z < top_z - 0.01 {
                continue;
            }
            if point_in_polygon(p, &r.outline) || polygon_distance(p, &r.outline) <= r.overhang {
                over = Some(over.map_or(z, |o: f64| o.min(z)));
            }
        }
        if let Some(z) = over {
            depth = dist;
            gap = gap.min(z - top_z);
        }
    }
    if depth == 0.0 {
        gap = 0.0;
    }
    (
        env::shade_factor(facing, latitude, height, reveal, reveal + depth, gap),
        depth,
    )
}

/// Whether a room is conditioned: not a common corridor, stair, garage or
/// plant room.
fn conditioned(usage: &str, name: &str) -> bool {
    let class = crate::ventilation::Class::of(usage, name);
    !(class.common && !class.dry_room() && !class.wet)
}

/// What a room needs for its loads besides its pieces.
struct LoadData {
    dry: bool,
    kitchen: bool,
    q: f64,
}

/// Fills in a room's loads from its pieces and bridges.
fn fill_loads(r: &mut RoomLoad, d: &LoadData, site: &ThermalSite, dt_heat: f64, dt_cool: f64) {
    let ua = |pick: &dyn Fn(Kind) -> bool| -> f64 {
        r.pieces
            .iter()
            .filter(|p| pick(p.kind))
            .map(|p| p.u * p.area)
            .sum()
    };
    let psi_all: f64 = r.bridges.iter().map(|b| b.psi * b.length).sum();
    let psi_air: f64 = r
        .bridges
        .iter()
        .filter(|b| b.junction != Junction::Ground)
        .map(|b| b.psi * b.length)
        .sum();
    r.heat_bridges = psi_all * dt_heat;
    r.heat_transmission =
        ua(&|k| !k.on_ground()) * dt_heat + ua(&|k| k.on_ground()) * dt_heat * 0.5 + r.heat_bridges;
    r.heat_ventilation = 1.2 * d.q * dt_heat;
    r.cool_transmission = (ua(&|k| !k.on_ground()) + psi_air) * dt_cool;
    r.cool_sun = r
        .pieces
        .iter()
        .filter(|p| p.kind.glazed())
        .map(|p| p.area * site.glass_g * p.facing.sun())
        .sum();
    let people = if d.dry {
        (r.area / 10.0).max(1.0).round()
    } else {
        0.0
    };
    r.cool_internal = people * PERSON_SENSIBLE
        + r.area
            * (LIGHT_W_M2
                + if d.kitchen {
                    KITCHEN_W_M2
                } else {
                    APPLIANCES_W_M2
                });
    r.cool_latent = people * PERSON_LATENT;
    r.cool_ventilation = 1.2 * d.q * dt_cool;
    r.ventilation = d.q;
    r.heating = r.heat_transmission + r.heat_ventilation;
    r.cooling =
        r.cool_transmission + r.cool_sun + r.cool_internal + r.cool_ventilation + r.cool_latent;
    r.unit = d.dry.then(|| pick(&INDOOR_UNITS, r.cooling.max(r.heating)));
}

/// Works out the envelope and the loads.
pub fn thermal(model: &Model) -> Thermal {
    let site = model.thermal_site().clone();
    let zone = site.winter.index();
    let dt_heat = (site.heated - site.winter_air).max(0.0);
    let dt_cool = (site.summer_air - site.cooled).max(0.0);
    let lambda_g = site.ground_conductivity;
    let latitude = model
        .georeference()
        .and_then(|g| g.lat_lon())
        .map_or(40.0, |(lat, _)| lat);
    // The ventilation of DB-HS 3, and its dwellings.
    let vent = crate::ventilation::ventilation(model);
    let mut inflow: BTreeMap<RoomId, f64> = BTreeMap::new();
    let mut dwelling_of: BTreeMap<RoomId, usize> = BTreeMap::new();
    for (i, d) in vent.dwellings.iter().enumerate() {
        for r in &d.rooms {
            inflow.insert(r.room, r.admission);
            dwelling_of.insert(r.room, i);
        }
    }
    // The outdoor air of the rooms of the RITE.
    for r in &vent.rite.rooms {
        inflow.entry(r.room).or_insert(r.outdoor);
    }
    let levels = model.levels();
    let first = levels.first().map(|l| l.id);
    let mut rooms: Vec<RoomLoad> = Vec::new();
    let mut data: Vec<LoadData> = Vec::new();
    // For UNE-EN ISO 13370, by level: area on the ground, its exposed
    // perimeter (façade and walls to unheated rooms), Σ façade thickness ·
    // length, Σ Rf · area and the façade's length.
    let mut ground_of: BTreeMap<LevelId, (f64, f64, f64, f64, f64)> = BTreeMap::new();
    let mut pillars_seen: BTreeSet<ColumnId> = BTreeSet::new();
    for level in &levels {
        let above = level_after(model, level.id);
        let storey = above.map_or(3.0, |(_, z)| z - level.elevation);
        // How deep the level is under the ±0,00 (the ground outside).
        let depth = (-level.elevation).max(0.0);
        for room in model.rooms_on_level(level.id) {
            let class = crate::ventilation::Class::of(&room.usage, &room.name);
            if !conditioned(&room.usage, &room.name) {
                // Corridors, stairs, garages and plant rooms: not
                // conditioned.
                continue;
            }
            let area = room.area();
            let h = room.height.unwrap_or((storey - 0.3).max(2.4));
            let p = room.label;
            // Its own floor slab (its layers' resistance for the ground).
            let own = model
                .slabs_on_level(level.id)
                .find(|s| point_in_polygon(p, &s.outline))
                .and_then(|s| model.slab_type(s.slab_type));
            let rf = own.map_or(0.0, |t| model.layers_resistance(&t.layers));
            let mut pieces: Vec<Piece> = Vec::new();
            let mut bridges: Vec<Bridge> = Vec::new();
            let add = |bridges: &mut Vec<Bridge>, j: Junction, ins: Insulation, len: f64| {
                if len > 1e-6 {
                    bridges.push(Bridge {
                        junction: j,
                        insulation: ins,
                        length: len,
                        psi: env::psi(model, j, ins),
                    });
                }
            };
            let n = room.outline.len();
            // The edges with a façade wall: their length out and where
            // the wall's insulation is.
            let mut walled: Vec<Option<(f64, Insulation)>> = vec![None; n];
            let mut perimeter_out = 0.0;
            let mut perimeter_unheated = 0.0;
            let mut thick_len = 0.0;
            #[allow(clippy::needless_range_loop)]
            for i in 0..n {
                let (a, b) = (room.outline[i], room.outline[(i + 1) % n]);
                let full = a.distance(b);
                if full < 0.05 {
                    continue;
                }
                let d = (b - a) * (1.0 / full);
                let out = Vec2::new(d.y, -d.x);
                let mid = (a + b) * 0.5;
                // The part of the edge with no room beyond it, by probes
                // every 25 cm (a partition between two rooms beyond would
                // fool a single one).
                let probes = ((full / 0.25).ceil() as usize).max(2);
                let outside = (0..probes)
                    .filter(|k| {
                        let p = a + d * (full * (*k as f64 + 0.5) / probes as f64) + out * 0.6;
                        !model
                            .rooms_on_level(level.id)
                            .any(|r| r.id != room.id && r.contains(p))
                    })
                    .count();
                if outside * 3 < probes {
                    // Against a room that is not conditioned (a portal, a
                    // garage): still the exposed perimeter of a floor on
                    // the ground (UNE-EN ISO 13370).
                    let unheated = (0..probes)
                        .filter(|k| {
                            let p = a + d * (full * (*k as f64 + 0.5) / probes as f64) + out * 0.6;
                            model.rooms_on_level(level.id).any(|r| {
                                r.id != room.id && r.contains(p) && !conditioned(&r.usage, &r.name)
                            })
                        })
                        .count();
                    perimeter_unheated += full * unheated as f64 / probes as f64;
                    continue;
                }
                let len = full * outside as f64 / probes as f64;
                let face = facing(model, out);
                // A curtain wall along it?
                let curtain = model
                    .curtain_walls()
                    .find(|c| c.level == level.id && segment_distance(mid, c.start, c.end).0 < 0.6);
                if curtain.is_some() {
                    // Façade around a floor on the ground all the same (a
                    // curtain wall's depth taken as 15 cm).
                    perimeter_out += len;
                    thick_len += CURTAIN_DEPTH * len;
                    pieces.push(Piece::plain(
                        Kind::Curtain,
                        "Muro cortina".into(),
                        len * h,
                        site.window_u,
                        face,
                    ));
                    continue;
                }
                let wall = model
                    .walls_on_level(level.id)
                    .filter_map(|w| {
                        let t = model.wall_type(w.wall_type)?;
                        let (dist, _) = segment_distance(mid, w.start, w.end);
                        let along = (w.end - w.start).normalized()?;
                        (dist <= t.thickness() + 0.35 && along.cross(d).abs() < 0.2)
                            .then_some((w, t, dist))
                    })
                    .min_by(|x, y| x.2.total_cmp(&y.2));
                let Some((wall, wtype, _)) = wall else {
                    continue;
                };
                // A partition to a space with no room (a stairwell, a
                // shaft) or a party wall against the neighbour: not the
                // envelope to the outside.
                let has_window = model.openings().any(|o| {
                    o.wall == wall.id
                        && model
                            .opening_type(o.opening_type)
                            .is_some_and(|t| matches!(t.kind, OpeningKind::Window(_)))
                });
                if (wtype.function == archisim_core::WallFunction::Interior && !has_window)
                    || wtype.name.to_lowercase().contains("medianer")
                {
                    continue;
                }
                let thick = wtype.thickness();
                let ins = env::insulation_of(model, &wtype.layers);
                walled[i] = Some((len, ins));
                perimeter_out += len;
                thick_len += thick * len;
                let mut opaque = len * h;
                let wdir = (wall.end - wall.start).normalized().unwrap_or(d);
                for o in model.openings().filter(|o| o.wall == wall.id) {
                    let Some(t) = model.opening_type(o.opening_type) else {
                        continue;
                    };
                    let at = wall.start + wdir * o.along;
                    let (dist, s) = segment_distance(at, a, b);
                    if dist > 0.8 || s <= 0.0 || s >= 1.0 {
                        continue;
                    }
                    let kind = match t.kind {
                        OpeningKind::Window(_) => Kind::Window,
                        OpeningKind::Door(_) => Kind::Door,
                        OpeningKind::Void => continue,
                    };
                    let oa = (t.width * t.height).min(opaque);
                    opaque -= oa;
                    // Jambs, head and sill (or threshold).
                    add(
                        &mut bridges,
                        Junction::OpeningPerimeter,
                        ins,
                        2.0 * (t.width + t.height),
                    );
                    let u = if kind == Kind::Window {
                        site.window_u
                    } else {
                        site.door_u
                    };
                    let (shade, overhang) = if kind == Kind::Window {
                        window_shade(
                            model,
                            above,
                            a + (b - a) * s,
                            out,
                            thick,
                            o.frame_setback(thick, t),
                            level.elevation + o.sill + t.height,
                            t.height,
                            face,
                            latitude,
                        )
                    } else {
                        (1.0, 0.0)
                    };
                    pieces.push(Piece {
                        opening_type: Some(o.opening_type),
                        shade,
                        overhang,
                        ..Piece::plain(kind, t.name.clone(), oa, u, face)
                    });
                }
                // Columns in the façade: within the wall's band along the
                // edge, each counted once.
                for c in model.columns_on_level(level.id) {
                    if pillars_seen.contains(&c.id) {
                        continue;
                    }
                    let (_, tt) = segment_distance(c.position, a, b);
                    let side = (c.position - a).dot(out);
                    if tt > 0.0 && tt < 1.0 && side > -0.35 && side < thick + 0.35 {
                        pillars_seen.insert(c.id);
                        add(&mut bridges, Junction::Pillar, ins, h);
                    }
                }
                let u_wall = u_value(model, &wtype.layers, RSI_WALL, RSE);
                // Under the ground, the part of the wall against it by
                // UNE-EN ISO 13370.
                let buried = if depth > 0.05 { depth.min(h) } else { 0.0 };
                let buried_area = (len * buried).min(opaque.max(0.0));
                if buried_area > 1e-6 {
                    let dt = thick + lambda_g * (env::GROUND_RSI + rf + env::GROUND_RSE);
                    let rw = model.layers_resistance(&wtype.layers);
                    pieces.push(Piece {
                        u_layers: u_wall,
                        ..Piece::plain(
                            Kind::GroundWall,
                            wtype.name.clone(),
                            buried_area,
                            env::basement_wall(lambda_g, dt, rw, depth),
                            face,
                        )
                    });
                }
                let above_ground = (opaque - buried_area).max(0.0);
                if above_ground > 1e-6 || buried_area <= 1e-6 {
                    pieces.push(Piece::plain(
                        Kind::Wall,
                        wtype.name.clone(),
                        above_ground,
                        u_wall,
                        face,
                    ));
                }
            }
            // Above: a room over it is inside; otherwise the slab of the
            // level above, or a roof.
            if let Some((up, _)) = above {
                let covered = model.rooms_on_level(up).any(|r| r.contains(p));
                if !covered {
                    let slab = model
                        .slabs_on_level(up)
                        .find(|s| point_in_polygon(p, &s.outline))
                        .and_then(|s| model.slab_type(s.slab_type));
                    let roof = model
                        .roofs()
                        .find(|r| point_in_polygon(p, &r.outline))
                        .and_then(|r| model.slab_type(r.roof_type));
                    if let Some(t) = slab.or(roof) {
                        pieces.push(Piece::plain(
                            Kind::Roof,
                            t.name.clone(),
                            area,
                            u_value(model, &t.layers, RSI_UP, RSE),
                            Facing::Up,
                        ));
                    }
                }
            } else if let Some(t) = model
                .roofs()
                .find(|r| point_in_polygon(p, &r.outline))
                .and_then(|r| model.slab_type(r.roof_type))
            {
                pieces.push(Piece::plain(
                    Kind::Roof,
                    t.name.clone(),
                    area,
                    u_value(model, &t.layers, RSI_UP, RSE),
                    Facing::Up,
                ));
            }
            // Below: the lowest level's slab on the ground (its U is made
            // equivalent below, by UNE-EN ISO 13370), or a floor over
            // nothing.
            let mut on_ground = false;
            if let Some(t) = own {
                if Some(level.id) == first {
                    on_ground = true;
                    pieces.push(Piece::plain(
                        Kind::Ground,
                        t.name.clone(),
                        area,
                        u_value(model, &t.layers, RSI_DOWN, 0.0),
                        Facing::Up,
                    ));
                    let g = ground_of
                        .entry(level.id)
                        .or_insert((0.0, 0.0, 0.0, 0.0, 0.0));
                    g.0 += area;
                    g.1 += perimeter_out + perimeter_unheated;
                    g.2 += thick_len;
                    g.3 += rf * area;
                    g.4 += perimeter_out;
                } else {
                    let below = levels
                        .iter()
                        .rev()
                        .find(|l| l.elevation < level.elevation - 0.1)
                        .map(|l| l.id);
                    let open = below.is_some_and(|b| {
                        !model.rooms_on_level(b).any(|r| r.contains(p))
                            && !model
                                .slabs_on_level(b)
                                .any(|s| point_in_polygon(p, &s.outline))
                    });
                    if open {
                        pieces.push(Piece::plain(
                            Kind::ExposedFloor,
                            t.name.clone(),
                            area,
                            u_value(model, &t.layers, RSI_DOWN, RSE),
                            Facing::Up,
                        ));
                    }
                }
            }
            // The bridges along the façade edges: at the top (the roof) and
            // the bottom (the ground slab, or the front of its own floor),
            // and at the corners between two of them.
            let roofed = pieces.iter().any(|pc| pc.kind == Kind::Roof);
            for (len, ins) in walled.iter().flatten() {
                if roofed {
                    add(&mut bridges, Junction::Roof, *ins, *len);
                }
                if on_ground {
                    add(&mut bridges, Junction::Ground, *ins, *len);
                } else if own.is_some() {
                    add(&mut bridges, Junction::SlabFront, *ins, *len);
                }
            }
            let turn = archisim_core::signed_area(&room.outline).signum();
            for i in 0..n {
                let k = (i + 1) % n;
                let (Some((_, ins)), Some(_)) = (walled[i], walled[k]) else {
                    continue;
                };
                let a = room.outline[i];
                let b = room.outline[k];
                let c = room.outline[(k + 1) % n];
                let (Some(d1), Some(d2)) = ((b - a).normalized(), (c - b).normalized()) else {
                    continue;
                };
                let cross = d1.cross(d2) * turn;
                if cross > 0.2 {
                    add(&mut bridges, Junction::OuterCorner, ins, h);
                } else if cross < -0.2 {
                    add(&mut bridges, Junction::InnerCorner, ins, h);
                }
            }
            // Lived in: a dry room of a dwelling, or any other room that is
            // not wet nor a passage (a shop, an office).
            let dry = class.dry_room() || (!class.wet && !class.passage && !class.common);
            let volume = area * h;
            let q = inflow
                .get(&room.id)
                .copied()
                .filter(|q| *q > 0.0)
                .unwrap_or(if dry { 0.5 * volume / 3.6 } else { 0.0 });
            let mut radiators = Vec::new();
            let mut radiator_power = 0.0;
            for d in model.devices().filter(|d| {
                d.level == level.id && d.kind == DeviceKind::Radiator && room.contains(d.position)
            }) {
                radiators.push(d.id);
                radiator_power += d.width * d.height * RADIATOR_W_M2;
            }
            rooms.push(RoomLoad {
                room: room.id,
                name: room.name.clone(),
                level: level.id,
                area,
                volume,
                pieces,
                bridges,
                ventilation: q,
                heat_transmission: 0.0,
                heat_bridges: 0.0,
                heat_ventilation: 0.0,
                heating: 0.0,
                cool_transmission: 0.0,
                cool_sun: 0.0,
                cool_internal: 0.0,
                cool_ventilation: 0.0,
                cool_latent: 0.0,
                cooling: 0.0,
                dry,
                unit: None,
                radiators,
                radiator_power,
            });
            data.push(LoadData {
                dry,
                kitchen: class.kitchen,
                q,
            });
        }
    }
    // The floors on the ground by UNE-EN ISO 13370: B' with the area and
    // the façade of the whole level, each slab type with its own layers.
    let mut ground = Vec::new();
    for (lvl, (a, per, tl, rfa, out_len)) in ground_of {
        if a <= 0.0 {
            continue;
        }
        // The thickness of the façade around it.
        let w = if out_len > 0.0 { tl / out_len } else { 0.3 };
        let depth = (-model.level(lvl).map_or(0.0, |l| l.elevation)).max(0.0);
        for r in rooms.iter_mut().filter(|r| r.level == lvl) {
            for pc in r.pieces.iter_mut().filter(|pc| pc.kind == Kind::Ground) {
                let rf = (1.0 / pc.u_layers - RSI_DOWN).max(0.0);
                pc.u = env::slab_on_ground(lambda_g, a, per, w, rf, depth, site.edge).u;
            }
        }
        ground.push((
            lvl,
            env::slab_on_ground(lambda_g, a, per, w, rfa / a, depth, site.edge),
        ));
    }
    for (r, d) in rooms.iter_mut().zip(&data) {
        fill_loads(r, d, &site, dt_heat, dt_cool);
    }
    // Groups: a dwelling, or a room on its own.
    let mut groups: Vec<Group> = Vec::new();
    let mut of_dwelling: BTreeMap<usize, usize> = BTreeMap::new();
    for r in &rooms {
        let g = match dwelling_of.get(&r.room) {
            Some(d) => *of_dwelling.entry(*d).or_insert_with(|| {
                groups.push(Group {
                    name: vent.dwellings[*d].name.clone(),
                    rooms: Vec::new(),
                    heating: 0.0,
                    cooling: 0.0,
                    heat_pump: 0.0,
                });
                groups.len() - 1
            }),
            None => {
                groups.push(Group {
                    name: r.name.clone(),
                    rooms: Vec::new(),
                    heating: 0.0,
                    cooling: 0.0,
                    heat_pump: 0.0,
                });
                groups.len() - 1
            }
        };
        groups[g].rooms.push(r.room);
        groups[g].heating += r.heating;
        groups[g].cooling += r.cooling;
    }
    for g in &mut groups {
        g.heat_pump = pick(&HEAT_PUMPS, g.heating.max(g.cooling));
    }
    // DB-HE 1: the types (U by area where it varies, against the ground).
    let mut by_type: BTreeMap<(Kind, String), (f64, f64, f64)> = BTreeMap::new();
    for pc in rooms.iter().flat_map(|r| &r.pieces) {
        let e = by_type
            .entry((pc.kind, pc.type_name.clone()))
            .or_insert((0.0, 0.0, pc.u_layers));
        e.0 += pc.u * pc.area;
        e.1 += pc.area;
    }
    let elements: Vec<Element> = by_type
        .into_iter()
        .filter(|(_, (_, area, _))| *area > 1e-6)
        .map(|((kind, type_name), (ua, area, u_layers))| Element {
            kind,
            type_name,
            u: ua / area,
            limit: kind.limit(zone),
            area,
            u_layers,
        })
        .collect();
    // The bridges by class.
    let f_min = env::f_rsi_min(zone, site.hygrometry);
    let mut by_class: BTreeMap<(Junction, Insulation), (f64, f64)> = BTreeMap::new();
    for b in rooms.iter().flat_map(|r| &r.bridges) {
        let e = by_class
            .entry((b.junction, b.insulation))
            .or_insert((0.0, b.psi));
        e.0 += b.length;
    }
    let bridges: Vec<BridgeSum> = by_class
        .into_iter()
        .map(|((junction, insulation), (length, psi))| BridgeSum {
            junction,
            insulation,
            length,
            psi,
            h: psi * length,
            f_rsi: env::f_rsi_bridge(junction, insulation),
            f_min,
        })
        .collect();
    let psi_l: f64 = bridges.iter().map(|b| b.h).sum();
    let envelope: f64 = elements.iter().map(|e| e.area).sum();
    let ua: f64 = elements.iter().map(|e| e.u * e.area).sum();
    let volume: f64 = rooms.iter().map(|r| r.volume).sum();
    let k = if envelope > 0.0 {
        (ua + psi_l) / envelope
    } else {
        0.0
    };
    let compactness = if envelope > 0.0 {
        volume / envelope
    } else {
        0.0
    };
    // Solar control of July.
    let residential = model.code_settings().building_use == BuildingUse::Dwelling;
    let useful: f64 = rooms.iter().map(|r| r.area).sum();
    let mut solar = SolarControl {
        useful,
        residential,
        limit: if residential {
            env::Q_SOL_LIMIT_DWELLING
        } else {
            env::Q_SOL_LIMIT_OTHER
        },
        ..Default::default()
    };
    let mut gains: BTreeMap<Facing, (f64, f64, f64)> = BTreeMap::new();
    for pc in rooms
        .iter()
        .flat_map(|r| &r.pieces)
        .filter(|pc| pc.kind.glazed())
    {
        let gain = pc.shade
            * site.shaded_g
            * (1.0 - site.frame_fraction)
            * pc.area
            * env::july_radiation(site.summer, pc.facing);
        let e = gains.entry(pc.facing).or_insert((0.0, 0.0, 0.0));
        e.0 += pc.area;
        e.1 += pc.shade * pc.area;
        e.2 += gain;
        if pc.kind == Kind::Window {
            solar.windows += 1;
            if pc.overhang > 0.0 {
                solar.overhung += 1;
            }
        }
    }
    solar.by_facing = gains
        .into_iter()
        .map(|(f, (a, sa, g))| (f, a, if a > 0.0 { sa / a } else { 1.0 }, g))
        .collect();
    let gain: f64 = solar.by_facing.iter().map(|x| x.3).sum();
    solar.q = if useful > 0.0 { gain / useful } else { 0.0 };
    // Air permeability: each window type, and n50.
    let mut by_window: BTreeMap<OpeningTypeId, AirWindow> = BTreeMap::new();
    let mut flows: Vec<(f64, f64)> = Vec::new();
    for pc in rooms.iter().flat_map(|r| &r.pieces) {
        let class = match (pc.kind, pc.opening_type) {
            (Kind::Window | Kind::Door, Some(t)) => site.air_class_of(t),
            (Kind::Curtain, _) => site.air_class,
            _ => continue,
        };
        flows.push((env::q100(class), pc.area));
        if let (Kind::Window, Some(t)) = (pc.kind, pc.opening_type) {
            let w = by_window.entry(t).or_insert_with(|| AirWindow {
                opening_type: t,
                type_name: pc.type_name.clone(),
                class,
                q100: env::q100(class),
                limit: env::Q100_LIMIT[zone],
                area: 0.0,
            });
            w.area += pc.area;
        }
    }
    let opaque_area: f64 = elements
        .iter()
        .filter(|e| e.kind.opaque_to_air())
        .map(|e| e.area)
        .sum();
    let air = Airtightness {
        windows: by_window.into_values().collect(),
        applies: residential && useful > env::N50_FROM_AREA,
        n50: site
            .n50
            .unwrap_or_else(|| env::n50(site.opaque_air, opaque_area, &flows, volume)),
        measured: site.n50.is_some(),
        limit: env::n50_limit(compactness),
        opaque_area,
        opaque_air: site.opaque_air,
    };
    // Surface condensation of the opaque types.
    let condensation = elements
        .iter()
        .filter(|e| e.kind.opaque_to_air() || e.kind == Kind::GroundWall)
        .map(|e| Condensation {
            kind: e.kind,
            type_name: e.type_name.clone(),
            f_rsi: env::f_rsi(e.u_layers),
            f_min,
        })
        .collect();
    Thermal {
        site,
        rooms,
        groups,
        elements,
        bridges,
        psi_l,
        ua,
        ground,
        solar,
        air,
        condensation,
        k,
        volume,
        envelope,
        compactness,
        k_limit: k_limit(zone, compactness),
    }
}

fn dec(v: f64, n: usize) -> String {
    let s = format!("{v:.n$}").replace('.', ",");
    // No «-0».
    if s.trim_start_matches('-')
        .chars()
        .all(|c| c == '0' || c == ',')
    {
        s.trim_start_matches('-').to_string()
    } else {
        s
    }
}

/// The calculation in words.
pub fn report(model: &Model, t: &Thermal) -> String {
    use std::fmt::Write as _;
    let mut s = String::new();
    let site = &t.site;
    let _ = writeln!(
        s,
        "ENVOLVENTE (DB-HE 1) Y CARGAS TÉRMICAS — zona climática {}; exterior {} °C en invierno y {} °C en verano; interior {} y {} °C",
        site.zone(),
        dec(site.winter_air, 1),
        dec(site.summer_air, 1),
        dec(site.heated, 0),
        dec(site.cooled, 0)
    );
    let _ = writeln!(s, "\nTransmitancias (tabla 3.1.1.a-HE1)");
    for e in &t.elements {
        let _ = writeln!(
            s,
            "  {} «{}»: U = {} W/m²K (≤ {}), {} m²{}{}",
            e.kind.name(),
            e.type_name,
            dec(e.u, 2),
            dec(e.limit, 2),
            dec(e.area, 1),
            if e.kind.on_ground() {
                format!(" (solo capas {}; UNE-EN ISO 13370)", dec(e.u_layers, 2))
            } else {
                String::new()
            },
            if e.fails() { " — NO CUMPLE" } else { "" }
        );
    }
    let level_name = |l: LevelId| model.level(l).map_or("—".to_string(), |x| x.name.clone());
    for (l, g) in &t.ground {
        let _ = writeln!(
            s,
            "  Suelo sobre el terreno de {} (UNE-EN ISO 13370): A = {} m², P = {} m, B' = {} m, w = {} m, dt = {} m{}{}; U = {} W/m²K (solo capas {})",
            level_name(*l),
            dec(g.area, 1),
            dec(g.perimeter, 1),
            dec(g.b, 2),
            dec(g.w, 2),
            dec(g.dt, 2),
            if g.depth > 0.0 {
                format!(", a {} m bajo el terreno", dec(g.depth, 2))
            } else {
                String::new()
            },
            if g.edge_psi != 0.0 {
                format!(", aislamiento perimetral ψ = {} W/mK", dec(g.edge_psi, 3))
            } else {
                String::new()
            },
            dec(g.u, 2),
            dec(g.u_layers, 2)
        );
    }
    if !t.bridges.is_empty() {
        let _ = writeln!(
            s,
            "\nPuentes térmicos (ψ habituales del DA DB-HE/3 o los del proyecto)"
        );
        for b in &t.bridges {
            let _ = writeln!(
                s,
                "  {} ({}): {} m × ψ {} W/mK = {} W/K; fRsi {} (≥ {}){}",
                b.junction.name(),
                b.insulation.name(),
                dec(b.length, 1),
                dec(b.psi, 2),
                dec(b.h, 1),
                dec(b.f_rsi, 2),
                dec(b.f_min, 2),
                if b.condenses() { " — CONDENSA" } else { "" }
            );
        }
    }
    let _ = writeln!(
        s,
        "  K global: ({} + {}) / {} m² = {} W/m²K (≤ {} con V/A = {} m: {} m³ / {} m²; tabla 3.1.1.b-HE1){}",
        dec(t.ua, 1),
        dec(t.psi_l, 1),
        dec(t.envelope, 0),
        dec(t.k, 2),
        dec(t.k_limit, 2),
        dec(t.compactness, 2),
        dec(t.volume, 0),
        dec(t.envelope, 0),
        if t.k_fails() { " — NO CUMPLE" } else { "" }
    );
    let sol = &t.solar;
    let _ = writeln!(
        s,
        "\nControl solar (3.1.2): factor solar con la protección móvil {}, marco {} %",
        dec(site.shaded_g, 2),
        dec(site.frame_fraction * 100.0, 0)
    );
    for (f, a, sh, g) in &sol.by_facing {
        let _ = writeln!(
            s,
            "  {}: {} m² de vidrio, H julio {} kWh/m², sombra media {} → {} kWh",
            f.name(),
            dec(*a, 1),
            dec(env::july_radiation(site.summer, *f), 0),
            dec(*sh, 2),
            dec(*g, 0)
        );
    }
    let _ = writeln!(
        s,
        "  q sol;jul = {} kWh/m²·mes sobre {} m² útiles (≤ {}, {}){}; {} de {} ventanas con un forjado, balcón o alero encima (todas con la sombra de su retranqueo)",
        dec(sol.q, 2),
        dec(sol.useful, 1),
        dec(sol.limit, 2),
        if sol.residential {
            "residencial privado"
        } else {
            "otros usos"
        },
        if sol.fails() { " — NO CUMPLE" } else { "" },
        sol.overhung,
        sol.windows
    );
    let air = &t.air;
    let _ = writeln!(s, "\nPermeabilidad al aire (3.1.3)");
    for w in &air.windows {
        let _ = writeln!(
            s,
            "  «{}» ({} m²): clase {}, {} m³/h·m² a 100 Pa (≤ {}, tabla 3.1.3.a){}",
            w.type_name,
            dec(w.area, 1),
            w.class,
            dec(w.q100, 0),
            dec(w.limit, 0),
            if w.fails() { " — NO CUMPLE" } else { "" }
        );
    }
    let _ = writeln!(
        s,
        "  n50 = {} h⁻¹ ({}){}",
        dec(air.n50, 2),
        if air.measured {
            "medida".to_string()
        } else {
            format!(
                "anejo H: 0,629·({} × {} m² opacos + huecos) / {} m³",
                dec(air.opaque_air, 0),
                dec(air.opaque_area, 0),
                dec(t.volume, 0)
            )
        },
        if air.applies {
            format!(
                "; ≤ {} (tabla 3.1.3.b){}",
                dec(air.limit, 2),
                if air.n50_fails() {
                    " — NO CUMPLE"
                } else {
                    ""
                }
            )
        } else {
            "; la tabla 3.1.3.b es para viviendas de más de 120 m² útiles".to_string()
        }
    );
    let _ = writeln!(
        s,
        "\nCondensaciones superficiales (higrometría {}; fRsi,min {})",
        site.hygrometry,
        dec(env::f_rsi_min(site.winter.index(), site.hygrometry), 2)
    );
    for c in &t.condensation {
        let _ = writeln!(
            s,
            "  {} «{}»: fRsi {} (≥ {}){}",
            c.kind.name(),
            c.type_name,
            dec(c.f_rsi, 2),
            dec(c.f_min, 2),
            if c.fails() { " — CONDENSA" } else { "" }
        );
    }
    let _ = writeln!(s, "\nCargas por local (W)");
    for r in &t.rooms {
        let _ = writeln!(
            s,
            "  {} ({}, {} m²): calefacción {} (transmisión {}, de ella puentes {}, + ventilación {}); refrigeración {} (transmisión {}, sol {}, interior {}, ventilación {}, latente {}){}{}",
            r.name,
            level_name(r.level),
            dec(r.area, 1),
            dec(r.heating, 0),
            dec(r.heat_transmission, 0),
            dec(r.heat_bridges, 0),
            dec(r.heat_ventilation, 0),
            dec(r.cooling, 0),
            dec(r.cool_transmission, 0),
            dec(r.cool_sun, 0),
            dec(r.cool_internal, 0),
            dec(r.cool_ventilation, 0),
            dec(r.cool_latent, 0),
            r.unit
                .map_or(String::new(), |u| format!("; unidad interior {} kW", dec(u, 1))),
            if r.radiators.is_empty() {
                String::new()
            } else {
                format!(
                    "; radiadores {} W{}",
                    dec(r.radiator_power, 0),
                    if r.radiators_short() { " — NO BASTAN" } else { "" }
                )
            }
        );
    }
    let _ = writeln!(s, "\nEquipos");
    for g in &t.groups {
        let _ = writeln!(
            s,
            "  {}: calefacción {} kW, refrigeración {} kW → bomba de calor de {} kW",
            g.name,
            dec(g.heating / 1000.0, 2),
            dec(g.cooling / 1000.0, 2),
            dec(g.heat_pump, 1)
        );
    }
    let _ = writeln!(
        s,
        "\nTotal: calefacción {} kW, refrigeración {} kW",
        dec(t.heating() / 1000.0, 1),
        dec(t.cooling() / 1000.0, 1)
    );
    s
}

#[cfg(test)]
mod tests {
    use super::*;
    use archisim_core::NewSlab;

    #[test]
    fn the_limits() {
        assert!((k_limit(1, 0.5) - 0.60).abs() < 1e-9);
        assert!((k_limit(1, 4.5) - 0.80).abs() < 1e-9);
        assert!((k_limit(1, 2.5) - 0.70).abs() < 1e-9);
        assert_eq!(pick(&HEAT_PUMPS, 4200.0), 5.0);
    }

    /// The flat of the ventilation, with a roof slab over it and the
    /// ground slab under it.
    #[test]
    fn a_flat_has_its_loads() {
        let mut m = Model::new("Piso");
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
        let square = vec![
            Point2::new(0.0, 0.0),
            Point2::new(10.0, 0.0),
            Point2::new(10.0, 6.0),
            Point2::new(0.0, 6.0),
        ];
        m.add_slab(NewSlab {
            slab_type: lib.ground_slab,
            level: l0,
            outline: square.clone(),
            top_offset: 0.0,
        })
        .unwrap();
        m.add_slab(NewSlab {
            slab_type: lib.flat_roof,
            level: l1,
            outline: square,
            top_offset: 0.0,
        })
        .unwrap();
        let t = thermal(&m);
        assert_eq!(
            t.rooms.len(),
            6,
            "{:?}",
            t.rooms.iter().map(|r| &r.name).collect::<Vec<_>>()
        );
        let living = t.rooms.iter().find(|r| r.name == "Estar").unwrap();
        // Its south wall, its west wall, the roof and the ground.
        assert!(living
            .pieces
            .iter()
            .any(|p| p.kind == Kind::Wall && p.facing == Facing::South));
        assert!(living.pieces.iter().any(|p| p.kind == Kind::Roof));
        assert!(living.pieces.iter().any(|p| p.kind == Kind::Ground));
        assert!(living.heating > 0.0 && living.cooling > 0.0);
        // The corridor has no outside wall but its ends.
        let corridor = t.rooms.iter().find(|r| r.name == "Pasillo").unwrap();
        assert!(
            corridor
                .pieces
                .iter()
                .filter(|p| p.kind == Kind::Wall)
                .count()
                <= 2
        );
        // The façade type: its U is under 1 W/m²K and the elements are
        // checked against zone A.
        let wall = t.elements.iter().find(|e| e.kind == Kind::Wall).unwrap();
        assert!(wall.u > 0.1 && wall.u < 1.0, "{}", wall.u);
        assert!((wall.limit - 0.70).abs() < 1e-9);
        assert!(t.k > 0.0 && t.compactness > 0.5);
        assert_eq!(t.groups.len(), 1, "one dwelling");
        assert!(t.groups[0].heat_pump >= 3.5);
        let text = report(&m, &t);
        if std::env::var("ARCHISIM_CLIMA_TEXTO").is_ok() {
            println!("{text}");
        }
        assert!(text.contains("tabla 3.1.1.a-HE1"));
    }

    /// A box drawn by hand: 10 × 6 m between wall axes, one living room
    /// inside the façade, a 1 m balcony over its south window and a column
    /// in the middle of the south wall. Lengths and figures checked by
    /// hand.
    #[test]
    fn a_box_by_hand() {
        use archisim_core::{
            Hand, NewColumn, NewOpening, NewRoom, NewWall, ReferenceLine, WallSide, WallTop,
        };
        let mut m = Model::new("Caja");
        let l0 = m.add_level("Planta baja", 0.0);
        let l1 = m.add_level("Cubierta", 3.0);
        let lib = archisim_library::add_library(&mut m).unwrap();
        let t = m.wall_type(lib.facade).unwrap().thickness();
        let mut walls = Vec::new();
        for (a, b) in [
            ((0.0, 0.0), (10.0, 0.0)),
            ((10.0, 0.0), (10.0, 6.0)),
            ((10.0, 6.0), (0.0, 6.0)),
            ((0.0, 6.0), (0.0, 0.0)),
        ] {
            walls.push(
                m.add_wall(NewWall {
                    wall_type: lib.facade,
                    level: l0,
                    start: Point2::new(a.0, a.1),
                    end: Point2::new(b.0, b.1),
                    bulge: 0.0,
                    reference: ReferenceLine::Center,
                    height: 3.0,
                    base_offset: 0.0,
                    top: WallTop::Unconnected,
                })
                .unwrap(),
            );
        }
        let h = t / 2.0;
        let room = m
            .add_room(NewRoom {
                level: l0,
                number: "1".into(),
                name: "Estar".into(),
                outline: vec![
                    Point2::new(h, h),
                    Point2::new(10.0 - h, h),
                    Point2::new(10.0 - h, 6.0 - h),
                    Point2::new(h, 6.0 - h),
                ],
                holes: Vec::new(),
                label: Point2::new(5.0, 3.0),
                bounded: false,
            })
            .unwrap();
        let mut r = m.room(room).unwrap().clone();
        r.usage = "Estar".into();
        m.set_room(r).unwrap();
        let window = m
            .add_opening(NewOpening {
                opening_type: lib.window_two_leaves,
                wall: walls[0],
                along: 3.0,
                sill: 0.9,
                opens_to: WallSide::Interior,
                hand: Hand::Left,
                setback: Some(0.05),
            })
            .unwrap();
        m.add_column(NewColumn {
            member_type: lib.column,
            level: l0,
            position: Point2::new(7.0, 0.0),
            rotation: 0.0,
            base_offset: 0.0,
            top: WallTop::Unconnected,
            height: 3.0,
        })
        .unwrap();
        let box_outline = |y0: f64| {
            vec![
                Point2::new(0.0, y0),
                Point2::new(10.0, y0),
                Point2::new(10.0, 6.0),
                Point2::new(0.0, 6.0),
            ]
        };
        m.add_slab(NewSlab {
            slab_type: lib.ground_slab,
            level: l0,
            outline: box_outline(0.0),
            top_offset: 0.0,
        })
        .unwrap();
        // The roof slab flies 1,05 m out of the south face: a balcony.
        m.add_slab(NewSlab {
            slab_type: lib.flat_roof,
            level: l1,
            outline: box_outline(-h - 1.05),
            top_offset: 0.0,
        })
        .unwrap();
        let th = thermal(&m);
        assert_eq!(th.rooms.len(), 1);
        let living = &th.rooms[0];
        let (lx, ly) = (10.0 - t, 6.0 - t);
        let (area, per) = (lx * ly, 2.0 * (lx + ly));
        let storey = 2.7;
        let ins = env::insulation_of(&m, &m.wall_type(lib.facade).unwrap().layers);
        let wt = m.opening_type(lib.window_two_leaves).unwrap().clone();
        let length = |j: Junction| -> f64 {
            living
                .bridges
                .iter()
                .filter(|b| b.junction == j)
                .map(|b| b.length)
                .sum()
        };
        let near = |a: f64, b: f64| (a - b).abs() < 1e-6;
        // Four outward corners, the window's jambs, head and sill, the
        // column, and the roof and ground along the whole façade.
        assert!(near(length(Junction::OuterCorner), 4.0 * storey));
        assert!(near(length(Junction::InnerCorner), 0.0));
        assert!(near(
            length(Junction::OpeningPerimeter),
            2.0 * (wt.width + wt.height)
        ));
        assert!(near(length(Junction::Pillar), storey));
        assert!(near(length(Junction::Roof), per));
        assert!(near(length(Junction::Ground), per));
        assert!(near(length(Junction::SlabFront), 0.0));
        assert!(living.bridges.iter().all(|b| b.insulation == ins));
        let psi_l = 4.0 * storey * env::usual_psi(Junction::OuterCorner, ins)
            + 2.0 * (wt.width + wt.height) * env::usual_psi(Junction::OpeningPerimeter, ins)
            + storey * env::usual_psi(Junction::Pillar, ins)
            + per * env::usual_psi(Junction::Roof, ins)
            + per * env::usual_psi(Junction::Ground, ins);
        assert!(near(th.psi_l, psi_l), "{} {}", th.psi_l, psi_l);
        assert!(near(th.k, (th.ua + th.psi_l) / th.envelope));
        assert!(near(living.heat_bridges, psi_l * 17.0));
        // The ground by UNE-EN ISO 13370: B' = A/(0,5·P), w the façade's.
        assert_eq!(th.ground.len(), 1);
        let g = &th.ground[0].1;
        assert!(near(g.area, area) && near(g.perimeter, per));
        assert!(near(g.b, area / (0.5 * per)));
        assert!(near(g.w, t));
        let floor = living
            .pieces
            .iter()
            .find(|p| p.kind == Kind::Ground)
            .unwrap();
        assert!(near(floor.u, g.u) && floor.u < floor.u_layers);
        // The south window: 1 m of balcony over it, its 5 cm reveal.
        let w = living
            .pieces
            .iter()
            .find(|p| p.kind == Kind::Window)
            .unwrap();
        assert_eq!(w.facing, Facing::South);
        assert!(near(w.overhang, 1.0), "{}", w.overhang);
        let top = 0.9 + wt.height;
        let slab_t = m.slab_type(lib.flat_roof).unwrap().thickness();
        let shade = env::shade_factor(
            Facing::South,
            40.0,
            wt.height,
            0.05,
            1.05,
            3.0 - slab_t - top,
        );
        assert!(near(w.shade, shade) && shade < 1.0);
        let q = shade * 0.15 * 0.75 * w.area * 95.0 / area;
        assert!(near(th.solar.q, q), "{} {}", th.solar.q, q);
        assert_eq!(th.solar.overhung, 1);
        // Permeability: class 3 passes in A; class 1 does not.
        assert_eq!(th.air.windows.len(), 1);
        assert!(!th.air.windows[0].fails());
        assert!(!th.air.applies, "under 120 m²");
        let opaque: f64 = th
            .elements
            .iter()
            .filter(|e| e.kind.opaque_to_air())
            .map(|e| e.area)
            .sum();
        let n50 = 0.629 * (16.0 * opaque + 9.0 * w.area) / th.volume;
        assert!(near(th.air.n50, n50));
        // Surface condensation of the façade.
        let wall = th
            .condensation
            .iter()
            .find(|c| c.kind == Kind::Wall)
            .unwrap();
        let u = th.elements.iter().find(|e| e.kind == Kind::Wall).unwrap().u;
        assert!(near(wall.f_rsi, 1.0 - 0.25 * u));
        assert!(near(wall.f_min, 0.50));
        // The project's ψ for a class, a window class 1, edge insulation.
        let mut site = m.thermal_site().clone();
        site.psi
            .insert(archisim_core::psi_key(Junction::Roof, ins), 1.0);
        site.air_classes.insert(window_type(&m, window), 1);
        site.edge = Some(archisim_core::EdgeInsulation {
            vertical: true,
            size: 0.6,
            resistance: 1.8,
            thickness: 0.06,
        });
        m.set_thermal_site(site).unwrap();
        let th2 = thermal(&m);
        let roof_psi = env::usual_psi(Junction::Roof, ins);
        assert!(near(th2.psi_l - th.psi_l, per * (1.0 - roof_psi)));
        assert!(th2.air.windows[0].fails());
        assert!(th2.ground[0].1.u < g.u);
        let text = report(&m, &th2);
        if std::env::var("ARCHISIM_CLIMA_TEXTO").is_ok() {
            println!("{text}");
        }
        assert!(text.contains("Puentes térmicos") && text.contains("q sol;jul"));
    }

    fn window_type(m: &Model, o: archisim_core::OpeningId) -> OpeningTypeId {
        m.openings().find(|x| x.id == o).unwrap().opening_type
    }

    /// A basement under the ±0,00: its walls against the ground.
    #[test]
    fn a_basement_has_walls_against_the_ground() {
        let mut m = Model::new("Sótano");
        let lb = m.add_level("Sótano", -3.0);
        let l0 = m.add_level("Planta baja", 0.0);
        let lib = archisim_library::add_library(&mut m).unwrap();
        crate::ventilation::example_flat(
            &mut m,
            lb,
            l0,
            lib.partition,
            lib.facade,
            lib.interior_door,
        );
        let square = vec![
            Point2::new(0.0, 0.0),
            Point2::new(10.0, 0.0),
            Point2::new(10.0, 6.0),
            Point2::new(0.0, 6.0),
        ];
        m.add_slab(NewSlab {
            slab_type: lib.ground_slab,
            level: lb,
            outline: square,
            top_offset: 0.0,
        })
        .unwrap();
        let t = thermal(&m);
        assert!(t.elements.iter().any(|e| e.kind == Kind::GroundWall));
        assert!(t.elements.iter().all(|e| e.kind != Kind::Wall));
        assert!(t.ground[0].1.depth > 2.9);
    }
}
