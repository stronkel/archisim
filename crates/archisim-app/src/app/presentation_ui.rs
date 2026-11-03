//! The «Estilos de presentación» window (P1): the project's styles, each
//! with its paper, palette, solid walls and line weight; the starter ones
//! and the studio's, kept in the library folder (`Estilos.aspres`). A
//! view takes one in its Visibilidad group.

use super::widgets::SwitchExt as _;
use archisim_core::{PresentationStyle, PresentationStyleId, Rgb};
use eframe::egui::{self, Color32, RichText, Ui};

use super::*;

/// Most tones of a palette.
const MAX_TONES: usize = 6;

#[derive(Default)]
pub struct PresentationUi {
    pub open: bool,
    /// The panel of the sun (the Sol tile of Presentación).
    pub sun_open: bool,
    /// The sun being moved with the sliders: written to the project when
    /// they are let go (one undo step, not one a frame).
    sun_draft: Option<archisim_core::Sun>,
    pub selected: Option<PresentationStyleId>,
    /// The style being edited; written to the project when the mouse and
    /// the keyboard leave it (one undo step per change, not per frame).
    draft: Option<PresentationStyle>,
}

/// Where the studio's styles are kept.
fn library_file() -> std::path::PathBuf {
    crate::editor::library::library_dir().join("Estilos.aspres")
}

fn swatch(ui: &mut Ui, c: Rgb, size: f32) {
    let (rect, _) = ui.allocate_exact_size(egui::vec2(size, size), egui::Sense::hover());
    ui.painter()
        .rect_filled(rect, 2.0, Color32::from_rgb(c[0], c[1], c[2]));
}

impl ArchiSimApp {
    pub(super) fn presentation_dialog(&mut self, ctx: &egui::Context) {
        if !self.presentation.open {
            return;
        }
        let mut open = true;
        super::widgets::side_sheet(ctx, "Estilos de presentación", 620.0)
            .open(&mut open)
            .show(ctx, |ui| self.presentation_contents(ui));
        self.presentation.open = open;
        if !open {
            self.commit_style_draft(true);
        }
    }

    fn presentation_contents(&mut self, ui: &mut Ui) {
        ui.label(
            RichText::new(
                "Cómo se presenta una vista: el color del papel, una paleta a la que pasan los colores del dibujo (por su claridad), los muros y pilares cortados macizados en un color y el grueso de las líneas. Cada planta, sección o vista de un plano elige el suyo en Propiedades → Visibilidad.",
            )
            .weak(),
        );
        ui.separator();
        let styles: Vec<PresentationStyle> =
            self.ed.model().presentation_styles().cloned().collect();
        if self
            .presentation
            .selected
            .is_none_or(|id| styles.iter().all(|s| s.id != id))
        {
            self.presentation.selected = styles.first().map(|s| s.id);
            self.presentation.draft = None;
        }
        let mut pick = None;
        let mut action = None;
        ui.horizontal_top(|ui| {
            ui.vertical(|ui| {
                ui.set_width(200.0);
                if styles.is_empty() {
                    ui.label(RichText::new("El proyecto no tiene estilos.").weak());
                }
                for s in &styles {
                    let on = self.presentation.selected == Some(s.id);
                    ui.horizontal(|ui| {
                        swatch(ui, s.poche.unwrap_or(s.background), 14.0);
                        for c in s.palette.iter().take(4) {
                            swatch(ui, *c, 8.0);
                        }
                        if ui.selectable_label(on, &s.name).clicked() {
                            pick = Some(s.id);
                        }
                    });
                }
                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    if ui.button("＋ Nuevo").clicked() {
                        action = Some("new");
                    }
                    if ui.button("Duplicar").clicked() {
                        action = Some("copy");
                    }
                    if ui.button("Borrar").clicked() {
                        action = Some("remove");
                    }
                });
                ui.add_space(6.0);
                if ui
                    .button("Estilos de partida")
                    .on_hover_text(
                        "Añade Blanco y negro, Terracota, Bosque, Azul y Monocromo si faltan",
                    )
                    .clicked()
                {
                    action = Some("starters");
                }
                ui.label(RichText::new("Biblioteca del estudio").strong());
                if ui
                    .button("Cargar de la biblioteca")
                    .on_hover_text(format!("Añade los de {}", library_file().display()))
                    .clicked()
                {
                    action = Some("load");
                }
                if ui
                    .button("Guardar en la biblioteca")
                    .on_hover_text(format!(
                        "Escribe los del proyecto en {}",
                        library_file().display()
                    ))
                    .clicked()
                {
                    action = Some("save");
                }
            });
            ui.separator();
            ui.vertical(|ui| self.style_form(ui));
        });
        if let Some(id) = pick {
            self.commit_style_draft(true);
            self.presentation.selected = Some(id);
            self.presentation.draft = None;
        }
        match action {
            Some("new") | Some("copy") => {
                self.commit_style_draft(true);
                let base = match (action, self.selected_style()) {
                    (Some("copy"), Some(s)) => s,
                    _ => PresentationStyle::new("Estilo nuevo"),
                };
                match self.ed.add_presentation_style(base) {
                    Ok(id) => {
                        self.presentation.selected = Some(id);
                        self.presentation.draft = None;
                    }
                    Err(e) => self.complain(format!("No se puede: {e}.")),
                }
            }
            Some("remove") => {
                if let Some(id) = self.presentation.selected {
                    self.presentation.draft = None;
                    if self.view_style.presentation == Some(id) {
                        self.view_style.presentation = None;
                    }
                    let r = self.ed.remove_presentation_style(id);
                    self.presentation.selected = None;
                    self.report(r, "Estilo borrado: sus vistas vuelven al dibujo técnico.");
                }
            }
            Some("starters") => {
                match self
                    .ed
                    .add_missing_styles(archisim_library::presentation_styles())
                {
                    Ok(0) => self.say("El proyecto ya tiene los estilos de partida."),
                    Ok(n) => self.say(format!("{n} estilos de partida añadidos.")),
                    Err(e) => self.complain(format!("No se puede: {e}.")),
                }
            }
            Some("load") => self.load_library_styles(),
            Some("save") => self.save_library_styles(),
            _ => {}
        }
        let ctx = ui.ctx();
        let busy = ctx.input(|i| i.pointer.any_down()) || ctx.memory(|m| m.focused().is_some());
        self.commit_style_draft_if(false, busy);
    }

    fn selected_style(&self) -> Option<PresentationStyle> {
        self.presentation
            .selected
            .and_then(|id| self.ed.model().presentation_style(id))
            .cloned()
    }

    fn style_form(&mut self, ui: &mut Ui) {
        let Some(current) = self.selected_style() else {
            ui.label(RichText::new("Elige un estilo o crea uno nuevo.").weak());
            return;
        };
        let d = self.presentation.draft.get_or_insert(current);
        egui::Grid::new("style-form")
            .num_columns(2)
            .spacing([12.0, 8.0])
            .show(ui, |ui| {
                ui.label("Nombre");
                ui.add(egui::TextEdit::singleline(&mut d.name).desired_width(220.0));
                ui.end_row();

                ui.label("Papel");
                ui.color_edit_button_srgb(&mut d.background);
                ui.end_row();

                ui.label("Macizado de muros y pilares");
                ui.horizontal(|ui| {
                    let mut on = d.poche.is_some();
                    if ui.switch(&mut on, "").changed() {
                        d.poche = on.then_some(d.poche.unwrap_or([40, 40, 40]));
                    }
                    if let Some(c) = &mut d.poche {
                        ui.color_edit_button_srgb(c);
                    } else {
                        ui.label(RichText::new("capas y tramas de su tipo").weak());
                    }
                });
                ui.end_row();

                ui.label("Paleta")
                    .on_hover_text("De oscuro a claro: cada color del dibujo pasa al tono de su claridad. Sin tonos, el dibujo conserva sus colores");
                ui.vertical(|ui| {
                    let mut remove = None;
                    ui.horizontal_wrapped(|ui| {
                        for (i, c) in d.palette.iter_mut().enumerate() {
                            ui.color_edit_button_srgb(c);
                            if ui.small_button("✕").on_hover_text("Quitar el tono").clicked() {
                                remove = Some(i);
                            }
                        }
                    });
                    if let Some(i) = remove {
                        d.palette.remove(i);
                    }
                    ui.horizontal(|ui| {
                        if d.palette.len() < MAX_TONES && ui.button("＋ Tono").clicked() {
                            let last = d.palette.last().copied().unwrap_or([60, 60, 60]);
                            d.palette.push(last.map(|v| v.saturating_add(40)));
                        }
                        if d.palette.is_empty() {
                            ui.label(RichText::new("los colores propios del dibujo").weak());
                        }
                    });
                });
                ui.end_row();

                ui.label("Sombras")
                    .on_hover_text("Del sol del proyecto (día y hora en las propiedades del proyecto, sin nada seleccionado)");
                ui.switch(&mut d.shadows, "arrojadas sobre el suelo");
                ui.end_row();

                ui.label("Grueso de las líneas");
                ui.add(
                    egui::Slider::new(&mut d.line_scale, 0.3..=2.0)
                        .step_by(0.05)
                        .custom_formatter(|v, _| format!("× {v:.2}")),
                );
                ui.end_row();
            });
        // A strip of what the style turns the usual colours into.
        ui.add_space(8.0);
        ui.label(RichText::new("Cómo quedan la tinta, un ladrillo, un césped y el papel:").weak());
        ui.horizontal(|ui| {
            for c in [
                [26, 26, 26],
                [200, 110, 80],
                [150, 195, 115],
                [255, 255, 255],
            ] {
                let t = if c == [255, 255, 255] {
                    d.background
                } else {
                    d.tone(c)
                };
                swatch(ui, t, 22.0);
            }
            if let Some(p) = d.poche {
                ui.label("muro:");
                swatch(ui, p, 22.0);
            }
        });
        let in_use = self
            .ed
            .model()
            .viewports()
            .filter(|v| v.style.presentation == Some(d.id))
            .count();
        if in_use > 0 {
            ui.label(RichText::new(format!("Lo usan {in_use} vistas de los planos.")).weak());
        }
        if ui
            .button("Usar en la vista activa")
            .on_hover_text("La planta o sección de la pestaña activa se dibuja con este estilo")
            .clicked()
        {
            self.view_style.presentation = Some(d.id);
        }
    }

    /// Writes the style being edited to the project, if it changed (and,
    /// unless `now`, nothing is being dragged or typed in `ui`).
    fn commit_style_draft(&mut self, now: bool) {
        self.commit_style_draft_if(now, false);
    }

    fn commit_style_draft_if(&mut self, now: bool, busy: bool) {
        let Some(d) = self.presentation.draft.clone() else {
            return;
        };
        if !now && busy {
            return;
        }
        let Some(current) = self.ed.model().presentation_style(d.id).cloned() else {
            self.presentation.draft = None;
            return;
        };
        if current == d {
            return;
        }
        if let Err(e) = self.ed.set_presentation_style(d) {
            self.complain(format!("No se puede: {e}."));
            self.presentation.draft = Some(current);
        }
    }

    fn load_library_styles(&mut self) {
        let path = library_file();
        let text = match std::fs::read_to_string(&path) {
            Ok(t) => t,
            Err(_) => {
                self.complain(format!(
                    "No hay estilos en {} todavía: guárdalos desde un proyecto.",
                    path.display()
                ));
                return;
            }
        };
        match archisim_core::parse_presentation_styles(&text) {
            Err(e) => self.complain(format!("{}: {e}", path.display())),
            Ok(styles) => match self.ed.add_missing_styles(styles) {
                Ok(n) => self.say(format!(
                    "{n} estilos de la biblioteca añadidos al proyecto."
                )),
                Err(e) => self.complain(format!("No se puede: {e}.")),
            },
        }
    }

    fn save_library_styles(&mut self) {
        self.commit_style_draft(true);
        let path = library_file();
        // The file keeps the studio's: the project's are added, or replace
        // those of the same name.
        let mut all = std::fs::read_to_string(&path)
            .ok()
            .and_then(|t| archisim_core::parse_presentation_styles(&t).ok())
            .unwrap_or_default();
        for s in self.ed.model().presentation_styles() {
            match all.iter_mut().find(|x| x.name == s.name) {
                Some(x) => *x = s.clone(),
                None => all.push(s.clone()),
            }
        }
        let n = all.len();
        let result = path
            .parent()
            .map_or(Ok(()), std::fs::create_dir_all)
            .and_then(|()| std::fs::write(&path, archisim_core::presentation_styles_text(&all)));
        match result {
            Ok(()) => self.say(format!("{n} estilos en {}.", path.display())),
            Err(e) => self.complain(format!("No se puede escribir {}: {e}.", path.display())),
        }
    }

    /// The Presentación row of a view's Visibilidad group.
    pub(super) fn presentation_row(&mut self, ui: &mut Ui) -> bool {
        let opts: Vec<(Option<PresentationStyleId>, String)> =
            std::iter::once((None, "Técnica (sin estilo)".to_string()))
                .chain(
                    self.ed
                        .model()
                        .presentation_styles()
                        .map(|s| (Some(s.id), s.name.clone())),
                )
                .collect();
        let style = &mut self.view_style;
        props::row(
            ui,
            "Estilo",
            "Papel, paleta, muros macizados y grueso de líneas de un estilo de presentación",
            |ui, w| {
                let refs: Vec<(Option<PresentationStyleId>, &str)> =
                    opts.iter().map(|(a, b)| (*a, b.as_str())).collect();
                style.presentation =
                    props::choice(ui, "view-presentation", w, style.presentation, &refs);
            },
        );
        ui.small_button("Estilos de presentación…")
            .on_hover_text("Crear y cambiar los estilos del proyecto")
            .clicked()
    }

    /// The panel of the sun (§12.4, the Sol tile of Presentación): its
    /// path over the project for midsummer, the equinox and midwinter, the
    /// day and the solar hour on sliders, and the three days architects
    /// look at, with the sun's height and bearing.
    pub(super) fn sun_dialog(&mut self, ctx: &egui::Context) {
        if !self.presentation.sun_open {
            return;
        }
        let mut open = true;
        super::widgets::dialog("Sol")
            .id(egui::Id::unique("sun-panel"))
            .open(&mut open)
            .default_width(300.0)
            .resizable(false)
            .show(ctx, |ui| self.sun_contents(ui));
        self.presentation.sun_open = open;
    }

    fn sun_contents(&mut self, ui: &mut Ui) {
        let p = super::theme::Palette::current(ui.ctx());
        let model = self.ed.model();
        let lat = model.latitude();
        let north = model.true_north();
        let georef = model.georeference().is_some();
        let mut sun = self.presentation.sun_draft.unwrap_or(model.sun());
        // The path of the sun, in plan: the horizon a circle, the zenith in
        // the middle; north where the project's north is.
        let size = ui.available_width().min(280.0);
        let (rect, _) = ui.allocate_exact_size(Vec2::new(size, size), egui::Sense::hover());
        let painter = ui.painter_at(rect);
        let c = rect.center();
        let r = size * 0.42;
        painter.circle_filled(c, r, p.raised);
        painter.circle_stroke(c, r, Stroke::new(1.0, p.line));
        for k in [1.0 / 3.0, 2.0 / 3.0] {
            painter.circle_stroke(c, r * k as f32, Stroke::new(0.5, p.line));
        }
        // A bearing from north (clockwise) and a height to the screen.
        let at = |alt: f64, az: f64| -> Pos2 {
            let b = (az + north) as f32;
            let d = r * (1.0 - (alt / std::f64::consts::FRAC_PI_2) as f32).clamp(0.0, 1.0);
            Pos2::new(c.x + d * b.sin(), c.y - d * b.cos())
        };
        for (label, b) in [("N", 0.0), ("E", 90.0), ("S", 180.0), ("O", 270.0)] {
            let q = at(-0.12, f64::to_radians(b));
            painter.text(
                q,
                egui::Align2::CENTER_CENTER,
                label,
                egui::FontId::proportional(11.0),
                if label == "N" { p.accent } else { p.muted },
            );
        }
        let days = [
            (172u16, Color32::from_rgb(220, 140, 40)),
            (80, Color32::from_rgb(200, 170, 60)),
            (355, Color32::from_rgb(110, 140, 190)),
        ];
        for (day, col) in days {
            let pts: Vec<Pos2> = (0..=96)
                .map(|i| archisim_core::Sun {
                    day,
                    hour: i as f64 / 4.0,
                })
                .map(|s| s.position(lat))
                .filter(|(alt, _)| *alt > 0.0)
                .map(|(alt, az)| at(alt, az))
                .collect();
            if pts.len() > 1 {
                painter.add(egui::Shape::line(pts, Stroke::new(1.5, col)));
            }
        }
        let (alt, az) = sun.position(lat);
        if alt > 0.0 {
            let q = at(alt, az);
            painter.line_segment([c, q], Stroke::new(1.0, p.muted));
            painter.circle_filled(q, 7.0, Color32::from_rgb(255, 196, 40));
            painter.circle_stroke(q, 7.0, Stroke::new(1.0, Color32::from_rgb(200, 130, 20)));
        }
        ui.horizontal(|ui| {
            for (day, col, name) in [
                (172u16, days[0].1, "Verano"),
                (80, days[1].1, "Equinoccio"),
                (355, days[2].1, "Invierno"),
            ] {
                let (dot, _) = ui.allocate_exact_size(Vec2::splat(9.0), egui::Sense::hover());
                ui.painter().circle_filled(dot.center(), 4.0, col);
                if ui
                    .small_button(name)
                    .on_hover_text("Ese día, a mediodía solar")
                    .clicked()
                {
                    sun = archisim_core::Sun { day, hour: 12.0 };
                    self.presentation.sun_draft = None;
                    let r = self.ed.set_sun(sun);
                    self.report(r, "Sol cambiado.");
                }
            }
        });
        ui.add_space(6.0);
        let (d, m) = day_month(sun.day);
        let mut day = sun.day as f64;
        let rd = ui.add(
            egui::Slider::new(&mut day, 1.0..=365.0)
                .step_by(1.0)
                .show_value(false)
                .text(format!("{d} de {}", MONTH_NAMES[m as usize - 1])),
        );
        let mut hour = sun.hour;
        let h = hour.floor();
        let clock = format!(
            "{:02}:{:02} solar",
            h as u32,
            ((hour - h) * 60.0).round() as u32
        );
        let rh = ui.add(
            egui::Slider::new(&mut hour, 4.0..=21.0)
                .step_by(0.25)
                .show_value(false)
                .text(clock),
        );
        let moved = rd.changed() || rh.changed();
        if moved {
            sun = archisim_core::Sun {
                day: (day.round() as u16).clamp(1, 365),
                hour: hour.clamp(0.0, 24.0),
            };
            self.presentation.sun_draft = Some(sun);
        }
        let let_go =
            (rd.drag_stopped() || rh.drag_stopped()) || (moved && !rd.dragged() && !rh.dragged());
        if let_go {
            if let Some(s) = self.presentation.sun_draft.take() {
                let r = self.ed.set_sun(s);
                self.report(
                    r,
                    "Sol cambiado: las sombras de las vistas de presentación y del 3D lo siguen.",
                );
            }
        }
        ui.add_space(4.0);
        let text = if alt > 0.0 {
            format!(
                "Altura {:.0}° · desde el norte {:.0}°",
                alt.to_degrees(),
                az.to_degrees()
            )
        } else {
            "El sol está bajo el horizonte".to_string()
        };
        ui.label(RichText::new(text).color(p.text));
        ui.label(
            RichText::new(if georef {
                format!("Latitud {lat:.2}°, de la georreferencia del proyecto")
            } else {
                format!("Latitud {lat:.2}° (Madrid): sin georreferencia en el proyecto")
            })
            .small()
            .color(p.muted),
        );
    }

    /// The sun of the shadows in the project's properties: the day (as
    /// day/month) and the solar hour.
    pub(super) fn sun_rows(&mut self, ui: &mut Ui) {
        let sun = self.ed.model().sun();
        let (day, month) = day_month(sun.day);
        let mut changed = None;
        props::row(
            ui,
            "Sol: día",
            "Día de las sombras en las vistas de presentación (día/mes)",
            |ui, w| {
                let shown = format!("{day}/{month}");
                if let Some(v) =
                    text_edit_committed(ui, egui::Id::unique("project-sun-day"), &shown, w - 8.0)
                {
                    changed = Some(parse_day(&v).map(|d| archisim_core::Sun { day: d, ..sun }));
                }
            },
        );
        props::row(
            ui,
            "Sol: hora solar",
            "12:00 es mediodía, con el sol al sur",
            |ui, w| {
                let h = sun.hour.floor();
                let shown = format!(
                    "{:02}:{:02}",
                    h as u32,
                    ((sun.hour - h) * 60.0).round() as u32
                );
                if let Some(v) =
                    text_edit_committed(ui, egui::Id::unique("project-sun-hour"), &shown, w - 8.0)
                {
                    changed = Some(parse_hour(&v).map(|hour| archisim_core::Sun { hour, ..sun }));
                }
            },
        );
        match changed {
            Some(Some(s)) => {
                let r = self.ed.set_sun(s);
                self.report(
                    r,
                    "Sol cambiado: las sombras de las vistas de presentación lo siguen.",
                );
            }
            Some(None) => self.complain("Escribe el día como 21/3 y la hora como 11:30."),
            None => {}
        }
    }
}

const MONTH_DAYS: [u16; 12] = [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];

const MONTH_NAMES: [&str; 12] = [
    "enero",
    "febrero",
    "marzo",
    "abril",
    "mayo",
    "junio",
    "julio",
    "agosto",
    "septiembre",
    "octubre",
    "noviembre",
    "diciembre",
];

/// Day and month of a day of the year.
fn day_month(n: u16) -> (u16, u16) {
    let mut left = n.clamp(1, 365);
    for (m, days) in MONTH_DAYS.iter().enumerate() {
        if left <= *days {
            return (left, m as u16 + 1);
        }
        left -= days;
    }
    (31, 12)
}

/// A day written as `21/3`; its day of the year.
fn parse_day(t: &str) -> Option<u16> {
    let (d, m) = t.trim().split_once('/')?;
    let (d, m): (u16, usize) = (d.trim().parse().ok()?, m.trim().parse().ok()?);
    if !(1..=12).contains(&m) || d < 1 || d > MONTH_DAYS[m - 1] {
        return None;
    }
    Some(MONTH_DAYS[..m - 1].iter().sum::<u16>() + d)
}

/// An hour written as `11:30` or `11`.
fn parse_hour(t: &str) -> Option<f64> {
    let t = t.trim().trim_end_matches('h');
    let (h, m) = t.split_once(':').unwrap_or((t, "0"));
    let (h, m): (f64, f64) = (h.trim().parse().ok()?, m.trim().parse().ok()?);
    let v = h + m / 60.0;
    ((0.0..=24.0).contains(&v) && (0.0..60.0).contains(&m)).then_some(v)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn days_and_hours_are_read_as_written() {
        assert_eq!(parse_day("21/3"), Some(80));
        assert_eq!(day_month(80), (21, 3));
        assert_eq!(day_month(172), (21, 6));
        assert_eq!(parse_day("30/2"), None);
        assert_eq!(parse_hour("11:30"), Some(11.5));
        assert_eq!(parse_hour("16"), Some(16.0));
        assert_eq!(parse_hour("25"), None);
    }
}
