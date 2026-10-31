//! Bringing 3D models from other programs (SketchUp, Blender, a
//! manufacturer's catalogue): File → Importar modelo 3D, a model file (or
//! the manufacturer's `.zip`) dropped on the window, or one put in the
//! library folder. A dialog says what was read (size, faces, materials),
//! shows it from above and from the front with its front marked, and asks
//! its name, what kind of object it is, its units, which way is up, which
//! way it faces and how much detail to keep; a design piece also asks who
//! made it, and is kept in the user's library («Modelos 3D/Diseño/
//! <Fabricante>/», never inside ArchiSim). A heavy model is lightened
//! aside, without stopping the window; the object tool then takes the new
//! type.

use std::path::{Path, PathBuf};
use std::sync::mpsc::Receiver;

use archisim_library::imported::{ImportOptions, Maker, ModelKind, DEFAULT_DETAIL, DETAILS};
use archisim_mesh::{Loaded, Mesh, Options, Up};
use eframe::egui;

use super::widgets::SwitchExt;
use super::*;

/// Units a model may be drawn in, with the metres of each.
const UNITS: [(&str, Option<f64>); 5] = [
    ("Automáticas", None),
    ("Metros", Some(1.0)),
    ("Centímetros", Some(0.01)),
    ("Milímetros", Some(0.001)),
    ("Pulgadas", Some(0.0254)),
];

/// The name of a unit by its metres.
fn unit_name(k: f64) -> &'static str {
    UNITS
        .iter()
        .skip(1)
        .find(|(_, m)| m.is_some_and(|m| (m - k).abs() < 1e-9))
        .map_or("metros", |(n, _)| n)
}

/// The two drawings of the dialog: from above and from the front.
pub struct Pictures {
    pub top: egui::TextureHandle,
    pub front: egui::TextureHandle,
    /// Width over depth (above) and width over height (front).
    pub aspect: [f32; 2],
}

/// A model being brought in.
pub struct MeshImport {
    pub path: PathBuf,
    pub name: String,
    pub kind: ModelKind,
    pub up: Up,
    /// Index into [`UNITS`].
    pub unit: usize,
    /// The model chosen inside a `.zip`.
    pub entry: Option<String>,
    /// Quarter turns given to bring its front to the front.
    pub turns: u8,
    /// Index into [`DETAILS`].
    pub detail: usize,
    /// A design piece of a manufacturer.
    pub design: bool,
    pub maker: Maker,
    /// Kept also in the user's library (design pieces).
    pub keep: bool,
    /// What was read with these settings, or why it could not be.
    pub read: Result<Loaded, String>,
    pictures: Option<(u8, Pictures)>,
    /// Being lightened aside: the result comes here.
    pub preparing: Option<Receiver<Mesh>>,
}

/// Silhouettes of `mesh` turned `turns` quarters: from above (front at
/// the bottom) and from the front, `size` pixels on their longer side.
fn silhouettes(mesh: &Mesh, turns: u8, size: usize) -> ([egui::ColorImage; 2], [f32; 2]) {
    let turn = |p: [f32; 3]| {
        let mut q = p;
        for _ in 0..turns % 4 {
            q = [-q[1], q[0], q[2]];
        }
        q
    };
    let pts: Vec<[f32; 3]> = mesh.positions.iter().map(|p| turn(*p)).collect();
    let mut lo = [f32::INFINITY; 3];
    let mut hi = [f32::NEG_INFINITY; 3];
    for p in &pts {
        for k in 0..3 {
            lo[k] = lo[k].min(p[k]);
            hi[k] = hi[k].max(p[k]);
        }
    }
    let ext = [0, 1, 2].map(|k| (hi[k] - lo[k]).max(1e-6));
    // (screen x, screen y down) for each view: above, x right and −y
    // (the front) down; front, x right and z up.
    let views: [(usize, usize); 2] = [(0, 1), (0, 2)];
    let mut out = Vec::new();
    let mut aspect = [1.0; 2];
    for (vi, (a, b)) in views.into_iter().enumerate() {
        let r = ext[a] / ext[b];
        aspect[vi] = r;
        let (w, h) = if r >= 1.0 {
            (size, ((size as f32 / r) as usize).max(4))
        } else {
            (((size as f32 * r) as usize).max(4), size)
        };
        let mut px = vec![0u8; w * h];
        let to_px = |p: [f32; 3]| -> [f32; 2] {
            let x = (p[a] - lo[a]) / ext[a] * (w as f32 - 1.0);
            let t = (p[b] - lo[b]) / ext[b];
            // Above: +y (the back) at the top; front: up at the top.
            let y = (1.0 - t) * (h as f32 - 1.0);
            [x, y]
        };
        for g in &mesh.groups {
            for t in &g.triangles {
                let [p0, p1, p2] = t.map(|i| to_px(pts[i as usize]));
                let x0 = p0[0].min(p1[0]).min(p2[0]).floor().max(0.0) as usize;
                let x1 = (p0[0].max(p1[0]).max(p2[0]).ceil() as usize).min(w - 1);
                let y0 = p0[1].min(p1[1]).min(p2[1]).floor().max(0.0) as usize;
                let y1 = (p0[1].max(p1[1]).max(p2[1]).ceil() as usize).min(h - 1);
                let edge = |a: [f32; 2], b: [f32; 2], x: f32, y: f32| {
                    (b[0] - a[0]) * (y - a[1]) - (b[1] - a[1]) * (x - a[0])
                };
                let area = edge(p0, p1, p2[0], p2[1]);
                for y in y0..=y1 {
                    for x in x0..=x1 {
                        let (fx, fy) = (x as f32, y as f32);
                        let e = [
                            edge(p0, p1, fx, fy),
                            edge(p1, p2, fx, fy),
                            edge(p2, p0, fx, fy),
                        ];
                        let inside = if area.abs() < 1e-9 {
                            // A sliver: the pixels it passes.
                            x1 - x0 <= 1 || y1 - y0 <= 1
                        } else {
                            e.iter()
                                .all(|v| v * area.signum() >= -0.5 * area.abs().sqrt())
                        };
                        if inside {
                            px[y * w + x] = 255;
                        }
                    }
                }
            }
        }
        let img = egui::ColorImage::from_rgba_unmultiplied(
            [w, h],
            &px.iter()
                .flat_map(|v| [120, 128, 140, *v])
                .collect::<Vec<u8>>(),
        );
        out.push(img);
    }
    let front = out.pop().expect("two views");
    let top = out.pop().expect("two views");
    ([top, front], aspect)
}

/// Where a design piece is kept in the user's library, and the file with
/// who made it next to it.
pub fn design_paths(name: &str, maker: &Maker) -> (PathBuf, PathBuf) {
    let clean = |s: &str| s.replace(['/', '\\', ':', '*', '?', '"', '<', '>', '|'], "-");
    let maker_dir = if maker.manufacturer.trim().is_empty() {
        "Otros fabricantes".to_string()
    } else {
        clean(maker.manufacturer.trim())
    };
    let dir = crate::editor::library::library_dir()
        .join("Modelos 3D")
        .join("Diseño")
        .join(maker_dir);
    let file = clean(name.trim());
    (
        dir.join(format!("{file}.glb")),
        dir.join(format!("{file}.json")),
    )
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
        let (_, _, options) = super::object_gallery::folder_model_info(path);
        let mut m = Self {
            path: path.to_path_buf(),
            name,
            kind,
            up: Up::Auto,
            unit: 0,
            entry: None,
            turns: 0,
            detail: DEFAULT_DETAIL,
            design: options.maker.is_some(),
            maker: options.maker.unwrap_or_default(),
            keep: true,
            read: Err(String::new()),
            pictures: None,
            preparing: None,
        };
        m.reread();
        if let Ok(l) = &m.read {
            // A model inside a zip names the piece better than the zip.
            if let Some(e) = &l.entry {
                if m.name.trim().is_empty() {
                    m.name = Path::new(e)
                        .file_stem()
                        .map(|s| s.to_string_lossy().replace(['_', '-'], " "))
                        .unwrap_or_default();
                }
            }
        }
        m
    }

    fn options(&self) -> Options {
        Options {
            up: self.up,
            unit: UNITS[self.unit].1,
        }
    }

    pub fn reread(&mut self) {
        self.read = archisim_mesh::load(&self.path, self.entry.as_deref(), self.options())
            .map_err(|e| e.to_string());
        if let Ok(l) = &self.read {
            self.entry = l.entry.clone();
        }
        self.pictures = None;
    }

    /// How it is brought in.
    pub fn import_options(&self) -> ImportOptions {
        ImportOptions {
            turns: self.turns,
            detail: DETAILS[self.detail].1,
            maker: (self.design && !self.maker.is_empty()).then(|| self.maker.clone()),
        }
    }

    /// Whether bringing it in takes a while (it is lightened aside).
    fn slow(&self) -> bool {
        match (&self.read, DETAILS[self.detail].1) {
            (Ok(l), Some(n)) => l.mesh.triangle_count() > n.max(60_000),
            _ => false,
        }
    }

    fn pictures(&mut self, ctx: &egui::Context) -> Option<&Pictures> {
        let stale = self.pictures.as_ref().is_none_or(|(t, _)| *t != self.turns);
        if stale {
            let l = self.read.as_ref().ok()?;
            let ([top, front], aspect) = silhouettes(&l.mesh, self.turns, 120);
            let opts = egui::TextureOptions::LINEAR;
            self.pictures = Some((
                self.turns,
                Pictures {
                    top: ctx.load_texture("mesh-import-top", top, opts),
                    front: ctx.load_texture("mesh-import-front", front, opts),
                    aspect,
                },
            ));
        }
        self.pictures.as_ref().map(|(_, p)| p)
    }
}

impl ArchiSimApp {
    /// File → Importar modelo 3D: chooses the file.
    pub(super) fn pick_mesh(&mut self) {
        let mut exts: Vec<&str> = archisim_mesh::EXTENSIONS.to_vec();
        exts.push("zip");
        let mut d = rfd::FileDialog::new()
            .set_title("Importar modelo 3D")
            .add_filter(
                "Modelos 3D (FBX, OBJ, glTF, GLB, COLLADA, 3DS, STL o el .zip del fabricante)",
                &exts,
            );
        if let Some(dir) = self.start_dir() {
            d = d.set_directory(dir);
        }
        if let Some(path) = d.pick_file() {
            self.mesh_import = Some(MeshImport::new(&path));
        }
    }

    /// Brings the model in as it is set in the dialog; the object tool
    /// takes its type. A heavy model is lightened aside first.
    pub(super) fn import_mesh_now(&mut self) {
        let Some(imp) = self.mesh_import.as_mut() else {
            return;
        };
        if imp.preparing.is_some() {
            return;
        }
        let mesh = match &imp.read {
            Ok(l) => l.mesh.clone(),
            Err(e) => {
                let e = e.clone();
                self.mesh_import = None;
                self.complain(format!("No se puede importar: {e}."));
                return;
            }
        };
        let options = imp.import_options();
        if imp.slow() {
            let (tx, rx) = std::sync::mpsc::channel();
            std::thread::spawn(move || {
                let _ = tx.send(archisim_library::imported::prepare(mesh, &options));
            });
            imp.preparing = Some(rx);
            return;
        }
        let mesh = archisim_library::imported::prepare(mesh, &options);
        self.finish_mesh_import(mesh);
    }

    /// The model, lightened and turned, into the project (and, a design
    /// piece, into the user's library).
    fn finish_mesh_import(&mut self, mesh: Mesh) {
        let Some(imp) = self.mesh_import.take() else {
            return;
        };
        let options = imp.import_options().prepared();
        let source = match imp.read.as_ref().ok().and_then(|l| l.entry.clone()) {
            Some(e) => format!(
                "{} ({})",
                imp.path
                    .file_name()
                    .map(|f| f.to_string_lossy().into_owned())
                    .unwrap_or_default(),
                Path::new(&e)
                    .file_name()
                    .map(|f| f.to_string_lossy().into_owned())
                    .unwrap_or(e)
            ),
            None => imp
                .path
                .file_name()
                .map(|f| f.to_string_lossy().into_owned())
                .unwrap_or_default(),
        };
        match self
            .ed
            .import_mesh_with(mesh, &imp.name, &source, imp.kind, &options)
        {
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
                let mut kept = String::new();
                if let (Some(maker), true) = (&options.maker, imp.keep) {
                    match self.keep_design_piece(r.object_type, &imp.name, maker, imp.kind) {
                        Ok(Some(path)) => {
                            kept = format!(
                                " Guardado en su biblioteca ({}).",
                                path.strip_prefix(crate::editor::library::library_dir())
                                    .unwrap_or(&path)
                                    .display()
                            )
                        }
                        Ok(None) => {}
                        Err(e) => kept = format!(" No se pudo guardar en la biblioteca: {e}."),
                    }
                }
                let dims = format!("{w:.2} × {d:.2} × {h:.2}").replace('.', ",");
                self.say(format!(
                    "Modelo 3D importado: {dims} m, {levels} caras (de cerca / de lejos), {} materiales. Haga clic para colocarlo.{kept}",
                    r.materials
                ));
            }
            Err(e) => self.complain(format!("No se puede importar: {e}.")),
        }
    }

    /// Keeps a design piece in the user's library: its model as a `.glb`
    /// and who made it next to it. The tests never write there.
    fn keep_design_piece(
        &mut self,
        t: archisim_core::ObjectTypeId,
        name: &str,
        maker: &Maker,
        kind: ModelKind,
    ) -> Result<Option<PathBuf>, String> {
        let (glb_path, json_path) = design_paths(name, maker);
        if cfg!(test) {
            return Ok(None);
        }
        let (glb, _) = archisim_library::imported::export_glb(self.ed.model(), t)
            .ok_or("el tipo no tiene modelo 3D")?;
        if let Some(dir) = glb_path.parent() {
            std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
        }
        std::fs::write(&glb_path, glb).map_err(|e| e.to_string())?;
        std::fs::write(&json_path, maker.to_json(name, kind)).map_err(|e| e.to_string())?;
        self.folder_objects = None;
        self.library.refresh();
        Ok(Some(glb_path))
    }

    pub(super) fn mesh_import_dialog(&mut self, ctx: &egui::Context) {
        // A model lightened aside: when it is ready, in.
        let ready = self
            .mesh_import
            .as_ref()
            .and_then(|i| i.preparing.as_ref())
            .and_then(|rx| rx.try_recv().ok());
        if let Some(mesh) = ready {
            self.finish_mesh_import(mesh);
            return;
        }
        let Some(imp) = self.mesh_import.as_mut() else {
            return;
        };
        let busy = imp.preparing.is_some();
        if busy {
            ctx.request_repaint_after(std::time::Duration::from_millis(100));
        }
        let mut open = true;
        let mut go = false;
        let mut reread = false;
        let pal = super::theme::Palette::current(ctx);
        super::widgets::dialog("Importar modelo 3D")
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .default_width(440.0)
            .show(ctx, |ui| {
                let tall = (ui.ctx().content_rect().height() - 190.0).max(240.0);
                egui::ScrollArea::vertical().max_height(tall).show(ui, |ui| {
                ui.add_enabled_ui(!busy, |ui| {
                    ui.label(
                        egui::RichText::new(
                            imp.path
                                .file_name()
                                .map(|f| f.to_string_lossy().into_owned())
                                .unwrap_or_default(),
                        )
                        .strong(),
                    );
                    let (models, others, entry) = match &imp.read {
                        Ok(l) => (l.models.clone(), l.others.clone(), l.entry.clone()),
                        Err(_) => (Vec::new(), Vec::new(), None),
                    };
                    if models.len() > 1 {
                        ui.horizontal(|ui| {
                            ui.label("Modelo del .zip");
                            egui::ComboBox::from_id_salt("mesh-entry")
                                .width(300.0)
                                .selected_text(entry.clone().unwrap_or_default())
                                .show_ui(ui, |ui| {
                                    for m in &models {
                                        if ui
                                            .selectable_label(entry.as_deref() == Some(m), m)
                                            .clicked()
                                        {
                                            imp.entry = Some(m.clone());
                                            reread = true;
                                        }
                                    }
                                });
                        });
                    }
                    if !others.is_empty() {
                        ui.label(
                            egui::RichText::new(format!(
                                "El .zip trae también {}, que ArchiSim no lee.",
                                others.join(", ")
                            ))
                            .small()
                            .color(pal.faint),
                        );
                    }
                    match &imp.read {
                        Ok(l) => {
                            let m = &l.mesh;
                            let (lo, hi) = m.bounds();
                            let s = [0, 1, 2].map(|k| (hi[k] - lo[k]) as f64);
                            let (w, d) = if imp.turns % 2 == 1 {
                                (s[1], s[0])
                            } else {
                                (s[0], s[1])
                            };
                            let textured =
                                m.materials.iter().filter(|x| x.texture.is_some()).count();
                            ui.label(
                                format!(
                                    "{:.2} × {:.2} m de planta, {:.2} m de alto · {} caras · {} materiales{}",
                                    w,
                                    d,
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
                            if let Some(k) = l.guessed_unit {
                                ui.label(
                                    egui::RichText::new(format!(
                                        "El fichero no dice sus unidades: leído en {} por su tamaño.",
                                        unit_name(k).to_lowercase()
                                    ))
                                    .small()
                                    .color(pal.faint),
                                );
                            }
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
                    // From above and from the front, the front marked.
                    let turns = imp.turns;
                    if let Some(p) = imp.pictures(ui.ctx()) {
                        ui.add_space(4.0);
                        ui.horizontal(|ui| {
                            for (tex, aspect, label) in [
                                (&p.top, p.aspect[0], "Planta"),
                                (&p.front, p.aspect[1], "Alzado frontal"),
                            ] {
                                ui.vertical(|ui| {
                                    ui.label(egui::RichText::new(label).small().color(pal.faint));
                                    let size = if aspect >= 1.0 {
                                        egui::vec2(120.0, 120.0 / aspect)
                                    } else {
                                        egui::vec2(120.0 * aspect, 120.0)
                                    };
                                    let (r, _) = ui.allocate_exact_size(
                                        egui::vec2(140.0, size.y + 18.0),
                                        egui::Sense::hover(),
                                    );
                                    let img = egui::Rect::from_min_size(
                                        r.min + egui::vec2((140.0 - size.x) / 2.0, 0.0),
                                        size,
                                    );
                                    ui.painter().image(
                                        tex.id(),
                                        img,
                                        egui::Rect::from_min_max(
                                            egui::pos2(0.0, 0.0),
                                            egui::pos2(1.0, 1.0),
                                        ),
                                        egui::Color32::WHITE,
                                    );
                                    if label == "Planta" {
                                        // Its front: the bottom side.
                                        let y = img.bottom() + 3.0;
                                        let c = img.center().x;
                                        ui.painter().arrow(
                                            egui::pos2(c, y),
                                            egui::vec2(0.0, 10.0),
                                            egui::Stroke::new(1.5, pal.accent),
                                        );
                                        ui.painter().text(
                                            egui::pos2(c + 8.0, y + 6.0),
                                            egui::Align2::LEFT_CENTER,
                                            "frente",
                                            egui::FontId::proportional(11.0),
                                            pal.accent,
                                        );
                                    }
                                });
                            }
                        });
                        ui.horizontal(|ui| {
                            if ui
                                .button("⟲ Girar")
                                .on_hover_text("Gira el modelo un cuarto de vuelta hasta que su frente quede abajo en la planta")
                                .clicked()
                            {
                                imp.turns = (turns + 1) % 4;
                            }
                            if ui.button("⟳").on_hover_text("Gira al otro lado").clicked() {
                                imp.turns = (turns + 3) % 4;
                            }
                        });
                    }
                    ui.add_space(6.0);
                    egui::Grid::new("mesh-import")
                        .num_columns(2)
                        .spacing([10.0, 6.0])
                        .show(ui, |ui| {
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
                                    Up::Y => "Eje Y (glTF y la mayoría de programas de animación)",
                                    Up::Z => "Eje Z (programas de arquitectura y modelado)",
                                })
                                .show_ui(ui, |ui| {
                                    for (u, n) in [
                                        (Up::Auto, "Lo que diga el fichero"),
                                        (Up::Y, "Eje Y (glTF y la mayoría de programas de animación)"),
                                        (Up::Z, "Eje Z (programas de arquitectura y modelado)"),
                                    ] {
                                        reread |= ui.selectable_value(&mut imp.up, u, n).changed();
                                    }
                                });
                            ui.end_row();
                            ui.label("Detalle");
                            egui::ComboBox::from_id_salt("mesh-detail")
                                .selected_text(DETAILS[imp.detail].0)
                                .show_ui(ui, |ui| {
                                    for (i, (n, _)) in DETAILS.iter().enumerate() {
                                        ui.selectable_value(&mut imp.detail, i, *n);
                                    }
                                })
                                .response
                                .on_hover_text("Los modelos de los fabricantes suelen tener cientos de miles de caras. Se aligeran sin perder su forma; de lejos se ven versiones aún más ligeras.");
                            ui.end_row();
                        });
                    ui.add_space(4.0);
                    ui.switch(&mut imp.design, "Pieza de diseño de un fabricante");
                    if imp.design {
                        egui::Grid::new("mesh-maker")
                            .num_columns(2)
                            .spacing([10.0, 4.0])
                            .show(ui, |ui| {
                                for (i, (_, label, _)) in Maker::FIELDS.iter().enumerate() {
                                    ui.label(*label);
                                    ui.add(
                                        egui::TextEdit::singleline(imp.maker.field_mut(i))
                                            .desired_width(260.0),
                                    );
                                    ui.end_row();
                                }
                            });
                        let (glb, _) = design_paths(&imp.name, &imp.maker);
                        let shown = glb
                            .strip_prefix(crate::editor::library::library_dir())
                            .map(|p| p.display().to_string())
                            .unwrap_or_default();
                        ui.switch(&mut imp.keep, format!("Guardarla en mi biblioteca ({shown})"));
                        ui.label(
                            egui::RichText::new(
                                "El modelo es del fabricante: se guarda en su biblioteca, no en ArchiSim. Sus datos van al IFC.",
                            )
                            .small()
                            .color(pal.faint),
                        );
                    }
                    ui.add_space(4.0);
                    ui.label(
                        egui::RichText::new(
                            "Lleva versiones más ligeras para verlo de lejos; su planta es su contorno; sus materiales toman la textura que dice su nombre (tela, madera, mármol…).",
                        )
                        .small()
                        .color(pal.faint),
                    );
                });
                });
                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    if busy {
                        ui.spinner();
                        ui.label("Aligerando el modelo sin perder su forma…");
                    } else if ui
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
