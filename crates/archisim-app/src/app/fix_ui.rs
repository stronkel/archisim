//! «Arreglar lo que no cumple», shared by the structural calculation and
//! the code check: the fix is worked out in another thread on a copy of
//! the model, then shown as a table of what changes (before → after),
//! with what it could not fix, and made in one step that Deshacer undoes.

use std::sync::mpsc::{channel, Receiver};

use eframe::egui::{self, Ui};

use super::theme::Palette;
use super::*;

/// A row of what changes.
#[derive(Clone, Debug)]
pub struct Row {
    pub element: Option<Element>,
    pub level: Option<archisim_core::LevelId>,
    pub name: String,
    pub what: &'static str,
    pub before: String,
    pub after: String,
}

/// A fix worked out, ready to be shown.
pub struct Shown<F> {
    pub fix: F,
    pub rows: Vec<Row>,
    pub fails_before: usize,
    pub fails_after: usize,
    pub left: Vec<String>,
}

/// The state of a window's fix: being worked out, or shown, for a
/// revision of the model.
pub enum FixState<F> {
    Working(u64, Receiver<Shown<F>>),
    Ready(u64, Shown<F>),
}

impl<F: Send + 'static> FixState<F> {
    /// Starts working out a fix in another thread.
    pub fn start(
        rev: u64,
        model: archisim_core::Model,
        ctx: egui::Context,
        work: impl FnOnce(&archisim_core::Model) -> Shown<F> + Send + 'static,
    ) -> Self {
        let (tx, rx) = channel();
        crate::platform::spawn(move || {
            let _ = tx.send(work(&model));
            ctx.request_repaint();
        });
        FixState::Working(rev, rx)
    }

    pub fn revision(&self) -> u64 {
        match self {
            FixState::Working(r, _) | FixState::Ready(r, _) => *r,
        }
    }

    /// Picks up the result when it is there.
    pub fn poll(&mut self) {
        if let FixState::Working(rev, rx) = self {
            if let Ok(s) = rx.try_recv() {
                *self = FixState::Ready(*rev, s);
            }
        }
    }
}

/// What was asked in the panel.
pub enum Answer {
    Nothing,
    Apply,
    Back,
    Pick(Option<Element>, Option<archisim_core::LevelId>),
}

/// The panel: what changes and what is left.
pub fn panel<F>(ui: &mut Ui, state: &FixState<F>, note: &str) -> Answer {
    let p = Palette::current(ui.ctx());
    let mut answer = Answer::Nothing;
    let s = match state {
        FixState::Working(..) => {
            ui.horizontal(|ui| {
                ui.spinner();
                ui.label("Buscando la solución más ajustada: se comprueba otra vez después de cada cambio…");
            });
            ui.ctx()
                .request_repaint_after(std::time::Duration::from_millis(200));
            if ui.button("Volver").clicked() {
                answer = Answer::Back;
            }
            return answer;
        }
        FixState::Ready(_, s) => s,
    };
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new("No cumplen:").size(12.0));
        ui.colored_label(p.bad, format!("{} ahora", s.fails_before));
        ui.label("→");
        if s.fails_after == 0 {
            ui.colored_label(p.good, "ninguna después de los cambios");
        } else {
            ui.colored_label(p.bad, format!("{} después de los cambios", s.fails_after));
        }
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui
                .add_enabled(
                    !s.rows.is_empty(),
                    egui::Button::new(format!("Aplicar los {} cambios", s.rows.len())),
                )
                .on_hover_text("Se hacen todos juntos: Deshacer los quita de una vez")
                .clicked()
            {
                answer = Answer::Apply;
            }
            if ui.button("Volver").clicked() {
                answer = Answer::Back;
            }
        });
    });
    ui.label(
        egui::RichText::new(format!(
            "{note} Clic en un elemento para verlo en su planta."
        ))
        .weak()
        .size(11.0),
    );
    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .max_height(ui.available_height() - 8.0)
        .show(ui, |ui| {
            if s.rows.is_empty() {
                ui.colored_label(p.muted, "No hay nada que se pueda arreglar solo.");
            } else {
                egui::Grid::new("fix-rows")
                    .num_columns(4)
                    .striped(true)
                    .spacing([12.0, 4.0])
                    .show(ui, |ui| {
                        ui.strong("Elemento");
                        ui.strong("Cambia");
                        ui.strong("Antes");
                        ui.strong("Después");
                        ui.end_row();
                        for r in &s.rows {
                            if ui.link(&r.name).clicked() {
                                answer = Answer::Pick(r.element, r.level);
                            }
                            ui.label(r.what);
                            ui.colored_label(p.muted, &r.before);
                            ui.label(&r.after);
                            ui.end_row();
                        }
                    });
            }
            if !s.left.is_empty() {
                ui.add_space(8.0);
                ui.strong(format!("Lo que tienes que decidir tú ({}):", s.left.len()));
                for l in &s.left {
                    ui.label(egui::RichText::new(format!("· {l}")).size(11.5));
                }
            }
        });
    answer
}

impl ArchiSimApp {
    /// Selects what a row is about, in its plan.
    pub(super) fn fix_pick(
        &mut self,
        element: Option<Element>,
        level: Option<archisim_core::LevelId>,
    ) {
        if let Some(level) = level {
            self.open_view(super::tabs::TabKind::Plan(level));
        }
        self.set_tool(Tool::Arrow);
        self.ed.selection = element.into_iter().collect();
    }
}

/// The element of a structural element's id, and its level.
pub fn structural_element(
    m: &archisim_core::Model,
    id: u64,
) -> (Option<Element>, Option<archisim_core::LevelId>) {
    use archisim_core::{BeamId, ColumnId, FootingId};
    if let Some(c) = m.column(ColumnId(id)) {
        return (Some(Element::Column(c.id)), Some(c.level));
    }
    if let Some(b) = m.beam(BeamId(id)) {
        return (Some(Element::Beam(b.id)), Some(b.level));
    }
    if let Some(f) = m.footing(FootingId(id)) {
        return (Some(Element::Footing(f.id)), Some(f.level));
    }
    let e = archisim_geom::rebar::owner_of(m, id).map(crate::pick::of_solid);
    (e, None)
}
