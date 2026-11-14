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
    slot: Arc<Mutex<Option<T>>>,
}

impl<K, T: Send + 'static> Pending<K, T> {
    /// Starts making the picture of `key` with `work`, on another thread.
    pub fn start(key: K, work: impl FnOnce() -> T + Send + 'static) -> Self {
        let slot = Arc::new(Mutex::new(None));
        let to = slot.clone();
        crate::platform::spawn(move || {
            let image = work();
            if let Ok(mut s) = to.lock() {
                *s = Some(image);
            }
        });
        Pending { key, slot }
    }

    /// The picture, once made.
    pub fn take(&self) -> Option<T> {
        self.slot.lock().ok().and_then(|mut s| s.take())
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
