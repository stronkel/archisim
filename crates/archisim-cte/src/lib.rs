//! Checks of the Spanish building code (Código Técnico de la Edificación)
//! on a model:
//!
//! - occupancy of every room and level (DB-SI 3, 2);
//! - exits, evacuation routes, and the width and swing of the doors and
//!   corridors they go through (DB-SI 3, 3 to 6);
//! - accessibility: accessible routes, doors, corridors, turning circles
//!   and ramps (DB-SUA 9 and Anejo A; DB-SUA 1, 4.3), as the building's
//!   settings ask;
//! - stairs (DB-SUA 1, 4.2), as the stairs check themselves;
//! - floor areas: usable area by room, use and level, and built area;
//! - water supply and drainage (DB-HS 4 and HS 5): the diameters of the
//!   pipes of each sanitary appliance and the slope of the drains.
//!
//! What is checked is described in `docs/CTE.md`. The checks read the model
//! as it is drawn: a room's use says what it is, doors connect the rooms
//! on both sides, and an outside door on the ground floor is an exit of the
//! building; on other floors the exits are the stairs.

pub mod access;
pub mod evacuation;
pub mod occupancy;
pub mod region;
pub mod services;
pub mod structure;
pub mod surfaces;

use archisim_core::{
    Accessibility, BuildingUse, LevelId, Model, ObjectId, OpeningId, Point2, RoomId, RunId, SlabId,
    StairId,
};

/// How an item came out.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Status {
    /// It does not meet the code.
    Fails,
    /// Something to look at: the check could not be made in full, or the
    /// result depends on what the drawing does not say.
    Warning,
    Passes,
    /// A figure, not a check (occupancy, areas).
    Info,
}

/// Which part of the code an item checks.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Section {
    /// DB-SI 3: occupancy and evacuation.
    Evacuation,
    /// DB-SUA 9 and Anejo A: accessibility.
    Accessibility,
    /// DB-SUA 1: stairs and ramps.
    StairsAndRamps,
    /// DB-HS 4 and HS 5: water supply and drainage.
    Services,
    /// Código Estructural: the reinforcement (E11).
    Structure,
    /// Floor areas.
    Areas,
}

impl Section {
    pub const ALL: [Section; 6] = [
        Section::Evacuation,
        Section::Accessibility,
        Section::StairsAndRamps,
        Section::Services,
        Section::Structure,
        Section::Areas,
    ];

    pub fn title(self) -> &'static str {
        match self {
            Section::Evacuation => "Evacuación (DB-SI 3)",
            Section::Accessibility => "Accesibilidad (DB-SUA 9)",
            Section::StairsAndRamps => "Escaleras y rampas (DB-SUA 1)",
            Section::Services => "Agua y saneamiento (DB-HS 4 y HS 5)",
            Section::Structure => "Armado (Código Estructural)",
            Section::Areas => "Superficies",
        }
    }
}

/// The element an item is about.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Subject {
    Building,
    Level(LevelId),
    Room(RoomId),
    Opening(OpeningId),
    Stair(StairId),
    Slab(SlabId),
    /// A sanitary appliance.
    Object(ObjectId),
    /// A pipe.
    Run(RunId),
    /// A column, beam, footing or slab with reinforcement (by its id).
    Reinforced(u64),
}

/// One result.
#[derive(Debug, Clone, PartialEq)]
pub struct Item {
    pub section: Section,
    pub status: Status,
    pub subject: Subject,
    pub level: Option<LevelId>,
    /// The rule, as the code numbers it ("SI 3, tabla 3.1").
    pub rule: &'static str,
    /// What is checked and what was found, in a sentence.
    pub text: String,
    /// The value found and what is asked ("27,4 m", "≤ 25,0 m").
    pub value: String,
    pub required: String,
    /// An evacuation route to draw: the longest of a level.
    pub route: Vec<Point2>,
}

/// Everything found.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Report {
    pub items: Vec<Item>,
    /// Occupancy and area of every room.
    pub rooms: Vec<surfaces::RoomRow>,
    /// Areas and occupancy of every level.
    pub levels: Vec<surfaces::LevelRow>,
}

impl Report {
    pub fn count(&self, status: Status) -> usize {
        self.items.iter().filter(|i| i.status == status).count()
    }

    pub fn items_of(&self, section: Section) -> impl Iterator<Item = &Item> {
        self.items.iter().filter(move |i| i.section == section)
    }

    /// The report as text, one line per item, by section.
    pub fn to_text(&self, model: &Model) -> String {
        let s = model.code_settings();
        let mut out = format!(
            "Comprobación CTE · {}\nUso: {} · Accesibilidad: {} · Extinción automática: {}\n\nNo cumple: {} · Avisos: {} · Cumple: {}\n",
            model.project_name,
            use_name(s.building_use),
            accessibility_name(s.accessibility),
            if s.sprinklers { "sí" } else { "no" },
            self.count(Status::Fails),
            self.count(Status::Warning),
            self.count(Status::Passes),
        );
        for section in Section::ALL {
            let items: Vec<&Item> = self.items_of(section).collect();
            if items.is_empty() {
                continue;
            }
            out.push_str(&format!("\n{}\n", section.title()));
            for i in items {
                let mark = match i.status {
                    Status::Fails => "NO CUMPLE",
                    Status::Warning => "AVISO    ",
                    Status::Passes => "cumple   ",
                    Status::Info => "         ",
                };
                let figures = match (i.value.is_empty(), i.required.is_empty()) {
                    (true, _) => String::new(),
                    (false, true) => format!(" [{}]", i.value),
                    (false, false) => format!(" [{} · exigido {}]", i.value, i.required),
                };
                let who = match i.subject {
                    Subject::Building | Subject::Level(_) => String::new(),
                    other => format!("{} · ", subject_name(model, other)),
                };
                out.push_str(&format!("  {mark} {who}{}{figures} ({})\n", i.text, i.rule));
            }
        }
        out
    }

    /// The items as CSV (semicolons, as Spanish spreadsheets read them).
    pub fn to_csv(&self, model: &Model) -> String {
        let q = |s: &str| format!("\"{}\"", s.replace('"', "\"\""));
        let mut out =
            String::from("Apartado;Estado;Planta;Elemento;Regla;Descripción;Valor;Exigido\n");
        for i in &self.items {
            let level = i
                .level
                .and_then(|l| model.level(l))
                .map(|l| l.name.clone())
                .unwrap_or_default();
            out.push_str(&format!(
                "{};{};{};{};{};{};{};{}\n",
                q(i.section.title()),
                status_name(i.status),
                q(&level),
                q(&subject_name(model, i.subject)),
                q(i.rule),
                q(&i.text),
                q(&i.value),
                q(&i.required)
            ));
        }
        out
    }
}

pub fn status_name(s: Status) -> &'static str {
    match s {
        Status::Fails => "No cumple",
        Status::Warning => "Aviso",
        Status::Passes => "Cumple",
        Status::Info => "Dato",
    }
}

pub fn use_name(u: BuildingUse) -> &'static str {
    match u {
        BuildingUse::Dwelling => "Residencial Vivienda",
        BuildingUse::PublicResidential => "Residencial Público",
        BuildingUse::Administrative => "Administrativo",
        BuildingUse::Teaching => "Docente",
        BuildingUse::Hospital => "Hospitalario",
        BuildingUse::Commercial => "Comercial",
        BuildingUse::Assembly => "Pública Concurrencia",
        BuildingUse::Parking => "Aparcamiento",
    }
}

pub fn accessibility_name(a: Accessibility) -> &'static str {
    match a {
        Accessibility::None => "no se comprueba",
        Accessibility::Routes => "itinerarios accesibles",
        Accessibility::WheelchairDwelling => "vivienda accesible para usuarios de silla de ruedas",
    }
}

/// How an element is named in the report.
pub fn subject_name(model: &Model, s: Subject) -> String {
    match s {
        Subject::Building => "Edificio".into(),
        Subject::Level(l) => model.level(l).map(|l| l.name.clone()).unwrap_or_default(),
        Subject::Room(r) => model
            .room(r)
            .map(|r| {
                if r.number.is_empty() {
                    r.name.clone()
                } else {
                    format!("{} {}", r.number, r.name)
                }
            })
            .unwrap_or_default(),
        Subject::Opening(o) => model
            .opening(o)
            .and_then(|o| model.opening_type(o.opening_type))
            .map(|t| {
                if t.mark.is_empty() {
                    t.name.clone()
                } else {
                    format!("{} {}", t.mark, t.name)
                }
            })
            .unwrap_or_default(),
        Subject::Stair(s) => format!("Escalera {}", s.0),
        Subject::Slab(s) => format!("Rampa (forjado {})", s.0),
        Subject::Object(o) => model
            .objects()
            .find(|x| x.id == o)
            .and_then(|x| model.object_type(x.object_type))
            .map(|t| t.name.clone())
            .unwrap_or_default(),
        Subject::Run(r) => model
            .run(r)
            .map(|r| format!("{} {} ({})", r.system.name(), r.size_text(), r.id.0))
            .unwrap_or_default(),
        Subject::Reinforced(id) => structure::element_name(model, id),
    }
}

/// Metres with two decimals and a comma.
pub fn m(x: f64) -> String {
    format!("{:.2} m", x).replace('.', ",")
}

pub fn m2(x: f64) -> String {
    format!("{:.2} m²", x).replace('.', ",")
}

pub fn pct(x: f64) -> String {
    format!("{:.1} %", x * 100.0).replace('.', ",")
}

/// Every check on the model.
pub fn check(model: &Model) -> Report {
    let mut report = Report::default();
    let (rooms, levels) = surfaces::table(model);
    report.rooms = rooms;
    report.levels = levels;
    let levels = report.levels.clone();
    surfaces::items(&levels, &mut report);
    evacuation::check(model, &mut report);
    access::check(model, &mut report);
    access::stairs_and_ramps(model, &mut report);
    services::check(model, &mut report);
    structure::check(model, &mut report);
    report
}

#[cfg(test)]
mod tests;
