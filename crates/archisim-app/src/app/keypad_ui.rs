//! What a touch screen adds over the drawing (`docs/IPAD.md` §3, T4): the
//! touch size, the number keypad and the magnifier.
//!
//! - **Touch size** (`Prefs::touch_size`, on once the screen has been
//!   touched): bigger buttons and rows (`theme`), and handles and points
//!   caught from twice as far (`crate::touch::reach`).
//! - **Keypad**: with no keyboard, the lengths of the temporary dimensions
//!   and the keys of the tools (Intro, Tab, Esc, Espacio, Retroceso) come
//!   from buttons over the drawing. They are typed as if from a keyboard:
//!   `raw_input_hook` puts them in the next frame's input
//!   (`keypad_queue`), so every tool reads them as it reads the keys.
//! - **Magnifier**: with «El dedo dibuja», while the finger draws on the
//!   plan, a square above it shows what is under the fingertip, twice as
//!   big, with the point that will be taken.

use eframe::egui::{
    self, Color32, CornerRadius, Event, Key, Modifiers, Order, Pos2, Rect, Stroke, Vec2,
};

use super::theme::Palette;
use super::{ArchiSimApp, Tool, ViewMode};

/// The keys of the keypad: what is written, and the key or text it types.
const KEYS: &[&[(&str, Press)]] = &[
    &[
        ("7", Press::Text("7")),
        ("8", Press::Text("8")),
        ("9", Press::Text("9")),
        ("⌫", Press::Key(Key::Backspace)),
    ],
    &[
        ("4", Press::Text("4")),
        ("5", Press::Text("5")),
        ("6", Press::Text("6")),
        ("Tab", Press::Key(Key::Tab)),
    ],
    &[
        ("1", Press::Text("1")),
        ("2", Press::Text("2")),
        ("3", Press::Text("3")),
        ("Esc", Press::Key(Key::Escape)),
    ],
    &[
        ("−", Press::Text("-")),
        ("0", Press::Text("0")),
        (",", Press::Text(",")),
        ("↵", Press::Key(Key::Enter)),
    ],
];

/// What a key of the keypad types.
#[derive(Clone, Copy, Debug)]
pub(super) enum Press {
    Text(&'static str),
    Key(Key),
}

/// The side of a key of the keypad (points): a fingertip.
const KEY: f32 = 46.0;
/// The magnifier: its side, how much bigger it shows, and how far above
/// the finger it goes.
const LOUPE: f32 = 132.0;
const LOUPE_ZOOM: f32 = 2.0;
const LOUPE_ABOVE: f32 = 110.0;

impl ArchiSimApp {
    /// Once per frame, after the window: the touch size, the keypad and
    /// the magnifier.
    pub(super) fn touch_overlays(&mut self, ctx: &egui::Context) {
        // The first time a screen is touched: the ribbon folded (once, so
        // what the person changes stays) and, on a narrow screen, the
        // navigator put away, to leave the drawing room.
        if self.touch.seen && !self.touch_layout_applied {
            self.touch_layout_applied = true;
            if !self.prefs.touch_layout {
                self.prefs.touch_layout = true;
                self.prefs.ribbon_compact = true;
                self.prefs.save();
            }
            if ctx.content_rect().width() < 1400.0 {
                self.show_navigator = false;
            }
        }
        let large = self.prefs.touch_size.unwrap_or(self.touch.seen);
        if self.touch_large_applied != Some(large) {
            self.touch_large_applied = Some(large);
            crate::touch::set_large(large);
            super::theme::apply(ctx, self.dark);
        }
        if self.touch.seen && self.prefs.keypad {
            self.keypad(ctx);
        }
        self.loupe(ctx);
        // Each view says it again when it paints.
        self.loupe_source = None;
    }

    /// Whether the keypad is useful now: a tool in hand, or something
    /// selected to move by a typed distance, on a view with a drawing.
    pub(super) fn keypad_wanted(&self) -> bool {
        let drawing = matches!(
            self.mode,
            ViewMode::Plan
                | ViewMode::Section
                | ViewMode::ThreeD
                | ViewMode::Drafting(_)
                | ViewMode::Sheet(_)
        );
        drawing && (self.tool != Tool::Arrow || !self.ed.selection.is_empty())
    }

    /// The number keypad, at the bottom right of the drawing; folded, a
    /// small «123» button.
    fn keypad(&mut self, ctx: &egui::Context) {
        let Some(area) = self.touch_area else {
            return;
        };
        if !self.keypad_wanted() {
            return;
        }
        let p = Palette::current(ctx);
        let open = self.keypad_open;
        let width = if open {
            4.0 * KEY + 3.0 * 6.0 + 20.0
        } else {
            64.0
        };
        let height = if open {
            5.0 * KEY + 4.0 * 6.0 + 20.0
        } else {
            44.0
        };
        let at = keypad_place(
            area.rect,
            Vec2::new(width, height),
            self.keypad_point(area.rect),
        );
        egui::Area::new(egui::Id::unique("teclado-numerico"))
            .order(Order::Foreground)
            .fixed_pos(at)
            .show(ctx, |ui| {
                egui::Frame::new()
                    .fill(p.panel)
                    .stroke(Stroke::new(1.0, p.line))
                    .corner_radius(CornerRadius::same(14))
                    .inner_margin(10.0)
                    .shadow(ctx.global_style().visuals.popup_shadow)
                    .show(ui, |ui| {
                        if !open {
                            if ui
                                .add_sized([44.0, 24.0], egui::Button::new("123").frame(false))
                                .on_hover_text("Teclado numérico para las cotas y las teclas de las herramientas")
                                .clicked()
                            {
                                self.keypad_open = true;
                            }
                            return;
                        }
                        ui.spacing_mut().item_spacing = Vec2::splat(6.0);
                        for row in KEYS {
                            ui.horizontal(|ui| {
                                for (label, press) in *row {
                                    let b = egui::Button::new(
                                        egui::RichText::new(*label).size(18.0),
                                    )
                                    .min_size(Vec2::splat(KEY))
                                    .corner_radius(CornerRadius::same(10));
                                    if ui.add(b).clicked() {
                                        self.type_key(*press);
                                    }
                                }
                            });
                        }
                        ui.horizontal(|ui| {
                            let wide = Vec2::new(3.0 * KEY + 2.0 * 6.0, KEY);
                            if ui
                                .add(
                                    egui::Button::new(egui::RichText::new("Espacio").size(15.0))
                                        .min_size(wide)
                                        .corner_radius(CornerRadius::same(10)),
                                )
                                .on_hover_text("Gira lo que se está colocando, como la barra espaciadora")
                                .clicked()
                            {
                                self.type_key(Press::Key(Key::Space));
                            }
                            if ui
                                .add(
                                    egui::Button::new(egui::RichText::new("▾").size(18.0))
                                        .min_size(Vec2::splat(KEY))
                                        .corner_radius(CornerRadius::same(10)),
                                )
                                .on_hover_text("Recoger el teclado")
                                .clicked()
                            {
                                self.keypad_open = false;
                            }
                        });
                    });
            });
    }

    /// Where on the screen the point being placed is, if there is one: the
    /// keypad goes beside it.
    pub(super) fn keypad_point(&self, rect: Rect) -> Option<Pos2> {
        if !matches!(
            self.mode,
            ViewMode::Plan | ViewMode::Section | ViewMode::Drafting(_)
        ) {
            return None;
        }
        let snap = self.cursor.as_ref()?;
        let p = self.view.to_screen(rect, snap.point);
        rect.contains(p).then_some(p)
    }

    /// Types a key of the keypad: it reaches the tools in the next frame,
    /// as from a keyboard.
    pub(super) fn type_key(&mut self, press: Press) {
        match press {
            Press::Text(t) => self.keypad_queue.push(Event::Text(t.to_string())),
            Press::Key(key) => {
                for pressed in [true, false] {
                    self.keypad_queue.push(Event::Key {
                        key,
                        physical_key: None,
                        pressed,
                        repeat: false,
                        modifiers: Modifiers::NONE,
                    });
                }
                if key == Key::Space {
                    self.keypad_queue.push(Event::Text(" ".into()));
                }
            }
        }
    }

    /// The magnifier, while a finger draws on the plan.
    fn loupe(&mut self, ctx: &egui::Context) {
        let Some(finger) = self.touch.drawing_finger() else {
            return;
        };
        if !self.finger_draws() {
            return;
        }
        let (Some((tex, image)), Some(area)) = (self.loupe_source, self.touch_area) else {
            return;
        };
        if area.three_d || !area.rect.contains(finger) {
            return;
        }
        let p = Palette::current(ctx);
        // Above the finger; below it if there is no room.
        let mut centre = finger - Vec2::new(0.0, LOUPE_ABOVE);
        if centre.y - LOUPE / 2.0 < area.rect.top() {
            centre = finger + Vec2::new(0.0, LOUPE_ABOVE);
        }
        centre.x = centre.x.clamp(
            area.rect.left() + LOUPE / 2.0,
            area.rect.right() - LOUPE / 2.0,
        );
        let dest = Rect::from_center_size(centre, Vec2::splat(LOUPE));
        let painter = ctx.layer_painter(egui::LayerId::new(
            Order::Foreground,
            egui::Id::unique("lupa"),
        ));
        // What is under the fingertip: a square half as big, from the
        // plan's picture.
        let src = Rect::from_center_size(finger, Vec2::splat(LOUPE / LOUPE_ZOOM));
        let uv = Rect::from_min_max(
            Pos2::new(
                (src.min.x - image.min.x) / image.width(),
                (src.min.y - image.min.y) / image.height(),
            ),
            Pos2::new(
                (src.max.x - image.min.x) / image.width(),
                (src.max.y - image.min.y) / image.height(),
            ),
        );
        painter.rect_filled(dest, CornerRadius::same(12), Color32::WHITE);
        painter.image(tex, dest.shrink(2.0), uv, Color32::WHITE);
        painter.rect_stroke(
            dest,
            CornerRadius::same(12),
            Stroke::new(2.0, p.accent),
            egui::StrokeKind::Outside,
        );
        // The point that will be taken (the snapped cursor) and the
        // fingertip.
        let to_loupe = |s: Pos2| centre + (s - finger) * LOUPE_ZOOM;
        let cross = Stroke::new(1.0, p.muted);
        painter.line_segment([centre - Vec2::X * 8.0, centre + Vec2::X * 8.0], cross);
        painter.line_segment([centre - Vec2::Y * 8.0, centre + Vec2::Y * 8.0], cross);
        if let Some(snap) = &self.cursor {
            let s = to_loupe(self.view.to_screen(area.rect, snap.point));
            if dest.shrink(4.0).contains(s) {
                painter.circle_stroke(s, 6.0, Stroke::new(2.0, p.accent));
                painter.circle_filled(s, 2.5, p.accent);
            }
        }
    }
}

/// Where the keypad goes: beside the point being placed (to its right and
/// below, or wherever it fits without covering it), or, with no point, at
/// the bottom right of the drawing.
pub(super) fn keypad_place(area: Rect, size: Vec2, point: Option<Pos2>) -> Pos2 {
    const GAP: f32 = 48.0;
    let corner = area.right_bottom() - size - Vec2::splat(12.0);
    let Some(p) = point else {
        return corner;
    };
    let fits = |at: Pos2| {
        area.shrink(4.0)
            .contains_rect(Rect::from_min_size(at, size))
    };
    [
        p + Vec2::new(GAP, GAP),
        p + Vec2::new(-GAP - size.x, GAP),
        p + Vec2::new(GAP, -GAP - size.y),
        p + Vec2::new(-GAP - size.x, -GAP - size.y),
    ]
    .into_iter()
    .find(|&at| fits(at))
    .unwrap_or(corner)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_keypad_goes_beside_the_point() {
        let area = Rect::from_min_size(Pos2::ZERO, Vec2::new(1000.0, 700.0));
        let size = Vec2::new(220.0, 300.0);
        // Room to the right and below.
        let at = keypad_place(area, size, Some(Pos2::new(200.0, 150.0)));
        assert_eq!(at, Pos2::new(248.0, 198.0));
        // At the bottom right: above and to the left, never over it.
        let p = Pos2::new(900.0, 600.0);
        let at = keypad_place(area, size, Some(p));
        assert!(!Rect::from_min_size(at, size).expand(20.0).contains(p));
        assert!(area.contains_rect(Rect::from_min_size(at, size)));
        // No point: the corner.
        assert_eq!(keypad_place(area, size, None), Pos2::new(768.0, 388.0));
    }
}
