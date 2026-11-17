//! What each part of the 3D takes (`docs/TERRITORIO.md` §4): the parts of
//! [`crate::solid::model_solids`] add their time here, on the thread that
//! makes them, and whoever measures (`archisim territorio-grande`, the
//! application's frame meter) takes them out.

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::time::Duration;

thread_local! {
    static PARTS: RefCell<BTreeMap<&'static str, Duration>> = const { RefCell::new(BTreeMap::new()) };
}

/// Runs `f`, adding the time it takes to `part`. In the browser, where
/// there is no clock to ask, it only runs it.
pub fn time<T>(part: &'static str, f: impl FnOnce() -> T) -> T {
    #[cfg(target_arch = "wasm32")]
    {
        let _ = part;
        f()
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        let started = std::time::Instant::now();
        let out = f();
        let took = started.elapsed();
        PARTS.with(|p| *p.borrow_mut().entry(part).or_default() += took);
        out
    }
}

/// The parts measured on this thread since the last call, slowest first,
/// in milliseconds.
pub fn take() -> Vec<(&'static str, f64)> {
    let parts = PARTS.with(|p| std::mem::take(&mut *p.borrow_mut()));
    let mut v: Vec<(&'static str, f64)> = parts
        .into_iter()
        .map(|(k, d)| (k, d.as_secs_f64() * 1000.0))
        .collect();
    v.sort_by(|a, b| b.1.total_cmp(&a.1));
    v
}

/// Adds parts measured on another thread (a worker of
/// [`crate::parallel::map`]) to this thread's.
pub fn add(parts: Vec<(&'static str, Duration)>) {
    PARTS.with(|p| {
        let mut p = p.borrow_mut();
        for (k, d) in parts {
            *p.entry(k).or_default() += d;
        }
    });
}

/// The parts measured on this thread, taken out as they are.
pub fn take_raw() -> Vec<(&'static str, Duration)> {
    PARTS.with(|p| std::mem::take(&mut *p.borrow_mut()).into_iter().collect())
}

/// A part measured from now until it is dropped (see [`time`]).
pub struct Span {
    part: &'static str,
    #[cfg(not(target_arch = "wasm32"))]
    started: std::time::Instant,
}

/// Starts measuring `part`; it is added when the span is dropped.
pub fn span(part: &'static str) -> Span {
    Span {
        part,
        #[cfg(not(target_arch = "wasm32"))]
        started: std::time::Instant::now(),
    }
}

impl Drop for Span {
    fn drop(&mut self) {
        #[cfg(not(target_arch = "wasm32"))]
        {
            let took = self.started.elapsed();
            PARTS.with(|p| *p.borrow_mut().entry(self.part).or_default() += took);
        }
        #[cfg(target_arch = "wasm32")]
        let _ = self.part;
    }
}
