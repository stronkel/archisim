//! ArchiSim in the browser (`docs/IPAD.md`, T1).
//!
//! The page (`index.html` of this crate) has a canvas `archisim`; the program starts
//! there with eframe's web runner (WebGPU where the browser has it, WebGL
//! if not). It opens the demo house that the build puts next to the page
//! (`demo.archisim`), or a new project if it is not there.
//!
//! Files (T2): the page has no folders, so a project or an IFC file comes
//! in through the iPad's file picker (`pick`, which can take it from
//! iCloud Drive or any app's folder) or dropped on the page, and goes out
//! through the share sheet or as a download (`deliver`). A copy of the
//! project stays in the browser (`keep`), and the page opens it the next
//! time (`kept`).

use std::sync::Mutex;

use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast as _;
use wasm_bindgen_futures::JsFuture;

#[wasm_bindgen(module = "/src/web_files.js")]
extern "C" {
    fn archisim_pick(title: &str, text: &str) -> js_sys::Promise;
    fn archisim_deliver(name: &str, bytes: &JsValue, mime: &str) -> js_sys::Promise;
    fn archisim_keep(key: &str, name: &str, bytes: &JsValue) -> js_sys::Promise;
    fn archisim_kept(key: &str) -> js_sys::Promise;
    fn archisim_set_unsaved(unsaved: bool);
    fn archisim_watch_pencil(canvas_id: &str);
    fn archisim_is_pen(identifier: i32) -> bool;
    fn archisim_workers() -> u32;
    fn archisim_run_aside(
        module: &JsValue,
        kind: &str,
        input: &js_sys::Uint8Array,
        params: &str,
        on_progress: &js_sys::Function,
    ) -> js_sys::Promise;
    #[wasm_bindgen(catch)]
    fn archisim_http_sync(url: &str, form: Option<String>) -> Result<js_sys::Uint8Array, JsValue>;
    fn archisim_has_bridge() -> bool;
    fn archisim_note_memory(bytes: f64);
}

/// Whether a touch (egui's id: the browser's `identifier`) is the Pencil.
pub fn is_pen(id: u64) -> bool {
    archisim_is_pen(id as i32)
}

/// What a chosen file is for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Purpose {
    /// A project (or an IFC file, which is imported).
    Open,
    /// An IFC file to import.
    Import,
    /// A project to start a new one from.
    Template,
}

/// A file chosen in the picker or dropped on the page.
pub struct Picked {
    pub purpose: Purpose,
    pub name: String,
    pub bytes: Vec<u8>,
}

static PICKED: Mutex<Vec<Picked>> = Mutex::new(Vec::new());

/// The key of the project's copy in the browser.
const KEPT: &str = "ultimo";

/// Asks for a file with the iPad's picker; it reaches the window by
/// `take_picked`.
pub fn pick(purpose: Purpose) {
    let (title, text) = match purpose {
        Purpose::Open => (
            "Abrir proyecto",
            "Un proyecto de ArchiSim (.archisim) o un IFC, de Archivos, iCloud Drive o cualquier carpeta.",
        ),
        Purpose::Import => ("Importar IFC", "Un fichero IFC (.ifc) de cualquier programa."),
        Purpose::Template => (
            "Nuevo desde plantilla",
            "Una plantilla o un proyecto de ArchiSim: se toman sus tipos, plantas y planos, sin lo dibujado.",
        ),
    };
    let promise = archisim_pick(title, text);
    wasm_bindgen_futures::spawn_local(async move {
        let Ok(answer) = JsFuture::from(promise).await else {
            return;
        };
        if answer.is_null() || answer.is_undefined() {
            return;
        }
        let pair = js_sys::Array::from(&answer);
        let name = pair.get(0).as_string().unwrap_or_default();
        let bytes = js_sys::Uint8Array::new(&pair.get(1)).to_vec();
        push_picked(Picked {
            purpose,
            name,
            bytes,
        });
    });
}

/// A file for the window (chosen, or dropped on the page).
pub fn push_picked(file: Picked) {
    if let Ok(mut q) = PICKED.lock() {
        q.push(file);
    }
    crate::open_files::wake();
}

/// The files chosen since the last frame.
pub fn take_picked() -> Vec<Picked> {
    PICKED
        .lock()
        .map(|mut q| std::mem::take(&mut *q))
        .unwrap_or_default()
}

/// Hands a file over: the share sheet («Guardar en Archivos») or a
/// download.
pub fn deliver(name: &str, bytes: &[u8], mime: &str) {
    // A copy: the call is answered after this frame's memory has moved on.
    let data = js_sys::Uint8Array::from(bytes);
    let _ = archisim_deliver(name, &data, mime);
}

/// Keeps a copy of the project in the browser.
pub fn keep(name: &str, bytes: &[u8]) {
    let data = js_sys::Uint8Array::from(bytes);
    let promise = archisim_keep(KEPT, name, &data);
    wasm_bindgen_futures::spawn_local(async move {
        if JsFuture::from(promise).await.is_err() {
            web_sys::console::warn_1(&"No se pudo guardar la copia en el navegador".into());
        }
    });
}

/// The copy kept in the browser: name, bytes and when (milliseconds since
/// 1970).
async fn kept() -> Option<(String, Vec<u8>, f64)> {
    let answer = JsFuture::from(archisim_kept(KEPT)).await.ok()?;
    if answer.is_null() || answer.is_undefined() {
        return None;
    }
    let v = js_sys::Array::from(&answer);
    let name = v.get(0).as_string()?;
    let bytes = js_sys::Uint8Array::new(&v.get(1)).to_vec();
    Some((name, bytes, v.get(2).as_f64().unwrap_or(0.0)))
}

/// Whether leaving the page asks first.
pub fn set_unsaved(unsaved: bool) {
    archisim_set_unsaved(unsaved);
}

/// A time in milliseconds since 1970, as the browser writes it in the
/// person's own clock («14/11/2026, 21:05»).
fn local_time(ms: f64) -> String {
    let d = js_sys::Date::new(&JsValue::from_f64(ms));
    d.to_locale_string("es-ES", &JsValue::UNDEFINED)
        .as_string()
        .unwrap_or_default()
}

use crate::app::ArchiSimApp;
use crate::editor::Editor;

/// Notes for the person (things the web version does not do yet), shown by
/// the window on its next frame.
static NOTES: Mutex<Vec<String>> = Mutex::new(Vec::new());

/// Leaves a note for the window, and in the browser's console.
pub fn note(text: &str) {
    web_sys::console::log_1(&text.into());
    if let Ok(mut n) = NOTES.lock() {
        n.push(text.to_string());
    }
}

/// The notes left since the last frame.
pub fn take_notes() -> Vec<String> {
    NOTES
        .lock()
        .map(|mut n| std::mem::take(&mut *n))
        .unwrap_or_default()
}

/// The file next to the page, as text (`None` if it is not there).
async fn fetch_text(url: &str) -> Option<String> {
    let window = web_sys::window()?;
    let answer = JsFuture::from(window.fetch_with_str(url)).await.ok()?;
    let response: web_sys::Response = answer.dyn_into().ok()?;
    if !response.ok() {
        return None;
    }
    let text = JsFuture::from(response.text().ok()?).await.ok()?;
    text.as_string()
}

/// Starts the program in the page's canvas (in the page only: the
/// browser's workers load the same program, and there it waits for jobs,
/// `archisim_worker_job`).
pub fn start() {
    if web_sys::window().is_none() {
        return;
    }
    wasm_bindgen_futures::spawn_local(async {
        let Some(canvas) = web_sys::window()
            .and_then(|w| w.document())
            .and_then(|d| d.get_element_by_id("archisim"))
            .and_then(|e| e.dyn_into::<web_sys::HtmlCanvasElement>().ok())
        else {
            web_sys::console::error_1(&"Falta el lienzo «archisim» en la página".into());
            return;
        };
        // The copy kept the last time, if there is one; if not, the demo.
        let kept_project = kept().await.and_then(|(name, bytes, time)| {
            let text = String::from_utf8(bytes).ok()?;
            let model = archisim_file::from_str(&text).ok()?;
            Some((name, model, time))
        });
        let ed = match kept_project {
            Some((name, model, time)) => {
                let mut ed = Editor::with_model(model, Some(std::path::PathBuf::from(&name)));
                // What the browser kept may not have been handed over yet.
                ed.mark_unhanded();
                note(&format!(
                    "Abierta la copia guardada en el navegador de «{name}» ({}). Guárdala con Ctrl+S o Archivo → Guardar para tenerla en Archivos.",
                    local_time(time)
                ));
                ed
            }
            None => match fetch_text("demo.archisim").await {
                Some(text) => match archisim_file::from_str(&text) {
                    Ok(model) => Editor::with_model(model, None),
                    Err(e) => {
                        note(&format!("No se puede abrir la casa de ejemplo: {e}."));
                        Editor::new_project()
                    }
                },
                None => Editor::new_project(),
            },
        };
        let app = ArchiSimApp::new(ed);
        archisim_watch_pencil("archisim");
        let started = eframe::WebRunner::new()
            .start(
                canvas,
                eframe::WebOptions::default(),
                Box::new(|cc| {
                    crate::open_files::set_context(&cc.egui_ctx);
                    Ok(Box::new(app))
                }),
            )
            .await;
        // The page shows its «loading» text until the program is running.
        if let Some(loading) = web_sys::window()
            .and_then(|w| w.document())
            .and_then(|d| d.get_element_by_id("cargando"))
        {
            match &started {
                Ok(()) => loading.remove(),
                Err(e) => {
                    loading.set_inner_html(&format!(
                        "ArchiSim no ha podido arrancar en este navegador: {e:?}"
                    ));
                }
            }
        }
    });
}

// --- Lo pesado (T5): the browser's workers, the NAS's bridge, memory ---

/// How much memory the program holds now, in bytes: what the browser
/// gave its WebAssembly memory, which only grows.
pub fn memory_bytes() -> usize {
    core::arch::wasm32::memory_size::<0>() * 65536
}

/// Leaves the memory where the page's tests read it.
pub fn note_memory() {
    archisim_note_memory(memory_bytes() as f64);
}

/// One address, waiting for its answer: through the NAS's bridge when the
/// page has it (`platform::bridged`).
pub fn http_get(url: &str, form: Option<&str>) -> Result<Vec<u8>, String> {
    let url = crate::platform::bridged(url, archisim_has_bridge());
    archisim_http_sync(&url, form.map(str::to_string))
        .map(|b| b.to_vec())
        .map_err(|e| e.as_string().unwrap_or_else(|| format!("{e:?}")))
}

/// Whether the page is served by the NAS with its bridge.
pub fn has_bridge() -> bool {
    archisim_has_bridge()
}

/// A job for a worker (`archisim_worker_job` there): its answer, or why
/// not. `progress` hears how far it has gone, from 0 to 1.
fn aside(
    kind: &str,
    input: &js_sys::Uint8Array,
    params: &str,
    progress: impl Fn(f64) + 'static,
) -> impl std::future::Future<Output = Result<Vec<u8>, String>> {
    let on_progress = Closure::<dyn Fn(f64)>::new(progress);
    let promise = archisim_run_aside(
        &wasm_bindgen::module(),
        kind,
        input,
        params,
        on_progress.as_ref().unchecked_ref(),
    );
    async move {
        let answer = JsFuture::from(promise).await;
        drop(on_progress);
        match answer {
            Ok(v) => Ok(js_sys::Uint8Array::new(&v).to_vec()),
            Err(e) => Err(e.as_string().unwrap_or_else(|| format!("{e:?}"))),
        }
    }
}

/// The picture of the render in the browser's workers (each its rows,
/// `archisim_render::render_rows`), into `slot`; here, as on the desktop,
/// if the browser has no workers or they fail.
pub fn render_aside(
    model: archisim_core::Model,
    view: archisim_render::View,
    options: archisim_render::Options,
    slot: std::sync::Arc<Mutex<Option<archisim_render::Image>>>,
    progress: std::sync::Arc<std::sync::atomic::AtomicU32>,
) {
    use std::sync::atomic::Ordering;
    let parts = archisim_workers() as usize;
    let p = progress.clone();
    let here = move |model: &archisim_core::Model| {
        archisim_render::render(model, &view, &options, &|f| {
            progress.store((f * 1000.0) as u32, Ordering::Relaxed)
        })
    };
    if parts == 0 {
        let image = here(&model);
        if let Ok(mut s) = slot.lock() {
            *s = Some(image);
        }
        return;
    }
    let text = archisim_file::to_string(&model);
    let input = js_sys::Uint8Array::from(text.as_bytes());
    let done = std::rc::Rc::new(std::cell::RefCell::new(vec![0.0f64; parts]));
    wasm_bindgen_futures::spawn_local(async move {
        let mut jobs = Vec::new();
        for k in 0..parts {
            let params = render_params(&view, &options, k, parts);
            let (done, p) = (done.clone(), p.clone());
            jobs.push(aside("render", &input, &params, move |f| {
                let mut d = done.borrow_mut();
                d[k] = f;
                let all = d.iter().sum::<f64>() / d.len() as f64;
                p.store((all * 1000.0) as u32, Ordering::Relaxed);
            }));
        }
        let (w, h) = (options.width.max(1), options.height.max(1));
        let mut rgb = vec![0u8; w * h * 3];
        let mut failed = None;
        for (k, job) in jobs.into_iter().enumerate() {
            match job.await {
                Ok(rows) => {
                    for (i, y) in (0..h).filter(|y| y % parts == k).enumerate() {
                        let row = w * 3;
                        if let Some(src) = rows.get(i * row..(i + 1) * row) {
                            rgb[y * row..(y + 1) * row].copy_from_slice(src);
                        }
                    }
                }
                Err(e) => failed = Some(e),
            }
        }
        let image = match failed {
            None => archisim_render::Image {
                width: w,
                height: h,
                rgb,
            },
            Some(e) => {
                note(&format!(
                    "El render no ha podido hacerse aparte ({e}); se hace aquí, con la ventana quieta."
                ));
                here(&model)
            }
        };
        if let Ok(mut s) = slot.lock() {
            *s = Some(image);
        }
    });
}

/// What a worker needs to know of a render besides the project.
fn render_params(
    view: &archisim_render::View,
    o: &archisim_render::Options,
    part: usize,
    parts: usize,
) -> String {
    let light = match o.light {
        archisim_render::Light::Day => 0,
        archisim_render::Light::Evening => 1,
        archisim_render::Light::Night => 2,
    };
    serde_json::json!({
        "eye": view.eye,
        "target": view.target,
        "fov_y": view.fov_y,
        "perspective": view.perspective,
        "half_height": view.half_height,
        "width": o.width,
        "height": o.height,
        "samples": o.samples,
        "sky_rays": o.sky_rays,
        "light": light,
        "textures": o.textures,
        "part": part,
        "parts": parts,
    })
    .to_string()
}

/// In a worker: does a job of the page (`aside`) and gives its answer.
/// `render`: the project (`.archisim`) and `render_params`; answers the
/// rows `y % parts == part` of the picture.
#[wasm_bindgen]
pub fn archisim_worker_job(
    kind: &str,
    input: &[u8],
    params: &str,
    progress: &js_sys::Function,
) -> Result<Vec<u8>, JsValue> {
    let fail = |e: String| JsValue::from_str(&e);
    match kind {
        "render" => {
            let p: serde_json::Value =
                serde_json::from_str(params).map_err(|e| fail(e.to_string()))?;
            let text = std::str::from_utf8(input).map_err(|e| fail(e.to_string()))?;
            let model = archisim_file::from_str(text).map_err(|e| fail(e.to_string()))?;
            let v3 = |k: &str| -> [f64; 3] {
                let a = &p[k];
                [0, 1, 2].map(|i| a[i].as_f64().unwrap_or(0.0))
            };
            let num = |k: &str| p[k].as_f64().unwrap_or(0.0);
            let int = |k: &str| p[k].as_u64().unwrap_or(1) as usize;
            let view = archisim_render::View {
                eye: v3("eye"),
                target: v3("target"),
                fov_y: num("fov_y"),
                perspective: p["perspective"].as_bool().unwrap_or(true),
                half_height: num("half_height"),
            };
            let options = archisim_render::Options {
                width: int("width"),
                height: int("height"),
                samples: int("samples"),
                sky_rays: int("sky_rays"),
                light: match p["light"].as_u64() {
                    Some(1) => archisim_render::Light::Evening,
                    Some(2) => archisim_render::Light::Night,
                    _ => archisim_render::Light::Day,
                },
                textures: p["textures"].as_bool().unwrap_or(true),
            };
            let (part, parts) = (int("part"), int("parts").max(1));
            let rows: Vec<usize> = (0..options.height.max(1))
                .filter(|y| y % parts == part)
                .collect();
            let tell = OneThread(progress);
            let image = archisim_render::render_rows(&model, &view, &options, &rows, &|f| {
                let _ = tell.0.call1(&JsValue::NULL, &JsValue::from_f64(f));
            });
            Ok(image.rgb)
        }
        _ => Err(fail(format!("trabajo desconocido «{kind}»"))),
    }
}

/// Something of JavaScript used from one thread only: a worker has no
/// threads of its own, so what the render asks to share between threads
/// is never shared.
struct OneThread<'a>(&'a js_sys::Function);
// SAFETY: WebAssembly without threads (this program in the browser) runs
// everything on one thread.
unsafe impl Sync for OneThread<'_> {}

// --- The page's own tests (scripts/web_heavy.py): `?prueba=render,lugar` ---

static FRAMES: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);

/// Whether the page's address asks for the test `name`, once, a few
/// frames after starting.
pub fn test_due(name: &str) -> bool {
    use std::sync::atomic::Ordering;
    const AT: u32 = 30;
    if FRAMES.load(Ordering::Relaxed) != AT {
        return false;
    }
    let search = web_sys::window()
        .and_then(|w| w.location().search().ok())
        .unwrap_or_default();
    search
        .trim_start_matches('?')
        .split('&')
        .filter_map(|kv| kv.strip_prefix("prueba="))
        .any(|v| v.split(',').any(|t| t == name))
}

/// One frame more (for `test_due`).
pub fn count_frame() {
    FRAMES.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
}

/// Leaves a number where the page's tests read it (`window[key]`).
pub fn report(key: &str, value: f64) {
    if let Some(w) = web_sys::window() {
        let _ = js_sys::Reflect::set(&w, &JsValue::from_str(key), &JsValue::from_f64(value));
    }
}
