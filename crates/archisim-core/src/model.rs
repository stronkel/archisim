//! Building elements and the [`Model`] that holds them.
//!
//! Naming follows Revit's organisation (types and instances) while the
//! defaults and vocabulary of the tools follow ArchiCAD. Each struct notes its
//! IFC equivalent.

use std::collections::BTreeMap;
use std::fmt;

use crate::drafting::{Draft, DrawLayer, ViewRef, LAYER_ZERO};
use crate::family::{
    self, Evaluation, Family, FamilyError, Mark, NotAnOpening, ParamKind, ParamScope, Value, Values,
};
use crate::ids::*;
use crate::math::{is_simple_polygon, signed_area, Arc, Point2, Vec2, EPSILON, MIN_BULGE};

mod appearance;
mod built_in;
mod concept;
mod curtain;
mod disciplines;
mod dormers;
mod flora;
mod foundations;
mod layered_library;
mod library;
mod meshes;
mod modules;
mod objects;
mod paint;
mod parameters;
mod phases;
mod plantings;
mod pools;
mod presentation;
mod prices;
mod props;
mod railings;
mod roofs;
mod sections;
mod services;
mod site;
mod stair_layout;
mod stairs;
mod structure;
mod sun;
mod sweeps;
mod urban;
mod views;
mod wall_profile;
mod worktops;
pub use appearance::*;
pub use concept::*;
pub use curtain::*;
pub use disciplines::*;
pub use dormers::*;
pub use flora::*;
pub use foundations::*;
pub use layered_library::*;
pub use library::*;
pub use meshes::*;
pub use modules::*;
pub use objects::*;
pub use paint::*;
pub use parameters::*;
pub use phases::*;
pub use plantings::*;
pub use pools::*;
pub use prices::*;
pub use railings::*;
pub use roofs::*;
pub use sections::*;
pub use services::*;
pub use site::*;
pub use stair_layout::*;
pub use stairs::*;
pub use structure::*;
pub use sun::*;
pub use sweeps::*;
pub use urban::*;
pub use views::*;
pub use wall_profile::*;
pub use worktops::*;

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
    /// Intersection priority of its layers, 0 to 999 (ArchiCAD's
    /// "prioridad de intersección"): where walls meet in a T, a layer goes
    /// through the other wall's layers of lower priority. `None`: by the
    /// layer's function (see [`LayerFunction::default_priority`]).
    pub priority: Option<u16>,
}

/// Highest intersection priority of a material.
pub const MAX_PRIORITY: u16 = 999;

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
    /// Intersection priority of a layer of this function when its material
    /// has none of its own: the structure goes through, finishes stop.
    pub fn default_priority(self) -> u16 {
        match self {
            LayerFunction::Finish => 100,
            LayerFunction::Membrane | LayerFunction::AirGap => 300,
            LayerFunction::Insulation => 500,
            LayerFunction::Substrate => 700,
            LayerFunction::Structure => 900,
        }
    }

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
    /// Skirting boards, bands, cornices and copings along its walls.
    pub sweeps: Vec<WallSweep>,
    /// Revit's «Envolver en extremos»: which finish turns round a free
    /// end and covers the ends of the other layers.
    pub wrap_ends: Wrap,
    /// Revit's «Envolver en huecos»: which finish turns into the reveals
    /// of doors and windows, up to the frame (jambs, head and sill).
    pub wrap_inserts: Wrap,
}

/// Which finish of a wall type wraps (Revit's «Envolver»): the exterior
/// one (first layer), the interior one (last layer), both or none.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Wrap {
    #[default]
    None,
    Exterior,
    Interior,
    Both,
}

impl Wrap {
    pub fn exterior(self) -> bool {
        matches!(self, Wrap::Exterior | Wrap::Both)
    }

    pub fn interior(self) -> bool {
        matches!(self, Wrap::Interior | Wrap::Both)
    }

    /// What wraps into the reveals of a type that does not say: the
    /// exterior render of a façade whose first layer is a finish (what
    /// ArchiSim drew before the option existed).
    pub fn inserts_by_default(function: WallFunction, layers: &[Layer]) -> Wrap {
        if function == WallFunction::Exterior
            && layers.len() > 1
            && layers[0].function == LayerFunction::Finish
        {
            Wrap::Exterior
        } else {
            Wrap::None
        }
    }
}

impl WallType {
    /// The layer that wraps on the exterior side and on the interior side
    /// for a given choice (none if the type has a single layer).
    pub fn wrapping_layers(&self, wrap: Wrap) -> (Option<usize>, Option<usize>) {
        let n = self.layers.len();
        if n < 2 {
            return (None, None);
        }
        (
            wrap.exterior().then_some(0),
            wrap.interior().then_some(n - 1),
        )
    }

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

/// A wall, straight or curved. IFC: `IfcWall` with an
/// `IfcMaterialLayerSetUsage`.
///
/// The exterior face is on the **left** of the axis when looking from `start`
/// to `end`. Drawing a building outline clockwise therefore puts the exterior
/// layers outside.
///
/// A curved wall's drawn line is a circular arc from `start` to `end`, given
/// by its `bulge` (see [`Arc`]); 0 is a straight wall.
#[derive(Debug, Clone, PartialEq)]
pub struct Wall {
    pub id: WallId,
    pub guid: GlobalId,
    pub wall_type: WallTypeId,
    pub level: LevelId,
    pub start: Point2,
    pub end: Point2,
    /// 0 for a straight wall; otherwise `tan(θ/4)` of the arc it follows
    /// (positive: turning left, counter-clockwise).
    pub bulge: f64,
    pub reference: ReferenceLine,
    /// Height above the base, in metres. With a top constraint it is
    /// worked out by the model from the levels and kept up to date.
    pub height: f64,
    /// Offset of the base above the level, in metres.
    pub base_offset: f64,
    /// Where the top of the wall is: a height of its own, or a level.
    pub top: WallTop,
    /// Whether each end (start, end) may join other walls: Revit's
    /// «Permitir unión» / «No permitir unión». A disallowed end stays a
    /// free end, square, even touching another wall, and neither wall
    /// mitres nor tees into the other there.
    pub allow_join: [bool; 2],
    /// Its outline in elevation, with holes (E3: «Editar perfil»); `None`:
    /// its rectangle. Straight walls only.
    pub profile: Option<WallProfile>,
}

/// The top constraint of a wall (Revit's "Restricción superior",
/// ArchiCAD's linked top storey).
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum WallTop {
    /// Its own height above the base (Revit's "Desconectada").
    #[default]
    Unconnected,
    /// Up to a level, plus an offset (negative: below the level). The
    /// wall follows when either level changes its elevation.
    Level { level: LevelId, offset: f64 },
}

impl Wall {
    /// Whether the end at `p` (its start or its end) may join other
    /// walls. A point that is neither end joins as before.
    pub fn joins_at(&self, p: Point2) -> bool {
        if self.start.approx_eq(p) {
            self.allow_join[0]
        } else if self.end.approx_eq(p) {
            self.allow_join[1]
        } else {
            true
        }
    }

    /// Length of the drawn line (along the arc for a curved wall).
    pub fn length(&self) -> f64 {
        match self.arc() {
            Some(a) => a.length(),
            None => self.start.distance(self.end),
        }
    }

    /// The arc of a curved wall, or `None` if it is straight.
    pub fn arc(&self) -> Option<Arc> {
        Arc::from_bulge(self.start, self.end, self.bulge)
    }

    pub fn is_curved(&self) -> bool {
        self.arc().is_some()
    }

    /// Where a door or window `width` wide centred `along` the drawn line
    /// stands: the point of its centre and the direction it runs. In a
    /// straight wall, the line itself; in a curved one, the tangent at its
    /// centre, moved towards the centre of the curve by half how far the
    /// curve parts from the tangent at its sides, so the straight frame
    /// stays inside the wall's thickness at both ends.
    pub fn opening_frame(&self, along: f64, width: f64) -> Option<(Point2, Vec2)> {
        match self.arc() {
            Some(arc) => {
                let f = (along / arc.length()).clamp(0.0, 1.0);
                let p = arc.point(f, 0.0);
                let u = arc.tangent(f);
                let (r, half) = (arc.radius, width / 2.0);
                let sag = if half < r {
                    r - (r * r - half * half).sqrt()
                } else {
                    0.0
                };
                let inward = (arc.centre - p).normalized().unwrap_or(Vec2::new(0.0, 0.0));
                Some((p + inward * (sag / 2.0), u))
            }
            None => {
                let u = (self.end - self.start).normalized()?;
                Some((self.start + u * along, u))
            }
        }
    }

    /// The point of the drawn line at fraction `f` (0 = start, 1 = end),
    /// moved `offset` to the left.
    pub fn point_at(&self, f: f64, offset: f64) -> Point2 {
        match self.arc() {
            Some(a) => a.point(f, offset),
            None => {
                let d = self.end - self.start;
                let n = d.normalized().map_or(Vec2::default(), |u| u.perp_left());
                self.start + d * f + n * offset
            }
        }
    }

    /// The middle of the drawn line.
    pub fn midpoint(&self) -> Point2 {
        self.point_at(0.5, 0.0)
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
    /// A cornice or an eave round its slabs.
    pub edge: Option<SlabEdge>,
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
    /// Holes through the slab (stairs, shafts), each a closed outline
    /// inside `outline`, counter-clockwise. IFC: the voids of the profile.
    pub holes: Vec<Vec<Point2>>,
    /// Inclined slabs (ramps, sloping floors): how the top face rises.
    /// `None`: level.
    pub slope: Option<SlabSlope>,
}

/// Slope of an inclined slab, as Revit's slope arrow: the top face is at
/// the slab's `top_offset` along the line through `from` square to the
/// arrow, and rises `slope` metres per metre (0.08 = 8 %) towards `to`
/// (falls if negative). The layers keep their thickness square to the
/// face; the edges of the slab stay vertical, so its plan is its outline.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SlabSlope {
    pub from: Point2,
    pub to: Point2,
    pub slope: f64,
}

/// Steepest slope a slab may have (metres per metre; 3 ≈ 72°).
pub const MAX_SLAB_SLOPE: f64 = 3.0;

impl SlabSlope {
    /// Unit vector of the arrow, in plan.
    pub fn direction(&self) -> Option<Vec2> {
        (self.to - self.from).normalized()
    }

    /// How much higher the top face is at `p` than at `from`.
    pub fn rise_at(&self, p: Point2) -> f64 {
        self.direction()
            .map_or(0.0, |u| (p - self.from).dot(u) * self.slope)
    }

    /// Vertical height of a layer of `thickness` measured square to the
    /// sloping face.
    pub fn vertical(&self, thickness: f64) -> f64 {
        thickness * (1.0 + self.slope * self.slope).sqrt()
    }

    fn validate(&self) -> Result<(), ModelError> {
        finite_point(self.from, "slab slope start")?;
        finite_point(self.to, "slab slope end")?;
        finite(self.slope, "slab slope")?;
        if self.from.approx_eq(self.to) || self.slope.abs() > MAX_SLAB_SLOPE {
            return Err(ModelError::InvalidSlope);
        }
        Ok(())
    }
}

impl Slab {
    /// Height of the top face above the level at `p` (its `top_offset`
    /// if it is level).
    pub fn top_at(&self, p: Point2) -> f64 {
        self.top_offset + self.slope.map_or(0.0, |s| s.rise_at(p))
    }

    /// Lowest and highest height of the top face above the level.
    pub fn top_range(&self) -> (f64, f64) {
        self.outline
            .iter()
            .fold((f64::INFINITY, f64::NEG_INFINITY), |(lo, hi), p| {
                let z = self.top_at(*p);
                (lo.min(z), hi.max(z))
            })
    }

    /// Vertical height of a layer at this slab (more than its thickness
    /// if it slopes).
    pub fn vertical(&self, thickness: f64) -> f64 {
        self.slope.map_or(thickness, |s| s.vertical(thickness))
    }

    /// Plan area of the outline less its holes (net area).
    pub fn area(&self) -> f64 {
        signed_area(&self.outline).abs()
            - self.holes.iter().map(|h| signed_area(h).abs()).sum::<f64>()
    }

    /// Whether a point is on the slab: inside its outline and not in a hole.
    pub fn covers(&self, p: Point2) -> bool {
        polygon_contains(&self.outline, p) && !self.holes.iter().any(|h| polygon_contains(h, p))
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
    /// Leaves hinged to each other, folding to one side ("plegable").
    Folding,
    /// One leaf turning on a vertical axis away from the jamb ("pivotante").
    Pivot,
    /// A garage door that goes up under the ceiling ("seccional",
    /// "basculante").
    Overhead,
    /// A garage door or shutter rolled up in a drum ("enrollable").
    Rolling,
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
    /// A rough opening with nothing in it ("hueco", Revit's wall opening).
    Void,
}

/// A door or window type: a type of a family ("Puerta abatible") with its
/// type parameters set, like a Revit family type. IFC: `IfcDoorType` or
/// `IfcWindowType`.
///
/// The family and its `values` are the definition. The fields from `kind`
/// on are what the family gives the rest of the program (its outputs),
/// worked out by the model whenever the type changes: never set them by
/// hand. `width` and `height` are the rough opening ("hueco de obra"): the
/// size of the hole in the wall, which the frame fills completely.
#[derive(Debug, Clone, PartialEq)]
pub struct OpeningType {
    pub id: OpeningTypeId,
    pub guid: GlobalId,
    pub name: String,
    /// Type mark for tags and schedules ("P-03", "V-01"); unique, or empty.
    pub mark: String,
    pub description: String,
    pub family: FamilyId,
    /// Value of every type parameter of the family.
    pub values: Values,
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
    /// For windows, how each leaf opens, as IFC names it
    /// (`IfcWindowPanelOperationEnum`: `SIDEHUNGLEFTHAND`, `FIXEDCASEMENT`…),
    /// from left to right. Empty for doors.
    pub panels: Vec<String>,
    /// The shape of the rough opening in elevation: a square head, an arch
    /// or an oculus (its recipe's `opening_shape` and `opening_rise`).
    pub shape: archisim_family::OpeningShape,
    /// Where its frame sits in the wall's thickness when a placed one does
    /// not say (its recipe's `frame_align` and `frame_inset`).
    pub frame_place: FramePlace,
}

/// Where a door's or window's frame sits across its wall, by default.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum FramePlace {
    /// In the middle of the wall.
    #[default]
    Centre,
    /// Its exterior face this far in from the wall's exterior face.
    Exterior(f64),
    /// Its interior face this far in from the wall's interior face.
    Interior(f64),
}

impl OpeningType {
    pub fn is_door(&self) -> bool {
        matches!(self.kind, OpeningKind::Door(_))
    }

    pub fn is_window(&self) -> bool {
        matches!(self.kind, OpeningKind::Window(_))
    }

    /// A rough opening with no door or window in it.
    pub fn is_void(&self) -> bool {
        self.kind == OpeningKind::Void
    }
}

/// Data needed to create a type of a door or window family.
#[derive(Debug, Clone, PartialEq)]
pub struct NewFamilyType {
    pub family: FamilyId,
    pub name: String,
    /// `None` gives the next free mark ("P-04", "V-02"…).
    pub mark: Option<String>,
    pub description: String,
    /// Value of every type parameter of the family.
    pub values: Values,
}

/// Data needed to create an opening type the way ArchiSim did before
/// families: by its operation and sizes. It becomes a type of the matching
/// system family ("Puerta abatible", "Ventana corredera"…).
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
    /// Value of every instance parameter of the type's family (none for
    /// the system families: placement, side and hand are fields above).
    pub values: Values,
}

impl Opening {
    /// Start and end of the opening along the wall's drawn line.
    pub fn span(&self, width: f64) -> (f64, f64) {
        (self.along - width / 2.0, self.along + width / 2.0)
    }

    /// Distance from the wall's exterior face to the frame's exterior face:
    /// its own, or where its type puts it, kept inside the wall.
    pub fn frame_setback(&self, wall_thickness: f64, t: &OpeningType) -> f64 {
        let fd = t.frame_depth;
        self.setback.unwrap_or(match t.frame_place {
            FramePlace::Centre => (wall_thickness - fd) / 2.0,
            FramePlace::Exterior(d) => d.clamp(0.0, (wall_thickness - fd).max(0.0)),
            FramePlace::Interior(d) => {
                (wall_thickness - fd - d).clamp(0.0, (wall_thickness - fd).max(0.0))
            }
        })
    }

    /// For a corner window (its family's instance parameter `corner`:
    /// «izquierda» or «derecha», seen from the side it opens to), which end
    /// of its wall it reaches: `Some(true)` the end, `Some(false)` the
    /// start. There it has no jamb, and the wall is opened through the
    /// corner, so it meets the window of the other wall.
    pub fn corner_end(&self) -> Option<bool> {
        let right = match self.values.get("corner").and_then(Value::as_text)? {
            "derecha" => true,
            "izquierda" => false,
            _ => return None,
        };
        // Family x runs with the wall when the leaves open to the interior
        // and against it when they open to the exterior.
        Some(match self.opens_to {
            WallSide::Interior => right,
            WallSide::Exterior => !right,
        })
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

/// A linear dimension in plan ("cota"), single or chained: the distances
/// between consecutive points measured along `direction`, drawn on a line
/// parallel to it through `position`.
///
/// Dimensions are annotations of a level's plan, not building elements:
/// they have no `GlobalId` and are not exported to IFC (Revit and ArchiCAD
/// do not export them either).
#[derive(Debug, Clone, PartialEq)]
pub struct Dimension {
    pub id: DimensionId,
    pub level: LevelId,
    /// The measured points (two or more), in any order.
    pub points: Vec<Point2>,
    /// Unit vector of the measuring direction.
    pub direction: Point2,
    /// A point the dimension line passes through.
    pub position: Point2,
    /// What each point is tied to (empty, or one per point): the
    /// dimension follows walls, doors and windows and slabs when they
    /// change, as Revit's and ArchiCAD's associative dimensions.
    pub anchors: Vec<Option<Anchor>>,
}

/// Which line of a wall or opening a dimension point sits on.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AnchorSide {
    Exterior,
    /// The drawn line (reference line) of the wall.
    Axis,
    Interior,
}

/// A point of an element a dimension point is tied to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Anchor {
    /// The corner of a wall end on a face, or the end of its drawn line.
    WallEnd {
        wall: WallId,
        start: bool,
        side: AnchorSide,
    },
    /// A jamb of a door or window on a face of its wall (`end`: the jamb
    /// farther from the wall's start).
    Jamb {
        opening: OpeningId,
        end: bool,
        side: AnchorSide,
    },
    /// A corner of a slab's outline.
    SlabCorner { slab: SlabId, index: usize },
}

impl Dimension {
    /// Positions of the points along the direction, sorted, without
    /// repeats: the dimension line runs from the first to the last, and
    /// every pair of neighbours is one measured segment.
    pub fn stations(&self) -> Vec<f64> {
        stations(&self.points, self.direction)
    }

    /// Lengths of the measured segments, in order.
    pub fn values(&self) -> Vec<f64> {
        self.stations().windows(2).map(|w| w[1] - w[0]).collect()
    }

    /// Signed distance of the dimension line from the origin along the
    /// left normal of the direction.
    pub fn line_offset(&self) -> f64 {
        self.position.dot(self.direction.perp_left())
    }
}

fn stations(points: &[Point2], direction: Point2) -> Vec<f64> {
    let mut v: Vec<f64> = points.iter().map(|p| p.dot(direction)).collect();
    v.sort_by(f64::total_cmp);
    v.dedup_by(|a, b| (*a - *b).abs() <= EPSILON);
    v
}

/// Data needed to create a dimension.
#[derive(Debug, Clone, PartialEq)]
pub struct NewDimension {
    pub level: LevelId,
    pub points: Vec<Point2>,
    /// Measuring direction; any length, it is stored as a unit vector.
    pub direction: Point2,
    pub position: Point2,
}

/// A level mark in plan ("cota de nivel"): shows the elevation of the
/// finished floor at its point, from project zero. The value is not stored:
/// it is the top face of the highest slab of the level under the point, or
/// the level's elevation if there is no slab there.
#[derive(Debug, Clone, PartialEq)]
pub struct ElevationMark {
    pub id: ElevationMarkId,
    pub level: LevelId,
    pub point: Point2,
}

/// Data needed to create a wall; ids and GUIDs are assigned by the model.
#[derive(Debug, Clone, PartialEq)]
pub struct NewWall {
    pub wall_type: WallTypeId,
    pub level: LevelId,
    pub start: Point2,
    pub end: Point2,
    /// 0 for a straight wall; the arc's bulge for a curved one.
    pub bulge: f64,
    pub reference: ReferenceLine,
    /// Height above the base; ignored when `top` is a level.
    pub height: f64,
    pub base_offset: f64,
    pub top: WallTop,
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
    /// A wall's top constraint would put its top at or below its base.
    WallTopBelowBase(WallId),
    /// A slab outline needs at least three points, a non-zero area and no
    /// crossing or touching edges.
    InvalidOutline,
    /// The wall's start and end are the same point.
    ZeroLengthWall,
    /// A curved wall bends more than its thickness allows (its inner face
    /// would have no radius left), or almost closes on itself.
    ArcTooTight,
    /// A door or window too wide for the curve of its wall (it is straight:
    /// the wall would part from its ends by more than half its thickness),
    /// or a corner window in a curved wall.
    OpeningInCurvedWall,
    /// A hole of a slab crosses its outline or another hole, or is not a
    /// valid outline itself.
    InvalidHole,
    /// A dimension that must be positive is not.
    NonPositive(&'static str),
    /// A coordinate or dimension is not a finite number (NaN or infinite).
    NotFinite(&'static str),
    /// Two elements share an internal id.
    DuplicateId(u64),
    /// An internal id is zero or too large to be stored safely.
    InvalidId(u64),
    /// Two elements share an IFC `GlobalId`.
    DuplicateGuid(String),
    /// A wall or slab type must have at least one layer.
    EmptyType,
    /// A name or label with nothing in it.
    EmptyName,
    /// The element is still used by others and cannot be removed.
    InUse(String),
    /// A project parameter or its value is not valid (why, in Spanish).
    InvalidParameter(String),
    /// A door cannot become a window, or the other way round.
    CategoryMismatch,
    UnknownDimension(DimensionId),
    UnknownElevationMark(ElevationMarkId),
    /// A dimension needs at least two points apart along its direction.
    DegenerateDimension,
    /// The project needs at least one level.
    LastLevel,
    UnknownFamily(FamilyId),
    /// A family recipe is wrong, or the values given to it are.
    Family(FamilyError),
    /// The project already has a family with that name.
    DuplicateFamily(String),
    /// Another door or window type already has that mark.
    DuplicateMark(String),
    /// The family cannot make door or window types.
    NotAnOpening(NotAnOpening),
    UnknownMemberType(MemberTypeId),
    UnknownColumn(ColumnId),
    UnknownBeam(BeamId),
    /// A column's top constraint would put its top at or below its base.
    ColumnTopBelowBase(ColumnId),
    /// A beam's start and end are the same point.
    ZeroLengthBeam,
    /// A steel section whose web or flanges do not fit in it.
    InvalidProfile,
    /// No element with that id can have a phase.
    UnknownElement(u64),
    UnknownSection(SectionId),
    /// A section line needs two different points.
    ZeroLengthSection,
    /// A slab slope with no direction or steeper than [`MAX_SLAB_SLOPE`].
    InvalidSlope,
    UnknownRoof(RoofId),
    /// A roof with no sloping side, a pitch out of range, a side without
    /// one, or too much overhang.
    InvalidRoof,
    /// A roof footprint with a re-entrant corner.
    RoofNotConvex,
    /// No dormer with that index on the roof.
    UnknownDormer(usize),
    /// A dormer or gutter with a size out of range.
    InvalidDormer,
    /// A dormer that is not all on one plane of its roof (or whose own
    /// roof cannot meet it).
    DormerDoesNotFit,
    UnknownStair(StairId),
    UnknownTerrain(TerrainId),
    UnknownCurtainWall(CurtainWallId),
    UnknownCurtainType(CurtainTypeId),
    UnknownRailing(RailingId),
    UnknownObjectType(ObjectTypeId),
    UnknownObject(ObjectId),
    UnknownMesh(MeshId),
    /// A 3D model that cannot be kept (no faces, indices out of range…).
    InvalidMesh(String),
    /// The family does not make objects (it is a door, a window, or goes in
    /// a wall).
    NotAnObject(String),
    /// A material priority above 999.
    InvalidPriority,
    /// A railing with fewer than two points, or out of range.
    InvalidRailing,
    /// A curtain wall with no height, too small a grid or a frame wider
    /// than its bays.
    InvalidCurtainWall,
    /// A curtain wall type with too small a division, a frame or panel out
    /// of range, or a row pointing at no panel.
    InvalidCurtainType,
    /// A terrain with fewer than three points out of line, or no depth.
    InvalidTerrain,
    /// A wall profile on a curved wall, or with an outline or hole
    /// without area.
    InvalidWallProfile(WallId),
    /// A stair with no flight, nothing to climb, or too narrow or shallow.
    InvalidStair,
    /// Another room on the level already has that number.
    DuplicateNumber(String),
    UnknownDrawLayer(DrawLayerId),
    UnknownDraft(DraftId),
    UnknownSheet(SheetId),
    UnknownViewport(ViewportId),
    UnknownRoom(RoomId),
    UnknownPresentationStyle(PresentationStyleId),
    UnknownPlanView(PlanViewId),
    UnknownViewTemplate(ViewTemplateId),
    UnknownRefPlane(RefPlaneId),
    UnknownTag(TagId),
    UnknownTable(TableId),
    /// A drafting element or annotation is not well formed.
    InvalidShape(&'static str),
    /// Another element of the same list already has that name.
    DuplicateName(String),
    /// The project already has a type with that `GlobalId` and other values.
    TypeConflict(String),
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
            ModelError::WallTopBelowBase(id) => {
                write!(f, "the top constraint of wall {id} is not above its base")
            }
            ModelError::InvalidOutline => write!(
                f,
                "a slab outline needs at least three points and must not cross itself"
            ),
            ModelError::ZeroLengthWall => write!(f, "a wall needs two different points"),
            ModelError::InvalidHole => write!(
                f,
                "a slab hole must lie inside the slab, clear of its other holes"
            ),
            ModelError::ArcTooTight => {
                write!(f, "the curved wall bends too much for its thickness")
            }
            ModelError::OpeningInCurvedWall => write!(
                f,
                "the door or window is too wide for the curve of its wall (or turns a corner of a curved wall)"
            ),
            ModelError::NonPositive(what) => write!(f, "{what} must be greater than zero"),
            ModelError::NotFinite(what) => write!(f, "{what} is not a valid number"),
            ModelError::DuplicateId(id) => write!(f, "id {id} is used by more than one element"),
            ModelError::InvalidId(id) => write!(f, "id {id} must be between 1 and {MAX_ID}"),
            ModelError::DuplicateGuid(guid) => {
                write!(f, "GlobalId {guid} is used by more than one element")
            }
            ModelError::EmptyType => write!(f, "a type needs at least one layer"),
            ModelError::EmptyName => write!(f, "a name cannot be empty"),
            ModelError::InUse(what) => write!(f, "still in use: {what}"),
            ModelError::InvalidParameter(why) => f.write_str(why),
            ModelError::CategoryMismatch => {
                write!(
                    f,
                    "a door can only become another door, and a window another window"
                )
            }
            ModelError::UnknownDimension(id) => write!(f, "dimension {id} does not exist"),
            ModelError::UnknownElevationMark(id) => {
                write!(f, "level mark {id} does not exist")
            }
            ModelError::DegenerateDimension => write!(
                f,
                "a dimension needs at least two points apart along its direction"
            ),
            ModelError::LastLevel => write!(f, "the project needs at least one level"),
            ModelError::UnknownFamily(id) => write!(f, "family {id} does not exist"),
            ModelError::Family(e) => write!(f, "{e}"),
            ModelError::DuplicateFamily(name) => {
                write!(f, "the project already has a family called «{name}»")
            }
            ModelError::DuplicateMark(mark) => {
                write!(f, "another type already has the mark «{mark}»")
            }
            ModelError::NotAnOpening(e) => write!(f, "{e}"),
            ModelError::UnknownMemberType(id) => {
                write!(f, "column or beam type {id} does not exist")
            }
            ModelError::UnknownColumn(id) => write!(f, "column {id} does not exist"),
            ModelError::UnknownBeam(id) => write!(f, "beam {id} does not exist"),
            ModelError::ColumnTopBelowBase(id) => {
                write!(f, "the top constraint of column {id} is not above its base")
            }
            ModelError::ZeroLengthBeam => write!(f, "a beam needs two different points"),
            ModelError::InvalidProfile => {
                write!(f, "the web and flanges do not fit in the section")
            }
            ModelError::UnknownElement(id) => write!(f, "element {id} does not exist"),
            ModelError::UnknownSection(id) => write!(f, "section {id} does not exist"),
            ModelError::ZeroLengthSection => write!(f, "a section line needs two different points"),
            ModelError::UnknownRoof(id) => write!(f, "unknown roof {id}"),
            ModelError::UnknownStair(id) => write!(f, "unknown stair {id}"),
            ModelError::UnknownTerrain(id) => write!(f, "unknown terrain {id}"),
            ModelError::UnknownCurtainWall(id) => write!(f, "unknown curtain wall {id}"),
            ModelError::UnknownCurtainType(id) => write!(f, "unknown curtain wall type {id}"),
            ModelError::InvalidCurtainType => write!(
                f,
                "a curtain wall type needs bays of at least 0.1 m (0.2 m when equal), frames of 5 mm to 1 m, at least one panel no thicker than the frame and rows pointing at its panels"
            ),
            ModelError::UnknownRailing(id) => write!(f, "unknown railing {id}"),
            ModelError::UnknownObjectType(id) => write!(f, "unknown object type {id}"),
            ModelError::UnknownObject(id) => write!(f, "unknown object {id}"),
            ModelError::UnknownMesh(id) => write!(f, "unknown 3D model {id}"),
            ModelError::InvalidMesh(why) => write!(f, "3D model not valid: {why}"),
            ModelError::NotAnObject(name) => {
                write!(f, "the family «{name}» does not make objects placed on a level")
            }
            ModelError::InvalidPriority => write!(f, "an intersection priority goes from 0 to 999"),
            ModelError::InvalidRailing => write!(
                f,
                "a railing needs two points at least, 5 cm apart, a height from 0.3 to 3 m and posts 0.3 to 5 m apart"
            ),
            ModelError::InvalidCurtainWall => write!(
                f,
                "a curtain wall needs some height and room between the frames of its type in every bay and row"
            ),
            ModelError::InvalidWallProfile(id) => write!(
                f,
                "wall {} cannot take that profile (straight walls only, outlines with area)",
                id.0
            ),
            ModelError::InvalidTerrain => write!(
                f,
                "a terrain needs three points not in a line and some depth"
            ),
            ModelError::InvalidStair => write!(
                f,
                "a stair needs a walking line long enough for a flight, something to climb, and a usable width and tread"
            ),
            ModelError::InvalidRoof => write!(
                f,
                "a roof needs a pitch for each side (or a gable), at least one sloping side, pitches up to {MAX_SLAB_SLOPE} and an overhang up to {MAX_OVERHANG} m"
            ),
            ModelError::UnknownDormer(i) => write!(f, "unknown dormer {i}"),
            ModelError::InvalidDormer => write!(
                f,
                "a dormer needs a width of 0.4 to 8 m, a height of 0.3 to 4 m and a pitch up to {MAX_SLAB_SLOPE}; a gutter a size of 5 to 50 cm"
            ),
            ModelError::DormerDoesNotFit => write!(
                f,
                "the dormer is not all on one plane of the roof (or its own roof is as steep as the plane)"
            ),
            ModelError::RoofNotConvex => write!(
                f,
                "the planes of this roof footprint cannot be worked out (or its eaves cross themselves)"
            ),
            ModelError::InvalidSlope => write!(
                f,
                "a slab slope needs a direction and at most {MAX_SLAB_SLOPE} m per metre"
            ),
            ModelError::DuplicateNumber(n) => {
                write!(f, "another room on the level already has the number «{n}»")
            }
            ModelError::UnknownDrawLayer(id) => write!(f, "drawing layer {id} does not exist"),
            ModelError::UnknownDraft(id) => write!(f, "drafting element {id} does not exist"),
            ModelError::UnknownSheet(id) => write!(f, "sheet {id} does not exist"),
            ModelError::UnknownViewport(id) => write!(f, "view {id} on a sheet does not exist"),
            ModelError::UnknownRoom(id) => write!(f, "room {id} does not exist"),
            ModelError::UnknownPresentationStyle(id) => {
                write!(f, "presentation style {id} does not exist")
            }
            ModelError::UnknownPlanView(id) => write!(f, "plan view {id} does not exist"),
            ModelError::UnknownViewTemplate(id) => {
                write!(f, "view template {id} does not exist")
            }
            ModelError::UnknownRefPlane(id) => write!(f, "reference plane {id} does not exist"),
            ModelError::UnknownTag(id) => write!(f, "tag {id} does not exist"),
            ModelError::UnknownTable(id) => write!(f, "schedule {id} on a sheet does not exist"),
            ModelError::InvalidShape(what) => write!(f, "{what}"),
            ModelError::DuplicateName(name) => {
                write!(f, "another element is already called «{name}»")
            }
            ModelError::TypeConflict(name) => {
                write!(f, "the project already has the type «{name}» with other values")
            }
        }
    }
}

impl std::error::Error for ModelError {}

/// Largest internal id: integers up to 2^53 survive any JSON reader.
pub const MAX_ID: u64 = 1 << 53;

/// Largest bulge of a curved wall: an arc of 340°. A full circle is drawn
/// as two half circles.
pub const MAX_BULGE: f64 = 11.430052302761343;

/// Smallest radius left for the inner face of a curved wall, in metres.
pub const MIN_INNER_RADIUS: f64 = 0.05;

/// Whether a closed polygon contains a point (ray casting).
pub(crate) fn polygon_contains(poly: &[Point2], p: Point2) -> bool {
    let mut inside = false;
    let n = poly.len();
    for i in 0..n {
        let (a, b) = (poly[i], poly[(i + 1) % n]);
        if (a.y > p.y) != (b.y > p.y) {
            let x = a.x + (p.y - a.y) / (b.y - a.y) * (b.x - a.x);
            if p.x < x {
                inside = !inside;
            }
        }
    }
    inside
}

/// What a door or window family type gives the rest of the program.
struct Resolved {
    spec: NewOpeningType,
    panels: Vec<String>,
    shape: archisim_family::OpeningShape,
    frame_place: FramePlace,
}

fn kind_is_window(kind: OpeningKind) -> bool {
    matches!(kind, OpeningKind::Window(_))
}

/// The type parameter of a door or window recipe that says whether its
/// opening lines are drawn in the model's sections and elevations
/// ([`Model::opening_lines_in_views`]).
pub const OPENING_LINES_IN_VIEWS: &str = "opening_lines_in_views";

/// The values the program gives every wall-hosted family: the side the
/// leaves open to and the handing.
pub fn opening_builtins(opens_to: WallSide, hand: Hand) -> Values {
    let mut v = Values::new();
    let side = match opens_to {
        WallSide::Interior => "interior",
        WallSide::Exterior => "exterior",
    };
    let hand = match hand {
        Hand::Left => "left",
        Hand::Right => "right",
    };
    v.insert("opens_to".into(), Value::Text(side.into()));
    v.insert("hand".into(), Value::Text(hand.into()));
    v.insert("detailed".into(), Value::Bool(false));
    // Where the wall's faces are (`y`, towards the side the leaves open
    // to), until a placed one says: a wall of 30 cm, the frame centred.
    let (ext, int) = match opens_to {
        WallSide::Interior => (-0.15, 0.15),
        WallSide::Exterior => (0.15, -0.15),
    };
    v.insert("wall_ext".into(), Value::Number(ext));
    v.insert("wall_int".into(), Value::Number(int));
    v.insert("sill_height".into(), Value::Number(0.0));
    v
}

/// Rejects NaN and infinite values.
fn finite(value: f64, what: &'static str) -> Result<(), ModelError> {
    if value.is_finite() {
        Ok(())
    } else {
        Err(ModelError::NotFinite(what))
    }
}

pub(crate) fn finite_point(p: Point2, what: &'static str) -> Result<(), ModelError> {
    finite(p.x, what)?;
    finite(p.y, what)
}

/// Every element of a model laid out flat, as stored in a project file.
///
/// [`Model::to_parts`] produces it and [`Model::from_parts`] rebuilds a model
/// from it, checking everything an interactive edit would check. Ids and
/// GUIDs are kept exactly as given.
#[derive(Debug, Clone, PartialEq)]
pub struct ModelParts {
    pub project_name: String,
    pub project_guid: GlobalId,
    pub site_guid: GlobalId,
    pub building_guid: GlobalId,
    pub levels: Vec<Level>,
    pub materials: Vec<Material>,
    pub wall_types: Vec<WallType>,
    pub walls: Vec<Wall>,
    pub slab_types: Vec<SlabType>,
    pub slabs: Vec<Slab>,
    /// Families, before the types that use them.
    pub families: Vec<Family>,
    /// Door and window types. Only the identity, family and values are
    /// read back: the rest is worked out from the family.
    pub opening_types: Vec<OpeningType>,
    pub openings: Vec<Opening>,
    pub dimensions: Vec<Dimension>,
    pub elevation_marks: Vec<ElevationMark>,
    pub member_types: Vec<MemberType>,
    pub columns: Vec<Column>,
    pub beams: Vec<Beam>,
    /// Elements that are not new: (id, phase).
    pub phases: Vec<(u64, Phase)>,
    /// Elements given a discipline by hand: (id, discipline).
    pub disciplines: Vec<(u64, ElementDiscipline)>,
    /// Parameters of the project and the values given to elements and
    /// types (by id).
    pub user_parameters: Vec<UserParameter>,
    pub parameter_values: Vec<(u64, UserParameterId, Value)>,
    pub sections: Vec<SectionLine>,
    pub grids: Vec<GridLine>,
    pub footings: Vec<Footing>,
    pub roofs: Vec<Roof>,
    /// Walls trimmed under a roof: (wall, roof).
    pub wall_roofs: Vec<(WallId, RoofId)>,
    pub stairs: Vec<Stair>,
    pub terrains: Vec<Terrain>,
    pub curtain_types: Vec<CurtainWallType>,
    pub curtain_walls: Vec<CurtainWall>,
    pub railings: Vec<Railing>,
    /// 3D models brought from other programs, before the types that
    /// show them.
    pub meshes: Vec<MeshModel>,
    pub object_types: Vec<ObjectType>,
    pub objects: Vec<Object>,
    /// Module files kept in the project, and the copies placed.
    pub module_sources: Vec<ModuleSource>,
    pub modules: Vec<ModulePlacement>,
    /// Volumes of the concept phase, and the plot area their figures are
    /// worked out with.
    pub masses: Vec<Mass>,
    /// Loose edges of the concept phase.
    pub edges: Vec<ConceptEdge>,
    /// Paint on the faces of walls and volumes, and painted zones of
    /// walls' faces.
    pub face_paints: Vec<(u64, FaceRef, MaterialId)>,
    pub paint_zones: Vec<PaintZone>,
    /// How materials look in 3D (colour and texture).
    pub appearances: Vec<(MaterialId, Appearance)>,
    /// How views can be presented.
    pub presentation_styles: Vec<crate::graphics::PresentationStyle>,
    /// Saved plans, the style of each level's own plan, view templates
    /// and reference planes.
    pub plan_views: Vec<PlanView>,
    pub level_styles: Vec<(LevelId, crate::graphics::ViewStyle)>,
    pub view_templates: Vec<ViewTemplate>,
    pub ref_planes: Vec<RefPlane>,
    /// When the shadows of presentation drawings are cast.
    pub sun: Sun,
    /// Prices of the budget's items, by their keys.
    pub prices: Vec<(String, PriceConcept)>,
    /// Overheads, profit and VAT of the budget.
    pub budget: BudgetSettings,
    pub plot_area: Option<f64>,
    /// Plots and roads of the site.
    pub plots: Vec<Plot>,
    pub roads: Vec<Road>,
    /// Plants spread along lines or over areas (after the objects they
    /// made).
    pub plantings: Vec<Planting>,
    /// Pools built in the ground.
    pub pools: Vec<Pool>,
    /// Pipes, ducts and devices.
    pub runs: Vec<ServiceRun>,
    pub devices: Vec<ServiceDevice>,
    /// Radians, clockwise from the plan's up.
    pub true_north: f64,
    /// The uses rooms of this project are offered (empty: the default
    /// residential list).
    pub room_usages: Vec<String>,
    /// Where the project is on the map.
    pub georeference: Option<crate::georef::Georeference>,
    /// Use, accessibility and sprinklers, for the code checks.
    pub code: crate::code::CodeSettings,
    /// Drawing layers, layer "0" included.
    pub draw_layers: Vec<DrawLayer>,
    pub drafts: Vec<Draft>,
    pub graphics: crate::graphics::GraphicStandard,
    pub info: crate::sheets::ProjectInfo,
    pub sheets: Vec<crate::sheets::Sheet>,
    pub viewports: Vec<crate::sheets::Viewport>,
    pub rooms: Vec<crate::rooms::Room>,
    pub tags: Vec<crate::rooms::Tag>,
    pub tables: Vec<crate::sheets::SheetTable>,
}

/// Why a model could not be rebuilt from its parts: which element failed and
/// why.
#[derive(Debug, Clone, PartialEq)]
pub struct RestoreError {
    /// Human-readable element reference, e.g. `wall 12 (2N1qQ…)`.
    pub element: String,
    pub error: ModelError,
}

impl fmt::Display for RestoreError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.element, self.error)
    }
}

impl std::error::Error for RestoreError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(&self.error)
    }
}

/// The building model: every element of a project, without undo history.
#[derive(Debug, Clone, PartialEq)]
pub struct Model {
    pub project_name: String,
    pub project_guid: GlobalId,
    pub site_guid: GlobalId,
    pub building_guid: GlobalId,
    /// Line weights of the drawings by scale.
    pub graphics: crate::graphics::GraphicStandard,
    next_id: u64,
    /// The highest id any element has had (team work hands out ids by
    /// blocks, so it can be above `next_id`).
    top_id: u64,
    levels: BTreeMap<LevelId, Level>,
    materials: BTreeMap<MaterialId, Material>,
    wall_types: BTreeMap<WallTypeId, WallType>,
    walls: BTreeMap<WallId, Wall>,
    slab_types: BTreeMap<SlabTypeId, SlabType>,
    slabs: BTreeMap<SlabId, Slab>,
    families: BTreeMap<FamilyId, Family>,
    opening_types: BTreeMap<OpeningTypeId, OpeningType>,
    openings: BTreeMap<OpeningId, Opening>,
    dimensions: BTreeMap<DimensionId, Dimension>,
    elevation_marks: BTreeMap<ElevationMarkId, ElevationMark>,
    member_types: BTreeMap<MemberTypeId, MemberType>,
    columns: BTreeMap<ColumnId, Column>,
    beams: BTreeMap<BeamId, Beam>,
    /// Phases of the elements that are not new, by id.
    phases: BTreeMap<u64, Phase>,
    /// Disciplines given by hand, by element id.
    disciplines: BTreeMap<u64, ElementDiscipline>,
    user_params: BTreeMap<UserParameterId, UserParameter>,
    param_values: BTreeMap<u64, BTreeMap<UserParameterId, Value>>,
    sections: BTreeMap<SectionId, SectionLine>,
    grids: BTreeMap<GridId, GridLine>,
    footings: BTreeMap<FootingId, Footing>,
    roofs: BTreeMap<RoofId, Roof>,
    /// Walls trimmed under a roof (ArchiCAD's "Recortar con cubierta",
    /// Revit's "Enlazar parte superior").
    wall_roofs: BTreeMap<WallId, RoofId>,
    stairs: BTreeMap<StairId, Stair>,
    terrains: BTreeMap<TerrainId, Terrain>,
    curtain_types: BTreeMap<CurtainTypeId, CurtainWallType>,
    curtain_walls: BTreeMap<CurtainWallId, CurtainWall>,
    railings: BTreeMap<RailingId, Railing>,
    meshes: BTreeMap<MeshId, MeshModel>,
    object_types: BTreeMap<ObjectTypeId, ObjectType>,
    objects: BTreeMap<ObjectId, Object>,
    module_sources: BTreeMap<ModuleSourceId, ModuleSource>,
    modules: BTreeMap<ModuleId, ModulePlacement>,
    masses: BTreeMap<MassId, Mass>,
    edges: BTreeMap<EdgeId, ConceptEdge>,
    face_paints: BTreeMap<(u64, FaceRef), MaterialId>,
    appearances: BTreeMap<MaterialId, Appearance>,
    paint_zones: Vec<PaintZone>,
    presentation_styles: BTreeMap<PresentationStyleId, crate::graphics::PresentationStyle>,
    plan_views: BTreeMap<PlanViewId, PlanView>,
    level_styles: BTreeMap<LevelId, crate::graphics::ViewStyle>,
    view_templates: BTreeMap<ViewTemplateId, ViewTemplate>,
    ref_planes: BTreeMap<RefPlaneId, RefPlane>,
    sun: Sun,
    prices: BTreeMap<String, PriceConcept>,
    budget: BudgetSettings,
    plot_area: Option<f64>,
    plots: BTreeMap<PlotId, Plot>,
    roads: BTreeMap<RoadId, Road>,
    plantings: BTreeMap<PlantingId, Planting>,
    pools: BTreeMap<PoolId, Pool>,
    runs: BTreeMap<RunId, ServiceRun>,
    devices: BTreeMap<DeviceId, ServiceDevice>,
    /// True north: radians, clockwise from the plan's up (+Y).
    true_north: f64,
    pub(crate) room_usages: Vec<String>,
    pub(crate) georeference: Option<crate::georef::Georeference>,
    pub(crate) code: crate::code::CodeSettings,
    /// The phase new elements are given (the survey is drawn as
    /// existing). Not saved.
    pub draw_phase: Phase,
    pub(crate) draw_layers: BTreeMap<DrawLayerId, DrawLayer>,
    pub(crate) drafts: BTreeMap<DraftId, Draft>,
    pub(crate) info: crate::sheets::ProjectInfo,
    pub(crate) sheets: BTreeMap<SheetId, crate::sheets::Sheet>,
    pub(crate) viewports: BTreeMap<ViewportId, crate::sheets::Viewport>,
    pub(crate) rooms: BTreeMap<RoomId, crate::rooms::Room>,
    pub(crate) tags: BTreeMap<TagId, crate::rooms::Tag>,
    pub(crate) tables: BTreeMap<TableId, crate::sheets::SheetTable>,
}

impl Model {
    pub fn new(project_name: impl Into<String>) -> Self {
        Self {
            project_name: project_name.into(),
            project_guid: GlobalId::new(),
            site_guid: GlobalId::new(),
            building_guid: GlobalId::new(),
            graphics: Default::default(),
            next_id: 1,
            top_id: 0,
            levels: BTreeMap::new(),
            materials: BTreeMap::new(),
            wall_types: BTreeMap::new(),
            walls: BTreeMap::new(),
            slab_types: BTreeMap::new(),
            slabs: BTreeMap::new(),
            families: BTreeMap::new(),
            opening_types: BTreeMap::new(),
            openings: BTreeMap::new(),
            dimensions: BTreeMap::new(),
            elevation_marks: BTreeMap::new(),
            member_types: BTreeMap::new(),
            columns: BTreeMap::new(),
            beams: BTreeMap::new(),
            phases: BTreeMap::new(),
            disciplines: BTreeMap::new(),
            user_params: BTreeMap::new(),
            param_values: BTreeMap::new(),
            sections: BTreeMap::new(),
            grids: BTreeMap::new(),
            footings: BTreeMap::new(),
            roofs: BTreeMap::new(),
            wall_roofs: BTreeMap::new(),
            stairs: BTreeMap::new(),
            terrains: BTreeMap::new(),
            curtain_types: BTreeMap::new(),
            curtain_walls: BTreeMap::new(),
            railings: BTreeMap::new(),
            meshes: BTreeMap::new(),
            object_types: BTreeMap::new(),
            objects: BTreeMap::new(),
            module_sources: BTreeMap::new(),
            modules: BTreeMap::new(),
            masses: BTreeMap::new(),
            edges: BTreeMap::new(),
            face_paints: BTreeMap::new(),
            appearances: BTreeMap::new(),
            paint_zones: Vec::new(),
            presentation_styles: BTreeMap::new(),
            plan_views: BTreeMap::new(),
            level_styles: BTreeMap::new(),
            view_templates: BTreeMap::new(),
            ref_planes: BTreeMap::new(),
            sun: Sun::default(),
            prices: BTreeMap::new(),
            budget: BudgetSettings::default(),
            plot_area: None,
            plots: BTreeMap::new(),
            roads: BTreeMap::new(),
            plantings: BTreeMap::new(),
            pools: BTreeMap::new(),
            runs: BTreeMap::new(),
            devices: BTreeMap::new(),
            true_north: 0.0,
            room_usages: Vec::new(),
            georeference: None,
            code: Default::default(),
            draw_phase: Phase::New,
            draw_layers: BTreeMap::from([(LAYER_ZERO, DrawLayer::zero())]),
            drafts: BTreeMap::new(),
            info: Default::default(),
            sheets: BTreeMap::new(),
            viewports: BTreeMap::new(),
            rooms: BTreeMap::new(),
            tags: BTreeMap::new(),
            tables: BTreeMap::new(),
        }
    }

    pub(crate) fn take_id(&mut self) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        self.top_id = self.top_id.max(id);
        id
    }

    /// The first id above every id the model has used: where the copies
    /// of modules number their elements in [`Model::flattened`].
    pub fn first_unused_id(&self) -> u64 {
        self.top_id.max(self.next_id.saturating_sub(1)) + 1
    }

    /// The id the next new element will get.
    pub fn next_id(&self) -> u64 {
        self.next_id
    }

    /// Makes new elements take ids from `next` on. Team work gives every
    /// member a block of its own, so two people adding elements at the same
    /// time never hand out the same id. The caller must make sure no
    /// element of the model already has an id at or above `next` within
    /// the ids it will use.
    pub fn set_next_id(&mut self, next: u64) {
        self.next_id = next.clamp(1, MAX_ID);
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

    /// Renames a level.
    pub fn rename_level(&mut self, id: LevelId, name: impl Into<String>) -> Result<(), ModelError> {
        let level = self
            .levels
            .get_mut(&id)
            .ok_or(ModelError::UnknownLevel(id))?;
        level.name = name.into();
        Ok(())
    }

    /// Changes a level's elevation. Everything on it moves with it: walls,
    /// slabs and doors are placed relative to their level.
    pub fn set_level_elevation(&mut self, id: LevelId, elevation: f64) -> Result<(), ModelError> {
        finite(elevation, "level elevation")?;
        let level = self
            .levels
            .get_mut(&id)
            .ok_or(ModelError::UnknownLevel(id))?;
        let before = std::mem::replace(&mut level.elevation, elevation);
        // Walls tied to a level up (or standing on this one) follow; if one
        // cannot, nothing changes.
        let walls = self.walls.clone();
        let columns = self.columns.clone();
        if let Err(e) = self
            .refresh_wall_heights()
            .and_then(|_| self.refresh_column_heights())
        {
            self.walls = walls;
            self.columns = columns;
            if let Some(l) = self.levels.get_mut(&id) {
                l.elevation = before;
            }
            return Err(e);
        }
        self.refresh_stair_heights();
        self.refresh_curtain_heights();
        Ok(())
    }

    /// Removes a level and everything on it (walls with their doors and
    /// windows, slabs, dimensions and level marks). The last level cannot be
    /// removed.
    pub fn remove_level(&mut self, id: LevelId) -> Result<Level, ModelError> {
        if !self.levels.contains_key(&id) {
            return Err(ModelError::UnknownLevel(id));
        }
        if self.levels.len() == 1 {
            return Err(ModelError::LastLevel);
        }
        let walls: Vec<WallId> = self.walls_on_level(id).map(|w| w.id).collect();
        for w in walls {
            self.remove_wall(w)?;
        }
        // Walls of other levels tied to this one keep their height, free.
        for w in self.walls.values_mut() {
            if matches!(w.top, WallTop::Level { level, .. } if level == id) {
                w.top = WallTop::Unconnected;
            }
        }
        self.slabs.retain(|_, s| s.level != id);
        self.remove_members_on_level(id);
        self.roofs.retain(|_, r| r.level != id);
        let roofs = &self.roofs;
        self.wall_roofs.retain(|_, r| roofs.contains_key(r));
        self.stairs.retain(|_, s| s.level != id);
        self.terrains.retain(|_, t| t.level != id);
        self.curtain_walls.retain(|_, c| c.level != id);
        self.railings.retain(|_, r| r.level != id);
        self.objects.retain(|_, o| o.level != id);
        self.footings.retain(|_, f| f.level != id);
        self.modules.retain(|_, m| m.level != id);
        self.masses.retain(|_, m| m.level != id);
        self.edges.retain(|_, e| e.level != id);
        self.plots.retain(|_, p| p.level != id);
        self.roads.retain(|_, r| r.level != id);
        self.plantings.retain(|_, p| p.level != id);
        self.pools.retain(|_, p| p.level != id);
        self.runs.retain(|_, r| r.level != id);
        self.devices.retain(|_, d| d.level != id);
        self.plan_views.retain(|_, v| v.level != id);
        self.level_styles.remove(&id);
        for c in self.curtain_walls.values_mut() {
            if matches!(c.top, WallTop::Level { level, .. } if level == id) {
                c.top = WallTop::Unconnected;
            }
        }
        for s in self.stairs.values_mut() {
            if matches!(s.top, WallTop::Level { level, .. } if level == id) {
                s.top = WallTop::Unconnected;
            }
        }
        self.dimensions.retain(|_, d| d.level != id);
        self.elevation_marks.retain(|_, m| m.level != id);
        self.drafts.retain(|_, d| d.view != ViewRef::Level(id));
        self.viewports.retain(|_, v| v.level != id);
        self.rooms.retain(|_, r| r.level != id);
        self.tags.retain(|_, t| t.level != id);
        for t in self.tables.values_mut() {
            if t.level == Some(id) {
                t.level = None;
            }
        }
        self.levels.remove(&id).ok_or(ModelError::UnknownLevel(id))
    }

    /// How many elements a level holds: walls, doors and windows, slabs and
    /// annotations (dimensions and level marks).
    pub fn level_contents(&self, id: LevelId) -> (usize, usize, usize, usize) {
        let walls: Vec<WallId> = self.walls_on_level(id).map(|w| w.id).collect();
        let openings = self
            .openings
            .values()
            .filter(|o| walls.contains(&o.wall))
            .count();
        let annotations = self.dimensions.values().filter(|d| d.level == id).count()
            + self
                .elevation_marks
                .values()
                .filter(|m| m.level == id)
                .count()
            + self
                .drafts
                .values()
                .filter(|d| d.view == ViewRef::Level(id))
                .count();
        (
            walls.len(),
            openings,
            self.slabs_on_level(id).count() + self.roofs_on_level(id).count(),
            annotations,
        )
    }

    /// The lowest level above `level`, if any.
    pub fn level_above(&self, level: LevelId) -> Option<LevelId> {
        let base = self.levels.get(&level)?.elevation;
        self.levels()
            .into_iter()
            .find(|l| l.elevation > base + EPSILON)
            .map(|l| l.id)
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
                priority: None,
            },
        );
        id
    }

    /// Sets (or clears, back to the layer function's) the intersection
    /// priority of a material.
    pub fn set_material_priority(
        &mut self,
        id: MaterialId,
        priority: Option<u16>,
    ) -> Result<(), ModelError> {
        if priority.is_some_and(|p| p > MAX_PRIORITY) {
            return Err(ModelError::InvalidPriority);
        }
        self.materials
            .get_mut(&id)
            .ok_or(ModelError::UnknownMaterial(id))?
            .priority = priority;
        Ok(())
    }

    /// Intersection priority of a layer: its material's, or its function's.
    pub fn layer_priority(&self, layer: &Layer) -> u16 {
        self.materials
            .get(&layer.material)
            .and_then(|m| m.priority)
            .unwrap_or_else(|| layer.function.default_priority())
    }

    pub fn material(&self, id: MaterialId) -> Option<&Material> {
        self.materials.get(&id)
    }

    pub fn materials(&self) -> impl Iterator<Item = &Material> {
        self.materials.values()
    }

    /// Changes a material's name, category and how it is drawn when cut.
    /// Every layer, frame and leaf made of it changes with it.
    pub fn set_material(
        &mut self,
        id: MaterialId,
        name: impl Into<String>,
        category: impl Into<String>,
        cut_color: [u8; 3],
        cut_pattern: CutPattern,
    ) -> Result<(), ModelError> {
        let m = self
            .materials
            .get_mut(&id)
            .ok_or(ModelError::UnknownMaterial(id))?;
        m.name = name.into();
        m.category = category.into();
        m.cut_color = cut_color;
        m.cut_pattern = cut_pattern;
        Ok(())
    }

    /// Names of the wall, slab, door and window types that use a material,
    /// in that order.
    pub fn material_users(&self, id: MaterialId) -> Vec<String> {
        let in_layers = |layers: &[Layer]| layers.iter().any(|l| l.material == id);
        let walls = self
            .wall_types
            .values()
            .filter(|t| in_layers(&t.layers))
            .map(|t| t.name.clone());
        let slabs = self
            .slab_types
            .values()
            .filter(|t| in_layers(&t.layers))
            .map(|t| t.name.clone());
        // A material of a parameter the type does not show (a transom's
        // glass on a door without one) is not a use.
        let openings = self
            .opening_types
            .values()
            .filter(|t| {
                let def = self.families.get(&t.family).map(|f| f.def());
                t.values.iter().any(|(k, v)| {
                    *v == Value::Material(id.0) && def.is_none_or(|d| d.is_visible(k, &t.values))
                })
            })
            .map(|t| t.name.clone());
        let members = self
            .member_types
            .values()
            .filter(|t| t.material == id)
            .map(|t| t.name.clone());
        let stairs = self
            .stairs
            .values()
            .filter(|s| s.material == id)
            .map(|s| format!("escalera {}", s.id.0))
            .chain(
                self.terrains
                    .values()
                    .filter(|t| t.material == id)
                    .map(|t| format!("terreno {}", t.id.0)),
            )
            .chain(
                self.curtain_types
                    .values()
                    .filter(|t| t.design.materials().contains(&id))
                    .map(|t| t.name.clone()),
            )
            .chain(
                self.railings
                    .values()
                    .filter(|r| r.material == id || r.glass == Some(id))
                    .map(|r| format!("barandilla {}", r.id.0)),
            )
            .chain(
                self.object_types
                    .values()
                    .filter(|t| t.values.values().any(|v| *v == Value::Material(id.0)))
                    .map(|t| t.name.clone()),
            );
        walls
            .chain(slabs)
            .chain(openings)
            .chain(members)
            .chain(stairs)
            .collect()
    }

    /// Removes a material that no type uses.
    pub fn remove_material(&mut self, id: MaterialId) -> Result<Material, ModelError> {
        if !self.materials.contains_key(&id) {
            return Err(ModelError::UnknownMaterial(id));
        }
        let users = self.material_users(id);
        if !users.is_empty() {
            return Err(ModelError::InUse(format!(
                "{} type(s) use this material",
                users.len()
            )));
        }
        let removed = self
            .materials
            .remove(&id)
            .ok_or(ModelError::UnknownMaterial(id))?;
        // Hidden parameters that pointed to it take another one.
        if let Some(other) = self.materials.keys().next().copied() {
            for t in self.opening_types.values_mut() {
                for v in t.values.values_mut() {
                    if *v == Value::Material(id.0) {
                        *v = Value::Material(other.0);
                    }
                }
            }
        }
        Ok(removed)
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
                wrap_inserts: Wrap::inserts_by_default(function, &layers),
                wrap_ends: Wrap::None,
                layers,
                sweeps: Vec::new(),
            },
        );
        Ok(id)
    }

    /// Revit's «Envolver en extremos» and «Envolver en huecos» of a wall
    /// type.
    pub fn set_wall_type_wrap(
        &mut self,
        id: WallTypeId,
        ends: Wrap,
        inserts: Wrap,
    ) -> Result<(), ModelError> {
        let t = self
            .wall_types
            .get_mut(&id)
            .ok_or(ModelError::UnknownWallType(id))?;
        t.wrap_ends = ends;
        t.wrap_inserts = inserts;
        Ok(())
    }

    fn validate_layers(&self, layers: &[Layer]) -> Result<(), ModelError> {
        if layers.is_empty() {
            return Err(ModelError::EmptyType);
        }
        for layer in layers {
            if !self.materials.contains_key(&layer.material) {
                return Err(ModelError::UnknownMaterial(layer.material));
            }
            finite(layer.thickness, "layer thickness")?;
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
        let height = self.validate_wall(&spec)?;
        let id = WallId(self.take_id());
        let wall = Wall {
            id,
            guid: GlobalId::new(),
            wall_type: spec.wall_type,
            level: spec.level,
            start: spec.start,
            end: spec.end,
            bulge: if spec.bulge.abs() < MIN_BULGE {
                0.0
            } else {
                spec.bulge
            },
            reference: spec.reference,
            height,
            base_offset: spec.base_offset,
            top: spec.top,
            allow_join: [true, true],
            profile: None,
        };
        self.walls.insert(id, wall);
        self.stamp_phase(id.0);
        Ok(id)
    }

    /// The height a wall gets from its constraints: its own, or from its
    /// base to its top level. `None` if a level does not exist.
    fn resolved_height(
        &self,
        level: LevelId,
        base_offset: f64,
        top: WallTop,
        height: f64,
    ) -> Option<f64> {
        match top {
            WallTop::Unconnected => Some(height),
            WallTop::Level { level: top, offset } => {
                let base = self.levels.get(&level)?.elevation + base_offset;
                Some(self.levels.get(&top)?.elevation + offset - base)
            }
        }
    }

    /// Checks a wall and returns the height it will have.
    fn validate_wall(&self, spec: &NewWall) -> Result<f64, ModelError> {
        if !self.levels.contains_key(&spec.level) {
            return Err(ModelError::UnknownLevel(spec.level));
        }
        if !self.wall_types.contains_key(&spec.wall_type) {
            return Err(ModelError::UnknownWallType(spec.wall_type));
        }
        finite_point(spec.start, "wall start")?;
        finite_point(spec.end, "wall end")?;
        finite(spec.height, "wall height")?;
        finite(spec.base_offset, "wall base offset")?;
        if spec.start.approx_eq(spec.end) {
            return Err(ModelError::ZeroLengthWall);
        }
        finite(spec.bulge, "wall bulge")?;
        if let Some(arc) = Arc::from_bulge(spec.start, spec.end, spec.bulge) {
            let t = self
                .wall_types
                .get(&spec.wall_type)
                .map_or(0.0, |t| t.thickness());
            let probe = Wall {
                id: WallId(0),
                guid: GlobalId::default(),
                wall_type: spec.wall_type,
                level: spec.level,
                start: spec.start,
                end: spec.end,
                bulge: spec.bulge,
                reference: spec.reference,
                height: spec.height,
                base_offset: spec.base_offset,
                top: spec.top,
                allow_join: [true, true],
                profile: None,
            };
            let (ext, int) = probe.face_offsets(t);
            let tight = arc.radius_at(ext).min(arc.radius_at(int)) < MIN_INNER_RADIUS;
            if spec.bulge.abs() > MAX_BULGE || tight {
                return Err(ModelError::ArcTooTight);
            }
        }
        if let WallTop::Level { level, offset } = spec.top {
            if !self.levels.contains_key(&level) {
                return Err(ModelError::UnknownLevel(level));
            }
            finite(offset, "wall top offset")?;
        }
        let height = self
            .resolved_height(spec.level, spec.base_offset, spec.top, spec.height)
            .ok_or(ModelError::UnknownLevel(spec.level))?;
        if height <= EPSILON {
            return Err(match spec.top {
                WallTop::Unconnected => ModelError::NonPositive("wall height"),
                WallTop::Level { .. } => ModelError::WallTopBelowBase(WallId(0)),
            });
        }
        Ok(height)
    }

    /// Moves a wall's end points, keeping its identity and GUID. Its doors
    /// and windows keep their distance from the wall start; the move is
    /// rejected if any of them would no longer fit.
    pub fn move_wall(&mut self, id: WallId, start: Point2, end: Point2) -> Result<(), ModelError> {
        finite_point(start, "wall start")?;
        finite_point(end, "wall end")?;
        if start.approx_eq(end) {
            return Err(ModelError::ZeroLengthWall);
        }
        let wall = self.walls.get_mut(&id).ok_or(ModelError::UnknownWall(id))?;
        let previous = (wall.start, wall.end);
        wall.start = start;
        wall.end = end;
        let check = self
            .check_arc(id)
            .and_then(|()| self.check_hosted_openings(id));
        if let Err(e) = check {
            let wall = self.walls.get_mut(&id).ok_or(ModelError::UnknownWall(id))?;
            (wall.start, wall.end) = previous;
            return Err(e);
        }
        Ok(())
    }

    /// Places a wall again with new ends and bulge, and its doors and
    /// windows at new distances from its start and with a new hand, all at
    /// once (moving, turning, scaling or mirroring a wall with what it
    /// hosts). Everything keeps its identity and GUID; if anything no
    /// longer fits, nothing changes.
    pub fn place_wall(
        &mut self,
        id: WallId,
        start: Point2,
        end: Point2,
        bulge: f64,
        openings: &[(OpeningId, f64, Hand)],
    ) -> Result<(), ModelError> {
        finite_point(start, "wall start")?;
        finite_point(end, "wall end")?;
        finite(bulge, "wall bulge")?;
        if start.approx_eq(end) {
            return Err(ModelError::ZeroLengthWall);
        }
        let wall = self.walls.get(&id).ok_or(ModelError::UnknownWall(id))?;
        let before_wall = wall.clone();
        let before_openings: Vec<Opening> = self
            .openings
            .values()
            .filter(|o| o.wall == id)
            .cloned()
            .collect();
        for (oid, along, hand) in openings {
            finite(*along, "opening position")?;
            match self.openings.get_mut(oid) {
                Some(o) if o.wall == id => {
                    o.along = *along;
                    o.hand = *hand;
                }
                _ => return Err(ModelError::UnknownOpening(*oid)),
            }
        }
        if let Some(w) = self.walls.get_mut(&id) {
            w.start = start;
            w.end = end;
            w.bulge = bulge;
        }
        let check = self
            .check_arc(id)
            .and_then(|()| self.check_hosted_openings(id));
        if let Err(e) = check {
            self.walls.insert(id, before_wall);
            for o in before_openings {
                self.openings.insert(o.id, o);
            }
            return Err(e);
        }
        Ok(())
    }

    /// Moves a wall's end points keeping its doors and windows where they
    /// are in the plan: each one is re-measured from the new start by
    /// projecting its old centre onto the new drawn line. This is what
    /// stretching a wall by one end does (ArchiCAD and Revit leave the
    /// openings in place). Rejected if any of them would no longer fit.
    pub fn stretch_wall(
        &mut self,
        id: WallId,
        start: Point2,
        end: Point2,
    ) -> Result<(), ModelError> {
        finite_point(start, "wall start")?;
        finite_point(end, "wall end")?;
        let wall = self.walls.get(&id).ok_or(ModelError::UnknownWall(id))?;
        let dir_old = (wall.end - wall.start)
            .normalized()
            .ok_or(ModelError::ZeroLengthWall)?;
        let dir_new = (end - start)
            .normalized()
            .ok_or(ModelError::ZeroLengthWall)?;
        let old_start = wall.start;
        let previous: Vec<(OpeningId, f64)> = self
            .openings
            .values()
            .filter(|o| o.wall == id)
            .map(|o| (o.id, o.along))
            .collect();
        for (oid, along) in &previous {
            let centre = old_start + dir_old * *along;
            if let Some(o) = self.openings.get_mut(oid) {
                o.along = (centre - start).dot(dir_new);
            }
        }
        if let Err(e) = self.move_wall(id, start, end) {
            for (oid, along) in previous {
                if let Some(o) = self.openings.get_mut(&oid) {
                    o.along = along;
                }
            }
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
        wall.bulge = -wall.bulge;
        let length = wall.length();
        for opening in self.openings.values_mut().filter(|o| o.wall == id) {
            opening.along = length - opening.along;
        }
        Ok(())
    }

    /// Applies an edit to a copy of a wall and keeps it only if the wall is
    /// still valid and every door and window it hosts still fits.
    fn edit_wall(&mut self, id: WallId, edit: impl FnOnce(&mut Wall)) -> Result<(), ModelError> {
        let previous = self
            .walls
            .get(&id)
            .cloned()
            .ok_or(ModelError::UnknownWall(id))?;
        let mut wall = previous.clone();
        edit(&mut wall);
        // A profile follows the wall: stretched with its length, turned
        // round with it, gone if it curves.
        if let Some(p) = wall.profile.take() {
            if !wall.is_curved() {
                let (l0, l1) = (previous.length(), wall.length());
                let turned =
                    wall.start.approx_eq(previous.end) && wall.end.approx_eq(previous.start);
                let p = if turned { p.reversed(l0) } else { p };
                wall.profile = Some(if l0 > EPSILON && (l1 - l0).abs() > 1e-9 {
                    p.stretched(l1 / l0)
                } else {
                    p
                });
            }
        }
        if let Some(p) = &wall.profile {
            wall_profile::check_profile(id, p)?;
        }
        wall.height = self
            .validate_wall(&NewWall {
                wall_type: wall.wall_type,
                level: wall.level,
                start: wall.start,
                end: wall.end,
                reference: wall.reference,
                height: wall.height,
                base_offset: wall.base_offset,
                bulge: wall.bulge,
                top: wall.top,
            })
            .map_err(|e| match e {
                ModelError::WallTopBelowBase(_) => ModelError::WallTopBelowBase(id),
                e => e,
            })?;
        self.walls.insert(id, wall);
        // Corner windows follow the corner (of this wall or of the walls
        // joined to it).
        let saved: Vec<(OpeningId, f64)> = self
            .openings
            .values()
            .filter(|o| o.corner_end().is_some())
            .map(|o| (o.id, o.along))
            .collect();
        self.snap_corner_openings();
        if let Err(e) = self.check_hosted_openings(id) {
            self.walls.insert(id, previous);
            for (oid, along) in saved {
                if let Some(o) = self.openings.get_mut(&oid) {
                    o.along = along;
                }
            }
            return Err(e);
        }
        Ok(())
    }

    /// Changes a wall's type (Revit "Type Selector"). The drawn line stays
    /// where it is; the doors and windows must still fit in the new
    /// thickness.
    pub fn set_wall_type(&mut self, id: WallId, wall_type: WallTypeId) -> Result<(), ModelError> {
        self.edit_wall(id, |w| w.wall_type = wall_type)
    }

    /// Changes a wall's height and the offset of its base above the level.
    /// A height typed by hand frees the wall from its top level, as
    /// typing a height does in ArchiCAD.
    pub fn set_wall_height(
        &mut self,
        id: WallId,
        height: f64,
        base_offset: f64,
    ) -> Result<(), ModelError> {
        self.edit_wall(id, |w| {
            w.height = height;
            w.base_offset = base_offset;
            w.top = WallTop::Unconnected;
        })
    }

    /// Changes the offset of a wall's base above its level. A wall tied to
    /// a top level keeps its top where it is (it grows or shrinks).
    pub fn set_wall_base_offset(&mut self, id: WallId, base_offset: f64) -> Result<(), ModelError> {
        self.edit_wall(id, |w| w.base_offset = base_offset)
    }

    /// Moves a wall to another level (Revit's "Restricción de base",
    /// ArchiCAD's home storey), with its doors and windows. It keeps its
    /// offset from the level, so it goes up or down with it. A wall tied to
    /// the level right above its old one is tied to the level right above
    /// the new one (it stays a full storey high); with none above, it keeps
    /// its height, free. Tied to another level, it keeps it, and the move
    /// is refused if that level is not above the new base.
    pub fn set_wall_level(&mut self, id: WallId, level: LevelId) -> Result<(), ModelError> {
        if !self.levels.contains_key(&level) {
            return Err(ModelError::UnknownLevel(level));
        }
        let wall = self.walls.get(&id).ok_or(ModelError::UnknownWall(id))?;
        if wall.level == level {
            return Ok(());
        }
        let next_above_old = self.level_above(wall.level);
        let top = match wall.top {
            WallTop::Level { level: top, offset } if Some(top) == next_above_old => {
                match self.level_above(level) {
                    Some(l) => WallTop::Level { level: l, offset },
                    None => WallTop::Unconnected,
                }
            }
            t => t,
        };
        self.edit_wall(id, |w| {
            w.level = level;
            w.top = top;
        })
    }

    /// Moves a slab to another level, keeping the height of its top face
    /// above the level.
    pub fn set_slab_level(&mut self, id: SlabId, level: LevelId) -> Result<(), ModelError> {
        if !self.levels.contains_key(&level) {
            return Err(ModelError::UnknownLevel(level));
        }
        let slab = self.slabs.get_mut(&id).ok_or(ModelError::UnknownSlab(id))?;
        slab.level = level;
        Ok(())
    }

    /// Changes a wall's top constraint. Freed from its level, the wall
    /// keeps the height it had.
    pub fn set_wall_top(&mut self, id: WallId, top: WallTop) -> Result<(), ModelError> {
        self.edit_wall(id, |w| w.top = top)
    }

    /// Works out again the height of every wall tied to a top level (after
    /// a level moved), refusing what no longer fits.
    fn refresh_wall_heights(&mut self) -> Result<(), ModelError> {
        let tied: Vec<WallId> = self
            .walls
            .values()
            .filter(|w| matches!(w.top, WallTop::Level { .. }))
            .map(|w| w.id)
            .collect();
        for id in tied {
            self.edit_wall(id, |_| {})?;
        }
        Ok(())
    }

    /// Bends a wall (or straightens it, with 0): its ends stay where they
    /// are and its line becomes the arc of that bulge (see [`Arc`]).
    pub fn set_wall_bulge(&mut self, id: WallId, bulge: f64) -> Result<(), ModelError> {
        self.edit_wall(id, |w| {
            w.bulge = if bulge.abs() < MIN_BULGE { 0.0 } else { bulge };
        })
    }

    /// Changes which line of the wall is the drawn one. The drawn line stays
    /// where it is and the wall body moves around it, so walls joined at
    /// their end points stay joined.
    pub fn set_wall_reference(
        &mut self,
        id: WallId,
        reference: ReferenceLine,
    ) -> Result<(), ModelError> {
        self.edit_wall(id, |w| w.reference = reference)
    }

    /// Revit's «Permitir unión» / «No permitir unión» at one end of a wall
    /// (`start`: its start, otherwise its end).
    pub fn set_wall_join(
        &mut self,
        id: WallId,
        start: bool,
        allow: bool,
    ) -> Result<(), ModelError> {
        self.edit_wall(id, |w| w.allow_join[if start { 0 } else { 1 }] = allow)
    }

    /// Removes a wall together with the doors and windows it hosts, as Revit
    /// and ArchiCAD do.
    pub fn remove_wall(&mut self, id: WallId) -> Result<Wall, ModelError> {
        let wall = self.walls.remove(&id).ok_or(ModelError::UnknownWall(id))?;
        self.openings.retain(|_, o| o.wall != id);
        self.wall_roofs.remove(&id);
        Ok(wall)
    }

    /// Whether a curved wall still has room for its inner face.
    fn check_arc(&self, id: WallId) -> Result<(), ModelError> {
        let w = self.walls.get(&id).ok_or(ModelError::UnknownWall(id))?;
        let Some(arc) = w.arc() else {
            return Ok(());
        };
        let t = self
            .wall_types
            .get(&w.wall_type)
            .map_or(0.0, |t| t.thickness());
        let (ext, int) = w.face_offsets(t);
        if w.bulge.abs() > MAX_BULGE
            || arc.radius_at(ext).min(arc.radius_at(int)) < MIN_INNER_RADIUS
        {
            return Err(ModelError::ArcTooTight);
        }
        Ok(())
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

    /// Redefines a wall type (Revit "Edit Type", ArchiCAD composite
    /// settings): name, function and layers. Every wall of the type changes
    /// with it, keeping its drawn line where it is. Rejected if a door or
    /// window in one of those walls would no longer fit, for example a frame
    /// deeper than the new thickness.
    pub fn edit_wall_type(
        &mut self,
        id: WallTypeId,
        name: impl Into<String>,
        function: WallFunction,
        layers: Vec<Layer>,
    ) -> Result<(), ModelError> {
        self.validate_layers(&layers)?;
        let previous = self
            .wall_types
            .get(&id)
            .cloned()
            .ok_or(ModelError::UnknownWallType(id))?;
        let mut edited = previous.clone();
        edited.name = name.into();
        edited.function = function;
        edited.layers = layers;
        self.wall_types.insert(id, edited);
        let walls: Vec<WallId> = self
            .walls
            .values()
            .filter(|w| w.wall_type == id)
            .map(|w| w.id)
            .collect();
        if let Err(e) = walls.iter().try_for_each(|w| {
            self.check_arc(*w)
                .and_then(|()| self.check_hosted_openings(*w))
        }) {
            self.wall_types.insert(id, previous);
            return Err(e);
        }
        Ok(())
    }

    /// How many walls use a wall type.
    pub fn wall_type_users(&self, id: WallTypeId) -> usize {
        self.walls.values().filter(|w| w.wall_type == id).count()
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
                edge: None,
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
        let users = self.slab_type_users(id);
        if users > 0 {
            return Err(ModelError::InUse(format!("{users} slab(s) use this type")));
        }
        self.slab_types
            .remove(&id)
            .ok_or(ModelError::UnknownSlabType(id))
    }

    /// Redefines a slab type: name, function and layers. Every slab of the
    /// type keeps its top face where it is and its layers hang from it.
    pub fn edit_slab_type(
        &mut self,
        id: SlabTypeId,
        name: impl Into<String>,
        function: SlabFunction,
        layers: Vec<Layer>,
    ) -> Result<(), ModelError> {
        self.validate_layers(&layers)?;
        let t = self
            .slab_types
            .get_mut(&id)
            .ok_or(ModelError::UnknownSlabType(id))?;
        t.name = name.into();
        t.function = function;
        t.layers = layers;
        Ok(())
    }

    /// How many slabs use a slab type.
    pub fn slab_type_users(&self, id: SlabTypeId) -> usize {
        self.slabs.values().filter(|s| s.slab_type == id).count()
            + self.roofs.values().filter(|r| r.roof_type == id).count()
    }

    // ---- Slabs --------------------------------------------------------

    /// Checks an outline and returns it counter-clockwise.
    pub(crate) fn normalized_outline(outline: Vec<Point2>) -> Result<Vec<Point2>, ModelError> {
        for p in &outline {
            finite_point(*p, "slab outline point")?;
        }
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
        finite(spec.top_offset, "slab top offset")?;
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
                holes: Vec::new(),
                slope: None,
            },
        );
        self.stamp_phase(id.0);
        Ok(id)
    }

    /// Replaces a slab's outline, keeping its identity and GUID. Its holes
    /// must still be inside.
    pub fn set_slab_outline(&mut self, id: SlabId, outline: Vec<Point2>) -> Result<(), ModelError> {
        let outline = Self::normalized_outline(outline)?;
        let slab = self.slabs.get_mut(&id).ok_or(ModelError::UnknownSlab(id))?;
        Self::check_holes(&outline, &slab.holes)?;
        slab.outline = outline;
        Ok(())
    }

    /// Moves a slab, with its holes, by `d`.
    pub fn move_slab(&mut self, id: SlabId, d: Vec2) -> Result<(), ModelError> {
        finite_point(d, "slab move")?;
        let slab = self.slabs.get_mut(&id).ok_or(ModelError::UnknownSlab(id))?;
        for p in slab
            .outline
            .iter_mut()
            .chain(slab.holes.iter_mut().flatten())
        {
            *p = *p + d;
        }
        if let Some(s) = &mut slab.slope {
            s.from = s.from + d;
            s.to = s.to + d;
        }
        Ok(())
    }

    /// Makes a slab inclined (or level again, with `None`).
    pub fn set_slab_slope(
        &mut self,
        id: SlabId,
        slope: Option<SlabSlope>,
    ) -> Result<(), ModelError> {
        if let Some(s) = &slope {
            s.validate()?;
        }
        let slab = self.slabs.get_mut(&id).ok_or(ModelError::UnknownSlab(id))?;
        slab.slope = slope;
        Ok(())
    }

    /// Replaces a slab's holes (each checked and stored counter-clockwise).
    pub fn set_slab_holes(
        &mut self,
        id: SlabId,
        holes: Vec<Vec<Point2>>,
    ) -> Result<(), ModelError> {
        let holes = holes
            .into_iter()
            .map(|h| Self::normalized_outline(h).map_err(|_| ModelError::InvalidHole))
            .collect::<Result<Vec<_>, _>>()?;
        let slab = self.slabs.get_mut(&id).ok_or(ModelError::UnknownSlab(id))?;
        Self::check_holes(&slab.outline, &holes)?;
        slab.holes = holes;
        Ok(())
    }

    /// Cuts a hole through a slab; returns its index.
    pub fn add_slab_hole(&mut self, id: SlabId, hole: Vec<Point2>) -> Result<usize, ModelError> {
        let mut holes = self
            .slabs
            .get(&id)
            .ok_or(ModelError::UnknownSlab(id))?
            .holes
            .clone();
        holes.push(hole);
        let n = holes.len() - 1;
        self.set_slab_holes(id, holes)?;
        Ok(n)
    }

    /// Fills a hole of a slab again.
    pub fn remove_slab_hole(&mut self, id: SlabId, index: usize) -> Result<(), ModelError> {
        let slab = self.slabs.get_mut(&id).ok_or(ModelError::UnknownSlab(id))?;
        if index >= slab.holes.len() {
            return Err(ModelError::InvalidHole);
        }
        slab.holes.remove(index);
        Ok(())
    }

    /// Every hole strictly inside the outline and clear of the others.
    fn check_holes(outline: &[Point2], holes: &[Vec<Point2>]) -> Result<(), ModelError> {
        let crosses = |a: &[Point2], b: &[Point2]| {
            (0..a.len()).any(|i| {
                (0..b.len()).any(|j| {
                    crate::math::segments_touch(
                        a[i],
                        a[(i + 1) % a.len()],
                        b[j],
                        b[(j + 1) % b.len()],
                    )
                })
            })
        };
        for (k, h) in holes.iter().enumerate() {
            if !h.iter().all(|p| polygon_contains(outline, *p)) || crosses(h, outline) {
                return Err(ModelError::InvalidHole);
            }
            for other in &holes[k + 1..] {
                let inside = other.iter().any(|p| polygon_contains(h, *p))
                    || h.iter().any(|p| polygon_contains(other, *p));
                if inside || crosses(h, other) {
                    return Err(ModelError::InvalidHole);
                }
            }
        }
        Ok(())
    }

    /// Changes a slab's type; the top face stays where it is.
    pub fn set_slab_type(&mut self, id: SlabId, slab_type: SlabTypeId) -> Result<(), ModelError> {
        if !self.slab_types.contains_key(&slab_type) {
            return Err(ModelError::UnknownSlabType(slab_type));
        }
        let slab = self.slabs.get_mut(&id).ok_or(ModelError::UnknownSlab(id))?;
        slab.slab_type = slab_type;
        Ok(())
    }

    /// Changes the height of a slab's top face above its level.
    pub fn set_slab_offset(&mut self, id: SlabId, top_offset: f64) -> Result<(), ModelError> {
        finite(top_offset, "slab top offset")?;
        let slab = self.slabs.get_mut(&id).ok_or(ModelError::UnknownSlab(id))?;
        slab.top_offset = top_offset;
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

    // ---- Families ---------------------------------------------------------

    /// Loads a family recipe into the project (its shared pieces already
    /// expanded). Family names are unique.
    pub fn add_family(&mut self, source: &str) -> Result<FamilyId, ModelError> {
        let id = FamilyId(self.next_id);
        let family = Family::new(id, source).map_err(ModelError::Family)?;
        if self.family_by_name(family.name()).is_some() {
            return Err(ModelError::DuplicateFamily(family.name().to_string()));
        }
        self.take_id();
        self.families.insert(id, family);
        Ok(id)
    }

    pub fn family(&self, id: FamilyId) -> Option<&Family> {
        self.families.get(&id)
    }

    pub fn families(&self) -> impl Iterator<Item = &Family> {
        self.families.values()
    }

    pub fn family_by_name(&self, name: &str) -> Option<&Family> {
        self.families.values().find(|f| f.name() == name)
    }

    /// The project's copy of a system family, loading it if needed.
    pub fn system_family(&mut self, name: &str) -> Result<FamilyId, ModelError> {
        if let Some(f) = self.family_by_name(name) {
            return Ok(f.id);
        }
        let source = family::system_family(name).ok_or_else(|| {
            ModelError::NotAnOpening(NotAnOpening {
                family: name.to_string(),
                reason: "there is no system family with that name".into(),
            })
        })?;
        self.add_family(&source)
    }

    /// The type values after a stretch by the handles: the family's
    /// `stretch` parameters (a sofa's seats, a table's chairs…) follow the
    /// new measurement in `values`. A family without them, or whose
    /// formulas fail, leaves the values as they are.
    pub fn follow_stretch(&self, family: FamilyId, values: &mut Values) {
        let Some(f) = self.families.get(&family) else {
            return;
        };
        if let Ok(changed) = f.def().evaluate_stretch(values) {
            values.extend(changed);
        }
    }

    /// Default values of one scope of a family, with material names turned
    /// into this project's materials (the first material when a name is not
    /// in the project).
    pub fn family_defaults(
        &self,
        family: FamilyId,
        scope: ParamScope,
    ) -> Result<Values, ModelError> {
        let def = self
            .families
            .get(&family)
            .ok_or(ModelError::UnknownFamily(family))?
            .def();
        let mut values = def.defaults(scope);
        for p in def.params_of(scope) {
            if p.kind != ParamKind::Material {
                continue;
            }
            let wanted = p.default.as_text().unwrap_or_default();
            let material = self
                .materials
                .values()
                .find(|m| m.name == wanted)
                .or_else(|| self.materials.values().next())
                .ok_or(ModelError::UnknownMaterial(MaterialId(0)))?;
            values.insert(p.name.clone(), Value::Material(material.id.0));
        }
        Ok(values)
    }

    // ---- Door and window types -----------------------------------------

    fn validate_opening_type(&self, spec: &NewOpeningType) -> Result<(), ModelError> {
        for m in [spec.frame_material, spec.panel_material] {
            if !self.materials.contains_key(&m) {
                return Err(ModelError::UnknownMaterial(m));
            }
        }
        let void = spec.kind == OpeningKind::Void;
        for (value, what) in [
            (spec.width, "opening width"),
            (spec.height, "opening height"),
            (spec.frame_width, "frame width"),
            (spec.frame_depth, "frame depth"),
        ] {
            finite(value, what)?;
            // A void has no frame.
            let frame = what.starts_with("frame");
            if value <= EPSILON && !(void && frame) {
                return Err(ModelError::NonPositive(what));
            }
        }
        // The frame must leave room for the leaf or the glass.
        if 2.0 * spec.frame_width >= spec.width || 2.0 * spec.frame_width >= spec.height {
            return Err(ModelError::NonPositive("clear opening inside the frame"));
        }
        Ok(())
    }

    /// The system family and the values that make an opening type
    /// described the pre-family way. Loads the family if needed. When it
    /// redefines `base`, a type of the same family, the family's other
    /// settings (such as how each leaf of a window opens) are kept unless
    /// the operation changes.
    fn legacy_values(
        &mut self,
        spec: &NewOpeningType,
        base: Option<OpeningTypeId>,
    ) -> Result<(FamilyId, Values), ModelError> {
        let (name, extras) = family::legacy_family(spec.kind);
        let id = self.system_family(name)?;
        let base = base
            .and_then(|b| self.opening_types.get(&b))
            .filter(|b| b.family == id)
            .cloned();
        let keep = base.as_ref().is_some_and(|b| b.kind == spec.kind);
        let mut values = match &base {
            Some(b) => b.values.clone(),
            None => self.family_defaults(id, ParamScope::Type)?,
        };
        // Only what the family has (a void has no frame).
        let has = |k: &str| self.families[&id].def().param(k).is_some();
        for (k, v) in [
            ("width", Value::Number(spec.width)),
            ("height", Value::Number(spec.height)),
            ("frame_width", Value::Number(spec.frame_width)),
            ("frame_depth", Value::Number(spec.frame_depth)),
            ("frame_material", Value::Material(spec.frame_material.0)),
            ("panel_material", Value::Material(spec.panel_material.0)),
        ] {
            if has(k) {
                values.insert(k.into(), v);
            }
        }
        if !keep {
            for (k, v) in extras {
                values.insert(k.into(), v);
            }
        }
        Ok((id, values))
    }

    /// Runs a legacy edit, unloading a system family it loaded if it fails.
    fn with_legacy_values<T>(
        &mut self,
        spec: &NewOpeningType,
        base: Option<OpeningTypeId>,
        edit: impl FnOnce(&mut Self, FamilyId, Values) -> Result<T, ModelError>,
    ) -> Result<T, ModelError> {
        self.validate_opening_type(spec)?;
        let before: Vec<FamilyId> = self.families.keys().copied().collect();
        let (family, values) = self.legacy_values(spec, base)?;
        let result = edit(self, family, values);
        if result.is_err() {
            self.families.retain(|id, _| before.contains(id));
        }
        result
    }

    /// Works out what a family type gives the rest of the program (rough
    /// opening, frame, materials, operation), checking that the family makes
    /// doors or windows and that the values are valid for it.
    fn resolve_opening_type(
        &self,
        family: FamilyId,
        values: &Values,
    ) -> Result<Resolved, ModelError> {
        let fam = self
            .families
            .get(&family)
            .ok_or(ModelError::UnknownFamily(family))?;
        let not_opening = |reason: String| {
            ModelError::NotAnOpening(NotAnOpening {
                family: fam.name().to_string(),
                reason,
            })
        };
        if !fam.is_opening() {
            return Err(not_opening(
                "it is not a door or window hosted in a wall".into(),
            ));
        }
        for v in values.values() {
            if let Value::Material(m) = v {
                if !self.materials.contains_key(&MaterialId(*m)) {
                    return Err(ModelError::UnknownMaterial(MaterialId(*m)));
                }
            }
        }
        let instance = self.family_defaults(family, ParamScope::Instance)?;
        let builtins = opening_builtins(WallSide::Interior, Hand::Left);
        let ev = fam
            .def()
            .evaluate(values, &instance, &builtins)
            .map_err(ModelError::Family)?;
        let missing = |name: &str| not_opening(format!("it gives no «{name}» output"));
        let number = |name: &str| {
            ev.outputs
                .get(name)
                .and_then(Value::as_number)
                .ok_or_else(|| missing(name))
        };
        let material = |name: &str| {
            ev.outputs
                .get(name)
                .and_then(Value::as_material)
                .map(MaterialId)
                .ok_or_else(|| missing(name))
        };
        let operation = ev
            .outputs
            .get("operation")
            .and_then(Value::as_text)
            .ok_or_else(|| missing("operation"))?;
        let kind = family::operation_from_name(&fam.def().category, operation)
            .ok_or_else(|| not_opening(format!("unknown operation «{operation}»")))?;
        // A void has no frame or leaves: any material stands in for theirs.
        let any_material = || {
            self.materials
                .keys()
                .next()
                .copied()
                .ok_or_else(|| not_opening("the project has no materials".into()))
        };
        let void = kind == OpeningKind::Void;
        let material_or_any = |name: &str| match material(name) {
            Err(_) if void => any_material(),
            other => other,
        };
        let spec = NewOpeningType {
            name: String::new(),
            kind,
            width: number("opening_width")?,
            height: number("opening_height")?,
            frame_width: number("frame_width")?,
            frame_depth: number("frame_depth")?,
            frame_material: material_or_any("frame_material")?,
            panel_material: material_or_any("panel_material")?,
        };
        self.validate_opening_type(&spec)?;
        // How each window leaf opens, if the family says (its `ifc`
        // section: `panel_count` and `panel_1`, `panel_2`…).
        let panels = match ev.ifc.get("panel_count").and_then(Value::as_number) {
            Some(n) if kind_is_window(kind) => {
                let n = n as usize;
                if !(1..=8).contains(&n) {
                    return Err(not_opening(format!("{n} leaves cannot be exported")));
                }
                (1..=n)
                    .map(|i| {
                        let key = format!("panel_{i}");
                        ev.ifc
                            .get(&key)
                            .and_then(Value::as_text)
                            .filter(|op| family::PANEL_OPERATIONS.contains(op))
                            .map(str::to_string)
                            .ok_or_else(|| {
                                not_opening(format!("«{key}» is not a window leaf operation"))
                            })
                    })
                    .collect::<Result<Vec<_>, _>>()?
            }
            _ => family::legacy_panels(kind),
        };
        // The shape of the rough opening: square unless the recipe says.
        let shape = match ev.outputs.get("opening_shape") {
            None => archisim_family::OpeningShape::RECT,
            Some(v) => {
                let kind = v
                    .as_text()
                    .and_then(archisim_family::ShapeKind::from_name)
                    .ok_or_else(|| not_opening("«opening_shape» is not a shape".into()))?;
                let rise = ev
                    .outputs
                    .get("opening_rise")
                    .and_then(Value::as_number)
                    .unwrap_or(0.0);
                archisim_family::OpeningShape::new(kind, rise)
            }
        };
        // Where its frame sits in the wall: in the middle unless the
        // recipe says.
        let inset = ev
            .outputs
            .get("frame_inset")
            .and_then(Value::as_number)
            .unwrap_or(0.0)
            .max(0.0);
        let frame_place = match ev.outputs.get("frame_align").and_then(Value::as_text) {
            Some("exterior") => FramePlace::Exterior(inset),
            Some("interior") => FramePlace::Interior(inset),
            _ => FramePlace::Centre,
        };
        Ok(Resolved {
            spec,
            panels,
            shape,
            frame_place,
        })
    }

    fn check_mark(&self, mark: &str, except: Option<OpeningTypeId>) -> Result<(), ModelError> {
        let taken = !mark.is_empty()
            && self
                .opening_types
                .values()
                .any(|t| Some(t.id) != except && t.mark == mark);
        if taken {
            Err(ModelError::DuplicateMark(mark.to_string()))
        } else {
            Ok(())
        }
    }

    /// The next free mark: "P-01", "P-02"… for doors, "V-01"… for windows.
    pub fn next_mark(&self, kind: OpeningKind) -> String {
        let prefix = match kind {
            OpeningKind::Door(_) => "P",
            OpeningKind::Window(_) => "V",
            OpeningKind::Void => "H",
        };
        (1..)
            .map(|n| format!("{prefix}-{n:02}"))
            .find(|m| self.check_mark(m, None).is_ok())
            .expect("there is always a free mark")
    }

    /// Creates a type of a door or window family.
    pub fn add_family_type(&mut self, spec: NewFamilyType) -> Result<OpeningTypeId, ModelError> {
        let Resolved {
            spec: r,
            panels,
            shape,
            frame_place,
        } = self.resolve_opening_type(spec.family, &spec.values)?;
        let mark = match spec.mark {
            Some(m) => {
                self.check_mark(&m, None)?;
                m
            }
            None => self.next_mark(r.kind),
        };
        let id = OpeningTypeId(self.take_id());
        self.opening_types.insert(
            id,
            OpeningType {
                id,
                guid: GlobalId::new(),
                name: spec.name,
                mark,
                description: spec.description,
                family: spec.family,
                values: spec.values,
                kind: r.kind,
                width: r.width,
                height: r.height,
                frame_width: r.frame_width,
                frame_depth: r.frame_depth,
                frame_material: r.frame_material,
                panel_material: r.panel_material,
                panels,
                shape,
                frame_place,
            },
        );
        Ok(id)
    }

    /// Creates a door or window type from its operation and sizes, as a type
    /// of the matching system family, with the next free mark.
    pub fn add_opening_type(&mut self, spec: NewOpeningType) -> Result<OpeningTypeId, ModelError> {
        let name = spec.name.clone();
        self.with_legacy_values(&spec, None, |m, family, values| {
            m.add_family_type(NewFamilyType {
                family,
                name,
                mark: None,
                description: String::new(),
                values,
            })
        })
    }

    /// A copy of a type with a new name, GUID and mark (Revit's
    /// "Duplicate…").
    pub fn duplicate_opening_type(
        &mut self,
        id: OpeningTypeId,
        name: impl Into<String>,
    ) -> Result<OpeningTypeId, ModelError> {
        let src = self
            .opening_types
            .get(&id)
            .ok_or(ModelError::UnknownOpeningType(id))?
            .clone();
        self.add_family_type(NewFamilyType {
            family: src.family,
            name: name.into(),
            mark: None,
            description: src.description,
            values: src.values,
        })
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

    /// Changes the name, mark and description of a type.
    pub fn set_opening_type_info(
        &mut self,
        id: OpeningTypeId,
        name: impl Into<String>,
        mark: impl Into<String>,
        description: impl Into<String>,
    ) -> Result<(), ModelError> {
        let mark = mark.into();
        self.check_mark(&mark, Some(id))?;
        let t = self
            .opening_types
            .get_mut(&id)
            .ok_or(ModelError::UnknownOpeningType(id))?;
        t.name = name.into();
        t.mark = mark;
        t.description = description.into();
        Ok(())
    }

    /// Redefines a type with a family and its values. Every door or window
    /// of the type changes with it, keeping its centre, sill and GUID. A
    /// door type stays a door and a window type a window. Rejected if one
    /// of them would no longer fit in its wall.
    pub fn edit_family_type(
        &mut self,
        id: OpeningTypeId,
        family: FamilyId,
        values: Values,
    ) -> Result<(), ModelError> {
        let Resolved {
            spec: r,
            panels,
            shape,
            frame_place,
        } = self.resolve_opening_type(family, &values)?;
        let previous = self
            .opening_types
            .get(&id)
            .cloned()
            .ok_or(ModelError::UnknownOpeningType(id))?;
        let edited = OpeningType {
            family,
            values,
            kind: r.kind,
            width: r.width,
            height: r.height,
            frame_width: r.frame_width,
            frame_depth: r.frame_depth,
            frame_material: r.frame_material,
            panel_material: r.panel_material,
            panels,
            shape,
            frame_place,
            ..previous.clone()
        };
        if edited.is_door() != previous.is_door() {
            return Err(ModelError::CategoryMismatch);
        }
        let family_changed = previous.family != family;
        self.opening_types.insert(id, edited);
        let users: Vec<OpeningId> = self
            .openings
            .values()
            .filter(|o| o.opening_type == id)
            .map(|o| o.id)
            .collect();
        let instance = self.family_defaults(family, ParamScope::Instance);
        let check = users.iter().try_for_each(|oid| {
            let o = &self.openings[oid];
            let spec = NewOpening {
                opening_type: o.opening_type,
                wall: o.wall,
                along: o.along,
                sill: o.sill,
                opens_to: o.opens_to,
                hand: o.hand,
                setback: o.setback,
            };
            self.validate_opening(&spec, Some(o.id))
        });
        let check = check.and(instance);
        match check {
            Err(e) => {
                self.opening_types.insert(id, previous);
                Err(e)
            }
            Ok(instance) => {
                if family_changed {
                    // Another family has other instance parameters.
                    for oid in users {
                        if let Some(o) = self.openings.get_mut(&oid) {
                            o.values = instance.clone();
                        }
                    }
                }
                self.snap_corner_openings();
                Ok(())
            }
        }
    }

    /// Redefines a door or window type from its operation and sizes (the
    /// name included), as a type of the matching system family. Every door
    /// or window of the type changes with it, keeping its centre and sill.
    /// A door type stays a door and a window type a window. Rejected if one
    /// of them would no longer fit in its wall (outside the wall,
    /// overlapping another opening, frame too deep).
    pub fn edit_opening_type(
        &mut self,
        id: OpeningTypeId,
        spec: NewOpeningType,
    ) -> Result<(), ModelError> {
        if !self.opening_types.contains_key(&id) {
            return Err(ModelError::UnknownOpeningType(id));
        }
        let name = spec.name.clone();
        self.with_legacy_values(&spec, Some(id), |m, family, values| {
            m.edit_family_type(id, family, values)?;
            if let Some(t) = m.opening_types.get_mut(&id) {
                t.name = name;
            }
            Ok(())
        })
    }

    /// How many doors or windows use a type.
    pub fn opening_type_users(&self, id: OpeningTypeId) -> usize {
        self.openings
            .values()
            .filter(|o| o.opening_type == id)
            .count()
    }

    /// Evaluates the family of a placed door or window: its 3D body (in the
    /// family's own axes) and outputs.
    pub fn evaluate_opening(&self, id: OpeningId) -> Result<Evaluation, ModelError> {
        let o = self
            .openings
            .get(&id)
            .ok_or(ModelError::UnknownOpening(id))?;
        let t = self
            .opening_types
            .get(&o.opening_type)
            .ok_or(ModelError::UnknownOpeningType(o.opening_type))?;
        let fam = self
            .families
            .get(&t.family)
            .ok_or(ModelError::UnknownFamily(t.family))?;
        fam.def()
            .evaluate(&t.values, &o.values, &self.placed_builtins(o, t))
            .map_err(ModelError::Family)
    }

    /// The values the program gives the recipe of a placed door or window:
    /// how it opens, and where its wall's faces are in the family's axes
    /// (`wall_ext`, `wall_int`) and its sill.
    fn placed_builtins(&self, o: &Opening, t: &OpeningType) -> Values {
        let mut b = opening_builtins(o.opens_to, o.hand);
        let thickness = self
            .walls
            .get(&o.wall)
            .and_then(|w| self.wall_types.get(&w.wall_type))
            .map(|wt| wt.thickness());
        if let Some(th) = thickness {
            // The frame's middle is `setback + depth / 2` in from the
            // exterior face; +y points to the side the leaves open to.
            let ext = o.frame_setback(th, t) + t.frame_depth / 2.0;
            let int = th - ext;
            let (e, i) = match o.opens_to {
                WallSide::Interior => (-ext, int),
                WallSide::Exterior => (ext, -int),
            };
            b.insert("wall_ext".into(), Value::Number(e));
            b.insert("wall_int".into(), Value::Number(i));
        }
        b.insert("sill_height".into(), Value::Number(o.sill));
        b
    }

    /// The family of a placed door or window evaluated for the detailed
    /// 3D: its body without what the family's generator makes (the
    /// leaves, with `detailed` on), and its outputs, which tell the
    /// generator where the leaves go.
    pub fn evaluate_opening_detailed(&self, id: OpeningId) -> Result<Evaluation, ModelError> {
        let o = self
            .openings
            .get(&id)
            .ok_or(ModelError::UnknownOpening(id))?;
        let t = self
            .opening_types
            .get(&o.opening_type)
            .ok_or(ModelError::UnknownOpeningType(o.opening_type))?;
        let fam = self
            .families
            .get(&t.family)
            .ok_or(ModelError::UnknownFamily(t.family))?;
        let mut builtins = self.placed_builtins(o, t);
        builtins.insert("detailed".into(), Value::Bool(true));
        fam.def()
            .evaluate(&t.values, &o.values, &builtins)
            .map_err(ModelError::Family)
    }

    /// The plan symbol of a placed door or window for a drawing scale
    /// (denominator), in the family's own axes. `wall_ext` and `wall_int`
    /// are where the wall's faces are in those axes (see
    /// [`crate::family::PLAN_BUILTINS`]); the plan view works them out.
    pub fn opening_plan_symbol(
        &self,
        id: OpeningId,
        scale: f64,
        wall_ext: f64,
        wall_int: f64,
    ) -> Result<Vec<Mark>, ModelError> {
        let o = self
            .openings
            .get(&id)
            .ok_or(ModelError::UnknownOpening(id))?;
        let t = self
            .opening_types
            .get(&o.opening_type)
            .ok_or(ModelError::UnknownOpeningType(o.opening_type))?;
        let fam = self
            .families
            .get(&t.family)
            .ok_or(ModelError::UnknownFamily(t.family))?;
        let mut builtins = opening_builtins(o.opens_to, o.hand);
        builtins.insert("wall_ext".into(), Value::Number(wall_ext));
        builtins.insert("wall_int".into(), Value::Number(wall_int));
        builtins.insert("sill_height".into(), Value::Number(o.sill));
        fam.def()
            .evaluate_plan(&t.values, &o.values, &builtins, scale)
            .map_err(ModelError::Family)
    }

    /// A door or window type's family evaluated with the default instance
    /// values (opening to the interior, left hand): its `outputs` and its
    /// `ifc` section, for values that belong to the type (the clear width,
    /// the IFC partitioning…).
    pub fn opening_type_evaluation(&self, id: OpeningTypeId) -> Option<Evaluation> {
        let t = self.opening_types.get(&id)?;
        let fam = self.families.get(&t.family)?;
        let instance = self.family_defaults(t.family, ParamScope::Instance).ok()?;
        fam.def()
            .evaluate(
                &t.values,
                &instance,
                &opening_builtins(WallSide::Interior, Hand::Left),
            )
            .ok()
    }

    /// The clear width of a door (its leaf, without sidelights or the
    /// pivot's short side), when its family says (`clear_width` output).
    pub fn opening_clear_width(&self, id: OpeningTypeId) -> Option<f64> {
        self.opening_type_evaluation(id)?
            .outputs
            .get("clear_width")
            .and_then(Value::as_number)
    }

    /// The elevation marks of a placed door or window (opening lines), in
    /// the family's `x` and `z`, seen from the side its leaves open to.
    pub fn opening_elevation_marks(&self, id: OpeningId) -> Result<Vec<Mark>, ModelError> {
        let o = self
            .openings
            .get(&id)
            .ok_or(ModelError::UnknownOpening(id))?;
        let t = self
            .opening_types
            .get(&o.opening_type)
            .ok_or(ModelError::UnknownOpeningType(o.opening_type))?;
        let fam = self
            .families
            .get(&t.family)
            .ok_or(ModelError::UnknownFamily(t.family))?;
        let mut builtins = opening_builtins(o.opens_to, o.hand);
        builtins.insert("sill_height".into(), Value::Number(o.sill));
        fam.def()
            .evaluate_elevation(&t.values, &o.values, &builtins)
            .map_err(ModelError::Family)
    }

    /// Whether the opening lines of a door or window are drawn in the
    /// model's sections and elevations. A recipe may declare the type
    /// parameter [`OPENING_LINES_IN_VIEWS`] (the windows do, off by default:
    /// an architect draws them only where they explain something); without
    /// it they are always drawn. The type's own drawings (type dialog,
    /// carpentry sheets) always show them.
    pub fn opening_lines_in_views(&self, id: OpeningId) -> bool {
        let Some(t) = self
            .openings
            .get(&id)
            .and_then(|o| self.opening_types.get(&o.opening_type))
        else {
            return true;
        };
        !matches!(
            t.values.get(OPENING_LINES_IN_VIEWS),
            Some(Value::Bool(false))
        )
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
        let corner = ignore
            .and_then(|id| self.openings.get(&id))
            .and_then(Opening::corner_end);
        self.validate_opening_at(spec, ignore, corner)
    }

    /// [`Model::validate_opening`] for a door or window that turns a
    /// corner (`corner`, see [`Opening::corner_end`]) or not.
    fn validate_opening_at(
        &self,
        spec: &NewOpening,
        ignore: Option<OpeningId>,
        corner: Option<bool>,
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
        // In a curved wall the door or window is straight, square to the
        // wall at its centre: the curve must not part from its ends by more
        // than half the wall; and it cannot turn a corner.
        if let Some(arc) = wall.arc() {
            let half = otype.width / 2.0;
            let r = arc.radius;
            let sag = if half < r {
                r - (r * r - half * half).sqrt()
            } else {
                f64::INFINITY
            };
            if corner.is_some() || sag > thickness / 2.0 + EPSILON {
                return Err(ModelError::OpeningInCurvedWall);
            }
        }
        finite(spec.along, "opening position")?;
        finite(spec.sill, "sill height")?;
        if let Some(s) = spec.setback {
            finite(s, "frame setback")?;
        }

        let (a, b) = (
            spec.along - otype.width / 2.0,
            spec.along + otype.width / 2.0,
        );
        let (bottom, top) = (spec.sill, spec.sill + otype.height);
        let wall_top = wall.base_offset + wall.height;
        // A corner window may reach past the end of the wall's line (into
        // the corner), by as much as the wall is thick.
        let (lo, hi) = match corner {
            Some(false) => (-thickness, wall.length()),
            Some(true) => (0.0, wall.length() + thickness),
            None => (0.0, wall.length()),
        };
        if a < lo - EPSILON
            || b > hi + EPSILON
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
        let family = self.opening_types[&spec.opening_type].family;
        let values = self.family_defaults(family, ParamScope::Instance)?;
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
                values,
            },
        );
        self.stamp_phase(id.0);
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
        self.snap_corner_openings();
        Ok(())
    }

    /// Where a corner window's jamb goes on its wall's line: where the
    /// wall's middle meets the middle of the wall joined at that end (or
    /// the end itself if none is), as a distance from the wall start.
    fn corner_position(&self, wall: WallId, end: bool) -> Option<f64> {
        let w = self.walls.get(&wall)?;
        let t = self.wall_types.get(&w.wall_type)?.thickness();
        let (ext, int) = w.face_offsets(t);
        let len = w.length();
        let u = (w.end - w.start).normalized()?;
        let n = u.perp_left();
        let p = if end { w.end } else { w.start };
        let fallback = if end { len } else { 0.0 };
        let other = self.walls.values().find(|o| {
            o.id != wall
                && o.level == w.level
                && !o.is_curved()
                && (o.start.approx_eq(p) || o.end.approx_eq(p))
                && w.joins_at(p)
                && o.joins_at(p)
        });
        let Some(o) = other else {
            return Some(fallback);
        };
        let ot = self.wall_types.get(&o.wall_type)?.thickness();
        let (oe, oi) = o.face_offsets(ot);
        let Some(u2) = (o.end - o.start).normalized() else {
            return Some(fallback);
        };
        let n2 = u2.perp_left();
        // This wall's middle: start + n·m + u·t; the other's: q + u2·s.
        let m = (ext + int) / 2.0;
        let q = o.start + n2 * ((oe + oi) / 2.0);
        let origin = w.start + n * m;
        let cross = u.x * u2.y - u.y * u2.x;
        if cross.abs() < 1e-9 {
            return Some(fallback);
        }
        let d = q - origin;
        let t_c = (d.x * u2.y - d.y * u2.x) / cross;
        // Only near the end (a wall crossing far away is not this corner).
        if (t_c - fallback).abs() > t.max(ot) * 1.5 {
            return Some(fallback);
        }
        Some(t_c)
    }

    /// Puts every corner window with its corner jamb where its wall meets
    /// the next one (see [`Opening::corner_end`]). Called after anything
    /// that moves walls or changes doors and windows.
    pub fn snap_corner_openings(&mut self) {
        let corners: Vec<(OpeningId, WallId, bool, f64)> = self
            .openings
            .values()
            .filter_map(|o| {
                let end = o.corner_end()?;
                let w = self.opening_types.get(&o.opening_type)?.width;
                Some((o.id, o.wall, end, w))
            })
            .collect();
        for (id, wall, end, width) in corners {
            let Some(t) = self.corner_position(wall, end) else {
                continue;
            };
            let along = if end {
                t - width / 2.0
            } else {
                t + width / 2.0
            };
            if let Some(o) = self.openings.get_mut(&id) {
                o.along = along;
            }
        }
    }

    /// Changes the instance parameters of a door or window (those of its
    /// type's family that belong to each placed element).
    pub fn set_opening_values(&mut self, id: OpeningId, values: Values) -> Result<(), ModelError> {
        let o = self
            .openings
            .get(&id)
            .ok_or(ModelError::UnknownOpening(id))?;
        let family = self
            .opening_types
            .get(&o.opening_type)
            .ok_or(ModelError::UnknownOpeningType(o.opening_type))?
            .family;
        self.families
            .get(&family)
            .ok_or(ModelError::UnknownFamily(family))?
            .def()
            .check_values(ParamScope::Instance, &values)
            .map_err(ModelError::Family)?;
        let previous = std::mem::replace(
            &mut self.openings.get_mut(&id).expect("checked").values,
            values,
        );
        if let Err(e) = self.evaluate_opening(id) {
            self.openings.get_mut(&id).expect("checked").values = previous;
            return Err(e);
        }
        // A window that becomes a corner one goes to its corner.
        let before = self.openings[&id].along;
        self.snap_corner_openings();
        let o = &self.openings[&id];
        let spec = NewOpening {
            opening_type: o.opening_type,
            wall: o.wall,
            along: o.along,
            sill: o.sill,
            opens_to: o.opens_to,
            hand: o.hand,
            setback: o.setback,
        };
        if let Err(e) = self.validate_opening(&spec, Some(id)) {
            let o = self.openings.get_mut(&id).expect("checked");
            o.values = previous;
            o.along = before;
            return Err(e);
        }
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
        self.snap_corner_openings();
        Ok(())
    }

    /// Swaps a door or window for another type of the same category (a door
    /// for a door, a window for a window), keeping its position and GUID.
    pub fn set_opening_type(
        &mut self,
        id: OpeningId,
        opening_type: OpeningTypeId,
    ) -> Result<(), ModelError> {
        let o = self
            .openings
            .get(&id)
            .ok_or(ModelError::UnknownOpening(id))?;
        let new_type = self
            .opening_types
            .get(&opening_type)
            .ok_or(ModelError::UnknownOpeningType(opening_type))?;
        let old_type = self
            .opening_types
            .get(&o.opening_type)
            .ok_or(ModelError::UnknownOpeningType(o.opening_type))?;
        if new_type.is_door() != old_type.is_door() {
            return Err(ModelError::CategoryMismatch);
        }
        let new_values = if new_type.family == old_type.family {
            None
        } else {
            Some(self.family_defaults(new_type.family, ParamScope::Instance)?)
        };
        let spec = NewOpening {
            opening_type,
            wall: o.wall,
            along: o.along,
            sill: o.sill,
            opens_to: o.opens_to,
            hand: o.hand,
            setback: o.setback,
        };
        self.validate_opening(&spec, Some(id))?;
        let o = self
            .openings
            .get_mut(&id)
            .ok_or(ModelError::UnknownOpening(id))?;
        o.opening_type = opening_type;
        if let Some(values) = new_values {
            // Another family has other instance parameters.
            o.values = values;
        }
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

    // ---- Dimensions and level marks ---------------------------------------

    fn validate_dimension(&self, spec: &NewDimension) -> Result<Point2, ModelError> {
        if !self.levels.contains_key(&spec.level) {
            return Err(ModelError::UnknownLevel(spec.level));
        }
        for p in &spec.points {
            finite_point(*p, "dimension point")?;
        }
        finite_point(spec.direction, "dimension direction")?;
        finite_point(spec.position, "dimension position")?;
        let dir = spec
            .direction
            .normalized()
            .ok_or(ModelError::DegenerateDimension)?;
        if stations(&spec.points, dir).len() < 2 {
            return Err(ModelError::DegenerateDimension);
        }
        Ok(dir)
    }

    pub fn add_dimension(&mut self, spec: NewDimension) -> Result<DimensionId, ModelError> {
        let direction = self.validate_dimension(&spec)?;
        let id = DimensionId(self.take_id());
        self.dimensions.insert(
            id,
            Dimension {
                id,
                level: spec.level,
                points: spec.points,
                direction,
                position: spec.position,
                anchors: Vec::new(),
            },
        );
        Ok(id)
    }

    /// Ties a dimension's points to elements (one entry per point, or
    /// none to free them all).
    pub fn set_dimension_anchors(
        &mut self,
        id: DimensionId,
        anchors: Vec<Option<Anchor>>,
    ) -> Result<(), ModelError> {
        let d = self
            .dimensions
            .get_mut(&id)
            .ok_or(ModelError::UnknownDimension(id))?;
        if !anchors.is_empty() && anchors.len() != d.points.len() {
            return Err(ModelError::DegenerateDimension);
        }
        d.anchors = if anchors.iter().all(Option::is_none) {
            Vec::new()
        } else {
            anchors
        };
        Ok(())
    }

    /// Replaces a dimension's points, direction and line position.
    pub fn set_dimension(
        &mut self,
        id: DimensionId,
        points: Vec<Point2>,
        position: Point2,
    ) -> Result<(), ModelError> {
        let d = self
            .dimensions
            .get(&id)
            .ok_or(ModelError::UnknownDimension(id))?;
        let spec = NewDimension {
            level: d.level,
            points,
            direction: d.direction,
            position,
        };
        self.validate_dimension(&spec)?;
        let d = self
            .dimensions
            .get_mut(&id)
            .ok_or(ModelError::UnknownDimension(id))?;
        if spec.points.len() != d.points.len() {
            d.anchors.clear();
        }
        d.points = spec.points;
        d.position = spec.position;
        Ok(())
    }

    /// Replaces a dimension's points, measuring direction and line position
    /// (turning or mirroring it).
    pub fn reshape_dimension(
        &mut self,
        id: DimensionId,
        points: Vec<Point2>,
        direction: Point2,
        position: Point2,
    ) -> Result<(), ModelError> {
        let d = self
            .dimensions
            .get(&id)
            .ok_or(ModelError::UnknownDimension(id))?;
        let spec = NewDimension {
            level: d.level,
            points,
            direction,
            position,
        };
        let dir = self.validate_dimension(&spec)?;
        let d = self
            .dimensions
            .get_mut(&id)
            .ok_or(ModelError::UnknownDimension(id))?;
        if spec.points.len() != d.points.len() {
            d.anchors.clear();
        }
        d.points = spec.points;
        d.direction = dir;
        d.position = spec.position;
        Ok(())
    }

    pub fn remove_dimension(&mut self, id: DimensionId) -> Result<Dimension, ModelError> {
        self.dimensions
            .remove(&id)
            .ok_or(ModelError::UnknownDimension(id))
    }

    pub fn dimension(&self, id: DimensionId) -> Option<&Dimension> {
        self.dimensions.get(&id)
    }

    pub fn dimensions(&self) -> impl Iterator<Item = &Dimension> {
        self.dimensions.values()
    }

    pub fn dimensions_on_level(&self, level: LevelId) -> impl Iterator<Item = &Dimension> {
        self.dimensions.values().filter(move |d| d.level == level)
    }

    pub fn add_elevation_mark(
        &mut self,
        level: LevelId,
        point: Point2,
    ) -> Result<ElevationMarkId, ModelError> {
        if !self.levels.contains_key(&level) {
            return Err(ModelError::UnknownLevel(level));
        }
        finite_point(point, "level mark point")?;
        let id = ElevationMarkId(self.take_id());
        self.elevation_marks
            .insert(id, ElevationMark { id, level, point });
        Ok(id)
    }

    pub fn move_elevation_mark(
        &mut self,
        id: ElevationMarkId,
        point: Point2,
    ) -> Result<(), ModelError> {
        finite_point(point, "level mark point")?;
        let m = self
            .elevation_marks
            .get_mut(&id)
            .ok_or(ModelError::UnknownElevationMark(id))?;
        m.point = point;
        Ok(())
    }

    pub fn remove_elevation_mark(
        &mut self,
        id: ElevationMarkId,
    ) -> Result<ElevationMark, ModelError> {
        self.elevation_marks
            .remove(&id)
            .ok_or(ModelError::UnknownElevationMark(id))
    }

    pub fn elevation_mark(&self, id: ElevationMarkId) -> Option<&ElevationMark> {
        self.elevation_marks.get(&id)
    }

    pub fn elevation_marks(&self) -> impl Iterator<Item = &ElevationMark> {
        self.elevation_marks.values()
    }

    pub fn elevation_marks_on_level(&self, level: LevelId) -> impl Iterator<Item = &ElevationMark> {
        self.elevation_marks
            .values()
            .filter(move |m| m.level == level)
    }

    /// The finished floor elevation at a point of a level, from project
    /// zero: the top face of the highest slab of the level whose outline
    /// contains the point, or the level's elevation.
    pub fn floor_elevation_at(&self, level: LevelId, p: Point2) -> Option<f64> {
        let base = self.levels.get(&level)?.elevation;
        let top = self
            .slabs_on_level(level)
            .filter(|s| s.covers(p))
            .map(|s| s.top_at(p))
            .fold(None, |acc: Option<f64>, v| {
                Some(acc.map_or(v, |a| a.max(v)))
            });
        Some(base + top.unwrap_or(0.0))
    }

    // ---- Saving and opening ---------------------------------------------

    /// Every element of the model, each list ordered by id.
    pub fn to_parts(&self) -> ModelParts {
        ModelParts {
            project_name: self.project_name.clone(),
            project_guid: self.project_guid.clone(),
            site_guid: self.site_guid.clone(),
            building_guid: self.building_guid.clone(),
            levels: self.levels.values().cloned().collect(),
            materials: self.materials.values().cloned().collect(),
            wall_types: self.wall_types.values().cloned().collect(),
            walls: self.walls.values().cloned().collect(),
            slab_types: self.slab_types.values().cloned().collect(),
            slabs: self.slabs.values().cloned().collect(),
            families: self.families.values().cloned().collect(),
            opening_types: self.opening_types.values().cloned().collect(),
            openings: self.openings.values().cloned().collect(),
            dimensions: self.dimensions.values().cloned().collect(),
            elevation_marks: self.elevation_marks.values().cloned().collect(),
            member_types: self.member_types.values().cloned().collect(),
            columns: self.columns.values().cloned().collect(),
            beams: self.beams.values().cloned().collect(),
            phases: self.phase_table(),
            disciplines: self.discipline_table(),
            user_parameters: self.user_params.values().cloned().collect(),
            parameter_values: self.parameter_values(),
            sections: self.sections.values().cloned().collect(),
            grids: self.grids.values().cloned().collect(),
            footings: self.footings.values().cloned().collect(),
            roofs: self.roofs.values().cloned().collect(),
            wall_roofs: self.wall_roofs.iter().map(|(w, r)| (*w, *r)).collect(),
            stairs: self.stairs.values().cloned().collect(),
            terrains: self.terrains.values().cloned().collect(),
            curtain_types: self.curtain_types.values().cloned().collect(),
            curtain_walls: self.curtain_walls.values().cloned().collect(),
            railings: self.railings.values().cloned().collect(),
            meshes: self.meshes.values().cloned().collect(),
            object_types: self.object_types.values().cloned().collect(),
            objects: self.objects.values().cloned().collect(),
            module_sources: self.module_sources.values().cloned().collect(),
            modules: self.modules.values().cloned().collect(),
            masses: self.masses.values().cloned().collect(),
            edges: self.edges.values().cloned().collect(),
            face_paints: self.face_paints().collect(),
            paint_zones: self.paint_zones().cloned().collect(),
            appearances: self.appearances().collect(),
            presentation_styles: self.presentation_styles.values().cloned().collect(),
            plan_views: self.plan_views.values().cloned().collect(),
            level_styles: self
                .level_styles
                .iter()
                .map(|(l, s)| (*l, s.clone()))
                .collect(),
            view_templates: self.view_templates.values().cloned().collect(),
            ref_planes: self.ref_planes.values().cloned().collect(),
            sun: self.sun,
            prices: self
                .prices
                .iter()
                .map(|(k, c)| (k.clone(), c.clone()))
                .collect(),
            budget: self.budget,
            plot_area: self.plot_area,
            plots: self.plots.values().cloned().collect(),
            roads: self.roads.values().cloned().collect(),
            plantings: self.plantings.values().cloned().collect(),
            pools: self.pools.values().cloned().collect(),
            runs: self.runs.values().cloned().collect(),
            devices: self.devices.values().cloned().collect(),
            true_north: self.true_north,
            room_usages: self.room_usages.clone(),
            georeference: self.georeference.clone(),
            code: self.code,
            draw_layers: self.draw_layers.values().cloned().collect(),
            drafts: self.drafts.values().cloned().collect(),
            graphics: self.graphics,
            info: self.info.clone(),
            sheets: self.sheets.values().cloned().collect(),
            viewports: self.viewports.values().cloned().collect(),
            rooms: self.rooms.values().cloned().collect(),
            tags: self.tags.values().cloned().collect(),
            tables: self.tables.values().cloned().collect(),
        }
    }

    /// Rebuilds a model from its parts, keeping every id and GUID.
    ///
    /// Everything is checked as if the elements were being drawn again:
    /// references must exist, ids and GUIDs must be unique, numbers must be
    /// finite, outlines valid and doors and windows must fit in their walls
    /// without overlapping. The first problem found is reported.
    pub fn from_parts(parts: ModelParts) -> Result<Self, RestoreError> {
        let fail = |element: String| move |error: ModelError| RestoreError { element, error };

        let mut model = Model::new(parts.project_name);
        model.project_guid = parts.project_guid;
        model.site_guid = parts.site_guid;
        model.building_guid = parts.building_guid;
        if !parts.graphics.is_valid() {
            return Err(fail("graphic standard".into())(ModelError::NonPositive(
                "line weight",
            )));
        }
        model.graphics = parts.graphics;
        model.info = parts.info;

        let mut ids = std::collections::HashSet::new();
        let mut guids = std::collections::HashSet::new();
        for guid in [&model.project_guid, &model.site_guid, &model.building_guid] {
            if !guids.insert(guid.clone()) {
                return Err(fail("project".into())(ModelError::DuplicateGuid(
                    guid.to_string(),
                )));
            }
        }
        let mut claim = |id: u64, guid: Option<&GlobalId>| -> Result<(), ModelError> {
            if id == 0 || id > MAX_ID {
                return Err(ModelError::InvalidId(id));
            }
            if !ids.insert(id) {
                return Err(ModelError::DuplicateId(id));
            }
            if let Some(g) = guid {
                if !guids.insert(g.clone()) {
                    return Err(ModelError::DuplicateGuid(g.to_string()));
                }
            }
            Ok(())
        };

        for level in parts.levels {
            let name = format!("level {} «{}»", level.id.0, level.name);
            claim(level.id.0, Some(&level.guid))
                .and_then(|_| finite(level.elevation, "level elevation"))
                .map_err(fail(name))?;
            model.levels.insert(level.id, level);
        }
        for material in parts.materials {
            let name = format!("material {} «{}»", material.id.0, material.name);
            claim(material.id.0, None)
                .and_then(|_| {
                    if material.priority.is_some_and(|p| p > MAX_PRIORITY) {
                        Err(ModelError::InvalidPriority)
                    } else {
                        Ok(())
                    }
                })
                .map_err(fail(name))?;
            model.materials.insert(material.id, material);
        }
        for wt in parts.wall_types {
            let name = format!("wall type {} «{}»", wt.id.0, wt.name);
            claim(wt.id.0, Some(&wt.guid))
                .and_then(|_| model.validate_layers(&wt.layers))
                .map_err(fail(name))?;
            model.wall_types.insert(wt.id, wt);
        }
        for wall in parts.walls {
            let name = format!("wall {} ({})", wall.id.0, wall.guid);
            let spec = NewWall {
                wall_type: wall.wall_type,
                level: wall.level,
                start: wall.start,
                end: wall.end,
                reference: wall.reference,
                height: wall.height,
                base_offset: wall.base_offset,
                bulge: wall.bulge,
                top: wall.top,
            };
            let mut wall = wall;
            // The levels decide the height of a wall tied to one.
            wall.height = claim(wall.id.0, Some(&wall.guid))
                .and_then(|_| model.validate_wall(&spec))
                .and_then(|h| match &wall.profile {
                    Some(_) if wall.is_curved() => Err(ModelError::InvalidWallProfile(wall.id)),
                    Some(p) => wall_profile::check_profile(wall.id, p).map(|_| h),
                    None => Ok(h),
                })
                .map_err(|e| match e {
                    ModelError::WallTopBelowBase(_) => ModelError::WallTopBelowBase(wall.id),
                    e => e,
                })
                .map_err(fail(name))?;
            model.walls.insert(wall.id, wall);
        }
        for st in parts.slab_types {
            let name = format!("slab type {} «{}»", st.id.0, st.name);
            claim(st.id.0, Some(&st.guid))
                .and_then(|_| model.validate_layers(&st.layers))
                .map_err(fail(name))?;
            model.slab_types.insert(st.id, st);
        }
        for mut slab in parts.slabs {
            let name = format!("slab {} ({})", slab.id.0, slab.guid);
            claim(slab.id.0, Some(&slab.guid))
                .and_then(|_| {
                    if !model.levels.contains_key(&slab.level) {
                        return Err(ModelError::UnknownLevel(slab.level));
                    }
                    if !model.slab_types.contains_key(&slab.slab_type) {
                        return Err(ModelError::UnknownSlabType(slab.slab_type));
                    }
                    finite(slab.top_offset, "slab top offset")?;
                    if let Some(s) = &slab.slope {
                        s.validate()?;
                    }
                    slab.outline = Self::normalized_outline(std::mem::take(&mut slab.outline))?;
                    slab.holes = std::mem::take(&mut slab.holes)
                        .into_iter()
                        .map(|h| Self::normalized_outline(h).map_err(|_| ModelError::InvalidHole))
                        .collect::<Result<_, _>>()?;
                    Self::check_holes(&slab.outline, &slab.holes)?;
                    Ok(())
                })
                .map_err(fail(name))?;
            model.slabs.insert(slab.id, slab);
        }
        for f in parts.families {
            let name = format!("family {} «{}»", f.id.0, f.name());
            claim(f.id.0, None)
                .and_then(|_| match model.family_by_name(f.name()) {
                    Some(_) => Err(ModelError::DuplicateFamily(f.name().to_string())),
                    None => Ok(()),
                })
                .map_err(fail(name))?;
            model.families.insert(f.id, f);
        }
        for ot in parts.opening_types {
            let name = format!("door or window type {} «{}»", ot.id.0, ot.name);
            let resolved = claim(ot.id.0, Some(&ot.guid))
                .and_then(|_| model.check_mark(&ot.mark, None))
                .and_then(|_| model.resolve_opening_type(ot.family, &ot.values))
                .map_err(fail(name))?;
            model.opening_types.insert(
                ot.id,
                OpeningType {
                    kind: resolved.spec.kind,
                    width: resolved.spec.width,
                    height: resolved.spec.height,
                    frame_width: resolved.spec.frame_width,
                    frame_depth: resolved.spec.frame_depth,
                    frame_material: resolved.spec.frame_material,
                    panel_material: resolved.spec.panel_material,
                    panels: resolved.panels,
                    shape: resolved.shape,
                    frame_place: resolved.frame_place,
                    ..ot
                },
            );
        }
        for mut o in parts.openings {
            let name = format!("door or window {} ({})", o.id.0, o.guid);
            // Instance parameters a newer recipe added take their defaults.
            if let Some(family) = model.opening_types.get(&o.opening_type).map(|t| t.family) {
                if let Ok(defaults) = model.family_defaults(family, ParamScope::Instance) {
                    for (k, v) in defaults {
                        o.values.entry(k).or_insert(v);
                    }
                }
            }
            let spec = NewOpening {
                opening_type: o.opening_type,
                wall: o.wall,
                along: o.along,
                sill: o.sill,
                opens_to: o.opens_to,
                hand: o.hand,
                setback: o.setback,
            };
            claim(o.id.0, Some(&o.guid))
                .and_then(|_| model.validate_opening_at(&spec, None, o.corner_end()))
                .and_then(|_| {
                    let family = model.opening_types[&o.opening_type].family;
                    model.families[&family]
                        .def()
                        .check_values(ParamScope::Instance, &o.values)
                        .map_err(ModelError::Family)
                })
                .map_err(fail(name))?;
            model.openings.insert(o.id, o);
        }

        for d in parts.dimensions {
            let name = format!("dimension {}", d.id.0);
            let spec = NewDimension {
                level: d.level,
                points: d.points.clone(),
                direction: d.direction,
                position: d.position,
            };
            let mut d = d;
            claim(d.id.0, None)
                .and_then(|_| model.validate_dimension(&spec))
                .map(|dir| d.direction = dir)
                .map_err(fail(name))?;
            if !d.anchors.is_empty() && d.anchors.len() != d.points.len() {
                d.anchors.clear();
            }
            model.dimensions.insert(d.id, d);
        }
        for m in parts.elevation_marks {
            let name = format!("level mark {}", m.id.0);
            claim(m.id.0, None)
                .and_then(|_| {
                    if !model.levels.contains_key(&m.level) {
                        return Err(ModelError::UnknownLevel(m.level));
                    }
                    finite_point(m.point, "level mark point")
                })
                .map_err(fail(name))?;
            model.elevation_marks.insert(m.id, m);
        }
        for layer in parts.draw_layers {
            let name = format!("drawing layer {} «{}»", layer.id.0, layer.name);
            // Layer "0" has id 0, which no other element can have.
            if layer.id != LAYER_ZERO {
                claim(layer.id.0, None).map_err(fail(name.clone()))?;
            }
            model.restore_draw_layer(layer).map_err(fail(name))?;
        }
        // Sections before sheets: views on sheets may show them.
        model.restore_sections(parts.sections, &mut claim)?;
        for sh in parts.sheets {
            let name = format!("sheet {} «{}»", sh.id.0, sh.number);
            claim(sh.id.0, None)
                .and_then(|_| model.restore_sheet(sh))
                .map_err(fail(name))?;
        }
        for v in parts.viewports {
            let name = format!("view {} on a sheet", v.id.0);
            claim(v.id.0, None)
                .and_then(|_| model.restore_viewport(v))
                .map_err(fail(name))?;
        }
        for d in parts.drafts {
            let name = format!("drafting element {} ({})", d.id.0, d.shape.label());
            claim(d.id.0, None)
                .and_then(|_| model.restore_draft(d))
                .map_err(fail(name))?;
        }
        for r in parts.rooms {
            let name = format!("room {} «{} {}»", r.id.0, r.number, r.name);
            claim(r.id.0, Some(&r.guid))
                .and_then(|_| model.restore_room(r))
                .map_err(fail(name))?;
        }
        for t in parts.tags {
            let name = format!("tag {}", t.id.0);
            claim(t.id.0, None)
                .and_then(|_| model.restore_tag(t))
                .map_err(fail(name))?;
        }
        for t in parts.tables {
            let name = format!("schedule {} on a sheet", t.id.0);
            claim(t.id.0, None)
                .and_then(|_| model.restore_table(t))
                .map_err(fail(name))?;
        }

        model.restore_members(parts.member_types, parts.columns, parts.beams, &mut claim)?;
        model.restore_grids_and_footings(parts.grids, parts.footings, &mut claim)?;
        model.restore_roofs(parts.roofs, &mut claim)?;
        for (w, r) in parts.wall_roofs {
            model
                .set_wall_roof(w, Some(r))
                .map_err(|error| RestoreError {
                    element: format!("wall {} under roof {}", w.0, r.0),
                    error,
                })?;
        }
        model.restore_stairs(parts.stairs, &mut claim)?;
        model.restore_terrains(parts.terrains, &mut claim)?;
        model.restore_curtain_walls(parts.curtain_types, parts.curtain_walls, &mut claim)?;
        model.check_type_accessories()?;
        model.restore_meshes(parts.meshes, &mut claim)?;
        model.restore_objects(parts.object_types, parts.objects, &mut claim)?;
        // (After the object types: a railing may take one as its type.)
        model.restore_railings(parts.railings, &mut claim)?;
        model.restore_modules(parts.module_sources, parts.modules, &mut claim)?;
        model.restore_masses(parts.masses, &mut claim)?;
        model.restore_edges(parts.edges, &mut claim)?;
        model.restore_paint(parts.face_paints, parts.paint_zones)?;
        model.restore_appearances(parts.appearances)?;
        model.restore_presentation_styles(parts.presentation_styles, &mut claim)?;
        model.restore_views(
            parts.plan_views,
            parts.level_styles,
            parts.view_templates,
            parts.ref_planes,
            &mut claim,
        )?;
        model.set_sun(parts.sun).map_err(|error| RestoreError {
            element: "sun".into(),
            error,
        })?;
        for (key, concept) in parts.prices {
            model
                .set_price(&key, Some(concept))
                .map_err(|error| RestoreError {
                    element: format!("price of {key}"),
                    error,
                })?;
        }
        model
            .set_budget_settings(parts.budget)
            .map_err(|error| RestoreError {
                element: "budget".into(),
                error,
            })?;
        model.restore_urban(parts.plots, parts.roads, &mut claim)?;
        model.restore_plantings(parts.plantings, &mut claim)?;
        model.restore_pools(parts.pools, &mut claim)?;
        model.restore_services(parts.runs, parts.devices, &mut claim)?;
        model
            .set_plot_area(parts.plot_area)
            .map_err(|error| RestoreError {
                element: "plot area".into(),
                error,
            })?;
        model
            .set_true_north(parts.true_north)
            .map_err(|error| RestoreError {
                element: "true north".into(),
                error,
            })?;
        model
            .set_room_usages(parts.room_usages)
            .map_err(|error| RestoreError {
                element: "room uses".into(),
                error,
            })?;
        model
            .set_georeference(parts.georeference)
            .map_err(|error| RestoreError {
                element: "georeference".into(),
                error,
            })?;
        model.set_code_settings(parts.code);
        model.restore_phases(parts.phases)?;
        model.restore_disciplines(parts.disciplines)?;
        model.restore_parameters(parts.user_parameters, parts.parameter_values)?;

        // Parameters take ids from the same counter as the elements.
        let params = model.user_params.keys().map(|p| p.0);
        model.next_id = ids.iter().copied().chain(params).max().map_or(1, |m| m + 1);
        model.top_id = model.next_id - 1;
        Ok(model)
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
            bulge: 0.0,
            top: crate::WallTop::Unconnected,
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
    fn curved_walls() {
        let (mut m, lvl, t) = sample();
        let brick = m.materials().next().unwrap().id;
        // A half circle of radius 2 bending towards −y.
        let mut s = spec(lvl, t);
        s.bulge = 1.0;
        let id = m.add_wall(s.clone()).unwrap();
        let w = m.wall(id).unwrap();
        assert!(w.is_curved());
        assert!((w.length() - 2.0 * std::f64::consts::PI).abs() < 1e-9);
        assert!(w.midpoint().approx_eq(Point2::new(2.0, -2.0)));
        // Moving keeps the bend; stretching the chord keeps its shape.
        m.move_wall(id, Point2::new(1.0, 0.0), Point2::new(5.0, 0.0))
            .unwrap();
        assert_eq!(m.wall(id).unwrap().bulge, 1.0);
        // Flipping turns the other way round along the same arc.
        let before = m.wall(id).unwrap().midpoint();
        m.flip_wall(id).unwrap();
        assert_eq!(m.wall(id).unwrap().bulge, -1.0);
        assert!(m.wall(id).unwrap().midpoint().approx_eq(before));
        // Doors and windows, straight, square to the wall at their centre.
        let otype = m
            .add_opening_type(NewOpeningType {
                name: "P".into(),
                kind: OpeningKind::Door(DoorOperation::SingleSwing),
                width: 0.8,
                height: 2.0,
                frame_width: 0.05,
                frame_depth: 0.08,
                frame_material: brick,
                panel_material: brick,
            })
            .unwrap();
        let op = NewOpening {
            opening_type: otype,
            wall: id,
            along: 2.0,
            sill: 0.0,
            opens_to: WallSide::Interior,
            hand: Hand::Left,
            setback: None,
        };
        let door_in_curve = m.add_opening(op.clone()).unwrap();
        m.remove_opening(door_in_curve).unwrap();
        // Too wide for the curve: 3,5 m in a radius of 2.
        let wide = m
            .add_opening_type(NewOpeningType {
                name: "Ancha".into(),
                kind: OpeningKind::Door(DoorOperation::SingleSwing),
                width: 3.5,
                height: 2.0,
                frame_width: 0.05,
                frame_depth: 0.08,
                frame_material: brick,
                panel_material: brick,
            })
            .unwrap();
        assert_eq!(
            m.add_opening(NewOpening {
                opening_type: wide,
                ..op.clone()
            }),
            Err(ModelError::OpeningInCurvedWall)
        );
        // Too tight: a 4 cm radius for a 10 cm wall.
        let mut tight = spec(lvl, t);
        tight.end = Point2::new(0.08, 0.0);
        tight.bulge = 1.0;
        assert_eq!(m.add_wall(tight), Err(ModelError::ArcTooTight));
        // Shrinking a curved wall until it is too tight is refused whole.
        let before = m.wall(id).unwrap().clone();
        assert_eq!(
            m.move_wall(id, Point2::new(0.0, 0.0), Point2::new(0.1, 0.0)),
            Err(ModelError::ArcTooTight)
        );
        assert_eq!(m.wall(id).unwrap(), &before);
        let mut closed = spec(lvl, t);
        closed.bulge = 20.0;
        assert_eq!(m.add_wall(closed), Err(ModelError::ArcTooTight));
        // A bulge too small to see is a straight wall.
        let mut flat = spec(lvl, t);
        flat.bulge = 1e-9;
        let f = m.add_wall(flat).unwrap();
        assert_eq!(m.wall(f).unwrap().bulge, 0.0);
        // Bending a wall keeps its ends; a wall with a door bends while
        // the door still fits in the curve.
        m.set_wall_bulge(f, 0.5).unwrap();
        assert!(m.wall(f).unwrap().is_curved());
        m.set_wall_bulge(f, 0.0).unwrap();
        let mut door = op.clone();
        door.wall = f;
        m.add_opening(door).unwrap();
        m.set_wall_bulge(f, 0.5).unwrap();
        // A wide door does not let its wall bend that much.
        let mut flat2 = spec(lvl, t);
        flat2.start = Point2::new(0.0, 10.0);
        flat2.end = Point2::new(4.0, 10.0);
        let g = m.add_wall(flat2).unwrap();
        m.add_opening(NewOpening {
            opening_type: wide,
            wall: g,
            ..op.clone()
        })
        .unwrap();
        assert_eq!(
            m.set_wall_bulge(g, 0.5),
            Err(ModelError::OpeningInCurvedWall)
        );
        // A type thick enough to close the curve (radius 2) is refused.
        let thick = m
            .add_wall_type(
                "Muro grueso",
                WallFunction::Exterior,
                vec![Layer {
                    material: brick,
                    thickness: 4.5,
                    function: LayerFunction::Structure,
                }],
            )
            .unwrap();
        assert_eq!(m.set_wall_type(id, thick), Err(ModelError::ArcTooTight));
    }

    #[test]
    fn slab_holes() {
        let (mut m, lvl, _) = sample();
        let mat = m.materials().next().unwrap().id;
        let st = m
            .add_slab_type(
                "Forjado",
                SlabFunction::Floor,
                vec![Layer {
                    material: mat,
                    thickness: 0.3,
                    function: LayerFunction::Structure,
                }],
            )
            .unwrap();
        let p = |x, y| Point2::new(x, y);
        let id = m
            .add_slab(NewSlab {
                slab_type: st,
                level: lvl,
                outline: vec![p(0.0, 0.0), p(10.0, 0.0), p(10.0, 8.0), p(0.0, 8.0)],
                top_offset: 0.0,
            })
            .unwrap();
        // A stair hole, drawn clockwise: stored counter-clockwise.
        let stair = vec![p(1.0, 1.0), p(1.0, 4.0), p(3.0, 4.0), p(3.0, 1.0)];
        assert_eq!(m.add_slab_hole(id, stair), Ok(0));
        let s = m.slab(id).unwrap();
        assert!(signed_area(&s.holes[0]) > 0.0);
        assert!((s.area() - 74.0).abs() < 1e-9);
        assert!(!s.covers(p(2.0, 2.0)) && s.covers(p(5.0, 5.0)));
        assert_eq!(m.floor_elevation_at(lvl, p(2.0, 2.0)), Some(0.0));
        // Outside the slab, crossing its edge or overlapping another hole:
        // refused.
        for bad in [
            vec![p(9.0, 1.0), p(11.0, 1.0), p(11.0, 2.0), p(9.0, 2.0)],
            vec![p(2.0, 2.0), p(5.0, 2.0), p(5.0, 3.0), p(2.0, 3.0)],
            vec![p(20.0, 1.0), p(21.0, 1.0), p(21.0, 2.0)],
        ] {
            assert_eq!(m.add_slab_hole(id, bad), Err(ModelError::InvalidHole));
        }
        m.add_slab_hole(id, vec![p(6.0, 6.0), p(7.0, 6.0), p(7.0, 7.0), p(6.0, 7.0)])
            .unwrap();
        // The outline cannot shrink past a hole.
        assert_eq!(
            m.set_slab_outline(id, vec![p(0.0, 0.0), p(5.0, 0.0), p(5.0, 8.0), p(0.0, 8.0)]),
            Err(ModelError::InvalidHole)
        );
        m.remove_slab_hole(id, 1).unwrap();
        m.set_slab_outline(id, vec![p(0.0, 0.0), p(5.0, 0.0), p(5.0, 8.0), p(0.0, 8.0)])
            .unwrap();
        // Saved and opened, the holes come back (and a bad one is refused).
        let back = Model::from_parts(m.to_parts()).unwrap();
        assert_eq!(back.slab(id).unwrap().holes, m.slab(id).unwrap().holes);
        let mut parts = m.to_parts();
        parts.slabs[0]
            .holes
            .push(vec![p(-1.0, -1.0), p(0.5, -1.0), p(0.5, 0.5)]);
        assert!(Model::from_parts(parts).is_err());
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

    /// Walls and slabs change level keeping their offsets; a wall a full
    /// storey high stays a full storey high on its new level, and one
    /// whose top level would not be above it is refused.
    #[test]
    fn walls_and_slabs_change_level() {
        let (mut m, wall, _, window) = with_openings();
        let pb = m.levels()[0].id;
        let p1 = m.add_level("Planta 1", 3.0);
        let p2 = m.add_level("Planta 2", 6.2);
        assert_eq!(m.level_above(pb), Some(p1));
        assert_eq!(m.level_above(p2), None);
        m.set_wall_top(
            wall,
            WallTop::Level {
                level: p1,
                offset: -0.3,
            },
        )
        .unwrap();
        let w = m.add_opening(place(window, wall, 2.0, 0.9)).unwrap();

        // Up one storey: tied to the next level up (3.2 m high now), the
        // window goes with it.
        m.set_wall_level(wall, p1).unwrap();
        let moved = m.wall(wall).unwrap();
        assert_eq!(moved.level, p1);
        assert_eq!(
            moved.top,
            WallTop::Level {
                level: p2,
                offset: -0.3
            }
        );
        assert!((moved.height - 2.9).abs() < 1e-12);
        assert_eq!(m.opening(w).unwrap().wall, wall);
        // Top storey: nothing above, it keeps its height, free.
        m.set_wall_level(wall, p2).unwrap();
        assert_eq!(m.wall(wall).unwrap().top, WallTop::Unconnected);
        assert!((m.wall(wall).unwrap().height - 2.9).abs() < 1e-12);

        // Tied to a level that is not the one right above: it keeps it, and
        // cannot go to that level or above it.
        m.set_wall_level(wall, pb).unwrap();
        m.set_wall_top(
            wall,
            WallTop::Level {
                level: p2,
                offset: 0.0,
            },
        )
        .unwrap();
        let before = m.clone();
        assert_eq!(
            m.set_wall_level(wall, p2),
            Err(ModelError::WallTopBelowBase(wall))
        );
        assert_eq!(m, before);
        m.set_wall_level(wall, p1).unwrap();
        assert!((m.wall(wall).unwrap().height - 3.2).abs() < 1e-12);

        // Slabs keep their top offset.
        let concrete = m.add_material("Hormigón", "concrete", [190, 190, 190], CutPattern::Solid);
        let slab_type = m
            .add_slab_type(
                "Losa",
                SlabFunction::Floor,
                vec![Layer {
                    material: concrete,
                    thickness: 0.25,
                    function: LayerFunction::Structure,
                }],
            )
            .unwrap();
        let slab = m
            .add_slab(NewSlab {
                slab_type,
                level: pb,
                outline: vec![
                    Point2::new(0.0, 0.0),
                    Point2::new(4.0, 0.0),
                    Point2::new(4.0, 3.0),
                ],
                top_offset: 0.05,
            })
            .unwrap();
        m.set_slab_level(slab, p1).unwrap();
        assert_eq!(m.slab(slab).unwrap().level, p1);
        assert_eq!(m.slab(slab).unwrap().top_offset, 0.05);
        assert!(m.set_slab_level(slab, LevelId(999)).is_err());
    }

    /// A wall tied to the level above follows it: its height is worked out
    /// from the levels, kept when either level moves, refused when its top
    /// would fall to its base or its window would no longer fit, freed when
    /// a height is typed or the level is removed, and restored from parts.
    #[test]
    fn walls_follow_their_top_level() {
        let (mut m, wall, _, window) = with_openings();
        let pb = m.levels()[0].id;
        let p1 = m.add_level("Planta 1", 3.2);
        let tied = WallTop::Level {
            level: p1,
            offset: -0.3,
        };
        m.set_wall_top(wall, tied).unwrap();
        assert!((m.wall(wall).unwrap().height - 2.9).abs() < 1e-12);
        m.add_opening(place(window, wall, 2.0, 0.9)).unwrap();

        // The upper level goes up: the wall grows; the ground floor goes up:
        // it shrinks (the top stays tied).
        m.set_level_elevation(p1, 3.5).unwrap();
        assert!((m.wall(wall).unwrap().height - 3.2).abs() < 1e-12);
        m.set_level_elevation(pb, 0.4).unwrap();
        assert!((m.wall(wall).unwrap().height - 2.8).abs() < 1e-12);
        // A base offset keeps the top where it is.
        m.set_wall_base_offset(wall, 0.1).unwrap();
        assert!((m.wall(wall).unwrap().height - 2.7).abs() < 1e-12);
        assert_eq!(m.wall(wall).unwrap().top, tied);

        // Too low for the window (0.9 + 1.2 m above the base): refused, and
        // nothing moved.
        let before = m.clone();
        assert_eq!(
            m.set_level_elevation(p1, 2.4),
            Err(ModelError::OpeningOutsideWall)
        );
        assert_eq!(m, before);
        // At or below the base: refused, naming the wall.
        assert_eq!(
            m.set_level_elevation(p1, 0.7),
            Err(ModelError::WallTopBelowBase(wall))
        );
        assert_eq!(m, before);
        // A wall drawn up to a level below its base is refused too.
        let mut low = spec(p1, m.wall(wall).unwrap().wall_type);
        low.top = WallTop::Level {
            level: pb,
            offset: 0.0,
        };
        assert!(matches!(
            m.add_wall(low),
            Err(ModelError::WallTopBelowBase(_))
        ));

        // Saved and opened again: same wall, height from the levels.
        let mut parts = m.to_parts();
        assert_eq!(Model::from_parts(parts.clone()).unwrap(), m);
        parts.walls[0].height = 99.0;
        let restored = Model::from_parts(parts.clone()).unwrap();
        assert!((restored.wall(wall).unwrap().height - 2.7).abs() < 1e-12);
        parts.walls[0].top = WallTop::Level {
            level: LevelId(9999),
            offset: 0.0,
        };
        assert!(matches!(
            restore_error(parts),
            ModelError::UnknownLevel(LevelId(9999))
        ));

        // Typing a height frees it; freeing keeps the height.
        let mut typed = m.clone();
        typed.set_wall_height(wall, 2.5, 0.1).unwrap();
        assert_eq!(typed.wall(wall).unwrap().top, WallTop::Unconnected);
        m.set_wall_top(wall, WallTop::Unconnected).unwrap();
        assert!((m.wall(wall).unwrap().height - 2.7).abs() < 1e-12);
        m.set_level_elevation(p1, 6.0).unwrap();
        assert!((m.wall(wall).unwrap().height - 2.7).abs() < 1e-12);

        // Removing the top level frees the walls tied to it.
        m.set_wall_top(wall, tied).unwrap();
        let height = m.wall(wall).unwrap().height;
        m.remove_level(p1).unwrap();
        let w = m.wall(wall).unwrap();
        assert_eq!((w.top, w.height), (WallTop::Unconnected, height));
    }

    #[test]
    fn materials_can_be_edited_and_removed_when_unused() {
        let (mut m, wall, door, _window) = with_openings();
        let brick = m
            .wall(wall)
            .map(|w| m.wall_type(w.wall_type).unwrap().layers[0].material)
            .unwrap();
        m.set_material(
            brick,
            "Ladrillo macizo",
            "brick",
            [180, 80, 50],
            CutPattern::Cross,
        )
        .unwrap();
        let mat = m.material(brick).unwrap();
        assert_eq!(
            (mat.name.as_str(), mat.cut_pattern),
            ("Ladrillo macizo", CutPattern::Cross)
        );
        // (The door takes the first material for its hardware too: the
        // model has no metal.)
        assert!(m.material_users(brick).contains(&"Tabique".to_string()));
        assert!(matches!(
            m.remove_material(brick),
            Err(ModelError::InUse(_))
        ));
        let wood = m.opening_type(door).unwrap().frame_material;
        assert_eq!(
            m.material_users(wood),
            vec!["P 80".to_string(), "V 120".to_string()]
        );
        let spare = m.add_material(
            "Corcho",
            "insulation",
            [200, 170, 120],
            CutPattern::Insulation,
        );
        assert!(m.material_users(spare).is_empty());
        assert_eq!(m.remove_material(spare).unwrap().name, "Corcho");
        assert_eq!(
            m.remove_material(spare),
            Err(ModelError::UnknownMaterial(spare))
        );
    }

    #[test]
    fn editing_a_wall_type_changes_its_walls_and_protects_frames() {
        let (mut m, wall, door, _window) = with_openings();
        let t = m.wall(wall).unwrap().wall_type;
        let guid = m.wall_type(t).unwrap().guid.clone();
        let brick = m.wall_type(t).unwrap().layers[0].material;
        let plaster = m.add_material("Yeso", "plaster", [245, 245, 245], CutPattern::Solid);
        m.add_opening(place(door, wall, 1.0, 0.0)).unwrap();
        assert_eq!(m.wall_type_users(t), 1);
        // Plaster both sides: 10 → 13 cm, same GUID, the wall follows.
        let layers = vec![
            Layer {
                material: plaster,
                thickness: 0.015,
                function: LayerFunction::Finish,
            },
            Layer {
                material: brick,
                thickness: 0.1,
                function: LayerFunction::Structure,
            },
            Layer {
                material: plaster,
                thickness: 0.015,
                function: LayerFunction::Finish,
            },
        ];
        m.edit_wall_type(t, "Tabique guarnecido", WallFunction::Interior, layers)
            .unwrap();
        let wt = m.wall_type(t).unwrap();
        assert!((wt.thickness() - 0.13).abs() < 1e-12);
        assert_eq!((wt.name.as_str(), &wt.guid), ("Tabique guarnecido", &guid));
        // Thinner than the 7 cm door frame: rejected, nothing changes.
        let thin = vec![Layer {
            material: brick,
            thickness: 0.05,
            function: LayerFunction::Structure,
        }];
        assert_eq!(
            m.edit_wall_type(t, "Fino", WallFunction::Interior, thin),
            Err(ModelError::FrameTooDeep)
        );
        assert_eq!(m.wall_type(t).unwrap().name, "Tabique guarnecido");
        assert_eq!(
            m.edit_wall_type(t, "Vacío", WallFunction::Interior, vec![]),
            Err(ModelError::EmptyType)
        );
        let zero = vec![Layer {
            material: brick,
            thickness: 0.0,
            function: LayerFunction::Structure,
        }];
        assert!(matches!(
            m.edit_wall_type(t, "Cero", WallFunction::Interior, zero),
            Err(ModelError::NonPositive(_))
        ));
        let unknown = vec![Layer {
            material: MaterialId(999),
            thickness: 0.1,
            function: LayerFunction::Structure,
        }];
        assert_eq!(
            m.edit_wall_type(t, "?", WallFunction::Interior, unknown),
            Err(ModelError::UnknownMaterial(MaterialId(999)))
        );
    }

    #[test]
    fn editing_a_slab_type_keeps_its_guid() {
        let (mut m, lvl, _) = sample();
        let st = slab_type(&mut m);
        let guid = m.slab_type(st).unwrap().guid.clone();
        let s = m
            .add_slab(NewSlab {
                slab_type: st,
                level: lvl,
                outline: vec![
                    Point2::new(0.0, 0.0),
                    Point2::new(4.0, 0.0),
                    Point2::new(4.0, 3.0),
                ],
                top_offset: 0.0,
            })
            .unwrap();
        assert_eq!(m.slab_type_users(st), 1);
        let mut layers = m.slab_type(st).unwrap().layers.clone();
        layers.reverse();
        m.edit_slab_type(st, "Cubierta", SlabFunction::Roof, layers.clone())
            .unwrap();
        let t = m.slab_type(st).unwrap();
        assert_eq!(
            (t.function, &t.guid, &t.layers),
            (SlabFunction::Roof, &guid, &layers)
        );
        assert_eq!(m.slab(s).unwrap().top_offset, 0.0);
        assert_eq!(
            m.edit_slab_type(SlabTypeId(999), "x", SlabFunction::Floor, layers),
            Err(ModelError::UnknownSlabType(SlabTypeId(999)))
        );
    }

    #[test]
    fn editing_a_door_type_checks_every_door() {
        let (mut m, wall, door, window) = with_openings();
        let d1 = m.add_opening(place(door, wall, 0.6, 0.0)).unwrap();
        m.add_opening(place(window, wall, 2.2, 0.9)).unwrap();
        let spec = |t: &OpeningType| NewOpeningType {
            name: t.name.clone(),
            kind: t.kind,
            width: t.width,
            height: t.height,
            frame_width: t.frame_width,
            frame_depth: t.frame_depth,
            frame_material: t.frame_material,
            panel_material: t.panel_material,
        };
        let base = spec(m.opening_type(door).unwrap());
        let guid = m.opening_type(door).unwrap().guid.clone();
        // 72.5 cm wide double door, same GUID, the door keeps its centre.
        let mut ok = base.clone();
        ok.width = 0.725;
        ok.kind = OpeningKind::Door(DoorOperation::Sliding);
        ok.name = "P 72,5".into();
        m.edit_opening_type(door, ok).unwrap();
        let t = m.opening_type(door).unwrap();
        assert_eq!(
            (t.width, t.kind, &t.guid),
            (0.725, OpeningKind::Door(DoorOperation::Sliding), &guid)
        );
        assert_eq!(m.opening(d1).unwrap().along, 0.6);
        // 1.4 m wide would leave the wall at its start (centre at 0.6 m).
        let mut wide = base.clone();
        wide.width = 1.4;
        assert_eq!(
            m.edit_opening_type(door, wide),
            Err(ModelError::OpeningOutsideWall)
        );
        let mut overlap = base.clone();
        overlap.width = 1.3;
        let mut m2 = m.clone();
        m2.move_opening(d1, 1.0, 0.0).unwrap();
        assert_eq!(
            m2.edit_opening_type(door, overlap),
            Err(ModelError::OverlappingOpenings)
        );
        // A frame deeper than the 10 cm wall.
        let mut deep = base.clone();
        deep.frame_depth = 0.12;
        assert_eq!(
            m.edit_opening_type(door, deep),
            Err(ModelError::FrameTooDeep)
        );
        // A door cannot become a window.
        let mut window_kind = base.clone();
        window_kind.kind = OpeningKind::Window(WindowOperation::Fixed);
        assert_eq!(
            m.edit_opening_type(door, window_kind),
            Err(ModelError::CategoryMismatch)
        );
        // Rejections leave the type as it was.
        assert_eq!(m.opening_type(door).unwrap().width, 0.725);
        assert_eq!(m.opening_type_users(door), 1);
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
    fn levels_can_be_renamed_moved_and_removed() {
        let (mut m, wall, door, _) = with_openings();
        let ground = m.wall(wall).unwrap().level;
        m.add_opening(place(door, wall, 2.0, 0.0)).unwrap();
        let first = m.add_level("Planta primera", 3.0);
        m.rename_level(first, "Planta 1").unwrap();
        m.set_level_elevation(first, 3.2).unwrap();
        assert_eq!(m.level(first).unwrap().name, "Planta 1");
        assert_eq!(m.level(first).unwrap().elevation, 3.2);
        assert!(m.set_level_elevation(first, f64::NAN).is_err());
        m.add_dimension(NewDimension {
            level: ground,
            points: vec![Point2::new(0.0, 0.0), Point2::new(4.0, 0.0)],
            direction: Point2::new(1.0, 0.0),
            position: Point2::new(0.0, -1.0),
        })
        .unwrap();
        assert_eq!(m.level_contents(ground), (1, 1, 0, 1));
        // Removing the ground floor takes its wall, door and dimension.
        m.remove_level(ground).unwrap();
        assert_eq!(m.walls().count(), 0);
        assert_eq!(m.openings().count(), 0);
        assert_eq!(m.dimensions().count(), 0);
        // The last level stays.
        assert_eq!(m.remove_level(first), Err(ModelError::LastLevel));
    }

    #[test]
    fn dimensions_and_level_marks() {
        let (mut m, lvl, _) = sample();
        // A chain along x with an unordered, repeated point.
        let d = m
            .add_dimension(NewDimension {
                level: lvl,
                points: vec![
                    Point2::new(4.0, 0.0),
                    Point2::new(0.0, 0.0),
                    Point2::new(1.5, 2.0),
                    Point2::new(4.0, 1.0),
                ],
                direction: Point2::new(2.0, 0.0),
                position: Point2::new(0.0, -0.8),
            })
            .unwrap();
        let dim = m.dimension(d).unwrap();
        assert_eq!(dim.direction, Point2::new(1.0, 0.0));
        assert_eq!(dim.values(), vec![1.5, 2.5]);
        assert_eq!(dim.line_offset(), -0.8);
        // All points at the same place along the direction: refused.
        assert_eq!(
            m.add_dimension(NewDimension {
                level: lvl,
                points: vec![Point2::new(1.0, 0.0), Point2::new(1.0, 3.0)],
                direction: Point2::new(1.0, 0.0),
                position: Point2::default(),
            }),
            Err(ModelError::DegenerateDimension)
        );
        m.set_dimension(
            d,
            vec![Point2::new(0.0, 0.0), Point2::new(2.0, 0.0)],
            Point2::new(0.0, 1.0),
        )
        .unwrap();
        assert_eq!(m.dimension(d).unwrap().values(), vec![2.0]);

        // A level mark reads the slab under it.
        let mark = m.add_elevation_mark(lvl, Point2::new(1.0, 1.0)).unwrap();
        assert_eq!(m.floor_elevation_at(lvl, Point2::new(1.0, 1.0)), Some(0.0));
        let st = m.slab_types().next().map(|t| t.id);
        let st = match st {
            Some(t) => t,
            None => {
                let mat = m.materials().next().unwrap().id;
                m.add_slab_type(
                    "Solera",
                    SlabFunction::BaseSlab,
                    vec![Layer {
                        material: mat,
                        thickness: 0.2,
                        function: LayerFunction::Structure,
                    }],
                )
                .unwrap()
            }
        };
        m.add_slab(NewSlab {
            slab_type: st,
            level: lvl,
            outline: vec![
                Point2::new(0.0, 0.0),
                Point2::new(2.0, 0.0),
                Point2::new(2.0, 2.0),
                Point2::new(0.0, 2.0),
            ],
            top_offset: 0.05,
        })
        .unwrap();
        assert_eq!(m.floor_elevation_at(lvl, Point2::new(1.0, 1.0)), Some(0.05));
        assert_eq!(m.floor_elevation_at(lvl, Point2::new(5.0, 1.0)), Some(0.0));
        m.move_elevation_mark(mark, Point2::new(5.0, 1.0)).unwrap();
        // Saved and restored with everything else.
        let back = Model::from_parts(m.to_parts()).unwrap();
        assert_eq!(back, m);
        m.remove_elevation_mark(mark).unwrap();
        m.remove_dimension(d).unwrap();
        assert_eq!(m.elevation_marks().count() + m.dimensions().count(), 0);
    }

    #[test]
    fn stretching_a_wall_leaves_its_openings_in_place() {
        // The sample wall runs from (0, 0) to (4, 0).
        let (mut m, wall, door, _) = with_openings();
        let o = m.add_opening(place(door, wall, 2.0, 0.0)).unwrap();
        let guid = m.wall(wall).unwrap().guid.clone();
        // Pulling the start 1 m back: the door stays at x = 2, now 3 m in.
        m.stretch_wall(wall, Point2::new(-1.0, 0.0), Point2::new(4.0, 0.0))
            .unwrap();
        assert!((m.opening(o).unwrap().along - 3.0).abs() < 1e-12);
        assert_eq!(m.wall(wall).unwrap().guid, guid);
        // Pushing the start past the door is refused and changes nothing.
        assert_eq!(
            m.stretch_wall(wall, Point2::new(2.0, 0.0), Point2::new(4.0, 0.0)),
            Err(ModelError::OpeningOutsideWall)
        );
        assert_eq!(m.wall(wall).unwrap().start, Point2::new(-1.0, 0.0));
        assert!((m.opening(o).unwrap().along - 3.0).abs() < 1e-12);
        assert_eq!(
            m.stretch_wall(wall, Point2::new(4.0, 0.0), Point2::new(4.0, 0.0)),
            Err(ModelError::ZeroLengthWall)
        );
    }

    #[test]
    fn wall_properties_keep_guid_and_protect_openings() {
        let (mut m, wall, door, window) = with_openings();
        let guid = m.wall(wall).unwrap().guid.clone();
        m.add_opening(place(window, wall, 2.5, 0.9)).unwrap();

        // Lowering the 2.7 m wall below the window head (2.1 m) is refused
        // and leaves it untouched.
        assert_eq!(
            m.set_wall_height(wall, 2.0, 0.0),
            Err(ModelError::OpeningOutsideWall)
        );
        assert_eq!(m.wall(wall).unwrap().height, 2.7);
        assert!(matches!(
            m.set_wall_height(wall, 0.0, 0.0),
            Err(ModelError::NonPositive(_))
        ));
        m.set_wall_height(wall, 3.0, 0.0).unwrap();

        // A thinner type than the 7 cm frame is refused; a thicker one is fine.
        let brick = m.materials().next().unwrap().id;
        let layer = |thickness| Layer {
            material: brick,
            thickness,
            function: LayerFunction::Structure,
        };
        let thin = m
            .add_wall_type("Tabicón 5", WallFunction::Interior, vec![layer(0.05)])
            .unwrap();
        let thick = m
            .add_wall_type("Muro 25", WallFunction::Exterior, vec![layer(0.25)])
            .unwrap();
        assert_eq!(m.set_wall_type(wall, thin), Err(ModelError::FrameTooDeep));
        m.set_wall_type(wall, thick).unwrap();
        assert!(matches!(
            m.set_wall_type(wall, WallTypeId(999)),
            Err(ModelError::UnknownWallType(_))
        ));

        // The drawn line stays put when its position in the wall changes.
        let (start, end) = (m.wall(wall).unwrap().start, m.wall(wall).unwrap().end);
        m.set_wall_reference(wall, ReferenceLine::InteriorFace)
            .unwrap();
        let w = m.wall(wall).unwrap();
        assert_eq!(
            (w.start, w.end, w.reference),
            (start, end, ReferenceLine::InteriorFace)
        );
        assert_eq!(w.guid, guid);
        assert_eq!(w.wall_type, thick);

        // A door can be swapped for another door, not for a window.
        let o = m.add_opening(place(door, wall, 0.5, 0.0)).unwrap();
        assert_eq!(
            m.set_opening_type(o, window),
            Err(ModelError::CategoryMismatch)
        );
        let wood = m.opening_type(door).unwrap().frame_material;
        let wide = m
            .add_opening_type(NewOpeningType {
                name: "P 120".into(),
                kind: OpeningKind::Door(DoorOperation::DoubleSwing),
                width: 1.2,
                height: 2.05,
                frame_width: 0.05,
                frame_depth: 0.07,
                frame_material: wood,
                panel_material: wood,
            })
            .unwrap();
        // 1.2 m wide centred at 0.5 m would stick out of the wall start.
        assert_eq!(
            m.set_opening_type(o, wide),
            Err(ModelError::OpeningOutsideWall)
        );
        m.move_opening(o, 0.8, 0.0).unwrap();
        let oguid = m.opening(o).unwrap().guid.clone();
        m.set_opening_type(o, wide).unwrap();
        assert_eq!(m.opening(o).unwrap().opening_type, wide);
        assert_eq!(m.opening(o).unwrap().guid, oguid);
    }

    #[test]
    fn slab_type_and_offset() {
        let (mut m, lvl, _) = sample();
        let st = slab_type(&mut m);
        let concrete = m.materials().find(|x| x.name == "Hormigón").unwrap().id;
        let other = m
            .add_slab_type(
                "Losa 25",
                SlabFunction::Floor,
                vec![Layer {
                    material: concrete,
                    thickness: 0.25,
                    function: LayerFunction::Structure,
                }],
            )
            .unwrap();
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
        m.set_slab_type(id, other).unwrap();
        m.set_slab_offset(id, -0.05).unwrap();
        assert_eq!(m.slab(id).unwrap().slab_type, other);
        assert_eq!(m.slab(id).unwrap().top_offset, -0.05);
        assert!(matches!(
            m.set_slab_offset(id, f64::NAN),
            Err(ModelError::NotFinite(_))
        ));
        assert!(matches!(
            m.set_slab_type(id, SlabTypeId(999)),
            Err(ModelError::UnknownSlabType(_))
        ));
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

    #[test]
    fn parts_round_trip_keeps_everything() {
        let (mut m, wall, door, window) = with_openings();
        m.add_opening(place(door, wall, 0.6, 0.0)).unwrap();
        m.add_opening(place(window, wall, 2.5, 0.9)).unwrap();
        let st = slab_type(&mut m);
        let lvl = m.levels()[0].id;
        m.add_slab(NewSlab {
            slab_type: st,
            level: lvl,
            outline: vec![
                Point2::new(0.0, 0.0),
                Point2::new(4.0, 0.0),
                Point2::new(4.0, 3.0),
            ],
            top_offset: -0.05,
        })
        .unwrap();
        let restored = Model::from_parts(m.to_parts()).unwrap();
        assert_eq!(restored, m);
        // New elements keep getting fresh ids.
        let mut restored = restored;
        let new = restored.add_level("Planta 1", 3.0);
        assert!(m.to_parts().levels.iter().all(|l| l.id != new));
    }

    fn restore_error(parts: ModelParts) -> ModelError {
        Model::from_parts(parts).unwrap_err().error
    }

    #[test]
    fn restoring_rejects_inconsistent_parts() {
        let (mut m, wall, door, window) = with_openings();
        m.add_opening(place(door, wall, 0.6, 0.0)).unwrap();
        let good = m.to_parts();

        let mut p = good.clone();
        p.walls[0].guid = p.levels[0].guid.clone();
        assert!(matches!(restore_error(p), ModelError::DuplicateGuid(_)));

        let mut p = good.clone();
        p.building_guid = p.project_guid.clone();
        assert!(matches!(restore_error(p), ModelError::DuplicateGuid(_)));

        let mut p = good.clone();
        p.materials[0].id = MaterialId(p.levels[0].id.0);
        assert!(matches!(restore_error(p), ModelError::DuplicateId(_)));

        let mut p = good.clone();
        p.walls[0].level = LevelId(999);
        assert_eq!(restore_error(p), ModelError::UnknownLevel(LevelId(999)));

        let mut p = good.clone();
        p.walls[0].height = f64::NAN;
        assert!(matches!(restore_error(p), ModelError::NotFinite(_)));

        let mut p = good.clone();
        p.levels[0].elevation = f64::INFINITY;
        assert!(matches!(restore_error(p), ModelError::NotFinite(_)));

        let mut p = good.clone();
        p.levels[0].id = LevelId(0);
        assert_eq!(restore_error(p), ModelError::InvalidId(0));

        // A window moved by hand on top of the door.
        let mut p = good.clone();
        let mut extra = p.openings[0].clone();
        extra.id = OpeningId(500);
        extra.guid = GlobalId::new();
        extra.opening_type = window;
        extra.along = 1.0;
        extra.sill = 0.9;
        p.openings.push(extra);
        let err = Model::from_parts(p).unwrap_err();
        assert_eq!(err.error, ModelError::OverlappingOpenings);
        assert!(err.to_string().starts_with("door or window 500"));

        let mut p = good.clone();
        p.opening_types[0]
            .values
            .insert("frame_material".into(), Value::Material(777));
        assert_eq!(
            restore_error(p),
            ModelError::UnknownMaterial(MaterialId(777))
        );

        // The resolved fields are worked out again from the family: a
        // hand-edited width there is ignored…
        let mut p = good.clone();
        p.opening_types[0].width = 3.0;
        let restored = Model::from_parts(p).unwrap();
        assert_eq!(restored.opening_type(door).unwrap().width, 0.8);
        // …but a value of the family is checked.
        let mut p = good.clone();
        p.opening_types[0]
            .values
            .insert("leaves".into(), Value::Number(3.0));
        assert!(matches!(
            restore_error(p),
            ModelError::Family(FamilyError::OutOfRange { .. })
        ));

        let mut p = good.clone();
        p.opening_types[0].family = FamilyId(999);
        assert_eq!(restore_error(p), ModelError::UnknownFamily(FamilyId(999)));

        let mut p = good.clone();
        p.opening_types[1].mark = p.opening_types[0].mark.clone();
        assert_eq!(restore_error(p), ModelError::DuplicateMark("P-01".into()));

        let mut p = good.clone();
        let copy = Family::new(FamilyId(600), p.families[0].source.clone()).unwrap();
        p.families.push(copy);
        assert!(matches!(restore_error(p), ModelError::DuplicateFamily(_)));

        let mut p = good;
        p.openings[0]
            .values
            .insert("colour".into(), Value::Text("rojo".into()));
        assert!(matches!(
            restore_error(p),
            ModelError::Family(FamilyError::UnknownParam { .. })
        ));
    }

    #[test]
    fn edits_reject_non_finite_numbers() {
        let (mut m, lvl, t) = sample();
        let mut s = spec(lvl, t);
        s.end = Point2::new(f64::NAN, 0.0);
        assert!(matches!(m.add_wall(s), Err(ModelError::NotFinite(_))));
        let w = m.add_wall(spec(lvl, t)).unwrap();
        assert!(matches!(
            m.move_wall(w, Point2::new(0.0, 0.0), Point2::new(f64::INFINITY, 0.0)),
            Err(ModelError::NotFinite(_))
        ));
    }
}

#[cfg(test)]
mod slope_tests {
    use super::*;

    /// A ramp: 6 m long rising 8 %, its floor elevation follows it.
    #[test]
    fn inclined_slabs() {
        let mut m = Model::new("t");
        let l = m.add_level("PB", 0.0);
        let concrete = m.add_material("HA", "concrete", [180, 180, 180], CutPattern::Solid);
        let t = m
            .add_slab_type(
                "Losa",
                SlabFunction::Floor,
                vec![Layer {
                    material: concrete,
                    thickness: 0.2,
                    function: LayerFunction::Structure,
                }],
            )
            .unwrap();
        let s = m
            .add_slab(NewSlab {
                slab_type: t,
                level: l,
                outline: vec![
                    Point2::new(0.0, 0.0),
                    Point2::new(6.0, 0.0),
                    Point2::new(6.0, 2.0),
                    Point2::new(0.0, 2.0),
                ],
                top_offset: 0.0,
            })
            .unwrap();
        let ramp = SlabSlope {
            from: Point2::new(0.0, 1.0),
            to: Point2::new(6.0, 1.0),
            slope: 0.08,
        };
        m.set_slab_slope(s, Some(ramp)).unwrap();
        let slab = m.slab(s).unwrap();
        assert!((slab.top_at(Point2::new(6.0, 0.0)) - 0.48).abs() < 1e-12);
        assert_eq!(slab.top_range(), (0.0, 0.48));
        assert!((m.floor_elevation_at(l, Point2::new(3.0, 1.0)).unwrap() - 0.24).abs() < 1e-12);
        assert!(slab.vertical(0.2) > 0.2);
        // Moving carries the arrow along.
        m.move_slab(s, Point2::new(1.0, 0.0)).unwrap();
        assert_eq!(
            m.slab(s).unwrap().slope.unwrap().from,
            Point2::new(1.0, 1.0)
        );
        // No direction, or a wall rather than a ramp: refused.
        let mut flat = ramp;
        flat.to = flat.from;
        assert_eq!(
            m.set_slab_slope(s, Some(flat)),
            Err(ModelError::InvalidSlope)
        );
        let mut steep = ramp;
        steep.slope = 10.0;
        assert_eq!(
            m.set_slab_slope(s, Some(steep)),
            Err(ModelError::InvalidSlope)
        );
        let back = Model::from_parts(m.to_parts()).unwrap();
        assert_eq!(back, m);
        m.set_slab_slope(s, None).unwrap();
        assert_eq!(m.slab(s).unwrap().top_range(), (0.0, 0.0));
    }

    /// E6: a door or window type says where its frame sits in the wall
    /// (centred, from the exterior face or from the interior one), and a
    /// placed one without its own setback follows it.
    #[test]
    fn types_say_where_their_frame_sits() {
        let mut m = Model::new("t");
        let mat = m.add_material("Aluminio", "metal", [90, 90, 90], CutPattern::Solid);
        let id = m
            .add_opening_type(NewOpeningType {
                name: "V".into(),
                kind: OpeningKind::Window(WindowOperation::DoubleCasement),
                width: 1.2,
                height: 1.2,
                frame_width: 0.06,
                frame_depth: 0.07,
                frame_material: mat,
                panel_material: mat,
            })
            .unwrap();
        let t = m.opening_type(id).unwrap().clone();
        assert_eq!(t.frame_place, FramePlace::Centre);
        let o = Opening {
            id: OpeningId(0),
            guid: GlobalId::new(),
            opening_type: id,
            wall: WallId(0),
            along: 1.0,
            sill: 0.9,
            opens_to: WallSide::Interior,
            hand: Hand::Left,
            setback: None,
            values: Values::new(),
        };
        assert!((o.frame_setback(0.3, &t) - 0.115).abs() < 1e-9);
        for (position, inset, setback) in [("Por fuera", 0.02, 0.02), ("Por dentro", 0.03, 0.2)] {
            let mut values = t.values.clone();
            values.insert("frame_position".into(), Value::Text(position.into()));
            values.insert("frame_inset".into(), Value::Number(inset));
            m.edit_family_type(id, t.family, values).unwrap();
            let t = m.opening_type(id).unwrap();
            assert!(
                (o.frame_setback(0.3, t) - setback).abs() < 1e-9,
                "{position}"
            );
        }
        // Its own setback, if it has one, stands.
        let own = Opening {
            setback: Some(0.05),
            ..o
        };
        assert_eq!(own.frame_setback(0.3, m.opening_type(id).unwrap()), 0.05);
    }
}
