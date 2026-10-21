//! How drawings look: the project's line weights by scale (its "normas
//! gráficas", as a pen table) and what each view shows (categories,
//! drawing layers, level of detail), as Revit's view templates and
//! ArchiCAD's pen sets and layer combinations.

use std::collections::BTreeSet;

use crate::ids::DrawLayerId;

/// Line weights and annotation sizes of a drawing, in paper millimetres.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Pens {
    /// Outline of what is cut (walls, slabs, columns).
    pub cut: f64,
    /// Frames and leaves of doors and windows.
    pub frame: f64,
    /// What is seen below the cut plane.
    pub view: f64,
    /// Layer boundaries, glazing, swings.
    pub thin: f64,
    /// Hatching of materials.
    pub hatch: f64,
    /// Dimension and extension lines.
    pub dimension: f64,
    /// Dimension ticks.
    pub tick: f64,
    /// Height of dimension texts.
    pub text: f64,
}

impl Pens {
    /// Fine scales (1:20, 1:50): UNE-EN ISO 128 weights, as ArchiSim drew
    /// so far.
    pub const FINE: Pens = Pens {
        cut: 0.5,
        frame: 0.25,
        view: 0.18,
        thin: 0.13,
        hatch: 0.09,
        dimension: 0.13,
        tick: 0.35,
        text: 2.5,
    };

    /// Coarse scales (1:100 and smaller): one step thinner.
    pub const COARSE: Pens = Pens {
        cut: 0.35,
        frame: 0.18,
        view: 0.13,
        thin: 0.09,
        hatch: 0.09,
        dimension: 0.09,
        tick: 0.25,
        text: 2.0,
    };
}

/// The project's graphic standard: pens for fine and coarse scales.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GraphicStandard {
    pub fine: Pens,
    pub coarse: Pens,
    /// Scales from 1:`coarse_from` down use the coarse pens.
    pub coarse_from: f64,
}

impl Default for GraphicStandard {
    fn default() -> Self {
        Self {
            fine: Pens::FINE,
            coarse: Pens::COARSE,
            coarse_from: 100.0,
        }
    }
}

impl GraphicStandard {
    /// The pens of a drawing at 1:`scale`.
    pub fn pens(&self, scale: f64) -> Pens {
        if scale >= self.coarse_from - 1e-9 {
            self.coarse
        } else {
            self.fine
        }
    }

    /// Whether every weight is a positive number.
    pub fn is_valid(&self) -> bool {
        let ok = |p: &Pens| {
            [
                p.cut,
                p.frame,
                p.view,
                p.thin,
                p.hatch,
                p.dimension,
                p.tick,
                p.text,
            ]
            .iter()
            .all(|v| v.is_finite() && *v > 0.0)
        };
        ok(&self.fine) && ok(&self.coarse) && self.coarse_from.is_finite() && self.coarse_from > 0.0
    }
}

/// What a view can hide.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Category {
    Walls,
    Slabs,
    Openings,
    Dimensions,
    LevelMarks,
    /// 2D drafting (lines, texts, hatches).
    Drafting,
    /// Locals and their names (IfcSpace).
    Rooms,
    /// Tags of doors, windows, walls and rooms.
    Tags,
    /// Columns and beams.
    Structure,
    Roofs,
    Stairs,
    CurtainWalls,
    Railings,
    /// Furniture, sanitary ware, kitchens and the garden's objects.
    Objects,
    Terrain,
    /// The marks of sections and elevations.
    SectionMarks,
    /// Setting-out grid lines.
    Grids,
    /// Footings and tie beams.
    Foundations,
    /// Volumes of the concept phase.
    Masses,
    /// Plots: boundaries and setbacks.
    Plots,
    Roads,
    /// Pipes, ducts and devices.
    Services,
}

impl Category {
    pub const ALL: [Category; 22] = [
        Category::Walls,
        Category::Slabs,
        Category::Openings,
        Category::Structure,
        Category::Foundations,
        Category::Roofs,
        Category::Stairs,
        Category::CurtainWalls,
        Category::Railings,
        Category::Objects,
        Category::Terrain,
        Category::Masses,
        Category::Plots,
        Category::Roads,
        Category::Services,
        Category::Dimensions,
        Category::LevelMarks,
        Category::SectionMarks,
        Category::Grids,
        Category::Drafting,
        Category::Rooms,
        Category::Tags,
    ];

    pub fn code(self) -> &'static str {
        match self {
            Category::Walls => "walls",
            Category::Slabs => "slabs",
            Category::Openings => "openings",
            Category::Dimensions => "dimensions",
            Category::LevelMarks => "level_marks",
            Category::Drafting => "drafting",
            Category::Rooms => "rooms",
            Category::Tags => "tags",
            Category::Structure => "structure",
            Category::Roofs => "roofs",
            Category::Stairs => "stairs",
            Category::CurtainWalls => "curtain_walls",
            Category::Railings => "railings",
            Category::Objects => "objects",
            Category::Terrain => "terrain",
            Category::SectionMarks => "section_marks",
            Category::Grids => "grids",
            Category::Foundations => "foundations",
            Category::Masses => "masses",
            Category::Plots => "plots",
            Category::Roads => "roads",
            Category::Services => "services",
        }
    }

    pub fn from_code(code: &str) -> Option<Category> {
        Category::ALL.into_iter().find(|c| c.code() == code)
    }

    pub fn label(self) -> &'static str {
        match self {
            Category::Walls => "Muros",
            Category::Slabs => "Forjados",
            Category::Openings => "Puertas y ventanas",
            Category::Dimensions => "Cotas",
            Category::LevelMarks => "Cotas de nivel",
            Category::Drafting => "Dibujo 2D",
            Category::Rooms => "Locales",
            Category::Tags => "Etiquetas",
            Category::Structure => "Pilares y vigas",
            Category::Roofs => "Cubiertas",
            Category::Stairs => "Escaleras",
            Category::CurtainWalls => "Muros cortina",
            Category::Railings => "Barandillas",
            Category::Objects => "Objetos y mobiliario",
            Category::Terrain => "Terreno",
            Category::SectionMarks => "Marcas de sección",
            Category::Grids => "Ejes",
            Category::Foundations => "Cimentación",
            Category::Masses => "Volúmenes (concepto)",
            Category::Plots => "Parcelas",
            Category::Roads => "Viales",
            Category::Services => "Instalaciones",
        }
    }
}

/// How much a view draws of each element.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Hash)]
pub enum Detail {
    /// Walls as one band, no hatches (1:200, site plans).
    Coarse,
    /// Layers without hatches (1:100).
    Medium,
    /// Everything: layers, hatches, every line of the symbols.
    #[default]
    Fine,
}

impl Detail {
    pub const ALL: [Detail; 3] = [Detail::Coarse, Detail::Medium, Detail::Fine];

    pub fn code(self) -> &'static str {
        match self {
            Detail::Coarse => "coarse",
            Detail::Medium => "medium",
            Detail::Fine => "fine",
        }
    }

    pub fn from_code(code: &str) -> Option<Detail> {
        Detail::ALL.into_iter().find(|d| d.code() == code)
    }

    pub fn label(self) -> &'static str {
        match self {
            Detail::Coarse => "Bajo",
            Detail::Medium => "Medio",
            Detail::Fine => "Alto",
        }
    }
}

/// What a view shows (Revit's visibility and view template).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ViewStyle {
    pub hidden: BTreeSet<Category>,
    pub hidden_layers: BTreeSet<DrawLayerId>,
    pub detail: Detail,
    /// What is drawn in grey, by discipline.
    pub focus: crate::DisciplineFocus,
    /// Disciplines left out of the view (the navigator's eye).
    pub hidden_disciplines: BTreeSet<crate::ElementDiscipline>,
    /// How it is presented: colours, solid walls, line weights.
    pub presentation: Option<crate::PresentationStyleId>,
}

impl ViewStyle {
    pub fn shows(&self, c: Category) -> bool {
        !self.hidden.contains(&c)
    }

    pub fn shows_layer(&self, l: DrawLayerId) -> bool {
        !self.hidden_layers.contains(&l)
    }
}

/// How a view is presented (the look of a presentation plan, as the
/// graphic overrides of ArchiCAD or Revit's view templates): the paper,
/// a palette the drawing's colours are turned to, walls and columns cut
/// solid in a colour, and the weight of the lines. It changes only how the
/// view is drawn.
#[derive(Debug, Clone, PartialEq)]
pub struct PresentationStyle {
    pub id: crate::PresentationStyleId,
    pub name: String,
    /// Colour of the paper.
    pub background: crate::Rgb,
    /// The tones of the drawing, from dark to light: every colour is
    /// turned to the tone of its lightness (the dark lines to the first,
    /// light fills to the last). Empty: the drawing keeps its colours.
    pub palette: Vec<crate::Rgb>,
    /// Cut walls and columns filled with this colour, without their
    /// layers; `None`: as the plan draws them.
    pub poche: Option<crate::Rgb>,
    /// Multiplies the weight of every line.
    pub line_scale: f64,
}

impl PresentationStyle {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            id: crate::PresentationStyleId(0),
            name: name.into(),
            background: [255, 255, 255],
            palette: Vec::new(),
            poche: None,
            line_scale: 1.0,
        }
    }

    pub fn is_valid(&self) -> bool {
        !self.name.trim().is_empty()
            && self.line_scale.is_finite()
            && self.line_scale > 0.0
            && self.line_scale <= 10.0
            && self.palette.len() <= 8
    }

    /// The tone a colour is turned to, by its lightness.
    pub fn tone(&self, c: crate::Rgb) -> crate::Rgb {
        let p = &self.palette;
        if p.is_empty() {
            return c;
        }
        if p.len() == 1 {
            return p[0];
        }
        let lum =
            |c: crate::Rgb| (0.3 * c[0] as f64 + 0.59 * c[1] as f64 + 0.11 * c[2] as f64) / 255.0;
        // The palette sorted by lightness, spread over 0..1.
        let mut tones = p.clone();
        tones.sort_by(|a, b| lum(*a).total_cmp(&lum(*b)));
        let t = lum(c) * (tones.len() - 1) as f64;
        let i = (t.floor() as usize).min(tones.len() - 2);
        let f = t - i as f64;
        let (a, b) = (tones[i], tones[i + 1]);
        let mix = |x: u8, y: u8| (x as f64 + (y as f64 - x as f64) * f).round() as u8;
        [mix(a[0], b[0]), mix(a[1], b[1]), mix(a[2], b[2])]
    }
}

/// The text of a file of presentation styles (`.aspres`, in the library
/// folder, to take them to other projects), one line per style:
///
/// ```text
/// style "Bosque" paper #f4f2ea palette #22342c #587a5e poche #2f4f3e lines 0.8
/// ```
pub fn presentation_styles_text(styles: &[PresentationStyle]) -> String {
    let hex = |c: crate::Rgb| format!("#{:02x}{:02x}{:02x}", c[0], c[1], c[2]);
    let mut s = String::from("# Estilos de presentación de ArchiSim\n");
    for st in styles {
        s.push_str(&format!(
            "style \"{}\" paper {}",
            st.name.replace('"', "'"),
            hex(st.background)
        ));
        if !st.palette.is_empty() {
            s.push_str(" palette");
            for c in &st.palette {
                s.push(' ');
                s.push_str(&hex(*c));
            }
        }
        if let Some(c) = st.poche {
            s.push_str(&format!(" poche {}", hex(c)));
        }
        s.push_str(&format!(" lines {}\n", st.line_scale));
    }
    s
}

/// Reads a file of presentation styles (see [`presentation_styles_text`]).
pub fn parse_presentation_styles(text: &str) -> Result<Vec<PresentationStyle>, String> {
    let colour = |w: &str| -> Option<crate::Rgb> {
        let h = w.strip_prefix('#')?;
        if h.len() != 6 || !h.is_ascii() {
            return None;
        }
        let v = |i: usize| u8::from_str_radix(&h[i..i + 2], 16).ok();
        Some([v(0)?, v(2)?, v(4)?])
    };
    let mut out = Vec::new();
    for (n, line) in text.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let bad = |what: &str| format!("línea {}: {what}", n + 1);
        let rest = line
            .strip_prefix("style \"")
            .ok_or_else(|| bad("debe empezar por style \"nombre\""))?;
        let end = rest
            .find('"')
            .ok_or_else(|| bad("falta la comilla del nombre"))?;
        let mut st = PresentationStyle::new(&rest[..end]);
        let words: Vec<&str> = rest[end + 1..].split_whitespace().collect();
        let mut i = 0;
        while i < words.len() {
            match words[i] {
                "paper" => {
                    i += 1;
                    st.background = words
                        .get(i)
                        .and_then(|w| colour(w))
                        .ok_or_else(|| bad("color del papel"))?;
                }
                "palette" => {
                    while let Some(c) = words.get(i + 1).and_then(|w| colour(w)) {
                        st.palette.push(c);
                        i += 1;
                    }
                }
                "poche" => {
                    i += 1;
                    st.poche = Some(
                        words
                            .get(i)
                            .and_then(|w| colour(w))
                            .ok_or_else(|| bad("color del macizado"))?,
                    );
                }
                "lines" => {
                    i += 1;
                    st.line_scale = words
                        .get(i)
                        .and_then(|w| w.parse().ok())
                        .ok_or_else(|| bad("grosor de las líneas"))?;
                }
                w => return Err(bad(&format!("no se entiende «{w}»"))),
            }
            i += 1;
        }
        if !st.is_valid() {
            return Err(bad("estilo no válido"));
        }
        out.push(st);
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pens_by_scale() {
        let g = GraphicStandard::default();
        assert_eq!(g.pens(50.0), Pens::FINE);
        assert_eq!(g.pens(100.0), Pens::COARSE);
        assert!(g.is_valid());
        let mut bad = g;
        bad.fine.cut = 0.0;
        assert!(!bad.is_valid());
        for c in Category::ALL {
            assert_eq!(Category::from_code(c.code()), Some(c));
        }
    }

    #[test]
    fn palette_tones_by_lightness() {
        let mut s = PresentationStyle::new("Terracota");
        assert_eq!(s.tone([10, 20, 30]), [10, 20, 30]);
        s.palette = vec![[250, 240, 230], [60, 30, 20], [190, 100, 70]];
        assert_eq!(s.tone([0, 0, 0]), [60, 30, 20]);
        assert_eq!(s.tone([255, 255, 255]), [250, 240, 230]);
        let mid = s.tone([128, 128, 128]);
        assert!(mid[0] > 150 && mid[0] < 220, "{mid:?}");
        assert!(s.is_valid());
        s.line_scale = 0.0;
        assert!(!s.is_valid());
    }

    #[test]
    fn styles_go_to_a_file_and_back() {
        let mut a = PresentationStyle::new("Bosque");
        a.background = [244, 242, 234];
        a.palette = vec![[34, 52, 44], [88, 122, 94]];
        a.poche = Some([47, 79, 62]);
        a.line_scale = 0.8;
        let b = PresentationStyle::new("Técnico");
        let text = presentation_styles_text(&[a.clone(), b.clone()]);
        let back = parse_presentation_styles(&text).unwrap();
        assert_eq!(back, vec![a, b]);
        assert!(parse_presentation_styles("style \"X\" paper rojo").is_err());
    }
}
