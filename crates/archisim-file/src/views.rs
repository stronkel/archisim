//! Saved plans, the looks of the levels' own plans, view templates and
//! reference planes (version 27).

use archisim_core as core;
use serde::{Deserialize, Serialize};

use crate::drafting::Style;
use crate::FileError;

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct PlanViewOf {
    pub id: u64,
    pub name: String,
    pub level: u64,
    /// `floor` or `ceiling`.
    #[serde(default = "floor", skip_serializing_if = "is_floor")]
    pub kind: String,
    pub scale: f64,
    pub cut_plane: f64,
    #[serde(default, skip_serializing_if = "Style::is_default")]
    pub style: Style,
}

fn floor() -> String {
    "floor".into()
}

fn is_floor(k: &String) -> bool {
    k == "floor"
}

impl PlanViewOf {
    pub fn out(v: &core::PlanView) -> Self {
        Self {
            id: v.id.0,
            name: v.name.clone(),
            level: v.level.0,
            kind: v.kind.code().into(),
            scale: v.scale,
            cut_plane: v.cut_plane,
            style: Style::out(&v.style),
        }
    }

    pub fn into_core(self) -> Result<core::PlanView, FileError> {
        let element = format!("plan view {}", self.id);
        Ok(core::PlanView {
            id: core::PlanViewId(self.id),
            kind: core::PlanKind::from_code(&self.kind).ok_or_else(|| FileError::Value {
                element: element.clone(),
                message: format!("unknown kind of plan «{}»", self.kind),
            })?,
            style: self.style.into_core(&element)?,
            name: self.name,
            level: core::LevelId(self.level),
            scale: self.scale,
            cut_plane: self.cut_plane,
        })
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct LevelStyleOf {
    pub level: u64,
    pub style: Style,
}

impl LevelStyleOf {
    pub fn out(level: core::LevelId, s: &core::ViewStyle) -> Self {
        Self {
            level: level.0,
            style: Style::out(s),
        }
    }

    pub fn into_core(self) -> Result<(core::LevelId, core::ViewStyle), FileError> {
        let element = format!("style of level {}", self.level);
        Ok((core::LevelId(self.level), self.style.into_core(&element)?))
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ViewTemplateOf {
    pub id: u64,
    pub name: String,
    #[serde(default, skip_serializing_if = "Style::is_default")]
    pub style: Style,
}

impl ViewTemplateOf {
    pub fn out(t: &core::ViewTemplate) -> Self {
        Self {
            id: t.id.0,
            name: t.name.clone(),
            style: Style::out(&t.style),
        }
    }

    pub fn into_core(self) -> Result<core::ViewTemplate, FileError> {
        let element = format!("view template {}", self.id);
        Ok(core::ViewTemplate {
            id: core::ViewTemplateId(self.id),
            style: self.style.into_core(&element)?,
            name: self.name,
        })
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct RefPlaneOf {
    pub id: u64,
    pub name: String,
    pub start: [f64; 2],
    pub end: [f64; 2],
}

impl RefPlaneOf {
    pub fn out(r: &core::RefPlane) -> Self {
        Self {
            id: r.id.0,
            name: r.name.clone(),
            start: [r.start.x, r.start.y],
            end: [r.end.x, r.end.y],
        }
    }

    pub fn into_core(self) -> core::RefPlane {
        core::RefPlane {
            id: core::RefPlaneId(self.id),
            name: self.name,
            start: core::Point2::new(self.start[0], self.start[1]),
            end: core::Point2::new(self.end[0], self.end[1]),
        }
    }
}
