//! Importing a topographic survey («Importar topografía»): a DXF drawing
//! (its points with height, blocks with a height attribute, 3D faces and
//! contour lines, from the layers chosen) or a list of points (CSV, TXT),
//! into the terrain of the active level. The survey is usually in UTM
//! coordinates and absolute heights: it is centred on the project origin
//! and its heights taken from the one chosen as the project's ±0,00.

use std::path::Path;

use archisim_core::{Point2, SurveyPoint};
use archisim_dxf::{Drawing, LayerSummary, Survey, SurveyOptions};
use eframe::egui::{self, Id};

use super::theme::Palette;
use super::{length_field, text_edit_committed, ArchiSimApp};
use crate::units::fmt_mm;

/// A survey being imported, with the choices of its window.
pub struct SurveyImport {
    pub file: String,
    source: Source,
    /// Layers of a drawing and whether each is taken.
    pub layers: Vec<(LayerSummary, bool)>,
    pub heights_from_texts: bool,
    /// Metres per drawing unit.
    pub unit: f64,
    /// Points closer than this (metres) are thinned out.
    pub min_spacing: f64,
    /// A list of points with the northing first.
    pub swap: bool,
    /// The survey's middle at the project origin (or its coordinates as
    /// they are).
    pub centre: bool,
    /// The survey height that is the project's ±0,00 (metres).
    pub datum: f64,
    pub contour: f64,
    /// Added to the level's terrain (or a new one).
    pub add: bool,
}

enum Source {
    Drawing(Drawing),
    Points(String),
}

impl SurveyImport {
    /// Reads a file: a DXF drawing, or else a list of points.
    pub fn open(path: &Path) -> Result<Self, String> {
        let bytes = std::fs::read(path).map_err(|e| format!("no se puede leer: {e}"))?;
        let file = path
            .file_name()
            .map_or(String::new(), |f| f.to_string_lossy().into_owned());
        let ext = path
            .extension()
            .map_or(String::new(), |e| e.to_string_lossy().to_lowercase());
        if ext == "dwg" {
            return Err("los DWG son un formato cerrado: guárdalo como DXF desde tu programa de CAD (o con el conversor gratuito ODA File Converter) e impórtalo".into());
        }
        let source = if ext == "dxf" {
            Source::Drawing(archisim_dxf::read_bytes(&bytes).map_err(|e| e.to_string())?)
        } else {
            Source::Points(archisim_dxf::decode(&bytes))
        };
        let (layers, unit) = match &source {
            Source::Drawing(d) => (
                archisim_dxf::layers(d)
                    .into_iter()
                    .map(|l| {
                        let on = l.looks_topographic();
                        (l, on)
                    })
                    .collect(),
                d.unit.unwrap_or(1.0),
            ),
            Source::Points(_) => (Vec::new(), 1.0),
        };
        let mut s = Self {
            file,
            source,
            layers,
            heights_from_texts: false,
            unit,
            min_spacing: 0.0,
            swap: false,
            centre: true,
            datum: 0.0,
            contour: 0.5,
            add: false,
        };
        let survey = s.survey();
        if survey.points.is_empty() && survey.lines.is_empty() {
            return Err("no hay puntos con cota (ni curvas de nivel) que leer".into());
        }
        // The ±0,00 at the lowest whole metre, and the contour interval of
        // the drawing.
        if let Some((lo, _)) = survey.z_range() {
            s.datum = lo.floor();
        }
        if let Some(step) = survey.contour_interval() {
            s.contour = step;
        }
        Ok(s)
    }

    /// The survey as chosen, in metres (survey coordinates).
    pub fn survey(&self) -> Survey {
        let mut s = match &self.source {
            Source::Drawing(d) => archisim_dxf::survey(
                d,
                &SurveyOptions {
                    layers: self
                        .layers
                        .iter()
                        .filter(|(_, on)| *on)
                        .map(|(l, _)| l.name.clone())
                        .collect(),
                    heights_from_texts: self.heights_from_texts,
                    text_reach: 2.0 / self.unit,
                    min_spacing: self.min_spacing / self.unit,
                },
            ),
            Source::Points(text) => {
                let (points, _) = archisim_dxf::points_from_text(text, self.swap);
                Survey {
                    points,
                    lines: Vec::new(),
                }
            }
        };
        if self.unit != 1.0 {
            let k = self.unit;
            for p in s.points.iter_mut().chain(s.lines.iter_mut().flatten()) {
                *p = [p[0] * k, p[1] * k, p[2] * k];
            }
        }
        if self.min_spacing > 0.0 {
            if let Source::Points(_) = self.source {
                let d = Drawing {
                    entities: s
                        .points
                        .iter()
                        .map(|p| archisim_dxf::Entity {
                            layer: String::new(),
                            shape: archisim_dxf::Shape::Point(*p),
                        })
                        .collect(),
                    ..Drawing::default()
                };
                s.points = archisim_dxf::survey(
                    &d,
                    &SurveyOptions {
                        min_spacing: self.min_spacing,
                        ..SurveyOptions::default()
                    },
                )
                .points;
            }
        }
        s
    }

    /// The plan offset taken off the survey's coordinates: its middle, to
    /// the metre.
    pub fn offset(&self, s: &Survey) -> [f64; 2] {
        match (self.centre, s.bounds()) {
            (true, Some((lo, hi))) => [
                ((lo[0] + hi[0]) / 2.0).round(),
                ((lo[1] + hi[1]) / 2.0).round(),
            ],
            _ => [0.0, 0.0],
        }
    }

    /// Points and contour lines in the project's axes.
    pub fn project(&self) -> (Vec<SurveyPoint>, Vec<(Vec<Point2>, f64)>) {
        let s = self.survey();
        let [dx, dy] = self.offset(&s);
        let at = |p: &[f64; 3]| Point2::new(p[0] - dx, p[1] - dy);
        let points = s
            .points
            .iter()
            .map(|p| SurveyPoint {
                at: at(p),
                z: p[2] - self.datum,
            })
            .collect();
        let lines = s
            .lines
            .iter()
            .map(|l| (l.iter().map(at).collect(), l[0][2] - self.datum))
            .collect();
        (points, lines)
    }
}

/// A height in metres as typed («641,5»).
fn metres(text: &str) -> Option<f64> {
    archisim_dxf::number(text)
}

fn fmt_m(v: f64) -> String {
    format!("{v:.2}").replace('.', ",")
}

impl ArchiSimApp {
    /// Asks for the survey file and opens its window.
    pub(super) fn pick_survey(&mut self) {
        let mut d = rfd::FileDialog::new()
            .set_title("Importar topografía")
            .add_filter(
                "Levantamiento (DXF, CSV, TXT)",
                &[
                    "dxf", "DXF", "csv", "CSV", "txt", "TXT", "xyz", "XYZ", "dwg", "DWG",
                ],
            );
        if let Some(dir) = self.start_dir() {
            d = d.set_directory(dir);
        }
        if let Some(path) = d.pick_file() {
            self.open_survey(&path);
        }
    }

    pub fn open_survey(&mut self, path: &Path) {
        match SurveyImport::open(path) {
            Ok(s) => self.survey_import = Some(s),
            Err(e) => self.complain(format!("No se puede importar la topografía: {e}.")),
        }
    }

    pub(super) fn survey_dialog(&mut self, ctx: &egui::Context) {
        let Some(mut s) = self.survey_import.take() else {
            return;
        };
        let mut open = true;
        let mut go = false;
        let mut cancel = false;
        let terrain_here = self.level_terrain();
        super::widgets::dialog("Importar topografía")
            .open(&mut open)
            .default_width(620.0)
            .resizable(true)
            .show(ctx, |ui| {
                let p = Palette::current(ui.ctx());
                ui.label(egui::RichText::new(&s.file).strong());
                ui.add_space(4.0);
                if !s.layers.is_empty() {
                    ui.label(
                        egui::RichText::new(
                            "Capas del dibujo: marca las del levantamiento (puntos con cota, curvas de nivel, textos de cotas). Las que solo tienen dibujo a cota 0 (edificios, viales) se dejan fuera.",
                        )
                        .color(p.muted),
                    );
                    egui::ScrollArea::vertical()
                        .max_height(200.0)
                        .show(ui, |ui| {
                            egui::Grid::new("survey-layers")
                                .num_columns(6)
                                .striped(true)
                                .spacing([12.0, 3.0])
                                .show(ui, |ui| {
                                    ui.label("");
                                    ui.strong("Capa");
                                    ui.strong("Puntos");
                                    ui.strong("Curvas");
                                    ui.strong("Textos de cota");
                                    ui.strong("Cotas");
                                    ui.end_row();
                                    for (l, on) in s.layers.iter_mut() {
                                        ui.checkbox(on, "");
                                        ui.label(&l.name);
                                        ui.label(l.points.to_string());
                                        ui.label(l.contours.to_string());
                                        ui.label(l.numbers.to_string());
                                        ui.label(l.z.map_or("—".into(), |(a, b)| {
                                            format!("{} … {}", fmt_m(a), fmt_m(b))
                                        }));
                                        ui.end_row();
                                    }
                                });
                        });
                    ui.horizontal(|ui| {
                        if ui.small_button("Solo las del levantamiento").clicked() {
                            for (l, on) in s.layers.iter_mut() {
                                *on = l.looks_topographic();
                            }
                        }
                        if ui.small_button("Todas").clicked() {
                            for (_, on) in s.layers.iter_mut() {
                                *on = true;
                            }
                        }
                    });
                    let numbers: usize = s
                        .layers
                        .iter()
                        .filter(|(_, on)| *on)
                        .map(|(l, _)| l.numbers)
                        .sum();
                    ui.add_enabled_ui(numbers > 0, |ui| {
                        ui.checkbox(
                            &mut s.heights_from_texts,
                            "Puntos sin cota: tomar la del texto de cota más cercano (a menos de 2 m)",
                        );
                    });
                    ui.horizontal(|ui| {
                        ui.label("Unidades del dibujo:");
                        for (k, name) in [(1.0, "metros"), (0.01, "centímetros"), (0.001, "milímetros")] {
                            ui.radio_value(&mut s.unit, k, name);
                        }
                    });
                } else {
                    ui.horizontal(|ui| {
                        ui.label("Columnas:");
                        ui.radio_value(&mut s.swap, false, "X, Y, Z");
                        ui.radio_value(&mut s.swap, true, "Y, X, Z (norte primero)");
                    });
                }
                ui.add_space(6.0);
                egui::Grid::new("survey-options")
                    .num_columns(2)
                    .spacing([12.0, 6.0])
                    .show(ui, |ui| {
                        ui.label("Distancia mínima entre puntos (mm)");
                        if let Some(v) =
                            length_field(ui, Id::unique("survey-spacing"), s.min_spacing, 120.0)
                        {
                            s.min_spacing = v.max(0.0);
                        }
                        ui.end_row();
                        ui.label("Cota del levantamiento que es el ±0,00 (m)");
                        if let Some(v) = text_edit_committed(
                            ui,
                            Id::unique("survey-datum"),
                            &fmt_m(s.datum),
                            120.0,
                        ) {
                            match metres(&v) {
                                Some(z) => s.datum = z,
                                None => self.complain(format!("«{v}» no es una cota: escribe 641,50.")),
                            }
                        }
                        ui.end_row();
                        ui.label("Equidistancia de las curvas (mm)");
                        if let Some(v) =
                            length_field(ui, Id::unique("survey-contour"), s.contour, 120.0)
                        {
                            s.contour = v.max(0.0);
                        }
                        ui.end_row();
                    });
                ui.checkbox(
                    &mut s.centre,
                    "Centrar el levantamiento en el origen del proyecto (si no, sus coordenadas tal cual)",
                );
                if terrain_here.is_some() {
                    ui.checkbox(&mut s.add, "Añadir al terreno de esta planta (si no, un terreno nuevo)");
                }
                ui.add_space(6.0);
                let survey = s.survey();
                let [dx, dy] = s.offset(&survey);
                let size = survey
                    .bounds()
                    .map_or(String::new(), |(lo, hi)| {
                        format!(" · {} × {} m", fmt_m(hi[0] - lo[0]), fmt_m(hi[1] - lo[1]))
                    });
                let range = survey.z_range().map_or(String::new(), |(lo, hi)| {
                    format!(
                        " · cotas {} a {} m ({} a {} sobre el ±0,00)",
                        fmt_m(lo),
                        fmt_m(hi),
                        fmt_mm(lo - s.datum),
                        fmt_mm(hi - s.datum)
                    )
                });
                ui.label(format!(
                    "Se importan {} puntos y {} curvas de nivel{size}{range}.",
                    survey.points.len(),
                    survey.lines.len()
                ));
                if s.centre {
                    ui.label(
                        egui::RichText::new(format!(
                            "El origen del proyecto queda en X {} · Y {} del levantamiento.",
                            fmt_m(dx),
                            fmt_m(dy)
                        ))
                        .color(p.muted),
                    );
                }
                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    let any = !survey.points.is_empty() || !survey.lines.is_empty();
                    if ui.add_enabled(any, egui::Button::new("Importar")).clicked() {
                        go = true;
                    }
                    if ui.button("Cancelar").clicked() {
                        cancel = true;
                    }
                });
            });
        if go {
            self.finish_survey(&s, terrain_here.filter(|_| s.add));
            return;
        }
        if open && !cancel {
            self.survey_import = Some(s);
        }
    }

    pub(super) fn finish_survey(
        &mut self,
        s: &SurveyImport,
        into: Option<archisim_core::TerrainId>,
    ) {
        let (points, lines) = s.project();
        let (np, nl) = (points.len(), lines.len());
        let settings = crate::editor::TerrainSettings {
            contour: s.contour,
            ..self.terrain_settings
        };
        match self.ed.import_terrain(&settings, points, lines, into) {
            Ok(id) => {
                self.ed.selection = [super::Element::Terrain(id)].into_iter().collect();
                self.fit_requested = true;
                let survey = s.survey();
                let [dx, dy] = s.offset(&survey);
                self.say(format!(
                    "Topografía importada: {np} puntos y {nl} curvas de nivel. El origen del proyecto es X {} · Y {} del levantamiento y el ±0,00, la cota {} m.",
                    fmt_m(dx),
                    fmt_m(dy),
                    fmt_m(s.datum)
                ));
            }
            Err(e) => self.complain(format!("No se puede crear el terreno: {e}.")),
        }
    }
}
