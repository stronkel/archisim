//! The properties palette, as Revit's. It shows, in this order of
//! preference:
//! - with a drawing tool active, what that tool will build (its type and
//!   its settings): it replaces ArchiCAD's info box;
//! - with a selection, the selected elements (type selector on top, then
//!   Restricciones, Construcción, Dimensiones and Datos de identidad);
//! - with nothing selected, the active view and the project.

use archisim_core::{
    DimensionId, ElevationMarkId, Hand, OpeningId, OpeningKind, SlabId, WallId, WallSide, WallTop,
    WallTypeId,
};
use archisim_geom::Contour;
use archisim_plan::annotate::{format_dimension, format_elevation};
use eframe::egui::{self, Id, Ui};

use super::icons::Icon;
use super::props::{self, choice, group, id_row, row, text_row, type_card, TypeItem};
use super::tabs::SCALES;
use super::widgets::{method_picker, segmented};
use super::*;
use crate::dim_tool::{DimDirection, DimMode};
use crate::editor::TopChoice;
use crate::slab_tool::SlabMode;
use crate::wall_tool::WallMode;

const REFERENCES: [(ReferenceLine, &str); 3] = [
    (ReferenceLine::ExteriorFace, "Cara exterior"),
    (ReferenceLine::Center, "Eje"),
    (ReferenceLine::InteriorFace, "Cara interior"),
];
const SIDES: [(WallSide, &str); 2] = [
    (WallSide::Interior, "Cara interior"),
    (WallSide::Exterior, "Cara exterior"),
];
const HANDS: [(Hand, &str); 2] = [(Hand::Left, "Izquierda"), (Hand::Right, "Derecha")];

impl ArchiSimApp {
    pub(super) fn properties(&mut self, ui: &mut Ui) {
        let selection: Vec<Element> = self.ed.selection.iter().copied().collect();
        if self.tool != Tool::Arrow {
            self.tool_properties(ui);
            return;
        }
        match selection.as_slice() {
            [] => self.view_properties(ui),
            [Element::Wall(id)] => self.wall_properties(ui, *id),
            [Element::Opening(id)] => self.opening_properties(ui, *id),
            [Element::Slab(id)] => self.slab_properties(ui, *id),
            [Element::Dimension(id)] => self.dimension_properties(ui, *id),
            [Element::Mark(id)] => self.mark_properties(ui, *id),
            [Element::Column(id)] => self.column_properties(ui, *id),
            [Element::Beam(id)] => self.beam_properties(ui, *id),
            [Element::Space(id)] => self.space_properties(ui, *id),
            many => self.many_properties(ui, many),
        }
    }

    /// The elements of the current level of the same type as `e`.
    pub(super) fn same_type(&self, e: Element) -> Vec<Element> {
        let model = self.ed.model();
        crate::pick::type_of(model, e)
            .map(|t| crate::pick::of_type_on_level(model, self.ed.level, t))
            .unwrap_or_default()
    }

    pub(super) fn select_instances(&mut self, found: Vec<Element>, what: &str) {
        let n = found.len();
        self.ed.selection = found.into_iter().collect();
        self.say(format!(
            "Seleccionados {n} {what} de este tipo en la planta."
        ));
    }

    /// What the pictures of the types depend on besides the type itself:
    /// the colours and hatches of the materials (and, for doors and
    /// windows, the walls that host them in the picture).
    pub(super) fn picture_signature(&self, hosts: bool) -> String {
        let m = self.ed.model();
        let mats: Vec<_> = m
            .materials()
            .map(|x| (x.id, x.cut_color, x.cut_pattern))
            .collect();
        if hosts {
            let walls: Vec<_> = m.wall_types().map(|t| (t.id, &t.layers)).collect();
            format!("{mats:?}{walls:?}")
        } else {
            format!("{mats:?}")
        }
    }

    fn wall_type_items(&mut self, ctx: &egui::Context) -> Vec<TypeItem<WallTypeId>> {
        let sig = self.picture_signature(false);
        let model = self.ed.model();
        let thumbs = &mut self.thumbs;
        model
            .wall_types()
            .map(|t| {
                let f = match t.function {
                    WallFunction::Exterior => "exterior",
                    WallFunction::Interior => "interior",
                };
                let thumb = thumbs.get(ctx, format!("wall:{:?}:{sig}", t.layers), || {
                    crate::type_preview::layers_model(model, &t.layers, 0.6)
                });
                TypeItem {
                    id: t.id,
                    name: t.name.clone(),
                    note: format!(
                        "{} mm · {f} · {} capas",
                        fmt_mm(t.thickness()),
                        t.layers.len()
                    ),
                    thumb,
                    caption: "Corte en planta 1:20 · exterior arriba",
                }
            })
            .collect()
    }

    fn slab_type_items(&mut self, ctx: &egui::Context) -> Vec<TypeItem<SlabTypeId>> {
        let sig = self.picture_signature(false);
        let model = self.ed.model();
        let thumbs = &mut self.thumbs;
        model
            .slab_types()
            .map(|t| {
                let f = match t.function {
                    SlabFunction::Floor => "forjado de planta",
                    SlabFunction::BaseSlab => "solera",
                    SlabFunction::Roof => "cubierta",
                };
                // The layers as a band, the top face up (as in the types
                // dialog).
                let thumb = thumbs.get(ctx, format!("slab:{:?}:{sig}", t.layers), || {
                    crate::type_preview::layers_model(model, &t.layers, 0.6)
                });
                TypeItem {
                    id: t.id,
                    name: t.name.clone(),
                    note: format!("canto {} mm · {f}", fmt_mm(t.thickness())),
                    thumb,
                    caption: "Capas 1:20 · cara superior arriba",
                }
            })
            .collect()
    }

    fn opening_type_items(
        &mut self,
        ctx: &egui::Context,
        door: bool,
    ) -> Vec<TypeItem<OpeningTypeId>> {
        let sig = self.picture_signature(true);
        let model = self.ed.model();
        let thumbs = &mut self.thumbs;
        model
            .opening_types()
            .filter(|t| t.is_door() == door)
            .map(|t| {
                let name = if t.mark.is_empty() {
                    t.name.clone()
                } else {
                    format!("{}  {}", t.mark, t.name)
                };
                let thumb = thumbs.get(ctx, format!("opening:{t:?}:{sig}"), || {
                    crate::type_preview::opening_model(model, t)
                });
                TypeItem {
                    id: t.id,
                    name,
                    note: format!(
                        "{} × {} mm · {}",
                        fmt_mm(t.width),
                        fmt_mm(t.height),
                        types_ui::family_label(model, t.family)
                    ),
                    thumb,
                    caption: "Planta 1:20 · exterior arriba",
                }
            })
            .collect()
    }

    // ---- Tools (ArchiCAD's info box) ----------------------------------------

    fn tool_properties(&mut self, ui: &mut Ui) {
        match self.tool {
            Tool::Arrow => {}
            Tool::Wall => self.wall_tool_properties(ui),
            Tool::Slab => self.slab_tool_properties(ui),
            Tool::Door => self.opening_tool_properties(ui, true),
            Tool::Window => self.opening_tool_properties(ui, false),
            Tool::Dimension => self.dim_tool_properties(ui),
            Tool::Column => self.column_tool_properties(ui),
            Tool::Beam => self.beam_tool_properties(ui),
            Tool::Zone => self.space_tool_properties(ui),
        }
    }

    fn wall_tool_properties(&mut self, ui: &mut Ui) {
        props::title(ui, "Herramienta Muro · lo que vas a dibujar");
        let items = self.wall_type_items(ui.ctx());
        let mut chosen = None;
        let card = type_card(
            ui,
            "tool-wall",
            Icon::Wall,
            "Muro",
            "IfcWall",
            Some(self.wall_settings.wall_type),
            &items,
            &mut chosen,
            None,
        );
        if let Some(t) = chosen {
            self.wall_settings.wall_type = t;
            // Façades are usually drawn by their outside face, partitions
            // by their axis.
            if let Some(f) = self.ed.model().wall_type(t).map(|t| t.function) {
                self.wall_settings.reference = default_reference(f);
            }
        }
        if card.edit {
            self.edit_type(TypesTab::Walls, self.wall_settings.wall_type.0);
        }
        let base_name = self
            .ed
            .model()
            .level(self.ed.level)
            .map(|l| l.name.clone())
            .unwrap_or_default();
        let above = self.levels_above(self.ed.level);
        let next = crate::editor::next_level_above(self.ed.model(), self.ed.level);
        let next_name = next
            .and_then(|l| self.ed.model().level(l))
            .map(|l| l.name.clone());
        let resolved = self.wall_settings.top_on(self.ed.model(), self.ed.level);
        group(ui, "tool-wall-method", "Método", |ui| {
            ui.add_space(2.0);
            let mut mode = self.wall_tool.mode;
            let options: Vec<(WallMode, Icon, &str, &str)> = WallMode::ALL
                .iter()
                .map(|m| {
                    let icon = match m {
                        WallMode::Single => Icon::GeoSingle,
                        WallMode::Chain => Icon::GeoChain,
                        WallMode::Rectangle => Icon::GeoRect,
                        WallMode::Arc => Icon::GeoArc,
                        WallMode::Circle => Icon::GeoCircle,
                    };
                    (*m, icon, m.name(), m.help())
                })
                .collect();
            if method_picker(ui, "wall-method", &mut mode, &options) {
                self.wall_tool.set_mode(mode);
            }
            ui.add_space(2.0);
        });
        group(ui, "tool-wall-constraints", "Restricciones", |ui| {
            text_row(
                ui,
                "Restricción de base",
                "La planta en la que dibujas",
                &base_name,
            );
            row(
                ui,
                "Desfase de base (mm)",
                "Altura de la base del muro sobre la planta",
                |ui, w| {
                    if let Some(v) = length_field(
                        ui,
                        Id::unique("tool-wall-base"),
                        self.wall_settings.base_offset,
                        w - 8.0,
                    ) {
                        self.wall_settings.base_offset = v;
                    }
                },
            );
            let next_label = match &next_name {
                Some(n) => format!("Planta superior ({n})"),
                None => "Planta superior (no hay)".to_string(),
            };
            let mut options: Vec<(TopChoice, String)> = vec![
                (TopChoice::Unconnected, "Desconectada".into()),
                (TopChoice::NextLevel, next_label),
            ];
            options.extend(
                above
                    .iter()
                    .map(|(id, name)| (TopChoice::Level(*id), format!("Hasta {name}"))),
            );
            row(
                ui,
                "Restricción superior",
                "Hasta dónde llega el muro: una planta (sigue sus cambios de cota) o una altura propia (desconectada). «Planta superior» es siempre la de encima de la que dibujas",
                |ui, w| {
                    let opts: Vec<(TopChoice, &str)> =
                        options.iter().map(|(v, n)| (*v, n.as_str())).collect();
                    self.wall_settings.top =
                        choice(ui, "tool-wall-top", w, self.wall_settings.top, &opts);
                },
            );
            match resolved {
                WallTop::Level { .. } => {
                    row(
                        ui,
                        "Desfase superior (mm)",
                        "Distancia de la coronación a la planta superior: negativo, por debajo (por ejemplo, el canto del forjado)",
                        |ui, w| {
                            if let Some(v) = length_field(
                                ui,
                                Id::unique("tool-wall-top-offset"),
                                self.wall_settings.top_offset,
                                w - 8.0,
                            ) {
                                self.wall_settings.top_offset = v;
                            }
                        },
                    );
                }
                WallTop::Unconnected => {
                    row(
                        ui,
                        "Altura desconectada (mm)",
                        "Altura del muro desde su base",
                        |ui, w| {
                            if let Some(v) = length_field(
                                ui,
                                Id::unique("tool-wall-height"),
                                self.wall_settings.height,
                                w - 8.0,
                            ) {
                                if v > 0.0 {
                                    self.wall_settings.height = v;
                                } else {
                                    self.complain("La altura debe ser mayor que cero.");
                                }
                            }
                        },
                    );
                    if self.wall_settings.top == TopChoice::NextLevel {
                        props::note(
                            ui,
                            "No hay planta encima de esta: el muro tendrá su propia altura. Añade una planta para unirlo a ella.",
                        );
                    }
                }
            }
        });
        group(ui, "tool-wall-construction", "Construcción", |ui| {
            row(
                ui,
                "Línea de referencia",
                "La línea que dibujas: cara exterior, eje o cara interior",
                |ui, w| {
                    self.wall_settings.reference = choice(
                        ui,
                        "tool-wall-ref",
                        w,
                        self.wall_settings.reference,
                        &REFERENCES,
                    );
                },
            );
            props::note(
                ui,
                "La cara exterior queda a la izquierda del sentido de dibujo.",
            );
        });
        self.layers_group(ui, self.wall_settings.wall_type);
    }

    fn slab_tool_properties(&mut self, ui: &mut Ui) {
        props::title(ui, "Herramienta Forjado · lo que vas a dibujar");
        let items = self.slab_type_items(ui.ctx());
        let mut chosen = None;
        let card = type_card(
            ui,
            "tool-slab",
            Icon::Slab,
            "Forjado",
            "IfcSlab",
            Some(self.slab_settings.slab_type),
            &items,
            &mut chosen,
            None,
        );
        if let Some(t) = chosen {
            self.slab_settings.slab_type = t;
        }
        if card.edit {
            self.edit_type(TypesTab::Slabs, self.slab_settings.slab_type.0);
        }
        group(ui, "tool-slab-method", "Método", |ui| {
            ui.add_space(2.0);
            #[derive(Clone, Copy, PartialEq)]
            enum Method {
                Polygon,
                Rectangle,
                Wand,
            }
            let current = match (self.slab_settings.wand, self.slab_tool.mode) {
                (true, _) => Method::Wand,
                (false, SlabMode::Rectangle) => Method::Rectangle,
                (false, SlabMode::Polygon) => Method::Polygon,
            };
            let mut method = current;
            if method_picker(
                ui,
                "slab-method",
                &mut method,
                &[
                    (Method::Polygon, Icon::GeoPolygon, "Polígono", "Esquina a esquina"),
                    (
                        Method::Rectangle,
                        Icon::GeoRect,
                        "Rectángulo",
                        "Dos esquinas opuestas (o la primera y su tamaño: 6000x4000)",
                    ),
                    (
                        Method::Wand,
                        Icon::GeoWand,
                        "Varita",
                        "Varita mágica: clic dentro de un recinto de muros (también con Espacio + clic)",
                    ),
                ],
            ) {
                self.slab_settings.wand = method == Method::Wand;
                match method {
                    Method::Polygon => self.slab_tool.set_mode(SlabMode::Polygon),
                    Method::Rectangle => self.slab_tool.set_mode(SlabMode::Rectangle),
                    Method::Wand => self.slab_tool.finish(),
                }
            }
            if !self.slab_settings.wand {
                ui.add_space(2.0);
                ui.checkbox(
                    &mut self.slab_settings.hole,
                    "Hueco en un forjado (escalera, patinillo)",
                )
                .on_hover_text(
                    "El contorno que dibujes se recorta del forjado de esta planta que lo contiene",
                );
            }
            ui.add_space(2.0);
            row(
                ui,
                "Contorno",
                "Hasta dónde llega el forjado de la varita mágica",
                |ui, w| {
                    self.slab_settings.contour = choice(
                        ui,
                        "tool-slab-contour",
                        w,
                        self.slab_settings.contour,
                        &[
                            (Contour::InteriorFace, "Cara interior"),
                            (Contour::WallAxis, "Eje de muros"),
                            (Contour::ExteriorFace, "Cara exterior"),
                        ],
                    );
                },
            );
        });
        group(ui, "tool-slab-constraints", "Restricciones", |ui| {
            row(
                ui,
                "Cara superior (mm)",
                "Altura de la cara superior sobre la planta; las capas cuelgan hacia abajo",
                |ui, w| {
                    if let Some(v) = length_field(
                        ui,
                        Id::unique("tool-slab-top"),
                        self.slab_settings.top_offset,
                        w - 8.0,
                    ) {
                        self.slab_settings.top_offset = v;
                    }
                },
            );
        });
    }

    fn opening_tool_properties(&mut self, ui: &mut Ui, door: bool) {
        props::title(
            ui,
            if door {
                "Herramienta Puerta · lo que vas a colocar"
            } else {
                "Herramienta Ventana · lo que vas a colocar"
            },
        );
        let items = self.opening_type_items(ui.ctx(), door);
        let current = if door {
            self.door_settings.opening_type
        } else {
            self.window_settings.opening_type
        };
        let mut chosen = None;
        let card = type_card(
            ui,
            if door { "tool-door" } else { "tool-window" },
            if door { Icon::Door } else { Icon::Window },
            if door { "Puerta" } else { "Ventana" },
            if door { "IfcDoor" } else { "IfcWindow" },
            Some(current),
            &items,
            &mut chosen,
            None,
        );
        if card.edit {
            let tab = if door {
                TypesTab::Doors
            } else {
                TypesTab::Windows
            };
            self.edit_type(tab, current.0);
        }
        let settings = if door {
            &mut self.door_settings
        } else {
            &mut self.window_settings
        };
        if let Some(t) = chosen {
            settings.opening_type = t;
        }
        let key = if door { "door" } else { "window" };
        let mut bad_setback = None;
        group(
            ui,
            &format!("tool-{key}-constraints"),
            "Restricciones",
            |ui| {
                row(
                    ui,
                    "Antepecho (mm)",
                    "Desde la planta hasta el hueco",
                    |ui, w| {
                        if let Some(v) =
                            length_field(ui, Id::unique(("tool-sill", key)), settings.sill, w - 8.0)
                        {
                            settings.sill = v;
                        }
                    },
                );
            },
        );
        group(
            ui,
            &format!("tool-{key}-construction"),
            "Construcción",
            |ui| {
                row(
                ui,
                "Mano",
                "UNE-EN 12519: mirando desde el lado hacia el que abre, bisagras a la izquierda o a la derecha",
                |ui, w| {
                    settings.hand = choice(ui, ("tool-hand", key), w, settings.hand, &HANDS);
                },
            );
                row(
                ui,
                "Retranqueo (mm)",
                "Desde la cara exterior del muro hasta la cara exterior del marco. Vacío: centrado en el muro",
                |ui, w| {
                    let shown = settings.setback.map(fmt_mm).unwrap_or_default();
                    if let Some(text) =
                        text_edit_committed(ui, Id::unique(("tool-setback", key)), &shown, w - 8.0)
                    {
                        if text.trim().is_empty() {
                            settings.setback = None;
                        } else if let Some(v) = parse_length(&text) {
                            settings.setback = Some(v);
                        } else {
                            bad_setback = Some(text);
                        }
                    }
                },
            );
                if settings.setback.is_none() {
                    props::note(ui, "Sin retranqueo: el marco se centra en el muro.");
                }
                props::note(
                    ui,
                    "Abre hacia el lado del muro en el que está el cursor al hacer clic.",
                );
            },
        );
        if let Some(text) = bad_setback {
            ui.ctx()
                .data_mut(|d| d.insert_temp(Id::unique("length-error"), text));
        }
    }

    fn dim_tool_properties(&mut self, ui: &mut Ui) {
        let marks = self.dim_settings.mode == DimMode::LevelMark;
        props::title(
            ui,
            if marks {
                "Herramienta Cota de nivel"
            } else {
                "Herramienta Cota"
            },
        );
        group(ui, "tool-dim", "Método", |ui| {
            let before = self.dim_settings.mode;
            row(ui, "Método", "", |ui, w| {
                self.dim_settings.mode = choice(
                    ui,
                    "tool-dim-mode",
                    w,
                    self.dim_settings.mode,
                    &[
                        (DimMode::Linear, "Lineal"),
                        (DimMode::WallOpenings, "Huecos de muro"),
                        (DimMode::LevelMark, "Cota de nivel"),
                    ],
                );
            });
            if self.dim_settings.mode != before {
                self.dim_tool.finish();
            }
            if self.dim_settings.mode == DimMode::Linear {
                row(ui, "Dirección", "", |ui, w| {
                    self.dim_settings.direction = choice(
                        ui,
                        "tool-dim-direction",
                        w,
                        self.dim_settings.direction,
                        &[
                            (DimDirection::Automatic, "Automática"),
                            (DimDirection::Horizontal, "Horizontal"),
                            (DimDirection::Vertical, "Vertical"),
                            (DimDirection::Aligned, "Alineada"),
                        ],
                    );
                });
            }
            props::note(
                ui,
                match self.dim_settings.mode {
                    DimMode::Linear => {
                        "Clic en cada punto, Intro, y clic donde va la línea de cota. En metros con dos decimales."
                    }
                    DimMode::WallOpenings => {
                        "Clic en un muro, del lado de la cara: acota sus extremos y las jambas de sus huecos."
                    }
                    DimMode::LevelMark => {
                        "Clic: cota del suelo terminado en ese punto, respecto al ±0,00."
                    }
                },
            );
        });
    }

    // ---- Nothing selected: the view and the project ---------------------------

    fn view_properties(&mut self, ui: &mut Ui) {
        match self.mode {
            ViewMode::Plan => self.plan_view_properties(ui),
            ViewMode::ThreeD => self.view_3d_properties(ui),
        }
        self.project_group(ui);
        ui.add_space(8.0);
        props::note(
            ui,
            "Selecciona un elemento para ver y cambiar sus propiedades, o elige una herramienta para ver lo que va a dibujar.",
        );
    }

    fn plan_view_properties(&mut self, ui: &mut Ui) {
        let level = self.ed.model().level(self.ed.level).cloned();
        let name = level.as_ref().map_or(String::new(), |l| l.name.clone());
        props::title(ui, &format!("Vista · {name}"));
        view_card(
            ui,
            Icon::Plan,
            &format!("Planta · {name}"),
            "Vista de planta",
        );
        group(ui, "view-graphics", "Gráficos", |ui| {
            row(
                ui,
                "Escala",
                "Grosores de línea, tramas, textos y símbolos de puertas y ventanas",
                |ui, w| {
                    let options: Vec<(u32, String)> = SCALES
                        .iter()
                        .map(|s| (*s as u32, format!("1:{s}")))
                        .collect();
                    let opts: Vec<(u32, &str)> =
                        options.iter().map(|(v, n)| (*v, n.as_str())).collect();
                    let v = choice(ui, "view-scale", w, self.plan_scale as u32, &opts);
                    self.set_scale(v as f64);
                },
            );
            row(
                ui,
                "Plano de corte (mm)",
                "Altura sobre la planta a la que se corta: lo de debajo se ve, lo de encima va discontinuo",
                |ui, w| {
                    if let Some(v) =
                        length_field(ui, Id::unique("view-cut"), self.cut_plane, w - 8.0)
                    {
                        if v > 0.0 {
                            self.cut_plane = v;
                        } else {
                            self.complain("El plano de corte debe estar por encima de la planta.");
                        }
                    }
                },
            );
            row(ui, "Líneas de referencia", "", |ui, _| {
                ui.checkbox(&mut self.show_reference_lines, "");
            });
            row(
                ui,
                "Líneas finas",
                "Todas las líneas a un píxel (apagado: grosor real a la escala de la vista)",
                |ui, _| {
                    ui.checkbox(&mut self.thin_lines, "");
                },
            );
            row(ui, "Rejilla", "Cuadrícula de 1 m y 5 m", |ui, _| {
                ui.checkbox(&mut self.show_grid, "");
            });
            row(
                ui,
                "Calco",
                "La planta de debajo en gris, para dibujar encima",
                |ui, _| {
                    ui.checkbox(&mut self.show_underlay, "");
                },
            );
        });
        if let Some(l) = level {
            group(ui, "view-level", "Planta", |ui| {
                row(ui, "Nombre", "", |ui, w| {
                    if let Some(new) =
                        text_edit_committed(ui, Id::unique(("level-name", l.id)), &l.name, w - 8.0)
                    {
                        let r = self.ed.rename_level(l.id, &new);
                        self.report(r, "Planta renombrada.");
                    }
                });
                row(
                    ui,
                    "Cota (mm)",
                    "Sobre el ±0,00; todo lo de la planta se mueve con ella",
                    |ui, w| {
                        if let Some(v) =
                            length_field(ui, Id::unique(("level-elev", l.id)), l.elevation, w - 8.0)
                        {
                            let r = self.ed.set_level_elevation(l.id, v);
                            self.report(r, "Cota de la planta cambiada.");
                        }
                    },
                );
                ui.horizontal(|ui| {
                    ui.add_space(4.0);
                    if ui
                        .button("Plantas…")
                        .on_hover_text("Añadir, renombrar, cambiar la cota o borrar plantas")
                        .clicked()
                    {
                        self.show_levels = true;
                    }
                });
            });
        }
    }

    fn view_3d_properties(&mut self, ui: &mut Ui) {
        props::title(ui, "Vista · 3D");
        view_card(ui, Icon::ThreeD, "Vista 3D", "Todo el modelo");
        group(ui, "view3d-camera", "Cámara", |ui| {
            ui.add_space(2.0);
            segmented(
                ui,
                "projection",
                &mut self.v3d.camera.perspective,
                &[
                    (true, "Perspectiva", ""),
                    (false, "Axonometría", "Proyección paralela"),
                ],
            );
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                ui.add_space(4.0);
                if ui.button("Ajustar").on_hover_text("Inicio").clicked() {
                    self.v3d.fit_requested = true;
                }
                if ui.button("Guardar imagen…").clicked() {
                    self.export_3d_picture();
                }
            });
        });
        self.section_box_group(ui);
    }

    fn project_group(&mut self, ui: &mut Ui) {
        let model = self.ed.model();
        let name = model.project_name.clone();
        let counts = (
            model.walls().count(),
            model.slabs().count(),
            model.openings().count(),
        );
        let guid = model.project_guid.to_string();
        group(ui, "project", "Proyecto", |ui| {
            row(ui, "Nombre", "", |ui, w| {
                if let Some(new) =
                    text_edit_committed(ui, Id::unique("project-name"), &name, w - 8.0)
                {
                    if new != name {
                        let r = self.ed.rename_project(&new);
                        self.report(r, "Proyecto renombrado.");
                    }
                }
            });
            text_row(ui, "Fichero", "", &self.ed.display_name());
            text_row(
                ui,
                "Contenido",
                "",
                &format!(
                    "{} muros · {} forjados · {} huecos",
                    counts.0, counts.1, counts.2
                ),
            );
            id_row(ui, "IfcProject", &guid);
        });
    }

    // ---- Elements ---------------------------------------------------------------

    fn layers_group(&self, ui: &mut Ui, t: WallTypeId) {
        let model = self.ed.model();
        let Some(t) = model.wall_type(t) else {
            return;
        };
        group(ui, "wall-layers", "Capas · de exterior a interior", |ui| {
            for l in &t.layers {
                let (name, rgb) = model
                    .material(l.material)
                    .map_or((String::new(), [200; 3]), |m| (m.name.clone(), m.cut_color));
                ui.horizontal(|ui| {
                    ui.add_space(8.0);
                    props::swatch(ui, rgb, 13.0);
                    ui.add_sized(
                        [52.0, 20.0],
                        egui::Label::new(egui::RichText::new(fmt_mm(l.thickness)).monospace()),
                    );
                    ui.add(egui::Label::new(name).truncate());
                });
            }
            text_row(
                ui,
                "Espesor total",
                "",
                &format!("{} mm", fmt_mm(t.thickness())),
            );
        });
    }

    /// The levels above `base`, bottom to top, with their names.
    pub(super) fn levels_above(&self, base: LevelId) -> Vec<(LevelId, String)> {
        let m = self.ed.model();
        let z = m.level(base).map_or(0.0, |l| l.elevation);
        m.levels()
            .into_iter()
            .filter(|l| l.elevation > z + 1e-9)
            .map(|l| (l.id, l.name.clone()))
            .collect()
    }

    /// The level of walls and slabs (Revit's "Restricción de base",
    /// ArchiCAD's home storey), as a list of every level: choosing another
    /// moves them there with their doors and windows. `current` is `None`
    /// when they are on different levels.
    pub(super) fn level_row(
        &mut self,
        ui: &mut Ui,
        salt: impl std::hash::Hash + std::fmt::Debug,
        label: &str,
        current: Option<LevelId>,
        items: &[Element],
    ) {
        let options: Vec<(Option<LevelId>, String)> = self
            .ed
            .model()
            .levels()
            .into_iter()
            .map(|l| {
                (
                    Some(l.id),
                    format!("{} ({})", l.name, fmt_level(l.elevation)),
                )
            })
            .collect();
        let picked = row(
            ui,
            label,
            "Cambia de planta conservando los desfases; las puertas y ventanas van con su muro. Un muro que llegaba a la planta de encima sigue llegando a la de encima de la nueva",
            |ui, w| {
                let mut opts: Vec<(Option<LevelId>, &str)> =
                    options.iter().map(|(v, n)| (*v, n.as_str())).collect();
                if current.is_none() {
                    opts.insert(0, (None, "Varias"));
                }
                choice(ui, salt, w, current, &opts)
            },
        );
        if let (Some(level), true) = (picked, picked != current) {
            let name = self
                .ed
                .model()
                .level(level)
                .map(|l| l.name.clone())
                .unwrap_or_default();
            match self.ed.set_elements_level(items, level) {
                Ok(n) if level != self.ed.level => self.say(format!(
                    "{n} elemento(s) pasan a «{name}»: ábrela en el navegador para verlos."
                )),
                Ok(_) => self.say(format!("Ahora en «{name}».")),
                Err(e) => self.complain(format!("No se puede cambiar de planta: {e}.")),
            }
        }
    }

    /// "Restricción superior" of selected walls: free, or up to a level
    /// above their base (keeping the offset they had).
    fn top_row(
        &mut self,
        ui: &mut Ui,
        salt: impl std::hash::Hash + std::fmt::Debug,
        walls: &[WallId],
        current: WallTop,
        above: &[(LevelId, String)],
    ) -> Option<WallTop> {
        let shown = match current {
            WallTop::Unconnected => None,
            WallTop::Level { level, .. } => Some(level),
        };
        let mut options: Vec<(Option<LevelId>, String)> = vec![(None, "Desconectada".to_string())];
        options.extend(
            above
                .iter()
                .map(|(id, n)| (Some(*id), format!("Hasta {n}"))),
        );
        if let (Some(l), false) = (shown, above.iter().any(|a| Some(a.0) == shown)) {
            // Tied to a level that is no longer above (shown anyway).
            let name = self
                .ed
                .model()
                .level(l)
                .map_or(String::new(), |l| l.name.clone());
            options.push((Some(l), format!("Hasta {name}")));
        }
        let picked = row(
            ui,
            "Restricción superior",
            "Hasta una planta (la coronación sigue sus cambios de cota) o desconectada (altura propia)",
            |ui, w| {
                let opts: Vec<(Option<LevelId>, &str)> =
                    options.iter().map(|(v, n)| (*v, n.as_str())).collect();
                choice(ui, salt, w, shown, &opts)
            },
        );
        if picked == shown {
            return None;
        }
        let offset = match current {
            WallTop::Level { offset, .. } => offset,
            WallTop::Unconnected => 0.0,
        };
        let top = match picked {
            None => WallTop::Unconnected,
            Some(level) => WallTop::Level { level, offset },
        };
        let r = self.ed.set_walls_top(walls, top);
        self.report(
            r,
            match top {
                WallTop::Unconnected => "Muro desconectado: conserva su altura.",
                WallTop::Level { .. } => {
                    "Restricción superior cambiada: la altura sale de las plantas."
                }
            },
        );
        Some(top)
    }

    fn top_row_many(
        &mut self,
        ui: &mut Ui,
        walls: &[WallId],
        current: Option<WallTop>,
        above: &[(LevelId, String)],
    ) {
        match current {
            Some(t) => {
                self.top_row(ui, "walls-top", walls, t, above);
            }
            None => {
                // Mixed: offer the choices without a current one.
                let mut options: Vec<(Option<Option<LevelId>>, String)> = vec![
                    (None, "(varias)".to_string()),
                    (Some(None), "Desconectada".into()),
                ];
                options.extend(
                    above
                        .iter()
                        .map(|(id, n)| (Some(Some(*id)), format!("Hasta {n}"))),
                );
                let picked = row(ui, "Restricción superior", "", |ui, w| {
                    let opts: Vec<(Option<Option<LevelId>>, &str)> =
                        options.iter().map(|(v, n)| (*v, n.as_str())).collect();
                    choice(ui, "walls-top-mixed", w, None, &opts)
                });
                if let Some(p) = picked {
                    let top = match p {
                        None => WallTop::Unconnected,
                        Some(level) => WallTop::Level { level, offset: 0.0 },
                    };
                    let r = self.ed.set_walls_top(walls, top);
                    self.report(
                        r,
                        "Restricción superior cambiada en los muros seleccionados.",
                    );
                }
            }
        }
    }

    fn wall_properties(&mut self, ui: &mut Ui, id: WallId) {
        let model = self.ed.model();
        let Some(w) = model.wall(id).cloned() else {
            return;
        };
        let thickness = model.wall_type(w.wall_type).map_or(0.0, |t| t.thickness());
        let openings = model.openings_in_wall(id).count();
        let same_type = self.same_type(Element::Wall(id));
        props::title(ui, "Muro · 1 seleccionado");
        let items = self.wall_type_items(ui.ctx());
        let mut chosen = None;
        let card = type_card(
            ui,
            "wall",
            Icon::Wall,
            "Muro",
            "IfcWall",
            Some(w.wall_type),
            &items,
            &mut chosen,
            Some(same_type.len()),
        );
        let walls = [id];
        if let Some(t) = chosen {
            let r = self.ed.set_walls_type(&walls, t);
            self.report(r, "Tipo de muro cambiado.");
        }
        if card.edit {
            self.edit_type(TypesTab::Walls, w.wall_type.0);
        }
        if card.select_all {
            self.select_instances(same_type, "muros");
        }
        let above = self.levels_above(w.level);
        group(ui, "wall-constraints", "Restricciones", |ui| {
            self.level_row(
                ui,
                ("wall-level", id),
                "Restricción de base",
                Some(w.level),
                &[Element::Wall(id)],
            );
            row(ui, "Desfase de base (mm)", "", |ui, wd| {
                if let Some(v) =
                    length_field(ui, Id::unique(("wall-b", id)), w.base_offset, wd - 8.0)
                {
                    let r = self.ed.set_walls_base_offset(&walls, v);
                    self.report(r, "Desfase de base cambiado.");
                }
            });
            self.top_row(ui, ("wall-top", id), &walls, w.top, &above);
            match w.top {
                WallTop::Level { level, offset } => {
                    row(
                        ui,
                        "Desfase superior (mm)",
                        "Negativo: la coronación queda por debajo de la planta",
                        |ui, wd| {
                            if let Some(v) =
                                length_field(ui, Id::unique(("wall-to", id)), offset, wd - 8.0)
                            {
                                let r = self
                                    .ed
                                    .set_walls_top(&walls, WallTop::Level { level, offset: v });
                                self.report(r, "Desfase superior cambiado.");
                            }
                        },
                    );
                    text_row(
                        ui,
                        "Altura (mm)",
                        "Sale de las plantas: cambia sola si cambian sus cotas. Para escribirla, desconecta el muro",
                        &format!("{} · de las plantas", fmt_mm(w.height)),
                    );
                }
                WallTop::Unconnected => {
                    row(ui, "Altura desconectada (mm)", "", |ui, wd| {
                        if let Some(v) =
                            length_field(ui, Id::unique(("wall-h", id)), w.height, wd - 8.0)
                        {
                            let r = self.ed.set_walls_height(&walls, v);
                            self.report(r, "Altura cambiada.");
                        }
                    });
                }
            }
        });
        group(ui, "wall-construction", "Construcción", |ui| {
            row(ui, "Línea de referencia", "", |ui, wd| {
                let r = choice(ui, ("wall-ref", id), wd, w.reference, &REFERENCES);
                if r != w.reference {
                    let res = self.ed.set_walls_reference(&walls, r);
                    self.report(
                        res,
                        "Línea de referencia cambiada (la línea se queda en su sitio).",
                    );
                }
            });
            row(
                ui,
                "Cara exterior",
                "Pasa la cara exterior al otro lado (la línea se queda en su sitio)",
                |ui, wd| {
                    if ui
                        .add_sized([wd - 8.0, 22.0], egui::Button::new("⇄ Invertir"))
                        .clicked()
                    {
                        let r = self.ed.flip_walls(&walls);
                        self.report(r, "Muro invertido.");
                    }
                },
            );
        });
        group(ui, "wall-dimensions", "Dimensiones", |ui| {
            row(
                ui,
                "Longitud (mm)",
                "Se cambia desde el inicio del muro",
                |ui, wd| {
                    if let Some(v) =
                        length_field(ui, Id::unique(("wall-len", id)), w.length(), wd - 8.0)
                    {
                        let r = self.ed.set_wall_length(id, v);
                        self.report(r, "Longitud cambiada desde el inicio del muro.");
                    }
                },
            );
            if let Some(arc) = w.arc() {
                row(
                    ui,
                    "Radio (mm)",
                    "Radio de la línea de referencia; los extremos no se mueven",
                    |ui, wd| {
                        if let Some(v) =
                            length_field(ui, Id::unique(("wall-radius", id)), arc.radius, wd - 8.0)
                        {
                            let r = self.ed.set_wall_radius(id, v);
                            self.report(r, "Radio cambiado.");
                        }
                    },
                );
                text_row(
                    ui,
                    "Ángulo",
                    "Lo que abarca el arco",
                    &format!("{:.1}°", arc.sweep.abs().to_degrees()).replace('.', ","),
                );
                row(
                    ui,
                    "Forma",
                    "Curvo; enderezarlo lo deja recto entre sus extremos",
                    |ui, wd| {
                        if ui
                            .add_sized([wd - 8.0, 22.0], egui::Button::new("Enderezar"))
                            .clicked()
                        {
                            let r = self.ed.set_walls_bulge(&[id], 0.0);
                            self.report(r, "Muro enderezado.");
                        }
                    },
                );
            }
            text_row(ui, "Espesor (mm)", "Del tipo", &fmt_mm(thickness));
            text_row(
                ui,
                "Inicio (m)",
                "",
                &format!("{}; {}", fmt_m(w.start.x), fmt_m(w.start.y)),
            );
            text_row(
                ui,
                "Fin (m)",
                "",
                &format!("{}; {}", fmt_m(w.end.x), fmt_m(w.end.y)),
            );
        });
        self.layers_group(ui, w.wall_type);
        group(ui, "wall-identity", "Datos de identidad", |ui| {
            text_row(ui, "Puertas y ventanas", "", &openings.to_string());
            id_row(ui, "GlobalId", &w.guid.to_string());
        });
    }

    fn opening_properties(&mut self, ui: &mut Ui, id: OpeningId) {
        let model = self.ed.model();
        let Some(o) = model.opening(id).cloned() else {
            return;
        };
        let Some(ot) = model.opening_type(o.opening_type).cloned() else {
            return;
        };
        let door = ot.is_door();
        let host = model
            .wall(o.wall)
            .map(|w| w.guid.to_string())
            .unwrap_or_default();
        let family = types_ui::family_label(model, ot.family);
        let operation = leaves_summary(&ot).unwrap_or_else(|| {
            match ot.kind {
                OpeningKind::Door(d) => match d {
                    archisim_core::DoorOperation::SingleSwing => "Abatible 1 hoja",
                    archisim_core::DoorOperation::DoubleSwing => "Abatible 2 hojas",
                    archisim_core::DoorOperation::Sliding => "Corredera",
                },
                OpeningKind::Window(w) => match w {
                    archisim_core::WindowOperation::Fixed => "Fija",
                    archisim_core::WindowOperation::SingleCasement => "Practicable 1 hoja",
                    archisim_core::WindowOperation::DoubleCasement => "Practicable 2 hojas",
                    archisim_core::WindowOperation::Sliding => "Corredera 2 hojas",
                },
                OpeningKind::Void => "Hueco sin carpintería",
            }
            .to_string()
        });
        let same_type = self.same_type(Element::Opening(id));
        props::title(
            ui,
            if door {
                "Puerta · 1 seleccionada"
            } else {
                "Ventana · 1 seleccionada"
            },
        );
        let items = self.opening_type_items(ui.ctx(), door);
        let mut chosen = None;
        let card = type_card(
            ui,
            "opening",
            if door { Icon::Door } else { Icon::Window },
            if door { "Puerta" } else { "Ventana" },
            if door { "IfcDoor" } else { "IfcWindow" },
            Some(o.opening_type),
            &items,
            &mut chosen,
            Some(same_type.len()),
        );
        if let Some(t) = chosen {
            let r = self.ed.set_opening_type(id, t);
            self.report(r, "Tipo cambiado.");
        }
        if card.edit {
            let tab = if door {
                TypesTab::Doors
            } else {
                TypesTab::Windows
            };
            self.edit_type(tab, o.opening_type.0);
        }
        if card.select_all {
            self.select_instances(same_type, if door { "puertas" } else { "ventanas" });
        }
        let model = self.ed.model();
        let level = model
            .wall(o.wall)
            .and_then(|w| model.level(w.level))
            .map(|l| l.name.clone())
            .unwrap_or_default();
        group(ui, "opening-constraints", "Restricciones", |ui| {
            text_row(
                ui,
                "Planta",
                "La de su muro: cambia de planta con él",
                &level,
            );
            row(ui, "Antepecho (mm)", "Desde la planta", |ui, w| {
                if let Some(v) = length_field(ui, Id::unique(("op-sill", id)), o.sill, w - 8.0) {
                    let r = self.ed.move_opening(id, o.along, v);
                    self.report(r, "Antepecho cambiado.");
                }
            });
            row(
                ui,
                "Posición (mm)",
                "Del centro del hueco, medida desde el inicio del muro",
                |ui, w| {
                    if let Some(v) =
                        length_field(ui, Id::unique(("op-along", id)), o.along, w - 8.0)
                    {
                        let r = self.ed.move_opening(id, v, o.sill);
                        self.report(r, "Posición cambiada.");
                    }
                },
            );
        });
        group(ui, "opening-construction", "Construcción", |ui| {
            text_row(ui, "Funcionamiento", "", &operation);
            let side = row(ui, "Abre hacia", "", |ui, w| {
                choice(ui, ("op-side", id), w, o.opens_to, &SIDES)
            });
            let hand = row(
                ui,
                "Mano",
                "UNE-EN 12519: mirando desde el lado hacia el que abre, bisagras a la izquierda o a la derecha",
                |ui, w| choice(ui, ("op-hand", id), w, o.hand, &HANDS),
            );
            if side != o.opens_to || hand != o.hand {
                let r = self.ed.set_opening_handing(id, side, hand);
                self.report(r, "Apertura cambiada.");
            }
        });
        group(ui, "opening-dimensions", "Dimensiones", |ui| {
            text_row(
                ui,
                "Hueco de obra (mm)",
                "Del tipo",
                &format!("{} × {}", fmt_mm(ot.width), fmt_mm(ot.height)),
            );
        });
        group(ui, "opening-identity", "Datos de identidad", |ui| {
            text_row(
                ui,
                "Marca de tipo",
                "",
                if ot.mark.is_empty() { "—" } else { &ot.mark },
            );
            text_row(ui, "Familia", "", &family);
            id_row(ui, "Muro anfitrión", &host);
            id_row(ui, "GlobalId", &o.guid.to_string());
        });
    }

    fn slab_properties(&mut self, ui: &mut Ui, id: SlabId) {
        let model = self.ed.model();
        let Some(s) = model.slab(id).cloned() else {
            return;
        };
        let thickness = model.slab_type(s.slab_type).map_or(0.0, |t| t.thickness());
        let same_type = self.same_type(Element::Slab(id));
        props::title(ui, "Forjado · 1 seleccionado");
        let items = self.slab_type_items(ui.ctx());
        let mut chosen = None;
        let card = type_card(
            ui,
            "slab",
            Icon::Slab,
            "Forjado",
            "IfcSlab",
            Some(s.slab_type),
            &items,
            &mut chosen,
            Some(same_type.len()),
        );
        if let Some(t) = chosen {
            let r = self.ed.set_slab_type(id, t);
            self.report(r, "Tipo de forjado cambiado.");
        }
        if card.edit {
            self.edit_type(TypesTab::Slabs, s.slab_type.0);
        }
        if card.select_all {
            self.select_instances(same_type, "forjados");
        }
        group(ui, "slab-constraints", "Restricciones", |ui| {
            self.level_row(
                ui,
                ("slab-level", id),
                "Planta",
                Some(s.level),
                &[Element::Slab(id)],
            );
            row(
                ui,
                "Cara superior (mm)",
                "Sobre la planta; las capas cuelgan hacia abajo",
                |ui, w| {
                    if let Some(v) =
                        length_field(ui, Id::unique(("slab-top", id)), s.top_offset, w - 8.0)
                    {
                        let r = self.ed.set_slab_offset(id, v);
                        self.report(r, "Desfase cambiado.");
                    }
                },
            );
        });
        group(ui, "slab-dimensions", "Dimensiones", |ui| {
            text_row(ui, "Canto (mm)", "Del tipo", &fmt_mm(thickness));
            text_row(
                ui,
                "Superficie",
                "Descontados los huecos",
                &fmt_area(s.area()),
            );
            text_row(ui, "Vértices", "", &s.outline.len().to_string());
        });
        if !s.holes.is_empty() {
            group(ui, "slab-holes", "Huecos", |ui| {
                for (k, h) in s.holes.iter().enumerate() {
                    let area = fmt_area(archisim_core::signed_area(h).abs());
                    row(
                        ui,
                        &format!("Hueco {} · {area}", k + 1),
                        "Escalera, patinillo…",
                        |ui, w| {
                            if ui
                                .add_sized([w - 8.0, 22.0], egui::Button::new("Quitar"))
                                .clicked()
                            {
                                let r = self.ed.remove_slab_hole(id, k);
                                self.report(r, "Hueco cerrado.");
                            }
                        },
                    );
                }
            });
        }
        group(ui, "slab-identity", "Datos de identidad", |ui| {
            id_row(ui, "GlobalId", &s.guid.to_string());
        });
    }

    fn many_properties(&mut self, ui: &mut Ui, many: &[Element]) {
        let walls: Vec<WallId> = many
            .iter()
            .filter_map(|e| match e {
                Element::Wall(id) => Some(*id),
                _ => None,
            })
            .collect();
        let count = |f: fn(&Element) -> bool| many.iter().filter(|e| f(e)).count();
        let openings = count(|e| matches!(e, Element::Opening(_)));
        let slabs = count(|e| matches!(e, Element::Slab(_)));
        let annotations = count(|e| matches!(e, Element::Dimension(_) | Element::Mark(_)));
        props::title(ui, &format!("{} elementos seleccionados", many.len()));
        group(ui, "many-summary", "Selección", |ui| {
            for (name, n) in [
                ("Muros", walls.len()),
                ("Puertas y ventanas", openings),
                ("Forjados", slabs),
                ("Cotas", annotations),
            ] {
                if n > 0 {
                    text_row(ui, name, "", &n.to_string());
                }
            }
        });
        let movable: Vec<Element> = many
            .iter()
            .copied()
            .filter(|e| matches!(e, Element::Wall(_) | Element::Slab(_)))
            .collect();
        if !movable.is_empty() {
            let model = self.ed.model();
            let levels: BTreeSet<LevelId> = movable
                .iter()
                .filter_map(|e| match e {
                    Element::Wall(id) => model.wall(*id).map(|w| w.level),
                    Element::Slab(id) => model.slab(*id).map(|s| s.level),
                    _ => None,
                })
                .collect();
            let current = (levels.len() == 1)
                .then(|| levels.iter().next().copied())
                .flatten();
            group(ui, "many-level", "Planta", |ui| {
                self.level_row(ui, "many-level-row", "Planta", current, &movable);
            });
        }
        if !walls.is_empty() && walls.len() == many.len() {
            let model = self.ed.model();
            let types: BTreeSet<WallTypeId> = walls
                .iter()
                .filter_map(|w| model.wall(*w))
                .map(|w| w.wall_type)
                .collect();
            let heights: Vec<f64> = walls
                .iter()
                .filter_map(|w| model.wall(*w))
                .map(|w| w.height)
                .collect();
            let same_height = heights.windows(2).all(|p| p[0] == p[1]);
            let current =
                (types.len() == 1).then(|| *types.iter().next().unwrap_or(&WallTypeId(0)));
            let items = self.wall_type_items(ui.ctx());
            let mut chosen = None;
            let card = type_card(
                ui,
                "walls",
                Icon::Wall,
                &format!("Muros ({})", walls.len()),
                "IfcWall",
                current,
                &items,
                &mut chosen,
                None,
            );
            if let Some(t) = chosen {
                let r = self.ed.set_walls_type(&walls, t);
                self.report(r, "Tipo cambiado en los muros seleccionados.");
            }
            if card.edit {
                if let Some(t) = current {
                    self.edit_type(TypesTab::Walls, t.0);
                }
            }
            let model = self.ed.model();
            let tops: Vec<WallTop> = walls
                .iter()
                .filter_map(|w| model.wall(*w))
                .map(|w| w.top)
                .collect();
            let bases: BTreeSet<LevelId> = walls
                .iter()
                .filter_map(|w| model.wall(*w))
                .map(|w| w.level)
                .collect();
            let same_top = tops.windows(2).all(|p| p[0] == p[1]);
            let base = bases.iter().next().copied().unwrap_or(self.ed.level);
            let above = self.levels_above(base);
            group(ui, "walls-constraints", "Restricciones", |ui| {
                if bases.len() == 1 {
                    let shown = if same_top {
                        tops.first().copied()
                    } else {
                        None
                    };
                    self.top_row_many(ui, &walls, shown, &above);
                }
                row(
                    ui,
                    if same_height {
                        "Altura (mm)"
                    } else {
                        "Altura (mm) · varias"
                    },
                    "Cambia la altura de todos los muros seleccionados",
                    |ui, w| {
                        let shown = heights.first().copied().unwrap_or(0.0);
                        if let Some(v) = length_field(ui, Id::unique("walls-h"), shown, w - 8.0) {
                            let r = self.ed.set_walls_height(&walls, v);
                            self.report(r, "Altura cambiada en los muros seleccionados.");
                        }
                    },
                );
                row(ui, "Cara exterior", "", |ui, w| {
                    if ui
                        .add_sized([w - 8.0, 22.0], egui::Button::new("⇄ Invertir"))
                        .clicked()
                    {
                        let r = self.ed.flip_walls(&walls);
                        self.report(r, "Muros invertidos.");
                    }
                });
            });
        }
        ui.add_space(8.0);
        ui.horizontal(|ui| {
            ui.add_space(4.0);
            if ui
                .button("🗑 Borrar selección")
                .on_hover_text("Supr")
                .clicked()
            {
                self.delete();
            }
        });
    }

    fn dimension_properties(&mut self, ui: &mut Ui, id: DimensionId) {
        let Some(d) = self.ed.model().dimension(id).cloned() else {
            return;
        };
        let values = d.values();
        let chained = values.len() > 1;
        props::title(ui, "Cota · 1 seleccionada");
        view_card(
            ui,
            Icon::Dimension,
            if chained {
                "Cota encadenada"
            } else {
                "Cota lineal"
            },
            "Anotación de la planta (no se exporta a IFC)",
        );
        let u = d.direction;
        let dir = if u.y.abs() < 1e-9 {
            "Horizontal".to_string()
        } else if u.x.abs() < 1e-9 {
            "Vertical".to_string()
        } else {
            format!("Alineada {}", fmt_angle(u.y.atan2(u.x)))
        };
        group(ui, "dim-values", "Dimensiones", |ui| {
            text_row(ui, "Dirección", "", &dir);
            text_row(
                ui,
                "Tramos (m)",
                "",
                &values
                    .iter()
                    .map(|v| format_dimension(*v))
                    .collect::<Vec<_>>()
                    .join(" · "),
            );
            text_row(ui, "Total (m)", "", &format_dimension(values.iter().sum()));
            text_row(ui, "Puntos", "", &d.points.len().to_string());
        });
        ui.add_space(4.0);
        props::note(
            ui,
            "Arrástrala para mover la línea de cota. No sigue a los muros al moverlos (cotas asociativas: fase 3).",
        );
    }

    fn mark_properties(&mut self, ui: &mut Ui, id: ElevationMarkId) {
        let model = self.ed.model();
        let Some(m) = model.elevation_mark(id).cloned() else {
            return;
        };
        let value = model.floor_elevation_at(m.level, m.point).unwrap_or(0.0);
        let base = model.level(m.level).map_or(0.0, |l| l.elevation);
        let level = model
            .level(m.level)
            .map(|l| format!("{} ({})", l.name, fmt_level(l.elevation)))
            .unwrap_or_default();
        props::title(ui, "Cota de nivel · 1 seleccionada");
        view_card(
            ui,
            Icon::Elevation,
            "Cota de nivel",
            "Lee el forjado más alto de la planta en ese punto",
        );
        group(ui, "mark-values", "Dimensiones", |ui| {
            text_row(ui, "Cota (m)", "", &format_elevation(value));
            text_row(ui, "Planta", "", &level);
            text_row(
                ui,
                "Punto (m)",
                "",
                &format!("{}; {}", fmt_m(m.point.x), fmt_m(m.point.y)),
            );
            text_row(ui, "Sobre la planta (mm)", "", &fmt_mm(value - base));
        });
    }
}

/// A header card for things without types (views, annotations).
pub(super) fn view_card_pub(ui: &mut Ui, icon: Icon, title: &str, subtitle: &str) {
    view_card(ui, icon, title, subtitle);
}

fn view_card(ui: &mut Ui, icon: Icon, title: &str, subtitle: &str) {
    let p = super::theme::Palette::current(ui.ctx());
    egui::Frame::new()
        .fill(p.raised)
        .stroke(egui::Stroke::new(1.0, p.line))
        .corner_radius(egui::CornerRadius::same(10))
        .inner_margin(egui::Margin::same(10))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.horizontal(|ui| {
                let (r, _) = ui.allocate_exact_size(Vec2::splat(40.0), Sense::hover());
                ui.painter()
                    .rect_filled(r, CornerRadius::same(8), p.accent_soft);
                super::icons::paint(ui.painter(), r.shrink(9.0), icon, p.accent);
                ui.vertical(|ui| {
                    ui.spacing_mut().item_spacing.y = 2.0;
                    ui.label(
                        egui::RichText::new(title)
                            .font(super::widgets::strong_font(13.5))
                            .color(p.text),
                    );
                    ui.add(
                        egui::Label::new(egui::RichText::new(subtitle).small().color(p.muted))
                            .wrap(),
                    );
                });
            });
        });
}
