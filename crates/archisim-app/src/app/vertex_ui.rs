//! Vertices of lines, polylines, fills and slabs, as in ArchiCAD: a double
//! click on an edge of the selection adds a vertex there (ready to drag), a
//! click on a vertex picks it, and Supr takes the picked vertex away instead
//! of the whole element.

use archisim_core::{Point2, Shape};

use super::*;
use crate::drag::Grip;

/// A place for a new vertex: how far from the click, the vertex's handle,
/// and the new shape (of a line or fill) or outline (of a slab).
type Candidate = (f64, Grip, Option<Shape>, Option<Vec<Point2>>);

impl ArchiSimApp {
    /// The picked vertex, if its element is still selected and still has
    /// it.
    pub(super) fn picked_vertex(&self) -> Option<Grip> {
        let g = self.vertex?;
        let model = self.ed.model();
        let ok = match g {
            Grip::DraftPoint { draft, index } => {
                self.ed.selection.contains(&Element::Draft(draft))
                    && model
                        .draft(draft)
                        .is_some_and(|d| archisim_draft::grips::is_vertex(&d.shape, index))
            }
            Grip::SlabCorner { slab, index } => {
                self.ed.selection.contains(&Element::Slab(slab))
                    && model.slab(slab).is_some_and(|s| index < s.outline.len())
            }
            _ => false,
        };
        ok.then_some(g)
    }

    /// A click on a handle of the selection: picks it if it is a vertex.
    /// Returns whether the click was used.
    pub(super) fn vertex_click(&mut self) -> bool {
        let model = self.ed.model();
        let vertex = match self.hover_grip {
            Some(g @ Grip::DraftPoint { draft, index }) => model
                .draft(draft)
                .is_some_and(|d| archisim_draft::grips::is_vertex(&d.shape, index))
                .then_some(g),
            Some(g @ Grip::SlabCorner { .. }) => Some(g),
            _ => None,
        };
        self.vertex = vertex;
        if vertex.is_some() {
            self.say("Vértice elegido: Supr lo quita; arrástrelo para moverlo.");
        }
        vertex.is_some()
    }

    /// A double click at `at`: a new vertex on the nearest edge of a
    /// selected line, polyline, fill or slab. Returns whether one was added.
    pub(super) fn vertex_double_click(&mut self, at: Point2) -> bool {
        let reach = self.view.to_metres(REACH);
        let model = self.ed.model();
        let mut best: Option<Candidate> = None;
        for e in &self.ed.selection {
            match *e {
                Element::Draft(id) => {
                    let Some(d) = model.draft(id) else { continue };
                    if let Some((shape, index)) =
                        archisim_draft::grips::insert_vertex(&d.shape, at, reach)
                    {
                        let dist = archisim_draft::distance(&d.shape, at, crate::pick::paper_k());
                        if best.as_ref().is_none_or(|b| dist < b.0) {
                            best = Some((
                                dist,
                                Grip::DraftPoint { draft: id, index },
                                Some(shape),
                                None,
                            ));
                        }
                    }
                }
                Element::Slab(id) => {
                    let Some(s) = model.slab(id) else { continue };
                    if let Some((outline, index, dist)) = with_corner(&s.outline, at) {
                        if dist <= reach && best.as_ref().is_none_or(|b| dist < b.0) {
                            best = Some((
                                dist,
                                Grip::SlabCorner { slab: id, index },
                                None,
                                Some(outline),
                            ));
                        }
                    }
                }
                _ => {}
            }
        }
        let Some((_, grip, shape, outline)) = best else {
            return false;
        };
        let r = match (grip, shape, outline) {
            (Grip::DraftPoint { draft, .. }, Some(shape), _) => {
                self.ed.set_draft_shape(draft, shape, "Añadir vértice")
            }
            (Grip::SlabCorner { slab, .. }, _, Some(outline)) => {
                self.ed.transact("Añadir esquina de forjado", |m| {
                    m.set_slab_outline(slab, outline)
                })
            }
            _ => return false,
        };
        match r {
            Ok(()) => {
                self.vertex = Some(grip);
                self.say("Vértice añadido: arrástrelo para moverlo; Supr lo quita.");
            }
            Err(e) => self.complain(format!("No se puede añadir el vértice: {e}.")),
        }
        true
    }

    /// Supr with a vertex picked: takes it away. Returns whether it did.
    pub(super) fn delete_vertex(&mut self) -> bool {
        let Some(g) = self.picked_vertex() else {
            return false;
        };
        let model = self.ed.model();
        let r = match g {
            Grip::DraftPoint { draft, index } => {
                let Some(shape) = model
                    .draft(draft)
                    .and_then(|d| archisim_draft::grips::remove_vertex(&d.shape, index))
                else {
                    self.complain("No se puede quitar: le quedarían muy pocos vértices.");
                    return true;
                };
                self.ed.set_draft_shape(draft, shape, "Quitar vértice")
            }
            Grip::SlabCorner { slab, index } => {
                let Some(mut outline) = model.slab(slab).map(|s| s.outline.clone()) else {
                    return false;
                };
                if outline.len() <= 3 {
                    self.complain(
                        "No se puede quitar: el forjado necesita al menos tres esquinas.",
                    );
                    return true;
                }
                outline.remove(index);
                self.ed.transact("Quitar esquina de forjado", |m| {
                    m.set_slab_outline(slab, outline)
                })
            }
            _ => return false,
        };
        self.vertex = None;
        match r {
            Ok(()) => self.say("Vértice quitado."),
            Err(e) => self.complain(format!("No se puede quitar el vértice: {e}.")),
        }
        true
    }
}

/// A closed outline with a new corner on its side nearest to `at`: the
/// outline, the new corner's index and how far `at` was from that side.
/// `None` when `at` is on a corner.
fn with_corner(outline: &[Point2], at: Point2) -> Option<(Vec<Point2>, usize, f64)> {
    let n = outline.len();
    let (i, q, d) = (0..n)
        .map(|i| {
            let (a, b) = (outline[i], outline[(i + 1) % n]);
            let ab = b - a;
            let l2 = ab.dot(ab);
            let t = if l2 < 1e-18 {
                0.0
            } else {
                ((at - a).dot(ab) / l2).clamp(0.0, 1.0)
            };
            let q = a + ab * t;
            (i, q, q.distance(at))
        })
        .min_by(|x, y| x.2.total_cmp(&y.2))?;
    if outline.iter().any(|c| c.distance(q) < 1e-6) {
        return None;
    }
    let mut out = outline.to_vec();
    out.insert(i + 1, q);
    Some((out, i + 1, d))
}
