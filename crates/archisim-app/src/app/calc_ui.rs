//! The structural calculation window: the site data of the actions
//! (snow, wind, earthquake, ground), what the calculation finds (the
//! takedown, the frame of columns and beams and the checks of the light
//! frames, as text) and the calculation report in PDF.

use archisim_core::{Importance, WindZone};
use eframe::egui::{self, Ui};

use super::*;

/// The window's state.
#[derive(Default)]
pub struct CalcWindow {
    pub open: bool,
    /// The results of a revision of the model: (revision, text, number of
    /// checks that fail).
    results: Option<(u64, String, usize)>,
    /// The reinforcement the calculation asks for, while it is being
    /// chosen: each with whether it goes in.
    proposals: Option<(u64, Vec<(archisim_struct::proposals::Proposal, bool)>)>,
}

impl ArchiSimApp {
    pub(super) fn calc_dialog(&mut self, ctx: &egui::Context) {
        if !self.calc.open {
            return;
        }
        let mut open = true;
        super::widgets::dialog("Cálculo de la estructura")
            .open(&mut open)
            .default_width(760.0)
            .default_height(620.0)
            .show(ctx, |ui| self.calc_contents(ui));
        self.calc.open = open;
    }

    fn calc_contents(&mut self, ui: &mut Ui) {
        let site = *self.ed.model().structural_site();
        let mut new = site;
        let geo_height = self.ed.model().georeference().map(|g| g.height);
        egui::Grid::new("calc-site")
            .num_columns(2)
            .spacing([12.0, 6.0])
            .show(ui, |ui| {
                ui.label("Altitud");
                ui.horizontal(|ui| {
                    let mut own = new.altitude.is_some();
                    let label = match geo_height {
                        Some(h) => format!("la de la georreferencia ({h:.0} m)"),
                        None => "0 m (sin georreferencia)".into(),
                    };
                    if ui.radio(!own, label).clicked() {
                        own = false;
                    }
                    if ui.radio(own, "otra:").clicked() {
                        own = true;
                    }
                    if own {
                        let mut a = new.altitude.unwrap_or(geo_height.unwrap_or(0.0));
                        ui.add(
                            egui::DragValue::new(&mut a)
                                .range(0.0..=3000.0)
                                .suffix(" m")
                                .speed(5.0),
                        );
                        new.altitude = Some(a);
                    } else {
                        new.altitude = None;
                    }
                });
                ui.end_row();
                ui.label("Nieve");
                egui::ComboBox::from_id_salt("calc-snow")
                    .selected_text(format!("Zona de clima invernal {}", new.snow_zone))
                    .show_ui(ui, |ui| {
                        for z in 1..=7u8 {
                            ui.selectable_value(&mut new.snow_zone, z, format!("Zona {z}"));
                        }
                    })
                    .response
                    .on_hover_text("DB-SE-AE, figura E.2 (Huelva: zona 6)");
                ui.end_row();
                ui.label("Viento");
                ui.horizontal(|ui| {
                    egui::ComboBox::from_id_salt("calc-wind")
                        .selected_text(format!("Zona {}", new.wind_zone.code()))
                        .show_ui(ui, |ui| {
                            for z in WindZone::ALL {
                                ui.selectable_value(
                                    &mut new.wind_zone,
                                    z,
                                    format!(
                                        "Zona {} · qb {:.2} kN/m²",
                                        z.code(),
                                        z.basic_pressure()
                                    ),
                                );
                            }
                        });
                    egui::ComboBox::from_id_salt("calc-rough")
                        .selected_text(
                            archisim_struct::actions::ROUGHNESS_NAMES
                                [(new.roughness.clamp(1, 5) - 1) as usize],
                        )
                        .width(300.0)
                        .show_ui(ui, |ui| {
                            for (i, n) in
                                archisim_struct::actions::ROUGHNESS_NAMES.iter().enumerate()
                            {
                                ui.selectable_value(&mut new.roughness, i as u8 + 1, *n);
                            }
                        });
                });
                ui.end_row();
                ui.label("Sismo (NCSE-02)");
                ui.horizontal(|ui| {
                    ui.label("ab/g");
                    ui.add(
                        egui::DragValue::new(&mut new.seismic_ab)
                            .range(0.0..=0.4)
                            .speed(0.01)
                            .max_decimals(2),
                    );
                    ui.label("K");
                    ui.add(
                        egui::DragValue::new(&mut new.seismic_k)
                            .range(1.0..=1.5)
                            .speed(0.01)
                            .max_decimals(2),
                    );
                    ui.label("C");
                    ui.add(
                        egui::DragValue::new(&mut new.soil)
                            .range(1.0..=2.0)
                            .speed(0.01)
                            .max_decimals(2),
                    );
                    egui::ComboBox::from_id_salt("calc-imp")
                        .selected_text(format!(
                            "Importancia {}",
                            new.importance.name().to_lowercase()
                        ))
                        .show_ui(ui, |ui| {
                            for i in Importance::ALL {
                                ui.selectable_value(&mut new.importance, i, i.name());
                            }
                        });
                });
                ui.end_row();
                ui.label("Terreno");
                ui.horizontal(|ui| {
                    ui.add(
                        egui::DragValue::new(&mut new.bearing)
                            .range(25.0..=1000.0)
                            .speed(5.0)
                            .suffix(" kN/m² admisibles"),
                    );
                    ui.label("φ");
                    ui.add(
                        egui::DragValue::new(&mut new.soil_friction)
                            .range(15.0..=45.0)
                            .speed(0.5)
                            .suffix("°"),
                    )
                    .on_hover_text(
                        "Ángulo de rozamiento interno: empuje sobre los muros de sótano",
                    );
                    ui.label("γ");
                    ui.add(
                        egui::DragValue::new(&mut new.soil_weight)
                            .range(12.0..=24.0)
                            .speed(0.5)
                            .suffix(" kN/m³"),
                    );
                });
                ui.end_row();
                ui.label("Materiales");
                ui.horizontal_wrapped(|ui| {
                    let m = &mut new.materials;
                    egui::ComboBox::from_id_salt("calc-fck")
                        .selected_text(format!("HA-{:.0}", m.concrete_fck))
                        .show_ui(ui, |ui| {
                            for f in archisim_core::StructuralMaterials::CONCRETES {
                                ui.selectable_value(&mut m.concrete_fck, f, format!("HA-{f:.0}"));
                            }
                        });
                    egui::ComboBox::from_id_salt("calc-fyk")
                        .selected_text(format!("B{:.0}S", m.rebar_fyk))
                        .show_ui(ui, |ui| {
                            for f in archisim_core::StructuralMaterials::REBARS {
                                ui.selectable_value(&mut m.rebar_fyk, f, format!("B{f:.0}S"));
                            }
                        });
                    egui::ComboBox::from_id_salt("calc-fy")
                        .selected_text(format!("S{:.0}", m.steel_fy))
                        .show_ui(ui, |ui| {
                            for f in archisim_core::StructuralMaterials::STEELS {
                                ui.selectable_value(&mut m.steel_fy, f, format!("S{f:.0}"));
                            }
                        });
                    egui::ComboBox::from_id_salt("calc-timber")
                        .selected_text(m.timber.code())
                        .show_ui(ui, |ui| {
                            for t in archisim_core::TimberClass::ALL {
                                ui.selectable_value(&mut m.timber, t, t.code());
                            }
                        });
                });
                ui.end_row();
                ui.label("Fábrica");
                {
                    let m = &mut new.materials;
                    ui.horizontal(|ui| {
                        ui.label("fk");
                        ui.add(
                            egui::DragValue::new(&mut m.masonry_fk)
                                .range(0.5..=20.0)
                                .speed(0.1)
                                .max_decimals(1)
                                .suffix(" N/mm²"),
                        )
                        .on_hover_text("DB-SE-F, tabla 4.4: según la pieza y el mortero");
                    });
                }
                ui.end_row();
                ui.label("Edificio");
                {
                    use super::widgets::SwitchExt;
                    ui.switch(
                        &mut new.single_house,
                        "vivienda unifamiliar (resistencia al fuego R 30)",
                    );
                }
                ui.end_row();
                ui.label("Uniones de acero");
                {
                    use super::widgets::SwitchExt;
                    ui.switch(
                        &mut new.materials.steel_rigid,
                        "rígidas (chapa de testa atornillada); si no, articuladas",
                    );
                }
                ui.end_row();
            });
        if new != site {
            let r = self.ed.set_structural_site(new);
            self.report(r, "Emplazamiento de la estructura cambiado.");
        }
        ui.label(
            egui::RichText::new(
                "Las sobrecargas salen del uso de cada local (Colaboración → Comprobación CTE para el uso del edificio); los pesos, de las capas de cada tipo. Es una ayuda al cálculo: sus hipótesis las revisa y firma el técnico.",
            )
            .weak()
            .size(11.0),
        );
        ui.separator();
        let rev = self.ed.revision();
        if self.calc.results.as_ref().is_none_or(|r| r.0 != rev) {
            let model = self.ed.shown();
            let text = archisim_struct::structure_report(&model);
            let fails = text.matches("NO CUMPLE").count();
            self.calc.results = Some((rev, text, fails));
        }
        let (_, text, fails) = self.calc.results.clone().expect("just made");
        let mut export = false;
        ui.horizontal(|ui| {
            let p = super::theme::Palette::current(ui.ctx());
            if fails == 0 {
                ui.colored_label(p.good, "Todo cumple");
            } else {
                ui.colored_label(p.bad, format!("{fails} comprobaciones no cumplen"));
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button("Anejo de cálculo (PDF)…").clicked() {
                    export = true;
                }
                if ui
                    .button("Aplicar el armado propuesto…")
                    .on_hover_text("El armado que pide el cálculo de vigas, pilares, zapatas y losas, con las reglas sísmicas si las hay: eliges a qué elementos se lo pones")
                    .clicked()
                {
                    let shown = self.ed.shown();
                    let model = archisim_struct::phases::as_built(&shown);
                    let an = archisim_struct::analysis::analyse(&model);
                    let list = archisim_struct::proposals::proposals(&model, &an)
                        .into_iter()
                        .map(|p| {
                            let on = p.is_new();
                            (p, on)
                        })
                        .collect();
                    self.calc.proposals = Some((rev, list));
                }
            });
        });
        if self.calc.proposals.as_ref().is_some_and(|p| p.0 != rev) {
            self.calc.proposals = None;
        }
        if self.calc.proposals.is_some() {
            self.proposals_panel(ui);
            return;
        }
        egui::ScrollArea::both()
            .auto_shrink([false, false])
            .max_height(ui.available_height() - 8.0)
            .show(ui, |ui| {
                ui.add(
                    egui::Label::new(egui::RichText::new(text).monospace().size(11.0))
                        .wrap_mode(egui::TextWrapMode::Extend),
                );
            });
        if export {
            self.export_calc_report();
        }
    }

    /// The proposals to choose from: a row each, with what it has now.
    fn proposals_panel(&mut self, ui: &mut Ui) {
        let Some((_, list)) = self.calc.proposals.as_mut() else {
            return;
        };
        let p = super::theme::Palette::current(ui.ctx());
        ui.label(
            egui::RichText::new(format!(
                "{} elementos de hormigón con armado propuesto. Marcados, los que no tienen armado dibujado; los que lo tienen se cambian solo si los marcas.",
                list.len()
            ))
            .size(12.0),
        );
        let mut apply = false;
        let mut close = false;
        ui.horizontal(|ui| {
            if ui.button("Todos").clicked() {
                list.iter_mut().for_each(|x| x.1 = true);
            }
            if ui.button("Solo los que no tienen armado").clicked() {
                list.iter_mut().for_each(|x| x.1 = x.0.is_new());
            }
            if ui.button("Ninguno").clicked() {
                list.iter_mut().for_each(|x| x.1 = false);
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let n = list.iter().filter(|x| x.1).count();
                if ui
                    .add_enabled(n > 0, egui::Button::new(format!("Aplicar a {n}")))
                    .clicked()
                {
                    apply = true;
                }
                if ui.button("Volver").clicked() {
                    close = true;
                }
            });
        });
        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .max_height(ui.available_height() - 8.0)
            .show(ui, |ui| {
                egui::Grid::new("calc-proposals")
                    .num_columns(3)
                    .striped(true)
                    .spacing([12.0, 4.0])
                    .show(ui, |ui| {
                        ui.label("");
                        ui.strong("Propuesto");
                        ui.strong("Ahora");
                        ui.end_row();
                        for (prop, on) in list.iter_mut() {
                            ui.checkbox(on, &prop.name);
                            ui.label(prop.reinforcement.summary());
                            match &prop.now {
                                Some(r) => ui.label(r.summary()),
                                None => ui.colored_label(p.muted, "sin armado"),
                            };
                            ui.end_row();
                        }
                    });
            });
        let chosen: Vec<(u64, archisim_core::Reinforcement)> = list
            .iter()
            .filter(|x| x.1)
            .map(|x| (x.0.element, x.0.reinforcement))
            .collect();
        if close {
            self.calc.proposals = None;
        }
        if apply {
            let n = chosen.len();
            let r = self.ed.transact("Armado propuesto", |m| {
                for (id, r) in &chosen {
                    m.set_reinforcement(*id, Some(*r))?;
                }
                Ok(())
            });
            self.report(
                r,
                &format!(
                    "Armado puesto en {n} elemento{}: míralo en planta a 1:50, en sección o en 3D, y su despiece en Archivo → Exportar.",
                    if n == 1 { "" } else { "s" }
                ),
            );
            self.calc.proposals = None;
        }
    }

    /// The calculation report as a PDF of A4 pages.
    pub(super) fn export_calc_report(&mut self) {
        let mut d = rfd::FileDialog::new()
            .set_title("Anejo de cálculo de la estructura (PDF)")
            .add_filter("PDF", &["pdf"])
            .set_file_name(format!("{}-anejo-calculo.pdf", self.file_name_stem()));
        if let Some(dir) = self.start_dir() {
            d = d.set_directory(dir);
        }
        let Some(path) = d.save_file() else {
            return;
        };
        let model = self.ed.shown();
        let blocks = archisim_struct::report::report(&model);
        let pages = archisim_plan::calc_report::pages(&model, &blocks);
        let sheets: Vec<(String, String)> = pages
            .into_iter()
            .enumerate()
            .map(|(i, svg)| (svg, format!("Anejo de cálculo {}", i + 1)))
            .collect();
        let r = crate::documents::pdf_of(&sheets, "Anejo de cálculo de la estructura")
            .and_then(|bytes| std::fs::write(&path, bytes).map_err(|e| e.to_string()));
        match r {
            Ok(()) => self.say(format!(
                "Anejo de cálculo: {} hojas A4 en {}.",
                sheets.len(),
                path.display()
            )),
            Err(e) => self.complain(format!("No se pudo exportar: {e}.")),
        }
    }

    /// The structure drawings as a PDF of A3 pages.
    pub(super) fn export_structure_drawings(&mut self) {
        let mut d = rfd::FileDialog::new()
            .set_title("Planos de estructura (PDF)")
            .add_filter("PDF", &["pdf"])
            .set_file_name(format!("{}-planos-estructura.pdf", self.file_name_stem()));
        if let Some(dir) = self.start_dir() {
            d = d.set_directory(dir);
        }
        let Some(path) = d.save_file() else {
            return;
        };
        let model = self.ed.shown();
        let pages = archisim_plan::structure_sheets::pages(&model);
        let sheets: Vec<(String, String)> = pages
            .into_iter()
            .enumerate()
            .map(|(i, svg)| (svg, format!("E-{:02}", i + 1)))
            .collect();
        let r = crate::documents::pdf_of(&sheets, "Planos de estructura")
            .and_then(|bytes| std::fs::write(&path, bytes).map_err(|e| e.to_string()));
        match r {
            Ok(()) => self.say(format!(
                "Planos de estructura: {} hojas A3 en {}.",
                sheets.len(),
                path.display()
            )),
            Err(e) => self.complain(format!("No se pudo exportar: {e}.")),
        }
    }
}
