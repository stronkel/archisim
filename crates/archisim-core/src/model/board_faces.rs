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

impl Model {
    /// The walls a wall's face composed by hand can be copied to as it is:
    /// of the same type, as long (within 2 cm) and as high.
    pub fn walls_like(&self, wall: WallId) -> Vec<WallId> {
        let Some(w) = self.wall(wall) else {
            return Vec::new();
        };
        self.walls()
            .filter(|o| {
                o.id != wall
                    && o.wall_type == w.wall_type
                    && (o.length() - w.length()).abs() < 0.02
                    && (o.height - w.height).abs() < 0.02
                    && (o.base_offset - w.base_offset).abs() < 0.02
            })
            .map(|o| o.id)
            .collect()
    }

    /// Copies a wall's layer composed by hand to other walls, as it is or
    /// reflected (its start at their end). Returns how many took it.
    pub fn copy_board_face(
        &mut self,
        from: WallId,
        layer: usize,
        to: &[WallId],
        reflected: bool,
    ) -> Result<usize, ModelError> {
        let face = self
            .board_face(from, layer)
            .cloned()
            .ok_or(ModelError::InvalidParameter(
                "that layer is not composed by hand".into(),
            ))?;
        let mut n = 0;
        for &w in to {
            if w == from {
                continue;
            }
            let Some(len) = self.wall(w).map(|w| w.length()) else {
                continue;
            };
            let along = |p: Point2| {
                if reflected {
                    Point2::new(len - p.x, p.y)
                } else {
                    p
                }
            };
            let copy = BoardFace {
                wall: w,
                layer,
                joints: face
                    .joints
                    .iter()
                    .map(|(a, b)| (along(*a), along(*b)))
                    .collect(),
                marks: face
                    .marks
                    .iter()
                    .map(|k| PieceMark {
                        at: along(k.at),
                        ..k.clone()
                    })
                    .collect(),
                joint: face.joint,
            };
            self.set_board_face(w, layer, Some(copy))?;
            n += 1;
        }
        Ok(n)
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
        // Copied, reflected, to a wall like it.
        let w2 = m
            .add_wall(NewWall {
                wall_type: wt,
                level: lv,
                start: Point2::new(0.0, 3.0),
                end: Point2::new(6.0, 3.0),
                bulge: 0.0,
                reference: ReferenceLine::ExteriorFace,
                height: 2.7,
                base_offset: 0.0,
                top: WallTop::Unconnected,
            })
            .unwrap();
        assert_eq!(m.walls_like(w), vec![w2]);
        assert_eq!(m.copy_board_face(w, 0, &[w2], true).unwrap(), 1);
        let c = m.board_face(w2, 0).unwrap();
        assert!((c.joints[0].0.x - 4.8).abs() < 1e-9);
        assert!((c.marks[0].at.x - 5.5).abs() < 1e-9);
        m.set_board_face(w, 0, None).unwrap();
        assert!(m.board_face(w, 0).is_none());
    }
}

/// What a layer composed by hand in plan belongs to: a slab (floor or
/// roof) or a false ceiling.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PlanHost {
    Slab(SlabId),
    Ceiling(CeilingId),
}

/// A slab's or a ceiling's layer of boards composed by hand, in plan
/// (version 111): as a wall's, but its lines and marks are points of the
/// plan, x and y in m.
#[derive(Debug, Clone, PartialEq)]
pub struct BoardPlan {
    pub host: PlanHost,
    /// The layer of its type (from the top, 0…).
    pub layer: usize,
    pub joints: Vec<(Point2, Point2)>,
    pub marks: Vec<PieceMark>,
    /// The open joint between its pieces, m; `None`: its material's.
    pub joint: Option<f64>,
}

impl BoardPlan {
    pub fn is_valid(&self) -> bool {
        BoardFace {
            wall: WallId(0),
            layer: self.layer,
            joints: self.joints.clone(),
            marks: self.marks.clone(),
            joint: self.joint,
        }
        .is_valid()
    }
}

impl Model {
    /// The layers of slabs and ceilings in plan composed by hand, of the
    /// ones still there.
    pub fn board_plans(&self) -> impl Iterator<Item = &BoardPlan> {
        self.board_plans.iter().filter(|f| match f.host {
            PlanHost::Slab(s) => self.slabs.contains_key(&s),
            PlanHost::Ceiling(c) => self.ceiling(c).is_some(),
        })
    }

    /// A slab's or a ceiling's layer composed by hand, if it is.
    pub fn board_plan(&self, host: PlanHost, layer: usize) -> Option<&BoardPlan> {
        self.board_plans()
            .find(|f| f.host == host && f.layer == layer)
    }

    /// The layers of a slab's or a ceiling's type.
    pub fn plan_host_layers(&self, host: PlanHost) -> Option<&[Layer]> {
        let t = match host {
            PlanHost::Slab(s) => self.slab(s)?.slab_type,
            PlanHost::Ceiling(c) => self.ceiling(c)?.ceiling_type,
        };
        self.slab_type(t).map(|t| t.layers.as_slice())
    }

    /// The outline and holes of a slab or a ceiling, in plan.
    pub fn plan_host_outline(&self, host: PlanHost) -> Option<(&[Point2], &[Vec<Point2>])> {
        match host {
            PlanHost::Slab(s) => self
                .slab(s)
                .map(|s| (s.outline.as_slice(), s.holes.as_slice())),
            PlanHost::Ceiling(c) => self
                .ceiling(c)
                .map(|c| (c.outline.as_slice(), c.holes.as_slice())),
        }
    }

    /// Composes a slab's or a ceiling's layer by hand (or, with `None`,
    /// gives it back to its material's rule).
    pub fn set_board_plan(
        &mut self,
        host: PlanHost,
        layer: usize,
        face: Option<BoardPlan>,
    ) -> Result<(), ModelError> {
        let id = match host {
            PlanHost::Slab(s) => s.0,
            PlanHost::Ceiling(c) => c.0,
        };
        let layers = self
            .plan_host_layers(host)
            .ok_or(ModelError::UnknownElement(id))?
            .len();
        if layer >= layers {
            return Err(ModelError::InvalidParameter(format!(
                "the element has no layer {}",
                layer + 1
            )));
        }
        self.board_plans
            .retain(|f| !(f.host == host && f.layer == layer));
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
            f.host = host;
            f.layer = layer;
            self.board_plans.push(f);
        }
        Ok(())
    }
}
