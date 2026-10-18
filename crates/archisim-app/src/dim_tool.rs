//! The dimension tool ("Cota"), without any user interface.
//!
//! Three ways of working, chosen in the info box:
//! - **Lineal**: click the points to measure (with the usual snaps), then
//!   Enter, a double click or the right button, then a click where the
//!   dimension line goes. Two points give a single dimension, more give a
//!   chain. The direction is horizontal or vertical depending on which side
//!   the line is placed, unless it is fixed (horizontal, vertical or aligned
//!   with the first two points).
//! - **Huecos de muro**: a click on a wall measures that face of the wall
//!   with the jambs of its doors and windows (the face on the side of the
//!   click); a second click places the line.
//! - **Cota de nivel**: a click places a level mark.

use archisim_core::{LevelId, NewDimension, Point2};
use archisim_geom::wall_face_chain;

use crate::pick::{pick, Element};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DimMode {
    #[default]
    Linear,
    WallOpenings,
    LevelMark,
}

/// Direction of linear dimensions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DimDirection {
    /// Horizontal or vertical by where the line is placed.
    #[default]
    Automatic,
    Horizontal,
    Vertical,
    /// Along the first two points.
    Aligned,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct DimSettings {
    pub mode: DimMode,
    pub direction: DimDirection,
}

/// What a click or key asks for.
#[derive(Debug, Clone, PartialEq)]
pub enum Outcome {
    Nothing,
    /// Build this dimension.
    Dimension(NewDimension),
    /// Place a level mark here.
    Mark(Point2),
    /// The click in "Huecos de muro" mode did not land on a wall.
    NoWall,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct DimTool {
    points: Vec<Point2>,
    /// Points are chosen; the next click places the line.
    placing: bool,
    /// Fixed direction (wall openings: along the wall).
    fixed: Option<Point2>,
}

impl DimTool {
    pub fn points(&self) -> &[Point2] {
        &self.points
    }

    pub fn is_placing(&self) -> bool {
        self.placing
    }

    pub fn is_active(&self) -> bool {
        !self.points.is_empty()
    }

    pub fn finish(&mut self) {
        *self = Self::default();
    }

    /// A click at `p` (snapped).
    pub fn click<G: crate::geo::Geo + ?Sized>(
        &mut self,
        geo: &G,
        level: LevelId,
        settings: DimSettings,
        p: Point2,
        reach: f64,
    ) -> Outcome {
        let model = geo.model();
        if self.placing {
            let out = match self.build(level, settings, p) {
                Some(d) => Outcome::Dimension(d),
                None => Outcome::Nothing,
            };
            self.finish();
            return out;
        }
        match settings.mode {
            DimMode::LevelMark => Outcome::Mark(p),
            DimMode::Linear => {
                if self.points.last().is_none_or(|q| !q.approx_eq(p)) {
                    self.points.push(p);
                }
                Outcome::Nothing
            }
            DimMode::WallOpenings => {
                let wall = match pick(geo, level, p, reach) {
                    Some(Element::Wall(w)) => Some(w),
                    Some(Element::Opening(o)) => model.opening(o).map(|o| o.wall),
                    _ => None,
                };
                let Some(wall) = wall else {
                    return Outcome::NoWall;
                };
                // The face on the side of the click.
                let ext = wall_face_chain(model, wall, true);
                let int = wall_face_chain(model, wall, false);
                let side = |c: &archisim_geom::FaceChain| (p - c.points[0]).dot(c.outward).abs();
                let chain = match (ext, int) {
                    (Some(e), Some(i)) => {
                        if side(&e) <= side(&i) {
                            e
                        } else {
                            i
                        }
                    }
                    (Some(c), None) | (None, Some(c)) => c,
                    (None, None) => return Outcome::NoWall,
                };
                self.points = chain.points;
                self.fixed = Some(chain.direction);
                self.placing = true;
                Outcome::Nothing
            }
        }
    }

    /// Enter, double click or right button: stop adding points and place
    /// the line. Returns whether there are enough points.
    pub fn enough(&mut self) -> bool {
        if self.points.len() >= 2 {
            self.placing = true;
        }
        self.placing
    }

    /// Esc: back one step. Returns `false` when there was nothing to undo.
    pub fn escape(&mut self) -> bool {
        if self.placing {
            if self.fixed.is_some() {
                self.finish();
            } else {
                self.placing = false;
            }
            true
        } else {
            self.points.pop().is_some()
        }
    }

    /// The direction the dimension would measure in, with its line through
    /// `at`.
    pub fn direction(&self, settings: DimSettings, at: Point2) -> Option<Point2> {
        if let Some(d) = self.fixed {
            return Some(d);
        }
        let (first, second) = (*self.points.first()?, *self.points.get(1)?);
        let along = (second - first).normalized()?;
        match settings.direction {
            DimDirection::Horizontal => Some(Point2::new(1.0, 0.0)),
            DimDirection::Vertical => Some(Point2::new(0.0, 1.0)),
            DimDirection::Aligned => Some(along),
            DimDirection::Automatic => {
                // Outside the points' box above or below: horizontal; to
                // the left or right: vertical.
                let (lo, hi) = self.points.iter().fold(
                    (
                        Point2::new(f64::INFINITY, f64::INFINITY),
                        Point2::new(f64::NEG_INFINITY, f64::NEG_INFINITY),
                    ),
                    |(lo, hi), p| {
                        (
                            Point2::new(lo.x.min(p.x), lo.y.min(p.y)),
                            Point2::new(hi.x.max(p.x), hi.y.max(p.y)),
                        )
                    },
                );
                let out_x = (lo.x - at.x).max(at.x - hi.x).max(0.0);
                let out_y = (lo.y - at.y).max(at.y - hi.y).max(0.0);
                if out_y >= out_x {
                    Some(Point2::new(1.0, 0.0))
                } else {
                    Some(Point2::new(0.0, 1.0))
                }
            }
        }
    }

    /// The dimension that a click at `at` would build now.
    pub fn build(&self, level: LevelId, settings: DimSettings, at: Point2) -> Option<NewDimension> {
        if self.points.len() < 2 {
            return None;
        }
        Some(NewDimension {
            level,
            points: self.points.clone(),
            direction: self.direction(settings, at)?,
            position: at,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pick::tests::room;

    fn p(x: f64, y: f64) -> Point2 {
        Point2::new(x, y)
    }

    #[test]
    fn linear_chain_then_place() {
        let (m, lvl, _) = room();
        let s = DimSettings::default();
        let mut t = DimTool::default();
        for q in [p(0.0, 0.0), p(2.0, 0.0), p(6.0, 0.0)] {
            assert_eq!(t.click(&m, lvl, s, q, 0.05), Outcome::Nothing);
        }
        assert!(t.enough());
        // Placed below: horizontal.
        let Outcome::Dimension(d) = t.click(&m, lvl, s, p(3.0, -1.0), 0.05) else {
            panic!()
        };
        assert_eq!(d.direction, p(1.0, 0.0));
        assert_eq!(d.points.len(), 3);
        assert!(!t.is_active());
        // Two points placed to the side: vertical.
        t.click(&m, lvl, s, p(0.0, 0.0), 0.05);
        t.click(&m, lvl, s, p(2.0, 4.0), 0.05);
        t.enough();
        assert_eq!(t.direction(s, p(-1.0, 2.0)), Some(p(0.0, 1.0)));
        assert_eq!(t.direction(s, p(1.0, 5.0)), Some(p(1.0, 0.0)));
        // Aligned with the pair when asked.
        assert!(t.escape());
        let aligned = DimSettings {
            direction: DimDirection::Aligned,
            ..s
        };
        let along = t.direction(aligned, p(5.0, 1.0)).unwrap();
        assert!((along.x - 2.0 / 20f64.sqrt()).abs() < 1e-9);
        // Esc steps back: points, then nothing left.
        assert!(t.escape());
        assert!(t.escape());
        assert!(!t.escape());
    }

    #[test]
    fn wall_openings_and_marks() {
        let (m, lvl, _) = room();
        let s = DimSettings {
            mode: DimMode::WallOpenings,
            ..Default::default()
        };
        let mut t = DimTool::default();
        // Outside the south wall, near its exterior face (y = 0).
        assert_eq!(t.click(&m, lvl, s, p(1.0, -0.02), 0.05), Outcome::Nothing);
        assert!(t.is_placing());
        // Ends of the face and the two jambs of the 0.8 m door at x = 3.
        let xs: Vec<f64> = t.points().iter().map(|q| q.x).collect();
        assert_eq!(xs.len(), 4, "{xs:?}");
        assert!(t.points().iter().all(|q| q.y.abs() < 1e-9));
        let Outcome::Dimension(d) = t.click(&m, lvl, s, p(3.0, -0.8), 0.05) else {
            panic!()
        };
        assert_eq!(d.position, p(3.0, -0.8));
        // Inside the room near the interior face: that face.
        t.click(&m, lvl, s, p(1.0, 0.3), 0.05);
        assert!(t.points().iter().all(|q| (q.y - 0.265).abs() < 1e-9));
        t.finish();
        // Nothing there.
        assert_eq!(t.click(&m, lvl, s, p(3.0, 2.0), 0.05), Outcome::NoWall);
        // Level marks.
        let s = DimSettings {
            mode: DimMode::LevelMark,
            ..Default::default()
        };
        assert_eq!(
            t.click(&m, lvl, s, p(3.0, 2.0), 0.05),
            Outcome::Mark(p(3.0, 2.0))
        );
    }
}
