//! LOCAL and ETIQUETA: rooms found inside walls (or drawn as a polygon)
//! and tags of doors, windows, walls and rooms.

use archisim_core::{Point2, Polyline, Shape};

use super::*;

/// LOCAL: a click inside walls makes the room; [Polígono] draws one.
#[derive(Default)]
pub struct RoomCmd {
    polygon: Option<Vec<Point2>>,
}

impl Command for RoomCmd {
    fn name(&self) -> &'static str {
        "LOCAL"
    }

    fn prompt(&self) -> Prompt {
        match &self.polygon {
            None => Prompt::new("Señale un punto dentro del local", Want::Point)
                .opts(&[opt("P", "Polígono")]),
            Some(p) if p.len() < 3 => Prompt::new("Precise vértice del contorno", Want::Point),
            Some(_) => Prompt::new("Precise vértice siguiente (Intro: cerrar)", Want::Point),
        }
    }

    fn feed(&mut self, input: Input, _cx: &Ctx<'_, dyn Geo + '_>, _st: &mut CadSettings) -> Step {
        match (&mut self.polygon, input) {
            (None, Input::Option("P")) => {
                self.polygon = Some(Vec::new());
                Step::Next
            }
            (None, Input::Point(p)) => Step::Apply(Changes {
                label: "Nuevo local",
                changes: vec![Change::RoomAt(p)],
            }),
            (Some(pts), Input::Point(p)) => {
                pts.push(p);
                Step::Next
            }
            (Some(pts), Input::Enter) if pts.len() >= 3 => {
                let outline = std::mem::take(pts);
                self.polygon = None;
                Step::Apply(Changes {
                    label: "Nuevo local",
                    changes: vec![Change::Room(outline)],
                })
            }
            (_, Input::Enter) => Step::Done(None),
            _ => Step::Next,
        }
    }

    fn preview(&self, cx: &Ctx<'_, dyn Geo + '_>, _st: &CadSettings) -> Vec<Shape> {
        match &self.polygon {
            Some(p) if !p.is_empty() => {
                let mut pts = p.clone();
                pts.push(cx.cursor);
                vec![Shape::Polyline(Polyline::straight(pts, true))]
            }
            _ => Vec::new(),
        }
    }

    fn base(&self) -> Option<Point2> {
        self.polygon.as_ref().and_then(|p| p.last().copied())
    }
}

/// ETIQUETA: pick an element, then where its tag goes (Enter: beside it).
/// [Todas] tags every door and window of the level.
#[derive(Default)]
pub struct TagCmd {
    target: Option<Element>,
}

impl Command for TagCmd {
    fn name(&self) -> &'static str {
        "ETIQUETA"
    }

    fn prompt(&self) -> Prompt {
        match self.target {
            None => Prompt::new("Designe puerta, ventana, muro o local", Want::Pick)
                .opts(&[opt("T", "Todas las puertas y ventanas")]),
            Some(_) => Prompt::new("Precise la posición de la etiqueta", Want::Point)
                .default("junto al elemento"),
        }
    }

    fn feed(&mut self, input: Input, _cx: &Ctx<'_, dyn Geo + '_>, _st: &mut CadSettings) -> Step {
        match (self.target, input) {
            (None, Input::Option("T")) => Step::Done(Some(Changes {
                label: "Etiquetar puertas y ventanas",
                changes: vec![Change::TagAll],
            })),
            (None, Input::Pick(e, _)) => {
                if crate::editor::tag_target(e).is_none() {
                    return Step::Refuse(
                        "Eso no se etiqueta: elija una puerta, una ventana, un muro o un local."
                            .into(),
                    );
                }
                self.target = Some(e);
                Step::Next
            }
            (Some(e), Input::Point(p)) => {
                self.target = None;
                Step::Apply(Changes {
                    label: "Etiquetar",
                    changes: vec![Change::Tag(e, Some(p))],
                })
            }
            (Some(e), Input::Enter) => {
                self.target = None;
                Step::Apply(Changes {
                    label: "Etiquetar",
                    changes: vec![Change::Tag(e, None)],
                })
            }
            (None, Input::Enter) => Step::Done(None),
            _ => Step::Next,
        }
    }

    fn picked(&self) -> Vec<Element> {
        self.target.into_iter().collect()
    }
}
