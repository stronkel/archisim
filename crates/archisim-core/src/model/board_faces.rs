//! A face composed by hand (M20, `docs/MODULAR.md`): the joints of a
//! wall's layer of boards drawn as lines on the face, seen from outside,
//! instead of laid by the material's rule; and, piece by piece, another
//! material (an accent) or «on site»: the piece left out of the box in the
//! factory and fitted in place once the boxes stand (over the joints
//! between boxes). The pieces are not kept: they are the regions the lines
//! cut the face in (`archisim_geom::boards`); a mark is a point inside one,
//! as a room's.

use super::*;

/// What a piece of a composed face takes.
#[derive(Debug, Clone, PartialEq)]
pub struct PieceMark {
    /// A point inside the piece: along the wall from its start and up from
    /// the level, m.
    pub at: Point2,
    /// Another material (an accent); `None`: the layer's.
    pub material: Option<MaterialId>,
    /// Fitted on site, not in the factory.
    pub on_site: bool,
}

/// A wall's layer of boards composed by hand.
#[derive(Debug, Clone, PartialEq)]
pub struct BoardFace {
    pub wall: WallId,
    /// The layer of the wall's type (from outside, 0…).
    pub layer: usize,
    /// The joints: lines on the face, along the wall from its start and up
    /// from the level, m. A line splits the pieces it crosses from side to
    /// side; one that stops short splits nothing.
    pub joints: Vec<(Point2, Point2)>,
    pub marks: Vec<PieceMark>,
    /// The open joint between its pieces, m; `None`: its material's
    /// (version 110).
    pub joint: Option<f64>,
}

impl BoardFace {
    pub fn is_valid(&self) -> bool {
        let ok = |p: &Point2| p.x.is_finite() && p.y.is_finite();
        self.joints
            .iter()
            .all(|(a, b)| ok(a) && ok(b) && (*b - *a).length() > 1e-4)
            && self.marks.iter().all(|m| ok(&m.at))
            && self
                .joint
                .is_none_or(|j| j.is_finite() && (0.0..=0.05).contains(&j))
    }
}

impl Model {
    /// The composed faces of the walls that are still there.
    pub fn board_faces(&self) -> impl Iterator<Item = &BoardFace> {
        self.board_faces
            .iter()
            .filter(|f| self.walls.contains_key(&f.wall))
    }

    /// A wall's layer composed by hand, if it is.
    pub fn board_face(&self, wall: WallId, layer: usize) -> Option<&BoardFace> {
        self.board_faces()
            .find(|f| f.wall == wall && f.layer == layer)
    }

    /// Composes a wall's layer by hand (or, with `None`, gives it back to
    /// its material's rule).
    pub fn set_board_face(
        &mut self,
        wall: WallId,
        layer: usize,
        face: Option<BoardFace>,
    ) -> Result<(), ModelError> {
        let w = self.wall(wall).ok_or(ModelError::UnknownElement(wall.0))?;
        let layers = self.wall_type(w.wall_type).map_or(0, |t| t.layers.len());
        if layer >= layers {
            return Err(ModelError::InvalidParameter(format!(
                "the wall has no layer {}",
                layer + 1
            )));
        }
        self.board_faces
            .retain(|f| !(f.wall == wall && f.layer == layer));
        if let Some(mut f) = face {
            if !f.is_valid() {
                return Err(ModelError::InvalidShape("a joint of no length"));
            }
            for m in &f.marks {
                if let Some(mat) = m.material {
                    if !self.materials.contains_key(&mat) {
                        return Err(ModelError::UnknownMaterial(mat));
                    }
                }
            }
            f.wall = wall;
            f.layer = layer;
            self.board_faces.push(f);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_composed_face_belongs_to_a_layer_of_its_wall() {
        let mut m = Model::new("T");
        let lv = m.add_level("PB", 0.0);
        let mat = m.add_material("Placa", "board", [200, 200, 200], CutPattern::Solid);
        let wt = m
            .add_wall_type(
                "Muro",
                WallFunction::Exterior,
                vec![Layer {
                    material: mat,
                    thickness: 0.2,
                    function: LayerFunction::Structure,
                }],
            )
            .unwrap();
        let w = m
            .add_wall(NewWall {
                wall_type: wt,
                level: lv,
                start: Point2::new(0.0, 0.0),
                end: Point2::new(6.0, 0.0),
                bulge: 0.0,
                reference: ReferenceLine::ExteriorFace,
                height: 2.7,
                base_offset: 0.0,
                top: WallTop::Unconnected,
            })
            .unwrap();
        let f = BoardFace {
            wall: w,
            layer: 0,
            joints: vec![(Point2::new(1.2, 0.0), Point2::new(1.2, 2.7))],
            marks: vec![PieceMark {
                at: Point2::new(0.5, 1.0),
                material: None,
                on_site: true,
            }],
            joint: Some(0.008),
        };
        m.set_board_face(w, 0, Some(f.clone())).unwrap();
        assert_eq!(m.board_face(w, 0), Some(&f));
        assert!(m.set_board_face(w, 99, Some(f.clone())).is_err());
        m.set_board_face(w, 0, None).unwrap();
        assert!(m.board_face(w, 0).is_none());
    }
}
