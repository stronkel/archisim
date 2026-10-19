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
        let reference = self.reference_level();
        self.other_walls = archisim_plan::other_level_walls(
            self.ed.model(),
            self.ed.level,
            self.cut_plane,
            reference,
        )
        .ids();
        self.track_cursor(&ctx, &response, rect);
        // Keys go to the plan unless a field is being typed in (a temporary
        // dimension's too: egui lets go of the focus on Esc before the plan
        // sees it).
        if ctx.memory(|m| m.focused().is_none()) && self.temp_edit.is_none() {
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
        self.draw_temp_dims(&painter, rect);
        self.draw_anchor_drag(&painter, rect);
        self.draw_flip_controls(&painter, rect);
        self.draw_hotspots(&painter, rect);
        self.draw_wall_preview(&painter, rect);
        self.draw_slab_preview(&painter, rect);
        self.draw_opening_preview(&painter, rect);
        self.draw_member_preview(&painter, rect);
        self.draw_space_preview(&painter, rect);
        self.draw_section_preview(&painter, rect);
        self.draw_roof_preview(&painter, rect);
        self.draw_stair_preview(&painter, rect);
        self.draw_terrain_preview(&painter, rect);
        self.draw_curtain_preview(&painter, rect);
        self.draw_railing_preview(&painter, rect);
        self.draw_wand_preview(&painter, rect);
        self.draw_dim_preview(&painter, rect);
        self.draw_drag_preview(&painter, rect);
        self.draw_tracking(&painter, rect);
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
        self.temp_edit_ui(&ctx);

        let icon = match self.tool {
            Tool::Wall
            | Tool::Slab
            | Tool::Door
            | Tool::Window
            | Tool::Dimension
            | Tool::Column
            | Tool::Beam
            | Tool::Zone
            | Tool::Section
            | Tool::Roof
            | Tool::Stair
            | Tool::Terrain
            | Tool::CurtainWall
            | Tool::Railing => CursorIcon::Crosshair,
            Tool::Arrow if self.drag.is_some() => CursorIcon::Grabbing,
            Tool::Arrow if matches!(self.hover_grip, Some(Grip::Body)) => CursorIcon::Move,
            Tool::Arrow if self.hover_grip.is_some() => CursorIcon::Grab,
            Tool::Arrow if self.hover.is_some() => CursorIcon::PointingHand,
            Tool::Arrow => CursorIcon::Default,
        };
        let icon = match response.hover_pos() {
            Some(pos) if self.drag.is_none() && self.over_temp_controls(rect, pos) => {
                CursorIcon::PointingHand
            }
            _ => icon,
        };
        if response.hovered() {
            ctx.set_cursor_icon(icon);
        }
    }

    pub(super) fn navigate(&mut self, ctx: &egui::Context, response: &egui::Response, rect: Rect) {
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

    /// The level drawn underneath the active plan, when the reference is on.
    pub(super) fn reference_level(&self) -> Option<LevelId> {
        if self.show_underlay {
            archisim_plan::level_below(self.ed.model(), self.ed.level)
        } else {
            None
        }
    }

    /// The snapped point near `raw`, with the snaps of the bottom bar.
    fn snap_at(&self, raw: Point2, reach: f64, extra: &[Point2], skip: &BTreeSet<Element>) -> Snap {
        snap_filtered(
            &self.ed,
            self.ed.level,
            raw,
            reach,
            extra,
            skip,
            self.snap_filter,
            &self.other_walls,
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
                let mut s = self.snap_at(raw, reach, &self.wall_tool.chain(), &BTreeSet::new());
                if let Some(a) = self.wall_tool.tracking_from() {
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
                self.cursor = Some(Snap::at(p, SnapKind::Free));
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
            Tool::Zone if self.zone_wand() => {
                let p = crate::snap::round_point(raw);
                self.cursor = Some(Snap::at(p, SnapKind::Free));
                self.hover = None;
                let contour = self.space_settings.contour;
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
            Tool::Roof if self.roof_wand() => {
                let p = crate::snap::round_point(raw);
                self.cursor = Some(Snap::at(p, SnapKind::Free));
                self.hover = None;
                let contour = Contour::ExteriorFace;
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
            Tool::Roof => {
                let mut s = self.snap_at(raw, reach, self.roof_tool.corners(), &BTreeSet::new());
                if let Some(a) = self.roof_tool.anchor() {
                    if ctx.input(|i| i.modifiers.shift) {
                        s = constrain(a, s);
                    } else {
                        s = self.track_at(a, s);
                    }
                }
                self.cursor = Some(s);
                self.hover = None;
            }
            Tool::Zone => {
                let mut s = self.snap_at(raw, reach, self.zone_tool.corners(), &BTreeSet::new());
                if let Some(a) = self.zone_tool.anchor() {
                    if ctx.input(|i| i.modifiers.shift) {
                        s = constrain(a, s);
                    } else {
                        s = self.track_at(a, s);
                    }
                }
                self.cursor = Some(s);
                self.hover = None;
            }
            Tool::Terrain => {
                let s = self.snap_at(raw, reach, &[], &BTreeSet::new());
                self.cursor = Some(s);
                self.hover = None;
            }
            Tool::Stair => {
                let mut s = self.snap_at(raw, reach, &self.stair_tool.points, &BTreeSet::new());
                if let Some(a) = self.stair_tool.points.last().copied() {
                    if ctx.input(|i| i.modifiers.shift) {
                        s = constrain(a, s);
                    } else {
                        s = self.track_at(a, s);
                    }
                }
                self.cursor = Some(s);
                self.hover = None;
            }
            Tool::Railing => {
                let mut s = self.snap_at(raw, reach, &self.railing_points, &BTreeSet::new());
                if let Some(a) = self.railing_points.last().copied() {
                    if ctx.input(|i| i.modifiers.shift) {
                        s = constrain(a, s);
                    } else {
                        s = self.track_at(a, s);
                    }
                }
                self.cursor = Some(s);
                self.hover = None;
            }
            Tool::CurtainWall => {
                let mut s = self.snap_at(raw, reach, &[], &BTreeSet::new());
                if let Some(a) = self.curtain_anchor {
                    if ctx.input(|i| i.modifiers.shift) {
                        s = constrain(a, s);
                    } else {
                        s = self.track_at(a, s);
                    }
                }
                self.cursor = Some(s);
                self.hover = None;
            }
            Tool::Section => {
                let mut s = self.snap_at(raw, reach, &[], &BTreeSet::new());
                if let Some(a) = self.section_tool.anchor {
                    if ctx.input(|i| i.modifiers.shift) {
                        s = constrain(a, s);
                    } else {
                        s = self.track_at(a, s);
                    }
                }
                self.cursor = Some(s);
                self.hover = None;
            }
            Tool::Column | Tool::Beam => {
                let mut s = self.snap_at(raw, reach, &[], &BTreeSet::new());
                if let Some(a) = self.beam_tool.anchor.filter(|_| self.tool == Tool::Beam) {
                    if ctx.input(|i| i.modifiers.shift) {
                        s = constrain(a, s);
                    } else {
                        s = self.track_at(a, s);
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
                self.placement = placement(&self.ed, self.ed.level, raw, reach, &settings);
                self.cursor = Some(Snap::at(crate::snap::round_point(raw), SnapKind::Free));
                self.hover = None;
            }
            Tool::Arrow if self.anchor_drag.is_some() => {
                // A witness end of a temporary dimension: anywhere it snaps.
                let s = self.snap_at(raw, reach, &[], &BTreeSet::new());
                self.cursor = Some(s);
                self.hover = None;
                self.hover_grip = None;
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
                    let ghost = crate::drag::preview(&self.ed, &self.ed.selection, &drag)
                        .map_err(|e| crate::units::explain(&e));
                    self.drag_preview = Some((to, variant, ghost));
                }
            }
            Tool::Arrow => {
                self.cursor = Some(Snap::at(crate::snap::round_point(raw), SnapKind::Free));
                self.hover_grip = if self.window_start.is_some() {
                    None
                } else {
                    grip_at(&self.ed, &self.ed.selection, raw, reach).map(|g| g.0)
                };
                self.hover = if self.window_start.is_some() {
                    None
                } else {
                    pick(&self.ed, self.ed.level, raw, reach)
                };
            }
        }
    }

    fn keyboard_in_view(&mut self, ctx: &egui::Context) {
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
        if self.drag.is_some() {
            self.keyboard_while_dragging(ctx, &events, enter, escape, backspace);
            return;
        }
        let drawing = self.wall_tool.is_drawing()
            || self.slab_tool.is_drawing()
            || self.dim_tool.is_active()
            || self.beam_tool.is_drawing()
            || self.zone_tool.is_drawing()
            || self.roof_tool.is_drawing()
            || !self.stair_tool.points.is_empty()
            || !self.terrain_points.is_empty();
        let mut used_text = false;
        for e in &events {
            if let egui::Event::Text(t) = e {
                for c in t.chars() {
                    used_text |= match self.tool {
                        Tool::Wall => self.wall_tool.type_char(c),
                        Tool::Slab => self.slab_tool.type_char(c),
                        Tool::Zone => self.zone_tool.type_char(c),
                        Tool::Roof => self.roof_tool.type_char(c),
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
                Tool::Zone => self.zone_tool.backspace(),
                Tool::Roof => self.roof_tool.backspace(),
                Tool::Stair => self.stair_tool.points.pop().is_some(),
                Tool::Railing => self.railing_points.pop().is_some(),
                Tool::Terrain => self.terrain_points.pop().is_some(),
                _ => false,
            };
        if !edited_length && (delete || backspace) && !drawing {
            self.delete();
        }
        // Space turns the selected door or window (or flips the selected
        // walls); with the door or window tool, the one being placed.
        if space && !drawing {
            match self.tool {
                Tool::Door | Tool::Window => {
                    let s = if self.tool == Tool::Door {
                        &mut self.door_settings
                    } else {
                        &mut self.window_settings
                    };
                    s.hand = match s.hand {
                        Hand::Left => Hand::Right,
                        Hand::Right => Hand::Left,
                    };
                    let hand = s.hand;
                    self.say(match hand {
                        Hand::Left => "Mano izquierda (Espacio para cambiarla).",
                        Hand::Right => "Mano derecha (Espacio para cambiarla).",
                    });
                }
                Tool::Arrow => {
                    let sel: Vec<Element> = self.ed.selection.iter().copied().collect();
                    match self.ed.turn_members(&sel) {
                        Ok(0) => {
                            self.flip_selected(None);
                        }
                        Ok(_) => self.say("Pilares girados 90° / vigas invertidas."),
                        Err(e) => self.complain(format!("No se puede girar: {e}.")),
                    }
                }
                Tool::Column => {
                    let r = &mut self.column_settings.rotation;
                    *r = crate::editor::normal_angle(*r + std::f64::consts::FRAC_PI_2);
                    let deg = r.to_degrees().round();
                    self.say(format!("Giro del pilar: {deg}° (Espacio: +90°)."));
                }
                _ => {}
            }
        }
        if enter {
            let towards = self.cursor.map(|c| c.point).unwrap_or_default();
            match self.tool {
                Tool::Wall => {
                    if let Outcome::Build(pieces) = self.wall_tool.enter(towards) {
                        self.build_walls(pieces);
                    }
                }
                Tool::Slab => match self.slab_tool.enter(towards) {
                    SlabOutcome::Build(outline) => self.build_slab(outline),
                    SlabOutcome::Invalid => self.complain(
                        "El contorno necesita al menos tres esquinas y no puede cruzarse.",
                    ),
                    SlabOutcome::Nothing => {}
                },
                Tool::Terrain => self.finish_terrain(),
                Tool::Railing => {
                    if let Some(c) = self.cursor {
                        if self.railing_points.len() == 1 {
                            self.railing_click(c.point);
                        }
                    }
                    self.finish_railing();
                }
                Tool::Stair => {
                    if let Some(c) = self.cursor {
                        if self.stair_tool.points.len() == 1 {
                            self.stair_click(c.point);
                        }
                    }
                    self.finish_stair();
                }
                Tool::Roof => match self.roof_tool.enter(towards) {
                    SlabOutcome::Build(outline) => self.build_roof(outline),
                    SlabOutcome::Invalid => self.complain(
                        "El contorno necesita al menos tres esquinas y no puede cruzarse.",
                    ),
                    SlabOutcome::Nothing => {}
                },
                Tool::Zone => match self.zone_tool.enter(towards) {
                    SlabOutcome::Build(outline) => self.build_space(outline),
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
                Tool::Door | Tool::Window | Tool::Column => self.set_tool(Tool::Arrow),
                Tool::Zone => {
                    if !self.zone_tool.escape() {
                        self.set_tool(Tool::Arrow);
                    }
                }
                Tool::Roof => {
                    if !self.roof_tool.escape() {
                        self.set_tool(Tool::Arrow);
                    }
                }
                Tool::Stair => {
                    if self.stair_tool.points.pop().is_none() {
                        self.set_tool(Tool::Arrow);
                    }
                }
                Tool::Terrain => {
                    if self.terrain_points.pop().is_none() {
                        self.set_tool(Tool::Arrow);
                    }
                }
                Tool::Section => {
                    if self.section_tool.anchor.take().is_none() {
                        self.set_tool(Tool::Arrow);
                    }
                }
                Tool::Railing => {
                    if self.railing_points.pop().is_none() {
                        self.set_tool(Tool::Arrow);
                    }
                }
                Tool::CurtainWall => {
                    if self.curtain_anchor.take().is_none() {
                        self.set_tool(Tool::Arrow);
                    }
                }
                Tool::Beam => {
                    if self.beam_tool.is_drawing() {
                        self.beam_tool.finish();
                    } else {
                        self.set_tool(Tool::Arrow);
                    }
                }
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
        // The values of the temporary dimensions and the flip arrows come
        // before the tool, as in Revit.
        if response.clicked_by(PointerButton::Primary) {
            if let Some(pos) = response.interact_pointer_pos() {
                if self.click_temp_controls(rect, pos) {
                    return;
                }
            }
        }
        match self.tool {
            Tool::Wall => {
                if response.clicked_by(PointerButton::Primary) {
                    if let Some(c) = self.cursor {
                        if let Outcome::Build(pieces) = self.wall_tool.click(c.point) {
                            self.build_walls(pieces);
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
            Tool::Terrain => {
                if response.double_clicked_by(PointerButton::Primary) {
                    self.finish_terrain();
                } else if response.clicked_by(PointerButton::Primary) {
                    if let Some(c) = self.cursor {
                        self.terrain_click(c.point);
                    }
                }
            }
            Tool::Stair => {
                if response.double_clicked_by(PointerButton::Primary)
                    || response.clicked_by(PointerButton::Secondary)
                {
                    self.finish_stair();
                } else if response.clicked_by(PointerButton::Primary) {
                    if let Some(c) = self.cursor {
                        self.stair_click(c.point);
                    }
                }
            }
            Tool::Roof => {
                if response.clicked_by(PointerButton::Primary) {
                    if let Some(c) = self.cursor {
                        self.roof_click(c.point);
                    }
                }
                if response.clicked_by(PointerButton::Secondary) {
                    self.roof_tool.finish();
                }
            }
            Tool::Zone => {
                if response.clicked_by(PointerButton::Primary) {
                    if let Some(c) = self.cursor {
                        self.space_click(c.point);
                    }
                }
                if response.clicked_by(PointerButton::Secondary) {
                    self.zone_tool.finish();
                }
            }
            Tool::Column | Tool::Beam => {
                if response.clicked_by(PointerButton::Primary) {
                    if let Some(c) = self.cursor {
                        self.member_click(c.point);
                    }
                }
                if response.clicked_by(PointerButton::Secondary)
                    || response.double_clicked_by(PointerButton::Primary)
                {
                    self.beam_tool.finish();
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
                            &self.ed,
                            self.ed.level,
                            self.dim_settings,
                            c.point,
                            reach,
                        );
                        self.dim_outcome(out);
                    }
                }
            }
            Tool::Railing => {
                if response.double_clicked_by(PointerButton::Primary)
                    || response.clicked_by(PointerButton::Secondary)
                {
                    self.finish_railing();
                } else if response.clicked_by(PointerButton::Primary) {
                    if let Some(c) = self.cursor {
                        self.railing_click(c.point);
                    }
                }
            }
            Tool::CurtainWall => {
                if response.clicked_by(PointerButton::Primary) {
                    if let Some(c) = self.cursor {
                        self.curtain_click(c.point);
                    }
                }
                if response.clicked_by(PointerButton::Secondary) {
                    self.curtain_anchor = None;
                }
            }
            Tool::Section => {
                if response.clicked_by(PointerButton::Primary) {
                    if let Some(c) = self.cursor {
                        self.section_click(c.point);
                    }
                }
                if response.clicked_by(PointerButton::Secondary) {
                    self.section_tool.anchor = None;
                }
            }
            Tool::Arrow => {
                // Double click on a section or elevation mark: its view.
                if response.double_clicked_by(PointerButton::Primary) {
                    if let Some(Element::Section(id)) = self.hover {
                        self.open_view(TabKind::Section(id));
                        return;
                    }
                }
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
                // Handles of the selection (wall ends, slab corners) win
                // over the witness ends of the temporary dimensions.
                let anchor = response
                    .drag_started_by(PointerButton::Primary)
                    .then(|| ctx.input(|i| i.pointer.press_origin()))
                    .flatten()
                    .filter(|p| {
                        let at = self.view.to_model(rect, *p);
                        let reach = self.view.to_metres(REACH);
                        !matches!(
                            grip_at(&self.ed, &self.ed.selection, at, reach),
                            Some((g, _)) if g != Grip::Body
                        )
                    })
                    .and_then(|p| self.anchor_at(rect, p));
                if let Some(a) = anchor {
                    self.anchor_drag = Some(a);
                } else if response.drag_started_by(PointerButton::Primary) {
                    let origin = ctx
                        .input(|i| i.pointer.press_origin())
                        .map(|p| self.view.to_model(rect, p));
                    let reach = self.view.to_metres(REACH);
                    let grip = origin.and_then(|o| grip_at(&self.ed, &self.ed.selection, o, reach));
                    match grip {
                        Some((grip, from)) => {
                            // The selection is grabbed by the nearest
                            // special point, so it can be dropped exactly on
                            // another one.
                            let from = if grip == Grip::Body {
                                snap(&self.ed, self.ed.level, from, reach, &[]).point
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
                if response.drag_stopped() && self.anchor_drag.is_some() {
                    self.drop_anchor();
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
                        let found = in_window(&self.ed, self.ed.level, a, c.point);
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
            reference_level: self.reference_level(),
            thin_lines: self.thin_lines,
            phase_view: self.phase_view,
        };
        if self.plan.as_ref().map(|p| &p.0) != Some(&key) {
            let started = std::time::Instant::now();
            let image = rasterize_with(
                self.ed.model(),
                self.ed.level,
                size,
                &archisim_plan::PlanOptions {
                    viewport: Some((lo, hi)),
                    show_reference_lines: self.show_reference_lines,
                    scale: self.plan_scale,
                    cut_plane: self.cut_plane,
                    reference_level: key.reference_level,
                    thin_lines: key.thin_lines,
                    phase_view: key.phase_view,
                    ..archisim_plan::PlanOptions::default()
                },
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

    /// The project origin (0, 0, ±0,00), as the survey mark of a hotspot:
    /// a circle with two filled quarters. It is a snap point of its own.
    fn draw_origin(&self, painter: &egui::Painter, rect: Rect) {
        let o = self.view.to_screen(rect, Point2::new(0.0, 0.0));
        let ink = Color32::from_rgb(70, 72, 84);
        let s = Stroke::new(1.2, ink);
        let r = 6.0;
        for (a0, a1) in [(0.0f32, 90.0f32), (180.0, 270.0)] {
            let mut pts = vec![o];
            for k in 0..=8 {
                let a = (a0 + (a1 - a0) * k as f32 / 8.0).to_radians();
                // Screen y grows downwards: quarters NE and SW.
                pts.push(o + Vec2::new(r * a.cos(), -r * a.sin()));
            }
            painter.add(Shape::convex_polygon(pts, ink, Stroke::NONE));
        }
        painter.circle_stroke(o, r, s);
        painter.line_segment(
            [o - Vec2::new(r + 5.0, 0.0), o + Vec2::new(r + 5.0, 0.0)],
            s,
        );
        painter.line_segment(
            [o - Vec2::new(0.0, r + 5.0), o + Vec2::new(0.0, r + 5.0)],
            s,
        );
    }

    pub(super) fn polygon(&self, rect: Rect, poly: &[Point2]) -> Vec<Pos2> {
        poly.iter().map(|p| self.view.to_screen(rect, *p)).collect()
    }

    fn draw_selection(&self, painter: &egui::Painter, rect: Rect) {
        let model = self.ed.model();
        if let Some(h) = self.hover {
            if !self.ed.selection.contains(&h) {
                if let Some(poly) = outline(&self.ed, h) {
                    painter.add(Shape::closed_line(
                        self.polygon(rect, &poly),
                        Stroke::new(2.0, ACCENT.gamma_multiply(0.5)),
                    ));
                }
            }
        }
        for e in &self.ed.selection {
            let Some(poly) = outline(&self.ed, *e) else {
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
                    match w.arc() {
                        Some(arc) => {
                            let pts = self.polygon(rect, &arc.polyline(0.0, 0.01));
                            dashed_path(painter, &pts, REF_LINE);
                        }
                        None => dashed(
                            painter,
                            self.view.to_screen(rect, w.start),
                            self.view.to_screen(rect, w.end),
                            REF_LINE,
                        ),
                    }
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
                Grip::WallBend { .. } => {
                    let r = if hot { 6.0 } else { 4.5 };
                    let inside = if hot { ACCENT } else { Color32::WHITE };
                    painter.circle(c, r, inside, Stroke::new(1.2, Color32::BLACK));
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
            (Grip::WallBend { .. }, _) => "Curvar",
            (Grip::BeamEnd { .. }, _) => "Estirar viga",
            (Grip::SpaceLabel { .. }, _) => "Rótulo",
            (Grip::SectionEnd { .. }, _) => "Estirar sección",
            (Grip::SlopeEnd { .. }, _) => "Pendiente",
            (Grip::RoofCorner { .. }, _) => "Esquina",
            (Grip::StairPoint { .. }, _) => "Línea de huella",
            (Grip::CurtainEnd { .. }, _) => "Estirar muro cortina",
            (Grip::RailingPoint { .. }, _) => "Barandilla",
            (Grip::TerrainPoint { .. }, _) => "Punto",
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
        let (Tool::Slab | Tool::Zone | Tool::Roof, Some(c), Some((_, _, _, found))) =
            (self.tool, self.cursor, &self.wand_preview)
        else {
            return;
        };
        let off = match self.tool {
            Tool::Zone => !self.zone_wand(),
            Tool::Roof => !self.roof_wand(),
            _ => self.slab_tool.is_drawing() || !(self.slab_settings.wand || self.space_held),
        };
        if off {
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

    /// The walls about to be built, with their layers, their drawn lines
    /// and a mark on the exterior side.
    fn draw_wall_preview(&self, painter: &egui::Painter, rect: Rect) {
        let (Tool::Wall, Some(a), Some(c)) = (self.tool, self.wall_tool.anchor(), self.cursor)
        else {
            return;
        };
        let model = self.ed.model();
        let Some(wtype) = model.wall_type(self.wall_settings.wall_type) else {
            return;
        };
        let pieces = self.wall_tool.preview(c.point);
        let mode = self.wall_tool.mode;
        // Placed points: the centre of a circle, the ends of an arc.
        let marks: Vec<Point2> = self.wall_tool.chain();
        for p in marks.iter().skip(marks.len().saturating_sub(2)) {
            let q = self.view.to_screen(rect, *p);
            painter.circle_stroke(q, 4.0, Stroke::new(1.5, ACCENT));
        }
        if mode == WallMode::Circle {
            dashed(
                painter,
                self.view.to_screen(rect, a),
                self.view.to_screen(rect, c.point),
                ACCENT,
            );
        }
        if pieces.is_empty() {
            return;
        }
        for w in &pieces {
            let probe = archisim_core::Wall {
                id: archisim_core::WallId(0),
                guid: archisim_core::GlobalId::new(),
                wall_type: wtype.id,
                level: self.ed.level,
                start: w.start,
                end: w.end,
                bulge: w.bulge,
                reference: self.wall_settings.reference,
                height: self.wall_settings.height,
                base_offset: self.wall_settings.base_offset,
                top: archisim_core::WallTop::Unconnected,
            };
            let (ext, int) = probe.face_offsets(wtype.thickness());
            // The curve (or line) of the wall cut into short straight
            // pieces, each drawn as convex quads per layer.
            let steps = probe
                .arc()
                .map_or(1, |arc| arc.segments(ext.max(int), 0.01).min(96));
            let at = |f: f64, off: f64| self.view.to_screen(rect, probe.point_at(f, off));
            let mut high = ext;
            for layer in &wtype.layers {
                let low = high - layer.thickness;
                let [r, g, bl] = model
                    .material(layer.material)
                    .map_or([200, 200, 200], |m| m.cut_color);
                let fill = Color32::from_rgba_unmultiplied(r, g, bl, 170);
                for i in 0..steps {
                    let (f0, f1) = (i as f64 / steps as f64, (i + 1) as f64 / steps as f64);
                    painter.add(Shape::convex_polygon(
                        vec![at(f0, high), at(f1, high), at(f1, low), at(f0, low)],
                        fill,
                        Stroke::NONE,
                    ));
                }
                high = low;
            }
            let face = |off: f64| -> Vec<Pos2> {
                (0..=steps)
                    .map(|i| at(i as f64 / steps as f64, off))
                    .collect()
            };
            let ink = Stroke::new(1.5, Color32::from_gray(30));
            painter.add(Shape::line(face(ext), ink));
            painter.add(Shape::line(face(int), ink));
            painter.line_segment([at(0.0, ext), at(0.0, int)], ink);
            painter.line_segment([at(1.0, ext), at(1.0, int)], ink);
            dashed_path(painter, &face(0.0), REF_LINE);
            // "ext." on the exterior side of façade walls, by the middle.
            if wtype.function == WallFunction::Exterior {
                let mid = probe.point_at(0.5, 0.0);
                let out_pt = probe.point_at(0.5, 1.0);
                let out = (self.view.to_screen(rect, out_pt) - self.view.to_screen(rect, mid))
                    .normalized();
                let label_pos = self.view.to_screen(rect, probe.point_at(0.5, ext)) + out * 14.0;
                painter.text(
                    label_pos,
                    Align2::CENTER_CENTER,
                    "ext.",
                    FontId::proportional(11.0),
                    Color32::from_gray(90),
                );
            }
        }

        // Sizes by the cursor.
        let typed = &self.wall_tool.typed;
        let w = pieces[0];
        let text = match (mode, w.arc()) {
            (WallMode::Rectangle, _) => {
                let (x, y) = (pieces[0].length(), pieces[1].length());
                if typed.is_empty() {
                    format!("{} × {} mm   S {}", fmt_mm(x), fmt_mm(y), fmt_area(x * y))
                } else {
                    format!("{typed}▏ (ancho x fondo)")
                }
            }
            (WallMode::Circle, Some(arc)) if typed.is_empty() => {
                format!("R {} mm", fmt_mm(arc.radius))
            }
            (WallMode::Circle, _) => format!("R: {typed}▏"),
            (WallMode::Arc, Some(arc)) if self.wall_tool.arc_end().is_some() => {
                if typed.is_empty() {
                    format!(
                        "R {} mm   L {} mm",
                        fmt_mm(arc.radius),
                        fmt_mm(arc.length())
                    )
                } else {
                    format!("R: {typed}▏")
                }
            }
            (WallMode::Arc, None) if self.wall_tool.arc_end().is_some() => {
                if typed.is_empty() {
                    "Recto (punto alineado)".to_string()
                } else {
                    format!("R: {typed}▏")
                }
            }
            _ => {
                let angle = (w.end - w.start).y.atan2((w.end - w.start).x);
                if typed.is_empty() {
                    format!("L {} mm   {}", fmt_mm(w.length()), fmt_angle(angle))
                } else {
                    format!("L: {typed}▏   {}", fmt_angle(angle))
                }
            }
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
        if self.slab_tool.mode == SlabMode::Rectangle {
            if let Some(outline) = self.slab_tool.rectangle_to(c.point) {
                let screen = self.polygon(rect, &outline);
                painter.add(Shape::closed_line(screen, Stroke::new(1.5, ACCENT)));
                let (x, y) = (
                    outline[0].distance(outline[1]),
                    outline[1].distance(outline[2]),
                );
                let text = if self.slab_tool.typed.is_empty() {
                    format!("{} × {} mm   S {}", fmt_mm(x), fmt_mm(y), fmt_area(x * y))
                } else {
                    format!("{}▏ (ancho x fondo)", self.slab_tool.typed)
                };
                self.draw_label(painter, rect, c.point, text);
            }
            return;
        }
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
            OpeningKind::Void => {}
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
                    D::Sliding | D::Folding => {}
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

    /// The point the line being drawn starts from: the last point of the
    /// wall chain, slab outline or dimension, or where a drag started.
    fn track_anchor(&self) -> Option<Point2> {
        if let Some(d) = &self.drag {
            return Some(d.from);
        }
        match self.tool {
            Tool::Wall => self.wall_tool.anchor(),
            Tool::Slab => self.slab_tool.anchor(),
            Tool::Beam => self.beam_tool.anchor,
            Tool::Section => self.section_tool.anchor,
            Tool::CurtainWall => self.curtain_anchor,
            Tool::Railing => self.railing_points.last().copied(),
            Tool::Zone => self.zone_tool.anchor(),
            Tool::Roof => self.roof_tool.anchor(),
            Tool::Stair => self.stair_tool.points.last().copied(),
            Tool::Dimension if !self.dim_tool.is_placing() => {
                self.dim_tool.points().last().copied()
            }
            _ => None,
        }
    }

    /// The tracking line: when the line being drawn runs exactly
    /// horizontal, vertical or at 45°, a blue line across the view through
    /// its start says so (Revit's and ArchiCAD's tracking line).
    fn draw_tracking(&self, painter: &egui::Painter, rect: Rect) {
        let (Some(a), Some(c)) = (self.track_anchor(), self.cursor) else {
            return;
        };
        if !is_tracking_line(a, c.point) {
            return;
        }
        let Some(dir) = (c.point - a).normalized() else {
            return;
        };
        // Long enough to cross the whole view in both directions.
        let span = self.view.to_metres(rect.width() + rect.height());
        let (p, q) = (
            self.view.to_screen(rect, a - dir * span),
            self.view.to_screen(rect, a + dir * span),
        );
        painter.extend(Shape::dashed_line(
            &[p, q],
            Stroke::new(1.0, TRACK_LINE),
            6.0,
            4.0,
        ));
    }

    fn draw_cursor(&self, painter: &egui::Painter, rect: Rect) {
        let points = self.tool.draws_points()
            && !(self.tool == Tool::Slab && self.slab_settings.wand)
            && !(self.tool == Tool::Zone && self.zone_wand())
            && !(self.tool == Tool::Roof && self.roof_wand());
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
            SnapKind::Centre => {
                painter.circle_stroke(p, 6.0, stroke);
                painter.circle_filled(p, 1.8, stroke.color);
            }
            SnapKind::Origin => {
                painter.circle_stroke(p, 10.0, Stroke::new(2.0, stroke.color));
                painter.rect_stroke(
                    Rect::from_center_size(p, Vec2::splat(13.0)),
                    CornerRadius::ZERO,
                    stroke,
                    StrokeKind::Middle,
                );
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
