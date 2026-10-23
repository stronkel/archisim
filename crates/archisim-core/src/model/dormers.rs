//! Dormers ("buhardillas"): a small roof with its front wall standing on a
//! plane of a pitched roof, to light the space under it. Each belongs to
//! its roof (`Roof::dormers`): placed by the centre of its front in plan,
//! it faces down the plane it stands on. A shed dormer ("a un agua") has
//! its roof rising backwards, flatter than the main one, until they meet;
//! a gable dormer ("a dos aguas", "capuchina") has a ridge running back
//! from its front until it meets the main roof, with valleys at its sides.
//!
//! IFC: `IfcRoof` (shed or gable) aggregated into the main roof's.

use super::*;

/// The shape of a dormer's roof.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DormerKind {
    /// One plane rising backwards ("a un agua").
    Shed,
    /// Two planes and a ridge running back ("a dos aguas").
    Gable,
}

impl DormerKind {
    pub fn code(self) -> &'static str {
        match self {
            DormerKind::Shed => "shed",
            DormerKind::Gable => "gable",
        }
    }

    pub fn from_code(s: &str) -> Option<Self> {
        match s {
            "shed" => Some(DormerKind::Shed),
            "gable" => Some(DormerKind::Gable),
            _ => None,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            DormerKind::Shed => "A un agua",
            DormerKind::Gable => "A dos aguas",
        }
    }
}

/// A dormer on a roof.
#[derive(Debug, Clone, PartialEq)]
pub struct Dormer {
    /// The centre of its front, in plan.
    pub position: Point2,
    /// Width of its front (outside), metres.
    pub width: f64,
    /// Height of its front wall above the roof, metres.
    pub height: f64,
    pub kind: DormerKind,
    /// Pitch of its roof, rise per run: for a shed dormer, rising
    /// backwards (flatter than the main roof); for a gable one, from its
    /// sides up to the ridge.
    pub pitch: f64,
    /// The material of its front and cheeks.
    pub wall_material: MaterialId,
    /// A window in its front.
    pub window: bool,
}

/// Where a dormer stands: the frame of the plane under its front.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DormerFrame {
    /// Centre of the front, in plan.
    pub origin: Point2,
    /// Down the plane, towards the eaves (unit, in plan).
    pub down: Vec2,
    /// Along the front (unit, in plan; to the right looking at it from
    /// outside).
    pub across: Vec2,
    /// Height of the roof's top face at the front centre, above the
    /// level.
    pub z0: f64,
    /// The main roof's pitch there.
    pub slope: f64,
}

/// A dormer worked out: its corners in a local frame (`x` across the
/// front, `t` back up the roof, `z` above the level).
#[derive(Debug, Clone, PartialEq)]
pub struct DormerShape {
    pub frame: DormerFrame,
    /// Where the front wall top is above the level.
    pub front_top: f64,
    /// How far back the dormer reaches (where its roof meets the main
    /// one): for a gable dormer, its ridge.
    pub depth: f64,
    /// For a gable dormer, how far back its eaves reach (where its
    /// valleys meet the main roof's... the cheeks' back corners).
    pub eave_depth: f64,
    /// Ridge height above the level (gable), or the front top (shed).
    pub ridge: f64,
}

/// Largest dormer, metres.
pub const MAX_DORMER: f64 = 10.0;

impl DormerFrame {
    /// A point of the frame, `x` across and `t` back up the roof.
    pub fn at(&self, x: f64, t: f64) -> Point2 {
        self.origin + self.across * x - self.down * t
    }

    /// The main roof's top face at `t` back from the front.
    pub fn roof_z(&self, t: f64) -> f64 {
        self.z0 + self.slope * t
    }
}

impl DormerShape {
    /// The outline in plan of where it stands on the roof (its cheeks and
    /// front), counter-clockwise.
    pub fn footprint(&self, d: &Dormer) -> Vec<Point2> {
        let f = &self.frame;
        let w = d.width / 2.0;
        let mut v = vec![f.at(-w, 0.0), f.at(w, 0.0)];
        match d.kind {
            DormerKind::Shed => {
                v.push(f.at(w, self.depth));
                v.push(f.at(-w, self.depth));
            }
            DormerKind::Gable => {
                v.push(f.at(w, self.eave_depth));
                v.push(f.at(0.0, self.depth));
                v.push(f.at(-w, self.eave_depth));
            }
        }
        if signed_area(&v) < 0.0 {
            v.reverse();
        }
        v
    }
}

impl Roof {
    /// The frame of the plane under `p`, if a sloping plane covers it.
    pub fn frame_at(&self, p: Point2) -> Option<DormerFrame> {
        let plane = self
            .planes()
            .into_iter()
            .find(|pl| point_in_polygon(p, &pl.outline))?;
        let up = plane.slope.direction()?;
        let down = up * -1.0;
        // Looking at the front from outside (facing up the roof), the
        // right is the downward direction turned a quarter to the left.
        let across = down.perp_left();
        Some(DormerFrame {
            origin: p,
            down,
            across,
            z0: self.top_at(p),
            slope: plane.slope.slope,
        })
    }

    /// A dormer's shape on this roof, or why it cannot stand there.
    pub fn dormer_shape(&self, d: &Dormer) -> Result<DormerShape, ModelError> {
        let bad = ModelError::InvalidShape;
        finite_point(d.position, "dormer position")?;
        for (v, what) in [
            (d.width, "dormer width"),
            (d.height, "dormer height"),
            (d.pitch, "dormer pitch"),
        ] {
            finite(v, what)?;
        }
        if d.width <= 0.1 || d.width > MAX_DORMER || d.height <= 0.1 || d.height > MAX_DORMER {
            return Err(bad(
                "a dormer's width and height must be between 0,1 and 10 m",
            ));
        }
        if d.pitch < 0.0 || d.pitch > MAX_SLAB_SLOPE {
            return Err(bad("a dormer's pitch must be between 0 and 300 %"));
        }
        let frame = self
            .frame_at(d.position)
            .ok_or(bad("a dormer must stand on a sloping plane of its roof"))?;
        let s = frame.slope;
        if s < 0.05 {
            return Err(bad("the roof is too flat there for a dormer"));
        }
        let front_top = frame.z0 + d.height;
        let shape = match d.kind {
            DormerKind::Shed => {
                if d.pitch >= s - 0.02 {
                    return Err(bad(
                        "a shed dormer's roof must be flatter than the roof it stands on",
                    ));
                }
                let depth = d.height / (s - d.pitch);
                DormerShape {
                    frame,
                    front_top,
                    depth,
                    eave_depth: depth,
                    ridge: front_top,
                }
            }
            DormerKind::Gable => {
                let ridge = front_top + d.pitch * d.width / 2.0;
                DormerShape {
                    frame,
                    front_top,
                    depth: (ridge - frame.z0) / s,
                    eave_depth: d.height / s,
                    ridge,
                }
            }
        };
        if shape.depth > MAX_DORMER * 2.0 {
            return Err(bad("the dormer would reach too far up the roof"));
        }
        // All of it on the plane it stands on (not over a ridge, a hip or
        // a valley, nor off the eaves).
        let plane = self
            .planes()
            .into_iter()
            .find(|pl| point_in_polygon(d.position, &pl.outline))
            .ok_or(bad("a dormer must stand on a sloping plane of its roof"))?;
        let foot = shape.footprint(d);
        let c = crate::centroid(&foot);
        let inside = foot.iter().all(|p| {
            let q = *p
                + (c - *p)
                    .normalized()
                    .map_or(Vec2::new(0.0, 0.0), |u| u * 0.01);
            point_in_polygon(q, &plane.outline)
        });
        if !inside {
            return Err(bad(
                "the dormer does not fit on its plane of the roof: make it narrower, lower or flatter, or move it down",
            ));
        }
        Ok(shape)
    }
}

impl Model {
    /// Puts a dormer on a roof; returns its number among the roof's.
    pub fn add_dormer(&mut self, roof: RoofId, dormer: Dormer) -> Result<usize, ModelError> {
        let r = self.roofs.get(&roof).ok_or(ModelError::UnknownRoof(roof))?;
        if self.material(dormer.wall_material).is_none() {
            return Err(ModelError::UnknownMaterial(dormer.wall_material));
        }
        r.dormer_shape(&dormer)?;
        let r = self.roofs.get_mut(&roof).expect("checked");
        r.dormers.push(dormer);
        Ok(r.dormers.len() - 1)
    }

    /// Replaces a roof's dormers; nothing changes if one is not valid.
    pub fn set_roof_dormers(
        &mut self,
        roof: RoofId,
        dormers: Vec<Dormer>,
    ) -> Result<(), ModelError> {
        let r = self.roofs.get(&roof).ok_or(ModelError::UnknownRoof(roof))?;
        for d in &dormers {
            if self.material(d.wall_material).is_none() {
                return Err(ModelError::UnknownMaterial(d.wall_material));
            }
            r.dormer_shape(d)?;
        }
        self.roofs.get_mut(&roof).expect("checked").dormers = dormers;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn roof() -> (Model, RoofId, MaterialId) {
        let mut m = Model::new("t");
        let l = m.add_level("PB", 0.0);
        let mat = m.add_material("Teja", "ceramic", [180, 80, 60], CutPattern::Solid);
        let st = m
            .add_slab_type(
                "Cubierta",
                SlabFunction::Roof,
                vec![Layer {
                    material: mat,
                    thickness: 0.25,
                    function: LayerFunction::Structure,
                }],
            )
            .unwrap();
        let r = m
            .add_roof(NewRoof {
                roof_type: st,
                level: l,
                outline: vec![
                    Point2::new(0.0, 0.0),
                    Point2::new(10.0, 0.0),
                    Point2::new(10.0, 8.0),
                    Point2::new(0.0, 8.0),
                ],
                // A gable roof: the long sides slope at 60 %.
                slopes: vec![Some(0.6), None, Some(0.6), None],
                base_offset: 3.0,
                overhang: 0.4,
            })
            .unwrap();
        (m, r, mat)
    }

    fn dormer(kind: DormerKind, pitch: f64, mat: MaterialId) -> Dormer {
        Dormer {
            position: Point2::new(5.0, 1.0),
            width: 1.6,
            height: 1.2,
            kind,
            pitch,
            wall_material: mat,
            window: true,
        }
    }

    #[test]
    fn a_shed_dormer_reaches_back_until_its_roof_meets_the_main_one() {
        let (mut m, r, mat) = roof();
        let d = dormer(DormerKind::Shed, 0.1, mat);
        m.add_dormer(r, d.clone()).unwrap();
        let roof = m.roof(r).unwrap();
        let s = roof.dormer_shape(&d).unwrap();
        // On the south plane, facing south.
        assert!(s.frame.down.y < -0.99, "{:?}", s.frame.down);
        assert!((s.frame.z0 - (3.0 + 0.6)).abs() < 1e-9);
        // 1,2 m over 60 % − 10 %: 2,4 m back.
        assert!((s.depth - 2.4).abs() < 1e-9);
        let f = s.footprint(&d);
        assert_eq!(f.len(), 4);
        assert!(signed_area(&f) > 0.0);
        // Its back meets the roof at its own roof's height.
        let back = s.frame.roof_z(s.depth);
        assert!((back - (s.front_top + 0.1 * s.depth)).abs() < 1e-9);
    }

    #[test]
    fn a_gable_dormer_has_its_ridge_further_back_than_its_eaves() {
        let (mut m, r, mat) = roof();
        let d = dormer(DormerKind::Gable, 0.5, mat);
        m.add_dormer(r, d.clone()).unwrap();
        let s = m.roof(r).unwrap().dormer_shape(&d).unwrap();
        assert!((s.eave_depth - 2.0).abs() < 1e-9);
        assert!((s.ridge - (s.front_top + 0.4)).abs() < 1e-9);
        assert!(s.depth > s.eave_depth);
        assert_eq!(s.footprint(&d).len(), 5);
    }

    #[test]
    fn dormers_that_cannot_stand_are_refused() {
        let (mut m, r, mat) = roof();
        // Steeper than the roof.
        assert!(m.add_dormer(r, dormer(DormerKind::Shed, 0.7, mat)).is_err());
        // A ridge so high it would cross the main one.
        assert!(m
            .add_dormer(r, dormer(DormerKind::Gable, 1.0, mat))
            .is_err());
        // Off the roof.
        let mut d = dormer(DormerKind::Shed, 0.1, mat);
        d.position = Point2::new(30.0, 1.0);
        assert!(m.add_dormer(r, d).is_err());
        assert!(m.roof(r).unwrap().dormers.is_empty());
    }
}
