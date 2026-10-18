//! Files the operating system asks the application to open while it is
//! running or starting: on macOS, a `.archisim` or `.ifc` double-clicked in
//! the Finder or dropped on the Dock icon; on every system, files dropped
//! on the window. They wait in a queue that the application takes from on
//! its next frame.

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, OnceLock};

use eframe::egui;

static QUEUE: Mutex<Vec<PathBuf>> = Mutex::new(Vec::new());
static CONTEXT: OnceLock<egui::Context> = OnceLock::new();
static QUIT: AtomicBool = AtomicBool::new(false);

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

/// The system asks the application to quit (on macOS: Cmd+Q, «Salir» in
/// the application menu or the Dock, closing the session). The window
/// handles it on its next frame like its close button, asking to save
/// first if the project has changes.
#[cfg_attr(not(target_os = "macos"), allow(dead_code))]
pub fn request_quit() {
    QUIT.store(true, Ordering::SeqCst);
    if let Some(ctx) = CONTEXT.get() {
        ctx.request_repaint();
    }
}

/// Whether quitting was asked for since the last call.
pub fn take_quit() -> bool {
    QUIT.swap(false, Ordering::SeqCst)
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
                report_line(&path.display().to_string());
                super::push(path);
            }
        }
    }

    /// `- (NSApplicationTerminateReply)applicationShouldTerminate:(NSApplication *)sender`
    ///
    /// Cmd+Q and «Salir» go through AppKit's `terminate:`, which would end
    /// the program at once, without the window hearing of it and so without
    /// asking to save. Instead the quit is cancelled here and handed to the
    /// window, which asks if needed and then ends the program itself.
    extern "C" fn should_terminate(
        _this: *mut AnyObject,
        _cmd: Sel,
        _application: *mut AnyObject,
    ) -> usize {
        report_line("salir");
        super::request_quit();
        // NSTerminateCancel.
        0
    }

    /// For the packaging check in continuous integration: with
    /// `ARCHISIM_PRUEBA_SALIR` set, once the window has been up for a few
    /// seconds, asks AppKit to quit (`terminate:`, the same as Cmd+Q).
    #[cfg_attr(test, allow(dead_code))]
    pub fn quit_for_check(ctx: &eframe::egui::Context) {
        use std::sync::atomic::{AtomicBool, Ordering};
        static SENT: AtomicBool = AtomicBool::new(false);
        if std::env::var_os("ARCHISIM_PRUEBA_SALIR").is_none() || SENT.load(Ordering::SeqCst) {
            return;
        }
        if ctx.input(|i| i.time) < 3.0 {
            ctx.request_repaint_after(std::time::Duration::from_millis(200));
            return;
        }
        SENT.store(true, Ordering::SeqCst);
        // SAFETY: on the main thread, during a frame; `sharedApplication`
        // returns the running application and `terminate:` takes any sender.
        unsafe {
            let app: *mut AnyObject =
                objc2::msg_send![objc2::class!(NSApplication), sharedApplication];
            let _: () = objc2::msg_send![app, terminate: std::ptr::null::<AnyObject>()];
        }
    }

    /// Adds the methods to winit's delegate. Call after the event loop is
    /// created (winit registers the class and sets the delegate then) and
    /// before it runs. Returns whether both were added.
    pub fn install() -> bool {
        let Some(class) = AnyClass::get("WinitApplicationDelegate") else {
            eprintln!("ArchiSim: no se encuentra el delegado de winit; los ficheros del Finder no se abrirán");
            return false;
        };
        let class_ptr = class as *const AnyClass as *mut ffi::objc_class;
        type ShouldTerminate = extern "C" fn(*mut AnyObject, Sel, *mut AnyObject) -> usize;
        let f: ShouldTerminate = should_terminate;
        // SAFETY: as below; the type encoding is an unsigned long return,
        // self, _cmd and one object.
        let imp: ffi::IMP =
            Some(unsafe { std::mem::transmute::<ShouldTerminate, unsafe extern "C" fn()>(f) });
        let quit_added = unsafe {
            ffi::class_addMethod(
                class_ptr,
                sel!(applicationShouldTerminate:).as_ptr(),
                imp,
                c"Q@:@".as_ptr(),
            )
        } != ffi::NO;
        if !quit_added {
            eprintln!("ArchiSim: no se pudo añadir la petición de salir; Cmd+Q no preguntará antes de cerrar");
        }
        type OpenUrls = extern "C" fn(*mut AnyObject, Sel, *mut AnyObject, *mut NSArray<NSURL>);
        let f: OpenUrls = open_urls;
        // SAFETY: an IMP is any C function pointer whose signature matches
        // the type encoding given: void return, self, _cmd and two objects.
        let imp: ffi::IMP =
            Some(unsafe { std::mem::transmute::<OpenUrls, unsafe extern "C" fn()>(f) });
        let added = unsafe {
            ffi::class_addMethod(
                class_ptr,
                sel!(application:openURLs:).as_ptr(),
                imp,
                c"v@:@@".as_ptr(),
            )
        };
        added != ffi::NO && quit_added
    }

    /// For the packaging check in continuous integration: with
    /// `ARCHISIM_OPENED_LOG` set, each file received from the Finder (and
    /// each request to quit, as «salir») is also written to that file.
    fn report_line(line: &str) {
        if let Some(log) = std::env::var_os("ARCHISIM_OPENED_LOG") {
            use std::io::Write;
            if let Ok(mut f) = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(log)
            {
                let _ = writeln!(f, "{line}");
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
