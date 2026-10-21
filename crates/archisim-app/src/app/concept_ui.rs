//! The Concepto discipline (I4): volumes drawn freely, as in SketchUp.
//!
//! Arista draws lines of the model: left open they stay as loose edges, and
//! closed (on their first point, or meeting other edges) they make a flat
//! face; its rectangle and circle methods make a face at once. Empujar gives a face or
//! volume its height (the mouse up the screen, or a typed value); Volumen
//! makes a prism of the height set; Masa, one by floors, with its built
//! area, plot ratio and coverage as it grows; Seguir, a band along a
//! path; Equidistancia, a parallel contour on top of a volume; Cinta
//! measures and leaves a guide; Pintar gives a volume a material; and
//! Convertir turns volumes into walls, slabs and a roof, remembering them.

use archisim_core::{MassId, MassShape, MaterialId, Point2};
use eframe::egui::{self, ComboBox, Id, Ui};

use super::props::{self, choice, group, row, text_row};
use super::*;
use crate::editor::{MassMode, MassWhat};

/// What a tile of the Concepto discipline takes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConceptPick {
    /// Arista: lines of the model that close into faces.
    Line,
    Push,
    Follow,
    Offset,
    Volume,
    Mass,
    Tape,
    Paint,
}

/// What the concept tools are doing.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ConceptState {
    /// The centre of the circle being drawn.
    pub centre: Option<Point2>,
    /// The points of the path being drawn.
    pub path: Vec<Point2>,
    /// The mass being pushed, the point where it was taken and its height
    /// then.
    pub push: Option<(MassId, Point2, f64)>,
    /// The volume a parallel contour is being drawn of.
    pub offset: Option<MassId>,
    /// The first point of the tape measure.
    pub tape: Option<Point2>,
    /// A value typed (height, distance).
    pub typed: String,
}

impl ConceptState {
    pub fn finish(&mut self) {
        *self = ConceptState::default();
    }

    pub fn is_drawing(&self) -> bool {
        self.centre.is_some()
            || !self.path.is_empty()
            || self.push.is_some()
            || self.offset.is_some()
            || self.tape.is_some()
    }
}

/// A combo of the project's materials.
fn material_combo(
    ui: &mut Ui,
    id: Id,
    model: &archisim_core::Model,
    value: MaterialId,
    width: f32,
) -> MaterialId {
    let mut v = value;
    let mut mats: Vec<(MaterialId, String)> =
        model.materials().map(|m| (m.id, m.name.clone())).collect();
    mats.sort_by_key(|a| a.1.to_lowercase());
    let shown = model
        .material(value)
        .map_or("—".to_string(), |m| m.name.clone());
    ComboBox::from_id_salt(id)
        .width(width - 8.0)
        .truncate()
        .selected_text(shown)
        .show_ui(ui, |ui| {
            for (mid, name) in &mats {
                ui.selectable_value(&mut v, *mid, name);
            }
        });
    v
}

fn comma(v: f64, d: usize) -> String {
    format!("{v:.d$}").replace('.', ",")
}

impl ArchiSimApp {
    /// Takes the tool of a tile of Concepto.
    pub(super) fn concept_pick(&mut self, p: ConceptPick) {
        let s = &mut self.mass_settings;
        let tool = match p {
            ConceptPick::Line => {
                (s.what, s.mode) = (MassWhat::Face, MassMode::Polygon);
                Tool::Mass
            }
            ConceptPick::Volume => {
                s.what = MassWhat::Volume;
                if s.mode == MassMode::Path {
                    s.mode = MassMode::Polygon;
                }
                Tool::Mass
            }
            ConceptPick::Mass => {
                s.what = MassWhat::Mass;
                if s.mode == MassMode::Path {
                    s.mode = MassMode::Polygon;
                }
                Tool::Mass
            }
            ConceptPick::Follow => {
                (s.what, s.mode) = (MassWhat::Volume, MassMode::Path);
                Tool::Mass
            }
            ConceptPick::Push => Tool::PushPull,
            ConceptPick::Offset => Tool::MassOffset,
            ConceptPick::Tape => Tool::Tape,
            ConceptPick::Paint => Tool::Paint,
        };
        self.mass_tool.set_mode(match self.mass_settings.mode {
            MassMode::Rectangle => SlabMode::Rectangle,
            _ => SlabMode::Polygon,
        });
        self.concept.finish();
        self.set_tool(tool);
    }

    /// Whether a tile of Concepto is the tool in use.
    pub(super) fn concept_picked(&self, p: ConceptPick) -> bool {
        let s = &self.mass_settings;
        match p {
            ConceptPick::Line => {
                self.tool == Tool::Mass && s.what == MassWhat::Face && s.mode != MassMode::Path
            }
            ConceptPick::Follow => self.tool == Tool::Mass && s.mode == MassMode::Path,
            ConceptPick::Volume => {
                self.tool == Tool::Mass && s.what == MassWhat::Volume && s.mode != MassMode::Path
            }
            ConceptPick::Mass => self.tool == Tool::Mass && s.what == MassWhat::Mass,
            ConceptPick::Push => self.tool == Tool::PushPull,
            ConceptPick::Offset => self.tool == Tool::MassOffset,
            ConceptPick::Tape => self.tool == Tool::Tape,
            ConceptPick::Paint => self.tool == Tool::Paint,
        }
    }

    /// Changes the contour of the volume tool (from the contextual bar).
    pub(super) fn set_mass_mode(&mut self, mode: MassMode) {
        if self.mass_settings.mode != mode {
            self.mass_settings.mode = mode;
            self.mass_tool.set_mode(if mode == MassMode::Rectangle {
                SlabMode::Rectangle
            } else {
                SlabMode::Polygon
            });
            self.concept.finish();
        }
    }

    /// The mass under `p` on the active level.
    fn mass_under(&self, p: Point2) -> Option<MassId> {
        let reach = self.view.to_metres(REACH);
        self.ed
            .model()
            .masses_on_level(self.ed.level)
            .filter(|m| {
                let o = m.outline();
                crate::pick::contains(&o, p) || crate::pick::distance_to_outline(&o, p) <= reach
            })
            .min_by(|a, b| {
                let area = |m: &archisim_core::Mass| archisim_core::signed_area(&m.outline()).abs();
                area(a).total_cmp(&area(b))
            })
            .map(|m| m.id)
    }

    /// Whether the volume tool is drawing edges (Arista: lines that stay
    /// open, or close into a face).
    pub(super) fn drawing_edges(&self) -> bool {
        self.tool == Tool::Mass
            && self.mass_settings.what == MassWhat::Face
            && self.mass_settings.mode == MassMode::Polygon
    }

    /// Whether the volume tool takes its points as an open line (a path
    /// or edges), not as a contour.
    pub(super) fn mass_line(&self) -> bool {
        self.tool == Tool::Mass
            && (self.mass_settings.mode == MassMode::Path || self.drawing_edges())
    }

    /// The anchor of what a concept tool is drawing (for tracking).
    pub(super) fn concept_anchor(&self) -> Option<Point2> {
        if self.mass_line() {
            return self.concept.path.last().copied();
        }
        match self.tool {
            Tool::Mass => match self.mass_settings.mode {
                MassMode::Circle => self.concept.centre,
                MassMode::Path => self.concept.path.last().copied(),
                _ => self.mass_tool.anchor(),
            },
            Tool::Tape => self.concept.tape,
            _ => None,
        }
    }

    /// The points of what a concept tool is drawing (for the guides).
    pub(super) fn concept_points(&self) -> Vec<Point2> {
        if self.mass_line() {
            return self.concept.path.clone();
        }
        match (self.tool, self.mass_settings.mode) {
            (Tool::Mass, MassMode::Path) => self.concept.path.clone(),
            (Tool::Mass, MassMode::Circle) => self.concept.centre.into_iter().collect(),
            (Tool::Mass, _) => self.mass_tool.corners().to_vec(),
            (Tool::Tape, _) => self.concept.tape.into_iter().collect(),
            _ => Vec::new(),
        }
    }

    // ---- Clicks -------------------------------------------------------------

    pub(super) fn concept_click(&mut self, p: Point2) {
        match self.tool {
            Tool::Mass => self.mass_click(p),
            Tool::PushPull => match self.concept.push {
                None => match self.mass_under(p) {
                    Some(id) => {
                        let h = self.ed.model().mass(id).map_or(0.0, |m| m.height);
                        self.concept.push = Some((id, p, h));
                        self.concept.typed.clear();
                        self.say("Mueve el ratón hacia arriba para subir (o escribe la altura) y haz clic.");
                    }
                    None => self.complain("Haz clic en una cara o un volumen para empujarlo."),
                },
                Some((id, from, h0)) => {
                    let h = h0 + (p.y - from.y);
                    self.finish_push(id, h);
                }
            },
            Tool::MassOffset => match self.concept.offset {
                None => match self.mass_under(p) {
                    Some(id)
                        if matches!(
                            self.ed.model().mass(id).map(|m| &m.shape),
                            Some(MassShape::Prism(_))
                        ) =>
                    {
                        self.concept.offset = Some(id);
                        self.concept.typed.clear();
                        self.say(
                            "Mueve el ratón dentro o fuera (o escribe la distancia) y haz clic.",
                        );
                    }
                    Some(_) => self.complain("La equidistancia se hace de volúmenes por contorno."),
                    None => self.complain("Haz clic en un volumen."),
                },
                Some(id) => {
                    let d = self.offset_distance(id, p);
                    self.finish_offset(id, d);
                }
            },
            Tool::Tape => match self.concept.tape {
                None => self.concept.tape = Some(p),
                Some(a) if a.approx_eq(p) => {}
                Some(a) => {
                    self.concept.tape = None;
                    let r = self.ed.add_guide(a, p);
                    match r {
                        Ok(()) => self.say(format!(
                            "Distancia {} mm. Guía en la capa «Guías» (no se imprime).",
                            fmt_mm(a.distance(p))
                        )),
                        Err(e) => self.complain(format!("No se puede dejar la guía: {e}.")),
                    }
                }
            },
            Tool::Paint => match self.mass_under(p) {
                Some(id) => {
                    let mat = self.mass_settings.material;
                    let r = self.ed.paint_mass(id, mat);
                    self.report(r, "Volumen pintado.");
                }
                None => self.complain("Haz clic en un volumen para pintarlo."),
            },
            _ => {}
        }
    }

    fn mass_click(&mut self, p: Point2) {
        if self.drawing_edges() {
            let reach = self.view.to_metres(REACH);
            let path = &self.concept.path;
            if path.len() >= 3 && path[0].distance(p) <= reach {
                // Back on the first point: a face.
                let outline = std::mem::take(&mut self.concept.path);
                self.build_mass(outline);
            } else if path.last().is_some_and(|l| l.approx_eq(p)) {
                self.finish_path();
            } else {
                self.concept.path.push(p);
            }
            return;
        }
        match self.mass_settings.mode {
            MassMode::Circle => match self.concept.centre {
                None => self.concept.centre = Some(p),
                Some(c) if c.approx_eq(p) => {}
                Some(c) => {
                    self.concept.centre = None;
                    let outline = crate::editor::circle(c, c.distance(p));
                    self.build_mass(outline);
                }
            },
            MassMode::Path => {
                if self.concept.path.last().is_some_and(|l| l.approx_eq(p)) {
                    self.finish_path();
                } else {
                    self.concept.path.push(p);
                }
            }
            MassMode::Polygon | MassMode::Rectangle => match self.mass_tool.click(p) {
                SlabOutcome::Build(outline) => self.build_mass(outline),
                SlabOutcome::Invalid => {
                    self.complain("El contorno no puede cruzarse: corrige la última esquina.")
                }
                SlabOutcome::Nothing => {}
            },
        }
    }

    pub(super) fn build_mass(&mut self, outline: Vec<Point2>) {
        let s = self.mass_settings;
        match self.ed.add_mass_outline(&s, outline) {
            Ok(id) => {
                self.mass_tool.accept();
                self.ed.selection = [Element::Mass(id)].into_iter().collect();
                self.say_mass(id, "creado");
            }
            Err(e) => self.complain(format!("No se puede crear el volumen: {e}.")),
        }
    }

    /// Ends the path of «Seguir» and makes its band; with Arista, the
    /// edges drawn (or the face they close with the edges there).
    pub(super) fn finish_path(&mut self) {
        let path = std::mem::take(&mut self.concept.path);
        if path.len() < 2 {
            return;
        }
        if self.drawing_edges() {
            self.finish_edges(path);
            return;
        }
        let s = self.mass_settings;
        match self.ed.add_mass_path(&s, path) {
            Ok(id) => {
                self.ed.selection = [Element::Mass(id)].into_iter().collect();
                self.say_mass(id, "creado");
            }
            Err(e) => self.complain(format!("No se puede crear el volumen: {e}.")),
        }
    }

    /// Loose edges drawn: if, with the edges already on the level, they
    /// close a contour, that contour becomes a face (and the edges that
    /// made it go); else they stay as an edge.
    pub(super) fn finish_edges(&mut self, path: Vec<Point2>) {
        let reach = self.view.to_metres(REACH).max(1e-6);
        let level = self.ed.level;
        let edges: Vec<(archisim_core::EdgeId, Vec<Point2>)> = self
            .ed
            .model()
            .edges_on_level(level)
            .map(|e| (e.id, e.points.clone()))
            .collect();
        if let Some((outline, used)) = close_with_edges(&path, &edges, reach) {
            let s = self.mass_settings;
            let r = self.ed.transact("Crear cara", |m| {
                for id in &used {
                    m.remove_edge(*id)?;
                }
                crate::editor::add_mass_in(m, &s, level, outline)
            });
            match r {
                Ok(id) => {
                    self.ed.selection = [Element::Mass(id)].into_iter().collect();
                    self.say_mass(id, "creada con las aristas");
                }
                Err(e) => self.complain(format!("No se puede crear la cara: {e}.")),
            }
            return;
        }
        let r = self
            .ed
            .transact("Crear arista", |m| m.add_edge(level, path, 0.0));
        match r {
            Ok(id) => {
                self.ed.selection = [Element::Edge(id)].into_iter().collect();
                self.say("Arista del modelo. Cierra un contorno (con esta y otras aristas) y se hace una cara.");
            }
            Err(e) => self.complain(format!("No se puede crear la arista: {e}.")),
        }
    }

    fn finish_push(&mut self, id: MassId, h: f64) {
        self.concept.push = None;
        self.concept.typed.clear();
        match self.ed.push_mass(id, h) {
            Ok(()) => {
                self.ed.selection = [Element::Mass(id)].into_iter().collect();
                self.say_mass(id, "empujado");
            }
            Err(e) => self.complain(format!("No se puede empujar: {e}.")),
        }
    }

    fn finish_offset(&mut self, id: MassId, d: f64) {
        self.concept.offset = None;
        self.concept.typed.clear();
        match self.ed.offset_mass(id, d) {
            Ok(new) => {
                self.ed.selection = [Element::Mass(new)].into_iter().collect();
                self.say(format!(
                    "Cara a {} mm {}, encima del volumen. Empújala para seguir.",
                    fmt_mm(d.abs()),
                    if d < 0.0 {
                        "hacia dentro"
                    } else {
                        "hacia fuera"
                    }
                ));
            }
            Err(e) => self.complain(format!("No se puede: {e}.")),
        }
    }

    /// The distance of a parallel contour: from the cursor to the volume's
    /// contour, outwards positive.
    fn offset_distance(&self, id: MassId, p: Point2) -> f64 {
        let Some(m) = self.ed.model().mass(id) else {
            return 0.0;
        };
        let o = m.outline();
        let d = crate::pick::distance_to_outline(&o, p);
        if crate::pick::contains(&o, p) {
            -d
        } else {
            d
        }
    }

    fn say_mass(&mut self, id: MassId, what: &str) {
        let model = self.ed.model();
        let Some(m) = model.mass(id) else {
            return;
        };
        let text = format!(
            "{} {what}. {}",
            archisim_plan::concept::mass_label(model, m),
            if m.is_face() {
                "Empujar le da altura."
            } else {
                "Convertir lo pasa a muros, forjados y cubierta."
            }
        );
        self.say(text);
    }

    // ---- Keys -------------------------------------------------------------

    /// Typing a value while pushing or offsetting (Enter applies it).
    /// Returns whether the key was used.
    pub(super) fn concept_type(&mut self, c: char) -> bool {
        if (self.concept.push.is_some() || self.concept.offset.is_some())
            && (c.is_ascii_digit() || matches!(c, ',' | '.' | '-' | 'm' | 'c'))
        {
            self.concept.typed.push(c);
            return true;
        }
        false
    }

    pub(super) fn concept_backspace(&mut self) -> bool {
        self.concept.typed.pop().is_some()
    }

    /// Enter: applies the typed value, or ends the path.
    pub(super) fn concept_enter(&mut self) {
        let v = crate::units::parse_length(&self.concept.typed);
        if let (Some((id, _, _)), Some(h)) = (self.concept.push, v) {
            self.finish_push(id, h);
        } else if let (Some(id), Some(d)) = (self.concept.offset, v) {
            self.finish_offset(id, d);
        } else if self.mass_line() {
            self.finish_path();
        } else if self.tool == Tool::Mass {
            let towards = self.cursor.map_or(Point2::new(0.0, 0.0), |c| c.point);
            match self.mass_tool.enter(towards) {
                SlabOutcome::Build(outline) => self.build_mass(outline),
                SlabOutcome::Invalid => self
                    .complain("El contorno necesita al menos tres esquinas y no puede cruzarse."),
                SlabOutcome::Nothing => {}
            }
        }
    }

    /// Esc: drops what is being drawn; with nothing, back to the arrow.
    pub(super) fn concept_escape(&mut self) {
        if self.tool == Tool::Mass && self.mass_tool.escape() {
            return;
        }
        if self.concept.is_drawing() {
            if self.mass_line() && self.concept.path.len() >= 2 {
                self.finish_path();
            }
            self.concept.finish();
        } else {
            self.set_tool(Tool::Arrow);
        }
    }

    // ---- Preview ----------------------------------------------------------

    pub(super) fn draw_concept_preview(&self, painter: &egui::Painter, rect: Rect) {
        let Some(c) = self.cursor else {
            return;
        };
        let ink = Stroke::new(1.5, ACCENT);
        let model = self.ed.model();
        if self.drawing_edges() {
            if !self.concept.path.is_empty() {
                let mut pts = self.concept.path.clone();
                pts.push(c.point);
                painter.add(Shape::line(self.polygon(rect, &pts), ink));
                let first = self.concept.path[0];
                if pts.len() >= 4 && first.distance(c.point) <= self.view.to_metres(REACH) {
                    let s = self.view.to_screen(rect, first);
                    painter.circle_stroke(s, 6.0, Stroke::new(1.5, ACCENT));
                    self.draw_label(painter, rect, c.point, "Cerrar: cara".to_string());
                } else {
                    let len: f64 = pts.windows(2).map(|w| w[0].distance(w[1])).sum();
                    self.draw_label(painter, rect, c.point, format!("L {} mm", fmt_mm(len)));
                }
            }
            return;
        }
        match self.tool {
            Tool::Mass => match self.mass_settings.mode {
                MassMode::Circle => {
                    if let Some(centre) = self.concept.centre {
                        let r = centre.distance(c.point);
                        let pts = self.polygon(rect, &crate::editor::circle(centre, r));
                        painter.add(Shape::closed_line(pts, ink));
                        self.draw_label(painter, rect, c.point, format!("R {} mm", fmt_mm(r)));
                    }
                }
                MassMode::Path => {
                    if !self.concept.path.is_empty() {
                        let mut pts = self.concept.path.clone();
                        pts.push(c.point);
                        painter.add(Shape::line(self.polygon(rect, &pts), ink));
                        let probe = archisim_core::Mass {
                            id: MassId(0),
                            guid: Default::default(),
                            name: String::new(),
                            level: self.ed.level,
                            shape: MassShape::Sweep {
                                path: pts,
                                width: self.mass_settings.width,
                            },
                            base_offset: 0.0,
                            height: 1.0,
                            floor_height: None,
                            material: self.mass_settings.material,
                            building: None,
                        };
                        let band = probe.outline();
                        if band.len() > 2 {
                            painter.add(Shape::closed_line(
                                self.polygon(rect, &band),
                                Stroke::new(1.0, ACCENT.gamma_multiply(0.6)),
                            ));
                        }
                    }
                }
                _ => {
                    let corners = self.mass_tool.corners();
                    if corners.is_empty() {
                        return;
                    }
                    if self.mass_tool.mode == SlabMode::Rectangle {
                        if let Some(outline) = self.mass_tool.rectangle_to(c.point) {
                            painter.add(Shape::closed_line(self.polygon(rect, &outline), ink));
                            self.draw_label(
                                painter,
                                rect,
                                c.point,
                                format!(
                                    "S {}",
                                    fmt_area(archisim_core::signed_area(&outline).abs())
                                ),
                            );
                        }
                        return;
                    }
                    let mut pts = corners.to_vec();
                    pts.push(self.mass_tool.typed_end(c.point).unwrap_or(c.point));
                    let screen = self.polygon(rect, &pts);
                    painter.add(Shape::line(screen.clone(), ink));
                    if pts.len() >= 3 {
                        dashed(painter, screen[screen.len() - 1], screen[0], ACCENT);
                    }
                }
            },
            Tool::PushPull => {
                if let Some((id, from, h0)) = self.concept.push {
                    let typed = crate::units::parse_length(&self.concept.typed);
                    let h = crate::editor::snap_height(
                        model,
                        id,
                        typed.unwrap_or(h0 + (c.point.y - from.y)),
                    );
                    if let Some(m) = model.mass(id) {
                        painter.add(Shape::closed_line(self.polygon(rect, &m.outline()), ink));
                        let mut probe = m.clone();
                        probe.height = h;
                        let f = probe.figures(model.plot_area());
                        let mut text = format!("H {} m", comma(h, 2));
                        if m.floor_height.is_some() {
                            text.push_str(&format!(
                                "   {} plantas   {} m²",
                                f.floors,
                                comma(f.built, 0)
                            ));
                            if let Some(r) = f.plot_ratio {
                                text.push_str(&format!("   edif. {} m²/m²", comma(r, 2)));
                            }
                        }
                        self.draw_label(painter, rect, c.point, text);
                    }
                }
            }
            Tool::MassOffset => {
                if let Some(id) = self.concept.offset {
                    let d = crate::units::parse_length(&self.concept.typed)
                        .unwrap_or_else(|| self.offset_distance(id, c.point));
                    if let Some(o) = model
                        .mass(id)
                        .and_then(|m| archisim_core::offset_contour(&m.outline(), d))
                    {
                        painter.add(Shape::closed_line(self.polygon(rect, &o), ink));
                    }
                    self.draw_label(painter, rect, c.point, format!("{} mm", fmt_mm(d)));
                }
            }
            Tool::Tape => {
                if let Some(a) = self.concept.tape {
                    dashed(
                        painter,
                        self.view.to_screen(rect, a),
                        self.view.to_screen(rect, c.point),
                        ACCENT,
                    );
                    self.draw_label(
                        painter,
                        rect,
                        c.point,
                        format!("{} mm", fmt_mm(a.distance(c.point))),
                    );
                }
            }
            _ => {}
        }
    }

    pub(super) fn concept_hint(&self) -> &'static str {
        if self.drawing_edges() {
            return if self.concept.path.is_empty() {
                "Arista: clic en el primer punto (líneas del modelo, no del dibujo) · Esc: flecha"
            } else {
                "Clic: siguiente punto · Clic en el primero: cara · Doble clic, Intro o Esc: dejar la arista"
            };
        }
        match self.tool {
            Tool::Mass => match self.mass_settings.mode {
                MassMode::Circle if self.concept.centre.is_some() => "Clic: radio · Esc: anular",
                MassMode::Circle => "Clic: centro del círculo · Esc: flecha",
                MassMode::Path if !self.concept.path.is_empty() => {
                    "Clic: siguiente punto · Doble clic, Intro o Esc: terminar el camino"
                }
                MassMode::Path => "Clic: comienzo del camino (el perfil, ancho × alto, en Propiedades) · Esc: flecha",
                MassMode::Rectangle if self.mass_tool.is_drawing() => "Clic: esquina opuesta · Esc: anular",
                MassMode::Rectangle => "Clic: primera esquina · Esc: flecha",
                _ if self.mass_tool.is_drawing() => {
                    "Clic: siguiente esquina · Clic en la primera o Intro: cerrar · Esc: quitar la última"
                }
                _ => "Clic: primera esquina del contorno · Esc: flecha",
            },
            Tool::PushPull if self.concept.push.is_some() => {
                "Ratón arriba: más alto · Escribe la altura + Intro · Clic: dejarlo · Esc: anular"
            }
            Tool::PushPull => "Clic en una cara o un volumen para empujarlo · Esc: flecha",
            Tool::MassOffset if self.concept.offset.is_some() => {
                "Ratón dentro o fuera · Escribe la distancia + Intro · Clic: dejarlo · Esc: anular"
            }
            Tool::MassOffset => "Clic en un volumen: un contorno paralelo encima · Esc: flecha",
            Tool::Tape if self.concept.tape.is_some() => "Clic: segundo punto (mide y deja una guía) · Esc: anular",
            Tool::Tape => "Clic: primer punto · Esc: flecha",
            Tool::Paint => "Clic en un volumen: le da el material elegido en Propiedades · Esc: flecha",
            _ => "",
        }
    }

    // ---- Properties -------------------------------------------------------

    pub(super) fn mass_tool_properties(&mut self, ui: &mut Ui) {
        let what = match (self.tool, self.mass_settings.what, self.mass_settings.mode) {
            (Tool::PushPull, ..) => "Empujar",
            (Tool::MassOffset, ..) => "Equidistancia",
            (Tool::Tape, ..) => "Cinta",
            (Tool::Paint, ..) => "Pintar",
            (_, _, MassMode::Path) => "Seguir",
            (_, MassWhat::Face, MassMode::Polygon) => "Arista",
            (_, MassWhat::Face, _) => "Cara",
            (_, MassWhat::Volume, _) => "Volumen",
            (_, MassWhat::Mass, _) => "Masa",
        };
        props::title(ui, &format!("Herramienta {what} · lo que vas a hacer"));
        let model = self.ed.model();
        let mut s = self.mass_settings;
        let mut plot = model.plot_area();
        let mut plot_changed = false;
        match self.tool {
            Tool::Mass => {
                group(ui, "tool-mass", "Volumen", |ui| {
                    if s.mode != MassMode::Path {
                        row(ui, "Hace", "Una cara para empujar, un volumen de una altura o una masa por plantas", |ui, w| {
                            s.what = choice(
                                ui,
                                "tool-mass-what",
                                w,
                                s.what,
                                &[
                                    (MassWhat::Face, "Arista o cara"),
                                    (MassWhat::Volume, "Volumen"),
                                    (MassWhat::Mass, "Masa por plantas"),
                                ],
                            );
                        });
                    }
                    match (s.what, s.mode) {
                        (MassWhat::Mass, _) => {
                            row(ui, "Plantas", "", |ui, w| {
                                let mut n = s.floors as u32;
                                ui.add_sized(
                                    [w - 8.0, 20.0],
                                    egui::DragValue::new(&mut n).range(1..=60),
                                );
                                s.floors = n as usize;
                            });
                            row(ui, "Altura de planta (mm)", "", |ui, w| {
                                if let Some(v) = length_field(
                                    ui,
                                    Id::unique("tool-mass-fh"),
                                    s.floor_height,
                                    w - 8.0,
                                ) {
                                    if v > 0.0 {
                                        s.floor_height = v;
                                    }
                                }
                            });
                        }
                        (MassWhat::Volume, _) | (_, MassMode::Path) => {
                            row(ui, "Altura (mm)", "", |ui, w| {
                                if let Some(v) =
                                    length_field(ui, Id::unique("tool-mass-h"), s.height, w - 8.0)
                                {
                                    if v > 0.0 {
                                        s.height = v;
                                    }
                                }
                            });
                        }
                        _ => {}
                    }
                    if s.mode == MassMode::Path {
                        row(
                            ui,
                            "Ancho (mm)",
                            "El perfil que sigue al camino",
                            |ui, w| {
                                if let Some(v) =
                                    length_field(ui, Id::unique("tool-mass-w"), s.width, w - 8.0)
                                {
                                    if v > 0.0 {
                                        s.width = v;
                                    }
                                }
                            },
                        );
                    }
                    row(ui, "Base (mm)", "Sobre la planta", |ui, w| {
                        if let Some(v) =
                            length_field(ui, Id::unique("tool-mass-base"), s.base_offset, w - 8.0)
                        {
                            s.base_offset = v;
                        }
                    });
                    row(ui, "Material", "", |ui, w| {
                        s.material =
                            material_combo(ui, Id::unique("tool-mass-mat"), model, s.material, w);
                    });
                });
            }
            Tool::Paint => {
                group(ui, "tool-paint", "Pintar", |ui| {
                    row(
                        ui,
                        "Material",
                        "El que se da al volumen pinchado",
                        |ui, w| {
                            s.material = material_combo(
                                ui,
                                Id::unique("tool-paint-mat"),
                                model,
                                s.material,
                                w,
                            );
                        },
                    );
                });
            }
            _ => {
                props::note(ui, self.concept_hint());
            }
        }
        group(ui, "tool-mass-plot", "Parcela", |ui| {
            row(
                ui,
                "Superficie (m²)",
                "Para la edificabilidad y la ocupación de las masas",
                |ui, w| {
                    let shown = plot.map(|a| comma(a, 2)).unwrap_or_default();
                    if let Some(t) =
                        text_edit_committed(ui, Id::unique("plot-area"), &shown, w - 8.0)
                    {
                        let t = t.trim().replace(',', ".");
                        plot = if t.is_empty() { None } else { t.parse().ok() };
                        plot_changed = true;
                    }
                },
            );
        });
        self.mass_settings = s;
        if plot_changed {
            let r = self.ed.set_plot_area(plot);
            self.report(r, "Superficie de la parcela cambiada.");
        }
    }

    pub(super) fn edge_properties(&mut self, ui: &mut Ui, id: archisim_core::EdgeId) {
        let Some(e) = self.ed.model().edge(id).cloned() else {
            return;
        };
        props::title(ui, "Arista · 1 seleccionada");
        group(ui, "edge-data", "Arista del modelo", |ui| {
            self.level_row(
                ui,
                ("edge-level", id),
                "Planta",
                Some(e.level),
                &[Element::Edge(id)],
            );
            row(ui, "Altura (mm)", "Sobre la planta", |ui, w| {
                if let Some(v) = length_field(ui, Id::unique(("edge-z", id)), e.elevation, w - 8.0)
                {
                    let r = self.ed.transact("Altura de la arista", |m| {
                        m.edit_edge(id, |x| x.elevation = v)
                    });
                    self.report(r, "Altura cambiada.");
                }
            });
            let len: f64 = e.points.windows(2).map(|w| w[0].distance(w[1])).sum();
            text_row(ui, "Longitud (m)", "", &comma(len, 2));
            text_row(ui, "Tramos", "", &(e.points.len() - 1).to_string());
            props::note(
                ui,
                "Una línea del modelo, no del dibujo: con Arista, cierra un contorno con ella y se hace una cara.",
            );
        });
    }

    pub(super) fn mass_properties(&mut self, ui: &mut Ui, id: MassId) {
        let model = self.ed.model();
        let Some(m) = model.mass(id).cloned() else {
            return;
        };
        let f = m.figures(model.plot_area());
        props::title(ui, &format!("{} · 1 seleccionado", m.name));
        group(ui, "mass-data", "Volumen", |ui| {
            row(ui, "Nombre", "", |ui, w| {
                if let Some(t) =
                    text_edit_committed(ui, Id::unique(("mass-name", id)), &m.name, w - 8.0)
                {
                    let r = self
                        .ed
                        .edit_mass("Nombre del volumen", id, |x| x.name = t.trim().to_string());
                    self.report(r, "Nombre cambiado.");
                }
            });
            self.level_row(
                ui,
                ("mass-level", id),
                "Planta",
                Some(m.level),
                &[Element::Mass(id)],
            );
            row(ui, "Base (mm)", "Sobre la planta", |ui, w| {
                if let Some(v) =
                    length_field(ui, Id::unique(("mass-base", id)), m.base_offset, w - 8.0)
                {
                    let r = self
                        .ed
                        .edit_mass("Base del volumen", id, |x| x.base_offset = v);
                    self.report(r, "Base cambiada.");
                }
            });
            match m.floor_height {
                Some(fh) => {
                    row(ui, "Plantas", "", |ui, w| {
                        let mut n = m.floors() as u32;
                        if ui
                            .add_sized([w - 8.0, 20.0], egui::DragValue::new(&mut n).range(1..=60))
                            .changed()
                        {
                            let r = self
                                .ed
                                .edit_mass("Plantas de la masa", id, |x| x.height = n as f64 * fh);
                            self.report(r, "Plantas cambiadas.");
                        }
                    });
                    row(ui, "Altura de planta (mm)", "", |ui, w| {
                        if let Some(v) = length_field(ui, Id::unique(("mass-fh", id)), fh, w - 8.0)
                        {
                            if v > 0.0 {
                                let floors = m.floors() as f64;
                                let r = self.ed.edit_mass("Altura de planta", id, |x| {
                                    x.floor_height = Some(v);
                                    x.height = floors * v;
                                });
                                self.report(r, "Altura de planta cambiada.");
                            }
                        }
                    });
                }
                None => {
                    row(ui, "Altura (mm)", "Cero: una cara", |ui, w| {
                        if let Some(v) =
                            length_field(ui, Id::unique(("mass-h", id)), m.height, w - 8.0)
                        {
                            let r = self
                                .ed
                                .edit_mass("Altura del volumen", id, |x| x.height = v.max(0.0));
                            self.report(r, "Altura cambiada.");
                        }
                    });
                }
            }
            if let MassShape::Sweep { width, .. } = m.shape {
                row(
                    ui,
                    "Ancho (mm)",
                    "El perfil a lo largo del camino",
                    |ui, w| {
                        if let Some(v) =
                            length_field(ui, Id::unique(("mass-w", id)), width, w - 8.0)
                        {
                            let r = self.ed.edit_mass("Ancho del volumen", id, |x| {
                                if let MassShape::Sweep { width, .. } = &mut x.shape {
                                    *width = v;
                                }
                            });
                            self.report(r, "Ancho cambiado.");
                        }
                    },
                );
            }
            let mat = m.material;
            row(ui, "Material", "", |ui, w| {
                let v = material_combo(ui, Id::unique(("mass-mat", id)), self.ed.model(), mat, w);
                if v != mat {
                    let r = self.ed.paint_mass(id, v);
                    self.report(r, "Material cambiado.");
                }
            });
        });
        group(ui, "mass-figures", "Superficies", |ui| {
            text_row(ui, "Huella (m²)", "", &comma(f.footprint, 2));
            text_row(
                ui,
                "Construida (m²)",
                "Huella por plantas",
                &comma(f.built, 2),
            );
            text_row(
                ui,
                "Edificabilidad",
                "Construida entre la superficie de la parcela: la de las parcelas dibujadas o la escrita en las propiedades de las herramientas de Concepto",
                &f.plot_ratio.map_or("— (sin parcela)".into(), |r| format!("{} m²/m²", comma(r, 2))),
            );
            text_row(
                ui,
                "Ocupación",
                "Huella entre la parcela",
                &f.coverage
                    .map_or("—".into(), |c| format!("{} %", comma(c * 100.0, 1))),
            );
            if let Some(check) = self.mass_setback(&m.outline()) {
                text_row(
                    ui,
                    "Parcela",
                    "Si el volumen queda dentro del retranqueo de la parcela en la que está",
                    check,
                );
            }
        });
        if matches!(m.shape, MassShape::Prism(_)) && !m.is_face() {
            group(ui, "mass-building", "Edificio", |ui| {
                let note = match &m.building {
                    None => "Convertir pone muros en sus caras, un forjado en cada planta y una cubierta plana encima.",
                    Some(_) if m.building_outdated() => "El volumen ha cambiado desde que se convirtió: actualiza el edificio.",
                    Some(_) => "Convertido en edificio. Si cambias el volumen, se puede rehacer.",
                };
                props::note(ui, note);
                let label = match &m.building {
                    None => "Convertir en edificio",
                    Some(_) => "Actualizar el edificio",
                };
                if ui.button(label).clicked() {
                    self.convert_masses(&[id]);
                }
            });
        }
    }

    /// Turns masses into buildings (Convertir).
    pub(super) fn convert_masses(&mut self, ids: &[MassId]) {
        if ids.is_empty() {
            self.complain("Selecciona primero un volumen o una masa para convertirlo en edificio.");
            return;
        }
        match self.ed.masses_to_buildings(ids) {
            Ok(n) => self.say(format!(
                "Edificio hecho: {n} muros y forjados, con las plantas que hacían falta."
            )),
            Err(e) => self.complain(format!("No se puede convertir: {e}.")),
        }
    }

    /// The masses selected.
    pub(super) fn selected_masses(&self) -> Vec<MassId> {
        self.ed
            .selection
            .iter()
            .filter_map(|e| match e {
                Element::Mass(id) => Some(*id),
                _ => None,
            })
            .collect()
    }
}

/// Whether a line drawn closes a contour with loose edges already there:
/// a chain of them from its last point back to its first. Returns the
/// contour and the edges used. Ends within `reach` count as meeting.
pub fn close_with_edges(
    path: &[Point2],
    edges: &[(archisim_core::EdgeId, Vec<Point2>)],
    reach: f64,
) -> Option<(Vec<Point2>, Vec<archisim_core::EdgeId>)> {
    let (start, end) = (*path.first()?, *path.last()?);
    if path.len() >= 3 && start.distance(end) <= reach {
        let mut outline = path.to_vec();
        outline.pop();
        return Some((outline, Vec::new()));
    }
    // Breadth first over the edges, from the line's end to its start.
    let mut seen = vec![false; edges.len()];
    let mut queue: std::collections::VecDeque<(Point2, Vec<(usize, bool)>)> =
        std::collections::VecDeque::new();
    queue.push_back((end, Vec::new()));
    while let Some((at, chain)) = queue.pop_front() {
        for (i, (_, pts)) in edges.iter().enumerate() {
            if seen[i] || pts.len() < 2 {
                continue;
            }
            let (a, b) = (pts[0], pts[pts.len() - 1]);
            let forward = if a.distance(at) <= reach {
                true
            } else if b.distance(at) <= reach {
                false
            } else {
                continue;
            };
            seen[i] = true;
            let other = if forward { b } else { a };
            let mut next = chain.clone();
            next.push((i, forward));
            if other.distance(start) <= reach {
                let mut outline = path.to_vec();
                for (j, fwd) in &next {
                    let p = &edges[*j].1;
                    let mut inner: Vec<Point2> = if *fwd {
                        p.clone()
                    } else {
                        p.iter().rev().copied().collect()
                    };
                    // Its first point is the one already there; its last,
                    // the next edge's first (or the line's start).
                    inner.remove(0);
                    inner.pop();
                    outline.extend(inner);
                }
                if outline.len() < 3 {
                    return None;
                }
                return Some((outline, next.iter().map(|(j, _)| edges[*j].0).collect()));
            }
            queue.push_back((other, next));
        }
    }
    None
}
