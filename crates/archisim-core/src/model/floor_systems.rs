//! How a concrete floor is built (`docs/ESTRUCTURA.md` §6): a solid slab
//! (nothing stored), a one-way floor of joists and blocks (forjado
//! unidireccional: precast prestressed joists or ribs cast in place, with
//! concrete, ceramic or polystyrene blocks and a topping) or a waffle
//! floor (forjado reticular: ribs both ways round lost or recoverable
//! pans, solid round the columns).
//!
//! The system belongs to the slab type, as the light frame does; the
//! direction of the joists of a one-way floor belongs to each slab
//! (`None`: the shorter way between its supports, which the calculation
//! finds).

use super::*;

/// One-way or waffle.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FloorKind {
    /// Forjado unidireccional de viguetas y bovedillas.
    OneWay,
    /// Forjado reticular (bidireccional de nervios).
    Waffle,
}

/// The joists of a one-way floor.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Joist {
    /// Precast prestressed joists (viguetas pretensadas): their strength
    /// is the manufacturer's; the calculation gives what they must take.
    Precast,
    /// Ribs reinforced in place (nervios armados in situ).
    InSitu,
}

/// What fills between the ribs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Infill {
    Concrete,
    Ceramic,
    Polystyrene,
    /// Recoverable pans (casetón recuperable): nothing stays.
    Recoverable,
}

impl FloorKind {
    pub fn code(self) -> &'static str {
        match self {
            FloorKind::OneWay => "one_way",
            FloorKind::Waffle => "waffle",
        }
    }
    pub fn from_code(s: &str) -> Option<Self> {
        Some(match s {
            "one_way" => FloorKind::OneWay,
            "waffle" => FloorKind::Waffle,
            _ => return None,
        })
    }
    pub fn label(self) -> &'static str {
        match self {
            FloorKind::OneWay => "Unidireccional (viguetas y bovedillas)",
            FloorKind::Waffle => "Reticular (nervios en dos direcciones)",
        }
    }
}

impl Joist {
    pub fn code(self) -> &'static str {
        match self {
            Joist::Precast => "precast",
            Joist::InSitu => "in_situ",
        }
    }
    pub fn from_code(s: &str) -> Option<Self> {
        Some(match s {
            "precast" => Joist::Precast,
            "in_situ" => Joist::InSitu,
            _ => return None,
        })
    }
    pub fn label(self) -> &'static str {
        match self {
            Joist::Precast => "Vigueta pretensada",
            Joist::InSitu => "Nervio armado in situ",
        }
    }
}

impl Infill {
    pub const ALL: [Infill; 4] = [
        Infill::Concrete,
        Infill::Ceramic,
        Infill::Polystyrene,
        Infill::Recoverable,
    ];
    pub fn code(self) -> &'static str {
        match self {
            Infill::Concrete => "concrete",
            Infill::Ceramic => "ceramic",
            Infill::Polystyrene => "polystyrene",
            Infill::Recoverable => "recoverable",
        }
    }
    pub fn from_code(s: &str) -> Option<Self> {
        Some(match s {
            "concrete" => Infill::Concrete,
            "ceramic" => Infill::Ceramic,
            "polystyrene" => Infill::Polystyrene,
            "recoverable" => Infill::Recoverable,
            _ => return None,
        })
    }
    pub fn label(self) -> &'static str {
        match self {
            Infill::Concrete => "Bovedilla de hormigón",
            Infill::Ceramic => "Bovedilla cerámica",
            Infill::Polystyrene => "Bovedilla de poliestireno",
            Infill::Recoverable => "Casetón recuperable",
        }
    }
    /// Weight of the blocks over the space they fill between ribs, kN/m³:
    /// they are hollow (a concrete block of 25 some 1,4 kN/m² of floor).
    pub fn weight(self) -> f64 {
        match self {
            Infill::Concrete => 7.0,
            Infill::Ceramic => 5.0,
            Infill::Polystyrene => 0.3,
            Infill::Recoverable => 0.0,
        }
    }
}

/// A one-way or waffle floor.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FloorSystem {
    pub kind: FloorKind,
    /// One-way floors only.
    pub joist: Joist,
    pub infill: Infill,
    /// Between rib axes (intereje), m.
    pub spacing: f64,
    /// Width of a rib, m.
    pub rib: f64,
    /// Total depth, topping included, m.
    pub depth: f64,
    /// The topping (capa de compresión), m.
    pub topping: f64,
}

impl FloorSystem {
    /// The usual one-way floor here: precast joists at 70 cm with
    /// concrete blocks, 25 + 5.
    pub fn one_way() -> Self {
        FloorSystem {
            kind: FloorKind::OneWay,
            joist: Joist::Precast,
            infill: Infill::Concrete,
            spacing: 0.70,
            rib: 0.12,
            depth: 0.30,
            topping: 0.05,
        }
    }

    /// A waffle floor of 25 + 5 with ribs of 12 at 80 cm and lost
    /// concrete blocks.
    pub fn waffle() -> Self {
        FloorSystem {
            kind: FloorKind::Waffle,
            joist: Joist::InSitu,
            infill: Infill::Concrete,
            spacing: 0.80,
            rib: 0.12,
            depth: 0.30,
            topping: 0.05,
        }
    }

    pub fn is_valid(&self) -> bool {
        self.spacing > self.rib
            && self.rib >= 0.07
            && self.spacing <= 1.2
            && self.topping >= 0.03
            && self.depth > self.topping + 0.08
            && self.depth <= 0.8
    }

    /// «25+5 / 70».
    pub fn label(&self) -> String {
        let cm = |x: f64| format!("{:.0}", x * 100.0);
        format!(
            "{}+{} / {}",
            cm(self.depth - self.topping),
            cm(self.topping),
            cm(self.spacing)
        )
    }

    /// Concrete per m² of floor, m³/m² (the ribs both ways in a waffle,
    /// less their crossings).
    pub fn concrete(&self) -> f64 {
        let h = self.depth - self.topping;
        let ribs = match self.kind {
            FloorKind::OneWay => self.rib * h / self.spacing,
            FloorKind::Waffle => {
                let one = self.rib / self.spacing;
                (2.0 * one - one * one) * h
            }
        };
        self.topping + ribs
    }

    /// Weight per m² of the floor itself, kN/m²: concrete at 25 and the
    /// blocks.
    pub fn weight(&self) -> f64 {
        let h = self.depth - self.topping;
        let blocks = (h - self.concrete() + self.topping).max(0.0);
        25.0 * self.concrete() + self.infill.weight() * blocks
    }

    /// The second moment of area of one rib with its share of topping
    /// (a T), about its centroid, m⁴, and its centroid from the top, m.
    pub fn rib_inertia(&self) -> (f64, f64) {
        let (b, t, w, h) = (self.spacing, self.topping, self.rib, self.depth);
        let a1 = b * t;
        let a2 = w * (h - t);
        let y = (a1 * t / 2.0 + a2 * (t + (h - t) / 2.0)) / (a1 + a2);
        let i = b * t.powi(3) / 12.0
            + a1 * (y - t / 2.0).powi(2)
            + w * (h - t).powi(3) / 12.0
            + a2 * (t + (h - t) / 2.0 - y).powi(2);
        (i, y)
    }
}

impl Model {
    /// The direction of a slab's joists: given by hand, or else along the
    /// shorter side of the narrowest rectangle round it (one of its edges'
    /// directions).
    pub fn joist_direction(&self, slab: SlabId) -> Option<Vec2> {
        let s = self.slab(slab)?;
        if let Some(a) = self.slab_joists(slab) {
            return Some(Vec2::new(a.cos(), a.sin()));
        }
        let n = s.outline.len();
        let mut best: Option<(f64, Vec2)> = None;
        for k in 0..n {
            let Some(u) = (s.outline[(k + 1) % n] - s.outline[k]).normalized() else {
                continue;
            };
            let v = Vec2::new(-u.y, u.x);
            let (mut a0, mut a1, mut b0, mut b1) = (f64::MAX, f64::MIN, f64::MAX, f64::MIN);
            for p in &s.outline {
                let d = *p - s.outline[0];
                let (a, b) = (d.x * u.x + d.y * u.y, d.x * v.x + d.y * v.y);
                a0 = a0.min(a);
                a1 = a1.max(a);
                b0 = b0.min(b);
                b1 = b1.max(b);
            }
            let area = (a1 - a0) * (b1 - b0);
            let dir = if a1 - a0 <= b1 - b0 { u } else { v };
            if best.is_none_or(|(ar, _)| area < ar - 1e-9) {
                best = Some((area, dir));
            }
        }
        best.map(|(_, d)| d)
    }

    /// The floor system of a slab type, if it is not a solid slab.
    pub fn floor_system(&self, id: SlabTypeId) -> Option<&FloorSystem> {
        self.floor_systems
            .get(&id.0)
            .filter(|_| self.slab_type(id).is_some())
    }

    /// The slab types with a floor system.
    pub fn floor_systems(&self) -> impl Iterator<Item = (SlabTypeId, &FloorSystem)> {
        self.floor_systems
            .iter()
            .filter(|(id, _)| self.slab_type(SlabTypeId(**id)).is_some())
            .map(|(id, f)| (SlabTypeId(*id), f))
    }

    /// Makes a slab type one-way or waffle (or solid again, `None`).
    pub fn set_floor_system(
        &mut self,
        id: SlabTypeId,
        f: Option<FloorSystem>,
    ) -> Result<(), ModelError> {
        match f {
            None => {
                self.floor_systems.remove(&id.0);
            }
            Some(f) => {
                if self.slab_type(id).is_none() {
                    return Err(ModelError::UnknownElement(id.0));
                }
                if !f.is_valid() {
                    return Err(ModelError::NonPositive("floor system"));
                }
                self.floor_systems.insert(id.0, f);
            }
        }
        Ok(())
    }

    /// The direction of the joists of a slab given by hand, radians from
    /// x (`None`: the shorter way, found by the calculation).
    pub fn slab_joists(&self, id: SlabId) -> Option<f64> {
        self.slab_joists
            .get(&id.0)
            .copied()
            .filter(|_| self.slab(id).is_some())
    }

    /// The slabs with their joists given by hand.
    pub fn all_slab_joists(&self) -> impl Iterator<Item = (SlabId, f64)> + '_ {
        self.slab_joists
            .iter()
            .filter(|(id, _)| self.slab(SlabId(**id)).is_some())
            .map(|(id, a)| (SlabId(*id), *a))
    }

    /// Gives the direction of a slab's joists (or lets the calculation
    /// choose, `None`). The angle is kept between 0 and π: a joist has no
    /// way.
    pub fn set_slab_joists(&mut self, id: SlabId, angle: Option<f64>) -> Result<(), ModelError> {
        if self.slab(id).is_none() {
            return Err(ModelError::UnknownElement(id.0));
        }
        match angle {
            None => {
                self.slab_joists.remove(&id.0);
            }
            Some(a) => {
                if !a.is_finite() {
                    return Err(ModelError::NonPositive("joist direction"));
                }
                self.slab_joists
                    .insert(id.0, a.rem_euclid(std::f64::consts::PI));
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn weights_of_the_usual_floors() {
        // 25+5 one-way with concrete blocks: some 3,3–3,8 kN/m².
        let g = FloorSystem::one_way().weight();
        assert!((3.0..4.0).contains(&g), "{g}");
        // With polystyrene, much less.
        let light = FloorSystem {
            infill: Infill::Polystyrene,
            ..FloorSystem::one_way()
        };
        assert!(light.weight() < 2.5, "{}", light.weight());
        // Waffle 25+5 with lost blocks: some 4–5 kN/m².
        let g = FloorSystem::waffle().weight();
        assert!((3.8..5.2).contains(&g), "{g}");
        assert_eq!(FloorSystem::one_way().label(), "25+5 / 70");
        let (i, y) = FloorSystem::one_way().rib_inertia();
        assert!(y > 0.05 && y < 0.15 && i > 1e-4 && i < 1e-3, "{i} {y}");
    }

    #[test]
    fn stored_by_type_and_slab() {
        let mut m = Model::new("F");
        let l = m.add_level("P1", 3.0);
        let mat = m.add_material("Hormigón", "concrete", [180, 180, 180], CutPattern::Solid);
        let t = m
            .add_slab_type(
                "Unidireccional",
                SlabFunction::Floor,
                vec![Layer {
                    material: mat,
                    thickness: 0.3,
                    function: LayerFunction::Structure,
                }],
            )
            .unwrap();
        m.set_floor_system(t, Some(FloorSystem::one_way())).unwrap();
        assert_eq!(m.floor_system(t).unwrap().kind, FloorKind::OneWay);
        let bad = FloorSystem {
            rib: 0.9,
            ..FloorSystem::one_way()
        };
        assert!(m.set_floor_system(t, Some(bad)).is_err());
        let s = m
            .add_slab(NewSlab {
                slab_type: t,
                level: l,
                outline: vec![
                    Point2::new(0.0, 0.0),
                    Point2::new(4.0, 0.0),
                    Point2::new(4.0, 6.0),
                ],
                top_offset: 0.0,
            })
            .unwrap();
        m.set_slab_joists(s, Some(-std::f64::consts::FRAC_PI_2))
            .unwrap();
        let a = m.slab_joists(s).unwrap();
        assert!((a - std::f64::consts::FRAC_PI_2).abs() < 1e-12);
        m.set_floor_system(t, None).unwrap();
        assert!(m.floor_system(t).is_none());
    }
}
