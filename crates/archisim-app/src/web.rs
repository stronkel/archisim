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

/// Starts the program in the page's canvas.
pub fn start() {
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
