//! «Descargar el entorno»: what «Traer el entorno» reads from files, asked
//! straight to the cadastre, OpenStreetMap and the IGN for the project's
//! georeference and a radius (`archisim_library::download` gives the
//! addresses). It runs on a thread; when it ends, «Traer el entorno»
//! opens with what arrived, and says what did not.

use std::io::Read as _;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use archisim_library::carto::CartoOptions;
use archisim_library::download::{self, Request};

use super::carto_ui::CartoImport;
use super::length_field;
use super::theme::Palette;
use super::widgets::SwitchExt as _;
use super::ArchiSimApp;
use eframe::egui;

/// The most a service may send, bytes.
const MAX_ANSWER: u64 = 400 * 1024 * 1024;

/// How a download goes.
#[derive(Debug, Default)]
pub struct Progress {
    pub done: usize,
    pub total: usize,
    /// What is being asked now.
    pub current: String,
    /// What arrived: name and text.
    pub files: Vec<(String, String)>,
    /// The orthophoto, if it arrived: name, PNG and the map rectangle.
    pub picture: Option<(String, Vec<u8>, [f64; 4])>,
    /// What did not: what it was, why, and its address.
    pub failures: Vec<(String, String, String)>,
    pub finished: bool,
    /// Asked to stop.
    pub stop: bool,
}

/// The window: what to bring, and the download while it runs.
pub struct CartoDownload {
    pub options: CartoOptions,
    pub job: Option<Arc<Mutex<Progress>>>,
    /// Why the last download brought nothing: what, why and its address.
    pub failures: Vec<(String, String, String)>,
}

impl Default for CartoDownload {
    fn default() -> Self {
        CartoDownload {
            options: CartoOptions {
                radius: 200.0,
                ..CartoOptions::default()
            },
            job: None,
            failures: Vec::new(),
        }
    }
}

/// Fetches the addresses one after the other with `fetch`, writing how it
/// goes in `progress` (the application's fetch is `http_get`; the tests
/// give their own).
pub fn run(
    requests: &[Request],
    progress: &Mutex<Progress>,
    fetch: impl Fn(&str, Option<&str>) -> Result<Vec<u8>, String>,
) {
    if let Ok(mut p) = progress.lock() {
        p.total = requests.len();
    }
    // The tiles of heights that arrived, to make the terrain at the end.
    let mut heights: std::collections::HashMap<(u32, u32), archisim_geom::decals::Pixels> =
        Default::default();
    let mut terrain = None;
    for r in requests {
        if let Ok(mut p) = progress.lock() {
            if p.stop {
                break;
            }
            p.current = r.source.name().to_string();
        }
        // The address, or its attempts in turn until one answers (each
        // one's failure kept, to say them all).
        let got = if r.attempts.is_empty() {
            fetch(&r.url, None)
        } else {
            let mut why = Vec::new();
            let mut got = None;
            for (url, form) in &r.attempts {
                match fetch(url, form.as_deref()).and_then(|b| {
                    download::answer_text(r.source, &String::from_utf8_lossy(&b)).map(|_| b)
                }) {
                    Ok(b) => {
                        got = Some(b);
                        break;
                    }
                    Err(e) => why.push(format!("{}: {e}", host_of(url))),
                }
            }
            got.ok_or_else(|| why.join("; "))
        };
        let Ok(mut p) = progress.lock() else { return };
        if let Some(t) = r.tile {
            terrain = Some(t);
            // A tile out of Spain or at sea is not there: no harm.
            if let Some(px) = got
                .as_deref()
                .ok()
                .and_then(archisim_geom::decals::decode_png)
            {
                heights.insert((t.x, t.y), px);
            }
            p.done += 1;
            continue;
        }
        let got = match (got, r.source, r.bbox) {
            (Ok(b), download::Source::Orthophoto, Some(bbox)) if download::is_png(&b) => {
                p.picture = Some((r.name.clone(), b, bbox));
                p.done += 1;
                continue;
            }
            (Ok(b), _, _) => download::answer_text(r.source, &String::from_utf8_lossy(&b))
                .and_then(|t| {
                    if r.source == download::Source::Orthophoto {
                        Err("no ha llegado una imagen PNG".into())
                    } else {
                        Ok(t)
                    }
                }),
            (Err(e), _, _) => Err(e),
        };
        match got {
            Ok(text) => p.files.push((r.name.clone(), text)),
            Err(e) => p
                .failures
                .push((r.source.name().to_string(), e, r.url.clone())),
        }
        p.done += 1;
    }
    // The terrain, from its tiles of heights.
    if let Some(t) = terrain {
        let grid = download::terrain_grid(&t, |x, y, u, v| {
            let px = heights.get(&(x, y))?;
            let c = px.at(u, 1.0 - v);
            (c[3] > 0).then_some([c[0], c[1], c[2]])
        });
        if let Ok(mut p) = progress.lock() {
            match grid {
                Some(text) => p.files.push(("ign-mdt.asc".into(), text)),
                None => p.failures.push((
                    download::Source::Terrain.name().to_string(),
                    "no han llegado las alturas".into(),
                    format!("{}/{}/{}/{}.png", download::IGN_HEIGHTS, t.z, t.x, t.y),
                )),
            }
        }
    }
    if let Ok(mut p) = progress.lock() {
        p.finished = true;
    }
}

/// The server of an address, to say which one failed.
fn host_of(url: &str) -> &str {
    let rest = url.split_once("://").map_or(url, |(_, r)| r);
    rest.split('/').next().unwrap_or(rest)
}

/// One address, by HTTP (through the system's proxy if it has one): a GET,
/// or a POST of a form.
pub fn http_get(url: &str, form: Option<&str>) -> Result<Vec<u8>, String> {
    let agent = ureq::AgentBuilder::new()
        .timeout_connect(Duration::from_secs(20))
        .timeout(Duration::from_secs(240))
        .user_agent(concat!(
            "ArchiSim/",
            env!("CARGO_PKG_VERSION"),
            " (+https://github.com/stronkel/archisim)"
        ))
        .try_proxy_from_env(true)
        .build();
    let res = match form {
        Some(f) => agent
            .post(url)
            .set("Content-Type", "application/x-www-form-urlencoded")
            .send_string(f),
        None => agent.get(url).call(),
    };
    let res = res.map_err(|e| match e {
        ureq::Error::Status(code, r) => {
            let body = r.into_string().unwrap_or_default();
            let body: String = body.chars().take(120).collect();
            format!("el servicio responde {code} {}", body.trim())
        }
        ureq::Error::Transport(t) => format!("no se llega al servicio ({})", t.kind()),
    })?;
    let mut bytes = Vec::new();
    res.into_reader()
        .take(MAX_ANSWER)
        .read_to_end(&mut bytes)
        .map_err(|e| format!("la respuesta se ha cortado ({e})"))?;
    Ok(bytes)
}

impl ArchiSimApp {
    /// Opens the window (Archivo → Descargar el entorno).
    pub(super) fn open_carto_download(&mut self) {
        if self.carto_download.is_none() {
            self.carto_download = Some(CartoDownload::default());
        }
    }

    /// Starts the download of what the window asks.
    pub fn start_carto_download(&mut self) -> Result<(), String> {
        let Some(d) = &mut self.carto_download else {
            return Err("no hay nada que descargar".into());
        };
        let requests = download::plan(self.ed.model(), &d.options)?;
        d.failures.clear();
        let job = Arc::new(Mutex::new(Progress::default()));
        d.job = Some(job.clone());
        std::thread::spawn(move || run(&requests, &job, http_get));
        Ok(())
    }

    /// When the download has ended, «Traer el entorno» with what arrived.
    pub fn finish_carto_download(&mut self) {
        let Some(d) = &self.carto_download else {
            return;
        };
        let Some(job) = &d.job else { return };
        let Ok(mut p) = job.lock() else { return };
        if !p.finished {
            return;
        }
        let files = std::mem::take(&mut p.files);
        let orthophoto = p.picture.take();
        let failures = p.failures.clone();
        let notes: Vec<String> = failures
            .iter()
            .map(|(what, why, url)| format!("{what}: {why}. Dirección: {url}"))
            .collect();
        let stopped = p.stop;
        drop(p);
        let options = d.options.clone();
        if files.is_empty() && orthophoto.is_none() {
            // The window stays, saying why.
            if let Some(d) = &mut self.carto_download {
                d.job = None;
                d.failures = if stopped { Vec::new() } else { failures };
            }
            return;
        }
        self.carto_download = None;
        self.carto_import = Some(CartoImport {
            files,
            options,
            notes,
            orthophoto,
        });
    }

    pub(super) fn carto_download_dialog(&mut self, ctx: &egui::Context) {
        self.finish_carto_download();
        let Some(mut d) = self.carto_download.take() else {
            return;
        };
        let mut open = true;
        let mut start = false;
        let georef = self.ed.model().georeference().cloned();
        let running = d.job.clone();
        super::widgets::dialog("Descargar el entorno")
            .open(&mut open)
            .default_width(520.0)
            .show(ctx, |ui| {
                let p = Palette::current(ui.ctx());
                if let Some(job) = &running {
                    let (done, total, current) = job
                        .lock()
                        .map(|p| (p.done, p.total, p.current.clone()))
                        .unwrap_or_default();
                    ui.label(format!("Descargando… {current}"));
                    ui.add(
                        egui::ProgressBar::new(if total == 0 {
                            0.0
                        } else {
                            done as f32 / total as f32
                        })
                        .text(format!("{done} de {total}")),
                    );
                    if ui.button("Parar").clicked() {
                        if let Ok(mut p) = job.lock() {
                            p.stop = true;
                        }
                    }
                    ctx.request_repaint_after(Duration::from_millis(200));
                    return;
                }
                match &georef {
                    Some(g) => ui.label(
                        egui::RichText::new(format!(
                            "Alrededor del origen del proyecto ({}, X {:.0} · Y {:.0}).",
                            g.crs, g.eastings, g.northings
                        ))
                        .color(p.muted),
                    ),
                    None => ui.label(
                        egui::RichText::new("El proyecto no está georreferenciado: pon antes su emplazamiento en las propiedades del proyecto («Emplazamiento (georreferencia)», sin nada seleccionado).")
                            .color(p.bad),
                    ),
                };
                ui.add_space(4.0);
                let o = &mut d.options;
                ui.horizontal_wrapped(|ui| {
                    ui.switch(&mut o.buildings, "Edificios (Catastro)");
                    ui.switch(&mut o.parcels, "Parcelas (Catastro)");
                    ui.switch(&mut o.streets, "Calles (OpenStreetMap)");
                    ui.switch(&mut o.trees, "Árboles (OpenStreetMap)");
                    ui.switch(&mut o.terrain, "Terreno (IGN, MDT05)");
                    ui.switch(&mut o.orthophoto, "Ortofoto (PNOA)");
                    ui.switch(&mut o.street_life, "Vida en la calle");
                });
                ui.horizontal(|ui| {
                    ui.label("Radio alrededor del origen");
                    if let Some(v) =
                        length_field(ui, egui::Id::unique("carto-download-radius"), o.radius, 90.0)
                    {
                        o.radius = v.clamp(10.0, 2000.0);
                    }
                });
                ui.label(
                    egui::RichText::new("Son servicios públicos y gratuitos; con radios grandes tardan unos minutos. Al terminar se abre «Traer el entorno» para elegir cómo se trae.")
                        .color(p.muted),
                );
                if !d.failures.is_empty() {
                    ui.add_space(4.0);
                    ui.label(egui::RichText::new("No ha llegado nada:").color(p.bad));
                    egui::ScrollArea::vertical()
                        .max_height(220.0)
                        .show(ui, |ui| {
                            for (what, why, url) in &d.failures {
                                ui.horizontal_wrapped(|ui| {
                                    ui.label(egui::RichText::new(what).strong());
                                    ui.label(egui::RichText::new(why).color(p.muted));
                                    ui.hyperlink_to("Abrir la dirección", url);
                                });
                            }
                        });
                    ui.label(
                        egui::RichText::new("Prueba las direcciones en el navegador, o descarga los ficheros a mano y usa «Traer el entorno».")
                            .color(p.muted),
                    );
                }
                ui.add_space(6.0);
                if ui
                    .add_enabled(georef.is_some(), egui::Button::new("Descargar"))
                    .clicked()
                {
                    start = true;
                }
            });
        self.carto_download = open.then_some(d);
        if !open {
            // Closing the window stops the download.
            if let Some(job) = running {
                if let Ok(mut p) = job.lock() {
                    p.stop = true;
                }
            }
        }
        if start {
            if let Err(e) = self.start_carto_download() {
                self.complain(format!("No se puede descargar el entorno: {e}."));
            }
        }
    }
}
