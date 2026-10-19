//! IFC import (IFC2X3 and IFC4): storeys, walls with their layers, slabs,
//! and doors and windows in walls, keeping every element's `GlobalId`.
//!
//! What the importer reads:
//! - units: SI lengths with any prefix (millimetres from Revit, metres
//!   from ArchiCAD) and conversion-based units (feet, inches, degrees);
//! - the storeys of every building, by the height of their placement;
//! - walls (`IfcWall`, `IfcWallStandardCase`, `IfcWallElementedCase`) by
//!   their axis, or along their placement when there is none; their height
//!   from the body; their layers from the material layer set usage (side,
//!   order and offset from the axis), which also gives the reference line;
//! - slabs (`IfcSlab`) from the outline of a vertical extrusion, or from
//!   the top face of a faceted body; the top face is the reference, as in
//!   ArchiSim; layers from the usage (`AXIS3`);
//! - doors and windows filling an opening in an imported wall: the rough
//!   opening from the opening element, the operation and hand from the
//!   element or its type, frame sizes from the lining properties.
//!
//! The exterior face of a wall is on the left of its axis in ArchiSim.
//! IFC has no exterior side, so for walls marked `IsExternal` the side
//! away from the middle of the storey's walls is taken as the exterior
//! (files written by ArchiSim keep their sides as written).
//!
//! Curved walls are read from an axis that is one circular arc (an
//! `IfcTrimmedCurve` on an `IfcCircle`, or an `IfcIndexedPolyCurve` with
//! one `IfcArcIndex`), with their layers measured radially.
//!
//! What it leaves out (and counts in the report): walls of several pieces
//! or other curves, closed circular walls, sloped slabs, holes in slabs, empty openings, and every other
//! class (columns, beams, stairs, roofs that are not slabs…).

mod compare;
pub mod geometry;
mod members;
pub mod report;
mod roofs;
mod spaces;
mod stairs;

use std::collections::{BTreeMap, HashMap, HashSet};
use std::fmt;

use archisim_core::{
    signed_area, Arc, CutPattern, DoorOperation, GlobalId, Hand, Layer, LayerFunction, LevelId,
    MaterialId, Model, ModelError, NewOpening, NewOpeningType, NewSlab, NewWall, OpeningKind,
    OpeningTypeId, Point2, ReferenceLine, RestoreError, SlabFunction, SlabTypeId, WallFunction,
    WallId, WallSide, WallTypeId, WindowOperation,
};

use crate::step_read::{self, Entity, ParseError, StepFile, Value};
pub use compare::differences;
use geometry::{AxisSeg, Body, Geo, Xf, P3};
pub use report::ImportReport;
use report::{class_name, PART_CLASSES, UNSUPPORTED_CLASSES};

/// The model read from an IFC file and what was left out.
#[derive(Debug, Clone)]
pub struct Imported {
    pub model: Model,
    pub report: ImportReport,
}

/// Why a file could not be imported at all.
#[derive(Debug, Clone, PartialEq)]
pub enum ImportError {
    /// Not a readable STEP file.
    Parse(ParseError),
    /// A STEP file, but not IFC (or an IFC version we do not read).
    Schema(String),
    /// The elements read could not be put together (should not happen).
    Model(RestoreError),
}

impl fmt::Display for ImportError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ImportError::Parse(e) => write!(f, "el fichero no se puede leer ({e})"),
            ImportError::Schema(s) if s.is_empty() => {
                write!(f, "el fichero no dice qué esquema usa; no parece un IFC")
            }
            ImportError::Schema(s) => write!(
                f,
                "el fichero es {s}; ArchiSim lee IFC2X3 e IFC4 (y sus variantes)"
            ),
            ImportError::Model(e) => write!(f, "el modelo importado no es coherente: {e}"),
        }
    }
}

impl std::error::Error for ImportError {}

/// Reads an IFC file into a new model.
pub fn import_ifc(text: &str) -> Result<Imported, ImportError> {
    let file = step_read::parse(text).map_err(ImportError::Parse)?;
    let schema = file.schemas.first().cloned().unwrap_or_default();
    if !(schema.starts_with("IFC2X3") || schema.starts_with("IFC4")) {
        return Err(ImportError::Schema(schema));
    }
    let mut imp = Importer::new(&file, schema);
    imp.run();
    imp.finish()
}

/// Rounds to the micrometre: what the file says, without the noise of the
/// placement arithmetic.
fn snap(v: f64) -> f64 {
    let r = (v * 1e6).round() / 1e6;
    if r == 0.0 {
        0.0
    } else {
        r
    }
}

fn p2(p: P3) -> Point2 {
    Point2::new(p[0], p[1])
}

fn snap2(p: Point2) -> Point2 {
    Point2::new(snap(p.x), snap(p.y))
}

/// Relationships, indexed once.
#[derive(Default)]
struct Index {
    /// Element → spatial structure containing it.
    container: HashMap<u64, u64>,
    /// Part → whole (IfcRelAggregates, IfcRelNests).
    parent: HashMap<u64, u64>,
    /// Element → openings voiding it.
    voids: HashMap<u64, Vec<u64>>,
    /// Opening → door or window filling it.
    fills: HashMap<u64, u64>,
    /// Door or window → the opening it fills.
    filled: HashMap<u64, u64>,
    /// Object → material definition.
    material: HashMap<u64, u64>,
    /// Object → type.
    type_of: HashMap<u64, u64>,
    /// Object → property sets.
    psets: HashMap<u64, Vec<u64>>,
    /// Material → its representation (colours).
    material_rep: HashMap<u64, u64>,
    /// Material → `IfcMaterialProperties`.
    material_props: HashMap<u64, Vec<u64>>,
}

/// A wall read from the file, before it is added to the model.
struct WallCandidate {
    ifc: u64,
    guid: String,
    a: Point2,
    b: Point2,
    /// 0 for a straight wall; the bulge of its arc from a to b if curved.
    bulge: f64,
    /// Layers with their position across the axis a→b (left positive):
    /// (low, high, layer).
    layers: Vec<(f64, f64, Layer)>,
    base: f64,
    top: f64,
    level: LevelId,
    external: bool,
    type_key_name: String,
    ifc_type: Option<u64>,
}

/// An imported wall, as needed to place its doors and windows.
struct WallFrame {
    id: WallId,
    level_elevation: f64,
    origin: Point2,
    u: Point2,
    n: Point2,
    length: f64,
    base: f64,
    top: f64,
    exterior: f64,
    thickness: f64,
    curved: bool,
}

#[derive(Clone, PartialEq, Eq, Hash)]
struct LayersKey(Vec<(u64, i64, u8)>);

fn layers_key(layers: &[Layer]) -> LayersKey {
    LayersKey(
        layers
            .iter()
            .map(|l| {
                (
                    l.material.0,
                    (l.thickness * 1e6).round() as i64,
                    l.function as u8,
                )
            })
            .collect(),
    )
}

/// IFC type, operation, sizes (µm) and materials of a door or window type.
type OpeningTypeKey = (Option<u64>, String, [i64; 4], u64, u64);

struct Importer<'a> {
    file: &'a StepFile,
    geo: Geo<'a>,
    ix: Index,
    model: Model,
    report: ImportReport,
    /// Written by ArchiSim: wall sides are as written.
    own_file: bool,
    /// Storeys by IFC id.
    storeys: HashMap<u64, LevelId>,
    /// (level, elevation) sorted by elevation.
    levels: Vec<(LevelId, f64)>,
    materials: HashMap<u64, MaterialId>,
    materials_by_key: HashMap<(String, String, [u8; 3], u8), MaterialId>,
    wall_types: HashMap<(Option<u64>, bool, LayersKey), WallTypeId>,
    slab_types: HashMap<(Option<u64>, u8, LayersKey), SlabTypeId>,
    opening_types: HashMap<OpeningTypeKey, OpeningTypeId>,
    /// IFC types already given their GUID.
    used_type_guids: HashSet<u64>,
    type_names: HashSet<String>,
    /// GUIDs to restore once the model is built.
    guids: GuidPlan,
    /// The walls made from each IFC wall (one per piece of its axis).
    walls: HashMap<u64, Vec<WallFrame>>,
    member_types: HashMap<members::MemberKey, archisim_core::MemberTypeId>,
    /// Imported elements and the IFC entity each came from, to read their
    /// phase (`Status`) at the end.
    sources: Vec<(u64, u64)>,
    /// `IfcRoof`s read back as ArchiSim roofs.
    roof_ids: HashSet<u64>,
    /// `IfcStair`s read back as ArchiSim stairs.
    stair_ids: HashSet<u64>,
}

#[derive(Default)]
struct GuidPlan {
    levels: Vec<(LevelId, String)>,
    wall_types: Vec<(WallTypeId, String)>,
    slab_types: Vec<(SlabTypeId, String)>,
    opening_types: Vec<(OpeningTypeId, String)>,
    walls: Vec<(WallId, String)>,
    slabs: Vec<(archisim_core::SlabId, String)>,
    openings: Vec<(archisim_core::OpeningId, String)>,
    member_types: Vec<(archisim_core::MemberTypeId, String)>,
    columns: Vec<(archisim_core::ColumnId, String)>,
    beams: Vec<(archisim_core::BeamId, String)>,
    spaces: Vec<(archisim_core::SpaceId, String)>,
    roofs: Vec<(archisim_core::RoofId, String)>,
    stairs: Vec<(archisim_core::StairId, String)>,
    project: Option<String>,
    site: Option<String>,
    building: Option<String>,
}

const DEFAULT_PATTERN: u8 = 0;

fn pattern_code(p: CutPattern) -> u8 {
    match p {
        CutPattern::Solid => 0,
        CutPattern::Diagonal => 1,
        CutPattern::Cross => 2,
        CutPattern::Insulation => 3,
        CutPattern::Dots => 4,
        CutPattern::Empty => 5,
    }
}

/// `CutPattern` from the name written by the exporter.
pub(crate) fn pattern_from_name(name: &str) -> Option<CutPattern> {
    Some(match name {
        "Solid" => CutPattern::Solid,
        "Diagonal" => CutPattern::Diagonal,
        "Cross" => CutPattern::Cross,
        "Insulation" => CutPattern::Insulation,
        "Dots" => CutPattern::Dots,
        "Empty" => CutPattern::Empty,
        _ => return None,
    })
}

/// Name of a `CutPattern` in the file.
pub(crate) fn pattern_name(p: CutPattern) -> &'static str {
    match p {
        CutPattern::Solid => "Solid",
        CutPattern::Diagonal => "Diagonal",
        CutPattern::Cross => "Cross",
        CutPattern::Insulation => "Insulation",
        CutPattern::Dots => "Dots",
        CutPattern::Empty => "Empty",
    }
}

/// Category, colour and hatch guessed from a material name (Spanish and
/// English words used by the usual libraries).
fn guess_material(name: &str) -> (&'static str, [u8; 3], CutPattern) {
    let n = name.to_lowercase();
    let has = |words: &[&str]| words.iter().any(|w| n.contains(w));
    if has(&["vidrio", "glass", "glazing", "cristal", "acristal"]) {
        ("glass", [190, 222, 236], CutPattern::Solid)
    } else if has(&[
        "aislam",
        "aislante",
        "insulat",
        "lana",
        "wool",
        "xps",
        "eps",
        "poliestireno",
        "polyurethane",
        "poliuretano",
        "pir",
        "corcho",
    ]) {
        ("insulation", [250, 243, 205], CutPattern::Insulation)
    } else if has(&["aire", "air", "cámara", "camara", "cavity"]) {
        ("air", [255, 255, 255], CutPattern::Empty)
    } else if has(&[
        "lámina",
        "lamina",
        "membran",
        "impermeab",
        "barrera",
        "vapour",
        "vapor",
        "betún",
        "bitum",
    ]) {
        ("membrane", [70, 70, 70], CutPattern::Solid)
    } else if has(&["hormig", "concrete", "beton", "béton"]) {
        ("concrete", [205, 205, 205], CutPattern::Cross)
    } else if has(&[
        "ladrill",
        "brick",
        "fábrica",
        "fabrica",
        "masonry",
        "bloque",
        "block",
        "termoarcilla",
        "cerámic",
        "ceramic",
        "tile",
        "baldosa",
        "azulejo",
    ]) {
        if has(&["baldosa", "tile", "azulejo", "gres"]) {
            ("ceramic", [214, 206, 190], CutPattern::Solid)
        } else {
            ("brick", [232, 196, 178], CutPattern::Diagonal)
        }
    } else if has(&[
        "mortero",
        "mortar",
        "monocapa",
        "revoco",
        "render",
        "enfoscado",
    ]) {
        ("mortar", [222, 222, 214], CutPattern::Dots)
    } else if has(&[
        "yeso", "plaster", "gypsum", "pladur", "drywall", "placa", "enlucido",
    ]) {
        ("plaster", [245, 245, 245], CutPattern::Solid)
    } else if has(&["madera", "wood", "timber", "pino", "roble", "oak", "pine"]) {
        ("wood", [196, 160, 112], CutPattern::Solid)
    } else if has(&["acero", "steel", "metal", "alumin", "hierro", "iron"]) {
        ("steel", [150, 154, 160], CutPattern::Solid)
    } else if has(&[
        "grava", "gravel", "arena", "sand", "terreno", "soil", "earth",
    ]) {
        ("gravel", [210, 204, 190], CutPattern::Dots)
    } else {
        ("", [200, 200, 200], CutPattern::Solid)
    }
}

/// What a layer does, from its IFC category, ventilation flag or material.
fn layer_function(
    category: Option<&str>,
    ventilated: bool,
    material_category: &str,
) -> LayerFunction {
    match category.map(|c| c.to_ascii_lowercase()).as_deref() {
        Some("loadbearing") | Some("structure") | Some("structural") => {
            return LayerFunction::Structure
        }
        Some("substrate") => return LayerFunction::Substrate,
        Some("insulation") => return LayerFunction::Insulation,
        Some("airgap") | Some("air") => return LayerFunction::AirGap,
        Some("membrane") => return LayerFunction::Membrane,
        Some("finish") => return LayerFunction::Finish,
        _ => {}
    }
    if ventilated {
        return LayerFunction::AirGap;
    }
    match material_category {
        "insulation" => LayerFunction::Insulation,
        "air" => LayerFunction::AirGap,
        "membrane" => LayerFunction::Membrane,
        "plaster" | "ceramic" | "mortar" => LayerFunction::Finish,
        "concrete" | "brick" | "steel" | "wood" => LayerFunction::Structure,
        _ => LayerFunction::Substrate,
    }
}

/// A material layer set as read: layers in file order and how they sit.
struct LayerSet {
    /// (material, thickness, function) in file order.
    layers: Vec<(MaterialId, f64, LayerFunction)>,
    name: String,
    /// `Some((positive, offset))` when it came with a usage.
    usage: Option<(bool, f64)>,
    dropped: usize,
}

impl<'a> Importer<'a> {
    fn new(file: &'a StepFile, schema: String) -> Self {
        let mut report = ImportReport {
            schema,
            ..Default::default()
        };
        let source = file
            .header("FILE_NAME")
            .map(|h| {
                let pre = h.arg(4).as_str().unwrap_or("").trim().to_string();
                let app = h.arg(5).as_str().unwrap_or("").trim().to_string();
                if app.is_empty() || app == pre {
                    pre
                } else if pre.is_empty() {
                    app
                } else {
                    format!("{app} ({pre})")
                }
            })
            .unwrap_or_default();
        let own_file = source.starts_with("ArchiSim");
        report.source = source;
        let mut imp = Importer {
            file,
            geo: Geo {
                file,
                length: 1.0,
                angle: 1.0,
            },
            ix: Index::default(),
            model: Model::new("Proyecto importado"),
            report,
            own_file,
            storeys: HashMap::new(),
            levels: Vec::new(),
            materials: HashMap::new(),
            materials_by_key: HashMap::new(),
            wall_types: HashMap::new(),
            slab_types: HashMap::new(),
            opening_types: HashMap::new(),
            used_type_guids: HashSet::new(),
            type_names: HashSet::new(),
            guids: GuidPlan::default(),
            walls: HashMap::new(),
            member_types: HashMap::new(),
            sources: Vec::new(),
            roof_ids: HashSet::new(),
            stair_ids: HashSet::new(),
        };
        imp.index();
        imp
    }

    fn ent(&self, id: u64) -> Option<&'a Entity> {
        self.file.get(id)
    }

    fn deref(&self, v: &Value) -> Option<&'a Entity> {
        self.file.deref(v)
    }

    fn index(&mut self) {
        let mut ids: Vec<&u64> = self.file.entities.keys().collect();
        ids.sort_unstable();
        for &id in ids {
            let e = &self.file.entities[&id];
            let related = || e.arg(4).refs();
            match e.kind.as_str() {
                "IFCRELCONTAINEDINSPATIALSTRUCTURE" => {
                    if let Some(s) = e.arg(5).as_ref() {
                        for r in related() {
                            self.ix.container.entry(r).or_insert(s);
                        }
                    }
                }
                "IFCRELAGGREGATES" | "IFCRELNESTS" => {
                    if let Some(p) = e.arg(4).as_ref() {
                        for c in e.arg(5).refs() {
                            self.ix.parent.entry(c).or_insert(p);
                        }
                    }
                }
                "IFCRELVOIDSELEMENT" => {
                    if let (Some(w), Some(o)) = (e.arg(4).as_ref(), e.arg(5).as_ref()) {
                        self.ix.voids.entry(w).or_default().push(o);
                    }
                }
                "IFCRELFILLSELEMENT" => {
                    if let (Some(o), Some(f)) = (e.arg(4).as_ref(), e.arg(5).as_ref()) {
                        self.ix.fills.entry(o).or_insert(f);
                        self.ix.filled.entry(f).or_insert(o);
                    }
                }
                "IFCRELASSOCIATESMATERIAL" => {
                    if let Some(m) = e.arg(5).as_ref() {
                        for r in related() {
                            self.ix.material.entry(r).or_insert(m);
                        }
                    }
                }
                "IFCRELDEFINESBYTYPE" => {
                    if let Some(t) = e.arg(5).as_ref() {
                        for r in related() {
                            self.ix.type_of.entry(r).or_insert(t);
                        }
                    }
                }
                "IFCRELDEFINESBYPROPERTIES" => {
                    if let Some(p) = e.arg(5).as_ref() {
                        for r in related() {
                            self.ix.psets.entry(r).or_default().push(p);
                        }
                    }
                }
                "IFCMATERIALDEFINITIONREPRESENTATION" => {
                    if let Some(m) = e.arg(3).as_ref() {
                        if let Some(r) = e.arg(2).refs().first() {
                            self.ix.material_rep.entry(m).or_insert(*r);
                        }
                    }
                }
                "IFCMATERIALPROPERTIES" => {
                    if let Some(m) = e.arg(3).as_ref() {
                        self.ix.material_props.entry(m).or_default().push(id);
                    }
                }
                "IFCWALLTYPE" | "IFCSLABTYPE" | "IFCDOORTYPE" | "IFCWINDOWTYPE"
                | "IFCDOORSTYLE" | "IFCWINDOWSTYLE" => {}
                _ => {}
            }
        }
    }

    // ---- Units ------------------------------------------------------

    fn unit_factor(&self, v: &Value) -> Option<f64> {
        let u = self.deref(v)?;
        match u.kind.as_str() {
            "IFCSIUNIT" => {
                let prefix = match u.arg(2).as_enum() {
                    Some("EXA") => 1e18,
                    Some("PETA") => 1e15,
                    Some("TERA") => 1e12,
                    Some("GIGA") => 1e9,
                    Some("MEGA") => 1e6,
                    Some("KILO") => 1e3,
                    Some("HECTO") => 1e2,
                    Some("DECA") => 1e1,
                    Some("DECI") => 1e-1,
                    Some("CENTI") => 1e-2,
                    Some("MILLI") => 1e-3,
                    Some("MICRO") => 1e-6,
                    Some("NANO") => 1e-9,
                    _ => 1.0,
                };
                Some(prefix)
            }
            "IFCCONVERSIONBASEDUNIT" | "IFCCONVERSIONBASEDUNITWITHOFFSET" => {
                let m = self.deref(u.arg(3))?;
                let value = m.arg(0).as_f64()?;
                let base = self.unit_factor(m.arg(1)).unwrap_or(1.0);
                Some(value * base)
            }
            _ => None,
        }
    }

    fn read_units(&mut self) {
        let Some(&project) = self.file.of_kind("IFCPROJECT").first() else {
            self.report
                .warn("El fichero no tiene proyecto (IfcProject): se suponen metros.");
            return;
        };
        let Some(assignment) = self.ent(project).and_then(|p| self.deref(p.arg(8))) else {
            self.report
                .warn("El proyecto no declara unidades: se suponen metros.");
            return;
        };
        for u in assignment.arg(0).as_list() {
            let Some(unit) = self.deref(u) else { continue };
            let ty = match unit.kind.as_str() {
                "IFCSIUNIT" => unit.arg(1).as_enum(),
                "IFCCONVERSIONBASEDUNIT" | "IFCCONVERSIONBASEDUNITWITHOFFSET" => {
                    unit.arg(1).as_enum()
                }
                _ => None,
            };
            match ty {
                Some("LENGTHUNIT") => {
                    if let Some(f) = self.unit_factor(u) {
                        self.geo.length = f;
                    }
                }
                Some("PLANEANGLEUNIT") => {
                    if let Some(f) = self.unit_factor(u) {
                        self.geo.angle = f;
                    }
                }
                _ => {}
            }
        }
    }

    // ---- Properties -------------------------------------------------

    /// A single property value from the element's property sets, then
    /// from its type's.
    fn property(&self, element: u64, pset: &str, name: &str) -> Option<&'a Value> {
        let mut sets: Vec<u64> = self.ix.psets.get(&element).cloned().unwrap_or_default();
        if let Some(t) = self.ix.type_of.get(&element).and_then(|&t| self.ent(t)) {
            sets.extend(t.arg(5).refs());
        }
        for s in sets {
            let Some(set) = self.ent(s) else { continue };
            if set.kind != "IFCPROPERTYSET" || set.arg(2).as_str() != Some(pset) {
                continue;
            }
            for p in set.arg(4).as_list() {
                let Some(p) = self.deref(p) else { continue };
                if p.kind == "IFCPROPERTYSINGLEVALUE" && p.arg(0).as_str() == Some(name) {
                    return Some(p.arg(2));
                }
            }
        }
        None
    }

    /// A single property value from the element's own property sets only.
    fn property_own(&self, element: u64, pset: &str, name: &str) -> Option<&'a Value> {
        for s in self.ix.psets.get(&element).cloned().unwrap_or_default() {
            let Some(set) = self.ent(s) else { continue };
            if set.kind != "IFCPROPERTYSET" || set.arg(2).as_str() != Some(pset) {
                continue;
            }
            for p in set.arg(4).as_list() {
                let Some(p) = self.deref(p) else { continue };
                if p.kind == "IFCPROPERTYSINGLEVALUE" && p.arg(0).as_str() == Some(name) {
                    return Some(p.arg(2));
                }
            }
        }
        None
    }

    /// The `Status` of an element's common property set (phases of a
    /// refurbishment), whatever the set is called.
    fn status_of(&self, element: u64) -> Option<archisim_core::Phase> {
        for s in self.ix.psets.get(&element).cloned().unwrap_or_default() {
            let Some(set) = self.ent(s) else { continue };
            if set.kind != "IFCPROPERTYSET" {
                continue;
            }
            for p in set.arg(4).as_list() {
                let Some(p) = self.deref(p) else { continue };
                if p.kind == "IFCPROPERTYSINGLEVALUE" && p.arg(0).as_str() == Some("Status") {
                    if let Some(v) = p.arg(2).as_str() {
                        return archisim_core::Phase::from_ifc_status(v);
                    }
                }
            }
        }
        None
    }

    /// Gives every imported element the phase its file says.
    fn read_statuses(&mut self) {
        let mut found = 0;
        for (id, ifc) in std::mem::take(&mut self.sources) {
            if let Some(p) = self.status_of(ifc) {
                if p != archisim_core::Phase::New && self.model.set_own_phase(id, p).is_ok() {
                    found += 1;
                }
            }
        }
        if found > 0 {
            self.report.warn(format!(
                "{found} elementos existentes o a derribar (fases de reforma)."
            ));
        }
    }

    fn is_external(&self, element: u64, pset: &str) -> Option<bool> {
        self.property(element, pset, "IsExternal")
            .and_then(Value::as_bool)
    }

    // ---- Spatial structure -----------------------------------------

    fn read_project(&mut self) {
        if let Some(p) = self
            .file
            .of_kind("IFCPROJECT")
            .first()
            .and_then(|&p| self.ent(p))
        {
            let name = p
                .arg(2)
                .as_str()
                .filter(|s| !s.trim().is_empty())
                .or(p.arg(5).as_str())
                .unwrap_or("Proyecto importado");
            self.model.project_name = name.to_string();
            self.guids.project = p.arg(0).as_str().map(String::from);
        }
        let sites = self.file.of_kind("IFCSITE");
        if let Some(s) = sites.first().and_then(|&s| self.ent(s)) {
            self.guids.site = s.arg(0).as_str().map(String::from);
        }
        let buildings = self.file.of_kind("IFCBUILDING");
        if buildings.len() > 1 {
            self.report.warn(format!(
                "El fichero tiene {} edificios: sus plantas se reúnen en uno.",
                buildings.len()
            ));
        }
        if let Some(b) = buildings.first().and_then(|&b| self.ent(b)) {
            self.guids.building = b.arg(0).as_str().map(String::from);
        }

        let mut storeys: Vec<(f64, u64, String, String)> = Vec::new();
        for id in self.file.of_kind("IFCBUILDINGSTOREY") {
            let e = &self.file.entities[&id];
            let elevation = if matches!(e.arg(5), Value::Ref(_)) {
                self.geo.placement(e.arg(5)).origin[2]
            } else {
                e.arg(9).as_f64().unwrap_or(0.0) * self.geo.length
            };
            let name = e
                .arg(2)
                .as_str()
                .filter(|s| !s.trim().is_empty())
                .or(e.arg(7).as_str())
                .map(String::from)
                .unwrap_or_else(|| format!("Planta {}", storeys.len() + 1));
            storeys.push((
                snap(elevation),
                id,
                name,
                e.arg(0).as_str().unwrap_or("").to_string(),
            ));
        }
        storeys.sort_by(|a, b| a.0.total_cmp(&b.0).then(a.1.cmp(&b.1)));
        for (elevation, id, name, guid) in storeys {
            let level = self.model.add_level(name, elevation);
            self.storeys.insert(id, level);
            self.levels.push((level, elevation));
            self.guids.levels.push((level, guid));
            self.report.levels += 1;
        }
    }

    /// The level an element belongs to: its storey, through spaces and
    /// wholes it is part of, or else the storey under `z`.
    fn level_of(&mut self, element: u64, z: f64) -> LevelId {
        let mut cur = element;
        for _ in 0..32 {
            if let Some(&level) = self.storeys.get(&cur) {
                return level;
            }
            match self.ix.container.get(&cur).or(self.ix.parent.get(&cur)) {
                Some(&next) => cur = next,
                None => break,
            }
        }
        if self.levels.is_empty() {
            let level = self.model.add_level("Planta baja", 0.0);
            self.levels.push((level, 0.0));
            self.report.levels += 1;
            self.report
                .warn("El fichero no tiene plantas: se ha creado «Planta baja» a cota 0.");
        }
        self.levels
            .iter()
            .rev()
            .find(|(_, e)| *e <= z + 0.01)
            .or(self.levels.first())
            .map(|(l, _)| *l)
            .expect("there is at least one level")
    }

    fn elevation(&self, level: LevelId) -> f64 {
        self.model.level(level).map_or(0.0, |l| l.elevation)
    }

    // ---- Materials --------------------------------------------------

    fn colour(&self, material: u64) -> Option<([u8; 3], bool)> {
        let rep = self.ent(*self.ix.material_rep.get(&material)?)?;
        // IfcStyledRepresentation → IfcStyledItem → styles.
        for item in rep.arg(3).as_list() {
            let item = self.deref(item)?;
            let mut styles: Vec<&Entity> = Vec::new();
            for s in item.arg(1).as_list() {
                let Some(s) = self.deref(s) else { continue };
                if s.kind == "IFCPRESENTATIONSTYLEASSIGNMENT" {
                    styles.extend(s.arg(0).as_list().iter().filter_map(|x| self.deref(x)));
                } else {
                    styles.push(s);
                }
            }
            for style in styles {
                if style.kind != "IFCSURFACESTYLE" {
                    continue;
                }
                for sub in style.arg(2).as_list() {
                    let Some(sub) = self.deref(sub) else { continue };
                    if sub.kind.starts_with("IFCSURFACESTYLESHADING")
                        || sub.kind.starts_with("IFCSURFACESTYLERENDERING")
                    {
                        let c = self.deref(sub.arg(0))?;
                        let rgb = [1, 2, 3].map(|i| {
                            (c.arg(i).as_f64().unwrap_or(0.8).clamp(0.0, 1.0) * 255.0).round() as u8
                        });
                        let transparent = sub.arg(1).as_f64().unwrap_or(0.0) > 0.3;
                        return Some((rgb, transparent));
                    }
                }
            }
        }
        None
    }

    fn material(&mut self, ifc: u64) -> MaterialId {
        if let Some(&m) = self.materials.get(&ifc) {
            return m;
        }
        let e = self.ent(ifc);
        let name = e
            .and_then(|e| e.arg(0).as_str())
            .filter(|s| !s.trim().is_empty())
            .unwrap_or("Material sin nombre")
            .to_string();
        let declared = e
            .filter(|e| e.args.len() > 2)
            .and_then(|e| e.arg(2).as_str())
            .map(|s| s.to_lowercase())
            .filter(|s| !s.is_empty());
        let (guess_cat, guess_rgb, guess_pattern) = guess_material(&name);
        let category = declared.unwrap_or_else(|| guess_cat.to_string());
        let colour = self.colour(ifc);
        let rgb = colour.map_or(guess_rgb, |(c, _)| c);
        let category = if category.is_empty() && colour.is_some_and(|(_, t)| t) {
            "glass".to_string()
        } else {
            category
        };
        // Hatch written by ArchiSim, else guessed from the category.
        let mut pattern = None;
        for &p in self.ix.material_props.get(&ifc).into_iter().flatten() {
            let Some(props) = self.ent(p) else { continue };
            if props.arg(0).as_str() != Some("ArchiSim_Material") {
                continue;
            }
            for v in props.arg(2).as_list() {
                let Some(v) = self.deref(v) else { continue };
                if v.arg(0).as_str() == Some("CutPattern") {
                    pattern = v.arg(2).as_str().and_then(pattern_from_name);
                }
            }
        }
        let pattern = pattern.unwrap_or(match category.as_str() {
            "concrete" => CutPattern::Cross,
            "brick" | "masonry" | "block" => CutPattern::Diagonal,
            "insulation" => CutPattern::Insulation,
            "mortar" | "gravel" => CutPattern::Dots,
            "air" => CutPattern::Empty,
            _ => guess_pattern,
        });
        let id = self.material_named(&name, &category, rgb, pattern);
        self.materials.insert(ifc, id);
        id
    }

    /// A material by its properties, created once.
    fn material_named(
        &mut self,
        name: &str,
        category: &str,
        rgb: [u8; 3],
        pattern: CutPattern,
    ) -> MaterialId {
        let key = (
            name.to_string(),
            category.to_string(),
            rgb,
            pattern_code(pattern),
        );
        if let Some(&m) = self.materials_by_key.get(&key) {
            return m;
        }
        let id = self.model.add_material(name, category, rgb, pattern);
        self.materials_by_key.insert(key, id);
        self.report.materials += 1;
        id
    }

    fn fallback_material(&mut self, kind: &str) -> MaterialId {
        match kind {
            "glass" => self.material_named("Vidrio", "glass", [190, 222, 236], CutPattern::Solid),
            "wood" => self.material_named("Madera", "wood", [196, 160, 112], CutPattern::Solid),
            "frame" => self.material_named(
                "Carpintería metálica",
                "aluminium",
                [120, 124, 128],
                CutPattern::Solid,
            ),
            _ => self.material_named(
                "Material sin definir",
                "",
                [200, 200, 200],
                pattern_from_code(DEFAULT_PATTERN),
            ),
        }
    }

    /// The material definition of an object, or of its type.
    fn material_of(&self, element: u64) -> Option<u64> {
        self.ix.material.get(&element).copied().or_else(|| {
            self.ix
                .type_of
                .get(&element)
                .and_then(|t| self.ix.material.get(t))
                .copied()
        })
    }

    /// Layers of an element (`IfcMaterialLayerSetUsage` on the element,
    /// or a layer set on it or its type).
    fn layer_set(&mut self, element: u64) -> Option<LayerSet> {
        // The element's own definition, then its type's.
        let mut def = self.ix.material.get(&element).and_then(|&m| self.ent(m));
        let mut usage = None;
        let mut set = None;
        for _ in 0..2 {
            match def.map(|d| d.kind.as_str()) {
                Some("IFCMATERIALLAYERSETUSAGE") => {
                    let d = def.unwrap();
                    let positive = d.arg(2).as_enum() != Some("NEGATIVE");
                    let offset = d.arg(3).as_f64().unwrap_or(0.0) * self.geo.length;
                    usage = Some((positive, offset));
                    set = self.deref(d.arg(0));
                    break;
                }
                Some("IFCMATERIALLAYERSET") => {
                    set = def;
                    break;
                }
                _ => {
                    def = self
                        .ix
                        .type_of
                        .get(&element)
                        .and_then(|t| self.ix.material.get(t))
                        .and_then(|&m| self.ent(m));
                }
            }
        }
        let set = set?;
        let mut layers = Vec::new();
        let mut dropped = 0;
        for l in set.arg(0).as_list() {
            let Some(l) = self.deref(l) else { continue };
            let thickness = snap(l.arg(1).as_f64().unwrap_or(0.0) * self.geo.length);
            if thickness <= 1e-6 {
                dropped += 1;
                continue;
            }
            let material = match self.deref(l.arg(0)) {
                Some(_) => self.material(l.arg(0).as_ref().unwrap()),
                None => self.fallback_material(""),
            };
            let mat_category = self
                .model
                .material(material)
                .map(|m| m.category.clone())
                .unwrap_or_default();
            let category = (l.args.len() > 5).then(|| l.arg(5).as_str()).flatten();
            let ventilated = l.arg(2).as_bool() == Some(true);
            layers.push((
                material,
                thickness,
                layer_function(category, ventilated, &mat_category),
            ));
        }
        Some(LayerSet {
            layers,
            name: set.arg(1).as_str().unwrap_or("").to_string(),
            usage,
            dropped,
        })
    }

    /// A single material for an element without layers.
    fn single_material(&mut self, element: u64) -> MaterialId {
        let Some(def) = self.material_of(element).and_then(|d| self.ent(d)) else {
            return self.fallback_material("");
        };
        let first = match def.kind.as_str() {
            "IFCMATERIAL" => self.material_of(element),
            "IFCMATERIALLIST" => def.arg(0).refs().first().copied(),
            "IFCMATERIALCONSTITUENTSET" => def
                .arg(2)
                .as_list()
                .iter()
                .filter_map(|c| self.deref(c))
                .find_map(|c| c.arg(2).as_ref()),
            "IFCMATERIALLAYERSET" => self
                .deref(def.arg(0).as_list().first().unwrap_or(&Value::Null))
                .and_then(|l| l.arg(0).as_ref()),
            "IFCMATERIALLAYERSETUSAGE" => self
                .deref(def.arg(0))
                .and_then(|s| self.deref(s.arg(0).as_list().first().unwrap_or(&Value::Null)))
                .and_then(|l| l.arg(0).as_ref()),
            "IFCMATERIALPROFILESETUSAGE" | "IFCMATERIALPROFILESET" => None,
            _ => None,
        };
        match first {
            Some(m) => self.material(m),
            None => self.fallback_material(""),
        }
    }

    // ---- Shapes -----------------------------------------------------

    /// The axis (element coordinates: `Some(None)` if it is a curve
    /// ArchiSim cannot read) and the body (world) of a product.
    fn shape(&self, e: &Entity, xf: &Xf) -> (Option<Option<Vec<AxisSeg>>>, Body) {
        let mut body = Body::default();
        let mut axis = None;
        let Some(shape) = self.deref(e.arg(6)) else {
            return (axis, body);
        };
        for rep in shape.arg(2).as_list() {
            let Some(rep) = self.deref(rep) else { continue };
            let id = rep.arg(1).as_str().unwrap_or("");
            let ty = rep.arg(2).as_str().unwrap_or("");
            match id {
                "Axis" => {
                    if let Some(item) = rep.arg(3).as_list().first() {
                        axis = Some(self.geo.axis_segments(item).filter(|s| !s.is_empty()));
                    }
                }
                "Body" | ""
                    if !ty.starts_with("Curve") && ty != "Annotation2D" && ty != "BoundingBox" =>
                {
                    self.geo.items(rep.arg(3).as_list(), xf, &mut body);
                }
                _ => {}
            }
        }
        if body.is_empty() {
            // Last resort: a bounding box representation.
            for rep in shape.arg(2).as_list() {
                let Some(rep) = self.deref(rep) else { continue };
                if rep.arg(1).as_str() == Some("Box") {
                    self.geo.items(rep.arg(3).as_list(), xf, &mut body);
                }
            }
        }
        (axis, body)
    }

    // ---- Walls ------------------------------------------------------

    /// The walls of an IFC wall: one per piece of its axis (a straight
    /// line or a circular arc; a closed circle is two halves).
    fn read_wall(&mut self, id: u64) -> Result<Vec<WallCandidate>, &'static str> {
        let e = self.ent(id).ok_or("no existe")?;
        let xf = self.geo.placement(e.arg(5));
        let (axis, body) = self.shape(e, &xf);
        if body.is_empty() {
            return Err("sin geometría que ArchiSim sepa leer");
        }
        // (start, end, bulge, local Y of the start), in plan.
        let mut pieces: Vec<(Point2, Point2, f64, f64)> = Vec::new();
        match axis {
            Some(None) => return Err("eje de una curva que ArchiSim no sabe leer"),
            Some(Some(segs)) => {
                for s in segs {
                    let (pa, pb) = (p2(xf.apply(s.a)), p2(xf.apply(s.b)));
                    let bulge = s
                        .mid
                        .and_then(|m| Arc::bulge_through(pa, p2(xf.apply(m)), pb))
                        .unwrap_or(0.0);
                    // Straight pieces in line with the previous one join it.
                    if let Some(last) = pieces.last_mut() {
                        let d = last.1 - last.0;
                        let in_line = bulge == 0.0
                            && last.2 == 0.0
                            && last.1.approx_eq(pa)
                            && d.length() > 0.0
                            && ((pb - last.0).cross(d) / d.length()).abs() <= 1e-3
                            && (pb - last.0).dot(d) > 0.0;
                        if in_line {
                            last.1 = pb;
                            continue;
                        }
                    }
                    pieces.push((pa, pb, bulge, s.a[1]));
                }
            }
            None => {
                let u_hint = Point2::new(xf.x[0], xf.x[1]);
                let u = u_hint.normalized().ok_or("muro sin dirección en planta")?;
                let o = p2(xf.origin);
                let ts: Vec<f64> = body.all_points().map(|p| (p2(p) - o).dot(u)).collect();
                let t0 = ts.iter().copied().fold(f64::INFINITY, f64::min);
                let t1 = ts.iter().copied().fold(f64::NEG_INFINITY, f64::max);
                pieces.push((o + u * t0, o + u * t1, 0.0, 0.0));
            }
        }
        if pieces.is_empty() {
            return Err("muro sin eje");
        }
        if pieces.len() > 1 {
            self.report.warn(
                "Muros de varios tramos (o circulares cerrados): se importan como un muro por tramo, unidos; el primero conserva el GlobalId.",
            );
        }
        let n = pieces.len();
        pieces
            .into_iter()
            .enumerate()
            .map(|(k, (a, b, bulge, line_y))| {
                self.wall_piece(id, &xf, &body, a, b, bulge, line_y, k, n)
            })
            .collect()
    }

    /// One wall from a piece of an IFC wall's axis: its layers from the
    /// layer set (or the body), its type and level.
    #[allow(clippy::too_many_arguments)]
    fn wall_piece(
        &mut self,
        id: u64,
        xf: &Xf,
        body: &Body,
        mut a: Point2,
        mut b: Point2,
        bulge: f64,
        line_y: f64,
        piece: usize,
        pieces: usize,
    ) -> Result<WallCandidate, &'static str> {
        let e = self.ent(id).ok_or("no existe")?;
        let xf = *xf;
        let (lo, hi) = body.bounds().ok_or("sin geometría")?;
        a = snap2(a);
        b = snap2(b);
        let length = a.distance(b);
        if length < 1e-3 {
            return Err("muro de longitud nula");
        }
        let u = (b - a) * (1.0 / length);
        let n = u.perp_left();
        let base = snap(lo[2]);
        let top = snap(hi[2]);
        if top - base < 1e-3 {
            return Err("muro sin altura");
        }
        if body.clipped {
            self.report.warn(
                "Muros recortados (por cubiertas u otros elementos): se importan rectos, con su altura máxima.",
            );
        }
        // Across the axis, left positive (radially for a curved wall), of
        // the points of the body beside this piece.
        let arc = Arc::from_bulge(a, b, bulge);
        let beside = |p: Point2| -> Option<f64> {
            match &arc {
                Some(arc) => {
                    let (f, off) = arc.project(p);
                    let inside = arc.point(f, off).distance(p) < 1e-6;
                    (pieces == 1 || inside).then_some(off)
                }
                None => {
                    let t = (p - a).dot(u);
                    (pieces == 1 || (t > -1e-6 && t < length + 1e-6)).then_some((p - a).dot(n))
                }
            }
        };
        let across: Vec<f64> = body.all_points().filter_map(|p| beside(p2(p))).collect();
        if across.is_empty() {
            return Err("tramo de muro sin geometría junto a él");
        }
        let s_lo = across.iter().copied().fold(f64::INFINITY, f64::min);
        let s_hi = across.iter().copied().fold(f64::NEG_INFINITY, f64::max);

        let set = self.layer_set(id);
        let mut layers: Vec<(f64, f64, Layer)> = Vec::new();
        let mut type_name = String::new();
        if let Some(set) = set.filter(|s| !s.layers.is_empty()) {
            if set.dropped > 0 {
                self.report
                    .warn("Capas de espesor cero (membranas sin espesor) omitidas.");
            }
            type_name = set.name.clone();
            let total: f64 = set.layers.iter().map(|l| l.1).sum();
            // Local +Y of the element, in terms of the left normal n.
            let ly = Point2::new(xf.y[0], xf.y[1]);
            let sign = if ly.dot(n) < 0.0 { -1.0 } else { 1.0 };
            let (positive, offset) = match set.usage {
                // The offset is from the axis, wherever the axis lies in
                // the element's coordinates.
                Some((p, off)) => (p, off + line_y),
                None => {
                    // No usage: fit the layers to the body.
                    let fit_lo = if (s_hi - s_lo - total).abs() < 0.005 {
                        s_lo
                    } else {
                        (s_lo + s_hi - total) / 2.0
                    };
                    // In local coordinates, laid out towards +Y.
                    if sign > 0.0 {
                        (true, fit_lo + line_y)
                    } else {
                        (true, -(fit_lo + total) + line_y)
                    }
                }
            };
            let mut acc = 0.0;
            for &(material, thickness, function) in &set.layers {
                let (y0, y1) = if positive {
                    (offset + acc, offset + acc + thickness)
                } else {
                    (offset - acc - thickness, offset - acc)
                };
                acc += thickness;
                // Relative to the line a–b, along n.
                let (s0, s1) = (sign * (y0 - line_y), sign * (y1 - line_y));
                layers.push((
                    s0.min(s1),
                    s0.max(s1),
                    Layer {
                        material,
                        thickness,
                        function,
                    },
                ));
            }
        } else {
            let thickness = snap(s_hi - s_lo);
            if thickness < 1e-3 {
                return Err("muro sin espesor");
            }
            let material = self.single_material(id);
            layers.push((
                s_lo,
                s_hi,
                Layer {
                    material,
                    thickness,
                    function: LayerFunction::Structure,
                },
            ));
            self.report
                .warn("Muros sin capas de material: se importan con una sola capa de su espesor.");
        }
        let ifc_type = self.ix.type_of.get(&id).copied().filter(|t| {
            self.ent(*t)
                .is_some_and(|t| t.kind.starts_with("IFCWALLTYPE") || t.kind == "IFCWALLSTYLE")
        });
        if let Some(t) = ifc_type.and_then(|t| self.ent(t)) {
            if let Some(name) = t.arg(2).as_str().filter(|s| !s.trim().is_empty()) {
                type_name = name.to_string();
            }
        }
        if type_name.is_empty() {
            type_name = e
                .arg(4)
                .as_str()
                .filter(|s| !s.trim().is_empty())
                .map(String::from)
                .unwrap_or_default();
        }
        let level = self.level_of(id, base);
        // The first piece keeps the wall's GlobalId; the others get one made
        // from it, the same on every import.
        let base_guid = e.arg(0).as_str().unwrap_or("").to_string();
        let guid = if piece == 0 {
            base_guid
        } else {
            piece_guid(&base_guid, piece)
        };
        Ok(WallCandidate {
            ifc: id,
            guid,
            a,
            b,
            bulge,
            layers,
            base,
            top,
            level,
            external: self.is_external(id, "Pset_WallCommon").unwrap_or(false),
            type_key_name: type_name,
            ifc_type,
        })
    }

    fn import_walls(&mut self) {
        let mut candidates = Vec::new();
        for kind in ["IFCWALL", "IFCWALLSTANDARDCASE", "IFCWALLELEMENTEDCASE"] {
            for id in self.file.of_kind(kind) {
                match self.read_wall(id) {
                    Ok(c) => candidates.extend(c),
                    Err(why) => self.report.skip("muros", why),
                }
            }
        }
        candidates.sort_by_key(|c| c.ifc);

        // Middle of each level's walls, weighted by length: the inside.
        let mut middle: BTreeMap<LevelId, (Point2, f64)> = BTreeMap::new();
        for c in &candidates {
            let len = c.a.distance(c.b);
            let m = middle
                .entry(c.level)
                .or_insert((Point2::new(0.0, 0.0), 0.0));
            m.0 = m.0 + (c.a + c.b) * (0.5 * len);
            m.1 += len;
        }

        for mut c in candidates {
            let u = (c.b - c.a).normalized().unwrap_or(Point2::new(1.0, 0.0));
            let mut n = u.perp_left();
            // Sides: the exterior is on the left of the line (+n).
            if c.external && !self.own_file {
                if let Some(&(sum, total)) = middle.get(&c.level) {
                    let centre = sum * (1.0 / total.max(1e-9));
                    let mid = (c.a + c.b) * 0.5;
                    if (centre - mid).dot(n) > 1e-6 {
                        std::mem::swap(&mut c.a, &mut c.b);
                        c.bulge = -c.bulge;
                        n = -n;
                        for l in &mut c.layers {
                            let (lo, hi) = (l.0, l.1);
                            l.0 = -hi;
                            l.1 = -lo;
                        }
                    }
                }
            }
            c.layers.sort_by(|x, y| y.1.total_cmp(&x.1));
            let exterior = c.layers.first().map_or(0.0, |l| l.1);
            let interior = c.layers.last().map_or(0.0, |l| l.0);
            let thickness: f64 = c.layers.iter().map(|l| l.2.thickness).sum();
            let tol = 1e-5;
            let (reference, shift) = if exterior.abs() < tol {
                (ReferenceLine::ExteriorFace, 0.0)
            } else if interior.abs() < tol {
                (ReferenceLine::InteriorFace, 0.0)
            } else if (exterior + interior).abs() < tol {
                (ReferenceLine::Center, 0.0)
            } else {
                (ReferenceLine::Center, (exterior + interior) / 2.0)
            };
            let (a, b) = match Arc::from_bulge(c.a, c.b, c.bulge) {
                // The same arc moved across: concentric, same sweep.
                Some(arc) => (snap2(arc.point(0.0, shift)), snap2(arc.point(1.0, shift))),
                None => (snap2(c.a + n * shift), snap2(c.b + n * shift)),
            };
            let exterior_face = match reference {
                ReferenceLine::ExteriorFace => 0.0,
                ReferenceLine::InteriorFace => thickness,
                ReferenceLine::Center => thickness / 2.0,
            };
            let layers: Vec<Layer> = c.layers.into_iter().map(|l| l.2).collect();
            let wall_type = match self.wall_type(c.ifc_type, &c.type_key_name, c.external, layers) {
                Ok(t) => t,
                Err(e) => {
                    self.report
                        .skip("muros", format!("tipo no válido ({})", explain(&e)));
                    continue;
                }
            };
            let elevation = self.elevation(c.level);
            let spec = NewWall {
                wall_type,
                level: c.level,
                start: a,
                end: b,
                reference,
                height: snap(c.top - c.base),
                base_offset: snap(c.base - elevation),
                bulge: c.bulge,
                top: archisim_core::WallTop::Unconnected,
            };
            match self.model.add_wall(spec) {
                Ok(wid) => {
                    self.report.walls += 1;
                    self.guids.walls.push((wid, c.guid));
                    self.sources.push((wid.0, c.ifc));
                    let length = a.distance(b);
                    self.walls.entry(c.ifc).or_default().push(WallFrame {
                        id: wid,
                        level_elevation: elevation,
                        origin: a,
                        u: (b - a) * (1.0 / length),
                        n,
                        length,
                        base: c.base,
                        top: c.top,
                        exterior: exterior_face,
                        thickness,
                        curved: c.bulge != 0.0,
                    });
                }
                Err(e) => self.report.skip("muros", explain(&e)),
            }
        }
    }

    /// A unique type name: `name`, or `name (2)`, `name (3)`…
    fn unique_name(&mut self, name: &str) -> String {
        let mut candidate = name.to_string();
        let mut i = 2;
        while self.type_names.contains(&candidate) {
            candidate = format!("{name} ({i})");
            i += 1;
        }
        self.type_names.insert(candidate.clone());
        candidate
    }

    /// GUID of an IFC type for its first ArchiSim type; later variants get
    /// new ones.
    fn type_guid(&mut self, ifc_type: Option<u64>) -> Option<String> {
        let t = ifc_type?;
        if !self.used_type_guids.insert(t) {
            return None;
        }
        self.ent(t)
            .and_then(|e| e.arg(0).as_str())
            .map(String::from)
    }

    fn wall_type(
        &mut self,
        ifc_type: Option<u64>,
        name: &str,
        external: bool,
        layers: Vec<Layer>,
    ) -> Result<WallTypeId, ModelError> {
        let key = (ifc_type, external, layers_key(&layers));
        if let Some(&t) = self.wall_types.get(&key) {
            return Ok(t);
        }
        let thickness: f64 = layers.iter().map(|l| l.thickness).sum();
        let base = if name.trim().is_empty() {
            format!("Muro {:.0} mm", thickness * 1000.0)
        } else {
            name.to_string()
        };
        let name = self.unique_name(&base);
        let function = if external {
            WallFunction::Exterior
        } else {
            WallFunction::Interior
        };
        let id = self.model.add_wall_type(name, function, layers)?;
        if let Some(g) = self.type_guid(ifc_type) {
            self.guids.wall_types.push((id, g));
        }
        self.wall_types.insert(key, id);
        self.report.wall_types += 1;
        Ok(id)
    }

    // ---- Slabs ------------------------------------------------------

    fn import_slabs(&mut self) {
        let mut ids = Vec::new();
        for kind in ["IFCSLAB", "IFCSLABSTANDARDCASE", "IFCSLABELEMENTEDCASE"] {
            ids.extend(self.file.of_kind(kind));
        }
        ids.sort_unstable();
        for id in ids {
            if let Err(why) = self.import_slab(id) {
                self.report.skip("forjados", why);
            }
        }
    }

    fn import_slab(&mut self, id: u64) -> Result<(), String> {
        // The landings of an ArchiSim stair come back with their stair.
        if self.own_file {
            if let Some(&p) = self.ix.parent.get(&id) {
                if self.ent(p).is_some_and(|x| x.kind == "IFCSTAIR")
                    && self.property_own(p, "ArchiSim_Stair", "Path").is_some()
                {
                    return Ok(());
                }
            }
        }
        let e = self.ent(id).ok_or("no existe")?;
        let xf = self.geo.placement(e.arg(5));
        let (_, body) = self.shape(e, &xf);
        if body.is_empty() {
            return Err("sin geometría que ArchiSim sepa leer".into());
        }
        // The outline: the largest vertical extrusion, else the highest
        // horizontal face.
        // Inclined slabs: the plane of the top face (z = a·x + b·y + c).
        let mut tilt: Option<[f64; 3]> = None;
        let (outline, holes, bottom, top) = if let Some(x) = body
            .extrusions
            .iter()
            .max_by(|a, b| area(&a.outline).total_cmp(&area(&b.outline)))
        {
            (x.outline.clone(), x.holes.clone(), x.bottom, x.top)
        } else {
            let (lo, hi) = body.bounds().ok_or("sin geometría")?;
            match geometry::top_outline(&body, hi[2]) {
                Some((outline, holes)) => (outline, holes, lo[2], hi[2]),
                None => match geometry::sloped_top(&body) {
                    Some(t) => {
                        tilt = Some(t.plane);
                        // Heights where the slope arrow starts: the top
                        // and the bottom at the lowest corner.
                        let low = t
                            .outline
                            .iter()
                            .map(|p| t.plane[0] * p[0] + t.plane[1] * p[1] + t.plane[2])
                            .fold(f64::INFINITY, f64::min);
                        (t.outline, t.holes, low - t.vertical, low)
                    }
                    None => {
                        return Err(if body.polygons.is_empty() {
                            "sin contorno horizontal legible".into()
                        } else {
                            "forjado de forma libre (ni plano ni inclinado; aún no se importa)"
                                .into()
                        })
                    }
                },
            }
        };
        // Holes: the voids of the profile, and the openings that void the
        // slab (how Revit and ArchiCAD export shafts and stairs), by their
        // outline in plan.
        let mut hole_outlines: Vec<Vec<Point2>> = holes
            .iter()
            .map(|h| clean_outline(h))
            .filter(|h| h.len() >= 3)
            .collect();
        for void in self.ix.voids.get(&id).cloned().unwrap_or_default() {
            match self.void_outline(void) {
                Some(h) => hole_outlines.push(h),
                None => self.report.warn(
                    "Huecos de forjado de forma que ArchiSim no sabe leer: se importan cerrados.",
                ),
            }
        }
        let outline = clean_outline(&outline);
        if outline.len() < 3 {
            return Err("contorno sin superficie".into());
        }
        let top = snap(top);
        // Square to the face (the vertical height is more if it slopes).
        let stretch = tilt.map_or(1.0, |p| (1.0 + p[0] * p[0] + p[1] * p[1]).sqrt());
        let body_thickness = snap((top - bottom) / stretch);

        // Function: the slab's own, its type's, or a roof it is part of.
        let predefined = |v: &Value| match v.as_enum() {
            Some("FLOOR") | Some("LANDING") => Some(SlabFunction::Floor),
            Some("BASESLAB") => Some(SlabFunction::BaseSlab),
            Some("ROOF") => Some(SlabFunction::Roof),
            _ => None,
        };
        let ifc_type = self
            .ix
            .type_of
            .get(&id)
            .copied()
            .filter(|t| self.ent(*t).is_some_and(|t| t.kind == "IFCSLABTYPE"));
        let in_roof = self
            .ix
            .parent
            .get(&id)
            .and_then(|&p| self.ent(p))
            .is_some_and(|p| p.kind == "IFCROOF");
        let function = predefined(e.arg(8))
            .or_else(|| {
                ifc_type
                    .and_then(|t| self.ent(t))
                    .and_then(|t| predefined(t.arg(9)))
            })
            .unwrap_or(if in_roof {
                SlabFunction::Roof
            } else {
                SlabFunction::Floor
            });

        let set = self.layer_set(id);
        let mut type_name = String::new();
        let layers: Vec<Layer> = match set.filter(|s| !s.layers.is_empty()) {
            Some(set) => {
                if set.dropped > 0 {
                    self.report
                        .warn("Capas de espesor cero (membranas sin espesor) omitidas.");
                }
                type_name = set.name.clone();
                // Positive: written from the bottom up.
                let positive = set.usage.is_none_or(|(p, _)| p);
                let mut v: Vec<Layer> = set
                    .layers
                    .iter()
                    .map(|&(material, thickness, function)| Layer {
                        material,
                        thickness,
                        function,
                    })
                    .collect();
                if positive {
                    v.reverse();
                }
                let total: f64 = v.iter().map(|l| l.thickness).sum();
                if (total - body_thickness).abs() > 0.001 && !body.clipped {
                    self.report.warn(
                        "Forjados cuyo sólido no mide lo que suman sus capas: se usan las capas, con la cara superior del sólido.",
                    );
                }
                v
            }
            None => {
                if body_thickness < 1e-3 {
                    return Err("forjado sin espesor".into());
                }
                self.report.warn(
                    "Forjados sin capas de material: se importan con una sola capa de su espesor.",
                );
                vec![Layer {
                    material: self.single_material(id),
                    thickness: body_thickness,
                    function: LayerFunction::Structure,
                }]
            }
        };
        if let Some(t) = ifc_type.and_then(|t| self.ent(t)) {
            if let Some(name) = t.arg(2).as_str().filter(|s| !s.trim().is_empty()) {
                type_name = name.to_string();
            }
        }
        if type_name.is_empty() {
            type_name = e
                .arg(4)
                .as_str()
                .filter(|s| !s.trim().is_empty())
                .map(String::from)
                .unwrap_or_default();
        }
        let slab_type = self
            .slab_type(ifc_type, &type_name, function, layers)
            .map_err(|e| format!("tipo no válido ({})", explain(&e)))?;
        let level = self.level_of(id, top - 0.01);
        let elevation = self.elevation(level);
        let sid = self
            .model
            .add_slab(NewSlab {
                slab_type,
                level,
                outline,
                top_offset: snap(top - elevation),
            })
            .map_err(|e| explain(&e))?;
        if let Some(plane) = tilt {
            // The arrow up the slope, from the lowest corner across the
            // whole slab.
            let g = Point2::new(plane[0], plane[1]);
            let slope = g.length();
            let u = g * (1.0 / slope);
            let outline = &self
                .model
                .slab(sid)
                .map(|s| s.outline.clone())
                .unwrap_or_default();
            let along = |p: &Point2| p.dot(u);
            let (lo, hi) = outline
                .iter()
                .fold((f64::INFINITY, f64::NEG_INFINITY), |(a, b), p| {
                    (a.min(along(p)), b.max(along(p)))
                });
            let low = outline
                .iter()
                .min_by(|a, b| along(a).total_cmp(&along(b)))
                .copied()
                .unwrap_or_default();
            let from = snap2(low);
            let to = snap2(low + u * (hi - lo).max(0.1));
            let slope = archisim_core::SlabSlope {
                from,
                to,
                slope: (slope * 1e6).round() / 1e6,
            };
            if let Err(e) = self.model.set_slab_slope(sid, Some(slope)) {
                self.report.warn(format!(
                    "Forjado inclinado que no se puede inclinar ({}): se importa horizontal.",
                    explain(&e)
                ));
            }
        }
        if !hole_outlines.is_empty() {
            // All at once if they fit; else one by one, keeping those that do.
            if self
                .model
                .set_slab_holes(sid, hole_outlines.clone())
                .is_err()
            {
                let mut dropped = 0;
                for h in hole_outlines {
                    if self.model.add_slab_hole(sid, h).is_err() {
                        dropped += 1;
                    }
                }
                if dropped > 0 {
                    self.report.warn(
                        "Huecos de forjado que se salen del forjado o se pisan con otros: se importan cerrados.",
                    );
                }
            }
        }
        self.report.slabs += 1;
        self.sources.push((sid.0, id));
        self.guids
            .slabs
            .push((sid, e.arg(0).as_str().unwrap_or("").to_string()));
        Ok(())
    }

    /// The outline in plan of an opening element that voids a slab: its
    /// vertical extrusion, or the box of its body.
    fn void_outline(&self, void: u64) -> Option<Vec<Point2>> {
        let e = self.ent(void)?;
        let xf = self.geo.placement(e.arg(5));
        let (_, body) = self.shape(e, &xf);
        if let Some(x) = body
            .extrusions
            .iter()
            .max_by(|a, b| area(&a.outline).total_cmp(&area(&b.outline)))
        {
            let o = clean_outline(&x.outline);
            return (o.len() >= 3).then_some(o);
        }
        let (lo, hi) = body.bounds()?;
        if hi[0] - lo[0] < 1e-3 || hi[1] - lo[1] < 1e-3 {
            return None;
        }
        Some(vec![
            snap2(Point2::new(lo[0], lo[1])),
            snap2(Point2::new(hi[0], lo[1])),
            snap2(Point2::new(hi[0], hi[1])),
            snap2(Point2::new(lo[0], hi[1])),
        ])
    }

    fn slab_type(
        &mut self,
        ifc_type: Option<u64>,
        name: &str,
        function: SlabFunction,
        layers: Vec<Layer>,
    ) -> Result<SlabTypeId, ModelError> {
        let key = (ifc_type, function as u8, layers_key(&layers));
        if let Some(&t) = self.slab_types.get(&key) {
            return Ok(t);
        }
        let thickness: f64 = layers.iter().map(|l| l.thickness).sum();
        let base = if name.trim().is_empty() {
            format!("Forjado {:.0} mm", thickness * 1000.0)
        } else {
            name.to_string()
        };
        let name = self.unique_name(&base);
        let id = self.model.add_slab_type(name, function, layers)?;
        if let Some(g) = self.type_guid(ifc_type) {
            self.guids.slab_types.push((id, g));
        }
        self.slab_types.insert(key, id);
        self.report.slab_types += 1;
        Ok(id)
    }

    // ---- Doors and windows -------------------------------------------

    fn import_openings(&mut self) {
        let mut walls: Vec<u64> = self.walls.keys().copied().collect();
        walls.sort_unstable();
        let mut placed: HashSet<u64> = HashSet::new();
        for wall in walls {
            let voids = self.ix.voids.get(&wall).cloned().unwrap_or_default();
            for void in voids {
                let Some(&filling) = self.ix.fills.get(&void) else {
                    // An opening with nothing in it: a void ("hueco").
                    if let Err(why) = self.import_void(wall, void) {
                        self.report.skip("huecos", why);
                    }
                    continue;
                };
                placed.insert(filling);
                let what = self
                    .ent(filling)
                    .map_or("otros elementos", |f| class_name(&f.kind));
                if let Err(why) = self.import_opening(wall, void, filling) {
                    self.report.skip(what, why);
                }
            }
        }
        // Doors and windows not in an imported wall.
        for kind in [
            "IFCDOOR",
            "IFCDOORSTANDARDCASE",
            "IFCWINDOW",
            "IFCWINDOWSTANDARDCASE",
        ] {
            for id in self.file.of_kind(kind) {
                if placed.contains(&id) {
                    continue;
                }
                let host = self.ix.filled.get(&id).and_then(|o| {
                    self.ix
                        .voids
                        .iter()
                        .find(|(_, v)| v.contains(o))
                        .map(|(w, _)| *w)
                });
                let why = match host.and_then(|h| self.ent(h)) {
                    Some(h) if h.kind.starts_with("IFCWALL") => "su muro no se ha importado",
                    Some(h) if h.kind == "IFCCURTAINWALL" => "en un muro cortina",
                    Some(_) => "en un elemento que no es un muro",
                    None => "no está en ningún hueco de muro",
                };
                self.report.skip(class_name(kind), why);
            }
        }
    }

    /// The wall (piece of an IFC wall) an opening's body lies in: the
    /// straight one whose span holds its middle, nearest to its axis.
    fn frame_of(&self, wall: u64, body: &Body) -> Option<&WallFrame> {
        let frames = self.walls.get(&wall)?;
        let pts: Vec<Point2> = body.all_points().map(p2).collect();
        if pts.is_empty() {
            return frames.first();
        }
        let c = pts.iter().fold(Point2::new(0.0, 0.0), |s, p| s + *p) * (1.0 / pts.len() as f64);
        frames
            .iter()
            .filter(|f| !f.curved)
            .filter(|f| {
                let t = (c - f.origin).dot(f.u);
                t > -1e-3 && t < f.length + 1e-3
            })
            .min_by(|a, b| {
                let d = |f: &WallFrame| (c - f.origin).dot(f.n).abs();
                d(a).total_cmp(&d(b))
            })
            .or_else(|| frames.first())
    }

    /// The rough opening of an opening element in wall coordinates:
    /// along the wall and heights, clipped to the wall.
    fn rough_opening(&self, wall: u64, body: &Body) -> Result<(f64, f64, f64, f64), String> {
        let frame = self
            .frame_of(wall, body)
            .ok_or("su muro no se ha importado")?;
        let (mut t0, mut t1, mut z0, mut z1) = (
            f64::INFINITY,
            f64::NEG_INFINITY,
            f64::INFINITY,
            f64::NEG_INFINITY,
        );
        for p in body.all_points() {
            let t = (p2(p) - frame.origin).dot(frame.u);
            t0 = t0.min(t);
            t1 = t1.max(t);
            z0 = z0.min(p[2]);
            z1 = z1.max(p[2]);
        }
        let (t0, t1) = (snap(t0.max(0.0)), snap(t1.min(frame.length)));
        let (z0, z1) = (snap(z0.max(frame.base)), snap(z1.min(frame.top)));
        if snap(t1 - t0) < 0.05 || snap(z1 - z0) < 0.05 {
            return Err("hueco fuera del muro o demasiado pequeño".into());
        }
        Ok((t0, t1, z0, z1))
    }

    /// An opening element in a wall with no door or window in it: a void
    /// of the "Hueco" family, of its size (or of the type ArchiSim wrote).
    fn import_void(&mut self, wall: u64, void: u64) -> Result<(), String> {
        let ve = self.ent(void).ok_or("hueco inexistente")?;
        let (_, body) = self.shape(ve, &self.geo.placement(ve.arg(5)));
        if body.is_empty() {
            return Err("hueco sin geometría".into());
        }
        let (t0, t1, z0, z1) = self.rough_opening(wall, &body)?;
        let (width, height) = (snap(t1 - t0), snap(z1 - z0));
        let frame = self
            .frame_of(wall, &body)
            .ok_or("su muro no se ha importado")?;
        let (wall_id, level_elevation) = (frame.id, frame.level_elevation);
        let guid = ve.arg(0).as_str().unwrap_or("").to_string();
        let own_name = self
            .property(void, "ArchiSim_Void", "TypeName")
            .and_then(Value::as_str)
            .map(String::from);
        let own_guid = self
            .property(void, "ArchiSim_Void", "TypeGuid")
            .and_then(Value::as_str)
            .map(String::from);
        let name = own_name
            .clone()
            .unwrap_or_else(|| format!("Hueco {:.0}×{:.0}", width * 100.0, height * 100.0));
        // A void has no frame or leaf: any material of the file will do (a
        // new one only if there is none).
        let first = self.model.materials().next().map(|m| m.id);
        let any = match first {
            Some(m) => m,
            None => self.fallback_material("frame"),
        };
        let key = (
            None,
            format!("void {:?}", own_guid),
            [
                (width * 1e6).round() as i64,
                (height * 1e6).round() as i64,
                0,
                0,
            ],
            0,
            0,
        );
        let opening_type = match self.opening_types.get(&key) {
            Some(&t) => t,
            None => {
                let unique = self.unique_name(&name);
                let t = self
                    .model
                    .add_opening_type(NewOpeningType {
                        name: unique,
                        kind: OpeningKind::Void,
                        width,
                        height,
                        frame_width: 0.0,
                        frame_depth: 0.0,
                        frame_material: any,
                        panel_material: any,
                    })
                    .map_err(|e| format!("tipo no válido ({})", explain(&e)))?;
                if let Some(g) = own_guid {
                    self.guids.opening_types.push((t, g));
                }
                self.opening_types.insert(key, t);
                self.report.opening_types += 1;
                t
            }
        };
        let oid = self
            .model
            .add_opening(NewOpening {
                opening_type,
                wall: wall_id,
                along: snap((t0 + t1) / 2.0),
                sill: snap(z0 - level_elevation),
                opens_to: WallSide::Interior,
                hand: Hand::Left,
                setback: None,
            })
            .map_err(|e| explain(&e))?;
        self.report.voids += 1;
        self.sources.push((oid.0, void));
        self.guids.openings.push((oid, guid));
        Ok(())
    }

    fn import_opening(&mut self, wall: u64, void: u64, filling: u64) -> Result<(), String> {
        let fe = self.ent(filling).ok_or("no existe")?;
        let is_door = fe.kind.starts_with("IFCDOOR");
        if !is_door && !fe.kind.starts_with("IFCWINDOW") {
            return Err("el hueco lo llena algo que no es puerta ni ventana".into());
        }
        // The rough opening, from the opening element (else the door or
        // window itself), in wall coordinates.
        let ve = self.ent(void).ok_or("hueco inexistente")?;
        let (_, mut vbody) = self.shape(ve, &self.geo.placement(ve.arg(5)));
        let fxf = self.geo.placement(fe.arg(5));
        let (_, fbody) = self.shape(fe, &fxf);
        if vbody.is_empty() {
            vbody = fbody.clone();
        }
        if vbody.is_empty() {
            return Err("sin geometría para situarla".into());
        }
        let frame = self
            .frame_of(wall, &vbody)
            .ok_or("su muro no se ha importado")?;
        let (origin, n) = (frame.origin, frame.n);
        let wall_id = frame.id;
        let (exterior, thickness) = (frame.exterior, frame.thickness);
        let level_elevation = frame.level_elevation;
        let (t0, t1, z0, z1) = self.rough_opening(wall, &vbody)?;
        let (width, height) = (snap(t1 - t0), snap(z1 - z0));

        // Type, operation and hand.
        let ifc_type = self.ix.type_of.get(&filling).copied().filter(|t| {
            self.ent(*t).is_some_and(|t| {
                matches!(
                    t.kind.as_str(),
                    "IFCDOORTYPE" | "IFCDOORSTYLE" | "IFCWINDOWTYPE" | "IFCWINDOWSTYLE"
                )
            })
        });
        let te = ifc_type.and_then(|t| self.ent(t));
        let type_props: Vec<&Entity> = te
            .map(|t| {
                t.arg(5)
                    .as_list()
                    .iter()
                    .filter_map(|p| self.deref(p))
                    .collect()
            })
            .unwrap_or_default();
        let lining = type_props
            .iter()
            .find(|p| p.kind == "IFCDOORLININGPROPERTIES" || p.kind == "IFCWINDOWLININGPROPERTIES");
        let k = self.geo.length;
        let lining_depth = lining.and_then(|l| l.arg(4).as_f64()).map(|v| snap(v * k));
        let lining_thickness = lining.and_then(|l| l.arg(5).as_f64()).map(|v| snap(v * k));

        let mut hand = None;
        // How each window leaf opens, for the family's leaf settings.
        let mut leaf_ops: Vec<String> = Vec::new();
        let kind = if is_door {
            let schema4 = self.report.schema.starts_with("IFC4");
            let own = if schema4 { fe.arg(11).as_enum() } else { None };
            let from_type = te.and_then(|t| {
                if t.kind == "IFCDOORTYPE" {
                    t.arg(10).as_enum()
                } else {
                    t.arg(8).as_enum()
                }
            });
            let op = own
                .filter(|o| *o != "NOTDEFINED" && *o != "USERDEFINED")
                .or(from_type)
                .unwrap_or("NOTDEFINED");
            let (operation, h) = door_from_ifc(op);
            if op == "NOTDEFINED" || op == "USERDEFINED" {
                self.report.warn(
                    "Puertas sin tipo de apertura en el fichero: se importan abatibles de una hoja.",
                );
            }
            hand = h;
            OpeningKind::Door(operation)
        } else {
            let schema4 = self.report.schema.starts_with("IFC4");
            let own = if schema4 { fe.arg(11).as_enum() } else { None };
            let from_type = te.and_then(|t| {
                if t.kind == "IFCWINDOWTYPE" {
                    t.arg(10).as_enum()
                } else {
                    t.arg(9).as_enum()
                }
            });
            let partitioning = own
                .filter(|o| *o != "NOTDEFINED" && *o != "USERDEFINED")
                .or(from_type)
                .unwrap_or("SINGLE_PANEL");
            let panels: Vec<&str> = type_props
                .iter()
                .filter(|p| p.kind == "IFCWINDOWPANELPROPERTIES")
                .filter_map(|p| p.arg(4).as_enum())
                .collect();
            let two = !matches!(partitioning, "SINGLE_PANEL" | "NOTDEFINED" | "USERDEFINED");
            let sliding = panels.iter().any(|p| p.starts_with("SLIDING"));
            if !sliding && (1..=4).contains(&panels.len()) {
                leaf_ops = panels
                    .iter()
                    .map(|p| leaf_operation(p).to_string())
                    .collect();
            } else if partitioning.starts_with("TRIPLE") || panels.len() > 4 {
                self.report.warn(
                    "Ventanas de tres o más hojas sin el funcionamiento de cada hoja (o de más de cuatro): se importan con dos.",
                );
            }
            let op = if sliding {
                WindowOperation::Sliding
            } else if !panels.is_empty() && panels.iter().all(|p| *p == "FIXEDCASEMENT") {
                WindowOperation::Fixed
            } else if two || panels.len() > 1 {
                WindowOperation::DoubleCasement
            } else {
                WindowOperation::SingleCasement
            };
            if op == WindowOperation::SingleCasement {
                hand = match panels.first().copied() {
                    Some(p) if p.ends_with("RIGHTHAND") => Some(Hand::Left),
                    Some(p) if p.ends_with("LEFTHAND") => Some(Hand::Right),
                    _ => None,
                };
            }
            OpeningKind::Window(op)
        };
        // Written by ArchiSim for every door and window.
        if let Some(h) = self
            .property(filling, "ArchiSim_Opening", "Hand")
            .and_then(Value::as_str)
        {
            hand = match h {
                "Left" => Some(Hand::Left),
                "Right" => Some(Hand::Right),
                _ => hand,
            };
        }
        let hand = hand.unwrap_or(Hand::Left);

        // Frame: from the lining properties, else usual sizes.
        let (default_width, default_depth) = if is_door { (0.05, 0.07) } else { (0.06, 0.07) };
        if lining_thickness.is_none() || lining_depth.is_none() {
            self.report.warn(
                "Puertas o ventanas sin medidas de marco en el fichero: se usan 5–6 cm de ancho y 7 cm de fondo.",
            );
        }
        let mut frame_width = lining_thickness
            .filter(|v| *v > 0.0)
            .unwrap_or(default_width);
        let max_frame = (width.min(height) / 2.0 - 0.01).max(0.005);
        if frame_width > max_frame {
            frame_width = snap(max_frame);
        }
        let mut frame_depth = lining_depth.filter(|v| *v > 0.0).unwrap_or(default_depth);
        if frame_depth > thickness {
            frame_depth = thickness;
        }

        // Materials of the type: frame and leaf or glazing.
        let (frame_material, panel_material) = self.opening_materials(filling, is_door);

        // Where the frame sits across the wall, and the side it opens to.
        let across: Vec<f64> = fbody
            .all_points()
            .map(|p| (p2(p) - origin).dot(n))
            .collect();
        let mid = if across.is_empty() {
            (p2(fxf.origin) - origin).dot(n)
        } else {
            let lo = across.iter().copied().fold(f64::INFINITY, f64::min);
            let hi = across.iter().copied().fold(f64::NEG_INFINITY, f64::max);
            (lo + hi) / 2.0
        };
        let ext_face = exterior; // offset of the exterior face
        let setback = snap(ext_face - (mid + frame_depth / 2.0));
        let centred = (thickness - frame_depth) / 2.0;
        let setback = if (setback - centred).abs() < 1e-4 {
            None
        } else {
            Some(setback.clamp(0.0, snap(thickness - frame_depth)))
        };
        let facing = Point2::new(fxf.y[0], fxf.y[1]).dot(n);
        let opens_to = if facing > 0.0 {
            WallSide::Exterior
        } else {
            WallSide::Interior
        };

        let type_name = te
            .and_then(|t| t.arg(2).as_str())
            .filter(|s| !s.trim().is_empty())
            .or(fe.arg(4).as_str().filter(|s| !s.trim().is_empty()))
            .map(String::from)
            .unwrap_or_else(|| {
                format!(
                    "{} {:.0}×{:.0}",
                    if is_door { "Puerta" } else { "Ventana" },
                    width * 100.0,
                    height * 100.0
                )
            });
        let family_data = te.and_then(|t| self.family_data(t, &type_props));
        let opening_type = self
            .opening_type(
                ifc_type,
                &type_name,
                family_data,
                NewOpeningType {
                    name: String::new(),
                    kind,
                    width,
                    height,
                    frame_width,
                    frame_depth,
                    frame_material,
                    panel_material,
                },
                &leaf_ops,
            )
            .map_err(|e| format!("tipo no válido ({})", explain(&e)))?;
        let oid = self
            .model
            .add_opening(NewOpening {
                opening_type,
                wall: wall_id,
                along: snap((t0 + t1) / 2.0),
                sill: snap(z0 - level_elevation),
                opens_to,
                hand,
                setback,
            })
            .map_err(|e| explain(&e))?;
        self.instance_values(filling, oid);
        self.sources.push((oid.0, filling));
        if is_door {
            self.report.doors += 1;
        } else {
            self.report.windows += 1;
        }
        self.guids
            .openings
            .push((oid, fe.arg(0).as_str().unwrap_or("").to_string()));
        Ok(())
    }

    fn opening_materials(&mut self, filling: u64, is_door: bool) -> (MaterialId, MaterialId) {
        let mut frame = None;
        let mut panel = None;
        if let Some(def) = self.material_of(filling).and_then(|d| self.ent(d)) {
            match def.kind.as_str() {
                "IFCMATERIALCONSTITUENTSET" => {
                    let parts: Vec<(String, u64)> = def
                        .arg(2)
                        .as_list()
                        .iter()
                        .filter_map(|c| self.deref(c))
                        .filter_map(|c| {
                            Some((
                                c.arg(0).as_str().unwrap_or("").to_lowercase(),
                                c.arg(2).as_ref()?,
                            ))
                        })
                        .collect();
                    for (name, m) in &parts {
                        if ["lining", "framing", "frame", "marco", "premarco"]
                            .iter()
                            .any(|w| name.contains(w))
                        {
                            frame = frame.or(Some(*m));
                        } else if ["panel", "glazing", "glass", "hoja", "vidrio", "leaf"]
                            .iter()
                            .any(|w| name.contains(w))
                        {
                            panel = panel.or(Some(*m));
                        }
                    }
                    frame = frame.or(parts.first().map(|p| p.1));
                    panel = panel.or(parts.get(1).map(|p| p.1)).or(frame);
                }
                "IFCMATERIALLIST" => {
                    let ms = def.arg(0).refs();
                    frame = ms.first().copied();
                    panel = ms.get(1).copied().or(frame);
                }
                "IFCMATERIAL" => {
                    frame = self.material_of(filling);
                    panel = if is_door { frame } else { None };
                }
                _ => {}
            }
        }
        let frame = match frame {
            Some(m) => self.material(m),
            None => self.fallback_material(if is_door { "wood" } else { "frame" }),
        };
        let panel = match panel {
            Some(m) => self.material(m),
            None if is_door => frame,
            None => self.fallback_material("glass"),
        };
        (frame, panel)
    }

    fn opening_type(
        &mut self,
        ifc_type: Option<u64>,
        name: &str,
        family_data: Option<FamilyData>,
        spec: NewOpeningType,
        leaf_ops: &[String],
    ) -> Result<OpeningTypeId, ModelError> {
        let micro = |v: f64| (v * 1e6).round() as i64;
        let kind = format!("{:?} {leaf_ops:?}", spec.kind);
        let key = (
            ifc_type,
            kind,
            [
                micro(spec.width),
                micro(spec.height),
                micro(spec.frame_width),
                micro(spec.frame_depth),
            ],
            spec.frame_material.0,
            spec.panel_material.0,
        );
        if let Some(&t) = self.opening_types.get(&key) {
            return Ok(t);
        }
        let name = self.unique_name(name);
        // Written by ArchiSim: the very same family, values and mark.
        if let Some(data) = family_data {
            match self.family_type(&name, data) {
                Ok(id) => {
                    if let Some(g) = self.type_guid(ifc_type) {
                        self.guids.opening_types.push((id, g));
                    }
                    self.opening_types.insert(key, id);
                    self.report.opening_types += 1;
                    return Ok(id);
                }
                Err(why) => self.report.warn(format!(
                    "El tipo «{name}» trae su familia de ArchiSim pero no se puede rehacer ({why}): se importa por sus medidas."
                )),
            }
        }
        let id = self
            .model
            .add_opening_type(NewOpeningType { name, ..spec })?;
        // A window family with one setting per leaf ("Ventana practicable"):
        // as many leaves as the file has, each opening as it says.
        if let Some(t) = self.model.opening_type(id).cloned() {
            let per_leaf =
                (1..=leaf_ops.len()).all(|i| t.values.contains_key(&format!("leaf_{i}")));
            if !leaf_ops.is_empty() && per_leaf {
                let mut values = t.values.clone();
                values.insert(
                    "leaves".into(),
                    archisim_core::Value::Number(leaf_ops.len() as f64),
                );
                for (i, op) in leaf_ops.iter().enumerate() {
                    values.insert(
                        format!("leaf_{}", i + 1),
                        archisim_core::Value::Text(op.clone()),
                    );
                }
                // Too many leaves for the frame: keep what the file allows.
                if self.model.edit_family_type(id, t.family, values).is_err() {
                    self.report.warn(
                        "Alguna ventana tiene más hojas de las que caben con su marco: se importa con menos.",
                    );
                }
            }
        }
        if let Some(g) = self.type_guid(ifc_type) {
            self.guids.opening_types.push((id, g));
        }
        self.opening_types.insert(key, id);
        self.report.opening_types += 1;
        Ok(id)
    }

    /// ArchiSim's family data of a door or window type (`ArchiSim_Family`
    /// and `ArchiSim_FamilyValues`), with its mark (`Tag`) and description.
    fn family_data(&self, te: &'a Entity, type_props: &[&'a Entity]) -> Option<FamilyData> {
        let set = |name: &str| {
            type_props
                .iter()
                .find(|p| p.kind == "IFCPROPERTYSET" && p.arg(2).as_str() == Some(name))
                .copied()
        };
        let props = |set: &'a Entity| -> Vec<(String, &'a Value)> {
            set.arg(4)
                .as_list()
                .iter()
                .filter_map(|p| self.deref(p))
                .filter(|p| p.kind == "IFCPROPERTYSINGLEVALUE")
                .filter_map(|p| Some((p.arg(0).as_str()?.to_string(), p.arg(2))))
                .collect()
        };
        let family = props(set("ArchiSim_Family")?);
        let text = |key: &str| {
            family
                .iter()
                .find(|(k, _)| k == key)
                .and_then(|(_, v)| v.as_str())
                .map(str::to_string)
        };
        let values = set("ArchiSim_FamilyValues")
            .map(|s| props(s).into_iter().map(|(k, v)| (k, v.clone())).collect())
            .unwrap_or_default();
        Some(FamilyData {
            source: text("Source")?,
            values,
            mark: te.arg(7).as_str().map(str::to_string),
            description: te.arg(3).as_str().map(str::to_string),
        })
    }

    /// Instance parameters written by ArchiSim (`ArchiSim_FamilyValues` on
    /// the door or window itself), for families that have them.
    fn instance_values(&mut self, filling: u64, oid: archisim_core::OpeningId) {
        use archisim_core::{ParamKind, Value as V};
        let Some(o) = self.model.opening(oid) else {
            return;
        };
        let Some(t) = self.model.opening_type(o.opening_type) else {
            return;
        };
        let Some(def) = self.model.family(t.family).map(|f| f.def().clone()) else {
            return;
        };
        let mut values = o.values.clone();
        let mut changed = false;
        for p in def.params_of(archisim_core::ParamScope::Instance) {
            let Some(raw) = self.property_own(filling, "ArchiSim_FamilyValues", &p.name) else {
                continue;
            };
            let value = match &p.kind {
                ParamKind::Bool => raw.as_bool().map(V::Bool),
                ParamKind::Length => raw.as_f64().map(|x| V::Number(snap(x * self.geo.length))),
                k if k.is_numeric() => raw.as_f64().map(V::Number),
                ParamKind::Material => raw
                    .as_str()
                    .and_then(|n| self.model.materials().find(|m| m.name == n))
                    .map(|m| V::Material(m.id.0)),
                _ => raw.as_str().map(|t| V::Text(t.to_string())),
            };
            if let Some(v) = value {
                values.insert(p.name.clone(), v);
                changed = true;
            }
        }
        if changed && self.model.set_opening_values(oid, values).is_err() {
            self.report
                .warn("Algún parámetro de ejemplar de una puerta o ventana no es válido: se deja el de la familia.");
        }
    }

    /// Rebuilds a door or window type from ArchiSim's family data: the
    /// family (loaded from its recipe if the project does not have it), its
    /// values (materials by name) and its mark.
    fn family_type(&mut self, name: &str, data: FamilyData) -> Result<OpeningTypeId, String> {
        use archisim_core::{Family, FamilyId, ParamKind, Value as V};
        let parsed = Family::new(FamilyId(1), data.source.clone()).map_err(|e| e.to_string())?;
        let family = match self.model.family_by_name(parsed.name()) {
            Some(f) if f.source == parsed.source => f.id,
            Some(_) => {
                return Err(format!(
                    "el proyecto ya tiene otra versión de la familia «{}»",
                    parsed.name()
                ))
            }
            None => self
                .model
                .add_family(&data.source)
                .map_err(|e| explain(&e))?,
        };
        let def = self
            .model
            .family(family)
            .ok_or("familia perdida")?
            .def()
            .clone();
        let mut values = self
            .model
            .family_defaults(family, archisim_core::ParamScope::Type)
            .map_err(|e| explain(&e))?;
        for (key, raw) in &data.values {
            let Some(p) = def.param(key) else {
                continue;
            };
            let value = match &p.kind {
                ParamKind::Material => {
                    let wanted = raw.as_str().unwrap_or_default();
                    let m = self
                        .model
                        .materials()
                        .find(|m| m.name == wanted)
                        .map(|m| m.id)
                        .ok_or_else(|| format!("no hay material «{wanted}»"))?;
                    V::Material(m.0)
                }
                ParamKind::Bool => V::Bool(raw.as_bool().ok_or("valor sí/no ilegible")?),
                k if k.is_numeric() => {
                    let x = raw.as_f64().ok_or("número ilegible")?;
                    // Lengths come in the file's unit.
                    let x = if *k == ParamKind::Length {
                        snap(x * self.geo.length)
                    } else {
                        x
                    };
                    V::Number(x)
                }
                _ => V::Text(raw.as_str().unwrap_or_default().to_string()),
            };
            values.insert(key.clone(), value);
        }
        let mark = data
            .mark
            .filter(|m| self.model.opening_types().all(|t| &t.mark != m));
        self.model
            .add_family_type(archisim_core::NewFamilyType {
                family,
                name: name.to_string(),
                mark,
                description: data.description.unwrap_or_default(),
                values,
            })
            .map_err(|e| explain(&e))
    }

    // ---- Everything else ---------------------------------------------

    fn count_unsupported(&mut self) {
        for kind in UNSUPPORTED_CLASSES {
            let n = self
                .file
                .of_kind(kind)
                .into_iter()
                .filter(|id| !self.stair_ids.contains(id))
                .count();
            for _ in 0..n {
                self.report.skip(
                    class_name(kind),
                    "ArchiSim aún no tiene esta clase de elemento",
                );
            }
        }
        // Parts of stairs or curtain walls are counted with them; loose
        // ones are counted alone.
        for kind in PART_CLASSES {
            for id in self.file.of_kind(kind) {
                let in_whole = self.ix.parent.contains_key(&id);
                if !in_whole {
                    self.report.skip(
                        class_name(kind),
                        "ArchiSim aún no tiene esta clase de elemento",
                    );
                }
            }
        }
        for id in self.file.of_kind("IFCROOF") {
            let has_slabs = self.ix.parent.values().any(|&p| p == id);
            if !has_slabs && !self.roof_ids.contains(&id) {
                self.report.skip(
                    class_name("IFCROOF"),
                    "cubierta sin forjados (las cubiertas inclinadas llegarán más adelante)",
                );
            }
        }
    }

    fn run(&mut self) {
        self.read_units();
        if (self.geo.length - 1.0).abs() > 1e-12 {
            let unit = match self.geo.length {
                l if (l - 0.001).abs() < 1e-12 => "milímetros".to_string(),
                l if (l - 0.01).abs() < 1e-12 => "centímetros".to_string(),
                l if (l - 0.3048).abs() < 1e-9 => "pies".to_string(),
                l if (l - 0.0254).abs() < 1e-9 => "pulgadas".to_string(),
                l => format!("unidades de {l} m"),
            };
            self.report.warn(format!(
                "Longitudes del fichero en {unit}: convertidas a metros."
            ));
        }
        self.read_project();
        // Every material, in the file's order, used or not.
        for id in self.file.of_kind("IFCMATERIAL") {
            self.material(id);
        }
        self.import_walls();
        self.import_slabs();
        self.import_members();
        self.import_openings();
        self.import_spaces();
        self.import_roofs();
        self.import_stairs();
        self.read_statuses();
        self.count_unsupported();
    }

    /// Restores the file's GUIDs and checks the whole model once more.
    fn finish(mut self) -> Result<Imported, ImportError> {
        let mut parts = self.model.to_parts();
        let mut seen: HashSet<String> = HashSet::new();
        let mut duplicates = 0;
        let mut invalid = 0;
        let mut take = |text: &Option<String>, current: &mut GlobalId| {
            let Some(text) = text else { return };
            match GlobalId::parse(text) {
                Some(g) if seen.insert(text.clone()) => *current = g,
                Some(_) => duplicates += 1,
                None if !text.is_empty() => invalid += 1,
                None => {}
            }
        };
        take(&self.guids.project, &mut parts.project_guid);
        take(&self.guids.site, &mut parts.site_guid);
        take(&self.guids.building, &mut parts.building_guid);
        macro_rules! restore {
            ($list:ident, $plan:ident) => {
                let plan: HashMap<_, _> = self.guids.$plan.iter().cloned().collect();
                for item in &mut parts.$list {
                    if let Some(g) = plan.get(&item.id) {
                        take(&Some(g.clone()), &mut item.guid);
                    }
                }
            };
        }
        restore!(levels, levels);
        restore!(wall_types, wall_types);
        restore!(walls, walls);
        restore!(slab_types, slab_types);
        restore!(slabs, slabs);
        restore!(opening_types, opening_types);
        restore!(openings, openings);
        restore!(member_types, member_types);
        restore!(columns, columns);
        restore!(beams, beams);
        restore!(spaces, spaces);
        restore!(roofs, roofs);
        restore!(stairs, stairs);
        if duplicates > 0 {
            self.report.warn(format!(
                "{duplicates} GlobalId repetidos en el fichero: esos elementos reciben uno nuevo."
            ));
        }
        if invalid > 0 {
            self.report.warn(format!(
                "{invalid} GlobalId no válidos en el fichero: esos elementos reciben uno nuevo."
            ));
        }
        let model = Model::from_parts(parts).map_err(ImportError::Model)?;
        Ok(Imported {
            model,
            report: self.report,
        })
    }
}

/// Why the model refused an element, for the report.
fn explain(e: &ModelError) -> String {
    match e {
        ModelError::OpeningOutsideWall => "no cabe en su muro".into(),
        ModelError::OverlappingOpenings => {
            "se solapa con otra puerta o ventana del mismo muro".into()
        }
        ModelError::FrameTooDeep => "el marco no cabe en el espesor del muro".into(),
        ModelError::InvalidOutline => "contorno que se corta a sí mismo o sin superficie".into(),
        ModelError::ZeroLengthWall => "muro de longitud nula".into(),
        ModelError::ArcTooTight => "muro curvo de radio menor que su espesor".into(),
        ModelError::NonPositive(what) => format!("medida nula o negativa ({what})"),
        ModelError::NotFinite(what) => format!("número no válido ({what})"),
        ModelError::EmptyType => "tipo sin capas".into(),
        other => format!("{other}"),
    }
}

fn pattern_from_code(c: u8) -> CutPattern {
    match c {
        1 => CutPattern::Diagonal,
        2 => CutPattern::Cross,
        3 => CutPattern::Insulation,
        4 => CutPattern::Dots,
        5 => CutPattern::Empty,
        _ => CutPattern::Solid,
    }
}

/// `IfcDoorTypeOperationEnum` → operation and hand. IFC reads left and
/// right looking along the door's +Y (see the exporter), the opposite of
/// UNE-EN 12519.
fn door_from_ifc(op: &str) -> (DoorOperation, Option<Hand>) {
    match op {
        "SINGLE_SWING_LEFT" => (DoorOperation::SingleSwing, Some(Hand::Right)),
        "SINGLE_SWING_RIGHT" => (DoorOperation::SingleSwing, Some(Hand::Left)),
        "DOUBLE_DOOR_SINGLE_SWING"
        | "DOUBLE_DOOR_DOUBLE_SWING"
        | "DOUBLE_SWING_LEFT"
        | "DOUBLE_SWING_RIGHT"
        | "DOUBLE_DOOR_SINGLE_SWING_OPPOSITE_LEFT"
        | "DOUBLE_DOOR_SINGLE_SWING_OPPOSITE_RIGHT" => (DoorOperation::DoubleSwing, None),
        "SLIDING_TO_LEFT" => (DoorOperation::Sliding, Some(Hand::Right)),
        "SLIDING_TO_RIGHT" => (DoorOperation::Sliding, Some(Hand::Left)),
        "DOUBLE_DOOR_SLIDING" => (DoorOperation::Sliding, None),
        _ => (DoorOperation::SingleSwing, None),
    }
}

fn area(poly: &[[f64; 2]]) -> f64 {
    let pts: Vec<Point2> = poly.iter().map(|p| Point2::new(p[0], p[1])).collect();
    signed_area(&pts).abs()
}

/// Snaps an outline to the micrometre and drops repeated and collinear
/// points.
/// A GlobalId made from another one and a number, the same every time.
fn piece_guid(guid: &str, piece: usize) -> String {
    let mut h: u128 = 0x6c62272e07bb014262b821756295c58d;
    for b in guid
        .bytes()
        .chain([0u8])
        .chain(format!("tramo {piece}").bytes())
    {
        h ^= b as u128;
        h = h.wrapping_mul(0x0000000001000000000000000000013B);
    }
    GlobalId::from_u128(h).to_string()
}

fn clean_outline(poly: &[[f64; 2]]) -> Vec<Point2> {
    let mut pts: Vec<Point2> = Vec::new();
    for p in poly {
        let q = snap2(Point2::new(p[0], p[1]));
        if pts.last().is_none_or(|l: &Point2| l.distance(q) > 1e-6) {
            pts.push(q);
        }
    }
    while pts.len() > 1 && pts[0].distance(pts[pts.len() - 1]) <= 1e-6 {
        pts.pop();
    }
    // Collinear points.
    let mut changed = true;
    while changed && pts.len() > 3 {
        changed = false;
        for i in 0..pts.len() {
            let a = pts[(i + pts.len() - 1) % pts.len()];
            let b = pts[i];
            let c = pts[(i + 1) % pts.len()];
            let ab = b - a;
            let bc = c - b;
            if ab.cross(bc).abs() < 1e-9 * ab.length().max(bc.length()) && ab.dot(bc) > 0.0 {
                pts.remove(i);
                changed = true;
                break;
            }
        }
    }
    pts
}

#[cfg(test)]
mod tests;

/// The leaf setting of the "Ventana practicable" family for an IFC window
/// panel operation (`IfcWindowPanelOperationEnum`).
fn leaf_operation(op: &str) -> &'static str {
    match op {
        "FIXEDCASEMENT" => "fija",
        "TILTANDTURNLEFTHAND" | "TILTANDTURNRIGHTHAND" => "oscilobatiente",
        "BOTTOMHUNG" => "abatible",
        "TOPHUNG" => "proyectante",
        "PIVOTHORIZONTAL" => "basculante",
        _ => "practicable",
    }
}

/// A door or window type's family data, as ArchiSim writes it.
struct FamilyData {
    source: String,
    values: Vec<(String, Value)>,
    mark: Option<String>,
    description: Option<String>,
}
