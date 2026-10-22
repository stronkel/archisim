//! The contextual bar (`docs/INTERFAZ.md` §2.6): fixed under the tabs,
//! always there and always in the same place. With a tool it shows the
//! tool's insertion methods; with something selected, «Modificar» with its
//! orders, the same as AutoCAD's keys; with an order running, its step;
//! with nothing, the orders in grey.

use eframe::egui::{self, Align2, Color32, CornerRadius, FontId, Rect, Sense, Stroke, Ui, Vec2};

use super::glyphs::{self, Glyph};
use super::icons::{self, Icon};
use super::theme::Palette;
use super::toolbox::Mark;
use super::widgets::strong_font;
use super::{ArchiSimApp, Tool};
use crate::cad::Input;
use crate::dim_tool::DimMode;
use crate::pick::Element;
use crate::slab_tool::SlabMode;
use crate::wall_tool::WallMode;

pub const CONTEXT_BAR_HEIGHT: f32 = 40.0;

/// What a method button does.
#[derive(Debug, Clone, Copy, PartialEq)]
enum MethodAct {
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
    /// Not for this tool.
    None,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum Outline {
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
        command: "SIMETRIA",
        mark: Mark::Icon(Icon::Mirror),
        name: "Simetría",
        key: "SI",
        hint: "Dos puntos del eje; las puertas y ventanas cambian de mano",
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

/// The name of one element, for «Modificar · Muro (1)».
fn element_name(e: &Element) -> &'static str {
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
        Element::Stair(_) => "Escalera",
        Element::Terrain(_) => "Terreno",
        Element::CurtainWall(_) => "Muro cortina",
        Element::Railing(_) => "Barandilla",
        Element::Object(_) => "Objeto",
        Element::Room(_) => "Local",
        Element::Tag(_) => "Etiqueta",
        Element::Footing(_) => "Cimentación",
        Element::Grid(_) => "Eje",
        Element::Module(_) => "Módulo",
        Element::Mass(_) => "Volumen",
        Element::Plot(_) => "Parcela",
        Element::Road(_) => "Vial",
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
        "ESCALA" => !sel.is_empty() && !sel.iter().any(|e| matches!(e, Element::Opening(_))),
        "DESPLAZA" | "COPIA" | "GIRA" | "SIMETRIA" | "MATRIZ" | "ALINEAR" => !sel.is_empty(),
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
                Tool::Door => ("Puerta", None, "Varita", ""),
                Tool::Window => ("Ventana", None, "Varita", ""),
                Tool::Column => ("Pilar", None, "Varita", ""),
                Tool::Beam => ("Viga", None, "Varita", ""),
                Tool::Section => ("Sección", None, "Varita", ""),
                Tool::Stair => ("Escalera", None, "Varita", ""),
                Tool::Terrain => ("Terreno", None, "Varita", ""),
                Tool::CurtainWall => ("Muro cortina", None, "Varita", ""),
                Tool::Railing => ("Barandilla", None, "Varita", ""),
                Tool::Object => ("Objeto", None, "Varita", ""),
                Tool::Grid => ("Eje", None, "Varita", ""),
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
                Tool::MassOffset => ("Equidistancia", None, "Varita", ""),
                Tool::Tape => ("Cinta", None, "Varita", ""),
                Tool::Paint => ("Pintar", None, "Varita", ""),
                Tool::Plot => (
                    "Parcela",
                    Some(outline(false, self.plot_tool.mode)),
                    "Varita",
                    "",
                ),
                Tool::Road => ("Vial", None, "Varita", ""),
                Tool::Planting => ("Plantación", None, "Varita", ""),
                Tool::Pipe => ("Tubería", None, "Varita", ""),
                Tool::Duct => ("Conducto", None, "Varita", ""),
                Tool::Device => ("Aparato", None, "Varita", ""),
                Tool::Footing => return Some(("Cimentación", self.footing_methods())),
                Tool::Arrow | Tool::Cad => return None,
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
            Some(_) if o == Outline::Wand && self.tool == Tool::Plot => MethodAct::None,
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
                "Un tramo por dos clics",
                straight,
                wall == Some(WallMode::Single) || line_tool,
            ),
            m(
                Mark::Icon(Icon::GeoChain),
                "Cadena",
                "Tramos seguidos: cada clic añade uno desde el anterior",
                wall_act(WallMode::Chain),
                wall == Some(WallMode::Chain),
            ),
            m(
                Mark::Icon(Icon::GeoRect),
                "Rectángulo",
                "Dos esquinas opuestas (o la primera y su tamaño: 6000x4000)",
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
            MethodAct::Wall(mode) => self.wall_tool.set_mode(mode),
            MethodAct::Outline(o) => match self.tool {
                Tool::Slab => set_outline(&mut self.slab_settings.wand, &mut self.slab_tool, o),
                Tool::Roof => set_outline(&mut self.roof_settings.wand, &mut self.roof_tool, o),
                Tool::Zone => set_outline(&mut self.room_settings.wand, &mut self.zone_tool, o),
                Tool::Plot => {
                    let mut wand = false;
                    set_outline(&mut wand, &mut self.plot_tool, o)
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
            MethodAct::MassMode(mode) => self.set_mass_mode(mode),
            MethodAct::Spline => self.set_draw_method(super::methods_ui::DrawMethod::Spline),
            MethodAct::PickLine => self.set_draw_method(super::methods_ui::DrawMethod::PickLine),
            MethodAct::FoundLevel => self.found_level(),
            MethodAct::Command(name) => self.start_command(name),
            MethodAct::Fixed | MethodAct::None => {}
        }
    }

    /// The bar under the tabs.
    pub(super) fn context_bar(&mut self, ui: &mut Ui) {
        let p = Palette::current(ui.ctx());
        ui.horizontal_centered(|ui| {
            ui.spacing_mut().item_spacing.x = 2.0;
            if self.tool == Tool::Cad {
                // A drawing order of the bottom bar: its variants, as the
                // methods of a tool, then its step.
                let name = self.cad.as_ref().map(|c| c.name()).unwrap_or_default();
                if let Some((title, variants)) = super::bottom_segments::segment_of(name) {
                    bar_title(ui, title, &p);
                    let mut chosen = None;
                    for v in variants {
                        let on = matches!(v.pick, super::toolbox::Pick::Command(c) if c == name);
                        let r = bar_button(ui, v.mark, v.name, on, true, false)
                            .on_hover_text(format!("{} ({})", v.name, v.key));
                        if r.clicked() && !on {
                            chosen = Some(v.pick);
                        }
                    }
                    bar_separator(ui, &p);
                    if let Some(pick) = chosen {
                        self.pick(pick);
                        return;
                    }
                    self.order_step_body(ui);
                    return;
                }
                self.order_step(ui);
                return;
            }
            if let Some((name, methods)) = self.methods() {
                bar_title(ui, name, &p);
                let room = ui.available_width() - 150.0;
                let named = names_fit(ui, methods.iter().map(|m| m.name), room);
                let mut chosen = None;
                for m in &methods {
                    let enabled = m.act != MethodAct::None;
                    let r = bar_button(ui, m.mark, m.name, m.on, enabled, named);
                    let r = if enabled {
                        r.on_hover_text(format!("{}: {}", m.name, m.hint))
                    } else {
                        r.on_hover_text(format!("{}: no vale para esta herramienta", m.name))
                    };
                    if r.clicked() && enabled {
                        chosen = Some(m.act);
                    }
                }
                if let Some(act) = chosen {
                    self.apply_method(act);
                }
                self.length_angle_fields(ui);
                return;
            }
            // Modificar: with the selection, or grey without it.
            let sel: Vec<Element> = self.ed.selection.iter().copied().collect();
            let label = selection_label(&sel);
            let title = if label.is_empty() {
                "Modificar".to_string()
            } else {
                format!("Modificar · {label}")
            };
            bar_title(ui, &title, &p);
            let named = names_fit(
                ui,
                TRANSFORM.iter().chain(GEOMETRY).map(|o| o.name),
                ui.available_width() - 11.0,
            );
            let mut run = None;
            for (i, group) in [TRANSFORM, GEOMETRY].iter().enumerate() {
                if i > 0 {
                    bar_separator(ui, &p);
                }
                for o in group.iter() {
                    let enabled = order_applies(o.command, &sel);
                    let r = bar_button(ui, o.mark, o.name, false, enabled, named);
                    let r = if enabled {
                        r.on_hover_text(format!("{} ({}) — {}", o.name, o.key, o.hint))
                    } else if sel.is_empty() {
                        r.on_hover_text(format!(
                            "{} ({}) — selecciona antes lo que quieres modificar",
                            o.name, o.key
                        ))
                    } else {
                        r.on_hover_text(format!(
                            "{} ({}) — no vale para lo seleccionado",
                            o.name, o.key
                        ))
                    };
                    if r.clicked() && enabled {
                        run = Some(o.command);
                    }
                }
            }
            if let Some(c) = run {
                self.start_command(c);
            }
        });
    }

    /// The step of the order running: its name, what it asks and its
    /// options; the last answer at the right.
    fn order_step(&mut self, ui: &mut Ui) {
        let p = Palette::current(ui.ctx());
        if self.cad_prompt().is_none() {
            return;
        }
        let name = self.cad.as_ref().map(|c| c.name()).unwrap_or_default();
        let title = match TRANSFORM.iter().chain(GEOMETRY).find(|o| o.command == name) {
            Some(o) => format!("Modificar · {}", o.name),
            None => super::toolbox::order_name(name).unwrap_or(name).to_string(),
        };
        bar_title(ui, &title, &p);
        self.order_step_body(ui);
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

/// A button of the bar: its drawing and name; accent when on, faint when
/// it does not apply.
pub fn bar_button(
    ui: &mut Ui,
    mark: Mark,
    name: &str,
    on: bool,
    enabled: bool,
    named: bool,
) -> egui::Response {
    let p = Palette::current(ui.ctx());
    let font = FontId::proportional(12.0);
    let w = button_width(ui, name, named);
    let sense = if enabled {
        Sense::click()
    } else {
        Sense::hover()
    };
    let (rect, resp) = ui.allocate_exact_size(Vec2::new(w, 28.0), sense);
    let (fill, fg) = if on {
        (p.accent_soft, p.accent)
    } else if !enabled {
        (Color32::TRANSPARENT, p.faint)
    } else if resp.hovered() {
        (p.raised, p.text)
    } else {
        (Color32::TRANSPARENT, p.text)
    };
    ui.painter().rect_filled(rect, CornerRadius::same(7), fill);
    if on {
        ui.painter().rect_stroke(
            rect,
            CornerRadius::same(7),
            Stroke::new(1.0, p.accent),
            egui::StrokeKind::Inside,
        );
    }
    let icon_at = if named {
        rect.left_center() + Vec2::new(8.0 + 9.0, 0.0)
    } else {
        rect.center()
    };
    paint_mark(
        ui.painter(),
        Rect::from_center_size(icon_at, Vec2::splat(18.0)),
        mark,
        fg,
    );
    if named {
        ui.painter().text(
            rect.left_center() + Vec2::new(8.0 + 18.0 + 5.0, 0.0),
            Align2::LEFT_CENTER,
            name,
            font,
            fg,
        );
    }
    resp
}

/// Width of a button of the bar, with its name or only its drawing.
fn button_width(ui: &Ui, name: &str, named: bool) -> f32 {
    if !named {
        return 32.0;
    }
    let g =
        ui.painter()
            .layout_no_wrap(name.to_string(), FontId::proportional(12.0), Color32::WHITE);
    8.0 + 18.0 + 5.0 + g.size().x + 9.0
}

/// Whether buttons with these names fit, with their names, in `room`.
fn names_fit<'a>(ui: &Ui, names: impl IntoIterator<Item = &'a str>, room: f32) -> bool {
    let need: f32 = names
        .into_iter()
        .map(|n| button_width(ui, n, true) + 2.0)
        .sum();
    need <= room
}

#[cfg(test)]
mod tests {
    use super::*;
    use archisim_core::{DraftId, OpeningId, WallId};

    #[test]
    fn selection_labels() {
        let walls = [Element::Wall(WallId(1)), Element::Wall(WallId(2))];
        assert_eq!(selection_label(&walls), "Muro (2)");
        let mixed = [Element::Wall(WallId(1)), Element::Opening(OpeningId(2))];
        assert_eq!(selection_label(&mixed), "Selección (2)");
        assert_eq!(selection_label(&[]), "");
    }

    #[test]
    fn orders_that_apply() {
        let door = [Element::Opening(OpeningId(1))];
        assert!(order_applies("DESPLAZA", &door));
        assert!(!order_applies("ESCALA", &door));
        assert!(!order_applies("RECORTA", &door));
        let line = [Element::Draft(DraftId(1))];
        assert!(order_applies("RECORTA", &line));
        assert!(order_applies("ESCALA", &line));
        assert!(!order_applies("DESPLAZA", &[]));
    }
}
