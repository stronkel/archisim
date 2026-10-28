//! Phases of a refurbishment (Revit's "Fases", ArchiCAD's renovation
//! status): every element is new (the default), existing, or existing and
//! to be demolished. IFC: the `Status` property of the element's common
//! property set (`NEW`, `EXISTING`, `DEMOLISH`).
//!
//! The phase of an element is kept by its id in a table of the model, so
//! elements of any kind can have one without each knowing about it; an
//! element that is not in the table is new.

use super::*;

/// The phase of an element.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, PartialOrd, Ord)]
pub enum Phase {
    /// Built by the project (the default).
    #[default]
    New,
    /// There before the project and kept.
    Existing,
    /// There before the project and demolished by it.
    Demolished,
}

impl Phase {
    /// The value of IFC's `Status` property (`PEnum_ElementStatus`).
    pub fn ifc_status(self) -> &'static str {
        match self {
            Phase::New => "NEW",
            Phase::Existing => "EXISTING",
            Phase::Demolished => "DEMOLISH",
        }
    }

    pub fn from_ifc_status(s: &str) -> Option<Phase> {
        match s.trim().to_ascii_uppercase().as_str() {
            "NEW" => Some(Phase::New),
            "EXISTING" => Some(Phase::Existing),
            "DEMOLISH" | "DEMOLISHED" => Some(Phase::Demolished),
            _ => None,
        }
    }
}

impl Model {
    /// The phase of an element, by its id (walls, slabs, doors and
    /// windows, columns, beams).
    pub fn phase(&self, id: u64) -> Phase {
        self.phases.get(&id).copied().unwrap_or_default()
    }

    /// Whether an element with that id exists and can have a phase.
    fn phased(&self, id: u64) -> bool {
        self.walls.contains_key(&WallId(id))
            || self.slabs.contains_key(&SlabId(id))
            || self.openings.contains_key(&OpeningId(id))
            || self.columns.contains_key(&ColumnId(id))
            || self.beams.contains_key(&BeamId(id))
            || self.roofs.contains_key(&RoofId(id))
            || self.stairs.contains_key(&StairId(id))
            || self.curtain_walls.contains_key(&CurtainWallId(id))
            || self.railings.contains_key(&RailingId(id))
            || self.objects.contains_key(&ObjectId(id))
            || self.footings.contains_key(&FootingId(id))
            || self.pools.contains_key(&PoolId(id))
    }

    /// Sets an element's phase. The doors and windows of a wall follow
    /// it (a door can then be given a phase of its own: a new door in an
    /// existing wall).
    pub fn set_phase(&mut self, id: u64, phase: Phase) -> Result<(), ModelError> {
        if !self.phased(id) {
            return Err(ModelError::UnknownElement(id));
        }
        self.put_phase(id, phase);
        if self.walls.contains_key(&WallId(id)) {
            let hosted: Vec<u64> = self
                .openings
                .values()
                .filter(|o| o.wall == WallId(id))
                .map(|o| o.id.0)
                .collect();
            for o in hosted {
                self.put_phase(o, phase);
            }
        }
        Ok(())
    }

    /// Sets the phase of one element only (its doors and windows keep
    /// theirs), as read from a file.
    pub fn set_own_phase(&mut self, id: u64, phase: Phase) -> Result<(), ModelError> {
        if !self.phased(id) {
            return Err(ModelError::UnknownElement(id));
        }
        self.put_phase(id, phase);
        Ok(())
    }

    fn put_phase(&mut self, id: u64, phase: Phase) {
        if phase == Phase::New {
            self.phases.remove(&id);
        } else {
            self.phases.insert(id, phase);
        }
    }

    /// Whether any element is not new (the project is a refurbishment).
    pub fn has_phases(&self) -> bool {
        self.phases.keys().any(|id| self.phased(*id))
    }

    /// New elements get the phase being drawn in (see
    /// [`Model::draw_phase`]).
    pub(super) fn stamp_phase(&mut self, id: u64) {
        let p = self.draw_phase;
        self.put_phase(id, p);
    }

    /// The phases worth saving: those of elements that exist and are not
    /// new, by id.
    pub fn phase_table(&self) -> Vec<(u64, Phase)> {
        self.phases
            .iter()
            .filter(|(id, p)| **p != Phase::New && self.phased(**id))
            .map(|(id, p)| (*id, *p))
            .collect()
    }

    pub(super) fn restore_phases(&mut self, table: Vec<(u64, Phase)>) -> Result<(), RestoreError> {
        for (id, p) in table {
            self.set_own_phase(id, p).map_err(|error| RestoreError {
                element: format!("phase of element {id}"),
                error,
            })?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn phases_follow_elements() {
        let mut m = Model::new("t");
        let l = m.add_level("PB", 0.0);
        let mat = m.add_material("Ladrillo", "brick", [200, 100, 80], CutPattern::Diagonal);
        let wt = m
            .add_wall_type(
                "Muro",
                WallFunction::Exterior,
                vec![Layer {
                    material: mat,
                    thickness: 0.24,
                    function: LayerFunction::Structure,
                }],
            )
            .unwrap();
        let w = m
            .add_wall(NewWall {
                wall_type: wt,
                level: l,
                start: Point2::new(0.0, 0.0),
                end: Point2::new(5.0, 0.0),
                reference: ReferenceLine::Center,
                height: 3.0,
                base_offset: 0.0,
                bulge: 0.0,
                top: WallTop::Unconnected,
            })
            .unwrap();
        assert_eq!(m.phase(w.0), Phase::New);
        assert!(!m.has_phases());
        m.set_phase(w.0, Phase::Existing).unwrap();
        assert_eq!(m.phase(w.0), Phase::Existing);
        assert!(m.has_phases());
        assert_eq!(
            m.set_phase(9999, Phase::Existing),
            Err(ModelError::UnknownElement(9999))
        );
        // Drawn while surveying: existing.
        m.draw_phase = Phase::Existing;
        let w2 = m
            .add_wall(NewWall {
                wall_type: wt,
                level: l,
                start: Point2::new(0.0, 3.0),
                end: Point2::new(5.0, 3.0),
                reference: ReferenceLine::Center,
                height: 3.0,
                base_offset: 0.0,
                bulge: 0.0,
                top: WallTop::Unconnected,
            })
            .unwrap();
        assert_eq!(m.phase(w2.0), Phase::Existing);
        m.draw_phase = Phase::New;
        m.set_phase(w2.0, Phase::Demolished).unwrap();
        let back = Model::from_parts(m.to_parts()).unwrap();
        assert_eq!(back.phase(w2.0), Phase::Demolished);
        assert_eq!(back.phase_table(), m.phase_table());
        m.remove_wall(w2).unwrap();
        assert_eq!(m.phase_table(), vec![(w.0, Phase::Existing)]);
        assert_eq!(Phase::from_ifc_status("demolish"), Some(Phase::Demolished));
    }
}
