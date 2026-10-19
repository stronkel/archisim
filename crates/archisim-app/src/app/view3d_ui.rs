//! The 3D view tab: navigation, picking and drawing with the graphics card.
//!
//! Mouse, as in Revit and ArchiCAD: drag with the left button to orbit
//! (or Shift + wheel button), drag with the wheel or the right button to
//! pan, roll the wheel to zoom where the cursor is, click to select.

use std::collections::BTreeSet;

use eframe::egui::{
    self, Align2, Color32, CornerRadius, CursorIcon, FontId, PointerButton, Pos2, Rect, Sense,
    Stroke, Ui, Vec2,
};
use eframe::egui_wgpu::{self, wgpu};

use super::ArchiSimApp;
use crate::pick::Element;
use crate::view3d::camera::Camera;
use crate::view3d::render::Renderer;
use crate::view3d::scene::Scene;

/// The picture is drawn at twice the screen resolution and scaled down,
/// which smooths edges (supersampling).
const SUPERSAMPLE: f32 = 2.0;

/// Which drawing the central area shows.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ViewMode {
    Plan,
    ThreeD,
}

/// Everything the 3D view keeps between frames.
pub struct View3dState {
    pub camera: Camera,
    pub fit_requested: bool,
    scene: Scene,
    /// What the scene was built from: model revision and selection.
    scene_key: Option<(u64, BTreeSet<Element>)>,
    gpu: Option<Gpu>,
    /// What the last picture was drawn with.
    drawn: Option<(Camera, [u32; 2], u64)>,
    scene_serial: u64,
    /// Screen area of the 3D view in the last frame.
    pub canvas: Rect,
    /// Last time the picture took to draw, in milliseconds.
    pub last_ms: f64,
}

impl Default for View3dState {
    fn default() -> Self {
        Self {
            camera: Camera::default(),
            fit_requested: true,
            scene: Scene::default(),
            scene_key: None,
            gpu: None,
            drawn: None,
            scene_serial: 0,
            canvas: Rect::NOTHING,
            last_ms: 0.0,
        }
    }
}

struct Gpu {
    state: egui_wgpu::RenderState,
    renderer: Renderer,
    texture: Option<egui::TextureId>,
    uploaded: u64,
}

impl View3dState {
    /// Forgets the scene, for a project just opened.
    pub fn reset(&mut self) {
        self.scene_key = None;
        self.fit_requested = true;
    }

    /// Builds the scene again if the model or the selection changed.
    fn refresh_scene(
        &mut self,
        revision: u64,
        selection: &BTreeSet<Element>,
        model: &archisim_core::Model,
    ) {
        let key = (revision, selection.clone());
        if self.scene_key.as_ref() != Some(&key) {
            self.scene = Scene::build(model, selection);
            self.scene_key = Some(key);
            self.scene_serial += 1;
        }
    }

    /// The element under a point of the view, if any.
    pub fn pick(&self, pos: Vec2, size: Vec2) -> Option<Element> {
        let ray = self.camera.ray(
            glam::Vec2::new(pos.x, pos.y),
            glam::Vec2::new(size.x, size.y),
        );
        self.scene.pick(ray)
    }

    /// Draws the picture if anything changed and returns its texture.
    fn draw(
        &mut self,
        gpu_state: Option<&egui_wgpu::RenderState>,
        size: [u32; 2],
    ) -> Option<egui::TextureId> {
        let state = gpu_state?;
        let gpu = self.gpu.get_or_insert_with(|| Gpu {
            renderer: Renderer::new(
                &state.device,
                state
                    .adapter
                    .get_downlevel_capabilities()
                    .flags
                    .contains(wgpu::DownlevelFlags::VIEW_FORMATS),
            ),
            state: state.clone(),
            texture: None,
            uploaded: 0,
        });
        if gpu.uploaded != self.scene_serial {
            gpu.renderer.set_scene(&gpu.state.device, &self.scene);
            gpu.uploaded = self.scene_serial;
        }
        let wanted = (self.camera, size, self.scene_serial);
        if self.drawn != Some(wanted) || gpu.texture.is_none() {
            let started = std::time::Instant::now();
            gpu.renderer.render(
                &gpu.state.device,
                &gpu.state.queue,
                &self.camera,
                size,
                SUPERSAMPLE as u32,
            );
            let view = gpu.renderer.display_view()?;
            let mut egui_renderer = gpu.state.renderer.write();
            match gpu.texture {
                Some(id) => egui_renderer.update_egui_texture_from_wgpu_texture(
                    &gpu.state.device,
                    &view,
                    wgpu::FilterMode::Linear,
                    id,
                ),
                None => {
                    gpu.texture = Some(egui_renderer.register_native_texture(
                        &gpu.state.device,
                        &view,
                        wgpu::FilterMode::Linear,
                    ));
                }
            }
            self.drawn = Some(wanted);
            self.last_ms = started.elapsed().as_secs_f64() * 1000.0;
        }
        gpu.texture
    }

    /// A small 3D view of a model of its own, as the preview of the types
    /// dialog: drag to orbit, wheel to zoom, double click to frame it
    /// again. `key` changes when the model does; `build` makes it then.
    pub fn preview(
        &mut self,
        ui: &mut Ui,
        gpu_state: Option<&egui_wgpu::RenderState>,
        key: u64,
        build: impl FnOnce() -> Option<archisim_core::Model>,
        size: Vec2,
    ) {
        let (rect, response) = ui.allocate_exact_size(size, Sense::click_and_drag());
        let painter = ui.painter_at(rect);
        let wanted = Some((key, BTreeSet::new()));
        if self.scene_key != wanted {
            self.scene =
                build().map_or_else(Scene::default, |m| Scene::build(&m, &BTreeSet::new()));
            self.scene_key = wanted;
            self.scene_serial += 1;
            self.fit_requested = true;
        }
        if self.fit_requested && self.scene.lo.x.is_finite() {
            self.fit_requested = false;
            self.camera
                .fit(self.scene.lo, self.scene.hi, size.x / size.y);
        }
        let delta = response.drag_delta();
        if response.dragged_by(PointerButton::Primary) {
            self.camera.orbit(glam::Vec2::new(delta.x, delta.y));
        } else if response.dragged_by(PointerButton::Middle)
            || response.dragged_by(PointerButton::Secondary)
        {
            self.camera.pan(glam::Vec2::new(delta.x, delta.y), size.y);
        }
        if response.double_clicked() {
            self.fit_requested = true;
        }
        if let Some(pos) = response.hover_pos() {
            let (scroll, zoom) = ui.ctx().input(|i| (i.smooth_scroll_delta, i.zoom_delta()));
            let factor = (-scroll.y / 300.0).exp() / zoom;
            if (factor - 1.0).abs() > 1e-6 {
                let local = pos - rect.min;
                self.camera.zoom_at(
                    factor,
                    glam::Vec2::new(local.x, local.y),
                    glam::Vec2::new(size.x, size.y),
                );
            }
        }
        let ppp = ui.ctx().pixels_per_point();
        let px = [
            ((size.x * ppp * SUPERSAMPLE).round() as u32).clamp(1, 4096),
            ((size.y * ppp * SUPERSAMPLE).round() as u32).clamp(1, 4096),
        ];
        match self.draw(gpu_state, px) {
            Some(texture) => {
                painter.image(
                    texture,
                    rect,
                    Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)),
                    Color32::WHITE,
                );
            }
            None => {
                painter.rect_filled(
                    rect,
                    CornerRadius::same(4),
                    Color32::from_rgb(225, 229, 236),
                );
                painter.text(
                    rect.center(),
                    Align2::CENTER_CENTER,
                    "La vista 3D necesita la tarjeta gráfica",
                    FontId::proportional(12.0),
                    Color32::from_gray(60),
                );
            }
        }
        response
            .on_hover_text("Arrastra para girar, rueda para acercar, doble clic para encuadrar");
    }

    /// The last picture as RGBA, to save it.
    pub fn picture(&self) -> Option<(u32, u32, Vec<u8>)> {
        let gpu = self.gpu.as_ref()?;
        gpu.renderer
            .read_pixels(&gpu.state.device, &gpu.state.queue)
    }
}

impl ArchiSimApp {
    /// Switches between the plan and the 3D view.
    pub(super) fn set_mode(&mut self, mode: super::ViewMode) {
        match mode {
            ViewMode::Plan => self.show_plan(),
            ViewMode::ThreeD => self.open_view(super::TabKind::ThreeD),
        }
    }

    pub(super) fn view_3d(&mut self, ui: &mut Ui) {
        let (rect, response) = ui.allocate_exact_size(ui.available_size(), Sense::click_and_drag());
        let ctx = ui.ctx().clone();
        let painter = ui.painter_at(rect);
        let v = &mut self.v3d;
        v.canvas = rect;
        v.refresh_scene(self.ed.revision(), &self.ed.selection, self.ed.model());
        let size = rect.size();
        if v.fit_requested && size.x > 50.0 && size.y > 50.0 {
            v.fit_requested = false;
            v.camera.fit(v.scene.lo, v.scene.hi, size.x / size.y);
        }

        // Navigation.
        let shift = ctx.input(|i| i.modifiers.shift);
        let delta = response.drag_delta();
        let delta = glam::Vec2::new(delta.x, delta.y);
        if response.dragged_by(PointerButton::Primary)
            || (shift && response.dragged_by(PointerButton::Middle))
        {
            v.camera.orbit(delta);
        } else if response.dragged_by(PointerButton::Middle)
            || response.dragged_by(PointerButton::Secondary)
        {
            v.camera.pan(delta, size.y);
        }
        if response.double_clicked_by(PointerButton::Middle) {
            v.fit_requested = true;
        }
        if let Some(pos) = response.hover_pos() {
            let (scroll, zoom) = ctx.input(|i| (i.smooth_scroll_delta, i.zoom_delta()));
            let factor = (-scroll.y / 300.0).exp() / zoom;
            if (factor - 1.0).abs() > 1e-6 {
                let local = pos - rect.min;
                v.camera.zoom_at(
                    factor,
                    glam::Vec2::new(local.x, local.y),
                    glam::Vec2::new(size.x, size.y),
                );
            }
        }

        // Selection: a click picks what is under the cursor.
        if response.clicked_by(PointerButton::Primary) {
            if let Some(pos) = response.interact_pointer_pos() {
                let hit = v.pick(pos - rect.min, size);
                let add = ctx.input(|i| i.modifiers.shift || i.modifiers.command);
                match (hit, add) {
                    (Some(e), true) => {
                        if !self.ed.selection.remove(&e) {
                            self.ed.selection.insert(e);
                        }
                    }
                    (Some(e), false) => {
                        self.ed.selection.clear();
                        self.ed.selection.insert(e);
                    }
                    (None, false) => self.ed.selection.clear(),
                    (None, true) => {}
                }
            }
        }
        let v = &mut self.v3d;
        v.refresh_scene(self.ed.revision(), &self.ed.selection, self.ed.model());

        // The picture.
        let ppp = ctx.pixels_per_point();
        let px = [
            ((size.x * ppp * SUPERSAMPLE).round() as u32).clamp(1, 8192),
            ((size.y * ppp * SUPERSAMPLE).round() as u32).clamp(1, 8192),
        ];
        match v.draw(self.gpu.as_ref(), px) {
            Some(texture) => {
                painter.image(
                    texture,
                    rect,
                    Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)),
                    Color32::WHITE,
                );
            }
            None => {
                painter.rect_filled(rect, CornerRadius::ZERO, Color32::from_rgb(225, 229, 236));
                painter.text(
                    rect.center(),
                    Align2::CENTER_CENTER,
                    "La vista 3D necesita la tarjeta gráfica (Vulkan, DirectX 12 o Metal),\n\
                     que no está disponible en este equipo.",
                    FontId::proportional(14.0),
                    Color32::from_gray(60),
                );
            }
        }
        draw_north(&painter, rect, &v.camera);
        self.draw_overlays(&ctx, &painter, rect);

        if response.hovered() {
            let icon = if response.dragged() {
                CursorIcon::Grabbing
            } else {
                CursorIcon::Default
            };
            ctx.set_cursor_icon(icon);
        }
    }
}

/// A north arrow in the corner, turning with the camera.
fn draw_north(painter: &egui::Painter, rect: Rect, camera: &Camera) {
    let c = rect.right_top() + Vec2::new(-34.0, 34.0);
    // North (+Y) on the screen: along screen-right it is cos(yaw), along
    // screen-up -sin(yaw)·sin(pitch); egui's Y grows downwards.
    let (sy, cy) = camera.yaw.sin_cos();
    let dir = Vec2::new(cy, sy * camera.pitch.sin());
    let len = dir.length();
    let dir = if len > 1e-3 {
        dir / len
    } else {
        Vec2::new(0.0, -1.0)
    };
    let ink = Color32::from_gray(50);
    painter.circle_filled(c, 20.0, Color32::from_white_alpha(170));
    painter.circle_stroke(c, 20.0, Stroke::new(1.0, Color32::from_gray(150)));
    let tip = c + dir * 15.0;
    let side = Vec2::new(-dir.y, dir.x) * 5.0;
    painter.add(egui::Shape::convex_polygon(
        vec![tip, c - dir * 6.0 + side, c - dir * 6.0 - side],
        ink,
        Stroke::NONE,
    ));
    painter.text(
        c - dir * 13.0,
        Align2::CENTER_CENTER,
        "N",
        FontId::proportional(10.0),
        ink,
    );
}
