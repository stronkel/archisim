//! Terrain and true north, each change one undoable step.

use archisim_core::{Model, NewTerrain, SurveyPoint, Terrain, TerrainId};

use super::{EditResult, Editor};

/// Settings of the terrain tool.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TerrainSettings {
    /// Height given to the next point clicked (from project zero).
    pub z: f64,
    pub contour: f64,
    pub depth: f64,
}

impl Default for TerrainSettings {
    fn default() -> Self {
        Self {
            z: 0.0,
            contour: 0.5,
            depth: 1.0,
        }
    }
}

/// The ground's material: the model's earth, or added.
fn earth(model: &mut Model) -> archisim_core::MaterialId {
    let found = model
        .materials()
        .find(|m| m.category == "earth")
        .map(|m| m.id);
    match found {
        Some(m) => m,
        None => model.add_material(
            "Terreno natural",
            "earth",
            [176, 156, 120],
            archisim_core::CutPattern::Dots,
        ),
    }
}

impl Editor {
    /// A terrain from survey points, on the active level.
    pub fn add_terrain(
        &mut self,
        s: &TerrainSettings,
        points: Vec<SurveyPoint>,
    ) -> EditResult<TerrainId> {
        let level = self.level;
        self.transact("Crear terreno", |m| {
            let material = earth(m);
            m.add_terrain(NewTerrain {
                level,
                points,
                material,
                depth: s.depth,
                contour: s.contour,
            })
        })
    }

    pub fn edit_terrain(
        &mut self,
        label: &str,
        id: TerrainId,
        edit: impl FnOnce(&mut Terrain),
    ) -> EditResult {
        self.transact(label, |m| m.edit_terrain(id, edit))
    }

    pub fn set_true_north(&mut self, degrees: f64) -> EditResult {
        self.transact("Norte", |m| m.set_true_north(degrees.to_radians()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use archisim_core::Point2;

    #[test]
    fn terrain_with_the_editor() {
        let mut ed = Editor::new_project();
        let sp = |x: f64, y: f64, z: f64| SurveyPoint {
            at: Point2::new(x, y),
            z,
        };
        let s = TerrainSettings::default();
        let id = ed
            .add_terrain(
                &s,
                vec![sp(0.0, 0.0, 0.0), sp(10.0, 0.0, 1.0), sp(0.0, 10.0, 2.0)],
            )
            .unwrap();
        assert!(ed.model().materials().any(|m| m.category == "earth"));
        ed.edit_terrain("Añadir punto", id, |t| t.points.push(sp(10.0, 10.0, 3.0)))
            .unwrap();
        assert_eq!(ed.model().terrain(id).unwrap().triangles().len(), 2);
        ed.set_true_north(15.0).unwrap();
        assert!((ed.model().true_north() - 15f64.to_radians()).abs() < 1e-9);
        ed.undo();
        assert_eq!(ed.model().true_north(), 0.0);
    }
}
