//! Views only for drawing (E7) in the window: a white sheet of real size
//! at the view's scale where the 2D drafting and the AutoCAD commands work
//! as in a plan, its underlays drawn under it; its properties (name,
//! scale, underlays) and the underlays of a level's plan.

use archisim_core::{DraftViewId, LevelId, Underlay, UnderlaySource, ViewRef};
use archisim_plan::drafting_views::{drafting_region, render_drafting_view};
use eframe::egui::{self, Ui};

use super::props::{self, group, row};
use super::widgets::SwitchExt as _;
use super::*;

/// What the picture of a view for drawing was drawn for.
#[derive(Debug, Clone, PartialEq)]
pub struct DraftingKey {
    revision: u64,
    view: DraftViewId,
    lo: Point2,
    hi: Point2,
    size: [usize; 2],
    thin: bool,
}

impl ArchiSimApp {
    /// The tab of a view for drawing.
    pub(super) fn drafting_view(&mut self, ui: &mut Ui, id: DraftViewId) {
        let (rect, response) = ui.allocate_exact_size(ui.available_size(), Sense::click_and_drag());
        let ctx = ui.ctx().clone();
        let painter = ui.painter_at(rect);
        self.canvas = rect;
        let Some(dv) = self.ed.model().draft_view(id).cloned() else {
            return;
        };
        if self.fit_requested && rect.width() > 50.0 {
            self.fit_requested = false;
            let (lo, hi) = drafting_region(
                self.ed.model(),
                id,
                &archisim_plan::PlanOptions {
                    scale: dv.scale,
                    ..Default::default()
                },
            );
            self.view.fit_exact(rect, lo, hi, 30.0);
        }
        let before = self.view;
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
        if self.view != before {
            self.view_moved_at = Some(std::time::Instant::now());
        }
        // The cursor, keys and clicks.
        let view = ViewRef::Drafting(id);
        match response.hover_pos() {
            Some(pos) => {
                let raw = self.view.to_model(rect, pos);
                let reach = self.view.to_metres(REACH);
                if self.tool == Tool::Cad {
                    self.cad_cursor(&ctx, raw, reach);
                } else {
                    self.cursor = Some(Snap::at(raw, SnapKind::Free));
                    self.hover = crate::pick::pick_drafts(self.ed.model(), view, raw, reach);
                }
            }
            None => {
                self.cursor = None;
                self.hover = None;
            }
        }
        if ctx.memory(|m| m.focused().is_none()) {
            self.drafting_keys(&ctx);
        }
        self.drafting_clicks(&ctx, &response, rect, id);

        painter.rect_filled(rect, CornerRadius::ZERO, Color32::WHITE);
        self.draw_drafting_image(&ctx, &painter, rect, id, dv.scale);
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
        if let (true, Some(c)) = (self.tool == Tool::Cad, self.cursor) {
            let p = self.view.to_screen(rect, c.point);
            let s = Stroke::new(1.0, ACCENT);
            painter.line_segment([p - Vec2::new(10.0, 0.0), p + Vec2::new(10.0, 0.0)], s);
            painter.line_segment([p - Vec2::new(0.0, 10.0), p + Vec2::new(0.0, 10.0)], s);
        }
        self.draw_overlays(&ctx, &painter, rect);
        if response.hovered() {
            ctx.set_cursor_icon(if self.tool == Tool::Cad {
                CursorIcon::Crosshair
            } else {
                CursorIcon::Default
            });
        }
    }

    fn drafting_keys(&mut self, ctx: &egui::Context) {
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
            self.ed.selection.clear();
            self.window_start = None;
        }
        if (delete || backspace) && !self.ed.selection.is_empty() {
            self.delete();
        }
    }

    fn drafting_clicks(
        &mut self,
        ctx: &egui::Context,
        response: &egui::Response,
        rect: Rect,
        id: DraftViewId,
    ) {
        if self.tool == Tool::Cad {
            self.cad_clicks(ctx, response, rect);
            return;
        }
        let shift = ctx.input(|i| i.modifiers.shift || i.modifiers.command);
        if response.clicked_by(PointerButton::Primary) {
            match self.hover {
                Some(e) => {
                    if shift {
                        if !self.ed.selection.remove(&e) {
                            self.ed.selection.insert(e);
                        }
                    } else {
                        self.ed.selection.clear();
                        self.ed.selection.insert(e);
                    }
                }
                None if !shift => self.ed.selection.clear(),
                None => {}
            }
        }
        if response.drag_started_by(PointerButton::Primary) {
            let origin = ctx
                .input(|i| i.pointer.press_origin())
                .map(|p| self.view.to_model(rect, p));
            if let Some(o) = origin {
                let on = crate::pick::pick_drafts(
                    self.ed.model(),
                    ViewRef::Drafting(id),
                    o,
                    self.view.to_metres(REACH),
                );
                match on {
                    Some(e) if self.ed.selection.contains(&e) => {
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
                if let Err(e) = self.ed.apply_cad(ViewRef::Drafting(id), &st, &step) {
                    self.complain(format!("No se puede: {e}."));
                }
            }
            if let (Some(a), Some(c)) = (self.window_start.take(), self.cursor) {
                let found = crate::pick::in_window_drafts(
                    self.ed.model(),
                    ViewRef::Drafting(id),
                    a,
                    c.point,
                );
                if !shift {
                    self.ed.selection.clear();
                }
                self.ed.selection.extend(found);
            }
        }
    }

    /// The drawing of the view for the part on screen, made again when the
    /// model, the view or the window change.
    fn draw_drafting_image(
        &mut self,
        ctx: &egui::Context,
        painter: &egui::Painter,
        rect: Rect,
        id: DraftViewId,
        scale: f64,
    ) {
        let ppp = ctx.pixels_per_point();
        let size = [
            (rect.width() * ppp).round().max(1.0) as usize,
            (rect.height() * ppp).round().max(1.0) as usize,
        ];
        let (lo, hi) = self.view.visible(rect);
        let key = DraftingKey {
            revision: self.ed.revision(),
            view: id,
            lo,
            hi,
            size,
            thin: self.thin_lines,
        };
        if self.drafting_image.as_ref().map(|p| &p.0) != Some(&key) {
            let px_per_m = size[0] as f64 / (hi.x - lo.x).max(1e-9);
            let svg = render_drafting_view(
                &self.ed.shown(),
                id,
                &archisim_plan::PlanOptions {
                    scale,
                    viewport: Some((lo, hi)),
                    px_per_mm: Some(px_per_m * scale / 1000.0),
                    title: false,
                    thin_lines: self.thin_lines,
                    ..Default::default()
                },
            );
            let image = crate::plan_image::rasterize_svg(&svg, size);
            match &mut self.drafting_image {
                Some((k, tex)) => {
                    tex.set(image, TextureOptions::LINEAR);
                    *k = key;
                }
                None => {
                    let tex = ctx.load_texture("drafting", image, TextureOptions::LINEAR);
                    self.drafting_image = Some((key, tex));
                }
            }
        }
        if let Some((_, tex)) = &self.drafting_image {
            painter.image(
                tex.id(),
                rect,
                Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)),
                Color32::WHITE,
            );
        }
    }

    /// A new view for drawing, opened in its tab.
    pub(super) fn new_drafting_view(&mut self) {
        let n = self.ed.model().draft_views().len() + 1;
        let name = format!("Detalle {n}");
        let r = self
            .ed
            .transact("Nueva vista de dibujo", |m| m.add_draft_view(name, 20.0));
        match r {
            Ok(id) => {
                self.open_view(super::tabs::TabKind::Drafting(id));
            }
            Err(e) => self.complain(format!("No se puede: {e}.")),
        }
    }

    /// The properties of a view for drawing: name, scale and underlays.
    pub(super) fn drafting_properties(&mut self, ui: &mut Ui, id: DraftViewId) {
        let Some(dv) = self.ed.model().draft_view(id).cloned() else {
            return;
        };
        props::title(ui, "Vista de dibujo");
        let mut changed = dv.clone();
        group(ui, "drafting-view", "Vista de dibujo", |ui| {
            row(ui, "Nombre", "", |ui, w| {
                if let Some(t) =
                    super::text_edit_committed(ui, Id::unique(("dv-name", id.0)), &dv.name, w - 8.0)
                {
                    if !t.trim().is_empty() {
                        changed.name = t.trim().to_string();
                    }
                }
            });
            row(
                ui,
                "Escala 1:",
                "A la que se dibuja y se coloca en los planos",
                |ui, w| {
                    let mut s = dv.scale;
                    let r = ui.add_sized(
                        [w - 8.0, 20.0],
                        egui::DragValue::new(&mut s).range(1.0..=1000.0).speed(1.0),
                    );
                    if r.changed() && s >= 1.0 {
                        changed.scale = s.round();
                    }
                },
            );
        });
        let mut underlays = dv.underlays.clone();
        if self.underlays_group(
            ui,
            ("dv", id.0),
            &mut underlays,
            UnderlaySource::Drafting(id),
        ) {
            changed.underlays = underlays;
        }
        if changed != dv {
            let r = self
                .ed
                .transact("Vista de dibujo", |m| m.set_drafting_view(changed));
            self.report(r, "Vista de dibujo cambiada.");
        }
        ui.horizontal(|ui| {
            ui.add_space(4.0);
            if ui.button("Borrar la vista de dibujo").clicked() {
                let r = self.ed.transact("Borrar vista de dibujo", |m| {
                    m.remove_draft_view(id).map(|_| ())
                });
                self.report(r, "Vista de dibujo borrada.");
            }
        });
    }

    /// The underlays of a level's plan, in the plan's properties.
    pub(super) fn level_underlays_group(&mut self, ui: &mut Ui, level: LevelId) {
        let mut list = self.ed.model().level_underlays(level).to_vec();
        if self.underlays_group(ui, ("lv", level.0), &mut list, UnderlaySource::Level(level)) {
            let r = self.ed.transact("Calcos de la planta", |m| {
                m.set_level_underlays(level, list)
            });
            self.report(r, "Calcos cambiados.");
        }
    }

    /// A list of underlays to edit: what each shows, its colour, how
    /// see-through, moved and turned; one more. `own`: the view they are
    /// under (not offered). Whether something changed.
    fn underlays_group(
        &mut self,
        ui: &mut Ui,
        salt: (&str, u64),
        list: &mut Vec<Underlay>,
        own: UnderlaySource,
    ) -> bool {
        let sources: Vec<(UnderlaySource, String)> = {
            let m = self.ed.model();
            let mut v: Vec<(UnderlaySource, String)> = m
                .levels()
                .iter()
                .map(|l| (UnderlaySource::Level(l.id), format!("Planta · {}", l.name)))
                .collect();
            v.extend(m.sections().map(|s| {
                (
                    UnderlaySource::Section(s.id),
                    format!("Sección · {}", s.title()),
                )
            }));
            v.extend(m.draft_views().iter().map(|d| {
                (
                    UnderlaySource::Drafting(d.id),
                    format!("Dibujo · {}", d.name),
                )
            }));
            v.retain(|(s, _)| *s != own);
            v
        };
        let name_of = |s: UnderlaySource| {
            sources
                .iter()
                .find(|(x, _)| *x == s)
                .map_or_else(|| "—".to_string(), |(_, n)| n.clone())
        };
        let mut changed = false;
        group(
            ui,
            &format!("underlays-{}-{}", salt.0, salt.1),
            "Calcos",
            |ui| {
                if list.is_empty() {
                    props::note(
                    ui,
                    "Una planta, sección o vista de dibujo debajo, como referencia para dibujar encima: de un color, transparente, movida y girada.",
                );
                }
                let mut remove = None;
                for (i, u) in list.iter_mut().enumerate() {
                    ui.label(egui::RichText::new(name_of(u.source)).strong());
                    row(
                        ui,
                        "Color",
                        "Todo de este color (apagado: sus colores)",
                        |ui, _| {
                            let mut on = u.color.is_some();
                            let mut rgb = u.color.unwrap_or([120, 140, 170]);
                            let a = ui.switch(&mut on, "").changed();
                            let b = on && ui.color_edit_button_srgb(&mut rgb).changed();
                            if a || b {
                                u.color = on.then_some(rgb);
                                changed = true;
                            }
                        },
                    );
                    row(ui, "Opacidad (%)", "", |ui, w| {
                        let mut o = u.opacity * 100.0;
                        if ui
                            .add_sized(
                                [w - 8.0, 20.0],
                                egui::DragValue::new(&mut o).range(0.0..=100.0),
                            )
                            .changed()
                        {
                            u.opacity = (o / 100.0).clamp(0.0, 1.0);
                            changed = true;
                        }
                    });
                    for (label, axis) in [("Mover X (m)", 0), ("Mover Y (m)", 1)] {
                        row(ui, label, "", |ui, w| {
                            let mut v = if axis == 0 { u.offset.x } else { u.offset.y };
                            if ui
                                .add_sized(
                                    [w - 8.0, 20.0],
                                    egui::DragValue::new(&mut v).speed(0.05).max_decimals(3),
                                )
                                .changed()
                            {
                                if axis == 0 {
                                    u.offset.x = v;
                                } else {
                                    u.offset.y = v;
                                }
                                changed = true;
                            }
                        });
                    }
                    row(
                        ui,
                        "Girar (°)",
                        "Sobre su origen, antes de moverlo",
                        |ui, w| {
                            let mut deg = u.rotation.to_degrees();
                            if ui
                                .add_sized(
                                    [w - 8.0, 20.0],
                                    egui::DragValue::new(&mut deg).speed(0.5),
                                )
                                .changed()
                            {
                                u.rotation = deg.to_radians();
                                changed = true;
                            }
                        },
                    );
                    if ui.small_button("Quitar este calco").clicked() {
                        remove = Some(i);
                    }
                    ui.separator();
                }
                if let Some(i) = remove {
                    list.remove(i);
                    changed = true;
                }
                ui.menu_button("Añadir calco…", |ui| {
                    for (s, n) in &sources {
                        if ui.button(n).clicked() {
                            list.push(Underlay::of(*s));
                            changed = true;
                            ui.close();
                        }
                    }
                });
            },
        );
        changed
    }
}
