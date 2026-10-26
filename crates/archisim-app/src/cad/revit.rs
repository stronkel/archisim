//! Revit's editing orders for building elements, as the Spanish Revit
//! names them: Alinear (AL), Recortar/extender para formar esquina (TR) and
//! Recortar/extender un elemento o varios elementos hasta un límite.
//!
//! They work by picking, as in Revit, not on a selection made beforehand:
//! the reference or the limit first, then what is aligned, trimmed or
//! extended. Walls are trimmed and extended along their location line, so
//! that the ends meet in a corner (L) or against the limit's line (T); what
//! is aligned is moved as when it is dragged, so the walls joined to it
//! stretch to follow.

use std::collections::BTreeSet;

use archisim_core::{Point2, WallId};

use super::*;

/// A straight line of the plan: a point and its direction.
#[derive(Debug, Clone, Copy, PartialEq)]
struct Line2 {
    a: Point2,
    b: Point2,
}

impl Line2 {
    fn dir(&self) -> Option<Point2> {
        (self.b - self.a).normalized()
    }

    /// Distance from `p` to the segment.
    fn distance(&self, p: Point2) -> f64 {
        crate::pick::nearest_on_segment(self.a, self.b, p).distance(p)
    }

    /// Where the two infinite lines meet; `None` if (nearly) parallel.
    fn meet(&self, other: &Line2) -> Option<Point2> {
        let (u, v) = (self.dir()?, other.dir()?);
        if u.cross(v).abs() < 1e-6 {
            return None;
        }
        archisim_geom::Line::new(self.a, u).intersect(&archisim_geom::Line::new(other.a, v))
    }
}

/// The location line of a straight wall.
fn wall_axis(cx: &Ctx<'_, dyn Geo + '_>, id: WallId) -> Result<Line2, String> {
    let w = cx
        .model()
        .wall(id)
        .ok_or_else(|| "Ese muro ya no existe.".to_string())?;
    if w.is_curved() {
        return Err("Los muros curvos no se recortan ni se extienden todavía.".into());
    }
    Ok(Line2 {
        a: w.start,
        b: w.end,
    })
}

/// The straight lines of an element that can be a reference or a limit:
/// the faces and the location line of a wall, the lines of a 2D drawing,
/// a grid line, a reference plane, and the outline of anything else.
fn element_lines(cx: &Ctx<'_, dyn Geo + '_>, e: Element) -> Vec<Line2> {
    let model = cx.model();
    let mut out = Vec::new();
    let mut ring = |pts: &[Point2]| {
        let n = pts.len();
        for i in 0..n {
            let (a, b) = (pts[i], pts[(i + 1) % n]);
            if a.distance(b) > 1e-6 {
                out.push(Line2 { a, b });
            }
        }
    };
    match e {
        Element::Wall(id) => {
            if let Some(g) = cx.geo.wall_geometry(id) {
                ring(&g.outline);
            }
            if let Some(w) = model.wall(id).filter(|w| !w.is_curved()) {
                out.push(Line2 {
                    a: w.start,
                    b: w.end,
                });
            }
        }
        Element::Draft(id) => {
            if let Some(d) = model.draft(id) {
                for s in archisim_draft::boundary_segs(&d.shape) {
                    if let archisim_draft::Seg::Line { a, b } = s {
                        out.push(Line2 { a, b });
                    }
                }
            }
        }
        Element::Grid(id) => {
            if let Some(g) = model.grid(id) {
                out.push(Line2 {
                    a: g.start,
                    b: g.end,
                });
            }
        }
        Element::RefPlane(id) => {
            if let Some(r) = model.ref_plane(id) {
                out.push(Line2 {
                    a: r.start,
                    b: r.end,
                });
            }
        }
        other => {
            if let Some(o) = crate::pick::outline(cx.geo, other) {
                ring(&o);
            }
        }
    }
    out
}

/// The line of an element nearest to where it was picked.
fn nearest_line(cx: &Ctx<'_, dyn Geo + '_>, e: Element, at: Point2) -> Option<Line2> {
    element_lines(cx, e)
        .into_iter()
        .min_by(|l, m| l.distance(at).total_cmp(&m.distance(at)))
}

/// The limit a wall is trimmed or extended to: the location line of a
/// wall, a grid line, a reference plane or a 2D line.
fn limit_line(cx: &Ctx<'_, dyn Geo + '_>, e: Element, at: Point2) -> Option<Line2> {
    match e {
        Element::Wall(id) => wall_axis(cx, id).ok(),
        _ => nearest_line(cx, e, at),
    }
}

/// New ends of a wall so that its location line reaches `p`: an end beyond
/// `p` is cut back, keeping the part where the wall was picked (`at`); if
/// `p` lies beyond an end, that end is extended to it.
fn ends_to(axis: &Line2, p: Point2, at: Point2) -> Option<(Point2, Point2)> {
    let v = axis.b - axis.a;
    let len2 = v.dot(v);
    if len2 < 1e-12 {
        return None;
    }
    let t = (p - axis.a).dot(v) / len2;
    let t_at = (at - axis.a).dot(v) / len2;
    // Beyond the start: the start comes to it; beyond the end, or cutting
    // with the picked part before it: the end does.
    let ends = if t <= 0.0 || (t < 1.0 && t_at >= t) {
        (p, axis.b)
    } else {
        (axis.a, p)
    };
    // A wall shorter than 1 cm would be left: refused.
    (ends.0.distance(ends.1) > 0.01).then_some(ends)
}

// ---- Recortar/extender para formar esquina ---------------------------------

/// Two walls trimmed or extended until their location lines meet: the
/// corner closes, each keeping the part where it was picked.
#[derive(Default)]
pub struct Corner {
    first: Option<(WallId, Point2)>,
}

impl Command for Corner {
    fn name(&self) -> &'static str {
        "ESQUINA"
    }

    fn prompt(&self) -> Prompt {
        Prompt::new(
            if self.first.is_none() {
                "Designe el primer muro, por la parte que se queda"
            } else {
                "Designe el segundo muro, por la parte que se queda"
            },
            Want::Pick,
        )
    }

    fn feed(&mut self, input: Input, cx: &Ctx<'_, dyn Geo + '_>, _st: &mut CadSettings) -> Step {
        match input {
            Input::Pick(Element::Wall(id), at) => {
                let axis = match wall_axis(cx, id) {
                    Ok(a) => a,
                    Err(e) => return Step::Refuse(e),
                };
                let Some((first, first_at)) = self.first else {
                    self.first = Some((id, at));
                    return Step::Next;
                };
                if first == id {
                    return Step::Refuse("Designe otro muro.".into());
                }
                let a = match wall_axis(cx, first) {
                    Ok(a) => a,
                    Err(e) => return Step::Refuse(e),
                };
                let Some(p) = a.meet(&axis) else {
                    return Step::Refuse("Los dos muros son paralelos: no forman esquina.".into());
                };
                let (Some(ea), Some(eb)) = (ends_to(&a, p, first_at), ends_to(&axis, p, at)) else {
                    return Step::Refuse("Uno de los muros se quedaría sin longitud.".into());
                };
                self.first = None;
                Step::Apply(Changes {
                    label: "Recortar/extender a esquina",
                    changes: vec![
                        Change::WallEnds(first, ea.0, ea.1),
                        Change::WallEnds(id, eb.0, eb.1),
                    ],
                })
            }
            Input::Pick(..) => Step::Refuse("Designe un muro.".into()),
            Input::Enter => Step::Done(None),
            _ => Step::Next,
        }
    }

    fn picked(&self) -> Vec<Element> {
        self.first
            .map(|(id, _)| Element::Wall(id))
            .into_iter()
            .collect()
    }
}

// ---- Recortar/extender un elemento o varios ---------------------------------

/// Walls trimmed or extended to a limit: the location line of a wall, a
/// grid line, a reference plane or a 2D line. With `many`, every wall picked
/// goes to the same limit until Enter; otherwise, after each wall a new
/// limit is asked for (Revit's "un elemento").
pub struct ToLimit {
    many: bool,
    limit: Option<(Element, Line2)>,
}

impl ToLimit {
    pub fn new(many: bool) -> Self {
        Self { many, limit: None }
    }
}

impl Command for ToLimit {
    fn name(&self) -> &'static str {
        if self.many {
            "LIMITE"
        } else {
            "LIMITE1"
        }
    }

    fn prompt(&self) -> Prompt {
        Prompt::new(
            match (self.limit.is_some(), self.many) {
                (false, _) => "Designe el límite (muro, eje, plano de referencia o línea)",
                (true, false) => "Designe el muro que se recorta o extiende, por la parte que se queda",
                (true, true) => {
                    "Designe los muros que se recortan o extienden, por la parte que se queda (Intro para terminar)"
                }
            },
            Want::Pick,
        )
    }

    fn feed(&mut self, input: Input, cx: &Ctx<'_, dyn Geo + '_>, _st: &mut CadSettings) -> Step {
        match input {
            Input::Pick(e, at) if self.limit.is_none() => match limit_line(cx, e, at) {
                Some(l) => {
                    self.limit = Some((e, l));
                    Step::Next
                }
                None => Step::Refuse(
                    "Ese elemento no tiene una línea recta que sirva de límite.".into(),
                ),
            },
            Input::Pick(Element::Wall(id), at) => {
                let (limit_el, limit) = self.limit.unwrap();
                if limit_el == Element::Wall(id) {
                    return Step::Refuse("Ese muro es el límite.".into());
                }
                let axis = match wall_axis(cx, id) {
                    Ok(a) => a,
                    Err(e) => return Step::Refuse(e),
                };
                let Some(p) = axis.meet(&limit) else {
                    return Step::Refuse("El muro es paralelo al límite.".into());
                };
                let Some((s, e)) = ends_to(&axis, p, at) else {
                    return Step::Refuse("El muro se quedaría sin longitud.".into());
                };
                if !self.many {
                    self.limit = None;
                }
                Step::Apply(Changes {
                    label: "Recortar/extender",
                    changes: vec![Change::WallEnds(id, s, e)],
                })
            }
            Input::Pick(..) => Step::Refuse("Solo se recortan y extienden muros.".into()),
            Input::Enter => Step::Done(None),
            _ => Step::Next,
        }
    }

    fn picked(&self) -> Vec<Element> {
        self.limit.map(|(e, _)| e).into_iter().collect()
    }
}

// ---- Alinear ------------------------------------------------------------------

/// Revit's align: a reference line first (a face or the location line of a
/// wall, a 2D line, a grid, a reference plane, an edge of anything), then
/// the line of another element to bring onto it. The element moves
/// across, perpendicular to the reference; it must be parallel to it.
#[derive(Default)]
pub struct AlignTo {
    reference: Option<(Element, Line2)>,
}

/// Lines closer than this angle (radians) are taken as parallel.
const PARALLEL: f64 = 0.01;

impl Command for AlignTo {
    fn name(&self) -> &'static str {
        "ALINEACARA"
    }

    fn prompt(&self) -> Prompt {
        Prompt::new(
            if self.reference.is_none() {
                "Designe la línea de referencia (cara de muro, eje, línea…)"
            } else {
                "Designe la cara o línea del elemento que se alinea"
            },
            Want::Pick,
        )
    }

    fn feed(&mut self, input: Input, cx: &Ctx<'_, dyn Geo + '_>, _st: &mut CadSettings) -> Step {
        match input {
            Input::Pick(e, at) if self.reference.is_none() => match nearest_line(cx, e, at) {
                Some(l) => {
                    self.reference = Some((e, l));
                    Step::Next
                }
                None => Step::Refuse("Ese elemento no tiene líneas rectas.".into()),
            },
            Input::Pick(e, at) => {
                let (ref_el, reference) = self.reference.unwrap();
                if ref_el == e {
                    return Step::Refuse("Designe otro elemento.".into());
                }
                if matches!(e, Element::Opening(_)) {
                    return Step::Refuse(
                        "Las puertas y ventanas se alinean moviéndolas en su muro.".into(),
                    );
                }
                let Some(line) = nearest_line(cx, e, at) else {
                    return Step::Refuse("Ese elemento no tiene líneas rectas.".into());
                };
                let (Some(u), Some(v)) = (reference.dir(), line.dir()) else {
                    return Step::Next;
                };
                if u.cross(v).abs() > PARALLEL {
                    return Step::Refuse(
                        "La línea no es paralela a la referencia: no se puede alinear.".into(),
                    );
                }
                // Across the reference, from the picked line to it.
                let n = u.perp_left();
                let d = n * (reference.a - line.a).dot(n);
                self.reference = None;
                if d.length() < 1e-9 {
                    return Step::Say("Ya está alineado.".into());
                }
                Step::Apply(Changes {
                    label: "Alinear",
                    changes: vec![Change::MoveBy(BTreeSet::from([e]), d)],
                })
            }
            Input::Enter => Step::Done(None),
            _ => Step::Next,
        }
    }

    fn picked(&self) -> Vec<Element> {
        self.reference.map(|(e, _)| e).into_iter().collect()
    }
}
