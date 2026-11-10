//! False ceilings (M7, `docs/MODULAR.md`): a board hung under the floor or
//! the roof above a room, drawn by its outline on the level of the room.
//! Its type is a slab type with the function [`SlabFunction::Ceiling`]: its
//! layers, from the top down to the face seen from the room (the board
//! last), are edited like those of any slab. It hangs: it does not bear,
//! does not take what crosses it and is not the floor of anything, so it
//! is an element of its own and not a slab. IFC: `IfcCovering` `.CEILING.`.

use super::*;

/// The height of a false ceiling's face over its level when none is said.
pub const CEILING_HEIGHT: f64 = 2.5;

/// A false ceiling.
#[derive(Debug, Clone, PartialEq)]
pub struct Ceiling {
    pub id: CeilingId,
    pub guid: GlobalId,
    /// A slab type whose function is [`SlabFunction::Ceiling`].
    pub ceiling_type: SlabTypeId,
    pub level: LevelId,
    /// Outline in plan, counter-clockwise, without repeating the first point.
    pub outline: Vec<Point2>,
    /// Holes in it (hatches, light fittings, a skylight), each inside the
    /// outline, counter-clockwise.
    pub holes: Vec<Vec<Point2>>,
    /// Height of its face (the underside, seen from the room) over the
    /// level.
    pub height: f64,
}

/// Data needed to make a false ceiling.
#[derive(Debug, Clone, PartialEq)]
pub struct NewCeiling {
    pub ceiling_type: SlabTypeId,
    pub level: LevelId,
    pub outline: Vec<Point2>,
    pub height: f64,
}

impl Ceiling {
    /// Its area seen from the room, holes left out.
    pub fn area(&self) -> f64 {
        signed_area(&self.outline).abs()
            - self.holes.iter().map(|h| signed_area(h).abs()).sum::<f64>()
    }
}

impl Model {
    fn check_ceiling_type(&self, t: SlabTypeId) -> Result<(), ModelError> {
        match self.slab_types.get(&t) {
            None => Err(ModelError::UnknownSlabType(t)),
            Some(st) if st.function != SlabFunction::Ceiling => Err(ModelError::InvalidShape(
                "a false ceiling needs a type of false ceiling",
            )),
            Some(st) if st.layers.is_empty() => Err(ModelError::InvalidShape(
                "a false ceiling needs a type with layers",
            )),
            Some(_) => Ok(()),
        }
    }

    pub fn add_ceiling(&mut self, spec: NewCeiling) -> Result<CeilingId, ModelError> {
        if !self.levels.contains_key(&spec.level) {
            return Err(ModelError::UnknownLevel(spec.level));
        }
        self.check_ceiling_type(spec.ceiling_type)?;
        finite(spec.height, "ceiling height")?;
        let outline = Self::normalized_outline(spec.outline)?;
        let id = CeilingId(self.take_id());
        self.ceilings.insert(
            id,
            Ceiling {
                id,
                guid: GlobalId::new(),
                ceiling_type: spec.ceiling_type,
                level: spec.level,
                outline,
                holes: Vec::new(),
                height: spec.height,
            },
        );
        self.stamp_phase(id.0);
        Ok(id)
    }

    /// Changes a false ceiling: its type, level, outline, holes or height,
    /// checked as when it was made.
    pub fn edit_ceiling(
        &mut self,
        id: CeilingId,
        edit: impl FnOnce(&mut Ceiling),
    ) -> Result<(), ModelError> {
        let mut c = self
            .ceilings
            .get(&id)
            .cloned()
            .ok_or(ModelError::UnknownElement(id.0))?;
        edit(&mut c);
        self.validate_ceiling(&mut c)?;
        self.ceilings.insert(id, c);
        Ok(())
    }

    fn validate_ceiling(&self, c: &mut Ceiling) -> Result<(), ModelError> {
        if !self.levels.contains_key(&c.level) {
            return Err(ModelError::UnknownLevel(c.level));
        }
        self.check_ceiling_type(c.ceiling_type)?;
        finite(c.height, "ceiling height")?;
        c.outline = Self::normalized_outline(std::mem::take(&mut c.outline))?;
        let holes = std::mem::take(&mut c.holes)
            .into_iter()
            .map(|h| Self::normalized_outline(h).map_err(|_| ModelError::InvalidHole))
            .collect::<Result<Vec<_>, _>>()?;
        Self::check_holes(&c.outline, &holes)?;
        c.holes = holes;
        Ok(())
    }

    pub fn remove_ceiling(&mut self, id: CeilingId) -> Result<Ceiling, ModelError> {
        self.ceilings
            .remove(&id)
            .ok_or(ModelError::UnknownElement(id.0))
    }

    pub fn ceiling(&self, id: CeilingId) -> Option<&Ceiling> {
        self.ceilings.get(&id)
    }

    pub fn ceilings(&self) -> impl Iterator<Item = &Ceiling> {
        self.ceilings.values()
    }

    pub fn ceilings_on_level(&self, level: LevelId) -> impl Iterator<Item = &Ceiling> {
        self.ceilings.values().filter(move |c| c.level == level)
    }

    /// The types false ceilings can have.
    pub fn ceiling_types(&self) -> impl Iterator<Item = &SlabType> {
        self.slab_types
            .values()
            .filter(|t| t.function == SlabFunction::Ceiling)
    }

    pub(super) fn restore_ceilings(
        &mut self,
        list: Vec<Ceiling>,
        claim: &mut structure::Claim<'_>,
    ) -> Result<(), RestoreError> {
        for mut c in list {
            let id = c.id;
            claim(id.0, Some(&c.guid))
                .and_then(|_| self.validate_ceiling(&mut c))
                .map_err(|error| RestoreError {
                    element: format!("false ceiling {}", id.0),
                    error,
                })?;
            self.ceilings.insert(id, c);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn square(s: f64) -> Vec<Point2> {
        vec![
            Point2::new(0.0, 0.0),
            Point2::new(s, 0.0),
            Point2::new(s, s),
            Point2::new(0.0, s),
        ]
    }

    #[test]
    fn a_false_ceiling_needs_a_ceiling_type() {
        let mut m = Model::new("P");
        let lv = m.add_level("PB", 0.0);
        m.add_level("P1", 3.0);
        let plaster = m.add_material(
            "Yeso laminado",
            "plaster",
            [240, 240, 240],
            CutPattern::Solid,
        );
        let board = Layer {
            material: plaster,
            thickness: 0.015,
            function: LayerFunction::Finish,
        };
        let floor = m
            .add_slab_type("Forjado", SlabFunction::Floor, vec![board.clone()])
            .unwrap();
        let ft = m
            .add_slab_type("Techo de placa", SlabFunction::Ceiling, vec![board])
            .unwrap();
        let spec = NewCeiling {
            ceiling_type: floor,
            level: lv,
            outline: square(4.0),
            height: CEILING_HEIGHT,
        };
        assert!(m.add_ceiling(spec.clone()).is_err());
        let id = m
            .add_ceiling(NewCeiling {
                ceiling_type: ft,
                ..spec
            })
            .unwrap();
        assert_eq!(m.slab_type_users(ft), 1);
        assert!(m.remove_slab_type(ft).is_err());
        // A hatch in it, and a hole outside it refused.
        let hatch = vec![
            Point2::new(1.0, 1.0),
            Point2::new(1.6, 1.0),
            Point2::new(1.6, 1.6),
            Point2::new(1.0, 1.6),
        ];
        m.edit_ceiling(id, |c| c.holes = vec![hatch]).unwrap();
        assert!((m.ceiling(id).unwrap().area() - (16.0 - 0.36)).abs() < 1e-9);
        let outside: Vec<Point2> = square(1.0)
            .into_iter()
            .map(|p| Point2::new(p.x + 10.0, p.y))
            .collect();
        assert!(m.edit_ceiling(id, |c| c.holes = vec![outside]).is_err());
        assert_eq!(m.ceilings_on_level(lv).count(), 1);
        assert_eq!(m.discipline(id.0), Some(ElementDiscipline::Architecture));
        // Gone with its level.
        m.remove_level(lv).unwrap();
        assert_eq!(m.ceilings().count(), 0);
    }
}
