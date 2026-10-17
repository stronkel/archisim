//! Building elements and the [`Model`] that holds them.
//!
//! Naming follows Revit's organisation (types and instances) while the
//! defaults and vocabulary of the tools follow ArchiCAD. Each struct notes its
//! IFC equivalent.

use std::collections::BTreeMap;
use std::fmt;

use crate::ids::*;
use crate::math::{Point2, EPSILON};

/// A floor of the building. IFC: `IfcBuildingStorey`.
#[derive(Debug, Clone, PartialEq)]
pub struct Level {
    pub id: LevelId,
    pub guid: GlobalId,
    pub name: String,
    /// Height of the floor above project zero, in metres.
    pub elevation: f64,
}

/// How a material is drawn when cut in plan or section.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CutPattern {
    /// Fill colour only.
    Solid,
    /// Diagonal lines (masonry, brick).
    Diagonal,
    /// Cross-hatching (concrete in many standards).
    Cross,
    /// Wavy line (insulation).
    Insulation,
    /// Dots (render, plaster, mortar).
    Dots,
    /// Nothing (air gaps).
    Empty,
}

/// A building material. IFC: `IfcMaterial`.
#[derive(Debug, Clone, PartialEq)]
pub struct Material {
    pub id: MaterialId,
    pub name: String,
    /// IFC material category, e.g. `"concrete"`, `"brick"`, `"insulation"`.
    pub category: String,
    /// Fill colour when cut, RGB.
    pub cut_color: [u8; 3],
    pub cut_pattern: CutPattern,
}

/// What a layer does inside a composite element. Mirrors Revit's
/// "Function" column of compound structures. IFC: `IfcMaterialLayer.Category`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LayerFunction {
    Structure,
    Substrate,
    Insulation,
    AirGap,
    Membrane,
    Finish,
}

impl LayerFunction {
    /// Category string used in IFC exports (IFC4 recommended values).
    pub fn ifc_category(self) -> &'static str {
        match self {
            LayerFunction::Structure => "LoadBearing",
            LayerFunction::Substrate => "Substrate",
            LayerFunction::Insulation => "Insulation",
            LayerFunction::AirGap => "AirGap",
            LayerFunction::Membrane => "Membrane",
            LayerFunction::Finish => "Finish",
        }
    }
}

/// One layer of a composite (ArchiCAD "multicapa", Revit "compound structure").
#[derive(Debug, Clone, PartialEq)]
pub struct Layer {
    pub material: MaterialId,
    /// Thickness in metres.
    pub thickness: f64,
    pub function: LayerFunction,
}

/// Whether a wall type faces the outside. Revit type parameter "Function".
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WallFunction {
    Exterior,
    Interior,
}

/// A wall type. IFC: `IfcWallType` + `IfcMaterialLayerSet`.
#[derive(Debug, Clone, PartialEq)]
pub struct WallType {
    pub id: WallTypeId,
    pub guid: GlobalId,
    pub name: String,
    pub function: WallFunction,
    /// Layers ordered from the exterior face to the interior face.
    pub layers: Vec<Layer>,
}

impl WallType {
    pub fn thickness(&self) -> f64 {
        self.layers.iter().map(|l| l.thickness).sum()
    }

    /// A type is load-bearing if any of its layers is structural.
    pub fn is_load_bearing(&self) -> bool {
        self.layers
            .iter()
            .any(|l| l.function == LayerFunction::Structure)
    }
}

/// Where the drawn line sits within the wall thickness.
/// ArchiCAD "línea de referencia", Revit "Location Line".
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ReferenceLine {
    #[default]
    Center,
    /// The drawn line is the exterior face.
    ExteriorFace,
    /// The drawn line is the interior face.
    InteriorFace,
}

/// A straight wall. IFC: `IfcWall` with an `IfcMaterialLayerSetUsage`.
///
/// The exterior face is on the **left** of the axis when looking from `start`
/// to `end`. Drawing a building outline clockwise therefore puts the exterior
/// layers outside.
#[derive(Debug, Clone, PartialEq)]
pub struct Wall {
    pub id: WallId,
    pub guid: GlobalId,
    pub wall_type: WallTypeId,
    pub level: LevelId,
    pub start: Point2,
    pub end: Point2,
    pub reference: ReferenceLine,
    /// Height above the base, in metres.
    pub height: f64,
    /// Offset of the base above the level, in metres.
    pub base_offset: f64,
}

impl Wall {
    pub fn length(&self) -> f64 {
        self.start.distance(self.end)
    }

    /// Signed offsets (left positive) of the exterior and interior faces from
    /// the drawn line, for a wall of the given thickness.
    pub fn face_offsets(&self, thickness: f64) -> (f64, f64) {
        match self.reference {
            ReferenceLine::Center => (thickness / 2.0, -thickness / 2.0),
            ReferenceLine::ExteriorFace => (0.0, -thickness),
            ReferenceLine::InteriorFace => (thickness, 0.0),
        }
    }
}

/// Data needed to create a wall; ids and GUIDs are assigned by the model.
#[derive(Debug, Clone, PartialEq)]
pub struct NewWall {
    pub wall_type: WallTypeId,
    pub level: LevelId,
    pub start: Point2,
    pub end: Point2,
    pub reference: ReferenceLine,
    pub height: f64,
    pub base_offset: f64,
}

/// Reasons an edit can be rejected.
#[derive(Debug, Clone, PartialEq)]
pub enum ModelError {
    UnknownLevel(LevelId),
    UnknownMaterial(MaterialId),
    UnknownWallType(WallTypeId),
    UnknownWall(WallId),
    /// The wall's start and end are the same point.
    ZeroLengthWall,
    /// A dimension that must be positive is not.
    NonPositive(&'static str),
    /// A wall type must have at least one layer.
    EmptyWallType,
    /// The element is still used by others and cannot be removed.
    InUse(String),
}

impl fmt::Display for ModelError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ModelError::UnknownLevel(id) => write!(f, "level {id} does not exist"),
            ModelError::UnknownMaterial(id) => write!(f, "material {id} does not exist"),
            ModelError::UnknownWallType(id) => write!(f, "wall type {id} does not exist"),
            ModelError::UnknownWall(id) => write!(f, "wall {id} does not exist"),
            ModelError::ZeroLengthWall => write!(f, "a wall needs two different points"),
            ModelError::NonPositive(what) => write!(f, "{what} must be greater than zero"),
            ModelError::EmptyWallType => write!(f, "a wall type needs at least one layer"),
            ModelError::InUse(what) => write!(f, "still in use: {what}"),
        }
    }
}

impl std::error::Error for ModelError {}

/// The building model: every element of a project, without undo history.
#[derive(Debug, Clone, PartialEq)]
pub struct Model {
    pub project_name: String,
    pub project_guid: GlobalId,
    pub site_guid: GlobalId,
    pub building_guid: GlobalId,
    next_id: u64,
    levels: BTreeMap<LevelId, Level>,
    materials: BTreeMap<MaterialId, Material>,
    wall_types: BTreeMap<WallTypeId, WallType>,
    walls: BTreeMap<WallId, Wall>,
}

impl Model {
    pub fn new(project_name: impl Into<String>) -> Self {
        Self {
            project_name: project_name.into(),
            project_guid: GlobalId::new(),
            site_guid: GlobalId::new(),
            building_guid: GlobalId::new(),
            next_id: 1,
            levels: BTreeMap::new(),
            materials: BTreeMap::new(),
            wall_types: BTreeMap::new(),
            walls: BTreeMap::new(),
        }
    }

    fn take_id(&mut self) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        id
    }

    // ---- Levels -------------------------------------------------------

    pub fn add_level(&mut self, name: impl Into<String>, elevation: f64) -> LevelId {
        let id = LevelId(self.take_id());
        self.levels.insert(
            id,
            Level {
                id,
                guid: GlobalId::new(),
                name: name.into(),
                elevation,
            },
        );
        id
    }

    pub fn level(&self, id: LevelId) -> Option<&Level> {
        self.levels.get(&id)
    }

    /// Levels sorted by elevation.
    pub fn levels(&self) -> Vec<&Level> {
        let mut v: Vec<_> = self.levels.values().collect();
        v.sort_by(|a, b| a.elevation.total_cmp(&b.elevation));
        v
    }

    // ---- Materials ----------------------------------------------------

    pub fn add_material(
        &mut self,
        name: impl Into<String>,
        category: impl Into<String>,
        cut_color: [u8; 3],
        cut_pattern: CutPattern,
    ) -> MaterialId {
        let id = MaterialId(self.take_id());
        self.materials.insert(
            id,
            Material {
                id,
                name: name.into(),
                category: category.into(),
                cut_color,
                cut_pattern,
            },
        );
        id
    }

    pub fn material(&self, id: MaterialId) -> Option<&Material> {
        self.materials.get(&id)
    }

    pub fn materials(&self) -> impl Iterator<Item = &Material> {
        self.materials.values()
    }

    // ---- Wall types ---------------------------------------------------

    pub fn add_wall_type(
        &mut self,
        name: impl Into<String>,
        function: WallFunction,
        layers: Vec<Layer>,
    ) -> Result<WallTypeId, ModelError> {
        self.validate_layers(&layers)?;
        let id = WallTypeId(self.take_id());
        self.wall_types.insert(
            id,
            WallType {
                id,
                guid: GlobalId::new(),
                name: name.into(),
                function,
                layers,
            },
        );
        Ok(id)
    }

    fn validate_layers(&self, layers: &[Layer]) -> Result<(), ModelError> {
        if layers.is_empty() {
            return Err(ModelError::EmptyWallType);
        }
        for layer in layers {
            if !self.materials.contains_key(&layer.material) {
                return Err(ModelError::UnknownMaterial(layer.material));
            }
            if layer.thickness <= EPSILON {
                return Err(ModelError::NonPositive("layer thickness"));
            }
        }
        Ok(())
    }

    pub fn wall_type(&self, id: WallTypeId) -> Option<&WallType> {
        self.wall_types.get(&id)
    }

    pub fn wall_types(&self) -> impl Iterator<Item = &WallType> {
        self.wall_types.values()
    }

    // ---- Walls --------------------------------------------------------

    pub fn add_wall(&mut self, spec: NewWall) -> Result<WallId, ModelError> {
        self.validate_wall(&spec)?;
        let id = WallId(self.take_id());
        let wall = Wall {
            id,
            guid: GlobalId::new(),
            wall_type: spec.wall_type,
            level: spec.level,
            start: spec.start,
            end: spec.end,
            reference: spec.reference,
            height: spec.height,
            base_offset: spec.base_offset,
        };
        self.walls.insert(id, wall);
        Ok(id)
    }

    fn validate_wall(&self, spec: &NewWall) -> Result<(), ModelError> {
        if !self.levels.contains_key(&spec.level) {
            return Err(ModelError::UnknownLevel(spec.level));
        }
        if !self.wall_types.contains_key(&spec.wall_type) {
            return Err(ModelError::UnknownWallType(spec.wall_type));
        }
        if spec.start.approx_eq(spec.end) {
            return Err(ModelError::ZeroLengthWall);
        }
        if spec.height <= EPSILON {
            return Err(ModelError::NonPositive("wall height"));
        }
        Ok(())
    }

    /// Moves a wall's end points, keeping its identity and GUID.
    pub fn move_wall(&mut self, id: WallId, start: Point2, end: Point2) -> Result<(), ModelError> {
        if start.approx_eq(end) {
            return Err(ModelError::ZeroLengthWall);
        }
        let wall = self.walls.get_mut(&id).ok_or(ModelError::UnknownWall(id))?;
        wall.start = start;
        wall.end = end;
        Ok(())
    }

    /// Swaps start and end, which moves the exterior face to the other side
    /// (ArchiCAD "Invertir"). The drawn line stays where it is.
    pub fn flip_wall(&mut self, id: WallId) -> Result<(), ModelError> {
        let wall = self.walls.get_mut(&id).ok_or(ModelError::UnknownWall(id))?;
        std::mem::swap(&mut wall.start, &mut wall.end);
        Ok(())
    }

    pub fn remove_wall(&mut self, id: WallId) -> Result<Wall, ModelError> {
        self.walls.remove(&id).ok_or(ModelError::UnknownWall(id))
    }

    pub fn remove_wall_type(&mut self, id: WallTypeId) -> Result<WallType, ModelError> {
        let users = self.walls.values().filter(|w| w.wall_type == id).count();
        if users > 0 {
            return Err(ModelError::InUse(format!("{users} wall(s) use this type")));
        }
        self.wall_types
            .remove(&id)
            .ok_or(ModelError::UnknownWallType(id))
    }

    pub fn wall(&self, id: WallId) -> Option<&Wall> {
        self.walls.get(&id)
    }

    pub fn walls(&self) -> impl Iterator<Item = &Wall> {
        self.walls.values()
    }

    pub fn walls_on_level(&self, level: LevelId) -> impl Iterator<Item = &Wall> {
        self.walls.values().filter(move |w| w.level == level)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> (Model, LevelId, WallTypeId) {
        let mut m = Model::new("Test");
        let lvl = m.add_level("Planta baja", 0.0);
        let brick = m.add_material("Ladrillo", "brick", [200, 90, 60], CutPattern::Diagonal);
        let t = m
            .add_wall_type(
                "Tabique",
                WallFunction::Interior,
                vec![Layer {
                    material: brick,
                    thickness: 0.1,
                    function: LayerFunction::Structure,
                }],
            )
            .unwrap();
        (m, lvl, t)
    }

    fn spec(level: LevelId, wall_type: WallTypeId) -> NewWall {
        NewWall {
            wall_type,
            level,
            start: Point2::new(0.0, 0.0),
            end: Point2::new(4.0, 0.0),
            reference: ReferenceLine::Center,
            height: 2.7,
            base_offset: 0.0,
        }
    }

    #[test]
    fn rejects_invalid_walls() {
        let (mut m, lvl, t) = sample();
        let mut s = spec(lvl, t);
        s.end = s.start;
        assert_eq!(m.add_wall(s), Err(ModelError::ZeroLengthWall));
        let mut s = spec(lvl, t);
        s.height = 0.0;
        assert!(matches!(m.add_wall(s), Err(ModelError::NonPositive(_))));
        let s = spec(LevelId(999), t);
        assert!(matches!(m.add_wall(s), Err(ModelError::UnknownLevel(_))));
    }

    #[test]
    fn type_in_use_cannot_be_removed() {
        let (mut m, lvl, t) = sample();
        let w = m.add_wall(spec(lvl, t)).unwrap();
        assert!(matches!(m.remove_wall_type(t), Err(ModelError::InUse(_))));
        m.remove_wall(w).unwrap();
        assert!(m.remove_wall_type(t).is_ok());
    }

    #[test]
    fn face_offsets_follow_reference_line() {
        let (mut m, lvl, t) = sample();
        let mut s = spec(lvl, t);
        s.reference = ReferenceLine::ExteriorFace;
        let id = m.add_wall(s).unwrap();
        let w = m.wall(id).unwrap();
        let (e, i) = w.face_offsets(0.3);
        assert_eq!((e, i), (0.0, -0.3));
    }

    #[test]
    fn flip_keeps_guid() {
        let (mut m, lvl, t) = sample();
        let id = m.add_wall(spec(lvl, t)).unwrap();
        let guid = m.wall(id).unwrap().guid.clone();
        m.flip_wall(id).unwrap();
        let w = m.wall(id).unwrap();
        assert_eq!(w.guid, guid);
        assert_eq!(w.start, Point2::new(4.0, 0.0));
    }
}
