//! Accessibility (CTE DB-SUA 9 and its Anejo A) and stairs and ramps
//! (DB-SUA 1, 4).
//!
//! What is checked depends on the building's settings: accessible routes
//! (doors, corridors and turning circles of halls and accessible toilets),
//! or a dwelling for wheelchair users (corridors, doors and the turning
//! circles of hall, living room, bedrooms, kitchen, one bathroom and
//! terraces). Turning circles must be clear of furniture and stairs.

use archisim_core::{Accessibility, BuildingUse, Model, Point2, Room};

use crate::evacuation::{doors, Door, LEAF_THICKNESS};
use crate::occupancy::{is_common, is_corridor, is_hall, room_text};
use crate::region::Region;
use crate::{m, pct, Item, Report, Section, Status, Subject};

fn has(text: &str, words: &[&str]) -> bool {
    words.iter().any(|w| text.contains(w))
}

/// The furniture and stairs on a room's floor, as outlines.
fn obstacles(model: &Model, room: &Room) -> Vec<Vec<Point2>> {
    let mut out: Vec<Vec<Point2>> = model
        .objects_on_level(room.level)
        .map(|o| model.object_outline(o.id))
        .filter(|o| o.len() >= 3 && o.iter().any(|p| room.contains(*p)))
        .collect();
    for s in model.stairs().filter(|s| s.level == room.level) {
        out.extend(
            s.outlines()
                .into_iter()
                .filter(|o| o.iter().any(|p| room.contains(*p))),
        );
    }
    out
}

/// The diameter of the largest free circle in a room.
pub fn free_diameter(model: &Model, room: &Room) -> f64 {
    Region::new(room.outline.clone(), room.holes.clone())
        .largest_free_circle(&obstacles(model, room))
        .map_or(0.0, |(_, r)| 2.0 * r)
}

fn turning_item(model: &Model, room: &Room, diameter: f64, what: &str) -> Item {
    let d = free_diameter(model, room);
    Item {
        section: Section::Accessibility,
        status: if d >= diameter - 0.005 {
            Status::Passes
        } else {
            Status::Fails
        },
        subject: Subject::Room(room.id),
        level: Some(room.level),
        rule: "SUA, Anejo A",
        text: format!(
            "Espacio de giro libre de obstáculos de Ø {} ({what})",
            m(diameter).trim_end_matches(" m")
        ),
        value: format!("Ø {}", m(d)),
        required: format!("≥ Ø {}", m(diameter)),
        route: Vec::new(),
    }
}

fn door_item(model: &Model, d: &Door, why: &str) -> Vec<Item> {
    let _ = model;
    let mut v = vec![Item {
        section: Section::Accessibility,
        status: if d.leaf >= 0.80 - 1e-6 {
            Status::Passes
        } else {
            Status::Fails
        },
        subject: Subject::Opening(d.id),
        level: Some(d.level),
        rule: "SUA, Anejo A (puertas)",
        text: format!("Anchura libre de paso de una hoja, medida en el marco ({why})"),
        value: m(d.leaf),
        required: "≥ 0,80 m".into(),
        route: Vec::new(),
    }];
    if d.swing.is_some() && d.leaf >= 0.80 - 1e-6 && d.leaf - LEAF_THICKNESS < 0.78 - 1e-6 {
        v.push(Item {
            section: Section::Accessibility,
            status: Status::Fails,
            subject: Subject::Opening(d.id),
            level: Some(d.level),
            rule: "SUA, Anejo A (puertas)",
            text: "Con la hoja abierta, descontando su grosor, quedan menos de 0,78 m".into(),
            value: m(d.leaf - LEAF_THICKNESS),
            required: "≥ 0,78 m".into(),
            route: Vec::new(),
        });
    }
    v
}

fn corridor_item(room: &Room, min: f64, why: &str) -> Item {
    let w = Region::new(room.outline.clone(), room.holes.clone()).width();
    Item {
        section: Section::Accessibility,
        status: if w >= min - 0.005 {
            Status::Passes
        } else {
            Status::Fails
        },
        subject: Subject::Room(room.id),
        level: Some(room.level),
        rule: "SUA, Anejo A (pasillos)",
        text: format!("Anchura libre del pasillo ({why})"),
        value: m(w),
        required: format!("≥ {}", m(min)),
        route: Vec::new(),
    }
}

/// Whether a toilet's door opens outwards or slides.
fn toilet_door_ok(doors: &[Door], room: &Room) -> Option<bool> {
    let ds: Vec<&Door> = doors
        .iter()
        .filter(|d| d.rooms.contains(&Some(room.id)))
        .collect();
    if ds.is_empty() {
        return None;
    }
    Some(ds.iter().all(|d| {
        if d.sliding || d.swing.is_none() {
            return true;
        }
        let side = d
            .rooms
            .iter()
            .position(|r| *r == Some(room.id))
            .unwrap_or(0);
        d.opens_towards(side) == Some(true)
    }))
}

pub fn check(model: &Model, report: &mut Report) {
    let s = model.code_settings();
    if s.accessibility == Accessibility::None {
        return;
    }
    lifts(model, report);
    let doors = doors(model);
    let dwelling_use = s.building_use == BuildingUse::Dwelling;
    match s.accessibility {
        Accessibility::None => {}
        Accessibility::Routes => {
            // In a dwelling building the routes are those of the common
            // zones; in the others, every zone (the private ones of a
            // dwelling are not public).
            let public = |r: &Room| !dwelling_use || is_common(&room_text(r));
            for d in &doors {
                let on_route = d
                    .rooms
                    .iter()
                    .flatten()
                    .any(|r| model.room(*r).is_some_and(public));
                if on_route {
                    report
                        .items
                        .extend(door_item(model, d, "itinerario accesible"));
                }
            }
            for r in model.rooms() {
                let t = room_text(r);
                if !public(r) {
                    continue;
                }
                if is_corridor(&t) {
                    let min = if dwelling_use { 1.10 } else { 1.20 };
                    report
                        .items
                        .push(corridor_item(r, min, "itinerario accesible"));
                }
                if is_hall(&t) {
                    report
                        .items
                        .push(turning_item(model, r, 1.50, "vestíbulo de entrada"));
                }
                if has(&t, &["accesible", "adaptado"])
                    && has(&t, &["aseo", "bano", "servicio", "vestuario"])
                {
                    report
                        .items
                        .push(turning_item(model, r, 1.50, "aseo accesible"));
                    if toilet_door_ok(&doors, r) == Some(false) {
                        report.items.push(Item {
                            section: Section::Accessibility,
                            status: Status::Fails,
                            subject: Subject::Room(r.id),
                            level: Some(r.level),
                            rule: "SUA, Anejo A (aseo accesible)",
                            text: "Las puertas del aseo accesible deben abrir hacia fuera o ser correderas".into(),
                            value: "abre hacia dentro".into(),
                            required: "hacia fuera o corredera".into(),
                            route: Vec::new(),
                        });
                    }
                }
            }
            if s.building_use != BuildingUse::Dwelling
                && !model.rooms().any(|r| {
                    let t = room_text(r);
                    has(&t, &["accesible", "adaptado"]) && has(&t, &["aseo", "bano"])
                })
                && model.rooms().any(|r| has(&room_text(r), &["aseo", "bano"]))
            {
                report.items.push(Item {
                    section: Section::Accessibility,
                    status: Status::Warning,
                    subject: Subject::Building,
                    level: None,
                    rule: "SUA 9, 1.2.6",
                    text: "Hay aseos pero ninguno es accesible: el uso de uno debe decir «Aseo accesible» para comprobarlo (uno por cada 10 aseos o fracción, cuando sean exigibles)".into(),
                    value: String::new(),
                    required: String::new(),
                    route: Vec::new(),
                });
            }
        }
        Accessibility::WheelchairDwelling => {
            for d in &doors {
                // Inside the dwelling: every door between two rooms, and
                // its entrance.
                if d.rooms.iter().flatten().next().is_some() {
                    report
                        .items
                        .extend(door_item(model, d, "vivienda accesible"));
                }
            }
            let mut main: Option<(&Room, f64)> = None;
            let mut baths: Vec<&Room> = Vec::new();
            for r in model.rooms() {
                let t = room_text(r);
                if is_corridor(&t) {
                    report
                        .items
                        .push(corridor_item(r, 1.10, "vivienda accesible"));
                } else if is_hall(&t) {
                    report
                        .items
                        .push(turning_item(model, r, 1.50, "vestíbulo de la vivienda"));
                } else if has(&t, &["dormitorio"]) {
                    report
                        .items
                        .push(turning_item(model, r, 1.50, "dormitorio"));
                } else if has(&t, &["cocina"]) {
                    report.items.push(turning_item(model, r, 1.50, "cocina"));
                } else if has(&t, &["terraza", "balcon"]) {
                    report.items.push(turning_item(model, r, 1.20, "terraza"));
                } else if has(&t, &["bano", "aseo"]) {
                    baths.push(r);
                } else if has(&t, &["salon", "estar", "estancia", "comedor"])
                    && main.is_none_or(|(_, a)| r.area() > a)
                {
                    main = Some((r, r.area()));
                }
            }
            if let Some((r, _)) = main {
                report
                    .items
                    .push(turning_item(model, r, 1.50, "estancia principal"));
            }
            if !baths.is_empty() {
                // At least one bathroom: the best of them.
                let best = baths
                    .iter()
                    .map(|b| {
                        let ok_door = toilet_door_ok(&doors, b) != Some(false);
                        (b, free_diameter(model, b), ok_door)
                    })
                    .max_by(|a, b| {
                        (a.2, a.1)
                            .partial_cmp(&(b.2, b.1))
                            .unwrap_or(std::cmp::Ordering::Equal)
                    });
                if let Some((b, _, ok_door)) = best {
                    let mut item = turning_item(model, b, 1.50, "al menos un baño");
                    if !ok_door {
                        item.status = Status::Fails;
                        item.text
                            .push_str("; sus puertas deben abrir hacia fuera o ser correderas");
                    }
                    report.items.push(item);
                }
            }
            let steps = model.stairs().count();
            if steps > 0 {
                report.items.push(Item {
                    section: Section::Accessibility,
                    status: Status::Warning,
                    subject: Subject::Building,
                    level: None,
                    rule: "SUA, Anejo A (vivienda accesible)",
                    text: "En una vivienda accesible no se admiten escalones: las escaleras necesitan ascensor accesible o rampa que comunique las plantas".into(),
                    value: format!("{steps} escalera(s)"),
                    required: String::new(),
                    route: Vec::new(),
                });
            }
        }
    }
}

/// Stairs (their own check, SUA 1, 4.2) and ramps (sloping slabs, SUA 1,
/// 4.3).
pub fn stairs_and_ramps(model: &Model, report: &mut Report) {
    use archisim_core::StairIssue as I;
    let s = model.code_settings();
    for st in model.stairs() {
        let issues = st.issues();
        let usage = match st.usage {
            archisim_core::StairUse::General => "uso general",
            archisim_core::StairUse::Restricted => "uso restringido",
        };
        if issues.is_empty() {
            report.items.push(Item {
                section: Section::StairsAndRamps,
                status: Status::Passes,
                subject: Subject::Stair(st.id),
                level: Some(st.level),
                rule: "SUA 1, 4.2",
                text: format!("Huella, tabica, anchura, tramos y pasamanos ({usage})"),
                value: String::new(),
                required: String::new(),
                route: Vec::new(),
            });
        }
        for i in issues {
            let (text, value, required) = match i {
                I::TreadTooShort { going, min } => {
                    ("Huella".to_string(), m(going), format!("≥ {}", m(min)))
                }
                I::RiserOutOfRange { riser, min, max } => {
                    ("Tabica".into(), m(riser), format!("{}–{}", m(min), m(max)))
                }
                I::StepRule { value } => ("Relación 2C + H".into(), m(value), "0,54–0,70 m".into()),
                I::TooNarrow { width, min } => {
                    ("Anchura útil".into(), m(width), format!("≥ {}", m(min)))
                }
                I::ShortFlight { risers } => (
                    "Tramo con menos de 3 peldaños".into(),
                    format!("{risers} peldaños"),
                    "≥ 3".into(),
                ),
                I::FlightTooHigh { height, max } => (
                    "Altura salvada por un tramo".into(),
                    m(height),
                    format!("≤ {}", m(max)),
                ),
                I::NoHandrail => (
                    "Pasamanos (salva 55 cm o más)".into(),
                    "sin pasamanos".into(),
                    "al menos en un lado".into(),
                ),
                I::HandrailBothSides => (
                    "Pasamanos en ambos lados (anchura ≥ 1,20 m)".into(),
                    "en un lado".into(),
                    "en ambos lados".into(),
                ),
                I::NarrowEdge { going, min } => (
                    "Huella de peldaño curvo en su lado estrecho".into(),
                    m(going),
                    format!("≥ {}", m(min)),
                ),
                I::WideEdge { going, max } => (
                    "Huella de peldaño curvo en su lado ancho".into(),
                    m(going),
                    format!("≤ {}", m(max)),
                ),
                I::OpenRisers => (
                    "Peldaños sin tabica: no valen en zonas de uso público ni en evacuación ascendente".into(),
                    "sin tabica".into(),
                    "con tabica".into(),
                ),
                I::SplitLanding => (
                    "Mesetas partidas con peldaños a 45°: solo se admiten en escaleras de uso restringido".into(),
                    "meseta partida".into(),
                    "meseta entera".into(),
                ),
            };
            report.items.push(Item {
                section: Section::StairsAndRamps,
                status: if i == I::OpenRisers {
                    Status::Warning
                } else {
                    Status::Fails
                },
                subject: Subject::Stair(st.id),
                level: Some(st.level),
                rule: "SUA 1, 4.2",
                text: format!("{text} ({usage})"),
                value,
                required,
                route: Vec::new(),
            });
        }
        if st.usage == archisim_core::StairUse::Restricted
            && s.building_use != BuildingUse::Dwelling
        {
            report.items.push(Item {
                section: Section::StairsAndRamps,
                status: Status::Warning,
                subject: Subject::Stair(st.id),
                level: Some(st.level),
                rule: "SUA 1, 4.2.1",
                text: "Escalera de uso restringido en un edificio que no es de viviendas: solo vale en zonas de uso restringido (menos de 10 personas habituales)".into(),
                value: String::new(),
                required: String::new(),
                route: Vec::new(),
            });
        }
    }
    let accessible = s.accessibility != Accessibility::None;
    for sl in model.slabs() {
        let Some(sp) = sl.slope else { continue };
        let slope = sp.slope.abs();
        // A slab sloping more than a ramp is not walked on (roofs and
        // vehicle ramps aside).
        if !(0.04..=0.20).contains(&slope) {
            continue;
        }
        let Some(dir) = sp.direction() else { continue };
        let along: Vec<f64> = sl.outline.iter().map(|p| (*p - sp.from).dot(dir)).collect();
        let length = along.iter().copied().fold(f64::NEG_INFINITY, f64::max)
            - along.iter().copied().fold(f64::INFINITY, f64::min);
        let (max, why) = if accessible {
            if length < 3.0 {
                (0.10, "rampa accesible de menos de 3 m")
            } else if length < 6.0 {
                (0.08, "rampa accesible de menos de 6 m")
            } else {
                (0.06, "rampa accesible")
            }
        } else {
            (0.12, "rampa de uso general")
        };
        report.items.push(Item {
            section: Section::StairsAndRamps,
            status: if slope <= max + 1e-6 {
                Status::Passes
            } else if slope > 0.12 && !accessible {
                Status::Warning
            } else {
                Status::Fails
            },
            subject: Subject::Slab(sl.id),
            level: Some(sl.level),
            rule: "SUA 1, 4.3.1",
            text: format!(
                "Pendiente de {why} ({} de largo){}",
                m(length),
                if slope > 0.12 && !accessible {
                    "; si es para vehículos, no es una rampa peatonal"
                } else {
                    ""
                }
            ),
            value: pct(slope),
            required: format!("≤ {}", pct(max)),
            route: Vec::new(),
        });
        if accessible && length > 9.0 {
            report.items.push(Item {
                section: Section::StairsAndRamps,
                status: Status::Fails,
                subject: Subject::Slab(sl.id),
                level: Some(sl.level),
                rule: "SUA 1, 4.3.2",
                text: "Los tramos de una rampa accesible miden 9 m como máximo (con mesetas entre ellos)".into(),
                value: m(length),
                required: "≤ 9,00 m".into(),
                route: Vec::new(),
            });
        }
    }
}

/// The lifts of an accessible route (SUA, Anejo A, «ascensor accesible»):
/// a car of 1,00 × 1,25 m at least (1,10 × 1,40 m in buildings of more
/// than 1.000 m² of other uses; with doors at an angle, 1,40 × 1,40 m,
/// which the lift does not have) and doors of 0,80 m clear.
fn lifts(model: &Model, report: &mut Report) {
    let big = model.code_settings().building_use != BuildingUse::Dwelling
        && model.rooms().map(|r| r.area()).sum::<f64>() > 1000.0;
    let (w, d) = if big { (1.1, 1.4) } else { (1.0, 1.25) };
    for e in model.elevators() {
        let car_ok = e.car_w >= w - 1e-6 && e.car_d >= d - 1e-6;
        let door_ok = e.door_w >= 0.8 - 1e-6;
        report.items.push(Item {
            section: Section::Accessibility,
            status: if car_ok && door_ok {
                Status::Passes
            } else {
                Status::Fails
            },
            subject: Subject::Building,
            level: Some(e.level),
            rule: "SUA, Anejo A (ascensor accesible)",
            text: format!(
                "{}: cabina de {} × {} y puerta de {}",
                e.name,
                crate::m(e.car_w),
                crate::m(e.car_d),
                crate::m(e.door_w)
            ),
            value: format!("{} × {}", crate::m(e.car_w), crate::m(e.car_d)),
            required: format!("≥ {} × {}, puerta ≥ 0,80 m", crate::m(w), crate::m(d)),
            route: Vec::new(),
        });
    }
}
