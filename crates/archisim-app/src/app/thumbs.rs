//! Pictures of types for the properties palette and its type gallery:
//! the plan cut of a piece of wall or the band of a slab, and a door or
//! window in a piece of wall, drawn by the plan writer at 1:20 like the
//! previews of the types dialog. Kept while they are shown.

use std::collections::{HashMap, HashSet};

use archisim_core::{LevelId, Model};
use eframe::egui::{self, TextureHandle, TextureId, TextureOptions, Vec2};

use crate::type_preview::draw;

/// Drawing density of the pictures, in screen points per metre.
const POINTS_PER_M: f64 = 460.0;

/// Pictures kept although not shown (switching selections is instant).
const KEEP: usize = 64;

/// A picture ready to paint: its texture and its natural size in points.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Thumb {
    pub tex: TextureId,
    pub size: Vec2,
}

impl Thumb {
    /// The size that fits in `max`, keeping proportions (never enlarged).
    pub fn fit(&self, max: Vec2) -> Vec2 {
        let k = (max.x / self.size.x).min(max.y / self.size.y).min(1.0);
        self.size * k
    }
}

#[derive(Default)]
pub struct Thumbs {
    cache: HashMap<String, (TextureHandle, Vec2)>,
    used: HashSet<String>,
}

impl Thumbs {
    /// The picture for `key`, drawn from the model `build` gives the first
    /// time it is asked for.
    pub fn get(
        &mut self,
        ctx: &egui::Context,
        key: String,
        build: impl FnOnce() -> Option<(Model, LevelId)>,
    ) -> Option<Thumb> {
        if let Some((tex, size)) = self.cache.get(&key) {
            self.used.insert(key);
            return Some(Thumb {
                tex: tex.id(),
                size: *size,
            });
        }
        let (model, level) = build()?;
        let ppp = ctx.pixels_per_point() as f64;
        let (img, lo, hi) = draw(&model, level, POINTS_PER_M * ppp);
        let size = Vec2::new(
            ((hi.x - lo.x) * POINTS_PER_M) as f32,
            ((hi.y - lo.y) * POINTS_PER_M) as f32,
        );
        let tex = ctx.load_texture(format!("thumb:{key}"), img, TextureOptions::LINEAR);
        let thumb = Thumb {
            tex: tex.id(),
            size,
        };
        self.used.insert(key.clone());
        self.cache.insert(key, (tex, size));
        Some(thumb)
    }

    /// Keeps the cache small: past `KEEP` pictures, forgets those not
    /// shown in this frame.
    pub fn end_frame(&mut self) {
        let used = std::mem::take(&mut self.used);
        if self.cache.len() > KEEP {
            self.cache.retain(|k, _| used.contains(k));
        }
    }

    #[cfg(test)]
    pub fn count(&self) -> usize {
        self.cache.len()
    }
}
