//! The fire protection installations (DB-SI 4, tabla 1.1), on the devices
//! drawn (H3): which installations the building's use and size ask for,
//! and how far each room is from them — an extinguisher within 15 m of
//! every point, a hose reel within 25 m (its 20 m hose and 5 m of jet) and
//! no further than 50 m from the next, a call point within 25 m —; the
//! sprinklers, one for 21 m² at most and 4,6 m apart (UNE-EN 12845, riesgo
//! ligero), and the smoke detectors, one for 60 m² (UNE 23007-14).
//!
//! Distances are in a straight line inside the level, so they are the
//! least the route can be: a room that fails, fails; one that passes has
//! its route to check where walls are in the way.

use std::collections::BTreeMap;

use archisim_core::{BuildingUse, DeviceKind, LevelId, Model, Point2, Room};

use crate::occupancy::{is_common, room_text};
use crate::{m, m2, Item, Report, Section, Status, Subject};

/// The furthest an extinguisher may be from any point (SI 4, tabla 1.1).
pub const EXTINGUISHER_REACH: f64 = 15.0;
/// A hose reel's reach: 20 m of hose and 5 m of jet (RIPCI, anexo I).
pub const HOSE_REACH: f64 = 25.0;
/// The most between two hose reels (RIPCI, anexo I).
pub const HOSE_APART: f64 = 50.0;
/// The furthest a call point may be from any point (RIPCI, anexo I).
pub const CALL_POINT_REACH: f64 = 25.0;
/// Floor area a sprinkler covers at most, and how far apart they may be
/// (UNE-EN 12845, riesgo ligero).
pub const SPRINKLER_AREA: f64 = 21.0;
pub const SPRINKLER_APART: f64 = 4.6;
/// Floor area an optical smoke detector covers (UNE 23007-14).
pub const DETECTOR_AREA: f64 = 60.0;

/// From what floor area (m²) of the rooms drawn a use asks for each
/// installation (SI 4, tabla 1.1): hose reels, alarm, detection; `Some(0)`
/// always, `None` never.
fn thresholds(use_: BuildingUse) -> [Option<f64>; 3] {
    match use_ {
        BuildingUse::Dwelling => [None, None, None],
        BuildingUse::PublicResidential => [Some(1000.0), Some(500.0), Some(500.0)],
        // Detection everywhere from 5.000 m² (from 2.000 m², only in the
        // zones of high risk, which are not drawn).
        BuildingUse::Administrative => [Some(2000.0), Some(1000.0), Some(5000.0)],
        BuildingUse::Teaching => [Some(2000.0), Some(1000.0), Some(5000.0)],
        BuildingUse::Hospital => [Some(0.0), Some(0.0), Some(0.0)],
        BuildingUse::Commercial => [Some(500.0), Some(1000.0), Some(2000.0)],
        // Its alarm goes by the occupants (more than 500), not the area.
        BuildingUse::Assembly => [Some(500.0), None, Some(1000.0)],
        BuildingUse::Parking => [Some(500.0), None, Some(500.0)],
    }
}

/// The rooms whose points the extinguishers must reach: all of them, but
/// in a building of dwellings only the shared ones (the dwellings
/// themselves need none).
fn reached(use_: BuildingUse, r: &Room) -> bool {
    if use_ != BuildingUse::Dwelling {
        return true;
    }
    let t = room_text(r);
    is_common(&t)
        || [
            "garaje",
            "aparcamiento",
            "trastero",
            "instalaciones",
            "cuarto de",
        ]
        .iter()
        .any(|w| t.contains(w))
}

/// Points of a room to measure from: its corners, the middle of its sides
/// and its label.
fn room_points(r: &Room) -> Vec<Point2> {
    let n = r.outline.len();
    let mut v = r.outline.clone();
    for i in 0..n {
        let (a, b) = (r.outline[i], r.outline[(i + 1) % n]);
        v.push(a + (b - a) * 0.5);
    }
    v.push(r.label);
    v
}

/// The furthest point of a room from the nearest of those devices.
fn furthest(r: &Room, at: &[Point2]) -> f64 {
    room_points(r)
        .into_iter()
        .map(|p| {
            at.iter()
                .map(|q| p.distance(*q))
                .fold(f64::INFINITY, f64::min)
        })
        .fold(0.0, f64::max)
}

struct Level<'a> {
    rooms: Vec<&'a Room>,
    by_kind: BTreeMap<DeviceKind, Vec<Point2>>,
}

#[allow(clippy::too_many_arguments)]
fn item(
    report: &mut Report,
    status: Status,
    subject: Subject,
    level: LevelId,
    rule: &'static str,
    text: String,
    value: String,
    required: String,
) {
    report.items.push(Item {
        section: Section::Fire,
        status,
        subject,
        level: Some(level),
        rule,
        text,
        value,
        required,
        route: Vec::new(),
    });
}

/// Each room within reach of one of those devices of its level.
#[allow(clippy::too_many_arguments)]
fn reach(
    report: &mut Report,
    level: LevelId,
    rooms: &[&Room],
    at: &[Point2],
    limit: f64,
    rule: &'static str,
    what: &str,
    to: &str,
) {
    for r in rooms {
        let d = furthest(r, at);
        item(
            report,
            if d <= limit + 1e-6 {
                Status::Passes
            } else {
                Status::Fails
            },
            Subject::Room(r.id),
            level,
            rule,
            format!(
                "{what}: el punto más lejano de «{}» está a {} {to} (en línea recta)",
                r.name,
                m(d)
            ),
            m(d),
            format!("≤ {}", m(limit)),
        );
    }
}

pub fn check(model: &Model, report: &mut Report) {
    let settings = model.code_settings();
    let use_ = settings.building_use;
    let mut levels: BTreeMap<LevelId, Level> = BTreeMap::new();
    for r in model.rooms() {
        levels
            .entry(r.level)
            .or_insert_with(|| Level {
                rooms: Vec::new(),
                by_kind: BTreeMap::new(),
            })
            .rooms
            .push(r);
    }
    let mut any: BTreeMap<DeviceKind, usize> = BTreeMap::new();
    for d in model.devices().filter(|d| d.kind.is_fire()) {
        *any.entry(d.kind).or_default() += 1;
        levels
            .entry(d.level)
            .or_insert_with(|| Level {
                rooms: Vec::new(),
                by_kind: BTreeMap::new(),
            })
            .by_kind
            .entry(d.kind)
            .or_default()
            .push(d.position);
    }
    let has = |k: DeviceKind| any.get(&k).copied().unwrap_or(0) > 0;
    let area: f64 = model.rooms().map(|r| r.area()).sum();
    let [hose_from, alarm_from, detect_from] = thresholds(use_);
    let asks = |from: Option<f64>| from.is_some_and(|a| area > a || a == 0.0);
    let building =
        |report: &mut Report, status, rule, text: String, value: String, required: String| {
            report.items.push(Item {
                section: Section::Fire,
                status,
                subject: Subject::Building,
                level: None,
                rule,
                text,
                value,
                required,
                route: Vec::new(),
            });
        };
    // What the use and the size ask for.
    for (from, kinds, name) in [
        (
            hose_from,
            &[DeviceKind::HoseReel][..],
            "Bocas de incendio equipadas",
        ),
        (
            alarm_from,
            &[DeviceKind::CallPoint, DeviceKind::Siren][..],
            "Sistema de alarma (pulsadores y sirenas)",
        ),
        (
            detect_from,
            &[DeviceKind::SmokeDetector][..],
            "Sistema de detección de incendio",
        ),
    ] {
        if !asks(from) {
            continue;
        }
        let present = kinds.iter().all(|k| has(*k));
        let limit = from.unwrap_or(0.0);
        building(
            report,
            if present {
                Status::Passes
            } else {
                Status::Fails
            },
            "SI 4, tabla 1.1",
            format!(
                "{name}: {} en un edificio de uso {} con {} de locales",
                if present { "dibujado" } else { "no dibujado" },
                crate::use_name(use_),
                m2(area)
            ),
            if present { "sí".into() } else { "no".into() },
            if limit == 0.0 {
                "en todo caso".into()
            } else {
                format!("si > {}", m2(limit))
            },
        );
    }
    if settings.sprinklers && !has(DeviceKind::Sprinkler) {
        building(
            report,
            Status::Fails,
            "SI 4, tabla 1.1",
            "El edificio se declara con extinción automática y no hay rociadores dibujados".into(),
            "0".into(),
            "rociadores".into(),
        );
    }
    for (&level, lv) in &levels {
        let at = |k: DeviceKind| lv.by_kind.get(&k).cloned().unwrap_or_default();
        // Extinguishers: every point of the rooms within 15 m.
        let rooms: Vec<&Room> = lv
            .rooms
            .iter()
            .copied()
            .filter(|r| reached(use_, r))
            .collect();
        let ext = at(DeviceKind::Extinguisher);
        if !rooms.is_empty() {
            if ext.is_empty() {
                item(
                    report,
                    Status::Fails,
                    Subject::Level(level),
                    level,
                    "SI 4, tabla 1.1",
                    format!(
                        "No hay extintores en la planta; {} {} deben tenerlo a {} como mucho",
                        rooms.len(),
                        if rooms.len() == 1 { "local" } else { "locales" },
                        m(EXTINGUISHER_REACH)
                    ),
                    "0".into(),
                    "≥ 1".into(),
                );
            } else {
                reach(
                    report,
                    level,
                    &rooms,
                    &ext,
                    EXTINGUISHER_REACH,
                    "SI 4, tabla 1.1",
                    "Extintor",
                    "del más cercano",
                );
            }
        }
        // Hose reels: within 25 m, and no further than 50 m apart.
        let hoses = at(DeviceKind::HoseReel);
        if !hoses.is_empty() {
            reach(
                report,
                level,
                &lv.rooms,
                &hoses,
                HOSE_REACH,
                "RIPCI, anexo I, BIE",
                "BIE",
                "de la más cercana",
            );
            if hoses.len() > 1 {
                for (i, p) in hoses.iter().enumerate() {
                    let d = hoses
                        .iter()
                        .enumerate()
                        .filter(|(j, _)| *j != i)
                        .map(|(_, q)| p.distance(*q))
                        .fold(f64::INFINITY, f64::min);
                    item(
                        report,
                        if d <= HOSE_APART {
                            Status::Passes
                        } else {
                            Status::Fails
                        },
                        Subject::Level(level),
                        level,
                        "RIPCI, anexo I, BIE",
                        format!("BIE {}: la siguiente está a {}", i + 1, m(d)),
                        m(d),
                        format!("≤ {}", m(HOSE_APART)),
                    );
                }
            }
        }
        // Call points: within 25 m.
        let calls = at(DeviceKind::CallPoint);
        if !calls.is_empty() {
            reach(
                report,
                level,
                &lv.rooms,
                &calls,
                CALL_POINT_REACH,
                "RIPCI, anexo I, alarma",
                "Pulsador",
                "del más cercano",
            );
        }
        // Sprinklers and detectors: as many as the area of each room they
        // are in asks for, and the sprinklers close enough together.
        for (kind, per, rule, name) in [
            (
                DeviceKind::Sprinkler,
                SPRINKLER_AREA,
                "UNE-EN 12845, riesgo ligero",
                "Rociadores",
            ),
            (
                DeviceKind::SmokeDetector,
                DETECTOR_AREA,
                "UNE 23007-14",
                "Detectores",
            ),
        ] {
            let pts = at(kind);
            if pts.is_empty() {
                if kind == DeviceKind::Sprinkler && settings.sprinklers && !lv.rooms.is_empty() {
                    item(
                        report,
                        Status::Fails,
                        Subject::Level(level),
                        level,
                        "SI 4, tabla 1.1",
                        "El edificio se declara con extinción automática y la planta no tiene rociadores"
                            .into(),
                        "0".into(),
                        "rociadores".into(),
                    );
                }
                continue;
            }
            for r in &lv.rooms {
                let inside: Vec<Point2> = pts.iter().copied().filter(|p| r.contains(*p)).collect();
                if inside.is_empty() {
                    continue;
                }
                let needed = (r.area() / per).ceil().max(1.0) as usize;
                item(
                    report,
                    if inside.len() >= needed {
                        Status::Passes
                    } else {
                        Status::Fails
                    },
                    Subject::Room(r.id),
                    level,
                    rule,
                    format!(
                        "{name} en «{}»: {} para {} (uno por cada {})",
                        r.name,
                        inside.len(),
                        m2(r.area()),
                        m2(per)
                    ),
                    inside.len().to_string(),
                    format!("≥ {needed}"),
                );
                if kind == DeviceKind::Sprinkler && inside.len() > 1 {
                    let worst = inside
                        .iter()
                        .enumerate()
                        .map(|(i, p)| {
                            inside
                                .iter()
                                .enumerate()
                                .filter(|(j, _)| *j != i)
                                .map(|(_, q)| p.distance(*q))
                                .fold(f64::INFINITY, f64::min)
                        })
                        .fold(0.0, f64::max);
                    item(
                        report,
                        if worst <= SPRINKLER_APART + 1e-6 {
                            Status::Passes
                        } else {
                            Status::Fails
                        },
                        Subject::Room(r.id),
                        level,
                        rule,
                        format!(
                            "Rociadores en «{}»: el más apartado está a {} del siguiente",
                            r.name,
                            m(worst)
                        ),
                        m(worst),
                        format!("≤ {}", m(SPRINKLER_APART)),
                    );
                }
            }
        }
        // Emergency lighting and signs: what there is.
        let lights = at(DeviceKind::EmergencyLight).len();
        let signs = at(DeviceKind::ExitSign).len();
        if lights + signs > 0 || (use_ != BuildingUse::Dwelling && !lv.rooms.is_empty()) {
            item(
                report,
                if lights == 0 && use_ != BuildingUse::Dwelling {
                    Status::Warning
                } else {
                    Status::Info
                },
                Subject::Level(level),
                level,
                "SUA 4, 2 y SI 3, 7",
                format!("{lights} luces de emergencia y {signs} señales de salida en la planta"),
                lights.to_string(),
                String::new(),
            );
        }
    }
}
