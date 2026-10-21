//! Choosing a hatch pattern by sight: a picture of each pattern next to
//! its name, the technical ones and those of presentation apart, and the
//! opacity of a fill. The pictures are the pattern's own lines
//! (`archisim_draft::hatch`) over a small square.

use archisim_core::{Hatch, HatchPattern, Point2, Polyline, Rgb};
use eframe::egui::{self, Color32, Pos2, Rect, Sense, Stroke, Ui, Vec2};

use super::theme::Palette;

fn colour(c: Rgb) -> Color32 {
    Color32::from_rgb(c[0], c[1], c[2])
}

/// Draws a square of `pattern` in `rect`, in `ink` over `paper`.
pub fn paint_swatch(ui: &Ui, rect: Rect, pattern: HatchPattern, ink: Rgb, paper: Option<Rgb>) {
    let painter = ui.painter_at(rect);
    painter.rect_filled(rect, 3.0, paper.map_or(Color32::WHITE, colour));
    // The square: 1,2 m of plan at 1:50 (patterns at real size: tiles,
    // setts, decking…), a little more for the lawn's bands.
    let side = if pattern == HatchPattern::LawnStripes {
        4.5
    } else {
        1.2
    };
    let k = side / 24.0; // 24 mm of paper across
    let h = Hatch {
        loops: vec![Polyline::straight(
            vec![
                Point2::new(0.0, 0.0),
                Point2::new(side, 0.0),
                Point2::new(side, side),
                Point2::new(0.0, side),
            ],
            true,
        )],
        pattern,
        scale: 1.0,
        angle: 0.0,
        background: None,
        opacity: 1.0,
    };
    let to = |p: Point2| {
        Pos2::new(
            rect.left() + (p.x / side) as f32 * rect.width(),
            rect.bottom() - (p.y / side) as f32 * rect.height(),
        )
    };
    let ink = colour(ink);
    if pattern == HatchPattern::Solid {
        painter.rect_filled(rect.shrink(1.0), 2.0, ink);
    }
    for band in archisim_draft::hatch::hatch_bands(&h) {
        let pts: Vec<Pos2> = band.iter().map(|p| to(*p)).collect();
        painter.add(egui::Shape::convex_polygon(pts, ink, Stroke::NONE));
    }
    if let Some(lines) = archisim_draft::hatch::hatch_lines(&h, k) {
        let stroke = Stroke::new(0.8, ink);
        for (a, b) in lines {
            if a.approx_eq(b) {
                painter.circle_filled(to(a), 0.7, ink);
            } else {
                painter.line_segment([to(a), to(b)], stroke);
            }
        }
    }
    let p = Palette::current(ui.ctx());
    painter.rect_stroke(
        rect,
        3.0,
        Stroke::new(1.0, p.line),
        egui::StrokeKind::Inside,
    );
}

/// A button with the picture and name of the pattern; clicked, a grid of
/// every pattern to choose from. Returns the pattern chosen. `inline`
/// opens the grid under the button instead of in a window of its own (for
/// a picker that is already inside a menu: egui keeps one menu open).
pub fn pattern_picker(
    ui: &mut Ui,
    salt: &str,
    width: f32,
    current: HatchPattern,
    ink: Rgb,
    paper: Option<Rgb>,
) -> HatchPattern {
    picker(ui, salt, width, current, ink, paper, false)
}

/// [`pattern_picker`] whose grid opens under the button ([`inline_grid`]).
pub fn pattern_picker_inline(
    ui: &mut Ui,
    salt: &str,
    width: f32,
    current: HatchPattern,
    ink: Rgb,
    paper: Option<Rgb>,
) -> HatchPattern {
    picker(ui, salt, width, current, ink, paper, true)
}

fn picker(
    ui: &mut Ui,
    salt: &str,
    width: f32,
    current: HatchPattern,
    ink: Rgb,
    paper: Option<Rgb>,
    inline: bool,
) -> HatchPattern {
    // One button of exactly `width`: the picture and the name, cut short
    // if it does not fit (the row must not widen the palette).
    let (rect, response) =
        ui.allocate_exact_size(Vec2::new((width - 8.0).max(60.0), 22.0), Sense::click());
    let pal = Palette::current(ui.ctx());
    let visuals = ui.style().interact(&response);
    ui.painter().rect_filled(rect, 3.0, visuals.weak_bg_fill);
    ui.painter()
        .rect_stroke(rect, 3.0, visuals.bg_stroke, egui::StrokeKind::Inside);
    paint_swatch(
        ui,
        Rect::from_min_size(rect.min, Vec2::splat(22.0)),
        current,
        ink,
        paper,
    );
    let job = egui::text::LayoutJob {
        wrap: egui::text::TextWrapping::truncate_at_width(rect.width() - 32.0),
        ..egui::text::LayoutJob::single_section(
            current.label().to_owned(),
            egui::TextFormat::simple(egui::FontId::proportional(12.0), pal.text),
        )
    };
    let g = ui.painter().layout_job(job);
    ui.painter().galley(
        Pos2::new(rect.left() + 28.0, rect.center().y - g.size().y / 2.0),
        g,
        pal.text,
    );
    let response = response.on_hover_text("Elegir la trama viendo cómo es");
    if inline {
        // The grid is drawn by `inline_grid`, under the row of the button.
        if response.clicked() {
            let id = inline_id(salt);
            let open = ui.data(|d| d.get_temp::<bool>(id)).unwrap_or(false);
            ui.data_mut(|d| d.insert_temp(id, !open));
        }
        return current;
    }
    let mut chosen = current;
    egui::Popup::menu(&response)
        .id(egui::Id::unique(("hatch-picker", salt.to_string())))
        .width(420.0)
        .close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside)
        .show(|ui| {
            if let Some(p) = pattern_grid(ui, current, ink, paper, 4) {
                chosen = p;
                ui.close();
            }
        });
    chosen
}

fn inline_id(salt: &str) -> egui::Id {
    egui::Id::unique(("hatch-picker-open", salt.to_string()))
}

/// The grid of a [`pattern_picker_inline`] (if its button opened it), to
/// draw under the button's row; the pattern clicked, which closes it.
pub fn inline_grid(
    ui: &mut Ui,
    salt: &str,
    current: HatchPattern,
    ink: Rgb,
    paper: Option<Rgb>,
) -> Option<HatchPattern> {
    let id = inline_id(salt);
    if !ui.data(|d| d.get_temp::<bool>(id)).unwrap_or(false) {
        return None;
    }
    let chosen = ui
        .vertical(|ui| pattern_grid(ui, current, ink, paper, 5))
        .inner;
    if chosen.is_some() {
        ui.data_mut(|d| d.insert_temp(id, false));
    }
    chosen
}

/// Every pattern in pictures, the technical ones and those of presentation
/// apart, `columns` to a row; the one clicked.
fn pattern_grid(
    ui: &mut Ui,
    current: HatchPattern,
    ink: Rgb,
    paper: Option<Rgb>,
    columns: usize,
) -> Option<HatchPattern> {
    let mut chosen = None;
    for (title, presentation) in [("Técnicas", false), ("Presentación", true)] {
        ui.label(egui::RichText::new(title).strong());
        let list: Vec<HatchPattern> = HatchPattern::ALL
            .into_iter()
            .filter(|p| p.is_presentation() == presentation)
            .collect();
        for row in list.chunks(columns) {
            ui.horizontal(|ui| {
                for p in row {
                    let r = ui
                        .vertical(|ui| {
                            ui.set_width(96.0);
                            let (rect, r) =
                                ui.allocate_exact_size(Vec2::new(96.0, 56.0), Sense::click());
                            paint_swatch(ui, rect, *p, ink, paper);
                            if *p == current {
                                let pal = Palette::current(ui.ctx());
                                ui.painter().rect_stroke(
                                    rect,
                                    3.0,
                                    Stroke::new(2.0, pal.accent),
                                    egui::StrokeKind::Outside,
                                );
                            }
                            ui.label(egui::RichText::new(p.label()).small());
                            r
                        })
                        .inner;
                    if r.clicked() {
                        chosen = Some(*p);
                    }
                }
            });
            ui.add_space(4.0);
        }
    }
    chosen
}

/// The opacity of a fill, as a slider in percent; the new value if moved.
/// `width` is all the slider may take, its number included.
pub fn opacity_slider(ui: &mut Ui, value: f64, width: f32) -> Option<f64> {
    let mut pct = (value * 100.0).round();
    ui.spacing_mut().slider_width = (width - 64.0).max(40.0);
    let r = ui.add(
        egui::Slider::new(&mut pct, 5.0..=100.0)
            .step_by(5.0)
            .suffix(" %")
            .max_decimals(0),
    );
    r.on_hover_text("100 %: opaco; menos, se ve lo que hay debajo")
        .changed()
        .then_some(pct / 100.0)
}
