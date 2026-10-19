//! Objects (furniture, sanitary ware, kitchen units), each change one
//! undoable step.

use archisim_core::{NewObject, Object, ObjectId, ObjectTypeId, Point2, Value};

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
        self.transact("Colocar objeto", |m| {
            m.add_object(NewObject {
                level,
                object_type,
                position: at,
                rotation: s.rotation,
                base_offset: s.base_offset,
            })
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
            m.edit_object(id, |o| o.object_type = new)?;
            Ok(new)
        })
    }

    /// Changes one value of an object type: every object of it follows.
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
}
