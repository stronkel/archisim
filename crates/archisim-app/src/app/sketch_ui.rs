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
use crate::sketch::{
    chain_from_segs, chains_from_segs, loops_from_segs, Sketch, SketchError, SketchTarget,
};

/// The sketch being edited.
pub(crate) struct SketchState {
    /// The element's contour when it started (for what it keeps: a roof's
    /// slopes, a terrain's heights).
    pub sketch: Sketch,
    /// The undo step before it started: Cancelar goes back to it, Aceptar
    /// joins everything after it into one step.
    mark: usize,
    /// The layer of the sketch lines and the plan they are drawn on.
    pub layer: DrawLayerId,
    pub view: ViewRef,
    /// The drawing settings before (the sketch draws on its layer, by
    /// layer).
    settings: crate::cad::CadSettings,
    /// Where the lines were left open at the last Aceptar.
    pub open_ends: Vec<Point2>,
    /// «Elegir línea» (Revit's «Seleccionar líneas»): a click copies the
    /// line or edge under it as a sketch line.
    pub pick: bool,
    /// Which of the lines under the cursor (Tab goes to the next).
    pub pick_cycle: usize,
    /// Where the cursor was when it last moved, to start again from the
    /// nearest line.
    pub pick_at: Option<Point2>,
    /// A wall's sketch is drawn in elevation on a view of its own, made
    /// for it and taken away at the end, with the face and its openings
    /// as reference lines on a grey layer.
    elevation: Option<(archisim_core::DraftViewId, DrawLayerId)>,
    /// The exterior face is seen from outside: its start on the right
    /// (`x` drawn as the wall's length minus `x`).
    mirror: Option<f64>,
    /// A face composed by hand: what a click on a piece does to it.
    pub piece: Option<PieceMode>,
    /// The material a click paints a piece with.
    pub accent: Option<archisim_core::MaterialId>,
    /// The pieces painted in this stroke of the mouse (painted once
    /// each while it is held).
    pub stroke: Vec<Point2>,
    /// «Repartir»: into how many pieces a click splits a piece, and
    /// whether with upright joints.
    pub split: (usize, bool),
}

/// What a click on a piece does, in a face composed by hand (M20).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PieceMode {
    /// Painted with the chosen material.
    Paint,
    /// Left out of the factory, fitted on site.
    OnSite,
    /// Back to the layer's material and the factory.
    Plain,
    /// Split into equal pieces (`SketchState::split`).
    Split,
}

/// How much of the rest of the drawing the sketch's veil hides (of 255).
const SKETCH_VEIL: u8 = 165;

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
        SketchTarget::WallFace(w, _) | SketchTarget::BoardFace(w, _) => (w, 0.0),
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
pub(super) const SKETCH: Color32 = Color32::from_rgb(196, 30, 150);
/// The sketch layer's name.
pub const SKETCH_LAYER: &str = "Boceto";

fn sketch_error(e: SketchError) -> ModelError {
    match e {
        SketchError::Model(e) => e,
        _ => ModelError::InvalidOutline,
    }
}

/// The tools of the sketch bar: label, command and what it does.
pub(super) const SKETCH_TOOLS: &[(&str, &str, &str)] = &[
    ("Línea", "LINEA", "Líneas sueltas, una tras otra"),
    ("Rectángulo", "RECTANG", "Cuatro líneas por dos esquinas"),
    ("Polígono", "POLIGONO", "Polígono regular"),
    ("Arco", "ARCO", "Arco por tres puntos"),
    ("Círculo", "CIRCULO", "Círculo por centro y radio"),
];

pub(super) const SKETCH_EDITS: &[(&str, &str, &str)] = &[
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
            SketchTarget::WallFace(w, archisim_core::FaceSide::Exterior)
            | SketchTarget::BoardFace(w, _) => self.ed.model().wall(w).map(|w| w.length()),
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
            SketchTarget::BoardFace(w, layer) => format!(
                "Despiece · muro {} · capa {} · {}",
                w.0,
                layer + 1,
                board_layer_name(self.ed.model(), w, layer)
            ),
            SketchTarget::Zones(..) => "Boceto · cara".to_string(),
            SketchTarget::BoardPlan(..) => format!(
                "Despiece en planta · {}",
                sketch_layer_name(self.ed.model(), sketch.target)
            ),
            _ => String::new(),
        };
        let references = wall_references(self.ed.model(), sketch.target);
        let open = sketch.is_open();
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
                // A street's axis is open: no line back to its start.
                let edges = if open { n.saturating_sub(1) } else { n };
                for i in 0..edges {
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
        let accent = match sketch.target {
            SketchTarget::BoardFace(..) | SketchTarget::BoardPlan(..) => {
                usual_accent(self.ed.model(), sketch.target)
            }
            _ => None,
        };
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
            pick: false,
            pick_cycle: 0,
            pick_at: None,
            elevation,
            mirror,
            piece: None,
            accent,
            stroke: Vec::new(),
            split: (2, true),
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
        let loops = if st.sketch.is_joints() {
            // Loose joint lines, each as drawn.
            Ok(segs
                .iter()
                .map(|s| {
                    let pts = match s {
                        archisim_draft::Seg::Line { a, b } => vec![*a, *b],
                        _ => s.tessellate(0.02),
                    };
                    pts.into_iter().map(|p| mirrored(mirror, p)).collect()
                })
                .collect())
        } else if matches!(st.sketch.target, SketchTarget::RoadEdges(_)) {
            chains_from_segs(&segs)
        } else if st.sketch.is_open() {
            chain_from_segs(&segs).map(|c| vec![c])
        } else if zones && segs.is_empty() {
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
                    SketchTarget::Road(_) => "Editar el eje",
                    SketchTarget::RoadEdges(_) => "Editar bordillos y fachadas",
                    SketchTarget::BoardFace(..) | SketchTarget::BoardPlan(..) => "Despiece a mano",
                    _ if zones => "Dividir cara",
                    _ => "Editar contorno",
                };
                self.ed.join_since(st.mark, label);
                let said = match st.sketch.target {
                    SketchTarget::WallProfile(_) => "Perfil del muro cambiado.",
                    SketchTarget::Road(_) => "Eje de la calle cambiado.",
                    SketchTarget::RoadEdges(_) => "Bordillos y fachadas cambiados: la calle cambia de sección donde los has movido.",
                    SketchTarget::BoardFace(..) => "Despiece a mano guardado: placas, compra, fábrica y 3D lo siguen.",
                    SketchTarget::BoardPlan(..) => "Despiece a mano guardado: placas, compra y fábrica lo siguen.",
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
        // Everything else behind a veil, as in Revit, so that the sketch
        // lines stand out; they are drawn again over it, thicker.
        painter.rect_filled(
            rect,
            0.0,
            Color32::from_rgba_unmultiplied(255, 255, 255, SKETCH_VEIL),
        );
        if st.sketch.is_joints() {
            self.draw_live_pieces(st, painter, rect);
        }
        let stroke = Stroke::new(2.2, SKETCH);
        for seg in self.sketch_segs(st) {
            let pts: Vec<egui::Pos2> = seg
                .tessellate(self.view.to_metres(2.0).max(0.005))
                .into_iter()
                .map(|q| self.view.to_screen(rect, q))
                .collect();
            painter.add(egui::Shape::line(pts, stroke));
        }
        for p in &st.open_ends {
            let c = self.view.to_screen(rect, *p);
            painter.circle_stroke(c, 7.0, Stroke::new(2.0, Color32::from_rgb(220, 40, 40)));
        }
    }

    /// «Elegir línea» in the sketch: on or off.
    pub(super) fn toggle_sketch_pick(&mut self) {
        self.end_command(true);
        let on = match self.sketch.as_mut() {
            Some(st) => {
                st.pick = !st.pick;
                st.pick
            }
            None => return,
        };
        self.pick_hint.clear();
        if on {
            self.say("Elegir línea: en azul, la línea que se toma; Tab pasa a la siguiente bajo el cursor.");
        }
    }

    /// Whether a click in the drawing picks a line for the sketch.
    pub(super) fn sketch_picking(&self) -> bool {
        self.tool == Tool::Arrow && self.sketch.as_ref().is_some_and(|s| s.pick)
    }

    /// What is being sketched and what the lines must do, for the options
    /// bar under the ribbon (where its Aceptar, Cancelar and orders are).
    pub(super) fn sketch_texts(&self) -> Option<(&'static str, &'static str)> {
        let st = self.sketch.as_ref()?;
        let what = match st.sketch.target {
            SketchTarget::Slab(_) => "contorno del forjado",
            SketchTarget::Roof(_) => "contorno de la cubierta",
            SketchTarget::Terrain(_) => "contorno del terreno",
            SketchTarget::TerrainPad(..) => "contorno de la plataforma",
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
            SketchTarget::Road(_) => "eje de la calle (una línea abierta)",
            SketchTarget::RoadEdges(_) => "bordillos y líneas de fachada de la calle",
            SketchTarget::BoardFace(..) => "despiece a mano (vista desde fuera)",
            SketchTarget::BoardPlan(archisim_core::PlanHost::Slab(_), _) => {
                "despiece a mano del forjado (en planta)"
            }
            SketchTarget::BoardPlan(..) => "despiece a mano del falso techo (en planta)",
        };
        let note = if st.sketch.is_joints() {
            match st.piece {
                Some(PieceMode::Paint) => "Clic o arrastrar sobre las piezas: se pintan con el material elegido (un clic en una ya pintada así la devuelve al de la capa).",
                Some(PieceMode::OnSite) => "Clic o arrastrar sobre las piezas: se colocan en obra, fuera de la fábrica (un clic en una en obra la devuelve a fábrica).",
                Some(PieceMode::Plain) => "Clic o arrastrar sobre las piezas: vuelven al material de la capa y a fábrica.",
                Some(PieceMode::Split) => "Clic en una pieza: se reparte en piezas iguales con juntas verticales u horizontales (en la barra de opciones).",
                None => "Cada línea es una junta: corta las piezas que cruza de lado a lado (llévala hasta otra junta o el borde). Luego pinta piezas o márcalas en obra.",
            }
        } else if matches!(st.sketch.target, SketchTarget::RoadEdges(_)) {
            "Cuatro líneas abiertas, de un extremo de la calle al otro: bordillo y fachada de cada lado. Donde las muevas, la calle cambia de sección con su transición; la calzada se queda en su sitio."
        } else if matches!(st.sketch.target, SketchTarget::Road(_)) {
            "Una sola línea abierta, de un extremo de la calle al otro."
        } else if matches!(st.sketch.target, SketchTarget::WallProfile(_)) {
            "Las líneas tienen que cerrar contornos: el mayor es el muro en alzado y los de dentro, sus huecos. En gris, sus puertas y ventanas."
        } else if st.sketch.is_zones() {
            "Cada contorno cerrado que dibujes sobre la cara es una zona que se pinta aparte (Pintar)."
        } else if st.sketch.takes_holes() {
            "Las líneas tienen que cerrar contornos: el mayor es el borde del forjado y los de dentro, sus huecos."
        } else {
            "Las líneas tienen que cerrar un solo contorno."
        };
        Some((what, note))
    }

    /// An order of the sketch's lines, from the ribbon.
    pub(super) fn sketch_command(&mut self, cmd: &'static str) {
        if self.cad.as_ref().map(|c| c.name()) == Some(cmd) {
            self.cad_escape();
            return;
        }
        if cmd == "EMPALME" {
            // A corner, as Revit's «Recortar/Extender a esquina».
            self.cad_settings.fillet_radius = 0.0;
        }
        if let Some(st) = self.sketch.as_mut() {
            st.open_ends.clear();
            st.pick = false;
            st.piece = None;
        }
        self.start_command(cmd);
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

/// The name of a wall's layer: its material.
pub(super) fn board_layer_name(
    m: &archisim_core::Model,
    w: archisim_core::WallId,
    layer: usize,
) -> String {
    sketch_layer_name(m, SketchTarget::BoardFace(w, layer))
}

/// The layer of boards a sketch composes by hand: a wall's or, in plan, a
/// slab's or a ceiling's.
pub(super) fn sketch_layer(
    m: &archisim_core::Model,
    target: SketchTarget,
) -> Option<archisim_core::Layer> {
    match target {
        SketchTarget::BoardFace(w, layer) => m
            .wall(w)
            .and_then(|w| m.wall_type(w.wall_type))
            .and_then(|t| t.layers.get(layer))
            .cloned(),
        SketchTarget::BoardPlan(host, layer) => {
            m.plan_host_layers(host).and_then(|l| l.get(layer)).cloned()
        }
        _ => None,
    }
}

/// The name of the material of the layer a sketch composes by hand.
pub(super) fn sketch_layer_name(m: &archisim_core::Model, target: SketchTarget) -> String {
    let n = match target {
        SketchTarget::BoardFace(_, l) | SketchTarget::BoardPlan(_, l) => l,
        _ => 0,
    };
    sketch_layer(m, target)
        .and_then(|l| m.material(l.material))
        .map_or_else(|| format!("capa {}", n + 1), |x| x.name.clone())
}

/// The accent a face composed by hand starts with: its material's, or a
/// wood one.
fn usual_accent(
    m: &archisim_core::Model,
    target: SketchTarget,
) -> Option<archisim_core::MaterialId> {
    let mat = sketch_layer(m, target).map(|l| l.material)?;
    m.board_layout(mat)
        .and_then(|l| l.accent.as_ref().map(|a| a.material))
        .or_else(|| {
            m.materials()
                .find(|x| x.id != mat && x.name.to_lowercase().contains("madera"))
                .map(|x| x.id)
        })
}

impl ArchiSimApp {
    /// The pieces of a face composed by hand as its lines cut it now, in
    /// the face's own axes (not mirrored).
    fn live_pieces(&self, st: &SketchState) -> Vec<Vec<(f64, f64)>> {
        if let SketchTarget::BoardPlan(host, _) = st.sketch.target {
            return archisim_geom::boards::plan_pieces(
                self.ed.model(),
                host,
                &self.live_joints(st),
            );
        }
        let SketchTarget::BoardFace(w, _) = st.sketch.target else {
            return Vec::new();
        };
        let Some(wall) = self.ed.model().wall(w) else {
            return Vec::new();
        };
        let face = [
            0.0,
            wall.base_offset,
            wall.length(),
            wall.base_offset + wall.height,
        ];
        archisim_geom::boards::face_pieces(face, &self.live_joints(st))
    }

    /// The joints of a face composed by hand as they are drawn now, in the
    /// face's own axes (not mirrored).
    pub(super) fn live_joints(&self, st: &SketchState) -> Vec<(Point2, Point2)> {
        self.sketch_segs(st)
            .iter()
            .flat_map(|s| {
                let pts = match s {
                    archisim_draft::Seg::Line { a, b } => vec![*a, *b],
                    _ => s.tessellate(0.02),
                };
                let pts: Vec<Point2> = pts.into_iter().map(|p| mirrored(st.mirror, p)).collect();
                pts.windows(2).map(|p| (p[0], p[1])).collect::<Vec<_>>()
            })
            .collect()
    }

    /// What the face being composed warns of now.
    pub(super) fn live_warnings(&self) -> Vec<archisim_geom::boards::FaceWarning> {
        let Some(st) = self.sketch.as_ref() else {
            return Vec::new();
        };
        if let SketchTarget::BoardPlan(host, layer) = st.sketch.target {
            return archisim_geom::boards::plan_warnings(
                self.ed.model(),
                host,
                layer,
                &self.live_joints(st),
                &st.sketch.marks,
            );
        }
        let SketchTarget::BoardFace(w, layer) = st.sketch.target else {
            return Vec::new();
        };
        archisim_geom::boards::face_warnings(
            self.ed.model(),
            w,
            layer,
            &self.live_joints(st),
            &st.sketch.marks,
        )
    }

    /// What a click on a piece does now: on, off or another.
    pub(super) fn set_piece_mode(&mut self, mode: PieceMode) {
        self.end_command(true);
        if let Some(st) = self.sketch.as_mut() {
            st.pick = false;
            st.piece = if st.piece == Some(mode) {
                None
            } else {
                Some(mode)
            };
        }
    }

    /// Whether a click in the drawing marks a piece of a face composed by
    /// hand.
    pub(super) fn sketch_marking(&self) -> bool {
        self.tool == Tool::Arrow && self.sketch.as_ref().is_some_and(|s| s.piece.is_some())
    }

    /// Marks the piece under a point (as drawn): painted, on site, or back
    /// to plain; a second click undoes it.
    pub(super) fn sketch_mark_at(&mut self, p: Point2) {
        self.sketch_mark(p, false);
    }

    /// Marks a piece under a point; `dragging`: as the mouse goes over the
    /// pieces with the button held, each one once and without undoing.
    pub(super) fn sketch_mark(&mut self, p: Point2, dragging: bool) {
        let Some(st) = self.sketch.as_ref() else {
            return;
        };
        let Some(mode) = st.piece else {
            return;
        };
        let q = mirrored(st.mirror, p);
        let pieces = self.live_pieces(st);
        let Some(poly) = pieces
            .into_iter()
            .find(|poly| archisim_geom::boards::inside_piece(poly, (q.x, q.y)))
        else {
            return;
        };
        if mode == PieceMode::Split {
            if !dragging {
                let (n, upright) = st.split;
                self.split_piece(&poly, n, upright);
            }
            return;
        }
        let accent = st.accent;
        if mode == PieceMode::Paint && accent.is_none() {
            self.complain("Elige primero el material en «Pintar con» de la barra de opciones.");
            return;
        }
        let n = poly.len() as f64;
        let centre = Point2::new(
            poly.iter().map(|v| v.0).sum::<f64>() / n,
            poly.iter().map(|v| v.1).sum::<f64>() / n,
        );
        let layer_material = sketch_layer(self.ed.model(), st.sketch.target).map(|l| l.material);
        let Some(st) = self.sketch.as_mut() else {
            return;
        };
        if dragging {
            if st
                .stroke
                .iter()
                .any(|q| archisim_geom::boards::inside_piece(&poly, (q.x, q.y)))
            {
                return;
            }
            st.stroke.push(centre);
        }
        let marks = &mut st.sketch.marks;
        let at = marks
            .iter()
            .position(|m| archisim_geom::boards::inside_piece(&poly, (m.at.x, m.at.y)));
        let mut mark = match at {
            Some(i) => marks.remove(i),
            None => archisim_core::PieceMark {
                at: centre,
                material: None,
                on_site: false,
            },
        };
        mark.at = centre;
        match mode {
            // A click on a piece already of that material gives it back
            // to the layer's; dragging only paints.
            PieceMode::Paint => {
                mark.material = if !dragging && mark.material == accent {
                    None
                } else {
                    accent
                };
                if mark.material == layer_material {
                    mark.material = None;
                }
            }
            PieceMode::OnSite => mark.on_site = dragging || !mark.on_site,
            PieceMode::Plain => {
                mark.material = None;
                mark.on_site = false;
            }
            PieceMode::Split => {}
        }
        if mark.material.is_some() || mark.on_site {
            marks.push(mark);
        }
    }

    /// The pieces of a face composed by hand over its sketch: the accent
    /// ones in wood, the ones on site in orange, and each one's size.
    fn draw_live_pieces(&self, st: &SketchState, painter: &egui::Painter, rect: Rect) {
        let m = self.ed.model();
        // Which layer is cut, over the face.
        if let SketchTarget::BoardFace(w, layer) = st.sketch.target {
            if let Some(wall) = m.wall(w) {
                let rule = m
                    .wall_type(wall.wall_type)
                    .and_then(|t| t.layers.get(layer))
                    .and_then(|l| m.board_layout(l.material))
                    .map_or(0.0, |l| l.joint);
                let joint = st.sketch.joint.unwrap_or(rule);
                let top = wall.base_offset + wall.height;
                let at = self
                    .view
                    .to_screen(rect, Point2::new(wall.length() / 2.0, top));
                painter.text(
                    at + egui::vec2(0.0, -10.0),
                    egui::Align2::CENTER_BOTTOM,
                    format!(
                        "Capa {} · {} · junta {:.0} mm · vista desde fuera",
                        layer + 1,
                        board_layer_name(m, w, layer),
                        joint * 1000.0
                    ),
                    egui::FontId::proportional(13.0),
                    SKETCH,
                );
            }
        }
        for poly in self.live_pieces(st) {
            let mark = st
                .sketch
                .marks
                .iter()
                .find(|k| archisim_geom::boards::inside_piece(&poly, (k.at.x, k.at.y)));
            let pts: Vec<egui::Pos2> = poly
                .iter()
                .map(|v| {
                    self.view
                        .to_screen(rect, mirrored(st.mirror, Point2::new(v.0, v.1)))
                })
                .collect();
            let fill = match mark {
                Some(k) if k.on_site => Some(Color32::from_rgba_unmultiplied(235, 120, 30, 90)),
                Some(k) if k.material.is_some() => {
                    let c = k.material.map_or([160, 110, 70], |x| m.surface_color(x));
                    Some(Color32::from_rgba_unmultiplied(c[0], c[1], c[2], 150))
                }
                _ => None,
            };
            if let Some(f) = fill {
                painter.add(egui::Shape::convex_polygon(pts.clone(), f, Stroke::NONE));
            }
            let x0 = poly.iter().map(|v| v.0).fold(f64::INFINITY, f64::min);
            let x1 = poly.iter().map(|v| v.0).fold(f64::NEG_INFINITY, f64::max);
            let y0 = poly.iter().map(|v| v.1).fold(f64::INFINITY, f64::min);
            let y1 = poly.iter().map(|v| v.1).fold(f64::NEG_INFINITY, f64::max);
            let centre = egui::Pos2::new(
                pts.iter().map(|p| p.x).sum::<f32>() / pts.len() as f32,
                pts.iter().map(|p| p.y).sum::<f32>() / pts.len() as f32,
            );
            let wide = pts.iter().map(|p| p.x).fold(f32::NEG_INFINITY, f32::max)
                - pts.iter().map(|p| p.x).fold(f32::INFINITY, f32::min);
            if wide > 46.0 {
                let mut text = format!("{:.0}×{:.0}", (x1 - x0) * 100.0, (y1 - y0) * 100.0);
                if mark.is_some_and(|k| k.on_site) {
                    text = format!("OBRA\n{text}");
                }
                painter.text(
                    centre,
                    egui::Align2::CENTER_CENTER,
                    text,
                    egui::FontId::proportional(11.0),
                    Color32::from_gray(70),
                );
            }
        }
        // What does not go: pieces too big (red) or too small (amber),
        // joints off a jamb or over the next layer's (dashed).
        use archisim_geom::boards::FaceWarningKind as K;
        for wn in self.live_warnings() {
            let pts: Vec<egui::Pos2> = wn
                .outline
                .iter()
                .map(|v| {
                    self.view
                        .to_screen(rect, mirrored(st.mirror, Point2::new(v.0, v.1)))
                })
                .collect();
            match wn.kind {
                K::TooBig | K::TooSmall => {
                    let c = if wn.kind == K::TooBig {
                        Color32::from_rgb(210, 40, 40)
                    } else {
                        Color32::from_rgb(220, 150, 20)
                    };
                    let mut ring = pts.clone();
                    if let Some(f) = ring.first().copied() {
                        ring.push(f);
                    }
                    painter.add(egui::Shape::line(ring, Stroke::new(2.5, c)));
                }
                K::NearJamb | K::OverJoint => {
                    let c = if wn.kind == K::NearJamb {
                        Color32::from_rgb(220, 150, 20)
                    } else {
                        Color32::from_rgb(210, 40, 40)
                    };
                    painter.extend(egui::Shape::dashed_line(
                        &pts,
                        Stroke::new(3.0, c),
                        8.0,
                        5.0,
                    ));
                }
            }
        }
    }
}

impl ArchiSimApp {
    /// The options bar of a face composed by hand: which layer is being
    /// cut, its joint, and the material it paints with.
    pub(super) fn board_face_bar(&mut self, ui: &mut Ui) {
        let Some(st) = self.sketch.as_ref() else {
            return;
        };
        let (key, layer) = match st.sketch.target {
            SketchTarget::BoardFace(w, layer) => (w.0, layer),
            SketchTarget::BoardPlan(
                archisim_core::PlanHost::Slab(archisim_core::SlabId(i))
                | archisim_core::PlanHost::Ceiling(archisim_core::CeilingId(i)),
                layer,
            ) => (i, layer),
            _ => return,
        };
        let target = st.sketch.target;
        let m = self.ed.model();
        let layer_mat = sketch_layer(m, target).map(|l| (l.material, l.thickness));
        let rule = layer_mat
            .and_then(|(mat, _)| m.board_layout(mat).map(|l| l.joint))
            .unwrap_or(0.0);
        let joint = st.sketch.joint.unwrap_or(rule);
        let mut paint = st.accent.or(layer_mat.map(|l| l.0));
        let title = format!(
            "Despiece a mano · Capa {} · {}{}",
            layer + 1,
            sketch_layer_name(m, target),
            layer_mat.map_or(String::new(), |(_, t)| format!(" ({:.0} mm)", t * 1000.0))
        );
        ui.add_space(6.0);
        ui.label(
            egui::RichText::new(title)
                .font(egui::FontId::proportional(12.5))
                .strong()
                .color(SKETCH),
        );
        ui.add_space(8.0);
        ui.label("Junta (mm)");
        let id = egui::Id::unique(("board-face-joint", key, layer));
        let mut text = ui
            .data(|d| d.get_temp::<String>(id))
            .unwrap_or_else(|| format!("{:.0}", joint * 1000.0));
        let r = ui
            .add(egui::TextEdit::singleline(&mut text).desired_width(34.0))
            .on_hover_text(format!(
                "La junta abierta entre las piezas de esta cara; la de su material es de {:.0} mm",
                rule * 1000.0
            ));
        if r.has_focus() {
            ui.data_mut(|d| d.insert_temp(id, text.clone()));
        } else {
            ui.data_mut(|d| d.remove::<String>(id));
        }
        if r.lost_focus() {
            if let Ok(v) = text.trim().replace(',', ".").parse::<f64>() {
                let v = (v / 1000.0).clamp(0.0, 0.05);
                if let Some(st) = self.sketch.as_mut() {
                    st.sketch.joint = if (v - rule).abs() < 1e-6 {
                        None
                    } else {
                        Some(v)
                    };
                }
            }
        }
        let warnings = self.live_warnings();
        if !warnings.is_empty() {
            ui.add_space(8.0);
            let mut list: Vec<String> = warnings.iter().map(|w| w.text.clone()).collect();
            list.dedup();
            ui.label(
                egui::RichText::new(format!(
                    "{} aviso{}",
                    warnings.len(),
                    if warnings.len() == 1 { "" } else { "s" }
                ))
                .strong()
                .color(Color32::from_rgb(200, 60, 30)),
            )
            .on_hover_text(list.join("\n"));
        }
        self.split_fields(ui);
        ui.add_space(8.0);
        ui.label("Pintar con");
        if let Some(cur) = paint.as_mut() {
            let before = *cur;
            super::cards::material_button(
                ui,
                egui::Id::unique("board-face-paint"),
                self.ed.model(),
                cur,
                170.0,
            );
            let chosen = *cur;
            if let Some(st) = self.sketch.as_mut() {
                if st.accent != Some(chosen) {
                    st.accent = Some(chosen);
                }
                if chosen != before {
                    // Choosing a material is meant to paint with it.
                    st.piece = Some(PieceMode::Paint);
                }
            }
        }
    }
}

impl ArchiSimApp {
    /// Splits a piece of the face being composed into `n` equal ones: the
    /// joint lines across it, drawn as sketch lines.
    fn split_piece(&mut self, poly: &[(f64, f64)], n: usize, upright: bool) {
        let Some(st) = self.sketch.as_ref() else {
            return;
        };
        let (layer, view, mirror) = (st.layer, st.view, st.mirror);
        let n = n.clamp(2, 50);
        let x0 = poly.iter().map(|q| q.0).fold(f64::INFINITY, f64::min);
        let x1 = poly.iter().map(|q| q.0).fold(f64::NEG_INFINITY, f64::max);
        let y0 = poly.iter().map(|q| q.1).fold(f64::INFINITY, f64::min);
        let y1 = poly.iter().map(|q| q.1).fold(f64::NEG_INFINITY, f64::max);
        let lines: Vec<(Point2, Point2)> = (1..n)
            .map(|k| {
                let f = k as f64 / n as f64;
                if upright {
                    let x = x0 + (x1 - x0) * f;
                    (Point2::new(x, y0), Point2::new(x, y1))
                } else {
                    let y = y0 + (y1 - y0) * f;
                    (Point2::new(x0, y), Point2::new(x1, y))
                }
            })
            .collect();
        let r = self.ed.transact("Repartir la pieza", |m| {
            for (a, b) in &lines {
                m.add_draft(NewDraft::new(
                    view,
                    layer,
                    Shape::Line {
                        a: mirrored(mirror, *a),
                        b: mirrored(mirror, *b),
                    },
                ))?;
            }
            Ok(())
        });
        if let Err(e) = r {
            self.complain(format!("No se puede repartir: {e}."));
        }
    }

    /// Marks on site, at once, the pieces at the ends of the wall that meet
    /// a joint between boxes.
    pub(super) fn joint_pieces_on_site(&mut self) {
        let Some(st) = self.sketch.as_ref() else {
            return;
        };
        let SketchTarget::BoardFace(w, _) = st.sketch.target else {
            return;
        };
        let joints = self.live_joints(st);
        let points = archisim_geom::boards::joint_end_pieces(self.ed.model(), w, &joints);
        if points.is_empty() {
            self.say("Este muro no llega a ninguna junta entre cajas.");
            return;
        }
        let pieces = self.live_pieces(st);
        let Some(st) = self.sketch.as_mut() else {
            return;
        };
        let mut n = 0;
        for p in points {
            let Some(poly) = pieces
                .iter()
                .find(|poly| archisim_geom::boards::inside_convex(poly, (p.x, p.y)))
            else {
                continue;
            };
            match st
                .sketch
                .marks
                .iter_mut()
                .find(|k| archisim_geom::boards::inside_convex(poly, (k.at.x, k.at.y)))
            {
                Some(k) => k.on_site = true,
                None => st.sketch.marks.push(archisim_core::PieceMark {
                    at: p,
                    material: None,
                    on_site: true,
                }),
            }
            n += 1;
        }
        self.say(format!(
            "{n} pieza{} junto a las juntas entre cajas, para colocar en obra.",
            if n == 1 { "" } else { "s" }
        ));
    }

    /// «Repartir» in the options bar: into how many and which way.
    pub(super) fn split_fields(&mut self, ui: &mut Ui) {
        let Some(st) = self.sketch.as_mut() else {
            return;
        };
        if st.piece != Some(PieceMode::Split) {
            return;
        }
        ui.add_space(8.0);
        ui.label("Repartir en");
        let mut n = st.split.0 as u32;
        ui.add(egui::DragValue::new(&mut n).range(2..=50));
        st.split.0 = n as usize;
        ui.selectable_value(&mut st.split.1, true, "columnas");
        ui.selectable_value(&mut st.split.1, false, "filas");
    }
}
