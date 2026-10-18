//! The building blocks of the properties palette, as Revit's: a type
//! selector card on top, and parameters in groups (Restricciones,
//! Construcción, Dimensiones, Datos de identidad) as two columns of
//! name and value.

use eframe::egui::{
    self, collapsing_header::CollapsingState, Align2, ComboBox, CornerRadius, FontId, Pos2, Rect,
    Sense, Stroke, StrokeKind, Ui, Vec2,
};

use super::icons::{self, Icon};
use super::theme::Palette;
use super::widgets::strong_font;

/// Height of a parameter row.
pub const ROW: f32 = 26.0;

/// Width of the names column for a palette `width` wide.
pub fn label_width(width: f32) -> f32 {
    (width * 0.44).clamp(96.0, 150.0)
}

/// A group of parameters with a header that folds it (remembered).
pub fn group(ui: &mut Ui, id: &str, title: &str, add: impl FnOnce(&mut Ui)) {
    let p = Palette::current(ui.ctx());
    let id = ui.make_persistent_id(("prop-group", id));
    let mut state = CollapsingState::load_with_default_open(ui.ctx(), id, true);
    ui.add_space(4.0);
    let (rect, resp) =
        ui.allocate_exact_size(Vec2::new(ui.available_width(), 26.0), Sense::click());
    let painter = ui.painter();
    painter.rect_filled(
        rect,
        CornerRadius::same(6),
        if resp.hovered() {
            p.accent_soft
        } else {
            p.raised
        },
    );
    let chevron = if state.is_open() {
        Icon::ChevronDown
    } else {
        Icon::ChevronRight
    };
    icons::paint(
        painter,
        Rect::from_center_size(
            Pos2::new(rect.left() + 12.0, rect.center().y),
            Vec2::splat(12.0),
        ),
        chevron,
        p.muted,
    );
    painter.text(
        Pos2::new(rect.left() + 24.0, rect.center().y),
        Align2::LEFT_CENTER,
        title,
        strong_font(12.0),
        p.text,
    );
    if resp.clicked() {
        state.toggle(ui);
    }
    state.show_body_unindented(ui, |ui| {
        ui.add_space(2.0);
        add(ui);
        ui.add_space(2.0);
    });
}

/// A row: the parameter's name on the left, `value` on the right, given
/// the width it can use.
pub fn row<R>(ui: &mut Ui, name: &str, hint: &str, value: impl FnOnce(&mut Ui, f32) -> R) -> R {
    let p = Palette::current(ui.ctx());
    let width = ui.available_width();
    let lw = label_width(width);
    ui.horizontal(|ui| {
        ui.set_min_height(ROW);
        ui.spacing_mut().item_spacing.x = 6.0;
        let (r, resp) = ui.allocate_exact_size(Vec2::new(lw - 6.0, ROW), Sense::hover());
        let job = egui::text::LayoutJob {
            wrap: egui::text::TextWrapping::truncate_at_width(lw - 14.0),
            ..egui::text::LayoutJob::single_section(
                name.to_owned(),
                egui::TextFormat::simple(FontId::proportional(12.5), p.muted),
            )
        };
        let g = ui.painter().layout_job(job);
        ui.painter().galley(
            Pos2::new(r.left() + 8.0, r.center().y - g.size().y / 2.0),
            g,
            p.muted,
        );
        if !hint.is_empty() {
            resp.on_hover_text(hint);
        }
        let w = ui.available_width();
        value(ui, w)
    })
    .inner
}

/// A row whose value is plain text (not editable).
pub fn text_row(ui: &mut Ui, name: &str, hint: &str, text: &str) {
    row(ui, name, hint, |ui, w| {
        left_aligned(ui, w, |ui| {
            ui.add(
                egui::Label::new(egui::RichText::new(text).color(Palette::current(ui.ctx()).text))
                    .truncate(),
            );
        });
    });
}

/// A row with an identifier (GlobalId): small, monospaced, selectable.
pub fn id_row(ui: &mut Ui, name: &str, id: &str) {
    row(
        ui,
        name,
        "Identificador IFC estable: no cambia nunca",
        |ui, w| {
            left_aligned(ui, w, |ui| {
                ui.add(
                    egui::Label::new(egui::RichText::new(id).monospace().size(11.0))
                        .selectable(true)
                        .truncate(),
                );
            });
        },
    );
}

/// The value column, left to right, `width` wide.
fn left_aligned(ui: &mut Ui, width: f32, add: impl FnOnce(&mut Ui)) {
    ui.allocate_ui_with_layout(
        Vec2::new(width, ROW),
        egui::Layout::left_to_right(egui::Align::Center),
        |ui| {
            ui.set_min_size(Vec2::new(width, ROW));
            ui.add_space(4.0);
            add(ui);
        },
    );
}

/// A combo box filling the value column. Returns the value chosen.
pub fn choice<T: PartialEq + Copy>(
    ui: &mut Ui,
    salt: impl std::hash::Hash + std::fmt::Debug,
    width: f32,
    value: T,
    options: &[(T, &str)],
) -> T {
    let mut v = value;
    let shown = options.iter().find(|o| o.0 == value).map_or("—", |o| o.1);
    ComboBox::from_id_salt(salt)
        .width(width - 8.0)
        .truncate()
        .selected_text(shown)
        .show_ui(ui, |ui| {
            for (o, name) in options {
                ui.selectable_value(&mut v, *o, *name);
            }
        });
    v
}

/// What the buttons of the type card ask for.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct CardAction {
    pub edit: bool,
    pub select_all: bool,
}

/// Revit's type selector: the category, the type in a large list, and
/// "Editar tipo". `items` are (type, name, note such as the thickness);
/// `current` is `None` when the selection mixes types. `instances`, when
/// given, adds a button to select every element of the type on the level.
#[allow(clippy::too_many_arguments)]
pub fn type_card<T: PartialEq + Copy>(
    ui: &mut Ui,
    salt: &str,
    icon: Icon,
    category: &str,
    ifc: &str,
    current: Option<T>,
    items: &[(T, String, String)],
    chosen: &mut Option<T>,
    instances: Option<usize>,
) -> CardAction {
    let p = Palette::current(ui.ctx());
    let mut action = CardAction::default();
    let frame = egui::Frame::new()
        .fill(p.raised)
        .stroke(Stroke::new(1.0, p.line))
        .corner_radius(CornerRadius::same(10))
        .inner_margin(egui::Margin::same(10));
    frame.show(ui, |ui| {
        ui.set_width(ui.available_width());
        ui.horizontal(|ui| {
            let (r, _) = ui.allocate_exact_size(Vec2::splat(40.0), Sense::hover());
            ui.painter()
                .rect_filled(r, CornerRadius::same(8), p.accent_soft);
            icons::paint(ui.painter(), r.shrink(9.0), icon, p.accent);
            ui.vertical(|ui| {
                ui.spacing_mut().item_spacing.y = 2.0;
                ui.horizontal(|ui| {
                    ui.label(
                        egui::RichText::new(category)
                            .font(strong_font(13.5))
                            .color(p.text),
                    );
                    if !ifc.is_empty() {
                        chip_label(ui, ifc);
                    }
                });
                let cur = current.and_then(|c| items.iter().find(|i| i.0 == c));
                let shown = match cur {
                    Some(i) => i.1.clone(),
                    None => "(varios tipos)".into(),
                };
                let mut v = current;
                ComboBox::from_id_salt(("type-card", salt))
                    .width(ui.available_width() - 4.0)
                    .truncate()
                    .selected_text(egui::RichText::new(shown).font(strong_font(12.5)))
                    .show_ui(ui, |ui| {
                        for (id, name, note) in items {
                            let text = if note.is_empty() {
                                name.clone()
                            } else {
                                format!("{name}   · {note}")
                            };
                            if ui.selectable_label(v == Some(*id), text).clicked() {
                                v = Some(*id);
                            }
                        }
                    });
                if v != current {
                    *chosen = v;
                }
                if let Some(i) = cur {
                    if !i.2.is_empty() {
                        ui.label(egui::RichText::new(&i.2).small().color(p.muted));
                    }
                }
            });
        });
        ui.add_space(6.0);
        ui.horizontal(|ui| {
            if ui
                .add_enabled(current.is_some(), egui::Button::new("✎ Editar tipo"))
                .on_hover_text(
                    "Capas, medidas y materiales del tipo: cambia todos los elementos de este tipo",
                )
                .clicked()
            {
                action.edit = true;
            }
            if let Some(n) = instances {
                if ui
                    .button(format!("Seleccionar los {n}"))
                    .on_hover_text("Selecciona todos los elementos de este tipo en la planta")
                    .clicked()
                {
                    action.select_all = true;
                }
            }
        });
    });
    action
}

/// A small rounded label (the IFC class beside the category).
pub fn chip_label(ui: &mut Ui, text: &str) {
    let p = Palette::current(ui.ctx());
    let g = ui
        .painter()
        .layout_no_wrap(text.to_owned(), FontId::monospace(10.0), p.muted);
    let (r, _) = ui.allocate_exact_size(g.size() + Vec2::new(10.0, 4.0), Sense::hover());
    ui.painter().rect(
        r,
        CornerRadius::same(4),
        p.panel,
        Stroke::new(1.0, p.line),
        StrokeKind::Inside,
    );
    ui.painter().galley(r.center() - g.size() / 2.0, g, p.muted);
}

/// The palette's title: "Propiedades" and what it shows.
pub fn title(ui: &mut Ui, what: &str) {
    let p = Palette::current(ui.ctx());
    ui.add_space(8.0);
    ui.horizontal(|ui| {
        ui.label(
            egui::RichText::new("Propiedades")
                .font(strong_font(13.0))
                .color(p.text),
        );
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.add(egui::Label::new(egui::RichText::new(what).small().color(p.muted)).truncate());
        });
    });
    ui.add_space(6.0);
}

/// A swatch of a material's cut colour.
pub fn swatch(ui: &mut Ui, rgb: [u8; 3], size: f32) {
    let (r, _) = ui.allocate_exact_size(Vec2::splat(size), Sense::hover());
    ui.painter().rect(
        r,
        CornerRadius::same(3),
        egui::Color32::from_rgb(rgb[0], rgb[1], rgb[2]),
        Stroke::new(1.0, egui::Color32::from_gray(140)),
        StrokeKind::Inside,
    );
}

/// A short note under a group.
pub fn note(ui: &mut Ui, text: &str) {
    let p = Palette::current(ui.ctx());
    ui.add(egui::Label::new(egui::RichText::new(text).small().color(p.muted)).wrap());
}
