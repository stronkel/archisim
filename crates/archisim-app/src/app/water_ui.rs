//! The water supply window (Colaboración → Cálculo de fontanería): what
//! the drawing does not give (the network's pressure at the meter and the
//! pipe), the calculation of the cold and hot networks as text, and
//! «Poner los diámetros calculados», which gives every pipe the diameter
//! its flow asks for in one undoable step; and the plumbing diagrams
//! (`archisim_plan::water_schematic`) in PDF or DXF.

use archisim_core::PipeMaterial;
use eframe::egui::{self, Ui};

use super::*;

/// The window's state.
#[derive(Default)]
pub struct WaterWindow {
    pub open: bool,
    /// The calculation of a revision of the model: (revision, text, the
    /// pipes whose diameter changes `(run, now, sized)`, taps short of
    /// pressure).
    results: Option<(u64, String, Vec<Resize>, usize)>,
    /// The calculation drawn (`report_view`), and whether shown as typed
    /// text.
    blocks: Vec<archisim_struct::report::Block>,
    as_text: bool,
}

/// A pipe whose diameter changes: the run, its diameter and the
/// calculated one.
type Resize = (archisim_core::RunId, f64, f64);

impl ArchiSimApp {
    pub(super) fn water_dialog(&mut self, ctx: &egui::Context) {
        if !self.water.open {
            return;
        }
        let mut open = true;
        super::widgets::dialog("Cálculo de fontanería (DB-HS 4)")
            .open(&mut open)
            .default_width(820.0)
            .default_height(560.0)
            .show(ctx, |ui| self.water_contents(ui));
        self.water.open = open;
    }

    fn water_contents(&mut self, ui: &mut Ui) {
        let site = *self.ed.model().water_site();
        let mut new = site;
        egui::Grid::new("water-site")
            .num_columns(2)
            .spacing([12.0, 6.0])
            .show(ui, |ui| {
                ui.label("Presión en el contador");
                ui.add(
                    egui::DragValue::new(&mut new.pressure)
                        .range(50.0..=1000.0)
                        .speed(5.0)
                        .suffix(" kPa"),
                )
                .on_hover_text("La que garantiza la compañía (pídela por escrito): 100 kPa ≈ 1 bar ≈ 10 m de agua");
                ui.end_row();
                ui.label("Pérdida en contador y filtro");
                ui.add(
                    egui::DragValue::new(&mut new.meter_loss)
                        .range(0.0..=200.0)
                        .speed(1.0)
                        .suffix(" kPa"),
                )
                .on_hover_text("Lo que pierden el contador, el filtro y las llaves de la acometida: el dato del fabricante (0: no se cuenta)");
                ui.end_row();
                ui.label("Pérdida en el calentador");
                ui.add(
                    egui::DragValue::new(&mut new.heater_loss)
                        .range(0.0..=200.0)
                        .speed(1.0)
                        .suffix(" kPa"),
                )
                .on_hover_text("Lo que pierde el agua al pasar por el termo o el acumulador: el dato del fabricante (0: no se cuenta)");
                ui.end_row();
                ui.label("Tubería");
                egui::ComboBox::from_id_salt("water-material")
                    .selected_text(new.material.name())
                    .width(260.0)
                    .show_ui(ui, |ui| {
                        for m in PipeMaterial::ALL {
                            ui.selectable_value(&mut new.material, m, m.name());
                        }
                    });
                ui.end_row();
            });
        if new != site {
            let r = self
                .ed
                .transact("Datos del suministro de agua", |m| m.set_water_site(new));
            if let Err(e) = r {
                self.complain(format!("No se pueden cambiar: {e}."));
            }
        }
        ui.label(
            egui::RichText::new(
                "El cálculo sigue las tuberías de agua fría desde el contador y las de caliente desde el termo, sube por los montantes y llega a los aparatos sanitarios dibujados. Si no hay contador, empieza en el extremo más bajo.",
            )
            .small(),
        );
        ui.separator();
        let rev = self.ed.revision();
        if self.water.results.as_ref().is_none_or(|r| r.0 != rev) {
            let model = self.ed.model();
            let w = archisim_mep::water::water(model);
            let text = archisim_mep::water::report(model, &w);
            let changes: Vec<_> = w
                .sized_runs()
                .into_iter()
                .filter_map(|(r, d)| {
                    let now = model.runs().find(|x| x.id == r)?.width;
                    ((now - d).abs() > 1e-4).then_some((r, now, d))
                })
                .collect();
            let short = w
                .networks
                .iter()
                .flat_map(|n| &n.taps)
                .filter(|t| t.pressure < archisim_mep::water::TAP_PRESSURE)
                .count();
            self.water.blocks = archisim_plan::services_report::water(model, &w);
            self.water.results = Some((rev, text, changes, short));
        }
        let (_, text, changes, short) = self.water.results.clone().expect("just made");
        let mut apply = false;
        let (mut pdf, mut dxf) = (false, false);
        ui.horizontal(|ui| {
            let p = super::theme::Palette::current(ui.ctx());
            if short > 0 {
                ui.colored_label(
                    p.bad,
                    format!(
                        "{short} grifo{} sin la presión mínima",
                        if short == 1 { "" } else { "s" }
                    ),
                );
            } else {
                ui.colored_label(p.good, "Todos los grifos con presión");
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui
                    .button("Esquemas (DXF)…")
                    .on_hover_text("El esquema vertical y la distribución de cada planta, un DXF por hoja")
                    .clicked()
                {
                    dxf = true;
                }
                if ui
                    .button("Esquemas (PDF)…")
                    .on_hover_text("El esquema vertical y la distribución de cada planta en hojas A3, con los diámetros calculados")
                    .clicked()
                {
                    pdf = true;
                }
                if ui
                    .add_enabled(
                        !changes.is_empty(),
                        egui::Button::new(format!(
                            "Poner los diámetros calculados ({})",
                            changes.len()
                        )),
                    )
                    .on_hover_text("Cada tubería toma el diámetro que pide su caudal (el mayor de sus tramos), en un paso de Deshacer: también las que estaban de más")
                    .clicked()
                {
                    apply = true;
                }
            });
        });
        let blocks = std::mem::take(&mut self.water.blocks);
        super::report_view::result(
            ui,
            &blocks,
            &text,
            &mut self.water.as_text,
            "el agua",
            "water",
            None,
        );
        self.water.blocks = blocks;
        if apply {
            self.apply_water_sizes();
        }
        if pdf {
            self.export_water_pdf();
        }
        if dxf {
            self.export_water_dxf();
        }
    }

    /// The plumbing diagrams as a PDF of A3 pages.
    pub(super) fn export_water_pdf(&mut self) {
        let mut d = rfd::FileDialog::new()
            .set_title("Esquemas de fontanería (PDF)")
            .add_filter("PDF", &["pdf"])
            .set_file_name(format!("{}-fontaneria.pdf", self.file_name_stem()));
        if let Some(dir) = self.start_dir() {
            d = d.set_directory(dir);
        }
        let Some(path) = d.save_file() else {
            return;
        };
        let model = self.ed.shown();
        let sheets: Vec<(String, String)> = archisim_plan::water_schematic::pages(&model)
            .into_iter()
            .enumerate()
            .map(|(i, svg)| (svg, format!("FO-{:02}", i + 1)))
            .collect();
        if sheets.is_empty() {
            self.complain("No hay tuberías de agua que lleguen a un aparato.");
            return;
        }
        let r = crate::documents::pdf_of(&sheets, "Esquemas de fontanería")
            .and_then(|bytes| std::fs::write(&path, bytes).map_err(|e| e.to_string()));
        match r {
            Ok(()) => self.say(format!(
                "Esquemas de fontanería: {} hojas A3 en {}.",
                sheets.len(),
                path.display()
            )),
            Err(e) => self.complain(format!("No se pudo exportar: {e}.")),
        }
    }

    /// The plumbing diagrams in DXF: one file per page.
    pub(super) fn export_water_dxf(&mut self) {
        let mut d = rfd::FileDialog::new()
            .set_title("Esquemas de fontanería (DXF)")
            .add_filter("DXF", &["dxf"])
            .set_file_name(format!("{}-fontaneria.dxf", self.file_name_stem()));
        if let Some(dir) = self.start_dir() {
            d = d.set_directory(dir);
        }
        let Some(path) = d.save_file() else {
            return;
        };
        let model = self.ed.shown();
        let sheets = archisim_plan::water_schematic::pages(&model);
        if sheets.is_empty() {
            self.complain("No hay tuberías de agua que lleguen a un aparato.");
            return;
        }
        let stem = path
            .file_stem()
            .map_or("fontaneria".into(), |s| s.to_string_lossy().to_string());
        for (i, svg) in sheets.iter().enumerate() {
            let file = path.with_file_name(format!("{stem}-fo-{:02}.dxf", i + 1));
            if let Err(e) = archisim_export::dxf::page_dxf(&model, svg)
                .and_then(|t| std::fs::write(&file, t).map_err(|e| e.to_string()))
            {
                self.complain(format!("No se pudo exportar: {e}."));
                return;
            }
        }
        self.say(format!(
            "Esquemas de fontanería en DXF: {} archivos junto a {}.",
            sheets.len(),
            path.display()
        ));
    }

    /// Gives every pipe of the water supply the diameter of the
    /// calculation, in one undoable step; how many change.
    pub(super) fn apply_water_sizes(&mut self) -> usize {
        let model = self.ed.model();
        let changes: Vec<Resize> = archisim_mep::water::water(model)
            .sized_runs()
            .into_iter()
            .filter_map(|(r, d)| {
                let now = model.runs().find(|x| x.id == r)?.width;
                ((now - d).abs() > 1e-4).then_some((r, now, d))
            })
            .collect();
        let n = changes.len();
        if n == 0 {
            return 0;
        }
        let r = self
            .ed
            .transact("Diámetros del cálculo de fontanería", |m| {
                for (run, _, d) in &changes {
                    m.edit_run(*run, |r| r.width = *d)?;
                }
                Ok(())
            });
        self.report(
            r,
            &format!(
                "Diámetros puestos en {n} tubería{}. Deshacer los quita.",
                if n == 1 { "" } else { "s" }
            ),
        );
        n
    }
}
