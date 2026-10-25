//! The curtain wall types window ("Tipos de muro cortina"), as ArchiCAD's
//! curtain wall settings but simpler and laid out to be read top to
//! bottom: on the left the project's types, in the middle what a type is
//! (scheme of columns and rows, frames, joints, panels and which row takes
//! which, accessories), on the right how it looks, always up to date: its
//! elevation with the bay and row sizes, a plan detail, a vertical section
//! and a 3D view, on a sample wall (that of the selected curtain wall when
//! opened from it).
//!
//! Every change is one undoable step and every curtain wall of the type
//! follows it; a change that would leave a wall of the type without room
//! between its frames is refused.

use super::widgets::SwitchExt as _;
use archisim_core::{
    CurtainCaps, CurtainDesign, CurtainDivision, CurtainJoint, CurtainLouvres, CurtainProfile,
    CurtainRest, CurtainTypeId, MaterialId, Model,
};
use eframe::egui::{self, Align2, Color32, FontId, Id, Pos2, Rect, Sense, Ui, Vec2};

use super::theme::Palette;
use super::type_views::{paint_drawing, paper, side_labels};
use super::widgets::segmented;
use super::{length_field, text_edit_committed, ArchiSimApp, View3dState};
use crate::units::fmt_mm;

/// What the preview shows.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Hash)]
pub enum CurtainView {
    #[default]
    Elevation,
    Plan,
    Section,
    ThreeD,
}

/// The state of the window.
pub struct CurtainTypesWindow {
    pub open: bool,
    pub selected: Option<CurtainTypeId>,
    /// Length and height of the sample wall of the preview.
    pub sample: (f64, f64),
    pub view: CurtainView,
    pub(super) v3d: View3dState,
}

impl Default for CurtainTypesWindow {
    fn default() -> Self {
        Self {
            open: false,
            selected: None,
            sample: (4.8, 3.0),
            view: CurtainView::Elevation,
            v3d: View3dState::default(),
        }
    }
}

/// Name, colour and hatch of the materials of a design: pictures change
/// when they do.
pub(super) fn materials_signature(model: &Model, d: &CurtainDesign) -> Vec<(String, [u8; 3])> {
    d.materials()
        .into_iter()
        .filter_map(|m| model.material(m))
        .map(|m| (format!("{}:{:?}", m.name, m.cut_pattern), m.cut_color))
        .collect()
}

const LIST: f32 = 220.0;
const EDITOR: f32 = 400.0;
const FIELD: f32 = 74.0;

/// A combo of the project's materials, by name, with its colour.
fn material_combo(ui: &mut Ui, id: Id, model: &Model, value: &mut MaterialId, width: f32) {
    let mut mats: Vec<(MaterialId, String, [u8; 3])> = model
        .materials()
        .map(|m| (m.id, m.name.clone(), m.cut_color))
        .collect();
    mats.sort_by_key(|a| a.1.to_lowercase());
    let shown = model
        .material(*value)
        .map_or("—".to_string(), |m| m.name.clone());
    egui::ComboBox::from_id_salt(id)
        .width(width)
        .truncate()
        .selected_text(shown)
        .show_ui(ui, |ui| {
            for (mid, name, rgb) in &mats {
                ui.horizontal(|ui| {
                    super::props::swatch(ui, *rgb, 12.0);
                    ui.selectable_value(value, *mid, name);
                });
            }
        });
}

/// A section heading of the editor column.
fn heading(ui: &mut Ui, text: &str, hint: &str) {
    let p = Palette::current(ui.ctx());
    ui.add_space(10.0);
    ui.label(egui::RichText::new(text).strong().color(p.text));
    if !hint.is_empty() {
        ui.label(egui::RichText::new(hint).small().color(p.muted));
    }
    ui.add_space(2.0);
}

/// The fixed bays and the rest of a division. Returns what changed.
fn division_ui(
    ui: &mut Ui,
    salt: &str,
    d: &mut CurtainDivision,
    rows: bool,
) -> Option<&'static str> {
    let mut changed = None;
    let (fixed_label, rest_label) = if rows {
        ("Filas fijas desde abajo", "Filas")
    } else {
        ("Paños fijos desde el inicio", "Paños")
    };
    ui.horizontal_wrapped(|ui| {
        ui.label(fixed_label);
        let mut remove = None;
        for (i, v) in d.fixed.iter_mut().enumerate() {
            if let Some(x) = length_field(ui, Id::unique((salt, "fixed", i)), *v, 58.0) {
                *v = x;
                changed = Some("Paño fijo");
            }
            if ui
                .small_button("✕")
                .on_hover_text("Quitar este paño fijo")
                .clicked()
            {
                remove = Some(i);
            }
        }
        if let Some(i) = remove {
            d.fixed.remove(i);
            changed = Some("Quitar paño fijo");
        }
        if ui
            .small_button("+ fijo")
            .on_hover_text(if rows {
                "Una fila de altura fija (un antepecho, un zócalo…) antes del resto"
            } else {
                "Un paño de ancho fijo (una puerta, un módulo de esquina…) antes del resto"
            })
            .clicked()
        {
            d.fixed.push(d.fixed.last().copied().unwrap_or(1.0));
            changed = Some("Añadir paño fijo");
        }
    });
    ui.horizontal(|ui| {
        ui.label(if d.fixed.is_empty() {
            rest_label
        } else {
            "El resto"
        });
        let equal = matches!(d.rest, CurtainRest::MaxSpacing(_));
        let mut mode = equal;
        egui::ComboBox::from_id_salt((salt, "mode"))
            .width(170.0)
            .selected_text(if equal {
                "iguales, sin pasar de"
            } else {
                "en partes iguales:"
            })
            .show_ui(ui, |ui| {
                ui.selectable_value(&mut mode, true, "iguales, sin pasar de");
                ui.selectable_value(&mut mode, false, "en partes iguales:");
            });
        if mode != equal {
            d.rest = if mode {
                CurtainRest::MaxSpacing(1.2)
            } else {
                CurtainRest::Count(3)
            };
            changed = Some("Reparto");
        }
        match &mut d.rest {
            CurtainRest::MaxSpacing(v) => {
                if let Some(x) = length_field(ui, Id::unique((salt, "max")), *v, FIELD) {
                    *v = x;
                    changed = Some("Módulo máximo");
                }
                ui.label("mm");
            }
            CurtainRest::Count(n) => {
                let mut k = *n;
                ui.add(egui::DragValue::new(&mut k).range(1..=100).speed(0.05));
                if k != *n {
                    *n = k;
                    changed = Some("Número de partes");
                }
            }
        }
    });
    changed
}

/// Width, depth and material of a frame on one line.
fn profile_ui(ui: &mut Ui, model: &Model, salt: &str, label: &str, p: &mut CurtainProfile) -> bool {
    let mut changed = false;
    ui.horizontal(|ui| {
        ui.add_sized([96.0, 20.0], egui::Label::new(label).truncate());
        if let Some(v) = length_field(ui, Id::unique((salt, "w")), p.width, 52.0) {
            p.width = v;
            changed = true;
        }
        ui.label("×");
        if let Some(v) = length_field(ui, Id::unique((salt, "d")), p.depth, 52.0) {
            p.depth = v;
            changed = true;
        }
        let mut m = p.material;
        material_combo(ui, Id::unique((salt, "m")), model, &mut m, 120.0);
        if m != p.material {
            p.material = m;
            changed = true;
        }
    });
    changed
}

impl ArchiSimApp {
    /// Opens the window on a type, with a sample wall of `sample` (length,
    /// height) if given.
    pub(super) fn open_curtain_types(
        &mut self,
        selected: Option<CurtainTypeId>,
        sample: Option<(f64, f64)>,
    ) {
        let w = &mut self.curtain_types;
        w.open = true;
        if selected.is_some() {
            w.selected = selected;
        }
        if let Some(s) = sample {
            w.sample = s;
        }
    }

    pub(super) fn curtain_types_dialog(&mut self, ctx: &egui::Context) {
        if !self.curtain_types.open {
            return;
        }
        let mut open = true;
        super::widgets::dialog("Tipos de muro cortina")
            .open(&mut open)
            .default_size([1080.0, 680.0])
            .resizable(true)
            .collapsible(false)
            .show(ctx, |ui| self.curtain_types_body(ui));
        self.curtain_types.open = open && self.curtain_types.open;
    }

    fn curtain_types_body(&mut self, ui: &mut Ui) {
        // The selected type, or the first.
        let model = self.ed.model();
        let sel = self
            .curtain_types
            .selected
            .filter(|t| model.curtain_type(*t).is_some())
            .or_else(|| model.curtain_types().next().map(|t| t.id));
        self.curtain_types.selected = sel;
        let height = ui.available_height().max(480.0);
        ui.horizontal_top(|ui| {
            ui.allocate_ui_with_layout(
                Vec2::new(LIST, height),
                egui::Layout::top_down(egui::Align::Min),
                |ui| {
                    ui.set_width(LIST);
                    self.curtain_type_list(ui, sel);
                },
            );
            ui.separator();
            let Some(id) = sel else {
                ui.label("El proyecto no tiene tipos de muro cortina.");
                return;
            };
            ui.allocate_ui_with_layout(
                Vec2::new(EDITOR, height),
                egui::Layout::top_down(egui::Align::Min),
                |ui| {
                    ui.set_width(EDITOR);
                    egui::ScrollArea::vertical()
                        .id_salt("curtain-type-editor")
                        .show(ui, |ui| self.curtain_type_editor(ui, id));
                },
            );
            ui.separator();
            ui.vertical(|ui| self.curtain_type_preview(ui, id));
        });
    }

    fn curtain_type_list(&mut self, ui: &mut Ui, sel: Option<CurtainTypeId>) {
        let p = Palette::current(ui.ctx());
        ui.label(egui::RichText::new("Tipos del proyecto").strong());
        ui.add_space(4.0);
        let ctx = ui.ctx().clone();
        let mut picked = None;
        egui::ScrollArea::vertical()
            .id_salt("curtain-type-list")
            .max_height(ui.available_height() - 40.0)
            .show(ui, |ui| {
                let model = self.ed.model();
                let thumbs = &mut self.thumbs;
                for t in model.curtain_types() {
                    let users = model.curtain_type_users(t.id);
                    let key = format!(
                        "curtain-list:{:?}:{:?}",
                        t.design,
                        materials_signature(model, &t.design)
                    );
                    let thumb = thumbs.get_drawing(&ctx, key, Vec2::new(64.0, 48.0), || {
                        let m = crate::type_preview::curtain_model(model, &t.design, 3.6, 3.0)?;
                        crate::type_drawing::curtain_elevation(&m).map(|x| x.0)
                    });
                    let on = Some(t.id) == sel;
                    let (rect, resp) = ui
                        .allocate_exact_size(Vec2::new(ui.available_width(), 56.0), Sense::click());
                    if on {
                        ui.painter()
                            .rect_filled(rect, egui::CornerRadius::same(6), p.accent_soft);
                    } else if resp.hovered() {
                        ui.painter()
                            .rect_filled(rect, egui::CornerRadius::same(6), p.raised);
                    }
                    let pic =
                        Rect::from_min_size(rect.min + Vec2::new(4.0, 4.0), Vec2::new(64.0, 48.0));
                    ui.painter()
                        .rect_filled(pic, egui::CornerRadius::same(3), Color32::WHITE);
                    if let Some(th) = thumb {
                        let size = th.fit(pic.size());
                        ui.painter().image(
                            th.tex,
                            Rect::from_center_size(pic.center(), size),
                            Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)),
                            Color32::WHITE,
                        );
                    }
                    let text_x = pic.right() + 8.0;
                    let job = egui::text::LayoutJob {
                        wrap: egui::text::TextWrapping {
                            max_width: rect.right() - text_x - 4.0,
                            max_rows: 2,
                            break_anywhere: false,
                            overflow_character: Some('…'),
                        },
                        ..egui::text::LayoutJob::single_section(
                            t.name.clone(),
                            egui::TextFormat::simple(FontId::proportional(12.5), p.text),
                        )
                    };
                    let name = ui.painter().layout_job(job);
                    let below = rect.top() + 6.0 + name.size().y + 2.0;
                    ui.painter()
                        .galley(Pos2::new(text_x, rect.top() + 6.0), name, p.text);
                    ui.painter().text(
                        Pos2::new(text_x, below),
                        Align2::LEFT_TOP,
                        match users {
                            0 => "sin usar".to_string(),
                            1 => "1 muro".to_string(),
                            n => format!("{n} muros"),
                        },
                        FontId::proportional(11.0),
                        p.muted,
                    );
                    if resp.clicked() {
                        picked = Some(t.id);
                    }
                }
            });
        if let Some(t) = picked {
            self.curtain_types.selected = Some(t);
        }
        ui.add_space(6.0);
        ui.horizontal(|ui| {
            if ui
                .button("Nuevo")
                .on_hover_text("Un tipo sencillo: montantes y travesaños de aluminio, vidrio")
                .clicked()
            {
                self.new_curtain_type();
            }
            if ui
                .add_enabled(sel.is_some(), egui::Button::new("Duplicar"))
                .clicked()
            {
                if let Some(id) = sel {
                    match self.ed.duplicate_curtain_type(id) {
                        Ok(t) => self.curtain_types.selected = Some(t),
                        Err(e) => self.complain(format!("No se puede duplicar: {e}.")),
                    }
                }
            }
            let used = sel.is_some_and(|t| self.ed.model().curtain_type_users(t) > 0);
            if ui
                .add_enabled(sel.is_some() && !used, egui::Button::new("Borrar"))
                .on_disabled_hover_text("Lo usan muros cortina del proyecto")
                .clicked()
            {
                if let Some(id) = sel {
                    let r = self.ed.remove_curtain_type(id);
                    self.report(r, "Tipo borrado.");
                }
            }
        });
    }

    fn new_curtain_type(&mut self) {
        let model = self.ed.model();
        let find = |cat: &str| model.materials().find(|m| m.category == cat).map(|m| m.id);
        let first = model.materials().next().map(|m| m.id);
        let (Some(frame), Some(glass)) = (find("aluminium").or(first), find("glass").or(first))
        else {
            self.complain("El proyecto no tiene materiales.");
            return;
        };
        let taken: Vec<String> = model.curtain_types().map(|t| t.name.clone()).collect();
        let name = (1..)
            .map(|n| match n {
                1 => "Muro cortina nuevo".to_string(),
                n => format!("Muro cortina nuevo {n}"),
            })
            .find(|n| !taken.contains(n))
            .expect("some name is free");
        let design = CurtainDesign::simple(
            1.2,
            1.5,
            CurtainProfile {
                width: 0.05,
                depth: 0.15,
                material: frame,
            },
            0.024,
            glass,
        );
        match self.ed.add_curtain_type(&name, design) {
            Ok(t) => self.curtain_types.selected = Some(t),
            Err(e) => self.complain(format!("No se puede crear el tipo: {e}.")),
        }
    }

    fn curtain_type_editor(&mut self, ui: &mut Ui, id: CurtainTypeId) {
        let p = Palette::current(ui.ctx());
        let model = self.ed.model();
        let Some(t) = model.curtain_type(id) else {
            return;
        };
        let (name, before) = (t.name.clone(), t.design.clone());
        let mut d = before.clone();
        let mut label: Option<&str> = None;
        let mut new_name = None;
        ui.horizontal(|ui| {
            ui.label("Nombre");
            new_name = text_edit_committed(ui, Id::unique(("curtain-name", id.0)), &name, 300.0);
        });
        let users = model.curtain_type_users(id);
        ui.label(
            egui::RichText::new(match users {
                0 => "Ningún muro cortina usa este tipo.".to_string(),
                1 => "Lo usa 1 muro cortina: cambia con el tipo.".to_string(),
                n => format!("Lo usan {n} muros cortina: todos cambian con el tipo."),
            })
            .small()
            .color(p.muted),
        );

        heading(
            ui,
            "1 · Esquema: columnas y filas",
            "Dónde van los montantes (a lo largo) y los travesaños (en altura), entre ejes.",
        );
        ui.label(egui::RichText::new("A lo largo").color(p.muted));
        if let Some(l) = division_ui(ui, &format!("cols{}", id.0), &mut d.columns, false) {
            label = Some(l);
        }
        ui.add_space(4.0);
        ui.label(egui::RichText::new("En altura").color(p.muted));
        if let Some(l) = division_ui(ui, &format!("rows{}", id.0), &mut d.rows, true) {
            label = Some(l);
        }

        heading(
            ui,
            "2 · Marcos",
            "Ancho visto desde fuera × fondo, en mm, y material. El marco perimetral queda dentro del contorno.",
        );
        for (salt, text, prof) in [
            ("border", "Perimetral", &mut d.border),
            ("mullion", "Montantes", &mut d.mullion),
            ("transom", "Travesaños", &mut d.transom),
        ] {
            if profile_ui(ui, model, &format!("{salt}{}", id.0), text, prof) {
                label = Some("Marcos");
            }
        }

        heading(
            ui,
            "3 · Uniones",
            "Qué perfiles pasan corridos donde se cruzan montantes y travesaños.",
        );
        let mut joint = d.joint;
        ui.allocate_ui(Vec2::new(EDITOR - 10.0, 30.0), |ui| {
            segmented(
                ui,
                ("curtain-joint", id.0),
                &mut joint,
                &[
                    (
                        CurtainJoint::Mullions,
                        "Montantes corridos",
                        "Los montantes suben de arriba abajo; los travesaños van entre ellos (sistema de montante y travesaño)",
                    ),
                    (
                        CurtainJoint::Transoms,
                        "Travesaños corridos",
                        "Los travesaños van de punta a punta; los montantes entre ellos (ventanal corrido)",
                    ),
                ],
            );
        });
        if joint != d.joint {
            d.joint = joint;
            label = Some("Uniones");
        }

        heading(
            ui,
            "4 · Paneles",
            "Los tipos de panel del muro (vidrio, antepecho opaco…) y cuál lleva cada fila.",
        );
        let mut remove = None;
        let n_panels = d.panels.len();
        for (k, panel) in d.panels.iter_mut().enumerate() {
            ui.horizontal(|ui| {
                if let Some(v) =
                    text_edit_committed(ui, Id::unique(("panel-name", id.0, k)), &panel.name, 96.0)
                {
                    panel.name = v.trim().to_string();
                    label = Some("Nombre del panel");
                }
                if let Some(v) =
                    length_field(ui, Id::unique(("panel-t", id.0, k)), panel.thickness, 52.0)
                {
                    panel.thickness = v;
                    label = Some("Espesor del panel");
                }
                ui.label("mm");
                let mut m = panel.material;
                material_combo(ui, Id::unique(("panel-m", id.0, k)), model, &mut m, 120.0);
                if m != panel.material {
                    panel.material = m;
                    label = Some("Material del panel");
                }
                if n_panels > 1
                    && ui
                        .small_button("✕")
                        .on_hover_text("Quitar este panel: sus filas pasan al primero")
                        .clicked()
                {
                    remove = Some(k);
                }
            });
        }
        if let Some(k) = remove {
            d.panels.remove(k);
            for r in &mut d.row_panels {
                if *r == k {
                    *r = 0;
                } else if *r > k {
                    *r -= 1;
                }
            }
            label = Some("Quitar panel");
        }
        if ui
            .small_button("+ panel")
            .on_hover_text("Otro tipo de panel, para darlo a alguna fila")
            .clicked()
        {
            let mut copy = d.panels.last().cloned().expect("at least one panel");
            copy.name = format!("Panel {}", d.panels.len() + 1);
            d.panels.push(copy);
            label = Some("Añadir panel");
        }
        // Which panel each row of the sample takes.
        let (len, high) = self.curtain_types.sample;
        let rows = archisim_core::curtain_grid(&d, len, high).transoms;
        if d.panels.len() > 1 {
            ui.add_space(4.0);
            ui.label(
                egui::RichText::new("Panel de cada fila, desde abajo (las de más arriba repiten la última de la lista)")
                    .small()
                    .color(p.muted),
            );
            let names: Vec<(usize, String)> = d
                .panels
                .iter()
                .enumerate()
                .map(|(i, p)| (i, p.name.clone()))
                .collect();
            let opts: Vec<(usize, &str)> = names.iter().map(|(i, n)| (*i, n.as_str())).collect();
            for r in (0..rows.len() - 1).rev() {
                let current = d.panel_of_row(r);
                let mut picked = current;
                ui.horizontal(|ui| {
                    ui.add_sized(
                        [130.0, 20.0],
                        egui::Label::new(format!(
                            "Fila {} · {}",
                            r + 1,
                            crate::units::fmt_len(rows[r + 1] - rows[r])
                        )),
                    );
                    picked =
                        super::props::choice(ui, ("row-panel", id.0, r), 170.0, current, &opts);
                });
                if picked != current {
                    let mut list: Vec<usize> = (0..=r.max(d.row_panels.len().saturating_sub(1)))
                        .map(|i| d.panel_of_row(i))
                        .collect();
                    list[r] = picked;
                    while list.len() > 1 && list[list.len() - 1] == list[list.len() - 2] {
                        list.pop();
                    }
                    if list == [0] {
                        list.clear();
                    }
                    d.row_panels = list;
                    label = Some("Panel de la fila");
                }
            }
        }

        heading(
            ui,
            "5 · Accesorios",
            "Por fuera de los marcos: tapajuntas sobre cada perfil y lamas horizontales de sombra.",
        );
        let mut caps = d.caps.is_some();
        if ui.switch(&mut caps, "Tapajuntas exteriores").changed() {
            d.caps = caps.then_some(CurtainCaps {
                width: d.mullion.width + 0.01,
                depth: 0.02,
                material: d.mullion.material,
            });
            label = Some("Tapajuntas");
        }
        if let Some(c) = &mut d.caps {
            ui.horizontal(|ui| {
                ui.add_space(20.0);
                if let Some(v) = length_field(ui, Id::unique(("cap-w", id.0)), c.width, 52.0) {
                    c.width = v;
                    label = Some("Tapajuntas");
                }
                ui.label("×");
                if let Some(v) = length_field(ui, Id::unique(("cap-d", id.0)), c.depth, 52.0) {
                    c.depth = v;
                    label = Some("Tapajuntas");
                }
                let mut m = c.material;
                material_combo(ui, Id::unique(("cap-m", id.0)), model, &mut m, 120.0);
                if m != c.material {
                    c.material = m;
                    label = Some("Tapajuntas");
                }
            });
        }
        let mut louvres = d.louvres.is_some();
        if ui.switch(&mut louvres, "Lamas horizontales").changed() {
            d.louvres = louvres.then_some(CurtainLouvres {
                depth: 0.2,
                thickness: 0.03,
                spacing: 0.35,
                offset: 0.1,
                material: d.mullion.material,
            });
            label = Some("Lamas");
        }
        if let Some(l) = &mut d.louvres {
            for (k, (text, v)) in [
                ("Vuelo", &mut l.depth),
                ("Espesor", &mut l.thickness),
                ("Separación", &mut l.spacing),
                ("Distancia al marco", &mut l.offset),
            ]
            .into_iter()
            .enumerate()
            {
                ui.horizontal(|ui| {
                    ui.add_space(20.0);
                    ui.add_sized([120.0, 20.0], egui::Label::new(text));
                    if let Some(x) = length_field(ui, Id::unique(("louvre", id.0, k)), *v, 60.0) {
                        *v = x;
                        label = Some("Lamas");
                    }
                    ui.label("mm");
                });
            }
            ui.horizontal(|ui| {
                ui.add_space(20.0);
                ui.add_sized([120.0, 20.0], egui::Label::new("Material"));
                let mut m = l.material;
                material_combo(ui, Id::unique(("louvre-m", id.0)), model, &mut m, 140.0);
                if m != l.material {
                    l.material = m;
                    label = Some("Lamas");
                }
            });
        }

        // Apply.
        if let Some(n) = new_name {
            let r = self
                .ed
                .edit_curtain_type("Nombre del tipo", id, &n, before.clone());
            self.report(r, "Tipo renombrado.");
        } else if d != before {
            let step = format!("Tipo de muro cortina: {}", label.unwrap_or("cambio"));
            if let Err(e) = self.ed.edit_curtain_type(&step, id, &name, d) {
                self.complain(format!("No se puede: {e}."));
            }
        }
    }

    fn curtain_type_preview(&mut self, ui: &mut Ui, id: CurtainTypeId) {
        let p = Palette::current(ui.ctx());
        let model = self.ed.model();
        let Some(t) = model.curtain_type(id) else {
            return;
        };
        let design = t.design.clone();
        let mut view = self.curtain_types.view;
        ui.allocate_ui(Vec2::new(ui.available_width(), 30.0), |ui| {
            segmented(
                ui,
                "curtain-view",
                &mut view,
                &[
                    (
                        CurtainView::Elevation,
                        "Alzado",
                        "Visto desde fuera, con los paños y las filas",
                    ),
                    (
                        CurtainView::Plan,
                        "Planta",
                        "Detalle cortado a media altura de la fila",
                    ),
                    (
                        CurtainView::Section,
                        "Sección",
                        "Cortado por el centro del primer paño",
                    ),
                    (CurtainView::ThreeD, "3D", ""),
                ],
            );
        });
        self.curtain_types.view = view;
        ui.add_space(4.0);
        let (mut len, mut high) = self.curtain_types.sample;
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new("Muestra").color(p.muted));
            if let Some(v) = length_field(ui, Id::unique("curtain-sample-l"), len, 64.0) {
                len = v.clamp(0.5, 60.0);
            }
            ui.label("×");
            if let Some(v) = length_field(ui, Id::unique("curtain-sample-h"), high, 64.0) {
                high = v.clamp(0.3, 30.0);
            }
            ui.label(
                egui::RichText::new("mm (largo × alto)")
                    .small()
                    .color(p.muted),
            );
        });
        self.curtain_types.sample = (len, high);
        let size = Vec2::new(
            ui.available_width(),
            (ui.available_height() - 40.0).max(260.0),
        );
        let Some(sample) = crate::type_preview::curtain_model(model, &design, len, high) else {
            ui.add_space(20.0);
            ui.label(format!(
                "Este tipo no cabe en un muro de {} × {}: los marcos no dejan sitio a los paneles. Prueba con una muestra mayor.",
                fmt_mm(len),
                crate::units::fmt_len(high)
            ));
            return;
        };
        let g = archisim_core::curtain_grid(&design, len, high);
        match view {
            CurtainView::ThreeD => {
                let key = {
                    use std::hash::{Hash, Hasher};
                    let mut h = std::collections::hash_map::DefaultHasher::new();
                    format!(
                        "{design:?}{:?}{len}{high}",
                        materials_signature(model, &design)
                    )
                    .hash(&mut h);
                    h.finish()
                };
                self.curtain_types
                    .v3d
                    .preview(ui, self.gpu.as_ref(), key, || Some(sample), size);
            }
            _ => {
                let (rect, _) = ui.allocate_exact_size(size, Sense::hover());
                let painter = ui.painter_at(rect);
                paper(&painter, rect, p.line);
                match view {
                    CurtainView::Elevation => {
                        let Some((dr, xs, ys)) = crate::type_drawing::curtain_elevation(&sample)
                        else {
                            return;
                        };
                        // Room for the sizes above and on the left.
                        let inner = Rect::from_min_max(
                            rect.min + Vec2::new(54.0, 28.0),
                            rect.max - Vec2::new(14.0, 14.0),
                        );
                        let area = paint_drawing(&painter, inner, &dr, 400.0, 1.0);
                        let k = area.width() / dr.width() as f32;
                        let sx = |x: f64| area.left() + (x - dr.lo[0]) as f32 * k;
                        let sy = |y: f64| area.bottom() - (y - dr.lo[1]) as f32 * k;
                        let ink = Color32::from_rgb(30, 30, 30);
                        let mut xs = xs;
                        xs.sort_by(f64::total_cmp);
                        let font = FontId::proportional(11.0);
                        for w in xs.windows(2) {
                            let (a, b) = (sx(w[0]), sx(w[1]));
                            let y = area.top() - 6.0;
                            painter.line_segment([Pos2::new(a, y), Pos2::new(b, y)], (0.8, ink));
                            for x in [a, b] {
                                painter.line_segment(
                                    [Pos2::new(x, y - 3.0), Pos2::new(x, y + 3.0)],
                                    (0.8, ink),
                                );
                            }
                            if b - a > 26.0 {
                                painter.text(
                                    Pos2::new((a + b) / 2.0, y - 2.0),
                                    Align2::CENTER_BOTTOM,
                                    fmt_mm(w[1] - w[0]),
                                    font.clone(),
                                    ink,
                                );
                            }
                        }
                        for w in ys.windows(2) {
                            let (a, b) = (sy(w[0]), sy(w[1]));
                            let x = area.left() - 6.0;
                            painter.line_segment([Pos2::new(x, a), Pos2::new(x, b)], (0.8, ink));
                            for y in [a, b] {
                                painter.line_segment(
                                    [Pos2::new(x - 3.0, y), Pos2::new(x + 3.0, y)],
                                    (0.8, ink),
                                );
                            }
                            if a - b > 14.0 {
                                painter.text(
                                    Pos2::new(x - 3.0, (a + b) / 2.0),
                                    Align2::RIGHT_CENTER,
                                    fmt_mm(w[1] - w[0]),
                                    font.clone(),
                                    ink,
                                );
                            }
                        }
                    }
                    CurtainView::Plan => {
                        if let Some(dr) = crate::type_drawing::curtain_plan_detail(&sample) {
                            paint_drawing(&painter, rect.shrink(16.0), &dr, 1800.0, 1.0);
                            side_labels(&painter, rect, dr.sides, Color32::from_gray(90));
                        }
                    }
                    _ => {
                        if let Some(dr) = crate::type_drawing::curtain_section(&sample) {
                            paint_drawing(&painter, rect.shrink(16.0), &dr, 1200.0, 1.0);
                            side_labels(&painter, rect, dr.sides, Color32::from_gray(90));
                        }
                    }
                }
            }
        }
        ui.add_space(4.0);
        ui.label(
            egui::RichText::new(format!(
                "{} paños × {} filas en la muestra · {}",
                g.mullions.len() - 1,
                g.transoms.len() - 1,
                super::curtain_ui::grid_note(&design)
            ))
            .small()
            .color(p.muted),
        );
    }
}
