//! The «Maqueta» environment (`docs/MAQUETA.md`): the same model and the
//! same tools, shown as Los Sims or Paralives would. The 3D view fills
//! the window; a catalog at the bottom (Construir · Amueblar · Exterior)
//! replaces the tiles and the types; a card next to what is selected
//! replaces Properties; the top bar keeps only undo, the level, the walls
//! cut away and the grid. «Técnica · Maqueta» switches between the two
//! environments, keeping the selection.

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
                "Otros",
            ],
            Self::Outside => &[
                "Vegetación",
                "Mobiliario urbano",
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
        "Cocina" | "Electrodomésticos" => "Cocina",
        "Accesorios de baño" | "Duchas y bañeras" | "Inodoros y bidés" | "Lavabos" => "Baño",
        "Camas" | "Dormitorio" | "Armarios" => "Dormitorio",
        "Despacho" => "Despacho",
        _ => "Otros",
    }
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
    Stair,
    Railing,
    Object(ObjectPick),
    Planting,
    Road,
    Terrain,
    Volume,
    Form(crate::editor::MassMode),
}

/// One card of the catalog.
struct Card {
    act: CatalogAct,
    name: String,
    glyph: Glyph,
    thumb: Option<super::thumbs::Thumb>,
}

/// Height of the catalog at the bottom.
const CATALOG_HEIGHT: f32 = 156.0;
/// Size of a card.
const CARD: Vec2 = Vec2::new(96.0, 92.0);
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
                "El 3D a pantalla con un catálogo, como Los Sims (Tab con el dibujo activo)",
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
        self.maquette_card(&ctx);
        self.maquette_more(&ctx);
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
            ui.checkbox(&mut self.ground_grid, "Cuadrícula")
                .on_hover_text(
                "Cuadrícula de 50 cm en el suelo de la planta, y los clics se ajustan a ella (G)",
            );
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
        };
        let sig = self.picture_signature(false);
        let openings = |app: &mut Self, door: bool| -> Vec<Card> {
            let model = app.ed.model();
            let thumbs = &mut app.thumbs;
            model
                .opening_types()
                .filter(|t| t.is_door() == door)
                .map(|t| {
                    let key = format!("maqueta:open:{}:{sig}", t.id.0);
                    let thumb =
                        thumbs.get(ctx, key, || crate::type_preview::opening_model(model, t));
                    Card {
                        act: if door {
                            CatalogAct::Door(t.id)
                        } else {
                            CatalogAct::Window(t.id)
                        },
                        name: t.name.clone(),
                        glyph: if door { Glyph::Door } else { Glyph::Window },
                        thumb,
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
            (CatalogGroup::Build, "Escaleras") => vec![
                card(CatalogAct::Stair, "Escalera", Glyph::Stair),
                card(CatalogAct::Railing, "Barandilla", Glyph::Railing),
            ],
            (CatalogGroup::Outside, "Terreno y caminos") => vec![
                card(CatalogAct::Terrain, "Terreno", Glyph::Terrain),
                card(CatalogAct::Road, "Camino o calle", Glyph::Road),
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
                            "Vegetación" => i.category == "Entorno",
                            "Mobiliario urbano" => i.category == "Mobiliario urbano",
                            "Gente y coches" => matches!(i.category, "Gente" | "Vehículos"),
                            _ => false,
                        },
                        CatalogGroup::Build => false,
                    })
                    .map(|i| Card {
                        act: CatalogAct::Object(i.pick),
                        name: i.name,
                        glyph: Glyph::Object,
                        thumb: i.thumb,
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
            CatalogAct::Stair => self.tool == Tool::Stair,
            CatalogAct::Railing => self.tool == Tool::Railing,
            CatalogAct::Object(ObjectPick::Project(t)) => {
                self.tool == Tool::Object && self.object_settings.object_type == Some(t)
            }
            CatalogAct::Object(_) => false,
            CatalogAct::Planting => self.tool == Tool::Planting,
            CatalogAct::Road => self.tool == Tool::Road,
            CatalogAct::Terrain => self.tool == Tool::Terrain,
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
            CatalogAct::Stair => self.pick(Pick::Tool(Tool::Stair)),
            CatalogAct::Railing => self.pick(Pick::Tool(Tool::Railing)),
            CatalogAct::Object(pick) => {
                if let Some(t) = self.object_pick_type(pick) {
                    self.object_scope = None;
                    self.object_settings.object_type = Some(t);
                    self.set_tool(Tool::Object);
                }
            }
            CatalogAct::Planting => self.pick(Pick::Tool(Tool::Planting)),
            CatalogAct::Road => self.pick(Pick::Tool(Tool::Road)),
            CatalogAct::Terrain => self.pick(Pick::Tool(Tool::Terrain)),
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
            });
        });
        ui.add_space(6.0);
        let cards = self.catalog_cards(&ctx, self.catalog_group, self.catalog_theme);
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
                            Vec2::new(CARD.x - 16.0, CARD.y - 34.0),
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
                        let name = egui::text::LayoutJob {
                            wrap: egui::text::TextWrapping::truncate_at_width(CARD.x - 10.0),
                            ..egui::text::LayoutJob::single_section(
                                c.name.clone(),
                                egui::TextFormat::simple(FontId::proportional(11.5), p.text),
                            )
                        };
                        let g = painter.layout_job(name);
                        painter.galley(
                            Pos2::new(rect.center().x - g.size().x / 2.0, rect.bottom() - 22.0),
                            g,
                            p.text,
                        );
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

    /// Where the card of the selection goes: over the middle of what is
    /// selected, as seen in the 3D view.
    fn selection_anchor(&self) -> Option<Pos2> {
        let rect = self.v3d.canvas;
        let model = self.ed.model();
        let mut pts = Vec::new();
        let mut z = 0.0;
        for e in &self.ed.selection {
            let Some(o) = crate::pick::outline(&self.ed, *e) else {
                continue;
            };
            pts.extend(o);
            z = match *e {
                Element::Dormer(r, i) => model
                    .roof(r)
                    .and_then(|x| {
                        x.dormers
                            .get(usize::from(i))
                            .and_then(|d| x.dormer_shape(d))
                    })
                    .map(|s| {
                        model
                            .roof(r)
                            .and_then(|x| model.level(x.level))
                            .map_or(0.0, |l| l.elevation)
                            + s.ridge_z
                    })
                    .unwrap_or(z),
                Element::Roof(r) => model
                    .roof(r)
                    .map(|x| model.level(x.level).map_or(0.0, |l| l.elevation) + x.top_range().1)
                    .unwrap_or(z),
                _ => self.v3d.level_z as f64 + 1.6,
            };
        }
        if pts.is_empty() {
            return None;
        }
        let n = pts.len() as f64;
        let c = pts.iter().fold(Point2::new(0.0, 0.0), |a, q| a + *q) * (1.0 / n);
        let s = self.v3d.camera.project_to_screen(
            glam::Vec3::new(c.x as f32, c.y as f32, z as f32),
            glam::Vec2::new(rect.width(), rect.height()),
        )?;
        let at = rect.min + Vec2::new(s.x, s.y);
        rect.contains(at).then_some(at)
    }

    /// The card next to what is selected: what it is, and quick actions.
    fn maquette_card(&mut self, ctx: &egui::Context) {
        // Not while a handle is dragged: it would hide what grows.
        if self.ed.selection.is_empty() || self.tool != Tool::Arrow || self.object_grip.is_some() {
            return;
        }
        let Some(at) = self.selection_anchor() else {
            return;
        };
        let p = Palette::current(ctx);
        let label = super::context_bar::selection_label(self.ed.selection.iter());
        let single =
            (self.ed.selection.len() == 1).then(|| *self.ed.selection.iter().next().expect("one"));
        let name = single.map(|e| self.element_title(e)).unwrap_or_default();
        let mut act = None;
        egui::Area::new(Id::unique("maquette-card"))
            .order(egui::Order::Foreground)
            .fixed_pos(at + Vec2::new(40.0, -150.0))
            .show(ctx, |ui| {
                egui::Frame::new()
                    .fill(p.panel)
                    .stroke(Stroke::new(1.0, p.line))
                    .corner_radius(CornerRadius::same(10))
                    .inner_margin(egui::Margin::same(8))
                    .show(ui, |ui| {
                        ui.label(
                            egui::RichText::new(&label)
                                .font(strong_font(12.5))
                                .color(p.text),
                        );
                        if !name.is_empty() {
                            ui.label(egui::RichText::new(&name).small().color(p.muted));
                        }
                        ui.horizontal(|ui| {
                            ui.spacing_mut().item_spacing.x = 4.0;
                            if ui
                                .button("⟳ Girar")
                                .on_hover_text(
                                    "Gira lo seleccionado el ángulo de al lado (sentido antihorario). Un objeto solo se gira también a mano con el anillo del suelo",
                                )
                                .clicked()
                            {
                                act = Some(CardAct::Turn);
                            }
                            ui.add(
                                egui::DragValue::new(&mut self.turn_by)
                                    .range(-180.0..=180.0)
                                    .speed(1.0)
                                    .max_decimals(1)
                                    .suffix("°"),
                            )
                            .on_hover_text("Ángulo del giro, en grados");
                            if ui
                                .button("⧉ Duplicar")
                                .on_hover_text("Una copia al lado, seleccionada")
                                .clicked()
                            {
                                act = Some(CardAct::Duplicate);
                            }
                            if single.is_some()
                                && ui
                                    .button("Cuentagotas")
                                    .on_hover_text("La herramienta para poner otro igual")
                                    .clicked()
                            {
                                act = Some(CardAct::Eyedropper);
                            }
                        });
                        ui.horizontal(|ui| {
                            ui.spacing_mut().item_spacing.x = 4.0;
                            if ui
                                .button("Más…")
                                .on_hover_text("Todas sus propiedades")
                                .clicked()
                            {
                                act = Some(CardAct::More);
                            }
                            if ui.button("🗑 Borrar").on_hover_text("Supr").clicked() {
                                act = Some(CardAct::Delete);
                            }
                        });
                    });
            });
        match act {
            Some(CardAct::Turn) => self.card_turn(),
            Some(CardAct::Duplicate) => self.card_duplicate(),
            Some(CardAct::Eyedropper) => {
                if let Some(e) = single {
                    self.eyedropper(e);
                }
            }
            Some(CardAct::More) => self.maquette_props = true,
            Some(CardAct::Delete) => match self.ed.delete_selection() {
                Ok(n) => self.say(format!("{n} borrado(s).")),
                Err(e) => self.complain(format!("No se puede borrar: {e}.")),
            },
            None => {}
        }
    }

    /// A short name of what an element is (its type).
    fn element_title(&self, e: Element) -> String {
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

    #[cfg(test)]
    pub(super) fn card_turn_pub(&mut self) {
        self.card_turn()
    }

    #[cfg(test)]
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
        egui::Window::new("Propiedades")
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

#[derive(Debug, Clone, Copy, PartialEq)]
enum CardAct {
    Turn,
    Duplicate,
    Eyedropper,
    More,
    Delete,
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
        assert_eq!(room_of("Modelos 3D"), "Otros");
        assert_eq!(
            snap_to_grid(Point2::new(1.26, -0.74)),
            Point2::new(1.5, -0.5)
        );
    }
}
