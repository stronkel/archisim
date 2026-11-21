//! «Traer el entorno» (H7): the cadastre's buildings and parcels,
//! OpenStreetMap's streets and trees and the CNIG's terrain around the
//! project, from files downloaded from their sites; its window says where
//! to get them and what to bring.

use std::path::{Path, PathBuf};

use archisim_library::carto::{import_carto, CartoOptions};

use super::length_field;
use super::theme::Palette;
use super::widgets::SwitchExt as _;
use super::{ArchiSimApp, Tool, ViewMode};
use archisim_core::Point2;
use eframe::egui;

/// The files chosen and the window's choices.
pub struct CartoImport {
    /// Name and text of each file.
    pub files: Vec<(String, String)>,
    pub options: CartoOptions,
    /// What did not arrive, when they were downloaded.
    pub notes: Vec<String>,
    /// An aerial picture: its name, PNG and the rectangle of the map it
    /// covers.
    pub orthophoto: Option<(String, Vec<u8>, [f64; 4])>,
}

impl CartoImport {
    pub fn open(paths: &[PathBuf]) -> Result<CartoImport, String> {
        let mut files = Vec::new();
        let mut orthophoto = None;
        for p in paths {
            if is_picture(p) {
                orthophoto = Some(picture_with_world_file(p)?);
                continue;
            }
            let bytes = std::fs::read(p).map_err(|e| format!("{}: {e}", p.display()))?;
            let name = p.file_name().map_or_else(
                || p.display().to_string(),
                |n| n.to_string_lossy().into_owned(),
            );
            files.push((name, String::from_utf8_lossy(&bytes).into_owned()));
        }
        Ok(CartoImport {
            files,
            options: CartoOptions::default(),
            notes: Vec::new(),
            orthophoto,
        })
    }

    /// What each file is, for the window.
    fn kinds(&self) -> Vec<(&str, &'static str)> {
        let picture = self
            .orthophoto
            .iter()
            .map(|(n, _, _)| (n.as_str(), "Ortofoto (imagen georreferenciada)"));
        self.files
            .iter()
            .map(|(n, t)| {
                let k = match archisim_carto::kind_of(t) {
                    Some(archisim_carto::FileKind::Cadastre) => "Catastro (GML INSPIRE)",
                    Some(archisim_carto::FileKind::Osm) => "OpenStreetMap",
                    Some(archisim_carto::FileKind::Grid)
                        if archisim_library::carto::is_surface_model(n) =>
                    {
                        "Modelo digital de superficies (MDS, LiDAR): da las plantas de los edificios"
                    }
                    Some(archisim_carto::FileKind::Grid) => "Modelo digital del terreno (ASC)",
                    None => "No se reconoce",
                };
                (n.as_str(), k)
            })
            .chain(picture)
            .collect()
    }
}

/// Where to download each file, for the window and the documentation.
pub const WHERE_FROM: [(&str, &str); 4] = [
    (
        "Catastro",
        "Sede Electrónica del Catastro → Descarga de datos y cartografía por municipio → INSPIRE: el fichero de edificios con sus partes (buildingpart, da las plantas) y el de parcelas (cadastralparcel).",
    ),
    (
        "OpenStreetMap",
        "openstreetmap.org → Exportar → elegir la zona → Exportar (fichero .osm): calles, árboles y, si no hay Catastro, edificios.",
    ),
    (
        "Terreno",
        "Centro de Descargas del CNIG → Modelos digitales de elevaciones → MDT05 (o MDT02) en formato ASC de la hoja de la zona.",
    ),
    (
        "Altura de los edificios (LiDAR)",
        "Centro de Descargas del CNIG → Modelos digitales de elevaciones → MDS05 (modelo de superficies) en ASC de la misma hoja (o, sin ficheros, «Descargar el entorno» con Terreno lo pide al servicio WCS del MDS del IGN para el recuadro del proyecto). Con su nombre (MDS…) se reconoce: el MDS menos el MDT da la altura, y de ella las plantas, de los edificios que se traen o de los que ya hay.",
    ),
];

impl ArchiSimApp {
    /// Asks for the files and opens the window.
    pub(super) fn pick_carto(&mut self) {
        let mut d = crate::file_dialog::FileDialog::new()
            .set_title("Traer el entorno (Catastro, OpenStreetMap, MDT)")
            .add_filter(
                "Cartografía (GML, OSM, ASC, PNG con .pgw)",
                &[
                    "gml", "GML", "xml", "XML", "osm", "OSM", "asc", "ASC", "png", "PNG",
                ],
            );
        if let Some(dir) = self.start_dir() {
            d = d.set_directory(dir);
        }
        if let Some(paths) = d.pick_files() {
            self.open_carto(&paths);
        }
    }

    pub fn open_carto(&mut self, paths: &[PathBuf]) {
        match CartoImport::open(paths) {
            Ok(c) => self.carto_import = Some(c),
            Err(e) => self.complain(format!("No se puede leer la cartografía: {e}.")),
        }
    }

    pub(super) fn carto_dialog(&mut self, ctx: &egui::Context) {
        if self.carto_outline_pick.is_some() {
            return;
        }
        let Some(mut c) = self.carto_import.take() else {
            return;
        };
        let mut open = true;
        let mut go = false;
        let mut cancel = false;
        let mut pick_outline = false;
        let georef = self.ed.model().georeference().cloned();
        super::widgets::dialog("Traer el entorno")
            .open(&mut open)
            .default_width(560.0)
            .resizable(true)
            .show(ctx, |ui| {
                let p = Palette::current(ui.ctx());
                for (name, kind) in c.kinds() {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new(name).strong());
                        ui.label(egui::RichText::new(kind).color(p.muted));
                    });
                }
                for n in &c.notes {
                    ui.label(egui::RichText::new(format!("No ha llegado: {n}")).color(p.muted));
                }
                ui.add_space(4.0);
                ui.label(
                    egui::RichText::new(match &georef {
                        Some(g) => format!(
                            "Se coloca por la georreferencia del proyecto ({}, X {:.0} · Y {:.0}).",
                            g.crs, g.eastings, g.northings
                        ),
                        None => "El proyecto no está georreferenciado: el origen se pondrá en el centro de lo que traen los ficheros (cámbialo después en las propiedades del proyecto, «Emplazamiento (georreferencia)», sin nada seleccionado).".into(),
                    })
                    .color(p.muted),
                );
                ui.add_space(6.0);
                let o = &mut c.options;
                ui.horizontal_wrapped(|ui| {
                    ui.switch(&mut o.buildings, "Edificios");
                    ui.switch(&mut o.parcels, "Parcelas");
                    ui.switch(&mut o.streets, "Calles");
                    ui.switch(&mut o.trees, "Árboles");
                    ui.switch(&mut o.terrain, "Terreno");
                    if c.orthophoto.is_some() {
                        ui.switch(&mut o.orthophoto, "Ortofoto");
                    }
                });
                ui.horizontal(|ui| {
                    ui.label("Radio alrededor del origen");
                    if let Some(v) = length_field(ui, egui::Id::unique("carto-radius"), o.radius, 90.0) {
                        o.radius = v.clamp(10.0, 2000.0);
                    }
                    ui.switch(&mut o.square, "Recuadro")
                        .on_hover_text("Un cuadrado de lado el doble del radio en lugar de un círculo: las calles se cortan en su borde como conexiones con fuera");
                    ui.label("Altura de planta");
                    if let Some(v) = length_field(ui, egui::Id::unique("carto-floor"), o.floor_height, 70.0) {
                        o.floor_height = v.clamp(2.0, 10.0);
                    }
                });
                if area_controls(ui, o, "carto-files") {
                    pick_outline = true;
                }
                ui.switch(&mut o.skip_own, "Sin los edificios que pisan los forjados del proyecto");
                ui.switch(
                    &mut o.street_life,
                    "Vida en la calle: árboles, farolas, bancos, coches, personas y jardines",
                );
                ui.switch(
                    &mut o.styles,
                    "Edificios con su estilo (por su uso, sus plantas y la zona)",
                );
                if o.styles {
                    ui.horizontal(|ui| {
                        ui.label("Zona");
                        let shown = o.zone.map_or("Según lo edificado alrededor", |z| z.name());
                        egui::ComboBox::from_id_salt("carto-zone")
                            .selected_text(shown)
                            .show_ui(ui, |ui| {
                                ui.selectable_value(&mut o.zone, None, "Según lo edificado alrededor");
                                for z in archisim_library::carto::Zone::ALL {
                                    ui.selectable_value(&mut o.zone, Some(z), z.name());
                                }
                            });
                    });
                }
                if georef.is_some() {
                    ui.switch(
                        &mut o.recentre,
                        "Llevar el origen del proyecto al centro de lo que traen los ficheros",
                    );
                }
                ui.add_space(6.0);
                egui::CollapsingHeader::new("Dónde descargar los ficheros (gratis)")
                    .id_salt("carto-where")
                    .show(ui, |ui| {
                        for (what, how) in WHERE_FROM {
                            ui.label(egui::RichText::new(what).strong());
                            ui.label(egui::RichText::new(how).color(p.muted));
                        }
                    });
                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    if ui.button("Traer").clicked() {
                        go = true;
                    }
                    if ui.button("Cancelar").clicked() {
                        cancel = true;
                    }
                });
            });
        if go {
            let mut lines = Vec::new();
            let res = self.ed.transact("Traer el entorno", |m| {
                if !c.files.is_empty() {
                    let r = import_carto(m, &c.files, &c.options)?;
                    lines = r.lines();
                }
                if let (true, Some((name, png, bbox))) = (c.options.orthophoto, &c.orthophoto) {
                    archisim_library::carto::place_orthophoto(m, name, png.clone(), *bbox)?;
                    lines.push("la ortofoto".into());
                }
                Ok(())
            });
            match res {
                Ok(()) if lines.is_empty() => {
                    self.complain("No había nada que traer dentro del radio.")
                }
                Ok(()) => {
                    self.say(format!(
                        "Entorno traído: {}. Si no cae en su sitio, encájalo por dos puntos en este panel.",
                        lines.join(", ")
                    ));
                    // Fitted next, when it does not fall in its place.
                    self.context_fit.open = true;
                }
                Err(e) => self.complain(format!("No se puede traer el entorno: {e}.")),
            }
            return;
        }
        if open && !cancel {
            self.carto_import = Some(c);
        }
        if pick_outline {
            self.start_carto_outline(false);
        }
    }

    /// Starts drawing the outline of the surroundings on the plan (N8).
    pub(super) fn start_carto_outline(&mut self, download: bool) {
        self.set_tool(Tool::Arrow);
        self.mode = ViewMode::Plan;
        self.carto_outline_pick = Some((download, Vec::new()));
        self.say("Clic en cada esquina del contorno del entorno; clic en la primera o Intro para cerrarlo.");
    }

    /// A click on the plan while drawing the outline of the surroundings.
    pub(super) fn carto_outline_click(&mut self, p: Point2) {
        let Some((download, mut pts)) = self.carto_outline_pick.take() else {
            return;
        };
        let reach = self.view.to_metres(crate::touch::reach(10.0));
        if pts.len() >= 3 && pts[0].distance(p) < reach {
            self.carto_outline_pick = Some((download, pts));
            self.carto_outline_finish();
            return;
        }
        pts.push(p);
        self.carto_outline_pick = Some((download, pts));
    }

    /// Closes the outline of the surroundings being drawn (Intro). Whether
    /// there was one.
    pub(super) fn carto_outline_finish(&mut self) -> bool {
        let Some((download, pts)) = self.carto_outline_pick.take() else {
            return false;
        };
        if pts.len() < 3 {
            self.say("El contorno necesita al menos tres esquinas.");
            return true;
        }
        let centre = archisim_core::centroid(&pts);
        let set = |o: &mut CartoOptions| {
            o.outline = Some(pts.clone());
            o.centre = centre;
            o.radius = pts.iter().map(|q| q.distance(centre)).fold(10.0, f64::max);
        };
        if download {
            if let Some(d) = self.carto_download.as_mut() {
                set(&mut d.options);
            }
        } else if let Some(c) = self.carto_import.as_mut() {
            set(&mut c.options);
        }
        self.say(format!(
            "Contorno de {} m² para traer el entorno.",
            super::urban_ui::comma(archisim_core::signed_area(&pts).abs(), 0)
        ));
        true
    }

    /// The outline being drawn, over the plan.
    pub(super) fn draw_carto_outline(&self, painter: &egui::Painter, rect: egui::Rect) {
        let Some((_, pts)) = &self.carto_outline_pick else {
            return;
        };
        let mut line = pts.clone();
        if let Some(c) = self.cursor {
            line.push(c.point);
        }
        painter.add(egui::Shape::dashed_line(
            &self.polygon(rect, &line),
            egui::Stroke::new(2.0, egui::Color32::from_rgb(40, 120, 200)),
            8.0,
            4.0,
        ));
    }
}

/// Whether a file is cartography this window reads (for dropped files).
pub fn is_carto(path: &Path) -> bool {
    let ext = path
        .extension()
        .map(|e| e.to_string_lossy().to_ascii_lowercase())
        .unwrap_or_default();
    matches!(ext.as_str(), "gml" | "osm" | "asc") || is_picture(path)
}

/// Whether a file is a picture with its world file beside it (an
/// orthophoto).
fn is_picture(path: &Path) -> bool {
    let png = path
        .extension()
        .is_some_and(|e| e.eq_ignore_ascii_case("png"));
    png && world_file_of(path).is_some()
}

/// The world file of a picture: «foto.pgw», «foto.pngw» or «foto.wld».
fn world_file_of(path: &Path) -> Option<PathBuf> {
    ["pgw", "PGW", "pngw", "wld"]
        .iter()
        .map(|e| path.with_extension(e))
        .find(|p| p.is_file())
}

/// A picture and the rectangle of the map its world file says.
fn picture_with_world_file(path: &Path) -> Result<(String, Vec<u8>, [f64; 4]), String> {
    let png = std::fs::read(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let (w, h) = archisim_core::png_size(&png)
        .ok_or_else(|| format!("{}: no es una imagen PNG", path.display()))?;
    let wf = world_file_of(path).ok_or("falta su fichero de georreferencia (.pgw)")?;
    let text = std::fs::read_to_string(&wf).map_err(|e| format!("{}: {e}", wf.display()))?;
    let bbox = archisim_library::carto::world_file_bbox(&text, w, h)
        .ok_or_else(|| format!("{}: no se entiende (o está girado)", wf.display()))?;
    let name = path.file_name().map_or_else(
        || "ortofoto.png".into(),
        |n| n.to_string_lossy().into_owned(),
    );
    Ok((name, png, bbox))
}

/// The area's turn and outline drawn by hand, and how fine the terrain is
/// (N8), for the two windows of the surroundings. Whether the outline is
/// to be drawn on the plan.
pub(super) fn area_controls(ui: &mut egui::Ui, o: &mut CartoOptions, id: &str) -> bool {
    let mut pick = false;
    ui.horizontal_wrapped(|ui| {
        if o.square && o.outline.is_none() {
            ui.label("Giro del recuadro");
            ui.add(
                egui::DragValue::new(&mut o.turn)
                    .range(-90.0..=90.0)
                    .speed(0.5)
                    .suffix(" °"),
            );
        }
        match &o.outline {
            Some(p) => {
                ui.label(format!("Contorno a mano de {} puntos", p.len()));
                if ui.small_button("Quitar el contorno").clicked() {
                    o.outline = None;
                }
            }
            None => {
                pick = ui
                    .small_button("Contorno en planta…")
                    .on_hover_text("Dibuja en la planta el contorno de lo que se trae: clic en cada esquina, clic en la primera o Intro para cerrar")
                    .clicked();
            }
        }
    });
    ui.horizontal(|ui| {
        ui.label("Detalle del terreno");
        let label = |n: usize| match n {
            0..=6_000 => "Normal (unos 4 000 puntos)",
            6_001..=30_000 => "Fino (unos 16 000 puntos)",
            _ => "Muy fino (unos 64 000 puntos)",
        };
        egui::ComboBox::from_id_salt((id, "terrain-detail"))
            .selected_text(label(o.terrain_points))
            .show_ui(ui, |ui| {
                for n in [4_000, 16_000, 64_000] {
                    ui.selectable_value(&mut o.terrain_points, n, label(n));
                }
            });
    });
    pick
}
