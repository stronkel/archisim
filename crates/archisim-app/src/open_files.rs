//! Files the operating system asks the application to open while it is
//! running or starting: on macOS, a `.archisim` or `.ifc` double-clicked in
//! the Finder or dropped on the Dock icon; on every system, files dropped
//! on the window. They wait in a queue that the application takes from on
//! its next frame.

use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};

use eframe::egui;

static QUEUE: Mutex<Vec<PathBuf>> = Mutex::new(Vec::new());
static CONTEXT: OnceLock<egui::Context> = OnceLock::new();

/// Queues a file to open and wakes the window so it is opened at once.
/// Only macOS hands files over this way (elsewhere they come as program
/// arguments or dropped on the window).
#[cfg_attr(not(target_os = "macos"), allow(dead_code))]
pub fn push(path: PathBuf) {
    if let Ok(mut q) = QUEUE.lock() {
        q.push(path);
    }
    if let Some(ctx) = CONTEXT.get() {
        ctx.request_repaint();
    }
}

/// The files waiting to be opened, oldest first.
pub fn take() -> Vec<PathBuf> {
    QUEUE
        .lock()
        .map(|mut q| std::mem::take(&mut *q))
        .unwrap_or_default()
}

/// The window's context, to wake it when a file arrives.
pub fn set_context(ctx: &egui::Context) {
    let _ = CONTEXT.set(ctx.clone());
}

/// Whether a file is something ArchiSim opens: a project or an IFC file.
pub fn is_openable(path: &std::path::Path) -> bool {
    path.extension().is_some_and(|e| {
        e.eq_ignore_ascii_case(archisim_file::EXTENSION) || e.eq_ignore_ascii_case("ifc")
    })
}

#[cfg(target_os = "macos")]
pub mod macos {
    //! Receiving the files opened from the Finder (`application:openURLs:`).
    //!
    //! winit 0.30 registers its own application delegate (the class
    //! `WinitApplicationDelegate`) and expects it to stay the delegate, so
    //! ArchiSim cannot install one of its own. Instead it adds the
    //! `application:openURLs:` method to winit's delegate class, once the
    //! event loop exists and before it runs.

    use objc2::ffi;
    use objc2::runtime::{AnyClass, AnyObject, Sel};
    use objc2::sel;
    use objc2_foundation::{NSArray, NSURL};

    /// `- (void)application:(NSApplication *)app openURLs:(NSArray<NSURL *> *)urls`
    extern "C" fn open_urls(
        _this: *mut AnyObject,
        _cmd: Sel,
        _application: *mut AnyObject,
        urls: *mut NSArray<NSURL>,
    ) {
        // SAFETY: AppKit passes a valid array of URLs for the duration of
        // the call; only plain getters are used. Nothing here panics.
        let Some(urls) = (unsafe { urls.as_ref() }) else {
            return;
        };
        for url in urls.iter() {
            // SAFETY: as above.
            if !unsafe { url.isFileURL() } {
                continue;
            }
            // SAFETY: as above.
            if let Some(path) = unsafe { url.path() } {
                let path = std::path::PathBuf::from(path.to_string());
                report(&path);
                super::push(path);
            }
        }
    }

    /// Adds the method to winit's delegate. Call after the event loop is
    /// created (winit registers the class and sets the delegate then) and
    /// before it runs. Returns whether it was added.
    pub fn install() -> bool {
        let Some(class) = AnyClass::get("WinitApplicationDelegate") else {
            eprintln!("ArchiSim: no se encuentra el delegado de winit; los ficheros del Finder no se abrirán");
            return false;
        };
        let f: extern "C" fn(*mut AnyObject, Sel, *mut AnyObject, *mut NSArray<NSURL>) = open_urls;
        // SAFETY: an IMP is any C function pointer whose signature matches
        // the type encoding given: void return, self, _cmd and two objects.
        let imp: ffi::IMP = Some(unsafe { std::mem::transmute::<_, unsafe extern "C" fn()>(f) });
        let added = unsafe {
            ffi::class_addMethod(
                class as *const AnyClass as *mut ffi::objc_class,
                sel!(application:openURLs:).as_ptr(),
                imp,
                c"v@:@@".as_ptr(),
            )
        };
        added != ffi::NO
    }

    /// For the packaging check in continuous integration: with
    /// `ARCHISIM_OPENED_LOG` set, each file received from the Finder is
    /// also written to that file.
    fn report(path: &std::path::Path) {
        if let Some(log) = std::env::var_os("ARCHISIM_OPENED_LOG") {
            use std::io::Write;
            if let Ok(mut f) = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(log)
            {
                let _ = writeln!(f, "{}", path.display());
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn queue_keeps_order_and_empties() {
        // The only test that touches the queue (the window does not read it
        // in tests, which run in parallel).
        push(PathBuf::from("a.archisim"));
        push(PathBuf::from("b.ifc"));
        assert_eq!(
            take(),
            vec![PathBuf::from("a.archisim"), PathBuf::from("b.ifc")]
        );
        assert!(take().is_empty());
        assert!(is_openable(std::path::Path::new("Casa.ARCHISIM")));
        assert!(is_openable(std::path::Path::new("modelo.IFC")));
        assert!(!is_openable(std::path::Path::new("plano.pdf")));
    }
}
