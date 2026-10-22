//! The command search (`docs/INTERFAZ.md` §3): Ctrl+Espacio or the field of
//! the top bar, type «viga», «cota de nivel» or «girar» and choose. It also
//! teaches where each thing is and its key.

use eframe::egui::{self, Align2, Key, Rect, Sense, Ui, Vec2};

use super::glyphs;
use super::icons;
use super::theme::Palette;
use super::toolbox::{self, Mark, Pick};
use super::widgets::strong_font;
use super::ArchiSimApp;

/// Results shown at once.
const SHOWN: usize = 9;

impl ArchiSimApp {
    pub(super) fn open_command_search(&mut self) {
        self.command_search = Some((String::new(), 0));
    }

    /// The field of the top bar that opens the search.
    pub(super) fn search_field(&mut self, ui: &mut Ui) {
        let p = Palette::current(ui.ctx());
        // As wide as fits before the buttons on the right of the bar (with
        // a large text there is less room).
        let right = if self.team.is_some() { 470.0 } else { 310.0 };
        let width = (ui.available_width() - right).clamp(110.0, 250.0);
        let (rect, resp) = ui.allocate_exact_size(Vec2::new(width, 26.0), Sense::click());
        let hovered = resp.hovered();
        ui.painter().rect(
            rect,
            egui::CornerRadius::same(7),
            if hovered { p.raised } else { p.panel },
            egui::Stroke::new(1.0, p.line),
            egui::StrokeKind::Inside,
        );
        glyphs::paint(
            ui.painter(),
            Rect::from_center_size(rect.left_center() + Vec2::new(14.0, 0.0), Vec2::splat(14.0)),
            glyphs::Glyph::Search,
            p.muted,
        );
        ui.painter().text(
            rect.left_center() + Vec2::new(27.0, 0.0),
            Align2::LEFT_CENTER,
            if width > 160.0 {
                "Buscar herramienta…"
            } else {
                "Buscar…"
            },
            egui::FontId::proportional(12.0),
            p.muted,
        );
        if width > 230.0 {
            ui.painter().text(
                rect.right_center() - Vec2::new(8.0, 0.0),
                Align2::RIGHT_CENTER,
                "Ctrl+Espacio",
                egui::FontId::monospace(9.5),
                p.faint,
            );
        }
        if resp
            .on_hover_text("Buscar una herramienta u orden por su nombre o su tecla")
            .clicked()
        {
            self.open_command_search();
        }
    }

    /// The search, while open: a field and the results under it, over the
    /// top of the drawing.
    pub(super) fn command_search_window(&mut self, ctx: &egui::Context) {
        let Some((mut text, mut chosen)) = self.command_search.take() else {
            return;
        };
        let p = Palette::current(ctx);
        let found = toolbox::search(&text);
        let (up, down, enter, escape) = ctx.input_mut(|i| {
            (
                i.consume_key(egui::Modifiers::NONE, Key::ArrowUp),
                i.consume_key(egui::Modifiers::NONE, Key::ArrowDown),
                i.consume_key(egui::Modifiers::NONE, Key::Enter),
                i.consume_key(egui::Modifiers::NONE, Key::Escape),
            )
        });
        let n = found.len().min(SHOWN);
        if n > 0 {
            if down {
                chosen = (chosen + 1) % n;
            }
            if up {
                chosen = (chosen + n - 1) % n;
            }
        }
        chosen = chosen.min(n.saturating_sub(1));
        let mut picked: Option<Pick> = None;
        let mut close = escape;
        let screen = ctx.content_rect();
        let width = 440.0;
        let area = egui::Area::new(egui::Id::unique("command-search"))
            .order(egui::Order::Foreground)
            .fixed_pos(egui::pos2(screen.center().x - width / 2.0, screen.top() + 52.0))
            .show(ctx, |ui| {
                egui::Frame::popup(ui.style())
                    .inner_margin(egui::Margin::same(8))
                    .show(ui, |ui| {
                        ui.set_width(width);
                        let edit = ui.add(
                            egui::TextEdit::singleline(&mut text)
                                .id(egui::Id::unique("command-search-text"))
                                .hint_text("Herramienta u orden: viga, cota de nivel, girar…")
                                .desired_width(f32::INFINITY),
                        );
                        if !edit.has_focus() {
                            edit.request_focus();
                        }
                        if edit.changed() {
                            chosen = 0;
                        }
                        if text.trim().is_empty() {
                            ui.add_space(4.0);
                            ui.label(
                                egui::RichText::new(
                                    "Escribe el nombre o la tecla. Flechas para elegir, Intro para usarla.",
                                )
                                .small()
                                .color(p.muted),
                            );
                        } else if found.is_empty() {
                            ui.add_space(4.0);
                            ui.label(
                                egui::RichText::new("Nada con ese nombre.")
                                    .small()
                                    .color(p.muted),
                            );
                        }
                        ui.add_space(4.0);
                        for (i, f) in found.iter().take(SHOWN).enumerate() {
                            let later = matches!(f.pick, Pick::Later(_));
                            let (rect, resp) = ui.allocate_exact_size(
                                Vec2::new(ui.available_width(), 34.0),
                                Sense::click(),
                            );
                            if resp.hovered() && ctx.input(|i| i.pointer.delta() != Vec2::ZERO) {
                                chosen = i;
                            }
                            if i == chosen {
                                ui.painter().rect_filled(
                                    rect,
                                    egui::CornerRadius::same(6),
                                    p.accent_soft,
                                );
                            }
                            let fg = if later { p.faint } else { p.text };
                            let mark =
                                Rect::from_center_size(rect.left_center() + Vec2::new(16.0, 0.0), Vec2::splat(20.0));
                            match f.mark {
                                Mark::Glyph(g) => glyphs::paint(ui.painter(), mark, g, fg),
                                Mark::Icon(icon) => {
                                    icons::paint(ui.painter(), mark.shrink(1.0), icon, fg)
                                }
                            }
                            ui.painter().text(
                                rect.left_center() + Vec2::new(34.0, -7.0),
                                Align2::LEFT_CENTER,
                                f.name,
                                strong_font(12.0),
                                fg,
                            );
                            let right = if f.key.is_empty() {
                                f.place.to_string()
                            } else {
                                format!("{} · {}", f.place, f.key)
                            };
                            let small = egui::FontId::proportional(10.5);
                            let right_w = ui
                                .painter()
                                .layout_no_wrap(right.clone(), small.clone(), p.muted)
                                .size()
                                .x;
                            ui.painter().text(
                                rect.right_center() - Vec2::new(8.0, 0.0),
                                Align2::RIGHT_CENTER,
                                right,
                                small.clone(),
                                p.muted,
                            );
                            // The hint, cut short before the place.
                            let room = rect.width() - 34.0 - right_w - 22.0;
                            let mut hint = if later {
                                format!("Todavía no: {}", f.hint)
                            } else {
                                f.hint.to_string()
                            };
                            let fits = |h: &str| {
                                ui.painter()
                                    .layout_no_wrap(h.to_string(), small.clone(), p.muted)
                                    .size()
                                    .x
                                    <= room
                            };
                            if !fits(&hint) {
                                while !hint.is_empty() && !fits(&format!("{hint}…")) {
                                    hint.pop();
                                }
                                hint = format!("{}…", hint.trim_end());
                            }
                            ui.painter().text(
                                rect.left_center() + Vec2::new(34.0, 8.0),
                                Align2::LEFT_CENTER,
                                hint,
                                small,
                                p.muted,
                            );
                            if resp.clicked() {
                                picked = Some(f.pick);
                            }
                        }
                    });
            });
        if enter {
            if let Some(f) = found.get(chosen) {
                picked = Some(f.pick);
            } else {
                close = true;
            }
        }
        // A click elsewhere closes it.
        if ctx.input(|i| i.pointer.any_pressed())
            && !ctx
                .input(|i| i.pointer.interact_pos())
                .is_some_and(|pos| area.response.rect.contains(pos))
        {
            close = true;
        }
        if let Some(pick) = picked {
            ctx.memory_mut(|m| m.surrender_focus(egui::Id::unique("command-search-text")));
            self.pick(pick);
            return;
        }
        if close {
            ctx.memory_mut(|m| m.surrender_focus(egui::Id::unique("command-search-text")));
            return;
        }
        self.command_search = Some((text, chosen));
    }
}
