//! Small controls of the interface, drawn to the theme: tool tiles,
//! segmented choices, toggle chips, icon buttons, list rows and key caps.

use eframe::egui::{
    self, Align2, Color32, CornerRadius, FontFamily, FontId, Pos2, Rect, Response, Sense, Stroke,
    StrokeKind, Ui, Vec2,
};

use super::icons::{self, Icon};
use super::theme::{Palette, STRONG};

pub fn strong_font(size: f32) -> FontId {
    FontId::new(size, FontFamily::Name(STRONG.into()))
}

/// One of several choices side by side, the chosen one raised. Returns
/// whether the choice changed.
pub fn segmented<T: PartialEq + Copy>(
    ui: &mut Ui,
    id_salt: impl std::hash::Hash + std::fmt::Debug,
    value: &mut T,
    options: &[(T, &str, &str)],
) -> bool {
    let p = Palette::current(ui.ctx());
    let height = 30.0;
    let (rect, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), height), Sense::hover());
    let painter = ui.painter_at(rect.expand(1.0));
    painter.rect(
        rect,
        CornerRadius::same(8),
        p.raised,
        Stroke::new(1.0, p.line),
        StrokeKind::Inside,
    );
    let inner = rect.shrink(3.0);
    let n = options.len().max(1) as f32;
    let w = inner.width() / n;
    let mut changed = false;
    let base = ui.make_persistent_id(id_salt);
    for (i, (v, label, hint)) in options.iter().enumerate() {
        let r = Rect::from_min_size(
            Pos2::new(inner.left() + w * i as f32, inner.top()),
            Vec2::new(w, inner.height()),
        );
        let mut resp = ui.interact(r, base.with(i), Sense::click());
        if !hint.is_empty() {
            resp = resp.on_hover_text(*hint);
        }
        let on = *value == *v;
        if on {
            painter.rect(
                r.shrink(1.0),
                CornerRadius::same(6),
                p.panel,
                Stroke::new(1.0, p.line),
                StrokeKind::Inside,
            );
        } else if resp.hovered() {
            painter.rect_filled(r.shrink(1.0), CornerRadius::same(6), p.accent_soft);
        }
        painter.text(
            r.center(),
            Align2::CENTER_CENTER,
            *label,
            if on {
                strong_font(12.0)
            } else {
                FontId::proportional(12.0)
            },
            if on { p.text } else { p.muted },
        );
        if resp.clicked() && !on {
            *value = *v;
            changed = true;
        }
    }
    changed
}

/// A rounded toggle with a label (bottom bar): accent when on.
pub fn chip(ui: &mut Ui, on: &mut bool, label: &str, hint: &str) -> Response {
    let p = Palette::current(ui.ctx());
    let font = FontId::proportional(11.5);
    let galley = ui.painter().layout_no_wrap(label.to_owned(), font, p.text);
    let size = Vec2::new(galley.size().x + 18.0, 22.0);
    let (rect, resp) = ui.allocate_exact_size(size, Sense::click());
    let resp = resp.on_hover_text(hint);
    let (fill, stroke, fg) = if *on {
        (p.accent_soft, p.accent, p.accent)
    } else if resp.hovered() {
        (p.raised, p.line, p.text)
    } else {
        (p.panel, p.line, p.muted)
    };
    let painter = ui.painter();
    painter.rect(
        rect,
        CornerRadius::same(11),
        fill,
        Stroke::new(1.0, stroke),
        StrokeKind::Inside,
    );
    painter.galley(rect.center() - galley.size() / 2.0, galley, fg);
    if resp.clicked() {
        *on = !*on;
    }
    resp
}

/// A square button with an icon; `active` tints it with the accent.
pub fn icon_button(ui: &mut Ui, icon: Icon, hint: &str, active: bool) -> Response {
    icon_button_sized(ui, icon, hint, active, Vec2::new(28.0, 26.0), true)
}

pub fn icon_button_sized(
    ui: &mut Ui,
    icon: Icon,
    hint: &str,
    active: bool,
    size: Vec2,
    enabled: bool,
) -> Response {
    let p = Palette::current(ui.ctx());
    let sense = if enabled {
        Sense::click()
    } else {
        Sense::hover()
    };
    let (rect, resp) = ui.allocate_exact_size(size, sense);
    let resp = resp.on_hover_text(hint);
    let fg = if !enabled {
        p.faint
    } else if active {
        p.accent
    } else {
        p.text
    };
    if active {
        ui.painter()
            .rect_filled(rect, CornerRadius::same(6), p.accent_soft);
    } else if resp.hovered() && enabled {
        ui.painter()
            .rect_filled(rect, CornerRadius::same(6), p.raised);
    }
    let s = (size.y * 0.62).min(16.0);
    icons::paint(
        ui.painter(),
        Rect::from_center_size(rect.center(), Vec2::splat(s)),
        icon,
        fg,
    );
    resp
}

/// A row of a list (navigator): icon, name and a note on the right.
pub fn list_row(
    ui: &mut Ui,
    icon: Option<Icon>,
    badge: Option<&str>,
    text: &str,
    note: &str,
    active: bool,
    indent: f32,
) -> Response {
    list_row_with(ui, Sense::click(), icon, badge, text, note, active, indent)
}

/// A row of a list that can also be dragged (onto a sheet).
pub fn list_row_draggable(
    ui: &mut Ui,
    icon: Option<Icon>,
    badge: Option<&str>,
    text: &str,
    note: &str,
    active: bool,
    indent: f32,
) -> Response {
    list_row_with(
        ui,
        Sense::click_and_drag(),
        icon,
        badge,
        text,
        note,
        active,
        indent,
    )
}

#[allow(clippy::too_many_arguments)]
fn list_row_with(
    ui: &mut Ui,
    sense: Sense,
    icon: Option<Icon>,
    badge: Option<&str>,
    text: &str,
    note: &str,
    active: bool,
    indent: f32,
) -> Response {
    let p = Palette::current(ui.ctx());
    let (rect, resp) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 28.0), sense);
    let painter = ui.painter_at(rect);
    if active {
        painter.rect_filled(rect, CornerRadius::same(6), p.accent_soft);
    } else if resp.hovered() {
        painter.rect_filled(rect, CornerRadius::same(6), p.raised);
    }
    let fg = if active { p.accent } else { p.text };
    let mut x = rect.left() + 8.0 + indent;
    if let Some(icon) = icon {
        icons::paint(
            &painter,
            Rect::from_center_size(Pos2::new(x + 7.0, rect.center().y), Vec2::splat(15.0)),
            icon,
            fg,
        );
        x += 22.0;
    }
    if let Some(badge) = badge {
        let g = painter.layout_no_wrap(badge.to_owned(), FontId::monospace(10.5), p.muted);
        let w = g.size().x.max(24.0) + 8.0;
        let r = Rect::from_min_size(Pos2::new(x, rect.center().y - 9.0), Vec2::new(w, 18.0));
        painter.rect(
            r,
            CornerRadius::same(4),
            p.raised,
            Stroke::new(1.0, p.line),
            StrokeKind::Inside,
        );
        painter.galley(r.center() - g.size() / 2.0, g, p.muted);
        x += w + 8.0;
    }
    let note_w = if note.is_empty() {
        0.0
    } else {
        let g = painter.layout_no_wrap(note.to_owned(), FontId::monospace(11.0), p.muted);
        let w = g.size().x;
        painter.galley(
            Pos2::new(rect.right() - 8.0 - w, rect.center().y - g.size().y / 2.0),
            g,
            p.muted,
        );
        w + 12.0
    };
    let font = if active {
        strong_font(12.5)
    } else {
        FontId::proportional(12.5)
    };
    let max = (rect.right() - 8.0 - note_w - x).max(10.0);
    let job = egui::text::LayoutJob {
        wrap: egui::text::TextWrapping::truncate_at_width(max),
        ..egui::text::LayoutJob::single_section(text.to_owned(), egui::TextFormat::simple(font, fg))
    };
    let g = painter.layout_job(job);
    painter.galley(Pos2::new(x, rect.center().y - g.size().y / 2.0), g, fg);
    resp
}

/// Draws hints as key caps ("Clic" siguiente punto · "Esc" terminar) in a
/// dark pill centred at the bottom of `area`.
pub fn hud(painter: &egui::Painter, area: Rect, hints: &[(String, String)], p: Palette) {
    if hints.is_empty() || area.width() < 200.0 {
        return;
    }
    let cap_font = FontId::monospace(10.5);
    let text_font = FontId::proportional(12.0);
    let light = Color32::from_rgb(232, 234, 237);
    let mut items = Vec::new();
    let mut total = 0.0;
    for (key, text) in hints {
        let k =
            (!key.is_empty()).then(|| painter.layout_no_wrap(key.clone(), cap_font.clone(), light));
        let t = (!text.is_empty())
            .then(|| painter.layout_no_wrap(text.clone(), text_font.clone(), light));
        let w = k.as_ref().map_or(0.0, |g| g.size().x + 12.0)
            + t.as_ref().map_or(0.0, |g| g.size().x)
            + if k.is_some() && t.is_some() { 6.0 } else { 0.0 };
        total += w;
        items.push((k, t, w));
    }
    let gap = 14.0;
    total += gap * (items.len() as f32 - 1.0);
    // Leave out the last hints if they do not fit.
    let max = area.width() - 40.0;
    while total > max && items.len() > 1 {
        let (_, _, w) = items.pop().unwrap_or((None, None, 0.0));
        total -= w + gap;
    }
    let pill = Rect::from_center_size(
        Pos2::new(area.center().x, area.bottom() - 30.0),
        Vec2::new(total + 24.0, 32.0),
    );
    painter.rect_filled(pill, CornerRadius::same(10), p.hud);
    let mut x = pill.left() + 12.0;
    let cy = pill.center().y;
    for (k, t, w) in items {
        let start = x;
        if let Some(k) = k {
            let r =
                Rect::from_min_size(Pos2::new(x, cy - 10.0), Vec2::new(k.size().x + 12.0, 20.0));
            painter.rect(
                r,
                CornerRadius::same(4),
                Color32::from_white_alpha(30),
                Stroke::new(1.0, Color32::from_white_alpha(56)),
                StrokeKind::Inside,
            );
            painter.galley(r.center() - k.size() / 2.0, k, light);
            x = r.right() + 6.0;
        }
        if let Some(t) = t {
            painter.galley(Pos2::new(x, cy - t.size().y / 2.0), t, light);
        }
        x = start + w + gap;
    }
}

/// Splits a hint like "Clic: siguiente punto · Esc: terminar" into key caps
/// and what they do.
pub fn parse_hints(text: &str) -> Vec<(String, String)> {
    text.split(" · ")
        .map(|part| match part.split_once(": ") {
            Some((k, v)) => (k.trim().to_owned(), v.trim().to_owned()),
            None => (String::new(), part.trim().to_owned()),
        })
        .filter(|(k, v)| !k.is_empty() || !v.is_empty())
        .collect()
}

/// The message of the last action, as a card in the corner of `area`.
pub fn toast(painter: &egui::Painter, area: Rect, text: &str, error: bool, alpha: f32, p: Palette) {
    if alpha <= 0.0 || area.width() < 160.0 {
        return;
    }
    let font = FontId::proportional(12.5);
    let max = (area.width() - 60.0).min(560.0);
    let job = egui::text::LayoutJob {
        wrap: egui::text::TextWrapping {
            max_width: max,
            max_rows: 3,
            ..Default::default()
        },
        ..egui::text::LayoutJob::single_section(
            text.to_owned(),
            egui::TextFormat::simple(font, p.text.gamma_multiply(alpha)),
        )
    };
    let g = painter.layout_job(job);
    let r = Rect::from_min_size(
        area.left_top() + Vec2::new(14.0, 14.0),
        g.size() + Vec2::new(38.0, 16.0),
    );
    painter.rect(
        r,
        CornerRadius::same(9),
        p.panel.gamma_multiply(alpha),
        Stroke::new(1.0, p.line.gamma_multiply(alpha)),
        StrokeKind::Inside,
    );
    let dot = if error { p.bad } else { p.good };
    painter.circle_filled(
        Pos2::new(r.left() + 15.0, r.top() + 16.5),
        4.0,
        dot.gamma_multiply(alpha),
    );
    painter.galley(Pos2::new(r.left() + 28.0, r.top() + 8.0), g, p.text);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hints_become_key_caps() {
        assert_eq!(
            parse_hints(
                "Clic: siguiente punto · Mayús: ángulos de 45° · Intro/Esc/botón derecho: terminar"
            ),
            vec![
                ("Clic".into(), "siguiente punto".into()),
                ("Mayús".into(), "ángulos de 45°".into()),
                ("Intro/Esc/botón derecho".into(), "terminar".into()),
            ]
        );
        assert_eq!(
            parse_hints("Varita mágica: clic dentro de un recinto de muros"),
            vec![(
                "Varita mágica".into(),
                "clic dentro de un recinto de muros".into()
            )]
        );
        assert_eq!(
            parse_hints("solo texto"),
            vec![(String::new(), "solo texto".into())]
        );
    }
}
