//! Reinforced concrete (E11, decision D26): ArchiSim draws and checks the
//! reinforcement of columns, beams, footings and slabs — the bars, their
//! cover and spacing, against the minimums of the Código Estructural
//! that need no calculation (`archisim-cte::structure`) — and does not
//! calculate the structure: that is for a structural program.
//!
//! The reinforcement of an element is kept apart from it, by the element's
//! id, and described as a designer says it («4Ø16, cercos Ø8 c/15»); the
//! bars themselves are laid by `archisim_geom::rebar` whenever they are
//! needed.

use super::*;

/// The diameters of bar on sale (metres), for the choices.
pub const BAR_DIAMETERS: [f64; 10] = [
    0.006, 0.008, 0.010, 0.012, 0.014, 0.016, 0.020, 0.025, 0.032, 0.040,
];

/// Bars at a spacing: «Ø12 c/20».
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Spaced {
    pub diameter: f64,
    pub spacing: f64,
}

impl Spaced {
    pub const fn new(diameter: f64, spacing: f64) -> Spaced {
        Spaced { diameter, spacing }
    }

    /// As written on a drawing: «Ø12 c/20».
    pub fn label(&self) -> String {
        format!("Ø{} c/{}", mm(self.diameter), fmt_cm(self.spacing))
    }
}

/// A number of bars: «4Ø16».
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Counted {
    pub count: u32,
    pub diameter: f64,
}

impl Counted {
    pub const fn new(count: u32, diameter: f64) -> Counted {
        Counted { count, diameter }
    }

    pub fn label(&self) -> String {
        format!("{}Ø{}", self.count, mm(self.diameter))
    }

    /// The area of steel, m².
    pub fn area(&self) -> f64 {
        self.count as f64 * bar_area(self.diameter)
    }
}

fn mm(v: f64) -> String {
    format!("{}", (v * 1000.0).round() as i64)
}

fn fmt_cm(v: f64) -> String {
    let cm = v * 100.0;
    if (cm - cm.round()).abs() < 1e-6 {
        format!("{}", cm.round() as i64)
    } else {
        format!("{cm:.1}").replace('.', ",")
    }
}

/// The area of a bar, m².
pub fn bar_area(diameter: f64) -> f64 {
    std::f64::consts::PI * diameter * diameter / 4.0
}

/// The weight of a metre of bar, kg (steel, 7850 kg/m³).
pub fn bar_weight(diameter: f64) -> f64 {
    7850.0 * bar_area(diameter)
}

/// The reinforcement of an element.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Reinforcement {
    /// A column: longitudinal bars round its section (the corners first)
    /// and ties; the ties closer at both ends, along `dense_length`.
    Column {
        cover: f64,
        bars: Counted,
        ties: Spaced,
        dense: Option<(f64, f64)>,
    },
    /// A beam: bars along its bottom and its top, and stirrups.
    Beam {
        cover: f64,
        bottom: Counted,
        top: Counted,
        stirrups: Spaced,
    },
    /// A footing: a mesh at the bottom (across and along) and, if it has
    /// one, at the top.
    Footing {
        cover: f64,
        bottom: [Spaced; 2],
        top: Option<[Spaced; 2]>,
    },
    /// A slab: a mesh at the bottom (along its first edge and across) and
    /// at the top.
    Slab {
        cover: f64,
        bottom: [Spaced; 2],
        top: Option<[Spaced; 2]>,
    },
}

/// What a reinforcement is for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RebarKind {
    Column,
    Beam,
    Footing,
    Slab,
}

impl Reinforcement {
    /// What a designer would put first, for each kind of element.
    pub fn default_for(kind: RebarKind) -> Reinforcement {
        match kind {
            RebarKind::Column => Reinforcement::Column {
                cover: 0.03,
                bars: Counted::new(4, 0.016),
                ties: Spaced::new(0.008, 0.15),
                dense: Some((0.10, 0.60)),
            },
            RebarKind::Beam => Reinforcement::Beam {
                cover: 0.03,
                bottom: Counted::new(3, 0.016),
                top: Counted::new(2, 0.012),
                stirrups: Spaced::new(0.008, 0.20),
            },
            RebarKind::Footing => Reinforcement::Footing {
                cover: 0.05,
                bottom: [Spaced::new(0.012, 0.20); 2],
                top: None,
            },
            RebarKind::Slab => Reinforcement::Slab {
                cover: 0.025,
                bottom: [Spaced::new(0.010, 0.20); 2],
                top: Some([Spaced::new(0.010, 0.20); 2]),
            },
        }
    }

    pub fn kind(&self) -> RebarKind {
        match self {
            Reinforcement::Column { .. } => RebarKind::Column,
            Reinforcement::Beam { .. } => RebarKind::Beam,
            Reinforcement::Footing { .. } => RebarKind::Footing,
            Reinforcement::Slab { .. } => RebarKind::Slab,
        }
    }

    pub fn cover(&self) -> f64 {
        match *self {
            Reinforcement::Column { cover, .. }
            | Reinforcement::Beam { cover, .. }
            | Reinforcement::Footing { cover, .. }
            | Reinforcement::Slab { cover, .. } => cover,
        }
    }

    /// As a designer writes it, in one line.
    pub fn summary(&self) -> String {
        let mesh = |m: &[Spaced; 2]| {
            if m[0] == m[1] {
                format!("{} en las dos direcciones", m[0].label())
            } else {
                format!("{} × {}", m[0].label(), m[1].label())
            }
        };
        match self {
            Reinforcement::Column { bars, ties, .. } => {
                format!("{}, cercos {}", bars.label(), ties.label())
            }
            Reinforcement::Beam {
                bottom,
                top,
                stirrups,
                ..
            } => format!(
                "Inferior {}, superior {}, estribos {}",
                bottom.label(),
                top.label(),
                stirrups.label()
            ),
            Reinforcement::Footing { bottom, top, .. }
            | Reinforcement::Slab { bottom, top, .. } => match top {
                Some(t) => format!("Inferior {}; superior {}", mesh(bottom), mesh(t)),
                None => format!("Inferior {}", mesh(bottom)),
            },
        }
    }

    fn is_valid(&self) -> bool {
        let ok = |v: f64| v.is_finite() && v > 0.0;
        let spaced = |s: &Spaced| ok(s.diameter) && ok(s.spacing) && s.spacing > s.diameter;
        let counted = |c: &Counted| ok(c.diameter);
        match self {
            Reinforcement::Column {
                cover,
                bars,
                ties,
                dense,
            } => {
                ok(*cover)
                    && counted(bars)
                    && bars.count >= 1
                    && spaced(ties)
                    && dense.is_none_or(|(s, l)| ok(s) && ok(l))
            }
            Reinforcement::Beam {
                cover,
                bottom,
                top,
                stirrups,
            } => ok(*cover) && counted(bottom) && counted(top) && spaced(stirrups),
            Reinforcement::Footing { cover, bottom, top }
            | Reinforcement::Slab { cover, bottom, top } => {
                ok(*cover) && bottom.iter().all(spaced) && top.is_none_or(|t| t.iter().all(spaced))
            }
        }
    }
}

impl Model {
    /// What kind of reinforcement an element can take: a column or beam of
    /// concrete (rectangular or round), a footing, a level slab.
    pub fn rebar_kind(&self, id: u64) -> Option<RebarKind> {
        let concrete = |t: MemberTypeId| {
            self.member_type(t).is_some_and(|t| {
                matches!(
                    t.profile,
                    Profile::Rectangle { .. } | Profile::Circle { .. }
                )
            })
        };
        if let Some(c) = self.column(ColumnId(id)) {
            return concrete(c.member_type).then_some(RebarKind::Column);
        }
        if let Some(b) = self.beam(BeamId(id)) {
            return concrete(b.member_type).then_some(RebarKind::Beam);
        }
        if self.footing(FootingId(id)).is_some() {
            return Some(RebarKind::Footing);
        }
        if let Some(s) = self.slab(SlabId(id)) {
            return s.slope.is_none().then_some(RebarKind::Slab);
        }
        None
    }

    /// The reinforcement of an element, if it has one (and still is).
    pub fn reinforcement(&self, id: u64) -> Option<&Reinforcement> {
        let r = self.reinforcements.get(&id)?;
        (self.rebar_kind(id) == Some(r.kind())).then_some(r)
    }

    /// The elements with reinforcement, and theirs.
    pub fn reinforcements(&self) -> impl Iterator<Item = (u64, &Reinforcement)> {
        self.reinforcements
            .iter()
            .filter(|(id, r)| self.rebar_kind(**id) == Some(r.kind()))
            .map(|(id, r)| (*id, r))
    }

    /// Gives an element its reinforcement (or takes it away, `None`).
    pub fn set_reinforcement(
        &mut self,
        id: u64,
        r: Option<Reinforcement>,
    ) -> Result<(), ModelError> {
        match r {
            None => {
                self.reinforcements.remove(&id);
            }
            Some(r) => {
                if self.rebar_kind(id) != Some(r.kind()) {
                    return Err(ModelError::NotAnObject(format!(
                        "el elemento {id} no lleva ese armado"
                    )));
                }
                if !r.is_valid() {
                    return Err(ModelError::NonPositive("reinforcement"));
                }
                self.reinforcements.insert(id, r);
            }
        }
        Ok(())
    }

    /// The material of the bars: the first steel of the project named for
    /// reinforcement, or any steel.
    pub fn rebar_material(&self) -> Option<MaterialId> {
        self.materials()
            .find(|m| m.name.starts_with("Acero B500"))
            .or_else(|| self.materials().find(|m| m.category == "steel"))
            .map(|m| m.id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn written_as_a_designer_does() {
        let r = Reinforcement::default_for(RebarKind::Column);
        assert_eq!(r.summary(), "4Ø16, cercos Ø8 c/15");
        let s = Reinforcement::default_for(RebarKind::Slab);
        assert_eq!(
            s.summary(),
            "Inferior Ø10 c/20 en las dos direcciones; superior Ø10 c/20 en las dos direcciones"
        );
        assert!((bar_weight(0.016) - 1.578).abs() < 0.01);
    }

    #[test]
    fn only_what_can_take_it() {
        let mut m = Model::new("x");
        let l = m.add_level("PB", 0.0);
        let mat = m.add_material("Hormigón", "concrete", [200, 200, 200], CutPattern::Solid);
        let t = m
            .add_member_type(
                "P 30×30",
                MemberUsage::Column,
                mat,
                Profile::Rectangle {
                    width: 0.3,
                    depth: 0.3,
                },
            )
            .unwrap();
        let c = m
            .add_column(NewColumn {
                member_type: t,
                level: l,
                position: Point2::new(0.0, 0.0),
                rotation: 0.0,
                base_offset: 0.0,
                top: WallTop::Unconnected,
                height: 3.0,
            })
            .unwrap();
        let r = Reinforcement::default_for(RebarKind::Column);
        m.set_reinforcement(c.0, Some(r)).unwrap();
        assert_eq!(m.reinforcement(c.0), Some(&r));
        assert!(m
            .set_reinforcement(c.0, Some(Reinforcement::default_for(RebarKind::Beam)))
            .is_err());
        m.remove_column(c).unwrap();
        assert!(m.reinforcement(c.0).is_none());
        assert_eq!(m.reinforcements().count(), 0);
    }
}
