//! The plan view: navigation, snapping, clicks of each tool and what is
//! drawn over the plan picture (selection, handles, previews, cursor).

use super::*;

impl ArchiSimApp {
    pub(super) fn plan_view(&mut self, ui: &mut Ui) {
        let (rect, response) = ui.allocate_exact_size(ui.available_size(), Sense::click_and_drag());
        let ctx = ui.ctx().clone();
        let painter = ui.painter_at(rect);
        self.canvas = rect;

        if self.fit_requested && rect.width() > 50.0 {
            self.fit_requested = false;
            match self.ed.level_bounds() {
                Some((lo, hi)) => self.view.fit(rect, lo, hi, 40.0),
                None => self
                    .view
                    .fit(rect, Point2::new(-2.0, -2.0), Point2::new(18.0, 12.0), 20.0),
            }
        }

        self.navigate(&ctx, &response, rect);
        self.track_cursor(&ctx, &response, rect);
        if ctx.memory(|m| m.focused().is_none()) {
            self.keyboard_in_view(&ctx);
        }
        self.clicks(&ctx, &response, rect);

        // Paper, grid, the plan itself, then what is being done on top.
        painter.rect_filled(rect, CornerRadius::ZERO, PAPER);
        if self.show_grid {
            self.draw_grid(&painter, rect);
        }
        self.draw_plan(&ctx, &painter, rect);
        self.draw_origin(&painter, rect);
        self.draw_selection(&painter, rect);
        self.draw_hotspots(&painter, rect);
        self.draw_wall_preview(&painter, rect);
        self.draw_slab_preview(&painter, rect);
        self.draw_opening_preview(&painter, rect);
        self.draw_wand_preview(&painter, rect);
        self.draw_dim_preview(&painter, rect);
        self.draw_drag_preview(&painter, rect);
        self.draw_cursor(&painter, rect);
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

        self.draw_overlays(&ctx, &painter, rect);

        let icon = match self.tool {
            Tool::Wall | Tool::Slab | Tool::Door | Tool::Window | Tool::Dimension => {
                CursorIcon::Crosshair
            }
            Tool::Arrow if self.drag.is_some() => CursorIcon::Grabbing,
            Tool::Arrow if matches!(self.hover_grip, Some(Grip::Body)) => CursorIcon::Move,
            Tool::Arrow if self.hover_grip.is_some() => CursorIcon::Grab,
            Tool::Arrow if self.hover.is_some() => CursorIcon::PointingHand,
            Tool::Arrow => CursorIcon::Default,
        };
        if response.hovered() {
            ctx.set_cursor_icon(icon);
        }
    }

    fn navigate(&mut self, ctx: &egui::Context, response: &egui::Response, rect: Rect) {
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
    }

    /// The snapped point near `raw`, with the snaps of the bottom bar.
    fn snap_at(&self, raw: Point2, reach: f64, extra: &[Point2], skip: &BTreeSet<Element>) -> Snap {
        snap_filtered(
            self.ed.model(),
            self.ed.level,
            raw,
            reach,
            extra,
            skip,
            self.snap_filter,
        )
    }

    /// Polar tracking from `from`, if it is on.
    fn track_at(&self, from: Point2, s: Snap) -> Snap {
        if self.snap_filter.tracking {
            track(from, s, self.view.to_metres(TRACK_REACH))
        } else {
            s
        }
    }

    fn track_cursor(&mut self, ctx: &egui::Context, response: &egui::Response, rect: Rect) {
        let Some(pos) = response.hover_pos() else {
            self.cursor = None;
            self.hover = None;
            return;
        };
        self.space_held = ctx.input(|i| i.key_down(Key::Space));
        let raw = self.view.to_model(rect, pos);
        let reach = self.view.to_metres(REACH);
        let model = self.ed.model();
        match self.tool {
            Tool::Wall => {
                let mut s = self.snap_at(raw, reach, self.wall_tool.chain(), &BTreeSet::new());
                if let Some(a) = self.wall_tool.anchor() {
                    if ctx.input(|i| i.modifiers.shift) {
                        s = constrain(a, s);
                    } else {
                        s = self.track_at(a, s);
                    }
                }
                self.cursor = Some(s);
                self.hover = None;
            }
            Tool::Slab if self.wand_active(ctx) => {
                let p = crate::snap::round_point(raw);
                self.cursor = Some(Snap {
                    point: p,
                    kind: SnapKind::Free,
                });
                self.hover = None;
                let contour = self.slab_settings.contour;
                let key = (self.ed.revision(), p, contour);
                let fresh = self
                    .wand_preview
                    .as_ref()
                    .is_some_and(|w| (w.0, w.1, w.2) == key);
                if !fresh {
                    let found = room_outline(model, self.ed.level, p, contour)
                        .map_err(|e| explain_room(&e));
                    self.wand_preview = Some((key.0, key.1, key.2, found));
                }
            }
            Tool::Slab => {
                let mut s = self.snap_at(raw, reach, self.slab_tool.corners(), &BTreeSet::new());
                if let Some(a) = self.slab_tool.anchor() {
                    if ctx.input(|i| i.modifiers.shift) {
                        s = constrain(a, s);
                    } else {
                        s = self.track_at(a, s);
                    }
                }
                self.cursor = Some(s);
                self.hover = None;
            }
            Tool::Dimension => {
                let mut s = self.snap_at(raw, reach, self.dim_tool.points(), &BTreeSet::new());
                if let (false, Some(a)) =
                    (self.dim_tool.is_placing(), self.dim_tool.points().last())
                {
                    if ctx.input(|i| i.modifiers.shift) {
                        s = constrain(*a, s);
                    } else {
                        s = self.track_at(*a, s);
                    }
                }
                self.cursor = Some(s);
                self.hover = None;
            }
            Tool::Door | Tool::Window => {
                let settings = if self.tool == Tool::Door {
                    self.door_settings
                } else {
                    self.window_settings
                };
                self.placement = placement(model, self.ed.level, raw, reach, &settings);
                self.cursor = Some(Snap {
                    point: crate::snap::round_point(raw),
                    kind: SnapKind::Free,
                });
                self.hover = None;
            }
            Tool::Arrow if self.drag.is_some() => {
                let Some(d) = &self.drag else {
                    return;
                };
                let mut s = self.snap_at(raw, reach, &[], &d.skip);
                if ctx.input(|i| i.modifiers.shift) {
                    s = constrain(d.from, s);
                } else {
                    s = self.track_at(d.from, s);
                }
                self.cursor = Some(s);
                self.hover = None;
                self.hover_grip = None;
                let variant = ctx.input(|i| i.modifiers.command);
                let to = d.target(s.point);
                let fresh = self
                    .drag_preview
                    .as_ref()
                    .is_some_and(|p| p.0 == to && p.1 == variant);
                if !fresh {
                    let drag = Drag {
                        grip: d.grip,
                        from: d.from,
                        to,
                        variant,
                    };
                    let ghost = crate::drag::preview(model, &self.ed.selection, &drag)
                        .map_err(|e| crate::units::explain(&e));
                    self.drag_preview = Some((to, variant, ghost));
                }
            }
            Tool::Arrow => {
                self.cursor = Some(Snap {
                    point: crate::snap::round_point(raw),
                    kind: SnapKind::Free,
                });
                self.hover_grip = if self.window_start.is_some() {
                    None
                } else {
                    grip_at(model, &self.ed.selection, raw, reach).map(|g| g.0)
                };
                self.hover = if self.window_start.is_some() {
                    None
                } else {
                    pick(model, self.ed.level, raw, reach)
                };
            }
        }
    }

    fn keyboard_in_view(&mut self, ctx: &egui::Context) {
        let (events, enter, escape, backspace, delete) = ctx.input(|i| {
            (
                i.events.clone(),
                i.key_pressed(Key::Enter),
                i.key_pressed(Key::Escape),
                i.key_pressed(Key::Backspace),
                i.key_pressed(Key::Delete),
            )
        });
        if self.drag.is_some() {
            self.keyboard_while_dragging(ctx, &events, enter, escape, backspace);
            return;
        }
        let drawing =
            self.wall_tool.is_drawing() || self.slab_tool.is_drawing() || self.dim_tool.is_active();
        let mut used_text = false;
        for e in &events {
            if let egui::Event::Text(t) = e {
                for c in t.chars() {
                    used_text |= match self.tool {
                        Tool::Wall => self.wall_tool.type_char(c),
                        Tool::Slab => self.slab_tool.type_char(c),
                        _ => false,
                    };
                }
            }
        }
        if !used_text && !drawing {
            // Tool keys: M muro, F forjado, P puerta, V ventana.
            for e in &events {
                if let egui::Event::Text(t) = e {
                    self.tool_key(t);
                }
            }
        }
        let edited_length = backspace
            && match self.tool {
                Tool::Wall => self.wall_tool.backspace(),
                Tool::Slab => self.slab_tool.backspace(),
                _ => false,
            };
        if !edited_length && (delete || backspace) && !drawing {
            self.delete();
        }
        if enter {
            let towards = self.cursor.map(|c| c.point).unwrap_or_default();
            match self.tool {
                Tool::Wall => {
                    if let Outcome::Build(a, b) = self.wall_tool.enter(towards) {
                        self.build_wall(a, b);
                    }
                }
                Tool::Slab => match self.slab_tool.enter(towards) {
                    SlabOutcome::Build(outline) => self.build_slab(outline),
                    SlabOutcome::Invalid => self.complain(
                        "El contorno necesita al menos tres esquinas y no puede cruzarse.",
                    ),
                    SlabOutcome::Nothing => {}
                },
                Tool::Dimension if !self.dim_tool.enough() => {
                    self.complain("Marca al menos dos puntos para acotar.");
                }
                _ => {}
            }
        }
        if escape {
            match self.tool {
                Tool::Wall => {
                    if self.wall_tool.escape() == Escape::Idle {
                        self.set_tool(Tool::Arrow);
                    }
                }
                Tool::Slab => {
                    if !self.slab_tool.escape() {
                        self.set_tool(Tool::Arrow);
                    }
                }
                Tool::Door | Tool::Window => self.set_tool(Tool::Arrow),
                Tool::Dimension => {
                    if !self.dim_tool.escape() {
                        self.set_tool(Tool::Arrow);
                    }
                }
                Tool::Arrow => {
                    self.window_start = None;
                    self.ed.selection.clear();
                }
            }
        }
    }

    /// While dragging: a typed distance and Enter drop the selection that
    /// far towards the cursor; Esc cancels.
    fn keyboard_while_dragging(
        &mut self,
        ctx: &egui::Context,
        events: &[egui::Event],
        enter: bool,
        escape: bool,
        backspace: bool,
    ) {
        if escape {
            self.drag = None;
            self.drag_preview = None;
            self.say("Cancelado.");
            return;
        }
        if let Some(d) = &mut self.drag {
            for e in events {
                if let egui::Event::Text(t) = e {
                    for c in t.chars() {
                        if c.is_ascii_digit() || matches!(c, ',' | '.' | 'm' | 'c' | 'M' | 'C') {
                            d.typed.push(c);
                        }
                    }
                }
            }
            if backspace {
                d.typed.pop();
            }
        }
        if enter {
            if let Some(c) = self.cursor {
                let variant = ctx.input(|i| i.modifiers.command);
                self.finish_drag(c.point, variant);
            }
        }
    }

    /// Whether a click with the slab tool uses the magic wand: the wand is
    /// chosen, or Space is held, and no outline is being drawn.
    fn wand_active(&self, ctx: &egui::Context) -> bool {
        !self.slab_tool.is_drawing()
            && (self.slab_settings.wand || ctx.input(|i| i.key_down(Key::Space)))
    }

    fn clicks(&mut self, ctx: &egui::Context, response: &egui::Response, rect: Rect) {
        let shift = ctx.input(|i| i.modifiers.shift || i.modifiers.command);
        match self.tool {
            Tool::Wall => {
                if response.clicked_by(PointerButton::Primary) {
                    if let Some(c) = self.cursor {
                        if let Outcome::Build(a, b) = self.wall_tool.click(c.point) {
                            self.build_wall(a, b);
                        }
                    }
                }
                if response.double_clicked_by(PointerButton::Primary)
                    || response.clicked_by(PointerButton::Secondary)
                {
                    self.wall_tool.finish();
                }
            }
            Tool::Slab if self.wand_active(ctx) => {
                if response.clicked_by(PointerButton::Primary) {
                    if let Some(c) = self.cursor {
                        self.build_slab_in_room(c.point);
                    }
                }
            }
            Tool::Slab => {
                if response.clicked_by(PointerButton::Primary) {
                    if let Some(c) = self.cursor {
                        match self.slab_tool.click(c.point) {
                            SlabOutcome::Build(outline) => self.build_slab(outline),
                            SlabOutcome::Invalid => self.complain(
                                "El contorno no puede cruzarse: corrige la última esquina.",
                            ),
                            SlabOutcome::Nothing => {}
                        }
                    }
                }
                if response.clicked_by(PointerButton::Secondary) {
                    self.slab_tool.finish();
                }
            }
            Tool::Door | Tool::Window => {
                if response.clicked_by(PointerButton::Primary) {
                    self.place_opening();
                }
            }
            Tool::Dimension => {
                if response.double_clicked_by(PointerButton::Primary)
                    || response.clicked_by(PointerButton::Secondary)
                {
                    self.dim_tool.enough();
                } else if response.clicked_by(PointerButton::Primary) {
                    if let Some(c) = self.cursor {
                        let reach = self.view.to_metres(REACH);
                        let out = self.dim_tool.click(
                            self.ed.model(),
                            self.ed.level,
                            self.dim_settings,
                            c.point,
                            reach,
                        );
                        self.dim_outcome(out);
                    }
                }
            }
            Tool::Arrow => {
                if response.clicked_by(PointerButton::Primary) {
                    let hit = self.hover;
                    match (hit, shift) {
                        (Some(e), true) => {
                            if !self.ed.selection.remove(&e) {
                                self.ed.selection.insert(e);
                            }
                        }
                        (Some(e), false) => {
                            self.ed.selection.clear();
                            self.ed.selection.insert(e);
                        }
                        (None, false) => self.ed.selection.clear(),
                        (None, true) => {}
                    }
                }
                if response.drag_started_by(PointerButton::Primary) {
                    let origin = ctx
                        .input(|i| i.pointer.press_origin())
                        .map(|p| self.view.to_model(rect, p));
                    let reach = self.view.to_metres(REACH);
                    let grip =
                        origin.and_then(|o| grip_at(self.ed.model(), &self.ed.selection, o, reach));
                    match grip {
                        Some((grip, from)) => {
                            // The selection is grabbed by the nearest
                            // special point, so it can be dropped exactly on
                            // another one.
                            let from = if grip == Grip::Body {
                                snap(self.ed.model(), self.ed.level, from, reach, &[]).point
                            } else {
                                from
                            };
                            let skip =
                                crate::drag::moving(self.ed.model(), &self.ed.selection, grip);
                            self.drag = Some(DragState {
                                grip,
                                from,
                                typed: String::new(),
                                skip,
                            });
                            self.drag_preview = None;
                        }
                        None => self.window_start = origin,
                    }
                }
                if response.drag_stopped() && self.drag.is_some() {
                    match self.cursor {
                        Some(c) => {
                            let variant = ctx.input(|i| i.modifiers.command);
                            self.finish_drag(c.point, variant);
                        }
                        None => self.say("Cancelado: se ha soltado fuera de la planta."),
                    }
                    self.drag = None;
                }
                if response.drag_stopped() {
                    if let (Some(a), Some(c)) = (self.window_start.take(), self.cursor) {
                        let found = in_window(self.ed.model(), self.ed.level, a, c.point);
                        if !shift {
                            self.ed.selection.clear();
                        }
                        self.ed.selection.extend(found);
                    }
                }
            }
        }
    }

    fn draw_grid(&self, painter: &egui::Painter, rect: Rect) {
        let (lo, hi) = self.view.visible(rect);
        for (step, color) in [(1.0, GRID_MINOR), (5.0, GRID_MAJOR)] {
            if step * self.view.zoom < 8.0 {
                continue;
            }
            let mut x = (lo.x / step).ceil() * step;
            while x <= hi.x {
                let sx = self.view.to_screen(rect, Point2::new(x, 0.0)).x;
                painter.line_segment(
                    [Pos2::new(sx, rect.top()), Pos2::new(sx, rect.bottom())],
                    Stroke::new(1.0, color),
                );
                x += step;
            }
            let mut y = (lo.y / step).ceil() * step;
            while y <= hi.y {
                let sy = self.view.to_screen(rect, Point2::new(0.0, y)).y;
                painter.line_segment(
                    [Pos2::new(rect.left(), sy), Pos2::new(rect.right(), sy)],
                    Stroke::new(1.0, color),
                );
                y += step;
            }
        }
    }

    fn draw_plan(&mut self, ctx: &egui::Context, painter: &egui::Painter, rect: Rect) {
        let ppp = ctx.pixels_per_point();
        let size = [
            (rect.width() * ppp).round().max(1.0) as usize,
            (rect.height() * ppp).round().max(1.0) as usize,
        ];
        let (lo, hi) = self.view.visible(rect);
        let key = PlanKey {
            revision: self.ed.revision(),
            level: self.ed.level,
            lo,
            hi,
            size,
            show_reference_lines: self.show_reference_lines,
            scale: self.plan_scale,
            cut_plane: self.cut_plane,
        };
        if self.plan.as_ref().map(|p| &p.0) != Some(&key) {
            let started = std::time::Instant::now();
            let image = rasterize_at(
                self.ed.model(),
                self.ed.level,
                lo,
                hi,
                size,
                self.show_reference_lines,
                self.plan_scale,
                self.cut_plane,
            );
            self.last_plan_ms = started.elapsed().as_secs_f64() * 1000.0;
            match &mut self.plan {
                Some((k, tex)) => {
                    tex.set(image, TextureOptions::LINEAR);
                    *k = key;
                }
                None => {
                    let tex = ctx.load_texture("plan", image, TextureOptions::LINEAR);
                    self.plan = Some((key, tex));
                }
            }
        }
        if let Some((_, tex)) = &self.plan {
            painter.image(
                tex.id(),
                rect,
                Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)),
                Color32::WHITE,
            );
        }
    }

    fn draw_origin(&self, painter: &egui::Painter, rect: Rect) {
        let o = self.view.to_screen(rect, Point2::new(0.0, 0.0));
        let s = Stroke::new(1.0, Color32::from_rgb(120, 120, 130));
        painter.line_segment([o - Vec2::new(8.0, 0.0), o + Vec2::new(8.0, 0.0)], s);
        painter.line_segment([o - Vec2::new(0.0, 8.0), o + Vec2::new(0.0, 8.0)], s);
        painter.circle_stroke(o, 4.0, s);
    }

    fn polygon(&self, rect: Rect, poly: &[Point2]) -> Vec<Pos2> {
        poly.iter().map(|p| self.view.to_screen(rect, *p)).collect()
    }

    fn draw_selection(&self, painter: &egui::Painter, rect: Rect) {
        let model = self.ed.model();
        if let Some(h) = self.hover {
            if !self.ed.selection.contains(&h) {
                if let Some(poly) = outline(model, h) {
                    painter.add(Shape::closed_line(
                        self.polygon(rect, &poly),
                        Stroke::new(2.0, ACCENT.gamma_multiply(0.5)),
                    ));
                }
            }
        }
        for e in &self.ed.selection {
            let Some(poly) = outline(model, *e) else {
                continue;
            };
            let pts = self.polygon(rect, &poly);
            painter.add(Shape::closed_line(pts.clone(), Stroke::new(2.0, ACCENT)));
            // A door or window is dragged by its body; mark its middle.
            let spots: Vec<Point2> = match e {
                Element::Opening(id) => opening_geometry(model, *id)
                    .map(|g| vec![g.point((g.t0 + g.t1) / 2.0, g.exterior_offset)])
                    .unwrap_or_default(),
                _ => Vec::new(),
            };
            for p in spots {
                let c = self.view.to_screen(rect, p);
                painter.rect_filled(
                    Rect::from_center_size(c, Vec2::splat(7.0)),
                    CornerRadius::ZERO,
                    Color32::BLACK,
                );
                painter.rect_stroke(
                    Rect::from_center_size(c, Vec2::splat(7.0)),
                    CornerRadius::ZERO,
                    Stroke::new(1.0, Color32::WHITE),
                    StrokeKind::Inside,
                );
            }
            if let Element::Wall(id) = e {
                if let Some(w) = model.wall(*id) {
                    dashed(
                        painter,
                        self.view.to_screen(rect, w.start),
                        self.view.to_screen(rect, w.end),
                        REF_LINE,
                    );
                }
            }
        }
    }

    /// Handles of the selection: wall ends and slab corners (black
    /// squares, to stretch) and slab edges (white diamonds, to move the
    /// edge); the one under the cursor is highlighted.
    fn draw_hotspots(&self, painter: &egui::Painter, rect: Rect) {
        for h in hotspots(self.ed.model(), &self.ed.selection) {
            let c = self.view.to_screen(rect, h.point);
            let hot = self.hover_grip == Some(h.grip)
                || self.drag.as_ref().is_some_and(|d| d.grip == h.grip);
            let fill = if hot { ACCENT } else { Color32::BLACK };
            match h.grip {
                Grip::SlabEdge { .. } => {
                    let r = if hot { 6.0 } else { 4.5 };
                    let pts = vec![
                        c + Vec2::new(0.0, -r),
                        c + Vec2::new(r, 0.0),
                        c + Vec2::new(0.0, r),
                        c + Vec2::new(-r, 0.0),
                    ];
                    let inside = if hot { ACCENT } else { Color32::WHITE };
                    painter.add(Shape::convex_polygon(
                        pts,
                        inside,
                        Stroke::new(1.0, Color32::BLACK),
                    ));
                }
                _ => {
                    let size = Vec2::splat(if hot { 9.0 } else { 7.0 });
                    painter.rect_filled(Rect::from_center_size(c, size), CornerRadius::ZERO, fill);
                    painter.rect_stroke(
                        Rect::from_center_size(c, size),
                        CornerRadius::ZERO,
                        Stroke::new(1.0, Color32::WHITE),
                        StrokeKind::Inside,
                    );
                }
            }
        }
    }

    /// The selection being dragged: the outlines it would have, the line
    /// from where it was grabbed to the cursor, and the distance.
    fn draw_drag_preview(&self, painter: &egui::Painter, rect: Rect) {
        let (Some(d), Some(c)) = (&self.drag, self.cursor) else {
            return;
        };
        let to = d.target(c.point);
        let (a, b) = (
            self.view.to_screen(rect, d.from),
            self.view.to_screen(rect, to),
        );
        dashed(painter, a, b, Color32::from_gray(60));
        let variant = self.drag_preview.as_ref().is_some_and(|p| p.1);
        let mut problem = None;
        if let Some((_, _, ghost)) = &self.drag_preview {
            match ghost {
                Ok(polys) => {
                    for poly in polys {
                        let pts = self.polygon(rect, poly);
                        painter.add(Shape::closed_line(pts, Stroke::new(1.5, ACCENT)));
                    }
                }
                Err(e) => problem = Some(e.clone()),
            }
        }
        let action = match (d.grip, variant) {
            (Grip::Body, true) => "Copiar",
            (Grip::Body, false) => "Mover",
            (Grip::WallEnd { .. }, false) => "Estirar",
            (Grip::WallEnd { .. }, true) => "Estirar solo este muro",
            (Grip::SlabCorner { .. }, _) => "Esquina",
            (Grip::SlabEdge { .. }, false) => "Arista",
            (Grip::SlabEdge { .. }, true) => "Nueva esquina",
        };
        let delta = to - d.from;
        let mut text = if d.typed.is_empty() {
            format!(
                "{action}  {} mm   {}",
                fmt_mm(delta.length()),
                fmt_angle(delta.y.atan2(delta.x))
            )
        } else {
            format!(
                "{action}  {}▏   {}",
                d.typed,
                fmt_angle(delta.y.atan2(delta.x))
            )
        };
        if let Some(p) = problem {
            text.push_str(&format!("\nNo se puede: {p}"));
        }
        self.draw_label(painter, rect, c.point, text);
    }

    /// The room the magic wand would turn into a slab.
    fn draw_wand_preview(&self, painter: &egui::Painter, rect: Rect) {
        let (Tool::Slab, Some(c), Some((_, _, _, found))) =
            (self.tool, self.cursor, &self.wand_preview)
        else {
            return;
        };
        if self.slab_tool.is_drawing() || !(self.slab_settings.wand || self.space_held) {
            return;
        }
        match found {
            Ok(poly) => {
                let pts = self.polygon(rect, poly);
                painter.add(Shape::closed_line(pts, Stroke::new(2.0, ACCENT)));
                self.draw_label(
                    painter,
                    rect,
                    c.point,
                    format!(
                        "Varita mágica   S {}",
                        fmt_area(archisim_core::signed_area(poly).abs())
                    ),
                );
            }
            Err(e) => self.draw_label(painter, rect, c.point, format!("Varita mágica: {e}")),
        }
    }

    /// The wall about to be built, with its layers, its drawn line and a
    /// mark on its exterior side.
    fn draw_wall_preview(&self, painter: &egui::Painter, rect: Rect) {
        let (Tool::Wall, Some(a), Some(c)) = (self.tool, self.wall_tool.anchor(), self.cursor)
        else {
            return;
        };
        let b = self.wall_tool.typed_end(c.point).unwrap_or(c.point);
        let model = self.ed.model();
        let Some(wtype) = model.wall_type(self.wall_settings.wall_type) else {
            return;
        };
        let Some(u) = (b - a).normalized() else {
            return;
        };
        let n = u.perp_left();
        let probe = archisim_core::Wall {
            id: archisim_core::WallId(0),
            guid: archisim_core::GlobalId::new(),
            wall_type: wtype.id,
            level: self.ed.level,
            start: a,
            end: b,
            reference: self.wall_settings.reference,
            height: self.wall_settings.height,
            base_offset: self.wall_settings.base_offset,
            top: archisim_core::WallTop::Unconnected,
        };
        let (ext, int) = probe.face_offsets(wtype.thickness());
        let band = |hi: f64, lo: f64| -> Vec<Pos2> {
            [a + n * hi, b + n * hi, b + n * lo, a + n * lo]
                .iter()
                .map(|p| self.view.to_screen(rect, *p))
                .collect()
        };
        let mut high = ext;
        for layer in &wtype.layers {
            let low = high - layer.thickness;
            let [r, g, bl] = model
                .material(layer.material)
                .map_or([200, 200, 200], |m| m.cut_color);
            painter.add(Shape::convex_polygon(
                band(high, low),
                Color32::from_rgba_unmultiplied(r, g, bl, 170),
                Stroke::new(0.5, Color32::from_gray(90)),
            ));
            high = low;
        }
        painter.add(Shape::closed_line(
            band(ext, int),
            Stroke::new(1.5, Color32::from_gray(30)),
        ));
        dashed(
            painter,
            self.view.to_screen(rect, a),
            self.view.to_screen(rect, b),
            REF_LINE,
        );

        // "ext." on the exterior side of façade walls, by the middle.
        if wtype.function == WallFunction::Exterior {
            let mid = (a + b) * 0.5;
            let out =
                (self.view.to_screen(rect, mid + n) - self.view.to_screen(rect, mid)).normalized();
            let label_pos = self.view.to_screen(rect, mid + n * ext) + out * 14.0;
            painter.text(
                label_pos,
                Align2::CENTER_CENTER,
                "ext.",
                FontId::proportional(11.0),
                Color32::from_gray(90),
            );
        }

        // Length and angle by the cursor.
        let len = a.distance(b);
        let angle = (b - a).y.atan2((b - a).x);
        let text = if self.wall_tool.typed.is_empty() {
            format!("L {} mm   {}", fmt_mm(len), fmt_angle(angle))
        } else {
            format!("L: {}▏   {}", self.wall_tool.typed, fmt_angle(angle))
        };
        self.draw_label(painter, rect, c.point, text);
    }

    /// The slab outline being drawn: placed corners, the edge to the cursor
    /// and, dashed, the edge that will close it.
    fn draw_slab_preview(&self, painter: &egui::Painter, rect: Rect) {
        let (Tool::Slab, Some(c)) = (self.tool, self.cursor) else {
            return;
        };
        let corners = self.slab_tool.corners();
        let Some(&last) = corners.last() else {
            return;
        };
        let next = self.slab_tool.typed_end(c.point).unwrap_or(c.point);
        let mut pts: Vec<Point2> = corners.to_vec();
        pts.push(next);
        let screen = self.polygon(rect, &pts);
        let ink = Stroke::new(1.5, ACCENT);
        painter.add(Shape::line(screen.clone(), ink));
        if pts.len() >= 3 {
            dashed(painter, screen[screen.len() - 1], screen[0], ACCENT);
        }
        for p in &screen[..screen.len() - 1] {
            painter.rect_filled(
                Rect::from_center_size(*p, Vec2::splat(6.0)),
                CornerRadius::ZERO,
                ACCENT,
            );
        }
        let len = last.distance(next);
        let angle = (next - last).y.atan2((next - last).x);
        let mut text = if self.slab_tool.typed.is_empty() {
            format!("L {} mm   {}", fmt_mm(len), fmt_angle(angle))
        } else {
            format!("L: {}▏   {}", self.slab_tool.typed, fmt_angle(angle))
        };
        if pts.len() >= 3 {
            text.push_str(&format!(
                "   S {}",
                fmt_area(archisim_core::signed_area(&pts).abs())
            ));
        }
        self.draw_label(painter, rect, c.point, text);
    }

    /// Where the door or window would go: its rough opening across the
    /// wall, the frame, and for doors the leaf and its swing.
    fn draw_opening_preview(&self, painter: &egui::Painter, rect: Rect) {
        if !matches!(self.tool, Tool::Door | Tool::Window) {
            return;
        }
        let (Some(pl), Some(c)) = (&self.placement, self.cursor) else {
            return;
        };
        let g = &pl.geometry;
        let color = if pl.problem.is_some() {
            Color32::from_rgb(210, 40, 40)
        } else {
            ACCENT
        };
        let to = |t: f64, off: f64| self.view.to_screen(rect, g.point(t, off));
        let foot = self.polygon(rect, &g.footprint());
        painter.add(Shape::convex_polygon(
            foot,
            color.gamma_multiply(0.18),
            Stroke::new(1.5, color),
        ));
        // Frame faces.
        let stroke = Stroke::new(1.0, color);
        for off in [g.frame_exterior, g.frame_interior] {
            painter.line_segment([to(g.t0, off), to(g.t1, off)], stroke);
        }
        match g.kind {
            OpeningKind::Door(op) => {
                use archisim_core::DoorOperation as D;
                let face = g.swing_frame_face();
                let out = g.swing_dir();
                let mut leaves: Vec<(f64, f64)> = Vec::new(); // (hinge t, free t)
                match op {
                    D::SingleSwing => {
                        if g.hinge_at_start() {
                            leaves.push((g.t0, g.t1));
                        } else {
                            leaves.push((g.t1, g.t0));
                        }
                    }
                    D::DoubleSwing => {
                        let mid = (g.t0 + g.t1) / 2.0;
                        leaves.push((g.t0, mid));
                        leaves.push((g.t1, mid));
                    }
                    D::Sliding => {}
                }
                for (hinge, free) in leaves {
                    let r = (free - hinge).abs();
                    let h = g.point(hinge, face);
                    let along = g.u * (free - hinge).signum();
                    // Leaf open at 90°, and the arc its edge sweeps.
                    painter.line_segment(
                        [
                            self.view.to_screen(rect, h),
                            self.view.to_screen(rect, h + out * r),
                        ],
                        Stroke::new(2.0, color),
                    );
                    let arc: Vec<Pos2> = (0..=24)
                        .map(|i| {
                            let a = i as f64 / 24.0 * std::f64::consts::FRAC_PI_2;
                            self.view
                                .to_screen(rect, h + along * (r * a.cos()) + out * (r * a.sin()))
                        })
                        .collect();
                    painter.add(Shape::line(arc, Stroke::new(1.0, color)));
                }
            }
            OpeningKind::Window(_) => {
                let mid = (g.frame_exterior + g.frame_interior) / 2.0;
                painter.line_segment([to(g.t0, mid), to(g.t1, mid)], Stroke::new(2.0, color));
            }
        }
        let side = match g.opens_to {
            WallSide::Interior => "abre al interior",
            WallSide::Exterior => "abre al exterior",
        };
        let text = match &pl.problem {
            Some(p) => format!("No cabe: {p}"),
            None => format!(
                "{} mm ◂ ▸ {} mm   {side}",
                fmt_mm(pl.before),
                fmt_mm(pl.after)
            ),
        };
        self.draw_label(painter, rect, c.point, text);
    }

    fn draw_cursor(&self, painter: &egui::Painter, rect: Rect) {
        let points =
            self.tool.draws_points() && !(self.tool == Tool::Slab && self.slab_settings.wand);
        let (true, Some(c)) = (points || self.drag.is_some(), self.cursor) else {
            return;
        };
        let p = self.view.to_screen(rect, c.point);
        let stroke = Stroke::new(1.5, Color32::from_rgb(0, 150, 60));
        match c.kind {
            SnapKind::Endpoint | SnapKind::Corner => {
                painter.rect_stroke(
                    Rect::from_center_size(p, Vec2::splat(11.0)),
                    CornerRadius::ZERO,
                    stroke,
                    StrokeKind::Middle,
                );
            }
            SnapKind::Midpoint => {
                let pts = vec![
                    p + Vec2::new(0.0, -7.0),
                    p + Vec2::new(6.5, 5.0),
                    p + Vec2::new(-6.5, 5.0),
                ];
                painter.add(Shape::closed_line(pts, stroke));
            }
            SnapKind::OnEdge => {
                painter.line_segment([p + Vec2::new(-5.0, -5.0), p + Vec2::new(5.0, 5.0)], stroke);
                painter.line_segment([p + Vec2::new(-5.0, 5.0), p + Vec2::new(5.0, -5.0)], stroke);
            }
            SnapKind::Constrained | SnapKind::Tracked | SnapKind::Free => {}
        }
        let ink = Stroke::new(1.0, Color32::from_gray(40));
        painter.line_segment([p - Vec2::new(12.0, 0.0), p + Vec2::new(12.0, 0.0)], ink);
        painter.line_segment([p - Vec2::new(0.0, 12.0), p + Vec2::new(0.0, 12.0)], ink);
    }
}
