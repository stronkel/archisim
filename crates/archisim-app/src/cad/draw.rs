//! Drawing commands: LINEA, POL, RECTANG, POLIGONO, ARCO, CIRCULO, ELIPSE,
//! PUNTO, TEXTO, SOMBCONT, NUBEREV and DIRECTRIZ.

use std::f64::consts::TAU;

use archisim_core::{Hatch, Point2, Polyline, Shape, Text, TextAlign};
use archisim_draft::ops::{fillet_polyline, polygon, rectangle, revision_cloud, seg_shape};
use archisim_draft::region::boundary_at;
use archisim_draft::seg::{angle_of, dir};
use archisim_draft::Seg;

use super::*;
use crate::units::parse_length;

fn add(label: &'static str, shapes: Vec<Shape>) -> Changes {
    Changes {
        label,
        changes: shapes.into_iter().map(Change::Add).collect(),
    }
}

// ---- LINEA ------------------------------------------------------------------

/// Lines from point to point; each segment is its own line (AutoCAD's
/// LINEA). They are made when the command ends (Enter, Esc, Cerrar).
#[derive(Default)]
pub struct Line {
    points: Vec<Point2>,
}

impl Line {
    fn shapes(&self) -> Vec<Shape> {
        self.points
            .windows(2)
            .filter(|w| w[0].distance(w[1]) > 1e-9)
            .map(|w| Shape::Line { a: w[0], b: w[1] })
            .collect()
    }
}

impl Command for Line {
    fn name(&self) -> &'static str {
        "LINEA"
    }

    fn prompt(&self) -> Prompt {
        match self.points.len() {
            0 => Prompt::new("Precise primer punto", Want::Point),
            1 | 2 => {
                Prompt::new("Precise punto siguiente", Want::Point).opts(&[opt("H", "desHacer")])
            }
            _ => Prompt::new("Precise punto siguiente", Want::Point)
                .opts(&[opt("C", "Cerrar"), opt("H", "desHacer")]),
        }
    }

    fn feed(&mut self, input: Input, _cx: &Ctx<'_, dyn Geo + '_>, _st: &mut CadSettings) -> Step {
        match input {
            Input::Point(p) => {
                if self.points.last().is_some_and(|l| l.distance(p) < 1e-9) {
                    return Step::Refuse("El punto es el mismo que el anterior.".into());
                }
                self.points.push(p);
                Step::Next
            }
            Input::Option("H") => {
                self.points.pop();
                Step::Next
            }
            Input::Option("C") if self.points.len() >= 3 => {
                let first = self.points[0];
                self.points.push(first);
                let shapes = self.shapes();
                self.points.clear();
                Step::Done(Some(add("Líneas", shapes)))
            }
            Input::Enter => {
                let shapes = self.shapes();
                self.points.clear();
                Step::Done((!shapes.is_empty()).then(|| add("Líneas", shapes)))
            }
            _ => Step::Refuse("Precise un punto.".into()),
        }
    }

    fn finish(&mut self, _cx: &Ctx<'_, dyn Geo + '_>, _st: &mut CadSettings) -> Option<Changes> {
        let shapes = self.shapes();
        self.points.clear();
        (!shapes.is_empty()).then(|| add("Líneas", shapes))
    }

    fn preview(&self, cx: &Ctx<'_, dyn Geo + '_>, _st: &CadSettings) -> Vec<Shape> {
        let mut v = self.shapes();
        if let Some(last) = self.points.last() {
            if last.distance(cx.cursor) > 1e-9 {
                v.push(Shape::Line {
                    a: *last,
                    b: cx.cursor,
                });
            }
        }
        v
    }

    fn base(&self) -> Option<Point2> {
        self.points.last().copied()
    }
}

// ---- POL ---------------------------------------------------------------------

/// A polyline of straight and arc segments (AutoCAD's POL): arcs follow on
/// tangent to the previous segment.
#[derive(Default)]
pub struct Pline {
    points: Vec<Point2>,
    bulges: Vec<f64>,
    arc: bool,
    width_asked: bool,
}

impl Pline {
    /// Direction of travel at the last point.
    fn tangent(&self) -> Point2 {
        let n = self.points.len();
        if n < 2 {
            return Point2::new(1.0, 0.0);
        }
        let s = Seg::from_bulge(self.points[n - 2], self.points[n - 1], self.bulges[n - 2]);
        s.tangent(1.0)
    }

    /// The bulge of an arc from the last point to `p`, tangent to the
    /// previous segment.
    fn arc_bulge(&self, p: Point2) -> f64 {
        let Some(last) = self.points.last() else {
            return 0.0;
        };
        let chord = p - *last;
        let t = self.tangent();
        // The arc turns twice the angle between tangent and chord.
        let half = t.cross(chord).atan2(t.dot(chord));
        (half * 2.0 / 4.0).tan()
    }

    fn polyline(&self, closed: bool, st: &CadSettings) -> Option<Polyline> {
        if self.points.len() < 2 {
            return None;
        }
        let mut bulges = self.bulges.clone();
        bulges.push(0.0);
        Some(Polyline {
            points: self.points.clone(),
            bulges,
            closed,
            width: st.polyline_width,
        })
    }
}

impl Command for Pline {
    fn name(&self) -> &'static str {
        "POL"
    }

    fn prompt(&self) -> Prompt {
        if self.width_asked {
            return Prompt::new("Anchura de la polilínea (mm)", Want::Value);
        }
        if self.points.is_empty() {
            return Prompt::new("Precise punto inicial", Want::Point);
        }
        let mut o = vec![if self.arc {
            opt("L", "Línea")
        } else {
            opt("A", "Arco")
        }];
        if self.points.len() >= 2 {
            o.push(opt("C", "Cerrar"));
        }
        o.push(opt("N", "aNchura"));
        o.push(opt("H", "desHacer"));
        let what = if self.arc {
            "Precise punto final del arco"
        } else {
            "Precise punto siguiente"
        };
        Prompt::new(what, Want::Point).opts(&o)
    }

    fn feed(&mut self, input: Input, _cx: &Ctx<'_, dyn Geo + '_>, st: &mut CadSettings) -> Step {
        if self.width_asked {
            self.width_asked = false;
            return match input {
                Input::Value(v) => match parse_length(&v) {
                    Some(w) if w >= 0.0 => {
                        st.polyline_width = w;
                        Step::Next
                    }
                    _ => Step::Refuse("Escriba una anchura en mm (0: línea fina).".into()),
                },
                _ => Step::Next,
            };
        }
        match input {
            Input::Point(p) => {
                if self.points.last().is_some_and(|l| l.distance(p) < 1e-9) {
                    return Step::Refuse("El punto es el mismo que el anterior.".into());
                }
                if !self.points.is_empty() {
                    let b = if self.arc { self.arc_bulge(p) } else { 0.0 };
                    self.bulges.push(b);
                }
                self.points.push(p);
                Step::Next
            }
            Input::Option("A") => {
                self.arc = true;
                Step::Next
            }
            Input::Option("L") => {
                self.arc = false;
                Step::Next
            }
            Input::Option("N") => {
                self.width_asked = true;
                Step::Next
            }
            Input::Option("H") => {
                self.points.pop();
                self.bulges.pop();
                Step::Next
            }
            Input::Option("C") => match self.polyline(true, st) {
                Some(p) => {
                    self.points.clear();
                    self.bulges.clear();
                    Step::Done(Some(add("Polilínea", vec![Shape::Polyline(p)])))
                }
                None => Step::Refuse("Faltan puntos para cerrar.".into()),
            },
            Input::Enter => {
                let p = self.polyline(false, st);
                self.points.clear();
                self.bulges.clear();
                Step::Done(p.map(|p| add("Polilínea", vec![Shape::Polyline(p)])))
            }
            _ => Step::Refuse("Precise un punto.".into()),
        }
    }

    fn finish(&mut self, _cx: &Ctx<'_, dyn Geo + '_>, st: &mut CadSettings) -> Option<Changes> {
        let p = self.polyline(false, st);
        self.points.clear();
        self.bulges.clear();
        p.map(|p| add("Polilínea", vec![Shape::Polyline(p)]))
    }

    fn preview(&self, cx: &Ctx<'_, dyn Geo + '_>, st: &CadSettings) -> Vec<Shape> {
        let mut v = Vec::new();
        if let Some(p) = self.polyline(false, st) {
            v.push(Shape::Polyline(p));
        }
        if let Some(last) = self.points.last() {
            if last.distance(cx.cursor) > 1e-9 {
                let b = if self.arc {
                    self.arc_bulge(cx.cursor)
                } else {
                    0.0
                };
                v.push(seg_shape(&Seg::from_bulge(*last, cx.cursor, b)));
            }
        }
        v
    }

    fn base(&self) -> Option<Point2> {
        self.points.last().copied()
    }
}

// ---- SPLINE -------------------------------------------------------------------

/// A free curve through the points clicked: open (Enter) or closed (C, or a
/// click on its first point). Kept as a polyline finely following the
/// curve, as the spline of the model tools.
#[derive(Default)]
pub struct SplineCmd {
    points: Vec<Point2>,
}

impl SplineCmd {
    fn polyline(points: &[Point2], closed: bool, st: &CadSettings) -> Option<Polyline> {
        if points.len() < 2 || (closed && points.len() < 3) {
            return None;
        }
        // The same curve as Arista's spline (Catmull-Rom, finely sampled).
        let mut pts = crate::spline::catmull_rom(points, closed, 12);
        pts.dedup_by(|a, b| a.distance(*b) < 1e-9);
        if closed && pts.len() > 1 && pts[0].distance(pts[pts.len() - 1]) < 1e-9 {
            pts.pop();
        }
        let bulges = vec![0.0; pts.len()];
        Some(Polyline {
            points: pts,
            bulges,
            closed,
            width: st.polyline_width,
        })
    }

    fn done(&mut self, closed: bool, st: &CadSettings) -> Step {
        let p = Self::polyline(&self.points, closed, st);
        self.points.clear();
        Step::Done(p.map(|p| add("Spline", vec![Shape::Polyline(p)])))
    }
}

impl Command for SplineCmd {
    fn name(&self) -> &'static str {
        "SPLINE"
    }

    fn prompt(&self) -> Prompt {
        if self.points.is_empty() {
            return Prompt::new("Precise primer punto de la curva", Want::Point);
        }
        let mut o = Vec::new();
        if self.points.len() >= 3 {
            o.push(opt("C", "Cerrar"));
        }
        o.push(opt("H", "desHacer"));
        Prompt::new("Precise punto siguiente (Intro: terminar)", Want::Point).opts(&o)
    }

    fn feed(&mut self, input: Input, cx: &Ctx<'_, dyn Geo + '_>, st: &mut CadSettings) -> Step {
        match input {
            Input::Point(p) => {
                if self.points.last().is_some_and(|l| l.distance(p) < 1e-9) {
                    return Step::Refuse("El punto es el mismo que el anterior.".into());
                }
                // Back on its first point: closed.
                if self.points.len() >= 3 && self.points[0].distance(p) <= 1.5 * cx.k {
                    return self.done(true, st);
                }
                self.points.push(p);
                Step::Next
            }
            Input::Option("C") => {
                if self.points.len() < 3 {
                    return Step::Refuse("Faltan puntos para cerrar.".into());
                }
                self.done(true, st)
            }
            Input::Option("H") => {
                self.points.pop();
                Step::Next
            }
            Input::Enter => self.done(false, st),
            _ => Step::Refuse("Precise un punto.".into()),
        }
    }

    fn finish(&mut self, _cx: &Ctx<'_, dyn Geo + '_>, st: &mut CadSettings) -> Option<Changes> {
        let p = Self::polyline(&self.points, false, st);
        self.points.clear();
        p.map(|p| add("Spline", vec![Shape::Polyline(p)]))
    }

    fn preview(&self, cx: &Ctx<'_, dyn Geo + '_>, st: &CadSettings) -> Vec<Shape> {
        let mut pts = self.points.clone();
        if pts.last().is_some_and(|l| l.distance(cx.cursor) > 1e-9) {
            pts.push(cx.cursor);
        }
        Self::polyline(&pts, false, st)
            .map(|p| vec![Shape::Polyline(p)])
            .unwrap_or_default()
    }

    fn base(&self) -> Option<Point2> {
        self.points.last().copied()
    }
}

// ---- RECTANG ------------------------------------------------------------------

#[derive(Default)]
pub struct Rectangle {
    first: Option<Point2>,
    angle: f64,
    asking: Option<&'static str>,
}

impl Rectangle {
    fn shape(&self, b: Point2, st: &CadSettings) -> Option<Shape> {
        let a = self.first?;
        let r = rectangle(a, b, self.angle);
        let r = if st.fillet_radius > 0.0 {
            fillet_polyline(&r, st.fillet_radius, false).unwrap_or(r)
        } else {
            r
        };
        Some(Shape::Polyline(Polyline {
            width: st.polyline_width,
            ..r
        }))
    }
}

impl Command for Rectangle {
    fn name(&self) -> &'static str {
        "RECTANG"
    }

    fn prompt(&self) -> Prompt {
        match self.asking {
            Some("G") => return Prompt::new("Ángulo de giro del rectángulo (°)", Want::Value),
            Some("E") => return Prompt::new("Radio de empalme de las esquinas (mm)", Want::Value),
            Some("D") => return Prompt::new("Dimensiones: ancho x alto (mm)", Want::Value),
            _ => {}
        }
        match self.first {
            None => {
                Prompt::new("Precise primera esquina", Want::Point).opts(&[opt("E", "Empalme")])
            }
            Some(_) => Prompt::new("Precise esquina opuesta", Want::Point)
                .opts(&[opt("D", "Dimensiones"), opt("G", "Giro")]),
        }
    }

    fn feed(&mut self, input: Input, cx: &Ctx<'_, dyn Geo + '_>, st: &mut CadSettings) -> Step {
        if let Some(asked) = self.asking.take() {
            let Input::Value(v) = &input else {
                return Step::Next;
            };
            match asked {
                "G" => match parse_angle(v) {
                    Some(a) => self.angle = a,
                    None => return Step::Refuse("Escriba un ángulo en grados.".into()),
                },
                "E" => match parse_length(v) {
                    Some(r) if r >= 0.0 => st.fillet_radius = r,
                    _ => return Step::Refuse("Escriba un radio en mm.".into()),
                },
                _ => {
                    let Some((w, h)) = crate::units::parse_size(v) else {
                        return Step::Refuse(
                            "Escriba ancho x alto, por ejemplo 6x4 (en metros).".into(),
                        );
                    };
                    let a = self.first.unwrap_or_default();
                    // Towards the side of the cursor.
                    let d = cx.cursor - a;
                    let (u, n) = (dir(self.angle), dir(self.angle).perp_left());
                    let sx = if d.dot(u) < 0.0 { -1.0 } else { 1.0 };
                    let sy = if d.dot(n) < 0.0 { -1.0 } else { 1.0 };
                    let b = a + u * (w * sx) + n * (h * sy);
                    let s = self.shape(b, st);
                    return Step::Done(s.map(|s| add("Rectángulo", vec![s])));
                }
            }
            return Step::Next;
        }
        match input {
            Input::Option(k @ ("G" | "E" | "D")) => {
                self.asking = Some(k);
                Step::Next
            }
            Input::Point(p) => match self.first {
                None => {
                    self.first = Some(p);
                    Step::Next
                }
                Some(a) => {
                    let d = p - a;
                    let u = dir(self.angle);
                    if d.dot(u).abs() < 1e-6 || d.dot(u.perp_left()).abs() < 1e-6 {
                        return Step::Refuse("El rectángulo no tendría superficie.".into());
                    }
                    let s = self.shape(p, st);
                    Step::Done(s.map(|s| add("Rectángulo", vec![s])))
                }
            },
            Input::Enter => Step::Done(None),
            _ => Step::Refuse("Precise un punto.".into()),
        }
    }

    fn preview(&self, cx: &Ctx<'_, dyn Geo + '_>, st: &CadSettings) -> Vec<Shape> {
        self.shape(cx.cursor, st).into_iter().collect()
    }

    fn base(&self) -> Option<Point2> {
        self.first
    }
}

// ---- POLIGONO ------------------------------------------------------------------

#[derive(Default)]
pub struct Polygon {
    sides: Option<usize>,
    centre: Option<Point2>,
    circumscribed: Option<bool>,
}

impl Command for Polygon {
    fn name(&self) -> &'static str {
        "POLIGONO"
    }

    fn prompt(&self) -> Prompt {
        if self.sides.is_none() {
            return Prompt::new("Número de lados", Want::Value);
        }
        if self.centre.is_none() {
            return Prompt::new("Precise centro del polígono", Want::Point);
        }
        if self.circumscribed.is_none() {
            return Prompt::new("Opción", Want::Value)
                .opts(&[opt("I", "Inscrito en el círculo"), opt("C", "Circunscrito")])
                .default("I");
        }
        Prompt::new("Radio del círculo (punto o mm)", Want::Point)
    }

    fn feed(&mut self, input: Input, cx: &Ctx<'_, dyn Geo + '_>, st: &mut CadSettings) -> Step {
        if self.sides.is_none() {
            return match input {
                Input::Enter => {
                    self.sides = Some(st.polygon_sides);
                    Step::Next
                }
                Input::Value(v) => match v.trim().parse::<usize>() {
                    Ok(n) if (3..=1024).contains(&n) => {
                        self.sides = Some(n);
                        st.polygon_sides = n;
                        Step::Next
                    }
                    _ => Step::Refuse("Entre 3 y 1024 lados.".into()),
                },
                _ => Step::Refuse("Escriba el número de lados.".into()),
            };
        }
        let Some(c) = self.centre else {
            return match input {
                Input::Point(p) => {
                    self.centre = Some(p);
                    Step::Next
                }
                _ => Step::Refuse("Precise el centro.".into()),
            };
        };
        if self.circumscribed.is_none() {
            self.circumscribed = Some(matches!(input, Input::Option("C")));
            return Step::Next;
        }
        let towards = match input {
            Input::Point(p) => p,
            Input::Value(v) => match parse_length(&v) {
                Some(r) if r > 0.0 => {
                    let d = (cx.cursor - c)
                        .normalized()
                        .unwrap_or(Point2::new(0.0, 1.0));
                    c + d * r
                }
                _ => return Step::Refuse("Escriba un radio en mm.".into()),
            },
            _ => return Step::Next,
        };
        let p = polygon(
            c,
            towards,
            self.sides.unwrap_or(6),
            !self.circumscribed.unwrap_or(false),
        );
        Step::Done(p.map(|p| add("Polígono", vec![Shape::Polyline(p)])))
    }

    fn preview(&self, cx: &Ctx<'_, dyn Geo + '_>, _st: &CadSettings) -> Vec<Shape> {
        match (self.sides, self.centre, self.circumscribed) {
            (Some(n), Some(c), Some(circ)) => polygon(c, cx.cursor, n, !circ)
                .map(Shape::Polyline)
                .into_iter()
                .collect(),
            _ => Vec::new(),
        }
    }

    fn base(&self) -> Option<Point2> {
        self.centre
    }
}

// ---- ARCO ------------------------------------------------------------------------

/// Arcs by three points (default) or by centre, start and end.
#[derive(Default)]
pub struct ArcCmd {
    by_centre: bool,
    points: Vec<Point2>,
}

impl ArcCmd {
    fn arc(&self, p: Point2) -> Option<Shape> {
        match (self.by_centre, self.points.as_slice()) {
            (false, [a, b]) => archisim_core::Arc::bulge_through(*a, *b, p)
                .map(|bl| seg_shape(&Seg::from_bulge(*a, p, bl))),
            (true, [c, s]) => {
                let r = c.distance(*s);
                let a0 = angle_of(*s - *c);
                let mut sweep = (angle_of(p - *c) - a0).rem_euclid(TAU);
                if sweep < 1e-6 {
                    sweep = TAU - 1e-6;
                }
                (r > 1e-9).then_some(Shape::Arc {
                    centre: *c,
                    radius: r,
                    start: a0,
                    sweep,
                })
            }
            _ => None,
        }
    }
}

impl Command for ArcCmd {
    fn name(&self) -> &'static str {
        "ARCO"
    }

    fn prompt(&self) -> Prompt {
        match (self.by_centre, self.points.len()) {
            (false, 0) => Prompt::new("Precise punto inicial del arco", Want::Point)
                .opts(&[opt("C", "Centro")]),
            (false, 1) => Prompt::new("Precise segundo punto del arco", Want::Point),
            (false, _) => Prompt::new("Precise punto final del arco", Want::Point),
            (true, 0) => Prompt::new("Precise centro del arco", Want::Point),
            (true, 1) => Prompt::new("Precise punto inicial del arco", Want::Point),
            (true, _) => Prompt::new(
                "Precise punto final del arco (en sentido antihorario)",
                Want::Point,
            ),
        }
    }

    fn feed(&mut self, input: Input, _cx: &Ctx<'_, dyn Geo + '_>, _st: &mut CadSettings) -> Step {
        match input {
            Input::Option("C") if self.points.is_empty() => {
                self.by_centre = true;
                Step::Next
            }
            Input::Point(p) => {
                if self.points.len() < 2 {
                    self.points.push(p);
                    return Step::Next;
                }
                match self.arc(p) {
                    Some(a) => Step::Done(Some(add("Arco", vec![a]))),
                    None => Step::Refuse("Los tres puntos están en línea.".into()),
                }
            }
            Input::Enter => Step::Done(None),
            _ => Step::Refuse("Precise un punto.".into()),
        }
    }

    fn preview(&self, cx: &Ctx<'_, dyn Geo + '_>, _st: &CadSettings) -> Vec<Shape> {
        match self.points.as_slice() {
            [a] => vec![Shape::Line {
                a: *a,
                b: cx.cursor,
            }],
            [_, _] => self.arc(cx.cursor).into_iter().collect(),
            _ => Vec::new(),
        }
    }

    fn base(&self) -> Option<Point2> {
        self.points.last().copied()
    }
}

// ---- CIRCULO -----------------------------------------------------------------------

#[derive(Default)]
pub struct Circle {
    mode: Option<&'static str>,
    points: Vec<Point2>,
    diameter: bool,
}

impl Circle {
    fn circle(&self, p: Point2) -> Option<Shape> {
        let circ = |c: Point2, r: f64| {
            (r > 1e-9).then_some(Shape::Circle {
                centre: c,
                radius: r,
            })
        };
        match (self.mode, self.points.as_slice()) {
            (None, [c]) => {
                let d = c.distance(p);
                circ(*c, if self.diameter { d / 2.0 } else { d })
            }
            (Some("2P"), [a]) => circ((*a + p) * 0.5, a.distance(p) / 2.0),
            (Some("3P"), [a, b]) => {
                // The circle through three points: centre where the
                // perpendicular bisectors meet.
                let m1 = (*a + *b) * 0.5;
                let m2 = (*b + p) * 0.5;
                let l1 = Seg::Line {
                    a: m1,
                    b: m1 + (*b - *a).perp_left(),
                };
                let l2 = Seg::Line {
                    a: m2,
                    b: m2 + (p - *b).perp_left(),
                };
                let c = archisim_draft::intersect_extended(&l1, &l2).first()?.2;
                circ(c, c.distance(*a))
            }
            _ => None,
        }
    }
}

impl Command for Circle {
    fn name(&self) -> &'static str {
        "CIRCULO"
    }

    fn prompt(&self) -> Prompt {
        match (self.mode, self.points.len()) {
            (None, 0) => Prompt::new("Precise centro del círculo", Want::Point)
                .opts(&[opt("3P", "3P"), opt("2P", "2P")]),
            (None, _) if self.diameter => Prompt::new("Precise diámetro del círculo", Want::Point),
            (None, _) => {
                Prompt::new("Precise radio del círculo", Want::Point).opts(&[opt("D", "Diámetro")])
            }
            (Some(_), n) => Prompt::new(
                match n {
                    0 => "Precise primer punto del círculo",
                    1 => "Precise segundo punto del círculo",
                    _ => "Precise tercer punto del círculo",
                },
                Want::Point,
            ),
        }
    }

    fn feed(&mut self, input: Input, cx: &Ctx<'_, dyn Geo + '_>, _st: &mut CadSettings) -> Step {
        let need = match self.mode {
            None => 1,
            Some("2P") => 1,
            _ => 2,
        };
        match input {
            Input::Option(k @ ("3P" | "2P")) if self.points.is_empty() => {
                self.mode = Some(k);
                Step::Next
            }
            Input::Option("D") => {
                self.diameter = true;
                Step::Next
            }
            Input::Value(v) if self.mode.is_none() && self.points.len() == 1 => {
                match parse_length(&v) {
                    Some(r) if r > 0.0 => {
                        let c = self.points[0];
                        let r = if self.diameter { r / 2.0 } else { r };
                        Step::Done(Some(add(
                            "Círculo",
                            vec![Shape::Circle {
                                centre: c,
                                radius: r,
                            }],
                        )))
                    }
                    _ => Step::Refuse("Escriba el radio en mm.".into()),
                }
            }
            Input::Point(p) => {
                if self.points.len() < need {
                    self.points.push(p);
                    return Step::Next;
                }
                match self.circle(p) {
                    Some(c) => Step::Done(Some(add("Círculo", vec![c]))),
                    None => Step::Refuse("Esos puntos no definen un círculo.".into()),
                }
            }
            Input::Enter => Step::Done(None),
            _ => {
                let _ = cx;
                Step::Refuse("Precise un punto.".into())
            }
        }
    }

    fn preview(&self, cx: &Ctx<'_, dyn Geo + '_>, _st: &CadSettings) -> Vec<Shape> {
        self.circle(cx.cursor).into_iter().collect()
    }

    fn base(&self) -> Option<Point2> {
        self.points.last().copied()
    }
}

// ---- ELIPSE ------------------------------------------------------------------------

/// An ellipse by the ends of one axis and the other half-axis, or by its
/// centre (option Centro).
#[derive(Default)]
pub struct Ellipse {
    by_centre: bool,
    points: Vec<Point2>,
}

impl Ellipse {
    fn ellipse(&self, p: Point2) -> Option<Shape> {
        let (c, end) = match (self.by_centre, self.points.as_slice()) {
            (false, [a, b]) => ((*a + *b) * 0.5, *b),
            (true, [c, e]) => (*c, *e),
            _ => return None,
        };
        let axis = end - c;
        let a = axis.length();
        if a < 1e-9 {
            return None;
        }
        // The other half-axis: the distance from the axis line.
        let u = axis * (1.0 / a);
        let b = (p - c).cross(u).abs();
        if b < 1e-9 {
            return None;
        }
        let (axis, ratio) = if b <= a {
            (axis, b / a)
        } else {
            (u.perp_left() * b, a / b)
        };
        Some(Shape::Ellipse {
            centre: c,
            axis,
            ratio,
            start: 0.0,
            sweep: TAU,
        })
    }
}

impl Command for Ellipse {
    fn name(&self) -> &'static str {
        "ELIPSE"
    }

    fn prompt(&self) -> Prompt {
        match (self.by_centre, self.points.len()) {
            (false, 0) => Prompt::new("Precise punto final del eje de la elipse", Want::Point)
                .opts(&[opt("C", "Centro")]),
            (false, 1) => Prompt::new("Precise otro punto final del eje", Want::Point),
            (true, 0) => Prompt::new("Precise centro de la elipse", Want::Point),
            (true, 1) => Prompt::new("Precise punto final del eje", Want::Point),
            _ => Prompt::new("Precise distancia al otro eje", Want::Point),
        }
    }

    fn feed(&mut self, input: Input, cx: &Ctx<'_, dyn Geo + '_>, _st: &mut CadSettings) -> Step {
        match input {
            Input::Option("C") if self.points.is_empty() => {
                self.by_centre = true;
                Step::Next
            }
            Input::Value(v) if self.points.len() == 2 => {
                let Some(d) = parse_length(&v) else {
                    return Step::Refuse("Escriba una distancia en mm.".into());
                };
                let c = if self.by_centre {
                    self.points[0]
                } else {
                    (self.points[0] + self.points[1]) * 0.5
                };
                let u = (self.points[1] - c)
                    .normalized()
                    .unwrap_or(Point2::new(1.0, 0.0));
                let e = self.ellipse(c + u.perp_left() * d);
                let _ = cx;
                Step::Done(e.map(|e| add("Elipse", vec![e])))
            }
            Input::Point(p) => {
                if self.points.len() < 2 {
                    self.points.push(p);
                    return Step::Next;
                }
                match self.ellipse(p) {
                    Some(e) => Step::Done(Some(add("Elipse", vec![e]))),
                    None => Step::Refuse("La elipse no tendría anchura.".into()),
                }
            }
            Input::Enter => Step::Done(None),
            _ => Step::Refuse("Precise un punto.".into()),
        }
    }

    fn preview(&self, cx: &Ctx<'_, dyn Geo + '_>, _st: &CadSettings) -> Vec<Shape> {
        match self.points.as_slice() {
            [a] => vec![Shape::Line {
                a: *a,
                b: cx.cursor,
            }],
            _ => self.ellipse(cx.cursor).into_iter().collect(),
        }
    }

    fn base(&self) -> Option<Point2> {
        self.points.last().copied()
    }
}

// ---- PUNTO -----------------------------------------------------------------------------

pub struct PointCmd;

impl Command for PointCmd {
    fn name(&self) -> &'static str {
        "PUNTO"
    }

    fn prompt(&self) -> Prompt {
        Prompt::new("Precise un punto", Want::Point)
    }

    fn feed(&mut self, input: Input, _cx: &Ctx<'_, dyn Geo + '_>, _st: &mut CadSettings) -> Step {
        match input {
            Input::Point(p) => Step::Apply(add("Punto", vec![Shape::Point { at: p }])),
            Input::Enter => Step::Done(None),
            _ => Step::Refuse("Precise un punto.".into()),
        }
    }
}

// ---- TEXTO ----------------------------------------------------------------------------------

/// A text: insertion point, height, angle, then the text itself (written in
/// a box; several lines allowed).
#[derive(Default)]
pub struct TextCmd {
    at: Option<Point2>,
    height: Option<f64>,
    angle: Option<f64>,
}

impl Command for TextCmd {
    fn name(&self) -> &'static str {
        "TEXTO"
    }

    fn prompt(&self) -> Prompt {
        if self.at.is_none() {
            return Prompt::new("Precise punto de inserción del texto", Want::Point)
                .opts(&[opt("J", "Justificar")]);
        }
        if self.height.is_none() {
            return Prompt::new("Altura del texto en el papel (mm)", Want::Value);
        }
        if self.angle.is_none() {
            return Prompt::new("Ángulo de rotación del texto (° o punto)", Want::Point)
                .default("0");
        }
        Prompt::new("Escriba el texto", Want::Text)
    }

    fn feed(&mut self, input: Input, cx: &Ctx<'_, dyn Geo + '_>, st: &mut CadSettings) -> Step {
        let Some(at) = self.at else {
            return match input {
                Input::Point(p) => {
                    self.at = Some(p);
                    Step::Next
                }
                Input::Option("J") => {
                    // Cycles the nine justifications.
                    let i = TextAlign::ALL
                        .iter()
                        .position(|a| *a == st.text_align)
                        .unwrap_or(0);
                    st.text_align = TextAlign::ALL[(i + 1) % 9];
                    Step::Say(format!("Justificación: {}", st.text_align.label()))
                }
                _ => Step::Refuse("Precise el punto de inserción.".into()),
            };
        };
        if self.height.is_none() {
            return match input {
                Input::Enter => {
                    self.height = Some(st.text_height);
                    Step::Next
                }
                Input::Value(v) => match parse_number(&v) {
                    Some(h) if h > 0.0 => {
                        st.text_height = h;
                        self.height = Some(h);
                        Step::Next
                    }
                    _ => Step::Refuse("Escriba la altura en mm de papel (2,5 · 3,5 · 5).".into()),
                },
                _ => Step::Next,
            };
        }
        if self.angle.is_none() {
            self.angle = Some(match input {
                Input::Point(p) if p.distance(at) > 1e-9 => angle_of(p - at),
                Input::Value(v) => match parse_angle(&v) {
                    Some(a) => a,
                    None => return Step::Refuse("Escriba un ángulo en grados.".into()),
                },
                _ => 0.0,
            });
            return Step::Next;
        }
        match input {
            Input::Value(t) if !t.trim().is_empty() => {
                let text = Shape::Text(Text {
                    at,
                    content: t.trim_end().to_string(),
                    height: self.height.unwrap_or(st.text_height),
                    angle: self.angle.unwrap_or(0.0),
                    align: st.text_align,
                });
                // Ready for the next text below, as AutoCAD does.
                let h = self.height.unwrap_or(st.text_height)
                    * 1.6
                    * cx.k
                    * t.lines().count().max(1) as f64;
                let down = dir(self.angle.unwrap_or(0.0)).perp_left() * -h;
                self.at = Some(at + down);
                Step::Apply(add("Texto", vec![text]))
            }
            _ => Step::Done(None),
        }
    }

    fn preview(&self, cx: &Ctx<'_, dyn Geo + '_>, st: &CadSettings) -> Vec<Shape> {
        let at = self.at.unwrap_or(cx.cursor);
        let angle = match (self.at, self.angle) {
            (_, Some(a)) => a,
            (Some(a), None) if self.height.is_some() && a.distance(cx.cursor) > 1e-9 => {
                angle_of(cx.cursor - a)
            }
            _ => 0.0,
        };
        vec![Shape::Text(Text {
            at,
            content: "Texto".into(),
            height: self.height.unwrap_or(st.text_height),
            angle,
            align: st.text_align,
        })]
    }

    fn base(&self) -> Option<Point2> {
        self.at
    }
}

// ---- SOMBCONT -------------------------------------------------------------------------------

/// Hatches: a click inside a closed area (made of drafting lines, wall
/// faces or slab edges) or chosen closed objects.
#[derive(Default)]
pub struct HatchCmd {
    asking: Option<&'static str>,
    selecting: bool,
}

impl HatchCmd {
    fn hatch(&self, loops: Vec<Polyline>, st: &CadSettings) -> Shape {
        Shape::Hatch(Hatch {
            loops,
            pattern: st.hatch_pattern,
            scale: st.hatch_scale,
            angle: st.hatch_angle.to_radians(),
            background: st.hatch_background,
            opacity: st.hatch_opacity,
        })
    }
}

/// The closed loop a shape encloses, if it is closed.
fn closed_loop(shape: &Shape) -> Option<Polyline> {
    match shape {
        Shape::Polyline(p) if p.closed => Some(Polyline {
            width: 0.0,
            ..p.clone()
        }),
        Shape::Circle { centre, radius } => Some(Polyline {
            points: vec![
                *centre + Point2::new(*radius, 0.0),
                *centre - Point2::new(*radius, 0.0),
            ],
            bulges: vec![1.0, 1.0],
            closed: true,
            width: 0.0,
        }),
        Shape::Ellipse {
            centre,
            axis,
            ratio,
            sweep,
            ..
        } if *sweep >= TAU - 1e-9 => {
            let mut pts = archisim_draft::ellipse_points(*centre, *axis, *ratio, 0.0, TAU, 1e-4);
            pts.pop();
            Some(Polyline::straight(pts, true))
        }
        Shape::Hatch(h) => h.loops.first().cloned(),
        _ => None,
    }
}

impl Command for HatchCmd {
    fn name(&self) -> &'static str {
        "SOMBCONT"
    }

    fn prompt(&self) -> Prompt {
        match self.asking {
            Some("P") => {
                let mut p = Prompt::new("Patrón", Want::Value);
                p.options = archisim_core::HatchPattern::ALL
                    .iter()
                    .map(|h| opt(h.code(), h.label()))
                    .collect();
                return p;
            }
            Some("E") => return Prompt::new("Escala del patrón", Want::Value),
            Some("N") => return Prompt::new("Ángulo del patrón (°)", Want::Value),
            _ => {}
        }
        if self.selecting {
            return Prompt::new(
                "Designe objetos cerrados (polilíneas, círculos) e Intro",
                Want::Select,
            );
        }
        Prompt::new("Designe punto interno", Want::Point).opts(&[
            opt("S", "Seleccionar objetos"),
            opt("P", "Patrón"),
            opt("E", "Escala"),
            opt("N", "áNgulo"),
        ])
    }

    fn feed(&mut self, input: Input, cx: &Ctx<'_, dyn Geo + '_>, st: &mut CadSettings) -> Step {
        if let Some(asked) = self.asking.take() {
            match (asked, input) {
                ("P", Input::Option(code)) => {
                    if let Some(p) = archisim_core::HatchPattern::from_code(code) {
                        st.hatch_pattern = p;
                    }
                }
                ("P", Input::Value(v)) => {
                    let p = archisim_core::HatchPattern::ALL
                        .into_iter()
                        .find(|p| p.label().to_lowercase().starts_with(&v.to_lowercase()));
                    match p {
                        Some(p) => st.hatch_pattern = p,
                        None => return Step::Refuse("No hay ningún patrón con ese nombre.".into()),
                    }
                }
                ("E", Input::Value(v)) => match parse_number(&v) {
                    Some(s) if s > 0.0 => st.hatch_scale = s,
                    _ => return Step::Refuse("Escriba una escala mayor que cero.".into()),
                },
                ("N", Input::Value(v)) => match parse_number(&v) {
                    Some(a) => st.hatch_angle = a,
                    None => return Step::Refuse("Escriba un ángulo en grados.".into()),
                },
                _ => {}
            }
            return Step::Next;
        }
        if self.selecting {
            return match input {
                Input::Enter => {
                    self.selecting = false;
                    let loops: Vec<Polyline> = cx
                        .selected_drafts()
                        .into_iter()
                        .filter_map(|id| cx.model().draft(id))
                        .filter_map(|d| closed_loop(&d.shape))
                        .collect();
                    if loops.is_empty() {
                        return Step::Refuse("Ninguno de los objetos está cerrado.".into());
                    }
                    Step::Apply(add("Sombreado", vec![self.hatch(loops, st)]))
                }
                _ => Step::Next,
            };
        }
        match input {
            Input::Option(k @ ("P" | "E" | "N")) => {
                self.asking = Some(k);
                Step::Next
            }
            Input::Option("S") => {
                self.selecting = true;
                Step::Next
            }
            Input::Point(p) => match boundary_at(&cx.edges(), p) {
                Ok(loops) => Step::Apply(add("Sombreado", vec![self.hatch(loops, st)])),
                Err(_) => Step::Refuse(
                    "No hay un contorno cerrado alrededor de ese punto (ciérralo o elige objetos)."
                        .into(),
                ),
            },
            Input::Enter => Step::Done(None),
            _ => Step::Refuse("Haga clic dentro de un recinto cerrado.".into()),
        }
    }
}

// ---- NUBEREV ---------------------------------------------------------------------------------

/// A revision cloud: a rectangle by two corners, or around a closed
/// object.
#[derive(Default)]
pub struct Cloud {
    first: Option<Point2>,
    object: bool,
}

impl Cloud {
    fn cloud(&self, b: Point2, cx: &Ctx<'_, dyn Geo + '_>, st: &CadSettings) -> Option<Shape> {
        let a = self.first?;
        let r = rectangle(a, b, 0.0);
        revision_cloud(&r.points, st.cloud_arc * cx.k).map(Shape::Polyline)
    }
}

impl Command for Cloud {
    fn name(&self) -> &'static str {
        "NUBEREV"
    }

    fn prompt(&self) -> Prompt {
        if self.object {
            return Prompt::new("Designe el objeto cerrado", Want::Pick);
        }
        match self.first {
            None => Prompt::new("Precise primera esquina", Want::Point).opts(&[opt("O", "Objeto")]),
            Some(_) => Prompt::new("Precise esquina opuesta", Want::Point),
        }
    }

    fn feed(&mut self, input: Input, cx: &Ctx<'_, dyn Geo + '_>, st: &mut CadSettings) -> Step {
        match input {
            Input::Option("O") => {
                self.object = true;
                Step::Next
            }
            Input::Pick(e, _) if self.object => {
                let Some(l) = cx.shape(e).and_then(closed_loop) else {
                    return Step::Refuse("Elija una polilínea cerrada o un círculo.".into());
                };
                let poly = archisim_draft::loop_polygon(&l, 0.01);
                let c = revision_cloud(&poly, st.cloud_arc * cx.k);
                Step::Done(c.map(|c| Changes {
                    label: "Nube de revisión",
                    changes: vec![Change::Remove(e), Change::Add(Shape::Polyline(c))],
                }))
            }
            Input::Point(p) => match self.first {
                None => {
                    self.first = Some(p);
                    Step::Next
                }
                Some(_) => {
                    let c = self.cloud(p, cx, st);
                    Step::Done(c.map(|c| add("Nube de revisión", vec![c])))
                }
            },
            Input::Enter => Step::Done(None),
            _ => Step::Refuse("Precise un punto.".into()),
        }
    }

    fn preview(&self, cx: &Ctx<'_, dyn Geo + '_>, st: &CadSettings) -> Vec<Shape> {
        self.cloud(cx.cursor, cx, st).into_iter().collect()
    }

    fn base(&self) -> Option<Point2> {
        self.first
    }
}

// ---- DIRECTRIZ -----------------------------------------------------------------------------------

/// A leader: an arrow at the first point, a line through the next ones and
/// a text at the end.
#[derive(Default)]
pub struct Leader {
    points: Vec<Point2>,
    text: bool,
}

/// The arrowhead of a leader at `tip`, pointing from `from`: a filled
/// triangle 2,5 × 0,8 mm on paper.
pub fn arrowhead(tip: Point2, from: Point2, k: f64) -> Option<Shape> {
    let d = (from - tip).normalized()?;
    let n = d.perp_left();
    let len = 2.5 * k;
    let half = 0.45 * k;
    let base = tip + d * len;
    Some(Shape::Hatch(Hatch {
        loops: vec![Polyline::straight(
            vec![tip, base + n * half, base - n * half],
            true,
        )],
        pattern: archisim_core::HatchPattern::Solid,
        scale: 1.0,
        angle: 0.0,
        background: None,
        opacity: 1.0,
    }))
}

impl Command for Leader {
    fn name(&self) -> &'static str {
        "DIRECTRIZ"
    }

    fn prompt(&self) -> Prompt {
        if self.text {
            return Prompt::new("Escriba el texto de la directriz", Want::Text);
        }
        match self.points.len() {
            0 => Prompt::new("Precise la punta de la flecha", Want::Point),
            1 => Prompt::new("Precise punto siguiente", Want::Point),
            _ => Prompt::new("Precise punto siguiente (Intro para el texto)", Want::Point),
        }
    }

    fn feed(&mut self, input: Input, cx: &Ctx<'_, dyn Geo + '_>, st: &mut CadSettings) -> Step {
        if self.text {
            let Input::Value(t) = input else {
                return Step::Done(None);
            };
            let n = self.points.len();
            let (a, b) = (self.points[n - 2], self.points[n - 1]);
            let right = b.x >= a.x;
            let gap = st.text_height * 0.5 * cx.k;
            let at = b + Point2::new(if right { gap } else { -gap }, 0.0);
            let mut shapes = vec![Shape::Polyline(Polyline::straight(
                self.points.clone(),
                false,
            ))];
            shapes.extend(arrowhead(self.points[0], self.points[1], cx.k));
            if !t.trim().is_empty() {
                shapes.push(Shape::Text(Text {
                    at,
                    content: t.trim_end().to_string(),
                    height: st.text_height,
                    angle: 0.0,
                    align: if right {
                        TextAlign::MiddleLeft
                    } else {
                        TextAlign::MiddleRight
                    },
                }));
            }
            return Step::Done(Some(add("Directriz", shapes)));
        }
        match input {
            Input::Point(p) => {
                self.points.push(p);
                Step::Next
            }
            Input::Enter if self.points.len() >= 2 => {
                self.text = true;
                Step::Next
            }
            Input::Enter => Step::Done(None),
            _ => Step::Refuse("Precise un punto.".into()),
        }
    }

    fn preview(&self, cx: &Ctx<'_, dyn Geo + '_>, _st: &CadSettings) -> Vec<Shape> {
        let mut pts = self.points.clone();
        if !self.text {
            pts.push(cx.cursor);
        }
        let mut v = Vec::new();
        if pts.len() >= 2 {
            v.push(Shape::Polyline(Polyline::straight(pts.clone(), false)));
            v.extend(arrowhead(pts[0], pts[1], cx.k));
        }
        v
    }

    fn base(&self) -> Option<Point2> {
        self.points.last().copied()
    }
}

// ---- Fills drawn directly (Relleno: rectangle, polygon, circle…) ---------------

/// A drawing order whose closed shape becomes a hatch with the current
/// pattern, scale, angle, background and opacity, instead of a line: the
/// fill is drawn at once, without a boundary first.
pub struct Filled {
    name: &'static str,
    label: &'static str,
    inner: Box<dyn Command>,
}

impl Filled {
    pub fn new(name: &'static str, label: &'static str, inner: Box<dyn Command>) -> Self {
        Self { name, label, inner }
    }

    /// The hatch a drawn shape becomes (an open polyline or spline is
    /// closed back to its first point).
    fn fill(shape: &Shape, st: &CadSettings) -> Option<Shape> {
        let ring = match shape {
            Shape::Polyline(p) if !p.closed && p.points.len() >= 3 => Some(Polyline {
                closed: true,
                width: 0.0,
                ..p.clone()
            }),
            s => closed_loop(s),
        }?;
        Some(Shape::Hatch(Hatch {
            loops: vec![ring],
            pattern: st.hatch_pattern,
            scale: st.hatch_scale,
            angle: st.hatch_angle.to_radians(),
            background: st.hatch_background,
            opacity: st.hatch_opacity,
        }))
    }

    fn filled(&self, c: Changes, st: &CadSettings) -> Option<Changes> {
        let changes: Vec<Change> = c
            .changes
            .into_iter()
            .filter_map(|ch| match ch {
                Change::Add(s) => Self::fill(&s, st).map(Change::Add),
                other => Some(other),
            })
            .collect();
        (!changes.is_empty()).then_some(Changes {
            label: self.label,
            changes,
        })
    }

    fn map(&self, step: Step, st: &CadSettings) -> Step {
        match step {
            Step::Apply(c) => match self.filled(c, st) {
                Some(c) => Step::Apply(c),
                None => Step::Refuse("El contorno no se cierra: no hay nada que rellenar.".into()),
            },
            Step::Done(Some(c)) => match self.filled(c, st) {
                Some(c) => Step::Done(Some(c)),
                None => Step::Refuse("El contorno no se cierra: no hay nada que rellenar.".into()),
            },
            other => other,
        }
    }
}

impl Command for Filled {
    fn name(&self) -> &'static str {
        self.name
    }

    fn prompt(&self) -> Prompt {
        self.inner.prompt()
    }

    fn start(&mut self, cx: &Ctx<'_, dyn Geo + '_>, st: &mut CadSettings) -> Step {
        let s = self.inner.start(cx, st);
        self.map(s, st)
    }

    fn feed(&mut self, input: Input, cx: &Ctx<'_, dyn Geo + '_>, st: &mut CadSettings) -> Step {
        let s = self.inner.feed(input, cx, st);
        self.map(s, st)
    }

    fn finish(&mut self, cx: &Ctx<'_, dyn Geo + '_>, st: &mut CadSettings) -> Option<Changes> {
        let c = self.inner.finish(cx, st)?;
        self.filled(c, st)
    }

    fn preview(&self, cx: &Ctx<'_, dyn Geo + '_>, st: &CadSettings) -> Vec<Shape> {
        let shapes = self.inner.preview(cx, st);
        let mut out: Vec<Shape> = shapes.iter().filter_map(|s| Self::fill(s, st)).collect();
        // The outline too, so the shape being drawn is seen.
        out.extend(shapes);
        out
    }

    fn base(&self) -> Option<Point2> {
        self.inner.base()
    }
}
