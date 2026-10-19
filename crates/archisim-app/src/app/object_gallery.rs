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

/// What is chosen in the gallery.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ObjectPick {
    Project(ObjectTypeId),
    /// A type of the library, by its name.
    Library(&'static str),
}

/// The gallery's categories: their name and the family categories in
/// them.
const CATEGORIES: [(&str, &[&str]); 5] = [
    ("Mobiliario", &["furniture", "kitchen"]),
    ("Sanitarios", &["sanitary"]),
    ("Entorno", &["site"]),
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
        items
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
                let mut place: Place = ui.ctx().data(|d| d.get_temp(place_id)).unwrap_or_default();
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
