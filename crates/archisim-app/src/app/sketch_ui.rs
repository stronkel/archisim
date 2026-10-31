//! The sketch mode of a slab's, a roof's or a terrain's contour, as
//! Revit's «Editar contorno» (G2): the element hides and its contour comes
//! back as loose magenta lines on a layer of their own («Boceto»), one per
//! edge. They are drawn and edited with the usual drawing commands — line,
//! rectangle, arc, circle, polygon; trim, extend, corner, split, offset,
//! move, delete — from the sketch bar at the top of the plan, and nothing
//! of the element changes until Aceptar: then the lines must close into
//! contours (a slab's largest is its outline and the rest its holes), the
//! element takes them, the lines go, and the whole edit is one step that
//! can be undone. Cancelar puts everything back as it was.
//!
//! The logic of the contours is in `crate::sketch`.

use archisim_core::{
    DrawLayer, DrawLayerId, LineStyle, ModelError, NewDraft, Point2, Shape, ViewRef,
};

use super::*;
use crate::sketch::{loops_from_segs, Sketch, SketchError, SketchTarget};

/// The sketch being edited.
pub(crate) struct SketchState {
    /// The element's contour when it started (for what it keeps: a roof's
    /// slopes, a terrain's heights).
    pub sketch: Sketch,
    /// The undo step before it started: Cancelar goes back to it, Aceptar
    /// joins everything after it into one step.
    mark: usize,
    /// The layer of the sketch lines and the plan they are drawn on.
    layer: DrawLayerId,
    view: ViewRef,
    /// The drawing settings before (the sketch draws on its layer, by
    /// layer).
    settings: crate::cad::CadSettings,
    /// Where the lines were left open at the last Aceptar.
    pub open_ends: Vec<Point2>,
    /// A wall's sketch is drawn in elevation on a view of its own, made
    /// for it and taken away at the end, with the face and its openings
    /// as reference lines on a grey layer.
    elevation: Option<(archisim_core::DraftViewId, DrawLayerId)>,
    /// The exterior face is seen from outside: its start on the right
    /// (`x` drawn as the wall's length minus `x`).
    mirror: Option<f64>,
}

/// The layer of the reference lines of a sketch in elevation.
pub const SKETCH_REFERENCE_LAYER: &str = "Boceto · referencia";

/// A point of a wall's sketch as drawn, or back (the same mirror).
fn mirrored(mirror: Option<f64>, p: Point2) -> Point2 {
    match mirror {
        Some(len) => Point2::new(len - p.x, p.y),
        None => p,
    }
}

/// The reference lines of a wall's sketch in elevation: the face's
/// border and its doors and windows, in the sketch's coordinates.
fn wall_references(m: &archisim_core::Model, target: SketchTarget) -> Vec<Vec<Point2>> {
    let (w, up) = match target {
        SketchTarget::WallFace(w, _) => (w, 0.0),
        SketchTarget::WallProfile(w) => (w, 1.0),
        // A face standing up: its outline seen from outside.
        SketchTarget::Zones(e, face) => {
            let Some(id) = super::paint_ui::plain_owner(e).map(|o| o.id()) else {
                return Vec::new();
            };
            return archisim_geom::solid::face_frame(m, id, face, None)
                .and_then(|f| f.extent())
                .map(|(lo, hi)| {
                    vec![vec![
                        lo,
                        Point2::new(hi.x, lo.y),
                        hi,
                        Point2::new(lo.x, hi.y),
                    ]]
                })
                .unwrap_or_default();
        }
        _ => return Vec::new(),
    };
    let Some(wall) = m.wall(w) else {
        return Vec::new();
    };
    // The face measures from the level, the profile from the wall's base.
    let base = wall.base_offset * (1.0 - up);
    let mut out = Vec::new();
    if up == 0.0 {
        let p = wall
            .profile
            .clone()
            .unwrap_or_else(|| archisim_core::WallProfile::rectangle(wall.length(), wall.height));
        out.push(
            p.outline
                .iter()
                .map(|q| Point2::new(q.x, q.y + base))
                .collect(),
        );
    }
    for o in m.openings().filter(|o| o.wall == w) {
        if let Some(g) = archisim_geom::opening_geometry(m, o.id) {
            let (z0, z1) = (
                g.sill - wall.base_offset * up,
                g.head - wall.base_offset * up,
            );
            out.push(vec![
                Point2::new(g.t0, z0),
                Point2::new(g.t1, z0),
                Point2::new(g.t1, z1),
                Point2::new(g.t0, z1),
            ]);
        }
    }
    out
}

/// The colour of a sketch, as Revit's.
const SKETCH: Color32 = Color32::from_rgb(196, 30, 150);
/// The sketch layer's name.
pub const SKETCH_LAYER: &str = "Boceto";

fn sketch_error(e: SketchError) -> ModelError {
    match e {
        SketchError::Model(e) => e,
        _ => ModelError::InvalidOutline,
    }
}

/// The tools of the sketch bar: label, command and what it does.
const SKETCH_TOOLS: &[(&str, &str, &str)] = &[
    ("Línea", "LINEA", "Líneas sueltas, una tras otra"),
    ("Rectángulo", "RECTANG", "Cuatro líneas por dos esquinas"),
    ("Polígono", "POLIGONO", "Polígono regular"),
    ("Arco", "ARCO", "Arco por tres puntos"),
    ("Círculo", "CIRCULO", "Círculo por centro y radio"),
];

const SKETCH_EDITS: &[(&str, &str, &str)] = &[
    (
        "Esquina",
        "EMPALME",
        "Recortar o alargar dos líneas hasta que formen esquina (con radio 0)",
    ),
    ("Recortar", "RECORTA", "Quitar el trozo de línea hasta otra"),
    ("Alargar", "ALARGA", "Alargar una línea hasta otra"),
    ("Partir", "PARTEPUNTO", "Partir una línea en un punto"),
    ("Desfase", "EQUIDIST", "Una línea paralela a una distancia"),
    ("Mover", "DESPLAZA", "Mover las líneas elegidas"),
    ("Copiar", "COPIA", "Copiar las líneas elegidas"),
    ("Borrar", "BORRA", "Borrar las líneas elegidas"),
];

impl ArchiSimApp {
    /// Starts editing the contour of an element: it hides, and its edges
    /// come back as sketch lines.
    pub(super) fn start_sketch(&mut self, target: SketchTarget) {
        let Some(sketch) = Sketch::of(self.ed.model(), target) else {
            return;
        };
        if self.sketch.is_some() {
            self.finish_sketch();
            if self.sketch.is_some() {
                return;
            }
        }
        self.end_command(false);
        let in_elevation = sketch.in_elevation();
        if self.mode != ViewMode::Plan && !in_elevation {
            self.show_plan();
        }
        // The exterior side of a wall is seen from outside.
        let mirror = match sketch.target {
            SketchTarget::WallFace(w, archisim_core::FaceSide::Exterior) => {
                self.ed.model().wall(w).map(|w| w.length())
            }
            _ => None,
        };
        let title = match sketch.target {
            SketchTarget::WallFace(w, side) => format!(
                "Boceto · {} · cara {}",
                self.ed
                    .model()
                    .wall(w)
                    .map_or("muro".into(), |w| format!("muro {}", w.id.0)),
                match side {
                    archisim_core::FaceSide::Exterior => "exterior",
                    archisim_core::FaceSide::Interior => "interior",
                }
            ),
            SketchTarget::WallProfile(w) => format!("Boceto · perfil del muro {}", w.0),
            SketchTarget::Zones(..) => "Boceto · cara".to_string(),
            _ => String::new(),
        };
        let references = wall_references(self.ed.model(), sketch.target);
        let mark = self.ed.undo_mark();
        let loops = sketch.loops.clone();
        let level_view = ViewRef::Level(self.ed.level);
        let r = self.ed.transact("Boceto", |m| {
            let layer = match m.draw_layer_by_name(SKETCH_LAYER) {
                Some(l) => l.id,
                None => m.add_draw_layer(DrawLayer::new(
                    SKETCH_LAYER,
                    [196, 30, 150],
                    0.35,
                    LineStyle::Continuous,
                ))?,
            };
            // In elevation: a view of its own with the references.
            let elevation = if in_elevation {
                let v = m.add_draft_view(title.clone(), 50.0)?;
                let grey = match m.draw_layer_by_name(SKETCH_REFERENCE_LAYER) {
                    Some(l) => l.id,
                    None => m.add_draw_layer(DrawLayer::new(
                        SKETCH_REFERENCE_LAYER,
                        [150, 150, 150],
                        0.18,
                        LineStyle::Continuous,
                    ))?,
                };
                for lp in &references {
                    let n = lp.len();
                    for i in 0..n {
                        m.add_draft(NewDraft::new(
                            ViewRef::Drafting(v),
                            grey,
                            Shape::Line {
                                a: mirrored(mirror, lp[i]),
                                b: mirrored(mirror, lp[(i + 1) % n]),
                            },
                        ))?;
                    }
                }
                Some((v, grey))
            } else {
                None
            };
            let view = elevation.map_or(level_view, |(v, _)| ViewRef::Drafting(v));
            for lp in &loops {
                let n = lp.len();
                for i in 0..n {
                    m.add_draft(NewDraft::new(
                        view,
                        layer,
                        Shape::Line {
                            a: mirrored(mirror, lp[i]),
                            b: mirrored(mirror, lp[(i + 1) % n]),
                        },
                    ))?;
                }
            }
            Ok((layer, elevation))
        });
        let (layer, elevation) = match r {
            Ok(l) => l,
            Err(e) => {
                self.complain(format!("No se puede: {e}."));
                return;
            }
        };
        let view = elevation.map_or(level_view, |(v, _)| ViewRef::Drafting(v));
        if let Some((v, _)) = elevation {
            self.open_view(super::tabs::TabKind::Drafting(v));
            self.fit_requested = true;
        }
        // A contour is redrawn without its element; a face is divided
        // over it, and an elevation drawn elsewhere.
        if !sketch.is_zones() && !in_elevation {
            self.ed.hide([sketch.element()]);
        }
        self.ed.selection.clear();
        let settings = self.cad_settings.clone();
        self.cad_settings.layer = layer;
        self.cad_settings.color = None;
        self.cad_settings.weight = None;
        self.cad_settings.line = None;
        self.tool = Tool::Arrow;
        self.drag = None;
        self.sketch = Some(SketchState {
            sketch,
            mark,
            layer,
            view,
            settings,
            open_ends: Vec::new(),
            elevation,
            mirror,
        });
    }

    /// The sketch lines now, as segments.
    fn sketch_segs(&self, st: &SketchState) -> Vec<archisim_draft::Seg> {
        self.ed
            .model()
            .drafts_in(st.view)
            .filter(|d| d.layer == st.layer)
            .flat_map(|d| archisim_draft::shape_segs(&d.shape))
            .collect()
    }

    /// Puts the sketch into the model, in one step that can be undone.
    pub(super) fn finish_sketch(&mut self) {
        self.end_command(true);
        let Some(mut st) = self.sketch.take() else {
            return;
        };
        let segs = self.sketch_segs(&st);
        let zones = st.sketch.is_zones();
        let mirror = st.mirror;
        let loops = if zones && segs.is_empty() {
            // Every zone taken away.
            Ok(Vec::new())
        } else {
            loops_from_segs(&segs).map(|ls| {
                ls.into_iter()
                    .map(|l| l.into_iter().map(|p| mirrored(mirror, p)).collect())
                    .collect()
            })
        };
        let sketch = match loops.and_then(|l| st.sketch.with_loops(l)) {
            Ok(s) => s,
            Err(e) => {
                if let SketchError::Open(at) = &e {
                    st.open_ends = at.clone();
                }
                self.sketch = Some(st);
                self.complain(format!("No se puede aceptar: {e}."));
                return;
            }
        };
        let (layer, view, elevation) = (st.layer, st.view, st.elevation);
        let r = self.ed.transact("Editar contorno", |m| {
            let lines: Vec<_> = m
                .drafts_in(view)
                .filter(|d| d.layer == layer || elevation.is_some_and(|(_, g)| d.layer == g))
                .map(|d| d.id)
                .collect();
            for id in lines {
                m.remove_draft(id)?;
            }
            if let Some((v, _)) = elevation {
                m.remove_draft_view(v)?;
            }
            sketch.apply(m).map_err(sketch_error)
        });
        match r {
            Ok(()) => {
                let label = match st.sketch.target {
                    SketchTarget::WallProfile(_) => "Editar perfil",
                    _ if zones => "Dividir cara",
                    _ => "Editar contorno",
                };
                self.ed.join_since(st.mark, label);
                let said = match st.sketch.target {
                    SketchTarget::WallProfile(_) => "Perfil del muro cambiado.",
                    _ if zones => "Cara dividida: pinta cada zona con Pintar.",
                    _ => "Contorno cambiado.",
                };
                self.leave_sketch(st);
                self.say(said);
            }
            Err(e) => {
                // Still in the sketch, to put it right.
                self.sketch = Some(st);
                self.complain(format!("No se puede: {e}."));
            }
        }
    }

    /// Leaves everything as it was before the sketch.
    pub(super) fn cancel_sketch(&mut self) {
        self.end_command(false);
        let Some(st) = self.sketch.take() else {
            return;
        };
        while self.ed.undo_mark() > st.mark {
            if self.ed.undo().is_none() {
                break;
            }
        }
        self.leave_sketch(st);
        self.say("Contorno sin cambios.");
    }

    fn leave_sketch(&mut self, st: SketchState) {
        if st.elevation.is_some() {
            // Its view is gone: back to where the wall was chosen.
            self.sync_tabs();
            if !matches!(self.mode, ViewMode::Plan | ViewMode::ThreeD) {
                self.show_plan();
            }
        }
        self.ed.unhide([st.sketch.element()]);
        self.ed.selection = [st.sketch.element()].into_iter().collect();
        self.cad_settings = st.settings;
        self.tool = Tool::Arrow;
    }

    /// Keys while sketching: Intro accepts when no command is running.
    pub(super) fn sketch_keys(&mut self, ctx: &egui::Context) {
        if self.sketch.is_none() || self.cad.is_some() || crate::app::typing_in_text(ctx) {
            return;
        }
        if ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Enter)) {
            self.finish_sketch();
        }
    }

    /// What the sketch adds over the plan: the ends left open, in red.
    pub(super) fn draw_sketch(&self, painter: &egui::Painter, rect: Rect) {
        let Some(st) = self.sketch.as_ref() else {
            return;
        };
        for p in &st.open_ends {
            let c = self.view.to_screen(rect, *p);
            painter.circle_stroke(c, 7.0, Stroke::new(2.0, Color32::from_rgb(220, 40, 40)));
        }
    }

    /// The bar of the sketch, at the top of the plan.
    pub(super) fn sketch_panel(&mut self, ctx: &egui::Context, rect: Rect) {
        let Some(st) = self.sketch.as_ref() else {
            return;
        };
        let what = match st.sketch.target {
            SketchTarget::Slab(_) => "contorno del forjado",
            SketchTarget::Roof(_) => "contorno de la cubierta",
            SketchTarget::Terrain(_) => "contorno del terreno",
            SketchTarget::Zones(_, archisim_core::FaceRef::Top) => "dividir la cara de arriba",
            SketchTarget::Zones(_, archisim_core::FaceRef::Bottom) => "dividir la cara de abajo",
            SketchTarget::Zones(..) if st.elevation.is_some() => {
                "dividir la cara (vista desde fuera)"
            }
            SketchTarget::Zones(..) => "dividir el faldón",
            SketchTarget::WallFace(_, archisim_core::FaceSide::Exterior) => {
                "dividir la cara exterior del muro (vista desde fuera)"
            }
            SketchTarget::WallFace(..) => "dividir la cara interior del muro",
            SketchTarget::WallProfile(_) => "perfil del muro en alzado",
        };
        let zones = st.sketch.is_zones();
        let profile = matches!(st.sketch.target, SketchTarget::WallProfile(_));
        let holes = st.sketch.takes_holes();
        let running = self.cad.as_ref().map(|c| c.name());
        let mut start: Option<&'static str> = None;
        let mut finish = false;
        let mut cancel = false;
        egui::Area::new(egui::Id::unique("sketch-bar"))
            .fixed_pos(rect.left_top() + Vec2::new(10.0, 10.0))
            .order(egui::Order::Foreground)
            .show(ctx, |ui| {
                egui::Frame::popup(ui.style()).show(ui, |ui| {
                    ui.set_max_width(rect.width() - 40.0);
                    ui.horizontal_wrapped(|ui| {
                        ui.label(
                            egui::RichText::new(format!("Modo boceto · {what}"))
                                .strong()
                                .color(SKETCH),
                        );
                        ui.separator();
                        ui.label(egui::RichText::new("Dibujar").small().weak());
                        for (label, cmd, hint) in SKETCH_TOOLS {
                            let on = running == Some(*cmd);
                            if ui.selectable_label(on, *label).on_hover_text(*hint).clicked() {
                                start = Some(cmd);
                            }
                        }
                        ui.separator();
                        ui.label(egui::RichText::new("Modificar").small().weak());
                        for (label, cmd, hint) in SKETCH_EDITS {
                            let on = running == Some(*cmd);
                            if ui.selectable_label(on, *label).on_hover_text(*hint).clicked() {
                                start = Some(cmd);
                            }
                        }
                        ui.separator();
                        if ui
                            .add(
                                egui::Button::new(
                                    egui::RichText::new("✔ Aceptar").strong().color(Color32::WHITE),
                                )
                                .fill(Color32::from_rgb(46, 125, 50)),
                            )
                            .on_hover_text("Intro: el elemento toma el contorno dibujado")
                            .clicked()
                        {
                            finish = true;
                        }
                        if ui
                            .button("✖ Cancelar")
                            .on_hover_text("Deja el elemento como estaba")
                            .clicked()
                        {
                            cancel = true;
                        }
                    });
                    let note = if profile {
                        "Las líneas tienen que cerrar contornos: el mayor es el muro en alzado y los de dentro, sus huecos. En gris, sus puertas y ventanas."
                    } else if zones {
                        "Cada contorno cerrado que dibujes sobre la cara es una zona que se pinta aparte (Pintar)."
                    } else if holes {
                        "Las líneas tienen que cerrar contornos: el mayor es el borde del forjado y los de dentro, sus huecos."
                    } else {
                        "Las líneas tienen que cerrar un solo contorno."
                    };
                    ui.label(egui::RichText::new(note).size(11.0).weak());
                });
            });
        if let Some(cmd) = start {
            if cmd == "EMPALME" {
                // A corner, as Revit's «Recortar/Extender a esquina».
                self.cad_settings.fillet_radius = 0.0;
            }
            if let Some(st) = self.sketch.as_mut() {
                st.open_ends.clear();
            }
            self.start_command(cmd);
        }
        if finish {
            self.finish_sketch();
        } else if cancel {
            self.cancel_sketch();
        }
    }
}

/// The button that starts editing an element's contour.
pub(super) fn edit_contour_button(ui: &mut Ui) -> bool {
    ui.add_space(4.0);
    ui.button("✏ Editar contorno")
        .on_hover_text(
            "Modo boceto: el contorno en líneas sueltas para dibujar, recortar, alargar o borrar; Aceptar o Cancelar",
        )
        .clicked()
}
