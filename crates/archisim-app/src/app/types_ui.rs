//! The «Tipos y materiales» dialog: ArchiCAD's building materials and
//! composites settings and Revit's "Edit Type", in one window. Materials,
//! wall, slab, door and window types are created by duplicating an
//! existing one, changed in place and deleted when nothing uses them.
//! Every change is one undoable step and applies at once to every element
//! of the type, so the plan and the 3D view follow while editing.

use archisim_core::{
    CutPattern, Layer, LayerFunction, MaterialId, Model, OpeningType, OpeningTypeId, SlabFunction,
    SlabTypeId, WallFunction, WallTypeId,
};
use eframe::egui::{
    self, Color32, ComboBox, Id, RichText, TextureHandle, TextureOptions, Ui, Vec2,
};

use super::{
    default_opening_settings, default_slab_settings, default_wall_settings, guid_label,
    length_field, text_edit_committed, ArchiSimApp,
};
use crate::type_preview::{draw, layers_model, opening_model, PREVIEW_SCALE};
use crate::units::fmt_mm;

/// The lists of the dialog, one per tab.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TypesTab {
    #[default]
    Walls,
    Slabs,
    Doors,
    Windows,
    Materials,
}

/// State of the dialog: which tab and which item of each list is shown.
#[derive(Default)]
pub struct TypesDialog {
    pub open: bool,
    pub tab: TypesTab,
    pub material: Option<MaterialId>,
    pub wall: Option<WallTypeId>,
    pub slab: Option<SlabTypeId>,
    pub door: Option<OpeningTypeId>,
    pub window: Option<OpeningTypeId>,
    /// Colour being picked, applied when the colour picker closes so that
    /// dragging across the picker is one undo step.
    color_draft: Option<(MaterialId, [u8; 3])>,
    /// The last preview drawn and what it shows.
    pub(super) preview: Option<(String, TextureHandle, Vec2)>,
}

/// What the dialog was asked to do this frame, applied after drawing it.
pub(super) enum Action {
    Select(TypesTab, u64),
    DuplicateMaterial(MaterialId),
    RemoveMaterial(MaterialId),
    EditMaterial(MaterialId, String, String, [u8; 3], CutPattern),
    DuplicateWall(WallTypeId),
    RemoveWall(WallTypeId),
    EditWall(WallTypeId, String, WallFunction, Vec<Layer>),
    DuplicateSlab(SlabTypeId),
    RemoveSlab(SlabTypeId),
    EditSlab(SlabTypeId, String, SlabFunction, Vec<Layer>),
    DuplicateOpening(OpeningTypeId),
    RemoveOpening(OpeningTypeId),
    /// One parameter of the type's family.
    EditOpeningValue(OpeningTypeId, String, archisim_core::Value),
    /// Name, type mark and description.
    EditOpeningInfo(OpeningTypeId, String, String, String),
    /// A new type of a family (by name): doors when true.
    NewOpening(String, bool),
}

/// IFC material categories offered (`IfcMaterial.Category`), in Spanish.
/// Glass is drawn see-through in 3D and in IFC viewers.
pub const CATEGORIES: &[(&str, &str)] = &[
    ("concrete", "Hormigón"),
    ("brick", "Fábrica de ladrillo"),
    ("block", "Bloque"),
    ("stone", "Piedra"),
    ("ceramic", "Cerámica (baldosa, teja)"),
    ("plaster", "Yeso y revocos"),
    ("mortar", "Mortero"),
    ("gypsum", "Placa de yeso laminado"),
    ("insulation", "Aislamiento"),
    ("membrane", "Lámina"),
    ("gravel", "Grava y áridos"),
    ("earth", "Terreno"),
    ("wood", "Madera"),
    ("steel", "Acero"),
    ("aluminium", "Aluminio"),
    ("glass", "Vidrio"),
    ("plastic", "Plástico"),
];

fn category_name(code: &str) -> String {
    CATEGORIES
        .iter()
        .find(|(c, _)| *c == code)
        .map_or_else(|| format!("Otra ({code})"), |(_, n)| (*n).to_string())
}

pub fn pattern_name(p: CutPattern) -> &'static str {
    match p {
        CutPattern::Solid => "Relleno liso",
        CutPattern::Diagonal => "Rayado diagonal (fábricas)",
        CutPattern::Cross => "Rayado cruzado (hormigón)",
        CutPattern::Insulation => "Zigzag (aislamiento)",
        CutPattern::Dots => "Punteado (morteros, revocos)",
        CutPattern::Empty => "Vacío (cámara de aire)",
    }
}

const PATTERNS: [CutPattern; 6] = [
    CutPattern::Solid,
    CutPattern::Diagonal,
    CutPattern::Cross,
    CutPattern::Insulation,
    CutPattern::Dots,
    CutPattern::Empty,
];

pub fn layer_function_name(f: LayerFunction) -> &'static str {
    match f {
        LayerFunction::Structure => "Estructura",
        LayerFunction::Substrate => "Soporte",
        LayerFunction::Insulation => "Aislamiento",
        LayerFunction::AirGap => "Cámara de aire",
        LayerFunction::Membrane => "Lámina",
        LayerFunction::Finish => "Acabado",
    }
}

const LAYER_FUNCTIONS: [LayerFunction; 6] = [
    LayerFunction::Structure,
    LayerFunction::Substrate,
    LayerFunction::Insulation,
    LayerFunction::AirGap,
    LayerFunction::Membrane,
    LayerFunction::Finish,
];

fn slab_function_name(f: SlabFunction) -> &'static str {
    match f {
        SlabFunction::Floor => "Forjado de planta",
        SlabFunction::BaseSlab => "Solera",
        SlabFunction::Roof => "Cubierta plana",
    }
}

fn wall_function_name(f: WallFunction) -> &'static str {
    match f {
        WallFunction::Exterior => "Exterior (fachada)",
        WallFunction::Interior => "Interior",
    }
}

fn swatch(ui: &mut Ui, rgb: [u8; 3]) {
    let (r, _) = ui.allocate_exact_size(Vec2::splat(12.0), egui::Sense::hover());
    let [red, g, b] = rgb;
    ui.painter()
        .rect_filled(r, egui::CornerRadius::ZERO, Color32::from_rgb(red, g, b));
    ui.painter().rect_stroke(
        r,
        egui::CornerRadius::ZERO,
        egui::Stroke::new(1.0, Color32::GRAY),
        egui::StrokeKind::Inside,
    );
}

/// A combo of the project's materials, by name.
fn material_combo(ui: &mut Ui, id: Id, model: &Model, value: &mut MaterialId, width: f32) {
    let mut mats: Vec<(MaterialId, String, [u8; 3])> = model
        .materials()
        .map(|m| (m.id, m.name.clone(), m.cut_color))
        .collect();
    mats.sort_by_key(|a| a.1.to_lowercase());
    let shown = model
        .material(*value)
        .map_or("—".to_string(), |m| m.name.clone());
    ComboBox::from_id_salt(id)
        .width(width)
        .truncate()
        .selected_text(shown)
        .show_ui(ui, |ui| {
            for (mid, name, rgb) in &mats {
                ui.horizontal(|ui| {
                    swatch(ui, *rgb);
                    ui.selectable_value(value, *mid, name);
                });
            }
        });
}

impl ArchiSimApp {
    /// Opens the dialog on a tab, showing one item of it (Revit's "Edit
    /// Type" from the properties of an element).
    pub fn edit_type(&mut self, tab: TypesTab, id: u64) {
        self.types.open = true;
        self.select_type(tab, id);
    }

    fn select_type(&mut self, tab: TypesTab, id: u64) {
        let d = &mut self.types;
        d.tab = tab;
        match tab {
            TypesTab::Materials => d.material = Some(MaterialId(id)),
            TypesTab::Walls => d.wall = Some(WallTypeId(id)),
            TypesTab::Slabs => d.slab = Some(SlabTypeId(id)),
            TypesTab::Doors => d.door = Some(OpeningTypeId(id)),
            TypesTab::Windows => d.window = Some(OpeningTypeId(id)),
        }
    }

    /// Keeps the tools pointing at types that exist after a type is
    /// deleted, or a duplicate is undone.
    pub(super) fn keep_tool_types_valid(&mut self) {
        let m = self.ed.model();
        if m.wall_type(self.wall_settings.wall_type).is_none() {
            let d = default_wall_settings(m);
            self.wall_settings.wall_type = d.wall_type;
            self.wall_settings.reference = d.reference;
        }
        if m.slab_type(self.slab_settings.slab_type).is_none() {
            self.slab_settings.slab_type = default_slab_settings(m).slab_type;
        }
        if m.opening_type(self.door_settings.opening_type).is_none() {
            self.door_settings.opening_type = default_opening_settings(m, true).opening_type;
        }
        if m.opening_type(self.window_settings.opening_type).is_none() {
            self.window_settings.opening_type = default_opening_settings(m, false).opening_type;
        }
    }

    /// The item shown in each list: the chosen one if it still exists,
    /// otherwise the first of the list.
    fn fix_type_selection(&mut self) {
        let m = self.ed.model();
        let d = &mut self.types;
        if d.material.and_then(|id| m.material(id)).is_none() {
            d.material = sorted_materials(m).first().map(|x| x.0);
        }
        if d.wall.and_then(|id| m.wall_type(id)).is_none() {
            d.wall = m.wall_types().next().map(|t| t.id);
        }
        if d.slab.and_then(|id| m.slab_type(id)).is_none() {
            d.slab = m.slab_types().next().map(|t| t.id);
        }
        let first = |door: bool| {
            m.opening_types()
                .find(|t| t.is_door() == door)
                .map(|t| t.id)
        };
        if !d
            .door
            .and_then(|id| m.opening_type(id))
            .is_some_and(|t| t.is_door())
        {
            d.door = first(true);
        }
        if !d
            .window
            .and_then(|id| m.opening_type(id))
            .is_some_and(|t| t.is_window())
        {
            d.window = first(false);
        }
    }

    pub(super) fn types_dialog(&mut self, ctx: &egui::Context) {
        if !self.types.open {
            return;
        }
        self.fix_type_selection();
        self.commit_color_draft(ctx);
        let mut open = true;
        let mut actions = Vec::new();
        egui::Window::new("Tipos y materiales")
            .id(Id::unique("types-dialog"))
            .open(&mut open)
            .resizable(true)
            .default_size([900.0, 600.0])
            .show(ctx, |ui| self.types_body(ui, &mut actions));
        self.types.open = open;
        for a in actions {
            self.apply_type_action(a);
        }
    }

    /// Applies the colour picked once the picker is closed.
    fn commit_color_draft(&mut self, ctx: &egui::Context) {
        let Some((id, rgb)) = self.types.color_draft else {
            return;
        };
        if egui::Popup::is_any_open(ctx) {
            return;
        }
        self.types.color_draft = None;
        if let Some(m) = self.ed.model().material(id).cloned() {
            if m.cut_color != rgb {
                self.apply_type_action(Action::EditMaterial(
                    id,
                    m.name,
                    m.category,
                    rgb,
                    m.cut_pattern,
                ));
            }
        }
    }

    fn types_body(&mut self, ui: &mut Ui, actions: &mut Vec<Action>) {
        ui.horizontal(|ui| {
            for (tab, name) in [
                (TypesTab::Walls, "Muros"),
                (TypesTab::Slabs, "Forjados"),
                (TypesTab::Doors, "Puertas"),
                (TypesTab::Windows, "Ventanas"),
                (TypesTab::Materials, "Materiales"),
            ] {
                ui.selectable_value(&mut self.types.tab, tab, name);
            }
        });
        ui.label(
            RichText::new(
                "Los cambios se aplican al momento a todos los elementos del tipo. Ctrl+Z los deshace uno a uno.",
            )
            .weak(),
        );
        ui.separator();
        let height = ui.available_height().max(300.0);
        ui.horizontal_top(|ui| {
            ui.vertical(|ui| {
                ui.set_width(270.0);
                ui.set_min_height(height);
                self.type_list(ui, actions);
            });
            ui.separator();
            ui.vertical(|ui| {
                egui::ScrollArea::vertical()
                    .id_salt("type-editor")
                    .auto_shrink([false, false])
                    .show(ui, |ui| self.type_editor(ui, actions));
            });
        });
    }

    // ---- Lists ----------------------------------------------------------------

    fn type_list(&mut self, ui: &mut Ui, actions: &mut Vec<Action>) {
        let m = self.ed.model();
        let tab = self.types.tab;
        // (id, name, detail)
        let rows: Vec<(u64, String, String)> = match tab {
            TypesTab::Materials => sorted_materials(m)
                .into_iter()
                .map(|(id, name)| {
                    let n = m.material_users(id).len();
                    let detail = if n == 0 {
                        "sin usar".to_string()
                    } else {
                        format!("en {n} tipo{}", if n == 1 { "" } else { "s" })
                    };
                    (id.0, name, detail)
                })
                .collect(),
            TypesTab::Walls => m
                .wall_types()
                .map(|t| {
                    let n = m.wall_type_users(t.id);
                    (
                        t.id.0,
                        t.name.clone(),
                        format!("{} mm · {n} muro{}", fmt_mm(t.thickness()), plural(n)),
                    )
                })
                .collect(),
            TypesTab::Slabs => m
                .slab_types()
                .map(|t| {
                    let n = m.slab_type_users(t.id);
                    (
                        t.id.0,
                        t.name.clone(),
                        format!("{} mm · {n} forjado{}", fmt_mm(t.thickness()), plural(n)),
                    )
                })
                .collect(),
            TypesTab::Doors | TypesTab::Windows => m
                .opening_types()
                .filter(|t| t.is_door() == (tab == TypesTab::Doors))
                .map(|t| {
                    let n = m.opening_type_users(t.id);
                    let mark = if t.mark.is_empty() {
                        String::new()
                    } else {
                        format!("{} · ", t.mark)
                    };
                    (
                        t.id.0,
                        t.name.clone(),
                        format!(
                            "{mark}{} × {} · {n} colocada{}",
                            fmt_mm(t.width),
                            fmt_mm(t.height),
                            plural(n)
                        ),
                    )
                })
                .collect(),
        };
        let selected = self.selected_id(tab);
        egui::ScrollArea::vertical()
            .id_salt(("type-list", tab as u8))
            .max_height(ui.available_height() - 40.0)
            .auto_shrink([false, false])
            .show(ui, |ui| {
                for (id, name, detail) in &rows {
                    let r = ui.selectable_label(
                        Some(*id) == selected,
                        RichText::new(name).strong().size(13.0),
                    );
                    if r.clicked() {
                        actions.push(Action::Select(tab, *id));
                    }
                    ui.label(RichText::new(detail).weak().small());
                    ui.add_space(2.0);
                }
            });
        ui.add_space(6.0);
        let doors = tab == TypesTab::Doors;
        let families = match tab {
            TypesTab::Doors | TypesTab::Windows => self.ed.opening_families(doors),
            _ => Vec::new(),
        };
        ui.horizontal(|ui| {
            if !families.is_empty() {
                ui.menu_button("＋ Nuevo…", |ui| {
                    ui.label(RichText::new("Nuevo tipo de la familia:").weak());
                    for f in &families {
                        if ui.button(f).clicked() {
                            actions.push(Action::NewOpening(f.clone(), doors));
                            ui.close();
                        }
                    }
                })
                .response
                .on_hover_text("Un tipo nuevo de una familia, con sus valores por defecto");
            }
            let Some(id) = selected else { return };
            if ui
                .button("⧉ Duplicar")
                .on_hover_text(
                    "Crea uno nuevo a partir del seleccionado (como «Duplicar» en Revit)",
                )
                .clicked()
            {
                actions.push(match tab {
                    TypesTab::Materials => Action::DuplicateMaterial(MaterialId(id)),
                    TypesTab::Walls => Action::DuplicateWall(WallTypeId(id)),
                    TypesTab::Slabs => Action::DuplicateSlab(SlabTypeId(id)),
                    TypesTab::Doors | TypesTab::Windows => {
                        Action::DuplicateOpening(OpeningTypeId(id))
                    }
                });
            }
            if ui
                .button("🗑 Borrar")
                .on_hover_text("Solo si ningún elemento lo usa")
                .clicked()
            {
                actions.push(match tab {
                    TypesTab::Materials => Action::RemoveMaterial(MaterialId(id)),
                    TypesTab::Walls => Action::RemoveWall(WallTypeId(id)),
                    TypesTab::Slabs => Action::RemoveSlab(SlabTypeId(id)),
                    TypesTab::Doors | TypesTab::Windows => Action::RemoveOpening(OpeningTypeId(id)),
                });
            }
        });
    }

    fn selected_id(&self, tab: TypesTab) -> Option<u64> {
        let d = &self.types;
        match tab {
            TypesTab::Materials => d.material.map(|x| x.0),
            TypesTab::Walls => d.wall.map(|x| x.0),
            TypesTab::Slabs => d.slab.map(|x| x.0),
            TypesTab::Doors => d.door.map(|x| x.0),
            TypesTab::Windows => d.window.map(|x| x.0),
        }
    }

    // ---- Editors --------------------------------------------------------------

    fn type_editor(&mut self, ui: &mut Ui, actions: &mut Vec<Action>) {
        // Why the last change was refused, where it is seen.
        if let Some(msg) = self.message.as_ref().filter(|m| m.error) {
            ui.colored_label(ui.visuals().error_fg_color, format!("⚠ {}", msg.text));
            ui.add_space(4.0);
        }
        match self.types.tab {
            TypesTab::Materials => {
                if let Some(id) = self.types.material {
                    self.material_editor(ui, id, actions);
                }
            }
            TypesTab::Walls => {
                if let Some(id) = self.types.wall {
                    self.wall_type_editor(ui, id, actions);
                }
            }
            TypesTab::Slabs => {
                if let Some(id) = self.types.slab {
                    self.slab_type_editor(ui, id, actions);
                }
            }
            TypesTab::Doors => {
                if let Some(id) = self.types.door {
                    self.opening_type_editor(ui, id, actions);
                }
            }
            TypesTab::Windows => {
                if let Some(id) = self.types.window {
                    self.opening_type_editor(ui, id, actions);
                }
            }
        }
    }

    fn material_editor(&mut self, ui: &mut Ui, id: MaterialId, actions: &mut Vec<Action>) {
        let m = self.ed.model();
        let Some(mat) = m.material(id).cloned() else {
            return;
        };
        let users = m.material_users(id);
        let shown_color = match self.types.color_draft {
            Some((d, rgb)) if d == id => rgb,
            _ => mat.cut_color,
        };
        ui.label(RichText::new("Material · IfcMaterial").strong());
        ui.add_space(4.0);
        egui::Grid::new(("material-grid", id.0))
            .num_columns(2)
            .spacing([12.0, 6.0])
            .show(ui, |ui| {
                ui.label("Nombre");
                if let Some(new) = text_edit_committed(ui, Id::unique(("mat-name", id.0)), &mat.name, 260.0)
                {
                    actions.push(Action::EditMaterial(
                        id,
                        new,
                        mat.category.clone(),
                        mat.cut_color,
                        mat.cut_pattern,
                    ));
                }
                ui.end_row();

                ui.label("Categoría (IFC)")
                    .on_hover_text("IfcMaterial.Category. Los vidrios se ven transparentes en 3D y en los visores IFC.");
                let mut cat = mat.category.clone();
                ComboBox::from_id_salt(("mat-cat", id.0))
                    .width(260.0)
                    .selected_text(category_name(&cat))
                    .show_ui(ui, |ui| {
                        for (code, name) in CATEGORIES {
                            ui.selectable_value(&mut cat, (*code).to_string(), *name);
                        }
                    });
                if cat != mat.category {
                    actions.push(Action::EditMaterial(
                        id,
                        mat.name.clone(),
                        cat,
                        mat.cut_color,
                        mat.cut_pattern,
                    ));
                }
                ui.end_row();

                ui.label("Color de corte");
                ui.horizontal(|ui| {
                    let mut rgb = shown_color;
                    if ui.color_edit_button_srgb(&mut rgb).changed() {
                        self.types.color_draft = Some((id, rgb));
                    }
                    let text = format!("{}, {}, {}", rgb[0], rgb[1], rgb[2]);
                    if let Some(new) =
                        text_edit_committed(ui, Id::unique(("mat-rgb", id.0)), &text, 110.0)
                    {
                        match parse_rgb(&new) {
                            Some(c) => actions.push(Action::EditMaterial(
                                id,
                                mat.name.clone(),
                                mat.category.clone(),
                                c,
                                mat.cut_pattern,
                            )),
                            None => self.complain(
                                "El color se escribe como tres números de 0 a 255: rojo, verde, azul.",
                            ),
                        }
                    }
                    ui.label(RichText::new("R, G, B").weak());
                });
                ui.end_row();

                ui.label("Trama de corte");
                let mut pat = mat.cut_pattern;
                ComboBox::from_id_salt(("mat-pat", id.0))
                    .width(260.0)
                    .selected_text(pattern_name(pat))
                    .show_ui(ui, |ui| {
                        for p in PATTERNS {
                            ui.selectable_value(&mut pat, p, pattern_name(p));
                        }
                    });
                if pat != mat.cut_pattern {
                    actions.push(Action::EditMaterial(
                        id,
                        mat.name.clone(),
                        mat.category.clone(),
                        mat.cut_color,
                        pat,
                    ));
                }
                ui.end_row();

                ui.label("Se usa en");
                if users.is_empty() {
                    ui.label(RichText::new("ningún tipo (se puede borrar)").weak());
                } else {
                    ui.vertical(|ui| {
                        for u in &users {
                            ui.label(u);
                        }
                    });
                }
                ui.end_row();
            });
        // A 20 cm wall of the material alone, cut in plan.
        let mut preview = mat.clone();
        preview.cut_color = shown_color;
        let mut pm = Model::new("m");
        let pid = pm.add_material(
            preview.name,
            preview.category,
            preview.cut_color,
            preview.cut_pattern,
        );
        let layers = [Layer {
            material: pid,
            thickness: 0.2,
            function: LayerFunction::Structure,
        }];
        let (key, built) = layers_preview(&pm, &layers, 0.8);
        self.show_preview(ui, key, built, "Corte en planta, muro de 20 cm", None);
    }

    fn wall_type_editor(&mut self, ui: &mut Ui, id: WallTypeId, actions: &mut Vec<Action>) {
        let m = self.ed.model();
        let Some(t) = m.wall_type(id).cloned() else {
            return;
        };
        let users = m.wall_type_users(id);
        ui.label(RichText::new("Tipo de muro · IfcWallType").strong());
        ui.add_space(4.0);
        egui::Grid::new(("wt-grid", id.0))
            .num_columns(2)
            .spacing([12.0, 6.0])
            .show(ui, |ui| {
                ui.label("Nombre");
                if let Some(new) = text_edit_committed(ui, Id::unique(("wt-name", id.0)), &t.name, 300.0) {
                    actions.push(Action::EditWall(id, new, t.function, t.layers.clone()));
                }
                ui.end_row();
                ui.label("Función").on_hover_text(
                    "Exterior: muros de fachada (IsExternal en IFC); al dibujarlos, la línea de referencia pasa a la cara exterior",
                );
                let mut f = t.function;
                ComboBox::from_id_salt(("wt-fun", id.0))
                    .selected_text(wall_function_name(f))
                    .show_ui(ui, |ui| {
                        for v in [WallFunction::Exterior, WallFunction::Interior] {
                            ui.selectable_value(&mut f, v, wall_function_name(v));
                        }
                    });
                if f != t.function {
                    actions.push(Action::EditWall(id, t.name.clone(), f, t.layers.clone()));
                }
                ui.end_row();
                ui.label("Espesor total");
                ui.strong(format!("{} mm", fmt_mm(t.thickness())));
                ui.end_row();
                ui.label("Muros de este tipo");
                ui.label(users.to_string());
                ui.end_row();
                ui.label("GlobalId");
                guid_label(ui, t.guid.as_str());
                ui.end_row();
            });
        ui.add_space(8.0);
        let edited = Self::layers_table(
            ui,
            self.ed.model(),
            ("wt", id.0),
            &t.layers,
            "Exterior",
            "Interior",
        );
        if let Some(layers) = edited {
            actions.push(Action::EditWall(id, t.name.clone(), t.function, layers));
        }
        let (key, built) = layers_preview(self.ed.model(), &t.layers, 1.0);
        self.show_preview(
            ui,
            key,
            built,
            "Corte en planta (exterior arriba)",
            Some(("Exterior", "Interior")),
        );
    }

    fn slab_type_editor(&mut self, ui: &mut Ui, id: SlabTypeId, actions: &mut Vec<Action>) {
        let m = self.ed.model();
        let Some(t) = m.slab_type(id).cloned() else {
            return;
        };
        let users = m.slab_type_users(id);
        ui.label(RichText::new("Tipo de forjado · IfcSlabType").strong());
        ui.add_space(4.0);
        egui::Grid::new(("st-grid", id.0))
            .num_columns(2)
            .spacing([12.0, 6.0])
            .show(ui, |ui| {
                ui.label("Nombre");
                if let Some(new) = text_edit_committed(ui, Id::unique(("st-name", id.0)), &t.name, 300.0) {
                    actions.push(Action::EditSlab(id, new, t.function, t.layers.clone()));
                }
                ui.end_row();
                ui.label("Función").on_hover_text(
                    "Forjado de planta (FLOOR), solera (BASESLAB) o cubierta plana (ROOF); solera y cubierta son exteriores",
                );
                let mut f = t.function;
                ComboBox::from_id_salt(("st-fun", id.0))
                    .selected_text(slab_function_name(f))
                    .show_ui(ui, |ui| {
                        for v in [SlabFunction::Floor, SlabFunction::BaseSlab, SlabFunction::Roof] {
                            ui.selectable_value(&mut f, v, slab_function_name(v));
                        }
                    });
                if f != t.function {
                    actions.push(Action::EditSlab(id, t.name.clone(), f, t.layers.clone()));
                }
                ui.end_row();
                ui.label("Canto total");
                ui.strong(format!("{} mm", fmt_mm(t.thickness())));
                ui.end_row();
                ui.label("Forjados de este tipo");
                ui.label(users.to_string());
                ui.end_row();
                ui.label("GlobalId");
                guid_label(ui, t.guid.as_str());
                ui.end_row();
            });
        ui.add_space(8.0);
        let edited = Self::layers_table(
            ui,
            self.ed.model(),
            ("st", id.0),
            &t.layers,
            "Cara superior",
            "Cara inferior",
        );
        if let Some(layers) = edited {
            actions.push(Action::EditSlab(id, t.name.clone(), t.function, layers));
        }
        let (key, built) = layers_preview(self.ed.model(), &t.layers, 1.0);
        self.show_preview(
            ui,
            key,
            built,
            "Capas en sección (cara superior arriba)",
            Some(("Cara superior", "Cara inferior")),
        );
    }

    /// The layers of a wall or slab type, editable: material, thickness and
    /// function of each, order, add and remove. Returns the new layers when
    /// something changed this frame.
    fn layers_table(
        ui: &mut Ui,
        model: &Model,
        key: (&str, u64),
        layers: &[Layer],
        first_side: &str,
        last_side: &str,
    ) -> Option<Vec<Layer>> {
        let mut out: Option<Vec<Layer>> = None;
        let n = layers.len();
        ui.label(
            RichText::new(format!(
                "Capas, de {} a {}",
                first_side.to_lowercase(),
                last_side.to_lowercase()
            ))
            .strong(),
        );
        ui.label(RichText::new(format!("▲ {first_side}")).weak().small());
        egui::Grid::new(("layers", key.0, key.1))
            .num_columns(6)
            .spacing([8.0, 4.0])
            .striped(true)
            .show(ui, |ui| {
                ui.label("");
                ui.strong("Material");
                ui.strong("Espesor (mm)");
                ui.strong("Función");
                ui.label("");
                ui.end_row();
                for (i, l) in layers.iter().enumerate() {
                    ui.label(format!("{}", i + 1));
                    let mut mat = l.material;
                    ui.horizontal(|ui| {
                        swatch(ui, model.material(mat).map_or([200; 3], |m| m.cut_color));
                        material_combo(
                            ui,
                            Id::unique(("layer-mat", key.0, key.1, i)),
                            model,
                            &mut mat,
                            200.0,
                        );
                    });
                    if mat != l.material {
                        let mut v = layers.to_vec();
                        v[i].material = mat;
                        out = Some(v);
                    }
                    if let Some(th) = length_field(
                        ui,
                        Id::unique(("layer-th", key.0, key.1, i)),
                        l.thickness,
                        70.0,
                    ) {
                        let mut v = layers.to_vec();
                        v[i].thickness = th;
                        out = Some(v);
                    }
                    let mut f = l.function;
                    ComboBox::from_id_salt(("layer-fun", key.0, key.1, i))
                        .width(120.0)
                        .selected_text(layer_function_name(f))
                        .show_ui(ui, |ui| {
                            for v in LAYER_FUNCTIONS {
                                ui.selectable_value(&mut f, v, layer_function_name(v));
                            }
                        });
                    if f != l.function {
                        let mut v = layers.to_vec();
                        v[i].function = f;
                        out = Some(v);
                    }
                    ui.horizontal(|ui| {
                        if ui
                            .add_enabled(i > 0, egui::Button::new("▲").small())
                            .on_hover_text(format!("Hacia {}", first_side.to_lowercase()))
                            .clicked()
                        {
                            let mut v = layers.to_vec();
                            v.swap(i, i - 1);
                            out = Some(v);
                        }
                        if ui
                            .add_enabled(i + 1 < n, egui::Button::new("▼").small())
                            .on_hover_text(format!("Hacia {}", last_side.to_lowercase()))
                            .clicked()
                        {
                            let mut v = layers.to_vec();
                            v.swap(i, i + 1);
                            out = Some(v);
                        }
                        if ui
                            .add(egui::Button::new("⧉").small())
                            .on_hover_text("Duplicar la capa")
                            .clicked()
                        {
                            let mut v = layers.to_vec();
                            v.insert(i + 1, l.clone());
                            out = Some(v);
                        }
                        if ui
                            .add_enabled(n > 1, egui::Button::new("✕").small())
                            .on_hover_text("Quitar la capa")
                            .on_disabled_hover_text("Un tipo necesita al menos una capa")
                            .clicked()
                        {
                            let mut v = layers.to_vec();
                            v.remove(i);
                            out = Some(v);
                        }
                    });
                    ui.end_row();
                }
            });
        ui.label(RichText::new(format!("▼ {last_side}")).weak().small());
        out
    }

    fn opening_type_editor(&mut self, ui: &mut Ui, id: OpeningTypeId, actions: &mut Vec<Action>) {
        let m = self.ed.model();
        let Some(t) = m.opening_type(id).cloned() else {
            return;
        };
        let Some(family) = m.family(t.family).cloned() else {
            return;
        };
        let users = m.opening_type_users(id);
        let door = t.is_door();
        ui.label(
            RichText::new(if door {
                "Tipo de puerta · IfcDoorType"
            } else {
                "Tipo de ventana · IfcWindowType"
            })
            .strong(),
        );
        ui.add_space(4.0);
        let mut info: Option<(String, String, String)> = None;
        egui::Grid::new(("ot-grid", id.0))
            .num_columns(2)
            .min_col_width(OPENING_LABELS)
            .spacing([12.0, 6.0])
            .show(ui, |ui| {
                ui.label("Nombre");
                if let Some(new) =
                    text_edit_committed(ui, Id::unique(("ot-name", id.0)), &t.name, 300.0)
                {
                    info = Some((new, t.mark.clone(), t.description.clone()));
                }
                ui.end_row();
                ui.label("Marca de tipo")
                    .on_hover_text("Para etiquetas, tablas y mediciones (P-01, V-03…); única en el proyecto");
                if let Some(new) =
                    text_edit_committed(ui, Id::unique(("ot-mark", id.0)), &t.mark, 80.0)
                {
                    info = Some((t.name.clone(), new, t.description.clone()));
                }
                ui.end_row();
                ui.label("Descripción");
                if let Some(new) =
                    text_edit_committed(ui, Id::unique(("ot-desc", id.0)), &t.description, 300.0)
                {
                    info = Some((t.name.clone(), t.mark.clone(), new));
                }
                ui.end_row();
                ui.label("Familia");
                ui.label(family_label(m, t.family)).on_hover_text(
                    "La receta de la que sale este tipo; abajo, sus parámetros. Una familia no se convierte en otra: para otro funcionamiento, crea un tipo nuevo de esa familia",
                );
                ui.end_row();
            });
        if let Some((name, mark, description)) = info {
            actions.push(Action::EditOpeningInfo(id, name, mark, description));
        }

        // The family's type parameters, by section, as the recipe declares
        // them; those that do not apply (a fourth leaf of a window with
        // two) are hidden.
        let def = family.def();
        let mut section: Option<&str> = None;
        let mut grid_rows: Vec<&archisim_core::family::Param> = Vec::new();
        let flush = |ui: &mut Ui,
                     title: Option<&str>,
                     rows: &mut Vec<&archisim_core::family::Param>,
                     actions: &mut Vec<Action>| {
            if rows.is_empty() {
                return;
            }
            ui.add_space(6.0);
            ui.label(RichText::new(title.unwrap_or("Parámetros")).strong());
            egui::Grid::new(("ot-params", id.0, title.unwrap_or("")))
                .num_columns(2)
                .min_col_width(OPENING_LABELS)
                .spacing([12.0, 6.0])
                .show(ui, |ui| {
                    for p in rows.drain(..) {
                        param_row(ui, m, id, &t, p, actions);
                        ui.end_row();
                    }
                });
        };
        for p in def.params_of(archisim_core::ParamScope::Type) {
            if !def.is_visible(&p.name, &t.values) {
                continue;
            }
            if p.section.as_deref() != section {
                flush(ui, section, &mut grid_rows, actions);
                section = p.section.as_deref();
            }
            grid_rows.push(p);
        }
        flush(ui, section, &mut grid_rows, actions);

        ui.add_space(6.0);
        egui::Grid::new(("ot-tail", id.0))
            .num_columns(2)
            .min_col_width(OPENING_LABELS)
            .spacing([12.0, 6.0])
            .show(ui, |ui| {
                ui.label(if door {
                    "Puertas de este tipo"
                } else {
                    "Ventanas de este tipo"
                });
                ui.label(users.to_string());
                ui.end_row();
                ui.label("GlobalId");
                guid_label(ui, t.guid.as_str());
                ui.end_row();
            });
        ui.add_space(4.0);
        ui.label(
            RichText::new(
                "Al cambiar las medidas, cada hueco colocado conserva su centro y su antepecho; si alguno dejara de caber en su muro, el cambio no se hace.",
            )
            .weak(),
        );
        let key = format!(
            "ot:{:?}:{:?}:{:?}",
            t,
            preview_materials(m, &[t.frame_material, t.panel_material]),
            m.wall_types().map(|w| &w.layers).collect::<Vec<_>>()
        );
        let built = opening_model(m, &t);
        self.show_preview(ui, key, built, "Planta (cara interior abajo)", None);
    }

    // ---- Previews ---------------------------------------------------------------

    /// Draws a preview model, redrawing its picture only when what it shows
    /// changed.
    fn show_preview(
        &mut self,
        ui: &mut Ui,
        key: String,
        built: Option<(Model, archisim_core::LevelId)>,
        caption: &str,
        sides: Option<(&str, &str)>,
    ) {
        ui.add_space(10.0);
        ui.label(RichText::new(format!("{caption} · 1:{PREVIEW_SCALE:.0}")).strong());
        let Some((model, level)) = built else {
            ui.label(RichText::new("Sin vista previa").weak());
            return;
        };
        let ppp = ui.ctx().pixels_per_point();
        // Shown about 440 points wide (a magnified 1:20 detail); line
        // weights and hatches are those of 1:20.
        let length = model.walls().next().map_or(1.0, |w| w.length()) + 0.04;
        let points_per_m = (ui.available_width().min(440.0) as f64 / length).max(50.0);
        let stale = self
            .types
            .preview
            .as_ref()
            .is_none_or(|(k, _, _)| *k != key);
        if stale {
            let (img, lo, hi) = draw(&model, level, points_per_m * ppp as f64);
            let size = Vec2::new(
                ((hi.x - lo.x) * points_per_m) as f32,
                ((hi.y - lo.y) * points_per_m) as f32,
            );
            let tex = ui
                .ctx()
                .load_texture("type-preview", img, TextureOptions::LINEAR);
            self.types.preview = Some((key, tex, size));
        }
        let Some((_, tex, size)) = &self.types.preview else {
            return;
        };
        let size = if size.x > ui.available_width() {
            *size * (ui.available_width() / size.x)
        } else {
            *size
        };
        if let Some((top, _)) = sides {
            ui.label(RichText::new(format!("▲ {top}")).weak().small());
        }
        let (rect, _) = ui.allocate_exact_size(size, egui::Sense::hover());
        ui.painter()
            .rect_filled(rect, egui::CornerRadius::same(2), Color32::WHITE);
        ui.painter().image(
            tex.id(),
            rect,
            egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
            Color32::WHITE,
        );
        if let Some((_, bottom)) = sides {
            ui.label(RichText::new(format!("▼ {bottom}")).weak().small());
        }
    }

    // ---- Applying -----------------------------------------------------------------

    pub(super) fn apply_type_action(&mut self, a: Action) {
        match a {
            Action::Select(tab, id) => self.select_type(tab, id),
            Action::DuplicateMaterial(id) => match self.ed.duplicate_material(id) {
                Ok(new) => {
                    self.types.material = Some(new);
                    self.say("Material duplicado: cámbiale el nombre y lo que quieras.");
                }
                Err(e) => self.complain(format!("No se puede: {e}.")),
            },
            Action::RemoveMaterial(id) => {
                let r = self.ed.remove_material(id);
                self.report(r, "Material borrado.");
            }
            Action::EditMaterial(id, name, cat, rgb, pat) => {
                let r = self.ed.edit_material(id, &name, &cat, rgb, pat);
                self.report(r, "Material modificado.");
            }
            Action::DuplicateWall(id) => match self.ed.duplicate_wall_type(id) {
                Ok(new) => {
                    self.types.wall = Some(new);
                    self.say("Tipo de muro duplicado: cámbiale el nombre y sus capas.");
                }
                Err(e) => self.complain(format!("No se puede: {e}.")),
            },
            Action::RemoveWall(id) => {
                let r = self.ed.remove_wall_type(id);
                self.report(r, "Tipo de muro borrado.");
            }
            Action::EditWall(id, name, f, layers) => {
                let n = self.ed.model().wall_type_users(id);
                let r = self.ed.edit_wall_type(id, &name, f, layers);
                self.report(
                    r,
                    &format!("Tipo de muro modificado ({n} muro{}).", plural(n)),
                );
            }
            Action::DuplicateSlab(id) => match self.ed.duplicate_slab_type(id) {
                Ok(new) => {
                    self.types.slab = Some(new);
                    self.say("Tipo de forjado duplicado: cámbiale el nombre y sus capas.");
                }
                Err(e) => self.complain(format!("No se puede: {e}.")),
            },
            Action::RemoveSlab(id) => {
                let r = self.ed.remove_slab_type(id);
                self.report(r, "Tipo de forjado borrado.");
            }
            Action::EditSlab(id, name, f, layers) => {
                let n = self.ed.model().slab_type_users(id);
                let r = self.ed.edit_slab_type(id, &name, f, layers);
                self.report(
                    r,
                    &format!("Tipo de forjado modificado ({n} forjado{}).", plural(n)),
                );
            }
            Action::DuplicateOpening(id) => match self.ed.duplicate_opening_type(id) {
                Ok(new) => {
                    let door = self
                        .ed
                        .model()
                        .opening_type(new)
                        .is_some_and(|t| t.is_door());
                    if door {
                        self.types.door = Some(new);
                    } else {
                        self.types.window = Some(new);
                    }
                    self.say("Tipo duplicado: cámbiale el nombre y sus medidas.");
                }
                Err(e) => self.complain(format!("No se puede: {e}.")),
            },
            Action::RemoveOpening(id) => {
                let r = self.ed.remove_opening_type(id);
                self.report(r, "Tipo borrado.");
            }
            Action::EditOpeningValue(id, param, value) => {
                let n = self.ed.model().opening_type_users(id);
                let r = self.ed.set_opening_type_value(id, &param, value);
                self.report(r, &format!("Tipo modificado ({n} colocada{}).", plural(n)));
            }
            Action::EditOpeningInfo(id, name, mark, description) => {
                let r = self
                    .ed
                    .set_opening_type_info(id, &name, &mark, &description);
                self.report(r, "Tipo modificado.");
            }
            Action::NewOpening(family, door) => match self.ed.new_opening_type(&family) {
                Ok(new) => {
                    let tab = if door {
                        TypesTab::Doors
                    } else {
                        TypesTab::Windows
                    };
                    self.types.tab = tab;
                    self.select_type(tab, new.0);
                    self.say(format!(
                        "Nuevo tipo de «{family}»: configúralo a la derecha."
                    ));
                }
                Err(e) => self.complain(format!("No se pudo crear: {e}.")),
            },
        }
        self.keep_tool_types_valid();
    }
}

/// One family parameter of a door or window type: its label and the field
/// for its kind. A change becomes an [`Action::EditOpeningValue`].
fn param_row(
    ui: &mut Ui,
    m: &Model,
    id: OpeningTypeId,
    t: &OpeningType,
    p: &archisim_core::family::Param,
    actions: &mut Vec<Action>,
) {
    use archisim_core::{ParamKind, Value};
    let label = match (&p.kind, p.label.as_str()) {
        (ParamKind::Length, l) => format!("{l} (mm)"),
        (ParamKind::Angle, l) => format!("{l} (°)"),
        (_, l) => l.to_string(),
    };
    ui.label(label);
    let current = t.values.get(&p.name).cloned();
    let field = Id::unique(("ot-param", id.0, p.name.as_str()));
    let mut set = |v: Value| actions.push(Action::EditOpeningValue(id, p.name.clone(), v));
    match (&p.kind, current) {
        (ParamKind::Length, Some(Value::Number(v))) => {
            if let Some(new) = length_field(ui, field, v, 80.0) {
                set(Value::Number(new));
            }
        }
        (ParamKind::Integer, Some(Value::Number(v)))
            if p.range.is_some_and(|(lo, hi)| hi - lo <= 12.0) =>
        {
            let (lo, hi) = p.range.expect("checked");
            let mut n = v as i64;
            ComboBox::from_id_salt(field)
                .width(80.0)
                .selected_text(n.to_string())
                .show_ui(ui, |ui| {
                    for k in lo as i64..=hi as i64 {
                        ui.selectable_value(&mut n, k, k.to_string());
                    }
                });
            if n != v as i64 {
                set(Value::Number(n as f64));
            }
        }
        (ParamKind::Angle, Some(Value::Number(v))) => {
            let shown = format!("{:.1}", v.to_degrees()).replace('.', ",");
            if let Some(text) = text_edit_committed(ui, field, &shown, 80.0) {
                if let Ok(deg) = text.trim().replace(',', ".").parse::<f64>() {
                    set(Value::Number(deg.to_radians()));
                }
            }
        }
        (ParamKind::Number | ParamKind::Integer, Some(Value::Number(v))) => {
            let shown = format!("{v}").replace('.', ",");
            if let Some(text) = text_edit_committed(ui, field, &shown, 80.0) {
                if let Ok(x) = text.trim().replace(',', ".").parse::<f64>() {
                    set(Value::Number(x));
                }
            }
        }
        (ParamKind::Bool, Some(Value::Bool(b))) => {
            let mut on = b;
            if ui.checkbox(&mut on, "").changed() {
                set(Value::Bool(on));
            }
        }
        (ParamKind::Choice(options), Some(Value::Text(chosen))) => {
            let mut c = chosen.clone();
            ComboBox::from_id_salt(field)
                .width(160.0)
                .selected_text(capitalized(&c))
                .show_ui(ui, |ui| {
                    for o in options {
                        ui.selectable_value(&mut c, o.clone(), capitalized(o));
                    }
                });
            if c != chosen {
                set(Value::Text(c));
            }
        }
        (ParamKind::Material, Some(Value::Material(mid))) => {
            let mut mat = MaterialId(mid);
            material_combo(ui, field, m, &mut mat, 220.0);
            if mat.0 != mid {
                set(Value::Material(mat.0));
            }
        }
        (ParamKind::Text, Some(Value::Text(text))) => {
            if let Some(new) = text_edit_committed(ui, field, &text, 220.0) {
                set(Value::Text(new));
            }
        }
        _ => {
            ui.label(RichText::new("—").weak());
        }
    }
}

/// Width of the label column of the door and window editor, so that its
/// sections line up.
const OPENING_LABELS: f32 = 190.0;

/// "oscilobatiente" → "Oscilobatiente".
fn capitalized(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        Some(f) => f.to_uppercase().chain(c).collect(),
        None => String::new(),
    }
}

/// "Puerta abatible (v1)": the family a type comes from.
pub(super) fn family_label(m: &archisim_core::Model, id: archisim_core::FamilyId) -> String {
    m.family(id)
        .map_or("—".into(), |f| format!("{} (v{})", f.name(), f.version()))
}

fn plural(n: usize) -> &'static str {
    if n == 1 {
        ""
    } else {
        "s"
    }
}

/// The preview of a wall or slab type's layers and the key that tells when
/// it must be drawn again.
fn layers_preview(
    source: &Model,
    layers: &[Layer],
    length: f64,
) -> (String, Option<(Model, archisim_core::LevelId)>) {
    let mats: Vec<MaterialId> = layers.iter().map(|l| l.material).collect();
    let key = format!(
        "layers:{layers:?}:{:?}:{length}",
        preview_materials(source, &mats)
    );
    (key, layers_model(source, layers, length))
}

/// Materials sorted by name, case-insensitively.
fn sorted_materials(m: &Model) -> Vec<(MaterialId, String)> {
    let mut v: Vec<(MaterialId, String)> = m.materials().map(|x| (x.id, x.name.clone())).collect();
    v.sort_by_key(|a| a.1.to_lowercase());
    v
}

/// What the preview depends on from the materials: their colour and hatch.
fn preview_materials(m: &Model, ids: &[MaterialId]) -> Vec<([u8; 3], CutPattern)> {
    ids.iter()
        .filter_map(|id| m.material(*id))
        .map(|x| (x.cut_color, x.cut_pattern))
        .collect()
}

/// `232, 196, 178` (also with spaces or semicolons) → RGB.
pub fn parse_rgb(text: &str) -> Option<[u8; 3]> {
    let parts: Vec<u8> = text
        .split([',', ';', ' '])
        .filter(|s| !s.is_empty())
        .map(|s| s.trim().parse::<u8>().ok())
        .collect::<Option<Vec<u8>>>()?;
    match parts.as_slice() {
        [r, g, b] => Some([*r, *g, *b]),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn colours_are_typed_as_three_numbers() {
        assert_eq!(parse_rgb("232, 196, 178"), Some([232, 196, 178]));
        assert_eq!(parse_rgb("10;20;30"), Some([10, 20, 30]));
        assert_eq!(parse_rgb("1 2 3"), Some([1, 2, 3]));
        assert_eq!(parse_rgb("256, 0, 0"), None);
        assert_eq!(parse_rgb("1, 2"), None);
        assert_eq!(category_name("glass"), "Vidrio");
        assert_eq!(category_name("adobe"), "Otra (adobe)");
    }
}
