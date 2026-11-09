//! Walls of variable section: a batter («talud») on one face or both, and
//! a strip footing («zarpa») at the base, for retaining walls and masonry
//! walls of stone or earth.
//!
//! The wall type's layers give the thickness at the top (the crown,
//! «coronación»). Going down, each battered face leans out by its batter
//! (horizontal per vertical: 0,1 is 1:10), and the layer that varies (the
//! structural one, unless another is given) takes up the difference; the
//! other layers keep their thickness and follow their face. The footing,
//! when there is one, is part of the wall: it takes the bottom `depth` of
//! its height, `toe_ext` and `toe_int` wider than the foot of the faces
//! on each side, in the material of the layer that varies.
//!
//! The section can be the type's (every wall of the type) or the wall's
//! own, which wins: a wall can be given a plain section,
//! [`WallSection::default`], to leave out the type's. Straight walls only.

use super::*;

/// A wall's section across: its batters and its footing.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct WallSection {
    /// How far the exterior face leans out per metre down (0,1 = 1:10).
    pub batter_ext: f64,
    /// The same for the interior face.
    pub batter_int: f64,
    /// The layer that grows (index from the exterior); `None`: the first
    /// structural layer, or the thickest one.
    pub layer: Option<usize>,
    pub footing: Option<WallFooting>,
}

/// A strip footing at the base of a wall, part of it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WallFooting {
    /// How far it comes out past the foot of the exterior face.
    pub toe_ext: f64,
    /// How far it comes out past the foot of the interior face.
    pub toe_int: f64,
    /// Its depth («canto»), from the base of the wall up.
    pub depth: f64,
}

/// The batter of a retaining wall's exterior face when one is asked for
/// (1:10), and the footing that goes with it.
pub const USUAL_BATTER: f64 = 0.1;
pub const USUAL_FOOTING: WallFooting = WallFooting {
    toe_ext: 0.6,
    toe_int: 0.4,
    depth: 0.5,
};

impl WallSection {
    /// Whether it changes nothing.
    pub fn is_plain(&self) -> bool {
        self.batter_ext.abs() < 1e-9 && self.batter_int.abs() < 1e-9 && self.footing.is_none()
    }

    /// A batter given as a ratio «1:n» (one across for `n` up).
    pub fn ratio(n: f64) -> f64 {
        if n > 0.0 {
            1.0 / n
        } else {
            0.0
        }
    }

    /// A batter given as the angle of the face from the vertical, degrees.
    pub fn from_angle(degrees: f64) -> f64 {
        degrees.to_radians().tan()
    }

    /// The batters that make a wall `crown` thick at the top `base` thick at
    /// the foot of its faces, over `stem` of height, shared between the faces
    /// as `share_ext` (1: all on the exterior, 0.5 both alike).
    pub fn batters_for(crown: f64, base: f64, stem: f64, share_ext: f64) -> (f64, f64) {
        if stem <= EPSILON {
            return (0.0, 0.0);
        }
        let b = ((base - crown) / stem).max(0.0);
        (b * share_ext, b * (1.0 - share_ext))
    }

    /// The layer that grows among `layers`.
    pub fn varying_layer(&self, layers: &[Layer]) -> usize {
        if let Some(i) = self.layer.filter(|i| *i < layers.len()) {
            return i;
        }
        layers
            .iter()
            .position(|l| l.function == LayerFunction::Structure)
            .or_else(|| {
                layers
                    .iter()
                    .enumerate()
                    .max_by(|a, b| a.1.thickness.total_cmp(&b.1.thickness))
                    .map(|(i, _)| i)
            })
            .unwrap_or(0)
    }

    /// The depth of its footing (0 without one).
    pub fn footing_depth(&self) -> f64 {
        self.footing.map_or(0.0, |f| f.depth)
    }

    /// How far the exterior and the interior face are out from the crown at
    /// `z` above the base of a wall `height` high (at most at the top of the
    /// footing: below it is the footing).
    pub fn leans_at(&self, height: f64, z: f64) -> (f64, f64) {
        let z = z.max(self.footing_depth());
        let down = height - z;
        (self.batter_ext * down, self.batter_int * down)
    }

    /// The thickness at the foot of the faces (on the footing, or at the
    /// base) of a wall `crown` thick and `height` high.
    pub fn base_thickness(&self, crown: f64, height: f64) -> f64 {
        let (e, i) = self.leans_at(height, 0.0);
        crown + e + i
    }

    /// The area of the section of a wall `crown` thick and `height` high:
    /// the trapezoid of the stem and the footing.
    pub fn area(&self, crown: f64, height: f64) -> f64 {
        let d = self.footing_depth();
        let base = self.base_thickness(crown, height);
        let stem = (height - d).max(0.0) * (crown + base) / 2.0;
        let foot = self
            .footing
            .map_or(0.0, |f| (base + f.toe_ext + f.toe_int) * f.depth);
        stem + foot
    }

    fn check(&self, height: Option<f64>) -> Result<(), ModelError> {
        finite(self.batter_ext, "wall batter")?;
        finite(self.batter_int, "wall batter")?;
        if self.batter_ext < 0.0 || self.batter_int < 0.0 {
            return Err(ModelError::NonPositive("wall batter"));
        }
        if self.batter_ext > 2.0 || self.batter_int > 2.0 {
            return Err(ModelError::NonPositive("wall batter (at most 2:1)"));
        }
        if let Some(f) = self.footing {
            finite(f.toe_ext, "footing toe")?;
            finite(f.toe_int, "footing toe")?;
            finite(f.depth, "footing depth")?;
            if f.toe_ext < 0.0 || f.toe_int < 0.0 {
                return Err(ModelError::NonPositive("footing toe"));
            }
            if f.depth <= EPSILON {
                return Err(ModelError::NonPositive("footing depth"));
            }
            if height.is_some_and(|h| f.depth >= h - EPSILON) {
                return Err(ModelError::NonPositive("wall height over its footing"));
            }
        }
        Ok(())
    }
}

impl Model {
    /// The section a wall is built with: its own, or its type's; `None`
    /// when it is plain (or the wall is curved).
    pub fn wall_section(&self, id: WallId) -> Option<WallSection> {
        let w = self.wall(id)?;
        if w.is_curved() {
            return None;
        }
        let s = match w.section {
            Some(s) => s,
            None => self.wall_type(w.wall_type)?.section?,
        };
        (!s.is_plain()).then_some(s)
    }

    /// The wall's own section (`None`: its type's). A plain one leaves the
    /// type's out.
    pub fn set_wall_section(
        &mut self,
        id: WallId,
        section: Option<WallSection>,
    ) -> Result<(), ModelError> {
        let w = self.walls.get(&id).ok_or(ModelError::UnknownWall(id))?;
        if let Some(s) = &section {
            s.check(Some(w.height))?;
        }
        self.edit_wall(id, |w| w.section = section)
    }

    /// The section of every wall of a type that has none of its own.
    pub fn set_wall_type_section(
        &mut self,
        id: WallTypeId,
        section: Option<WallSection>,
    ) -> Result<(), ModelError> {
        if let Some(s) = &section {
            s.check(None)?;
        }
        let t = self
            .wall_types
            .get_mut(&id)
            .ok_or(ModelError::UnknownWallType(id))?;
        t.section = section.filter(|s| !s.is_plain());
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn batters_thicknesses_and_area() {
        assert!((WallSection::ratio(10.0) - 0.1).abs() < 1e-12);
        assert!((WallSection::from_angle(45.0) - 1.0).abs() < 1e-12);
        let s = WallSection {
            batter_ext: 0.1,
            batter_int: 0.0,
            layer: None,
            footing: Some(WallFooting {
                toe_ext: 0.6,
                toe_int: 0.4,
                depth: 0.5,
            }),
        };
        // 3 m wall, 0,5 m footing: 2,5 m of stem at 1:10, 0,30 → 0,55.
        assert!((s.base_thickness(0.3, 3.0) - 0.55).abs() < 1e-12);
        let area = 2.5 * (0.3 + 0.55) / 2.0 + (0.55 + 1.0) * 0.5;
        assert!((s.area(0.3, 3.0) - area).abs() < 1e-12);
        let (b, c) = WallSection::batters_for(0.3, 0.55, 2.5, 1.0);
        assert!((b - 0.1).abs() < 1e-12 && c.abs() < 1e-12);
        assert!(WallSection::default().is_plain());
    }

    #[test]
    fn the_wall_wins_over_its_type() {
        let mut m = Model::new("T");
        let level = m.add_level("Planta baja", 0.0);
        let mat = m.add_material("Hormigón", "concrete", [200, 200, 200], CutPattern::Cross);
        let t = m
            .add_wall_type(
                "Muro de contención",
                WallFunction::Exterior,
                vec![Layer {
                    material: mat,
                    thickness: 0.3,
                    function: LayerFunction::Structure,
                }],
            )
            .unwrap();
        let w = m
            .add_wall(NewWall {
                wall_type: t,
                level,
                start: Point2::new(0.0, 0.0),
                end: Point2::new(5.0, 0.0),
                reference: ReferenceLine::Center,
                height: 3.0,
                base_offset: 0.0,
                bulge: 0.0,
                top: WallTop::Unconnected,
            })
            .unwrap();
        assert!(m.wall_section(w).is_none());
        let talud = WallSection {
            batter_ext: 0.1,
            ..WallSection::default()
        };
        m.set_wall_type_section(t, Some(talud)).unwrap();
        assert_eq!(m.wall_section(w), Some(talud));
        m.set_wall_section(w, Some(WallSection::default())).unwrap();
        assert!(m.wall_section(w).is_none());
        let deep = WallSection {
            footing: Some(WallFooting {
                toe_ext: 0.0,
                toe_int: 0.0,
                depth: 3.5,
            }),
            ..WallSection::default()
        };
        assert!(m.set_wall_section(w, Some(deep)).is_err());
        assert!(m
            .set_wall_type_section(
                t,
                Some(WallSection {
                    batter_ext: -0.1,
                    ..WallSection::default()
                })
            )
            .is_err());
    }
}
