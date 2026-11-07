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
    /// what fails, the materials of the envelope).
    results: Option<(u64, String, usize, Vec<archisim_core::MaterialId>)>,
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
                .on_hover_text("Con su protección solar (g)");
                ui.end_row();
            });
        let rev = self.ed.revision();
        if self.climate.results.as_ref().is_none_or(|r| r.0 != rev) {
            let model = self.ed.model();
            let t = archisim_mep::thermal::thermal(model);
            let text = archisim_mep::thermal::report(model, &t);
            let mats = envelope_materials(model, &t);
            self.climate.results = Some((rev, text, t.fails(), mats));
        }
        let (_, text, fails, mats) = self.climate.results.clone().expect("just made");
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
        if new != site {
            let r = self
                .ed
                .transact("Datos de la envolvente", |m| m.set_thermal_site(new));
            if let Err(e) = r {
                self.complain(format!("No se puede cambiar: {e}."));
            }
        }
        ui.separator();
        let p = super::theme::Palette::current(ui.ctx());
        if fails > 0 {
            ui.colored_label(
                p.bad,
                format!(
                    "{fails} cosa{} por encima del límite",
                    if fails == 1 { "" } else { "s" }
                ),
            );
        } else {
            ui.colored_label(p.good, "La envolvente cumple el DB-HE 1");
        }
        egui::ScrollArea::both()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                ui.add(
                    egui::Label::new(egui::RichText::new(text).monospace().size(11.0))
                        .wrap_mode(egui::TextWrapMode::Extend),
                );
            });
    }
}
