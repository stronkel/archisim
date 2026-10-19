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
        props::title(ui, "Herramienta Objeto · lo que vas a colocar");
        let current = self.tool_object_type();
        if let Some(pick) = self.object_card(ui, "tool-object", current) {
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
                    let label = if p.label.is_empty() {
                        p.name.clone()
                    } else {
                        p.label.clone()
                    };
                    match (&p.kind, t.values.get(&p.name)) {
                        (ParamKind::Length, Some(Value::Number(x))) => {
                            row(ui, &format!("{label} (mm)"), "", |ui, w| {
                                if let Some(v) = length_field(
                                    ui,
                                    Id::unique(("object-tv", t.id.0, p.name.as_str())),
                                    *x,
                                    w - 8.0,
                                ) {
                                    let r = self.ed.set_object_type_value(
                                        t.id,
                                        &p.name,
                                        Value::Number(v),
                                    );
                                    self.report(r, "Tipo de objeto cambiado.");
                                }
                            });
                        }
                        (ParamKind::Integer, Some(Value::Number(x))) => {
                            let shown = format!("{}", x.round() as i64);
                            row(ui, &label, "", |ui, w| {
                                if let Some(v) = text_edit_committed(
                                    ui,
                                    Id::unique(("object-tv", t.id.0, p.name.as_str())),
                                    &shown,
                                    w - 8.0,
                                ) {
                                    match v.trim().parse::<i64>() {
                                        Ok(n) => {
                                            let r = self.ed.set_object_type_value(
                                                t.id,
                                                &p.name,
                                                Value::Number(n as f64),
                                            );
                                            self.report(r, "Tipo de objeto cambiado.");
                                        }
                                        Err(_) => self.complain("Escribe un número entero."),
                                    }
                                }
                            });
                        }
                        (ParamKind::Bool, Some(Value::Bool(b))) => {
                            row(ui, &label, "", |ui, _| {
                                let mut on = *b;
                                if ui.checkbox(&mut on, "").changed() {
                                    let r = self.ed.set_object_type_value(
                                        t.id,
                                        &p.name,
                                        Value::Bool(on),
                                    );
                                    self.report(r, "Tipo de objeto cambiado.");
                                }
                            });
                        }
                        (ParamKind::Material, Some(Value::Material(m))) => {
                            let name = self
                                .ed
                                .model()
                                .material(archisim_core::MaterialId(*m))
                                .map_or(String::new(), |m| m.name.clone());
                            text_row(ui, &label, "En Tipos y materiales", &name);
                        }
                        _ => {}
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
        group(ui, "object-identity", "Datos de identidad", |ui| {
            props::id_row(ui, "GlobalId", &o.guid.to_string());
        });
    }
}
