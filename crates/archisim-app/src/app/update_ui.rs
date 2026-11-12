//! Updating ArchiSim from itself (`docs/ACTUALIZAR.md`): it asks GitHub
//! for the latest version when it opens (or from Ayuda → «Buscar
//! actualizaciones…»), says what it changes, downloads its installer with
//! a bar, and puts it in place, opening again with the same project. The
//! logic is in `crate::updates`.

use std::io::Read;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use eframe::egui::{self, Ui};

use super::*;
use crate::updates::{self, Release};

/// What GitHub answers, when it does.
type Answer = Arc<Mutex<Option<Result<Release, String>>>>;

/// The window and what it is doing.
#[derive(Default)]
pub struct UpdateState {
    pub open: bool,
    /// The look at GitHub has been started (once per session on its own).
    asked: bool,
    /// Started on its own when opening: only a note if there is something.
    quiet: bool,
    check: Option<Answer>,
    found: Option<Release>,
    error: Option<String>,
    download: Option<Arc<Mutex<Download>>>,
    /// The note of a new version, put away for this session.
    hide_note: bool,
}

#[derive(Default)]
struct Download {
    got: u64,
    total: u64,
    done: Option<Result<PathBuf, String>>,
}

/// Downloads a file to `to`, telling how much has come.
fn fetch_file(url: &str, to: &PathBuf, progress: &Arc<Mutex<Download>>) -> Result<(), String> {
    let agent = ureq::AgentBuilder::new()
        .timeout_connect(Duration::from_secs(20))
        .user_agent(concat!("ArchiSim/", env!("CARGO_PKG_VERSION")))
        .try_proxy_from_env(true)
        .build();
    let res = agent
        .get(url)
        .call()
        .map_err(|e| format!("no se puede descargar ({e})"))?;
    if let Some(n) = res.header("Content-Length").and_then(|n| n.parse().ok()) {
        if let Ok(mut p) = progress.lock() {
            p.total = n;
        }
    }
    let mut reader = res.into_reader();
    let tmp = to.with_extension("parte");
    let mut file = std::fs::File::create(&tmp).map_err(|e| e.to_string())?;
    let mut buf = vec![0u8; 1 << 16];
    loop {
        let n = reader
            .read(&mut buf)
            .map_err(|e| format!("la descarga se ha cortado ({e})"))?;
        if n == 0 {
            break;
        }
        std::io::Write::write_all(&mut file, &buf[..n]).map_err(|e| e.to_string())?;
        if let Ok(mut p) = progress.lock() {
            p.got += n as u64;
        }
    }
    drop(file);
    std::fs::rename(&tmp, to).map_err(|e| e.to_string())
}

impl ArchiSimApp {
    /// Asks GitHub for the latest version, in a thread of its own.
    pub(super) fn check_for_updates(&mut self, quiet: bool) {
        let slot = Arc::new(Mutex::new(None));
        let out = slot.clone();
        std::thread::spawn(move || {
            let answer = super::carto_download_ui::http_get(updates::LATEST_URL, None)
                .map_err(|e| format!("no se ha podido preguntar a GitHub: {e}"))
                .and_then(|b| updates::read_release(&String::from_utf8_lossy(&b)));
            if let Ok(mut s) = out.lock() {
                *s = Some(answer);
            }
        });
        let u = &mut self.update;
        u.asked = true;
        u.quiet = quiet;
        u.check = Some(slot);
        u.error = None;
    }

    /// Every frame: the look when opening, its answer, the note of a new
    /// version and the window.
    pub(super) fn update_frame(&mut self, ctx: &egui::Context) {
        if !self.update.asked && self.prefs.check_updates && !cfg!(test) {
            self.check_for_updates(true);
        }
        let answer = self
            .update
            .check
            .as_ref()
            .and_then(|c| c.lock().ok().and_then(|mut s| s.take()));
        if let Some(a) = answer {
            self.update.check = None;
            match a {
                Ok(r) => self.update.found = Some(r),
                Err(e) => {
                    // When opening, without internet, nothing is said.
                    if !self.update.quiet {
                        self.update.error = Some(e);
                    }
                }
            }
        }
        if self.update.check.is_some() || self.update.download.is_some() {
            ctx.request_repaint_after(Duration::from_millis(200));
        }
        self.update_note(ctx);
        if self.update.open {
            let mut open = true;
            super::widgets::dialog("Actualizar ArchiSim")
                .open(&mut open)
                .default_width(560.0)
                .show(ctx, |ui| self.update_contents(ui));
            self.update.open = open && self.update.open;
        }
    }

    /// The version found, if it is newer than this one.
    fn newer(&self) -> Option<&Release> {
        self.update
            .found
            .as_ref()
            .filter(|r| updates::is_newer(&r.version, updates::CURRENT))
    }

    /// A small note in a corner when a new version is out.
    fn update_note(&mut self, ctx: &egui::Context) {
        if self.update.open || self.update.hide_note {
            return;
        }
        let Some(r) = self.newer() else {
            return;
        };
        if self.prefs.skipped_version.as_deref() == Some(r.version.as_str()) {
            return;
        }
        let text = format!("Hay una versión nueva de ArchiSim: la {}", r.version);
        let p = super::theme::Palette::current(ctx);
        let mut see = false;
        let mut later = false;
        egui::Area::new(egui::Id::unique("update-note"))
            .anchor(egui::Align2::RIGHT_BOTTOM, egui::vec2(-16.0, -48.0))
            .order(egui::Order::Foreground)
            .show(ctx, |ui| {
                egui::Frame::NONE
                    .fill(p.panel)
                    .stroke(egui::Stroke::new(1.0, p.accent))
                    .corner_radius(egui::CornerRadius::same(8))
                    .inner_margin(egui::Margin::same(10))
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.label(egui::RichText::new(text).strong());
                            see = ui.button("Ver novedades y actualizar").clicked();
                            later = ui.button("Más tarde").clicked();
                        });
                    });
            });
        if see {
            self.update.open = true;
        }
        if later {
            self.update.hide_note = true;
        }
    }

    fn update_contents(&mut self, ui: &mut Ui) {
        ui.label(format!("Tienes la versión {}.", updates::CURRENT));
        if self.update.check.is_some() {
            ui.horizontal(|ui| {
                ui.spinner();
                ui.label("Preguntando a GitHub por la última versión…");
            });
        }
        if let Some(e) = self.update.error.clone() {
            ui.colored_label(ui.visuals().warn_fg_color, e);
        }
        let mut save = false;
        let mut install: Option<PathBuf> = None;
        let mut download: Option<(String, String)> = None;
        let mut skip = None;
        match (self.newer().cloned(), self.update.found.is_some()) {
            (None, true) => {
                ui.label("Es la última: no hay nada que actualizar.");
            }
            (None, false) => {}
            (Some(r), _) => {
                ui.add_space(4.0);
                ui.label(
                    egui::RichText::new(format!("Hay una versión nueva: la {}", r.version))
                        .strong()
                        .size(15.0),
                );
                ui.add_space(4.0);
                ui.label(egui::RichText::new("Qué cambia").strong());
                egui::ScrollArea::vertical()
                    .max_height(260.0)
                    .auto_shrink([false, true])
                    .show(ui, |ui| {
                        let text = updates::changes(&r.notes);
                        if text.is_empty() {
                            ui.label("La página de la versión no lo dice.");
                        } else {
                            ui.label(text);
                        }
                    });
                if !r.page.is_empty() {
                    ui.hyperlink_to("La versión en GitHub", &r.page);
                }
                ui.add_space(6.0);
                let progress = self
                    .update
                    .download
                    .as_ref()
                    .and_then(|d| d.lock().ok().map(|d| (d.got, d.total, d.done.clone())));
                match (r.installer.clone(), progress) {
                    (None, _) => {
                        ui.label("En este sistema, ArchiSim se actualiza compilándolo.");
                    }
                    (Some((url, size)), None) => {
                        if ui
                            .button(format!("Descargar ({:.0} MB)", size as f64 / 1_000_000.0))
                            .clicked()
                        {
                            download = Some((url, r.version.clone()));
                        }
                    }
                    (Some(_), Some((got, total, None))) => {
                        let f = if total > 0 {
                            got as f32 / total as f32
                        } else {
                            0.0
                        };
                        ui.add(
                            egui::ProgressBar::new(f)
                                .text(format!("Descargando… {:.0} MB", got as f64 / 1e6)),
                        );
                    }
                    (Some(_), Some((_, _, Some(Err(e))))) => {
                        ui.colored_label(ui.visuals().warn_fg_color, e);
                        if ui.button("Volver a intentarlo").clicked() {
                            self.update.download = None;
                        }
                    }
                    (Some(_), Some((_, _, Some(Ok(path))))) => {
                        if self.ed.is_modified() {
                            ui.label("Guarda el proyecto antes de actualizar: ArchiSim se cerrará y volverá a abrirse con él.");
                            save = ui.button("Guardar").clicked();
                        } else {
                            ui.label("Descargada. ArchiSim se cerrará, se instalará la nueva y volverá a abrirse con este proyecto.");
                            if ui
                                .add(egui::Button::new(
                                    egui::RichText::new("Instalar y volver a abrir").strong(),
                                ))
                                .clicked()
                            {
                                install = Some(path);
                            }
                        }
                    }
                }
                ui.add_space(4.0);
                if ui
                    .small_button("No volver a avisarme de esta versión")
                    .clicked()
                {
                    skip = Some(r.version.clone());
                }
            }
        }
        ui.separator();
        ui.horizontal(|ui| {
            if ui
                .checkbox(
                    &mut self.prefs.check_updates,
                    "Buscar actualizaciones al abrir ArchiSim",
                )
                .changed()
            {
                self.prefs.save();
            }
            if self.update.check.is_none() && ui.button("Buscar ahora").clicked() {
                self.check_for_updates(false);
            }
        });
        if let Some(v) = skip {
            self.prefs.skipped_version = Some(v);
            self.prefs.save();
            self.update.open = false;
        }
        if save {
            self.save(false);
        }
        if let Some((url, version)) = download {
            let state = Arc::new(Mutex::new(Download::default()));
            let s = state.clone();
            let to = updates::download_path(&version);
            std::thread::spawn(move || {
                let done = fetch_file(&url, &to, &s).map(|()| to);
                if let Ok(mut d) = s.lock() {
                    d.done = Some(done);
                }
            });
            self.update.download = Some(state);
        }
        if let Some(path) = install {
            let project = self.ed.path.clone();
            match updates::start_install(&path, project.as_deref()) {
                Ok(()) => {
                    self.allow_close = true;
                    ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
                }
                Err(e) => self.update.error = Some(format!("No se puede instalar: {e}.")),
            }
        }
    }
}
