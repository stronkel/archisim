//! The electrical installation of a dwelling, by the REBT, on what is
//! drawn: the board («cuadro general de mando y protección», a device of
//! the kind `Panel`), the mechanisms with their circuits (C1 to C10,
//! ITC-BT-25 tabla 1) and the appliances that take a circuit of their own
//! (the washing machine and the dishwasher, on C4).
//!
//! For each board:
//!
//! - the **degree of electrification** (ITC-BT-10): the one given, or high
//!   with more than 160 m² of rooms, more points than the basic circuits
//!   take, or a mechanism on a circuit of the high ones;
//! - every **circuit**: its points (C1 to C5 always, with one point when
//!   none is drawn), its design power P = n·Pa·Fs·Fu and current I = P/U,
//!   its breaker, section and tube (tabla 1), the current its wires carry
//!   (UNE-HD 60364-5-52, tabla B.52.2, method B1: in a tube in the wall,
//!   PVC, two loaded conductors), its length from the board to its
//!   farthest point (along the floor and up to the ceiling, as wires go)
//!   and its voltage drop ΔU = 2·L·P/(γ·S·U) against 3 % (ITC-BT-19 2.2.2);
//!   when the section of the table does not do, the one that does;
//! - the **main switch** («interruptor general automático», 25 A basic,
//!   40 A high, or more if the circuits ask for it), the **residual current
//!   devices** (40 A, 30 mA, one for every five circuits, ITC-BT-25 2.3)
//!   and the **surge protector** (ITC-BT-23);
//! - the **feeder** from the meter (ITC-BT-15): the section that carries
//!   its current and drops 1,5 % at most with a single user (1 % with
//!   centralised meters), 6 mm² at least.
//!
//! Single phase at 230 V, copper at 70 °C (γ = 48 m/(Ω·mm²)), cos φ = 1.

use std::collections::BTreeMap;

use archisim_core::{
    circuit_spec, CircuitSpec, DeviceId, DeviceKind, Electrification, LevelId, Model, ObjectId,
    Point2, CIRCUITS,
};

/// The voltage, V.
pub const VOLTAGE: f64 = 230.0;
/// The conductivity of copper at 70 °C, m/(Ω·mm²).
pub const GAMMA: f64 = 48.0;
/// The most a circuit of a dwelling may drop, from the board (ITC-BT-19).
pub const CIRCUIT_DROP: f64 = 0.03;
/// Where the wires run: this high over each floor (under the ceiling).
pub const WIRE_HEIGHT: f64 = 2.5;
/// The sections of the cables, mm².
pub const SECTIONS: [f64; 9] = [1.5, 2.5, 4.0, 6.0, 10.0, 16.0, 25.0, 35.0, 50.0];

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
        _ => 151.0,
    }
}

/// The voltage drop of a single-phase line, as a share of the voltage.
pub fn drop(length: f64, power: f64, section: f64) -> f64 {
    2.0 * length * power / (GAMMA * section * VOLTAGE * VOLTAGE)
}

/// The breakers made, A.
pub const BREAKERS: [u32; 8] = [10, 16, 20, 25, 32, 40, 50, 63];

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
    /// The section that does (the table's, or more for the voltage drop),
    /// mm².
    pub chosen: f64,
    /// The current its wires carry, A.
    pub ampacity: f64,
    /// From the board to its farthest point, m (an assumed 10 m when it
    /// has no point drawn).
    pub length: f64,
    /// Voltage drop with the chosen section, share of the voltage.
    pub drop: f64,
    /// Voltage drop with the table's section.
    pub table_drop: f64,
    /// Tube, mm.
    pub tube: u32,
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

    pub fn bigger_section(&self) -> bool {
        self.chosen > self.section + 1e-9
    }

    /// «C1».
    pub fn code(&self) -> String {
        format!("C{}", self.spec.number)
    }
}

/// The feeder from the meter to the board («derivación individual»).
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
}

/// A residual current device and the circuits after it.
#[derive(Debug, Clone, PartialEq)]
pub struct Differential {
    /// A.
    pub rating: u32,
    /// mA.
    pub sensitivity: u32,
    pub circuits: Vec<u8>,
}

/// A dwelling's installation: what hangs from one board.
#[derive(Debug, Clone, PartialEq)]
pub struct Installation {
    /// The board drawn (`None`: none is drawn; the calculation stands it
    /// at the first mechanism).
    pub board: Option<DeviceId>,
    pub level: LevelId,
    pub position: Point2,
    /// The height of the board's middle over its level.
    pub z: f64,
    pub grade: Electrification,
    /// Why high, when it is worked out.
    pub reasons: Vec<String>,
    /// Area of its rooms, m².
    pub area: f64,
    /// The power it is designed for, W.
    pub power: f64,
    /// Main switch, A.
    pub main_switch: u32,
    pub feeder: Feeder,
    pub differentials: Vec<Differential>,
    pub circuits: Vec<Circuit>,
}

/// The height of the middle of a device over its level.
fn mid(d: &archisim_core::ServiceDevice) -> f64 {
    d.elevation + d.height / 2.0
}

/// Every installation of the model: one per board, with the mechanisms
/// nearest to it; one with no board if mechanisms are drawn and no board.
pub fn installations(model: &Model) -> Vec<Installation> {
    let level_z = |l: LevelId| model.level(l).map_or(0.0, |l| l.elevation);
    let boards: Vec<&archisim_core::ServiceDevice> = model
        .devices()
        .filter(|d| d.kind == DeviceKind::Panel)
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
        return Vec::new();
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
    for r in model.rooms() {
        let c = archisim_core::rooms::centroid(&r.outline);
        areas[near(r.level, c)] += r.area();
    }
    let site = *model.electrical_site();
    let mut out = Vec::new();
    for (i, pts) in by_board.into_iter().enumerate() {
        let (board, level, position, z) = match boards.get(i) {
            Some(b) => (Some(b.id), b.level, b.position, mid(b)),
            None => {
                let Some(p) = pts.first() else { continue };
                (None, p.1.level, p.1.position, 1.8)
            }
        };
        if board.is_none() && pts.is_empty() {
            continue;
        }
        let mut grouped: BTreeMap<u8, Vec<CircuitPoint>> = BTreeMap::new();
        for (n, p) in pts {
            grouped.entry(n).or_default().push(p);
        }
        // The degree of electrification.
        let mut reasons = Vec::new();
        let area = areas[i];
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
        let grade = match site.grade {
            Electrification::Auto if reasons.is_empty() => Electrification::Basic,
            Electrification::Auto => Electrification::High,
            g => g,
        };
        // The circuits: C1 to C5 always, the others with points.
        let mut circuits = Vec::new();
        for spec in CIRCUITS.iter() {
            let list = grouped.remove(&spec.number).unwrap_or_default();
            if spec.high_only && list.is_empty() {
                continue;
            }
            circuits.push(work_circuit(model, spec, list, level, position, z));
        }
        // Circuits not of the table: as a C2.
        for (n, list) in grouped {
            if n == 0 {
                continue;
            }
            let mut spec = CIRCUITS[1];
            spec.number = n;
            spec.name = "Circuito propio";
            circuits.push(work_circuit(model, &spec, list, level, position, z));
        }
        let designed = match grade {
            Electrification::High => 9200.0,
            _ => 5750.0,
        };
        // The power of the degree (ITC-BT-10), and its main switch.
        let power = designed;
        let main_switch = if grade == Electrification::High {
            40
        } else {
            25
        };
        let differentials = circuits
            .chunks(5)
            .map(|c| Differential {
                rating: if main_switch > 40 { 63 } else { 40 },
                sensitivity: 30,
                circuits: c.iter().map(|c| c.spec.number).collect(),
            })
            .collect();
        let feeder = feeder(&site, power);
        out.push(Installation {
            board,
            level,
            position,
            z,
            grade,
            reasons,
            area,
            power,
            main_switch,
            feeder,
            differentials,
            circuits,
        });
    }
    out
}

/// The feeder that carries `power` from the meter.
fn feeder(site: &archisim_core::ElectricalSite, power: f64) -> Feeder {
    let limit = if site.single_user { 0.015 } else { 0.01 };
    let current = power / VOLTAGE;
    let section = SECTIONS
        .into_iter()
        .filter(|s| *s >= 6.0)
        .find(|s| ampacity(*s) >= current && drop(site.feeder_length, power, *s) <= limit)
        .unwrap_or(50.0);
    Feeder {
        length: site.feeder_length,
        power,
        current,
        section,
        ampacity: ampacity(section),
        drop: drop(site.feeder_length, power, section),
        limit,
        tube: match section {
            s if s <= 10.0 => 32,
            s if s <= 16.0 => 40,
            _ => 50,
        },
    }
}

fn work_circuit(
    model: &Model,
    spec: &CircuitSpec,
    points: Vec<CircuitPoint>,
    level: LevelId,
    board: Point2,
    board_z: f64,
) -> Circuit {
    let level_z = |l: LevelId| model.level(l).map_or(0.0, |l| l.elevation);
    let count = (points.len() as u32).max(1);
    let power = f64::from(count) * spec.point_power * spec.simultaneity * spec.utilisation;
    // A single point takes its own power (an oven, a dryer).
    let power = power.max(if count == 1 && spec.number != 1 && spec.number != 6 {
        spec.point_power * spec.utilisation
    } else {
        0.0
    });
    let current = power / VOLTAGE;
    let length = points
        .iter()
        .map(|p| {
            let d = p.position - board;
            d.x.abs()
                + d.y.abs()
                + (WIRE_HEIGHT - board_z).abs()
                + (WIRE_HEIGHT - p.z).abs()
                + (level_z(p.level) - level_z(level)).abs()
        })
        .fold(0.0, f64::max);
    let length = if points.is_empty() { 10.0 } else { length };
    let table_drop = drop(length, power, spec.section);
    let chosen = SECTIONS
        .into_iter()
        .filter(|s| *s >= spec.section)
        .find(|s| {
            drop(length, power, *s) <= CIRCUIT_DROP && ampacity(*s) >= f64::from(spec.breaker)
        })
        .unwrap_or(50.0);
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
        length,
        drop: drop(length, power, chosen),
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

/// The installation in words, for the window and the report.
pub fn report(model: &Model) -> String {
    let all = installations(model);
    if all.is_empty() {
        return "No hay cuadro ni mecanismos eléctricos dibujados.\n".into();
    }
    let mut s = String::new();
    for (i, inst) in all.iter().enumerate() {
        let name = inst
            .board
            .and_then(|b| model.device(b))
            .map_or("Cuadro (sin dibujar)".to_string(), |d| d.name.clone());
        s.push_str(&format!("{}. {}\n", i + 1, name));
        s.push_str(&format!(
            "  Electrificación {} · {} W · IGA {} A{}\n",
            match inst.grade {
                Electrification::High => "elevada",
                _ => "básica",
            },
            fmt(inst.power, 0),
            inst.main_switch,
            if inst.reasons.is_empty() {
                String::new()
            } else {
                format!(" ({})", inst.reasons.join("; "))
            }
        ));
        let f = inst.feeder;
        s.push_str(&format!(
            "  Derivación individual: {} m, {} A, 2×{}+TT×{} mm² Cu, tubo Ø{}, ΔU {} % (≤ {} %)\n",
            fmt(f.length, 1),
            fmt(f.current, 1),
            section_text(f.section),
            section_text(f.section),
            f.tube,
            fmt(f.drop * 100.0, 2),
            fmt(f.limit * 100.0, 1)
        ));
        for d in &inst.differentials {
            s.push_str(&format!(
                "  Diferencial {} A / {} mA: {}\n",
                d.rating,
                d.sensitivity,
                d.circuits
                    .iter()
                    .map(|n| format!("C{n}"))
                    .collect::<Vec<_>>()
                    .join(", ")
            ));
        }
        for c in &inst.circuits {
            s.push_str(&format!(
                "  {} {}: {} punto{}, {} W, {} A, PIA {} A, {} mm², tubo Ø{}, {} m, ΔU {} %{}\n",
                c.code(),
                c.spec.name,
                c.points.len(),
                if c.points.len() == 1 { "" } else { "s" },
                fmt(c.power, 0),
                fmt(c.current, 1),
                c.breaker,
                section_text(c.chosen),
                c.tube,
                fmt(c.length, 1),
                fmt(c.drop * 100.0, 2),
                if c.bigger_section() {
                    format!(
                        " (con {} mm² caería un {} %: se sube la sección)",
                        section_text(c.section),
                        fmt(c.table_drop * 100.0, 2)
                    )
                } else {
                    String::new()
                }
            ));
        }
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;
    use archisim_core::NewDevice;

    fn dev(m: &mut Model, level: LevelId, kind: DeviceKind, x: f64, y: f64) -> DeviceId {
        let (w, d, h, z) = kind.defaults();
        m.add_device(NewDevice {
            level,
            kind,
            name: kind.name().into(),
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
}
