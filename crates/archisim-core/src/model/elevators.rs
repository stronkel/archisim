//! Lifts (H4): the lift of a building as one element, from its lowest
//! stop to its highest, with its shaft, the pit under the lowest stop, the
//! overrun over the highest, a landing door at each stop and the car.
//!
//! An [`Elevator`] stands on its lowest level; it stops at every level up
//! to `top` (the levels between them, by their height). Its shaft is
//! `shaft_w` × `shaft_d` inside, centred on `position`, the doors on its
//! `+y` side turned by `rotation`; with `wall` > 0 the element builds the
//! shaft's walls, with 0 they are drawn with walls apart (and the doors
//! go through them). With `cut_slabs`, the slabs of the levels it crosses
//! get a hole of the shaft's outside ([`Model::sync_elevator_holes`]),
//! which follows it when it moves and goes when it goes. IFC: an
//! `IfcTransportElement` (`ELEVATOR`), its shaft an `IfcWall`.

use super::*;

/// A lift of the building.
#[derive(Debug, Clone, PartialEq)]
pub struct Elevator {
    pub id: ElevatorId,
    pub guid: GlobalId,
    pub name: String,
    /// The lowest stop.
    pub level: LevelId,
    /// The highest stop.
    pub top: LevelId,
    /// The centre of the shaft's inside.
    pub position: Point2,
    /// Turn, radians: the landing doors look to the turned `+y`.
    pub rotation: f64,
    pub shaft_w: f64,
    pub shaft_d: f64,
    /// The shaft's walls; 0: drawn with walls apart.
    pub wall: f64,
    pub pit: f64,
    pub overrun: f64,
    /// The car's inside.
    pub car_w: f64,
    pub car_d: f64,
    pub car_h: f64,
    /// The doors' clear width and height.
    pub door_w: f64,
    pub door_h: f64,
    /// Rated load, kg, and persons.
    pub load: u32,
    /// The level the car is drawn at; `None`, the lowest.
    pub car_at: Option<LevelId>,
    /// Whether it opens its holes in the slabs it crosses.
    pub cut_slabs: bool,
    /// The material of its shaft; `None`, the plain concrete grey.
    pub material: Option<MaterialId>,
}

/// Data needed to make a lift.
#[derive(Debug, Clone, PartialEq)]
pub struct NewElevator {
    pub name: String,
    pub level: LevelId,
    pub top: LevelId,
    pub position: Point2,
    pub rotation: f64,
    pub shaft_w: f64,
    pub shaft_d: f64,
    pub wall: f64,
    pub pit: f64,
    pub overrun: f64,
    pub car_w: f64,
    pub car_d: f64,
    pub car_h: f64,
    pub door_w: f64,
    pub door_h: f64,
    pub load: u32,
    pub car_at: Option<LevelId>,
    pub cut_slabs: bool,
    pub material: Option<MaterialId>,
}

/// The usual lifts (EN 81-20 and EN 81-70): name, load, persons, car
/// inside (width, depth), door, shaft inside (width, depth).
pub struct ElevatorPreset {
    pub name: &'static str,
    pub load: u32,
    pub persons: u32,
    pub car: (f64, f64),
    pub door: f64,
    pub shaft: (f64, f64),
}

pub const ELEVATOR_PRESETS: [ElevatorPreset; 5] = [
    ElevatorPreset {
        name: "450 kg · 6 personas (accesible tipo 1)",
        load: 450,
        persons: 6,
        car: (1.0, 1.25),
        door: 0.8,
        shaft: (1.55, 1.6),
    },
    ElevatorPreset {
        name: "630 kg · 8 personas (accesible tipo 2)",
        load: 630,
        persons: 8,
        car: (1.1, 1.4),
        door: 0.9,
        shaft: (1.65, 1.75),
    },
    ElevatorPreset {
        name: "1000 kg · 13 personas",
        load: 1000,
        persons: 13,
        car: (1.1, 2.1),
        door: 0.9,
        shaft: (1.65, 2.45),
    },
    ElevatorPreset {
        name: "1275 kg · 17 personas (accesible tipo 3)",
        load: 1275,
        persons: 17,
        car: (2.0, 1.4),
        door: 1.1,
        shaft: (2.6, 1.75),
    },
    ElevatorPreset {
        name: "1600 kg · montacamillas",
        load: 1600,
        persons: 21,
        car: (1.4, 2.4),
        door: 1.1,
        shaft: (2.1, 2.8),
    },
];

impl NewElevator {
    /// A lift of the second preset (630 kg, 8 persons), with a pit of
    /// 1,20 m and an overrun of 3,60 m, its walls of 20 cm.
    pub fn new(level: LevelId, top: LevelId, position: Point2) -> Self {
        let p = &ELEVATOR_PRESETS[1];
        NewElevator {
            name: String::new(),
            level,
            top,
            position,
            rotation: 0.0,
            shaft_w: p.shaft.0,
            shaft_d: p.shaft.1,
            wall: 0.2,
            pit: 1.2,
            overrun: 3.6,
            car_w: p.car.0,
            car_d: p.car.1,
            car_h: 2.2,
            door_w: p.door,
            door_h: 2.0,
            load: p.load,
            car_at: None,
            cut_slabs: true,
            material: None,
        }
    }

    /// Its sizes from a preset.
    pub fn with_preset(mut self, p: &ElevatorPreset) -> Self {
        self.car_w = p.car.0;
        self.car_d = p.car.1;
        self.door_w = p.door;
        self.shaft_w = p.shaft.0;
        self.shaft_d = p.shaft.1;
        self.load = p.load;
        self
    }
}

impl Elevator {
    pub fn spec(&self) -> NewElevator {
        NewElevator {
            name: self.name.clone(),
            level: self.level,
            top: self.top,
            position: self.position,
            rotation: self.rotation,
            shaft_w: self.shaft_w,
            shaft_d: self.shaft_d,
            wall: self.wall,
            pit: self.pit,
            overrun: self.overrun,
            car_w: self.car_w,
            car_d: self.car_d,
            car_h: self.car_h,
            door_w: self.door_w,
            door_h: self.door_h,
            load: self.load,
            car_at: self.car_at,
            cut_slabs: self.cut_slabs,
            material: self.material,
        }
    }

    /// The persons of its load (EN 81-20: 75 kg each).
    pub fn persons(&self) -> u32 {
        self.load / 75
    }

    /// A point of the lift's axes (`x` across, `y` towards the doors) in
    /// plan.
    pub fn point_at(&self, x: f64, y: f64) -> Point2 {
        let (c, s) = (self.rotation.cos(), self.rotation.sin());
        Point2::new(
            self.position.x + x * c - y * s,
            self.position.y + x * s + y * c,
        )
    }

    fn rect(&self, hw: f64, hd: f64) -> Vec<Point2> {
        [(-hw, -hd), (hw, -hd), (hw, hd), (-hw, hd)]
            .into_iter()
            .map(|(x, y)| self.point_at(x, y))
            .collect()
    }

    /// The inside of the shaft, counter-clockwise.
    pub fn inside(&self) -> Vec<Point2> {
        self.rect(self.shaft_w / 2.0, self.shaft_d / 2.0)
    }

    /// The outside of the shaft (its walls' outer faces), counter-clockwise.
    pub fn outside(&self) -> Vec<Point2> {
        self.rect(
            self.shaft_w / 2.0 + self.wall,
            self.shaft_d / 2.0 + self.wall,
        )
    }

    /// What the slabs lose: the shaft's outside, or its inside when its
    /// walls are drawn apart.
    pub fn hole(&self) -> Vec<Point2> {
        self.outside()
    }

    /// Where the car is, from the shaft's centre: centred across, its
    /// front 5 cm behind the doors.
    pub fn car_centre_y(&self) -> f64 {
        (self.shaft_d / 2.0 - 0.05 - self.car_d / 2.0).max(-self.shaft_d / 2.0 + self.car_d / 2.0)
    }
}

impl Model {
    /// The levels a lift stops at, lowest first: its own and every level
    /// up to its top, by their height.
    pub fn elevator_stops(&self, id: ElevatorId) -> Vec<LevelId> {
        let Some(e) = self.elevators.get(&id) else {
            return Vec::new();
        };
        let (Some(lo), Some(hi)) = (self.level(e.level), self.level(e.top)) else {
            return Vec::new();
        };
        let (z0, z1) = (lo.elevation, hi.elevation);
        let mut v: Vec<&Level> = self
            .levels()
            .into_iter()
            .filter(|l| l.elevation >= z0 - EPSILON && l.elevation <= z1 + EPSILON)
            .collect();
        v.sort_by(|a, b| a.elevation.total_cmp(&b.elevation));
        v.into_iter().map(|l| l.id).collect()
    }

    /// Height of each stop over the lowest.
    pub fn elevator_stop_heights(&self, id: ElevatorId) -> Vec<f64> {
        let stops = self.elevator_stops(id);
        let z0 = stops
            .first()
            .and_then(|l| self.level(*l))
            .map_or(0.0, |l| l.elevation);
        stops
            .iter()
            .filter_map(|l| self.level(*l))
            .map(|l| l.elevation - z0)
            .collect()
    }

    /// The lift's model in its axes ([`archisim_props::fine::lift`]).
    pub fn elevator_spec(&self, id: ElevatorId) -> Option<archisim_props::fine::lift::LiftSpec> {
        let e = self.elevators.get(&id)?;
        let stops = self.elevator_stops(id);
        let car_stop = e
            .car_at
            .and_then(|l| stops.iter().position(|s| *s == l))
            .unwrap_or(0);
        Some(archisim_props::fine::lift::LiftSpec {
            shaft_w: e.shaft_w,
            shaft_d: e.shaft_d,
            wall: e.wall,
            pit: e.pit,
            overrun: e.overrun,
            stops: self.elevator_stop_heights(id),
            car_w: e.car_w,
            car_d: e.car_d,
            car_h: e.car_h,
            door_w: e.door_w,
            door_h: e.door_h,
            car_stop,
        })
    }

    fn validate_elevator(&self, e: &NewElevator) -> Result<(), ModelError> {
        for l in [e.level, e.top] {
            if !self.levels.contains_key(&l) {
                return Err(ModelError::UnknownLevel(l));
            }
        }
        if let Some(l) = e.car_at {
            if !self.levels.contains_key(&l) {
                return Err(ModelError::UnknownLevel(l));
            }
        }
        if let Some(m) = e.material {
            if !self.materials.contains_key(&m) {
                return Err(ModelError::UnknownMaterial(m));
            }
        }
        finite_point(e.position, "lift position")?;
        finite(e.rotation, "lift rotation")?;
        for (v, what) in [
            (e.shaft_w, "lift shaft width"),
            (e.shaft_d, "lift shaft depth"),
            (e.car_w, "lift car width"),
            (e.car_d, "lift car depth"),
            (e.car_h, "lift car height"),
            (e.door_w, "lift door width"),
            (e.door_h, "lift door height"),
        ] {
            finite(v, what)?;
            if v <= EPSILON {
                return Err(ModelError::NonPositive(what));
            }
        }
        for (v, what) in [
            (e.wall, "lift shaft wall"),
            (e.pit, "lift pit"),
            (e.overrun, "lift overrun"),
        ] {
            finite(v, what)?;
            if v < 0.0 {
                return Err(ModelError::NonPositive(what));
            }
        }
        if e.car_w > e.shaft_w - 0.1 || e.car_d > e.shaft_d - 0.05 {
            return Err(ModelError::InvalidShape(
                "the lift's car does not fit in its shaft",
            ));
        }
        if e.door_w > e.car_w || e.door_h > e.car_h {
            return Err(ModelError::InvalidShape(
                "the lift's door is wider or taller than its car",
            ));
        }
        let (lo, hi) = (
            self.level(e.level).map_or(0.0, |l| l.elevation),
            self.level(e.top).map_or(0.0, |l| l.elevation),
        );
        if hi < lo - EPSILON {
            return Err(ModelError::InvalidShape(
                "a lift's top stop is under its lowest",
            ));
        }
        Ok(())
    }

    pub fn add_elevator(&mut self, spec: NewElevator) -> Result<ElevatorId, ModelError> {
        self.validate_elevator(&spec)?;
        let id = ElevatorId(self.take_id());
        let name = if spec.name.is_empty() {
            self.next_elevator_name()
        } else {
            spec.name
        };
        self.elevators.insert(
            id,
            Elevator {
                id,
                guid: GlobalId::new(),
                name,
                level: spec.level,
                top: spec.top,
                position: spec.position,
                rotation: spec.rotation,
                shaft_w: spec.shaft_w,
                shaft_d: spec.shaft_d,
                wall: spec.wall,
                pit: spec.pit,
                overrun: spec.overrun,
                car_w: spec.car_w,
                car_d: spec.car_d,
                car_h: spec.car_h,
                door_w: spec.door_w,
                door_h: spec.door_h,
                load: spec.load,
                car_at: spec.car_at,
                cut_slabs: spec.cut_slabs,
                material: spec.material,
            },
        );
        self.sync_elevator_holes(None, id);
        Ok(id)
    }

    pub fn edit_elevator(
        &mut self,
        id: ElevatorId,
        edit: impl FnOnce(&mut Elevator),
    ) -> Result<(), ModelError> {
        let old = self
            .elevators
            .get(&id)
            .cloned()
            .ok_or(ModelError::UnknownElement(id.0))?;
        let mut e = old.clone();
        edit(&mut e);
        self.validate_elevator(&e.spec())?;
        self.elevators.insert(id, e);
        self.sync_elevator_holes(Some(&old), id);
        Ok(())
    }

    pub fn remove_elevator(&mut self, id: ElevatorId) -> Result<Elevator, ModelError> {
        let e = self
            .elevators
            .get(&id)
            .cloned()
            .ok_or(ModelError::UnknownElement(id.0))?;
        self.drop_elevator_holes(&e);
        self.elevators.remove(&id);
        Ok(e)
    }

    pub fn elevator(&self, id: ElevatorId) -> Option<&Elevator> {
        self.elevators.get(&id)
    }

    pub fn elevators(&self) -> impl Iterator<Item = &Elevator> {
        self.elevators.values()
    }

    /// The lifts that cross a level: those that stop at it, and those
    /// whose pit or overrun reach it.
    pub fn elevators_on_level(&self, level: LevelId) -> impl Iterator<Item = &Elevator> {
        let z = self.level(level).map(|l| l.elevation);
        self.elevators.values().filter(move |e| {
            let (Some(z), Some(lo), Some(hi)) = (
                z,
                self.level(e.level).map(|l| l.elevation),
                self.level(e.top).map(|l| l.elevation),
            ) else {
                return false;
            };
            z >= lo - EPSILON && z <= hi + EPSILON
        })
    }

    /// A name for a new lift: «Ascensor 1».
    pub fn next_elevator_name(&self) -> String {
        (1..)
            .map(|n: usize| format!("Ascensor {n}"))
            .find(|n| !self.elevators.values().any(|e| e.name == *n))
            .unwrap_or_default()
    }

    /// The slabs a lift goes through: those of its stops above the lowest,
    /// and the lowest one's too when it has a pit under it, whose outline
    /// holds the shaft.
    fn elevator_slabs(&self, e: &Elevator) -> Vec<SlabId> {
        let lo = self.level(e.level).map_or(0.0, |l| l.elevation);
        let hi = self.level(e.top).map_or(0.0, |l| l.elevation);
        let hole = e.hole();
        self.slabs
            .values()
            .filter(|s| {
                let z = self.level(s.level).map_or(0.0, |l| l.elevation) + s.top_offset;
                let from = if e.pit > 0.1 { lo - 0.5 } else { lo + 0.5 };
                z > from && z <= hi + 0.5
            })
            .filter(|s| hole.iter().all(|p| polygon_contains(&s.outline, *p)))
            .map(|s| s.id)
            .collect()
    }

    fn same_hole(a: &[Point2], b: &[Point2]) -> bool {
        a.len() == b.len() && a.iter().all(|p| b.iter().any(|q| q.distance(*p) < 1e-6))
    }

    fn drop_elevator_holes(&mut self, e: &Elevator) {
        let hole = e.hole();
        for s in self.slabs.values_mut() {
            s.holes.retain(|h| !Self::same_hole(h, &hole));
        }
    }

    /// Takes away the holes of the lift as it was (`old`) and opens those
    /// of the lift as it is, when it cuts the slabs.
    pub fn sync_elevator_holes(&mut self, old: Option<&Elevator>, id: ElevatorId) {
        if let Some(o) = old {
            self.drop_elevator_holes(o);
        }
        let Some(e) = self.elevators.get(&id).cloned() else {
            return;
        };
        if !e.cut_slabs {
            return;
        }
        let mut hole = e.hole();
        if signed_area(&hole) < 0.0 {
            hole.reverse();
        }
        for s in self.elevator_slabs(&e) {
            let Some(slab) = self.slabs.get(&s) else {
                continue;
            };
            if slab.holes.iter().any(|h| Self::same_hole(h, &hole)) {
                continue;
            }
            let mut holes = slab.holes.clone();
            holes.push(hole.clone());
            if Self::check_holes(&slab.outline, &holes).is_ok() {
                if let Some(slab) = self.slabs.get_mut(&s) {
                    slab.holes = holes;
                }
            }
        }
    }

    pub(super) fn restore_elevators(
        &mut self,
        elevators: Vec<Elevator>,
        claim: &mut structure::Claim<'_>,
    ) -> Result<(), RestoreError> {
        for e in elevators {
            claim(e.id.0, Some(&e.guid))
                .and_then(|_| self.validate_elevator(&e.spec()))
                .map_err(|error| RestoreError {
                    element: format!("lift {} «{}»", e.id.0, e.name),
                    error,
                })?;
            self.elevators.insert(e.id, e);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_lift_stops_at_its_levels_and_opens_its_holes() {
        let mut m = Model::new("Ascensor");
        let l0 = m.add_level("Planta baja", 0.0);
        let l1 = m.add_level("Planta 1", 3.2);
        let l2 = m.add_level("Planta 2", 6.4);
        let _roof = m.add_level("Cubierta", 9.6);
        let mat = m.add_material("Hormigón", "concrete", [180, 180, 180], CutPattern::Solid);
        let t = m
            .add_slab_type(
                "Forjado",
                SlabFunction::Floor,
                vec![Layer {
                    material: mat,
                    thickness: 0.3,
                    function: LayerFunction::Structure,
                }],
            )
            .unwrap();
        let sq = |s: f64| {
            vec![
                Point2::new(0.0, 0.0),
                Point2::new(s, 0.0),
                Point2::new(s, s),
                Point2::new(0.0, s),
            ]
        };
        let mut slabs = Vec::new();
        for l in [l0, l1, l2] {
            slabs.push(
                m.add_slab(NewSlab {
                    slab_type: t,
                    level: l,
                    outline: sq(10.0),
                    top_offset: 0.0,
                })
                .unwrap(),
            );
        }
        let id = m
            .add_elevator(NewElevator::new(l0, l2, Point2::new(5.0, 5.0)))
            .unwrap();
        assert_eq!(m.elevator_stops(id), vec![l0, l1, l2]);
        assert_eq!(m.elevator_stop_heights(id), vec![0.0, 3.2, 6.4]);
        // Every slab is cut, the ground floor's too (the pit is under it).
        assert_eq!(m.slab(slabs[0]).unwrap().holes.len(), 1);
        assert_eq!(m.slab(slabs[1]).unwrap().holes.len(), 1);
        assert_eq!(m.slab(slabs[2]).unwrap().holes.len(), 1);
        // Moved, the holes follow; removed, they go.
        m.edit_elevator(id, |e| e.position = Point2::new(3.0, 3.0))
            .unwrap();
        let h = &m.slab(slabs[1]).unwrap().holes;
        assert_eq!(h.len(), 1);
        assert!(h[0].iter().any(|p| p.x < 3.0));
        m.remove_elevator(id).unwrap();
        assert!(m.slab(slabs[1]).unwrap().holes.is_empty());
        // Without a pit, the lowest slab keeps its floor.
        let mut flat = NewElevator::new(l0, l2, Point2::new(5.0, 5.0));
        flat.pit = 0.0;
        m.add_elevator(flat).unwrap();
        assert!(m.slab(slabs[0]).unwrap().holes.is_empty());
        assert_eq!(m.slab(slabs[1]).unwrap().holes.len(), 1);
        // A car that does not fit.
        let mut bad = NewElevator::new(l0, l2, Point2::new(5.0, 5.0));
        bad.car_w = 3.0;
        assert!(m.add_elevator(bad).is_err());
    }
}
