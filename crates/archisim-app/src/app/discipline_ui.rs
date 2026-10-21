//! Disciplines in the interface (`docs/INTERFAZ.md` §3 and §8): what the
//! chosen discipline greys in the view, the discipline of the selected
//! element in Properties, and the eye and lock of each discipline in the
//! navigator.

use archisim_core::{DisciplineFocus, ElementDiscipline};
use eframe::egui::{self, Rect, Sense, Ui, Vec2};

use super::glyphs::{self, Glyph};
use super::props::{choice, group, note, row};
use super::theme::Palette;
use super::toolbox::Discipline;
use super::*;

/// The id of an element that has a discipline.
pub fn element_id(e: Element) -> Option<u64> {
    match e {
        Element::Terrain(i) => Some(i.0),
        other => super::params_ui::raw_id(other),
    }
}

/// What working in a discipline greys in the view.
pub fn focus_for(d: Discipline) -> DisciplineFocus {
    match d {
        Discipline::Concept => DisciplineFocus::Volumes,
        Discipline::Structure => DisciplineFocus::Only(ElementDiscipline::Structure),
        Discipline::Services => DisciplineFocus::Only(ElementDiscipline::Services),
        Discipline::Site => DisciplineFocus::Only(ElementDiscipline::Site),
        Discipline::Architecture
        | Discipline::Documentation
        | Discipline::Analysis
        | Discipline::Coordination => DisciplineFocus::None,
    }
}

fn glyph_of(d: ElementDiscipline) -> Glyph {
    match d {
        ElementDiscipline::Architecture => Glyph::Wall,
        ElementDiscipline::Structure => Glyph::Column,
        ElementDiscipline::Services => Glyph::Pipe,
        ElementDiscipline::Site => Glyph::Tree,
    }
}

impl ArchiSimApp {
    /// Changes the discipline worked in; with attenuation on, the view
    /// shown greys what is not of it.
    pub(super) fn set_discipline(&mut self, d: Discipline) {
        self.discipline = d;
        self.apply_focus();
    }

    /// The view shown takes the focus of the discipline (with attenuation
    /// on).
    pub(super) fn apply_focus(&mut self) {
        if self.attenuate {
            self.view_style.focus = focus_for(self.discipline);
        }
    }

    /// The focus the 3D view and the sections use now.
    pub(super) fn screen_focus(&self) -> DisciplineFocus {
        if self.attenuate {
            focus_for(self.discipline)
        } else {
            self.view_style.focus
        }
    }

    /// Elements of locked disciplines cannot be selected.
    pub(super) fn drop_locked_from_selection(&mut self) {
        if self.locked_disciplines.is_empty() {
            return;
        }
        let model = self.ed.model();
        let locked = &self.locked_disciplines;
        let before = self.ed.selection.len();
        let keep: std::collections::BTreeSet<Element> = self
            .ed
            .selection
            .iter()
            .copied()
            .filter(|e| {
                !element_id(*e)
                    .and_then(|id| model.discipline(id))
                    .is_some_and(|d| locked.contains(&d))
            })
            .collect();
        self.ed.selection = keep;
        if self.ed.selection.len() < before {
            self.say("Esa disciplina está bloqueada: desbloquéala en el navegador para editarla.");
        }
    }

    /// The eye and the lock of each discipline, at the top of the model.
    pub(super) fn nav_disciplines(&mut self, ui: &mut Ui) {
        let p = Palette::current(ui.ctx());
        let model = self.ed.model();
        let counts: Vec<(ElementDiscipline, usize)> = ElementDiscipline::ALL
            .iter()
            .map(|d| {
                let n = model
                    .levels()
                    .iter()
                    .flat_map(|l| crate::pick::elements_on_level(model, l.id))
                    .filter_map(element_id)
                    .filter(|id| model.discipline(*id) == Some(*d))
                    .count();
                (*d, n)
            })
            .collect();
        let mut toggle_eye = None;
        let mut toggle_lock = None;
        for (d, n) in counts {
            let hidden = self.hidden_disciplines.contains(&d);
            let locked = self.locked_disciplines.contains(&d);
            let (rect, _) =
                ui.allocate_exact_size(Vec2::new(ui.available_width(), 28.0), Sense::hover());
            let fg = if hidden { p.faint } else { p.text };
            glyphs::paint(
                ui.painter(),
                Rect::from_center_size(
                    rect.left_center() + Vec2::new(16.0, 0.0),
                    Vec2::splat(16.0),
                ),
                glyph_of(d),
                fg,
            );
            ui.painter().text(
                rect.left_center() + Vec2::new(32.0, 0.0),
                egui::Align2::LEFT_CENTER,
                d.name(),
                egui::FontId::proportional(12.5),
                fg,
            );
            ui.painter().text(
                rect.right_center() - Vec2::new(66.0, 0.0),
                egui::Align2::RIGHT_CENTER,
                n.to_string(),
                egui::FontId::monospace(10.5),
                p.muted,
            );
            let eye = Rect::from_center_size(
                rect.right_center() - Vec2::new(44.0, 0.0),
                Vec2::splat(22.0),
            );
            let lock = Rect::from_center_size(
                rect.right_center() - Vec2::new(16.0, 0.0),
                Vec2::splat(22.0),
            );
            let re = ui
                .interact(eye, ui.make_persistent_id(("eye", d)), Sense::click())
                .on_hover_text(if hidden {
                    "Mostrar"
                } else {
                    "Ocultar en las vistas"
                });
            let rl = ui
                .interact(lock, ui.make_persistent_id(("lock", d)), Sense::click())
                .on_hover_text(if locked {
                    "Desbloquear"
                } else {
                    "Bloquear: se ve, pero no se puede seleccionar"
                });
            for (r, on, glyph) in [(&re, !hidden, Glyph::Eye), (&rl, locked, Glyph::Lock)] {
                if r.hovered() {
                    ui.painter()
                        .rect_filled(r.rect, egui::CornerRadius::same(5), p.raised);
                }
                glyphs::paint(
                    ui.painter(),
                    r.rect.shrink(3.0),
                    glyph,
                    if on { p.accent } else { p.faint },
                );
            }
            if re.clicked() {
                toggle_eye = Some(d);
            }
            if rl.clicked() {
                toggle_lock = Some(d);
            }
        }
        if let Some(d) = toggle_eye {
            self.toggle_hidden_discipline(d);
        }
        if let Some(d) = toggle_lock {
            if !self.locked_disciplines.remove(&d) {
                self.locked_disciplines.insert(d);
                self.drop_locked_from_selection();
            }
        }
    }

    /// Shows or hides a discipline in every view.
    pub(super) fn toggle_hidden_discipline(&mut self, d: ElementDiscipline) {
        if !self.hidden_disciplines.remove(&d) {
            self.hidden_disciplines.insert(d);
        }
        let hidden = self.hidden_disciplines.clone();
        self.view_style.hidden_disciplines = hidden.clone();
        for t in &mut self.tabs {
            t.style.hidden_disciplines = hidden.clone();
        }
    }

    /// The discipline of the selected element, in Properties.
    pub(super) fn discipline_group(&mut self, ui: &mut Ui, e: Element) {
        let Some(id) = element_id(e) else {
            return;
        };
        let model = self.ed.model();
        let (Some(natural), Some(current)) = (model.natural_discipline(id), model.discipline(id))
        else {
            return;
        };
        let by_hand = model.discipline_set_by_hand(id);
        let mut picked = None;
        group(ui, "element-discipline", "Disciplina", |ui| {
            let auto = format!("Automática ({})", natural.name());
            let mut opts: Vec<(Option<ElementDiscipline>, &str)> = vec![(None, auto.as_str())];
            opts.extend(ElementDiscipline::ALL.iter().map(|d| (Some(*d), d.name())));
            let shown = if by_hand { Some(current) } else { None };
            let v = row(
                ui,
                "Disciplina",
                "Con qué disciplina se resalta o se atenúa. Automática: pilares y vigas, estructura; muros y forjados con una capa estructural de hormigón, acero o madera, estructura; terreno y vegetación, urbanismo; lo demás, arquitectura",
                |ui, w| choice(ui, "element-discipline-choice", w, shown, &opts),
            );
            if v != shown {
                picked = Some(v);
            }
            if by_hand {
                note(ui, "Cambiada a mano.");
            }
        });
        if let Some(v) = picked {
            let r = self.ed.set_discipline(&[id], v);
            self.report(r, "Disciplina cambiada.");
        }
    }
}
