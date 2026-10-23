//! Pictures in 3D for the Maqueta's catalog (as Paralives' cards): each
//! piece drawn by the graphics card from the front and a little from
//! above, with its colours, textures and the sun's light, its background
//! left transparent. Made a few each frame (the others keep their plan
//! drawing meanwhile) and kept while they are seen.

use std::collections::{HashMap, HashSet};

use archisim_core::{LevelId, Model};
use eframe::egui::{self, ColorImage, TextureHandle, TextureOptions, Vec2};
use eframe::wgpu;

use super::thumbs::Thumb;
use crate::view3d::camera::Camera;
use crate::view3d::look::Quality;
use crate::view3d::render::Renderer;
use crate::view3d::scene::Scene;

/// Pictures made in one frame, at most.
const PER_FRAME: usize = 4;
/// The picture's size, in points (drawn at twice the screen's pixels).
pub const SIZE: Vec2 = Vec2::new(84.0, 66.0);

#[derive(Default)]
pub struct Thumbs3d {
    renderer: Option<Renderer>,
    cache: HashMap<String, (TextureHandle, Vec2)>,
    used: HashSet<String>,
    /// Pictures made this frame.
    made: usize,
    /// Some were left for the next frames.
    pub pending: bool,
}

impl Thumbs3d {
    /// Starts a frame: a few more pictures may be made.
    pub fn begin(&mut self) {
        self.made = 0;
        self.pending = false;
    }

    /// Forgets the pictures not asked for since the last call.
    pub fn sweep(&mut self) {
        let used = std::mem::take(&mut self.used);
        self.cache.retain(|k, _| used.contains(k));
    }

    /// The picture for `key`, drawn from the model `build` gives (with the
    /// piece alone) the first time it is asked for, if the graphics card
    /// is there and this frame has room for one more.
    pub fn get(
        &mut self,
        ctx: &egui::Context,
        gpu: Option<&egui_wgpu::RenderState>,
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
        let gpu = gpu?;
        if crate::crash::GPU_FAILED.load(std::sync::atomic::Ordering::Relaxed) {
            return None;
        }
        if self.made >= PER_FRAME {
            self.pending = true;
            return None;
        }
        self.made += 1;
        let (model, _) = build()?;
        let ppp = ctx.pixels_per_point();
        let px = [
            (SIZE.x * ppp * 2.0).round() as u32,
            (SIZE.y * ppp * 2.0).round() as u32,
        ];
        let renderer = self.renderer.get_or_insert_with(|| {
            Renderer::new(
                &gpu.device,
                gpu.adapter
                    .get_downlevel_capabilities()
                    .flags
                    .contains(wgpu::DownlevelFlags::VIEW_FORMATS),
                super::view3d_ui::default_textures(),
            )
        });
        let (w, h, rgba) = draw(renderer, &gpu.device, &gpu.queue, &model, px)?;
        let img = ColorImage::from_rgba_premultiplied([w as usize, h as usize], &rgba);
        let tex = ctx.load_texture(format!("thumb3d:{key}"), img, TextureOptions::LINEAR);
        let size = SIZE;
        let id = tex.id();
        self.cache.insert(key.clone(), (tex, size));
        self.used.insert(key);
        Some(Thumb { tex: id, size })
    }
}

/// The camera a piece is seen with in its picture: from the front and to
/// its right, a little from above, close enough to fill it.
pub fn camera_for(lo: glam::Vec3, hi: glam::Vec3, aspect: f32) -> Camera {
    let mut c = Camera {
        target: (lo + hi) / 2.0,
        // In front of the piece (it looks to +y) and to its right.
        yaw: 62f32.to_radians(),
        pitch: 24f32.to_radians(),
        distance: 1.0,
        perspective: true,
    };
    let radius = ((hi - lo).length() / 2.0).max(0.05);
    let half = (crate::view3d::camera::FOV_Y / 2.0).tan() * aspect.min(1.0);
    c.distance = radius / half.atan().sin() * 0.95;
    c
}

/// Draws a model alone, with a transparent background, as RGBA bytes
/// (premultiplied).
pub fn draw(
    r: &mut Renderer,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    model: &Model,
    px: [u32; 2],
) -> Option<(u32, u32, Vec<u8>)> {
    let scene = Scene::build(model, &Default::default());
    if scene.is_blank() {
        return None;
    }
    let camera = camera_for(scene.lo, scene.hi, px[0] as f32 / px[1] as f32);
    r.set_scene(device, &scene);
    r.set_clip(None);
    r.set_cut(None);
    r.set_atmosphere(None);
    r.set_transparent(true);
    r.set_look(
        Quality::Normal,
        Some(glam::Vec3::new(0.35, 0.75, 0.55).normalize()),
    );
    r.set_shadow_focus(None);
    r.render(device, queue, &camera, px, 2);
    r.read_pixels(device, queue)
}
