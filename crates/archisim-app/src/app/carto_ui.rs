//! «Traer el entorno» (H7): the cadastre's buildings and parcels,
//! OpenStreetMap's streets and trees and the CNIG's terrain around the
//! project, from files downloaded from their sites; its window says where
//! to get them and what to bring.

use std::path::{Path, PathBuf};

use archisim_library::carto::{import_carto, CartoOptions};

use super::length_field;
use super::theme::Palette;
use super::widgets::SwitchExt as _;
use super::ArchiSimApp;
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
        let Some(mut c) = self.carto_import.take() else {
            return;
        };
        let mut open = true;
        let mut go = false;
        let mut cancel = false;
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
                    ui.label("Altura de planta");
                    if let Some(v) = length_field(ui, egui::Id::unique("carto-floor"), o.floor_height, 70.0) {
                        o.floor_height = v.clamp(2.0, 10.0);
                    }
                });
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
                Ok(()) => self.say(format!("Entorno traído: {}.", lines.join(", "))),
                Err(e) => self.complain(format!("No se puede traer el entorno: {e}.")),
            }
            return;
        }
        if open && !cancel {
            self.carto_import = Some(c);
        }
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
