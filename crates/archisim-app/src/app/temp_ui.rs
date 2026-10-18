//! What the plan shows around a selected wall, door or window to place it
//! exactly, as Revit does: the blue temporary dimensions, whose value can
//! be clicked and typed, and the two arrows that turn a door or window
//! round (across the wall and along it). Space turns it too.

use super::*;
use crate::temp_dims::{edit, temp_dims, TempDim};

/// How far the line of a temporary dimension is drawn off what it
/// measures, in screen points.
const TEMP_OFFSET: f32 = 22.0;
/// Click reach of the flip arrows, in screen points.
const FLIP_REACH: f32 = 11.0;

/// A temporary dimension being typed.
#[derive(Debug, Clone, PartialEq)]
pub struct TempEdit {
    pub dim: TempDim,
    pub text: String,
    /// Where the field is, on screen (the middle of the label).
    pub at: Pos2,
    focused: bool,
}

/// The arrows beside a selected door or window.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FlipControl {
    /// Open to the other side of the wall.
    Side,
    /// Hinges on the other jamb.
    Hand,
}

impl ArchiSimApp {
    /// The temporary dimensions shown now: with one wall, door or window
    /// selected, in plan, and not while drawing or dragging.
    pub(super) fn shown_temp_dims(&self) -> Vec<TempDim> {
        let busy = self.drag.is_some()
            || self.wall_tool.is_drawing()
            || self.slab_tool.is_drawing()
            || self.dim_tool.is_active()
            || self.mode != ViewMode::Plan;
        // With the arrow, or with the tool that has just drawn the element
        // (as Revit leaves the last wall or door selected); other tools
        // keep their clicks.
        let model = self.ed.model();
        let fits = self.ed.selection.iter().all(|e| match (self.tool, e) {
            (Tool::Arrow, _) => true,
            (Tool::Wall, Element::Wall(_)) => true,
            (Tool::Door | Tool::Window, Element::Opening(o)) => {
                model.opening(*o).is_some_and(|o| {
                    model
                        .opening_type(o.opening_type)
                        .is_some_and(|t| t.is_door() == (self.tool == Tool::Door))
                })
            }
            _ => false,
        });
        if busy || !fits {
            return Vec::new();
        }
        temp_dims(model, &self.ed.selection)
    }

    /// Where a temporary dimension is drawn on screen: its two ends, the
    /// two ends of its line and its label (centre and clickable area).
    pub(super) fn temp_layout(&self, rect: Rect, d: &TempDim) -> (Pos2, Pos2, Pos2, Pos2, Rect) {
        let a = self.view.to_screen(rect, d.a);
        let b = self.view.to_screen(rect, d.b);
        let side = self.view.to_screen(rect, d.a + d.side) - a;
        let off = if side.length() > 1e-3 {
            side.normalized() * TEMP_OFFSET
        } else {
            Vec2::ZERO
        };
        let (la, lb) = (a + off, b + off);
        let along = (lb - la).normalized();
        // The label sits over the middle of the line, on the far side.
        let normal = if off.length() > 0.0 {
            off.normalized()
        } else {
            Vec2::new(-along.y, along.x)
        };
        let mid = la + (lb - la) * 0.5 + normal * 10.0;
        let text = fmt_mm(d.value);
        let w = 8.0 + 7.2 * text.chars().count() as f32;
        (
            a,
            b,
            la,
            lb,
            Rect::from_center_size(mid, Vec2::new(w, 18.0)),
        )
    }

    pub(super) fn draw_temp_dims(&self, painter: &egui::Painter, rect: Rect) {
        let stroke = Stroke::new(1.0, TRACK_LINE);
        for d in self.shown_temp_dims() {
            let (a, b, la, lb, label) = self.temp_layout(rect, &d);
            if a.distance(la) > 0.5 {
                painter.line_segment([a, la + (la - a).normalized() * 3.0], stroke);
                painter.line_segment([b, lb + (lb - b).normalized() * 3.0], stroke);
            }
            painter.line_segment([la, lb], stroke);
            // Oblique ticks, as dimensions are drawn in Spain.
            let along = (lb - la).normalized();
            let tick = Vec2::new(along.x - along.y, along.y + along.x).normalized() * 4.0;
            for p in [la, lb] {
                painter.line_segment([p - tick, p + tick], Stroke::new(1.5, TRACK_LINE));
            }
            if self
                .temp_edit
                .as_ref()
                .is_some_and(|e| e.dim.target == d.target)
            {
                continue;
            }
            painter.rect_filled(label, CornerRadius::same(3), Color32::WHITE);
            painter.rect_stroke(
                label,
                CornerRadius::same(3),
                Stroke::new(1.0, TRACK_LINE),
                StrokeKind::Inside,
            );
            painter.text(
                label.center(),
                Align2::CENTER_CENTER,
                fmt_mm(d.value),
                FontId::monospace(12.0),
                TRACK_LINE,
            );
        }
    }

    /// The flip arrows of the selected door or window: on the side its
    /// leaves do not open to, the one across the wall at the middle, the
    /// one along it beside. Each with its centre and direction on screen.
    pub(super) fn flip_controls(&self, rect: Rect) -> Vec<(Pos2, FlipControl, Vec2)> {
        if self.drag.is_some()
            || self.mode != ViewMode::Plan
            || self.ed.selection.len() != 1
            || !matches!(self.tool, Tool::Arrow | Tool::Door | Tool::Window)
        {
            return Vec::new();
        }
        let Some(Element::Opening(id)) = self.ed.selection.iter().next().copied() else {
            return Vec::new();
        };
        let Some(g) = opening_geometry(self.ed.model(), id) else {
            return Vec::new();
        };
        let (back_face, back) = match g.opens_to {
            WallSide::Interior => (g.exterior_offset, g.n),
            WallSide::Exterior => (g.interior_offset, -g.n),
        };
        let mid = (g.t0 + g.t1) / 2.0;
        let base = self.view.to_screen(rect, g.point(mid, back_face));
        let out = (self.view.to_screen(rect, g.point(mid, back_face) + back) - base).normalized();
        let along = (self.view.to_screen(rect, g.point(mid + 1.0, back_face)) - base).normalized();
        vec![
            (base + out * 20.0, FlipControl::Side, out),
            (base + out * 20.0 + along * 30.0, FlipControl::Hand, along),
        ]
    }

    pub(super) fn draw_flip_controls(&self, painter: &egui::Painter, rect: Rect) {
        for (c, _, dir) in self.flip_controls(rect) {
            painter.circle_filled(c, 9.5, Color32::WHITE);
            painter.circle_stroke(c, 9.5, Stroke::new(1.0, TRACK_LINE));
            let (p, q) = (c - dir * 6.0, c + dir * 6.0);
            let s = Stroke::new(1.5, TRACK_LINE);
            painter.line_segment([p, q], s);
            let side = Vec2::new(-dir.y, dir.x) * 3.0;
            for (tip, back) in [(q, -dir), (p, dir)] {
                painter.line_segment([tip, tip + back * 3.5 + side], s);
                painter.line_segment([tip, tip + back * 3.5 - side], s);
            }
        }
    }

    /// Whether `pos` is over a temporary dimension's value or a flip arrow
    /// (the cursor becomes a hand there).
    pub(super) fn over_temp_controls(&self, rect: Rect, pos: Pos2) -> bool {
        self.flip_controls(rect)
            .iter()
            .any(|(c, _, _)| c.distance(pos) <= FLIP_REACH)
            || self
                .shown_temp_dims()
                .iter()
                .any(|d| self.temp_layout(rect, d).4.contains(pos))
    }

    /// A click on a flip arrow turns the door or window; on the value of a
    /// temporary dimension, opens it for typing. Returns whether the click
    /// was for them (then the tool does not get it).
    pub(super) fn click_temp_controls(&mut self, rect: Rect, pos: Pos2) -> bool {
        let flip = self
            .flip_controls(rect)
            .into_iter()
            .find(|(c, _, _)| c.distance(pos) <= FLIP_REACH);
        if let Some((_, control, _)) = flip {
            self.flip_selected(Some(control));
            return true;
        }
        let hit = self
            .shown_temp_dims()
            .into_iter()
            .map(|d| (self.temp_layout(rect, &d).4, d))
            .find(|(r, _)| r.contains(pos));
        if let Some((r, dim)) = hit {
            self.temp_edit = Some(TempEdit {
                dim,
                text: fmt_mm(dim.value),
                at: r.center(),
                focused: false,
            });
            return true;
        }
        false
    }

    /// Turns the selected door or window: across the wall, along it, or
    /// (`None`, the Space bar) to the next of its four ways. Selected walls
    /// are flipped by Space (their exterior face goes to the other side).
    /// Returns whether there was something to turn.
    pub(super) fn flip_selected(&mut self, control: Option<FlipControl>) -> bool {
        let sel: Vec<Element> = self.ed.selection.iter().copied().collect();
        if let [Element::Opening(id)] = sel[..] {
            let Some(o) = self.ed.model().opening(id) else {
                return false;
            };
            let (side, hand) = match control {
                Some(FlipControl::Side) => crate::flip::flip_side(o.opens_to, o.hand),
                Some(FlipControl::Hand) => crate::flip::flip_hand(o.opens_to, o.hand),
                None => crate::flip::next(o.opens_to, o.hand),
            };
            let r = self.ed.set_opening_handing(id, side, hand);
            let text = format!(
                "Abre al {}, mano {}.",
                match side {
                    WallSide::Interior => "interior",
                    WallSide::Exterior => "exterior",
                },
                match hand {
                    Hand::Left => "izquierda",
                    Hand::Right => "derecha",
                }
            );
            self.report(r, &text);
            return true;
        }
        let walls: Vec<archisim_core::WallId> = sel
            .iter()
            .filter_map(|e| match e {
                Element::Wall(id) => Some(*id),
                _ => None,
            })
            .collect();
        if control.is_none() && !walls.is_empty() && walls.len() == sel.len() {
            let r = self.ed.flip_walls(&walls);
            self.report(r, "Muro invertido: la cara exterior pasa al otro lado.");
            return true;
        }
        false
    }

    /// The field where a temporary dimension is typed: Enter moves the
    /// element so the dimension is exactly that; Esc or a click elsewhere
    /// leaves it as it was.
    pub(super) fn temp_edit_ui(&mut self, ctx: &egui::Context) {
        let Some(mut te) = self.temp_edit.take() else {
            return;
        };
        let id = Id::unique("temp-dim-edit");
        let mut done = None;
        egui::Area::new(id.with("area"))
            .order(egui::Order::Foreground)
            .fixed_pos(te.at - Vec2::new(45.0, 12.0))
            .show(ctx, |ui| {
                egui::Frame::new()
                    .fill(Color32::WHITE)
                    .stroke(Stroke::new(1.5, TRACK_LINE))
                    .corner_radius(CornerRadius::same(3))
                    .inner_margin(egui::Margin::same(2))
                    .show(ui, |ui| {
                        let r = ui.add(
                            TextEdit::singleline(&mut te.text)
                                .id(id)
                                .desired_width(80.0)
                                .font(FontId::monospace(13.0))
                                .text_color(TRACK_LINE),
                        );
                        if !te.focused {
                            // The value comes selected: typing replaces it.
                            r.request_focus();
                            if let Some(mut state) =
                                egui::text_edit::TextEditState::load(ui.ctx(), id)
                            {
                                state.cursor.set_char_range(Some(
                                    egui::text_selection::CCursorRange::select_all_str(&te.text),
                                ));
                                state.store(ui.ctx(), id);
                            }
                            te.focused = true;
                        } else if r.lost_focus() {
                            done = Some(ui.input(|i| i.key_pressed(Key::Enter)));
                        } else if !r.has_focus() {
                            done = Some(false);
                        }
                    });
            });
        match done {
            None => self.temp_edit = Some(te),
            Some(false) => {}
            Some(true) => self.apply_temp_edit(&te.dim, &te.text),
        }
    }

    /// Moves the element of a temporary dimension so that it measures
    /// `text` (millimetres, or with a unit).
    pub(super) fn apply_temp_edit(&mut self, dim: &TempDim, text: &str) {
        let Some(value) = parse_length(text).filter(|v| *v > 0.0) else {
            self.complain(format!(
                "«{text}» no es una medida válida: escribe milímetros (1200) o con unidad (1,2m · 120cm)."
            ));
            return;
        };
        if (value - dim.value).abs() < 1e-9 {
            return;
        }
        let Some((elements, drag)) = edit(self.ed.model(), dim, value) else {
            return;
        };
        match self.ed.drag_elements(&elements, &drag) {
            Ok(_) => self.say(format!("Cota fijada en {} mm.", fmt_mm(value))),
            Err(e) => self.complain(format!("No se puede: {e}.")),
        }
    }
}
