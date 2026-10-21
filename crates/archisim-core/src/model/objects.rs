//! Objects ("objetos"): furniture, sanitary ware and kitchen units placed
//! on a level, made by families that need no wall (`host none`). Like door
//! and window types, an object type is a family plus the values of its
//! type parameters; each object places one at a point, turned.
//!
//! Family axes for objects: origin at the middle of the back side, `x` to
//! the right, `y` forwards, `z` up from the floor (plus the object's base
//! offset). IFC: `IfcFurniture` or `IfcSanitaryTerminal`, as the recipe's
//! `ifc_class` and `ifc_type` outputs say.

use super::*;

/// Families whose objects ArchiSim places on a level: furniture, sanitary
/// ware, kitchen units, the site (vegetation, street furniture), people
/// and vehicles.
pub const OBJECT_CATEGORIES: [&str; 7] = [
    "furniture",
    "sanitary",
    "kitchen",
    "site",
    "street",
    "people",
    "vehicle",
];

#[derive(Debug, Clone, PartialEq)]
pub struct ObjectType {
    pub id: ObjectTypeId,
    pub guid: GlobalId,
    pub name: String,
    pub family: FamilyId,
    /// Value of every type parameter of the family.
    pub values: Values,
    /// A 3D model shown instead of the family's boxes (scaled to the
    /// type's height).
    pub mesh: Option<MeshId>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Object {
    pub id: ObjectId,
    pub guid: GlobalId,
    pub level: LevelId,
    pub object_type: ObjectTypeId,
    /// The family's origin in plan.
    pub position: Point2,
    /// Turn of the family's `x` axis from the plan's, counter-clockwise.
    pub rotation: f64,
    pub base_offset: f64,
    /// Its own values of the family's instance parameters (how a tree's
    /// canopy is drawn…); those left out take the family's default.
    pub values: Values,
}

/// Data needed to place an object.
#[derive(Debug, Clone, PartialEq)]
pub struct NewObject {
    pub level: LevelId,
    pub object_type: ObjectTypeId,
    pub position: Point2,
    pub rotation: f64,
    pub base_offset: f64,
}

impl Object {
    /// A point in the family's axes, in plan.
    pub fn to_plan(&self, x: f64, y: f64) -> Point2 {
        let (c, s) = (self.rotation.cos(), self.rotation.sin());
        Point2::new(
            self.position.x + x * c - y * s,
            self.position.y + x * s + y * c,
        )
    }
}

/// How an object is drawn in plan ([`Model::object_drawing`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ObjectDrawing {
    /// Its drawing of presentation (the canopy of a tree) in every view.
    Presentation,
    /// Its technical symbol for the scale in every view.
    Technical,
    /// As the view's style says.
    ByStyle,
}

impl Model {
    fn object_family(&self, family: FamilyId) -> Result<&Family, ModelError> {
        let fam = self
            .families
            .get(&family)
            .ok_or(ModelError::UnknownFamily(family))?;
        let def = fam.def();
        if !OBJECT_CATEGORIES.contains(&def.category.as_str())
            || def.host != archisim_family::Host::None
        {
            return Err(ModelError::NotAnObject(fam.name().to_string()));
        }
        Ok(fam)
    }

    fn check_object_type(&self, family: FamilyId, values: &Values) -> Result<(), ModelError> {
        let fam = self.object_family(family)?;
        for v in values.values() {
            if let Value::Material(m) = v {
                if !self.materials.contains_key(&MaterialId(*m)) {
                    return Err(ModelError::UnknownMaterial(MaterialId(*m)));
                }
            }
        }
        let instance = self.family_defaults(family, ParamScope::Instance)?;
        fam.def()
            .evaluate(values, &instance, &Values::new())
            .map(|_| ())
            .map_err(ModelError::Family)
    }

    /// An object's instance values: its own over the family's defaults.
    pub fn object_values(&self, id: ObjectId) -> Values {
        let Some(o) = self.objects.get(&id) else {
            return Values::new();
        };
        let Some(family) = self.object_types.get(&o.object_type).map(|t| t.family) else {
            return Values::new();
        };
        let Ok(mut v) = self.family_defaults(family, ParamScope::Instance) else {
            return Values::new();
        };
        for (k, x) in &o.values {
            if v.contains_key(k) {
                v.insert(k.clone(), x.clone());
            }
        }
        v
    }

    /// How an object is drawn in plan: its own «Dibujo en planta», or its
    /// type's when it says «del tipo». Objects whose family has no such
    /// choice follow the view's style.
    pub fn object_drawing(&self, id: ObjectId) -> ObjectDrawing {
        let own = self
            .object_values(id)
            .get("drawing")
            .and_then(|v| v.as_text().map(String::from));
        let of_type = || {
            let o = self.objects.get(&id)?;
            let t = self.object_types.get(&o.object_type)?;
            let mut v = self.family_defaults(t.family, ParamScope::Type).ok()?;
            v.extend(t.values.clone());
            v.get("plan_drawing")
                .and_then(|v| v.as_text().map(String::from))
        };
        let choice = match own.as_deref() {
            None | Some("del tipo") => of_type(),
            Some(_) => own,
        };
        match choice.as_deref() {
            Some("presentación") => ObjectDrawing::Presentation,
            Some("técnico") => ObjectDrawing::Technical,
            _ => ObjectDrawing::ByStyle,
        }
    }

    /// Sets an object's own instance values (checked against its family).
    pub fn set_object_values(&mut self, id: ObjectId, values: Values) -> Result<(), ModelError> {
        let o = self.objects.get(&id).ok_or(ModelError::UnknownObject(id))?;
        let t = self
            .object_types
            .get(&o.object_type)
            .ok_or(ModelError::UnknownObjectType(o.object_type))?;
        let def = self.object_family(t.family)?.def();
        let mut all = self.family_defaults(t.family, ParamScope::Instance)?;
        all.extend(values.clone());
        def.evaluate(&t.values, &all, &Values::new())
            .map_err(ModelError::Family)?;
        if let Some(o) = self.objects.get_mut(&id) {
            o.values = values;
        }
        Ok(())
    }

    /// A new object type of a family (furniture, sanitary or kitchen).
    pub fn add_object_type(
        &mut self,
        name: impl Into<String>,
        family: FamilyId,
        values: Values,
    ) -> Result<ObjectTypeId, ModelError> {
        self.check_object_type(family, &values)?;
        let id = ObjectTypeId(self.take_id());
        self.object_types.insert(
            id,
            ObjectType {
                id,
                guid: GlobalId::new(),
                name: name.into(),
                family,
                values,
                mesh: None,
            },
        );
        Ok(id)
    }

    /// Changes the values of an object type; every object of it follows.
    pub fn set_object_type_values(
        &mut self,
        id: ObjectTypeId,
        values: Values,
    ) -> Result<(), ModelError> {
        let family = self
            .object_types
            .get(&id)
            .ok_or(ModelError::UnknownObjectType(id))?
            .family;
        self.check_object_type(family, &values)?;
        if let Some(t) = self.object_types.get_mut(&id) {
            t.values = values;
        }
        Ok(())
    }

    /// Gives an object type a `GlobalId` of its own choosing (a library
    /// made always the same, as the extended library is).
    pub fn set_object_type_guid(
        &mut self,
        id: ObjectTypeId,
        guid: GlobalId,
    ) -> Result<(), ModelError> {
        if self
            .object_types
            .values()
            .any(|t| t.id != id && t.guid == guid)
        {
            return Err(ModelError::DuplicateGuid(guid.to_string()));
        }
        let t = self
            .object_types
            .get_mut(&id)
            .ok_or(ModelError::UnknownObjectType(id))?;
        t.guid = guid;
        Ok(())
    }

    pub fn object_type(&self, id: ObjectTypeId) -> Option<&ObjectType> {
        self.object_types.get(&id)
    }

    pub fn object_types(&self) -> impl Iterator<Item = &ObjectType> {
        self.object_types.values()
    }

    /// How many objects use a type.
    pub fn object_type_users(&self, id: ObjectTypeId) -> usize {
        self.objects
            .values()
            .filter(|o| o.object_type == id)
            .count()
    }

    fn validate_object(&self, o: &NewObject) -> Result<(), ModelError> {
        if !self.levels.contains_key(&o.level) {
            return Err(ModelError::UnknownLevel(o.level));
        }
        if !self.object_types.contains_key(&o.object_type) {
            return Err(ModelError::UnknownObjectType(o.object_type));
        }
        finite_point(o.position, "object position")?;
        finite(o.rotation, "object rotation")?;
        finite(o.base_offset, "object base offset")
    }

    pub fn add_object(&mut self, spec: NewObject) -> Result<ObjectId, ModelError> {
        self.validate_object(&spec)?;
        let id = ObjectId(self.take_id());
        self.objects.insert(
            id,
            Object {
                id,
                guid: GlobalId::new(),
                level: spec.level,
                object_type: spec.object_type,
                position: spec.position,
                rotation: spec.rotation,
                base_offset: spec.base_offset,
                values: Values::new(),
            },
        );
        self.stamp_phase(id.0);
        Ok(id)
    }

    /// Changes an object; nothing changes if the result is not valid.
    pub fn edit_object(
        &mut self,
        id: ObjectId,
        edit: impl FnOnce(&mut Object),
    ) -> Result<(), ModelError> {
        let mut o = self
            .objects
            .get(&id)
            .cloned()
            .ok_or(ModelError::UnknownObject(id))?;
        edit(&mut o);
        self.validate_object(&spec_of(&o))?;
        self.objects.insert(id, o);
        Ok(())
    }

    pub fn remove_object(&mut self, id: ObjectId) -> Result<Object, ModelError> {
        self.objects
            .remove(&id)
            .ok_or(ModelError::UnknownObject(id))
    }

    pub fn object(&self, id: ObjectId) -> Option<&Object> {
        self.objects.get(&id)
    }

    pub fn objects(&self) -> impl Iterator<Item = &Object> {
        self.objects.values()
    }

    pub fn objects_on_level(&self, level: LevelId) -> impl Iterator<Item = &Object> {
        self.objects.values().filter(move |o| o.level == level)
    }

    /// The 3D body and outputs of a placed object, in the family's axes.
    pub fn evaluate_object(&self, id: ObjectId) -> Result<Evaluation, ModelError> {
        let o = self.objects.get(&id).ok_or(ModelError::UnknownObject(id))?;
        let t = self
            .object_types
            .get(&o.object_type)
            .ok_or(ModelError::UnknownObjectType(o.object_type))?;
        let values = self.object_values(id);
        self.object_family(t.family)?
            .def()
            .evaluate(&t.values, &values, &Values::new())
            .map_err(ModelError::Family)
    }

    /// The plan symbol of a placed object for a drawing scale, in the
    /// family's axes.
    pub fn object_plan_symbol(&self, id: ObjectId, scale: f64) -> Result<Vec<Mark>, ModelError> {
        let o = self.objects.get(&id).ok_or(ModelError::UnknownObject(id))?;
        let t = self
            .object_types
            .get(&o.object_type)
            .ok_or(ModelError::UnknownObjectType(o.object_type))?;
        self.object_family(t.family)?
            .def()
            .evaluate_plan(&t.values, &self.object_values(id), &Values::new(), scale)
            .map_err(ModelError::Family)
    }

    /// The presentation symbol of a placed object (its family's `plan
    /// presentation`), each object a little different by its id; `None` if
    /// the family has none.
    pub fn object_presentation_symbol(
        &self,
        id: ObjectId,
    ) -> Option<Result<Vec<Mark>, ModelError>> {
        let o = self.objects.get(&id)?;
        let t = self.object_types.get(&o.object_type)?;
        let family = self.object_family(t.family).ok()?;
        // A number from 0 to 1 that stays with the object.
        let seed = {
            let mut h = id.0.wrapping_mul(0x9E37_79B9_7F4A_7C15);
            h ^= h >> 31;
            h = h.wrapping_mul(0xBF58_476D_1CE4_E5B9);
            h ^= h >> 29;
            (h >> 11) as f64 / (1u64 << 53) as f64
        };
        let builtins: Values = [("seed".to_string(), crate::family::Value::Number(seed))]
            .into_iter()
            .collect();
        family
            .def()
            .evaluate_presentation(&t.values, &self.object_values(id), &builtins)
            .map(|r| r.map_err(ModelError::Family))
    }

    /// The box of an object type in plan, in the family's axes: lowest and
    /// highest `x` and `y` of its parts.
    pub fn object_type_box(&self, id: ObjectTypeId) -> Option<([f64; 2], [f64; 2])> {
        let t = self.object_types.get(&id)?;
        let instance = self.family_defaults(t.family, ParamScope::Instance).ok()?;
        let ev = self
            .object_family(t.family)
            .ok()?
            .def()
            .evaluate(&t.values, &instance, &Values::new())
            .ok()?;
        let (mut lo, mut hi) = ([f64::MAX; 2], [f64::MIN; 2]);
        for p in &ev.parts {
            for k in 0..2 {
                lo[k] = lo[k].min(p.min[k]);
                hi[k] = hi[k].max(p.max[k]);
            }
        }
        (!ev.parts.is_empty()).then_some((lo, hi))
    }

    /// The box of a placed object in plan (its 3D parts seen from above),
    /// counter-clockwise; empty if it has none.
    pub fn object_outline(&self, id: ObjectId) -> Vec<Point2> {
        let (Some(o), Ok(ev)) = (self.objects.get(&id), self.evaluate_object(id)) else {
            return Vec::new();
        };
        if ev.parts.is_empty() {
            return Vec::new();
        }
        let (mut lo, mut hi) = ([f64::MAX; 2], [f64::MIN; 2]);
        for p in &ev.parts {
            for k in 0..2 {
                lo[k] = lo[k].min(p.min[k]);
                hi[k] = hi[k].max(p.max[k]);
            }
        }
        vec![
            o.to_plan(lo[0], lo[1]),
            o.to_plan(hi[0], lo[1]),
            o.to_plan(hi[0], hi[1]),
            o.to_plan(lo[0], hi[1]),
        ]
    }

    pub(super) fn restore_objects(
        &mut self,
        types: Vec<ObjectType>,
        objects: Vec<Object>,
        claim: &mut structure::Claim<'_>,
    ) -> Result<(), RestoreError> {
        for t in types {
            let name = format!("object type {} «{}»", t.id.0, t.name);
            claim(t.id.0, Some(&t.guid))
                .and_then(|_| self.check_object_type(t.family, &t.values))
                .and_then(|_| match t.mesh {
                    Some(m) if !self.meshes.contains_key(&m) => Err(ModelError::UnknownMesh(m)),
                    _ => Ok(()),
                })
                .map_err(|error| RestoreError {
                    element: name,
                    error,
                })?;
            self.object_types.insert(t.id, t);
        }
        for o in objects {
            let name = format!("object {} ({})", o.id.0, o.guid);
            claim(o.id.0, Some(&o.guid))
                .and_then(|_| self.validate_object(&spec_of(&o)))
                .map_err(|error| RestoreError {
                    element: name,
                    error,
                })?;
            self.objects.insert(o.id, o);
        }
        Ok(())
    }
}

fn spec_of(o: &Object) -> NewObject {
    NewObject {
        level: o.level,
        object_type: o.object_type,
        position: o.position,
        rotation: o.rotation,
        base_offset: o.base_offset,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every way of drawing a tree's canopy draws, as the tree's own
    /// choice or its type's.
    #[test]
    fn every_canopy_draws() {
        let mut m = Model::new("t");
        let pb = m.add_level("PB", 0.0);
        m.add_material(
            "Vegetación",
            "vegetation",
            [126, 164, 98],
            CutPattern::Solid,
        );
        m.add_material("Corteza", "wood", [112, 86, 62], CutPattern::Solid);
        let f = m.system_family("Árbol").unwrap();
        let values = m.family_defaults(f, ParamScope::Type).unwrap();
        let t = m.add_object_type("Árbol", f, values).unwrap();
        let o = m
            .add_object(NewObject {
                level: pb,
                object_type: t,
                position: Point2::new(0.0, 0.0),
                rotation: 0.0,
                base_offset: 0.0,
            })
            .unwrap();
        for shape in [
            "lóbulos",
            "círculo",
            "acuarela",
            "ramas",
            "follaje",
            "palmera",
            "conífera",
        ] {
            for look in ["transparente", "rellena", "contorno"] {
                let v: Values = [
                    ("shape_own".to_string(), Value::Text(shape.into())),
                    ("look_own".to_string(), Value::Text(look.into())),
                    ("opacity_own".to_string(), Value::Number(0.8)),
                ]
                .into_iter()
                .collect();
                m.set_object_values(o, v).unwrap();
                let marks = m.object_presentation_symbol(o).unwrap().unwrap();
                assert!(marks.len() >= 2, "{shape} {look}");
                let tinted = marks
                    .iter()
                    .any(|k| matches!(k, Mark::Polygon { tint: Some(_), .. }));
                assert!(tinted, "the trunk at least: {shape} {look}");
            }
        }
        assert!(m
            .set_object_values(
                o,
                [("shape_own".to_string(), Value::Text("cuadrada".into()))]
                    .into_iter()
                    .collect()
            )
            .is_err());
    }

    #[test]
    fn every_object_family_places_and_draws() {
        let mut m = Model::new("t");
        let pb = m.add_level("PB", 0.0);
        m.add_material("Madera de pino", "wood", [196, 160, 112], CutPattern::Solid);
        m.add_material("Porcelana", "ceramic", [240, 240, 236], CutPattern::Solid);
        for (name, _) in family::object_families() {
            let f = m.system_family(&name).unwrap();
            let values = m.family_defaults(f, ParamScope::Type).unwrap();
            let t = m
                .add_object_type(format!("{name} tipo"), f, values)
                .unwrap();
            let o = m
                .add_object(NewObject {
                    level: pb,
                    object_type: t,
                    position: Point2::new(1.0, 2.0),
                    rotation: 0.5,
                    base_offset: 0.0,
                })
                .unwrap();
            let ev = m.evaluate_object(o).unwrap();
            assert!(!ev.parts.is_empty(), "{name}");
            let category = m.family(f).unwrap().def().category.clone();
            // Trees, shrubs and people stand on their point.
            let centred = name == "Mesa" || category == "people" || {
                (category == "site" && name != "Seto")
                    || matches!(
                        name.as_str(),
                        "Farola"
                            | "Papelera"
                            | "Bolardo"
                            | "Alcorque"
                            | "Aparcabicis"
                            | "Señal de tráfico"
                    )
            };
            for p in &ev.parts {
                // On the floor; against the wall (nothing behind the
                // origin) but for tables, which have chairs all round.
                assert!(p.min[2] >= -1e-9, "{name} {p:?}");
                assert!(centred || p.min[1] >= -1e-9, "{name} {p:?}");
            }
            for scale in [50.0, 100.0] {
                assert!(
                    !m.object_plan_symbol(o, scale).unwrap().is_empty(),
                    "{name}"
                );
            }
            assert_eq!(m.object_outline(o).len(), 4);
            assert!([
                "IfcFurniture",
                "IfcSanitaryTerminal",
                "IfcGeographicElement",
                "IfcBuildingElementProxy",
                "IfcVehicle"
            ]
            .contains(&ev.outputs["ifc_class"].as_text().unwrap()));
            assert!(!m.family(f).unwrap().def().folder.is_empty(), "{name}");
        }
        // A door family cannot make objects.
        let door = m.system_family(family::SWING_DOOR).unwrap();
        let v = m.family_defaults(door, ParamScope::Type).unwrap();
        assert!(matches!(
            m.add_object_type("x", door, v),
            Err(ModelError::NotAnObject(_))
        ));
        let back = Model::from_parts(m.to_parts()).unwrap();
        assert_eq!(back, m);
    }
}
