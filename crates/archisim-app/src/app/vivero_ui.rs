//! The vivero ("nursery"): the window where species of plants are made
//! and tuned, as EZ-Tree does it. On the left the project's species (and
//! those of the library, to bring in); in the middle every parameter of
//! the chosen species with sliders (height and width, habit and crown,
//! trunk, branches, leaves, flowers and fruit, colours, drawing in plan);
//! on the right the plant in 3D and in plan, technical and of
//! presentation, redrawn as the sliders move. «Otro ejemplar» grows
//! another plant of the same species. Every change is the type's: all
//! the plants of the species in the project follow it.

use archisim_core::{Model, ObjectTypeId, ParamScope, Value, Values};
use eframe::egui::{self, Rect, Sense, Ui, Vec2};

use super::props;
use super::theme::Palette;
use super::{ArchiSimApp, View3dState};

/// The state of the window.
pub struct ViveroWindow {
    pub open: bool,
    pub selected: Option<ObjectTypeId>,
    /// The plant shown: its seed and how far it may stray.
    pub seed: u64,
    pub variation: f64,
    pub(super) v3d: View3dState,
    /// The library species chosen to bring in.
    library_pick: usize,
}

impl Default for ViveroWindow {
    fn default() -> Self {
        // A plant is seen best from a little above the ground.
        let mut v3d = View3dState::default();
        v3d.camera.pitch = 12f32.to_radians();
        Self {
            open: false,
            selected: None,
            seed: 1,
            variation: 0.5,
            v3d,
            library_pick: 0,
        }
    }
}

const LIST: f32 = 210.0;
const EDITOR: f32 = 360.0;

fn random_seed(old: u64) -> u64 {
    let t = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(1);
    archisim_core::flora::seed_of(old ^ t, 3) % 999_983 + 1
}

/// A model with one plant of a species, grown from `seed`, drawn as asked
/// («técnico» or «presentación»).
fn plant_model(
    source: &Model,
    t: &archisim_core::ObjectType,
    seed: u64,
    variation: f64,
    drawing: &str,
) -> Option<(Model, archisim_core::LevelId)> {
    let (mut m, level) = crate::type_preview::object_model(source, t)?;
    let id = m.objects().next()?.id;
    let mut v = m.object(id)?.values.clone();
    v.insert("plant_seed".into(), Value::Number(seed as f64));
    v.insert("variation".into(), Value::Number(variation));
    v.insert("drawing".into(), Value::Text(drawing.into()));
    m.set_object_values(id, v).ok()?;
    Some((m, level))
}

impl ArchiSimApp {
    /// Opens the vivero on a species (the selected plant's own values give
    /// the plant shown).
    pub(super) fn open_vivero(&mut self, t: Option<ObjectTypeId>, own: Values) {
        let w = &mut self.vivero;
        w.open = true;
        if t.is_some() {
            w.selected = t;
        }
        if let Some(s) = own.get("plant_seed").and_then(|v| v.as_number()) {
            if s >= 1.0 {
                w.seed = s as u64;
            }
        }
        if let Some(v) = own.get("variation").and_then(|v| v.as_number()) {
            w.variation = v;
        }
    }

    pub(super) fn vivero_dialog(&mut self, ctx: &egui::Context) {
        if !self.vivero.open {
            return;
        }
        let mut open = true;
        super::widgets::dialog("Vivero · especies de vegetación")
            .open(&mut open)
            .default_size([1100.0, 700.0])
            .resizable(true)
            .collapsible(false)
            .show(ctx, |ui| self.vivero_body(ui));
        self.vivero.open = open && self.vivero.open;
    }

    fn vivero_body(&mut self, ui: &mut Ui) {
        let model = self.ed.model();
        let sel = self
            .vivero
            .selected
            .filter(|t| model.type_grows(*t))
            .or_else(|| {
                model
                    .object_types()
                    .find(|t| model.type_grows(t.id))
                    .map(|t| t.id)
            });
        self.vivero.selected = sel;
        let height = ui.available_height().max(500.0);
        ui.horizontal_top(|ui| {
            ui.allocate_ui_with_layout(
                Vec2::new(LIST, height),
                egui::Layout::top_down(egui::Align::Min),
                |ui| {
                    ui.set_width(LIST);
                    self.vivero_list(ui, sel);
                },
            );
            ui.separator();
            let Some(id) = sel else {
                ui.label("El proyecto no tiene todavía especies: trae una de la biblioteca a la izquierda.");
                return;
            };
            ui.allocate_ui_with_layout(
                Vec2::new(EDITOR, height),
                egui::Layout::top_down(egui::Align::Min),
                |ui| {
                    ui.set_width(EDITOR);
                    egui::ScrollArea::vertical()
                        .id_salt("vivero-editor")
                        .show(ui, |ui| self.vivero_editor(ui, id));
                },
            );
            ui.separator();
            ui.vertical(|ui| self.vivero_preview(ui, id));
        });
    }

    fn vivero_list(&mut self, ui: &mut Ui, sel: Option<ObjectTypeId>) {
        let p = Palette::current(ui.ctx());
        ui.label(egui::RichText::new("Especies del proyecto").strong());
        ui.add_space(4.0);
        let ctx = ui.ctx().clone();
        let mut picked = None;
        egui::ScrollArea::vertical()
            .id_salt("vivero-list")
            .max_height(ui.available_height() - 150.0)
            .show(ui, |ui| {
                let model = self.ed.model();
                let thumbs = &mut self.thumbs;
                for t in model.object_types().filter(|t| model.type_grows(t.id)) {
                    let users = model.object_type_users(t.id);
                    let key = format!("vivero-list:{}:{:?}", t.id.0, t.values);
                    let thumb =
                        thumbs.get(&ctx, key, || plant_model(model, t, 7, 0.4, "presentación"));
                    let on = Some(t.id) == sel;
                    let (rect, resp) = ui
                        .allocate_exact_size(Vec2::new(ui.available_width(), 52.0), Sense::click());
                    if on {
                        ui.painter()
                            .rect_filled(rect, egui::CornerRadius::same(6), p.accent_soft);
                    } else if resp.hovered() {
                        ui.painter()
                            .rect_filled(rect, egui::CornerRadius::same(6), p.raised);
                    }
                    let pic =
                        Rect::from_min_size(rect.min + Vec2::new(4.0, 4.0), Vec2::new(44.0, 44.0));
                    if let Some(th) = thumb {
                        let k = (44.0 / th.size.x).min(44.0 / th.size.y);
                        let r = Rect::from_center_size(pic.center(), th.size * k);
                        ui.painter().image(
                            th.tex,
                            r,
                            Rect::from_min_max(egui::Pos2::ZERO, egui::Pos2::new(1.0, 1.0)),
                            egui::Color32::WHITE,
                        );
                    }
                    ui.painter().text(
                        rect.min + Vec2::new(56.0, 10.0),
                        egui::Align2::LEFT_TOP,
                        &t.name,
                        egui::FontId::proportional(13.0),
                        p.text,
                    );
                    ui.painter().text(
                        rect.min + Vec2::new(56.0, 30.0),
                        egui::Align2::LEFT_TOP,
                        format!("{users} en el proyecto"),
                        egui::FontId::proportional(11.0),
                        p.muted,
                    );
                    if resp.clicked() {
                        picked = Some(t.id);
                    }
                }
            });
        if let Some(t) = picked {
            self.vivero.selected = Some(t);
        }
        ui.add_space(6.0);
        if let Some(t) = sel {
            if ui
                .button("Duplicar especie")
                .on_hover_text("Una especie nueva a partir de esta, para cambiarla sin tocar las plantas de esta")
                .clicked()
            {
                let model = self.ed.model();
                if let Some(ty) = model.object_type(t).cloned() {
                    let taken: Vec<String> = model.object_types().map(|t| t.name.clone()).collect();
                    let name = crate::editor::types::copy_name(&ty.name, &taken);
                    let r = self.ed.transact("Duplicar especie", |m| {
                        m.add_object_type(name, ty.family, ty.values.clone())
                    });
                    match r {
                        Ok(new) => {
                            self.vivero.selected = Some(new);
                            self.object_settings.object_type = Some(new);
                            self.say("Especie duplicada: sus cambios ya no tocan la otra.");
                        }
                        Err(e) => self.complain(format!("No se puede duplicar: {e}.")),
                    }
                }
            }
        }
        ui.add_space(6.0);
        ui.label(egui::RichText::new("Traer de la biblioteca").strong());
        let names: Vec<&'static str> = archisim_library::flora::flora_names().collect();
        let i = self.vivero.library_pick.min(names.len().saturating_sub(1));
        egui::ComboBox::from_id_salt("vivero-library")
            .width(LIST - 12.0)
            .selected_text(names.get(i).copied().unwrap_or("—"))
            .show_ui(ui, |ui| {
                let mut group = "";
                for (k, e) in archisim_library::flora::FLORA_CATALOGUE.iter().enumerate() {
                    if e.group != group {
                        group = e.group;
                        ui.label(egui::RichText::new(group).small().color(p.muted));
                    }
                    ui.selectable_value(&mut self.vivero.library_pick, k, e.name);
                }
            });
        if ui.button("Traer al proyecto").clicked() {
            if let Some(name) = names.get(self.vivero.library_pick) {
                match self.ed.library_object_type(name) {
                    Ok(t) => {
                        self.vivero.selected = Some(t);
                        self.say(format!("«{name}» en el proyecto."));
                    }
                    Err(e) => self.complain(format!("No se puede traer: {e}.")),
                }
            }
        }
    }

    fn vivero_editor(&mut self, ui: &mut Ui, id: ObjectTypeId) {
        let model = self.ed.model();
        let Some(t) = model.object_type(id).cloned() else {
            return;
        };
        let Some(def) = model.family(t.family).map(|f| f.def().clone()) else {
            return;
        };
        let materials: Vec<(u64, String)> = model
            .materials()
            .map(|m| (m.id.0, m.name.clone()))
            .collect();
        let users = model.object_type_users(id);
        let mut name = t.name.clone();
        ui.horizontal(|ui| {
            ui.label("Nombre");
            if ui
                .add(egui::TextEdit::singleline(&mut name).desired_width(EDITOR - 80.0))
                .lost_focus()
                && name != t.name
                && !name.trim().is_empty()
            {
                let n = name.trim().to_string();
                let r = self
                    .ed
                    .transact("Renombrar especie", |m| m.rename_object_type(id, n));
                self.report(r, "Especie renombrada.");
            }
        });
        props::note(
            ui,
            &if users == 0 {
                "Ninguna planta del proyecto es de esta especie todavía.".to_string()
            } else {
                format!("Lo que cambies aquí cambia sus {users} plantas del proyecto.")
            },
        );
        let mut section: Option<String> = None;
        for p in def.params_of(ParamScope::Type) {
            if !def.is_visible(&p.name, &t.values) {
                continue;
            }
            if p.section != section {
                section = p.section.clone();
                if let Some(s) = &section {
                    ui.add_space(6.0);
                    ui.label(egui::RichText::new(s).strong());
                }
            }
            let Some(cur) = t.values.get(&p.name) else {
                continue;
            };
            let salt = format!("vivero-{}-{}", id.0, p.name);
            if let Some(v) = super::object_ui::param_slider_row(ui, &salt, p, cur, &materials) {
                let r = self.ed.set_object_type_value(id, &p.name, v);
                if let Err(e) = r {
                    self.complain(format!("No cabe: {e}."));
                }
            }
        }
    }

    fn vivero_preview(&mut self, ui: &mut Ui, id: ObjectTypeId) {
        let p = Palette::current(ui.ctx());
        let model = self.ed.model();
        let Some(t) = model.object_type(id).cloned() else {
            return;
        };
        let (seed, variation) = (self.vivero.seed, self.vivero.variation);
        ui.horizontal(|ui| {
            if ui
                .button("🎲 Otro ejemplar")
                .on_hover_text("Otra planta de la misma especie: cada semilla crece distinta")
                .clicked()
            {
                self.vivero.seed = random_seed(self.vivero.seed);
            }
            ui.label(
                egui::RichText::new(format!("Semilla {seed}"))
                    .small()
                    .color(p.muted),
            );
            ui.add_space(8.0);
            ui.label("Variación");
            let mut v = (variation * 100.0).round();
            if ui
                .add(
                    egui::Slider::new(&mut v, 0.0..=100.0)
                        .suffix(" %")
                        .max_decimals(0),
                )
                .on_hover_text("Cuánto se aparta este ejemplar de su especie")
                .changed()
            {
                self.vivero.variation = v / 100.0;
            }
        });
        let avail = ui.available_width().max(300.0);
        let size = Vec2::new(avail, (ui.available_height() * 0.62).clamp(260.0, 520.0));
        let key = {
            use std::hash::{Hash, Hasher};
            let mut h = std::collections::hash_map::DefaultHasher::new();
            format!("{:?}{seed}{variation}", t.values).hash(&mut h);
            for m in model.materials() {
                m.cut_color.hash(&mut h);
            }
            h.finish()
        };
        let gpu = self.gpu.clone();
        self.vivero.v3d.preview(
            ui,
            gpu.as_ref(),
            key,
            || plant_model(model, &t, seed, variation, "presentación").map(|x| x.0),
            size,
        );
        ui.label(
            egui::RichText::new(
                "Arrastra para girar · rueda para acercar · doble clic para encuadrar",
            )
            .color(p.muted)
            .size(11.0),
        );
        ui.add_space(6.0);
        let ctx = ui.ctx().clone();
        let half = Vec2::new((avail - 12.0) / 2.0, 170.0);
        ui.horizontal(|ui| {
            for (drawing, caption) in [
                ("técnico", "Planta técnica"),
                ("presentación", "Planta de presentación"),
            ] {
                ui.vertical(|ui| {
                    let k = format!("vivero:{drawing}:{:?}:{seed}:{variation}", t.values);
                    let thumb = self
                        .thumbs
                        .get(&ctx, k, || plant_model(model, &t, seed, variation, drawing));
                    if let Some(th) = thumb {
                        props::paper(ui, th, half);
                    }
                    ui.label(egui::RichText::new(caption).color(p.muted).size(11.0));
                });
            }
        });
        if let Some(look) = model.plant_look_of(id, &Values::new(), seed) {
            let plant = archisim_core::flora::plant(&look.species, seed, variation);
            let tris: Vec<String> = archisim_core::flora::Lod::ALL
                .iter()
                .map(|l| plant.mesh(&look.colors, *l).triangle_count().to_string())
                .collect();
            props::note(
                ui,
                &format!(
                    "Este ejemplar: {:.2} m de alto y {:.2} m de ancho · {} caras en 3D (de cerca / de lejos / con cientos de plantas).",
                    plant.height,
                    plant.radius * 2.0,
                    tris.join(" / ")
                )
                .replace('.', ","),
            );
        }
    }
}
