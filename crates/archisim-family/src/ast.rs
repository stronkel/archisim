//! The parsed form of a family recipe.

use std::collections::BTreeMap;

use crate::Value;

/// A parsed family recipe.
#[derive(Debug, Clone, PartialEq)]
pub struct FamilyDef {
    /// Name shown to the user, e.g. "Puerta abatible".
    pub name: String,
    /// What the element is: `door`, `window`, `furniture`, `sanitary`…
    pub category: String,
    pub host: Host,
    /// Version of the recipe, raised by its author on every change.
    pub version: u32,
    pub description: String,
    /// The folder it is shown in within its category in the gallery
    /// ("Camas", "Vegetación"…); empty: the category's own.
    pub folder: String,
    /// Type parameters first, then instance parameters, each in the order
    /// written.
    pub params: Vec<Param>,
    /// Top-level `let` and `require` lines, evaluated in order before the
    /// body and the outputs.
    pub prelude: Vec<Stmt>,
    /// Shared pieces ("piezas comunes"), by name.
    pub components: BTreeMap<String, Component>,
    /// The 3D body.
    pub body: Vec<Stmt>,
    /// Plan symbols, one per drawing scale (see [`FamilyDef::plan_for`]).
    pub plans: Vec<PlanSymbol>,
    /// Marks drawn on the element seen in elevation from the side its
    /// leaves open to (opening lines), in `x` and `z`; `None` if the
    /// recipe has no `elevation` block.
    pub elevation: Option<PlanSymbol>,
    /// Values read by the rest of the program (rough opening, frame…).
    pub outputs: Vec<Output>,
    /// IFC data (class, predefined type…).
    pub ifc: Vec<Output>,
    /// Types kept with the recipe in a library file (`catalog`), like the
    /// types of a Revit family file.
    pub catalog: Catalog,
}

/// The types saved with a family in a library file, and the materials they
/// use (so another project can make them as they were).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Catalog {
    pub materials: Vec<CatalogMaterial>,
    pub types: Vec<CatalogType>,
}

/// A material a catalogue type uses, by name.
#[derive(Debug, Clone, PartialEq)]
pub struct CatalogMaterial {
    pub name: String,
    pub category: String,
    /// sRGB.
    pub color: [u8; 3],
}

/// A type of the family: its name, mark, `GlobalId` and description, and
/// the values of its type parameters (materials by name).
#[derive(Debug, Clone, PartialEq)]
pub struct CatalogType {
    pub name: String,
    pub mark: String,
    pub guid: String,
    pub description: String,
    pub values: Vec<(String, Value)>,
}

impl FamilyDef {
    pub fn param(&self, name: &str) -> Option<&Param> {
        self.params.iter().find(|p| p.name == name)
    }

    /// The plan symbol to draw at a scale (denominator, 50 for 1:50): the
    /// one written for the nearest scale, as ArchiCAD picks the detail level
    /// by scale.
    pub fn plan_for(&self, scale: f64) -> Option<&PlanSymbol> {
        let distance = |p: &PlanSymbol| (scale.max(1.0) / p.scale).ln().abs();
        self.plans
            .iter()
            .min_by(|a, b| distance(a).total_cmp(&distance(b)))
    }

    /// Parameters of one scope, in order.
    pub fn params_of(&self, scope: ParamScope) -> impl Iterator<Item = &Param> {
        self.params.iter().filter(move |p| p.scope == scope)
    }
}

/// Where an element can be placed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Host {
    /// Placed freely on a level (furniture).
    None,
    /// Placed in a wall, cutting its rough opening (doors and windows).
    Wall,
}

/// Whether a parameter belongs to the type or to each placed element.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParamScope {
    Type,
    Instance,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ParamKind {
    /// Metres.
    Length,
    /// Radians.
    Angle,
    Number,
    Integer,
    Bool,
    Material,
    Text,
    /// One of a list of options.
    Choice(Vec<String>),
    /// A line style for plan marks (its code: `"dashed"`…).
    LineStyle,
}

impl ParamKind {
    pub fn name(&self) -> &'static str {
        match self {
            ParamKind::Length => "length",
            ParamKind::Angle => "angle",
            ParamKind::Number => "number",
            ParamKind::Integer => "integer",
            ParamKind::Bool => "bool",
            ParamKind::Material => "material",
            ParamKind::Text => "text",
            ParamKind::Choice(_) => "choice",
            ParamKind::LineStyle => "linestyle",
        }
    }

    pub fn is_numeric(&self) -> bool {
        matches!(
            self,
            ParamKind::Length | ParamKind::Angle | ParamKind::Number | ParamKind::Integer
        )
    }
}

/// A parameter declaration.
#[derive(Debug, Clone, PartialEq)]
pub struct Param {
    /// Name used in formulas (English, no spaces), e.g. `frame_width`.
    pub name: String,
    pub scope: ParamScope,
    pub kind: ParamKind,
    /// Default value. For materials, the material's name as
    /// [`Value::Text`]; the caller resolves it.
    pub default: Value,
    /// Allowed range of a numeric parameter, both ends included.
    pub range: Option<(f64, f64)>,
    /// Name shown to the user, e.g. "Hueco de obra: ancho".
    pub label: String,
    /// Section of the settings dialog, e.g. "Dimensiones".
    pub section: Option<String>,
    /// Shown in the settings dialog only when this is true.
    pub visible_if: Option<Expr>,
    pub line: usize,
}

/// A shared piece: a named block of geometry with arguments.
#[derive(Debug, Clone, PartialEq)]
pub struct Component {
    pub name: String,
    pub args: Vec<String>,
    pub body: Vec<Stmt>,
    pub line: usize,
}

/// A plan symbol for one drawing scale.
#[derive(Debug, Clone, PartialEq)]
pub struct PlanSymbol {
    /// Scale denominator: 50 for `plan 1:50`.
    pub scale: f64,
    pub body: Vec<Stmt>,
    pub line: usize,
}

/// Line weight of a plan mark. The real thickness depends on the scale and
/// the drawing standard, not on the family.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Pen {
    /// Heavy line of cut elements.
    Cut,
    /// Medium line of frames and leaves.
    Frame,
    /// Line of things seen below the cut plane.
    View,
    /// Thin line: glazing, swings, edges.
    Thin,
    /// Hatching.
    Hatch,
}

/// How a plan line is drawn: continuous or with one of the usual dash
/// patterns (UNE-EN ISO 128-2). Written in recipes by its English name.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LineStyle {
    Continuous,
    Dashed,
    Dotted,
    DashDot,
    DashDotDot,
    LongDash,
}

impl LineStyle {
    pub const ALL: [LineStyle; 6] = [
        LineStyle::Continuous,
        LineStyle::Dashed,
        LineStyle::Dotted,
        LineStyle::DashDot,
        LineStyle::DashDotDot,
        LineStyle::LongDash,
    ];

    /// The name used in recipes and in project files.
    pub fn code(self) -> &'static str {
        match self {
            LineStyle::Continuous => "continuous",
            LineStyle::Dashed => "dashed",
            LineStyle::Dotted => "dotted",
            LineStyle::DashDot => "dashdot",
            LineStyle::DashDotDot => "dashdotdot",
            LineStyle::LongDash => "longdash",
        }
    }

    pub fn from_code(code: &str) -> Option<LineStyle> {
        LineStyle::ALL.into_iter().find(|s| s.code() == code)
    }

    /// The name shown to the user, in Spanish.
    pub fn label(self) -> &'static str {
        match self {
            LineStyle::Continuous => "Continua",
            LineStyle::Dashed => "Discontinua",
            LineStyle::Dotted => "De puntos",
            LineStyle::DashDot => "Trazo y punto",
            LineStyle::DashDotDot => "Trazo y dos puntos",
            LineStyle::LongDash => "Trazos largos",
        }
    }

    /// Dash pattern in paper millimetres (dash, gap, dash, gap…); empty for
    /// a continuous line.
    pub fn pattern_mm(self) -> &'static [f64] {
        match self {
            LineStyle::Continuous => &[],
            LineStyle::Dashed => &[2.0, 1.0],
            LineStyle::Dotted => &[0.25, 0.75],
            LineStyle::DashDot => &[4.0, 1.0, 0.25, 1.0],
            LineStyle::DashDotDot => &[4.0, 1.0, 0.25, 1.0, 0.25, 1.0],
            LineStyle::LongDash => &[6.0, 2.0],
        }
    }
}

/// One value the family gives to the rest of the program.
#[derive(Debug, Clone, PartialEq)]
pub struct Output {
    pub name: String,
    pub expr: Expr,
    pub line: usize,
}

/// A rule the parameter values must satisfy, with the message shown when
/// they do not.
#[derive(Debug, Clone, PartialEq)]
pub struct Require {
    pub cond: Expr,
    pub message: String,
    pub line: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Stmt {
    /// A box between two corners, in the family's own axes.
    Box {
        min: [Expr; 3],
        max: [Expr; 3],
        material: Expr,
        line: usize,
    },
    /// Places a shared piece.
    Use {
        name: String,
        args: Vec<Expr>,
        line: usize,
    },
    /// Repeats a block for `var` from `from` up to `to` (excluded).
    Repeat {
        var: String,
        from: Expr,
        to: Expr,
        body: Vec<Stmt>,
        line: usize,
    },
    If {
        cond: Expr,
        then: Vec<Stmt>,
        otherwise: Vec<Stmt>,
        line: usize,
    },
    Let {
        name: String,
        expr: Expr,
        line: usize,
    },
    Require(Require),
    /// A plan line.
    Line {
        from: [Expr; 2],
        to: [Expr; 2],
        pen: Pen,
        style: Expr,
        line: usize,
    },
    /// A closed plan outline, filled white (it hides what is under it) or
    /// not.
    Polygon {
        points: Vec<[Expr; 2]>,
        pen: Pen,
        style: Expr,
        filled: bool,
        line: usize,
    },
    /// A plan arc around `center`, from one point to another, the short way.
    Arc {
        center: [Expr; 2],
        from: [Expr; 2],
        to: [Expr; 2],
        pen: Pen,
        style: Expr,
        line: usize,
    },
}

impl Stmt {
    /// Whether it draws in plan (it may not appear in the 3D body).
    pub fn is_plan_mark(&self) -> bool {
        matches!(
            self,
            Stmt::Line { .. } | Stmt::Polygon { .. } | Stmt::Arc { .. }
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinOp {
    Add,
    Sub,
    Mul,
    Div,
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
    And,
    Or,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    Num(f64),
    Str(String),
    Bool(bool),
    Var(String),
    Neg(Box<Expr>),
    Not(Box<Expr>),
    Bin(BinOp, Box<Expr>, Box<Expr>),
    Call(String, Vec<Expr>),
    If(Box<Expr>, Box<Expr>, Box<Expr>),
}
