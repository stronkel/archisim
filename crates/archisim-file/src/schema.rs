//! Version 2 of the `.archisim` file layout and its conversion to and from
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
    pub opening_types: Vec<OpeningType>,
    pub openings: Vec<Opening>,
    /// Plan dimensions (version 2).
    pub dimensions: Vec<Dimension>,
    /// Level marks in plan (version 2).
    pub elevation_marks: Vec<ElevationMark>,
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
    pub reference: ReferenceLine,
    pub height: f64,
    pub base_offset: f64,
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
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum OpeningCategory {
    Door,
    Window,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum Operation {
    SingleSwing,
    DoubleSwing,
    Sliding,
    Fixed,
    SingleCasement,
    DoubleCasement,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct OpeningType {
    pub id: u64,
    pub guid: String,
    pub name: String,
    pub category: OpeningCategory,
    pub operation: Operation,
    pub width: f64,
    pub height: f64,
    pub frame_width: f64,
    pub frame_depth: f64,
    pub frame_material: u64,
    pub panel_material: u64,
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
                    reference: match w.reference {
                        core::ReferenceLine::Center => ReferenceLine::Center,
                        core::ReferenceLine::ExteriorFace => ReferenceLine::ExteriorFace,
                        core::ReferenceLine::InteriorFace => ReferenceLine::InteriorFace,
                    },
                    height: w.height,
                    base_offset: w.base_offset,
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
                })
                .collect(),
            opening_types: p
                .opening_types
                .into_iter()
                .map(|t| {
                    let (category, operation) = match t.kind {
                        core::OpeningKind::Door(op) => (
                            OpeningCategory::Door,
                            match op {
                                core::DoorOperation::SingleSwing => Operation::SingleSwing,
                                core::DoorOperation::DoubleSwing => Operation::DoubleSwing,
                                core::DoorOperation::Sliding => Operation::Sliding,
                            },
                        ),
                        core::OpeningKind::Window(op) => (
                            OpeningCategory::Window,
                            match op {
                                core::WindowOperation::Fixed => Operation::Fixed,
                                core::WindowOperation::SingleCasement => Operation::SingleCasement,
                                core::WindowOperation::DoubleCasement => Operation::DoubleCasement,
                                core::WindowOperation::Sliding => Operation::Sliding,
                            },
                        ),
                    };
                    OpeningType {
                        id: t.id.0,
                        guid: t.guid.to_string(),
                        name: t.name,
                        category,
                        operation,
                        width: t.width,
                        height: t.height,
                        frame_width: t.frame_width,
                        frame_depth: t.frame_depth,
                        frame_material: t.frame_material.0,
                        panel_material: t.panel_material.0,
                    }
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
            opening_types: Vec::new(),
            openings: Vec::new(),
            dimensions: Vec::new(),
            elevation_marks: Vec::new(),
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
                reference: match w.reference {
                    ReferenceLine::Center => core::ReferenceLine::Center,
                    ReferenceLine::ExteriorFace => core::ReferenceLine::ExteriorFace,
                    ReferenceLine::InteriorFace => core::ReferenceLine::InteriorFace,
                },
                height: w.height,
                base_offset: w.base_offset,
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
            });
        }
        for t in self.opening_types {
            let kind = match (t.category, t.operation) {
                (OpeningCategory::Door, Operation::SingleSwing) => {
                    core::OpeningKind::Door(core::DoorOperation::SingleSwing)
                }
                (OpeningCategory::Door, Operation::DoubleSwing) => {
                    core::OpeningKind::Door(core::DoorOperation::DoubleSwing)
                }
                (OpeningCategory::Door, Operation::Sliding) => {
                    core::OpeningKind::Door(core::DoorOperation::Sliding)
                }
                (OpeningCategory::Window, Operation::Fixed) => {
                    core::OpeningKind::Window(core::WindowOperation::Fixed)
                }
                (OpeningCategory::Window, Operation::SingleCasement) => {
                    core::OpeningKind::Window(core::WindowOperation::SingleCasement)
                }
                (OpeningCategory::Window, Operation::DoubleCasement) => {
                    core::OpeningKind::Window(core::WindowOperation::DoubleCasement)
                }
                (OpeningCategory::Window, Operation::Sliding) => {
                    core::OpeningKind::Window(core::WindowOperation::Sliding)
                }
                (category, operation) => {
                    return Err(FileError::Value {
                        element: format!("door or window type {}", t.id),
                        message: format!(
                            "a {} cannot have the operation «{}»",
                            enum_name(&category),
                            enum_name(&operation)
                        ),
                    })
                }
            };
            parts.opening_types.push(core::OpeningType {
                id: core::OpeningTypeId(t.id),
                guid: guid(&format!("door or window type {}", t.id), &t.guid)?,
                name: t.name,
                kind,
                width: t.width,
                height: t.height,
                frame_width: t.frame_width,
                frame_depth: t.frame_depth,
                frame_material: core::MaterialId(t.frame_material),
                panel_material: core::MaterialId(t.panel_material),
            });
        }
        for o in self.openings {
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
        core::Model::from_parts(parts).map_err(FileError::Model)
    }
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

/// The name an enum value has in the file, e.g. `single_swing`.
fn enum_name<T: Serialize>(value: &T) -> String {
    serde_json::to_value(value)
        .ok()
        .and_then(|v| v.as_str().map(str::to_string))
        .unwrap_or_default()
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
