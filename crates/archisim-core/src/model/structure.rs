//! Structural members: columns and beams, with their types.
//!
//! A member type (Revit's column or framing type, ArchiCAD's column or beam
//! settings saved as a favourite) fixes the section and the material; the
//! members place it. IFC: `IfcColumnType` / `IfcBeamType` with an
//! `IfcMaterialProfileSet`, `IfcColumn` / `IfcBeam`.
//!
//! As decided in `docs/DISCIPLINAS.md`, there is one model: a column is an
//! element like any other, marked as load-bearing.

use super::*;

/// The cross-section of a column or beam, in metres.
///
/// Local axes of the section: `x` across (the width), `y` the other way
/// (the depth). For a column the section lies in plan and turns with the
/// column's rotation; for a beam `x` is horizontal across the beam and `y`
/// is vertical, the depth hanging down from the top face.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Profile {
    /// Rectangular section (concrete, timber). IFC: `IfcRectangleProfileDef`.
    Rectangle { width: f64, depth: f64 },
    /// Round section. IFC: `IfcCircleProfileDef`.
    Circle { diameter: f64 },
    /// Steel I or H section (IPE, HEB…), without root radii. IFC:
    /// `IfcIShapeProfileDef`.
    IShape {
        width: f64,
        depth: f64,
        web: f64,
        flange: f64,
    },
    /// Steel channel (UPN), web on the -X side and flanges towards +X.
    /// IFC: `IfcUShapeProfileDef`.
    UShape {
        width: f64,
        depth: f64,
        web: f64,
        flange: f64,
    },
    /// Steel T section, flange on top (+Y) and web down. IFC:
    /// `IfcTShapeProfileDef`.
    TShape {
        width: f64,
        depth: f64,
        web: f64,
        flange: f64,
    },
    /// Steel angle with equal thickness, legs along -X and -Y. IFC:
    /// `IfcLShapeProfileDef`.
    LShape {
        width: f64,
        depth: f64,
        thickness: f64,
    },
}

impl Profile {
    /// Overall width and depth of the section.
    pub fn size(self) -> (f64, f64) {
        match self {
            Profile::Rectangle { width, depth } => (width, depth),
            Profile::Circle { diameter } => (diameter, diameter),
            Profile::IShape { width, depth, .. }
            | Profile::UShape { width, depth, .. }
            | Profile::TShape { width, depth, .. }
            | Profile::LShape { width, depth, .. } => (width, depth),
        }
    }

    /// Area of the section, in m².
    pub fn area(self) -> f64 {
        match self {
            Profile::Rectangle { width, depth } => width * depth,
            Profile::Circle { diameter } => std::f64::consts::PI * diameter * diameter / 4.0,
            Profile::IShape {
                width,
                depth,
                web,
                flange,
            } => 2.0 * width * flange + (depth - 2.0 * flange) * web,
            Profile::UShape {
                width,
                depth,
                web,
                flange,
            } => 2.0 * width * flange + (depth - 2.0 * flange) * web,
            Profile::TShape {
                width,
                depth,
                web,
                flange,
            } => width * flange + (depth - flange) * web,
            Profile::LShape {
                width,
                depth,
                thickness,
            } => thickness * (width + depth - thickness),
        }
    }

    /// The outline of the section, counter-clockwise, centred on the
    /// origin. Circles are drawn with `segments` sides.
    pub fn outline(self, segments: usize) -> Vec<Point2> {
        let p = |x: f64, y: f64| Point2 { x, y };
        match self {
            Profile::Rectangle { width, depth } => {
                let (a, b) = (width / 2.0, depth / 2.0);
                vec![p(-a, -b), p(a, -b), p(a, b), p(-a, b)]
            }
            Profile::Circle { diameter } => {
                let n = segments.max(8);
                let r = diameter / 2.0;
                (0..n)
                    .map(|i| {
                        let t = std::f64::consts::TAU * i as f64 / n as f64;
                        p(r * t.cos(), r * t.sin())
                    })
                    .collect()
            }
            Profile::IShape {
                width,
                depth,
                web,
                flange,
            } => {
                let (a, b, w, f) = (width / 2.0, depth / 2.0, web / 2.0, flange);
                vec![
                    p(-a, -b),
                    p(a, -b),
                    p(a, -b + f),
                    p(w, -b + f),
                    p(w, b - f),
                    p(a, b - f),
                    p(a, b),
                    p(-a, b),
                    p(-a, b - f),
                    p(-w, b - f),
                    p(-w, -b + f),
                    p(-a, -b + f),
                ]
            }
            Profile::UShape {
                width,
                depth,
                web,
                flange,
            } => {
                let (a, b, f) = (width / 2.0, depth / 2.0, flange);
                vec![
                    p(-a, -b),
                    p(a, -b),
                    p(a, -b + f),
                    p(-a + web, -b + f),
                    p(-a + web, b - f),
                    p(a, b - f),
                    p(a, b),
                    p(-a, b),
                ]
            }
            Profile::TShape {
                width,
                depth,
                web,
                flange,
            } => {
                let (a, b, w, f) = (width / 2.0, depth / 2.0, web / 2.0, flange);
                vec![
                    p(-w, -b),
                    p(w, -b),
                    p(w, b - f),
                    p(a, b - f),
                    p(a, b),
                    p(-a, b),
                    p(-a, b - f),
                    p(-w, b - f),
                ]
            }
            Profile::LShape {
                width,
                depth,
                thickness: t,
            } => {
                let (a, b) = (width / 2.0, depth / 2.0);
                vec![
                    p(-a, -b),
                    p(a, -b),
                    p(a, -b + t),
                    p(-a + t, -b + t),
                    p(-a + t, b),
                    p(-a, b),
                ]
            }
        }
    }

    fn validate(self) -> Result<(), ModelError> {
        let positive = |v: f64, what: &'static str| -> Result<(), ModelError> {
            finite(v, what)?;
            if v <= EPSILON {
                return Err(ModelError::NonPositive(what));
            }
            Ok(())
        };
        match self {
            Profile::Rectangle { width, depth } => {
                positive(width, "section width")?;
                positive(depth, "section depth")
            }
            Profile::Circle { diameter } => positive(diameter, "section diameter"),
            Profile::IShape {
                width,
                depth,
                web,
                flange,
            } => {
                positive(width, "section width")?;
                positive(depth, "section depth")?;
                positive(web, "web thickness")?;
                positive(flange, "flange thickness")?;
                if web >= width || 2.0 * flange >= depth {
                    return Err(ModelError::InvalidProfile);
                }
                Ok(())
            }
            Profile::UShape {
                width,
                depth,
                web,
                flange,
            }
            | Profile::TShape {
                width,
                depth,
                web,
                flange,
            } => {
                positive(width, "section width")?;
                positive(depth, "section depth")?;
                positive(web, "web thickness")?;
                positive(flange, "flange thickness")?;
                let u = matches!(self, Profile::UShape { .. });
                let flanges = if u { 2.0 } else { 1.0 };
                if web >= width || flanges * flange >= depth {
                    return Err(ModelError::InvalidProfile);
                }
                Ok(())
            }
            Profile::LShape {
                width,
                depth,
                thickness,
            } => {
                positive(width, "section width")?;
                positive(depth, "section depth")?;
                positive(thickness, "leg thickness")?;
                if thickness >= width.min(depth) {
                    return Err(ModelError::InvalidProfile);
                }
                Ok(())
            }
        }
    }
}

/// Registers an id (and GUID) while rebuilding a model, rejecting
/// duplicates.
pub(super) type Claim<'a> = dyn FnMut(u64, Option<&GlobalId>) -> Result<(), ModelError> + 'a;

/// Whether a member type is for columns or for beams.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemberUsage {
    Column,
    Beam,
}

/// A column or beam type. IFC: `IfcColumnType` / `IfcBeamType` with an
/// `IfcMaterialProfileSet`.
#[derive(Debug, Clone, PartialEq)]
pub struct MemberType {
    pub id: MemberTypeId,
    pub guid: GlobalId,
    pub name: String,
    pub usage: MemberUsage,
    pub material: MaterialId,
    pub profile: Profile,
}

/// A column standing on a level. IFC: `IfcColumn`.
///
/// Like a wall, it may reach up to a level (its top constraint), and then
/// its height follows the levels.
#[derive(Debug, Clone, PartialEq)]
pub struct Column {
    pub id: ColumnId,
    pub guid: GlobalId,
    pub member_type: MemberTypeId,
    pub level: LevelId,
    /// Centre of the section in plan.
    pub position: Point2,
    /// Angle of the section's `x` axis from the project's `x`, in radians.
    pub rotation: f64,
    /// Offset of the base above the level.
    pub base_offset: f64,
    pub top: WallTop,
    /// Height above the base; worked out by the model with a top level.
    pub height: f64,
}

/// Data needed to create a column.
#[derive(Debug, Clone, PartialEq)]
pub struct NewColumn {
    pub member_type: MemberTypeId,
    pub level: LevelId,
    pub position: Point2,
    pub rotation: f64,
    pub base_offset: f64,
    pub top: WallTop,
    pub height: f64,
}

/// A beam between two points. IFC: `IfcBeam`.
///
/// Its reference is the axis of the **top face**, at the level elevation
/// plus `top_offset` (0: flush with the top of the slab on that level, as
/// a beam under a floor is usually drawn); the section hangs down.
#[derive(Debug, Clone, PartialEq)]
pub struct Beam {
    pub id: BeamId,
    pub guid: GlobalId,
    pub member_type: MemberTypeId,
    pub level: LevelId,
    pub start: Point2,
    pub end: Point2,
    pub top_offset: f64,
}

impl Beam {
    pub fn length(&self) -> f64 {
        self.start.distance(self.end)
    }
}

/// Data needed to create a beam.
#[derive(Debug, Clone, PartialEq)]
pub struct NewBeam {
    pub member_type: MemberTypeId,
    pub level: LevelId,
    pub start: Point2,
    pub end: Point2,
    pub top_offset: f64,
}

impl Column {
    /// The section's outline in plan, in project coordinates.
    pub fn plan_outline(&self, profile: Profile, segments: usize) -> Vec<Point2> {
        let (c, s) = (self.rotation.cos(), self.rotation.sin());
        profile
            .outline(segments)
            .into_iter()
            .map(|p| Point2 {
                x: self.position.x + p.x * c - p.y * s,
                y: self.position.y + p.x * s + p.y * c,
            })
            .collect()
    }
}

impl Model {
    // ---- Member types ---------------------------------------------------

    pub fn add_member_type(
        &mut self,
        name: impl Into<String>,
        usage: MemberUsage,
        material: MaterialId,
        profile: Profile,
    ) -> Result<MemberTypeId, ModelError> {
        if !self.materials.contains_key(&material) {
            return Err(ModelError::UnknownMaterial(material));
        }
        profile.validate()?;
        let id = MemberTypeId(self.take_id());
        self.member_types.insert(
            id,
            MemberType {
                id,
                guid: GlobalId::new(),
                name: name.into(),
                usage,
                material,
                profile,
            },
        );
        Ok(id)
    }

    pub fn member_type(&self, id: MemberTypeId) -> Option<&MemberType> {
        self.member_types.get(&id)
    }

    pub fn member_types(&self) -> impl Iterator<Item = &MemberType> {
        self.member_types.values()
    }

    /// Changes a member type's name, material and section; every column or
    /// beam of the type follows.
    pub fn edit_member_type(
        &mut self,
        id: MemberTypeId,
        name: impl Into<String>,
        material: MaterialId,
        profile: Profile,
    ) -> Result<(), ModelError> {
        if !self.materials.contains_key(&material) {
            return Err(ModelError::UnknownMaterial(material));
        }
        profile.validate()?;
        let t = self
            .member_types
            .get_mut(&id)
            .ok_or(ModelError::UnknownMemberType(id))?;
        t.name = name.into();
        t.material = material;
        t.profile = profile;
        Ok(())
    }

    /// Copies a member type under a new name and a new GlobalId.
    pub fn duplicate_member_type(
        &mut self,
        id: MemberTypeId,
        name: impl Into<String>,
    ) -> Result<MemberTypeId, ModelError> {
        let t = self
            .member_types
            .get(&id)
            .cloned()
            .ok_or(ModelError::UnknownMemberType(id))?;
        self.add_member_type(name, t.usage, t.material, t.profile)
    }

    /// How many columns or beams use a type.
    pub fn member_type_users(&self, id: MemberTypeId) -> usize {
        self.columns
            .values()
            .filter(|c| c.member_type == id)
            .count()
            + self.beams.values().filter(|b| b.member_type == id).count()
    }

    pub fn remove_member_type(&mut self, id: MemberTypeId) -> Result<MemberType, ModelError> {
        let users = self.member_type_users(id);
        if users > 0 {
            return Err(ModelError::InUse(format!(
                "{users} element(s) use this type"
            )));
        }
        self.member_types
            .remove(&id)
            .ok_or(ModelError::UnknownMemberType(id))
    }

    fn check_member_type(&self, id: MemberTypeId, usage: MemberUsage) -> Result<(), ModelError> {
        let t = self
            .member_types
            .get(&id)
            .ok_or(ModelError::UnknownMemberType(id))?;
        if t.usage != usage {
            return Err(ModelError::CategoryMismatch);
        }
        Ok(())
    }

    // ---- Columns ----------------------------------------------------------

    fn validate_column(&self, spec: &NewColumn) -> Result<f64, ModelError> {
        if !self.levels.contains_key(&spec.level) {
            return Err(ModelError::UnknownLevel(spec.level));
        }
        self.check_member_type(spec.member_type, MemberUsage::Column)?;
        finite_point(spec.position, "column position")?;
        finite(spec.rotation, "column rotation")?;
        finite(spec.base_offset, "column base offset")?;
        finite(spec.height, "column height")?;
        if let WallTop::Level { level, offset } = spec.top {
            if !self.levels.contains_key(&level) {
                return Err(ModelError::UnknownLevel(level));
            }
            finite(offset, "column top offset")?;
        }
        let height = self
            .resolved_height(spec.level, spec.base_offset, spec.top, spec.height)
            .ok_or(ModelError::UnknownLevel(spec.level))?;
        if height <= EPSILON {
            return Err(match spec.top {
                WallTop::Unconnected => ModelError::NonPositive("column height"),
                WallTop::Level { .. } => ModelError::ColumnTopBelowBase(ColumnId(0)),
            });
        }
        Ok(height)
    }

    pub fn add_column(&mut self, spec: NewColumn) -> Result<ColumnId, ModelError> {
        let height = self.validate_column(&spec)?;
        let id = ColumnId(self.take_id());
        self.columns.insert(
            id,
            Column {
                id,
                guid: GlobalId::new(),
                member_type: spec.member_type,
                level: spec.level,
                position: spec.position,
                rotation: spec.rotation,
                base_offset: spec.base_offset,
                top: spec.top,
                height,
            },
        );
        self.stamp_phase(id.0);
        Ok(id)
    }

    /// Edits a column and works out its height again; nothing changes if
    /// the result is not valid.
    pub fn edit_column(
        &mut self,
        id: ColumnId,
        edit: impl FnOnce(&mut Column),
    ) -> Result<(), ModelError> {
        let mut c = self
            .columns
            .get(&id)
            .cloned()
            .ok_or(ModelError::UnknownColumn(id))?;
        edit(&mut c);
        c.height = self
            .validate_column(&NewColumn {
                member_type: c.member_type,
                level: c.level,
                position: c.position,
                rotation: c.rotation,
                base_offset: c.base_offset,
                top: c.top,
                height: c.height,
            })
            .map_err(|e| match e {
                ModelError::ColumnTopBelowBase(_) => ModelError::ColumnTopBelowBase(id),
                e => e,
            })?;
        self.columns.insert(id, c);
        Ok(())
    }

    /// Moves a column to another level, keeping its offsets; one that
    /// reached the level above its old one reaches the level above the new
    /// one (as walls do).
    pub fn set_column_level(&mut self, id: ColumnId, level: LevelId) -> Result<(), ModelError> {
        if !self.levels.contains_key(&level) {
            return Err(ModelError::UnknownLevel(level));
        }
        let c = self.columns.get(&id).ok_or(ModelError::UnknownColumn(id))?;
        if c.level == level {
            return Ok(());
        }
        let top = match c.top {
            WallTop::Level { level: top, offset } if Some(top) == self.level_above(c.level) => {
                match self.level_above(level) {
                    Some(l) => WallTop::Level { level: l, offset },
                    None => WallTop::Unconnected,
                }
            }
            t => t,
        };
        self.edit_column(id, |c| {
            c.level = level;
            c.top = top;
        })
    }

    pub fn remove_column(&mut self, id: ColumnId) -> Result<Column, ModelError> {
        self.columns
            .remove(&id)
            .ok_or(ModelError::UnknownColumn(id))
    }

    pub fn column(&self, id: ColumnId) -> Option<&Column> {
        self.columns.get(&id)
    }

    pub fn columns(&self) -> impl Iterator<Item = &Column> {
        self.columns.values()
    }

    pub fn columns_on_level(&self, level: LevelId) -> impl Iterator<Item = &Column> {
        self.columns.values().filter(move |c| c.level == level)
    }

    // ---- Beams -------------------------------------------------------------

    fn validate_beam(&self, spec: &NewBeam) -> Result<(), ModelError> {
        if !self.levels.contains_key(&spec.level) {
            return Err(ModelError::UnknownLevel(spec.level));
        }
        self.check_member_type(spec.member_type, MemberUsage::Beam)?;
        finite_point(spec.start, "beam start")?;
        finite_point(spec.end, "beam end")?;
        finite(spec.top_offset, "beam top offset")?;
        if spec.start.approx_eq(spec.end) {
            return Err(ModelError::ZeroLengthBeam);
        }
        Ok(())
    }

    pub fn add_beam(&mut self, spec: NewBeam) -> Result<BeamId, ModelError> {
        self.validate_beam(&spec)?;
        let id = BeamId(self.take_id());
        self.beams.insert(
            id,
            Beam {
                id,
                guid: GlobalId::new(),
                member_type: spec.member_type,
                level: spec.level,
                start: spec.start,
                end: spec.end,
                top_offset: spec.top_offset,
            },
        );
        self.stamp_phase(id.0);
        Ok(id)
    }

    pub fn edit_beam(
        &mut self,
        id: BeamId,
        edit: impl FnOnce(&mut Beam),
    ) -> Result<(), ModelError> {
        let mut b = self
            .beams
            .get(&id)
            .cloned()
            .ok_or(ModelError::UnknownBeam(id))?;
        edit(&mut b);
        self.validate_beam(&NewBeam {
            member_type: b.member_type,
            level: b.level,
            start: b.start,
            end: b.end,
            top_offset: b.top_offset,
        })?;
        self.beams.insert(id, b);
        Ok(())
    }

    pub fn remove_beam(&mut self, id: BeamId) -> Result<Beam, ModelError> {
        self.beams.remove(&id).ok_or(ModelError::UnknownBeam(id))
    }

    pub fn beam(&self, id: BeamId) -> Option<&Beam> {
        self.beams.get(&id)
    }

    pub fn beams(&self) -> impl Iterator<Item = &Beam> {
        self.beams.values()
    }

    pub fn beams_on_level(&self, level: LevelId) -> impl Iterator<Item = &Beam> {
        self.beams.values().filter(move |b| b.level == level)
    }

    // ---- Shared ------------------------------------------------------------

    /// Columns tied to a level follow its elevation (see
    /// [`Model::set_level_elevation`]).
    pub(super) fn refresh_column_heights(&mut self) -> Result<(), ModelError> {
        let tied: Vec<ColumnId> = self
            .columns
            .values()
            .filter(|c| matches!(c.top, WallTop::Level { .. }))
            .map(|c| c.id)
            .collect();
        for id in tied {
            self.edit_column(id, |_| {})?;
        }
        Ok(())
    }

    /// When a level goes: its columns and beams go with it, and columns of
    /// other levels tied to it keep their height, free.
    pub(super) fn remove_members_on_level(&mut self, level: LevelId) {
        self.columns.retain(|_, c| c.level != level);
        self.beams.retain(|_, b| b.level != level);
        for c in self.columns.values_mut() {
            if matches!(c.top, WallTop::Level { level: l, .. } if l == level) {
                c.top = WallTop::Unconnected;
            }
        }
    }

    pub(super) fn restore_members(
        &mut self,
        types: Vec<MemberType>,
        columns: Vec<Column>,
        beams: Vec<Beam>,
        claim: &mut Claim<'_>,
    ) -> Result<(), RestoreError> {
        let fail = |element: String| move |error: ModelError| RestoreError { element, error };
        for t in types {
            let name = format!("member type {} «{}»", t.id.0, t.name);
            claim(t.id.0, Some(&t.guid))
                .and_then(|_| {
                    if !self.materials.contains_key(&t.material) {
                        return Err(ModelError::UnknownMaterial(t.material));
                    }
                    t.profile.validate()
                })
                .map_err(fail(name))?;
            self.member_types.insert(t.id, t);
        }
        for mut c in columns {
            let name = format!("column {} ({})", c.id.0, c.guid);
            let spec = NewColumn {
                member_type: c.member_type,
                level: c.level,
                position: c.position,
                rotation: c.rotation,
                base_offset: c.base_offset,
                top: c.top,
                height: c.height,
            };
            c.height = claim(c.id.0, Some(&c.guid))
                .and_then(|_| self.validate_column(&spec))
                .map_err(|e| match e {
                    ModelError::ColumnTopBelowBase(_) => ModelError::ColumnTopBelowBase(c.id),
                    e => e,
                })
                .map_err(fail(name))?;
            self.columns.insert(c.id, c);
        }
        for b in beams {
            let name = format!("beam {} ({})", b.id.0, b.guid);
            let spec = NewBeam {
                member_type: b.member_type,
                level: b.level,
                start: b.start,
                end: b.end,
                top_offset: b.top_offset,
            };
            claim(b.id.0, Some(&b.guid))
                .and_then(|_| self.validate_beam(&spec))
                .map_err(fail(name))?;
            self.beams.insert(b.id, b);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn model() -> (Model, LevelId, LevelId, MaterialId) {
        let mut m = Model::new("t");
        let l0 = m.add_level("PB", 0.0);
        let l1 = m.add_level("P1", 3.0);
        let mat = m.add_material(
            "Hormigón armado",
            "concrete",
            [200, 200, 200],
            CutPattern::Cross,
        );
        (m, l0, l1, mat)
    }

    #[test]
    fn column_follows_its_top_level() {
        let (mut m, l0, l1, mat) = model();
        let t = m
            .add_member_type(
                "HA 30×30",
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
                level: l0,
                position: Point2 { x: 1.0, y: 2.0 },
                rotation: 0.0,
                base_offset: 0.0,
                top: WallTop::Level {
                    level: l1,
                    offset: -0.3,
                },
                height: 1.0,
            })
            .unwrap();
        assert!((m.column(c).unwrap().height - 2.7).abs() < 1e-9);
        m.set_level_elevation(l1, 3.5).unwrap();
        assert!((m.column(c).unwrap().height - 3.2).abs() < 1e-9);
        // Too low: rejected and nothing changes.
        assert!(m.set_level_elevation(l1, 0.2).is_err());
        assert!((m.column(c).unwrap().height - 3.2).abs() < 1e-9);
        m.remove_level(l1).unwrap();
        assert_eq!(m.column(c).unwrap().top, WallTop::Unconnected);
    }

    #[test]
    fn beam_type_cannot_be_a_column() {
        let (mut m, l0, _, mat) = model();
        let t = m
            .add_member_type(
                "IPE 300",
                MemberUsage::Beam,
                mat,
                Profile::IShape {
                    width: 0.15,
                    depth: 0.3,
                    web: 0.0071,
                    flange: 0.0107,
                },
            )
            .unwrap();
        let col = NewColumn {
            member_type: t,
            level: l0,
            position: Point2::default(),
            rotation: 0.0,
            base_offset: 0.0,
            top: WallTop::Unconnected,
            height: 3.0,
        };
        assert_eq!(m.add_column(col), Err(ModelError::CategoryMismatch));
        let b = m
            .add_beam(NewBeam {
                member_type: t,
                level: l0,
                start: Point2 { x: 0.0, y: 0.0 },
                end: Point2 { x: 5.0, y: 0.0 },
                top_offset: 0.0,
            })
            .unwrap();
        assert_eq!(m.member_type_users(t), 1);
        assert!(m.remove_member_type(t).is_err());
        m.remove_beam(b).unwrap();
        m.remove_member_type(t).unwrap();
    }

    #[test]
    fn profiles() {
        let i = Profile::IShape {
            width: 0.2,
            depth: 0.3,
            web: 0.01,
            flange: 0.02,
        };
        let a = signed_area(&i.outline(0));
        assert!((a - i.area()).abs() < 1e-12);
        let c = Profile::Circle { diameter: 0.3 };
        assert!((signed_area(&c.outline(64)) - c.area()).abs() < 2e-3);
        assert_eq!(
            Profile::IShape {
                width: 0.2,
                depth: 0.3,
                web: 0.3,
                flange: 0.02
            }
            .validate(),
            Err(ModelError::InvalidProfile)
        );
    }

    #[test]
    fn parts_round_trip() {
        let (mut m, l0, l1, mat) = model();
        let t = m
            .add_member_type(
                "Ø30",
                MemberUsage::Column,
                mat,
                Profile::Circle { diameter: 0.3 },
            )
            .unwrap();
        m.add_column(NewColumn {
            member_type: t,
            level: l0,
            position: Point2 { x: 3.0, y: 0.0 },
            rotation: 0.5,
            base_offset: 0.0,
            top: WallTop::Level {
                level: l1,
                offset: 0.0,
            },
            height: 0.0,
        })
        .unwrap();
        let back = Model::from_parts(m.to_parts()).unwrap();
        assert_eq!(back, m);
    }
}

#[cfg(test)]
mod profile_tests {
    use super::*;

    #[test]
    fn steel_sections_have_the_area_of_their_outline() {
        for p in [
            Profile::UShape {
                width: 0.075,
                depth: 0.2,
                web: 0.0085,
                flange: 0.0115,
            },
            Profile::TShape {
                width: 0.1,
                depth: 0.1,
                web: 0.011,
                flange: 0.011,
            },
            Profile::LShape {
                width: 0.08,
                depth: 0.06,
                thickness: 0.008,
            },
        ] {
            p.validate().unwrap();
            let o = p.outline(16);
            assert!(signed_area(&o) > 0.0, "{p:?}");
            assert!((signed_area(&o) - p.area()).abs() < 1e-12, "{p:?}");
            let (lo, hi) = o
                .iter()
                .fold((f64::MAX, f64::MIN), |(l, h), q| (l.min(q.x), h.max(q.x)));
            assert!((lo + hi).abs() < 1e-12, "centred on its box: {p:?}");
        }
        assert!(Profile::LShape {
            width: 0.05,
            depth: 0.05,
            thickness: 0.05
        }
        .validate()
        .is_err());
    }
}
