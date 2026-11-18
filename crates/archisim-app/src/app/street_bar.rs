//! The street bar (N10, `docs/TERRITORIO.md` §2.13): while drawing streets,
//! a small bar over the drawing (in plan, in 3D and in the Maqueta) with
//! everything at hand, as in Cities: Skylines 2: the type of street, how
//! it is drawn (continuous curve, straight, curve, whole axis), the grid,
//! the parallel street and «Mejorar» (another type on a street that is
//! there), following the ground, anarchy and the height (PgUp and PgDn).
//! While a stretch is drawn, its length, turn, radius and grade go by the
//! cursor.

use archisim_core::{LevelId, Model, ModelError, Point2, RoadId, Vec2};
use eframe::egui::{self, Color32, CornerRadius, Id, Pos2, Rect, Shape, Stroke};

use super::theme::Palette;

use super::urban_ui::RoadDraw;
use super::*;

/// What the clicks of the street tool do, besides drawing streets.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RoadAct {
    /// A grid of streets: a corner, the end of its first side and how deep
    /// it goes; a street every `grid_spacing` both ways.
    Grid,
    /// A street parallel to the one clicked, on the side clicked.
    Parallel,
    /// The type and materials chosen put on the street clicked (its axis
    /// stays): «Mejorar» in Cities: Skylines.
    Upgrade,
}

impl RoadAct {
    fn steps(self) -> usize {
        match self {
            RoadAct::Grid => 3,
            RoadAct::Parallel | RoadAct::Upgrade => 1,
        }
    }
}

/// The streets of a grid: from `a`, a first side to `b` and as deep as
/// `c` is from it, a street about every `spacing` both ways (the spacing
/// fitted so that both sides end on a street). Each street end to end.
pub fn grid_streets(a: Point2, b: Point2, c: Point2, spacing: f64) -> Vec<[Point2; 2]> {
    let side = b - a;
    let long = side.length();
    if long < 1.0 || spacing < 5.0 {
        return Vec::new();
    }
    let u = side * (1.0 / long);
    let mut v = Vec2::new(-u.y, u.x);
    let mut deep = (c - a).dot(v);
    if deep < 0.0 {
        v = v * -1.0;
        deep = -deep;
    }
    let count = |l: f64| ((l / spacing).round() as usize).max(1);
    let (nu, nv) = (count(long), if deep < 1.0 { 0 } else { count(deep) });
    let mut out = Vec::new();
    // Along the first side, one every row.
    for k in 0..=nv {
        let off = if nv == 0 {
            0.0
        } else {
            deep * k as f64 / nv as f64
        };
        out.push([a + v * off, b + v * off]);
    }
    if nv > 0 {
        for k in 0..=nu {
            let off = long * k as f64 / nu as f64;
            out.push([a + u * off, a + u * off + v * deep]);
        }
    }
    out
}

/// Where the streets of a grid cross: for their zebras.
fn grid_crossings(lines: &[[Point2; 2]]) -> Vec<Point2> {
    let mut out = Vec::new();
    for (i, p) in lines.iter().enumerate() {
        for q in &lines[i + 1..] {
            let (r, s) = (p[1] - p[0], q[1] - q[0]);
            let d = r.cross(s);
            if d.abs() < 1e-9 {
                continue;
            }
            let t = (q[0] - p[0]).cross(s) / d;
            let w = (q[0] - p[0]).cross(r) / d;
            if (-1e-6..=1.0 + 1e-6).contains(&t) && (-1e-6..=1.0 + 1e-6).contains(&w) {
                out.push(p[0] + r * t);
            }
        }
    }
    out
}

/// Lays a grid of streets with the tool's settings, in one step; how many.
pub fn lay_grid(
    model: &mut Model,
    level: LevelId,
    lines: &[[Point2; 2]],
    s: &super::urban_ui::RoadSettings,
) -> Result<usize, ModelError> {
    for l in lines {
        let id = model.add_road_stretch(archisim_core::RoadStretch {
            level,
            points: l.to_vec(),
            continues: None,
            section: s.section.clone(),
            top_offset: s.top_offset,
            pavement: s.pavement,
            kerb_material: s.kerb,
            sidewalk_material: s.sidewalk,
            follow_ground: (s.follow_ground && !s.anarchy).then_some(s.max_grade),
        })?;
        if s.anarchy {
            model.set_road_free(id, true)?;
        }
    }
    if !s.anarchy {
        for q in grid_crossings(lines) {
            model.crossings_at(level, q)?;
        }
    }
    Ok(lines.len())
}

/// The radius of the circle through three points (None: in line).
fn radius_through(a: Point2, b: Point2, c: Point2) -> Option<f64> {
    let (ab, bc, ca) = (a.distance(b), b.distance(c), c.distance(a));
    let area2 = (b - a).cross(c - a).abs();
    (area2 > 1e-6).then(|| ab * bc * ca / (2.0 * area2))
}

impl ArchiSimApp {
    /// A click of the street tool while it does something else than draw.
    /// True if it took the click.
    pub(super) fn road_act_click(&mut self, p: Point2) -> bool {
        let Some(act) = self.road_settings.act else {
            return false;
        };
        let level = self.ed.level;
        match act {
            RoadAct::Grid => {
                self.road_points.push(p);
                if self.road_points.len() < act.steps() {
                    return true;
                }
                let pts = std::mem::take(&mut self.road_points);
                let lines = grid_streets(pts[0], pts[1], pts[2], self.road_settings.grid_spacing);
                if lines.is_empty() {
                    self.complain("La cuadrícula es demasiado pequeña: dale al menos un lado de varios metros.");
                    return true;
                }
                let s = self.road_settings.clone();
                let mut n = 0;
                let r = self.ed.transact("Cuadrícula de calles", |m| {
                    n = lay_grid(m, level, &lines, &s)?;
                    Ok::<(), ModelError>(())
                });
                self.report(
                    r.map_err(|e| e.to_string()),
                    &format!("Cuadrícula de {n} calles; los cruces se hacen solos."),
                );
            }
            RoadAct::Parallel | RoadAct::Upgrade => {
                let Some((id, _)) = archisim_core::road_axis_near(self.ed.model(), level, p, 2.0)
                else {
                    self.complain("Haz clic sobre una calle.");
                    return true;
                };
                if act == RoadAct::Parallel {
                    self.parallel_at(id, p);
                } else {
                    self.upgrade_road(id);
                }
            }
        }
        true
    }

    fn parallel_at(&mut self, id: RoadId, p: Point2) {
        let Some(r) = self.ed.model().road(id) else {
            return;
        };
        let side = if r.place_of(p).1 >= 0.0 { 1.0 } else { -1.0 };
        let d = side * self.road_settings.parallel;
        let mut made = None;
        let r = self.ed.transact("Calle paralela", |m| {
            made = Some(m.parallel_road(id, d)?);
            Ok::<(), ModelError>(())
        });
        if let (Ok(()), Some(new)) = (&r, made) {
            self.ed.selection = [Element::Road(new)].into_iter().collect();
        }
        self.report(
            r.map_err(|e| e.to_string()),
            &format!(
                "Calle paralela a {}.",
                crate::units::fmt_len(self.road_settings.parallel)
            ),
        );
    }

    /// The type and materials of the tool on a street that is there.
    fn upgrade_road(&mut self, id: RoadId) {
        let s = self.road_settings.clone();
        let r = self.ed.transact("Mejorar calle", |m| {
            m.edit_road(id, |r| {
                let mut sec = s.section.clone();
                // What is along it stays: its crossings and buildouts.
                sec.crossings = r.section.crossings.clone();
                sec.buildouts = r.section.buildouts.clone();
                r.section = sec;
                r.pavement = s.pavement;
                r.kerb_material = s.kerb;
                r.sidewalk_material = s.sidewalk;
            })
        });
        let name = archisim_core::street_type_of(&s.section)
            .map_or("su sección".to_string(), |t| t.name.to_string());
        self.report(
            r.map_err(|e| e.to_string()),
            &format!("Calle cambiada a {name}."),
        );
    }

    /// PgUp and PgDn with the street tool: the next stretch a metre higher
    /// or lower (bridges and underpasses, as in a city builder).
    pub(super) fn road_height_keys(&mut self, ctx: &egui::Context) -> bool {
        if self.tool != Tool::Road {
            return false;
        }
        let (up, down) = ctx.input(|i| {
            (
                i.key_pressed(egui::Key::PageUp),
                i.key_pressed(egui::Key::PageDown),
            )
        });
        if !(up || down) {
            return false;
        }
        let step = if up { 1.0 } else { -1.0 };
        let s = &mut self.road_settings;
        s.top_offset = ((s.top_offset + step) * 100.0).round() / 100.0;
        // Above or below the ground: it does not follow it any more.
        if s.top_offset.abs() > 1e-9 {
            s.follow_ground = false;
        }
        let h = s.top_offset;
        self.say(format!(
            "Calle a {} sobre la planta{}.",
            crate::units::fmt_len(h),
            if h.abs() > 1e-9 {
                " (no sigue el terreno)"
            } else {
                ""
            }
        ));
        true
    }

    /// The bar over the drawing while the street tool is on.
    pub(super) fn street_bar(&mut self, ctx: &egui::Context, area: Rect) {
        if self.tool != Tool::Road {
            return;
        }
        let p = Palette::current(ctx);
        let thumbs = self.street_thumbs(ctx);
        let origin = Pos2::new(area.center().x, area.bottom() - 14.0);
        let mut s = self.road_settings.clone();
        let mut restart = false;
        egui::Area::new(Id::unique("street-bar"))
            .order(egui::Order::Foreground)
            .pivot(egui::Align2::CENTER_BOTTOM)
            .fixed_pos(origin)
            .show(ctx, |ui| {
                egui::Frame::new()
                    .fill(p.panel)
                    .stroke(Stroke::new(1.0, p.line))
                    .corner_radius(CornerRadius::same(14))
                    .inner_margin(egui::Margin::symmetric(10, 6))
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.spacing_mut().item_spacing.x = 4.0;
                            // The type, and every type when clicked.
                            let name = archisim_core::street_type_of(&s.section)
                                .map_or("Sección propia".to_string(), |t| t.name.to_string());
                            ui.menu_button(egui::RichText::new(format!("▾ {name}")).strong(), |ui| {
                                ui.set_max_width(420.0);
                                if let Some(picked) =
                                    super::street_types_ui::street_type_cards(ui, 400.0, &s.section, &thumbs)
                                {
                                    let mut n = match picked {
                                        super::street_types_ui::PickedStreet::Type(sec) => sec,
                                        super::street_types_ui::PickedStreet::Saved(t) => t.section,
                                    };
                                    n.crossings = s.section.crossings.clone();
                                    n.buildouts = s.section.buildouts.clone();
                                    s.section = n;
                                    ui.close();
                                }
                            });
                            ui.separator();
                            for d in [RoadDraw::Smooth, RoadDraw::Straight, RoadDraw::Curve, RoadDraw::Polyline] {
                                let (short, hint) = match d {
                                    RoadDraw::Smooth => ("Continua", "Curva continua: cada tramo sale en la dirección del anterior"),
                                    RoadDraw::Straight => ("Recta", "Tramos rectos"),
                                    RoadDraw::Curve => ("Curva", "Un clic tira de la curva y el siguiente la acaba"),
                                    RoadDraw::Polyline => ("Eje", "El eje entero por sus puntos; doble clic acaba"),
                                };
                                let on = s.act.is_none() && s.draw == d;
                                if ui.selectable_label(on, short).on_hover_text(hint).clicked() {
                                    s.draw = d;
                                    s.act = None;
                                    restart = true;
                                }
                            }
                            ui.separator();
                            for (act, short, hint) in [
                                (RoadAct::Grid, "Cuadrícula", "Una esquina, el final del primer lado y el fondo: una calle cada separación en los dos sentidos"),
                                (RoadAct::Parallel, "Paralela", "Clic en una calle, del lado donde va la nueva"),
                                (RoadAct::Upgrade, "Mejorar", "Clic en una calle: le pone este tipo y estos materiales sin mover su eje"),
                            ] {
                                if ui
                                    .selectable_label(s.act == Some(act), short)
                                    .on_hover_text(hint)
                                    .clicked()
                                {
                                    s.act = if s.act == Some(act) { None } else { Some(act) };
                                    restart = true;
                                }
                            }
                            match s.act {
                                Some(RoadAct::Grid) => {
                                    ui.label("cada");
                                    ui.add(
                                        egui::DragValue::new(&mut s.grid_spacing)
                                            .range(20.0..=400.0)
                                            .speed(1.0)
                                            .suffix(" m"),
                                    );
                                }
                                Some(RoadAct::Parallel) => {
                                    ui.label("a");
                                    ui.add(
                                        egui::DragValue::new(&mut s.parallel)
                                            .range(5.0..=400.0)
                                            .speed(0.5)
                                            .suffix(" m"),
                                    );
                                }
                                _ => {}
                            }
                            ui.separator();
                            if ui
                                .selectable_label(s.follow_ground && !s.anarchy, "Terreno")
                                .on_hover_text("La calle sigue el terreno con su rasante suavizada")
                                .clicked()
                            {
                                s.follow_ground = !s.follow_ground;
                            }
                            if ui
                                .selectable_label(s.anarchy, "Anarquía")
                                .on_hover_text("Sin cruces ni terreno: queda a la cota que le des")
                                .clicked()
                            {
                                s.anarchy = !s.anarchy;
                            }
                            ui.separator();
                            if ui.small_button("−").on_hover_text("Un metro más baja (AvPág)").clicked() {
                                s.top_offset -= 1.0;
                                s.follow_ground &= s.top_offset.abs() < 1e-9;
                            }
                            ui.label(
                                egui::RichText::new(format!("{:+.1} m", s.top_offset).replace('.', ","))
                                    .monospace(),
                            )
                            .on_hover_text("Altura de la calle sobre la planta");
                            if ui.small_button("+").on_hover_text("Un metro más alta (RePág)").clicked() {
                                s.top_offset += 1.0;
                                s.follow_ground = false;
                            }
                        });
                    });
            });
        if restart {
            self.road_points.clear();
            self.road_chain = None;
            self.road_start_join = None;
        }
        self.road_settings = s;
    }

    /// Over the plan, while the street tool draws: the grid to be laid, and
    /// by the cursor the length, turn, radius and grade of the stretch.
    pub(super) fn draw_street_readout(&self, painter: &egui::Painter, rect: Rect) {
        if self.tool != Tool::Road {
            return;
        }
        let Some(c) = self.cursor.map(|c| c.point) else {
            return;
        };
        let accent = Color32::from_rgb(0x1f, 0x6f, 0xd1);
        let s = &self.road_settings;
        if s.act == Some(RoadAct::Grid) {
            let mut pts = self.road_points.clone();
            pts.push(c);
            let lines = match pts.len() {
                2 => vec![[pts[0], pts[1]]],
                3 => grid_streets(pts[0], pts[1], pts[2], s.grid_spacing),
                _ => Vec::new(),
            };
            for l in &lines {
                painter.add(Shape::line(
                    self.polygon(rect, l),
                    Stroke::new(2.0, accent.gamma_multiply(0.8)),
                ));
            }
            if pts.len() == 3 {
                self.readout(painter, rect, c, &format!("{} calles", lines.len()));
            }
            return;
        }
        if s.act.is_some() || self.road_points.is_empty() {
            return;
        }
        let pts = if s.draw.by_stretches() {
            match self.road_stretch_points(c) {
                Some(p) => p,
                None => return,
            }
        } else {
            let mut p = self.road_points.clone();
            p.push(c);
            p
        };
        let (a, b) = (pts[0], *pts.last().unwrap_or(&c));
        let length: f64 = pts.windows(2).map(|w| w[0].distance(w[1])).sum();
        let mut lines = vec![crate::units::fmt_len(length)];
        // Its direction: the bearing from north, and how much it turns
        // from the street it goes on from.
        let chord = b - a;
        if chord.length() > 1e-6 {
            let north = self.ed.model().true_north();
            let bearing =
                (90.0 - chord.y.atan2(chord.x).to_degrees() + north.to_degrees()).rem_euclid(360.0);
            lines.push(format!("rumbo {bearing:.0}°"));
        }
        if pts.len() >= 3 {
            if let Some(r) = radius_through(a, pts[pts.len() / 2], b) {
                if r < 5000.0 {
                    lines.push(format!("R {}", crate::units::fmt_len(r)));
                }
            }
        }
        if s.follow_ground && !s.anarchy && length > 1.0 {
            let m = self.ed.model();
            if let (Some(za), Some(zb)) = (
                crate::context_fit::ground_height(m, a),
                crate::context_fit::ground_height(m, b),
            ) {
                let g = (zb - za) / length * 100.0;
                lines.push(format!("{g:+.1} %").replace('.', ","));
            }
        } else if s.top_offset.abs() > 1e-9 {
            lines.push(format!("a {:+.1} m", s.top_offset).replace('.', ","));
        }
        self.readout(painter, rect, c, &lines.join(" · "));
    }

    /// A small label beside the cursor.
    fn readout(&self, painter: &egui::Painter, rect: Rect, at: Point2, text: &str) {
        let Some(q) = self.polygon(rect, &[at]).first().copied() else {
            return;
        };
        let font = egui::FontId::proportional(12.0);
        let galley = painter.layout_no_wrap(text.to_string(), font, Color32::WHITE);
        let pos = q + egui::vec2(16.0, 14.0);
        let r = Rect::from_min_size(pos, galley.size()).expand(4.0);
        painter.rect_filled(
            r,
            CornerRadius::same(5),
            Color32::from_rgb(0x1f, 0x6f, 0xd1),
        );
        painter.galley(pos, galley, Color32::WHITE);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_grid_fits_its_spacing() {
        let lines = grid_streets(
            Point2::new(0.0, 0.0),
            Point2::new(200.0, 0.0),
            Point2::new(50.0, 160.0),
            80.0,
        );
        // Three rows along the first side (0, 80, 160), and four across
        // (0, 66,7, 133,3, 200).
        assert_eq!(lines.len(), 3 + 4);
        assert!(lines.iter().any(|l| (l[0].y - 160.0).abs() < 1e-9));
        assert_eq!(grid_crossings(&lines).len(), 12);
        // Downwards too.
        let down = grid_streets(
            Point2::new(0.0, 0.0),
            Point2::new(200.0, 0.0),
            Point2::new(50.0, -160.0),
            80.0,
        );
        assert!(down.iter().any(|l| (l[0].y + 160.0).abs() < 1e-9));
    }

    #[test]
    fn three_points_give_their_radius() {
        let r = radius_through(
            Point2::new(-10.0, 0.0),
            Point2::new(0.0, 10.0),
            Point2::new(10.0, 0.0),
        )
        .unwrap();
        assert!((r - 10.0).abs() < 1e-9);
        assert!(radius_through(
            Point2::new(0.0, 0.0),
            Point2::new(1.0, 0.0),
            Point2::new(2.0, 0.0)
        )
        .is_none());
    }
}
