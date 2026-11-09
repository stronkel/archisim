//! The thermal window (Colaboración → Envolvente y cargas térmicas): what
//! the drawing does not give (the climate zone, the design temperatures,
//! the windows' transmittance and solar factor, and the conductivity of
//! the materials of the envelope), the DB-HE 1 check and the loads of
//! every room as text.

use eframe::egui::{self, Ui};

use super::*;

/// The window's state.
#[derive(Default)]
pub struct ClimateWindow {
    pub open: bool,
    /// The calculation of a revision of the model: (revision, text,
    /// what fails, the materials of the envelope, the calculation).
    #[allow(clippy::type_complexity)]
    results: Option<(
        u64,
        String,
        usize,
        Vec<archisim_core::MaterialId>,
        archisim_mep::thermal::Thermal,
    )>,
    /// The calculation drawn (`report_view`), and whether shown as typed
    /// text.
    blocks: Vec<archisim_struct::report::Block>,
    as_text: bool,
}

fn fmt_dec(v: f64, n: usize) -> String {
    format!("{v:.n$}").replace('.', ",")
}

/// The materials in the layers of the envelope's wall, slab and roof
/// types.
fn envelope_materials(
    model: &archisim_core::Model,
    t: &archisim_mep::thermal::Thermal,
) -> Vec<archisim_core::MaterialId> {
    let mut out: Vec<archisim_core::MaterialId> = Vec::new();
    for e in &t.elements {
        let layers: Vec<&archisim_core::Layer> = model
            .wall_types()
            .filter(|w| w.name == e.type_name)
            .flat_map(|w| w.layers.iter())
            .chain(
                model
                    .slab_types()
                    .filter(|s| s.name == e.type_name)
                    .flat_map(|s| s.layers.iter()),
            )
            .collect();
        for l in layers {
            if l.function != archisim_core::LayerFunction::AirGap && !out.contains(&l.material) {
                out.push(l.material);
            }
        }
    }
    out
}

impl ArchiSimApp {
    pub(super) fn climate_dialog(&mut self, ctx: &egui::Context) {
        if !self.climate.open {
            return;
        }
        let mut open = true;
        super::widgets::dialog("Envolvente (DB-HE 1) y cargas térmicas")
            .open(&mut open)
            .default_width(900.0)
            .default_height(600.0)
            .show(ctx, |ui| self.climate_contents(ui));
        self.climate.open = open;
    }

    fn climate_contents(&mut self, ui: &mut Ui) {
        let site = self.ed.model().thermal_site().clone();
        let mut new = site.clone();
        let rev = self.ed.revision();
        if self.climate.results.as_ref().is_none_or(|r| r.0 != rev) {
            let model = self.ed.model();
            let t = archisim_mep::thermal::thermal(model);
            let text = archisim_mep::thermal::report(model, &t);
            let mats = envelope_materials(model, &t);
            self.climate.blocks = archisim_plan::services_report::thermal(model, &t);
            self.climate.results = Some((rev, text, t.fails(), mats, t));
        }
        let (_, text, fails, mats, calc) = self.climate.results.clone().expect("just made");
        // The data in a scrolling part, so the results below stay in view.
        let max = (ui.available_height() * 0.5).max(160.0);
        egui::ScrollArea::vertical()
            .id_salt("climate-data")
            .max_height(max)
            .auto_shrink([false, true])
            .show(ui, |ui| {
        egui::Grid::new("climate-site")
            .num_columns(4)
            .spacing([12.0, 6.0])
            .show(ui, |ui| {
                ui.label("Zona climática");
                ui.horizontal(|ui| {
                    egui::ComboBox::from_id_salt("winter-zone")
                        .selected_text(new.winter.letter())
                        .width(48.0)
                        .show_ui(ui, |ui| {
                            for z in archisim_core::WinterZone::ALL {
                                ui.selectable_value(&mut new.winter, z, z.letter());
                            }
                        });
                    egui::ComboBox::from_id_salt("summer-zone")
                        .selected_text(new.summer.to_string())
                        .width(40.0)
                        .show_ui(ui, |ui| {
                            for s in 1..=4u8 {
                                ui.selectable_value(&mut new.summer, s, s.to_string());
                            }
                        });
                })
                .response
                .on_hover_text("La del municipio: DB-HE, anejo B (Huelva, A4)");
                if ui
                    .button("Temperaturas de la zona")
                    .on_hover_text("Pone las temperaturas exteriores habituales de la zona")
                    .clicked()
                {
                    new.winter_air = new.winter.usual_winter_air();
                    new.summer_air = archisim_core::usual_summer_air(new.summer);
                }
                ui.end_row();
                let t = |ui: &mut Ui, v: &mut f64| {
                    ui.add(
                        egui::DragValue::new(v)
                            .range(-30.0..=50.0)
                            .speed(0.5)
                            .suffix(" °C"),
                    );
                };
                ui.label("Exterior en invierno");
                t(ui, &mut new.winter_air);
                ui.label("Exterior en verano");
                t(ui, &mut new.summer_air);
                ui.end_row();
                ui.label("Interior calefactado");
                t(ui, &mut new.heated);
                ui.label("Interior refrigerado");
                t(ui, &mut new.cooled);
                ui.end_row();
                ui.label("U de las ventanas");
                ui.add(
                    egui::DragValue::new(&mut new.window_u)
                        .range(0.5..=6.0)
                        .speed(0.05)
                        .suffix(" W/m²K"),
                )
                .on_hover_text("Marco y vidrio juntos (UH)");
                ui.label("U de las puertas");
                ui.add(
                    egui::DragValue::new(&mut new.door_u)
                        .range(0.5..=6.0)
                        .speed(0.05)
                        .suffix(" W/m²K"),
                );
                ui.end_row();
                ui.label("Factor solar del vidrio");
                ui.add(
                    egui::DragValue::new(&mut new.glass_g)
                        .range(0.05..=1.0)
                        .speed(0.01),
                )
                .on_hover_text("Con su protección solar (g), para las cargas");
                ui.label("Con la protección móvil");
                ui.add(
                    egui::DragValue::new(&mut new.shaded_g)
                        .range(0.0..=1.0)
                        .speed(0.01),
                )
                .on_hover_text("g del vidrio con la persiana, el toldo o las contraventanas echados: el control solar de julio (HE 1, 3.1.2)");
                ui.end_row();
                ui.label("Fracción de marco");
                ui.add(
                    egui::DragValue::new(&mut new.frame_fraction)
                        .range(0.0..=0.9)
                        .speed(0.01)
                        .custom_formatter(|v, _| format!("{:.0} %", v * 100.0))
                        .custom_parser(|t| {
                            t.trim_end_matches('%').trim().replace(',', ".").parse::<f64>().ok().map(|v| v / 100.0)
                        }),
                );
                ui.label("Permeabilidad de las ventanas");
                egui::ComboBox::from_id_salt("air-class")
                    .selected_text(format!("Clase {}", new.air_class))
                    .show_ui(ui, |ui| {
                        for c in 1..=4u8 {
                            ui.selectable_value(
                                &mut new.air_class,
                                c,
                                format!(
                                    "Clase {c} ({} m³/h·m² a 100 Pa)",
                                    archisim_mep::envelope::q100(c)
                                ),
                            );
                        }
                    })
                    .response
                    .on_hover_text("UNE-EN 12207; la de cada tipo de ventana se cambia más abajo");
                ui.end_row();
                ui.label("n50 medida");
                ui.horizontal(|ui| {
                    let mut measured = new.n50.is_some();
                    if ui
                        .checkbox(&mut measured, "")
                        .on_hover_text("Con un ensayo de presurización; si no, sale del anejo H")
                        .changed()
                    {
                        new.n50 = measured.then_some(5.0);
                    }
                    if let Some(v) = new.n50.as_mut() {
                        ui.add(
                            egui::DragValue::new(v)
                                .range(0.1..=30.0)
                                .speed(0.05)
                                .suffix(" h⁻¹"),
                        );
                    }
                });
                ui.label("Parte opaca a 100 Pa");
                ui.add(
                    egui::DragValue::new(&mut new.opaque_air)
                        .range(0.0..=100.0)
                        .speed(0.5)
                        .suffix(" m³/h·m²"),
                )
                .on_hover_text("Anejo H: 16 en edificios nuevos, 29 en existentes");
                ui.end_row();
                ui.label("Higrometría");
                egui::ComboBox::from_id_salt("hygrometry")
                    .selected_text(format!("Clase {}", new.hygrometry))
                    .show_ui(ui, |ui| {
                        for c in 1..=5u8 {
                            ui.selectable_value(&mut new.hygrometry, c, format!("Clase {c}"));
                        }
                    })
                    .response
                    .on_hover_text("3 o menos en viviendas; 4 cocinas industriales, lavanderías; 5 piscinas");
                ui.label("Conductividad del terreno");
                ui.add(
                    egui::DragValue::new(&mut new.ground_conductivity)
                        .range(0.5..=5.0)
                        .speed(0.05)
                        .suffix(" W/mK"),
                )
                .on_hover_text("UNE-EN ISO 13370: 1,5 arcilla o limo, 2,0 arena o grava, 3,5 roca");
                ui.end_row();
            });
        // The ψ of every class of bridge found in the drawing.
        if !calc.bridges.is_empty() {
            egui::CollapsingHeader::new(format!(
                "Puentes térmicos: {} W/K (ψ por clase)",
                fmt_dec(calc.psi_l, 1)
            ))
            .id_salt("climate-psi")
            .default_open(false)
            .show(ui, |ui| {
                ui.label(
                    egui::RichText::new(
                        "Longitudes medidas en el dibujo. Los ψ habituales están escritos de memoria (DA DB-HE/3): cámbialos por los del detalle del proyecto.",
                    )
                    .small()
                    .weak(),
                );
                egui::Grid::new("climate-psi-grid")
                    .num_columns(5)
                    .spacing([12.0, 4.0])
                    .show(ui, |ui| {
                        for b in &calc.bridges {
                            ui.label(b.junction.name());
                            ui.label(b.insulation.name());
                            ui.label(format!("{} m", fmt_dec(b.length, 1)));
                            let mut v = archisim_mep::envelope::psi(
                                self.ed.model(),
                                b.junction,
                                b.insulation,
                            );
                            if ui
                                .add(
                                    egui::DragValue::new(&mut v)
                                        .range(-1.0..=3.0)
                                        .speed(0.005)
                                        .max_decimals(3)
                                        .suffix(" W/mK"),
                                )
                                .changed()
                            {
                                new.psi
                                    .insert(archisim_core::psi_key(b.junction, b.insulation), v);
                            }
                            if site.psi_set(b.junction, b.insulation).is_some() {
                                if ui.small_button("Habitual").clicked() {
                                    new.psi
                                        .remove(&archisim_core::psi_key(b.junction, b.insulation));
                                }
                            } else {
                                ui.label("");
                            }
                            ui.end_row();
                        }
                    });
            });
        }
        if !calc.ground.is_empty() {
            egui::CollapsingHeader::new("Aislamiento perimetral de la solera (UNE-EN ISO 13370)")
                .id_salt("climate-edge")
                .default_open(false)
                .show(ui, |ui| {
                    let mut on = new.edge.is_some();
                    if ui
                        .checkbox(&mut on, "Lleva aislamiento perimetral")
                        .changed()
                    {
                        new.edge = on.then_some(archisim_core::EdgeInsulation {
                            vertical: true,
                            size: 0.6,
                            resistance: 1.5,
                            thickness: 0.05,
                        });
                    }
                    if let Some(e) = new.edge.as_mut() {
                        egui::Grid::new("climate-edge-grid")
                            .num_columns(4)
                            .spacing([12.0, 4.0])
                            .show(ui, |ui| {
                                ui.label("Colocado");
                                ui.horizontal(|ui| {
                                    ui.selectable_value(&mut e.vertical, true, "Vertical");
                                    ui.selectable_value(&mut e.vertical, false, "Horizontal");
                                });
                                ui.label(if e.vertical { "Profundidad" } else { "Ancho" });
                                ui.add(
                                    egui::DragValue::new(&mut e.size)
                                        .range(0.1..=3.0)
                                        .speed(0.05)
                                        .suffix(" m"),
                                );
                                ui.end_row();
                                ui.label("Resistencia");
                                ui.add(
                                    egui::DragValue::new(&mut e.resistance)
                                        .range(0.0..=10.0)
                                        .speed(0.05)
                                        .suffix(" m²K/W"),
                                );
                                ui.label("Espesor");
                                ui.add(
                                    egui::DragValue::new(&mut e.thickness)
                                        .range(0.0..=0.3)
                                        .speed(0.005)
                                        .suffix(" m"),
                                );
                                ui.end_row();
                            });
                    }
                });
        }
        if !calc.air.windows.is_empty() {
            egui::CollapsingHeader::new("Permeabilidad al aire por tipo de ventana")
                .id_salt("climate-air")
                .default_open(false)
                .show(ui, |ui| {
                    egui::Grid::new("climate-air-grid")
                        .num_columns(3)
                        .spacing([12.0, 4.0])
                        .show(ui, |ui| {
                            for w in &calc.air.windows {
                                ui.label(&w.type_name);
                                let mut c = site.air_class_of(w.opening_type);
                                egui::ComboBox::from_id_salt(("air-of", w.opening_type.0))
                                    .selected_text(format!("Clase {c}"))
                                    .show_ui(ui, |ui| {
                                        for k in 1..=4u8 {
                                            ui.selectable_value(&mut c, k, format!("Clase {k}"));
                                        }
                                    });
                                if c != site.air_class_of(w.opening_type) {
                                    new.air_classes.insert(w.opening_type, c);
                                }
                                if site.air_classes.contains_key(&w.opening_type) {
                                    if ui.small_button("La general").clicked() {
                                        new.air_classes.remove(&w.opening_type);
                                    }
                                } else {
                                    ui.label("");
                                }
                                ui.end_row();
                            }
                        });
                });
        }
        if !mats.is_empty() {
            egui::CollapsingHeader::new("Conductividad de los materiales de la envolvente")
                .default_open(false)
                .show(ui, |ui| {
                    egui::Grid::new("climate-lambda")
                        .num_columns(3)
                        .spacing([12.0, 4.0])
                        .show(ui, |ui| {
                            for m in &mats {
                                let name = self
                                    .ed
                                    .model()
                                    .material(*m)
                                    .map_or(String::new(), |x| x.name.clone());
                                let mut v = self.ed.model().conductivity(*m);
                                ui.label(name);
                                let r = ui.add(
                                    egui::DragValue::new(&mut v)
                                        .range(0.01..=400.0)
                                        .speed(0.005)
                                        .max_decimals(3)
                                        .suffix(" W/mK"),
                                );
                                if r.changed() {
                                    new.conductivity.insert(*m, v);
                                }
                                if site.conductivity.contains_key(m) {
                                    if ui.small_button("Habitual").clicked() {
                                        new.conductivity.remove(m);
                                    }
                                } else {
                                    ui.label("");
                                }
                                ui.end_row();
                            }
                        });
                });
        }
            });
        if new != site {
            let r = self
                .ed
                .transact("Datos de la envolvente", |m| m.set_thermal_site(new));
            if let Err(e) = r {
                self.complain(format!("No se puede cambiar: {e}."));
            }
        }
        ui.separator();
        let blocks = std::mem::take(&mut self.climate.blocks);
        super::report_view::result(
            ui,
            &blocks,
            &text,
            &mut self.climate.as_text,
            "la envolvente",
            "climate",
            Some(fails),
        );
        self.climate.blocks = blocks;
    }
}

impl ArchiSimApp {
    /// The installations' report («Memoria de instalaciones») as a PDF of
    /// A4 pages.
    pub(super) fn export_services_report(&mut self) {
        let mut d = rfd::FileDialog::new()
            .set_title("Memoria de instalaciones (PDF)")
            .add_filter("PDF", &["pdf"])
            .set_file_name(format!(
                "{}-memoria-instalaciones.pdf",
                self.file_name_stem()
            ));
        if let Some(dir) = self.start_dir() {
            d = d.set_directory(dir);
        }
        let Some(path) = d.save_file() else {
            return;
        };
        let model = self.ed.shown();
        let blocks = archisim_plan::services_report::report(&model);
        let pages = archisim_plan::calc_report::pages(&model, &blocks);
        let sheets: Vec<(String, String)> = pages
            .into_iter()
            .enumerate()
            .map(|(i, svg)| (svg, format!("Memoria de instalaciones {}", i + 1)))
            .collect();
        let r = crate::documents::pdf_of(&sheets, "Memoria de instalaciones")
            .and_then(|bytes| std::fs::write(&path, bytes).map_err(|e| e.to_string()));
        match r {
            Ok(()) => self.say(format!(
                "Memoria de instalaciones: {} hojas A4 en {}.",
                sheets.len(),
                path.display()
            )),
            Err(e) => self.complain(format!("No se pudo exportar: {e}.")),
        }
    }
}
