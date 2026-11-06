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
use super::thumbs::Thumb;
use super::widgets::strong_font;

/// Height of a parameter row.
pub const ROW: f32 = 26.0;

/// Width of the names column for a palette `width` wide.
pub fn label_width(width: f32) -> f32 {
    (width * 0.56).clamp(96.0, 172.0)
}

/// A group of parameters with a header that folds it (remembered).
pub fn group(ui: &mut Ui, id: &str, title: &str, add: impl FnOnce(&mut Ui)) {
    let p = Palette::current(ui.ctx());
    // What is only looked at now and then starts folded (§12.6): the
    // identity, the phase, the discipline and the checks.
    // Also the faces, the profile, the layers and the holes, which are
    // looked at more than changed, so that what is edited fits without
    // scrolling (Antonio, 6/11/2026).
    let quiet = [
        "-identity",
        "-phase",
        "-checks",
        "-discipline",
        "-tools",
        "-profile",
        "-layers",
        "-holes",
        "-usages-list",
    ]
    .iter()
    .any(|s| id.ends_with(s));
    let id = ui.make_persistent_id(("prop-group", id));
    let mut state = CollapsingState::load_with_default_open(ui.ctx(), id, !quiet);
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
            wrap: egui::text::TextWrapping::truncate_at_width(lw - 10.0),
            ..egui::text::LayoutJob::single_section(
                crate::units::unit_label(name).into_owned(),
                egui::TextFormat::simple(FontId::proportional(12.0), p.muted),
            )
        };
        let g = ui.painter().layout_job(job);
        ui.painter().galley(
            Pos2::new(r.left() + 4.0, r.center().y - g.size().y / 2.0),
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

/// One type offered by the type selector.
pub struct TypeItem<T> {
    pub id: T,
    pub name: String,
    /// Thickness, size or family, under the name.
    pub note: String,
    pub thumb: Option<Thumb>,
    /// What the picture shows ("Corte en planta 1:20 · exterior arriba").
    pub caption: &'static str,
    /// The folder of the gallery it goes in ("Muros exteriores (fachada)",
    /// the family of a door…); empty for a gallery without folders.
    pub folder: String,
}

/// Revit's type selector, with a picture: the type's plan cut (walls,
/// slabs) or its door or window in plan, the category with its IFC class,
/// and the type, chosen from a gallery of pictures with a search box.
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
    items: &[TypeItem<T>],
    chosen: &mut Option<T>,
    instances: Option<usize>,
) -> CardAction {
    let p = Palette::current(ui.ctx());
    let mut action = CardAction::default();
    let cur = current.and_then(|c| items.iter().find(|i| i.id == c));
    let frame = egui::Frame::new()
        .fill(p.raised)
        .stroke(Stroke::new(1.0, p.line))
        .corner_radius(CornerRadius::same(10))
        .inner_margin(egui::Margin::same(10));
    frame.show(ui, |ui| {
        ui.set_width(ui.available_width());
        // A small picture on white paper beside the category (the big one
        // in its tip): the card leaves room for what is edited below.
        ui.horizontal(|ui| {
            if let Some(t) = cur.and_then(|c| c.thumb) {
                let caption = cur.map(|c| c.caption).unwrap_or_default();
                paper(ui, t, Vec2::new(86.0, 58.0)).on_hover_ui(|ui| {
                    paper(ui, t, Vec2::new(320.0, 220.0));
                    ui.label(egui::RichText::new(caption).small());
                });
                ui.add_space(4.0);
            }
            ui.vertical(|ui| {
                ui.horizontal(|ui| {
                    icons::paint(
                        ui.painter(),
                        Rect::from_min_size(
                            ui.cursor().min + Vec2::new(0.0, 1.0),
                            Vec2::splat(16.0),
                        ),
                        icon,
                        p.accent,
                    );
                    ui.add_space(22.0);
                    ui.label(
                        egui::RichText::new(category)
                            .font(strong_font(13.5))
                            .color(p.text),
                    );
                });
                if !ifc.is_empty() {
                    chip_label(ui, ifc);
                }
                if let Some(c) = cur {
                    if !c.note.is_empty() {
                        ui.add(
                            egui::Label::new(egui::RichText::new(&c.note).small().color(p.muted))
                                .truncate(),
                        );
                    }
                }
            });
        });
        ui.add_space(4.0);
        // The type: a large button that opens the gallery.
        let shown = cur.map_or("(varios tipos)".to_string(), |c| c.name.clone());
        let (rect, resp) =
            ui.allocate_exact_size(Vec2::new(ui.available_width(), 32.0), Sense::click());
        let resp = resp.on_hover_text("Elegir otro tipo");
        #[cfg(test)]
        ui.ctx()
            .data_mut(|d| d.insert_temp(egui::Id::unique("test-type-button"), rect));
        ui.painter().rect(
            rect,
            CornerRadius::same(7),
            p.panel,
            Stroke::new(1.0, if resp.hovered() { p.accent } else { p.line }),
            StrokeKind::Inside,
        );
        let job = egui::text::LayoutJob {
            wrap: egui::text::TextWrapping::truncate_at_width(rect.width() - 40.0),
            ..egui::text::LayoutJob::single_section(
                shown,
                egui::TextFormat::simple(strong_font(12.5), p.text),
            )
        };
        let g = ui.painter().layout_job(job);
        ui.painter().galley(
            Pos2::new(rect.left() + 10.0, rect.center().y - g.size().y / 2.0),
            g,
            p.text,
        );
        icons::paint(
            ui.painter(),
            Rect::from_center_size(
                Pos2::new(rect.right() - 16.0, rect.center().y),
                Vec2::splat(12.0),
            ),
            Icon::ChevronDown,
            p.muted,
        );
        if let Some(v) = gallery(ui, &resp, salt, current, items) {
            if Some(v) != current {
                *chosen = Some(v);
            }
        }
        // The types like it (§12.6): those of its folder, or the ones next
        // to it in the list, to change with one click.
        if let Some(v) = similar_types(ui, salt, cur, items) {
            if Some(v) != current {
                *chosen = Some(v);
            }
        }
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

/// Up to four types like `cur` in small cards with their picture: those
/// in its folder, else its neighbours in the list. Returns the one
/// clicked.
fn similar_types<T: PartialEq + Copy>(
    ui: &mut Ui,
    salt: &str,
    cur: Option<&TypeItem<T>>,
    items: &[TypeItem<T>],
) -> Option<T> {
    let cur = cur?;
    let at = items.iter().position(|i| i.id == cur.id)?;
    let mut like: Vec<&TypeItem<T>> = if cur.folder.is_empty() {
        Vec::new()
    } else {
        items
            .iter()
            .filter(|i| i.id != cur.id && i.folder == cur.folder)
            .collect()
    };
    if like.len() < 2 {
        // Its neighbours, after it first.
        like = items[at + 1..]
            .iter()
            .chain(items[..at].iter().rev())
            .collect();
    }
    like.truncate(4);
    if like.is_empty() {
        return None;
    }
    let p = Palette::current(ui.ctx());
    ui.add_space(6.0);
    ui.label(egui::RichText::new("Parecidos").small().color(p.muted));
    let gap = 4.0;
    let w = ((ui.available_width() - gap * 3.0) / 4.0).max(30.0);
    let size = Vec2::new(w, 52.0);
    let mut picked = None;
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = gap;
        for (k, item) in like.iter().enumerate() {
            let (rect, resp) = ui.allocate_exact_size(size, Sense::click());
            let resp = resp.on_hover_text(&item.name);
            #[cfg(test)]
            ui.ctx()
                .data_mut(|d| d.insert_temp(egui::Id::unique(("test-similar", salt, k)), rect));
            let _ = (salt, k);
            ui.painter().rect(
                rect,
                CornerRadius::same(6),
                egui::Color32::WHITE,
                Stroke::new(1.0, if resp.hovered() { p.accent } else { p.line }),
                StrokeKind::Inside,
            );
            match item.thumb {
                Some(t) => {
                    let s = t.fit(rect.shrink(4.0).size());
                    ui.painter().image(
                        t.tex,
                        Rect::from_center_size(rect.center(), s),
                        Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)),
                        egui::Color32::WHITE,
                    );
                }
                None => {
                    let job = egui::text::LayoutJob {
                        wrap: egui::text::TextWrapping {
                            max_width: rect.width() - 6.0,
                            max_rows: 3,
                            break_anywhere: false,
                            overflow_character: Some('…'),
                        },
                        halign: egui::Align::Center,
                        ..egui::text::LayoutJob::single_section(
                            item.name.clone(),
                            egui::TextFormat::simple(egui::FontId::proportional(10.0), p.text),
                        )
                    };
                    let g = ui.painter().layout_job(job);
                    let h = g.size().y;
                    ui.painter().galley(
                        Pos2::new(rect.center().x, rect.center().y - h / 2.0),
                        g,
                        p.text,
                    );
                }
            }
            if resp.clicked() {
                picked = Some(item.id);
            }
        }
    });
    picked
}

/// A type picture on white paper, fitted in `max`, centred.
pub fn paper(ui: &mut Ui, t: Thumb, max: Vec2) -> egui::Response {
    let (outer, resp) =
        ui.allocate_exact_size(Vec2::new(max.x, max.y.min(t.size.y + 16.0)), Sense::hover());
    ui.painter()
        .rect_filled(outer, CornerRadius::same(6), egui::Color32::WHITE);
    let size = t.fit(outer.size() - Vec2::splat(12.0));
    let r = Rect::from_center_size(outer.center(), size);
    ui.painter().image(
        t.tex,
        r,
        Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)),
        egui::Color32::WHITE,
    );
    resp
}

/// The gallery of types under the type button: a search box and a tile
/// per type with its picture. Returns the type clicked.
fn gallery<T: PartialEq + Copy>(
    ui: &mut Ui,
    button: &egui::Response,
    salt: &str,
    current: Option<T>,
    items: &[TypeItem<T>],
) -> Option<T> {
    let p = Palette::current(ui.ctx());
    let search_id = ui.make_persistent_id(("type-search", salt));
    let mut picked = None;
    let shown = egui::Popup::menu(button)
        .width(button.rect.width().max(320.0))
        .close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside)
        .show(|ui| {
            let mut query: String = ui.ctx().data(|d| d.get_temp(search_id)).unwrap_or_default();
            let edit = ui.add(
                egui::TextEdit::singleline(&mut query)
                    .hint_text("Buscar tipo…")
                    .desired_width(f32::INFINITY),
            );
            if !edit.has_focus() && query.is_empty() {
                edit.request_focus();
            }
            ui.ctx()
                .data_mut(|d| d.insert_temp(search_id, query.clone()));
            let q = query.to_lowercase();
            ui.add_space(4.0);
            let shown: Vec<&TypeItem<T>> = items
                .iter()
                .filter(|i| {
                    q.is_empty()
                        || i.name.to_lowercase().contains(&q)
                        || i.note.to_lowercase().contains(&q)
                })
                .collect();
            if shown.is_empty() {
                ui.label(
                    egui::RichText::new("Ningún tipo coincide")
                        .small()
                        .color(p.muted),
                );
            }
            // The folders, in the order the items bring them (as in the
            // types dialog: exterior walls, interior walls; one per family).
            let mut folders: Vec<&str> = Vec::new();
            for i in &shown {
                if !folders.contains(&i.folder.as_str()) {
                    folders.push(&i.folder);
                }
            }
            egui::ScrollArea::vertical()
                .max_height(420.0)
                .show(ui, |ui| {
                    // The tiles' width is taken where they go: inside a
                    // folder the indent leaves less room, and tiles as wide
                    // as outside it would widen the gallery at every frame.
                    let mut grid = |ui: &mut Ui, list: &[&TypeItem<T>]| {
                        let w = ((ui.available_width() - 8.0) / 2.0).max(60.0);
                        for pair in list.chunks(2) {
                            ui.horizontal(|ui| {
                                ui.spacing_mut().item_spacing.x = 8.0;
                                for item in pair {
                                    if tile(ui, w, item, current == Some(item.id)).clicked() {
                                        picked = Some(item.id);
                                    }
                                }
                            });
                            ui.add_space(2.0);
                        }
                    };
                    if folders.len() <= 1 && folders.first().is_none_or(|f| f.is_empty()) {
                        grid(ui, &shown);
                    } else {
                        for f in folders {
                            let list: Vec<&TypeItem<T>> =
                                shown.iter().copied().filter(|i| i.folder == f).collect();
                            let name = if f.is_empty() { "Otros" } else { f };
                            egui::CollapsingHeader::new(
                                egui::RichText::new(format!("{name}  ({})", list.len()))
                                    .font(strong_font(12.5))
                                    .color(p.text),
                            )
                            .id_salt(("type-folder", salt, f))
                            .default_open(true)
                            .show(ui, |ui| grid(ui, &list));
                        }
                    }
                });
            if picked.is_some() {
                ui.ctx().data_mut(|d| d.remove::<String>(search_id));
                ui.close();
            }
        });
    #[cfg(test)]
    if let Some(r) = shown {
        ui.ctx()
            .data_mut(|d| d.insert_temp(egui::Id::unique("test-type-gallery"), r.response.rect));
    }
    #[cfg(not(test))]
    let _ = shown;
    picked
}

/// A tile of the gallery: picture, name and note.
pub fn tile<T>(ui: &mut Ui, width: f32, item: &TypeItem<T>, selected: bool) -> egui::Response {
    let p = Palette::current(ui.ctx());
    let (rect, resp) = ui.allocate_exact_size(Vec2::new(width, 118.0), Sense::click());
    let resp = resp.on_hover_text(format!("{}\n{}", item.name, item.note));
    let fill = if selected {
        p.accent_soft
    } else if resp.hovered() {
        p.raised
    } else {
        p.panel
    };
    let stroke = if selected || resp.hovered() {
        p.accent
    } else {
        p.line
    };
    ui.painter().rect(
        rect,
        CornerRadius::same(8),
        fill,
        Stroke::new(if selected { 1.5 } else { 1.0 }, stroke),
        StrokeKind::Inside,
    );
    let pic = Rect::from_min_size(rect.min + Vec2::splat(6.0), Vec2::new(width - 12.0, 62.0));
    ui.painter()
        .rect_filled(pic, CornerRadius::same(5), egui::Color32::WHITE);
    if let Some(t) = item.thumb {
        let size = t.fit(pic.size() - Vec2::splat(8.0));
        ui.painter().image(
            t.tex,
            Rect::from_center_size(pic.center(), size),
            Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)),
            egui::Color32::WHITE,
        );
    }
    let text = |s: &str, font: FontId, color, top: f32, rows: usize| {
        let job = egui::text::LayoutJob {
            wrap: egui::text::TextWrapping {
                max_width: width - 14.0,
                max_rows: rows,
                break_anywhere: false,
                overflow_character: Some('…'),
            },
            ..egui::text::LayoutJob::single_section(
                s.to_owned(),
                egui::TextFormat::simple(font, color),
            )
        };
        let g = ui.painter().layout_job(job);
        ui.painter()
            .galley(Pos2::new(rect.left() + 7.0, top), g, color);
    };
    text(
        &item.name,
        strong_font(11.5),
        if selected { p.accent } else { p.text },
        pic.bottom() + 5.0,
        2,
    );
    text(
        &item.note,
        FontId::proportional(10.5),
        p.muted,
        rect.bottom() - 17.0,
        1,
    );
    resp
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

/// The colours Properties is drawn in: those of the discipline of what is
/// shown (§12.1), set before it is drawn.
pub fn set_hue(ctx: &egui::Context, hue: super::toolbox::Hue) {
    ctx.data_mut(|d| d.insert_temp(egui::Id::unique("props-hue"), hue));
}

fn hue(ctx: &egui::Context) -> Option<super::toolbox::Hue> {
    ctx.data(|d| d.get_temp(egui::Id::unique("props-hue")))
}

/// The palette's title: "Propiedades" and what it shows, on a band of the
/// discipline's colour.
pub fn title(ui: &mut Ui, what: &str) {
    let p = Palette::current(ui.ctx());
    let h = hue(ui.ctx());
    ui.add_space(8.0);
    egui::Frame::new()
        .fill(h.map_or(p.raised, |h| h.soft))
        .corner_radius(CornerRadius::same(8))
        .inner_margin(egui::Margin::symmetric(10, 7))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.horizontal(|ui| {
                ui.label(
                    egui::RichText::new("Propiedades")
                        .font(strong_font(13.0))
                        .color(h.map_or(p.text, |h| h.ink)),
                );
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.add(
                        egui::Label::new(
                            egui::RichText::new(what)
                                .small()
                                .color(h.map_or(p.muted, |h| h.ink)),
                        )
                        .truncate(),
                    );
                });
            });
        });
    ui.add_space(6.0);
}

/// A measure shown big on a tile (§12.6): what it is, its value in metres,
/// and whether it is typed in here.
pub struct Measure {
    pub label: &'static str,
    pub hint: &'static str,
    pub value: f64,
    pub editable: bool,
}

/// The main measures of an element as big figures, three to a row; a
/// figure that can be typed is a field. Returns the one changed and its
/// new value.
pub fn measure_tiles(
    ui: &mut Ui,
    salt: impl std::hash::Hash + std::fmt::Debug,
    tiles: &[Measure],
) -> Option<(usize, f64)> {
    let p = Palette::current(ui.ctx());
    let mut changed = None;
    let gap = 6.0;
    let n = tiles.len().clamp(1, 3) as f32;
    let w = (ui.available_width() - gap * (n - 1.0)) / n;
    let salt = egui::Id::unique(("measure-tiles", salt));
    ui.add_space(4.0);
    for (row, chunk) in tiles.chunks(3).enumerate() {
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = gap;
            for (i, m) in chunk.iter().enumerate() {
                let index = row * 3 + i;
                egui::Frame::new()
                    .fill(p.raised)
                    .corner_radius(CornerRadius::same(9))
                    .inner_margin(egui::Margin::symmetric(8, 6))
                    .show(ui, |ui| {
                        ui.vertical(|ui| {
                            let cw = (w - 16.0).max(30.0);
                            ui.set_width(cw);
                            ui.set_max_width(cw);
                            ui.add(
                                egui::Label::new(
                                    egui::RichText::new(format!(
                                        "{} ({})",
                                        m.label,
                                        crate::units::unit().symbol()
                                    ))
                                    .small()
                                    .color(p.muted),
                                )
                                .truncate(),
                            )
                            .on_hover_text(m.hint);
                            if m.editable {
                                ui.scope(|ui| {
                                    ui.style_mut().override_font_id = Some(strong_font(15.0));
                                    ui.spacing_mut().button_padding = Vec2::new(2.0, 1.0);
                                    if let Some(v) =
                                        super::length_field(ui, salt.with(index), m.value, cw - 6.0)
                                    {
                                        changed = Some((index, v));
                                    }
                                });
                            } else {
                                ui.add(
                                    egui::Label::new(
                                        egui::RichText::new(crate::units::fmt_mm(m.value))
                                            .font(strong_font(15.0))
                                            .color(p.text),
                                    )
                                    .truncate(),
                                )
                                .on_hover_text(m.hint);
                            }
                        })
                    });
            }
        });
        ui.add_space(gap);
    }
    changed
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
