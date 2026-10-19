//! Sheets in the window: the sheet as paper in its tab (views placed on it
//! can be selected, moved and opened; 2D drafting works on it as in a
//! plan), its properties and those of the selected view, and new sheets.

use archisim_core::{
    NewSheet, NewViewport, ScheduleKind, Sheet, SheetId, SheetTable, TableId, ViewRef, Viewport,
    ViewportId,
};
use archisim_plan::sheet::{render_sheet, viewport_rect, SheetOptions};
use eframe::egui::{self, Ui};

use super::props::{self, choice, group, row, text_row};
use super::tabs::SCALES;
use super::*;

/// What the sheet picture was drawn for.
#[derive(Debug, Clone, PartialEq)]
pub struct SheetKey {
    revision: u64,
    sheet: SheetId,
    lo: Point2,
    hi: Point2,
    size: [usize; 2],
}

/// Something placed on a sheet: a view or a schedule.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Placed {
    View(ViewportId),
    Table(TableId),
}

/// A view or schedule being dragged on its sheet.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ViewportDrag {
    pub item: Placed,
    pub from: Point2,
    /// The view's centre, or the schedule's top-left corner.
    pub origin: Point2,
}

/// The paper a schedule takes: lower-left and upper-right, metres.
pub fn table_rect(model: &archisim_core::Model, t: &SheetTable) -> (Point2, Point2) {
    let (w, h) = archisim_plan::schedules::table(model, t.kind, t.level).size();
    (
        Point2::new(t.at.x, t.at.y - h * MM),
        Point2::new(t.at.x + w * MM, t.at.y),
    )
}

/// Paper metres per millimetre.
const MM: f64 = 0.001;

impl ArchiSimApp {
    /// The view drafting and commands work on: the plan's level, or the
    /// sheet shown.
    pub(super) fn active_view(&self) -> ViewRef {
        match self.mode {
            ViewMode::Sheet(s) => ViewRef::Sheet(s),
            _ => ViewRef::Level(self.ed.level),
        }
    }

    /// Drawing units per paper millimetre in the active view.
    pub(super) fn active_k(&self) -> f64 {
        match self.mode {
            ViewMode::Sheet(_) => MM,
            _ => self.plan_scale / 1000.0,
        }
    }

    /// The element under a point of the active view.
    pub(super) fn pick_here(&self, p: Point2, reach: f64) -> Option<crate::pick::Element> {
        match self.mode {
            ViewMode::Sheet(s) => crate::pick::pick_on_sheet(self.ed.model(), s, p, reach),
            _ => crate::pick::pick(&self.ed, self.ed.level, p, reach),
        }
    }

    /// What a selection window takes in the active view.
    pub(super) fn window_here(&self, a: Point2, b: Point2) -> Vec<crate::pick::Element> {
        match self.mode {
            ViewMode::Sheet(s) => crate::pick::in_window_on_sheet(self.ed.model(), s, a, b),
            _ => crate::pick::in_window(&self.ed, self.ed.level, a, b),
        }
    }

    /// A new sheet, A1 landscape, with the next number, opened in its tab.
    pub(super) fn new_sheet(&mut self) {
        let number = self.ed.model().next_sheet_number();
        let r = self.ed.add_sheet(NewSheet {
            number: number.clone(),
            name: "Plano nuevo".into(),
            width: 841.0,
            height: 594.0,
        });
        match r {
            Ok(id) => {
                self.say(format!(
                    "Plano {number} creado. Añade vistas en sus propiedades."
                ));
                self.open_view(TabKind::Sheet(id));
            }
            Err(e) => self.complain(format!("No se puede: {e}.")),
        }
    }

    /// The sheet in the central area.
    pub(super) fn sheet_view(&mut self, ui: &mut Ui, id: SheetId) {
        let (rect, response) = ui.allocate_exact_size(ui.available_size(), Sense::click_and_drag());
        let ctx = ui.ctx().clone();
        let painter = ui.painter_at(rect);
        self.canvas = rect;
        let Some(sheet) = self.ed.model().sheet(id).cloned() else {
            return;
        };
        let (w, h) = (sheet.width * MM, sheet.height * MM);
        if self.fit_requested && rect.width() > 50.0 {
            self.fit_requested = false;
            self.view
                .fit_exact(rect, Point2::new(0.0, 0.0), Point2::new(w, h), 30.0);
        }
        // Pan and zoom as in plan.
        if response.dragged_by(PointerButton::Middle)
            || response.dragged_by(PointerButton::Secondary)
        {
            self.view.pan(response.drag_delta());
        }
        if response.double_clicked_by(PointerButton::Middle) {
            self.fit_requested = true;
        }
        if let Some(pos) = response.hover_pos() {
            let (scroll, zoom) = ctx.input(|i| (i.smooth_scroll_delta, i.zoom_delta()));
            let factor = zoom as f64 * (scroll.y as f64 / 300.0).exp();
            if (factor - 1.0).abs() > 1e-6 {
                self.view.zoom_at(rect, pos, factor);
            }
        }
        self.sheet_cursor(&ctx, &response, rect, id);
        if ctx.memory(|m| m.focused().is_none()) {
            self.sheet_keys(&ctx);
        }
        self.sheet_clicks(&ctx, &response, rect, id);

        // Grey desk, the paper, the drawing, then what is going on.
        painter.rect_filled(rect, CornerRadius::ZERO, Color32::from_rgb(150, 154, 160));
        let paper = Rect::from_two_pos(
            self.view.to_screen(rect, Point2::new(0.0, 0.0)),
            self.view.to_screen(rect, Point2::new(w, h)),
        );
        painter.rect_filled(
            paper.translate(Vec2::new(4.0, 4.0)),
            CornerRadius::ZERO,
            Color32::from_black_alpha(60),
        );
        self.draw_sheet_image(&ctx, &painter, rect, id);
        // Views and schedules: their outline when hovered or selected.
        let model = self.ed.model();
        let placed: Vec<(Placed, (Point2, Point2))> = model
            .viewports_on(id)
            .map(|v| (Placed::View(v.id), viewport_rect(model, v)))
            .chain(
                model
                    .tables_on(id)
                    .map(|t| (Placed::Table(t.id), table_rect(model, t))),
            )
            .collect();
        for (item, (lo, hi)) in placed {
            let r =
                Rect::from_two_pos(self.view.to_screen(rect, lo), self.view.to_screen(rect, hi));
            let selected = self.sheet_sel == Some(item);
            let hovered = self.sheet_hover == Some(item);
            if selected || hovered {
                painter.rect_stroke(
                    r,
                    CornerRadius::ZERO,
                    Stroke::new(
                        if selected { 2.0 } else { 1.0 },
                        ACCENT.gamma_multiply(if selected { 1.0 } else { 0.5 }),
                    ),
                    StrokeKind::Outside,
                );
            }
        }
        self.draw_selection(&painter, rect);
        self.draw_cad(&painter, rect);
        if let (Some(a), Some(c)) = (self.window_start, self.cursor) {
            let r = Rect::from_two_pos(
                self.view.to_screen(rect, a),
                self.view.to_screen(rect, c.point),
            );
            painter.rect_filled(r, CornerRadius::ZERO, ACCENT.gamma_multiply(0.08));
            painter.rect_stroke(
                r,
                CornerRadius::ZERO,
                Stroke::new(1.0, ACCENT),
                StrokeKind::Inside,
            );
        }
        if self.tool == Tool::Cad {
            self.draw_sheet_cursor(&painter, rect);
        }
        self.draw_overlays(&ctx, &painter, rect);
        if response.hovered() {
            ctx.set_cursor_icon(if self.tool == Tool::Cad {
                CursorIcon::Crosshair
            } else if self.sheet_drag.is_some() {
                CursorIcon::Grabbing
            } else if self.sheet_hover.is_some() {
                CursorIcon::Move
            } else {
                CursorIcon::Default
            });
        }
    }

    fn draw_sheet_image(
        &mut self,
        ctx: &egui::Context,
        painter: &egui::Painter,
        rect: Rect,
        id: SheetId,
    ) {
        let ppp = ctx.pixels_per_point();
        let size = [
            (rect.width() * ppp).round().max(1.0) as usize,
            (rect.height() * ppp).round().max(1.0) as usize,
        ];
        let (lo, hi) = self.view.visible(rect);
        let key = SheetKey {
            revision: self.ed.revision(),
            sheet: id,
            lo,
            hi,
            size,
        };
        if self.sheet_image.as_ref().map(|p| &p.0) != Some(&key) {
            let px_per_m = size[0] as f64 / (hi.x - lo.x).max(1e-9);
            let svg = render_sheet(
                self.ed.model(),
                id,
                &SheetOptions {
                    region: Some((lo, hi)),
                    px_per_mm: Some(px_per_m * MM),
                    ..Default::default()
                },
            );
            let image = crate::plan_image::rasterize_svg(&svg, size);
            match &mut self.sheet_image {
                Some((k, tex)) => {
                    tex.set(image, TextureOptions::LINEAR);
                    *k = key;
                }
                None => {
                    let tex = ctx.load_texture("sheet", image, TextureOptions::LINEAR);
                    self.sheet_image = Some((key, tex));
                }
            }
        }
        if let Some((_, tex)) = &self.sheet_image {
            painter.image(
                tex.id(),
                rect,
                Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)),
                Color32::WHITE,
            );
        }
    }

    /// The schedule or view of the sheet under a point.
    fn viewport_at(&self, id: SheetId, p: Point2) -> Option<Placed> {
        let model = self.ed.model();
        let inside =
            |(lo, hi): (Point2, Point2)| p.x >= lo.x && p.x <= hi.x && p.y >= lo.y && p.y <= hi.y;
        model
            .tables_on(id)
            .find_map(|t| inside(table_rect(model, t)).then_some(Placed::Table(t.id)))
            .or_else(|| {
                model
                    .viewports_on(id)
                    .find_map(|v| inside(viewport_rect(model, v)).then_some(Placed::View(v.id)))
            })
    }

    fn sheet_cursor(
        &mut self,
        ctx: &egui::Context,
        response: &egui::Response,
        rect: Rect,
        id: SheetId,
    ) {
        let Some(pos) = response.hover_pos() else {
            self.cursor = None;
            self.hover = None;
            self.sheet_hover = None;
            return;
        };
        let raw = self.view.to_model(rect, pos);
        let reach = self.view.to_metres(REACH);
        if self.tool == Tool::Cad {
            self.cad_cursor(ctx, raw, reach);
            self.sheet_hover = None;
            return;
        }
        self.cursor = Some(Snap::at(raw, SnapKind::Free));
        self.hover = crate::pick::pick_on_sheet(self.ed.model(), id, raw, reach);
        self.sheet_hover = if self.hover.is_some() {
            None
        } else {
            self.viewport_at(id, raw)
        };
    }

    fn sheet_keys(&mut self, ctx: &egui::Context) {
        let (events, enter, escape, backspace, delete, space) = ctx.input(|i| {
            (
                i.events.clone(),
                i.key_pressed(Key::Enter),
                i.key_pressed(Key::Escape),
                i.key_pressed(Key::Backspace),
                i.key_pressed(Key::Delete),
                i.key_pressed(Key::Space),
            )
        });
        if self.tool == Tool::Cad
            && ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, Key::F8))
        {
            self.toggle_ortho();
        }
        if self.command_keys(&events, enter, escape, backspace, space) {
            return;
        }
        if self.tool == Tool::Cad {
            return;
        }
        if escape {
            self.sheet_sel = None;
            self.ed.selection.clear();
            self.window_start = None;
        }
        if delete || backspace {
            if let Some(item) = self.sheet_sel.take() {
                match item {
                    Placed::View(v) => {
                        let r = self.ed.remove_viewport(v);
                        self.report(r, "Vista quitada del plano (la planta sigue en el modelo).");
                    }
                    Placed::Table(t) => {
                        let r = self.ed.remove_table(t);
                        self.report(r, "Tabla quitada del plano.");
                    }
                }
            } else if !self.ed.selection.is_empty() {
                self.delete();
            }
        }
    }

    fn sheet_clicks(
        &mut self,
        ctx: &egui::Context,
        response: &egui::Response,
        rect: Rect,
        id: SheetId,
    ) {
        if self.tool == Tool::Cad {
            self.cad_clicks(ctx, response, rect);
            return;
        }
        let shift = ctx.input(|i| i.modifiers.shift || i.modifiers.command);
        if response.double_clicked_by(PointerButton::Primary) {
            // Into the plan of the view.
            let view = match self.sheet_sel {
                Some(Placed::View(v)) => self.ed.model().viewport(v).cloned(),
                _ => None,
            };
            if let Some(v) = view {
                self.open_view(TabKind::Plan(v.level));
                return;
            }
        }
        if response.clicked_by(PointerButton::Primary) {
            match (self.hover, self.sheet_hover) {
                (Some(e), _) => {
                    self.sheet_sel = None;
                    if shift {
                        if !self.ed.selection.remove(&e) {
                            self.ed.selection.insert(e);
                        }
                    } else {
                        self.ed.selection.clear();
                        self.ed.selection.insert(e);
                    }
                }
                (None, Some(v)) => {
                    self.ed.selection.clear();
                    self.sheet_sel = Some(v);
                }
                (None, None) => {
                    self.ed.selection.clear();
                    self.sheet_sel = None;
                }
            }
        }
        if response.drag_started_by(PointerButton::Primary) {
            let origin = ctx
                .input(|i| i.pointer.press_origin())
                .map(|p| self.view.to_model(rect, p));
            if let Some(o) = origin {
                let on_view = self.viewport_at(id, o);
                let on_draft =
                    crate::pick::pick_on_sheet(self.ed.model(), id, o, self.view.to_metres(REACH));
                match (on_draft, on_view) {
                    (None, Some(item)) => {
                        let model = self.ed.model();
                        let origin = match item {
                            Placed::View(v) => model.viewport(v).map(|v| v.centre),
                            Placed::Table(t) => model.table(t).map(|t| t.at),
                        }
                        .unwrap_or(o);
                        self.sheet_sel = Some(item);
                        self.sheet_drag = Some(ViewportDrag {
                            item,
                            from: o,
                            origin,
                        });
                    }
                    (Some(e), _) if self.ed.selection.contains(&e) => {
                        self.drag = Some(DragState {
                            grip: crate::drag::Grip::Body,
                            from: o,
                            typed: String::new(),
                            skip: [e].into_iter().collect(),
                        });
                    }
                    _ => self.window_start = Some(o),
                }
            }
        }
        if response.drag_stopped() {
            if let (Some(d), Some(c)) = (self.sheet_drag.take(), self.cursor) {
                let to = d.origin + (c.point - d.from);
                match d.item {
                    Placed::View(v) => {
                        if let Some(mut v) = self.ed.model().viewport(v).cloned() {
                            v.centre = to;
                            let r = self.ed.set_viewport(v, "Mover vista del plano");
                            self.report(r, "Vista movida.");
                        }
                    }
                    Placed::Table(t) => {
                        if let Some(mut t) = self.ed.model().table(t).cloned() {
                            t.at = to;
                            let r = self.ed.set_table(t, "Mover tabla del plano");
                            self.report(r, "Tabla movida.");
                        }
                    }
                }
            }
            if let (Some(d), Some(c)) = (self.drag.take(), self.cursor) {
                let delta = archisim_draft::Transform::translate(c.point - d.from);
                let copy = ctx.input(|i| i.modifiers.command);
                let step = crate::cad::Changes {
                    label: if copy { "Copiar" } else { "Mover" },
                    changes: vec![crate::cad::Change::Transform {
                        items: self.ed.selection.clone(),
                        t: delta,
                        copy,
                    }],
                };
                let st = self.cad_settings.clone();
                if let Err(e) = self.ed.apply_cad(ViewRef::Sheet(id), &st, &step) {
                    self.complain(format!("No se puede: {e}."));
                }
            }
            if let (Some(a), Some(c)) = (self.window_start.take(), self.cursor) {
                let found = crate::pick::in_window_on_sheet(self.ed.model(), id, a, c.point);
                if !shift {
                    self.ed.selection.clear();
                }
                self.ed.selection.extend(found);
            }
        }
        // While dragging a view, it follows the cursor (drawn as its
        // outline).
        if let (Some(d), Some(c)) = (self.sheet_drag, self.cursor) {
            let model = self.ed.model();
            let to = d.origin + (c.point - d.from);
            let moved = match d.item {
                Placed::View(v) => model.viewport(v).map(|v| {
                    let mut m = v.clone();
                    m.centre = to;
                    viewport_rect(model, &m)
                }),
                Placed::Table(t) => model.table(t).map(|t| {
                    let mut m = t.clone();
                    m.at = to;
                    table_rect(model, &m)
                }),
            };
            if let Some((lo, hi)) = moved {
                let r = Rect::from_two_pos(
                    self.view.to_screen(rect, lo),
                    self.view.to_screen(rect, hi),
                );
                ctx.layer_painter(egui::LayerId::new(
                    egui::Order::Foreground,
                    Id::unique("vp-drag"),
                ))
                .rect_stroke(
                    r,
                    CornerRadius::ZERO,
                    Stroke::new(1.5, ACCENT),
                    StrokeKind::Outside,
                );
            }
        }
    }

    fn draw_sheet_cursor(&self, painter: &egui::Painter, rect: Rect) {
        let Some(c) = self.cursor else {
            return;
        };
        let p = self.view.to_screen(rect, c.point);
        let ink = Stroke::new(1.0, Color32::from_gray(40));
        painter.line_segment([p - Vec2::new(12.0, 0.0), p + Vec2::new(12.0, 0.0)], ink);
        painter.line_segment([p - Vec2::new(0.0, 12.0), p + Vec2::new(0.0, 12.0)], ink);
        if c.kind != SnapKind::Free {
            painter.rect_stroke(
                Rect::from_center_size(p, Vec2::splat(11.0)),
                CornerRadius::ZERO,
                Stroke::new(1.5, Color32::from_rgb(0, 150, 60)),
                StrokeKind::Middle,
            );
        }
    }

    /// Properties of the sheet shown, or of its selected view.
    pub(super) fn sheet_properties(&mut self, ui: &mut Ui, id: SheetId) {
        match self.sheet_sel {
            Some(Placed::View(v)) => {
                if let Some(v) = self.ed.model().viewport(v).cloned() {
                    self.viewport_properties(ui, v);
                    return;
                }
            }
            Some(Placed::Table(t)) => {
                if let Some(t) = self.ed.model().table(t).cloned() {
                    self.table_properties(ui, t);
                    return;
                }
            }
            None => {}
        }
        let Some(sheet) = self.ed.model().sheet(id).cloned() else {
            return;
        };
        props::title(
            ui,
            &format!("Plano {} · {}", sheet.number, sheet.size_name()),
        );
        let mut new = sheet.clone();
        group(ui, "sheet-id", "Plano", |ui| {
            row(ui, "Número", "Único en el proyecto", |ui, w| {
                if let Some(t) = text_edit_committed(
                    ui,
                    Id::unique(("sheet-number", id)),
                    &sheet.number,
                    w - 8.0,
                ) {
                    new.number = t;
                }
            });
            row(ui, "Nombre", "", |ui, w| {
                if let Some(t) =
                    text_edit_committed(ui, Id::unique(("sheet-name", id)), &sheet.name, w - 8.0)
                {
                    new.name = t;
                }
            });
            row(ui, "Revisión", "", |ui, w| {
                if let Some(t) =
                    text_edit_committed(ui, Id::unique(("sheet-rev", id)), &sheet.revision, w - 8.0)
                {
                    new.revision = t;
                }
            });
        });
        group(ui, "sheet-paper", "Papel", |ui| {
            row(ui, "Tamaño", "", |ui, w| {
                let landscape = sheet.width >= sheet.height;
                let mut opts: Vec<((u32, u32), String)> = archisim_core::PAPER_SIZES
                    .iter()
                    .map(|(n, a, b)| {
                        let (pw, ph) = if landscape { (*b, *a) } else { (*a, *b) };
                        ((pw as u32, ph as u32), n.to_string())
                    })
                    .collect();
                let cur = (sheet.width.round() as u32, sheet.height.round() as u32);
                if !opts.iter().any(|o| o.0 == cur) {
                    opts.push((cur, format!("{} × {}", cur.0, cur.1)));
                }
                let refs: Vec<((u32, u32), &str)> =
                    opts.iter().map(|(a, b)| (*a, b.as_str())).collect();
                let v = choice(ui, "sheet-size", w, cur, &refs);
                if v != cur {
                    new.width = v.0 as f64;
                    new.height = v.1 as f64;
                }
            });
            row(ui, "Orientación", "", |ui, w| {
                let landscape = new.width >= new.height;
                let v = choice(
                    ui,
                    "sheet-orientation",
                    w,
                    landscape,
                    &[(true, "Horizontal"), (false, "Vertical")],
                );
                if v != landscape {
                    std::mem::swap(&mut new.width, &mut new.height);
                }
            });
            row(ui, "Marco y cajetín", "", |ui, _| {
                ui.checkbox(&mut new.title_block, "");
            });
        });
        if new != sheet {
            let r = self.ed.set_sheet(new);
            self.report(r, "Plano cambiado.");
        }
        // Add a view.
        let levels: Vec<(LevelId, String)> = self
            .ed
            .model()
            .levels()
            .iter()
            .map(|l| (l.id, l.name.clone()))
            .collect();
        let mut add = None;
        group(ui, "sheet-add", "Añadir vista", |ui| {
            let mut level = self.sheet_new_level.unwrap_or(self.ed.level);
            if !levels.iter().any(|l| l.0 == level) {
                level = levels.first().map(|l| l.0).unwrap_or(level);
            }
            row(ui, "Planta", "", |ui, w| {
                let refs: Vec<(LevelId, &str)> =
                    levels.iter().map(|(a, b)| (*a, b.as_str())).collect();
                level = choice(ui, "sheet-add-level", w, level, &refs);
            });
            self.sheet_new_level = Some(level);
            let mut scale = self.sheet_new_scale as u32;
            row(ui, "Escala", "", |ui, w| {
                let options: Vec<(u32, String)> = [20.0, 50.0, 100.0, 200.0, 500.0]
                    .iter()
                    .map(|s| (*s as u32, format!("1:{s}")))
                    .collect();
                let refs: Vec<(u32, &str)> =
                    options.iter().map(|(v, n)| (*v, n.as_str())).collect();
                scale = choice(ui, "sheet-add-scale", w, scale, &refs);
            });
            self.sheet_new_scale = scale as f64;
            if ui
                .button("Colocar en el plano")
                .on_hover_text("Se coloca en el centro; luego arrástrala")
                .clicked()
            {
                add = Some((level, scale as f64));
            }
        });
        if let Some((level, scale)) = add {
            let centre = Point2::new(
                (sheet.width - archisim_plan::sheet::TITLE_W / 2.0) * MM / 2.0 + 0.01,
                sheet.height * MM / 2.0 + 0.02,
            );
            let r = self.ed.add_viewport(NewViewport {
                sheet: id,
                level,
                scale,
                cut_plane: self.cut_plane,
                crop: None,
                centre,
            });
            match r {
                Ok(v) => {
                    self.sheet_sel = Some(Placed::View(v));
                    self.say("Vista colocada: arrástrala a su sitio.");
                }
                Err(e) => self.complain(format!("No se puede: {e}.")),
            }
        }
        group(ui, "sheet-views", "Vistas en el plano", |ui| {
            let model = self.ed.model();
            let views: Vec<(ViewportId, String)> = model
                .viewports_on(id)
                .map(|v| {
                    (
                        v.id,
                        format!(
                            "{} · 1:{}",
                            archisim_plan::sheet::viewport_title(model, v),
                            v.scale
                        ),
                    )
                })
                .collect();
            if views.is_empty() {
                props::note(ui, "Ninguna todavía.");
            }
            for (v, label) in views {
                if ui
                    .selectable_label(self.sheet_sel == Some(Placed::View(v)), label)
                    .clicked()
                {
                    self.sheet_sel = Some(Placed::View(v));
                }
            }
        });
        // Schedules.
        let levels: Vec<(Option<LevelId>, String)> =
            std::iter::once((None, "Todo el edificio".to_string()))
                .chain(
                    self.ed
                        .model()
                        .levels()
                        .iter()
                        .map(|l| (Some(l.id), l.name.clone())),
                )
                .collect();
        let mut place = None;
        group(ui, "sheet-tables", "Tablas", |ui| {
            let mut kind = self.table_new_kind;
            row(ui, "Tabla", "", |ui, w| {
                let opts: Vec<(ScheduleKind, &str)> =
                    ScheduleKind::ALL.iter().map(|k| (*k, k.label())).collect();
                kind = choice(ui, "sheet-table-kind", w, kind, &opts);
            });
            self.table_new_kind = kind;
            let mut level = self.table_new_level;
            row(ui, "De", "", |ui, w| {
                let opts: Vec<(Option<LevelId>, &str)> =
                    levels.iter().map(|(a, b)| (*a, b.as_str())).collect();
                level = choice(ui, "sheet-table-level", w, level, &opts);
            });
            self.table_new_level = level;
            if ui.button("Colocar la tabla").clicked() {
                place = Some((kind, level));
            }
            let model = self.ed.model();
            let placed: Vec<(TableId, String)> = model
                .tables_on(id)
                .map(|t| {
                    let of = t
                        .level
                        .and_then(|l| model.level(l))
                        .map_or("todo el edificio".to_string(), |l| l.name.clone());
                    (t.id, format!("{} · {of}", t.kind.label()))
                })
                .collect();
            for (t, label) in placed {
                if ui
                    .selectable_label(self.sheet_sel == Some(Placed::Table(t)), label)
                    .clicked()
                {
                    self.sheet_sel = Some(Placed::Table(t));
                }
            }
        });
        if let Some((kind, level)) = place {
            // Top right, under the frame.
            let at = Point2::new(
                (sheet.width - archisim_plan::sheet::FRAME - 10.0) * MM - 0.15,
                (sheet.height - archisim_plan::sheet::FRAME - 10.0) * MM,
            );
            match self.ed.add_table(id, kind, level, at) {
                Ok(t) => {
                    self.sheet_sel = Some(Placed::Table(t));
                    self.say("Tabla colocada: arrástrala a su sitio.");
                }
                Err(e) => self.complain(format!("No se puede: {e}.")),
            }
        }
        self.project_info_group(ui);
        ui.add_space(4.0);
        props::note(
            ui,
            "Arrastra las vistas para colocarlas; doble clic abre su planta. Los textos y líneas que dibujes aquí (L, TX…) son del plano. Supr quita la vista seleccionada.",
        );
    }

    fn viewport_properties(&mut self, ui: &mut Ui, v: Viewport) {
        let model = self.ed.model();
        let title = archisim_plan::sheet::viewport_title(model, &v);
        props::title(ui, &format!("Vista · {title}"));
        let mut new = v.clone();
        let levels: Vec<(LevelId, String)> = model
            .levels()
            .iter()
            .map(|l| (l.id, l.name.clone()))
            .collect();
        group(ui, "vp-view", "Vista", |ui| {
            row(ui, "Planta", "", |ui, w| {
                let refs: Vec<(LevelId, &str)> =
                    levels.iter().map(|(a, b)| (*a, b.as_str())).collect();
                new.level = choice(ui, "vp-level", w, new.level, &refs);
            });
            row(ui, "Escala", "", |ui, w| {
                let mut opts: Vec<(u32, String)> = [20.0, 50.0, 100.0, 200.0, 500.0]
                    .iter()
                    .chain(SCALES.iter())
                    .map(|s| (*s as u32, format!("1:{s}")))
                    .collect();
                opts.sort();
                opts.dedup();
                let refs: Vec<(u32, &str)> = opts.iter().map(|(a, b)| (*a, b.as_str())).collect();
                new.scale = choice(ui, "vp-scale", w, new.scale as u32, &refs) as f64;
            });
            row(ui, "Plano de corte (mm)", "", |ui, w| {
                if let Some(c) =
                    length_field(ui, Id::unique(("vp-cut", v.id)), v.cut_plane, w - 8.0)
                {
                    if c > 0.0 {
                        new.cut_plane = c;
                    }
                }
            });
            row(ui, "Título", "Vacío: el nombre de la planta", |ui, w| {
                if let Some(t) =
                    text_edit_committed(ui, Id::unique(("vp-title", v.id)), &v.title, w - 8.0)
                {
                    new.title = t;
                }
            });
            row(ui, "Mostrar título", "", |ui, _| {
                ui.checkbox(&mut new.show_title, "");
            });
        });
        group(ui, "vp-crop", "Recorte", |ui| {
            let (lo, hi) = archisim_plan::sheet::viewport_crop(model, &v);
            text_row(
                ui,
                "Ancho × alto (mm)",
                "",
                &format!("{} × {}", fmt_mm(hi.x - lo.x), fmt_mm(hi.y - lo.y)),
            );
            let mut custom = v.crop.is_some();
            row(
                ui,
                "Recorte propio",
                "Sin recorte, la vista muestra toda la planta",
                |ui, _| {
                    if ui.checkbox(&mut custom, "").changed() {
                        new.crop = custom.then_some((lo, hi));
                    }
                },
            );
            if let Some((a, b)) = v.crop {
                let mut c = (a, b);
                for (name, which) in [
                    ("Izquierda", 0),
                    ("Derecha", 1),
                    ("Abajo", 2),
                    ("Arriba", 3),
                ] {
                    row(
                        ui,
                        &format!("{name} (mm)"),
                        "Coordenada del borde del recorte",
                        |ui, w| {
                            let val = match which {
                                0 => c.0.x,
                                1 => c.1.x,
                                2 => c.0.y,
                                _ => c.1.y,
                            };
                            if let Some(x) =
                                length_field(ui, Id::unique(("vp-crop", v.id, which)), val, w - 8.0)
                            {
                                match which {
                                    0 => c.0.x = x,
                                    1 => c.1.x = x,
                                    2 => c.0.y = x,
                                    _ => c.1.y = x,
                                }
                            }
                        },
                    );
                }
                if c != (a, b) {
                    new.crop = Some(c);
                }
            }
        });
        // Visibility of this view.
        let saved = std::mem::replace(&mut self.view_style, new.style.clone());
        self.visibility_group(ui);
        new.style = std::mem::replace(&mut self.view_style, saved);
        if new != v {
            let r = self.ed.set_viewport(new, "Cambiar vista del plano");
            self.report(r, "Vista cambiada.");
        }
        ui.add_space(4.0);
        if ui.button("Abrir la planta").clicked() {
            self.open_view(TabKind::Plan(v.level));
        }
        if ui.button("Quitar del plano").clicked() {
            let r = self.ed.remove_viewport(v.id);
            self.sheet_sel = None;
            self.report(r, "Vista quitada del plano.");
        }
    }

    fn table_properties(&mut self, ui: &mut Ui, t: SheetTable) {
        props::title(ui, &format!("Tabla · {}", t.kind.label()));
        let model = self.ed.model();
        let table = archisim_plan::schedules::table(model, t.kind, t.level);
        let levels: Vec<(Option<LevelId>, String)> =
            std::iter::once((None, "Todo el edificio".to_string()))
                .chain(model.levels().iter().map(|l| (Some(l.id), l.name.clone())))
                .collect();
        let mut new = t.clone();
        group(ui, "table", "Tabla", |ui| {
            row(ui, "Tabla", "", |ui, w| {
                let opts: Vec<(ScheduleKind, &str)> =
                    ScheduleKind::ALL.iter().map(|k| (*k, k.label())).collect();
                new.kind = choice(ui, "table-kind", w, t.kind, &opts);
            });
            row(ui, "De", "", |ui, w| {
                let opts: Vec<(Option<LevelId>, &str)> =
                    levels.iter().map(|(a, b)| (*a, b.as_str())).collect();
                new.level = choice(ui, "table-level", w, t.level, &opts);
            });
            text_row(ui, "Filas", "", &table.rows.len().to_string());
        });
        if new != t {
            let r = self.ed.set_table(new, "Cambiar tabla del plano");
            self.report(r, "Tabla cambiada.");
        }
        ui.add_space(4.0);
        if ui.button("Ver y exportar (CSV)…").clicked() {
            self.schedule_open = Some((t.kind, t.level));
        }
        if ui.button("Quitar del plano").clicked() {
            let r = self.ed.remove_table(t.id);
            self.sheet_sel = None;
            self.report(r, "Tabla quitada del plano.");
        }
        props::note(ui, "La tabla se rehace sola cuando cambia el modelo.");
    }

    /// The project's data for the title block.
    pub(super) fn project_info_group(&mut self, ui: &mut Ui) {
        let info = self.ed.model().info().clone();
        let mut new = info.clone();
        group(ui, "project-info", "Datos del cajetín", |ui| {
            type Field = fn(&mut archisim_core::ProjectInfo) -> &mut String;
            let fields: [(&str, Field); 7] = [
                ("Estudio", |i| &mut i.studio),
                ("Promotor", |i| &mut i.client),
                ("Emplazamiento", |i| &mut i.site),
                ("Autor", |i| &mut i.author),
                ("Nº de expediente", |i| &mut i.number),
                ("Fase", |i| &mut i.phase),
                ("Fecha", |i| &mut i.date),
            ];
            for (name, f) in fields {
                let current = f(&mut new.clone()).clone();
                row(ui, name, "", |ui, w| {
                    if let Some(t) =
                        text_edit_committed(ui, Id::unique(("info", name)), &current, w - 8.0)
                    {
                        *f(&mut new) = t;
                    }
                });
            }
        });
        if new != info {
            let r = self.ed.set_info(new);
            self.report(r, "Datos del proyecto cambiados.");
        }
    }

    /// Sheets in the navigator.
    pub(super) fn nav_sheet_rows(
        &mut self,
        ui: &mut Ui,
        active: Option<TabKind>,
    ) -> Option<TabKind> {
        let sheets: Vec<Sheet> = self.ed.model().sheets().into_iter().cloned().collect();
        let mut open = None;
        let mut remove = None;
        for s in &sheets {
            let kind = TabKind::Sheet(s.id);
            let r = super::widgets::list_row(
                ui,
                Some(super::icons::Icon::Sheet),
                None,
                &format!("{} {}", s.number, s.name),
                &s.size_name(),
                active == Some(kind),
                10.0,
            )
            .on_hover_text("Abrir el plano");
            if r.clicked() {
                open = Some(kind);
            }
            r.context_menu(|ui| {
                if ui.button("Borrar el plano").clicked() {
                    remove = Some(s.id);
                    ui.close();
                }
            });
        }
        if let Some(id) = remove {
            let r = self.ed.remove_sheet(id);
            self.report(r, "Plano borrado.");
        }
        open
    }
}
