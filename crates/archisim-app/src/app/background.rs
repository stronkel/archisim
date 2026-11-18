//! Pictures drawn on another thread (remate de F1): when a section, an
//! elevation or a sheet has been moved or zoomed, its new picture is made
//! aside while the old one, moved, stays on screen, so the window never
//! waits for it.

use std::sync::{Arc, Mutex};

use eframe::egui::ColorImage;

use super::ArchiSimApp;

/// A picture being made for `key`.
pub struct Pending<K, T = ColorImage> {
    pub key: K,
    slot: Arc<Mutex<Option<Result<T, String>>>>,
    /// When it was started.
    pub started: web_time::Instant,
}

/// What a panic said.
pub fn panic_text(payload: &(dyn std::any::Any + Send)) -> String {
    payload
        .downcast_ref::<&str>()
        .map(|s| s.to_string())
        .or_else(|| payload.downcast_ref::<String>().cloned())
        .unwrap_or_else(|| "error desconocido".to_string())
}

impl<K, T: Send + 'static> Pending<K, T> {
    /// Starts making the picture of `key` with `work`, on another thread.
    /// If the work fails, it says so ([`Self::failed`]) instead of never
    /// coming (0.92.1: a plan or a section left waiting for ever).
    pub fn start(key: K, work: impl FnOnce() -> T + Send + 'static) -> Self {
        let slot = Arc::new(Mutex::new(None));
        let to = slot.clone();
        crate::platform::spawn(move || {
            let done = std::panic::catch_unwind(std::panic::AssertUnwindSafe(work))
                .map_err(|e| panic_text(e.as_ref()));
            let mut s = to.lock().unwrap_or_else(|e| e.into_inner());
            *s = Some(done);
        });
        Pending {
            key,
            slot,
            started: web_time::Instant::now(),
        }
    }

    /// The picture, once made.
    pub fn take(&self) -> Option<T> {
        let mut s = self.slot.lock().unwrap_or_else(|e| e.into_inner());
        match s.take() {
            Some(Ok(t)) => Some(t),
            Some(Err(e)) => {
                *s = Some(Err(e));
                None
            }
            None => None,
        }
    }

    /// Why it could not be made, if it failed.
    pub fn failed(&self) -> Option<String> {
        let s = self.slot.lock().unwrap_or_else(|e| e.into_inner());
        match &*s {
            Some(Err(e)) => Some(e.clone()),
            _ => None,
        }
    }
}

impl ArchiSimApp {
    /// The model shown, shared with the threads that draw pictures (a copy
    /// made once per change).
    pub(super) fn shared_model(&mut self) -> Arc<archisim_core::Model> {
        let revision = self.ed.revision();
        match &self.shared_model {
            Some((r, m)) if *r == revision => m.clone(),
            _ => {
                let m = Arc::new((*self.ed.shown()).clone());
                self.shared_model = Some((revision, m.clone()));
                m
            }
        }
    }
}
