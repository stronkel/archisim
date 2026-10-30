//! Schedules ("tablas de planificación"): doors and windows, walls and
//! slabs by type, and the rooms with their areas and finishes, read from
//! the model as Revit's schedules and ArchiCAD's interactive schedules
//! are. They are drawn on sheets and written as CSV; the measurements go
//! to a BC3 budget (`archisim-export`).

use std::collections::BTreeMap;
use std::fmt::Write;

use archisim_core::{
    signed_area, LevelId, Model, OpeningKind, Phase, Point2, ScheduleKind, Text, TextAlign,
};
use archisim_draft::font::text_width;

use crate::{num, pt};

/// A column of a schedule.
#[derive(Debug, Clone, PartialEq)]
pub struct Column {
    pub name: std::borrow::Cow<'static, str>,
    /// Numbers align to the right.
    pub numeric: bool,
}

/// A schedule: its rows as written, and the totals row if it has one.
#[derive(Debug, Clone, PartialEq)]
pub struct Table {
    pub title: String,
    pub columns: Vec<Column>,
    pub rows: Vec<Vec<String>>,
    pub totals: Option<Vec<String>>,
}

const fn col(name: &'static str, numeric: bool) -> Column {
    Column {
        name: std::borrow::Cow::Borrowed(name),
        numeric,
    }
}

/// A value of a project parameter as a schedule writes it.
fn param_text(kind: &archisim_core::UserKind, v: &archisim_core::Value) -> String {
    use archisim_core::{UserKind, Value};
    match (kind, v) {
        (_, Value::Text(t)) => t.clone(),
        (_, Value::Bool(true)) => "Sí".into(),
        (_, Value::Bool(false)) => "No".into(),
        (UserKind::Integer, Value::Number(n)) => format!("{}", *n as i64),
        (UserKind::Length | UserKind::Area, Value::Number(n)) => dec(*n, 2),
        (_, Value::Number(n)) => dec(*n, 2),
        _ => String::new(),
    }
}

/// Adds to a schedule of door and window types a column for each shared
/// parameter of their families (`ifc "…"` in the recipe: the fire rating of
/// a door…), by its name, with each type's value.
fn with_shared_family_values(mut t: Table, model: &Model, keys: &[u64]) -> Table {
    let mut cols: Vec<(String, String)> = Vec::new(); // (IFC property, label)
    for k in keys {
        let Some(ot) = model.opening_type(archisim_core::OpeningTypeId(*k)) else {
            continue;
        };
        let Some(f) = model.family(ot.family) else {
            continue;
        };
        for p in &f.def().params {
            if let Some((_, prop)) = &p.ifc {
                if !cols.iter().any(|(q, _)| q == prop) {
                    cols.push((prop.clone(), p.label.clone()));
                }
            }
        }
    }
    for (prop, label) in cols {
        t.columns.push(Column {
            name: std::borrow::Cow::Owned(label),
            numeric: false,
        });
        for (row, k) in t.rows.iter_mut().zip(keys) {
            let text = model
                .opening_type(archisim_core::OpeningTypeId(*k))
                .and_then(|ot| {
                    let f = model.family(ot.family)?;
                    let p = f
                        .def()
                        .params
                        .iter()
                        .find(|p| p.ifc.as_ref().is_some_and(|(_, q)| *q == prop))?;
                    ot.values.get(&p.name).map(|v| match v {
                        archisim_core::Value::Text(t) => t.clone(),
                        archisim_core::Value::Bool(b) => {
                            if *b {
                                "Sí".into()
                            } else {
                                "No".into()
                            }
                        }
                        archisim_core::Value::Number(n) => dec(*n, 2),
                        archisim_core::Value::Material(m) => model
                            .material(archisim_core::MaterialId(*m))
                            .map_or(String::new(), |m| m.name.clone()),
                    })
                })
                .unwrap_or_default();
            row.push(text);
        }
        if let Some(tot) = &mut t.totals {
            tot.push(String::new());
        }
    }
    t
}

/// Adds to a schedule a column for each project parameter of its
/// categories (of the type, for schedules by type; of the element, for
/// those by element), with the value of the type or element of each row.
fn with_parameters(
    mut t: Table,
    model: &Model,
    keys: &[u64],
    cats: &[archisim_core::ParamCategory],
    by_type: bool,
) -> Table {
    let params: Vec<&archisim_core::UserParameter> = model
        .user_parameters()
        .filter(|p| p.by_type == by_type && p.categories.iter().any(|c| cats.contains(c)))
        .collect();
    for p in params {
        let unit = match p.kind {
            archisim_core::UserKind::Length => " (m)",
            archisim_core::UserKind::Area => " (m²)",
            _ => "",
        };
        t.columns.push(Column {
            name: std::borrow::Cow::Owned(format!("{}{unit}", p.name)),
            numeric: matches!(
                p.kind,
                archisim_core::UserKind::Number
                    | archisim_core::UserKind::Integer
                    | archisim_core::UserKind::Length
                    | archisim_core::UserKind::Area
            ),
        });
        for (row, key) in t.rows.iter_mut().zip(keys) {
            let v = if by_type {
                model
                    .type_parameters(*key)
                    .into_iter()
                    .find(|(q, _)| q.id == p.id)
                    .map(|(_, v)| v)
            } else {
                model.element_parameter(*key, p.id)
            };
            row.push(v.map(|v| param_text(&p.kind, &v)).unwrap_or_default());
        }
        if let Some(tot) = &mut t.totals {
            tot.push(String::new());
        }
    }
    t
}

fn dec(v: f64, places: usize) -> String {
    let f = 10f64.powi(places as i32);
    let v = (v * f).round() / f;
    let v = if v == 0.0 { 0.0 } else { v };
    format!("{v:.places$}").replace('.', ",")
}

/// One quantity of a type, element by element, for budgets.
#[derive(Debug, Clone, PartialEq)]
pub struct Measured {
    /// What the line is: "Muro 12 · Planta baja".
    pub comment: String,
    pub units: f64,
    pub length: f64,
    pub width: f64,
    pub height: f64,
    /// The quantity it adds (units × the dimensions that apply).
    pub quantity: f64,
}

/// A line of a budget: a type and its measurements.
#[derive(Debug, Clone, PartialEq)]
pub struct Item {
    /// Short code: the type mark, or one made up ("M01").
    pub code: String,
    /// What the item measures, stable while the model changes (a type's
    /// id, a finish's name): prices from a bank are assigned to it.
    pub key: String,
    /// `m2`, `m3`, `m`, `ud`.
    pub unit: &'static str,
    pub summary: String,
    pub lines: Vec<Measured>,
}

impl Item {
    pub fn total(&self) -> f64 {
        self.lines.iter().map(|l| l.quantity).sum()
    }
}

/// Cut and fill of the terrains' platforms (E8), m³.
fn earthworks(model: &Model) -> Vec<Item> {
    let mut cut = Vec::new();
    let mut fill = Vec::new();
    for t in model.terrains().filter(|t| !t.pads.is_empty()) {
        let (c, f) = t.earthworks();
        for (v, list) in [(c, &mut cut), (f, &mut fill)] {
            if v > 0.0005 {
                list.push(Measured {
                    comment: format!("Terreno {} · {} explanaciones", t.id.0, t.pads.len()),
                    units: 1.0,
                    length: 0.0,
                    width: 0.0,
                    height: 0.0,
                    quantity: (v * 1000.0).round() / 1000.0,
                });
            }
        }
    }
    let mut items = Vec::new();
    for (lines, code, key, summary) in [
        (
            cut,
            "T01",
            "tierras:desmonte",
            "Desmonte (excavación a cielo abierto)",
        ),
        (
            fill,
            "T02",
            "tierras:terraplen",
            "Terraplén (relleno compactado)",
        ),
    ] {
        if !lines.is_empty() {
            items.push(Item {
                code: code.into(),
                key: key.into(),
                unit: "m3",
                summary: summary.into(),
                lines,
            });
        }
    }
    items
}

/// The chapters of a budget's measurements: (code, title, items).
pub fn measurements(model: &Model) -> Vec<(&'static str, &'static str, Vec<Item>)> {
    let level_name = |l: LevelId| model.level(l).map_or(String::new(), |l| l.name.clone());
    // A refurbishment: what is there and stays is not measured; what is
    // demolished goes to its own chapter.
    let new = |id: u64| model.phase(id) == Phase::New;
    // Walls by type, m² of one face less the openings.
    let mut walls: BTreeMap<String, Item> = BTreeMap::new();
    for (i, t) in model.wall_types().enumerate() {
        let mut lines: Vec<Measured> = Vec::new();
        for w in model.walls().filter(|w| w.wall_type == t.id && new(w.id.0)) {
            lines.push(Measured {
                comment: format!("Muro {} · {}", w.id.0, level_name(w.level)),
                units: 1.0,
                length: w.length(),
                width: 0.0,
                height: w.height,
                quantity: w.length() * w.height,
            });
            // Its doors and windows, taken off.
            for o in model.openings_in_wall(w.id).filter(|o| new(o.id.0)) {
                if let Some(ot) = model.opening_type(o.opening_type) {
                    let name = if ot.mark.trim().is_empty() {
                        ot.name.clone()
                    } else {
                        ot.mark.clone()
                    };
                    lines.push(Measured {
                        comment: format!("A deducir: hueco {name}"),
                        units: -1.0,
                        length: 0.0,
                        width: ot.width,
                        height: ot.height,
                        // An arch or an oculus deducts its own shape.
                        quantity: -if ot.shape.is_rect() {
                            ot.width * ot.height
                        } else {
                            archisim_core::shape::area(&ot.shape.polygon(ot.width, ot.height)).abs()
                        },
                    });
                }
            }
        }
        if !lines.is_empty() {
            walls.insert(
                format!("{:03}", i),
                Item {
                    code: format!("M{:02}", i + 1),
                    key: format!("muro:{}", t.id.0),
                    unit: "m2",
                    summary: format!("{} ({} cm)", t.name, dec(t.thickness() * 100.0, 1)),
                    lines,
                },
            );
        }
    }
    let mut slabs: BTreeMap<String, Item> = BTreeMap::new();
    for (i, t) in model.slab_types().enumerate() {
        let lines: Vec<Measured> = model
            .slabs()
            .filter(|s| s.slab_type == t.id && new(s.id.0))
            .map(|s| Measured {
                comment: format!("Forjado {} · {}", s.id.0, level_name(s.level)),
                units: 1.0,
                length: 0.0,
                width: 0.0,
                height: 0.0,
                quantity: s.area(),
            })
            .collect();
        if !lines.is_empty() {
            slabs.insert(
                format!("{:03}", i),
                Item {
                    code: format!("F{:02}", i + 1),
                    key: format!("forjado:{}", t.id.0),
                    unit: "m2",
                    summary: format!("{} ({} cm)", t.name, dec(t.thickness() * 100.0, 1)),
                    lines,
                },
            );
        }
    }
    let mut joinery: BTreeMap<String, Item> = BTreeMap::new();
    for (i, t) in model.opening_types().enumerate() {
        if t.kind == OpeningKind::Void {
            continue;
        }
        let lines: Vec<Measured> = model
            .openings()
            .filter(|o| o.opening_type == t.id && new(o.id.0))
            .map(|o| {
                let level = model.wall(o.wall).map(|w| w.level);
                Measured {
                    comment: format!(
                        "{} {} · {}",
                        if t.is_door() { "Puerta" } else { "Ventana" },
                        o.id.0,
                        level.map(level_name).unwrap_or_default()
                    ),
                    units: 1.0,
                    length: 0.0,
                    width: t.width,
                    height: t.height,
                    quantity: 1.0,
                }
            })
            .collect();
        if !lines.is_empty() {
            let code = code_of(&t.mark).unwrap_or_else(|| format!("C{:02}", i + 1));
            joinery.insert(
                format!("{code}{i:03}"),
                Item {
                    code,
                    key: format!("carpinteria:{}", t.id.0),
                    unit: "ud",
                    summary: format!("{} {}×{}", t.name, dec(t.width, 2), dec(t.height, 2)),
                    lines,
                },
            );
        }
    }
    // Finishes of the rooms: floors and ceilings by area, walls by
    // perimeter times height.
    let mut finishes: BTreeMap<(u8, String), Item> = BTreeMap::new();
    for r in model.rooms() {
        let h = model.room_height(r);
        for (kind, name, unit_area) in [
            (0u8, &r.finishes.floor, r.area()),
            (1, &r.finishes.walls, r.perimeter() * h),
            (2, &r.finishes.ceiling, r.area()),
        ] {
            if name.trim().is_empty() {
                continue;
            }
            let what = ["Suelo", "Paredes", "Techo"][kind as usize];
            let n = finishes.len() + 1;
            let item = finishes
                .entry((kind, name.clone()))
                .or_insert_with(|| Item {
                    code: format!("A{n:02}"),
                    key: format!("acabado:{kind}:{name}"),
                    unit: "m2",
                    summary: format!("{what}: {name}"),
                    lines: Vec::new(),
                });
            item.lines.push(Measured {
                comment: format!("{} {} · {}", r.number, r.name, level_name(r.level)),
                units: 1.0,
                length: if kind == 1 { r.perimeter() } else { 0.0 },
                width: 0.0,
                height: if kind == 1 { h } else { 0.0 },
                quantity: unit_area,
            });
        }
    }
    let mut chapters = vec![
        ("00", "Demoliciones", demolitions(model)),
        ("01", "Fábricas y muros", walls.into_values().collect()),
        ("02", "Forjados", slabs.into_values().collect()),
        ("03", "Carpintería", joinery.into_values().collect()),
        (
            "04",
            "Acabados de locales",
            finishes.into_values().collect(),
        ),
        ("05", "Estructura", structure(model)),
        ("06", "Cubiertas", roofs(model)),
        ("07", "Escaleras y barandillas", stairs_and_railings(model)),
        ("08", "Muros cortina", curtain_walls(model)),
        ("09", "Instalaciones", services(model)),
        ("10", "Mobiliario de cocina", kitchen(model)),
        ("11", "Cerrajería y protecciones", outsides(model)),
        ("12", "Movimiento de tierras", earthworks(model)),
    ];
    chapters.retain(|(_, _, items)| !items.is_empty());
    chapters
}

/// What a refurbishment demolishes: walls (m² of one face), slabs (m²),
/// doors and windows taken out (ud), columns and beams (m³).
fn demolitions(model: &Model) -> Vec<Item> {
    let gone = |id: u64| model.phase(id) == Phase::Demolished;
    let level_name = |l: LevelId| model.level(l).map_or(String::new(), |l| l.name.clone());
    let mut walls = Vec::new();
    for w in model.walls().filter(|w| gone(w.id.0)) {
        let name = model
            .wall_type(w.wall_type)
            .map_or(String::new(), |t| t.name.clone());
        walls.push(Measured {
            comment: format!("Muro {} ({name}) · {}", w.id.0, level_name(w.level)),
            units: 1.0,
            length: w.length(),
            width: 0.0,
            height: w.height,
            quantity: w.length() * w.height,
        });
    }
    let slabs: Vec<Measured> = model
        .slabs()
        .filter(|s| gone(s.id.0))
        .map(|s| Measured {
            comment: format!("Forjado {} · {}", s.id.0, level_name(s.level)),
            units: 1.0,
            length: 0.0,
            width: 0.0,
            height: 0.0,
            quantity: s.area(),
        })
        .collect();
    let joinery: Vec<Measured> = model
        .openings()
        .filter(|o| gone(o.id.0))
        .filter_map(|o| {
            let t = model.opening_type(o.opening_type)?;
            (t.kind != OpeningKind::Void).then(|| Measured {
                comment: format!("{} {}×{}", t.name, dec(t.width, 2), dec(t.height, 2)),
                units: 1.0,
                length: 0.0,
                width: t.width,
                height: t.height,
                quantity: 1.0,
            })
        })
        .collect();
    let members: Vec<Measured> = member_lines(model, gone);
    [
        ("D01", "m2", "Demolición de muros y tabiques", walls),
        ("D02", "m2", "Demolición de forjados", slabs),
        ("D03", "ud", "Desmontaje de carpinterías", joinery),
        ("D04", "m3", "Demolición de pilares y vigas", members),
    ]
    .into_iter()
    .filter(|(_, _, _, l)| !l.is_empty())
    .map(|(code, unit, summary, lines)| Item {
        code: code.into(),
        key: format!("demolicion:{code}"),
        unit,
        summary: summary.into(),
        lines,
    })
    .collect()
}

/// Columns and beams that pass `keep`, as m³ of their section.
fn member_lines(model: &Model, keep: impl Fn(u64) -> bool) -> Vec<Measured> {
    let mut v = Vec::new();
    for c in model.columns().filter(|c| keep(c.id.0)) {
        if let Some(t) = model.member_type(c.member_type) {
            let a = t.profile.area();
            v.push(Measured {
                comment: format!("Pilar {} ({})", c.id.0, t.name),
                units: 1.0,
                length: c.height,
                width: 0.0,
                height: 0.0,
                quantity: a * c.height,
            });
        }
    }
    for b in model.beams().filter(|b| keep(b.id.0)) {
        if let Some(t) = model.member_type(b.member_type) {
            let a = t.profile.area();
            v.push(Measured {
                comment: format!("Viga {} ({})", b.id.0, t.name),
                units: 1.0,
                length: b.length(),
                width: 0.0,
                height: 0.0,
                quantity: a * b.length(),
            });
        }
    }
    v
}

/// Density of structural steel, kg/m³.
const STEEL_DENSITY: f64 = 7850.0;

/// Columns and beams by type: concrete and timber in m³, steel in kg.
fn structure(model: &Model) -> Vec<Item> {
    let level_name = |l: LevelId| model.level(l).map_or(String::new(), |l| l.name.clone());
    let mut items = Vec::new();
    for (i, t) in model.member_types().enumerate() {
        let steel = model
            .material(t.material)
            .is_some_and(|m| matches!(m.category.as_str(), "steel" | "metal"));
        let (unit, factor) = if steel {
            ("kg", t.profile.area() * STEEL_DENSITY)
        } else {
            ("m3", t.profile.area())
        };
        let mut lines = Vec::new();
        for c in model
            .columns()
            .filter(|c| c.member_type == t.id && model.phase(c.id.0) == Phase::New)
        {
            lines.push(Measured {
                comment: format!("Pilar {} · {}", c.id.0, level_name(c.level)),
                units: 1.0,
                length: c.height,
                width: 0.0,
                height: 0.0,
                quantity: c.height * factor,
            });
        }
        for b in model
            .beams()
            .filter(|b| b.member_type == t.id && model.phase(b.id.0) == Phase::New)
        {
            lines.push(Measured {
                comment: format!("Viga {} · {}", b.id.0, level_name(b.level)),
                units: 1.0,
                length: b.length(),
                width: 0.0,
                height: 0.0,
                quantity: b.length() * factor,
            });
        }
        if !lines.is_empty() {
            items.push(Item {
                code: format!("E{:02}", i + 1),
                key: format!("estructura:{}", t.id.0),
                unit,
                summary: t.name.clone(),
                lines,
            });
        }
    }
    items
}

/// Pitched roofs by type: m² of their sloping surface.
fn roofs(model: &Model) -> Vec<Item> {
    let level_name = |l: LevelId| model.level(l).map_or(String::new(), |l| l.name.clone());
    let mut items = Vec::new();
    for (i, t) in model.slab_types().enumerate() {
        let lines: Vec<Measured> = model
            .roofs()
            .filter(|r| r.roof_type == t.id && model.phase(r.id.0) == Phase::New)
            .map(|r| Measured {
                comment: format!(
                    "Cubierta {} · {} (en planta {} m²)",
                    r.id.0,
                    level_name(r.level),
                    dec(signed_area(&r.eaves()).abs(), 2)
                ),
                units: 1.0,
                length: 0.0,
                width: 0.0,
                height: 0.0,
                quantity: r.surface_area(),
            })
            .collect();
        if !lines.is_empty() {
            items.push(Item {
                code: format!("T{:02}", i + 1),
                key: format!("cubierta:{}", t.id.0),
                unit: "m2",
                summary: format!("{} (superficie inclinada)", t.name),
                lines,
            });
        }
    }
    items.extend(roof_parts(model));
    items
}

/// What the roofs carry (bloque K): tiles (m², with the pieces), ridge
/// tiles (m), gutters and downpipes (m) and dormers (ud).
fn roof_parts(model: &Model) -> Vec<Item> {
    let new: Vec<&archisim_core::Roof> = model
        .roofs()
        .filter(|r| model.phase(r.id.0) == Phase::New)
        .collect();
    let mut items: Vec<Item> = Vec::new();
    let mut push = |key: String,
                    code: String,
                    unit: &'static str,
                    summary: String,
                    line: Measured| {
        match items.iter_mut().find(|i| i.key == key) {
            Some(i) => i.lines.push(line),
            None => items.push(Item {
                code,
                key,
                unit,
                summary,
                lines: vec![line],
            }),
        }
    };
    let ground = model
        .levels()
        .iter()
        .map(|l| l.elevation)
        .fold(f64::INFINITY, f64::min);
    for r in &new {
        let lz = model.level(r.level).map_or(0.0, |l| l.elevation);
        let line = |comment: String, quantity: f64| Measured {
            comment,
            units: 1.0,
            length: 0.0,
            width: 0.0,
            height: 0.0,
            quantity,
        };
        if let Some(t) = r.covering.tiles {
            let area = r.surface_area();
            let pieces = t.per_m2().map_or(String::new(), |n| {
                format!(" ≈ {} piezas", (area * n).ceil())
            });
            push(
                format!("teja:{}", t.code()),
                format!("TJ{}", t.code()[..2].to_uppercase()),
                "m2",
                format!("Cubrición de {}", t.name().to_lowercase()),
                line(format!("Cubierta {}{pieces}", r.id.0), area),
            );
            let ridge: f64 = r
                .ridge_lines()
                .iter()
                .map(|(p, q)| {
                    let dz = r.top_at(*p) - r.top_at(*q);
                    (p.distance(*q).powi(2) + dz * dz).sqrt()
                })
                .sum();
            if ridge > 0.0 {
                push(
                    "caballete".into(),
                    "TJCB".into(),
                    "m",
                    "Caballete de cumbreras y limatesas".into(),
                    line(format!("Cubierta {}", r.id.0), ridge),
                );
            }
        }
        if let Some(g) = &r.covering.gutter {
            let runs = r.gutter_runs();
            let length: f64 = runs.iter().map(|x| x.start.distance(x.end)).sum();
            push(
                format!("canalon:{}", g.shape.code()),
                "CN01".into(),
                "m",
                format!(
                    "Canalón {} de {} cm",
                    g.shape.name().to_lowercase(),
                    dec(g.size * 100.0, 1)
                ),
                line(format!("Cubierta {}", r.id.0), length),
            );
            let pipes: Vec<f64> = runs
                .iter()
                .flat_map(|x| x.downpipes.iter().map(move |_| lz + x.z - ground))
                .collect();
            push(
                "bajante".into(),
                "CN02".into(),
                "m",
                format!("Bajante de Ø {} mm", dec(g.downpipe * 1000.0, 0)),
                Measured {
                    comment: format!("Cubierta {}: {} bajantes", r.id.0, pipes.len()),
                    units: pipes.len() as f64,
                    length: 0.0,
                    width: 0.0,
                    height: 0.0,
                    quantity: pipes.iter().sum(),
                },
            );
        }
        for d in &r.dormers {
            if r.dormer_shape(d).is_none() {
                continue;
            }
            push(
                format!("buhardilla:{}", d.kind.code()),
                format!("BH{}", d.kind.code()[..2].to_uppercase()),
                "ud",
                d.kind.name().to_string(),
                line(
                    format!(
                        "Cubierta {} · {} cm de ancho",
                        r.id.0,
                        dec(d.width * 100.0, 0)
                    ),
                    1.0,
                ),
            );
        }
    }
    items
}

/// Stairs (ud, with their risers and treads) and railings (m).
fn stairs_and_railings(model: &Model) -> Vec<Item> {
    let level_name = |l: LevelId| model.level(l).map_or(String::new(), |l| l.name.clone());
    let stairs: Vec<Measured> = model
        .stairs()
        .filter(|s| model.phase(s.id.0) == Phase::New)
        .map(|s| {
            let l = s.layout();
            Measured {
                comment: format!(
                    "Escalera {} · {}: {} tabicas de {} mm, huella {} mm, ancho {} m",
                    s.id.0,
                    level_name(s.level),
                    l.risers(),
                    dec(l.riser * 1000.0, 0),
                    dec(s.going * 1000.0, 0),
                    dec(s.width, 2)
                ),
                units: 1.0,
                length: 0.0,
                width: s.width,
                height: s.height,
                quantity: 1.0,
            }
        })
        .collect();
    let railings: Vec<Measured> = model
        .railings()
        .filter(|r| model.phase(r.id.0) == Phase::New)
        .map(|r| {
            let len: f64 = r.path.windows(2).map(|w| w[0].distance(w[1])).sum();
            Measured {
                comment: format!(
                    "Barandilla {} · {}{}",
                    r.id.0,
                    level_name(r.level),
                    if r.glass.is_some() {
                        " (con vidrio)"
                    } else {
                        ""
                    }
                ),
                units: 1.0,
                length: len,
                width: 0.0,
                height: r.height,
                quantity: len,
            }
        })
        .collect();
    let mut v = Vec::new();
    if !stairs.is_empty() {
        v.push(Item {
            code: "S01".into(),
            key: "escalera".into(),
            unit: "ud",
            summary: "Escalera de losa de hormigón armado".into(),
            lines: stairs,
        });
    }
    if !railings.is_empty() {
        v.push(Item {
            code: "S02".into(),
            key: "barandilla".into(),
            unit: "m",
            summary: "Barandilla".into(),
            lines: railings,
        });
    }
    v
}

/// Curtain walls by type: m² of façade.
fn curtain_walls(model: &Model) -> Vec<Item> {
    let level_name = |l: LevelId| model.level(l).map_or(String::new(), |l| l.name.clone());
    let mut items = Vec::new();
    for (i, t) in model.curtain_types().enumerate() {
        let lines: Vec<Measured> = model
            .curtain_walls()
            .filter(|c| c.curtain_type == t.id && model.phase(c.id.0) == Phase::New)
            .map(|c| {
                let len = c.start.distance(c.end);
                Measured {
                    comment: format!("Muro cortina {} · {}", c.id.0, level_name(c.level)),
                    units: 1.0,
                    length: len,
                    width: 0.0,
                    height: c.height,
                    quantity: len * c.height,
                }
            })
            .collect();
        if !lines.is_empty() {
            items.push(Item {
                code: format!("K{:02}", i + 1),
                key: format!("cortina:{}", t.id.0),
                unit: "m2",
                summary: t.name.clone(),
                lines,
            });
        }
    }
    items
}

/// Worktops by material, in metres along the fronts of their modules (a
/// run of modules is one worktop, `Model::worktops`); kitchen furniture
/// (the objects of the kitchen families) by type, in units.
fn kitchen(model: &Model) -> Vec<Item> {
    let level_name = |l: LevelId| model.level(l).map_or(String::new(), |l| l.name.clone());
    let new = |id: u64| model.phase(id) == Phase::New;
    let mut tops: BTreeMap<archisim_core::MaterialId, Vec<Measured>> = BTreeMap::new();
    for run in model.worktops() {
        if !run.members.iter().all(|m| new(m.0)) {
            continue;
        }
        let what = match run.members.len() {
            1 => "Encimera de un módulo".to_string(),
            n => format!("Encimera de {n} módulos"),
        };
        let depth = if run.length > 0.0 {
            run.area / run.length
        } else {
            0.0
        };
        tops.entry(run.material).or_default().push(Measured {
            comment: format!("{what} · {}", level_name(run.level)),
            units: 1.0,
            length: run.length,
            width: depth,
            height: 0.0,
            quantity: run.length,
        });
    }
    // Upstands (copetes), by material, measured along.
    let mut upstands: BTreeMap<archisim_core::MaterialId, Vec<Measured>> = BTreeMap::new();
    for run in model.worktops() {
        if run.upstands.is_empty() || !run.members.iter().all(|m| new(m.0)) {
            continue;
        }
        let length: f64 = run.upstands.iter().map(|(e, _)| e[0].distance(e[1])).sum();
        let height = run.upstands.iter().map(|(_, h)| *h).fold(0.0, f64::max);
        upstands.entry(run.material).or_default().push(Measured {
            comment: format!("Copete · {}", level_name(run.level)),
            units: 1.0,
            length,
            width: 0.0,
            height,
            quantity: length,
        });
    }
    let name_of = |m: archisim_core::MaterialId| {
        model
            .material(m)
            .map_or("?".to_string(), |m| m.name.clone())
    };
    let mut items: Vec<Item> = tops
        .into_iter()
        .enumerate()
        .map(|(i, (m, lines))| Item {
            code: format!("E{:02}", i + 1),
            key: format!("encimera:{}", m.0),
            unit: "m",
            summary: format!("Encimera de {}", name_of(m)),
            lines,
        })
        .collect();
    let first = items.len();
    items.extend(
        upstands
            .into_iter()
            .enumerate()
            .map(|(i, (m, lines))| Item {
                code: format!("E{:02}", first + i + 1),
                key: format!("copete:{}", m.0),
                unit: "m",
                summary: format!("Copete de {}", name_of(m)),
                lines,
            }),
    );
    let mut units: BTreeMap<archisim_core::ObjectTypeId, Vec<Measured>> = BTreeMap::new();
    for o in model.objects().filter(|o| new(o.id.0)) {
        let Some(t) = model.object_type(o.object_type) else {
            continue;
        };
        let kitchen = model
            .family(t.family)
            .is_some_and(|f| f.def().category == "kitchen");
        if !kitchen {
            continue;
        }
        units.entry(t.id).or_default().push(Measured {
            comment: format!("{} · {}", t.name, level_name(o.level)),
            units: 1.0,
            length: 0.0,
            width: 0.0,
            height: 0.0,
            quantity: 1.0,
        });
    }
    for (i, (t, lines)) in units.into_iter().enumerate() {
        let Some(ty) = model.object_type(t) else {
            continue;
        };
        items.push(Item {
            code: format!("K{:02}", i + 1),
            key: format!("mueble:{}", t.0),
            unit: "ud",
            summary: ty.name.clone(),
            lines,
        });
    }
    items
}

/// Pipes and ducts by system and size, in metres; devices by kind, by
/// units.
fn services(model: &Model) -> Vec<Item> {
    let level_name = |l: LevelId| model.level(l).map_or(String::new(), |l| l.name.clone());
    let new = |id: u64| model.phase(id) == Phase::New;
    let mut runs: BTreeMap<(archisim_core::ServiceSystem, String), Vec<Measured>> = BTreeMap::new();
    for r in model.runs().filter(|r| new(r.id.0)) {
        let len = r.true_length();
        runs.entry((r.system, r.size_text()))
            .or_default()
            .push(Measured {
                comment: format!(
                    "{} {} · {}",
                    if r.is_duct() { "Conducto" } else { "Tubería" },
                    r.id.0,
                    level_name(r.level)
                ),
                units: 1.0,
                length: len,
                width: 0.0,
                height: 0.0,
                quantity: len,
            });
    }
    let mut items: Vec<Item> = runs
        .into_iter()
        .enumerate()
        .map(|(i, ((system, size), lines))| Item {
            code: format!("T{:02}", i + 1),
            key: format!("red:{}:{size}", system.name()),
            unit: "m",
            summary: format!("{} {size}", system.name()),
            lines,
        })
        .collect();
    // Elbows and tees, by units: they follow from the paths.
    let mut fittings: BTreeMap<
        (
            archisim_core::ServiceSystem,
            archisim_core::FittingKind,
            String,
        ),
        Vec<Measured>,
    > = BTreeMap::new();
    for f in model.run_fittings() {
        let Some(r) = model.run(f.run).filter(|r| new(r.id.0)) else {
            continue;
        };
        fittings
            .entry((r.system, f.kind, r.size_text()))
            .or_default()
            .push(Measured {
                comment: format!(
                    "{} de la {} {} · {}",
                    f.kind.name(),
                    if r.is_duct() {
                        "red de conductos"
                    } else {
                        "tubería"
                    },
                    r.id.0,
                    level_name(r.level)
                ),
                units: 1.0,
                length: 0.0,
                width: 0.0,
                height: 0.0,
                quantity: 1.0,
            });
    }
    items.extend(
        fittings
            .into_iter()
            .enumerate()
            .map(|(i, ((system, kind, size), lines))| Item {
                code: format!("P{:02}", i + 1),
                key: format!("pieza:{}:{}:{size}", system.name(), kind.name()),
                unit: "ud",
                summary: format!("{} {} {size}", kind.name(), system.name()),
                lines,
            }),
    );
    let mut devices: BTreeMap<(archisim_core::DeviceKind, String), Vec<Measured>> = BTreeMap::new();
    for d in model.devices().filter(|d| new(d.id.0)) {
        devices
            .entry((d.kind, d.name.clone()))
            .or_default()
            .push(Measured {
                comment: format!("{} {} · {}", d.name, d.id.0, level_name(d.level)),
                units: 1.0,
                length: 0.0,
                width: d.width,
                height: d.height,
                quantity: 1.0,
            });
    }
    items.extend(
        devices
            .into_iter()
            .enumerate()
            .map(|(i, ((kind, name), lines))| Item {
                code: format!("A{:02}", i + 1),
                key: format!("aparato:{kind:?}:{name}"),
                unit: "ud",
                summary: name,
                lines,
            }),
    );
    items
}

/// A type mark usable as a BC3 code (letters, digits, `.`, `-`, `_`).
fn code_of(mark: &str) -> Option<String> {
    let m = mark.trim();
    (!m.is_empty()
        && m.len() <= 18
        && m.chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '_')))
    .then(|| m.to_string())
}

/// A schedule of the model (of one level, or of all).
pub fn table(model: &Model, kind: ScheduleKind, level: Option<LevelId>) -> Table {
    let on = |l: Option<LevelId>| level.is_none() || l == level;
    let level_name = |l: LevelId| model.level(l).map_or(String::new(), |l| l.name.clone());
    let scope = level
        .map(|l| format!(" · {}", level_name(l)))
        .unwrap_or_default();
    match kind {
        ScheduleKind::Openings => {
            let mut rows = Vec::new();
            let mut keys = Vec::new();
            let mut total = 0;
            for t in model.opening_types() {
                if t.kind == OpeningKind::Void {
                    continue;
                }
                let mut levels: Vec<String> = Vec::new();
                let mut n = 0;
                for o in model.openings().filter(|o| o.opening_type == t.id) {
                    let l = model.wall(o.wall).map(|w| w.level);
                    if !on(l) {
                        continue;
                    }
                    n += 1;
                    if let Some(l) = l {
                        let name = level_name(l);
                        if !levels.contains(&name) {
                            levels.push(name);
                        }
                    }
                }
                if n == 0 {
                    continue;
                }
                total += n;
                keys.push(t.id.0);
                rows.push(vec![
                    t.mark.clone(),
                    t.name.clone(),
                    if t.is_door() { "Puerta" } else { "Ventana" }.into(),
                    dec(t.width, 2),
                    dec(t.height, 2),
                    n.to_string(),
                    levels.join(", "),
                ]);
            }
            let t = Table {
                title: format!("CARPINTERÍAS{scope}"),
                columns: vec![
                    col("Marca", false),
                    col("Tipo", false),
                    col("Clase", false),
                    col("Ancho (m)", true),
                    col("Alto (m)", true),
                    col("Uds.", true),
                    col("Plantas", false),
                ],
                rows,
                totals: Some(vec![
                    "Total".into(),
                    String::new(),
                    String::new(),
                    String::new(),
                    String::new(),
                    total.to_string(),
                    String::new(),
                ]),
            };
            let t = with_parameters(
                t,
                model,
                &keys,
                &[
                    archisim_core::ParamCategory::Doors,
                    archisim_core::ParamCategory::Windows,
                ],
                true,
            );
            with_shared_family_values(t, model, &keys)
        }
        ScheduleKind::Walls => {
            let mut rows = Vec::new();
            let mut keys = Vec::new();
            let (mut tl, mut ta, mut tv) = (0.0, 0.0, 0.0);
            for t in model.wall_types() {
                let (mut n, mut l, mut a, mut v) = (0, 0.0, 0.0, 0.0);
                for w in model
                    .walls()
                    .filter(|w| w.wall_type == t.id && on(Some(w.level)))
                {
                    let cut: f64 = model
                        .openings_in_wall(w.id)
                        .filter_map(|o| model.opening_type(o.opening_type))
                        .map(|ot| ot.width * ot.height)
                        .sum();
                    let area = w.length() * w.height - cut;
                    n += 1;
                    l += w.length();
                    a += area;
                    v += area * t.thickness();
                }
                if n == 0 {
                    continue;
                }
                tl += l;
                ta += a;
                tv += v;
                keys.push(t.id.0);
                rows.push(vec![
                    t.name.clone(),
                    dec(t.thickness() * 100.0, 1),
                    n.to_string(),
                    dec(l, 2),
                    dec(a, 2),
                    dec(v, 2),
                ]);
            }
            let t = Table {
                title: format!("MUROS{scope}"),
                columns: vec![
                    col("Tipo", false),
                    col("Espesor (cm)", true),
                    col("Nº", true),
                    col("Longitud (m)", true),
                    col("Superficie (m²)", true),
                    col("Volumen (m³)", true),
                ],
                rows,
                totals: Some(vec![
                    "Total".into(),
                    String::new(),
                    String::new(),
                    dec(tl, 2),
                    dec(ta, 2),
                    dec(tv, 2),
                ]),
            };
            with_parameters(
                t,
                model,
                &keys,
                &[archisim_core::ParamCategory::Walls],
                true,
            )
        }
        ScheduleKind::Slabs => {
            let mut rows = Vec::new();
            let mut keys = Vec::new();
            let (mut ta, mut tv) = (0.0, 0.0);
            for t in model.slab_types() {
                let (mut n, mut a) = (0, 0.0);
                for s in model
                    .slabs()
                    .filter(|s| s.slab_type == t.id && on(Some(s.level)))
                {
                    n += 1;
                    a += s.area();
                }
                if n == 0 {
                    continue;
                }
                let v = a * t.thickness();
                ta += a;
                tv += v;
                keys.push(t.id.0);
                rows.push(vec![
                    t.name.clone(),
                    dec(t.thickness() * 100.0, 1),
                    n.to_string(),
                    dec(a, 2),
                    dec(v, 2),
                ]);
            }
            let t = Table {
                title: format!("FORJADOS{scope}"),
                columns: vec![
                    col("Tipo", false),
                    col("Espesor (cm)", true),
                    col("Nº", true),
                    col("Superficie (m²)", true),
                    col("Volumen (m³)", true),
                ],
                rows,
                totals: Some(vec![
                    "Total".into(),
                    String::new(),
                    String::new(),
                    dec(ta, 2),
                    dec(tv, 2),
                ]),
            };
            with_parameters(
                t,
                model,
                &keys,
                &[archisim_core::ParamCategory::Slabs],
                true,
            )
        }
        ScheduleKind::Rooms => {
            let mut rooms: Vec<&archisim_core::Room> =
                model.rooms().filter(|r| on(Some(r.level))).collect();
            let elev = |l: LevelId| model.level(l).map_or(0.0, |l| l.elevation);
            rooms.sort_by(|a, b| {
                elev(a.level)
                    .total_cmp(&elev(b.level))
                    .then_with(|| natural(&a.number).cmp(&natural(&b.number)))
            });
            let total: f64 = rooms.iter().map(|r| r.area()).sum();
            let built = |r: &archisim_core::Room| {
                // To the axes of its walls, for rooms bounded by them.
                if !r.bounded {
                    return None;
                }
                archisim_geom::room_outline(
                    model,
                    r.level,
                    r.label,
                    archisim_geom::Contour::WallAxis,
                )
                .ok()
                .map(|o| archisim_core::signed_area(&o).abs())
            };
            let built_areas: Vec<Option<f64>> = rooms.iter().map(|r| built(r)).collect();
            let built_total: f64 = built_areas.iter().flatten().sum();
            let rows = rooms
                .iter()
                .zip(&built_areas)
                .map(|(r, b)| {
                    let h = model.room_height(r);
                    vec![
                        r.number.clone(),
                        r.name.clone(),
                        r.usage.clone(),
                        level_name(r.level),
                        dec(r.area(), 2),
                        b.map(|b| dec(b, 2)).unwrap_or_default(),
                        dec(r.perimeter(), 2),
                        dec(h, 2),
                        dec(r.area() * h, 2),
                        r.finishes.floor.clone(),
                        r.finishes.walls.clone(),
                        r.finishes.ceiling.clone(),
                    ]
                })
                .collect();
            let keys: Vec<u64> = rooms.iter().map(|r| r.id.0).collect();
            let t = Table {
                title: format!("CUADRO DE SUPERFICIES{scope}"),
                columns: vec![
                    col("Nº", false),
                    col("Local", false),
                    col("Uso", false),
                    col("Planta", false),
                    col("Sup. útil (m²)", true),
                    col("Sup. constr. (m²)", true),
                    col("Perímetro (m)", true),
                    col("Altura (m)", true),
                    col("Volumen (m³)", true),
                    col("Suelo", false),
                    col("Paredes", false),
                    col("Techo", false),
                ],
                rows,
                totals: Some(vec![
                    "Total".into(),
                    String::new(),
                    String::new(),
                    String::new(),
                    dec(total, 2),
                    if built_total > 0.0 {
                        dec(built_total, 2)
                    } else {
                        String::new()
                    },
                    String::new(),
                    String::new(),
                    String::new(),
                    String::new(),
                    String::new(),
                    String::new(),
                ]),
            };
            with_parameters(
                t,
                model,
                &keys,
                &[archisim_core::ParamCategory::Rooms],
                false,
            )
        }
        ScheduleKind::RoomAreas => {
            let mut rooms: Vec<&archisim_core::Room> =
                model.rooms().filter(|r| on(Some(r.level))).collect();
            let elev = |l: LevelId| model.level(l).map_or(0.0, |l| l.elevation);
            rooms.sort_by(|a, b| {
                elev(a.level)
                    .total_cmp(&elev(b.level))
                    .then_with(|| natural(&a.number).cmp(&natural(&b.number)))
            });
            let total: f64 = rooms.iter().map(|r| r.area()).sum();
            Table {
                title: format!("SUPERFICIES ÚTILES{scope}"),
                columns: vec![col("Nº", false), col("Local", false), col("m²", true)],
                rows: rooms
                    .iter()
                    .map(|r| vec![r.number.clone(), r.name.clone(), dec(r.area(), 2)])
                    .collect(),
                totals: Some(vec!["Total".into(), String::new(), dec(total, 2)]),
            }
        }
        ScheduleKind::Structure => {
            let mut rows = Vec::new();
            let (mut tl, mut tv) = (0.0, 0.0);
            for t in model.member_types() {
                let a = t.profile.area();
                let (mut n, mut l) = (0, 0.0);
                for c in model
                    .columns()
                    .filter(|c| c.member_type == t.id && on(Some(c.level)))
                {
                    n += 1;
                    l += c.height;
                }
                for b in model
                    .beams()
                    .filter(|b| b.member_type == t.id && on(Some(b.level)))
                {
                    n += 1;
                    l += b.length();
                }
                if n == 0 {
                    continue;
                }
                tl += l;
                tv += l * a;
                let usage = match t.usage {
                    archisim_core::MemberUsage::Column => "Pilar",
                    archisim_core::MemberUsage::Beam => "Viga",
                };
                let material = model
                    .material(t.material)
                    .map_or(String::new(), |m| m.name.clone());
                rows.push(vec![
                    t.name.clone(),
                    usage.into(),
                    material,
                    n.to_string(),
                    dec(l, 2),
                    dec(l * a, 3),
                ]);
            }
            Table {
                title: format!("PILARES Y VIGAS{scope}"),
                columns: vec![
                    col("Tipo", false),
                    col("Uso", false),
                    col("Material", false),
                    col("Nº", true),
                    col("Longitud (m)", true),
                    col("Volumen (m³)", true),
                ],
                rows,
                totals: Some(vec![
                    "Total".into(),
                    String::new(),
                    String::new(),
                    String::new(),
                    dec(tl, 2),
                    dec(tv, 3),
                ]),
            }
        }
        ScheduleKind::Roofs => {
            let mut rows = Vec::new();
            let (mut tp, mut ts) = (0.0, 0.0);
            for r in model.roofs().filter(|r| on(Some(r.level))) {
                let plan = archisim_core::signed_area(&r.eaves()).abs();
                let surface = r.surface_area();
                let slopes: Vec<String> = r
                    .slopes
                    .iter()
                    .flatten()
                    .map(|p| format!("{} %", dec(p * 100.0, 0)))
                    .collect::<std::collections::BTreeSet<_>>()
                    .into_iter()
                    .collect();
                tp += plan;
                ts += surface;
                rows.push(vec![
                    model
                        .slab_type(r.roof_type)
                        .map_or(String::new(), |t| t.name.clone()),
                    level_name(r.level),
                    slopes.join(", "),
                    dec(r.overhang, 2),
                    dec(plan, 2),
                    dec(surface, 2),
                ]);
            }
            Table {
                title: format!("CUBIERTAS{scope}"),
                columns: vec![
                    col("Tipo", false),
                    col("Planta", false),
                    col("Pendientes", false),
                    col("Alero (m)", true),
                    col("En planta (m²)", true),
                    col("Inclinada (m²)", true),
                ],
                rows,
                totals: Some(vec![
                    "Total".into(),
                    String::new(),
                    String::new(),
                    String::new(),
                    dec(tp, 2),
                    dec(ts, 2),
                ]),
            }
        }
        ScheduleKind::Stairs => {
            let rows = model
                .stairs()
                .filter(|s| on(Some(s.level)))
                .map(|s| {
                    let l = s.layout();
                    let issues = s.issues();
                    vec![
                        format!("{} · {}", s.id.0, level_name(s.level)),
                        match s.usage {
                            archisim_core::StairUse::General => "General".into(),
                            archisim_core::StairUse::Restricted => "Restringido".into(),
                        },
                        l.risers().to_string(),
                        dec(l.riser * 1000.0, 0),
                        dec(s.going * 1000.0, 0),
                        dec(s.width, 2),
                        if issues.is_empty() {
                            "Cumple".into()
                        } else {
                            format!("{} incumplimientos", issues.len())
                        },
                    ]
                })
                .collect();
            Table {
                title: format!("ESCALERAS{scope}"),
                columns: vec![
                    col("Escalera", false),
                    col("Uso (CTE)", false),
                    col("Tabicas", true),
                    col("Tabica (mm)", true),
                    col("Huella (mm)", true),
                    col("Ancho (m)", true),
                    col("DB-SUA", false),
                ],
                rows,
                totals: None,
            }
        }
        ScheduleKind::CurtainWalls => {
            let mut rows = Vec::new();
            let (mut tl, mut ta) = (0.0, 0.0);
            for t in model.curtain_types() {
                let (mut n, mut l, mut a) = (0, 0.0, 0.0);
                for c in model
                    .curtain_walls()
                    .filter(|c| c.curtain_type == t.id && on(Some(c.level)))
                {
                    let len = c.start.distance(c.end);
                    n += 1;
                    l += len;
                    a += len * c.height;
                }
                if n == 0 {
                    continue;
                }
                tl += l;
                ta += a;
                rows.push(vec![t.name.clone(), n.to_string(), dec(l, 2), dec(a, 2)]);
            }
            Table {
                title: format!("MUROS CORTINA{scope}"),
                columns: vec![
                    col("Tipo", false),
                    col("Nº", true),
                    col("Longitud (m)", true),
                    col("Superficie (m²)", true),
                ],
                rows,
                totals: Some(vec!["Total".into(), String::new(), dec(tl, 2), dec(ta, 2)]),
            }
        }
    }
}

/// "0.10" after "0.9": digits compared as numbers.
fn natural(s: &str) -> Vec<(String, u64)> {
    let mut out = Vec::new();
    let mut text = String::new();
    let mut digits = String::new();
    for c in s.chars() {
        if c.is_ascii_digit() {
            digits.push(c);
        } else {
            if !digits.is_empty() {
                out.push((std::mem::take(&mut text), digits.parse().unwrap_or(0)));
                digits.clear();
            }
            text.push(c);
        }
    }
    out.push((text, digits.parse().unwrap_or(0)));
    out
}

impl Table {
    /// As CSV for a spreadsheet: semicolons and decimal commas, as Spanish
    /// spreadsheets read it.
    pub fn csv(&self) -> String {
        let cell = |s: &str| {
            if s.contains([';', '"', '\n']) {
                format!("\"{}\"", s.replace('"', "\"\""))
            } else {
                s.to_string()
            }
        };
        let mut out = String::new();
        let line = |out: &mut String, cells: Vec<String>| {
            let v: Vec<String> = cells.iter().map(|c| cell(c)).collect();
            out.push_str(&v.join(";"));
            out.push_str("\r\n");
        };
        line(&mut out, vec![self.title.clone()]);
        line(
            &mut out,
            self.columns.iter().map(|c| c.name.to_string()).collect(),
        );
        for r in &self.rows {
            line(&mut out, r.clone());
        }
        if let Some(t) = &self.totals {
            line(&mut out, t.clone());
        }
        out
    }

    /// Width of each column and height of a row, paper millimetres.
    pub fn layout(&self) -> (Vec<f64>, f64) {
        let pad = 2.0;
        let widths = (0..self.columns.len())
            .map(|i| {
                let head = text_width(&self.columns[i].name, HEAD);
                let cells = self
                    .rows
                    .iter()
                    .chain(self.totals.iter())
                    .filter_map(|r| r.get(i))
                    .map(|c| text_width(c, BODY))
                    .fold(0.0, f64::max);
                head.max(cells) + 2.0 * pad
            })
            .collect();
        (widths, ROW)
    }

    /// Size on paper, millimetres.
    pub fn size(&self) -> (f64, f64) {
        let (w, h) = self.layout();
        let rows = self.rows.len() + 2 + usize::from(self.totals.is_some());
        (w.iter().sum(), h * rows as f64)
    }
}

const HEAD: f64 = 2.0;
const BODY: f64 = 2.0;
const ROW: f64 = 6.0;

/// Draws a schedule on a sheet with its top-left corner at `at` (paper
/// metres).
pub fn draw_table(s: &mut String, t: &Table, at: Point2) {
    let k = 0.001;
    let (widths, row) = t.layout();
    let total_w: f64 = widths.iter().sum();
    let n_rows = t.rows.len() + 2 + usize::from(t.totals.is_some());
    let p = |x: f64, y: f64| Point2::new(at.x + x * k, at.y - y * k);
    let ink = "#1a1a1a";
    let mut d = String::new();
    let mut thick = String::new();
    // Outline and the line under the headings, thicker.
    let _ = write!(
        thick,
        "M{} L{} L{} L{} Z M{} L{} ",
        pt(p(0.0, 0.0)),
        pt(p(total_w, 0.0)),
        pt(p(total_w, row * n_rows as f64)),
        pt(p(0.0, row * n_rows as f64)),
        pt(p(0.0, 2.0 * row)),
        pt(p(total_w, 2.0 * row)),
    );
    for i in 3..n_rows {
        let y = row * i as f64;
        let _ = write!(d, "M{} L{} ", pt(p(0.0, y)), pt(p(total_w, y)));
    }
    let _ = write!(d, "M{} L{} ", pt(p(0.0, row)), pt(p(total_w, row)));
    let mut x = 0.0;
    for w in &widths[..widths.len().saturating_sub(1)] {
        x += w;
        let _ = write!(d, "M{} L{} ", pt(p(x, row)), pt(p(x, row * n_rows as f64)));
    }
    let _ = writeln!(
        s,
        r#"<path d="{}" stroke="{ink}" stroke-width="{}" fill="none"/>"#,
        d.trim_end(),
        num(0.13 * k)
    );
    let _ = writeln!(
        s,
        r#"<path d="{}" stroke="{ink}" stroke-width="{}" fill="none"/>"#,
        thick.trim_end(),
        num(0.35 * k)
    );
    let mut texts: Vec<Text> = Vec::new();
    texts.push(Text {
        at: p(total_w / 2.0, row / 2.0),
        content: t.title.clone(),
        height: 2.5,
        angle: 0.0,
        align: TextAlign::Middle,
    });
    let cell = |texts: &mut Vec<Text>, i: usize, r: usize, content: &str, h: f64| {
        let x0: f64 = widths[..i].iter().sum();
        let y = row * r as f64 + row / 2.0;
        let numeric = t.columns[i].numeric && r >= 2;
        let (x, align) = if numeric {
            (x0 + widths[i] - 2.0, TextAlign::MiddleRight)
        } else {
            (x0 + 2.0, TextAlign::MiddleLeft)
        };
        if !content.is_empty() {
            texts.push(Text {
                at: p(x, y),
                content: content.to_string(),
                height: h,
                angle: 0.0,
                align,
            });
        }
    };
    for (i, c) in t.columns.iter().enumerate() {
        cell(&mut texts, i, 1, &c.name, HEAD);
    }
    for (r, rowv) in t.rows.iter().chain(t.totals.iter()).enumerate() {
        for (i, v) in rowv.iter().enumerate().take(t.columns.len()) {
            cell(&mut texts, i, r + 2, v, BODY);
        }
    }
    for tx in &texts {
        let _ = writeln!(
            s,
            r#"<path d="{}" {}/>"#,
            crate::lettering::text_d(tx, k),
            crate::lettering::paint(tx.height * k, ink)
        );
    }
}

/// What goes outside the windows: grilles by their weight (kg) and
/// shutters by the opening they close (m²), one item per type.
fn outsides(model: &Model) -> Vec<Item> {
    use archisim_geom::solid::{grille_weight, opening_outside_solids, Outside};
    let level_name = |l: LevelId| model.level(l).map_or(String::new(), |l| l.name.clone());
    let mut grilles: BTreeMap<archisim_core::OpeningTypeId, Vec<Measured>> = BTreeMap::new();
    let mut shutters: BTreeMap<archisim_core::OpeningTypeId, Vec<Measured>> = BTreeMap::new();
    for o in model.openings() {
        if model.phase(o.id.0) != Phase::New {
            continue;
        }
        let Some(t) = model.opening_type(o.opening_type) else {
            continue;
        };
        let level = model
            .wall(o.wall)
            .map_or(String::new(), |w| level_name(w.level));
        let comment = format!("{} · {}", t.mark, level);
        let kg = grille_weight(model, o.id);
        if kg > 0.0 {
            grilles.entry(t.id).or_default().push(Measured {
                comment: comment.clone(),
                units: 1.0,
                length: 0.0,
                width: 0.0,
                height: 0.0,
                quantity: kg,
            });
        }
        if !opening_outside_solids(model, o.id, Outside::Shutters).is_empty() {
            shutters.entry(t.id).or_default().push(Measured {
                comment,
                units: 1.0,
                length: t.width,
                width: 0.0,
                height: t.height,
                quantity: t.width * t.height,
            });
        }
    }
    let item =
        |what: &str, unit: &'static str, key: &str, id: archisim_core::OpeningTypeId, lines| {
            let t = model.opening_type(id);
            Item {
                code: t.map_or(String::new(), |t| {
                    format!("{}-{}", t.mark, key[..1].to_uppercase())
                }),
                key: format!("{key}:{}", id.0),
                unit,
                summary: format!("{what} · {}", t.map_or("", |t| t.name.as_str())),
                lines,
            }
        };
    grilles
        .into_iter()
        .map(|(id, lines)| item("Reja", "kg", "reja", id, lines))
        .chain(
            shutters
                .into_iter()
                .map(|(id, lines)| item("Contraventanas", "m2", "contraventana", id, lines)),
        )
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn numbers_in_order_and_csv() {
        let mut v = vec!["0.10", "0.9", "0.02"];
        v.sort_by_key(|s| natural(s));
        assert_eq!(v, ["0.02", "0.9", "0.10"]);
        let t = Table {
            title: "T".into(),
            columns: vec![col("A", false), col("B", true)],
            rows: vec![vec!["x;y".into(), "1,50".into()]],
            totals: None,
        };
        assert_eq!(t.csv(), "T\r\nA;B\r\n\"x;y\";1,50\r\n");
        let (w, h) = t.size();
        assert!(w > 10.0 && (h - 18.0).abs() < 1e-9);
    }
}
