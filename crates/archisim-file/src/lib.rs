//! ArchiSim project files (`.archisim`).
//!
//! A project is stored as plain JSON so that it can be read, compared and
//! versioned with ordinary tools:
//!
//! ```json
//! {
//!   "format": "ArchiSim",
//!   "version": 9,
//!   "units": "m",
//!   "generator": "ArchiSim 0.4.0",
//!   "project": { "name": "…", "guid": "…", … },
//!   "levels": [ … ],
//!   "materials": [ … ],
//!   …
//! }
//! ```
//!
//! - **Versioned.** `version` is the layout of the file. Opening a file from
//!   a newer ArchiSim is refused with a clear message; files from older
//!   versions are upgraded on opening (see [`migrate`]).
//! - **Stable identities.** Every element keeps its internal id and its IFC
//!   `GlobalId`, so saving, opening and exporting again gives the same IFC.
//! - **Checked on opening.** The model is rebuilt with every rule the editor
//!   applies ([`archisim_core::Model::from_parts`]); a broken or hand-edited
//!   file is rejected with the element and the reason, never half-loaded.
//! - **Safe saving.** The file is written next to the target and then renamed
//!   over it, so a crash while saving never leaves a truncated project.
//! - **Readable and diff-friendly.** Two-space indentation, one element per
//!   block, points as `[x, y]` on one line, elements ordered by id. Saving
//!   the same model twice gives byte-identical files. The undo history is
//!   not saved.

mod drafting;
mod rooms;
mod schema;

use std::fmt;
use std::io::Write as _;
use std::path::Path;

use archisim_core::{Model, RestoreError};
use serde::Deserialize;
use serde_json::Value;

/// Value of the `format` field.
pub const FORMAT: &str = "ArchiSim";
/// File layout written by this version of ArchiSim.
pub const CURRENT_VERSION: u32 = 9;
/// File name extension, without the dot.
pub const EXTENSION: &str = "archisim";
/// Length unit of every number in the file.
pub const UNITS: &str = "m";

/// Why a project could not be saved or opened.
#[derive(Debug)]
pub enum FileError {
    /// The file could not be read or written.
    Io(std::io::Error),
    /// The text is not valid JSON, or a field is missing, unknown or of the
    /// wrong kind. The message gives the line and column.
    Json(serde_json::Error),
    /// The file is JSON but not an ArchiSim project.
    NotAProject,
    /// The file was written by a newer ArchiSim.
    TooNew { version: u32 },
    /// The `version` field is not one this program knows.
    UnknownVersion(u64),
    /// The file uses a length unit other than metres.
    Units(String),
    /// A field has a value of the right kind but not a valid one.
    Value { element: String, message: String },
    /// The elements are inconsistent (missing references, overlapping
    /// openings, invalid outlines…).
    Model(RestoreError),
}

impl fmt::Display for FileError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FileError::Io(e) => write!(f, "{e}"),
            FileError::Json(e) => write!(f, "{e}"),
            FileError::NotAProject => write!(
                f,
                "not an ArchiSim project (the \"format\" field must be \"{FORMAT}\")"
            ),
            FileError::TooNew { version } => write!(
                f,
                "the project was saved with a newer ArchiSim (file version {version}; \
                 this program reads up to version {CURRENT_VERSION}). Update ArchiSim to open it"
            ),
            FileError::UnknownVersion(v) => write!(f, "unknown file version {v}"),
            FileError::Units(u) => {
                write!(f, "unsupported units «{u}»: only \"{UNITS}\" is supported")
            }
            FileError::Value { element, message } => write!(f, "{element}: {message}"),
            FileError::Model(e) => write!(f, "{e}"),
        }
    }
}

impl std::error::Error for FileError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            FileError::Io(e) => Some(e),
            FileError::Json(e) => Some(e),
            FileError::Model(e) => Some(e),
            _ => None,
        }
    }
}

impl From<std::io::Error> for FileError {
    fn from(e: std::io::Error) -> Self {
        FileError::Io(e)
    }
}

/// The project as `.archisim` text.
pub fn to_string(model: &Model) -> String {
    let file = schema::ProjectFile::from_model(model);
    let value = serde_json::to_value(&file).expect("the file schema always serialises");
    let mut out = String::new();
    write_pretty(&value, 0, &mut out);
    out.push('\n');
    out
}

/// Reads a project from `.archisim` text.
pub fn from_str(text: &str) -> Result<Model, FileError> {
    // Editors on Windows may add a byte order mark.
    let text = text.strip_prefix('\u{feff}').unwrap_or(text);

    #[derive(Deserialize)]
    struct Header {
        format: Option<Value>,
        version: Option<Value>,
    }
    let header: Header = serde_json::from_str(text).map_err(FileError::Json)?;
    if header.format.as_ref().and_then(Value::as_str) != Some(FORMAT) {
        return Err(FileError::NotAProject);
    }
    let version = header
        .version
        .as_ref()
        .and_then(Value::as_u64)
        .ok_or(FileError::NotAProject)?;
    if version > u64::from(CURRENT_VERSION) {
        return Err(FileError::TooNew {
            version: u32::try_from(version).unwrap_or(u32::MAX),
        });
    }
    if version == 0 {
        return Err(FileError::UnknownVersion(version));
    }

    let file: schema::ProjectFile = if version == u64::from(CURRENT_VERSION) {
        // Parse the text directly so that errors point at a line and column.
        serde_json::from_str(text).map_err(FileError::Json)?
    } else {
        let value = serde_json::from_str(text).map_err(FileError::Json)?;
        serde_json::from_value(migrate(value, version)?).map_err(FileError::Json)?
    };
    file.into_model()
}

/// Upgrades an older file, step by step, to [`CURRENT_VERSION`].
///
/// Each future format change adds one step here (`1 → 2`, `2 → 3`…) that
/// rewrites the JSON, so every old project keeps opening.
pub fn migrate(value: Value, from: u64) -> Result<Value, FileError> {
    let mut value = value;
    let mut version = from;
    while version < u64::from(CURRENT_VERSION) {
        value = match version {
            // 1 → 2: plan dimensions and level marks.
            1 => {
                let mut v = value;
                if let Value::Object(map) = &mut v {
                    map.insert("version".into(), Value::from(2));
                    map.entry("dimensions")
                        .or_insert_with(|| Value::Array(Vec::new()));
                    map.entry("elevation_marks")
                        .or_insert_with(|| Value::Array(Vec::new()));
                }
                v
            }
            // 2 → 3: door and window types become types of families.
            2 => to_families(value)?,
            // 3 → 4: walls may be tied to a top level (`top`); the walls
            // of older files keep their own height, which is what a wall
            // without `top` means.
            3 => {
                let mut v = value;
                if let Value::Object(map) = &mut v {
                    map.insert("version".into(), Value::from(4));
                }
                v
            }
            // 4 → 5: walls may be curved (`bulge`); a wall without it is
            // straight, as all walls of older files are. 5 → 6: slabs may
            // have holes (`holes`) and walls voids (the "Hueco" family);
            // older files have neither.
            4 | 5 => {
                let mut v = value;
                if let Value::Object(map) = &mut v {
                    map.insert("version".into(), Value::from(version + 1));
                }
                v
            }
            // 6 → 7: phase 2 elements (columns, beams, sections, roofs,
            // stairs…) and phase 3 drawing (layers, drafting, sheets,
            // rooms, tags); all absent in older files, but drawing layers
            // and drafting elements are always written.
            6 => {
                let mut v = value;
                if let Value::Object(map) = &mut v {
                    map.insert("version".into(), Value::from(7));
                }
                with_drafting(v)
            }
            // 7 → 8: curtain walls take their grid, frames and panels from
            // a type. Version 7 was written by two branches of ArchiSim
            // 0.25 (phase 2 and phase 3); both go through here.
            7 => with_drafting(to_curtain_types(value)),
            // 8 → 9: phase 2 rooms (`spaces`) and phase 3 rooms (`rooms`)
            // become one kind of room.
            8 => spaces_to_rooms(with_drafting(value))?,
            other => return Err(FileError::UnknownVersion(other)),
        };
        version += 1;
    }
    Ok(value)
}

/// Drawing layers and drafting elements, written by every file since
/// version 7; the phase 2 files of version 7 and 8 have neither.
fn with_drafting(mut value: Value) -> Value {
    if let Value::Object(map) = &mut value {
        map.entry("draw_layers")
            .or_insert_with(|| Value::Array(Vec::new()));
        map.entry("drafts")
            .or_insert_with(|| Value::Array(Vec::new()));
    }
    value
}

/// Version 8 → 9. The rooms of phase 2 (`spaces`: number, name, use,
/// outline, height and an optional label point) become rooms with their
/// own height, not tied to walls, their name where it was (or in the
/// middle); the project's list of uses (`space_usages`) becomes
/// `room_usages`.
fn spaces_to_rooms(mut value: Value) -> Result<Value, FileError> {
    let Value::Object(map) = &mut value else {
        return Ok(value);
    };
    map.insert("version".into(), Value::from(9));
    if let Some(Value::Object(project)) = map.get_mut("project") {
        if let Some(u) = project.remove("space_usages") {
            project.insert("room_usages".into(), u);
        }
    }
    let Some(spaces) = map.remove("spaces") else {
        return Ok(value);
    };
    let bad = |message: &str| FileError::Value {
        element: "room (version 8)".into(),
        message: message.into(),
    };
    let Value::Array(spaces) = spaces else {
        return Err(bad("«spaces» is not a list"));
    };
    let mut rooms = match map.remove("rooms") {
        Some(Value::Array(r)) => r,
        _ => Vec::new(),
    };
    for mut sp in spaces {
        let Value::Object(o) = &mut sp else {
            return Err(bad("not an object"));
        };
        let outline: Vec<archisim_core::Point2> = o
            .get("outline")
            .and_then(Value::as_array)
            .ok_or_else(|| bad("no outline"))?
            .iter()
            .map(|p| {
                let x = p.get(0).and_then(Value::as_f64);
                let y = p.get(1).and_then(Value::as_f64);
                match (x, y) {
                    (Some(x), Some(y)) => Ok(archisim_core::Point2::new(x, y)),
                    _ => Err(bad("outline point is not [x, y]")),
                }
            })
            .collect::<Result<_, _>>()?;
        let label = match o.remove("label") {
            Some(l @ Value::Array(_)) => l,
            _ => {
                let c = archisim_core::rooms::label_center(&outline);
                serde_json::json!([c.x, c.y])
            }
        };
        o.insert("label".into(), label);
        o.entry("number").or_insert_with(|| Value::from(""));
        rooms.push(sp);
    }
    map.insert("rooms".into(), Value::Array(rooms));
    Ok(value)
}

/// Version 7 → 8. Version 7 curtain walls carried their own grid
/// (largest bay and row), frame (width, depth, material) and glass
/// (thickness, material); each different set becomes a curtain wall type
/// ("Muro cortina 1200 × 1500") with that frame all round and one panel,
/// numbered after the highest id in the file.
fn to_curtain_types(mut value: Value) -> Value {
    let Value::Object(map) = &mut value else {
        return value;
    };
    map.insert("version".into(), Value::from(8));
    let mut next_id = 0;
    for items in map.values() {
        if let Value::Array(items) = items {
            for id in items.iter().filter_map(|i| i.get("id")?.as_u64()) {
                next_id = next_id.max(id);
            }
        }
    }
    let mut types: Vec<Value> = Vec::new();
    let mut seen: Vec<(Value, u64)> = Vec::new();
    if let Some(Value::Array(walls)) = map.get_mut("curtain_walls") {
        for w in walls.iter_mut() {
            let Value::Object(obj) = w else {
                continue;
            };
            let mut take = |k: &str| obj.remove(k).unwrap_or(Value::Null);
            let (bay, row) = (take("bay"), take("row"));
            let (width, depth, thickness) = (
                take("frame_width"),
                take("frame_depth"),
                take("panel_thickness"),
            );
            let (frame, glass) = (take("frame_material"), take("panel_material"));
            let profile = serde_json::json!({
                "width": width, "depth": depth, "material": frame,
            });
            let design = serde_json::json!({
                "columns": { "rest": { "max_spacing": bay } },
                "rows": { "rest": { "max_spacing": row } },
                "border": profile,
                "mullion": profile,
                "transom": profile,
                "panels": [{ "name": "Vidrio", "thickness": thickness, "material": glass }],
            });
            let id = match seen.iter().find(|(d, _)| *d == design) {
                Some((_, id)) => *id,
                None => {
                    next_id += 1;
                    let mm = |v: &Value| v.as_f64().map_or(0.0, |v| (v * 1000.0).round());
                    let mut t = design.clone();
                    if let Value::Object(t) = &mut t {
                        t.insert("id".into(), Value::from(next_id));
                        t.insert(
                            "guid".into(),
                            Value::from(archisim_core::GlobalId::new().to_string()),
                        );
                        t.insert(
                            "name".into(),
                            Value::from(format!("Muro cortina {} × {}", mm(&bay), mm(&row))),
                        );
                    }
                    types.push(t);
                    seen.push((design, next_id));
                    next_id
                }
            };
            obj.insert("type".into(), Value::from(id));
        }
    }
    if !types.is_empty() {
        map.insert("curtain_types".into(), Value::Array(types));
    }
    value
}

/// Version 2 → 3. Each door or window type, described by its category,
/// operation and sizes, becomes a type of the matching system family
/// ("Puerta abatible", "Ventana corredera"…), whose recipe is added to the
/// file. Types get marks in file order (P-01, P-02… and V-01…). The
/// families are numbered after the highest id in the file.
fn to_families(mut value: Value) -> Result<Value, FileError> {
    use archisim_core::family::{legacy_family, operation_from_name, system_family};
    use archisim_core::OpeningKind;

    let Value::Object(map) = &mut value else {
        return Ok(value);
    };
    map.insert("version".into(), Value::from(3));
    let mut next_id = 0;
    for key in [
        "levels",
        "materials",
        "wall_types",
        "walls",
        "slab_types",
        "slabs",
        "opening_types",
        "openings",
        "dimensions",
        "elevation_marks",
    ] {
        if let Some(Value::Array(items)) = map.get(key) {
            for id in items.iter().filter_map(|i| i.get("id")?.as_u64()) {
                next_id = next_id.max(id);
            }
        }
    }
    let mut families: Vec<Value> = Vec::new();
    let mut by_name: Vec<(&str, u64, archisim_core::Values)> = Vec::new();
    let (mut doors, mut windows) = (0, 0);
    if let Some(Value::Array(types)) = map.get_mut("opening_types") {
        for t in types.iter_mut() {
            let Value::Object(obj) = t else {
                continue;
            };
            let id = obj.get("id").cloned().unwrap_or(Value::Null);
            let text = |v: Option<Value>| v.and_then(|v| v.as_str().map(str::to_string));
            let category = text(obj.remove("category")).unwrap_or_default();
            let operation = text(obj.remove("operation")).unwrap_or_default();
            let kind =
                operation_from_name(&category, &operation).ok_or_else(|| FileError::Value {
                    element: format!("door or window type {id}"),
                    message: format!("a {category} cannot have the operation «{operation}»"),
                })?;
            let (name, extras) = legacy_family(kind);
            let family = match by_name.iter().find(|(n, _, _)| *n == name) {
                Some((_, fid, _)) => *fid,
                None => {
                    next_id += 1;
                    let source = system_family(name).expect("system family");
                    let parsed =
                        archisim_core::Family::new(archisim_core::FamilyId(next_id), source)
                            .expect("system families parse");
                    families.push(serde_json::json!({
                        "id": next_id,
                        "name": parsed.name(),
                        "version": parsed.version(),
                        "source": parsed.source.lines().collect::<Vec<_>>(),
                    }));
                    let defaults = parsed.def().defaults(archisim_core::ParamScope::Type);
                    by_name.push((name, next_id, defaults));
                    next_id
                }
            };
            // The family's defaults (for its other parameters, such as how
            // the leaves it does not use open), then the type's own values.
            let defaults = &by_name
                .iter()
                .find(|(n, _, _)| *n == name)
                .expect("added")
                .2;
            let mut values = serde_json::Map::new();
            for (k, v) in defaults {
                values.insert(k.clone(), json_value(v));
            }
            for key in [
                "width",
                "height",
                "frame_width",
                "frame_depth",
                "frame_material",
                "panel_material",
            ] {
                if let Some(v) = obj.remove(key) {
                    values.insert(key.into(), v);
                }
            }
            for (k, v) in extras {
                values.insert(k.into(), json_value(&v));
            }
            let mark = match kind {
                OpeningKind::Door(_) => {
                    doors += 1;
                    format!("P-{doors:02}")
                }
                OpeningKind::Window(_) => {
                    windows += 1;
                    format!("V-{windows:02}")
                }
                // Version 2 files had no voids.
                OpeningKind::Void => format!("H-{:02}", doors + windows + 1),
            };
            obj.insert("mark".into(), Value::from(mark));
            obj.insert("description".into(), Value::from(""));
            obj.insert("family".into(), Value::from(family));
            obj.insert("values".into(), Value::Object(values));
        }
    }
    map.insert("families".into(), Value::Array(families));
    Ok(value)
}

/// A parameter value as written in the file.
fn json_value(v: &archisim_core::Value) -> Value {
    match v {
        archisim_core::Value::Number(n) => Value::from(*n),
        archisim_core::Value::Bool(b) => Value::from(*b),
        archisim_core::Value::Text(t) => Value::from(t.clone()),
        archisim_core::Value::Material(m) => Value::from(*m),
    }
}

/// Saves the project. The previous file, if any, is replaced only once the
/// new one is completely written.
pub fn save(model: &Model, path: impl AsRef<Path>) -> Result<(), FileError> {
    let path = path.as_ref();
    let mut tmp_name = path
        .file_name()
        .ok_or_else(|| {
            FileError::Io(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "the path has no file name",
            ))
        })?
        .to_os_string();
    tmp_name.push(".saving");
    let tmp = path.with_file_name(tmp_name);

    let result = (|| {
        let mut f = std::fs::File::create(&tmp)?;
        f.write_all(to_string(model).as_bytes())?;
        f.sync_all()?;
        drop(f);
        std::fs::rename(&tmp, path)
    })();
    if result.is_err() {
        let _ = std::fs::remove_file(&tmp);
    }
    result.map_err(FileError::Io)
}

/// Opens a project file.
pub fn open(path: impl AsRef<Path>) -> Result<Model, FileError> {
    from_str(&std::fs::read_to_string(path)?)
}

/// Writes JSON with two-space indentation, keeping arrays of plain numbers
/// (points) on one line.
fn write_pretty(value: &Value, indent: usize, out: &mut String) {
    let pad = |n: usize| "  ".repeat(n);
    match value {
        Value::Array(items) if items.is_empty() => out.push_str("[]"),
        Value::Array(items) if items.iter().all(Value::is_number) => {
            out.push('[');
            for (i, v) in items.iter().enumerate() {
                if i > 0 {
                    out.push_str(", ");
                }
                out.push_str(&v.to_string());
            }
            out.push(']');
        }
        Value::Array(items) => {
            out.push_str("[\n");
            for (i, v) in items.iter().enumerate() {
                out.push_str(&pad(indent + 1));
                write_pretty(v, indent + 1, out);
                out.push_str(if i + 1 < items.len() { ",\n" } else { "\n" });
            }
            out.push_str(&pad(indent));
            out.push(']');
        }
        Value::Object(map) if map.is_empty() => out.push_str("{}"),
        Value::Object(map) => {
            out.push_str("{\n");
            for (i, (k, v)) in map.iter().enumerate() {
                out.push_str(&pad(indent + 1));
                out.push_str(&Value::String(k.clone()).to_string());
                out.push_str(": ");
                write_pretty(v, indent + 1, out);
                out.push_str(if i + 1 < map.len() { ",\n" } else { "\n" });
            }
            out.push_str(&pad(indent));
            out.push('}');
        }
        scalar => out.push_str(&scalar.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use archisim_core::*;
    use serde_json::Value;

    /// A room with a door and a window, a slab and every kind of value the
    /// file has to carry (negative offsets, `null` setback, both sides…).
    fn sample() -> Model {
        let mut m = Model::new("Casa «Prueba» \"1\"");
        let pb = m.add_level("Planta baja", 0.0);
        let cubierta = m.add_level("Cubierta", 3.25);
        let brick = m.add_material("Ladrillo", "brick", [232, 196, 178], CutPattern::Diagonal);
        let wool = m.add_material(
            "Lana",
            "insulation",
            [250, 243, 205],
            CutPattern::Insulation,
        );
        let wood = m.add_material("Madera", "wood", [180, 140, 90], CutPattern::Solid);
        let glass = m.add_material("Vidrio", "glass", [200, 230, 240], CutPattern::Empty);
        let layer = |material, thickness, function| Layer {
            material,
            thickness,
            function,
        };
        let wt = m
            .add_wall_type(
                "Fachada",
                WallFunction::Exterior,
                vec![
                    layer(brick, 0.115, LayerFunction::Structure),
                    layer(wool, 0.05, LayerFunction::Insulation),
                    layer(brick, 0.07, LayerFunction::Substrate),
                ],
            )
            .unwrap();
        let wall = |start, end| NewWall {
            wall_type: wt,
            level: pb,
            start,
            end,
            reference: ReferenceLine::ExteriorFace,
            height: 3.0,
            base_offset: 0.0,
            bulge: 0.0,
            top: archisim_core::WallTop::Unconnected,
        };
        let w = m
            .add_wall(wall(Point2::new(0.0, 0.0), Point2::new(0.0, 5.0)))
            .unwrap();
        let mut second = wall(Point2::new(0.0, 5.0), Point2::new(4.2, 5.0));
        second.reference = ReferenceLine::Center;
        second.base_offset = -0.1;
        // Tied to the roof level, 25 cm under it (version 4).
        second.top = archisim_core::WallTop::Level {
            level: cubierta,
            offset: -0.25,
        };
        m.add_wall(second).unwrap();
        let st = m
            .add_slab_type(
                "Solera",
                SlabFunction::BaseSlab,
                vec![layer(brick, 0.15, LayerFunction::Structure)],
            )
            .unwrap();
        m.add_slab(NewSlab {
            slab_type: st,
            level: pb,
            outline: vec![
                Point2::new(0.0, 0.0),
                Point2::new(4.2, 0.0),
                Point2::new(4.2, 5.0),
                Point2::new(0.0, 5.0),
            ],
            top_offset: -0.1,
        })
        .unwrap();
        let door = m
            .add_opening_type(NewOpeningType {
                name: "P 80".into(),
                kind: OpeningKind::Door(DoorOperation::SingleSwing),
                width: 0.8,
                height: 2.05,
                frame_width: 0.05,
                frame_depth: 0.07,
                frame_material: wood,
                panel_material: wood,
            })
            .unwrap();
        let window = m
            .add_opening_type(NewOpeningType {
                name: "V corredera".into(),
                kind: OpeningKind::Window(WindowOperation::Sliding),
                width: 1.2,
                height: 1.1,
                frame_width: 0.06,
                frame_depth: 0.07,
                frame_material: wood,
                panel_material: glass,
            })
            .unwrap();
        m.add_opening(NewOpening {
            opening_type: door,
            wall: w,
            along: 1.0,
            sill: 0.0,
            opens_to: WallSide::Interior,
            hand: Hand::Right,
            setback: None,
        })
        .unwrap();
        m.add_opening(NewOpening {
            opening_type: window,
            wall: w,
            along: 3.3,
            sill: 0.95,
            opens_to: WallSide::Exterior,
            hand: Hand::Left,
            setback: Some(0.13),
        })
        .unwrap();
        m.add_dimension(archisim_core::NewDimension {
            level: pb,
            points: vec![Point2::new(0.0, 0.0), Point2::new(0.0, 5.0)],
            direction: Point2::new(0.0, 1.0),
            position: Point2::new(-0.8, 0.0),
        })
        .unwrap();
        m.add_elevation_mark(pb, Point2::new(2.0, 2.5)).unwrap();
        m
    }

    /// Columns, beams and rooms (version 7) are saved and come back identical.
    #[test]
    fn columns_and_beams_round_trip() {
        use archisim_core::{MemberUsage, NewBeam, NewColumn, Profile, WallTop};
        let mut m = sample();
        let levels: Vec<_> = m.levels().iter().map(|l| l.id).collect();
        let concrete = m.add_material("HA-25", "concrete", [190, 190, 190], CutPattern::Cross);
        let col = m
            .add_member_type(
                "Pilar 30×30",
                MemberUsage::Column,
                concrete,
                Profile::Rectangle {
                    width: 0.3,
                    depth: 0.3,
                },
            )
            .unwrap();
        let beam = m
            .add_member_type(
                "IPE 300",
                MemberUsage::Beam,
                concrete,
                Profile::IShape {
                    width: 0.15,
                    depth: 0.3,
                    web: 0.0071,
                    flange: 0.0107,
                },
            )
            .unwrap();
        m.add_column(NewColumn {
            member_type: col,
            level: levels[0],
            position: Point2::new(4.2, 0.0),
            rotation: 0.25,
            base_offset: 0.0,
            top: WallTop::Level {
                level: levels[1],
                offset: -0.3,
            },
            height: 0.0,
        })
        .unwrap();
        m.add_beam(NewBeam {
            member_type: beam,
            level: levels[1],
            start: Point2::new(0.0, 0.0),
            end: Point2::new(4.2, 0.0),
            top_offset: -0.05,
        })
        .unwrap();
        let room = m
            .add_room(archisim_core::rooms::NewRoom {
                level: levels[0],
                number: "0.01".into(),
                name: "Salón «comedor»".into(),
                outline: vec![
                    Point2::new(0.3, 0.3),
                    Point2::new(4.0, 0.3),
                    Point2::new(4.0, 4.7),
                    Point2::new(0.3, 4.7),
                ],
                holes: Vec::new(),
                label: Point2::new(1.0, 1.0),
                bounded: false,
            })
            .unwrap();
        let mut r = m.room(room).unwrap().clone();
        r.usage = "Estar".into();
        r.height = Some(2.6);
        m.set_room(r).unwrap();
        // Concrete goes through anything (material intersection priority).
        m.set_material_priority(concrete, Some(990)).unwrap();
        // An office project: its own room uses.
        m.set_room_usages(vec!["Despacho".into(), "Sala de reuniones".into()])
            .unwrap();
        // A refurbishment: the first wall existing, the column demolished.
        let first = m.walls().next().unwrap().id.0;
        m.set_phase(first, archisim_core::Phase::Existing).unwrap();
        let col_id = m.columns().next().unwrap().id.0;
        m.set_phase(col_id, archisim_core::Phase::Demolished)
            .unwrap();
        m.add_section(archisim_core::NewSection {
            name: "A".into(),
            kind: archisim_core::SectionKind::Section,
            start: Point2::new(-1.0, 2.0),
            end: Point2::new(5.0, 2.0),
            depth: 12.0,
        })
        .unwrap();
        // A U stair up to the next level.
        let concrete = m.materials().next().unwrap().id;
        m.add_stair(archisim_core::NewStair {
            level: levels[0],
            path: vec![
                Point2::new(0.0, 0.0),
                Point2::new(2.5, 0.0),
                Point2::new(2.5, 1.1),
                Point2::new(0.0, 1.1),
            ],
            width: 1.0,
            going: 0.28,
            base_offset: 0.0,
            top: archisim_core::WallTop::Level {
                level: levels[1],
                offset: 0.0,
            },
            height: 0.0,
            waist: 0.15,
            material: concrete,
            usage: archisim_core::StairUse::Restricted,
        })
        .unwrap();
        // A small terrain and the true north.
        let earth = m.materials().last().unwrap().id;
        let sp = |x: f64, y: f64, z: f64| archisim_core::SurveyPoint {
            at: Point2::new(x, y),
            z,
        };
        m.add_terrain(archisim_core::NewTerrain {
            level: levels[0],
            points: vec![
                sp(-5.0, -5.0, -0.5),
                sp(15.0, -5.0, -0.2),
                sp(15.0, 15.0, 0.4),
                sp(-5.0, 15.0, 0.1),
            ],
            material: earth,
            depth: 1.5,
            contour: 0.25,
            lines: vec![vec![0, 1]],
        })
        .unwrap();
        m.set_true_north(0.4).unwrap();
        // A roof with a gable, and an inclined slab.
        let st = m.slab_types().next().unwrap().id;
        m.add_roof(archisim_core::NewRoof {
            roof_type: st,
            level: levels[1],
            outline: vec![
                Point2::new(0.0, 0.0),
                Point2::new(5.0, 0.0),
                Point2::new(5.0, 4.0),
                Point2::new(0.0, 4.0),
            ],
            slopes: vec![Some(0.3), None, Some(0.35), Some(0.3)],
            base_offset: 0.1,
            overhang: 0.4,
        })
        .unwrap();
        let slab = m.slabs().next().unwrap().id;
        m.set_slab_slope(
            slab,
            Some(archisim_core::SlabSlope {
                from: Point2::new(0.0, 0.0),
                to: Point2::new(0.0, 1.0),
                slope: -0.02,
            }),
        )
        .unwrap();
        let text = to_string(&m);
        assert!(text.contains("\"shape\": \"i_shape\""), "{text}");
        assert!(
            text.contains("\"slopes\": [") && text.contains("null"),
            "{text}"
        );
        let back = from_str(&text).unwrap();
        assert_eq!(back, m);
        // A version 6 file has none of them and still opens.
        let v6 = to_string(&sample()).replacen("\"version\": 9", "\"version\": 6", 1);
        assert_eq!(from_str(&v6).unwrap().columns().count(), 0);
    }

    #[test]
    fn version_1_files_still_open() {
        // A project saved by ArchiSim 0.8 (file version 1): no dimensions
        // nor level marks.
        let mut m = sample();
        let ids: Vec<_> = m.dimensions().map(|d| d.id).collect();
        for d in ids {
            m.remove_dimension(d).unwrap();
        }
        let ids: Vec<_> = m.elevation_marks().map(|e| e.id).collect();
        for e in ids {
            m.remove_elevation_mark(e).unwrap();
        }
        let mut v = as_version_2(&m);
        let map = v.as_object_mut().unwrap();
        map.remove("dimensions");
        map.remove("elevation_marks");
        map.insert("version".into(), Value::from(1));
        let old = serde_json::to_string_pretty(&v).unwrap();
        assert!(old.contains("\"version\": 1"));
        let opened = from_str(&old).unwrap();
        assert_same_project(&opened, &m);
        // Saved again, it is a version 5 file.
        assert!(to_string(&opened).contains("\"version\": 9"));
    }

    /// The project as ArchiSim 0.10–0.13 wrote it (file version 2): door and
    /// window types by category, operation and sizes, no families.
    fn as_version_2(m: &Model) -> Value {
        let mut v: Value = serde_json::from_str(&to_string(m)).unwrap();
        let map = v.as_object_mut().unwrap();
        map.insert("version".into(), Value::from(2));
        map.remove("families");
        for t in map["opening_types"].as_array_mut().unwrap() {
            let obj = t.as_object_mut().unwrap();
            let id = obj["id"].as_u64().unwrap();
            let ty = m.opening_type(OpeningTypeId(id)).unwrap();
            for k in ["mark", "description", "family", "values"] {
                obj.remove(k);
            }
            let (category, operation) = match ty.kind {
                OpeningKind::Void => unreachable!("version 2 had no voids"),
                OpeningKind::Door(DoorOperation::SingleSwing) => ("door", "single_swing"),
                OpeningKind::Door(DoorOperation::DoubleSwing) => ("door", "double_swing"),
                OpeningKind::Door(DoorOperation::Sliding) => ("door", "sliding"),
                OpeningKind::Door(DoorOperation::Folding) => ("door", "folding"),
                OpeningKind::Window(WindowOperation::Fixed) => ("window", "fixed"),
                OpeningKind::Window(WindowOperation::SingleCasement) => {
                    ("window", "single_casement")
                }
                OpeningKind::Window(WindowOperation::DoubleCasement) => {
                    ("window", "double_casement")
                }
                OpeningKind::Window(WindowOperation::Sliding) => ("window", "sliding"),
            };
            obj.insert("category".into(), Value::from(category));
            obj.insert("operation".into(), Value::from(operation));
            for (k, x) in [
                ("width", ty.width),
                ("height", ty.height),
                ("frame_width", ty.frame_width),
                ("frame_depth", ty.frame_depth),
            ] {
                obj.insert(k.into(), Value::from(x));
            }
            obj.insert("frame_material".into(), Value::from(ty.frame_material.0));
            obj.insert("panel_material".into(), Value::from(ty.panel_material.0));
        }
        for o in map["openings"].as_array_mut().unwrap() {
            o.as_object_mut().unwrap().remove("values");
        }
        v
    }

    /// Same elements, ids, GUIDs and resolved door and window types; the
    /// families may have other ids (they did not exist before version 3).
    fn assert_same_project(opened: &Model, m: &Model) {
        let (mut a, mut b) = (opened.to_parts(), m.to_parts());
        assert_eq!(a.families.len(), b.families.len());
        for p in [&mut a, &mut b] {
            p.families.clear();
            for t in &mut p.opening_types {
                t.family = FamilyId(0);
            }
        }
        assert_eq!(a, b);
    }

    #[test]
    fn version_2_types_become_types_of_the_system_families() {
        let m = sample();
        let old = serde_json::to_string_pretty(&as_version_2(&m)).unwrap();
        let opened = from_str(&old).unwrap();
        assert_same_project(&opened, &m);
        for t in opened.opening_types() {
            let fam = opened.family(t.family).unwrap();
            assert_eq!(fam.name(), archisim_core::family::legacy_family(t.kind).0);
        }
        // The families are numbered after every other element.
        let max = m
            .to_parts()
            .openings
            .iter()
            .map(|o| o.id.0)
            .chain(m.to_parts().dimensions.iter().map(|d| d.id.0))
            .max()
            .unwrap();
        assert!(opened.families().all(|f| f.id.0 > max));
        // An impossible operation is still explained.
        let broken = old.replacen("\"operation\": \"sliding\"", "\"operation\": \"fixed\"", 1);
        let broken = broken.replacen("\"category\": \"window\"", "\"category\": \"door\"", 1);
        let err = from_str(&broken).unwrap_err();
        assert!(matches!(err, FileError::Value { .. }), "{err}");
    }

    /// The vivienda de prueba as saved by ArchiSim 0.13 (file version 2)
    /// opens with the same doors and windows, now as family types.
    #[test]
    fn the_demo_saved_by_version_0_13_opens_the_same() {
        let text = include_str!("../tests/fixtures/demo_v2.archisim");
        let json: Value = serde_json::from_str(text).unwrap();
        assert_eq!(json["version"], 2);
        let m = from_str(text).unwrap();
        let types = json["opening_types"].as_array().unwrap();
        assert_eq!(m.opening_types().count(), types.len());
        for t in types {
            let ty = m
                .opening_type(OpeningTypeId(t["id"].as_u64().unwrap()))
                .unwrap();
            assert_eq!(ty.guid.as_str(), t["guid"].as_str().unwrap());
            assert_eq!(ty.name, t["name"].as_str().unwrap());
            for (k, x) in [
                ("width", ty.width),
                ("height", ty.height),
                ("frame_width", ty.frame_width),
                ("frame_depth", ty.frame_depth),
            ] {
                assert_eq!(x, t[k].as_f64().unwrap(), "{k}");
            }
            assert_eq!(ty.frame_material.0, t["frame_material"].as_u64().unwrap());
            assert_eq!(ty.panel_material.0, t["panel_material"].as_u64().unwrap());
            assert_eq!(ty.is_door(), t["category"] == "door");
        }
        let marks: Vec<_> = m.opening_types().map(|t| t.mark.as_str()).collect();
        assert_eq!(
            marks,
            ["P-01", "P-02", "P-03", "P-04", "V-01", "V-02", "V-03", "V-04"]
        );
        assert_eq!(
            m.openings().count(),
            json["openings"].as_array().unwrap().len()
        );
        // Saved as version 3 and opened again: the same model.
        let again = from_str(&to_string(&m)).unwrap();
        assert_eq!(again, m);
    }

    #[test]
    fn round_trip_is_exact_and_stable() {
        let m = sample();
        let text = to_string(&m);
        let back = from_str(&text).unwrap();
        assert_eq!(back, m);
        // Saving the reopened model gives the very same bytes.
        assert_eq!(to_string(&back), text);
        // Values that are not short decimals survive too.
        let mut m2 = m.clone();
        let third = m2.add_level("Rara", 0.1 + 0.2);
        let back = from_str(&to_string(&m2)).unwrap();
        assert_eq!(back.level(third).unwrap().elevation, 0.1 + 0.2);
    }

    /// Version 3 files (ArchiSim 0.14 to 0.16) have no `top`: their walls
    /// open with their own height; a tied wall saved by version 4 follows
    /// its level when the file says it moved.
    #[test]
    fn walls_tied_to_a_level_and_older_files() {
        let m = sample();
        let text = to_string(&m);
        let tied = m
            .walls()
            .find(|w| w.top != archisim_core::WallTop::Unconnected)
            .unwrap();
        assert!((tied.height - (3.25 - 0.25 + 0.1)).abs() < 1e-12);
        // The roof level moved by hand in the file: the wall follows.
        let moved = from_str(&text.replacen("3.25", "3.5", 1)).unwrap();
        assert!((moved.wall(tied.id).unwrap().height - 3.35).abs() < 1e-12);
        // An older file: no `top`, version 3.
        let start = text.find("      \"top\": {").unwrap();
        let end = start + text[start..].find("}\n").unwrap() + 2;
        let old = format!("{}{}", &text[..start], &text[end..])
            .replacen("\"version\": 9", "\"version\": 3", 1)
            .replace("\"base_offset\": -0.1,\n", "\"base_offset\": -0.1\n");
        let opened = from_str(&old).unwrap();
        let w = opened.wall(tied.id).unwrap();
        assert_eq!(w.top, archisim_core::WallTop::Unconnected);
        assert_eq!(w.height, tied.height);
    }

    /// Slab holes and voids (version 6) are saved and come back; version 5
    /// files, without either, open as they are.
    #[test]
    fn slab_holes_voids_and_version_5_files() {
        let mut m = sample();
        let slab = m.slabs().next().unwrap().clone();
        let lo = slab
            .outline
            .iter()
            .fold(Point2::new(f64::INFINITY, f64::INFINITY), |a, p| {
                Point2::new(a.x.min(p.x), a.y.min(p.y))
            });
        let hole = vec![
            lo + Point2::new(0.5, 0.5),
            lo + Point2::new(1.5, 0.5),
            lo + Point2::new(1.5, 1.5),
            lo + Point2::new(0.5, 1.5),
        ];
        m.add_slab_hole(slab.id, hole).unwrap();
        let free = m
            .walls()
            .find(|w| m.openings_in_wall(w.id).next().is_none() && w.length() > 1.0)
            .unwrap();
        let (wall, mid) = (free.id, free.length() / 2.0);
        let void = m
            .add_opening_type(archisim_core::NewOpeningType {
                name: "Hueco 60×60".into(),
                kind: archisim_core::OpeningKind::Void,
                width: 0.6,
                height: 0.6,
                frame_width: 0.0,
                frame_depth: 0.0,
                frame_material: slab_material(&m),
                panel_material: slab_material(&m),
            })
            .unwrap();
        m.add_opening(archisim_core::NewOpening {
            opening_type: void,
            wall,
            along: mid,
            sill: 1.0,
            opens_to: archisim_core::WallSide::Interior,
            hand: archisim_core::Hand::Left,
            setback: None,
        })
        .unwrap();
        let text = to_string(&m);
        assert!(text.contains("\"holes\""));
        let back = from_str(&text).unwrap();
        assert_same_project(&back, &m);
        assert_eq!(back.slab(slab.id).unwrap().holes.len(), 1);
        assert!(back.opening_type(void).unwrap().is_void());
        assert_eq!(back.opening_type(void).unwrap().mark, "H-01");
        // Version 5 (ArchiSim 0.19): no holes, no voids.
        let v5 = to_string(&sample()).replacen("\"version\": 9", "\"version\": 5", 1);
        assert!(from_str(&v5).unwrap().slabs().all(|s| s.holes.is_empty()));
    }

    #[test]
    fn curtain_wall_types_and_version_7_files() {
        let mut m = sample();
        let level = m.levels()[0].id;
        let glass = m.add_material("Vidrio", "glass", [190, 222, 236], CutPattern::Solid);
        let al = m.add_material("Aluminio", "aluminium", [120, 124, 128], CutPattern::Solid);
        let frame = archisim_core::CurtainProfile {
            width: 0.05,
            depth: 0.12,
            material: al,
        };
        let mut design = archisim_core::CurtainDesign::simple(1.2, 1.5, frame, 0.024, glass);
        design.rows.fixed = vec![0.9];
        design.caps = Some(archisim_core::CurtainCaps {
            width: 0.06,
            depth: 0.02,
            material: al,
        });
        let t = m.add_curtain_type("Con tapajuntas", design).unwrap();
        for x in [0.0, 6.0] {
            m.add_curtain_wall(archisim_core::NewCurtainWall {
                curtain_type: t,
                level,
                start: Point2::new(x, -3.0),
                end: Point2::new(x + 4.8, -3.0),
                base_offset: 0.0,
                top: archisim_core::WallTop::Unconnected,
                height: 2.7,
            })
            .unwrap();
        }
        let text = to_string(&m);
        assert!(text.contains("\"curtain_types\""));
        assert!(text.contains("\"max_spacing\": 1.2"));
        let back = from_str(&text).unwrap();
        assert_same_project(&back, &m);
        assert_eq!(back.curtain_types().count(), 1);

        // Version 7 (ArchiSim 0.25): each curtain wall with its own grid;
        // the same grid gives one type.
        let mut v: Value = serde_json::from_str(&text).unwrap();
        v["version"] = Value::from(7);
        v.as_object_mut().unwrap().remove("curtain_types");
        for w in v["curtain_walls"].as_array_mut().unwrap() {
            let w = w.as_object_mut().unwrap();
            w.remove("type");
            for (k, x) in [
                ("bay", 1.0),
                ("row", 1.5),
                ("frame_width", 0.05),
                ("frame_depth", 0.12),
                ("panel_thickness", 0.024),
            ] {
                w.insert(k.into(), Value::from(x));
            }
            w.insert("frame_material".into(), Value::from(al.0));
            w.insert("panel_material".into(), Value::from(glass.0));
        }
        let old = from_str(&serde_json::to_string(&v).unwrap()).unwrap();
        assert_eq!(old.curtain_types().count(), 1);
        let t = old.curtain_types().next().unwrap();
        assert_eq!(t.name, "Muro cortina 1000 × 1500");
        assert_eq!(
            t.design.columns,
            archisim_core::CurtainDivision::max_spacing(1.0)
        );
        assert_eq!(t.design.border.material, al);
        assert_eq!(t.design.panels[0].material, glass);
        assert!(old.curtain_walls().all(|c| c.curtain_type == t.id));
        // Five bays of 0,96 on 4,80 m, as before.
        let c = old.curtain_walls().next().unwrap();
        assert_eq!(c.grid(&t.design).mullions.len(), 6);
    }

    /// Phase 2 files (version 7 and 8 of ArchiSim 0.25) have rooms as
    /// `spaces`, the uses as `space_usages`, and no drawing layers: they
    /// open with those rooms as rooms of their own height.
    #[test]
    fn phase_2_rooms_and_version_8_files() {
        let m = sample();
        let level = m.levels()[0].id;
        let mut v: Value = serde_json::from_str(&to_string(&m)).unwrap();
        let map = v.as_object_mut().unwrap();
        map.insert("version".into(), Value::from(8));
        for k in ["draw_layers", "drafts", "rooms", "tags"] {
            map.remove(k);
        }
        map.get_mut("project")
            .unwrap()
            .as_object_mut()
            .unwrap()
            .insert(
                "space_usages".into(),
                serde_json::json!(["Cocina", "Taller"]),
            );
        let l_shape = [
            [0.0, 0.0],
            [4.0, 0.0],
            [4.0, 1.0],
            [1.0, 1.0],
            [1.0, 4.0],
            [0.0, 4.0],
        ];
        let g1 = archisim_core::GlobalId::new().to_string();
        let g2 = archisim_core::GlobalId::new().to_string();
        map.insert(
            "spaces".into(),
            serde_json::json!([
                {"id": 90001, "guid": g1, "level": level.0, "number": "0.01",
                 "name": "Cocina", "usage": "Cocina", "height": 2.6,
                 "outline": [[5.0, 0.0], [8.0, 0.0], [8.0, 3.0], [5.0, 3.0]],
                 "label": [6.0, 2.0]},
                {"id": 90002, "guid": g2, "level": level.0,
                 "name": "Pasillo", "height": 2.5, "outline": l_shape}
            ]),
        );
        let old = from_str(&serde_json::to_string(&v).unwrap()).unwrap();
        assert_eq!(old.room_usages(), ["Cocina", "Taller"]);
        let rooms: Vec<_> = old.rooms().collect();
        assert_eq!(rooms.len(), 2);
        let kitchen = rooms.iter().find(|r| r.name == "Cocina").unwrap();
        assert_eq!(kitchen.number, "0.01");
        assert_eq!(kitchen.usage, "Cocina");
        assert_eq!(kitchen.height, Some(2.6));
        assert_eq!(kitchen.label, Point2::new(6.0, 2.0));
        assert_eq!(kitchen.guid.to_string(), g1);
        assert!(!kitchen.bounded);
        assert!((kitchen.area() - 9.0).abs() < 1e-9);
        // Without a label point, the name goes inside the L.
        let hall = rooms.iter().find(|r| r.name == "Pasillo").unwrap();
        assert!(hall.contains(hall.label));
        // Saved again, it is a current file, and reads back the same.
        let text = to_string(&old);
        assert!(text.contains("\"room_usages\""));
        assert!(!text.contains("\"spaces\""));
        assert_same_project(&from_str(&text).unwrap(), &old);
    }

    fn slab_material(m: &Model) -> archisim_core::MaterialId {
        m.materials().next().unwrap().id
    }

    /// Curved walls (version 5) keep their bulge; straight walls write
    /// none, so version 4 files are read as they are.
    #[test]
    fn curved_walls_and_version_4_files() {
        let mut m = sample();
        let straight = m.walls().next().unwrap().clone();
        let curved = m
            .add_wall(NewWall {
                wall_type: straight.wall_type,
                level: straight.level,
                start: Point2::new(4.2, 0.0),
                end: Point2::new(4.2, 5.0),
                bulge: -0.4,
                reference: ReferenceLine::Center,
                height: 3.0,
                base_offset: 0.0,
                top: archisim_core::WallTop::Unconnected,
            })
            .unwrap();
        let text = to_string(&m);
        assert_eq!(text.matches("\"bulge\":").count(), 1);
        assert!(text.contains("\"bulge\": -0.4,"));
        let back = from_str(&text).unwrap();
        assert_same_project(&back, &m);
        assert_eq!(back.wall(curved).unwrap().bulge, -0.4);
        // Version 4 (ArchiSim 0.17–0.18): no bulge anywhere.
        let v4 = to_string(&sample()).replacen("\"version\": 9", "\"version\": 4", 1);
        let opened = from_str(&v4).unwrap();
        assert!(opened.walls().all(|w| w.bulge == 0.0));
        // A bend tighter than the wall is thick is refused on opening.
        let tight = text.replace("\"bulge\": -0.4,", "\"bulge\": 30.0,");
        let err = from_str(&tight).unwrap_err();
        assert!(err.to_string().contains("bends too much"), "{err}");
    }

    #[test]
    fn layout_is_readable() {
        let text = to_string(&sample());
        assert!(text.starts_with("{\n  \"format\": \"ArchiSim\",\n  \"version\": 9,\n"));
        // A wall tied to a level says which, and how far from it; a free
        // wall has no `top`.
        assert!(text.contains("\"top\": {\n        \"level\": "));
        assert!(text.contains("\"offset\": -0.25"));
        assert_eq!(text.matches("\"top\":").count(), 1);
        // Families are written one recipe line per item, readable.
        assert!(text.contains("      \"source\": [\n        \"family \\\"Puerta abatible\\\"\",\n"));
        assert!(text.contains("\"mark\": \"P-01\""));
        assert!(text.contains("\"leaves\": 1.0"));
        assert!(text.contains("\"units\": \"m\""));
        assert!(text.contains("\"start\": [0.0, 0.0]"));
        assert!(text.contains("\"cut_color\": \"#E8C4B2\""));
        assert!(text.contains("\"setback\": null"));
        assert!(text.contains("\"name\": \"Casa «Prueba» \\\"1\\\"\""));
        assert!(text.ends_with("}\n"));
    }

    fn edited(find: &str, replace: &str) -> Result<Model, FileError> {
        let text = to_string(&sample());
        assert!(text.contains(find), "sample has no «{find}»");
        from_str(&text.replacen(find, replace, 1))
    }

    #[test]
    fn hand_edits_are_applied() {
        let m = edited("\"height\": 3.0", "\"height\": 2.7").unwrap();
        assert!(m.walls().any(|w| w.height == 2.7));
        // A byte order mark from a Windows editor is accepted.
        let text = format!("\u{feff}{}", to_string(&sample()));
        assert!(from_str(&text).is_ok());
    }

    #[test]
    fn broken_files_are_explained() {
        // Not JSON: the error gives the line.
        let err = edited("\"units\": \"m\",", "\"units\": \"m\"").unwrap_err();
        assert!(matches!(err, FileError::Json(_)));
        assert!(err.to_string().contains("line 5"), "{err}");

        assert!(matches!(from_str("{}"), Err(FileError::NotAProject)));
        assert!(matches!(
            from_str(r#"{"format": "IFC", "version": 1}"#),
            Err(FileError::NotAProject)
        ));
        assert!(matches!(
            edited("\"version\": 9", "\"version\": 10"),
            Err(FileError::TooNew { version: 10 })
        ));
        assert!(matches!(
            edited("\"version\": 9", "\"version\": 0"),
            Err(FileError::UnknownVersion(0))
        ));
        assert!(matches!(
            edited("\"units\": \"m\"", "\"units\": \"mm\""),
            Err(FileError::Units(_))
        ));

        // A misspelt field is reported, not ignored.
        let err = edited("\"height\": 3.0", "\"heigth\": 3.0").unwrap_err();
        assert!(err.to_string().contains("heigth"), "{err}");

        let err = edited("\"reference\": \"center\"", "\"reference\": \"middle\"").unwrap_err();
        assert!(err.to_string().contains("middle"), "{err}");

        // A value outside the family's range, a broken recipe, a recipe
        // that does not match its name, a material that is not an id.
        let err = edited("\"leaves\": 1.0", "\"leaves\": 5.0").unwrap_err();
        match err {
            FileError::Model(e) => assert!(matches!(
                e.error,
                ModelError::Family(FamilyError::OutOfRange { .. })
            )),
            other => panic!("unexpected {other}"),
        }
        let err = edited("\"  category door\"", "\"  category\"").unwrap_err();
        assert!(matches!(err, FileError::Value { .. }));
        assert!(err.to_string().starts_with("family "), "{err}");
        let err = edited("\"name\": \"Puerta abatible\"", "\"name\": \"Puerta\"").unwrap_err();
        assert!(err.to_string().contains("must match"), "{err}");
        let err = edited(
            "\"frame_material\": ",
            "\"frame_material\": \"Madera\", \"x\": ",
        )
        .unwrap_err();
        assert!(err.to_string().contains("material id"), "{err}");

        let err = edited("\"cut_color\": \"#E8C4B2\"", "\"cut_color\": \"rojo\"").unwrap_err();
        assert!(matches!(err, FileError::Value { .. }));

        let text = to_string(&sample());
        let guid = sample_guid(&text);
        let err = from_str(&text.replacen(&guid, "not-a-guid", 1)).unwrap_err();
        assert!(err.to_string().contains("GlobalId"), "{err}");
    }

    /// The project GlobalId written in a file.
    fn sample_guid(text: &str) -> String {
        let start = text.find("\"guid\": \"").unwrap() + 9;
        text[start..start + 22].to_string()
    }

    #[test]
    fn inconsistent_models_are_rejected() {
        // The window moved by hand on top of the door.
        let err = edited("\"along\": 3.3", "\"along\": 1.5").unwrap_err();
        match err {
            FileError::Model(e) => assert_eq!(e.error, ModelError::OverlappingOpenings),
            other => panic!("unexpected {other}"),
        }
        // A level that does not exist.
        let err = edited("\"level\": 1,", "\"level\": 99,").unwrap_err();
        assert!(
            err.to_string().contains("level LevelId#99 does not exist"),
            "{err}"
        );
    }

    #[test]
    fn save_and_open_on_disk() {
        let dir = std::env::temp_dir().join(format!(
            "archisim-file-test-{}-{}",
            std::process::id(),
            GlobalId::new()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("casa.archisim");
        let m = sample();
        save(&m, &path).unwrap();
        // Saving again replaces the file.
        let mut m2 = m.clone();
        m2.project_name = "Casa revisada".into();
        save(&m2, &path).unwrap();
        assert_eq!(open(&path).unwrap(), m2);
        let names: Vec<_> = std::fs::read_dir(&dir)
            .unwrap()
            .map(|e| e.unwrap().file_name())
            .collect();
        assert_eq!(names, vec![std::ffi::OsString::from("casa.archisim")]);
        assert!(matches!(
            open(dir.join("no-existe.archisim")),
            Err(FileError::Io(_))
        ));
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
