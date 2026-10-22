//! Onboarding missions on the screen (`docs/INICIACION.md`): the welcome
//! card the first time, the card of the mission in hand at the bottom
//! right of the drawing, the ring around the control to use, and the
//! window listing them (Ayuda → Misiones de iniciación). The steps and
//! when each is done are in `crate::missions`; the size of the text, in
//! `crate::prefs`.
//!
//! Controls a mission may point at mark themselves with [`mark`] as they
//! are drawn ("tile:Muro", "method:Rectángulo", "seg:Proyecto"…); the ring
//! is painted at the end of the frame over everything.

use std::collections::HashMap;

use eframe::egui::{self, Align2, Color32, CornerRadius, Id, Rect, Stroke, Vec2};

use super::tabs::TabKind;
use super::theme::Palette;
use super::widgets::strong_font;
use super::ArchiSimApp;
use crate::missions::{self, Obs, Progress, ViewKind, MISSIONS};
use crate::prefs::TextSize;

#[derive(Default)]
pub struct MissionsUi {
    pub progress: Option<Progress>,
    pub window_open: bool,
}

#[derive(Clone, Default)]
struct Marks(HashMap<String, Rect>);

fn marks_id() -> Id {
    Id::unique("mission-marks")
}

/// A control a mission may point at says where it is this frame.
pub fn mark(ctx: &egui::Context, key: impl Into<String>, rect: Rect) {
    ctx.data_mut(|d| {
        d.get_temp_mut_or_default::<Marks>(marks_id())
            .0
            .insert(key.into(), rect);
    });
}

/// Where a control was drawn this frame.
pub(super) fn marked(ctx: &egui::Context, key: &str) -> Option<Rect> {
    ctx.data(|d| d.get_temp::<Marks>(marks_id()))
        .and_then(|m| m.0.get(key).copied())
}

/// Forget last frame's marks (start of the frame).
pub fn clear_marks(ctx: &egui::Context) {
    ctx.data_mut(|d| d.remove_temp::<Marks>(marks_id()));
}

/// The control to ring for a target: itself if it is on the screen, or
/// the one that brings it there (the Proyecto tab of the navigator, the
/// Plantas… button, the Arquitectura discipline).
fn ring_target(ctx: &egui::Context, target: &str) -> Option<Rect> {
    if target.is_empty() {
        return None;
    }
    let chain: &[&str] = if target.starts_with("tile:") {
        &["rail:Arquitectura"]
    } else if target == "button:Añadir planta encima" {
        &["button:Plantas…", "seg:Proyecto"]
    } else if target.starts_with("nav:") || target.starts_with("button:") {
        &["seg:Proyecto"]
    } else {
        &[]
    };
    std::iter::once(target)
        .chain(chain.iter().copied())
        .find_map(|k| marked(ctx, k))
}

impl ArchiSimApp {
    /// What the program looks like now, for the missions.
    pub(super) fn observe(&self) -> Obs {
        let m = self.ed.model();
        let (mut doors, mut windows) = (0, 0);
        for o in m.openings() {
            match m.opening_type(o.opening_type) {
                Some(t) if t.is_door() => doors += 1,
                Some(t) if t.is_window() => windows += 1,
                _ => {}
            }
        }
        let mut levels: Vec<_> = m.levels().iter().map(|l| (l.elevation, l.id)).collect();
        levels.sort_by(|a, b| a.0.total_cmp(&b.0));
        let view = match self.tabs.get(self.active_tab).map(|t| t.kind) {
            Some(TabKind::ThreeD) => ViewKind::ThreeD,
            Some(TabKind::Section(_)) => ViewKind::Section,
            Some(TabKind::Sheet(_)) => ViewKind::Sheet,
            Some(TabKind::Schedule(_)) => ViewKind::Schedule,
            _ => ViewKind::Plan,
        };
        Obs {
            tool: Some(self.tool),
            discipline: self.discipline.name(),
            walls: m.walls().count(),
            slabs: m.slabs().count(),
            doors,
            windows,
            rooms: m.rooms().count(),
            levels: levels.len(),
            sheets: m.sheets().len(),
            viewports: m.viewports().count(),
            view,
            level_index: levels
                .iter()
                .position(|l| l.1 == self.ed.level)
                .unwrap_or(0),
            levels_open: self.show_levels,
            nav_project: self.nav_tab == super::navigator::NavTab::Project,
            wall_rectangle: self.wall_tool.mode == crate::wall_tool::WallMode::Rectangle,
            slab_wand: self.slab_settings.wand,
        }
    }

    pub(super) fn start_mission(&mut self, i: usize) {
        let now = self.observe();
        self.missions.progress = Some(Progress::begin(i, &now));
        self.missions.window_open = false;
        self.prefs.welcomed = true;
    }

    /// Keeps the preferences: applies the size of the text and writes the
    /// file when something changed.
    pub(super) fn sync_prefs(&mut self, ctx: &egui::Context) {
        self.prefs.dark = self.dark;
        self.prefs.keys_autocad = self.keys_autocad;
        crate::units::set_unit(self.prefs.units);
        let zoom = self.prefs.text_size.zoom();
        if (zoom - self.applied_zoom).abs() > 1e-3 {
            ctx.set_zoom_factor(zoom);
            self.applied_zoom = zoom;
        }
        if self.prefs != self.saved_prefs {
            self.prefs.save();
            self.saved_prefs = self.prefs.clone();
        }
    }

    /// End of the frame: moves the mission on, rings its control and
    /// draws the cards and the window.
    pub(super) fn missions_frame(&mut self, ctx: &egui::Context) {
        let now = self.observe();
        if let Some(p) = &mut self.missions.progress {
            if p.advance(&now) {
                let id = p.mission().id.to_string();
                if !self.prefs.missions_done.contains(&id) {
                    self.prefs.missions_done.push(id);
                }
            }
        }
        if let Some(target) = self
            .missions
            .progress
            .as_ref()
            .and_then(|p| p.current())
            .map(|s| s.target)
        {
            if let Some(rect) = ring_target(ctx, target) {
                paint_ring(ctx, rect);
            }
        }
        self.mission_card(ctx);
        self.missions_window(ctx);
    }

    fn card_area(&self, ctx: &egui::Context, id: &str, add: impl FnOnce(&mut egui::Ui)) {
        let p = Palette::current(ctx);
        let right = if self.show_navigator {
            self.side_width
        } else {
            0.0
        };
        egui::Area::new(Id::unique(id))
            .order(egui::Order::Foreground)
            .anchor(Align2::RIGHT_BOTTOM, Vec2::new(-(right + 16.0), -96.0))
            .show(ctx, |ui| {
                egui::Frame::new()
                    .fill(p.panel)
                    .stroke(Stroke::new(1.5, p.accent))
                    .corner_radius(CornerRadius::same(10))
                    .inner_margin(egui::Margin::same(14))
                    .shadow(egui::epaint::Shadow {
                        offset: [0, 4],
                        blur: 16,
                        spread: 0,
                        color: Color32::from_black_alpha(40),
                    })
                    .show(ui, |ui| {
                        ui.set_width(320.0);
                        add(ui)
                    });
            });
    }

    fn mission_card(&mut self, ctx: &egui::Context) {
        let p = Palette::current(ctx);
        if self.missions.progress.is_none() {
            // The start screen offers them too.
            if self.prefs.welcomed || self.start.open {
                return;
            }
            // The first time: offer the missions, and a bigger text.
            let mut start = false;
            let mut later = false;
            let mut size = self.prefs.text_size;
            self.card_area(ctx, "mission-welcome", |ui| {
                ui.label(
                    egui::RichText::new("¿Primera vez con ArchiSim?")
                        .font(strong_font(15.0))
                        .color(p.text),
                );
                ui.add_space(4.0);
                ui.label(format!(
                    "{} misiones cortas te enseñan dónde está cada cosa: muros, puertas y ventanas, forjados, el 3D, las plantas y los planos.",
                    MISSIONS.len()
                ));
                ui.add_space(8.0);
                text_size_row(ui, &mut size);
                ui.add_space(10.0);
                ui.horizontal(|ui| {
                    if ui
                        .add(egui::Button::new(
                            egui::RichText::new("Empezar").color(Color32::WHITE),
                        )
                        .fill(p.accent))
                        .clicked()
                    {
                        start = true;
                    }
                    if ui.button("Ahora no").clicked() {
                        later = true;
                    }
                });
                ui.label(
                    egui::RichText::new("Siempre están en Ayuda → Misiones de iniciación.")
                        .small()
                        .color(p.muted),
                );
            });
            self.prefs.text_size = size;
            if start {
                if let Some(i) = missions::next_mission(&self.prefs.missions_done, None) {
                    self.start_mission(i);
                }
            } else if later {
                self.prefs.welcomed = true;
            }
            return;
        }
        let Some(progress) = self.missions.progress.clone() else {
            return;
        };
        let m = progress.mission();
        let mut skip = false;
        let mut close = false;
        let mut next = None;
        self.card_area(ctx, "mission-card", |ui| {
            let n = progress.mission + 1;
            if let Some(step) = progress.current() {
                ui.label(
                    egui::RichText::new(format!(
                        "MISIÓN {n} DE {} · PASO {} DE {}",
                        MISSIONS.len(),
                        progress.step + 1,
                        m.steps.len()
                    ))
                    .small()
                    .color(p.muted),
                );
                ui.label(
                    egui::RichText::new(m.title)
                        .font(strong_font(15.0))
                        .color(p.text),
                );
                ui.add_space(4.0);
                ui.label(egui::RichText::new(step.say).color(p.text));
                ui.add_space(6.0);
                steps_dots(ui, progress.step, m.steps.len(), &p);
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    if ui
                        .small_button("Saltar paso")
                        .on_hover_text("Si ya sabes hacerlo o no puedes ahora")
                        .clicked()
                    {
                        skip = true;
                    }
                    if ui.small_button("Dejar la misión").clicked() {
                        close = true;
                    }
                });
            } else {
                ui.label(
                    egui::RichText::new(format!("MISIÓN {n} DE {} · HECHA", MISSIONS.len()))
                        .small()
                        .color(p.accent),
                );
                ui.label(
                    egui::RichText::new(format!("✓ {}", m.title))
                        .font(strong_font(15.0))
                        .color(p.text),
                );
                ui.add_space(4.0);
                ui.label(egui::RichText::new(m.then).color(p.text));
                ui.add_space(10.0);
                let following =
                    missions::next_mission(&self.prefs.missions_done, Some(progress.mission));
                ui.horizontal(|ui| {
                    if let Some(f) = following {
                        if ui
                            .add(
                                egui::Button::new(
                                    egui::RichText::new(format!(
                                        "Siguiente: {}",
                                        MISSIONS[f].title
                                    ))
                                    .color(Color32::WHITE),
                                )
                                .fill(p.accent),
                            )
                            .clicked()
                        {
                            next = Some(f);
                        }
                    }
                    if ui.button("Cerrar").clicked() {
                        close = true;
                    }
                });
                if following.is_none() {
                    ui.label(
                        egui::RichText::new("Has hecho todas las misiones.")
                            .small()
                            .color(p.muted),
                    );
                }
            }
        });
        if skip {
            let now = self.observe();
            if let Some(pr) = &mut self.missions.progress {
                pr.skip(&now);
                pr.advance(&now);
            }
        }
        if let Some(f) = next {
            self.start_mission(f);
        } else if close {
            self.missions.progress = None;
        }
        if self.missions.progress.is_some() {
            ctx.request_repaint_after(std::time::Duration::from_millis(60));
        }
    }

    fn missions_window(&mut self, ctx: &egui::Context) {
        if !self.missions.window_open {
            return;
        }
        let p = Palette::current(ctx);
        let mut open = true;
        let mut start = None;
        let mut size = self.prefs.text_size;
        let mut reset = false;
        egui::Window::new("Misiones de iniciación")
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .default_width(420.0)
            .show(ctx, |ui| {
                ui.label("Tareas cortas que enseñan dónde está cada cosa. Cada paso se marca solo al hacerlo; el control que hay que usar se ve rodeado en azul.");
                ui.add_space(8.0);
                for (i, m) in MISSIONS.iter().enumerate() {
                    let done = self.prefs.missions_done.iter().any(|d| d == m.id);
                    ui.horizontal(|ui| {
                        ui.label(
                            egui::RichText::new(if done { "✓" } else { "○" })
                                .font(strong_font(14.0))
                                .color(if done { p.accent } else { p.muted }),
                        );
                        ui.vertical(|ui| {
                            ui.label(egui::RichText::new(m.title).font(strong_font(13.0)).color(p.text));
                            ui.label(egui::RichText::new(m.about).small().color(p.muted));
                        });
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui.button(if done { "Repetir" } else { "Empezar" }).clicked() {
                                start = Some(i);
                            }
                        });
                    });
                    ui.add_space(4.0);
                }
                ui.separator();
                text_size_row(ui, &mut size);
                if !self.prefs.missions_done.is_empty()
                    && ui.small_button("Empezar de cero").on_hover_text("Quita las marcas de hecho").clicked()
                {
                    reset = true;
                }
            });
        self.prefs.text_size = size;
        if reset {
            self.prefs.missions_done.clear();
        }
        self.missions.window_open = open;
        if let Some(i) = start {
            self.start_mission(i);
        }
    }
}

/// Metros · Centímetros · Milímetros.
pub(super) fn units_row(ui: &mut egui::Ui, units: &mut crate::units::LengthUnit) {
    ui.horizontal(|ui| {
        ui.label("Unidades");
        for u in crate::units::LengthUnit::ALL {
            ui.selectable_value(units, u, u.name());
        }
    })
    .response
    .on_hover_text(
        "En las que se ven y se escriben las medidas (también se puede escribir 270cm o 2700mm)",
    );
}

/// Normal · Grande · Muy grande.
pub(super) fn text_size_row(ui: &mut egui::Ui, size: &mut TextSize) {
    ui.horizontal(|ui| {
        ui.label("Tamaño de letra");
        for t in TextSize::ALL {
            ui.selectable_value(size, t, t.name());
        }
    });
}

fn steps_dots(ui: &mut egui::Ui, at: usize, n: usize, p: &Palette) {
    let (rect, _) = ui.allocate_exact_size(Vec2::new(n as f32 * 14.0, 8.0), egui::Sense::hover());
    for i in 0..n {
        let c = rect.left_center() + Vec2::new(4.0 + i as f32 * 14.0, 0.0);
        if i < at {
            ui.painter().circle_filled(c, 4.0, p.accent);
        } else if i == at {
            ui.painter()
                .circle_stroke(c, 4.0, Stroke::new(1.5, p.accent));
        } else {
            ui.painter().circle_stroke(c, 4.0, Stroke::new(1.0, p.line));
        }
    }
}

/// A pulsing ring around the control to use.
fn paint_ring(ctx: &egui::Context, rect: Rect) {
    let p = Palette::current(ctx);
    let t = ctx.input(|i| i.time);
    let pulse = (0.5 + 0.5 * (t * 4.0).sin()) as f32;
    let painter = ctx.layer_painter(egui::LayerId::new(
        egui::Order::Tooltip,
        Id::unique("mission-ring"),
    ));
    let r = rect.expand(3.0 + 2.0 * pulse);
    painter.rect_stroke(
        r,
        CornerRadius::same(8),
        Stroke::new(2.5, p.accent.gamma_multiply(0.55 + 0.45 * pulse)),
        egui::StrokeKind::Outside,
    );
}
