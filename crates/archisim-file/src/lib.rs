//! ArchiSim project files (`.archisim`).
//!
//! A project is stored as plain JSON so that it can be read, compared and
//! versioned with ordinary tools:
//!
//! ```json
//! {
//!   "format": "ArchiSim",
//!   "version": 4,
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
pub const CURRENT_VERSION: u32 = 4;
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
            other => return Err(FileError::UnknownVersion(other)),
        };
        version += 1;
    }
    Ok(value)
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
        // Saved again, it is a version 4 file.
        assert!(to_string(&opened).contains("\"version\": 4"));
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
                OpeningKind::Door(DoorOperation::SingleSwing) => ("door", "single_swing"),
                OpeningKind::Door(DoorOperation::DoubleSwing) => ("door", "double_swing"),
                OpeningKind::Door(DoorOperation::Sliding) => ("door", "sliding"),
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
            .replacen("\"version\": 4", "\"version\": 3", 1)
            .replace("\"base_offset\": -0.1,\n", "\"base_offset\": -0.1\n");
        let opened = from_str(&old).unwrap();
        let w = opened.wall(tied.id).unwrap();
        assert_eq!(w.top, archisim_core::WallTop::Unconnected);
        assert_eq!(w.height, tied.height);
    }

    #[test]
    fn layout_is_readable() {
        let text = to_string(&sample());
        assert!(text.starts_with("{\n  \"format\": \"ArchiSim\",\n  \"version\": 4,\n"));
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
            edited("\"version\": 4", "\"version\": 7"),
            Err(FileError::TooNew { version: 7 })
        ));
        assert!(matches!(
            edited("\"version\": 4", "\"version\": 0"),
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
