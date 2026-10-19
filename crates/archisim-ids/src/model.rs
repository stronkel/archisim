//! An IFC file seen the way IDS asks about it: attributes by name, the
//! property sets, materials and classifications of each element (its own
//! and those it takes from its type), and what it is part of.

use std::collections::{BTreeMap, HashMap};

use archisim_ifc::step_read::{Entity, StepFile, Value};

use crate::schema::Schema;

/// A value found in the file, ready to compare with an IDS value.
#[derive(Debug, Clone, PartialEq)]
pub enum Found {
    Text(String),
    /// A number, and whether it was written as an integer.
    Number(f64, bool),
    /// `.T.`/`.F.`; `None` for `.U.` (unknown).
    Logical(Option<bool>),
    /// An enumeration value (`.SWINGING.`).
    Enum(String),
    /// A reference to another entity, a list, or anything else that is
    /// there but cannot be compared with a value.
    Object,
}

/// One property as IDS sees it.
#[derive(Debug, Clone)]
pub struct Prop {
    pub name: String,
    /// The values to compare (one for a single value, several for lists,
    /// enumerations, bounded and table properties), each with its IFC type
    /// (`IFCLABEL`, `IFCLENGTHMEASURE`…) if known. Empty: no value.
    pub values: Vec<(Found, Option<String>)>,
    /// Complex and reference properties cannot be checked.
    pub unsupported: bool,
}

pub struct IfcModel<'a> {
    pub file: &'a StepFile,
    pub schema_name: String,
    pub schema: &'static Schema,
    /// SI value of one file unit, by unit kind (`LENGTHUNIT`…).
    units: HashMap<String, f64>,
    defined_by: HashMap<u64, Vec<u64>>,
    type_of: HashMap<u64, u64>,
    materials: HashMap<u64, Vec<u64>>,
    classifications: HashMap<u64, Vec<u64>>,
    material_psets: HashMap<u64, Vec<u64>>,
    /// Parent by relation kind: element → (relation, parent).
    parents: HashMap<u64, Vec<(&'static str, u64)>>,
}

impl<'a> IfcModel<'a> {
    pub fn new(file: &'a StepFile) -> IfcModel<'a> {
        let schema_name = file
            .schemas
            .first()
            .cloned()
            .unwrap_or_else(|| "IFC4".into());
        let schema = Schema::for_file(&schema_name);
        let mut m = IfcModel {
            file,
            schema_name,
            schema,
            units: HashMap::new(),
            defined_by: HashMap::new(),
            type_of: HashMap::new(),
            materials: HashMap::new(),
            classifications: HashMap::new(),
            material_psets: HashMap::new(),
            parents: HashMap::new(),
        };
        m.index();
        m
    }

    /// Attribute of an entity by name, `Null` if it has no such attribute.
    pub fn attr<'e>(&self, e: &'e Entity, name: &str) -> &'e Value {
        match self.schema.index_of(&e.kind, name) {
            Some(i) => e.arg(i),
            None => &Value::Null,
        }
    }

    pub fn get(&self, id: u64) -> Option<&'a Entity> {
        self.file.get(id)
    }

    fn deref(&self, v: &Value) -> Option<&'a Entity> {
        v.as_ref().and_then(|r| self.file.get(r))
    }

    fn refs(v: &Value) -> Vec<u64> {
        match v {
            Value::Ref(r) => vec![*r],
            Value::List(_) => v.refs(),
            _ => Vec::new(),
        }
    }

    fn index(&mut self) {
        let file = self.file;
        let mut ids: Vec<&u64> = file.entities.keys().collect();
        ids.sort_unstable();
        for &id in ids {
            let e = &file.entities[&id];
            let s = self.schema;
            let kind = e.kind.as_str();
            let rel = |name: &str| s.index_of(kind, name).map(|i| e.arg(i));
            match kind {
                "IFCRELDEFINESBYPROPERTIES" => {
                    let def = Self::refs(rel("RelatingPropertyDefinition").unwrap_or(&Value::Null));
                    for o in Self::refs(rel("RelatedObjects").unwrap_or(&Value::Null)) {
                        self.defined_by
                            .entry(o)
                            .or_default()
                            .extend(def.iter().copied());
                    }
                }
                "IFCRELDEFINESBYTYPE" => {
                    if let Some(t) = rel("RelatingType").and_then(Value::as_ref) {
                        for o in Self::refs(rel("RelatedObjects").unwrap_or(&Value::Null)) {
                            self.type_of.insert(o, t);
                        }
                    }
                }
                "IFCRELASSOCIATESMATERIAL" => {
                    if let Some(mat) = rel("RelatingMaterial").and_then(Value::as_ref) {
                        for o in Self::refs(rel("RelatedObjects").unwrap_or(&Value::Null)) {
                            self.materials.entry(o).or_default().push(mat);
                        }
                    }
                }
                "IFCRELASSOCIATESCLASSIFICATION" => {
                    if let Some(c) = rel("RelatingClassification").and_then(Value::as_ref) {
                        for o in Self::refs(rel("RelatedObjects").unwrap_or(&Value::Null)) {
                            self.classifications.entry(o).or_default().push(c);
                        }
                    }
                }
                "IFCEXTERNALREFERENCERELATIONSHIP" => {
                    // IFC4: classifications of resources (materials…).
                    if let Some(c) = rel("RelatingReference").and_then(Value::as_ref) {
                        let is_class = file.get(c).is_some_and(|x| {
                            x.kind == "IFCCLASSIFICATIONREFERENCE" || x.kind == "IFCCLASSIFICATION"
                        });
                        if is_class {
                            for o in
                                Self::refs(rel("RelatedResourceObjects").unwrap_or(&Value::Null))
                            {
                                self.classifications.entry(o).or_default().push(c);
                            }
                        }
                    }
                }
                "IFCMATERIALPROPERTIES" | "IFCEXTENDEDMATERIALPROPERTIES" => {
                    if let Some(mat) = rel("Material").and_then(Value::as_ref) {
                        self.material_psets.entry(mat).or_default().push(id);
                    }
                }
                "IFCRELAGGREGATES" | "IFCRELNESTS" => {
                    let k = if kind == "IFCRELAGGREGATES" {
                        "IFCRELAGGREGATES"
                    } else {
                        "IFCRELNESTS"
                    };
                    if let Some(p) = rel("RelatingObject").and_then(Value::as_ref) {
                        for o in Self::refs(rel("RelatedObjects").unwrap_or(&Value::Null)) {
                            self.parents.entry(o).or_default().push((k, p));
                        }
                    }
                }
                "IFCRELCONTAINEDINSPATIALSTRUCTURE" => {
                    if let Some(p) = rel("RelatingStructure").and_then(Value::as_ref) {
                        for o in Self::refs(rel("RelatedElements").unwrap_or(&Value::Null)) {
                            self.parents
                                .entry(o)
                                .or_default()
                                .push(("IFCRELCONTAINEDINSPATIALSTRUCTURE", p));
                        }
                    }
                }
                "IFCRELASSIGNSTOGROUP" | "IFCRELASSIGNSTOGROUPBYFACTOR" => {
                    if let Some(p) = rel("RelatingGroup").and_then(Value::as_ref) {
                        for o in Self::refs(rel("RelatedObjects").unwrap_or(&Value::Null)) {
                            self.parents
                                .entry(o)
                                .or_default()
                                .push(("IFCRELASSIGNSTOGROUP", p));
                        }
                    }
                }
                "IFCRELVOIDSELEMENT" => {
                    if let (Some(host), Some(opening)) = (
                        rel("RelatingBuildingElement").and_then(Value::as_ref),
                        rel("RelatedOpeningElement").and_then(Value::as_ref),
                    ) {
                        self.parents
                            .entry(opening)
                            .or_default()
                            .push(("IFCRELVOIDSELEMENT", host));
                    }
                }
                "IFCRELFILLSELEMENT" => {
                    if let (Some(opening), Some(filler)) = (
                        rel("RelatingOpeningElement").and_then(Value::as_ref),
                        rel("RelatedBuildingElement").and_then(Value::as_ref),
                    ) {
                        self.parents
                            .entry(filler)
                            .or_default()
                            .push(("IFCRELFILLSELEMENT", opening));
                    }
                }
                "IFCUNITASSIGNMENT" => {
                    for u in e.arg(0).refs() {
                        if let Some((kind, k)) = self.unit_scale(u) {
                            self.units.insert(kind, k);
                        }
                    }
                }
                _ => {}
            }
        }
    }

    fn unit_scale(&self, id: u64) -> Option<(String, f64)> {
        let u = self.get(id)?;
        match u.kind.as_str() {
            "IFCSIUNIT" => {
                let kind = u.arg(1).as_enum()?.to_string();
                let base = match kind.as_str() {
                    "AREAUNIT" => 2,
                    "VOLUMEUNIT" => 3,
                    _ => 1,
                };
                let p = prefix(u.arg(2).as_enum().unwrap_or(""));
                // Grams are the SI name, kilograms the base unit.
                let gram = if u.arg(3).as_enum() == Some("GRAM") {
                    1e-3
                } else {
                    1.0
                };
                Some((kind, p.powi(base) * gram))
            }
            "IFCCONVERSIONBASEDUNIT" | "IFCCONVERSIONBASEDUNITWITHOFFSET" => {
                let kind = u.arg(1).as_enum()?.to_string();
                let measure = self.get(u.arg(3).as_ref()?)?;
                let value = measure.arg(0).as_f64()?;
                let inner = measure
                    .arg(1)
                    .as_ref()
                    .and_then(|r| self.unit_scale(r))
                    .map_or(1.0, |(_, k)| k);
                Some((kind, value * inner))
            }
            _ => None,
        }
    }

    /// The SI factor for a measure type (`IFCLENGTHMEASURE` → the file's
    /// length unit in metres).
    pub fn measure_scale(&self, data_type: &str) -> f64 {
        let kind = match data_type {
            "IFCLENGTHMEASURE" | "IFCPOSITIVELENGTHMEASURE" | "IFCNONNEGATIVELENGTHMEASURE" => {
                "LENGTHUNIT"
            }
            "IFCAREAMEASURE" => "AREAUNIT",
            "IFCVOLUMEMEASURE" => "VOLUMEUNIT",
            "IFCMASSMEASURE" | "IFCPOSITIVEMASSMEASURE" => "MASSUNIT",
            "IFCTIMEMEASURE" => "TIMEUNIT",
            "IFCPLANEANGLEMEASURE" | "IFCPOSITIVEPLANEANGLEMEASURE" => "PLANEANGLEUNIT",
            _ => return 1.0,
        };
        self.units.get(kind).copied().unwrap_or(1.0)
    }

    /// The classes an IDS entity facet can recognise an element by: its
    /// own, and in IFC2X3 the IFC4 name its type gives it.
    pub fn classes(&self, id: u64) -> Vec<String> {
        let Some(e) = self.get(id) else {
            return Vec::new();
        };
        let mut out = vec![e.kind.clone()];
        if self.schema_name.starts_with("IFC2X3") {
            if let Some(t) = self.type_of(id).and_then(|t| self.get(t)) {
                for (name, occ, ty) in crate::schema::IFC2X3_TYPED_CLASSES {
                    if *occ == e.kind && *ty == t.kind {
                        out.push(name.to_string());
                    }
                }
            }
        }
        out
    }

    /// The type object of an occurrence (`IfcRelDefinesByType`, or the
    /// `IsTypedBy`-less IFC2X3 equivalent).
    pub fn type_of(&self, id: u64) -> Option<u64> {
        self.type_of.get(&id).copied()
    }

    /// The predefined type of an element: its own, or its type's if it
    /// has none (`NOTDEFINED` counts as none); `USERDEFINED` resolves to
    /// the object, element or process type text.
    /// Both `USERDEFINED` and the text are given for user-defined types.
    pub fn predefined_type(&self, id: u64) -> Vec<String> {
        let Some(e) = self.get(id) else {
            return Vec::new();
        };
        let own = |e: &Entity| -> Vec<String> {
            let Some(p) = self.attr(e, "PredefinedType").as_enum() else {
                return Vec::new();
            };
            if p == "NOTDEFINED" {
                return Vec::new();
            }
            let mut out = vec![p.to_string()];
            if p == "USERDEFINED" {
                for name in ["ObjectType", "ElementType", "ProcessType"] {
                    if let Some(s) = self.attr(e, name).as_str().filter(|s| !s.is_empty()) {
                        out.insert(0, s.to_string());
                        break;
                    }
                }
            }
            out
        };
        let mine = own(e);
        if !mine.is_empty() {
            return mine;
        }
        self.type_of(id)
            .and_then(|t| self.get(t))
            .map(own)
            .unwrap_or_default()
    }

    /// The property sets of an element by name: its type's first, then
    /// its own, which override the type's property by property.
    pub fn property_sets(&self, id: u64) -> BTreeMap<String, PsetView> {
        let mut out: BTreeMap<String, PsetView> = BTreeMap::new();
        let add = |defs: &[u64], out: &mut BTreeMap<String, PsetView>| {
            for &d in defs {
                let Some(pset) = self.get(d) else { continue };
                let name = match pset.kind.as_str() {
                    "IFCMATERIALPROPERTIES" => self.attr(pset, "Name"),
                    "IFCEXTENDEDMATERIALPROPERTIES" => self.attr(pset, "Name"),
                    _ => self.attr(pset, "Name"),
                }
                .as_str()
                .unwrap_or("")
                .to_string();
                let view = out.entry(name).or_default();
                for p in self.pset_properties(pset) {
                    view.props.insert(p.name.clone(), p);
                }
            }
        };
        let Some(e) = self.get(id) else {
            return out;
        };
        if let Some(t) = self.type_of(id).and_then(|t| self.get(t)) {
            let defs = Self::refs(self.attr(t, "HasPropertySets"));
            add(&defs, &mut out);
        }
        // A type object's own sets.
        let defs = Self::refs(self.attr(e, "HasPropertySets"));
        add(&defs, &mut out);
        if let Some(defs) = self.defined_by.get(&id) {
            // IFC4 `IfcPropertySetDefinitionSet` lists come through `refs`.
            add(defs, &mut out);
        }
        if let Some(defs) = self.material_psets.get(&id) {
            add(defs, &mut out);
        }
        out
    }

    fn pset_properties(&self, pset: &Entity) -> Vec<Prop> {
        let s = self.schema;
        match pset.kind.as_str() {
            "IFCPROPERTYSET" => Self::refs(self.attr(pset, "HasProperties"))
                .into_iter()
                .filter_map(|p| self.get(p))
                .map(|p| self.property(p))
                .collect(),
            "IFCMATERIALPROPERTIES" => Self::refs(self.attr(pset, "Properties"))
                .into_iter()
                .filter_map(|p| self.get(p))
                .map(|p| self.property(p))
                .collect(),
            "IFCEXTENDEDMATERIALPROPERTIES" => Self::refs(self.attr(pset, "ExtendedProperties"))
                .into_iter()
                .filter_map(|p| self.get(p))
                .map(|p| self.property(p))
                .collect(),
            "IFCELEMENTQUANTITY" => Self::refs(self.attr(pset, "Quantities"))
                .into_iter()
                .filter_map(|q| self.get(q))
                .map(|q| self.quantity(q))
                .collect(),
            kind if s.is_a(kind, "IFCPROPERTYSETDEFINITION") => {
                // Predefined property sets: their own attributes are the
                // properties.
                let base = s.attributes("IFCPROPERTYSETDEFINITION").len();
                let all = s.attributes(kind);
                let base = if s.has("IFCPREDEFINEDPROPERTYSET") {
                    s.attributes("IFCPREDEFINEDPROPERTYSET").len()
                } else {
                    base
                };
                all.iter()
                    .enumerate()
                    .skip(base)
                    .map(|(i, name)| {
                        let v = pset.arg(i);
                        let (found, ty) = found(v, 1.0);
                        Prop {
                            name: name.to_string(),
                            values: found.map(|f| (f, ty)).into_iter().collect(),
                            unsupported: false,
                        }
                    })
                    .collect()
            }
            _ => Vec::new(),
        }
    }

    fn scaled(&self, v: &Value) -> (Option<Found>, Option<String>) {
        let ty = match v {
            Value::Typed(t, _) => Some(t.clone()),
            _ => None,
        };
        let k = ty.as_deref().map_or(1.0, |t| self.measure_scale(t));
        found(v, k)
    }

    fn property(&self, p: &Entity) -> Prop {
        let name = p.arg(0).as_str().unwrap_or("").to_string();
        let mut prop = Prop {
            name,
            values: Vec::new(),
            unsupported: false,
        };
        let push = |v: &Value, prop: &mut Prop| {
            let (f, t) = self.scaled(v);
            if let Some(f) = f {
                prop.values.push((f, t));
            }
        };
        match p.kind.as_str() {
            "IFCPROPERTYSINGLEVALUE" => push(p.arg(2), &mut prop),
            "IFCPROPERTYENUMERATEDVALUE" | "IFCPROPERTYLISTVALUE" => {
                for v in p.arg(2).as_list() {
                    push(v, &mut prop);
                }
            }
            "IFCPROPERTYBOUNDEDVALUE" => {
                for i in [2, 3, 5] {
                    push(p.arg(i), &mut prop);
                }
            }
            "IFCPROPERTYTABLEVALUE" => {
                for i in [2, 3] {
                    for v in p.arg(i).as_list() {
                        push(v, &mut prop);
                    }
                }
            }
            _ => prop.unsupported = true,
        }
        prop
    }

    fn quantity(&self, q: &Entity) -> Prop {
        let ty = match q.kind.as_str() {
            "IFCQUANTITYLENGTH" => "IFCLENGTHMEASURE",
            "IFCQUANTITYAREA" => "IFCAREAMEASURE",
            "IFCQUANTITYVOLUME" => "IFCVOLUMEMEASURE",
            "IFCQUANTITYCOUNT" => "IFCCOUNTMEASURE",
            "IFCQUANTITYWEIGHT" => "IFCMASSMEASURE",
            "IFCQUANTITYTIME" => "IFCTIMEMEASURE",
            "IFCQUANTITYNUMBER" => "IFCNUMERICMEASURE",
            _ => {
                return Prop {
                    name: q.arg(0).as_str().unwrap_or("").to_string(),
                    values: Vec::new(),
                    unsupported: true,
                }
            }
        };
        let v = q.arg(3);
        let k = self.measure_scale(ty);
        let (f, _) = found(v, k);
        Prop {
            name: q.arg(0).as_str().unwrap_or("").to_string(),
            values: f.map(|f| (f, Some(ty.to_string()))).into_iter().collect(),
            unsupported: false,
        }
    }

    /// Material names and categories an element can be checked against:
    /// those associated with it, or with its type if it has none.
    pub fn material_values(&self, id: u64) -> Option<Vec<String>> {
        let mats = self
            .materials
            .get(&id)
            .or_else(|| self.type_of(id).and_then(|t| self.materials.get(&t)))?;
        let mut out = Vec::new();
        for &m in mats {
            self.material_names(m, &mut out, 0);
        }
        Some(out)
    }

    fn material_names(&self, id: u64, out: &mut Vec<String>, depth: usize) {
        let Some(e) = self.get(id) else { return };
        if depth > 8 {
            return;
        }
        let text = |name: &str, out: &mut Vec<String>| {
            if let Some(s) = self.attr(e, name).as_str().filter(|s| !s.is_empty()) {
                out.push(s.to_string());
            }
        };
        match e.kind.as_str() {
            "IFCMATERIAL" => {
                text("Name", out);
                text("Category", out);
            }
            "IFCMATERIALLAYERSET" => {
                text("LayerSetName", out);
                for l in Self::refs(self.attr(e, "MaterialLayers")) {
                    self.material_names(l, out, depth + 1);
                }
            }
            "IFCMATERIALLAYER" | "IFCMATERIALLAYERWITHOFFSETS" => {
                text("Name", out);
                text("Category", out);
                if let Some(m) = self.attr(e, "Material").as_ref() {
                    self.material_names(m, out, depth + 1);
                }
            }
            "IFCMATERIALLAYERSETUSAGE" => {
                if let Some(s) = self.attr(e, "ForLayerSet").as_ref() {
                    self.material_names(s, out, depth + 1);
                }
            }
            "IFCMATERIALPROFILESET" => {
                text("Name", out);
                for p in Self::refs(self.attr(e, "MaterialProfiles")) {
                    self.material_names(p, out, depth + 1);
                }
            }
            "IFCMATERIALPROFILE" | "IFCMATERIALPROFILEWITHOFFSETS" => {
                text("Name", out);
                text("Category", out);
                if let Some(m) = self.attr(e, "Material").as_ref() {
                    self.material_names(m, out, depth + 1);
                }
            }
            "IFCMATERIALPROFILESETUSAGE" | "IFCMATERIALPROFILESETUSAGETAPERING" => {
                if let Some(s) = self.attr(e, "ForProfileSet").as_ref() {
                    self.material_names(s, out, depth + 1);
                }
            }
            "IFCMATERIALCONSTITUENTSET" => {
                text("Name", out);
                for c in Self::refs(self.attr(e, "MaterialConstituents")) {
                    self.material_names(c, out, depth + 1);
                }
            }
            "IFCMATERIALCONSTITUENT" => {
                text("Name", out);
                text("Category", out);
                if let Some(m) = self.attr(e, "Material").as_ref() {
                    self.material_names(m, out, depth + 1);
                }
            }
            "IFCMATERIALLIST" => {
                for m in Self::refs(self.attr(e, "Materials")) {
                    self.material_names(m, out, depth + 1);
                }
            }
            _ => {}
        }
    }

    /// Classification references of an element as (system, references
    /// from the most specific up). The occurrence's own references
    /// replace its type's in the same system.
    pub fn classifications(&self, id: u64) -> Option<Vec<(Option<String>, Vec<String>)>> {
        let own: Vec<_> = self
            .classifications
            .get(&id)
            .into_iter()
            .flatten()
            .filter_map(|&c| self.classification(c))
            .collect();
        let mut out = own.clone();
        if let Some(t) = self.type_of(id) {
            for c in self.classifications.get(&t).into_iter().flatten() {
                if let Some(c) = self.classification(*c) {
                    if !own.iter().any(|o| o.0 == c.0) {
                        out.push(c);
                    }
                }
            }
        }
        (!out.is_empty()).then_some(out)
    }

    fn classification(&self, id: u64) -> Option<(Option<String>, Vec<String>)> {
        let mut e = self.get(id)?;
        let mut refs = Vec::new();
        for _ in 0..16 {
            match e.kind.as_str() {
                "IFCCLASSIFICATION" => {
                    return Some((self.attr(e, "Name").as_str().map(str::to_string), refs));
                }
                "IFCCLASSIFICATIONREFERENCE" => {
                    let code = self
                        .attr(e, "Identification")
                        .as_str()
                        .or_else(|| self.attr(e, "ItemReference").as_str());
                    if let Some(c) = code {
                        refs.push(c.to_string());
                    }
                    match self.deref(self.attr(e, "ReferencedSource")) {
                        Some(next) => e = next,
                        None => return Some((None, refs)),
                    }
                }
                _ => return Some((None, refs)),
            }
        }
        Some((None, refs))
    }

    /// Parents of an element through a kind of relation (or any, with
    /// `None`), nearest first. Aggregation, nesting and containment
    /// climb: a part of an aggregate in a storey is contained in that
    /// storey and in the building it belongs to.
    pub fn parents(&self, id: u64, relation: Option<&str>) -> Vec<u64> {
        let mut out = Vec::new();
        let mut seen = std::collections::HashSet::new();
        match relation {
            Some("IFCRELASSIGNSTOGROUP") => {
                for &(k, p) in self.parents.get(&id).into_iter().flatten() {
                    if k == "IFCRELASSIGNSTOGROUP" {
                        out.push(p);
                    }
                }
            }
            Some("IFCRELAGGREGATES") | Some("IFCRELNESTS") => {
                let rel = relation.unwrap();
                let mut stack = vec![id];
                while let Some(x) = stack.pop() {
                    for &(k, p) in self.parents.get(&x).into_iter().flatten() {
                        if k == rel && seen.insert(p) {
                            out.push(p);
                            stack.push(p);
                        }
                    }
                }
            }
            Some("IFCRELCONTAINEDINSPATIALSTRUCTURE") => {
                // Up through aggregates to the containment, then up through
                // the spatial aggregation.
                let mut seen2 = std::collections::HashSet::new();
                let mut stack = vec![(id, false)];
                while let Some((x, inside)) = stack.pop() {
                    for &(k, p) in self.parents.get(&x).into_iter().flatten() {
                        let next = match k {
                            "IFCRELCONTAINEDINSPATIALSTRUCTURE" => true,
                            "IFCRELAGGREGATES" | "IFCRELNESTS" => inside,
                            _ => continue,
                        };
                        if seen2.insert((p, next)) {
                            if next {
                                out.push(p);
                            }
                            stack.push((p, next));
                        }
                    }
                }
            }
            Some(other) => {
                // IFCRELVOIDSELEMENT, IFCRELFILLSELEMENT, or both
                // ("IFCRELVOIDSELEMENT IFCRELFILLSELEMENT").
                let kinds: Vec<&str> = other.split_whitespace().collect();
                let mut stack = vec![id];
                while let Some(x) = stack.pop() {
                    for &(k, p) in self.parents.get(&x).into_iter().flatten() {
                        if kinds.contains(&k) && seen.insert(p) {
                            out.push(p);
                            stack.push(p);
                        }
                    }
                }
            }
            None => {
                let mut stack = vec![id];
                while let Some(x) = stack.pop() {
                    for &(_, p) in self.parents.get(&x).into_iter().flatten() {
                        if seen.insert(p) {
                            out.push(p);
                            stack.push(p);
                        }
                    }
                }
            }
        }
        out
    }

    /// A short human reference: class, name and GlobalId.
    pub fn label(&self, id: u64) -> String {
        let Some(e) = self.get(id) else {
            return format!("#{id}");
        };
        let name = self.attr(e, "Name").as_str().unwrap_or("");
        let gid = self.global_id(id).unwrap_or_default();
        let mut s = class_display(&e.kind);
        if !name.is_empty() {
            s += &format!(" «{name}»");
        }
        if !gid.is_empty() {
            s += &format!(" {gid}");
        } else {
            s += &format!(" #{id}");
        }
        s
    }

    pub fn global_id(&self, id: u64) -> Option<String> {
        let e = self.get(id)?;
        self.attr(e, "GlobalId").as_str().map(str::to_string)
    }
}

#[derive(Debug, Clone, Default)]
pub struct PsetView {
    pub props: BTreeMap<String, Prop>,
}

/// `IFCWALLSTANDARDCASE` → `IfcWallStandardCase` is not derivable without
/// the schema's spelling, so the upper-case name is kept but prefixed
/// in the usual way: `IfcWALL`. Readers recognise both.
pub fn class_display(kind: &str) -> String {
    match kind.strip_prefix("IFC") {
        Some(rest) => format!("Ifc{}", title(rest)),
        None => kind.to_string(),
    }
}

fn title(s: &str) -> String {
    let mut out = String::new();
    for (i, c) in s.chars().enumerate() {
        if i == 0 {
            out.push(c);
        } else {
            out.extend(c.to_lowercase());
        }
    }
    out
}

fn prefix(p: &str) -> f64 {
    match p {
        "EXA" => 1e18,
        "PETA" => 1e15,
        "TERA" => 1e12,
        "GIGA" => 1e9,
        "MEGA" => 1e6,
        "KILO" => 1e3,
        "HECTO" => 1e2,
        "DECA" => 1e1,
        "DECI" => 1e-1,
        "CENTI" => 1e-2,
        "MILLI" => 1e-3,
        "MICRO" => 1e-6,
        "NANO" => 1e-9,
        "PICO" => 1e-12,
        "FEMTO" => 1e-15,
        "ATTO" => 1e-18,
        _ => 1.0,
    }
}

/// A file value as something to compare, and its IFC type if it is a
/// typed value. Numbers are multiplied by `scale`. `None`: no value
/// (`$`, an empty string or an empty list).
pub fn found(v: &Value, scale: f64) -> (Option<Found>, Option<String>) {
    match v {
        Value::Null | Value::Derived => (None, None),
        Value::Str(s) if s.is_empty() => (None, None),
        Value::Str(s) => (Some(Found::Text(s.clone())), None),
        Value::Int(i) => (Some(Found::Number(*i as f64 * scale, true)), None),
        Value::Real(r) => (Some(Found::Number(r * scale, false)), None),
        Value::Enum(e) => match e.as_str() {
            "T" => (Some(Found::Logical(Some(true))), None),
            "F" => (Some(Found::Logical(Some(false))), None),
            "U" => (Some(Found::Logical(None)), None),
            _ => (Some(Found::Enum(e.clone())), None),
        },
        Value::Ref(_) => (Some(Found::Object), None),
        Value::List(l) if l.is_empty() => (None, None),
        Value::List(_) => (Some(Found::Object), None),
        Value::Typed(t, inner) => (found(inner, scale).0, Some(t.clone())),
    }
}
