//! Rooms and tags in the project file (version 7).

use archisim_core::{self as core, GlobalId, Point2};
use serde::{Deserialize, Serialize};

type Xy = [f64; 2];
use crate::FileError;

fn xy(p: Point2) -> Xy {
    [p.x, p.y]
}

fn pt(p: Xy) -> Point2 {
    Point2::new(p[0], p[1])
}

fn is_empty(s: &str) -> bool {
    s.is_empty()
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Room {
    pub id: u64,
    pub guid: String,
    pub level: u64,
    pub number: String,
    pub name: String,
    #[serde(default, skip_serializing_if = "is_empty")]
    pub usage: String,
    pub outline: Vec<Xy>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub holes: Vec<Vec<Xy>>,
    pub label: Xy,
    #[serde(default)]
    pub bounded: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub height: Option<f64>,
    #[serde(default, skip_serializing_if = "is_empty")]
    pub floor: String,
    #[serde(default, skip_serializing_if = "is_empty")]
    pub walls: String,
    #[serde(default, skip_serializing_if = "is_empty")]
    pub ceiling: String,
    /// m² per occupant (version 12); absent: from its use.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub occupancy: Option<f64>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(
    deny_unknown_fields,
    rename_all = "snake_case",
    tag = "kind",
    content = "id"
)]
pub(crate) enum Target {
    Opening(u64),
    Wall(u64),
    Room(u64),
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Tag {
    pub id: u64,
    pub level: u64,
    pub target: Target,
    pub at: Xy,
    #[serde(default)]
    pub leader: bool,
}

pub(crate) fn rooms_out(rooms: &[core::Room]) -> Vec<Room> {
    rooms
        .iter()
        .map(|r| Room {
            id: r.id.0,
            guid: r.guid.to_string(),
            level: r.level.0,
            number: r.number.clone(),
            name: r.name.clone(),
            usage: r.usage.clone(),
            outline: r.outline.iter().copied().map(xy).collect(),
            holes: r
                .holes
                .iter()
                .map(|h| h.iter().copied().map(xy).collect())
                .collect(),
            label: xy(r.label),
            bounded: r.bounded,
            height: r.height,
            floor: r.finishes.floor.clone(),
            walls: r.finishes.walls.clone(),
            ceiling: r.finishes.ceiling.clone(),
            occupancy: r.occupancy,
        })
        .collect()
}

pub(crate) fn rooms_in(rooms: Vec<Room>) -> Result<Vec<core::Room>, FileError> {
    rooms
        .into_iter()
        .map(|r| {
            let guid = GlobalId::parse(&r.guid).ok_or_else(|| FileError::Value {
                element: format!("room {}", r.id),
                message: format!("«{}» is not a 22-character IFC GlobalId", r.guid),
            })?;
            Ok(core::Room {
                id: core::RoomId(r.id),
                guid,
                level: core::LevelId(r.level),
                number: r.number,
                name: r.name,
                usage: r.usage,
                outline: r.outline.into_iter().map(pt).collect(),
                holes: r
                    .holes
                    .into_iter()
                    .map(|h| h.into_iter().map(pt).collect())
                    .collect(),
                label: pt(r.label),
                bounded: r.bounded,
                height: r.height,
                finishes: core::Finishes {
                    floor: r.floor,
                    walls: r.walls,
                    ceiling: r.ceiling,
                },
                occupancy: r.occupancy,
            })
        })
        .collect()
}

pub(crate) fn tags_out(tags: &[core::Tag]) -> Vec<Tag> {
    tags.iter()
        .map(|t| Tag {
            id: t.id.0,
            level: t.level.0,
            target: match t.target {
                core::TagTarget::Opening(o) => Target::Opening(o.0),
                core::TagTarget::Wall(w) => Target::Wall(w.0),
                core::TagTarget::Room(r) => Target::Room(r.0),
            },
            at: xy(t.at),
            leader: t.leader,
        })
        .collect()
}

pub(crate) fn tags_in(tags: Vec<Tag>) -> Vec<core::Tag> {
    tags.into_iter()
        .map(|t| core::Tag {
            id: core::TagId(t.id),
            level: core::LevelId(t.level),
            target: match t.target {
                Target::Opening(o) => core::TagTarget::Opening(core::OpeningId(o)),
                Target::Wall(w) => core::TagTarget::Wall(core::WallId(w)),
                Target::Room(r) => core::TagTarget::Room(core::RoomId(r)),
            },
            at: pt(t.at),
            leader: t.leader,
        })
        .collect()
}
