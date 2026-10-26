//! AutoCAD-style drafting in the window: the command line at the bottom of
//! the drawing, typed commands and coordinates, the running command's
//! clicks and what it draws following the cursor, and the box to write
//! texts in.

use super::*;
use crate::cad::{self, CadSettings, Changes, Ctx, Input, Launch, Prompt, Step, Want};

/// Lines kept in the command line's history.
const HISTORY: usize = 200;

/// The command line: what is being typed, what was said, and the last
/// command (Enter on an empty line repeats it).
#[derive(Debug, Default)]
pub struct CmdLine {
    pub buffer: String,
    /// Lines shown above the prompt; `true` for errors.
    pub history: Vec<(String, bool)>,
    pub last: Option<&'static str>,
}

impl CmdLine {
    pub fn log(&mut self, text: impl Into<String>, error: bool) {
        for line in text.into().lines() {
            self.history.push((line.to_string(), error));
        }
        let n = self.history.len();
        if n > HISTORY {
            self.history.drain(..n - HISTORY);
        }
    }
}

/// Commands that work on the selection: it is cleared when they finish,
/// as in AutoCAD.
const EDITING: &[&str] = &[
    "DESPLAZA",
    "COPIA",
    "GIRA",
    "ESCALA",
    "SIMETRIA",
    "ALINEAR",
    "MATRIZ",
    "JUNTAR",
    "DESCOMP",
    "BORRA",
    "IGUALARPROP",
    "LIST",
];

impl ArchiSimApp {
    /// The prompt of the running command.
    pub(super) fn cad_prompt(&self) -> Option<Prompt> {
        self.cad.as_ref().map(|c| c.prompt())
    }

    /// What the running command asks for.
    pub(super) fn cad_want(&self) -> Option<Want> {
        self.cad_prompt().map(|p| p.want)
    }

    /// Runs `f` on the running command with a context of the model.
    fn with_cmd<R>(
        &mut self,
        f: impl FnOnce(&mut dyn cad::Command, &Ctx<'_, dyn crate::geo::Geo + '_>, &mut CadSettings) -> R,
    ) -> Option<R> {
        let mut cmd = self.cad.take()?;
        let mut st = std::mem::take(&mut self.cad_settings);
        let cursor = self.cursor.map(|c| c.point).unwrap_or_default();
        let (view, k) = (self.active_view(), self.active_k());
        let r = {
            let cx = Ctx {
                geo: &self.ed as &dyn crate::geo::Geo,
                view,
                selection: &self.ed.selection,
                k,
                cursor,
            };
            f(cmd.as_mut(), &cx, &mut st)
        };
        self.cad_settings = st;
        self.cad = Some(cmd);
        Some(r)
    }

    /// Starts something typed in the command line or chosen in the
    /// toolbox.
    pub(super) fn launch(&mut self, l: Launch) {
        match l {
            Launch::Command(name) => self.start_command(name),
            Launch::Wall => self.pick(toolbox::Pick::Tool(Tool::Wall)),
            Launch::Slab => self.pick(toolbox::Pick::Tool(Tool::Slab)),
            Launch::Door => self.pick(toolbox::Pick::Tool(Tool::Door)),
            Launch::Window => self.pick(toolbox::Pick::Tool(Tool::Window)),
            Launch::Dimension => self.pick(toolbox::Pick::Tool(Tool::Dimension)),
            Launch::LevelMark => self.pick(toolbox::Pick::LevelMark),
            Launch::Layers => self.show_layers = true,
            Launch::Undo => self.undo(),
            Launch::Redo => self.redo(),
            Launch::ZoomExtents => self.fit_active_view(),
            Launch::Ortho => self.toggle_ortho(),
            Launch::Select => self.set_tool(Tool::Arrow),
        }
    }

    pub(super) fn toggle_ortho(&mut self) {
        self.cad_settings.ortho = !self.cad_settings.ortho;
        let text = if self.cad_settings.ortho {
            "<Orto activado>"
        } else {
            "<Orto desactivado>"
        };
        self.cmdline.log(text, false);
        self.say(text);
    }

    /// Starts a 2D command by its name.
    pub fn start_command(&mut self, name: &'static str) {
        let Some(cmd) = cad::new_command(name) else {
            return;
        };
        self.end_command(false);
        self.set_tool(Tool::Cad);
        self.cad = Some(cmd);
        self.cmdline.last = Some(name);
        self.cmdline.log(format!("Comando: {name}"), false);
        if let Some(step) = self.with_cmd(|c, cx, st| c.start(cx, st)) {
            self.cad_step(step);
        }
    }

    /// Gives an input to the running command.
    pub(super) fn cad_feed(&mut self, input: Input) {
        if let Some(step) = self.with_cmd(|c, cx, st| c.feed(input, cx, st)) {
            self.cad_step(step);
        }
    }

    fn apply_changes(&mut self, ch: &Changes) -> bool {
        let view = self.active_view();
        match self.ed.apply_cad(view, &self.cad_settings, ch) {
            Ok(_) => true,
            Err(e) => {
                self.cmdline.log(format!("No se puede: {e}"), true);
                self.complain(format!("No se puede: {e}."));
                false
            }
        }
    }

    fn cad_step(&mut self, step: Step) {
        match step {
            Step::Next => {}
            Step::Apply(ch) => {
                self.apply_changes(&ch);
            }
            Step::Done(ch) => {
                if let Some(ch) = ch {
                    if self.apply_changes(&ch) {
                        self.say(ch.label);
                    }
                }
                self.end_command(false);
            }
            Step::Say(text) => {
                self.cmdline.log(text.clone(), false);
                self.say(text);
            }
            Step::SayDone(text) => {
                self.cmdline.log(text.clone(), false);
                self.say(text.lines().next().unwrap_or_default().to_string());
                self.end_command(false);
            }
            Step::Refuse(text) => {
                self.cmdline.log(text.clone(), true);
                self.complain(text);
            }
        }
    }

    /// Ends the running command; with `keep`, first makes what it still
    /// holds (Esc on a line drawn segment by segment keeps it).
    pub(super) fn end_command(&mut self, keep: bool) {
        if self.cad.is_none() {
            return;
        }
        if keep {
            if let Some(Some(ch)) = self.with_cmd(|c, cx, st| c.finish(cx, st)) {
                self.apply_changes(&ch);
            }
        }
        let name = self.cad.take().map(|c| c.name());
        if name.is_some_and(|n| EDITING.contains(&n)) {
            self.ed.selection.clear();
        }
        if self.tool == Tool::Cad {
            self.tool = Tool::Arrow;
        }
    }

    /// Esc with a command running: keep what it holds and end it.
    pub(super) fn cad_escape(&mut self) {
        self.cmdline.log("*Cancelar*", false);
        self.end_command(true);
    }

    /// Enter on the command line: the text goes to the running command, or
    /// starts one.
    pub(super) fn submit_command_line(&mut self) {
        let text = std::mem::take(&mut self.cmdline.buffer);
        let text = text.trim().to_string();
        if text.is_empty() {
            return;
        }
        let cursor = self.cursor.map(|c| c.point).unwrap_or_default();
        if let Some(prompt) = self.cad_prompt() {
            self.cmdline.log(format!("{} {text}", prompt.line()), false);
            let base = self.cad.as_ref().and_then(|c| c.base());
            let input = cad::typed_input(&prompt, &text, base, cursor);
            // U (DESHACER) or REHACER typed while a command runs (between
            // the corners of Esquina, say): the last step is undone and the
            // command goes on.
            if let Input::Value(v) = &input {
                match cad::launch(v) {
                    Some(cad::Launch::Undo) => {
                        self.undo();
                        return;
                    }
                    Some(cad::Launch::Redo) => {
                        self.redo();
                        return;
                    }
                    _ => {}
                }
            }
            // Another command typed while one runs: it replaces it.
            if let Input::Value(v) = &input {
                if prompt.want == Want::Point {
                    if let Some(l) = cad::launch(v) {
                        self.end_command(true);
                        self.launch(l);
                        return;
                    }
                    if parse_length(v).is_none() {
                        self.cmdline.log("Punto no válido: escribe x,y · @dx,dy · @distancia<ángulo · o una distancia.", true);
                        return;
                    }
                }
            }
            self.cad_feed(input);
            return;
        }
        // A coordinate for the wall, slab or dimension tool.
        if text.starts_with(['@', '#']) || (text.contains(';') && self.bim_anchor().is_some()) {
            let base = self.bim_anchor();
            match cad::parse_point(&text, base, cursor) {
                Some(p) => {
                    self.cmdline.log(format!("Punto: {text}"), false);
                    self.bim_point(p);
                }
                None => self.cmdline.log(format!("«{text}» no es un punto."), true),
            }
            return;
        }
        match cad::launch(&text) {
            Some(l @ Launch::Command(_)) => self.launch(l),
            Some(l) => {
                self.cmdline.log(format!("Comando: {text}"), false);
                self.launch(l);
            }
            None => {
                let msg = format!("Orden desconocida «{text}». Prueba L, PL, C, REC, D, CP, GI, SI, EQ, RR, AL, EM, MA, SB, TX…");
                self.cmdline.log(msg.clone(), true);
                self.complain(msg);
            }
        }
    }

    /// The last point of the wall, slab or dimension being drawn.
    pub(super) fn bim_anchor(&self) -> Option<Point2> {
        match self.tool {
            Tool::Wall => self.wall_tool.anchor(),
            Tool::Slab => self.slab_tool.anchor(),
            Tool::Dimension => self.dim_tool.points().last().copied(),
            _ => None,
        }
    }

    /// A point typed for the wall, slab or dimension tool: as a click there.
    fn bim_point(&mut self, p: Point2) {
        match self.tool {
            Tool::Wall => {
                if let crate::wall_tool::Outcome::Build(pieces) = self.wall_tool.click(p) {
                    self.build_walls(pieces);
                }
            }
            Tool::Slab => match self.slab_tool.click(p) {
                SlabOutcome::Build(outline) => self.build_slab(outline),
                SlabOutcome::Invalid => {
                    self.complain("El contorno no puede cruzarse: corrige la última esquina.")
                }
                SlabOutcome::Nothing => {}
            },
            Tool::Dimension => {
                let reach = self.view.to_metres(REACH);
                let out = self
                    .dim_tool
                    .click(&self.ed, self.ed.level, self.dim_settings, p, reach);
                self.dim_outcome(out);
            }
            _ => self
                .cmdline
                .log("No hay ninguna herramienta esperando un punto.", true),
        }
    }

    /// Keys for the command line. Returns whether they were used (then
    /// the plan does not see them).
    /// Whether a building element is being drawn (a wall, slab, dimension,
    /// beam, room, roof, stair, terrain, railing, curtain wall or section
    /// line): what is typed then are its lengths, not commands.
    pub(super) fn drawing_bim(&self) -> bool {
        self.wall_tool.is_drawing()
            || self.slab_tool.is_drawing()
            || self.dim_tool.is_active()
            || self.beam_tool.is_drawing()
            || self.zone_tool.is_drawing()
            || self.roof_tool.is_drawing()
            || !self.stair_tool.points.is_empty()
            || !self.terrain_points.is_empty()
            || !self.terrain_line.is_empty()
            || !self.railing_points.is_empty()
            || self.curtain_anchor.is_some()
            || self.section_tool.anchor.is_some()
            || self.mass_tool.is_drawing()
            || self.concept.is_drawing()
            || self.plot_tool.is_drawing()
            || !self.road_points.is_empty()
            || !self.spline_points.is_empty()
            || self.grid_anchor.is_some()
            || self.footing_anchor.is_some()
    }

    pub(super) fn command_keys(
        &mut self,
        events: &[egui::Event],
        enter: bool,
        escape: bool,
        backspace: bool,
        space: bool,
    ) -> bool {
        let drawing_bim = self.drawing_bim();
        let mut used = false;
        let mut escaped = false;
        for e in events {
            let cad = self.tool == Tool::Cad;
            // Esc acts where it was pressed among the letters typed in the
            // same instant (Esc and then a new command).
            if let egui::Event::Key {
                key: Key::Escape,
                pressed: true,
                ..
            } = e
            {
                if !self.cmdline.buffer.is_empty() {
                    self.cmdline.buffer.clear();
                    escaped = true;
                    used = true;
                } else if cad {
                    self.cad_escape();
                    escaped = true;
                    used = true;
                }
                continue;
            }
            let egui::Event::Text(t) = e else { continue };
            for c in t.chars() {
                if c == ' ' {
                    continue;
                }
                let to_buffer = if cad || !self.cmdline.buffer.is_empty() || matches!(c, '@' | '#')
                {
                    true
                } else if drawing_bim {
                    // Lengths typed while drawing go to the tool, as always.
                    false
                } else if self.keys_autocad {
                    true
                } else {
                    // ArchiCAD keys: the tool letters act at once.
                    Tool::from_key(&c.to_string()).is_none()
                        && !c.eq_ignore_ascii_case(&'n')
                        && !c.is_ascii_digit()
                };
                if to_buffer {
                    self.cmdline.buffer.push(c);
                    used = true;
                }
            }
        }
        let cad = self.tool == Tool::Cad;
        let has_text = !self.cmdline.buffer.is_empty();
        if escaped {
            return true;
        }
        if escape {
            if has_text {
                self.cmdline.buffer.clear();
                return true;
            }
            if cad {
                self.cad_escape();
                return true;
            }
            return used;
        }
        if backspace && has_text {
            self.cmdline.buffer.pop();
            return true;
        }
        if enter || space {
            if has_text {
                self.submit_command_line();
                return true;
            }
            if cad {
                self.cad_feed(Input::Enter);
                return true;
            }
            // Enter on an empty line with nothing going on repeats the last
            // command, as in AutoCAD.
            if enter && self.tool == Tool::Arrow && self.drag.is_none() {
                if let Some(last) = self.cmdline.last {
                    self.start_command(last);
                    return true;
                }
            }
        }
        used || (cad && (enter || space || backspace))
    }

    /// A click on the plan while a command runs.
    pub(super) fn cad_clicks(
        &mut self,
        ctx: &egui::Context,
        response: &egui::Response,
        rect: Rect,
    ) {
        let Some(want) = self.cad_want() else {
            return;
        };
        if response.clicked_by(PointerButton::Secondary) {
            self.cad_feed(Input::Enter);
            return;
        }
        match want {
            Want::Point | Want::Value => {
                if response.clicked_by(PointerButton::Primary) {
                    if let Some(c) = self.cursor {
                        self.cad_feed(Input::Point(c.point));
                    }
                }
            }
            Want::Pick => {
                if response.clicked_by(PointerButton::Primary) {
                    let Some(pos) = response.interact_pointer_pos() else {
                        return;
                    };
                    let raw = self.view.to_model(rect, pos);
                    let reach = self.view.to_metres(REACH);
                    match self.pick_here(raw, reach) {
                        Some(e) => {
                            let shift = ctx.input(|i| i.modifiers.shift);
                            if shift {
                                self.cad_feed(Input::Option("INVERT"));
                            }
                            self.cad_feed(Input::Pick(e, raw));
                            if shift && self.cad.is_some() {
                                self.cad_feed(Input::Option("INVERT"));
                            }
                        }
                        None => self.cmdline.log("No hay nada ahí.", true),
                    }
                }
            }
            Want::Select => self.select_clicks(ctx, response, rect),
            Want::Text => {}
        }
    }

    /// Selecting objects for a command, as in AutoCAD: a click adds what is
    /// under the cursor (Shift+click takes it out), a window from left to
    /// right takes what is inside it, from right to left also what it
    /// touches.
    fn select_clicks(&mut self, ctx: &egui::Context, response: &egui::Response, rect: Rect) {
        let shift = ctx.input(|i| i.modifiers.shift);
        if response.clicked_by(PointerButton::Primary) {
            if let Some(e) = self.hover {
                if shift {
                    self.ed.selection.remove(&e);
                } else {
                    self.ed.selection.insert(e);
                }
            }
        }
        if response.drag_started_by(PointerButton::Primary) {
            self.window_start = ctx
                .input(|i| i.pointer.press_origin())
                .map(|p| self.view.to_model(rect, p));
        }
        if response.drag_stopped() {
            if let (Some(a), Some(c)) = (self.window_start.take(), self.cursor) {
                let found = self.window_here(a, c.point);
                for e in found {
                    if shift {
                        self.ed.selection.remove(&e);
                    } else {
                        self.ed.selection.insert(e);
                    }
                }
            }
        }
    }

    /// The point the cursor gives to the running command: snapped (with
    /// AutoCAD's object snaps), orthogonal (F8) or tracked from the last
    /// point.
    pub(super) fn cad_cursor(&mut self, ctx: &egui::Context, raw: Point2, reach: f64) {
        let want = self.cad_want();
        if matches!(want, Some(Want::Pick) | Some(Want::Select)) {
            self.cursor = Some(Snap::at(raw, SnapKind::Free));
            self.hover = if want == Some(Want::Select) && self.window_start.is_none() {
                self.pick_here(raw, reach)
            } else {
                None
            };
            return;
        }
        let base = self.cad.as_ref().and_then(|c| c.base());
        let extra: Vec<Point2> = base.into_iter().collect();
        let mut s = match self.mode {
            ViewMode::Sheet(sheet) => crate::snap::snap_sheet(
                self.ed.model(),
                sheet,
                raw,
                reach,
                &extra,
                self.snap_filter,
                base,
            ),
            _ => crate::snap::snap_cad(
                &self.ed,
                self.ed.level,
                raw,
                reach,
                &extra,
                &BTreeSet::new(),
                self.snap_filter,
                &self.other_walls,
                base,
            ),
        };
        if let Some(b) = base {
            if self.cad_settings.ortho && matches!(s.kind, SnapKind::Free | SnapKind::OnEdge) {
                s = Snap::at(cad::ortho(b, s.point), SnapKind::Constrained);
            } else if ctx.input(|i| i.modifiers.shift) {
                s = constrain(b, s);
            } else if self.snap_filter.tracking {
                s = track(b, s, self.view.to_metres(TRACK_REACH));
            }
        }
        self.cursor = Some(s);
        self.hover = None;
    }

    /// What the running command draws: its preview following the cursor
    /// and the objects it has picked.
    pub(super) fn draw_cad(&self, painter: &egui::Painter, rect: Rect) {
        let Some(cmd) = &self.cad else {
            return;
        };
        let k = self.active_k();
        let stroke = Stroke::new(1.5, ACCENT);
        for e in cmd.picked() {
            for path in self.element_paths(e) {
                let pts = self.polygon(rect, &path);
                painter.extend(Shape::dashed_line(&pts, Stroke::new(2.0, ACCENT), 6.0, 3.0));
            }
        }
        let cursor = self.cursor.map(|c| c.point).unwrap_or_default();
        let cx = Ctx {
            geo: &self.ed as &dyn crate::geo::Geo,
            view: self.active_view(),
            selection: &self.ed.selection,
            k,
            cursor,
        };
        for shape in cmd.preview(&cx, &self.cad_settings) {
            if let archisim_core::Shape::Text(t) = &shape {
                // Texts as they will be written.
                for st in archisim_draft::font::text_strokes(t, k) {
                    painter.add(Shape::line(
                        self.polygon(rect, &st),
                        Stroke::new(1.2, ACCENT),
                    ));
                }
                continue;
            }
            for path in crate::pick::shape_paths(&shape, k) {
                painter.add(Shape::line(self.polygon(rect, &path), stroke));
            }
        }
        // What is being typed, beside the cursor (AutoCAD's dynamic input).
        if !self.cmdline.buffer.is_empty() {
            if let Some(c) = self.cursor {
                self.draw_label(painter, rect, c.point, format!("{}▏", self.cmdline.buffer));
            }
        }
    }

    /// The outline of an element as polylines (drafting elements as
    /// drawn, others by their outline closed).
    pub(super) fn element_paths(&self, e: Element) -> Vec<Vec<Point2>> {
        match e {
            Element::Draft(id) => crate::pick::draft_paths(self.ed.model(), id),
            other => crate::pick::outline(&self.ed, other)
                .map(|mut p| {
                    if let Some(f) = p.first().copied() {
                        p.push(f);
                    }
                    vec![p]
                })
                .unwrap_or_default(),
        }
    }

    /// The command line, under the drawing: the last lines said and the
    /// prompt with what is being typed. Options can be clicked.
    pub(super) fn command_line(&mut self, ui: &mut Ui) {
        let p = theme::Palette::current(ui.ctx());
        let mono = FontId::monospace(12.5);
        let rows = 2;
        let start = self.cmdline.history.len().saturating_sub(rows);
        for i in 0..rows {
            let (text, error) = self
                .cmdline
                .history
                .get(start + i)
                .cloned()
                .unwrap_or_default();
            let color = if error { p.bad } else { p.muted };
            ui.add(
                egui::Label::new(egui::RichText::new(text).font(mono.clone()).color(color))
                    .truncate(),
            );
        }
        let prompt = self.cad_prompt();
        let mut clicked: Option<&'static str> = None;
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 4.0;
            let head = match &prompt {
                Some(pr) => {
                    let name = self.cad.as_ref().map(|c| c.name()).unwrap_or_default();
                    format!("{name} {}", pr.text)
                }
                None => "Comando:".to_string(),
            };
            ui.label(egui::RichText::new(head).font(mono.clone()).color(p.text));
            if let Some(pr) = &prompt {
                if !pr.options.is_empty() {
                    ui.label(egui::RichText::new("o").font(mono.clone()).color(p.muted));
                    for o in &pr.options {
                        let b = egui::Button::new(
                            egui::RichText::new(o.label).font(FontId::monospace(11.5)).color(p.accent),
                        )
                        .small()
                        .frame(false);
                        if ui.add(b).on_hover_text(format!("Escribe {} e Intro", o.key)).clicked() {
                            clicked = Some(o.key);
                        }
                    }
                }
                if let Some(d) = &pr.default {
                    ui.label(egui::RichText::new(format!("<{d}>")).font(mono.clone()).color(p.muted));
                }
            }
            let caret = if (ui.input(|i| i.time) * 2.0) as i64 % 2 == 0 { "▏" } else { " " };
            ui.label(
                egui::RichText::new(format!("{}{caret}", self.cmdline.buffer))
                    .font(mono.clone())
                    .color(p.accent),
            );
            ui.ctx().request_repaint_after(std::time::Duration::from_millis(500));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let label = if self.keys_autocad {
                    "Teclas: AutoCAD"
                } else {
                    "Teclas: ArchiCAD"
                };
                let hint = if self.keys_autocad {
                    "Escribe la orden y pulsa Intro o Espacio (L, PL, M de muro…). Clic: teclas de ArchiCAD (M, F, P, V, C y N al momento)"
                } else {
                    "M, F, P, V, C y N cambian de herramienta al momento. Clic: escribir órdenes como en AutoCAD (orden + Intro)"
                };
                if ui.small_button(label).on_hover_text(hint).clicked() {
                    self.keys_autocad = !self.keys_autocad;
                }
                let ortho = if self.cad_settings.ortho { "ORTO ●" } else { "ORTO" };
                if ui
                    .small_button(ortho)
                    .on_hover_text("Modo orto (F8): líneas solo horizontales y verticales")
                    .clicked()
                {
                    self.toggle_ortho();
                }
            });
        });
        if let Some(k) = clicked {
            self.cmdline.log(k.to_string(), false);
            self.cad_feed(Input::Option(k));
        }
    }

    /// The box to write a text in, when the command asks for one.
    pub(super) fn text_entry(&mut self, ctx: &egui::Context) {
        if self.cad_want() != Some(Want::Text) {
            self.text_draft = None;
            return;
        }
        let draft = self.text_draft.get_or_insert_with(String::new);
        let mut accept = false;
        let mut cancel = false;
        super::widgets::dialog("Texto")
            .collapsible(false)
            .resizable(true)
            .default_width(360.0)
            .anchor(Align2::CENTER_CENTER, Vec2::ZERO)
            .show(ctx, |ui| {
                ui.label("Escribe el texto (Intro: nueva línea; Ctrl+Intro: aceptar).");
                let edit = TextEdit::multiline(draft)
                    .desired_rows(4)
                    .desired_width(f32::INFINITY)
                    .id(Id::unique("cad-text"));
                let r = ui.add(edit);
                r.request_focus();
                if ui.input(|i| i.key_pressed(Key::Enter) && i.modifiers.command) {
                    accept = true;
                }
                if ui.input(|i| i.key_pressed(Key::Escape)) {
                    cancel = true;
                }
                ui.horizontal(|ui| {
                    if ui.button("Aceptar").clicked() {
                        accept = true;
                    }
                    if ui.button("Cancelar").clicked() {
                        cancel = true;
                    }
                });
            });
        if accept {
            let text = self.text_draft.take().unwrap_or_default();
            // Ctrl+Enter may have added a line break.
            let text = text.trim_end_matches('\n').to_string();
            self.cmdline
                .log(format!("Texto: {}", text.replace('\n', " / ")), false);
            self.cad_feed(Input::Value(text));
        } else if cancel {
            self.text_draft = None;
            self.cad_escape();
        }
    }
}
