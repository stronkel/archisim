//! The toolbox (ArchiCAD's): tools by sections, as tiles with their icon,
//! name and key. Tools still to come are shown faint, with the item of the
//! roadmap that brings them, so the box already has its final shape.

use eframe::egui::{self, Rect, Sense, Ui, Vec2};

use super::icons::{self, Icon};
use super::theme::Palette;
use super::widgets::{section_label, tool_tile};
use super::{ArchiSimApp, Tool};
use crate::dim_tool::DimMode;

/// What a tile of the toolbox does.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Pick {
    Tool(Tool),
    /// The dimension tool placing level marks.
    LevelMark,
    /// A 2D drafting or editing command (AutoCAD's), by its name.
    Command(&'static str),
    /// Not there yet: the roadmap item that brings it.
    Later(&'static str),
}

pub struct Tile {
    pub pick: Pick,
    pub icon: Icon,
    pub name: &'static str,
    pub key: &'static str,
    pub hint: &'static str,
}

const fn tile(
    pick: Pick,
    icon: Icon,
    name: &'static str,
    key: &'static str,
    hint: &'static str,
) -> Tile {
    Tile {
        pick,
        icon,
        name,
        key,
        hint,
    }
}

/// The sections of the toolbox, top to bottom.
pub const SECTIONS: &[(&str, &[Tile])] = &[
    (
        "SELECCIÓN",
        &[tile(
            Pick::Tool(Tool::Arrow),
            Icon::Arrow,
            "Flecha",
            "Esc",
            "Seleccionar, mover, copiar y estirar. Arrastrar en vacío: ventana de selección",
        )],
    ),
    (
        "MODELADO",
        &[
            tile(
                Pick::Tool(Tool::Wall),
                Icon::Wall,
                "Muro",
                "M",
                "Muros multicapa por su línea de referencia",
            ),
            tile(Pick::Later("2.5"), Icon::Column, "Pilar", "", "Pilares"),
            tile(
                Pick::Tool(Tool::Slab),
                Icon::Slab,
                "Forjado",
                "F",
                "Forjados por contorno o con la varita mágica",
            ),
            tile(Pick::Later("2.5"), Icon::Beam, "Viga", "", "Vigas"),
            tile(
                Pick::Tool(Tool::Door),
                Icon::Door,
                "Puerta",
                "P",
                "Puertas en muros",
            ),
            tile(
                Pick::Tool(Tool::Window),
                Icon::Window,
                "Ventana",
                "V",
                "Ventanas en muros",
            ),
            tile(
                Pick::Later("2.4"),
                Icon::Roof,
                "Cubierta",
                "",
                "Cubiertas inclinadas",
            ),
            tile(Pick::Later("2.6"), Icon::Stair, "Escalera", "", "Escaleras"),
            tile(
                Pick::Later("2.2"),
                Icon::Object,
                "Objeto",
                "",
                "Mobiliario y sanitarios",
            ),
            tile(
                Pick::Later("2.9"),
                Icon::Room,
                "Zona",
                "",
                "Locales (IfcSpace)",
            ),
        ],
    ),
    (
        "DOCUMENTACIÓN",
        &[
            tile(
                Pick::Tool(Tool::Dimension),
                Icon::Dimension,
                "Cota",
                "C",
                "Cotas lineales, encadenadas y de huecos de muro",
            ),
            tile(
                Pick::Command("ACOANGULO"),
                Icon::DimAngle,
                "Angular",
                "DAN",
                "Cota del ángulo entre dos líneas o de un arco",
            ),
            tile(
                Pick::Command("ACORADIO"),
                Icon::DimRadius,
                "Radio",
                "DRA",
                "Cota de radio de un arco o círculo (diámetro: DDI)",
            ),
            tile(
                Pick::Command("ACOARCO"),
                Icon::DimArc,
                "Long. arco",
                "DAR",
                "Cota de la longitud de un arco",
            ),
            tile(
                Pick::LevelMark,
                Icon::Elevation,
                "Cota nivel",
                "N",
                "Cota de nivel del suelo terminado",
            ),
            tile(
                Pick::Command("TEXTO"),
                Icon::Text,
                "Texto",
                "TX",
                "Textos de una o varias líneas",
            ),
            tile(Pick::Later("3.3"), Icon::Tag, "Etiqueta", "", "Etiquetas"),
            tile(
                Pick::Command("SOMBCONT"),
                Icon::Hatch,
                "Sombreado",
                "SB",
                "Tramas y rellenos: clic dentro de un recinto",
            ),
            tile(
                Pick::Command("DIRECTRIZ"),
                Icon::Leader,
                "Directriz",
                "DIR",
                "Flecha con texto",
            ),
            tile(
                Pick::Command("NUBEREV"),
                Icon::Cloud,
                "Nube",
                "NR",
                "Nube de revisión",
            ),
        ],
    ),
    (
        "DIBUJO 2D",
        &[
            tile(
                Pick::Command("LINEA"),
                Icon::Line,
                "Línea",
                "L",
                "Líneas de punto a punto",
            ),
            tile(
                Pick::Command("POL"),
                Icon::Pline,
                "Polilínea",
                "PL",
                "Tramos rectos y de arco, con anchura",
            ),
            tile(
                Pick::Command("RECTANG"),
                Icon::GeoRect,
                "Rectángulo",
                "REC",
                "Por dos esquinas, girado o con esquinas redondeadas",
            ),
            tile(
                Pick::Command("POLIGONO"),
                Icon::GeoPolygon,
                "Polígono",
                "PG",
                "Polígono regular inscrito o circunscrito",
            ),
            tile(
                Pick::Command("ARCO"),
                Icon::GeoArc,
                "Arco",
                "A",
                "Por tres puntos o por centro, inicio y fin",
            ),
            tile(
                Pick::Command("CIRCULO"),
                Icon::GeoCircle,
                "Círculo",
                "C",
                "Centro y radio o diámetro; 2P; 3P",
            ),
            tile(
                Pick::Command("ELIPSE"),
                Icon::Ellipse,
                "Elipse",
                "EL",
                "Por un eje y la otra distancia, o por su centro",
            ),
        ],
    ),
    (
        "EDICIÓN",
        &[
            tile(
                Pick::Command("DESPLAZA"),
                Icon::Move,
                "Desplazar",
                "D",
                "Mover lo seleccionado de un punto base a otro",
            ),
            tile(
                Pick::Command("COPIA"),
                Icon::Copy,
                "Copiar",
                "CP",
                "Copias de lo seleccionado, una tras otra",
            ),
            tile(
                Pick::Command("GIRA"),
                Icon::Rotate,
                "Girar",
                "GI",
                "Girar alrededor de un punto (Copia, Referencia)",
            ),
            tile(
                Pick::Command("SIMETRIA"),
                Icon::Mirror,
                "Simetría",
                "SI",
                "Reflejar respecto a una línea",
            ),
            tile(
                Pick::Command("ESCALA"),
                Icon::Scale,
                "Escala",
                "ES",
                "Cambiar de tamaño desde un punto base",
            ),
            tile(
                Pick::Command("EQUIDIST"),
                Icon::Offset,
                "Desfase",
                "EQ",
                "Paralela a una distancia",
            ),
            tile(
                Pick::Command("RECORTA"),
                Icon::Trim,
                "Recortar",
                "RR",
                "Recortar contra cualquier borde (Mayús: alargar)",
            ),
            tile(
                Pick::Command("ALARGA"),
                Icon::Extend,
                "Alargar",
                "AL",
                "Alargar hasta el borde más próximo (Mayús: recortar)",
            ),
            tile(
                Pick::Command("EMPALME"),
                Icon::Fillet,
                "Empalme",
                "EM",
                "Unir dos líneas en esquina o con un arco",
            ),
            tile(
                Pick::Command("MATRIZ"),
                Icon::Array,
                "Matriz",
                "MA",
                "Copias en filas y columnas o alrededor de un centro",
            ),
            tile(
                Pick::Command("DIST"),
                Icon::Measure,
                "Medir",
                "DI",
                "Distancia entre dos puntos (AA: área)",
            ),
        ],
    ),
    (
        "VISTAS",
        &[
            tile(
                Pick::Later("2.10"),
                Icon::Section,
                "Sección",
                "",
                "Secciones",
            ),
            tile(
                Pick::Later("2.10"),
                Icon::Elevation,
                "Alzado",
                "",
                "Alzados",
            ),
        ],
    ),
];

impl ArchiSimApp {
    /// Whether a tile is the tool in use.
    fn picked(&self, pick: Pick) -> Option<bool> {
        let level_marks = self.dim_settings.mode == DimMode::LevelMark;
        match pick {
            Pick::Tool(Tool::Dimension) => Some(self.tool == Tool::Dimension && !level_marks),
            Pick::Tool(t) => Some(self.tool == t),
            Pick::LevelMark => Some(self.tool == Tool::Dimension && level_marks),
            Pick::Command(name) => {
                Some(self.tool == Tool::Cad && self.cad.as_ref().is_some_and(|c| c.name() == name))
            }
            Pick::Later(_) => None,
        }
    }

    /// Takes the tool of a tile (or of its key).
    pub(super) fn pick(&mut self, pick: Pick) {
        match pick {
            Pick::Tool(Tool::Dimension) => {
                if self.dim_settings.mode == DimMode::LevelMark {
                    self.dim_settings.mode = DimMode::Linear;
                    self.dim_tool.finish();
                }
                self.set_tool(Tool::Dimension);
            }
            Pick::Tool(t) => self.set_tool(t),
            Pick::LevelMark => {
                if self.dim_settings.mode != DimMode::LevelMark {
                    self.dim_settings.mode = DimMode::LevelMark;
                    self.dim_tool.finish();
                }
                self.set_tool(Tool::Dimension);
            }
            Pick::Command(name) => self.start_command(name),
            Pick::Later(item) => self.say(format!(
                "Esta herramienta llegará con el punto {item} de la hoja de ruta."
            )),
        }
    }

    pub(super) fn toolbox(&mut self, ui: &mut Ui) {
        let p = Palette::current(ui.ctx());
        let compact = self.compact_toolbox;
        // Collapse button at the top.
        ui.add_space(8.0);
        ui.horizontal(|ui| {
            if !compact {
                ui.add_space(2.0);
                ui.label(
                    egui::RichText::new("Herramientas")
                        .font(super::widgets::strong_font(12.0))
                        .color(p.text),
                );
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let icon = if compact {
                    Icon::ChevronRight
                } else {
                    Icon::ChevronLeft
                };
                let hint = if compact {
                    "Mostrar los nombres"
                } else {
                    "Solo iconos"
                };
                if super::widgets::icon_button(ui, icon, hint, false).clicked() {
                    self.compact_toolbox = !self.compact_toolbox;
                }
            });
        });
        ui.add_space(4.0);
        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                let gap = 4.0;
                let full = ui.available_width();
                for (title, tiles) in SECTIONS {
                    if compact {
                        let (r, _) = ui.allocate_exact_size(Vec2::new(full, 9.0), Sense::hover());
                        ui.painter().line_segment(
                            [
                                r.left_center() + Vec2::new(10.0, 0.0),
                                r.right_center() - Vec2::new(10.0, 0.0),
                            ],
                            egui::Stroke::new(1.0, p.line),
                        );
                    } else {
                        ui.add_space(4.0);
                        section_label(ui, title);
                    }
                    let columns = if compact || tiles.len() == 1 { 1 } else { 2 };
                    let w = (full - gap * (columns as f32 - 1.0)) / columns as f32;
                    for row in tiles.chunks(columns) {
                        ui.horizontal(|ui| {
                            ui.spacing_mut().item_spacing.x = gap;
                            for t in row {
                                let active = self.picked(t.pick);
                                let resp = if compact {
                                    compact_tile(ui, w, t.icon, active)
                                } else {
                                    tool_tile(ui, w, t.icon, t.name, t.key, active)
                                };
                                let hint = match t.pick {
                                    Pick::Later(item) => format!(
                                        "{} — {} (llegará con el punto {item} de la hoja de ruta)",
                                        t.name, t.hint
                                    ),
                                    _ if t.key.is_empty() => format!("{} — {}", t.name, t.hint),
                                    _ => format!("{} ({}) — {}", t.name, t.key, t.hint),
                                };
                                if resp.on_hover_text(hint).clicked() {
                                    self.pick(t.pick);
                                }
                            }
                        });
                        ui.add_space(gap - ui.spacing().item_spacing.y);
                    }
                }
            });
    }
}

/// A square tile with only the icon (compact toolbox).
fn compact_tile(ui: &mut Ui, width: f32, icon: Icon, active: Option<bool>) -> egui::Response {
    let p = Palette::current(ui.ctx());
    let sense = if active.is_some() {
        Sense::click()
    } else {
        Sense::hover()
    };
    let (rect, resp) = ui.allocate_exact_size(Vec2::new(width, 36.0), sense);
    let (fill, fg) = match active {
        Some(true) => (p.accent_soft, p.accent),
        Some(false) if resp.hovered() => (p.raised, p.text),
        Some(false) => (egui::Color32::TRANSPARENT, p.text),
        None => (egui::Color32::TRANSPARENT, p.faint),
    };
    ui.painter()
        .rect_filled(rect, egui::CornerRadius::same(8), fill);
    if active == Some(true) {
        ui.painter().rect_stroke(
            rect,
            egui::CornerRadius::same(8),
            egui::Stroke::new(1.5, p.accent),
            egui::StrokeKind::Inside,
        );
    }
    icons::paint(
        ui.painter(),
        Rect::from_center_size(rect.center(), Vec2::splat(22.0)),
        icon,
        fg,
    );
    resp
}
