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
    //! The application delegate that receives the files opened from the
    //! Finder (`application:openURLs:`). winit installs no delegate of its
    //! own, so ArchiSim can (see winit's `platform::macos` documentation).

    use objc2::rc::Retained;
    use objc2::runtime::ProtocolObject;
    use objc2::{declare_class, msg_send_id, mutability, ClassType, DeclaredClass};
    use objc2_app_kit::{NSApplication, NSApplicationDelegate};
    use objc2_foundation::{MainThreadMarker, NSArray, NSObject, NSObjectProtocol, NSURL};

    declare_class!(
        struct AppDelegate;

        unsafe impl ClassType for AppDelegate {
            type Super = NSObject;
            type Mutability = mutability::MainThreadOnly;
            const NAME: &'static str = "ArchiSimAppDelegate";
        }

        impl DeclaredClass for AppDelegate {}

        unsafe impl NSObjectProtocol for AppDelegate {}

        unsafe impl NSApplicationDelegate for AppDelegate {
            #[method(application:openURLs:)]
            fn application_open_urls(&self, _application: &NSApplication, urls: &NSArray<NSURL>) {
                for url in urls.iter() {
                    // SAFETY: plain getters of an NSURL handed to us by AppKit.
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
        }
    );

    impl AppDelegate {
        fn new(mtm: MainThreadMarker) -> Retained<Self> {
            unsafe { msg_send_id![super(mtm.alloc().set_ivars(())), init] }
        }
    }

    /// Installs the delegate. Call after the event loop is created and
    /// before it runs, on the main thread.
    pub fn install() {
        let Some(mtm) = MainThreadMarker::new() else {
            return;
        };
        let delegate = AppDelegate::new(mtm);
        let app = NSApplication::sharedApplication(mtm);
        app.setDelegate(Some(ProtocolObject::from_ref(&*delegate)));
        // AppKit keeps only a weak reference to its delegate.
        std::mem::forget(delegate);
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
