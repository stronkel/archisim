//! The mini bar on the right of the drawing (W10, as Synaps): a narrow
//! card of a few round buttons to measure, dimension, write a note and
//! move about the view, always at hand, on the side away from the tools.

use eframe::egui::{self, CornerRadius, Id, Pos2, Rect, Sense, Stroke, Vec2};

use super::glyphs::{self, Glyph};
use super::icons::{self, Icon};
use super::toolbox::Pick;
use super::{theme, ArchiSimApp, Tool, ViewMode};

/// The side of a button.
const BUTTON: f32 = 30.0;

#[derive(Clone, Copy)]
enum Mark {
    Glyph(Glyph),
    Icon(Icon),
    Text(&'static str),
}

#[derive(Clone, Copy)]
enum Do {
    Pick(Pick),
    ZoomIn,
    ZoomOut,
    Fit,
}

impl ArchiSimApp {
    /// The mini bar over the right edge of `area` (the view's canvas).
    pub(super) fn mini_bar(&mut self, ctx: &egui::Context, area: Rect) {
        if area.width() < 300.0 || area.height() < 260.0 {
            return;
        }
        let p = theme::Palette::current(ctx);
        let plan = self.mode == ViewMode::Plan;
        let mut items: Vec<(Mark, &str, Do, bool)> = Vec::new();
        if plan {
            items.push((
                Mark::Glyph(Glyph::Tape),
                "Cinta: medir entre dos puntos",
                Do::Pick(Pick::Concept(super::concept_ui::ConceptPick::Tape)),
                self.tool == Tool::Tape,
            ));
            items.push((
                Mark::Glyph(Glyph::Dimension),
                "Cota alineada (DI)",
                Do::Pick(Pick::Tool(Tool::Dimension)),
                self.tool == Tool::Dimension,
            ));
        }
        items.push((
            Mark::Glyph(Glyph::Text),
            "Texto o nota (TX)",
            Do::Pick(Pick::Command("TEXTO")),
            false,
        ));
        items.push((Mark::Text("+"), "Acercar", Do::ZoomIn, false));
        items.push((Mark::Text("−"), "Alejar", Do::ZoomOut, false));
        items.push((
            Mark::Icon(Icon::Fit),
            "Ajustar a la ventana (Inicio)",
            Do::Fit,
            false,
        ));
        let n = items.len() as f32;
        let gaps = 1.0;
        let h = n * BUTTON + (n - 1.0) * 2.0 + gaps * 9.0 + 12.0;
        let w = BUTTON + 10.0;
        let at = Pos2::new(area.right() - w - 12.0, area.center().y - h / 2.0);
        let mut done = None;
        egui::Area::new(Id::unique("mini-bar"))
            .fixed_pos(at)
            .order(egui::Order::Middle)
            .constrain(false)
            .show(ctx, |ui| {
                egui::Frame::new()
                    .fill(p.panel)
                    .stroke(Stroke::new(1.0, p.line))
                    .corner_radius(CornerRadius::same(16))
                    .shadow(egui::epaint::Shadow {
                        offset: [0, 2],
                        blur: 10,
                        spread: 0,
                        color: egui::Color32::from_black_alpha(18),
                    })
                    .inner_margin(egui::Margin::same(5))
                    .show(ui, |ui| {
                        ui.spacing_mut().item_spacing.y = 2.0;
                        for (i, (mark, hint, act, on)) in items.iter().enumerate() {
                            // The view's own buttons apart from the tools.
                            if matches!(act, Do::ZoomIn) && i > 0 {
                                let (r, _) =
                                    ui.allocate_exact_size(Vec2::new(BUTTON, 9.0), Sense::hover());
                                ui.painter().hline(
                                    r.shrink2(Vec2::new(6.0, 0.0)).x_range(),
                                    r.center().y,
                                    Stroke::new(1.0, p.line),
                                );
                            }
                            let (r, resp) =
                                ui.allocate_exact_size(Vec2::splat(BUTTON), Sense::click());
                            if *on {
                                ui.painter()
                                    .circle_filled(r.center(), BUTTON / 2.0, p.accent_soft);
                            } else if resp.hovered() {
                                ui.painter()
                                    .circle_filled(r.center(), BUTTON / 2.0, p.raised);
                            }
                            let ink = if *on { p.accent } else { p.text };
                            let inner = Rect::from_center_size(r.center(), Vec2::splat(18.0));
                            match mark {
                                Mark::Glyph(g) => glyphs::paint(ui.painter(), inner, *g, ink),
                                Mark::Icon(ic) => icons::paint(ui.painter(), inner, *ic, ink),
                                Mark::Text(t) => {
                                    ui.painter().text(
                                        r.center(),
                                        egui::Align2::CENTER_CENTER,
                                        *t,
                                        egui::FontId::proportional(17.0),
                                        ink,
                                    );
                                }
                            }
                            if resp.on_hover_text(*hint).clicked() {
                                done = Some(*act);
                            }
                        }
                    });
            });
        match done {
            Some(Do::Pick(pick)) => self.pick(pick),
            Some(Do::ZoomIn) => self.zoom_view(area, 1.25),
            Some(Do::ZoomOut) => self.zoom_view(area, 0.8),
            Some(Do::Fit) => self.fit_active_view(),
            None => {}
        }
    }

    /// Zooms the 2D view about the middle of `area`.
    fn zoom_view(&mut self, area: Rect, factor: f64) {
        let rect = self.canvas;
        self.view.zoom_at(rect, area.center(), factor);
    }
}
