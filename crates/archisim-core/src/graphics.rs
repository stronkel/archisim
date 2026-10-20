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
}

impl Category {
    pub const ALL: [Category; 21] = [
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
}

impl ViewStyle {
    pub fn shows(&self, c: Category) -> bool {
        !self.hidden.contains(&c)
    }

    pub fn shows_layer(&self, l: DrawLayerId) -> bool {
        !self.hidden_layers.contains(&l)
    }
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
}
