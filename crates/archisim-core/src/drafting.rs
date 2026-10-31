//! 2D drafting: drawing layers and the lines, arcs, texts and hatches drawn
//! on a view, as in AutoCAD.
//!
//! Drafting elements are annotations of a view (the plan of a level, or a
//! sheet): they are not building elements, have no `GlobalId` and are not
//! exported to IFC (Revit's detail lines and ArchiCAD's 2D elements are not
//! either). Coordinates are model metres in a plan view and paper metres on
//! a sheet. Text heights and the spacing of drafting hatches are paper
//! millimetres, so they print the same size at any scale.

use crate::family::LineStyle;
use crate::ids::{DraftId, DrawLayerId, LevelId};
use crate::math::{Point2, Vec2, EPSILON};
use crate::model::{Model, ModelError};

/// An RGB colour.
pub type Rgb = [u8; 3];

/// The layer every project has, as AutoCAD's layer "0". It cannot be
/// removed or renamed.
pub const LAYER_ZERO: DrawLayerId = DrawLayerId(0);

/// Line weights offered for layers and elements, in paper millimetres
/// (UNE-EN ISO 128-20 series).
pub const LINE_WEIGHTS: [f64; 8] = [0.09, 0.13, 0.18, 0.25, 0.35, 0.5, 0.7, 1.0];

/// A drawing layer (AutoCAD "capa"): how its elements are drawn unless they
/// say otherwise, and whether they are shown, editable and printed.
#[derive(Debug, Clone, PartialEq)]
pub struct DrawLayer {
    pub id: DrawLayerId,
    pub name: String,
    pub color: Rgb,
    /// Line weight in paper millimetres.
    pub weight: f64,
    pub line: LineStyle,
    /// Shown in views.
    pub visible: bool,
    /// Its elements cannot be selected or changed.
    pub locked: bool,
    /// Printed and exported (PDF, DXF); a layer for construction lines is
    /// usually not.
    pub printable: bool,
}

impl DrawLayer {
    /// A new visible, unlocked, printable layer.
    pub fn new(name: impl Into<String>, color: Rgb, weight: f64, line: LineStyle) -> Self {
        Self {
            id: DrawLayerId(0),
            name: name.into(),
            color,
            weight,
            line,
            visible: true,
            locked: false,
            printable: true,
        }
    }

    /// Layer "0": black, 0,18 mm, continuous.
    pub fn zero() -> Self {
        Self {
            id: LAYER_ZERO,
            ..Self::new("0", [0, 0, 0], 0.18, LineStyle::Continuous)
        }
    }
}

/// The view a drafting element is drawn on.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ViewRef {
    /// The plan of a level.
    Level(LevelId),
    /// A drawing sheet (coordinates in paper metres).
    Sheet(crate::ids::SheetId),
    /// A view only for drawing (E7: a detail, a diagram), in real metres.
    Drafting(crate::ids::DraftViewId),
}

/// A polyline of straight and arc segments (AutoCAD "polilínea").
#[derive(Debug, Clone, PartialEq)]
pub struct Polyline {
    pub points: Vec<Point2>,
    /// The bulge of the segment that starts at each point (`tan(θ/4)` of
    /// the angle it sweeps, positive counter-clockwise; 0 is straight), as
    /// in DXF. Same length as `points`; the last one is the closing segment
    /// of a closed polyline and is ignored on an open one.
    pub bulges: Vec<f64>,
    pub closed: bool,
    /// Constant width, in model metres (0: a plain line).
    pub width: f64,
}

impl Polyline {
    /// A polyline of straight segments.
    pub fn straight(points: Vec<Point2>, closed: bool) -> Self {
        let n = points.len();
        Self {
            points,
            bulges: vec![0.0; n],
            closed,
            width: 0.0,
        }
    }

    /// Number of segments.
    pub fn segment_count(&self) -> usize {
        let n = self.points.len();
        if n < 2 {
            0
        } else if self.closed {
            n
        } else {
            n - 1
        }
    }

    /// Segment `i`: its start, end and bulge.
    pub fn segment(&self, i: usize) -> (Point2, Point2, f64) {
        let n = self.points.len();
        (
            self.points[i],
            self.points[(i + 1) % n],
            self.bulges.get(i).copied().unwrap_or(0.0),
        )
    }

    /// Whether every segment is straight.
    pub fn is_straight(&self) -> bool {
        (0..self.segment_count()).all(|i| self.segment(i).2.abs() < 1e-12)
    }
}

/// Where a text sits on its insertion point (AutoCAD's justification).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TextAlign {
    TopLeft,
    TopCentre,
    TopRight,
    MiddleLeft,
    Middle,
    MiddleRight,
    #[default]
    BottomLeft,
    BottomCentre,
    BottomRight,
}

impl TextAlign {
    pub const ALL: [TextAlign; 9] = [
        TextAlign::TopLeft,
        TextAlign::TopCentre,
        TextAlign::TopRight,
        TextAlign::MiddleLeft,
        TextAlign::Middle,
        TextAlign::MiddleRight,
        TextAlign::BottomLeft,
        TextAlign::BottomCentre,
        TextAlign::BottomRight,
    ];

    /// Name used in project files.
    pub fn code(self) -> &'static str {
        match self {
            TextAlign::TopLeft => "top_left",
            TextAlign::TopCentre => "top_centre",
            TextAlign::TopRight => "top_right",
            TextAlign::MiddleLeft => "middle_left",
            TextAlign::Middle => "middle",
            TextAlign::MiddleRight => "middle_right",
            TextAlign::BottomLeft => "bottom_left",
            TextAlign::BottomCentre => "bottom_centre",
            TextAlign::BottomRight => "bottom_right",
        }
    }

    pub fn from_code(code: &str) -> Option<TextAlign> {
        TextAlign::ALL.into_iter().find(|a| a.code() == code)
    }

    /// Name shown to the user.
    pub fn label(self) -> &'static str {
        match self {
            TextAlign::TopLeft => "Superior izquierda",
            TextAlign::TopCentre => "Superior centro",
            TextAlign::TopRight => "Superior derecha",
            TextAlign::MiddleLeft => "Medio izquierda",
            TextAlign::Middle => "Medio centro",
            TextAlign::MiddleRight => "Medio derecha",
            TextAlign::BottomLeft => "Inferior izquierda",
            TextAlign::BottomCentre => "Inferior centro",
            TextAlign::BottomRight => "Inferior derecha",
        }
    }

    /// Horizontal anchor: 0 left, 0.5 centre, 1 right.
    pub fn horizontal(self) -> f64 {
        match self {
            TextAlign::TopLeft | TextAlign::MiddleLeft | TextAlign::BottomLeft => 0.0,
            TextAlign::TopCentre | TextAlign::Middle | TextAlign::BottomCentre => 0.5,
            _ => 1.0,
        }
    }

    /// Vertical anchor: 0 bottom (baseline of the last line), 0.5 middle,
    /// 1 top (cap height of the first line).
    pub fn vertical(self) -> f64 {
        match self {
            TextAlign::TopLeft | TextAlign::TopCentre | TextAlign::TopRight => 1.0,
            TextAlign::MiddleLeft | TextAlign::Middle | TextAlign::MiddleRight => 0.5,
            _ => 0.0,
        }
    }
}

/// A text: one or more lines (`\n`) written with the technical lettering.
#[derive(Debug, Clone, PartialEq)]
pub struct Text {
    pub at: Point2,
    pub content: String,
    /// Capital height in paper millimetres.
    pub height: f64,
    /// Direction of the baseline, radians counter-clockwise from +x.
    pub angle: f64,
    pub align: TextAlign,
}

/// Space between the baselines of a text's lines, as a multiple of its
/// height.
pub const LINE_SPACING: f64 = 1.6;

/// The patterns a hatch can be filled with. ArchiSim's own designs, after
/// the usual conventions of Spanish drawings.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum HatchPattern {
    /// Filled with colour.
    Solid,
    /// Parallel lines.
    Lines,
    /// Square grid.
    Grid,
    /// Diagonal lines at 45° (brickwork in section).
    Diagonal,
    /// Diagonal crossed lines.
    DiagonalCross,
    /// Brick bond in elevation (real size: 24 × 7 cm with joints).
    Brick,
    /// Concrete: dots and small strokes.
    Concrete,
    /// Dots (render, sand).
    Dots,
    /// Gravel: small circles.
    Gravel,
    /// Floor tiles (real size, 30 × 30 cm).
    Tiles,
    /// Ground in section: groups of diagonal strokes.
    Earth,
    /// Lawn: short tufts.
    Grass,
    /// Timber grain.
    Wood,
    /// Paving setts, 20 × 10 cm in stretcher bond (real size).
    Cobbles,
    /// Stone or concrete slabs, 60 × 40 cm in stretcher bond (real size).
    Slabs,
    /// Crazy paving: irregular stones of about 40 cm (real size).
    Stone,
    /// Decking: boards 14,5 cm wide with staggered joints (real size).
    Decking,
    /// Wood grain: wavy lines, each different.
    WoodGrain,
    /// Gravel as scattered dots.
    GravelDots,
    /// Lawn in mown stripes 1,5 m wide (real size): the pattern colour
    /// over the background, in alternate bands.
    LawnStripes,
    /// Lawn as scattered dots.
    LawnDots,
    /// A sheet of water: short ripples.
    Water,
    /// Timber boards with their joints and grain.
    WoodBoards,
    /// Flat-sawn grain: cathedral figures, no joints.
    WoodVeins,
    /// Finished timber: long smooth boards, little grain.
    WoodFinished,
    /// Pine boards: bold cathedrals and knots.
    Pine,
    /// Oak strips: straight grain and ray flecks.
    Oak,
    /// Maple strips: faint, quiet grain.
    Maple,
    /// Birch panels: fine wavy grain and flecks.
    Birch,
    /// Ash boards: strong cathedrals and open pores.
    Ash,
    /// Walnut boards: swirling grain.
    Walnut,
    /// Mahogany boards: ribbon stripes.
    Mahogany,
    /// Brushed metal sheets with seams and streaks.
    MetalBrushed,
    /// Perforated metal: round holes, staggered.
    MetalPerforated,
    /// Tread plate: raised lenses.
    DiamondPlate,
    /// Profiled (corrugated) sheet seen face on.
    Corrugated,
    /// Expanded metal mesh.
    MetalMesh,
    /// Fair-faced concrete: formwork panels and tie holes.
    ConcreteFinished,
    /// Raw concrete: dense stipple and aggregate.
    ConcreteRough,
    /// Bare concrete: light stipple, pores, hairline cracks.
    ConcreteBare,
    /// Concrete pavers 20 × 10 cm in herringbone.
    ConcretePavers,
    /// Board-formed concrete: the print of the boards.
    BoardFormed,
    /// Gravel: small rounded stones.
    Pebbles,
    /// Limestone slabs in courses, with fossils.
    Limestone,
    /// Rough stone blocks in courses.
    StoneBlocks,
    /// Cut stone 60 × 30 cm with drafted margins.
    Ashlar,
    /// Terrazzo: marble chips.
    Terrazzo,
    /// Rounded flat stones with grit between.
    RockyPath,
    /// Setts laid in segmental arcs.
    Setts,
    /// Rubble walling roughly in courses.
    Rubble,
}

impl HatchPattern {
    pub const ALL: [HatchPattern; 50] = [
        HatchPattern::Solid,
        HatchPattern::Lines,
        HatchPattern::Grid,
        HatchPattern::Diagonal,
        HatchPattern::DiagonalCross,
        HatchPattern::Brick,
        HatchPattern::Concrete,
        HatchPattern::Dots,
        HatchPattern::Gravel,
        HatchPattern::Tiles,
        HatchPattern::Earth,
        HatchPattern::Grass,
        HatchPattern::Wood,
        HatchPattern::Cobbles,
        HatchPattern::Slabs,
        HatchPattern::Stone,
        HatchPattern::Decking,
        HatchPattern::WoodGrain,
        HatchPattern::GravelDots,
        HatchPattern::LawnStripes,
        HatchPattern::LawnDots,
        HatchPattern::Water,
        HatchPattern::WoodBoards,
        HatchPattern::WoodVeins,
        HatchPattern::WoodFinished,
        HatchPattern::Pine,
        HatchPattern::Oak,
        HatchPattern::Maple,
        HatchPattern::Birch,
        HatchPattern::Ash,
        HatchPattern::Walnut,
        HatchPattern::Mahogany,
        HatchPattern::MetalBrushed,
        HatchPattern::MetalPerforated,
        HatchPattern::DiamondPlate,
        HatchPattern::Corrugated,
        HatchPattern::MetalMesh,
        HatchPattern::ConcreteFinished,
        HatchPattern::ConcreteRough,
        HatchPattern::ConcreteBare,
        HatchPattern::ConcretePavers,
        HatchPattern::BoardFormed,
        HatchPattern::Pebbles,
        HatchPattern::Limestone,
        HatchPattern::StoneBlocks,
        HatchPattern::Ashlar,
        HatchPattern::Terrazzo,
        HatchPattern::RockyPath,
        HatchPattern::Setts,
        HatchPattern::Rubble,
    ];

    pub fn code(self) -> &'static str {
        match self {
            HatchPattern::Solid => "solid",
            HatchPattern::Lines => "lines",
            HatchPattern::Grid => "grid",
            HatchPattern::Diagonal => "diagonal",
            HatchPattern::DiagonalCross => "diagonal_cross",
            HatchPattern::Brick => "brick",
            HatchPattern::Concrete => "concrete",
            HatchPattern::Dots => "dots",
            HatchPattern::Gravel => "gravel",
            HatchPattern::Tiles => "tiles",
            HatchPattern::Earth => "earth",
            HatchPattern::Grass => "grass",
            HatchPattern::Wood => "wood",
            HatchPattern::Cobbles => "cobbles",
            HatchPattern::Slabs => "slabs",
            HatchPattern::Stone => "stone",
            HatchPattern::Decking => "decking",
            HatchPattern::WoodGrain => "wood_grain",
            HatchPattern::GravelDots => "gravel_dots",
            HatchPattern::LawnStripes => "lawn_stripes",
            HatchPattern::LawnDots => "lawn_dots",
            HatchPattern::Water => "water",
            HatchPattern::WoodBoards => "wood_boards",
            HatchPattern::WoodVeins => "wood_veins",
            HatchPattern::WoodFinished => "wood_finished",
            HatchPattern::Pine => "pine",
            HatchPattern::Oak => "oak",
            HatchPattern::Maple => "maple",
            HatchPattern::Birch => "birch",
            HatchPattern::Ash => "ash",
            HatchPattern::Walnut => "walnut",
            HatchPattern::Mahogany => "mahogany",
            HatchPattern::MetalBrushed => "metal_brushed",
            HatchPattern::MetalPerforated => "metal_perforated",
            HatchPattern::DiamondPlate => "diamond_plate",
            HatchPattern::Corrugated => "corrugated",
            HatchPattern::MetalMesh => "metal_mesh",
            HatchPattern::ConcreteFinished => "concrete_finished",
            HatchPattern::ConcreteRough => "concrete_rough",
            HatchPattern::ConcreteBare => "concrete_bare",
            HatchPattern::ConcretePavers => "concrete_pavers",
            HatchPattern::BoardFormed => "board_formed",
            HatchPattern::Pebbles => "pebbles",
            HatchPattern::Limestone => "limestone",
            HatchPattern::StoneBlocks => "stone_blocks",
            HatchPattern::Ashlar => "ashlar",
            HatchPattern::Terrazzo => "terrazzo",
            HatchPattern::RockyPath => "rocky_path",
            HatchPattern::Setts => "setts",
            HatchPattern::Rubble => "rubble",
        }
    }

    pub fn from_code(code: &str) -> Option<HatchPattern> {
        HatchPattern::ALL.into_iter().find(|p| p.code() == code)
    }

    pub fn label(self) -> &'static str {
        match self {
            HatchPattern::Solid => "Relleno sólido",
            HatchPattern::Lines => "Líneas paralelas",
            HatchPattern::Grid => "Cuadrícula",
            HatchPattern::Diagonal => "Diagonal (fábrica)",
            HatchPattern::DiagonalCross => "Diagonal cruzada",
            HatchPattern::Brick => "Ladrillo visto",
            HatchPattern::Concrete => "Hormigón",
            HatchPattern::Dots => "Puntos",
            HatchPattern::Gravel => "Grava",
            HatchPattern::Tiles => "Baldosa 30×30",
            HatchPattern::Earth => "Terreno",
            HatchPattern::Grass => "Césped",
            HatchPattern::Wood => "Madera",
            HatchPattern::Cobbles => "Adoquín",
            HatchPattern::Slabs => "Losa 60×40",
            HatchPattern::Stone => "Piedra irregular",
            HatchPattern::Decking => "Tarima",
            HatchPattern::WoodGrain => "Veta de madera",
            HatchPattern::GravelDots => "Grava punteada",
            HatchPattern::LawnStripes => "Césped en franjas",
            HatchPattern::LawnDots => "Césped punteado",
            HatchPattern::Water => "Lámina de agua",
            HatchPattern::WoodBoards => "Madera en tablas",
            HatchPattern::WoodVeins => "Vetas de madera",
            HatchPattern::WoodFinished => "Madera terminada",
            HatchPattern::Pine => "Pino",
            HatchPattern::Oak => "Roble",
            HatchPattern::Maple => "Arce",
            HatchPattern::Birch => "Abedul",
            HatchPattern::Ash => "Fresno",
            HatchPattern::Walnut => "Nogal",
            HatchPattern::Mahogany => "Caoba",
            HatchPattern::MetalBrushed => "Metal cepillado",
            HatchPattern::MetalPerforated => "Metal perforado",
            HatchPattern::DiamondPlate => "Chapa lagrimada",
            HatchPattern::Corrugated => "Chapa grecada",
            HatchPattern::MetalMesh => "Malla metálica",
            HatchPattern::ConcreteFinished => "Hormigón visto",
            HatchPattern::ConcreteRough => "Hormigón en bruto",
            HatchPattern::ConcreteBare => "Hormigón desnudo",
            HatchPattern::ConcretePavers => "Adoquín de hormigón",
            HatchPattern::BoardFormed => "Hormigón encofrado con tablas",
            HatchPattern::Pebbles => "Grava gruesa",
            HatchPattern::Limestone => "Piedra caliza",
            HatchPattern::StoneBlocks => "Bloques de piedra",
            HatchPattern::Ashlar => "Sillería",
            HatchPattern::Terrazzo => "Terrazo",
            HatchPattern::RockyPath => "Camino de piedras",
            HatchPattern::Setts => "Adoquín en abanico",
            HatchPattern::Rubble => "Mampostería",
        }
    }

    /// Patterns drawn at their real size in the model (bricks, tiles), as
    /// Revit's model patterns; the others keep their size on paper.
    pub fn is_model_scale(self) -> bool {
        matches!(
            self,
            HatchPattern::Brick
                | HatchPattern::Tiles
                | HatchPattern::Cobbles
                | HatchPattern::Slabs
                | HatchPattern::Stone
                | HatchPattern::Decking
                | HatchPattern::LawnStripes
        ) || self.texture().is_some()
    }

    /// The family of a presentation texture, for lists: «Madera»,
    /// «Metal», «Hormigón» or «Piedra»; `None` for the other patterns.
    pub fn texture(self) -> Option<&'static str> {
        use HatchPattern::*;
        match self {
            WoodBoards | WoodVeins | WoodFinished | Pine | Oak | Maple | Birch | Ash | Walnut
            | Mahogany => Some("Madera"),
            MetalBrushed | MetalPerforated | DiamondPlate | Corrugated | MetalMesh => Some("Metal"),
            ConcreteFinished | ConcreteRough | ConcreteBare | ConcretePavers | BoardFormed => {
                Some("Hormigón")
            }
            Pebbles | Limestone | StoneBlocks | Ashlar | Terrazzo | RockyPath | Setts | Rubble => {
                Some("Piedra")
            }
            _ => None,
        }
    }

    /// Presentation patterns (textures for plans and site plans), after
    /// the technical ones in lists.
    pub fn is_presentation(self) -> bool {
        matches!(
            self,
            HatchPattern::Cobbles
                | HatchPattern::Slabs
                | HatchPattern::Stone
                | HatchPattern::Decking
                | HatchPattern::WoodGrain
                | HatchPattern::GravelDots
                | HatchPattern::LawnStripes
                | HatchPattern::LawnDots
                | HatchPattern::Water
        ) || self.texture().is_some()
    }
}

/// A hatch: a pattern filling the area inside closed boundaries (AutoCAD
/// "sombreado"). Loops inside other loops are holes (even-odd rule).
#[derive(Debug, Clone, PartialEq)]
pub struct Hatch {
    /// Closed boundaries.
    pub loops: Vec<Polyline>,
    pub pattern: HatchPattern,
    /// Multiplies the pattern's spacing.
    pub scale: f64,
    /// Turns the pattern, radians.
    pub angle: f64,
    /// Background colour behind the pattern (none: transparent).
    pub background: Option<Rgb>,
    /// How opaque the hatch is, its background and its pattern: 1 solid,
    /// down to 0,05 (see-through, over what is under it).
    pub opacity: f64,
}

/// What a drafted dimension measures (AutoCAD's angular, radius,
/// diameter and arc length dimensions). Its value is read from its own
/// geometry, so it changes when the dimension is stretched or scaled.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Measure {
    /// The angle from `start` through `sweep` (counter-clockwise, radians)
    /// about `centre`, drawn as an arc of `radius`.
    Angle {
        centre: Point2,
        start: f64,
        sweep: f64,
        radius: f64,
    },
    /// The radius of a circle or arc, drawn from its centre towards
    /// `angle`; the text at `reach` from the centre.
    Radius {
        centre: Point2,
        radius: f64,
        angle: f64,
        reach: f64,
    },
    /// The diameter, drawn across the circle through its centre at `angle`.
    Diameter {
        centre: Point2,
        radius: f64,
        angle: f64,
        reach: f64,
    },
    /// The length of an arc, drawn concentric with it, `offset` outside.
    ArcLength {
        centre: Point2,
        radius: f64,
        start: f64,
        sweep: f64,
        offset: f64,
    },
}

impl Measure {
    /// The measured value: radians for angles, metres otherwise.
    pub fn value(&self) -> f64 {
        match *self {
            Measure::Angle { sweep, .. } => sweep,
            Measure::Radius { radius, .. } => radius,
            Measure::Diameter { radius, .. } => 2.0 * radius,
            Measure::ArcLength { radius, sweep, .. } => radius * sweep,
        }
    }

    pub fn centre(&self) -> Point2 {
        match *self {
            Measure::Angle { centre, .. }
            | Measure::Radius { centre, .. }
            | Measure::Diameter { centre, .. }
            | Measure::ArcLength { centre, .. } => centre,
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            Measure::Angle { .. } => "Cota angular",
            Measure::Radius { .. } => "Cota de radio",
            Measure::Diameter { .. } => "Cota de diámetro",
            Measure::ArcLength { .. } => "Cota de longitud de arco",
        }
    }
}

/// The geometry of a drafting element.
#[derive(Debug, Clone, PartialEq)]
pub enum Shape {
    Line {
        a: Point2,
        b: Point2,
    },
    Polyline(Polyline),
    Circle {
        centre: Point2,
        radius: f64,
    },
    /// An arc counter-clockwise from `start` through `sweep` radians
    /// (0 < sweep < 2π), angles from +x.
    Arc {
        centre: Point2,
        radius: f64,
        start: f64,
        sweep: f64,
    },
    /// An ellipse (or an arc of it, from parameter `start` through `sweep`
    /// counter-clockwise; a full ellipse sweeps 2π). `axis` is the major
    /// semi-axis as a vector, `ratio` the minor to major ratio (0 < r ≤ 1).
    Ellipse {
        centre: Point2,
        axis: Vec2,
        ratio: f64,
        start: f64,
        sweep: f64,
    },
    /// A point marker.
    Point {
        at: Point2,
    },
    Text(Text),
    Hatch(Hatch),
    /// A drafted dimension.
    Dim(Measure),
}

impl Shape {
    /// The name shown in the interface.
    pub fn label(&self) -> &'static str {
        match self {
            Shape::Line { .. } => "Línea",
            Shape::Polyline(p) if p.closed => "Polilínea cerrada",
            Shape::Polyline(_) => "Polilínea",
            Shape::Circle { .. } => "Círculo",
            Shape::Arc { .. } => "Arco",
            Shape::Ellipse { sweep, .. } if *sweep >= std::f64::consts::TAU - 1e-9 => "Elipse",
            Shape::Ellipse { .. } => "Arco de elipse",
            Shape::Point { .. } => "Punto",
            Shape::Text(_) => "Texto",
            Shape::Hatch(_) => "Sombreado",
            Shape::Dim(m) => m.label(),
        }
    }

    /// Every coordinate that defines the shape (for checks and bounds;
    /// circles, arcs and ellipses give their bounding corners).
    pub fn key_points(&self) -> Vec<Point2> {
        match self {
            Shape::Line { a, b } => vec![*a, *b],
            Shape::Polyline(p) => p.points.clone(),
            Shape::Circle { centre, radius } | Shape::Arc { centre, radius, .. } => vec![
                *centre - Point2::new(*radius, *radius),
                *centre + Point2::new(*radius, *radius),
            ],
            Shape::Ellipse { centre, axis, .. } => {
                let r = axis.length();
                vec![*centre - Point2::new(r, r), *centre + Point2::new(r, r)]
            }
            Shape::Point { at } => vec![*at],
            Shape::Text(t) => vec![t.at],
            Shape::Hatch(h) => h.loops.iter().flat_map(|l| l.points.clone()).collect(),
            Shape::Dim(m) => {
                let c = m.centre();
                let r = match *m {
                    Measure::Angle { radius, .. } => radius,
                    Measure::Radius { radius, reach, .. }
                    | Measure::Diameter { radius, reach, .. } => radius.max(reach),
                    Measure::ArcLength { radius, offset, .. } => radius + offset.max(0.0),
                };
                vec![c - Point2::new(r, r), c + Point2::new(r, r)]
            }
        }
    }
}

/// A drafting element of a view.
#[derive(Debug, Clone, PartialEq)]
pub struct Draft {
    pub id: DraftId,
    pub view: ViewRef,
    pub layer: DrawLayerId,
    pub shape: Shape,
    /// Colour, weight (paper mm) and line style of the element; `None`
    /// takes the layer's ("PorCapa").
    pub color: Option<Rgb>,
    pub weight: Option<f64>,
    pub line: Option<LineStyle>,
}

/// Data needed to create a drafting element.
#[derive(Debug, Clone, PartialEq)]
pub struct NewDraft {
    pub view: ViewRef,
    pub layer: DrawLayerId,
    pub shape: Shape,
    pub color: Option<Rgb>,
    pub weight: Option<f64>,
    pub line: Option<LineStyle>,
}

impl NewDraft {
    /// A shape on a view and layer, with the layer's colour, weight and
    /// line style.
    pub fn new(view: ViewRef, layer: DrawLayerId, shape: Shape) -> Self {
        Self {
            view,
            layer,
            shape,
            color: None,
            weight: None,
            line: None,
        }
    }
}

/// How a drafting element is drawn: its own colour, weight and line style,
/// or its layer's.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DraftLook {
    pub color: Rgb,
    pub weight: f64,
    pub line: LineStyle,
}

fn finite(v: f64, what: &'static str) -> Result<(), ModelError> {
    if v.is_finite() {
        Ok(())
    } else {
        Err(ModelError::NotFinite(what))
    }
}

fn finite_point(p: Point2, what: &'static str) -> Result<(), ModelError> {
    finite(p.x, what)?;
    finite(p.y, what)
}

/// Largest bulge of a polyline segment: almost a full circle.
const MAX_DRAFT_BULGE: f64 = 1.0e3;

fn check_polyline(p: &Polyline, closed_needed: bool) -> Result<(), ModelError> {
    let min = if p.closed || closed_needed { 3 } else { 2 };
    let arcs = p.bulges.iter().any(|b| b.abs() > 1e-12);
    // A closed loop of two points is fine when it bulges (a circle drawn
    // as two half arcs).
    let min = if arcs && (p.closed || closed_needed) {
        2
    } else {
        min
    };
    if p.points.len() < min {
        return Err(ModelError::InvalidShape("polyline needs more points"));
    }
    if closed_needed && !p.closed {
        return Err(ModelError::InvalidShape("hatch boundaries must be closed"));
    }
    if p.bulges.len() != p.points.len() {
        return Err(ModelError::InvalidShape(
            "polyline needs one bulge per point",
        ));
    }
    for q in &p.points {
        finite_point(*q, "polyline point")?;
    }
    for b in &p.bulges {
        finite(*b, "polyline bulge")?;
        if b.abs() > MAX_DRAFT_BULGE {
            return Err(ModelError::InvalidShape("polyline arc too large"));
        }
    }
    finite(p.width, "polyline width")?;
    if p.width < 0.0 {
        return Err(ModelError::NonPositive("polyline width"));
    }
    let n = p.points.len();
    if (0..p.segment_count()).all(|i| p.points[i].distance(p.points[(i + 1) % n]) <= EPSILON) {
        return Err(ModelError::InvalidShape("polyline has no length"));
    }
    Ok(())
}

/// Checks a shape: finite numbers, positive sizes, enough points.
pub fn check_shape(shape: &Shape) -> Result<(), ModelError> {
    use std::f64::consts::TAU;
    match shape {
        Shape::Line { a, b } => {
            finite_point(*a, "line point")?;
            finite_point(*b, "line point")?;
            if a.distance(*b) <= EPSILON {
                return Err(ModelError::InvalidShape("line has no length"));
            }
        }
        Shape::Polyline(p) => check_polyline(p, false)?,
        Shape::Circle { centre, radius } => {
            finite_point(*centre, "circle centre")?;
            finite(*radius, "circle radius")?;
            if *radius <= EPSILON {
                return Err(ModelError::NonPositive("circle radius"));
            }
        }
        Shape::Arc {
            centre,
            radius,
            start,
            sweep,
        } => {
            finite_point(*centre, "arc centre")?;
            finite(*radius, "arc radius")?;
            finite(*start, "arc start")?;
            finite(*sweep, "arc sweep")?;
            if *radius <= EPSILON {
                return Err(ModelError::NonPositive("arc radius"));
            }
            if *sweep <= 1e-9 || *sweep >= TAU - 1e-9 {
                return Err(ModelError::InvalidShape(
                    "arc sweep must be between 0 and 360°",
                ));
            }
        }
        Shape::Ellipse {
            centre,
            axis,
            ratio,
            start,
            sweep,
        } => {
            finite_point(*centre, "ellipse centre")?;
            finite_point(*axis, "ellipse axis")?;
            finite(*ratio, "ellipse ratio")?;
            finite(*start, "ellipse start")?;
            finite(*sweep, "ellipse sweep")?;
            if axis.length() <= EPSILON {
                return Err(ModelError::NonPositive("ellipse axis"));
            }
            if *ratio <= 1e-6 || *ratio > 1.0 + 1e-9 {
                return Err(ModelError::InvalidShape("ellipse ratio must be in (0, 1]"));
            }
            if *sweep <= 1e-9 || *sweep > TAU + 1e-9 {
                return Err(ModelError::InvalidShape(
                    "ellipse sweep must be in (0, 360°]",
                ));
            }
        }
        Shape::Point { at } => finite_point(*at, "point")?,
        Shape::Text(t) => {
            finite_point(t.at, "text position")?;
            finite(t.height, "text height")?;
            finite(t.angle, "text angle")?;
            if t.height <= 0.0 {
                return Err(ModelError::NonPositive("text height"));
            }
            if t.content.trim().is_empty() {
                return Err(ModelError::InvalidShape("text is empty"));
            }
        }
        Shape::Hatch(h) => {
            if h.loops.is_empty() {
                return Err(ModelError::InvalidShape("hatch needs a boundary"));
            }
            for l in &h.loops {
                check_polyline(l, true)?;
            }
            finite(h.scale, "hatch scale")?;
            finite(h.angle, "hatch angle")?;
            if h.scale <= 0.0 {
                return Err(ModelError::NonPositive("hatch scale"));
            }
        }
        Shape::Dim(m) => {
            finite_point(m.centre(), "dimension centre")?;
            let (a, b, c) = match *m {
                Measure::Angle {
                    start,
                    sweep,
                    radius,
                    ..
                } => {
                    if sweep <= 1e-9 || sweep >= TAU - 1e-9 {
                        return Err(ModelError::InvalidShape("angle must be between 0 and 360°"));
                    }
                    (start, sweep, radius)
                }
                Measure::Radius {
                    radius,
                    angle,
                    reach,
                    ..
                }
                | Measure::Diameter {
                    radius,
                    angle,
                    reach,
                    ..
                } => (angle, reach, radius),
                Measure::ArcLength {
                    radius,
                    start,
                    sweep,
                    offset,
                    ..
                } => {
                    finite(offset, "dimension offset")?;
                    if sweep <= 1e-9 || sweep >= TAU - 1e-9 {
                        return Err(ModelError::InvalidShape(
                            "arc sweep must be between 0 and 360°",
                        ));
                    }
                    (start, sweep, radius)
                }
            };
            finite(a, "dimension angle")?;
            finite(b, "dimension value")?;
            finite(c, "dimension radius")?;
            if c <= EPSILON {
                return Err(ModelError::NonPositive("dimension radius"));
            }
        }
    }
    Ok(())
}

impl Model {
    // ---- Drawing layers ------------------------------------------------

    pub fn draw_layer(&self, id: DrawLayerId) -> Option<&DrawLayer> {
        self.draw_layers.get(&id)
    }

    /// Every drawing layer, "0" first and then by name.
    pub fn draw_layers(&self) -> Vec<&DrawLayer> {
        let mut v: Vec<&DrawLayer> = self.draw_layers.values().collect();
        v.sort_by(|a, b| {
            (a.id != LAYER_ZERO)
                .cmp(&(b.id != LAYER_ZERO))
                .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
        });
        v
    }

    pub fn draw_layer_by_name(&self, name: &str) -> Option<&DrawLayer> {
        self.draw_layers
            .values()
            .find(|l| l.name.eq_ignore_ascii_case(name))
    }

    fn check_draw_layer(
        &self,
        layer: &DrawLayer,
        this: Option<DrawLayerId>,
    ) -> Result<(), ModelError> {
        let name = layer.name.trim();
        if name.is_empty() {
            return Err(ModelError::InvalidShape("layer name is empty"));
        }
        if let Some(other) = self.draw_layer_by_name(name) {
            if Some(other.id) != this {
                return Err(ModelError::DuplicateName(name.to_string()));
            }
        }
        finite(layer.weight, "layer line weight")?;
        if layer.weight <= 0.0 {
            return Err(ModelError::NonPositive("layer line weight"));
        }
        Ok(())
    }

    /// Adds a drawing layer; names are unique (ignoring case).
    pub fn add_draw_layer(&mut self, layer: DrawLayer) -> Result<DrawLayerId, ModelError> {
        self.check_draw_layer(&layer, None)?;
        let id = DrawLayerId(self.take_id());
        self.draw_layers.insert(
            id,
            DrawLayer {
                id,
                name: layer.name.trim().to_string(),
                ..layer
            },
        );
        Ok(id)
    }

    /// Changes a layer's name, colour, weight, line style and switches.
    /// Layer "0" keeps its name.
    pub fn set_draw_layer(&mut self, layer: DrawLayer) -> Result<(), ModelError> {
        let id = layer.id;
        let old = self
            .draw_layers
            .get(&id)
            .ok_or(ModelError::UnknownDrawLayer(id))?;
        if id == LAYER_ZERO && layer.name.trim() != old.name {
            return Err(ModelError::InvalidShape("layer 0 cannot be renamed"));
        }
        self.check_draw_layer(&layer, Some(id))?;
        self.draw_layers.insert(
            id,
            DrawLayer {
                name: layer.name.trim().to_string(),
                ..layer
            },
        );
        Ok(())
    }

    /// Number of drafting elements on a layer.
    pub fn draw_layer_users(&self, id: DrawLayerId) -> usize {
        self.drafts.values().filter(|d| d.layer == id).count()
    }

    /// Removes a layer, moving its elements to `to` (AutoCAD's LAYMRG), or
    /// refusing if it still has elements and `to` is `None`. Layer "0"
    /// cannot be removed.
    pub fn remove_draw_layer(
        &mut self,
        id: DrawLayerId,
        to: Option<DrawLayerId>,
    ) -> Result<DrawLayer, ModelError> {
        if id == LAYER_ZERO {
            return Err(ModelError::InUse("layer 0".into()));
        }
        if !self.draw_layers.contains_key(&id) {
            return Err(ModelError::UnknownDrawLayer(id));
        }
        let users = self.draw_layer_users(id);
        match to {
            Some(t) if t != id => {
                if !self.draw_layers.contains_key(&t) {
                    return Err(ModelError::UnknownDrawLayer(t));
                }
                for d in self.drafts.values_mut().filter(|d| d.layer == id) {
                    d.layer = t;
                }
            }
            _ if users > 0 => {
                return Err(ModelError::InUse(format!("{users} drafting elements")));
            }
            _ => {}
        }
        self.draw_layers
            .remove(&id)
            .ok_or(ModelError::UnknownDrawLayer(id))
    }

    // ---- Drafting elements -------------------------------------------

    fn check_view(&self, view: ViewRef) -> Result<(), ModelError> {
        match view {
            ViewRef::Level(l) => {
                if self.level(l).is_none() {
                    return Err(ModelError::UnknownLevel(l));
                }
            }
            ViewRef::Sheet(s) => {
                if self.sheet(s).is_none() {
                    return Err(ModelError::UnknownSheet(s));
                }
            }
            ViewRef::Drafting(v) => {
                if self.draft_view(v).is_none() {
                    return Err(ModelError::UnknownDraftView(v));
                }
            }
        }
        Ok(())
    }

    fn check_draft(&self, spec: &NewDraft) -> Result<(), ModelError> {
        self.check_view(spec.view)?;
        if !self.draw_layers.contains_key(&spec.layer) {
            return Err(ModelError::UnknownDrawLayer(spec.layer));
        }
        if let Some(w) = spec.weight {
            finite(w, "line weight")?;
            if w <= 0.0 {
                return Err(ModelError::NonPositive("line weight"));
            }
        }
        check_shape(&spec.shape)
    }

    pub fn add_draft(&mut self, spec: NewDraft) -> Result<DraftId, ModelError> {
        self.check_draft(&spec)?;
        let id = DraftId(self.take_id());
        self.drafts.insert(
            id,
            Draft {
                id,
                view: spec.view,
                layer: spec.layer,
                shape: spec.shape,
                color: spec.color,
                weight: spec.weight,
                line: spec.line,
            },
        );
        Ok(id)
    }

    /// Replaces a drafting element's geometry.
    pub fn set_draft_shape(&mut self, id: DraftId, shape: Shape) -> Result<(), ModelError> {
        let d = self.drafts.get(&id).ok_or(ModelError::UnknownDraft(id))?;
        let spec = NewDraft {
            shape,
            ..self.draft_spec(d)
        };
        self.check_draft(&spec)?;
        if let Some(d) = self.drafts.get_mut(&id) {
            d.shape = spec.shape;
        }
        Ok(())
    }

    /// Replaces a drafting element's layer, colour, weight and line style.
    pub fn set_draft_style(
        &mut self,
        id: DraftId,
        layer: DrawLayerId,
        color: Option<Rgb>,
        weight: Option<f64>,
        line: Option<LineStyle>,
    ) -> Result<(), ModelError> {
        let d = self.drafts.get(&id).ok_or(ModelError::UnknownDraft(id))?;
        let spec = NewDraft {
            layer,
            color,
            weight,
            line,
            ..self.draft_spec(d)
        };
        self.check_draft(&spec)?;
        if let Some(d) = self.drafts.get_mut(&id) {
            d.layer = layer;
            d.color = color;
            d.weight = weight;
            d.line = line;
        }
        Ok(())
    }

    /// Moves a drafting element to another view, keeping its coordinates.
    pub fn set_draft_view(&mut self, id: DraftId, view: ViewRef) -> Result<(), ModelError> {
        self.check_view(view)?;
        let d = self
            .drafts
            .get_mut(&id)
            .ok_or(ModelError::UnknownDraft(id))?;
        d.view = view;
        Ok(())
    }

    fn draft_spec(&self, d: &Draft) -> NewDraft {
        NewDraft {
            view: d.view,
            layer: d.layer,
            shape: d.shape.clone(),
            color: d.color,
            weight: d.weight,
            line: d.line,
        }
    }

    pub fn remove_draft(&mut self, id: DraftId) -> Result<Draft, ModelError> {
        self.drafts.remove(&id).ok_or(ModelError::UnknownDraft(id))
    }

    pub fn draft(&self, id: DraftId) -> Option<&Draft> {
        self.drafts.get(&id)
    }

    pub fn drafts(&self) -> impl Iterator<Item = &Draft> {
        self.drafts.values()
    }

    /// The drafting elements of a view, in drawing order.
    pub fn drafts_in(&self, view: ViewRef) -> impl Iterator<Item = &Draft> {
        self.drafts.values().filter(move |d| d.view == view)
    }

    /// How an element is drawn, resolving "by layer".
    pub fn draft_look(&self, d: &Draft) -> DraftLook {
        let zero = DrawLayer::zero();
        let layer = self.draw_layers.get(&d.layer).unwrap_or(&zero);
        DraftLook {
            color: d.color.unwrap_or(layer.color),
            weight: d.weight.unwrap_or(layer.weight),
            line: d.line.unwrap_or(layer.line),
        }
    }

    /// Whether an element's layer is shown.
    pub fn draft_visible(&self, d: &Draft) -> bool {
        self.draw_layers.get(&d.layer).is_none_or(|l| l.visible)
    }

    /// Whether an element can be selected and changed (its layer is not
    /// locked, and shown).
    pub fn draft_editable(&self, d: &Draft) -> bool {
        self.draw_layers
            .get(&d.layer)
            .is_none_or(|l| l.visible && !l.locked)
    }

    /// Restores a layer from a project file.
    pub(crate) fn restore_draw_layer(&mut self, layer: DrawLayer) -> Result<(), ModelError> {
        self.check_draw_layer(&layer, Some(layer.id))?;
        self.draw_layers.insert(layer.id, layer);
        Ok(())
    }

    /// Restores a drafting element from a project file.
    pub(crate) fn restore_draft(&mut self, d: Draft) -> Result<(), ModelError> {
        self.check_draft(&self.draft_spec(&d))?;
        self.drafts.insert(d.id, d);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn model() -> (Model, LevelId) {
        let mut m = Model::new("Prueba");
        let l = m.add_level("Planta baja", 0.0);
        (m, l)
    }

    #[test]
    fn layer_zero_exists_and_stays() {
        let (mut m, _) = model();
        assert_eq!(m.draw_layers().len(), 1);
        assert_eq!(m.draw_layer(LAYER_ZERO).unwrap().name, "0");
        assert!(m.remove_draw_layer(LAYER_ZERO, None).is_err());
        let mut z = DrawLayer::zero();
        z.name = "Cero".into();
        assert!(m.set_draw_layer(z).is_err());
        let mut z = DrawLayer::zero();
        z.color = [255, 0, 0];
        m.set_draw_layer(z).unwrap();
        assert_eq!(m.draw_layer(LAYER_ZERO).unwrap().color, [255, 0, 0]);
    }

    #[test]
    fn layer_names_are_unique() {
        let (mut m, _) = model();
        let a = m
            .add_draw_layer(DrawLayer::new(
                " Ejes ",
                [200, 0, 0],
                0.13,
                LineStyle::DashDot,
            ))
            .unwrap();
        assert_eq!(m.draw_layer(a).unwrap().name, "Ejes");
        assert_eq!(
            m.add_draw_layer(DrawLayer::new("ejes", [0; 3], 0.13, LineStyle::Continuous)),
            Err(ModelError::DuplicateName("ejes".into()))
        );
    }

    #[test]
    fn drafts_take_their_layer_look_and_move_layers() {
        let (mut m, l) = model();
        let ejes = m
            .add_draw_layer(DrawLayer::new(
                "Ejes",
                [200, 0, 0],
                0.13,
                LineStyle::DashDot,
            ))
            .unwrap();
        let line = m
            .add_draft(NewDraft::new(
                ViewRef::Level(l),
                ejes,
                Shape::Line {
                    a: Point2::new(0.0, 0.0),
                    b: Point2::new(5.0, 0.0),
                },
            ))
            .unwrap();
        let look = m.draft_look(m.draft(line).unwrap());
        assert_eq!(look.color, [200, 0, 0]);
        assert_eq!(look.line, LineStyle::DashDot);
        m.set_draft_style(line, ejes, Some([0, 0, 255]), Some(0.5), None)
            .unwrap();
        let look = m.draft_look(m.draft(line).unwrap());
        assert_eq!(look.color, [0, 0, 255]);
        assert_eq!(look.weight, 0.5);
        assert!(m.remove_draw_layer(ejes, None).is_err());
        m.remove_draw_layer(ejes, Some(LAYER_ZERO)).unwrap();
        assert_eq!(m.draft(line).unwrap().layer, LAYER_ZERO);
        assert_eq!(m.drafts_in(ViewRef::Level(l)).count(), 1);
    }

    #[test]
    fn shapes_are_checked() {
        let (mut m, l) = model();
        let v = ViewRef::Level(l);
        let bad = [
            Shape::Line {
                a: Point2::new(1.0, 1.0),
                b: Point2::new(1.0, 1.0),
            },
            Shape::Circle {
                centre: Point2::new(0.0, 0.0),
                radius: 0.0,
            },
            Shape::Arc {
                centre: Point2::new(0.0, 0.0),
                radius: 1.0,
                start: 0.0,
                sweep: 7.0,
            },
            Shape::Polyline(Polyline::straight(vec![Point2::new(0.0, 0.0)], false)),
            Shape::Text(Text {
                at: Point2::new(0.0, 0.0),
                content: "  ".into(),
                height: 2.5,
                angle: 0.0,
                align: TextAlign::BottomLeft,
            }),
            Shape::Hatch(Hatch {
                loops: vec![Polyline::straight(
                    vec![
                        Point2::new(0.0, 0.0),
                        Point2::new(1.0, 0.0),
                        Point2::new(1.0, 1.0),
                    ],
                    false,
                )],
                pattern: HatchPattern::Solid,
                scale: 1.0,
                angle: 0.0,
                background: None,
                opacity: 1.0,
            }),
            Shape::Point {
                at: Point2::new(f64::NAN, 0.0),
            },
        ];
        for s in bad {
            assert!(
                m.add_draft(NewDraft::new(v, LAYER_ZERO, s.clone()))
                    .is_err(),
                "{s:?}"
            );
        }
        // Unknown layer.
        assert!(m
            .add_draft(NewDraft::new(
                v,
                DrawLayerId(999),
                Shape::Point {
                    at: Point2::new(0.0, 0.0)
                }
            ))
            .is_err());
    }

    #[test]
    fn removing_a_level_removes_its_drafts() {
        let (mut m, l) = model();
        let l2 = m.add_level("Primera", 3.0);
        m.add_draft(NewDraft::new(
            ViewRef::Level(l2),
            LAYER_ZERO,
            Shape::Point {
                at: Point2::new(0.0, 0.0),
            },
        ))
        .unwrap();
        assert_eq!(m.level_contents(l2).3, 1);
        m.remove_level(l2).unwrap();
        assert_eq!(m.drafts().count(), 0);
        assert_eq!(m.level_contents(l).3, 0);
    }
}
