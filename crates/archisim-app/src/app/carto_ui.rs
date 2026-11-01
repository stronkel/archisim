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
}

impl CartoImport {
    pub fn open(paths: &[PathBuf]) -> Result<CartoImport, String> {
        let mut files = Vec::new();
        for p in paths {
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
        })
    }

    /// What each file is, for the window.
    fn kinds(&self) -> Vec<(&str, &'static str)> {
        self.files
            .iter()
            .map(|(n, t)| {
                let k = match archisim_carto::kind_of(t) {
                    Some(archisim_carto::FileKind::Cadastre) => "Catastro (GML INSPIRE)",
                    Some(archisim_carto::FileKind::Osm) => "OpenStreetMap",
                    Some(archisim_carto::FileKind::Grid) => "Modelo digital del terreno (ASC)",
                    None => "No se reconoce",
                };
                (n.as_str(), k)
            })
            .collect()
    }
}

/// Where to download each file, for the window and the documentation.
pub const WHERE_FROM: [(&str, &str); 3] = [
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
];

impl ArchiSimApp {
    /// Asks for the files and opens the window.
    pub(super) fn pick_carto(&mut self) {
        let mut d = rfd::FileDialog::new()
            .set_title("Traer el entorno (Catastro, OpenStreetMap, MDT)")
            .add_filter(
                "Cartografía (GML, OSM, ASC)",
                &["gml", "GML", "xml", "XML", "osm", "OSM", "asc", "ASC"],
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
                });
                ui.horizontal(|ui| {
                    ui.label("Radio alrededor del origen (mm)");
                    if let Some(v) = length_field(ui, egui::Id::unique("carto-radius"), o.radius, 90.0) {
                        o.radius = v.clamp(10.0, 2000.0);
                    }
                    ui.label("Altura de planta (mm)");
                    if let Some(v) = length_field(ui, egui::Id::unique("carto-floor"), o.floor_height, 70.0) {
                        o.floor_height = v.clamp(2.0, 10.0);
                    }
                });
                ui.switch(&mut o.skip_own, "Sin los edificios que pisan los forjados del proyecto");
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
                let r = import_carto(m, &c.files, &c.options)?;
                lines = r.lines();
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
    matches!(ext.as_str(), "gml" | "osm" | "asc")
}
