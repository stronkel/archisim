//! Walls, second round (E3), in a wall's properties: split it in two
//! (Revit's «Dividir elemento») and paint a band of one of its faces —
//! a skirting, a dado, a stripe — without drawing in Concepto.

use archisim_core::{FaceSide, MaterialId, Point2, Wall, WallId};
use eframe::egui::{Id, Ui};

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
                    .button("Pintar la franja")
                    .on_hover_text("A todo lo largo del muro, en la cara elegida")
                    .clicked()
                {
                    paint = true;
                }
            });
            row(ui, "", "", |ui, _| {
                if ui
                    .button("Pintar caras con el ratón…")
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
                    .button("Dividir el muro en dos")
                    .on_hover_text("Como «Dividir elemento» de Revit: cada parte puede tener luego su tipo, su altura y su pintura")
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
