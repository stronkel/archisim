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
use crate::view3d::scene::{LinkedPart, Owner, Scene};
use crate::view3d::section_box::{Face, SectionBox};

/// The picture is drawn at twice the screen resolution and scaled down,
/// which smooths edges (supersampling).
const SUPERSAMPLE: f32 = 2.0;

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
    drawn: Option<(Camera, [u32; 2], u64, Option<SectionBox>)>,
    scene_serial: u64,
    /// Screen area of the 3D view in the last frame.
    pub canvas: Rect,
    /// Last time the picture took to draw, in milliseconds.
    pub last_ms: f64,
    /// The section box, when it is on.
    pub section: Option<SectionBox>,
    /// The last section box used, to switch it back on where it was.
    pub section_saved: Option<SectionBox>,
    /// The face of the section box being dragged, and where the drag was.
    section_drag: Option<Face>,
    /// The handle under the cursor.
    section_hover: Option<Face>,
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
            section_saved: None,
            section_drag: None,
            section_hover: None,
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
        }
    }

    /// The element (of the project or linked) under a point of the view.
    pub fn pick(&self, pos: Vec2, size: Vec2) -> Option<Owner> {
        let ray = self.camera.ray(
            glam::Vec2::new(pos.x, pos.y),
            glam::Vec2::new(size.x, size.y),
        );
        self.scene.pick_in(ray, self.section.as_ref())
    }

    /// What is under a point of the view: the point hit, its triangle's
    /// normal and what it belongs to.
    pub fn hit(&self, pos: Vec2, size: Vec2) -> Option<(glam::Vec3, glam::Vec3, Owner)> {
        let ray = self.camera.ray(
            glam::Vec2::new(pos.x, pos.y),
            glam::Vec2::new(size.x, size.y),
        );
        let (d, i) = self.scene.hit_in(ray, self.section.as_ref())?;
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
        let wanted = (self.camera, size, self.scene_serial, self.section);
        if self.drawn != Some(wanted) || gpu.texture.is_none() {
            let started = std::time::Instant::now();
            gpu.renderer.set_clip(self.section.map(|b| (b.lo, b.hi)));
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
            ViewMode::Section | ViewMode::Schedule => {}
            ViewMode::Sheet(s) => self.open_view(super::TabKind::Sheet(s)),
        }
    }

    pub(super) fn view_3d(&mut self, ui: &mut Ui) {
        let (rect, response) = ui.allocate_exact_size(ui.available_size(), Sense::click_and_drag());
        let ctx = ui.ctx().clone();
        let painter = ui.painter_at(rect);
        let v = &mut self.v3d;
        v.canvas = rect;
        let disciplines = crate::view3d::scene::Disciplines {
            focus: self.screen_focus(),
            hidden: self.hidden_disciplines.clone(),
        };
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

        // Navigation.
        let shift = ctx.input(|i| i.modifiers.shift);
        let dragging_box = v.section_drag.is_some();
        if (response.dragged_by(PointerButton::Primary) && !dragging_box)
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

        // A tool in use draws and places here as in the plan (R14).
        let tool3d = self.tool_in_3d();
        if tool3d {
            self.tools_in_3d(&ctx, &response, rect);
        } else {
            self.cursor3d = None;
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
        let v = &mut self.v3d;
        v.refresh_scene(
            self.ed.revision(),
            &self.ed.selection,
            &self.ed.shown(),
            self.phase_view,
            &self.coord,
            &disciplines,
        );

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
        draw_section_box(&painter, rect, v);
        self.draw_edges_3d(&painter, rect);
        if tool3d {
            self.draw_tool_3d(&painter, rect);
        }
        let v = &mut self.v3d;
        draw_north(
            &painter,
            rect,
            &v.camera,
            self.ed.model().true_north() as f32,
        );
        let on_handle = v.section_hover.is_some() || v.section_drag.is_some();
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
                    ui.checkbox(&mut on, "");
                },
            );
            if on != self.v3d.section.is_some() {
                self.v3d.toggle_section();
            }
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
