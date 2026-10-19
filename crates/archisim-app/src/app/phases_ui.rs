//! Phases of a refurbishment in the interface: the phase of the selected
//! elements and which phases the plan shows.

use eframe::egui::Ui;

use super::props::{choice, group, row};
use super::*;

/// Phases in the interface, with their Spanish names.
pub const PHASES: [(archisim_core::Phase, &str); 3] = [
    (archisim_core::Phase::New, "Nuevo"),
    (archisim_core::Phase::Existing, "Existente"),
    (archisim_core::Phase::Demolished, "A derribar"),
];

impl ArchiSimApp {
    /// The phase of the selected elements (reforms): new, existing or to
    /// be demolished.
    pub(super) fn phase_group(&mut self, ui: &mut Ui, salt: &str, items: &[Element]) {
        let model = self.ed.model();
        let phased: Vec<Element> = items
            .iter()
            .copied()
            .filter(|e| {
                matches!(
                    e,
                    Element::Wall(_)
                        | Element::Slab(_)
                        | Element::Opening(_)
                        | Element::Column(_)
                        | Element::Beam(_)
                )
            })
            .collect();
        if phased.is_empty() {
            return;
        }
        let id_of = |e: &Element| match e {
            Element::Wall(i) => i.0,
            Element::Slab(i) => i.0,
            Element::Opening(i) => i.0,
            Element::Column(i) => i.0,
            Element::Beam(i) => i.0,
            _ => 0,
        };
        let first = model.phase(id_of(&phased[0]));
        let current = phased
            .iter()
            .all(|e| model.phase(id_of(e)) == first)
            .then_some(first);
        group(ui, salt, "Fases (reforma)", |ui| {
            let mut opts: Vec<(Option<archisim_core::Phase>, &str)> =
                PHASES.iter().map(|(p, n)| (Some(*p), *n)).collect();
            if current.is_none() {
                opts.insert(0, (None, "Varias"));
            }
            let picked = row(
                ui,
                "Fase",
                "Nuevo: lo que construye el proyecto; existente: lo que hay y se conserva; a derribar: lo que hay y se demuele (sus puertas y ventanas con él)",
                |ui, w| choice(ui, (salt, "phase"), w, current, &opts),
            );
            if let (Some(p), true) = (picked, picked != current) {
                match self.ed.set_phase(&phased, p) {
                    Ok(n) => self.say(format!("{n} elemento(s) en fase «{}».", phase_name(p))),
                    Err(e) => self.complain(format!("No se puede cambiar la fase: {e}.")),
                }
            }
        });
    }

    /// Phases of the active plan: what it shows, and what new elements are
    /// drawn as.
    pub(super) fn phase_view_group(&mut self, ui: &mut Ui) {
        group(ui, "view-phases", "Fases (reforma)", |ui| {
            let opts: Vec<(archisim_plan::phases::PhaseView, &str)> =
                archisim_plan::phases::PhaseView::ALL
                    .iter()
                    .map(|v| (*v, v.name()))
                    .collect();
            row(
                ui,
                "Mostrar",
                "Todo; estado actual (lo existente y lo que se derriba); demoliciones (en amarillo); estado reformado (lo nuevo en rojo)",
                |ui, w| {
                    self.phase_view = choice(ui, "view-phase", w, self.phase_view, &opts);
                },
            );
            let draw = self.ed.draw_phase();
            let opts: Vec<(archisim_core::Phase, &str)> = PHASES.to_vec();
            let picked = row(
                ui,
                "Dibujar como",
                "Fase de lo que dibujes a partir de ahora (el levantamiento del estado actual, como existente)",
                |ui, w| choice(ui, "draw-phase", w, draw, &opts),
            );
            if picked != draw {
                self.ed.set_draw_phase(picked);
                self.say(format!(
                    "Lo que dibujes ahora será «{}».",
                    phase_name(picked)
                ));
            }
        });
    }
}

pub fn phase_name(p: archisim_core::Phase) -> &'static str {
    PHASES.iter().find(|(q, _)| *q == p).map_or("", |(_, n)| *n)
}
