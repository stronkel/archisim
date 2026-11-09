//! The electrical calculation window (Colaboración → Cálculo eléctrico):
//! what the drawing does not give (the degree of electrification, the
//! feeder's length, a single user or centralised meters, the supply, the
//! general feeder, the short circuit at the supply and the breakers'
//! breaking capacity and curve), the calculation
//! of each board as text, and the single-line diagram in PDF or DXF.

use archisim_core::{Electrification, Supply, TripCurve};
use eframe::egui::{self, Ui};

use super::*;

/// The window's state.
#[derive(Default)]
pub struct ElectricalWindow {
    pub open: bool,
    /// The calculation of a revision of the model: (revision, text, number
    /// of boards, the blocks drawn).
    results: Option<(u64, String, usize, Vec<archisim_struct::report::Block>)>,
    /// Shown as typed text.
    as_text: bool,
}

impl ArchiSimApp {
    pub(super) fn electrical_dialog(&mut self, ctx: &egui::Context) {
        if !self.electrical.open {
            return;
        }
        let mut open = true;
        super::widgets::dialog("Cálculo eléctrico (REBT)")
            .open(&mut open)
            .default_width(760.0)
            .default_height(560.0)
            .show(ctx, |ui| self.electrical_contents(ui));
        self.electrical.open = open;
    }

    fn electrical_contents(&mut self, ui: &mut Ui) {
        let site = *self.ed.model().electrical_site();
        let mut new = site;
        egui::Grid::new("el-site")
            .num_columns(2)
            .spacing([12.0, 6.0])
            .show(ui, |ui| {
                ui.label("Electrificación");
                egui::ComboBox::from_id_salt("el-grade")
                    .selected_text(new.grade.name())
                    .width(220.0)
                    .show_ui(ui, |ui| {
                        for g in Electrification::ALL {
                            ui.selectable_value(&mut new.grade, g, g.name());
                        }
                    })
                    .response
                    .on_hover_text("ITC-BT-10: elevada con más de 160 m², calefacción o aire acondicionado eléctricos, o más puntos de los que admiten los circuitos básicos");
                ui.end_row();
                ui.label("Derivación individual");
                ui.horizontal(|ui| {
                    ui.add(
                        egui::DragValue::new(&mut new.feeder_length)
                            .range(0.0..=200.0)
                            .speed(0.5)
                            .suffix(" m"),
                    )
                    .on_hover_text("Del contador al cuadro");
                    super::widgets::switch(
                        ui,
                        &mut new.single_user,
                        "un solo usuario (contador propio): ΔU ≤ 1,5 %; si no, 1 %",
                    );
                });
                ui.end_row();
                ui.label("Suministro");
                egui::ComboBox::from_id_salt("el-supply")
                    .selected_text(new.supply.name())
                    .width(220.0)
                    .show_ui(ui, |ui| {
                        for g in Supply::ALL {
                            ui.selectable_value(&mut new.supply, g, g.name());
                        }
                    })
                    .response
                    .on_hover_text("Según la potencia: trifásico por encima de 14 490 W (ITC-BT-10); los circuitos se reparten por fases");
                ui.end_row();
                if !new.single_user {
                    ui.label("Línea general");
                    ui.horizontal(|ui| {
                        ui.add(
                            egui::DragValue::new(&mut new.general_length)
                                .range(0.0..=300.0)
                                .speed(0.5)
                                .suffix(" m"),
                        )
                        .on_hover_text("De la caja general de protección a la centralización de contadores (ITC-BT-14). Las derivaciones se miden en el dibujo desde un cuadro llamado «Contadores» o, si no lo hay, desde el de servicios generales");
                        super::widgets::switch(
                            ui,
                            &mut new.partly_centralised,
                            "contadores en más de un lugar: línea general ≤ 1 % y derivaciones ≤ 0,5 %",
                        );
                    });
                    ui.end_row();
                }
                ui.label("Cortocircuito");
                ui.horizontal(|ui| {
                    ui.add(
                        egui::DragValue::new(&mut new.supply_fault)
                            .range(0.0..=50.0)
                            .speed(0.1)
                            .max_decimals(1)
                            .suffix(" kA"),
                    )
                    .on_hover_text("La intensidad de cortocircuito en la acometida que da la compañía; 0: se calcula desde el transformador");
                    if new.supply_fault <= 0.0 {
                        ui.label("desde el transformador, a");
                        ui.add(
                            egui::DragValue::new(&mut new.network_length)
                                .range(0.0..=2000.0)
                                .speed(1.0)
                                .suffix(" m"),
                        )
                        .on_hover_text("Longitud de la línea de la compañía (150 mm² de aluminio) del transformador a la acometida");
                    } else {
                        ui.label("en la acometida (dato de la compañía)");
                    }
                });
                ui.end_row();
                ui.label("Magnetotérmicos");
                ui.horizontal(|ui| {
                    egui::ComboBox::from_id_salt("el-breaking")
                        .selected_text(format!(
                            "poder de corte {}",
                            archisim_mep::electrical::ka(new.breaking_capacity)
                        ))
                        .width(170.0)
                        .show_ui(ui, |ui| {
                            for b in archisim_mep::electrical::BREAKING_CAPACITIES {
                                ui.selectable_value(
                                    &mut new.breaking_capacity,
                                    b,
                                    archisim_mep::electrical::ka(b),
                                );
                            }
                        });
                    egui::ComboBox::from_id_salt("el-curve")
                        .selected_text(format!("curva {}", new.curve.name()))
                        .width(90.0)
                        .show_ui(ui, |ui| {
                            for c in TripCurve::ALL {
                                ui.selectable_value(&mut new.curve, c, format!("curva {}", c.name()));
                            }
                        })
                        .response
                        .on_hover_text("Disparo magnético: B a 5·In, C a 10·In, D a 20·In; se comprueba con el cortocircuito al final de cada circuito");
                });
                ui.end_row();
            });
        if new != site {
            let r = self.ed.transact("Datos de la instalación eléctrica", |m| {
                m.set_electrical_site(new)
            });
            if let Err(e) = r {
                self.complain(format!("No se pueden cambiar: {e}."));
            }
        }
        ui.label(
            egui::RichText::new(
                "El cálculo sale del cuadro, los mecanismos con su circuito y las lavadoras y lavavajillas dibujados. Los circuitos C1 a C5 están siempre; si un circuito no tiene puntos dibujados, se calcula con uno. Es una ayuda al proyecto: el instalador firma su certificado.",
            )
            .small(),
        );
        ui.separator();
        let rev = self.ed.revision();
        if self.electrical.results.as_ref().is_none_or(|r| r.0 != rev) {
            let model = self.ed.shown();
            let text = archisim_mep::electrical::report(&model);
            let all = archisim_mep::electrical::analysis(&model);
            let blocks = archisim_plan::services_report::electricity(&model, &all);
            self.electrical.results = Some((rev, text, all.installations.len(), blocks));
        }
        let (_, text, boards, blocks) = self.electrical.results.clone().expect("just made");
        let (mut pdf, mut dxf) = (false, false);
        ui.horizontal(|ui| {
            ui.label(match boards {
                0 => "Sin cuadros ni mecanismos".to_string(),
                1 => "1 cuadro".to_string(),
                n => format!("{n} cuadros"),
            });
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui
                    .add_enabled(boards > 0, egui::Button::new("Esquema unifilar (DXF)…"))
                    .clicked()
                {
                    dxf = true;
                }
                if ui
                    .add_enabled(boards > 0, egui::Button::new("Esquema unifilar (PDF)…"))
                    .clicked()
                {
                    pdf = true;
                }
            });
        });
        super::report_view::result(
            ui,
            &blocks,
            &text,
            &mut self.electrical.as_text,
            "la instalación eléctrica",
            "electrical",
            None,
        );
        if pdf {
            self.export_single_line_pdf();
        }
        if dxf {
            self.export_single_line_dxf();
        }
    }

    /// The single-line diagrams as a PDF of A3 pages.
    pub(super) fn export_single_line_pdf(&mut self) {
        let mut d = rfd::FileDialog::new()
            .set_title("Esquema unifilar (PDF)")
            .add_filter("PDF", &["pdf"])
            .set_file_name(format!("{}-unifilar.pdf", self.file_name_stem()));
        if let Some(dir) = self.start_dir() {
            d = d.set_directory(dir);
        }
        let Some(path) = d.save_file() else {
            return;
        };
        let model = self.ed.shown();
        let sheets: Vec<(String, String)> = archisim_plan::single_line::pages(&model)
            .into_iter()
            .enumerate()
            .map(|(i, svg)| (svg, format!("IE-{:02}", i + 1)))
            .collect();
        if sheets.is_empty() {
            self.complain("No hay cuadro ni mecanismos eléctricos dibujados.");
            return;
        }
        let r = crate::documents::pdf_of(&sheets, "Esquema unifilar")
            .and_then(|bytes| std::fs::write(&path, bytes).map_err(|e| e.to_string()));
        match r {
            Ok(()) => self.say(format!(
                "Esquema unifilar: {} hojas A3 en {}.",
                sheets.len(),
                path.display()
            )),
            Err(e) => self.complain(format!("No se pudo exportar: {e}.")),
        }
    }

    /// The single-line diagrams in DXF: one file per board.
    pub(super) fn export_single_line_dxf(&mut self) {
        let mut d = rfd::FileDialog::new()
            .set_title("Esquema unifilar (DXF)")
            .add_filter("DXF", &["dxf"])
            .set_file_name(format!("{}-unifilar.dxf", self.file_name_stem()));
        if let Some(dir) = self.start_dir() {
            d = d.set_directory(dir);
        }
        let Some(path) = d.save_file() else {
            return;
        };
        let model = self.ed.shown();
        let sheets = archisim_plan::single_line::pages(&model);
        let many = sheets.len() > 1;
        let mut written = 0;
        for (i, svg) in sheets.iter().enumerate() {
            let file = if many {
                path.with_file_name(format!(
                    "{}-ie-{:02}.dxf",
                    path.file_stem()
                        .map_or("unifilar".into(), |s| s.to_string_lossy()),
                    i + 1
                ))
            } else {
                path.clone()
            };
            match archisim_export::dxf::page_dxf(&model, svg)
                .and_then(|t| std::fs::write(&file, t).map_err(|e| e.to_string()))
            {
                Ok(()) => written += 1,
                Err(e) => {
                    self.complain(format!("No se pudo exportar: {e}."));
                    return;
                }
            }
        }
        self.say(format!(
            "Esquema unifilar en DXF: {written} archivo(s) en {}.",
            path.display()
        ));
    }
}
