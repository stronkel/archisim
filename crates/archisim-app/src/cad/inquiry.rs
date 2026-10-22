//! Inquiry commands: DIST, AREA and LIST.

use archisim_core::{Point2, Polyline, Shape};
use archisim_draft::seg::angle_of;

use super::*;
use crate::units::{fmt_angle, fmt_area, fmt_mm};

/// Distance between two points, with its angle and its X and Y parts.
#[derive(Default)]
pub struct Dist {
    first: Option<Point2>,
}

impl Command for Dist {
    fn name(&self) -> &'static str {
        "DIST"
    }

    fn prompt(&self) -> Prompt {
        match self.first {
            None => Prompt::new("Precise primer punto", Want::Point),
            Some(_) => Prompt::new("Precise segundo punto", Want::Point),
        }
    }

    fn feed(&mut self, input: Input, _cx: &Ctx<'_, dyn Geo + '_>, _st: &mut CadSettings) -> Step {
        match (self.first, input) {
            (None, Input::Point(p)) => {
                self.first = Some(p);
                Step::Next
            }
            (Some(a), Input::Point(b)) => {
                let d = b - a;
                Step::SayDone(format!(
                    "Distancia = {} · ángulo {} · ΔX = {} · ΔY = {}",
                    crate::units::fmt_len(d.length()),
                    fmt_angle(angle_of(d)),
                    fmt_mm(d.x),
                    fmt_mm(d.y)
                ))
            }
            (_, Input::Enter) => Step::Done(None),
            _ => Step::Next,
        }
    }

    fn preview(&self, cx: &Ctx<'_, dyn Geo + '_>, _st: &CadSettings) -> Vec<Shape> {
        self.first
            .map(|a| Shape::Line { a, b: cx.cursor })
            .into_iter()
            .collect()
    }

    fn base(&self) -> Option<Point2> {
        self.first
    }
}

/// Area and perimeter of a polygon given point by point, or of a closed
/// object.
#[derive(Default)]
pub struct AreaCmd {
    points: Vec<Point2>,
    object: bool,
}

fn report(area: f64, perimeter: f64) -> String {
    format!(
        "Área = {} · Perímetro = {}",
        fmt_area(area),
        crate::units::fmt_len(perimeter)
    )
}

impl Command for AreaCmd {
    fn name(&self) -> &'static str {
        "AREA"
    }

    fn prompt(&self) -> Prompt {
        if self.object {
            return Prompt::new("Designe el objeto", Want::Pick);
        }
        match self.points.len() {
            0 => Prompt::new("Precise primer punto de esquina", Want::Point)
                .opts(&[opt("O", "Objeto")]),
            _ => Prompt::new("Precise punto siguiente (Intro para terminar)", Want::Point),
        }
    }

    fn feed(&mut self, input: Input, cx: &Ctx<'_, dyn Geo + '_>, _st: &mut CadSettings) -> Step {
        match input {
            Input::Option("O") => {
                self.object = true;
                Step::Next
            }
            Input::Pick(e, _) if self.object => {
                let shape = match e {
                    Element::Draft(_) => cx.shape(e).cloned(),
                    Element::Slab(id) => cx
                        .model()
                        .slab(id)
                        .map(|s| Shape::Polyline(Polyline::straight(s.outline.clone(), true))),
                    _ => None,
                };
                let Some(s) = shape else {
                    return Step::Refuse("Elija un elemento cerrado o un forjado.".into());
                };
                let Some(a) = archisim_draft::area(&s) else {
                    return Step::Refuse("El objeto no está cerrado.".into());
                };
                // A slab's holes are taken out.
                let a = match e {
                    Element::Slab(id) => cx.model().slab(id).map_or(a, |s| s.area()),
                    _ => a,
                };
                Step::SayDone(report(a, archisim_draft::length(&s)))
            }
            Input::Point(p) => {
                self.points.push(p);
                Step::Next
            }
            Input::Enter if self.points.len() >= 3 => {
                let p = Shape::Polyline(Polyline::straight(self.points.clone(), true));
                let a = archisim_draft::area(&p).unwrap_or(0.0);
                Step::SayDone(report(a, archisim_draft::length(&p)))
            }
            Input::Enter => Step::Done(None),
            _ => Step::Next,
        }
    }

    fn preview(&self, cx: &Ctx<'_, dyn Geo + '_>, _st: &CadSettings) -> Vec<Shape> {
        if self.points.is_empty() {
            return Vec::new();
        }
        let mut pts = self.points.clone();
        pts.push(cx.cursor);
        vec![Shape::Polyline(Polyline::straight(pts, true))]
    }

    fn base(&self) -> Option<Point2> {
        self.points.last().copied()
    }
}

/// What the selected drafting elements are: kind, layer, length, area.
#[derive(Default)]
pub struct List;

impl List {
    fn run(cx: &Ctx<'_, dyn Geo + '_>) -> Step {
        let mut lines = Vec::new();
        for id in cx.selected_drafts() {
            let Some(d) = cx.model().draft(id) else {
                continue;
            };
            let layer = cx
                .model()
                .draw_layer(d.layer)
                .map(|l| l.name.clone())
                .unwrap_or_default();
            let mut s = format!("{} · capa «{layer}»", d.shape.label());
            let len = archisim_draft::length(&d.shape);
            if len > 0.0 {
                s.push_str(&format!(" · longitud {}", crate::units::fmt_len(len)));
            }
            if let Some(a) = archisim_draft::area(&d.shape) {
                s.push_str(&format!(" · área {}", fmt_area(a)));
            }
            lines.push(s);
        }
        if lines.is_empty() {
            return Step::SayDone("No hay elementos de dibujo seleccionados.".into());
        }
        Step::SayDone(lines.join("\n"))
    }
}

impl Command for List {
    fn name(&self) -> &'static str {
        "LIST"
    }

    fn start(&mut self, cx: &Ctx<'_, dyn Geo + '_>, _st: &mut CadSettings) -> Step {
        if cx.selected_drafts().is_empty() {
            Step::Next
        } else {
            Self::run(cx)
        }
    }

    fn prompt(&self) -> Prompt {
        Prompt::new("Designe objetos e Intro", Want::Select)
    }

    fn feed(&mut self, input: Input, cx: &Ctx<'_, dyn Geo + '_>, _st: &mut CadSettings) -> Step {
        if input == Input::Enter {
            return Self::run(cx);
        }
        Step::Next
    }
}
