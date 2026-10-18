//! Painting the elevations and sections of `type_drawing` on the screen:
//! white paper, the pens of the plan, the cut hatches of the materials.

use archisim_core::CutPattern;
use eframe::egui::{
    self, Color32, CornerRadius, Painter, Pos2, Rect, Shape, Stroke, StrokeKind, Vec2,
};

use crate::type_drawing::{Drawing, Item, Pen};

const INK: Color32 = Color32::from_rgb(30, 30, 30);

/// Paints `d` centred in `rect`, as large as it fits (never more than
/// `max_ppm` points per metre), and returns where it went. `weight`
/// scales the pens (smaller in list thumbnails).
pub fn paint_drawing(
    painter: &Painter,
    rect: Rect,
    d: &Drawing,
    max_ppm: f32,
    weight: f32,
) -> Rect {
    let (w, h) = (d.width() as f32, d.height() as f32);
    if w <= 0.0 || h <= 0.0 {
        return rect;
    }
    let k = (rect.width() / w).min(rect.height() / h).min(max_ppm);
    let size = Vec2::new(w * k, h * k);
    let area = Rect::from_center_size(rect.center(), size);
    let to = |p: [f64; 2]| {
        Pos2::new(
            area.left() + (p[0] - d.lo[0]) as f32 * k,
            area.bottom() - (p[1] - d.lo[1]) as f32 * k,
        )
    };
    let pen = |p: Pen| {
        let w = match p {
            Pen::Thin => 0.6,
            Pen::Medium => 1.0,
            Pen::Thick => 1.7,
        };
        Stroke::new(w * weight, INK)
    };
    let clip = painter.clip_rect();
    for item in &d.items {
        match item {
            Item::Rect {
                min,
                max,
                fill,
                pattern,
                pen: p,
            } => {
                let r = Rect::from_two_pos(to(*min), to(*max));
                let [cr, cg, cb, ca] = *fill;
                if ca > 0 {
                    painter.rect_filled(
                        r,
                        CornerRadius::ZERO,
                        Color32::from_rgba_unmultiplied(cr, cg, cb, ca),
                    );
                }
                if let Some(pat) = pattern {
                    hatch(&painter.with_clip_rect(r.intersect(clip)), r, *pat, weight);
                }
                painter.rect_stroke(r, CornerRadius::ZERO, pen(*p), StrokeKind::Middle);
            }
            Item::Line {
                from,
                to: end,
                pen: p,
                dashed,
            } => {
                let pts = [to(*from), to(*end)];
                if *dashed {
                    painter.extend(Shape::dashed_line(&pts, pen(*p), 4.0, 3.0));
                } else {
                    painter.line_segment(pts, pen(*p));
                }
            }
        }
    }
    area
}

/// The cut hatch of a material inside `r` (the painter clips it).
fn hatch(painter: &Painter, r: Rect, pattern: CutPattern, weight: f32) {
    let stroke = Stroke::new(0.5 * weight.max(0.6), Color32::from_gray(60));
    let gap = 5.0 * weight.max(0.6);
    let diagonals = |painter: &Painter, up: bool| {
        let span = r.width() + r.height();
        let mut t = 0.0;
        while t < span {
            let (a, b) = if up {
                (
                    Pos2::new(r.left() + t, r.bottom()),
                    Pos2::new(r.left() + t - r.height(), r.top()),
                )
            } else {
                (
                    Pos2::new(r.left() + t - r.height(), r.bottom()),
                    Pos2::new(r.left() + t, r.top()),
                )
            };
            painter.line_segment([a, b], stroke);
            t += gap;
        }
    };
    match pattern {
        CutPattern::Solid | CutPattern::Empty => {}
        CutPattern::Diagonal => diagonals(painter, false),
        CutPattern::Cross => {
            diagonals(painter, false);
            diagonals(painter, true);
        }
        CutPattern::Dots => {
            let mut y = r.top() + gap / 2.0;
            let mut row = 0;
            while y < r.bottom() {
                let mut x = r.left() + gap / 2.0 + if row % 2 == 1 { gap / 2.0 } else { 0.0 };
                while x < r.right() {
                    painter.circle_filled(Pos2::new(x, y), 0.6 * weight.max(0.6), stroke.color);
                    x += gap;
                }
                y += gap;
                row += 1;
            }
        }
        CutPattern::Insulation => {
            // A zigzag across the thickness, along the longer side.
            let along_x = r.width() >= r.height();
            let (len, depth) = if along_x {
                (r.width(), r.height())
            } else {
                (r.height(), r.width())
            };
            let step = (depth * 0.5).max(2.0);
            let mut pts = Vec::new();
            let mut t = 0.0;
            let mut up = false;
            while t <= len + step {
                let across = if up { depth * 0.9 } else { depth * 0.1 };
                pts.push(if along_x {
                    Pos2::new(r.left() + t, r.bottom() - across)
                } else {
                    Pos2::new(r.left() + across, r.top() + t)
                });
                t += step;
                up = !up;
            }
            painter.add(Shape::line(pts, stroke));
        }
    }
}

/// White paper with a thin border, where previews are drawn.
pub fn paper(painter: &Painter, rect: Rect, line: Color32) {
    painter.rect(
        rect,
        CornerRadius::same(6),
        Color32::WHITE,
        Stroke::new(1.0, line),
        StrokeKind::Inside,
    );
}

/// A sample of a line style, `width` points long, in `rect`.
pub fn line_sample(painter: &Painter, rect: Rect, style: archisim_core::LineStyle, color: Color32) {
    let y = rect.center().y;
    let stroke = Stroke::new(1.2, color);
    let pattern = style.pattern_mm();
    if pattern.is_empty() {
        painter.line_segment(
            [Pos2::new(rect.left(), y), Pos2::new(rect.right(), y)],
            stroke,
        );
        return;
    }
    // Paper millimetres shown 2.5 points each.
    let k = 2.5;
    let mut x = rect.left();
    let mut i = 0;
    while x < rect.right() {
        let len = (pattern[i % pattern.len()] as f32 * k).max(0.8);
        if i % 2 == 0 {
            let end = (x + len).min(rect.right());
            if end - x < 1.0 {
                painter.circle_filled(Pos2::new(x, y), 0.8, color);
            } else {
                painter.line_segment([Pos2::new(x, y), Pos2::new(end, y)], stroke);
            }
        }
        x += len;
        i += 1;
    }
}

/// Labels at the lower corners of a preview ("Exterior", "Interior").
pub fn side_labels(painter: &Painter, rect: Rect, sides: (&str, &str), color: Color32) {
    let font = egui::FontId::proportional(10.5);
    if !sides.0.is_empty() {
        painter.text(
            rect.left_bottom() + Vec2::new(8.0, -6.0),
            egui::Align2::LEFT_BOTTOM,
            sides.0,
            font.clone(),
            color,
        );
    }
    if !sides.1.is_empty() {
        painter.text(
            rect.right_bottom() + Vec2::new(-8.0, -6.0),
            egui::Align2::RIGHT_BOTTOM,
            sides.1,
            font,
            color,
        );
    }
}
