//! Stairs: drawing them by their walking line and changing them, each
//! change one undoable step.

use archisim_core::{
    LevelId, MaterialId, Model, NewStair, Point2, Stair, StairId, StairUse, WallTop,
};

use super::{EditResult, Editor, TopChoice};

/// Settings of the stair tool.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StairSettings {
    pub width: f64,
    pub going: f64,
    pub waist: f64,
    pub material: MaterialId,
    pub usage: StairUse,
    pub top: TopChoice,
    /// Height of stairs not tied to a level.
    pub height: f64,
}

/// The tool's settings for a model: 1 m wide, 28 cm treads, a 15 cm
/// concrete slab, up to the next level.
pub fn default_stair_settings(model: &Model) -> StairSettings {
    let material = model
        .materials()
        .find(|m| m.category == "concrete")
        .or_else(|| model.materials().next())
        .map_or(MaterialId(0), |m| m.id);
    StairSettings {
        width: 1.0,
        going: 0.28,
        waist: 0.15,
        material,
        usage: StairUse::General,
        top: TopChoice::NextLevel,
        height: 3.0,
    }
}

impl StairSettings {
    pub fn top_on(&self, model: &Model, level: LevelId) -> WallTop {
        let target = match self.top {
            TopChoice::Unconnected => None,
            TopChoice::NextLevel => model.level_above(level),
            TopChoice::Level(l) => Some(l),
        };
        match target {
            Some(level) => WallTop::Level { level, offset: 0.0 },
            None => WallTop::Unconnected,
        }
    }

    /// The height a stair drawn on `level` climbs.
    pub fn height_on(&self, model: &Model, level: LevelId) -> f64 {
        match self.top_on(model, level) {
            WallTop::Level { level: to, offset } => {
                let z0 = model.level(level).map_or(0.0, |l| l.elevation);
                model
                    .level(to)
                    .map_or(self.height, |l| l.elevation + offset - z0)
            }
            WallTop::Unconnected => self.height,
        }
    }
}

impl Editor {
    /// Draws a stair along a walking line on the active level.
    pub fn add_stair(&mut self, s: &StairSettings, path: Vec<Point2>) -> EditResult<StairId> {
        let level = self.level;
        let top = s.top_on(self.model(), level);
        self.transact("Crear escalera", |m| {
            m.add_stair(NewStair {
                level,
                path,
                width: s.width,
                going: s.going,
                base_offset: 0.0,
                top,
                height: s.height,
                waist: s.waist,
                material: s.material,
                usage: s.usage,
            })
        })
    }

    pub fn edit_stair(
        &mut self,
        label: &str,
        id: StairId,
        edit: impl FnOnce(&mut Stair),
    ) -> EditResult {
        self.transact(label, |m| m.edit_stair(id, edit))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stairs_with_the_editor() {
        let mut ed = Editor::new_project();
        let pb = ed.level;
        ed.add_level("Planta 1", 2.97).unwrap();
        ed.level = pb;
        let s = default_stair_settings(ed.model());
        assert!((s.height_on(ed.model(), pb) - 2.97).abs() < 1e-12);
        let id = ed
            .add_stair(&s, vec![Point2::new(0.0, 0.0), Point2::new(4.48, 0.0)])
            .unwrap();
        assert_eq!(ed.model().stair(id).unwrap().layout().risers(), 17);
        ed.edit_stair("Ancho", id, |s| s.width = 1.2).unwrap();
        assert!(ed.edit_stair("Ancho", id, |s| s.width = 0.0).is_err());
        ed.undo();
        assert_eq!(ed.model().stair(id).unwrap().width, 1.0);
    }
}
