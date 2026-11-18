//! «Sustituir todos» (N4, `docs/TERRITORIO.md` §2.7): every object of a
//! type, or every street material, changed for another in the project, on
//! the level, inside an area picked with two corners or along a stretch of
//! the street selected (as BOB and Repaint in Cities: Skylines).

use archisim_core::{MaterialId, ObjectTypeId, Point2, ReplaceScope, RoadId};
use eframe::egui::{self, ComboBox, Id, Ui};

use super::props::{self, row};
use super::*;

/// What is replaced.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ReplaceWhat {
    /// Objects of a type (trees, lamps, benches…).
    #[default]
    Objects,
    /// Street materials (paving, kerbs, sidewalks).
    Materials,
}

/// Where it reaches, as chosen in the window.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ReplaceWhere {
    #[default]
    Project,
    Level,
    /// The rectangle picked on the plan.
    Area,
    /// A stretch of the street selected.
    Street,
}

/// The window «Sustituir todos».
#[derive(Debug, Clone, Default)]
pub struct ReplaceState {
    pub open: bool,
    pub what: ReplaceWhat,
    pub scope: ReplaceWhere,
    pub from_type: Option<ObjectTypeId>,
    pub to_type: Option<ObjectTypeId>,
    pub from_material: Option<MaterialId>,
    pub to_material: Option<MaterialId>,
    /// The area's two corners, once picked.
    pub area: Option<(Point2, Point2)>,
    /// Picking the area on the plan: its first corner, once clicked.
    pub picking: Option<Option<Point2>>,
    /// The stretch of the street: its street, from and to.
    pub street: Option<(RoadId, f64, f64)>,
}

impl ReplaceState {
    /// The scope as the model takes it.
    pub fn scope(&self, level: archisim_core::LevelId) -> Option<ReplaceScope> {
        Some(match self.scope {
            ReplaceWhere::Project => ReplaceScope::All,
            ReplaceWhere::Level => ReplaceScope::Level(level),
            ReplaceWhere::Area => {
                let (a, b) = self.area?;
                ReplaceScope::Area {
                    level,
                    outline: vec![
                        Point2::new(a.x.min(b.x), a.y.min(b.y)),
                        Point2::new(a.x.max(b.x), a.y.min(b.y)),
                        Point2::new(a.x.max(b.x), a.y.max(b.y)),
                        Point2::new(a.x.min(b.x), a.y.max(b.y)),
                    ],
                }
            }
            ReplaceWhere::Street => {
                let (road, from, to) = self.street?;
                ReplaceScope::Road { road, from, to }
            }
        })
    }
}

impl ArchiSimApp {
    pub(super) fn replace_dialog(&mut self, ctx: &egui::Context) {
        if !self.replace.open {
            return;
        }
        let mut open = true;
        super::widgets::side_sheet(ctx, "Sustituir todos", 420.0)
            .open(&mut open)
            .show(ctx, |ui| self.replace_contents(ui));
        self.replace.open = open;
        if !open {
            self.replace.picking = None;
        }
    }

    /// A click on the plan while picking the area: its two corners.
    pub(super) fn replace_area_click(&mut self, p: Point2) {
        match self.replace.picking {
            Some(None) => {
                self.replace.picking = Some(Some(p));
                self.say("Ahora la esquina opuesta de la zona.");
            }
            Some(Some(a)) => {
                self.replace.picking = None;
                self.replace.area = Some((a, p));
                self.replace.scope = ReplaceWhere::Area;
                self.say(format!(
                    "Zona de {} × {} m.",
                    super::urban_ui::comma((p.x - a.x).abs(), 1),
                    super::urban_ui::comma((p.y - a.y).abs(), 1)
                ));
            }
            None => {}
        }
    }

    fn replace_contents(&mut self, ui: &mut Ui) {
        let model = self.ed.model();
        let level = self.ed.level;
        props::note(
            ui,
            "Cambia de una vez todos los objetos de un tipo (árboles, farolas, bancos…) o un material de las calles por otro, en todo el proyecto, en esta planta, en una zona o en un tramo de una calle.",
        );
        let st = &mut self.replace;
        ui.horizontal(|ui| {
            ui.selectable_value(&mut st.what, ReplaceWhat::Objects, "Objetos");
            ui.selectable_value(
                &mut st.what,
                ReplaceWhat::Materials,
                "Materiales de las calles",
            );
        });
        // The street selected, for «En un tramo».
        let selected_road = self.ed.selection.iter().find_map(|e| match e {
            Element::Road(r) => Some(*r),
            _ => None,
        });
        if let Some(r) = selected_road.and_then(|r| model.road(r)) {
            if st.street.is_none_or(|(id, _, _)| id != r.id) {
                st.street = Some((r.id, 0.0, r.length()));
            }
        }
        let mut pick_area = false;
        egui::Grid::new("replace-where").show(ui, |ui| {
            ui.label("Dónde");
            ui.vertical(|ui| {
                ui.radio_value(&mut st.scope, ReplaceWhere::Project, "En todo el proyecto");
                ui.radio_value(&mut st.scope, ReplaceWhere::Level, "En esta planta");
                ui.horizontal(|ui| {
                    ui.add_enabled(
                        st.area.is_some(),
                        egui::RadioButton::new(st.scope == ReplaceWhere::Area, "En la zona"),
                    )
                    .clicked()
                    .then(|| st.scope = ReplaceWhere::Area);
                    pick_area = ui
                        .small_button("Elegir en planta")
                        .on_hover_text("Dos clics: dos esquinas opuestas de un rectángulo")
                        .clicked();
                });
                ui.add_enabled(
                    st.street.is_some(),
                    egui::RadioButton::new(
                        st.scope == ReplaceWhere::Street,
                        "En un tramo de la calle elegida",
                    ),
                )
                .clicked()
                .then(|| st.scope = ReplaceWhere::Street);
            });
            ui.end_row();
        });
        if st.scope == ReplaceWhere::Street {
            if let Some((road, from, to)) = st.street.as_mut() {
                let len = model.road(*road).map_or(0.0, |r| r.length());
                let name = model.road(*road).map_or("", |r| r.name.as_str());
                props::note(
                    ui,
                    &format!("{name}, de {} m de eje.", super::urban_ui::comma(len, 1)),
                );
                for (label, v) in [("Desde (mm)", from), ("Hasta (mm)", to)] {
                    row(ui, label, "A lo largo del eje", |ui, w| {
                        if let Some(x) =
                            length_field(ui, Id::unique(("replace-at", label)), *v, w - 8.0)
                        {
                            *v = x.clamp(0.0, len);
                        }
                    });
                }
            }
        } else if st.street.is_none() {
            props::note(ui, "Para sustituir en un tramo, elige antes una calle.");
        }
        let mut go = None;
        match st.what {
            ReplaceWhat::Objects => {
                let mut types: Vec<(ObjectTypeId, String, usize)> = model
                    .object_types()
                    .filter(|t| !model.is_railing_type(t.id))
                    .map(|t| (t.id, t.name.clone(), model.object_type_users(t.id)))
                    .collect();
                types.sort_by(|a, b| a.1.cmp(&b.1));
                let name = |id: Option<ObjectTypeId>| {
                    id.and_then(|i| types.iter().find(|t| t.0 == i))
                        .map_or("Elige un tipo".to_string(), |t| t.1.clone())
                };
                row(ui, "Cambiar", "Los objetos de este tipo", |ui, w| {
                    ComboBox::from_id_salt("replace-from-type")
                        .width(w - 8.0)
                        .selected_text(name(st.from_type))
                        .show_ui(ui, |ui| {
                            for (id, n, users) in types.iter().filter(|t| t.2 > 0) {
                                ui.selectable_value(
                                    &mut st.from_type,
                                    Some(*id),
                                    format!("{n} ({users})"),
                                );
                            }
                        });
                });
                row(ui, "Por", "El tipo que pasan a ser", |ui, w| {
                    ComboBox::from_id_salt("replace-to-type")
                        .width(w - 8.0)
                        .selected_text(name(st.to_type))
                        .show_ui(ui, |ui| {
                            for (id, n, _) in &types {
                                ui.selectable_value(&mut st.to_type, Some(*id), n);
                            }
                        });
                });
                if let (Some(a), Some(b), Some(scope)) = (st.from_type, st.to_type, st.scope(level))
                {
                    let n = model
                        .objects_in(&scope)
                        .into_iter()
                        .filter(|id| model.object(*id).is_some_and(|o| o.object_type == a))
                        .count();
                    props::note(ui, &format!("{n} objetos de ese tipo aquí."));
                    if ui
                        .add_enabled(n > 0 && a != b, egui::Button::new("Sustituir"))
                        .clicked()
                    {
                        go = Some((scope, Some((a, b)), None));
                    }
                }
            }
            ReplaceWhat::Materials => {
                let mut used: Vec<MaterialId> = model
                    .roads()
                    .flat_map(|r| {
                        let mut v = vec![r.pavement, r.kerb_material, r.sidewalk_material];
                        v.extend(r.section.planting);
                        v.extend(
                            r.skins
                                .iter()
                                .flat_map(|k| k.materials().into_iter().flatten()),
                        );
                        v
                    })
                    .collect();
                used.sort();
                used.dedup();
                let mname = |id: Option<MaterialId>| {
                    id.and_then(|i| model.material(i))
                        .map_or("Elige un material".to_string(), |m| m.name.clone())
                };
                row(ui, "Cambiar", "El material de las calles", |ui, w| {
                    ComboBox::from_id_salt("replace-from-mat")
                        .width(w - 8.0)
                        .selected_text(mname(st.from_material))
                        .show_ui(ui, |ui| {
                            for id in &used {
                                ui.selectable_value(
                                    &mut st.from_material,
                                    Some(*id),
                                    mname(Some(*id)),
                                );
                            }
                        });
                });
                row(ui, "Por", "El que se pone", |ui, w| {
                    let mut to = st
                        .to_material
                        .unwrap_or_else(|| st.from_material.unwrap_or(MaterialId(0)));
                    super::cards::material_button(
                        ui,
                        Id::unique("replace-to-mat"),
                        model,
                        &mut to,
                        w - 8.0,
                    );
                    if model.material(to).is_some() {
                        st.to_material = Some(to);
                    }
                });
                if let (Some(a), Some(b), Some(scope)) =
                    (st.from_material, st.to_material, st.scope(level))
                {
                    if ui
                        .add_enabled(a != b, egui::Button::new("Sustituir"))
                        .clicked()
                    {
                        go = Some((scope, None, Some((a, b))));
                    }
                }
            }
        }
        if pick_area {
            self.replace.picking = Some(None);
            self.set_tool(Tool::Arrow);
            self.replace.picking = Some(None);
            self.say("Clic en una esquina de la zona.");
        }
        if let Some((scope, types, mats)) = go {
            let mut n = 0;
            let res = self.ed.transact("Sustituir todos", |m| {
                n = match (types, mats) {
                    (Some((a, b)), _) => m.replace_object_type(a, b, &scope)?,
                    (_, Some((a, b))) => m.replace_road_material(a, b, &scope)?,
                    _ => 0,
                };
                Ok::<(), archisim_core::ModelError>(())
            });
            let done = match (types, n) {
                (Some(_), 1) => "1 objeto sustituido.".to_string(),
                (Some(_), n) => format!("{n} objetos sustituidos."),
                (None, 1) => "Material cambiado en 1 calle.".to_string(),
                (None, n) => format!("Material cambiado en {n} calles."),
            };
            self.report(res, &done);
        }
    }
}
