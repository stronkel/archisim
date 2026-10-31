//! Bringing a DXF into a view: before it comes in, the drawing's unit
//! (what the file says, or what its size gives away — many are drawn in
//! metres and say millimetres or nothing) with the size it will have, and
//! whether each block placed stays a group, as it was in the drawing.

use std::path::{Path, PathBuf};

use archisim_export::dxf_read::Drawing;
use eframe::egui;

use super::widgets::SwitchExt as _;
use super::*;

/// A DXF read, waiting to come in.
pub struct DxfImport {
    pub path: PathBuf,
    /// As read (in the unit the file says).
    pub drawing: Drawing,
    /// The unit chosen, metres per drawing unit.
    pub unit: f64,
    /// Each block placed kept as a group.
    pub groups: bool,
}

/// The units offered: metres per unit and name.
const UNITS: &[(f64, &str)] = &[
    (0.001, "Milímetros"),
    (0.01, "Centímetros"),
    (1.0, "Metros"),
    (0.0254, "Pulgadas"),
];

fn unit_name(u: f64) -> &'static str {
    UNITS
        .iter()
        .find(|(v, _)| (v - u).abs() < 1e-12)
        .map_or("Otra", |(_, n)| *n)
}

impl ArchiSimApp {
    pub(super) fn open_dxf_import(&mut self, path: &Path) {
        let read = std::fs::read(path)
            .map_err(|e| format!("no se pudo leer el DXF: {e}"))
            .and_then(|b| archisim_export::dxf_read::read(&b));
        match read {
            Ok(drawing) if drawing.items.is_empty() => self.complain(
                "No se pudo importar: el DXF no tiene nada que ArchiSim sepa dibujar en 2D.",
            ),
            Ok(drawing) => {
                let unit = drawing.likely_unit();
                self.dxf_import = Some(DxfImport {
                    path: path.to_path_buf(),
                    drawing,
                    unit,
                    groups: true,
                });
            }
            Err(e) => self.complain(format!("No se pudo importar: {e}.")),
        }
    }

    /// Brings in the DXF waiting, as chosen.
    pub(crate) fn confirm_dxf_import(&mut self) {
        let Some(imp) = self.dxf_import.take() else {
            return;
        };
        let mut drawing = imp.drawing.in_unit(imp.unit);
        if !imp.groups {
            for it in &mut drawing.items {
                it.block = None;
            }
        }
        let name = imp
            .path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        let (view, k) = (self.active_view(), self.active_k());
        match self.ed.import_dxf_drawing(&drawing, &name, view, k) {
            Ok(r) => {
                self.say(r.describe());
                self.fit_requested = true;
            }
            Err(e) => self.complain(format!("No se pudo importar: {e}.")),
        }
    }

    pub(super) fn dxf_import_dialog(&mut self, ctx: &egui::Context) {
        let Some(imp) = self.dxf_import.as_mut() else {
            return;
        };
        let mut open = true;
        let mut go = false;
        let mut cancelled = false;
        let pal = super::theme::Palette::current(ctx);
        super::widgets::dialog("Importar DXF")
            .open(&mut open)
            .resizable(false)
            .default_width(380.0)
            .show(ctx, |ui| {
                ui.label(
                    egui::RichText::new(
                        imp.path
                            .file_name()
                            .map(|f| f.to_string_lossy().into_owned())
                            .unwrap_or_default(),
                    )
                    .strong(),
                );
                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    ui.label("Unidades del dibujo");
                    egui::ComboBox::from_id_salt("dxf-unit")
                        .width(150.0)
                        .selected_text(unit_name(imp.unit))
                        .show_ui(ui, |ui| {
                            for (u, n) in UNITS {
                                if ui
                                    .selectable_label((imp.unit - u).abs() < 1e-12, *n)
                                    .clicked()
                                {
                                    imp.unit = *u;
                                }
                            }
                        });
                });
                let said = if imp.drawing.unit_stated {
                    format!(
                        "El fichero dice {}",
                        unit_name(imp.drawing.unit).to_lowercase()
                    )
                } else {
                    "El fichero no dice sus unidades".to_string()
                };
                let guessed = imp.drawing.likely_unit();
                let note = if (guessed - imp.drawing.unit).abs() > 1e-12 {
                    format!(
                        "{said}; por su tamaño parece dibujado en {}.",
                        unit_name(guessed).to_lowercase()
                    )
                } else {
                    format!("{said}.")
                };
                ui.label(egui::RichText::new(note).small().color(pal.muted));
                if let Some((lo, hi)) = imp.drawing.extent() {
                    let f = imp.unit / imp.drawing.unit;
                    let (w, h) = ((hi.x - lo.x) * f, (hi.y - lo.y) * f);
                    ui.label(format!("Medirá {w:.2} × {h:.2} m").replace('.', ","));
                }
                let blocks = imp
                    .drawing
                    .items
                    .iter()
                    .filter_map(|i| i.block.as_ref().map(|b| b.0))
                    .collect::<std::collections::BTreeSet<_>>()
                    .len();
                if blocks > 0 {
                    ui.add_space(4.0);
                    ui.switch(
                        &mut imp.groups,
                        format!("Cada bloque como un grupo ({blocks})"),
                    );
                    ui.label(
                        egui::RichText::new(
                            "Un clic elige el bloque entero; para tocar sus líneas, Desagrupar.",
                        )
                        .small()
                        .color(pal.muted),
                    );
                }
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    if ui.button("Importar").clicked() {
                        go = true;
                    }
                    if ui.button("Cancelar").clicked() {
                        cancelled = true;
                    }
                });
            });
        if go {
            self.confirm_dxf_import();
        } else if !open || cancelled {
            self.dxf_import = None;
        }
    }
}
