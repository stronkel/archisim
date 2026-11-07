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
    /// The scale it sets for the views that follow it (version 92).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scale: Option<f64>,
    /// The cut plane it sets for the plans that follow it (version 92).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cut_plane: Option<f64>,
}

impl ViewTemplateOf {
    pub fn out(t: &core::ViewTemplate) -> Self {
        Self {
            id: t.id.0,
            name: t.name.clone(),
            style: Style::out(&t.style),
            scale: t.scale,
            cut_plane: t.cut_plane,
        }
    }

    pub fn into_core(self) -> Result<core::ViewTemplate, FileError> {
        let element = format!("view template {}", self.id);
        Ok(core::ViewTemplate {
            id: core::ViewTemplateId(self.id),
            style: self.style.into_core(&element)?,
            name: self.name,
            scale: self.scale,
            cut_plane: self.cut_plane,
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

/// A saved camera of the 3D (version 72).
#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct CameraOf {
    pub id: u64,
    pub name: String,
    pub target: [f64; 3],
    pub pitch: f64,
    pub yaw: f64,
    pub distance: f64,
    #[serde(default = "yes", skip_serializing_if = "is_true")]
    pub perspective: bool,
}

fn yes() -> bool {
    true
}

fn is_true(v: &bool) -> bool {
    *v
}

impl CameraOf {
    pub fn out(c: &core::SavedCamera) -> Self {
        Self {
            id: c.id.0,
            name: c.name.clone(),
            target: c.target,
            pitch: c.pitch,
            yaw: c.yaw,
            distance: c.distance,
            perspective: c.perspective,
        }
    }

    pub fn into_core(self) -> core::SavedCamera {
        core::SavedCamera {
            id: core::CameraId(self.id),
            name: self.name,
            target: self.target,
            pitch: self.pitch,
            yaw: self.yaw,
            distance: self.distance,
            perspective: self.perspective,
        }
    }
}

/// A title block of the project (version 74).
#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct TitleBlockOf {
    pub id: u64,
    pub name: String,
    pub design: String,
    pub accent: [u8; 3],
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub studio: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub monogram: String,
    /// The logo, a PNG in base64, and the file it came from.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub logo: Option<String>,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub logo_source: String,
    /// The rows: label and what it shows.
    pub rows: Vec<(String, String)>,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub stamp: String,
    pub width: f64,
}

impl TitleBlockOf {
    pub fn out(t: &core::TitleBlock) -> Self {
        Self {
            id: t.id.0,
            name: t.name.clone(),
            design: t.design.code().to_string(),
            accent: t.accent,
            studio: t.studio.clone(),
            monogram: t.monogram.clone(),
            logo: t.logo.as_ref().map(|l| core::base64_encode(&l.png)),
            logo_source: t
                .logo
                .as_ref()
                .map(|l| l.source.clone())
                .unwrap_or_default(),
            rows: t
                .rows
                .iter()
                .map(|r| (r.label.clone(), r.value.code()))
                .collect(),
            stamp: t.stamp.clone(),
            width: t.width,
        }
    }

    pub fn into_core(self) -> Result<core::TitleBlock, FileError> {
        let element = format!("title block {}", self.id);
        let bad = |m: String| FileError::Value {
            element: element.clone(),
            message: m,
        };
        let logo = match self.logo {
            None => None,
            Some(b) => {
                let png =
                    core::base64_decode(&b).ok_or_else(|| bad("logo is not base64".into()))?;
                Some(
                    core::decal_image(&self.logo_source, png)
                        .ok_or_else(|| bad("logo is not a PNG".into()))?,
                )
            }
        };
        Ok(core::TitleBlock {
            id: core::TitleBlockId(self.id),
            name: self.name,
            design: core::TitleBlockDesign::from_code(&self.design)
                .ok_or_else(|| bad(format!("unknown design «{}»", self.design)))?,
            accent: self.accent,
            studio: self.studio,
            monogram: self.monogram,
            logo,
            rows: self
                .rows
                .into_iter()
                .map(|(label, v)| {
                    core::TitleValue::from_code(&v)
                        .map(|value| core::TitleRow { label, value })
                        .ok_or_else(|| bad(format!("unknown row «{v}»")))
                })
                .collect::<Result<_, _>>()?,
            stamp: self.stamp,
            width: self.width,
        })
    }
}
