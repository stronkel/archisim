//! The drainage window (Colaboración → Cálculo de saneamiento): what the
//! drawing does not give (the rainfall intensity of the place), the
//! calculation of the waste and rain networks as text, «Poner los
//! diámetros calculados», which takes every pipe and downpipe drawn
//! smaller up to what it carries in one undoable step, and the drainage
//! diagrams (`archisim_plan::water_schematic::drainage_pages`) in PDF or
//! DXF.

use eframe::egui::{self, Ui};

use super::*;

/// The window's state.
#[derive(Default)]
pub struct DrainWindow {
    pub open: bool,
    /// The calculation of a revision of the model: (revision, text, the
    /// changes, stretches drawn smaller).
    results: Option<(u64, String, Grow, usize)>,
    /// The calculation drawn (`report_view`), and whether shown as typed
    /// text.
    blocks: Vec<archisim_struct::report::Block>,
    as_text: bool,
}

/// What grows: pipes and downpipes, from their diameter to the calculated
/// one.
#[derive(Clone, Default)]
struct Grow {
    runs: Vec<(archisim_core::RunId, f64)>,
    downpipes: Vec<(archisim_core::DeviceId, f64)>,
}

impl Grow {
    fn len(&self) -> usize {
        self.runs.len() + self.downpipes.len()
    }

    fn of(model: &archisim_core::Model) -> Self {
        let d = archisim_mep::drain::drainage(model);
        let (runs, pipes) = d.sized_runs();
        Grow {
            runs: runs
                .into_iter()
                .filter(|(r, w)| {
                    model
                        .runs()
                        .find(|x| x.id == *r)
                        .is_some_and(|x| x.width + 1e-4 < *w)
                })
                .collect(),
            downpipes: pipes
                .into_iter()
                .filter(|(p, w)| {
                    model
                        .devices()
                        .find(|x| x.id == *p)
                        .is_some_and(|x| x.width + 1e-4 < *w)
                })
                .collect(),
        }
    }
}

impl ArchiSimApp {
    pub(super) fn drain_dialog(&mut self, ctx: &egui::Context) {
        if !self.drain.open {
            return;
        }
        let mut open = true;
        super::widgets::dialog("Cálculo de saneamiento (DB-HS 5)")
            .open(&mut open)
            .default_width(860.0)
            .default_height(560.0)
            .show(ctx, |ui| self.drain_contents(ui));
        self.drain.open = open;
    }

    fn drain_contents(&mut self, ui: &mut Ui) {
        let site = *self.ed.model().water_site();
        let mut new = site;
        egui::Grid::new("drain-site")
            .num_columns(2)
            .spacing([12.0, 6.0])
            .show(ui, |ui| {
                ui.label("Intensidad de lluvia");
                ui.add(
                    egui::DragValue::new(&mut new.rain_intensity)
                        .range(30.0..=250.0)
                        .speed(1.0)
                        .suffix(" mm/h"),
                )
                .on_hover_text("La del lugar: DB-HS 5, anejo B, tabla B.1, por su zona (A o B, figura B.1) y su isoyeta. Las tablas de pluviales son para 100 mm/h");
                ui.end_row();
                ui.label("Cierre hidráulico");
                let mut seal = new.trap_seal * 1000.0;
                if ui
                    .add(
                        egui::DragValue::new(&mut seal)
                            .range(20.0..=150.0)
                            .speed(1.0)
                            .suffix(" mm"),
                    )
                    .on_hover_text("La altura del cierre de los sifones y botes sifónicos (HS 5, 3.3.1.1: de 50 a 100 mm; 70 en usos discontinuos)")
                    .changed()
                {
                    new.trap_seal = seal / 1000.0;
                }
                ui.end_row();
                ui.label("Cubiertas");
                egui::ComboBox::from_id_salt("drain-roof-access")
                    .selected_text(new.roof_access.name())
                    .show_ui(ui, |ui| {
                        for a in archisim_core::RoofAccess::ALL {
                            ui.selectable_value(&mut new.roof_access, a, a.name());
                        }
                    })
                    .response
                    .on_hover_text("Para la ventilación primaria (HS 5, 3.3.3.1): la bajante sale 2,00 m sobre una cubierta transitable y 1,30 m sobre una que no lo es. «Según el dibujo»: transitable si en su planta hay locales o barandillas");
                ui.end_row();
            });
        if new != site {
            let r = self
                .ed
                .transact("Datos del saneamiento", |m| m.set_water_site(new));
            if let Err(e) = r {
                self.complain(format!("No se puede cambiar: {e}."));
            }
        }
        ui.label(
            egui::RichText::new(
                "Residuales: de cada aparato sanitario por sus desagües y bajantes hasta el punto más bajo de la red (la acometida), por unidades de desagüe. Pluviales: las cubiertas y terrazas al aire reparten su lluvia entre los sumideros (extremos altos de las tuberías de pluviales) a su altura. Ventilación: cada bajante hasta su cabeza sobre la cubierta, o con una válvula de aireación; la columna de ventilación secundaria es una bajante sin aparatos junto a ella.",
            )
            .small(),
        );
        ui.separator();
        let rev = self.ed.revision();
        if self.drain.results.as_ref().is_none_or(|r| r.0 != rev) {
            let model = self.ed.model();
            let d = archisim_mep::drain::drainage(model);
            let text = archisim_mep::drain::report(model, &d);
            let short = d.networks.iter().flat_map(|n| n.short()).count();
            self.drain.blocks = archisim_plan::services_report::drainage(model, &d);
            self.drain.results = Some((rev, text, Grow::of(model), short));
        }
        let (_, text, grow, short) = self.drain.results.clone().expect("just made");
        let mut apply = false;
        let (mut pdf, mut dxf) = (false, false);
        ui.horizontal(|ui| {
            let p = super::theme::Palette::current(ui.ctx());
            if short > 0 {
                ui.colored_label(
                    p.bad,
                    format!(
                        "{short} tramo{} más pequeño{} de lo exigido",
                        if short == 1 { "" } else { "s" },
                        if short == 1 { "" } else { "s" }
                    ),
                );
            } else {
                ui.colored_label(p.good, "Todos los tramos con su diámetro");
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
                    .on_hover_text("El esquema vertical y la red de cada planta en hojas A3, con los diámetros calculados")
                    .clicked()
                {
                    pdf = true;
                }
                if ui
                    .add_enabled(
                        grow.len() > 0,
                        egui::Button::new(format!("Poner los diámetros calculados ({})", grow.len())),
                    )
                    .on_hover_text("Cada tubería y bajante dibujada más pequeña de lo que lleva toma el diámetro calculado, en un paso de Deshacer (las que están de más se quedan)")
                    .clicked()
                {
                    apply = true;
                }
            });
        });
        let blocks = std::mem::take(&mut self.drain.blocks);
        super::report_view::result(
            ui,
            &blocks,
            &text,
            &mut self.drain.as_text,
            "el saneamiento",
            "drain",
            None,
        );
        self.drain.blocks = blocks;
        if apply {
            self.apply_drain_sizes();
        }
        if pdf {
            self.export_drain_pdf();
        }
        if dxf {
            self.export_drain_dxf();
        }
    }

    /// The drainage diagrams as a PDF of A3 pages.
    pub(super) fn export_drain_pdf(&mut self) {
        let mut d = crate::file_dialog::FileDialog::new()
            .set_title("Esquemas de saneamiento (PDF)")
            .add_filter("PDF", &["pdf"])
            .set_file_name(format!("{}-saneamiento.pdf", self.file_name_stem()));
        if let Some(dir) = self.start_dir() {
            d = d.set_directory(dir);
        }
        let Some(path) = d.save_file() else {
            return;
        };
        let model = self.ed.shown();
        let sheets: Vec<(String, String)> = archisim_plan::water_schematic::drainage_pages(&model)
            .into_iter()
            .enumerate()
            .map(|(i, svg)| (svg, format!("SA-{:02}", i + 1)))
            .collect();
        if sheets.is_empty() {
            self.complain("No hay desagües que lleguen a un aparato ni pluviales con cubierta.");
            return;
        }
        let r = crate::documents::pdf_of(&sheets, "Esquemas de saneamiento")
            .and_then(|bytes| std::fs::write(&path, bytes).map_err(|e| e.to_string()));
        match r {
            Ok(()) => self.say(format!(
                "Esquemas de saneamiento: {} hojas A3 en {}.",
                sheets.len(),
                path.display()
            )),
            Err(e) => self.complain(format!("No se pudo exportar: {e}.")),
        }
    }

    /// The drainage diagrams in DXF: one file per page.
    pub(super) fn export_drain_dxf(&mut self) {
        let mut d = crate::file_dialog::FileDialog::new()
            .set_title("Esquemas de saneamiento (DXF)")
            .add_filter("DXF", &["dxf"])
            .set_file_name(format!("{}-saneamiento.dxf", self.file_name_stem()));
        if let Some(dir) = self.start_dir() {
            d = d.set_directory(dir);
        }
        let Some(path) = d.save_file() else {
            return;
        };
        let model = self.ed.shown();
        let sheets = archisim_plan::water_schematic::drainage_pages(&model);
        if sheets.is_empty() {
            self.complain("No hay desagües que lleguen a un aparato ni pluviales con cubierta.");
            return;
        }
        let stem = path
            .file_stem()
            .map_or("saneamiento".into(), |s| s.to_string_lossy().to_string());
        for (i, svg) in sheets.iter().enumerate() {
            let file = path.with_file_name(format!("{stem}-sa-{:02}.dxf", i + 1));
            if let Err(e) = archisim_export::dxf::page_dxf(&model, svg)
                .and_then(|t| std::fs::write(&file, t).map_err(|e| e.to_string()))
            {
                self.complain(format!("No se pudo exportar: {e}."));
                return;
            }
        }
        self.say(format!(
            "Esquemas de saneamiento en DXF: {} archivos junto a {}.",
            sheets.len(),
            path.display()
        ));
    }

    /// Gives every drain and downpipe drawn smaller than it carries the
    /// diameter of the calculation, in one undoable step; how many change.
    pub(super) fn apply_drain_sizes(&mut self) -> usize {
        let grow = Grow::of(self.ed.model());
        let n = grow.len();
        if n == 0 {
            return 0;
        }
        let r = self
            .ed
            .transact("Diámetros del cálculo de saneamiento", |m| {
                for (run, d) in &grow.runs {
                    m.edit_run(*run, |r| r.width = *d)?;
                }
                for (pipe, d) in &grow.downpipes {
                    m.edit_device(*pipe, |x| {
                        x.width = *d;
                        x.depth = *d;
                    })?;
                }
                Ok(())
            });
        self.report(
            r,
            &format!(
                "Diámetros puestos en {n} tubería{} y bajante{}. Deshacer los quita.",
                if n == 1 { "" } else { "s" },
                if n == 1 { "" } else { "s" }
            ),
        );
        n
    }
}
