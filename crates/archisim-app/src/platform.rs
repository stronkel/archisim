//! What changes between the desktop and the web (`docs/IPAD.md`).
//!
//! In the browser there are no threads of our own (yet: Web Workers come
//! in T5): what the desktop does aside runs at once, and the window waits
//! for it.

/// Whether this is the version for the browser.
pub const WEB: bool = cfg!(target_arch = "wasm32");

/// Runs `work` aside: on another thread on the desktop, at once on the web.
/// Whatever it hands back through a channel or a shared slot is there when
/// the window looks for it, as it would be after the thread ended.
pub fn spawn(work: impl FnOnce() + Send + 'static) {
    #[cfg(not(target_arch = "wasm32"))]
    {
        std::thread::spawn(work);
    }
    #[cfg(target_arch = "wasm32")]
    {
        work();
    }
}

/// How many pieces a long job can be split into at the same time.
#[allow(dead_code)]
pub fn parallelism() -> usize {
    if WEB {
        1
    } else {
        std::thread::available_parallelism().map_or(4, |n| n.get())
    }
}
