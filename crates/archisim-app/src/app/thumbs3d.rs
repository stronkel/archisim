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
pub const SIZE: Vec2 = Vec2::new(80.0, 58.0);

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

    /// The picture for `key`, drawn from the model `build` gives (with the
    /// piece alone) the first time it is asked for, if the graphics card
    /// is there and this frame has room for one more.
    pub fn get(
        &mut self,
        ctx: &egui::Context,
        gpu: Option<&eframe::egui_wgpu::RenderState>,
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
        if self.cache.len() > 600 {
            // Many themes seen: keep the ones shown since the last time.
            let used = std::mem::take(&mut self.used);
            self.cache.retain(|k, _| used.contains(k));
        }
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
    if scene.opaque.is_empty() && scene.glass.is_empty() {
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
    r.render(device, queue, &camera, px, 1);
    r.read_pixels(device, queue)
}

impl super::ArchiSimApp {
    /// The 3D picture of the piece on a card of the Maqueta's catalog (the
    /// project's types, the library's, the collection's), if it is ready.
    pub(super) fn card_thumb_3d(
        &mut self,
        ctx: &egui::Context,
        act: super::maquette_ui::CatalogAct,
    ) -> Option<Thumb> {
        use super::maquette_ui::CatalogAct;
        use super::object_gallery::ObjectPick;
        let sig = self.picture_signature(false);
        let gpu = self.gpu.as_ref();
        match act {
            CatalogAct::Object(ObjectPick::Project(t)) => {
                let model = self.ed.model();
                let ty = model.object_type(t)?;
                let key = format!("p:{}:{:?}:{sig}", ty.family.0, ty.values);
                self.thumbs3d.get(ctx, gpu, key, || {
                    crate::type_preview::object_model(model, ty)
                })
            }
            CatalogAct::Object(ObjectPick::Library(name)) => {
                let lib = self
                    .object_library
                    .as_ref()
                    .filter(|l| l.object_types().any(|t| t.name == name))
                    .or(self.flora_library.as_deref())?;
                let ty = lib.object_types().find(|t| t.name == name)?;
                self.thumbs3d.get(ctx, gpu, format!("l:{name}"), || {
                    crate::type_preview::object_model(lib, ty)
                })
            }
            CatalogAct::Collected(name) => {
                let c = self.collection?;
                let lib = &self
                    .collection_libraries
                    .iter()
                    .find(|(n, _)| *n == c.name)?
                    .1;
                let variant = archisim_library::collections::variant_name(name, c);
                let ty = lib.object_types().find(|t| t.name == variant)?;
                self.thumbs3d
                    .get(ctx, gpu, format!("c:{}:{name}", c.name), || {
                        crate::type_preview::object_model(lib, ty)
                    })
            }
            CatalogAct::Stair(c) => {
                let key = format!("stair:{c:?}");
                self.thumbs3d.get(ctx, gpu, key, || stair_model(c))
            }
            _ => None,
        }
    }
}

/// A small model with one stair of a catalog card, alone.
pub fn stair_model(c: super::maquette_ui::StairCard) -> Option<(Model, LevelId)> {
    use archisim_core::{NewStair, Point2, StairPreset, StairUse, Vec2, WallTop};
    let mut m = archisim_library::new_project("Escalera");
    let level = m.levels().first()?.id;
    let mut s = crate::editor::default_stair_settings(&m);
    s.preset = if c.preset == StairPreset::Free {
        StairPreset::LShape
    } else {
        c.preset
    };
    s.winders = c.winders;
    s.structure = c.structure;
    s.closed_risers = c.structure == archisim_core::StairStructure::Slab;
    s.top = crate::editor::TopChoice::Unconnected;
    // Short, so that the picture shows the steps (a spiral, its turn).
    s.height = if c.preset == StairPreset::Spiral {
        2.8
    } else {
        1.6
    };
    if c.preset == StairPreset::Spiral {
        s.width = 0.8;
        s.going = 0.24;
        s.core = 0.12;
    }
    // Wooden treads on the stairs that are not of concrete.
    if c.structure != archisim_core::StairStructure::Slab {
        s.tread_material = m.materials().find(|x| x.category == "wood").map(|x| x.id);
        s.material = m
            .materials()
            .find(|x| x.category == "steel")
            .map_or(s.material, |x| x.id);
    }
    let (path, design) = s.preset_on(&m, level, Point2::new(0.0, 0.0), Vec2::new(1.0, 0.0))?;
    m.add_stair(NewStair {
        level,
        path,
        width: s.width,
        going: s.going,
        base_offset: 0.0,
        top: WallTop::Unconnected,
        height: s.height,
        waist: s.waist,
        material: s.material,
        usage: StairUse::Restricted,
        design,
    })
    .ok()?;
    // Without railings: at the card's size they hide the steps.
    let id = m.stairs().next()?.id;
    m.edit_stair(id, |s| s.railings = archisim_core::Railings::None)
        .ok()?;
    Some((m, level))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every kind of stair on the catalog has its picture's model.
    #[test]
    fn every_stair_card_has_a_model() {
        use archisim_core::{StairPreset as P, StairStructure as S};
        for preset in [P::Straight, P::LShape, P::UShape, P::Spiral, P::Free] {
            for winders in [false, true] {
                for structure in [S::Slab, S::Stringers, S::Spine, S::Cantilever, S::Newel] {
                    let c = crate::app::maquette_ui::StairCard {
                        preset,
                        winders,
                        structure,
                    };
                    let (m, _) = stair_model(c).unwrap_or_else(|| panic!("{c:?}"));
                    assert_eq!(m.stairs().count(), 1);
                }
            }
        }
    }

    /// A sofa of the library drawn alone fills its picture, on a
    /// transparent background.
    #[test]
    fn a_piece_in_3d() {
        let Some((device, queue, view_formats)) = crate::view3d::gpu_tests::gpu() else {
            eprintln!("No graphics adapter: 3D drawing test skipped.");
            return;
        };
        let lib = archisim_library::new_project("Biblioteca");
        let t = lib
            .object_types()
            .find(|t| t.name == "Sofá acogedor de 3 plazas")
            .unwrap();
        let (model, _) = crate::type_preview::object_model(&lib, t).unwrap();
        let mut r = Renderer::new(&device, view_formats, true);
        let (w, h, px) = draw(&mut r, &device, &queue, &model, [160, 116]).unwrap();
        assert_eq!([w, h], [160, 116]);
        let px = px.as_chunks::<4>().0;
        let opaque = px.iter().filter(|p| p[3] > 200).count();
        let clear = px.iter().filter(|p| p[3] == 0).count();
        assert!(opaque > 3000, "the sofa fills the picture: {opaque}");
        assert!(clear > 2000, "the background is left clear: {clear}");
        // Its corners are background.
        assert_eq!(px[0][3], 0);
    }
}
