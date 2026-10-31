//! Drawing layers and drafting elements in the project file (version 7).

use archisim_core as core;
use archisim_core::{LineStyle, Point2};
use serde::{Deserialize, Serialize};

use crate::FileError;

type Xy = [f64; 2];

fn xy(p: Point2) -> Xy {
    [p.x, p.y]
}

fn pt(p: Xy) -> Point2 {
    Point2::new(p[0], p[1])
}

fn is_zero(v: &f64) -> bool {
    *v == 0.0
}

fn is_false(v: &bool) -> bool {
    !*v
}

fn is_true(v: &bool) -> bool {
    *v
}

fn yes() -> bool {
    true
}

pub(crate) fn hex(c: [u8; 3]) -> String {
    format!("#{:02x}{:02x}{:02x}", c[0], c[1], c[2])
}

pub(crate) fn parse_hex(element: &str, text: &str) -> Result<[u8; 3], FileError> {
    let bad = || FileError::Value {
        element: element.to_string(),
        message: format!("«{text}» is not a colour like #rrggbb"),
    };
    let t = text.strip_prefix('#').ok_or_else(bad)?;
    if t.len() != 6 || !t.is_ascii() {
        return Err(bad());
    }
    let c = |i: usize| u8::from_str_radix(&t[i..i + 2], 16).map_err(|_| bad());
    Ok([c(0)?, c(2)?, c(4)?])
}

fn line_style(element: &str, code: &str) -> Result<LineStyle, FileError> {
    LineStyle::from_code(code).ok_or_else(|| FileError::Value {
        element: element.to_string(),
        message: format!("unknown line style «{code}»"),
    })
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct DrawLayer {
    pub id: u64,
    pub name: String,
    /// `#rrggbb`.
    pub color: String,
    /// Paper millimetres.
    pub weight: f64,
    pub line: String,
    #[serde(default = "yes", skip_serializing_if = "is_true")]
    pub visible: bool,
    #[serde(default, skip_serializing_if = "is_false")]
    pub locked: bool,
    #[serde(default = "yes", skip_serializing_if = "is_true")]
    pub printable: bool,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub(crate) enum View {
    Level(u64),
    Sheet(u64),
    /// A view only for drawing (version 48).
    Drafting(u64),
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Polyline {
    pub points: Vec<Xy>,
    /// One per point; left out when every segment is straight.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub bulges: Vec<f64>,
    #[serde(default, skip_serializing_if = "is_false")]
    pub closed: bool,
    #[serde(default, skip_serializing_if = "is_zero")]
    pub width: f64,
}

impl Polyline {
    fn out(p: &core::Polyline) -> Self {
        Self {
            points: p.points.iter().copied().map(xy).collect(),
            bulges: if p.is_straight() {
                Vec::new()
            } else {
                p.bulges.clone()
            },
            closed: p.closed,
            width: p.width,
        }
    }

    fn into_core(self) -> core::Polyline {
        let n = self.points.len();
        core::Polyline {
            points: self.points.into_iter().map(pt).collect(),
            bulges: if self.bulges.is_empty() {
                vec![0.0; n]
            } else {
                self.bulges
            },
            closed: self.closed,
            width: self.width,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields, tag = "kind", rename_all = "snake_case")]
pub(crate) enum Shape {
    Line {
        a: Xy,
        b: Xy,
    },
    Polyline(Polyline),
    Circle {
        centre: Xy,
        radius: f64,
    },
    Arc {
        centre: Xy,
        radius: f64,
        start: f64,
        sweep: f64,
    },
    Ellipse {
        centre: Xy,
        axis: Xy,
        ratio: f64,
        start: f64,
        sweep: f64,
    },
    Point {
        at: Xy,
    },
    Text {
        at: Xy,
        content: String,
        height: f64,
        #[serde(default, skip_serializing_if = "is_zero")]
        angle: f64,
        align: String,
    },
    Hatch {
        loops: Vec<Polyline>,
        pattern: String,
        scale: f64,
        #[serde(default, skip_serializing_if = "is_zero")]
        angle: f64,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        background: Option<String>,
        /// Version 23; absent: solid.
        #[serde(default = "one", skip_serializing_if = "is_one")]
        opacity: f64,
    },
    AngleDim {
        centre: Xy,
        start: f64,
        sweep: f64,
        radius: f64,
    },
    RadiusDim {
        centre: Xy,
        radius: f64,
        angle: f64,
        reach: f64,
    },
    DiameterDim {
        centre: Xy,
        radius: f64,
        angle: f64,
        reach: f64,
    },
    ArcLengthDim {
        centre: Xy,
        radius: f64,
        start: f64,
        sweep: f64,
        offset: f64,
    },
}

impl Shape {
    pub fn out(s: &core::Shape) -> Self {
        match s {
            core::Shape::Line { a, b } => Shape::Line {
                a: xy(*a),
                b: xy(*b),
            },
            core::Shape::Polyline(p) => Shape::Polyline(Polyline::out(p)),
            core::Shape::Circle { centre, radius } => Shape::Circle {
                centre: xy(*centre),
                radius: *radius,
            },
            core::Shape::Arc {
                centre,
                radius,
                start,
                sweep,
            } => Shape::Arc {
                centre: xy(*centre),
                radius: *radius,
                start: *start,
                sweep: *sweep,
            },
            core::Shape::Ellipse {
                centre,
                axis,
                ratio,
                start,
                sweep,
            } => Shape::Ellipse {
                centre: xy(*centre),
                axis: xy(*axis),
                ratio: *ratio,
                start: *start,
                sweep: *sweep,
            },
            core::Shape::Point { at } => Shape::Point { at: xy(*at) },
            core::Shape::Text(t) => Shape::Text {
                at: xy(t.at),
                content: t.content.clone(),
                height: t.height,
                angle: t.angle,
                align: t.align.code().into(),
            },
            core::Shape::Hatch(h) => Shape::Hatch {
                loops: h.loops.iter().map(Polyline::out).collect(),
                pattern: h.pattern.code().into(),
                scale: h.scale,
                angle: h.angle,
                background: h.background.map(hex),
                opacity: h.opacity,
            },
            core::Shape::Dim(m) => match *m {
                core::Measure::Angle {
                    centre,
                    start,
                    sweep,
                    radius,
                } => Shape::AngleDim {
                    centre: xy(centre),
                    start,
                    sweep,
                    radius,
                },
                core::Measure::Radius {
                    centre,
                    radius,
                    angle,
                    reach,
                } => Shape::RadiusDim {
                    centre: xy(centre),
                    radius,
                    angle,
                    reach,
                },
                core::Measure::Diameter {
                    centre,
                    radius,
                    angle,
                    reach,
                } => Shape::DiameterDim {
                    centre: xy(centre),
                    radius,
                    angle,
                    reach,
                },
                core::Measure::ArcLength {
                    centre,
                    radius,
                    start,
                    sweep,
                    offset,
                } => Shape::ArcLengthDim {
                    centre: xy(centre),
                    radius,
                    start,
                    sweep,
                    offset,
                },
            },
        }
    }

    pub fn into_core(self, element: &str) -> Result<core::Shape, FileError> {
        let value = |message: String| FileError::Value {
            element: element.to_string(),
            message,
        };
        Ok(match self {
            Shape::Line { a, b } => core::Shape::Line { a: pt(a), b: pt(b) },
            Shape::Polyline(p) => core::Shape::Polyline(p.into_core()),
            Shape::Circle { centre, radius } => core::Shape::Circle {
                centre: pt(centre),
                radius,
            },
            Shape::Arc {
                centre,
                radius,
                start,
                sweep,
            } => core::Shape::Arc {
                centre: pt(centre),
                radius,
                start,
                sweep,
            },
            Shape::Ellipse {
                centre,
                axis,
                ratio,
                start,
                sweep,
            } => core::Shape::Ellipse {
                centre: pt(centre),
                axis: pt(axis),
                ratio,
                start,
                sweep,
            },
            Shape::Point { at } => core::Shape::Point { at: pt(at) },
            Shape::Text {
                at,
                content,
                height,
                angle,
                align,
            } => core::Shape::Text(core::Text {
                at: pt(at),
                content,
                height,
                angle,
                align: core::TextAlign::from_code(&align)
                    .ok_or_else(|| value(format!("unknown text alignment «{align}»")))?,
            }),
            Shape::Hatch {
                loops,
                pattern,
                scale,
                angle,
                background,
                opacity,
            } => core::Shape::Hatch(core::Hatch {
                loops: loops.into_iter().map(Polyline::into_core).collect(),
                pattern: core::HatchPattern::from_code(&pattern)
                    .ok_or_else(|| value(format!("unknown hatch pattern «{pattern}»")))?,
                scale,
                angle,
                background: background.map(|b| parse_hex(element, &b)).transpose()?,
                opacity: opacity.clamp(0.05, 1.0),
            }),
            Shape::AngleDim {
                centre,
                start,
                sweep,
                radius,
            } => core::Shape::Dim(core::Measure::Angle {
                centre: pt(centre),
                start,
                sweep,
                radius,
            }),
            Shape::RadiusDim {
                centre,
                radius,
                angle,
                reach,
            } => core::Shape::Dim(core::Measure::Radius {
                centre: pt(centre),
                radius,
                angle,
                reach,
            }),
            Shape::DiameterDim {
                centre,
                radius,
                angle,
                reach,
            } => core::Shape::Dim(core::Measure::Diameter {
                centre: pt(centre),
                radius,
                angle,
                reach,
            }),
            Shape::ArcLengthDim {
                centre,
                radius,
                start,
                sweep,
                offset,
            } => core::Shape::Dim(core::Measure::ArcLength {
                centre: pt(centre),
                radius,
                start,
                sweep,
                offset,
            }),
        })
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Draft {
    pub id: u64,
    pub view: View,
    pub layer: u64,
    pub shape: Shape,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub weight: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub line: Option<String>,
    /// Its group (version 53): number and name.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub group: Option<DraftGroup>,
}

/// A group of drafting elements (version 53).
#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct DraftGroup {
    pub id: u64,
    pub name: String,
}

pub(crate) fn layers_out(layers: &[core::DrawLayer]) -> Vec<DrawLayer> {
    layers
        .iter()
        .map(|l| DrawLayer {
            id: l.id.0,
            name: l.name.clone(),
            color: hex(l.color),
            weight: l.weight,
            line: l.line.code().into(),
            visible: l.visible,
            locked: l.locked,
            printable: l.printable,
        })
        .collect()
}

pub(crate) fn layers_in(layers: Vec<DrawLayer>) -> Result<Vec<core::DrawLayer>, FileError> {
    layers
        .into_iter()
        .map(|l| {
            let element = format!("drawing layer {} «{}»", l.id, l.name);
            Ok(core::DrawLayer {
                id: core::DrawLayerId(l.id),
                color: parse_hex(&element, &l.color)?,
                line: line_style(&element, &l.line)?,
                name: l.name,
                weight: l.weight,
                visible: l.visible,
                locked: l.locked,
                printable: l.printable,
            })
        })
        .collect()
}

pub(crate) fn drafts_out(drafts: &[core::Draft]) -> Vec<Draft> {
    drafts
        .iter()
        .map(|d| Draft {
            id: d.id.0,
            view: match d.view {
                core::ViewRef::Level(l) => View::Level(l.0),
                core::ViewRef::Sheet(s) => View::Sheet(s.0),
                core::ViewRef::Drafting(v) => View::Drafting(v.0),
            },
            layer: d.layer.0,
            shape: Shape::out(&d.shape),
            color: d.color.map(hex),
            weight: d.weight,
            line: d.line.map(|l| l.code().to_string()),
            group: d.group.as_ref().map(|g| DraftGroup {
                id: g.id,
                name: g.name.clone(),
            }),
        })
        .collect()
}

pub(crate) fn drafts_in(drafts: Vec<Draft>) -> Result<Vec<core::Draft>, FileError> {
    drafts
        .into_iter()
        .map(|d| {
            let element = format!("drafting element {}", d.id);
            Ok(core::Draft {
                id: core::DraftId(d.id),
                view: match d.view {
                    View::Level(l) => core::ViewRef::Level(core::LevelId(l)),
                    View::Sheet(s) => core::ViewRef::Sheet(core::SheetId(s)),
                    View::Drafting(v) => core::ViewRef::Drafting(core::DraftViewId(v)),
                },
                layer: core::DrawLayerId(d.layer),
                shape: d.shape.into_core(&element)?,
                color: d.color.map(|c| parse_hex(&element, &c)).transpose()?,
                weight: d.weight,
                line: d.line.map(|l| line_style(&element, &l)).transpose()?,
                group: d.group.map(|g| core::DraftGroup {
                    id: g.id,
                    name: g.name,
                }),
            })
        })
        .collect()
}

/// Line weights of a scale range, paper millimetres.
#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Pens {
    pub cut: f64,
    pub frame: f64,
    pub view: f64,
    pub thin: f64,
    pub hatch: f64,
    pub dimension: f64,
    pub tick: f64,
    pub text: f64,
}

impl Pens {
    fn out(p: &core::Pens) -> Self {
        Self {
            cut: p.cut,
            frame: p.frame,
            view: p.view,
            thin: p.thin,
            hatch: p.hatch,
            dimension: p.dimension,
            tick: p.tick,
            text: p.text,
        }
    }

    fn into_core(self) -> core::Pens {
        core::Pens {
            cut: self.cut,
            frame: self.frame,
            view: self.view,
            thin: self.thin,
            hatch: self.hatch,
            dimension: self.dimension,
            tick: self.tick,
            text: self.text,
        }
    }
}

/// The project's graphic standard (version 7).
#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Graphics {
    pub fine: Pens,
    pub coarse: Pens,
    /// Scales from 1:`coarse_from` down use the coarse pens.
    pub coarse_from: f64,
    /// Lettering of texts and dimensions (version 27; sans if missing).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub font: Option<String>,
}

impl Graphics {
    pub fn out(g: &core::GraphicStandard) -> Self {
        Self {
            fine: Pens::out(&g.fine),
            coarse: Pens::out(&g.coarse),
            coarse_from: g.coarse_from,
            font: (g.font != core::TextFont::default()).then(|| g.font.code().to_string()),
        }
    }

    pub fn into_core(self) -> Result<core::GraphicStandard, FileError> {
        Ok(core::GraphicStandard {
            fine: self.fine.into_core(),
            coarse: self.coarse.into_core(),
            coarse_from: self.coarse_from,
            font: match &self.font {
                None => core::TextFont::default(),
                Some(f) => core::TextFont::from_code(f).ok_or_else(|| FileError::Value {
                    element: "graphics".into(),
                    message: format!("unknown font «{f}»"),
                })?,
            },
        })
    }
}

// ---- Sheets (version 7) --------------------------------------------------

#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Info {
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub client: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub site: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub author: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub studio: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub number: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub phase: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub date: String,
}

impl Info {
    pub fn out(i: &core::ProjectInfo) -> Option<Self> {
        (i != &core::ProjectInfo::default()).then(|| Self {
            client: i.client.clone(),
            site: i.site.clone(),
            author: i.author.clone(),
            studio: i.studio.clone(),
            number: i.number.clone(),
            phase: i.phase.clone(),
            date: i.date.clone(),
        })
    }

    pub fn into_core(self) -> core::ProjectInfo {
        core::ProjectInfo {
            client: self.client,
            site: self.site,
            author: self.author,
            studio: self.studio,
            number: self.number,
            phase: self.phase,
            date: self.date,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Sheet {
    pub id: u64,
    pub number: String,
    #[serde(default)]
    pub name: String,
    /// Millimetres.
    pub width: f64,
    pub height: f64,
    #[serde(default = "yes", skip_serializing_if = "is_true")]
    pub title_block: bool,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub revision: String,
    /// Presentation style of the sheet itself (version 22).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub presentation: Option<u64>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Style {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub hidden: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub hidden_layers: Vec<u64>,
    #[serde(default = "fine", skip_serializing_if = "is_fine")]
    pub detail: String,
    /// What is drawn in grey by discipline (version 15).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub focus: Option<String>,
    /// Disciplines left out (version 15).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub hidden_disciplines: Vec<String>,
    /// Presentation style (version 22).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub presentation: Option<u64>,
    /// How categories are drawn (version 27).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub overrides: Vec<OverrideOf>,
}

/// A category drawn otherwise in a view (version 27): only what changes.
#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct OverrideOf {
    pub category: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub line: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cut_fill: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub surface_fill: Option<String>,
    #[serde(default, skip_serializing_if = "is_zero_u8")]
    pub transparency: u8,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub halftone: bool,
    #[serde(default = "yes", skip_serializing_if = "is_yes")]
    pub patterns: bool,
    #[serde(default = "hundred", skip_serializing_if = "is_hundred")]
    pub weight: u16,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub layer_lines: Option<String>,
}

fn is_zero_u8(v: &u8) -> bool {
    *v == 0
}
fn is_yes(v: &bool) -> bool {
    *v
}
fn hundred() -> u16 {
    100
}
fn is_hundred(v: &u16) -> bool {
    *v == 100
}

impl OverrideOf {
    fn out(c: core::Category, g: &core::GraphicOverride) -> Self {
        Self {
            category: c.code().into(),
            line: g.line.map(hex),
            cut_fill: g.cut_fill.map(hex),
            surface_fill: g.surface_fill.map(hex),
            transparency: g.transparency,
            halftone: g.halftone,
            patterns: g.patterns,
            weight: g.weight,
            layer_lines: (g.layer_lines != core::LayerLines::All)
                .then(|| g.layer_lines.code().to_string()),
        }
    }

    fn into_core(
        self,
        element: &str,
    ) -> Result<(core::Category, core::GraphicOverride), FileError> {
        let bad = |m: String| FileError::Value {
            element: element.to_string(),
            message: m,
        };
        let category = core::Category::from_code(&self.category)
            .ok_or_else(|| bad(format!("unknown category «{}»", self.category)))?;
        let colour = |c: &Option<String>| c.as_deref().map(|t| parse_hex(element, t)).transpose();
        Ok((
            category,
            core::GraphicOverride {
                line: colour(&self.line)?,
                cut_fill: colour(&self.cut_fill)?,
                surface_fill: colour(&self.surface_fill)?,
                transparency: self.transparency.min(100),
                halftone: self.halftone,
                patterns: self.patterns,
                weight: self.weight.clamp(10, 1000),
                layer_lines: match &self.layer_lines {
                    None => core::LayerLines::All,
                    Some(l) => core::LayerLines::from_code(l)
                        .ok_or_else(|| bad(format!("unknown layer lines «{l}»")))?,
                },
            },
        ))
    }
}

impl Default for Style {
    fn default() -> Self {
        Self {
            hidden: Vec::new(),
            hidden_layers: Vec::new(),
            detail: fine(),
            focus: None,
            hidden_disciplines: Vec::new(),
            presentation: None,
            overrides: Vec::new(),
        }
    }
}

fn fine() -> String {
    "fine".into()
}

fn is_fine(d: &String) -> bool {
    d == "fine"
}

impl Style {
    pub(crate) fn out(s: &core::ViewStyle) -> Self {
        Self {
            hidden: s.hidden.iter().map(|c| c.code().to_string()).collect(),
            hidden_layers: s.hidden_layers.iter().map(|l| l.0).collect(),
            detail: s.detail.code().into(),
            focus: (s.focus != core::DisciplineFocus::None).then(|| s.focus.code().to_string()),
            hidden_disciplines: s
                .hidden_disciplines
                .iter()
                .map(|d| d.code().to_string())
                .collect(),
            presentation: s.presentation.map(|p| p.0),
            overrides: s
                .overrides
                .iter()
                .map(|(c, g)| OverrideOf::out(*c, g))
                .collect(),
        }
    }

    pub(crate) fn into_core(self, element: &str) -> Result<core::ViewStyle, FileError> {
        let bad = |m: String| FileError::Value {
            element: element.to_string(),
            message: m,
        };
        Ok(core::ViewStyle {
            hidden: self
                .hidden
                .iter()
                .map(|c| {
                    core::Category::from_code(c)
                        .ok_or_else(|| bad(format!("unknown category «{c}»")))
                })
                .collect::<Result<_, _>>()?,
            hidden_layers: self
                .hidden_layers
                .into_iter()
                .map(core::DrawLayerId)
                .collect(),
            detail: core::Detail::from_code(&self.detail)
                .ok_or_else(|| bad(format!("unknown detail «{}»", self.detail)))?,
            focus: match &self.focus {
                None => core::DisciplineFocus::None,
                Some(c) => core::DisciplineFocus::from_code(c)
                    .ok_or_else(|| bad(format!("unknown focus «{c}»")))?,
            },
            hidden_disciplines: self
                .hidden_disciplines
                .iter()
                .map(|c| {
                    core::ElementDiscipline::from_code(c)
                        .ok_or_else(|| bad(format!("unknown discipline «{c}»")))
                })
                .collect::<Result<_, _>>()?,
            presentation: self.presentation.map(core::PresentationStyleId),
            overrides: self
                .overrides
                .into_iter()
                .map(|o| o.into_core(element))
                .collect::<Result<_, _>>()?,
        })
    }

    pub(crate) fn is_default(&self) -> bool {
        self.hidden.is_empty()
            && self.hidden_layers.is_empty()
            && self.detail == "fine"
            && self.focus.is_none()
            && self.hidden_disciplines.is_empty()
            && self.presentation.is_none()
            && self.overrides.is_empty()
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Viewport {
    pub id: u64,
    pub sheet: u64,
    pub level: u64,
    /// A section or elevation instead of the level's plan (version 10).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub section: Option<u64>,
    /// An axonometric 3D view: where it looks from, yaw and pitch in
    /// radians (version 14).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub axo: Option<[f64; 2]>,
    pub scale: f64,
    pub cut_plane: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub crop: Option<[Xy; 2]>,
    /// Paper metres from the lower-left corner of the sheet.
    pub centre: Xy,
    #[serde(default, skip_serializing_if = "Style::is_default")]
    pub style: Style,
    /// The reflected ceiling plan of the level (version 27).
    #[serde(default, skip_serializing_if = "is_false")]
    pub ceiling: bool,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub title: String,
    #[serde(default = "yes", skip_serializing_if = "is_true")]
    pub show_title: bool,
    /// A view only for drawing instead of a plan (version 48).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub drafting: Option<u64>,
}

/// A view only for drawing (version 48): its name, scale and underlays.
#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct DraftView {
    pub id: u64,
    pub name: String,
    pub scale: f64,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub underlays: Vec<Underlay>,
}

/// An underlay (version 48): what it shows (`level`, `section` or
/// `drafting` and its id), its colour (`#rrggbb`, or none: its own),
/// opacity, offset `[x, y]` and rotation (radians).
#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Underlay {
    pub kind: String,
    pub of: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,
    pub opacity: f64,
    #[serde(default, skip_serializing_if = "is_origin")]
    pub offset: Xy,
    #[serde(default, skip_serializing_if = "is_zero")]
    pub rotation: f64,
    /// Only this rectangle of the source (version 52): `[[x, y], [x, y]]`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub crop: Option<[Xy; 2]>,
}

/// The underlays of a level's plan (version 48).
#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct LevelUnderlays {
    pub level: u64,
    pub underlays: Vec<Underlay>,
}

fn is_origin(p: &Xy) -> bool {
    p[0] == 0.0 && p[1] == 0.0
}

impl Underlay {
    fn out(u: &core::Underlay) -> Self {
        let (kind, of) = match u.source {
            core::UnderlaySource::Level(l) => ("level", l.0),
            core::UnderlaySource::Section(s) => ("section", s.0),
            core::UnderlaySource::Drafting(d) => ("drafting", d.0),
        };
        Underlay {
            kind: kind.into(),
            of,
            color: u.color.map(hex),
            opacity: u.opacity,
            offset: xy(u.offset),
            rotation: u.rotation,
            crop: u.crop.map(|[a, b]| [xy(a), xy(b)]),
        }
    }

    fn into_core(self, element: &str) -> Result<core::Underlay, FileError> {
        let source = match self.kind.as_str() {
            "level" => core::UnderlaySource::Level(core::LevelId(self.of)),
            "section" => core::UnderlaySource::Section(core::SectionId(self.of)),
            "drafting" => core::UnderlaySource::Drafting(core::DraftViewId(self.of)),
            other => {
                return Err(FileError::Value {
                    element: element.to_string(),
                    message: format!("unknown underlay kind «{other}»"),
                })
            }
        };
        Ok(core::Underlay {
            source,
            color: self.color.map(|c| parse_hex(element, &c)).transpose()?,
            opacity: self.opacity,
            offset: pt(self.offset),
            rotation: self.rotation,
            crop: self.crop.map(|[a, b]| [pt(a), pt(b)]),
        })
    }
}

pub(crate) fn draft_views_out(v: &[core::DraftView]) -> Vec<DraftView> {
    v.iter()
        .map(|v| DraftView {
            id: v.id.0,
            name: v.name.clone(),
            scale: v.scale,
            underlays: v.underlays.iter().map(Underlay::out).collect(),
        })
        .collect()
}

pub(crate) fn draft_views_in(v: Vec<DraftView>) -> Result<Vec<core::DraftView>, FileError> {
    v.into_iter()
        .map(|v| {
            let element = format!("view for drawing {}", v.id);
            Ok(core::DraftView {
                id: core::DraftViewId(v.id),
                name: v.name,
                scale: v.scale,
                underlays: v
                    .underlays
                    .into_iter()
                    .map(|u| u.into_core(&element))
                    .collect::<Result<_, _>>()?,
            })
        })
        .collect()
}

pub(crate) fn level_underlays_out(
    v: &[(core::LevelId, Vec<core::Underlay>)],
) -> Vec<LevelUnderlays> {
    v.iter()
        .map(|(l, u)| LevelUnderlays {
            level: l.0,
            underlays: u.iter().map(Underlay::out).collect(),
        })
        .collect()
}

pub(crate) fn level_underlays_in(
    v: Vec<LevelUnderlays>,
) -> Result<Vec<(core::LevelId, Vec<core::Underlay>)>, FileError> {
    v.into_iter()
        .map(|l| {
            let element = format!("underlays of level {}", l.level);
            Ok((
                core::LevelId(l.level),
                l.underlays
                    .into_iter()
                    .map(|u| u.into_core(&element))
                    .collect::<Result<_, _>>()?,
            ))
        })
        .collect()
}

pub(crate) fn sheets_out(sheets: &[core::Sheet]) -> Vec<Sheet> {
    sheets
        .iter()
        .map(|s| Sheet {
            id: s.id.0,
            number: s.number.clone(),
            name: s.name.clone(),
            width: s.width,
            height: s.height,
            title_block: s.title_block,
            revision: s.revision.clone(),
            presentation: s.presentation.map(|p| p.0),
        })
        .collect()
}

pub(crate) fn sheets_in(sheets: Vec<Sheet>) -> Vec<core::Sheet> {
    sheets
        .into_iter()
        .map(|s| core::Sheet {
            id: core::SheetId(s.id),
            number: s.number,
            name: s.name,
            width: s.width,
            height: s.height,
            title_block: s.title_block,
            revision: s.revision,
            presentation: s.presentation.map(core::PresentationStyleId),
        })
        .collect()
}

pub(crate) fn viewports_out(v: &[core::Viewport]) -> Vec<Viewport> {
    v.iter()
        .map(|v| Viewport {
            id: v.id.0,
            sheet: v.sheet.0,
            level: v.level.0,
            section: v.section.map(|s| s.0),
            axo: v.axo.map(|a| [a.yaw, a.pitch]),
            scale: v.scale,
            cut_plane: v.cut_plane,
            crop: v.crop.map(|(a, b)| [xy(a), xy(b)]),
            centre: xy(v.centre),
            style: Style::out(&v.style),
            ceiling: v.ceiling,
            title: v.title.clone(),
            show_title: v.show_title,
            drafting: v.drafting.map(|d| d.0),
        })
        .collect()
}

pub(crate) fn viewports_in(v: Vec<Viewport>) -> Result<Vec<core::Viewport>, FileError> {
    v.into_iter()
        .map(|v| {
            let element = format!("view {} on a sheet", v.id);
            Ok(core::Viewport {
                id: core::ViewportId(v.id),
                sheet: core::SheetId(v.sheet),
                level: core::LevelId(v.level),
                section: v.section.map(core::SectionId),
                axo: v.axo.map(|[yaw, pitch]| core::Axo { yaw, pitch }),
                scale: v.scale,
                cut_plane: v.cut_plane,
                crop: v.crop.map(|[a, b]| (pt(a), pt(b))),
                centre: pt(v.centre),
                style: v.style.into_core(&element)?,
                ceiling: v.ceiling,
                title: v.title,
                show_title: v.show_title,
                drafting: v.drafting.map(core::DraftViewId),
            })
        })
        .collect()
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Table {
    pub id: u64,
    pub sheet: u64,
    pub kind: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub level: Option<u64>,
    pub at: Xy,
}

pub(crate) fn tables_out(t: &[core::SheetTable]) -> Vec<Table> {
    t.iter()
        .map(|t| Table {
            id: t.id.0,
            sheet: t.sheet.0,
            kind: t.kind.code().into(),
            level: t.level.map(|l| l.0),
            at: xy(t.at),
        })
        .collect()
}

pub(crate) fn tables_in(t: Vec<Table>) -> Result<Vec<core::SheetTable>, FileError> {
    t.into_iter()
        .map(|t| {
            Ok(core::SheetTable {
                id: core::TableId(t.id),
                sheet: core::SheetId(t.sheet),
                kind: core::ScheduleKind::from_code(&t.kind).ok_or_else(|| FileError::Value {
                    element: format!("schedule {}", t.id),
                    message: format!("unknown schedule «{}»", t.kind),
                })?,
                level: t.level.map(core::LevelId),
                at: pt(t.at),
            })
        })
        .collect()
}

fn one() -> f64 {
    1.0
}

fn is_one(v: &f64) -> bool {
    (*v - 1.0).abs() < 1e-12
}
