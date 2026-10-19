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
    /// Rooms (version 7).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub spaces: Vec<Space>,
}

/// A room (version 7).
#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Space {
    pub id: u64,
    pub guid: String,
    pub level: u64,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub number: String,
    pub name: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub usage: String,
    pub outline: Vec<Xy>,
    pub height: f64,
    /// Where its label is drawn; absent: in the middle.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<Xy>,
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
#[derive(Debug, Serialize, Deserialize)]
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
            spaces: p
                .spaces
                .into_iter()
                .map(|s| Space {
                    id: s.id.0,
                    guid: s.guid.to_string(),
                    level: s.level.0,
                    number: s.number,
                    name: s.name,
                    usage: s.usage,
                    outline: s.outline.into_iter().map(xy).collect(),
                    height: s.height,
                    label: s.label.map(xy),
                })
                .collect(),
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
            spaces: Vec::new(),
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
            parts.wall_types.push(core::WallType {
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

        for d in self.dimensions {
            parts.dimensions.push(core::Dimension {
                id: core::DimensionId(d.id),
                level: core::LevelId(d.level),
                points: d.points.into_iter().map(pt).collect(),
                direction: pt(d.direction),
                position: pt(d.position),
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
        for s in self.spaces {
            parts.spaces.push(core::Space {
                id: core::SpaceId(s.id),
                guid: guid(&format!("space {}", s.id), &s.guid)?,
                level: core::LevelId(s.level),
                number: s.number,
                name: s.name,
                usage: s.usage,
                outline: s.outline.into_iter().map(pt).collect(),
                height: s.height,
                label: s.label.map(pt),
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
