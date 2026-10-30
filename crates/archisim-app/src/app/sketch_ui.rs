//! The sketch mode of a slab's, a roof's or a terrain's contour (E4): its
//! lines drawn over the plan in magenta, their vertices and edges dragged,
//! a double click adds a vertex, Supr takes the picked one away, and a
//! bar at the top of the plan offsets the outline, turns the picked edge
//! into an arc, trims or extends two edges to their corner, adds or takes
//! away a slab's holes, and finishes (Intro) or cancels (Esc). The logic
//! is in `crate::sketch`.

use archisim_core::{ModelError, Point2};

use super::*;
use crate::sketch::{Sketch, SketchError, SketchTarget};

/// The sketch being edited and what the mouse is doing with it.
pub(crate) struct SketchState {
    pub sketch: Sketch,
    /// What is being dragged: a vertex, or an edge from where it was
    /// taken.
    drag: Option<SketchDrag>,
    /// The vertex picked (Supr takes it away).
    vertex: Option<(usize, usize)>,
    /// How far the outline is offset (m; negative, inwards).
    offset: f64,
    /// How far the arc bulges, as a fraction of its chord.
    bulge: f64,
    /// Waiting for the second edge of a corner.
    corner: bool,
}

#[derive(Debug, Clone, Copy)]
enum SketchDrag {
    Vertex(usize, usize),
    Edge(usize, usize, Point2),
}

/// The colour of a sketch, as Revit's.
const SKETCH: Color32 = Color32::from_rgb(196, 30, 150);

fn sketch_error(e: SketchError) -> ModelError {
    match e {
        SketchError::Model(e) => e,
        _ => ModelError::InvalidOutline,
    }
}

impl ArchiSimApp {
    /// Starts editing the contour of an element.
    pub(super) fn start_sketch(&mut self, target: SketchTarget) {
        let Some(sketch) = Sketch::of(self.ed.model(), target) else {
            return;
        };
        self.tool = Tool::Arrow;
        self.drag = None;
        self.sketch = Some(SketchState {
            sketch,
            drag: None,
            vertex: None,
            offset: 0.1,
            bulge: 0.15,
            corner: false,
        });
        self.say(
            "Editar contorno: arrastre vértices y aristas; doble clic añade un vértice, Supr quita el elegido. Intro termina, Esc cancela.",
        );
    }

    /// Puts the sketch into the model, in one step that can be undone.
    pub(super) fn finish_sketch(&mut self) {
        let Some(st) = self.sketch.take() else {
            return;
        };
        let sketch = st.sketch.clone();
        let r = self
            .ed
            .transact("Editar contorno", |m| sketch.apply(m).map_err(sketch_error));
        match r {
            Ok(()) => self.say("Contorno cambiado."),
            Err(e) => {
                // Still in the sketch, to put it right.
                self.sketch = Some(st);
                self.complain(format!("No se puede: {e}."));
            }
        }
    }

    pub(super) fn cancel_sketch(&mut self) {
        if self.sketch.take().is_some() {
            self.say("Contorno sin cambios.");
        }
    }

    /// The mouse and keys while sketching (instead of the usual ones).
    pub(super) fn sketch_input(
        &mut self,
        ctx: &egui::Context,
        response: &egui::Response,
        rect: Rect,
    ) {
        let reach = self.view.to_metres(REACH);
        let at = self.cursor.map(|c| c.point);
        let typing = crate::app::typing_in_text(ctx);
        let (enter, esc, del) = ctx.input(|i| {
            (
                i.key_pressed(egui::Key::Enter),
                i.key_pressed(egui::Key::Escape),
                i.key_pressed(egui::Key::Delete) || i.key_pressed(egui::Key::Backspace),
            )
        });
        if !typing {
            if enter {
                self.finish_sketch();
                return;
            }
            if esc {
                self.cancel_sketch();
                return;
            }
        }
        let Some(st) = self.sketch.as_mut() else {
            return;
        };
        if del && !typing {
            if let Some((l, i)) = st.vertex.take() {
                if let Err(e) = st.sketch.remove_vertex(l, i) {
                    let msg = format!("No se puede: {e}.");
                    self.complain(msg);
                }
                return;
            }
        }
        let Some(at) = at else {
            return;
        };
        if response.double_clicked_by(PointerButton::Primary) {
            if let Some((l, i)) = st.sketch.edge_near(at, reach) {
                let k = st.sketch.insert_vertex(l, i, at);
                st.vertex = k.map(|k| (l, k));
            }
            return;
        }
        if response.drag_started_by(PointerButton::Primary) {
            // What is under where the button went down, not where the
            // pointer is once it has moved enough to be a drag.
            let from = ctx
                .input(|i| i.pointer.press_origin())
                .map_or(at, |p| self.view.to_model(rect, p));
            let at = from;
            st.drag = if let Some((l, i)) = st.sketch.vertex_near(at, reach) {
                st.vertex = Some((l, i));
                Some(SketchDrag::Vertex(l, i))
            } else if let Some((l, i)) = st.sketch.edge_near(at, reach) {
                st.sketch.edge = Some((l, i));
                Some(SketchDrag::Edge(l, i, at))
            } else {
                None
            };
        }
        if response.dragged_by(PointerButton::Primary) {
            match st.drag {
                Some(SketchDrag::Vertex(l, i)) => st.sketch.move_vertex(l, i, at),
                Some(SketchDrag::Edge(l, i, from)) => {
                    st.sketch.move_edge(l, i, at - from);
                    st.drag = Some(SketchDrag::Edge(l, i, at));
                }
                None => {}
            }
        }
        if response.drag_stopped() {
            st.drag = None;
        }
        if response.clicked_by(PointerButton::Primary) {
            if let Some(v) = st.sketch.vertex_near(at, reach) {
                st.vertex = Some(v);
                return;
            }
            st.vertex = None;
            let picked = st.sketch.edge_near(at, reach);
            match (st.corner, st.sketch.edge, picked) {
                (true, Some((l, i)), Some((l2, j))) if l == l2 => {
                    st.corner = false;
                    match st.sketch.corner(l, i, j) {
                        Ok(()) => st.sketch.edge = None,
                        Err(e) => {
                            let msg = format!("No se puede: {e}.");
                            self.complain(msg);
                        }
                    }
                }
                _ => {
                    st.corner = false;
                    st.sketch.edge = picked;
                }
            }
        }
    }

    /// The sketch over the plan.
    pub(super) fn draw_sketch(&self, painter: &egui::Painter, rect: Rect) {
        let Some(st) = self.sketch.as_ref() else {
            return;
        };
        let s = &st.sketch;
        for (l, lp) in s.loops.iter().enumerate() {
            let n = lp.len();
            for i in 0..n {
                let (a, b) = (
                    self.view.to_screen(rect, lp[i]),
                    self.view.to_screen(rect, lp[(i + 1) % n]),
                );
                let picked = s.edge == Some((l, i));
                let w = if picked { 3.5 } else { 2.0 };
                let c = if picked { ACCENT } else { SKETCH };
                painter.line_segment([a, b], Stroke::new(w, c));
            }
            for (i, p) in lp.iter().enumerate() {
                let q = self.view.to_screen(rect, *p);
                let r = Rect::from_center_size(q, Vec2::splat(7.0));
                let on = st.vertex == Some((l, i));
                painter.rect_filled(r, CornerRadius::ZERO, if on { ACCENT } else { PAPER });
                painter.rect_stroke(
                    r,
                    CornerRadius::ZERO,
                    Stroke::new(1.5, SKETCH),
                    StrokeKind::Inside,
                );
            }
        }
    }

    /// The bar of the sketch, at the top of the plan.
    pub(super) fn sketch_panel(&mut self, ctx: &egui::Context, rect: Rect) {
        let Some(st) = self.sketch.as_mut() else {
            return;
        };
        let what = match st.sketch.target {
            SketchTarget::Slab(_) => "del forjado",
            SketchTarget::Roof(_) => "de la cubierta",
            SketchTarget::Terrain(_) => "del terreno",
        };
        let mut finish = false;
        let mut cancel = false;
        let mut problem: Option<String> = None;
        egui::Area::new(egui::Id::unique("sketch-bar"))
            .fixed_pos(rect.left_top() + Vec2::new(10.0, 44.0))
            .order(egui::Order::Foreground)
            .show(ctx, |ui| {
                egui::Frame::popup(ui.style()).show(ui, |ui| {
                    ui.set_max_width(rect.width() - 40.0);
                    ui.horizontal_wrapped(|ui| {
                        ui.label(egui::RichText::new(format!("Editar contorno {what}")).strong());
                        ui.separator();
                        ui.label("Desfase");
                        ui.add(
                            egui::DragValue::new(&mut st.offset)
                                .speed(0.01)
                                .range(-5.0..=5.0)
                                .suffix(" m"),
                        );
                        if ui
                            .button("Desfasar")
                            .on_hover_text("Todo el contorno, hacia fuera (negativo: hacia dentro)")
                            .clicked()
                        {
                            if let Err(e) = st.sketch.offset(st.offset) {
                                problem = Some(e.to_string());
                            }
                        }
                        ui.separator();
                        let edge = st.sketch.edge;
                        ui.label("Arco");
                        ui.add(
                            egui::DragValue::new(&mut st.bulge)
                                .speed(0.01)
                                .range(-0.5..=0.5)
                                .custom_formatter(|v, _| format!("{:.0} %", v * 100.0))
                                .custom_parser(|t| {
                                    t.trim_end_matches('%')
                                        .trim()
                                        .replace(',', ".")
                                        .parse::<f64>()
                                        .ok()
                                        .map(|v| v / 100.0)
                                }),
                        )
                        .on_hover_text("Flecha del arco, en tanto por ciento de la arista (negativo: hacia dentro)");
                        if ui
                            .add_enabled(edge.is_some(), egui::Button::new("Arista en arco"))
                            .on_disabled_hover_text("Elija antes una arista")
                            .clicked()
                        {
                            if let Some((l, i)) = edge {
                                if st.sketch.arc_edge(l, i, st.bulge, 16).is_none() {
                                    problem = Some("esa arista no puede ser un arco".into());
                                }
                            }
                        }
                        if ui
                            .add_enabled(edge.is_some(), egui::Button::new("Recortar/alargar a esquina"))
                            .on_hover_text("Elija después la otra arista: las dos se recortan o alargan hasta encontrarse")
                            .on_disabled_hover_text("Elija antes una arista")
                            .clicked()
                        {
                            st.corner = true;
                        }
                        if st.sketch.takes_holes() {
                            ui.separator();
                            if ui.button("Añadir hueco").clicked() {
                                let c = centroid(&st.sketch.loops[0]);
                                st.sketch.add_hole(c, 1.0, 1.0);
                            }
                            let hole = edge.map(|e| e.0).filter(|&l| l > 0);
                            if ui
                                .add_enabled(hole.is_some(), egui::Button::new("Quitar hueco"))
                                .on_disabled_hover_text("Elija antes una arista del hueco")
                                .clicked()
                            {
                                if let Some(l) = hole {
                                    st.sketch.remove_hole(l);
                                }
                            }
                        }
                        ui.separator();
                        if ui
                            .add(egui::Button::new(egui::RichText::new("✔ Finalizar").strong()))
                            .on_hover_text("Intro")
                            .clicked()
                        {
                            finish = true;
                        }
                        if ui.button("✖ Cancelar").on_hover_text("Esc").clicked() {
                            cancel = true;
                        }
                    });
                    if st.corner {
                        ui.label(
                            egui::RichText::new("Elija la otra arista de la esquina.")
                                .color(SKETCH)
                                .size(11.5),
                        );
                    }
                });
            });
        if let Some(p) = problem {
            self.complain(format!("No se puede: {p}."));
        }
        if finish {
            self.finish_sketch();
        } else if cancel {
            self.cancel_sketch();
        }
    }
}

/// The middle of a polygon (its vertices' mean).
fn centroid(lp: &[Point2]) -> Point2 {
    let n = lp.len().max(1) as f64;
    let (x, y) = lp.iter().fold((0.0, 0.0), |a, p| (a.0 + p.x, a.1 + p.y));
    Point2::new(x / n, y / n)
}

/// The button that starts editing an element's contour.
pub(super) fn edit_contour_button(ui: &mut Ui) -> bool {
    ui.add_space(4.0);
    ui.button("✏ Editar contorno")
        .on_hover_text(
            "Modo boceto: mueva vértices y aristas, desfase el contorno, arcos y esquinas; Finalizar o Cancelar",
        )
        .clicked()
}
