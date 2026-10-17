//! ArchiSim project files (`.archisim`).
//!
//! A project is stored as plain JSON so that it can be read, compared and
//! versioned with ordinary tools:
//!
//! ```json
//! {
//!   "format": "ArchiSim",
//!   "version": 1,
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
pub const CURRENT_VERSION: u32 = 1;
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
    let file = schema::FileV1::from_model(model);
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

    let file: schema::FileV1 = if version == u64::from(CURRENT_VERSION) {
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
    match from {
        1 => Ok(value),
        other => Err(FileError::UnknownVersion(other)),
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

    /// A room with a door and a window, a slab and every kind of value the
    /// file has to carry (negative offsets, `null` setback, both sides…).
    fn sample() -> Model {
        let mut m = Model::new("Casa «Prueba» \"1\"");
        let pb = m.add_level("Planta baja", 0.0);
        m.add_level("Cubierta", 3.25);
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
        };
        let w = m
            .add_wall(wall(Point2::new(0.0, 0.0), Point2::new(0.0, 5.0)))
            .unwrap();
        let mut second = wall(Point2::new(0.0, 5.0), Point2::new(4.2, 5.0));
        second.reference = ReferenceLine::Center;
        second.base_offset = -0.1;
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
        m
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

    #[test]
    fn layout_is_readable() {
        let text = to_string(&sample());
        assert!(text.starts_with("{\n  \"format\": \"ArchiSim\",\n  \"version\": 1,\n"));
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
            edited("\"version\": 1", "\"version\": 7"),
            Err(FileError::TooNew { version: 7 })
        ));
        assert!(matches!(
            edited("\"version\": 1", "\"version\": 0"),
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

        let err = edited(
            "\"operation\": \"sliding\"",
            "\"operation\": \"single_swing\"",
        )
        .unwrap_err();
        assert!(matches!(err, FileError::Value { .. }));
        assert!(err.to_string().contains("window"), "{err}");

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
