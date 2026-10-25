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

/// An on/off switch with its label, the one drawing for every yes-or-no
/// of the interface (it replaces egui's check boxes): a rounded track,
/// accent when on, with its knob sliding to the right.
pub fn switch(ui: &mut Ui, on: &mut bool, text: impl Into<egui::WidgetText>) -> Response {
    let p = Palette::current(ui.ctx());
    let text: egui::WidgetText = text.into();
    let track = Vec2::new(28.0, 16.0);
    let galley = (!text.is_empty()).then(|| {
        text.into_galley(
            ui,
            Some(egui::TextWrapMode::Extend),
            f32::INFINITY,
            egui::TextStyle::Body,
        )
    });
    let gap = 7.0;
    let width = track.x + galley.as_ref().map_or(0.0, |g| gap + g.size().x);
    let height = ui
        .spacing()
        .interact_size
        .y
        .min(22.0)
        .max(galley.as_ref().map_or(0.0, |g| g.size().y));
    let (rect, mut resp) = ui.allocate_exact_size(Vec2::new(width, height), Sense::click());
    if resp.clicked() {
        *on = !*on;
        resp.mark_changed();
    }
    resp.widget_info(|| {
        egui::WidgetInfo::selected(
            egui::Role::CheckBox,
            ui.is_enabled(),
            *on,
            galley.as_ref().map_or("", |g| g.text()),
        )
    });
    if ui.is_rect_visible(rect) {
        let enabled = ui.is_enabled();
        let t = ui.ctx().animate_bool(resp.id, *on);
        let r = Rect::from_min_size(
            Pos2::new(rect.left(), rect.center().y - track.y / 2.0),
            track,
        );
        let painter = ui.painter();
        let (fill, stroke) = if *on {
            (p.accent, p.accent)
        } else if resp.hovered() && enabled {
            (p.accent_soft, p.accent)
        } else {
            (p.raised, p.line)
        };
        let fill = if enabled {
            fill
        } else {
            fill.gamma_multiply(0.5)
        };
        painter.rect(
            r,
            CornerRadius::same(8),
            fill,
            Stroke::new(1.0, stroke),
            StrokeKind::Inside,
        );
        let knob = egui::lerp((r.left() + 8.0)..=(r.right() - 8.0), t);
        painter.circle(
            Pos2::new(knob, r.center().y),
            5.5,
            egui::Color32::WHITE,
            Stroke::new(1.0, if *on { p.accent } else { p.line }),
        );
        if let Some(g) = galley {
            let at = Pos2::new(r.right() + gap, rect.center().y - g.size().y / 2.0);
            let colour = if enabled { p.text } else { p.faint };
            painter.galley(at, g, colour);
        }
    }
    resp
}

/// [`switch`] as a method of `Ui`, where egui would take a check box.
pub trait SwitchExt {
    fn switch(&mut self, on: &mut bool, text: impl Into<egui::WidgetText>) -> Response;
}

impl SwitchExt for Ui {
    fn switch(&mut self, on: &mut bool, text: impl Into<egui::WidgetText>) -> Response {
        switch(self, on, text)
    }
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

/// A side panel (concepto de interfaz del 24/10/2026): the managers —
/// Plantas, Capas, Estilos, Parámetros, Biblioteca — come in from the
/// right over the navigator, as tall as the window between its bars, and
/// do not cover the drawing more than they need. They slide in (0,15 s)
/// so it is seen where they come from.
pub fn side_sheet<'a>(ctx: &egui::Context, title: &str, width: f32) -> egui::Window<'a> {
    let screen = ctx.content_rect();
    let frame = ctx.cumulative_frame_nr();
    let now = ctx.input(|i| i.time);
    let id = egui::Id::unique(("side-sheet", title.to_string()));
    // When it was opened: it was not drawn the frame before.
    let (last, start) = ctx
        .data(|d| d.get_temp::<(u64, f64)>(id))
        .unwrap_or((0, now));
    let start = if last + 1 >= frame { start } else { now };
    ctx.data_mut(|d| d.insert_temp(id, (frame, start)));
    let t = ((now - start) / 0.15).clamp(0.0, 1.0) as f32;
    if t < 1.0 {
        ctx.request_repaint();
    }
    let w = width.min(screen.width() - 40.0);
    let slide = (1.0 - t) * (w + 20.0);
    let (top, bottom) = (62.0, 52.0);
    let h = (screen.height() - top - bottom - 30.0).max(200.0);
    egui::Window::new(title.to_string())
        .id(id)
        .collapsible(false)
        .resizable(false)
        .anchor(egui::Align2::RIGHT_TOP, egui::vec2(-10.0 + slide, top))
        // Wide enough for what it holds, never off the screen; as tall as
        // the space between the bars, scrolling.
        .min_width(w)
        .max_width((screen.width() - 40.0).max(w))
        .min_height(h)
        .max_height(h)
        .vscroll(true)
}

/// A dialog only for deciding (purge, a conflict, an import): in the
/// middle, the window behind dimmed; it does not close until something is
/// chosen.
pub fn decision<'a>(ctx: &egui::Context, title: &str) -> egui::Window<'a> {
    let screen = ctx.content_rect();
    ctx.layer_painter(egui::LayerId::new(
        egui::Order::Middle,
        egui::Id::unique("decision-dim"),
    ))
    .rect_filled(
        screen,
        CornerRadius::ZERO,
        egui::Color32::from_black_alpha(70),
    );
    egui::Window::new(title.to_string())
        .collapsible(false)
        .resizable(false)
        .order(egui::Order::Foreground)
        .pivot(Align2::CENTER_CENTER)
        .default_pos(screen.center())
}
