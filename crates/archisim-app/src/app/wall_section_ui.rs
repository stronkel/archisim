//! «Sección variable» of a wall and of a wall type: a batter («talud») on
//! one face or both and a strip footing («zarpa») at its base, with a
//! small section across to see it.
//!
//! The batter is written as a slope «1:10» (one across for ten up), as a
//! percentage «10 %» or as the angle of the face «5,7°»; or the thickness
//! at the foot of the faces is given and the batter worked out from it.

use archisim_core::{Layer, Wall, WallFooting, WallId, WallSection, USUAL_BATTER, USUAL_FOOTING};
use eframe::egui::{self, Color32, Id, Pos2, Rect, Sense, Shape, Stroke, Ui, Vec2};

use super::props::{choice, group, note, row, text_row};
use super::*;

/// What the section is like, as chosen in the list.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Shape_ {
    None,
    Batter,
    Footing,
    Both,
}

impl Shape_ {
    fn of(s: &WallSection) -> Self {
        let batter = s.batter_ext > 1e-9 || s.batter_int > 1e-9;
        match (batter, s.footing.is_some()) {
            (false, false) => Shape_::None,
            (true, false) => Shape_::Batter,
            (false, true) => Shape_::Footing,
            (true, true) => Shape_::Both,
        }
    }

    /// The section with this shape, keeping what it had.
    fn apply(self, s: &WallSection) -> WallSection {
        let mut s = *s;
        let batter = matches!(self, Shape_::Batter | Shape_::Both);
        let footing = matches!(self, Shape_::Footing | Shape_::Both);
        if batter && s.batter_ext < 1e-9 && s.batter_int < 1e-9 {
            s.batter_ext = USUAL_BATTER;
        }
        if !batter {
            s.batter_ext = 0.0;
            s.batter_int = 0.0;
        }
        s.footing = if footing {
            Some(s.footing.unwrap_or(USUAL_FOOTING))
        } else {
            None
        };
        s
    }
}

/// A batter as the slope «1:n» (`0` for none).
pub(super) fn fmt_batter(b: f64) -> String {
    if b < 1e-9 {
        return "0".into();
    }
    let n = 1.0 / b;
    let n = (n * 100.0).round() / 100.0;
    format!("1:{n}").replace('.', ",")
}

/// A batter written as «1:10», «10 %», «5,7°» or as the horizontal per
/// vertical («0,1»).
pub(super) fn parse_batter(text: &str) -> Option<f64> {
    let t = text.trim().replace(',', ".").replace(' ', "");
    let b = if let Some((a, n)) = t.split_once(':') {
        let (a, n): (f64, f64) = (a.parse().ok()?, n.parse().ok()?);
        if n <= 0.0 {
            return None;
        }
        a / n
    } else if let Some(p) = t.strip_suffix('%') {
        p.parse::<f64>().ok()? / 100.0
    } else if let Some(d) = t.strip_suffix('°').or_else(|| t.strip_suffix('º')) {
        WallSection::from_angle(d.parse().ok()?)
    } else {
        t.parse().ok()?
    };
    (b.is_finite() && (0.0..=2.0).contains(&b)).then_some(b)
}

fn batter_field(ui: &mut Ui, id: Id, value: f64, width: f32) -> Option<f64> {
    let text = super::text_edit_committed(ui, id, &fmt_batter(value), width)?;
    parse_batter(&text)
}

/// The rows of a section: its shape, batters, the thickness at the foot,
/// the layer that grows and the footing. `height`: the wall's, to show
/// and edit the thickness at the foot. Returns the section changed.
pub(super) fn section_rows(
    ui: &mut Ui,
    salt: &str,
    s: &WallSection,
    layers: &[Layer],
    layer_names: &[String],
    height: Option<f64>,
) -> Option<WallSection> {
    let mut out: Option<WallSection> = None;
    let shape = Shape_::of(s);
    row(
        ui,
        "Forma",
        "Talud: las caras inclinadas, más grueso abajo. Zarpa: zapata corrida en la base, parte del muro",
        |ui, wd| {
            let picked = choice(
                ui,
                (salt, "shape"),
                wd,
                shape,
                &[
                    (Shape_::None, "Constante"),
                    (Shape_::Batter, "Talud"),
                    (Shape_::Footing, "Zarpa"),
                    (Shape_::Both, "Talud y zarpa"),
                ],
            );
            if picked != shape {
                out = Some(picked.apply(s));
            }
        },
    );
    if shape == Shape_::None {
        return out;
    }
    let crown: f64 = layers.iter().map(|l| l.thickness).sum();
    if matches!(shape, Shape_::Batter | Shape_::Both) {
        for (ext, label) in [(true, "Talud exterior"), (false, "Talud interior")] {
            let now = if ext { s.batter_ext } else { s.batter_int };
            row(
                ui,
                label,
                "Lo que sale la cara por cada metro hacia abajo: 1:10, 10 % o 5,7°; 0, a plomo",
                |ui, wd| {
                    if let Some(b) = batter_field(ui, Id::unique((salt, label)), now, wd - 8.0) {
                        let mut n = *s;
                        if ext {
                            n.batter_ext = b;
                        } else {
                            n.batter_int = b;
                        }
                        out = Some(n);
                    }
                },
            );
        }
        if let Some(h) = height {
            let stem = (h - s.footing_depth()).max(0.0);
            row(
                ui,
                "Espesor en la base (mm)",
                "Al pie de las caras; cambiarlo cambia los taludes, en la misma proporción",
                |ui, wd| {
                    let base = s.base_thickness(crown, h);
                    if let Some(v) = length_field(ui, Id::unique((salt, "base")), base, wd - 8.0) {
                        let total = s.batter_ext + s.batter_int;
                        let share = if total > 1e-9 {
                            s.batter_ext / total
                        } else {
                            1.0
                        };
                        let (e, i) = WallSection::batters_for(crown, v, stem, share);
                        out = Some(WallSection {
                            batter_ext: e,
                            batter_int: i,
                            ..*s
                        });
                    }
                },
            );
        }
        if layers.len() > 1 {
            let k = s.varying_layer(layers);
            let mut options: Vec<(Option<usize>, &str)> =
                vec![(None, "La estructural (automática)")];
            for (i, n) in layer_names.iter().enumerate() {
                options.push((Some(i), n.as_str()));
            }
            row(
                ui,
                "Capa que crece",
                "Las demás capas siguen su cara con su espesor",
                |ui, wd| {
                    let picked = choice(ui, (salt, "layer"), wd, s.layer, &options);
                    if picked != s.layer {
                        out = Some(WallSection {
                            layer: picked,
                            ..*s
                        });
                    }
                },
            );
            if s.layer.is_none() {
                note(
                    ui,
                    &format!("Crece «{}».", layer_names.get(k).map_or("", |n| n.as_str())),
                );
            }
        }
    }
    if let Some(f) = s.footing {
        for (key, label, hint, value) in [
            (
                0,
                "Vuelo exterior (mm)",
                "Lo que sale la zarpa por fuera del pie de la cara exterior (el talón, si las tierras van por fuera)",
                f.toe_ext,
            ),
            (
                1,
                "Vuelo interior (mm)",
                "Lo que sale por dentro (la puntera)",
                f.toe_int,
            ),
            (2, "Canto (mm)", "Desde la base del muro", f.depth),
        ] {
            row(ui, label, hint, |ui, wd| {
                if let Some(v) = length_field(ui, Id::unique((salt, label)), value, wd - 8.0) {
                    let mut g = f;
                    match key {
                        0 => g.toe_ext = v.max(0.0),
                        1 => g.toe_int = v.max(0.0),
                        _ => g.depth = v.max(0.01),
                    }
                    out = Some(WallSection {
                        footing: Some(g),
                        ..*s
                    });
                }
            });
        }
    }
    if let Some(h) = height {
        let base = s.base_thickness(crown, h);
        text_row(
            ui,
            "Resumen",
            "Superficie de la sección (alzado y zarpa)",
            &format!(
                "{} m en coronación · {} en la base · {} m²",
                fmt_two(crown),
                fmt_two(base),
                fmt_two(s.area(crown, h))
            ),
        );
        section_picture(ui, s, crown, h);
    }
    out
}

fn fmt_two(v: f64) -> String {
    format!("{v:.2}").replace('.', ",")
}

/// The section across, exterior on the left, to scale.
pub(super) fn section_picture(ui: &mut Ui, s: &WallSection, crown: f64, h: f64) {
    let f = s.footing.unwrap_or(WallFooting {
        toe_ext: 0.0,
        toe_int: 0.0,
        depth: 0.0,
    });
    let (e, i) = s.leans_at(h, 0.0);
    let d = f.depth;
    // (w from the crown's exterior face, inwards; z up)
    let mut pts = vec![(-e - f.toe_ext, 0.0), (crown + i + f.toe_int, 0.0)];
    if d > 0.0 {
        pts.extend([(crown + i + f.toe_int, d), (crown + i, d)]);
    }
    pts.extend([(crown, h), (0.0, h)]);
    if d > 0.0 {
        pts.extend([(-e, d), (-e - f.toe_ext, d)]);
    }
    let (x0, x1) = (-e - f.toe_ext, crown + i + f.toe_int);
    let width = ui.available_width().min(220.0);
    let height = 120.0_f32;
    let (rect, _) = ui.allocate_exact_size(Vec2::new(width, height), Sense::hover());
    let k = ((width as f64 - 20.0) / (x1 - x0).max(0.01)).min((height as f64 - 16.0) / h.max(0.01));
    let cx = rect.center().x as f64 - (x0 + x1) / 2.0 * k;
    let to =
        |(w, z): (f64, f64)| Pos2::new((cx + w * k) as f32, rect.bottom() - 8.0 - (z * k) as f32);
    let painter = ui.painter_at(rect);
    let pal = super::theme::Palette::current(ui.ctx());
    let shape: Vec<Pos2> = pts.iter().map(|p| to(*p)).collect();
    // Concave (stem on footing): its two parts.
    let stem = vec![
        to((-e, d)),
        to((crown + i, d)),
        to((crown, h)),
        to((0.0, h)),
    ];
    painter.add(Shape::convex_polygon(
        stem,
        Color32::from_gray(200),
        Stroke::NONE,
    ));
    if d > 0.0 {
        let foot = Rect::from_two_pos(to((x0, 0.0)), to((x1, d)));
        painter.rect_filled(foot, 0.0, Color32::from_gray(200));
    }
    let mut closed = shape.clone();
    closed.push(shape[0]);
    painter.add(Shape::line(closed, Stroke::new(1.2, pal.text)));
    // The ground line under it.
    painter.line_segment(
        [
            Pos2::new(rect.left() + 4.0, rect.bottom() - 8.0),
            Pos2::new(rect.right() - 4.0, rect.bottom() - 8.0),
        ],
        Stroke::new(0.6, pal.muted),
    );
    painter.text(
        Pos2::new(rect.left() + 4.0, rect.top() + 2.0),
        egui::Align2::LEFT_TOP,
        "ext.",
        egui::FontId::proportional(10.0),
        pal.muted,
    );
    painter.text(
        Pos2::new(rect.right() - 4.0, rect.top() + 2.0),
        egui::Align2::RIGHT_TOP,
        "int.",
        egui::FontId::proportional(10.0),
        pal.muted,
    );
}

impl ArchiSimApp {
    /// The «Sección variable» group of a straight wall's properties: its
    /// own section, or its type's.
    pub(super) fn wall_section_group(&mut self, ui: &mut Ui, id: WallId, w: &Wall) {
        if w.is_curved() {
            return;
        }
        let m = self.ed.model();
        let Some(t) = m.wall_type(w.wall_type) else {
            return;
        };
        let layers = t.layers.clone();
        let names: Vec<String> = layers
            .iter()
            .map(|l| {
                m.material(l.material)
                    .map_or(String::new(), |x| x.name.clone())
            })
            .collect();
        let own = w.section;
        let effective = own.or(t.section).unwrap_or_default();
        let mut set: Option<Option<WallSection>> = None;
        group(ui, "wall-section", "Sección variable", |ui| {
            match (own, t.section) {
                (None, Some(_)) => note(
                    ui,
                    "La del tipo; cambiarla aquí la hace propia de este muro.",
                ),
                (None, None) => note(
                    ui,
                    "Muros de contención y de mampostería: caras en talud y zarpa en la base.",
                ),
                _ => {}
            }
            if let Some(s) = section_rows(
                ui,
                "wall-section",
                &effective,
                &layers,
                &names,
                Some(w.height),
            ) {
                set = Some(Some(s));
            }
            if own.is_some() {
                ui.horizontal(|ui| {
                    ui.add_space(4.0);
                    if ui
                        .button("Como el tipo")
                        .on_hover_text("Quita la sección propia: el muro vuelve a la de su tipo")
                        .clicked()
                    {
                        set = Some(None);
                    }
                });
            }
        });
        if let Some(s) = set {
            let r = self
                .ed
                .transact("Sección del muro", |m| m.set_wall_section(id, s));
            self.report(r, "Sección del muro cambiada.");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn batters_are_read_every_way() {
        assert_eq!(fmt_batter(0.1), "1:10");
        assert_eq!(fmt_batter(0.0), "0");
        assert!((parse_batter("1:10").unwrap() - 0.1).abs() < 1e-12);
        assert!((parse_batter("1 : 20").unwrap() - 0.05).abs() < 1e-12);
        assert!((parse_batter("10 %").unwrap() - 0.1).abs() < 1e-12);
        assert!((parse_batter("45°").unwrap() - 1.0).abs() < 1e-9);
        assert!((parse_batter("0,1").unwrap() - 0.1).abs() < 1e-12);
        assert!(parse_batter("1:0").is_none());
        assert!(parse_batter("x").is_none());
        assert!(parse_batter("-0,1").is_none());
        let s = WallSection::default();
        assert_eq!(Shape_::of(&s), Shape_::None);
        let b = Shape_::Both.apply(&s);
        assert_eq!(b.batter_ext, USUAL_BATTER);
        assert_eq!(b.footing, Some(USUAL_FOOTING));
        assert_eq!(Shape_::of(&Shape_::Footing.apply(&b)), Shape_::Footing);
    }
}
