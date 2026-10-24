//! Bringing 3D models from other programs (SketchUp, Blender, a
//! manufacturer's catalogue): File → Importar modelo 3D, a model file
//! dropped on the window, or one put in the library folder. A short
//! dialog says what was read (size, faces, materials) and asks its name,
//! what kind of object it is, its units and which way is up; the model
//! becomes an object type and the object tool takes it.

use std::path::{Path, PathBuf};

use archisim_library::imported::ModelKind;
use archisim_mesh::{Mesh, Options, Up};
use eframe::egui;

use super::*;

/// Units a model may be drawn in, with the metres of each.
const UNITS: [(&str, Option<f64>); 5] = [
    ("Las del fichero", None),
    ("Metros", Some(1.0)),
    ("Centímetros", Some(0.01)),
    ("Milímetros", Some(0.001)),
    ("Pulgadas", Some(0.0254)),
];

/// A model being brought in.
pub struct MeshImport {
    pub path: PathBuf,
    pub name: String,
    pub kind: ModelKind,
    pub up: Up,
    /// Index into [`UNITS`].
    pub unit: usize,
    /// What was read with these settings, or why it could not be.
    pub read: Result<Mesh, String>,
}

impl MeshImport {
    pub fn new(path: &Path) -> Self {
        let name = path
            .file_stem()
            .map(|s| s.to_string_lossy().replace(['_', '-'], " "))
            .unwrap_or_default();
        let folder = path
            .parent()
            .and_then(|p| p.file_name())
            .map(|f| f.to_string_lossy().into_owned())
            .unwrap_or_default();
        let kind = ModelKind::from_folder(&format!("{folder} {name}"));
        let mut m = Self {
            path: path.to_path_buf(),
            name,
            kind,
            up: Up::Auto,
            unit: 0,
            read: Err(String::new()),
        };
        m.reread();
        m
    }

    fn options(&self) -> Options {
        Options {
            up: self.up,
            unit: UNITS[self.unit].1,
        }
    }

    pub fn reread(&mut self) {
        self.read = archisim_mesh::read_file(&self.path, self.options()).map_err(|e| e.to_string());
    }
}

impl ArchiSimApp {
    /// File → Importar modelo 3D: chooses the file.
    pub(super) fn pick_mesh(&mut self) {
        let mut d = rfd::FileDialog::new()
            .set_title("Importar modelo 3D")
            .add_filter(
                "Modelos 3D (OBJ, glTF, GLB, COLLADA)",
                &archisim_mesh::EXTENSIONS,
            );
        if let Some(dir) = self.start_dir() {
            d = d.set_directory(dir);
        }
        if let Some(path) = d.pick_file() {
            self.mesh_import = Some(MeshImport::new(&path));
        }
    }

    /// Brings the model in as it is set in the dialog; the object tool
    /// takes its type.
    pub(super) fn import_mesh_now(&mut self) {
        let Some(imp) = self.mesh_import.take() else {
            return;
        };
        let mesh = match imp.read {
            Ok(m) => m,
            Err(e) => {
                self.complain(format!("No se puede importar: {e}."));
                return;
            }
        };
        let source = imp
            .path
            .file_name()
            .map(|f| f.to_string_lossy().into_owned())
            .unwrap_or_default();
        match self.ed.import_mesh(mesh, &imp.name, &source, imp.kind) {
            Ok(r) => {
                let [w, d, h] = r.size;
                let levels = r
                    .triangles
                    .iter()
                    .map(|t| t.to_string())
                    .collect::<Vec<_>>()
                    .join(" / ");
                self.object_settings.object_type = Some(r.object_type);
                self.object_scope = None;
                self.set_tool(Tool::Object);
                self.say(format!(
                    "Modelo 3D importado: {:.2} × {:.2} × {:.2} m, {levels} caras (de cerca / de lejos), {} materiales. Haga clic para colocarlo.",
                    w, d, h, r.materials
                ).replace('.', ","));
            }
            Err(e) => self.complain(format!("No se puede importar: {e}.")),
        }
    }

    pub(super) fn mesh_import_dialog(&mut self, ctx: &egui::Context) {
        let Some(imp) = self.mesh_import.as_mut() else {
            return;
        };
        let mut open = true;
        let mut go = false;
        let mut reread = false;
        super::widgets::dialog("Importar modelo 3D")
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .default_width(380.0)
            .show(ctx, |ui| {
                ui.label(
                    egui::RichText::new(
                        imp.path
                            .file_name()
                            .map(|f| f.to_string_lossy().into_owned())
                            .unwrap_or_default(),
                    )
                    .strong(),
                );
                match &imp.read {
                    Ok(m) => {
                        let (lo, hi) = m.bounds();
                        let s = [0, 1, 2].map(|k| (hi[k] - lo[k]) as f64);
                        let textured = m.materials.iter().filter(|x| x.texture.is_some()).count();
                        ui.label(
                            format!(
                                "{:.2} × {:.2} m de planta, {:.2} m de alto · {} caras · {} materiales{}",
                                s[0],
                                s[1],
                                s[2],
                                m.triangle_count(),
                                m.materials.len(),
                                if textured > 0 {
                                    format!(" ({textured} con textura)")
                                } else {
                                    String::new()
                                }
                            )
                            .replace('.', ","),
                        );
                        if s.iter().any(|v| *v > 100.0) || s[2] < 0.05 {
                            ui.colored_label(
                                egui::Color32::from_rgb(200, 90, 40),
                                "¿Son esas las medidas? Si no, cambie las unidades o el eje vertical.",
                            );
                        }
                    }
                    Err(e) => {
                        ui.colored_label(egui::Color32::from_rgb(200, 60, 40), e);
                    }
                }
                ui.add_space(6.0);
                egui::Grid::new("mesh-import").num_columns(2).spacing([10.0, 6.0]).show(ui, |ui| {
                    ui.label("Nombre del tipo");
                    ui.text_edit_singleline(&mut imp.name);
                    ui.end_row();
                    ui.label("Es");
                    egui::ComboBox::from_id_salt("mesh-kind")
                        .selected_text(imp.kind.label())
                        .show_ui(ui, |ui| {
                            for k in ModelKind::ALL {
                                ui.selectable_value(&mut imp.kind, k, k.label());
                            }
                        });
                    ui.end_row();
                    ui.label("Unidades");
                    egui::ComboBox::from_id_salt("mesh-unit")
                        .selected_text(UNITS[imp.unit].0)
                        .show_ui(ui, |ui| {
                            for (i, (n, _)) in UNITS.iter().enumerate() {
                                reread |= ui.selectable_value(&mut imp.unit, i, *n).changed();
                            }
                        });
                    ui.end_row();
                    ui.label("Hacia arriba");
                    egui::ComboBox::from_id_salt("mesh-up")
                        .selected_text(match imp.up {
                            Up::Auto => "Lo que diga el fichero",
                            Up::Y => "Eje Y (Blender, glTF)",
                            Up::Z => "Eje Z (SketchUp)",
                        })
                        .show_ui(ui, |ui| {
                            for (u, n) in [
                                (Up::Auto, "Lo que diga el fichero"),
                                (Up::Y, "Eje Y (Blender, glTF)"),
                                (Up::Z, "Eje Z (SketchUp)"),
                            ] {
                                reread |= ui.selectable_value(&mut imp.up, u, n).changed();
                            }
                        });
                    ui.end_row();
                });
                ui.add_space(4.0);
                ui.label(
                    egui::RichText::new(
                        "Se guarda en el proyecto con dos versiones más ligeras para verlo de lejos; su planta es su contorno visto desde arriba. La altura del tipo lo escala.",
                    )
                    .small(),
                );
                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    if ui
                        .add_enabled(imp.read.is_ok(), egui::Button::new("Importar"))
                        .clicked()
                    {
                        go = true;
                    }
                });
            });
        if reread {
            if let Some(imp) = self.mesh_import.as_mut() {
                imp.reread();
            }
        }
        if !open {
            self.mesh_import = None;
        } else if go {
            self.import_mesh_now();
        }
    }
}
