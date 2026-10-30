//! ArchiSim desktop application.
//!
//! ```text
//! archisim-app                       proyecto nuevo con la plantilla
//! archisim-app casa.archisim         abre un proyecto
//! ```

// Windows: no console window behind the application in release builds.
#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

mod app;
mod brush;
mod cad;
mod carpentry;
mod coord;
mod crash;
mod dim_tool;
mod documents;
mod drag;
mod editor;
mod flip;
mod geo;
mod missions;
mod open_files;
mod opening_tool;
mod paper;
mod pick;
mod placing;
mod plan_image;
mod prefs;
mod sketch;
mod slab_tool;
mod snap;
mod spline;
mod team;
mod temp_dims;
mod type_drawing;
mod type_preview;
mod units;
mod view;
mod view3d;
mod wall_faces;
mod wall_tool;
mod xform;

use app::ArchiSimApp;
use editor::Editor;

fn main() -> eframe::Result {
    crash::install();
    crash::watch_frames();
    let ed = Editor::new_project();
    let mut app = ArchiSimApp::new(ed);
    if let Some(log) = crash::take_previous() {
        app.previous_crash(&log);
    }
    // A project or an IFC file given on the command line (or by opening a
    // file with the application); a survey (DXF) goes into a new project's
    // terrain.
    if let Some(path) = std::env::args_os().nth(1) {
        let path = std::path::PathBuf::from(path);
        let ext = path
            .extension()
            .map(|e| e.to_string_lossy().to_lowercase())
            .unwrap_or_default();
        match ext.as_str() {
            "ifc" => app.import_path(&path),
            "dxf" => app.open_survey(&path),
            "asequipo" => app.open_team(&path),
            // A 3D model (or a manufacturer's .zip): a new project and the
            // dialog to bring it in.
            _ if archisim_mesh::is_model_file(&path) => app.open_files(vec![path]),
            _ => app.open_path(&path),
        }
    }
    let mut viewport = eframe::egui::ViewportBuilder::default()
        .with_title("ArchiSim")
        .with_inner_size([1440.0, 900.0])
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
