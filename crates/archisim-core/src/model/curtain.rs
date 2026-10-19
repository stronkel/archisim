//! Curtain walls ("muros cortina"): a grid of mullions and transoms with
//! panels between them, along a straight line, as Revit's curtain walls
//! and ArchiCAD's curtain wall tool in their simplest form.
//!
//! The grid divides the length and the height into equal bays no wider
//! (or taller) than the spacing asked for. Mullions stand at both ends and
//! between bays; transoms run at the bottom, the top and between rows;
//! each cell has a panel (glass by default) in the middle of the depth.
//!
//! IFC: `IfcCurtainWall` made of `IfcMember` (`MULLION`) and `IfcPlate`
//! (`CURTAIN_PANEL`).

use super::*;

#[derive(Debug, Clone, PartialEq)]
pub struct CurtainWall {
    pub id: CurtainWallId,
    pub guid: GlobalId,
    pub level: LevelId,
    pub start: Point2,
    pub end: Point2,
    pub base_offset: f64,
    /// Up to a level, or by `height` (as walls).
    pub top: WallTop,
    pub height: f64,
    /// Largest distance between mullions.
    pub bay: f64,
    /// Largest distance between transoms.
    pub row: f64,
    /// Width of mullions and transoms seen from outside.
    pub frame_width: f64,
    /// Depth of mullions and transoms across the wall.
    pub frame_depth: f64,
    pub panel_thickness: f64,
    pub frame_material: MaterialId,
    pub panel_material: MaterialId,
}

/// Data needed to create a curtain wall.
#[derive(Debug, Clone, PartialEq)]
pub struct NewCurtainWall {
    pub level: LevelId,
    pub start: Point2,
    pub end: Point2,
    pub base_offset: f64,
    pub top: WallTop,
    pub height: f64,
    pub bay: f64,
    pub row: f64,
    pub frame_width: f64,
    pub frame_depth: f64,
    pub panel_thickness: f64,
    pub frame_material: MaterialId,
    pub panel_material: MaterialId,
}

/// What a piece of a curtain wall is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CurtainPieceKind {
    /// A vertical frame member (IFC `IfcMember` `MULLION`).
    Mullion,
    /// A horizontal frame member (also `MULLION` in IFC, as Revit
    /// exports them).
    Transom,
    /// A panel between frames (IFC `IfcPlate` `CURTAIN_PANEL`).
    Panel,
}

/// A box of a curtain wall in its own frame: `along` from the start of
/// its line, `across` to the left of it (outside), `up` from its base.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CurtainPiece {
    pub kind: CurtainPieceKind,
    pub along: (f64, f64),
    pub across: (f64, f64),
    pub up: (f64, f64),
}

/// Where the grid lines of a curtain wall are: along it from its start,
/// and up from its base.
#[derive(Debug, Clone, PartialEq)]
pub struct CurtainGrid {
    pub mullions: Vec<f64>,
    pub transoms: Vec<f64>,
}

impl CurtainWall {
    pub fn length(&self) -> f64 {
        self.start.distance(self.end)
    }

    pub fn grid(&self) -> CurtainGrid {
        let div = |total: f64, max: f64| {
            let n = (total / max - 1e-9).ceil().max(1.0) as usize;
            (0..=n).map(|i| total * i as f64 / n as f64).collect()
        };
        CurtainGrid {
            mullions: div(self.length(), self.bay),
            transoms: div(self.height, self.row),
        }
    }

    /// Height of its base above project zero, given its level's.
    pub fn base_z(&self, level_elevation: f64) -> f64 {
        level_elevation + self.base_offset
    }

    /// Mullions at every grid line (flush with the ends), transoms
    /// between them at every row line (flush with the base and the top),
    /// and a panel in each cell, centred in the depth.
    pub fn pieces(&self) -> Vec<CurtainPiece> {
        let g = self.grid();
        let w = self.frame_width;
        let d = self.frame_depth / 2.0;
        let t = self.panel_thickness / 2.0;
        // Extent of the frame member at grid line i of n.
        let band = |lines: &[f64], i: usize| {
            let x = lines[i];
            if i == 0 {
                (x, x + w)
            } else if i + 1 == lines.len() {
                (x - w, x)
            } else {
                (x - w / 2.0, x + w / 2.0)
            }
        };
        let (ms, ts) = (&g.mullions, &g.transoms);
        let mut out = Vec::new();
        for i in 0..ms.len() {
            out.push(CurtainPiece {
                kind: CurtainPieceKind::Mullion,
                along: band(ms, i),
                across: (-d, d),
                up: (0.0, self.height),
            });
        }
        for i in 0..ms.len() - 1 {
            let along = (band(ms, i).1, band(ms, i + 1).0);
            for j in 0..ts.len() {
                out.push(CurtainPiece {
                    kind: CurtainPieceKind::Transom,
                    along,
                    across: (-d, d),
                    up: band(ts, j),
                });
            }
            for j in 0..ts.len() - 1 {
                out.push(CurtainPiece {
                    kind: CurtainPieceKind::Panel,
                    along,
                    across: (-t, t),
                    up: (band(ts, j).1, band(ts, j + 1).0),
                });
            }
        }
        out
    }

    /// A piece's footprint in plan, counter-clockwise.
    pub fn piece_footprint(&self, p: &CurtainPiece) -> [Point2; 4] {
        let u = (self.end - self.start)
            .normalized()
            .unwrap_or(Vec2::new(1.0, 0.0));
        let n = u.perp_left();
        let at = |a: f64, v: f64| self.start + u * a + n * v;
        [
            at(p.along.0, p.across.0),
            at(p.along.1, p.across.0),
            at(p.along.1, p.across.1),
            at(p.along.0, p.across.1),
        ]
    }

    /// The outline in plan: the line of the wall as deep as its frame.
    pub fn outline(&self) -> Vec<Point2> {
        let u = (self.end - self.start)
            .normalized()
            .unwrap_or(Vec2::new(1.0, 0.0));
        let n = u.perp_left() * (self.frame_depth / 2.0);
        vec![self.start - n, self.end - n, self.end + n, self.start + n]
    }
}

impl Model {
    fn curtain_height(
        &self,
        level: LevelId,
        base: f64,
        top: WallTop,
        height: f64,
    ) -> Result<f64, ModelError> {
        match top {
            WallTop::Unconnected => Ok(height),
            WallTop::Level { level: to, offset } => {
                let z0 = self
                    .levels
                    .get(&level)
                    .ok_or(ModelError::UnknownLevel(level))?
                    .elevation;
                let z1 = self
                    .levels
                    .get(&to)
                    .ok_or(ModelError::UnknownLevel(to))?
                    .elevation;
                Ok(z1 + offset - z0 - base)
            }
        }
    }

    fn validate_curtain(&self, c: &NewCurtainWall) -> Result<f64, ModelError> {
        if !self.levels.contains_key(&c.level) {
            return Err(ModelError::UnknownLevel(c.level));
        }
        for m in [c.frame_material, c.panel_material] {
            if !self.materials.contains_key(&m) {
                return Err(ModelError::UnknownMaterial(m));
            }
        }
        finite_point(c.start, "curtain wall start")?;
        finite_point(c.end, "curtain wall end")?;
        for (v, what) in [
            (c.base_offset, "curtain wall base offset"),
            (c.height, "curtain wall height"),
            (c.bay, "curtain wall bay"),
            (c.row, "curtain wall row"),
            (c.frame_width, "curtain wall frame width"),
            (c.frame_depth, "curtain wall frame depth"),
            (c.panel_thickness, "curtain wall panel thickness"),
        ] {
            finite(v, what)?;
        }
        if c.start.approx_eq(c.end) {
            return Err(ModelError::ZeroLengthWall);
        }
        let height = self.curtain_height(c.level, c.base_offset, c.top, c.height)?;
        let ok = height > 0.05
            && c.bay >= 0.2
            && c.row >= 0.2
            && c.frame_width > 0.005
            && c.frame_width * 2.0 < c.bay.min(c.row)
            && c.frame_depth > 0.005
            && c.panel_thickness > 0.001
            && c.panel_thickness <= c.frame_depth;
        if !ok {
            return Err(ModelError::InvalidCurtainWall);
        }
        Ok(height)
    }

    pub fn add_curtain_wall(&mut self, spec: NewCurtainWall) -> Result<CurtainWallId, ModelError> {
        let height = self.validate_curtain(&spec)?;
        let id = CurtainWallId(self.take_id());
        self.curtain_walls.insert(
            id,
            CurtainWall {
                id,
                guid: GlobalId::new(),
                level: spec.level,
                start: spec.start,
                end: spec.end,
                base_offset: spec.base_offset,
                top: spec.top,
                height,
                bay: spec.bay,
                row: spec.row,
                frame_width: spec.frame_width,
                frame_depth: spec.frame_depth,
                panel_thickness: spec.panel_thickness,
                frame_material: spec.frame_material,
                panel_material: spec.panel_material,
            },
        );
        self.stamp_phase(id.0);
        Ok(id)
    }

    /// Changes a curtain wall; nothing changes if the result is not valid.
    pub fn edit_curtain_wall(
        &mut self,
        id: CurtainWallId,
        edit: impl FnOnce(&mut CurtainWall),
    ) -> Result<(), ModelError> {
        let mut c = self
            .curtain_walls
            .get(&id)
            .cloned()
            .ok_or(ModelError::UnknownCurtainWall(id))?;
        edit(&mut c);
        c.height = self.validate_curtain(&spec_of(&c))?;
        self.curtain_walls.insert(id, c);
        Ok(())
    }

    pub fn remove_curtain_wall(&mut self, id: CurtainWallId) -> Result<CurtainWall, ModelError> {
        self.curtain_walls
            .remove(&id)
            .ok_or(ModelError::UnknownCurtainWall(id))
    }

    pub fn curtain_wall(&self, id: CurtainWallId) -> Option<&CurtainWall> {
        self.curtain_walls.get(&id)
    }

    pub fn curtain_walls(&self) -> impl Iterator<Item = &CurtainWall> {
        self.curtain_walls.values()
    }

    pub fn curtain_walls_on_level(&self, level: LevelId) -> impl Iterator<Item = &CurtainWall> {
        self.curtain_walls
            .values()
            .filter(move |c| c.level == level)
    }

    /// Heights of curtain walls tied to a level, after levels change.
    pub(super) fn refresh_curtain_heights(&mut self) {
        let ids: Vec<CurtainWallId> = self.curtain_walls.keys().copied().collect();
        for id in ids {
            let c = self.curtain_walls[&id].clone();
            if let Ok(h) = self.curtain_height(c.level, c.base_offset, c.top, c.height) {
                if h > 0.05 {
                    if let Some(x) = self.curtain_walls.get_mut(&id) {
                        x.height = h;
                    }
                }
            }
        }
    }

    pub(super) fn restore_curtain_walls(
        &mut self,
        list: Vec<CurtainWall>,
        claim: &mut structure::Claim<'_>,
    ) -> Result<(), RestoreError> {
        for mut c in list {
            let name = format!("curtain wall {} ({})", c.id.0, c.guid);
            claim(c.id.0, Some(&c.guid))
                .and_then(|_| self.validate_curtain(&spec_of(&c)))
                .map(|h| c.height = h)
                .map_err(|error| RestoreError {
                    element: name,
                    error,
                })?;
            self.curtain_walls.insert(c.id, c);
        }
        Ok(())
    }
}

fn spec_of(c: &CurtainWall) -> NewCurtainWall {
    NewCurtainWall {
        level: c.level,
        start: c.start,
        end: c.end,
        base_offset: c.base_offset,
        top: c.top,
        height: c.height,
        bay: c.bay,
        row: c.row,
        frame_width: c.frame_width,
        frame_depth: c.frame_depth,
        panel_thickness: c.panel_thickness,
        frame_material: c.frame_material,
        panel_material: c.panel_material,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn grid_of_a_curtain_wall() {
        let mut m = Model::new("t");
        let pb = m.add_level("PB", 0.0);
        let p1 = m.add_level("P1", 3.0);
        let al = m.add_material("Aluminio", "aluminium", [120, 120, 125], CutPattern::Solid);
        let glass = m.add_material("Vidrio", "glass", [190, 220, 240], CutPattern::Solid);
        let id = m
            .add_curtain_wall(NewCurtainWall {
                level: pb,
                start: Point2::new(0.0, 0.0),
                end: Point2::new(5.0, 0.0),
                base_offset: 0.0,
                top: WallTop::Level {
                    level: p1,
                    offset: 0.0,
                },
                height: 0.0,
                bay: 1.5,
                row: 1.2,
                frame_width: 0.05,
                frame_depth: 0.15,
                panel_thickness: 0.024,
                frame_material: al,
                panel_material: glass,
            })
            .unwrap();
        let c = m.curtain_wall(id).unwrap();
        let g = c.grid();
        // 5 m in bays of 1,25 (4); 3 m in rows of 1,00 (3).
        assert_eq!(g.mullions.len(), 5);
        assert!((g.mullions[1] - 1.25).abs() < 1e-12);
        assert_eq!(g.transoms.len(), 4);
        let pieces = c.pieces();
        let count = |k| pieces.iter().filter(|p| p.kind == k).count();
        assert_eq!(count(CurtainPieceKind::Mullion), 5);
        assert_eq!(count(CurtainPieceKind::Transom), 16);
        assert_eq!(count(CurtainPieceKind::Panel), 12);
        // The first panel sits between the first two mullions.
        let panel = pieces
            .iter()
            .find(|p| p.kind == CurtainPieceKind::Panel)
            .unwrap();
        assert!((panel.along.0 - 0.05).abs() < 1e-12);
        assert!((panel.along.1 - 1.225).abs() < 1e-12);
        assert!((panel.up.0 - 0.05).abs() < 1e-12);
        m.set_level_elevation(p1, 3.3).unwrap();
        assert!((m.curtain_wall(id).unwrap().height - 3.3).abs() < 1e-12);
        assert_eq!(
            m.edit_curtain_wall(id, |c| c.frame_width = 1.0),
            Err(ModelError::InvalidCurtainWall)
        );
        let back = Model::from_parts(m.to_parts()).unwrap();
        assert_eq!(back, m);
    }
}
