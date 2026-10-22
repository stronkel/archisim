//! What went wrong, written down: a panic or an error of the graphics card
//! is appended to `errores.log` in the settings folder (with where it
//! happened), so a crash that only happens on one computer can be read
//! afterwards. On the next start the log is set aside as
//! `errores-anterior.log` and the window says so once.

use std::io::Write as _;

use eframe::wgpu;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

/// The graphics card failed: the 3D view stops drawing instead of
/// crashing the program.
pub static GPU_FAILED: AtomicBool = AtomicBool::new(false);

fn log_file() -> Option<PathBuf> {
    crate::prefs::settings_dir().map(|d| d.join("errores.log"))
}

/// Appends a report to the log.
pub fn write(what: &str) {
    if cfg!(test) {
        return;
    }
    let Some(path) = log_file() else {
        return;
    };
    if let Some(dir) = path.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    if let Ok(mut f) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
    {
        let _ = writeln!(
            f,
            "--- {} · ArchiSim {} · {} {}\n{what}\n",
            archisim_ifc::utc_now(),
            env!("CARGO_PKG_VERSION"),
            std::env::consts::OS,
            std::env::consts::ARCH,
        );
    }
}

/// Writes every panic to the log, then lets it go on as before.
pub fn install() {
    let previous = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let place = info
            .location()
            .map_or(String::new(), |l| format!(" en {}:{}", l.file(), l.line()));
        let message = info
            .payload()
            .downcast_ref::<&str>()
            .map(|s| s.to_string())
            .or_else(|| info.payload().downcast_ref::<String>().cloned())
            .unwrap_or_default();
        let thread = std::thread::current()
            .name()
            .unwrap_or("sin nombre")
            .to_string();
        write(&format!(
            "Fallo (hilo {thread}){place}: {message}\n{}",
            std::backtrace::Backtrace::force_capture()
        ));
        previous(info);
    }));
}

/// Errors of the graphics card are written down (the first few) and stop
/// the 3D view, instead of ending the program.
pub fn watch_device(device: &wgpu::Device) {
    static WRITTEN: AtomicUsize = AtomicUsize::new(0);
    device.on_uncaptured_error(std::sync::Arc::new(|e: wgpu::Error| {
        GPU_FAILED.store(true, Ordering::Relaxed);
        if WRITTEN.fetch_add(1, Ordering::Relaxed) < 5 {
            write(&format!("Error de la tarjeta gráfica: {e}"));
        }
    }));
}

/// Frames finished, and what the window was doing last: a watchdog
/// thread writes it down when a frame takes too long (the spinning
/// beach ball on a Mac), so a hang leaves a trace as a crash does.
static FRAMES: AtomicUsize = AtomicUsize::new(0);
static STAGE: std::sync::Mutex<&'static str> = std::sync::Mutex::new("arranque");

/// Marks the end of a frame.
pub fn frame_done() {
    FRAMES.fetch_add(1, Ordering::Relaxed);
}

/// Says what the window is about to do (for the watchdog's report).
pub fn stage(what: &'static str) {
    if let Ok(mut s) = STAGE.lock() {
        *s = what;
    }
}

/// Starts the watchdog: a frame still unfinished after 4 seconds is
/// written down once, with what it was doing.
pub fn watch_frames() {
    if cfg!(test) {
        return;
    }
    std::thread::Builder::new()
        .name("vigilante".into())
        .spawn(|| {
            let mut last = FRAMES.load(Ordering::Relaxed);
            let mut since = std::time::Instant::now();
            let mut written = false;
            loop {
                std::thread::sleep(std::time::Duration::from_millis(500));
                let now = FRAMES.load(Ordering::Relaxed);
                if now != last {
                    last = now;
                    since = std::time::Instant::now();
                    written = false;
                } else if !written && last > 0 && since.elapsed().as_secs() >= 4 {
                    written = true;
                    let what = STAGE.lock().map_or("¿?", |s| *s);
                    write(&format!(
                        "La ventana no responde desde hace {} s; estaba en: {what}",
                        since.elapsed().as_secs()
                    ));
                }
            }
        })
        .ok();
}

/// The log of the last session, if something was written in it: set
/// aside, and its path returned.
pub fn take_previous() -> Option<PathBuf> {
    if cfg!(test) {
        return None;
    }
    let path = log_file()?;
    let size = std::fs::metadata(&path).ok()?.len();
    if size == 0 {
        return None;
    }
    let old = path.with_file_name("errores-anterior.log");
    std::fs::rename(&path, &old).ok()?;
    Some(old)
}
