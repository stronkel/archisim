//! Road markings at will (U4): the tool (lines and hatched areas by their
//! points, crossings by two points across the road, arrows and symbols by
//! one click, turned with the lane they are on) and their properties, the
//! kinds as cards drawn in plan.

use archisim_core::{MarkingId, MarkingKind, MarkingPlace, Model, NewMarking, Point2, Vec2};
use eframe::egui::{self, Color32, Id, Pos2, Rect, Shape, Stroke, Ui};

use super::props::{self, group, row, text_row};
use super::*;

/// What the tool paints.
#[derive(Debug, Clone, PartialEq)]
pub struct MarkingSettings {
    pub kind: MarkingKind,
    pub width: f64,
    pub dash: f64,
    pub gap: f64,
    pub depth: f64,
}

impl Default for MarkingSettings {
    fn default() -> Self {
        MarkingSettings::of(MarkingKind::Zebra)
    }
}

impl MarkingSettings {
    pub fn of(kind: MarkingKind) -> Self {
        let (width, dash, gap) = kind.defaults();
        MarkingSettings {
            kind,
            width,
            dash,
            gap,
            depth: 4.0,
        }
    }
}

/// The direction of the lane at `p`: along the nearest road's axis, the
/// way the traffic goes on that side (on the right of the axis, forward;
/// on its left, back). None if no road is near.
pub fn lane_direction(m: &Model, level: archisim_core::LevelId, p: Point2) -> Option<Vec2> {
    let r = m
        .roads_on_level(level)
        .map(|r| {
            let s = r.station_of(p);
            (r.axis_at(s), r)
        })
        .filter(|((q, _), r)| q.distance(p) <= r.widest() / 2.0 + 1.0)
        .min_by(|a, b| a.0 .0.distance(p).total_cmp(&b.0 .0.distance(p)))?;
    let ((q, u), _) = r;
    let left = (p - q).cross(u) < 0.0;
    Some(if left { u * -1.0 } else { u })
}

/// A marking kind drawn small on white: its paint along a sample path.
pub fn paint_kind(ui: &Ui, r: Rect, kind: MarkingKind) {
    let painter = ui.painter();
    let path = match kind.place() {
        MarkingPlace::Area => vec![
            Point2::new(0.0, 0.0),
            Point2::new(5.0, 0.0),
            Point2::new(5.0, 2.5),
            Point2::new(0.0, 2.5),
        ],
        MarkingPlace::Across => vec![Point2::new(2.5, -3.0), Point2::new(2.5, 3.0)],
        MarkingPlace::Symbol => vec![Point2::new(0.0, 0.0), Point2::new(1.0, 0.0)],
        MarkingPlace::Line => vec![Point2::new(0.0, 0.0), Point2::new(12.0, 0.0)],
    };
    let s = NewMarking::usual(
        archisim_core::LevelId(0),
        kind,
        path,
        archisim_core::MaterialId(0),
    );
    let m = archisim_core::Marking {
        id: MarkingId(0),
        guid: archisim_core::GlobalId::new(),
        level: s.level,
        kind,
        path: s.path,
        width: s.width,
        dash: s.dash,
        gap: s.gap,
        depth: s.depth,
        material: s.material,
    };
    let mut shapes = m.shapes();
    if let Some(f) = m.fill() {
        shapes.insert(0, f);
    }
    let pts: Vec<Point2> = shapes.iter().flatten().copied().collect();
    if pts.is_empty() {
        return;
    }
    let (lo, hi) = pts.iter().fold(
        (
            Point2::new(f64::MAX, f64::MAX),
            Point2::new(f64::MIN, f64::MIN),
        ),
        |(lo, hi), p| {
            (
                Point2::new(lo.x.min(p.x), lo.y.min(p.y)),
                Point2::new(hi.x.max(p.x), hi.y.max(p.y)),
            )
        },
    );
    let inner = r.shrink(8.0);
    let k = (inner.width() as f64 / (hi.x - lo.x).max(0.5))
        .min(inner.height() as f64 / (hi.y - lo.y).max(0.5));
    let c = Point2::new((lo.x + hi.x) / 2.0, (lo.y + hi.y) / 2.0);
    let to = |p: Point2| {
        Pos2::new(
            inner.center().x + ((p.x - c.x) * k) as f32,
            inner.center().y - ((p.y - c.y) * k) as f32,
        )
    };
    // The asphalt under it.
    painter.rect_filled(r.shrink(3.0), 3.0, Color32::from_rgb(92, 96, 102));
    let [rr, gg, bb] = kind.colour();
    for (i, s) in shapes.iter().enumerate() {
        let fill = if i == 0 && m.fill().is_some() {
            Color32::from_rgb(rr, gg, bb)
        } else if m.fill().is_some() {
            Color32::WHITE
        } else {
            Color32::from_rgb(rr, gg, bb)
        };
        let poly: Vec<Pos2> = s.iter().map(|p| to(*p)).collect();
        // Convex or not: fill by triangles of the polygon.
        for t in archisim_geom::solid::triangulate(s) {
            painter.add(Shape::convex_polygon(
                vec![poly[t[0]], poly[t[1]], poly[t[2]]],
                fill,
                Stroke::NONE,
            ));
        }
    }
}

impl ArchiSimApp {
    pub(super) fn marking_click(&mut self, p: Point2) {
        let s = self.marking_settings.clone();
        match s.kind.place() {
            MarkingPlace::Symbol => {
                let u = lane_direction(self.ed.model(), self.ed.level, p)
                    .unwrap_or(Vec2::new(1.0, 0.0));
                self.add_marking(vec![p, p + u]);
            }
            MarkingPlace::Across => {
                self.marking_points.push(p);
                if self.marking_points.len() == 2 {
                    let path = std::mem::take(&mut self.marking_points);
                    // A zebra across a road is the road's: it goes with it
                    // and lowers its kerbs.
                    if s.kind == archisim_core::MarkingKind::Zebra {
                        let level = self.ed.level;
                        let hit =
                            self.ed.model().roads_on_level(level).find_map(|r| {
                                r.station_across(path[0], path[1]).map(|at| (r.id, at))
                            });
                        if let Some((road, at)) = hit {
                            let res = self.ed.transact("Paso de peatones", |m| {
                                m.edit_road(road, |r| {
                                    r.section
                                        .crossings
                                        .push(archisim_core::Crossing { at, width: s.depth })
                                })
                            });
                            self.report(
                                res,
                                "Paso de peatones con sus rebajes añadido a la calle.",
                            );
                            return;
                        }
                    }
                    self.add_marking(path);
                }
            }
            MarkingPlace::Line | MarkingPlace::Area => self.marking_points.push(p),
        }
    }

    /// Ends a line or an area (Enter, double click or right click).
    pub(super) fn finish_marking(&mut self) {
        let need = match self.marking_settings.kind.place() {
            MarkingPlace::Area => 3,
            _ => 2,
        };
        let path = std::mem::take(&mut self.marking_points);
        if path.len() >= need {
            self.add_marking(path);
        }
    }

    fn add_marking(&mut self, path: Vec<Point2>) {
        let s = self.marking_settings.clone();
        let level = self.ed.level;
        let mut id = None;
        let res = self.ed.transact("Marca vial", |m| {
            let material = m.marking_material(s.kind.colour());
            id = Some(m.add_marking(NewMarking {
                level,
                kind: s.kind,
                path,
                width: s.width,
                dash: s.dash,
                gap: s.gap,
                depth: s.depth,
                material,
            })?);
            Ok(())
        });
        match (res, id) {
            (Ok(()), Some(_)) => {}
            (Err(e), _) => self.complain(format!("No se puede pintar la marca: {e}.")),
            _ => {}
        }
    }

    pub(super) fn marking_hint(&self) -> &'static str {
        match self.marking_settings.kind.place() {
            MarkingPlace::Symbol => "Clic: donde va (se gira con el carril) · Esc: flecha",
            MarkingPlace::Across if self.marking_points.is_empty() => {
                "Clic: un lado de la calle · Esc: flecha"
            }
            MarkingPlace::Across => "Clic: el otro lado de la calle · Esc: anular",
            MarkingPlace::Line => {
                "Clic: puntos de la línea · Intro, doble clic o botón derecho: terminar · Esc: quitar el último"
            }
            MarkingPlace::Area => {
                "Clic: esquinas · Intro, doble clic o botón derecho: cerrar · Esc: quitar la última"
            }
        }
    }

    /// The line being drawn, to the cursor.
    pub(super) fn draw_marking_preview(&self, painter: &egui::Painter, rect: Rect) {
        let Some(c) = self.cursor else {
            return;
        };
        if self.marking_points.is_empty() {
            return;
        }
        let mut drawn = self.marking_points.clone();
        drawn.push(c.point);
        if self.marking_settings.kind.place() == MarkingPlace::Area {
            drawn.push(drawn[0]);
        }
        painter.add(Shape::line(
            self.polygon(rect, &drawn),
            Stroke::new(1.5, ACCENT),
        ));
    }

    pub(super) fn marking_tool_properties(&mut self, ui: &mut Ui) {
        props::title(ui, "Herramienta Marca vial · lo que vas a pintar");
        let mut s = self.marking_settings.clone();
        group(ui, "tool-marking", "Marca vial (Norma 8.2-IC)", |ui| {
            if let Some(k) = kind_cards(ui, s.kind) {
                s = MarkingSettings::of(k);
                self.marking_points.clear();
            }
            sizes(
                ui,
                "tool-mk",
                s.kind,
                &mut s.width,
                &mut s.dash,
                &mut s.gap,
                &mut s.depth,
            );
        });
        self.marking_settings = s;
    }

    pub(super) fn marking_properties(&mut self, ui: &mut Ui, id: MarkingId) {
        let Some(m) = self.ed.model().marking(id).cloned() else {
            return;
        };
        props::title(ui, &format!("Marca vial · {}", m.kind.name()));
        let mut n = m.clone();
        group(ui, "mk", "Marca vial", |ui| {
            if let Some(k) = kind_cards(ui, n.kind) {
                // Another kind of the same placing keeps its path.
                if k.place() == n.kind.place() {
                    let (w, d, g) = k.defaults();
                    n.kind = k;
                    n.width = w;
                    n.dash = d;
                    n.gap = g;
                }
            }
            sizes(
                ui,
                &format!("mk-{}", id.0),
                n.kind,
                &mut n.width,
                &mut n.dash,
                &mut n.gap,
                &mut n.depth,
            );
            row(ui, "Pintura", "", |ui, w| {
                n.material = urban_ui::material_combo(
                    ui,
                    Id::unique(("mk-mat", id)),
                    self.ed.model(),
                    n.material,
                    w,
                );
            });
            let len: f64 = n.path.windows(2).map(|w| w[0].distance(w[1])).sum();
            text_row(
                ui,
                "Longitud (m)",
                "",
                &format!("{len:.2}").replace('.', ","),
            );
        });
        group(ui, "mk-identity", "Datos de identidad", |ui| {
            text_row(ui, "Clase IFC", "", "IfcBuildingElementProxy (MARKING)");
        });
        if n != m {
            let res = self
                .ed
                .transact("Marca vial", |model| model.edit_marking(id, |x| *x = n));
            self.report(res, "Marca vial cambiada.");
        }
    }
}

/// The kinds as cards in their groups; the one picked.
fn kind_cards(ui: &mut Ui, now: MarkingKind) -> Option<MarkingKind> {
    let mut groups: Vec<(String, Vec<String>)> = Vec::new();
    for k in MarkingKind::ALL {
        match groups.iter_mut().find(|(g, _)| g == k.group()) {
            Some((_, v)) => v.push(k.name().to_string()),
            None => groups.push((k.group().to_string(), vec![k.name().to_string()])),
        }
    }
    let picked = super::cards::card_row(ui, "Tipo de marca", |ui, w| {
        super::cards::grouped_cards(ui, w, &groups, now.name(), &mut |ui, r, n| {
            if let Some(k) = MarkingKind::ALL.into_iter().find(|k| k.name() == n) {
                paint_kind(ui, r, k);
            }
        })
    })?;
    MarkingKind::ALL.into_iter().find(|k| k.name() == picked)
}

/// Width, dashes and a crossing's depth.
fn sizes(
    ui: &mut Ui,
    salt: &str,
    kind: MarkingKind,
    width: &mut f64,
    dash: &mut f64,
    gap: &mut f64,
    depth: &mut f64,
) {
    let dashed = matches!(
        kind,
        MarkingKind::Dashed
            | MarkingKind::ContinuousDashed
            | MarkingKind::GiveWayLine
            | MarkingKind::SignalCrossing
            | MarkingKind::Zebra
            | MarkingKind::RaisedCrossing
            | MarkingKind::Chevrons
    );
    let crossing = kind.place() == MarkingPlace::Across;
    let mut rows: Vec<(&str, f64, usize)> = vec![(
        if crossing {
            "Ancho de las bandas (mm)"
        } else {
            "Ancho de la línea (mm)"
        },
        *width,
        0,
    )];
    if dashed {
        rows.push(if crossing {
            ("Hueco entre bandas (mm)", *gap, 1)
        } else {
            ("Trazo (mm)", *dash, 1)
        });
        if !crossing {
            rows.push(("Vano (mm)", *gap, 2));
        }
    }
    if crossing {
        rows.push(("Ancho del paso (mm)", *depth, 3));
    }
    for (name, value, i) in rows {
        row(ui, name, "", |ui, w| {
            if let Some(v) = length_field(ui, Id::unique((salt, i)), value, w - 8.0) {
                let v = v.max(0.01);
                match i {
                    0 => *width = v,
                    1 => {
                        // A crossing's «hueco» is the gap between its bars.
                        if crossing {
                            *gap = v;
                        } else {
                            *dash = v;
                        }
                    }
                    2 => *gap = v,
                    _ => *depth = v,
                }
            }
        });
    }
}
