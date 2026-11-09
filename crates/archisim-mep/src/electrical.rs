//! The electrical installation of a dwelling or a building, by the REBT,
//! on what is drawn: the boards («cuadro general de mando y protección»,
//! devices of the kind `Panel`), the mechanisms with their circuits (C1 to
//! C10, ITC-BT-25 tabla 1) and the appliances that take a circuit of their
//! own (the washing machine and the dishwasher, on C4).
//!
//! For each board:
//!
//! - what it serves (`BoardUse`): a dwelling, the general services, a
//!   shop or office, or a garage, by its name or the rooms around it;
//! - the **degree of electrification** of a dwelling (ITC-BT-10): the one
//!   given, or high with more than 160 m² of rooms, more points than the
//!   basic circuits take, or a mechanism on a circuit of the high ones;
//! - its **power**: the degree's for a dwelling, the sum of its circuits
//!   for the general services, and so many W/m² (with a minimum) for a
//!   shop or a garage;
//! - its **supply**: single phase at 230 V, or three phase at 3×400/230 V
//!   when the power passes 14 490 W (or when it is chosen), with the
//!   circuits shared out over the three phases, the heaviest first on the
//!   lightest phase, and the load of each phase and its unbalance;
//! - every **circuit**: its points (C1 to C5 always in a dwelling, with
//!   one point when none is drawn), its design power P = n·Pa·Fs·Fu and
//!   current I = P/U, its breaker, section and tube (tabla 1), the current
//!   its wires carry (UNE-HD 60364-5-52, tabla B.52.2, method B1: in a tube
//!   in the wall, PVC), and its **route**: the wires go from the board to
//!   the nearest point and on from point to point (the shortest tree, along
//!   the floor and up to the ceiling as wires go); the voltage drop is
//!   added stretch by stretch, each stretch with the power of the points
//!   after it, ΔU = 2·L·P/(γ·S·U²), up to the point that drops most,
//!   against 3 % (ITC-BT-19 2.2.2);
//! - the **short circuit** (the simplified method of the technical guide,
//!   Icc = 0,8·U/R, with the resistance of the phase conductors at 20 °C
//!   from the transformer or from a current given at the supply): at the
//!   board, against the breaking capacity of the breakers, and at the end
//!   of each circuit, against the magnetic trip of its curve (B 5·In,
//!   C 10·In, D 20·In); a circuit takes the section that drops what it may
//!   and trips at its end;
//! - the **main switch** (25 A basic, 40 A high, or what the current asks
//!   for; four poles in three phase), the **residual current devices**
//!   (one for every five circuits, ITC-BT-25 2.3) and the **surge
//!   protector** (ITC-BT-23);
//! - the **feeder** from the meter (ITC-BT-15): the section that carries
//!   its current and drops what it may, 6 mm² at least; measured on the
//!   drawing from the meters when they are centralised.
//!
//! For the building (ITC-BT-10 and 14): the forecast of its loads (the
//! dwellings by their mean power times the coefficient of simultaneity of
//! their number, the general services, the shops and the garage) and the
//! **general feeder** («línea general de alimentación») from the main fuse
//! box to the meters, with the dwellings shared out over its phases.
//!
//! Copper at 70 °C (γ = 48 m/(Ω·mm²)) for the voltage drops; cos φ = 1 in
//! the single-phase lines of a dwelling and 0,9 in the three-phase ones.

use std::collections::BTreeMap;

use archisim_core::{
    circuit_spec, CircuitSpec, DeviceId, DeviceKind, Electrification, LevelId, Model, ObjectId,
    Point2, Supply, TripCurve, CIRCUITS,
};

// The values of the REBT, its technical guide and the UNE tables, written
// from memory: para revisar.

/// The voltage between a phase and the neutral, V.
pub const VOLTAGE: f64 = 230.0;
/// The voltage between phases, V.
pub const LINE_VOLTAGE: f64 = 400.0;
/// The conductivity of copper at 70 °C, m/(Ω·mm²).
pub const GAMMA: f64 = 48.0;
/// The conductivity of copper at 20 °C, for the short circuit.
pub const GAMMA_20: f64 = 56.0;
/// The conductivity of aluminium at 20 °C (the company's line).
pub const GAMMA_AL_20: f64 = 35.0;
/// The section of the company's line from the transformer, mm² Al.
pub const NETWORK_SECTION: f64 = 150.0;
/// The factor of the simplified short circuit, Icc = 0,8·U/R (guide).
pub const FAULT_FACTOR: f64 = 0.8;
/// Over this power, three phase (ITC-BT-10: 14 490 W = 63 A at 230 V).
pub const THREE_PHASE_FROM: f64 = 14_490.0;
/// The power factor of the three-phase lines.
pub const COS_PHI: f64 = 0.9;
/// The most a circuit of a dwelling may drop, from the board (ITC-BT-19).
pub const CIRCUIT_DROP: f64 = 0.03;
/// The feeder of a single user, with no general feeder (ITC-BT-15).
pub const FEEDER_DROP_SINGLE_USER: f64 = 0.015;
/// The feeder with the meters all in one place.
pub const FEEDER_DROP_CENTRALISED: f64 = 0.01;
/// The feeder with the meters in more than one place.
pub const FEEDER_DROP_PARTLY: f64 = 0.005;
/// The general feeder with the meters all in one place (ITC-BT-14).
pub const GENERAL_DROP_CENTRALISED: f64 = 0.005;
/// The general feeder with the meters in more than one place.
pub const GENERAL_DROP_PARTLY: f64 = 0.01;
/// The least section of a feeder, mm² (ITC-BT-15).
pub const FEEDER_MIN_SECTION: f64 = 6.0;
/// The least section of the general feeder, mm² (ITC-BT-14).
pub const GENERAL_MIN_SECTION: f64 = 10.0;
/// The coefficient of simultaneity of 1 to 21 dwellings (ITC-BT-10,
/// tabla 1); over 21, 15,3 + 0,5 for each one more.
pub const SIMULTANEITY: [f64; 21] = [
    1.0, 2.0, 3.0, 3.8, 4.6, 5.4, 6.2, 7.0, 7.8, 8.5, 9.2, 9.9, 10.6, 11.3, 11.9, 12.5, 13.1, 13.7,
    14.3, 14.8, 15.3,
];
/// What each dwelling over 21 adds to the coefficient.
pub const SIMULTANEITY_STEP: f64 = 0.5;
/// Shops and offices: W/m², and the least per shop (ITC-BT-10).
pub const LOCAL_W_M2: f64 = 100.0;
pub const LOCAL_MIN: f64 = 3450.0;
/// Garages with forced ventilation: W/m², and the least.
pub const GARAGE_W_M2: f64 = 20.0;
pub const GARAGE_MIN: f64 = 3450.0;
/// The breaking capacities of the breakers made, kA.
pub const BREAKING_CAPACITIES: [f64; 5] = [4.5, 6.0, 10.0, 15.0, 25.0];
/// Over this unbalance between the phases, a warning.
pub const UNBALANCE_WARNING: f64 = 0.2;

/// The magnetic trip of a curve, times its rating.
pub fn trip_multiple(curve: TripCurve) -> f64 {
    match curve {
        TripCurve::B => 5.0,
        TripCurve::C => 10.0,
        TripCurve::D => 20.0,
    }
}

/// Where the wires run: this high over each floor (under the ceiling).
pub const WIRE_HEIGHT: f64 = 2.5;
/// The sections of the cables, mm².
pub const SECTIONS: [f64; 12] = [
    1.5, 2.5, 4.0, 6.0, 10.0, 16.0, 25.0, 35.0, 50.0, 70.0, 95.0, 120.0,
];

/// The current the wires of a section carry, A (UNE-HD 60364-5-52, tabla
/// B.52.2, method B1, PVC, two loaded conductors, 30 °C).
pub fn ampacity(section: f64) -> f64 {
    match section {
        s if s <= 1.5 => 17.5,
        s if s <= 2.5 => 24.0,
        s if s <= 4.0 => 32.0,
        s if s <= 6.0 => 41.0,
        s if s <= 10.0 => 57.0,
        s if s <= 16.0 => 76.0,
        s if s <= 25.0 => 101.0,
        s if s <= 35.0 => 125.0,
        s if s <= 50.0 => 151.0,
        s if s <= 70.0 => 192.0,
        s if s <= 95.0 => 232.0,
        _ => 269.0,
    }
}

/// The same with three loaded conductors (a three-phase line).
pub fn ampacity_three(section: f64) -> f64 {
    match section {
        s if s <= 1.5 => 15.5,
        s if s <= 2.5 => 21.0,
        s if s <= 4.0 => 28.0,
        s if s <= 6.0 => 36.0,
        s if s <= 10.0 => 50.0,
        s if s <= 16.0 => 68.0,
        s if s <= 25.0 => 89.0,
        s if s <= 35.0 => 110.0,
        s if s <= 50.0 => 134.0,
        s if s <= 70.0 => 171.0,
        s if s <= 95.0 => 207.0,
        _ => 239.0,
    }
}

/// The voltage drop of a single-phase line, as a share of the voltage.
pub fn drop(length: f64, power: f64, section: f64) -> f64 {
    2.0 * length * power / (GAMMA * section * VOLTAGE * VOLTAGE)
}

/// The voltage drop of a balanced three-phase line, as a share of the
/// voltage between phases.
pub fn drop_three(length: f64, power: f64, section: f64) -> f64 {
    length * power / (GAMMA * section * LINE_VOLTAGE * LINE_VOLTAGE)
}

/// The current of a line: P/U in single phase, P/(√3·U·cos φ) in three.
pub fn line_current(power: f64, three_phase: bool) -> f64 {
    if three_phase {
        power / (3f64.sqrt() * LINE_VOLTAGE * COS_PHI)
    } else {
        power / VOLTAGE
    }
}

/// The coefficient of simultaneity of so many dwellings (ITC-BT-10).
pub fn simultaneity(dwellings: usize) -> f64 {
    match dwellings {
        0 => 0.0,
        n if n <= 21 => SIMULTANEITY[n - 1],
        n => SIMULTANEITY[20] + SIMULTANEITY_STEP * (n - 21) as f64,
    }
}

/// The resistance of a phase conductor at 20 °C, Ω.
pub fn phase_resistance(length: f64, section: f64) -> f64 {
    length / (GAMMA_20 * section)
}

/// The short circuit current with so much resistance before it, kA.
pub fn fault_current(resistance: f64) -> f64 {
    FAULT_FACTOR * VOLTAGE / resistance.max(1e-5) / 1000.0
}

/// The resistance before the main fuse box: from the current the company
/// gives, or from its line to the transformer.
pub fn upstream_resistance(site: &archisim_core::ElectricalSite) -> f64 {
    if site.supply_fault > 0.0 {
        FAULT_FACTOR * VOLTAGE / (site.supply_fault * 1000.0)
    } else {
        phase_resistance(
            site.network_length,
            NETWORK_SECTION * GAMMA_AL_20 / GAMMA_20,
        )
    }
}

/// The breakers made, A.
pub const BREAKERS: [u32; 11] = [10, 16, 20, 25, 32, 40, 50, 63, 80, 100, 125];

/// The least breaker that carries a current.
fn breaker_for(current: f64) -> u32 {
    BREAKERS
        .into_iter()
        .find(|b| f64::from(*b) + 1e-9 >= current)
        .unwrap_or(BREAKERS[BREAKERS.len() - 1])
}

/// What a board serves.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum BoardUse {
    Dwelling,
    /// The building's general services (stairs, lift, portal).
    Services,
    /// A shop or an office.
    Local,
    Garage,
}

impl BoardUse {
    pub fn name(self) -> &'static str {
        match self {
            BoardUse::Dwelling => "vivienda",
            BoardUse::Services => "servicios generales",
            BoardUse::Local => "local",
            BoardUse::Garage => "garaje",
        }
    }
}

/// What a room is, for the board that serves it.
fn room_use(usage: &str, name: &str) -> Option<BoardUse> {
    let t = format!("{usage} {name}").to_lowercase();
    let any = |w: &[&str]| w.iter().any(|x| t.contains(x));
    if any(&["garaje", "aparcamiento", "parking"]) {
        Some(BoardUse::Garage)
    } else if any(&[
        "local",
        "comercial",
        "tienda",
        "oficina",
        "café",
        "cafe",
        "bar ",
        "restaurante",
    ]) {
        Some(BoardUse::Local)
    } else if any(&[
        "portal",
        "escalera",
        "común",
        "comun",
        "instalaciones",
        "ascensor",
        "trastero",
    ]) {
        Some(BoardUse::Services)
    } else {
        None
    }
}

/// What a board serves, by its name or else by the rooms around it.
fn board_use(name: &str, areas: &BTreeMap<Option<BoardUse>, f64>) -> BoardUse {
    let n = name.to_lowercase();
    let has = |w: &[&str]| w.iter().any(|x| n.contains(x));
    if has(&["vivienda"]) {
        return BoardUse::Dwelling;
    }
    if has(&["garaje", "aparcamiento"]) {
        return BoardUse::Garage;
    }
    if has(&["local", "comercial", "oficina", "tienda"]) {
        return BoardUse::Local;
    }
    if has(&["general", "servicios", "comunes", "escalera"]) {
        return BoardUse::Services;
    }
    let of = |u: Option<BoardUse>| areas.get(&u).copied().unwrap_or(0.0);
    let home = of(None);
    [BoardUse::Garage, BoardUse::Local]
        .into_iter()
        .find(|u| of(Some(*u)) > home && of(Some(*u)) > 0.0)
        .unwrap_or(BoardUse::Dwelling)
}

/// Whether a board drawn is the meters' centralisation, not a board.
fn is_meters(name: &str) -> bool {
    let n = name.to_lowercase();
    n.contains("contador") || n.contains("centraliz")
}

/// What a point of a circuit is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum PointOf {
    Device(DeviceId),
    /// A lamp or an appliance placed as an object.
    Object(ObjectId),
}

/// A point of a circuit.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CircuitPoint {
    pub of: PointOf,
    pub level: LevelId,
    pub position: Point2,
    /// Height over its level.
    pub z: f64,
}

/// A stretch of a circuit's wires: from the board or a point to a point.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Stretch {
    /// The point it comes from (`None`: the board).
    pub from: Option<usize>,
    /// The point it goes to (an index of the circuit's points).
    pub to: usize,
    pub length: f64,
    /// The power of the points after it, W.
    pub power: f64,
    /// Its own drop with the chosen section, share of the voltage.
    pub drop: f64,
    /// From the board to its end: length and drop.
    pub distance: f64,
    pub drop_at: f64,
}

/// A circuit worked out.
#[derive(Debug, Clone, PartialEq)]
pub struct Circuit {
    pub spec: CircuitSpec,
    /// The points drawn.
    pub points: Vec<CircuitPoint>,
    /// The points it is calculated with (one at least for C1 to C5).
    pub count: u32,
    /// Design power, W.
    pub power: f64,
    /// Design current, A.
    pub current: f64,
    /// Breaker, A.
    pub breaker: u32,
    /// The section of the table, mm².
    pub section: f64,
    /// The section that does (the table's, or more for the voltage drop
    /// or the trip), mm².
    pub chosen: f64,
    /// The current its wires carry, A.
    pub ampacity: f64,
    /// From the board to the point that drops most, along the wires, m
    /// (an assumed 10 m when it has no point drawn).
    pub length: f64,
    /// Voltage drop with the chosen section, share of the voltage, up to
    /// the point that drops most.
    pub drop: f64,
    /// Voltage drop with the table's section.
    pub table_drop: f64,
    /// Tube, mm.
    pub tube: u32,
    /// The stretches of its wires (none when no point is drawn).
    pub stretches: Vec<Stretch>,
    /// The phase it hangs from in three phase (1 to 3).
    pub phase: Option<u8>,
    /// Short circuit at its start (the board) and at its farthest end,
    /// kA.
    pub fault_start: f64,
    pub fault_end: f64,
    /// The current its breaker trips at by its curve, A.
    pub trip: f64,
    /// The section went up so that its breaker trips at its end.
    pub raised_for_trip: bool,
}

impl Circuit {
    pub fn too_many_points(&self) -> bool {
        self.spec.max_points > 0 && self.points.len() as u32 > self.spec.max_points
    }

    /// Ib ≤ In ≤ Iz.
    pub fn protected(&self) -> bool {
        self.current <= f64::from(self.breaker) + 1e-9
            && f64::from(self.breaker) <= self.ampacity + 1e-9
    }

    /// The breaker trips with the least short circuit, at its end.
    pub fn trips(&self) -> bool {
        self.fault_end * 1000.0 + 1e-6 >= self.trip
    }

    pub fn bigger_section(&self) -> bool {
        self.chosen > self.section + 1e-9
    }

    /// «C1».
    pub fn code(&self) -> String {
        format!("C{}", self.spec.number)
    }

    /// «L2», or empty in single phase.
    pub fn phase_text(&self) -> String {
        self.phase.map_or(String::new(), |p| format!("L{p}"))
    }

    /// The stretches from the board to the point that drops most.
    pub fn worst_path(&self) -> Vec<&Stretch> {
        let Some(last) = self
            .stretches
            .iter()
            .max_by(|a, b| a.drop_at.total_cmp(&b.drop_at))
        else {
            return Vec::new();
        };
        let mut path = vec![last];
        let mut at = last.from;
        while let Some(p) = at {
            let Some(s) = self.stretches.iter().find(|s| s.to == p) else {
                break;
            };
            path.push(s);
            at = s.from;
        }
        path.reverse();
        path
    }
}

/// A feeder: the one from the meter to the board («derivación
/// individual») or the general one from the main fuse box to the meters
/// («línea general de alimentación»).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Feeder {
    pub length: f64,
    pub power: f64,
    pub current: f64,
    pub section: f64,
    pub ampacity: f64,
    pub drop: f64,
    pub limit: f64,
    pub tube: u32,
    pub three_phase: bool,
    /// Its length measured on the drawing (not the one given).
    pub measured: bool,
}

impl Feeder {
    /// «2×10+TT×10» or «4×16+TT×16».
    pub fn wires(&self) -> String {
        let s = section_text(self.section);
        format!("{}×{s}+TT×{s}", if self.three_phase { 4 } else { 2 })
    }

    pub fn passes(&self) -> bool {
        self.drop <= self.limit + 1e-9 && self.current <= self.ampacity + 1e-9
    }
}

/// A residual current device and the circuits after it.
#[derive(Debug, Clone, PartialEq)]
pub struct Differential {
    /// A.
    pub rating: u32,
    /// mA.
    pub sensitivity: u32,
    pub circuits: Vec<u8>,
    /// 2 or 4.
    pub poles: u8,
}

/// An installation: what hangs from one board.
#[derive(Debug, Clone, PartialEq)]
pub struct Installation {
    /// The board drawn (`None`: none is drawn; the calculation stands it
    /// at the first mechanism).
    pub board: Option<DeviceId>,
    pub level: LevelId,
    pub position: Point2,
    /// The height of the board's middle over its level.
    pub z: f64,
    pub use_: BoardUse,
    pub grade: Electrification,
    /// Why high, when it is worked out.
    pub reasons: Vec<String>,
    /// Area of its rooms, m².
    pub area: f64,
    /// The power it is designed for, W.
    pub power: f64,
    pub three_phase: bool,
    /// The power of the circuits on each phase, W (all on the first in
    /// single phase).
    pub phase_loads: [f64; 3],
    /// (heaviest − mean) / mean, 0 in single phase.
    pub unbalance: f64,
    /// Main switch, A.
    pub main_switch: u32,
    pub feeder: Feeder,
    /// The drop of the general feeder before its meter, share.
    pub general_drop: f64,
    /// The short circuit at the board, kA.
    pub fault: f64,
    /// The breaking capacity of its breakers, kA.
    pub breaking_capacity: f64,
    pub differentials: Vec<Differential>,
    pub circuits: Vec<Circuit>,
}

impl Installation {
    /// The breakers break the short circuit at the board.
    pub fn breaks(&self) -> bool {
        self.fault <= self.breaking_capacity + 1e-9
    }

    /// «2P» or «4P».
    pub fn poles(&self) -> &'static str {
        if self.three_phase {
            "4P"
        } else {
            "2P"
        }
    }

    /// The least breaking capacity made that breaks its short circuit.
    pub fn breaking_needed(&self) -> Option<f64> {
        BREAKING_CAPACITIES
            .into_iter()
            .find(|b| *b + 1e-9 >= self.fault)
    }

    /// The drop from the main fuse box to the point of its circuits that
    /// drops most (single-phase circuits as a share of 230 V, the feeders
    /// of theirs).
    pub fn total_drop(&self) -> f64 {
        self.general_drop
            + self.feeder.drop
            + self.circuits.iter().map(|c| c.drop).fold(0.0, f64::max)
    }
}

/// The building's forecast and general feeder (ITC-BT-10 and 14).
#[derive(Debug, Clone, PartialEq)]
pub struct Building {
    pub dwellings: usize,
    /// The mean power of the dwellings, W.
    pub dwelling_mean: f64,
    pub coefficient: f64,
    pub dwellings_power: f64,
    pub services: f64,
    pub locals: f64,
    pub garage: f64,
    pub total: f64,
    pub three_phase: bool,
    /// The general feeder, with the meters centralised.
    pub general: Option<Feeder>,
    /// The power of the installations on each phase of the supply, W.
    pub phase_loads: [f64; 3],
    pub unbalance: f64,
    /// The short circuit at the main fuse box and at the meters, kA.
    pub fault_supply: f64,
    pub fault_meters: f64,
}

/// The whole calculation.
#[derive(Debug, Clone, PartialEq)]
pub struct Electrical {
    pub installations: Vec<Installation>,
    pub building: Option<Building>,
}

/// The height of the middle of a device over its level.
fn mid(d: &archisim_core::ServiceDevice) -> f64 {
    d.elevation + d.height / 2.0
}

/// The phases a list of loads go on, the heaviest first on the lightest
/// phase; with each phase's load and the unbalance.
fn share_out(loads: &[f64]) -> (Vec<u8>, [f64; 3], f64) {
    let mut order: Vec<usize> = (0..loads.len()).collect();
    order.sort_by(|a, b| loads[*b].total_cmp(&loads[*a]).then(a.cmp(b)));
    let mut phases = vec![1u8; loads.len()];
    let mut sum = [0.0f64; 3];
    for i in order {
        let p = (0..3)
            .min_by(|a, b| sum[*a].total_cmp(&sum[*b]).then(a.cmp(b)))
            .unwrap_or(0);
        sum[p] += loads[i];
        phases[i] = p as u8 + 1;
    }
    (phases, sum, unbalance(sum))
}

fn unbalance(sum: [f64; 3]) -> f64 {
    let mean = (sum[0] + sum[1] + sum[2]) / 3.0;
    if mean <= 0.0 {
        0.0
    } else {
        (sum.iter().copied().fold(0.0, f64::max) - mean) / mean
    }
}

/// An installation before its feeder: its circuits' powers and points.
struct Draft {
    board: Option<DeviceId>,
    level: LevelId,
    position: Point2,
    z: f64,
    use_: BoardUse,
    grade: Electrification,
    reasons: Vec<String>,
    area: f64,
    power: f64,
    circuits: Vec<(CircuitSpec, Vec<CircuitPoint>)>,
}

/// Every installation of the model: one per board, with the mechanisms
/// nearest to it; one with no board if mechanisms are drawn and no board.
pub fn installations(model: &Model) -> Vec<Installation> {
    analysis(model).installations
}

/// The installations and the building.
pub fn analysis(model: &Model) -> Electrical {
    let level_z = |l: LevelId| model.level(l).map_or(0.0, |l| l.elevation);
    let panels: Vec<&archisim_core::ServiceDevice> = model
        .devices()
        .filter(|d| d.kind == DeviceKind::Panel)
        .collect();
    let meters = panels.iter().find(|d| is_meters(&d.name)).copied();
    let boards: Vec<&archisim_core::ServiceDevice> = panels
        .iter()
        .filter(|d| !is_meters(&d.name))
        .copied()
        .collect();
    // Every point, with its circuit.
    let mut points: Vec<(u8, CircuitPoint)> = Vec::new();
    for d in model.devices() {
        if matches!(d.kind, DeviceKind::Switch | DeviceKind::TwoWaySwitch) {
            continue;
        }
        if let Some(n) = model.device_circuit(d.id) {
            points.push((
                n,
                CircuitPoint {
                    of: PointOf::Device(d.id),
                    level: d.level,
                    position: d.position,
                    z: mid(d),
                },
            ));
        }
    }
    for (id, level) in model.lamp_points() {
        if let Some(o) = model.objects().find(|o| o.id == id) {
            points.push((
                1,
                CircuitPoint {
                    of: PointOf::Object(id),
                    level,
                    position: o.position,
                    z: WIRE_HEIGHT,
                },
            ));
        }
    }
    for o in model.objects() {
        if matches!(
            archisim_core::Appliance::of(model, o.id),
            Some(archisim_core::Appliance::Washer | archisim_core::Appliance::Dishwasher)
        ) {
            points.push((
                4,
                CircuitPoint {
                    of: PointOf::Object(o.id),
                    level: o.level,
                    position: o.position,
                    z: 0.5,
                },
            ));
        }
    }
    if boards.is_empty() && points.is_empty() {
        return Electrical {
            installations: Vec::new(),
            building: None,
        };
    }
    // Which board each point and room hangs from: the nearest, counting a
    // floor as 10 m.
    let near = |level: LevelId, p: Point2| -> usize {
        boards
            .iter()
            .enumerate()
            .min_by(|a, b| {
                let da =
                    a.1.position.distance(p) + 10.0 * (level_z(a.1.level) - level_z(level)).abs();
                let db =
                    b.1.position.distance(p) + 10.0 * (level_z(b.1.level) - level_z(level)).abs();
                da.total_cmp(&db)
            })
            .map_or(0, |b| b.0)
    };
    let count = boards.len().max(1);
    let mut by_board: Vec<Vec<(u8, CircuitPoint)>> = vec![Vec::new(); count];
    for (n, p) in points {
        by_board[near(p.level, p.position)].push((n, p));
    }
    let mut areas = vec![0.0; count];
    let mut use_areas: Vec<BTreeMap<Option<BoardUse>, f64>> = vec![BTreeMap::new(); count];
    for r in model.rooms() {
        let c = archisim_core::rooms::centroid(&r.outline);
        let i = near(r.level, c);
        areas[i] += r.area();
        *use_areas[i].entry(room_use(&r.usage, &r.name)).or_default() += r.area();
    }
    let site = *model.electrical_site();
    let mut drafts = Vec::new();
    for (i, pts) in by_board.into_iter().enumerate() {
        let (board, level, position, z, name) = match boards.get(i) {
            Some(b) => (Some(b.id), b.level, b.position, mid(b), b.name.clone()),
            None => {
                let Some(p) = pts.first() else { continue };
                (None, p.1.level, p.1.position, 1.8, String::new())
            }
        };
        if board.is_none() && pts.is_empty() {
            continue;
        }
        let use_ = board_use(&name, &use_areas[i]);
        let mut grouped: BTreeMap<u8, Vec<CircuitPoint>> = BTreeMap::new();
        for (n, p) in pts {
            grouped.entry(n).or_default().push(p);
        }
        // The degree of electrification.
        let mut reasons = Vec::new();
        let area = areas[i];
        if use_ == BoardUse::Dwelling {
            if area > 160.0 {
                reasons.push(format!("{} m² de locales (más de 160 m²)", fmt(area, 0)));
            }
            for (n, list) in &grouped {
                if let Some(spec) = circuit_spec(*n) {
                    if spec.high_only {
                        reasons.push(format!(
                            "hay mecanismos en C{n} ({})",
                            spec.name.to_lowercase()
                        ));
                    } else if spec.max_points > 0 && list.len() as u32 > spec.max_points {
                        reasons.push(format!(
                            "C{n} tiene {} puntos (más de {})",
                            list.len(),
                            spec.max_points
                        ));
                    }
                }
            }
        }
        let grade = match site.grade {
            Electrification::Auto if reasons.is_empty() => Electrification::Basic,
            Electrification::Auto => Electrification::High,
            g => g,
        };
        // The circuits: in a dwelling, C1 to C5 always; the others with
        // points.
        let mut circuits = Vec::new();
        for spec in CIRCUITS.iter() {
            let list = grouped.remove(&spec.number).unwrap_or_default();
            if (spec.high_only || use_ != BoardUse::Dwelling) && list.is_empty() {
                continue;
            }
            circuits.push((*spec, list));
        }
        // Circuits not of the table: as a C2.
        for (n, list) in grouped {
            if n == 0 {
                continue;
            }
            let mut spec = CIRCUITS[1];
            spec.number = n;
            spec.name = "Circuito propio";
            circuits.push((spec, list));
        }
        let sum: f64 = circuits
            .iter()
            .map(|(spec, list)| circuit_power(spec, list.len()))
            .sum();
        // The power (ITC-BT-10).
        let power = match use_ {
            BoardUse::Dwelling if grade == Electrification::High => 9200.0,
            BoardUse::Dwelling => 5750.0,
            BoardUse::Services => sum,
            BoardUse::Local => (LOCAL_W_M2 * area).max(LOCAL_MIN),
            BoardUse::Garage => (GARAGE_W_M2 * area).max(GARAGE_MIN),
        };
        drafts.push(Draft {
            board,
            level,
            position,
            z,
            use_,
            grade,
            reasons,
            area,
            power,
            circuits,
        });
    }
    let three = |power: f64| match site.supply {
        Supply::Three => true,
        Supply::Single => false,
        Supply::Auto => power > THREE_PHASE_FROM + 1e-9,
    };
    // The building.
    let of = |u: BoardUse| drafts.iter().filter(move |d| d.use_ == u);
    let dwellings = of(BoardUse::Dwelling).count();
    let dwelling_mean = if dwellings == 0 {
        0.0
    } else {
        of(BoardUse::Dwelling).map(|d| d.power).sum::<f64>() / dwellings as f64
    };
    let coefficient = simultaneity(dwellings);
    let services: f64 = of(BoardUse::Services).map(|d| d.power).sum();
    let locals: f64 = of(BoardUse::Local).map(|d| d.power).sum();
    let garage: f64 = of(BoardUse::Garage).map(|d| d.power).sum();
    let dwellings_power = dwelling_mean * coefficient;
    let total = dwellings_power + services + locals + garage;
    let general_three = three(total);
    let r_up = upstream_resistance(&site);
    let general = (!site.single_user).then(|| {
        feeder(
            site.general_length,
            total,
            if site.partly_centralised {
                GENERAL_DROP_PARTLY
            } else {
                GENERAL_DROP_CENTRALISED
            },
            general_three,
            GENERAL_MIN_SECTION,
            false,
        )
    });
    let r_meters = r_up
        + general
            .as_ref()
            .map_or(0.0, |g| phase_resistance(g.length, g.section));
    // Where the meters are: drawn, or by the general services' board.
    let meters_at = meters
        .map(|d| (d.level, d.position, mid(d)))
        .or_else(|| {
            drafts
                .iter()
                .find(|d| d.use_ == BoardUse::Services && d.board.is_some())
                .map(|d| (d.level, d.position, d.z))
        })
        .filter(|_| !site.single_user);
    let mut out = Vec::new();
    for d in drafts {
        let three_phase = three(d.power);
        let (length, measured) = match meters_at {
            Some((l, p, z)) => {
                let dd = d.position - p;
                let up = (level_z(d.level) + d.z - level_z(l) - z).abs();
                ((dd.x.abs() + dd.y.abs() + up).max(1.0), true)
            }
            None => (site.feeder_length, false),
        };
        let limit = if site.single_user {
            FEEDER_DROP_SINGLE_USER
        } else if site.partly_centralised {
            FEEDER_DROP_PARTLY
        } else {
            FEEDER_DROP_CENTRALISED
        };
        let feeder = feeder(
            length,
            d.power,
            limit,
            three_phase,
            FEEDER_MIN_SECTION,
            measured,
        );
        let r_board = r_meters + phase_resistance(feeder.length, feeder.section);
        let fault = fault_current(r_board);
        let mut circuits: Vec<Circuit> = d
            .circuits
            .into_iter()
            .map(|(spec, list)| {
                work_circuit(
                    model, &spec, list, d.level, d.position, d.z, r_board, site.curve,
                )
            })
            .collect();
        let (phase_loads, unbalance) = if three_phase {
            let loads: Vec<f64> = circuits.iter().map(|c| c.power).collect();
            let (phases, sum, u) = share_out(&loads);
            for (c, p) in circuits.iter_mut().zip(phases) {
                c.phase = Some(p);
            }
            (sum, u)
        } else {
            ([circuits.iter().map(|c| c.power).sum(), 0.0, 0.0], 0.0)
        };
        let current = line_current(d.power, three_phase);
        let main_switch = match d.use_ {
            BoardUse::Dwelling if !three_phase => {
                let least = if d.grade == Electrification::High {
                    40
                } else {
                    25
                };
                breaker_for(current).max(least)
            }
            _ => breaker_for(current).max(25),
        };
        let differentials = circuits
            .chunks(5)
            .map(|c| Differential {
                rating: match main_switch {
                    m if m <= 40 => 40,
                    m if m <= 63 => 63,
                    m => m,
                },
                sensitivity: 30,
                circuits: c.iter().map(|c| c.spec.number).collect(),
                poles: if three_phase { 4 } else { 2 },
            })
            .collect();
        out.push(Installation {
            board: d.board,
            level: d.level,
            position: d.position,
            z: d.z,
            use_: d.use_,
            grade: d.grade,
            reasons: d.reasons,
            area: d.area,
            power: d.power,
            three_phase,
            phase_loads,
            unbalance,
            main_switch,
            feeder,
            general_drop: general.as_ref().map_or(0.0, |g| g.drop),
            fault,
            breaking_capacity: site.breaking_capacity,
            differentials,
            circuits,
        });
    }
    // The installations over the phases of the supply: the single-phase
    // ones whole, the three-phase ones a third on each.
    let singles: Vec<f64> = out
        .iter()
        .map(|i| if i.three_phase { 0.0 } else { i.power })
        .collect();
    let (_, mut phase_loads, _) = share_out(&singles);
    for i in out.iter().filter(|i| i.three_phase) {
        for p in phase_loads.iter_mut() {
            *p += i.power / 3.0;
        }
    }
    let building = (!out.is_empty()).then(|| Building {
        dwellings,
        dwelling_mean,
        coefficient,
        dwellings_power,
        services,
        locals,
        garage,
        total,
        three_phase: general_three,
        general,
        unbalance: unbalance(phase_loads),
        phase_loads,
        fault_supply: fault_current(r_up),
        fault_meters: fault_current(r_meters),
    });
    Electrical {
        installations: out,
        building,
    }
}

/// The feeder that carries `power`.
fn feeder(
    length: f64,
    power: f64,
    limit: f64,
    three_phase: bool,
    least: f64,
    measured: bool,
) -> Feeder {
    let current = line_current(power, three_phase);
    let carries = |s: f64| {
        if three_phase {
            ampacity_three(s)
        } else {
            ampacity(s)
        }
    };
    let drops = |s: f64| {
        if three_phase {
            drop_three(length, power, s)
        } else {
            drop(length, power, s)
        }
    };
    let section = SECTIONS
        .into_iter()
        .filter(|s| *s >= least)
        .find(|s| carries(*s) >= current && drops(*s) <= limit)
        .unwrap_or(SECTIONS[SECTIONS.len() - 1]);
    Feeder {
        length,
        power,
        current,
        section,
        ampacity: carries(section),
        drop: drops(section),
        limit,
        tube: match section {
            s if s <= 10.0 => 32,
            s if s <= 16.0 => 40,
            s if s <= 35.0 => 50,
            s if s <= 70.0 => 63,
            _ => 75,
        },
        three_phase,
        measured,
    }
}

/// The design power of a circuit with so many points drawn.
fn circuit_power(spec: &CircuitSpec, points: usize) -> f64 {
    let count = (points as u32).max(1);
    let power = f64::from(count) * spec.point_power * spec.simultaneity * spec.utilisation;
    // A single point takes its own power (an oven, a dryer).
    power.max(if count == 1 && spec.number != 1 && spec.number != 6 {
        spec.point_power * spec.utilisation
    } else {
        0.0
    })
}

/// The shortest tree from the board through the points: each point's
/// parent (`None`: the board) and the length of the wire to it.
fn route(
    model: &Model,
    points: &[CircuitPoint],
    level: LevelId,
    board: Point2,
    board_z: f64,
) -> Vec<(Option<usize>, f64)> {
    let level_z = |l: LevelId| model.level(l).map_or(0.0, |l| l.elevation);
    let wire = |a: Point2, az: f64, al: f64, b: &CircuitPoint| {
        let d = b.position - a;
        d.x.abs()
            + d.y.abs()
            + (WIRE_HEIGHT - az).abs()
            + (WIRE_HEIGHT - b.z).abs()
            + (al - level_z(b.level)).abs()
    };
    let n = points.len();
    let mut best: Vec<(Option<usize>, f64)> = points
        .iter()
        .map(|p| (None, wire(board, board_z, level_z(level), p)))
        .collect();
    let mut done = vec![false; n];
    for _ in 0..n {
        let Some(k) = (0..n)
            .filter(|i| !done[*i])
            .min_by(|a, b| best[*a].1.total_cmp(&best[*b].1).then(a.cmp(b)))
        else {
            break;
        };
        done[k] = true;
        let pk = points[k];
        for j in 0..n {
            if done[j] {
                continue;
            }
            let l = wire(pk.position, pk.z, level_z(pk.level), &points[j]);
            if l < best[j].1 - 1e-9 {
                best[j] = (Some(k), l);
            }
        }
    }
    best
}

#[allow(clippy::too_many_arguments)]
fn work_circuit(
    model: &Model,
    spec: &CircuitSpec,
    points: Vec<CircuitPoint>,
    level: LevelId,
    board: Point2,
    board_z: f64,
    r_board: f64,
    curve: TripCurve,
) -> Circuit {
    let count = (points.len() as u32).max(1);
    let power = circuit_power(spec, points.len());
    let current = power / VOLTAGE;
    // The stretches, with the power after each one and their drop with
    // 1 mm² (it goes as 1/S).
    let tree = route(model, &points, level, board, board_z);
    let n = points.len();
    let share = if n == 0 { 0.0 } else { power / n as f64 };
    let mut after = vec![1usize; n];
    // Points in the order they were reached: parents before children.
    let mut order: Vec<usize> = (0..n).collect();
    let depth = |mut i: usize| {
        let mut d = 0;
        while let Some(p) = tree[i].0 {
            i = p;
            d += 1;
            if d > n {
                break;
            }
        }
        d
    };
    order.sort_by_key(|i| (depth(*i), *i));
    for &i in order.iter().rev() {
        if let Some(p) = tree[i].0 {
            after[p] += after[i];
        }
    }
    let mut distance = vec![0.0; n];
    let mut unit_drop = vec![0.0; n];
    for &i in &order {
        let (parent, l) = tree[i];
        let (d0, u0) = parent.map_or((0.0, 0.0), |p| (distance[p], unit_drop[p]));
        distance[i] = d0 + l;
        unit_drop[i] = u0 + drop(l, share * after[i] as f64, 1.0);
    }
    let (worst_unit, worst_length) = if n == 0 {
        (drop(10.0, power, 1.0), 10.0)
    } else {
        (0..n)
            .map(|i| (unit_drop[i], distance[i]))
            .fold((0.0, 0.0), |a, b| if b.0 > a.0 { b } else { a })
    };
    let farthest = if n == 0 {
        10.0
    } else {
        distance.iter().copied().fold(0.0, f64::max)
    };
    let trip = trip_multiple(curve) * f64::from(spec.breaker);
    let end_fault = |s: f64| fault_current(r_board + phase_resistance(farthest, s));
    let table_drop = worst_unit / spec.section;
    let fits = |s: f64| worst_unit / s <= CIRCUIT_DROP && ampacity(s) >= f64::from(spec.breaker);
    let chosen = SECTIONS
        .into_iter()
        .filter(|s| *s >= spec.section)
        .find(|s| fits(*s) && end_fault(*s) * 1000.0 >= trip)
        .unwrap_or(SECTIONS[SECTIONS.len() - 1]);
    let by_drop = SECTIONS
        .into_iter()
        .filter(|s| *s >= spec.section)
        .find(|s| fits(*s))
        .unwrap_or(chosen);
    let stretches = (0..n)
        .map(|i| {
            let (from, l) = tree[i];
            Stretch {
                from,
                to: i,
                length: l,
                power: share * after[i] as f64,
                drop: drop(l, share * after[i] as f64, chosen),
                distance: distance[i],
                drop_at: unit_drop[i] / chosen,
            }
        })
        .collect();
    Circuit {
        spec: *spec,
        points,
        count,
        power,
        current,
        breaker: spec.breaker,
        section: spec.section,
        chosen,
        ampacity: ampacity(chosen),
        length: worst_length,
        drop: worst_unit / chosen,
        table_drop,
        tube: if chosen > spec.section {
            match chosen {
                s if s <= 2.5 => 20,
                s if s <= 6.0 => 25,
                s if s <= 10.0 => 32,
                _ => 40,
            }
        } else {
            spec.tube
        },
        stretches,
        phase: None,
        fault_start: fault_current(r_board),
        fault_end: end_fault(chosen),
        trip,
        raised_for_trip: chosen > by_drop + 1e-9,
    }
}

/// «12,3».
pub fn fmt(v: f64, decimals: usize) -> String {
    format!("{v:.decimals$}").replace('.', ",")
}

/// «1,5».
pub fn section_text(s: f64) -> String {
    if s.fract().abs() < 1e-9 {
        format!("{s:.0}")
    } else {
        fmt(s, 1)
    }
}

/// «4,6 kA».
pub fn ka(v: f64) -> String {
    if v >= 100.0 {
        format!("{} kA", fmt(v, 0))
    } else {
        format!("{} kA", fmt(v, 2))
    }
}

/// The building's forecast in words.
pub fn building_text(b: &Building, site: &archisim_core::ElectricalSite) -> String {
    let mut s = String::from("Previsión de cargas (ITC-BT-10)\n");
    if b.dwellings > 0 {
        s.push_str(&format!(
            "  Viviendas: {}, de {} W de media × coeficiente de simultaneidad {} = {} W\n",
            b.dwellings,
            fmt(b.dwelling_mean, 0),
            fmt(b.coefficient, 1),
            fmt(b.dwellings_power, 0)
        ));
    }
    if b.services > 0.0 {
        s.push_str(&format!(
            "  Servicios generales: {} W\n",
            fmt(b.services, 0)
        ));
    }
    if b.locals > 0.0 {
        s.push_str(&format!(
            "  Locales y oficinas: {} W ({} W/m², {} W por local como mínimo)\n",
            fmt(b.locals, 0),
            fmt(LOCAL_W_M2, 0),
            fmt(LOCAL_MIN, 0)
        ));
    }
    if b.garage > 0.0 {
        s.push_str(&format!(
            "  Garaje: {} W ({} W/m², {} W como mínimo)\n",
            fmt(b.garage, 0),
            fmt(GARAGE_W_M2, 0),
            fmt(GARAGE_MIN, 0)
        ));
    }
    s.push_str(&format!(
        "  Total: {} W · suministro {}\n",
        fmt(b.total, 0),
        if b.three_phase {
            "trifásico 3×400/230 V"
        } else {
            "monofásico 230 V"
        }
    ));
    if b.three_phase {
        s.push_str(&format!(
            "  Fases (potencia de las instalaciones): L1 {} W · L2 {} W · L3 {} W · desequilibrio {} %\n",
            fmt(b.phase_loads[0], 0),
            fmt(b.phase_loads[1], 0),
            fmt(b.phase_loads[2], 0),
            fmt(b.unbalance * 100.0, 0)
        ));
    }
    s.push_str(&format!(
        "  Cortocircuito en la CGP: {} ({})\n",
        ka(b.fault_supply),
        if site.supply_fault > 0.0 {
            "dado por la compañía".to_string()
        } else {
            format!(
                "desde el transformador, {} m de línea de {} mm² Al",
                fmt(site.network_length, 0),
                fmt(NETWORK_SECTION, 0)
            )
        }
    ));
    if let Some(g) = &b.general {
        s.push_str(&format!(
            "  Línea general de alimentación (ITC-BT-14): {} m, {} A, {} mm² Cu, tubo Ø{}, ΔU {} % (≤ {} %){}\n",
            fmt(g.length, 1),
            fmt(g.current, 1),
            g.wires(),
            g.tube,
            fmt(g.drop * 100.0, 2),
            fmt(g.limit * 100.0, 1),
            if g.passes() { "" } else { " NO CUMPLE" }
        ));
        s.push_str(&format!(
            "  Cortocircuito en los contadores: {}\n",
            ka(b.fault_meters)
        ));
    }
    s
}

/// The installation in words, for the window and the report.
pub fn report(model: &Model) -> String {
    let e = analysis(model);
    let all = &e.installations;
    if all.is_empty() {
        return "No hay cuadro ni mecanismos eléctricos dibujados.\n".into();
    }
    let site = *model.electrical_site();
    let mut s = String::new();
    if let Some(b) = &e.building {
        if all.len() > 1 || b.general.is_some() {
            s.push_str(&building_text(b, &site));
            s.push('\n');
        }
    }
    for (i, inst) in all.iter().enumerate() {
        let name = inst
            .board
            .and_then(|b| model.device(b))
            .map_or("Cuadro (sin dibujar)".to_string(), |d| d.name.clone());
        s.push_str(&format!("{}. {}\n", i + 1, name));
        let what = match inst.use_ {
            BoardUse::Dwelling => format!(
                "Vivienda de electrificación {}",
                match inst.grade {
                    Electrification::High => "elevada",
                    _ => "básica",
                }
            ),
            BoardUse::Services => "Servicios generales".to_string(),
            BoardUse::Local => format!("Local de {} m²", fmt(inst.area, 0)),
            BoardUse::Garage => format!("Garaje de {} m²", fmt(inst.area, 0)),
        };
        s.push_str(&format!(
            "  {what} · {} W · {} · IGA {} A {}{}\n",
            fmt(inst.power, 0),
            if inst.three_phase {
                "trifásica 3×400/230 V"
            } else {
                "monofásica 230 V"
            },
            inst.main_switch,
            inst.poles(),
            if inst.reasons.is_empty() {
                String::new()
            } else {
                format!(" ({})", inst.reasons.join("; "))
            }
        ));
        if inst.three_phase {
            s.push_str(&format!(
                "  Fases: L1 {} W · L2 {} W · L3 {} W · desequilibrio {} %{}\n",
                fmt(inst.phase_loads[0], 0),
                fmt(inst.phase_loads[1], 0),
                fmt(inst.phase_loads[2], 0),
                fmt(inst.unbalance * 100.0, 0),
                if inst.unbalance > UNBALANCE_WARNING {
                    " (conviene repartir mejor)"
                } else {
                    ""
                }
            ));
        }
        let f = inst.feeder;
        s.push_str(&format!(
            "  Derivación individual: {} m{}, {} A, {} mm² Cu, tubo Ø{}, ΔU {} % (≤ {} %){}\n",
            fmt(f.length, 1),
            if f.measured {
                " (desde los contadores)"
            } else {
                ""
            },
            fmt(f.current, 1),
            f.wires(),
            f.tube,
            fmt(f.drop * 100.0, 2),
            fmt(f.limit * 100.0, 1),
            if f.passes() { "" } else { " NO CUMPLE" }
        ));
        s.push_str(&format!(
            "  Cortocircuito en el cuadro: {} · poder de corte {}{}\n",
            ka(inst.fault),
            ka(inst.breaking_capacity),
            if inst.breaks() {
                String::new()
            } else {
                format!(
                    " NO CUMPLE (hace falta {})",
                    inst.breaking_needed()
                        .map_or("protección de acompañamiento".into(), ka)
                )
            }
        ));
        for d in &inst.differentials {
            s.push_str(&format!(
                "  Diferencial {} A / {} mA {}P: {}\n",
                d.rating,
                d.sensitivity,
                d.poles,
                d.circuits
                    .iter()
                    .map(|n| format!("C{n}"))
                    .collect::<Vec<_>>()
                    .join(", ")
            ));
        }
        for c in &inst.circuits {
            s.push_str(&format!(
                "  {}{} {}: {} punto{}, {} W, {} A, PIA {} A curva {}, {} mm², tubo Ø{}, {} m, ΔU {} %, Icc final {} (dispara con {}){}{}\n",
                c.code(),
                if c.phase.is_some() {
                    format!(" ({})", c.phase_text())
                } else {
                    String::new()
                },
                c.spec.name,
                c.points.len(),
                if c.points.len() == 1 { "" } else { "s" },
                fmt(c.power, 0),
                fmt(c.current, 1),
                c.breaker,
                site.curve.name(),
                section_text(c.chosen),
                c.tube,
                fmt(c.length, 1),
                fmt(c.drop * 100.0, 2),
                ka(c.fault_end),
                ka(c.trip / 1000.0),
                if c.trips() { "" } else { " NO DISPARA" },
                if c.raised_for_trip {
                    format!(
                        " (sección subida para que dispare; con {} mm² caería un {} %)",
                        section_text(c.section),
                        fmt(c.table_drop * 100.0, 2)
                    )
                } else if c.bigger_section() {
                    format!(
                        " (con {} mm² caería un {} %: se sube la sección)",
                        section_text(c.section),
                        fmt(c.table_drop * 100.0, 2)
                    )
                } else {
                    String::new()
                }
            ));
            let path = c.worst_path();
            if path.len() > 1 {
                s.push_str(&format!(
                    "     por tramos: {}\n",
                    path.iter()
                        .map(|t| format!(
                            "{} m {} W {} %",
                            fmt(t.length, 1),
                            fmt(t.power, 0),
                            fmt(t.drop_at * 100.0, 2)
                        ))
                        .collect::<Vec<_>>()
                        .join(" → ")
                ));
            }
        }
        s.push_str(&format!(
            "  Caída de tensión desde la CGP hasta el punto más lejano: {} %\n",
            fmt(inst.total_drop() * 100.0, 2)
        ));
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;
    use archisim_core::NewDevice;

    fn dev(m: &mut Model, level: LevelId, kind: DeviceKind, x: f64, y: f64) -> DeviceId {
        named(m, level, kind, kind.name(), x, y)
    }

    fn named(
        m: &mut Model,
        level: LevelId,
        kind: DeviceKind,
        name: &str,
        x: f64,
        y: f64,
    ) -> DeviceId {
        let (w, d, h, z) = kind.defaults();
        m.add_device(NewDevice {
            level,
            kind,
            name: name.into(),
            position: Point2::new(x, y),
            rotation: 0.0,
            elevation: z,
            width: w,
            depth: d,
            height: h,
        })
        .unwrap()
    }

    #[test]
    fn the_numbers_of_the_rebt() {
        // C1 with its 30 points: 30·200·0,75·0,5 = 2 250 W, under 10 A.
        let c1 = CIRCUITS[0];
        let full = 30.0 * c1.point_power * c1.simultaneity * c1.utilisation / VOLTAGE;
        assert!(full < f64::from(c1.breaker), "{full}");
        // 10 m of 1,5 mm² with 2 250 W drop 1,18 %.
        let d = drop(10.0, 2250.0, 1.5);
        assert!((d - 0.011815).abs() < 1e-5, "{d}");
        assert!(ampacity(1.5) > 10.0 && ampacity(2.5) > 16.0 && ampacity(6.0) > 25.0);
    }

    #[test]
    fn a_small_flat() {
        let mut m = Model::new("Piso");
        let l = m.add_level("Planta", 0.0);
        let board = dev(&mut m, l, DeviceKind::Panel, 0.0, 0.0);
        for i in 0..8 {
            dev(&mut m, l, DeviceKind::Light, 1.0 + i as f64, 2.0);
            dev(&mut m, l, DeviceKind::Socket, 1.0 + i as f64, 0.2);
        }
        dev(&mut m, l, DeviceKind::PowerSocket, 6.0, 4.0);
        let all = installations(&m);
        assert_eq!(all.len(), 1);
        let inst = &all[0];
        assert_eq!(inst.board, Some(board));
        assert_eq!(inst.grade, Electrification::Basic);
        assert_eq!(inst.main_switch, 25);
        assert_eq!(inst.circuits.len(), 5);
        assert_eq!(inst.differentials.len(), 1);
        let c1 = &inst.circuits[0];
        assert_eq!(c1.points.len(), 8);
        assert!((c1.power - 8.0 * 200.0 * 0.75 * 0.5).abs() < 1e-9);
        assert!(c1.protected());
        assert!(c1.drop <= CIRCUIT_DROP);
        assert!(inst.feeder.section >= 6.0);
        assert!(inst.feeder.drop <= 0.015);
        // C4 and C5 with no points drawn are still there.
        assert_eq!(inst.circuits[3].points.len(), 0);
        assert!(report(&m).contains("C3 Cocina y horno"));
    }

    #[test]
    fn far_sockets_take_a_bigger_section_and_many_lights_a_high_grade() {
        let mut m = Model::new("Casa");
        let l = m.add_level("Planta", 0.0);
        dev(&mut m, l, DeviceKind::Panel, 0.0, 0.0);
        dev(&mut m, l, DeviceKind::Socket, 80.0, 60.0);
        for i in 0..32 {
            dev(&mut m, l, DeviceKind::Light, f64::from(i) * 0.5, 1.0);
        }
        let inst = &installations(&m)[0];
        let c2 = inst.circuits.iter().find(|c| c.spec.number == 2).unwrap();
        assert!(c2.length > 140.0);
        assert!(c2.bigger_section(), "{c2:?}");
        assert!(c2.drop <= CIRCUIT_DROP);
        assert_eq!(inst.grade, Electrification::High);
        assert_eq!(inst.main_switch, 40);
        assert!(inst.circuits[0].too_many_points());
    }

    fn room(m: &mut Model, level: LevelId, name: &str, x0: f64, x1: f64, y1: f64) {
        m.add_room(archisim_core::NewRoom {
            level,
            number: format!("{x0}"),
            name: name.into(),
            outline: vec![
                Point2::new(x0, 0.0),
                Point2::new(x1, 0.0),
                Point2::new(x1, y1),
                Point2::new(x0, y1),
            ],
            holes: Vec::new(),
            label: Point2::new((x0 + x1) / 2.0, y1 / 2.0),
            bounded: false,
        })
        .unwrap();
    }

    #[test]
    fn the_wires_go_point_to_point_and_drop_stretch_by_stretch() {
        let mut m = Model::new("Piso");
        let l = m.add_level("Planta", 0.0);
        dev(&mut m, l, DeviceKind::Panel, 0.0, 0.0);
        for x in [3.0, 6.0, 9.0] {
            dev(&mut m, l, DeviceKind::Light, x, 0.0);
        }
        let inst = &installations(&m)[0];
        let c1 = &inst.circuits[0];
        assert_eq!(c1.stretches.len(), 3);
        // A chain: the first stretch carries the three lights, the last one.
        let path = c1.worst_path();
        assert_eq!(path.len(), 3);
        let share = c1.power / 3.0;
        assert!((path[0].power - 3.0 * share).abs() < 1e-9);
        assert!((path[2].power - share).abs() < 1e-9);
        // The drop adds up stretch by stretch, to less than the whole
        // power over the whole length.
        let sum: f64 = path.iter().map(|s| s.drop).sum();
        assert!((sum - c1.drop).abs() < 1e-12);
        assert!(c1.drop < drop(c1.length, c1.power, c1.chosen));
        // Between lights: 3 m along the ceiling and down to each box.
        assert!(
            path[1].length > 3.0 && path[1].length < 3.5,
            "{:?}",
            path[1]
        );
        assert!((c1.length - path[2].distance).abs() < 1e-9);
    }

    #[test]
    fn three_phase_shares_the_circuits_out() {
        let mut m = Model::new("Piso");
        let l = m.add_level("Planta", 0.0);
        dev(&mut m, l, DeviceKind::Panel, 0.0, 0.0);
        dev(&mut m, l, DeviceKind::Light, 3.0, 2.0);
        let mut site = *m.electrical_site();
        site.supply = Supply::Three;
        m.set_electrical_site(site).unwrap();
        let inst = &installations(&m)[0];
        assert!(inst.three_phase);
        assert_eq!(inst.poles(), "4P");
        assert!(inst.feeder.three_phase);
        assert!(inst.feeder.wires().starts_with("4×"));
        let i = 5750.0 / (3f64.sqrt() * 400.0 * COS_PHI);
        assert!((inst.feeder.current - i).abs() < 1e-9);
        // Every circuit on a phase, the loads adding up.
        assert!(inst.circuits.iter().all(|c| c.phase.is_some()));
        let sum: f64 = inst.circuits.iter().map(|c| c.power).sum();
        assert!((inst.phase_loads.iter().sum::<f64>() - sum).abs() < 1e-6);
        // C3 (4 050 W) alone on its phase.
        let c3 = inst.circuits.iter().find(|c| c.spec.number == 3).unwrap();
        let alone = inst.circuits.iter().filter(|c| c.phase == c3.phase).count();
        assert_eq!(alone, 1);
        assert!(inst.unbalance > 0.0);
        assert!(inst.differentials.iter().all(|d| d.poles == 4));
        // A shop of 200 m² is 20 kW: three phase by itself.
        let mut m = Model::new("Local");
        let l = m.add_level("Planta", 0.0);
        named(&mut m, l, DeviceKind::Panel, "Cuadro del local", 5.0, 5.0);
        dev(&mut m, l, DeviceKind::Light, 3.0, 2.0);
        room(&mut m, l, "Local comercial", 0.0, 20.0, 10.0);
        let inst = &installations(&m)[0];
        assert_eq!(inst.use_, BoardUse::Local);
        assert!((inst.power - 20_000.0).abs() < 1e-6);
        assert!(inst.three_phase);
        assert_eq!(inst.main_switch, 40);
        // Not C1 to C5: only what has points.
        assert_eq!(inst.circuits.len(), 1);
    }

    #[test]
    fn a_building_forecast_and_its_general_feeder() {
        assert!((simultaneity(1) - 1.0).abs() < 1e-12);
        assert!((simultaneity(21) - 15.3).abs() < 1e-12);
        assert!((simultaneity(24) - 16.8).abs() < 1e-12);
        let mut m = Model::new("Edificio");
        let l0 = m.add_level("Planta baja", 0.0);
        let l1 = m.add_level("Planta 1", 3.0);
        named(&mut m, l0, DeviceKind::Panel, "Cuadro general", 0.0, 0.0);
        dev(&mut m, l0, DeviceKind::Light, 2.0, 0.0);
        for x in [10.0, 20.0, 30.0] {
            named(&mut m, l1, DeviceKind::Panel, "Cuadro de vivienda", x, 0.0);
            dev(&mut m, l1, DeviceKind::Light, x + 2.0, 1.0);
        }
        let mut site = *m.electrical_site();
        site.single_user = false;
        m.set_electrical_site(site).unwrap();
        let e = analysis(&m);
        let b = e.building.as_ref().unwrap();
        assert_eq!(b.dwellings, 3);
        assert!((b.dwellings_power - 3.0 * 5750.0).abs() < 1e-6);
        assert!(b.services > 0.0);
        assert!((b.total - b.dwellings_power - b.services).abs() < 1e-6);
        assert!(b.three_phase);
        let g = b.general.unwrap();
        assert!(g.section >= GENERAL_MIN_SECTION);
        assert!((g.limit - GENERAL_DROP_CENTRALISED).abs() < 1e-12);
        assert!(g.passes());
        // The feeders measured from the meters, by the general board: the
        // third dwelling 30 m away, one floor up.
        let third = &e.installations[3];
        assert!(third.feeder.measured);
        assert!(third.feeder.length > 30.0 && third.feeder.length < 35.0);
        assert!((third.feeder.limit - FEEDER_DROP_CENTRALISED).abs() < 1e-12);
        // The dwellings over the phases: one on each.
        assert!(b.phase_loads.iter().all(|p| *p >= 5750.0 - 1e-6));
        // The short circuit goes down from the supply to the boards.
        assert!(b.fault_supply > b.fault_meters);
        assert!(b.fault_meters > third.fault);
        assert!(third.general_drop > 0.0);
        assert!(third.total_drop() > third.feeder.drop);
        let text = report(&m);
        assert!(text.contains("Previsión de cargas"));
        assert!(text.contains("Línea general de alimentación"));
    }

    #[test]
    fn the_short_circuit_breaks_and_trips() {
        let mut m = Model::new("Casa");
        let l = m.add_level("Planta", 0.0);
        dev(&mut m, l, DeviceKind::Panel, 0.0, 0.0);
        dev(&mut m, l, DeviceKind::Socket, 95.0, 0.0);
        let mut site = *m.electrical_site();
        // 20 kA at the supply and a feeder of 4 m: more than 6 kA at the
        // board.
        site.supply_fault = 20.0;
        site.feeder_length = 4.0;
        m.set_electrical_site(site).unwrap();
        let inst = &installations(&m)[0];
        let r = upstream_resistance(&site) + phase_resistance(4.0, inst.feeder.section);
        assert!((inst.fault - 0.8 * 230.0 / r / 1000.0).abs() < 1e-9);
        assert!(inst.fault > 6.0 && inst.fault < 20.0, "{}", inst.fault);
        assert!(!inst.breaks());
        assert_eq!(inst.breaking_needed(), Some(10.0));
        // With curve D, the far socket's 2,5 mm² does not trip at 20·16 A:
        // its section goes up.
        site.curve = TripCurve::D;
        site.supply_fault = 0.0;
        site.feeder_length = 15.0;
        m.set_electrical_site(site).unwrap();
        let inst = &installations(&m)[0];
        assert!(inst.breaks());
        let c2 = inst.circuits.iter().find(|c| c.spec.number == 2).unwrap();
        assert!((c2.trip - 320.0).abs() < 1e-9);
        assert!(c2.raised_for_trip, "{c2:?}");
        assert!(c2.trips());
        assert!(c2.chosen > 2.5);
        assert!(c2.fault_end < c2.fault_start);
    }
}
