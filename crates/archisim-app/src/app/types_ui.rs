//! The «Tipos y materiales» dialog, laid out as ArchiCAD's object
//! settings: on the left the types in folders (doors and windows by
//! family, walls and slabs by function, materials by category), each with
//! a small elevation or section; on the right the chosen type, first its
//! preview (plan, section, elevation or 3D) and then all its options in
//! groups. Materials, wall, slab, door and window types are created from a
//! family or by duplicating one, changed in place and deleted when nothing
//! uses them. Every change is one undoable step and applies at once to
//! every element of the type, so the plan and the 3D view follow.

use std::collections::HashMap;

use archisim_core::{
    CutPattern, Layer, LayerFunction, LineStyle, MaterialId, Model, OpeningType, OpeningTypeId,
    SlabFunction, SlabTypeId, WallFunction, WallTypeId,
};
use eframe::egui::{
    self, collapsing_header::CollapsingState, Align2, Color32, ComboBox, CornerRadius, FontId, Id,
    Pos2, Rect, RichText, Sense, Stroke, StrokeKind, TextureHandle, TextureOptions, Ui, Vec2,
};

use super::icons::{self, Icon};
use super::props;
use super::theme::Palette;
use super::type_views::{line_sample, paint_drawing, paper, side_labels};
use super::view3d_ui::View3dState;
use super::widgets::{segmented, strong_font};
use super::{
    default_opening_settings, default_slab_settings, default_wall_settings, length_field,
    text_edit_committed, ArchiSimApp,
};
use crate::type_drawing::{
    layers_section, material_swatch, opening_elevation, opening_section, Drawing,
};
use crate::type_preview::{draw, layers_model, opening_model, slab_piece_model, wall_piece_model};
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

/// What the preview of the chosen type shows.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Hash)]
pub enum PreviewView {
    #[default]
    Plan,
    Section,
    Elevation,
    ThreeD,
}

/// The previews each list offers, the first one by default.
pub fn views(tab: TypesTab) -> &'static [(PreviewView, &'static str, &'static str)] {
    const PLAN: (PreviewView, &str, &str) = (PreviewView::Plan, "Planta", "Corte en planta, 1:20");
    const SECTION: (PreviewView, &str, &str) = (PreviewView::Section, "Sección", "Corte vertical");
    const ELEVATION: (PreviewView, &str, &str) = (
        PreviewView::Elevation,
        "Alzado",
        "Visto desde el lado hacia el que abre",
    );
    const THREE_D: (PreviewView, &str, &str) = (PreviewView::ThreeD, "3D", "Arrastra para girarlo");
    match tab {
        TypesTab::Walls => &[PLAN, SECTION, THREE_D],
        TypesTab::Slabs => &[SECTION, THREE_D],
        TypesTab::Doors | TypesTab::Windows => &[ELEVATION, PLAN, SECTION, THREE_D],
        TypesTab::Materials => &[PLAN],
    }
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
    /// The preview chosen (kept while it applies to the list shown).
    pub view: PreviewView,
    /// Search in the list, by name or mark.
    pub filter: String,
    /// Colour being picked, applied when the colour picker closes so that
    /// dragging across the picker is one undo step.
    color_draft: Option<(MaterialId, [u8; 3])>,
    /// The last plan preview drawn and what it shows.
    pub(super) preview: Option<(String, TextureHandle, Vec2)>,
    /// Elevations and sections already drawn, by what they show.
    drawings: HashMap<String, Drawing>,
    /// The 3D preview, with a camera and a scene of its own.
    pub(super) v3d: View3dState,
    /// What the preview showed in the last frame ("plan:…", "section:…").
    pub(super) shown: String,
    /// Bring the chosen type into view in the list (it was chosen from
    /// outside the list, or just made).
    reveal: bool,
}

/// What the dialog was asked to do this frame, applied after drawing it.
pub(super) enum Action {
    Select(TypesTab, u64),
    /// Open the window with every material's priority.
    ShowPriorities,
    DuplicateMaterial(MaterialId),
    RemoveMaterial(MaterialId),
    EditMaterial(MaterialId, String, String, [u8; 3], CutPattern),
    MaterialPriority(MaterialId, Option<u16>),
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

/// A folder of the list: types that share what cannot change between them
/// (the family of a door, the function of a wall, the category of a
/// material).
struct Folder {
    key: String,
    title: String,
    /// For doors and windows, the family new types of the folder come
    /// from.
    family: Option<String>,
    rows: Vec<Row>,
}

struct Row {
    id: u64,
    name: String,
    /// Type mark, shown as a badge.
    badge: String,
    detail: String,
    /// Key of the small drawing in [`TypesDialog::drawings`].
    thumb: String,
}

/// Size of the drawings of the list.
const THUMB: f32 = 46.0;
/// Height of the preview.
const PREVIEW_HEIGHT: f32 = 220.0;
/// Width of the list.
const LIST_WIDTH: f32 = 300.0;

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

/// Where a material category goes in the list (unknown ones last).
fn category_order(code: &str) -> usize {
    CATEGORIES
        .iter()
        .position(|(c, _)| *c == code)
        .unwrap_or(CATEGORIES.len())
}

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
        self.types.reveal = true;
        if self.types.tab != tab {
            self.types.view = views(tab)[0].0;
        }
        self.select_type(tab, id);
    }

    fn select_type(&mut self, tab: TypesTab, id: u64) {
        let d = &mut self.types;
        d.reveal = d.tab != tab || d.reveal;
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
            // The first of the list, as its folders show it.
            d.material = m
                .materials()
                .min_by_key(|x| (category_order(&x.category), x.name.to_lowercase()))
                .map(|x| x.id);
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
            .is_some_and(|t| !t.is_door())
        {
            d.window = first(false);
        }
        let views = views(d.tab);
        if !views.iter().any(|v| v.0 == d.view) {
            d.view = views[0].0;
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
            .collapsible(false)
            .default_size([1060.0, 780.0])
            .min_size([820.0, 520.0])
            .show(ctx, |ui| self.types_body(ui, &mut actions));
        self.types.open = open;
        for a in actions {
            self.apply_type_action(a);
        }
        if self.types.drawings.len() > 400 {
            self.types.drawings.clear();
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
        let p = Palette::current(ui.ctx());
        ui.horizontal(|ui| {
            ui.allocate_ui(Vec2::new(520.0, 30.0), |ui| {
                let before = self.types.tab;
                segmented(
                    ui,
                    "types-tabs",
                    &mut self.types.tab,
                    &[
                        (TypesTab::Walls, "Muros", "Tipos de muro (IfcWallType)"),
                        (TypesTab::Slabs, "Forjados", "Tipos de forjado (IfcSlabType)"),
                        (TypesTab::Doors, "Puertas", "Tipos de puerta (IfcDoorType)"),
                        (
                            TypesTab::Windows,
                            "Ventanas",
                            "Tipos de ventana (IfcWindowType) y huecos",
                        ),
                        (TypesTab::Materials, "Materiales", "Materiales (IfcMaterial)"),
                    ],
                );
                if before != self.types.tab {
                    self.types.filter.clear();
                    // Each list opens on its usual view: elevation for
                    // doors and windows, section for slabs, plan for walls.
                    self.types.view = views(self.types.tab)[0].0;
                }
            });
            ui.add_space(12.0);
            ui.label(
                RichText::new(
                    "Los cambios se aplican al momento a todos los elementos del tipo; Ctrl+Z los deshace.",
                )
                .color(p.muted)
                .size(11.5),
            );
        });
        ui.add_space(6.0);
        let height = ui.available_height().max(360.0);
        ui.horizontal_top(|ui| {
            ui.vertical(|ui| {
                ui.set_width(LIST_WIDTH);
                ui.set_height(height);
                self.type_list(ui, actions);
            });
            ui.add(egui::Separator::default().vertical().spacing(14.0));
            ui.vertical(|ui| {
                ui.set_height(height);
                self.type_editor(ui, actions);
            });
        });
    }

    // ---- Lists ----------------------------------------------------------------

    /// The folders of the list shown, with their types (those matching the
    /// search), and their drawings ready in the cache.
    fn folders(&mut self) -> Vec<Folder> {
        let m = self.ed.model();
        let tab = self.types.tab;
        let filter = self.types.filter.trim().to_lowercase();
        let matches = |name: &str, mark: &str| {
            filter.is_empty()
                || name.to_lowercase().contains(&filter)
                || mark.to_lowercase().contains(&filter)
        };
        let drawings = &mut self.types.drawings;
        let mut folders: Vec<Folder> = Vec::new();
        let put = |folders: &mut Vec<Folder>, key: String, title: String, row: Option<Row>| {
            let f = match folders.iter().position(|f| f.key == key) {
                Some(i) => &mut folders[i],
                None => {
                    folders.push(Folder {
                        key,
                        title,
                        family: None,
                        rows: Vec::new(),
                    });
                    folders.last_mut().expect("just pushed")
                }
            };
            f.rows.extend(row);
        };
        match tab {
            TypesTab::Walls => {
                for f in [WallFunction::Exterior, WallFunction::Interior] {
                    for t in m.wall_types().filter(|t| t.function == f) {
                        if !matches(&t.name, "") {
                            continue;
                        }
                        let n = m.wall_type_users(t.id);
                        let (key, _) = layers_preview(m, &t.layers, 1.0);
                        let thumb = format!("thumb:{key}");
                        drawings
                            .entry(thumb.clone())
                            .or_insert_with(|| layers_section(m, &t.layers, false));
                        put(
                            &mut folders,
                            format!("{f:?}"),
                            wall_folder_name(f).into(),
                            Some(Row {
                                id: t.id.0,
                                name: t.name.clone(),
                                badge: String::new(),
                                detail: format!(
                                    "{} mm · {n} muro{}",
                                    fmt_mm(t.thickness()),
                                    plural(n)
                                ),
                                thumb,
                            }),
                        );
                    }
                }
            }
            TypesTab::Slabs => {
                for f in [
                    SlabFunction::Floor,
                    SlabFunction::BaseSlab,
                    SlabFunction::Roof,
                ] {
                    for t in m.slab_types().filter(|t| t.function == f) {
                        if !matches(&t.name, "") {
                            continue;
                        }
                        let n = m.slab_type_users(t.id);
                        let (key, _) = layers_preview(m, &t.layers, 1.0);
                        let thumb = format!("thumb-slab:{key}");
                        drawings
                            .entry(thumb.clone())
                            .or_insert_with(|| layers_section(m, &t.layers, true));
                        put(
                            &mut folders,
                            format!("{f:?}"),
                            slab_folder_name(f).into(),
                            Some(Row {
                                id: t.id.0,
                                name: t.name.clone(),
                                badge: String::new(),
                                detail: format!(
                                    "{} mm · {n} forjado{}",
                                    fmt_mm(t.thickness()),
                                    plural(n)
                                ),
                                thumb,
                            }),
                        );
                    }
                }
            }
            TypesTab::Doors | TypesTab::Windows => {
                let doors = tab == TypesTab::Doors;
                // Every family that can make types of the list, with or
                // without types yet, so that new ones can be made from it.
                for family in self.ed.opening_families(doors) {
                    put(&mut folders, family.clone(), family, None);
                }
                for t in m.opening_types().filter(|t| t.is_door() == doors) {
                    let family = m
                        .family(t.family)
                        .map_or("Otras".to_string(), |f| f.name().to_string());
                    if !matches(&t.name, &t.mark) {
                        put(&mut folders, family.clone(), family, None);
                        continue;
                    }
                    let n = m.opening_type_users(t.id);
                    let thumb = format!("thumb:{}", opening_key(m, t));
                    drawings
                        .entry(thumb.clone())
                        .or_insert_with(|| opening_elevation(m, t).unwrap_or_else(empty));
                    put(
                        &mut folders,
                        family.clone(),
                        family,
                        Some(Row {
                            id: t.id.0,
                            name: t.name.clone(),
                            badge: t.mark.clone(),
                            detail: format!(
                                "{} × {} · {n} colocada{}",
                                fmt_mm(t.width),
                                fmt_mm(t.height),
                                plural(n)
                            ),
                            thumb,
                        }),
                    );
                }
                for f in &mut folders {
                    if f.title != "Otras" {
                        f.family = Some(f.title.clone());
                    }
                }
            }
            TypesTab::Materials => {
                let mut mats: Vec<_> = m.materials().collect();
                let order = category_order;
                mats.sort_by(|a, b| {
                    (order(&a.category), a.name.to_lowercase())
                        .cmp(&(order(&b.category), b.name.to_lowercase()))
                });
                for mat in mats {
                    if !matches(&mat.name, "") {
                        continue;
                    }
                    let n = m.material_users(mat.id).len();
                    let thumb = format!("mat:{:?}", (mat.cut_color, mat.cut_pattern));
                    drawings
                        .entry(thumb.clone())
                        .or_insert_with(|| material_swatch(m, mat.id));
                    put(
                        &mut folders,
                        mat.category.clone(),
                        category_name(&mat.category),
                        Some(Row {
                            id: mat.id.0,
                            name: mat.name.clone(),
                            badge: String::new(),
                            detail: if n == 0 {
                                "sin usar".to_string()
                            } else {
                                format!("en {n} tipo{}", plural(n))
                            },
                            thumb,
                        }),
                    );
                }
            }
        }
        folders
    }

    fn type_list(&mut self, ui: &mut Ui, actions: &mut Vec<Action>) {
        let p = Palette::current(ui.ctx());
        let tab = self.types.tab;
        ui.add(
            egui::TextEdit::singleline(&mut self.types.filter)
                .hint_text("Buscar por nombre o marca")
                .desired_width(f32::INFINITY),
        );
        ui.add_space(4.0);
        let folders = self.folders();
        let selected = self.selected_id(tab);
        let doors = tab == TypesTab::Doors;
        egui::ScrollArea::vertical()
            .id_salt(("type-list", tab as u8))
            .max_height(ui.available_height() - 40.0)
            .auto_shrink([false, false])
            .show(ui, |ui| {
                for f in &folders {
                    let id = ui.make_persistent_id(("types-folder", tab as u8, f.key.as_str()));
                    let mut state = CollapsingState::load_with_default_open(ui.ctx(), id, true);
                    let searching = !self.types.filter.trim().is_empty();
                    if searching && f.rows.is_empty() {
                        continue;
                    }
                    let (toggle, new) = folder_header(ui, f, state.is_open() || searching);
                    if toggle {
                        state.toggle(ui);
                    }
                    if new {
                        if let Some(family) = &f.family {
                            actions.push(Action::NewOpening(family.clone(), doors));
                        }
                    }
                    if !(state.is_open() || searching) {
                        state.store(ui.ctx());
                        continue;
                    }
                    state.store(ui.ctx());
                    if f.rows.is_empty() {
                        ui.horizontal(|ui| {
                            ui.add_space(30.0);
                            ui.label(
                                RichText::new("Sin tipos: «＋» crea uno de esta familia")
                                    .color(p.muted)
                                    .size(11.5),
                            );
                        });
                    }
                    for r in &f.rows {
                        let drawing = self.types.drawings.get(&r.thumb);
                        let on = Some(r.id) == selected;
                        let resp = type_row(ui, r, drawing, on);
                        if on && self.types.reveal {
                            resp.scroll_to_me(Some(egui::Align::Center));
                            self.types.reveal = false;
                        }
                        if resp.clicked() {
                            actions.push(Action::Select(tab, r.id));
                        }
                    }
                    ui.add_space(4.0);
                }
            });
        ui.add_space(6.0);
        ui.horizontal(|ui| {
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

    // ---- Editor -----------------------------------------------------------------

    fn type_editor(&mut self, ui: &mut Ui, actions: &mut Vec<Action>) {
        let p = Palette::current(ui.ctx());
        let tab = self.types.tab;
        let Some(id) = self.selected_id(tab) else {
            ui.label(RichText::new("No hay ningún tipo en esta lista.").color(p.muted));
            return;
        };
        let m = self.ed.model();
        let (title, subtitle) = match tab {
            TypesTab::Materials => (
                m.material(MaterialId(id)).map(|x| x.name.clone()),
                "Material · IfcMaterial".to_string(),
            ),
            TypesTab::Walls => (
                m.wall_type(WallTypeId(id)).map(|x| x.name.clone()),
                "Tipo de muro · IfcWallType".to_string(),
            ),
            TypesTab::Slabs => (
                m.slab_type(SlabTypeId(id)).map(|x| x.name.clone()),
                "Tipo de forjado · IfcSlabType".to_string(),
            ),
            TypesTab::Doors | TypesTab::Windows => {
                let t = m.opening_type(OpeningTypeId(id));
                let class = match t {
                    Some(t) if t.is_door() => "Tipo de puerta · IfcDoorType",
                    Some(t) if t.is_void() => "Hueco de obra · IfcOpeningElement",
                    _ => "Tipo de ventana · IfcWindowType",
                };
                (
                    t.map(|t| {
                        if t.mark.is_empty() {
                            t.name.clone()
                        } else {
                            format!("{}  ·  {}", t.mark, t.name)
                        }
                    }),
                    format!(
                        "{class} · familia {}",
                        t.map_or("—".into(), |t| family_label(m, t.family))
                    ),
                )
            }
        };
        let Some(title) = title else { return };
        ui.label(RichText::new(title).font(strong_font(16.0)).color(p.text));
        ui.label(RichText::new(subtitle).color(p.muted).size(11.5));
        ui.add_space(6.0);
        self.preview_area(ui, id);
        // Why the last change was refused, where it is seen.
        if let Some(msg) = self.message.as_ref().filter(|m| m.error) {
            ui.add_space(4.0);
            ui.colored_label(ui.visuals().error_fg_color, format!("⚠ {}", msg.text));
        }
        ui.add_space(4.0);
        egui::ScrollArea::vertical()
            .id_salt(("type-editor", tab as u8))
            .auto_shrink([false, false])
            .show(ui, |ui| {
                ui.set_max_width(ui.available_width() - 8.0);
                match tab {
                    TypesTab::Materials => self.material_editor(ui, MaterialId(id), actions),
                    TypesTab::Walls => self.wall_type_editor(ui, WallTypeId(id), actions),
                    TypesTab::Slabs => self.slab_type_editor(ui, SlabTypeId(id), actions),
                    TypesTab::Doors | TypesTab::Windows => {
                        self.opening_type_editor(ui, OpeningTypeId(id), actions)
                    }
                }
            });
    }

    // ---- Previews ---------------------------------------------------------------

    /// The view switcher and the preview of the chosen type.
    fn preview_area(&mut self, ui: &mut Ui, id: u64) {
        let p = Palette::current(ui.ctx());
        let tab = self.types.tab;
        let options = views(tab);
        if options.len() > 1 {
            ui.allocate_ui(
                Vec2::new(
                    (options.len() as f32 * 86.0).min(ui.available_width()),
                    30.0,
                ),
                |ui| {
                    segmented(ui, ("type-view", tab as u8), &mut self.types.view, options);
                },
            );
            ui.add_space(6.0);
        }
        let view = self.types.view;
        let size = Vec2::new(ui.available_width(), PREVIEW_HEIGHT);
        let m = self.ed.model();
        // What is drawn: the key that says when it changed, and how.
        let (key, sides): (String, (&str, &str)) = match tab {
            TypesTab::Materials => {
                let Some(mat) = m.material(MaterialId(id)) else {
                    return;
                };
                let rgb = match self.types.color_draft {
                    Some((d, rgb)) if d.0 == id => rgb,
                    _ => mat.cut_color,
                };
                (
                    format!("mat:{}:{rgb:?}:{:?}", mat.id.0, mat.cut_pattern),
                    ("", ""),
                )
            }
            TypesTab::Walls => {
                let Some(t) = m.wall_type(WallTypeId(id)) else {
                    return;
                };
                (
                    layers_preview(m, &t.layers, 1.0).0,
                    ("Exterior", "Interior"),
                )
            }
            TypesTab::Slabs => {
                let Some(t) = m.slab_type(SlabTypeId(id)) else {
                    return;
                };
                (
                    format!("slab:{}", layers_preview(m, &t.layers, 1.0).0),
                    ("", ""),
                )
            }
            TypesTab::Doors | TypesTab::Windows => {
                let Some(t) = m.opening_type(OpeningTypeId(id)) else {
                    return;
                };
                (opening_key(m, t), ("", ""))
            }
        };
        let caption = match (tab, view) {
            (TypesTab::Materials, _) => "Corte en planta de un muro de 20 cm · 1:20",
            (TypesTab::Walls, PreviewView::Plan) => "Corte en planta · 1:20 · exterior arriba",
            (TypesTab::Walls, PreviewView::Section) => "Sección vertical · exterior a la izquierda",
            (TypesTab::Slabs, PreviewView::Section) => "Sección · cara superior arriba",
            (_, PreviewView::Plan) => "Planta · 1:20 · lado de apertura abajo",
            (_, PreviewView::Section) => {
                "Sección por la primera hoja · exterior a la izquierda, lado de apertura a la derecha"
            }
            (_, PreviewView::Elevation) => "Alzado visto desde el lado hacia el que abre",
            (_, PreviewView::ThreeD) => {
                "Arrastra para girar · rueda para acercar · doble clic para encuadrar"
            }
        };
        match view {
            PreviewView::ThreeD => {
                let build = || -> Option<Model> {
                    match tab {
                        TypesTab::Walls => {
                            wall_piece_model(m, &m.wall_type(WallTypeId(id))?.layers)
                        }
                        TypesTab::Slabs => {
                            slab_piece_model(m, &m.slab_type(SlabTypeId(id))?.layers)
                        }
                        TypesTab::Doors | TypesTab::Windows => {
                            opening_model(m, m.opening_type(OpeningTypeId(id))?).map(|x| x.0)
                        }
                        TypesTab::Materials => None,
                    }
                };
                let hash = {
                    use std::hash::{Hash, Hasher};
                    let mut h = std::collections::hash_map::DefaultHasher::new();
                    key.hash(&mut h);
                    h.finish()
                };
                self.types
                    .v3d
                    .preview(ui, self.gpu.as_ref(), hash, build, size);
                self.types.shown = format!("3d:{key}");
            }
            PreviewView::Section | PreviewView::Elevation => {
                let dkey = format!("{view:?}:{key}");
                if !self.types.drawings.contains_key(&dkey) {
                    let d = match tab {
                        TypesTab::Walls => m
                            .wall_type(WallTypeId(id))
                            .map(|t| layers_section(m, &t.layers, false)),
                        TypesTab::Slabs => m
                            .slab_type(SlabTypeId(id))
                            .map(|t| layers_section(m, &t.layers, true)),
                        TypesTab::Doors | TypesTab::Windows => {
                            m.opening_type(OpeningTypeId(id)).and_then(|t| {
                                if view == PreviewView::Section {
                                    opening_section(m, t)
                                } else {
                                    opening_elevation(m, t)
                                }
                            })
                        }
                        TypesTab::Materials => None,
                    };
                    self.types
                        .drawings
                        .insert(dkey.clone(), d.unwrap_or_else(empty));
                }
                let (rect, _) = ui.allocate_exact_size(size, Sense::hover());
                let painter = ui.painter_at(rect);
                paper(&painter, rect, p.line);
                if let Some(d) = self.types.drawings.get(&dkey) {
                    // At most a 1:10 detail on a screen of 96 points per inch.
                    paint_drawing(&painter, rect.shrink(14.0), d, 380.0, 1.0);
                    let labels = if d.sides.0.is_empty() { sides } else { d.sides };
                    side_labels(&painter, rect, labels, Color32::from_gray(110));
                }
                self.types.shown = format!(
                    "{}:{key}",
                    if view == PreviewView::Section {
                        "section"
                    } else {
                        "elevation"
                    }
                );
            }
            PreviewView::Plan => {
                let built = match tab {
                    TypesTab::Materials => m.material(MaterialId(id)).and_then(|mat| {
                        // A 20 cm wall of the material alone, cut in plan.
                        let mut pm = Model::new("m");
                        let rgb = match self.types.color_draft {
                            Some((d, rgb)) if d.0 == id => rgb,
                            _ => mat.cut_color,
                        };
                        let pid = pm.add_material(
                            mat.name.clone(),
                            mat.category.clone(),
                            rgb,
                            mat.cut_pattern,
                        );
                        let layers = [Layer {
                            material: pid,
                            thickness: 0.2,
                            function: LayerFunction::Structure,
                        }];
                        layers_model(&pm, &layers, 0.8)
                    }),
                    TypesTab::Walls => m
                        .wall_type(WallTypeId(id))
                        .and_then(|t| layers_model(m, &t.layers, 1.0)),
                    TypesTab::Slabs => None,
                    TypesTab::Doors | TypesTab::Windows => m
                        .opening_type(OpeningTypeId(id))
                        .and_then(|t| opening_model(m, t)),
                };
                self.plan_preview(ui, size, key.clone(), built, sides);
                self.types.shown = format!("plan:{key}");
            }
        }
        ui.label(RichText::new(caption).color(p.muted).size(11.0));
    }

    /// A preview model drawn in plan by the plan writer at 1:20, its
    /// picture redrawn only when what it shows changed.
    fn plan_preview(
        &mut self,
        ui: &mut Ui,
        size: Vec2,
        key: String,
        built: Option<(Model, archisim_core::LevelId)>,
        sides: (&str, &str),
    ) {
        let p = Palette::current(ui.ctx());
        let (rect, _) = ui.allocate_exact_size(size, Sense::hover());
        let painter = ui.painter_at(rect);
        paper(&painter, rect, p.line);
        let area = rect.shrink(14.0);
        let Some((model, level)) = built else {
            painter.text(
                rect.center(),
                Align2::CENTER_CENTER,
                "Sin vista previa",
                FontId::proportional(12.0),
                Color32::GRAY,
            );
            return;
        };
        let ppp = ui.ctx().pixels_per_point();
        let length = model.walls().next().map_or(1.0, |w| w.length()) + 0.04;
        let points_per_m = (area.width() as f64 / length).clamp(50.0, 600.0);
        let full_key = format!("{key}:{:.0}", area.width());
        let stale = self
            .types
            .preview
            .as_ref()
            .is_none_or(|(k, _, _)| *k != full_key);
        if stale {
            let (img, lo, hi) = draw(&model, level, points_per_m * ppp as f64);
            let natural = Vec2::new(
                ((hi.x - lo.x) * points_per_m) as f32,
                ((hi.y - lo.y) * points_per_m) as f32,
            );
            let tex = ui
                .ctx()
                .load_texture("type-preview", img, TextureOptions::LINEAR);
            self.types.preview = Some((full_key, tex, natural));
        }
        let Some((_, tex, natural)) = &self.types.preview else {
            return;
        };
        let k = (area.width() / natural.x)
            .min(area.height() / natural.y)
            .min(1.0);
        let shown = Rect::from_center_size(area.center(), *natural * k);
        painter.image(
            tex.id(),
            shown,
            Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)),
            Color32::WHITE,
        );
        if !sides.0.is_empty() {
            let font = FontId::proportional(10.5);
            painter.text(
                Pos2::new(rect.left() + 8.0, shown.top() + 2.0),
                Align2::LEFT_TOP,
                format!("▲ {}", sides.0),
                font.clone(),
                Color32::from_gray(110),
            );
            painter.text(
                Pos2::new(rect.left() + 8.0, shown.bottom() - 2.0),
                Align2::LEFT_BOTTOM,
                format!("▼ {}", sides.1),
                font,
                Color32::from_gray(110),
            );
        }
    }

    // ---- Editors of each list ------------------------------------------------------

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
        let mut draft = None;
        let mut bad_rgb = false;
        props::group(ui, "types-material", "Material", |ui| {
            props::row(ui, "Nombre", "", |ui, w| {
                if let Some(new) =
                    text_edit_committed(ui, Id::unique(("mat-name", id.0)), &mat.name, w - 8.0)
                {
                    actions.push(Action::EditMaterial(
                        id,
                        new,
                        mat.category.clone(),
                        mat.cut_color,
                        mat.cut_pattern,
                    ));
                }
            });
            props::row(
                ui,
                "Categoría (IFC)",
                "IfcMaterial.Category. Los vidrios se ven transparentes en 3D y en los visores IFC.",
                |ui, w| {
                    let mut cat = mat.category.clone();
                    ComboBox::from_id_salt(("mat-cat", id.0))
                        .width(w - 8.0)
                        .selected_text(category_name(&cat))
                        .show_ui(ui, |ui| {
                            for (code, name) in CATEGORIES {
                                ui.selectable_value(&mut cat, (*code).to_string(), *name);
                            }
                        });
                    if cat != mat.category {
                        actions.push(Action::EditMaterial(id, mat.name.clone(), cat, mat.cut_color, mat.cut_pattern));
                    }
                },
            );
            props::row(
                ui,
                "Color de corte",
                "Rojo, verde y azul, de 0 a 255",
                |ui, _| {
                    let mut rgb = shown_color;
                    if ui.color_edit_button_srgb(&mut rgb).changed() {
                        draft = Some(rgb);
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
                            None => bad_rgb = true,
                        }
                    }
                },
            );
            props::row(ui, "Trama de corte", "", |ui, w| {
                let mut pat = mat.cut_pattern;
                ComboBox::from_id_salt(("mat-pat", id.0))
                    .width(w - 8.0)
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
            });
            props::row(
                ui,
                "Prioridad de intersección",
                "De 0 a 999: en las uniones en T, una capa atraviesa las del otro muro de menor prioridad; entre muro y forjado, se queda el sitio la de más prioridad (a igualdad, el forjado). Vacío: según la función de cada capa (estructura 900, soporte 700, aislante 500, lámina o cámara 300, acabado 100).",
                |ui, w| {
                    let shown = mat.priority.map(|p| p.to_string()).unwrap_or_default();
                    if let Some(new) =
                        text_edit_committed(ui, Id::unique(("mat-prio", id.0)), &shown, w - 8.0)
                    {
                        let t = new.trim();
                        if t.is_empty() {
                            actions.push(Action::MaterialPriority(id, None));
                        } else if let Ok(p) = t.parse::<u16>() {
                            actions.push(Action::MaterialPriority(id, Some(p)));
                        } else {
                            actions.push(Action::MaterialPriority(id, Some(u16::MAX)));
                        }
                    }
                },
            );
            ui.horizontal(|ui| {
                ui.add_space(4.0);
                if ui
                    .small_button("Ver todas las prioridades…")
                    .on_hover_text("Una barra por material, como en ArchiCAD")
                    .clicked()
                {
                    actions.push(Action::ShowPriorities);
                }
            });
        });
        if let Some(rgb) = draft {
            self.types.color_draft = Some((id, rgb));
        }
        if bad_rgb {
            self.complain("El color se escribe como tres números de 0 a 255: rojo, verde, azul.");
        }
        props::group(ui, "types-material-use", "Se usa en", |ui| {
            if users.is_empty() {
                props::note(ui, "Ningún tipo lo usa: se puede borrar.");
            } else {
                for u in &users {
                    props::text_row(ui, "Tipo", "", u);
                }
            }
        });
    }

    fn wall_type_editor(&mut self, ui: &mut Ui, id: WallTypeId, actions: &mut Vec<Action>) {
        let m = self.ed.model();
        let Some(t) = m.wall_type(id).cloned() else {
            return;
        };
        let users = m.wall_type_users(id);
        props::group(ui, "types-wall-id", "Identidad y función", |ui| {
            props::row(ui, "Nombre", "", |ui, w| {
                if let Some(new) =
                    text_edit_committed(ui, Id::unique(("wt-name", id.0)), &t.name, w - 8.0)
                {
                    actions.push(Action::EditWall(id, new, t.function, t.layers.clone()));
                }
            });
            props::row(
                ui,
                "Función",
                "Exterior: muros de fachada (IsExternal en IFC); al dibujarlos, la línea de referencia pasa a la cara exterior",
                |ui, w| {
                    let f = props::choice(
                        ui,
                        ("wt-fun", id.0),
                        w,
                        t.function,
                        &[
                            (WallFunction::Exterior, wall_function_name(WallFunction::Exterior)),
                            (WallFunction::Interior, wall_function_name(WallFunction::Interior)),
                        ],
                    );
                    if f != t.function {
                        actions.push(Action::EditWall(id, t.name.clone(), f, t.layers.clone()));
                    }
                },
            );
            props::text_row(
                ui,
                "Espesor total",
                "",
                &format!("{} mm", fmt_mm(t.thickness())),
            );
            props::text_row(ui, "Muros de este tipo", "", &users.to_string());
            props::id_row(ui, "GlobalId", t.guid.as_str());
        });
        props::group(ui, "types-wall-layers", "Capas", |ui| {
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
        });
    }

    fn slab_type_editor(&mut self, ui: &mut Ui, id: SlabTypeId, actions: &mut Vec<Action>) {
        let m = self.ed.model();
        let Some(t) = m.slab_type(id).cloned() else {
            return;
        };
        let users = m.slab_type_users(id);
        props::group(ui, "types-slab-id", "Identidad y función", |ui| {
            props::row(ui, "Nombre", "", |ui, w| {
                if let Some(new) =
                    text_edit_committed(ui, Id::unique(("st-name", id.0)), &t.name, w - 8.0)
                {
                    actions.push(Action::EditSlab(id, new, t.function, t.layers.clone()));
                }
            });
            props::row(
                ui,
                "Función",
                "Forjado de planta (FLOOR), solera (BASESLAB) o cubierta plana (ROOF); solera y cubierta son exteriores",
                |ui, w| {
                    let f = props::choice(
                        ui,
                        ("st-fun", id.0),
                        w,
                        t.function,
                        &[
                            (SlabFunction::Floor, slab_function_name(SlabFunction::Floor)),
                            (SlabFunction::BaseSlab, slab_function_name(SlabFunction::BaseSlab)),
                            (SlabFunction::Roof, slab_function_name(SlabFunction::Roof)),
                        ],
                    );
                    if f != t.function {
                        actions.push(Action::EditSlab(id, t.name.clone(), f, t.layers.clone()));
                    }
                },
            );
            props::text_row(
                ui,
                "Canto total",
                "",
                &format!("{} mm", fmt_mm(t.thickness())),
            );
            props::text_row(ui, "Forjados de este tipo", "", &users.to_string());
            props::id_row(ui, "GlobalId", t.guid.as_str());
        });
        props::group(ui, "types-slab-layers", "Capas", |ui| {
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
        });
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
        let mut info: Option<(String, String, String)> = None;
        props::group(ui, "types-opening-id", "Identidad", |ui| {
            props::row(ui, "Nombre", "", |ui, w| {
                if let Some(new) =
                    text_edit_committed(ui, Id::unique(("ot-name", id.0)), &t.name, w - 8.0)
                {
                    info = Some((new, t.mark.clone(), t.description.clone()));
                }
            });
            props::row(
                ui,
                "Marca de tipo",
                "Para etiquetas, tablas y mediciones (P-01, V-03…); única en el proyecto",
                |ui, _| {
                    if let Some(new) =
                        text_edit_committed(ui, Id::unique(("ot-mark", id.0)), &t.mark, 90.0)
                    {
                        info = Some((t.name.clone(), new, t.description.clone()));
                    }
                },
            );
            props::row(ui, "Descripción", "", |ui, w| {
                if let Some(new) =
                    text_edit_committed(ui, Id::unique(("ot-desc", id.0)), &t.description, w - 8.0)
                {
                    info = Some((t.name.clone(), t.mark.clone(), new));
                }
            });
            props::text_row(
                ui,
                "Familia",
                "La receta de la que sale este tipo; abajo, sus parámetros. Una familia no se convierte en otra: para otro funcionamiento, crea un tipo nuevo en su carpeta",
                &family_label(m, t.family),
            );
        });
        if let Some((name, mark, description)) = info {
            actions.push(Action::EditOpeningInfo(id, name, mark, description));
        }

        // The family's type parameters, a group per section of the recipe;
        // those that do not apply (a fourth leaf of a window with two) are
        // hidden.
        let def = family.def();
        let mut sections: Vec<(Option<&str>, Vec<&archisim_core::family::Param>)> = Vec::new();
        for p in def.params_of(archisim_core::ParamScope::Type) {
            if !def.is_visible(&p.name, &t.values) {
                continue;
            }
            match sections.last_mut() {
                Some((s, rows)) if *s == p.section.as_deref() => rows.push(p),
                _ => sections.push((p.section.as_deref(), vec![p])),
            }
        }
        for (title, rows) in sections {
            let title = title.unwrap_or("Parámetros");
            props::group(ui, &format!("types-opening-{title}"), title, |ui| {
                for p in rows {
                    param_row(ui, m, id, &t, p, actions);
                }
            });
        }

        props::group(ui, "types-opening-data", "Datos", |ui| {
            props::text_row(
                ui,
                if door {
                    "Puertas de este tipo"
                } else {
                    "Colocadas de este tipo"
                },
                "",
                &users.to_string(),
            );
            props::id_row(ui, "GlobalId", t.guid.as_str());
        });
        props::note(
            ui,
            "Al cambiar las medidas, cada hueco colocado conserva su centro y su antepecho; si alguno dejara de caber en su muro, el cambio no se hace.",
        );
    }

    // ---- Applying -----------------------------------------------------------------

    pub(super) fn apply_type_action(&mut self, a: Action) {
        match a {
            Action::Select(tab, id) => self.select_type(tab, id),
            Action::DuplicateMaterial(id) => match self.ed.duplicate_material(id) {
                Ok(new) => {
                    self.types.material = Some(new);
                    self.types.reveal = true;
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
            Action::ShowPriorities => self.show_priorities = true,
            Action::MaterialPriority(id, p) => {
                let r = self.ed.set_material_priority(id, p);
                self.report(
                    r,
                    "Prioridad cambiada: las uniones en T y los encuentros con forjados se rehacen con ella.",
                );
            }
            Action::DuplicateWall(id) => match self.ed.duplicate_wall_type(id) {
                Ok(new) => {
                    self.types.wall = Some(new);
                    self.types.reveal = true;
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
                    self.types.reveal = true;
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
                    self.types.reveal = true;
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
                    self.types.reveal = true;
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

/// One family parameter of a door or window type: its name and the field
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
    let current = t.values.get(&p.name).cloned();
    let field = Id::unique(("ot-param", id.0, p.name.as_str()));
    let mut set = |v: Value| actions.push(Action::EditOpeningValue(id, p.name.clone(), v));
    props::row(ui, &label, "", |ui, w| match (&p.kind, current) {
        (ParamKind::Length, Some(Value::Number(v))) => {
            if let Some(new) = length_field(ui, field, v, 90.0) {
                set(Value::Number(new));
            }
        }
        (ParamKind::Integer, Some(Value::Number(v)))
            if p.range.is_some_and(|(lo, hi)| hi - lo <= 12.0) =>
        {
            let (lo, hi) = p.range.expect("checked");
            let mut n = v as i64;
            ComboBox::from_id_salt(field)
                .width(90.0)
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
            if let Some(text) = text_edit_committed(ui, field, &shown, 90.0) {
                if let Ok(deg) = text.trim().replace(',', ".").parse::<f64>() {
                    set(Value::Number(deg.to_radians()));
                }
            }
        }
        (ParamKind::Number | ParamKind::Integer, Some(Value::Number(v))) => {
            let shown = format!("{v}").replace('.', ",");
            if let Some(text) = text_edit_committed(ui, field, &shown, 90.0) {
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
                .width((w - 8.0).min(220.0))
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
        (ParamKind::LineStyle, Some(Value::Text(code))) => {
            let style = LineStyle::from_code(&code).unwrap_or(LineStyle::Continuous);
            let mut chosen = style;
            let color = Palette::current(ui.ctx()).text;
            ComboBox::from_id_salt(field)
                .width((w - 70.0).clamp(80.0, 180.0))
                .selected_text(style.label())
                .show_ui(ui, |ui| {
                    for s in LineStyle::ALL {
                        ui.horizontal(|ui| {
                            let (r, _) =
                                ui.allocate_exact_size(Vec2::new(44.0, 16.0), Sense::hover());
                            line_sample(ui.painter(), r, s, color);
                            ui.selectable_value(&mut chosen, s, s.label());
                        });
                    }
                });
            let (r, _) = ui.allocate_exact_size(Vec2::new(44.0, 16.0), Sense::hover());
            line_sample(ui.painter(), r, style, color);
            if chosen != style {
                set(Value::Text(chosen.code().to_string()));
            }
        }
        (ParamKind::Material, Some(Value::Material(mid))) => {
            let mut mat = MaterialId(mid);
            material_combo(ui, field, m, &mut mat, (w - 30.0).min(240.0));
            if mat.0 != mid {
                set(Value::Material(mat.0));
            }
        }
        (ParamKind::Text, Some(Value::Text(text))) => {
            if let Some(new) = text_edit_committed(ui, field, &text, w - 8.0) {
                set(Value::Text(new));
            }
        }
        _ => {
            ui.label(RichText::new("—").weak());
        }
    });
}

/// The header of a folder: chevron, folder, name and number of types, and
/// for door and window families a «＋» that makes a new type of it.
/// Returns whether it was clicked (to fold it) and whether «＋» was.
fn folder_header(ui: &mut Ui, f: &Folder, open: bool) -> (bool, bool) {
    let p = Palette::current(ui.ctx());
    let (rect, resp) =
        ui.allocate_exact_size(Vec2::new(ui.available_width(), 28.0), Sense::click());
    let painter = ui.painter_at(rect);
    if resp.hovered() {
        painter.rect_filled(rect, CornerRadius::same(6), p.raised);
    }
    let cy = rect.center().y;
    icons::paint(
        &painter,
        Rect::from_center_size(Pos2::new(rect.left() + 10.0, cy), Vec2::splat(11.0)),
        if open {
            Icon::ChevronDown
        } else {
            Icon::ChevronRight
        },
        p.muted,
    );
    icons::paint(
        &painter,
        Rect::from_center_size(Pos2::new(rect.left() + 28.0, cy), Vec2::splat(17.0)),
        if open { Icon::FolderOpen } else { Icon::Folder },
        p.accent,
    );
    let count = format!("{}", f.rows.len());
    let g = painter.layout_no_wrap(count, FontId::monospace(10.5), p.muted);
    let mut right = rect.right() - 8.0;
    let mut new = false;
    if f.family.is_some() {
        let r = Rect::from_center_size(Pos2::new(right - 10.0, cy), Vec2::splat(22.0));
        let id = ui.make_persistent_id(("folder-new", f.key.as_str()));
        let b = ui.interact(r, id, Sense::click()).on_hover_text(format!(
            "Nuevo tipo de «{}», con sus valores por defecto",
            f.title
        ));
        if b.hovered() {
            painter.rect_filled(r, CornerRadius::same(5), p.accent_soft);
        }
        icons::paint(&painter, r.shrink(4.0), Icon::Plus, p.accent);
        new = b.clicked();
        right = r.left() - 6.0;
    }
    painter.galley(
        Pos2::new(right - g.size().x, cy - g.size().y / 2.0),
        g.clone(),
        p.muted,
    );
    let max = (right - g.size().x - 8.0 - (rect.left() + 42.0)).max(10.0);
    let job = egui::text::LayoutJob {
        wrap: egui::text::TextWrapping::truncate_at_width(max),
        ..egui::text::LayoutJob::single_section(
            f.title.clone(),
            egui::TextFormat::simple(strong_font(12.5), p.text),
        )
    };
    let title = painter.layout_job(job);
    painter.galley(
        Pos2::new(rect.left() + 42.0, cy - title.size().y / 2.0),
        title,
        p.text,
    );
    (resp.clicked() && !new, new)
}

/// A type in its folder: its drawing, name, mark and a line of data.
fn type_row(ui: &mut Ui, r: &Row, drawing: Option<&Drawing>, selected: bool) -> egui::Response {
    let p = Palette::current(ui.ctx());
    let h = THUMB + 8.0;
    let (rect, resp) = ui.allocate_exact_size(Vec2::new(ui.available_width(), h), Sense::click());
    let painter = ui.painter_at(rect);
    if selected {
        painter.rect_filled(rect, CornerRadius::same(6), p.accent_soft);
    } else if resp.hovered() {
        painter.rect_filled(rect, CornerRadius::same(6), p.raised);
    }
    let thumb = Rect::from_min_size(
        Pos2::new(rect.left() + 22.0, rect.top() + 4.0),
        Vec2::splat(THUMB),
    );
    painter.rect(
        thumb,
        CornerRadius::same(4),
        Color32::WHITE,
        Stroke::new(1.0, p.line),
        StrokeKind::Inside,
    );
    if let Some(d) = drawing {
        paint_drawing(
            &painter.with_clip_rect(thumb),
            thumb.shrink(3.0),
            d,
            f32::INFINITY,
            0.6,
        );
    }
    let x = thumb.right() + 10.0;
    let fg = if selected { p.accent } else { p.text };
    let mut name_x = x;
    if !r.badge.is_empty() {
        let g = painter.layout_no_wrap(r.badge.clone(), FontId::monospace(10.5), p.muted);
        let w = g.size().x + 8.0;
        let b = Rect::from_min_size(Pos2::new(x, rect.top() + 9.0), Vec2::new(w, 17.0));
        painter.rect(
            b,
            CornerRadius::same(4),
            p.raised,
            Stroke::new(1.0, p.line),
            StrokeKind::Inside,
        );
        painter.galley(b.center() - g.size() / 2.0, g, p.muted);
        name_x += w + 6.0;
    }
    let text = |s: &str, font: FontId, color: Color32, at: Pos2, max: f32| {
        let job = egui::text::LayoutJob {
            wrap: egui::text::TextWrapping::truncate_at_width(max.max(10.0)),
            ..egui::text::LayoutJob::single_section(
                s.to_owned(),
                egui::TextFormat::simple(font, color),
            )
        };
        let g = painter.layout_job(job);
        painter.galley(at, g, color);
    };
    text(
        &r.name,
        if selected {
            strong_font(12.5)
        } else {
            FontId::proportional(12.5)
        },
        fg,
        Pos2::new(name_x, rect.top() + 9.0),
        rect.right() - 6.0 - name_x,
    );
    text(
        &r.detail,
        FontId::proportional(11.0),
        p.muted,
        Pos2::new(x, rect.top() + 30.0),
        rect.right() - 6.0 - x,
    );
    resp.on_hover_text(&r.name)
}

/// What a door or window's previews depend on: the type, the colours of
/// its materials and the walls that could hold it.
fn opening_key(m: &Model, t: &OpeningType) -> String {
    format!(
        "ot:{:?}:{:?}:{:?}",
        t,
        preview_materials(m, &[t.frame_material, t.panel_material]),
        m.wall_types().map(|w| &w.layers).collect::<Vec<_>>()
    )
}

fn empty() -> Drawing {
    layers_section(&Model::new("empty"), &[], false)
}

fn wall_folder_name(f: WallFunction) -> &'static str {
    match f {
        WallFunction::Exterior => "Muros exteriores (fachada)",
        WallFunction::Interior => "Muros interiores",
    }
}

fn slab_folder_name(f: SlabFunction) -> &'static str {
    match f {
        SlabFunction::Floor => "Forjados de planta",
        SlabFunction::BaseSlab => "Soleras",
        SlabFunction::Roof => "Cubiertas planas",
    }
}

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
