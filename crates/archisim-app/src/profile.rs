//! What each part of a frame takes, for the frame-time meter
//! (Ctrl+Alt+F) and for measuring big projects
//! (`app::tests::big_project_frames`).

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::time::{Duration, Instant};

thread_local! {
    static PARTS: RefCell<BTreeMap<&'static str, (Duration, u32)>> = const { RefCell::new(BTreeMap::new()) };
}

/// Runs `f`, adding the time it takes to `part`.
pub fn time<T>(part: &'static str, f: impl FnOnce() -> T) -> T {
    let started = Instant::now();
    let out = f();
    let took = started.elapsed();
    PARTS.with(|p| {
        let mut p = p.borrow_mut();
        let e = p.entry(part).or_default();
        e.0 += took;
        e.1 += 1;
    });
    out
}

/// The parts measured since the last call, slowest first: name and
/// milliseconds per frame over `frames` frames.
pub fn take(frames: u32) -> Vec<(&'static str, f64)> {
    let parts = PARTS.with(|p| std::mem::take(&mut *p.borrow_mut()));
    let mut v: Vec<(&'static str, f64)> = parts
        .into_iter()
        .map(|(k, (d, _))| (k, d.as_secs_f64() * 1000.0 / frames.max(1) as f64))
        .collect();
    v.sort_by(|a, b| b.1.total_cmp(&a.1));
    v
}
