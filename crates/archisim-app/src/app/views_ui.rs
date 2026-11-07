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

use super::widgets::SwitchExt as _;
use archisim_core::{
    Category, GraphicOverride, LayerLines, LevelId, Model, ModelError, PlanKind, PlanView,
    PlanViewId, Rgb, ViewStyle, ViewTemplate, ViewTemplateId, ViewportId,
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
    /// The page of the window Representación.
    tab: VgTab,
    /// The view on a sheet whose properties are being shown, if any: the
    /// window Representación opened from them edits that view.
    pub styling_viewport: Option<ViewportId>,
    /// The view on a sheet the window Representación edits (none: the
    /// active plan).
    pub vg_viewport: Option<ViewportId>,
    /// Its look while the mouse is still held.
    pub(super) vg_pending: Option<(ViewportId, ViewStyle)>,
}

impl ViewsUi {
    /// A look for the view on a sheet, as if the mouse had just been let go
    /// after changing it in the window.
    #[cfg(test)]
    pub fn set_pending(&mut self, id: ViewportId, style: ViewStyle) {
        self.vg_pending = Some((id, style));
    }

    /// Goes to a page of the window Representación.
    #[cfg(test)]
    pub fn set_tab(&mut self, tab: VgTab) {
        self.tab = tab;
    }
}

/// The pages of the window Representación.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum VgTab {
    #[default]
    Categories,
    Layers,
    Look,
}

/// The page «Vista y presentación»: the level of detail, the view's
/// discipline and its presentation style.
fn look_tab(
    ui: &mut Ui,
    style: &mut ViewStyle,
    presentations: &[(Option<archisim_core::PresentationStyleId>, String)],
    open_styles: &mut bool,
) {
    let p = Palette::current(ui.ctx());
    ui.label(
        egui::RichText::new(
            "Lo que vale para toda la vista. Si la vista sigue una plantilla, se guarda en ella.",
        )
        .color(p.muted),
    );
    ui.add_space(6.0);
    egui::Grid::new("vg-look")
        .num_columns(2)
        .spacing([16.0, 8.0])
        .show(ui, |ui| {
            ui.label("Nivel de detalle");
            egui::ComboBox::from_id_salt("vg-detail")
                .selected_text(style.detail.label())
                .width(220.0)
                .show_ui(ui, |ui| {
                    for d in archisim_core::Detail::ALL {
                        ui.selectable_value(&mut style.detail, d, d.label());
                    }
                });
            ui.end_row();
            ui.label("Disciplina de la vista");
            let shown = style
                .discipline
                .map_or("Todas", super::presentation_ui::discipline_label);
            egui::ComboBox::from_id_salt("vg-discipline")
                .selected_text(shown)
                .width(220.0)
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut style.discipline, None, "Todas");
                    for d in archisim_core::ElementDiscipline::ALL {
                        ui.selectable_value(
                            &mut style.discipline,
                            Some(d),
                            super::presentation_ui::discipline_label(d),
                        );
                    }
                });
            ui.end_row();
            ui.label("Estilo de presentación");
            ui.horizontal(|ui| {
                let shown = presentations
                    .iter()
                    .find(|(id, _)| *id == style.presentation)
                    .map_or("Técnica (sin estilo)", |(_, n)| n.as_str());
                egui::ComboBox::from_id_salt("vg-presentation")
                    .selected_text(shown)
                    .width(220.0)
                    .show_ui(ui, |ui| {
                        for (id, name) in presentations {
                            ui.selectable_value(&mut style.presentation, *id, name);
                        }
                    });
                if ui
                    .button("Estilos de presentación…")
                    .on_hover_text("Papel, paleta, muros macizados, grosores, textos y acento de cada estilo del proyecto")
                    .clicked()
                {
                    *open_styles = true;
                }
            });
            ui.end_row();
        });
}

/// A style without what belongs to the session, not to the view: the
/// discipline greyed and the disciplines hidden from the navigator.
/// What a view that follows a template changes in its look goes to the
/// template, and so to every view that follows it (Revit's view
/// templates). Nothing happens for a view with a look of its own.
pub(super) fn to_template(m: &mut Model, style: &ViewStyle) -> Result<(), ModelError> {
    let Some(mut t) = style.template.and_then(|id| m.view_template(id).cloned()) else {
        return Ok(());
    };
    let mut look = kept(style);
    look.template = None;
    if t.style != look {
        t.style = look;
        m.set_view_template(t)?;
    }
    Ok(())
}

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

    /// Makes the active view follow a template (`docs/INTERFAZ.md`
    /// §13.5): its look becomes the template's, and changes with it.
    /// `None`: it keeps the look as its own.
    pub(super) fn apply_template(&mut self, id: Option<ViewTemplateId>) {
        match id.and_then(|id| self.ed.model().view_template(id).cloned()) {
            Some(t) => {
                self.view_style = t.applied(&self.view_style);
                let n = self.ed.model().template_followers(t.id);
                self.say(format!(
                    "La vista sigue la plantilla «{}»{}: lo que cambies en su aspecto cambia la plantilla y todas sus vistas.",
                    t.name,
                    if n > 0 { format!(" (con {n} más)") } else { String::new() }
                ));
            }
            None => {
                self.view_style.template = None;
                self.say("La vista tiene ya su propio aspecto (el que tenía).");
            }
        }
    }

    /// The pill at the top left of a plan: the template the view follows
    /// and, on a click, every template to switch to — the disciplines
    /// first, then each kind of installation — and the one the levels'
    /// plans follow (`docs/INTERFAZ.md` §13.5).
    pub(super) fn template_pill(&mut self, ctx: &egui::Context, rect: egui::Rect) {
        let p = super::theme::Palette::current(ctx);
        let model = self.ed.model();
        let templates: Vec<(ViewTemplateId, String)> = model
            .view_templates()
            .map(|t| (t.id, t.name.clone()))
            .collect();
        let missing = archisim_library::standards::discipline_templates()
            .iter()
            .any(|t| !templates.iter().any(|x| x.1 == t.0));
        let plans = model.plan_template();
        let following = self.view_style.template;
        // An installation's template by the installation alone.
        let shown = following
            .and_then(|id| templates.iter().find(|t| t.0 == id))
            .map_or("Aspecto propio".to_string(), |t| {
                t.1.trim_start_matches("Instalaciones · ").to_string()
            });
        let installations = |n: &str| n.starts_with("Instalaciones ·");
        let disciplines: Vec<&str> = archisim_library::standards::discipline_templates()
            .iter()
            .map(|t| t.0)
            .filter(|n| !installations(n))
            .collect();
        let mut pick: Option<Option<ViewTemplateId>> = None;
        let mut for_plans = false;
        let mut bring = false;
        egui::Area::new(egui::Id::unique("template-pill"))
            .order(egui::Order::Foreground)
            .fixed_pos(rect.left_top() + egui::vec2(12.0, 12.0))
            .show(ctx, |ui| {
                egui::Frame::new()
                    .fill(p.panel)
                    .stroke(egui::Stroke::new(1.0, p.line))
                    .corner_radius(egui::CornerRadius::same(12))
                    .inner_margin(egui::Margin::symmetric(8, 3))
                    .show(ui, |ui| {
                        ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Extend);
                        ui.menu_button(format!("{shown}  ▾"), |ui| {
                            ui.set_min_width(240.0);
                            ui.label(egui::RichText::new("Disciplinas").small().weak());
                            for (id, name) in templates.iter().filter(|t| disciplines.contains(&t.1.as_str())) {
                                if ui.selectable_label(following == Some(*id), name).clicked() {
                                    pick = Some(Some(*id));
                                    ui.close();
                                }
                            }
                            ui.separator();
                            ui.label(egui::RichText::new("Instalaciones, una a una").small().weak());
                            for (id, name) in templates.iter().filter(|t| installations(&t.1)) {
                                let short = name.trim_start_matches("Instalaciones · ");
                                if ui.selectable_label(following == Some(*id), short).clicked() {
                                    pick = Some(Some(*id));
                                    ui.close();
                                }
                            }
                            let others: Vec<&(ViewTemplateId, String)> = templates
                                .iter()
                                .filter(|t| !installations(&t.1) && !disciplines.contains(&t.1.as_str()))
                                .collect();
                            if !others.is_empty() {
                                ui.separator();
                                ui.label(egui::RichText::new("Otras plantillas").small().weak());
                                for (id, name) in others {
                                    if ui.selectable_label(following == Some(*id), name).clicked() {
                                        pick = Some(Some(*id));
                                        ui.close();
                                    }
                                }
                            }
                            ui.separator();
                            if ui
                                .selectable_label(following.is_none(), "Aspecto propio (todo)")
                                .on_hover_text("Sin plantilla: la vista enseña lo que diga Visibilidad y gráficos")
                                .clicked()
                            {
                                pick = Some(None);
                                ui.close();
                            }
                            if missing {
                                ui.separator();
                                if ui
                                    .button("Traer las plantillas de disciplina e instalaciones")
                                    .on_hover_text("Añade al proyecto las que le faltan y, si las plantas no siguen ninguna, pone Arquitectura en todas")
                                    .clicked()
                                {
                                    bring = true;
                                    ui.close();
                                }
                            }
                            if following.is_some() && following != plans {
                                ui.separator();
                                if ui
                                    .button("Usarla en todas las plantas")
                                    .on_hover_text("Las plantas que no tienen su propio aspecto siguen esta plantilla")
                                    .clicked()
                                {
                                    for_plans = true;
                                    ui.close();
                                }
                            }
                        })
                        .response
                        .on_hover_text("Plantilla de la vista: qué se ve en ella. Arquitectura deja fuera las instalaciones; cada instalación tiene la suya");
                    });
            });
        if bring {
            let r = self.ed.transact("Plantillas de disciplina", |m| {
                let (_, added) = archisim_library::standards::add_schemes(m);
                if m.plan_template().is_none() {
                    archisim_library::standards::set_architecture_plans(m);
                }
                Ok(added)
            });
            match r {
                Ok(n) => {
                    // The open plan follows what its level now follows.
                    self.view_style = self.ed.model().level_style(self.ed.level);
                    self.say(format!(
                        "{n} plantilla{} añadida{}; las plantas siguen Arquitectura, sin instalaciones.",
                        if n == 1 { "" } else { "s" },
                        if n == 1 { "" } else { "s" }
                    ));
                }
                Err(e) => self.complain(format!("No se puede: {e}.")),
            }
        }
        if let Some(id) = pick {
            self.apply_template(id);
        }
        if for_plans {
            let id = self.view_style.template;
            let name = shown.clone();
            let r = self
                .ed
                .transact("Plantilla de las plantas", |m| m.set_plan_template(id));
            match r {
                Ok(()) => self.say(format!(
                    "Todas las plantas siguen «{name}» salvo las que tienen su propio aspecto."
                )),
                Err(e) => self.complain(format!("No se puede: {e}.")),
            }
        }
    }

    /// The template the active view follows, if any.
    pub(super) fn followed_template(&self) -> Option<ViewTemplate> {
        self.view_style
            .template
            .and_then(|id| self.ed.model().view_template(id).cloned())
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
                    let r = self.ed.transact("Gráficos de la vista", |m| {
                        // A view that follows a template: what changes in
                        // its look changes the template, and so every view
                        // that follows it (Revit's view templates).
                        to_template(m, &now.style)?;
                        match saved {
                            Some(id) => {
                                let mut v = m.plan_view(id).cloned().expect("checked above");
                                v.scale = now.scale;
                                v.cut_plane = now.cut;
                                v.style = now.style.clone();
                                m.set_plan_view(v)
                            }
                            None => m.set_level_style(level, now.style.clone()),
                        }
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
            self.views.vg_viewport = None;
            return;
        }
        let Some(id) = self.views.vg_viewport else {
            return self.vg_window_of(ctx, None);
        };
        // A view on a sheet: the window edits its look, as the properties
        // do (the plan's own look is put aside meanwhile).
        let Some(v) = self.ed.model().viewport(id).cloned() else {
            self.views.vg_viewport = None;
            return;
        };
        let name = archisim_plan::sheet::viewport_title(self.ed.model(), &v);
        // While the mouse is held (a slider), the look waits here.
        let start = match self.views.vg_pending.take() {
            Some((pid, s)) if pid == id => s,
            _ => v.style.clone(),
        };
        let saved = std::mem::replace(&mut self.view_style, start);
        self.vg_window_of(ctx, Some(format!("Representación · {name} (en el plano)")));
        let style = std::mem::replace(&mut self.view_style, saved);
        if style != v.style && ctx.input(|i| i.pointer.any_down()) {
            self.views.vg_pending = Some((id, style));
        } else if style != v.style {
            let mut new = v;
            new.style = style;
            let r = self.ed.transact("Gráficos de la vista del plano", |m| {
                to_template(m, &new.style)?;
                m.set_viewport(new)
            });
            if let Err(e) = r {
                self.complain(format!("No se puede guardar la vista: {e}."));
            }
        }
        if !self.views.vg_open {
            self.views.vg_viewport = None;
        }
    }

    fn vg_window_of(&mut self, ctx: &egui::Context, title: Option<String>) {
        let p = Palette::current(ctx);
        let title = match title {
            Some(t) => t,
            None => match self.active_saved().and_then(|id| self.saved_view_name(id)) {
                Some(n) => format!("Representación · {n}"),
                None => {
                    let l = self
                        .ed
                        .model()
                        .level(self.ed.level)
                        .map(|l| l.name.clone())
                        .unwrap_or_default();
                    format!("Representación · {l}")
                }
            },
        };
        let mut open = true;
        let mut style = self.view_style.clone();
        let templates: Vec<(ViewTemplateId, String)> = self
            .ed
            .model()
            .view_templates()
            .map(|t| (t.id, t.name.clone()))
            .collect();
        let mut apply: Option<Option<ViewTemplateId>> = None;
        let mut save_as = None;
        let mut delete_template = None;
        let mut open_graphics = false;
        let mut open_styles = false;
        let layers = self.layer_options();
        let following = self.followed_template();
        let followers = following
            .as_ref()
            .map_or(0, |t| self.ed.model().template_followers(t.id));
        let presentations: Vec<(Option<archisim_core::PresentationStyleId>, String)> =
            std::iter::once((None, "Técnica (sin estilo)".to_string()))
                .chain(
                    self.ed
                        .model()
                        .presentation_styles()
                        .map(|s| (Some(s.id), s.name.clone())),
                )
                .collect();
        super::widgets::dialog(title)
            .id(egui::Id::unique("vg-window"))
            .open(&mut open)
            .default_width(780.0)
            .default_height(560.0)
            .resizable(true)
            .show(ctx, |ui| {
                // The view's template first: the first of the three levels
                // (project, template, view; `docs/INTERFAZ.md` §13.5).
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("Plantilla de vista").strong());
                    let shown = following
                        .as_ref()
                        .map_or("Ninguna: aspecto propio".to_string(), |t| t.name.clone());
                    egui::ComboBox::from_id_salt("vg-template")
                        .selected_text(shown)
                        .width(220.0)
                        .show_ui(ui, |ui| {
                            if ui
                                .selectable_label(following.is_none(), "Ninguna: aspecto propio")
                                .clicked()
                            {
                                apply = Some(None);
                            }
                            for (id, name) in &templates {
                                let on = following.as_ref().is_some_and(|t| t.id == *id);
                                if ui.selectable_label(on, name).clicked() {
                                    apply = Some(Some(*id));
                                }
                            }
                        });
                    egui::ComboBox::from_id_salt("vg-template-delete")
                        .selected_text("Borrar plantilla…")
                        .show_ui(ui, |ui| {
                            for (id, name) in &templates {
                                if ui.selectable_label(false, name).clicked() {
                                    delete_template = Some(*id);
                                }
                            }
                        });
                });
                match &following {
                    Some(t) => {
                        let others = followers.saturating_sub(1);
                        ui.label(
                            egui::RichText::new(format!(
                                "Esta vista sigue «{}»: lo que cambies aquí cambia la plantilla{}. Para cambiar solo esta vista, deja de seguirla (Ninguna).",
                                t.name,
                                if others > 0 {
                                    format!(" y las otras {others} vistas que la siguen")
                                } else {
                                    String::new()
                                }
                            ))
                            .color(p.accent),
                        );
                    }
                    None => {
                        ui.horizontal(|ui| {
                            ui.label(
                                egui::RichText::new("Aspecto propio de esta vista.").color(p.muted),
                            );
                            ui.add(
                                egui::TextEdit::singleline(&mut self.views.template_name)
                                    .hint_text("Nombre de la plantilla")
                                    .desired_width(160.0),
                            );
                            if ui
                                .button("Crear plantilla con este aspecto")
                                .on_hover_text("La vista pasa a seguirla; dale la misma plantilla a otras vistas para que se dibujen igual")
                                .clicked()
                            {
                                save_as = Some(self.views.template_name.trim().to_string());
                            }
                        });
                    }
                }
                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    for (t, label) in [
                        (VgTab::Categories, "Categorías del modelo"),
                        (VgTab::Layers, "Capas de dibujo 2D"),
                        (VgTab::Look, "Vista y presentación"),
                    ] {
                        if ui.selectable_label(self.views.tab == t, label).clicked() {
                            self.views.tab = t;
                        }
                    }
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui
                            .button("Normas gráficas…")
                            .on_hover_text("Del proyecto, para todas las vistas: grosores de línea por escala y letra")
                            .clicked()
                        {
                            open_graphics = true;
                        }
                        if ui
                            .button("Restablecer todo")
                            .on_hover_text("Todas las categorías y capas como siempre")
                            .clicked()
                        {
                            style.overrides.clear();
                            style.hidden.clear();
                            style.hidden_layers.clear();
                        }
                    });
                });
                ui.separator();
                let tab = self.views.tab;
                egui::ScrollArea::vertical().show(ui, |ui| {
                    if tab == VgTab::Categories {
                    ui.label(
                        egui::RichText::new("Cómo se dibuja cada categoría. Lo que no se toca queda como siempre; «Auto» deja el color propio de cada elemento.")
                            .color(p.muted),
                    );
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
                    }
                    if tab == VgTab::Layers {
                        ui.label(
                            egui::RichText::new("Las capas del dibujo 2D (líneas, textos, rellenos, DXF) que se ven en esta vista.")
                                .color(p.muted),
                        );
                        ui.add_space(4.0);
                        egui::Grid::new("vg-layers")
                            .striped(true)
                            .spacing([10.0, 4.0])
                            .show(ui, |ui| {
                                for (id, name) in &layers {
                                    ui.label(name);
                                    let mut on = style.shows_layer(*id);
                                    if ui.switch(&mut on, "").changed() {
                                        if on {
                                            style.hidden_layers.remove(id);
                                        } else {
                                            style.hidden_layers.insert(*id);
                                        }
                                    }
                                    ui.end_row();
                                }
                            });
                    }
                    if tab == VgTab::Look {
                        look_tab(ui, &mut style, &presentations, &mut open_styles);
                    }
                });
            });
        if open_graphics {
            self.show_graphics = true;
        }
        if open_styles {
            self.presentation.open = true;
            self.presentation.selected = style.presentation;
        }
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
                        Some(id) => m.set_view_template(ViewTemplate { id, ..t }).map(|_| id),
                        None => m.add_view_template(t),
                    }
                });
                match r {
                    Ok(id) => {
                        self.views.template_name.clear();
                        self.apply_template(Some(id));
                    }
                    Err(e) => self.complain(format!("No se puede guardar la plantilla: {e}.")),
                }
            }
        }
        if let Some(id) = delete_template {
            let r = self.ed.transact("Borrar plantilla de vista", |m| {
                m.remove_view_template(id).map(drop)
            });
            if r.is_ok() && self.view_style.template == Some(id) {
                self.view_style.template = None;
            }
            self.report(
                r,
                "Plantilla borrada (las vistas que la seguían conservan su aspecto).",
            );
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
                .switch(&mut set, "")
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
    if ui.switch(&mut on, "").changed() {
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
    ui.switch(&mut g.halftone, "");
    ui.switch(&mut g.patterns, "");
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
