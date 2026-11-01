//! The discipline of each element (`docs/INTERFAZ.md` §8): architecture,
//! structure, services or site. Most elements take it from what they are
//! (a column is structure, the terrain is site); walls and slabs from their
//! type: they are structure when a structural layer is of concrete, steel
//! or timber, and architecture otherwise (a brick façade of a framed
//! building closes, it does not carry). Any element can be given another
//! by hand; that is kept by its id in a table of the model, as phases are.
//!
//! The interface dims what is not of the discipline being worked in.

use super::*;
use std::collections::BTreeSet;

/// What an element belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum ElementDiscipline {
    Architecture,
    Structure,
    Services,
    Site,
}

impl ElementDiscipline {
    pub const ALL: [ElementDiscipline; 4] = [
        ElementDiscipline::Architecture,
        ElementDiscipline::Structure,
        ElementDiscipline::Services,
        ElementDiscipline::Site,
    ];

    /// Its code in files.
    pub fn code(self) -> &'static str {
        match self {
            ElementDiscipline::Architecture => "architecture",
            ElementDiscipline::Structure => "structure",
            ElementDiscipline::Services => "services",
            ElementDiscipline::Site => "site",
        }
    }

    pub fn from_code(c: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|d| d.code() == c)
    }

    pub fn name(self) -> &'static str {
        match self {
            ElementDiscipline::Architecture => "Arquitectura",
            ElementDiscipline::Structure => "Estructura",
            ElementDiscipline::Services => "Instalaciones",
            ElementDiscipline::Site => "Urbanismo",
        }
    }
}

/// Which elements a view draws at full strength; the rest in grey, as a
/// reference to work on (a structural plan with the architecture in
/// grey). A style of the view, so it prints too.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum DisciplineFocus {
    /// Everything at full strength.
    #[default]
    None,
    /// The elements of one discipline; the others in grey.
    Only(ElementDiscipline),
    /// Every element in grey (the concept phase: the building as
    /// reference for the volumes).
    Volumes,
}

impl DisciplineFocus {
    /// Whether an element of discipline `d` is drawn in grey.
    pub fn dims(self, d: ElementDiscipline) -> bool {
        match self {
            DisciplineFocus::None => false,
            DisciplineFocus::Only(f) => d != f,
            DisciplineFocus::Volumes => true,
        }
    }

    /// Its code in files.
    pub fn code(self) -> &'static str {
        match self {
            DisciplineFocus::None => "none",
            DisciplineFocus::Only(ElementDiscipline::Architecture) => "architecture",
            DisciplineFocus::Only(ElementDiscipline::Structure) => "structure",
            DisciplineFocus::Only(ElementDiscipline::Services) => "services",
            DisciplineFocus::Only(ElementDiscipline::Site) => "site",
            DisciplineFocus::Volumes => "volumes",
        }
    }

    pub fn from_code(c: &str) -> Option<Self> {
        Some(match c {
            "none" => DisciplineFocus::None,
            "architecture" => DisciplineFocus::Only(ElementDiscipline::Architecture),
            "structure" => DisciplineFocus::Only(ElementDiscipline::Structure),
            "services" => DisciplineFocus::Only(ElementDiscipline::Services),
            "site" => DisciplineFocus::Only(ElementDiscipline::Site),
            "volumes" => DisciplineFocus::Volumes,
            _ => return None,
        })
    }

    /// Its name in the interface.
    pub fn label(self) -> &'static str {
        match self {
            DisciplineFocus::None => "Ninguna",
            DisciplineFocus::Only(ElementDiscipline::Architecture) => "Resaltar arquitectura",
            DisciplineFocus::Only(ElementDiscipline::Structure) => "Resaltar estructura",
            DisciplineFocus::Only(ElementDiscipline::Services) => "Resaltar instalaciones",
            DisciplineFocus::Only(ElementDiscipline::Site) => "Resaltar urbanismo",
            DisciplineFocus::Volumes => "Todo en gris",
        }
    }

    pub const ALL: [DisciplineFocus; 6] = [
        DisciplineFocus::None,
        DisciplineFocus::Only(ElementDiscipline::Architecture),
        DisciplineFocus::Only(ElementDiscipline::Structure),
        DisciplineFocus::Only(ElementDiscipline::Services),
        DisciplineFocus::Only(ElementDiscipline::Site),
        DisciplineFocus::Volumes,
    ];
}

/// Materials that make a structural layer structure (and not a brick
/// wall that only closes).
const STRUCTURAL_MATERIALS: [&str; 5] = ["concrete", "steel", "timber", "wood", "metal"];

impl Model {
    fn carries(&self, layers: &[Layer]) -> bool {
        layers.iter().any(|l| {
            l.function == LayerFunction::Structure
                && self
                    .material(l.material)
                    .is_some_and(|m| STRUCTURAL_MATERIALS.contains(&m.category.as_str()))
        })
    }

    /// The discipline an element has from what it is, whatever it was
    /// given by hand; `None` for what is not an element of the model
    /// (annotations, types…).
    pub fn natural_discipline(&self, id: u64) -> Option<ElementDiscipline> {
        use ElementDiscipline::*;
        if let Some(w) = self.walls.get(&WallId(id)) {
            let carries = self
                .wall_type(w.wall_type)
                .is_some_and(|t| self.carries(&t.layers));
            return Some(if carries { Structure } else { Architecture });
        }
        if let Some(s) = self.slabs.get(&SlabId(id)) {
            let carries = self
                .slab_type(s.slab_type)
                .is_some_and(|t| self.carries(&t.layers));
            return Some(if carries { Structure } else { Architecture });
        }
        if self.columns.contains_key(&ColumnId(id))
            || self.beams.contains_key(&BeamId(id))
            || self.footings.contains_key(&FootingId(id))
        {
            return Some(Structure);
        }
        if self.pools.contains_key(&PoolId(id)) || self.elevators.contains_key(&ElevatorId(id)) {
            return Some(Architecture);
        }
        if self.runs.contains_key(&RunId(id)) || self.devices.contains_key(&DeviceId(id)) {
            return Some(Services);
        }
        if self.terrains.contains_key(&TerrainId(id))
            || self.plots.contains_key(&PlotId(id))
            || self.roads.contains_key(&RoadId(id))
            || self.roundabouts.contains_key(&RoundaboutId(id))
            || self.markings.contains_key(&MarkingId(id))
        {
            return Some(Site);
        }
        if let Some(o) = self.objects.get(&ObjectId(id)) {
            let category = self
                .object_type(o.object_type)
                .and_then(|t| self.family(t.family))
                .map(|f| f.def().category.clone())
                .unwrap_or_default();
            return Some(
                if matches!(category.as_str(), "site" | "street" | "nature" | "vehicle") {
                    Site
                } else {
                    Architecture
                },
            );
        }
        if self.openings.contains_key(&OpeningId(id))
            || self.roofs.contains_key(&RoofId(id))
            || self.stairs.contains_key(&StairId(id))
            || self.curtain_walls.contains_key(&CurtainWallId(id))
            || self.railings.contains_key(&RailingId(id))
            || self.rooms.contains_key(&RoomId(id))
        {
            return Some(Architecture);
        }
        None
    }

    /// The discipline of an element: the one given by hand, or its own.
    pub fn discipline(&self, id: u64) -> Option<ElementDiscipline> {
        let natural = self.natural_discipline(id)?;
        Some(self.disciplines.get(&id).copied().unwrap_or(natural))
    }

    /// Gives an element a discipline by hand; `None` gives it back its own.
    pub fn set_discipline(
        &mut self,
        id: u64,
        discipline: Option<ElementDiscipline>,
    ) -> Result<(), ModelError> {
        let natural = self
            .natural_discipline(id)
            .ok_or(ModelError::UnknownElement(id))?;
        match discipline {
            Some(d) if d != natural => {
                self.disciplines.insert(id, d);
            }
            _ => {
                self.disciplines.remove(&id);
            }
        }
        Ok(())
    }

    /// Whether a view hiding `hidden` leaves the element `id` out.
    pub fn hidden_by_discipline(&self, hidden: &BTreeSet<ElementDiscipline>, id: u64) -> bool {
        !hidden.is_empty() && self.discipline(id).is_some_and(|d| hidden.contains(&d))
    }

    /// Whether a view with `focus` draws the element `id` in grey.
    pub fn dimmed(&self, focus: DisciplineFocus, id: u64) -> bool {
        focus != DisciplineFocus::None && self.discipline(id).is_some_and(|d| focus.dims(d))
    }

    /// Whether an element's discipline was given by hand.
    pub fn discipline_set_by_hand(&self, id: u64) -> bool {
        self.disciplines.contains_key(&id) && self.natural_discipline(id).is_some()
    }

    /// The disciplines given by hand that are worth saving, by id.
    pub fn discipline_table(&self) -> Vec<(u64, ElementDiscipline)> {
        self.disciplines
            .iter()
            .filter(|(id, d)| self.natural_discipline(**id).is_some_and(|n| n != **d))
            .map(|(id, d)| (*id, *d))
            .collect()
    }

    pub(super) fn restore_disciplines(
        &mut self,
        table: Vec<(u64, ElementDiscipline)>,
    ) -> Result<(), RestoreError> {
        for (id, d) in table {
            self.set_discipline(id, Some(d))
                .map_err(|error| RestoreError {
                    element: format!("discipline of element {id}"),
                    error,
                })?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn wall_of(m: &mut Model, l: LevelId, category: &str, y: f64) -> WallId {
        let mat = m.add_material(category, category, [200, 200, 200], CutPattern::Diagonal);
        let t = m
            .add_wall_type(
                format!("Muro {category}"),
                WallFunction::Exterior,
                vec![Layer {
                    material: mat,
                    thickness: 0.2,
                    function: LayerFunction::Structure,
                }],
            )
            .unwrap();
        m.add_wall(NewWall {
            wall_type: t,
            level: l,
            start: Point2::new(0.0, y),
            end: Point2::new(5.0, y),
            reference: ReferenceLine::Center,
            height: 3.0,
            base_offset: 0.0,
            bulge: 0.0,
            top: WallTop::Unconnected,
        })
        .unwrap()
    }

    #[test]
    fn disciplines_from_what_elements_are_and_by_hand() {
        let mut m = Model::new("t");
        let l = m.add_level("PB", 0.0);
        let brick = wall_of(&mut m, l, "brick", 0.0);
        let concrete = wall_of(&mut m, l, "concrete", 3.0);
        assert_eq!(m.discipline(brick.0), Some(ElementDiscipline::Architecture));
        assert_eq!(m.discipline(concrete.0), Some(ElementDiscipline::Structure));
        assert_eq!(m.discipline(9999), None);
        // By hand: a brick bearing wall.
        m.set_discipline(brick.0, Some(ElementDiscipline::Structure))
            .unwrap();
        assert_eq!(m.discipline(brick.0), Some(ElementDiscipline::Structure));
        assert!(m.discipline_set_by_hand(brick.0));
        let back = Model::from_parts(m.to_parts()).unwrap();
        assert_eq!(back.discipline(brick.0), Some(ElementDiscipline::Structure));
        assert_eq!(back.discipline_table(), m.discipline_table());
        // Back to its own.
        m.set_discipline(brick.0, None).unwrap();
        assert!(!m.discipline_set_by_hand(brick.0));
        assert!(m.discipline_table().is_empty());
        assert_eq!(
            m.set_discipline(9999, Some(ElementDiscipline::Site)),
            Err(ModelError::UnknownElement(9999))
        );
    }
}
