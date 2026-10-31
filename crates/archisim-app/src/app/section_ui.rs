//! Sections and elevations: the section tool in plan, their views in tabs
//! (drawn by `archisim_plan::section` from the same solids as the 3D and
//! the IFC) and their properties.

use super::widgets::SwitchExt as _;
use archisim_core::{SectionId, SectionKind};
use archisim_plan::section::{render_section, section_drawing, SectionDrawing};
use eframe::egui::{self, Id, Ui};

use super::icons::Icon;
use super::props::{self, group, row, text_row};
use super::*;

/// The section tool: its first point while drawing, and how deep the next
/// section sees.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SectionTool {
    pub anchor: Option<Point2>,
    pub depth: f64,
}

impl Default for SectionTool {
    fn default() -> Self {
        Self {
            anchor: None,
            depth: 8.0,
        }
    }
}

/// What the picture of a section on screen depends on.
#[derive(Debug, Clone, PartialEq)]
pub struct SectionKey {
    lo: Point2,
    hi: Point2,
    size: [usize; 2],
    scale: f64,
    thin_lines: bool,
    focus: archisim_core::DisciplineFocus,
    hidden: std::collections::BTreeSet<archisim_core::ElementDiscipline>,
}

impl SectionKey {
    /// Whether `other` is the same picture of another region or size (the
    /// view has only moved or zoomed). A picture marked to be drawn again
    /// (size 0, after the model changed) is not.
    fn same_but_view(&self, other: &SectionKey) -> bool {
        self.size != [0, 0]
            && *other
                == SectionKey {
                    lo: other.lo,
                    hi: other.hi,
                    size: other.size,
                    ..self.clone()
                }
    }
}

/// How a section is drawn for the picture of `key`.
fn section_options(key: &SectionKey) -> archisim_plan::PlanOptions {
    let (lo, hi) = (key.lo, key.hi);
    let px_per_m = key.size[0] as f64 / (hi.x - lo.x).max(1e-9);
    archisim_plan::PlanOptions {
        viewport: Some((lo, hi)),
        scale: key.scale,
        thin_lines: key.thin_lines,
        px_per_mm: Some(px_per_m * key.scale / 1000.0),
        style: archisim_core::ViewStyle {
            focus: key.focus,
            hidden_disciplines: key.hidden.clone(),
            ..Default::default()
        },
        ..archisim_plan::PlanOptions::default()
    }
}

/// How many sections seen before keep their drawing.
const STASHED: usize = 4;

/// The drawing of the section shown, worked out once per revision of the
/// model, and its picture on screen.
pub struct SectionCache {
    revision: u64,
    section: SectionId,
    phase_view: archisim_plan::phases::PhaseView,
    drawing: std::sync::Arc<SectionDrawing>,
    picture: Option<(SectionKey, TextureHandle)>,
}

/// Depths offered by the tool (metres).
const DEPTHS: [f64; 5] = [2.0, 4.0, 8.0, 15.0, 30.0];

impl ArchiSimApp {
    /// The section of the active tab, if it shows one.
    pub(super) fn active_section(&self) -> Option<SectionId> {
        match self.tabs.get(self.active_tab).map(|t| t.kind) {
            Some(TabKind::Section(id)) => Some(id),
            _ => None,
        }
    }

    /// A click of the section tool.
    pub(super) fn section_click(&mut self, p: Point2) {
        match self.section_tool.anchor {
            None => self.section_tool.anchor = Some(p),
            Some(a) if a.approx_eq(p) => {}
            Some(a) => {
                self.section_tool.anchor = None;
                match self.ed.add_section(a, p, self.section_tool.depth) {
                    Ok(id) => {
                        self.ed.selection = [Element::Section(id)].into_iter().collect();
                        let title = self
                            .ed
                            .model()
                            .section(id)
                            .map(|s| s.title())
                            .unwrap_or_default();
                        self.say(format!(
                            "{title} creada. Doble clic en su marca (o «Abrir vista») para verla."
                        ));
                    }
                    Err(e) => self.complain(format!("No se puede crear la sección: {e}.")),
                }
            }
        }
    }

    /// Makes the four elevations around the building.
    pub(super) fn create_elevations(&mut self) {
        match self.ed.add_elevations() {
            Ok(ids) if ids.is_empty() => {
                if self
                    .ed
                    .model()
                    .sections()
                    .any(|s| s.kind == SectionKind::Elevation)
                {
                    self.say("Los cuatro alzados ya existen.");
                } else {
                    self.complain("No hay nada que ver en alzado: dibuja algún elemento antes.");
                }
            }
            Ok(ids) => {
                self.say(format!(
                    "{} alzados creados, con su marca en planta. Ábrelos desde el navegador.",
                    ids.len()
                ));
            }
            Err(e) => self.complain(format!("No se pueden crear los alzados: {e}.")),
        }
    }

    /// The line being drawn, with an arrow to the side it will look at.
    pub(super) fn draw_section_preview(&self, painter: &egui::Painter, rect: Rect) {
        let (Tool::Section, Some(c), Some(a)) = (self.tool, self.cursor, self.section_tool.anchor)
        else {
            return;
        };
        let (pa, pb) = (
            self.view.to_screen(rect, a),
            self.view.to_screen(rect, c.point),
        );
        dashed(painter, pa, pb, ACCENT);
        if let Some(u) = (c.point - a).normalized() {
            let look = u.perp_left();
            let mid = a + (c.point - a) * 0.5;
            let m = self.view.to_screen(rect, mid);
            let tip = self
                .view
                .to_screen(rect, mid + look * self.view.to_metres(28.0));
            painter.arrow(m, tip - m, Stroke::new(2.0, ACCENT));
        }
        let d = c.point - a;
        self.draw_label(
            painter,
            rect,
            c.point,
            format!(
                "Sección {}   L {}",
                self.ed.model().next_section_name(),
                crate::units::fmt_len(d.length())
            ),
        );
    }

    // ---- The view --------------------------------------------------------

    /// Keeps the drawing of the section shown up to date.
    fn refresh_section(&mut self, id: SectionId) {
        let revision = self.ed.revision();
        let phase_view = self.phase_view;
        let fresh = self.section_cache.as_ref().is_some_and(|c| {
            c.revision == revision && c.section == id && c.phase_view == phase_view
        });
        if fresh {
            return;
        }
        // A section seen before, unchanged since: back to it as it was.
        let up_to_date = |c: &SectionCache| {
            c.revision == revision && c.section == id && c.phase_view == phase_view
        };
        if let Some(i) = self.section_stash.iter().position(up_to_date) {
            let back = self.section_stash.remove(i);
            if let Some(old) = self.section_cache.replace(back) {
                self.stash_section(old);
            }
            return;
        }
        let shown = self.ed.shown();
        let model: &archisim_core::Model = &shown;
        let Some(sec) = model.section(id) else {
            return;
        };
        let phased = archisim_plan::phases::phase_model(model, phase_view);
        let drawing = section_drawing(phased.as_ref().unwrap_or(model), sec);
        // The texture of the one shown before is reused when it was this
        // section; another section is put aside to come back to.
        let picture = match self.section_cache.take() {
            Some(c) if c.section == id => c.picture,
            Some(c) => {
                self.stash_section(c);
                None
            }
            None => None,
        };
        self.section_cache = Some(SectionCache {
            revision,
            section: id,
            phase_view,
            drawing: std::sync::Arc::new(drawing),
            picture: picture.map(|(mut k, t)| {
                // Same texture, new picture.
                k.size = [0, 0];
                (k, t)
            }),
        });
    }

    /// Keeps the drawing of a section no longer shown, to go back to it,
    /// with the few most recent ones (out of date ones are dropped).
    fn stash_section(&mut self, c: SectionCache) {
        let revision = self.ed.revision();
        self.section_stash
            .retain(|s| s.section != c.section && s.revision == revision);
        if c.revision == revision {
            self.section_stash.push(c);
        }
        if self.section_stash.len() > STASHED {
            self.section_stash.remove(0);
        }
    }

    pub(super) fn section_view(&mut self, ui: &mut Ui) {
        let (rect, response) = ui.allocate_exact_size(ui.available_size(), Sense::click_and_drag());
        let ctx = ui.ctx().clone();
        let painter = ui.painter_at(rect);
        self.canvas = rect;
        self.cursor = None;
        painter.rect_filled(rect, CornerRadius::ZERO, PAPER);
        let Some(id) = self.active_section() else {
            return;
        };
        self.refresh_section(id);
        let Some(cache) = self.section_cache.as_ref() else {
            return;
        };
        if self.fit_requested && rect.width() > 50.0 {
            self.fit_requested = false;
            let d = &cache.drawing;
            // Room for the level names on the right.
            let k = self.plan_scale / 1000.0;
            let hi = Point2::new(d.hi.x + 1.0 + 30.0 * k, d.hi.y);
            self.view.fit(rect, d.lo, hi, 40.0);
        }
        self.navigate(&ctx, &response, rect);
        self.draw_section(&ctx, &painter, rect);
        if response.hovered() {
            if let Some(pos) = response.hover_pos() {
                let p = self.view.to_model(rect, pos);
                self.section_hover = Some(p);
            }
        } else {
            self.section_hover = None;
        }
        if self.detail_pick.is_some_and(|(s, _)| s == id) {
            self.detail_rectangle(&ctx, &response, &painter, rect, id);
            return;
        }
        // Selection, as in plan: click an element cut or seen (Shift adds
        // or takes it), click nothing to clear.
        let under = self
            .section_hover
            .and_then(|p| self.section_cache.as_ref()?.drawing.pick(p))
            .map(crate::pick::of_solid);
        if response.clicked_by(egui::PointerButton::Primary) {
            let shift = ctx.input(|i| i.modifiers.shift || i.modifiers.command);
            match (under, shift) {
                (Some(e), true) => {
                    if !self.ed.selection.remove(&e) {
                        self.ed.selection.insert(e);
                    }
                }
                (Some(e), false) => {
                    self.ed.selection = [e].into_iter().collect();
                }
                (None, false) => self.ed.selection.clear(),
                (None, true) => {}
            }
        }
        self.draw_section_selection(&painter, rect, under);
        if under.is_some() {
            ctx.set_cursor_icon(egui::CursorIcon::PointingHand);
        }
        self.section_move(&ctx, &response, &painter, rect, id, under);
    }

    /// The rectangle of a detail (E17): two clicks give its corners and a
    /// view for drawing is made with that piece of the section under it,
    /// and opened; Esc leaves it.
    fn detail_rectangle(
        &mut self,
        ctx: &egui::Context,
        response: &egui::Response,
        painter: &egui::Painter,
        rect: Rect,
        id: SectionId,
    ) {
        ctx.set_cursor_icon(egui::CursorIcon::Crosshair);
        if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            self.detail_pick = None;
            return;
        }
        let first = self.detail_pick.and_then(|(_, a)| a);
        if let (Some(a), Some(b)) = (first, self.section_hover) {
            let quad = [a, Point2::new(b.x, a.y), b, Point2::new(a.x, b.y)];
            painter.add(Shape::closed_line(
                self.polygon(rect, &quad),
                Stroke::new(1.5, ACCENT),
            ));
            ctx.request_repaint();
        }
        if !response.clicked_by(egui::PointerButton::Primary) {
            return;
        }
        let Some(p) = self.section_hover else {
            return;
        };
        let Some(a) = first else {
            self.detail_pick = Some((id, Some(p)));
            return;
        };
        if (p.x - a.x).abs() < 0.01 || (p.y - a.y).abs() < 0.01 {
            return;
        }
        self.detail_pick = None;
        let scale = if self.plan_scale > 20.0 {
            10.0
        } else {
            self.plan_scale.min(10.0)
        };
        let r = self.ed.transact("Vista de detalle", |m| {
            m.add_section_detail(id, a, p, scale)
        });
        match r {
            Ok(v) => {
                self.open_view(super::tabs::TabKind::Drafting(v));
                self.say("Detalle creado: la zona de la sección queda debajo, en gris; dibuja encima con las órdenes de AutoCAD.");
            }
            Err(e) => self.complain(format!("No se puede: {e}.")),
        }
    }

    /// Moving objects, doors and windows in a section or elevation (E15):
    /// dragged along the section and up or down, a corner, the middle of a
    /// side or the centre of their box pulled to those of what is drawn
    /// and to the levels; on release, objects move and rise, doors and
    /// windows slide along their wall and change their sill.
    fn section_move(
        &mut self,
        ctx: &egui::Context,
        response: &egui::Response,
        painter: &egui::Painter,
        rect: Rect,
        id: SectionId,
        under: Option<Element>,
    ) {
        let movable = |e: &Element| matches!(e, Element::Object(_) | Element::Opening(_));
        if response.drag_started_by(egui::PointerButton::Primary) {
            if let (Some(p), Some(e)) = (self.section_hover, under.filter(movable)) {
                if !self.ed.selection.contains(&e) {
                    self.ed.selection = [e].into_iter().collect();
                }
                let elements: Vec<Element> =
                    self.ed.selection.iter().copied().filter(movable).collect();
                if let Some(cache) = self.section_cache.as_ref() {
                    let owners: Vec<_> = elements
                        .iter()
                        .filter_map(|e| crate::pick::solid_of(*e))
                        .collect();
                    let mut lo = Point2::new(f64::MAX, f64::MAX);
                    let mut hi = Point2::new(f64::MIN, f64::MIN);
                    for o in &owners {
                        let (cut, faces) = cache.drawing.shape_of(*o);
                        for q in cut.iter().chain(&faces).flatten() {
                            lo = Point2::new(lo.x.min(q.x), lo.y.min(q.y));
                            hi = Point2::new(hi.x.max(q.x), hi.y.max(q.y));
                        }
                    }
                    if lo.x <= hi.x {
                        let c = (lo + hi) * 0.5;
                        let own = vec![
                            lo,
                            Point2::new(hi.x, lo.y),
                            hi,
                            Point2::new(lo.x, hi.y),
                            Point2::new(c.x, lo.y),
                            Point2::new(c.x, hi.y),
                            Point2::new(lo.x, c.y),
                            Point2::new(hi.x, c.y),
                            c,
                        ];
                        self.section_drag = Some(SectionDrag {
                            section: id,
                            start: p,
                            delta: Point2::new(0.0, 0.0),
                            elements,
                            own,
                            targets: cache.drawing.points_except(&owners),
                            levels: cache.drawing.level_heights(),
                            snapped: None,
                        });
                    }
                }
            }
        }
        let Some(mut drag) = self.section_drag.take() else {
            return;
        };
        if drag.section != id {
            return;
        }
        if response.dragged_by(egui::PointerButton::Primary) {
            if let Some(p) = self.section_hover {
                let raw = p - drag.start;
                let tol = 10.0 / self.view.zoom.max(1e-9);
                drag.delta = raw;
                drag.snapped = None;
                // The nearest pair of one of its points and a point drawn.
                let mut best: Option<(f64, Point2, Point2)> = None;
                for a in &drag.own {
                    let moved = *a + raw;
                    for b in &drag.targets {
                        let d = moved.distance(*b);
                        if d < tol && best.is_none_or(|(x, _, _)| d < x) {
                            best = Some((d, *b - *a, *b));
                        }
                    }
                }
                if let Some((_, d, at)) = best {
                    drag.delta = d;
                    drag.snapped = Some(at);
                } else {
                    // Or its top or bottom on a level.
                    for a in &drag.own[..4] {
                        for z in &drag.levels {
                            if (a.y + raw.y - z).abs() < tol {
                                drag.delta = Point2::new(raw.x, z - a.y);
                            }
                        }
                    }
                }
            }
            // Its outline where it would go.
            let [lo, _, hi, ..] = [drag.own[0], drag.own[1], drag.own[2]];
            let quad = [
                lo + drag.delta,
                Point2::new(hi.x, lo.y) + drag.delta,
                hi + drag.delta,
                Point2::new(lo.x, hi.y) + drag.delta,
            ];
            painter.add(Shape::closed_line(
                self.polygon(rect, &quad),
                Stroke::new(1.5, ACCENT),
            ));
            if let Some(at) = drag.snapped {
                let c = self.view.to_screen(rect, at);
                painter.circle_stroke(c, 5.0, Stroke::new(1.5, ACCENT));
            }
            ctx.request_repaint();
        }
        if response.drag_stopped() {
            let d = drag.delta;
            if d.length() > 1e-6 {
                let along = self.ed.model().section(id).and_then(|s| s.along());
                let elements = drag.elements.clone();
                let r = self.ed.transact("Mover en la sección", |m| {
                    let Some(u) = along else {
                        return Ok(());
                    };
                    for e in &elements {
                        match e {
                            Element::Object(o) => m.edit_object(*o, |x| {
                                x.position = x.position + u * d.x;
                                x.base_offset += d.y;
                            })?,
                            Element::Opening(o) => {
                                let Some(op) = m.opening(*o).cloned() else {
                                    continue;
                                };
                                let w = m
                                    .wall(op.wall)
                                    .and_then(|w| (w.end - w.start).normalized())
                                    .unwrap_or(u);
                                let k = w.dot(u);
                                let along = if k.abs() > 0.3 {
                                    op.along + d.x * k
                                } else {
                                    op.along
                                };
                                m.move_opening(*o, along, op.sill + d.y)?;
                            }
                            _ => {}
                        }
                    }
                    Ok(())
                });
                self.report(r.map_err(|e| e.to_string()), "Movido.");
            }
            return;
        }
        self.section_drag = Some(drag);
    }

    /// The selected elements over the section drawing, and the one under
    /// the cursor: their cut outlined, their faces seen tinted.
    fn draw_section_selection(&self, painter: &egui::Painter, rect: Rect, under: Option<Element>) {
        let Some(cache) = self.section_cache.as_ref() else {
            return;
        };
        let mut shown: Vec<(Element, bool)> =
            self.ed.selection.iter().map(|e| (*e, true)).collect();
        if let Some(e) = under.filter(|e| !self.ed.selection.contains(e)) {
            shown.push((e, false));
        }
        for (e, selected) in shown {
            let Some(owner) = crate::pick::solid_of(e) else {
                continue;
            };
            let (cut, faces) = cache.drawing.shape_of(owner);
            let alpha = if selected { 110 } else { 60 };
            let tint = Color32::from_rgba_unmultiplied(ACCENT.r(), ACCENT.g(), ACCENT.b(), alpha);
            let mut mesh = egui::Mesh::default();
            for poly in &faces {
                let pts = self.polygon(rect, poly);
                let base = mesh.vertices.len() as u32;
                for p in &pts {
                    mesh.colored_vertex(*p, tint);
                }
                for i in 1..pts.len().saturating_sub(1) as u32 {
                    mesh.add_triangle(base, base + i, base + i + 1);
                }
            }
            painter.add(Shape::mesh(mesh));
            let width = if selected { 2.5 } else { 1.5 };
            for l in &cut {
                painter.add(Shape::closed_line(
                    self.polygon(rect, l),
                    Stroke::new(width, ACCENT),
                ));
            }
        }
    }

    fn draw_section(&mut self, ctx: &egui::Context, painter: &egui::Painter, rect: Rect) {
        let ppp = ctx.pixels_per_point();
        let size = [
            (rect.width() * ppp).round().max(1.0) as usize,
            (rect.height() * ppp).round().max(1.0) as usize,
        ];
        let (lo, hi) = self.view.visible(rect);
        let key = SectionKey {
            lo,
            hi,
            size,
            scale: self.plan_scale,
            thin_lines: self.thin_lines,
            focus: self.screen_focus(),
            hidden: self.hidden_disciplines.clone(),
        };
        let moving = self.view_moving(ctx);
        let only_moved = self
            .section_cache
            .as_ref()
            .and_then(|c| c.picture.as_ref())
            .is_some_and(|p| p.0 != key && p.0.same_but_view(&key));
        if only_moved {
            // While it moves, and then while its new picture is made aside,
            // the old one moved.
            let ready = match &self.section_pending {
                Some(p) if p.key == key => p.take(),
                _ if moving => None,
                _ => {
                    let model = self.shared_model();
                    let Some(drawing) = self.section_cache.as_ref().map(|c| c.drawing.clone())
                    else {
                        return;
                    };
                    let options = section_options(&key);
                    let size = key.size;
                    self.section_pending =
                        Some(super::background::Pending::start(key.clone(), move || {
                            let svg = render_section(&model, &drawing, &options);
                            crate::plan_image::rasterize_svg(&svg, size)
                        }));
                    None
                }
            };
            match ready {
                Some(image) => {
                    self.section_pending = None;
                    self.view_moved_at = None;
                    if let Some((k, tex)) =
                        self.section_cache.as_mut().and_then(|c| c.picture.as_mut())
                    {
                        tex.set(image, TextureOptions::LINEAR);
                        *k = key.clone();
                    }
                }
                None => {
                    if let Some((old, tex)) =
                        self.section_cache.as_ref().and_then(|c| c.picture.as_ref())
                    {
                        self.paint_moved(painter, rect, tex, old.lo, old.hi);
                    }
                    ctx.request_repaint_after(std::time::Duration::from_millis(30));
                    return;
                }
            }
        }
        let shown = self.ed.shown();
        let model: &archisim_core::Model = &shown;
        let Some(cache) = self.section_cache.as_mut() else {
            return;
        };
        if cache.picture.as_ref().map(|p| &p.0) != Some(&key) {
            self.view_moved_at = None;
            self.section_pending = None;
            let options = section_options(&key);
            let svg = render_section(model, &cache.drawing, &options);
            let image = crate::plan_image::rasterize_svg(&svg, size);
            match &mut cache.picture {
                Some((k, tex)) => {
                    tex.set(image, TextureOptions::LINEAR);
                    *k = key;
                }
                None => {
                    let tex = ctx.load_texture("section", image, TextureOptions::LINEAR);
                    cache.picture = Some((key, tex));
                }
            }
        }
        if let Some((_, tex)) = &cache.picture {
            painter.image(
                tex.id(),
                rect,
                Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)),
                Color32::WHITE,
            );
        }
    }

    // ---- Properties ------------------------------------------------------

    pub(super) fn section_tool_properties(&mut self, ui: &mut Ui) {
        props::title(ui, "Herramienta Sección · lo que vas a crear");
        let next = self.ed.model().next_section_name();
        group(ui, "tool-section", "Sección", |ui| {
            text_row(
                ui,
                "Nombre",
                "La siguiente letra libre",
                &format!("Sección {next}"),
            );
            row(
                ui,
                "Profundidad",
                "Hasta dónde ve la sección más allá de su línea",
                |ui, w| {
                    let opts: Vec<(u32, String)> = DEPTHS
                        .iter()
                        .map(|d| ((*d * 1000.0) as u32, format!("{} m", d)))
                        .collect();
                    let opts: Vec<(u32, &str)> =
                        opts.iter().map(|(v, n)| (*v, n.as_str())).collect();
                    let current = (self.section_tool.depth * 1000.0) as u32;
                    let v = props::choice(ui, "tool-section-depth", w, current, &opts);
                    self.section_tool.depth = v as f64 / 1000.0;
                },
            );
        });
        ui.add_space(6.0);
        props::note(
            ui,
            "Dos clics: inicio y final de la línea. La sección mira hacia la izquierda del trazo (de inicio a fin); para que mire al otro lado, «Invertir» en sus propiedades. Los alzados se crean desde el navegador.",
        );
    }

    pub(super) fn section_properties(&mut self, ui: &mut Ui, id: SectionId) {
        let Some(s) = self.ed.model().section(id).cloned() else {
            return;
        };
        let what = match s.kind {
            SectionKind::Section => "Sección",
            SectionKind::Elevation => "Alzado",
        };
        props::title(ui, &format!("{what} · 1 seleccionado"));
        super::properties::view_card(
            ui,
            match s.kind {
                SectionKind::Section => Icon::Section,
                SectionKind::Elevation => Icon::Elevation,
            },
            &s.title(),
            "Vista (no se exporta al IFC)",
        );
        group(ui, "section-data", "Datos", |ui| {
            row(
                ui,
                "Nombre",
                "Único entre las secciones (o los alzados)",
                |ui, w| {
                    if let Some(v) =
                        text_edit_committed(ui, Id::unique(("section-name", id)), &s.name, w - 8.0)
                    {
                        let r = self.ed.rename_section(id, &v);
                        self.report(r, "Nombre cambiado.");
                    }
                },
            );
            row(
                ui,
                "Profundidad (mm)",
                "Hasta dónde ve más allá de su línea",
                |ui, w| {
                    if let Some(v) =
                        length_field(ui, Id::unique(("section-depth", id)), s.depth, w - 8.0)
                    {
                        let r = self
                            .ed
                            .edit_section("Profundidad de la sección", id, |s| s.depth = v);
                        self.report(r, "Profundidad cambiada.");
                    }
                },
            );
            text_row(
                ui,
                "Longitud",
                "",
                &crate::units::fmt_len(s.start.distance(s.end)),
            );
            row(
                ui,
                "Acotar alturas",
                "Alturas entre plantas a la izquierda y, en una sección, la altura libre de cada local cortado",
                |ui, _| {
                    let mut on = s.heights;
                    if ui.switch(&mut on, "").changed() {
                        let r = self
                            .ed
                            .edit_section("Acotar alturas de la sección", id, |s| s.heights = on);
                        self.report(r, "Sección cambiada.");
                    }
                },
            );
        });
        ui.add_space(6.0);
        ui.horizontal(|ui| {
            if ui
                .button("Abrir vista")
                .on_hover_text("La sección en una pestaña (también: doble clic en su marca)")
                .clicked()
            {
                self.open_view(TabKind::Section(id));
            }
            if ui
                .button("Invertir")
                .on_hover_text("Que mire hacia el otro lado")
                .clicked()
            {
                let r = self.ed.flip_section(id);
                self.report(r, "La sección mira ahora al otro lado.");
            }
        });
    }

    pub(super) fn section_view_properties(&mut self, ui: &mut Ui) {
        let Some(s) = self
            .active_section()
            .and_then(|id| self.ed.model().section(id).cloned())
        else {
            return;
        };
        props::title(ui, &format!("Vista · {}", s.title()));
        super::properties::view_card(
            ui,
            match s.kind {
                SectionKind::Section => Icon::Section,
                SectionKind::Elevation => Icon::Elevation,
            },
            &s.title(),
            match s.kind {
                SectionKind::Section => "Sección: lo cortado con su trama, lo de detrás en vista",
                SectionKind::Elevation => "Alzado: el edificio visto desde fuera",
            },
        );
        group(ui, "section-view-graphics", "Gráficos", |ui| {
            row(
                ui,
                "Escala",
                "Grosores de línea, tramas y textos",
                |ui, w| {
                    let options: Vec<(u32, String)> = super::tabs::SCALES
                        .iter()
                        .map(|s| (*s as u32, format!("1:{s}")))
                        .collect();
                    let opts: Vec<(u32, &str)> =
                        options.iter().map(|(v, n)| (*v, n.as_str())).collect();
                    let v =
                        props::choice(ui, "section-view-scale", w, self.plan_scale as u32, &opts);
                    self.set_scale(v as f64);
                },
            );
            row(ui, "Profundidad (mm)", "Hasta dónde ve", |ui, w| {
                if let Some(v) = length_field(
                    ui,
                    Id::unique(("section-view-depth", s.id)),
                    s.depth,
                    w - 8.0,
                ) {
                    let r = self
                        .ed
                        .edit_section("Profundidad de la sección", s.id, |x| x.depth = v);
                    self.report(r, "Profundidad cambiada.");
                }
            });
        });
        group(ui, "section-view-detail", "Detalle", |ui| {
            let picking = self.detail_pick.is_some_and(|(x, _)| x == s.id);
            let label = if picking {
                "Marca dos esquinas… (Esc)"
            } else {
                "Vista de detalle de una zona"
            };
            if ui
                .add_sized([ui.available_width(), 24.0], egui::Button::new(label).selected(picking))
                .on_hover_text("Marca con dos clics un rectángulo de la sección: se hace una vista de dibujo a 1:10 con esa zona debajo, en gris, para dibujar el detalle encima")
                .clicked()
            {
                self.detail_pick = if picking { None } else { Some((s.id, None)) };
            }
        });
        if let Some(p) = self.section_hover {
            group(ui, "section-view-cursor", "Cursor", |ui| {
                text_row(
                    ui,
                    "Altura",
                    "Del cursor, respecto al ±0,00",
                    &fmt_level(p.y),
                );
            });
        }
    }
}

/// A move in a section or elevation, while the mouse is held (E15).
#[derive(Debug, Clone)]
pub struct SectionDrag {
    section: SectionId,
    start: Point2,
    delta: Point2,
    elements: Vec<Element>,
    /// The corners, middles of the sides and centre of the box of what
    /// moves, in the drawing.
    own: Vec<Point2>,
    /// The points drawn, to pull to.
    targets: Vec<Point2>,
    levels: Vec<f64>,
    snapped: Option<Point2>,
}
