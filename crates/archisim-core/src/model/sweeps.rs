//! Accessories of wall and slab types (bloque 5 de la revisión del
//! 22/10/2026), as Revit's sweeps: a profile that runs along every wall of
//! a type — a skirting board at the foot of its interior face, a band or a
//! cornice on its exterior face, a coping on its top — and along the edge
//! of every slab of a type (a cornice or an eave). They follow their walls
//! and slabs, stop at the doors and windows they meet, and go to the IFC
//! as coverings of their element (`IfcCovering` MOLDING, SKIRTINGBOARD,
//! COPING).

use super::*;

/// What an accessory is: its name and its IFC.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SweepKind {
    /// «Rodapié»: at the foot of a face.
    Skirting,
    /// «Moldura» or «imposta»: a band along a face.
    Moulding,
    /// «Cornisa»: at the top of a face, standing out.
    Cornice,
    /// «Albardilla»: on top of the wall, across its thickness.
    Coping,
}

impl SweepKind {
    pub const ALL: [SweepKind; 4] = [
        SweepKind::Skirting,
        SweepKind::Moulding,
        SweepKind::Cornice,
        SweepKind::Coping,
    ];

    pub fn name(self) -> &'static str {
        match self {
            SweepKind::Skirting => "Rodapié",
            SweepKind::Moulding => "Moldura",
            SweepKind::Cornice => "Cornisa",
            SweepKind::Coping => "Albardilla",
        }
    }

    pub fn code(self) -> &'static str {
        match self {
            SweepKind::Skirting => "skirting",
            SweepKind::Moulding => "moulding",
            SweepKind::Cornice => "cornice",
            SweepKind::Coping => "coping",
        }
    }

    pub fn from_code(c: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|k| k.code() == c)
    }
}

/// The face of the wall an accessory runs along.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SweepSide {
    Exterior,
    Interior,
    /// On top of the wall.
    Top,
}

impl SweepSide {
    pub fn code(self) -> &'static str {
        match self {
            SweepSide::Exterior => "exterior",
            SweepSide::Interior => "interior",
            SweepSide::Top => "top",
        }
    }

    pub fn from_code(c: &str) -> Option<Self> {
        [SweepSide::Exterior, SweepSide::Interior, SweepSide::Top]
            .into_iter()
            .find(|k| k.code() == c)
    }

    pub fn name(self) -> &'static str {
        match self {
            SweepSide::Exterior => "Cara exterior",
            SweepSide::Interior => "Cara interior",
            SweepSide::Top => "Coronación",
        }
    }
}

/// An accessory of a wall type.
#[derive(Debug, Clone, PartialEq)]
pub struct WallSweep {
    pub kind: SweepKind,
    pub side: SweepSide,
    /// Height of its foot over the wall's base or, `from_top`, of its top
    /// under the wall's top (on the top: over it).
    pub offset: f64,
    pub from_top: bool,
    /// Its section: each point out from the face (`x`; on the top, from
    /// the exterior face towards the interior one) and up from its foot
    /// (`y`), metres.
    pub profile: Vec<Point2>,
    pub material: MaterialId,
}

/// The edge of a slab type: a profile round every slab's outline, out
/// from its edge (`x`) and up from its top face (`y`, below it negative).
#[derive(Debug, Clone, PartialEq)]
pub struct SlabEdge {
    pub profile: Vec<Point2>,
    pub material: MaterialId,
}

/// The ready sections of each kind; a coping's spans a wall `thickness`
/// thick with 4 cm drips on both sides.
pub fn sweep_preset(kind: SweepKind, thickness: f64) -> Vec<Point2> {
    let p = Point2::new;
    match kind {
        SweepKind::Skirting => vec![
            p(0.0, 0.0),
            p(0.015, 0.0),
            p(0.015, 0.07),
            p(0.008, 0.08),
            p(0.0, 0.08),
        ],
        SweepKind::Moulding => vec![
            p(0.0, 0.0),
            p(0.03, 0.0),
            p(0.05, 0.02),
            p(0.05, 0.13),
            p(0.03, 0.15),
            p(0.0, 0.15),
        ],
        SweepKind::Cornice => vec![
            p(0.0, 0.0),
            p(0.06, 0.0),
            p(0.06, 0.06),
            p(0.22, 0.2),
            p(0.3, 0.2),
            p(0.3, 0.3),
            p(0.0, 0.3),
        ],
        SweepKind::Coping => vec![
            p(-0.04, 0.0),
            p(thickness + 0.04, 0.0),
            p(thickness + 0.04, 0.03),
            p(thickness / 2.0, 0.06),
            p(-0.04, 0.03),
        ],
    }
}

/// A slab's edge, ready: a cornice 20 cm out (`eave`: an eave 60 cm out,
/// thin).
pub fn slab_edge_preset(eave: bool) -> Vec<Point2> {
    let p = Point2::new;
    if eave {
        vec![p(0.0, -0.12), p(0.6, -0.06), p(0.6, 0.0), p(0.0, 0.0)]
    } else {
        vec![
            p(0.0, -0.3),
            p(0.08, -0.3),
            p(0.08, -0.2),
            p(0.2, -0.08),
            p(0.2, 0.05),
            p(0.0, 0.05),
        ]
    }
}

fn check_profile(profile: &[Point2]) -> Result<(), ModelError> {
    for q in profile {
        finite_point(*q, "accessory profile")?;
    }
    if profile.len() < 3 || signed_area(profile).abs() <= EPSILON {
        return Err(ModelError::InvalidShape(
            "an accessory's section needs three points or more round an area",
        ));
    }
    Ok(())
}

impl Model {
    fn check_sweeps(&self, sweeps: &[WallSweep]) -> Result<(), ModelError> {
        for s in sweeps {
            if !self.materials.contains_key(&s.material) {
                return Err(ModelError::UnknownMaterial(s.material));
            }
            finite(s.offset, "accessory offset")?;
            check_profile(&s.profile)?;
        }
        Ok(())
    }

    /// The accessories of a wall type: every wall of the type gets them.
    pub fn set_wall_type_sweeps(
        &mut self,
        id: WallTypeId,
        sweeps: Vec<WallSweep>,
    ) -> Result<(), ModelError> {
        self.check_sweeps(&sweeps)?;
        let t = self
            .wall_types
            .get_mut(&id)
            .ok_or(ModelError::UnknownWallType(id))?;
        t.sweeps = sweeps;
        Ok(())
    }

    /// The edge of a slab type (none: plain).
    pub fn set_slab_type_edge(
        &mut self,
        id: SlabTypeId,
        edge: Option<SlabEdge>,
    ) -> Result<(), ModelError> {
        if let Some(e) = &edge {
            if !self.materials.contains_key(&e.material) {
                return Err(ModelError::UnknownMaterial(e.material));
            }
            check_profile(&e.profile)?;
        }
        let t = self
            .slab_types
            .get_mut(&id)
            .ok_or(ModelError::UnknownSlabType(id))?;
        t.edge = edge;
        Ok(())
    }

    /// Checks what a file brings.
    pub(super) fn check_type_accessories(&self) -> Result<(), RestoreError> {
        for t in self.wall_types.values() {
            self.check_sweeps(&t.sweeps).map_err(|error| RestoreError {
                element: format!("wall type {} «{}»", t.id.0, t.name),
                error,
            })?;
        }
        for t in self.slab_types.values() {
            if let Some(e) = &t.edge {
                let r = if self.materials.contains_key(&e.material) {
                    check_profile(&e.profile)
                } else {
                    Err(ModelError::UnknownMaterial(e.material))
                };
                r.map_err(|error| RestoreError {
                    element: format!("slab type {} «{}»", t.id.0, t.name),
                    error,
                })?;
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn types_carry_their_accessories() {
        let mut m = Model::new("t");
        let mat = m.add_material("Piedra", "stone", [200, 200, 190], CutPattern::Solid);
        let layer = Layer {
            material: mat,
            thickness: 0.2,
            function: LayerFunction::Structure,
        };
        let wt = m
            .add_wall_type("Muro", WallFunction::Exterior, vec![layer.clone()])
            .unwrap();
        let skirting = WallSweep {
            kind: SweepKind::Skirting,
            side: SweepSide::Interior,
            offset: 0.0,
            from_top: false,
            profile: sweep_preset(SweepKind::Skirting, 0.2),
            material: mat,
        };
        m.set_wall_type_sweeps(wt, vec![skirting.clone()]).unwrap();
        assert_eq!(m.wall_type(wt).unwrap().sweeps, vec![skirting]);
        let bad = WallSweep {
            profile: vec![Point2::new(0.0, 0.0)],
            ..m.wall_type(wt).unwrap().sweeps[0].clone()
        };
        assert!(m.set_wall_type_sweeps(wt, vec![bad]).is_err());
        let st = m
            .add_slab_type("Forjado", SlabFunction::Floor, vec![layer])
            .unwrap();
        let edge = SlabEdge {
            profile: slab_edge_preset(false),
            material: mat,
        };
        m.set_slab_type_edge(st, Some(edge.clone())).unwrap();
        assert_eq!(m.slab_type(st).unwrap().edge, Some(edge));
        let back = Model::from_parts(m.to_parts()).unwrap();
        assert_eq!(back, m);
    }
}
