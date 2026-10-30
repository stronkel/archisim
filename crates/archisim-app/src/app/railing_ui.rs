//! The railing tool ("Barandilla"): its line clicked point by point, for
//! terraces, flat roofs and voids; height, foot and posts in Properties.

use super::widgets::SwitchExt as _;
use archisim_core::RailingId;
use eframe::egui::{self, Id, Ui};

use super::icons::Icon;
use super::props::{self, group, row, text_row};
use super::*;

impl ArchiSimApp {
    pub(super) fn railing_click(&mut self, p: Point2) {
        if self.railing_points.last().is_some_and(|q| q.approx_eq(p)) {
            return;
        }
        self.railing_points.push(p);
    }

    pub(super) fn finish_railing(&mut self) {
        let path = std::mem::take(&mut self.railing_points);
        if path.len() < 2 {
            self.railing_points = path;
            return;
        }
        match self.ed.add_railing(&self.railing_settings, path) {
            Ok(id) => {
                self.ed.selection = [Element::Railing(id)].into_iter().collect();
                self.say(
                    "Barandilla creada: montantes, pasamanos y barrotes a menos de 10 cm (CTE).",
                );
            }
            Err(e) => self.complain(format!("No se puede crear la barandilla: {e}.")),
        }
    }

    pub(super) fn draw_railing_preview(&self, painter: &egui::Painter, rect: Rect) {
        if self.tool != Tool::Railing || self.railing_points.is_empty() {
            return;
        }
        let mut pts = self.railing_points.clone();
        if let Some(c) = self.cursor {
            pts.push(c.point);
        }
        let screen: Vec<Pos2> = pts.iter().map(|p| self.view.to_screen(rect, *p)).collect();
        painter.add(Shape::line(screen, Stroke::new(2.0, ACCENT)));
        if let (Some(c), Some(a)) = (self.cursor, self.railing_points.last()) {
            self.draw_label(
                painter,
                rect,
                c.point,
                format!(
                    "Barandilla   L {}",
                    crate::units::fmt_len(a.distance(c.point))
                ),
            );
        }
    }

    pub(super) fn railing_tool_properties(&mut self, ui: &mut Ui) {
        props::title(ui, "Herramienta Barandilla · lo que vas a dibujar");
        group(ui, "tool-railing", "Barandilla", |ui| {
            let s = &mut self.railing_settings;
            let fields: [(&str, &str, &mut f64); 3] = [
                (
                    "Altura (mm)",
                    "Del pie al pasamanos (CTE: 900 o 1100)",
                    &mut s.height,
                ),
                (
                    "Desfase base (mm)",
                    "Respecto a la planta",
                    &mut s.base_offset,
                ),
                (
                    "Montantes cada (mm)",
                    "Distancia mayor entre montantes",
                    &mut s.post_spacing,
                ),
            ];
            for (i, (label, hint, v)) in fields.into_iter().enumerate() {
                row(ui, label, hint, |ui, w| {
                    if let Some(x) = length_field(ui, Id::unique(("tool-railing", i)), *v, w - 8.0)
                    {
                        *v = x;
                    }
                });
            }
        });
        let model = self.ed.model();
        let now = if self.railing_settings.plain {
            None
        } else {
            self.railing_settings
                .railing_type
                .filter(|t| model.is_railing_type(*t))
                .or_else(|| crate::editor::default_railing_type(model))
        };
        if let Some(pick) = railing_type_choice(ui, model, "tool-railing-type", now) {
            self.railing_settings.plain = pick.is_none();
            self.railing_settings.railing_type = pick;
        }
        if now.is_none() {
            group(ui, "tool-railing-plain", "Sencilla", |ui| {
                row(
                    ui,
                    "Vidrio",
                    "Paneles de vidrio entre montantes en vez de barrotes",
                    |ui, _| {
                        ui.switch(&mut self.railing_settings.glass, "");
                    },
                );
            });
        }
        ui.add_space(6.0);
        props::note(
            ui,
            "Clic en cada punto de la barandilla; Intro, doble clic o clic derecho la termina; Esc quita el último punto. Los barrotes se reparten sin dejar más de 10 cm entre ellos.",
        );
    }

    pub(super) fn railing_properties(&mut self, ui: &mut Ui, id: RailingId) {
        let Some(r) = self.ed.model().railing(id).cloned() else {
            return;
        };
        props::title(ui, "Barandilla · 1 seleccionada");
        super::properties::view_card(
            ui,
            Icon::Railing,
            &format!("Barandilla de {} m", fmt_m(r.length())),
            "IfcRailing",
        );
        group(ui, "railing-data", "Barandilla", |ui| {
            type Set = fn(&mut archisim_core::Railing, f64);
            let fields: [(&str, &str, f64, Set); 3] = [
                ("Altura (mm)", "Del pie al pasamanos", r.height, |r, v| {
                    r.height = v
                }),
                (
                    "Desfase base (mm)",
                    "Respecto a la planta",
                    r.base_offset,
                    |r, v| r.base_offset = v,
                ),
                (
                    "Montantes cada (mm)",
                    "Distancia mayor",
                    r.post_spacing,
                    |r, v| r.post_spacing = v,
                ),
            ];
            for (i, (label, hint, value, set)) in fields.into_iter().enumerate() {
                row(ui, label, hint, |ui, w| {
                    if let Some(v) =
                        length_field(ui, Id::unique(("railing", id.0, i)), value, w - 8.0)
                    {
                        let r = self.ed.edit_railing(label, id, |r| set(r, v));
                        self.report(r, "Barandilla cambiada.");
                    }
                });
            }
            if r.railing_type.is_none() {
                row(
                    ui,
                    "Vidrio",
                    "Paneles de vidrio entre montantes en vez de barrotes",
                    |ui, _| {
                        let mut on = r.glass.is_some();
                        if ui.switch(&mut on, "").changed() {
                            let res = self.ed.set_railing_glass(id, on);
                            self.report(res, "Barandilla cambiada.");
                        }
                    },
                );
            }
            let posts = r
                .pieces()
                .iter()
                .filter(|p| p.kind == archisim_core::RailingPieceKind::Post)
                .count();
            text_row(ui, "Montantes", "Contando las esquinas", &posts.to_string());
        });
        // Its type: what it is made of.
        if let Some(pick) = railing_type_choice(
            ui,
            self.ed.model(),
            &format!("railing-type-{}", id.0),
            r.railing_type,
        ) {
            let res = self
                .ed
                .edit_railing("Tipo de barandilla", id, |r| r.railing_type = pick);
            self.report(res, "Tipo de barandilla cambiado.");
        }
        if let Some(t) = r
            .railing_type
            .and_then(|t| self.ed.model().object_type(t).cloned())
        {
            self.railing_type_values(ui, &t);
        }
        group(ui, "railing-identity", "Datos de identidad", |ui| {
            props::id_row(ui, "GlobalId", &r.guid.to_string());
        });
    }

    /// The values of a railing type, as an object type's: cards for the
    /// choices (with the railing in 3D), materials with their swatches.
    fn railing_type_values(&mut self, ui: &mut Ui, t: &archisim_core::ObjectType) {
        use archisim_core::ParamScope;
        let Some(def) = self.ed.model().family(t.family).map(|f| f.def().clone()) else {
            return;
        };
        let materials: Vec<(u64, String)> = self
            .ed
            .model()
            .materials()
            .map(|m| (m.id.0, m.name.clone()))
            .collect();
        let users = self.ed.model().object_type_users(t.id);
        group(
            ui,
            "railing-type-values",
            &format!("Tipo · {}", t.name),
            |ui| {
                props::note(
                    ui,
                    &if users <= 1 {
                        "Lo que cambies aquí cambia el tipo.".to_string()
                    } else {
                        format!("Lo que cambies aquí cambia el tipo: sus {users} barandillas.")
                    },
                );
                for p in def.params_of(ParamScope::Type) {
                    if !def.is_visible(&p.name, &t.values) {
                        continue;
                    }
                    let Some(cur) = t.values.get(&p.name) else {
                        continue;
                    };
                    let salt = format!("railing-tv-{}-{}", t.id.0, p.name);
                    let mark = self.ed.undo_mark();
                    if let Some(v) = self.object_type_param_row(ui, &salt, p, cur, t, &materials) {
                        let r = self.ed.set_object_type_value(t.id, &p.name, v);
                        self.ed.join_since(mark, "Tipo de barandilla cambiado");
                        self.report(r, "Tipo de barandilla cambiado.");
                    }
                }
            },
        );
    }
}

/// A list of the project's railing types and «Sencilla» (no type); the
/// one picked, when it changes.
pub(crate) fn railing_type_choice(
    ui: &mut Ui,
    model: &archisim_core::Model,
    salt: &str,
    now: Option<archisim_core::ObjectTypeId>,
) -> Option<Option<archisim_core::ObjectTypeId>> {
    const PLAIN: &str = "Sencilla (montantes, pasamanos y barrotes)";
    let types: Vec<(archisim_core::ObjectTypeId, String)> = model
        .railing_types()
        .into_iter()
        .map(|t| (t.id, t.name.clone()))
        .collect();
    let shown = now
        .and_then(|n| types.iter().find(|(id, _)| *id == n))
        .map_or(PLAIN.to_string(), |(_, name)| name.clone());
    let mut pick = None;
    group(ui, &format!("{salt}-group"), "Tipo de barandilla", |ui| {
        row(
            ui,
            "Tipo",
            "Montantes, pasamanos, relleno y anclaje",
            |ui, w| {
                egui::ComboBox::from_id_salt(salt)
                    .width(w - 8.0)
                    .selected_text(shown.clone())
                    .show_ui(ui, |ui| {
                        for (id, name) in &types {
                            if ui.selectable_label(now == Some(*id), name).clicked()
                                && now != Some(*id)
                            {
                                pick = Some(Some(*id));
                            }
                        }
                        if ui.selectable_label(now.is_none(), PLAIN).clicked() && now.is_some() {
                            pick = Some(None);
                        }
                    });
            },
        );
    });
    pick
}
