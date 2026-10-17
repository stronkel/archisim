//! ArchiSim desktop application.
//!
//! ```text
//! archisim-app                       proyecto nuevo con la plantilla
//! archisim-app casa.archisim         abre un proyecto
//! ```

// Windows: no console window behind the application in release builds.
#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

mod app;
mod dim_tool;
mod drag;
mod editor;
mod opening_tool;
mod pick;
mod plan_image;
mod slab_tool;
mod snap;
mod type_preview;
mod units;
mod view;
mod view3d;
mod wall_tool;

use app::ArchiSimApp;
use editor::Editor;

fn main() -> eframe::Result {
    let ed = Editor::new_project();
    let mut app = ArchiSimApp::new(ed);
    // A project or an IFC file given on the command line (or by opening a
    // file with the application).
    if let Some(path) = std::env::args_os().nth(1) {
        let path = std::path::PathBuf::from(path);
        let is_ifc = path
            .extension()
            .is_some_and(|e| e.eq_ignore_ascii_case("ifc"));
        if is_ifc {
            app.import_path(&path);
        } else {
            app.open_path(&path);
        }
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
