//! «Equipar la vivienda» (M19, `docs/MODULAR.md`): the same as
//! `archisim equipar`, from the app, in one step of Undo: the installations
//! each room needs by its use, joined up and sized by the calculations,
//! and, with modular boxes, the system's rules on a model brought from
//! another program.

use eframe::egui::{self, Ui};

use super::*;

/// The window's state.
#[derive(Default)]
pub struct EquipWindow {
    pub open: bool,
    /// The collection the furniture takes (none: as it is).
    collection: Option<&'static str>,
    /// The windows' U and the airtightness to declare, if wanted.
    uw: Option<f64>,
    n50: Option<f64>,
    /// Even if the project has installations already.
    again: bool,
    /// What the last run did.
    report: Option<String>,
}

impl ArchiSimApp {
    pub(super) fn equip_dialog(&mut self, ctx: &egui::Context) {
        if !self.equip.open {
            return;
        }
        let mut open = true;
        super::widgets::dialog("Equipar la vivienda")
            .open(&mut open)
            .default_width(620.0)
            .show(ctx, |ui| self.equip_contents(ui));
        self.equip.open = open;
    }

    fn equip_contents(&mut self, ui: &mut Ui) {
        ui.label(
            "Pone las instalaciones de cada local por su uso y las calcula: electricidad (REBT), \
             fontanería y ACS (HS 4), saneamiento y pluviales (HS 5) y ventilación (HS 3). Con \
             cajas, aplica además las reglas del sistema a lo que viene de otro programa: \
             entramado, suelos y cubiertas de las cajas, un muro por junta, montantes y chasis \
             por cálculo. Hace falta que los locales tengan nombre y uso.",
        );
        ui.add_space(6.0);
        let has = {
            let m = self.ed.model();
            m.devices().any(|d| {
                matches!(
                    d.kind,
                    archisim_core::DeviceKind::Panel | archisim_core::DeviceKind::WaterMeter
                )
            })
        };
        let rooms = self.ed.model().rooms().count();
        egui::Grid::new("equip-options")
            .num_columns(2)
            .spacing([12.0, 6.0])
            .show(ui, |ui| {
                ui.label("Colección de los muebles");
                egui::ComboBox::from_id_salt("equip-collection")
                    .selected_text(self.equip.collection.unwrap_or("Como están"))
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut self.equip.collection, None, "Como están");
                        for c in archisim_library::collections::COLLECTIONS {
                            ui.selectable_value(&mut self.equip.collection, Some(c.name), c.name);
                        }
                    });
                ui.end_row();
                ui.label("U de las ventanas");
                ui.horizontal(|ui| {
                    let mut on = self.equip.uw.is_some();
                    ui.checkbox(&mut on, "");
                    if on {
                        let mut v = self.equip.uw.unwrap_or(1.4);
                        ui.add(
                            egui::DragValue::new(&mut v)
                                .range(0.6..=5.7)
                                .speed(0.05)
                                .suffix(" W/m²K"),
                        );
                        self.equip.uw = Some(v);
                        ui.weak("a confirmar con la carpintería");
                    } else {
                        self.equip.uw = None;
                        ui.weak("la de Envolvente y cargas");
                    }
                });
                ui.end_row();
                ui.label("Estanqueidad n50");
                ui.horizontal(|ui| {
                    let mut on = self.equip.n50.is_some();
                    ui.checkbox(&mut on, "");
                    if on {
                        let mut v = self.equip.n50.unwrap_or(3.0);
                        ui.add(
                            egui::DragValue::new(&mut v)
                                .range(0.3..=10.0)
                                .speed(0.1)
                                .suffix(" h⁻¹"),
                        );
                        self.equip.n50 = Some(v);
                        ui.weak("objetivo, con el ensayo de presurización");
                    } else {
                        self.equip.n50 = None;
                        ui.weak("la estimada");
                    }
                });
                ui.end_row();
            });
        if has {
            ui.add_space(4.0);
            ui.colored_label(
                ui.visuals().warn_fg_color,
                "El proyecto ya tiene instalaciones: se añadirían otras encima.",
            );
            ui.checkbox(&mut self.equip.again, "Añadirlas igualmente");
        }
        ui.add_space(6.0);
        let can = rooms > 0 && (!has || self.equip.again);
        if ui
            .add_enabled(can, egui::Button::new("Equipar"))
            .on_disabled_hover_text(if rooms == 0 {
                "Pon antes los locales con su nombre y uso"
            } else {
                "Marca «Añadirlas igualmente»"
            })
            .clicked()
        {
            let (coll, n50, uw) = (self.equip.collection, self.equip.n50, self.equip.uw);
            let mut text = String::new();
            let r = self.ed.transact("Equipar la vivienda", |m| {
                text = archisim_equip::equip(m, None, coll, &[], n50, uw)
                    .map_err(|e| archisim_core::ModelError::InvalidParameter(e.to_string()))?;
                Ok(())
            });
            self.equip.report = Some(match r {
                Ok(()) => text,
                Err(e) => format!("No se ha podido: {e}"),
            });
        }
        if let Some(r) = &self.equip.report {
            ui.separator();
            egui::ScrollArea::vertical()
                .max_height(320.0)
                .show(ui, |ui| {
                    ui.label(egui::RichText::new(r).monospace());
                });
        }
    }
}
