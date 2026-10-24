//! Views kept with the project (block 2 of Antonio's review, `docs/
//! GRAFICOS.md`): each level's own plan and saved plans of a level
//! («Planta baja — albañilería», ceiling plans), each with its scale, cut
//! plane and look; view templates; and the window «Visibilidad y gráficos»
//! (Revit's VG) where each category gets its colours, fills, transparency,
//! halftone, patterns and line weight.
//!
//! The view in the active tab lives in the app's fields (`plan_scale`,
//! `cut_plane`, `view_style`); [`ArchiSimApp::sync_views`] keeps them and
//! the model in step every frame: what the user changes goes to the model
//! (one undoable step once the mouse is let go), and what the model
//! changes (undo, a teammate) comes back to the view.

use archisim_core::{
    Category, GraphicOverride, LayerLines, LevelId, PlanKind, PlanView, PlanViewId, Rgb, ViewStyle,
    ViewTemplate, ViewTemplateId,
};
use eframe::egui::{self, Ui};

use super::tabs::{TabKind, ViewTab};
use super::theme::Palette;
use super::ArchiSimApp;

/// What a view keeps in the model.
#[derive(Debug, Clone, PartialEq)]
struct ViewState {
    scale: f64,
    cut: f64,
    style: ViewStyle,
}

#[derive(Default)]
pub struct ViewsUi {
    /// The view last agreed with the model, and which one it is.
    synced: Option<((LevelId, Option<PlanViewId>), ViewState)>,
    /// The window «Visibilidad y gráficos» is open.
    pub vg_open: bool,
    /// Name typed for a new template.
    template_name: String,
}

/// A style without what belongs to the session, not to the view: the
/// discipline greyed and the disciplines hidden from the navigator.
fn kept(style: &ViewStyle) -> ViewStyle {
    let mut s = style.clone();
    s.focus = archisim_core::DisciplineFocus::None;
    s.hidden_disciplines.clear();
    s
}

/// A kept style shown with the session's own parts of `now`.
fn shown(kept: &ViewStyle, now: &ViewStyle) -> ViewStyle {
    let mut s = kept.clone();
    s.focus = now.focus;
    s.hidden_disciplines = now.hidden_disciplines.clone();
    s
}

impl ArchiSimApp {
    /// The saved plan shown in the active tab, if it is one.
    pub(super) fn active_saved(&self) -> Option<PlanViewId> {
        self.tabs
            .get(self.active_tab)
            .filter(|t| matches!(t.kind, TabKind::Plan(_)))
            .and_then(|t| t.saved)
    }

    /// Whether the active plan is a ceiling plan.
    pub(super) fn active_ceiling(&self) -> bool {
        self.active_saved()
            .and_then(|id| self.ed.model().plan_view(id))
            .is_some_and(|v| v.kind == PlanKind::Ceiling)
    }

    /// Opens a saved plan in its tab (a new one if not open).
    pub(super) fn open_saved_view(&mut self, id: PlanViewId) {
        let Some(v) = self.ed.model().plan_view(id).cloned() else {
            return;
        };
        if let Some(i) = self
            .tabs
            .iter()
            .position(|t| t.saved == Some(id) && t.kind == TabKind::Plan(v.level))
        {
            self.activate_tab(i);
            return;
        }
        let tab = ViewTab {
            saved: Some(id),
            scale: v.scale,
            cut_plane: v.cut_plane,
            style: shown(&v.style, &self.view_style),
            ..ViewTab::plan(v.level)
        };
        self.insert_tab(tab);
    }

    /// The name shown on a plan tab.
    pub(super) fn saved_view_name(&self, id: PlanViewId) -> Option<String> {
        self.ed.model().plan_view(id).map(|v| v.name.clone())
    }

    /// Saves the active plan as a new view (`kind`), and opens it.
    pub(super) fn duplicate_view(&mut self, kind: PlanKind) {
        let level = self.ed.level;
        let base = match (
            self.active_saved().and_then(|id| self.saved_view_name(id)),
            kind,
        ) {
            (Some(n), PlanKind::Floor) => format!("{n} — copia"),
            (_, k) => {
                let l = self
                    .ed
                    .model()
                    .level(level)
                    .map(|l| l.name.clone())
                    .unwrap_or_default();
                match k {
                    PlanKind::Ceiling => format!("{l} — techos"),
                    PlanKind::Floor => format!("{l} — copia"),
                }
            }
        };
        let name = self.ed.model().free_view_name(&base);
        let v = PlanView {
            id: PlanViewId(0),
            name: name.clone(),
            level,
            kind,
            scale: self.plan_scale,
            cut_plane: if kind == PlanKind::Ceiling {
                self.cut_plane.max(1.5)
            } else {
                self.cut_plane
            },
            style: kept(&self.view_style),
        };
        match self.ed.transact("Guardar vista", |m| m.add_plan_view(v)) {
            Ok(id) => {
                self.open_saved_view(id);
                self.say(format!(
                    "Vista «{name}» guardada: cambia en ella lo que se ve y cómo, sin tocar las demás."
                ));
            }
            Err(e) => self.complain(format!("No se puede guardar la vista: {e}.")),
        }
    }

    pub(super) fn rename_view(&mut self, id: PlanViewId, name: &str) {
        let Some(mut v) = self.ed.model().plan_view(id).cloned() else {
            return;
        };
        v.name = name.trim().to_string();
        let r = self.ed.transact("Renombrar vista", |m| m.set_plan_view(v));
        self.report(r, "Vista renombrada.");
    }

    pub(super) fn delete_view(&mut self, id: PlanViewId) {
        let r = self
            .ed
            .transact("Borrar vista", |m| m.remove_plan_view(id).map(drop));
        if r.is_ok() {
            // Its tabs go back to the level's own plan.
            for t in &mut self.tabs {
                if t.saved == Some(id) {
                    t.saved = None;
                }
            }
        }
        self.report(r, "Vista borrada (la planta sigue).");
    }

    /// Gives the active view a template's look.
    pub(super) fn apply_template(&mut self, id: ViewTemplateId) {
        let Some(t) = self.ed.model().view_template(id).cloned() else {
            return;
        };
        self.view_style = shown(&t.style, &self.view_style);
        self.say(format!("Plantilla «{}» aplicada a la vista.", t.name));
    }

    /// Keeps the active plan view and the model in step (see the module).
    pub(super) fn sync_views(&mut self, ctx: &egui::Context) {
        let Some(TabKind::Plan(level)) = self.tabs.get(self.active_tab).map(|t| t.kind) else {
            return;
        };
        let mut saved = self.active_saved();
        if let Some(id) = saved {
            if self.ed.model().plan_view(id).is_none() {
                // Deleted (or undone): the tab shows the level's own plan.
                if let Some(t) = self.tabs.get_mut(self.active_tab) {
                    t.saved = None;
                }
                saved = None;
            }
        }
        let key = (level, saved);
        let now = ViewState {
            scale: self.plan_scale,
            cut: self.cut_plane,
            style: kept(&self.view_style),
        };
        let stored = match saved {
            Some(id) => {
                let v = self
                    .ed
                    .model()
                    .plan_view(id)
                    .cloned()
                    .expect("checked above");
                ViewState {
                    scale: v.scale,
                    cut: v.cut_plane,
                    style: v.style,
                }
            }
            // A level's own plan keeps its look; scale and cut plane are
            // the tab's.
            None => ViewState {
                scale: now.scale,
                cut: now.cut,
                style: self.ed.model().level_style(level),
            },
        };
        let adopt = |app: &mut Self, s: &ViewState| {
            app.plan_scale = s.scale;
            app.cut_plane = s.cut;
            app.view_style = shown(&s.style, &app.view_style);
        };
        match &self.views.synced {
            Some((k, last)) if *k == key => {
                if now != *last {
                    // Changed here: to the model, once the mouse is let go
                    // (a slider dragged is one step).
                    if ctx.input(|i| i.pointer.any_down()) {
                        return;
                    }
                    let r = self.ed.transact("Gráficos de la vista", |m| match saved {
                        Some(id) => {
                            let mut v = m.plan_view(id).cloned().expect("checked above");
                            v.scale = now.scale;
                            v.cut_plane = now.cut;
                            v.style = now.style.clone();
                            m.set_plan_view(v)
                        }
                        None => m.set_level_style(level, now.style.clone()),
                    });
                    if let Err(e) = r {
                        self.complain(format!("No se puede guardar la vista: {e}."));
                    }
                    self.views.synced = Some((key, now));
                } else if stored != *last {
                    // Changed in the model (undo, a teammate).
                    adopt(self, &stored);
                    self.views.synced = Some((key, stored));
                }
            }
            _ => {
                adopt(self, &stored);
                self.views.synced = Some((key, stored));
            }
        }
    }

    /// The window «Visibilidad y gráficos» of the active plan.
    pub(super) fn vg_window(&mut self, ctx: &egui::Context) {
        if !self.views.vg_open {
            return;
        }
        let p = Palette::current(ctx);
        let title = match self.active_saved().and_then(|id| self.saved_view_name(id)) {
            Some(n) => format!("Visibilidad y gráficos · {n}"),
            None => {
                let l = self
                    .ed
                    .model()
                    .level(self.ed.level)
                    .map(|l| l.name.clone())
                    .unwrap_or_default();
                format!("Visibilidad y gráficos · {l}")
            }
        };
        let mut open = true;
        let mut style = self.view_style.clone();
        let templates: Vec<(ViewTemplateId, String)> = self
            .ed
            .model()
            .view_templates()
            .map(|t| (t.id, t.name.clone()))
            .collect();
        let mut apply = None;
        let mut save_as = None;
        let mut delete_template = None;
        super::widgets::dialog(title)
            .id(egui::Id::unique("vg-window"))
            .open(&mut open)
            .default_width(780.0)
            .default_height(560.0)
            .resizable(true)
            .show(ctx, |ui| {
                ui.label(
                    egui::RichText::new(
                        "Cómo se dibuja cada categoría en esta vista. Lo que no se toca queda como siempre; «Auto» deja el color propio de cada elemento. Se guarda con la vista y se imprime igual.",
                    )
                    .color(p.muted),
                );
                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    ui.label("Plantilla:");
                    egui::ComboBox::from_id_salt("vg-template")
                        .selected_text("Aplicar…")
                        .show_ui(ui, |ui| {
                            for (id, name) in &templates {
                                if ui.selectable_label(false, name).clicked() {
                                    apply = Some(*id);
                                }
                            }
                        });
                    ui.add(
                        egui::TextEdit::singleline(&mut self.views.template_name)
                            .hint_text("Nombre de plantilla")
                            .desired_width(140.0),
                    );
                    if ui
                        .button("Guardar como plantilla")
                        .on_hover_text("Para dar este aspecto a otras vistas (y a otros proyectos con la biblioteca)")
                        .clicked()
                    {
                        save_as = Some(self.views.template_name.trim().to_string());
                    }
                    egui::ComboBox::from_id_salt("vg-template-delete")
                        .selected_text("Borrar plantilla…")
                        .show_ui(ui, |ui| {
                            for (id, name) in &templates {
                                if ui.selectable_label(false, name).clicked() {
                                    delete_template = Some(*id);
                                }
                            }
                        });
                    if ui
                        .button("Restablecer todo")
                        .on_hover_text("Todas las categorías como siempre")
                        .clicked()
                    {
                        style.overrides.clear();
                        style.hidden.clear();
                    }
                });
                ui.separator();
                egui::ScrollArea::vertical().show(ui, |ui| {
                    egui::Grid::new("vg-grid")
                        .striped(true)
                        .spacing([10.0, 4.0])
                        .show(ui, |ui| {
                            for h in [
                                "Categoría",
                                "Visible",
                                "Líneas",
                                "Relleno de corte",
                                "Relleno de superficie",
                                "Transparencia",
                                "Medios tonos",
                                "Tramas",
                                "Grosor",
                                "",
                            ] {
                                ui.strong(h);
                            }
                            ui.end_row();
                            for c in Category::ALL {
                                category_row(ui, &mut style, c);
                                ui.end_row();
                            }
                        });
                });
            });
        if style != self.view_style {
            self.view_style = style;
        }
        if let Some(id) = apply {
            self.apply_template(id);
        }
        if let Some(name) = save_as {
            if name.is_empty() {
                self.complain("Escribe el nombre de la plantilla.");
            } else {
                let t = ViewTemplate {
                    id: ViewTemplateId(0),
                    name: name.clone(),
                    style: kept(&self.view_style),
                };
                let r = self.ed.transact("Guardar plantilla de vista", |m| {
                    let existing = m.view_templates().find(|t| t.name == name).map(|t| t.id);
                    match existing {
                        Some(id) => m.set_view_template(ViewTemplate { id, ..t }),
                        None => m.add_view_template(t).map(drop),
                    }
                });
                self.report(r, &format!("Plantilla «{name}» guardada."));
            }
        }
        if let Some(id) = delete_template {
            let r = self.ed.transact("Borrar plantilla de vista", |m| {
                m.remove_view_template(id).map(drop)
            });
            self.report(r, "Plantilla borrada (las vistas conservan su aspecto).");
        }
        self.views.vg_open = open;
    }
}

/// An optional colour: «Auto» or a colour button.
fn colour_cell(ui: &mut Ui, salt: (Category, &str), value: &mut Option<Rgb>, fallback: Rgb) {
    ui.push_id(salt, |ui| {
        ui.horizontal(|ui| {
            let mut set = value.is_some();
            if ui
                .checkbox(&mut set, "")
                .on_hover_text(
                    "Marcado: un color para toda la categoría; sin marcar: el de cada elemento",
                )
                .changed()
            {
                *value = set.then_some(value.unwrap_or(fallback));
            }
            match value {
                Some(c) => {
                    ui.color_edit_button_srgb(c);
                }
                None => {
                    ui.label(egui::RichText::new("Auto").weak());
                }
            }
        });
    });
}

/// One category of the window: whether it is seen and how it is drawn.
fn category_row(ui: &mut Ui, style: &mut ViewStyle, c: Category) {
    let mut g: GraphicOverride = style.graphics(c);
    ui.label(c.label());
    let mut on = style.shows(c);
    if ui.checkbox(&mut on, "").changed() {
        if on {
            style.hidden.remove(&c);
        } else {
            style.hidden.insert(c);
        }
    }
    colour_cell(ui, (c, "line"), &mut g.line, [26, 26, 26]);
    colour_cell(ui, (c, "cut"), &mut g.cut_fill, [255, 255, 255]);
    colour_cell(ui, (c, "surface"), &mut g.surface_fill, [255, 255, 255]);
    let mut t = g.transparency as f32;
    if ui
        .add(
            egui::Slider::new(&mut t, 0.0..=100.0)
                .suffix(" %")
                .show_value(true),
        )
        .changed()
    {
        g.transparency = t.round() as u8;
    }
    ui.checkbox(&mut g.halftone, "");
    ui.checkbox(&mut g.patterns, "");
    let mut w = g.weight as f32;
    if ui
        .add(
            egui::DragValue::new(&mut w)
                .range(10.0..=400.0)
                .suffix(" %")
                .speed(5.0),
        )
        .on_hover_text("Grosor de las líneas, en % del de la norma gráfica")
        .changed()
    {
        g.weight = w.round() as u16;
    }
    ui.horizontal(|ui| {
        if c == Category::Walls {
            egui::ComboBox::from_id_salt(("vg-layers", c.code()))
                .selected_text(g.layer_lines.label())
                .width(150.0)
                .show_ui(ui, |ui| {
                    for l in LayerLines::ALL {
                        ui.selectable_value(&mut g.layer_lines, l, l.label());
                    }
                });
        }
        if !g.is_default() && ui.small_button("↺").on_hover_text("Como siempre").clicked() {
            g = GraphicOverride::default();
        }
    });
    style.set_graphics(c, g);
}
