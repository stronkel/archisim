//! The pictures of the plan and of the sections on screen, made so the
//! window never waits for them (0.85, «que todo responda al instante»):
//!
//! - each picture covers a margin around what is seen, so moving the view
//!   only moves it and nothing is drawn again until the margin runs out;
//! - a new picture (another zoom, a change in the model) is made on another
//!   thread while the one before, moved and scaled, stays on screen;
//! - first a quick one at half the resolution (fewer details, in a fraction
//!   of the time), then the full one;
//! - only when a picture is cheap (its last one took little) is it drawn at
//!   once, so a small project stays as crisp as before.

use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use eframe::egui::{self, ColorImage, TextureHandle, TextureOptions};

use super::background::Pending;
use archisim_core::Point2;

/// How much more than the view each picture covers, on each side, as a
/// share of the view.
pub const MARGIN: f64 = 0.2;
/// The biggest picture, in pixels a side.
pub const MAX_SIDE: usize = 4096;
/// A picture is drawn at once (not on another thread) when the last one
/// took less than this.
/// (The tests draw everything at once, but the ones that try the other
/// way: `ArchiSimApp::at_once`.)
pub const AT_ONCE: Duration = Duration::from_millis(60);

/// Whether a drawing that took `last` the time before is made on another
/// thread. Not knowing yet (the first time), a big model's is.
pub fn aside(last: Duration, at_once: Duration, model: &archisim_core::Model) -> bool {
    if last.is_zero() {
        at_once <= AT_ONCE && big(model)
    } else {
        last >= at_once
    }
}

/// A model whose drawings take time: hundreds of elements.
pub fn big(model: &archisim_core::Model) -> bool {
    model.walls().count()
        + model.slabs().count()
        + model.objects().count()
        + model.openings().count()
        > 600
}

/// What a picture is of: the region and the size, and the rest (what is
/// drawn and how), which must be the same to reuse it moved.
pub trait Region: Clone + PartialEq + Send + 'static {
    fn lo(&self) -> Point2;
    fn hi(&self) -> Point2;
    fn size(&self) -> [usize; 2];
    /// The same key for another region and size.
    fn with_view(&self, lo: Point2, hi: Point2, size: [usize; 2]) -> Self;
    /// Whether it is the same drawing, of whatever region.
    fn same_drawing(&self, other: &Self) -> bool {
        self.with_view(other.lo(), other.hi(), other.size()) == *other
    }
    /// Pixels per metre.
    fn density(&self) -> f64 {
        self.size()[0] as f64 / (self.hi().x - self.lo().x).max(1e-9)
    }
}

impl Region for crate::plan_image::PlanKey {
    fn lo(&self) -> Point2 {
        self.lo
    }
    fn hi(&self) -> Point2 {
        self.hi
    }
    fn size(&self) -> [usize; 2] {
        self.size
    }
    fn with_view(&self, lo: Point2, hi: Point2, size: [usize; 2]) -> Self {
        crate::plan_image::PlanKey {
            lo,
            hi,
            size,
            ..self.clone()
        }
    }
}

/// A picture on screen and the ones being made.
pub struct Picture<K: Region> {
    name: &'static str,
    shown: Option<(K, TextureHandle)>,
    /// Whether what is shown is the quick one.
    rough: bool,
    full: Option<Pending<K>>,
    quick: Option<Pending<K>>,
    /// How long the last full picture took (written by its thread).
    took: Arc<Mutex<Duration>>,
    /// The drawing changed: what is shown is the one before, shown until
    /// the new one is ready.
    stale: bool,
}

/// Makes the picture of a key.
pub type Maker<K> = Box<dyn Fn(K) -> ColorImage + Send + 'static>;

impl<K: Region> Picture<K> {
    pub fn new(name: &'static str) -> Self {
        Picture {
            name,
            shown: None,
            rough: false,
            full: None,
            quick: None,
            took: Arc::new(Mutex::new(Duration::ZERO)),
            stale: false,
        }
    }

    /// The drawing changed (the model, for a picture whose key does not
    /// say it): what is shown stays until the new one is ready.
    pub fn outdated(&mut self) {
        self.stale = true;
        self.full = None;
        self.quick = None;
    }

    /// Forgets what is shown (another project).
    pub fn clear(&mut self) {
        *self = Picture::new(self.name);
    }

    /// The key and texture of what is shown.
    #[cfg(test)]
    pub fn shown(&self) -> Option<&(K, TextureHandle)> {
        self.shown.as_ref()
    }

    /// Whether a new picture is being made.
    pub fn busy(&self) -> bool {
        self.full.is_some()
    }

    /// How long the last full picture took.
    pub fn last(&self) -> Duration {
        self.took.lock().map(|t| *t).unwrap_or_default()
    }

    /// The picture wanted for `view` (the key of exactly what is seen, at
    /// the screen's pixels): the same with its margin, as big as allowed.
    pub fn wanted(view: &K) -> K {
        let (lo, hi) = (view.lo(), view.hi());
        let (w, h) = (hi.x - lo.x, hi.y - lo.y);
        let [sw, sh] = view.size();
        let grow = 1.0 + 2.0 * MARGIN;
        let (mut pw, mut ph) = (sw as f64 * grow, sh as f64 * grow);
        let fit = (MAX_SIDE as f64 / pw.max(ph)).min(1.0);
        pw *= fit;
        ph *= fit;
        // The margin shrinks if the picture would be too big.
        let (mx, my) = (
            ((pw / sw as f64) - 1.0).max(0.0) / 2.0 * w,
            ((ph / sh as f64) - 1.0).max(0.0) / 2.0 * h,
        );
        view.with_view(
            Point2::new(lo.x - mx, lo.y - my),
            Point2::new(hi.x + mx, hi.y + my),
            [pw.round().max(1.0) as usize, ph.round().max(1.0) as usize],
        )
    }

    /// Whether `shown` serves `view` as it is: the same drawing, covering
    /// all of it, at (nearly) the same pixels per metre.
    fn serves(shown: &K, view: &K) -> bool {
        shown.same_drawing(view)
            && shown.lo().x <= view.lo().x + 1e-9
            && shown.lo().y <= view.lo().y + 1e-9
            && shown.hi().x >= view.hi().x - 1e-9
            && shown.hi().y >= view.hi().y - 1e-9
            && (shown.density() / view.density() - 1.0).abs() < 0.02
    }

    /// Keeps the picture for `view` up to date and returns the one to
    /// paint (its key says where it goes). `moving`: the view is being
    /// moved, so nothing new is started. `make()` gives what makes a
    /// picture; it runs on another thread unless pictures are cheap.
    pub fn update(
        &mut self,
        ctx: &egui::Context,
        view: &K,
        moving: bool,
        at_once: Duration,
        mut make: impl FnMut() -> Maker<K>,
    ) -> Option<&(K, TextureHandle)> {
        // Finished pictures.
        if let Some(image) = self.full.as_ref().and_then(|p| p.take()) {
            if let Some(key) = self.full.take().map(|p| p.key) {
                self.put(ctx, key, image);
                self.rough = false;
                self.stale = false;
                self.quick = None;
            }
        }
        if let Some(image) = self.quick.as_ref().and_then(|p| p.take()) {
            if let Some(key) = self.quick.take().map(|p| p.key) {
                if self.full.is_some() && key.same_drawing(view) {
                    self.put(ctx, key, image);
                    self.rough = true;
                    self.stale = false;
                }
            }
        }
        let fine = self
            .shown
            .as_ref()
            .is_some_and(|(k, _)| !self.stale && !self.rough && Self::serves(k, view));
        if fine {
            return self.shown.as_ref();
        }
        let coming = self
            .full
            .as_ref()
            .is_some_and(|p| Self::serves(&p.key, view));
        if !coming {
            let nothing = self.shown.is_none();
            let want = Self::wanted(view);
            if self.last() < at_once && (nothing || !moving) {
                // Cheap pictures: at once, as before.
                let started = Instant::now();
                let image = make()(want.clone());
                if let Ok(mut t) = self.took.lock() {
                    *t = started.elapsed();
                }
                self.put(ctx, want, image);
                self.rough = false;
                self.stale = false;
                self.full = None;
                self.quick = None;
                return self.shown.as_ref();
            }
            // One picture made aside at a time: another is started once
            // it is done (moving on, the one on the way is shown first).
            if (!moving || nothing) && self.full.is_none() {
                // The quick one too when there is nothing near this zoom
                // to show meanwhile.
                let far = self.shown.as_ref().is_none_or(|(k, _)| {
                    let r = k.density() / view.density();
                    !(0.6..=1.67).contains(&r)
                });
                let work = make();
                let took = self.took.clone();
                let w = want.clone();
                self.full = Some(Pending::start(want.clone(), move || {
                    let started = Instant::now();
                    let image = work(w);
                    if let Ok(mut t) = took.lock() {
                        *t = started.elapsed();
                    }
                    image
                }));
                self.quick = None;
                if far {
                    let work = make();
                    let [w, h] = want.size();
                    let small =
                        want.with_view(want.lo(), want.hi(), [(w / 2).max(1), (h / 2).max(1)]);
                    self.quick = Some(Pending::start(small.clone(), move || work(small)));
                }
            }
        }
        ctx.request_repaint_after(Duration::from_millis(30));
        self.shown.as_ref()
    }

    fn put(&mut self, ctx: &egui::Context, key: K, image: ColorImage) {
        match &mut self.shown {
            Some((k, tex)) => {
                tex.set(image, TextureOptions::LINEAR);
                *k = key;
            }
            None => {
                let tex = ctx.load_texture(self.name, image, TextureOptions::LINEAR);
                self.shown = Some((key, tex));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Clone, PartialEq, Debug)]
    struct K {
        lo: Point2,
        hi: Point2,
        size: [usize; 2],
        what: u32,
    }

    impl Region for K {
        fn lo(&self) -> Point2 {
            self.lo
        }
        fn hi(&self) -> Point2 {
            self.hi
        }
        fn size(&self) -> [usize; 2] {
            self.size
        }
        fn with_view(&self, lo: Point2, hi: Point2, size: [usize; 2]) -> Self {
            K {
                lo,
                hi,
                size,
                ..self.clone()
            }
        }
    }

    #[test]
    fn a_picture_covers_a_margin_and_serves_while_it_covers() {
        let p = Point2::new;
        let view = K {
            lo: p(0.0, 0.0),
            hi: p(10.0, 5.0),
            size: [1000, 500],
            what: 1,
        };
        let w = Picture::<K>::wanted(&view);
        assert!(
            (w.lo.x + 2.0).abs() < 1e-9 && (w.hi.y - 6.0).abs() < 1e-9,
            "{w:?}"
        );
        assert_eq!(w.size, [1400, 700]);
        assert!((w.density() - view.density()).abs() < 1e-9);
        // Moved a metre: still served. Moved four: not. Zoomed: not.
        let moved = |dx: f64| view.with_view(p(dx, 0.0), p(10.0 + dx, 5.0), view.size);
        assert!(Picture::<K>::serves(&w, &moved(1.0)));
        assert!(!Picture::<K>::serves(&w, &moved(4.0)));
        let zoomed = view.with_view(p(2.0, 1.0), p(8.0, 4.0), view.size);
        assert!(!Picture::<K>::serves(&w, &zoomed));
        // Another drawing: not.
        let other = K {
            what: 2,
            ..view.clone()
        };
        assert!(!Picture::<K>::serves(&w, &other));
    }
}
