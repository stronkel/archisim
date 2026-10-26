//! The 3D view tab: navigation, picking and drawing with the graphics card.
//!
//! Mouse, as in Revit and ArchiCAD: drag with the left button to orbit
//! (or Shift + wheel button), drag with the wheel or the right button to
//! pan, roll the wheel to zoom where the cursor is, click to select.

use super::widgets::SwitchExt as _;
use std::collections::BTreeSet;

use eframe::egui::{
    self, Align2, Color32, CornerRadius, CursorIcon, FontId, PointerButton, Pos2, Rect, Sense,
    Stroke, Ui, Vec2,
};
use eframe::egui_wgpu::{self, wgpu};

use super::ArchiSimApp;
use crate::pick::Element;
use crate::view3d::camera::Camera;
use crate::view3d::cutaway::{CutRule, Front};
use crate::view3d::look::Quality;
use crate::view3d::render::{CapStyle, Renderer, TextureMode};
use crate::view3d::scene::{LinkedPart, Owner, Scene};
use crate::view3d::section_box::{Face, SectionBox};

/// The picture is drawn at twice the screen resolution and scaled down,
/// which smooths edges (supersampling) — on ordinary screens. A Retina
/// screen already has two pixels a point: doubling again made pictures of
/// 5000 × 3000 pixels with four samples each, hundreds of megabytes of the
/// graphics card's memory, for no visible gain.
fn supersample(ppp: f32) -> f32 {
    if ppp >= 1.5 {
        1.0
    } else {
        2.0
    }
}

/// The largest side of the picture, in pixels.
const MAX_SIDE: f32 = 4096.0;

/// The picture's size for a view of `size` points: at most [`MAX_SIDE`]
/// on its longer side, keeping its shape.
fn picture_size(size: Vec2, ppp: f32, quality: Quality) -> ([u32; 2], u32) {
    let ss = if quality.supersample() {
        supersample(ppp)
    } else {
        1.0
    };
    let w = size.x * ppp * ss;
    let h = size.y * ppp * ss;
    let k = (MAX_SIDE / w.max(h).max(1.0)).min(1.0);
    (
        [
            ((w * k).round() as u32).max(1),
            ((h * k).round() as u32).max(1),
        ],
        ss as u32,
    )
}

/// Which drawing the central area shows.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ViewMode {
    Plan,
    ThreeD,
    /// A section or elevation.
    Section,
    /// A sheet (paper with views on it).
    Sheet(archisim_core::SheetId),
    /// A schedule or the measurements, as a table.
    Schedule,
}

/// How much of the walls the 3D view shows (Los Sims' «walls up / cut
/// away / down»): the building above a height over the active level is
/// cut away, with its cut faces filled, so the inside of a floor is seen.
/// The furniture is never cut: it stays whole, or goes if it is upstairs
/// (`crate::view3d::cutaway`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Cutaway {
    /// Everything.
    #[default]
    Up,
    /// The walls between the camera and what is looked at cut low, the
    /// others whole; the floors above and the roof gone. It follows the
    /// camera as it turns.
    Front,
    /// Cut 1.20 m above the level: walls at sill height, the floors
    /// above and the roof gone.
    Cut,
    /// Cut 30 cm above the level: the floor plan in 3D.
    Low,
}

/// Height the walls in front are cut at, over the level.
const FRONT_LOW: f32 = 0.3;

impl Cutaway {
    pub const ALL: [Cutaway; 4] = [Cutaway::Up, Cutaway::Front, Cutaway::Cut, Cutaway::Low];

    pub fn name(self) -> &'static str {
        match self {
            Cutaway::Up => "Muros arriba",
            Cutaway::Front => "Muros delante bajos",
            Cutaway::Cut => "Muros recortados",
            Cutaway::Low => "Muros bajos",
        }
    }

    /// Height of the cut above the level.
    pub fn height(self) -> Option<f32> {
        match self {
            Cutaway::Up => None,
            Cutaway::Front | Cutaway::Cut => Some(1.2),
            Cutaway::Low => Some(0.3),
        }
    }
}

/// What a picture was drawn with: camera, size, scene, section box, cut
/// away walls and whether its background was left transparent.
type Drawn = (
    Camera,
    [u32; 2],
    u64,
    Option<SectionBox>,
    Option<CutRule>,
    bool,
    u64,
    (Quality, Option<[i32; 3]>, bool, CapStyle),
    Option<crate::view3d::look::Atmosphere>,
);

/// Everything the 3D view keeps between frames.
pub struct View3dState {
    pub camera: Camera,
    pub fit_requested: bool,
    scene: Scene,
    /// What the scene was built from: model revision, selection, the
    /// phases shown and the state of the linked models.
    scene_key: Option<(
        u64,
        BTreeSet<Element>,
        archisim_plan::phases::PhaseView,
        u64,
        crate::view3d::scene::Disciplines,
    )>,
    gpu: Option<Gpu>,
    /// What the last picture was drawn with.
    drawn: Option<Drawn>,
    /// Walls up, cut away or low.
    pub cutaway: Cutaway,
    /// Height of the active level, for the cut away walls.
    pub level_z: f32,
    /// The picture's background is left transparent: the ground grid is
    /// drawn under it.
    pub under: bool,
    scene_serial: u64,
    /// Screen area of the 3D view in the last frame.
    pub canvas: Rect,
    /// Last time the picture took to draw, in milliseconds.
    pub last_ms: f64,
    /// The section box, when it is on.
    pub section: Option<SectionBox>,
    /// What the cuts show: each material's cut colour or a flat colour.
    pub caps: CapStyle,
    /// The last section box used, to switch it back on where it was.
    pub section_saved: Option<SectionBox>,
    /// The face of the section box being dragged, and where the drag was.
    section_drag: Option<Face>,
    /// The handle under the cursor.
    section_hover: Option<Face>,
    /// Pixels a point of the picture (2: supersampled), for its edges.
    edge_step: u32,
    /// Contours one pixel wide (the app's "Líneas finas").
    pub thin_lines: bool,
    /// How the materials' textures (brick, wood, stone…) are drawn.
    pub textures: TextureMode,
    /// What is being stretched, as it will be, drawn in place of the
    /// selected element while the handle is dragged: what it was built
    /// for, and its triangles.
    overlay: Option<(String, Scene)>,
    overlay_serial: u64,
    /// How the picture looks: its quality (shadows…).
    pub quality: Quality,
    /// Towards the model's sun (none: below the horizon).
    sun: Option<glam::Vec3>,
    /// The sky, the haze and the grid on the ground (the Maqueta's; none:
    /// the plain background).
    pub atmosphere: Option<crate::view3d::look::Atmosphere>,
    /// The light the Maqueta is seen with: day, evening or night.
    pub light: crate::view3d::look::Light,
}

/// Full textures, except on Intel Macs: their AMD graphics drivers (on
/// recent macOS, often patched) hang drawing them, so they look them up in
/// pictures made once (C4; before, the light ones).
pub fn default_textures() -> TextureMode {
    if cfg!(all(target_os = "macos", target_arch = "x86_64")) {
        TextureMode::Pictures
    } else {
        TextureMode::Full
    }
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
            section: None,
            caps: CapStyle::Layers,
            section_saved: None,
            section_drag: None,
            section_hover: None,
            edge_step: 2,
            thin_lines: false,
            textures: default_textures(),
            cutaway: Cutaway::Up,
            level_z: 0.0,
            under: false,
            overlay: None,
            overlay_serial: 0,
            quality: crate::view3d::look::default_quality(),
            sun: None,
            atmosphere: None,
            light: Default::default(),
        }
    }
}

struct Gpu {
    state: egui_wgpu::RenderState,
    renderer: Renderer,
    texture: Option<egui::TextureId>,
    uploaded: u64,
    overlay_uploaded: u64,
}

impl View3dState {
    /// The section box, if on: it cuts everything, furniture too.
    pub fn clip_box(&self) -> Option<SectionBox> {
        self.section
    }

    /// The cut away walls, if on: what they take away depends on each
    /// element (`CutRule::top_of`), and with walls in front on the
    /// camera.
    pub fn cut_rule(&self) -> Option<CutRule> {
        let top = self.level_z + self.cutaway.height()?;
        let front = (self.cutaway == Cutaway::Front).then(|| {
            let eye = self.camera.eye();
            let t = self.camera.target;
            let mut toward = glam::Vec2::new(eye.x - t.x, eye.y - t.y);
            if toward.length_squared() < 1e-8 {
                // Straight down: the south side.
                toward = glam::Vec2::new(0.0, -1.0);
            }
            let toward = toward.normalize();
            Front {
                toward,
                at: toward.dot(glam::Vec2::new(t.x, t.y)),
                low: self.level_z + FRONT_LOW,
                above: self.level_z + 0.5,
            }
        });
        Some(CutRule { top, front })
    }

    /// A 3D view with its textures drawn so and its quality.
    pub fn with_look(textures: TextureMode, quality: Quality) -> Self {
        Self {
            textures,
            quality,
            ..Self::default()
        }
    }

    /// Changes how the textures are drawn: the shaders are made again.
    pub fn set_textures(&mut self, on: TextureMode) {
        if on == self.textures {
            return;
        }
        self.textures = on;
        if let Some(g) = self.gpu.take() {
            if let Some(id) = g.texture {
                g.state.renderer.write().free_texture(&id);
            }
        }
        self.drawn = None;
    }

    /// Draws `make()` in place of the selected element (what it is built
    /// for is `key`: it is made again only when that changes), or nothing.
    pub fn set_overlay(&mut self, key: Option<String>, make: impl FnOnce() -> Option<Scene>) {
        match key {
            None => {
                if self.overlay.take().is_some() {
                    self.overlay_serial += 1;
                }
            }
            Some(k) => {
                if self.overlay.as_ref().is_some_and(|(o, _)| *o == k) {
                    return;
                }
                self.overlay = make().map(|s| (k, s));
                self.overlay_serial += 1;
            }
        }
    }

    /// Whether something is drawn in place of the selected element.
    #[cfg(test)]
    pub fn has_overlay(&self) -> bool {
        self.overlay.is_some()
    }

    /// Forgets the scene, for a project just opened.
    pub fn reset(&mut self) {
        self.scene_key = None;
        self.fit_requested = true;
    }

    /// Builds the scene again if the model, the selection or the linked
    /// models changed.
    fn refresh_scene(
        &mut self,
        revision: u64,
        selection: &BTreeSet<Element>,
        model: &archisim_core::Model,
        phases: archisim_plan::phases::PhaseView,
        coord: &crate::coord::Coord,
        disciplines: &crate::view3d::scene::Disciplines,
    ) {
        let key = (
            revision,
            selection.clone(),
            phases,
            coord.serial,
            disciplines.clone(),
        );
        if self.scene_key.as_ref() != Some(&key) {
            let parts = coord.parts();
            let linked: Vec<LinkedPart> = parts
                .iter()
                .map(|p| LinkedPart {
                    triangles: &p.element.mesh.triangles,
                    color: p.color,
                    alpha: p.alpha,
                    link: p.link,
                    index: p.index,
                })
                .collect();
            let shown = archisim_plan::phases::phase_model(model, phases);
            self.scene = Scene::build_with(
                shown.as_ref().unwrap_or(model),
                selection,
                &linked,
                coord.isolate,
                disciplines,
            );
            self.scene_key = Some(key);
            self.scene_serial += 1;
            self.sun = model
                .sun_direction()
                .map(|d| glam::Vec3::new(d[0] as f32, d[1] as f32, d[2] as f32));
        }
    }

    /// The element (of the project or linked) under a point of the view.
    pub fn pick(&self, pos: Vec2, size: Vec2) -> Option<Owner> {
        let ray = self.camera.ray(
            glam::Vec2::new(pos.x, pos.y),
            glam::Vec2::new(size.x, size.y),
        );
        self.scene
            .pick_in(ray, self.clip_box().as_ref(), self.cut_rule().as_ref())
    }

    /// What is under a point of the view: the point hit, its triangle's
    /// normal and what it belongs to.
    pub fn hit(&self, pos: Vec2, size: Vec2) -> Option<(glam::Vec3, glam::Vec3, Owner)> {
        let ray = self.camera.ray(
            glam::Vec2::new(pos.x, pos.y),
            glam::Vec2::new(size.x, size.y),
        );
        let (d, i) = self
            .scene
            .hit_in(ray, self.clip_box().as_ref(), self.cut_rule().as_ref())?;
        let [a, b, c] = self.scene.triangles().nth(i)?;
        let mut n = (b - a).cross(c - a).normalize_or_zero();
        if n.dot(ray.dir) > 0.0 {
            n = -n;
        }
        Some((ray.origin + ray.dir * d, n, self.scene.owner(i)?))
    }

    /// The corner or edge midpoint of an element nearest to a point of
    /// the view, within `reach` points.
    pub fn snap_vertex(
        &self,
        owner: Owner,
        pos: Vec2,
        size: Vec2,
        reach: f32,
    ) -> Option<(glam::Vec3, bool)> {
        let gsize = glam::Vec2::new(size.x, size.y);
        let p = glam::Vec2::new(pos.x, pos.y);
        let mut best: Option<(f32, glam::Vec3, bool)> = None;
        for [a, b, c] in self.scene.triangles_of(owner) {
            for (q, corner) in [
                (a, true),
                (b, true),
                (c, true),
                ((a + b) / 2.0, false),
                ((b + c) / 2.0, false),
                ((c + a) / 2.0, false),
            ] {
                if let Some(s) = self.camera.project_to_screen(q, gsize) {
                    // Corners win over midpoints at the same distance.
                    let d = s.distance(p) + if corner { 0.0 } else { 2.0 };
                    if d <= reach && best.is_none_or(|(bd, ..)| d < bd) {
                        best = Some((d, q, corner));
                    }
                }
            }
        }
        best.map(|(_, q, c)| (q, c))
    }

    /// The box around the whole model.
    pub fn model_bounds(&self) -> (glam::Vec3, glam::Vec3) {
        (self.scene.lo, self.scene.hi)
    }

    /// The box around each element of the project in the scene.
    pub fn element_bounds(&self) -> std::collections::BTreeMap<Element, (glam::Vec3, glam::Vec3)> {
        self.scene.element_bounds()
    }

    /// The box around some elements, if they have any 3D.
    pub fn bounds_of(&self, elements: &BTreeSet<Element>) -> Option<(glam::Vec3, glam::Vec3)> {
        self.scene.bounds_of(elements)
    }

    /// Switches the section box on (where it last was, or around the whole
    /// model) or off.
    pub fn toggle_section(&mut self) {
        match self.section.take() {
            Some(b) => self.section_saved = Some(b),
            None => {
                self.section = Some(
                    self.section_saved
                        .unwrap_or_else(|| SectionBox::around(self.scene.lo, self.scene.hi)),
                );
            }
        }
    }

    /// Draws the picture if anything changed and returns its texture.
    fn draw(
        &mut self,
        gpu_state: Option<&egui_wgpu::RenderState>,
        size: [u32; 2],
    ) -> Option<egui::TextureId> {
        // After an error of the graphics card nothing more is drawn (it is
        // written in errores.log): better a message than a crash.
        if crate::crash::GPU_FAILED.load(std::sync::atomic::Ordering::Relaxed) {
            return None;
        }
        let state = gpu_state?;
        let clip = self.clip_box();
        let cut = self.cut_rule();
        if self.gpu.is_none() {
            crate::crash::stage("3D: preparar los sombreadores de la tarjeta gráfica");
        }
        // With OpenGL (older cards, Linux without Vulkan), the pictures'
        // texture is not seen: the light textures instead.
        let textures = match self.textures {
            TextureMode::Pictures if state.adapter.get_info().backend == wgpu::Backend::Gl => {
                TextureMode::Light
            }
            t => t,
        };
        let gpu = self.gpu.get_or_insert_with(|| Gpu {
            renderer: Renderer::new(
                &state.device,
                state
                    .adapter
                    .get_downlevel_capabilities()
                    .flags
                    .contains(wgpu::DownlevelFlags::VIEW_FORMATS),
                textures,
            ),
            state: state.clone(),
            texture: None,
            uploaded: 0,
            overlay_uploaded: 0,
        });
        if gpu.uploaded != self.scene_serial {
            crate::crash::stage("3D: subir la escena a la tarjeta gráfica");
            gpu.renderer.set_scene(&gpu.state.device, &self.scene);
            gpu.uploaded = self.scene_serial;
        }
        if gpu.overlay_uploaded != self.overlay_serial {
            gpu.renderer
                .set_overlay(&gpu.state.device, self.overlay.as_ref().map(|o| &o.1));
            gpu.overlay_uploaded = self.overlay_serial;
        }
        let wanted = (
            self.camera,
            size,
            self.scene_serial,
            clip,
            cut,
            self.under,
            self.overlay_serial,
            (
                self.quality,
                self.sun.map(|s| (s * 1000.0).round().as_ivec3().to_array()),
                self.thin_lines,
                self.caps,
            ),
            self.atmosphere.clone(),
        );
        if self.drawn.as_ref() != Some(&wanted) || gpu.texture.is_none() {
            let started = std::time::Instant::now();
            crate::crash::stage("3D: dibujar en la tarjeta gráfica");
            gpu.renderer.set_clip(clip.map(|b| (b.lo, b.hi)));
            gpu.renderer.set_caps(self.caps);
            gpu.renderer.set_cut(cut);
            gpu.renderer.set_look(self.quality, self.sun);
            gpu.renderer
                .set_shadow_focus(Some(shadow_focus(&self.camera)));
            gpu.renderer.set_transparent(self.under);
            gpu.renderer.set_atmosphere(self.atmosphere.clone());
            gpu.renderer.thin_lines = self.thin_lines;
            gpu.renderer.render(
                &gpu.state.device,
                &gpu.state.queue,
                &self.camera,
                size,
                self.edge_step,
            );
            crate::crash::stage("3D: pasar la imagen a la ventana");
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
        let wanted = Some((
            key,
            BTreeSet::new(),
            Default::default(),
            0,
            Default::default(),
        ));
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
        // Nothing to pick here: either button turns it, the wheel moves it.
        let delta = response.drag_delta();
        if response.dragged_by(PointerButton::Primary)
            || response.dragged_by(PointerButton::Secondary)
        {
            self.camera.orbit(glam::Vec2::new(delta.x, delta.y));
        } else if response.dragged_by(PointerButton::Middle) {
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
        let (px, ss) = picture_size(size, ui.ctx().pixels_per_point(), self.quality);
        self.edge_step = ss;
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
    /// The view drawn again `width` pixels wide (as it is on screen, its
    /// lines as thick for the size), read back as RGBA bytes. The next
    /// frame draws the screen's picture again.
    pub fn capture(&mut self, width: u32) -> Option<(u32, u32, Vec<u8>)> {
        let canvas = self.canvas;
        if canvas.width() < 1.0 || canvas.height() < 1.0 {
            return None;
        }
        let w = width.clamp(64, 8192);
        let h = ((w as f32 * canvas.height() / canvas.width()).round() as u32).clamp(1, 8192);
        let step = ((w as f32 / canvas.width()).round() as u32).clamp(1, 6);
        let clip = self.clip_box();
        let cut = self.cut_rule();
        let gpu = self.gpu.as_mut()?;
        gpu.renderer.set_clip(clip.map(|b| (b.lo, b.hi)));
        gpu.renderer.set_caps(self.caps);
        gpu.renderer.set_cut(cut);
        gpu.renderer.set_look(self.quality, self.sun);
        gpu.renderer
            .set_shadow_focus(Some(shadow_focus(&self.camera)));
        gpu.renderer.set_transparent(self.under);
        gpu.renderer.set_atmosphere(self.atmosphere.clone());
        gpu.renderer.thin_lines = self.thin_lines;
        gpu.renderer.render(
            &gpu.state.device,
            &gpu.state.queue,
            &self.camera,
            [w, h],
            step,
        );
        self.drawn = None;
        self.picture()
    }

    pub fn picture(&self) -> Option<(u32, u32, Vec<u8>)> {
        let gpu = self.gpu.as_ref()?;
        let (w, h, mut px) = gpu
            .renderer
            .read_pixels(&gpu.state.device, &gpu.state.queue)?;
        // Drawn over the ground grid (transparent, premultiplied): put the
        // background back.
        if self.under {
            for p in px.as_chunks_mut::<4>().0 {
                let a = f32::from(p[3]) / 255.0;
                for (c, bg) in p.iter_mut().take(3).zip([224.0_f32, 230.0, 237.0]) {
                    *c = (f32::from(*c) + bg * (1.0 - a)).round().min(255.0) as u8;
                }
                p[3] = 255;
            }
        }
        Some((w, h, px))
    }
}

impl ArchiSimApp {
    /// Switches between the plan and the 3D view.
    pub(super) fn set_mode(&mut self, mode: super::ViewMode) {
        match mode {
            ViewMode::Plan => self.show_plan(),
            ViewMode::ThreeD => self.open_view(super::TabKind::ThreeD),
            ViewMode::Section | ViewMode::Schedule => {}
            ViewMode::Sheet(s) => self.open_view(super::TabKind::Sheet(s)),
        }
    }

    pub(super) fn view_3d(&mut self, ui: &mut Ui) {
        let (rect, response) = ui.allocate_exact_size(ui.available_size(), Sense::click_and_drag());
        let ctx = ui.ctx().clone();
        let painter = ui.painter_at(rect);
        let level_z = self
            .ed
            .model()
            .level(self.ed.level)
            .map_or(0.0, |l| l.elevation) as f32;
        let v = &mut self.v3d;
        v.canvas = rect;
        v.level_z = level_z;
        let disciplines = crate::view3d::scene::Disciplines {
            focus: self.screen_focus(),
            hidden: self.hidden_disciplines.clone(),
        };
        crate::crash::stage("3D: preparar la escena");
        let v = &mut self.v3d;
        v.refresh_scene(
            self.ed.revision(),
            &self.ed.selection,
            &self.ed.shown(),
            self.phase_view,
            &self.coord,
            &disciplines,
        );
        let size = rect.size();
        if v.fit_requested && size.x > 50.0 && size.y > 50.0 {
            v.fit_requested = false;
            v.camera.fit(v.scene.lo, v.scene.hi, size.x / size.y);
        }

        // Section box handles: under the cursor, grabbed, dragged.
        let gsize = glam::Vec2::new(size.x, size.y);
        let handle_at = |v: &super::View3dState, pos: Pos2| -> Option<Face> {
            let b = v.section?;
            let p = glam::Vec2::new(pos.x - rect.min.x, pos.y - rect.min.y);
            Face::ALL
                .iter()
                .filter_map(|f| {
                    v.camera
                        .project_to_screen(b.handle(*f), gsize)
                        .map(|s| (*f, s.distance(p)))
                })
                .filter(|(_, d)| *d <= HANDLE_REACH)
                .min_by(|a, b| a.1.total_cmp(&b.1))
                .map(|(f, _)| f)
        };
        if v.section_drag.is_none() {
            v.section_hover = response.hover_pos().and_then(|p| handle_at(v, p));
        }
        if response.drag_started_by(PointerButton::Primary) {
            v.section_drag = ctx
                .input(|i| i.pointer.press_origin())
                .and_then(|p| handle_at(v, p));
        }
        let delta = response.drag_delta();
        let delta = glam::Vec2::new(delta.x, delta.y);
        if let (Some(f), Some(mut b)) = (v.section_drag, v.section) {
            if response.dragged_by(PointerButton::Primary) {
                let amount = b.drag_amount(f, delta, &v.camera, gsize);
                b.set_face(f, b.face(f) + amount);
                v.section = Some(b);
            }
        }
        if response.drag_stopped() {
            v.section_drag = None;
        }

        // Handles of the object selected (bloque 5): its size.
        let grabbing_object = self.object_grips_3d(&ctx, &response, rect)
            | self.mass_grips_3d(&ctx, &response, rect)
            | self.object_drag_3d(&ctx, &response, rect);
        // With something selected, the camera turns about it.
        let pivot = if self.ed.selection.is_empty() {
            None
        } else {
            self.v3d
                .scene
                .bounds_of(&self.ed.selection)
                .map(|(lo, hi)| (lo + hi) / 2.0)
        };
        let v = &mut self.v3d;

        // Navigation: the right button held turns the view (with Shift,
        // the wheel pressed too), the wheel pressed moves it.
        let shift = ctx.input(|i| i.modifiers.shift);
        let dragging_box = v.section_drag.is_some() || grabbing_object;
        if response.dragged_by(PointerButton::Secondary)
            || (shift && response.dragged_by(PointerButton::Middle))
        {
            match pivot {
                Some(p) => v.camera.orbit_about(delta, p),
                None => v.camera.orbit(delta),
            }
        } else if response.dragged_by(PointerButton::Middle) {
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

        // The left button picks and moves: a drag moves what is selected
        // or opens a selection window.
        if !dragging_box && !self.tool_in_3d() {
            self.left_drag_3d(&ctx, &response, rect);
        } else {
            self.drag3d = None;
        }

        // A tool in use draws and places here as in the plan (R14).
        let tool3d = self.tool_in_3d();
        if tool3d {
            self.tools_in_3d(&ctx, &response, rect);
        } else {
            self.cursor3d = None;
        }

        // A right click (not held: that turns the view) opens the menu of
        // what is under the cursor.
        if !tool3d && response.clicked_by(PointerButton::Secondary) {
            if let Some(pos) = response.interact_pointer_pos() {
                let e = self.element_at_3d(rect, pos).map(|h| h.0);
                let frame = ctx.cumulative_frame_nr();
                self.open_context_menu(pos, e, None, frame);
            }
        }
        // Selection: a click picks what is under the cursor.
        if !tool3d && response.clicked_by(PointerButton::Primary) {
            if let Some(pos) = response.interact_pointer_pos() {
                let owner = self.v3d.pick(pos - rect.min, size);
                let add = ctx.input(|i| i.modifiers.shift || i.modifiers.command);
                // A linked element: say what it is and mark it.
                if let Some(Owner::Linked(link, index)) = owner {
                    let said = self.coord.select_linked(link, index);
                    if let Some(text) = said {
                        self.say(text);
                    }
                } else {
                    self.coord.select_linked_none();
                }
                let hit = match owner {
                    // An element of a module's copy: the copy.
                    Some(Owner::Model(e)) => Some(
                        crate::pick::solid_of(e)
                            .and_then(|o| self.ed.model().module_of_flat_id(o.id()))
                            .map_or(e, Element::Module),
                    ),
                    _ => None,
                };
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
                    (None, false) => {
                        self.ed.selection.clear();
                        self.coord.set_isolate(false);
                    }
                    (None, true) => {}
                }
            }
        }
        let disciplines = crate::view3d::scene::Disciplines {
            focus: self.screen_focus(),
            hidden: self.hidden_disciplines.clone(),
        };
        crate::crash::stage("3D: preparar la escena");
        let v = &mut self.v3d;
        v.refresh_scene(
            self.ed.revision(),
            &self.ed.selection,
            &self.ed.shown(),
            self.phase_view,
            &self.coord,
            &disciplines,
        );

        // The picture, over the ground grid when it is on.
        let (px, ss) = picture_size(size, ctx.pixels_per_point(), v.quality);
        v.edge_step = ss;
        v.thin_lines = self.thin_lines;
        // In the Maqueta, the sky, the haze and the grid are drawn with the
        // picture; elsewhere the grid goes under it.
        if self.maquette {
            let step = super::maquette_ui::GRID_STEP as f32;
            let t = v.camera.target;
            v.atmosphere = Some(crate::view3d::look::Atmosphere {
                light: v.light,
                ground_z: v.level_z,
                grid: self.ground_grid.then(|| {
                    (
                        glam::Vec2::new((t.x / step).round() * step, (t.y / step).round() * step),
                        10.0_f32.max(v.camera.distance * 0.5).min(30.0),
                        step,
                    )
                }),
            });
            v.under = false;
        } else {
            v.atmosphere = None;
            v.under = self.ground_grid;
        }
        let under = v.under;
        let drawn = v.draw(self.gpu.as_ref(), px);
        if under && drawn.is_some() {
            painter.rect_filled(rect, CornerRadius::ZERO, Color32::from_rgb(224, 230, 237));
            self.draw_ground_grid(&painter, rect);
        }
        match drawn {
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
                    if crate::crash::GPU_FAILED.load(std::sync::atomic::Ordering::Relaxed) {
                        "La tarjeta gráfica dio un error al dibujar el 3D y la vista se ha detenido\n\
                         (está anotado en errores.log, en la carpeta de ajustes de ArchiSim).\n\
                         Guarda el proyecto y vuelve a abrir ArchiSim."
                    } else {
                        "La vista 3D necesita la tarjeta gráfica (Vulkan, DirectX 12 o Metal),\n\
                         que no está disponible en este equipo."
                    },
                    FontId::proportional(14.0),
                    Color32::from_gray(60),
                );
            }
        }
        let v = &self.v3d;
        draw_section_box(&painter, rect, v);
        self.draw_object_grips_3d(&painter, rect);
        self.draw_mass_grips_3d(&painter, rect);
        self.draw_drag_3d(&painter, rect);
        self.draw_clearances_3d(&painter, rect);
        self.draw_edges_3d(&painter, rect);
        if tool3d {
            self.draw_tool_3d(&painter, rect);
            self.draw_brush_3d(&painter, rect);
        }
        let v = &mut self.v3d;
        draw_north(
            &painter,
            rect,
            &v.camera,
            self.ed.model().true_north() as f32,
        );
        let on_handle = v.section_hover.is_some()
            || v.section_drag.is_some()
            || self.object_grip_hover.is_some()
            || self.object_grip.is_some()
            || self.mass_grip_hover.is_some()
            || self.mass_grip.is_some()
            || self.object_move.is_some();
        let dragging = response.dragged();
        self.draw_overlays(&ctx, &painter, rect);

        if response.hovered() {
            let icon = if dragging {
                CursorIcon::Grabbing
            } else if on_handle {
                CursorIcon::Grab
            } else if tool3d {
                CursorIcon::Crosshair
            } else {
                CursorIcon::Default
            };
            ctx.set_cursor_icon(icon);
        }
    }
}

/// How close to a handle of the section box the cursor grabs it, in points.
const HANDLE_REACH: f32 = 12.0;

/// The section box: its edges, and a handle on each face (an arrow out of
/// it), highlighted under the cursor.
fn draw_section_box(painter: &egui::Painter, rect: Rect, v: &super::View3dState) {
    let Some(b) = v.section else {
        return;
    };
    let size = glam::Vec2::new(rect.width(), rect.height());
    let at = |p: glam::Vec3| {
        v.camera
            .project_to_screen(p, size)
            .map(|s| rect.min + Vec2::new(s.x, s.y))
    };
    let ink = Color32::from_rgb(30, 110, 230);
    for (a, c) in b.edges() {
        if let (Some(a), Some(c)) = (at(a), at(c)) {
            painter.line_segment([a, c], Stroke::new(1.2, ink.gamma_multiply(0.8)));
        }
    }
    for f in Face::ALL {
        let h = b.handle(f);
        let (Some(p), Some(q)) = (at(h), at(h + f.normal() * 0.6)) else {
            continue;
        };
        let active = v.section_drag == Some(f) || v.section_hover == Some(f);
        let r = if active { 8.0 } else { 6.0 };
        let dir = (q - p).normalized();
        if dir.length() > 0.5 {
            let tip = p + dir * (r + 9.0);
            let side = Vec2::new(-dir.y, dir.x) * 5.0;
            painter.add(egui::Shape::convex_polygon(
                vec![tip, p + dir * r + side, p + dir * r - side],
                ink,
                Stroke::NONE,
            ));
        }
        painter.circle_filled(p, r, if active { ink } else { Color32::WHITE });
        painter.circle_stroke(p, r, Stroke::new(1.5, ink));
    }
}

/// A north arrow in the corner, turning with the camera, towards the
/// project's true north (`north`: radians clockwise from the plan's up).
fn draw_north(painter: &egui::Painter, rect: Rect, camera: &Camera, north: f32) {
    let c = rect.right_top() + Vec2::new(-34.0, 34.0);
    // North (+Y) on the screen: along screen-right it is cos(yaw), along
    // screen-up -sin(yaw)·sin(pitch); egui's Y grows downwards.
    let (sy, cy) = (camera.yaw + north).sin_cos();
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

impl ArchiSimApp {
    /// The section box in the 3D view's properties: on or off, its six
    /// faces in millimetres, and quick ways to place it.
    pub(super) fn section_box_group(&mut self, ui: &mut Ui) {
        use super::props::{group, row};
        group(ui, "view3d-section", "Caja de sección", |ui| {
            let mut on = self.v3d.section.is_some();
            row(
                ui,
                "Activa",
                "Recorta el modelo fuera de la caja (interiores, secciones en perspectiva)",
                |ui, _| {
                    ui.switch(&mut on, "");
                },
            );
            if on != self.v3d.section.is_some() {
                self.v3d.toggle_section();
            }
            row(
                ui,
                "Lo cortado",
                "Cómo se ve lo que corta la caja (y los muros recortados): macizo con sus capas, como el poché de la planta, o de un color liso",
                |ui, _| caps_choice(ui, &mut self.v3d.caps),
            );
            let Some(mut b) = self.v3d.section else {
                super::props::note(
                    ui,
                    "Actívala para recortar el modelo; aparecen flechas azules en sus caras para arrastrarlas.",
                );
                return;
            };
            let mut changed = false;
            for (axis, name) in [(0usize, "X"), (1, "Y"), (2, "Z")] {
                for high in [false, true] {
                    let f = Face { axis, high };
                    let label = format!("{name} {} (mm)", if high { "máx." } else { "mín." });
                    row(ui, &label, f.name(), |ui, w| {
                        if let Some(v) = super::length_field(
                            ui,
                            egui::Id::unique(("section-face", axis, high)),
                            b.face(f) as f64,
                            w - 8.0,
                        ) {
                            b.set_face(f, v as f32);
                            changed = true;
                        }
                    });
                }
            }
            if changed {
                self.v3d.section = Some(b);
            }
            ui.add_space(4.0);
            ui.horizontal_wrapped(|ui| {
                ui.add_space(4.0);
                if ui
                    .button("Todo el modelo")
                    .on_hover_text("La caja alrededor de todo el modelo")
                    .clicked()
                {
                    let (lo, hi) = self.v3d.model_bounds();
                    self.v3d.section = Some(SectionBox::around(lo, hi));
                }
                if ui
                    .button("Selección")
                    .on_hover_text("La caja alrededor de lo seleccionado (como «Caja de selección» de Revit)")
                    .clicked()
                {
                    match self.v3d.bounds_of(&self.ed.selection) {
                        Some((lo, hi)) => self.v3d.section = Some(SectionBox::around(lo, hi)),
                        None => self.complain("Selecciona antes algún elemento."),
                    }
                }
                if ui
                    .button("Planta activa")
                    .on_hover_text("De la planta activa a la de encima, en todo el modelo")
                    .clicked()
                {
                    let model = self.ed.model();
                    let z0 = model.level(self.ed.level).map_or(0.0, |l| l.elevation);
                    let z1 = model
                        .level_above(self.ed.level)
                        .and_then(|l| model.level(l))
                        .map_or(z0 + 3.0, |l| l.elevation);
                    let (lo, hi) = self.v3d.model_bounds();
                    let mut bx = SectionBox::around(lo, hi);
                    bx.lo.z = z0 as f32 - 0.05;
                    bx.hi.z = (z1 as f32 - 0.01).max(bx.lo.z + 0.1);
                    self.v3d.section = Some(bx);
                }
                if ui
                    .button("Planta en 3D")
                    .on_hover_text("Corta la planta activa a la altura de su plano de corte: una planta en perspectiva")
                    .clicked()
                {
                    let model = self.ed.model();
                    let z0 = model.level(self.ed.level).map_or(0.0, |l| l.elevation);
                    let (lo, hi) = self.v3d.model_bounds();
                    let mut bx = SectionBox::around(lo, hi);
                    bx.lo.z = z0 as f32 - 0.6;
                    bx.hi.z = (z0 + self.cut_plane) as f32;
                    self.v3d.section = Some(bx);
                }
            });
        });
    }
}

/// The part of the model the shadows are drawn for: a square around what
/// is looked at, as wide as the view from that far (and at least 16 m),
/// snapped to a grid of a quarter of its side so the shadows are not drawn
/// again at every move of the camera.
fn shadow_focus(camera: &Camera) -> (glam::Vec3, glam::Vec3) {
    let r = (camera.distance * 1.2).clamp(8.0, 300.0);
    // The side as a power of two, so a little zoom does not change it.
    let r = 2f32.powf(r.log2().ceil());
    let step = r / 4.0;
    let c = (camera.target / step).round() * step;
    (
        glam::Vec3::new(c.x - r, c.y - r, f32::MIN),
        glam::Vec3::new(c.x + r, c.y + r, f32::MAX),
    )
}

/// The choice of what a cut shows in 3D: «Capas», «Blanco», «Negro»,
/// «Gris» (in the section box group and the Maqueta's cut away menu).
pub(super) fn caps_choice(ui: &mut Ui, caps: &mut CapStyle) {
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing.x = 2.0;
        for (name, style, hint) in [
            (
                "Capas",
                CapStyle::Layers,
                "Cada capa del muro con su color de corte, como en planta",
            ),
            (
                "Blanco",
                CapStyle::Flat([255, 255, 255]),
                "Todo lo cortado en blanco",
            ),
            (
                "Negro",
                CapStyle::Flat([30, 30, 30]),
                "Todo lo cortado en negro, como un poché",
            ),
            (
                "Gris",
                CapStyle::Flat([150, 150, 150]),
                "Todo lo cortado en gris",
            ),
        ] {
            if ui
                .selectable_label(*caps == style, name)
                .on_hover_text(hint)
                .clicked()
            {
                *caps = style;
            }
        }
    });
}
