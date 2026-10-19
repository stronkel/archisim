//! Curtain walls ("muros cortina"): a grid of mullions and transoms with
//! panels between them, along a straight line.
//!
//! What a curtain wall looks like is its **type** ([`CurtainWallType`]),
//! as Revit's curtain wall types or ArchiCAD's saved curtain wall
//! settings: how its length and height are divided ([`CurtainDivision`]),
//! the profiles of its border, mullions and transoms, which of them run
//! through at the joints, its panel kinds and which row takes which,
//! pressure caps and sun louvres outside. The wall itself is only a line,
//! a foot and a top, as walls.
//!
//! The grid's lines are the axes of the interior mullions and transoms;
//! the border sits inside the wall's ends, its base and its top. Panels
//! fill each cell between the frames, centred in the depth; the outside is
//! to the left of the line (positive `across`), as for walls.
//!
//! IFC: `IfcCurtainWall` (typed by an `IfcCurtainWallType`) made of
//! `IfcMember` (`MULLION`: frames and caps), `IfcPlate` (`CURTAIN_PANEL`)
//! and `IfcShadingDevice` (`LOUVER`).

use super::*;

/// Bays left smaller than this after the fixed ones are added to the last
/// fixed bay.
const MIN_REST: f64 = 0.05;
/// Fewest clear millimetres between two frames.
const MIN_CLEAR: f64 = 0.01;
/// Most grid lines in either direction.
const MAX_LINES: usize = 400;

/// How what is left after the fixed bays is divided.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CurtainRest {
    /// Equal bays no larger than this.
    MaxSpacing(f64),
    /// This many equal bays.
    Count(usize),
}

/// How the length (or the height) of a curtain wall is divided: first the
/// fixed bays, from its start (or its base), then the rest.
#[derive(Debug, Clone, PartialEq)]
pub struct CurtainDivision {
    pub fixed: Vec<f64>,
    pub rest: CurtainRest,
}

impl CurtainDivision {
    /// Equal bays no larger than `spacing`.
    pub fn max_spacing(spacing: f64) -> Self {
        Self {
            fixed: Vec::new(),
            rest: CurtainRest::MaxSpacing(spacing),
        }
    }

    /// The grid lines over `total`, from 0 to `total`. Fixed bays that do
    /// not fit are left out; a rest under 5 cm goes to the last fixed bay.
    pub fn lines(&self, total: f64) -> Vec<f64> {
        let mut out = vec![0.0];
        let mut at = 0.0;
        for &s in &self.fixed {
            if at + s > total - MIN_REST {
                break;
            }
            at += s;
            out.push(at);
        }
        let rest = total - at;
        if rest > MIN_REST || out.len() == 1 {
            let n = match self.rest {
                CurtainRest::MaxSpacing(m) => (rest / m - 1e-9).ceil().max(1.0) as usize,
                CurtainRest::Count(n) => n.max(1),
            }
            .min(MAX_LINES);
            for i in 1..n {
                out.push(at + rest * i as f64 / n as f64);
            }
        } else {
            out.pop();
        }
        out.push(total);
        out
    }

    fn validate(&self, what: &'static str) -> Result<(), ModelError> {
        for &s in &self.fixed {
            finite(s, what)?;
            if s < 0.1 {
                return Err(ModelError::InvalidCurtainType);
            }
        }
        match self.rest {
            CurtainRest::MaxSpacing(m) => {
                finite(m, what)?;
                if m < 0.2 {
                    return Err(ModelError::InvalidCurtainType);
                }
            }
            CurtainRest::Count(n) => {
                if n == 0 || n > MAX_LINES {
                    return Err(ModelError::InvalidCurtainType);
                }
            }
        }
        if self.fixed.len() > MAX_LINES {
            return Err(ModelError::InvalidCurtainType);
        }
        Ok(())
    }
}

/// The section of a frame member: its width seen from outside, its depth
/// across the wall (centred on it), and its material.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CurtainProfile {
    pub width: f64,
    pub depth: f64,
    pub material: MaterialId,
}

/// Which members run through where mullions and transoms meet.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CurtainJoint {
    /// Mullions (and the border's sides) run full height; transoms stop
    /// at them. The usual stick system.
    #[default]
    Mullions,
    /// Transoms (and the border's base and top) run full length; mullions
    /// stop at them, as in a ribbon window.
    Transoms,
}

/// A kind of panel: glass, an opaque spandrel, a louvred vent…
#[derive(Debug, Clone, PartialEq)]
pub struct CurtainPanel {
    pub name: String,
    pub thickness: f64,
    pub material: MaterialId,
}

/// Pressure caps ("tapajuntas"): a cover strip outside every frame.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CurtainCaps {
    pub width: f64,
    pub depth: f64,
    pub material: MaterialId,
}

/// Horizontal sun louvres ("lamas") outside the wall, its whole length.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CurtainLouvres {
    /// How far each blade sticks out.
    pub depth: f64,
    pub thickness: f64,
    /// Between the blades, from the base.
    pub spacing: f64,
    /// Gap between the outside of the frame (or the caps) and the blades.
    pub offset: f64,
    pub material: MaterialId,
}

/// Everything a curtain wall type says about its walls.
#[derive(Debug, Clone, PartialEq)]
pub struct CurtainDesign {
    /// Along the wall: where the mullions go.
    pub columns: CurtainDivision,
    /// Up the wall: where the transoms go.
    pub rows: CurtainDivision,
    /// The frame round the edges.
    pub border: CurtainProfile,
    pub mullion: CurtainProfile,
    pub transom: CurtainProfile,
    pub joint: CurtainJoint,
    /// At least one.
    pub panels: Vec<CurtainPanel>,
    /// The panel of each row, from the base (index into `panels`); rows
    /// above the list take its last one. Empty: all the first.
    pub row_panels: Vec<usize>,
    pub caps: Option<CurtainCaps>,
    pub louvres: Option<CurtainLouvres>,
}

impl CurtainDesign {
    /// The simplest grid: equal bays and rows no larger than `bay` and
    /// `row`, one profile everywhere, one panel.
    pub fn simple(
        bay: f64,
        row: f64,
        frame: CurtainProfile,
        panel_thickness: f64,
        panel_material: MaterialId,
    ) -> Self {
        Self {
            columns: CurtainDivision::max_spacing(bay),
            rows: CurtainDivision::max_spacing(row),
            border: frame,
            mullion: frame,
            transom: frame,
            joint: CurtainJoint::Mullions,
            panels: vec![CurtainPanel {
                name: "Vidrio".into(),
                thickness: panel_thickness,
                material: panel_material,
            }],
            row_panels: Vec::new(),
            caps: None,
            louvres: None,
        }
    }

    /// The panel kind of row `r` (from the base).
    pub fn panel_of_row(&self, r: usize) -> usize {
        self.row_panels
            .get(r)
            .or(self.row_panels.last())
            .copied()
            .unwrap_or(0)
            .min(self.panels.len().saturating_sub(1))
    }

    /// The deepest frame: how thick the wall is in plan.
    pub fn frame_depth(&self) -> f64 {
        self.border
            .depth
            .max(self.mullion.depth)
            .max(self.transom.depth)
    }

    /// How far out from the line the outermost thing goes (frame, caps,
    /// louvres).
    pub fn outer_extent(&self) -> f64 {
        let mut x = self.frame_depth() / 2.0;
        if let Some(c) = self.caps {
            x += c.depth;
        }
        if let Some(l) = self.louvres {
            x += l.offset + l.depth;
        }
        x
    }

    /// Every material it uses.
    pub fn materials(&self) -> Vec<MaterialId> {
        let mut out = vec![
            self.border.material,
            self.mullion.material,
            self.transom.material,
        ];
        out.extend(self.panels.iter().map(|p| p.material));
        out.extend(self.caps.map(|c| c.material));
        out.extend(self.louvres.map(|l| l.material));
        out
    }

    fn validate(&self, materials: &BTreeMap<MaterialId, Material>) -> Result<(), ModelError> {
        for m in self.materials() {
            if !materials.contains_key(&m) {
                return Err(ModelError::UnknownMaterial(m));
            }
        }
        self.columns.validate("curtain wall columns")?;
        self.rows.validate("curtain wall rows")?;
        for p in [self.border, self.mullion, self.transom] {
            finite(p.width, "curtain wall profile width")?;
            finite(p.depth, "curtain wall profile depth")?;
            if p.width <= 0.005 || p.depth <= 0.005 || p.width > 1.0 || p.depth > 2.0 {
                return Err(ModelError::InvalidCurtainType);
            }
        }
        if self.panels.is_empty() || self.panels.len() > 32 {
            return Err(ModelError::InvalidCurtainType);
        }
        for p in &self.panels {
            finite(p.thickness, "curtain wall panel thickness")?;
            if p.thickness <= 0.001 || p.thickness > self.frame_depth() {
                return Err(ModelError::InvalidCurtainType);
            }
        }
        if self.row_panels.iter().any(|&i| i >= self.panels.len()) {
            return Err(ModelError::InvalidCurtainType);
        }
        if let Some(c) = self.caps {
            finite(c.width, "curtain wall cap width")?;
            finite(c.depth, "curtain wall cap depth")?;
            if c.width <= 0.005 || c.depth <= 0.002 || c.width > 1.0 || c.depth > 1.0 {
                return Err(ModelError::InvalidCurtainType);
            }
        }
        if let Some(l) = self.louvres {
            for (v, what) in [
                (l.depth, "louvre depth"),
                (l.thickness, "louvre thickness"),
                (l.spacing, "louvre spacing"),
                (l.offset, "louvre offset"),
            ] {
                finite(v, what)?;
            }
            if l.depth <= 0.01
                || l.thickness <= 0.002
                || l.spacing <= l.thickness + 0.01
                || l.offset < 0.0
                || l.depth > 3.0
                || l.offset > 3.0
            {
                return Err(ModelError::InvalidCurtainType);
            }
        }
        Ok(())
    }
}

/// A curtain wall type. IFC: `IfcCurtainWallType`.
#[derive(Debug, Clone, PartialEq)]
pub struct CurtainWallType {
    pub id: CurtainTypeId,
    pub guid: GlobalId,
    pub name: String,
    pub design: CurtainDesign,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CurtainWall {
    pub id: CurtainWallId,
    pub guid: GlobalId,
    pub curtain_type: CurtainTypeId,
    pub level: LevelId,
    pub start: Point2,
    pub end: Point2,
    pub base_offset: f64,
    /// Up to a level, or by `height` (as walls).
    pub top: WallTop,
    pub height: f64,
}

/// Data needed to create a curtain wall.
#[derive(Debug, Clone, PartialEq)]
pub struct NewCurtainWall {
    pub curtain_type: CurtainTypeId,
    pub level: LevelId,
    pub start: Point2,
    pub end: Point2,
    pub base_offset: f64,
    pub top: WallTop,
    pub height: f64,
}

/// What a piece of a curtain wall is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CurtainPieceKind {
    /// The frame round the edges (IFC `IfcMember` `MULLION`).
    Border,
    /// A vertical frame member (IFC `IfcMember` `MULLION`).
    Mullion,
    /// A horizontal frame member (also `MULLION` in IFC, as Revit
    /// exports them).
    Transom,
    /// A panel between frames, of this kind of the type (IFC `IfcPlate`
    /// `CURTAIN_PANEL`).
    Panel(usize),
    /// A pressure cap outside a frame (IFC `IfcMember` `MULLION`).
    Cap,
    /// A sun louvre (IFC `IfcShadingDevice` `LOUVER`).
    Louvre,
}

impl CurtainPieceKind {
    pub fn is_frame(self) -> bool {
        matches!(self, Self::Border | Self::Mullion | Self::Transom)
    }
}

/// A box of a curtain wall in its own frame: `along` from the start of
/// its line, `across` to the left of it (outside), `up` from its base.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CurtainPiece {
    pub kind: CurtainPieceKind,
    pub along: (f64, f64),
    pub across: (f64, f64),
    pub up: (f64, f64),
    pub material: MaterialId,
}

impl CurtainPiece {
    /// Whether it stands up (its length is its height).
    pub fn is_vertical(&self) -> bool {
        self.up.1 - self.up.0 > self.along.1 - self.along.0
    }
}

/// Where the grid lines of a curtain wall are: along it from its start,
/// and up from its base.
#[derive(Debug, Clone, PartialEq)]
pub struct CurtainGrid {
    pub mullions: Vec<f64>,
    pub transoms: Vec<f64>,
}

/// The extent of the frame at grid line `i`: the border inside the ends,
/// the others centred on their line.
fn band(lines: &[f64], i: usize, border: f64, inner: f64) -> (f64, f64) {
    let x = lines[i];
    if i == 0 {
        (x, x + border)
    } else if i + 1 == lines.len() {
        (x - border, x)
    } else {
        (x - inner / 2.0, x + inner / 2.0)
    }
}

/// The grid and pieces of a curtain wall `length` long and `height` high
/// of design `d`.
pub fn curtain_grid(d: &CurtainDesign, length: f64, height: f64) -> CurtainGrid {
    CurtainGrid {
        mullions: d.columns.lines(length),
        transoms: d.rows.lines(height),
    }
}

/// Every box of a curtain wall `length` long and `height` high of design
/// `d`: frames, panels, caps and louvres.
pub fn curtain_pieces(d: &CurtainDesign, length: f64, height: f64) -> Vec<CurtainPiece> {
    let g = curtain_grid(d, length, height);
    let (ms, ts) = (&g.mullions, &g.transoms);
    let col = |i: usize| band(ms, i, d.border.width, d.mullion.width);
    let row = |j: usize| band(ts, j, d.border.width, d.transom.width);
    let edge_col = |i: usize| i == 0 || i + 1 == ms.len();
    let edge_row = |j: usize| j == 0 || j + 1 == ts.len();
    let half = |p: CurtainProfile| (-p.depth / 2.0, p.depth / 2.0);
    let mut out = Vec::new();
    let mut frame = |kind, profile: CurtainProfile, along, up| {
        out.push(CurtainPiece {
            kind,
            along,
            across: half(profile),
            up,
            material: profile.material,
        })
    };
    let vertical = |i: usize| {
        if edge_col(i) {
            (CurtainPieceKind::Border, d.border)
        } else {
            (CurtainPieceKind::Mullion, d.mullion)
        }
    };
    let horizontal = |j: usize| {
        if edge_row(j) {
            (CurtainPieceKind::Border, d.border)
        } else {
            (CurtainPieceKind::Transom, d.transom)
        }
    };
    match d.joint {
        CurtainJoint::Mullions => {
            for i in 0..ms.len() {
                let (k, p) = vertical(i);
                frame(k, p, col(i), (0.0, height));
            }
            for i in 0..ms.len() - 1 {
                let along = (col(i).1, col(i + 1).0);
                for j in 0..ts.len() {
                    let (k, p) = horizontal(j);
                    frame(k, p, along, row(j));
                }
            }
        }
        CurtainJoint::Transoms => {
            for j in 0..ts.len() {
                let (k, p) = horizontal(j);
                frame(k, p, (0.0, length), row(j));
            }
            for j in 0..ts.len() - 1 {
                let up = (row(j).1, row(j + 1).0);
                for i in 0..ms.len() {
                    let (k, p) = vertical(i);
                    frame(k, p, col(i), up);
                }
            }
        }
    }
    // Panels.
    for j in 0..ts.len() - 1 {
        let kind = d.panel_of_row(j);
        let panel = &d.panels[kind];
        let t = panel.thickness / 2.0;
        for i in 0..ms.len() - 1 {
            out.push(CurtainPiece {
                kind: CurtainPieceKind::Panel(kind),
                along: (col(i).1, col(i + 1).0),
                across: (-t, t),
                up: (row(j).1, row(j + 1).0),
                material: panel.material,
            });
        }
    }
    // Caps, on one plane outside the deepest frame; those of the members
    // that run through are whole, the others stop at them.
    let face = d.frame_depth() / 2.0;
    if let Some(c) = d.caps {
        let across = (face, face + c.depth);
        let centre = |(a, b): (f64, f64)| (a + b) / 2.0;
        let strip = |(a, b): (f64, f64), total: f64| {
            let m = centre((a, b));
            ((m - c.width / 2.0).max(0.0), (m + c.width / 2.0).min(total))
        };
        let cap = |along, up| CurtainPiece {
            kind: CurtainPieceKind::Cap,
            along,
            across,
            up,
            material: c.material,
        };
        match d.joint {
            CurtainJoint::Mullions => {
                for i in 0..ms.len() {
                    out.push(cap(strip(col(i), length), (0.0, height)));
                }
                for i in 0..ms.len() - 1 {
                    let along = (strip(col(i), length).1, strip(col(i + 1), length).0);
                    if along.1 - along.0 > 1e-6 {
                        for j in 0..ts.len() {
                            out.push(cap(along, strip(row(j), height)));
                        }
                    }
                }
            }
            CurtainJoint::Transoms => {
                for j in 0..ts.len() {
                    out.push(cap((0.0, length), strip(row(j), height)));
                }
                for j in 0..ts.len() - 1 {
                    let up = (strip(row(j), height).1, strip(row(j + 1), height).0);
                    if up.1 - up.0 > 1e-6 {
                        for i in 0..ms.len() {
                            out.push(cap(strip(col(i), length), up));
                        }
                    }
                }
            }
        }
    }
    if let Some(l) = d.louvres {
        let x0 = face + d.caps.map_or(0.0, |c| c.depth) + l.offset;
        let mut k = 1;
        loop {
            let z = l.spacing * k as f64;
            if z + l.thickness / 2.0 > height - 1e-6 || k > MAX_LINES {
                break;
            }
            out.push(CurtainPiece {
                kind: CurtainPieceKind::Louvre,
                along: (0.0, length),
                across: (x0, x0 + l.depth),
                up: (z - l.thickness / 2.0, z + l.thickness / 2.0),
                material: l.material,
            });
            k += 1;
        }
    }
    out
}

/// Whether a design fits a wall `length` long and `height` high: every
/// cell has room between its frames.
pub fn curtain_fits(d: &CurtainDesign, length: f64, height: f64) -> bool {
    let g = curtain_grid(d, length, height);
    let (ms, ts) = (&g.mullions, &g.transoms);
    if ms.len() > MAX_LINES + 1 || ts.len() > MAX_LINES + 1 {
        return false;
    }
    let clear = |lines: &[f64], inner: f64| {
        (0..lines.len() - 1).all(|i| {
            band(lines, i + 1, d.border.width, inner).0 - band(lines, i, d.border.width, inner).1
                > MIN_CLEAR
        })
    };
    clear(ms, d.mullion.width) && clear(ts, d.transom.width)
}

impl CurtainWall {
    pub fn length(&self) -> f64 {
        self.start.distance(self.end)
    }

    /// Height of its base above project zero, given its level's.
    pub fn base_z(&self, level_elevation: f64) -> f64 {
        level_elevation + self.base_offset
    }

    pub fn grid(&self, d: &CurtainDesign) -> CurtainGrid {
        curtain_grid(d, self.length(), self.height)
    }

    pub fn pieces(&self, d: &CurtainDesign) -> Vec<CurtainPiece> {
        curtain_pieces(d, self.length(), self.height)
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
    pub fn outline(&self, d: &CurtainDesign) -> Vec<Point2> {
        let u = (self.end - self.start)
            .normalized()
            .unwrap_or(Vec2::new(1.0, 0.0));
        let n = u.perp_left() * (d.frame_depth() / 2.0);
        vec![self.start - n, self.end - n, self.end + n, self.start + n]
    }
}

impl Model {
    // ---- Curtain wall types ---------------------------------------------

    pub fn add_curtain_type(
        &mut self,
        name: impl Into<String>,
        design: CurtainDesign,
    ) -> Result<CurtainTypeId, ModelError> {
        design.validate(&self.materials)?;
        let id = CurtainTypeId(self.take_id());
        self.curtain_types.insert(
            id,
            CurtainWallType {
                id,
                guid: GlobalId::new(),
                name: name.into(),
                design,
            },
        );
        Ok(id)
    }

    pub fn curtain_type(&self, id: CurtainTypeId) -> Option<&CurtainWallType> {
        self.curtain_types.get(&id)
    }

    pub fn curtain_types(&self) -> impl Iterator<Item = &CurtainWallType> {
        self.curtain_types.values()
    }

    /// The design of a curtain wall's type.
    pub fn curtain_design(&self, id: CurtainWallId) -> Option<&CurtainDesign> {
        let c = self.curtain_walls.get(&id)?;
        self.curtain_types.get(&c.curtain_type).map(|t| &t.design)
    }

    /// The pieces of a curtain wall.
    pub fn curtain_wall_pieces(&self, id: CurtainWallId) -> Vec<CurtainPiece> {
        match (self.curtain_walls.get(&id), self.curtain_design(id)) {
            (Some(c), Some(d)) => c.pieces(d),
            _ => Vec::new(),
        }
    }

    /// Changes a type's name and design; every curtain wall of the type
    /// follows. Nothing changes if one of them would not fit it.
    pub fn edit_curtain_type(
        &mut self,
        id: CurtainTypeId,
        name: impl Into<String>,
        design: CurtainDesign,
    ) -> Result<(), ModelError> {
        if !self.curtain_types.contains_key(&id) {
            return Err(ModelError::UnknownCurtainType(id));
        }
        design.validate(&self.materials)?;
        let misfit = self
            .curtain_walls
            .values()
            .any(|c| c.curtain_type == id && !curtain_fits(&design, c.length(), c.height));
        if misfit {
            return Err(ModelError::InvalidCurtainWall);
        }
        let t = self.curtain_types.get_mut(&id).expect("checked");
        t.name = name.into();
        t.design = design;
        Ok(())
    }

    /// Copies a type under a new name and a new GlobalId.
    pub fn duplicate_curtain_type(
        &mut self,
        id: CurtainTypeId,
        name: impl Into<String>,
    ) -> Result<CurtainTypeId, ModelError> {
        let t = self
            .curtain_types
            .get(&id)
            .cloned()
            .ok_or(ModelError::UnknownCurtainType(id))?;
        self.add_curtain_type(name, t.design)
    }

    /// How many curtain walls use a type.
    pub fn curtain_type_users(&self, id: CurtainTypeId) -> usize {
        self.curtain_walls
            .values()
            .filter(|c| c.curtain_type == id)
            .count()
    }

    pub fn remove_curtain_type(
        &mut self,
        id: CurtainTypeId,
    ) -> Result<CurtainWallType, ModelError> {
        let users = self.curtain_type_users(id);
        if users > 0 {
            return Err(ModelError::InUse(format!(
                "{users} curtain wall(s) use this type"
            )));
        }
        self.curtain_types
            .remove(&id)
            .ok_or(ModelError::UnknownCurtainType(id))
    }

    // ---- Curtain walls --------------------------------------------------

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
        let t = self
            .curtain_types
            .get(&c.curtain_type)
            .ok_or(ModelError::UnknownCurtainType(c.curtain_type))?;
        finite_point(c.start, "curtain wall start")?;
        finite_point(c.end, "curtain wall end")?;
        finite(c.base_offset, "curtain wall base offset")?;
        finite(c.height, "curtain wall height")?;
        if c.start.approx_eq(c.end) {
            return Err(ModelError::ZeroLengthWall);
        }
        let height = self.curtain_height(c.level, c.base_offset, c.top, c.height)?;
        if !(height > 0.05 && curtain_fits(&t.design, c.start.distance(c.end), height)) {
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
                curtain_type: spec.curtain_type,
                level: spec.level,
                start: spec.start,
                end: spec.end,
                base_offset: spec.base_offset,
                top: spec.top,
                height,
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
                let fits = self
                    .curtain_types
                    .get(&c.curtain_type)
                    .is_some_and(|t| curtain_fits(&t.design, c.length(), h));
                if h > 0.05 && fits {
                    if let Some(x) = self.curtain_walls.get_mut(&id) {
                        x.height = h;
                    }
                }
            }
        }
    }

    pub(super) fn restore_curtain_walls(
        &mut self,
        types: Vec<CurtainWallType>,
        list: Vec<CurtainWall>,
        claim: &mut structure::Claim<'_>,
    ) -> Result<(), RestoreError> {
        for t in types {
            let name = format!("curtain wall type {} «{}»", t.id.0, t.name);
            claim(t.id.0, Some(&t.guid))
                .and_then(|_| t.design.validate(&self.materials))
                .map_err(|error| RestoreError {
                    element: name,
                    error,
                })?;
            self.curtain_types.insert(t.id, t);
        }
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
        curtain_type: c.curtain_type,
        level: c.level,
        start: c.start,
        end: c.end,
        base_offset: c.base_offset,
        top: c.top,
        height: c.height,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn materials(m: &mut Model) -> (MaterialId, MaterialId, MaterialId) {
        let al = m.add_material("Aluminio", "aluminium", [120, 120, 125], CutPattern::Solid);
        let glass = m.add_material("Vidrio", "glass", [190, 220, 240], CutPattern::Solid);
        let panel = m.add_material("Panel", "metal", [90, 90, 95], CutPattern::Solid);
        (al, glass, panel)
    }

    fn profile(width: f64, depth: f64, material: MaterialId) -> CurtainProfile {
        CurtainProfile {
            width,
            depth,
            material,
        }
    }

    #[test]
    fn grid_of_a_curtain_wall() {
        let mut m = Model::new("t");
        let pb = m.add_level("PB", 0.0);
        let p1 = m.add_level("P1", 3.0);
        let (al, glass, _) = materials(&mut m);
        let t = m
            .add_curtain_type(
                "Sencillo",
                CurtainDesign::simple(1.5, 1.2, profile(0.05, 0.15, al), 0.024, glass),
            )
            .unwrap();
        let id = m
            .add_curtain_wall(NewCurtainWall {
                curtain_type: t,
                level: pb,
                start: Point2::new(0.0, 0.0),
                end: Point2::new(5.0, 0.0),
                base_offset: 0.0,
                top: WallTop::Level {
                    level: p1,
                    offset: 0.0,
                },
                height: 0.0,
            })
            .unwrap();
        let c = m.curtain_wall(id).unwrap();
        let d = m.curtain_design(id).unwrap();
        let g = c.grid(d);
        // 5 m in bays of 1,25 (4); 3 m in rows of 1,00 (3).
        assert_eq!(g.mullions.len(), 5);
        assert!((g.mullions[1] - 1.25).abs() < 1e-12);
        assert_eq!(g.transoms.len(), 4);
        let pieces = c.pieces(d);
        let count =
            |f: &dyn Fn(CurtainPieceKind) -> bool| pieces.iter().filter(|p| f(p.kind)).count();
        // Two border sides and three mullions, full height; between them
        // the border's base and top and two transoms, in each of four bays.
        assert_eq!(count(&|k| k == CurtainPieceKind::Mullion), 3);
        assert_eq!(count(&|k| k == CurtainPieceKind::Border), 2 + 4 * 2);
        assert_eq!(count(&|k| k == CurtainPieceKind::Transom), 4 * 2);
        assert_eq!(count(&|k| matches!(k, CurtainPieceKind::Panel(_))), 12);
        // The first panel sits between the first two mullions.
        let panel = pieces
            .iter()
            .find(|p| matches!(p.kind, CurtainPieceKind::Panel(_)))
            .unwrap();
        assert!((panel.along.0 - 0.05).abs() < 1e-12);
        assert!((panel.along.1 - 1.225).abs() < 1e-12);
        assert!((panel.up.0 - 0.05).abs() < 1e-12);
        assert_eq!(panel.material, glass);
        m.set_level_elevation(p1, 3.3).unwrap();
        assert!((m.curtain_wall(id).unwrap().height - 3.3).abs() < 1e-12);
        // A type whose frames leave no room is refused while the wall uses
        // it.
        let mut fat = m.curtain_type(t).unwrap().design.clone();
        fat.columns.rest = CurtainRest::Count(100);
        assert_eq!(
            m.edit_curtain_type(t, "Sencillo", fat),
            Err(ModelError::InvalidCurtainWall)
        );
        assert_eq!(
            m.remove_curtain_type(t).map(|_| ()),
            Err(ModelError::InUse("1 curtain wall(s) use this type".into()))
        );
        let back = Model::from_parts(m.to_parts()).unwrap();
        assert_eq!(back, m);
    }

    #[test]
    fn fixed_bays_rows_of_panels_caps_and_louvres() {
        // Fixed bays first, then equal ones no larger than the spacing.
        let div = CurtainDivision {
            fixed: vec![1.0, 0.6],
            rest: CurtainRest::MaxSpacing(1.5),
        };
        let lines = div.lines(5.2);
        let want = [0.0, 1.0, 1.6, 2.8, 4.0, 5.2];
        assert_eq!(lines.len(), want.len());
        assert!(lines.iter().zip(want).all(|(a, b)| (a - b).abs() < 1e-12));
        // A rest under 5 cm goes to the last fixed bay; those that do not
        // fit are left out.
        assert_eq!(div.lines(1.62), vec![0.0, 1.0, 1.62]);
        assert_eq!(div.lines(0.8), vec![0.0, 0.8]);
        let count = CurtainDivision {
            fixed: Vec::new(),
            rest: CurtainRest::Count(3),
        };
        assert_eq!(count.lines(3.0), vec![0.0, 1.0, 2.0, 3.0]);

        let mut m = Model::new("t");
        let pb = m.add_level("PB", 0.0);
        let (al, glass, opaque) = materials(&mut m);
        let mut d = CurtainDesign::simple(1.2, 1.5, profile(0.06, 0.15, al), 0.024, glass);
        d.mullion = profile(0.05, 0.15, al);
        d.transom = profile(0.05, 0.10, al);
        d.rows = CurtainDivision {
            fixed: vec![0.9],
            rest: CurtainRest::MaxSpacing(2.1),
        };
        d.panels.push(CurtainPanel {
            name: "Antepecho".into(),
            thickness: 0.05,
            material: opaque,
        });
        // The first row is the opaque spandrel; the rest glass.
        d.row_panels = vec![1, 0];
        d.joint = CurtainJoint::Transoms;
        d.caps = Some(CurtainCaps {
            width: 0.06,
            depth: 0.02,
            material: al,
        });
        d.louvres = Some(CurtainLouvres {
            depth: 0.25,
            thickness: 0.03,
            spacing: 0.4,
            offset: 0.1,
            material: al,
        });
        let t = m.add_curtain_type("Fachada", d.clone()).unwrap();
        let id = m
            .add_curtain_wall(NewCurtainWall {
                curtain_type: t,
                level: pb,
                start: Point2::new(0.0, 0.0),
                end: Point2::new(3.6, 0.0),
                base_offset: 0.0,
                top: WallTop::Unconnected,
                height: 3.0,
            })
            .unwrap();
        let pieces = m.curtain_wall_pieces(id);
        let panels: Vec<_> = pieces
            .iter()
            .filter(|p| matches!(p.kind, CurtainPieceKind::Panel(_)))
            .collect();
        // Three bays of 1,20; rows of 0,90 and 2,10.
        assert_eq!(panels.len(), 6);
        let low: Vec<_> = panels.iter().filter(|p| p.up.0 < 0.5).collect();
        assert_eq!(low.len(), 3);
        assert!(low.iter().all(|p| p.kind == CurtainPieceKind::Panel(1)));
        assert!(low.iter().all(|p| p.material == opaque));
        assert!((low[0].across.1 - 0.025).abs() < 1e-12);
        // Transoms run through: the border's base and top and the transom
        // at 0,90 are whole; the mullions stop at them.
        let whole: Vec<_> = pieces
            .iter()
            .filter(|p| p.kind.is_frame() && (p.along.1 - p.along.0 - 3.6).abs() < 1e-9)
            .collect();
        assert_eq!(whole.len(), 3);
        let transom = whole
            .iter()
            .find(|p| p.kind == CurtainPieceKind::Transom)
            .unwrap();
        assert!((transom.up.0 - 0.875).abs() < 1e-12);
        assert!((transom.across.1 - 0.05).abs() < 1e-12);
        // Caps outside the deepest frame; louvres beyond them.
        let caps: Vec<_> = pieces
            .iter()
            .filter(|p| p.kind == CurtainPieceKind::Cap)
            .collect();
        assert!(caps.iter().all(|p| (p.across.0 - 0.075).abs() < 1e-12));
        assert_eq!(caps.len(), 3 + 4 * 2);
        let louvres: Vec<_> = pieces
            .iter()
            .filter(|p| p.kind == CurtainPieceKind::Louvre)
            .collect();
        assert_eq!(louvres.len(), 7);
        assert!((louvres[0].across.0 - 0.195).abs() < 1e-12);
        assert!((d.outer_extent() - 0.445).abs() < 1e-12);
        // No piece overlaps another's room in the frame (caps with caps).
        for (a, p) in caps.iter().enumerate() {
            for q in &caps[a + 1..] {
                let o = (p.along.1.min(q.along.1) - p.along.0.max(q.along.0)).max(0.0)
                    * (p.up.1.min(q.up.1) - p.up.0.max(q.up.0)).max(0.0);
                assert!(o < 1e-12, "{p:?} {q:?}");
            }
        }
        // The type survives being written and read back.
        let back = Model::from_parts(m.to_parts()).unwrap();
        assert_eq!(back, m);
        // A row list pointing at no panel is refused.
        let mut bad = d.clone();
        bad.row_panels = vec![2];
        assert_eq!(
            m.add_curtain_type("Mal", bad),
            Err(ModelError::InvalidCurtainType)
        );
        let copy = m.duplicate_curtain_type(t, "Copia").unwrap();
        assert_eq!(m.curtain_type(copy).unwrap().design, d);
        m.edit_curtain_wall(id, |c| c.curtain_type = copy).unwrap();
        assert_eq!(m.curtain_type_users(t), 0);
        m.remove_curtain_type(t).unwrap();
    }
}
