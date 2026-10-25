//! The Exterior brushes (MQ3, C5) on the screen: the circle under the
//! cursor, in the plan and in 3D; the stroke while the button is held (a
//! dab every little distance, all in one step of Deshacer); and their
//! settings on the bar and in Propiedades.

use archisim_core::Point2;
use eframe::egui::{self, Color32, Pos2, Rect, Stroke, Ui};

use super::*;
use crate::brush::{self, BrushKind};

/// A stroke being made: where Deshacer starts it, the last dab, the
/// height it flattens to and how many dabs so far.
#[derive(Debug, Clone, Copy)]
pub struct BrushStroke {
    mark: usize,
    last: Point2,
    flat: f64,
    dabs: u64,
    plants: usize,
}

impl ArchiSimApp {
    /// What a click does with the brush, for the bar.
    pub(super) fn brush_hint(&self) -> &'static str {
        match self.brush.kind {
            BrushKind::Raise | BrushKind::Lower => {
                "Arrastra sobre el suelo: sube o baja, más en el centro · Radio y fuerza arriba · Esc: flecha"
            }
            BrushKind::Smooth => "Arrastra: suaviza el terreno · Esc: flecha",
            BrushKind::Flatten => "Arrastra: deja el terreno a la altura del punto donde empiezas · Esc: flecha",
            BrushKind::Lawn | BrushKind::Gravel | BrushKind::Path => {
                "Arrastra: pinta sobre el terreno (sin terreno, se hace uno llano) · Esc: flecha"
            }
            BrushKind::Plants => "Arrastra: esparce plantas de la especie de Propiedades · Esc: flecha",
        }
    }

    /// Chooses a brush (and the tool).
    pub(super) fn take_brush(&mut self, kind: BrushKind) {
        self.set_tool(Tool::Brush);
        self.set_brush_kind(kind);
    }

    /// Changes the brush, with the strength that suits it.
    pub(super) fn set_brush_kind(&mut self, kind: BrushKind) {
        if self.brush.kind != kind {
            let was_sculpt = matches!(self.brush.kind, BrushKind::Raise | BrushKind::Lower);
            let is_sculpt = matches!(kind, BrushKind::Raise | BrushKind::Lower);
            if was_sculpt != is_sculpt || kind == BrushKind::Plants {
                self.brush.strength = match kind {
                    BrushKind::Raise | BrushKind::Lower => 0.15,
                    BrushKind::Plants => 1.0,
                    _ => 0.5,
                };
            }
            if kind == BrushKind::Plants && self.brush.radius < 3.0 {
                self.brush.radius = 4.0;
            }
        }
        self.brush.kind = kind;
    }

    /// The species the plants brush scatters: the one chosen, or the first
    /// plant of the project, or an olive tree from the library.
    fn brush_species(&mut self) -> Option<archisim_core::ObjectTypeId> {
        if let Some(t) = self
            .brush
            .species
            .filter(|t| self.ed.model().object_type(*t).is_some())
        {
            return Some(t);
        }
        // A grown species first (they look like trees), then any plant.
        let first = |model: &archisim_core::Model, grown: bool| {
            model
                .object_types()
                .find(|t| {
                    t.own.is_none()
                        && brush::is_plant(model, t.id)
                        && model.type_grows(t.id) == grown
                })
                .map(|t| t.id)
        };
        let t = match first(self.ed.model(), true) {
            Some(t) => t,
            None => {
                let olive = self
                    .ed
                    .transact("Cargar especie del vivero", |m| {
                        archisim_library::flora::add_flora_entry(m, "Olivo")
                    })
                    .ok()
                    .flatten();
                olive.or_else(|| first(self.ed.model(), false))?
            }
        };
        self.brush.species = Some(t);
        Some(t)
    }

    /// The mouse with the brush: a dab when the button goes down and every
    /// little way while it is held; on letting go, the stroke is one step.
    pub(super) fn brush_clicks(&mut self, ctx: &egui::Context, response: &egui::Response) {
        let down = response.is_pointer_button_down_on() && ctx.input(|i| i.pointer.primary_down());
        let Some(at) = self.cursor.map(|c| c.point) else {
            if !down {
                self.end_brush_stroke();
            }
            return;
        };
        if !down {
            self.end_brush_stroke();
            return;
        }
        let r = self.brush.radius.max(0.2);
        match self.brush_stroke {
            None => {
                let mark = self.ed.undo_mark();
                let level = self.ed.level;
                let flat = self
                    .ed
                    .model()
                    .terrains_on_level(level)
                    .find_map(|t| t.height_at(at))
                    .unwrap_or_else(|| self.ed.model().level(level).map_or(0.0, |l| l.elevation));
                self.brush_stroke = Some(BrushStroke {
                    mark,
                    last: at,
                    flat,
                    dabs: 0,
                    plants: 0,
                });
                self.brush_dab(at);
            }
            Some(s) if s.last.distance(at) >= r * brush::DAB_STEP => {
                // Every step of the way from the last dab (the mouse may
                // have gone far in one frame): a continuous stroke.
                let step = r * brush::DAB_STEP;
                let d = at - s.last;
                let n = (d.length() / step).floor().clamp(1.0, 200.0) as usize;
                for i in 1..=n {
                    let q = if i == n {
                        at
                    } else {
                        s.last + d * (i as f64 / n as f64)
                    };
                    self.brush_dab(q);
                }
            }
            Some(_) => {}
        }
        ctx.request_repaint();
    }

    /// One dab of the brush at `at`.
    fn brush_dab(&mut self, at: Point2) {
        let Some(mut s) = self.brush_stroke else {
            return;
        };
        let b = self.brush;
        let level = self.ed.level;
        let species = if b.kind == BrushKind::Plants {
            self.brush_species()
        } else {
            None
        };
        let seed = 0x9E37_79B9 ^ s.dabs.wrapping_mul(0x2545_F491) ^ (at.x * 1000.0) as u64;
        let r = self.ed.transact(b.kind.undo_label(), |m| {
            let t = brush::ensure_terrain(m, level, at)?;
            match b.kind {
                BrushKind::Lawn | BrushKind::Gravel | BrushKind::Path => {
                    if let Some(mat) = brush::paint_material(m, b.kind) {
                        brush::paint(m, t, mat, at, b.radius)?;
                    }
                    Ok(0)
                }
                BrushKind::Plants => match species {
                    Some(sp) => brush::scatter(m, level, sp, at, b.radius, b.strength, seed)
                        .map(|v| v.len()),
                    None => Ok(0),
                },
                k => brush::sculpt(m, t, k, at, b.radius, b.strength, s.flat).map(|_| 0),
            }
        });
        match r {
            Ok(n) => {
                s.plants += n;
                s.dabs += 1;
                s.last = at;
                self.brush_stroke = Some(s);
            }
            Err(e) => {
                self.brush_stroke = Some(BrushStroke { last: at, ..s });
                self.complain(format!("No se puede: {e}."));
            }
        }
    }

    /// Ends the stroke being made: all its dabs, one step of Deshacer.
    pub(super) fn end_brush_stroke(&mut self) {
        let Some(s) = self.brush_stroke.take() else {
            return;
        };
        if s.dabs == 0 {
            return;
        }
        let label = self.brush.kind.undo_label();
        self.ed.join_since(s.mark, label);
        if self.brush.kind == BrushKind::Plants {
            self.say(format!("{} plantas esparcidas.", s.plants));
        } else {
            self.say(format!("{label}."));
        }
    }

    /// The brush's circle in the plan.
    pub(super) fn draw_brush_plan(&self, painter: &egui::Painter, rect: Rect) {
        if self.tool != Tool::Brush {
            return;
        }
        let Some(c) = self.cursor else {
            return;
        };
        let centre = self.view.to_screen(rect, c.point);
        let r = (self.brush.radius * self.view.zoom) as f32;
        painter.circle_stroke(centre, r, Stroke::new(1.5, brush_colour(self.brush.kind)));
        painter.circle_stroke(
            centre,
            r * 0.5,
            Stroke::new(0.8, brush_colour(self.brush.kind)),
        );
    }

    /// The brush's circle in 3D, on the ground under it.
    pub(super) fn draw_brush_3d(&self, painter: &egui::Painter, rect: Rect) {
        if self.tool != Tool::Brush {
            return;
        }
        let Some(c) = self.cursor3d else {
            return;
        };
        let model = self.ed.model();
        let level = self.ed.level;
        let z0 = model.level(level).map_or(0.0, |l| l.elevation);
        let terrain = model.terrains_on_level(level).next();
        let size = glam::Vec2::new(rect.width(), rect.height());
        let centre = Point2::new(c.world.x as f64, c.world.y as f64);
        let pts: Vec<Pos2> = (0..=48)
            .filter_map(|i| {
                let a = i as f64 / 48.0 * std::f64::consts::TAU;
                let p = centre + Point2::new(a.cos(), a.sin()) * self.brush.radius;
                let z = terrain.and_then(|t| t.height_at(p)).unwrap_or(z0) + 0.03;
                self.v3d
                    .camera
                    .project_to_screen(glam::Vec3::new(p.x as f32, p.y as f32, z as f32), size)
                    .map(|s| Pos2::new(rect.min.x + s.x, rect.min.y + s.y))
            })
            .collect();
        painter.add(egui::Shape::line(
            pts,
            Stroke::new(2.0, brush_colour(self.brush.kind)),
        ));
    }

    /// Radius and strength, on the bar after the brushes.
    pub(super) fn brush_fields(&mut self, ui: &mut Ui) {
        if self.tool != Tool::Brush {
            return;
        }
        ui.add_space(8.0);
        ui.label(egui::RichText::new("Radio").small());
        ui.add(
            egui::DragValue::new(&mut self.brush.radius)
                .range(0.3..=30.0)
                .speed(0.05)
                .suffix(" m"),
        );
        ui.add_space(4.0);
        let (label, speed, max, suffix) = match self.brush.kind {
            BrushKind::Raise | BrushKind::Lower => ("Fuerza", 0.01, 2.0, " m"),
            BrushKind::Plants => ("Densidad", 0.05, 20.0, " /10 m²"),
            BrushKind::Lawn | BrushKind::Gravel | BrushKind::Path => return,
            _ => ("Fuerza", 0.01, 1.0, ""),
        };
        ui.label(egui::RichText::new(label).small());
        ui.add(
            egui::DragValue::new(&mut self.brush.strength)
                .range(0.01..=max)
                .speed(speed)
                .suffix(suffix),
        );
    }

    /// The brush in Propiedades: which, radius, strength and the species.
    pub(super) fn brush_tool_properties(&mut self, ui: &mut Ui) {
        use super::props::{group, row};
        props::title(ui, "Pincel de Exterior");
        group(ui, "tool-brush", "Pincel", |ui| {
            row(ui, "Qué hace", self.brush.kind.hint(), |ui, w| {
                let mut kind = self.brush.kind;
                egui::ComboBox::from_id_salt("brush-kind")
                    .width(w - 8.0)
                    .selected_text(kind.name())
                    .show_ui(ui, |ui| {
                        for k in BrushKind::ALL {
                            ui.selectable_value(&mut kind, k, k.name());
                        }
                    });
                if kind != self.brush.kind {
                    self.set_brush_kind(kind);
                }
            });
            row(ui, "Radio", "Hasta dónde llega el pincel", |ui, _| {
                ui.add(
                    egui::DragValue::new(&mut self.brush.radius)
                        .range(0.3..=30.0)
                        .speed(0.05)
                        .suffix(" m"),
                );
            });
        });
        if self.brush.kind == BrushKind::Plants {
            let current = self.brush_species();
            let model = self.ed.model();
            let mut plants: Vec<(archisim_core::ObjectTypeId, String)> = model
                .object_types()
                .filter(|t| brush::is_plant(model, t.id) && t.own.is_none())
                .map(|t| (t.id, t.name.clone()))
                .collect();
            plants.sort_by(|a, b| a.1.cmp(&b.1));
            let mut chosen = current;
            group(ui, "tool-brush-plants", "Plantas", |ui| {
                row(
                    ui,
                    "Especie",
                    "Las del proyecto; más en el Vivero o en el catálogo de Exterior",
                    |ui, w| {
                        let name = chosen
                            .and_then(|t| plants.iter().find(|p| p.0 == t))
                            .map_or("—".to_string(), |p| p.1.clone());
                        egui::ComboBox::from_id_salt("brush-species")
                            .width(w - 8.0)
                            .selected_text(name)
                            .show_ui(ui, |ui| {
                                for (id, n) in &plants {
                                    ui.selectable_value(&mut chosen, Some(*id), n);
                                }
                            });
                    },
                );
            });
            if chosen != current {
                self.brush.species = chosen;
            }
        }
    }
}

/// The colour of a brush's circle.
fn brush_colour(kind: BrushKind) -> Color32 {
    match kind {
        BrushKind::Lawn => Color32::from_rgb(70, 140, 60),
        BrushKind::Gravel => Color32::from_rgb(150, 140, 120),
        BrushKind::Path => Color32::from_rgb(120, 110, 100),
        BrushKind::Plants => Color32::from_rgb(40, 120, 70),
        _ => Color32::from_rgb(150, 100, 50),
    }
}
