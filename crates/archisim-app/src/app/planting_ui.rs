//! Urbanismo → Plantación: plants along a line (clicks, a spline or a
//! line of the drawing) or inside an outline, as many as asked, more or
//! less regular and varied, of one species or several mixed. While the
//! line is drawn, the plants are seen where they will go. A planting is
//! one element: changing its number, species or variation (or moving its
//! line) makes its plants again.

use super::widgets::SwitchExt as _;
use archisim_core::{Arrangement, NewPlanting, ObjectTypeId, PlantingId, Point2};
use eframe::egui::{self, Ui};

use super::object_gallery::ObjectScope;
use super::props::{self, group, row, text_row};
use super::*;

/// What the planting tool plants.
#[derive(Debug, Clone, PartialEq)]
pub struct PlantingSettings {
    pub arrangement: Arrangement,
    pub count: u32,
    /// Mixed at random; empty: the species the Vegetación tool has.
    pub species: Vec<ObjectTypeId>,
    pub size_variation: f64,
    pub shape_variation: f64,
    pub spread: f64,
    pub seed: u64,
    /// A curve through the clicks instead of straight pieces.
    pub smooth: bool,
}

impl Default for PlantingSettings {
    fn default() -> Self {
        PlantingSettings {
            arrangement: Arrangement::Line,
            count: 8,
            species: Vec::new(),
            size_variation: 0.3,
            shape_variation: 0.5,
            spread: 0.15,
            seed: 1,
            smooth: true,
        }
    }
}

/// A new number to start the chance with.
fn new_seed(old: u64) -> u64 {
    let t = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(0);
    archisim_core::flora::seed_of(old ^ t, 1) % 999_983 + 1
}

/// A share from 0 to 1 as a slider in %.
fn share_slider(ui: &mut Ui, v: f64, w: f32) -> Option<f64> {
    let mut pct = (v * 100.0).round();
    ui.spacing_mut().slider_width = (w - 60.0).max(40.0);
    let r = ui.add(
        egui::Slider::new(&mut pct, 0.0..=100.0)
            .suffix(" %")
            .max_decimals(0),
    );
    r.changed().then_some(pct / 100.0)
}

impl ArchiSimApp {
    /// The species the tool plants: its own, or the Vegetación tool's,
    /// or an olive tree.
    fn planting_species(&mut self) -> Vec<ObjectTypeId> {
        let model = self.ed.model();
        let mut v: Vec<ObjectTypeId> = self
            .planting_settings
            .species
            .iter()
            .copied()
            .filter(|t| model.object_type(*t).is_some())
            .collect();
        if v.is_empty() {
            if let Some(t) = self
                .object_settings
                .object_type
                .filter(|t| model.type_grows(*t))
            {
                v.push(t);
            } else if let Some(t) =
                self.object_pick_type(super::object_gallery::ObjectPick::Library("Olivo"))
            {
                v.push(t);
            }
        }
        self.planting_settings.species = v.clone();
        v
    }

    pub(super) fn planting_click(&mut self, p: Point2) {
        let reach = self.view.to_metres(REACH);
        if self.planting_points.len() >= 3 && self.planting_points[0].distance(p) < reach {
            self.finish_planting(true);
            return;
        }
        if !self.planting_points.last().is_some_and(|q| q.approx_eq(p)) {
            self.planting_points.push(p);
        }
    }

    /// Makes the planting along the points drawn: closed if asked or if
    /// it plants inside an outline.
    pub(super) fn finish_planting(&mut self, closed: bool) {
        let mut path = std::mem::take(&mut self.planting_points);
        let area = self.planting_settings.arrangement == Arrangement::Area;
        // A line from a spline or the drawing comes already curved.
        let smooth = self.planting_settings.smooth && path.len() <= 40;
        if path.len() > 1 && path[0].approx_eq(path[path.len() - 1]) {
            path.pop();
        }
        let closed = closed || area;
        if path.len() < 2 || (closed && path.len() < 3) {
            if area {
                self.complain("Para plantar dentro de un contorno hacen falta tres puntos o más.");
            }
            return;
        }
        let species = self.planting_species();
        if species.is_empty() {
            self.complain("No hay especie que plantar: elige una en Propiedades.");
            return;
        }
        let s = self.planting_settings.clone();
        let level = self.ed.level;
        let r = self.ed.transact("Crear plantación", |m| {
            m.add_planting(NewPlanting {
                name: m.next_planting_name(),
                level,
                path,
                closed,
                smooth,
                arrangement: s.arrangement,
                count: s.count,
                species,
                size_variation: s.size_variation,
                shape_variation: s.shape_variation,
                spread: s.spread,
                seed: s.seed,
                base_offset: 0.0,
            })
        });
        match r {
            Ok(id) => {
                self.ed.selection = [Element::Planting(id)].into_iter().collect();
                let (name, n) = self
                    .ed
                    .model()
                    .planting(id)
                    .map(|p| (p.name.clone(), p.members.len()))
                    .unwrap_or_default();
                self.say(format!(
                    "{name}: {n} plantas. Número, especies y variación, en Propiedades."
                ));
                // The next one, other plants.
                self.planting_settings.seed = new_seed(self.planting_settings.seed);
            }
            Err(e) => self.complain(format!("No se puede plantar: {e}.")),
        }
    }

    /// While drawing: the line and the plants where they will go. Always:
    /// the lines of the plantings of the level (they are not printed).
    pub(super) fn draw_planting_preview(&self, painter: &egui::Painter, rect: Rect) {
        let model = self.ed.model();
        let guide = Stroke::new(
            1.0,
            egui::Color32::from_rgb(92, 140, 84).gamma_multiply(0.7),
        );
        for p in model.plantings_on_level(self.ed.level) {
            let line = archisim_core::planting_line(&p.path, p.closed, p.smooth);
            let pts = self.polygon(rect, &line);
            for seg in pts.windows(2).step_by(2) {
                painter.line_segment([seg[0], seg[1]], guide);
            }
            for q in &p.path {
                let s = self.view.to_screen(rect, *q);
                painter.circle_stroke(s, 2.5, guide);
            }
        }
        if self.tool != Tool::Planting || self.planting_points.is_empty() {
            return;
        }
        let mut path = self.planting_points.clone();
        if let Some(c) = self.cursor {
            path.push(c.point);
        }
        let s = &self.planting_settings;
        let area = s.arrangement == Arrangement::Area;
        let closed = area && path.len() >= 3;
        let line = archisim_core::planting_line(&path, closed, s.smooth);
        let ink = Stroke::new(1.5, ACCENT);
        painter.add(Shape::line(self.polygon(rect, &line), ink));
        let species: Vec<ObjectTypeId> = if s.species.is_empty() {
            self.object_settings.object_type.into_iter().collect()
        } else {
            s.species.clone()
        };
        if species.is_empty() || path.len() < 2 {
            return;
        }
        let spec = NewPlanting {
            name: String::new(),
            level: self.ed.level,
            path,
            closed,
            smooth: s.smooth,
            arrangement: s.arrangement,
            count: s.count,
            species,
            size_variation: s.size_variation,
            shape_variation: s.shape_variation,
            spread: s.spread,
            seed: s.seed,
            base_offset: 0.0,
        };
        let soft = Stroke::new(1.0, ACCENT.gamma_multiply(0.7));
        for spot in archisim_core::planting_spots(&spec) {
            let r = model
                .object_type_box(spot.object_type)
                .map_or(0.5, |(lo, hi)| (hi[0] - lo[0]) / 2.0)
                * spot.size;
            let c = self.view.to_screen(rect, spot.position);
            let rr = (r / self.view.to_metres(1.0)) as f32;
            painter.circle_stroke(c, rr.max(2.0), soft);
            painter.circle_filled(c, 1.5, ACCENT);
        }
    }

    pub(super) fn planting_hint(&self) -> &'static str {
        let area = self.planting_settings.arrangement == Arrangement::Area;
        match (self.planting_points.is_empty(), area) {
            (true, false) => "Clic: comienzo de la línea de plantación (o Spline / Elegir línea arriba) · Esc: flecha",
            (true, true) => "Clic: primera esquina del contorno a plantar · Esc: flecha",
            (false, false) => "Clic: siguiente punto · Doble clic o Intro: plantar · Clic en el primero: cerrarla · Esc: quitar el último",
            (false, true) => "Clic: siguiente esquina · Doble clic, Intro o clic en la primera: plantar · Esc: quitar la última",
        }
    }

    // ---- Properties -----------------------------------------------------

    /// The rows of a planting's rules; returns the changed settings.
    fn planting_rows(
        &mut self,
        ui: &mut Ui,
        salt: &str,
        s: &PlantingSettings,
    ) -> Option<PlantingSettings> {
        let mut out = s.clone();
        let mut changed = false;
        row(
            ui,
            "Distribución",
            "A lo largo de la línea, o repartidas dentro del contorno cerrado",
            |ui, w| {
                let v = props::choice(
                    ui,
                    format!("{salt}-arr"),
                    w,
                    s.arrangement == Arrangement::Area,
                    &[
                        (false, "A lo largo de la línea"),
                        (true, "Dentro del contorno"),
                    ],
                );
                if v != (s.arrangement == Arrangement::Area) {
                    out.arrangement = if v {
                        Arrangement::Area
                    } else {
                        Arrangement::Line
                    };
                    changed = true;
                }
            },
        );
        row(ui, "Número de plantas", "Cuántas se reparten", |ui, _| {
            let mut n = s.count as f64;
            if ui
                .add(
                    egui::DragValue::new(&mut n)
                        .range(1.0..=5000.0)
                        .speed(0.2)
                        .max_decimals(0),
                )
                .changed()
            {
                out.count = n.round().max(1.0) as u32;
                changed = true;
            }
        });
        row(
            ui,
            "Variación de tamaño",
            "0 %: todas iguales; 100 %: ±40 %",
            |ui, w| {
                if let Some(v) = share_slider(ui, s.size_variation, w) {
                    out.size_variation = v;
                    changed = true;
                }
            },
        );
        row(
            ui,
            "Variación de forma",
            "Cuánto se aparta cada planta de su especie",
            |ui, w| {
                if let Some(v) = share_slider(ui, s.shape_variation, w) {
                    out.shape_variation = v;
                    changed = true;
                }
            },
        );
        row(
            ui,
            "Irregularidad",
            "0 %: en fila y a distancias iguales; 100 %: al azar",
            |ui, w| {
                if let Some(v) = share_slider(ui, s.spread, w) {
                    out.spread = v;
                    changed = true;
                }
            },
        );
        row(
            ui,
            "Curva por los puntos",
            "La línea pasa en curva suave por los puntos marcados",
            |ui, _| {
                let mut on = s.smooth;
                if ui.switch(&mut on, "").changed() {
                    out.smooth = on;
                    changed = true;
                }
            },
        );
        row(
            ui,
            "Azar",
            "Otro reparto y otras plantas, con las mismas reglas",
            |ui, _| {
                ui.label(egui::RichText::new(s.seed.to_string()).small());
                if ui
                    .button("🎲 Otro")
                    .on_hover_text("Otras posiciones y otras plantas")
                    .clicked()
                {
                    out.seed = new_seed(s.seed);
                    changed = true;
                }
            },
        );
        // Species: the first by the gallery, the others listed.
        ui.add_space(4.0);
        ui.label(egui::RichText::new("Especies (se mezclan al azar)").strong());
        let mut remove = None;
        for (i, t) in s.species.iter().enumerate() {
            let name = self
                .ed
                .model()
                .object_type(*t)
                .map_or("—".to_string(), |x| x.name.clone());
            ui.horizontal(|ui| {
                ui.label(format!("· {name}"));
                if s.species.len() > 1
                    && ui
                        .small_button("✕")
                        .on_hover_text("Quitar esta especie")
                        .clicked()
                {
                    remove = Some(i);
                }
            });
        }
        if let Some(i) = remove {
            out.species.remove(i);
            changed = true;
        }
        self.scoped_card = true;
        let saved = self.object_scope;
        self.object_scope = Some(ObjectScope::Vegetation);
        let picked = self.object_card(ui, &format!("{salt}-species"), s.species.first().copied());
        self.object_scope = saved;
        self.scoped_card = false;
        if let Some(pick) = picked {
            if let Some(t) = self.object_pick_type(pick) {
                if ui.input(|i| i.modifiers.shift) || s.species.is_empty() {
                    out.species.push(t);
                } else {
                    out.species[0] = t;
                }
                out.species.dedup();
                changed = true;
            }
        }
        if let Some(t) = self
            .object_settings
            .object_type
            .filter(|t| !out.species.contains(t))
        {
            if self.ed.model().type_grows(t) {
                let name = self
                    .ed
                    .model()
                    .object_type(t)
                    .map(|x| x.name.clone())
                    .unwrap_or_default();
                if ui
                    .button(format!("+ Mezclar con {name}"))
                    .on_hover_text("Añade a la plantación la especie de la herramienta Vegetación")
                    .clicked()
                {
                    out.species.push(t);
                    changed = true;
                }
            }
        }
        props::note(
            ui,
            "Elegir en la galería cambia la primera especie; con Mayús, se añade a la mezcla.",
        );
        changed.then_some(out)
    }

    pub(super) fn planting_tool_properties(&mut self, ui: &mut Ui) {
        props::title(ui, "Herramienta Plantación · lo que vas a plantar");
        self.planting_species();
        let s = self.planting_settings.clone();
        group(ui, "tool-planting", "Plantación", |ui| {
            if let Some(n) = self.planting_rows(ui, "tool-planting", &s) {
                self.planting_settings = n;
            }
        });
        props::note(
            ui,
            "Dibuja la línea con clics (o Spline o Elegir línea en la barra de arriba): las plantas se ven antes de plantarlas. Doble clic o Intro: plantar. Después, la plantación se cambia en Propiedades y sus plantas se rehacen.",
        );
    }

    pub(super) fn planting_properties(&mut self, ui: &mut Ui, id: PlantingId) {
        let Some(p) = self.ed.model().planting(id).cloned() else {
            return;
        };
        props::title(ui, "Plantación · 1 seleccionada");
        super::properties::view_card(
            ui,
            super::icons::Icon::Object,
            &p.name,
            "IfcGeographicElement",
        );
        let line = archisim_core::planting_line(&p.path, p.closed, p.smooth);
        let length: f64 = line.windows(2).map(|w| w[0].distance(w[1])).sum();
        group(ui, "planting-data", "Plantación", |ui| {
            text_row(
                ui,
                "Plantas",
                "Las que ha plantado",
                &p.members.len().to_string(),
            );
            let measure = if p.arrangement == Arrangement::Area {
                format!("{:.2} m²", archisim_core::signed_area(&p.path).abs()).replace('.', ",")
            } else {
                format!("{length:.2} m").replace('.', ",")
            };
            text_row(
                ui,
                "Línea",
                "Longitud de la línea, o superficie del contorno",
                &measure,
            );
            let s = PlantingSettings {
                arrangement: p.arrangement,
                count: p.count,
                species: p.species.clone(),
                size_variation: p.size_variation,
                shape_variation: p.shape_variation,
                spread: p.spread,
                seed: p.seed,
                smooth: p.smooth,
            };
            if let Some(n) = self.planting_rows(ui, &format!("planting-{}", id.0), &s) {
                if n.arrangement == Arrangement::Area && !p.closed && p.path.len() < 3 {
                    self.complain(
                        "Para plantar dentro, la línea tiene que tener tres puntos o más.",
                    );
                } else {
                    let r = self.ed.transact("Cambiar plantación", |m| {
                        m.edit_planting(id, |x| {
                            x.arrangement = n.arrangement;
                            if n.arrangement == Arrangement::Area {
                                x.closed = true;
                            }
                            x.count = n.count;
                            x.species = n.species.clone();
                            x.size_variation = n.size_variation;
                            x.shape_variation = n.shape_variation;
                            x.spread = n.spread;
                            x.seed = n.seed;
                            x.smooth = n.smooth;
                        })
                    });
                    self.report(r, "Plantación rehecha.");
                }
            }
        });
        if ui
            .button("Seleccionar sus plantas")
            .on_hover_text("Para verlas o cambiar su especie a la vez")
            .clicked()
        {
            self.ed.selection = p.members.iter().map(|m| Element::Object(*m)).collect();
        }
        group(ui, "planting-identity", "Datos de identidad", |ui| {
            let mut name = p.name.clone();
            row(ui, "Nombre", "", |ui, w| {
                if ui
                    .add(egui::TextEdit::singleline(&mut name).desired_width(w - 8.0))
                    .lost_focus()
                    && name != p.name
                {
                    let n = name.clone();
                    let r = self.ed.transact("Renombrar plantación", |m| {
                        m.edit_planting(id, |x| x.name = n)
                    });
                    self.report(r, "Plantación renombrada.");
                }
            });
            props::id_row(ui, "GlobalId", &p.guid.to_string());
        });
    }
}
