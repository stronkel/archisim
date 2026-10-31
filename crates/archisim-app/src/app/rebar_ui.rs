//! The reinforcement of a column, beam, footing or slab in its properties
//! (E11): whether it has one, and the bars as a designer writes them —
//! «4Ø16, cercos Ø8 c/15» — with what the Código Estructural asks of them
//! that needs no calculation, checked as it is written.

use archisim_core::{Counted, RebarKind, Reinforcement, Spaced, BAR_DIAMETERS};
use eframe::egui::{Id, RichText, Ui};

use super::widgets::SwitchExt;
use super::{props, ArchiSimApp};
use crate::pick::Element;

fn element_id(e: Element) -> Option<u64> {
    match e {
        Element::Column(id) => Some(id.0),
        Element::Beam(id) => Some(id.0),
        Element::Footing(id) => Some(id.0),
        Element::Slab(id) => Some(id.0),
        _ => None,
    }
}

/// The diameters as choices.
fn diameter_choice(ui: &mut Ui, salt: (u64, &str), w: f32, now: f64) -> f64 {
    let names: Vec<String> = BAR_DIAMETERS
        .iter()
        .map(|d| format!("Ø{}", (d * 1000.0).round() as i64))
        .collect();
    let options: Vec<(f64, &str)> = BAR_DIAMETERS
        .iter()
        .zip(&names)
        .map(|(d, n)| (*d, n.as_str()))
        .collect();
    let now = BAR_DIAMETERS
        .iter()
        .copied()
        .min_by(|a, b| (a - now).abs().total_cmp(&(b - now).abs()))
        .unwrap_or(now);
    props::choice(ui, salt, w, now, &options)
}

impl ArchiSimApp {
    /// The group «Armado» of a column, beam, footing or slab.
    pub(super) fn rebar_group(&mut self, ui: &mut Ui, e: Element) {
        let Some(id) = element_id(e) else {
            return;
        };
        let m = self.ed.model();
        let Some(kind) = m.rebar_kind(id) else {
            return;
        };
        let now = m.reinforcement(id).copied();
        let mut new: Option<Option<Reinforcement>> = None;
        props::group(ui, "props-rebar", "Armado (hormigón armado)", |ui| {
            let mut on = now.is_some();
            if ui.switch(&mut on, "Lleva armado").changed() {
                new = Some(on.then(|| Reinforcement::default_for(kind)));
            }
            let Some(r) = now else {
                props::note(
                    ui,
                    "Dibuja y comprueba el armado; el cálculo de la estructura es de un programa de cálculo.",
                );
                return;
            };
            ui.label(RichText::new(r.summary()).strong());
            let mut r2 = r;
            Self::rebar_fields(ui, id, &mut r2);
            if r2 != r {
                new = Some(Some(r2));
            }
        });
        if let Some(r) = new {
            let result = self
                .ed
                .transact("Armado", |m| {
                    if r.is_some() {
                        archisim_library::rebar_material(m);
                    }
                    m.set_reinforcement(id, r)
                })
                .map_err(|e| e.to_string());
            self.report(result, "Armado cambiado.");
        }
        // What the code asks of it.
        if now.is_some() {
            self.rebar_checks(ui, id, kind);
        }
    }

    fn rebar_fields(ui: &mut Ui, id: u64, r: &mut Reinforcement) {
        let len = |ui: &mut Ui, name: &str, hint: &str, v: &mut f64| {
            props::row(ui, name, hint, |ui, w| {
                if let Some(x) =
                    super::length_field(ui, Id::unique(("rebar", id, name)), *v, w - 8.0)
                {
                    if x > 0.0 {
                        *v = x;
                    }
                }
            });
        };
        let counted = |ui: &mut Ui, name: &str, c: &mut Counted| {
            props::row(ui, name, "Número de barras y diámetro", |ui, w| {
                ui.horizontal(|ui| {
                    let half = (w - 12.0) / 2.0;
                    if let Some(t) = super::text_edit_committed(
                        ui,
                        Id::unique(("rebar-n", id, name)),
                        &c.count.to_string(),
                        half,
                    ) {
                        if let Ok(n) = t.trim().parse::<u32>() {
                            c.count = n.clamp(1, 60);
                        }
                    }
                    c.diameter = diameter_choice(ui, (id, name), half, c.diameter);
                });
            });
        };
        let spaced = |ui: &mut Ui, name: &str, s: &mut Spaced| {
            props::row(ui, name, "Diámetro y separación", |ui, w| {
                ui.horizontal(|ui| {
                    let half = (w - 12.0) / 2.0;
                    s.diameter = diameter_choice(ui, (id, name), half, s.diameter);
                    if let Some(x) =
                        super::length_field(ui, Id::unique(("rebar-s", id, name)), s.spacing, half)
                    {
                        if x > s.diameter {
                            s.spacing = x;
                        }
                    }
                });
            });
        };
        match r {
            Reinforcement::Column {
                cover,
                bars,
                ties,
                dense,
            } => {
                len(
                    ui,
                    "Recubrimiento",
                    "Del paramento a la cara del cerco",
                    cover,
                );
                counted(ui, "Barras", bars);
                spaced(ui, "Cercos", ties);
                let mut on = dense.is_some();
                if ui
                    .switch(&mut on, "Cercos más juntos en los extremos")
                    .changed()
                {
                    *dense = on.then_some((ties.spacing / 2.0, 0.6));
                }
                if let Some((s, l)) = dense.as_mut() {
                    len(ui, "Separación en extremos", "", s);
                    len(ui, "Longitud de cada extremo", "", l);
                }
            }
            Reinforcement::Beam {
                cover,
                bottom,
                top,
                stirrups,
            } => {
                len(
                    ui,
                    "Recubrimiento",
                    "Del paramento a la cara del estribo",
                    cover,
                );
                counted(ui, "Inferior", bottom);
                counted(ui, "Superior", top);
                spaced(ui, "Estribos", stirrups);
            }
            Reinforcement::Footing { cover, bottom, top }
            | Reinforcement::Slab { cover, bottom, top } => {
                len(ui, "Recubrimiento", "", cover);
                spaced(ui, "Inferior en x", &mut bottom[0]);
                spaced(ui, "Inferior en y", &mut bottom[1]);
                let mut on = top.is_some();
                if ui.switch(&mut on, "Malla superior").changed() {
                    *top = on.then_some(*bottom);
                }
                if let Some(t) = top.as_mut() {
                    spaced(ui, "Superior en x", &mut t[0]);
                    spaced(ui, "Superior en y", &mut t[1]);
                }
            }
        }
    }

    /// The checks of the Código Estructural on this element.
    fn rebar_checks(&mut self, ui: &mut Ui, id: u64, _kind: RebarKind) {
        let mut report = archisim_cte::Report::default();
        archisim_cte::structure::check(self.ed.model(), &mut report);
        let mine: Vec<_> = report
            .items
            .into_iter()
            .filter(|i| i.subject == archisim_cte::Subject::Reinforced(id))
            .collect();
        let fails: Vec<_> = mine
            .iter()
            .filter(|i| i.status == archisim_cte::Status::Fails)
            .collect();
        props::group(ui, "props-rebar-checks", "Comprobación del armado", |ui| {
            if fails.is_empty() {
                props::note(
                    ui,
                    &format!(
                        "Cumple las {} comprobaciones del Código Estructural que no necesitan cálculo.",
                        mine.len()
                    ),
                );
            }
            for i in fails {
                props::text_row(
                    ui,
                    i.rule,
                    &i.text,
                    &format!("{} ({})", i.value, i.required),
                );
            }
        });
    }
}
