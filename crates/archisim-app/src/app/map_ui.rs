//! The map view (N8, `docs/TERRITORIO.md` §2.11), as CSL Map View in
//! Cities: Skylines: a window with the whole area of the surroundings,
//! flat and in colour (`archisim_plan::map`), with the part the plan shows
//! framed on it. A click on the map takes the plan there; «Ver toda el
//! área» fits the plan to it.

use archisim_core::Point2;
use eframe::egui::{self, Color32, Sense, Stroke, TextureOptions};

use super::*;

/// How many pixels across the map is drawn.
const MAP_PIXELS: f64 = 900.0;

/// The map window.
#[derive(Default)]
pub struct MapState {
    pub open: bool,
    /// The map drawn, its frame and the model's revision it was drawn at.
    texture: Option<(egui::TextureHandle, (Point2, Point2), u64, u64)>,
}

impl MapState {
    /// The part of the plan the map drawn covers, if one is drawn.
    pub fn frame(&self) -> Option<(Point2, Point2)> {
        self.texture.as_ref().map(|t| t.1)
    }
}

impl ArchiSimApp {
    pub(super) fn map_dialog(&mut self, ctx: &egui::Context) {
        if !self.map.open {
            return;
        }
        let level = self.ed.level;
        let revision = self.ed.revision();
        let fresh = self
            .map
            .texture
            .as_ref()
            .is_some_and(|(_, _, r, l)| *r == revision && *l == level.0);
        if !fresh {
            let model = self.ed.shown();
            self.map.texture =
                archisim_plan::map::render_map(&model, level, MAP_PIXELS).and_then(|svg| {
                    let (lo, hi) = archisim_plan::map::map_frame(&model, level)?;
                    // The same margin the map leaves round its frame.
                    let margin = ((hi.x - lo.x).max(hi.y - lo.y) * 0.03).max(5.0);
                    let lo = Point2::new(lo.x - margin, lo.y - margin);
                    let hi = Point2::new(hi.x + margin, hi.y + margin);
                    let h = (MAP_PIXELS * (hi.y - lo.y) / (hi.x - lo.x)).round() as usize;
                    let image =
                        crate::plan_image::rasterize_svg(&svg, [MAP_PIXELS as usize, h.max(1)]);
                    let tex = ctx.load_texture("map", image, TextureOptions::LINEAR);
                    Some((tex, (lo, hi), revision, level.0))
                });
        }
        let mut open = true;
        let mut go_to: Option<Point2> = None;
        let mut fit = false;
        let canvas = self.canvas;
        let view = self.view;
        super::widgets::dialog("Mapa")
            .open(&mut open)
            .default_size([460.0, 500.0])
            .resizable(true)
            .collapsible(false)
            .show(ctx, |ui| {
                let Some((tex, (lo, hi), _, _)) = &self.map.texture else {
                    ui.label(
                        "Esta planta no tiene calles, parcelas ni edificios que poner en un mapa.",
                    );
                    return;
                };
                let (lo, hi) = (*lo, *hi);
                let avail = ui.available_size() - egui::vec2(0.0, 30.0);
                let ratio = ((hi.y - lo.y) / (hi.x - lo.x)) as f32;
                let w = avail.x.min(avail.y / ratio).max(120.0);
                let size = egui::vec2(w, w * ratio);
                let (rect, response) = ui.allocate_exact_size(size, Sense::click_and_drag());
                ui.painter().image(
                    tex.id(),
                    rect,
                    egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                    Color32::WHITE,
                );
                let to_map = |p: Point2| {
                    egui::pos2(
                        rect.left() + ((p.x - lo.x) / (hi.x - lo.x)) as f32 * rect.width(),
                        rect.bottom() - ((p.y - lo.y) / (hi.y - lo.y)) as f32 * rect.height(),
                    )
                };
                let to_model = |q: egui::Pos2| {
                    Point2::new(
                        lo.x + ((q.x - rect.left()) / rect.width()) as f64 * (hi.x - lo.x),
                        lo.y + ((rect.bottom() - q.y) / rect.height()) as f64 * (hi.y - lo.y),
                    )
                };
                // What the plan shows.
                let (a, b) = view.visible(canvas);
                let shown = egui::Rect::from_two_pos(to_map(a), to_map(b)).intersect(rect);
                ui.painter().rect_stroke(
                    shown,
                    0.0,
                    Stroke::new(2.0, Color32::from_rgb(200, 60, 40)),
                    egui::StrokeKind::Middle,
                );
                if response.clicked() || response.dragged() {
                    if let Some(q) = response.interact_pointer_pos() {
                        go_to = Some(to_model(q));
                    }
                }
                let response = response.on_hover_text("Clic o arrastrar: llevar la planta ahí");
                let _ = response;
                ui.horizontal(|ui| {
                    if ui.button("Ver toda el área").clicked() {
                        fit = true;
                    }
                    let ends = self.ed.model().outside_ends(self.ed.level).len();
                    if ends > 0 {
                        ui.label(format!("{ends} conexiones con fuera"));
                    }
                });
            });
        self.map.open = open;
        if let Some(p) = go_to {
            self.view.center = p;
        }
        if fit {
            if let Some((lo, hi)) = self.map.frame() {
                self.view.fit_exact(self.canvas, lo, hi, 8.0);
            }
        }
    }
}
