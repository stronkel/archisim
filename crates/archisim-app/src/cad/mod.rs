//! AutoCAD-style commands, without any user interface: the command line's
//! names and aliases, typed coordinates, and the commands themselves as
//! small state machines that ask for points, values, options or objects
//! and give back the changes to make.
//!
//! The application feeds a running command with what the user does
//! ([`Input`]) and draws what it shows ([`Command::preview`]); each finished
//! step comes back as [`Changes`], applied to the model as one undoable
//! step.

pub mod dims;
pub mod draw;
pub mod inquiry;
pub mod modify;
pub mod rooms;

use std::collections::BTreeSet;

use archisim_core::{
    DraftId, DrawLayerId, HatchPattern, LineStyle, Model, NewDraft, Point2, Rgb, Shape, TextAlign,
    ViewRef, LAYER_ZERO,
};
use archisim_draft::{Seg, Transform};

use crate::geo::Geo;
use crate::pick::Element;
use crate::units::parse_length;

/// What the command asks for now.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Want {
    /// A point: a click (snapped) or typed coordinates.
    Point,
    /// A number or text typed in the command line (or a point, for
    /// distances given with two clicks).
    Value,
    /// Objects: the selection, made with clicks and windows; Enter to go on.
    Select,
    /// One object picked by a click (where it is clicked matters).
    Pick,
    /// A text, possibly of several lines, written in a box.
    Text,
}

/// An option of the prompt, chosen by typing its key or clicking it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Opt {
    pub key: &'static str,
    pub label: &'static str,
}

const fn opt(key: &'static str, label: &'static str) -> Opt {
    Opt { key, label }
}

/// The question of the command line: `CIRCULO Precise centro o [3P/2P]`.
#[derive(Debug, Clone, PartialEq)]
pub struct Prompt {
    pub text: String,
    pub options: Vec<Opt>,
    pub want: Want,
    /// The value Enter takes, shown as `<3000>`.
    pub default: Option<String>,
    /// A plain number typed is a value (an angle, a factor), not a
    /// distance from the last point.
    pub numeric: bool,
}

impl Prompt {
    fn new(text: impl Into<String>, want: Want) -> Self {
        Self {
            text: text.into(),
            options: Vec::new(),
            want,
            default: None,
            numeric: false,
        }
    }

    fn numeric(mut self) -> Self {
        self.numeric = true;
        self
    }

    fn opts(mut self, options: &[Opt]) -> Self {
        self.options = options.to_vec();
        self
    }

    fn default(mut self, d: impl Into<String>) -> Self {
        self.default = Some(d.into());
        self
    }

    /// The whole line as shown: text, options and default.
    pub fn line(&self) -> String {
        let mut s = self.text.clone();
        if !self.options.is_empty() {
            let o: Vec<&str> = self.options.iter().map(|o| o.label).collect();
            s.push_str(&format!(" o [{}]", o.join("/")));
        }
        if let Some(d) = &self.default {
            s.push_str(&format!(" <{d}>"));
        }
        s.push(':');
        s
    }
}

/// What the user gave to a command.
#[derive(Debug, Clone, PartialEq)]
pub enum Input {
    Point(Point2),
    /// Text typed in the command line (a number, a text).
    Value(String),
    /// One of the prompt's options, by its key.
    Option(&'static str),
    /// Enter (or Space, or right click) with nothing typed.
    Enter,
    /// An object picked, and where.
    Pick(Element, Point2),
}

/// A change a command makes to the model.
#[derive(Debug, Clone, PartialEq)]
pub enum Change {
    /// A new drafting element with the current layer and style.
    Add(Shape),
    /// A new drafting element exactly as given.
    AddDraft(NewDraft),
    /// New geometry for a drafting element.
    Reshape(DraftId, Shape),
    /// Removes an element.
    Remove(Element),
    /// Moves, turns, scales or mirrors elements (or copies of them).
    Transform {
        items: BTreeSet<Element>,
        t: Transform,
        copy: bool,
    },
    /// Copies the layer and style of `from` to the elements.
    MatchStyle { from: DraftId, to: Vec<DraftId> },
    /// A room found inside the walls around a point.
    RoomAt(Point2),
    /// A room with the given outline.
    Room(Vec<Point2>),
    /// A tag of an element; `None`: where it goes by default.
    Tag(Element, Option<Point2>),
    /// Tags on every door and window of the level that has none.
    TagAll,
}

/// The changes of a finished step of a command.
#[derive(Debug, Clone, PartialEq)]
pub struct Changes {
    /// Name of the step in the Edit menu.
    pub label: &'static str,
    pub changes: Vec<Change>,
}

/// What a command does after an input.
#[derive(Debug, Clone, PartialEq)]
pub enum Step {
    /// Waits for the next input.
    Next,
    /// Applies changes and waits for more.
    Apply(Changes),
    /// Applies changes (if any) and finishes.
    Done(Option<Changes>),
    /// Shows a result (distances, areas…) in the command line and waits.
    Say(String),
    /// Shows a result and finishes.
    SayDone(String),
    /// The input does not fit: say why and ask again.
    Refuse(String),
}

/// Settings of 2D drafting, kept between commands (AutoCAD keeps the
/// fillet radius, offset distance… as system variables).
#[derive(Debug, Clone, PartialEq)]
pub struct CadSettings {
    /// The current layer.
    pub layer: DrawLayerId,
    /// Colour, weight and line style of new elements (`None`: by layer).
    pub color: Option<Rgb>,
    pub weight: Option<f64>,
    pub line: Option<LineStyle>,
    /// Text height in paper millimetres, and alignment.
    pub text_height: f64,
    pub text_align: TextAlign,
    pub hatch_pattern: HatchPattern,
    pub hatch_scale: f64,
    /// Degrees.
    pub hatch_angle: f64,
    pub hatch_background: Option<Rgb>,
    /// 1 solid, less: see-through.
    pub hatch_opacity: f64,
    pub fillet_radius: f64,
    pub chamfer: (f64, f64),
    pub offset: f64,
    pub polygon_sides: usize,
    /// Revision cloud arc length, in paper millimetres.
    pub cloud_arc: f64,
    /// Width of new polylines, in metres.
    pub polyline_width: f64,
    pub array_rows: usize,
    pub array_cols: usize,
    pub array_count: usize,
    /// Orthogonal mode (F8): lines go horizontal or vertical.
    pub ortho: bool,
}

impl Default for CadSettings {
    fn default() -> Self {
        Self {
            layer: LAYER_ZERO,
            color: None,
            weight: None,
            line: None,
            text_height: 2.5,
            text_align: TextAlign::BottomLeft,
            hatch_pattern: HatchPattern::Diagonal,
            hatch_scale: 1.0,
            hatch_angle: 0.0,
            hatch_background: None,
            hatch_opacity: 1.0,
            fillet_radius: 0.0,
            chamfer: (0.0, 0.0),
            offset: 0.1,
            polygon_sides: 6,
            cloud_arc: 8.0,
            polyline_width: 0.0,
            array_rows: 2,
            array_cols: 3,
            array_count: 6,
            ortho: false,
        }
    }
}

/// What a command can look at.
pub struct Ctx<'a, G: Geo + ?Sized> {
    pub geo: &'a G,
    pub view: ViewRef,
    /// The selection, as it is now.
    pub selection: &'a BTreeSet<Element>,
    /// Metres of the plan per paper millimetre.
    pub k: f64,
    /// The snapped cursor.
    pub cursor: Point2,
}

impl<G: Geo + ?Sized> Ctx<'_, G> {
    pub fn model(&self) -> &Model {
        self.geo.model()
    }

    /// The shape of a drafting element.
    pub fn shape(&self, e: Element) -> Option<&Shape> {
        match e {
            Element::Draft(id) => self.model().draft(id).map(|d| &d.shape),
            _ => None,
        }
    }

    /// Every line and arc that can cut or bound in the view: drafting
    /// elements, the faces of walls and the edges of slabs (AutoCAD's
    /// "quick mode": everything is an edge).
    pub fn edges(&self) -> Vec<Seg> {
        let model = self.model();
        let mut v: Vec<Seg> = model
            .drafts_in(self.view)
            .filter(|d| model.draft_visible(d))
            .flat_map(|d| archisim_draft::boundary_segs(&d.shape))
            .collect();
        let ViewRef::Level(level) = self.view else {
            return v;
        };
        for w in model.walls_on_level(level) {
            if let Some(g) = self.geo.wall_geometry(w.id) {
                let n = g.outline.len();
                for i in 0..n {
                    let (a, b) = (g.outline[i], g.outline[(i + 1) % n]);
                    if a.distance(b) > 1e-9 {
                        v.push(Seg::Line { a, b });
                    }
                }
            }
        }
        for s in model.slabs_on_level(level) {
            let n = s.outline.len();
            for i in 0..n {
                v.push(Seg::Line {
                    a: s.outline[i],
                    b: s.outline[(i + 1) % n],
                });
            }
        }
        v
    }

    /// The drafting elements of the selection.
    pub fn selected_drafts(&self) -> Vec<DraftId> {
        self.selection
            .iter()
            .filter_map(|e| match e {
                Element::Draft(id) => Some(*id),
                _ => None,
            })
            .collect()
    }
}

/// A running command.
pub trait Command {
    /// Its name in the command line (Spanish, upper case).
    fn name(&self) -> &'static str;
    fn prompt(&self) -> Prompt;
    /// Called once when the command starts: commands that work on objects
    /// take the selection made beforehand (AutoCAD's noun-verb).
    fn start(&mut self, _cx: &Ctx<'_, dyn Geo + '_>, _st: &mut CadSettings) -> Step {
        Step::Next
    }
    fn feed(&mut self, input: Input, cx: &Ctx<'_, dyn Geo + '_>, st: &mut CadSettings) -> Step;
    /// Esc or Enter at the end: what is still to be made (a line drawn
    /// segment by segment is kept when the command is left).
    fn finish(&mut self, _cx: &Ctx<'_, dyn Geo + '_>, _st: &mut CadSettings) -> Option<Changes> {
        None
    }
    /// What to draw following the cursor.
    fn preview(&self, _cx: &Ctx<'_, dyn Geo + '_>, _st: &CadSettings) -> Vec<Shape> {
        Vec::new()
    }
    /// The last point given: `@` coordinates and tracking start from it.
    fn base(&self) -> Option<Point2> {
        None
    }
    /// Elements the command has picked so far, to highlight them.
    fn picked(&self) -> Vec<Element> {
        Vec::new()
    }
}

// ---- Command names ------------------------------------------------------

/// What a word typed in the command line starts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Launch {
    /// A drafting or editing command, by its name.
    Command(&'static str),
    /// A tool of the toolbox.
    Wall,
    Slab,
    Door,
    Window,
    Dimension,
    LevelMark,
    /// The drawing layers dialog.
    Layers,
    Undo,
    Redo,
    /// Zoom to fit.
    ZoomExtents,
    /// Orthogonal mode on or off.
    Ortho,
    /// The selection arrow.
    Select,
}

/// Every name and alias the command line knows, Spanish first (as in the
/// Spanish AutoCAD), then English.
pub const ALIASES: &[(&str, Launch)] = &[
    // Drawing.
    ("LINEA", Launch::Command("LINEA")),
    ("L", Launch::Command("LINEA")),
    ("LINE", Launch::Command("LINEA")),
    ("RELLRECT", Launch::Command("RELLRECT")),
    ("RELLPOLIG", Launch::Command("RELLPOLIG")),
    ("RELLREGULAR", Launch::Command("RELLREGULAR")),
    ("RELLCIRC", Launch::Command("RELLCIRC")),
    ("RELLELIPSE", Launch::Command("RELLELIPSE")),
    ("RELLSPLINE", Launch::Command("RELLSPLINE")),
    ("POL", Launch::Command("POL")),
    ("PL", Launch::Command("POL")),
    ("POLILINEA", Launch::Command("POL")),
    ("PLINE", Launch::Command("POL")),
    ("RECTANG", Launch::Command("RECTANG")),
    ("REC", Launch::Command("RECTANG")),
    ("RECTANGULO", Launch::Command("RECTANG")),
    ("RECTANGLE", Launch::Command("RECTANG")),
    ("POLIGONO", Launch::Command("POLIGONO")),
    ("PG", Launch::Command("POLIGONO")),
    ("POLYGON", Launch::Command("POLIGONO")),
    ("SPLINE", Launch::Command("SPLINE")),
    ("SPL", Launch::Command("SPLINE")),
    ("ARCO", Launch::Command("ARCO")),
    ("A", Launch::Command("ARCO")),
    ("ARC", Launch::Command("ARCO")),
    ("CIRCULO", Launch::Command("CIRCULO")),
    ("C", Launch::Command("CIRCULO")),
    ("CI", Launch::Command("CIRCULO")),
    ("CIRCLE", Launch::Command("CIRCULO")),
    ("ELIPSE", Launch::Command("ELIPSE")),
    ("EL", Launch::Command("ELIPSE")),
    ("ELLIPSE", Launch::Command("ELIPSE")),
    ("PUNTO", Launch::Command("PUNTO")),
    ("PO", Launch::Command("PUNTO")),
    ("POINT", Launch::Command("PUNTO")),
    ("TEXTO", Launch::Command("TEXTO")),
    ("TX", Launch::Command("TEXTO")),
    ("T", Launch::Command("TEXTO")),
    ("TEXTOM", Launch::Command("TEXTO")),
    ("TEXT", Launch::Command("TEXTO")),
    ("DT", Launch::Command("TEXTO")),
    ("MT", Launch::Command("TEXTO")),
    ("MTEXT", Launch::Command("TEXTO")),
    ("SOMBCONT", Launch::Command("SOMBCONT")),
    ("SOMBREADO", Launch::Command("SOMBCONT")),
    ("SB", Launch::Command("SOMBCONT")),
    ("H", Launch::Command("SOMBCONT")),
    ("BH", Launch::Command("SOMBCONT")),
    ("HATCH", Launch::Command("SOMBCONT")),
    ("NUBEREV", Launch::Command("NUBEREV")),
    ("NR", Launch::Command("NUBEREV")),
    ("REVCLOUD", Launch::Command("NUBEREV")),
    ("DIRECTRIZ", Launch::Command("DIRECTRIZ")),
    ("DIR", Launch::Command("DIRECTRIZ")),
    ("LE", Launch::Command("DIRECTRIZ")),
    ("LEADER", Launch::Command("DIRECTRIZ")),
    ("MLEADER", Launch::Command("DIRECTRIZ")),
    // Editing.
    ("DESPLAZA", Launch::Command("DESPLAZA")),
    ("D", Launch::Command("DESPLAZA")),
    ("MOVE", Launch::Command("DESPLAZA")),
    ("COPIA", Launch::Command("COPIA")),
    ("CP", Launch::Command("COPIA")),
    ("CO", Launch::Command("COPIA")),
    ("COPY", Launch::Command("COPIA")),
    ("GIRA", Launch::Command("GIRA")),
    ("GI", Launch::Command("GIRA")),
    ("RO", Launch::Command("GIRA")),
    ("ROTATE", Launch::Command("GIRA")),
    ("ESCALA", Launch::Command("ESCALA")),
    ("ES", Launch::Command("ESCALA")),
    ("SC", Launch::Command("ESCALA")),
    ("SCALE", Launch::Command("ESCALA")),
    ("SIMETRIA", Launch::Command("SIMETRIA")),
    ("SI", Launch::Command("SIMETRIA")),
    ("MI", Launch::Command("SIMETRIA")),
    ("MIRROR", Launch::Command("SIMETRIA")),
    ("EQUIDIST", Launch::Command("EQUIDIST")),
    ("EQ", Launch::Command("EQUIDIST")),
    ("O", Launch::Command("EQUIDIST")),
    ("OFFSET", Launch::Command("EQUIDIST")),
    ("RECORTA", Launch::Command("RECORTA")),
    ("RR", Launch::Command("RECORTA")),
    ("TR", Launch::Command("RECORTA")),
    ("TRIM", Launch::Command("RECORTA")),
    ("ALARGA", Launch::Command("ALARGA")),
    ("AL", Launch::Command("ALARGA")),
    ("EX", Launch::Command("ALARGA")),
    ("EXTEND", Launch::Command("ALARGA")),
    ("EMPALME", Launch::Command("EMPALME")),
    ("EM", Launch::Command("EMPALME")),
    ("F", Launch::Command("EMPALME")),
    ("FILLET", Launch::Command("EMPALME")),
    ("CHAFLAN", Launch::Command("CHAFLAN")),
    ("CHA", Launch::Command("CHAFLAN")),
    ("CHAMFER", Launch::Command("CHAFLAN")),
    ("MATRIZ", Launch::Command("MATRIZ")),
    ("MA", Launch::Command("MATRIZ")),
    ("AR", Launch::Command("MATRIZ")),
    ("ARRAY", Launch::Command("MATRIZ")),
    ("PARTE", Launch::Command("PARTE")),
    ("PA", Launch::Command("PARTE")),
    ("BR", Launch::Command("PARTE")),
    ("BREAK", Launch::Command("PARTE")),
    ("PARTEPUNTO", Launch::Command("PARTEPUNTO")),
    ("PP", Launch::Command("PARTEPUNTO")),
    ("BREAKATPOINT", Launch::Command("PARTEPUNTO")),
    ("JUNTAR", Launch::Command("JUNTAR")),
    ("JUN", Launch::Command("JUNTAR")),
    ("UNIR", Launch::Command("JUNTAR")),
    ("J", Launch::Command("JUNTAR")),
    ("JOIN", Launch::Command("JUNTAR")),
    ("DESCOMP", Launch::Command("DESCOMP")),
    ("DES", Launch::Command("DESCOMP")),
    ("X", Launch::Command("DESCOMP")),
    ("EXPLODE", Launch::Command("DESCOMP")),
    ("ESTIRA", Launch::Command("ESTIRA")),
    ("EST", Launch::Command("ESTIRA")),
    ("S", Launch::Command("ESTIRA")),
    ("STRETCH", Launch::Command("ESTIRA")),
    ("LONGITUD", Launch::Command("LONGITUD")),
    ("LO", Launch::Command("LONGITUD")),
    ("LEN", Launch::Command("LONGITUD")),
    ("LENGTHEN", Launch::Command("LONGITUD")),
    ("BORRA", Launch::Command("BORRA")),
    ("BORRAR", Launch::Command("BORRA")),
    ("B", Launch::Command("BORRA")),
    ("E", Launch::Command("BORRA")),
    ("ERASE", Launch::Command("BORRA")),
    ("IGUALARPROP", Launch::Command("IGUALARPROP")),
    ("IP", Launch::Command("IGUALARPROP")),
    ("MATCHPROP", Launch::Command("IGUALARPROP")),
    ("DIVIDE", Launch::Command("DIVIDE")),
    ("DIV", Launch::Command("DIVIDE")),
    ("ALINEAR", Launch::Command("ALINEAR")),
    ("ALI", Launch::Command("ALINEAR")),
    ("ALIGN", Launch::Command("ALINEAR")),
    // Inquiry.
    ("DIST", Launch::Command("DIST")),
    ("DI", Launch::Command("DIST")),
    ("AREA", Launch::Command("AREA")),
    ("AA", Launch::Command("AREA")),
    ("LIST", Launch::Command("LIST")),
    ("LI", Launch::Command("LIST")),
    ("LISTA", Launch::Command("LIST")),
    // ArchiSim tools.
    ("MURO", Launch::Wall),
    ("MU", Launch::Wall),
    ("M", Launch::Wall),
    ("WALL", Launch::Wall),
    ("FORJADO", Launch::Slab),
    ("FO", Launch::Slab),
    ("SLAB", Launch::Slab),
    ("PUERTA", Launch::Door),
    ("PUE", Launch::Door),
    ("P", Launch::Door),
    ("DOOR", Launch::Door),
    ("VENTANA", Launch::Window),
    ("VE", Launch::Window),
    ("V", Launch::Window),
    ("WINDOW", Launch::Window),
    ("LOCAL", Launch::Command("LOCAL")),
    ("LOC", Launch::Command("LOCAL")),
    ("ROOM", Launch::Command("LOCAL")),
    ("ESPACIO", Launch::Command("LOCAL")),
    ("ETIQUETA", Launch::Command("ETIQUETA")),
    ("ETI", Launch::Command("ETIQUETA")),
    ("TAG", Launch::Command("ETIQUETA")),
    ("ACOANGULO", Launch::Command("ACOANGULO")),
    ("DAN", Launch::Command("ACOANGULO")),
    ("DIMANGULAR", Launch::Command("ACOANGULO")),
    ("ACORADIO", Launch::Command("ACORADIO")),
    ("DRA", Launch::Command("ACORADIO")),
    ("DIMRADIUS", Launch::Command("ACORADIO")),
    ("ACODIAMETRO", Launch::Command("ACODIAMETRO")),
    ("DDI", Launch::Command("ACODIAMETRO")),
    ("DIMDIAMETER", Launch::Command("ACODIAMETRO")),
    ("ACOARCO", Launch::Command("ACOARCO")),
    ("DAR", Launch::Command("ACOARCO")),
    ("DIMARC", Launch::Command("ACOARCO")),
    ("COTA", Launch::Dimension),
    ("COT", Launch::Dimension),
    ("ACO", Launch::Dimension),
    ("ACOTAR", Launch::Dimension),
    ("DLI", Launch::Dimension),
    ("DIMLINEAR", Launch::Dimension),
    ("COTANIVEL", Launch::LevelMark),
    ("N", Launch::LevelMark),
    ("CN", Launch::LevelMark),
    // Settings and view.
    ("CAPA", Launch::Layers),
    ("CA", Launch::Layers),
    ("LA", Launch::Layers),
    ("LAYER", Launch::Layers),
    ("DESHACER", Launch::Undo),
    ("U", Launch::Undo),
    ("UNDO", Launch::Undo),
    ("REHACER", Launch::Redo),
    ("REDO", Launch::Redo),
    ("ZOOM", Launch::ZoomExtents),
    ("Z", Launch::ZoomExtents),
    ("ORTO", Launch::Ortho),
    ("ORTHO", Launch::Ortho),
    ("SEL", Launch::Select),
    ("SELECT", Launch::Select),
];

/// What a word typed in the command line starts, if anything. Spanish
/// accents and case do not matter (`línea`, `LINEA`).
pub fn launch(word: &str) -> Option<Launch> {
    let w: String = word
        .trim()
        .trim_start_matches(['_', '.', '-'])
        .chars()
        .map(|c| match c {
            'á' | 'Á' => 'A',
            'é' | 'É' => 'E',
            'í' | 'Í' => 'I',
            'ó' | 'Ó' => 'O',
            'ú' | 'Ú' => 'U',
            'ñ' | 'Ñ' => 'N',
            c => c.to_ascii_uppercase(),
        })
        .collect();
    ALIASES.iter().find(|(n, _)| *n == w).map(|(_, l)| *l)
}

/// A new command by its name.
pub fn new_command(name: &str) -> Option<Box<dyn Command>> {
    Some(match name {
        "LINEA" => Box::new(draw::Line::default()),
        "POL" => Box::new(draw::Pline::default()),
        "RECTANG" => Box::new(draw::Rectangle::default()),
        "POLIGONO" => Box::new(draw::Polygon::default()),
        "SPLINE" => Box::new(draw::SplineCmd::default()),
        "ARCO" => Box::new(draw::ArcCmd::default()),
        "CIRCULO" => Box::new(draw::Circle::default()),
        "ELIPSE" => Box::new(draw::Ellipse::default()),
        "PUNTO" => Box::new(draw::PointCmd),
        "TEXTO" => Box::new(draw::TextCmd::default()),
        "SOMBCONT" => Box::new(draw::HatchCmd::default()),
        "RELLRECT" => Box::new(draw::Filled::new(
            "RELLRECT",
            "Relleno",
            Box::new(draw::Rectangle::default()),
        )),
        "RELLPOLIG" => Box::new(draw::Filled::new(
            "RELLPOLIG",
            "Relleno",
            Box::new(draw::Pline::default()),
        )),
        "RELLREGULAR" => Box::new(draw::Filled::new(
            "RELLREGULAR",
            "Relleno",
            Box::new(draw::Polygon::default()),
        )),
        "RELLCIRC" => Box::new(draw::Filled::new(
            "RELLCIRC",
            "Relleno",
            Box::new(draw::Circle::default()),
        )),
        "RELLELIPSE" => Box::new(draw::Filled::new(
            "RELLELIPSE",
            "Relleno",
            Box::new(draw::Ellipse::default()),
        )),
        "RELLSPLINE" => Box::new(draw::Filled::new(
            "RELLSPLINE",
            "Relleno",
            Box::new(draw::SplineCmd::default()),
        )),
        "NUBEREV" => Box::new(draw::Cloud::default()),
        "DIRECTRIZ" => Box::new(draw::Leader::default()),
        "DESPLAZA" => Box::new(modify::Xform::new(modify::XformKind::Move)),
        "COPIA" => Box::new(modify::Xform::new(modify::XformKind::Copy)),
        "GIRA" => Box::new(modify::Xform::new(modify::XformKind::Rotate)),
        "ESCALA" => Box::new(modify::Xform::new(modify::XformKind::Scale)),
        "SIMETRIA" => Box::new(modify::Xform::new(modify::XformKind::Mirror)),
        "ALINEAR" => Box::new(modify::Xform::new(modify::XformKind::Align)),
        "EQUIDIST" => Box::new(modify::Offset::default()),
        "RECORTA" => Box::new(modify::TrimExtend::new(false)),
        "ALARGA" => Box::new(modify::TrimExtend::new(true)),
        "EMPALME" => Box::new(modify::Fillet::new(false)),
        "CHAFLAN" => Box::new(modify::Fillet::new(true)),
        "MATRIZ" => Box::new(modify::Array::default()),
        "PARTE" => Box::new(modify::Break::new(false)),
        "PARTEPUNTO" => Box::new(modify::Break::new(true)),
        "JUNTAR" => Box::new(modify::Join::default()),
        "DESCOMP" => Box::new(modify::Explode::default()),
        "ESTIRA" => Box::new(modify::Stretch::default()),
        "LONGITUD" => Box::new(modify::Lengthen::default()),
        "BORRA" => Box::new(modify::Erase::default()),
        "IGUALARPROP" => Box::new(modify::MatchProp::default()),
        "DIVIDE" => Box::new(modify::Divide::default()),
        "DIST" => Box::new(inquiry::Dist::default()),
        "AREA" => Box::new(inquiry::AreaCmd::default()),
        "LIST" => Box::new(inquiry::List),
        "LOCAL" => Box::new(rooms::RoomCmd::default()),
        "ETIQUETA" => Box::new(rooms::TagCmd::default()),
        "ACOANGULO" => Box::new(dims::DimAngle::default()),
        "ACORADIO" => Box::new(dims::DimRadius::new(false)),
        "ACODIAMETRO" => Box::new(dims::DimRadius::new(true)),
        "ACOARCO" => Box::new(dims::DimArc::default()),
        _ => return None,
    })
}

// ---- Typed coordinates and values --------------------------------------

/// A number typed in the command line (decimal comma or point).
pub fn parse_number(text: &str) -> Option<f64> {
    let t = text.trim().replace(',', ".");
    let v: f64 = t.parse().ok()?;
    v.is_finite().then_some(v)
}

/// An angle typed in degrees (counter-clockwise from +x), in radians.
pub fn parse_angle(text: &str) -> Option<f64> {
    let t = text.trim().trim_end_matches(['°', 'º', 'd', 'D']);
    parse_number(t).map(f64::to_radians)
}

fn pair(text: &str) -> Option<(f64, f64)> {
    let (a, b) = if text.contains(';') {
        text.split_once(';')?
    } else if text.matches(',').count() == 1 {
        text.split_once(',')?
    } else {
        return None;
    };
    Some((parse_length(a)?, parse_length(b)?))
}

/// A point typed in the command line, as in AutoCAD (millimetres, or with
/// `m`/`cm`):
/// - `3000,2000` or `3000;2000` absolute (also `#3000,2000`);
/// - `@3000,2000` relative to the last point;
/// - `@3000<45` polar from the last point (degrees);
/// - `3000` alone: that distance from the last point towards the cursor.
pub fn parse_point(text: &str, base: Option<Point2>, cursor: Point2) -> Option<Point2> {
    let t: String = text.chars().filter(|c| !c.is_whitespace()).collect();
    if t.is_empty() {
        return None;
    }
    if let Some(rel) = t.strip_prefix('@') {
        let b = base.unwrap_or_default();
        if let Some((d, a)) = rel.split_once('<') {
            let d = parse_length(d)?;
            let a = parse_angle(a)?;
            return Some(b + Point2::new(a.cos(), a.sin()) * d);
        }
        let (x, y) = pair(rel)?;
        return Some(b + Point2::new(x, y));
    }
    let abs = t.strip_prefix('#').unwrap_or(&t);
    if let Some((x, y)) = pair(abs) {
        return Some(Point2::new(x, y));
    }
    if let Some((d, a)) = abs.split_once('<') {
        // Polar from the origin.
        let d = parse_length(d)?;
        let a = parse_angle(a)?;
        return Some(Point2::new(a.cos(), a.sin()) * d);
    }
    // Direct distance entry.
    let d = parse_length(abs)?;
    let b = base?;
    let dir = (cursor - b).normalized()?;
    Some(b + dir * d)
}

/// The option of a prompt a typed word chooses: its key, or the start of
/// its label (case and accents ignored).
pub fn match_option(prompt: &Prompt, word: &str) -> Option<&'static str> {
    let w = word.trim().to_uppercase();
    if w.is_empty() {
        return None;
    }
    prompt
        .options
        .iter()
        .find(|o| o.key.eq_ignore_ascii_case(&w))
        .or_else(|| {
            prompt
                .options
                .iter()
                .find(|o| strip(o.label).starts_with(&strip(&w)))
        })
        .map(|o| o.key)
}

fn strip(s: &str) -> String {
    s.chars()
        .map(|c| match c {
            'á' | 'Á' => 'A',
            'é' | 'É' => 'E',
            'í' | 'Í' => 'I',
            'ó' | 'Ó' => 'O',
            'ú' | 'Ú' => 'U',
            c => c.to_ascii_uppercase(),
        })
        .collect()
}

/// Turns what was typed into the input the prompt wants: an option, a
/// point (for point prompts) or a plain value.
pub fn typed_input(prompt: &Prompt, text: &str, base: Option<Point2>, cursor: Point2) -> Input {
    if let Some(k) = match_option(prompt, text) {
        // A number never chooses an option.
        if parse_number(text).is_none() {
            return Input::Option(k);
        }
    }
    if prompt.numeric && parse_number(text).is_some() {
        return Input::Value(text.trim().to_string());
    }
    if prompt.want == Want::Point {
        if let Some(p) = parse_point(text, base, cursor) {
            return Input::Point(p);
        }
    }
    Input::Value(text.trim().to_string())
}

/// The point the cursor gives in orthogonal mode: horizontal or vertical
/// from `base`, whichever is nearer.
pub fn ortho(base: Point2, p: Point2) -> Point2 {
    let d = p - base;
    if d.x.abs() >= d.y.abs() {
        Point2::new(p.x, base.y)
    } else {
        Point2::new(base.x, p.y)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(x: f64, y: f64) -> Point2 {
        Point2::new(x, y)
    }

    #[test]
    fn coordinates_as_in_autocad() {
        let base = Some(p(1.0, 1.0));
        let cur = p(10.0, 1.0);
        assert_eq!(parse_point("3000,2000", base, cur), Some(p(3.0, 2.0)));
        assert_eq!(parse_point("3000;2000", base, cur), Some(p(3.0, 2.0)));
        assert_eq!(parse_point("@3000,-500", base, cur), Some(p(4.0, 0.5)));
        let polar = parse_point("@2000<90", base, cur).unwrap();
        assert!(polar.approx_eq(p(1.0, 3.0)));
        assert_eq!(parse_point("2,5m;1m", base, cur), Some(p(2.5, 1.0)));
        // Direct distance towards the cursor.
        assert_eq!(parse_point("1500", base, cur), Some(p(2.5, 1.0)));
        assert_eq!(parse_point("abc", base, cur), None);
    }

    #[test]
    fn names_and_options() {
        assert_eq!(launch("l"), Some(Launch::Command("LINEA")));
        assert_eq!(launch("línea"), Some(Launch::Command("LINEA")));
        assert_eq!(launch("TRIM"), Some(Launch::Command("RECORTA")));
        assert_eq!(launch("m"), Some(Launch::Wall));
        assert_eq!(launch("xyz"), None);
        for (name, l) in ALIASES {
            if let Launch::Command(c) = l {
                assert!(new_command(c).is_some(), "{name} → {c}");
            }
        }
        // Every alias appears once.
        let mut seen = std::collections::HashSet::new();
        for (name, _) in ALIASES {
            assert!(seen.insert(*name), "{name} repetido");
        }
        let pr =
            Prompt::new("Precise centro", Want::Point).opts(&[opt("3P", "3P"), opt("2P", "2P")]);
        assert_eq!(pr.line(), "Precise centro o [3P/2P]:");
        assert_eq!(match_option(&pr, "3p"), Some("3P"));
        assert_eq!(
            typed_input(&pr, "@1000,0", Some(p(0.0, 0.0)), p(0.0, 0.0)),
            Input::Point(p(1.0, 0.0))
        );
    }
}
