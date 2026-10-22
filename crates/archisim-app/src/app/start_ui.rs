//! The start screen (bloque 3 de la revisión del 22/10/2026): the welcome
//! when the program opens without a file, with the projects opened lately
//! as cards with a small plan of each, and the ways to begin — a new
//! project, open, import an IFC, a shared project, the first-steps
//! missions. The list of recent files lives in the preferences; the small
//! plans in the settings folder (`miniaturas/`), drawn when a project is
//! saved or opened.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use archisim_core::Model;
use eframe::egui::{self, Align2, Color32, CornerRadius, Id, Rect, Sense, Stroke, Vec2};

use super::*;
use crate::app::theme::Palette;
use crate::app::widgets::strong_font;

/// How many files the list keeps.
pub const RECENT_MAX: usize = 12;

/// Size of the small plans, in pixels.
const THUMB: [usize; 2] = [320, 200];

#[derive(Default)]
pub struct StartScreen {
    pub open: bool,
    /// Small plans loaded, by file (`None`: there is none).
    pub(super) thumbs: HashMap<PathBuf, Option<egui::TextureHandle>>,
}

/// Puts `path` first in the list of recent files.
pub fn remember(recent: &mut Vec<String>, path: &Path) {
    let p = path
        .canonicalize()
        .unwrap_or_else(|_| path.to_path_buf())
        .display()
        .to_string();
    recent.retain(|r| *r != p);
    recent.insert(0, p);
    recent.truncate(RECENT_MAX);
}

/// Where the small plan of a file is kept.
fn thumb_file(path: &Path) -> Option<PathBuf> {
    use std::hash::{Hash, Hasher};
    let mut h = std::collections::hash_map::DefaultHasher::new();
    path.hash(&mut h);
    crate::prefs::settings_dir().map(|d| {
        d.join("miniaturas")
            .join(format!("{:016x}.png", h.finish()))
    })
}

/// The level a small plan shows: the one with most walls (the ground
/// floor of a house, not its empty roof).
fn thumb_level(model: &Model) -> Option<archisim_core::LevelId> {
    model
        .levels()
        .iter()
        .max_by_key(|l| model.walls().filter(|w| w.level == l.id).count())
        .map(|l| l.id)
}

/// Draws the small plan of a project: its fullest level, fitted with a
/// margin, at 1:200 with thin lines.
pub fn thumbnail(model: &Model) -> Option<egui::ColorImage> {
    let level = thumb_level(model)?;
    let geo = crate::geo::Indexed::new(model);
    let pts: Vec<archisim_core::Point2> = crate::pick::elements_on_level(model, level)
        .into_iter()
        .filter_map(|e| crate::pick::outline(&geo, e))
        .flatten()
        .collect();
    let first = *pts.first()?;
    let (mut lo, mut hi) = pts.iter().fold((first, first), |(lo, hi), p| {
        (
            archisim_core::Point2::new(lo.x.min(p.x), lo.y.min(p.y)),
            archisim_core::Point2::new(hi.x.max(p.x), hi.y.max(p.y)),
        )
    });
    // Fitted to the picture's proportions, with a margin.
    let (w, h) = ((hi.x - lo.x).max(1.0), (hi.y - lo.y).max(1.0));
    let aspect = THUMB[0] as f64 / THUMB[1] as f64;
    let (w2, h2) = if w / h > aspect {
        (w, w / aspect)
    } else {
        (h * aspect, h)
    };
    let c = (lo + hi) * 0.5;
    let half = archisim_core::Point2::new(w2 * 0.56, h2 * 0.56);
    lo = c - half;
    hi = c + half;
    let options = archisim_plan::PlanOptions {
        viewport: Some((lo, hi)),
        scale: 200.0,
        thin_lines: true,
        ..archisim_plan::PlanOptions::default()
    };
    let image = crate::plan_image::rasterize_with(model, level, THUMB, &options);
    // On white paper, as in the plan.
    let mut out = egui::ColorImage::new(THUMB, vec![Color32::WHITE; THUMB[0] * THUMB[1]]);
    for (o, p) in out.pixels.iter_mut().zip(&image.pixels) {
        let a = p.a() as u32;
        let mix = |c: u8| (c as u32 + (255 - a)).min(255) as u8;
        *o = Color32::from_rgb(mix(p.r()), mix(p.g()), mix(p.b()));
    }
    Some(out)
}

/// Keeps the small plan of `path` in the settings folder.
pub fn save_thumbnail(path: &Path, model: &Model) {
    if cfg!(test) {
        return;
    }
    let (Some(file), Some(image)) = (thumb_file(&canonical(path)), thumbnail(model)) else {
        return;
    };
    let rgb: Vec<u8> = image
        .pixels
        .iter()
        .flat_map(|p| [p.r(), p.g(), p.b()])
        .collect();
    let png = archisim_export::png::encode_rgb(THUMB[0] as u32, THUMB[1] as u32, &rgb, 96.0);
    if let Some(dir) = file.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    let _ = std::fs::write(file, png);
}

fn canonical(path: &Path) -> PathBuf {
    path.canonicalize().unwrap_or_else(|_| path.to_path_buf())
}

/// Reads a small plan back.
fn load_thumbnail(path: &Path) -> Option<egui::ColorImage> {
    let file = std::fs::File::open(thumb_file(path)?).ok()?;
    let mut reader = png::Decoder::new(std::io::BufReader::new(file))
        .read_info()
        .ok()?;
    let mut buf = vec![0; reader.output_buffer_size()?];
    let info = reader.next_frame(&mut buf).ok()?;
    let (w, h) = (info.width as usize, info.height as usize);
    let channels = match info.color_type {
        png::ColorType::Rgb => 3,
        png::ColorType::Rgba => 4,
        _ => return None,
    };
    let pixels = buf[..w * h * channels]
        .chunks(channels)
        .map(|c| Color32::from_rgb(c[0], c[1], c[2]))
        .collect();
    Some(egui::ColorImage::new([w, h], pixels))
}

/// When a file was last changed, as people say it.
fn when(path: &Path) -> Option<String> {
    let modified = std::fs::metadata(path).ok()?.modified().ok()?;
    let ago = std::time::SystemTime::now()
        .duration_since(modified)
        .ok()?
        .as_secs();
    Some(match ago {
        0..=59 => "hace un momento".to_string(),
        60..=3599 => format!("hace {} min", ago / 60),
        3600..=86_399 => format!("hace {} h", ago / 3600),
        86_400..=172_799 => "ayer".to_string(),
        172_800..=604_799 => format!("hace {} días", ago / 86_400),
        _ => {
            let secs = modified
                .duration_since(std::time::UNIX_EPOCH)
                .ok()?
                .as_secs() as i64;
            let (y, m, d) = civil(secs.div_euclid(86_400));
            format!("{d}/{m}/{y}")
        }
    })
}

/// Year, month and day of a count of days since 1970-01-01.
fn civil(days: i64) -> (i64, u32, u32) {
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    let y = yoe + era * 400 + if m <= 2 { 1 } else { 0 };
    (y, m, d)
}

/// What the start screen asked for.
enum Choice {
    New,
    Open,
    Import,
    Team,
    Recent(PathBuf),
    Forget(String),
    Missions,
    Close,
}

impl ArchiSimApp {
    /// Opens a file of the recent list, by its kind.
    pub(super) fn open_recent(&mut self, path: &Path) {
        if !path.exists() {
            self.complain(format!("No se encuentra {}.", path.display()));
            return;
        }
        let ext = path
            .extension()
            .map(|e| e.to_string_lossy().to_lowercase())
            .unwrap_or_default();
        if ext == archisim_team::EXTENSION {
            self.open_team(path);
        } else if ext == "ifc" {
            self.guarded(Pending::Import(Some(path.to_path_buf())));
        } else {
            self.guarded(Pending::Open(Some(path.to_path_buf())));
        }
    }

    /// Notes a file opened or saved: first in the recent list, with its
    /// small plan.
    pub(super) fn note_recent(&mut self, path: &Path) {
        remember(&mut self.prefs.recent, path);
        save_thumbnail(path, &self.ed.shown());
        self.start.thumbs.remove(&canonical(path));
    }

    /// The «Abrir reciente» menu.
    pub(super) fn recent_menu(&mut self, ui: &mut egui::Ui) {
        ui.menu_button("Abrir reciente", |ui| {
            if self.prefs.recent.is_empty() {
                ui.label("Todavía no hay ninguno.");
            }
            let mut open = None;
            for r in &self.prefs.recent {
                let p = PathBuf::from(r);
                let name = p
                    .file_name()
                    .map_or_else(|| r.clone(), |n| n.to_string_lossy().into_owned());
                if ui.button(name).on_hover_text(r).clicked() {
                    open = Some(p);
                    ui.close();
                }
            }
            if let Some(p) = open {
                self.open_recent(&p);
            }
        });
        if ui.button("Pantalla de inicio").clicked() {
            self.start.open = true;
            ui.close();
        }
    }

    /// Draws the start screen over the window, while it is open.
    pub(super) fn start_screen(&mut self, ctx: &egui::Context) {
        if !self.start.open {
            return;
        }
        let p = Palette::current(ctx);
        let screen = ctx.content_rect();
        // It offers the missions: the welcome card is not needed.
        self.prefs.welcomed = true;
        // The window behind, dimmed, and not clickable.
        egui::Area::new(Id::unique("start-backdrop"))
            .order(egui::Order::Middle)
            .fixed_pos(screen.min)
            .show(ctx, |ui| {
                ui.allocate_exact_size(screen.size(), Sense::click());
            });
        let size = Vec2::new(
            (screen.width() - 64.0).clamp(560.0, 980.0),
            (screen.height() - 64.0).clamp(420.0, 620.0),
        );
        let mut choice = None;
        egui::Area::new(Id::unique("start-screen"))
            .order(egui::Order::Foreground)
            .anchor(Align2::CENTER_CENTER, Vec2::ZERO)
            .show(ctx, |ui| {
                // Under the screen, on its own layer (the area's painter
                // only reaches the area).
                ui.ctx().layer_painter(ui.layer_id()).rect_filled(
                    screen,
                    CornerRadius::ZERO,
                    Color32::from_black_alpha(90),
                );
                egui::Frame::new()
                    .fill(p.panel)
                    .stroke(Stroke::new(1.0, p.line))
                    .corner_radius(CornerRadius::same(12))
                    .shadow(egui::epaint::Shadow {
                        offset: [0, 8],
                        blur: 32,
                        spread: 0,
                        color: Color32::from_black_alpha(60),
                    })
                    .show(ui, |ui| {
                        ui.set_min_size(size);
                        ui.set_max_size(size);
                        ui.horizontal_top(|ui| {
                            choice = self.start_left(ui, &p, size.y).or(choice.take());
                            ui.add_space(4.0);
                            if let Some(c) = self.start_recent(ui, &p, size - Vec2::new(250.0, 0.0))
                            {
                                choice = Some(c);
                            }
                        });
                    });
            });
        if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            choice = Some(Choice::Close);
        }
        match choice {
            Some(Choice::New) => {
                self.start.open = false;
                self.guarded(Pending::New);
            }
            Some(Choice::Open) => self.guarded(Pending::Open(None)),
            Some(Choice::Import) => self.guarded(Pending::Import(None)),
            Some(Choice::Team) => self.pick_team_folder(),
            Some(Choice::Recent(path)) => self.open_recent(&path),
            Some(Choice::Forget(r)) => self.prefs.recent.retain(|x| *x != r),
            Some(Choice::Missions) => {
                self.start.open = false;
                self.missions.window_open = true;
                self.prefs.welcomed = true;
            }
            Some(Choice::Close) => self.start.open = false,
            None => {}
        }
    }

    /// The welcome and the ways to begin.
    fn start_left(&mut self, ui: &mut egui::Ui, p: &Palette, height: f32) -> Option<Choice> {
        let mut choice = None;
        egui::Frame::new()
            .fill(p.raised)
            .corner_radius(CornerRadius {
                nw: 12,
                sw: 12,
                ne: 0,
                se: 0,
            })
            .inner_margin(egui::Margin::same(22))
            .show(ui, |ui| {
                ui.vertical(|ui| self.start_left_body(ui, p, height, &mut choice));
            });
        choice
    }

    fn start_left_body(
        &mut self,
        ui: &mut egui::Ui,
        p: &Palette,
        height: f32,
        choice: &mut Option<Choice>,
    ) {
        ui.set_width(206.0);
        ui.set_min_height(height - 44.0);
        ui.label(
            egui::RichText::new("ArchiSim")
                .font(strong_font(26.0))
                .color(p.text),
        );
        ui.label(
            egui::RichText::new(format!("versión {}", env!("CARGO_PKG_VERSION")))
                .small()
                .color(p.muted),
        );
        ui.add_space(18.0);
        ui.label(egui::RichText::new("Bienvenido.").color(p.text));
        ui.label(
            egui::RichText::new("Empieza un proyecto o sigue con uno de los últimos.")
                .color(p.muted),
        );
        ui.add_space(18.0);
        let big = |ui: &mut egui::Ui, text: &str, accent: bool| {
            let b = egui::Button::new(egui::RichText::new(text).color(if accent {
                Color32::WHITE
            } else {
                p.text
            }))
            .min_size(Vec2::new(206.0, 32.0));
            let b = if accent { b.fill(p.accent) } else { b };
            ui.add(b).clicked()
        };
        if big(ui, "Proyecto nuevo", true) {
            *choice = Some(Choice::New);
        }
        ui.add_space(4.0);
        if big(ui, "Abrir…", false) {
            *choice = Some(Choice::Open);
        }
        ui.add_space(4.0);
        if big(ui, "Importar IFC…", false) {
            *choice = Some(Choice::Import);
        }
        ui.add_space(4.0);
        if big(ui, "Proyecto en equipo…", false) {
            *choice = Some(Choice::Team);
        }
        ui.add_space(18.0);
        ui.separator();
        ui.add_space(8.0);
        if ui
            .link(format!(
                "Primeros pasos: {} misiones",
                crate::missions::MISSIONS.len()
            ))
            .on_hover_text("Te enseñan dónde está cada cosa, paso a paso")
            .clicked()
        {
            *choice = Some(Choice::Missions);
        }
        ui.add_space(12.0);
        if ui.link("Seguir sin abrir nada").clicked() {
            *choice = Some(Choice::Close);
        }
    }

    /// The projects opened lately, as cards.
    fn start_recent(&mut self, ui: &mut egui::Ui, p: &Palette, size: Vec2) -> Option<Choice> {
        let mut choice = None;
        ui.vertical(|ui| {
            ui.add_space(20.0);
            ui.label(egui::RichText::new("Recientes").font(strong_font(15.0)).color(p.text));
            ui.add_space(8.0);
            if self.prefs.recent.is_empty() {
                ui.add_space(40.0);
                ui.label(
                    egui::RichText::new(
                        "Aquí saldrán los proyectos que abras o guardes, con un dibujo de su planta.",
                    )
                    .color(p.muted),
                );
                return;
            }
            let card = Vec2::new(176.0, 160.0);
            let cols = ((size.x - 24.0) / (card.x + 10.0)).floor().max(1.0) as usize;
            egui::ScrollArea::vertical()
                .max_height(size.y - 64.0)
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    let recent = self.prefs.recent.clone();
                    egui::Grid::new("start-recent")
                        .spacing(Vec2::new(10.0, 10.0))
                        .show(ui, |ui| {
                            for (i, r) in recent.iter().enumerate() {
                                if let Some(c) = self.recent_card(ui, p, r, card) {
                                    choice = Some(c);
                                }
                                if (i + 1) % cols == 0 {
                                    ui.end_row();
                                }
                            }
                        });
                });
        });
        choice
    }

    fn recent_card(
        &mut self,
        ui: &mut egui::Ui,
        p: &Palette,
        r: &str,
        card: Vec2,
    ) -> Option<Choice> {
        let path = PathBuf::from(r);
        let exists = path.exists();
        let (rect, resp) = ui.allocate_exact_size(card, Sense::click());
        let painter = ui.painter_at(rect);
        let hovered = resp.hovered() && exists;
        painter.rect(
            rect,
            CornerRadius::same(8),
            if hovered { p.accent_soft } else { p.raised },
            Stroke::new(1.0, if hovered { p.accent } else { p.line }),
            egui::StrokeKind::Inside,
        );
        let pic = Rect::from_min_size(
            rect.min + Vec2::new(6.0, 6.0),
            Vec2::new(card.x - 12.0, 100.0),
        );
        let tex = self
            .start
            .thumbs
            .entry(canonical(&path))
            .or_insert_with(|| {
                load_thumbnail(&canonical(&path)).map(|img| {
                    ui.ctx()
                        .load_texture(format!("thumb-{r}"), img, egui::TextureOptions::LINEAR)
                })
            })
            .clone();
        painter.rect_filled(pic, CornerRadius::same(5), Color32::WHITE);
        match tex {
            Some(t) => {
                // Fitted, keeping its proportions.
                let [w, h] = t.size().map(|v| v as f32);
                let k = (pic.width() / w).min(pic.height() / h);
                let r = Rect::from_center_size(pic.center(), Vec2::new(w * k, h * k));
                painter.image(
                    t.id(),
                    r,
                    Rect::from_min_max(egui::Pos2::ZERO, egui::Pos2::new(1.0, 1.0)),
                    if exists {
                        Color32::WHITE
                    } else {
                        Color32::from_white_alpha(90)
                    },
                );
            }
            None => {
                let kind = path
                    .extension()
                    .map(|e| e.to_string_lossy().to_uppercase())
                    .unwrap_or_default();
                painter.text(
                    pic.center(),
                    Align2::CENTER_CENTER,
                    kind,
                    strong_font(18.0),
                    p.faint,
                );
            }
        }
        // A shared project is known by its folder.
        let team = path
            .extension()
            .is_some_and(|e| e.eq_ignore_ascii_case(archisim_team::EXTENSION));
        let named = if team {
            path.parent()
        } else {
            Some(path.as_path())
        };
        let name = named
            .and_then(|p| if team { p.file_name() } else { p.file_stem() })
            .map_or_else(|| r.to_string(), |n| n.to_string_lossy().into_owned());
        let text_x = rect.left() + 8.0;
        let mut y = pic.bottom() + 8.0;
        let fit = |s: &str, n: usize| {
            if s.chars().count() > n {
                format!("{}…", s.chars().take(n - 1).collect::<String>())
            } else {
                s.to_string()
            }
        };
        painter.text(
            egui::pos2(text_x, y),
            Align2::LEFT_TOP,
            fit(&name, 22),
            strong_font(13.0),
            if exists { p.text } else { p.faint },
        );
        y += 18.0;
        let sub = if exists {
            when(&path).unwrap_or_default()
        } else {
            "No se encuentra".to_string()
        };
        painter.text(
            egui::pos2(text_x, y),
            Align2::LEFT_TOP,
            sub,
            egui::FontId::proportional(11.0),
            p.muted,
        );
        let mut choice = None;
        let resp = resp.on_hover_text(r);
        if resp.clicked() && exists {
            choice = Some(Choice::Recent(path.clone()));
        }
        resp.context_menu(|ui| {
            if exists && ui.button("Abrir").clicked() {
                choice = Some(Choice::Recent(path.clone()));
                ui.close();
            }
            if ui.button("Quitar de la lista").clicked() {
                choice = Some(Choice::Forget(r.to_string()));
                ui.close();
            }
        });
        choice
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recent_files_first_and_without_repeats() {
        let mut r = Vec::new();
        for f in ["/a.archisim", "/b.archisim", "/a.archisim"] {
            remember(&mut r, Path::new(f));
        }
        assert_eq!(
            r,
            vec!["/a.archisim".to_string(), "/b.archisim".to_string()]
        );
        for i in 0..20 {
            remember(&mut r, Path::new(&format!("/p{i}.archisim")));
        }
        assert_eq!(r.len(), RECENT_MAX);
        assert_eq!(r[0], "/p19.archisim");
    }

    #[test]
    fn dates_of_files() {
        assert_eq!(civil(0), (1970, 1, 1));
        assert_eq!(civil(20_748), (2026, 10, 22));
    }
}
