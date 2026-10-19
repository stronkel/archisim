//! Curtain walls and their types, each change one undoable step.

use archisim_core::{
    CurtainDesign, CurtainTypeId, CurtainWall, CurtainWallId, NewCurtainWall, Point2, WallTop,
};

use super::{EditResult, Editor};

/// Settings of the curtain wall tool: its type and how high it goes (the
/// rest is the type's).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CurtainSettings {
    /// `None`: the first type of the project.
    pub curtain_type: Option<CurtainTypeId>,
    pub height: f64,
    pub base_offset: f64,
}

impl Default for CurtainSettings {
    fn default() -> Self {
        Self {
            curtain_type: None,
            height: 2.7,
            base_offset: 0.0,
        }
    }
}

impl Editor {
    /// The type the tool draws with: the one chosen, else the first; a
    /// project with none (an imported one) gets the library's.
    fn tool_curtain_type(&mut self, s: &CurtainSettings) -> EditResult<CurtainTypeId> {
        let model = self.model();
        if let Some(t) = s.curtain_type.filter(|t| model.curtain_type(*t).is_some()) {
            return Ok(t);
        }
        if let Some(t) = model.curtain_types().next() {
            return Ok(t.id);
        }
        self.transact("Cargar tipos de muro cortina", |m| {
            archisim_library::add_curtain_types(m)
        })
    }

    /// A curtain wall from `start` to `end` (outside to the left), on the
    /// active level.
    pub fn add_curtain_wall(
        &mut self,
        s: &CurtainSettings,
        start: Point2,
        end: Point2,
    ) -> EditResult<CurtainWallId> {
        let curtain_type = self.tool_curtain_type(s)?;
        let level = self.level;
        self.transact("Crear muro cortina", |m| {
            m.add_curtain_wall(NewCurtainWall {
                curtain_type,
                level,
                start,
                end,
                base_offset: s.base_offset,
                top: WallTop::Unconnected,
                height: s.height,
            })
        })
    }

    pub fn edit_curtain_wall(
        &mut self,
        label: &str,
        id: CurtainWallId,
        edit: impl FnOnce(&mut CurtainWall),
    ) -> EditResult {
        self.transact(label, |m| m.edit_curtain_wall(id, edit))
    }

    // ---- Types ---------------------------------------------------------

    fn curtain_name(&self, id: Option<CurtainTypeId>, name: &str) -> EditResult<String> {
        let name = name.trim();
        if name.is_empty() {
            return Err("el tipo necesita un nombre".into());
        }
        if self
            .model()
            .curtain_types()
            .any(|t| Some(t.id) != id && t.name == name)
        {
            return Err(format!("ya hay un tipo llamado «{name}»"));
        }
        Ok(name.to_string())
    }

    /// Changes a curtain wall type (`label` names the step); all its walls
    /// follow.
    pub fn edit_curtain_type(
        &mut self,
        label: &str,
        id: CurtainTypeId,
        name: &str,
        design: CurtainDesign,
    ) -> EditResult {
        let name = self.curtain_name(Some(id), name)?;
        self.transact(label, |m| m.edit_curtain_type(id, name, design))
    }

    /// Copies a curtain wall type under a new name.
    pub fn duplicate_curtain_type(&mut self, id: CurtainTypeId) -> EditResult<CurtainTypeId> {
        let model = self.model();
        let Some(t) = model.curtain_type(id) else {
            return Err("el tipo no existe".into());
        };
        let taken: Vec<String> = model.curtain_types().map(|t| t.name.clone()).collect();
        let name = super::types::copy_name(&t.name, &taken);
        self.transact("Duplicar tipo de muro cortina", |m| {
            m.duplicate_curtain_type(id, name)
        })
    }

    /// A new type with a design of its own.
    pub fn add_curtain_type(
        &mut self,
        name: &str,
        design: CurtainDesign,
    ) -> EditResult<CurtainTypeId> {
        let name = self.curtain_name(None, name)?;
        self.transact("Nuevo tipo de muro cortina", |m| {
            m.add_curtain_type(name, design)
        })
    }

    pub fn remove_curtain_type(&mut self, id: CurtainTypeId) -> EditResult {
        let users = self.model().curtain_type_users(id);
        if users > 0 {
            return Err(format!("lo usan {users} muro(s) cortina"));
        }
        self.transact("Borrar tipo de muro cortina", |m| {
            m.remove_curtain_type(id).map(|_| ())
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_curtain_wall_and_its_type_are_one_step_each() {
        let mut ed = Editor::new_project();
        let id = ed
            .add_curtain_wall(
                &CurtainSettings::default(),
                Point2::new(0.0, 0.0),
                Point2::new(4.8, 0.0),
            )
            .unwrap();
        let t = ed.model().curtain_wall(id).unwrap().curtain_type;
        let d = ed.model().curtain_design(id).unwrap().clone();
        assert_eq!(
            ed.model().curtain_wall(id).unwrap().grid(&d).mullions.len(),
            5
        );
        let name = ed.model().curtain_type(t).unwrap().name.clone();
        let mut wide = d.clone();
        wide.columns = archisim_core::CurtainDivision::max_spacing(2.4);
        ed.edit_curtain_type("Módulo", t, &name, wide).unwrap();
        let c = ed.model().curtain_wall(id).unwrap();
        assert_eq!(
            c.grid(ed.model().curtain_design(id).unwrap())
                .mullions
                .len(),
            3
        );
        // Names are unique; a type in use stays.
        let copy = ed.duplicate_curtain_type(t).unwrap();
        assert!(ed
            .model()
            .curtain_type(copy)
            .unwrap()
            .name
            .ends_with("(copia)"));
        assert!(ed
            .edit_curtain_type("Nombre", copy, &name, d.clone())
            .is_err());
        assert!(ed.remove_curtain_type(t).is_err());
        ed.remove_curtain_type(copy).unwrap();
        ed.undo();
        ed.undo();
        ed.undo();
        assert_eq!(ed.model().curtain_type(t).unwrap().design, d);
        ed.undo();
        assert!(ed.model().curtain_wall(id).is_none());
    }

    #[test]
    fn a_project_with_no_curtain_types_gets_the_library_s() {
        let mut model = archisim_core::Model::new("importado");
        model.add_level("PB", 0.0);
        let mut ed = Editor::with_model(model, None);
        ed.add_curtain_wall(
            &CurtainSettings::default(),
            Point2::new(0.0, 0.0),
            Point2::new(3.0, 0.0),
        )
        .unwrap();
        assert_eq!(ed.model().curtain_types().count(), 5);
    }
}
