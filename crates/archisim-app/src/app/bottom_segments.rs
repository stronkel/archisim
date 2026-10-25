//! The segments of the bottom bar (`docs/INTERFAZ.md` §2.8): Línea,
//! Relleno, Texto and Cota, the 2D drawing and annotation of the view,
//! always at hand in any discipline; and the snaps, as drawings. Each
//! segment starts its order with a click; its arrow opens the variants and
//! the settings of what is drawn next.

use archisim_core::LineStyle;
use eframe::egui::{self, Align2, Color32, CornerRadius, FontId, Rect, Sense, Stroke, Ui, Vec2};

use super::context_bar::paint_mark;
use super::draft_props::{line_options, weight_options, ByLayer};
use super::glyphs::Glyph;
use super::icons::Icon;
use super::theme::Palette;
use super::toolbox::{Mark, Pick};
use super::widgets::strong_font;
use super::{ArchiSimApp, Tool};
use crate::dim_tool::DimMode;

/// What a segment's menu offers to draw.
pub(super) struct Variant {
    pub mark: Mark,
    pub name: &'static str,
    pub key: &'static str,
    pub pick: Pick,
}

const LINES: &[Variant] = &[
    Variant {
        mark: Mark::Glyph(Glyph::Line),
        name: "Línea",
        key: "L",
        pick: Pick::Command("LINEA"),
    },
    Variant {
        mark: Mark::Icon(Icon::Pline),
        name: "Polilínea",
        key: "PL",
        pick: Pick::Command("POL"),
    },
    Variant {
        mark: Mark::Glyph(Glyph::Spline),
        name: "Spline",
        key: "SPL",
        pick: Pick::Command("SPLINE"),
    },
    Variant {
        mark: Mark::Icon(Icon::GeoRect),
        name: "Rectángulo",
        key: "REC",
        pick: Pick::Command("RECTANG"),
    },
    Variant {
        mark: Mark::Icon(Icon::GeoPolygon),
        name: "Polígono",
        key: "PG",
        pick: Pick::Command("POLIGONO"),
    },
    Variant {
        mark: Mark::Icon(Icon::GeoArc),
        name: "Arco",
        key: "A",
        pick: Pick::Command("ARCO"),
    },
    Variant {
        mark: Mark::Icon(Icon::GeoCircle),
        name: "Círculo",
        key: "CI",
        pick: Pick::Command("CIRCULO"),
    },
    Variant {
        mark: Mark::Icon(Icon::Ellipse),
        name: "Elipse",
        key: "EL",
        pick: Pick::Command("ELIPSE"),
    },
];

const FILLS: &[Variant] = &[
    Variant {
        mark: Mark::Icon(Icon::GeoRect),
        name: "Rectángulo",
        key: "RELLRECT",
        pick: Pick::Command("RELLRECT"),
    },
    Variant {
        mark: Mark::Icon(Icon::Pline),
        name: "Polígono",
        key: "RELLPOLIG",
        pick: Pick::Command("RELLPOLIG"),
    },
    Variant {
        mark: Mark::Glyph(Glyph::Spline),
        name: "Spline",
        key: "RELLSPLINE",
        pick: Pick::Command("RELLSPLINE"),
    },
    Variant {
        mark: Mark::Icon(Icon::GeoPolygon),
        name: "Polígono regular",
        key: "RELLREGULAR",
        pick: Pick::Command("RELLREGULAR"),
    },
    Variant {
        mark: Mark::Icon(Icon::GeoCircle),
        name: "Círculo",
        key: "RELLCIRC",
        pick: Pick::Command("RELLCIRC"),
    },
    Variant {
        mark: Mark::Icon(Icon::Ellipse),
        name: "Elipse",
        key: "RELLELIPSE",
        pick: Pick::Command("RELLELIPSE"),
    },
    Variant {
        mark: Mark::Glyph(Glyph::Hatch),
        name: "Por recinto o contorno",
        key: "SB",
        pick: Pick::Command("SOMBCONT"),
    },
];

const TEXTS: &[Variant] = &[
    Variant {
        mark: Mark::Glyph(Glyph::Text),
        name: "Texto",
        key: "TX",
        pick: Pick::Command("TEXTO"),
    },
    Variant {
        mark: Mark::Icon(Icon::Leader),
        name: "Directriz",
        key: "DIR",
        pick: Pick::Command("DIRECTRIZ"),
    },
    Variant {
        mark: Mark::Icon(Icon::Cloud),
        name: "Nube de revisión",
        key: "NR",
        pick: Pick::Command("NUBEREV"),
    },
    Variant {
        mark: Mark::Glyph(Glyph::Tag),
        name: "Etiqueta",
        key: "ETI",
        pick: Pick::Command("ETIQUETA"),
    },
];

const DIMS: &[Variant] = &[
    Variant {
        mark: Mark::Glyph(Glyph::Dimension),
        name: "Lineal",
        key: "C",
        pick: Pick::Tool(Tool::Dimension),
    },
    Variant {
        mark: Mark::Glyph(Glyph::LevelMark),
        name: "Cota de nivel",
        key: "N",
        pick: Pick::LevelMark,
    },
    Variant {
        mark: Mark::Icon(Icon::DimAngle),
        name: "Angular",
        key: "DAN",
        pick: Pick::Command("ACOANGULO"),
    },
    Variant {
        mark: Mark::Icon(Icon::DimRadius),
        name: "Radio",
        key: "DRA",
        pick: Pick::Command("ACORADIO"),
    },
    Variant {
        mark: Mark::Icon(Icon::DimArc),
        name: "Longitud de arco",
        key: "DAR",
        pick: Pick::Command("ACOARCO"),
    },
];

/// The segment of the bottom bar an order belongs to: its name and its
/// variants (shown as methods in the contextual bar while it runs).
pub(super) fn segment_of(command: &str) -> Option<(&'static str, &'static [Variant])> {
    [
        ("Línea", LINES),
        ("Relleno", FILLS),
        ("Texto", TEXTS),
        ("Cota", DIMS),
    ]
    .into_iter()
    .find(|(_, vs)| {
        vs.iter()
            .any(|v| matches!(v.pick, Pick::Command(c) if c == command))
    })
}

/// Which segment's menu is being drawn.
#[derive(Clone, Copy, PartialEq)]
enum Seg {
    Line,
    Fill,
    Text,
    Dim,
}

impl ArchiSimApp {
    /// Línea · Relleno · Texto · Cota.
    pub(super) fn drawing_segments(&mut self, ui: &mut Ui) {
        let st = &self.cad_settings;
        let layer = self
            .ed
            .model()
            .draw_layers()
            .iter()
            .find(|l| l.id == st.layer)
            .map_or("0".to_string(), |l| l.name.clone());
        let line_on = self.running_any(LINES);
        let fill_on = self.running_any(FILLS);
        let text_on = self.running_any(TEXTS);
        let dim_on = self.running_any(DIMS);
        let text_value = format!("{:.1} mm", st.text_height).replace('.', ",");
        // «Diagonal (fábrica)» → «Diagonal».
        let hatch = st
            .hatch_pattern
            .label()
            .split(" (")
            .next()
            .unwrap_or_default();
        let dim_value = match (self.tool, self.dim_settings.mode) {
            (Tool::Dimension, DimMode::LevelMark) => "Nivel",
            (Tool::Dimension, DimMode::WallOpenings) => "Huecos",
            _ => "Lineal",
        };
        let segs = [
            (Seg::Line, Glyph::Line, "Línea", layer, line_on, "Líneas, polilíneas, rectángulos, arcos y círculos de dibujo 2D (L). Flecha: más formas, capa, grosor y tipo de línea"),
            (Seg::Fill, Glyph::Hatch, "Relleno", hatch.to_string(), fill_on, "Tramas y rellenos: clic dentro de un recinto (SB). Flecha: trama, escala y ángulo"),
            (Seg::Text, Glyph::Text, "Texto", text_value, text_on, "Textos (TX). Flecha: directriz, nube de revisión, etiquetas y altura del texto"),
            (Seg::Dim, Glyph::Dimension, "Cota", dim_value.to_string(), dim_on, "Cotas (C). Flecha: cota de nivel, angular, de radio y de arco"),
        ];
        for (seg, glyph, name, value, on, hint) in segs {
            let (main, arrow) = segment(ui, glyph, name, &value, on);
            if main.on_hover_text(hint).clicked() {
                let first = match seg {
                    Seg::Line => LINES[0].pick,
                    Seg::Fill => FILLS[0].pick,
                    Seg::Text => TEXTS[0].pick,
                    Seg::Dim => DIMS[0].pick,
                };
                self.pick(first);
            }
            egui::Popup::menu(&arrow)
                .close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside)
                .show(|ui| {
                    ui.set_min_width(230.0);
                    self.segment_menu(ui, seg);
                });
        }
    }

    /// Whether the order of one of `variants` is the one running.
    fn running_any(&self, variants: &[Variant]) -> bool {
        let level_marks = self.dim_settings.mode == DimMode::LevelMark;
        variants.iter().any(|v| match v.pick {
            Pick::Command(name) => {
                self.tool == Tool::Cad && self.cad.as_ref().is_some_and(|c| c.name() == name)
            }
            Pick::Tool(Tool::Dimension) => self.tool == Tool::Dimension && !level_marks,
            Pick::LevelMark => self.tool == Tool::Dimension && level_marks,
            _ => false,
        })
    }

    fn segment_menu(&mut self, ui: &mut Ui, seg: Seg) {
        let variants = match seg {
            Seg::Line => LINES,
            Seg::Fill => FILLS,
            Seg::Text => TEXTS,
            Seg::Dim => DIMS,
        };
        let mut chosen = None;
        for v in variants {
            if menu_row(ui, v.mark, v.name, v.key).clicked() {
                chosen = Some(v.pick);
            }
        }
        if let Some(p) = chosen {
            self.pick(p);
            ui.close();
            return;
        }
        ui.separator();
        let layers = self.layer_options();
        let st = &mut self.cad_settings;
        match seg {
            Seg::Line => {
                let shown = layers
                    .iter()
                    .find(|l| l.0 == st.layer)
                    .map_or("0", |l| l.1.as_str())
                    .to_string();
                setting(ui, "Capa", |ui| {
                    egui::ComboBox::from_id_salt("seg-layer")
                        .width(130.0)
                        .selected_text(shown)
                        .show_ui(ui, |ui| {
                            for (id, name) in &layers {
                                ui.selectable_value(&mut st.layer, *id, name);
                            }
                        });
                });
                let weights = weight_options();
                let cur =
                    st.weight.map_or(
                        ByLayer::Layer,
                        |w| ByLayer::Own((w * 1000.0).round() as u64),
                    );
                let mut v = cur;
                setting(ui, "Grosor", |ui| {
                    egui::ComboBox::from_id_salt("seg-weight")
                        .width(130.0)
                        .selected_text(
                            weights
                                .iter()
                                .find(|o| o.0 == cur)
                                .map_or(String::new(), |o| o.1.clone()),
                        )
                        .show_ui(ui, |ui| {
                            for (o, name) in &weights {
                                ui.selectable_value(&mut v, *o, name);
                            }
                        });
                });
                st.weight = match v {
                    ByLayer::Layer => None,
                    ByLayer::Own(x) => Some(x as f64 / 1000.0),
                };
                let lines = line_options();
                let cur: ByLayer<LineStyle> = st.line.map_or(ByLayer::Layer, ByLayer::Own);
                let mut v = cur;
                setting(ui, "Tipo de línea", |ui| {
                    egui::ComboBox::from_id_salt("seg-line")
                        .width(130.0)
                        .selected_text(lines.iter().find(|o| o.0 == cur).map_or("", |o| o.1))
                        .show_ui(ui, |ui| {
                            for (o, name) in &lines {
                                ui.selectable_value(&mut v, *o, *name);
                            }
                        });
                });
                st.line = match v {
                    ByLayer::Layer => None,
                    ByLayer::Own(l) => Some(l),
                };
            }
            Seg::Fill => {
                setting(ui, "Trama", |ui| {
                    let ink = st.color.unwrap_or([40, 40, 40]);
                    st.hatch_pattern = super::hatch_ui::pattern_picker_inline(
                        ui,
                        "seg-hatch",
                        220.0,
                        st.hatch_pattern,
                        ink,
                        st.hatch_background,
                    );
                });
                let ink = st.color.unwrap_or([40, 40, 40]);
                if let Some(p) = super::hatch_ui::inline_grid(
                    ui,
                    "seg-hatch",
                    st.hatch_pattern,
                    ink,
                    st.hatch_background,
                ) {
                    st.hatch_pattern = p;
                }
                setting(ui, "Opacidad", |ui| {
                    if let Some(v) = super::hatch_ui::opacity_slider(ui, st.hatch_opacity, 160.0) {
                        st.hatch_opacity = v;
                    }
                });
                setting(ui, "Escala", |ui| {
                    ui.add(
                        egui::DragValue::new(&mut st.hatch_scale)
                            .range(0.1..=50.0)
                            .speed(0.05)
                            .max_decimals(2),
                    );
                });
                setting(ui, "Ángulo (°)", |ui| {
                    ui.add(
                        egui::DragValue::new(&mut st.hatch_angle)
                            .range(-180.0..=180.0)
                            .speed(1.0)
                            .max_decimals(1),
                    );
                });
            }
            Seg::Text => {
                setting(ui, "Altura (mm de papel)", |ui| {
                    ui.add(
                        egui::DragValue::new(&mut st.text_height)
                            .range(0.5..=50.0)
                            .speed(0.1)
                            .max_decimals(1),
                    );
                });
            }
            Seg::Dim => {
                ui.label(
                    egui::RichText::new(
                        "La dirección de las cotas lineales y sus huecos de muro están en Propiedades con la herramienta Cota.",
                    )
                    .small()
                    .weak(),
                );
            }
        }
    }

    /// The snaps, as drawings: ends, middles, edges, tracking and ortho.
    pub(super) fn snap_toggles(&mut self, ui: &mut Ui) {
        ui.spacing_mut().item_spacing.x = 2.0;
        let f = &mut self.snap_filter;
        toggle(
            ui,
            Glyph::SnapEnd,
            &mut f.points,
            "Extremos: forzar a extremos de muro, esquinas y extremos de líneas",
        );
        toggle(
            ui,
            Glyph::SnapMid,
            &mut f.midpoints,
            "Medios: forzar al punto medio de muros, caras y líneas",
        );
        toggle(
            ui,
            Glyph::SnapEdge,
            &mut f.edges,
            "Aristas: forzar a cualquier punto de las caras de muro y bordes de forjado",
        );
        toggle(
            ui,
            Glyph::SnapTrack,
            &mut f.tracking,
            "Rastreo: bloquea en horizontal y vertical (a menos de 8°) y en 45° al acercarse, con una línea azul. Mayús: siempre múltiplos de 45°",
        );
        let mut ortho = self.cad_settings.ortho;
        if toggle(
            ui,
            Glyph::Ortho,
            &mut ortho,
            "Orto (F8): líneas de dibujo solo horizontales y verticales",
        ) {
            self.toggle_ortho();
        }
    }
}

/// A row of a segment's menu.
fn menu_row(ui: &mut Ui, mark: Mark, name: &str, key: &str) -> egui::Response {
    let p = Palette::current(ui.ctx());
    let (rect, resp) =
        ui.allocate_exact_size(Vec2::new(ui.available_width(), 28.0), Sense::click());
    if resp.hovered() {
        ui.painter()
            .rect_filled(rect, CornerRadius::same(5), p.accent_soft);
    }
    paint_mark(
        ui.painter(),
        Rect::from_center_size(rect.left_center() + Vec2::new(14.0, 0.0), Vec2::splat(17.0)),
        mark,
        p.text,
    );
    ui.painter().text(
        rect.left_center() + Vec2::new(30.0, 0.0),
        Align2::LEFT_CENTER,
        name,
        FontId::proportional(12.5),
        p.text,
    );
    ui.painter().text(
        rect.right_center() - Vec2::new(8.0, 0.0),
        Align2::RIGHT_CENTER,
        key,
        FontId::monospace(10.5),
        p.muted,
    );
    resp
}

fn setting(ui: &mut Ui, label: &str, add: impl FnOnce(&mut Ui)) {
    ui.horizontal(|ui| {
        ui.add_sized(
            [92.0, 20.0],
            egui::Label::new(egui::RichText::new(label).small()),
        );
        add(ui);
    });
}

/// A segment: its drawing, name and current value, and an arrow at the
/// right for its menu. Returns the responses of the body and the arrow.
fn segment(
    ui: &mut Ui,
    glyph: Glyph,
    name: &str,
    value: &str,
    on: bool,
) -> (egui::Response, egui::Response) {
    let p = Palette::current(ui.ctx());
    let name_font = strong_font(11.5);
    let value_font = FontId::proportional(10.5);
    let gn = ui
        .painter()
        .layout_no_wrap(name.to_string(), name_font.clone(), p.text);
    let mut value = value.to_string();
    while value.chars().count() > 14 {
        value.pop();
    }
    let gv = ui
        .painter()
        .layout_no_wrap(value.clone(), value_font.clone(), p.muted);
    let body_w = 6.0 + 16.0 + 6.0 + gn.size().x + 6.0 + gv.size().x + 6.0;
    let arrow_w = 16.0;
    let (rect, _) = ui.allocate_exact_size(Vec2::new(body_w + arrow_w, 24.0), Sense::hover());
    let body = Rect::from_min_size(rect.min, Vec2::new(body_w, rect.height()));
    let arrow = Rect::from_min_max(egui::pos2(body.right(), rect.top()), rect.max);
    let id = ui.make_persistent_id(("segment", name));
    let rb = ui.interact(body, id, Sense::click());
    let ra = ui.interact(arrow, id.with("arrow"), Sense::click());
    let painter = ui.painter();
    let fill = if on {
        p.accent_soft
    } else if rb.hovered() || ra.hovered() {
        p.raised
    } else {
        Color32::TRANSPARENT
    };
    painter.rect(
        rect,
        CornerRadius::same(6),
        fill,
        Stroke::new(1.0, if on { p.accent } else { p.line }),
        egui::StrokeKind::Inside,
    );
    if ra.hovered() {
        painter.rect_filled(arrow.shrink(2.0), CornerRadius::same(4), p.accent_soft);
    }
    painter.vline(
        arrow.left(),
        rect.y_range().shrink(5.0),
        Stroke::new(1.0, p.line),
    );
    let fg = if on { p.accent } else { p.text };
    paint_mark(
        painter,
        Rect::from_center_size(
            rect.left_center() + Vec2::new(6.0 + 8.0, 0.0),
            Vec2::splat(16.0),
        ),
        Mark::Glyph(glyph),
        fg,
    );
    let x = rect.left() + 6.0 + 16.0 + 6.0;
    painter.text(
        egui::pos2(x, rect.center().y),
        Align2::LEFT_CENTER,
        name,
        name_font,
        fg,
    );
    painter.text(
        egui::pos2(x + gn.size().x + 6.0, rect.center().y),
        Align2::LEFT_CENTER,
        value,
        value_font,
        p.muted,
    );
    // The arrow: a small chevron pointing up (the menu opens upwards).
    let c = arrow.center();
    painter.line(
        vec![
            c + Vec2::new(-3.5, 1.8),
            c + Vec2::new(0.0, -1.8),
            c + Vec2::new(3.5, 1.8),
        ],
        Stroke::new(1.3, p.muted),
    );
    (rb, ra.on_hover_text(format!("Más de {name}")))
}

/// A snap as its drawing, accent when on. Returns whether it changed.
fn toggle(ui: &mut Ui, glyph: Glyph, on: &mut bool, hint: &str) -> bool {
    let p = Palette::current(ui.ctx());
    let (rect, resp) = ui.allocate_exact_size(Vec2::new(26.0, 24.0), Sense::click());
    let fill = if *on {
        p.accent_soft
    } else if resp.hovered() {
        p.raised
    } else {
        Color32::TRANSPARENT
    };
    ui.painter().rect_filled(rect, CornerRadius::same(6), fill);
    paint_mark(
        ui.painter(),
        Rect::from_center_size(rect.center(), Vec2::splat(17.0)),
        Mark::Glyph(glyph),
        if *on { p.accent } else { p.muted },
    );
    let state = if *on { "activado" } else { "desactivado" };
    if resp.on_hover_text(format!("{hint} ({state})")).clicked() {
        *on = !*on;
        return true;
    }
    false
}
