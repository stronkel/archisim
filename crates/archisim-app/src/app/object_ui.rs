//! The object tool ("Objeto"): furniture, sanitary ware and kitchen units
//! from the project's object types, placed with a click (back side on the
//! point, Space turns it 90°); their properties and type sizes.

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
        let t = valid.or_else(|| model.object_types().next().map(|t| t.id));
        self.object_settings.object_type = t;
        t
    }

    pub(super) fn object_click(&mut self, p: Point2) {
        if self.tool_object_type().is_none() {
            self.complain("El proyecto no tiene tipos de objeto.");
            return;
        }
        match self.ed.add_object(&self.object_settings, p) {
            Ok(id) => {
                self.ed.selection = [Element::Object(id)].into_iter().collect();
                self.say("Objeto colocado. Espacio: girarlo 90° antes de colocar el siguiente.");
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
        if let Some(pick) = self.object_card(ui, &format!("object-{}", id.0), Some(t.id)) {
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
                        if let Some(v) = param_row(ui, &salt, p, cur, &materials) {
                            let mut values = o.values.clone();
                            values.insert(p.name.clone(), v);
                            let r = self.ed.set_object_values(id, values);
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
                    if let Some(v) = param_row(ui, &salt, p, cur, &materials) {
                        let r = self.ed.set_object_type_value(t.id, &p.name, v);
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
        group(ui, "object-identity", "Datos de identidad", |ui| {
            props::id_row(ui, "GlobalId", &o.guid.to_string());
        });
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
                if ui.checkbox(&mut on, "").changed() {
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
