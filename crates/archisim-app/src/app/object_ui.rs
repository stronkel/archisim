//! The object tool ("Objeto"): furniture, sanitary ware and kitchen units
//! from the project's object types, placed with a click (back side on the
//! point, Space turns it 90°); their properties and type sizes.

use super::widgets::SwitchExt as _;
use archisim_core::{ObjectId, ObjectTypeId, ParamKind, ParamScope, Value};
use eframe::egui::{self, Id, Ui};

use super::icons::Icon;
use super::props::{self, group, row, text_row};
use super::structure_ui::angle_field;
use super::*;

impl ArchiSimApp {
    /// The tool's type, or the first one of the project.
    fn tool_object_type(&mut self) -> Option<ObjectTypeId> {
        let model = self.ed.model();
        let valid = self
            .object_settings
            .object_type
            .filter(|t| model.object_type(*t).is_some());
        let t = valid.or_else(|| {
            model
                .object_types()
                .find(|t| !model.is_railing_type(t.id))
                .map(|t| t.id)
        });
        self.object_settings.object_type = t;
        t
    }

    pub(super) fn object_click(&mut self, p: Point2) {
        if self.tool_object_type().is_none() {
            self.complain("El proyecto no tiene tipos de objeto.");
            return;
        }
        // Small things rest on the table or shelf under the cursor.
        // (and, in the Maqueta, furniture goes against the wall near it).
        let mut s = self.object_settings;
        let (at, rotation, base) = self.placement_spot(p);
        let surface = base - s.base_offset;
        s.rotation = rotation;
        s.base_offset = base;
        let before = self.ed.model().objects().count();
        match self.ed.add_object(&s, at) {
            Ok(id) => {
                self.ed.selection = [Element::Object(id)].into_iter().collect();
                if self.ed.model().objects().count() == before {
                    self.say("Unido al de al lado: ahora son una sola pieza.");
                } else if surface > 0.0 {
                    self.say(format!(
                        "Colocado encima, a {}.",
                        crate::units::fmt_len(surface)
                    ));
                } else {
                    self.say(
                        "Objeto colocado. Espacio: girarlo 90° antes de colocar el siguiente.",
                    );
                }
            }
            Err(e) => self.complain(format!("No se puede colocar: {e}.")),
        }
    }

    pub(super) fn draw_object_preview(&self, painter: &egui::Painter, rect: Rect) {
        let (Tool::Object, Some(c), Some(t)) =
            (self.tool, self.cursor, self.object_settings.object_type)
        else {
            return;
        };
        let Some((lo, hi)) = self.ed.model().object_type_box(t) else {
            return;
        };
        let (cs, sn) = (
            self.object_settings.rotation.cos(),
            self.object_settings.rotation.sin(),
        );
        let at =
            |x: f64, y: f64| Point2::new(c.point.x + x * cs - y * sn, c.point.y + x * sn + y * cs);
        let poly = [
            at(lo[0], lo[1]),
            at(hi[0], lo[1]),
            at(hi[0], hi[1]),
            at(lo[0], hi[1]),
        ];
        painter.add(Shape::closed_line(
            self.polygon(rect, &poly),
            Stroke::new(1.5, ACCENT),
        ));
        // The front: where the object faces.
        let mid = at((lo[0] + hi[0]) / 2.0, hi[1]);
        let back = at((lo[0] + hi[0]) / 2.0, lo[1]);
        painter.arrow(
            self.view.to_screen(rect, back),
            self.view.to_screen(rect, mid) - self.view.to_screen(rect, back),
            Stroke::new(1.0, ACCENT),
        );
    }

    pub(super) fn object_tool_properties(&mut self, ui: &mut Ui) {
        let (title, salt) = match self.object_scope {
            Some(s) => (
                format!("Herramienta {} · lo que vas a colocar", s.title()),
                format!("tool-object-{s:?}"),
            ),
            None => (
                "Herramienta Objeto · lo que vas a colocar".to_string(),
                "tool-object".to_string(),
            ),
        };
        props::title(ui, &title);
        let current = self.tool_object_type();
        self.scoped_card = true;
        let card = self.object_card(ui, &salt, current);
        self.scoped_card = false;
        if let Some(pick) = card {
            if let Some(t) = self.object_pick_type(pick) {
                self.object_settings.object_type = Some(t);
            }
        }
        ui.add_space(6.0);
        group(ui, "tool-object", "Objeto", |ui| {
            row(ui, "Giro (°)", "Espacio: +90°", |ui, w| {
                if let Some(a) = angle_field(
                    ui,
                    Id::unique("tool-object-rot"),
                    self.object_settings.rotation,
                    w - 8.0,
                ) {
                    self.object_settings.rotation = a;
                }
            });
            row(
                ui,
                "Desfase base (mm)",
                "Altura sobre la planta",
                |ui, w| {
                    if let Some(v) = length_field(
                        ui,
                        Id::unique("tool-object-base"),
                        self.object_settings.base_offset,
                        w - 8.0,
                    ) {
                        self.object_settings.base_offset = v;
                    }
                },
            );
        });
        ui.add_space(6.0);
        props::note(
            ui,
            "Clic: colocar el objeto con su lado de atrás en el punto (contra una cara de muro, con los forzados); árboles y personas, con su centro. La flecha marca hacia dónde mira. Espacio: girar 90°. Árboles, personas y vehículos se apoyan en el terreno que haya debajo.",
        );
    }

    pub(super) fn object_properties(&mut self, ui: &mut Ui, id: ObjectId) {
        let model = self.ed.model();
        let Some(o) = model.object(id).cloned() else {
            return;
        };
        let Some(t) = model.object_type(o.object_type).cloned() else {
            return;
        };
        let family = model.family(t.family).map(|f| f.def().clone());
        let users = model.object_type_users(t.id);
        let ifc = model
            .evaluate_object(id)
            .ok()
            .and_then(|ev| {
                ev.outputs
                    .get("ifc_class")
                    .and_then(|v| v.as_text().map(String::from))
            })
            .unwrap_or_default();
        props::title(ui, "Objeto · 1 seleccionado");
        super::properties::view_card(ui, Icon::Object, &t.name, &ifc);
        // A tree offers trees, a bench street furniture: the gallery of an
        // object of Urbanismo keeps to its part of the library.
        let scope = self
            .object_type_category(t.id)
            .and_then(super::object_gallery::ObjectScope::of_category);
        let (saved, saved_card) = (self.object_scope, self.scoped_card);
        if scope.is_some() {
            self.object_scope = scope;
            self.scoped_card = true;
        }
        let card = self.object_card(ui, &format!("object-{}", id.0), Some(t.id));
        (self.object_scope, self.scoped_card) = (saved, saved_card);
        if let Some(pick) = card {
            if let Some(picked) = self.object_pick_type(pick).filter(|p| *p != t.id) {
                let r = self
                    .ed
                    .edit_object("Cambiar tipo de objeto", id, |o| o.object_type = picked);
                self.report(r, "Tipo cambiado.");
            }
        }
        ui.add_space(6.0);
        if self.ed.model().type_grows(t.id) {
            self.plant_group(ui, id, &o);
        }
        if family
            .as_ref()
            .is_some_and(|f| f.name == "Armario empotrado")
        {
            self.built_in_group(ui, id);
        }
        group(ui, "object-data", "Objeto", |ui| {
            row(ui, "Giro (°)", "Espacio: +90°", |ui, w| {
                if let Some(a) =
                    angle_field(ui, Id::unique(("object-rot", id.0)), o.rotation, w - 8.0)
                {
                    let r = self.ed.edit_object("Girar objeto", id, |o| o.rotation = a);
                    self.report(r, "Objeto girado.");
                }
            });
            row(
                ui,
                "Desfase base (mm)",
                "Altura sobre la planta",
                |ui, w| {
                    if let Some(v) = length_field(
                        ui,
                        Id::unique(("object-base", id.0)),
                        o.base_offset,
                        w - 8.0,
                    ) {
                        let r = self
                            .ed
                            .edit_object("Desfase de objeto", id, |o| o.base_offset = v);
                        self.report(r, "Desfase cambiado.");
                    }
                },
            );
        });
        if let Some(def) = family {
            let materials: Vec<(u64, String)> = self
                .ed
                .model()
                .materials()
                .map(|m| (m.id.0, m.name.clone()))
                .collect();
            // This object's own choices (how a tree's canopy is drawn…).
            let own = self.ed.model().object_values(id);
            if def.params_of(ParamScope::Instance).next().is_some() {
                group(ui, "object-own-values", "Este objeto", |ui| {
                    props::note(ui, "Solo cambia este objeto, no su tipo.");
                    for p in def.params_of(ParamScope::Instance) {
                        if !def.is_visible(&p.name, &own) {
                            continue;
                        }
                        let Some(cur) = own.get(&p.name) else {
                            continue;
                        };
                        let salt = format!("object-iv-{}-{}", id.0, p.name);
                        let mark = self.ed.undo_mark();
                        let v = match (&p.kind, cur) {
                            (ParamKind::Material, Value::Material(mid)) => {
                                let label = p.label.clone();
                                self.material_param_row(
                                    ui,
                                    &salt,
                                    &label,
                                    &p.name,
                                    archisim_core::MaterialId(*mid),
                                )
                                .map(Value::Material)
                            }
                            // A choice: its options as cards, drawn in plan
                            // as this object would be (E2).
                            (ParamKind::Choice(options), Value::Text(now)) => {
                                let ctx = ui.ctx().clone();
                                let label = if p.label.is_empty() {
                                    p.name.clone()
                                } else {
                                    p.label.clone()
                                };
                                let mut out = None;
                                super::cards::card_row(ui, &label, |ui, w| {
                                    let mut thumb = |opt: &str| {
                                        self.object_instance_option_thumb(
                                            &ctx, &t, &own, &p.name, opt,
                                        )
                                    };
                                    out = super::cards::option_cards(
                                        ui,
                                        w - 8.0,
                                        options,
                                        now,
                                        &mut thumb,
                                    );
                                });
                                out.filter(|x| x != now).map(Value::Text)
                            }
                            _ => param_row(ui, &salt, p, cur, &materials),
                        };
                        if let Some(v) = v {
                            let mut values = o.values.clone();
                            values.insert(p.name.clone(), v);
                            let r = self.ed.set_object_values(id, values);
                            self.ed.join_since(mark, "Objeto cambiado");
                            self.report(r, "Objeto cambiado.");
                        }
                    }
                });
            }
            let title = format!("Tipo · {}", def.name);
            group(ui, "object-type-values", &title, |ui| {
                props::note(
                    ui,
                    &if users == 1 {
                        "Lo que cambies aquí cambia el tipo (solo lo usa este objeto).".to_string()
                    } else {
                        format!("Lo que cambies aquí cambia el tipo: sus {users} objetos.")
                    },
                );
                for p in def.params_of(ParamScope::Type) {
                    if !def.is_visible(&p.name, &t.values) {
                        continue;
                    }
                    let Some(cur) = t.values.get(&p.name) else {
                        continue;
                    };
                    let salt = format!("object-tv-{}-{}", t.id.0, p.name);
                    let mark = self.ed.undo_mark();
                    if let Some(v) = self.object_type_param_row(ui, &salt, p, cur, &t, &materials) {
                        let r = self.ed.set_object_type_value(t.id, &p.name, v);
                        self.ed.join_since(mark, "Tipo de objeto cambiado");
                        self.report(r, "Tipo de objeto cambiado.");
                    }
                }
            });
        }
        if users > 1 && ui.button("Tipo propio para este objeto").clicked() {
            match self.ed.object_own_type(id) {
                Ok(_) => self.say("Tipo duplicado: sus medidas ya solo cambian este objeto."),
                Err(e) => self.complain(format!("No se puede duplicar: {e}.")),
            }
        }
        if let Some((mesh, k, shown)) = self.ed.model().object_mesh(id) {
            let levels = mesh
                .levels
                .iter()
                .map(|l| l.triangle_count().to_string())
                .collect::<Vec<_>>()
                .join(" / ");
            let source = mesh.source.clone();
            let showing = shown.triangle_count();
            group(ui, "object-mesh", "Modelo 3D", |ui| {
                props::text_row(ui, "Fichero", "De donde se importó", &source);
                props::text_row(
                    ui,
                    "Caras",
                    "Del modelo y de sus versiones más ligeras, para verlo de lejos",
                    &levels,
                );
                props::text_row(
                    ui,
                    "En las vistas",
                    "Con muchos modelos en el proyecto, todos se ven con una versión más ligera",
                    &showing.to_string(),
                );
                props::text_row(
                    ui,
                    "Escala",
                    "La altura del tipo sobre la del modelo",
                    &format!("{k:.3}").replace('.', ","),
                );
            });
        }
        self.symbol_group(ui, o.object_type);
        self.elevation_group(ui, o.object_type);
        group(ui, "object-identity", "Datos de identidad", |ui| {
            props::id_row(ui, "GlobalId", &o.guid.to_string());
        });
    }

    /// The plan symbol of an object's type (bloque 6): brought from a DXF,
    /// stretched to the type's size or at its own, or taken away.
    fn symbol_group(&mut self, ui: &mut Ui, t: ObjectTypeId) {
        let Some(ty) = self.ed.model().object_type(t) else {
            return;
        };
        let symbol = ty.symbol.clone();
        let users = self.ed.model().object_type_users(t);
        let mut import = false;
        let mut fit_to = None;
        let mut remove = false;
        let mut from_library = false;
        let has_library = super::symbol_pick::has_library_files();
        group(ui, "object-symbol", "Símbolo de planta", |ui| {
            match &symbol {
                None => props::note(
                    ui,
                    "El de la familia. Puedes poner uno tuyo de un DXF (un árbol, una persona, un mueble dibujados en CAD): lo dibujarán todos los objetos del tipo.",
                ),
                Some(s) => {
                    props::text_row(ui, "Fichero", "De donde se importó", &s.source);
                    props::text_row(ui, "Piezas", "", &s.items.len().to_string());
                    let mut fit = s.fit;
                    props::row(
                        ui,
                        "Tamaño",
                        "Estirado: tan grande como el objeto (cambia con él); propio: a la medida del dibujo",
                        |ui, w| {
                            fit = props::choice(
                                ui,
                                ("symbol-fit", t.0),
                                w,
                                s.fit,
                                &[(true, "Del objeto"), (false, "Del dibujo")],
                            );
                        },
                    );
                    if fit != s.fit {
                        fit_to = Some(fit);
                    }
                }
            }
            ui.horizontal(|ui| {
                if ui
                    .button(if symbol.is_some() {
                        "Cambiar…"
                    } else {
                        "Importar DXF…"
                    })
                    .on_hover_text(if users > 1 {
                        format!("Lo dibujarán los {users} objetos de este tipo")
                    } else {
                        "Lo dibujará este objeto (es el único de su tipo)".to_string()
                    })
                    .clicked()
                {
                    import = true;
                }
                if has_library
                    && ui
                        .button("De mi biblioteca…")
                        .on_hover_text(format!(
                            "Elegir entre los dibujos de los DXF de la carpeta «{}» de tu biblioteca, viéndolos",
                            super::symbol_pick::FOLDER
                        ))
                        .clicked()
                {
                    from_library = true;
                }
                if symbol.is_some() && ui.button("Quitar").clicked() {
                    remove = true;
                }
            });
        });
        if from_library {
            self.pick_from_library(t);
        }
        // A picture for presentation plans.
        let picture = self
            .ed
            .model()
            .object_type(t)
            .and_then(|x| x.plan_image.as_ref().map(|i| i.source.clone()));
        let mut picture_add = false;
        let mut picture_remove = false;
        ui.horizontal(|ui| {
            ui.add_space(4.0);
            ui.label(egui::RichText::new("Imagen en planta de presentación").strong());
        });
        ui.horizontal(|ui| {
            ui.add_space(4.0);
            match &picture {
                Some(src) => {
                    ui.label(src);
                    if ui.small_button("Quitar").clicked() {
                        picture_remove = true;
                    }
                }
                None => {
                    if ui
                        .button("PNG…")
                        .on_hover_text("Una imagen vista desde arriba (con fondo transparente), dibujada en las plantas de presentación en lugar del símbolo")
                        .clicked()
                    {
                        picture_add = true;
                    }
                }
            }
        });
        if picture_add {
            let mut d = rfd::FileDialog::new()
                .set_title("Imagen para plantas de presentación (PNG)")
                .add_filter("Imagen PNG", &["png"]);
            if let Some(dir) = self.start_dir() {
                d = d.set_directory(dir);
            }
            if let Some(path) = d.pick_file() {
                self.set_plan_image_from(t, &path);
            }
        }
        if picture_remove {
            let r = self.ed.transact("Quitar imagen en planta", |m| {
                m.set_object_type_plan_image(t, None)
            });
            self.report(r, "Imagen quitada: vuelve el símbolo.");
        }
        if import {
            self.pick_symbol(t);
        }
        if remove {
            let r = self
                .ed
                .transact("Quitar símbolo", |m| m.set_object_type_symbol(t, None));
            self.report(r, "Símbolo quitado: vuelve el de la familia.");
        }
        if let (Some(fit), Some(s)) = (fit_to, symbol) {
            // Between stretched and real size, the drawing keeps its look:
            // scaled by the type's size (or back).
            let size = self
                .ed
                .model()
                .object_type_box(t)
                .map_or(1.0, |(lo, hi)| (hi[0] - lo[0]).max(hi[1] - lo[1]).max(0.01));
            let k = if fit { 1.0 / size } else { size };
            let origin = Point2::new(0.0, 0.0);
            let tr = archisim_draft::Transform::scale(origin, k);
            let scaled = |x: &archisim_core::PlanSymbol| archisim_core::PlanSymbol {
                fit,
                items: x
                    .items
                    .iter()
                    .map(|i| archisim_core::SymbolItem {
                        shape: archisim_draft::transform_shape(&i.shape, &tr),
                        ..i.clone()
                    })
                    .collect(),
                ..x.clone()
            };
            // Every drawing of the type (when it has several).
            let more = self
                .ed
                .model()
                .object_type(t)
                .map(|x| x.more_symbols.clone())
                .unwrap_or_default();
            let all: Vec<archisim_core::PlanSymbol> =
                std::iter::once(&s).chain(more.iter()).map(scaled).collect();
            let r = self.ed.transact("Tamaño del símbolo", |m| {
                m.set_object_type_symbols(t, all)
            });
            self.report(r, "Tamaño del símbolo cambiado.");
        }
    }

    /// The type's own drawings for elevations, sections and axonometries:
    /// a DXF (technical) or a PNG (presentation), from the front or the
    /// side.
    fn elevation_group(&mut self, ui: &mut Ui, t: ObjectTypeId) {
        let Some(ty) = self.ed.model().object_type(t) else {
            return;
        };
        let figures = ty.elevation.clone();
        let users = self.ed.model().object_type_users(t);
        let mut add: Option<(bool, bool)> = None;
        let mut remove: Option<usize> = None;
        let mut from_library: Option<bool> = None;
        group(ui, "object-elevation", "Dibujo en alzado", |ui| {
            if figures.is_empty() {
                props::note(
                    ui,
                    "El que hace ArchiSim. Puedes poner el tuyo: un DXF (para las vistas técnicas) o una imagen PNG con fondo transparente (para las de presentación), visto de frente o de lado. Se dibuja de pie, tan alto como el objeto, mirando a quien mira, en alzados, secciones y axonometrías.",
                );
            }
            for (i, f) in figures.iter().enumerate() {
                let what = match (&f.art, f.side) {
                    (archisim_core::ElevationArt::Lines(_), false) => "DXF de frente",
                    (archisim_core::ElevationArt::Lines(_), true) => "DXF de lado",
                    (archisim_core::ElevationArt::Image { .. }, false) => "Imagen de frente",
                    (archisim_core::ElevationArt::Image { .. }, true) => "Imagen de lado",
                };
                ui.horizontal(|ui| {
                    ui.add_space(4.0);
                    ui.label(egui::RichText::new(what).strong());
                    ui.label(&f.source);
                    if ui.small_button("Quitar").clicked() {
                        remove = Some(i);
                    }
                });
            }
            let hint = if users > 1 {
                format!("Lo dibujarán los {users} objetos de este tipo")
            } else {
                "Lo dibujará este objeto (es el único de su tipo)".to_string()
            };
            let library = super::symbol_pick::has_library_files();
            ui.horizontal_wrapped(|ui| {
                ui.add_space(4.0);
                for (label, side) in [("De mi biblioteca, de frente…", false), ("De mi biblioteca, de lado…", true)] {
                    if library
                        && ui
                            .button(label)
                            .on_hover_text(format!(
                                "Elegir entre los dibujos de los DXF de la carpeta «{}» de tu biblioteca, viéndolos",
                                super::symbol_pick::FOLDER
                            ))
                            .clicked()
                    {
                        from_library = Some(side);
                    }
                }
                for (label, image, side) in [
                    ("DXF de frente…", false, false),
                    ("DXF de lado…", false, true),
                    ("PNG de frente…", true, false),
                    ("PNG de lado…", true, true),
                ] {
                    if ui.button(label).on_hover_text(&hint).clicked() {
                        add = Some((image, side));
                    }
                }
            });
        });
        if let Some(side) = from_library {
            self.pick_from_library_for(t, super::symbol_pick::PickFor::Elevation { side });
        }
        if let Some((image, side)) = add {
            let mut d = rfd::FileDialog::new().set_title(if image {
                "Imagen para alzados (PNG con fondo transparente)"
            } else {
                "Dibujo para alzados (DXF)"
            });
            d = if image {
                d.add_filter("Imagen PNG", &["png"])
            } else {
                d.add_filter("Dibujo DXF", &["dxf"])
            };
            if let Some(dir) = self.start_dir() {
                d = d.set_directory(dir);
            }
            if let Some(path) = d.pick_file() {
                self.add_elevation_from(t, &path, side);
            }
        }
        if let Some(i) = remove {
            let mut list = figures;
            list.remove(i);
            let r = self.ed.transact("Quitar dibujo en alzado", |m| {
                m.set_object_type_elevation(t, list)
            });
            self.report(r, "Dibujo en alzado quitado.");
        }
    }

    /// Puts a PNG as the type's picture for presentation plans.
    pub(super) fn set_plan_image_from(&mut self, t: ObjectTypeId, path: &std::path::Path) {
        let bytes = match std::fs::read(path) {
            Ok(b) => b,
            Err(e) => {
                self.complain(format!("No se puede leer {}: {e}.", path.display()));
                return;
            }
        };
        let name = path.file_name().map_or_else(
            || path.display().to_string(),
            |n| n.to_string_lossy().into_owned(),
        );
        match archisim_export::symbol::elevation_from_png(&bytes, &name, false) {
            Ok(archisim_core::ElevationFigure {
                art: archisim_core::ElevationArt::Image { png, aspect },
                ..
            }) => {
                let img = archisim_core::PlanImage {
                    source: name.clone(),
                    png,
                    aspect,
                };
                let r = self.ed.transact("Imagen en planta", |m| {
                    m.set_object_type_plan_image(t, Some(img))
                });
                self.report(r, &format!("{name}: en las plantas de presentación."));
            }
            Ok(_) => {}
            Err(e) => self.complain(format!("No se puede usar {name}: {e}.")),
        }
    }

    /// Adds a DXF or a PNG as a drawing of the type for elevations (a
    /// drawing of the same kind and view it had is replaced).
    pub(super) fn add_elevation_from(
        &mut self,
        t: ObjectTypeId,
        path: &std::path::Path,
        side: bool,
    ) {
        let bytes = match std::fs::read(path) {
            Ok(b) => b,
            Err(e) => {
                self.complain(format!("No se puede leer {}: {e}.", path.display()));
                return;
            }
        };
        let name = path.file_name().map_or_else(
            || path.display().to_string(),
            |n| n.to_string_lossy().into_owned(),
        );
        let is_png = name.to_lowercase().ends_with(".png");
        // A library (a sheet of trees, one block each): choose one, seeing
        // them; the file is kept in the user's library.
        if !is_png {
            if let Ok(set) = super::symbol_pick::read_set(path, false) {
                if !set.symbols.is_empty() {
                    let n = set.symbols.len();
                    super::symbol_pick::keep_in_library(path);
                    self.symbol_picker = Some(super::symbol_pick::SymbolPicker::new(
                        t,
                        false,
                        vec![set],
                        super::symbol_pick::PickFor::Elevation { side },
                    ));
                    self.say(format!("{n} dibujos en el fichero: elige uno."));
                    return;
                }
            }
        }
        let figure = if is_png {
            archisim_export::symbol::elevation_from_png(&bytes, &name, side)
        } else {
            archisim_export::symbol::elevation_from_dxf(&bytes, &name, side)
        };
        match figure {
            Ok(f) => {
                let mut list = self
                    .ed
                    .model()
                    .object_type(t)
                    .map(|x| x.elevation.clone())
                    .unwrap_or_default();
                let image = |x: &archisim_core::ElevationFigure| {
                    matches!(x.art, archisim_core::ElevationArt::Image { .. })
                };
                list.retain(|x| !(x.side == f.side && image(x) == image(&f)));
                list.push(f);
                let r = self
                    .ed
                    .transact("Dibujo en alzado", |m| m.set_object_type_elevation(t, list));
                self.report(
                    r,
                    &format!(
                        "{name}: se dibujará en alzados y secciones {}.",
                        if is_png {
                            "de presentación"
                        } else {
                            "técnicos (y en los de presentación si no hay imagen)"
                        }
                    ),
                );
            }
            Err(e) => self.complain(format!("No se puede usar {name}: {e}.")),
        }
    }

    /// Asks for a DXF and puts its drawing as the type's symbol: stretched
    /// to the type for vegetation, at its own size for the rest.
    pub(super) fn pick_symbol(&mut self, t: ObjectTypeId) {
        let mut d = rfd::FileDialog::new()
            .set_title("Símbolo de planta (DXF)")
            .add_filter("Dibujo DXF", &["dxf"]);
        if let Some(dir) = self.start_dir() {
            d = d.set_directory(dir);
        }
        if let Some(path) = d.pick_file() {
            self.set_symbol_from(t, &path);
        }
    }

    /// Whether a symbol is stretched to the type (vegetation, people,
    /// site) or keeps the drawing's size (the rest).
    pub(super) fn symbol_fits(&self, t: ObjectTypeId) -> bool {
        self.ed.model().type_grows(t)
            || self
                .ed
                .model()
                .object_type(t)
                .and_then(|ty| self.ed.model().family(ty.family))
                .is_some_and(|f| matches!(f.def().category.as_str(), "site" | "people"))
    }

    pub(super) fn set_symbol_from(&mut self, t: ObjectTypeId, path: &std::path::Path) {
        let fit = self.symbol_fits(t);
        // A library (a sheet of trees, one block each): choose one,
        // seeing them; the file is kept in the user's library.
        match super::symbol_pick::read_set(path, fit) {
            Ok(set) if !set.symbols.is_empty() => {
                let n = set.symbols.len();
                let kept = super::symbol_pick::keep_in_library(path);
                self.symbol_picker = Some(super::symbol_pick::SymbolPicker::new(
                    t,
                    fit,
                    vec![set],
                    super::symbol_pick::PickFor::Plan,
                ));
                let note = if kept.is_some() {
                    format!(
                        " Guardado en tu biblioteca, carpeta «{}».",
                        super::symbol_pick::FOLDER
                    )
                } else {
                    String::new()
                };
                self.say(format!("{n} dibujos en el fichero: elige uno.{note}"));
                return;
            }
            _ => {}
        }
        let bytes = match std::fs::read(path) {
            Ok(b) => b,
            Err(e) => {
                self.complain(format!("No se puede leer {}: {e}.", path.display()));
                return;
            }
        };
        let name = path.file_name().map_or_else(
            || path.display().to_string(),
            |n| n.to_string_lossy().into_owned(),
        );
        match archisim_export::symbol::symbol_from_dxf(&bytes, &name, fit) {
            Ok(symbol) => {
                let n = symbol.items.len();
                let r = self.ed.transact("Símbolo de planta", |m| {
                    m.set_object_type_symbol(t, Some(symbol))
                });
                self.report(
                    r,
                    &format!(
                        "Símbolo de {name}: {n} piezas, {}.",
                        if fit {
                            "estirado al tamaño del objeto"
                        } else {
                            "a la medida del dibujo"
                        }
                    ),
                );
            }
            Err(e) => self.complain(format!("No se puede usar {name}: {e}.")),
        }
    }
}

impl ArchiSimApp {
    /// A plant grown by ArchiSim: another plant of its species with a
    /// click, its planting, the vivero.
    fn plant_group(&mut self, ui: &mut Ui, id: ObjectId, o: &archisim_core::Object) {
        let planting = self.ed.model().planting_of(id);
        group(ui, "object-plant", "Planta", |ui| {
            if let Some(look) = self.ed.model().plant_look(id) {
                let plant = archisim_core::flora::plant(&look.species, look.seed, look.variation);
                text_row(
                    ui,
                    "Tamaño",
                    "Alto y ancho de esta planta, como ha crecido",
                    &format!("{:.2} × {:.2} m", plant.height, plant.radius * 2.0).replace('.', ","),
                );
            }
            ui.horizontal_wrapped(|ui| {
                if ui
                    .button("🎲 Otra planta")
                    .on_hover_text("La misma especie, crecida de otra manera (otra semilla)")
                    .clicked()
                {
                    let mut values = o.values.clone();
                    let old = values
                        .get("plant_seed")
                        .and_then(|v| v.as_number())
                        .unwrap_or(0.0) as u64;
                    let t = std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .map(|d| d.as_nanos() as u64)
                        .unwrap_or(1);
                    let seed = archisim_core::flora::seed_of(old ^ t, id.0) % 999_983 + 1;
                    values.insert("plant_seed".into(), Value::Number(seed as f64));
                    let r = self.ed.set_object_values(id, values);
                    self.report(r, "Otra planta de la misma especie.");
                }
                if ui
                    .button("Vivero…")
                    .on_hover_text("La especie con todos sus parámetros y su vista 3D")
                    .clicked()
                {
                    self.open_vivero(Some(o.object_type), o.values.clone());
                }
            });
            if let Some(pid) = planting {
                let name = self
                    .ed
                    .model()
                    .planting(pid)
                    .map(|p| p.name.clone())
                    .unwrap_or_default();
                props::note(
                    ui,
                    &format!("Es de «{name}»: si cambias la plantación, sus plantas se rehacen."),
                );
                if ui.button("Seleccionar la plantación").clicked() {
                    self.ed.selection = [Element::Planting(pid)].into_iter().collect();
                }
            }
        });
    }
}

/// A row editing one family parameter; the new value when it changes.
pub(super) fn param_row(
    ui: &mut Ui,
    salt: &str,
    p: &archisim_core::family::Param,
    cur: &Value,
    materials: &[(u64, String)],
) -> Option<Value> {
    let label = if p.label.is_empty() {
        p.name.clone()
    } else {
        p.label.clone()
    };
    let id = Id::unique(("param", salt.to_string()));
    let mut out = None;
    match (&p.kind, cur) {
        (ParamKind::Length, Value::Number(x)) => {
            row(ui, &format!("{label} (mm)"), "", |ui, w| {
                if let Some(v) = length_field(ui, id, *x, w - 8.0) {
                    out = Some(Value::Number(v));
                }
            });
        }
        (ParamKind::Integer | ParamKind::Number, Value::Number(x)) => {
            let integer = p.kind == ParamKind::Integer;
            let (lo, hi) = p.range.unwrap_or((f64::MIN, f64::MAX));
            row(ui, &label, "", |ui, _| {
                let mut v = *x;
                let drag = egui::DragValue::new(&mut v)
                    .range(lo..=hi)
                    .speed(if integer { 0.2 } else { 0.01 })
                    .max_decimals(if integer { 0 } else { 2 });
                if ui.add(drag).changed() {
                    out = Some(Value::Number(if integer { v.round() } else { v }));
                }
            });
        }
        (ParamKind::Bool, Value::Bool(b)) => {
            row(ui, &label, "", |ui, _| {
                let mut on = *b;
                if ui.switch(&mut on, "").changed() {
                    out = Some(Value::Bool(on));
                }
            });
        }
        (ParamKind::Choice(opts), Value::Text(t)) => {
            row(ui, &label, "", |ui, w| {
                let refs: Vec<(&str, &str)> =
                    opts.iter().map(|o| (o.as_str(), o.as_str())).collect();
                let v = props::choice(ui, salt, w, t.as_str(), &refs);
                if v != t.as_str() {
                    out = Some(Value::Text(v.to_string()));
                }
            });
        }
        (ParamKind::Material, Value::Material(m)) => {
            row(ui, &label, "", |ui, w| {
                let refs: Vec<(u64, &str)> =
                    materials.iter().map(|(a, b)| (*a, b.as_str())).collect();
                let v = props::choice(ui, salt, w, *m, &refs);
                if v != *m {
                    out = Some(Value::Material(v));
                }
            });
        }
        _ => {}
    }
    out
}

/// As [`param_row`], with sliders for numbers and lengths with a range
/// (the vivero's way: moved and seen at once).
pub(super) fn param_slider_row(
    ui: &mut Ui,
    salt: &str,
    p: &archisim_core::family::Param,
    cur: &Value,
    materials: &[(u64, String)],
) -> Option<Value> {
    let (Some((lo, hi)), Value::Number(x)) = (p.range, cur) else {
        return param_row(ui, salt, p, cur, materials);
    };
    let label = if p.label.is_empty() {
        p.name.clone()
    } else {
        p.label.clone()
    };
    let mut out = None;
    match p.kind {
        ParamKind::Length => {
            row(ui, &format!("{label} (m)"), "", |ui, w| {
                let mut v = *x;
                ui.spacing_mut().slider_width = (w - 70.0).max(60.0);
                let s = egui::Slider::new(&mut v, lo..=hi)
                    .max_decimals(2)
                    .logarithmic(hi / lo.max(1e-3) > 50.0);
                if ui.add(s).changed() {
                    out = Some(Value::Number((v * 1000.0).round() / 1000.0));
                }
            });
        }
        ParamKind::Integer | ParamKind::Number => {
            let integer = p.kind == ParamKind::Integer;
            row(ui, &label, "", |ui, w| {
                let mut v = *x;
                ui.spacing_mut().slider_width = (w - 70.0).max(60.0);
                let s =
                    egui::Slider::new(&mut v, lo..=hi).max_decimals(if integer { 0 } else { 2 });
                let s = if integer { s.step_by(1.0) } else { s };
                if ui.add(s).changed() {
                    out = Some(Value::Number(if integer { v.round() } else { v }));
                }
            });
        }
        _ => return param_row(ui, salt, p, cur, materials),
    }
    out
}

impl ArchiSimApp {
    /// A built-in wardrobe (E10): between partitions, wall face to wall
    /// face; or in a hole in the wall it stands in.
    fn built_in_group(&mut self, ui: &mut Ui, id: ObjectId) {
        let mut fit = false;
        let mut hole = false;
        props::group(ui, "built-in", "Empotrado", |ui| {
            props::note(
                ui,
                "Entre tabiques: ocupa de cara a cara de los muros que tiene a los lados. En un hueco de muro: colócalo dentro del muro y abre en él un hueco de su tamaño.",
            );
            ui.horizontal_wrapped(|ui| {
                ui.add_space(4.0);
                if ui
                    .button("Encajar entre tabiques")
                    .on_hover_text("Su ancho, de cara a cara de los muros de los lados")
                    .clicked()
                {
                    fit = true;
                }
                if ui
                    .button("Abrir hueco en el muro")
                    .on_hover_text(
                        "Un hueco sin carpintería de su ancho y su alto en el muro donde está",
                    )
                    .clicked()
                {
                    hole = true;
                }
            });
        });
        if fit {
            let Some((Some(left), Some(right))) = self.ed.model().room_between_walls(id) else {
                self.complain("No hay muros a los dos lados del armario.");
                return;
            };
            let shift = (right - left) / 2.0;
            let r = self.ed.transact("Encajar armario", |m| {
                let o = m
                    .object(id)
                    .cloned()
                    .ok_or(archisim_core::ModelError::UnknownObject(id))?;
                let ux = archisim_core::Point2::new(o.rotation.cos(), o.rotation.sin());
                m.edit_object(id, |x| x.position = o.position + ux * shift)?;
                crate::drag::set_object_size(m, id, "width", left + right)
            });
            self.report(
                r,
                &format!(
                    "Armario encajado: {} m de ancho.",
                    format!("{:.2}", left + right).replace('.', ",")
                ),
            );
        }
        if hole {
            let r = self.ed.transact("Hueco para el armario", |m| {
                m.hole_for_object(id).map(|_| ())
            });
            self.report(r, "Hueco abierto en el muro.");
        }
    }
}
