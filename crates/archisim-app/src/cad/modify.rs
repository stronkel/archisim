//! Editing commands: DESPLAZA, COPIA, GIRA, ESCALA, SIMETRIA, ALINEAR,
//! EQUIDIST, RECORTA, ALARGA, EMPALME, CHAFLAN, MATRIZ, PARTE, PARTEPUNTO,
//! JUNTAR, DESCOMP, ESTIRA, LONGITUD, BORRA, IGUALARPROP and DIVIDE.

use std::collections::BTreeSet;
use std::f64::consts::TAU;

use archisim_core::{NewDraft, Point2, Shape};
use archisim_draft::ops;
use archisim_draft::seg::angle_of;
use archisim_draft::Transform;

use super::*;
use crate::units::{fmt_mm, parse_length};

/// Changes that replace a drafting element by some shapes: the first keeps
/// its identity, the others are new elements with its layer and style; no
/// shapes removes it.
fn replace(cx: &Ctx<'_, dyn Geo + '_>, id: DraftId, shapes: Vec<Shape>) -> Vec<Change> {
    let Some(d) = cx.model().draft(id) else {
        return Vec::new();
    };
    let mut it = shapes.into_iter();
    let Some(first) = it.next() else {
        return vec![Change::Remove(Element::Draft(id))];
    };
    let mut v = vec![Change::Reshape(id, first)];
    for s in it {
        v.push(Change::AddDraft(NewDraft {
            view: d.view,
            layer: d.layer,
            shape: s,
            color: d.color,
            weight: d.weight,
            line: d.line,
        }));
    }
    v
}

/// A copy of a drafting element's style for a new shape.
fn like(cx: &Ctx<'_, dyn Geo + '_>, id: DraftId, shape: Shape) -> Option<Change> {
    let d = cx.model().draft(id)?;
    Some(Change::AddDraft(NewDraft {
        view: d.view,
        layer: d.layer,
        shape,
        color: d.color,
        weight: d.weight,
        line: d.line,
    }))
}

fn take_selection(cx: &Ctx<'_, dyn Geo + '_>) -> BTreeSet<Element> {
    cx.selection
        .iter()
        .copied()
        // A door or window moves with its wall, not on its own here.
        .filter(|e| !matches!(e, Element::Opening(_)))
        .collect()
}

// ---- DESPLAZA, COPIA, GIRA, ESCALA, SIMETRIA, ALINEAR ----------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum XformKind {
    Move,
    Copy,
    Rotate,
    Scale,
    Mirror,
    Align,
}

/// Move, copy, rotate, scale, mirror and align: the selection, then a base
/// point, then where it goes. Works on walls, slabs and annotations too.
pub struct Xform {
    kind: XformKind,
    items: Option<BTreeSet<Element>>,
    points: Vec<Point2>,
    /// GIRA and ESCALA: copy instead of moving (option Copia).
    copy: bool,
    /// Reference angle or length typed (option Referencia).
    reference: Option<Vec<Point2>>,
    asking_erase: bool,
}

impl Xform {
    pub fn new(kind: XformKind) -> Self {
        Self {
            kind,
            items: None,
            points: Vec::new(),
            copy: kind == XformKind::Copy,
            reference: None,
            asking_erase: false,
        }
    }

    fn label(&self) -> &'static str {
        match (self.kind, self.copy) {
            (XformKind::Move, _) => "Desplazar",
            (XformKind::Copy, _) => "Copiar",
            (XformKind::Rotate, false) => "Girar",
            (XformKind::Rotate, true) => "Girar copia",
            (XformKind::Scale, false) => "Escalar",
            (XformKind::Scale, true) => "Escalar copia",
            (XformKind::Mirror, _) => "Simetría",
            (XformKind::Align, _) => "Alinear",
        }
    }

    /// The transform for the last point (the cursor, while previewing).
    fn transform_to(&self, p: Point2) -> Option<Transform> {
        let base = *self.points.first()?;
        match self.kind {
            XformKind::Move | XformKind::Copy => Some(Transform::translate(p - base)),
            XformKind::Rotate => {
                if p.distance(base) < 1e-9 {
                    return None;
                }
                let a = angle_of(p - base);
                let r = match self.reference.as_deref() {
                    Some([a0, a1]) => angle_of(*a1 - *a0),
                    _ => 0.0,
                };
                Some(Transform::rotate(base, a - r))
            }
            XformKind::Scale => {
                let d = p.distance(base);
                let r = match self.reference.as_deref() {
                    Some([a0, a1]) => a0.distance(*a1),
                    _ => 1.0,
                };
                (d > 1e-9 && r > 1e-9).then(|| Transform::scale(base, d / r))
            }
            XformKind::Mirror => (p.distance(base) > 1e-9).then(|| Transform::mirror(base, p)),
            XformKind::Align => match self.points.as_slice() {
                [s1, d1, s2] => {
                    let a = angle_of(p - *d1) - angle_of(*s2 - *s1);
                    Some(Transform::translate(*d1 - *s1).then(&Transform::rotate(*d1, a)))
                }
                [s1, d1] => Some(Transform::translate(*d1 - *s1)),
                _ => Some(Transform::translate(p - base)),
            },
        }
    }

    fn apply(&self, t: Transform, copy: bool) -> Changes {
        Changes {
            label: self.label(),
            changes: vec![Change::Transform {
                items: self.items.clone().unwrap_or_default(),
                t,
                copy,
            }],
        }
    }
}

impl Command for Xform {
    fn name(&self) -> &'static str {
        match self.kind {
            XformKind::Move => "DESPLAZA",
            XformKind::Copy => "COPIA",
            XformKind::Rotate => "GIRA",
            XformKind::Scale => "ESCALA",
            XformKind::Mirror => "SIMETRIA",
            XformKind::Align => "ALINEAR",
        }
    }

    fn start(&mut self, cx: &Ctx<'_, dyn Geo + '_>, _st: &mut CadSettings) -> Step {
        let sel = take_selection(cx);
        if !sel.is_empty() {
            self.items = Some(sel);
        }
        Step::Next
    }

    fn prompt(&self) -> Prompt {
        if self.items.is_none() {
            return Prompt::new("Designe objetos e Intro", Want::Select);
        }
        if self.asking_erase {
            return Prompt::new("¿Borrar objetos de origen?", Want::Value)
                .opts(&[opt("S", "Sí"), opt("N", "No")])
                .default("No");
        }
        if let Some(r) = &self.reference {
            return Prompt::new(
                if r.len() < 2 {
                    "Precise punto de referencia"
                } else if self.kind == XformKind::Rotate {
                    "Precise nuevo ángulo (punto o °)"
                } else {
                    "Precise nueva longitud (punto o mm)"
                },
                Want::Point,
            );
        }
        match (self.kind, self.points.len()) {
            (XformKind::Align, 0) => Prompt::new("Precise primer punto de origen", Want::Point),
            (XformKind::Align, 1) => Prompt::new("Precise primer punto de destino", Want::Point),
            (XformKind::Align, 2) => Prompt::new("Precise segundo punto de origen", Want::Point),
            (XformKind::Align, _) => Prompt::new("Precise segundo punto de destino", Want::Point),
            (XformKind::Mirror, 0) => {
                Prompt::new("Precise primer punto de la línea de simetría", Want::Point)
            }
            (XformKind::Mirror, _) => {
                Prompt::new("Precise segundo punto de la línea de simetría", Want::Point)
            }
            (_, 0) => Prompt::new("Precise punto base", Want::Point),
            (XformKind::Move, _) => {
                Prompt::new("Precise segundo punto (o desplazamiento)", Want::Point)
            }
            (XformKind::Copy, _) => {
                Prompt::new("Precise segundo punto (Intro para terminar)", Want::Point)
            }
            (XformKind::Rotate, _) => {
                Prompt::new("Precise ángulo de rotación (punto o °)", Want::Point)
                    .opts(&[opt("C", "Copia"), opt("R", "Referencia")])
                    .numeric()
            }
            (XformKind::Scale, _) => Prompt::new("Precise factor de escala (o punto)", Want::Point)
                .opts(&[opt("C", "Copia"), opt("R", "Referencia")])
                .numeric(),
        }
    }

    fn feed(&mut self, input: Input, cx: &Ctx<'_, dyn Geo + '_>, _st: &mut CadSettings) -> Step {
        if self.items.is_none() {
            return match input {
                Input::Enter => {
                    let sel = take_selection(cx);
                    if sel.is_empty() {
                        Step::Refuse("No hay nada seleccionado.".into())
                    } else {
                        self.items = Some(sel);
                        Step::Next
                    }
                }
                _ => Step::Next,
            };
        }
        if self.asking_erase {
            let erase = matches!(input, Input::Option("S"));
            let [a, b] = [self.points[0], self.points[1]];
            return Step::Done(Some(self.apply(Transform::mirror(a, b), !erase)));
        }
        if let Some(r) = &mut self.reference {
            if r.len() < 2 {
                let Input::Point(p) = input else {
                    return Step::Refuse("Precise un punto.".into());
                };
                r.push(p);
                return Step::Next;
            }
            // The new angle or length.
            let base = self.points[0];
            let target = match (&input, self.kind) {
                (Input::Point(p), _) => *p,
                (Input::Value(v), XformKind::Rotate) => match parse_angle(v) {
                    Some(a) => base + archisim_draft::seg::dir(a),
                    None => return Step::Refuse("Escriba un ángulo en grados.".into()),
                },
                (Input::Value(v), _) => match parse_length(v) {
                    Some(l) if l > 0.0 => base + Point2::new(l, 0.0),
                    _ => return Step::Refuse("Escriba una longitud en mm.".into()),
                },
                _ => return Step::Next,
            };
            return match self.transform_to(target) {
                Some(t) => Step::Done(Some(self.apply(t, self.copy))),
                None => Step::Refuse("Ese punto no vale.".into()),
            };
        }
        match input {
            Input::Option("C") if matches!(self.kind, XformKind::Rotate | XformKind::Scale) => {
                self.copy = !self.copy;
                Step::Say(if self.copy {
                    "Se girará o escalará una copia.".into()
                } else {
                    "Se girarán o escalarán los originales.".into()
                })
            }
            Input::Option("R") if matches!(self.kind, XformKind::Rotate | XformKind::Scale) => {
                self.reference = Some(Vec::new());
                Step::Next
            }
            Input::Value(v) if !self.points.is_empty() => {
                let base = self.points[0];
                match self.kind {
                    XformKind::Rotate => match parse_angle(&v) {
                        Some(a) => {
                            Step::Done(Some(self.apply(Transform::rotate(base, a), self.copy)))
                        }
                        None => Step::Refuse("Escriba un ángulo en grados.".into()),
                    },
                    XformKind::Scale => match parse_number(&v) {
                        Some(f) if f > 0.0 => {
                            Step::Done(Some(self.apply(Transform::scale(base, f), self.copy)))
                        }
                        _ => Step::Refuse("Escriba un factor mayor que cero.".into()),
                    },
                    _ => Step::Refuse("Precise un punto.".into()),
                }
            }
            Input::Point(p) => {
                if self.points.is_empty() {
                    self.points.push(p);
                    return Step::Next;
                }
                match self.kind {
                    XformKind::Mirror => {
                        if p.distance(self.points[0]) < 1e-9 {
                            return Step::Refuse("Los dos puntos son el mismo.".into());
                        }
                        self.points.push(p);
                        self.asking_erase = true;
                        Step::Next
                    }
                    XformKind::Align if self.points.len() < 3 => {
                        self.points.push(p);
                        Step::Next
                    }
                    XformKind::Copy => match self.transform_to(p) {
                        Some(t) => Step::Apply(self.apply(t, true)),
                        None => Step::Next,
                    },
                    _ => match self.transform_to(p) {
                        Some(t) => Step::Done(Some(self.apply(t, self.copy))),
                        None => Step::Refuse("Ese punto no vale.".into()),
                    },
                }
            }
            Input::Enter if self.kind == XformKind::Align && self.points.len() == 2 => {
                // Only the first pair: a plain move.
                let t = Transform::translate(self.points[1] - self.points[0]);
                Step::Done(Some(self.apply(t, false)))
            }
            Input::Enter => Step::Done(None),
            _ => Step::Next,
        }
    }

    fn preview(&self, cx: &Ctx<'_, dyn Geo + '_>, _st: &CadSettings) -> Vec<Shape> {
        let Some(items) = &self.items else {
            return Vec::new();
        };
        if self.points.is_empty() || self.asking_erase {
            return Vec::new();
        }
        if self.reference.as_ref().is_some_and(|r| r.len() < 2) {
            return Vec::new();
        }
        let Some(t) = self.transform_to(cx.cursor) else {
            return Vec::new();
        };
        let mut v = ghost(cx, items, &t);
        v.push(Shape::Line {
            a: self.points[self.points.len() - 1],
            b: cx.cursor,
        });
        v
    }

    fn base(&self) -> Option<Point2> {
        self.points.last().copied()
    }

    fn picked(&self) -> Vec<Element> {
        self.items.iter().flatten().copied().collect()
    }
}

/// The outlines of elements after a transform, to follow the cursor.
fn ghost(cx: &Ctx<'_, dyn Geo + '_>, items: &BTreeSet<Element>, t: &Transform) -> Vec<Shape> {
    let mut v = Vec::new();
    for e in items.iter().take(500) {
        match e {
            Element::Draft(id) => {
                if let Some(d) = cx.model().draft(*id) {
                    v.push(archisim_draft::transform_shape(&d.shape, t));
                }
            }
            other => {
                if let Some(poly) = crate::pick::outline(cx.geo, *other) {
                    let pts: Vec<Point2> = poly.iter().map(|p| t.apply(*p)).collect();
                    v.push(Shape::Polyline(archisim_core::Polyline::straight(
                        pts, true,
                    )));
                }
            }
        }
    }
    v
}

// ---- EQUIDIST ----------------------------------------------------------------

/// Offsets: a distance (or a point it must pass through), then objects and
/// the side, again and again.
#[derive(Default)]
pub struct Offset {
    distance: Option<f64>,
    through: bool,
    object: Option<(DraftId, Point2)>,
}

impl Command for Offset {
    fn name(&self) -> &'static str {
        "EQUIDIST"
    }

    fn prompt(&self) -> Prompt {
        if self.distance.is_none() && !self.through {
            return Prompt::new("Precise distancia de desfase (mm)", Want::Value)
                .opts(&[opt("P", "Punto a atravesar")]);
        }
        match self.object {
            None => Prompt::new(
                "Designe objeto a desplazar (Intro para terminar)",
                Want::Pick,
            ),
            Some(_) if self.through => Prompt::new("Precise punto a atravesar", Want::Point),
            Some(_) => Prompt::new("Precise punto en el lado de desplazamiento", Want::Point),
        }
    }

    fn start(&mut self, _cx: &Ctx<'_, dyn Geo + '_>, _st: &mut CadSettings) -> Step {
        Step::Next
    }

    fn feed(&mut self, input: Input, cx: &Ctx<'_, dyn Geo + '_>, st: &mut CadSettings) -> Step {
        if self.distance.is_none() && !self.through {
            return match input {
                Input::Option("P") => {
                    self.through = true;
                    Step::Next
                }
                Input::Enter => {
                    self.distance = Some(st.offset);
                    Step::Next
                }
                Input::Value(v) => match parse_length(&v) {
                    Some(d) if d > 0.0 => {
                        st.offset = d;
                        self.distance = Some(d);
                        Step::Next
                    }
                    _ => Step::Refuse("Escriba una distancia mayor que cero.".into()),
                },
                Input::Point(_) => Step::Refuse("Escriba la distancia en mm.".into()),
                _ => Step::Next,
            };
        }
        match (self.object, input) {
            (None, Input::Pick(Element::Draft(id), at)) => {
                self.object = Some((id, at));
                Step::Next
            }
            (None, Input::Pick(..)) => {
                Step::Refuse("Solo se desfasan elementos de dibujo 2D.".into())
            }
            (None, Input::Enter) => Step::Done(None),
            (Some((id, _)), Input::Point(side)) => {
                self.object = None;
                let Some(shape) = cx.model().draft(id).map(|d| d.shape.clone()) else {
                    return Step::Next;
                };
                let d = if self.through {
                    archisim_draft::distance(&shape, side, cx.k)
                } else {
                    self.distance.unwrap_or(st.offset)
                };
                match ops::offset(&shape, d, side) {
                    Ok(s) => match like(cx, id, s) {
                        Some(c) => Step::Apply(Changes {
                            label: "Equidistancia",
                            changes: vec![c],
                        }),
                        None => Step::Next,
                    },
                    Err(e) => Step::Refuse(format!("No se puede desfasar: {e}.")),
                }
            }
            _ => Step::Next,
        }
    }

    fn preview(&self, cx: &Ctx<'_, dyn Geo + '_>, st: &CadSettings) -> Vec<Shape> {
        let Some((id, _)) = self.object else {
            return Vec::new();
        };
        let Some(shape) = cx.model().draft(id).map(|d| &d.shape) else {
            return Vec::new();
        };
        let d = if self.through {
            archisim_draft::distance(shape, cx.cursor, cx.k)
        } else {
            self.distance.unwrap_or(st.offset)
        };
        ops::offset(shape, d, cx.cursor).into_iter().collect()
    }

    fn picked(&self) -> Vec<Element> {
        self.object
            .map(|(id, _)| Element::Draft(id))
            .into_iter()
            .collect()
    }
}

// ---- RECORTA, ALARGA -------------------------------------------------------------

/// Trim and extend against every edge in the view (AutoCAD's quick mode).
/// Picking with Shift held does the other one, as in AutoCAD.
pub struct TrimExtend {
    extend: bool,
}

impl TrimExtend {
    pub fn new(extend: bool) -> Self {
        Self { extend }
    }
}

/// The edges of the view other than the shape's own segments.
fn edges_except(cx: &Ctx<'_, dyn Geo + '_>, shape: &Shape) -> Vec<archisim_draft::Seg> {
    let own = archisim_draft::boundary_segs(shape);
    cx.edges()
        .into_iter()
        .filter(|s| !own.contains(s))
        .collect()
}

impl Command for TrimExtend {
    fn name(&self) -> &'static str {
        if self.extend {
            "ALARGA"
        } else {
            "RECORTA"
        }
    }

    fn prompt(&self) -> Prompt {
        Prompt::new(
            if self.extend {
                "Designe objeto a alargar (Mayús: recortar; Intro para terminar)"
            } else {
                "Designe objeto a recortar (Mayús: alargar; Intro para terminar)"
            },
            Want::Pick,
        )
    }

    fn feed(&mut self, input: Input, cx: &Ctx<'_, dyn Geo + '_>, _st: &mut CadSettings) -> Step {
        match input {
            Input::Pick(Element::Draft(id), at) => {
                let Some(shape) = cx.model().draft(id).map(|d| d.shape.clone()) else {
                    return Step::Next;
                };
                let edges = edges_except(cx, &shape);
                // Shift is passed as the option "invert".
                if self.extend {
                    match ops::extend(&shape, at, &edges) {
                        Ok(s) => Step::Apply(Changes {
                            label: "Alargar",
                            changes: vec![Change::Reshape(id, s)],
                        }),
                        Err(e) => Step::Refuse(format!("No se puede alargar: {e}.")),
                    }
                } else {
                    match ops::trim(&shape, at, &edges) {
                        Ok(pieces) => Step::Apply(Changes {
                            label: "Recortar",
                            changes: replace(cx, id, pieces),
                        }),
                        // Nothing cuts it: AutoCAD deletes it.
                        Err(ops::EditError::NoIntersection) => Step::Apply(Changes {
                            label: "Recortar",
                            changes: vec![Change::Remove(Element::Draft(id))],
                        }),
                        Err(e) => Step::Refuse(format!("No se puede recortar: {e}.")),
                    }
                }
            }
            Input::Option("INVERT") => {
                self.extend = !self.extend;
                Step::Next
            }
            Input::Pick(..) => {
                Step::Refuse("Solo se recortan y alargan elementos de dibujo 2D.".into())
            }
            Input::Enter => Step::Done(None),
            _ => Step::Next,
        }
    }
}

// ---- EMPALME, CHAFLAN ----------------------------------------------------------------

pub struct Fillet {
    chamfer: bool,
    first: Option<(DraftId, Point2)>,
    asking: Option<&'static str>,
    polyline: bool,
}

impl Fillet {
    pub fn new(chamfer: bool) -> Self {
        Self {
            chamfer,
            first: None,
            asking: None,
            polyline: false,
        }
    }

    fn label(&self) -> &'static str {
        if self.chamfer {
            "Chaflán"
        } else {
            "Empalme"
        }
    }
}

impl Command for Fillet {
    fn name(&self) -> &'static str {
        if self.chamfer {
            "CHAFLAN"
        } else {
            "EMPALME"
        }
    }

    fn prompt(&self) -> Prompt {
        match self.asking {
            Some("R") => return Prompt::new("Radio de empalme (mm)", Want::Value),
            Some("D1") => return Prompt::new("Primera distancia de chaflán (mm)", Want::Value),
            Some("D2") => return Prompt::new("Segunda distancia de chaflán (mm)", Want::Value),
            _ => {}
        }
        if self.polyline {
            return Prompt::new("Designe la polilínea", Want::Pick);
        }
        match self.first {
            None => {
                let o = if self.chamfer {
                    opt("D", "Distancias")
                } else {
                    opt("R", "Radio")
                };
                Prompt::new("Designe primera línea", Want::Pick).opts(&[o, opt("P", "Polilínea")])
            }
            Some(_) => Prompt::new("Designe segunda línea", Want::Pick),
        }
    }

    fn feed(&mut self, input: Input, cx: &Ctx<'_, dyn Geo + '_>, st: &mut CadSettings) -> Step {
        if let Some(asked) = self.asking.take() {
            let Input::Value(v) = input else {
                return Step::Next;
            };
            let Some(d) = parse_length(&v).filter(|d| *d >= 0.0) else {
                return Step::Refuse("Escriba una medida en mm.".into());
            };
            match asked {
                "R" => st.fillet_radius = d,
                "D1" => {
                    st.chamfer.0 = d;
                    st.chamfer.1 = d;
                    self.asking = Some("D2");
                }
                _ => st.chamfer.1 = d,
            }
            return Step::Next;
        }
        match input {
            Input::Option("R") => {
                self.asking = Some("R");
                Step::Next
            }
            Input::Option("D") => {
                self.asking = Some("D1");
                Step::Next
            }
            Input::Option("P") => {
                self.polyline = true;
                Step::Next
            }
            Input::Pick(Element::Draft(id), at) if self.polyline => {
                let Some(Shape::Polyline(p)) = cx.model().draft(id).map(|d| d.shape.clone()) else {
                    return Step::Refuse("Elija una polilínea.".into());
                };
                let r = if self.chamfer {
                    st.chamfer.0
                } else {
                    st.fillet_radius
                };
                let _ = at;
                match ops::fillet_polyline(&p, r, self.chamfer) {
                    Ok(q) => Step::Done(Some(Changes {
                        label: self.label(),
                        changes: vec![Change::Reshape(id, Shape::Polyline(q))],
                    })),
                    Err(e) => Step::Refuse(format!("No se puede: {e}.")),
                }
            }
            Input::Pick(Element::Draft(id), at) => match self.first {
                None => {
                    self.first = Some((id, at));
                    Step::Next
                }
                Some((a, pa)) if a != id => {
                    let (Some(sa), Some(sb)) = (
                        cx.model().draft(a).map(|d| d.shape.clone()),
                        cx.model().draft(id).map(|d| d.shape.clone()),
                    ) else {
                        return Step::Next;
                    };
                    let result = if self.chamfer {
                        ops::chamfer(&sa, pa, &sb, at, st.chamfer.0, st.chamfer.1)
                    } else {
                        ops::fillet(&sa, pa, &sb, at, st.fillet_radius)
                    };
                    self.first = None;
                    match result {
                        Ok((na, nb, extra)) => {
                            let mut changes = vec![Change::Reshape(a, na), Change::Reshape(id, nb)];
                            if let Some(e) = extra {
                                changes.extend(like(cx, a, e));
                            }
                            Step::Apply(Changes {
                                label: self.label(),
                                changes,
                            })
                        }
                        Err(e) => Step::Refuse(format!("No se puede: {e}.")),
                    }
                }
                Some(_) => Step::Refuse("Elija otra línea.".into()),
            },
            Input::Pick(..) => Step::Refuse("Elija líneas de dibujo 2D.".into()),
            Input::Enter => Step::Done(None),
            _ => Step::Next,
        }
    }

    fn picked(&self) -> Vec<Element> {
        self.first
            .map(|(id, _)| Element::Draft(id))
            .into_iter()
            .collect()
    }
}

// ---- MATRIZ -------------------------------------------------------------------------

/// Arrays of the selection (any element): rectangular or polar.
#[derive(Default)]
pub struct Array {
    items: Option<BTreeSet<Element>>,
    polar: Option<bool>,
    values: Vec<f64>,
    centre: Option<Point2>,
}

impl Command for Array {
    fn name(&self) -> &'static str {
        "MATRIZ"
    }

    fn start(&mut self, cx: &Ctx<'_, dyn Geo + '_>, _st: &mut CadSettings) -> Step {
        let sel = take_selection(cx);
        if !sel.is_empty() {
            self.items = Some(sel);
        }
        Step::Next
    }

    fn prompt(&self) -> Prompt {
        if self.items.is_none() {
            return Prompt::new("Designe objetos e Intro", Want::Select);
        }
        match self.polar {
            None => Prompt::new("Tipo de matriz", Want::Value)
                .opts(&[opt("R", "Rectangular"), opt("P", "Polar")])
                .default("Rectangular"),
            Some(false) => Prompt::new(
                match self.values.len() {
                    0 => "Número de filas",
                    1 => "Número de columnas",
                    2 => "Distancia entre filas (mm, negativa hacia abajo)",
                    _ => "Distancia entre columnas (mm, negativa a la izquierda)",
                },
                Want::Value,
            ),
            Some(true) if self.centre.is_none() => {
                Prompt::new("Precise centro de la matriz", Want::Point)
            }
            Some(true) => Prompt::new(
                if self.values.is_empty() {
                    "Número total de elementos"
                } else {
                    "Ángulo a rellenar (°, 360 = vuelta entera)"
                },
                Want::Value,
            )
            .default(if self.values.is_empty() { "6" } else { "360" }),
        }
    }

    fn feed(&mut self, input: Input, cx: &Ctx<'_, dyn Geo + '_>, st: &mut CadSettings) -> Step {
        if self.items.is_none() {
            if input == Input::Enter {
                let sel = take_selection(cx);
                if sel.is_empty() {
                    return Step::Refuse("No hay nada seleccionado.".into());
                }
                self.items = Some(sel);
            }
            return Step::Next;
        }
        let Some(polar) = self.polar else {
            self.polar = Some(matches!(input, Input::Option("P")));
            return Step::Next;
        };
        if polar && self.centre.is_none() {
            if let Input::Point(p) = input {
                self.centre = Some(p);
            }
            return Step::Next;
        }
        let value = match &input {
            Input::Value(v) => {
                // Counts are plain numbers; distances in mm.
                let counting =
                    (!polar && self.values.len() < 2) || (polar && self.values.is_empty());
                if counting || polar {
                    parse_number(v)
                } else {
                    parse_length(v)
                }
            }
            Input::Enter if polar => Some(if self.values.is_empty() {
                st.array_count as f64
            } else {
                360.0
            }),
            Input::Enter => Some(match self.values.len() {
                0 => st.array_rows as f64,
                1 => st.array_cols as f64,
                _ => 1.0,
            }),
            _ => None,
        };
        let Some(v) = value else {
            return Step::Refuse("Escriba un número.".into());
        };
        self.values.push(v);
        let items = self.items.clone().unwrap_or_default();
        let mut changes = Vec::new();
        if !polar && self.values.len() == 4 {
            let (rows, cols) = (
                self.values[0].round().max(1.0) as usize,
                self.values[1].round().max(1.0) as usize,
            );
            st.array_rows = rows;
            st.array_cols = cols;
            for r in 0..rows {
                for c in 0..cols {
                    if r == 0 && c == 0 {
                        continue;
                    }
                    changes.push(Change::Transform {
                        items: items.clone(),
                        t: Transform::translate(Point2::new(
                            self.values[3] * c as f64,
                            self.values[2] * r as f64,
                        )),
                        copy: true,
                    });
                }
            }
        } else if polar && self.values.len() == 2 {
            let n = self.values[0].round().max(1.0) as usize;
            st.array_count = n;
            let fill = self.values[1].to_radians();
            let full = (fill.abs() - TAU).abs() < 1e-9;
            let step = if full {
                fill / n as f64
            } else if n > 1 {
                fill / (n - 1) as f64
            } else {
                0.0
            };
            let c = self.centre.unwrap_or_default();
            for i in 1..n {
                changes.push(Change::Transform {
                    items: items.clone(),
                    t: Transform::rotate(c, step * i as f64),
                    copy: true,
                });
            }
        } else {
            return Step::Next;
        }
        Step::Done(Some(Changes {
            label: "Matriz",
            changes,
        }))
    }

    fn picked(&self) -> Vec<Element> {
        self.items.iter().flatten().copied().collect()
    }
}

// ---- PARTE, PARTEPUNTO ---------------------------------------------------------------

pub struct Break {
    at_point: bool,
    object: Option<(DraftId, Point2)>,
    first_asked: bool,
}

impl Break {
    pub fn new(at_point: bool) -> Self {
        Self {
            at_point,
            object: None,
            first_asked: false,
        }
    }
}

impl Command for Break {
    fn name(&self) -> &'static str {
        if self.at_point {
            "PARTEPUNTO"
        } else {
            "PARTE"
        }
    }

    fn prompt(&self) -> Prompt {
        match self.object {
            None => Prompt::new("Designe objeto", Want::Pick),
            Some(_) if self.at_point => Prompt::new("Precise punto de ruptura", Want::Point),
            Some(_) if self.first_asked => {
                Prompt::new("Precise primer punto de ruptura", Want::Point)
            }
            Some(_) => Prompt::new("Precise segundo punto de ruptura", Want::Point)
                .opts(&[opt("P", "Primer punto")]),
        }
    }

    fn feed(&mut self, input: Input, cx: &Ctx<'_, dyn Geo + '_>, _st: &mut CadSettings) -> Step {
        match input {
            Input::Pick(Element::Draft(id), at) if self.object.is_none() => {
                self.object = Some((id, at));
                Step::Next
            }
            Input::Option("P") => {
                self.first_asked = true;
                Step::Next
            }
            Input::Point(p) => {
                let Some((id, at)) = self.object else {
                    return Step::Next;
                };
                if self.first_asked {
                    self.object = Some((id, p));
                    self.first_asked = false;
                    return Step::Next;
                }
                let Some(shape) = cx.model().draft(id).map(|d| d.shape.clone()) else {
                    return Step::Done(None);
                };
                let first = if self.at_point { p } else { at };
                match ops::break_between(&shape, first, p) {
                    Ok(pieces) => Step::Done(Some(Changes {
                        label: "Partir",
                        changes: replace(cx, id, pieces),
                    })),
                    Err(e) => Step::Refuse(format!("No se puede partir: {e}.")),
                }
            }
            Input::Pick(..) => Step::Refuse("Elija un elemento de dibujo 2D.".into()),
            Input::Enter => Step::Done(None),
            _ => Step::Next,
        }
    }

    fn picked(&self) -> Vec<Element> {
        self.object
            .map(|(id, _)| Element::Draft(id))
            .into_iter()
            .collect()
    }
}

// ---- JUNTAR, DESCOMP, BORRA ------------------------------------------------------------

/// Commands that take a selection and do one thing to it.
fn selection_start<T>(
    items: &mut Option<BTreeSet<Element>>,
    cx: &Ctx<'_, dyn Geo + '_>,
) -> Option<T> {
    let sel = take_selection(cx);
    if !sel.is_empty() {
        *items = Some(sel);
    }
    None
}

#[derive(Default)]
pub struct Join {
    items: Option<BTreeSet<Element>>,
}

impl Join {
    fn run(&self, cx: &Ctx<'_, dyn Geo + '_>) -> Step {
        let ids: Vec<DraftId> = self
            .items
            .iter()
            .flatten()
            .filter_map(|e| match e {
                Element::Draft(id) => Some(*id),
                _ => None,
            })
            .collect();
        let shapes: Vec<Shape> = ids
            .iter()
            .filter_map(|id| cx.model().draft(*id).map(|d| d.shape.clone()))
            .collect();
        match ops::join(&shapes, 1e-4) {
            Ok((joined, used)) => {
                let first = ids[used[0]];
                let mut changes = vec![Change::Reshape(first, joined)];
                for i in used.iter().skip(1) {
                    changes.push(Change::Remove(Element::Draft(ids[*i])));
                }
                Step::Done(Some(Changes {
                    label: "Juntar",
                    changes,
                }))
            }
            Err(_) => Step::SayDone("No se ha juntado nada: los extremos no coinciden.".into()),
        }
    }
}

impl Command for Join {
    fn name(&self) -> &'static str {
        "JUNTAR"
    }

    fn start(&mut self, cx: &Ctx<'_, dyn Geo + '_>, _st: &mut CadSettings) -> Step {
        selection_start::<()>(&mut self.items, cx);
        if self.items.is_some() {
            return self.run(cx);
        }
        Step::Next
    }

    fn prompt(&self) -> Prompt {
        Prompt::new(
            "Designe líneas, arcos y polilíneas a juntar e Intro",
            Want::Select,
        )
    }

    fn feed(&mut self, input: Input, cx: &Ctx<'_, dyn Geo + '_>, _st: &mut CadSettings) -> Step {
        if input == Input::Enter {
            self.items = Some(take_selection(cx));
            return self.run(cx);
        }
        Step::Next
    }
}

#[derive(Default)]
pub struct Explode {
    items: Option<BTreeSet<Element>>,
}

impl Explode {
    fn run(&self, cx: &Ctx<'_, dyn Geo + '_>) -> Step {
        let mut changes = Vec::new();
        for e in self.items.iter().flatten() {
            if let Element::Draft(id) = e {
                if let Some(Ok(parts)) = cx.model().draft(*id).map(|d| ops::explode(&d.shape)) {
                    changes.extend(replace(cx, *id, parts));
                }
            }
        }
        if changes.is_empty() {
            return Step::SayDone("Nada que descomponer (solo polilíneas y sombreados).".into());
        }
        Step::Done(Some(Changes {
            label: "Descomponer",
            changes,
        }))
    }
}

impl Command for Explode {
    fn name(&self) -> &'static str {
        "DESCOMP"
    }

    fn start(&mut self, cx: &Ctx<'_, dyn Geo + '_>, _st: &mut CadSettings) -> Step {
        selection_start::<()>(&mut self.items, cx);
        if self.items.is_some() {
            return self.run(cx);
        }
        Step::Next
    }

    fn prompt(&self) -> Prompt {
        Prompt::new("Designe objetos a descomponer e Intro", Want::Select)
    }

    fn feed(&mut self, input: Input, cx: &Ctx<'_, dyn Geo + '_>, _st: &mut CadSettings) -> Step {
        if input == Input::Enter {
            self.items = Some(take_selection(cx));
            return self.run(cx);
        }
        Step::Next
    }
}

#[derive(Default)]
pub struct Erase {
    items: Option<BTreeSet<Element>>,
}

impl Erase {
    fn run(&self) -> Step {
        let items: Vec<Change> = self
            .items
            .iter()
            .flatten()
            .map(|e| Change::Remove(*e))
            .collect();
        Step::Done((!items.is_empty()).then_some(Changes {
            label: "Borrar",
            changes: items,
        }))
    }
}

impl Command for Erase {
    fn name(&self) -> &'static str {
        "BORRA"
    }

    fn start(&mut self, cx: &Ctx<'_, dyn Geo + '_>, _st: &mut CadSettings) -> Step {
        // Here doors and windows count too.
        if !cx.selection.is_empty() {
            self.items = Some(cx.selection.clone());
            return self.run();
        }
        Step::Next
    }

    fn prompt(&self) -> Prompt {
        Prompt::new("Designe objetos a borrar e Intro", Want::Select)
    }

    fn feed(&mut self, input: Input, cx: &Ctx<'_, dyn Geo + '_>, _st: &mut CadSettings) -> Step {
        if input == Input::Enter {
            self.items = Some(cx.selection.clone());
            return self.run();
        }
        Step::Next
    }
}

// ---- ESTIRA -----------------------------------------------------------------------------

/// Stretch: a crossing window (two corners), then base and second point.
/// Drafting points inside the window move; elements wholly inside move.
#[derive(Default)]
pub struct Stretch {
    corners: Vec<Point2>,
    base: Option<Point2>,
}

impl Stretch {
    fn changes(&self, cx: &Ctx<'_, dyn Geo + '_>, to: Point2) -> Vec<Change> {
        let (Some(base), [a, b]) = (self.base, self.corners.as_slice()) else {
            return Vec::new();
        };
        let lo = Point2::new(a.x.min(b.x), a.y.min(b.y));
        let hi = Point2::new(a.x.max(b.x), a.y.max(b.y));
        let d = to - base;
        let mut v = Vec::new();
        for dr in cx.model().drafts_in(cx.view) {
            if !cx.model().draft_editable(dr) {
                continue;
            }
            if archisim_draft::crosses_window(&dr.shape, lo, hi, cx.k) {
                let s = ops::stretch(&dr.shape, lo, hi, d);
                if s != dr.shape {
                    v.push(Change::Reshape(dr.id, s));
                }
            }
        }
        v
    }
}

impl Command for Stretch {
    fn name(&self) -> &'static str {
        "ESTIRA"
    }

    fn prompt(&self) -> Prompt {
        match (self.corners.len(), self.base) {
            (0, _) => Prompt::new(
                "Precise primera esquina de la ventana de captura",
                Want::Point,
            ),
            (1, _) => Prompt::new("Precise esquina opuesta", Want::Point),
            (_, None) => Prompt::new("Precise punto base", Want::Point),
            _ => Prompt::new("Precise segundo punto (o desplazamiento)", Want::Point),
        }
    }

    fn feed(&mut self, input: Input, cx: &Ctx<'_, dyn Geo + '_>, _st: &mut CadSettings) -> Step {
        let Input::Point(p) = input else {
            return if input == Input::Enter {
                Step::Done(None)
            } else {
                Step::Next
            };
        };
        if self.corners.len() < 2 {
            self.corners.push(p);
            return Step::Next;
        }
        if self.base.is_none() {
            self.base = Some(p);
            return Step::Next;
        }
        let changes = self.changes(cx, p);
        Step::Done((!changes.is_empty()).then_some(Changes {
            label: "Estirar",
            changes,
        }))
    }

    fn preview(&self, cx: &Ctx<'_, dyn Geo + '_>, _st: &CadSettings) -> Vec<Shape> {
        match (self.corners.as_slice(), self.base) {
            ([a], _) => vec![Shape::Polyline(ops::rectangle(*a, cx.cursor, 0.0))],
            ([_, _], Some(_)) => self
                .changes(cx, cx.cursor)
                .into_iter()
                .filter_map(|c| match c {
                    Change::Reshape(_, s) => Some(s),
                    _ => None,
                })
                .collect(),
            _ => Vec::new(),
        }
    }

    fn base(&self) -> Option<Point2> {
        self.base.or(self.corners.last().copied())
    }
}

// ---- LONGITUD -----------------------------------------------------------------------------

#[derive(Default)]
pub struct Lengthen {
    delta: Option<f64>,
}

impl Command for Lengthen {
    fn name(&self) -> &'static str {
        "LONGITUD"
    }

    fn prompt(&self) -> Prompt {
        match self.delta {
            None => Prompt::new(
                "Incremento de longitud (mm, negativo para acortar)",
                Want::Value,
            ),
            Some(_) => Prompt::new(
                "Designe el extremo a cambiar (Intro para terminar)",
                Want::Pick,
            ),
        }
    }

    fn feed(&mut self, input: Input, cx: &Ctx<'_, dyn Geo + '_>, _st: &mut CadSettings) -> Step {
        match (self.delta, input) {
            (None, Input::Value(v)) => match parse_length(&v) {
                Some(d) if d != 0.0 => {
                    self.delta = Some(d);
                    Step::Next
                }
                _ => Step::Refuse("Escriba una longitud en mm.".into()),
            },
            (Some(d), Input::Pick(Element::Draft(id), at)) => {
                let Some(shape) = cx.model().draft(id).map(|x| x.shape.clone()) else {
                    return Step::Next;
                };
                match ops::lengthen(&shape, at, d) {
                    Ok(s) => Step::Apply(Changes {
                        label: "Longitud",
                        changes: vec![Change::Reshape(id, s)],
                    }),
                    Err(e) => Step::Refuse(format!("No se puede: {e}.")),
                }
            }
            (_, Input::Enter) => Step::Done(None),
            _ => Step::Next,
        }
    }
}

// ---- IGUALARPROP ------------------------------------------------------------------------

#[derive(Default)]
pub struct MatchProp {
    from: Option<DraftId>,
}

impl Command for MatchProp {
    fn name(&self) -> &'static str {
        "IGUALARPROP"
    }

    fn prompt(&self) -> Prompt {
        match self.from {
            None => Prompt::new("Designe objeto de origen", Want::Pick),
            Some(_) => Prompt::new("Designe objetos de destino e Intro", Want::Select),
        }
    }

    fn feed(&mut self, input: Input, cx: &Ctx<'_, dyn Geo + '_>, _st: &mut CadSettings) -> Step {
        match (self.from, input) {
            (None, Input::Pick(Element::Draft(id), _)) => {
                self.from = Some(id);
                Step::Next
            }
            (None, Input::Pick(..)) => Step::Refuse("Elija un elemento de dibujo 2D.".into()),
            (Some(from), Input::Enter) => {
                let to: Vec<DraftId> = cx
                    .selected_drafts()
                    .into_iter()
                    .filter(|d| *d != from)
                    .collect();
                Step::Done((!to.is_empty()).then_some(Changes {
                    label: "Igualar propiedades",
                    changes: vec![Change::MatchStyle { from, to }],
                }))
            }
            (None, Input::Enter) => Step::Done(None),
            _ => Step::Next,
        }
    }

    fn picked(&self) -> Vec<Element> {
        self.from.map(Element::Draft).into_iter().collect()
    }
}

// ---- DIVIDE ------------------------------------------------------------------------------

#[derive(Default)]
pub struct Divide {
    object: Option<DraftId>,
}

impl Command for Divide {
    fn name(&self) -> &'static str {
        "DIVIDE"
    }

    fn prompt(&self) -> Prompt {
        match self.object {
            None => Prompt::new("Designe objeto a dividir", Want::Pick),
            Some(_) => Prompt::new("Número de segmentos", Want::Value),
        }
    }

    fn feed(&mut self, input: Input, cx: &Ctx<'_, dyn Geo + '_>, _st: &mut CadSettings) -> Step {
        match (self.object, input) {
            (None, Input::Pick(Element::Draft(id), _)) => {
                self.object = Some(id);
                Step::Next
            }
            (Some(id), Input::Value(v)) => {
                let Some(n) = v
                    .trim()
                    .parse::<usize>()
                    .ok()
                    .filter(|n| (2..=32767).contains(n))
                else {
                    return Step::Refuse("Entre 2 y 32767 segmentos.".into());
                };
                let Some(shape) = cx.model().draft(id).map(|d| d.shape.clone()) else {
                    return Step::Done(None);
                };
                let pts = ops::divide(&shape, n);
                let total = archisim_draft::length(&shape);
                let changes: Vec<Change> = pts
                    .into_iter()
                    .map(|at| Change::Add(Shape::Point { at }))
                    .collect();
                let _ = fmt_mm(total);
                Step::Done(Some(Changes {
                    label: "Dividir",
                    changes,
                }))
            }
            (_, Input::Enter) => Step::Done(None),
            _ => Step::Next,
        }
    }
}
