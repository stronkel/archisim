//! The renewables window (Colaboración → Energías renovables): what the
//! drawing does not give (whether the building is new, its province and
//! height above the capital, what makes the hot water renewable, the
//! panels' power, the built area or the demand where they are given) and
//! the DB-HE 4 and HE 5 calculation.

use eframe::egui::{self, Ui};

use super::*;

/// The window's state.
#[derive(Default)]
pub struct RenewablesWindow {
    pub open: bool,
    /// The calculation of a revision of the model: (revision, text, what
    /// fails).
    results: Option<(u64, String, usize)>,
    blocks: Vec<archisim_struct::report::Block>,
    as_text: bool,
}

impl ArchiSimApp {
    pub(super) fn renewables_dialog(&mut self, ctx: &egui::Context) {
        if !self.renewables.open {
            return;
        }
        let mut open = true;
        super::widgets::dialog("Energías renovables (DB-HE 4 y HE 5)")
            .open(&mut open)
            .default_width(900.0)
            .default_height(600.0)
            .show(ctx, |ui| self.renewables_contents(ui));
        self.renewables.open = open;
    }

    fn renewables_contents(&mut self, ui: &mut Ui) {
        let site = self.ed.model().renewables_site().clone();
        let mut new = site.clone();
        egui::Grid::new("renewables-site")
            .num_columns(4)
            .spacing([12.0, 6.0])
            .show(ui, |ui| {
                ui.label("Edificio nuevo");
                super::widgets::switch(ui, &mut new.new_building, "")
                    .on_hover_text(
                        "Nuevo, ampliación o reforma entera: HE 4 y HE 5 se aplican",
                    );
                ui.label("Provincia");
                egui::ComboBox::from_id_salt("renewables-province")
                    .selected_text(new.province.clone())
                    .width(170.0)
                    .show_ui(ui, |ui| {
                        for (p, _) in archisim_core::COLD_WATER {
                            ui.selectable_value(&mut new.province, p.to_string(), p);
                        }
                    })
                    .response
                    .on_hover_text("Su capital da el agua fría de cada mes (HE 4, Anejo G)");
                ui.end_row();
                ui.label("Altura sobre la capital");
                ui.add(
                    egui::DragValue::new(&mut new.above_capital)
                        .range(-1000.0..=3000.0)
                        .speed(5.0)
                        .suffix(" m"),
                )
                .on_hover_text("Altitud del municipio menos la de la capital: baja el agua fría");
                ui.label("Acumulación");
                ui.add(
                    egui::DragValue::new(&mut new.storage_temp)
                        .range(45.0..=80.0)
                        .speed(0.5)
                        .suffix(" °C"),
                );
                ui.end_row();
                ui.label("Agua caliente renovable con");
                egui::ComboBox::from_id_salt("renewables-source")
                    .selected_text(new.acs_source.name())
                    .width(200.0)
                    .show_ui(ui, |ui| {
                        for s in archisim_core::AcsSource::ALL {
                            ui.selectable_value(&mut new.acs_source, s, s.name());
                        }
                    });
                match new.acs_source {
                    archisim_core::AcsSource::HeatPump => {
                        ui.label("SCOP del equipo");
                        ui.add(
                            egui::DragValue::new(&mut new.scop)
                                .range(1.0..=8.0)
                                .speed(0.05)
                                .max_decimals(2),
                        )
                        .on_hover_text(
                            "Rendimiento estacional para el agua caliente (de su ficha); renovable 1 − 1/SCOP",
                        );
                    }
                    archisim_core::AcsSource::None => {
                        ui.label("");
                        ui.label("");
                    }
                    _ => {
                        ui.label("Fracción renovable");
                        let mut pct = new.fraction * 100.0;
                        if ui
                            .add(
                                egui::DragValue::new(&mut pct)
                                    .range(0.0..=100.0)
                                    .speed(0.5)
                                    .suffix(" %"),
                            )
                            .on_hover_text("La del cálculo de la instalación (p. ej. f-Chart)")
                            .changed()
                        {
                            new.fraction = pct / 100.0;
                        }
                    }
                }
                ui.end_row();
                ui.label("Potencia de las placas");
                ui.add(
                    egui::DragValue::new(&mut new.panel_kwp_m2)
                        .range(0.05..=0.4)
                        .speed(0.005)
                        .max_decimals(3)
                        .suffix(" kWp/m²"),
                )
                .on_hover_text("Pico por m² de placa: 400 W en 2 m², 0,2");
                ui.end_row();
                let given = |ui: &mut Ui, v: &mut Option<f64>, unit: &str, tip: &str| {
                    ui.horizontal(|ui| {
                        let mut on = v.is_some();
                        if super::widgets::switch(ui, &mut on, "").on_hover_text(tip).changed() {
                            *v = on.then_some(0.0);
                        }
                        if let Some(x) = v {
                            ui.add(
                                egui::DragValue::new(x)
                                    .range(0.0..=1.0e6)
                                    .speed(5.0)
                                    .suffix(unit),
                            );
                        } else {
                            ui.weak("calculada");
                        }
                    });
                };
                ui.label("Demanda de agua caliente");
                given(
                    ui,
                    &mut new.acs_demand,
                    " l/día",
                    "Darla a mano (a 60 °C): para otros usos, por la tabla c del Anejo F",
                );
                ui.label("Superficie construida");
                given(
                    ui,
                    &mut new.built_area,
                    " m²",
                    "Darla a mano si la de las plantas no es la del proyecto",
                );
                ui.end_row();
            });
        if new != site {
            let r = self
                .ed
                .transact("Datos de las renovables", |m| m.set_renewables_site(new));
            if let Err(e) = r {
                self.complain(format!("No se puede cambiar: {e}."));
            }
        }
        let rev = self.ed.revision();
        if self.renewables.results.as_ref().is_none_or(|r| r.0 != rev) {
            let r = archisim_mep::renewables::renewables(self.ed.model());
            self.renewables.blocks = archisim_plan::services_report::renewables(&r);
            self.renewables.results = Some((rev, archisim_mep::renewables::report(&r), r.fails()));
        }
        let (_, text, fails) = self.renewables.results.clone().expect("just made");
        ui.separator();
        let blocks = std::mem::take(&mut self.renewables.blocks);
        super::report_view::result(
            ui,
            &blocks,
            &text,
            &mut self.renewables.as_text,
            "las renovables",
            "renewables",
            Some(fails),
        );
        self.renewables.blocks = blocks;
    }
}
