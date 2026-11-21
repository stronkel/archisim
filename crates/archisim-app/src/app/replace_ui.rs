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

/// How the area is being picked on the plan.
#[derive(Debug, Clone, PartialEq, Default)]
pub enum AreaPick {
    /// Two opposite corners of a rectangle (the first, once clicked).
    #[default]
    Rect,
    RectFrom(Point2),
    /// A free outline: its corners so far (a click on the first closes it).
    Free(Vec<Point2>),
    /// A click on a plot takes its outline; outside the plots, the city
    /// block round the click.
    Plot,
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
    /// The area's outline, once picked.
    pub area: Option<Vec<Point2>>,
    /// Picking the area on the plan, and how.
    pub picking: Option<AreaPick>,
    /// The stretch of the street: its street, from and to.
    pub street: Option<(RoadId, f64, f64)>,
}

impl ReplaceState {
    /// The scope as the model takes it.
    pub fn scope(&self, level: archisim_core::LevelId) -> Option<ReplaceScope> {
        Some(match self.scope {
            ReplaceWhere::Project => ReplaceScope::All,
            ReplaceWhere::Level => ReplaceScope::Level(level),
            ReplaceWhere::Area => ReplaceScope::Area {
                level,
                outline: self.area.clone()?,
            },
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

    /// A click on the plan while picking the area.
    pub(super) fn replace_area_click(&mut self, p: Point2) {
        match self.replace.picking.take() {
            Some(AreaPick::Rect) => {
                self.replace.picking = Some(AreaPick::RectFrom(p));
                self.say("Ahora la esquina opuesta de la zona.");
            }
            Some(AreaPick::RectFrom(a)) => {
                self.set_replace_area(vec![
                    Point2::new(a.x.min(p.x), a.y.min(p.y)),
                    Point2::new(a.x.max(p.x), a.y.min(p.y)),
                    Point2::new(a.x.max(p.x), a.y.max(p.y)),
                    Point2::new(a.x.min(p.x), a.y.max(p.y)),
                ]);
            }
            Some(AreaPick::Free(mut pts)) => {
                let reach = self.view.to_metres(crate::touch::reach(10.0));
                if pts.len() >= 3 && pts[0].distance(p) < reach {
                    self.set_replace_area(pts);
                } else {
                    pts.push(p);
                    self.say(format!(
                        "{} puntos · clic en el primero para cerrar la zona.",
                        pts.len()
                    ));
                    self.replace.picking = Some(AreaPick::Free(pts));
                }
            }
            Some(AreaPick::Plot) => {
                let model = self.ed.model();
                let level = self.ed.level;
                let plot = model
                    .plots()
                    .filter(|x| x.level == level)
                    .find(|x| archisim_core::point_in_polygon(p, &x.outline))
                    .map(|x| (x.outline.clone(), x.name.clone()));
                let found = plot.or_else(|| {
                    archisim_geom::blocks::city_blocks(model, level)
                        .into_iter()
                        .find(|b| archisim_core::point_in_polygon(p, b))
                        .map(|b| (b, "la manzana".to_string()))
                });
                match found {
                    Some((outline, name)) => {
                        self.set_replace_area(outline);
                        self.say(format!("Zona: {name}."));
                    }
                    None => {
                        self.replace.picking = Some(AreaPick::Plot);
                        self.say("Ahí no hay parcela ni manzana: clic dentro de una.");
                    }
                }
            }
            None => {}
        }
    }

    /// Closes the free outline being drawn (Intro). Whether there was one.
    pub(super) fn replace_area_finish(&mut self) -> bool {
        match self.replace.picking.take() {
            Some(AreaPick::Free(pts)) if pts.len() >= 3 => {
                self.set_replace_area(pts);
                true
            }
            other => {
                self.replace.picking = other;
                false
            }
        }
    }

    fn set_replace_area(&mut self, outline: Vec<Point2>) {
        let area = archisim_core::signed_area(&outline).abs();
        self.replace.picking = None;
        self.replace.area = Some(outline);
        self.replace.scope = ReplaceWhere::Area;
        self.say(format!("Zona de {} m².", super::urban_ui::comma(area, 0)));
    }

    /// The area picked (and the one being picked), over the plan.
    pub(super) fn draw_replace_area(&self, painter: &egui::Painter, rect: egui::Rect) {
        if !self.replace.open {
            return;
        }
        let ink = egui::Color32::from_rgb(200, 60, 140);
        let ring = |pts: &[Point2], closed: bool| {
            let mut line: Vec<Point2> = pts.to_vec();
            if closed {
                if let Some(&f) = pts.first() {
                    line.push(f);
                }
            }
            painter.add(egui::Shape::dashed_line(
                &self.polygon(rect, &line),
                egui::Stroke::new(2.0, ink),
                8.0,
                4.0,
            ));
        };
        if let Some(a) = &self.replace.area {
            ring(a, true);
        }
        let cursor = self.cursor.map(|c| c.point);
        match (&self.replace.picking, cursor) {
            (Some(AreaPick::RectFrom(a)), Some(c)) => {
                ring(&[*a, Point2::new(c.x, a.y), c, Point2::new(a.x, c.y)], true)
            }
            (Some(AreaPick::Free(pts)), c) => {
                let mut line = pts.clone();
                line.extend(c);
                ring(&line, false);
            }
            _ => {}
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
        let mut pick_area: Option<AreaPick> = None;
        egui::Grid::new("replace-where").show(ui, |ui| {
            ui.label("Dónde");
            ui.vertical(|ui| {
                ui.radio_value(&mut st.scope, ReplaceWhere::Project, "En todo el proyecto");
                ui.radio_value(&mut st.scope, ReplaceWhere::Level, "En esta planta");
                ui.add_enabled(
                    st.area.is_some(),
                    egui::RadioButton::new(st.scope == ReplaceWhere::Area, "En la zona"),
                )
                .clicked()
                .then(|| st.scope = ReplaceWhere::Area);
                ui.horizontal_wrapped(|ui| {
                    if ui
                        .small_button("Rectángulo")
                        .on_hover_text("Dos clics: dos esquinas opuestas")
                        .clicked()
                    {
                        pick_area = Some(AreaPick::Rect);
                    }
                    if ui
                        .small_button("Contorno")
                        .on_hover_text(
                            "Clic en cada esquina; clic en la primera o Intro para cerrar",
                        )
                        .clicked()
                    {
                        pick_area = Some(AreaPick::Free(Vec::new()));
                    }
                    if ui
                        .small_button("Parcela o manzana")
                        .on_hover_text("Clic dentro de una parcela, o de una manzana entre calles")
                        .clicked()
                    {
                        pick_area = Some(AreaPick::Plot);
                    }
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
        if let Some(how) = pick_area {
            self.set_tool(Tool::Arrow);
            self.say(match how {
                AreaPick::Plot => "Clic dentro de una parcela o de una manzana.",
                AreaPick::Free(_) => "Clic en cada esquina de la zona.",
                _ => "Clic en una esquina de la zona.",
            });
            self.replace.picking = Some(how);
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
