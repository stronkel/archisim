//! ArchiSim in the browser (`docs/IPAD.md`, T1).
//!
//! The page (`web/index.html`) has a canvas `archisim`; the program starts
//! there with eframe's web runner (WebGPU where the browser has it, WebGL
//! if not). It opens the demo house that the build puts next to the page
//! (`demo.archisim`), or a new project if it is not there.

use std::sync::Mutex;

use wasm_bindgen::JsCast as _;
use wasm_bindgen_futures::JsFuture;

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
        let ed = match fetch_text("demo.archisim").await {
            Some(text) => match archisim_file::from_str(&text) {
                Ok(model) => Editor::with_model(model, None),
                Err(e) => {
                    note(&format!("No se puede abrir la casa de ejemplo: {e}."));
                    Editor::new_project()
                }
            },
            None => Editor::new_project(),
        };
        let app = ArchiSimApp::new(ed);
        note("Versión web de prueba: abrir y guardar archivos llega en T2.");
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
