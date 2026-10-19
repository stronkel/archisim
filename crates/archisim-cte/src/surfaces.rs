//! Floor areas and occupancy: usable area of every room, by use and level,
//! and the built area of every level (what the walls enclose, walls
//! included).

use std::collections::BTreeMap;

use archisim_core::{LevelId, Model, RoomId};
use archisim_geom::junctions::{union, Shape};

use crate::occupancy::density;
use crate::{m2, Item, Report, Section, Status, Subject};

/// A room's figures.
#[derive(Debug, Clone, PartialEq)]
pub struct RoomRow {
    pub room: RoomId,
    pub level: LevelId,
    pub number: String,
    pub name: String,
    pub usage: String,
    /// Usable area, m².
    pub area: f64,
    /// m² per person; `None`: no occupants.
    pub per_person: Option<f64>,
    pub density_source: String,
    /// Area ÷ area per person, not rounded.
    pub people: f64,
}

/// A level's figures.
#[derive(Debug, Clone, PartialEq)]
pub struct LevelRow {
    pub level: LevelId,
    pub name: String,
    pub usable: f64,
    /// Built area: inside the outer faces of the walls.
    pub built: f64,
    /// Usable area by use.
    pub by_use: BTreeMap<String, f64>,
    /// Occupants, rounded up for the whole level.
    pub people: u32,
}

/// Rooms and levels.
pub fn table(model: &Model) -> (Vec<RoomRow>, Vec<LevelRow>) {
    let use_ = model.code_settings().building_use;
    let mut rooms: Vec<RoomRow> = model
        .rooms()
        .map(|r| {
            let d = density(model, use_, r);
            let area = r.area();
            RoomRow {
                room: r.id,
                level: r.level,
                number: r.number.clone(),
                name: r.name.clone(),
                usage: r.usage.clone(),
                area,
                per_person: d.per_person,
                people: d.per_person.map_or(0.0, |p| area / p),
                density_source: d.source,
            }
        })
        .collect();
    rooms.sort_by(|a, b| {
        level_order(model, a.level)
            .total_cmp(&level_order(model, b.level))
            .then(a.number.cmp(&b.number))
            .then(a.name.cmp(&b.name))
    });
    let mut levels = Vec::new();
    for l in model.levels() {
        let here: Vec<&RoomRow> = rooms.iter().filter(|r| r.level == l.id).collect();
        let walls = archisim_geom::wall_geometries(model, l.id);
        if here.is_empty() && walls.is_empty() {
            continue;
        }
        let mut by_use = BTreeMap::new();
        for r in &here {
            let key = if r.usage.trim().is_empty() {
                "(sin uso)".to_string()
            } else {
                r.usage.clone()
            };
            *by_use.entry(key).or_insert(0.0) += r.area;
        }
        let mut shapes: Vec<Shape> = model
            .rooms_on_level(l.id)
            .map(|r| (r.outline.clone(), Vec::new()))
            .collect();
        shapes.extend(walls.iter().map(|g| (g.outline.clone(), Vec::new())));
        let built = if shapes.is_empty() {
            0.0
        } else {
            // Rooms and walls together, with their courtyards left out
            // but the gaps between them (columns, shafts) in.
            let joined = union(&shapes, &[]);
            joined
                .iter()
                .map(|(o, hs)| {
                    let holes: f64 = hs
                        .iter()
                        .map(|h| archisim_core::signed_area(h).abs())
                        .filter(|a| *a > 1.0)
                        .sum();
                    archisim_core::signed_area(o).abs() - holes
                })
                .sum::<f64>()
        };
        let fraction: f64 = here.iter().map(|r| r.people).sum();
        levels.push(LevelRow {
            level: l.id,
            name: l.name.clone(),
            usable: here.iter().map(|r| r.area).sum(),
            built,
            by_use,
            people: (fraction - 1e-9).ceil().max(0.0) as u32,
        });
    }
    (rooms, levels)
}

fn level_order(model: &Model, l: LevelId) -> f64 {
    model.level(l).map_or(0.0, |l| l.elevation)
}

/// The area and occupancy figures as items.
pub fn items(levels: &[LevelRow], report: &mut Report) {
    let (mut usable, mut built, mut people) = (0.0, 0.0, 0);
    for l in levels {
        usable += l.usable;
        built += l.built;
        people += l.people;
        let uses: Vec<String> = l
            .by_use
            .iter()
            .map(|(u, a)| format!("{u} {}", m2(*a)))
            .collect();
        report.items.push(Item {
            section: Section::Areas,
            status: Status::Info,
            subject: Subject::Level(l.level),
            level: Some(l.level),
            rule: "Superficies",
            text: format!(
                "{}: útil {}, construida {}{}",
                l.name,
                m2(l.usable),
                m2(l.built),
                if uses.is_empty() {
                    String::new()
                } else {
                    format!(" ({})", uses.join(", "))
                }
            ),
            value: m2(l.usable),
            required: String::new(),
            route: Vec::new(),
        });
        report.items.push(Item {
            section: Section::Evacuation,
            status: Status::Info,
            subject: Subject::Level(l.level),
            level: Some(l.level),
            rule: "SI 3, 2 (tabla 2.1)",
            text: format!("Ocupación de {}: {} personas", l.name, l.people),
            value: format!("{} personas", l.people),
            required: String::new(),
            route: Vec::new(),
        });
    }
    if !levels.is_empty() {
        report.items.push(Item {
            section: Section::Areas,
            status: Status::Info,
            subject: Subject::Building,
            level: None,
            rule: "Superficies",
            text: format!(
                "Edificio: útil {}, construida {}, {} personas",
                m2(usable),
                m2(built),
                people
            ),
            value: m2(built),
            required: String::new(),
            route: Vec::new(),
        });
    }
}
