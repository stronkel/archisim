//! Objects (furniture, sanitary ware, kitchen units), each change one
//! undoable step.

use archisim_core::{NewObject, Object, ObjectId, ObjectTypeId, Point2, Value};
use archisim_library::collections::Collection;

use super::{EditResult, Editor};

/// Settings of the object tool.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ObjectSettings {
    pub object_type: Option<ObjectTypeId>,
    pub rotation: f64,
    pub base_offset: f64,
}

impl Default for ObjectSettings {
    fn default() -> Self {
        Self {
            object_type: None,
            rotation: 0.0,
            base_offset: 0.0,
        }
    }
}

impl Editor {
    /// An object of the tool's type at `at`, on the active level.
    pub fn add_object(&mut self, s: &ObjectSettings, at: Point2) -> EditResult<ObjectId> {
        let Some(object_type) = s.object_type else {
            return Err("elige antes el tipo de objeto".into());
        };
        let level = self.level;
        // Trees, people and vehicles stand on the terrain under them.
        let model = self.model();
        let outdoors = model
            .object_type(object_type)
            .and_then(|t| model.family(t.family))
            .is_some_and(|f| {
                matches!(
                    f.def().category.as_str(),
                    "site" | "street" | "nature" | "people" | "vehicle"
                )
            });
        let ground = outdoors
            .then(|| {
                let elevation = model.level(level)?.elevation;
                let z = model
                    .terrains_on_level(level)
                    .find_map(|t| t.height_at(at))?;
                Some((z - elevation) * 1000.0)
            })
            .flatten()
            .map_or(0.0, |mm| mm.round() / 1000.0);
        self.transact("Colocar objeto", |m| {
            let id = m.add_object(NewObject {
                level,
                object_type,
                position: at,
                rotation: s.rotation,
                base_offset: s.base_offset + ground,
            })?;
            // Against another that joins (worktops, bookcases…): one.
            Ok(crate::placing::join_alongside(m, id).unwrap_or(id))
        })
    }

    /// The project's type of a library object (see
    /// [`archisim_library::OBJECT_CATALOGUE`]), loaded if it has none.
    pub fn library_object_type(&mut self, name: &str) -> EditResult<ObjectTypeId> {
        if let Some(t) = self.model().object_types().find(|t| t.name == name) {
            return Ok(t.id);
        }
        self.transact("Cargar objeto de la biblioteca", |m| {
            archisim_library::add_object_entry(m, name)
        })
    }

    pub fn edit_object(
        &mut self,
        label: &str,
        id: ObjectId,
        edit: impl FnOnce(&mut Object),
    ) -> EditResult {
        self.transact(label, |m| m.edit_object(id, edit))
    }

    /// Gives an object a copy of its type, to change its sizes alone.
    pub fn object_own_type(&mut self, id: ObjectId) -> EditResult<ObjectTypeId> {
        let model = self.model();
        let Some(o) = model.object(id) else {
            return Err("el objeto no existe".into());
        };
        let Some(t) = model.object_type(o.object_type).cloned() else {
            return Err("el tipo no existe".into());
        };
        let taken: Vec<String> = model.object_types().map(|t| t.name.clone()).collect();
        let name = super::types::copy_name(&t.name, &taken);
        self.transact("Duplicar tipo de objeto", |m| {
            let new = m.add_object_type(name, t.family, t.values.clone())?;
            m.set_object_type_mesh(new, t.mesh)?;
            m.edit_object(id, |o| o.object_type = new)?;
            Ok(new)
        })
    }

    /// Changes one value of an object type: every object of it follows.
    /// Sets an object's own instance values.
    pub fn set_object_values(&mut self, id: ObjectId, values: archisim_core::Values) -> EditResult {
        self.transact("Modificar objeto", |m| m.set_object_values(id, values))
    }

    /// Gives one piece of an object (its material parameter `key`) a
    /// colour of the cosy palette, as Paralives' swatches: only this
    /// object changes (in its own type), in one step.
    pub fn paint_object_piece(&mut self, id: ObjectId, key: &str, swatch: &str) -> EditResult {
        if self.model().object(id).is_none() {
            return Err("el objeto no existe".into());
        }
        let ids = self.piece_mates(id, key);
        let key = key.to_string();
        let swatch = swatch.to_string();
        self.transact("Color de la pieza", move |m| {
            let mat = archisim_library::palette_material(m, &swatch)
                .ok_or_else(|| archisim_core::ModelError::NotAnObject(swatch.clone()))?;
            for id in &ids {
                crate::drag::set_object_value(m, *id, &key, Value::Material(mat.0))?;
            }
            Ok(())
        })
    }

    /// The objects a piece's colour goes to: the object, or, for the
    /// worktop of a base module under a worktop run, every module of the
    /// run (it is one worktop: given to one module only, that module
    /// would leave the run).
    pub fn piece_mates(&self, id: ObjectId, key: &str) -> Vec<ObjectId> {
        if key == "top" {
            if let Some(run) = self.model().worktop_run_of(id) {
                return run.members;
            }
        }
        vec![id]
    }

    /// Gives one piece of an object (its material parameter `key`) any
    /// colour, from the free colour wheel: a material like the one it had
    /// in that colour. Only this object changes, in one step.
    pub fn paint_object_piece_rgb(&mut self, id: ObjectId, key: &str, rgb: [u8; 3]) -> EditResult {
        let base = self
            .model()
            .object_values(id)
            .get(key)
            .or_else(|| {
                let o = self.model().object(id)?;
                self.model().object_type(o.object_type)?.values.get(key)
            })
            .and_then(|v| match v {
                Value::Material(m) => Some(archisim_core::MaterialId(*m)),
                _ => None,
            });
        let Some(base) = base.or_else(|| self.model().materials().next().map(|m| m.id)) else {
            return Err("el proyecto no tiene materiales".into());
        };
        let ids = self.piece_mates(id, key);
        let key = key.to_string();
        self.transact("Color de la pieza", move |m| {
            let mat = archisim_library::colour_material(m, base, rgb)
                .ok_or_else(|| archisim_core::ModelError::NotAnObject("material".into()))?;
            for id in &ids {
                crate::drag::set_object_value(m, *id, &key, Value::Material(mat.0))?;
            }
            Ok(())
        })
    }

    /// Takes what was changed on an object (its own type) to the type it
    /// came from: every object of that type changes alike.
    pub fn object_changes_to_type(&mut self, id: ObjectId) -> EditResult {
        self.transact("Aplicar a todos los del tipo", |m| {
            m.own_type_to_base(id).map(|_| ())
        })
    }

    /// The type of a piece of the library in a collection: the project's,
    /// or loaded (one undoable step).
    pub fn collection_object_type(
        &mut self,
        name: &str,
        c: &'static Collection,
    ) -> EditResult<ObjectTypeId> {
        let variant = archisim_library::collections::variant_name(name, c);
        if let Some(t) = self.model().object_types().find(|t| t.name == variant) {
            return Ok(t.id);
        }
        self.transact("Cargar objeto de la colección", |m| {
            archisim_library::collections::add_collection_entry(m, name, c)
        })
    }

    /// Puts objects in a collection (`None`: in none), one undoable step.
    /// Returns how many changed.
    pub fn apply_collection(
        &mut self,
        objects: &[ObjectId],
        c: Option<&'static Collection>,
    ) -> EditResult<usize> {
        let label = match c {
            Some(c) => format!("Colección {}", c.name),
            None => "Sin colección".to_string(),
        };
        self.transact(&label, |m| {
            archisim_library::collections::apply_collection(m, objects, c)
        })
    }

    pub fn set_object_type_value(
        &mut self,
        id: ObjectTypeId,
        name: &str,
        value: Value,
    ) -> EditResult {
        let Some(t) = self.model().object_type(id) else {
            return Err("el tipo no existe".into());
        };
        let mut values = t.values.clone();
        values.insert(name.to_string(), value);
        self.transact("Modificar tipo de objeto", |m| {
            m.set_object_type_values(id, values)
        })
    }
}

impl Editor {
    /// Brings a 3D model in as a new object type, turned, lightened and
    /// with who made it as `options` say (one undoable step; see
    /// [`archisim_library::imported::import_mesh_with`]).
    pub fn import_mesh_with(
        &mut self,
        mesh: archisim_mesh::Mesh,
        name: &str,
        source: &str,
        kind: archisim_library::imported::ModelKind,
        options: &archisim_library::imported::ImportOptions,
    ) -> EditResult<archisim_library::imported::Imported> {
        self.transact("Importar modelo 3D", |m| {
            archisim_library::imported::import_mesh_with(m, mesh, name, source, kind, options)
                .map_err(archisim_core::ModelError::InvalidMesh)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn objects_are_placed_and_their_type_changes() {
        let mut ed = Editor::new_project();
        let t = ed
            .model()
            .object_types()
            .find(|t| t.name.starts_with("Mesa"))
            .unwrap()
            .id;
        let s = ObjectSettings {
            object_type: Some(t),
            ..Default::default()
        };
        let id = ed.add_object(&s, Point2::new(2.0, 3.0)).unwrap();
        let wide = |ed: &Editor| {
            let o = ed.model().object_outline(id);
            o[0].distance(o[1])
        };
        assert!((wide(&ed) - 1.6).abs() < 1e-9);
        ed.set_object_type_value(t, "width", Value::Number(2.0))
            .unwrap();
        assert!((wide(&ed) - 2.0).abs() < 1e-9);
        ed.undo();
        assert!((wide(&ed) - 1.6).abs() < 1e-9);
        // Its own type: the others keep theirs.
        let other = ed.add_object(&s, Point2::new(6.0, 3.0)).unwrap();
        let own = ed.object_own_type(id).unwrap();
        assert_eq!(ed.model().object(id).unwrap().object_type, own);
        assert_eq!(ed.model().object(other).unwrap().object_type, t);
        assert!(ed
            .model()
            .object_type(own)
            .unwrap()
            .name
            .ends_with("(copia)"));
    }

    /// What is changed on a piece in the Maqueta is of that piece only
    /// (Antonio, 24/10/2026): the type it came from (the catalogue's) and
    /// the others of it stay as they were; «Aplicar a todos» takes it to
    /// them; deleting the piece takes its own type with it.
    #[test]
    fn changes_in_the_maquette_are_of_the_object() {
        let mut ed = Editor::new_project();
        let t = ed
            .model()
            .object_types()
            .find(|t| t.name.starts_with("Mesa"))
            .unwrap()
            .id;
        let s = ObjectSettings {
            object_type: Some(t),
            ..Default::default()
        };
        let a = ed.add_object(&s, Point2::new(2.0, 3.0)).unwrap();
        let wide = |ed: &Editor, id| {
            let o = ed.model().object_outline(id);
            o[0].distance(o[1])
        };
        // Alone of its type, it still gets its own: the type stays.
        ed.transact("x", |m| {
            crate::drag::set_object_value(m, a, "width", Value::Number(2.0))
        })
        .unwrap();
        let own = ed.model().object(a).unwrap().object_type;
        assert_ne!(own, t);
        assert!(ed
            .model()
            .object_type(own)
            .unwrap()
            .name
            .ends_with("(ejemplar)"));
        assert!((wide(&ed, a) - 2.0).abs() < 1e-9);
        let b = ed.add_object(&s, Point2::new(6.0, 3.0)).unwrap();
        assert!(
            (wide(&ed, b) - 1.6).abs() < 1e-9,
            "the catalogue's type stays"
        );
        // Changed again: the same own type, no new one.
        ed.transact("x", |m| {
            crate::drag::set_object_value(m, a, "width", Value::Number(2.2))
        })
        .unwrap();
        assert_eq!(ed.model().object(a).unwrap().object_type, own);
        // To all of the type.
        ed.object_changes_to_type(a).unwrap();
        assert_eq!(ed.model().object(a).unwrap().object_type, t);
        assert!(ed.model().object_type(own).is_none());
        assert!((wide(&ed, b) - 2.2).abs() < 1e-9);
        // Deleted, its own type goes with it.
        ed.paint_object_piece(b, "mat", "Nogal").unwrap();
        let own_b = ed.model().object(b).unwrap().object_type;
        assert_ne!(own_b, t);
        ed.transact("x", |m| m.remove_object(b).map(|_| ()))
            .unwrap();
        assert!(ed.model().object_type(own_b).is_none());
    }

    /// The free colour wheel: a piece in any colour, a material like the
    /// one it had (its texture kept), one step to undo; the same colour
    /// again reuses that material.
    #[test]
    fn a_piece_in_any_colour() {
        let mut ed = Editor::new_project();
        let t = ed
            .model()
            .object_types()
            .find(|t| t.name.starts_with("Mesa"))
            .unwrap()
            .id;
        let s = ObjectSettings {
            object_type: Some(t),
            ..Default::default()
        };
        let a = ed.add_object(&s, Point2::new(2.0, 3.0)).unwrap();
        let mat_of = |ed: &Editor| {
            let m = ed.model();
            let ty = m.object_type(m.object(a).unwrap().object_type).unwrap();
            match ty.values.get("mat") {
                Some(Value::Material(x)) => archisim_core::MaterialId(*x),
                other => panic!("{other:?}"),
            }
        };
        let before = mat_of(&ed);
        let texture = ed.model().appearance(before).texture;
        ed.paint_object_piece_rgb(a, "mat", [40, 90, 160]).unwrap();
        let blue = mat_of(&ed);
        assert_ne!(blue, before);
        assert_eq!(ed.model().surface_color(blue), [40, 90, 160]);
        assert_eq!(ed.model().appearance(blue).texture, texture);
        assert!(ed.model().material(blue).unwrap().name.ends_with("#285AA0"));
        // Again in blue from another colour: the same material.
        ed.paint_object_piece_rgb(a, "mat", [200, 10, 10]).unwrap();
        ed.paint_object_piece_rgb(a, "mat", [40, 90, 160]).unwrap();
        assert_eq!(mat_of(&ed), blue);
        assert!(!ed.model().material(blue).unwrap().name.contains("#C80A0A"));
        ed.undo();
        ed.undo();
        assert_eq!(mat_of(&ed), blue);
    }
}
