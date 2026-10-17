//! ArchiSim desktop application.
//!
//! ```text
//! archisim-app                       proyecto nuevo con la plantilla
//! archisim-app casa.archisim         abre un proyecto
//! ```

// Windows: no console window behind the application in release builds.
#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

mod app;
mod editor;
mod opening_tool;
mod pick;
mod plan_image;
mod slab_tool;
mod snap;
mod units;
mod view;
mod view3d;
mod wall_tool;

use app::ArchiSimApp;
use editor::Editor;

fn main() -> eframe::Result {
    let mut ed = Editor::new_project();
    let mut open_error = None;
    if let Some(path) = std::env::args_os().nth(1) {
        let path = std::path::PathBuf::from(path);
        match Editor::open(&path) {
            Ok(e) => ed = e,
            Err(e) => open_error = Some(format!("No se puede abrir {}: {e}.", path.display())),
        }
    }
    let mut app = ArchiSimApp::new(ed);
    if let Some(e) = open_error {
        app.message = Some(app::Message {
            text: e,
            error: true,
        });
    }
    let options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_title("ArchiSim")
            .with_inner_size([1280.0, 800.0])
            .with_min_inner_size([800.0, 500.0]),
        ..Default::default()
    };
    eframe::run_native("ArchiSim", options, Box::new(|_cc| Ok(Box::new(app))))
}
