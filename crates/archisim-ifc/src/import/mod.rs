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
//! What it leaves out (and counts in the report): curved and polygonal
//! walls, sloped slabs, holes in slabs, empty openings, and every other
//! class (columns, beams, stairs, roofs that are not slabs…).

mod compare;
pub mod geometry;
pub mod report;

use std::collections::{BTreeMap, HashMap, HashSet};
use std::fmt;

use archisim_core::{
    signed_area, CutPattern, DoorOperation, GlobalId, Hand, Layer, LayerFunction, LevelId,
    MaterialId, Model, ModelError, NewOpening, NewOpeningType, NewSlab, NewWall, OpeningKind,
    OpeningTypeId, Point2, ReferenceLine, RestoreError, SlabFunction, SlabTypeId, WallFunction,
    WallId, WallSide, WallTypeId, WindowOperation,
};

use crate::step_read::{self, Entity, ParseError, StepFile, Value};
pub use compare::differences;
use geometry::{Body, Geo, Xf, P3};
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
    walls: HashMap<u64, WallFrame>,
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

    /// The axis curve (element coordinates) and the body (world) of a
    /// product.
    fn shape(&self, e: &Entity, xf: &Xf) -> (Option<(Vec<P3>, bool)>, Body) {
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
                        let mut curved = false;
                        if let Some(pts) = self.geo.curve(item, &mut curved) {
                            axis = Some((pts, curved));
                        }
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

    fn read_wall(&mut self, id: u64) -> Result<WallCandidate, &'static str> {
        let e = self.ent(id).ok_or("no existe")?;
        let xf = self.geo.placement(e.arg(5));
        let (axis, body) = self.shape(e, &xf);
        if body.is_empty() {
            return Err("sin geometría que ArchiSim sepa leer");
        }
        let (lo, hi) = body.bounds().ok_or("sin geometría")?;
        let u_hint = Point2::new(xf.x[0], xf.x[1]);
        let line_y; // local Y of the line a–b
        let (mut a, mut b) = match &axis {
            Some((_, true)) => return Err("muro curvo (aún no se importan)"),
            Some((pts, false)) => {
                let first = pts[0];
                let last = pts[pts.len() - 1];
                let (pa, pb) = (p2(xf.apply(first)), p2(xf.apply(last)));
                let d = pb - pa;
                let len = d.length();
                for p in &pts[1..pts.len() - 1] {
                    let q = p2(xf.apply(*p)) - pa;
                    if len > 0.0 && (q.cross(d) / len).abs() > 1e-3 {
                        return Err("muro de varios tramos (aún no se importan)");
                    }
                }
                line_y = first[1];
                (pa, pb)
            }
            None => {
                let u = u_hint.normalized().ok_or("muro sin dirección en planta")?;
                let o = p2(xf.origin);
                let ts: Vec<f64> = body.all_points().map(|p| (p2(p) - o).dot(u)).collect();
                let t0 = ts.iter().copied().fold(f64::INFINITY, f64::min);
                let t1 = ts.iter().copied().fold(f64::NEG_INFINITY, f64::max);
                line_y = 0.0;
                (o + u * t0, o + u * t1)
            }
        };
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
        // Across the axis, left positive.
        let across: Vec<f64> = body.all_points().map(|p| (p2(p) - a).dot(n)).collect();
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
                Some(u) => u,
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
        Ok(WallCandidate {
            ifc: id,
            guid: e.arg(0).as_str().unwrap_or("").to_string(),
            a,
            b,
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
                    Ok(c) => candidates.push(c),
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
            let (a, b) = (snap2(c.a + n * shift), snap2(c.b + n * shift));
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
                through: None,
                reference,
                height: snap(c.top - c.base),
                base_offset: snap(c.base - elevation),
                top: archisim_core::WallTop::Unconnected,
            };
            match self.model.add_wall(spec) {
                Ok(wid) => {
                    self.report.walls += 1;
                    self.guids.walls.push((wid, c.guid));
                    let length = a.distance(b);
                    self.walls.insert(
                        c.ifc,
                        WallFrame {
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
                        },
                    );
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
        let e = self.ent(id).ok_or("no existe")?;
        let xf = self.geo.placement(e.arg(5));
        let (_, body) = self.shape(e, &xf);
        if body.is_empty() {
            return Err("sin geometría que ArchiSim sepa leer".into());
        }
        // The outline: the largest vertical extrusion, else the highest
        // horizontal face.
        let (outline, holes, bottom, top) = if let Some(x) = body
            .extrusions
            .iter()
            .max_by(|a, b| area(&a.outline).total_cmp(&area(&b.outline)))
        {
            (x.outline.clone(), x.holes, x.bottom, x.top)
        } else {
            let (lo, hi) = body.bounds().ok_or("sin geometría")?;
            match geometry::top_outline(&body, hi[2]) {
                Some((outline, holes)) => (outline, holes, lo[2], hi[2]),
                None => {
                    return Err(if body.polygons.is_empty() {
                        "sin contorno horizontal legible".into()
                    } else {
                        "forjado inclinado o de forma libre (aún no se importa)".into()
                    })
                }
            }
        };
        if holes > 0 || self.ix.voids.contains_key(&id) {
            self.report.warn(
                "Huecos en forjados (patios, escaleras, instalaciones): se importan cerrados.",
            );
        }
        let outline = clean_outline(&outline);
        if outline.len() < 3 {
            return Err("contorno sin superficie".into());
        }
        let top = snap(top);
        let body_thickness = snap(top - bottom);

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
        self.report.slabs += 1;
        self.guids
            .slabs
            .push((sid, e.arg(0).as_str().unwrap_or("").to_string()));
        Ok(())
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
                    self.report
                        .skip("huecos", "hueco sin puerta ni ventana (aún no se importan)");
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

    fn import_opening(&mut self, wall: u64, void: u64, filling: u64) -> Result<(), String> {
        let fe = self.ent(filling).ok_or("no existe")?;
        let is_door = fe.kind.starts_with("IFCDOOR");
        if !is_door && !fe.kind.starts_with("IFCWINDOW") {
            return Err("el hueco lo llena algo que no es puerta ni ventana".into());
        }
        let frame = &self.walls[&wall];
        let (origin, u, n) = (frame.origin, frame.u, frame.n);
        let (wall_id, length, base, top) = (frame.id, frame.length, frame.base, frame.top);
        let (exterior, thickness) = (frame.exterior, frame.thickness);
        let level_elevation = frame.level_elevation;

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
        let (mut t0, mut t1, mut z0, mut z1) = (
            f64::INFINITY,
            f64::NEG_INFINITY,
            f64::INFINITY,
            f64::NEG_INFINITY,
        );
        for p in vbody.all_points() {
            let t = (p2(p) - origin).dot(u);
            t0 = t0.min(t);
            t1 = t1.max(t);
            z0 = z0.min(p[2]);
            z1 = z1.max(p[2]);
        }
        let (t0, t1) = (snap(t0.max(0.0)), snap(t1.min(length)));
        let (z0, z1) = (snap(z0.max(base)), snap(z1.min(top)));
        let (width, height) = (snap(t1 - t0), snap(z1 - z0));
        if width < 0.05 || height < 0.05 {
            return Err("hueco fuera del muro o demasiado pequeño".into());
        }

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
        let opening_type = self
            .opening_type(
                ifc_type,
                &type_name,
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

    // ---- Everything else ---------------------------------------------

    fn count_unsupported(&mut self) {
        for kind in UNSUPPORTED_CLASSES {
            let n = self.file.of_kind(kind).len();
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
            if !has_slabs {
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
        self.import_openings();
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
