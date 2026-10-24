//! Collections in the Maqueta (revisión de Antonio del 23/10/2026): a
//! style for the whole house — nórdico cálido, mediterráneo, años 50,
//! japandi — chosen above the catalog. The catalog then shows every piece
//! in its colours and places it so; «Aplicar» puts what is selected in it.
//! The rules (what each material becomes) are in
//! `archisim_library::collections`.

use archisim_library::collections::{self, Collection, COLLECTIONS};
use eframe::egui::{self, Color32, CornerRadius, Rect, Sense, Ui, Vec2};

use super::ArchiSimApp;
use crate::pick::Element;

/// A piece of the catalog in the collection chosen.
pub(super) struct CollectedItem {
    /// Its name in the library (without the collection).
    pub name: &'static str,
    pub folder: String,
    pub category: &'static str,
    pub thumb: Option<super::thumbs::Thumb>,
}

/// The colour of a material of the palette.
fn swatch(name: &str) -> Color32 {
    archisim_library::COZY_PALETTE
        .iter()
        .find(|e| e.0 == name)
        .map_or(Color32::GRAY, |e| Color32::from_rgb(e.2[0], e.2[1], e.2[2]))
}

/// A collection's colours, for the cards.
pub(super) fn colours(c: &Collection) -> Vec<Color32> {
    c.swatches().iter().map(|n| swatch(n)).collect()
}

/// A strip with a collection's colours.
fn strip(ui: &mut Ui, c: &Collection) {
    let (rect, _) = ui.allocate_exact_size(Vec2::new(54.0, 12.0), Sense::hover());
    let s = c.swatches();
    let w = rect.width() / s.len() as f32;
    for (i, name) in s.iter().enumerate() {
        let r = Rect::from_min_size(
            rect.min + Vec2::new(i as f32 * w, 0.0),
            Vec2::new(w, rect.height()),
        );
        ui.painter()
            .rect_filled(r.shrink(0.5), CornerRadius::same(2), swatch(name));
    }
}

impl ArchiSimApp {
    /// The chooser of the collection, and «Aplicar» for what is selected.
    pub(super) fn collection_chooser(&mut self, ui: &mut Ui) {
        let now = self.collection;
        let shown = now.map_or("Sin colección", |c| c.name);
        egui::ComboBox::from_id_salt("maquette-collection")
            .selected_text(format!("Colección: {shown}"))
            .width(190.0)
            .show_ui(ui, |ui| {
                if ui
                    .selectable_label(now.is_none(), "Sin colección")
                    .on_hover_text("Las piezas de la biblioteca con sus colores de siempre")
                    .clicked()
                {
                    self.collection = None;
                }
                for c in COLLECTIONS {
                    let on = now.is_some_and(|n| n.name == c.name);
                    let r = ui
                        .horizontal(|ui| {
                            strip(ui, c);
                            ui.selectable_label(on, c.name)
                        })
                        .inner
                        .on_hover_text(c.hint);
                    if r.clicked() {
                        self.collection = Some(c);
                    }
                }
            });
        let objects = self.selected_objects();
        let hint = match self.collection {
            Some(c) => format!(
                "Pasa lo seleccionado a la colección {}: sus maderas, telas, metales y cerámicas",
                c.name
            ),
            None => "Devuelve lo seleccionado a sus colores de la biblioteca".to_string(),
        };
        if ui
            .add_enabled(!objects.is_empty(), egui::Button::new("Aplicar"))
            .on_hover_text(hint)
            .on_disabled_hover_text("Selecciona muebles (o una habitación entera con un recuadro) para pasarlos a la colección")
            .clicked()
        {
            self.apply_collection_to_selection();
        }
    }

    /// The objects selected.
    fn selected_objects(&self) -> Vec<archisim_core::ObjectId> {
        self.ed
            .selection
            .iter()
            .filter_map(|e| match e {
                Element::Object(id) => Some(*id),
                _ => None,
            })
            .collect()
    }

    /// Puts what is selected in the collection chosen.
    pub(super) fn apply_collection_to_selection(&mut self) {
        let objects = self.selected_objects();
        let c = self.collection;
        match self.ed.apply_collection(&objects, c) {
            Ok(0) => self.say("Ya estaban así."),
            Ok(n) => {
                let what = if n == 1 { "pieza" } else { "piezas" };
                match c {
                    Some(c) => self.say(format!("{n} {what} en la colección {}.", c.name)),
                    None => self.say(format!("{n} {what} con sus colores de la biblioteca.")),
                }
            }
            Err(e) => self.complain(format!("No se puede aplicar la colección: {e}.")),
        }
    }

    /// The library in the collection chosen.
    fn collection_library(&mut self, c: &'static Collection) -> &archisim_core::Model {
        if let Some(i) = self
            .collection_libraries
            .iter()
            .position(|(n, _)| *n == c.name)
        {
            return &self.collection_libraries[i].1;
        }
        self.collection_libraries
            .push((c.name, collections::collection_library(c)));
        &self.collection_libraries.last().expect("just pushed").1
    }

    /// Every piece of the library in the collection chosen, with its
    /// picture in its colours.
    pub(super) fn collected_items(
        &mut self,
        ctx: &egui::Context,
        c: &'static Collection,
    ) -> Vec<CollectedItem> {
        self.collection_library(c);
        let lib = &self
            .collection_libraries
            .iter()
            .find(|(n, _)| *n == c.name)
            .expect("just made")
            .1;
        let thumbs = &mut self.thumbs;
        let mut out = Vec::new();
        for (name, _, _) in archisim_library::OBJECT_CATALOGUE {
            let variant = collections::variant_name(name, c);
            let Some(t) = lib.object_types().find(|t| t.name == variant) else {
                continue;
            };
            let def = lib.family(t.family).map(|f| f.def());
            let (category, folder) = def.map_or(("Mobiliario", String::new()), |d| {
                (
                    super::object_gallery::category_name(&d.category),
                    d.folder.clone(),
                )
            });
            let thumb = thumbs.get(ctx, format!("objcol:{}:{name}", c.name), || {
                crate::type_preview::object_model(lib, t)
            });
            out.push(CollectedItem {
                name,
                folder,
                category,
                thumb,
            });
        }
        out
    }

    /// Takes the object tool with a piece of the collection.
    pub(super) fn take_collected(&mut self, name: &'static str) {
        let Some(c) = self.collection else {
            return;
        };
        match self.ed.collection_object_type(name, c) {
            Ok(t) => {
                self.object_scope = None;
                self.object_settings.object_type = Some(t);
                self.set_tool(super::Tool::Object);
            }
            Err(e) => self.complain(format!("No se puede cargar «{name}»: {e}.")),
        }
    }
}
