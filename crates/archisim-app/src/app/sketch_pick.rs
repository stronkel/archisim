//! «Elegir línea» of the sketch, as Revit's «Seleccionar líneas» and
//! «Seleccionar muros» (`docs/INTERFAZ.md` §13.6): what is under the cursor
//! is shown before clicking, Tab goes through what is there, the faces of
//! walls are whole (from end to end, or their core with «Extender en muro
//! (al núcleo)»), a «Desfase» moves the line towards the cursor, and the
//! picked lines trim or extend to meet the sketch lines whose ends are near,
//! so that picking the faces of a room closes its contour.

use archisim_core::{DrawLayerId, LayerFunction, Point2, Shape, ViewRef, WallId};

use crate::pick::Element;
use eframe::egui::{self, Color32, Pos2, Rect, Stroke};

use super::ArchiSimApp;
use crate::spline::PickedLine;

/// How far apart the ends of two sketch lines may be and still be joined
/// at their corner when one of them is picked (a wall's thickness and some
/// more).
const CORNER_REACH: f64 = 1.0;

/// The colour of the line that a click would take.
const PREVIEW: Color32 = Color32::from_rgb(30, 110, 230);

/// What «Elegir línea» does with the line it takes.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PickSettings {
    /// Moved this much towards the side of the cursor (Revit's «Desfase»).
    pub offset: f64,
    /// A wall gives the face of its core, not of its finishes (Revit's
    /// «Extender en muro (al núcleo)»).
    pub core: bool,
}

impl Default for PickSettings {
    fn default() -> Self {
        PickSettings {
            offset: 0.0,
            core: false,
        }
    }
}

/// A line «Elegir línea» could take.
#[derive(Debug, Clone)]
pub struct Candidate {
    pub line: PickedLine,
    /// Its distance from the cursor.
    pub distance: f64,
    /// What it is, for the options bar.
    pub what: &'static str,
}

/// The distance from `p` to the segment `a`–`b`.
fn segment_distance(p: Point2, a: Point2, b: Point2) -> f64 {
    let ab = b - a;
    let len2 = ab.dot(ab);
    if len2 < 1e-18 {
        return p.distance(a);
    }
    let t = ((p - a).dot(ab) / len2).clamp(0.0, 1.0);
    p.distance(a + ab * t)
}

fn straight(a: Point2, b: Point2) -> PickedLine {
    PickedLine {
        pieces: vec![(a, b, 0.0)],
        closed: false,
        points: vec![a, b],
    }
}

/// The distance from `p` to a picked line.
fn line_distance(l: &PickedLine, p: Point2) -> f64 {
    l.pieces
        .iter()
        .map(|(a, b, bulge)| {
            let seg = archisim_draft::Seg::from_bulge(*a, *b, *bulge);
            let pts = seg.tessellate(0.02);
            pts.windows(2)
                .map(|w| segment_distance(p, w[0], w[1]))
                .fold(f64::INFINITY, f64::min)
        })
        .fold(f64::INFINITY, f64::min)
}

impl ArchiSimApp {
    /// Everything «Elegir línea» could take within `reach` of `p`, nearest
    /// first: the lines of the 2D drawing in `view` (not on `skip`) and, in
    /// a plan, the faces of the walls (whole, or of their core), the edges
    /// of slabs, roofs, columns, beams and footings, and grids and
    /// reference planes. Hidden elements (the one being sketched) are left
    /// out.
    pub(super) fn line_candidates(
        &self,
        p: Point2,
        reach: f64,
        view: ViewRef,
        skip: Option<DrawLayerId>,
        core: bool,
    ) -> Vec<Candidate> {
        let model = self.ed.model();
        let mut out: Vec<Candidate> = Vec::new();
        let mut take = |line: PickedLine, what: &'static str| {
            let distance = line_distance(&line, p);
            if distance <= reach {
                out.push(Candidate {
                    line,
                    distance,
                    what,
                });
            }
        };
        for d in model.drafts_in(view) {
            if Some(d.layer) == skip {
                continue;
            }
            if let Some(l) = crate::spline::picked_line(&d.shape) {
                take(l, "Línea del dibujo");
            }
        }
        if let ViewRef::Level(level) = view {
            let hidden = self.ed.hidden();
            for e in crate::pick::elements_on_level(model, level) {
                if hidden.contains(&e) {
                    continue;
                }
                match e {
                    Element::Wall(id) => {
                        for (line, what) in self.wall_face_lines(id, core) {
                            take(line, what);
                        }
                    }
                    Element::Grid(id) => {
                        if let Some(g) = model.grid(id) {
                            take(straight(g.start, g.end), "Eje");
                        }
                    }
                    Element::RefPlane(id) => {
                        if let Some(r) = model.ref_plane(id) {
                            take(straight(r.start, r.end), "Plano de referencia");
                        }
                    }
                    Element::Slab(_)
                    | Element::Roof(_)
                    | Element::Column(_)
                    | Element::Beam(_)
                    | Element::Footing(_) => {
                        let what = match e {
                            Element::Slab(_) => "Borde de forjado",
                            Element::Roof(_) => "Borde de cubierta",
                            Element::Column(_) => "Cara de pilar",
                            Element::Beam(_) => "Cara de viga",
                            _ => "Borde de zapata",
                        };
                        let Some(o) = crate::pick::outline(&self.ed, e) else {
                            continue;
                        };
                        let n = o.len();
                        for i in 0..n {
                            let (a, b) = (o[i], o[(i + 1) % n]);
                            if a.distance(b) > 1e-6 {
                                take(straight(a, b), what);
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
        out.sort_by(|a, b| a.distance.total_cmp(&b.distance));
        out
    }

    /// The two faces of a wall from end to end, as Revit picks them: the
    /// finish faces, or with `core` the faces of its structure.
    fn wall_face_lines(&self, id: WallId, core: bool) -> Vec<(PickedLine, &'static str)> {
        let model = self.ed.model();
        let Some(w) = model.wall(id) else {
            return Vec::new();
        };
        let Some(t) = model.wall_type(w.wall_type) else {
            return Vec::new();
        };
        let thickness = t.thickness();
        let (ext, int) = w.face_offsets(thickness);
        // The finishes outside and inside the core.
        let (mut out_finish, mut in_finish) = (0.0, 0.0);
        if core {
            let structure: Vec<usize> = t
                .layers
                .iter()
                .enumerate()
                .filter(|(_, l)| l.function == LayerFunction::Structure)
                .map(|(i, _)| i)
                .collect();
            if let (Some(&first), Some(&last)) = (structure.first(), structure.last()) {
                out_finish = t.layers[..first].iter().map(|l| l.thickness).sum();
                in_finish = t.layers[last + 1..].iter().map(|l| l.thickness).sum();
            }
        }
        let face = |off: f64| {
            let (a, b) = (w.point_at(0.0, off), w.point_at(1.0, off));
            PickedLine {
                pieces: vec![(a, b, w.bulge)],
                closed: false,
                points: vec![a, b],
            }
        };
        let (what_ext, what_int) = if core {
            ("Núcleo del muro (exterior)", "Núcleo del muro (interior)")
        } else {
            ("Cara exterior del muro", "Cara interior del muro")
        };
        vec![
            (face(ext - out_finish), what_ext),
            (face(int + in_finish), what_int),
        ]
    }

    /// The line a click would take now: the nearest, or the one Tab has
    /// gone to.
    pub(super) fn sketch_candidate(&self, p: Point2) -> Option<(Candidate, usize)> {
        let st = self.sketch.as_ref()?;
        let reach = self.view.to_metres(super::REACH * 1.4);
        let all = self.line_candidates(p, reach, st.view, Some(st.layer), self.pick_settings.core);
        if all.is_empty() {
            return None;
        }
        let n = all.len();
        let i = st.pick_cycle % n;
        all.into_iter().nth(i).map(|c| (c, n))
    }

    /// The point under the mouse in the drawing, not snapped.
    pub(super) fn pointer_point(&self, ctx: &egui::Context, rect: Rect) -> Option<Point2> {
        let pos = ctx.input(|i| i.pointer.hover_pos())?;
        rect.contains(pos).then(|| self.view.to_model(rect, pos))
    }

    /// The line a click would take, drawn over the sketch in blue.
    pub(super) fn draw_sketch_candidate(
        &mut self,
        ctx: &egui::Context,
        painter: &egui::Painter,
        rect: Rect,
    ) {
        if !self.sketch_picking() {
            return;
        }
        self.pick_hint.clear();
        let Some(p) = self.pointer_point(ctx, rect) else {
            return;
        };
        // Moving the mouse starts again from the nearest.
        if let Some(st) = self.sketch.as_mut() {
            if st
                .pick_at
                .is_none_or(|q| q.distance(p) > self.view.to_metres(6.0))
            {
                st.pick_at = Some(p);
                st.pick_cycle = 0;
            }
        }
        let Some((c, n)) = self.sketch_candidate(p) else {
            return;
        };
        let line = self.offset_line(&c.line, p);
        for (a, b, bulge) in &line.pieces {
            let pts: Vec<Pos2> = archisim_draft::Seg::from_bulge(*a, *b, *bulge)
                .tessellate(self.view.to_metres(2.0).max(0.005))
                .into_iter()
                .map(|q| self.view.to_screen(rect, q))
                .collect();
            painter.add(egui::Shape::line(
                pts,
                Stroke::new(4.0, PREVIEW.gamma_multiply(0.45)),
            ));
        }
        let more = if n > 1 {
            format!(
                " · {} de {} (Tab: la siguiente)",
                self.sketch.as_ref().map_or(0, |s| s.pick_cycle % n) + 1,
                n
            )
        } else {
            String::new()
        };
        self.pick_hint = format!("{}{}", c.what, more);
    }

    /// The line moved by «Desfase» towards the side of `p`.
    fn offset_line(&self, l: &PickedLine, p: Point2) -> PickedLine {
        let d = self.pick_settings.offset;
        if d.abs() < 1e-9 {
            return l.clone();
        }
        let pieces = l
            .pieces
            .iter()
            .map(|(a, b, bulge)| {
                if bulge.abs() > 1e-9 {
                    return (*a, *b, *bulge);
                }
                let dir = (*b - *a).normalized().unwrap_or_default();
                let n = dir.perp_left();
                let side = if (p - *a).dot(n) >= 0.0 { 1.0 } else { -1.0 };
                (*a + n * (d * side), *b + n * (d * side), 0.0)
            })
            .collect::<Vec<_>>();
        let points = pieces
            .iter()
            .map(|(a, _, _)| *a)
            .chain(pieces.last().map(|(_, b, _)| *b))
            .collect();
        PickedLine {
            pieces,
            closed: l.closed,
            points,
        }
    }

    /// Tab while picking: the next line under the cursor.
    pub(super) fn next_sketch_candidate(&mut self) {
        if let Some(st) = self.sketch.as_mut() {
            st.pick_cycle += 1;
        }
    }

    /// A click with «Elegir línea»: the line becomes a sketch line, and
    /// the sketch lines whose ends are near meet it at their corners.
    pub(super) fn sketch_pick_at(&mut self, p: Point2) {
        let Some(st) = self.sketch.as_ref() else {
            return;
        };
        let (view, layer) = (st.view, st.layer);
        let Some((c, _)) = self.sketch_candidate(p) else {
            self.complain("Ahí no hay ninguna línea ni cara que elegir.");
            return;
        };
        let line = self.offset_line(&c.line, p);
        let mut pieces = line.pieces.clone();
        // The corners with the sketch lines already there.
        let existing: Vec<(archisim_core::DraftId, Point2, Point2)> = self
            .ed
            .model()
            .drafts_in(view)
            .filter(|d| d.layer == layer)
            .filter_map(|d| match d.shape {
                Shape::Line { a, b } => Some((d.id, a, b)),
                _ => None,
            })
            .collect();
        let mut moved: Vec<(archisim_core::DraftId, Point2, Point2)> = Vec::new();
        if let [(a, b, bulge)] = pieces.as_mut_slice() {
            if bulge.abs() < 1e-9 {
                for (id, pa, pb) in &existing {
                    let Some(x) = line_crossing(*a, *b, *pa, *pb) else {
                        continue;
                    };
                    let near_new = a.distance(x).min(b.distance(x));
                    let near_old = pa.distance(x).min(pb.distance(x));
                    if near_new > CORNER_REACH || near_old > CORNER_REACH {
                        continue;
                    }
                    if a.distance(x) < b.distance(x) {
                        *a = x;
                    } else {
                        *b = x;
                    }
                    let (na, nb) = if pa.distance(x) < pb.distance(x) {
                        (x, *pb)
                    } else {
                        (*pa, x)
                    };
                    moved.push((*id, na, nb));
                }
            }
        }
        let r = self.ed.transact("Elegir línea", |m| {
            for (id, a, b) in &moved {
                if a.distance(*b) > 1e-6 {
                    m.set_draft_shape(*id, Shape::Line { a: *a, b: *b })?;
                }
            }
            for (a, b, bulge) in &pieces {
                if a.distance(*b) < 1e-6 {
                    continue;
                }
                let seg = archisim_draft::Seg::from_bulge(*a, *b, *bulge);
                m.add_draft(archisim_core::NewDraft::new(
                    view,
                    layer,
                    archisim_draft::seg_shape(&seg),
                ))?;
            }
            Ok(())
        });
        if let Err(e) = r {
            self.complain(format!("No se puede: {e}."));
        }
        if let Some(st) = self.sketch.as_mut() {
            st.pick_cycle = 0;
        }
    }

    /// Desfase and «Extender en muro (al núcleo)» of «Elegir línea», in the
    /// options bar.
    pub(super) fn pick_fields(&mut self, ui: &mut egui::Ui) {
        if !self.sketch_picking() {
            return;
        }
        let p = super::theme::Palette::current(ui.ctx());
        // What a click takes, first: it is what is looked at.
        let hint = if self.pick_hint.is_empty() {
            "Pasa el ratón por una línea o una cara".to_string()
        } else {
            self.pick_hint.clone()
        };
        ui.label(egui::RichText::new(hint).small().strong().color(PREVIEW));
        ui.add_space(8.0);
        ui.label(egui::RichText::new("Desfase").small().color(p.muted))
            .on_hover_text("La línea elegida se aparta esto hacia el lado del cursor");
        if let Some(v) = super::text_edit_committed(
            ui,
            egui::Id::unique("pick-offset"),
            &crate::units::fmt_len(self.pick_settings.offset),
            56.0,
        )
        .and_then(|t| crate::units::parse_length(&t))
        {
            self.pick_settings.offset = v.abs();
        }
        use super::widgets::SwitchExt;
        ui.add_space(4.0);
        ui.switch(&mut self.pick_settings.core, "Al núcleo").on_hover_text(
            "Extender en muro (al núcleo): de un muro se toma la cara de su capa de estructura, no la del acabado",
        );
    }
}

/// Where the lines through `a`–`b` and `c`–`d` cross; `None` if they are
/// parallel.
fn line_crossing(a: Point2, b: Point2, c: Point2, d: Point2) -> Option<Point2> {
    let r = b - a;
    let s = d - c;
    let den = r.x * s.y - r.y * s.x;
    if den.abs() < 1e-9 * r.length().max(1e-9) * s.length().max(1e-9) {
        return None;
    }
    let t = ((c - a).x * s.y - (c - a).y * s.x) / den;
    Some(a + r * t)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn crossing_of_two_lines() {
        let x = line_crossing(
            Point2::new(0.0, 0.0),
            Point2::new(4.0, 0.0),
            Point2::new(5.0, -1.0),
            Point2::new(5.0, 3.0),
        )
        .unwrap();
        assert!(x.approx_eq(Point2::new(5.0, 0.0)));
        assert!(line_crossing(
            Point2::new(0.0, 0.0),
            Point2::new(1.0, 0.0),
            Point2::new(0.0, 1.0),
            Point2::new(1.0, 1.0)
        )
        .is_none());
    }
}
