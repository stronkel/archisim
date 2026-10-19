//! Dimension commands of drafting: ACOANGULO, ACORADIO, ACODIAMETRO and
//! ACOARCO, as in AutoCAD. The dimensions are drafting elements of the
//! current layer; their value follows their geometry.

use std::f64::consts::{PI, TAU};

use archisim_core::{Measure, Point2, Shape};
use archisim_draft::seg::{angle_of, norm_angle};
use archisim_draft::Seg;

use super::*;

/// The line or arc of an element nearest to where it was picked: a
/// drafting element's own, or an edge of a wall or slab.
fn picked_seg(cx: &Ctx<'_, dyn Geo + '_>, e: Element, at: Point2) -> Option<Seg> {
    let segs: Vec<Seg> = match e {
        Element::Draft(id) => archisim_draft::boundary_segs(&cx.model().draft(id)?.shape),
        Element::Wall(id) => {
            // A curved wall by its drawn arc.
            if let Some(a) = cx.model().wall(id)?.arc() {
                return Some(Seg::Arc {
                    centre: a.centre,
                    radius: a.radius,
                    start: a.start_angle,
                    sweep: a.sweep,
                });
            }
            let g = cx.geo.wall_geometry(id)?;
            let n = g.outline.len();
            (0..n)
                .map(|i| Seg::Line {
                    a: g.outline[i],
                    b: g.outline[(i + 1) % n],
                })
                .filter(|s| s.length() > 1e-9)
                .collect()
        }
        Element::Room(id) => {
            let o = &cx.model().room(id)?.outline;
            (0..o.len())
                .map(|i| Seg::Line {
                    a: o[i],
                    b: o[(i + 1) % o.len()],
                })
                .collect()
        }
        Element::Slab(id) => {
            let o = &cx.model().slab(id)?.outline;
            (0..o.len())
                .map(|i| Seg::Line {
                    a: o[i],
                    b: o[(i + 1) % o.len()],
                })
                .collect()
        }
        _ => Vec::new(),
    };
    segs.into_iter()
        .min_by(|a, b| a.distance(at).total_cmp(&b.distance(at)))
}

/// An arc as centre, radius, start and counter-clockwise sweep.
fn ccw_arc(s: &Seg) -> Option<(Point2, f64, f64, f64)> {
    match *s {
        Seg::Arc {
            centre,
            radius,
            start,
            sweep,
        } => Some(if sweep >= 0.0 {
            (centre, radius, start, sweep)
        } else {
            (centre, radius, start + sweep, -sweep)
        }),
        Seg::Line { .. } => None,
    }
}

/// The angle between two lines meeting at `vertex`, in the quarter where
/// `p` is: start and sweep (less than half a turn).
fn sector(vertex: Point2, a1: f64, a2: f64, p: Point2) -> Option<(f64, f64)> {
    let at = angle_of(p - vertex);
    let mut best = None;
    for r1 in [a1, a1 + PI] {
        for r2 in [a2, a2 + PI] {
            for (from, to) in [(r1, r2), (r2, r1)] {
                let sweep = norm_angle(to - from);
                if sweep <= 1e-9 || sweep >= PI - 1e-9 {
                    continue;
                }
                let into = norm_angle(at - from);
                if into <= sweep {
                    best = Some((norm_angle(from), sweep));
                }
            }
        }
    }
    best
}

fn add(label: &'static str, m: Measure) -> Step {
    let shape = Shape::Dim(m);
    match archisim_core::check_shape(&shape) {
        Ok(()) => Step::Done(Some(Changes {
            label,
            changes: vec![Change::Add(shape)],
        })),
        Err(_) => Step::Refuse("Ahí la cota no tiene medida.".into()),
    }
}

/// ACOANGULO: the angle between two lines, or the angle of an arc.
#[derive(Default)]
pub struct DimAngle {
    first: Option<(Seg, Element)>,
    lines: Option<(Point2, f64, f64)>,
    arc: Option<(Point2, f64, f64)>,
    picked: Vec<Element>,
}

impl DimAngle {
    fn measure(&self, p: Point2) -> Option<Measure> {
        if let Some((centre, start, sweep)) = self.arc {
            return Some(Measure::Angle {
                centre,
                start,
                sweep,
                radius: p.distance(centre).max(1e-6),
            });
        }
        let (vertex, a1, a2) = self.lines?;
        let (start, sweep) = sector(vertex, a1, a2, p)?;
        Some(Measure::Angle {
            centre: vertex,
            start,
            sweep,
            radius: p.distance(vertex).max(1e-6),
        })
    }
}

impl Command for DimAngle {
    fn name(&self) -> &'static str {
        "ACOANGULO"
    }

    fn prompt(&self) -> Prompt {
        if self.lines.is_some() || self.arc.is_some() {
            Prompt::new("Precise la posición del arco de cota", Want::Point)
        } else if self.first.is_some() {
            Prompt::new("Designe la segunda línea", Want::Pick)
        } else {
            Prompt::new("Designe un arco o la primera línea", Want::Pick)
        }
    }

    fn feed(&mut self, input: Input, cx: &Ctx<'_, dyn Geo + '_>, _st: &mut CadSettings) -> Step {
        match input {
            Input::Pick(e, at) if self.lines.is_none() && self.arc.is_none() => {
                let Some(seg) = picked_seg(cx, e, at) else {
                    return Step::Refuse("Elija una línea o un arco.".into());
                };
                match (self.first, seg) {
                    (None, Seg::Arc { .. }) => {
                        let (c, _, start, sweep) = ccw_arc(&seg).expect("arc");
                        if sweep >= TAU - 1e-9 {
                            return Step::Refuse(
                                "Un círculo entero no tiene ángulo: elija un arco.".into(),
                            );
                        }
                        self.arc = Some((c, start, sweep));
                        self.picked = vec![e];
                        Step::Next
                    }
                    (None, line) => {
                        self.first = Some((line, e));
                        self.picked = vec![e];
                        Step::Next
                    }
                    (Some((l1, _)), Seg::Line { .. }) => {
                        let Some((_, _, vertex)) =
                            archisim_draft::seg::intersect_extended(&l1, &seg)
                                .first()
                                .copied()
                        else {
                            return Step::Refuse(
                                "Las líneas son paralelas: no forman ángulo.".into(),
                            );
                        };
                        let dir = |s: &Seg| angle_of(s.end() - s.start());
                        self.lines = Some((vertex, dir(&l1), dir(&seg)));
                        self.picked.push(e);
                        Step::Next
                    }
                    (Some(_), _) => Step::Refuse("Elija otra línea.".into()),
                }
            }
            Input::Point(p) if self.lines.is_some() || self.arc.is_some() => {
                match self.measure(p) {
                    Some(m) => add("Acotar ángulo", m),
                    None => Step::Refuse("Señale dentro del ángulo.".into()),
                }
            }
            Input::Enter => Step::Done(None),
            _ => Step::Next,
        }
    }

    fn preview(&self, cx: &Ctx<'_, dyn Geo + '_>, _st: &CadSettings) -> Vec<Shape> {
        self.measure(cx.cursor)
            .map(Shape::Dim)
            .into_iter()
            .collect()
    }

    fn picked(&self) -> Vec<Element> {
        self.picked.clone()
    }
}

/// ACORADIO and ACODIAMETRO: of a circle or an arc.
pub struct DimRadius {
    diameter: bool,
    circle: Option<(Point2, f64)>,
    picked: Vec<Element>,
}

impl DimRadius {
    pub fn new(diameter: bool) -> Self {
        Self {
            diameter,
            circle: None,
            picked: Vec::new(),
        }
    }

    fn measure(&self, p: Point2) -> Option<Measure> {
        let (centre, radius) = self.circle?;
        let d = p - centre;
        let angle = if d.length() > 1e-9 {
            d.y.atan2(d.x)
        } else {
            0.0
        };
        let reach = d.length();
        Some(if self.diameter {
            Measure::Diameter {
                centre,
                radius,
                angle,
                reach,
            }
        } else {
            Measure::Radius {
                centre,
                radius,
                angle,
                reach,
            }
        })
    }
}

impl Command for DimRadius {
    fn name(&self) -> &'static str {
        if self.diameter {
            "ACODIAMETRO"
        } else {
            "ACORADIO"
        }
    }

    fn prompt(&self) -> Prompt {
        match self.circle {
            None => Prompt::new("Designe un arco o un círculo", Want::Pick),
            Some(_) => Prompt::new("Precise la posición de la línea de cota", Want::Point),
        }
    }

    fn feed(&mut self, input: Input, cx: &Ctx<'_, dyn Geo + '_>, _st: &mut CadSettings) -> Step {
        match input {
            Input::Pick(e, at) if self.circle.is_none() => {
                match picked_seg(cx, e, at).as_ref().and_then(ccw_arc) {
                    Some((c, r, _, _)) => {
                        self.circle = Some((c, r));
                        self.picked = vec![e];
                        Step::Next
                    }
                    None => Step::Refuse("Elija un arco o un círculo.".into()),
                }
            }
            Input::Point(p) if self.circle.is_some() => match self.measure(p) {
                Some(m) => add(
                    if self.diameter {
                        "Acotar diámetro"
                    } else {
                        "Acotar radio"
                    },
                    m,
                ),
                None => Step::Next,
            },
            Input::Enter => Step::Done(None),
            _ => Step::Next,
        }
    }

    fn preview(&self, cx: &Ctx<'_, dyn Geo + '_>, _st: &CadSettings) -> Vec<Shape> {
        self.measure(cx.cursor)
            .map(Shape::Dim)
            .into_iter()
            .collect()
    }

    fn picked(&self) -> Vec<Element> {
        self.picked.clone()
    }
}

/// ACOARCO: the length of an arc.
#[derive(Default)]
pub struct DimArc {
    arc: Option<(Point2, f64, f64, f64)>,
    picked: Vec<Element>,
}

impl DimArc {
    fn measure(&self, p: Point2) -> Option<Measure> {
        let (centre, radius, start, sweep) = self.arc?;
        Some(Measure::ArcLength {
            centre,
            radius,
            start,
            sweep,
            offset: p.distance(centre) - radius,
        })
    }
}

impl Command for DimArc {
    fn name(&self) -> &'static str {
        "ACOARCO"
    }

    fn prompt(&self) -> Prompt {
        match self.arc {
            None => Prompt::new("Designe un arco", Want::Pick),
            Some(_) => Prompt::new("Precise la posición de la cota", Want::Point),
        }
    }

    fn feed(&mut self, input: Input, cx: &Ctx<'_, dyn Geo + '_>, _st: &mut CadSettings) -> Step {
        match input {
            Input::Pick(e, at) if self.arc.is_none() => {
                match picked_seg(cx, e, at).as_ref().and_then(ccw_arc) {
                    Some(a) if a.3 < TAU - 1e-9 => {
                        self.arc = Some(a);
                        self.picked = vec![e];
                        Step::Next
                    }
                    _ => Step::Refuse("Elija un arco (o un tramo curvo de polilínea).".into()),
                }
            }
            Input::Point(p) if self.arc.is_some() => match self.measure(p) {
                Some(m) => add("Acotar longitud de arco", m),
                None => Step::Next,
            },
            Input::Enter => Step::Done(None),
            _ => Step::Next,
        }
    }

    fn preview(&self, cx: &Ctx<'_, dyn Geo + '_>, _st: &CadSettings) -> Vec<Shape> {
        self.measure(cx.cursor)
            .map(Shape::Dim)
            .into_iter()
            .collect()
    }

    fn picked(&self) -> Vec<Element> {
        self.picked.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_quarter_where_the_point_is() {
        let v = Point2::new(0.0, 0.0);
        // Lines along x and y: the first quarter has 90° from 0.
        let (s, w) = sector(v, 0.0, PI / 2.0, Point2::new(1.0, 1.0)).unwrap();
        assert!(s.abs() < 1e-9 && (w - PI / 2.0).abs() < 1e-9);
        // In the third quarter it starts at 180°.
        let (s, w) = sector(v, 0.0, PI / 2.0, Point2::new(-1.0, -1.0)).unwrap();
        assert!((s - PI).abs() < 1e-9 && (w - PI / 2.0).abs() < 1e-9);
        // Lines at 0° and 30°: between them, 30°; outside, 150°.
        let (_, w) = sector(v, 0.0, PI / 6.0, Point2::new(1.0, 0.2)).unwrap();
        assert!((w - PI / 6.0).abs() < 1e-9);
        let (_, w) = sector(v, 0.0, PI / 6.0, Point2::new(0.0, 1.0)).unwrap();
        assert!((w - 5.0 * PI / 6.0).abs() < 1e-9);
    }
}
