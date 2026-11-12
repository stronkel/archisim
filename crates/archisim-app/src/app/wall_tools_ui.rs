//! Walls, second round (E3), in a wall's properties: split it in two
//! (Revit's «Dividir elemento») and paint a band of one of its faces —
//! a skirting, a dado, a stripe — without drawing in Concepto.

use archisim_core::{FaceSide, MaterialId, Point2, Wall, WallId};
use eframe::egui::{self, Id, Ui};

use super::props::{choice, group, row};
use super::*;

/// The band being prepared in a wall's properties.
#[derive(Debug, Clone, Copy)]
pub(crate) struct WallBand {
    pub side: FaceSide,
    /// From and to, up from the wall's base (m).
    pub from: f64,
    pub to: f64,
    pub material: Option<MaterialId>,
    /// Where the wall is split, from its start (m; none: halfway).
    pub split_at: Option<f64>,
}

impl Default for WallBand {
    fn default() -> Self {
        Self {
            side: FaceSide::Interior,
            from: 0.0,
            to: 1.0,
            material: None,
            split_at: None,
        }
    }
}

impl ArchiSimApp {
    /// The «Caras y división» group of a wall's properties.
    pub(super) fn wall_tools_group(&mut self, ui: &mut Ui, id: WallId, w: &Wall) {
        let len = w.start.distance(w.end);
        let mut band = self.wall_band;
        if band.material.is_none() {
            band.material = self.ed.model().materials().next().map(|m| m.id);
        }
        let mut paint = false;
        let mut split = false;
        let mut pick_paint = false;
        group(ui, "wall-tools", "Caras y división", |ui| {
            row(
                ui,
                "Franja: cara",
                "La cara del muro que se pinta",
                |ui, wd| {
                    band.side = choice(
                        ui,
                        ("wall-band-side", id.0),
                        wd,
                        band.side,
                        &[
                            (FaceSide::Interior, "Interior"),
                            (FaceSide::Exterior, "Exterior"),
                        ],
                    );
                },
            );
            row(
                ui,
                "Franja: desde (m)",
                "Sobre la base del muro",
                |ui, wd| {
                    if let Some(v) =
                        length_field(ui, Id::unique(("wall-band-from", id)), band.from, wd - 8.0)
                    {
                        band.from = v.max(0.0);
                    }
                },
            );
            row(
                ui,
                "Franja: hasta (m)",
                "Sobre la base del muro (un zócalo: 1,00)",
                |ui, wd| {
                    if let Some(v) =
                        length_field(ui, Id::unique(("wall-band-to", id)), band.to, wd - 8.0)
                    {
                        band.to = v.max(0.0);
                    }
                },
            );
            row(ui, "Franja: material", "", |ui, wd| {
                if let Some(mut m) = band.material {
                    super::cards::material_button(
                        ui,
                        Id::unique(("wall-band-mat", id)),
                        self.ed.model(),
                        &mut m,
                        wd - 8.0,
                    );
                    band.material = Some(m);
                }
            });
            row(ui, "", "", |ui, _| {
                if ui
                    .button("Pintar franja")
                    .on_hover_text("A todo lo largo del muro, en la cara elegida")
                    .clicked()
                {
                    paint = true;
                }
            });
            row(ui, "", "", |ui, _| {
                if ui
                    .button("Pintar a mano…")
                    .on_hover_text("La herramienta Pintar: un clic en una cara del muro (en 3D, también en la zona entre aristas dibujadas en ella)")
                    .clicked()
                {
                    pick_paint = true;
                }
            });
            let at = band.split_at.unwrap_or(len / 2.0).clamp(0.0, len);
            row(ui, "Dividir a (m)", "Desde el inicio del muro", |ui, wd| {
                if let Some(v) = length_field(ui, Id::unique(("wall-split", id)), at, wd - 8.0) {
                    band.split_at = Some(v);
                }
            });
            row(ui, "", "", |ui, _| {
                if ui
                    .button("Dividir en dos")
                    .on_hover_text("Lo parte en dos: cada parte puede tener luego su tipo, su altura y su pintura")
                    .clicked()
                {
                    split = true;
                }
            });
        });
        self.wall_band = band;
        if pick_paint {
            self.concept_pick(super::concept_ui::ConceptPick::Paint);
        }
        if paint {
            let (lo, hi) = (band.from.min(band.to), band.from.max(band.to));
            let Some(material) = band.material else {
                return;
            };
            if hi - lo < 0.005 {
                self.complain("La franja no tiene altura.");
                return;
            }
            let (y0, y1) = (w.base_offset + lo, w.base_offset + hi);
            let outline = vec![
                Point2::new(0.0, y0),
                Point2::new(len, y0),
                Point2::new(len, y1),
                Point2::new(0.0, y1),
            ];
            let r = self.ed.transact("Pintar franja", |m| {
                m.set_paint_zone(id, band.side, outline, Some(material))
            });
            self.report(r, "Franja pintada.");
        }
        if split {
            let at = band.split_at.unwrap_or(len / 2.0);
            let r = self
                .ed
                .transact("Dividir muro", |m| m.split_wall(id, at).map(|_| ()));
            self.wall_band.split_at = None;
            self.report(r, "Muro dividido en dos.");
        }
    }
}

impl ArchiSimApp {
    /// The «Despiece a mano» group of a wall's properties (M20): each of
    /// its layers of boards drawn by hand on the face, or by its
    /// material's rule.
    pub(super) fn board_face_group(&mut self, ui: &mut Ui, id: WallId, w: &Wall) {
        let m = self.ed.model();
        let Some(t) = m.wall_type(w.wall_type) else {
            return;
        };
        // Each board layer: its index, name and, if drawn by hand, its
        // joints and pieces on site.
        type Row = (usize, String, Option<(usize, usize)>);
        let layers: Vec<Row> = t
            .layers
            .iter()
            .enumerate()
            .filter(|(_, l)| archisim_geom::boards::laying(m, l.material, l.thickness).is_some())
            .map(|(i, l)| {
                let name = m
                    .material(l.material)
                    .map_or_else(|| format!("capa {}", i + 1), |x| x.name.clone());
                let by_hand = m
                    .board_face(id, i)
                    .map(|f| (f.joints.len(), f.marks.iter().filter(|k| k.on_site).count()));
                (i, name, by_hand)
            })
            .collect();
        if layers.is_empty() {
            return;
        }
        let mut draw: Option<usize> = None;
        let mut back: Option<usize> = None;
        let mut copy: Option<(usize, bool)> = None;
        let like_count = m.walls_like(id).len();
        group(ui, "wall-board-faces", "Despiece a mano", |ui| {
            for (i, name, by_hand) in &layers {
                let state = match by_hand {
                    Some((j, 0)) => format!("A mano, {j} juntas"),
                    Some((j, s)) => format!("A mano, {j} juntas, {s} en obra"),
                    None => "Por la regla".to_string(),
                };
                row(ui, &format!("Capa {}", i + 1), name, |ui, _| {
                    ui.add(egui::Label::new(state).truncate())
                        .on_hover_text(name.as_str());
                });
                row(ui, "", "", |ui, _| {
                    if ui
                        .button("Dibujar…")
                        .on_hover_text("El despiece de esta capa en alzado, visto desde fuera: las juntas son líneas (empieza con las de la regla); luego marca piezas de acento o para colocar en obra")
                        .clicked()
                    {
                        draw = Some(*i);
                    }
                });
                if by_hand.is_some() {
                    row(ui, "", "", |ui, _| {
                        let like = like_count;
                        let r = ui.add_enabled(like > 0, egui::Button::new(format!("Copiar a iguales ({like})")))
                            .on_hover_text("A los muros del mismo tipo, largo y alto: las mismas juntas y piezas")
                            .on_disabled_hover_text("No hay otro muro del mismo tipo, largo y alto");
                        if r.clicked() {
                            copy = Some((*i, false));
                        }
                    });
                    row(ui, "", "", |ui, _| {
                        if ui
                            .add_enabled(like_count > 0, egui::Button::new("… en simetría"))
                            .on_hover_text(
                                "Lo mismo, reflejado: el inicio de este muro en el final del otro",
                            )
                            .clicked()
                        {
                            copy = Some((*i, true));
                        }
                    });
                    row(ui, "", "", |ui, _| {
                        if ui
                            .button("A la regla")
                            .on_hover_text(
                                "Quita el despiece a mano de esta capa: vuelve el de su material",
                            )
                            .clicked()
                        {
                            back = Some(*i);
                        }
                    });
                }
            }
        });
        if let Some(i) = draw {
            self.start_sketch(crate::sketch::SketchTarget::BoardFace(id, i));
        }
        if let Some((i, reflected)) = copy {
            let to = self.ed.model().walls_like(id);
            match self.ed.transact("Copiar el despiece", |m| {
                m.copy_board_face(id, i, &to, reflected)
            }) {
                Ok(n) => self.say(format!(
                    "Despiece copiado a {n} muro{}.",
                    if n == 1 { "" } else { "s" }
                )),
                Err(e) => self.complain(format!("No se puede copiar: {e}.")),
            }
        }
        if let Some(i) = back {
            match self
                .ed
                .transact("Volver a la regla", |m| m.set_board_face(id, i, None))
            {
                Ok(()) => self.say("La capa vuelve al despiece de su material."),
                Err(e) => self.complain(format!("No se puede: {e}.")),
            }
        }
    }

    /// «Despiece a mano» of a slab or a ceiling: each layer of boards by
    /// its material's rule or drawn by hand in plan.
    pub(super) fn board_plan_group(&mut self, ui: &mut Ui, host: archisim_core::PlanHost) {
        let m = self.ed.model();
        let Some(layers) = m.plan_host_layers(host) else {
            return;
        };
        type Row = (usize, String, Option<(usize, usize)>);
        let layers: Vec<Row> = layers
            .iter()
            .enumerate()
            .filter(|(_, l)| archisim_geom::boards::laying(m, l.material, l.thickness).is_some())
            .map(|(i, l)| {
                let name = m
                    .material(l.material)
                    .map_or_else(|| format!("capa {}", i + 1), |x| x.name.clone());
                let by_hand = m
                    .board_plan(host, i)
                    .map(|f| (f.joints.len(), f.marks.iter().filter(|k| k.on_site).count()));
                (i, name, by_hand)
            })
            .collect();
        if layers.is_empty() {
            return;
        }
        let mut draw: Option<usize> = None;
        let mut back: Option<usize> = None;
        group(ui, "plan-board-faces", "Despiece a mano", |ui| {
            for (i, name, by_hand) in &layers {
                let state = match by_hand {
                    Some((j, 0)) => format!("A mano, {j} juntas"),
                    Some((j, s)) => format!("A mano, {j} juntas, {s} en obra"),
                    None => "Por la regla".to_string(),
                };
                row(ui, &format!("Capa {}", i + 1), name, |ui, _| {
                    ui.add(egui::Label::new(state).truncate())
                        .on_hover_text(name.as_str());
                });
                row(ui, "", "", |ui, _| {
                    if ui
                        .button("Dibujar…")
                        .on_hover_text("El despiece de esta capa en planta: las juntas son líneas que cruzan de lado a lado; luego marca piezas de acento o para colocar en obra")
                        .clicked()
                    {
                        draw = Some(*i);
                    }
                });
                if by_hand.is_some() {
                    row(ui, "", "", |ui, _| {
                        if ui
                            .button("A la regla")
                            .on_hover_text(
                                "Quita el despiece a mano de esta capa: vuelve el de su material",
                            )
                            .clicked()
                        {
                            back = Some(*i);
                        }
                    });
                }
            }
        });
        if let Some(i) = draw {
            // Drawn on the plan of its level (a ceiling, on its ceiling plan).
            let level = match host {
                archisim_core::PlanHost::Slab(s) => self.ed.model().slab(s).map(|s| s.level),
                archisim_core::PlanHost::Ceiling(c) => self.ed.model().ceiling(c).map(|c| c.level),
            };
            if let Some(level) = level {
                if level != self.ed.level {
                    self.go_to_level_plan(level);
                }
            }
            self.start_sketch(crate::sketch::SketchTarget::BoardPlan(host, i));
        }
        if let Some(i) = back {
            match self
                .ed
                .transact("Volver a la regla", |m| m.set_board_plan(host, i, None))
            {
                Ok(()) => self.say("La capa vuelve al despiece de su material."),
                Err(e) => self.complain(format!("No se puede: {e}.")),
            }
        }
    }
}
