//! The open views, as tabs over the drawing (ArchiCAD's tab bar): each
//! level's plan and the 3D view. A plan tab keeps its own framing, drawing
//! scale and cut plane; the active one lives in the app's fields (`view`,
//! `plan_scale`, `cut_plane`, `ed.level`) so that the plan code needs to
//! know nothing about tabs.

use archisim_core::{LevelId, SheetId};
use eframe::egui::{
    self, CornerRadius, FontId, PointerButton, Pos2, Rect, Sense, Stroke, StrokeKind, Ui, Vec2,
};

use super::icons::{self, Icon};
use super::theme::Palette;
use super::widgets::strong_font;
use super::{ArchiSimApp, ViewMode};
use crate::view::View;

/// What a tab shows.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TabKind {
    Plan(LevelId),
    ThreeD,
    /// A sheet: paper with views placed on it.
    Sheet(SheetId),
}

/// A view open in a tab, with what it remembers while another is shown.
#[derive(Debug, Clone, PartialEq)]
pub struct ViewTab {
    pub kind: TabKind,
    pub view: View,
    /// Drawing scale denominator (50 means 1:50).
    pub scale: f64,
    /// Height of the cut plane above the level, in metres.
    pub cut_plane: f64,
    /// Framed once already (a new tab is fitted to its level).
    pub fitted: bool,
    /// What the view shows: categories, drawing layers, detail.
    pub style: archisim_core::ViewStyle,
}

impl ViewTab {
    pub fn plan(level: LevelId) -> Self {
        let d = archisim_plan::PlanOptions::default();
        Self {
            kind: TabKind::Plan(level),
            view: View::default(),
            scale: d.scale,
            cut_plane: d.cut_plane,
            fitted: false,
            style: Default::default(),
        }
    }

    pub fn three_d() -> Self {
        Self {
            kind: TabKind::ThreeD,
            ..Self::plan(LevelId(0))
        }
    }

    pub fn sheet(id: SheetId) -> Self {
        Self {
            kind: TabKind::Sheet(id),
            ..Self::plan(LevelId(0))
        }
    }
}

/// The scales offered for plans.
pub const SCALES: [f64; 4] = [20.0, 50.0, 100.0, 200.0];

impl ArchiSimApp {
    /// Keeps the active tab's state in its slot of the list.
    fn store_active_tab(&mut self) {
        let (view, scale, cut) = (self.view, self.plan_scale, self.cut_plane);
        let style = self.view_style.clone();
        if let Some(t) = self.tabs.get_mut(self.active_tab) {
            if matches!(t.kind, TabKind::Plan(_)) {
                t.view = view;
                t.scale = scale;
                t.cut_plane = cut;
                t.style = style;
                t.fitted = true;
            } else if matches!(t.kind, TabKind::Sheet(_)) {
                t.view = view;
                t.fitted = true;
            }
        }
    }

    /// Shows tab `i`.
    pub(super) fn activate_tab(&mut self, i: usize) {
        if i >= self.tabs.len() {
            return;
        }
        if i != self.active_tab {
            self.store_active_tab();
        }
        self.active_tab = i;
        let tab = self.tabs[i].clone();
        match tab.kind {
            TabKind::Plan(level) => {
                self.view = tab.view;
                self.plan_scale = tab.scale;
                self.cut_plane = tab.cut_plane;
                self.view_style = tab.style;
                if !tab.fitted {
                    self.fit_requested = true;
                }
                if level != self.ed.level {
                    self.switch_level(level);
                }
                self.mode = ViewMode::Plan;
                self.last_plan_tab = i;
            }
            TabKind::ThreeD => {
                self.set_tool(super::Tool::Arrow);
                self.mode = ViewMode::ThreeD;
            }
            TabKind::Sheet(id) => {
                if !matches!(self.tool, super::Tool::Arrow | super::Tool::Cad) {
                    self.set_tool(super::Tool::Arrow);
                }
                self.end_command(true);
                self.ed.selection.clear();
                self.sheet_sel = None;
                self.view = tab.view;
                if !tab.fitted {
                    self.fit_requested = true;
                }
                self.mode = ViewMode::Sheet(id);
            }
        }
    }

    /// Opens a view, or shows it if a tab has it already.
    pub(super) fn open_view(&mut self, kind: TabKind) {
        if let Some(i) = self.tabs.iter().position(|t| t.kind == kind) {
            self.activate_tab(i);
            return;
        }
        let tab = match kind {
            TabKind::Plan(level) => ViewTab::plan(level),
            TabKind::ThreeD => ViewTab::three_d(),
            TabKind::Sheet(id) => ViewTab::sheet(id),
        };
        // Next to the active tab, as browsers do.
        let at = (self.active_tab + 1).min(self.tabs.len());
        self.store_active_tab();
        self.tabs.insert(at, tab);
        if self.last_plan_tab >= at {
            self.last_plan_tab += 1;
        }
        // The active slot moved too if it was after the new one.
        if self.active_tab >= at {
            self.active_tab += 1;
        }
        self.activate_tab(at);
    }

    /// Closes tab `i`; the last one cannot be closed.
    pub(super) fn close_tab(&mut self, i: usize) {
        if self.tabs.len() <= 1 || i >= self.tabs.len() {
            return;
        }
        self.store_active_tab();
        let was_active = i == self.active_tab;
        self.tabs.remove(i);
        if self.active_tab > i || self.active_tab >= self.tabs.len() {
            self.active_tab = self.active_tab.saturating_sub(1);
        }
        if self.last_plan_tab > i || self.last_plan_tab >= self.tabs.len() {
            self.last_plan_tab = self.last_plan_tab.saturating_sub(1);
        }
        if was_active {
            let next = self.active_tab;
            // Load it without storing the closed one.
            self.active_tab = usize::MAX;
            self.activate_tab(next);
        }
    }

    /// Shows a plan: the last plan tab used, or the current level's.
    pub(super) fn show_plan(&mut self) {
        if self.mode == ViewMode::Plan {
            return;
        }
        let i = self.last_plan_tab;
        if self
            .tabs
            .get(i)
            .is_some_and(|t| matches!(t.kind, TabKind::Plan(_)))
        {
            self.activate_tab(i);
        } else {
            self.open_view(TabKind::Plan(self.ed.level));
        }
    }

    /// Follows changes made elsewhere: levels deleted (their tabs close),
    /// the current level changed by an edit (adding a level makes it
    /// current) or a project opened.
    pub(super) fn sync_tabs(&mut self) {
        let exists = |m: &archisim_core::Model, l: LevelId| m.level(l).is_some();
        let model = self.ed.model();
        let stale: Vec<usize> = (0..self.tabs.len())
            .filter(|&i| match self.tabs[i].kind {
                TabKind::Plan(l) => !exists(model, l),
                TabKind::Sheet(s) => model.sheet(s).is_none(),
                TabKind::ThreeD => false,
            })
            .collect();
        for i in stale.into_iter().rev() {
            if self.tabs.len() > 1 {
                self.close_tab(i);
            } else {
                self.tabs[0] = ViewTab::plan(self.ed.level);
                self.active_tab = usize::MAX;
                self.activate_tab(0);
            }
        }
        if self.tabs.is_empty() {
            self.tabs.push(ViewTab::plan(self.ed.level));
            self.active_tab = usize::MAX;
            self.activate_tab(0);
        }
        if let Some(TabKind::Plan(l)) = self.tabs.get(self.active_tab).map(|t| t.kind) {
            if l != self.ed.level {
                let level = self.ed.level;
                match self
                    .tabs
                    .iter()
                    .position(|t| t.kind == TabKind::Plan(level))
                {
                    Some(i) => {
                        // Show that tab with its own state.
                        self.ed.level = l;
                        self.activate_tab(i);
                    }
                    None => {
                        // A level just added (it becomes current): in a
                        // tab of its own, beside the one being shown.
                        self.ed.level = l;
                        self.open_view(TabKind::Plan(level));
                    }
                }
            }
        }
        self.mode = match self.tabs.get(self.active_tab).map(|t| t.kind) {
            Some(TabKind::ThreeD) => ViewMode::ThreeD,
            Some(TabKind::Sheet(s)) => ViewMode::Sheet(s),
            _ => ViewMode::Plan,
        };
    }

    /// Changes the current level, ending what was being drawn.
    pub(super) fn switch_level(&mut self, level: LevelId) {
        if level == self.ed.level {
            return;
        }
        self.ed.level = level;
        self.ed.selection.clear();
        self.wall_tool.finish();
        self.slab_tool.finish();
        self.dim_tool.finish();
        self.drag = None;
        self.window_start = None;
    }

    pub(super) fn tab_title(&self, kind: TabKind) -> String {
        match kind {
            TabKind::Plan(l) => self
                .ed
                .model()
                .level(l)
                .map_or_else(|| "Planta".into(), |l| l.name.clone()),
            TabKind::ThreeD => "Vista 3D".into(),
            TabKind::Sheet(s) => self
                .ed
                .model()
                .sheet(s)
                .map_or_else(|| "Plano".into(), |s| format!("{} {}", s.number, s.name)),
        }
    }

    /// The strip of tabs over the drawing, with a button to open views.
    pub(super) fn tab_strip(&mut self, ui: &mut Ui) {
        let p = Palette::current(ui.ctx());
        let height = 34.0;
        let (strip, _) =
            ui.allocate_exact_size(Vec2::new(ui.available_width(), height), Sense::hover());
        ui.painter()
            .rect_filled(strip, CornerRadius::ZERO, p.ground);
        // Bottom line of the strip; the active tab opens over it.
        ui.painter().line_segment(
            [
                Pos2::new(strip.left(), strip.bottom() - 0.5),
                Pos2::new(strip.right(), strip.bottom() - 0.5),
            ],
            Stroke::new(1.0, p.line),
        );
        let mut x = strip.left() + 6.0;
        let mut activate = None;
        let mut close = None;
        let closable = self.tabs.len() > 1;
        for (i, tab) in self.tabs.iter().enumerate() {
            let title = self.tab_title(tab.kind);
            let active = i == self.active_tab;
            let font = if active {
                strong_font(12.5)
            } else {
                FontId::proportional(12.5)
            };
            let fg = if active { p.text } else { p.muted };
            let galley = ui.painter().layout_no_wrap(title, font, fg);
            let scale = match tab.kind {
                TabKind::Plan(_) => {
                    let s = if active { self.plan_scale } else { tab.scale };
                    format!("1:{s}")
                }
                TabKind::ThreeD => String::new(),
                TabKind::Sheet(s) => self
                    .ed
                    .model()
                    .sheet(s)
                    .map(|s| s.size_name())
                    .unwrap_or_default(),
            };
            let note = ui
                .painter()
                .layout_no_wrap(scale, FontId::monospace(10.5), p.muted);
            let w = 12.0 + 18.0 + galley.size().x + 8.0 + note.size().x + 30.0;
            let rect =
                Rect::from_min_size(Pos2::new(x, strip.top() + 5.0), Vec2::new(w, height - 5.0));
            x += w + 4.0;
            let id = ui.make_persistent_id(("tab", i));
            let resp = ui
                .interact(rect, id, Sense::click())
                .on_hover_text(match tab.kind {
                    TabKind::Plan(_) => "Planta · clic con la rueda: cerrar",
                    TabKind::ThreeD => "Vista 3D · clic con la rueda: cerrar",
                    TabKind::Sheet(_) => "Plano · clic con la rueda: cerrar",
                });
            let painter = ui.painter();
            if active {
                painter.rect(
                    rect,
                    CornerRadius {
                        nw: 8,
                        ne: 8,
                        sw: 0,
                        se: 0,
                    },
                    p.panel,
                    Stroke::new(1.0, p.line),
                    StrokeKind::Inside,
                );
                // Opens onto the drawing below.
                painter.line_segment(
                    [
                        Pos2::new(rect.left() + 1.0, rect.bottom()),
                        Pos2::new(rect.right() - 1.0, rect.bottom()),
                    ],
                    Stroke::new(2.0, p.panel),
                );
                painter.line_segment(
                    [
                        Pos2::new(rect.left() + 8.0, rect.top() + 0.5),
                        Pos2::new(rect.right() - 8.0, rect.top() + 0.5),
                    ],
                    Stroke::new(2.0, p.accent),
                );
            } else if resp.hovered() {
                painter.rect_filled(
                    rect.shrink2(Vec2::new(0.0, 2.0)),
                    CornerRadius::same(7),
                    p.raised,
                );
            }
            let cy = rect.center().y + 1.0;
            let icon = match tab.kind {
                TabKind::Plan(_) => Icon::Plan,
                TabKind::ThreeD => Icon::ThreeD,
                TabKind::Sheet(_) => Icon::Sheet,
            };
            icons::paint(
                painter,
                Rect::from_center_size(Pos2::new(rect.left() + 19.0, cy), Vec2::splat(15.0)),
                icon,
                if active { p.accent } else { p.muted },
            );
            let tx = rect.left() + 32.0;
            let gsize = galley.size();
            painter.galley(Pos2::new(tx, cy - gsize.y / 2.0), galley, fg);
            let nsize = note.size();
            painter.galley(
                Pos2::new(tx + gsize.x + 8.0, cy - nsize.y / 2.0),
                note,
                p.muted,
            );
            // Close button.
            let xr = Rect::from_center_size(Pos2::new(rect.right() - 14.0, cy), Vec2::splat(18.0));
            let xresp = ui.interact(xr, id.with("close"), Sense::click());
            if closable && (active || resp.hovered() || xresp.hovered()) {
                if xresp.hovered() {
                    ui.painter()
                        .rect_filled(xr, CornerRadius::same(4), p.accent_soft);
                }
                icons::paint(
                    ui.painter(),
                    xr.shrink(4.0),
                    Icon::Close,
                    if xresp.hovered() { p.text } else { p.muted },
                );
            }
            if closable && (xresp.clicked() || resp.clicked_by(PointerButton::Middle)) {
                close = Some(i);
            } else if resp.clicked() {
                activate = Some(i);
            }
        }
        // "+": open a view.
        let plus = Rect::from_min_size(
            Pos2::new(x + 2.0, strip.top() + 7.0),
            Vec2::new(26.0, height - 11.0),
        );
        let presp = ui
            .interact(plus, ui.make_persistent_id("tab-plus"), Sense::click())
            .on_hover_text("Abrir una vista");
        if presp.hovered() {
            ui.painter()
                .rect_filled(plus, CornerRadius::same(6), p.raised);
        }
        icons::paint(ui.painter(), plus.shrink(5.0), Icon::Plus, p.muted);
        let levels: Vec<(LevelId, String)> = self
            .ed
            .model()
            .levels()
            .iter()
            .rev()
            .map(|l| (l.id, l.name.clone()))
            .collect();
        let sheets: Vec<(SheetId, String)> = self
            .ed
            .model()
            .sheets()
            .iter()
            .map(|s| (s.id, format!("{} {}", s.number, s.name)))
            .collect();
        let mut open = None;
        egui::Popup::menu(&presp).show(|ui| {
            ui.label(egui::RichText::new("Plantas").small().weak());
            for (id, name) in &levels {
                if ui.button(name).clicked() {
                    open = Some(TabKind::Plan(*id));
                }
            }
            ui.separator();
            if ui.button("Vista 3D").clicked() {
                open = Some(TabKind::ThreeD);
            }
            if !sheets.is_empty() {
                ui.separator();
                ui.label(egui::RichText::new("Planos").small().weak());
                for (id, name) in &sheets {
                    if ui.button(name).clicked() {
                        open = Some(TabKind::Sheet(*id));
                    }
                }
            }
        });
        if let Some(i) = close {
            self.close_tab(i);
        } else if let Some(i) = activate {
            self.activate_tab(i);
        }
        if let Some(kind) = open {
            self.open_view(kind);
        }
    }
}
