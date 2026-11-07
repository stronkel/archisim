//! The studio standards a project can take in one go: line weights by
//! scale, lettering, the sheets' title block and accent colour, and the
//! presentation styles and view templates that go with them.
//!
//! The presentation styles carry the standard's pens and accent, so each
//! view (and each sheet) can be drawn in any of them while the project's
//! own standard is only what views without a style take. Every project
//! starts with them ([`add_schemes`]), and with a view template per
//! discipline ([`discipline_templates`]).
//!
//! - «Clásica»: what ArchiSim has always drawn (UNE-EN ISO 128 weights,
//!   the title block across the bottom right).
//! - «Hilo»: hairlines and a ladder of fine weights, small spaced capitals,
//!   a narrow title column under a monogram, and four ways of presenting
//!   the plans (hatched, solid black, solid white, soft colour).
//! - «Faro»: firmer cut lines, grey solid walls, the annotations and the
//!   sheets' titles in a warm red, and a strip down the right edge of the
//!   sheet with a stamp while the project is not for building.

use archisim_core::{
    Category, ElementDiscipline, GraphicStandard, Model, Pens, PresentationStyle, StylePens,
    TextFont, TitleBlock, TitleBlockDesign, TitleBlockId, ViewStyle, ViewTemplate, ViewTemplateId,
};

/// A studio standard, by its name.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Standard {
    Classic,
    Thread,
    Beacon,
}

impl Standard {
    pub const ALL: [Standard; 3] = [Standard::Classic, Standard::Thread, Standard::Beacon];

    pub fn name(self) -> &'static str {
        match self {
            Standard::Classic => "Clásica",
            Standard::Thread => "Hilo",
            Standard::Beacon => "Faro",
        }
    }

    pub fn description(self) -> &'static str {
        match self {
            Standard::Classic => {
                "Grosores de UNE-EN ISO 128 y el cajetín de celdas abajo a la derecha"
            }
            Standard::Thread => {
                "Líneas muy finas, capitales pequeñas, cajetín en columna con monograma y cuatro presentaciones: rayada, macizado negro, macizado blanco y color suave"
            }
            Standard::Beacon => {
                "Corte firme, muros macizados en gris, cotas y títulos en rojo cálido y una franja a la derecha del plano con sello «No válido para obra»"
            }
        }
    }

    /// Its graphic standard: pens, lettering, title block and accent.
    pub fn graphics(self) -> GraphicStandard {
        match self {
            Standard::Classic => GraphicStandard::default(),
            Standard::Thread => GraphicStandard {
                fine: Pens {
                    cut: 0.35,
                    frame: 0.18,
                    view: 0.13,
                    thin: 0.09,
                    hatch: 0.05,
                    dimension: 0.09,
                    tick: 0.18,
                    text: 2.0,
                },
                coarse: Pens {
                    cut: 0.25,
                    frame: 0.13,
                    view: 0.09,
                    thin: 0.05,
                    hatch: 0.05,
                    dimension: 0.05,
                    tick: 0.13,
                    text: 1.8,
                },
                coarse_from: 100.0,
                font: TextFont::Sans,
                title_block: TitleBlockDesign::Thread,
                accent: [26, 26, 26],
            },
            Standard::Beacon => GraphicStandard {
                fine: Pens {
                    cut: 0.6,
                    frame: 0.3,
                    view: 0.18,
                    thin: 0.13,
                    hatch: 0.09,
                    dimension: 0.13,
                    tick: 0.35,
                    text: 2.5,
                },
                coarse: Pens {
                    cut: 0.45,
                    frame: 0.25,
                    view: 0.13,
                    thin: 0.09,
                    hatch: 0.09,
                    dimension: 0.09,
                    tick: 0.25,
                    text: 2.0,
                },
                coarse_from: 100.0,
                font: TextFont::Sans,
                title_block: TitleBlockDesign::Beacon,
                accent: BEACON_RED,
            },
        }
    }

    /// The presentation styles that go with it, with its pens (and, for
    /// «Faro», its accent on the annotations).
    pub fn presentation_styles(self) -> Vec<PresentationStyle> {
        let g = self.graphics();
        let pens = Some(StylePens {
            fine: g.fine,
            coarse: g.coarse,
        });
        let style = |name: &str, palette: &[[u8; 3]], poche: Option<[u8; 3]>, lines: f64| {
            let mut s = PresentationStyle::new(name);
            s.palette = palette.to_vec();
            s.poche = poche;
            s.line_scale = lines;
            s.shadows = palette.len() > 2;
            s.pens = pens;
            s
        };
        let accented = |mut s: PresentationStyle| {
            s.accent = Some(BEACON_RED);
            s.accent_on = ACCENTED.into_iter().collect();
            // Flat greys, without the sun's shadows.
            s.shadows = false;
            s
        };
        match self {
            Standard::Classic => Vec::new(),
            Standard::Thread => vec![
                style("Hilo · rayado", &[], None, 1.0),
                style("Hilo · macizado negro", &[], Some([34, 34, 34]), 1.0),
                style("Hilo · macizado blanco", &[], Some([255, 255, 255]), 1.0),
                // Soft warm and cool tones, walls in a dusty rose.
                style(
                    "Hilo · color",
                    &[
                        [72, 66, 62],
                        [176, 150, 140],
                        [206, 214, 200],
                        [226, 222, 206],
                        [250, 248, 243],
                    ],
                    Some([214, 170, 162]),
                    1.0,
                ),
            ],
            Standard::Beacon => vec![
                // A ladder of greys, the walls in a middle one.
                accented(style(
                    "Faro",
                    &[
                        [40, 40, 40],
                        [140, 140, 140],
                        [218, 218, 218],
                        [255, 255, 255],
                    ],
                    Some([102, 102, 102]),
                    1.0,
                )),
                accented(style(
                    "Faro · gris claro",
                    &[
                        [60, 60, 60],
                        [170, 170, 170],
                        [230, 230, 230],
                        [255, 255, 255],
                    ],
                    Some([170, 170, 170]),
                    1.0,
                )),
                // Black and white, only the annotations in red.
                accented(style(
                    "Faro · blanco y negro",
                    &[[0, 0, 0], [255, 255, 255]],
                    Some([0, 0, 0]),
                    1.0,
                )),
            ],
        }
    }

    /// Takes the standard: its graphics for the whole project, and its
    /// presentation styles and view templates where the project has none
    /// of their names. Returns how many styles and templates were added.
    pub fn apply(self, m: &mut Model) -> (usize, usize) {
        m.graphics = self.graphics();
        // Its title block, the sheets' default.
        let tb = title_block_of(m, self.graphics().title_block);
        let _ = m.set_default_title_block(tb);
        let mut styles = 0;
        for s in self.presentation_styles() {
            if m.presentation_styles().any(|x| x.name == s.name) {
                continue;
            }
            if m.add_presentation_style(s).is_ok() {
                styles += 1;
            }
        }
        let style_id = |m: &Model, name: &str| {
            m.presentation_styles()
                .find(|s| s.name == name)
                .map(|s| s.id)
        };
        let mut templates = Vec::new();
        match self {
            Standard::Classic => {}
            Standard::Thread => {
                for (name, style) in [
                    ("Hilo · planta rayada", "Hilo · rayado"),
                    ("Hilo · planta macizado negro", "Hilo · macizado negro"),
                    ("Hilo · planta macizado blanco", "Hilo · macizado blanco"),
                    ("Hilo · planta en color", "Hilo · color"),
                ] {
                    let v = ViewStyle {
                        presentation: style_id(m, style),
                        ..Default::default()
                    };
                    templates.push((name, v));
                }
            }
            Standard::Beacon => {
                // The annotations in red come with the style's accent.
                let v = ViewStyle {
                    presentation: style_id(m, "Faro"),
                    ..Default::default()
                };
                templates.push(("Faro · planta", v));
            }
        }
        let mut added = 0;
        for (name, style) in templates {
            if m.view_templates().any(|t| t.name == name) {
                continue;
            }
            let t = ViewTemplate {
                id: ViewTemplateId(0),
                name: name.into(),
                style,
            };
            if m.add_view_template(t).is_ok() {
                added += 1;
            }
        }
        (styles, added)
    }

    /// The standard the project's graphics follow, if they are one of these.
    pub fn of(m: &Model) -> Option<Standard> {
        Standard::ALL
            .into_iter()
            .find(|s| s.graphics() == m.graphics)
    }
}

/// The warm red of «Faro».
pub const BEACON_RED: [u8; 3] = [214, 72, 44];

/// What «Faro» draws in its accent: the annotations (not the rooms, whose
/// floors are drawn with them).
pub const ACCENTED: [Category; 5] = [
    Category::Dimensions,
    Category::Tags,
    Category::LevelMarks,
    Category::SectionMarks,
    Category::Grids,
];

/// Every standard's presentation styles (the classic one has none).
pub fn schemes() -> Vec<PresentationStyle> {
    Standard::ALL
        .into_iter()
        .flat_map(|s| s.presentation_styles())
        .collect()
}

/// A view template per discipline: the view's discipline says what it
/// leaves out and greys (see `ViewStyle::effective`).
pub fn discipline_templates() -> Vec<(&'static str, ViewStyle)> {
    use ElementDiscipline as D;
    let style = |d: D| ViewStyle {
        discipline: Some(d),
        ..Default::default()
    };
    let mut v = vec![
        ("Arquitectura", style(D::Architecture)),
        ("Estructura", style(D::Structure)),
        ("Instalaciones", style(D::Services)),
        ("Urbanización", style(D::Site)),
    ];
    // One per kind of installation: the rest of them left out.
    for (name, g) in SERVICE_TEMPLATES {
        let mut s = style(D::Services);
        s.systems = [g].into();
        if g == archisim_core::ServiceGroup::Urban {
            s.discipline = Some(D::Site);
        }
        v.push((name, s));
    }
    v
}

/// The templates of each kind of installation, by name.
pub const SERVICE_TEMPLATES: [(&str, archisim_core::ServiceGroup); 7] = {
    use archisim_core::ServiceGroup as G;
    [
        ("Instalaciones · Electricidad", G::Electricity),
        ("Instalaciones · Fontanería", G::Water),
        ("Instalaciones · Saneamiento", G::Drainage),
        ("Instalaciones · Ventilación", G::Ventilation),
        ("Instalaciones · Climatización", G::Climate),
        ("Instalaciones · Contra incendios", G::Fire),
        ("Instalaciones · Redes urbanas", G::Urban),
    ]
};

/// The template the plans of a new project follow: «Arquitectura», which
/// leaves out the installations (they have their own templates).
pub fn set_architecture_plans(m: &mut Model) {
    let id = m
        .view_templates()
        .find(|t| t.name == "Arquitectura")
        .map(|t| t.id);
    if let Some(id) = id {
        let _ = m.set_plan_template(Some(id));
    }
}

/// The project's title block of a layout, by the layout's name; added
/// as ArchiSim draws it if missing.
pub fn title_block_of(m: &mut Model, design: TitleBlockDesign) -> Option<TitleBlockId> {
    let name = design.label();
    if let Some(t) = m.title_blocks().find(|t| t.name == name) {
        return Some(t.id);
    }
    m.add_title_block(TitleBlock::standard(design)).ok()
}

/// Adds the schemes, the discipline templates and the three title blocks
/// a project lacks (by name), the classic one the sheets' default if
/// none is. Returns how many styles and templates were added.
pub fn add_schemes(m: &mut Model) -> (usize, usize) {
    for d in TitleBlockDesign::ALL {
        let id = title_block_of(m, d);
        if m.default_title_block().is_none() && d == m.graphics.title_block {
            let _ = m.set_default_title_block(id);
        }
    }
    let mut styles = 0;
    for s in schemes() {
        if !m.presentation_styles().any(|x| x.name == s.name) && m.add_presentation_style(s).is_ok()
        {
            styles += 1;
        }
    }
    let mut templates = 0;
    for (name, style) in discipline_templates() {
        if m.view_templates().any(|t| t.name == name) {
            continue;
        }
        let t = ViewTemplate {
            id: ViewTemplateId(0),
            name: name.into(),
            style,
        };
        if m.add_view_template(t).is_ok() {
            templates += 1;
        }
    }
    (styles, templates)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn standards_apply_once() {
        let mut m = Model::new("Normas");
        let (s, t) = Standard::Thread.apply(&mut m);
        assert_eq!((s, t), (4, 4));
        assert_eq!(m.graphics.title_block, TitleBlockDesign::Thread);
        assert_eq!(Standard::of(&m), Some(Standard::Thread));
        let tb = m
            .default_title_block()
            .and_then(|t| m.title_block(t))
            .unwrap();
        assert_eq!(tb.design, TitleBlockDesign::Thread);
        // Again: nothing more is added.
        assert_eq!(Standard::Thread.apply(&mut m), (0, 0));
        let (s, t) = Standard::Beacon.apply(&mut m);
        assert_eq!((s, t), (3, 1));
        assert_eq!(m.graphics.accent, BEACON_RED);
        let tpl = m
            .view_templates()
            .find(|t| t.name == "Faro · planta")
            .unwrap();
        let faro = m
            .presentation_style(tpl.style.presentation.unwrap())
            .unwrap();
        // The red annotations come with the style, and its pens.
        let red = faro.accented(&tpl.style).unwrap();
        assert_eq!(red.graphics(Category::Dimensions).line, Some(BEACON_RED));
        assert_eq!(
            faro.pens_at(&m.graphics, 50.0),
            Some(Standard::Beacon.graphics().fine)
        );
        Standard::Classic.apply(&mut m);
        assert_eq!(m.graphics, GraphicStandard::default());
    }

    #[test]
    fn new_projects_have_the_schemes_and_discipline_templates() {
        let mut m = crate::new_project("Normas");
        assert!(m.presentation_styles().any(|s| s.name == "Hilo · color"));
        assert!(m.presentation_styles().any(|s| s.name == "Faro"));
        let arq = m
            .view_templates()
            .find(|t| t.name == "Arquitectura")
            .unwrap();
        assert!(arq
            .style
            .effective()
            .hidden_disciplines
            .contains(&ElementDiscipline::Services));
        // The three title blocks, the classic one by default.
        assert_eq!(m.title_blocks().count(), 3);
        let tb = m
            .default_title_block()
            .and_then(|t| m.title_block(t))
            .unwrap();
        assert_eq!(tb.design, TitleBlockDesign::Classic);
        // A template per kind of installation, showing only it.
        let water = m
            .view_templates()
            .find(|t| t.name == "Instalaciones · Fontanería")
            .unwrap();
        assert_eq!(
            water.style.systems,
            [archisim_core::ServiceGroup::Water].into()
        );
        // Nothing twice.
        assert_eq!(add_schemes(&mut m), (0, 0));
        assert_eq!(m.title_blocks().count(), 3);
    }
}
