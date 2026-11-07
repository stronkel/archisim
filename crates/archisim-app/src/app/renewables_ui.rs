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
                            .on_hover_text(
                                "La del cálculo de la instalación; con captadores solares dibujados se calcula por f-Chart",
                            )
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
                ui.label("Volumen del acumulador");
                given(
                    ui,
                    &mut new.tank_volume,
                    " l",
                    "Darlo a mano; si no, se toma la demanda de un día a la temperatura de acumulación",
                );
                ui.end_row();
            });
        if new.acs_source == archisim_core::AcsSource::Solar {
            self.solar_group(ui, &mut new);
        }
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

    /// The collectors' data and the site's climate of the f-Chart: η0 and
    /// a1 of the collectors, and the irradiation and air of each month,
    /// typed, read from a monthly file of PVGIS or estimated.
    fn solar_group(&mut self, ui: &mut Ui, new: &mut archisim_core::RenewablesSite) {
        ui.add_space(6.0);
        egui::CollapsingHeader::new("Captadores solares y clima (f-Chart)")
            .default_open(true)
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label("Rendimiento óptico η0");
                    ui.add(
                        egui::DragValue::new(&mut new.collector_eta0)
                            .range(0.3..=0.9)
                            .speed(0.005)
                            .max_decimals(3),
                    )
                    .on_hover_text("De la ficha del captador (ensayo UNE-EN ISO 9806)");
                    ui.label("Pérdidas a1");
                    ui.add(
                        egui::DragValue::new(&mut new.collector_k1)
                            .range(0.5..=10.0)
                            .speed(0.05)
                            .max_decimals(2)
                            .suffix(" W/m²K"),
                    )
                    .on_hover_text("Coeficiente de pérdidas de la ficha del captador");
                });
                ui.horizontal(|ui| {
                    let estimated = new.irradiation.is_none() || new.ambient.is_none();
                    if estimated {
                        ui.weak("Clima estimado por la latitud y el agua fría.");
                    } else {
                        ui.weak("Clima del lugar.");
                    }
                    if ui
                        .button("Leer de un fichero de PVGIS…")
                        .on_hover_text(
                            "El fichero de «Monthly radiation» de PVGIS, con la radiación horizontal y la temperatura",
                        )
                        .clicked()
                    {
                        self.read_pvgis_into(new);
                    }
                    if ui
                        .button("Escribirlo a mano")
                        .on_hover_text("Parte de lo estimado, para corregirlo mes a mes")
                        .clicked()
                    {
                        let (sun, air) =
                            archisim_mep::solar::estimated_climate(self.ed.model(), new);
                        let round = |v: [f64; 12], k: f64| v.map(|x| (x * k).round() / k);
                        new.irradiation = Some(new.irradiation.unwrap_or(round(sun, 100.0)));
                        new.ambient = Some(new.ambient.unwrap_or(round(air, 10.0)));
                    }
                    if !estimated && ui.button("Volver a estimarlo").clicked() {
                        new.irradiation = None;
                        new.ambient = None;
                    }
                });
                if let (Some(sun), Some(air)) = (&mut new.irradiation, &mut new.ambient) {
                    const NAMES: [&str; 12] = [
                        "ene", "feb", "mar", "abr", "may", "jun", "jul", "ago", "sep", "oct",
                        "nov", "dic",
                    ];
                    egui::Grid::new("renewables-climate")
                        .num_columns(13)
                        .spacing([6.0, 4.0])
                        .show(ui, |ui| {
                            ui.label("");
                            for n in NAMES {
                                ui.label(n);
                            }
                            ui.end_row();
                            ui.label("H horizontal, kWh/m²·día");
                            for v in sun.iter_mut() {
                                ui.add(
                                    egui::DragValue::new(v)
                                        .range(0.0..=12.0)
                                        .speed(0.02)
                                        .max_decimals(2),
                                );
                            }
                            ui.end_row();
                            ui.label("Aire de día, °C");
                            for v in air.iter_mut() {
                                ui.add(
                                    egui::DragValue::new(v)
                                        .range(-30.0..=50.0)
                                        .speed(0.1)
                                        .max_decimals(1),
                                );
                            }
                            ui.end_row();
                        });
                }
            });
    }

    fn read_pvgis_into(&mut self, new: &mut archisim_core::RenewablesSite) {
        let mut d = rfd::FileDialog::new()
            .set_title("Leer el clima de PVGIS")
            .add_filter("PVGIS (CSV o texto)", &["csv", "txt", "CSV", "TXT"]);
        if let Some(dir) = self.start_dir() {
            d = d.set_directory(dir);
        }
        let Some(path) = d.pick_file() else {
            return;
        };
        match std::fs::read_to_string(&path)
            .ok()
            .and_then(|t| archisim_mep::solar::read_pvgis(&t))
        {
            Some((sun, air)) => {
                new.irradiation = Some(sun);
                if air.is_some() {
                    new.ambient = air;
                } else if new.ambient.is_none() {
                    let (_, air) = archisim_mep::solar::estimated_climate(self.ed.model(), new);
                    new.ambient = Some(air.map(|x| (x * 10.0).round() / 10.0));
                }
            }
            None => self.complain(
                "No se encontró en ese fichero la radiación mensual de PVGIS (columnas «month» y «H(h)_m»).",
            ),
        }
    }
}
