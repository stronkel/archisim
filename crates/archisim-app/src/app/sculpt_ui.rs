//! Modelling the concept volumes further (bloque 4 de la revisión del
//! 22/10/2026), as Antonio asked after trying Concepto:
//!
//! - Dividir: two clicks on a face of a volume divide it along that line;
//!   each part is painted or pushed on its own.
//! - Empujar takes any face now, not only a contour's top: a side (the
//!   contour's edge moves, in plan too), the base, a part of a divided
//!   face, any face of a solid — along its normal, out or in.
//! - Deformar: a volume twisted round its middle (moving the mouse round
//!   it) or, with Mayús, bent towards the mouse; the numbers in
//!   Properties.
//! - Seguir sweeps a profile: the plain rectangle, a tube, an L, a U, a
//!   double T, a cornice, a half-round, or one drawn (a closed polyline
//!   of the 2D drawing, or a face of Concepto).
//! - Redondear: rounds the edges of the volumes selected at once.
//!
//! The geometry is `archisim_geom::sculpt`; this is the clicks, the
//! previews and the rows of Properties.

use archisim_core::{Deform, MassId, MassShape, Point2};
use archisim_geom::sculpt;
use eframe::egui::{self, Color32, Id, Pos2, Rect, Shape, Stroke, Ui};

use super::props::{group, row};
use super::*;
use crate::editor::SweepProfile;
use crate::view3d::scene::Owner;

/// A face of a volume taken by a tool: the volume, its face, a point on
/// it (`z` over the volume's base) and its outward normal.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FacePick {
    pub mass: MassId,
    pub face: u32,
    pub point: [f64; 3],
    pub normal: [f64; 3],
}

/// Empujar on any face: the face taken, and how far it goes now.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FacePush {
    pub pick: FacePick,
    /// Where it was taken in plan.
    pub from: Point2,
    pub d: f64,
}

/// Deformar: the volume, its middle, the point taken and the twist and
/// bend it had.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DeformDrag {
    pub mass: MassId,
    pub centre: Point2,
    pub from: Point2,
    pub before: Deform,
    pub now: Deform,
}

/// How near a side of a volume a click in plan takes that side, metres
/// per screen point (as the other picks).
fn round_cm(v: f64) -> f64 {
    (v * 100.0).round() / 100.0
}

fn dot(a: [f64; 3], b: [f64; 3]) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

fn cross(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

impl ArchiSimApp {
    /// The floor height of a volume's base.
    fn mass_base_z(&self, id: MassId) -> f64 {
        let m = self.ed.model();
        m.mass(id).map_or(0.0, |x| {
            m.level(x.level).map_or(0.0, |l| l.elevation) + x.base_offset
        })
    }

    /// The face of a volume the cursor is on: in 3D the one hit; in plan,
    /// a side if the click is by one (`sides`), else the highest face
    /// looking up.
    pub(super) fn face_under(&self, p: Point2, sides: bool) -> Option<FacePick> {
        if self.mode == ViewMode::ThreeD {
            let c = self.cursor3d?;
            let Some(Owner::Model(Element::Mass(id))) = c.owner else {
                return None;
            };
            let n = c.normal?;
            let model = self.ed.model();
            let face = archisim_geom::solid::face_of(
                model,
                archisim_geom::solid::SolidOwner::Mass(id),
                [c.world.x as f64, c.world.y as f64, c.world.z as f64],
                [n.x as f64, n.y as f64, n.z as f64],
            )?;
            let m = model.mass(id)?;
            let face = m.face_number(face)?;
            let body = archisim_geom::forms::mass_body(m);
            let normal = sculpt::face_normal(&body, face)?;
            let z = c.world.z as f64 - self.mass_base_z(id);
            return Some(FacePick {
                mass: id,
                face,
                point: [c.world.x as f64, c.world.y as f64, z],
                normal,
            });
        }
        let reach = self.view.to_metres(REACH);
        let model = self.ed.model();
        if sides {
            // The side nearest the click, of any volume near it.
            let mut best: Option<(f64, FacePick)> = None;
            for m in model.masses_on_level(self.ed.level) {
                let body = archisim_geom::forms::mass_body(m);
                if let Some((face, point, normal, d)) = sculpt::side_face_near(&body, p, reach) {
                    if best.is_none_or(|b| d < b.0) {
                        best = Some((
                            d,
                            FacePick {
                                mass: m.id,
                                face,
                                point,
                                normal,
                            },
                        ));
                    }
                }
            }
            if let Some((_, pick)) = best {
                // Only when the click is not well inside the volume's top.
                let inside = model.mass(pick.mass).is_some_and(|m| {
                    let o = m.outline();
                    crate::pick::contains(&o, p)
                        && crate::pick::distance_to_outline(&o, p) > reach * 0.6
                });
                if !inside {
                    return Some(pick);
                }
            }
        }
        let id = self.mass_under(p)?;
        let m = model.mass(id)?;
        let body = archisim_geom::forms::mass_body(m);
        let (face, z) = sculpt::top_face_at(&body, p)?;
        Some(FacePick {
            mass: id,
            face,
            point: [p.x, p.y, z],
            normal: sculpt::face_normal(&body, face)?,
        })
    }

    // ---- Empujar on any face ------------------------------------------

    /// Whether Empujar on this face is the plain one (a contour's height,
    /// with the mouse up the screen).
    pub(super) fn push_is_height(&self, pick: &FacePick) -> bool {
        self.ed.model().mass(pick.mass).is_some_and(|m| {
            pick.face == archisim_geom::forms::TOP
                && !matches!(m.shape, MassShape::Mesh(_))
                && !matches!(
                    m.shape,
                    MassShape::Sweep {
                        profile: Some(_),
                        ..
                    }
                )
                && m.deform.is_none_or(|d| d.is_none())
        })
    }

    /// Takes a face to push.
    pub(super) fn start_face_push(&mut self, pick: FacePick, p: Point2) {
        self.concept.face_push = Some(FacePush {
            pick,
            from: p,
            d: 0.0,
        });
        self.concept.typed.clear();
        self.say("Mueve el ratón hacia fuera o hacia dentro de la cara (o escribe la distancia) y haz clic.");
    }

    /// How far the face goes with the cursor at `p` in plan: along the
    /// normal for a side; up the screen for a face looking up or down.
    pub(super) fn face_push_distance(&self, fp: &FacePush, p: Point2) -> f64 {
        let n = fp.pick.normal;
        let h = (n[0] * n[0] + n[1] * n[1]).sqrt();
        let d = if h > 0.5 {
            ((p.x - fp.from.x) * n[0] + (p.y - fp.from.y) * n[1]) / h
        } else {
            (p.y - fp.from.y) * n[2].signum()
        };
        round_cm(d)
    }

    /// In the 3D view: how far along the face's normal the ray of the
    /// cursor passes nearest.
    pub(super) fn face_push_from_ray(&mut self, origin: glam::Vec3, dir: glam::Vec3) {
        let Some(fp) = self.concept.face_push else {
            return;
        };
        let base = self.mass_base_z(fp.pick.mass);
        let o = glam::DVec3::new(fp.pick.point[0], fp.pick.point[1], fp.pick.point[2] + base);
        let n = glam::DVec3::from_array(fp.pick.normal);
        let (r0, rd) = (origin.as_dvec3(), dir.as_dvec3().normalize_or_zero());
        let w0 = o - r0;
        let b = n.dot(rd);
        let den = 1.0 - b * b;
        if den.abs() < 1e-6 {
            return;
        }
        let t = (b * rd.dot(w0) - n.dot(w0)) / den;
        if let Some(f) = &mut self.concept.face_push {
            f.d = round_cm(t);
        }
    }

    pub(super) fn finish_face_push(&mut self, d: f64) {
        let Some(fp) = self.concept.face_push.take() else {
            return;
        };
        self.concept.typed.clear();
        match self.ed.push_mass_face(fp.pick.mass, fp.pick.face, d) {
            Ok(()) => {
                self.ed.selection = [Element::Mass(fp.pick.mass)].into_iter().collect();
                self.say(format!(
                    "Cara empujada {} {}.",
                    crate::units::fmt_len(d.abs()),
                    if d < 0.0 {
                        "hacia dentro"
                    } else {
                        "hacia fuera"
                    }
                ));
            }
            Err(e) => self.complain(format!("No se puede empujar: {e}.")),
        }
    }

    // ---- Dividir --------------------------------------------------------

    pub(super) fn divide_click(&mut self, p: Point2) {
        match self.concept.divide {
            None => match self.face_under(p, false) {
                Some(pick) => {
                    self.concept.divide = Some(pick);
                    self.say("Clic en el otro punto de la línea, sobre la misma cara.");
                }
                None => self.complain("Haz clic en una cara de un volumen (mejor en 3D)."),
            },
            Some(a) => {
                self.concept.divide = None;
                let b = self.divide_second_point(&a, p);
                let along = [b[0] - a.point[0], b[1] - a.point[1], b[2] - a.point[2]];
                let normal = cross(along, a.normal);
                let len = dot(normal, normal).sqrt();
                if len < 1e-6 {
                    self.complain("Los dos puntos de la línea son el mismo.");
                    return;
                }
                let normal = normal.map(|v| v / len);
                match self.ed.divide_mass_face(a.mass, a.face, a.point, normal) {
                    Ok(_) => {
                        self.ed.selection = [Element::Mass(a.mass)].into_iter().collect();
                        self.say("Cara dividida: pinta o empuja cada parte por separado.");
                    }
                    Err(e) => self.complain(format!("No se puede dividir: {e}.")),
                }
            }
        }
    }

    /// The second point of Dividir: where the cursor is on the face (in
    /// 3D) or, in plan, at the first point's height.
    fn divide_second_point(&self, a: &FacePick, p: Point2) -> [f64; 3] {
        if self.mode == ViewMode::ThreeD {
            if let Some(c) = self.cursor3d {
                let z = c.world.z as f64 - self.mass_base_z(a.mass);
                return [c.world.x as f64, c.world.y as f64, z];
            }
        }
        [p.x, p.y, a.point[2]]
    }

    // ---- Deformar -------------------------------------------------------

    pub(super) fn deform_click(&mut self, p: Point2) {
        if let Some(dd) = self.concept.deform.take() {
            self.finish_deform(dd.mass, dd.now);
            return;
        }
        let Some(id) = self.mass_under(p) else {
            self.complain("Haz clic en un volumen.");
            return;
        };
        let Some(m) = self.ed.model().mass(id) else {
            return;
        };
        if m.building.is_some() {
            self.complain("Un volumen convertido en edificio no se deforma.");
            return;
        }
        let o = m.outline();
        let n = o.len().max(1) as f64;
        let centre = o.iter().fold(Point2::new(0.0, 0.0), |a, q| a + *q) * (1.0 / n);
        let before = m.deform.unwrap_or_default();
        self.concept.deform = Some(DeformDrag {
            mass: id,
            centre,
            from: p,
            before,
            now: before,
        });
        self.concept.typed.clear();
        self.say("Gira el ratón alrededor del volumen para retorcerlo; con Mayús, llévalo hacia un lado para doblarlo. Clic: dejarlo.");
    }

    /// Follows the cursor: the twist by the angle turned round the
    /// middle; with Mayús, the bend towards the cursor.
    pub(super) fn deform_follow(&mut self, p: Point2, bend: bool) {
        let height = self
            .concept
            .deform
            .and_then(|d| self.ed.model().mass(d.mass))
            .map_or(1.0, |m| m.height.max(0.1));
        let Some(dd) = &mut self.concept.deform else {
            return;
        };
        let mut now = dd.before;
        if bend {
            let v = p - dd.from;
            // A quarter of a turn for each height of the volume.
            now.bend =
                (v.length() / height * std::f64::consts::FRAC_PI_2).min(std::f64::consts::PI);
            now.bend = (now.bend.to_degrees() / 5.0).round() * 5.0;
            now.bend = now.bend.to_radians();
            if v.length() > 1e-6 {
                now.bend_direction = v.y.atan2(v.x);
            }
        } else {
            let a0 = (dd.from - dd.centre).y.atan2((dd.from - dd.centre).x);
            let a1 = (p - dd.centre).y.atan2((p - dd.centre).x);
            let mut t = a1 - a0;
            while t > std::f64::consts::PI {
                t -= std::f64::consts::TAU;
            }
            while t < -std::f64::consts::PI {
                t += std::f64::consts::TAU;
            }
            now.twist = dd.before.twist + (t.to_degrees() / 5.0).round().to_radians() * 5.0;
        }
        dd.now = now;
    }

    pub(super) fn finish_deform(&mut self, id: MassId, d: Deform) {
        self.concept.deform = None;
        self.concept.typed.clear();
        match self.ed.set_mass_deform(id, d) {
            Ok(()) => {
                self.ed.selection = [Element::Mass(id)].into_iter().collect();
                self.say(format!(
                    "Volumen deformado: torsión {}°, curvatura {}°.",
                    d.twist.to_degrees().round(),
                    d.bend.to_degrees().round()
                ));
            }
            Err(e) => self.complain(format!("No se puede deformar: {e}.")),
        }
    }

    /// Each frame: Deformar follows the cursor.
    pub(super) fn sculpt_follow(&mut self, ctx: &egui::Context) {
        if let (Some(_), Some(c)) = (self.concept.deform, self.cursor) {
            let bend = ctx.input(|i| i.modifiers.shift);
            self.deform_follow(c.point, bend);
        }
    }

    // ---- Redondear ------------------------------------------------------

    /// Rounds the edges of the volumes selected (or makes them sharp).
    pub(super) fn round_selected_edges(&mut self) {
        let ids: Vec<MassId> = self
            .ed
            .selection
            .iter()
            .filter_map(|e| match e {
                Element::Mass(id) => Some(*id),
                _ => None,
            })
            .collect();
        if ids.is_empty() {
            self.complain("Selecciona antes los volúmenes que quieres redondear.");
            return;
        }
        match self.ed.round_mass_edges(&ids) {
            Ok(true) => self.say("Esquinas y aristas de arriba redondeadas (15 cm). El tamaño y cuáles, en Propiedades → Aristas."),
            Ok(false) => self.say("Aristas vivas otra vez."),
            Err(e) => self.complain(format!("No se pueden redondear: {e}.")),
        }
    }

    // ---- A profile from the drawing ------------------------------------

    /// Starts taking a profile from the drawing: for the band `target`,
    /// or for the tool.
    pub(super) fn take_profile(&mut self, target: Option<MassId>) {
        self.concept_pick(super::concept_ui::ConceptPick::Follow);
        self.concept.pick_profile = Some(target);
        self.say("Clic en una polilínea cerrada del dibujo 2D o en una cara de Concepto: su forma será el perfil (de lado: el ancho; hacia arriba: la altura).");
    }

    /// The click that takes the profile.
    pub(super) fn profile_click(&mut self, p: Point2) {
        let Some(target) = self.concept.pick_profile.take() else {
            return;
        };
        let reach = self.view.to_metres(REACH);
        let model = self.ed.model();
        let outline = match crate::pick::pick(&self.ed, self.ed.level, p, reach) {
            Some(Element::Draft(id)) => match model.draft(id).map(|d| &d.shape) {
                Some(archisim_core::Shape::Polyline(pl)) if pl.closed => Some(pl.points.clone()),
                _ => None,
            },
            Some(Element::Mass(id)) => model
                .mass(id)
                .filter(|m| matches!(m.shape, MassShape::Prism(_)))
                .map(|m| m.outline()),
            _ => None,
        };
        let Some(profile) = outline.and_then(|o| crate::editor::profile_from_outline(&o)) else {
            self.complain("Ahí no hay una polilínea cerrada ni una cara: vuelve a probar con «Tomar del dibujo».");
            return;
        };
        match target {
            Some(id) => {
                let (w, h) = profile_size(&profile);
                let r = self.ed.edit_mass("Perfil", id, |x| {
                    if let MassShape::Sweep {
                        profile: p, width, ..
                    } = &mut x.shape
                    {
                        *p = Some(profile.clone());
                        *width = w;
                    }
                    x.height = h;
                });
                self.report(r, "Perfil cambiado por el dibujado.");
                self.ed.selection = [Element::Mass(id)].into_iter().collect();
                self.set_tool(Tool::Arrow);
            }
            None => {
                let (w, h) = profile_size(&profile);
                self.concept.drawn_profile = Some(profile);
                self.mass_settings.profile = SweepProfile::Drawn;
                self.mass_settings.width = w;
                self.mass_settings.height = h;
                self.say("Perfil tomado del dibujo. Dibuja el camino: clic en su comienzo.");
            }
        }
    }

    // ---- Previews -------------------------------------------------------

    /// The border of a face of a volume, moved `d` along its normal, in
    /// plan and floor height.
    fn moved_border(&self, pick: &FacePick, d: f64) -> Vec<[[f64; 3]; 2]> {
        let Some(m) = self.ed.model().mass(pick.mass) else {
            return Vec::new();
        };
        let base = self.mass_base_z(pick.mass);
        let body = archisim_geom::forms::mass_body(m);
        let off = pick.normal.map(|v| v * d);
        sculpt::face_border(&body, pick.face)
            .into_iter()
            .map(|e| e.map(|p| [p[0] + off[0], p[1] + off[1], p[2] + off[2] + base]))
            .collect()
    }

    /// What the new tools are doing, over the plan.
    pub(super) fn draw_sculpt_preview(&self, painter: &egui::Painter, rect: Rect) {
        let Some(c) = self.cursor else {
            return;
        };
        let ink = Stroke::new(2.0, ACCENT);
        if let Some(fp) = &self.concept.face_push {
            let d = crate::units::parse_length(&self.concept.typed)
                .unwrap_or_else(|| self.face_push_distance(fp, c.point));
            for [a, b] in self.moved_border(&fp.pick, d) {
                painter.line_segment(
                    [
                        self.view.to_screen(rect, Point2::new(a[0], a[1])),
                        self.view.to_screen(rect, Point2::new(b[0], b[1])),
                    ],
                    ink,
                );
            }
            self.draw_label(
                painter,
                rect,
                c.point,
                format!("Empujar {}", crate::units::fmt_len(d)),
            );
        }
        if let Some(a) = &self.concept.divide {
            let from = self
                .view
                .to_screen(rect, Point2::new(a.point[0], a.point[1]));
            painter.line_segment([from, self.view.to_screen(rect, c.point)], ink);
            painter.circle_filled(from, 3.5, ACCENT);
        }
        if let Some(dd) = &self.concept.deform {
            if let Some(m) = self.ed.model().mass(dd.mass) {
                // The top, turned and moved as the bend leaves it.
                let top = deformed_top(m, dd);
                painter.add(Shape::closed_line(self.polygon(rect, &top), ink));
                painter.add(Shape::closed_line(
                    self.polygon(rect, &m.outline()),
                    Stroke::new(1.0, ACCENT.gamma_multiply(0.6)),
                ));
                self.draw_label(painter, rect, c.point, deform_text(&dd.now));
            }
        }
        if self.concept.pick_profile.is_some() {
            self.draw_label(
                painter,
                rect,
                c.point,
                "Perfil: clic en una polilínea cerrada o una cara".into(),
            );
        }
    }

    /// The same over the 3D view.
    pub(super) fn draw_sculpt_3d(&self, painter: &egui::Painter, rect: Rect) {
        let size = glam::Vec2::new(rect.width(), rect.height());
        let to_screen = |p: [f64; 3]| {
            self.v3d
                .camera
                .project_to_screen(glam::Vec3::new(p[0] as f32, p[1] as f32, p[2] as f32), size)
                .map(|s| Pos2::new(rect.min.x + s.x, rect.min.y + s.y))
        };
        let ink = Stroke::new(2.0, ACCENT);
        if let Some(fp) = &self.concept.face_push {
            let d = crate::units::parse_length(&self.concept.typed).unwrap_or(fp.d);
            let border = self.moved_border(&fp.pick, d);
            for [a, b] in &border {
                if let (Some(a), Some(b)) = (to_screen(*a), to_screen(*b)) {
                    painter.line_segment([a, b], ink);
                }
            }
            // From where it was to where it goes.
            let back = self.moved_border(&fp.pick, 0.0);
            for ([a, _], [c, _]) in back.iter().zip(&border) {
                if let (Some(a), Some(c)) = (to_screen(*a), to_screen(*c)) {
                    painter.line_segment([a, c], Stroke::new(1.0, ACCENT));
                }
            }
            if let Some(p) = border.first().and_then(|e| to_screen(e[0])) {
                hud_text(painter, p, &format!("Empujar {}", crate::units::fmt_len(d)));
            }
        }
        if let (Some(a), Some(c)) = (&self.concept.divide, self.cursor3d) {
            let base = self.mass_base_z(a.mass);
            let from = [a.point[0], a.point[1], a.point[2] + base];
            let to = [c.world.x as f64, c.world.y as f64, c.world.z as f64];
            if let (Some(p), Some(q)) = (to_screen(from), to_screen(to)) {
                painter.line_segment([p, q], ink);
                painter.circle_filled(p, 3.5, ACCENT);
            }
        }
        if let Some(dd) = &self.concept.deform {
            if let Some(m) = self.ed.model().mass(dd.mass) {
                let base = self.mass_base_z(dd.mass);
                let mut probe = m.clone();
                probe.deform = Some(dd.now);
                let body = archisim_geom::forms::mass_body(&probe);
                let top = body.top();
                // Its highest ring, where it ends.
                let ring: Vec<Pos2> = body
                    .triangles
                    .iter()
                    .zip(&body.faces)
                    .filter(|(_, f)| **f == archisim_geom::forms::TOP)
                    .flat_map(|(t, _)| t.iter().map(|i| body.positions[*i as usize]))
                    .filter_map(|p| to_screen([p[0], p[1], p[2] + base]))
                    .collect();
                for p in &ring {
                    painter.circle_filled(*p, 1.5, ACCENT);
                }
                let _ = top;
                if let Some(c) = self.cursor3d {
                    if let Some(p) =
                        to_screen([c.world.x as f64, c.world.y as f64, c.world.z as f64])
                    {
                        hud_text(painter, p + egui::vec2(14.0, -14.0), &deform_text(&dd.now));
                    }
                }
            }
        }
    }

    // ---- Properties -----------------------------------------------------

    /// Twist and bend of a volume, and the profile of a band.
    pub(super) fn sculpt_groups(&mut self, ui: &mut Ui, m: &archisim_core::Mass) {
        let id = m.id;
        if let MassShape::Sweep { profile, .. } = &m.shape {
            let current = profile_label(profile.as_deref());
            group(ui, "mass-profile", "Perfil", |ui| {
                row(
                    ui,
                    "Perfil",
                    "Lo que lleva el camino: se estira al ancho y a la altura",
                    |ui, w| {
                        let mut pick: Option<Option<Vec<Point2>>> = None;
                        egui::ComboBox::from_id_salt(Id::unique(("mass-profile", id)))
                            .width(w - 8.0)
                            .selected_text(current.clone())
                            .show_ui(ui, |ui| {
                                if ui
                                    .selectable_label(profile.is_none(), "Rectangular")
                                    .clicked()
                                {
                                    pick = Some(None);
                                }
                                for (i, name) in sculpt::PRESETS.iter().enumerate() {
                                    let label = SweepProfile::Preset(i).label();
                                    if ui.selectable_label(current == label, label).clicked() {
                                        pick = Some(sculpt::preset_profile(name, 1.0, 1.0));
                                    }
                                }
                            });
                        if let Some(p) = pick {
                            let r = self.ed.set_sweep_profile(id, p);
                            self.report(r, "Perfil cambiado.");
                        }
                    },
                );
                if ui
                    .button("Tomar del dibujo…")
                    .on_hover_text("Una polilínea cerrada del dibujo 2D o una cara de Concepto, dibujada como el perfil: de lado el ancho, hacia arriba la altura")
                    .clicked()
                {
                    self.take_profile(Some(id));
                }
            });
        }
        if matches!(m.shape, MassShape::Mesh(_)) && m.deform.is_none() {
            // A solid can be deformed too: the same rows.
        }
        let d = m.deform.unwrap_or_default();
        let mut changed = None;
        group(ui, "mass-deform", "Deformar", |ui| {
            row(
                ui,
                "Torsión (°)",
                "Cuánto gira su parte de arriba alrededor de su centro",
                |ui, w| {
                    let mut v = d.twist.to_degrees();
                    if ui
                        .add_sized(
                            [w - 8.0, 20.0],
                            egui::DragValue::new(&mut v)
                                .range(-720.0..=720.0)
                                .speed(1.0)
                                .suffix("°"),
                        )
                        .changed()
                    {
                        changed = Some(Deform {
                            twist: v.to_radians(),
                            ..d
                        });
                    }
                },
            );
            row(
                ui,
                "Curvatura (°)",
                "Cuánto se dobla de abajo arriba",
                |ui, w| {
                    let mut v = d.bend.to_degrees();
                    if ui
                        .add_sized(
                            [w - 8.0, 20.0],
                            egui::DragValue::new(&mut v)
                                .range(-180.0..=180.0)
                                .speed(1.0)
                                .suffix("°"),
                        )
                        .changed()
                    {
                        changed = Some(Deform {
                            bend: v.to_radians(),
                            ..d
                        });
                    }
                },
            );
            row(
                ui,
                "Hacia (°)",
                "La dirección en planta hacia la que se dobla (0: este, 90: norte)",
                |ui, w| {
                    let mut v = d.bend_direction.to_degrees();
                    if ui
                        .add_sized(
                            [w - 8.0, 20.0],
                            egui::DragValue::new(&mut v)
                                .range(-360.0..=360.0)
                                .speed(1.0)
                                .suffix("°"),
                        )
                        .changed()
                    {
                        changed = Some(Deform {
                            bend_direction: v.to_radians(),
                            ..d
                        });
                    }
                },
            );
            if !d.is_none() && ui.button("Enderezar").clicked() {
                changed = Some(Deform::default());
            }
        });
        if let Some(nd) = changed {
            let r = self.ed.set_mass_deform(id, nd);
            self.report(r, "Deformación cambiada.");
        }
    }
}

/// The profile row of Seguir's tool.
pub(super) fn profile_tool_row(ui: &mut Ui, s: &mut crate::editor::MassSettings, has_drawn: bool) {
    row(
        ui,
        "Perfil",
        "Lo que lleva el camino, estirado al ancho y la altura",
        |ui, w| {
            egui::ComboBox::from_id_salt(Id::unique("tool-mass-profile"))
                .width(w - 8.0)
                .selected_text(s.profile.label())
                .show_ui(ui, |ui| {
                    ui.selectable_value(
                        &mut s.profile,
                        SweepProfile::Band,
                        SweepProfile::Band.label(),
                    );
                    for i in 0..sculpt::PRESETS.len() {
                        let p = SweepProfile::Preset(i);
                        ui.selectable_value(&mut s.profile, p, p.label());
                    }
                    if has_drawn {
                        ui.selectable_value(
                            &mut s.profile,
                            SweepProfile::Drawn,
                            SweepProfile::Drawn.label(),
                        );
                    }
                });
        },
    );
}

/// The width and height a drawn profile has.
fn profile_size(p: &[Point2]) -> (f64, f64) {
    let (lo, hi) = p.iter().fold(
        (
            Point2::new(f64::MAX, f64::MAX),
            Point2::new(f64::MIN, f64::MIN),
        ),
        |(lo, hi), q| {
            (
                Point2::new(lo.x.min(q.x), lo.y.min(q.y)),
                Point2::new(hi.x.max(q.x), hi.y.max(q.y)),
            )
        },
    );
    ((hi.x - lo.x).max(0.01), (hi.y - lo.y).max(0.01))
}

/// The name of a band's profile.
fn profile_label(p: Option<&[Point2]>) -> String {
    let Some(p) = p else {
        return "Rectangular".into();
    };
    for (i, name) in sculpt::PRESETS.iter().enumerate() {
        if sculpt::preset_profile(name, 1.0, 1.0).as_deref() == Some(p) {
            return SweepProfile::Preset(i).label();
        }
    }
    "Dibujado".into()
}

fn deform_text(d: &Deform) -> String {
    let mut t = format!("Torsión {}°", d.twist.to_degrees().round());
    if d.bend.abs() > 1e-9 {
        t.push_str(&format!(
            "   Curvatura {}° hacia {}°",
            d.bend.to_degrees().round(),
            d.bend_direction.to_degrees().round()
        ));
    }
    t
}

/// The top of a volume as a deformation would leave it, in plan.
fn deformed_top(m: &archisim_core::Mass, dd: &DeformDrag) -> Vec<Point2> {
    let o = m.outline();
    let h = m.height.max(0.01);
    let d = dd.now;
    let dir = Point2::new(d.bend_direction.cos(), d.bend_direction.sin());
    let perp = Point2::new(-dir.y, dir.x);
    o.iter()
        .map(|q| {
            let v = *q - dd.centre;
            let (s, c) = d.twist.sin_cos();
            let mut r = Point2::new(v.x * c - v.y * s, v.x * s + v.y * c);
            if d.bend.abs() > 1e-9 {
                let u = r.x * dir.x + r.y * dir.y;
                let w = r.x * perp.x + r.y * perp.y;
                let rad = h / d.bend;
                let nu = rad - (rad - u) * d.bend.cos();
                r = dir * nu + perp * w;
            }
            dd.centre + r
        })
        .collect()
}

/// A label on a dark pill, as the other hints over the 3D view.
fn hud_text(painter: &egui::Painter, at: Pos2, text: &str) {
    let font = egui::FontId::proportional(12.0);
    let galley = painter.layout_no_wrap(text.to_string(), font, Color32::WHITE);
    let r = Rect::from_min_size(at + egui::vec2(10.0, -22.0), galley.size()).expand(4.0);
    painter.rect_filled(r, 4.0, Color32::from_rgba_unmultiplied(20, 24, 32, 220));
    painter.galley(r.min + egui::vec2(4.0, 4.0), galley, Color32::WHITE);
}
