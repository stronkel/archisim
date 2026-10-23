//! The elements of an IFC file as coordination sees them: identity
//! (`GlobalId`, class, name, type, storey), their body as triangles, the
//! openings that void them, and a fingerprint of their data to tell
//! versions apart.

use std::collections::{BTreeMap, HashMap};
use std::hash::{Hash, Hasher};

use archisim_ids::model::{Found, IfcModel};
use archisim_ids::schema::Schema;
use archisim_ifc::import::geometry::P3;
use archisim_ifc::step_read::{self, StepFile};

use crate::mesh::{Mesh, Mesher, Tri};

/// The discipline a file or an element belongs to, as people sort
/// linked models.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Discipline {
    Architecture,
    Structure,
    Mechanical,
    Plumbing,
    Electrical,
    Site,
    Other,
}

impl Discipline {
    pub const ALL: [Discipline; 7] = [
        Discipline::Architecture,
        Discipline::Structure,
        Discipline::Mechanical,
        Discipline::Plumbing,
        Discipline::Electrical,
        Discipline::Site,
        Discipline::Other,
    ];

    /// How the settings files name it.
    pub fn code(self) -> &'static str {
        match self {
            Discipline::Architecture => "architecture",
            Discipline::Structure => "structure",
            Discipline::Mechanical => "mechanical",
            Discipline::Plumbing => "plumbing",
            Discipline::Electrical => "electrical",
            Discipline::Site => "site",
            Discipline::Other => "other",
        }
    }

    pub fn from_code(c: &str) -> Option<Discipline> {
        Self::ALL.into_iter().find(|d| d.code() == c)
    }

    pub fn name(self) -> &'static str {
        match self {
            Discipline::Architecture => "Arquitectura",
            Discipline::Structure => "Estructura",
            Discipline::Mechanical => "Climatización y ventilación",
            Discipline::Plumbing => "Fontanería y saneamiento",
            Discipline::Electrical => "Electricidad",
            Discipline::Site => "Urbanización",
            Discipline::Other => "Otros",
        }
    }

    /// From the class of an element (the usual IFC split).
    pub fn of_class(class: &str) -> Discipline {
        let s = Schema::for_file("IFC4");
        let c = class;
        if [
            "IFCBEAM",
            "IFCCOLUMN",
            "IFCFOOTING",
            "IFCPILE",
            "IFCREINFORCINGBAR",
            "IFCREINFORCINGMESH",
            "IFCTENDON",
            "IFCMEMBER",
            "IFCPLATE",
        ]
        .contains(&c)
        {
            return Discipline::Structure;
        }
        if s.is_a(c, "IFCFLOWSEGMENT")
            || s.is_a(c, "IFCFLOWFITTING")
            || s.is_a(c, "IFCFLOWTERMINAL")
            || s.is_a(c, "IFCFLOWCONTROLLER")
            || s.is_a(c, "IFCFLOWMOVINGDEVICE")
            || s.is_a(c, "IFCENERGYCONVERSIONDEVICE")
            || s.is_a(c, "IFCFLOWSTORAGEDEVICE")
            || s.is_a(c, "IFCFLOWTREATMENTDEVICE")
            || s.is_a(c, "IFCDISTRIBUTIONELEMENT")
        {
            if c.contains("DUCT") || c.contains("AIR") || c.contains("FAN") || c.contains("DAMPER")
            {
                return Discipline::Mechanical;
            }
            if c.contains("CABLE")
                || c.contains("LIGHT")
                || c.contains("LAMP")
                || c.contains("ELECTRIC")
                || c.contains("OUTLET")
                || c.contains("SWITCH")
                || c.contains("JUNCTION")
            {
                return Discipline::Electrical;
            }
            return Discipline::Plumbing;
        }
        if c.starts_with("IFCGEOGRAPHIC") || c.starts_with("IFCEARTHWORKS") || c == "IFCROAD" {
            return Discipline::Site;
        }
        if s.is_a(c, "IFCBUILDINGELEMENT") || s.is_a(c, "IFCBUILTELEMENT") {
            return Discipline::Architecture;
        }
        Discipline::Other
    }
}

/// One element of a file.
#[derive(Debug, Clone)]
pub struct Element {
    pub step_id: u64,
    pub global_id: String,
    /// Upper case, as in the file (`IFCWALL`).
    pub class: String,
    pub name: String,
    pub type_name: String,
    pub storey: Option<String>,
    pub discipline: Discipline,
    pub mesh: Mesh,
    pub bounds: Option<(P3, P3)>,
    /// Openings cutting it (`IfcRelVoidsElement`), as bodies.
    pub voids: Vec<Vec<Tri>>,
    /// Fingerprint of everything that is not geometry: name, type,
    /// attributes and properties. Equal fingerprints, same data.
    pub data_hash: u64,
    /// Volume of the body, m³ (to compare versions).
    pub volume: f64,
}

/// What a file contains, ready for viewing, comparing and checking
/// clashes.
#[derive(Debug, Clone)]
pub struct ModelElements {
    pub schema: String,
    pub project_name: String,
    /// The program that wrote it (`FILE_NAME` originating system).
    pub application: String,
    pub elements: Vec<Element>,
    /// Elements without readable body (classes), and pieces read only
    /// approximately, for the report.
    pub without_body: BTreeMap<String, usize>,
    pub approximate: usize,
}

impl ModelElements {
    pub fn by_guid(&self) -> HashMap<&str, &Element> {
        self.elements
            .iter()
            .map(|e| (e.global_id.as_str(), e))
            .collect()
    }

    pub fn bounds(&self) -> Option<(P3, P3)> {
        crate::mesh::bounds_of(
            self.elements
                .iter()
                .filter_map(|e| e.bounds)
                .flat_map(|(a, b)| [a, b]),
        )
    }

    /// The disciplines present, with how many elements each.
    pub fn disciplines(&self) -> BTreeMap<Discipline, usize> {
        let mut m = BTreeMap::new();
        for e in &self.elements {
            *m.entry(e.discipline).or_default() += 1;
        }
        m
    }

    /// The discipline most of its elements have.
    pub fn main_discipline(&self) -> Discipline {
        self.disciplines()
            .into_iter()
            .filter(|(d, _)| *d != Discipline::Other)
            .max_by_key(|(_, n)| *n)
            .map(|(d, _)| d)
            .unwrap_or(Discipline::Other)
    }
}

/// Classes that are not physical elements for coordination.
const SKIP: &[&str] = &[
    "IFCSITE",
    "IFCBUILDING",
    "IFCBUILDINGSTOREY",
    "IFCSPACE",
    "IFCOPENINGELEMENT",
    "IFCOPENINGSTANDARDCASE",
    "IFCVOIDINGFEATURE",
    "IFCANNOTATION",
    "IFCGRID",
    "IFCVIRTUALELEMENT",
    "IFCPROJECT",
    "IFCFACILITY",
    "IFCFACILITYPART",
    "IFCBRIDGE",
    "IFCROADPART",
    "IFCSPATIALZONE",
    "IFCEXTERNALSPATIALELEMENT",
    "IFCALIGNMENT",
    "IFCPORT",
    "IFCDISTRIBUTIONPORT",
];

/// Reads the elements of an IFC text.
pub fn read_elements(text: &str) -> Result<ModelElements, String> {
    let file = step_read::parse(text).map_err(|e| format!("el fichero no se puede leer ({e})"))?;
    Ok(elements_of(&file))
}

pub fn elements_of(file: &StepFile) -> ModelElements {
    let model = IfcModel::new(file);
    let schema = model.schema;
    let mesher = Mesher::new(file);
    let mut ids: Vec<u64> = file.entities.keys().copied().collect();
    ids.sort_unstable();
    // Relations needed here: voids, containment, aggregation.
    let mut voids: HashMap<u64, Vec<u64>> = HashMap::new();
    let mut container: HashMap<u64, u64> = HashMap::new();
    let mut parent: HashMap<u64, u64> = HashMap::new();
    for &id in &ids {
        let e = &file.entities[&id];
        match e.kind.as_str() {
            "IFCRELVOIDSELEMENT" => {
                if let (Some(h), Some(o)) = (e.arg(4).as_ref(), e.arg(5).as_ref()) {
                    voids.entry(h).or_default().push(o);
                }
            }
            "IFCRELCONTAINEDINSPATIALSTRUCTURE" => {
                if let Some(s) = e.arg(5).as_ref() {
                    for r in e.arg(4).refs() {
                        container.entry(r).or_insert(s);
                    }
                }
            }
            "IFCRELAGGREGATES" => {
                if let Some(p) = e.arg(4).as_ref() {
                    for r in e.arg(5).refs() {
                        parent.entry(r).or_insert(p);
                    }
                }
            }
            _ => {}
        }
    }
    let wholes: std::collections::HashSet<u64> = parent.values().copied().collect();
    let storey_of = |mut id: u64| -> Option<String> {
        for _ in 0..16 {
            if let Some(&s) = container.get(&id) {
                let e = file.get(s)?;
                return e.arg(2).as_str().map(str::to_string);
            }
            id = *parent.get(&id)?;
        }
        None
    };
    let mut out = ModelElements {
        schema: model.schema_name.clone(),
        project_name: file
            .of_kind("IFCPROJECT")
            .first()
            .and_then(|&p| file.get(p))
            .and_then(|p| p.arg(2).as_str().or(p.arg(4).as_str()))
            .unwrap_or("")
            .to_string(),
        application: file
            .header("FILE_NAME")
            .and_then(|h| h.arg(5).as_str().or(h.arg(4).as_str()))
            .unwrap_or("")
            .to_string(),
        elements: Vec::new(),
        without_body: BTreeMap::new(),
        approximate: 0,
    };
    for &id in &ids {
        let e = &file.entities[&id];
        if SKIP.contains(&e.kind.as_str())
            || !schema.is_a(&e.kind, "IFCPRODUCT")
            || schema.is_a(&e.kind, "IFCSPATIALSTRUCTUREELEMENT")
            || schema.is_a(&e.kind, "IFCSPATIALELEMENT")
            || schema.is_a(&e.kind, "IFCFEATUREELEMENTSUBTRACTION")
        {
            continue;
        }
        let Some(global_id) = e.arg(0).as_str() else {
            continue;
        };
        let mesh = mesher.product(e);
        if mesh.triangles.is_empty() {
            // Assemblies (a stair, a curtain wall) have their body in their
            // parts; anything else is reported.
            let has_parts = wholes.contains(&id);
            if !has_parts && e.arg(6).as_ref().is_some() {
                *out.without_body.entry(e.kind.clone()).or_default() += 1;
            }
            continue;
        }
        if mesh.approximate {
            out.approximate += 1;
        }
        let void_meshes: Vec<Vec<Tri>> = voids
            .get(&id)
            .into_iter()
            .flatten()
            .filter_map(|&o| file.get(o))
            .map(|o| mesher.product(o).triangles)
            .filter(|t| !t.is_empty())
            .collect();
        let type_name = model
            .type_of(id)
            .and_then(|t| file.get(t))
            .and_then(|t| t.arg(2).as_str())
            .unwrap_or("")
            .to_string();
        let bounds = mesh.bounds();
        let data_hash = data_fingerprint(&model, id, &type_name);
        let volume = mesh.volume();
        out.elements.push(Element {
            step_id: id,
            global_id: global_id.to_string(),
            class: e.kind.clone(),
            name: e.arg(2).as_str().unwrap_or("").to_string(),
            type_name,
            storey: storey_of(id),
            discipline: Discipline::of_class(&e.kind),
            mesh,
            bounds,
            voids: void_meshes,
            data_hash,
            volume,
        });
    }
    out
}

fn data_fingerprint(model: &IfcModel, id: u64, type_name: &str) -> u64 {
    let mut h = std::collections::hash_map::DefaultHasher::new();
    let Some(e) = model.get(id) else {
        return 0;
    };
    e.kind.hash(&mut h);
    type_name.hash(&mut h);
    // Name, description, object type, tag, predefined type: every
    // attribute that is a plain value (not geometry, not history).
    for (i, a) in e.args.iter().enumerate() {
        if i == 0 || i == 1 || i == 5 || i == 6 {
            continue;
        }
        format!("{a:?}").hash(&mut h);
    }
    for (set, view) in model.property_sets(id) {
        set.hash(&mut h);
        for (name, p) in &view.props {
            name.hash(&mut h);
            for (v, t) in &p.values {
                t.hash(&mut h);
                match v {
                    // Rounded: the same value written by another export.
                    Found::Number(n, _) => ((n * 1e6).round() as i64).hash(&mut h),
                    other => format!("{other:?}").hash(&mut h),
                }
            }
        }
    }
    h.finish()
}
