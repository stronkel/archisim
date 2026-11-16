//! Work shared out over the computer's cores (`docs/TERRITORIO.md` §4,
//! step 5): the pieces of a job that do not depend on each other, such as
//! the tiles of the ground. In the browser, one after the other.

/// `f` of each item, in their order, made on as many threads as there are
/// cores (each takes the next item left).
pub fn map<T: Sync, U: Send>(items: &[T], f: impl Fn(&T) -> U + Sync) -> Vec<U> {
    #[cfg(target_arch = "wasm32")]
    {
        items.iter().map(f).collect()
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        let workers = std::thread::available_parallelism()
            .map_or(1, |n| n.get())
            .min(items.len());
        if workers <= 1 {
            return items.iter().map(f).collect();
        }
        let next = std::sync::atomic::AtomicUsize::new(0);
        let mut done: Vec<(usize, U)> = std::thread::scope(|scope| {
            let handles: Vec<_> = (0..workers)
                .map(|_| {
                    scope.spawn(|| {
                        let mut mine = Vec::new();
                        loop {
                            let k = next.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                            let Some(item) = items.get(k) else {
                                break;
                            };
                            mine.push((k, f(item)));
                        }
                        mine
                    })
                })
                .collect();
            handles
                .into_iter()
                .flat_map(|h| h.join().expect("a worker failed"))
                .collect()
        });
        done.sort_by_key(|(k, _)| *k);
        done.into_iter().map(|(_, u)| u).collect()
    }
}
