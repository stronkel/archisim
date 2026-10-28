//! The pool tool (paquete 8, tanda 8d): a pool built in the ground by the
//! outline of its water, like a slab (polygon or rectangle), with its
//! depths, slope, shell and coping; and the properties of a pool.

use archisim_core::{MaterialId, Model, NewPool, Point2, PoolId};
use eframe::egui::{self, Id, Ui};

use super::props::{self, group, row, text_row};
use super::structure_ui::angle_field;
use super::urban_ui::material_combo;
use super::*;

/// What the pool tool draws: its measurements and materials (none: the
/// usual ones of the palette, made when the first pool is built).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PoolSettings {
    pub shallow: f64,
    pub deep: f64,
    pub freeboard: f64,
    pub wall: f64,
    pub coping: f64,
    pub top_offset: f64,
    pub lining: Option<MaterialId>,
    pub shell: Option<MaterialId>,
    pub coping_material: Option<MaterialId>,
    pub water: Option<MaterialId>,
}

impl Default for PoolSettings {
    fn default() -> Self {
        PoolSettings {
            shallow: 1.0,
            deep: 2.0,
            freeboard: 0.12,
            wall: 0.25,
            coping: 0.3,
            top_offset: 0.0,
            lining: None,
            shell: None,
            coping_material: None,
            water: None,
        }
    }
}

/// The palette's material of that name, made if the project has none.
fn palette(m: &mut Model, name: &str) -> MaterialId {
    archisim_library::palette_material(m, name)
        .or_else(|| m.materials().next().map(|x| x.id))
        .unwrap_or(MaterialId(0))
}

fn comma(v: f64, d: usize) -> String {
    format!("{v:.d$}").replace('.', ",")
}

impl ArchiSimApp {
    pub(super) fn pool_click(&mut self, p: Point2) {
        match self.pool_tool.click(p) {
            SlabOutcome::Build(outline) => self.build_pool(outline),
            SlabOutcome::Invalid => self.complain(
                "El contorno de la piscina no puede cruzarse: corrige la última esquina.",
            ),
            SlabOutcome::Nothing => {}
        }
    }

    pub(super) fn build_pool(&mut self, outline: Vec<Point2>) {
        let level = self.ed.level;
        let s = self.pool_settings;
        // The slope along the first side drawn, from its start to its end.
        let slope = match (outline.first(), outline.get(1)) {
            (Some(a), Some(b)) if a.distance(*b) > 1e-6 => (b.y - a.y).atan2(b.x - a.x),
            _ => 0.0,
        };
        let r = self.ed.transact("Crear piscina", |m| {
            let lining = s.lining.unwrap_or_else(|| palette(m, "Gresite azul"));
            let mut spec = NewPool::new(level, outline, lining);
            spec.shallow = s.shallow;
            spec.deep = s.deep;
            spec.freeboard = s.freeboard;
            spec.wall = s.wall;
            spec.coping = s.coping;
            spec.top_offset = s.top_offset;
            spec.slope = slope;
            spec.shell = s.shell.unwrap_or_else(|| palette(m, "Hormigón pulido"));
            spec.coping_material = s
                .coping_material
                .unwrap_or_else(|| palette(m, "Piedra caliza"));
            spec.water = s.water.unwrap_or_else(|| palette(m, "Agua de fuente"));
            m.add_pool(spec)
        });
        match r {
            Ok(id) => {
                self.pool_tool.accept();
                self.ed.selection = [Element::Pool(id)].into_iter().collect();
                let (name, area, volume) = self
                    .ed
                    .model()
                    .pool(id)
                    .map(|p| (p.name.clone(), p.area(), p.water_volume()))
                    .unwrap_or_default();
                self.say(format!(
                    "{name}: {} m² de lámina de agua, {} m³. La pendiente va a lo largo del primer lado; se cambia en Propiedades.",
                    comma(area, 2),
                    comma(volume, 1)
                ));
            }
            Err(e) => self.complain(format!("No se puede crear la piscina: {e}.")),
        }
    }

    /// The outline being drawn, with its area.
    pub(super) fn draw_pool_preview(&self, painter: &egui::Painter, rect: Rect) {
        if self.tool != Tool::Pool {
            return;
        }
        let Some(c) = self.cursor else {
            return;
        };
        let ink = Stroke::new(1.5, ACCENT);
        let corners = self.pool_tool.corners();
        if corners.is_empty() {
            return;
        }
        if self.pool_tool.mode == SlabMode::Rectangle {
            if let Some(o) = self.pool_tool.rectangle_to(c.point) {
                painter.add(Shape::closed_line(self.polygon(rect, &o), ink));
                self.draw_label(
                    painter,
                    rect,
                    c.point,
                    format!("{} m²", comma(archisim_core::signed_area(&o).abs(), 2)),
                );
            }
            return;
        }
        let mut pts = corners.to_vec();
        pts.push(self.pool_tool.typed_end(c.point).unwrap_or(c.point));
        let screen = self.polygon(rect, &pts);
        painter.add(Shape::line(screen.clone(), ink));
        if pts.len() >= 3 {
            dashed(painter, screen[screen.len() - 1], screen[0], ACCENT);
            self.draw_label(
                painter,
                rect,
                c.point,
                format!("{} m²", comma(archisim_core::signed_area(&pts).abs(), 2)),
            );
        }
    }

    pub(super) fn pool_hint(&self) -> &'static str {
        if self.pool_tool.is_drawing() && self.pool_tool.mode == SlabMode::Rectangle {
            "Clic: esquina opuesta · Esc: anular"
        } else if self.pool_tool.is_drawing() {
            "Clic: siguiente esquina del borde del agua · Clic en la primera o Intro: cerrar · Esc: quitar la última"
        } else {
            "Clic: primera esquina del borde del agua (la pendiente irá a lo largo del primer lado) · Esc: flecha"
        }
    }

    /// The rows of a pool's measurements, from `s`; returns them changed.
    fn pool_rows(ui: &mut Ui, salt: u64, s: PoolSettings) -> Option<PoolSettings> {
        let mut out = s;
        let mut changed = false;
        let fields: [(&str, &str, f64); 6] = [
            (
                "Profundidad menor (mm)",
                "El agua en la zona poco profunda",
                s.shallow,
            ),
            (
                "Profundidad mayor (mm)",
                "El agua en la zona profunda",
                s.deep,
            ),
            (
                "Resguardo (mm)",
                "Lo que queda el agua bajo la coronación",
                s.freeboard,
            ),
            ("Vaso (mm)", "Grueso del hormigón del vaso", s.wall),
            ("Coronación (mm)", "Ancho de la coronación", s.coping),
            (
                "Coronación sobre la planta (mm)",
                "Altura de la coronación sobre la planta",
                s.top_offset,
            ),
        ];
        for (i, (name, hint, value)) in fields.into_iter().enumerate() {
            row(ui, name, hint, |ui, w| {
                if let Some(v) = length_field(ui, Id::unique(("pool-f", salt, i)), value, w - 8.0) {
                    match i {
                        0 => out.shallow = v.max(0.2),
                        1 => out.deep = v.max(0.2),
                        2 => out.freeboard = v.max(0.0),
                        3 => out.wall = v.max(0.05),
                        4 => out.coping = v.max(0.0),
                        _ => out.top_offset = v,
                    }
                    changed = true;
                }
            });
        }
        changed.then_some(out)
    }

    pub(super) fn pool_tool_properties(&mut self, ui: &mut Ui) {
        props::title(ui, "Herramienta Piscina · lo que vas a dibujar");
        let mut s = self.pool_settings;
        group(ui, "tool-pool", "Vaso", |ui| {
            if let Some(n) = Self::pool_rows(ui, 0, s) {
                s = n;
            }
            props::note(
                ui,
                "Se dibuja el borde del agua. La piscina recorta el terreno que haya debajo.",
            );
        });
        let model = self.ed.model();
        group(ui, "tool-pool-mat", "Materiales", |ui| {
            for (i, (name, value)) in [
                ("Revestimiento", s.lining),
                ("Vaso", s.shell),
                ("Coronación", s.coping_material),
                ("Agua", s.water),
            ]
            .into_iter()
            .enumerate()
            {
                row(ui, name, "Sin elegir: el de la paleta", |ui, w| {
                    let shown = value.unwrap_or(MaterialId(u64::MAX));
                    let n = material_combo(ui, Id::unique(("tool-pool-mat", i)), model, shown, w);
                    if n != shown {
                        match i {
                            0 => s.lining = Some(n),
                            1 => s.shell = Some(n),
                            2 => s.coping_material = Some(n),
                            _ => s.water = Some(n),
                        }
                    }
                });
            }
        });
        self.pool_settings = s;
    }

    pub(super) fn pool_properties(&mut self, ui: &mut Ui, id: PoolId) {
        let Some(p) = self.ed.model().pool(id).cloned() else {
            return;
        };
        props::title(ui, &format!("{} · 1 seleccionada", p.name));
        group(ui, "pool-data", "Piscina", |ui| {
            row(ui, "Nombre", "", |ui, w| {
                if let Some(t) =
                    text_edit_committed(ui, Id::unique(("pool-name", id)), &p.name, w - 8.0)
                {
                    let r = self.ed.transact("Nombre de la piscina", |m| {
                        m.edit_pool(id, |x| x.name = t.trim().to_string())
                    });
                    self.report(r, "Nombre cambiado.");
                }
            });
            self.level_row(
                ui,
                ("pool-level", id),
                "Planta",
                Some(p.level),
                &[Element::Pool(id)],
            );
        });
        group(ui, "pool-shell", "Vaso", |ui| {
            let s = PoolSettings {
                shallow: p.shallow,
                deep: p.deep,
                freeboard: p.freeboard,
                wall: p.wall,
                coping: p.coping,
                top_offset: p.top_offset,
                ..Default::default()
            };
            if let Some(n) = Self::pool_rows(ui, id.0, s) {
                let r = self.ed.transact("Medidas de la piscina", |m| {
                    m.edit_pool(id, |x| {
                        x.shallow = n.shallow;
                        x.deep = n.deep;
                        x.freeboard = n.freeboard;
                        x.wall = n.wall;
                        x.coping = n.coping;
                        x.top_offset = n.top_offset;
                    })
                });
                self.report(r, "Medidas cambiadas.");
            }
            row(
                ui,
                "Pendiente hacia (°)",
                "Dirección de la zona profunda, desde el eje X",
                |ui, w| {
                    if let Some(a) =
                        angle_field(ui, Id::unique(("pool-slope", id)), p.slope, w - 8.0)
                    {
                        let r = self.ed.transact("Pendiente de la piscina", |m| {
                            m.edit_pool(id, |x| x.slope = a)
                        });
                        self.report(r, "Pendiente cambiada.");
                    }
                },
            );
            ui.horizontal(|ui| {
                if ui
                    .button("Girar 90°")
                    .on_hover_text("La zona profunda pasa al lado siguiente")
                    .clicked()
                {
                    let r = self.ed.transact("Pendiente de la piscina", |m| {
                        m.edit_pool(id, |x| {
                            x.slope =
                                crate::editor::normal_angle(x.slope + std::f64::consts::FRAC_PI_2)
                        })
                    });
                    self.report(r, "Pendiente girada.");
                }
                if ui
                    .button("Invertir")
                    .on_hover_text("La zona profunda pasa al otro extremo")
                    .clicked()
                {
                    let r = self.ed.transact("Pendiente de la piscina", |m| {
                        m.edit_pool(id, |x| {
                            x.slope = crate::editor::normal_angle(x.slope + std::f64::consts::PI)
                        })
                    });
                    self.report(r, "Pendiente invertida.");
                }
            });
        });
        let model = self.ed.model();
        let mats = [p.lining, p.shell, p.coping_material, p.water];
        let mut picked = None;
        group(ui, "pool-mat", "Materiales", |ui| {
            for (i, name) in ["Revestimiento", "Vaso", "Coronación", "Agua"]
                .into_iter()
                .enumerate()
            {
                row(ui, name, "", |ui, w| {
                    let n = material_combo(ui, Id::unique(("pool-mat", id, i)), model, mats[i], w);
                    if n != mats[i] {
                        picked = Some((i, n));
                    }
                });
            }
        });
        if let Some((i, n)) = picked {
            let r = self.ed.transact("Material de la piscina", |m| {
                m.edit_pool(id, |x| match i {
                    0 => x.lining = n,
                    1 => x.shell = n,
                    2 => x.coping_material = n,
                    _ => x.water = n,
                })
            });
            self.report(r, "Material cambiado.");
        }
        group(ui, "pool-figures", "Mediciones", |ui| {
            text_row(ui, "Lámina de agua (m²)", "", &comma(p.area(), 2));
            text_row(ui, "Perímetro (m)", "", &comma(p.perimeter(), 2));
            text_row(ui, "Volumen de agua (m³)", "", &comma(p.water_volume(), 2));
            let dug = archisim_core::signed_area(&p.outer()).abs();
            let deepest = p.deep.max(p.shallow) + p.freeboard + p.wall;
            text_row(
                ui,
                "Excavación (m³)",
                "Huella del vaso por su fondo más bajo, aproximada",
                &comma(dug * deepest, 1),
            );
        });
        group(ui, "pool-identity", "Datos de identidad", |ui| {
            text_row(
                ui,
                "Clase IFC",
                "",
                "IfcElementAssembly «Piscina» (solera, paredes, revestimiento y coronación) y el agua como IfcSpace",
            );
        });
    }
}
