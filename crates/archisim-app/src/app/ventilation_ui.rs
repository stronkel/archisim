//! The ventilation window (Colaboración → Cálculo de ventilación): the
//! dwellings' ventilation by the DB-HS 3 as text, and «Poner los tamaños
//! calculados», which takes every inlet, extract valve, duct and shaft
//! drawn smaller up to what it needs, in one undoable step.

use eframe::egui::{self, Ui};

use super::*;

/// The window's state.
#[derive(Default)]
pub struct VentilationWindow {
    pub open: bool,
    /// The calculation of a revision of the model: (revision, text, the
    /// changes, what fails).
    results: Option<(u64, String, Grow, usize)>,
}

/// What grows: ducts (their width) and devices (width and depth).
#[derive(Clone, Default)]
struct Grow {
    runs: Vec<(archisim_core::RunId, f64)>,
    devices: Vec<(archisim_core::DeviceId, f64, f64)>,
}

impl Grow {
    fn len(&self) -> usize {
        self.runs.len() + self.devices.len()
    }

    fn of(model: &archisim_core::Model, v: &archisim_mep::ventilation::Ventilation) -> Self {
        Grow {
            runs: v
                .sized_runs(model)
                .into_iter()
                .filter(|(r, w)| model.run(*r).is_some_and(|x| x.width + 1e-4 < *w))
                .collect(),
            devices: v.sized_devices(model),
        }
    }
}

impl ArchiSimApp {
    pub(super) fn ventilation_dialog(&mut self, ctx: &egui::Context) {
        if !self.ventilation.open {
            return;
        }
        let mut open = true;
        super::widgets::dialog("Cálculo de ventilación (DB-HS 3)")
            .open(&mut open)
            .default_width(860.0)
            .default_height(560.0)
            .show(ctx, |ui| self.ventilation_contents(ui));
        self.ventilation.open = open;
    }

    fn ventilation_contents(&mut self, ui: &mut Ui) {
        ui.label(
            egui::RichText::new(
                "Viviendas: los locales de cada planta unidos por puertas, sin los comunes. Entra el aire por los aireadores de los locales secos, pasa bajo las puertas y sale por las bocas de extracción de los húmedos, por los conductos del sistema Extracción hasta el extractor.",
            )
            .small(),
        );
        ui.separator();
        let rev = self.ed.revision();
        if self.ventilation.results.as_ref().is_none_or(|r| r.0 != rev) {
            let model = self.ed.model();
            let v = archisim_mep::ventilation::ventilation(model);
            let text = archisim_mep::ventilation::report(model, &v);
            let fails = v.fails();
            self.ventilation.results = Some((rev, text, Grow::of(model, &v), fails));
        }
        let (_, text, grow, fails) = self.ventilation.results.clone().expect("just made");
        let (mut apply, mut pdf, mut dxf) = (false, false, false);
        ui.horizontal(|ui| {
            let p = super::theme::Palette::current(ui.ctx());
            if fails > 0 {
                ui.colored_label(
                    p.bad,
                    format!(
                        "{fails} abertura{} o tramo{} por debajo de lo exigido",
                        if fails == 1 { "" } else { "s" },
                        if fails == 1 { "" } else { "s" }
                    ),
                );
            } else {
                ui.colored_label(p.good, "Todo con su tamaño");
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui
                    .button("Esquemas (DXF)…")
                    .on_hover_text("El esquema vertical y la red de cada planta, un DXF por hoja")
                    .clicked()
                {
                    dxf = true;
                }
                if ui
                    .button("Esquemas (PDF)…")
                    .on_hover_text("El esquema vertical de la extracción y las campanas, y la red de cada planta, en hojas A3")
                    .clicked()
                {
                    pdf = true;
                }
                if ui
                    .add_enabled(
                        grow.len() > 0,
                        egui::Button::new(format!("Poner los tamaños calculados ({})", grow.len())),
                    )
                    .on_hover_text("Los aireadores, bocas, conductos y conductos verticales dibujados más pequeños toman el tamaño calculado, en un paso de Deshacer (los pasos de las puertas se quedan: holgura o rejilla, a decidir)")
                    .clicked()
                {
                    apply = true;
                }
            });
        });
        egui::ScrollArea::both()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                ui.add(
                    egui::Label::new(egui::RichText::new(text).monospace().size(11.0))
                        .wrap_mode(egui::TextWrapMode::Extend),
                );
            });
        if apply {
            self.apply_ventilation_sizes();
        }
        if pdf {
            self.export_ventilation_pdf();
        }
        if dxf {
            self.export_ventilation_dxf();
        }
    }

    /// The ventilation diagrams as a PDF of A3 pages.
    pub(super) fn export_ventilation_pdf(&mut self) {
        let mut d = rfd::FileDialog::new()
            .set_title("Esquemas de ventilación (PDF)")
            .add_filter("PDF", &["pdf"])
            .set_file_name(format!("{}-ventilacion.pdf", self.file_name_stem()));
        if let Some(dir) = self.start_dir() {
            d = d.set_directory(dir);
        }
        let Some(path) = d.save_file() else {
            return;
        };
        let model = self.ed.shown();
        let sheets: Vec<(String, String)> =
            archisim_plan::water_schematic::ventilation_pages(&model)
                .into_iter()
                .enumerate()
                .map(|(i, svg)| (svg, format!("VE-{:02}", i + 1)))
                .collect();
        if sheets.is_empty() {
            self.complain("No hay conductos de extracción que lleguen a una boca o a una campana.");
            return;
        }
        let r = crate::documents::pdf_of(&sheets, "Esquemas de ventilación")
            .and_then(|bytes| std::fs::write(&path, bytes).map_err(|e| e.to_string()));
        match r {
            Ok(()) => self.say(format!(
                "Esquemas de ventilación: {} hojas A3 en {}.",
                sheets.len(),
                path.display()
            )),
            Err(e) => self.complain(format!("No se pudo exportar: {e}.")),
        }
    }

    /// The ventilation diagrams in DXF: one file per page.
    pub(super) fn export_ventilation_dxf(&mut self) {
        let mut d = rfd::FileDialog::new()
            .set_title("Esquemas de ventilación (DXF)")
            .add_filter("DXF", &["dxf"])
            .set_file_name(format!("{}-ventilacion.dxf", self.file_name_stem()));
        if let Some(dir) = self.start_dir() {
            d = d.set_directory(dir);
        }
        let Some(path) = d.save_file() else {
            return;
        };
        let model = self.ed.shown();
        let sheets = archisim_plan::water_schematic::ventilation_pages(&model);
        if sheets.is_empty() {
            self.complain("No hay conductos de extracción que lleguen a una boca o a una campana.");
            return;
        }
        let stem = path
            .file_stem()
            .map_or("ventilacion".into(), |s| s.to_string_lossy().to_string());
        for (i, svg) in sheets.iter().enumerate() {
            let file = path.with_file_name(format!("{stem}-ve-{:02}.dxf", i + 1));
            if let Err(e) = archisim_export::dxf::page_dxf(&model, svg)
                .and_then(|t| std::fs::write(&file, t).map_err(|e| e.to_string()))
            {
                self.complain(format!("No se pudo exportar: {e}."));
                return;
            }
        }
        self.say(format!(
            "Esquemas de ventilación en DXF: {} archivos junto a {}.",
            sheets.len(),
            path.display()
        ));
    }

    /// Gives every inlet, valve, duct and shaft drawn smaller than it
    /// needs the size of the calculation, in one undoable step; how many
    /// change.
    pub(super) fn apply_ventilation_sizes(&mut self) -> usize {
        let model = self.ed.model();
        let v = archisim_mep::ventilation::ventilation(model);
        let grow = Grow::of(model, &v);
        let n = grow.len();
        if n == 0 {
            return 0;
        }
        let r = self.ed.transact("Tamaños del cálculo de ventilación", |m| {
            for (run, w) in &grow.runs {
                m.edit_run(*run, |r| r.width = *w)?;
            }
            for (d, w, dp) in &grow.devices {
                m.edit_device(*d, |x| {
                    x.width = *w;
                    x.depth = *dp;
                })?;
            }
            Ok(())
        });
        self.report(
            r,
            &format!(
                "Tamaños puestos en {n} elemento{} de la ventilación. Deshacer los quita.",
                if n == 1 { "" } else { "s" }
            ),
        );
        n
    }
}
