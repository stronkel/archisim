//! The bar of what is selected in the Maqueta (concepto de interfaz del
//! 24/10/2026, as in Paralives): at the bottom left, over the catalogue,
//! a card with its picture and its name, then what can be done with it —
//! turn, flip, duplicate, take its type, delete — and its materials, each
//! a round swatch that opens the colours of the project above it. It
//! replaces the card that floated next to the selection (it covered what
//! it described) and the keys over the drawing: everything is a button.

use archisim_core::{ObjectId, Value};
use eframe::egui::{self, Color32, CornerRadius, Id, Pos2, Rect, Sense, Stroke, Vec2};

use super::glyphs::{self, Glyph};
use super::icons::Icon;
use super::theme::Palette;
use super::toolbox::Mark;
use super::*;

/// What a button of the bar asked for.
#[derive(Debug, Clone, PartialEq)]
enum BarAct {
    Turn,
    Flip,
    Duplicate,
    Eyedropper,
    Delete,
    More,
    /// Open (or close) the colours of a material of the object.
    Swatches(String),
    /// Give a material of the object this one.
    SetMaterial(String, u64),
    /// Give a piece of the object a colour of the cosy palette.
    PaintPiece(String, &'static str),
    /// Give a value to a measure or choice of its type (to its whole row
    /// of modules with `true`).
    SetValue(String, Value, bool),
}

/// A count or a choice of the object's type the bar offers: how many
/// drawers, its handle, its front…
#[derive(Debug, Clone)]
enum Control {
    /// A small whole number, as buttons (0 1 2 3 4).
    Count {
        key: String,
        label: String,
        lo: i64,
        hi: i64,
        now: i64,
    },
    /// One of a list, in a popover.
    Choice {
        key: String,
        label: String,
        options: Vec<String>,
        now: String,
    },
}

/// Size of the picture card.
const CARD: f32 = 76.0;

impl ArchiSimApp {
    /// The materials an object can change: the material parameters of
    /// its family, with their name, what they are now and its colour.
    pub(super) fn object_materials(&self, id: ObjectId) -> Vec<(String, String, u64, [u8; 3])> {
        let model = self.ed.model();
        let Some(o) = model.object(id) else {
            return Vec::new();
        };
        let Some(t) = model.object_type(o.object_type) else {
            return Vec::new();
        };
        let Some(def) = model.family(t.family).map(|f| f.def().clone()) else {
            return Vec::new();
        };
        def.params_of(archisim_family::ParamScope::Type)
            .filter(|p| p.kind == archisim_family::ParamKind::Material)
            .filter(|p| def.is_visible(&p.name, &t.values))
            .filter_map(|p| match t.values.get(&p.name) {
                Some(Value::Material(m)) => Some((
                    p.name.clone(),
                    p.label.clone(),
                    *m,
                    model.surface_color(archisim_core::MaterialId(*m)),
                )),
                _ => None,
            })
            .collect()
    }

    /// The counts and choices of an object's type that its bar offers (not
    /// its model, which the catalogue chooses).
    fn object_controls(&self, id: ObjectId) -> Vec<Control> {
        let model = self.ed.model();
        let Some(t) = model
            .object(id)
            .and_then(|o| model.object_type(o.object_type))
        else {
            return Vec::new();
        };
        let Some(def) = model.family(t.family).map(|f| f.def().clone()) else {
            return Vec::new();
        };
        let defaults = model
            .family_defaults(t.family, archisim_family::ParamScope::Type)
            .unwrap_or_default();
        let value = |k: &str| t.values.get(k).or(defaults.get(k)).cloned();
        let mut out = Vec::new();
        for p in def.params_of(archisim_family::ParamScope::Type) {
            if p.name == "model3d" || !def.is_visible(&p.name, &t.values) {
                continue;
            }
            match (&p.kind, value(&p.name)) {
                (archisim_family::ParamKind::Integer, Some(Value::Number(n))) => {
                    let (lo, hi) = p.range.unwrap_or((0.0, 0.0));
                    if hi - lo >= 1.0 && hi - lo <= 5.0 {
                        out.push(Control::Count {
                            key: p.name.clone(),
                            label: p.label.clone(),
                            lo: lo as i64,
                            hi: hi as i64,
                            now: n.round() as i64,
                        });
                    }
                }
                (archisim_family::ParamKind::Choice(options), Some(Value::Text(now))) => {
                    out.push(Control::Choice {
                        key: p.name.clone(),
                        label: p.label.clone(),
                        options: options.clone(),
                        now,
                    });
                }
                _ => {}
            }
        }
        out
    }

    /// The objects in a row with `id`: of its family, turned the same
    /// way, their backs on one line and touching one after another (a run
    /// of kitchen modules).
    pub(super) fn row_of(&self, id: ObjectId) -> Vec<ObjectId> {
        let m = self.ed.model();
        let Some(o) = m.object(id) else {
            return Vec::new();
        };
        let Some(family) = m.object_type(o.object_type).map(|t| t.family) else {
            return vec![id];
        };
        let (c, s) = (o.rotation.cos(), o.rotation.sin());
        let along = |p: archisim_core::Point2| (p - o.position).x * c + (p - o.position).y * s;
        let back = |p: archisim_core::Point2| -(p - o.position).x * s + (p - o.position).y * c;
        let mut row: Vec<(f64, f64, ObjectId)> = m
            .objects()
            .filter(|x| x.level == o.level)
            .filter(|x| {
                m.object_type(x.object_type)
                    .is_some_and(|t| t.family == family)
            })
            .filter(|x| {
                let turn = (x.rotation - o.rotation).rem_euclid(std::f64::consts::TAU);
                turn.min(std::f64::consts::TAU - turn) < 0.02 && back(x.position).abs() < 0.03
            })
            .filter_map(|x| {
                let (lo, hi) = crate::drag::object_box(m, x.id)?;
                let a = along(x.position);
                Some((a + lo[0], a + hi[0], x.id))
            })
            .collect();
        row.sort_by(|a, b| a.0.total_cmp(&b.0));
        // The run of touching ones that holds `id`.
        let Some(at) = row.iter().position(|r| r.2 == id) else {
            return vec![id];
        };
        let (mut a, mut b) = (at, at);
        while a > 0 && (row[a].0 - row[a - 1].1).abs() < 0.02 {
            a -= 1;
        }
        while b + 1 < row.len() && (row[b + 1].0 - row[b].1).abs() < 0.02 {
            b += 1;
        }
        row[a..=b].iter().map(|r| r.2).collect()
    }

    /// What the bar does when one of its counts or choices is picked.
    #[cfg(test)]
    pub(super) fn bar_set_value_pub(&mut self, key: &str, v: Value, whole_row: bool) {
        self.bar_do(BarAct::SetValue(key.to_string(), v, whole_row));
    }

    /// The bar of the selection, at the bottom left over the catalogue.
    pub(super) fn object_bar(&mut self, ctx: &egui::Context, bottom: f32) {
        if self.ed.selection.is_empty() || self.tool != Tool::Arrow {
            self.bar_swatches = None;
            return;
        }
        let p = Palette::current(ctx);
        let sel: Vec<Element> = self.ed.selection.iter().copied().collect();
        let single = (sel.len() == 1).then(|| sel[0]);
        let label = super::context_bar::selection_label(sel.iter());
        let title = single
            .map(|e| self.element_title(e))
            .filter(|t| !t.is_empty())
            .unwrap_or(label);
        let flips = matches!(single, Some(Element::Opening(_)) | Some(Element::Wall(_)));
        let object = match single {
            Some(Element::Object(id)) => Some(id),
            _ => None,
        };
        let materials = object
            .map(|id| self.object_materials(id))
            .unwrap_or_default();
        let controls = object
            .map(|id| self.object_controls(id))
            .unwrap_or_default();
        let thumb = single.and_then(|e| self.bar_thumb(ctx, e));
        let mut act = None;
        let origin = Pos2::new(16.0, bottom - 16.0);
        let area = egui::Area::new(Id::unique("object-bar"))
            .order(egui::Order::Foreground)
            .pivot(egui::Align2::LEFT_BOTTOM)
            .fixed_pos(origin)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 0.0;
                    // The card: the picture, and the name over its corner.
                    let (card, _) = ui.allocate_exact_size(Vec2::splat(CARD), Sense::hover());
                    let painter = ui.painter();
                    painter.rect(
                        card,
                        CornerRadius::same(14),
                        p.panel,
                        Stroke::new(1.0, p.line),
                        egui::StrokeKind::Inside,
                    );
                    match thumb {
                        Some(t) => {
                            let size = t.fit(Vec2::splat(CARD - 16.0));
                            painter.image(
                                t.tex,
                                Rect::from_center_size(card.center(), size),
                                Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)),
                                Color32::WHITE,
                            );
                        }
                        None => {
                            let g = single.map_or(Glyph::Object, glyph_of);
                            glyphs::paint(painter, card.shrink(20.0), g, p.text);
                        }
                    }
                    // The bar, joined to the card.
                    egui::Frame::new()
                        .fill(p.panel)
                        .stroke(Stroke::new(1.0, p.line))
                        .corner_radius(CornerRadius {
                            nw: 0,
                            sw: 0,
                            ne: 14,
                            se: 14,
                        })
                        .inner_margin(egui::Margin {
                            left: 8,
                            right: 10,
                            top: 4,
                            bottom: 4,
                        })
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                ui.spacing_mut().item_spacing.x = 2.0;
                                let mut b = |ui: &mut Ui, mark, name: &str, hint: &str, a: BarAct| {
                                    if bar_tool(ui, mark, name, false, p).on_hover_text(hint).clicked() {
                                        act = Some(a);
                                    }
                                };
                                b(ui, Mark::Icon(Icon::Rotate), "Girar", "Un cuarto de vuelta (o el punto naranja, a mano)", BarAct::Turn);
                                if flips {
                                    b(ui, Mark::Icon(Icon::Mirror), "Invertir", "Abrir hacia el otro lado o con la otra mano (un muro: su cara exterior al otro lado)", BarAct::Flip);
                                }
                                b(ui, Mark::Icon(Icon::Copy), "Duplicar", "Una copia al lado, seleccionada", BarAct::Duplicate);
                                if single.is_some() {
                                    b(ui, Mark::Glyph(Glyph::Paint), "Igualar", "La herramienta para poner otro igual (cuentagotas)", BarAct::Eyedropper);
                                }
                                b(ui, Mark::Icon(Icon::Close), "Borrar", "Quitarlo (Supr)", BarAct::Delete);
                                if !controls.is_empty() {
                                    bar_gap(ui, p);
                                    for c in &controls {
                                        match c {
                                            Control::Count { key, label, lo, hi, now } => {
                                                if let Some(n) = count_buttons(ui, label, *lo, *hi, *now, p) {
                                                    act = Some(BarAct::SetValue(key.clone(), Value::Number(n as f64), false));
                                                }
                                            }
                                            Control::Choice { key, label, now, .. } => {
                                                let k = format!("choice:{key}");
                                                let open = self.bar_swatches.as_deref() == Some(k.as_str());
                                                let r = choice_button(ui, label, now, open, p);
                                                if r.clicked() {
                                                    act = Some(BarAct::Swatches(k));
                                                }
                                                if open {
                                                    self.bar_swatch_at = Some(r.rect);
                                                }
                                            }
                                        }
                                    }
                                }
                                if !materials.is_empty() {
                                    bar_gap(ui, p);
                                    for (key, name, _, rgb) in &materials {
                                        let open = self.bar_swatches.as_deref() == Some(key.as_str());
                                        let r = swatch_button(ui, *rgb, short(name), open, p)
                                            .on_hover_text(format!("{name}: cambiar el material (solo este objeto)"));
                                        if r.clicked() {
                                            act = Some(BarAct::Swatches(key.clone()));
                                        }
                                        if open {
                                            self.bar_swatch_at = Some(r.rect);
                                        }
                                    }
                                }
                                bar_gap(ui, p);
                                if ui
                                    .add(egui::Button::new(
                                        egui::RichText::new("Más…").color(p.accent),
                                    ).frame(false))
                                    .on_hover_text("Todas sus propiedades")
                                    .clicked()
                                {
                                    act = Some(BarAct::More);
                                }
                            });
                        });
                });
            });
        // The name, as a label over the card's top right corner.
        let card = Rect::from_min_size(area.response.rect.min, Vec2::splat(CARD));
        let painter = ctx.layer_painter(egui::LayerId::new(
            egui::Order::Tooltip,
            Id::unique("object-bar-name"),
        ));
        let font = egui::FontId::proportional(13.0);
        let galley = painter.layout_no_wrap(title, font, Color32::from_rgb(35, 38, 43));
        let tag = Rect::from_min_size(
            Pos2::new(card.right() - 14.0, card.top() - 14.0),
            galley.size(),
        )
        .expand2(Vec2::new(9.0, 4.0));
        painter.rect_filled(tag, CornerRadius::same(7), Color32::from_rgb(255, 243, 196));
        painter.galley(
            tag.min + Vec2::new(9.0, 4.0),
            galley,
            Color32::from_rgb(35, 38, 43),
        );

        // The colours of the material open, or the options of a choice,
        // above its button.
        if let (Some(key), Some(at), Some(id)) =
            (self.bar_swatches.clone(), self.bar_swatch_at, object)
        {
            let found = match key.strip_prefix("choice:") {
                Some(name) => controls.iter().find_map(|c| match c {
                    Control::Choice {
                        key,
                        label,
                        options,
                        now,
                    } if key == name => {
                        Some(self.choice_options(ctx, key, label, options, now, at, id))
                    }
                    _ => None,
                }),
                None => Some(self.material_swatches(ctx, &key, at, id)),
            };
            if let Some(Some(a)) = found {
                act = Some(a);
            }
        }
        if let Some(a) = act {
            self.bar_do(a);
        }
    }

    /// Does what a button of the bar asked for.
    fn bar_do(&mut self, act: BarAct) {
        let single =
            (self.ed.selection.len() == 1).then(|| *self.ed.selection.iter().next().expect("one"));
        let object = match single {
            Some(Element::Object(id)) => Some(id),
            _ => None,
        };
        match act {
            BarAct::Turn => self.card_turn_pub(),
            BarAct::Flip => {
                self.flip_selected(None);
            }
            BarAct::Duplicate => self.card_duplicate_pub(),
            BarAct::Eyedropper => {
                if let Some(e) = single {
                    self.eyedropper(e);
                }
            }
            BarAct::Delete => self.delete_pub(),
            BarAct::More => self.maquette_props = true,
            BarAct::Swatches(key) => {
                self.bar_swatches = if self.bar_swatches.as_deref() == Some(key.as_str()) {
                    None
                } else {
                    Some(key)
                };
            }
            BarAct::PaintPiece(key, swatch) => {
                if let Some(id) = object {
                    let r = self.ed.paint_object_piece(id, &key, swatch);
                    self.report(r, &format!("{swatch}."));
                }
            }
            BarAct::SetValue(key, v, whole_row) => {
                if let Some(id) = object {
                    let ids = if whole_row { self.row_of(id) } else { vec![id] };
                    let n = ids.len();
                    let r = self.ed.transact("Cambiar el mueble", |model| {
                        for i in &ids {
                            crate::drag::set_object_value(model, *i, &key, v.clone())?;
                        }
                        Ok(())
                    });
                    if !whole_row {
                        self.bar_swatches = None;
                    }
                    self.report(
                        r,
                        &if n > 1 {
                            format!("Cambiados los {n} módulos de la fila.")
                        } else {
                            "Cambiado.".to_string()
                        },
                    );
                }
            }
            BarAct::SetMaterial(key, m) => {
                if let Some(id) = object {
                    let r = self.ed.transact("Cambiar material", |model| {
                        crate::drag::set_object_value(model, id, &key, Value::Material(m))
                    });
                    self.report(r, "Material cambiado.");
                }
            }
        }
    }

    /// A small picture of what is selected for the card.
    fn bar_thumb(&mut self, ctx: &egui::Context, e: Element) -> Option<super::thumbs::Thumb> {
        // An object: drawn in 3D, as the catalogue's cards.
        if let Element::Object(id) = e {
            let t = self.ed.model().object(id)?.object_type;
            let act = super::maquette_ui::CatalogAct::Object(
                super::object_gallery::ObjectPick::Project(t),
            );
            self.thumbs3d.begin();
            if let Some(th) = self.card_thumb_3d(ctx, act) {
                return Some(th);
            }
            if self.thumbs3d.pending {
                ctx.request_repaint();
            }
        }
        let sig = self.picture_signature(false);
        let model = self.ed.model();
        let thumbs = &mut self.thumbs;
        match e {
            Element::Object(id) => {
                let t = model.object_type(model.object(id)?.object_type)?;
                let key = format!("bar:object:{}:{:?}:{sig}", t.family.0, t.values);
                thumbs.get(ctx, key, || crate::type_preview::object_model(model, t))
            }
            Element::Opening(id) => {
                let t = model.opening_type(model.opening(id)?.opening_type)?;
                let key = format!("elev:{}:{:?}:{sig}", t.id.0, t.values);
                thumbs.get_drawing(ctx, key, Vec2::splat(CARD - 16.0), || {
                    crate::type_drawing::opening_elevation(model, t)
                })
            }
            _ => None,
        }
    }

    /// The options of a choice above its button at `at`: a click gives
    /// them to this object, or to its whole row.
    #[allow(clippy::too_many_arguments)]
    fn choice_options(
        &mut self,
        ctx: &egui::Context,
        key: &str,
        label: &str,
        options: &[String],
        now: &str,
        at: Rect,
        id: ObjectId,
    ) -> Option<BarAct> {
        let p = Palette::current(ctx);
        let row = self.row_of(id).len();
        let mut act = None;
        let area = egui::Area::new(Id::unique("object-bar-choice"))
            .order(egui::Order::Foreground)
            .pivot(egui::Align2::LEFT_BOTTOM)
            .fixed_pos(Pos2::new(at.left() - 8.0, at.top() - 10.0))
            .constrain(true)
            .show(ctx, |ui| {
                egui::Frame::popup(ui.style()).show(ui, |ui| {
                    ui.set_min_width(200.0);
                    ui.label(egui::RichText::new(label).strong().color(p.text));
                    for o in options {
                        let on = o == now;
                        let text = egui::RichText::new(o).color(if on { p.accent } else { p.text });
                        if ui
                            .add(
                                egui::Button::new(text)
                                    .frame(on)
                                    .min_size(Vec2::new(190.0, 24.0)),
                            )
                            .clicked()
                        {
                            act = Some(BarAct::SetValue(
                                key.to_string(),
                                Value::Text(o.clone()),
                                false,
                            ));
                        }
                    }
                    if row > 1 {
                        ui.separator();
                        if ui
                            .link(format!("«{now}» a toda la fila ({row})"))
                            .on_hover_text("Los módulos que se tocan con este, en línea")
                            .clicked()
                        {
                            act = Some(BarAct::SetValue(
                                key.to_string(),
                                Value::Text(now.to_string()),
                                true,
                            ));
                        }
                    }
                })
            });
        let outside = ctx.input(|i| {
            i.pointer.any_pressed()
                && i.pointer
                    .interact_pos()
                    .is_some_and(|q| !area.response.rect.contains(q) && !at.contains(q))
        });
        if outside && act.is_none() {
            self.bar_swatches = None;
        }
        act
    }

    /// The colours of the project's materials, above the swatch at `at`:
    /// a click gives the object that material.
    fn material_swatches(
        &mut self,
        ctx: &egui::Context,
        key: &str,
        at: Rect,
        id: ObjectId,
    ) -> Option<BarAct> {
        let p = Palette::current(ctx);
        let model = self.ed.model();
        let now = self
            .object_materials(id)
            .into_iter()
            .find(|m| m.0 == key)
            .map(|m| m.2);
        let mats: Vec<(u64, String, [u8; 3])> = model
            .materials()
            .map(|m| (m.id.0, m.name.clone(), model.surface_color(m.id)))
            .collect();
        // The colours that suit this piece (fabrics for upholstery, woods
        // and metals for legs…), then every material of the project.
        let piece = self.object_pieces(id).into_iter().find(|x| x.key == key);
        let mut act = None;
        let area = egui::Area::new(Id::unique("object-bar-swatches"))
            .order(egui::Order::Foreground)
            .pivot(egui::Align2::LEFT_BOTTOM)
            .fixed_pos(Pos2::new(at.left() - 8.0, at.top() - 10.0))
            .constrain(true)
            .show(ctx, |ui| {
                egui::Frame::popup(ui.style()).show(ui, |ui| {
                    ui.set_max_width(320.0);
                    if let Some(piece) = &piece {
                        ui.label(
                            egui::RichText::new("Colores que le van")
                                .small()
                                .color(p.muted),
                        );
                        ui.horizontal_wrapped(|ui| {
                            ui.spacing_mut().item_spacing = Vec2::splat(6.0);
                            for (swatch, rgb) in &piece.swatches {
                                let on = piece.current.as_deref() == Some(*swatch);
                                let (r, resp) =
                                    ui.allocate_exact_size(Vec2::splat(30.0), Sense::click());
                                let painter = ui.painter();
                                if on {
                                    painter.circle_stroke(
                                        r.center(),
                                        15.0,
                                        Stroke::new(2.0, p.accent),
                                    );
                                }
                                painter.circle(
                                    r.center(),
                                    12.0,
                                    Color32::from_rgb(rgb[0], rgb[1], rgb[2]),
                                    Stroke::new(1.0, Color32::from_black_alpha(40)),
                                );
                                if resp.on_hover_text(*swatch).clicked() {
                                    act = Some(BarAct::PaintPiece(key.to_string(), swatch));
                                }
                            }
                        });
                        ui.add_space(4.0);
                        ui.label(
                            egui::RichText::new("Todos los materiales")
                                .small()
                                .color(p.muted),
                        );
                    }
                    egui::ScrollArea::vertical()
                        .max_height(220.0)
                        .show(ui, |ui| {
                            egui::Grid::new("object-bar-swatch-grid")
                                .spacing(Vec2::splat(6.0))
                                .show(ui, |ui| {
                                    for (i, (m, name, rgb)) in mats.iter().enumerate() {
                                        let on = now == Some(*m);
                                        let (r, resp) = ui
                                            .allocate_exact_size(Vec2::splat(30.0), Sense::click());
                                        let painter = ui.painter();
                                        if on {
                                            painter.circle_stroke(
                                                r.center(),
                                                15.0,
                                                Stroke::new(2.0, p.accent),
                                            );
                                        }
                                        painter.circle(
                                            r.center(),
                                            12.0,
                                            Color32::from_rgb(rgb[0], rgb[1], rgb[2]),
                                            Stroke::new(1.0, Color32::from_black_alpha(40)),
                                        );
                                        if resp.on_hover_text(name).clicked() {
                                            act = Some(BarAct::SetMaterial(key.to_string(), *m));
                                        }
                                        if i % 8 == 7 {
                                            ui.end_row();
                                        }
                                    }
                                });
                        });
                })
            });
        // A click elsewhere closes it.
        let outside = ctx.input(|i| {
            i.pointer.any_pressed()
                && i.pointer
                    .interact_pos()
                    .is_some_and(|q| !area.response.rect.contains(q) && !at.contains(q))
        });
        if outside && act.is_none() {
            self.bar_swatches = None;
        }
        act
    }
}

/// The drawing of a kind of element, when it has no picture.
fn glyph_of(e: Element) -> Glyph {
    match e {
        Element::Wall(_) => Glyph::Wall,
        Element::Slab(_) => Glyph::Slab,
        Element::Opening(_) => Glyph::Window,
        Element::Roof(_) | Element::Dormer(..) => Glyph::Roof,
        Element::Stair(_) => Glyph::Stair,
        Element::Railing(_) => Glyph::Railing,
        Element::Column(_) => Glyph::Column,
        Element::Beam(_) => Glyph::Beam,
        Element::Room(_) => Glyph::Room,
        _ => Glyph::Object,
    }
}

/// «Material del cuerpo» → «Cuerpo»: the last words that say which part.
fn short(name: &str) -> &str {
    name.strip_prefix("Material de la ")
        .or_else(|| name.strip_prefix("Material del "))
        .or_else(|| name.strip_prefix("Material de "))
        .or_else(|| name.strip_prefix("Material "))
        .unwrap_or(name)
}

/// A button of the bar: its drawing over its name.
fn bar_tool(ui: &mut Ui, mark: Mark, name: &str, on: bool, p: Palette) -> egui::Response {
    let (r, resp) = ui.allocate_exact_size(Vec2::new(58.0, 52.0), Sense::click());
    let painter = ui.painter();
    if on {
        painter.rect_filled(r, CornerRadius::same(10), p.accent_soft);
    } else if resp.hovered() {
        painter.rect_filled(r, CornerRadius::same(10), p.line.gamma_multiply(0.5));
    }
    let color = if name == "Borrar" { p.bad } else { p.text };
    let icon = Rect::from_center_size(Pos2::new(r.center().x, r.top() + 18.0), Vec2::splat(20.0));
    super::context_bar::paint_mark(painter, icon, mark, color);
    painter.text(
        Pos2::new(r.center().x, r.bottom() - 10.0),
        egui::Align2::CENTER_CENTER,
        name,
        egui::FontId::proportional(11.0),
        color,
    );
    resp
}

/// A round swatch of a material with its name under it.
fn swatch_button(ui: &mut Ui, rgb: [u8; 3], name: &str, open: bool, p: Palette) -> egui::Response {
    let (r, resp) = ui.allocate_exact_size(Vec2::new(62.0, 52.0), Sense::click());
    let painter = ui.painter();
    if open {
        painter.rect_filled(r, CornerRadius::same(10), p.accent_soft);
    } else if resp.hovered() {
        painter.rect_filled(r, CornerRadius::same(10), p.line.gamma_multiply(0.5));
    }
    let c = Pos2::new(r.center().x, r.top() + 18.0);
    painter.circle(
        c,
        10.0,
        Color32::from_rgb(rgb[0], rgb[1], rgb[2]),
        Stroke::new(1.5, Color32::from_black_alpha(60)),
    );
    painter.text(
        Pos2::new(r.center().x, r.bottom() - 10.0),
        egui::Align2::CENTER_CENTER,
        truncate(name, 10),
        egui::FontId::proportional(11.0),
        p.text,
    );
    resp
}

/// A small count as a row of buttons under its name; the one clicked.
fn count_buttons(ui: &mut Ui, label: &str, lo: i64, hi: i64, now: i64, p: Palette) -> Option<i64> {
    let mut out = None;
    ui.vertical(|ui| {
        ui.add_space(6.0);
        ui.label(egui::RichText::new(label).size(11.0).color(p.muted));
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 1.0;
            for n in lo..=hi {
                let on = n == now;
                let (r, resp) = ui.allocate_exact_size(Vec2::new(22.0, 22.0), Sense::click());
                let painter = ui.painter();
                painter.rect_filled(
                    r,
                    CornerRadius::same(6),
                    if on {
                        p.accent_soft
                    } else if resp.hovered() {
                        p.line.gamma_multiply(0.5)
                    } else {
                        Color32::TRANSPARENT
                    },
                );
                painter.text(
                    r.center(),
                    egui::Align2::CENTER_CENTER,
                    n.to_string(),
                    egui::FontId::proportional(12.0),
                    if on { p.accent } else { p.text },
                );
                if resp.clicked() && !on {
                    out = Some(n);
                }
            }
        });
    });
    out
}

/// A choice of the bar: its name over what it is now.
fn choice_button(ui: &mut Ui, label: &str, now: &str, open: bool, p: Palette) -> egui::Response {
    let (r, resp) = ui.allocate_exact_size(Vec2::new(78.0, 52.0), Sense::click());
    let painter = ui.painter();
    if open {
        painter.rect_filled(r, CornerRadius::same(10), p.accent_soft);
    } else if resp.hovered() {
        painter.rect_filled(r, CornerRadius::same(10), p.line.gamma_multiply(0.5));
    }
    painter.text(
        Pos2::new(r.center().x, r.top() + 15.0),
        egui::Align2::CENTER_CENTER,
        truncate(label, 12),
        egui::FontId::proportional(11.0),
        p.muted,
    );
    painter.text(
        Pos2::new(r.center().x, r.bottom() - 17.0),
        egui::Align2::CENTER_CENTER,
        format!("{} ▾", truncate(now, 10)),
        egui::FontId::proportional(12.0),
        if open { p.accent } else { p.text },
    );
    resp.on_hover_text(format!("{label}: {now}"))
}

fn truncate(s: &str, n: usize) -> String {
    if s.chars().count() <= n {
        s.to_string()
    } else {
        let mut t: String = s.chars().take(n - 1).collect();
        t.push('…');
        t
    }
}

/// A thin line between groups of the bar.
fn bar_gap(ui: &mut Ui, p: Palette) {
    let (r, _) = ui.allocate_exact_size(Vec2::new(13.0, 52.0), Sense::hover());
    ui.painter().line_segment(
        [
            Pos2::new(r.center().x, r.top() + 10.0),
            Pos2::new(r.center().x, r.bottom() - 10.0),
        ],
        Stroke::new(1.0, p.line),
    );
}
