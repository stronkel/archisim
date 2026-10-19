//! Curtain walls, each change one undoable step.

use archisim_core::{
    CurtainWall, CurtainWallId, MaterialId, Model, NewCurtainWall, Point2, WallTop,
};

use super::{EditResult, Editor};

/// Settings of the curtain wall tool.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CurtainSettings {
    pub height: f64,
    pub base_offset: f64,
    pub bay: f64,
    pub row: f64,
    pub frame_width: f64,
    pub frame_depth: f64,
    pub panel_thickness: f64,
}

impl Default for CurtainSettings {
    fn default() -> Self {
        Self {
            height: 2.7,
            base_offset: 0.0,
            bay: 1.2,
            row: 1.5,
            frame_width: 0.05,
            frame_depth: 0.12,
            panel_thickness: 0.024,
        }
    }
}

/// A material of the model by category, or added.
fn material(model: &mut Model, category: &str, name: &str, color: [u8; 3]) -> MaterialId {
    let found = model
        .materials()
        .find(|m| m.category == category)
        .map(|m| m.id);
    match found {
        Some(m) => m,
        None => model.add_material(name, category, color, archisim_core::CutPattern::Solid),
    }
}

impl Editor {
    /// A curtain wall from `start` to `end` (outside to the left), on the
    /// active level.
    pub fn add_curtain_wall(
        &mut self,
        s: &CurtainSettings,
        start: Point2,
        end: Point2,
    ) -> EditResult<CurtainWallId> {
        let level = self.level;
        self.transact("Crear muro cortina", |m| {
            let frame_material = material(m, "aluminium", "Aluminio lacado", [120, 124, 128]);
            let panel_material = material(m, "glass", "Vidrio doble", [190, 222, 236]);
            m.add_curtain_wall(NewCurtainWall {
                level,
                start,
                end,
                base_offset: s.base_offset,
                top: WallTop::Unconnected,
                height: s.height,
                bay: s.bay,
                row: s.row,
                frame_width: s.frame_width,
                frame_depth: s.frame_depth,
                panel_thickness: s.panel_thickness,
                frame_material,
                panel_material,
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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_curtain_wall_is_one_step() {
        let mut ed = Editor::new_project();
        let id = ed
            .add_curtain_wall(
                &CurtainSettings::default(),
                Point2::new(0.0, 0.0),
                Point2::new(4.8, 0.0),
            )
            .unwrap();
        let c = ed.model().curtain_wall(id).unwrap();
        assert_eq!(c.grid().mullions.len(), 5);
        ed.edit_curtain_wall("Módulo", id, |c| c.bay = 2.4).unwrap();
        assert_eq!(
            ed.model().curtain_wall(id).unwrap().grid().mullions.len(),
            3
        );
        ed.undo();
        ed.undo();
        assert!(ed.model().curtain_wall(id).is_none());
    }
}
