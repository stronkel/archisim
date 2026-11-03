//! The project's title blocks (cajetines): a side panel with the list, a
//! preview of the bottom right of an A3 sheet and what can be changed —
//! the layout it starts from (Clásico, Hilo, Faro), the studio's name,
//! monogram and logo, the accent colour, the stamp, the width and the rows
//! of data with their labels. Each sheet takes one in its properties; the
//! project's default is set here.

use archisim_core::{TitleBlock, TitleBlockDesign, TitleBlockId, TitleRow, TitleValue};
use eframe::egui::{self, RichText, TextureHandle, Ui};

use super::ArchiSimApp;

#[derive(Default)]
pub struct TitleBlocksUi {
    pub open: bool,
    pub selected: Option<TitleBlockId>,
    /// The title block being edited, written to the project when it stops
    /// changing.
    draft: Option<TitleBlock>,
    /// The preview and the title block it shows.
    preview: Option<(TitleBlock, TextureHandle)>,
}

/// Size of the preview: the bottom right of an A3 landscape sheet (its
/// whole right side for a strip down the sheet).
fn preview_mm(design: TitleBlockDesign) -> (f64, f64) {
    match design {
        TitleBlockDesign::Beacon => (150.0, 297.0),
        _ => (240.0, 160.0),
    }
}

impl ArchiSimApp {
    pub(super) fn title_blocks_dialog(&mut self, ctx: &egui::Context) {
        if !self.title_blocks_ui.open {
            return;
        }
        let mut open = true;
        super::widgets::side_sheet(ctx, "Cajetines", 640.0)
            .open(&mut open)
            .show(ctx, |ui| {
                egui::ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .show(ui, |ui| self.title_blocks_contents(ui));
            });
        self.title_blocks_ui.open = open;
        if !open {
            self.commit_title_block(true);
        }
    }

    fn title_blocks_contents(&mut self, ui: &mut Ui) {
        ui.label(
            RichText::new(
                "El marco y el cajetín de los planos. Cada plano elige el suyo en sus propiedades; los que no eligen ninguno llevan el de por defecto.",
            )
            .weak(),
        );
        ui.add_space(6.0);
        let list: Vec<(TitleBlockId, String)> = self
            .ed
            .model()
            .title_blocks()
            .map(|t| (t.id, t.name.clone()))
            .collect();
        let default = self.ed.model().default_title_block();
        if self.title_blocks_ui.selected.is_none() {
            self.title_blocks_ui.selected = default.or(list.first().map(|l| l.0));
        }
        let mut pick = None;
        ui.horizontal_wrapped(|ui| {
            for (id, name) in &list {
                let label = if Some(*id) == default {
                    format!("{name} ★")
                } else {
                    name.clone()
                };
                if ui
                    .selectable_label(self.title_blocks_ui.selected == Some(*id), label)
                    .on_hover_text("★: el de por defecto")
                    .clicked()
                {
                    pick = Some(*id);
                }
            }
        });
        if let Some(id) = pick {
            self.commit_title_block(true);
            self.title_blocks_ui.selected = Some(id);
            self.title_blocks_ui.draft = None;
        }
        ui.horizontal(|ui| {
            if ui
                .button("＋ Nuevo a partir de este")
                .on_hover_text("Una copia del seleccionado para hacerlo a tu manera")
                .clicked()
            {
                self.duplicate_title_block();
            }
            let sel = self.title_blocks_ui.selected;
            if ui
                .add_enabled(
                    sel.is_some() && sel != default,
                    egui::Button::new("Por defecto"),
                )
                .on_hover_text("Que lo lleven los planos que no eligen otro")
                .clicked()
            {
                let r = self
                    .ed
                    .transact("Cajetín por defecto", |m| m.set_default_title_block(sel));
                self.report(r, "Cajetín por defecto cambiado.");
            }
            if ui
                .add_enabled(sel.is_some(), egui::Button::new("Borrar"))
                .on_hover_text("Sus planos pasan al de por defecto")
                .clicked()
            {
                if let Some(id) = sel {
                    let r = self
                        .ed
                        .transact("Borrar cajetín", |m| m.remove_title_block(id).map(drop));
                    self.report(r, "Cajetín borrado.");
                    self.title_blocks_ui.selected = None;
                    self.title_blocks_ui.draft = None;
                }
            }
        });
        ui.separator();
        let Some(current) = self
            .title_blocks_ui
            .selected
            .and_then(|id| self.ed.model().title_block(id).cloned())
        else {
            ui.label(RichText::new("No hay cajetines: crea uno.").weak());
            if list.is_empty() && ui.button("Añadir Clásico, Hilo y Faro").clicked() {
                let r = self.ed.transact("Cajetines", |m| {
                    archisim_library::standards::add_schemes(m);
                    Ok(())
                });
                self.report(r, "Cajetines añadidos.");
            }
            return;
        };
        self.title_block_preview(ui, &current);
        let mut logo_pick = false;
        let d = self.title_blocks_ui.draft.get_or_insert(current);
        egui::Grid::new("title-block-form")
            .num_columns(2)
            .spacing([12.0, 8.0])
            .show(ui, |ui| {
                ui.label("Nombre");
                ui.add(egui::TextEdit::singleline(&mut d.name).desired_width(240.0));
                ui.end_row();

                ui.label("Composición")
                    .on_hover_text("Clásico: un bloque de celdas abajo a la derecha. Hilo: una columna estrecha con monograma. Faro: una franja a toda la altura del plano");
                ui.horizontal(|ui| {
                    for design in TitleBlockDesign::ALL {
                        if ui
                            .selectable_label(d.design == design, design.label())
                            .clicked()
                            && d.design != design
                        {
                            d.design = design;
                            d.width = design.default_width();
                        }
                    }
                });
                ui.end_row();

                ui.label("Ancho (mm)");
                ui.add(egui::DragValue::new(&mut d.width).range(30.0..=250.0).speed(0.5));
                ui.end_row();

                ui.label("Color de acento")
                    .on_hover_text("Rótulos, sello, número del plano y títulos de las vistas");
                ui.color_edit_button_srgb(&mut d.accent);
                ui.end_row();

                ui.label("Estudio")
                    .on_hover_text("Vacío: el nombre del estudio de los datos del proyecto");
                ui.add(
                    egui::TextEdit::singleline(&mut d.studio)
                        .hint_text("el del proyecto")
                        .desired_width(240.0),
                );
                ui.end_row();

                ui.label("Logotipo")
                    .on_hover_text("Una imagen PNG en lugar del nombre o del monograma");
                ui.horizontal(|ui| {
                    match &d.logo {
                        Some(l) => {
                            ui.label(RichText::new(&l.source).weak());
                            if ui.small_button("Quitar").clicked() {
                                d.logo = None;
                            }
                        }
                        None => {
                            ui.label(RichText::new("sin logotipo").weak());
                        }
                    }
                    if ui.small_button("Elegir PNG…").clicked() {
                        logo_pick = true;
                    }
                });
                ui.end_row();

                if d.design == TitleBlockDesign::Thread {
                    ui.label("Monograma")
                        .on_hover_text("Vacío: la inicial del estudio con un punto");
                    ui.add(
                        egui::TextEdit::singleline(&mut d.monogram)
                            .hint_text("A.")
                            .desired_width(80.0),
                    );
                    ui.end_row();
                }

                ui.label("Sello")
                    .on_hover_text("Se ve mientras la fase del proyecto no es la de ejecución; vacío, sin sello");
                ui.add(
                    egui::TextEdit::singleline(&mut d.stamp)
                        .hint_text("sin sello")
                        .desired_width(240.0),
                );
                ui.end_row();
            });
        ui.add_space(6.0);
        ui.label(RichText::new("Filas de datos").strong())
            .on_hover_text(match d.design {
                TitleBlockDesign::Classic => {
                    "Las celdas de la fila de abajo, de izquierda a derecha"
                }
                TitleBlockDesign::Thread => "De arriba abajo, sobre el número del plano",
                TitleBlockDesign::Beacon => "Bajo el proyecto, antes de las revisiones",
            });
        let mut remove = None;
        let mut swap = None;
        let n = d.rows.len();
        for (i, r) in d.rows.iter_mut().enumerate() {
            ui.horizontal(|ui| {
                ui.add(egui::TextEdit::singleline(&mut r.label).desired_width(120.0));
                let is_text = matches!(r.value, TitleValue::Text(_));
                egui::ComboBox::from_id_salt(("tb-row", i))
                    .width(150.0)
                    .selected_text(r.value.label())
                    .show_ui(ui, |ui| {
                        for k in TitleValue::KINDS {
                            let on = r.value == k;
                            if ui.selectable_label(on, k.label()).clicked() {
                                r.value = k;
                            }
                        }
                        if ui.selectable_label(is_text, "Texto propio").clicked() && !is_text {
                            r.value = TitleValue::Text(String::new());
                        }
                    });
                if let TitleValue::Text(t) = &mut r.value {
                    ui.add(egui::TextEdit::singleline(t).desired_width(120.0));
                }
                if ui
                    .add_enabled(i > 0, egui::Button::new("↑").small())
                    .clicked()
                {
                    swap = Some((i - 1, i));
                }
                if ui
                    .add_enabled(i + 1 < n, egui::Button::new("↓").small())
                    .clicked()
                {
                    swap = Some((i, i + 1));
                }
                if ui.small_button("✕").clicked() {
                    remove = Some(i);
                }
            });
        }
        if let Some((a, b)) = swap {
            d.rows.swap(a, b);
        }
        if let Some(i) = remove {
            d.rows.remove(i);
        }
        if d.rows.len() < 16 && ui.button("＋ Fila").clicked() {
            d.rows
                .push(TitleRow::new("DATO", TitleValue::Text(String::new())));
        }
        if logo_pick {
            self.pick_title_logo();
        }
        let busy =
            ui.ctx().input(|i| i.pointer.any_down()) || ui.ctx().memory(|m| m.focused().is_some());
        self.commit_title_block_if(false, busy);
    }

    /// The bottom right of an A3 sheet with the title block being edited.
    fn title_block_preview(&mut self, ui: &mut Ui, current: &TitleBlock) {
        let shown = self
            .title_blocks_ui
            .draft
            .clone()
            .unwrap_or_else(|| current.clone());
        let stale = self
            .title_blocks_ui
            .preview
            .as_ref()
            .is_none_or(|(t, _)| *t != shown);
        if stale {
            let mut m = archisim_core::Model::new(&self.ed.model().project_name);
            m.set_info(self.ed.model().info().clone());
            let mut t = shown.clone();
            t.name = "preview".into();
            let tb = m.add_title_block(t).ok();
            let sheet = m
                .add_sheet(archisim_core::NewSheet {
                    number: "A-01".into(),
                    name: "Planta baja. Distribución".into(),
                    width: 420.0,
                    height: 297.0,
                })
                .ok();
            if let (Some(sheet), Some(tb)) = (sheet, tb) {
                if let Some(mut s) = m.sheet(sheet).cloned() {
                    s.title_type = Some(tb);
                    s.revision = "0".into();
                    let _ = m.set_sheet(s);
                }
                let (w, h) = preview_mm(shown.design);
                let mm = 0.001;
                let options = archisim_plan::sheet::SheetOptions {
                    region: Some((
                        archisim_core::Point2::new((420.0 - w) * mm, 0.0),
                        archisim_core::Point2::new(420.0 * mm, h * mm),
                    )),
                    ..Default::default()
                };
                let svg = archisim_plan::sheet::render_sheet(&m, sheet, &options);
                let px = 3.0;
                let image =
                    crate::plan_image::rasterize_svg(&svg, [(w * px) as usize, (h * px) as usize]);
                let tex = ui.ctx().load_texture(
                    "title-block-preview",
                    image,
                    egui::TextureOptions::LINEAR,
                );
                self.title_blocks_ui.preview = Some((shown, tex));
            }
        }
        if let Some((t, tex)) = &self.title_blocks_ui.preview {
            let (pw, ph) = preview_mm(t.design);
            let w = ui
                .available_width()
                .min(560.0)
                .min(360.0 * (pw / ph) as f32);
            let size = egui::vec2(w, w * (ph / pw) as f32);
            ui.add(egui::Image::new((tex.id(), size)));
            ui.label(
                RichText::new("La esquina de un A3 con este cajetín y los datos del proyecto.")
                    .small()
                    .weak(),
            );
        }
        ui.add_space(6.0);
    }

    fn pick_title_logo(&mut self) {
        let Some(path) = rfd::FileDialog::new()
            .set_title("Logotipo del cajetín")
            .add_filter("Imagen PNG", &["png", "PNG"])
            .pick_file()
        else {
            return;
        };
        match std::fs::read(&path) {
            Ok(bytes) => {
                let name = path
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_default();
                match archisim_core::decal_image(&name, bytes) {
                    Some(img) => {
                        if let Some(d) = &mut self.title_blocks_ui.draft {
                            d.logo = Some(img);
                        }
                    }
                    None => self.complain("Ese fichero no es una imagen PNG."),
                }
            }
            Err(e) => self.complain(format!("No se pudo leer la imagen: {e}")),
        }
    }

    /// A copy of the selected title block (or a classic one), selected.
    pub(super) fn duplicate_title_block(&mut self) {
        self.commit_title_block(true);
        let base = self
            .title_blocks_ui
            .selected
            .and_then(|id| self.ed.model().title_block(id).cloned())
            .unwrap_or_else(|| TitleBlock::standard(TitleBlockDesign::Classic));
        let taken = |n: &str| self.ed.model().title_blocks().any(|t| t.name == n);
        let name = (1..)
            .map(|i| {
                if i == 1 {
                    format!("{} (propio)", base.name)
                } else {
                    format!("{} (propio {i})", base.name)
                }
            })
            .find(|n| !taken(n))
            .unwrap_or_default();
        let t = TitleBlock { name, ..base };
        match self.ed.transact("Nuevo cajetín", |m| m.add_title_block(t)) {
            Ok(id) => {
                self.title_blocks_ui.selected = Some(id);
                self.title_blocks_ui.draft = None;
            }
            Err(e) => self.complain(format!("No se puede crear el cajetín: {e}.")),
        }
    }

    fn commit_title_block(&mut self, now: bool) {
        self.commit_title_block_if(now, false);
    }

    /// Writes the title block being edited to the project, if it changed
    /// (and, unless `now`, nothing is being dragged or typed).
    fn commit_title_block_if(&mut self, now: bool, busy: bool) {
        if busy && !now {
            return;
        }
        let Some(d) = self.title_blocks_ui.draft.clone() else {
            return;
        };
        if self.ed.model().title_block(d.id) == Some(&d) {
            return;
        }
        let r = self
            .ed
            .transact("Cambiar cajetín", |m| m.set_title_block(d));
        if let Err(e) = r {
            self.complain(format!("No se puede guardar el cajetín: {e}."));
            self.title_blocks_ui.draft = None;
        }
    }
}
