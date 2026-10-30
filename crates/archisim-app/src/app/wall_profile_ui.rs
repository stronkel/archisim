//! A wall's elevation profile (E3, Revit's «Editar perfil»): in its
//! properties, the shapes ready to use (gable, sloping top) and a window
//! where its outline and holes are drawn over the wall seen from outside.
//!
//! In the window: drag a corner (it snaps to the centimetre and to the
//! wall's rectangle), double-click an edge to add a corner, right-click a
//! corner to take it away; «Hueco» adds a square or round hole.

use archisim_core::{Point2, Wall, WallId, WallProfile};
use eframe::egui::{self, Color32, Pos2, Rect, Sense, Shape, Stroke, Ui, Vec2};

use super::props::{group, text_row};
use super::*;

/// The profile being edited in its window.
#[derive(Debug, Clone)]
pub(crate) struct ProfileEdit {
    pub wall: WallId,
    pub profile: WallProfile,
    pub length: f64,
    pub height: f64,
    /// The corner picked: ring (0 outline, k hole k − 1) and index.
    pub picked: Option<(usize, usize)>,
    dragging: bool,
}

impl ProfileEdit {
    pub fn new(w: &Wall) -> Self {
        let (length, height) = (w.length(), w.height);
        Self {
            wall: w.id,
            profile: w
                .profile
                .clone()
                .unwrap_or_else(|| WallProfile::rectangle(length, height)),
            length,
            height,
            picked: None,
            dragging: false,
        }
    }

    fn ring(&self, r: usize) -> &Vec<Point2> {
        if r == 0 {
            &self.profile.outline
        } else {
            &self.profile.holes[r - 1]
        }
    }

    fn ring_mut(&mut self, r: usize) -> &mut Vec<Point2> {
        if r == 0 {
            &mut self.profile.outline
        } else {
            &mut self.profile.holes[r - 1]
        }
    }

    fn rings(&self) -> usize {
        1 + self.profile.holes.len()
    }

    /// Adds a hole in the middle of the wall: square or round.
    pub fn add_hole(&mut self, round: bool) {
        let c = Point2::new(self.length / 2.0, self.height * 0.6);
        let r = (self.length.min(self.height) * 0.15).clamp(0.2, 0.6);
        let hole: Vec<Point2> = if round {
            (0..24)
                .map(|i| {
                    let a = i as f64 * std::f64::consts::TAU / 24.0;
                    Point2::new(c.x + r * a.cos(), c.y + r * a.sin())
                })
                .collect()
        } else {
            vec![
                Point2::new(c.x - r, c.y - r),
                Point2::new(c.x + r, c.y - r),
                Point2::new(c.x + r, c.y + r),
                Point2::new(c.x - r, c.y + r),
            ]
        };
        self.profile.holes.push(hole);
        self.picked = Some((self.profile.holes.len(), 0));
    }

    /// Moves a corner, to the centimetre and onto the wall's rectangle
    /// when close (`snap` metres).
    pub fn move_corner(&mut self, r: usize, i: usize, to: Point2, snap: f64) {
        let round = |v: f64| (v * 100.0).round() / 100.0;
        let mut p = Point2::new(round(to.x), round(to.y));
        for edge in [0.0, self.length] {
            if (p.x - edge).abs() < snap {
                p.x = edge;
            }
        }
        for edge in [0.0, self.height] {
            if (p.y - edge).abs() < snap {
                p.y = edge;
            }
        }
        if let Some(q) = self.ring_mut(r).get_mut(i) {
            *q = p;
        }
    }

    /// Adds a corner on edge `i` (from corner `i` to the next) at `at`.
    pub fn insert_corner(&mut self, r: usize, i: usize, at: Point2) {
        let ring = self.ring_mut(r);
        ring.insert(i + 1, at);
        self.picked = Some((r, i + 1));
    }

    /// Takes a corner away; a ring left with fewer than three goes (the
    /// outline never does).
    pub fn remove_corner(&mut self, r: usize, i: usize) {
        if self.ring(r).len() > 3 {
            self.ring_mut(r).remove(i);
        } else if r > 0 {
            self.profile.holes.remove(r - 1);
        }
        self.picked = None;
    }
}

impl ArchiSimApp {
    /// The «Perfil en alzado» group of a straight wall's properties.
    pub(super) fn wall_profile_group(&mut self, ui: &mut Ui, id: WallId, w: &Wall) {
        if w.is_curved() {
            return;
        }
        let (len, h) = (w.length(), w.height);
        let mut set: Option<Option<WallProfile>> = None;
        group(ui, "wall-profile", "Perfil en alzado", |ui| {
            match &w.profile {
                Some(p) => text_row(
                    ui,
                    "Perfil",
                    "Visto desde la cara exterior",
                    &format!(
                        "{} vértices · {} huecos · {} m²",
                        p.outline.len(),
                        p.holes.len(),
                        format!("{:.2}", p.area()).replace('.', ",")
                    ),
                ),
                None => props::note(
                    ui,
                    "El muro es su rectángulo. Edita el perfil para recortarlo con la forma que quieras (un hastial, un remate inclinado, huecos sin carpintería).",
                ),
            }
            ui.horizontal_wrapped(|ui| {
                ui.add_space(4.0);
                if ui
                    .button("Editar perfil…")
                    .on_hover_text("El muro visto desde fuera: arrastra sus vértices, doble clic en un lado para añadir uno")
                    .clicked()
                {
                    self.profile_edit = Some(ProfileEdit::new(w));
                }
                if ui
                    .button("Hastial")
                    .on_hover_text("Sube hasta la mitad: 1,50 m más alto en el centro")
                    .clicked()
                {
                    set = Some(Some(WallProfile::gable(len, h, h + 1.5)));
                }
                if ui
                    .button("Inclinado")
                    .on_hover_text("Remate inclinado: 1,00 m más bajo al final")
                    .clicked()
                {
                    set = Some(Some(WallProfile::sloping(len, h, (h - 1.0).max(0.3))));
                }
                if w.profile.is_some() && ui.button("Quitar perfil").clicked() {
                    set = Some(None);
                }
            });
        });
        if let Some(p) = set {
            let r = self
                .ed
                .transact("Perfil del muro", |m| m.set_wall_profile(id, p));
            self.report(r, "Perfil del muro cambiado.");
        }
    }

    /// The window where a wall's profile is drawn.
    pub(super) fn wall_profile_dialog(&mut self, ctx: &egui::Context) {
        let Some(mut edit) = self.profile_edit.take() else {
            return;
        };
        if self.ed.model().wall(edit.wall).is_none() {
            return;
        }
        let openings: Vec<(f64, f64, f64, f64)> = {
            let m = self.ed.model();
            let base = m.wall(edit.wall).map_or(0.0, |w| w.base_offset);
            archisim_geom::openings_in_wall(m, edit.wall)
                .iter()
                .map(|o| (o.t0, o.t1, o.sill - base, o.head - base))
                .collect()
        };
        let mut open = true;
        let mut apply = false;
        let mut cancel = false;
        super::widgets::dialog("Perfil del muro")
            .open(&mut open)
            .pivot(egui::Align2::CENTER_CENTER)
            .default_pos(ctx.content_rect().center())
            .default_size([680.0, 470.0])
            .show(ctx, |ui| {
                ui.label("Visto desde la cara exterior, del inicio del muro (izquierda) a su final. Arrastra los vértices; doble clic en un lado añade uno; clic derecho en un vértice lo quita.");
                ui.add_space(4.0);
                profile_canvas(ui, &mut edit, &openings);
                ui.add_space(6.0);
                if let Some((r, i)) = edit.picked {
                    if let Some(q) = edit.ring(r).get(i).copied() {
                        ui.horizontal(|ui| {
                            ui.label(if r == 0 { "Vértice del contorno" } else { "Vértice del hueco" });
                            let (mut x, mut z) = (q.x, q.y);
                            ui.label("a lo largo (m)");
                            let a = ui.add(
                                egui::DragValue::new(&mut x)
                                    .speed(0.01)
                                    .max_decimals(2)
                                    .custom_formatter(|v, _| format!("{v:.2}").replace('.', ","))
                                    .custom_parser(|t| t.trim().replace(',', ".").parse().ok()),
                            );
                            ui.label("altura (m)");
                            let b = ui.add(
                                egui::DragValue::new(&mut z)
                                    .speed(0.01)
                                    .max_decimals(2)
                                    .custom_formatter(|v, _| format!("{v:.2}").replace('.', ","))
                                    .custom_parser(|t| t.trim().replace(',', ".").parse().ok()),
                            );
                            if a.changed() || b.changed() {
                                edit.move_corner(r, i, Point2::new(x, z), 0.0);
                            }
                            if ui.button("Quitar vértice").clicked() {
                                edit.remove_corner(r, i);
                            }
                            if r > 0 && ui.button("Quitar hueco").clicked() {
                                edit.profile.holes.remove(r - 1);
                                edit.picked = None;
                            }
                        });
                    }
                }
                ui.horizontal(|ui| {
                    if ui.button("Hueco cuadrado").clicked() {
                        edit.add_hole(false);
                    }
                    if ui.button("Hueco redondo").clicked() {
                        edit.add_hole(true);
                    }
                    if ui
                        .button("Rectángulo")
                        .on_hover_text("Vuelve a empezar desde el rectángulo del muro")
                        .clicked()
                    {
                        edit.profile = WallProfile::rectangle(edit.length, edit.height);
                        edit.picked = None;
                    }
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button("Aplicar").clicked() {
                            apply = true;
                        }
                        if ui.button("Cancelar").clicked() {
                            cancel = true;
                        }
                    });
                });
            });
        if apply {
            let (id, p) = (edit.wall, edit.profile.clone());
            let r = self
                .ed
                .transact("Perfil del muro", |m| m.set_wall_profile(id, Some(p)));
            match r {
                Ok(()) => self.say("Perfil del muro aplicado."),
                Err(e) => {
                    self.complain(format!("No se puede: {e}."));
                    self.profile_edit = Some(edit);
                }
            }
            return;
        }
        if open && !cancel {
            self.profile_edit = Some(edit);
        }
    }
}

/// The wall in elevation with its profile, and the corners to drag.
fn profile_canvas(ui: &mut Ui, edit: &mut ProfileEdit, openings: &[(f64, f64, f64, f64)]) {
    let size = Vec2::new(ui.available_width().max(400.0), 320.0);
    let (rect, resp) = ui.allocate_exact_size(size, Sense::click_and_drag());
    let painter = ui.painter_at(rect);
    painter.rect_filled(rect, 4.0, Color32::WHITE);
    let top = edit.height.max(edit.profile.top());
    let (x0, x1) = (-0.4, edit.length + 0.4);
    let (z0, z1) = (-0.3, top + 0.6);
    let k =
        ((rect.width() - 20.0) as f64 / (x1 - x0)).min((rect.height() - 20.0) as f64 / (z1 - z0));
    let origin = Pos2::new(
        rect.center().x - ((x0 + x1) / 2.0 * k) as f32,
        rect.center().y + ((z0 + z1) / 2.0 * k) as f32,
    );
    let to_screen = |p: Point2| Pos2::new(origin.x + (p.x * k) as f32, origin.y - (p.y * k) as f32);
    let to_model = |s: Pos2| Point2::new((s.x - origin.x) as f64 / k, (origin.y - s.y) as f64 / k);
    let ink = Color32::from_rgb(40, 44, 52);
    let accent = Color32::from_rgb(37, 99, 235);
    // The ground line and the wall's rectangle.
    painter.line_segment(
        [
            to_screen(Point2::new(x0, 0.0)),
            to_screen(Point2::new(x1, 0.0)),
        ],
        Stroke::new(1.0, Color32::from_gray(150)),
    );
    let rect_pts: Vec<Pos2> = WallProfile::rectangle(edit.length, edit.height)
        .outline
        .iter()
        .map(|p| to_screen(*p))
        .collect();
    painter.add(Shape::dashed_line(
        &[rect_pts.clone(), vec![rect_pts[0]]].concat(),
        Stroke::new(1.0, Color32::from_gray(160)),
        6.0,
        4.0,
    ));
    // The wall as it will be: its outline filled, holes white.
    let outline: Vec<Pos2> = edit.profile.outline.iter().map(|p| to_screen(*p)).collect();
    let fill = Color32::from_rgb(232, 196, 178);
    for t in archisim_geom::solid::triangulate(&edit.profile.outline) {
        painter.add(Shape::convex_polygon(
            t.iter().map(|i| outline[*i]).collect(),
            fill,
            Stroke::NONE,
        ));
    }
    for h in &edit.profile.holes {
        let pts: Vec<Pos2> = h.iter().map(|p| to_screen(*p)).collect();
        for t in archisim_geom::solid::triangulate(h) {
            painter.add(Shape::convex_polygon(
                t.iter().map(|i| pts[*i]).collect(),
                Color32::WHITE,
                Stroke::NONE,
            ));
        }
    }
    // Doors and windows, for reference.
    for (a, b, lo, hi) in openings {
        let r = Rect::from_two_pos(
            to_screen(Point2::new(*a, *lo)),
            to_screen(Point2::new(*b, *hi)),
        );
        painter.rect_stroke(
            r,
            0.0,
            Stroke::new(1.0, Color32::from_gray(120)),
            egui::StrokeKind::Inside,
        );
    }
    for r in 0..edit.rings() {
        let pts: Vec<Pos2> = edit.ring(r).iter().map(|p| to_screen(*p)).collect();
        painter.add(Shape::closed_line(pts.clone(), Stroke::new(1.6, ink)));
        for (i, p) in pts.iter().enumerate() {
            let on = edit.picked == Some((r, i));
            painter.circle(
                *p,
                if on { 5.0 } else { 4.0 },
                if on { accent } else { Color32::WHITE },
                Stroke::new(1.4, if on { accent } else { ink }),
            );
        }
    }
    // The nearest corner and edge to the pointer.
    let hit = |edit: &ProfileEdit, s: Pos2| -> Option<(usize, usize)> {
        let mut best = None;
        let mut d = 9.0f32;
        for r in 0..edit.rings() {
            for (i, p) in edit.ring(r).iter().enumerate() {
                let e = to_screen(*p).distance(s);
                if e < d {
                    d = e;
                    best = Some((r, i));
                }
            }
        }
        best
    };
    let pointer = resp.interact_pointer_pos().or(resp.hover_pos());
    if resp.drag_started() {
        if let Some(s) = resp.interact_pointer_pos() {
            edit.picked = hit(edit, s);
            edit.dragging = edit.picked.is_some();
        }
    }
    if resp.dragged() && edit.dragging {
        if let (Some((r, i)), Some(s)) = (edit.picked, pointer) {
            edit.move_corner(r, i, to_model(s), 8.0 / k);
        }
    }
    if resp.drag_stopped() {
        edit.dragging = false;
    }
    if resp.clicked() {
        if let Some(s) = pointer {
            edit.picked = hit(edit, s);
        }
    }
    if resp.secondary_clicked() {
        if let Some((r, i)) = pointer.and_then(|s| hit(edit, s)) {
            edit.remove_corner(r, i);
        }
    }
    if resp.double_clicked() {
        if let Some(s) = pointer {
            let q = to_model(s);
            let mut best: Option<(usize, usize, f64)> = None;
            for r in 0..edit.rings() {
                let ring = edit.ring(r);
                let n = ring.len();
                for i in 0..n {
                    let (a, b) = (ring[i], ring[(i + 1) % n]);
                    let ab = b - a;
                    let t = ((q - a).dot(ab) / ab.dot(ab).max(1e-12)).clamp(0.0, 1.0);
                    let d = q.distance(a + ab * t);
                    if best.is_none_or(|x| d < x.2) {
                        best = Some((r, i, d));
                    }
                }
            }
            if let Some((r, i, d)) = best {
                if d * k < 10.0 {
                    let round = |v: f64| (v * 100.0).round() / 100.0;
                    edit.insert_corner(r, i, Point2::new(round(q.x), round(q.y)));
                }
            }
        }
    }
}
