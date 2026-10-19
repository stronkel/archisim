//! Properties of 2D drafting (with a command running, or of the selected
//! lines, texts and hatches), and the drawing layers dialog (AutoCAD's
//! layer manager).

use archisim_core::{
    DraftId, DrawLayer, DrawLayerId, HatchPattern, LineStyle, Rgb, Shape, TextAlign, LINE_WEIGHTS,
};
use eframe::egui::{self, Ui};

use super::icons::Icon;
use super::props::{self, choice, group, row, text_row};
use super::*;
use crate::units::{fmt_area, fmt_mm};

/// Layer, colour, weight and line style to change (`None`: unchanged).
type StyleChange = (
    Option<DrawLayerId>,
    Option<Option<Rgb>>,
    Option<Option<f64>>,
    Option<Option<LineStyle>>,
);

/// "By layer" or a value, for the combos.
#[derive(Debug, Clone, Copy, PartialEq)]
enum ByLayer<T> {
    Layer,
    Own(T),
}

fn weight_label(w: f64) -> String {
    format!("{w:.2} mm").replace('.', ",")
}

/// The options of the line weight combo.
fn weight_options() -> Vec<(ByLayer<u64>, String)> {
    let mut v = vec![(ByLayer::Layer, "PorCapa".to_string())];
    v.extend(
        LINE_WEIGHTS
            .iter()
            .map(|w| (ByLayer::Own((w * 1000.0).round() as u64), weight_label(*w))),
    );
    v
}

fn line_options() -> Vec<(ByLayer<LineStyle>, &'static str)> {
    let mut v = vec![(ByLayer::Layer, "PorCapa")];
    v.extend(LineStyle::ALL.iter().map(|l| (ByLayer::Own(*l), l.label())));
    v
}

fn combo<T: PartialEq + Copy>(ui: &mut Ui, salt: &str, w: f32, v: T, opts: &[(T, String)]) -> T {
    let refs: Vec<(T, &str)> = opts.iter().map(|(a, b)| (*a, b.as_str())).collect();
    choice(ui, salt, w, v, &refs)
}

impl ArchiSimApp {
    fn layer_options(&self) -> Vec<(DrawLayerId, String)> {
        self.ed
            .model()
            .draw_layers()
            .iter()
            .map(|l| (l.id, l.name.clone()))
            .collect()
    }

    /// Colour row: by layer, or a colour of its own.
    fn color_row(ui: &mut Ui, salt: &str, current: Option<Rgb>) -> Option<Option<Rgb>> {
        let mut out = None;
        row(ui, "Color", "PorCapa: el de la capa", |ui, _w| {
            let mut by_layer = current.is_none();
            if ui.checkbox(&mut by_layer, "PorCapa").changed() {
                out = Some(if by_layer { None } else { Some([0, 0, 0]) });
            }
            if let Some(mut c) = current {
                ui.push_id(salt, |ui| {
                    if ui.color_edit_button_srgb(&mut c).changed() {
                        out = Some(Some(c));
                    }
                });
            }
        });
        out
    }

    /// With a 2D command running: the layer and style of what will be
    /// drawn, and the values commands remember.
    pub(super) fn cad_tool_properties(&mut self, ui: &mut Ui) {
        let name = self.cad.as_ref().map(|c| c.name()).unwrap_or("");
        props::title(ui, &format!("Dibujo 2D · {name}"));
        let layers = self.layer_options();
        let st = &mut self.cad_settings;
        let mut open_layers = false;
        group(ui, "cad-style", "Capa y estilo", |ui| {
            row(ui, "Capa actual", "Capa de lo que se dibuja", |ui, w| {
                st.layer = combo(ui, "cad-layer", w - 30.0, st.layer, &layers);
                if super::widgets::icon_button(ui, Icon::Layers, "Capas…", false).clicked() {
                    open_layers = true;
                }
            });
            if let Some(c) = Self::color_row(ui, "cad-color", st.color) {
                st.color = c;
            }
            row(ui, "Grosor", "Grosor de línea en el papel", |ui, w| {
                let cur =
                    st.weight.map_or(
                        ByLayer::Layer,
                        |w| ByLayer::Own((w * 1000.0).round() as u64),
                    );
                let v = combo(ui, "cad-weight", w, cur, &weight_options());
                st.weight = match v {
                    ByLayer::Layer => None,
                    ByLayer::Own(x) => Some(x as f64 / 1000.0),
                };
            });
            row(ui, "Tipo de línea", "", |ui, w| {
                let cur = st.line.map_or(ByLayer::Layer, ByLayer::Own);
                let opts: Vec<_> = line_options()
                    .into_iter()
                    .map(|(a, b)| (a, b.to_string()))
                    .collect();
                st.line = match combo(ui, "cad-line", w, cur, &opts) {
                    ByLayer::Layer => None,
                    ByLayer::Own(l) => Some(l),
                };
            });
        });
        group(ui, "cad-text", "Texto", |ui| {
            row(ui, "Altura (mm de papel)", "", |ui, _w| {
                ui.add(
                    egui::DragValue::new(&mut st.text_height)
                        .range(0.5..=50.0)
                        .speed(0.1)
                        .max_decimals(1),
                );
            });
            row(ui, "Justificación", "", |ui, w| {
                let opts: Vec<_> = TextAlign::ALL
                    .iter()
                    .map(|a| (*a, a.label().to_string()))
                    .collect();
                st.text_align = combo(ui, "cad-align", w, st.text_align, &opts);
            });
        });
        group(ui, "cad-hatch", "Sombreado", |ui| {
            row(ui, "Patrón", "", |ui, w| {
                let opts: Vec<_> = HatchPattern::ALL
                    .iter()
                    .map(|p| (*p, p.label().to_string()))
                    .collect();
                st.hatch_pattern = combo(ui, "cad-pattern", w, st.hatch_pattern, &opts);
            });
            row(ui, "Escala", "", |ui, _w| {
                ui.add(
                    egui::DragValue::new(&mut st.hatch_scale)
                        .range(0.05..=100.0)
                        .speed(0.05),
                );
            });
            row(ui, "Ángulo (°)", "", |ui, _w| {
                ui.add(
                    egui::DragValue::new(&mut st.hatch_angle)
                        .range(-360.0..=360.0)
                        .speed(1.0),
                );
            });
            if let Some(c) = Self::fill_row(ui, "cad-bg", st.hatch_background) {
                st.hatch_background = c;
            }
        });
        group(ui, "cad-values", "Valores de las órdenes", |ui| {
            row(ui, "Radio de empalme (mm)", "EMPALME, RECTANG", |ui, _w| {
                let mut mm = st.fillet_radius * 1000.0;
                if ui
                    .add(egui::DragValue::new(&mut mm).range(0.0..=1.0e6).speed(1.0))
                    .changed()
                {
                    st.fillet_radius = mm / 1000.0;
                }
            });
            row(ui, "Desfase (mm)", "EQUIDIST", |ui, _w| {
                let mut mm = st.offset * 1000.0;
                if ui
                    .add(egui::DragValue::new(&mut mm).range(1.0..=1.0e6).speed(1.0))
                    .changed()
                {
                    st.offset = mm / 1000.0;
                }
            });
            row(
                ui,
                "Anchura de polilínea (mm)",
                "POL, RECTANG",
                |ui, _w| {
                    let mut mm = st.polyline_width * 1000.0;
                    if ui
                        .add(egui::DragValue::new(&mut mm).range(0.0..=1.0e5).speed(1.0))
                        .changed()
                    {
                        st.polyline_width = mm / 1000.0;
                    }
                },
            );
            row(
                ui,
                "Orto (F8)",
                "Líneas horizontales y verticales",
                |ui, _w| {
                    ui.checkbox(&mut st.ortho, "");
                },
            );
        });
        if open_layers {
            self.show_layers = true;
        }
        ui.add_space(4.0);
        props::note(
            ui,
            "Escribe valores en la línea de órdenes (bajo el dibujo): 3000,2000 · @1500,0 · @2000<45 · o una distancia en la dirección del cursor. Intro o Espacio aceptan; Esc termina.",
        );
    }

    /// Background colour row of a hatch: none, or a colour.
    fn fill_row(ui: &mut Ui, salt: &str, current: Option<Rgb>) -> Option<Option<Rgb>> {
        let mut out = None;
        row(ui, "Fondo", "Color detrás de la trama", |ui, _w| {
            let mut on = current.is_some();
            if ui.checkbox(&mut on, "").changed() {
                out = Some(on.then_some([255, 255, 255]));
            }
            if let Some(mut c) = current {
                ui.push_id(salt, |ui| {
                    if ui.color_edit_button_srgb(&mut c).changed() {
                        out = Some(Some(c));
                    }
                });
            }
        });
        out
    }

    /// The selected drafting elements: layer and style, and what each kind
    /// has (text, hatch pattern, measures).
    pub(super) fn draft_properties(&mut self, ui: &mut Ui, ids: &[DraftId]) {
        let model = self.ed.model();
        let drafts: Vec<archisim_core::Draft> = ids
            .iter()
            .filter_map(|id| model.draft(*id).cloned())
            .collect();
        let Some(first) = drafts.first().cloned() else {
            return;
        };
        let title = if drafts.len() == 1 {
            format!("{} · 1 seleccionado", first.shape.label())
        } else {
            format!("Dibujo 2D · {} seleccionados", drafts.len())
        };
        props::title(ui, &title);
        let same =
            |f: &dyn Fn(&archisim_core::Draft) -> String| drafts.iter().all(|d| f(d) == f(&first));
        let layers = self.layer_options();
        let mut change: StyleChange = (None, None, None, None);
        group(ui, "draft-style", "Capa y estilo", |ui| {
            row(ui, "Capa", "", |ui, w| {
                let cur = if same(&|d| format!("{:?}", d.layer)) {
                    first.layer
                } else {
                    DrawLayerId(u64::MAX)
                };
                let mut opts = layers.clone();
                if cur.0 == u64::MAX {
                    opts.insert(0, (cur, "(varias)".into()));
                }
                let v = combo(ui, "draft-layer", w, cur, &opts);
                if v != cur {
                    change.0 = Some(v);
                }
            });
            if let Some(c) = Self::color_row(ui, "draft-color", first.color) {
                change.1 = Some(c);
            }
            row(ui, "Grosor", "", |ui, w| {
                let cur = first.weight.map_or(ByLayer::Layer, |w| {
                    ByLayer::Own((w * 1000.0).round() as u64)
                });
                let v = combo(ui, "draft-weight", w, cur, &weight_options());
                if v != cur {
                    change.2 = Some(match v {
                        ByLayer::Layer => None,
                        ByLayer::Own(x) => Some(x as f64 / 1000.0),
                    });
                }
            });
            row(ui, "Tipo de línea", "", |ui, w| {
                let cur = first.line.map_or(ByLayer::Layer, ByLayer::Own);
                let opts: Vec<_> = line_options()
                    .into_iter()
                    .map(|(a, b)| (a, b.to_string()))
                    .collect();
                let v = combo(ui, "draft-line", w, cur, &opts);
                if v != cur {
                    change.3 = Some(match v {
                        ByLayer::Layer => None,
                        ByLayer::Own(l) => Some(l),
                    });
                }
            });
        });
        if change != (None, None, None, None) {
            let r = self
                .ed
                .set_drafts_style(ids, change.0, change.1, change.2, change.3);
            self.report(r, "Estilo cambiado.");
        }
        // Measures.
        let total_len: f64 = drafts
            .iter()
            .map(|d| archisim_draft::length(&d.shape))
            .sum();
        let total_area: f64 = drafts
            .iter()
            .filter_map(|d| archisim_draft::area(&d.shape))
            .sum();
        group(ui, "draft-measure", "Medidas", |ui| {
            if total_len > 0.0 {
                text_row(ui, "Longitud", "", &format!("{} mm", fmt_mm(total_len)));
            }
            if total_area > 0.0 {
                text_row(ui, "Área", "", &fmt_area(total_area));
            }
            if drafts.len() == 1 {
                match &first.shape {
                    Shape::Circle { radius, .. } | Shape::Arc { radius, .. } => {
                        text_row(ui, "Radio", "", &format!("{} mm", fmt_mm(*radius)));
                    }
                    Shape::Line { a, b } => {
                        text_row(ui, "Ángulo", "", &fmt_angle((*b - *a).y.atan2((*b - *a).x)));
                    }
                    _ => {}
                }
            }
        });
        if drafts.len() != 1 {
            return;
        }
        let id = first.id;
        match first.shape.clone() {
            Shape::Text(t) => {
                let mut new = t.clone();
                group(ui, "draft-text", "Texto", |ui| {
                    let edit_id = ui.make_persistent_id(("draft-text-edit", id));
                    let mut buf: String = ui
                        .data_mut(|d| d.get_temp::<String>(edit_id))
                        .unwrap_or_else(|| t.content.clone());
                    let r = ui.add(
                        egui::TextEdit::multiline(&mut buf)
                            .desired_rows(2)
                            .desired_width(f32::INFINITY),
                    );
                    if r.changed() {
                        ui.data_mut(|d| d.insert_temp(edit_id, buf.clone()));
                    }
                    if r.lost_focus() {
                        if !buf.trim().is_empty() {
                            new.content = buf.trim_end().to_string();
                        }
                        ui.data_mut(|d| d.remove::<String>(edit_id));
                    }
                    row(ui, "Altura (mm de papel)", "", |ui, _w| {
                        ui.add(
                            egui::DragValue::new(&mut new.height)
                                .range(0.5..=50.0)
                                .speed(0.1),
                        );
                    });
                    row(ui, "Ángulo (°)", "", |ui, _w| {
                        let mut deg = new.angle.to_degrees();
                        if ui.add(egui::DragValue::new(&mut deg).speed(1.0)).changed() {
                            new.angle = deg.to_radians();
                        }
                    });
                    row(ui, "Justificación", "", |ui, w| {
                        let opts: Vec<_> = TextAlign::ALL
                            .iter()
                            .map(|a| (*a, a.label().to_string()))
                            .collect();
                        new.align = combo(ui, "draft-align", w, new.align, &opts);
                    });
                });
                if new != t {
                    let r = self
                        .ed
                        .set_draft_shape(id, Shape::Text(new), "Cambiar texto");
                    self.report(r, "Texto cambiado.");
                }
            }
            Shape::Hatch(h) => {
                let mut new = h.clone();
                group(ui, "draft-hatch", "Sombreado", |ui| {
                    row(ui, "Patrón", "", |ui, w| {
                        let opts: Vec<_> = HatchPattern::ALL
                            .iter()
                            .map(|p| (*p, p.label().to_string()))
                            .collect();
                        new.pattern = combo(ui, "draft-pattern", w, new.pattern, &opts);
                    });
                    row(ui, "Escala", "", |ui, _w| {
                        ui.add(
                            egui::DragValue::new(&mut new.scale)
                                .range(0.05..=100.0)
                                .speed(0.05),
                        );
                    });
                    row(ui, "Ángulo (°)", "", |ui, _w| {
                        let mut deg = new.angle.to_degrees();
                        if ui.add(egui::DragValue::new(&mut deg).speed(1.0)).changed() {
                            new.angle = deg.to_radians();
                        }
                    });
                    if let Some(c) = Self::fill_row(ui, "draft-bg", new.background) {
                        new.background = c;
                    }
                    text_row(ui, "Contornos", "", &new.loops.len().to_string());
                });
                if new != h {
                    let r = self
                        .ed
                        .set_draft_shape(id, Shape::Hatch(new), "Cambiar sombreado");
                    self.report(r, "Sombreado cambiado.");
                }
            }
            Shape::Polyline(p) => {
                let mut new = p.clone();
                group(ui, "draft-pline", "Polilínea", |ui| {
                    text_row(ui, "Vértices", "", &p.points.len().to_string());
                    row(ui, "Cerrada", "", |ui, _w| {
                        ui.checkbox(&mut new.closed, "");
                    });
                    row(ui, "Anchura (mm)", "", |ui, _w| {
                        let mut mm = new.width * 1000.0;
                        if ui
                            .add(egui::DragValue::new(&mut mm).range(0.0..=1.0e5).speed(1.0))
                            .changed()
                        {
                            new.width = mm / 1000.0;
                        }
                    });
                });
                if new != p {
                    let r = self
                        .ed
                        .set_draft_shape(id, Shape::Polyline(new), "Cambiar polilínea");
                    self.report(r, "Polilínea cambiada.");
                }
            }
            Shape::Circle { centre, radius } => {
                let mut r = radius;
                group(ui, "draft-circle", "Círculo", |ui| {
                    row(ui, "Radio (mm)", "", |ui, _w| {
                        let mut mm = r * 1000.0;
                        if ui
                            .add(egui::DragValue::new(&mut mm).range(1.0..=1.0e7).speed(1.0))
                            .changed()
                        {
                            r = mm / 1000.0;
                        }
                    });
                });
                if r != radius {
                    let res = self.ed.set_draft_shape(
                        id,
                        Shape::Circle { centre, radius: r },
                        "Cambiar radio",
                    );
                    self.report(res, "Radio cambiado.");
                }
            }
            _ => {}
        }
    }

    /// The drawing layers dialog: every layer with its switches, colour,
    /// weight and line style; the current one; new and delete.
    pub(super) fn layers_dialog(&mut self, ctx: &egui::Context) {
        if !self.show_layers {
            return;
        }
        let mut open = true;
        let layers: Vec<DrawLayer> = self.ed.model().draw_layers().into_iter().cloned().collect();
        let mut set: Option<DrawLayer> = None;
        let mut remove: Option<DrawLayerId> = None;
        let mut add = false;
        egui::Window::new("Capas de dibujo")
            .open(&mut open)
            .default_width(620.0)
            .show(ctx, |ui| {
                egui::Grid::new("layers-grid")
                    .striped(true)
                    .spacing([10.0, 6.0])
                    .show(ui, |ui| {
                        for h in [
                            "Actual",
                            "Nombre",
                            "Visible",
                            "Bloqueada",
                            "Imprimir",
                            "Color",
                            "Grosor",
                            "Tipo de línea",
                            "Elementos",
                            "",
                        ] {
                            ui.strong(h);
                        }
                        ui.end_row();
                        for l in &layers {
                            let mut v = l.clone();
                            ui.radio_value(&mut self.cad_settings.layer, l.id, "");
                            if l.id == archisim_core::LAYER_ZERO {
                                ui.label("0");
                            } else {
                                let id = ui.make_persistent_id(("layer-name", l.id));
                                let mut buf: String = ui
                                    .data_mut(|d| d.get_temp::<String>(id))
                                    .unwrap_or_else(|| l.name.clone());
                                let r = ui
                                    .add(egui::TextEdit::singleline(&mut buf).desired_width(130.0));
                                if r.changed() {
                                    ui.data_mut(|d| d.insert_temp(id, buf.clone()));
                                }
                                if r.lost_focus() {
                                    v.name = buf;
                                    ui.data_mut(|d| d.remove::<String>(id));
                                }
                            }
                            ui.checkbox(&mut v.visible, "");
                            ui.checkbox(&mut v.locked, "");
                            ui.checkbox(&mut v.printable, "");
                            ui.color_edit_button_srgb(&mut v.color);
                            let opts: Vec<(u64, String)> = LINE_WEIGHTS
                                .iter()
                                .map(|w| ((w * 1000.0).round() as u64, weight_label(*w)))
                                .collect();
                            let cur = (v.weight * 1000.0).round() as u64;
                            let w = combo(ui, format!("lw{}", l.id.0).as_str(), 90.0, cur, &opts);
                            v.weight = w as f64 / 1000.0;
                            let lopts: Vec<(LineStyle, String)> = LineStyle::ALL
                                .iter()
                                .map(|s| (*s, s.label().to_string()))
                                .collect();
                            v.line =
                                combo(ui, format!("lt{}", l.id.0).as_str(), 130.0, v.line, &lopts);
                            ui.label(self.ed.model().draw_layer_users(l.id).to_string());
                            if l.id != archisim_core::LAYER_ZERO
                                && ui
                                    .small_button("Borrar")
                                    .on_hover_text("Borra la capa; sus elementos pasan a la capa 0")
                                    .clicked()
                            {
                                remove = Some(l.id);
                            }
                            ui.end_row();
                            if v != *l {
                                set = Some(v);
                            }
                        }
                    });
                ui.add_space(8.0);
                if ui.button("Nueva capa").clicked() {
                    add = true;
                }
            });
        if !open {
            self.show_layers = false;
        }
        if let Some(l) = set {
            let r = self.ed.set_draw_layer(l);
            if let Err(e) = r {
                self.complain(format!("No se puede: {e}."));
            }
        }
        if let Some(id) = remove {
            if self.cad_settings.layer == id {
                self.cad_settings.layer = archisim_core::LAYER_ZERO;
            }
            let r = self.ed.remove_draw_layer(id);
            self.report(r, "Capa borrada; sus elementos están en la capa 0.");
        }
        if add {
            let n = layers.len();
            let name = (n..n + 1000)
                .map(|i| format!("Capa {i}"))
                .find(|nm| self.ed.model().draw_layer_by_name(nm).is_none())
                .unwrap_or_else(|| "Capa nueva".into());
            match self.ed.add_draw_layer(DrawLayer::new(
                name,
                [0, 0, 0],
                0.18,
                LineStyle::Continuous,
            )) {
                Ok(id) => self.cad_settings.layer = id,
                Err(e) => self.complain(format!("No se puede: {e}.")),
            }
        }
    }
}
