//! How handles look, the same in the plan and in 3D (concepto de interfaz
//! del 24/10/2026): a blue arrow where something stretches, pointing the
//! way it grows; an orange dot on a ring where it turns; a white pin on
//! the side that stays put while it stretches.

use eframe::egui::{Color32, Painter, Pos2, Shape, Stroke, Vec2};

/// The colour of the turning handle (the stretching arrows use the accent).
pub const TURN: Color32 = Color32::from_rgb(224, 112, 42);

/// A stretching arrow at `at`, pointing along `dir` (on the screen).
pub fn arrow(painter: &Painter, at: Pos2, dir: Vec2, fill: Color32, hot: bool) {
    let d = if dir.length() > 1e-6 {
        dir.normalized()
    } else {
        Vec2::X
    };
    let n = Vec2::new(-d.y, d.x);
    let k = if hot { 1.25 } else { 1.0 };
    let (shaft, head, back, tip, neck) = (3.0 * k, 7.5 * k, -3.0 * k, 15.0 * k, 7.0 * k);
    let p = |a: f32, b: f32| at + d * a + n * b;
    let pts = vec![
        p(back, -shaft),
        p(neck, -shaft),
        p(neck, -head),
        p(tip, 0.0),
        p(neck, head),
        p(neck, shaft),
        p(back, shaft),
    ];
    // Drawn as two convex pieces (egui fills only convex polygons).
    let white = Stroke::new(2.5, Color32::WHITE);
    painter.add(Shape::closed_line(pts.clone(), white));
    painter.add(Shape::convex_polygon(
        vec![
            p(back, -shaft),
            p(neck + 0.5, -shaft),
            p(neck + 0.5, shaft),
            p(back, shaft),
        ],
        fill,
        Stroke::NONE,
    ));
    painter.add(Shape::convex_polygon(
        vec![p(neck, -head), p(tip, 0.0), p(neck, head)],
        fill,
        Stroke::NONE,
    ));
}

/// The turning handle at `at`, with its ring round `centre`: a short arc
/// through it, the whole ring while it is taken.
pub fn turn(painter: &Painter, centre: Pos2, at: Pos2, hot: bool) {
    let r = centre.distance(at);
    let alpha = if hot { 200 } else { 150 };
    let ring = Color32::from_rgba_unmultiplied(TURN.r(), TURN.g(), TURN.b(), alpha);
    if r > 4.0 {
        let a = (at - centre).angle();
        let (from, span) = if hot {
            (0.0, std::f32::consts::TAU)
        } else {
            let half = (28.0 / r).min(0.6);
            (a - half, 2.0 * half)
        };
        // Dashed.
        let n = ((r * span / 8.0) as usize).clamp(4, 180);
        for i in (0..n).step_by(2) {
            let a0 = from + span * i as f32 / n as f32;
            let a1 = from + span * (i + 1) as f32 / n as f32;
            painter.line_segment(
                [centre + Vec2::angled(a0) * r, centre + Vec2::angled(a1) * r],
                Stroke::new(1.8, ring),
            );
        }
    }
    let k = if hot { 7.0 } else { 5.5 };
    painter.circle(at, k, TURN, Stroke::new(2.0, Color32::WHITE));
}

/// The pin on the side that stays put.
pub fn pin(painter: &Painter, at: Pos2) {
    painter.circle(
        at,
        4.5,
        Color32::WHITE,
        Stroke::new(1.8, Color32::from_gray(60)),
    );
}

/// A small dark label with a measurement, next to a handle.
pub fn tag(painter: &Painter, at: Pos2, text: String, fill: Color32) {
    let font = eframe::egui::FontId::proportional(12.5);
    let galley = painter.layout_no_wrap(text, font, Color32::WHITE);
    let r =
        eframe::egui::Rect::from_min_size(at + Vec2::new(14.0, -28.0), galley.size()).expand(5.0);
    painter.rect_filled(r, 6.0, fill);
    painter.galley(r.min + Vec2::splat(5.0), galley, Color32::WHITE);
}
