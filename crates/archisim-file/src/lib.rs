//! ArchiSim project files (`.archisim`).
//!
//! A project is stored as plain JSON so that it can be read, compared and
//! versioned with ordinary tools:
//!
//! ```json
//! {
//!   "format": "ArchiSim",
//!   "version": 12,
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
mod meshes;
mod rooms;
mod schema;
mod views;

use std::fmt;
use std::io::Write as _;
use std::path::Path;

use archisim_core::{Model, RestoreError};
use serde::Deserialize;
use serde_json::Value;

/// Value of the `format` field.
pub const FORMAT: &str = "ArchiSim";
/// File layout written by this version of ArchiSim.
pub const CURRENT_VERSION: u32 = 63;
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

/// A JSON value written as ArchiSim writes its files.
pub(crate) fn pretty(value: &Value) -> String {
    let mut out = String::new();
    write_pretty(value, 0, &mut out);
    out.push('\n');
    out
}

/// A module file read from disk: its text as the project keeps it (written
/// again as ArchiSim writes its files), the fingerprint of the text on disk
/// and the model.
pub fn read_module(text: &str) -> Result<(String, String, Model), FileError> {
    let model = from_str(text)?;
    let plain = text.strip_prefix('\u{feff}').unwrap_or(text);
    let value: Value = serde_json::from_str(plain).map_err(FileError::Json)?;
    Ok((pretty(&value), archisim_core::text_stamp(text), model))
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
            // 9 → 10: views on sheets may show a section or elevation
            // (`section`); a view without it is a plan, as all of them were.
            // 10 → 11: the project may be georeferenced
            // (`project.georeference`); older ones are not.
            // 11 → 12: code settings (`project.code`) and rooms' area per
            // occupant (`occupancy`); older ones are dwellings, with the
            // occupancy of each room's use.
            // 12 → 13: parameters of the project (`user_parameters`,
            // `parameter_values`); older ones have none.
            // 13 → 14: views on sheets may be axonometric 3D views
            // (`axo`); older ones are plans and sections.
            // 14 → 15: elements may be given a discipline by hand
            // (`disciplines`); in older ones every element has its own.
            // 15 → 16: grid lines and footings (`grids`, `footings`);
            // older ones have none. 16 → 17: modules (`module_sources`,
            // `modules`); older ones have none. 17 → 18: concept volumes
            // (`masses`, `plot_area`); older ones have none. 18 → 19: plots
            // and roads (`plots`, `roads`); older ones have none. 19 → 20:
            // pipes, ducts and devices (`runs`, `devices`); older ones have
            // none. 20 → 21: loose edges of the concept phase (`edges`);
            // older ones have none. 21 → 22: presentation styles
            // (`presentation_styles`, a view's `presentation`) and rooms'
            // `floor_fill`; older ones have none. 22 → 23: objects' own
            // values (`values`) and hatches' `opacity`; older ones have
            // none (solid). 23 → 24: 3D models (`meshes`, an object type's
            // `mesh`); older ones have none. 24 → 25: plantings
            // (`plantings`); older ones have none. 25 → 26: round forms,
            // solids and edge finishes of volumes, and materials'
            // `appearances`; older ones have none. 26 → 27: saved plans,
            // levels' styles, view templates, reference planes, graphic
            // overrides of views and the graphics' `font`; older ones have
            // none (an older file's texts change to the sans lettering).
            // 27 → 28: volumes' `deform` and sweeps' `profile`; older ones
            // have none. 28 → 29: wall types' `sweeps` and slab types'
            // `edge`; older ones have none. 29 → 30: object types'
            // `symbol`; older ones have none. 30 → 31: prices of the
            // budget's items (`prices`); older ones have none. 31 → 32:
            // roofs' `dormers` and `covering`; older ones have none.
            // 32 → 33: stairs' `design` (winders, spirals, stringers…);
            // older ones are of flights and landings on a slab. ArchiSim
            // 0.48 wrote its own version 32, with dormers placed by the
            // middle of their front (`position`): those are set apart and
            // placed again on opening. 33 → 34: the slope of runs, the
            // decomposition of prices and the budget's overheads, profit
            // and VAT (`budget`); older ones have none. 34 → 35: object
            // types of one object (`own`) and their own drawings for
            // elevations (`elevation`); older ones have none. 35 → 36:
            // the presentation textures (wood, metal, concrete, stone) as
            // hatch patterns; nothing changes in older files. 36 → 37: the
            // areas painted over terrains (`patches`); older ones have
            // none. 37 → 38: split landings and the landing at the head of
            // a spiral (`split`, `arrival`); older stairs have neither.
            // 38 → 39: grass and flowers on materials (`growth`), and the
            // soil and leaves textures; older files have none. 39 → 40:
            // walls whose ends may not join (`allow_join`); in older files
            // every end joins. 40 → 41: the wrapping of wall types at ends
            // and inserts (`wrap_ends`, `wrap_inserts`); older types keep
            // the reveals of façades wrapped, as they were drawn. 41 → 42:
            // pools built in the ground (`pools`); older files have none.
            // 42 → 43: railings may take a type (`railing_type`); older
            // railings have none and are drawn as before. 43 → 44: stairs
            // may take a railing type too (`railing_type`). 44 → 45:
            // terrains may have an edge drawn by hand (`outline`) and say
            // whether their heights and slope arrows are drawn; older
            // ones have neither. 45 → 46: terrains may have platforms
            // (`pads`) and keep the ground as surveyed (`ground`); older
            // ones have none. 46 → 47: walls may have an elevation
            // profile (`profile`); older walls are their rectangle. 47 → 48:
            // views only for drawing (`draft_views`), drafting in them and
            // views of them on sheets, and underlays; older files have
            // none. 48 → 49: wall types may have a façade bond and slab
            // fronts (`facade`); older ones have a plain finish. 49 → 50:
            // columns, beams, footings and slabs may have reinforcement
            // (`reinforcements`); older files have none. 50 → 51: more
            // kinds of device (the electrical mechanisms, the board, risers
            // and downpipes) and the circuits of mechanisms
            // (`device_circuits`); older files have neither. 51 → 52:
            // underlays may show only a rectangle of their source (`crop`);
            // older ones show it whole. 52 → 53: drafting elements may be
            // in a group (`group`, a block from a DXF); older ones are not.
            // 53 → 54: faces of slabs, roofs, columns, beams and footings
            // may be painted, and the faces looking up or down divided
            // (`face_zones`); older files have neither. 54 → 55: a light
            // point may have its fitting (`fixture`); older ones are
            // downlights. 55 → 56: the pieces of a stair, a railing or a
            // curtain wall may be painted by material (`material <id>`).
            // 56 → 57: curtain walls may have doors (`doors`); older ones
            // have none. 57 → 58: new kinds of devices (fire protection,
            // the water's equipment, solar panels) and the fire
            // protection's pipes; older files have none. 58 → 59: lifts
            // (`elevators`); older files have none. 59 → 60: a road's
            // section is its strips across (`strips`, with parking modes,
            // build-outs, crossings); older ones keep their four widths,
            // read as the same section. 60 → 61: a road's profile
            // (`profile`) and the radius of its corners (`corner_radius`);
            // older roads are level, with corners of 4 m. 61 → 62: a road's
            // changes of section along it (`changes`); older ones have none.
            // 62 → 63: new bands of a road's strips (bus, tram, shoulder,
            // ditch, tree pits, bike path on the sidewalk); nothing to change,
            // only so that older versions say they cannot read them.
            9..=62 => {
                let mut v = if version == 32 {
                    legacy_dormers(value)
                } else {
                    value
                };
                if let Value::Object(map) = &mut v {
                    map.insert("version".into(), Value::from(version + 1));
                }
                v
            }
            other => return Err(FileError::UnknownVersion(other)),
        };
        version += 1;
    }
    Ok(value)
}

/// ArchiSim 0.48's version 32 dormers (by `position`) moved to each
/// roof's `legacy_dormers`, read apart; the others are left as they are.
fn legacy_dormers(mut value: Value) -> Value {
    let Some(roofs) = value.get_mut("roofs").and_then(Value::as_array_mut) else {
        return value;
    };
    for roof in roofs {
        let Some(map) = roof.as_object_mut() else {
            continue;
        };
        let Some(Value::Array(list)) = map.remove("dormers") else {
            continue;
        };
        let (old, new): (Vec<Value>, Vec<Value>) =
            list.into_iter().partition(|d| d.get("position").is_some());
        if !new.is_empty() {
            map.insert("dormers".into(), Value::Array(new));
        }
        if !old.is_empty() {
            map.insert("legacy_dormers".into(), Value::Array(old));
        }
    }
    value
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
    let first_material = map
        .get("materials")
        .and_then(Value::as_array)
        .and_then(|ms| ms.iter().filter_map(|m| m.get("id")?.as_u64()).min());
    let mut families: Vec<Value> = Vec::new();
    let mut by_name: Vec<(&str, u64, archisim_core::Values, Vec<String>)> = Vec::new();
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
            let family = match by_name.iter().find(|(n, _, _, _)| *n == name) {
                Some((_, fid, _, _)) => *fid,
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
                    let materials = parsed
                        .def()
                        .params_of(archisim_core::ParamScope::Type)
                        .filter(|p| p.kind == archisim_core::ParamKind::Material)
                        .map(|p| p.name.clone())
                        .collect();
                    by_name.push((name, next_id, defaults, materials));
                    next_id
                }
            };
            // The family's defaults (for its other parameters, such as how
            // the leaves it does not use open), then the type's own values.
            let (_, _, defaults, materials) = by_name
                .iter()
                .find(|(n, _, _, _)| *n == name)
                .expect("added");
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
            // Materials the old types did not have (a transom's glass, a
            // subframe…): the project's first, as a new type would take.
            if let Some(first) = first_material {
                for k in materials {
                    if !values.get(k).is_some_and(Value::is_u64) {
                        values.insert(k.clone(), Value::from(first));
                    }
                }
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

    /// A material for the treads: the model's last.
    fn earth_for_treads(m: &archisim_core::Model) -> archisim_core::MaterialId {
        m.materials().last().unwrap().id
    }
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
        // A wooden floor drawn in plan (version 22), in oak (version 36).
        r.floor_fill = Some(archisim_core::FloorFill {
            pattern: archisim_core::HatchPattern::Oak,
            color: [120, 80, 40],
            background: Some([230, 210, 180]),
            scale: 1.0,
            angle: 0.5,
        });
        m.set_room(r).unwrap();
        // A tree drawn its own way (version 23).
        {
            let f = m.system_family("Árbol").unwrap();
            let values = m
                .family_defaults(f, archisim_core::ParamScope::Type)
                .unwrap();
            let t = m.add_object_type("Árbol", f, values).unwrap();
            let o = m
                .add_object(archisim_core::NewObject {
                    level: levels[0],
                    object_type: t,
                    position: Point2::new(-3.0, 2.0),
                    rotation: 0.0,
                    base_offset: 0.0,
                })
                .unwrap();
            let own: archisim_core::Values = [
                (
                    "shape_own".to_string(),
                    archisim_core::Value::Text("acuarela".into()),
                ),
                ("opacity_own".to_string(), archisim_core::Value::Number(0.4)),
                ("color_own".to_string(), archisim_core::Value::Bool(true)),
                (
                    "leaves_own".to_string(),
                    archisim_core::Value::Material(m.materials().next().unwrap().id.0),
                ),
            ]
            .into_iter()
            .collect();
            m.set_object_values(o, own).unwrap();
            // Another tree of its own type (version 35), as the Maqueta
            // leaves an object it changed.
            let o2 = m
                .add_object(archisim_core::NewObject {
                    level: levels[0],
                    object_type: t,
                    position: Point2::new(-6.0, 2.0),
                    rotation: 0.0,
                    base_offset: 0.0,
                })
                .unwrap();
            let own_t = m.own_object_type(o2).unwrap();
            assert_eq!(
                m.object_type(own_t)
                    .unwrap()
                    .own
                    .map(|w| (w.object, w.base)),
                Some((o2, t))
            );
            // A 3D model shown by a type of the tree (version 24): a
            // tetrahedron and a lighter version of one triangle.
            let mat = m.materials().next().unwrap().id;
            let mesh = m
                .add_mesh(archisim_core::MeshModel {
                    id: archisim_core::MeshId(0),
                    name: "Árbol importado".into(),
                    source: "arbol.dae".into(),
                    levels: vec![
                        archisim_core::MeshLevel {
                            positions: vec![
                                [0.0, 0.0, 0.0],
                                [1.0, 0.0, 0.0],
                                [0.0, 1.0, 0.0],
                                [0.25, 0.25, 4.0],
                            ],
                            colors: vec![[10, 20, 30], [40, 50, 60], [70, 80, 90], [1, 2, 3]],
                            groups: vec![archisim_core::MeshGroup {
                                material: mat,
                                triangles: vec![[0, 2, 1], [0, 1, 3], [1, 2, 3], [2, 0, 3]],
                            }],
                        },
                        archisim_core::MeshLevel {
                            positions: vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.1], [0.0, 1.0, 4.0]],
                            colors: vec![[1, 1, 1]; 3],
                            groups: vec![archisim_core::MeshGroup {
                                material: mat,
                                triangles: vec![[0, 1, 2]],
                            }],
                        },
                    ],
                })
                .unwrap();
            let values = m
                .family_defaults(f, archisim_core::ParamScope::Type)
                .unwrap();
            let t2 = m.add_object_type("Árbol de malla", f, values).unwrap();
            m.set_object_type_mesh(t2, Some(mesh)).unwrap();
        }
        // A presentation style (version 22).
        {
            let mut st = archisim_core::PresentationStyle::new("Terracota");
            st.palette = vec![[60, 30, 20], [240, 230, 220]];
            st.poche = Some([180, 90, 60]);
            st.line_scale = 0.7;
            st.shadows = true;
            m.add_presentation_style(st).unwrap();
            m.set_sun(archisim_core::Sun {
                day: 172,
                hour: 16.5,
            })
            .unwrap();
        }
        // Prices of the budget (version 31).
        m.set_price(
            "muro:1",
            Some(archisim_core::PriceConcept {
                code: "FFQ010".into(),
                unit: "m2".into(),
                summary: "Hoja de partición".into(),
                price: 28.4,
                text: "Hoja de partición interior de ladrillo…".into(),
                bank: "Banco de prueba".into(),
                // Its decomposition (version 32).
                pieces: vec![
                    archisim_core::PricePiece {
                        code: "mo021".into(),
                        unit: "h".into(),
                        summary: "Oficial 1ª construcción".into(),
                        kind: 1,
                        quantity: 0.6,
                        price: 21.5,
                    },
                    archisim_core::PricePiece {
                        code: "%".into(),
                        unit: "%".into(),
                        summary: "Costes directos complementarios".into(),
                        kind: 0,
                        quantity: 2.0,
                        price: 0.0,
                    },
                ],
            }),
        )
        .unwrap();
        // Overheads, profit and VAT of the budget (version 32).
        m.set_budget_settings(archisim_core::BudgetSettings {
            overheads: 0.15,
            profit: 0.06,
            vat: 0.1,
        })
        .unwrap();
        // Saved plans, level styles, templates, reference planes and the
        // technical lettering (version 27).
        {
            let pb = m.levels()[0].id;
            let mut style = archisim_core::ViewStyle::ad5();
            style.hidden.insert(archisim_core::Category::Objects);
            m.add_plan_view(archisim_core::PlanView {
                id: archisim_core::PlanViewId(0),
                name: "Planta baja — techos".into(),
                level: pb,
                kind: archisim_core::PlanKind::Ceiling,
                scale: 100.0,
                cut_plane: 1.5,
                style,
            })
            .unwrap();
            m.set_level_style(pb, archisim_core::ViewStyle::ad5())
                .unwrap();
            m.add_view_template(archisim_core::ViewTemplate {
                id: archisim_core::ViewTemplateId(0),
                name: "AD5".into(),
                style: archisim_core::ViewStyle::ad5(),
            })
            .unwrap();
            m.add_ref_plane(
                "Eje de simetría",
                Point2::new(6.0, -2.0),
                Point2::new(6.0, 11.0),
            )
            .unwrap();
            m.graphics.font = archisim_core::TextFont::Technical;
        }
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
        // The first wall is a bearing wall, by hand (version 15).
        m.set_discipline(first, Some(archisim_core::ElementDiscipline::Structure))
            .unwrap();
        // The first wall's exterior face painted (version 21).
        {
            let w = m.walls().next().unwrap().id;
            let mat = m.materials().last().unwrap().id;
            m.set_face_paint(
                w.0,
                archisim_core::FaceRef::Wall(archisim_core::FaceSide::Exterior),
                Some(mat),
            )
            .unwrap();
            m.set_paint_zone(
                w,
                archisim_core::FaceSide::Interior,
                vec![
                    archisim_core::Point2::new(0.5, 0.0),
                    archisim_core::Point2::new(1.5, 0.0),
                    archisim_core::Point2::new(1.5, 1.0),
                ],
                Some(mat),
            )
            .unwrap();
        }
        // A loose edge of the concept phase (version 21).
        {
            let level = m.levels()[0].id;
            m.add_edge_3d(
                level,
                vec![
                    archisim_core::Point2::new(0.0, -2.0),
                    archisim_core::Point2::new(3.0, -2.0),
                    archisim_core::Point2::new(3.0, -4.0),
                ],
                vec![0.0, 0.0, 1.5],
                0.5,
            )
            .unwrap();
        }
        // A pipe, a duct and a light (version 20).
        {
            m.add_run(archisim_core::NewRun {
                level: levels[0],
                system: archisim_core::ServiceSystem::HotWater,
                path: vec![
                    Point2::new(0.5, 0.5),
                    Point2::new(3.0, 0.5),
                    Point2::new(3.0, 2.0),
                ],
                elevation: 0.4,
                width: 0.02,
                height: 0.02,
                slope: 0.0,
            })
            .unwrap();
            // A drain with its slope (version 32).
            m.add_run(archisim_core::NewRun {
                level: levels[0],
                system: archisim_core::ServiceSystem::Drainage,
                path: vec![Point2::new(0.5, 1.0), Point2::new(3.5, 1.0)],
                elevation: 0.3,
                width: 0.11,
                height: 0.11,
                slope: 0.02,
            })
            .unwrap();
            m.add_run(archisim_core::NewRun {
                level: levels[0],
                system: archisim_core::ServiceSystem::ExhaustAir,
                path: vec![Point2::new(0.5, 3.0), Point2::new(4.0, 3.0)],
                elevation: 2.6,
                width: 0.3,
                height: 0.2,
                slope: 0.0,
            })
            .unwrap();
            m.add_device(archisim_core::NewDevice {
                level: levels[0],
                kind: archisim_core::DeviceKind::Light,
                name: "Downlight".into(),
                position: Point2::new(2.0, 2.0),
                rotation: 0.0,
                elevation: 2.6,
                width: 0.2,
                depth: 0.2,
                height: 0.1,
            })
            .unwrap();
        }
        // A plot with its setback and a road (version 19).
        {
            let mat = m.materials().next().unwrap().id;
            m.add_plot(archisim_core::NewPlot {
                name: "Parcela 1".into(),
                level: levels[0],
                outline: vec![
                    Point2::new(-5.0, -5.0),
                    Point2::new(45.0, -5.0),
                    Point2::new(45.0, 20.0),
                    Point2::new(-5.0, 20.0),
                ],
                setback: 3.0,
                reference: "1234567VK4713S0001AB".into(),
            })
            .unwrap();
            m.add_road(archisim_core::NewRoad {
                name: "Calle Mayor".into(),
                level: levels[0],
                axis: vec![
                    Point2::new(-5.0, -15.0),
                    Point2::new(45.0, -15.0),
                    Point2::new(60.0, -5.0),
                ],
                section: archisim_core::RoadSection::default(),
                top_offset: -0.15,
                profile: Vec::new(),

                changes: Vec::new(),
                pavement: mat,
                kerb_material: mat,
                sidewalk_material: mat,
            })
            .unwrap();
        }
        // A mass by floors turned into a building, and a band (version 18).
        {
            let mat = m.materials().next().unwrap().id;
            m.set_plot_area(Some(500.0)).unwrap();
            let mass = m
                .add_mass(archisim_core::NewMass {
                    name: "Masa 1".into(),
                    level: levels[0],
                    shape: archisim_core::MassShape::Prism(vec![
                        Point2::new(30.0, 0.0),
                        Point2::new(40.0, 0.0),
                        Point2::new(40.0, 8.0),
                        Point2::new(30.0, 8.0),
                    ]),
                    base_offset: 0.0,
                    height: 6.0,
                    floor_height: Some(3.0),
                    material: mat,
                })
                .unwrap();
            let wt = m.wall_types().next().unwrap().id;
            let st = m.slab_types().next().unwrap().id;
            m.mass_to_building(mass, wt, st, st, st).unwrap();
            m.add_mass(archisim_core::NewMass {
                name: "Pérgola".into(),
                level: levels[0],
                shape: archisim_core::MassShape::Sweep {
                    path: vec![Point2::new(30.0, -3.0), Point2::new(40.0, -3.0)],
                    width: 1.0,
                    profile: None,
                },
                base_offset: 2.5,
                height: 0.3,
                floor_height: None,
                material: mat,
            })
            .unwrap();
            // A cornice by its own profile, and a twisted tower (version 28).
            m.add_mass(archisim_core::NewMass {
                name: "Cornisa".into(),
                level: levels[0],
                shape: archisim_core::MassShape::Sweep {
                    path: vec![Point2::new(30.0, -6.0), Point2::new(40.0, -6.0)],
                    width: 0.4,
                    profile: Some(vec![
                        Point2::new(-0.2, 0.0),
                        Point2::new(0.2, 0.0),
                        Point2::new(0.2, 0.3),
                        Point2::new(-0.1, 0.3),
                    ]),
                },
                base_offset: 3.0,
                height: 0.3,
                floor_height: None,
                material: mat,
            })
            .unwrap();
            let tower = m
                .add_mass(archisim_core::NewMass {
                    name: "Torre".into(),
                    level: levels[0],
                    shape: archisim_core::MassShape::Prism(vec![
                        Point2::new(50.0, 0.0),
                        Point2::new(56.0, 0.0),
                        Point2::new(56.0, 6.0),
                        Point2::new(50.0, 6.0),
                    ]),
                    base_offset: 0.0,
                    height: 30.0,
                    floor_height: None,
                    material: mat,
                })
                .unwrap();
            m.edit_mass(tower, |x| {
                x.deform = Some(archisim_core::Deform {
                    twist: 0.5,
                    bend: 0.25,
                    bend_direction: 1.0,
                })
            })
            .unwrap();
            // Accessories of a wall type and the edge of a slab type
            // (version 29).
            let wt = m.wall_types().next().unwrap().id;
            let st = m.slab_types().next().unwrap().id;
            let t = m.wall_types().next().unwrap().thickness();
            m.set_wall_type_sweeps(
                wt,
                vec![
                    archisim_core::WallSweep {
                        kind: archisim_core::SweepKind::Skirting,
                        side: archisim_core::SweepSide::Interior,
                        offset: 0.0,
                        from_top: false,
                        profile: archisim_core::sweep_preset(archisim_core::SweepKind::Skirting, t),
                        material: mat,
                    },
                    archisim_core::WallSweep {
                        kind: archisim_core::SweepKind::Coping,
                        side: archisim_core::SweepSide::Top,
                        offset: 0.0,
                        from_top: true,
                        profile: archisim_core::sweep_preset(archisim_core::SweepKind::Coping, t),
                        material: mat,
                    },
                ],
            )
            .unwrap();
            m.set_slab_type_edge(
                st,
                Some(archisim_core::SlabEdge {
                    profile: archisim_core::slab_edge_preset(true),
                    material: mat,
                }),
            )
            .unwrap();
            // A plan symbol on an object type (version 30).
            let first_type = m.object_types().next().map(|t| t.id);
            if let Some(ot) = first_type {
                m.set_object_type_symbol(
                    ot,
                    Some(archisim_core::PlanSymbol {
                        source: "pino.dxf".into(),
                        fit: true,
                        items: vec![
                            archisim_core::SymbolItem {
                                shape: archisim_core::Shape::Circle {
                                    centre: Point2::new(0.0, 0.0),
                                    radius: 0.5,
                                },
                                color: Some([40, 120, 60]),
                                weight: Some(0.25),
                            },
                            archisim_core::SymbolItem {
                                shape: archisim_core::Shape::Line {
                                    a: Point2::new(-0.5, 0.0),
                                    b: Point2::new(0.5, 0.0),
                                },
                                color: None,
                                weight: None,
                            },
                        ],
                    }),
                )
                .unwrap();
                // Two drawings of the species, taken in turn (version 37).
                let first = m.object_type(ot).unwrap().symbol.clone().unwrap();
                let mut second = first.clone();
                second.source = "pino.dxf · 2".into();
                second.items.truncate(1);
                m.set_object_type_symbols(ot, vec![first, second]).unwrap();
                assert_eq!(m.object_type(ot).unwrap().more_symbols.len(), 1);
            }
        }
        // A module placed twice, once mirrored (version 17).
        {
            let mut house = Model::new("Tipo");
            house.add_level("Baja", 0.0);
            let text = to_string(&house);
            let (text, stamp, house) = read_module(&text).unwrap();
            let src = m
                .add_module_source("Tipo", "tipo.archisim", text.into(), stamp, house)
                .unwrap();
            for mirror in [false, true] {
                m.add_module(archisim_core::NewModulePlacement {
                    source: src,
                    level: levels[0],
                    position: Point2::new(20.0, 0.0),
                    rotation: 0.5,
                    mirror,
                })
                .unwrap();
            }
        }
        // A grid line and a pad under the column (version 16).
        m.add_grid(archisim_core::NewGrid {
            name: "A".into(),
            start: Point2::new(-1.0, 0.0),
            end: Point2::new(6.0, 0.0),
        })
        .unwrap();
        let concrete_mat = m.materials().next().unwrap().id;
        m.add_footing(archisim_core::NewFooting {
            level: levels[0],
            kind: archisim_core::FootingKind::Pad {
                position: Point2::new(0.0, 0.0),
                length: 1.2,
                rotation: 0.0,
                column: Some(archisim_core::ColumnId(col_id)),
            },
            material: concrete_mat,
            width: 1.2,
            depth: 0.5,
            top_offset: -0.4,
        })
        .unwrap();
        m.add_footing(archisim_core::NewFooting {
            level: levels[0],
            kind: archisim_core::FootingKind::TieBeam {
                start: Point2::new(0.0, 0.0),
                end: Point2::new(4.0, 0.0),
            },
            material: concrete_mat,
            width: 0.4,
            depth: 0.4,
            top_offset: -0.4,
        })
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
            design: Default::default(),
        })
        .unwrap();
        // A spiral one with stringers, open treads of another material.
        let spiral = m
            .add_stair(archisim_core::NewStair {
                level: levels[0],
                path: vec![Point2::new(6.0, 6.0), Point2::new(6.0, 6.7)],
                width: 0.9,
                going: 0.25,
                base_offset: 0.0,
                top: archisim_core::WallTop::Unconnected,
                height: 2.8,
                waist: 0.15,
                material: concrete,
                usage: archisim_core::StairUse::Restricted,
                design: archisim_core::StairDesign {
                    form: archisim_core::StairForm::Spiral,
                    sweep: -5.0,
                    core: 0.12,
                    structure: archisim_core::StairStructure::Stringers,
                    closed_risers: false,
                    tread: 0.05,
                    tread_material: Some(earth_for_treads(&m)),
                    stringer: [0.06, 0.25],
                    split: false,
                    arrival: std::f64::consts::FRAC_PI_2,
                },
            })
            .unwrap();
        assert_eq!(
            m.stair(spiral).unwrap().design.form,
            archisim_core::StairForm::Spiral
        );
        // A view for drawing with a line, its underlay, on a sheet, and
        // under the ground floor (version 48).
        {
            let dv = m.add_draft_view("Detalle de alero", 5.0).unwrap();
            let layer = m.draw_layers()[0].id;
            let line = m
                .add_draft(archisim_core::NewDraft::new(
                    archisim_core::ViewRef::Drafting(dv),
                    layer,
                    archisim_core::Shape::Line {
                        a: Point2::new(0.0, 0.0),
                        b: Point2::new(0.4, 0.2),
                    },
                ))
                .unwrap();
            // A block from a DXF, kept as a group (version 53).
            m.group_drafts(&[line], "árbol").unwrap();
            // A slab's top painted and divided, a zone painted (version 54).
            let first = m.slabs().next().map(|s| (s.id, s.outline[0]));
            if let Some((slab, at)) = first {
                let mat = m.materials().next().unwrap().id;
                m.set_face_paint(slab.0, archisim_core::FaceRef::Top, Some(mat))
                    .unwrap();
                m.set_face_zone(
                    slab.0,
                    archisim_core::FaceRef::Top,
                    vec![
                        at,
                        at + archisim_core::Vec2::new(0.5, 0.0),
                        at + archisim_core::Vec2::new(0.5, 0.5),
                    ],
                    Some(mat),
                )
                .unwrap();
            }
            let pb = m.levels()[0].id;
            let mut v = m.draft_view(dv).unwrap().clone();
            let mut u = archisim_core::Underlay::of(archisim_core::UnderlaySource::Level(pb));
            u.offset = Point2::new(1.5, -2.0);
            u.rotation = 0.3;
            v.underlays.push(u);
            m.set_drafting_view(v).unwrap();
            m.set_level_underlays(
                pb,
                vec![archisim_core::Underlay::of(
                    archisim_core::UnderlaySource::Drafting(dv),
                )],
            )
            .unwrap();
            if let Some(sheet) = m.sheets().first().map(|s| s.id) {
                m.add_drafting_viewport(sheet, dv, Point2::new(0.1, 0.1), None)
                    .unwrap();
            }
            // A detail over a piece of a section (version 52).
            let sec = m.sections().next().map(|s| s.id);
            if let Some(sec) = sec {
                m.add_section_detail(sec, Point2::new(0.5, 2.0), Point2::new(1.5, 3.2), 10.0)
                    .unwrap();
            }
        }
        // A gable wall with a round hole (version 47).
        let straight = m
            .walls()
            .find(|w| !w.is_curved() && w.profile.is_none())
            .map(|w| (w.id, w.length(), w.height))
            .unwrap();
        let mut gable = archisim_core::WallProfile::gable(straight.1, straight.2, straight.2 + 1.2);
        gable.holes.push(
            (0..12)
                .map(|i| {
                    let a = i as f64 * std::f64::consts::TAU / 12.0;
                    Point2::new(straight.1 / 2.0 + 0.3 * a.cos(), straight.2 + 0.3 * a.sin())
                })
                .collect(),
        );
        m.set_wall_profile(straight.0, Some(gable)).unwrap();
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
        .map(|t| {
            // A lawn painted over it, with a hole (version 37).
            m.edit_terrain(t, |t| {
                t.patches = vec![archisim_core::TerrainPatch {
                    material: earth,
                    regions: vec![(
                        vec![
                            Point2::new(0.0, 0.0),
                            Point2::new(4.0, 0.0),
                            Point2::new(4.0, 3.0),
                        ],
                        vec![vec![
                            Point2::new(2.0, 0.5),
                            Point2::new(3.0, 0.5),
                            Point2::new(3.0, 1.2),
                        ]],
                    )],
                }];
                // A platform dug into it (version 46).
                t.set_pads(vec![archisim_core::TerrainPad {
                    outline: vec![
                        Point2::new(6.0, 6.0),
                        Point2::new(10.0, 6.0),
                        Point2::new(10.0, 9.0),
                        Point2::new(6.0, 9.0),
                    ],
                    z: -0.6,
                    slope: 1.0,
                }]);
            })
            .unwrap()
        })
        .unwrap();
        // A meadow with flowers on that earth, of soil (version 39).
        m.set_appearance(
            earth,
            archisim_core::Appearance {
                color: Some([120, 150, 80]),
                texture: Some(archisim_core::TextureKind::Soil),
                scale: None,
                growth: Some(archisim_core::Growth::flowering(
                    0.3,
                    12.0,
                    &[[240, 200, 40], [140, 120, 220]],
                    9.0,
                )),
            },
        )
        .unwrap();
        // A pool with its slope turned (version 42).
        {
            let level = m.levels()[0].id;
            let mut spec = archisim_core::NewPool::new(
                level,
                vec![
                    Point2::new(20.0, 0.0),
                    Point2::new(28.0, 0.0),
                    Point2::new(28.0, 4.0),
                    Point2::new(20.0, 4.0),
                ],
                earth,
            );
            spec.slope = 0.5;
            spec.deep = 1.8;
            m.add_pool(spec).unwrap();
        }
        // A lift from the first level to the second (version 59).
        {
            let mut lift =
                archisim_core::NewElevator::new(levels[0], levels[1], Point2::new(32.0, 10.0));
            lift.rotation = 0.5;
            lift.car_at = Some(levels[1]);
            m.add_elevator(lift).unwrap();
        }
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
        // With a roof window, tiles and gutters (version 32).
        let roof = m.roofs().next().unwrap().id;
        m.add_dormer(
            roof,
            archisim_core::DormerKind::Skylight,
            Point2::new(2.5, 0.6),
        )
        .unwrap();
        let gutter = m.default_gutter();
        m.set_roof_covering(
            roof,
            archisim_core::RoofCovering {
                tiles: Some(archisim_core::TileKind::Slate),
                tile_material: None,
                gutter,
            },
        )
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
        assert!(text.contains("\"kind\": \"skylight\""), "{text}");
        assert!(text.contains("\"tiles\": \"slate\""), "{text}");
        assert!(text.contains("\"shape\": \"i_shape\""), "{text}");
        assert!(
            text.contains("\"slopes\": [") && text.contains("null"),
            "{text}"
        );
        let back = from_str(&text).unwrap();
        assert_eq!(back, m);
        // A version 6 file has none of them and still opens.
        let v6 = to_string(&sample()).replacen("\"version\": 62", "\"version\": 6", 1);
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
        assert!(to_string(&opened).contains("\"version\": 62"));
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
                OpeningKind::Door(
                    DoorOperation::Pivot | DoorOperation::Overhead | DoorOperation::Rolling,
                ) => ("door", "single_swing"),
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

    /// Version 41: the wrapping of wall types; older files keep the
    /// reveals of façades wrapped, as they were drawn.
    #[test]
    fn wall_type_wrapping() {
        use archisim_core::Wrap;
        let mut m = sample();
        let t = m.wall_types().next().unwrap().id;
        m.set_wall_type_wrap(t, Wrap::Exterior, Wrap::Both).unwrap();
        let text = to_string(&m);
        assert!(text.contains("\"wrap_ends\": \"exterior\""));
        let back = from_str(&text).unwrap();
        assert_eq!(back, m);
        let plain = to_string(&sample());
        assert!(!plain.contains("wrap_"));
        let old = from_str(&plain.replacen("\"version\": 62", "\"version\": 40", 1)).unwrap();
        for t in old.wall_types() {
            assert_eq!(t.wrap_ends, Wrap::None);
            assert_eq!(
                t.wrap_inserts,
                Wrap::inserts_by_default(t.function, &t.layers)
            );
        }
    }

    /// Version 49: the façade of a wall type (bond, pattern and slab
    /// fronts); older files have plain finishes.
    #[test]
    fn wall_type_facades() {
        use archisim_core::{Bond, Facade, FacadeSystem, Pattern, SlabFront};
        let mut m = sample();
        let t = m.wall_types().next().unwrap().id;
        let dark = m.materials().next().unwrap().id;
        let mut f = Facade::default();
        f.coursing.bond = Bond::Flemish;
        f.coursing.accent = Some(dark);
        f.coursing.pattern = Pattern::Diamonds(3);
        f.slab_front = Some(SlabFront {
            projection: 0.02,
            ..SlabFront::soldiers()
        });
        m.set_wall_type_facade(t, Some(f)).unwrap();
        let text = to_string(&m);
        assert!(text.contains("\"bond\": \"flemish\""));
        let back = from_str(&text).unwrap();
        assert_eq!(back, m);
        assert_eq!(
            back.wall_type(t).unwrap().facade.unwrap().coursing.system,
            FacadeSystem::Brick
        );
        let plain = to_string(&sample());
        let old = from_str(&plain.replacen("\"version\": 62", "\"version\": 48", 1)).unwrap();
        assert!(old.wall_types().all(|t| t.facade.is_none()));
    }

    /// Version 50: the reinforcement of columns, beams, footings and
    /// slabs; older files have none.
    #[test]
    fn reinforcement_is_kept() {
        use archisim_core::{RebarKind, Reinforcement};
        let mut m = sample();
        let slab = m.slabs().next().unwrap().id;
        let r = Reinforcement::default_for(RebarKind::Slab);
        m.set_reinforcement(slab.0, Some(r)).unwrap();
        let text = to_string(&m);
        assert!(text.contains("\"reinforcements\""));
        let back = from_str(&text).unwrap();
        assert_eq!(back, m);
        assert_eq!(back.reinforcement(slab.0), Some(&r));
    }

    /// Version 40: an end that may not join (Revit's «No permitir unión»)
    /// is kept; older files open with every end joining.
    #[test]
    fn wall_ends_that_may_not_join() {
        let mut m = sample();
        let w = m.walls().next().unwrap().id;
        m.set_wall_join(w, false, false).unwrap();
        let text = to_string(&m);
        assert!(text.contains("\"allow_join\""));
        let back = from_str(&text).unwrap();
        assert_eq!(back.wall(w).unwrap().allow_join, [true, false]);
        assert_eq!(back, m);
        let plain = to_string(&sample());
        assert!(!plain.contains("allow_join"));
        let old = plain.replacen("\"version\": 62", "\"version\": 39", 1);
        assert!(from_str(&old)
            .unwrap()
            .walls()
            .all(|w| w.allow_join == [true, true]));
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
            .replacen("\"version\": 62", "\"version\": 3", 1)
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
        let v5 = to_string(&sample()).replacen("\"version\": 62", "\"version\": 5", 1);
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
        // A door in the first (version 57).
        let door = m
            .opening_types()
            .find(|t| matches!(t.kind, archisim_core::OpeningKind::Door(_)))
            .map(|t| t.id);
        if let Some(door) = door {
            let cw = m.curtain_walls().next().unwrap().id;
            m.add_curtain_door(
                cw,
                1,
                door,
                archisim_core::Hand::Right,
                archisim_core::WallSide::Exterior,
            )
            .unwrap();
        }
        let text = to_string(&m);
        assert!(text.contains("\"doors\""));
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

    /// Drawing layers and every kind of drafting element (version 7) are
    /// saved and come back; version 6 files open with layer "0" only.
    #[test]
    fn drafting_and_version_6_files() {
        let mut m = sample();
        let level = m.levels()[0].id;
        let v = ViewRef::Level(level);
        let ejes = m
            .add_draw_layer(DrawLayer {
                locked: true,
                printable: false,
                ..DrawLayer::new("Ejes", [200, 30, 30], 0.13, LineStyle::DashDot)
            })
            .unwrap();
        let p = |x, y| Point2::new(x, y);
        let shapes = vec![
            Shape::Line {
                a: p(0.0, 0.0),
                b: p(4.0, 1.0),
            },
            Shape::Polyline(Polyline {
                points: vec![p(0.0, 0.0), p(2.0, 0.0), p(2.0, 2.0)],
                bulges: vec![0.0, 0.5, 0.0],
                closed: false,
                width: 0.05,
            }),
            Shape::Circle {
                centre: p(1.0, 1.0),
                radius: 0.5,
            },
            Shape::Arc {
                centre: p(1.0, 1.0),
                radius: 2.0,
                start: 0.3,
                sweep: 1.2,
            },
            Shape::Ellipse {
                centre: p(3.0, 3.0),
                axis: p(2.0, 1.0),
                ratio: 0.4,
                start: 0.0,
                sweep: std::f64::consts::TAU,
            },
            Shape::Point { at: p(-1.0, -1.0) },
            Shape::Text(Text {
                at: p(0.0, 5.0),
                content: "Salón-comedor\n25,40 m²".into(),
                height: 3.5,
                angle: 0.25,
                align: TextAlign::Middle,
            }),
            Shape::Hatch(Hatch {
                loops: vec![
                    Polyline::straight(
                        vec![p(0.0, 0.0), p(3.0, 0.0), p(3.0, 3.0), p(0.0, 3.0)],
                        true,
                    ),
                    Polyline::straight(vec![p(1.0, 1.0), p(2.0, 1.0), p(2.0, 2.0)], true),
                ],
                pattern: HatchPattern::Brick,
                scale: 1.5,
                angle: 0.1,
                background: Some([250, 250, 240]),
                opacity: 0.6,
            }),
        ];
        for (i, s) in shapes.into_iter().enumerate() {
            let mut d = NewDraft::new(v, if i % 2 == 0 { ejes } else { LAYER_ZERO }, s);
            if i == 3 {
                d.color = Some([0, 128, 255]);
                d.weight = Some(0.35);
                d.line = Some(LineStyle::Dashed);
            }
            m.add_draft(d).unwrap();
        }
        let text = to_string(&m);
        assert!(text.contains("\"draw_layers\""));
        assert!(text.contains("\"kind\": \"hatch\""));
        let back = from_str(&text).unwrap();
        assert_eq!(back.draw_layers(), m.draw_layers());
        assert_eq!(
            back.drafts().collect::<Vec<_>>(),
            m.drafts().collect::<Vec<_>>()
        );
        assert_same_project(&back, &m);
        // Version 6 (ArchiSim 0.20 to 0.24): no layers, no drafting.
        let mut old: Value = serde_json::from_str(&to_string(&sample())).unwrap();
        let map = old.as_object_mut().unwrap();
        map.remove("draw_layers");
        map.remove("drafts");
        map.insert("version".into(), Value::from(6));
        let opened = from_str(&serde_json::to_string_pretty(&old).unwrap()).unwrap();
        assert_eq!(opened.draw_layers().len(), 1);
        assert_eq!(opened.drafts().count(), 0);
        // A wrong alignment is explained.
        let err =
            from_str(&text.replace("\"align\": \"middle\"", "\"align\": \"centro\"")).unwrap_err();
        assert!(err.to_string().contains("centro"), "{err}");
    }

    /// Sheets, their views, their drafting, the project data and the pen
    /// table come back as they were.
    #[test]
    fn sheets_views_info_and_graphics() {
        let mut m = sample();
        let level = m.levels()[0].id;
        m.set_info(ProjectInfo {
            client: "Ayuntamiento".into(),
            site: "Punta Umbría (Huelva)".into(),
            studio: "AD5".into(),
            phase: "Proyecto básico".into(),
            ..Default::default()
        });
        m.graphics.coarse.cut = 0.3;
        let s = m
            .add_sheet(NewSheet {
                number: "A-01".into(),
                name: "Planta baja".into(),
                width: 594.0,
                height: 420.0,
            })
            .unwrap();
        let v = m
            .add_viewport(NewViewport {
                sheet: s,
                level,
                section: None,
                axo: None,
                scale: 50.0,
                cut_plane: 1.1,
                crop: Some((Point2::new(-1.0, -1.0), Point2::new(8.0, 6.0))),
                centre: Point2::new(0.25, 0.22),
            })
            .unwrap();
        let mut vv = m.viewport(v).unwrap().clone();
        vv.style.hidden.insert(Category::Dimensions);
        vv.style.detail = Detail::Medium;
        vv.title = "PLANTA BAJA".into();
        vv.style.hidden.insert(Category::Objects);
        m.set_viewport(vv).unwrap();
        // A section on the sheet too (version 10).
        let sec = m
            .add_section(archisim_core::NewSection {
                name: "A".into(),
                kind: archisim_core::SectionKind::Section,
                start: Point2::new(-1.0, 2.0),
                end: Point2::new(9.0, 2.0),
                depth: 4.0,
            })
            .unwrap();
        m.add_viewport(NewViewport {
            sheet: s,
            level,
            section: Some(sec),
            axo: None,
            scale: 100.0,
            cut_plane: 1.0,
            crop: Some((Point2::new(-1.0, -0.5), Point2::new(11.0, 4.0))),
            centre: Point2::new(0.25, 0.08),
        })
        .unwrap();
        // And an axonometric 3D view (version 14).
        m.add_viewport(NewViewport {
            sheet: s,
            level,
            section: None,
            axo: Some(archisim_core::Axo {
                yaw: -1.0,
                pitch: 0.5,
            }),
            scale: 200.0,
            cut_plane: 1.0,
            crop: Some((Point2::new(-5.0, -3.0), Point2::new(9.0, 6.0))),
            centre: Point2::new(0.45, 0.2),
        })
        .unwrap();
        m.add_draft(NewDraft::new(
            ViewRef::Sheet(s),
            LAYER_ZERO,
            Shape::Line {
                a: Point2::new(0.02, 0.02),
                b: Point2::new(0.1, 0.02),
            },
        ))
        .unwrap();
        let text = to_string(&m);
        let back = from_str(&text).unwrap();
        assert_eq!(back.info(), m.info());
        assert_eq!(back.graphics, m.graphics);
        assert_eq!(back.sheets(), m.sheets());
        assert_eq!(
            back.viewports().collect::<Vec<_>>(),
            m.viewports().collect::<Vec<_>>()
        );
        assert_eq!(
            back.drafts().collect::<Vec<_>>(),
            m.drafts().collect::<Vec<_>>()
        );
        assert_same_project(&back, &m);
        assert_eq!(
            back.viewports().filter(|v| v.section == Some(sec)).count(),
            1
        );
        // Removing the section removes its view.
        let mut m2 = back;
        m2.remove_section(sec).unwrap();
        assert_eq!(m2.viewports().count(), 2);
        assert_eq!(m2.viewports().filter(|v| v.axo.is_some()).count(), 1);
    }

    /// The georeference (version 11) is saved and comes back.
    #[test]
    fn georeference_round_trip() {
        let mut m = sample();
        m.set_georeference(Some(archisim_core::Georeference {
            crs: "EPSG:25829".into(),
            eastings: 680_123.456,
            northings: 4_119_876.5,
            height: 3.2,
        }))
        .unwrap();
        let text = to_string(&m);
        assert!(text.contains("\"crs\": \"EPSG:25829\""));
        let back = from_str(&text).unwrap();
        assert_eq!(back.georeference(), m.georeference());
        assert_same_project(&back, &m);
    }

    /// The code settings and a room's area per occupant (version 12).
    #[test]
    fn code_settings_round_trip() {
        let mut m = sample();
        m.set_code_settings(archisim_core::CodeSettings {
            building_use: archisim_core::BuildingUse::Administrative,
            accessibility: archisim_core::Accessibility::Routes,
            sprinklers: true,
        });
        let level = m.levels()[0].id;
        let id = m
            .add_room(archisim_core::NewRoom {
                level,
                number: "1".into(),
                name: "Instalaciones".into(),
                outline: vec![
                    Point2::new(20.0, 0.0),
                    Point2::new(22.0, 0.0),
                    Point2::new(22.0, 2.0),
                    Point2::new(20.0, 2.0),
                ],
                holes: Vec::new(),
                label: Point2::new(21.0, 1.0),
                bounded: false,
            })
            .unwrap();
        let mut r = m.room(id).unwrap().clone();
        r.occupancy = Some(0.0);
        m.set_room(r).unwrap();
        let text = to_string(&m);
        assert!(text.contains("\"building_use\": \"administrative\""));
        let back = from_str(&text).unwrap();
        assert_eq!(back.code_settings(), m.code_settings());
        assert_eq!(back.room(id).unwrap().occupancy, Some(0.0));
        assert_same_project(&back, &m);
        // A version 11 file is a dwelling.
        let mut v: Value = serde_json::from_str(&to_string(&sample())).unwrap();
        v.as_object_mut()
            .unwrap()
            .insert("version".into(), Value::from(11));
        let old = from_str(&v.to_string()).unwrap();
        assert_eq!(old.code_settings(), archisim_core::CodeSettings::default());
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
        let v4 = to_string(&sample()).replacen("\"version\": 62", "\"version\": 4", 1);
        let opened = from_str(&v4).unwrap();
        assert!(opened.walls().all(|w| w.bulge == 0.0));
        // A bend tighter than the wall is thick is refused on opening.
        let tight = text.replace("\"bulge\": -0.4,", "\"bulge\": 30.0,");
        let err = from_str(&tight).unwrap_err();
        assert!(err.to_string().contains("bends too much"), "{err}");
    }

    /// ArchiSim 0.48 wrote its own version 32, with dormers by the middle
    /// of their front: they open as dormers of today, where they were.
    #[test]
    fn dormers_of_archisim_0_48_are_placed_again() {
        let mut m = sample();
        let level = m.levels()[0].id;
        let st = m.slab_types().next().unwrap().id;
        let roof = m
            .add_roof(NewRoof {
                roof_type: st,
                level,
                outline: vec![
                    Point2::new(20.0, 0.0),
                    Point2::new(30.0, 0.0),
                    Point2::new(30.0, 8.0),
                    Point2::new(20.0, 8.0),
                ],
                slopes: vec![Some(0.6), None, Some(0.6), None],
                base_offset: 3.0,
                overhang: 0.4,
            })
            .unwrap();
        let wall = m.materials().next().unwrap().id;
        let text = to_string(&m).replacen("\"version\": 62", "\"version\": 32", 1);
        let mut v: Value = serde_json::from_str(&text).unwrap();
        let roofs = v["roofs"].as_array_mut().unwrap();
        let at = roofs.iter().position(|r| r["id"] == roof.0).unwrap();
        roofs[at]["dormers"] = serde_json::json!([{
            "position": [25.0, 1.2],
            "width": 1.2,
            "height": 0.3,
            "kind": "shed",
            "pitch": 0.05,
            "wall_material": wall.0,
            "window": true
        }]);
        let opened = from_str(&serde_json::to_string_pretty(&v).unwrap()).unwrap();
        let r = opened.roof(roof).unwrap();
        assert_eq!(r.dormers.len(), 1);
        let d = &r.dormers[0];
        assert_eq!(d.kind, DormerKind::Shed);
        assert!((d.width - 1.2).abs() < 1e-9 && (d.pitch - 0.05).abs() < 1e-9);
        assert_eq!(d.wall_material, wall);
        assert!(r.dormer_shape(d).is_some());
        // Saved again, it is of today's version and keeps it.
        let again = to_string(&opened);
        assert!(!again.contains("legacy_dormers"));
        assert_eq!(
            from_str(&again).unwrap().roof(roof).unwrap().dormers.len(),
            1
        );
    }

    #[test]
    fn layout_is_readable() {
        let text = to_string(&sample());
        assert!(text.starts_with("{\n  \"format\": \"ArchiSim\",\n  \"version\": 62,\n"));
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
            edited("\"version\": 63", "\"version\": 64"),
            Err(FileError::TooNew { version: 64 })
        ));
        assert!(matches!(
            edited("\"version\": 62", "\"version\": 0"),
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

    /// A planting (version 25) keeps its line, its rules and the plants
    /// it made, with their seeds and sizes; a plant taken out by hand
    /// stays out.
    #[test]
    fn plantings_round_trip() {
        use archisim_core::{Arrangement, NewPlanting, Point2, Value, Values};
        let mut m = Model::new("Jardín");
        let pb = m.add_level("Planta baja", 0.0);
        m.add_material(
            "Vegetación",
            "vegetation",
            [100, 140, 80],
            CutPattern::Solid,
        );
        m.add_material("Corteza", "wood", [110, 90, 70], CutPattern::Solid);
        let f = m.system_family(archisim_core::VEGETATION_FAMILY).unwrap();
        let values = m
            .family_defaults(f, archisim_core::ParamScope::Type)
            .unwrap();
        let t = m.add_object_type("Árbol", f, values).unwrap();
        let id = m
            .add_planting(NewPlanting {
                name: "Alineación".into(),
                level: pb,
                path: vec![
                    Point2::new(0.0, 0.0),
                    Point2::new(20.0, 3.0),
                    Point2::new(40.0, 0.0),
                ],
                closed: false,
                smooth: true,
                arrangement: Arrangement::Line,
                count: 6,
                species: vec![t],
                size_variation: 0.4,
                shape_variation: 0.6,
                spread: 0.2,
                seed: 9,
                base_offset: 0.0,
            })
            .unwrap();
        let gone = m.planting(id).unwrap().members[2];
        m.remove_object(gone).unwrap();
        let back = from_str(&to_string(&m)).unwrap();
        let p = back.planting(id).unwrap();
        assert_eq!(p.members.len(), 5);
        assert!(!p.members.contains(&gone));
        assert_eq!(p, &{
            let mut q = m.planting(id).unwrap().clone();
            q.members.retain(|x| *x != gone);
            q
        });
        for mid in &p.members {
            let a = m.object(*mid).unwrap();
            let b = back.object(*mid).unwrap();
            assert_eq!(a, b);
            // The same plant grows again.
            assert_eq!(m.plant_look(*mid), back.plant_look(*mid));
        }
        let own: &Values = &back.object(p.members[0]).unwrap().values;
        assert!(matches!(own.get("plant_seed"), Some(Value::Number(_))));
    }
}
