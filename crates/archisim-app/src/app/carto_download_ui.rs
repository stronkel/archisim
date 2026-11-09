//! «Descargar el entorno»: what «Traer el entorno» reads from files, asked
//! straight to the cadastre, OpenStreetMap and the IGN for the project's
//! georeference and a radius (`archisim_library::download` gives the
//! addresses). It runs on a thread; when it ends, «Traer el entorno»
//! opens with what arrived, and says what did not.
//!
//! The place is looked up by an address, a site, a municipality, a
//! cadastral reference or coordinates (`archisim_library::geocode`, on
//! another thread): the project without a georeference takes one with its
//! origin there; the one that has it brings the surroundings round the
//! place, or round its origin. A little orthophoto shows where and how
//! far.

use std::io::Read as _;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use archisim_library::carto::CartoOptions;
use archisim_library::download::{self, Request};
use archisim_library::geocode::{self, Place};

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

/// An answer that a thread is waiting for: none until it arrives.
pub type Pending<T> = Arc<Mutex<Option<Result<T, String>>>>;

/// How an address is asked: a GET, or a POST of a form.
pub type Fetch = fn(&str, Option<&str>) -> Result<Vec<u8>, String>;

/// The little map of the window: the orthophoto round a point.
pub struct Preview {
    /// Its centre, latitude and longitude.
    pub centre: (f64, f64),
    /// Half its side, metres.
    pub half: f64,
    pub bbox: download::Bbox,
    pub job: Pending<Vec<u8>>,
    pub texture: Option<egui::TextureHandle>,
    /// Why there is no picture.
    pub failed: Option<String>,
}

/// The window: what to bring, and the download while it runs.
pub struct CartoDownload {
    pub options: CartoOptions,
    pub job: Option<Arc<Mutex<Progress>>>,
    /// Why the last download brought nothing: what, why and its address.
    pub failures: Vec<(String, String, String)>,
    /// What is written in the field of the place.
    pub query: String,
    /// The search while it runs.
    pub search: Option<Pending<Vec<Place>>>,
    /// What it found, and why it found nothing.
    pub results: Vec<Place>,
    pub search_error: Option<String>,
    /// The place chosen.
    pub chosen: Option<Place>,
    /// With a georeferenced project: bring round the chosen place (true)
    /// or round the project's origin.
    pub around_place: bool,
    pub preview: Option<Preview>,
    /// How addresses are asked (the tests give their own).
    pub fetch: Fetch,
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
            query: String::new(),
            search: None,
            results: Vec::new(),
            search_error: None,
            chosen: None,
            around_place: true,
            preview: None,
            fetch: default_fetch(),
        }
    }
}

/// The application's fetch: by HTTP; in a debug build with
/// `ARCHISIM_BUSQUEDA_EJEMPLO=carpeta`, answers read from that folder
/// (`candidatos.json`, `find.json`, `nominatim.json`, `catastro.xml`,
/// `pnoa.png`), to see the window without network.
fn default_fetch() -> Fetch {
    #[cfg(debug_assertions)]
    if std::env::var_os("ARCHISIM_BUSQUEDA_EJEMPLO").is_some() {
        return sample_fetch;
    }
    if cfg!(test) {
        return |_, _| Err("sin red en las pruebas".into());
    }
    http_get
}

#[cfg(debug_assertions)]
fn sample_fetch(url: &str, _form: Option<&str>) -> Result<Vec<u8>, String> {
    let dir =
        std::path::PathBuf::from(std::env::var_os("ARCHISIM_BUSQUEDA_EJEMPLO").unwrap_or_default());
    let file = if url.contains("/candidates") {
        "candidatos.json"
    } else if url.contains("/find") {
        "find.json"
    } else if url.contains("nominatim") {
        "nominatim.json"
    } else if url.contains("catastro") {
        "catastro.xml"
    } else if url.contains("pnoa") {
        "pnoa.png"
    } else {
        return Err("no se llega al servicio (ejemplo sin red)".into());
    };
    std::fs::read(dir.join(file)).map_err(|_| "no se llega al servicio (ejemplo sin red)".into())
}

/// When Nominatim was last asked: its policy is one request a second.
static NOMINATIM_LAST: Mutex<Option<std::time::Instant>> = Mutex::new(None);

/// `fetch` for the search: a GET, waiting so that Nominatim is asked no
/// more than once a second.
fn paced(fetch: Fetch, url: &str) -> Result<Vec<u8>, String> {
    if url.starts_with(geocode::NOMINATIM) {
        if let Ok(mut last) = NOMINATIM_LAST.lock() {
            if let Some(t) = *last {
                let gap = Duration::from_millis(1100);
                if t.elapsed() < gap {
                    std::thread::sleep(gap - t.elapsed());
                }
            }
            *last = Some(std::time::Instant::now());
        }
    }
    fetch(url, None)
}

/// Runs `job` on a thread, its answer into a [`Pending`].
fn spawn<T: Send + 'static>(
    job: impl FnOnce() -> Result<T, String> + Send + 'static,
) -> Pending<T> {
    let pending: Pending<T> = Arc::new(Mutex::new(None));
    let out = pending.clone();
    std::thread::spawn(move || {
        let r = job();
        if let Ok(mut p) = out.lock() {
            *p = Some(r);
        }
    });
    pending
}

/// Takes the answer of a [`Pending`] if it has arrived.
fn take<T>(p: &Pending<T>) -> Option<Result<T, String>> {
    p.lock().ok()?.take()
}

impl CartoDownload {
    /// Starts looking up what is written.
    pub fn start_search(&mut self) {
        // What can be said at once is said at once.
        if let Err(e) = geocode::read_query(&self.query) {
            self.search_error = Some(e);
            self.results.clear();
            return;
        }
        let (q, fetch) = (self.query.clone(), self.fetch);
        self.search_error = None;
        self.results.clear();
        self.search = Some(spawn(move || {
            geocode::search(&q, &mut |url| paced(fetch, url))
        }));
    }

    /// When the search has ended, its places (the only one, chosen).
    pub fn finish_search(&mut self) {
        let Some(job) = &self.search else { return };
        let Some(r) = take(job) else { return };
        self.search = None;
        match r {
            Ok(places) => {
                self.results = places;
                if self.results.len() == 1 {
                    self.choose(0);
                }
            }
            Err(e) => self.search_error = Some(e),
        }
    }

    /// Chooses one of the places found.
    pub fn choose(&mut self, i: usize) {
        if let Some(p) = self.results.get(i) {
            self.chosen = Some(p.clone());
            self.around_place = true;
        }
    }

    /// Latitude and longitude of the centre of what is brought, if known.
    pub fn centre_lat_lon(&self, m: &archisim_core::Model) -> Option<(f64, f64)> {
        match (&self.chosen, m.georeference().is_some()) {
            (Some(p), false) => p.lat_lon,
            (Some(p), true) if self.around_place => p.lat_lon,
            _ => geocode::lat_lon_of(m, archisim_core::Point2::new(0.0, 0.0)),
        }
    }

    /// Asks the little map again if its centre or the radius changed.
    fn refresh_preview(&mut self, centre: Option<(f64, f64)>) {
        let Some(c) = centre else {
            self.preview = None;
            return;
        };
        let half = ((self.options.radius * 1.3).max(100.0) / 50.0).ceil() * 50.0;
        if let Some(p) = &self.preview {
            let moved = (p.centre.0 - c.0).abs() > 1e-6 || (p.centre.1 - c.1).abs() > 1e-6;
            if !moved && half <= p.half && half * 2.0 > p.half {
                return;
            }
        }
        let (url, bbox) = geocode::preview(c.0, c.1, half);
        let fetch = self.fetch;
        self.preview = Some(Preview {
            centre: c,
            half,
            bbox,
            job: spawn(move || fetch(&url, None)),
            texture: None,
            failed: None,
        });
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

    /// Puts the chosen place in the project and the options: the project
    /// without a georeference takes one with its origin there (one step
    /// of Deshacer), its ±0,00 to go on the terrain; the one that has it
    /// brings round the place (it does not move) or round its origin.
    pub fn apply_chosen_place(&mut self) -> Result<(), String> {
        let Some(d) = &self.carto_download else {
            return Err("no hay nada que descargar".into());
        };
        let chosen = d.chosen.clone();
        let around_place = d.around_place;
        let mut centre = archisim_core::Point2::new(0.0, 0.0);
        let mut level_to_ground = false;
        match chosen {
            Some(place) if self.ed.model().georeference().is_none() => {
                let g = geocode::georeference_at(&place)
                    .ok_or("el lugar elegido no tiene coordenadas")?;
                self.ed
                    .transact("Emplazamiento desde la búsqueda", |m| {
                        m.set_georeference(Some(g))
                    })
                    .map_err(|e| e.to_string())?;
                level_to_ground = true;
            }
            Some(place) if around_place => {
                let (lat, lon) = place
                    .lat_lon
                    .ok_or("el lugar elegido no tiene coordenadas")?;
                centre = geocode::plan_point(self.ed.model(), lat, lon).ok_or(
                    "el emplazamiento del proyecto no está en UTM: no se sabe dónde cae el lugar",
                )?;
            }
            _ => {}
        }
        if let Some(d) = &mut self.carto_download {
            d.options.centre = centre;
            d.options.level_to_ground = level_to_ground;
        }
        Ok(())
    }

    /// Starts the download of what the window asks.
    pub fn start_carto_download(&mut self) -> Result<(), String> {
        self.apply_chosen_place()?;
        let Some(d) = &mut self.carto_download else {
            return Err("no hay nada que descargar".into());
        };
        let requests = download::plan(self.ed.model(), &d.options)?;
        d.failures.clear();
        let job = Arc::new(Mutex::new(Progress::default()));
        d.job = Some(job.clone());
        let fetch = d.fetch;
        std::thread::spawn(move || run(&requests, &job, fetch));
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
        d.finish_search();
        let centre = d.centre_lat_lon(self.ed.model());
        d.refresh_preview(centre);
        // Where the chosen place falls in the plan, and the origin.
        let model_origin = Origins {
            chosen_in_plan: d
                .chosen
                .as_ref()
                .and_then(|c| c.lat_lon)
                .and_then(|(lat, lon)| geocode::plan_point(self.ed.model(), lat, lon)),
            origin: geocode::lat_lon_of(self.ed.model(), archisim_core::Point2::new(0.0, 0.0)),
            centre,
        };
        if d.search.is_some()
            || d.preview
                .as_ref()
                .is_some_and(|p| p.texture.is_none() && p.failed.is_none())
        {
            ctx.request_repaint_after(Duration::from_millis(200));
        }
        let can_start = georef.is_some() || d.chosen.as_ref().is_some_and(|c| c.lat_lon.is_some());
        let running = d.job.clone();
        super::widgets::dialog("Descargar el entorno")
            .open(&mut open)
            .default_width(560.0)
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
                place_section(ui, &mut d, georef.as_ref(), &model_origin, &p);
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
                    ui.label("Radio");
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
                    .add_enabled(can_start, egui::Button::new("Descargar"))
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

/// Where things are, for the place's part of the window.
struct Origins {
    /// The chosen place in the project's plan (with a georeference).
    chosen_in_plan: Option<archisim_core::Point2>,
    /// The project's origin, latitude and longitude.
    origin: Option<(f64, f64)>,
    /// The centre of what is brought, latitude and longitude.
    centre: Option<(f64, f64)>,
}

/// The place: the field to look it up, what was found, the one chosen,
/// round what it is brought and the little map.
fn place_section(
    ui: &mut egui::Ui,
    d: &mut CartoDownload,
    georef: Option<&archisim_core::Georeference>,
    at: &Origins,
    p: &Palette,
) {
    ui.label(egui::RichText::new("Lugar").strong());
    let mut search = false;
    ui.horizontal(|ui| {
        let r = ui.add(
            egui::TextEdit::singleline(&mut d.query)
                .id(egui::Id::unique("carto-place-query"))
                .hint_text("Calle Ancha 12, Punta Umbría")
                .desired_width(ui.available_width() - 80.0),
        );
        if r.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
            search = true;
        }
        if ui
            .add_enabled(d.search.is_none(), egui::Button::new("Buscar"))
            .clicked()
        {
            search = true;
        }
    });
    ui.label(
        egui::RichText::new("Una dirección, un sitio o un municipio; una referencia catastral (14 caracteres, o 20 de un inmueble); o unas coordenadas como las copia un mapa web («37,18, -6,93»).")
            .small()
            .color(p.muted),
    );
    if search && d.search.is_none() {
        d.start_search();
    }
    if d.search.is_some() {
        ui.horizontal(|ui| {
            ui.spinner();
            ui.label(format!("Buscando «{}»…", d.query.trim()));
        });
    }
    if let Some(e) = &d.search_error {
        ui.label(egui::RichText::new(e).color(p.bad));
    }
    // What was found.
    if d.results.len() > 1 || (d.results.len() == 1 && d.chosen.as_ref() != d.results.first()) {
        let mut picked = None;
        egui::ScrollArea::vertical()
            .id_salt("carto-place-results")
            .max_height(170.0)
            .show(ui, |ui| {
                for (i, r) in d.results.iter().enumerate() {
                    let on = d.chosen.as_ref() == Some(r);
                    let resp = ui
                        .horizontal(|ui| {
                            let s = ui.selectable_label(on, egui::RichText::new(&r.name).strong());
                            ui.label(&r.municipality);
                            ui.label(
                                egui::RichText::new(format!("{} · {}", r.kind, r.origin.name()))
                                    .small()
                                    .color(p.muted),
                            );
                            s
                        })
                        .inner;
                    if resp.clicked() {
                        picked = Some(i);
                    }
                }
            });
        if let Some(i) = picked {
            d.choose(i);
        }
    }
    ui.add_space(4.0);
    // Round what it is brought.
    let radius = d.options.radius;
    match (&d.chosen, georef) {
        (Some(c), None) => {
            chosen_line(ui, c, p);
            let g = geocode::georeference_at(c);
            ui.label(
                egui::RichText::new(format!(
                    "El proyecto no tiene emplazamiento: se le pone con su origen en este punto ({}), y su ±0,00 en el terreno de ahí si se trae el terreno. Se deshace en un paso.",
                    g.map_or_else(String::new, |g| g.crs)
                ))
                .color(p.muted),
            );
        }
        (Some(c), Some(g)) => {
            chosen_line(ui, c, p);
            let far = at
                .chosen_in_plan
                .map(|q| q.distance(archisim_core::Point2::new(0.0, 0.0)));
            ui.radio_value(
                &mut d.around_place,
                true,
                match far {
                    Some(f) => format!(
                        "Alrededor del lugar elegido, a {} del origen del proyecto (el proyecto no se mueve: el entorno queda donde le toca)",
                        distance_text(f)
                    ),
                    None => "Alrededor del lugar elegido (el proyecto no se mueve)".into(),
                },
            );
            ui.radio_value(
                &mut d.around_place,
                false,
                format!(
                    "Alrededor del origen del proyecto ({}, X {:.0} · Y {:.0})",
                    g.crs, g.eastings, g.northings
                ),
            );
            if far.is_some_and(|f| f > 20_000.0) && d.around_place {
                ui.label(
                    egui::RichText::new("El lugar está muy lejos del proyecto: ¿es el sitio que buscas? Si el emplazamiento del proyecto está mal, cámbialo en sus propiedades.")
                        .color(p.bad),
                );
            }
        }
        (None, Some(g)) => {
            ui.label(
                egui::RichText::new(format!(
                    "Alrededor del origen del proyecto ({}, X {:.0} · Y {:.0}). Busca un lugar para traerlo alrededor de otro sitio.",
                    g.crs, g.eastings, g.northings
                ))
                .color(p.muted),
            );
        }
        (None, None) => {
            ui.label(
                egui::RichText::new("El proyecto no tiene emplazamiento: busca el lugar arriba, o ponlo en las propiedades del proyecto («Emplazamiento (georreferencia)», sin nada seleccionado).")
                    .color(p.bad),
            );
        }
    }
    if let Some(pv) = &mut d.preview {
        little_map(ui, pv, d.chosen.as_ref(), at, radius, p);
    }
}

/// «Calle Ancha, 12 · Punta Umbría (Huelva)», its coordinates and who
/// found it.
fn chosen_line(ui: &mut egui::Ui, c: &Place, p: &Palette) {
    ui.horizontal_wrapped(|ui| {
        ui.label("Lugar elegido:");
        ui.label(egui::RichText::new(c.line()).strong().color(p.accent));
    });
    ui.label(
        egui::RichText::new(format!(
            "{} · {} · {}",
            c.kind,
            c.coordinates_text(),
            c.origin.name()
        ))
        .small()
        .color(p.muted),
    );
}

/// «850 m», «12,3 km».
fn distance_text(d: f64) -> String {
    if d < 1000.0 {
        format!("{:.0} m", d)
    } else {
        format!("{:.1} km", d / 1000.0).replace('.', ",")
    }
}

/// The orthophoto round the centre, with the circle of what is brought,
/// the chosen place and the project's origin.
fn little_map(
    ui: &mut egui::Ui,
    pv: &mut Preview,
    chosen: Option<&Place>,
    at: &Origins,
    radius: f64,
    p: &Palette,
) {
    if pv.texture.is_none() && pv.failed.is_none() {
        if let Some(r) = take(&pv.job) {
            match r.and_then(|b| {
                archisim_geom::decals::decode_png(&b)
                    .ok_or_else(|| "no ha llegado una imagen".to_string())
            }) {
                Ok(px) => {
                    let img =
                        egui::ColorImage::from_rgba_unmultiplied([px.width, px.height], &px.rgba);
                    pv.texture = Some(ui.ctx().load_texture(
                        "carto-preview",
                        img,
                        egui::TextureOptions::LINEAR,
                    ));
                }
                Err(e) => pv.failed = Some(e),
            }
        }
    }
    let side = 230.0;
    ui.horizontal(|ui| {
        let (rect, _) = ui.allocate_exact_size(egui::vec2(side, side), egui::Sense::hover());
        let painter = ui.painter_at(rect);
        painter.rect_filled(rect, 4.0, p.ground);
        match (&pv.texture, &pv.failed) {
            (Some(t), _) => {
                painter.image(
                    t.id(),
                    rect,
                    egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                    egui::Color32::WHITE,
                );
            }
            (None, Some(_)) => {
                painter.text(
                    rect.center() + egui::vec2(0.0, -side * 0.3),
                    egui::Align2::CENTER_CENTER,
                    "Sin ortofoto",
                    egui::FontId::proportional(12.0),
                    p.muted,
                );
            }
            (None, None) => {
                painter.text(
                    rect.center() + egui::vec2(0.0, -side * 0.3),
                    egui::Align2::CENTER_CENTER,
                    "Cargando la ortofoto…",
                    egui::FontId::proportional(12.0),
                    p.muted,
                );
            }
        }
        let spot = |lat: f64, lon: f64| {
            let (u, v) = geocode::preview_spot(pv.centre.0, pv.centre.1, pv.bbox, lat, lon);
            rect.min + egui::vec2(u as f32 * side, v as f32 * side)
        };
        let k = side / (pv.bbox[2] - pv.bbox[0]) as f32;
        let ink = egui::Color32::from_rgb(230, 60, 40);
        if let Some((lat, lon)) = at.centre {
            let c = spot(lat, lon);
            painter.circle_stroke(
                c,
                radius as f32 * k,
                egui::Stroke::new(2.5, egui::Color32::WHITE),
            );
            painter.circle_stroke(c, radius as f32 * k, egui::Stroke::new(1.5, ink));
        }
        if let Some((lat, lon)) = chosen.and_then(|c| c.lat_lon) {
            let c = spot(lat, lon);
            painter.circle_filled(c, 5.0, egui::Color32::WHITE);
            painter.circle_filled(c, 3.5, ink);
        }
        if let (Some((lat, lon)), true) = (at.origin, chosen.is_some() || at.centre.is_some()) {
            // The project's origin: a cross.
            let c = spot(lat, lon);
            for (a, b) in [
                (egui::vec2(-6.0, 0.0), egui::vec2(6.0, 0.0)),
                (egui::vec2(0.0, -6.0), egui::vec2(0.0, 6.0)),
            ] {
                painter.line_segment([c + a, c + b], egui::Stroke::new(3.0, egui::Color32::WHITE));
                painter.line_segment([c + a, c + b], egui::Stroke::new(1.5, egui::Color32::BLACK));
            }
        }
        painter.text(
            rect.right_bottom() + egui::vec2(-4.0, -3.0),
            egui::Align2::RIGHT_BOTTOM,
            "PNOA © IGN",
            egui::FontId::proportional(10.0),
            egui::Color32::WHITE,
        );
        ui.vertical(|ui| {
            ui.label(egui::RichText::new(format!("Radio: {}", distance_text(radius))).strong());
            ui.label(
                egui::RichText::new(format!("Lado del mapa: {}", distance_text(pv.half * 2.0)))
                    .small()
                    .color(p.muted),
            );
            if chosen.is_some() {
                ui.label(egui::RichText::new("● lugar elegido").small().color(ink));
            }
            if at.origin.is_some() {
                ui.label(
                    egui::RichText::new("+ origen del proyecto")
                        .small()
                        .color(p.muted),
                );
            }
            if let Some(e) = &pv.failed {
                ui.label(
                    egui::RichText::new(format!("Sin ortofoto: {e}"))
                        .small()
                        .color(p.muted),
                );
            }
        });
    });
}
