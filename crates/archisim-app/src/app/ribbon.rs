//! The ribbon under the tabs (`docs/INTERFAZ.md` §13), drawn as Revit's
//! «Modificar» tab: panels with their name underneath, big buttons that
//! take the three rows and small ones stacked in columns of three, so that
//! many orders fit in little width. The panels of what is in hand (the tool,
//! the sketch, the selection) come last, tinted with the discipline's
//! colour. When they do not fit, the least used fold into a button with
//! their name that opens them: nothing is cut off at the right.
//!
//! This module only lays out and draws; what each panel holds is decided in
//! `context_bar`.

use eframe::egui::{self, Align2, Color32, CornerRadius, FontId, Rect, Sense, Stroke, Ui, Vec2};

use super::context_bar::paint_mark;
use super::theme::Palette;
use super::toolbox::{Hue, Mark};

/// Rows of small buttons in a panel.
const ROWS: usize = 3;
const ROW_H: f32 = 24.0;
const ROW_GAP: f32 = 2.0;
/// Room above the buttons and for the panel's name under them.
const TOP_PAD: f32 = 4.0;
const LABEL_H: f32 = 17.0;
const SIDE_PAD: f32 = 5.0;
/// A small button with only its drawing.
const SMALL_W: f32 = 26.0;
const BIG_W_MIN: f32 = 44.0;
const ICON: f32 = 17.0;
const BIG_ICON: f32 = 26.0;
/// Between panels (a thin line in the middle).
const GAP: f32 = 7.0;
/// The fixed panels wanted most (Seleccionar, Modificar): they fold after
/// those of what is in hand lose their names.
pub const MAIN_KEEP: u8 = 7;
/// A panel folded into a button: its height.
const FOLDED_H: f32 = 26.0;

/// The ribbon's height, open and folded (`compact`).
pub const RIBBON_HEIGHT: f32 =
    TOP_PAD + ROWS as f32 * ROW_H + (ROWS - 1) as f32 * ROW_GAP + LABEL_H + 3.0;
pub const RIBBON_COMPACT_HEIGHT: f32 = FOLDED_H + 8.0;

fn font() -> FontId {
    FontId::proportional(12.0)
}

/// How a button takes its place in a panel.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Size {
    /// The three rows: a big drawing with the name under it.
    Big,
    /// One row, only its drawing (the name in its tip).
    Small,
    /// One row, drawing and name.
    Named,
}

/// A button of the ribbon.
#[derive(Debug, Clone)]
pub struct Item<A> {
    pub mark: Mark,
    pub name: String,
    pub hint: String,
    pub size: Size,
    /// The one in use (a method, the order running).
    pub on: bool,
    /// Faint when it does not apply now (its tip says why).
    pub enabled: bool,
    pub act: A,
    /// The id a mission may point at (`missions_ui::mark`).
    pub mission: Option<String>,
}

impl<A> Item<A> {
    pub fn new(mark: Mark, name: impl Into<String>, size: Size, act: A) -> Self {
        Item {
            mark,
            name: name.into(),
            hint: String::new(),
            size,
            on: false,
            enabled: true,
            act,
            mission: None,
        }
    }

    pub fn hint(mut self, hint: impl Into<String>) -> Self {
        self.hint = hint.into();
        self
    }

    pub fn on(mut self, on: bool) -> Self {
        self.on = on;
        self
    }

    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    pub fn mission(mut self, id: impl Into<String>) -> Self {
        self.mission = Some(id.into());
        self
    }
}

/// A panel: its name, its buttons and how much it is wanted when there
/// is little room (the lowest fold first).
#[derive(Debug, Clone)]
pub struct Panel<A> {
    pub title: String,
    pub items: Vec<Item<A>>,
    /// Of what is in hand: tinted, and the last to fold.
    pub contextual: bool,
    pub keep: u8,
}

impl<A> Panel<A> {
    pub fn new(title: impl Into<String>, keep: u8, items: Vec<Item<A>>) -> Self {
        Panel {
            title: title.into(),
            items,
            contextual: false,
            keep,
        }
    }

    pub fn contextual(mut self) -> Self {
        self.contextual = true;
        self
    }
}

/// The columns of a panel: a big button alone, or up to three small ones.
fn columns<A>(items: &[Item<A>]) -> Vec<Vec<usize>> {
    let mut cols: Vec<Vec<usize>> = Vec::new();
    let mut open = false;
    for (i, it) in items.iter().enumerate() {
        if it.size == Size::Big {
            cols.push(vec![i]);
            open = false;
        } else if open && cols.last().is_some_and(|c| c.len() < ROWS) {
            cols.last_mut().unwrap().push(i);
        } else {
            cols.push(vec![i]);
            open = true;
        }
    }
    cols
}

fn text_w(ui: &Ui, text: &str) -> f32 {
    ui.painter()
        .layout_no_wrap(text.to_string(), font(), Color32::WHITE)
        .size()
        .x
}

/// The name of a big button in at most two lines.
fn big_lines(name: &str) -> Vec<String> {
    let words: Vec<&str> = name.split_whitespace().collect();
    if words.len() < 2 || name.chars().count() <= 9 {
        return vec![name.to_string()];
    }
    // The split that leaves the two lines most even.
    let total = name.chars().count();
    let mut best = (usize::MAX, 1);
    let mut first = 0;
    for (i, w) in words.iter().enumerate().take(words.len() - 1) {
        first += w.chars().count() + usize::from(i > 0);
        let rest = total - first - 1;
        let worst = first.max(rest);
        if worst < best.0 {
            best = (worst, i + 1);
        }
    }
    vec![words[..best.1].join(" "), words[best.1..].join(" ")]
}

fn column_width<A>(ui: &Ui, items: &[Item<A>], col: &[usize], labels: bool) -> f32 {
    col.iter()
        .map(|&i| {
            let it = &items[i];
            match shown_size(it.size, labels) {
                Size::Big => big_lines(&it.name)
                    .iter()
                    .map(|l| text_w(ui, l) + 10.0)
                    .fold(BIG_W_MIN, f32::max),
                Size::Small => SMALL_W,
                Size::Named => 6.0 + ICON + 5.0 + text_w(ui, &it.name) + 8.0,
            }
        })
        .fold(0.0, f32::max)
}

/// A named button without its name, when there is little room.
fn shown_size(size: Size, labels: bool) -> Size {
    match size {
        Size::Named if !labels => Size::Small,
        s => s,
    }
}

/// The width of a panel open: its columns or its name, the wider.
fn open_width<A>(ui: &Ui, panel: &Panel<A>, labels: bool) -> f32 {
    let cols: f32 = columns(&panel.items)
        .iter()
        .map(|c| column_width(ui, &panel.items, c, labels) + 1.0)
        .sum();
    cols.max(text_w(ui, &panel.title) + 12.0) + 2.0 * SIDE_PAD
}

/// The width of a panel folded into a button: in a row with its drawing
/// and name (compact), or as a big button with its name under it.
fn folded_width<A>(ui: &Ui, panel: &Panel<A>, compact: bool) -> f32 {
    if compact {
        6.0 + ICON + 5.0 + text_w(ui, &panel.title) + 16.0
    } else {
        (text_w(ui, &panel.title) + 14.0).max(BIG_W_MIN)
    }
}

/// How a panel is shown.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Shown {
    /// Open, with the names of its buttons.
    Open,
    /// Open, its small buttons with only their drawings.
    Bare,
    /// A button with its name that opens it.
    Folded,
    /// In the compact ribbon, when even the names do not fit: a button
    /// with only its drawing (its name in the tip).
    Icon,
}

/// A folded panel with only its drawing and its little triangle.
const ICON_ONLY_W: f32 = 6.0 + ICON + 16.0;

/// How each panel is shown in `room`. As in Revit, the least wanted
/// lose first the names of their buttons and then fold, until the rest
/// fit (all of them fold if the ribbon is compact). Those of what is in
/// hand are the last.
fn layout<A>(ui: &Ui, panels: &[Panel<A>], room: f32, compact: bool) -> Vec<Shown> {
    let mut order: Vec<usize> = (0..panels.len()).collect();
    // The least wanted first; among equals, the one more to the right.
    order.sort_by_key(|&i| (panels[i].keep, std::cmp::Reverse(i)));
    if compact {
        let mut shown = vec![Shown::Folded; panels.len()];
        let width = |shown: &[Shown]| -> f32 {
            shown
                .iter()
                .enumerate()
                .map(|(i, s)| {
                    GAP + if *s == Shown::Icon {
                        ICON_ONLY_W
                    } else {
                        folded_width(ui, &panels[i], true)
                    }
                })
                .sum()
        };
        for contextual in [false, true] {
            for &i in order
                .iter()
                .filter(|&&i| panels[i].contextual == contextual)
            {
                if width(&shown) <= room {
                    return shown;
                }
                shown[i] = Shown::Icon;
            }
        }
        return shown;
    }
    let mut shown = vec![Shown::Open; panels.len()];
    let width_of = |i: usize, s: Shown| match s {
        Shown::Open => open_width(ui, &panels[i], true),
        Shown::Bare => open_width(ui, &panels[i], false),
        Shown::Folded | Shown::Icon => folded_width(ui, &panels[i], false),
    };
    let width = |shown: &[Shown]| -> f32 {
        shown
            .iter()
            .enumerate()
            .map(|(i, s)| width_of(i, *s) + GAP)
            .sum()
    };
    // In this order: the fixed panels lose their names; the least wanted
    // of them fold; those of what is in hand lose their names; the most
    // wanted fold (Seleccionar, Modificar); last, those of what is in
    // hand.
    let main = |i: usize| panels[i].keep >= MAIN_KEEP;
    let stages: [(Shown, &dyn Fn(usize) -> bool); 5] = [
        (Shown::Bare, &|i| !panels[i].contextual),
        (Shown::Folded, &|i| !panels[i].contextual && !main(i)),
        (Shown::Bare, &|i| panels[i].contextual),
        (Shown::Folded, &|i| !panels[i].contextual),
        (Shown::Folded, &|i| panels[i].contextual),
    ];
    for (step, which) in stages {
        for &i in order.iter().filter(|&&i| which(i)) {
            if width(&shown) <= room {
                return shown;
            }
            // Only if it takes less room that way.
            if width_of(i, step) < width_of(i, shown[i]) {
                shown[i] = step;
            }
        }
    }
    shown
}

/// Draws the ribbon and gives back what was clicked. `room` is left free
/// at its right for the button that folds it.
pub fn show<A: Clone>(
    ui: &mut Ui,
    panels: &[Panel<A>],
    compact: bool,
    hue: Hue,
    room_right: f32,
) -> Option<A> {
    let p = Palette::current(ui.ctx());
    let room = ui.available_width() - room_right;
    let shown = layout(ui, panels, room, compact);
    let mut chosen = None;
    let top = ui.max_rect().top();
    let full = Rect::from_min_max(
        egui::pos2(ui.max_rect().left(), top),
        egui::pos2(ui.max_rect().left() + room, ui.max_rect().bottom()),
    );
    let mut x = full.left();
    for (i, panel) in panels.iter().enumerate() {
        if i > 0 {
            let line_x = x + GAP / 2.0;
            let (y0, y1) = if compact {
                (full.center().y - 9.0, full.center().y + 9.0)
            } else {
                (full.top() + 6.0, full.bottom() - 5.0)
            };
            ui.painter()
                .vline(line_x, y0..=y1, Stroke::new(1.0, p.line));
            x += GAP;
        }
        if matches!(shown[i], Shown::Folded | Shown::Icon) {
            let named = shown[i] == Shown::Folded;
            let w = if named {
                folded_width(ui, panel, compact)
            } else {
                ICON_ONLY_W
            };
            let rect = if compact {
                Rect::from_center_size(
                    egui::pos2(x + w / 2.0, full.center().y),
                    Vec2::new(w, FOLDED_H),
                )
            } else {
                Rect::from_min_size(
                    egui::pos2(x, full.top() + TOP_PAD),
                    Vec2::new(w, full.height() - TOP_PAD - 3.0),
                )
            };
            if let Some(a) = folded_panel(ui, rect, panel, &p, hue, compact, named) {
                chosen = Some(a);
            }
            x += w;
        } else {
            let labels = shown[i] == Shown::Open;
            let w = open_width(ui, panel, labels);
            let rect = Rect::from_min_size(egui::pos2(x, full.top()), Vec2::new(w, full.height()));
            if let Some(a) = draw_panel(ui, rect, panel, &p, hue, labels) {
                chosen = Some(a);
            }
            x += w;
        }
    }
    chosen
}

/// A panel folded: a button with its first drawing and its name (in a row
/// when the ribbon is compact; as a big button otherwise, as Revit does);
/// it opens the panel under it.
fn folded_panel<A: Clone>(
    ui: &mut Ui,
    rect: Rect,
    panel: &Panel<A>,
    p: &Palette,
    hue: Hue,
    compact: bool,
    named: bool,
) -> Option<A> {
    let resp = ui.interact(
        rect,
        ui.scope_id().with(("folded", &panel.title)),
        Sense::click(),
    );
    let any_on = panel.items.iter().any(|it| it.on);
    let fill = if any_on {
        p.accent_soft
    } else if resp.hovered() {
        p.raised
    } else if panel.contextual {
        hue.soft
    } else {
        Color32::TRANSPARENT
    };
    let fg = if any_on {
        p.accent
    } else if panel.contextual {
        hue.ink
    } else {
        p.text
    };
    ui.painter().rect_filled(rect, CornerRadius::same(7), fill);
    let mark = panel
        .items
        .iter()
        .find(|it| it.on)
        .or(panel.items.first())
        .map(|it| it.mark);
    let triangle = |c: egui::Pos2| {
        egui::Shape::convex_polygon(
            vec![
                c + Vec2::new(-3.0, -1.5),
                c + Vec2::new(3.0, -1.5),
                c + Vec2::new(0.0, 2.0),
            ],
            fg,
            Stroke::NONE,
        )
    };
    if compact {
        if let Some(m) = mark {
            paint_mark(
                ui.painter(),
                Rect::from_center_size(
                    rect.left_center() + Vec2::new(6.0 + ICON / 2.0, 0.0),
                    Vec2::splat(ICON),
                ),
                m,
                fg,
            );
        }
        if named {
            ui.painter().text(
                rect.left_center() + Vec2::new(6.0 + ICON + 5.0, 0.0),
                Align2::LEFT_CENTER,
                &panel.title,
                font(),
                fg,
            );
        }
        ui.painter()
            .add(triangle(rect.right_center() - Vec2::new(8.0, 0.0)));
    } else {
        if let Some(m) = mark {
            paint_mark(
                ui.painter(),
                Rect::from_center_size(
                    egui::pos2(rect.center().x, rect.top() + 8.0 + BIG_ICON / 2.0),
                    Vec2::splat(BIG_ICON),
                ),
                m,
                fg,
            );
        }
        ui.painter().text(
            egui::pos2(rect.center().x, rect.bottom() - 26.0),
            Align2::CENTER_TOP,
            &panel.title,
            FontId::proportional(11.0),
            fg,
        );
        ui.painter()
            .add(triangle(egui::pos2(rect.center().x, rect.bottom() - 7.0)));
    }
    super::missions_ui::mark(ui.ctx(), format!("panel:{}", panel.title), rect);
    let resp = resp.on_hover_text(format!("{}: pulsa para abrirlo", panel.title));
    let mut chosen = None;
    egui::Popup::menu(&resp)
        .close_behavior(egui::PopupCloseBehavior::CloseOnClick)
        .show(|ui| {
            let w = open_width(ui, panel, true);
            let (r, _) = ui.allocate_exact_size(Vec2::new(w, RIBBON_HEIGHT), Sense::hover());
            chosen = draw_panel(ui, r, panel, p, hue, true);
        });
    chosen
}

/// A panel open in `rect`: its columns of buttons and its name under them.
fn draw_panel<A: Clone>(
    ui: &mut Ui,
    rect: Rect,
    panel: &Panel<A>,
    p: &Palette,
    hue: Hue,
    labels: bool,
) -> Option<A> {
    let mut chosen = None;
    // The name, on a band at the bottom (tinted if it is of what is in
    // hand).
    {
        let band = Rect::from_min_max(
            egui::pos2(rect.left() + 1.0, rect.bottom() - LABEL_H),
            egui::pos2(rect.right() - 1.0, rect.bottom() - 1.0),
        );
        let (fill, ink) = if panel.contextual {
            (hue.soft, hue.ink)
        } else {
            (p.raised, p.muted)
        };
        ui.painter().rect_filled(band, CornerRadius::same(5), fill);
        ui.painter().text(
            band.center(),
            Align2::CENTER_CENTER,
            &panel.title,
            FontId::proportional(11.0),
            ink,
        );
    }
    let mut x = rect.left() + SIDE_PAD;
    let y0 = rect.top() + TOP_PAD;
    let cols = columns(&panel.items);
    let used: f32 = cols
        .iter()
        .map(|c| column_width(ui, &panel.items, c, labels) + 1.0)
        .sum();
    // Centred when the name is wider than the buttons.
    x += ((rect.width() - 2.0 * SIDE_PAD - used) / 2.0).max(0.0);
    for col in &cols {
        let w = column_width(ui, &panel.items, col, labels);
        for (row, &i) in col.iter().enumerate() {
            let it = &panel.items[i];
            let size = shown_size(it.size, labels);
            let r = match size {
                Size::Big => Rect::from_min_size(
                    egui::pos2(x, y0),
                    Vec2::new(w, ROWS as f32 * ROW_H + (ROWS - 1) as f32 * ROW_GAP),
                ),
                _ => Rect::from_min_size(
                    egui::pos2(x, y0 + row as f32 * (ROW_H + ROW_GAP)),
                    Vec2::new(w, ROW_H),
                ),
            };
            if button(ui, r, it, size, p) {
                chosen = Some(it.act.clone());
            }
        }
        x += w + 1.0;
    }
    chosen
}

/// One button: true when clicked (and it applies).
fn button<A>(ui: &mut Ui, rect: Rect, it: &Item<A>, size: Size, p: &Palette) -> bool {
    let sense = if it.enabled {
        Sense::click()
    } else {
        Sense::hover()
    };
    let resp = ui.interact(
        rect,
        ui.scope_id()
            .with(("ribbon", &it.name, rect.min.x as i32, rect.min.y as i32)),
        sense,
    );
    let (fill, fg) = if it.on {
        (p.accent_soft, p.accent)
    } else if !it.enabled {
        (Color32::TRANSPARENT, p.faint)
    } else if resp.hovered() {
        (p.raised, p.text)
    } else {
        (Color32::TRANSPARENT, p.text)
    };
    ui.painter().rect_filled(rect, CornerRadius::same(6), fill);
    if it.on {
        ui.painter().rect_stroke(
            rect,
            CornerRadius::same(6),
            Stroke::new(1.0, p.accent),
            egui::StrokeKind::Inside,
        );
    }
    match size {
        Size::Big => {
            let lines = big_lines(&it.name);
            let text_h = 14.0 * lines.len() as f32;
            let icon_c = egui::pos2(
                rect.center().x,
                rect.top() + 4.0 + BIG_ICON / 2.0 + (2.0 - lines.len() as f32) * 4.0,
            );
            paint_mark(
                ui.painter(),
                Rect::from_center_size(icon_c, Vec2::splat(BIG_ICON)),
                it.mark,
                fg,
            );
            let mut y = rect.bottom() - text_h - 3.0;
            for l in &lines {
                ui.painter().text(
                    egui::pos2(rect.center().x, y),
                    Align2::CENTER_TOP,
                    l,
                    font(),
                    fg,
                );
                y += 14.0;
            }
        }
        Size::Small => {
            paint_mark(
                ui.painter(),
                Rect::from_center_size(rect.center(), Vec2::splat(ICON)),
                it.mark,
                fg,
            );
        }
        Size::Named => {
            paint_mark(
                ui.painter(),
                Rect::from_center_size(
                    rect.left_center() + Vec2::new(6.0 + ICON / 2.0, 0.0),
                    Vec2::splat(ICON),
                ),
                it.mark,
                fg,
            );
            ui.painter().text(
                rect.left_center() + Vec2::new(6.0 + ICON + 5.0, 0.0),
                Align2::LEFT_CENTER,
                &it.name,
                font(),
                fg,
            );
        }
    }
    if let Some(id) = &it.mission {
        super::missions_ui::mark(ui.ctx(), id.clone(), rect);
    }
    let tip = if it.hint.is_empty() {
        it.name.clone()
    } else if size == Size::Small || !it.hint.starts_with(&it.name) {
        format!("{}: {}", it.name, it.hint)
    } else {
        it.hint.clone()
    };
    let resp = resp.on_hover_text(tip);
    resp.clicked() && it.enabled
}

#[cfg(test)]
mod tests {
    use super::*;

    fn item(size: Size) -> Item<u8> {
        Item::new(Mark::Icon(super::super::icons::Icon::Move), "x", size, 0)
    }

    /// Small buttons stack in threes; a big one takes a column alone.
    #[test]
    fn columns_of_three() {
        let items = vec![
            item(Size::Big),
            item(Size::Small),
            item(Size::Small),
            item(Size::Named),
            item(Size::Small),
            item(Size::Big),
            item(Size::Small),
        ];
        assert_eq!(
            columns(&items),
            vec![vec![0], vec![1, 2, 3], vec![4], vec![5], vec![6]]
        );
    }

    /// Long names of big buttons go in two even lines.
    #[test]
    fn big_names_in_two_lines() {
        assert_eq!(big_lines("Pegar"), vec!["Pegar"]);
        assert_eq!(big_lines("Crear similar"), vec!["Crear", "similar"]);
        assert_eq!(
            big_lines("Editar perfil del muro"),
            vec!["Editar perfil", "del muro"]
        );
    }
}
