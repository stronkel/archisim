//! Building elements and the [`Model`] that holds them.
//!
//! Naming follows Revit's organisation (types and instances) while the
//! defaults and vocabulary of the tools follow ArchiCAD. Each struct notes its
//! IFC equivalent.

use std::collections::BTreeMap;
use std::fmt;

use crate::ids::*;
use crate::math::{is_simple_polygon, signed_area, Point2, EPSILON};

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

/// What a slab is for. Revit "Function" / IFC `IfcSlabTypeEnum`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SlabFunction {
    /// Floor between storeys ("forjado de planta"). IFC `FLOOR`.
    Floor,
    /// Ground-bearing slab ("solera"). IFC `BASESLAB`.
    BaseSlab,
    /// Flat roof ("cubierta plana"). IFC `ROOF`.
    Roof,
}

impl SlabFunction {
    /// `IfcSlabTypeEnum` value.
    pub fn ifc_predefined_type(self) -> &'static str {
        match self {
            SlabFunction::Floor => "FLOOR",
            SlabFunction::BaseSlab => "BASESLAB",
            SlabFunction::Roof => "ROOF",
        }
    }

    /// Whether the slab separates inside from outside (or ground), which
    /// is what `Pset_SlabCommon.IsExternal` means.
    pub fn is_external(self) -> bool {
        !matches!(self, SlabFunction::Floor)
    }
}

/// A slab type. IFC: `IfcSlabType` + `IfcMaterialLayerSet`.
#[derive(Debug, Clone, PartialEq)]
pub struct SlabType {
    pub id: SlabTypeId,
    pub guid: GlobalId,
    pub name: String,
    pub function: SlabFunction,
    /// Layers ordered from the top face down to the soffit.
    pub layers: Vec<Layer>,
}

impl SlabType {
    pub fn thickness(&self) -> f64 {
        self.layers.iter().map(|l| l.thickness).sum()
    }

    pub fn is_load_bearing(&self) -> bool {
        self.layers
            .iter()
            .any(|l| l.function == LayerFunction::Structure)
    }
}

/// A slab drawn by its outline. IFC: `IfcSlab` with an
/// `IfcMaterialLayerSetUsage`.
///
/// As in ArchiCAD and Revit, the reference is the **top face**: it sits at
/// the level elevation plus `top_offset`, and the layers hang downwards.
#[derive(Debug, Clone, PartialEq)]
pub struct Slab {
    pub id: SlabId,
    pub guid: GlobalId,
    pub slab_type: SlabTypeId,
    pub level: LevelId,
    /// Outline in plan, counter-clockwise, without repeating the first point.
    pub outline: Vec<Point2>,
    /// Height of the top face above the level, in metres (may be negative).
    pub top_offset: f64,
}

impl Slab {
    /// Plan area of the outline (gross area).
    pub fn area(&self) -> f64 {
        signed_area(&self.outline).abs()
    }
}

/// Data needed to create a slab; ids and GUIDs are assigned by the model.
#[derive(Debug, Clone, PartialEq)]
pub struct NewSlab {
    pub slab_type: SlabTypeId,
    pub level: LevelId,
    /// Outline in plan, in either direction; it is stored counter-clockwise.
    pub outline: Vec<Point2>,
    pub top_offset: f64,
}

/// How a door opens. IFC: `IfcDoorTypeOperationEnum`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DoorOperation {
    /// One hinged leaf ("abatible de una hoja").
    SingleSwing,
    /// Two hinged leaves meeting in the middle ("abatible de dos hojas").
    DoubleSwing,
    /// One leaf sliding along the face of the wall ("corredera").
    Sliding,
}

/// How a window opens. IFC: `IfcWindowTypePartitioningEnum` plus the
/// operation of its panels.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WindowOperation {
    /// A single fixed pane ("fija").
    Fixed,
    /// One side-hung leaf ("practicable de una hoja").
    SingleCasement,
    /// Two side-hung leaves ("practicable de dos hojas").
    DoubleCasement,
    /// Two horizontally sliding leaves ("corredera de dos hojas").
    Sliding,
}

impl WindowOperation {
    /// Number of glazed panels across the width.
    pub fn panels(self) -> usize {
        match self {
            WindowOperation::Fixed | WindowOperation::SingleCasement => 1,
            WindowOperation::DoubleCasement | WindowOperation::Sliding => 2,
        }
    }
}

/// Whether an opening type is a door or a window.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OpeningKind {
    Door(DoorOperation),
    Window(WindowOperation),
}

/// A door or window type: a Revit door/window family type, an ArchiCAD
/// door/window object with its settings. IFC: `IfcDoorType` or
/// `IfcWindowType`.
///
/// `width` and `height` are the rough opening ("hueco de obra"): the size of
/// the hole in the wall, which the frame fills completely.
#[derive(Debug, Clone, PartialEq)]
pub struct OpeningType {
    pub id: OpeningTypeId,
    pub guid: GlobalId,
    pub name: String,
    pub kind: OpeningKind,
    /// Rough opening width, in metres.
    pub width: f64,
    /// Rough opening height, in metres.
    pub height: f64,
    /// Visible width of the frame profile (jambs and head), in metres.
    pub frame_width: f64,
    /// Depth of the frame across the wall, in metres.
    pub frame_depth: f64,
    /// Material of the frame ("carpintería", "cerco").
    pub frame_material: MaterialId,
    /// Material of the infill: the door leaf or the glazing.
    pub panel_material: MaterialId,
}

impl OpeningType {
    pub fn is_door(&self) -> bool {
        matches!(self.kind, OpeningKind::Door(_))
    }

    pub fn is_window(&self) -> bool {
        matches!(self.kind, OpeningKind::Window(_))
    }
}

/// Data needed to create an opening type.
#[derive(Debug, Clone, PartialEq)]
pub struct NewOpeningType {
    pub name: String,
    pub kind: OpeningKind,
    pub width: f64,
    pub height: f64,
    pub frame_width: f64,
    pub frame_depth: f64,
    pub frame_material: MaterialId,
    pub panel_material: MaterialId,
}

/// One of the two faces of a wall.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WallSide {
    Exterior,
    Interior,
}

/// Handing of a door, following UNE-EN 12519: standing on the side the leaf
/// opens towards (where the hinges are seen), the hinges are on the left or
/// on the right. For a sliding door, the side the leaf slides to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Hand {
    Left,
    Right,
}

/// A door or window placed in a wall. IFC: an `IfcOpeningElement` that
/// voids the wall, filled by an `IfcDoor` or `IfcWindow`.
///
/// The position is measured along the wall's drawn line, from its start to
/// the centre of the opening (Revit places hosted elements by their centre).
/// The sill height is measured from the level, as Revit's "Sill Height".
#[derive(Debug, Clone, PartialEq)]
pub struct Opening {
    pub id: OpeningId,
    pub guid: GlobalId,
    pub opening_type: OpeningTypeId,
    /// The host wall. The opening is on the wall's level.
    pub wall: WallId,
    /// Distance from the wall start to the centre of the opening, in metres.
    pub along: f64,
    /// Height of the bottom of the opening above the level, in metres.
    pub sill: f64,
    /// Side the leaves open towards (Revit "facing"). Usually the interior.
    pub opens_to: WallSide,
    pub hand: Hand,
    /// Distance from the wall's exterior face to the exterior face of the
    /// frame ("retranqueo"). `None` centres the frame in the wall.
    pub setback: Option<f64>,
}

impl Opening {
    /// Start and end of the opening along the wall's drawn line.
    pub fn span(&self, width: f64) -> (f64, f64) {
        (self.along - width / 2.0, self.along + width / 2.0)
    }

    /// Distance from the wall's exterior face to the frame's exterior face.
    pub fn frame_setback(&self, wall_thickness: f64, frame_depth: f64) -> f64 {
        self.setback.unwrap_or((wall_thickness - frame_depth) / 2.0)
    }
}

/// Data needed to place a door or window.
#[derive(Debug, Clone, PartialEq)]
pub struct NewOpening {
    pub opening_type: OpeningTypeId,
    pub wall: WallId,
    pub along: f64,
    pub sill: f64,
    pub opens_to: WallSide,
    pub hand: Hand,
    pub setback: Option<f64>,
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
    UnknownSlabType(SlabTypeId),
    UnknownSlab(SlabId),
    UnknownOpeningType(OpeningTypeId),
    UnknownOpening(OpeningId),
    /// A door or window does not fit inside its wall (along the wall or
    /// between its base and top).
    OpeningOutsideWall,
    /// Two doors or windows in the same wall overlap.
    OverlappingOpenings,
    /// The frame of a door or window is deeper than the wall allows.
    FrameTooDeep,
    /// A slab outline needs at least three points, a non-zero area and no
    /// crossing or touching edges.
    InvalidOutline,
    /// The wall's start and end are the same point.
    ZeroLengthWall,
    /// A dimension that must be positive is not.
    NonPositive(&'static str),
    /// A wall or slab type must have at least one layer.
    EmptyType,
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
            ModelError::UnknownSlabType(id) => write!(f, "slab type {id} does not exist"),
            ModelError::UnknownSlab(id) => write!(f, "slab {id} does not exist"),
            ModelError::UnknownOpeningType(id) => {
                write!(f, "door or window type {id} does not exist")
            }
            ModelError::UnknownOpening(id) => write!(f, "door or window {id} does not exist"),
            ModelError::OpeningOutsideWall => {
                write!(f, "the door or window does not fit inside its wall")
            }
            ModelError::OverlappingOpenings => {
                write!(
                    f,
                    "the door or window overlaps another one in the same wall"
                )
            }
            ModelError::FrameTooDeep => {
                write!(f, "the frame does not fit in the wall thickness")
            }
            ModelError::InvalidOutline => write!(
                f,
                "a slab outline needs at least three points and must not cross itself"
            ),
            ModelError::ZeroLengthWall => write!(f, "a wall needs two different points"),
            ModelError::NonPositive(what) => write!(f, "{what} must be greater than zero"),
            ModelError::EmptyType => write!(f, "a type needs at least one layer"),
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
    slab_types: BTreeMap<SlabTypeId, SlabType>,
    slabs: BTreeMap<SlabId, Slab>,
    opening_types: BTreeMap<OpeningTypeId, OpeningType>,
    openings: BTreeMap<OpeningId, Opening>,
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
            slab_types: BTreeMap::new(),
            slabs: BTreeMap::new(),
            opening_types: BTreeMap::new(),
            openings: BTreeMap::new(),
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
            return Err(ModelError::EmptyType);
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

    /// Moves a wall's end points, keeping its identity and GUID. Its doors
    /// and windows keep their distance from the wall start; the move is
    /// rejected if any of them would no longer fit.
    pub fn move_wall(&mut self, id: WallId, start: Point2, end: Point2) -> Result<(), ModelError> {
        if start.approx_eq(end) {
            return Err(ModelError::ZeroLengthWall);
        }
        let wall = self.walls.get_mut(&id).ok_or(ModelError::UnknownWall(id))?;
        let previous = (wall.start, wall.end);
        wall.start = start;
        wall.end = end;
        if let Err(e) = self.check_hosted_openings(id) {
            let wall = self.walls.get_mut(&id).ok_or(ModelError::UnknownWall(id))?;
            (wall.start, wall.end) = previous;
            return Err(e);
        }
        Ok(())
    }

    /// Swaps start and end, which moves the exterior face to the other side
    /// (ArchiCAD "Invertir"). The drawn line stays where it is, and so do its
    /// doors and windows: their position is measured from the new start.
    pub fn flip_wall(&mut self, id: WallId) -> Result<(), ModelError> {
        let wall = self.walls.get_mut(&id).ok_or(ModelError::UnknownWall(id))?;
        std::mem::swap(&mut wall.start, &mut wall.end);
        let length = wall.length();
        for opening in self.openings.values_mut().filter(|o| o.wall == id) {
            opening.along = length - opening.along;
        }
        Ok(())
    }

    /// Removes a wall together with the doors and windows it hosts, as Revit
    /// and ArchiCAD do.
    pub fn remove_wall(&mut self, id: WallId) -> Result<Wall, ModelError> {
        let wall = self.walls.remove(&id).ok_or(ModelError::UnknownWall(id))?;
        self.openings.retain(|_, o| o.wall != id);
        Ok(wall)
    }

    /// Checks that every door and window in a wall still fits.
    fn check_hosted_openings(&self, wall: WallId) -> Result<(), ModelError> {
        for o in self.openings.values().filter(|o| o.wall == wall) {
            let spec = NewOpening {
                opening_type: o.opening_type,
                wall: o.wall,
                along: o.along,
                sill: o.sill,
                opens_to: o.opens_to,
                hand: o.hand,
                setback: o.setback,
            };
            self.validate_opening(&spec, Some(o.id))?;
        }
        Ok(())
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

    // ---- Slab types ---------------------------------------------------

    pub fn add_slab_type(
        &mut self,
        name: impl Into<String>,
        function: SlabFunction,
        layers: Vec<Layer>,
    ) -> Result<SlabTypeId, ModelError> {
        self.validate_layers(&layers)?;
        let id = SlabTypeId(self.take_id());
        self.slab_types.insert(
            id,
            SlabType {
                id,
                guid: GlobalId::new(),
                name: name.into(),
                function,
                layers,
            },
        );
        Ok(id)
    }

    pub fn slab_type(&self, id: SlabTypeId) -> Option<&SlabType> {
        self.slab_types.get(&id)
    }

    pub fn slab_types(&self) -> impl Iterator<Item = &SlabType> {
        self.slab_types.values()
    }

    pub fn remove_slab_type(&mut self, id: SlabTypeId) -> Result<SlabType, ModelError> {
        let users = self.slabs.values().filter(|s| s.slab_type == id).count();
        if users > 0 {
            return Err(ModelError::InUse(format!("{users} slab(s) use this type")));
        }
        self.slab_types
            .remove(&id)
            .ok_or(ModelError::UnknownSlabType(id))
    }

    // ---- Slabs --------------------------------------------------------

    /// Checks an outline and returns it counter-clockwise.
    fn normalized_outline(outline: Vec<Point2>) -> Result<Vec<Point2>, ModelError> {
        if !is_simple_polygon(&outline) || signed_area(&outline).abs() <= EPSILON {
            return Err(ModelError::InvalidOutline);
        }
        let mut outline = outline;
        if signed_area(&outline) < 0.0 {
            outline.reverse();
        }
        Ok(outline)
    }

    pub fn add_slab(&mut self, spec: NewSlab) -> Result<SlabId, ModelError> {
        if !self.levels.contains_key(&spec.level) {
            return Err(ModelError::UnknownLevel(spec.level));
        }
        if !self.slab_types.contains_key(&spec.slab_type) {
            return Err(ModelError::UnknownSlabType(spec.slab_type));
        }
        let outline = Self::normalized_outline(spec.outline)?;
        let id = SlabId(self.take_id());
        self.slabs.insert(
            id,
            Slab {
                id,
                guid: GlobalId::new(),
                slab_type: spec.slab_type,
                level: spec.level,
                outline,
                top_offset: spec.top_offset,
            },
        );
        Ok(id)
    }

    /// Replaces a slab's outline, keeping its identity and GUID.
    pub fn set_slab_outline(&mut self, id: SlabId, outline: Vec<Point2>) -> Result<(), ModelError> {
        let outline = Self::normalized_outline(outline)?;
        let slab = self.slabs.get_mut(&id).ok_or(ModelError::UnknownSlab(id))?;
        slab.outline = outline;
        Ok(())
    }

    pub fn remove_slab(&mut self, id: SlabId) -> Result<Slab, ModelError> {
        self.slabs.remove(&id).ok_or(ModelError::UnknownSlab(id))
    }

    pub fn slab(&self, id: SlabId) -> Option<&Slab> {
        self.slabs.get(&id)
    }

    pub fn slabs(&self) -> impl Iterator<Item = &Slab> {
        self.slabs.values()
    }

    pub fn slabs_on_level(&self, level: LevelId) -> impl Iterator<Item = &Slab> {
        self.slabs.values().filter(move |s| s.level == level)
    }

    // ---- Door and window types -----------------------------------------

    pub fn add_opening_type(&mut self, spec: NewOpeningType) -> Result<OpeningTypeId, ModelError> {
        for m in [spec.frame_material, spec.panel_material] {
            if !self.materials.contains_key(&m) {
                return Err(ModelError::UnknownMaterial(m));
            }
        }
        for (value, what) in [
            (spec.width, "opening width"),
            (spec.height, "opening height"),
            (spec.frame_width, "frame width"),
            (spec.frame_depth, "frame depth"),
        ] {
            if value <= EPSILON {
                return Err(ModelError::NonPositive(what));
            }
        }
        // The frame must leave room for the leaf or the glass.
        if 2.0 * spec.frame_width >= spec.width || 2.0 * spec.frame_width >= spec.height {
            return Err(ModelError::NonPositive("clear opening inside the frame"));
        }
        let id = OpeningTypeId(self.take_id());
        self.opening_types.insert(
            id,
            OpeningType {
                id,
                guid: GlobalId::new(),
                name: spec.name,
                kind: spec.kind,
                width: spec.width,
                height: spec.height,
                frame_width: spec.frame_width,
                frame_depth: spec.frame_depth,
                frame_material: spec.frame_material,
                panel_material: spec.panel_material,
            },
        );
        Ok(id)
    }

    pub fn opening_type(&self, id: OpeningTypeId) -> Option<&OpeningType> {
        self.opening_types.get(&id)
    }

    pub fn opening_types(&self) -> impl Iterator<Item = &OpeningType> {
        self.opening_types.values()
    }

    pub fn remove_opening_type(&mut self, id: OpeningTypeId) -> Result<OpeningType, ModelError> {
        let users = self
            .openings
            .values()
            .filter(|o| o.opening_type == id)
            .count();
        if users > 0 {
            return Err(ModelError::InUse(format!(
                "{users} door(s) or window(s) use this type"
            )));
        }
        self.opening_types
            .remove(&id)
            .ok_or(ModelError::UnknownOpeningType(id))
    }

    // ---- Doors and windows ----------------------------------------------

    /// Checks that a door or window fits in its wall: inside its length and
    /// height, with a frame no deeper than the wall, and clear of the other
    /// doors and windows in the same wall (except `ignore`, when moving one).
    fn validate_opening(
        &self,
        spec: &NewOpening,
        ignore: Option<OpeningId>,
    ) -> Result<(), ModelError> {
        let otype = self
            .opening_types
            .get(&spec.opening_type)
            .ok_or(ModelError::UnknownOpeningType(spec.opening_type))?;
        let wall = self
            .walls
            .get(&spec.wall)
            .ok_or(ModelError::UnknownWall(spec.wall))?;
        let thickness = self
            .wall_types
            .get(&wall.wall_type)
            .ok_or(ModelError::UnknownWallType(wall.wall_type))?
            .thickness();

        let (a, b) = (
            spec.along - otype.width / 2.0,
            spec.along + otype.width / 2.0,
        );
        let (bottom, top) = (spec.sill, spec.sill + otype.height);
        let wall_top = wall.base_offset + wall.height;
        if a < -EPSILON
            || b > wall.length() + EPSILON
            || bottom < wall.base_offset - EPSILON
            || top > wall_top + EPSILON
        {
            return Err(ModelError::OpeningOutsideWall);
        }
        let setback = spec
            .setback
            .unwrap_or((thickness - otype.frame_depth) / 2.0);
        if setback < -EPSILON || setback + otype.frame_depth > thickness + EPSILON {
            return Err(ModelError::FrameTooDeep);
        }

        for other in self.openings.values() {
            if other.wall != spec.wall || Some(other.id) == ignore {
                continue;
            }
            let Some(ot) = self.opening_types.get(&other.opening_type) else {
                continue;
            };
            let (oa, ob) = other.span(ot.width);
            let (obot, otop) = (other.sill, other.sill + ot.height);
            let overlap_along = a.max(oa) < b.min(ob) - EPSILON;
            let overlap_height = bottom.max(obot) < top.min(otop) - EPSILON;
            if overlap_along && overlap_height {
                return Err(ModelError::OverlappingOpenings);
            }
        }
        Ok(())
    }

    /// Places a door or window in a wall.
    pub fn add_opening(&mut self, spec: NewOpening) -> Result<OpeningId, ModelError> {
        self.validate_opening(&spec, None)?;
        let id = OpeningId(self.take_id());
        self.openings.insert(
            id,
            Opening {
                id,
                guid: GlobalId::new(),
                opening_type: spec.opening_type,
                wall: spec.wall,
                along: spec.along,
                sill: spec.sill,
                opens_to: spec.opens_to,
                hand: spec.hand,
                setback: spec.setback,
            },
        );
        Ok(id)
    }

    /// Moves a door or window along its wall and changes its sill height,
    /// keeping its identity and GUID.
    pub fn move_opening(&mut self, id: OpeningId, along: f64, sill: f64) -> Result<(), ModelError> {
        let o = self
            .openings
            .get(&id)
            .ok_or(ModelError::UnknownOpening(id))?;
        let spec = NewOpening {
            opening_type: o.opening_type,
            wall: o.wall,
            along,
            sill,
            opens_to: o.opens_to,
            hand: o.hand,
            setback: o.setback,
        };
        self.validate_opening(&spec, Some(id))?;
        let o = self
            .openings
            .get_mut(&id)
            .ok_or(ModelError::UnknownOpening(id))?;
        o.along = along;
        o.sill = sill;
        Ok(())
    }

    /// Changes which way a door or window opens (Revit's "flip facing" and
    /// "flip hand" arrows).
    pub fn set_opening_handing(
        &mut self,
        id: OpeningId,
        opens_to: WallSide,
        hand: Hand,
    ) -> Result<(), ModelError> {
        let o = self
            .openings
            .get_mut(&id)
            .ok_or(ModelError::UnknownOpening(id))?;
        o.opens_to = opens_to;
        o.hand = hand;
        Ok(())
    }

    pub fn remove_opening(&mut self, id: OpeningId) -> Result<Opening, ModelError> {
        self.openings
            .remove(&id)
            .ok_or(ModelError::UnknownOpening(id))
    }

    pub fn opening(&self, id: OpeningId) -> Option<&Opening> {
        self.openings.get(&id)
    }

    pub fn openings(&self) -> impl Iterator<Item = &Opening> {
        self.openings.values()
    }

    /// Doors and windows hosted by a wall.
    pub fn openings_in_wall(&self, wall: WallId) -> impl Iterator<Item = &Opening> {
        self.openings.values().filter(move |o| o.wall == wall)
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

    fn slab_type(m: &mut Model) -> SlabTypeId {
        let concrete = m.add_material("Hormigón", "concrete", [200, 200, 200], CutPattern::Cross);
        let tile = m.add_material("Baldosa", "ceramic", [220, 220, 210], CutPattern::Solid);
        m.add_slab_type(
            "Forjado 30",
            SlabFunction::Floor,
            vec![
                Layer {
                    material: tile,
                    thickness: 0.02,
                    function: LayerFunction::Finish,
                },
                Layer {
                    material: concrete,
                    thickness: 0.3,
                    function: LayerFunction::Structure,
                },
            ],
        )
        .unwrap()
    }

    #[test]
    fn slab_outline_is_stored_counter_clockwise() {
        let (mut m, lvl, _) = sample();
        let st = slab_type(&mut m);
        assert!((m.slab_type(st).unwrap().thickness() - 0.32).abs() < 1e-12);
        // Drawn clockwise.
        let id = m
            .add_slab(NewSlab {
                slab_type: st,
                level: lvl,
                outline: vec![
                    Point2::new(0.0, 0.0),
                    Point2::new(0.0, 5.0),
                    Point2::new(4.0, 5.0),
                    Point2::new(4.0, 0.0),
                ],
                top_offset: 0.0,
            })
            .unwrap();
        let s = m.slab(id).unwrap();
        assert!(signed_area(&s.outline) > 0.0);
        assert_eq!(s.area(), 20.0);
        assert_eq!(m.slabs_on_level(lvl).count(), 1);
    }

    #[test]
    fn rejects_invalid_slabs() {
        let (mut m, lvl, _) = sample();
        let st = slab_type(&mut m);
        let spec = |outline: Vec<Point2>| NewSlab {
            slab_type: st,
            level: lvl,
            outline,
            top_offset: 0.0,
        };
        let bow = vec![
            Point2::new(0.0, 0.0),
            Point2::new(2.0, 2.0),
            Point2::new(2.0, 0.0),
            Point2::new(0.0, 2.0),
        ];
        assert_eq!(m.add_slab(spec(bow)), Err(ModelError::InvalidOutline));
        let line = vec![
            Point2::new(0.0, 0.0),
            Point2::new(1.0, 0.0),
            Point2::new(2.0, 0.0),
        ];
        assert_eq!(m.add_slab(spec(line)), Err(ModelError::InvalidOutline));
        let mut s = spec(vec![
            Point2::new(0.0, 0.0),
            Point2::new(1.0, 0.0),
            Point2::new(1.0, 1.0),
        ]);
        s.slab_type = SlabTypeId(999);
        assert!(matches!(m.add_slab(s), Err(ModelError::UnknownSlabType(_))));
        assert_eq!(
            m.add_slab_type("Vacío", SlabFunction::Roof, vec![]),
            Err(ModelError::EmptyType)
        );
    }

    #[test]
    fn slab_edits_keep_guid_and_protect_types() {
        let (mut m, lvl, _) = sample();
        let st = slab_type(&mut m);
        let id = m
            .add_slab(NewSlab {
                slab_type: st,
                level: lvl,
                outline: vec![
                    Point2::new(0.0, 0.0),
                    Point2::new(4.0, 0.0),
                    Point2::new(4.0, 4.0),
                ],
                top_offset: 0.0,
            })
            .unwrap();
        let guid = m.slab(id).unwrap().guid.clone();
        m.set_slab_outline(
            id,
            vec![
                Point2::new(0.0, 0.0),
                Point2::new(6.0, 0.0),
                Point2::new(6.0, 6.0),
                Point2::new(0.0, 6.0),
            ],
        )
        .unwrap();
        assert_eq!(m.slab(id).unwrap().guid, guid);
        assert_eq!(m.slab(id).unwrap().area(), 36.0);
        assert!(matches!(m.remove_slab_type(st), Err(ModelError::InUse(_))));
        m.remove_slab(id).unwrap();
        assert!(m.remove_slab_type(st).is_ok());
    }

    /// A 4 m long, 0.1 m thick, 2.7 m high wall with a 0.8 × 2.05 door type
    /// and a 1.2 × 1.2 window type.
    fn with_openings() -> (Model, WallId, OpeningTypeId, OpeningTypeId) {
        let (mut m, lvl, t) = sample();
        let wall = m.add_wall(spec(lvl, t)).unwrap();
        let wood = m.add_material("Madera", "wood", [180, 140, 90], CutPattern::Solid);
        let glass = m.add_material("Vidrio", "glass", [200, 230, 240], CutPattern::Solid);
        let door = m
            .add_opening_type(NewOpeningType {
                name: "P 80".into(),
                kind: OpeningKind::Door(DoorOperation::SingleSwing),
                width: 0.8,
                height: 2.05,
                frame_width: 0.05,
                frame_depth: 0.07,
                frame_material: wood,
                panel_material: wood,
            })
            .unwrap();
        let window = m
            .add_opening_type(NewOpeningType {
                name: "V 120".into(),
                kind: OpeningKind::Window(WindowOperation::DoubleCasement),
                width: 1.2,
                height: 1.2,
                frame_width: 0.06,
                frame_depth: 0.07,
                frame_material: wood,
                panel_material: glass,
            })
            .unwrap();
        (m, wall, door, window)
    }

    fn place(opening_type: OpeningTypeId, wall: WallId, along: f64, sill: f64) -> NewOpening {
        NewOpening {
            opening_type,
            wall,
            along,
            sill,
            opens_to: WallSide::Interior,
            hand: Hand::Left,
            setback: None,
        }
    }

    #[test]
    fn openings_must_fit_in_their_wall() {
        let (mut m, wall, door, window) = with_openings();
        // Past either end of the 4 m wall.
        assert_eq!(
            m.add_opening(place(door, wall, 0.3, 0.0)),
            Err(ModelError::OpeningOutsideWall)
        );
        assert_eq!(
            m.add_opening(place(door, wall, 3.7, 0.0)),
            Err(ModelError::OpeningOutsideWall)
        );
        // Above the 2.7 m wall top: 1.6 + 1.2 = 2.8.
        assert_eq!(
            m.add_opening(place(window, wall, 2.0, 1.6)),
            Err(ModelError::OpeningOutsideWall)
        );
        // Touching the ends exactly is fine.
        let d = m.add_opening(place(door, wall, 0.4, 0.0)).unwrap();
        // Overlapping the door.
        assert_eq!(
            m.add_opening(place(window, wall, 1.2, 0.9)),
            Err(ModelError::OverlappingOpenings)
        );
        // Side by side, sharing a jamb, is allowed.
        let w = m.add_opening(place(window, wall, 1.4, 0.9)).unwrap();
        assert_eq!(m.openings_in_wall(wall).count(), 2);
        // A frame deeper than the 0.1 m wall.
        let mut deep = place(window, wall, 3.0, 0.9);
        deep.setback = Some(0.05);
        assert_eq!(m.add_opening(deep), Err(ModelError::FrameTooDeep));
        // Moving keeps the GUID and is checked against the others.
        let guid = m.opening(w).unwrap().guid.clone();
        assert_eq!(
            m.move_opening(w, 1.0, 0.9),
            Err(ModelError::OverlappingOpenings)
        );
        m.move_opening(w, 3.0, 1.0).unwrap();
        assert_eq!(m.opening(w).unwrap().guid, guid);
        assert_eq!(m.opening(w).unwrap().along, 3.0);
        assert!(m.opening(d).is_some());
    }

    #[test]
    fn opening_types_are_checked_and_protected() {
        let (mut m, wall, door, _) = with_openings();
        let wood = m.opening_type(door).unwrap().frame_material;
        let bad = NewOpeningType {
            name: "Mal".into(),
            kind: OpeningKind::Door(DoorOperation::SingleSwing),
            width: 0.08,
            height: 2.0,
            frame_width: 0.05,
            frame_depth: 0.07,
            frame_material: wood,
            panel_material: wood,
        };
        assert!(matches!(
            m.add_opening_type(bad),
            Err(ModelError::NonPositive(_))
        ));
        let o = m.add_opening(place(door, wall, 2.0, 0.0)).unwrap();
        assert!(matches!(
            m.remove_opening_type(door),
            Err(ModelError::InUse(_))
        ));
        m.remove_opening(o).unwrap();
        assert!(m.remove_opening_type(door).is_ok());
    }

    #[test]
    fn walls_carry_their_openings() {
        let (mut m, wall, door, _) = with_openings();
        let o = m.add_opening(place(door, wall, 1.0, 0.0)).unwrap();
        // Flipping keeps the door where it physically is.
        m.flip_wall(wall).unwrap();
        assert!((m.opening(o).unwrap().along - 3.0).abs() < 1e-12);
        // Shortening the wall so the door falls off is rejected, untouched.
        assert_eq!(
            m.move_wall(wall, Point2::new(4.0, 0.0), Point2::new(2.0, 0.0)),
            Err(ModelError::OpeningOutsideWall)
        );
        assert_eq!(m.wall(wall).unwrap().end, Point2::new(0.0, 0.0));
        m.move_wall(wall, Point2::new(5.0, 0.0), Point2::new(0.0, 0.0))
            .unwrap();
        // Deleting the wall deletes its door.
        m.remove_wall(wall).unwrap();
        assert!(m.opening(o).is_none());
        assert_eq!(m.openings().count(), 0);
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
