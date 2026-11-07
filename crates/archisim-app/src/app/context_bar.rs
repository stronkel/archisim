//! What the ribbon under the tabs holds (`docs/INTERFAZ.md` §13), and the
//! options bar under it, as Revit's «Modificar» tab: always the same
//! panels in the same order (Seleccionar, Portapapeles, Geometría,
//! Modificar, Vista, Medir, Crear), and after them those of what is in hand
//! — the tool's ways of drawing or placing, the sketch's Aceptar and
//! Cancelar, the selection's contour and faces. The options bar says what
//! is in hand («Modificar | Colocar Muro») with its settings (altura, línea
//! de ubicación, cadena, longitud y ángulo…), the step of the order running
//! and what a click does now. How it is laid out and drawn is in `ribbon`.

use eframe::egui::{self, Color32, CornerRadius, FontId, Rect, Sense, Stroke, Ui, Vec2};

use super::context_menu::MenuAct;
use super::glyphs::{self, Glyph};
use super::icons::{self, Icon};
use super::ribbon::{Item, Panel, Size};
use super::theme::Palette;
use super::toolbox::{Mark, Pick};
use super::widgets::{strong_font, SwitchExt};
use super::{ArchiSimApp, Tool};
use crate::cad::Input;
use crate::dim_tool::DimMode;
use crate::pick::Element;
use crate::slab_tool::SlabMode;
use crate::wall_tool::WallMode;

/// The options bar under the ribbon.
pub const OPTIONS_BAR_HEIGHT: f32 = 32.0;

/// What a button of the ribbon does.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) enum Act {
    Method(MethodAct),
    /// An order of «Modificar» (again while it runs: it ends).
    Order(&'static str),
    SameType,
    /// What the right-click menu does.
    Menu(MenuAct),
    Paint,
    /// Dividir cara on a face picked in 3D.
    DivideFace3d,
    Round,
    /// Its contour, or the zones of one of its faces, in sketch mode.
    Sketch(crate::sketch::SketchTarget),
    /// An order of the sketch's lines.
    SketchCmd(&'static str),
    SketchAccept,
    SketchCancel,
    /// «Elegir línea» in the sketch.
    SketchPick,
    Pick(Pick),
    CreateSimilar,
}

/// The colours of the ribbon's drawings, by family of orders, as Revit's
/// icons: so that a drawing is found by its colour as well as its shape.
mod colours {
    use eframe::egui::Color32;
    pub const SELECT: Color32 = Color32::from_rgb(60, 100, 170);
    pub const CLIPBOARD: Color32 = Color32::from_rgb(184, 117, 20);
    pub const SHAPE: Color32 = Color32::from_rgb(192, 88, 47);
    pub const MOVE: Color32 = Color32::from_rgb(47, 111, 181);
    pub const FACES: Color32 = Color32::from_rgb(110, 85, 194);
    pub const ENDS: Color32 = Color32::from_rgb(19, 133, 122);
    pub const DELETE: Color32 = Color32::from_rgb(192, 57, 43);
    pub const VIEW: Color32 = Color32::from_rgb(76, 106, 146);
    pub const MEASURE: Color32 = Color32::from_rgb(22, 137, 122);
    pub const CREATE: Color32 = Color32::from_rgb(46, 139, 87);
    pub const ACCEPT: Color32 = Color32::from_rgb(46, 125, 50);
    pub const CANCEL: Color32 = Color32::from_rgb(198, 40, 40);
}

/// The colour of a button of a fixed panel: by its order, or by its panel.
fn item_colour(panel: &str, act: &Act) -> Color32 {
    let order = match act {
        Act::Order(c) | Act::SketchCmd(c) => Some(*c),
        Act::Menu(MenuAct::Delete) => Some("BORRA"),
        _ => None,
    };
    match order {
        Some("DESPLAZA" | "COPIA" | "GIRA" | "REFLEJAR" | "ESCALA" | "MATRIZ") => colours::MOVE,
        Some("ALINEACARA" | "EQUIDIST" | "ALINEAR") => colours::FACES,
        Some(
            "ESQUINA" | "LIMITE1" | "LIMITE" | "RECORTA" | "ALARGA" | "PARTE" | "PARTEPUNTO"
            | "JUNTAR" | "EMPALME",
        ) => colours::ENDS,
        Some("BORRA") => colours::DELETE,
        _ => match panel {
            "Seleccionar" => colours::SELECT,
            "Portapapeles" => colours::CLIPBOARD,
            "Geometría" => colours::SHAPE,
            "Vista" => colours::VIEW,
            "Medir" => colours::MEASURE,
            "Crear" => colours::CREATE,
            _ => colours::MOVE,
        },
    }
}

/// The orders of the «Modificar» panel, column by column (three to a
/// column): moving, mirrored and repeated, faces, ends, lines, deleting.
const MODIFY_COLUMNS: &[&str] = &[
    "DESPLAZA",
    "COPIA",
    "GIRA",
    "REFLEJAR",
    "ESCALA",
    "MATRIZ",
    "ALINEACARA",
    "EQUIDIST",
    "ALINEAR",
    "ESQUINA",
    "LIMITE1",
    "LIMITE",
    "RECORTA",
    "ALARGA",
    "PARTE",
    "JUNTAR",
    "BORRA",
];

/// The drawing of an order of the sketch.
fn sketch_mark(command: &str) -> Mark {
    match command {
        "LINEA" => Mark::Icon(Icon::GeoSingle),
        "RECTANG" => Mark::Icon(Icon::GeoRect),
        "POLIGONO" => Mark::Icon(Icon::GeoPolygon),
        "ARCO" => Mark::Icon(Icon::GeoArc),
        "CIRCULO" => Mark::Icon(Icon::GeoCircle),
        "EMPALME" => Mark::Icon(Icon::Fillet),
        "RECORTA" => Mark::Icon(Icon::Trim),
        "ALARGA" => Mark::Icon(Icon::Extend),
        "PARTEPUNTO" => Mark::Glyph(Glyph::Break),
        "EQUIDIST" => Mark::Icon(Icon::Offset),
        "DESPLAZA" => Mark::Icon(Icon::Move),
        "COPIA" => Mark::Icon(Icon::Copy),
        "BORRA" => Mark::Glyph(Glyph::Delete),
        _ => Mark::Icon(Icon::Line),
    }
}

/// What a method button does.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) enum MethodAct {
    Wall(WallMode),
    /// Slab, roof or room outline: by corners, rectangle or wand.
    Outline(Outline),
    Dim(DimMode),
    Footing(crate::editor::FootingMode),
    MassMode(crate::editor::MassMode),
    Spline,
    PickLine,
    /// Footings under the columns and walls of the level.
    FoundLevel,
    Command(&'static str),
    /// The tool draws only like this (a beam by two points).
    Fixed,
    /// Roof tool: clicks put a dormer of this kind on a roof (none: draw
    /// roofs again).
    Dormer(archisim_core::DormerKind),
    /// Door and window tools: how they are placed along the wall.
    Place(crate::opening_tool::OpeningPlace),
    /// The Exterior brush: what it does.
    Brush(crate::brush::BrushMode),
    /// Not for this tool.
    None,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) enum Outline {
    Polygon,
    Rectangle,
    Wand,
}

struct Method {
    mark: Mark,
    name: &'static str,
    hint: &'static str,
    act: MethodAct,
    on: bool,
}

/// An order of «Modificar»: its command, drawing, name and key.
struct Order {
    command: &'static str,
    mark: Mark,
    name: &'static str,
    key: &'static str,
    hint: &'static str,
}

const TRANSFORM: &[Order] = &[
    Order {
        command: "DESPLAZA",
        mark: Mark::Icon(Icon::Move),
        name: "Mover",
        key: "D",
        hint: "Punto base y destino (arrastrar lo seleccionado también mueve)",
    },
    Order {
        command: "COPIA",
        mark: Mark::Icon(Icon::Copy),
        name: "Copiar",
        key: "CP",
        hint: "Punto base y destinos, una copia tras otra (Ctrl + arrastrar también copia)",
    },
    Order {
        command: "GIRA",
        mark: Mark::Icon(Icon::Rotate),
        name: "Girar",
        key: "GI",
        hint: "Centro y ángulo (o ángulo de referencia y final)",
    },
    Order {
        command: "REFLEJAR",
        mark: Mark::Icon(Icon::Mirror),
        name: "Reflejar",
        key: "MM",
        hint: "Reflejar: pincha una línea como eje (cara o eje de muro, eje, línea) y sale la copia reflejada; D: dibujar el eje con dos puntos (DM); C: mover en vez de copiar. Las puertas y ventanas cambian de mano",
    },
    Order {
        command: "ESCALA",
        mark: Mark::Icon(Icon::Scale),
        name: "Escala",
        key: "ES",
        hint: "Punto base y factor",
    },
    Order {
        command: "MATRIZ",
        mark: Mark::Icon(Icon::Array),
        name: "Matriz",
        key: "MA",
        hint: "Copias en filas y columnas o alrededor de un centro",
    },
];

const GEOMETRY: &[Order] = &[
    Order {
        command: "EQUIDIST",
        mark: Mark::Icon(Icon::Offset),
        name: "Desfase",
        key: "EQ",
        hint: "Paralela a una distancia",
    },
    Order {
        command: "ALINEAR",
        mark: Mark::Glyph(Glyph::Align),
        name: "Alinear",
        key: "ALI",
        hint: "Llevar lo seleccionado sobre otra línea: pares de puntos de origen y destino",
    },
    Order {
        command: "RECORTA",
        mark: Mark::Icon(Icon::Trim),
        name: "Recortar",
        key: "RR",
        hint: "Recortar contra cualquier borde (Mayús: alargar)",
    },
    Order {
        command: "ALARGA",
        mark: Mark::Icon(Icon::Extend),
        name: "Alargar",
        key: "AL",
        hint: "Alargar hasta el borde más próximo (Mayús: recortar)",
    },
    Order {
        command: "PARTE",
        mark: Mark::Glyph(Glyph::Break),
        name: "Partir",
        key: "PA",
        hint: "Partir una línea entre dos puntos (en un punto: PP)",
    },
    Order {
        command: "JUNTAR",
        mark: Mark::Glyph(Glyph::Join),
        name: "Unir",
        key: "JUN",
        hint: "Unir líneas y arcos que se tocan en una polilínea",
    },
];

/// Revit's orders for building elements, picked one by one (they need no
/// selection beforehand).
const BUILDING: &[Order] = &[
    Order {
        command: "ALINEACARA",
        mark: Mark::Glyph(Glyph::Align),
        name: "Alinear caras",
        key: "ALC",
        hint: "Alinear: la línea de referencia (cara o eje de muro, eje, línea…) y después la cara del elemento que se lleva a ella",
    },
    Order {
        command: "ESQUINA",
        mark: Mark::Icon(Icon::Trim),
        name: "Esquina",
        key: "ESQ",
        hint: "Recortar/extender para formar esquina (TR): dos muros, por la parte que se queda",
    },
    Order {
        command: "LIMITE1",
        mark: Mark::Icon(Icon::Extend),
        name: "Hasta límite",
        key: "LIM1",
        hint: "Recortar/extender un elemento: el límite (muro, eje, plano de referencia o línea) y el muro, por la parte que se queda",
    },
    Order {
        command: "LIMITE",
        mark: Mark::Icon(Icon::Extend),
        name: "Varios al límite",
        key: "LIM",
        hint: "Recortar/extender varios elementos: el límite y después todos los muros que lleguen a él (Intro para terminar)",
    },
];

/// The name of one element, for «Modificar · Muro (1)».
pub(super) fn element_name(e: &Element) -> &'static str {
    match e {
        Element::Dimension(_) => "Cota",
        Element::Mark(_) => "Cota de nivel",
        Element::Draft(_) => "Dibujo 2D",
        Element::Opening(_) => "Hueco",
        Element::Wall(_) => "Muro",
        Element::Slab(_) => "Forjado",
        Element::Column(_) => "Pilar",
        Element::Beam(_) => "Viga",
        Element::Section(_) => "Sección",
        Element::Roof(_) => "Cubierta",
        Element::Dormer(..) => "Buhardilla",
        Element::Stair(_) => "Escalera",
        Element::Terrain(_) => "Terreno",
        Element::CurtainWall(_) => "Muro cortina",
        Element::Railing(_) => "Barandilla",
        Element::Object(_) => "Objeto",
        Element::Room(_) => "Local",
        Element::Tag(_) => "Etiqueta",
        Element::Footing(_) => "Cimentación",
        Element::Grid(_) => "Eje",
        Element::RefPlane(_) => "Plano de referencia",
        Element::Module(_) => "Módulo",
        Element::Mass(_) => "Volumen",
        Element::Plot(_) => "Parcela",
        Element::Pool(_) => "Piscina",
        Element::Elevator(_) => "Ascensor",
        Element::Road(_) => "Vial",
        Element::Roundabout(_) => "Glorieta",
        Element::Marking(_) => "Marca vial",
        Element::Decal(_) => "Pegatina",
        Element::Planting(_) => "Plantación",
        Element::Run(_) => "Instalación",
        Element::Edge(_) => "Arista",
        Element::Device(_) => "Aparato",
    }
}

/// «Muro (3)», or «Selección (5)» when they are of several kinds.
pub fn selection_label<'a>(sel: impl IntoIterator<Item = &'a Element>) -> String {
    let mut n = 0;
    let mut name: Option<&str> = None;
    let mut mixed = false;
    for e in sel {
        n += 1;
        let this = element_name(e);
        match name {
            None => name = Some(this),
            Some(x) if x != this => mixed = true,
            _ => {}
        }
    }
    match (name, mixed) {
        (None, _) => String::new(),
        (Some(_), true) => format!("Selección ({n})"),
        (Some(x), false) => format!("{x} ({n})"),
    }
}

/// Whether an order of «Modificar» can work on the selection.
fn order_applies(command: &str, sel: &[Element]) -> bool {
    let drafts_only = sel.iter().all(|e| matches!(e, Element::Draft(_)));
    match command {
        // A door or window lives in its wall: it is not scaled.
        "ESCALA" => !sel.is_empty() && sel.iter().all(crate::xform::scales),
        "DESPLAZA" | "COPIA" | "GIRA" | "MATRIZ" | "ALINEAR" => !sel.is_empty(),
        // Mirrored: what has a plan to mirror (doors and windows go with
        // their wall); the rest of the phase-2 elements are not mirrored.
        "SIMETRIA" | "REFLEJAR" | "REFLEJARDIB" => {
            !sel.is_empty() && sel.iter().all(crate::xform::mirrors)
        }
        // Picked one by one, as in Revit: no selection needed.
        "ALINEACARA" | "ESQUINA" | "LIMITE1" | "LIMITE" => true,
        // The orders of the 2D geometry: on lines, arcs and polylines.
        _ => !sel.is_empty() && drafts_only,
    }
}

impl ArchiSimApp {
    /// The insertion methods of the tool in use (all drawing tools show
    /// the same row; what does not apply is shown faint).
    fn methods(&self) -> Option<(&'static str, Vec<Method>)> {
        let (name, mut list) = self.tool_methods()?;
        // With a spline or a picked line, the tool's own way is not the one
        // in use.
        if self.draw_method != super::methods_ui::DrawMethod::Normal {
            for m in &mut list {
                if !matches!(m.act, MethodAct::Spline | MethodAct::PickLine) {
                    m.on = false;
                }
            }
        }
        Some((name, list))
    }

    fn tool_methods(&self) -> Option<(&'static str, Vec<Method>)> {
        let outline = |wand: bool, mode: SlabMode| match (wand, mode) {
            (true, _) => Outline::Wand,
            (false, SlabMode::Rectangle) => Outline::Rectangle,
            (false, SlabMode::Polygon) => Outline::Polygon,
        };
        let (name, current, wand_name, wand_hint): (&str, Option<Outline>, &str, &str) =
            match self.tool {
                Tool::Slab if self.slab_settings.hole => (
                    "Abertura",
                    Some(outline(false, self.slab_tool.mode)),
                    "Varita",
                    "",
                ),
                Tool::Slab => (
                    "Forjado",
                    Some(outline(self.slab_settings.wand, self.slab_tool.mode)),
                    "Varita",
                    "Varita mágica: clic dentro de un recinto de muros (también con Espacio + clic)",
                ),
                Tool::Roof => (
                    "Cubierta",
                    Some(outline(self.roof_settings.wand, self.roof_tool.mode)),
                    "Edificio",
                    "Clic dentro del edificio: la cubierta por las caras exteriores de sus muros",
                ),
                Tool::Zone => (
                    "Local",
                    Some(outline(self.room_settings.wand, self.zone_tool.mode)),
                    "Recinto",
                    "Clic dentro de un recinto de muros: el local por las caras interiores; sigue a los muros",
                ),
                Tool::Dimension => return Some(("Cota", self.dim_methods())),
                Tool::Wall => ("Muro", None, "Varita", ""),
                Tool::Door => return Some(("Puerta", self.opening_methods())),
                Tool::Brush => return Some(("Pincel", self.brush_methods())),
                Tool::Window => return Some(("Ventana", self.opening_methods())),
                Tool::Column => ("Pilar", None, "Varita", ""),
                Tool::Beam => ("Viga", None, "Varita", ""),
                Tool::Section => ("Sección", None, "Varita", ""),
                Tool::Stair => ("Escalera", None, "Varita", ""),
                Tool::Terrain => ("Terreno", None, "Varita", ""),
                Tool::CurtainWall => ("Muro cortina", None, "Varita", ""),
                Tool::Railing => ("Barandilla", None, "Varita", ""),
                Tool::Object => ("Objeto", None, "Varita", ""),
                Tool::Grid => ("Eje", None, "Varita", ""),
                Tool::RefPlane => ("Plano de referencia", None, "Varita", ""),
                Tool::Module => ("Módulo", None, "Varita", ""),
                Tool::Mass => {
                    let title = match self.mass_settings.what {
                        crate::editor::MassWhat::Face => "Arista",
                        crate::editor::MassWhat::Volume => "Volumen",
                        crate::editor::MassWhat::Mass => "Masa",
                    };
                    return Some((title, self.mass_methods()));
                }
                Tool::PushPull => ("Empujar", None, "Varita", ""),
                Tool::DivideFace => ("Dividir cara", None, "Varita", ""),
                Tool::Deform => ("Deformar", None, "Varita", ""),
                Tool::MassOffset => ("Equidistancia", None, "Varita", ""),
                Tool::Tape => ("Cinta", None, "Varita", ""),
                Tool::Paint => ("Pintar", None, "Varita", ""),
                Tool::Plot => (
                    "Parcela",
                    Some(outline(false, self.plot_tool.mode)),
                    "Varita",
                    "",
                ),
                Tool::Pool => (
                    "Piscina",
                    Some(outline(false, self.pool_tool.mode)),
                    "Varita",
                    "",
                ),
                Tool::Road => ("Vial", None, "Varita", ""),
                Tool::Roundabout => ("Glorieta", None, "Varita", ""),
                Tool::World => ("Edificio del mundo", None, "", ""),
                Tool::Decal => ("Pegatina", None, "Varita", ""),
                Tool::Marking => ("Marca vial", None, "Varita", ""),
                Tool::Planting => ("Plantación", None, "Varita", ""),
                Tool::Pipe => ("Tubería", None, "Varita", ""),
                Tool::Duct => ("Conducto", None, "Varita", ""),
                Tool::Device => ("Aparato", None, "Varita", ""),
                Tool::Elevator => ("Ascensor", None, "Varita", ""),
                Tool::Footing => return Some(("Cimentación", self.footing_methods())),
                Tool::Arrow | Tool::Cad | Tool::TeamZone => return None,
            };
        let wall = (self.tool == Tool::Wall).then_some(self.wall_tool.mode);
        let line_tool = matches!(
            self.tool,
            Tool::Beam
                | Tool::Section
                | Tool::Stair
                | Tool::CurtainWall
                | Tool::Railing
                | Tool::Grid
                | Tool::RefPlane
        );
        let m = |mark, name, hint, act, on| Method {
            mark,
            name,
            hint,
            act,
            on,
        };
        let wall_act = |mode: WallMode| {
            if wall.is_some() {
                MethodAct::Wall(mode)
            } else {
                MethodAct::None
            }
        };
        let outline_act = |o: Outline| match current {
            Some(_) if o == Outline::Wand && self.tool == Tool::Slab && self.slab_settings.hole => {
                MethodAct::None
            }
            Some(_) if o == Outline::Wand && matches!(self.tool, Tool::Plot | Tool::Pool) => {
                MethodAct::None
            }
            Some(_) => MethodAct::Outline(o),
            None => MethodAct::None,
        };
        let straight = match wall {
            Some(_) => wall_act(WallMode::Single),
            None if line_tool => MethodAct::Fixed,
            None => MethodAct::None,
        };
        let list = vec![
            m(
                Mark::Icon(Icon::GeoSingle),
                "Recta",
                "Un tramo por dos clics (los muros, seguidos con Cadena, en la barra de opciones)",
                straight,
                matches!(wall, Some(WallMode::Single | WallMode::Chain)) || line_tool,
            ),
            m(
                Mark::Icon(Icon::GeoRect),
                "Rectángulo",
                "Dos esquinas opuestas (o la primera y su tamaño: 6x4 en metros)",
                match wall {
                    Some(_) => wall_act(WallMode::Rectangle),
                    None => outline_act(Outline::Rectangle),
                },
                wall == Some(WallMode::Rectangle) || current == Some(Outline::Rectangle),
            ),
            m(
                Mark::Icon(Icon::GeoPolygon),
                "Polígono",
                "Esquina a esquina",
                outline_act(Outline::Polygon),
                current == Some(Outline::Polygon),
            ),
            m(
                Mark::Icon(Icon::GeoArc),
                "Arco",
                "Inicio, fin y un punto por el que pasa",
                wall_act(WallMode::Arc),
                wall == Some(WallMode::Arc),
            ),
            m(
                Mark::Icon(Icon::GeoCircle),
                "Círculo",
                "Centro y un punto de la circunferencia",
                wall_act(WallMode::Circle),
                wall == Some(WallMode::Circle),
            ),
            m(
                Mark::Icon(Icon::GeoWand),
                wand_name,
                wand_hint,
                outline_act(Outline::Wand),
                current == Some(Outline::Wand),
            ),
        ];
        let mut list = list;
        list.extend(self.curve_methods());
        if self.tool == Tool::Roof {
            // Dormers and roof windows on a roof already drawn.
            let dormer = self.roof_settings.dormer;
            if dormer.is_some() {
                for m in &mut list {
                    m.on = false;
                }
            }
            let gable = match dormer {
                Some(k) if k != archisim_core::DormerKind::Skylight => k,
                _ => archisim_core::DormerKind::Gable,
            };
            list.push(m(
                Mark::Glyph(Glyph::Roof),
                "Buhardilla",
                "Clic en un faldón: la buhardilla mira aguas abajo (a dos aguas, a un agua o plana, en Propiedades)",
                MethodAct::Dormer(gable),
                dormer.is_some_and(|k| k != archisim_core::DormerKind::Skylight),
            ));
            list.push(m(
                Mark::Glyph(Glyph::Window),
                "Ventana de cubierta",
                "Clic en un faldón: una ventana en su plano",
                MethodAct::Dormer(archisim_core::DormerKind::Skylight),
                dormer == Some(archisim_core::DormerKind::Skylight),
            ));
        }
        Some((name, list))
    }

    /// Spline and Elegir línea, for every tool that takes them.
    fn curve_methods(&self) -> Vec<Method> {
        use super::methods_ui::DrawMethod;
        let m = |mark, name, hint, act, on| Method {
            mark,
            name,
            hint,
            act,
            on,
        };
        vec![
            m(
                Mark::Glyph(Glyph::Spline),
                "Spline",
                "Curva libre por puntos: muros curvos, bordes de forjados, locales, barandillas, viales",
                if self.spline_closes().is_some() {
                    MethodAct::Spline
                } else {
                    MethodAct::None
                },
                self.draw_method == DrawMethod::Spline,
            ),
            m(
                Mark::Glyph(Glyph::PickLine),
                "Elegir línea",
                "Seguir una línea, polilínea, arco o círculo del dibujo 2D o de un DXF importado",
                if self.picks_lines() {
                    MethodAct::PickLine
                } else {
                    MethodAct::None
                },
                self.draw_method == DrawMethod::PickLine,
            ),
        ]
    }

    fn mass_methods(&self) -> Vec<Method> {
        use crate::editor::MassMode;
        let mode = self.mass_settings.mode;
        let m = |mark, name, hint, act, on| Method {
            mark,
            name,
            hint,
            act,
            on,
        };
        let edges = self.mass_settings.what == crate::editor::MassWhat::Face;
        if mode.is_form() {
            return vec![
                m(
                    Mark::Glyph(Glyph::Sphere),
                    "Esfera",
                    "Centro y radio: tan alta como ancha",
                    MethodAct::MassMode(MassMode::Sphere),
                    mode == MassMode::Sphere,
                ),
                m(
                    Mark::Glyph(Glyph::Cylinder),
                    "Cilindro",
                    "Centro y radio; la altura, en Propiedades",
                    MethodAct::MassMode(MassMode::Cylinder),
                    mode == MassMode::Cylinder,
                ),
                m(
                    Mark::Glyph(Glyph::Cone),
                    "Cono",
                    "Centro y radio; la altura y el radio de arriba (tronco de cono), en Propiedades",
                    MethodAct::MassMode(MassMode::Cone),
                    mode == MassMode::Cone,
                ),
                m(
                    Mark::Glyph(Glyph::Revolution),
                    "Revolución",
                    "Clic en el eje y dibuja el medio perfil abatido sobre la planta: al lado, el radio; hacia arriba, la altura",
                    MethodAct::MassMode(MassMode::Revolution),
                    mode == MassMode::Revolution,
                ),
            ];
        }
        vec![
            m(
                Mark::Icon(if edges {
                    Icon::GeoChain
                } else {
                    Icon::GeoPolygon
                }),
                if edges { "Línea" } else { "Polígono" },
                if edges {
                    "Aristas por puntos: quedan abiertas (Intro o doble clic); clic en el primero, o cerrar con otras aristas, hace una cara"
                } else {
                    "Clic en cada esquina; clic en la primera o Intro para cerrar"
                },
                MethodAct::MassMode(MassMode::Polygon),
                mode == MassMode::Polygon,
            ),
            m(
                Mark::Icon(Icon::GeoRect),
                "Rectángulo",
                "Dos esquinas opuestas",
                MethodAct::MassMode(MassMode::Rectangle),
                mode == MassMode::Rectangle,
            ),
            m(
                Mark::Icon(Icon::GeoCircle),
                "Círculo",
                "Centro y radio",
                MethodAct::MassMode(MassMode::Circle),
                mode == MassMode::Circle,
            ),
            m(
                Mark::Icon(Icon::GeoChain),
                "Camino",
                "Seguir: un perfil a lo largo de un camino (doble clic para terminar)",
                MethodAct::MassMode(MassMode::Path),
                mode == MassMode::Path,
            ),
        ]
        .into_iter()
        .chain(self.curve_methods())
        .collect()
    }

    /// How the door and window tools place along the clear stretch of
    /// the wall under the cursor.
    fn opening_methods(&self) -> Vec<Method> {
        use crate::opening_tool::OpeningPlace as P;
        let now = self.opening_settings().place;
        let m = |icon, place: P, hint| Method {
            mark: Mark::Icon(icon),
            name: place.name(),
            hint,
            act: MethodAct::Place(place),
            on: std::mem::discriminant(&now) == std::mem::discriminant(&place),
        };
        vec![
            m(
                Icon::PlaceFree,
                P::Free,
                "Su centro donde está el cursor (al centímetro); luego, clic en una cota azul para ajustarla",
            ),
            m(
                Icon::PlaceCentre,
                P::Centre,
                "En el centro del tramo libre del muro: entre los muros que lo encuentran y las otras puertas y ventanas, en la cara hacia la que abre",
            ),
            m(
                Icon::PlaceEnd,
                P::FromEnd(self.place_gap),
                "La jamba a la distancia que digas del extremo del tramo libre más cercano al cursor",
            ),
            m(
                Icon::PlaceSpread,
                P::Spread(self.place_count),
                "Las que digas en el tramo libre, con el mismo machón entre ellas y en los extremos",
            ),
        ]
    }

    /// What the Exterior brush does.
    fn brush_methods(&self) -> Vec<Method> {
        crate::brush::BrushMode::ALL
            .iter()
            .map(|mode| Method {
                mark: Mark::Glyph(super::brush_ui::mode_glyph(*mode)),
                name: mode.name(),
                hint: mode.hint(),
                act: MethodAct::Brush(*mode),
                on: self.brush.mode == *mode,
            })
            .collect()
    }

    fn footing_methods(&self) -> Vec<Method> {
        use crate::editor::FootingMode;
        let mode = self.footing_settings.mode;
        let m = |mark, name, hint, act, on| Method {
            mark,
            name,
            hint,
            act,
            on,
        };
        vec![
            m(
                Mark::Glyph(Glyph::Footing),
                "Aislada",
                "Zapata aislada: clic en un pilar (lo sigue) o en su centro",
                MethodAct::Footing(FootingMode::Pad),
                mode == FootingMode::Pad,
            ),
            m(
                Mark::Icon(Icon::Wall),
                "Corrida",
                "Zapata corrida: clic en un muro (lo sigue) o dos puntos",
                MethodAct::Footing(FootingMode::Strip),
                mode == FootingMode::Strip,
            ),
            m(
                Mark::Icon(Icon::GeoSingle),
                "Viga de atado",
                "Entre dos puntos; encadena la siguiente",
                MethodAct::Footing(FootingMode::TieBeam),
                mode == FootingMode::TieBeam,
            ),
            m(
                Mark::Glyph(Glyph::Raft),
                "Cimentar planta",
                "Zapatas bajo todos los pilares y muros rectos de la planta que no la tengan",
                MethodAct::FoundLevel,
                false,
            ),
        ]
        .into_iter()
        .chain(self.curve_methods())
        .collect()
    }

    fn dim_methods(&self) -> Vec<Method> {
        let mode = self.dim_settings.mode;
        let m = |mark, name, hint, act, on| Method {
            mark,
            name,
            hint,
            act,
            on,
        };
        vec![
            m(
                Mark::Glyph(Glyph::Dimension),
                "Lineal",
                "Clic en cada punto, Intro, y clic donde va la línea de cota",
                MethodAct::Dim(DimMode::Linear),
                mode == DimMode::Linear,
            ),
            m(
                Mark::Icon(Icon::Wall),
                "Huecos de muro",
                "Clic en un muro: acota sus extremos y las jambas de sus huecos",
                MethodAct::Dim(DimMode::WallOpenings),
                mode == DimMode::WallOpenings,
            ),
            m(
                Mark::Glyph(Glyph::LevelMark),
                "Cota de nivel",
                "Cota de nivel del suelo terminado",
                MethodAct::Dim(DimMode::LevelMark),
                mode == DimMode::LevelMark,
            ),
            m(
                Mark::Icon(Icon::DimAngle),
                "Angular",
                "Ángulo entre dos líneas o de un arco (DAN)",
                MethodAct::Command("ACOANGULO"),
                false,
            ),
            m(
                Mark::Icon(Icon::DimRadius),
                "Radio",
                "Radio de un arco o círculo (DRA; diámetro: DDI)",
                MethodAct::Command("ACORADIO"),
                false,
            ),
            m(
                Mark::Icon(Icon::DimArc),
                "Long. arco",
                "Longitud de un arco (DAR)",
                MethodAct::Command("ACOARCO"),
                false,
            ),
        ]
    }

    /// The methods offered now: name, whether it applies and whether it
    /// is the one in use.
    #[cfg(test)]
    pub(super) fn method_states(&self) -> Vec<(&'static str, bool, bool)> {
        self.methods()
            .map(|(_, list)| {
                list.iter()
                    .map(|m| (m.name, m.act != MethodAct::None, m.on))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Chooses a method by its name, as a click on it.
    #[cfg(test)]
    pub(super) fn choose_method(&mut self, name: &str) {
        let act = self
            .methods()
            .and_then(|(_, list)| list.into_iter().find(|m| m.name == name))
            .map(|m| m.act);
        if let Some(act) = act {
            self.apply_method(act);
        }
    }

    fn apply_method(&mut self, act: MethodAct) {
        let set_outline = |wand: &mut bool, tool: &mut crate::slab_tool::SlabTool, o: Outline| {
            *wand = o == Outline::Wand;
            match o {
                Outline::Polygon => tool.set_mode(SlabMode::Polygon),
                Outline::Rectangle => tool.set_mode(SlabMode::Rectangle),
                Outline::Wand => tool.finish(),
            }
        };
        match act {
            // A straight wall goes on in a chain if Cadena is on.
            MethodAct::Wall(WallMode::Single | WallMode::Chain) => {
                self.wall_tool.set_mode(if self.wall_chain {
                    WallMode::Chain
                } else {
                    WallMode::Single
                })
            }
            MethodAct::Wall(mode) => self.wall_tool.set_mode(mode),
            MethodAct::Dormer(k) => {
                self.roof_settings.dormer = Some(k);
                self.roof_tool.finish();
            }
            MethodAct::Outline(o) => match self.tool {
                Tool::Slab => set_outline(&mut self.slab_settings.wand, &mut self.slab_tool, o),
                Tool::Roof => {
                    self.roof_settings.dormer = None;
                    set_outline(&mut self.roof_settings.wand, &mut self.roof_tool, o)
                }
                Tool::Zone => set_outline(&mut self.room_settings.wand, &mut self.zone_tool, o),
                Tool::Plot => {
                    let mut wand = false;
                    set_outline(&mut wand, &mut self.plot_tool, o)
                }
                Tool::Pool => {
                    let mut wand = false;
                    set_outline(&mut wand, &mut self.pool_tool, o)
                }
                _ => {}
            },
            MethodAct::Dim(mode) => {
                if self.dim_settings.mode != mode {
                    self.dim_settings.mode = mode;
                    self.dim_tool.finish();
                }
            }
            MethodAct::Footing(mode) => self.set_footing_mode(mode),
            MethodAct::Place(place) => self.opening_settings_mut().place = place,
            MethodAct::Brush(mode) => self.set_brush_mode(mode),
            MethodAct::MassMode(mode) => self.set_mass_mode(mode),
            MethodAct::Spline => self.set_draw_method(super::methods_ui::DrawMethod::Spline),
            MethodAct::PickLine => self.set_draw_method(super::methods_ui::DrawMethod::PickLine),
            MethodAct::FoundLevel => self.found_level(),
            MethodAct::Command(name) => self.start_command(name),
            MethodAct::Fixed | MethodAct::None => {}
        }
    }

    /// Which line of a wall or road the clicks draw: exterior face, axis
    /// or interior face (left edge, axis or right edge of a road), as
    /// three drawings of the wall seen in plan with the line marked.
    /// The settings of the door or window tool in use (the door's
    /// otherwise).
    pub(super) fn opening_settings(&self) -> crate::opening_tool::OpeningSettings {
        if self.tool == Tool::Window {
            self.window_settings
        } else {
            self.door_settings
        }
    }

    pub(super) fn opening_settings_mut(&mut self) -> &mut crate::opening_tool::OpeningSettings {
        if self.tool == Tool::Window {
            &mut self.window_settings
        } else {
            &mut self.door_settings
        }
    }

    /// «Desde un extremo»: the distance; «Repartidas»: how many.
    fn place_fields(&mut self, ui: &mut Ui) {
        use crate::opening_tool::OpeningPlace as P;
        if !matches!(self.tool, Tool::Door | Tool::Window) {
            return;
        }
        let place = self.opening_settings().place;
        match place {
            P::FromEnd(d) => {
                ui.add_space(8.0);
                ui.label(egui::RichText::new("Distancia").small());
                let typed = super::text_edit_committed(
                    ui,
                    egui::Id::unique("bar-place-gap"),
                    &crate::units::fmt_len(d),
                    64.0,
                )
                .and_then(|t| crate::units::parse_length(&t));
                if let Some(d) = typed.filter(|d| *d >= 0.0) {
                    self.place_gap = d;
                    self.opening_settings_mut().place = P::FromEnd(d);
                }
            }
            P::Spread(n) => {
                ui.add_space(8.0);
                ui.label(egui::RichText::new("Cuántas").small());
                let mut k = n as u32;
                if ui
                    .add(egui::DragValue::new(&mut k).range(1..=20))
                    .on_hover_text("Cuántas en el tramo libre, con machones iguales")
                    .changed()
                {
                    self.place_count = k as usize;
                    self.opening_settings_mut().place = P::Spread(k as usize);
                }
            }
            _ => {}
        }
    }

    fn reference_buttons(&mut self, ui: &mut Ui) {
        use super::urban_ui::RoadReference;
        use archisim_core::ReferenceLine;
        let p = Palette::current(ui.ctx());
        // 0: the line on the left side (up in the drawing), 1: axis, 2:
        // the right side.
        let (current, hints): (usize, [&str; 3]) = match self.tool {
            Tool::Wall => (
                match self.wall_settings.reference {
                    ReferenceLine::ExteriorFace => 0,
                    ReferenceLine::Center => 1,
                    ReferenceLine::InteriorFace => 2,
                },
                [
                    "Cara exterior: los clics van por la cara exterior (a la izquierda según dibujas; dibuja el contorno en el sentido de las agujas del reloj)",
                    "Eje: los clics van por el centro del muro",
                    "Cara interior: los clics van por la cara interior (a la derecha según dibujas)",
                ],
            ),
            Tool::Road => (
                match self.road_settings.reference {
                    RoadReference::LeftEdge => 0,
                    RoadReference::Axis => 1,
                    RoadReference::RightEdge => 2,
                },
                [
                    "Borde izquierdo: los clics van por el borde exterior de la acera de la izquierda (según dibujas); el vial queda a la derecha",
                    "Eje: los clics van por el eje del vial",
                    "Borde derecho: los clics van por el borde exterior de la acera de la derecha; el vial queda a la izquierda",
                ],
            ),
            _ => return,
        };
        bar_separator(ui, &p);
        if ui.available_width() > 3.0 * 34.0 + 200.0 {
            ui.label(egui::RichText::new("Referencia").small().color(p.muted));
            ui.add_space(4.0);
        }
        let mut chosen = None;
        for (i, hint) in hints.iter().enumerate() {
            let on = i == current;
            let (rect, resp) = ui.allocate_exact_size(Vec2::new(32.0, 28.0), Sense::click());
            let (fill, fg) = if on {
                (p.accent_soft, p.accent)
            } else if resp.hovered() {
                (p.raised, p.text)
            } else {
                (Color32::TRANSPARENT, p.text)
            };
            ui.painter().rect_filled(rect, CornerRadius::same(7), fill);
            // The wall (or road) as a band, the line clicked along it.
            let band = Rect::from_center_size(rect.center(), Vec2::new(20.0, 10.0));
            ui.painter().rect_stroke(
                band,
                CornerRadius::ZERO,
                Stroke::new(1.0, fg.gamma_multiply(0.7)),
                egui::StrokeKind::Inside,
            );
            let y = match i {
                0 => band.top() + 0.5,
                1 => band.center().y,
                _ => band.bottom() - 0.5,
            };
            ui.painter().line_segment(
                [
                    egui::pos2(band.left() - 3.0, y),
                    egui::pos2(band.right() + 3.0, y),
                ],
                Stroke::new(2.0, p.accent),
            );
            super::missions_ui::mark(ui.ctx(), format!("reference:{i}"), rect);
            if resp.on_hover_text(*hint).clicked() {
                chosen = Some(i);
            }
        }
        if let Some(i) = chosen {
            match self.tool {
                Tool::Wall => {
                    self.wall_settings.reference = [
                        ReferenceLine::ExteriorFace,
                        ReferenceLine::Center,
                        ReferenceLine::InteriorFace,
                    ][i];
                }
                Tool::Road => {
                    self.road_settings.reference = RoadReference::ALL[i];
                    // By an edge, the street is drawn whole (its stretches
                    // go by the axis).
                    if RoadReference::ALL[i] != RoadReference::Axis {
                        self.road_settings.draw = super::urban_ui::RoadDraw::Polyline;
                    }
                }
                _ => {}
            }
        }
    }

    /// The ribbon under the tabs (`docs/INTERFAZ.md` §13).
    pub(super) fn ribbon(&mut self, ui: &mut Ui) {
        let panels = self.ribbon_panels();
        let hue = self.discipline.hue(ui.visuals().dark_mode);
        let compact = self.prefs.ribbon_compact;
        if let Some(act) = super::ribbon::show(ui, &panels, compact, hue, 0.0) {
            self.ribbon_act(act);
        }
    }

    /// The order running, if it is one of «Modificar».
    fn running(&self) -> Option<&'static str> {
        if self.tool != Tool::Cad {
            return None;
        }
        self.cad.as_ref().map(|c| c.name())
    }

    /// What the ribbon holds now: the panels of «Modificar», always the
    /// same and in the same order, and after them those of what is in
    /// hand.
    pub(super) fn ribbon_panels(&self) -> Vec<Panel<Act>> {
        let sel: Vec<Element> = self.ed.selection.iter().copied().collect();
        let some = !sel.is_empty();
        let running = self.running();
        let mut panels = Vec::new();

        // Seleccionar (the arrow itself is at the top of the rail, on the
        // far left).
        panels.push(Panel::new(
            "Seleccionar",
            7,
            vec![
                Item::new(
                    Mark::Glyph(Glyph::SelectSame),
                    "Los de su tipo",
                    Size::Named,
                    Act::SameType,
                )
                .enabled(sel.len() == 1)
                .hint(if sel.len() == 1 {
                    "Elige todos los ejemplares del proyecto con el tipo de lo seleccionado"
                } else {
                    "Elige antes un elemento: luego, todos los de su tipo"
                }),
                Item::new(
                    Mark::Glyph(Glyph::SelectAll),
                    "Todo",
                    Size::Named,
                    Act::Menu(MenuAct::SelectAll),
                )
                .hint("Todo lo de la vista (Ctrl+A)"),
            ],
        ));

        // Portapapeles.
        let pick_first = "Elige antes lo que quieres";
        panels.push(Panel::new(
            "Portapapeles",
            4,
            vec![
                Item::new(Mark::Glyph(Glyph::Paste), "Pegar", Size::Big, Act::Menu(MenuAct::Paste))
                    .enabled(self.clipboard.is_some())
                    .hint(if self.clipboard.is_some() {
                        "Pegar (Ctrl+V): lo copiado, a medio metro; con el clic derecho, donde lo pinches"
                    } else {
                        "Pegar (Ctrl+V): copia o corta antes algo"
                    }),
                Item::new(Mark::Glyph(Glyph::Cut), "Cortar", Size::Small, Act::Menu(MenuAct::Cut))
                    .enabled(some)
                    .hint(if some { "Ctrl+X: para pegarlo en otro sitio o en otra planta".to_string() } else { format!("{pick_first} cortar") }),
                Item::new(Mark::Glyph(Glyph::ClipCopy), "Copiar al portapapeles", Size::Small, Act::Menu(MenuAct::Copy))
                    .enabled(some)
                    .hint(if some { "Ctrl+C: para pegarlo en otro sitio o en otra planta".to_string() } else { format!("{pick_first} copiar") }),
                Item::new(Mark::Glyph(Glyph::MatchProps), "Igualar propiedades", Size::Small, Act::Order("IGUALARPROP"))
                    .on(running == Some("IGUALARPROP"))
                    .hint("Igualar propiedades (MA): pincha el modelo y después los elementos que toman su tipo y sus propiedades"),
            ],
        ));

        // Geometría.
        let masses = sel.iter().any(|e| matches!(e, Element::Mass(_)));
        panels.push(Panel::new(
            "Geometría",
            5,
            vec![
                Item::new(Mark::Glyph(Glyph::Paint), "Pintar", Size::Named, Act::Paint)
                    .on(self.tool == Tool::Paint && !self.divide_face_pick)
                    .hint("Pintar (PT): un material en la cara que pinches de cualquier elemento (mejor en 3D). Si la cara está dividida, solo esa zona"),
                Item::new(Mark::Glyph(Glyph::DivideFace), "Dividir cara", Size::Named, Act::DivideFace3d)
                    .on(self.divide_face_pick)
                    .hint("Dividir cara (SF): pincha en 3D la cara y dibuja sus zonas en modo boceto; con un elemento elegido, sus caras están en su grupo, a la derecha"),
                Item::new(Mark::Glyph(Glyph::RoundEdge), "Redondear", Size::Named, Act::Round)
                    .enabled(masses)
                    .hint(if masses {
                        "Redondear las esquinas y las aristas de arriba de los volúmenes (otra vez: aristas vivas)"
                    } else {
                        "Redondear: elige antes uno o varios volúmenes de Concepto"
                    }),
            ],
        ));

        // Modificar: in a sketch, the orders of its lines.
        let mut orders = Vec::new();
        if self.sketch.is_some() {
            for (label, cmd, hint) in super::sketch_ui::SKETCH_EDITS {
                orders.push(
                    Item::new(sketch_mark(cmd), *label, Size::Small, Act::SketchCmd(cmd))
                        .on(running == Some(*cmd))
                        .hint(*hint),
                );
            }
        } else {
            for command in MODIFY_COLUMNS {
                if *command == "BORRA" {
                    orders.push(
                        Item::new(
                            Mark::Glyph(Glyph::Delete),
                            "Borrar",
                            Size::Small,
                            Act::Menu(MenuAct::Delete),
                        )
                        .enabled(some)
                        .hint(if some {
                            "Borrar lo seleccionado (Supr, DE)"
                        } else {
                            "Borrar (Supr, DE): elige antes lo que quieres borrar"
                        }),
                    );
                    continue;
                }
                let Some(o) = TRANSFORM
                    .iter()
                    .chain(GEOMETRY)
                    .chain(BUILDING)
                    .find(|o| o.command == *command)
                else {
                    continue;
                };
                let enabled = order_applies(o.command, &sel);
                let hint = if enabled {
                    format!("({}) {}", o.key, o.hint)
                } else if sel.is_empty() {
                    format!("({}) Elige antes lo que quieres modificar", o.key)
                } else {
                    format!("({}) No vale para lo seleccionado", o.key)
                };
                orders.push(
                    Item::new(o.mark, o.name, Size::Small, Act::Order(o.command))
                        .on(running == Some(o.command))
                        .enabled(enabled || running == Some(o.command))
                        .hint(hint),
                );
            }
        }
        panels.push(Panel::new("Modificar", 8, orders));

        // Vista.
        let hidden = !self.ed.hidden().is_empty();
        panels.push(Panel::new(
            "Vista",
            3,
            vec![
                Item::new(
                    Mark::Glyph(Glyph::Hide),
                    "Ocultar",
                    Size::Named,
                    Act::Menu(MenuAct::Hide),
                )
                .enabled(some)
                .hint("Ocultar lo seleccionado en las vistas, por ahora (HH)"),
                Item::new(
                    Mark::Glyph(Glyph::Isolate),
                    "Aislar",
                    Size::Named,
                    Act::Menu(MenuAct::Isolate),
                )
                .enabled(some)
                .hint("Ver solo lo seleccionado, por ahora"),
                Item::new(
                    Mark::Glyph(Glyph::ShowAll),
                    "Mostrar todo",
                    Size::Named,
                    Act::Menu(MenuAct::ShowAll),
                )
                .enabled(hidden)
                .hint(if hidden {
                    "Volver a ver lo ocultado (HR)"
                } else {
                    "No hay nada oculto"
                }),
            ],
        ));

        // Medir.
        panels.push(Panel::new(
            "Medir",
            2,
            vec![
                Item::new(
                    Mark::Glyph(Glyph::Tape),
                    "Cinta",
                    Size::Named,
                    Act::Pick(Pick::Concept(super::concept_ui::ConceptPick::Tape)),
                )
                .on(self.tool == Tool::Tape)
                .hint("Medir entre dos puntos; deja una guía"),
                Item::new(
                    Mark::Glyph(Glyph::Dimension),
                    "Cota",
                    Size::Named,
                    Act::Pick(Pick::Tool(Tool::Dimension)),
                )
                .on(self.tool == Tool::Dimension)
                .hint("Cota alineada (DI)"),
            ],
        ));

        // Crear.
        let similar = self.similar_tool(&sel);
        panels.push(Panel::new(
            "Crear",
            1,
            vec![Item::new(Mark::Glyph(Glyph::CreateSimilar), "Crear similar", Size::Big, Act::CreateSimilar)
                .enabled(similar.is_some())
                .hint(match similar {
                    Some((_, name)) => format!("Crear similar (CS): la herramienta {name} con el tipo de lo seleccionado"),
                    None => "Crear similar (CS): elige antes un muro, forjado, puerta, ventana, pilar, viga, cubierta, barandilla, muro cortina u objeto".to_string(),
                })],
        ));

        let mut contextual = self.contextual_panels(&sel);
        let own = self.discipline.hue(false).main;
        for p in &mut contextual {
            for it in &mut p.items {
                it.tint.get_or_insert(own);
            }
        }
        panels.extend(contextual);
        for p in &mut panels {
            for it in &mut p.items {
                if it.tint.is_none() {
                    it.tint = Some(item_colour(&p.title, &it.act));
                }
            }
        }
        panels
    }

    /// The panels of what is in hand: the sketch, the drawing order, the
    /// tool or the selection.
    fn contextual_panels(&self, sel: &[Element]) -> Vec<Panel<Act>> {
        let running = self.running();
        if self.sketch.is_some() {
            let accept = Item::new(
                Mark::Glyph(Glyph::Accept),
                "Aceptar",
                Size::Big,
                Act::SketchAccept,
            )
            .tint(colours::ACCEPT)
            .hint("Terminar el modo boceto (Intro): el elemento toma lo dibujado");
            let cancel = Item::new(
                Mark::Glyph(Glyph::Cancel),
                "Cancelar",
                Size::Big,
                Act::SketchCancel,
            )
            .tint(colours::CANCEL)
            .hint("Salir del modo boceto dejando el elemento como estaba");
            let picking = self.sketch.as_ref().is_some_and(|st| st.pick);
            let mut draw: Vec<Item<Act>> = super::sketch_ui::SKETCH_TOOLS
                .iter()
                .map(|(label, cmd, hint)| {
                    Item::new(sketch_mark(cmd), *label, Size::Small, Act::SketchCmd(cmd))
                        .on(running == Some(*cmd))
                        .hint(*hint)
                })
                .collect();
            // Revit's «Seleccionar líneas»: an existing line or edge,
            // copied as a sketch line.
            draw.push(
                Item::new(Mark::Glyph(Glyph::PickLine), "Elegir línea", Size::Small, Act::SketchPick)
                    .on(picking)
                    .hint("Clic en una línea del dibujo o en una cara o borde de muro, forjado, pilar, eje o plano de referencia: se copia como línea del boceto. Otra vez: dejar de elegir")
                    .mission("sketch:Elegir línea"),
            );
            return vec![
                Panel::new("Modo", 10, vec![accept, cancel]).contextual(),
                Panel::new("Dibujar", 9, draw).contextual(),
            ];
        }
        if self.tool == Tool::Cad {
            let name = running.unwrap_or_default();
            if let Some((title, variants)) = super::bottom_segments::segment_of(name) {
                let items = variants
                    .iter()
                    .map(|v| {
                        let on = matches!(v.pick, Pick::Command(c) if c == name);
                        Item::new(v.mark, v.name, Size::Named, Act::Pick(v.pick))
                            .on(on)
                            .hint(format!("{} ({})", v.name, v.key))
                    })
                    .collect();
                return vec![Panel::new(title, 9, items).contextual()];
            }
            return Vec::new();
        }
        if let Some((name, methods)) = self.methods() {
            // How the methods of this tool are shown: drawings of shapes in
            // a gallery, or ways of placing with their names.
            let (title, size) = match self.tool {
                Tool::Door | Tool::Window => ("Colocación", Size::Named),
                Tool::Brush => ("Pincel", Size::Named),
                Tool::Dimension => ("Cota", Size::Named),
                Tool::Footing => ("Cimentación", Size::Named),
                Tool::Mass if self.mass_settings.mode.is_form() => ("Formas", Size::Named),
                _ => ("Dibujar", Size::Small),
            };
            let mut main = Vec::new();
            let mut dormers = Vec::new();
            for m in methods {
                let enabled = m.act != MethodAct::None;
                let hint = if enabled {
                    m.hint.to_string()
                } else {
                    "No vale para esta herramienta".to_string()
                };
                let item = Item::new(m.mark, m.name, size, Act::Method(m.act))
                    .on(m.on)
                    .enabled(enabled)
                    .hint(hint)
                    .mission(format!("method:{}", m.name));
                if matches!(m.act, MethodAct::Dormer(_)) {
                    dormers.push(Item {
                        size: Size::Named,
                        ..item
                    });
                } else {
                    main.push(item);
                }
            }
            let _ = name;
            let mut out = vec![Panel::new(title, 9, main).contextual()];
            if !dormers.is_empty() {
                out.push(Panel::new("Buhardillas", 9, dormers).contextual());
            }
            return out;
        }
        if sel.is_empty() {
            return Vec::new();
        }
        // The selection: its contour and its faces.
        let mut items = Vec::new();
        let contour = match sel {
            [Element::Slab(id)] => {
                Some((crate::sketch::SketchTarget::Slab(*id), "Editar contorno"))
            }
            [Element::Roof(id)] => {
                Some((crate::sketch::SketchTarget::Roof(*id), "Editar contorno"))
            }
            [Element::Terrain(id)] => {
                Some((crate::sketch::SketchTarget::Terrain(*id), "Editar contorno"))
            }
            [Element::Road(id)] => {
                Some((crate::sketch::SketchTarget::RoadEdges(*id), "Editar bordes"))
            }
            // A wall: its profile in elevation (Revit's «Editar perfil»).
            [Element::Wall(id)] if self.ed.model().wall(*id).is_some_and(|w| !w.is_curved()) => {
                Some((
                    crate::sketch::SketchTarget::WallProfile(*id),
                    "Editar perfil",
                ))
            }
            _ => None,
        };
        if let Some((target, label)) = contour {
            items.push(
                Item::new(Mark::Glyph(Glyph::EditContour), label, Size::Big, Act::Sketch(target))
                    .hint(if label == "Editar perfil" {
                        "Modo boceto: el muro en alzado, con sus puertas y ventanas en gris; dibuja su contorno y sus huecos"
                    } else {
                        "Modo boceto: su contorno en líneas sueltas para dibujar, recortar, alargar o borrar; Aceptar o Cancelar"
                    })
                    .mission("bar:Editar contorno"),
            );
        }
        use crate::sketch::SketchTarget as T;
        use archisim_core::{FaceRef, FaceSide};
        let divide = |items: &mut Vec<Item<Act>>, name: &str, t: T, hint: &str| {
            items.push(
                Item::new(
                    Mark::Glyph(Glyph::DivideFace),
                    name,
                    Size::Named,
                    Act::Sketch(t),
                )
                .hint(hint),
            );
        };
        match sel {
            [Element::Wall(w)] => {
                divide(&mut items, "Dividir cara exterior", T::WallFace(*w, FaceSide::Exterior), "Zonas en la cara exterior, en alzado vista desde fuera; luego se pinta cada una");
                divide(
                    &mut items,
                    "Dividir cara interior",
                    T::WallFace(*w, FaceSide::Interior),
                    "Zonas en la cara interior, en alzado; luego se pinta cada una",
                );
            }
            [e @ Element::Roof(_)] => {
                divide(&mut items, "Dividir intradós", T::Zones(*e, FaceRef::Bottom), "Zonas en la cara de abajo de la cubierta; los faldones y frentes, con Dividir cara en 3D");
            }
            [e
            @ (Element::Slab(_) | Element::Footing(_) | Element::Column(_) | Element::Beam(_))] => {
                divide(
                    &mut items,
                    "Dividir cara de arriba",
                    T::Zones(*e, FaceRef::Top),
                    "Zonas en la cara de arriba, en modo boceto; luego se pinta cada una",
                );
                divide(
                    &mut items,
                    "Dividir cara de abajo",
                    T::Zones(*e, FaceRef::Bottom),
                    "Zonas en la cara de abajo; luego se pinta cada una",
                );
            }
            _ => {}
        }
        if sel.iter().all(|e| matches!(e, Element::Draft(_))) {
            items.push(
                Item::new(
                    Mark::Glyph(Glyph::Join),
                    "Agrupar",
                    Size::Named,
                    Act::Menu(MenuAct::Group),
                )
                .enabled(sel.len() > 1)
                .hint("Un grupo: un clic lo elige entero"),
            );
            items.push(
                Item::new(
                    Mark::Glyph(Glyph::Break),
                    "Desagrupar",
                    Size::Named,
                    Act::Menu(MenuAct::Ungroup),
                )
                .hint("Cada línea del grupo, por su cuenta"),
            );
        }
        if items.is_empty() {
            return Vec::new();
        }
        let title = match selection_label(sel) {
            l if l.starts_with("Selección") => "Selección".to_string(),
            l => l.split(" (").next().unwrap_or("").to_string(),
        };
        vec![Panel::new(title, 9, items).contextual()]
    }

    /// The tool and the type «Crear similar» would take from the
    /// selection.
    pub(super) fn similar_tool(&self, sel: &[Element]) -> Option<(Tool, &'static str)> {
        let m = self.ed.model();
        let [e] = sel else {
            return None;
        };
        match *e {
            Element::Wall(id) => m.wall(id).map(|_| (Tool::Wall, "Muro")),
            Element::Slab(id) => m.slab(id).map(|_| (Tool::Slab, "Forjado")),
            Element::Roof(id) => m.roof(id).map(|_| (Tool::Roof, "Cubierta")),
            Element::Column(id) => m.column(id).map(|_| (Tool::Column, "Pilar")),
            Element::Beam(id) => m.beam(id).map(|_| (Tool::Beam, "Viga")),
            Element::Object(id) => m.object(id).map(|_| (Tool::Object, "Objeto")),
            Element::Railing(id) => m.railing(id).map(|_| (Tool::Railing, "Barandilla")),
            Element::CurtainWall(id) => m
                .curtain_wall(id)
                .map(|_| (Tool::CurtainWall, "Muro cortina")),
            Element::Opening(id) => {
                let o = m.opening(id)?;
                let t = m.opening_type(o.opening_type)?;
                if t.is_door() {
                    Some((Tool::Door, "Puerta"))
                } else if t.is_window() {
                    Some((Tool::Window, "Ventana"))
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    /// «Crear similar» (Revit's CS): the tool of the element selected,
    /// with its type.
    pub(super) fn create_similar(&mut self) {
        let sel: Vec<Element> = self.ed.selection.iter().copied().collect();
        let Some((tool, name)) = self.similar_tool(&sel) else {
            self.complain("Crear similar: elige antes un solo elemento con tipo (muro, forjado, puerta, ventana, pilar, viga, cubierta, barandilla, muro cortina u objeto).");
            return;
        };
        let e = sel[0];
        self.pick(Pick::Tool(tool));
        let m = self.ed.model();
        match e {
            Element::Wall(id) => {
                if let Some(w) = m.wall(id) {
                    self.wall_settings.wall_type = w.wall_type;
                    self.wall_settings.reference = w.reference;
                }
            }
            Element::Slab(id) => {
                if let Some(s) = m.slab(id) {
                    self.slab_settings.slab_type = s.slab_type;
                }
            }
            Element::Roof(id) => {
                if let Some(r) = m.roof(id) {
                    self.roof_settings.roof_type = r.roof_type;
                }
            }
            Element::Column(id) => {
                if let Some(c) = m.column(id) {
                    self.column_settings.member_type = c.member_type;
                }
            }
            Element::Beam(id) => {
                if let Some(b) = m.beam(id) {
                    self.beam_settings.member_type = b.member_type;
                }
            }
            Element::Object(id) => {
                if let Some(o) = m.object(id) {
                    self.object_settings.object_type = Some(o.object_type);
                }
            }
            Element::Railing(id) => {
                if let Some(r) = m.railing(id) {
                    self.railing_settings.railing_type = r.railing_type;
                }
            }
            Element::CurtainWall(id) => {
                if let Some(c) = m.curtain_wall(id) {
                    self.curtain_settings.curtain_type = Some(c.curtain_type);
                }
            }
            Element::Opening(id) => {
                if let Some(o) = m.opening(id) {
                    let t = o.opening_type;
                    self.opening_settings_mut().opening_type = t;
                }
            }
            _ => {}
        }
        self.say(format!("{name} con el tipo de lo seleccionado."));
    }

    /// Does what was clicked in the ribbon.
    fn ribbon_act(&mut self, act: Act) {
        match act {
            Act::Method(m) => self.apply_method(m),
            Act::Order(c) => {
                if self.running() == Some(c) {
                    self.cad_escape();
                } else {
                    self.start_command(c);
                }
            }
            Act::SameType => {
                if let Some(e) = self.ed.selection.iter().next().copied() {
                    let found = self.same_type(e);
                    let what = super::context_menu::plural(element_name(&e));
                    self.select_instances(found, &what);
                }
            }
            Act::Menu(m) => {
                let menu = super::context_menu::ContextMenu::bare();
                self.menu_act(m, &menu);
            }
            Act::Paint => {
                self.divide_face_pick = false;
                self.concept_pick(super::concept_ui::ConceptPick::Paint);
            }
            Act::DivideFace3d => {
                self.concept_pick(super::concept_ui::ConceptPick::Paint);
                self.divide_face_pick = true;
                if self.mode != super::ViewMode::ThreeD {
                    self.open_view(super::tabs::TabKind::ThreeD);
                }
                self.say("Pincha en 3D la cara que quieres dividir.");
            }
            Act::Round => self.round_selected_edges(),
            Act::Sketch(t) => self.start_sketch(t),
            Act::SketchCmd(cmd) => self.sketch_command(cmd),
            Act::SketchAccept => self.finish_sketch(),
            Act::SketchPick => self.toggle_sketch_pick(),
            Act::SketchCancel => self.cancel_sketch(),
            Act::Pick(p) => self.pick(p),
            Act::CreateSimilar => self.create_similar(),
        }
    }

    /// Clicks an item of the ribbon by its name, as the mouse would (for
    /// the tests).
    #[cfg(test)]
    pub(super) fn click_ribbon(&mut self, name: &str) -> bool {
        let found = self
            .ribbon_panels()
            .into_iter()
            .flat_map(|p| p.items)
            .find(|it| it.name == name && it.enabled);
        match found {
            Some(it) => {
                self.ribbon_act(it.act);
                true
            }
            None => false,
        }
    }

    /// The bar under the ribbon (Revit's options bar): what is in hand,
    /// its settings and what a click does now.
    pub(super) fn options_bar(&mut self, ui: &mut Ui) {
        let p = Palette::current(ui.ctx());
        // The button that folds the ribbon, at the right.
        let fold = Rect::from_center_size(
            ui.max_rect().right_center() - Vec2::new(14.0, 0.0),
            Vec2::new(26.0, 22.0),
        );
        // What does not fit is cut before the button, not under it.
        let inner = Rect::from_min_max(
            ui.max_rect().min,
            egui::pos2(fold.left() - 6.0, ui.max_rect().bottom()),
        );
        ui.scope_builder(egui::UiBuilder::new().max_rect(inner), |ui| {
            ui.set_clip_rect(inner.intersect(ui.clip_rect()));
            ui.horizontal_centered(|ui| {
                ui.spacing_mut().item_spacing.x = 4.0;
                if self.tool == Tool::Cad {
                    if self.cad_prompt().is_none() {
                        return;
                    }
                    let name = self.running().unwrap_or_default();
                    let title = match TRANSFORM
                        .iter()
                        .chain(GEOMETRY)
                        .chain(BUILDING)
                        .find(|o| o.command == name)
                    {
                        Some(o) => o.name.to_string(),
                        None => super::toolbox::order_name(name).unwrap_or(name).to_string(),
                    };
                    bar_title(ui, &format!("Modificar | {title}"), &p);
                    self.order_step_body(ui);
                    return;
                }
                if let Some((what, note)) = self.sketch_texts() {
                    if self.sketch_picking() {
                        ui.add_space(6.0);
                        ui.label(
                            egui::RichText::new(format!("Modo boceto · {what}"))
                                .font(strong_font(12.5))
                                .color(super::sketch_ui::SKETCH),
                        );
                        bar_separator(ui, &p);
                        self.pick_fields(ui);
                        return;
                    }
                    ui.add_space(6.0);
                    ui.label(
                        egui::RichText::new(format!("Modo boceto · {what}"))
                            .font(strong_font(12.5))
                            .color(super::sketch_ui::SKETCH),
                    );
                    ui.add_space(8.0);
                    ui.add(
                        egui::Label::new(
                            egui::RichText::new(note)
                                .font(FontId::proportional(12.0))
                                .color(p.muted),
                        )
                        .truncate(),
                    )
                    .on_hover_text(note);
                    return;
                }
                if let Some((name, _)) = self.methods() {
                    let verb = match self.tool {
                        Tool::Brush
                        | Tool::Paint
                        | Tool::Tape
                        | Tool::PushPull
                        | Tool::DivideFace
                        | Tool::Deform
                        | Tool::MassOffset => "",
                        _ => "Colocar ",
                    };
                    bar_title(ui, &format!("Modificar | {verb}{name}"), &p);
                    self.top_fields(ui);
                    self.reference_buttons(ui);
                    self.chain_switch(ui);
                    self.place_fields(ui);
                    self.brush_fields(ui);
                    self.length_angle_fields(ui);
                    self.tool_step(ui);
                    return;
                }
                let sel: Vec<Element> = self.ed.selection.iter().copied().collect();
                let label = selection_label(&sel);
                let title = if label.is_empty() {
                    match self.tool {
                        Tool::Arrow => "Modificar".to_string(),
                        _ => format!("Modificar | {}", self.tool_name()),
                    }
                } else {
                    format!("Modificar | {label}")
                };
                bar_title(ui, &title, &p);
                self.tool_step(ui);
            });
        });
        // Fold and unfold the ribbon.
        let compact = self.prefs.ribbon_compact;
        let resp = ui.interact(fold, ui.scope_id().with("fold-ribbon"), Sense::click());
        if resp.hovered() {
            ui.painter()
                .rect_filled(fold, CornerRadius::same(6), p.raised);
        }
        icons::paint(
            ui.painter(),
            Rect::from_center_size(fold.center(), Vec2::splat(14.0)),
            if compact {
                Icon::ChevronDown
            } else {
                Icon::ChevronRight
            },
            p.muted,
        );
        if resp
            .on_hover_text(if compact {
                "Desplegar la cinta: los grupos con sus botones"
            } else {
                "Recoger la cinta: solo el nombre de cada grupo, que se abre al pulsarlo (más sitio para el dibujo)"
            })
            .clicked()
        {
            self.prefs.ribbon_compact = !compact;
            self.prefs.save();
        }
    }

    /// The name of the tool in hand, for the options bar.
    fn tool_name(&self) -> &'static str {
        match self.tool {
            Tool::Paint => "Pintar",
            Tool::Tape => "Cinta",
            Tool::TeamZone => "Zona de equipo",
            _ => "",
        }
    }

    /// Altura: where the walls and columns end at the top, as in Revit's
    /// options bar (the same as in Propiedades).
    fn top_fields(&mut self, ui: &mut Ui) {
        use crate::editor::TopChoice;
        let (top, height, offset) = match self.tool {
            Tool::Wall => (
                self.wall_settings.top,
                self.wall_settings.height,
                self.wall_settings.top_offset,
            ),
            Tool::Column => (
                self.column_settings.top,
                self.column_settings.height,
                self.column_settings.top_offset,
            ),
            _ => return,
        };
        let p = Palette::current(ui.ctx());
        let model = self.ed.model();
        let level = self.ed.level;
        let next = crate::editor::next_level_above(model, level)
            .and_then(|l| model.level(l))
            .map(|l| l.name.clone());
        let mut options: Vec<(TopChoice, String)> = vec![
            (TopChoice::Unconnected, "Desconectada".into()),
            (
                TopChoice::NextLevel,
                match &next {
                    Some(n) => format!("P. superior ({n})"),
                    None => "P. superior (no hay)".into(),
                },
            ),
        ];
        options.extend(
            self.levels_above(level)
                .into_iter()
                .map(|(id, name)| (TopChoice::Level(id), format!("Hasta {name}"))),
        );
        bar_separator(ui, &p);
        ui.label(egui::RichText::new("Altura").small().color(p.muted));
        let shown = options
            .iter()
            .find(|(v, _)| *v == top)
            .map(|(_, n)| n.clone())
            .unwrap_or_default();
        let mut chosen = top;
        egui::ComboBox::from_id_salt("bar-top")
            .width(118.0)
            .selected_text(egui::RichText::new(shown).small())
            .show_ui(ui, |ui| {
                for (v, n) in &options {
                    ui.selectable_value(&mut chosen, *v, n);
                }
            })
            .response
            .on_hover_text("Hasta dónde llega: una planta (sigue sus cambios de cota) o una altura propia (desconectada)");
        let to_level = match self.tool {
            Tool::Wall => {
                let mut s = self.wall_settings;
                s.top = chosen;
                matches!(
                    s.top_on(self.ed.model(), level),
                    archisim_core::WallTop::Level { .. }
                )
            }
            _ => {
                let mut s = self.column_settings;
                s.top = chosen;
                matches!(
                    s.top_on(self.ed.model(), level),
                    archisim_core::WallTop::Level { .. }
                )
            }
        };
        let (label, value, tip) = if to_level {
            ("Desfase", offset, "Distancia de la coronación a la planta: negativo, por debajo (el canto del forjado)")
        } else {
            ("", height, "Altura desde su base")
        };
        if !label.is_empty() {
            ui.label(egui::RichText::new(label).small().color(p.muted))
                .on_hover_text(tip);
        }
        let typed = super::text_edit_committed(
            ui,
            egui::Id::unique(("bar-top-value", to_level)),
            &crate::units::fmt_len(value),
            64.0,
        )
        .and_then(|t| crate::units::parse_length(&t));
        let (s_top, s_height, s_offset) = match self.tool {
            Tool::Wall => (
                &mut self.wall_settings.top,
                &mut self.wall_settings.height,
                &mut self.wall_settings.top_offset,
            ),
            _ => (
                &mut self.column_settings.top,
                &mut self.column_settings.height,
                &mut self.column_settings.top_offset,
            ),
        };
        *s_top = chosen;
        if let Some(v) = typed {
            if to_level {
                *s_offset = v;
            } else if v > 0.0 {
                *s_height = v;
            } else {
                self.complain("La altura debe ser mayor que cero.");
            }
        }
    }

    /// Cadena, for walls: each click goes on from the last one.
    fn chain_switch(&mut self, ui: &mut Ui) {
        if self.tool != Tool::Wall
            || !matches!(self.wall_tool.mode, WallMode::Single | WallMode::Chain)
        {
            return;
        }
        let mut on = self.wall_tool.mode == WallMode::Chain;
        ui.add_space(6.0);
        if ui
            .switch(&mut on, "Cadena")
            .on_hover_text("Tramos seguidos: cada clic añade un muro desde el anterior (Intro o Esc para terminar)")
            .changed()
        {
            self.wall_chain = on;
            self.wall_tool
                .set_mode(if on { WallMode::Chain } else { WallMode::Single });
        }
    }

    /// What a click does now with the tool in use, in grey at the end of
    /// the bar (it used to be a pill over the drawing): its first part,
    /// the rest when the cursor rests on it.
    fn tool_step(&mut self, ui: &mut Ui) {
        let p = Palette::current(ui.ctx());
        let hint = self.hint_text();
        let first = hint.split(" · ").next().unwrap_or(hint);
        if ui.available_width() < 80.0 {
            return;
        }
        bar_separator(ui, &p);
        let label = egui::Label::new(
            egui::RichText::new(first)
                .font(FontId::proportional(12.0))
                .color(p.muted),
        )
        .truncate();
        ui.add(label).on_hover_text(hint.replace(" · ", "\n"));
    }

    /// What the order running asks, its options and the last answer.
    fn order_step_body(&mut self, ui: &mut Ui) {
        let p = Palette::current(ui.ctx());
        let Some(prompt) = self.cad_prompt() else {
            return;
        };
        ui.label(
            egui::RichText::new(&prompt.text)
                .font(FontId::proportional(12.5))
                .color(p.text),
        );
        let mut clicked = None;
        for o in &prompt.options {
            let b = egui::Button::new(
                egui::RichText::new(o.label)
                    .font(FontId::proportional(12.0))
                    .color(p.accent),
            )
            .small();
            if ui
                .add(b)
                .on_hover_text(format!("También: escribe {} e Intro", o.key))
                .clicked()
            {
                clicked = Some(o.key);
            }
        }
        if let Some(d) = &prompt.default {
            ui.label(
                egui::RichText::new(format!("Intro: {d}"))
                    .small()
                    .color(p.muted),
            );
        }
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui
                .small_button("Terminar")
                .on_hover_text("Termina la orden (Esc)")
                .clicked()
            {
                self.cad_escape();
            }
            if let Some((text, error)) = self.cmdline.history.last() {
                ui.add(
                    egui::Label::new(egui::RichText::new(text).small().color(if *error {
                        p.bad
                    } else {
                        p.muted
                    }))
                    .truncate(),
                );
            }
        });
        if let Some(k) = clicked {
            self.cmdline.log(k.to_string(), false);
            self.cad_feed(Input::Option(k));
        }
    }
}

fn bar_title(ui: &mut Ui, text: &str, p: &Palette) {
    ui.add_space(6.0);
    ui.label(
        egui::RichText::new(text)
            .font(strong_font(12.5))
            .color(p.text),
    );
    ui.add_space(8.0);
}

fn bar_separator(ui: &mut Ui, p: &Palette) {
    let (r, _) = ui.allocate_exact_size(Vec2::new(11.0, 24.0), Sense::hover());
    ui.painter()
        .vline(r.center().x, r.y_range(), Stroke::new(1.0, p.line));
}

pub fn paint_mark(painter: &egui::Painter, rect: Rect, mark: Mark, color: Color32) {
    match mark {
        Mark::Glyph(g) => glyphs::paint(painter, rect, g, color),
        Mark::Icon(i) => icons::paint(painter, rect.shrink(1.0), i, color),
    }
}
