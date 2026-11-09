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
    /// Clicks draw a contour line (all its points at `z`), not points.
    pub line: bool,
    /// Clicks draw the terrain's edge (E5): closed, it makes the terrain
    /// (its corners at `z`) or gives the level's terrain that edge.
    pub outline: bool,
    /// A click on a line of a DXF drawn in the plan makes it a contour line
    /// (E5), at the height of the nearest text that is a number.
    pub wand: bool,
    /// Clicks draw a platform in the level's terrain (bloque Terreno).
    pub pad: bool,
    /// Clicks draw a ramp: its axis from one end to the other.
    pub ramp: bool,
    /// A platform drawn as a rectangle from two corners (else a polygon).
    pub pad_rect: bool,
    /// Height of the platform, or of the ramp's start; `None`: the
    /// ground's where it is first clicked.
    pub pad_z: Option<f64>,
    /// Height of the ramp's end; `None`: the ground's where it ends.
    pub ramp_end: Option<f64>,
    /// Width of a ramp.
    pub ramp_width: f64,
    /// The banks, horizontal by vertical (1: 45°).
    pub pad_run: f64,
}

impl Default for TerrainSettings {
    fn default() -> Self {
        Self {
            z: 0.0,
            contour: 0.5,
            depth: 1.0,
            line: false,
            outline: false,
            wand: false,
            pad: false,
            ramp: false,
            pad_rect: true,
            pad_z: None,
            ramp_end: None,
            ramp_width: 3.0,
            pad_run: 1.0,
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
        lines: Vec<Vec<usize>>,
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
                lines,
            })
        })
    }

    /// A survey imported (points, and contour lines each at its height):
    /// a new terrain on the active level, or added to `into`.
    pub fn import_terrain(
        &mut self,
        s: &TerrainSettings,
        points: Vec<SurveyPoint>,
        contours: Vec<(Vec<archisim_core::Point2>, f64)>,
        into: Option<TerrainId>,
    ) -> EditResult<TerrainId> {
        let level = self.level;
        self.transact("Importar topografía", |m| match into {
            Some(id) => {
                m.edit_terrain(id, |t| {
                    let existing = std::mem::take(&mut t.points);
                    let (all, lines) = archisim_core::survey_points(existing, points, &contours);
                    t.points = all;
                    t.lines.extend(lines);
                })?;
                Ok(id)
            }
            None => {
                let material = earth(m);
                let (all, lines) = archisim_core::survey_points(Vec::new(), points, &contours);
                m.add_terrain(NewTerrain {
                    level,
                    points: all,
                    material,
                    depth: s.depth,
                    contour: s.contour,
                    lines,
                })
            }
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

    /// Adds a platform (or a ramp) to a terrain and grades its ground
    /// again; returns its index.
    pub fn add_terrain_pad(
        &mut self,
        id: TerrainId,
        pad: archisim_core::TerrainPad,
    ) -> EditResult<usize> {
        let label = if pad.tilt.is_some() {
            "Rampa"
        } else {
            "Plataforma"
        };
        self.transact(label, |m| {
            let mut index = 0;
            m.edit_terrain(id, |t| {
                let mut pads = t.pads.clone();
                pads.push(pad);
                index = pads.len() - 1;
                t.set_pads(pads);
            })?;
            Ok(index)
        })
    }

    /// Sets (or clears) where the project is on the map.
    pub fn set_georeference(&mut self, g: Option<archisim_core::Georeference>) -> EditResult {
        self.transact("Emplazamiento", |m| m.set_georeference(g))
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
                Vec::new(),
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
