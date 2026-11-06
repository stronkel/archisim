//! The water supply window (Colaboración → Cálculo de fontanería): what
//! the drawing does not give (the network's pressure at the meter and the
//! pipe), the calculation of the cold and hot networks as text, and
//! «Poner los diámetros calculados», which gives every pipe the diameter
//! its flow asks for in one undoable step.

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
            self.water.results = Some((rev, text, changes, short));
        }
        let (_, text, changes, short) = self.water.results.clone().expect("just made");
        let mut apply = false;
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
        egui::ScrollArea::both()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                ui.add(
                    egui::Label::new(egui::RichText::new(text).monospace().size(11.0))
                        .wrap_mode(egui::TextWrapMode::Extend),
                );
            });
        if apply {
            self.apply_water_sizes();
        }
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
