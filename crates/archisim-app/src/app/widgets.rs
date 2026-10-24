//! Small controls of the interface, drawn to the theme: tool tiles,
//! segmented choices, toggle chips, icon buttons, list rows and key caps.

use eframe::egui::{
    self, Align2, CornerRadius, FontFamily, FontId, Pos2, Rect, Response, Sense, Stroke,
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
        super::missions_ui::mark(ui.ctx(), format!("seg:{label}"), r);
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

thread_local! {
    /// Where the last click was, and the size of the window: where a
    /// dialog opened by it appears.
    static LAST_CLICK: std::cell::Cell<(Pos2, egui::Vec2)> =
        const { std::cell::Cell::new((Pos2::new(200.0, 120.0), egui::Vec2::new(1440.0, 900.0))) };
}

/// Notes the click of this frame (called once a frame, before any window).
pub fn note_click(ctx: &egui::Context) {
    let size = ctx.content_rect().size();
    let at = ctx.input(|i| {
        i.pointer
            .any_pressed()
            .then(|| i.pointer.interact_pos())
            .flatten()
    });
    LAST_CLICK.with(|c| {
        let (p, _) = c.get();
        c.set((at.unwrap_or(p), size));
    });
}

/// A dialog of the application (concepto de interfaz del 24/10/2026): it
/// opens next to the button that called it — to its left when that is on
/// the right half of the window, to its right otherwise — and never in a
/// corner over the tools; it cannot be folded away. After that, egui
/// remembers where it was moved.
pub fn dialog<'a>(title: impl Into<egui::WidgetText>) -> egui::Window<'a> {
    let (at, size) = LAST_CLICK.with(|c| c.get());
    let right = at.x > size.x / 2.0;
    let low = at.y > size.y * 0.6;
    let pivot = match (right, low) {
        (true, false) => egui::Align2::RIGHT_TOP,
        (false, false) => egui::Align2::LEFT_TOP,
        (true, true) => egui::Align2::RIGHT_BOTTOM,
        (false, true) => egui::Align2::LEFT_BOTTOM,
    };
    let gap = if right { -16.0 } else { 16.0 };
    egui::Window::new(title)
        .collapsible(false)
        .pivot(pivot)
        .default_pos(Pos2::new(at.x + gap, at.y))
        .constrain(true)
}
