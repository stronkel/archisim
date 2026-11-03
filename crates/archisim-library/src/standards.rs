//! The studio standards a project can take in one go: line weights by
//! scale, lettering, the sheets' title block and accent colour, and the
//! presentation styles and view templates that go with them.
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
    Category, GraphicOverride, GraphicStandard, Model, Pens, PresentationStyle, TextFont,
    TitleBlockDesign, ViewStyle, ViewTemplate, ViewTemplateId,
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

    /// The presentation styles that go with it.
    pub fn presentation_styles(self) -> Vec<PresentationStyle> {
        let style = |name: &str, palette: &[[u8; 3]], poche: Option<[u8; 3]>, lines: f64| {
            let mut s = PresentationStyle::new(name);
            s.palette = palette.to_vec();
            s.poche = poche;
            s.line_scale = lines;
            s.shadows = !palette.is_empty();
            s
        };
        match self {
            Standard::Classic => Vec::new(),
            Standard::Thread => vec![
                style("Hilo · rayado", &[], None, 0.85),
                style("Hilo · macizado negro", &[], Some([34, 34, 34]), 0.85),
                style("Hilo · macizado blanco", &[], Some([255, 255, 255]), 0.85),
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
                    0.85,
                ),
            ],
            Standard::Beacon => vec![
                style("Faro", &[], Some([102, 102, 102]), 1.0),
                style("Faro · gris claro", &[], Some([153, 153, 153]), 1.0),
            ],
        }
    }

    /// Takes the standard: its graphics for the whole project, and its
    /// presentation styles and view templates where the project has none
    /// of their names. Returns how many styles and templates were added.
    pub fn apply(self, m: &mut Model) -> (usize, usize) {
        m.graphics = self.graphics();
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
                // The annotations in the accent colour.
                let mut v = ViewStyle {
                    presentation: style_id(m, "Faro"),
                    ..Default::default()
                };
                let red = GraphicOverride {
                    line: Some(BEACON_RED),
                    ..Default::default()
                };
                for c in [Category::Dimensions, Category::Tags, Category::Rooms] {
                    v.set_graphics(c, red);
                }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn standards_apply_once() {
        let mut m = crate::new_project("Normas");
        let (s, t) = Standard::Thread.apply(&mut m);
        assert_eq!((s, t), (4, 4));
        assert_eq!(m.graphics.title_block, TitleBlockDesign::Thread);
        assert_eq!(Standard::of(&m), Some(Standard::Thread));
        // Again: nothing more is added.
        assert_eq!(Standard::Thread.apply(&mut m), (0, 0));
        let (s, t) = Standard::Beacon.apply(&mut m);
        assert_eq!((s, t), (2, 1));
        assert_eq!(m.graphics.accent, BEACON_RED);
        let tpl = m
            .view_templates()
            .find(|t| t.name == "Faro · planta")
            .unwrap();
        assert_eq!(
            tpl.style.graphics(Category::Dimensions).line,
            Some(BEACON_RED)
        );
        assert!(tpl.style.presentation.is_some());
        Standard::Classic.apply(&mut m);
        assert_eq!(m.graphics, GraphicStandard::default());
    }
}
