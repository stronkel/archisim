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
mod open_files;
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
    let mut viewport = eframe::egui::ViewportBuilder::default()
        .with_title("ArchiSim")
        .with_inner_size([1280.0, 800.0])
        .with_min_inner_size([800.0, 500.0]);
    if let Ok(icon) = eframe::icon_data::from_png_bytes(include_bytes!("../assets/icon.png")) {
        viewport = viewport.with_icon(icon);
    }
    let options = eframe::NativeOptions {
        viewport,
        ..Default::default()
    };
    // The event loop is created here, not by eframe, so that on macOS the
    // method that receives files opened from the Finder is added to winit's
    // application delegate after the event loop exists and before it runs.
    let event_loop = winit::event_loop::EventLoop::<eframe::UserEvent>::with_user_event()
        .build()
        .map_err(|e| eframe::Error::AppCreation(Box::new(e)))?;
    #[cfg(target_os = "macos")]
    let _ = open_files::macos::install();
    let mut winit_app = eframe::create_native(
        "ArchiSim",
        options,
        Box::new(|cc| {
            open_files::set_context(&cc.egui_ctx);
            Ok(Box::new(app))
        }),
        &event_loop,
    );
    event_loop
        .run_app(&mut winit_app)
        .map_err(|e| eframe::Error::AppCreation(Box::new(e)))
}
