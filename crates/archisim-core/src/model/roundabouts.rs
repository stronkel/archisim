//! Roundabouts and squares where streets meet (U3): a circle on the plan
//! that the roads reaching it are cut at. Its ring is a carriageway round
//! an island (a garden or paving, with a mountable apron round it for
//! lorries); a square is paved all over, at the street's level. The
//! junction it makes with its roads is worked out each time
//! (`archisim_geom::crossroads`), like any other: what is stored is only
//! the circle and what it is made of.

use super::*;

/// What the island of a roundabout is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum IslandFinish {
    /// Soil with grass or plants.
    #[default]
    Garden,
    /// Paving (or a fountain's base).
    Paved,
}

impl IslandFinish {
    pub const ALL: [IslandFinish; 2] = [IslandFinish::Garden, IslandFinish::Paved];

    pub fn name(self) -> &'static str {
        match self {
            IslandFinish::Garden => "Ajardinada",
            IslandFinish::Paved => "Pavimentada",
        }
    }

    pub fn code(self) -> &'static str {
        match self {
            IslandFinish::Garden => "garden",
            IslandFinish::Paved => "paved",
        }
    }

    pub fn from_code(c: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|f| f.code() == c)
    }
}

/// A roundabout (or a square) where streets meet.
#[derive(Debug, Clone, PartialEq)]
pub struct Roundabout {
    pub id: RoundaboutId,
    pub guid: GlobalId,
    pub name: String,
    pub level: LevelId,
    pub centre: Point2,
    /// Radius of its outer kerb.
    pub radius: f64,
    /// Radius of its island (0: none).
    pub island: f64,
    /// Width of the mountable apron round the island.
    pub apron: f64,
    /// Height of its carriageway over the level.
    pub top_offset: f64,
    /// A square: paved all over, for people.
    pub plaza: bool,
    pub island_finish: IslandFinish,
    pub pavement: MaterialId,
    pub kerb_material: MaterialId,
    pub island_material: MaterialId,
}

/// Data needed to make a roundabout.
#[derive(Debug, Clone, PartialEq)]
pub struct NewRoundabout {
    pub name: String,
    pub level: LevelId,
    pub centre: Point2,
    pub radius: f64,
    pub island: f64,
    pub apron: f64,
    pub top_offset: f64,
    pub plaza: bool,
    pub island_finish: IslandFinish,
    pub pavement: MaterialId,
    pub kerb_material: MaterialId,
    pub island_material: MaterialId,
}

/// The usual roundabout: 15 m to its outer kerb, an island of 7 m with an
/// apron of 1,5 m (a ring of 6,5 m).
pub const ROUNDABOUT_RADIUS: f64 = 15.0;
pub const ROUNDABOUT_ISLAND: f64 = 7.0;
pub const ROUNDABOUT_APRON: f64 = 1.5;
/// The narrowest ring a roundabout may have.
pub const MIN_RING: f64 = 4.0;
/// Segments of its circles.
pub const CIRCLE_STEPS: usize = 64;

/// A circle as a polygon, counter-clockwise.
pub fn circle_of(c: Point2, r: f64, steps: usize) -> Vec<Point2> {
    (0..steps)
        .map(|k| {
            let t = std::f64::consts::TAU * k as f64 / steps as f64;
            Point2::new(c.x + r * t.cos(), c.y + r * t.sin())
        })
        .collect()
}

impl Roundabout {
    /// Width of its ring (the carriageway round the island).
    pub fn ring(&self) -> f64 {
        self.radius - self.island - self.apron
    }

    /// Its outer kerb's circle.
    pub fn outline(&self) -> Vec<Point2> {
        circle_of(self.centre, self.radius, CIRCLE_STEPS)
    }

    fn spec(&self) -> NewRoundabout {
        NewRoundabout {
            name: self.name.clone(),
            level: self.level,
            centre: self.centre,
            radius: self.radius,
            island: self.island,
            apron: self.apron,
            top_offset: self.top_offset,
            plaza: self.plaza,
            island_finish: self.island_finish,
            pavement: self.pavement,
            kerb_material: self.kerb_material,
            island_material: self.island_material,
        }
    }

    /// Its numbers as a text, for IFC: `radius island apron top plaza
    /// finish`.
    pub fn encode(&self) -> String {
        format!(
            "{} {} {} {} {} {}",
            self.radius,
            self.island,
            self.apron,
            self.top_offset,
            u8::from(self.plaza),
            self.island_finish.code()
        )
    }
}

impl Model {
    fn validate_roundabout(&self, r: &NewRoundabout) -> Result<(), ModelError> {
        if self.level(r.level).is_none() {
            return Err(ModelError::UnknownLevel(r.level));
        }
        for m in [r.pavement, r.kerb_material, r.island_material] {
            if self.material(m).is_none() {
                return Err(ModelError::UnknownMaterial(m));
            }
        }
        let ok = [r.radius, r.island, r.apron, r.top_offset]
            .iter()
            .all(|x| x.is_finite())
            && r.radius >= 3.0
            && r.island >= 0.0
            && r.apron >= 0.0
            && (r.plaza || r.radius - r.island - r.apron >= MIN_RING - EPSILON);
        if !ok {
            return Err(ModelError::InvalidShape(
                "a roundabout needs a ring of 4 m at least round its island",
            ));
        }
        Ok(())
    }

    pub fn add_roundabout(&mut self, spec: NewRoundabout) -> Result<RoundaboutId, ModelError> {
        self.validate_roundabout(&spec)?;
        let id = RoundaboutId(self.take_id());
        self.roundabouts.insert(
            id,
            Roundabout {
                id,
                guid: GlobalId::new(),
                name: spec.name,
                level: spec.level,
                centre: spec.centre,
                radius: spec.radius,
                island: spec.island,
                apron: spec.apron,
                top_offset: spec.top_offset,
                plaza: spec.plaza,
                island_finish: spec.island_finish,
                pavement: spec.pavement,
                kerb_material: spec.kerb_material,
                island_material: spec.island_material,
            },
        );
        Ok(id)
    }

    pub fn edit_roundabout(
        &mut self,
        id: RoundaboutId,
        edit: impl FnOnce(&mut Roundabout),
    ) -> Result<(), ModelError> {
        let mut r = self
            .roundabouts
            .get(&id)
            .cloned()
            .ok_or(ModelError::UnknownElement(id.0))?;
        edit(&mut r);
        self.validate_roundabout(&r.spec())?;
        self.roundabouts.insert(id, r);
        Ok(())
    }

    pub fn remove_roundabout(&mut self, id: RoundaboutId) -> Result<Roundabout, ModelError> {
        self.roundabouts
            .remove(&id)
            .ok_or(ModelError::UnknownElement(id.0))
    }

    pub fn roundabout(&self, id: RoundaboutId) -> Option<&Roundabout> {
        self.roundabouts.get(&id)
    }

    pub fn roundabouts(&self) -> impl Iterator<Item = &Roundabout> {
        self.roundabouts.values()
    }

    pub fn roundabouts_on_level(&self, level: LevelId) -> impl Iterator<Item = &Roundabout> {
        self.roundabouts.values().filter(move |r| r.level == level)
    }

    pub fn next_roundabout_name(&self) -> String {
        format!("Glorieta {}", self.roundabouts.len() + 1)
    }

    pub(super) fn restore_roundabouts(
        &mut self,
        list: Vec<Roundabout>,
        claim: &mut structure::Claim<'_>,
    ) -> Result<(), RestoreError> {
        for r in list {
            claim(r.id.0, Some(&r.guid))
                .and_then(|_| self.validate_roundabout(&r.spec()))
                .map_err(|error| RestoreError {
                    element: format!("roundabout {} «{}»", r.id.0, r.name),
                    error,
                })?;
            self.roundabouts.insert(r.id, r);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_roundabout_needs_its_ring() {
        let mut m = Model::new("P");
        let lv = m.add_level("PB", 0.0);
        let mat = m.add_material("Asfalto", "asphalt", [70, 70, 70], CutPattern::Solid);
        let spec = NewRoundabout {
            name: "G".into(),
            level: lv,
            centre: Point2::new(0.0, 0.0),
            radius: ROUNDABOUT_RADIUS,
            island: ROUNDABOUT_ISLAND,
            apron: ROUNDABOUT_APRON,
            top_offset: 0.0,
            plaza: false,
            island_finish: IslandFinish::Garden,
            pavement: mat,
            kerb_material: mat,
            island_material: mat,
        };
        let id = m.add_roundabout(spec.clone()).unwrap();
        assert!((m.roundabout(id).unwrap().ring() - 6.5).abs() < 1e-9);
        assert!(m.edit_roundabout(id, |r| r.island = 11.0).is_err());
        // A square: paved all over.
        assert!(m
            .edit_roundabout(id, |r| {
                r.plaza = true;
                r.island = 0.0;
            })
            .is_ok());
        assert_eq!(m.roundabouts_on_level(lv).count(), 1);
    }
}
