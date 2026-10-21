//! Paint on the faces of walls: a material for one side, over the wall's
//! own (as painting a face in SketchUp or overriding a surface in
//! ArchiCAD). It changes how the wall is seen in 3D, not what it is made
//! of: its layers, measurements and IFC materials stay the type's.

use super::*;

/// A side of a wall: to the left of its line (start → end) is the
/// exterior, to the right the interior.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum FaceSide {
    Exterior,
    Interior,
}

impl FaceSide {
    pub fn code(self) -> &'static str {
        match self {
            FaceSide::Exterior => "exterior",
            FaceSide::Interior => "interior",
        }
    }

    pub fn from_code(c: &str) -> Option<Self> {
        match c {
            "exterior" => Some(FaceSide::Exterior),
            "interior" => Some(FaceSide::Interior),
            _ => None,
        }
    }
}

impl Model {
    /// Paints a face of a wall (`None` takes the paint off).
    pub fn set_face_paint(
        &mut self,
        wall: WallId,
        side: FaceSide,
        material: Option<MaterialId>,
    ) -> Result<(), ModelError> {
        if !self.walls.contains_key(&wall) {
            return Err(ModelError::UnknownWall(wall));
        }
        match material {
            Some(m) => {
                if !self.materials.contains_key(&m) {
                    return Err(ModelError::UnknownMaterial(m));
                }
                self.face_paints.insert((wall, side), m);
            }
            None => {
                self.face_paints.remove(&(wall, side));
            }
        }
        Ok(())
    }

    /// The paint on a face of a wall.
    pub fn face_paint(&self, wall: WallId, side: FaceSide) -> Option<MaterialId> {
        self.face_paints.get(&(wall, side)).copied()
    }

    /// Every painted face, of walls that are there.
    pub fn face_paints(&self) -> impl Iterator<Item = (WallId, FaceSide, MaterialId)> + '_ {
        self.face_paints
            .iter()
            .filter(|((w, _), _)| self.walls.contains_key(w))
            .map(|((w, s), m)| (*w, *s, *m))
    }

    pub(super) fn restore_face_paints(
        &mut self,
        paints: Vec<(WallId, FaceSide, MaterialId)>,
    ) -> Result<(), RestoreError> {
        for (w, side, m) in paints {
            self.set_face_paint(w, side, Some(m))
                .map_err(|error| RestoreError {
                    element: format!("paint of wall {}", w.0),
                    error,
                })?;
        }
        Ok(())
    }
}
