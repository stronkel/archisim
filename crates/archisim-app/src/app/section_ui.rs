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

/// How many sections seen before keep their drawing.
const STASHED: usize = 4;

/// The drawing of the section shown, worked out once per revision of the
/// model, and its picture on screen.
pub struct SectionCache {
    revision: u64,
    section: SectionId,
    phase_view: archisim_plan::phases::PhaseView,
    drawing: SectionDrawing,
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
            drawing,
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
        if let Some((old, tex)) = self
            .section_cache
            .as_ref()
            .and_then(|c| c.picture.as_ref())
            .filter(|p| p.0 != key)
        {
            if moving && old.same_but_view(&key) {
                self.paint_moved(painter, rect, tex, old.lo, old.hi);
                return;
            }
        }
        let shown = self.ed.shown();
        let model: &archisim_core::Model = &shown;
        let Some(cache) = self.section_cache.as_mut() else {
            return;
        };
        if cache.picture.as_ref().map(|p| &p.0) != Some(&key) {
            self.view_moved_at = None;
            let px_per_m = size[0] as f64 / (hi.x - lo.x).max(1e-9);
            let options = archisim_plan::PlanOptions {
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
            };
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
