//! The «Estilos de presentación» window (P1): the project's styles, each
//! with its paper, palette, solid walls and line weight; the starter ones
//! and the studio's, kept in the library folder (`Estilos.aspres`). A
//! view takes one in its Visibilidad group.

use archisim_core::{PresentationStyle, PresentationStyleId, Rgb};
use eframe::egui::{self, Color32, RichText, Ui};

use super::*;

/// Most tones of a palette.
const MAX_TONES: usize = 6;

#[derive(Default)]
pub struct PresentationUi {
    pub open: bool,
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
        egui::Window::new("Estilos de presentación")
            .open(&mut open)
            .default_width(620.0)
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
                    .on_hover_text("Añade AD5, Terracota, Bosque, Azul y Monocromo si faltan")
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
                    if ui.checkbox(&mut on, "").changed() {
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
                ui.checkbox(&mut d.shadows, "arrojadas sobre el suelo");
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
