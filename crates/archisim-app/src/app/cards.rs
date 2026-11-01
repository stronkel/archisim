//! Choosing by pictures (Antonio, 29/10/2026: «es imprescindible que en
//! las propiedades o editar tipo estén las opciones de forma visual en
//! tarjetas, sea lo que sea»). A choice of a family (its model, the front
//! of a kitchen, the opening of a window…) shows the option chosen as a
//! card with its picture, and a click opens every option as a card, each
//! drawn as it would be. A material shows its swatch with its texture; a
//! click opens the materials recommended for that piece (those of its
//! kind, from the project and from the palette of the collections) and,
//! below, every other material of the project.

use std::collections::HashMap;

use archisim_core::{MaterialId, Model, TextureKind};
use eframe::egui::{
    self, Color32, ColorImage, CornerRadius, Pos2, Rect, Sense, Stroke, StrokeKind, TextureHandle,
    TextureId, TextureOptions, Ui, Vec2,
};

use super::theme::Palette;
use super::thumbs::Thumb;

/// The size of an option's picture in its card.
const PICTURE: Vec2 = Vec2::new(92.0, 66.0);
/// The width of an option's card in the grid.
const CARD_W: f32 = 104.0;

/// Paints `t` fitted in `r` on white paper.
fn paint_picture(ui: &Ui, r: Rect, t: Option<Thumb>) {
    let painter = ui.painter();
    painter.rect_filled(r, CornerRadius::same(5), Color32::WHITE);
    match t {
        Some(t) => {
            let size = t.fit(r.size() - Vec2::splat(4.0));
            painter.image(
                t.tex,
                Rect::from_center_size(r.center(), size),
                Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)),
                Color32::WHITE,
            );
        }
        None => {
            // Still being drawn (a few pictures each frame).
            painter.circle_stroke(r.center(), 6.0, Stroke::new(1.5, Color32::from_gray(200)));
            ui.ctx().request_repaint();
        }
    }
}

/// Whether a family's choice is about how it is drawn in plan (a tree's
/// drawing, canopy, colour in presentation…): its cards are drawn in
/// plan, not in 3D (E2).
pub(super) fn plan_choice(name: &str) -> bool {
    matches!(
        name,
        "plan_drawing"
            | "drawing"
            | "tech_style"
            | "style"
            | "style_own"
            | "look"
            | "look_own"
            | "shape"
            | "shape_own"
    )
}

/// "practicable" → "Practicable" (the recipes write some options in
/// lower case).
fn capitalized(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        Some(f) => f.to_uppercase().chain(c).collect(),
        None => String::new(),
    }
}

/// A card of the grid: its picture and its name below; `on` the one
/// chosen.
fn card(ui: &mut Ui, name: &str, on: bool, picture: impl FnOnce(&Ui, Rect)) -> egui::Response {
    card_tall(ui, name, on, 38.0, picture)
}

/// A card with `text` points under its picture for its name.
fn card_tall(
    ui: &mut Ui,
    name: &str,
    on: bool,
    text: f32,
    picture: impl FnOnce(&Ui, Rect),
) -> egui::Response {
    let p = Palette::current(ui.ctx());
    let name = &capitalized(name);
    let (rect, resp) = ui.allocate_exact_size(Vec2::new(CARD_W, PICTURE.y + text), Sense::click());
    let resp = resp.on_hover_text(name);
    let fill = if on {
        p.accent_soft
    } else if resp.hovered() {
        p.raised
    } else {
        p.panel
    };
    let stroke = if on || resp.hovered() {
        p.accent
    } else {
        p.line
    };
    ui.painter().rect(
        rect,
        CornerRadius::same(7),
        fill,
        Stroke::new(if on { 1.5 } else { 1.0 }, stroke),
        StrokeKind::Inside,
    );
    let pic = Rect::from_min_size(
        Pos2::new(rect.center().x - PICTURE.x / 2.0, rect.min.y + 6.0),
        PICTURE,
    );
    picture(ui, pic);
    let text = Rect::from_min_max(
        Pos2::new(rect.min.x + 4.0, pic.max.y + 2.0),
        Pos2::new(rect.max.x - 4.0, rect.max.y - 2.0),
    );
    let galley = ui.painter().layout(
        name.to_string(),
        egui::FontId::proportional(11.0),
        if on { p.accent } else { p.text },
        text.width(),
    );
    ui.painter().galley(
        Pos2::new(text.center().x - galley.size().x / 2.0, text.min.y),
        galley,
        p.text,
    );
    resp
}

/// A row whose value is a card: its name above, the card below across
/// the whole panel (cards need the width a field does not).
pub(super) fn card_row<R>(ui: &mut Ui, label: &str, value: impl FnOnce(&mut Ui, f32) -> R) -> R {
    let p = Palette::current(ui.ctx());
    ui.add_space(2.0);
    ui.label(egui::RichText::new(label).color(p.muted));
    let w = ui.available_width();
    let r = value(ui, w);
    ui.add_space(2.0);
    r
}

/// Every option as a card, `per_row` in a row; the one clicked.
pub(super) fn option_grid(
    ui: &mut Ui,
    options: &[String],
    current: &str,
    per_row: usize,
    thumb: &mut dyn FnMut(&str) -> Option<Thumb>,
) -> Option<String> {
    let mut picked = None;
    for chunk in options.chunks(per_row.max(1)) {
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 6.0;
            for o in chunk {
                let t = thumb(o);
                if card(ui, o, o == current, |ui, r| paint_picture(ui, r, t)).clicked() {
                    picked = Some(o.clone());
                }
            }
        });
        ui.add_space(4.0);
    }
    picked
}

/// The chosen option as a wide button with its picture: the row's value.
fn current_button(
    ui: &mut Ui,
    width: f32,
    name: &str,
    picture: impl FnOnce(&Ui, Rect),
) -> egui::Response {
    let p = Palette::current(ui.ctx());
    let name = &capitalized(name);
    let (rect, resp) = ui.allocate_exact_size(Vec2::new(width, 48.0), Sense::click());
    let stroke = if resp.hovered() { p.accent } else { p.line };
    ui.painter().rect(
        rect,
        CornerRadius::same(6),
        p.panel,
        Stroke::new(1.0, stroke),
        StrokeKind::Inside,
    );
    let pic = Rect::from_min_size(rect.min + Vec2::splat(4.0), Vec2::new(56.0, 40.0));
    picture(ui, pic);
    let text = Rect::from_min_max(
        Pos2::new(pic.max.x + 6.0, rect.min.y),
        Pos2::new(rect.max.x - 16.0, rect.max.y),
    );
    let galley = ui.painter().layout(
        name.to_string(),
        egui::FontId::proportional(12.0),
        p.text,
        text.width().max(10.0),
    );
    ui.painter().galley(
        Pos2::new(text.min.x, text.center().y - galley.size().y / 2.0),
        galley,
        p.text,
    );
    // The chevron that says it opens.
    let c = Pos2::new(rect.max.x - 9.0, rect.center().y);
    ui.painter().line_segment(
        [c + Vec2::new(-4.0, -2.0), c + Vec2::new(0.0, 2.0)],
        Stroke::new(1.3, p.muted),
    );
    ui.painter().line_segment(
        [c + Vec2::new(0.0, 2.0), c + Vec2::new(4.0, -2.0)],
        Stroke::new(1.3, p.muted),
    );
    resp.on_hover_text("Ver todas las opciones")
}

/// A choice shown as cards: the option chosen, and every option when it
/// is clicked. `thumb` draws an option as it would be. The option picked.
pub(super) fn option_cards(
    ui: &mut Ui,
    width: f32,
    options: &[String],
    current: &str,
    thumb: &mut dyn FnMut(&str) -> Option<Thumb>,
) -> Option<String> {
    let t = thumb(current);
    let button = current_button(ui, width, current, |ui, r| paint_picture(ui, r, t));
    let mut picked = None;
    egui::Popup::menu(&button)
        .width(CARD_W * 3.0 + 30.0)
        .close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside)
        .show(|ui| {
            egui::ScrollArea::vertical()
                .max_height(460.0)
                .show(ui, |ui| {
                    picked = option_grid(ui, options, current, 3, thumb);
                });
            if picked.is_some() {
                ui.close();
            }
        });
    picked
}

/// A choice of cards in groups, each card painted by `paint` (on white
/// paper): the option chosen as a wide button and, when clicked, every
/// option under its group's title. The option picked.
pub(super) fn grouped_cards(
    ui: &mut Ui,
    width: f32,
    groups: &[(String, Vec<String>)],
    current: &str,
    paint: &mut dyn FnMut(&Ui, Rect, &str),
) -> Option<String> {
    let blank = |ui: &Ui, r: Rect| {
        ui.painter()
            .rect_filled(r, CornerRadius::same(5), Color32::WHITE);
    };
    let button = current_button(ui, width, current, |ui, r| {
        blank(ui, r);
        paint(ui, r, current);
    });
    let mut picked = None;
    egui::Popup::menu(&button)
        .width(CARD_W * 4.0 + 40.0)
        .close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside)
        .show(|ui| {
            let p = Palette::current(ui.ctx());
            egui::ScrollArea::vertical()
                .max_height(520.0)
                .show(ui, |ui| {
                    for (title, options) in groups {
                        if options.is_empty() {
                            continue;
                        }
                        ui.label(egui::RichText::new(title).strong().color(p.muted));
                        for chunk in options.chunks(4) {
                            ui.horizontal(|ui| {
                                ui.spacing_mut().item_spacing.x = 6.0;
                                for o in chunk {
                                    let resp = card_tall(ui, o, o == current, 52.0, |ui, r| {
                                        blank(ui, r);
                                        paint(ui, r, o);
                                    });
                                    if resp.clicked() {
                                        picked = Some(o.clone());
                                    }
                                }
                            });
                            ui.add_space(4.0);
                        }
                        ui.add_space(4.0);
                    }
                });
            if picked.is_some() {
                ui.close();
            }
        });
    picked
}

// ------------------------------------------------------------ materials

/// A material offered: one of the project's, or one of the palette of
/// the collections not yet in it (added when chosen).
#[derive(Debug, Clone, PartialEq)]
pub(super) enum MatPick {
    Project(u64),
    Palette(&'static str),
}

/// A material's card: what it is, its name, colour and texture.
#[derive(Debug, Clone)]
pub(super) struct MatCard {
    pub pick: MatPick,
    pub name: String,
    pub rgb: [u8; 3],
    pub texture: Option<(TextureKind, f64)>,
}

/// The card of one of the project's materials.
pub(super) fn project_card(m: &Model, id: MaterialId) -> Option<MatCard> {
    let mat = m.material(id)?;
    Some(MatCard {
        pick: MatPick::Project(id.0),
        name: mat.name.clone(),
        rgb: m.surface_color(id),
        texture: archisim_geom::solid::texture_of(m, id, true),
    })
}

/// What is woven (rattan, rush, bamboo, straw, reed).
const WOVEN: &[&str] = &["Ratán", "Enea", "Bambú", "Paja", "Cañizo"];

/// The materials recommended for the piece `param` now of `current`:
/// first what each of the eight collections gives it, then the palette's
/// of its kind (woods for a wood, metals for a metal…). Those not yet in
/// the project are added when chosen.
pub(super) fn recommended(m: &Model, param: &str, current: MaterialId) -> Vec<MatCard> {
    let Some(now) = m.material(current) else {
        return Vec::new();
    };
    let (name, cat) = (now.name.clone(), now.category.clone());
    let mut names: Vec<&'static str> = Vec::new();
    for c in archisim_library::collections::COLLECTIONS {
        if let Some(n) = archisim_library::collections::collection_material(param, &name, &cat, c) {
            if !names.contains(&n) {
                names.push(n);
            }
        }
    }
    // Of its kind, but not what is woven or bark for a board (nor a board
    // for what is woven).
    let odd = |n: &str| WOVEN.contains(&n) || n.starts_with("Corteza");
    for (n, category, _) in archisim_library::COZY_PALETTE {
        if *category == cat && !names.contains(n) && odd(n) == odd(&name) {
            names.push(n);
        }
    }
    // And the project's own of its kind (bricks for a brick, insulations
    // for an insulation…).
    let own: Vec<String> = m
        .materials()
        .filter(|x| x.category == cat && !names.contains(&x.name.as_str()))
        .filter(|x| odd(&x.name) == odd(&name))
        .map(|x| x.name.clone())
        .collect();
    let mut out: Vec<MatCard> = Vec::new();
    // The one it has now, first, when it is not among them.
    if !names.contains(&name.as_str()) {
        out.extend(project_card(m, current));
    }
    for n in names {
        let card = match m.materials().find(|x| x.name == n) {
            Some(x) => project_card(m, x.id),
            None => archisim_library::COZY_PALETTE
                .iter()
                .find(|e| e.0 == n)
                .map(|(n, category, rgb)| MatCard {
                    pick: MatPick::Palette(n),
                    name: n.to_string(),
                    rgb: *rgb,
                    texture: TextureKind::suggested(n, category).map(|t| (t, t.default_scale())),
                }),
        };
        out.extend(card);
    }
    for n in own {
        if n == name {
            continue;
        }
        if let Some(x) = m.materials().find(|x| x.name == n) {
            out.extend(project_card(m, x.id));
        }
    }
    out
}

/// Every material of the project not in `shown`.
pub(super) fn others(m: &Model, shown: &[MatCard]) -> Vec<MatCard> {
    let mut out: Vec<MatCard> = m
        .materials()
        .filter(|x| !shown.iter().any(|c| c.pick == MatPick::Project(x.id.0)))
        .filter_map(|x| project_card(m, x.id))
        .collect();
    out.sort_by_key(|c| c.name.to_lowercase());
    out
}

/// The pictures of the swatches, made once each.
#[derive(Default)]
pub struct Swatches {
    cache: HashMap<String, TextureHandle>,
}

/// The pixels of a swatch along its side.
const SWATCH_PX: usize = 64;

impl Swatches {
    /// The picture of a material's swatch: its colour with its texture,
    /// as a patch some 40 cm across.
    pub(super) fn get(&mut self, ctx: &egui::Context, c: &MatCard) -> TextureId {
        let key = format!(
            "{:?}{:?}{:?}",
            c.rgb,
            c.texture.map(|t| t.0),
            c.texture.map(|t| (t.1 * 1000.0) as i64)
        );
        if let Some(t) = self.cache.get(&key) {
            return t.id();
        }
        let img = swatch_image(c.rgb, c.texture);
        let tex = ctx.load_texture(format!("swatch:{key}"), img, TextureOptions::LINEAR);
        let id = tex.id();
        self.cache.insert(key, tex);
        id
    }
}

impl Swatches {
    /// A picture made once by `build`, kept under `key`.
    pub(super) fn image(
        &mut self,
        ctx: &egui::Context,
        key: &str,
        build: impl FnOnce() -> ColorImage,
    ) -> TextureId {
        if let Some(t) = self.cache.get(key) {
            return t.id();
        }
        let tex = ctx.load_texture(format!("swatch:{key}"), build(), TextureOptions::LINEAR);
        let id = tex.id();
        self.cache.insert(key.to_string(), tex);
        id
    }
}

/// A material's cut pattern drawn over its cut colour: what a wall or
/// slab of it looks like cut (E2).
pub fn pattern_image(p: archisim_core::CutPattern, rgb: [u8; 3]) -> ColorImage {
    use archisim_core::CutPattern as C;
    let n = SWATCH_PX;
    let fill = match p {
        C::Empty => Color32::WHITE,
        _ => Color32::from_rgb(rgb[0], rgb[1], rgb[2]),
    };
    let ink = Color32::from_gray(40);
    let mut pixels = vec![fill; n * n];
    for j in 0..n {
        for i in 0..n {
            let (x, y) = (i as i64, j as i64);
            let on = match p {
                C::Solid => false,
                C::Diagonal => (x + y).rem_euclid(10) == 0,
                C::Cross => (x + y).rem_euclid(10) == 0 || (x - y).rem_euclid(10) == 0,
                C::Insulation => {
                    // A zigzag every 16 rows.
                    let k = y.rem_euclid(16);
                    let t = x.rem_euclid(16);
                    let z = if t < 8 { t } else { 16 - t };
                    (k - 4 - z).abs() == 0
                }
                C::Dots => x.rem_euclid(6) == 2 && y.rem_euclid(6) == 2,
                C::Empty => {
                    (x == 0 || y == 0 || x == n as i64 - 1 || y == n as i64 - 1)
                        && (x + y).rem_euclid(8) < 4
                }
            };
            if on {
                pixels[j * n + i] = ink;
            }
        }
    }
    ColorImage::new([n, n], pixels)
}

/// A square of `rgb` with its texture, 40 cm a side.
pub fn swatch_image(rgb: [u8; 3], texture: Option<(TextureKind, f64)>) -> ColorImage {
    let base = rgb.map(|v| v as f64 / 255.0);
    let tile = texture.map(|(k, s)| (archisim_render::textures::tile(k), s));
    let n = SWATCH_PX;
    let mut pixels = Vec::with_capacity(n * n);
    let span = 0.4;
    for j in 0..n {
        for i in 0..n {
            let c = match &tile {
                Some((t, scale)) => {
                    let p = [
                        (i as f64 + 0.5) / n as f64 * span / scale,
                        (1.0 - (j as f64 + 0.5) / n as f64) * span / scale,
                    ];
                    t.apply(base, p)
                }
                None => base,
            };
            let u = |v: f64| (v.clamp(0.0, 1.0) * 255.0).round() as u8;
            pixels.push(Color32::from_rgb(u(c[0]), u(c[1]), u(c[2])));
        }
    }
    ColorImage::new([n, n], pixels)
}

/// Paints a swatch in `r`, rounded.
fn paint_swatch(ui: &Ui, r: Rect, tex: TextureId) {
    ui.painter().add(
        egui::epaint::RectShape::filled(r, CornerRadius::same(5), Color32::WHITE)
            .with_texture(tex, Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0))),
    );
    ui.painter().rect_stroke(
        r,
        CornerRadius::same(5),
        Stroke::new(1.0, Color32::from_black_alpha(50)),
        StrokeKind::Inside,
    );
}

/// A material of the project chosen by its swatch and the materials as
/// cards (for tables and rows with no room for more: the layers of a
/// wall, the material of an edge). Only the project's are offered.
pub(super) fn material_button(
    ui: &mut Ui,
    id: egui::Id,
    m: &Model,
    value: &mut MaterialId,
    width: f32,
) {
    // The pictures of the swatches, kept for the whole window.
    let store = ui.ctx().data_mut(|d| {
        d.get_temp_mut_or_default::<std::sync::Arc<std::sync::Mutex<Swatches>>>(egui::Id::unique(
            "material-swatches",
        ))
        .clone()
    });
    let Ok(mut swatches) = store.lock() else {
        return;
    };
    let Some(now) = project_card(m, *value) else {
        return;
    };
    let tex = swatches.get(ui.ctx(), &now);
    let button = ui
        .push_id(id, |ui| {
            current_button_small(ui, width, &now.name, |ui, r| paint_swatch(ui, r, tex))
        })
        .inner;
    egui::Popup::menu(&button)
        .width(320.0)
        .close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside)
        .show(|ui| {
            let rec: Vec<MatCard> = recommended(m, "", *value)
                .into_iter()
                .filter(|c| matches!(c.pick, MatPick::Project(_)))
                .collect();
            let oth = others(m, &rec);
            let salt = format!("{id:?}");
            if let Some(MatPick::Project(new)) =
                material_panel(ui, &salt, &mut swatches, &rec, &oth, &now.pick)
            {
                *value = MaterialId(new);
                ui.close();
            }
        });
}

/// The chosen material as a slim button: its swatch and name.
fn current_button_small(
    ui: &mut Ui,
    width: f32,
    name: &str,
    picture: impl FnOnce(&Ui, Rect),
) -> egui::Response {
    let p = Palette::current(ui.ctx());
    let (rect, resp) = ui.allocate_exact_size(Vec2::new(width, 24.0), Sense::click());
    let stroke = if resp.hovered() { p.accent } else { p.line };
    ui.painter().rect(
        rect,
        CornerRadius::same(4),
        p.panel,
        Stroke::new(1.0, stroke),
        StrokeKind::Inside,
    );
    let pic = Rect::from_min_size(rect.min + Vec2::splat(3.0), Vec2::splat(18.0));
    picture(ui, pic);
    let galley =
        ui.painter()
            .layout_no_wrap(name.to_string(), egui::FontId::proportional(12.0), p.text);
    let clip = Rect::from_min_max(
        Pos2::new(pic.max.x + 6.0, rect.min.y),
        Pos2::new(rect.max.x - 14.0, rect.max.y),
    );
    ui.painter().with_clip_rect(clip).galley(
        Pos2::new(clip.min.x, rect.center().y - galley.size().y / 2.0),
        galley,
        p.text,
    );
    let c = Pos2::new(rect.max.x - 8.0, rect.center().y);
    ui.painter().line_segment(
        [c + Vec2::new(-3.0, -1.5), c + Vec2::new(0.0, 1.5)],
        Stroke::new(1.2, p.muted),
    );
    ui.painter().line_segment(
        [c + Vec2::new(0.0, 1.5), c + Vec2::new(3.0, -1.5)],
        Stroke::new(1.2, p.muted),
    );
    resp.on_hover_text(name)
}

/// The materials as cards, `per_row` in a row; the one clicked.
fn material_grid(
    ui: &mut Ui,
    swatches: &mut Swatches,
    cards: &[MatCard],
    current: &MatPick,
    per_row: usize,
) -> Option<MatPick> {
    let p = Palette::current(ui.ctx());
    let mut picked = None;
    for chunk in cards.chunks(per_row.max(1)) {
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 6.0;
            for c in chunk {
                let tex = swatches.get(ui.ctx(), c);
                let on = &c.pick == current;
                let (rect, resp) = ui.allocate_exact_size(Vec2::new(70.0, 82.0), Sense::click());
                let resp = resp.on_hover_text(&c.name);
                if on || resp.hovered() {
                    ui.painter().rect(
                        rect,
                        CornerRadius::same(6),
                        if on { p.accent_soft } else { p.raised },
                        Stroke::new(if on { 1.5 } else { 1.0 }, p.accent),
                        StrokeKind::Inside,
                    );
                }
                let sw = Rect::from_min_size(
                    Pos2::new(rect.center().x - 26.0, rect.min.y + 5.0),
                    Vec2::splat(52.0),
                );
                paint_swatch(ui, sw, tex);
                let galley = ui.painter().layout(
                    c.name.clone(),
                    egui::FontId::proportional(10.0),
                    if on { p.accent } else { p.text },
                    rect.width() - 4.0,
                );
                ui.painter().galley(
                    Pos2::new(rect.center().x - galley.size().x / 2.0, sw.max.y + 2.0),
                    galley,
                    p.text,
                );
                if resp.clicked() {
                    picked = Some(c.pick.clone());
                }
            }
        });
        ui.add_space(3.0);
    }
    picked
}

/// The materials offered for a piece: the recommended first, as cards,
/// then the others of the project, with a search box over both.
pub(super) fn material_panel(
    ui: &mut Ui,
    salt: &str,
    swatches: &mut Swatches,
    recommended: &[MatCard],
    others: &[MatCard],
    current: &MatPick,
) -> Option<MatPick> {
    let p = Palette::current(ui.ctx());
    let search_id = ui.make_persistent_id(("material-search", salt));
    let mut query: String = ui.ctx().data(|d| d.get_temp(search_id)).unwrap_or_default();
    ui.add(
        egui::TextEdit::singleline(&mut query)
            .hint_text("Buscar material…")
            .desired_width(f32::INFINITY),
    );
    ui.ctx()
        .data_mut(|d| d.insert_temp(search_id, query.clone()));
    let q = query.to_lowercase();
    let keep = |c: &&MatCard| q.is_empty() || c.name.to_lowercase().contains(&q);
    let rec: Vec<MatCard> = recommended.iter().filter(keep).cloned().collect();
    let oth: Vec<MatCard> = others.iter().filter(keep).cloned().collect();
    let mut picked = None;
    egui::ScrollArea::vertical()
        .max_height(440.0)
        .show(ui, |ui| {
            if !rec.is_empty() {
                ui.label(
                    egui::RichText::new("Recomendados para esta pieza")
                        .small()
                        .strong()
                        .color(p.muted),
                );
                ui.add_space(2.0);
                picked = picked
                    .take()
                    .or(material_grid(ui, swatches, &rec, current, 4));
            }
            if !oth.is_empty() {
                ui.add_space(4.0);
                ui.label(
                    egui::RichText::new("Otros materiales del proyecto")
                        .small()
                        .strong()
                        .color(p.muted),
                );
                ui.add_space(2.0);
                picked = picked
                    .take()
                    .or(material_grid(ui, swatches, &oth, current, 4));
            }
        });
    picked
}

/// A material shown as its swatch and name; a click opens the materials
/// as cards. The material picked.
pub(super) fn material_cards(
    ui: &mut Ui,
    salt: &str,
    width: f32,
    swatches: &mut Swatches,
    m: &Model,
    param: &str,
    current: MaterialId,
) -> Option<MatPick> {
    let now = project_card(m, current)?;
    let tex = swatches.get(ui.ctx(), &now);
    let button = current_button(ui, width, &now.name, |ui, r| {
        let s = Rect::from_center_size(r.center(), Vec2::splat(r.height()));
        paint_swatch(ui, s, tex);
    });
    let mut picked = None;
    egui::Popup::menu(&button)
        .width(320.0)
        .close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside)
        .show(|ui| {
            let rec = recommended(m, param, current);
            let oth = others(m, &rec);
            picked = material_panel(ui, salt, swatches, &rec, &oth, &now.pick);
            if picked.is_some() {
                ui.close();
            }
        });
    picked
}

impl super::ArchiSimApp {
    /// The material of a pick: one of the project's, or one of the
    /// palette, added to the project now (in its own step; join it with
    /// what uses it).
    pub(super) fn picked_material(&mut self, pick: &MatPick) -> Option<u64> {
        match pick {
            MatPick::Project(id) => Some(*id),
            MatPick::Palette(name) => {
                let name = name.to_string();
                let found = |m: &Model| m.materials().find(|x| x.name == name).map(|x| x.id.0);
                if let Some(id) = found(self.ed.model()) {
                    return Some(id);
                }
                let n = name.clone();
                let r = self.ed.transact("Material de la paleta", move |m| {
                    archisim_library::palette_material(m, &n)
                        .map(|_| ())
                        .ok_or_else(|| archisim_core::ModelError::NotAnObject(n.clone()))
                });
                if let Err(e) = r {
                    self.complain(format!("No se pudo añadir el material: {e}."));
                    return None;
                }
                found(self.ed.model())
            }
        }
    }

    /// The picture of an object type with `key` set to `option`: in 3D,
    /// or in plan for a choice of how it is drawn in plan (E2).
    pub(super) fn object_option_thumb(
        &mut self,
        ctx: &egui::Context,
        t: &archisim_core::ObjectType,
        key: &str,
        option: &str,
    ) -> Option<Thumb> {
        let mut ty = t.clone();
        ty.values.insert(
            key.to_string(),
            archisim_core::Value::Text(option.to_string()),
        );
        if plan_choice(key) {
            let k = format!("opt-plan:{}:{:?}", ty.family.0, ty.values);
            let model = self.ed.model();
            return self.thumbs.get(ctx, k, || {
                crate::type_preview::object_model_with(model, &ty, Some(Default::default()))
            });
        }
        let sig = self.picture_signature(false);
        let k = format!("opt:{}:{:?}:{sig}", ty.family.0, ty.values);
        let model = self.ed.model();
        self.thumbs3d.get(ctx, self.gpu.as_ref(), k, || {
            crate::type_preview::object_model(model, &ty)
        })
    }

    /// The picture in plan of an object with its own value `key` set to
    /// `option` (its type's values for the rest).
    pub(super) fn object_instance_option_thumb(
        &mut self,
        ctx: &egui::Context,
        t: &archisim_core::ObjectType,
        own: &archisim_core::Values,
        key: &str,
        option: &str,
    ) -> Option<Thumb> {
        let mut own = own.clone();
        own.insert(
            key.to_string(),
            archisim_core::Value::Text(option.to_string()),
        );
        let k = format!("opt-own:{}:{:?}:{:?}", t.family.0, t.values, own);
        let model = self.ed.model();
        self.thumbs.get(ctx, k, || {
            crate::type_preview::object_model_with(model, t, Some(own))
        })
    }

    /// The picture of a door or window type with `key` set to `option`:
    /// in 3D when ArchiSim models its leaves (the door's design, its
    /// casings and handle only show there), in elevation otherwise.
    pub(super) fn opening_option_thumb(
        &mut self,
        ctx: &egui::Context,
        t: &archisim_core::OpeningType,
        key: &str,
        option: &str,
    ) -> Option<Thumb> {
        let mut ty = t.clone();
        ty.values.insert(
            key.to_string(),
            archisim_core::Value::Text(option.to_string()),
        );
        let model = self.ed.model();
        if model.family_prop(ty.family).is_some() {
            // What goes outside a window, seen from the street.
            let outside = matches!(
                key,
                "grille" | "grille_box" | "outside_shutters" | "shutters_open"
            );
            let sig = self.picture_signature(false);
            let k = format!("opt3d:{}:{:?}:{sig}:{outside}", ty.family.0, ty.values);
            let model = self.ed.model();
            let front = if outside {
                super::thumbs3d::OUTSIDE
            } else {
                super::thumbs3d::FRONT
            };
            return self
                .thumbs3d
                .get_seen(ctx, self.gpu.as_ref(), k, front, || {
                    crate::type_preview::opening_model_3d(model, &ty)
                });
        }
        let k = format!("opt-elev:{}:{:?}", ty.family.0, ty.values);
        self.thumbs.get_drawing(ctx, k, PICTURE, || {
            crate::type_drawing::opening_elevation(model, &ty)
        })
    }
}

impl super::ArchiSimApp {
    /// A row for a type parameter of an object: a choice as cards drawn
    /// in 3D, a material as its swatch and the materials as cards, the
    /// rest as [`super::object_ui::param_row`]. The new value.
    pub(super) fn object_type_param_row(
        &mut self,
        ui: &mut Ui,
        salt: &str,
        p: &archisim_core::family::Param,
        cur: &archisim_core::Value,
        t: &archisim_core::ObjectType,
        materials: &[(u64, String)],
    ) -> Option<archisim_core::Value> {
        use archisim_core::{ParamKind, Value};
        let label = if p.label.is_empty() {
            p.name.clone()
        } else {
            p.label.clone()
        };
        match (&p.kind, cur) {
            (ParamKind::Choice(options), Value::Text(now)) => {
                let ctx = ui.ctx().clone();
                let mut out = None;
                card_row(ui, &label, |ui, w| {
                    let mut thumb = |o: &str| self.object_option_thumb(&ctx, t, &p.name, o);
                    out = option_cards(ui, w - 8.0, options, now, &mut thumb);
                });
                out.filter(|o| o != now).map(Value::Text)
            }
            (ParamKind::Material, Value::Material(mid)) => self
                .material_param_row(ui, salt, &label, &p.name, MaterialId(*mid))
                .map(Value::Material),
            _ => super::object_ui::param_row(ui, salt, p, cur, materials),
        }
    }

    /// A row for a material parameter: its swatch, and the materials as
    /// cards. The material chosen (a palette's is added to the project).
    pub(super) fn material_param_row(
        &mut self,
        ui: &mut Ui,
        salt: &str,
        label: &str,
        param: &str,
        now: MaterialId,
    ) -> Option<u64> {
        let mut pick = None;
        card_row(ui, label, |ui, w| {
            pick = material_cards(
                ui,
                salt,
                w - 8.0,
                &mut self.swatches,
                self.ed.model(),
                param,
                now,
            );
        });
        let pick = pick.filter(|p| *p != MatPick::Project(now.0))?;
        self.picked_material(&pick).filter(|id| *id != now.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// E2: the cut patterns are drawn, each its own picture.
    #[test]
    fn cut_patterns_are_drawn() {
        use archisim_core::CutPattern as C;
        let pics: Vec<ColorImage> = [
            C::Solid,
            C::Diagonal,
            C::Cross,
            C::Insulation,
            C::Dots,
            C::Empty,
        ]
        .iter()
        .map(|p| pattern_image(*p, [200, 120, 90]))
        .collect();
        for (i, a) in pics.iter().enumerate() {
            for b in &pics[i + 1..] {
                assert_ne!(a.pixels, b.pixels);
            }
        }
        assert!(plan_choice("shape_own") && !plan_choice("model3d"));
    }

    /// A wood's swatch shows its grain: not all of one colour.
    #[test]
    fn swatches_show_their_texture() {
        let plain = swatch_image([180, 140, 100], None);
        assert!(plain.pixels.iter().all(|p| *p == plain.pixels[0]));
        let wood = swatch_image([180, 140, 100], Some((TextureKind::Planks, 0.2)));
        let distinct: std::collections::BTreeSet<[u8; 4]> =
            wood.pixels.iter().map(|p| p.to_array()).collect();
        assert!(distinct.len() > 20, "{}", distinct.len());
    }

    /// A fabric is offered fabrics, of the project and of the palette;
    /// the others are the rest of the project's.
    #[test]
    fn fabrics_are_recommended_for_fabric() {
        let mut m = Model::new("Prueba");
        let fabric = m.add_material(
            "Lino propio",
            "fabric",
            [200, 190, 170],
            archisim_core::CutPattern::Solid,
        );
        m.add_material(
            "Pino",
            "wood",
            [200, 170, 120],
            archisim_core::CutPattern::Solid,
        );
        let rec = recommended(&m, "mat", fabric);
        assert!(!rec.is_empty());
        // First what the collections give the upholstery.
        assert_eq!(rec[0].name, "Lino propio");
        assert_eq!(
            rec[1].name,
            archisim_library::collections::COLLECTIONS[0].fabric
        );
        assert!(rec.iter().any(|c| matches!(c.pick, MatPick::Palette(_))));
        for c in &rec {
            if let MatPick::Project(id) = c.pick {
                assert_eq!(m.material(MaterialId(id)).unwrap().category, "fabric");
            }
        }
        let oth = others(&m, &rec);
        assert_eq!(
            oth.len()
                + rec
                    .iter()
                    .filter(|c| matches!(c.pick, MatPick::Project(_)))
                    .count(),
            m.materials().count()
        );
    }
}
