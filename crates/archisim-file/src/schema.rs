//! Version 3 of the `.archisim` file layout and its conversion to and from
//! the core model.
//!
//! The structs here are the file format, not the model: they change only
//! with a new format version, while the core model is free to evolve. Every
//! number is in metres; every reference between elements uses the element's
//! `id`. Unknown fields are rejected so that typing mistakes in a
//! hand-edited file are reported instead of silently ignored.

use archisim_core as core;
use archisim_core::{GlobalId, Point2};
use serde::{Deserialize, Serialize};

use crate::FileError;

/// Plan coordinates `[x, y]` in metres.
type Xy = [f64; 2];

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ProjectFile {
    pub format: String,
    pub version: u32,
    pub units: String,
    pub generator: String,
    pub project: Project,
    pub levels: Vec<Level>,
    pub materials: Vec<Material>,
    pub wall_types: Vec<WallType>,
    pub walls: Vec<Wall>,
    pub slab_types: Vec<SlabType>,
    pub slabs: Vec<Slab>,
    /// Family recipes used by the door and window types (version 3).
    pub families: Vec<Family>,
    pub opening_types: Vec<OpeningType>,
    pub openings: Vec<Opening>,
    /// Plan dimensions (version 2).
    pub dimensions: Vec<Dimension>,
    /// Level marks in plan (version 2).
    pub elevation_marks: Vec<ElevationMark>,
    /// Column and beam types (version 7).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub member_types: Vec<MemberType>,
    /// Columns (version 7).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub columns: Vec<Column>,
    /// Beams (version 7).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub beams: Vec<Beam>,
    /// Phases of the elements that are not new (version 7).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub phases: Vec<PhaseOf>,
    /// Disciplines given to elements by hand (version 15).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub disciplines: Vec<DisciplineOf>,
    /// Parameters of the project (version 13).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub user_parameters: Vec<UserParameter>,
    /// Values given to elements and types (version 13).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub parameter_values: Vec<ParameterValue>,
    /// Section and elevation lines (version 7).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub sections: Vec<Section>,
    /// Setting-out grid lines (version 16).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub grids: Vec<Grid>,
    /// Footings and tie beams (version 16).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub footings: Vec<Footing>,
    /// Pitched roofs (version 7).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub roofs: Vec<Roof>,
    /// Walls trimmed under a roof (version 7).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub wall_roofs: Vec<WallRoof>,
    /// Stairs (version 7).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub stairs: Vec<Stair>,
    /// Terrains (version 7).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub terrains: Vec<Terrain>,
    /// Curtain wall types (version 8).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub curtain_types: Vec<CurtainType>,
    /// Curtain walls (version 7; with a type from version 8).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub curtain_walls: Vec<CurtainWall>,
    /// Loose railings (version 7).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub railings: Vec<Railing>,
    /// 3D models brought from other programs (version 24), before the
    /// object types that show them.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub meshes: Vec<crate::meshes::Mesh>,
    /// Object types (version 7): a family with no wall and its values.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub object_types: Vec<ObjectType>,
    /// Objects: furniture, sanitary ware, kitchen units (version 7).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub objects: Vec<Object>,
    /// Module files kept in the project (version 17).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub module_sources: Vec<ModuleSource>,
    /// Copies of the modules (version 17).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub modules: Vec<Module>,
    /// Volumes of the concept phase (version 18).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub masses: Vec<Mass>,
    /// Loose edges of the concept phase (version 21).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub edges: Vec<Edge>,
    /// Paint on the faces of walls and volumes (version 21).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub face_paints: Vec<FacePaint>,
    /// Painted zones of walls' faces (version 21).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub paint_zones: Vec<PaintZoneOf>,
    /// How materials look in 3D: colour and texture (version 26).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub appearances: Vec<AppearanceOf>,
    /// Presentation styles of the views (version 22).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub presentation_styles: Vec<PresentationStyleOf>,
    /// Prices of the budget's items (version 31).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub prices: Vec<PriceOf>,
    /// Overheads, profit and VAT of the budget (version 32; absent: 13, 6
    /// and 21 %).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub budget: Option<Budget>,
    /// Saved plans, the looks of the levels' plans, view templates and
    /// reference planes (version 27).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub plan_views: Vec<crate::views::PlanViewOf>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub level_styles: Vec<crate::views::LevelStyleOf>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub view_templates: Vec<crate::views::ViewTemplateOf>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub ref_planes: Vec<crate::views::RefPlaneOf>,
    /// Plot area for the masses' figures, m² (version 18).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub plot_area: Option<f64>,
    /// Plots (version 19).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub plots: Vec<Plot>,
    /// Roads (version 19).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub roads: Vec<Road>,
    /// Plantings: plants along a line or over an area (version 25).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub plantings: Vec<PlantingOf>,
    /// Pipes and ducts (version 20).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub runs: Vec<Run>,
    /// Equipment, lights, sockets and switches (version 20).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub devices: Vec<Device>,
    /// Drawing layers (version 7).
    pub draw_layers: Vec<crate::drafting::DrawLayer>,
    /// Lines, arcs, texts and hatches of the views (version 7).
    pub drafts: Vec<crate::drafting::Draft>,
    /// Line weights by scale (version 7; the defaults if missing).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub graphics: Option<crate::drafting::Graphics>,
    /// Project data for the title block (version 7).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub info: Option<crate::drafting::Info>,
    /// Drawing sheets and the views placed on them (version 7).
    #[serde(default)]
    pub sheets: Vec<crate::drafting::Sheet>,
    #[serde(default)]
    pub viewports: Vec<crate::drafting::Viewport>,
    /// Rooms (IfcSpace) and tags of doors, windows, walls and rooms
    /// (version 7).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub rooms: Vec<crate::rooms::Room>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<crate::rooms::Tag>,
    /// Schedules placed on sheets (version 7).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tables: Vec<crate::drafting::Table>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ObjectType {
    pub id: u64,
    pub guid: String,
    pub name: String,
    pub family: u64,
    pub values: ParamValues,
    /// The 3D model it shows (version 24).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mesh: Option<u64>,
    /// A plan symbol from a DXF (version 30).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub symbol: Option<SymbolOf>,
}

/// An object type's plan symbol (version 30): its source, whether it is
/// stretched to the type's size, and its pieces as drafting shapes.
#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct SymbolOf {
    pub source: String,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub fit: bool,
    pub items: Vec<SymbolItemOf>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct SymbolItemOf {
    pub shape: crate::drafting::Shape,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub weight: Option<f64>,
}

impl SymbolOf {
    pub(crate) fn out(s: &core::PlanSymbol) -> Self {
        SymbolOf {
            source: s.source.clone(),
            fit: s.fit,
            items: s
                .items
                .iter()
                .map(|i| SymbolItemOf {
                    shape: crate::drafting::Shape::out(&i.shape),
                    color: i.color.map(crate::drafting::hex),
                    weight: i.weight,
                })
                .collect(),
        }
    }

    pub(crate) fn into_core(self, element: &str) -> Result<core::PlanSymbol, FileError> {
        let items = self
            .items
            .into_iter()
            .map(|i| {
                Ok(core::SymbolItem {
                    shape: i.shape.into_core(element)?,
                    color: i
                        .color
                        .map(|c| crate::drafting::parse_hex(element, &c))
                        .transpose()?,
                    weight: i.weight,
                })
            })
            .collect::<Result<Vec<_>, FileError>>()?;
        Ok(core::PlanSymbol {
            source: self.source,
            fit: self.fit,
            items,
        })
    }
}

/// An object (version 7): its type, level, insertion point, turn
/// (radians, counter-clockwise) and height above the level.
#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Object {
    pub id: u64,
    pub guid: String,
    #[serde(rename = "type")]
    pub object_type: u64,
    pub level: u64,
    pub position: Xy,
    #[serde(default, skip_serializing_if = "is_zero")]
    pub rotation: f64,
    #[serde(default, skip_serializing_if = "is_zero")]
    pub base_offset: f64,
    /// Its own instance values (version 23): how a tree's canopy is drawn…
    #[serde(default, skip_serializing_if = "serde_json::Map::is_empty")]
    pub values: ParamValues,
}

/// A railing (version 7): its line, foot, height, post spacing and
/// material.
#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Railing {
    pub id: u64,
    pub guid: String,
    pub level: u64,
    pub path: Vec<Xy>,
    pub base_offset: f64,
    pub height: f64,
    pub post_spacing: f64,
    pub material: u64,
    /// Glass panels instead of bars: their material.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub glass: Option<u64>,
}

/// A curtain wall (version 8): its type, line, foot and top (as walls).
#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct CurtainWall {
    pub id: u64,
    pub guid: String,
    #[serde(rename = "type")]
    pub curtain_type: u64,
    pub level: u64,
    pub start: Xy,
    pub end: Xy,
    pub base_offset: f64,
    pub height: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub top: Option<WallTop>,
}

/// How the rest of a curtain wall's length or height is divided.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub(crate) enum CurtainRest {
    MaxSpacing(f64),
    Count(usize),
}

/// Fixed bays from the start, then the rest.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct CurtainDivision {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub fixed: Vec<f64>,
    pub rest: CurtainRest,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct CurtainProfile {
    pub width: f64,
    pub depth: f64,
    pub material: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum CurtainJoint {
    #[default]
    Mullions,
    Transoms,
}

fn is_default_joint(j: &CurtainJoint) -> bool {
    *j == CurtainJoint::Mullions
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct CurtainPanel {
    pub name: String,
    pub thickness: f64,
    pub material: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct CurtainCaps {
    pub width: f64,
    pub depth: f64,
    pub material: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct CurtainLouvres {
    pub depth: f64,
    pub thickness: f64,
    pub spacing: f64,
    pub offset: f64,
    pub material: u64,
}

/// A curtain wall type (version 8): its grid, frames, panels (with the
/// panel of each row from the base) and extras.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct CurtainType {
    pub id: u64,
    pub guid: String,
    pub name: String,
    pub columns: CurtainDivision,
    pub rows: CurtainDivision,
    pub border: CurtainProfile,
    pub mullion: CurtainProfile,
    pub transom: CurtainProfile,
    #[serde(default, skip_serializing_if = "is_default_joint")]
    pub joint: CurtainJoint,
    pub panels: Vec<CurtainPanel>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub row_panels: Vec<usize>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub caps: Option<CurtainCaps>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub louvres: Option<CurtainLouvres>,
}

fn division_out(d: core::CurtainDivision) -> CurtainDivision {
    CurtainDivision {
        fixed: d.fixed,
        rest: match d.rest {
            core::CurtainRest::MaxSpacing(v) => CurtainRest::MaxSpacing(v),
            core::CurtainRest::Count(n) => CurtainRest::Count(n),
        },
    }
}

fn division_in(d: CurtainDivision) -> core::CurtainDivision {
    core::CurtainDivision {
        fixed: d.fixed,
        rest: match d.rest {
            CurtainRest::MaxSpacing(v) => core::CurtainRest::MaxSpacing(v),
            CurtainRest::Count(n) => core::CurtainRest::Count(n),
        },
    }
}

fn curtain_profile_out(p: core::CurtainProfile) -> CurtainProfile {
    CurtainProfile {
        width: p.width,
        depth: p.depth,
        material: p.material.0,
    }
}

fn curtain_profile_in(p: CurtainProfile) -> core::CurtainProfile {
    core::CurtainProfile {
        width: p.width,
        depth: p.depth,
        material: core::MaterialId(p.material),
    }
}

fn curtain_type_out(t: core::CurtainWallType) -> CurtainType {
    let d = t.design;
    CurtainType {
        id: t.id.0,
        guid: t.guid.to_string(),
        name: t.name,
        columns: division_out(d.columns),
        rows: division_out(d.rows),
        border: curtain_profile_out(d.border),
        mullion: curtain_profile_out(d.mullion),
        transom: curtain_profile_out(d.transom),
        joint: match d.joint {
            core::CurtainJoint::Mullions => CurtainJoint::Mullions,
            core::CurtainJoint::Transoms => CurtainJoint::Transoms,
        },
        panels: d
            .panels
            .into_iter()
            .map(|p| CurtainPanel {
                name: p.name,
                thickness: p.thickness,
                material: p.material.0,
            })
            .collect(),
        row_panels: d.row_panels,
        caps: d.caps.map(|c| CurtainCaps {
            width: c.width,
            depth: c.depth,
            material: c.material.0,
        }),
        louvres: d.louvres.map(|l| CurtainLouvres {
            depth: l.depth,
            thickness: l.thickness,
            spacing: l.spacing,
            offset: l.offset,
            material: l.material.0,
        }),
    }
}

fn curtain_type_in(t: CurtainType) -> Result<core::CurtainWallType, FileError> {
    Ok(core::CurtainWallType {
        id: core::CurtainTypeId(t.id),
        guid: GlobalId::parse(&t.guid).ok_or_else(|| FileError::Value {
            element: format!("curtain wall type {}", t.id),
            message: format!("«{}» is not a 22-character IFC GlobalId", t.guid),
        })?,
        name: t.name,
        design: core::CurtainDesign {
            columns: division_in(t.columns),
            rows: division_in(t.rows),
            border: curtain_profile_in(t.border),
            mullion: curtain_profile_in(t.mullion),
            transom: curtain_profile_in(t.transom),
            joint: match t.joint {
                CurtainJoint::Mullions => core::CurtainJoint::Mullions,
                CurtainJoint::Transoms => core::CurtainJoint::Transoms,
            },
            panels: t
                .panels
                .into_iter()
                .map(|p| core::CurtainPanel {
                    name: p.name,
                    thickness: p.thickness,
                    material: core::MaterialId(p.material),
                })
                .collect(),
            row_panels: t.row_panels,
            caps: t.caps.map(|c| core::CurtainCaps {
                width: c.width,
                depth: c.depth,
                material: core::MaterialId(c.material),
            }),
            louvres: t.louvres.map(|l| core::CurtainLouvres {
                depth: l.depth,
                thickness: l.thickness,
                spacing: l.spacing,
                offset: l.offset,
                material: core::MaterialId(l.material),
            }),
        },
    })
}

/// A terrain (version 7): survey points as `[x, y, z]` (z from project
/// zero), material, depth below the lowest point and contour interval.
#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Terrain {
    pub id: u64,
    pub guid: String,
    pub level: u64,
    pub points: Vec<[f64; 3]>,
    pub material: u64,
    pub depth: f64,
    pub contour: f64,
    /// Contour lines drawn by hand, as point indices (optional).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub lines: Vec<Vec<usize>>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum StairUse {
    General,
    Restricted,
}

/// A stair (version 7): its walking line, width, going aimed at, foot
/// and top (as walls), height, waist, material and use.
#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Stair {
    pub id: u64,
    pub guid: String,
    pub level: u64,
    pub path: Vec<Xy>,
    pub width: f64,
    pub going: f64,
    pub base_offset: f64,
    pub height: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub top: Option<WallTop>,
    pub waist: f64,
    pub material: u64,
    pub usage: StairUse,
    #[serde(default = "both", skip_serializing_if = "is_both")]
    pub railings: Railings,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum Railings {
    None,
    Left,
    Right,
    Both,
}

fn both() -> Railings {
    Railings::Both
}

fn is_both(r: &Railings) -> bool {
    *r == Railings::Both
}

/// A wall trimmed under a roof (version 7).
#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct WallRoof {
    pub wall: u64,
    pub roof: u64,
}

/// A pitched roof (version 7): its footprint, the pitch of each side
/// (`null`: a gable), the height of its top face at the footprint and the
/// overhang.
#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Roof {
    pub id: u64,
    pub guid: String,
    #[serde(rename = "type")]
    pub roof_type: u64,
    pub level: u64,
    pub outline: Vec<Xy>,
    pub slopes: Vec<Option<f64>>,
    pub base_offset: f64,
    pub overhang: f64,
    /// Its dormers (version 32).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub dormers: Vec<DormerOf>,
}

/// A dormer on a roof (version 32).
#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct DormerOf {
    pub position: Xy,
    pub width: f64,
    pub height: f64,
    /// `shed` or `gable`.
    pub kind: String,
    pub pitch: f64,
    pub wall_material: u64,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub window: bool,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum SectionKind {
    Section,
    Elevation,
}

/// A section or elevation line (version 7): it looks to the left of
/// `start` → `end`, `depth` metres far.
#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Section {
    pub id: u64,
    pub name: String,
    pub kind: SectionKind,
    pub start: Xy,
    pub end: Xy,
    pub depth: f64,
    /// Draws its heights (version 10; absent: yes).
    #[serde(default = "yes_heights", skip_serializing_if = "is_yes")]
    pub heights: bool,
}

/// A setting-out grid line (version 16).
#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Grid {
    pub id: u64,
    pub guid: String,
    pub name: String,
    pub start: Xy,
    pub end: Xy,
}

/// A module file kept in the project (version 17): where it is, the
/// fingerprint of the file when it was loaded, and its content (a whole
/// `.archisim` file).
#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ModuleSource {
    pub id: u64,
    pub name: String,
    pub path: String,
    pub stamp: String,
    pub content: serde_json::Value,
}

/// A pipe or duct (version 20).
#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Run {
    pub id: u64,
    pub guid: String,
    pub level: u64,
    pub system: String,
    pub path: Vec<Xy>,
    pub elevation: f64,
    pub width: f64,
    pub height: f64,
    /// Fall along the path (version 32).
    #[serde(default, skip_serializing_if = "is_zero")]
    pub slope: f64,
}

/// A device (version 20).
#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Device {
    pub id: u64,
    pub guid: String,
    pub level: u64,
    pub kind: String,
    pub name: String,
    pub position: Xy,
    #[serde(default, skip_serializing_if = "is_zero")]
    pub rotation: f64,
    pub elevation: f64,
    pub width: f64,
    pub depth: f64,
    pub height: f64,
}

/// A plot (version 19).
#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Plot {
    pub id: u64,
    pub guid: String,
    pub name: String,
    pub level: u64,
    pub outline: Vec<Xy>,
    #[serde(default, skip_serializing_if = "is_zero")]
    pub setback: f64,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub reference: String,
}

/// A road (version 19): its axis and cross-section.
#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Road {
    pub id: u64,
    pub guid: String,
    pub name: String,
    pub level: u64,
    pub axis: Vec<Xy>,
    pub carriageway: f64,
    pub parking: f64,
    pub kerb: f64,
    pub sidewalk: f64,
    pub kerb_height: f64,
    pub depth: f64,
    #[serde(default, skip_serializing_if = "is_zero")]
    pub top_offset: f64,
    pub pavement: u64,
    pub kerb_material: u64,
    pub sidewalk_material: u64,
}

/// A planting (version 25): its line, its rules and the objects it made.
#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct PlantingOf {
    pub id: u64,
    pub guid: String,
    pub name: String,
    pub level: u64,
    pub path: Vec<Xy>,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub closed: bool,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub smooth: bool,
    /// `line` or `area`.
    pub arrangement: String,
    pub count: u32,
    pub species: Vec<u64>,
    #[serde(default, skip_serializing_if = "is_zero")]
    pub size_variation: f64,
    #[serde(default, skip_serializing_if = "is_zero")]
    pub shape_variation: f64,
    #[serde(default, skip_serializing_if = "is_zero")]
    pub spread: f64,
    pub seed: u64,
    pub members: Vec<u64>,
    #[serde(default, skip_serializing_if = "is_zero")]
    pub base_offset: f64,
}

/// Paint on a face of a wall (version 21).
#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct FacePaint {
    pub element: u64,
    /// A wall's `exterior` or `interior`; a volume's `top`, `bottom` or
    /// `side 2` (the side on the edge from corner 2 to 3).
    pub face: String,
    pub material: u64,
}

/// A painted zone of a wall's face (version 21): its outline in metres
/// along the wall (x) and up from the level (y).
#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct PaintZoneOf {
    pub wall: u64,
    pub side: String,
    pub outline: Vec<Xy>,
    pub material: u64,
}

/// How a material looks in 3D (version 26).
#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct AppearanceOf {
    pub material: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub texture: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scale: Option<f64>,
}

/// Chamfered or rounded edges of a volume (version 26).
#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct EdgeFinishOf {
    /// `chamfer` or `round`.
    pub kind: String,
    pub size: f64,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub vertical: bool,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub top: bool,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub bottom: bool,
}

/// The price of an item of the measurements (version 31).
#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct PriceOf {
    /// The item of the measurements: "muro:3", "acabado:0:Gres"…
    pub item: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub code: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub unit: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub summary: String,
    pub price: f64,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub text: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub bank: String,
    /// Its decomposition (version 32).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub pieces: Vec<PieceOf>,
}

/// A piece of a price's decomposition (version 32).
#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct PieceOf {
    pub code: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub unit: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub summary: String,
    #[serde(default, skip_serializing_if = "is_zero_u8")]
    pub kind: u8,
    pub quantity: f64,
    pub price: f64,
}

fn is_zero_u8(v: &u8) -> bool {
    *v == 0
}

/// Overheads, profit and VAT (version 32).
#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Budget {
    pub overheads: f64,
    pub profit: f64,
    pub vat: f64,
}

impl PriceOf {
    fn out(item: &str, c: &core::PriceConcept) -> Self {
        PriceOf {
            item: item.to_string(),
            code: c.code.clone(),
            unit: c.unit.clone(),
            summary: c.summary.clone(),
            price: c.price,
            text: c.text.clone(),
            bank: c.bank.clone(),
            pieces: c
                .pieces
                .iter()
                .map(|p| PieceOf {
                    code: p.code.clone(),
                    unit: p.unit.clone(),
                    summary: p.summary.clone(),
                    kind: p.kind,
                    quantity: p.quantity,
                    price: p.price,
                })
                .collect(),
        }
    }

    fn into_core(self) -> (String, core::PriceConcept) {
        (
            self.item,
            core::PriceConcept {
                code: self.code,
                unit: self.unit,
                summary: self.summary,
                price: self.price,
                text: self.text,
                bank: self.bank,
                pieces: self
                    .pieces
                    .into_iter()
                    .map(|p| core::PricePiece {
                        code: p.code,
                        unit: p.unit,
                        summary: p.summary,
                        kind: p.kind,
                        quantity: p.quantity,
                        price: p.price,
                    })
                    .collect(),
            },
        )
    }
}

/// A presentation style (version 22).
#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct PresentationStyleOf {
    pub id: u64,
    pub name: String,
    pub background: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub palette: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub poche: Option<String>,
    pub line_scale: f64,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub shadows: bool,
}

impl PresentationStyleOf {
    pub(crate) fn out(s: &core::PresentationStyle) -> Self {
        use crate::drafting::hex;
        Self {
            id: s.id.0,
            name: s.name.clone(),
            background: hex(s.background),
            palette: s.palette.iter().map(|c| hex(*c)).collect(),
            poche: s.poche.map(hex),
            line_scale: s.line_scale,
            shadows: s.shadows,
        }
    }

    pub(crate) fn into_core(self) -> Result<core::PresentationStyle, FileError> {
        use crate::drafting::parse_hex;
        let element = format!("presentation style {}", self.id);
        Ok(core::PresentationStyle {
            id: core::PresentationStyleId(self.id),
            name: self.name,
            background: parse_hex(&element, &self.background)?,
            palette: self
                .palette
                .iter()
                .map(|c| parse_hex(&element, c))
                .collect::<Result<_, _>>()?,
            poche: self.poche.map(|c| parse_hex(&element, &c)).transpose()?,
            line_scale: self.line_scale,
            shadows: self.shadows,
        })
    }
}

/// A loose edge of the concept phase (version 21).
#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Edge {
    pub id: u64,
    pub guid: String,
    pub level: u64,
    pub points: Vec<Xy>,
    /// The height of each point (an edge drawn up a face); none, flat.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub heights: Vec<f64>,
    #[serde(default, skip_serializing_if = "is_zero")]
    pub elevation: f64,
}

/// A volume of the concept phase (version 18).
#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Mass {
    pub id: u64,
    pub guid: String,
    pub name: String,
    pub level: u64,
    pub shape: MassShapeOf,
    #[serde(default, skip_serializing_if = "is_zero")]
    pub base_offset: f64,
    pub height: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub floor_height: Option<f64>,
    pub material: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub building: Option<MassBuildingOf>,
    /// Chamfered or rounded edges (version 26).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub finish: Option<EdgeFinishOf>,
    /// Twisted or bent (version 28).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub deform: Option<DeformOf>,
}

/// A twist and a bend, in degrees (version 28).
#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct DeformOf {
    #[serde(default, skip_serializing_if = "is_zero")]
    pub twist: f64,
    #[serde(default, skip_serializing_if = "is_zero")]
    pub bend: f64,
    #[serde(default, skip_serializing_if = "is_zero")]
    pub bend_direction: f64,
}

impl DeformOf {
    pub(crate) fn out(d: &core::Deform) -> Self {
        DeformOf {
            twist: d.twist.to_degrees(),
            bend: d.bend.to_degrees(),
            bend_direction: d.bend_direction.to_degrees(),
        }
    }

    pub(crate) fn into_core(self) -> core::Deform {
        core::Deform {
            twist: self.twist.to_radians(),
            bend: self.bend.to_radians(),
            bend_direction: self.bend_direction.to_radians(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields, tag = "kind", rename_all = "snake_case")]
pub(crate) enum MassShapeOf {
    Prism {
        outline: Vec<Xy>,
    },
    Sweep {
        path: Vec<Xy>,
        width: f64,
        /// Any profile, across and up (version 28).
        #[serde(default, skip_serializing_if = "Option::is_none")]
        profile: Option<Vec<Xy>>,
    },
    /// Round forms (version 26).
    Sphere {
        center: Xy,
        radius: f64,
    },
    Cylinder {
        center: Xy,
        radius: f64,
    },
    Cone {
        center: Xy,
        radius: f64,
        top_radius: f64,
    },
    /// A half-section: (distance from the axis, height) of each point.
    Revolution {
        center: Xy,
        profile: Vec<Xy>,
    },
    /// A closed solid (version 26): points `[x, y, z]` (`z` over the
    /// base), triangles by point, and the face of each triangle.
    Mesh {
        points: Vec<[f64; 3]>,
        triangles: Vec<[u32; 3]>,
        faces: Vec<u32>,
    },
}

impl MassShapeOf {
    pub(crate) fn out(shape: &core::MassShape) -> Self {
        use core::{MassShape, Primitive};
        match shape {
            MassShape::Prism(o) => MassShapeOf::Prism {
                outline: o.iter().copied().map(xy).collect(),
            },
            MassShape::Sweep {
                path,
                width,
                profile,
            } => MassShapeOf::Sweep {
                path: path.iter().copied().map(xy).collect(),
                width: *width,
                profile: profile
                    .as_ref()
                    .map(|p| p.iter().copied().map(xy).collect()),
            },
            MassShape::Primitive(Primitive::Sphere { center, radius }) => MassShapeOf::Sphere {
                center: xy(*center),
                radius: *radius,
            },
            MassShape::Primitive(Primitive::Cylinder { center, radius }) => MassShapeOf::Cylinder {
                center: xy(*center),
                radius: *radius,
            },
            MassShape::Primitive(Primitive::Cone {
                center,
                radius,
                top_radius,
            }) => MassShapeOf::Cone {
                center: xy(*center),
                radius: *radius,
                top_radius: *top_radius,
            },
            MassShape::Primitive(Primitive::Revolution { center, profile }) => {
                MassShapeOf::Revolution {
                    center: xy(*center),
                    profile: profile.iter().copied().map(xy).collect(),
                }
            }
            MassShape::Mesh(m) => MassShapeOf::Mesh {
                points: m.positions.clone(),
                triangles: m.triangles.clone(),
                faces: m.faces.clone(),
            },
        }
    }

    pub(crate) fn into_core(self) -> core::MassShape {
        use core::{MassShape, Primitive};
        let pt = |p: Xy| Point2::new(p[0], p[1]);
        match self {
            MassShapeOf::Prism { outline } => {
                MassShape::Prism(outline.into_iter().map(pt).collect())
            }
            MassShapeOf::Sweep {
                path,
                width,
                profile,
            } => MassShape::Sweep {
                path: path.into_iter().map(pt).collect(),
                width,
                profile: profile.map(|p| p.into_iter().map(pt).collect()),
            },
            MassShapeOf::Sphere { center, radius } => MassShape::Primitive(Primitive::Sphere {
                center: pt(center),
                radius,
            }),
            MassShapeOf::Cylinder { center, radius } => MassShape::Primitive(Primitive::Cylinder {
                center: pt(center),
                radius,
            }),
            MassShapeOf::Cone {
                center,
                radius,
                top_radius,
            } => MassShape::Primitive(Primitive::Cone {
                center: pt(center),
                radius,
                top_radius,
            }),
            MassShapeOf::Revolution { center, profile } => {
                MassShape::Primitive(Primitive::Revolution {
                    center: pt(center),
                    profile: profile.into_iter().map(pt).collect(),
                })
            }
            MassShapeOf::Mesh {
                points,
                triangles,
                faces,
            } => MassShape::Mesh(core::MassMesh {
                positions: points,
                triangles,
                faces,
            }),
        }
    }
}

impl EdgeFinishOf {
    pub(crate) fn out(f: &core::EdgeFinish) -> Self {
        Self {
            kind: match f.kind {
                core::EdgeKind::Chamfer => "chamfer",
                core::EdgeKind::Round => "round",
            }
            .into(),
            size: f.size,
            vertical: f.vertical,
            top: f.top,
            bottom: f.bottom,
        }
    }

    pub(crate) fn into_core(self, element: &str) -> Result<core::EdgeFinish, FileError> {
        let kind = match self.kind.as_str() {
            "chamfer" => core::EdgeKind::Chamfer,
            "round" => core::EdgeKind::Round,
            k => {
                return Err(FileError::Value {
                    element: element.into(),
                    message: format!("unknown edge finish «{k}»"),
                })
            }
        };
        Ok(core::EdgeFinish {
            kind,
            size: self.size,
            vertical: self.vertical,
            top: self.top,
            bottom: self.bottom,
        })
    }
}

/// The building a mass was turned into (version 18).
#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct MassBuildingOf {
    pub wall_type: u64,
    pub ground_type: u64,
    pub floor_type: u64,
    pub roof_type: u64,
    pub elements: Vec<u64>,
    pub made_from: MassFrom,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct MassFrom {
    pub outline: Vec<Xy>,
    pub base_offset: f64,
    pub height: f64,
}

/// A copy of a module (version 17).
#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Module {
    pub id: u64,
    pub guid: String,
    pub source: u64,
    pub level: u64,
    pub position: Xy,
    #[serde(default, skip_serializing_if = "is_zero")]
    pub rotation: f64,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub mirror: bool,
}

/// A footing (version 16): `pad` at a point (under a column), `strip`
/// along a line (under a wall) or `tie_beam` between two points.
#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Footing {
    pub id: u64,
    pub guid: String,
    pub level: u64,
    pub kind: FootingKindOf,
    pub material: u64,
    pub width: f64,
    pub depth: f64,
    pub top_offset: f64,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields, tag = "shape", rename_all = "snake_case")]
pub(crate) enum FootingKindOf {
    Pad {
        position: Xy,
        length: f64,
        #[serde(default, skip_serializing_if = "is_zero")]
        rotation: f64,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        column: Option<u64>,
    },
    Strip {
        start: Xy,
        end: Xy,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        wall: Option<u64>,
    },
    TieBeam {
        start: Xy,
        end: Xy,
    },
}

fn footing_kind_out(k: core::FootingKind) -> FootingKindOf {
    match k {
        core::FootingKind::Pad {
            position,
            length,
            rotation,
            column,
        } => FootingKindOf::Pad {
            position: xy(position),
            length,
            rotation,
            column: column.map(|c| c.0),
        },
        core::FootingKind::Strip { start, end, wall } => FootingKindOf::Strip {
            start: xy(start),
            end: xy(end),
            wall: wall.map(|w| w.0),
        },
        core::FootingKind::TieBeam { start, end } => FootingKindOf::TieBeam {
            start: xy(start),
            end: xy(end),
        },
    }
}

fn point_of(p: Xy) -> Point2 {
    Point2::new(p[0], p[1])
}

fn footing_kind_in(k: &FootingKindOf) -> core::FootingKind {
    match *k {
        FootingKindOf::Pad {
            position,
            length,
            rotation,
            column,
        } => core::FootingKind::Pad {
            position: point_of(position),
            length,
            rotation,
            column: column.map(core::ColumnId),
        },
        FootingKindOf::Strip { start, end, wall } => core::FootingKind::Strip {
            start: point_of(start),
            end: point_of(end),
            wall: wall.map(core::WallId),
        },
        FootingKindOf::TieBeam { start, end } => core::FootingKind::TieBeam {
            start: point_of(start),
            end: point_of(end),
        },
    }
}

fn yes_heights() -> bool {
    true
}

fn is_yes(v: &bool) -> bool {
    *v
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum Phase {
    New,
    Existing,
    Demolished,
}

/// The discipline given by hand to one element (version 15).
#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct DisciplineOf {
    pub id: u64,
    pub discipline: Discipline,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum Discipline {
    Architecture,
    Structure,
    Services,
    Site,
}

impl Discipline {
    fn out(d: core::ElementDiscipline) -> Self {
        match d {
            core::ElementDiscipline::Architecture => Discipline::Architecture,
            core::ElementDiscipline::Structure => Discipline::Structure,
            core::ElementDiscipline::Services => Discipline::Services,
            core::ElementDiscipline::Site => Discipline::Site,
        }
    }

    fn into_core(self) -> core::ElementDiscipline {
        match self {
            Discipline::Architecture => core::ElementDiscipline::Architecture,
            Discipline::Structure => core::ElementDiscipline::Structure,
            Discipline::Services => core::ElementDiscipline::Services,
            Discipline::Site => core::ElementDiscipline::Site,
        }
    }
}

/// A parameter of the project (version 13).
#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct UserParameter {
    pub id: u64,
    pub guid: String,
    pub name: String,
    /// `text`, `number`, `integer`, `length`, `area`, `bool` or `choice`.
    pub kind: String,
    /// The options of a `choice`.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub options: Vec<String>,
    /// Its IFC property set.
    pub group: String,
    pub categories: Vec<String>,
    pub by_type: bool,
    /// Text, number or true/false.
    pub default: serde_json::Value,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub description: String,
}

/// A value of a parameter given to an element or a type (version 13).
#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ParameterValue {
    /// The element or type, by id.
    pub element: u64,
    pub parameter: u64,
    pub value: serde_json::Value,
}

fn user_value_out(v: &core::Value) -> serde_json::Value {
    match v {
        core::Value::Text(t) => serde_json::Value::from(t.as_str()),
        core::Value::Bool(b) => serde_json::Value::from(*b),
        core::Value::Number(n) => serde_json::Value::from(*n),
        core::Value::Material(m) => serde_json::Value::from(*m),
    }
}

fn user_value_in(v: &serde_json::Value) -> Option<core::Value> {
    Some(match v {
        serde_json::Value::String(t) => core::Value::Text(t.clone()),
        serde_json::Value::Bool(b) => core::Value::Bool(*b),
        serde_json::Value::Number(n) => core::Value::Number(n.as_f64()?),
        _ => return None,
    })
}

/// The phase of one element, by its id (version 7).
#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct PhaseOf {
    pub id: u64,
    pub phase: Phase,
}

/// A column or beam section (version 7), in metres.
#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields, tag = "shape", rename_all = "snake_case")]
pub(crate) enum Profile {
    Rectangle {
        width: f64,
        depth: f64,
    },
    Circle {
        diameter: f64,
    },
    IShape {
        width: f64,
        depth: f64,
        web: f64,
        flange: f64,
    },
    UShape {
        width: f64,
        depth: f64,
        web: f64,
        flange: f64,
    },
    TShape {
        width: f64,
        depth: f64,
        web: f64,
        flange: f64,
    },
    LShape {
        width: f64,
        depth: f64,
        thickness: f64,
    },
    RectHollow {
        width: f64,
        depth: f64,
        wall: f64,
    },
    CircleHollow {
        diameter: f64,
        wall: f64,
    },
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum MemberUsage {
    Column,
    Beam,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct MemberType {
    pub id: u64,
    pub guid: String,
    pub name: String,
    pub usage: MemberUsage,
    pub material: u64,
    pub profile: Profile,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Column {
    pub id: u64,
    pub guid: String,
    #[serde(rename = "type")]
    pub member_type: u64,
    pub level: u64,
    pub position: Xy,
    #[serde(default, skip_serializing_if = "is_zero")]
    pub rotation: f64,
    pub base_offset: f64,
    pub height: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub top: Option<WallTop>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Beam {
    pub id: u64,
    pub guid: String,
    #[serde(rename = "type")]
    pub member_type: u64,
    pub level: u64,
    pub start: Xy,
    pub end: Xy,
    pub top_offset: f64,
}

fn profile_out(p: core::Profile) -> Profile {
    match p {
        core::Profile::Rectangle { width, depth } => Profile::Rectangle { width, depth },
        core::Profile::Circle { diameter } => Profile::Circle { diameter },
        core::Profile::IShape {
            width,
            depth,
            web,
            flange,
        } => Profile::IShape {
            width,
            depth,
            web,
            flange,
        },
        core::Profile::UShape {
            width,
            depth,
            web,
            flange,
        } => Profile::UShape {
            width,
            depth,
            web,
            flange,
        },
        core::Profile::TShape {
            width,
            depth,
            web,
            flange,
        } => Profile::TShape {
            width,
            depth,
            web,
            flange,
        },
        core::Profile::LShape {
            width,
            depth,
            thickness,
        } => Profile::LShape {
            width,
            depth,
            thickness,
        },
        core::Profile::RectHollow { width, depth, wall } => {
            Profile::RectHollow { width, depth, wall }
        }
        core::Profile::CircleHollow { diameter, wall } => Profile::CircleHollow { diameter, wall },
    }
}

fn profile_in(p: Profile) -> core::Profile {
    match p {
        Profile::Rectangle { width, depth } => core::Profile::Rectangle { width, depth },
        Profile::Circle { diameter } => core::Profile::Circle { diameter },
        Profile::IShape {
            width,
            depth,
            web,
            flange,
        } => core::Profile::IShape {
            width,
            depth,
            web,
            flange,
        },
        Profile::UShape {
            width,
            depth,
            web,
            flange,
        } => core::Profile::UShape {
            width,
            depth,
            web,
            flange,
        },
        Profile::TShape {
            width,
            depth,
            web,
            flange,
        } => core::Profile::TShape {
            width,
            depth,
            web,
            flange,
        },
        Profile::LShape {
            width,
            depth,
            thickness,
        } => core::Profile::LShape {
            width,
            depth,
            thickness,
        },
        Profile::RectHollow { width, depth, wall } => {
            core::Profile::RectHollow { width, depth, wall }
        }
        Profile::CircleHollow { diameter, wall } => core::Profile::CircleHollow { diameter, wall },
    }
}

fn top_out(top: core::WallTop) -> Option<WallTop> {
    match top {
        core::WallTop::Unconnected => None,
        core::WallTop::Level { level, offset } => Some(WallTop {
            level: level.0,
            offset,
        }),
    }
}

fn top_in(top: Option<WallTop>) -> core::WallTop {
    match top {
        None => core::WallTop::Unconnected,
        Some(t) => core::WallTop::Level {
            level: core::LevelId(t.level),
            offset: t.offset,
        },
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Dimension {
    pub id: u64,
    pub level: u64,
    pub points: Vec<Xy>,
    pub direction: Xy,
    pub position: Xy,
    /// What each point is tied to (`null`: free); absent when none is.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub anchors: Vec<Option<Anchor>>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub(crate) enum AnchorSide {
    Exterior,
    Axis,
    Interior,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "snake_case", tag = "kind")]
pub(crate) enum Anchor {
    WallEnd {
        wall: u64,
        start: bool,
        side: AnchorSide,
    },
    Jamb {
        opening: u64,
        end: bool,
        side: AnchorSide,
    },
    SlabCorner {
        slab: u64,
        index: usize,
    },
}

impl Anchor {
    fn out(a: core::Anchor) -> Self {
        let side = |s: core::AnchorSide| match s {
            core::AnchorSide::Exterior => AnchorSide::Exterior,
            core::AnchorSide::Axis => AnchorSide::Axis,
            core::AnchorSide::Interior => AnchorSide::Interior,
        };
        match a {
            core::Anchor::WallEnd {
                wall,
                start,
                side: s,
            } => Anchor::WallEnd {
                wall: wall.0,
                start,
                side: side(s),
            },
            core::Anchor::Jamb {
                opening,
                end,
                side: s,
            } => Anchor::Jamb {
                opening: opening.0,
                end,
                side: side(s),
            },
            core::Anchor::SlabCorner { slab, index } => Anchor::SlabCorner {
                slab: slab.0,
                index,
            },
        }
    }

    fn into_core(self) -> core::Anchor {
        let side = |s: AnchorSide| match s {
            AnchorSide::Exterior => core::AnchorSide::Exterior,
            AnchorSide::Axis => core::AnchorSide::Axis,
            AnchorSide::Interior => core::AnchorSide::Interior,
        };
        match self {
            Anchor::WallEnd {
                wall,
                start,
                side: s,
            } => core::Anchor::WallEnd {
                wall: core::WallId(wall),
                start,
                side: side(s),
            },
            Anchor::Jamb {
                opening,
                end,
                side: s,
            } => core::Anchor::Jamb {
                opening: core::OpeningId(opening),
                end,
                side: side(s),
            },
            Anchor::SlabCorner { slab, index } => core::Anchor::SlabCorner {
                slab: core::SlabId(slab),
                index,
            },
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ElevationMark {
    pub id: u64,
    pub level: u64,
    pub point: Xy,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Project {
    pub name: String,
    pub guid: String,
    pub site_guid: String,
    pub building_guid: String,
    /// True north, degrees clockwise from the plan's up (version 7).
    #[serde(default, skip_serializing_if = "is_zero")]
    pub true_north: f64,
    /// The sun of the shadows (version 22): day of the year and solar
    /// hour; absent: the spring equinox at 11.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sun: Option<[f64; 2]>,
    /// Room uses offered in this project (version 9; `space_usages` in
    /// the version 7 and 8 files of phase 2); absent: the program's
    /// default list.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub room_usages: Vec<String>,
    /// Where the project is on the map (version 11).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub georeference: Option<Georeference>,
    /// Use, accessibility and sprinklers for the code checks (version
    /// 12); absent: a dwelling, without either.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub code: Option<Code>,
}

/// The building's settings for the code checks (version 12).
#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Code {
    /// `dwelling`, `public_residential`, `administrative`, `teaching`,
    /// `hospital`, `commercial`, `assembly` or `parking`.
    pub building_use: String,
    /// `none`, `routes` or `wheelchair_dwelling`.
    pub accessibility: String,
    #[serde(default)]
    pub sprinklers: bool,
}

/// Map coordinates of the ±0,00 (version 11).
#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Georeference {
    /// `EPSG:<code>`.
    pub crs: String,
    pub eastings: f64,
    pub northings: f64,
    pub height: f64,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Level {
    pub id: u64,
    pub guid: String,
    pub name: String,
    pub elevation: f64,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum CutPattern {
    Solid,
    Diagonal,
    Cross,
    Insulation,
    Dots,
    Empty,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Material {
    pub id: u64,
    pub name: String,
    pub category: String,
    /// `#RRGGBB`.
    pub cut_color: String,
    pub cut_pattern: CutPattern,
    /// Intersection priority, 0–999 (version 7); by layer function if
    /// absent.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub priority: Option<u16>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum LayerFunction {
    Structure,
    Substrate,
    Insulation,
    AirGap,
    Membrane,
    Finish,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Layer {
    pub material: u64,
    pub thickness: f64,
    pub function: LayerFunction,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum WallFunction {
    Exterior,
    Interior,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct WallType {
    pub id: u64,
    pub guid: String,
    pub name: String,
    pub function: WallFunction,
    /// From the exterior face to the interior face.
    pub layers: Vec<Layer>,
    /// Skirtings, bands, cornices and copings (version 29).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub sweeps: Vec<WallSweepOf>,
}

/// An accessory of a wall type (version 29).
#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct WallSweepOf {
    pub kind: String,
    pub side: String,
    #[serde(default, skip_serializing_if = "is_zero")]
    pub offset: f64,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub from_top: bool,
    pub profile: Vec<Xy>,
    pub material: u64,
}

impl WallSweepOf {
    pub(crate) fn out(s: &core::WallSweep) -> Self {
        WallSweepOf {
            kind: s.kind.code().into(),
            side: s.side.code().into(),
            offset: s.offset,
            from_top: s.from_top,
            profile: s.profile.iter().copied().map(xy).collect(),
            material: s.material.0,
        }
    }

    pub(crate) fn into_core(self, element: &str) -> Result<core::WallSweep, FileError> {
        let bad = |what: &str, v: &str| FileError::Value {
            element: element.to_string(),
            message: format!("unknown accessory {what} \"{v}\""),
        };
        Ok(core::WallSweep {
            kind: core::SweepKind::from_code(&self.kind)
                .ok_or_else(|| bad("accesorio", &self.kind))?,
            side: core::SweepSide::from_code(&self.side).ok_or_else(|| bad("lado", &self.side))?,
            offset: self.offset,
            from_top: self.from_top,
            profile: self
                .profile
                .into_iter()
                .map(|p| Point2::new(p[0], p[1]))
                .collect(),
            material: core::MaterialId(self.material),
        })
    }
}

/// The edge of a slab type (version 29).
#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct SlabEdgeOf {
    pub profile: Vec<Xy>,
    pub material: u64,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum ReferenceLine {
    Center,
    ExteriorFace,
    InteriorFace,
}

fn is_zero(v: &f64) -> bool {
    *v == 0.0
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Wall {
    pub id: u64,
    pub guid: String,
    #[serde(rename = "type")]
    pub wall_type: u64,
    pub level: u64,
    pub start: Xy,
    pub end: Xy,
    /// Curved walls (version 5): `tan(θ/4)` of the arc from `start` to
    /// `end`, positive counter-clockwise. Absent: a straight wall.
    #[serde(default, skip_serializing_if = "is_zero")]
    pub bulge: f64,
    pub reference: ReferenceLine,
    pub height: f64,
    pub base_offset: f64,
    /// Top constraint (version 4): up to a level. Absent: the wall has its
    /// own height.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub top: Option<WallTop>,
}

/// A wall's top level and the offset from it (version 4).
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct WallTop {
    pub level: u64,
    pub offset: f64,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum SlabFunction {
    Floor,
    BaseSlab,
    Roof,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct SlabType {
    pub id: u64,
    pub guid: String,
    pub name: String,
    pub function: SlabFunction,
    /// From the top face down to the soffit.
    pub layers: Vec<Layer>,
    /// A cornice or an eave round its slabs (version 29).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub edge: Option<SlabEdgeOf>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Slab {
    pub id: u64,
    pub guid: String,
    #[serde(rename = "type")]
    pub slab_type: u64,
    pub level: u64,
    pub outline: Vec<Xy>,
    pub top_offset: f64,
    /// Holes through the slab (version 6), each an outline.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub holes: Vec<Vec<Xy>>,
    /// Inclined slabs (version 7): the slope arrow.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub slope: Option<SlabSlope>,
}

/// Slope arrow of an inclined slab: the top face rises `slope` metres per
/// metre from `from` towards `to`.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct SlabSlope {
    pub from: Xy,
    pub to: Xy,
    pub slope: f64,
}

/// A family recipe, one text line per array item. `name` and `version`
/// repeat what the recipe says, for whoever reads the file.
#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Family {
    pub id: u64,
    pub name: String,
    pub version: u32,
    pub source: Vec<String>,
}

/// Parameter values by name: numbers (lengths in metres), `true`/`false`,
/// texts, and materials by their `id`.
pub(crate) type ParamValues = serde_json::Map<String, serde_json::Value>;

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct OpeningType {
    pub id: u64,
    pub guid: String,
    pub name: String,
    pub mark: String,
    pub description: String,
    pub family: u64,
    pub values: ParamValues,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum Side {
    Exterior,
    Interior,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum Hand {
    Left,
    Right,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Opening {
    pub id: u64,
    pub guid: String,
    #[serde(rename = "type")]
    pub opening_type: u64,
    pub wall: u64,
    pub along: f64,
    pub sill: f64,
    pub opens_to: Side,
    pub hand: Hand,
    /// `null` centres the frame in the wall.
    pub setback: Option<f64>,
    /// Instance parameters of the family, if it has any.
    #[serde(default, skip_serializing_if = "serde_json::Map::is_empty")]
    pub values: ParamValues,
}

// ---- Model → file ------------------------------------------------------

fn xy(p: Point2) -> Xy {
    [p.x, p.y]
}

fn hex(c: [u8; 3]) -> String {
    format!("#{:02X}{:02X}{:02X}", c[0], c[1], c[2])
}

fn layers_out(layers: &[core::Layer]) -> Vec<Layer> {
    layers
        .iter()
        .map(|l| Layer {
            material: l.material.0,
            thickness: l.thickness,
            function: match l.function {
                core::LayerFunction::Structure => LayerFunction::Structure,
                core::LayerFunction::Substrate => LayerFunction::Substrate,
                core::LayerFunction::Insulation => LayerFunction::Insulation,
                core::LayerFunction::AirGap => LayerFunction::AirGap,
                core::LayerFunction::Membrane => LayerFunction::Membrane,
                core::LayerFunction::Finish => LayerFunction::Finish,
            },
        })
        .collect()
}

impl ProjectFile {
    pub fn from_model(model: &core::Model) -> Self {
        let p = model.to_parts();
        ProjectFile {
            format: crate::FORMAT.into(),
            version: crate::CURRENT_VERSION,
            units: crate::UNITS.into(),
            generator: format!("ArchiSim {}", env!("CARGO_PKG_VERSION")),
            project: Project {
                name: p.project_name,
                guid: p.project_guid.to_string(),
                site_guid: p.site_guid.to_string(),
                building_guid: p.building_guid.to_string(),
                true_north: p.true_north.to_degrees(),
                sun: (p.sun != core::Sun::default()).then_some([p.sun.day as f64, p.sun.hour]),
                room_usages: p.room_usages,
                georeference: p.georeference.map(|g| Georeference {
                    crs: g.crs,
                    eastings: g.eastings,
                    northings: g.northings,
                    height: g.height,
                }),
                code: (p.code != core::CodeSettings::default()).then(|| Code {
                    building_use: p.code.building_use.code().into(),
                    accessibility: p.code.accessibility.code().into(),
                    sprinklers: p.code.sprinklers,
                }),
            },
            levels: p
                .levels
                .into_iter()
                .map(|l| Level {
                    id: l.id.0,
                    guid: l.guid.to_string(),
                    name: l.name,
                    elevation: l.elevation,
                })
                .collect(),
            materials: p
                .materials
                .into_iter()
                .map(|m| Material {
                    id: m.id.0,
                    name: m.name,
                    category: m.category,
                    cut_color: hex(m.cut_color),
                    priority: m.priority,
                    cut_pattern: match m.cut_pattern {
                        core::CutPattern::Solid => CutPattern::Solid,
                        core::CutPattern::Diagonal => CutPattern::Diagonal,
                        core::CutPattern::Cross => CutPattern::Cross,
                        core::CutPattern::Insulation => CutPattern::Insulation,
                        core::CutPattern::Dots => CutPattern::Dots,
                        core::CutPattern::Empty => CutPattern::Empty,
                    },
                })
                .collect(),
            wall_types: p
                .wall_types
                .into_iter()
                .map(|t| WallType {
                    id: t.id.0,
                    guid: t.guid.to_string(),
                    layers: layers_out(&t.layers),
                    sweeps: t.sweeps.iter().map(WallSweepOf::out).collect(),
                    name: t.name,
                    function: match t.function {
                        core::WallFunction::Exterior => WallFunction::Exterior,
                        core::WallFunction::Interior => WallFunction::Interior,
                    },
                })
                .collect(),
            walls: p
                .walls
                .into_iter()
                .map(|w| Wall {
                    id: w.id.0,
                    guid: w.guid.to_string(),
                    wall_type: w.wall_type.0,
                    level: w.level.0,
                    start: xy(w.start),
                    end: xy(w.end),
                    bulge: w.bulge,
                    reference: match w.reference {
                        core::ReferenceLine::Center => ReferenceLine::Center,
                        core::ReferenceLine::ExteriorFace => ReferenceLine::ExteriorFace,
                        core::ReferenceLine::InteriorFace => ReferenceLine::InteriorFace,
                    },
                    height: w.height,
                    base_offset: w.base_offset,
                    top: match w.top {
                        core::WallTop::Unconnected => None,
                        core::WallTop::Level { level, offset } => Some(WallTop {
                            level: level.0,
                            offset,
                        }),
                    },
                })
                .collect(),
            slab_types: p
                .slab_types
                .into_iter()
                .map(|t| SlabType {
                    id: t.id.0,
                    guid: t.guid.to_string(),
                    layers: layers_out(&t.layers),
                    edge: t.edge.as_ref().map(|e| SlabEdgeOf {
                        profile: e.profile.iter().copied().map(xy).collect(),
                        material: e.material.0,
                    }),
                    name: t.name,
                    function: match t.function {
                        core::SlabFunction::Floor => SlabFunction::Floor,
                        core::SlabFunction::BaseSlab => SlabFunction::BaseSlab,
                        core::SlabFunction::Roof => SlabFunction::Roof,
                    },
                })
                .collect(),
            slabs: p
                .slabs
                .into_iter()
                .map(|s| Slab {
                    id: s.id.0,
                    guid: s.guid.to_string(),
                    slab_type: s.slab_type.0,
                    level: s.level.0,
                    outline: s.outline.into_iter().map(xy).collect(),
                    top_offset: s.top_offset,
                    holes: s
                        .holes
                        .into_iter()
                        .map(|h| h.into_iter().map(xy).collect())
                        .collect(),
                    slope: s.slope.map(|a| SlabSlope {
                        from: xy(a.from),
                        to: xy(a.to),
                        slope: a.slope,
                    }),
                })
                .collect(),
            families: p
                .families
                .iter()
                .map(|f| Family {
                    id: f.id.0,
                    name: f.name().to_string(),
                    version: f.version(),
                    source: f.source.lines().map(str::to_string).collect(),
                })
                .collect(),
            opening_types: p
                .opening_types
                .into_iter()
                .map(|t| OpeningType {
                    id: t.id.0,
                    guid: t.guid.to_string(),
                    name: t.name,
                    mark: t.mark,
                    description: t.description,
                    family: t.family.0,
                    values: values_out(&t.values),
                })
                .collect(),
            openings: p
                .openings
                .into_iter()
                .map(|o| Opening {
                    id: o.id.0,
                    guid: o.guid.to_string(),
                    opening_type: o.opening_type.0,
                    wall: o.wall.0,
                    along: o.along,
                    sill: o.sill,
                    opens_to: match o.opens_to {
                        core::WallSide::Exterior => Side::Exterior,
                        core::WallSide::Interior => Side::Interior,
                    },
                    hand: match o.hand {
                        core::Hand::Left => Hand::Left,
                        core::Hand::Right => Hand::Right,
                    },
                    setback: o.setback,
                    values: values_out(&o.values),
                })
                .collect(),
            dimensions: p
                .dimensions
                .into_iter()
                .map(|d| Dimension {
                    id: d.id.0,
                    level: d.level.0,
                    points: d.points.into_iter().map(xy).collect(),
                    direction: xy(d.direction),
                    position: xy(d.position),
                    anchors: d.anchors.into_iter().map(|a| a.map(Anchor::out)).collect(),
                })
                .collect(),
            elevation_marks: p
                .elevation_marks
                .into_iter()
                .map(|m| ElevationMark {
                    id: m.id.0,
                    level: m.level.0,
                    point: xy(m.point),
                })
                .collect(),
            member_types: p
                .member_types
                .into_iter()
                .map(|t| MemberType {
                    id: t.id.0,
                    guid: t.guid.to_string(),
                    name: t.name,
                    usage: match t.usage {
                        core::MemberUsage::Column => MemberUsage::Column,
                        core::MemberUsage::Beam => MemberUsage::Beam,
                    },
                    material: t.material.0,
                    profile: profile_out(t.profile),
                })
                .collect(),
            columns: p
                .columns
                .into_iter()
                .map(|c| Column {
                    id: c.id.0,
                    guid: c.guid.to_string(),
                    member_type: c.member_type.0,
                    level: c.level.0,
                    position: xy(c.position),
                    rotation: c.rotation,
                    base_offset: c.base_offset,
                    height: c.height,
                    top: top_out(c.top),
                })
                .collect(),
            beams: p
                .beams
                .into_iter()
                .map(|b| Beam {
                    id: b.id.0,
                    guid: b.guid.to_string(),
                    member_type: b.member_type.0,
                    level: b.level.0,
                    start: xy(b.start),
                    end: xy(b.end),
                    top_offset: b.top_offset,
                })
                .collect(),
            user_parameters: p
                .user_parameters
                .iter()
                .map(|u| UserParameter {
                    id: u.id.0,
                    guid: u.guid.to_string(),
                    name: u.name.clone(),
                    kind: u.kind.code().to_string(),
                    options: match &u.kind {
                        core::UserKind::Choice(o) => o.clone(),
                        _ => Vec::new(),
                    },
                    group: u.group.clone(),
                    categories: u.categories.iter().map(|c| c.code().to_string()).collect(),
                    by_type: u.by_type,
                    default: user_value_out(&u.default),
                    description: u.description.clone(),
                })
                .collect(),
            parameter_values: p
                .parameter_values
                .iter()
                .map(|(element, param, v)| ParameterValue {
                    element: *element,
                    parameter: param.0,
                    value: user_value_out(v),
                })
                .collect(),
            phases: p
                .phases
                .into_iter()
                .map(|(id, ph)| PhaseOf {
                    id,
                    phase: match ph {
                        core::Phase::New => Phase::New,
                        core::Phase::Existing => Phase::Existing,
                        core::Phase::Demolished => Phase::Demolished,
                    },
                })
                .collect(),
            disciplines: p
                .disciplines
                .into_iter()
                .map(|(id, d)| DisciplineOf {
                    id,
                    discipline: Discipline::out(d),
                })
                .collect(),
            sections: p
                .sections
                .into_iter()
                .map(|s| Section {
                    id: s.id.0,
                    name: s.name,
                    kind: match s.kind {
                        core::SectionKind::Section => SectionKind::Section,
                        core::SectionKind::Elevation => SectionKind::Elevation,
                    },
                    start: xy(s.start),
                    end: xy(s.end),
                    depth: s.depth,
                    heights: s.heights,
                })
                .collect(),
            grids: p
                .grids
                .into_iter()
                .map(|g| Grid {
                    id: g.id.0,
                    guid: g.guid.to_string(),
                    name: g.name,
                    start: xy(g.start),
                    end: xy(g.end),
                })
                .collect(),
            footings: p
                .footings
                .into_iter()
                .map(|f| Footing {
                    id: f.id.0,
                    guid: f.guid.to_string(),
                    level: f.level.0,
                    kind: footing_kind_out(f.kind),
                    material: f.material.0,
                    width: f.width,
                    depth: f.depth,
                    top_offset: f.top_offset,
                })
                .collect(),
            roofs: p
                .roofs
                .into_iter()
                .map(|r| Roof {
                    id: r.id.0,
                    guid: r.guid.to_string(),
                    roof_type: r.roof_type.0,
                    level: r.level.0,
                    outline: r.outline.into_iter().map(xy).collect(),
                    slopes: r.slopes,
                    base_offset: r.base_offset,
                    overhang: r.overhang,
                    dormers: r
                        .dormers
                        .iter()
                        .map(|d| DormerOf {
                            position: xy(d.position),
                            width: d.width,
                            height: d.height,
                            kind: d.kind.code().to_string(),
                            pitch: d.pitch,
                            wall_material: d.wall_material.0,
                            window: d.window,
                        })
                        .collect(),
                })
                .collect(),
            terrains: p
                .terrains
                .into_iter()
                .map(|t| Terrain {
                    id: t.id.0,
                    guid: t.guid.to_string(),
                    level: t.level.0,
                    points: t.points.iter().map(|q| [q.at.x, q.at.y, q.z]).collect(),
                    material: t.material.0,
                    depth: t.depth,
                    contour: t.contour,
                    lines: t.lines,
                })
                .collect(),
            object_types: p
                .object_types
                .iter()
                .map(|t| ObjectType {
                    id: t.id.0,
                    guid: t.guid.to_string(),
                    name: t.name.clone(),
                    family: t.family.0,
                    values: values_out(&t.values),
                    mesh: t.mesh.map(|m| m.0),
                    symbol: t.symbol.as_ref().map(SymbolOf::out),
                })
                .collect(),
            meshes: p.meshes.iter().map(crate::meshes::mesh_out).collect(),
            face_paints: p
                .face_paints
                .iter()
                .map(|(e, f, m)| FacePaint {
                    element: *e,
                    face: f.code(),
                    material: m.0,
                })
                .collect(),
            appearances: p
                .appearances
                .iter()
                .map(|(m, a)| AppearanceOf {
                    material: m.0,
                    color: a.color.map(crate::drafting::hex),
                    texture: a.texture.map(|t| t.code().to_string()),
                    scale: a.scale,
                })
                .collect(),
            presentation_styles: p
                .presentation_styles
                .iter()
                .map(PresentationStyleOf::out)
                .collect(),
            prices: p.prices.iter().map(|(k, c)| PriceOf::out(k, c)).collect(),
            budget: (p.budget != core::BudgetSettings::default()).then_some(Budget {
                overheads: p.budget.overheads,
                profit: p.budget.profit,
                vat: p.budget.vat,
            }),
            plan_views: p
                .plan_views
                .iter()
                .map(crate::views::PlanViewOf::out)
                .collect(),
            level_styles: p
                .level_styles
                .iter()
                .map(|(l, s)| crate::views::LevelStyleOf::out(*l, s))
                .collect(),
            view_templates: p
                .view_templates
                .iter()
                .map(crate::views::ViewTemplateOf::out)
                .collect(),
            ref_planes: p
                .ref_planes
                .iter()
                .map(crate::views::RefPlaneOf::out)
                .collect(),
            paint_zones: p
                .paint_zones
                .iter()
                .map(|z| PaintZoneOf {
                    wall: z.wall.0,
                    side: z.side.code().into(),
                    outline: z.outline.iter().copied().map(xy).collect(),
                    material: z.material.0,
                })
                .collect(),
            edges: p
                .edges
                .iter()
                .map(|e| Edge {
                    id: e.id.0,
                    guid: e.guid.to_string(),
                    level: e.level.0,
                    points: e.points.iter().copied().map(xy).collect(),
                    heights: if e.heights.iter().all(|h| h.abs() < 1e-12) {
                        Vec::new()
                    } else {
                        e.heights.clone()
                    },
                    elevation: e.elevation,
                })
                .collect(),
            masses: p
                .masses
                .iter()
                .map(|m| Mass {
                    id: m.id.0,
                    guid: m.guid.to_string(),
                    name: m.name.clone(),
                    level: m.level.0,
                    shape: MassShapeOf::out(&m.shape),
                    finish: m.finish.as_ref().map(EdgeFinishOf::out),
                    deform: m.deform.as_ref().map(DeformOf::out),
                    base_offset: m.base_offset,
                    height: m.height,
                    floor_height: m.floor_height,
                    material: m.material.0,
                    building: m.building.as_ref().map(|b| MassBuildingOf {
                        wall_type: b.wall_type.0,
                        ground_type: b.ground_type.0,
                        floor_type: b.floor_type.0,
                        roof_type: b.roof_type.0,
                        elements: b.elements.clone(),
                        made_from: MassFrom {
                            outline: b.made_from.0.iter().copied().map(xy).collect(),
                            base_offset: b.made_from.1,
                            height: b.made_from.2,
                        },
                    }),
                })
                .collect(),
            plot_area: p.plot_area,
            plots: p
                .plots
                .iter()
                .map(|x| Plot {
                    id: x.id.0,
                    guid: x.guid.to_string(),
                    name: x.name.clone(),
                    level: x.level.0,
                    outline: x.outline.iter().copied().map(xy).collect(),
                    setback: x.setback,
                    reference: x.reference.clone(),
                })
                .collect(),
            plantings: p
                .plantings
                .iter()
                .map(|x| PlantingOf {
                    id: x.id.0,
                    guid: x.guid.to_string(),
                    name: x.name.clone(),
                    level: x.level.0,
                    path: x.path.iter().copied().map(xy).collect(),
                    closed: x.closed,
                    smooth: x.smooth,
                    arrangement: match x.arrangement {
                        core::Arrangement::Line => "line".into(),
                        core::Arrangement::Area => "area".into(),
                    },
                    count: x.count,
                    species: x.species.iter().map(|t| t.0).collect(),
                    size_variation: x.size_variation,
                    shape_variation: x.shape_variation,
                    spread: x.spread,
                    seed: x.seed,
                    members: x.members.iter().map(|m| m.0).collect(),
                    base_offset: x.base_offset,
                })
                .collect(),
            roads: p
                .roads
                .iter()
                .map(|r| Road {
                    id: r.id.0,
                    guid: r.guid.to_string(),
                    name: r.name.clone(),
                    level: r.level.0,
                    axis: r.axis.iter().copied().map(xy).collect(),
                    carriageway: r.section.carriageway,
                    parking: r.section.parking,
                    kerb: r.section.kerb,
                    sidewalk: r.section.sidewalk,
                    kerb_height: r.section.kerb_height,
                    depth: r.section.depth,
                    top_offset: r.top_offset,
                    pavement: r.pavement.0,
                    kerb_material: r.kerb_material.0,
                    sidewalk_material: r.sidewalk_material.0,
                })
                .collect(),
            runs: p
                .runs
                .iter()
                .map(|r| Run {
                    id: r.id.0,
                    guid: r.guid.to_string(),
                    level: r.level.0,
                    system: r.system.code().into(),
                    path: r.path.iter().copied().map(xy).collect(),
                    elevation: r.elevation,
                    width: r.width,
                    height: r.height,
                    slope: r.slope,
                })
                .collect(),
            devices: p
                .devices
                .iter()
                .map(|d| Device {
                    id: d.id.0,
                    guid: d.guid.to_string(),
                    level: d.level.0,
                    kind: d.kind.code().into(),
                    name: d.name.clone(),
                    position: xy(d.position),
                    rotation: d.rotation,
                    elevation: d.elevation,
                    width: d.width,
                    depth: d.depth,
                    height: d.height,
                })
                .collect(),
            module_sources: p
                .module_sources
                .iter()
                .map(|s| ModuleSource {
                    id: s.id.0,
                    name: s.name.clone(),
                    path: s.path.clone(),
                    stamp: s.stamp.clone(),
                    content: serde_json::from_str(&s.text).unwrap_or(serde_json::Value::Null),
                })
                .collect(),
            modules: p
                .modules
                .iter()
                .map(|m| Module {
                    id: m.id.0,
                    guid: m.guid.to_string(),
                    source: m.source.0,
                    level: m.level.0,
                    position: xy(m.position),
                    rotation: m.rotation,
                    mirror: m.mirror,
                })
                .collect(),
            objects: p
                .objects
                .iter()
                .map(|o| Object {
                    id: o.id.0,
                    guid: o.guid.to_string(),
                    object_type: o.object_type.0,
                    level: o.level.0,
                    position: xy(o.position),
                    rotation: o.rotation,
                    base_offset: o.base_offset,
                    values: values_out(&o.values),
                })
                .collect(),
            railings: p
                .railings
                .into_iter()
                .map(|r| Railing {
                    id: r.id.0,
                    guid: r.guid.to_string(),
                    level: r.level.0,
                    path: r.path.into_iter().map(xy).collect(),
                    base_offset: r.base_offset,
                    height: r.height,
                    post_spacing: r.post_spacing,
                    material: r.material.0,
                    glass: r.glass.map(|g| g.0),
                })
                .collect(),
            curtain_types: p.curtain_types.into_iter().map(curtain_type_out).collect(),
            curtain_walls: p
                .curtain_walls
                .into_iter()
                .map(|c| CurtainWall {
                    id: c.id.0,
                    guid: c.guid.to_string(),
                    curtain_type: c.curtain_type.0,
                    level: c.level.0,
                    start: xy(c.start),
                    end: xy(c.end),
                    base_offset: c.base_offset,
                    height: c.height,
                    top: top_out(c.top),
                })
                .collect(),
            wall_roofs: p
                .wall_roofs
                .iter()
                .map(|(w, r)| WallRoof {
                    wall: w.0,
                    roof: r.0,
                })
                .collect(),
            stairs: p
                .stairs
                .into_iter()
                .map(|s| Stair {
                    id: s.id.0,
                    guid: s.guid.to_string(),
                    level: s.level.0,
                    path: s.path.into_iter().map(xy).collect(),
                    width: s.width,
                    going: s.going,
                    base_offset: s.base_offset,
                    height: s.height,
                    top: top_out(s.top),
                    waist: s.waist,
                    material: s.material.0,
                    usage: match s.usage {
                        core::StairUse::General => StairUse::General,
                        core::StairUse::Restricted => StairUse::Restricted,
                    },
                    railings: match s.railings {
                        core::Railings::None => Railings::None,
                        core::Railings::Left => Railings::Left,
                        core::Railings::Right => Railings::Right,
                        core::Railings::Both => Railings::Both,
                    },
                })
                .collect(),
            draw_layers: crate::drafting::layers_out(&p.draw_layers),
            drafts: crate::drafting::drafts_out(&p.drafts),
            graphics: (p.graphics != core::GraphicStandard::default())
                .then(|| crate::drafting::Graphics::out(&p.graphics)),
            info: crate::drafting::Info::out(&p.info),
            sheets: crate::drafting::sheets_out(&p.sheets),
            viewports: crate::drafting::viewports_out(&p.viewports),
            rooms: crate::rooms::rooms_out(&p.rooms),
            tags: crate::rooms::tags_out(&p.tags),
            tables: crate::drafting::tables_out(&p.tables),
        }
    }

    // ---- File → model --------------------------------------------------

    pub fn into_model(self) -> Result<core::Model, FileError> {
        if self.units != crate::UNITS {
            return Err(FileError::Units(self.units));
        }
        let guid = |element: &str, text: &str| {
            GlobalId::parse(text).ok_or_else(|| FileError::Value {
                element: element.to_string(),
                message: format!("«{text}» is not a 22-character IFC GlobalId"),
            })
        };
        let pt = |p: Xy| Point2::new(p[0], p[1]);

        let project = self.project;
        let mut parts = core::ModelParts {
            project_guid: guid("project", &project.guid)?,
            site_guid: guid("project site", &project.site_guid)?,
            building_guid: guid("project building", &project.building_guid)?,
            project_name: project.name,
            levels: Vec::new(),
            materials: Vec::new(),
            wall_types: Vec::new(),
            walls: Vec::new(),
            slab_types: Vec::new(),
            slabs: Vec::new(),
            families: Vec::new(),
            opening_types: Vec::new(),
            openings: Vec::new(),
            dimensions: Vec::new(),
            elevation_marks: Vec::new(),
            member_types: Vec::new(),
            columns: Vec::new(),
            beams: Vec::new(),
            user_parameters: self
                .user_parameters
                .iter()
                .map(|u| {
                    let bad = |message: String| FileError::Value {
                        element: format!("parameter {} «{}»", u.id, u.name),
                        message,
                    };
                    let kind = match u.kind.as_str() {
                        "text" => core::UserKind::Text,
                        "number" => core::UserKind::Number,
                        "integer" => core::UserKind::Integer,
                        "length" => core::UserKind::Length,
                        "area" => core::UserKind::Area,
                        "bool" => core::UserKind::Bool,
                        "choice" => core::UserKind::Choice(u.options.clone()),
                        other => return Err(bad(format!("unknown kind «{other}»"))),
                    };
                    let categories = u
                        .categories
                        .iter()
                        .map(|c| {
                            core::ParamCategory::from_code(c)
                                .ok_or_else(|| bad(format!("unknown category «{c}»")))
                        })
                        .collect::<Result<Vec<_>, _>>()?;
                    Ok(core::UserParameter {
                        id: core::UserParameterId(u.id),
                        guid: guid(&format!("parameter {}", u.id), &u.guid)?,
                        name: u.name.clone(),
                        kind,
                        group: u.group.clone(),
                        categories,
                        by_type: u.by_type,
                        default: user_value_in(&u.default).ok_or_else(|| {
                            bad("its default is not a text, number or yes/no".into())
                        })?,
                        description: u.description.clone(),
                    })
                })
                .collect::<Result<Vec<_>, FileError>>()?,
            parameter_values: self
                .parameter_values
                .iter()
                .map(|v| {
                    user_value_in(&v.value)
                        .map(|x| (v.element, core::UserParameterId(v.parameter), x))
                        .ok_or_else(|| FileError::Value {
                            element: format!(
                                "value of parameter {} of element {}",
                                v.parameter, v.element
                            ),
                            message: "not a text, number or yes/no".into(),
                        })
                })
                .collect::<Result<Vec<_>, FileError>>()?,
            phases: self
                .phases
                .iter()
                .map(|p| {
                    (
                        p.id,
                        match p.phase {
                            Phase::New => core::Phase::New,
                            Phase::Existing => core::Phase::Existing,
                            Phase::Demolished => core::Phase::Demolished,
                        },
                    )
                })
                .collect(),
            disciplines: self
                .disciplines
                .iter()
                .map(|d| (d.id, d.discipline.into_core()))
                .collect(),
            sections: self
                .sections
                .iter()
                .map(|s| core::SectionLine {
                    id: core::SectionId(s.id),
                    name: s.name.clone(),
                    kind: match s.kind {
                        SectionKind::Section => core::SectionKind::Section,
                        SectionKind::Elevation => core::SectionKind::Elevation,
                    },
                    start: pt(s.start),
                    end: pt(s.end),
                    depth: s.depth,
                    heights: s.heights,
                })
                .collect(),
            grids: self
                .grids
                .iter()
                .map(|g| {
                    Ok(core::GridLine {
                        id: core::GridId(g.id),
                        guid: guid(&format!("grid line {}", g.id), &g.guid)?,
                        name: g.name.clone(),
                        start: pt(g.start),
                        end: pt(g.end),
                    })
                })
                .collect::<Result<Vec<_>, FileError>>()?,
            footings: self
                .footings
                .iter()
                .map(|f| {
                    Ok(core::Footing {
                        id: core::FootingId(f.id),
                        guid: guid(&format!("footing {}", f.id), &f.guid)?,
                        level: core::LevelId(f.level),
                        kind: footing_kind_in(&f.kind),
                        material: core::MaterialId(f.material),
                        width: f.width,
                        depth: f.depth,
                        top_offset: f.top_offset,
                    })
                })
                .collect::<Result<Vec<_>, FileError>>()?,
            roofs: self
                .roofs
                .iter()
                .map(|r| {
                    Ok(core::Roof {
                        id: core::RoofId(r.id),
                        guid: guid(&format!("roof {}", r.id), &r.guid)?,
                        roof_type: core::SlabTypeId(r.roof_type),
                        level: core::LevelId(r.level),
                        outline: r.outline.iter().copied().map(pt).collect(),
                        slopes: r.slopes.clone(),
                        base_offset: r.base_offset,
                        overhang: r.overhang,
                        dormers: r
                            .dormers
                            .iter()
                            .map(|d| {
                                Ok(core::Dormer {
                                    position: pt(d.position),
                                    width: d.width,
                                    height: d.height,
                                    kind: core::DormerKind::from_code(&d.kind).ok_or_else(
                                        || FileError::Value {
                                            element: format!("dormer of roof {}", r.id),
                                            message: format!("unknown kind «{}»", d.kind),
                                        },
                                    )?,
                                    pitch: d.pitch,
                                    wall_material: core::MaterialId(d.wall_material),
                                    window: d.window,
                                })
                            })
                            .collect::<Result<Vec<_>, FileError>>()?,
                    })
                })
                .collect::<Result<Vec<_>, FileError>>()?,
            terrains: self
                .terrains
                .iter()
                .map(|t| {
                    Ok(core::Terrain {
                        id: core::TerrainId(t.id),
                        guid: guid(&format!("terrain {}", t.id), &t.guid)?,
                        level: core::LevelId(t.level),
                        points: t
                            .points
                            .iter()
                            .map(|q| core::SurveyPoint {
                                at: core::Point2::new(q[0], q[1]),
                                z: q[2],
                            })
                            .collect(),
                        material: core::MaterialId(t.material),
                        depth: t.depth,
                        contour: t.contour,
                        lines: t.lines.clone(),
                    })
                })
                .collect::<Result<Vec<_>, FileError>>()?,
            curtain_types: self
                .curtain_types
                .iter()
                .cloned()
                .map(curtain_type_in)
                .collect::<Result<Vec<_>, FileError>>()?,
            curtain_walls: self
                .curtain_walls
                .iter()
                .map(|c| {
                    Ok(core::CurtainWall {
                        id: core::CurtainWallId(c.id),
                        guid: guid(&format!("curtain wall {}", c.id), &c.guid)?,
                        curtain_type: core::CurtainTypeId(c.curtain_type),
                        level: core::LevelId(c.level),
                        start: pt(c.start),
                        end: pt(c.end),
                        base_offset: c.base_offset,
                        top: top_in(c.top),
                        height: c.height,
                    })
                })
                .collect::<Result<Vec<_>, FileError>>()?,
            meshes: Vec::new(),
            object_types: Vec::new(),
            objects: Vec::new(),
            module_sources: Vec::new(),
            modules: Vec::new(),
            masses: Vec::new(),
            edges: Vec::new(),
            face_paints: Vec::new(),
            paint_zones: Vec::new(),
            appearances: Vec::new(),
            presentation_styles: Vec::new(),
            prices: self.prices.into_iter().map(PriceOf::into_core).collect(),
            budget: self.budget.map_or_else(core::BudgetSettings::default, |b| {
                core::BudgetSettings {
                    overheads: b.overheads,
                    profit: b.profit,
                    vat: b.vat,
                }
            }),
            plan_views: Vec::new(),
            level_styles: Vec::new(),
            view_templates: Vec::new(),
            ref_planes: Vec::new(),
            plot_area: None,
            plots: Vec::new(),
            roads: Vec::new(),
            plantings: Vec::new(),
            runs: Vec::new(),
            devices: Vec::new(),
            railings: self
                .railings
                .iter()
                .map(|r| {
                    Ok(core::Railing {
                        id: core::RailingId(r.id),
                        guid: guid(&format!("railing {}", r.id), &r.guid)?,
                        level: core::LevelId(r.level),
                        path: r.path.iter().copied().map(pt).collect(),
                        base_offset: r.base_offset,
                        height: r.height,
                        post_spacing: r.post_spacing,
                        material: core::MaterialId(r.material),
                        glass: r.glass.map(core::MaterialId),
                    })
                })
                .collect::<Result<Vec<_>, FileError>>()?,
            true_north: project.true_north.to_radians(),
            sun: project
                .sun
                .map_or(core::Sun::default(), |[d, h]| core::Sun {
                    day: d.clamp(0.0, 400.0) as u16,
                    hour: h,
                }),
            room_usages: project.room_usages.clone(),
            georeference: project.georeference.as_ref().map(|g| core::Georeference {
                crs: g.crs.clone(),
                eastings: g.eastings,
                northings: g.northings,
                height: g.height,
            }),
            code: match &project.code {
                None => core::CodeSettings::default(),
                Some(c) => core::CodeSettings {
                    building_use: core::BuildingUse::from_code(&c.building_use).ok_or_else(
                        || FileError::Value {
                            element: "project".into(),
                            message: format!("unknown building use «{}»", c.building_use),
                        },
                    )?,
                    accessibility: core::Accessibility::from_code(&c.accessibility).ok_or_else(
                        || FileError::Value {
                            element: "project".into(),
                            message: format!("unknown accessibility «{}»", c.accessibility),
                        },
                    )?,
                    sprinklers: c.sprinklers,
                },
            },
            wall_roofs: self
                .wall_roofs
                .iter()
                .map(|x| (core::WallId(x.wall), core::RoofId(x.roof)))
                .collect(),
            stairs: self
                .stairs
                .iter()
                .map(|s| {
                    Ok(core::Stair {
                        id: core::StairId(s.id),
                        guid: guid(&format!("stair {}", s.id), &s.guid)?,
                        level: core::LevelId(s.level),
                        path: s.path.iter().copied().map(pt).collect(),
                        width: s.width,
                        going: s.going,
                        base_offset: s.base_offset,
                        top: top_in(s.top),
                        height: s.height,
                        waist: s.waist,
                        material: core::MaterialId(s.material),
                        usage: match s.usage {
                            StairUse::General => core::StairUse::General,
                            StairUse::Restricted => core::StairUse::Restricted,
                        },
                        railings: match s.railings {
                            Railings::None => core::Railings::None,
                            Railings::Left => core::Railings::Left,
                            Railings::Right => core::Railings::Right,
                            Railings::Both => core::Railings::Both,
                        },
                    })
                })
                .collect::<Result<Vec<_>, FileError>>()?,
            draw_layers: crate::drafting::layers_in(self.draw_layers)?,
            drafts: crate::drafting::drafts_in(self.drafts)?,
            graphics: self
                .graphics
                .map(|g| g.into_core())
                .transpose()?
                .unwrap_or_default(),
            info: self.info.map(|i| i.into_core()).unwrap_or_default(),
            sheets: crate::drafting::sheets_in(self.sheets),
            viewports: crate::drafting::viewports_in(self.viewports)?,
            rooms: crate::rooms::rooms_in(self.rooms)?,
            tags: crate::rooms::tags_in(self.tags),
            tables: crate::drafting::tables_in(self.tables)?,
        };

        for l in self.levels {
            parts.levels.push(core::Level {
                id: core::LevelId(l.id),
                guid: guid(&format!("level {}", l.id), &l.guid)?,
                name: l.name,
                elevation: l.elevation,
            });
        }
        for m in self.materials {
            let cut_color = parse_hex(&m.cut_color).ok_or_else(|| FileError::Value {
                element: format!("material {}", m.id),
                message: format!("«{}» is not a colour like #E8C4B2", m.cut_color),
            })?;
            parts.materials.push(core::Material {
                id: core::MaterialId(m.id),
                name: m.name,
                category: m.category,
                cut_color,
                priority: m.priority,
                cut_pattern: match m.cut_pattern {
                    CutPattern::Solid => core::CutPattern::Solid,
                    CutPattern::Diagonal => core::CutPattern::Diagonal,
                    CutPattern::Cross => core::CutPattern::Cross,
                    CutPattern::Insulation => core::CutPattern::Insulation,
                    CutPattern::Dots => core::CutPattern::Dots,
                    CutPattern::Empty => core::CutPattern::Empty,
                },
            });
        }
        for t in self.wall_types {
            let element = format!("wall type {} «{}»", t.id, t.name);
            let sweeps = t
                .sweeps
                .into_iter()
                .map(|w| w.into_core(&element))
                .collect::<Result<Vec<_>, _>>()?;
            parts.wall_types.push(core::WallType {
                sweeps,
                id: core::WallTypeId(t.id),
                guid: guid(&format!("wall type {}", t.id), &t.guid)?,
                name: t.name,
                function: match t.function {
                    WallFunction::Exterior => core::WallFunction::Exterior,
                    WallFunction::Interior => core::WallFunction::Interior,
                },
                layers: layers_in(t.layers),
            });
        }
        for w in self.walls {
            parts.walls.push(core::Wall {
                id: core::WallId(w.id),
                guid: guid(&format!("wall {}", w.id), &w.guid)?,
                wall_type: core::WallTypeId(w.wall_type),
                level: core::LevelId(w.level),
                start: pt(w.start),
                end: pt(w.end),
                bulge: w.bulge,
                reference: match w.reference {
                    ReferenceLine::Center => core::ReferenceLine::Center,
                    ReferenceLine::ExteriorFace => core::ReferenceLine::ExteriorFace,
                    ReferenceLine::InteriorFace => core::ReferenceLine::InteriorFace,
                },
                height: w.height,
                base_offset: w.base_offset,
                top: match w.top {
                    None => core::WallTop::Unconnected,
                    Some(t) => core::WallTop::Level {
                        level: core::LevelId(t.level),
                        offset: t.offset,
                    },
                },
            });
        }
        for t in self.slab_types {
            parts.slab_types.push(core::SlabType {
                id: core::SlabTypeId(t.id),
                guid: guid(&format!("slab type {}", t.id), &t.guid)?,
                name: t.name,
                function: match t.function {
                    SlabFunction::Floor => core::SlabFunction::Floor,
                    SlabFunction::BaseSlab => core::SlabFunction::BaseSlab,
                    SlabFunction::Roof => core::SlabFunction::Roof,
                },
                layers: layers_in(t.layers),
                edge: t.edge.map(|e| core::SlabEdge {
                    profile: e
                        .profile
                        .into_iter()
                        .map(|p| Point2::new(p[0], p[1]))
                        .collect(),
                    material: core::MaterialId(e.material),
                }),
            });
        }
        for s in self.slabs {
            parts.slabs.push(core::Slab {
                id: core::SlabId(s.id),
                guid: guid(&format!("slab {}", s.id), &s.guid)?,
                slab_type: core::SlabTypeId(s.slab_type),
                level: core::LevelId(s.level),
                outline: s.outline.into_iter().map(pt).collect(),
                top_offset: s.top_offset,
                holes: s
                    .holes
                    .into_iter()
                    .map(|h| h.into_iter().map(pt).collect())
                    .collect(),
                slope: s.slope.map(|a| core::SlabSlope {
                    from: pt(a.from),
                    to: pt(a.to),
                    slope: a.slope,
                }),
            });
        }
        for f in self.families {
            let element = format!("family {} «{}»", f.id, f.name);
            let mut text = f.source.join("\n");
            text.push('\n');
            let family =
                core::Family::new(core::FamilyId(f.id), text).map_err(|e| FileError::Value {
                    element: element.clone(),
                    message: e.to_string(),
                })?;
            if family.name() != f.name || family.version() != f.version {
                return Err(FileError::Value {
                    element,
                    message: format!(
                        "the recipe is «{}» version {}: «name» and «version» must match it",
                        family.name(),
                        family.version()
                    ),
                });
            }
            parts.families.push(family);
        }
        let mut type_family = std::collections::HashMap::new();
        for t in self.opening_types {
            let element = format!("door or window type {} «{}»", t.id, t.name);
            let family = parts.families.iter().find(|f| f.id.0 == t.family);
            let values = values_in(family, core::ParamScope::Type, t.values, &element)?;
            type_family.insert(t.id, family);
            parts.opening_types.push(core::OpeningType {
                id: core::OpeningTypeId(t.id),
                guid: guid(&element, &t.guid)?,
                name: t.name,
                mark: t.mark,
                description: t.description,
                family: core::FamilyId(t.family),
                values,
                // Worked out again from the family when the model is built.
                kind: core::OpeningKind::Door(core::DoorOperation::SingleSwing),
                width: 0.0,
                height: 0.0,
                frame_width: 0.0,
                frame_depth: 0.0,
                frame_material: core::MaterialId(0),
                panel_material: core::MaterialId(0),
                panels: Vec::new(),
            });
        }
        for o in self.openings {
            let element = format!("door or window {}", o.id);
            let family = type_family.get(&o.opening_type).copied().flatten();
            let values = values_in(family, core::ParamScope::Instance, o.values, &element)?;
            parts.openings.push(core::Opening {
                id: core::OpeningId(o.id),
                guid: guid(&format!("door or window {}", o.id), &o.guid)?,
                opening_type: core::OpeningTypeId(o.opening_type),
                wall: core::WallId(o.wall),
                along: o.along,
                sill: o.sill,
                opens_to: match o.opens_to {
                    Side::Exterior => core::WallSide::Exterior,
                    Side::Interior => core::WallSide::Interior,
                },
                hand: match o.hand {
                    Hand::Left => core::Hand::Left,
                    Hand::Right => core::Hand::Right,
                },
                setback: o.setback,
                values,
            });
        }

        for t in self.object_types {
            let element = format!("object type {} «{}»", t.id, t.name);
            let family = parts.families.iter().find(|f| f.id.0 == t.family);
            let values = values_in(family, core::ParamScope::Type, t.values, &element)?;
            parts.object_types.push(core::ObjectType {
                id: core::ObjectTypeId(t.id),
                guid: guid(&element, &t.guid)?,
                name: t.name,
                family: core::FamilyId(t.family),
                values,
                mesh: t.mesh.map(core::MeshId),
                symbol: t.symbol.map(|s| s.into_core(&element)).transpose()?,
            });
        }
        for m in self.meshes {
            parts.meshes.push(crate::meshes::mesh_in(m)?);
        }
        for f in self.face_paints {
            let face = core::FaceRef::from_code(&f.face).ok_or_else(|| FileError::Value {
                element: format!("paint of element {}", f.element),
                message: format!("unknown face «{}»", f.face),
            })?;
            parts
                .face_paints
                .push((f.element, face, core::MaterialId(f.material)));
        }
        for st in self.presentation_styles {
            parts.presentation_styles.push(st.into_core()?);
        }
        for v in self.plan_views {
            parts.plan_views.push(v.into_core()?);
        }
        for l in self.level_styles {
            parts.level_styles.push(l.into_core()?);
        }
        for t in self.view_templates {
            parts.view_templates.push(t.into_core()?);
        }
        for r in self.ref_planes {
            parts.ref_planes.push(r.into_core());
        }
        for a in self.appearances {
            let element = format!("appearance of material {}", a.material);
            let texture = match a.texture {
                Some(t) => {
                    Some(
                        core::TextureKind::from_code(&t).ok_or_else(|| FileError::Value {
                            element: element.clone(),
                            message: format!("unknown texture «{t}»"),
                        })?,
                    )
                }
                None => None,
            };
            let color = a
                .color
                .map(|c| crate::drafting::parse_hex(&element, &c))
                .transpose()?;
            parts.appearances.push((
                core::MaterialId(a.material),
                core::Appearance {
                    color,
                    texture,
                    scale: a.scale,
                },
            ));
        }
        for z in self.paint_zones {
            let side = core::FaceSide::from_code(&z.side).ok_or_else(|| FileError::Value {
                element: format!("painted zone of wall {}", z.wall),
                message: format!("unknown side «{}»", z.side),
            })?;
            parts.paint_zones.push(core::PaintZone {
                wall: core::WallId(z.wall),
                side,
                outline: z.outline.into_iter().map(pt).collect(),
                material: core::MaterialId(z.material),
            });
        }
        for e in self.edges {
            let element = format!("edge {}", e.id);
            parts.edges.push(core::ConceptEdge {
                id: core::EdgeId(e.id),
                guid: guid(&element, &e.guid)?,
                level: core::LevelId(e.level),
                heights: if e.heights.is_empty() {
                    vec![0.0; e.points.len()]
                } else {
                    e.heights
                },
                points: e.points.into_iter().map(pt).collect(),
                elevation: e.elevation,
            });
        }
        for m in self.masses {
            let element = format!("mass {} «{}»", m.id, m.name);
            parts.masses.push(core::Mass {
                id: core::MassId(m.id),
                guid: guid(&element, &m.guid)?,
                name: m.name,
                level: core::LevelId(m.level),
                shape: m.shape.into_core(),
                finish: m.finish.map(|f| f.into_core(&element)).transpose()?,
                deform: m.deform.map(DeformOf::into_core),
                base_offset: m.base_offset,
                height: m.height,
                floor_height: m.floor_height,
                material: core::MaterialId(m.material),
                building: m.building.map(|b| core::MassBuilding {
                    wall_type: core::WallTypeId(b.wall_type),
                    ground_type: core::SlabTypeId(b.ground_type),
                    floor_type: core::SlabTypeId(b.floor_type),
                    roof_type: core::SlabTypeId(b.roof_type),
                    elements: b.elements,
                    made_from: (
                        b.made_from.outline.into_iter().map(pt).collect(),
                        b.made_from.base_offset,
                        b.made_from.height,
                    ),
                }),
            });
        }
        parts.plot_area = self.plot_area;
        for x in self.plots {
            let element = format!("plot {} «{}»", x.id, x.name);
            parts.plots.push(core::Plot {
                id: core::PlotId(x.id),
                guid: guid(&element, &x.guid)?,
                name: x.name,
                level: core::LevelId(x.level),
                outline: x.outline.into_iter().map(pt).collect(),
                setback: x.setback,
                reference: x.reference,
            });
        }
        for r in self.runs {
            let element = format!("run {}", r.id);
            parts.runs.push(core::ServiceRun {
                id: core::RunId(r.id),
                guid: guid(&element, &r.guid)?,
                level: core::LevelId(r.level),
                system: core::ServiceSystem::from_code(&r.system).ok_or_else(|| {
                    FileError::Value {
                        element: element.clone(),
                        message: format!("«{}» is not a system", r.system),
                    }
                })?,
                path: r.path.into_iter().map(pt).collect(),
                elevation: r.elevation,
                width: r.width,
                height: r.height,
                slope: r.slope,
            });
        }
        for d in self.devices {
            let element = format!("device {} «{}»", d.id, d.name);
            parts.devices.push(core::ServiceDevice {
                id: core::DeviceId(d.id),
                guid: guid(&element, &d.guid)?,
                level: core::LevelId(d.level),
                kind: core::DeviceKind::from_code(&d.kind).ok_or_else(|| FileError::Value {
                    element: element.clone(),
                    message: format!("«{}» is not a kind of device", d.kind),
                })?,
                name: d.name,
                position: pt(d.position),
                rotation: d.rotation,
                elevation: d.elevation,
                width: d.width,
                depth: d.depth,
                height: d.height,
            });
        }
        for x in self.plantings {
            let element = format!("planting {} «{}»", x.id, x.name);
            parts.plantings.push(core::Planting {
                id: core::PlantingId(x.id),
                guid: guid(&element, &x.guid)?,
                name: x.name,
                level: core::LevelId(x.level),
                path: x.path.into_iter().map(pt).collect(),
                closed: x.closed,
                smooth: x.smooth,
                arrangement: match x.arrangement.as_str() {
                    "line" => core::Arrangement::Line,
                    "area" => core::Arrangement::Area,
                    other => {
                        return Err(FileError::Value {
                            element,
                            message: format!("«{other}» is not a way of planting"),
                        })
                    }
                },
                count: x.count,
                species: x.species.into_iter().map(core::ObjectTypeId).collect(),
                size_variation: x.size_variation,
                shape_variation: x.shape_variation,
                spread: x.spread,
                seed: x.seed,
                members: x.members.into_iter().map(core::ObjectId).collect(),
                base_offset: x.base_offset,
            });
        }
        for r in self.roads {
            let element = format!("road {} «{}»", r.id, r.name);
            parts.roads.push(core::Road {
                id: core::RoadId(r.id),
                guid: guid(&element, &r.guid)?,
                name: r.name,
                level: core::LevelId(r.level),
                axis: r.axis.into_iter().map(pt).collect(),
                section: core::RoadSection {
                    carriageway: r.carriageway,
                    parking: r.parking,
                    kerb: r.kerb,
                    sidewalk: r.sidewalk,
                    kerb_height: r.kerb_height,
                    depth: r.depth,
                },
                top_offset: r.top_offset,
                pavement: core::MaterialId(r.pavement),
                kerb_material: core::MaterialId(r.kerb_material),
                sidewalk_material: core::MaterialId(r.sidewalk_material),
            });
        }
        for s in self.module_sources {
            let element = format!("module {} «{}»", s.id, s.name);
            let text: std::sync::Arc<str> = crate::pretty(&s.content).into();
            let model = crate::from_str(&text).map_err(|e| FileError::Value {
                element: element.clone(),
                message: format!("the module file inside does not open: {e}"),
            })?;
            parts.module_sources.push(core::ModuleSource {
                id: core::ModuleSourceId(s.id),
                name: s.name,
                path: s.path,
                text,
                stamp: s.stamp,
                model: std::sync::Arc::new(model),
            });
        }
        for m in self.modules {
            let element = format!("module copy {}", m.id);
            parts.modules.push(core::ModulePlacement {
                id: core::ModuleId(m.id),
                guid: guid(&element, &m.guid)?,
                source: core::ModuleSourceId(m.source),
                level: core::LevelId(m.level),
                position: pt(m.position),
                rotation: m.rotation,
                mirror: m.mirror,
            });
        }
        for o in self.objects {
            let element = format!("object {}", o.id);
            let family = parts
                .object_types
                .iter()
                .find(|t| t.id.0 == o.object_type)
                .and_then(|t| parts.families.iter().find(|f| f.id == t.family));
            let values = values_in(family, core::ParamScope::Instance, o.values, &element)?;
            parts.objects.push(core::Object {
                id: core::ObjectId(o.id),
                guid: guid(&element, &o.guid)?,
                level: core::LevelId(o.level),
                object_type: core::ObjectTypeId(o.object_type),
                position: pt(o.position),
                rotation: o.rotation,
                base_offset: o.base_offset,
                values,
            });
        }

        for d in self.dimensions {
            parts.dimensions.push(core::Dimension {
                id: core::DimensionId(d.id),
                level: core::LevelId(d.level),
                points: d.points.into_iter().map(pt).collect(),
                direction: pt(d.direction),
                position: pt(d.position),
                anchors: d
                    .anchors
                    .into_iter()
                    .map(|a| a.map(Anchor::into_core))
                    .collect(),
            });
        }
        for m in self.elevation_marks {
            parts.elevation_marks.push(core::ElevationMark {
                id: core::ElevationMarkId(m.id),
                level: core::LevelId(m.level),
                point: pt(m.point),
            });
        }
        for t in self.member_types {
            parts.member_types.push(core::MemberType {
                id: core::MemberTypeId(t.id),
                guid: guid(&format!("member type {}", t.id), &t.guid)?,
                name: t.name,
                usage: match t.usage {
                    MemberUsage::Column => core::MemberUsage::Column,
                    MemberUsage::Beam => core::MemberUsage::Beam,
                },
                material: core::MaterialId(t.material),
                profile: profile_in(t.profile),
            });
        }
        for c in self.columns {
            parts.columns.push(core::Column {
                id: core::ColumnId(c.id),
                guid: guid(&format!("column {}", c.id), &c.guid)?,
                member_type: core::MemberTypeId(c.member_type),
                level: core::LevelId(c.level),
                position: pt(c.position),
                rotation: c.rotation,
                base_offset: c.base_offset,
                top: top_in(c.top),
                height: c.height,
            });
        }
        for b in self.beams {
            parts.beams.push(core::Beam {
                id: core::BeamId(b.id),
                guid: guid(&format!("beam {}", b.id), &b.guid)?,
                member_type: core::MemberTypeId(b.member_type),
                level: core::LevelId(b.level),
                start: pt(b.start),
                end: pt(b.end),
                top_offset: b.top_offset,
            });
        }
        core::Model::from_parts(parts).map_err(FileError::Model)
    }
}

/// Parameter values as written in the file: materials by their id.
fn values_out(values: &core::Values) -> ParamValues {
    values
        .iter()
        .map(|(k, v)| {
            let json = match v {
                core::Value::Number(n) => serde_json::Value::from(*n),
                core::Value::Bool(b) => serde_json::Value::from(*b),
                core::Value::Text(t) => serde_json::Value::from(t.clone()),
                core::Value::Material(m) => serde_json::Value::from(*m),
            };
            (k.clone(), json)
        })
        .collect()
}

/// Parameter values read from the file, typed by the family's parameters.
/// Values for parameters the family does not have are kept as they come,
/// for the model to report.
fn values_in(
    family: Option<&core::Family>,
    scope: core::ParamScope,
    json: ParamValues,
    element: &str,
) -> Result<core::Values, FileError> {
    let mut out = core::Values::new();
    for (name, v) in json {
        let kind = family
            .and_then(|f| f.def().param(&name))
            .filter(|p| p.scope == scope)
            .map(|p| p.kind.clone());
        let bad = |what: &str| FileError::Value {
            element: element.to_string(),
            message: format!("«{name}» must be {what}"),
        };
        let value = match (&kind, &v) {
            (Some(core::ParamKind::Material), serde_json::Value::Number(n)) => {
                core::Value::Material(n.as_u64().ok_or_else(|| bad("a material id"))?)
            }
            (Some(core::ParamKind::Material), _) => return Err(bad("a material id")),
            (_, serde_json::Value::Number(n)) => {
                core::Value::Number(n.as_f64().ok_or_else(|| bad("a number"))?)
            }
            (_, serde_json::Value::Bool(b)) => core::Value::Bool(*b),
            (_, serde_json::Value::String(t)) => core::Value::Text(t.clone()),
            _ => return Err(bad("a number, true/false or a text")),
        };
        out.insert(name, value);
    }
    Ok(out)
}

fn layers_in(layers: Vec<Layer>) -> Vec<core::Layer> {
    layers
        .into_iter()
        .map(|l| core::Layer {
            material: core::MaterialId(l.material),
            thickness: l.thickness,
            function: match l.function {
                LayerFunction::Structure => core::LayerFunction::Structure,
                LayerFunction::Substrate => core::LayerFunction::Substrate,
                LayerFunction::Insulation => core::LayerFunction::Insulation,
                LayerFunction::AirGap => core::LayerFunction::AirGap,
                LayerFunction::Membrane => core::LayerFunction::Membrane,
                LayerFunction::Finish => core::LayerFunction::Finish,
            },
        })
        .collect()
}

/// Parses `#RRGGBB` (case-insensitive).
fn parse_hex(text: &str) -> Option<[u8; 3]> {
    let digits = text.strip_prefix('#')?;
    if digits.len() != 6 || !digits.is_ascii() {
        return None;
    }
    let byte = |i: usize| u8::from_str_radix(&digits[i..i + 2], 16).ok();
    Some([byte(0)?, byte(2)?, byte(4)?])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn colours() {
        assert_eq!(hex([232, 196, 178]), "#E8C4B2");
        assert_eq!(parse_hex("#e8c4b2"), Some([232, 196, 178]));
        assert_eq!(parse_hex("E8C4B2"), None);
        assert_eq!(parse_hex("#E8C4B"), None);
        assert_eq!(parse_hex("#GGC4B2"), None);
        assert_eq!(parse_hex("#ÁÁÁ"), None);
    }
}
