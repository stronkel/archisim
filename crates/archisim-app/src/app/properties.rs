//! The properties palette, as Revit's. It shows, in this order of
//! preference:
//! - with a drawing tool active, what that tool will build (its type and
//!   its settings): it replaces ArchiCAD's info box;
//! - with a selection, the selected elements (type selector on top, then
//!   Restricciones, Construcción, Dimensiones and Datos de identidad);
//! - with nothing selected, the active view and the project.

use super::widgets::SwitchExt as _;
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
use super::widgets::segmented;
use super::*;
use crate::dim_tool::{DimDirection, DimMode};
use crate::editor::TopChoice;

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
        if self.team.is_some() {
            self.team_selection_strip(ui, &selection);
        }
        let drafts: Vec<archisim_core::DraftId> = selection
            .iter()
            .filter_map(|e| match e {
                Element::Draft(id) => Some(*id),
                _ => None,
            })
            .collect();
        if !drafts.is_empty() && drafts.len() == selection.len() {
            self.draft_properties(ui, &drafts);
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

            [Element::Section(id)] => self.section_properties(ui, *id),
            [Element::Roof(id)] => self.roof_properties(ui, *id),
            [Element::Dormer(id, i)] => self.dormer_properties(ui, *id, usize::from(*i)),
            [Element::Stair(id)] => self.stair_properties(ui, *id),
            [Element::Terrain(id)] => self.terrain_properties(ui, *id),
            [Element::CurtainWall(id)] => self.curtain_wall_properties(ui, *id),
            [Element::Railing(id)] => self.railing_properties(ui, *id),
            [Element::Object(id)] => self.object_properties(ui, *id),
            [Element::Room(id)] => self.room_properties(ui, *id),
            [Element::Tag(id)] => self.tag_properties(ui, *id),
            [Element::Footing(id)] => self.footing_properties(ui, *id),
            [Element::Grid(id)] => self.grid_properties(ui, *id),
            [Element::RefPlane(id)] => self.ref_plane_properties(ui, *id),
            [Element::Module(id)] => self.module_properties(ui, *id),
            [Element::Mass(id)] => self.mass_properties(ui, *id),
            [Element::Plot(id)] => self.plot_properties(ui, *id),
            [Element::Pool(id)] => self.pool_properties(ui, *id),
            [Element::Road(id)] => self.road_properties(ui, *id),
            [Element::Planting(id)] => self.planting_properties(ui, *id),
            [Element::Run(id)] => self.run_properties(ui, *id),
            [Element::Edge(id)] => self.edge_properties(ui, *id),
            [Element::Device(id)] => self.device_properties(ui, *id),
            many => self.many_properties(ui, many),
        }
        if let [e] = selection.as_slice() {
            self.discipline_group(ui, *e);
            self.user_params_group(ui, *e);
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
        // Exterior walls first, as in the types dialog.
        let mut list: Vec<_> = model.wall_types().collect();
        list.sort_by_key(|t| t.function != WallFunction::Exterior);
        list.into_iter()
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
                        "{} · {f} · {} capas",
                        crate::units::fmt_len(t.thickness()),
                        t.layers.len()
                    ),
                    thumb,
                    caption: "Corte en planta 1:20 · exterior arriba",
                    folder: types_ui::wall_folder_name(t.function).to_string(),
                }
            })
            .collect()
    }

    fn slab_type_items(&mut self, ctx: &egui::Context) -> Vec<TypeItem<SlabTypeId>> {
        let sig = self.picture_signature(false);
        let model = self.ed.model();
        let thumbs = &mut self.thumbs;
        let mut list: Vec<_> = model.slab_types().collect();
        list.sort_by_key(|t| match t.function {
            SlabFunction::Floor => 0,
            SlabFunction::BaseSlab => 1,
            SlabFunction::Roof => 2,
        });
        list.into_iter()
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
                    note: format!("canto {} · {f}", crate::units::fmt_len(t.thickness())),
                    thumb,
                    caption: "Capas 1:20 · cara superior arriba",
                    folder: types_ui::slab_folder_name(t.function).to_string(),
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
        let view = self.opening_card_view;
        let ids: Vec<OpeningTypeId> = self
            .ed
            .model()
            .opening_types()
            .filter(|t| t.is_door() == door)
            .map(|t| t.id)
            .collect();
        if view == CardView::ThreeD {
            self.thumbs3d.begin();
        }
        let mut out = Vec::new();
        for id in ids {
            let gpu = self.gpu.clone();
            let model = self.ed.model();
            let Some(t) = model.opening_type(id) else {
                continue;
            };
            let name = if t.mark.is_empty() {
                t.name.clone()
            } else {
                format!("{}  {}", t.mark, t.name)
            };
            // Planta · Alzado · 3D (concepto de interfaz del 24/10/2026):
            // in plan one window is a line like any other.
            let (thumb, caption) = match view {
                CardView::Plan => (
                    self.thumbs.get(ctx, format!("opening:{t:?}:{sig}"), || {
                        crate::type_preview::opening_model(model, t)
                    }),
                    "Planta 1:20 · exterior arriba",
                ),
                CardView::Elevation => (
                    self.thumbs.get_drawing(
                        ctx,
                        format!("elev:{}:{:?}:{sig}", t.id.0, t.values),
                        egui::vec2(260.0, 130.0),
                        || crate::type_drawing::opening_elevation(model, t),
                    ),
                    "Alzado desde el exterior",
                ),
                CardView::ThreeD => (
                    self.thumbs3d.get(
                        ctx,
                        gpu.as_ref(),
                        format!("open3d:{}:{:?}:{sig}", t.id.0, t.values),
                        || crate::type_preview::opening_model(model, t),
                    ),
                    "3D",
                ),
            };
            out.push(TypeItem {
                id: t.id,
                name,
                note: format!(
                    "{} × {} · {}",
                    fmt_mm(t.width),
                    crate::units::fmt_len(t.height),
                    types_ui::family_label(model, t.family)
                ),
                thumb,
                caption,
                folder: model
                    .family(t.family)
                    .map_or("Otras".to_string(), |f| f.name().to_string()),
            });
        }
        if view == CardView::ThreeD && self.thumbs3d.pending {
            ctx.request_repaint();
        }
        out
    }

    /// The switch over a door's or window's type card: its picture in
    /// plan, in elevation or in 3D.
    fn opening_view_switch(&mut self, ui: &mut Ui) {
        let mut v = self.opening_card_view;
        ui.allocate_ui(egui::vec2(ui.available_width(), 26.0), |ui| {
            super::widgets::segmented(
                ui,
                "opening-card-view",
                &mut v,
                &[
                    (CardView::Plan, "Planta", "La planta a 1:20"),
                    (CardView::Elevation, "Alzado", "Desde el exterior"),
                    (CardView::ThreeD, "3D", "En su trozo de muro"),
                ],
            );
        });
        self.opening_card_view = v;
        ui.add_space(4.0);
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
            Tool::Zone => self.room_tool_properties(ui),
            Tool::Section => self.section_tool_properties(ui),
            Tool::Roof => self.roof_tool_properties(ui),
            Tool::Stair => self.stair_tool_properties(ui),
            Tool::Terrain => self.terrain_tool_properties(ui),
            Tool::CurtainWall => self.curtain_tool_properties(ui),
            Tool::Railing => self.railing_tool_properties(ui),
            Tool::Object => self.object_tool_properties(ui),
            Tool::Grid => self.grid_tool_properties(ui),
            Tool::RefPlane => self.ref_plane_tool_properties(ui),
            Tool::Footing => self.footing_tool_properties(ui),
            Tool::Module => self.module_tool_properties(ui),
            Tool::Mass
            | Tool::PushPull
            | Tool::DivideFace
            | Tool::Deform
            | Tool::MassOffset
            | Tool::Tape
            | Tool::Paint => self.mass_tool_properties(ui),
            Tool::Plot => self.plot_tool_properties(ui),
            Tool::Pool => self.pool_tool_properties(ui),
            Tool::Road => self.road_tool_properties(ui),
            Tool::Planting => self.planting_tool_properties(ui),
            Tool::Brush => self.brush_tool_properties(ui),
            Tool::Pipe | Tool::Duct => self.run_tool_properties(ui),
            Tool::Device => self.device_tool_properties(ui),
            Tool::TeamZone => {}
            Tool::Cad => self.cad_tool_properties(ui),
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
        group(ui, "tool-slab-method", "Contorno", |ui| {
            if !self.slab_settings.wand {
                ui.add_space(2.0);
                ui.switch(
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
        self.opening_view_switch(ui);
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
        group(ui, "tool-dim", "Cota", |ui| {
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
            ViewMode::Section => self.section_view_properties(ui),
            ViewMode::Schedule => {}
            ViewMode::Sheet(id) => {
                self.sheet_properties(ui, id);
                return;
            }
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
        let saved = self
            .active_saved()
            .and_then(|id| self.ed.model().plan_view(id).cloned());
        match &saved {
            Some(v) => view_card(
                ui,
                Icon::Plan,
                &v.name,
                if v.kind == archisim_core::PlanKind::Ceiling {
                    "Planta de techos guardada"
                } else {
                    "Vista de planta guardada"
                },
            ),
            None => view_card(
                ui,
                Icon::Plan,
                &format!("Planta · {name}"),
                "Vista de planta",
            ),
        }
        self.view_actions_group(ui, saved.as_ref());
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
                ui.switch(&mut self.show_reference_lines, "");
            });
            row(
                ui,
                "Líneas finas",
                "Todas las líneas a un píxel (apagado: grosor real a la escala de la vista)",
                |ui, _| {
                    ui.switch(&mut self.thin_lines, "");
                },
            );
            row(ui, "Rejilla", "Cuadrícula de 1 m y 5 m", |ui, _| {
                ui.switch(&mut self.show_grid, "");
            });
            row(
                ui,
                "Calco",
                "La planta de debajo en gris, para dibujar encima",
                |ui, _| {
                    ui.switch(&mut self.show_underlay, "");
                },
            );
        });
        self.phase_view_group(ui);
        self.visibility_group(ui);
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
            let mut textures = self.v3d.textures;
            super::props::row(
                ui,
                "Texturas",
                "Ladrillo, madera, piedra… dibujados en las caras. «En imagen» son las mismas, sacadas de imágenes hechas una vez: más ligeras para tarjetas gráficas lentas (las de los Mac con Intel). «Ligeras» las dibuja más sencillas; sin texturas, colores lisos",
                |ui, w| {
                    egui::ComboBox::from_id_salt("textures-3d")
                        .width(w - 8.0)
                        .selected_text(textures.label())
                        .show_ui(ui, |ui| {
                            for m in crate::view3d::render::TextureMode::ALL {
                                ui.selectable_value(&mut textures, m, m.label());
                            }
                        });
                },
            );
            if textures != self.v3d.textures {
                self.v3d.set_textures(textures);
                self.prefs.textures_3d = Some(textures.key().to_string());
            }
            let mut quality = self.v3d.quality;
            super::props::row(ui, "Imagen", quality.hint(), |ui, w| {
                egui::ComboBox::from_id_salt("quality-3d")
                    .width(w - 8.0)
                    .selected_text(quality.name())
                    .show_ui(ui, |ui| {
                        for q in crate::view3d::look::Quality::ALL {
                            ui.selectable_value(&mut quality, q, q.name())
                                .on_hover_text(q.hint());
                        }
                    });
            });
            if quality != self.v3d.quality {
                self.v3d.quality = quality;
                self.prefs.quality_3d = Some(quality);
            }
        });
        self.render_group(ui);
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
            self.north_row(ui);
            self.sun_rows(ui);
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
        group(ui, "project-junctions", "Encuentros", |ui| {
            props::note(
                ui,
                "Qué capa se queda el sitio donde se encuentran muros y forjados: una barra por material.",
            );
            ui.horizontal(|ui| {
                ui.add_space(4.0);
                if ui.button("Prioridades de intersección…").clicked() {
                    self.show_priorities = true;
                }
            });
        });
        self.georeference_group(ui);
        self.usages_group(ui);
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
                &crate::units::fmt_len(t.thickness()),
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
        self.wall_tools_group(ui, id, &w);
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
            // Trimmed under a roof (straight walls).
            if w.arc().is_none() {
                let roofs: Vec<(Option<archisim_core::RoofId>, String)> =
                    std::iter::once((None, "No".to_string()))
                        .chain(self.ed.model().roofs().map(|r| {
                            let level = self
                                .ed
                                .model()
                                .level(r.level)
                                .map(|l| l.name.clone())
                                .unwrap_or_default();
                            (Some(r.id), format!("Cubierta {} ({level})", r.id.0))
                        }))
                        .collect();
                if roofs.len() > 1 {
                    let current = self.ed.model().wall_roof(id);
                    row(
                        ui,
                        "Bajo cubierta",
                        "El muro sube hasta la cara inferior de la cubierta (piñones incluidos)",
                        |ui, wd| {
                            let opts: Vec<_> =
                                roofs.iter().map(|(v, n)| (*v, n.as_str())).collect();
                            let picked = choice(ui, ("wall-roof", id), wd, current, &opts);
                            if picked != current {
                                let r = self.ed.set_walls_roof(&walls, picked);
                                self.report(r, "Recorte bajo cubierta cambiado.");
                            }
                        },
                    );
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
            // Revit's «Permitir unión» at each end (also with the right
            // click on an end).
            for (i, label) in [(0, "Unión al inicio"), (1, "Unión al final")] {
                row(
                    ui,
                    label,
                    "Apagada: ese extremo queda libre, cuadrado, aunque toque otro muro (No permitir unión)",
                    |ui, _| {
                        let mut on = w.allow_join[i];
                        ui.switch(&mut on, "");
                        if on != w.allow_join[i] {
                            let r = self
                                .ed
                                .transact("Unión de muros", |m| m.set_wall_join(id, i == 0, on));
                            self.report(
                                r,
                                if on {
                                    "Unión permitida."
                                } else {
                                    "Unión no permitida: el extremo queda libre."
                                },
                            );
                        }
                    },
                );
            }
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
        self.phase_group(ui, "wall-phase", &[Element::Wall(id)]);
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
                    archisim_core::DoorOperation::Folding => "Plegable",
                    archisim_core::DoorOperation::Pivot => "Pivotante",
                    archisim_core::DoorOperation::Overhead => "Garaje (seccional o basculante)",
                    archisim_core::DoorOperation::Rolling => "Enrollable",
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
        self.opening_view_switch(ui);
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
        // The family's instance parameters (a corner window's corner…), as
        // the recipe declares them.
        let instance: Vec<archisim_core::family::Param> = self
            .ed
            .model()
            .family(ot.family)
            .map(|f| {
                let def = f.def();
                def.params_of(archisim_core::ParamScope::Instance)
                    .filter(|p| def.is_visible(&p.name, &o.values))
                    .cloned()
                    .collect()
            })
            .unwrap_or_default();
        if !instance.is_empty() {
            let mut change: Option<(String, archisim_core::Value)> = None;
            group(ui, "opening-instance", "Parámetros del ejemplar", |ui| {
                for p in &instance {
                    if let Some(v) = instance_param_row(ui, id, p, o.values.get(&p.name)) {
                        change = Some((p.name.clone(), v));
                    }
                }
            });
            if let Some((name, v)) = change {
                let r = self.ed.set_opening_value(id, &name, v);
                self.report(r, "Parámetro cambiado.");
            }
        }
        group(ui, "opening-dimensions", "Dimensiones", |ui| {
            text_row(
                ui,
                "Hueco de obra (mm)",
                "Del tipo",
                &format!("{} × {}", fmt_mm(ot.width), fmt_mm(ot.height)),
            );
        });
        self.phase_group(ui, "opening-phase", &[Element::Opening(id)]);
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

    /// Inclination of a slab (ramps, sloping floors): on or off, the slope
    /// in per cent and which way it rises; the arrow's ends are dragged in
    /// plan for any other direction.
    fn slab_slope_group(&mut self, ui: &mut Ui, s: &archisim_core::Slab) {
        let id = s.id;
        group(ui, "slab-slope", "Pendiente", |ui| {
            let mut on = s.slope.is_some();
            row(ui, "Inclinado", "Rampa o forjado en pendiente", |ui, _| {
                if ui.switch(&mut on, "").changed() {
                    let slope = on.then(|| default_slope(s, Point2::new(1.0, 0.0), 0.05));
                    let r = self.ed.set_slab_slope(id, slope);
                    self.report(
                        r,
                        if on {
                            "Forjado inclinado: arrastra los extremos de su flecha para cambiar la dirección."
                        } else {
                            "Forjado horizontal."
                        },
                    );
                }
            });
            let Some(a) = s.slope else {
                return;
            };
            row(
                ui,
                "Pendiente (%)",
                "Sube en el sentido de la flecha; negativa, baja",
                |ui, w| {
                    let shown = fmt_percent(a.slope * 100.0);
                    if let Some(v) =
                        text_edit_committed(ui, Id::unique(("slab-slope-pc", id)), &shown, w - 8.0)
                    {
                        match v
                            .trim()
                            .trim_end_matches('%')
                            .trim()
                            .replace(',', ".")
                            .parse::<f64>()
                        {
                            Ok(pc) if pc.is_finite() => {
                                let r = self.ed.set_slab_slope(
                                    id,
                                    Some(archisim_core::SlabSlope {
                                        slope: pc / 100.0,
                                        ..a
                                    }),
                                );
                                self.report(r, "Pendiente cambiada.");
                            }
                            _ => self
                                .complain(format!("«{v}» no es un porcentaje: escribe 8 o 8,5.")),
                        }
                    }
                },
            );
            row(ui, "Sube hacia", "Dirección de la flecha", |ui, w| {
                let dirs: [(u32, &str, Point2); 4] = [
                    (0, "Este", Point2::new(1.0, 0.0)),
                    (1, "Norte", Point2::new(0.0, 1.0)),
                    (2, "Oeste", Point2::new(-1.0, 0.0)),
                    (3, "Sur", Point2::new(0.0, -1.0)),
                ];
                let u = a.direction().unwrap_or(Point2::new(1.0, 0.0));
                let current = dirs.iter().find(|d| d.2.dot(u) > 0.9999).map_or(9, |d| d.0);
                let mut opts: Vec<(u32, &str)> = dirs.iter().map(|d| (d.0, d.1)).collect();
                if current == 9 {
                    opts.push((9, "Otra (arrastrada)"));
                }
                let v = props::choice(ui, ("slab-slope-dir", id.0), w, current, &opts);
                if v != current {
                    if let Some(d) = dirs.iter().find(|d| d.0 == v) {
                        let r = self
                            .ed
                            .set_slab_slope(id, Some(default_slope(s, d.2, a.slope)));
                        self.report(r, "Dirección cambiada.");
                    }
                }
            });
            let (lo, hi) = s.top_range();
            text_row(
                ui,
                "Ángulo",
                "",
                &format!("{}°", fmt_angle_deg(a.slope.abs().atan())),
            );
            text_row(
                ui,
                "Cara superior",
                "Más baja y más alta, sobre la planta",
                &format!("{} … {}", fmt_mm(lo), crate::units::fmt_len(hi)),
            );
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
        if super::sketch_ui::edit_contour_button(ui) {
            self.start_sketch(crate::sketch::SketchTarget::Slab(id));
        }
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
        self.slab_slope_group(ui, &s);
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
        self.phase_group(ui, "slab-phase", &[Element::Slab(id)]);
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
        let drafts = count(|e| matches!(e, Element::Draft(_)));
        let rooms = count(|e| matches!(e, Element::Room(_)));
        let tags = count(|e| matches!(e, Element::Tag(_)));
        props::title(ui, &format!("{} elementos seleccionados", many.len()));
        group(ui, "many-summary", "Selección", |ui| {
            for (name, n) in [
                ("Muros", walls.len()),
                ("Puertas y ventanas", openings),
                ("Forjados", slabs),
                ("Cotas", annotations),
                ("Dibujo 2D", drafts),
                ("Locales", rooms),
                ("Etiquetas", tags),
            ] {
                if n > 0 {
                    text_row(ui, name, "", &n.to_string());
                }
            }
        });
        self.mass_operations(ui, many);
        self.phase_group(ui, "many-phase", many);
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
            let tied = d.anchors.iter().filter(|a| a.is_some()).count();
            text_row(
                ui,
                "Asociativa",
                "Los puntos asociados siguen a muros, huecos y forjados cuando cambian",
                &if tied == 0 {
                    "No".to_string()
                } else {
                    format!("{tied} de {} puntos", d.points.len())
                },
            );
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
pub(super) fn view_card(ui: &mut Ui, icon: Icon, title: &str, subtitle: &str) {
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

/// A slope arrow across a slab, through its middle, rising towards `u`
/// from one side of it to the other (the slab keeps its top at that side).
pub(super) fn default_slope(
    s: &archisim_core::Slab,
    u: Point2,
    slope: f64,
) -> archisim_core::SlabSlope {
    let along = |p: &Point2| p.dot(u);
    let (lo, hi) = s
        .outline
        .iter()
        .fold((f64::INFINITY, f64::NEG_INFINITY), |(a, b), p| {
            (a.min(along(p)), b.max(along(p)))
        });
    let c = archisim_core::centroid(&s.outline);
    let base = c - u * (along(&c) - lo);
    archisim_core::SlabSlope {
        from: base,
        to: base + u * (hi - lo).max(0.1),
        slope,
    }
}

/// A percentage with up to two decimals and a decimal comma.
fn fmt_percent(v: f64) -> String {
    let r = (v * 100.0).round() / 100.0;
    let t = format!("{r:.2}");
    let t = t.trim_end_matches('0').trim_end_matches('.');
    t.replace('.', ",")
}

/// An angle in degrees with one decimal and a decimal comma.
fn fmt_angle_deg(radians: f64) -> String {
    format!("{:.1}", radians.to_degrees()).replace('.', ",")
}

/// A row for one instance parameter of a door or window's family: its
/// name and a field for its kind. Returns the value chosen, if it changed.
fn instance_param_row(
    ui: &mut Ui,
    id: OpeningId,
    p: &archisim_core::family::Param,
    current: Option<&archisim_core::Value>,
) -> Option<archisim_core::Value> {
    use archisim_core::{ParamKind, Value};
    let label = match p.kind {
        ParamKind::Length => format!("{} (mm)", p.label),
        _ => p.label.clone(),
    };
    let field = Id::unique(("op-param", id, p.name.as_str()));
    row(
        ui,
        &label,
        "Del ejemplar: cada puerta o ventana tiene el suyo",
        |ui, w| match (&p.kind, current) {
            (ParamKind::Choice(options), Some(Value::Text(chosen))) => {
                let mut c = chosen.clone();
                egui::ComboBox::from_id_salt(field)
                    .width(w - 8.0)
                    .selected_text(capitalized(&c))
                    .show_ui(ui, |ui| {
                        for o in options {
                            ui.selectable_value(&mut c, o.clone(), capitalized(o));
                        }
                    });
                (c != *chosen).then_some(Value::Text(c))
            }
            (ParamKind::Bool, Some(Value::Bool(b))) => {
                let mut on = *b;
                ui.switch(&mut on, "").changed().then_some(Value::Bool(on))
            }
            (ParamKind::Length, Some(Value::Number(v))) => {
                length_field(ui, field, *v, w - 8.0).map(Value::Number)
            }
            (ParamKind::Number | ParamKind::Integer, Some(Value::Number(v))) => {
                let shown = format!("{v}").replace('.', ",");
                text_edit_committed(ui, field, &shown, w - 8.0)
                    .and_then(|t| t.trim().replace(',', ".").parse::<f64>().ok())
                    .map(Value::Number)
            }
            (ParamKind::Text, Some(Value::Text(t))) => {
                text_edit_committed(ui, field, t, w - 8.0).map(Value::Text)
            }
            _ => {
                ui.label(egui::RichText::new("—").weak());
                None
            }
        },
    )
}

/// "izquierda" → "Izquierda".
fn capitalized(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        Some(f) => f.to_uppercase().chain(c).collect(),
        None => String::new(),
    }
}

impl ArchiSimApp {
    /// Saved views, templates and «Visibilidad y gráficos» of the active
    /// plan.
    fn view_actions_group(&mut self, ui: &mut Ui, saved: Option<&archisim_core::PlanView>) {
        let templates: Vec<(archisim_core::ViewTemplateId, String)> = self
            .ed
            .model()
            .view_templates()
            .map(|t| (t.id, t.name.clone()))
            .collect();
        let mut apply = None;
        let mut rename = None;
        let mut duplicate = None;
        let mut vg = false;
        group(ui, "view-saved", "Vista", |ui| {
            if let Some(v) = saved {
                row(ui, "Nombre", "El de la vista guardada", |ui, w| {
                    if let Some(t) =
                        text_edit_committed(ui, Id::unique(("view-name", v.id.0)), &v.name, w - 8.0)
                    {
                        rename = Some((v.id, t));
                    }
                });
            }
            row(
                ui,
                "Plantilla",
                "Da a la vista el aspecto de una plantilla (Visibilidad y gráficos)",
                |ui, w| {
                    egui::ComboBox::from_id_salt("view-template")
                        .width(w - 8.0)
                        .selected_text("Aplicar…")
                        .show_ui(ui, |ui| {
                            for (id, name) in &templates {
                                if ui.selectable_label(false, name).clicked() {
                                    apply = Some(*id);
                                }
                            }
                        });
                },
            );
            ui.horizontal_wrapped(|ui| {
                ui.add_space(4.0);
                let r = ui
                    .button("Visibilidad y gráficos…")
                    .on_hover_text("Colores, rellenos, transparencia, tramas y grosor de cada categoría en esta vista (VG)");
                super::missions_ui::mark(ui.ctx(), "button:Visibilidad y gráficos…", r.rect);
                if r.clicked() {
                    vg = true;
                }
                if ui
                    .button("Guardar vista")
                    .on_hover_text("Otra vista de esta planta con su propio aspecto (albañilería, superficies…)")
                    .clicked()
                {
                    duplicate = Some(archisim_core::PlanKind::Floor);
                }
                if ui
                    .button("Planta de techos")
                    .on_hover_text("La planta vista hacia arriba: techos, luminarias, vigas")
                    .clicked()
                {
                    duplicate = Some(archisim_core::PlanKind::Ceiling);
                }
            });
        });
        if let Some(id) = apply {
            self.apply_template(id);
        }
        if let Some((id, t)) = rename {
            self.rename_view(id, &t);
        }
        if let Some(kind) = duplicate {
            self.duplicate_view(kind);
        }
        if vg {
            self.views.vg_open = true;
        }
    }
}

/// How the type card of a door or window shows it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CardView {
    Plan,
    #[default]
    Elevation,
    ThreeD,
}
