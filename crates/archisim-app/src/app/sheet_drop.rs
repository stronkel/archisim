//! Dragging views and schedules from the navigator onto a sheet: a plan,
//! section, elevation, schedule or the 3D view is taken by its row and
//! dropped where it goes on the paper, at its own scale.

use archisim_core::{LevelId, Point2, ScheduleKind, SectionId};
use eframe::egui::{self, Response, Ui};

use super::sheet_ui::{Placed, ViewChoice};
use super::tabs::TabKind;
use super::theme::Palette;
use super::ArchiSimApp;

/// What is being carried.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Drop {
    Level(LevelId),
    /// A saved plan, with its scale, cut plane and look.
    View(archisim_core::PlanViewId),
    Section(SectionId),
    Table(ScheduleKind),
    /// An axonometric of the model as the 3D view looks at it.
    ThreeD,
    /// A view only for drawing (E7), at its own scale.
    Drafting(archisim_core::DraftViewId),
}

/// What travels with the pointer: the thing and its name.
#[derive(Debug, Clone)]
struct Carried {
    what: Drop,
    label: String,
}

/// Makes a row of the navigator something to drag onto a sheet.
pub fn drag_source(ui: &Ui, resp: &Response, what: Drop, label: &str) {
    if resp.drag_started() {
        egui::DragAndDrop::set_payload(
            ui.ctx(),
            Carried {
                what,
                label: label.to_string(),
            },
        );
    }
}

/// Draws, beside the pointer, what is being carried.
pub fn paint_carried(ctx: &egui::Context, on_sheet: bool) {
    let Some(c) = egui::DragAndDrop::payload::<Carried>(ctx) else {
        return;
    };
    let Some(pos) = ctx.pointer_latest_pos() else {
        return;
    };
    let p = Palette::current(ctx);
    let text = if on_sheet {
        format!("Colocar {} aquí", c.label)
    } else {
        format!("{}: suéltalo en un plano", c.label)
    };
    egui::Area::new(egui::Id::unique("carried-to-sheet"))
        .order(egui::Order::Tooltip)
        .fixed_pos(pos + egui::vec2(14.0, 10.0))
        .interactable(false)
        .show(ctx, |ui| {
            egui::Frame::popup(ui.style()).show(ui, |ui| {
                ui.label(egui::RichText::new(text).small().color(if on_sheet {
                    p.accent
                } else {
                    p.text
                }));
            });
        });
}

impl ArchiSimApp {
    /// The scale a view is placed at: that of its tab if it is open, or
    /// the usual one for its kind.
    fn drop_scale(&self, what: Drop) -> f64 {
        if let Drop::View(id) = what {
            if let Some(v) = self.ed.model().plan_view(id) {
                return v.scale;
            }
        }
        let kind = match what {
            Drop::Level(l) => Some(TabKind::Plan(l)),
            Drop::Section(s) => Some(TabKind::Section(s)),
            _ => None,
        };
        let open = kind.and_then(|k| {
            self.tabs
                .iter()
                .enumerate()
                .find(|(_, t)| t.kind == k)
                .map(|(i, t)| {
                    if i == self.active_tab {
                        self.plan_scale
                    } else {
                        t.scale
                    }
                })
        });
        if let (None, Drop::ThreeD) = (open, what) {
            return self.axo_scale();
        }
        open.unwrap_or(match what {
            Drop::Level(_) => archisim_plan::PlanOptions::default().scale,
            _ => 100.0,
        })
    }

    /// The 3D view has no scale of its own: the first usual one at which
    /// the whole model takes at most half the paper's width and two thirds
    /// of its height.
    fn axo_scale(&self) -> f64 {
        let super::ViewMode::Sheet(sheet) = self.mode else {
            return 100.0;
        };
        let Some(s) = self.ed.model().sheet(sheet) else {
            return 100.0;
        };
        let (w, h) = (s.width / 1000.0, s.height / 1000.0);
        let (lo, hi) = archisim_plan::axo::axo_extent(&self.ed.shown(), &self.camera_axo(), 100.0);
        [50.0, 100.0, 200.0, 250.0, 500.0, 1000.0]
            .into_iter()
            .find(|k| (hi.x - lo.x) / k <= w * 0.5 && (hi.y - lo.y) / k <= h * 0.66)
            .unwrap_or(1000.0)
    }

    /// On the sheet shown: takes what is dropped on it and places it with
    /// its centre (a schedule, its top-left corner) at `at`.
    pub(super) fn sheet_drop(
        &mut self,
        ctx: &egui::Context,
        response: &Response,
        at: Option<Point2>,
    ) {
        let hovering = response.dnd_hover_payload::<Carried>().is_some();
        paint_carried(ctx, hovering);
        let Some(c) = response.dnd_release_payload::<Carried>() else {
            return;
        };
        if let Some(at) = at {
            self.place_dropped(c.what, &c.label, at);
        }
    }

    /// Places what was dropped on the sheet shown, at `at` (paper metres).
    pub(super) fn place_dropped(&mut self, what: Drop, label: &str, at: Point2) {
        let super::ViewMode::Sheet(sheet) = self.mode else {
            return;
        };
        let r = match what {
            Drop::Table(kind) => self
                .ed
                .add_table(sheet, kind, self.schedule_level, at)
                .map(Placed::Table),
            Drop::Drafting(d) => self
                .ed
                .transact("Vista de dibujo en el plano", |m| {
                    m.add_drafting_viewport(sheet, d, at, None)
                })
                .map(Placed::View),
            Drop::View(id) => {
                let Some(v) = self.ed.model().plan_view(id).cloned() else {
                    return;
                };
                // The view as saved: its scale, cut plane, look and kind.
                self.place_view(sheet, ViewChoice::Level(v.level), v.scale, at)
                    .and_then(|vp| {
                        self.ed
                            .transact("Vista guardada en el plano", |m| {
                                let mut p = m
                                    .viewport(vp)
                                    .cloned()
                                    .ok_or(archisim_core::ModelError::UnknownViewport(vp))?;
                                p.cut_plane = v.cut_plane;
                                p.style = v.style.clone();
                                p.ceiling = v.kind == archisim_core::PlanKind::Ceiling;
                                p.title = v.name.to_uppercase();
                                m.set_viewport(p)
                            })
                            .map(|_| vp)
                    })
                    .map(Placed::View)
            }
            other => {
                let scale = self.drop_scale(other);
                let view = match other {
                    Drop::Level(l) => ViewChoice::Level(l),
                    Drop::Section(s) => ViewChoice::Section(s),
                    _ => ViewChoice::ThreeD,
                };
                self.place_view(sheet, view, scale, at).map(Placed::View)
            }
        };
        match r {
            Ok(p) => {
                self.sheet_sel = Some(p);
                self.say(format!(
                    "En el plano: {label}. Arrástralo para moverlo; la escala, en Propiedades."
                ));
            }
            Err(e) => self.complain(format!("No se puede colocar: {e}.")),
        }
    }
}
