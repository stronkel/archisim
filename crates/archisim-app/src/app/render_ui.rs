//! Realistic pictures (bloque 7 de la revisión del 22/10/2026): «Renderizar»
//! in the 3D view's properties makes, on another thread, a picture of the
//! model from the camera of the view, with the project's sun, its shadows
//! and the light of the sky (`archisim_render`), and shows it in a window
//! with «Guardar PNG…».

use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, Mutex};

use eframe::egui::{self, TextureOptions};

use super::*;

/// How fine the picture is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum RenderQuality {
    /// Quick, to see the light: half the width, one ray a pixel.
    Draft,
    #[default]
    Normal,
    /// For a presentation: twice the width, nine rays a pixel.
    Fine,
}

impl RenderQuality {
    pub const ALL: [RenderQuality; 3] = [Self::Draft, Self::Normal, Self::Fine];

    pub fn label(self) -> &'static str {
        match self {
            Self::Draft => "Borrador",
            Self::Normal => "Normal",
            Self::Fine => "Alta",
        }
    }

    /// Width, rays along each side of a pixel and rays to the sky.
    fn numbers(self) -> (usize, usize, usize) {
        match self {
            Self::Draft => (800, 1, 6),
            Self::Normal => (1600, 2, 12),
            Self::Fine => (2800, 3, 16),
        }
    }
}

/// A picture being made, or made.
#[derive(Default)]
pub struct RenderState {
    pub quality: RenderQuality,
    pub open: bool,
    /// Thousandths done, written by the thread.
    progress: Arc<AtomicU32>,
    job: Option<Arc<Mutex<Option<archisim_render::Image>>>>,
    started: Option<std::time::Instant>,
    pub image: Option<archisim_render::Image>,
    seconds: f64,
    texture: Option<egui::TextureHandle>,
}

impl RenderState {
    pub fn busy(&self) -> bool {
        self.job.is_some()
    }
}

/// The renderer's view from the 3D view's camera, for a picture of
/// `aspect` (width over height).
pub fn view_of(camera: &crate::view3d::camera::Camera) -> archisim_render::View {
    let eye = camera.eye().as_dvec3();
    let target = camera.target.as_dvec3();
    let fov = crate::view3d::camera::FOV_Y as f64;
    archisim_render::View {
        eye: [eye.x, eye.y, eye.z],
        target: [target.x, target.y, target.z],
        fov_y: fov,
        perspective: camera.perspective,
        half_height: camera.distance as f64 * (fov / 2.0).tan(),
    }
}

impl ArchiSimApp {
    /// Starts a picture from the 3D view's camera, with the shape of the
    /// view.
    pub(super) fn start_render(&mut self) {
        if self.render.busy() {
            return;
        }
        let model: archisim_core::Model = (*self.ed.shown()).clone();
        let view = view_of(&self.v3d.camera);
        let (width, samples, sky_rays) = self.render.quality.numbers();
        let canvas = self.v3d.canvas;
        let aspect = if canvas.width() > 50.0 && canvas.height() > 50.0 {
            (canvas.width() / canvas.height()) as f64
        } else {
            1.6
        };
        let options = archisim_render::Options {
            width,
            height: ((width as f64 / aspect).round() as usize).max(16),
            samples,
            sky_rays,
            // The light and the textures of the 3D view.
            light: match self.v3d.light {
                crate::view3d::look::Light::Day => archisim_render::Light::Day,
                crate::view3d::look::Light::Evening => archisim_render::Light::Evening,
                crate::view3d::look::Light::Night => archisim_render::Light::Night,
            },
            textures: self.v3d.textures != crate::view3d::render::TextureMode::Off,
        };
        let slot = Arc::new(Mutex::new(None));
        let progress = Arc::new(AtomicU32::new(0));
        self.render.progress = progress.clone();
        self.render.job = Some(slot.clone());
        self.render.started = Some(std::time::Instant::now());
        self.render.open = true;
        std::thread::spawn(move || {
            let image = archisim_render::render(&model, &view, &options, &|f| {
                progress.store((f * 1000.0) as u32, Ordering::Relaxed);
            });
            if let Ok(mut s) = slot.lock() {
                *s = Some(image);
            }
        });
    }

    /// The window with the picture (or how far it has gone).
    pub(super) fn render_window(&mut self, ctx: &egui::Context) {
        // A picture finished?
        if let Some(slot) = &self.render.job {
            let done = slot.lock().ok().and_then(|mut s| s.take());
            match done {
                Some(image) => {
                    self.render.seconds = self
                        .render
                        .started
                        .map_or(0.0, |t| t.elapsed().as_secs_f64());
                    let tex = ctx.load_texture(
                        "render",
                        egui::ColorImage::from_rgb([image.width, image.height], &image.rgb),
                        TextureOptions::LINEAR,
                    );
                    self.render.texture = Some(tex);
                    self.render.image = Some(image);
                    self.render.job = None;
                }
                None => ctx.request_repaint_after(std::time::Duration::from_millis(150)),
            }
        }
        if !self.render.open {
            return;
        }
        let mut open = true;
        let mut again = false;
        let mut save = false;
        super::widgets::dialog("Renderizado")
            .open(&mut open)
            .pivot(egui::Align2::CENTER_CENTER)
            .default_pos(ctx.content_rect().center())
            .default_size([760.0, 520.0])
            .resizable(true)
            .collapsible(false)
            .show(ctx, |ui| {
                if self.render.busy() {
                    let f = self.render.progress.load(Ordering::Relaxed) as f32 / 1000.0;
                    let secs = self
                        .render
                        .started
                        .map_or(0.0, |t| t.elapsed().as_secs_f64());
                    ui.label(format!(
                        "Calculando la luz del sol y del cielo… {:.0} s",
                        secs
                    ));
                    ui.add(egui::ProgressBar::new(f).show_percentage());
                    ui.add_space(6.0);
                }
                if let (Some(tex), Some(img)) = (&self.render.texture, &self.render.image) {
                    let avail = ui.available_size() - egui::vec2(0.0, 34.0);
                    let s = (avail.x / img.width as f32)
                        .min(avail.y / img.height as f32)
                        .max(0.05);
                    ui.image((
                        tex.id(),
                        egui::vec2(img.width as f32 * s, img.height as f32 * s),
                    ));
                    ui.horizontal(|ui| {
                        ui.label(format!(
                            "{} × {} píxeles, {:.0} s",
                            img.width, img.height, self.render.seconds
                        ));
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui.button("Guardar PNG…").clicked() {
                                save = true;
                            }
                            if ui
                                .add_enabled(
                                    !self.render.busy(),
                                    egui::Button::new("Volver a renderizar"),
                                )
                                .on_hover_text("Desde la cámara de la vista 3D de ahora")
                                .clicked()
                            {
                                again = true;
                            }
                        });
                    });
                } else if !self.render.busy() {
                    ui.label("Aún no hay imagen.");
                }
            });
        self.render.open = open;
        if again {
            self.start_render();
        }
        if save {
            self.save_render();
        }
    }

    fn save_render(&mut self) {
        let Some(img) = &self.render.image else {
            return;
        };
        let mut d = rfd::FileDialog::new()
            .set_title("Guardar el renderizado")
            .add_filter("PNG", &["png"])
            .set_file_name(format!("{} - render.png", self.file_name_stem()));
        if let Some(dir) = self.start_dir() {
            d = d.set_directory(dir);
        }
        let Some(path) = d.save_file() else {
            return;
        };
        let bytes =
            archisim_export::png::encode_rgb(img.width as u32, img.height as u32, &img.rgb, 96.0);
        match std::fs::write(&path, bytes) {
            Ok(()) => self.say(format!("Imagen guardada en {}.", path.display())),
            Err(e) => self.complain(format!("No se pudo guardar la imagen: {e}.")),
        }
    }

    /// «Renderizado» in the 3D view's properties.
    pub(super) fn render_group(&mut self, ui: &mut Ui) {
        use super::props::{group, row};
        group(ui, "view3d-render", "Renderizado", |ui| {
            row(
                ui,
                "Calidad",
                "Borrador para ver la luz en unos segundos; Alta para presentar",
                |ui, w| {
                    egui::ComboBox::from_id_salt("render-quality")
                        .width(w - 8.0)
                        .selected_text(self.render.quality.label())
                        .show_ui(ui, |ui| {
                            for q in RenderQuality::ALL {
                                ui.selectable_value(&mut self.render.quality, q, q.label());
                            }
                        });
                },
            );
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                ui.add_space(4.0);
                let busy = self.render.busy();
                if ui
                    .add_enabled(!busy, egui::Button::new("Renderizar"))
                    .on_hover_text(
                        "Una imagen realista desde esta cámara, con la luz y las texturas de la vista 3D: el sol del proyecto y sus sombras, la luz del cielo y, al atardecer y de noche, las lámparas encendidas con sus sombras",
                    )
                    .clicked()
                {
                    self.start_render();
                }
                if self.render.image.is_some() && !self.render.open && ui.button("Ver la última").clicked() {
                    self.render.open = true;
                }
            });
        });
    }
}
