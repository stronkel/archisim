//! Setting-out grids and foundations (`docs/DISCIPLINAS.md`, structure).
//!
//! A **grid line** ("eje") is a named straight line in plan, seen on every
//! level and in the sections it crosses. IFC: one `IfcGrid` with an
//! `IfcGridAxis` per line.
//!
//! A **footing** sits on a level, its top at the level plus `top_offset`
//! (usually below the floor) and `depth` deep:
//! - a **pad** ("zapata aislada"), a rectangle centred at a point; placed
//!   under a column, it follows the column;
//! - a **strip** ("zapata corrida") along a line; placed under a wall, it
//!   follows the wall's centre line;
//! - a **tie beam** ("viga de atado o de centrado") between two points.
//!
//! IFC: `IfcFooting` (`PAD_FOOTING`, `STRIP_FOOTING`, `FOOTING_BEAM`). A
//! raft ("losa de cimentación") is a slab of a raft type.

use super::*;

/// A setting-out grid line.
#[derive(Debug, Clone, PartialEq)]
pub struct GridLine {
    pub id: GridId,
    pub guid: GlobalId,
    /// «1», «2»… or «A», «B»…
    pub name: String,
    pub start: Point2,
    pub end: Point2,
}

/// Data needed to create a grid line.
#[derive(Debug, Clone, PartialEq)]
pub struct NewGrid {
    pub name: String,
    pub start: Point2,
    pub end: Point2,
}

/// What kind of footing, and where it is.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum FootingKind {
    /// A rectangle `width` × `length` centred at `position`, turned by
    /// `rotation` (radians); under `column`, it takes the column's centre
    /// and rotation.
    Pad {
        position: Point2,
        length: f64,
        rotation: f64,
        column: Option<ColumnId>,
    },
    /// Along a line, `width` wide; under `wall`, it takes the wall's
    /// centre line.
    Strip {
        start: Point2,
        end: Point2,
        wall: Option<WallId>,
    },
    /// Between two points, `width` wide.
    TieBeam { start: Point2, end: Point2 },
}

/// A footing. IFC: `IfcFooting`.
#[derive(Debug, Clone, PartialEq)]
pub struct Footing {
    pub id: FootingId,
    pub guid: GlobalId,
    pub level: LevelId,
    pub kind: FootingKind,
    pub material: MaterialId,
    /// Width across (and, for a pad, along its `x`), metres.
    pub width: f64,
    /// Depth of the footing below its top face, metres.
    pub depth: f64,
    /// Top face above the level (negative: below the floor).
    pub top_offset: f64,
}

/// Data needed to create a footing.
#[derive(Debug, Clone, PartialEq)]
pub struct NewFooting {
    pub level: LevelId,
    pub kind: FootingKind,
    pub material: MaterialId,
    pub width: f64,
    pub depth: f64,
    pub top_offset: f64,
}

impl Footing {
    /// Its outline in plan, counter-clockwise.
    pub fn outline(&self) -> Vec<Point2> {
        match self.kind {
            FootingKind::Pad {
                position,
                length,
                rotation,
                ..
            } => {
                let (c, s) = (rotation.cos(), rotation.sin());
                let (hx, hy) = (self.width / 2.0, length / 2.0);
                [(-hx, -hy), (hx, -hy), (hx, hy), (-hx, hy)]
                    .into_iter()
                    .map(|(x, y)| {
                        Point2::new(position.x + x * c - y * s, position.y + x * s + y * c)
                    })
                    .collect()
            }
            FootingKind::Strip { start, end, .. } | FootingKind::TieBeam { start, end } => {
                let Some(u) = (end - start).normalized() else {
                    return Vec::new();
                };
                let n = u.perp_left() * (self.width / 2.0);
                vec![start - n, end - n, end + n, start + n]
            }
        }
    }

    /// Its kind's name.
    pub fn kind_name(&self) -> &'static str {
        match self.kind {
            FootingKind::Pad { .. } => "Zapata aislada",
            FootingKind::Strip { .. } => "Zapata corrida",
            FootingKind::TieBeam { .. } => "Viga de atado",
        }
    }

    /// Its volume, m³.
    pub fn volume(&self) -> f64 {
        let o = self.outline();
        let n = o.len();
        let area: f64 = (0..n)
            .map(|i| o[i].x * o[(i + 1) % n].y - o[(i + 1) % n].x * o[i].y)
            .sum::<f64>()
            / 2.0;
        area.abs() * self.depth
    }
}

impl Model {
    // ---- Grid lines -------------------------------------------------------

    fn validate_grid(&self, g: &NewGrid) -> Result<(), ModelError> {
        finite_point(g.start, "grid line start")?;
        finite_point(g.end, "grid line end")?;
        if g.start.approx_eq(g.end) {
            return Err(ModelError::InvalidShape(
                "a grid line needs two different points",
            ));
        }
        Ok(())
    }

    pub fn add_grid(&mut self, spec: NewGrid) -> Result<GridId, ModelError> {
        self.validate_grid(&spec)?;
        let id = GridId(self.take_id());
        self.grids.insert(
            id,
            GridLine {
                id,
                guid: GlobalId::new(),
                name: spec.name,
                start: spec.start,
                end: spec.end,
            },
        );
        Ok(id)
    }

    pub fn edit_grid(
        &mut self,
        id: GridId,
        edit: impl FnOnce(&mut GridLine),
    ) -> Result<(), ModelError> {
        let mut g = self
            .grids
            .get(&id)
            .cloned()
            .ok_or(ModelError::UnknownElement(id.0))?;
        edit(&mut g);
        self.validate_grid(&NewGrid {
            name: g.name.clone(),
            start: g.start,
            end: g.end,
        })?;
        self.grids.insert(id, g);
        Ok(())
    }

    pub fn remove_grid(&mut self, id: GridId) -> Result<GridLine, ModelError> {
        self.grids
            .remove(&id)
            .ok_or(ModelError::UnknownElement(id.0))
    }

    pub fn grid(&self, id: GridId) -> Option<&GridLine> {
        self.grids.get(&id)
    }

    pub fn grids(&self) -> impl Iterator<Item = &GridLine> {
        self.grids.values()
    }

    /// The next name for a grid line drawn from `start` to `end`: lines
    /// running more up than across are numbered (1, 2…), the others take
    /// letters (A, B…), as grids are usually named.
    pub fn next_grid_name(&self, start: Point2, end: Point2) -> String {
        let d = end - start;
        let numbered = d.y.abs() >= d.x.abs();
        let taken: Vec<&str> = self.grids.values().map(|g| g.name.as_str()).collect();
        if numbered {
            (1..)
                .map(|n: usize| n.to_string())
                .find(|n| !taken.contains(&n.as_str()))
                .unwrap_or_default()
        } else {
            (0..26)
                .map(|i| ((b'A' + i as u8) as char).to_string())
                .chain((0..26).map(|i| format!("A{}", (b'A' + i as u8) as char)))
                .find(|n| !taken.contains(&n.as_str()))
                .unwrap_or_else(|| "?".into())
        }
    }

    // ---- Footings -------------------------------------------------------------

    fn validate_footing(&self, f: &NewFooting) -> Result<(), ModelError> {
        if !self.levels.contains_key(&f.level) {
            return Err(ModelError::UnknownLevel(f.level));
        }
        if !self.materials.contains_key(&f.material) {
            return Err(ModelError::UnknownMaterial(f.material));
        }
        finite(f.width, "footing width")?;
        finite(f.depth, "footing depth")?;
        finite(f.top_offset, "footing top offset")?;
        if f.width <= EPSILON {
            return Err(ModelError::NonPositive("footing width"));
        }
        if f.depth <= EPSILON {
            return Err(ModelError::NonPositive("footing depth"));
        }
        match f.kind {
            FootingKind::Pad {
                position,
                length,
                rotation,
                column,
            } => {
                finite_point(position, "footing position")?;
                finite(length, "footing length")?;
                finite(rotation, "footing rotation")?;
                if length <= EPSILON {
                    return Err(ModelError::NonPositive("footing length"));
                }
                if let Some(c) = column {
                    if !self.columns.contains_key(&c) {
                        return Err(ModelError::UnknownColumn(c));
                    }
                }
            }
            FootingKind::Strip { start, end, wall } => {
                finite_point(start, "footing start")?;
                finite_point(end, "footing end")?;
                if start.approx_eq(end) {
                    return Err(ModelError::InvalidShape(
                        "a footing needs two different points",
                    ));
                }
                if let Some(w) = wall {
                    if !self.walls.contains_key(&w) {
                        return Err(ModelError::UnknownWall(w));
                    }
                }
            }
            FootingKind::TieBeam { start, end } => {
                finite_point(start, "tie beam start")?;
                finite_point(end, "tie beam end")?;
                if start.approx_eq(end) {
                    return Err(ModelError::InvalidShape(
                        "a tie beam needs two different points",
                    ));
                }
            }
        }
        Ok(())
    }

    pub fn add_footing(&mut self, spec: NewFooting) -> Result<FootingId, ModelError> {
        self.validate_footing(&spec)?;
        let id = FootingId(self.take_id());
        self.footings.insert(
            id,
            Footing {
                id,
                guid: GlobalId::new(),
                level: spec.level,
                kind: spec.kind,
                material: spec.material,
                width: spec.width,
                depth: spec.depth,
                top_offset: spec.top_offset,
            },
        );
        self.stamp_phase(id.0);
        self.footings_follow_hosts();
        Ok(id)
    }

    pub fn edit_footing(
        &mut self,
        id: FootingId,
        edit: impl FnOnce(&mut Footing),
    ) -> Result<(), ModelError> {
        let mut f = self
            .footings
            .get(&id)
            .cloned()
            .ok_or(ModelError::UnknownElement(id.0))?;
        edit(&mut f);
        self.validate_footing(&NewFooting {
            level: f.level,
            kind: f.kind,
            material: f.material,
            width: f.width,
            depth: f.depth,
            top_offset: f.top_offset,
        })?;
        self.footings.insert(id, f);
        Ok(())
    }

    pub fn remove_footing(&mut self, id: FootingId) -> Result<Footing, ModelError> {
        self.footings
            .remove(&id)
            .ok_or(ModelError::UnknownElement(id.0))
    }

    pub fn footing(&self, id: FootingId) -> Option<&Footing> {
        self.footings.get(&id)
    }

    pub fn footings(&self) -> impl Iterator<Item = &Footing> {
        self.footings.values()
    }

    pub fn footings_on_level(&self, level: LevelId) -> impl Iterator<Item = &Footing> {
        self.footings.values().filter(move |f| f.level == level)
    }

    /// The centre line of a wall: its drawn line moved to the middle of
    /// its thickness (the chord, for a curved wall).
    pub fn wall_centre_line(&self, id: WallId) -> Option<(Point2, Point2)> {
        let w = self.walls.get(&id)?;
        let t = self.wall_type(w.wall_type)?.thickness();
        let u = (w.end - w.start).normalized()?;
        // The exterior face is on the left of the drawn line.
        let shift = match w.reference {
            ReferenceLine::Center => 0.0,
            ReferenceLine::ExteriorFace => -t / 2.0,
            ReferenceLine::InteriorFace => t / 2.0,
        };
        let n = u.perp_left() * shift;
        Some((w.start + n, w.end + n))
    }

    /// Footings placed under a column or a wall take their place again;
    /// one whose host is gone keeps where it was, on its own. Called after
    /// every change (by the editor) and when a footing is added.
    pub fn footings_follow_hosts(&mut self) {
        let ids: Vec<FootingId> = self.footings.keys().copied().collect();
        for id in ids {
            let Some(f) = self.footings.get(&id).cloned() else {
                continue;
            };
            let kind = match f.kind {
                FootingKind::Pad {
                    length,
                    column: Some(c),
                    position,
                    rotation,
                } => match self.columns.get(&c) {
                    Some(col) => FootingKind::Pad {
                        position: {
                            let o = self
                                .footing_offsets
                                .get(&id.0)
                                .copied()
                                .unwrap_or(Vec2::new(0.0, 0.0));
                            let (cs, sn) = (col.rotation.cos(), col.rotation.sin());
                            col.position + Vec2::new(o.x * cs - o.y * sn, o.x * sn + o.y * cs)
                        },
                        length,
                        rotation: col.rotation,
                        column: Some(c),
                    },
                    None => FootingKind::Pad {
                        position,
                        length,
                        rotation,
                        column: None,
                    },
                },
                FootingKind::Strip {
                    wall: Some(w),
                    start,
                    end,
                } => match self.wall_centre_line(w) {
                    Some((a, b)) => FootingKind::Strip {
                        start: a,
                        end: b,
                        wall: Some(w),
                    },
                    None => FootingKind::Strip {
                        start,
                        end,
                        wall: None,
                    },
                },
                k => k,
            };
            if kind != f.kind {
                if let Some(f) = self.footings.get_mut(&id) {
                    f.kind = kind;
                }
            }
        }
    }

    pub(super) fn restore_grids_and_footings(
        &mut self,
        grids: Vec<GridLine>,
        footings: Vec<Footing>,
        claim: &mut structure::Claim<'_>,
    ) -> Result<(), RestoreError> {
        for g in grids {
            let spec = NewGrid {
                name: g.name.clone(),
                start: g.start,
                end: g.end,
            };
            claim(g.id.0, Some(&g.guid))
                .and_then(|_| self.validate_grid(&spec))
                .map_err(|error| RestoreError {
                    element: format!("grid line {} «{}»", g.id.0, g.name),
                    error,
                })?;
            self.grids.insert(g.id, g);
        }
        for f in footings {
            let spec = NewFooting {
                level: f.level,
                kind: f.kind,
                material: f.material,
                width: f.width,
                depth: f.depth,
                top_offset: f.top_offset,
            };
            claim(f.id.0, Some(&f.guid))
                .and_then(|_| self.validate_footing(&spec))
                .map_err(|error| RestoreError {
                    element: format!("footing {}", f.id.0),
                    error,
                })?;
            self.footings.insert(f.id, f);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn grids_are_named_and_footings_follow_their_hosts() {
        let mut m = Model::new("t");
        let l = m.add_level("PB", 0.0);
        assert_eq!(
            m.next_grid_name(Point2::new(0.0, 0.0), Point2::new(0.0, 10.0)),
            "1"
        );
        assert_eq!(
            m.next_grid_name(Point2::new(0.0, 0.0), Point2::new(10.0, 0.0)),
            "A"
        );
        let g = m
            .add_grid(NewGrid {
                name: "1".into(),
                start: Point2::new(0.0, -1.0),
                end: Point2::new(0.0, 9.0),
            })
            .unwrap();
        assert_eq!(
            m.next_grid_name(Point2::new(5.0, 0.0), Point2::new(5.0, 10.0)),
            "2"
        );
        assert!(m.grid(g).is_some());

        let concrete = m.add_material("HA-25", "concrete", [190, 190, 190], CutPattern::Cross);
        let t = m
            .add_member_type(
                "Pilar 30×30",
                MemberUsage::Column,
                concrete,
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
                position: Point2::new(2.0, 3.0),
                rotation: 0.0,
                base_offset: 0.0,
                top: WallTop::Unconnected,
                height: 3.0,
            })
            .unwrap();
        let f = m
            .add_footing(NewFooting {
                level: l,
                kind: FootingKind::Pad {
                    position: Point2::new(0.0, 0.0),
                    length: 1.4,
                    rotation: 0.0,
                    column: Some(c),
                },
                material: concrete,
                width: 1.4,
                depth: 0.6,
                top_offset: -0.4,
            })
            .unwrap();
        // Under the column at once.
        let centre = |m: &Model| match m.footing(f).unwrap().kind {
            FootingKind::Pad { position, .. } => position,
            _ => unreachable!(),
        };
        assert_eq!(centre(&m), Point2::new(2.0, 3.0));
        assert!((m.footing(f).unwrap().volume() - 1.4 * 1.4 * 0.6).abs() < 1e-9);
        // The column moves: the footing follows.
        m.edit_column(c, |c| c.position = Point2::new(4.0, 3.0))
            .unwrap();
        m.footings_follow_hosts();
        assert_eq!(centre(&m), Point2::new(4.0, 3.0));
        // The column goes: the footing stays where it was, on its own.
        m.remove_column(c).unwrap();
        m.footings_follow_hosts();
        assert!(matches!(
            m.footing(f).unwrap().kind,
            FootingKind::Pad { column: None, .. }
        ));
        let back = Model::from_parts(m.to_parts()).unwrap();
        assert_eq!(back.footing(f), m.footing(f));
        assert_eq!(back.grid(g), m.grid(g));
    }
}

/// The piles under a pad footing, which is then their cap (encepado).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Piles {
    pub count: u32,
    pub diameter: f64,
    /// Below the cap, m.
    pub length: f64,
    /// What one pile may take in service, kN, from the geotechnical
    /// report (0: its structural top only, `Piles::structural_top`).
    pub capacity: f64,
}

impl Piles {
    /// Concrete piles cast in place of `diameter`, 12 m long.
    pub fn new(count: u32, diameter: f64) -> Self {
        Piles {
            count,
            diameter,
            length: 12.0,
            capacity: 0.0,
        }
    }

    pub fn is_valid(&self) -> bool {
        (1..=16).contains(&self.count)
            && (0.25..=2.0).contains(&self.diameter)
            && self.length > 1.0
            && self.capacity >= 0.0
    }

    /// Between pile axes: three diameters.
    pub fn spacing(&self) -> f64 {
        3.0 * self.diameter
    }

    /// The structural top of a pile cast in place: 4 N/mm² over its
    /// section (DB-SE-C, table 5.1, the usual piles here), kN.
    pub fn structural_top(&self) -> f64 {
        4000.0 * std::f64::consts::PI * self.diameter * self.diameter / 4.0
    }

    /// What one pile may take: the report's or its structural top.
    pub fn allowed(&self) -> f64 {
        if self.capacity > 0.0 {
            self.capacity.min(self.structural_top())
        } else {
            self.structural_top()
        }
    }

    /// Where the piles are, in the cap's axes (x across its width, y
    /// along its length): in a line along the longer side for two, a
    /// triangle for three, a square for four, rows beyond.
    pub fn layout(&self, width: f64, length: f64) -> Vec<(f64, f64)> {
        let s = self.spacing();
        let along_y = length >= width;
        let place = |a: f64, b: f64| if along_y { (b, a) } else { (a, b) };
        match self.count {
            0 | 1 => vec![(0.0, 0.0)],
            2 => vec![place(-s / 2.0, 0.0), place(s / 2.0, 0.0)],
            3 => {
                let h = s * 3f64.sqrt() / 2.0;
                vec![
                    place(-s / 2.0, -h / 3.0),
                    place(s / 2.0, -h / 3.0),
                    place(0.0, 2.0 * h / 3.0),
                ]
            }
            n => {
                let cols = (f64::from(n)).sqrt().ceil() as u32;
                let rows = n.div_ceil(cols);
                let mut out = Vec::new();
                for r in 0..rows {
                    for c in 0..cols {
                        if out.len() as u32 == n {
                            break;
                        }
                        let a = (f64::from(c) - f64::from(cols - 1) / 2.0) * s;
                        let b = (f64::from(r) - f64::from(rows - 1) / 2.0) * s;
                        out.push(place(a, b));
                    }
                }
                out
            }
        }
    }

    /// The smallest cap for them: the piles' extent plus a diameter and
    /// 15 cm round, (width, length), m.
    pub fn cap_size(&self) -> (f64, f64) {
        let pts = self.layout(1.0, 2.0);
        let (mut x, mut y) = (0.0f64, 0.0f64);
        for (a, b) in &pts {
            x = x.max(a.abs());
            y = y.max(b.abs());
        }
        let m = self.diameter + 0.30;
        (2.0 * x + m, 2.0 * y + m)
    }
}

impl Model {
    /// How far a pad footing's centre is from its column, in the column's
    /// axes (a footing against a party wall or at a corner).
    pub fn footing_offset(&self, id: FootingId) -> Option<Vec2> {
        self.footing_offsets
            .get(&id.0)
            .copied()
            .filter(|_| self.footing(id).is_some())
    }

    /// The pad footings off their column.
    pub fn footing_offsets(&self) -> impl Iterator<Item = (FootingId, Vec2)> + '_ {
        self.footing_offsets
            .iter()
            .filter(|(id, _)| self.footing(FootingId(**id)).is_some())
            .map(|(id, v)| (FootingId(*id), *v))
    }

    /// Moves a pad footing off its column (or centres it again, `None`),
    /// and puts it there.
    pub fn set_footing_offset(&mut self, id: FootingId, v: Option<Vec2>) -> Result<(), ModelError> {
        let Some(f) = self.footing(id) else {
            return Err(ModelError::UnknownElement(id.0));
        };
        if !matches!(f.kind, FootingKind::Pad { .. }) {
            return Err(ModelError::UnknownElement(id.0));
        }
        match v {
            Some(v) if v.x.hypot(v.y) > 1e-6 => {
                if !v.x.is_finite() || !v.y.is_finite() || v.x.hypot(v.y) > 10.0 {
                    return Err(ModelError::NonPositive("footing offset"));
                }
                self.footing_offsets.insert(id.0, v);
            }
            _ => {
                self.footing_offsets.remove(&id.0);
            }
        }
        self.footings_follow_hosts();
        Ok(())
    }

    /// The piles under a pad footing.
    pub fn footing_piles(&self, id: FootingId) -> Option<&Piles> {
        self.footing_piles
            .get(&id.0)
            .filter(|_| self.footing(id).is_some())
    }

    /// The pile caps.
    pub fn all_footing_piles(&self) -> impl Iterator<Item = (FootingId, &Piles)> {
        self.footing_piles
            .iter()
            .filter(|(id, _)| self.footing(FootingId(**id)).is_some())
            .map(|(id, p)| (FootingId(*id), p))
    }

    /// Puts piles under a pad footing (or takes them away, `None`).
    pub fn set_footing_piles(&mut self, id: FootingId, p: Option<Piles>) -> Result<(), ModelError> {
        let Some(f) = self.footing(id) else {
            return Err(ModelError::UnknownElement(id.0));
        };
        if !matches!(f.kind, FootingKind::Pad { .. }) {
            return Err(ModelError::UnknownElement(id.0));
        }
        match p {
            Some(p) => {
                if !p.is_valid() {
                    return Err(ModelError::NonPositive("piles"));
                }
                self.footing_piles.insert(id.0, p);
            }
            None => {
                self.footing_piles.remove(&id.0);
            }
        }
        Ok(())
    }
}

impl Model {
    /// Where the piles of a pile cap are, in plan.
    pub fn pile_points(&self, id: FootingId) -> Vec<Point2> {
        let (Some(f), Some(p)) = (self.footing(id), self.footing_piles(id)) else {
            return Vec::new();
        };
        let FootingKind::Pad {
            position,
            length,
            rotation,
            ..
        } = f.kind
        else {
            return Vec::new();
        };
        let (c, s) = (rotation.cos(), rotation.sin());
        p.layout(f.width, length)
            .into_iter()
            .map(|(x, y)| Point2::new(position.x + x * c - y * s, position.y + x * s + y * c))
            .collect()
    }
}
