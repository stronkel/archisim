//! The «Maqueta» environment (`docs/MAQUETA.md`): the same model and the
//! same tools, shown as Los Sims or Paralives would. The 3D view fills
//! the window; a catalog at the bottom (Construir · Amueblar · Exterior)
//! replaces the tiles and the types; a card next to what is selected
//! replaces Properties; the top bar keeps only undo, the level, the walls
//! cut away and the grid. «Técnica · Maqueta» switches between the two
//! environments, keeping the selection.

use super::widgets::SwitchExt as _;
use archisim_core::{DormerKind, LevelId, OpeningTypeId, Point2};
use eframe::egui::{
    self, Align2, Color32, CornerRadius, FontId, Id, Pos2, Rect, Sense, Stroke, Ui, Vec2,
};

use super::glyphs::{self, Glyph};
use super::object_gallery::ObjectPick;
use super::theme::Palette;
use super::toolbox::Pick;
use super::view3d_ui::Cutaway;
use super::widgets::strong_font;
use super::{ArchiSimApp, Tool, ViewMode};
use crate::pick::Element;

/// The groups of the catalog.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CatalogGroup {
    #[default]
    Build,
    Furnish,
    Outside,
}

impl CatalogGroup {
    const ALL: [CatalogGroup; 3] = [Self::Build, Self::Furnish, Self::Outside];

    fn name(self) -> &'static str {
        match self {
            Self::Build => "Construir",
            Self::Furnish => "Amueblar",
            Self::Outside => "Exterior",
        }
    }

    /// Its themes, in order.
    fn themes(self) -> &'static [&'static str] {
        match self {
            Self::Build => &[
                "Habitaciones",
                "Puertas",
                "Ventanas",
                "Cubiertas",
                "Escaleras",
            ],
            Self::Furnish => &[
                "Salón",
                "Comedor",
                "Cocina",
                "Baño",
                "Dormitorio",
                "Despacho",
                "Decoración",
                "Deporte",
                "Hostelería y comercio",
                "Diseño",
                "Otros",
            ],
            Self::Outside => &[
                "Jardín y terraza",
                "Vegetación",
                "Mobiliario urbano",
                "Piscinas y deporte",
                "Rocas y troncos",
                "Gente y coches",
                "Terreno y caminos",
                "Formas",
            ],
        }
    }
}

/// The room a folder of the object library furnishes.
fn room_of(folder: &str) -> &'static str {
    match folder {
        "Sofás y sillones" | "Salón" | "Estanterías" => "Salón",
        "Mesas" | "Sillas" => "Comedor",
        "Cocina" | "Electrodomésticos" | "Taburetes" => "Cocina",
        "Accesorios de baño" | "Duchas y bañeras" | "Inodoros y bidés" | "Lavabos" => "Baño",
        "Camas" | "Dormitorio" | "Armarios" => "Dormitorio",
        "Despacho" | "Oficina" => "Despacho",
        "Deporte en casa" | "Gimnasio" => "Deporte",
        "Hostelería" | "Comercio" => "Hostelería y comercio",
        // What makes a house lived in, for any room.
        "Alfombras"
        | "Iluminación"
        | "Plantas de interior"
        | "Cortinas"
        | "Decoración"
        | "Cuadros y espejos"
        | "Cojines y mantas" => "Decoración",
        // The manufacturers' pieces each user keeps in their library.
        "Diseño" => "Diseño",
        // In «Exterior».
        "Jardín y terraza" => "Jardín y terraza",
        "Piscinas" | "Pistas deportivas" | "Parque infantil" => "Piscinas y deporte",
        _ => "Otros",
    }
}

/// The kind of door or window a family makes, for the catalog's chips.
fn opening_kind(family: &str) -> String {
    let f = family.to_lowercase();
    let k = if f.contains("empotrada") {
        "Empotradas"
    } else if f.contains("corredera") {
        "Correderas"
    } else if f.contains("guillotina") {
        "Guillotina"
    } else if f.contains("plegable") {
        "Plegables"
    } else if f.contains("pivotante") {
        "Pivotantes"
    } else if f.contains("garaje") {
        "Garaje"
    } else if f.contains("hueco") {
        "Huecos"
    } else if f.contains("fij") {
        "Fijas"
    } else if f.contains("abatible") {
        "Abatibles"
    } else if f.contains("practicable") {
        "Practicables"
    } else {
        return family.to_string();
    };
    k.to_string()
}

/// The kind of a piece of the library within its room, for the catalog's
/// chips: a kitchen's by what it is (base, wall, tall units, hoods…), the
/// rest by the library's folder.
fn object_kind(family: &str, folder: &str) -> String {
    match family {
        "Módulo bajo de cocina" => "Bajos",
        "Mueble alto de cocina" => "Altos",
        "Columna de cocina" | "Frigorífico" => "Columnas",
        "Campana extractora" => "Campanas",
        "Encimera de cocina" => "Cocinas enteras",
        "Isla de cocina" => "Islas",
        "Lavadora" => "Electrodomésticos",
        "Cosas de la encimera" => "Sobre la encimera",
        _ => return folder.to_string(),
    }
    .to_string()
}

/// The order of the kitchen's kinds (the others keep theirs, after).
fn kind_order(k: &str) -> usize {
    [
        "Bajos",
        "Altos",
        "Columnas",
        "Campanas",
        "Islas",
        "Sobre la encimera",
        "Electrodomésticos",
        "Cocinas enteras",
        "Practicables",
        "Abatibles",
        "Correderas",
        "Guillotina",
        "Fijas",
    ]
    .iter()
    .position(|x| *x == k)
    .unwrap_or(99)
}

/// A kind of stair on the catalog: its shape, its turns and what carries
/// it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StairCard {
    pub preset: archisim_core::StairPreset,
    pub winders: bool,
    pub structure: archisim_core::StairStructure,
    /// Landings split at 45°.
    pub split: bool,
}

/// What a card of the catalog does.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CatalogAct {
    /// Walls by a rectangle, with their floor and room (Los Sims' room).
    Room,
    Wall,
    /// A floor inside walls (the magic wand).
    Floor,
    /// A room ("local") inside walls.
    Zone,
    Door(OpeningTypeId),
    Window(OpeningTypeId),
    Roof(crate::editor::RoofKind),
    Dormer(DormerKind),
    Stair(StairCard),
    /// A piece of the library in the Maqueta's collection, by its name.
    Collected(&'static str),
    Railing,
    Object(ObjectPick),
    Planting,
    Road,
    /// A pool built in the ground.
    Pool,
    Terrain,
    /// An Exterior brush (MQ3).
    /// The Exterior brush.
    Brush,
    Volume,
    Form(crate::editor::MassMode),
}

/// One card of the catalog.
struct Card {
    act: CatalogAct,
    name: String,
    glyph: Glyph,
    thumb: Option<super::thumbs::Thumb>,
    /// The colours of the collection the piece is shown in.
    swatches: Vec<Color32>,
    /// Its kind within the theme (the third level of the catalog's tree:
    /// «Bajos», «Correderas»…); empty when the theme has no kinds.
    sub: String,
}

/// Height of the catalog at the bottom.
const CATALOG_HEIGHT: f32 = 204.0;
/// Size of a card.
const CARD: Vec2 = Vec2::new(112.0, 108.0);
/// Spacing of the ground grid.
pub const GRID_STEP: f64 = 0.5;

impl ArchiSimApp {
    /// Enters or leaves the Maqueta: in, the 3D view (the technical view
    /// is kept to come back to); out, back to it.
    pub(super) fn set_maquette(&mut self, on: bool) {
        if on == self.maquette {
            return;
        }
        self.maquette = on;
        if on {
            self.maquette_return = Some(self.active_tab);
            if self.mode != ViewMode::ThreeD {
                self.set_mode(ViewMode::ThreeD);
            }
            if self.maquette_first {
                // As in Los Sims: the walls in front cut away and the
                // grid on.
                self.maquette_first = false;
                self.v3d.cutaway = Cutaway::Front;
                self.ground_grid = true;
            }
            self.say("Maqueta: elige abajo qué poner y pincha en el 3D. Técnica, arriba a la derecha, vuelve a la ventana de siempre con lo mismo seleccionado.");
        } else if let Some(t) = self.maquette_return.take() {
            if t < self.tabs.len() {
                self.activate_tab(t);
            }
        }
    }

    /// The two buttons «Técnica · Maqueta», in the same place in both.
    pub(super) fn workspace_switch(&mut self, ui: &mut Ui) {
        let p = Palette::current(ui.ctx());
        ui.spacing_mut().item_spacing.x = 0.0;
        for (on, name, hint) in [
            (
                true,
                "Maqueta",
                "El 3D a pantalla con un catálogo, como Los Sims (F4)",
            ),
            (
                false,
                "Técnica",
                "La ventana de siempre: disciplinas, Propiedades, planta y planos",
            ),
        ] {
            let active = self.maquette == on;
            let text = egui::RichText::new(name)
                .font(strong_font(12.0))
                .color(if active { Color32::WHITE } else { p.text });
            let b = egui::Button::new(text)
                .fill(if active { p.accent } else { p.panel })
                .stroke(Stroke::new(1.0, p.line));
            if ui.add(b).on_hover_text(hint).clicked() {
                self.set_maquette(on);
            }
        }
    }

    /// The whole window in the Maqueta.
    pub(super) fn maquette_frame(&mut self, ui: &mut Ui) {
        let p = Palette::current(ui.ctx());
        let bar = |fill| {
            egui::Frame::new()
                .fill(fill)
                .inner_margin(egui::Margin::symmetric(10, 4))
                .stroke(Stroke::new(1.0, p.line))
        };
        if self.mode != ViewMode::ThreeD {
            self.set_mode(ViewMode::ThreeD);
        }
        egui::Panel::top("maquette-top")
            .frame(bar(p.panel))
            .show(ui, |ui| self.maquette_top_bar(ui));
        egui::Panel::bottom("maquette-catalog")
            .resizable(false)
            .exact_size(CATALOG_HEIGHT)
            .frame(bar(p.panel))
            .show(ui, |ui| self.maquette_catalog(ui));
        egui::CentralPanel::no_frame().show(ui, |ui| {
            ui.spacing_mut().item_spacing.y = 0.0;
            self.view_3d(ui);
        });
        let ctx = ui.ctx().clone();
        let bottom = ctx.content_rect().bottom() - CATALOG_HEIGHT;
        self.object_bar(&ctx, bottom);
        self.maquette_more(&ctx);
        let top = ctx.content_rect().top() + 44.0;
        self.brush_panel_maquette(&ctx, top, bottom);
    }

    /// Walking as in a game: W A S D forwards, left, back and right along
    /// the ground, Q down and E up, faster with Mayús; held, it keeps
    /// going. The letters are the walk's here, not the tools'.
    pub(super) fn maquette_walk(&mut self, ctx: &egui::Context) {
        if crate::app::typing_in_text(ctx) {
            return;
        }
        use egui::Key;
        let walk_keys = [Key::W, Key::A, Key::S, Key::D, Key::Q, Key::E];
        let (held, fast, dt) = ctx.input_mut(|i| {
            // Ctrl+S and the like are not walking.
            let plain = !i.modifiers.command && !i.modifiers.ctrl && !i.modifiers.alt;
            let held = walk_keys.map(|k| plain && i.key_down(k));
            // The tools do not see these letters.
            i.events.retain(|e| match e {
                egui::Event::Text(t) => !(plain && t.chars().all(|c| "wasdqeWASDQE".contains(c))),
                egui::Event::Key { key, modifiers, .. } => {
                    !(walk_keys.contains(key)
                        && !modifiers.command
                        && !modifiers.ctrl
                        && !modifiers.alt)
                }
                _ => true,
            });
            (held, i.modifiers.shift, i.stable_dt.clamp(0.0, 0.1))
        });
        if !held.iter().any(|h| *h) {
            return;
        }
        let axis = |plus: bool, minus: bool| f32::from(u8::from(plus)) - f32::from(u8::from(minus));
        let forward = axis(held[0], held[2]);
        let right = axis(held[3], held[1]);
        let up = axis(held[5], held[4]);
        // Metres a second: more from afar, so it feels the same at any zoom.
        let speed = self.v3d.camera.distance.clamp(3.0, 200.0) * 0.6 * if fast { 3.0 } else { 1.0 };
        let step = speed * dt;
        self.v3d
            .camera
            .walk(forward * step, right * step, up * step * 0.5);
        ctx.request_repaint();
    }

    fn maquette_top_bar(&mut self, ui: &mut Ui) {
        let p = Palette::current(ui.ctx());
        ui.horizontal(|ui| {
            ui.set_min_height(32.0);
            ui.label(
                egui::RichText::new("ArchiSim")
                    .font(strong_font(14.0))
                    .color(p.text),
            );
            ui.add_space(8.0);
            if ui
                .add_enabled(
                    self.ed.undo_label().is_some(),
                    egui::Button::new("↶ Deshacer"),
                )
                .clicked()
            {
                self.undo();
            }
            if ui
                .add_enabled(
                    self.ed.redo_label().is_some(),
                    egui::Button::new("↷ Rehacer"),
                )
                .clicked()
            {
                self.redo();
            }
            ui.separator();
            // The level: down and up.
            let levels: Vec<(LevelId, String, f64)> = {
                let mut v: Vec<_> = self
                    .ed
                    .model()
                    .levels()
                    .iter()
                    .map(|l| (l.id, l.name.clone(), l.elevation))
                    .collect();
                v.sort_by(|a, b| a.2.total_cmp(&b.2));
                v
            };
            let at = levels
                .iter()
                .position(|l| l.0 == self.ed.level)
                .unwrap_or(0);
            if ui
                .add_enabled(at > 0, egui::Button::new("▼"))
                .on_hover_text("Planta de abajo")
                .clicked()
            {
                self.switch_level(levels[at - 1].0);
            }
            ui.label(
                egui::RichText::new(levels.get(at).map_or("", |l| l.1.as_str()))
                    .font(strong_font(12.5))
                    .color(p.text),
            );
            if ui
                .add_enabled(at + 1 < levels.len(), egui::Button::new("▲"))
                .on_hover_text("Planta de arriba")
                .clicked()
            {
                self.switch_level(levels[at + 1].0);
            }
            ui.separator();
            self.cutaway_buttons(ui);
            ui.separator();
            ui.switch(&mut self.ground_grid, "Cuadrícula")
                .on_hover_text(
                "Cuadrícula de 50 cm en el suelo de la planta, y los clics se ajustan a ella (G)",
            );
            ui.separator();
            self.look_menus(ui);
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                self.workspace_switch(ui);

                ui.add_space(10.0);
                if ui
                    .button("Encuadrar")
                    .on_hover_text("Todo el modelo a la vista (Inicio)")
                    .clicked()
                {
                    self.v3d.fit_requested = true;
                }
            });
        });
    }

    /// The picture's quality and its light (day, evening or night), as
    /// two short lists: the Maqueta's bar has little room.
    fn look_menus(&mut self, ui: &mut Ui) {
        use crate::view3d::look::{Light, Quality};
        egui::ComboBox::from_id_salt("maquette-quality")
            .selected_text(format!("Imagen: {}", self.v3d.quality.name()))
            .width(120.0)
            .show_ui(ui, |ui| {
                for q in Quality::ALL {
                    if ui
                        .selectable_label(self.v3d.quality == q, q.name())
                        .on_hover_text(q.hint())
                        .clicked()
                    {
                        self.v3d.quality = q;
                        self.prefs.quality_3d = Some(q);
                    }
                }
                ui.separator();
                if ui
                    .button("Captura grande…")
                    .on_hover_text("Guarda la vista tal cual en una imagen de 3840 píxeles de ancho (PNG), para un render con IA, D5 o Twinmotion")
                    .clicked()
                {
                    ui.close();
                    self.big_capture();
                }
            });
        egui::ComboBox::from_id_salt("maquette-light")
            .selected_text(format!("Luz: {}", self.v3d.light.name()))
            .width(110.0)
            .show_ui(ui, |ui| {
                for l in Light::ALL {
                    if ui
                        .selectable_label(self.v3d.light == l, l.name())
                        .on_hover_text(l.hint())
                        .clicked()
                    {
                        self.v3d.light = l;
                    }
                }
            });
    }

    /// Walls up, cut away or low (in the 3D view of both environments).
    pub(super) fn cutaway_buttons(&mut self, ui: &mut Ui) {
        ui.spacing_mut().item_spacing.x = 2.0;
        for c in Cutaway::ALL {
            let hint = match c {
                Cutaway::Up => "Todo el edificio",
                Cutaway::Front => "Los muros entre la cámara y el centro de la vista bajan a 30 cm; los del fondo quedan enteros. Sigue a la cámara al girar. Los muebles no se cortan",
                Cutaway::Cut => "Corte a 1,20 m sobre la planta: se ve el interior; las plantas de arriba y la cubierta se ocultan. Los muebles no se cortan",
                Cutaway::Low => "Corte a 30 cm: la planta en 3D. Los muebles no se cortan",
            };
            if ui
                .selectable_label(self.v3d.cutaway == c, c.name())
                .on_hover_text(hint)
                .clicked()
            {
                self.v3d.cutaway = c;
            }
        }
        // What the cut walls show (the same choice as the section box's).
        if self.v3d.cutaway != Cutaway::Up {
            ui.separator();
            super::view3d_ui::caps_choice(ui, &mut self.v3d.caps);
        }
    }

    /// The names on the cards of a theme (for the tests).
    #[cfg(test)]
    pub(super) fn catalog_cards_pub(
        &mut self,
        ctx: &egui::Context,
        group: CatalogGroup,
        theme: &str,
    ) -> Vec<String> {
        self.catalog_cards(ctx, group, theme)
            .into_iter()
            .map(|c| c.name)
            .collect()
    }

    /// The kinds (third level) of a theme, as its chips show them.
    #[cfg(test)]
    pub(super) fn catalog_kinds_pub(
        &mut self,
        ctx: &egui::Context,
        group: CatalogGroup,
        theme: &str,
    ) -> Vec<String> {
        let mut v: Vec<String> = Vec::new();
        for c in self.catalog_cards(ctx, group, theme) {
            if !c.sub.is_empty() && !v.contains(&c.sub) {
                v.push(c.sub);
            }
        }
        v
    }

    /// The cards of a theme.
    fn catalog_cards(
        &mut self,
        ctx: &egui::Context,
        group: CatalogGroup,
        theme: &str,
    ) -> Vec<Card> {
        use crate::editor::{MassMode, RoofKind};
        let card = |act, name: &str, glyph| Card {
            act,
            name: name.to_string(),
            glyph,
            thumb: None,
            swatches: Vec::new(),
            sub: String::new(),
        };
        let sig = self.picture_signature(false);
        let openings = |app: &mut Self, door: bool| -> Vec<Card> {
            let model = app.ed.model();
            let thumbs = &mut app.thumbs;
            model
                .opening_types()
                .filter(|t| t.is_door() == door)
                .map(|t| {
                    // In elevation: in plan, one window is a line like any
                    // other (concepto de interfaz del 24/10/2026).
                    let key = format!("maqueta:elev:{}:{:?}:{sig}", t.id.0, t.values);
                    let thumb = thumbs.get_drawing(ctx, key, CARD - Vec2::new(16.0, 46.0), || {
                        crate::type_drawing::opening_elevation(model, t)
                    });
                    Card {
                        act: if door {
                            CatalogAct::Door(t.id)
                        } else {
                            CatalogAct::Window(t.id)
                        },
                        name: t.name.clone(),
                        glyph: if door { Glyph::Door } else { Glyph::Window },
                        thumb,
                        swatches: Vec::new(),
                        sub: opening_kind(
                            model.family(t.family).map_or("", |f| f.def().name.as_str()),
                        ),
                    }
                })
                .collect()
        };
        match (group, theme) {
            (CatalogGroup::Build, "Habitaciones") => vec![
                card(CatalogAct::Room, "Habitación", Glyph::Room),
                card(CatalogAct::Wall, "Muro", Glyph::Wall),
                card(CatalogAct::Floor, "Suelo", Glyph::Slab),
                card(CatalogAct::Zone, "Local", Glyph::Room),
            ],
            (CatalogGroup::Build, "Puertas") => openings(self, true),
            (CatalogGroup::Build, "Ventanas") => openings(self, false),
            (CatalogGroup::Build, "Cubiertas") => vec![
                card(
                    CatalogAct::Roof(RoofKind::Gable),
                    "A dos aguas",
                    Glyph::Roof,
                ),
                card(
                    CatalogAct::Roof(RoofKind::Hip),
                    "A cuatro aguas",
                    Glyph::Roof,
                ),
                card(CatalogAct::Roof(RoofKind::Shed), "A un agua", Glyph::Roof),
                card(
                    CatalogAct::Dormer(DormerKind::Gable),
                    "Buhardilla a dos aguas",
                    Glyph::Roof,
                ),
                card(
                    CatalogAct::Dormer(DormerKind::Shed),
                    "Buhardilla a un agua",
                    Glyph::Roof,
                ),
                card(
                    CatalogAct::Dormer(DormerKind::Flat),
                    "Buhardilla plana",
                    Glyph::Roof,
                ),
                card(
                    CatalogAct::Dormer(DormerKind::Skylight),
                    "Ventana de cubierta",
                    Glyph::Window,
                ),
            ],
            (CatalogGroup::Build, "Escaleras") => {
                use archisim_core::{StairPreset as P, StairStructure as S};
                let stair = |preset, winders, structure, name| {
                    card(
                        CatalogAct::Stair(StairCard {
                            preset,
                            winders,
                            structure,
                            split: false,
                        }),
                        name,
                        Glyph::Stair,
                    )
                };
                vec![
                    stair(P::Straight, false, S::Slab, "Recta"),
                    stair(P::LShape, false, S::Slab, "En L con meseta"),
                    card(
                        CatalogAct::Stair(StairCard {
                            preset: P::LShape,
                            winders: false,
                            structure: S::Slab,
                            split: true,
                        }),
                        "En L con meseta partida",
                        Glyph::Stair,
                    ),
                    stair(P::LShape, true, S::Slab, "En L compensada"),
                    stair(P::UShape, false, S::Slab, "En U con meseta"),
                    stair(P::UShape, true, S::Slab, "En U compensada"),
                    stair(P::Spiral, false, S::Newel, "De caracol"),
                    stair(P::Straight, false, S::Stringers, "De zancas"),
                    stair(P::Straight, false, S::Spine, "De zanca central"),
                    stair(P::Straight, false, S::Cantilever, "De peldaños volados"),
                    stair(P::Free, false, S::Slab, "Libre"),
                    card(CatalogAct::Railing, "Barandilla", Glyph::Railing),
                ]
            }
            (CatalogGroup::Outside, "Terreno y caminos") => vec![
                card(CatalogAct::Brush, "Pincel", Glyph::Brush),
                card(CatalogAct::Terrain, "Terreno", Glyph::Terrain),
                card(CatalogAct::Road, "Camino o calle", Glyph::Road),
                card(CatalogAct::Pool, "Piscina", Glyph::Pool),
                card(CatalogAct::Planting, "Plantación", Glyph::Planting),
            ],
            (CatalogGroup::Outside, "Formas") => vec![
                card(CatalogAct::Volume, "Volumen", Glyph::Volume),
                card(CatalogAct::Form(MassMode::Sphere), "Esfera", Glyph::Sphere),
                card(
                    CatalogAct::Form(MassMode::Cylinder),
                    "Cilindro",
                    Glyph::Cylinder,
                ),
                card(CatalogAct::Form(MassMode::Cone), "Cono", Glyph::Cone),
                card(
                    CatalogAct::Form(MassMode::Revolution),
                    "Revolución",
                    Glyph::Revolution,
                ),
            ],
            (CatalogGroup::Furnish, _) | (CatalogGroup::Outside, "Jardín y terraza")
                if self.collection.is_some() =>
            {
                // The library's pieces in the collection, in its colours.
                let c = self.collection.expect("checked");
                self.collected_items(ctx, c)
                    .into_iter()
                    .filter(|i| match group {
                        CatalogGroup::Furnish => {
                            matches!(i.category, "Mobiliario" | "Sanitarios")
                                && room_of(&i.folder) == theme
                        }
                        _ => i.folder == "Jardín y terraza",
                    })
                    .map(|i| Card {
                        act: CatalogAct::Collected(i.name),
                        name: i.name.to_string(),
                        glyph: Glyph::Object,
                        thumb: i.thumb,
                        swatches: super::collection_ui::colours(c),
                        sub: i.folder.clone(),
                    })
                    .collect()
            }
            _ => {
                // Objects of the library and the project, by room.
                let items = self.object_items_pub(ctx);
                items
                    .into_iter()
                    .filter(|i| match group {
                        CatalogGroup::Furnish => {
                            matches!(i.category, "Mobiliario" | "Sanitarios")
                                && room_of(&i.folder) == theme
                        }
                        CatalogGroup::Outside => match theme {
                            "Jardín y terraza" => i.folder == "Jardín y terraza",
                            "Vegetación" => i.category == "Entorno",
                            "Mobiliario urbano" => {
                                i.category == "Mobiliario urbano"
                                    && !matches!(
                                        room_of(&i.folder),
                                        "Jardín y terraza" | "Piscinas y deporte"
                                    )
                            }
                            "Piscinas y deporte" => room_of(&i.folder) == "Piscinas y deporte",
                            "Gente y coches" => matches!(i.category, "Gente" | "Vehículos"),
                            "Rocas y troncos" => i.category == "Elementos naturales",
                            _ => false,
                        },
                        CatalogGroup::Build => false,
                    })
                    .map(|i| Card {
                        act: CatalogAct::Object(i.pick),
                        sub: object_kind(&i.family, &i.folder),
                        name: i.name,
                        glyph: Glyph::Object,
                        thumb: i.thumb,
                        swatches: Vec::new(),
                    })
                    .collect()
            }
        }
    }

    /// Whether a card is what the tool in use is doing.
    fn card_on(&self, act: CatalogAct) -> bool {
        match act {
            CatalogAct::Room => self.tool == Tool::Wall && self.room_walls,
            CatalogAct::Wall => self.tool == Tool::Wall && !self.room_walls,
            CatalogAct::Floor => self.tool == Tool::Slab,
            CatalogAct::Zone => self.tool == Tool::Zone,
            CatalogAct::Door(t) => self.tool == Tool::Door && self.door_settings.opening_type == t,
            CatalogAct::Window(t) => {
                self.tool == Tool::Window && self.window_settings.opening_type == t
            }
            CatalogAct::Roof(k) => {
                self.tool == Tool::Roof
                    && self.roof_settings.dormer.is_none()
                    && self.roof_settings.kind == k
            }
            CatalogAct::Dormer(k) => {
                self.tool == Tool::Roof && self.roof_settings.dormer == Some(k)
            }
            CatalogAct::Stair(c) => {
                let s = &self.stair_settings;
                self.tool == Tool::Stair
                    && s.preset == c.preset
                    && s.winders == c.winders
                    && s.split == c.split
                    && s.structure == c.structure
            }
            CatalogAct::Railing => self.tool == Tool::Railing,
            CatalogAct::Object(ObjectPick::Project(t)) => {
                self.tool == Tool::Object && self.object_settings.object_type == Some(t)
            }
            CatalogAct::Object(_) => false,
            CatalogAct::Collected(name) => {
                self.tool == Tool::Object
                    && self.collection.is_some_and(|c| {
                        let m = self.ed.model();
                        self.object_settings
                            .object_type
                            .and_then(|t| m.object_type(t))
                            .is_some_and(|t| {
                                t.name == archisim_library::collections::variant_name(name, c)
                            })
                    })
            }
            CatalogAct::Planting => self.tool == Tool::Planting,
            CatalogAct::Road => self.tool == Tool::Road,
            CatalogAct::Pool => self.tool == Tool::Pool,
            CatalogAct::Terrain => self.tool == Tool::Terrain,
            CatalogAct::Brush => self.tool == Tool::Brush,
            CatalogAct::Volume => {
                self.tool == Tool::Mass
                    && self.mass_settings.what == crate::editor::MassWhat::Volume
                    && !self.mass_settings.mode.is_form()
            }
            CatalogAct::Form(m) => self.tool == Tool::Mass && self.mass_settings.mode == m,
        }
    }

    /// Takes the tool of a card, with its type.
    pub(super) fn catalog_take(&mut self, act: CatalogAct) {
        use crate::wall_tool::WallMode;
        self.room_walls = false;
        match act {
            CatalogAct::Room => {
                self.pick(Pick::Tool(Tool::Wall));
                self.room_walls = true;
                self.wall_tool.set_mode(WallMode::Rectangle);
                self.say("Habitación: arrastra o pincha dos esquinas opuestas; salen sus muros, su suelo y su local.");
            }
            CatalogAct::Wall => {
                self.pick(Pick::Tool(Tool::Wall));
                self.wall_tool.set_mode(WallMode::Chain);
            }
            CatalogAct::Floor => {
                self.pick(Pick::Tool(Tool::Slab));
                self.slab_settings.wand = true;
                self.slab_tool.finish();
            }
            CatalogAct::Zone => {
                self.pick(Pick::Tool(Tool::Zone));
                self.room_settings.wand = true;
            }
            CatalogAct::Door(t) => {
                self.door_settings.opening_type = t;
                self.pick(Pick::Tool(Tool::Door));
            }
            CatalogAct::Window(t) => {
                self.window_settings.opening_type = t;
                self.pick(Pick::Tool(Tool::Window));
            }
            CatalogAct::Roof(k) => {
                self.pick(Pick::Tool(Tool::Roof));
                self.roof_settings.kind = k;
                self.roof_settings.dormer = None;
                self.roof_settings.wand = true;
                self.say("Cubierta: pincha dentro del edificio y sale por las caras exteriores de sus muros.");
            }
            CatalogAct::Dormer(k) => {
                self.pick(Pick::Tool(Tool::Roof));
                self.roof_settings.dormer = Some(k);
                self.roof_tool.finish();
                self.say(format!("{}: pincha en un faldón.", k.name()));
            }
            CatalogAct::Stair(c) => {
                use archisim_core::{StairPreset as P, StairStructure as S, StairUse};
                self.pick(Pick::Tool(Tool::Stair));
                self.stair_tool.points.clear();
                let s = &mut self.stair_settings;
                s.preset = c.preset;
                s.winders = c.winders;
                s.split = c.split;
                s.structure = c.structure;
                // Split landings: only in a dwelling's stair (CTE).
                if c.split {
                    s.usage = StairUse::Restricted;
                }
                // Concrete closed, the others open; a spiral narrower and
                // steeper, for a dwelling.
                s.closed_risers = c.structure == S::Slab;
                if c.preset == P::Spiral {
                    s.width = 0.8;
                    s.going = 0.24;
                    s.core = 0.12;
                    s.usage = StairUse::Restricted;
                }
                self.say(match c.preset {
                    P::Free => "Escalera libre: clic en cada giro de la línea de huella; Intro para terminar.",
                    P::Spiral => "Escalera de caracol: clic en el centro y clic hacia donde arranca.",
                    _ => "Escalera: clic en el arranque y clic hacia donde sube; Espacio la gira al otro lado.",
                });
            }
            CatalogAct::Railing => self.pick(Pick::Tool(Tool::Railing)),
            CatalogAct::Object(pick) => {
                if let Some(t) = self.object_pick_type(pick) {
                    self.object_scope = None;
                    self.object_settings.object_type = Some(t);
                    self.set_tool(Tool::Object);
                }
            }
            CatalogAct::Collected(name) => self.take_collected(name),
            CatalogAct::Planting => self.pick(Pick::Tool(Tool::Planting)),
            CatalogAct::Road => self.pick(Pick::Tool(Tool::Road)),
            CatalogAct::Pool => self.pick(Pick::Tool(Tool::Pool)),
            CatalogAct::Terrain => self.pick(Pick::Tool(Tool::Terrain)),
            CatalogAct::Brush => {
                let mode = self.brush.mode;
                self.take_brush(mode);
            }
            CatalogAct::Volume => self.pick(Pick::Concept(super::concept_ui::ConceptPick::Volume)),
            CatalogAct::Form(m) => {
                self.pick(Pick::Concept(super::concept_ui::ConceptPick::Form));
                self.mass_settings.mode = m;
            }
        }
    }

    fn maquette_catalog(&mut self, ui: &mut Ui) {
        let p = Palette::current(ui.ctx());
        let ctx = ui.ctx().clone();
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 4.0;
            for g in CatalogGroup::ALL {
                let on = self.catalog_group == g;
                let text = egui::RichText::new(g.name())
                    .font(strong_font(13.0))
                    .color(if on { Color32::WHITE } else { p.text });
                if ui
                    .add(egui::Button::new(text).fill(if on { p.accent } else { p.panel }))
                    .clicked()
                {
                    self.catalog_group = g;
                    self.catalog_theme = g.themes()[0];
                    self.catalog_sub = None;
                }
            }
            ui.separator();
            let themes = self.catalog_group.themes();
            if !themes.contains(&self.catalog_theme) {
                self.catalog_theme = themes[0];
            }
            for t in themes {
                if ui.selectable_label(self.catalog_theme == *t, *t).clicked() {
                    self.catalog_theme = t;
                    self.catalog_sub = None;
                }
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui
                    .button("Seleccionar")
                    .on_hover_text("La flecha: elegir, mover y estirar lo que ya está (Esc)")
                    .clicked()
                {
                    self.set_tool(Tool::Arrow);
                }
                if self.catalog_group != CatalogGroup::Build {
                    ui.separator();
                    ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                        self.collection_chooser(ui);
                    });
                }
            });
        });
        ui.add_space(4.0);
        let mut cards = self.catalog_cards(&ctx, self.catalog_group, self.catalog_theme);
        // The third level of the tree: the kinds of the theme (Bajos,
        // Altos, Columnas…), as chips; «Todas» shows them all.
        let mut subs: Vec<String> = Vec::new();
        for c in &cards {
            if !c.sub.is_empty() && !subs.contains(&c.sub) {
                subs.push(c.sub.clone());
            }
        }
        subs.sort_by_key(|k| kind_order(k));
        if self.catalog_sub.as_ref().is_some_and(|k| !subs.contains(k)) {
            self.catalog_sub = None;
        }
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 4.0;
            ui.add_space(2.0);
            ui.label(egui::RichText::new("↳").color(p.muted));
            if subs.len() < 2 {
                ui.label(
                    egui::RichText::new(self.catalog_theme)
                        .font(strong_font(12.5))
                        .color(p.text),
                );
                return;
            }
            let chip = |ui: &mut Ui, label: &str, on: bool| -> bool {
                let text =
                    egui::RichText::new(label)
                        .size(12.5)
                        .color(if on { p.accent } else { p.text });
                ui.add(
                    egui::Button::new(text)
                        .fill(if on { p.accent_soft } else { p.panel })
                        .stroke(Stroke::new(1.0, if on { p.accent_soft } else { p.line }))
                        .corner_radius(CornerRadius::same(12)),
                )
                .clicked()
            };
            if chip(ui, "Todas", self.catalog_sub.is_none()) {
                self.catalog_sub = None;
            }
            for k in &subs {
                if chip(ui, k, self.catalog_sub.as_deref() == Some(k.as_str())) {
                    self.catalog_sub = Some(k.clone());
                }
            }
        });
        if let Some(k) = &self.catalog_sub {
            cards.retain(|c| &c.sub == k);
        }
        ui.add_space(4.0);
        // The pieces drawn in 3D, a few each frame.
        self.thumbs3d.begin();
        for c in &mut cards {
            if let Some(t) = self.card_thumb_3d(&ctx, c.act) {
                c.thumb = Some(t);
            }
        }
        if self.thumbs3d.pending {
            ctx.request_repaint();
        }
        if cards.is_empty() {
            ui.label(
                egui::RichText::new("Nada en este tema todavía: la biblioteca del estudio y los tipos del proyecto salen aquí.")
                    .color(p.muted),
            );
            return;
        }
        let mut take = None;
        egui::ScrollArea::horizontal()
            .id_salt("maquette-cards")
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 8.0;
                    for c in &cards {
                        let on = self.card_on(c.act);
                        let (rect, resp) = ui.allocate_exact_size(CARD, Sense::click());
                        super::missions_ui::mark(&ctx, format!("card:{}", c.name), rect);
                        let painter = ui.painter_at(rect);
                        let fill = if on {
                            p.accent_soft
                        } else if resp.hovered() {
                            p.raised
                        } else {
                            p.ground
                        };
                        painter.rect_filled(rect, CornerRadius::same(8), fill);
                        painter.rect_stroke(
                            rect,
                            CornerRadius::same(8),
                            Stroke::new(
                                if on { 1.5 } else { 1.0 },
                                if on { p.accent } else { p.line },
                            ),
                            egui::StrokeKind::Inside,
                        );
                        let pic = Rect::from_min_size(
                            rect.min + Vec2::new(8.0, 6.0),
                            Vec2::new(CARD.x - 16.0, CARD.y - 46.0),
                        );
                        match &c.thumb {
                            Some(t) => {
                                let s = t.fit(pic.size());
                                let r = Rect::from_center_size(pic.center(), s);
                                painter.image(
                                    t.tex,
                                    r,
                                    Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)),
                                    Color32::WHITE,
                                );
                            }
                            None => glyphs::paint(
                                &painter,
                                Rect::from_center_size(pic.center(), Vec2::splat(40.0)),
                                c.glyph,
                                if on { p.accent } else { p.text },
                            ),
                        }
                        // The collection's colours, under the picture.
                        if !c.swatches.is_empty() {
                            let n = c.swatches.len() as f32;
                            let w = (CARD.x - 24.0) / n;
                            for (i, col) in c.swatches.iter().enumerate() {
                                let r = Rect::from_min_size(
                                    Pos2::new(
                                        rect.left() + 12.0 + i as f32 * w,
                                        rect.bottom() - 41.0,
                                    ),
                                    Vec2::new(w - 1.0, 4.0),
                                );
                                painter.rect_filled(r, CornerRadius::same(1), *col);
                            }
                        }
                        // The whole name, in two lines if it needs them.
                        let name = egui::text::LayoutJob {
                            wrap: egui::text::TextWrapping {
                                max_width: CARD.x - 10.0,
                                max_rows: 2,
                                break_anywhere: false,
                                overflow_character: Some('…'),
                            },
                            halign: egui::Align::Center,
                            ..egui::text::LayoutJob::single_section(
                                c.name.clone(),
                                egui::TextFormat::simple(FontId::proportional(11.5), p.text),
                            )
                        };
                        let g = painter.layout_job(name);
                        painter.galley(Pos2::new(rect.center().x, rect.bottom() - 36.0), g, p.text);
                        if resp.on_hover_text(&c.name).clicked() {
                            take = Some(c.act);
                        }
                    }
                });
            });
        if let Some(act) = take {
            self.catalog_take(act);
        }
    }

    /// A short name of what an element is (its type).
    pub(super) fn element_title(&self, e: Element) -> String {
        let m = self.ed.model();
        match e {
            Element::Object(id) => m
                .object(id)
                .and_then(|o| m.object_type(o.object_type))
                .map(|t| t.name.clone()),
            Element::Opening(id) => m
                .opening(id)
                .and_then(|o| m.opening_type(o.opening_type))
                .map(|t| t.name.clone()),
            Element::Wall(id) => m
                .wall(id)
                .and_then(|w| m.wall_type(w.wall_type))
                .map(|t| t.name.clone()),
            Element::Slab(id) => m
                .slab(id)
                .and_then(|s| m.slab_type(s.slab_type))
                .map(|t| t.name.clone()),
            Element::Roof(id) => m
                .roof(id)
                .and_then(|r| m.slab_type(r.roof_type))
                .map(|t| t.name.clone()),
            Element::Dormer(r, i) => m
                .roof(r)
                .and_then(|x| x.dormers.get(usize::from(i)))
                .map(|d| d.kind.name().to_string()),
            _ => None,
        }
        .unwrap_or_default()
    }

    pub(super) fn card_turn_pub(&mut self) {
        self.card_turn()
    }

    pub(super) fn card_duplicate_pub(&mut self) {
        self.card_duplicate()
    }

    /// Turns the selection by the card's angle about its middle.
    fn card_turn(&mut self) {
        let items = self.ed.selection.clone();
        let pts: Vec<Point2> = items
            .iter()
            .filter_map(|e| crate::pick::outline(&self.ed, *e))
            .flatten()
            .collect();
        if pts.is_empty() {
            return;
        }
        let c = pts.iter().fold(Point2::new(0.0, 0.0), |a, q| a + *q) * (1.0 / pts.len() as f64);
        let t = archisim_draft::Transform::rotate(c, self.turn_by.to_radians());
        let res = self
            .ed
            .transact("Girar", |m| crate::xform::transform(m, &items, &t, false));
        match res {
            Ok(sel) => self.ed.selection = sel,
            Err(e) => self.complain(format!("No se puede girar: {e}.")),
        }
    }

    /// A copy of the selection 50 cm aside, selected.
    fn card_duplicate(&mut self) {
        let items = self.ed.selection.clone();
        let t = archisim_draft::Transform::translate(archisim_core::Vec2::new(0.5, -0.5));
        let res = self
            .ed
            .transact("Duplicar", |m| crate::xform::transform(m, &items, &t, true));
        match res {
            Ok(sel) => {
                self.ed.selection = sel;
                self.say("Copia hecha: arrástrala a su sitio.");
            }
            Err(e) => self.complain(format!("No se puede duplicar: {e}.")),
        }
    }

    /// The tool that makes another one like `e`, with its type (the
    /// eyedropper, in both environments).
    pub(super) fn eyedropper(&mut self, e: Element) {
        let m = self.ed.model();
        match e {
            Element::Object(id) => {
                if let Some(t) = m.object(id).map(|o| o.object_type) {
                    self.catalog_take(CatalogAct::Object(ObjectPick::Project(t)));
                }
            }
            Element::Opening(id) => {
                let Some(t) = m.opening(id).map(|o| o.opening_type) else {
                    return;
                };
                let door = m.opening_type(t).is_some_and(|x| x.is_door());
                self.catalog_take(if door {
                    CatalogAct::Door(t)
                } else {
                    CatalogAct::Window(t)
                });
            }
            Element::Wall(id) => {
                if let Some(t) = m.wall(id).map(|w| w.wall_type) {
                    self.wall_settings.wall_type = t;
                    self.catalog_take(CatalogAct::Wall);
                }
            }
            Element::Slab(id) => {
                if let Some(t) = m.slab(id).map(|s| s.slab_type) {
                    self.slab_settings.slab_type = t;
                    self.catalog_take(CatalogAct::Floor);
                }
            }
            Element::Dormer(r, i) => {
                if let Some(k) = m
                    .roof(r)
                    .and_then(|x| x.dormers.get(usize::from(i)))
                    .map(|d| d.kind)
                {
                    self.catalog_take(CatalogAct::Dormer(k));
                }
            }
            Element::Roof(r) => {
                if let Some(t) = m.roof(r).map(|x| x.roof_type) {
                    self.roof_settings.roof_type = t;
                    let kind = self.roof_settings.kind;
                    self.catalog_take(CatalogAct::Roof(kind));
                }
            }
            _ => {
                self.complain("El cuentagotas aún no sabe copiar esto.");
                return;
            }
        }
        self.say("Cuentagotas: la herramienta está lista para poner otro igual.");
    }

    /// All the properties of the selection, in a window (Más…).
    fn maquette_more(&mut self, ctx: &egui::Context) {
        if !self.maquette_props {
            return;
        }
        let mut open = true;
        super::widgets::dialog("Propiedades")
            .id(Id::unique("maquette-props"))
            .open(&mut open)
            .default_width(300.0)
            .default_height(480.0)
            .anchor(Align2::RIGHT_TOP, Vec2::new(-12.0, 60.0))
            .show(ctx, |ui| {
                egui::ScrollArea::vertical().show(ui, |ui| self.properties(ui));
            });
        if !open || self.ed.selection.is_empty() && self.tool == Tool::Arrow {
            self.maquette_props = open && !self.ed.selection.is_empty();
        }
    }

    /// The ground grid of the active level in the 3D view: lines every
    /// [`GRID_STEP`] round the camera's target, fainter every metre.
    pub(super) fn draw_ground_grid(&self, painter: &egui::Painter, rect: Rect) {
        if !self.ground_grid {
            return;
        }
        let p = Palette::current(painter.ctx());
        let cam = &self.v3d.camera;
        let size = glam::Vec2::new(rect.width(), rect.height());
        let z = self.v3d.level_z;
        let t = cam.target;
        let reach = 10.0_f32.max(cam.distance * 0.5).min(30.0);
        let step = GRID_STEP as f32;
        let n = (reach / step).ceil() as i32;
        let cx = (t.x / step).round() * step;
        let cy = (t.y / step).round() * step;
        let to = |x: f32, y: f32| {
            cam.project_to_screen(glam::Vec3::new(x, y, z), size)
                .map(|s| rect.min + Vec2::new(s.x, s.y))
        };
        for k in -n..=n {
            let major = k.rem_euclid(2) == 0;
            let _ = &p;
            let color = if major {
                Color32::from_rgba_unmultiplied(120, 132, 150, 120)
            } else {
                Color32::from_rgba_unmultiplied(120, 132, 150, 60)
            };
            let o = k as f32 * step;
            for (a, b) in [
                ((cx + o, cy - reach), (cx + o, cy + reach)),
                ((cx - reach, cy + o), (cx + reach, cy + o)),
            ] {
                if let (Some(pa), Some(pb)) = (to(a.0, a.1), to(b.0, b.1)) {
                    painter.line_segment([pa, pb], Stroke::new(0.7, color));
                }
            }
        }
    }
}

/// The point snapped to the ground grid, if it is on.
pub fn snap_to_grid(p: Point2) -> Point2 {
    let r = |v: f64| (v / GRID_STEP).round() * GRID_STEP;
    Point2::new(r(p.x), r(p.y))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rooms_of_the_library_folders() {
        assert_eq!(room_of("Camas"), "Dormitorio");
        assert_eq!(room_of("Lavabos"), "Baño");
        assert_eq!(room_of("Mesas"), "Comedor");
        assert_eq!(room_of("Iluminación"), "Decoración");
        assert_eq!(room_of("Modelos 3D"), "Otros");
        assert_eq!(room_of("Diseño"), "Diseño");
        assert_eq!(
            snap_to_grid(Point2::new(1.26, -0.74)),
            Point2::new(1.5, -0.5)
        );
    }
}
