//! Types of families between projects: a type goes to a library file as
//! its family's recipe with a catalogue entry (name, mark, `GlobalId`,
//! description, values, the materials it uses by name), and comes back into
//! any project the same, with the same `GlobalId`. Also updating a
//! project's copy of a family to a newer recipe, and purging the types no
//! element uses.
//!
//! Reading and writing the files is the application's business; here only
//! the model changes.

use archisim_family::{CatalogMaterial, CatalogType, ParamKind, ParamScope};

use super::{CutPattern, Model, ModelError, NewFamilyType};
use crate::family::{Family, Value, Values};
use crate::ids::*;

/// A type made from a family: a door or window type, or an object type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum FamilyTypeRef {
    Opening(OpeningTypeId),
    Object(ObjectTypeId),
}

/// A type ready for a library file: the recipe of its family (without a
/// catalogue), its entry and the materials it uses.
#[derive(Debug, Clone, PartialEq)]
pub struct CatalogEntry {
    pub family_name: String,
    pub recipe: String,
    pub entry: CatalogType,
    pub materials: Vec<CatalogMaterial>,
}

/// What to do when the project already has a type with the same `GlobalId`
/// but other values.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OnConflict {
    /// Fail with [`ModelError::TypeConflict`], so the user can choose.
    Ask,
    /// The project's type takes the library's values (its elements change).
    Replace,
    /// A new type with a new `GlobalId` and a free name.
    Rename,
}

/// What loading a type did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Loaded {
    /// A new type in the project.
    New(FamilyTypeRef),
    /// The project already had it, the same.
    AlreadyThere(FamilyTypeRef),
    /// The project's type of the same `GlobalId` took the library's values.
    Replaced(FamilyTypeRef),
}

impl Loaded {
    pub fn type_ref(self) -> FamilyTypeRef {
        match self {
            Loaded::New(t) | Loaded::AlreadyThere(t) | Loaded::Replaced(t) => t,
        }
    }
}

impl Model {
    fn type_parts(&self, t: FamilyTypeRef) -> Result<(FamilyId, &Values, &GlobalId), ModelError> {
        match t {
            FamilyTypeRef::Opening(id) => {
                let ty = self
                    .opening_type(id)
                    .ok_or(ModelError::UnknownOpeningType(id))?;
                Ok((ty.family, &ty.values, &ty.guid))
            }
            FamilyTypeRef::Object(id) => {
                let ty = self
                    .object_type(id)
                    .ok_or(ModelError::UnknownObjectType(id))?;
                Ok((ty.family, &ty.values, &ty.guid))
            }
        }
    }

    /// Every type made from a family, doors, windows and objects alike.
    pub fn family_types(&self) -> Vec<(FamilyTypeRef, FamilyId, String)> {
        let mut v: Vec<(FamilyTypeRef, FamilyId, String)> = self
            .opening_types()
            .map(|t| (FamilyTypeRef::Opening(t.id), t.family, t.name.clone()))
            .collect();
        v.extend(
            self.object_types()
                .map(|t| (FamilyTypeRef::Object(t.id), t.family, t.name.clone())),
        );
        v
    }

    /// A type as it is saved in a library file.
    pub fn catalog_entry(&self, t: FamilyTypeRef) -> Result<CatalogEntry, ModelError> {
        let (family, values, guid) = self.type_parts(t)?;
        let fam = self
            .family(family)
            .ok_or(ModelError::UnknownFamily(family))?;
        let (name, mark, description) = match t {
            FamilyTypeRef::Opening(id) => {
                let ty = self.opening_type(id).expect("checked");
                (ty.name.clone(), ty.mark.clone(), ty.description.clone())
            }
            FamilyTypeRef::Object(id) => {
                let ty = self.object_type(id).expect("checked");
                (ty.name.clone(), String::new(), String::new())
            }
        };
        let mut materials: Vec<CatalogMaterial> = Vec::new();
        let mut out = Vec::new();
        for p in fam.def().params_of(ParamScope::Type) {
            let Some(v) = values.get(&p.name) else {
                continue;
            };
            let v = match v {
                Value::Material(m) => {
                    let Some(mat) = self.material(MaterialId(*m)) else {
                        continue;
                    };
                    if !materials.iter().any(|x| x.name == mat.name) {
                        materials.push(CatalogMaterial {
                            name: mat.name.clone(),
                            category: mat.category.clone(),
                            color: mat.cut_color,
                        });
                    }
                    Value::Text(mat.name.clone())
                }
                other => other.clone(),
            };
            out.push((p.name.clone(), v));
        }
        Ok(CatalogEntry {
            family_name: fam.name().to_string(),
            recipe: archisim_family::without_catalog(&fam.source),
            entry: CatalogType {
                name,
                mark,
                guid: guid.to_string(),
                description,
                values: out,
            },
            materials,
        })
    }

    /// Every type of the project made from a family, as library files: one
    /// per family, with its recipe and the catalogue of its types and their
    /// materials. Returns (category of the family, family name, text).
    pub fn catalog_files(&self) -> Vec<(String, String, String)> {
        let mut by_family: std::collections::BTreeMap<
            String,
            (String, String, archisim_family::Catalog),
        > = std::collections::BTreeMap::new();
        for (t, family, _) in self.family_types() {
            let (Ok(e), Some(f)) = (self.catalog_entry(t), self.family(family)) else {
                continue;
            };
            let slot = by_family.entry(e.family_name.clone()).or_insert_with(|| {
                (
                    f.def().category.clone(),
                    e.recipe.clone(),
                    Default::default(),
                )
            });
            for mat in e.materials {
                if !slot.2.materials.iter().any(|x| x.name == mat.name) {
                    slot.2.materials.push(mat);
                }
            }
            slot.2.types.push(e.entry);
        }
        by_family
            .into_iter()
            .map(|(name, (category, recipe, catalog))| {
                let text = format!(
                    "{}\n{}",
                    recipe.trim_end(),
                    archisim_family::catalog_text(&catalog)
                );
                (category, name, text)
            })
            .collect()
    }

    /// The project's family of a recipe's name, loading the recipe if the
    /// project has none (a project keeps its own copy even if the library
    /// has a newer one: see [`Model::update_family`]).
    pub fn family_for_recipe(&mut self, recipe: &str) -> Result<FamilyId, ModelError> {
        let recipe = crate::family::expand_shared_pieces(&archisim_family::without_catalog(recipe))
            .map_err(ModelError::Family)?;
        let def = archisim_family::parse(&recipe).map_err(ModelError::Family)?;
        if let Some(f) = self.family_by_name(&def.name) {
            return Ok(f.id);
        }
        self.add_family(&recipe)
    }

    /// A catalogue's values in this project: materials by name (made from
    /// the catalogue's description of them when the project has none),
    /// type parameters the entry lacks at their defaults.
    fn catalog_values(
        &mut self,
        family: FamilyId,
        entry: &CatalogType,
        materials: &[CatalogMaterial],
    ) -> Result<Values, ModelError> {
        let mut values = self.family_defaults(family, ParamScope::Type)?;
        let fam = self
            .family(family)
            .ok_or(ModelError::UnknownFamily(family))?
            .clone();
        for (k, v) in &entry.values {
            let Some(p) = fam.def().param(k) else {
                // A parameter the project's recipe does not have.
                continue;
            };
            if p.scope != ParamScope::Type {
                continue;
            }
            let v = match (&p.kind, v) {
                (ParamKind::Material, Value::Text(name)) => {
                    let found = self.materials().find(|m| m.name == *name).map(|m| m.id);
                    let id = match found {
                        Some(id) => id,
                        None => {
                            let (category, color) = materials
                                .iter()
                                .find(|m| m.name == *name)
                                .map(|m| (m.category.clone(), m.color))
                                .unwrap_or_else(|| ("generic".into(), [200, 200, 200]));
                            self.add_material(name.clone(), category, color, CutPattern::Solid)
                        }
                    };
                    Value::Material(id.0)
                }
                (_, v) => v.clone(),
            };
            values.insert(k.clone(), v);
        }
        Ok(values)
    }

    /// Loads a type of a library file into the project. Its family is the
    /// project's of the same name (the recipe is added if missing). The
    /// type keeps its `GlobalId`: if the project already has it with the
    /// same values nothing changes; with others, `on_conflict` decides.
    pub fn load_catalog_type(
        &mut self,
        recipe: &str,
        entry: &CatalogType,
        materials: &[CatalogMaterial],
        on_conflict: OnConflict,
    ) -> Result<Loaded, ModelError> {
        let family = self.family_for_recipe(recipe)?;
        let values = self.catalog_values(family, entry, materials)?;
        let guid = GlobalId::parse(&entry.guid);
        let is_opening = self
            .family(family)
            .ok_or(ModelError::UnknownFamily(family))?
            .is_opening();
        // The same type already in the project.
        let existing = guid.as_ref().and_then(|g| {
            self.family_types()
                .into_iter()
                .map(|(t, _, _)| t)
                .find(|t| self.type_parts(*t).is_ok_and(|(_, _, tg)| tg == g))
        });
        let mut new_guid = guid;
        if let Some(t) = existing {
            let (f, v, _) = self.type_parts(t)?;
            if f == family && *v == values {
                return Ok(Loaded::AlreadyThere(t));
            }
            match on_conflict {
                OnConflict::Ask => return Err(ModelError::TypeConflict(entry.name.clone())),
                OnConflict::Replace => {
                    match t {
                        FamilyTypeRef::Opening(id) => self.edit_family_type(id, family, values)?,
                        FamilyTypeRef::Object(id) => {
                            if self.object_type(id).is_some_and(|o| o.family != family) {
                                return Err(ModelError::TypeConflict(entry.name.clone()));
                            }
                            self.set_object_type_values(id, values)?;
                        }
                    }
                    return Ok(Loaded::Replaced(t));
                }
                OnConflict::Rename => new_guid = None,
            }
        }
        // A free name.
        let taken: Vec<String> = self.family_types().into_iter().map(|(_, _, n)| n).collect();
        let mut name = entry.name.clone();
        let mut n = 2;
        while taken.contains(&name) {
            name = format!("{} ({n})", entry.name);
            n += 1;
        }
        let t = if is_opening {
            let mark = (!entry.mark.trim().is_empty()
                && self.check_mark(&entry.mark, None).is_ok())
            .then(|| entry.mark.clone());
            let id = self.add_family_type(NewFamilyType {
                family,
                name,
                mark,
                description: entry.description.clone(),
                values,
            })?;
            if let (Some(g), Some(ty)) = (new_guid, self.opening_types.get_mut(&id)) {
                ty.guid = g;
            }
            FamilyTypeRef::Opening(id)
        } else {
            let id = self.add_object_type(name, family, values)?;
            if let (Some(g), Some(ty)) = (new_guid, self.object_types.get_mut(&id)) {
                ty.guid = g;
            }
            FamilyTypeRef::Object(id)
        };
        Ok(Loaded::New(t))
    }

    /// Replaces the recipe of a family of the project with another of the
    /// same family (a newer version, from the program or a library). Every
    /// type keeps its values for the parameters still there and takes the
    /// defaults of the new ones; so do the instance values of doors and
    /// windows. If any type, door or window does not work with it, nothing
    /// changes. Returns how many types it updated.
    pub fn update_family(&mut self, id: FamilyId, recipe: &str) -> Result<usize, ModelError> {
        let recipe = crate::family::expand_shared_pieces(&archisim_family::without_catalog(recipe))
            .map_err(ModelError::Family)?;
        let old = self
            .family(id)
            .ok_or(ModelError::UnknownFamily(id))?
            .clone();
        let new = Family::new(id, recipe).map_err(ModelError::Family)?;
        if new.name() != old.name() {
            return Err(ModelError::InvalidShape(
                "a family can only be updated with a recipe of the same name",
            ));
        }
        let backup = self.clone();
        self.families.insert(id, new.clone());
        let result = self.reapply_family(id, &new);
        if result.is_err() {
            *self = backup;
        }
        result
    }

    fn reapply_family(&mut self, id: FamilyId, new: &Family) -> Result<usize, ModelError> {
        let merge = |m: &Model, scope: ParamScope, old: &Values| -> Result<Values, ModelError> {
            let mut v = m.family_defaults(id, scope)?;
            for p in new.def().params_of(scope) {
                if let Some(x) = old.get(&p.name) {
                    v.insert(p.name.clone(), x.clone());
                }
            }
            Ok(v)
        };
        let mut n = 0;
        let openings: Vec<(OpeningTypeId, Values)> = self
            .opening_types()
            .filter(|t| t.family == id)
            .map(|t| (t.id, t.values.clone()))
            .collect();
        for (t, old) in openings {
            let values = merge(self, ParamScope::Type, &old)?;
            self.edit_family_type(t, id, values)?;
            n += 1;
        }
        let instances: Vec<(OpeningId, Values)> = self
            .openings()
            .filter(|o| {
                self.opening_type(o.opening_type)
                    .is_some_and(|t| t.family == id)
            })
            .map(|o| (o.id, o.values.clone()))
            .collect();
        for (o, old) in instances {
            let values = merge(self, ParamScope::Instance, &old)?;
            if let Some(op) = self.openings.get_mut(&o) {
                op.values = values;
            }
        }
        let objects: Vec<(ObjectTypeId, Values)> = self
            .object_types()
            .filter(|t| t.family == id)
            .map(|t| (t.id, t.values.clone()))
            .collect();
        for (t, old) in objects {
            let values = merge(self, ParamScope::Type, &old)?;
            self.set_object_type_values(t, values)?;
            n += 1;
        }
        Ok(n)
    }

    /// Removes every type no element uses: door, window and object types,
    /// and wall and slab types (of these two, one is always kept, so the
    /// tools have one to draw with). Families with no type left go too, except the system ones.
    /// Returns how many types it removed.
    pub fn purge_unused_types(&mut self) -> usize {
        let mut n = 0;
        let unused: Vec<OpeningTypeId> = self
            .opening_types()
            .filter(|t| self.opening_type_users(t.id) == 0)
            .map(|t| t.id)
            .collect();
        for t in unused {
            if self.remove_opening_type(t).is_ok() {
                n += 1;
            }
        }
        let unused: Vec<ObjectTypeId> = self
            .object_types()
            .filter(|t| self.object_type_users(t.id) == 0)
            .map(|t| t.id)
            .collect();
        for t in unused {
            if self.object_types.remove(&t).is_some() {
                n += 1;
            }
        }
        // Wall and slab types: keep one of each.
        let unused: Vec<WallTypeId> = self
            .wall_types()
            .filter(|t| !self.walls().any(|w| w.wall_type == t.id))
            .map(|t| t.id)
            .collect();
        let keep = self.wall_types().count() == unused.len();
        for t in unused.into_iter().skip(usize::from(keep)) {
            if self.remove_wall_type(t).is_ok() {
                n += 1;
            }
        }
        let unused: Vec<SlabTypeId> = self
            .slab_types()
            .filter(|t| {
                !self.slabs().any(|s| s.slab_type == t.id)
                    && !self.roofs().any(|r| r.roof_type == t.id)
            })
            .map(|t| t.id)
            .collect();
        let keep = self.slab_types().count() == unused.len();
        for t in unused.into_iter().skip(usize::from(keep)) {
            if self.remove_slab_type(t).is_ok() {
                n += 1;
            }
        }
        // Families of no type, not the program's own.
        let system: Vec<String> = crate::family::system_families()
            .into_iter()
            .chain(crate::family::object_families())
            .map(|(name, _)| name)
            .collect();
        let used: std::collections::BTreeSet<FamilyId> =
            self.family_types().into_iter().map(|(_, f, _)| f).collect();
        let gone: Vec<FamilyId> = self
            .families()
            .filter(|f| !used.contains(&f.id) && !system.iter().any(|s| s == f.name()))
            .map(|f| f.id)
            .collect();
        for f in gone {
            self.families.remove(&f);
        }
        n
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::family;

    fn project() -> Model {
        let mut m = Model::new("P");
        m.add_level("PB", 0.0);
        m.add_material("Madera lacada", "wood", [240, 238, 232], CutPattern::Solid);
        m
    }

    /// The criterion of block E: a type made in one project loads in
    /// another the same, with the same GlobalId; loading it again changes
    /// nothing.
    #[test]
    fn a_type_goes_to_another_project_the_same() {
        let mut a = project();
        let door = a.system_family(family::SWING_DOOR).unwrap();
        let mut values = a.family_defaults(door, ParamScope::Type).unwrap();
        values.insert("width".into(), Value::Number(0.825));
        let lacquer = a
            .materials()
            .find(|m| m.name == "Madera lacada")
            .unwrap()
            .id;
        values.insert("panel_material".into(), Value::Material(lacquer.0));
        let t = a
            .add_family_type(NewFamilyType {
                family: door,
                name: "Paso 82,5 lacada".into(),
                mark: Some("P-07".into()),
                description: "Hoja lisa".into(),
                values,
            })
            .unwrap();
        let entry = a.catalog_entry(FamilyTypeRef::Opening(t)).unwrap();
        assert!(
            entry.materials.iter().any(|m| m.name == "Madera lacada"),
            "{:?}",
            entry.materials
        );
        // Through the text of a library file.
        let file = format!(
            "{}\n{}",
            entry.recipe,
            archisim_family::catalog_text(&archisim_family::Catalog {
                materials: entry.materials.clone(),
                types: vec![entry.entry.clone()],
            })
        );
        let def = archisim_family::parse(&file).unwrap();
        let mut b = project();
        let loaded = b
            .load_catalog_type(
                &file,
                &def.catalog.types[0],
                &def.catalog.materials,
                OnConflict::Ask,
            )
            .unwrap();
        let FamilyTypeRef::Opening(bt) = loaded.type_ref() else {
            panic!()
        };
        assert!(matches!(loaded, Loaded::New(_)));
        let (ta, tb) = (a.opening_type(t).unwrap(), b.opening_type(bt).unwrap());
        assert_eq!(ta.guid, tb.guid);
        assert_eq!(
            (ta.name.as_str(), ta.mark.as_str()),
            (tb.name.as_str(), tb.mark.as_str())
        );
        assert_eq!(ta.description, tb.description);
        assert!((ta.width - tb.width).abs() < 1e-12);
        assert_eq!(
            a.material(ta.panel_material).unwrap().name,
            b.material(tb.panel_material).unwrap().name
        );
        // A project without the material: it is made as the library says.
        let mut c = Model::new("C");
        c.add_material("Hormigón", "concrete", [180, 180, 180], CutPattern::Solid);
        let lc = c
            .load_catalog_type(
                &file,
                &def.catalog.types[0],
                &def.catalog.materials,
                OnConflict::Ask,
            )
            .unwrap();
        let FamilyTypeRef::Opening(ct) = lc.type_ref() else {
            panic!()
        };
        let made = c
            .material(c.opening_type(ct).unwrap().panel_material)
            .unwrap();
        assert_eq!(
            (made.name.as_str(), made.cut_color),
            ("Madera lacada", [240, 238, 232])
        );
        // Again: nothing new.
        let again = b
            .load_catalog_type(
                &file,
                &def.catalog.types[0],
                &def.catalog.materials,
                OnConflict::Ask,
            )
            .unwrap();
        assert_eq!(again, Loaded::AlreadyThere(loaded.type_ref()));
        assert_eq!(b.opening_types().filter(|x| x.guid == ta.guid).count(), 1);
        // Changed in the library: ask, replace or rename.
        let mut changed = def.catalog.types[0].clone();
        for (k, v) in changed.values.iter_mut() {
            if k == "width" {
                *v = Value::Number(0.9);
            }
        }
        assert!(matches!(
            b.load_catalog_type(&file, &changed, &[], OnConflict::Ask),
            Err(ModelError::TypeConflict(_))
        ));
        let renamed = b
            .load_catalog_type(&file, &changed, &[], OnConflict::Rename)
            .unwrap();
        let FamilyTypeRef::Opening(r) = renamed.type_ref() else {
            panic!()
        };
        assert_eq!(b.opening_type(r).unwrap().name, "Paso 82,5 lacada (2)");
        assert_ne!(b.opening_type(r).unwrap().guid, ta.guid);
        b.load_catalog_type(&file, &changed, &[], OnConflict::Replace)
            .unwrap();
        assert!((b.opening_type(bt).unwrap().width - 0.9).abs() < 1e-12);
    }

    /// Updating a family keeps the types' values and gives them the new
    /// parameters; a recipe that does not fit is refused whole.
    #[test]
    fn families_update_and_types_purge() {
        let mut m = project();
        let door = m.system_family(family::SWING_DOOR).unwrap();
        let values = m.family_defaults(door, ParamScope::Type).unwrap();
        let t = m
            .add_family_type(NewFamilyType {
                family: door,
                name: "Paso".into(),
                mark: None,
                description: String::new(),
                values,
            })
            .unwrap();
        let old = m.family(door).unwrap().source.clone();
        // A version with one more parameter.
        let v = m.family(door).unwrap().version();
        let newer = old
            .replacen(
                &format!("  version {v}"),
                &format!("  version {}", v + 1),
                1,
            )
            .replacen(
                "  section \"Hojas\"\n",
                "  section \"Hojas\"\n    handle bool = yes label \"Manilla\"\n",
                1,
            );
        assert_ne!(newer, old);
        assert_eq!(m.update_family(door, &newer).unwrap(), 1);
        assert_eq!(m.family(door).unwrap().version(), v + 1);
        assert_eq!(
            m.opening_type(t).unwrap().values.get("handle"),
            Some(&Value::Bool(true))
        );
        // Another family's recipe is refused.
        let window = crate::family::system_family(family::CASEMENT_WINDOW).unwrap();
        assert!(m.update_family(door, &window).is_err());
        assert_eq!(m.family(door).unwrap().version(), v + 1);
        // Library files of every family with types: they read back.
        let files = m.catalog_files();
        assert!(!files.is_empty());
        for (_, name, text) in &files {
            let def = archisim_family::parse(text).unwrap();
            assert_eq!(&def.name, name);
            assert!(!def.catalog.types.is_empty());
        }
        // Purge: the unused door type goes; the system family stays.
        let before = m.opening_types().count();
        assert!(m.purge_unused_types() >= 1);
        assert!(m.opening_types().count() < before);
        assert!(m.family(door).is_some());
    }
}
