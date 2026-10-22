//! The gallery of objects, as ArchiCAD's library: categories (Mobiliario,
//! Sanitarios, Entorno, Gente, Vehículos) with their folders (Camas,
//! Vegetación…), a picture of each type in plan, and a search box. It
//! offers the project's object types and those of ArchiSim's library the
//! project does not have yet; choosing one of these loads it.

use archisim_core::ObjectTypeId;
use eframe::egui::{self, Id, Ui, Vec2};

use super::props::{self, TypeItem};
use super::theme::Palette;
use super::thumbs::Thumb;
use super::ArchiSimApp;
use crate::units::fmt_mm;

/// Part of the library a tool offers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ObjectScope {
    /// Trees, shrubs and hedges, with people and vehicles.
    Vegetation,
    /// Street furniture.
    Street,
}

impl ObjectScope {
    /// The part of the library of a gallery category, if it is one.
    pub fn of_category(category: &str) -> Option<Self> {
        [ObjectScope::Vegetation, ObjectScope::Street]
            .into_iter()
            .find(|s| s.categories().contains(&category))
    }

    /// The gallery categories it shows, the first open.
    pub fn categories(self) -> &'static [&'static str] {
        match self {
            ObjectScope::Vegetation => &["Entorno", "Gente", "Vehículos"],
            ObjectScope::Street => &["Mobiliario urbano"],
        }
    }

    /// The type the tool starts with.
    pub fn first(self) -> &'static str {
        match self {
            ObjectScope::Vegetation => "Olivo",
            ObjectScope::Street => "Farola 4,5 m",
        }
    }

    pub fn title(self) -> &'static str {
        match self {
            ObjectScope::Vegetation => "Vegetación",
            ObjectScope::Street => "Mobiliario urbano",
        }
    }
}

/// What is chosen in the gallery.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ObjectPick {
    Project(ObjectTypeId),
    /// A type of the library, by its name.
    Library(&'static str),
    /// A type of a file of the library folder: file and type.
    Folder(usize, usize),
    /// A 3D model file of the library folder, by its index.
    FolderModel(usize),
}

/// The object types of the library folder, loaded into a model of their
/// own for their pictures.
pub struct FolderObjects {
    read: std::time::Instant,
    lib: crate::editor::library::Library,
    model: archisim_core::Model,
    /// File, type in it, the type in `model`, its `GlobalId`.
    types: Vec<(usize, usize, ObjectTypeId, String)>,
    /// 3D model files (OBJ, glTF, DAE) of the folder, each brought into a
    /// model of its own.
    models: Vec<FolderModel>,
}

/// A 3D model file of the library folder.
#[derive(Clone)]
pub struct FolderModel {
    pub path: std::path::PathBuf,
    pub kind: archisim_library::imported::ModelKind,
    pub model: std::sync::Arc<archisim_core::Model>,
    pub object_type: ObjectTypeId,
}

/// Models already read from the library folder, by file and the time it
/// was changed: a folder is looked at again every half minute, a model
/// read once.
type ModelCache = std::sync::Mutex<
    std::collections::HashMap<std::path::PathBuf, (std::time::SystemTime, Option<FolderModel>)>,
>;

fn model_cache() -> &'static ModelCache {
    static CACHE: std::sync::OnceLock<ModelCache> = std::sync::OnceLock::new();
    CACHE.get_or_init(Default::default)
}

/// The 3D model files under `dir` (four folders deep at most).
fn model_files(dir: &std::path::Path, depth: usize, out: &mut Vec<std::path::PathBuf>) {
    let Ok(rd) = std::fs::read_dir(dir) else {
        return;
    };
    let mut entries: Vec<_> = rd.flatten().map(|e| e.path()).collect();
    entries.sort();
    for p in entries {
        if p.is_dir() && depth < 4 {
            model_files(&p, depth + 1, out);
        } else if archisim_mesh::is_model_file(&p) {
            out.push(p);
        }
    }
}

/// The kind of a model by the folders it is in and its name.
pub fn kind_of(path: &std::path::Path) -> archisim_library::imported::ModelKind {
    let words: String = path
        .iter()
        .rev()
        .take(4)
        .map(|c| c.to_string_lossy().into_owned())
        .collect::<Vec<_>>()
        .join(" ");
    archisim_library::imported::ModelKind::from_folder(&words)
}

/// A model of the folder, read now or taken from the cache.
fn folder_model(path: &std::path::Path) -> Option<FolderModel> {
    let changed = std::fs::metadata(path).and_then(|m| m.modified()).ok()?;
    if let Some((t, m)) = model_cache().lock().ok()?.get(path) {
        if *t == changed {
            return m.clone();
        }
    }
    let kind = kind_of(path);
    let made = archisim_mesh::read_file(path, Default::default())
        .ok()
        .and_then(|mesh| {
            let mut model = archisim_core::Model::new("Modelo de la biblioteca");
            model.add_level("Planta baja", 0.0);
            let name = path.file_stem()?.to_string_lossy().replace(['_', '-'], " ");
            let source = path.file_name()?.to_string_lossy().into_owned();
            let r = archisim_library::imported::import_mesh(&mut model, mesh, &name, &source, kind)
                .ok()?;
            Some(FolderModel {
                path: path.to_path_buf(),
                kind,
                model: std::sync::Arc::new(model),
                object_type: r.object_type,
            })
        });
    if let Ok(mut c) = model_cache().lock() {
        c.insert(path.to_path_buf(), (changed, made.clone()));
    }
    made
}

impl FolderObjects {
    fn read() -> Self {
        let lib = crate::editor::library::scan(&crate::editor::library::library_dir());
        let mut model = archisim_core::Model::new("Carpeta de la biblioteca");
        model.add_level("Planta baja", 0.0);
        let mut types = Vec::new();
        for (fi, f) in lib.files.iter().enumerate() {
            for (ti, t) in f.types.iter().enumerate() {
                let loaded = model.load_catalog_type(
                    &f.text,
                    t,
                    &f.materials,
                    archisim_core::OnConflict::Rename,
                );
                if let Ok(l) = loaded {
                    if let archisim_core::FamilyTypeRef::Object(id) = l.type_ref() {
                        types.push((fi, ti, id, t.guid.clone()));
                    }
                }
            }
        }
        let mut files = Vec::new();
        model_files(&lib.dir, 0, &mut files);
        let models = files.iter().filter_map(|p| folder_model(p)).collect();
        Self {
            read: std::time::Instant::now(),
            lib,
            model,
            types,
            models,
        }
    }
}

/// The gallery's categories: their name and the family categories in
/// them.
const CATEGORIES: [(&str, &[&str]); 6] = [
    ("Mobiliario", &["furniture", "kitchen"]),
    ("Sanitarios", &["sanitary"]),
    ("Entorno", &["site"]),
    ("Mobiliario urbano", &["street"]),
    ("Gente", &["people"]),
    ("Vehículos", &["vehicle"]),
];

/// The gallery category of a family category.
pub fn category_name(family_category: &str) -> &'static str {
    CATEGORIES
        .iter()
        .find(|(_, cs)| cs.contains(&family_category))
        .map_or("Mobiliario", |(n, _)| n)
}

/// Width by depth, to the centimetre.
fn size_note(lo: [f64; 2], hi: [f64; 2]) -> String {
    let cm = |v: f64| (v * 100.0).round() / 100.0;
    format!(
        "{} × {} mm",
        fmt_mm(cm(hi[0] - lo[0])),
        fmt_mm(cm(hi[1] - lo[1]))
    )
}

/// One type the gallery offers.
struct Item {
    pick: ObjectPick,
    name: String,
    note: String,
    category: &'static str,
    folder: String,
    thumb: Option<Thumb>,
}

/// Which category and folder the gallery shows (`None`: all).
#[derive(Debug, Clone, Default)]
struct Place {
    category: Option<&'static str>,
    folder: Option<String>,
}

impl ArchiSimApp {
    /// The model with the whole library, for the pictures of the types the
    /// project does not have yet.
    fn library_model(&mut self) -> &archisim_core::Model {
        self.object_library
            .get_or_insert_with(|| archisim_library::new_project("Biblioteca"))
    }

    fn object_items(&mut self, ctx: &egui::Context) -> Vec<Item> {
        let sig = self.picture_signature(false);
        let mut items = Vec::new();
        // The project's types.
        {
            let model = self.ed.model();
            let thumbs = &mut self.thumbs;
            for t in model.object_types() {
                let def = model.family(t.family).map(|f| f.def());
                let (category, folder) = def.map_or(("Mobiliario", String::new()), |d| {
                    (category_name(&d.category), d.folder.clone())
                });
                let size = model
                    .object_type_box(t.id)
                    .map(|(lo, hi)| size_note(lo, hi))
                    .unwrap_or_default();
                let family = def.map_or(String::new(), |d| d.name.clone());
                let key = format!("object:{family}:{:?}:{sig}", t.values);
                let thumb = thumbs.get(ctx, key, || crate::type_preview::object_model(model, t));
                items.push(Item {
                    pick: ObjectPick::Project(t.id),
                    name: t.name.clone(),
                    note: format!("{size} · {family}"),
                    category,
                    folder,
                    thumb,
                });
            }
        }
        // The library's, not in the project.
        let missing: Vec<&'static str> = archisim_library::OBJECT_CATALOGUE
            .iter()
            .map(|e| e.0)
            .filter(|n| !self.ed.model().object_types().any(|t| t.name == *n))
            .collect();
        if !missing.is_empty() {
            self.library_model();
            let lib = self.object_library.as_ref().expect("just made");
            let thumbs = &mut self.thumbs;
            for name in missing {
                let Some(t) = lib.object_types().find(|t| t.name == name) else {
                    continue;
                };
                let def = lib.family(t.family).map(|f| f.def());
                let (category, folder, family) = def
                    .map_or(("Mobiliario", String::new(), String::new()), |d| {
                        (category_name(&d.category), d.folder.clone(), d.name.clone())
                    });
                let size = lib
                    .object_type_box(t.id)
                    .map(|(lo, hi)| size_note(lo, hi))
                    .unwrap_or_default();
                let thumb = thumbs.get(ctx, format!("objlib:{name}"), || {
                    crate::type_preview::object_model(lib, t)
                });
                items.push(Item {
                    pick: ObjectPick::Library(name),
                    name: name.to_string(),
                    note: format!("{size} · {family} · biblioteca"),
                    category,
                    folder,
                    thumb,
                });
            }
        }
        // The plants ArchiSim grows, not in the project.
        {
            let have: std::collections::HashSet<String> = self
                .ed
                .model()
                .object_types()
                .map(|t| t.name.clone())
                .collect();
            let lib = self
                .flora_library
                .get_or_insert_with(
                    || std::sync::Arc::new(archisim_library::flora::flora_library()),
                )
                .clone();
            let thumbs = &mut self.thumbs;
            for e in archisim_library::flora::FLORA_CATALOGUE {
                if have.contains(e.name) {
                    continue;
                }
                let Some(t) = lib.object_types().find(|t| t.name == e.name) else {
                    continue;
                };
                let size = lib
                    .object_type_box(t.id)
                    .map(|(lo, hi)| size_note(lo, hi))
                    .unwrap_or_default();
                let thumb = thumbs.get(ctx, format!("flora:{}", e.name), || {
                    crate::type_preview::object_model(&lib, t)
                });
                items.push(Item {
                    pick: ObjectPick::Library(e.name),
                    name: e.name.to_string(),
                    note: format!("{size} · planta generada · {}", e.group),
                    category: "Entorno",
                    folder: e.group.to_string(),
                    thumb,
                });
            }
        }
        // The library folder's, not in the project (by their GlobalId).
        if self
            .folder_objects
            .as_ref()
            .is_none_or(|f| f.read.elapsed() > std::time::Duration::from_secs(30))
        {
            self.folder_objects = Some(FolderObjects::read());
        }
        {
            let project = self.ed.model();
            let have: std::collections::HashSet<String> =
                project.object_types().map(|t| t.guid.to_string()).collect();
            let names: std::collections::HashSet<&str> =
                items.iter().map(|i| i.name.as_str()).collect();
            let names: std::collections::HashSet<String> =
                names.into_iter().map(String::from).collect();
            let fo = self.folder_objects.as_ref().expect("just read");
            let thumbs = &mut self.thumbs;
            for (fi, ti, id, guid) in &fo.types {
                let Some(t) = fo.model.object_type(*id) else {
                    continue;
                };
                if have.contains(guid) || names.contains(&t.name) {
                    continue;
                }
                let def = fo.model.family(t.family).map(|f| f.def());
                let (category, folder, family) = def
                    .map_or(("Mobiliario", String::new(), String::new()), |d| {
                        (category_name(&d.category), d.folder.clone(), d.name.clone())
                    });
                let size = fo
                    .model
                    .object_type_box(t.id)
                    .map(|(lo, hi)| size_note(lo, hi))
                    .unwrap_or_default();
                let lib_model = &fo.model;
                let thumb = thumbs.get(ctx, format!("objdir:{guid}:{sig}"), || {
                    crate::type_preview::object_model(lib_model, t)
                });
                items.push(Item {
                    pick: ObjectPick::Folder(*fi, *ti),
                    name: t.name.clone(),
                    note: format!("{size} · {family} · biblioteca del estudio"),
                    category,
                    folder,
                    thumb,
                });
            }
        }
        // The folder's 3D models, not in the project (by their name).
        {
            let project = self.ed.model();
            let fo = self.folder_objects.as_ref().expect("just read");
            let thumbs = &mut self.thumbs;
            for (i, fm) in fo.models.iter().enumerate() {
                let Some(t) = fm.model.object_type(fm.object_type) else {
                    continue;
                };
                if project.object_types().any(|x| x.name == t.name) {
                    continue;
                }
                let def = fm.model.family(t.family).map(|f| f.def());
                let category = def.map_or("Mobiliario", |d| category_name(&d.category));
                let size = fm
                    .model
                    .object_type_box(t.id)
                    .map(|(lo, hi)| size_note(lo, hi))
                    .unwrap_or_default();
                let file = fm
                    .path
                    .file_name()
                    .map(|f| f.to_string_lossy().into_owned())
                    .unwrap_or_default();
                let m = &fm.model;
                let thumb = thumbs.get(ctx, format!("objmesh:{}:{sig}", fm.path.display()), || {
                    crate::type_preview::object_model(m, t)
                });
                items.push(Item {
                    pick: ObjectPick::FolderModel(i),
                    name: t.name.clone(),
                    note: format!("{size} · modelo 3D ({file}) · biblioteca del estudio"),
                    category,
                    folder: "Modelos 3D".into(),
                    thumb,
                });
            }
        }
        if let Some(scope) = self.object_scope.filter(|_| self.scoped_card) {
            items.retain(|i| scope.categories().contains(&i.category));
        }
        items
    }

    /// The gallery category of a project's object type.
    pub(super) fn object_type_category(&self, t: ObjectTypeId) -> Option<&'static str> {
        let m = self.ed.model();
        let ty = m.object_type(t)?;
        Some(category_name(&m.family(ty.family)?.def().category))
    }

    /// The object tool with part of the library: its first type, unless
    /// the one chosen is already of it.
    pub(super) fn objects_pick(&mut self, scope: ObjectScope) {
        self.object_scope = Some(scope);
        let fits = self
            .object_settings
            .object_type
            .and_then(|t| self.object_type_category(t))
            .is_some_and(|c| scope.categories().contains(&c));
        if !fits {
            let name = scope.first();
            let have = self
                .ed
                .model()
                .object_types()
                .find(|t| t.name == name)
                .map(|t| t.id);
            let t = match have {
                Some(t) => Some(t),
                None => self.object_pick_type(ObjectPick::Library(name)),
            };
            if t.is_some() {
                self.object_settings.object_type = t;
            }
        }
        self.set_tool(super::Tool::Object);
    }

    /// The object type card: the picture of the current type, its
    /// category and folder, and a button opening the gallery. Returns
    /// what is chosen in it.
    pub(super) fn object_card(
        &mut self,
        ui: &mut Ui,
        salt: &str,
        current: Option<ObjectTypeId>,
    ) -> Option<ObjectPick> {
        let ctx = ui.ctx().clone();
        let items = self.object_items(&ctx);
        let p = Palette::current(&ctx);
        let cur = items
            .iter()
            .find(|i| current.is_some_and(|c| i.pick == ObjectPick::Project(c)));
        let mut picked = None;
        let frame = egui::Frame::new()
            .fill(p.raised)
            .stroke(egui::Stroke::new(1.0, p.line))
            .corner_radius(egui::CornerRadius::same(10))
            .inner_margin(egui::Margin::same(10));
        frame.show(ui, |ui| {
            ui.set_width(ui.available_width());
            if let Some(t) = cur.and_then(|c| c.thumb) {
                props::paper(ui, t, Vec2::new(ui.available_width(), 130.0));
                ui.label(
                    egui::RichText::new("Planta · el frente, hacia arriba")
                        .small()
                        .color(p.muted),
                );
                ui.add_space(6.0);
            }
            if let Some(c) = cur {
                let path = if c.folder.is_empty() {
                    c.category.to_string()
                } else {
                    format!("{} › {}", c.category, c.folder)
                };
                ui.label(egui::RichText::new(path).color(p.muted));
            }
            let shown = cur.map_or("Elegir objeto…".to_string(), |c| c.name.clone());
            let button = ui.add_sized(
                [ui.available_width(), 30.0],
                egui::Button::new(egui::RichText::new(shown).strong()),
            );
            let button = button.on_hover_text("Elegir otro objeto de la galería");
            if let Some(c) = cur {
                ui.label(egui::RichText::new(&c.note).small().color(p.muted));
            }
            picked = self.object_gallery(ui, &button, salt, current, &items);
        });
        picked
    }

    /// The gallery under the button: folders on the left, pictures on the
    /// right, a search box over them.
    fn object_gallery(
        &mut self,
        ui: &mut Ui,
        button: &egui::Response,
        salt: &str,
        current: Option<ObjectTypeId>,
        items: &[Item],
    ) -> Option<ObjectPick> {
        let p = Palette::current(ui.ctx());
        let place_id = Id::unique(("object-gallery-place", salt));
        let search_id = Id::unique(("object-gallery-search", salt));
        let mut picked = None;
        egui::Popup::menu(button)
            .width(640.0)
            .close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside)
            .show(|ui| {
                // It opens where the current type is (a tree among the
                // trees), or at the first category of the tool's part of
                // the library.
                let here = items
                    .iter()
                    .find(|i| current.is_some_and(|c| i.pick == ObjectPick::Project(c)));
                let opening = match here {
                    Some(i) => Place {
                        category: Some(i.category),
                        folder: (!i.folder.is_empty()).then(|| i.folder.clone()),
                    },
                    None => Place {
                        category: self
                            .object_scope
                            .filter(|_| self.scoped_card)
                            .map(|s| s.categories()[0]),
                        folder: None,
                    },
                };
                let mut place: Place = ui.ctx().data(|d| d.get_temp(place_id)).unwrap_or(opening);
                let mut query: String =
                    ui.ctx().data(|d| d.get_temp(search_id)).unwrap_or_default();
                ui.add(
                    egui::TextEdit::singleline(&mut query)
                        .hint_text("Buscar objeto…")
                        .desired_width(f32::INFINITY),
                );
                ui.add_space(4.0);
                let q = query.to_lowercase();
                ui.horizontal_top(|ui| {
                    // Folders.
                    ui.allocate_ui_with_layout(
                        Vec2::new(170.0, 440.0),
                        egui::Layout::top_down(egui::Align::Min),
                        |ui| {
                            ui.set_width(170.0);
                            let all = place.category.is_none();
                            if ui
                                .selectable_label(all, format!("Todo ({})", items.len()))
                                .clicked()
                            {
                                place = Place::default();
                            }
                            for (cat, _) in CATEGORIES {
                                let n = items.iter().filter(|i| i.category == cat).count();
                                if n == 0 {
                                    continue;
                                }
                                let on = place.category == Some(cat);
                                let text = egui::RichText::new(format!("{cat} ({n})")).strong();
                                if ui
                                    .selectable_label(on && place.folder.is_none(), text)
                                    .clicked()
                                {
                                    place = Place {
                                        category: Some(cat),
                                        folder: None,
                                    };
                                }
                                if on {
                                    let mut folders: Vec<&str> = items
                                        .iter()
                                        .filter(|i| i.category == cat && !i.folder.is_empty())
                                        .map(|i| i.folder.as_str())
                                        .collect();
                                    folders.sort_unstable();
                                    folders.dedup();
                                    for f in folders {
                                        let n = items
                                            .iter()
                                            .filter(|i| i.category == cat && i.folder == f)
                                            .count();
                                        ui.horizontal(|ui| {
                                            ui.add_space(14.0);
                                            let sel = place.folder.as_deref() == Some(f);
                                            if ui
                                                .selectable_label(sel, format!("{f} ({n})"))
                                                .clicked()
                                            {
                                                place.folder = Some(f.to_string());
                                            }
                                        });
                                    }
                                }
                            }
                        },
                    );
                    ui.separator();
                    // Pictures.
                    let shown: Vec<&Item> = items
                        .iter()
                        .filter(|i| place.category.is_none_or(|c| i.category == c))
                        .filter(|i| place.folder.as_ref().is_none_or(|f| &i.folder == f))
                        .filter(|i| {
                            q.is_empty()
                                || i.name.to_lowercase().contains(&q)
                                || i.folder.to_lowercase().contains(&q)
                                || i.note.to_lowercase().contains(&q)
                        })
                        .collect();
                    ui.vertical(|ui| {
                        if shown.is_empty() {
                            ui.label(
                                egui::RichText::new("Ningún objeto coincide")
                                    .small()
                                    .color(p.muted),
                            );
                        }
                        egui::ScrollArea::vertical()
                            .max_height(440.0)
                            .show(ui, |ui| {
                                let w = (ui.available_width() - 16.0) / 3.0;
                                for row in shown.chunks(3) {
                                    ui.horizontal(|ui| {
                                        ui.spacing_mut().item_spacing.x = 8.0;
                                        for item in row {
                                            let tile = TypeItem {
                                                id: (),
                                                name: item.name.clone(),
                                                note: item.note.clone(),
                                                thumb: item.thumb,
                                                caption: "",
                                            };
                                            let selected = current.is_some_and(|c| {
                                                item.pick == ObjectPick::Project(c)
                                            });
                                            if props::tile(ui, w, &tile, selected).clicked() {
                                                picked = Some(item.pick);
                                            }
                                        }
                                    });
                                    ui.add_space(2.0);
                                }
                            });
                    });
                });
                // Keep the place between openings; forget the search.
                ui.ctx().data_mut(|d| {
                    d.insert_temp(place_id, place);
                    d.insert_temp(search_id, query);
                });
                if picked.is_some() {
                    ui.ctx().data_mut(|d| d.remove::<String>(search_id));
                    ui.close();
                }
            });
        picked
    }

    /// The project's type for what was chosen in the gallery (a library
    /// type is loaded into the project).
    pub(super) fn object_pick_type(&mut self, pick: ObjectPick) -> Option<ObjectTypeId> {
        match pick {
            ObjectPick::Project(t) => Some(t),
            ObjectPick::Folder(fi, ti) => {
                let file = self.folder_objects.as_ref()?.lib.files.get(fi)?.clone();
                let name = file
                    .types
                    .get(ti)
                    .map(|t| t.name.clone())
                    .unwrap_or_default();
                match self
                    .ed
                    .load_library_type(&file, ti, archisim_core::OnConflict::Rename)
                {
                    Ok(l) => {
                        self.say(format!("«{name}» cargado de la biblioteca en el proyecto."));
                        match l.type_ref() {
                            archisim_core::FamilyTypeRef::Object(t) => Some(t),
                            _ => None,
                        }
                    }
                    Err(e) => {
                        self.complain(format!("No se puede cargar «{name}»: {e}."));
                        None
                    }
                }
            }
            ObjectPick::FolderModel(i) => {
                let fm = self.folder_objects.as_ref()?.models.get(i)?.clone();
                let name = fm
                    .model
                    .object_type(fm.object_type)
                    .map(|t| t.name.clone())
                    .unwrap_or_default();
                let source = fm
                    .path
                    .file_name()
                    .map(|f| f.to_string_lossy().into_owned())
                    .unwrap_or_default();
                let mesh = match archisim_mesh::read_file(&fm.path, Default::default()) {
                    Ok(m) => m,
                    Err(e) => {
                        self.complain(format!("No se puede leer «{source}»: {e}."));
                        return None;
                    }
                };
                match self.ed.import_mesh(mesh, &name, &source, fm.kind) {
                    Ok(r) => {
                        self.say(format!(
                            "«{name}» (modelo 3D) cargado de la biblioteca en el proyecto."
                        ));
                        Some(r.object_type)
                    }
                    Err(e) => {
                        self.complain(format!("No se puede cargar «{name}»: {e}."));
                        None
                    }
                }
            }
            ObjectPick::Library(name) => match self.ed.library_object_type(name) {
                Ok(t) => {
                    self.say(format!("«{name}» cargado de la biblioteca en el proyecto."));
                    Some(t)
                }
                Err(e) => {
                    self.complain(format!("No se puede cargar «{name}»: {e}."));
                    None
                }
            },
        }
    }
}
