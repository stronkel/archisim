//! Modular boxes (M1): the tool (a click puts a box with its middle under
//! the cursor; near another box it goes against it, a joint away, flush
//! with one of its ends; Space turns it a quarter) and their properties
//! (mark, kind, size, the state of each side, its chassis, its weight and
//! lifting, what it holds and what to look at).

use archisim_core::{
    BoxFill, BoxId, Model, NewBox, Point2, SideState, SlabFunction, SlabTypeId, WallTypeId,
    SIDE_NAMES,
};
use eframe::egui::{self, ComboBox, Id, Ui};

use super::props::{self, group, row, text_row};
use super::widgets::SwitchExt;
use super::*;

/// How far from a place against another box a click still goes to it.
pub const BOX_REACH: f64 = 1.5;

/// What the tool makes.
#[derive(Debug, Clone, PartialEq)]
pub struct BoxSettings {
    pub length: f64,
    pub width: f64,
    pub height: f64,
    pub bottom: f64,
    /// Direction of its length, in radians (Space turns it).
    pub angle: f64,
    pub kind: String,
    /// Fill it with its envelope when it is made.
    pub fill: bool,
    pub wall_type: Option<WallTypeId>,
    pub floor_type: Option<SlabTypeId>,
    pub roof_type: Option<SlabTypeId>,
    pub ceiling_type: Option<SlabTypeId>,
    pub ceiling_height: f64,
    /// How the work on site under the boxes is made (M10).
    pub base: archisim_core::BaseOptions,
}

impl BoxSettings {
    /// The usual box, filled with the first types of each kind.
    pub fn for_model(model: &Model) -> Self {
        let slab = |f: SlabFunction| model.slab_types().find(|t| t.function == f).map(|t| t.id);
        BoxSettings {
            length: archisim_core::BOX_LENGTH,
            width: archisim_core::BOX_WIDTH,
            height: archisim_core::BOX_HEIGHT,
            bottom: archisim_core::BOX_BOTTOM,
            angle: 0.0,
            kind: String::new(),
            fill: true,
            wall_type: model
                .wall_types()
                .find(|t| t.function == archisim_core::WallFunction::Exterior)
                .map(|t| t.id),
            floor_type: slab(SlabFunction::Floor),
            roof_type: slab(SlabFunction::Roof),
            ceiling_type: slab(SlabFunction::Ceiling),
            ceiling_height: 2.93,
            base: archisim_core::BaseOptions::default(),
        }
    }

    pub fn box_fill(&self) -> BoxFill {
        BoxFill {
            wall_type: self.wall_type,
            floor_type: self.floor_type,
            roof_type: self.roof_type,
            ceiling_type: self.ceiling_type,
            ceiling_height: self.ceiling_height,
            parapet: archisim_core::BOX_PARAPET,
        }
    }
}

/// The box the tool would put with its middle at `p`: against the box
/// beside it when one is near.
pub fn box_at(model: &Model, level: archisim_core::LevelId, s: &BoxSettings, p: Point2) -> NewBox {
    let along = Point2::new(s.angle.cos(), s.angle.sin());
    let across = along.perp_left();
    let origin = p - along * (s.length / 2.0) - across * (s.width / 2.0);
    let mut spec = NewBox {
        mark: None,
        kind: s.kind.clone(),
        level,
        origin,
        angle: s.angle,
        length: s.length,
        width: s.width,
        bottom: s.bottom,
        height: s.height,
        sides: [SideState::Free; 4],
        chassis: archisim_core::Chassis::default(),
        progress: archisim_core::BoxProgress::default(),
    };
    if let Some(at) = model.box_snap(&spec, BOX_REACH) {
        spec.origin = at;
    }
    spec
}

impl ArchiSimApp {
    pub(super) fn box_settings(&mut self) -> &mut BoxSettings {
        if self.box_settings.is_none() {
            self.box_settings = Some(BoxSettings::for_model(self.ed.model()));
        }
        self.box_settings.as_mut().expect("just set")
    }

    /// Space: the box turns a quarter.
    pub(super) fn box_turn(&mut self) {
        let s = self.box_settings();
        s.angle = crate::editor::normal_angle(s.angle + std::f64::consts::FRAC_PI_2);
    }

    pub(super) fn box_would(&mut self, p: Point2) -> NewBox {
        let level = self.ed.level;
        let s = self.box_settings().clone();
        box_at(self.ed.model(), level, &s, p)
    }

    pub(super) fn box_click(&mut self, p: Point2) {
        let s = self.box_settings().clone();
        let spec = self.box_would(p);
        let mut id = None;
        let res = self.ed.transact("Caja modular", |m| {
            // The steel its chassis is drawn with.
            archisim_library::frame_material(m, archisim_core::FrameMaterial::Steel);
            archisim_library::joint_materials(m);
            let b = m.add_box(spec)?;
            m.set_box_sides_from_neighbours(b)?;
            if s.fill {
                m.fill_box(b, s.box_fill())?;
            }
            id = Some(b);
            Ok(())
        });
        match (res, id) {
            (Ok(()), Some(id)) => {
                self.ed.selection = [Element::Box(id)].into_iter().collect();
                let mark = self
                    .ed
                    .model()
                    .modular_box(id)
                    .map_or(String::new(), |b| b.mark.clone());
                self.say(format!("Caja {mark} colocada."));
            }
            (Err(e), _) => self.complain(format!("No se puede poner la caja: {e}.")),
            _ => {}
        }
    }

    pub(super) fn box_tool_properties(&mut self, ui: &mut Ui) {
        props::title(ui, "Herramienta Caja · lo que vas a poner");
        let mut s = self.box_settings().clone();
        group(ui, "tool-box", "Caja modular", |ui| {
            props::note(
                ui,
                "Un clic pone la caja con su centro en el cursor; cerca de otra caja se pega a ella, a 2 cm de junta, y la que ya estaba lleva el muro de la junta. Espacio la gira.",
            );
            row(ui, "Tipo de caja", "Qué caja del sistema es", |ui, w| {
                ui.add(egui::TextEdit::singleline(&mut s.kind).desired_width(w - 8.0));
            });
            sizes(
                ui,
                "tool-box",
                &mut s.length,
                &mut s.width,
                &mut s.height,
                &mut s.bottom,
            );
            row(
                ui,
                "Giro",
                "Dirección del largo; Espacio la gira 90°",
                |ui, w| {
                    if let Some(v) = super::structure_ui::angle_field(
                        ui,
                        Id::unique("tool-box-angle"),
                        s.angle,
                        w - 8.0,
                    ) {
                        s.angle = v;
                    }
                },
            );
        });
        group(ui, "tool-box-fill", "Envolvente", |ui| {
            ui.switch(
                &mut s.fill,
                "Ponerle muros, suelo, cubierta y techo al colocarla",
            );
            if s.fill {
                let model = self.ed.model();
                fill_rows(ui, model, "tool-box", &mut s);
            }
        });
        *self.box_settings() = s;
        self.boxes_on_floors_group(ui);
        self.catalogue_group(ui);
    }

    /// A house drawn without boxes (imported from another program): one
    /// box over each floor of a box's size, with its sides read from the
    /// walls at the joints.
    fn boxes_on_floors_group(&mut self, ui: &mut Ui) {
        let floors = self.ed.model().box_floor_slabs();
        let mut go = false;
        group(ui, "tool-box-floors", "Cajas sobre lo dibujado", |ui| {
            props::note(
                ui,
                "Para una vivienda importada (de Revit o ArchiCAD) sin cajas: una caja sobre cada suelo del tamaño de una caja, llevada al largo y ancho de arriba si cabe, con sus lados leídos de los muros de las juntas (del muro, abierto o libre).",
            );
            if floors.is_empty() {
                props::note(ui, "No hay suelos del tamaño de una caja sin caja encima.");
                return;
            }
            go = ui
                .button(format!("Poner {} cajas sobre los suelos", floors.len()))
                .clicked();
        });
        if go {
            let module = (self.box_settings().length, self.box_settings().width);
            let res = self.ed.transact("Cajas sobre los suelos", move |m| {
                archisim_library::frame_material(m, archisim_core::FrameMaterial::Steel);
                archisim_library::joint_materials(m);
                let ids = m.boxes_on_floors(&floors, Some(module))?;
                m.box_sides_from_walls(&ids)?;
                Ok(ids)
            });
            match res {
                Ok(ids) => {
                    let issues: usize = ids
                        .iter()
                        .map(|id| {
                            self.ed
                                .model()
                                .box_issues(*id)
                                .iter()
                                .filter(|i| i.is_error())
                                .count()
                        })
                        .sum();
                    self.say(format!(
                        "{} cajas puestas{}.",
                        ids.len(),
                        if issues == 0 {
                            String::new()
                        } else {
                            format!(": {issues} avisos que mirar en sus propiedades")
                        }
                    ));
                }
                Err(e) => self.complain(format!("No se pueden poner las cajas: {e}.")),
            }
        }
    }

    pub(super) fn box_properties(&mut self, ui: &mut Ui, id: BoxId) {
        let Some(b) = self.ed.model().modular_box(id).cloned() else {
            return;
        };
        props::title(ui, &format!("Caja modular · {}", b.mark));
        let mut n = b.clone();
        group(ui, "box", "Caja", |ui| {
            row(ui, "Marca", "En los planos y en el taller", |ui, w| {
                ui.add(egui::TextEdit::singleline(&mut n.mark).desired_width(w - 8.0));
            });
            row(ui, "Tipo de caja", "Qué caja del sistema es", |ui, w| {
                ui.add(egui::TextEdit::singleline(&mut n.kind).desired_width(w - 8.0));
            });
            sizes(
                ui,
                &format!("box-{}", id.0),
                &mut n.length,
                &mut n.width,
                &mut n.height,
                &mut n.bottom,
            );
            row(ui, "Giro", "Dirección del largo", |ui, w| {
                if let Some(v) = super::structure_ui::angle_field(
                    ui,
                    Id::unique(("box-angle", id)),
                    n.angle,
                    w - 8.0,
                ) {
                    n.angle = v;
                }
            });
            text_row(
                ui,
                "Superficie (m²)",
                "",
                &format!("{:.2}", b.area()).replace('.', ","),
            );
        });
        group(ui, "box-sides", "Lados", |ui| {
            props::note(
                ui,
                "A y C son los lados largos (A desde el origen), B y D los testeros. Cada junta tiene dueño: una caja lleva el muro y la otra queda abierta.",
            );
            for (i, (name, side)) in SIDE_NAMES.iter().zip(n.sides.iter_mut()).enumerate() {
                row(ui, &format!("Lado {name}"), "", |ui, w| {
                    ComboBox::from_id_salt(("box-side", id, i))
                        .width(w - 8.0)
                        .selected_text(side.label())
                        .show_ui(ui, |ui| {
                            for st in SideState::ALL {
                                ui.selectable_value(side, st, st.label());
                            }
                        });
                });
            }
            if ui.button("Ajustar a las cajas de al lado").clicked() {
                let res = self
                    .ed
                    .transact("Lados de la caja", |m| m.set_box_sides_from_neighbours(id));
                if let Err(e) = res {
                    self.complain(format!("No se pueden ajustar los lados: {e}."));
                }
            }
        });
        progress_group(ui, id, &mut n.progress);
        chassis_group(ui, id, &mut n.chassis);
        self.lift_group(ui, id);
        self.crane_group(ui, id);
        self.base_group(ui, id);
        let model = self.ed.model();
        let held = model.box_members(id);
        group(ui, "box-held", "Lo que lleva", |ui| {
            for (name, count) in [
                ("Muros", held.walls.len()),
                ("Puertas y ventanas", held.openings.len()),
                ("Forjados", held.slabs.len()),
                ("Techos", held.ceilings.len()),
                ("Pilares y vigas", held.columns.len() + held.beams.len()),
                ("Objetos", held.objects.len()),
            ] {
                text_row(ui, name, "", &count.to_string());
            }
            props::note(
                ui,
                "Lo que está dentro de la caja, o en un lado que no está abierto, es de la caja: se mueve, se copia y se gira con ella.",
            );
        });
        let mut fill_now = None;
        if held.walls.is_empty() && held.slabs.is_empty() {
            let mut s = self.box_settings().clone();
            let model = self.ed.model();
            group(ui, "box-fill", "Envolvente", |ui| {
                fill_rows(ui, model, &format!("box-{}", id.0), &mut s);
                if ui.button("Ponerle la envolvente").clicked() {
                    fill_now = Some(s.box_fill());
                }
            });
            *self.box_settings() = s;
        }
        let issues = self.ed.model().box_issues(id);
        group(ui, "box-check", "Comprobación", |ui| {
            if issues.is_empty() {
                props::note(
                    ui,
                    "Nada que señalar: cabe en el camión y sus juntas tienen dueño.",
                );
            }
            for i in &issues {
                let mark = if i.is_error() { "✗" } else { "·" };
                props::note(ui, &format!("{mark} {}", i.text(self.ed.model())));
            }
        });
        ui.add_space(4.0);
        if ui
            .button("Guardar como caja tipo…")
            .on_hover_text("La caja y lo que lleva, como caja tipo del catálogo (en diseño), en la carpeta «Cajas tipo» de la biblioteca")
            .clicked()
        {
            self.save_box_type(id);
        }
        if let Some(fill) = fill_now {
            let res = self
                .ed
                .transact("Envolvente de la caja", |m| m.fill_box(id, fill).map(drop));
            if let Err(e) = res {
                self.complain(format!("No se puede poner la envolvente: {e}."));
            }
        }
        if n != b {
            let res = self
                .ed
                .transact("Caja modular", |m| m.edit_box(id, |x| *x = n));
            if let Err(e) = res {
                self.complain(format!("No se puede cambiar la caja: {e}."));
            }
        }
    }
}

impl ArchiSimApp {
    /// What the box weighs, how it hangs and when it goes up.
    fn lift_group(&mut self, ui: &mut Ui, id: BoxId) {
        let model = self.ed.model();
        let check = archisim_struct::boxes::lift_check(model, id);
        let order = archisim_struct::boxes::assembly_order(model);
        let n = |v: f64, d: usize| format!("{v:.d$}").replace('.', ",");
        group(ui, "box-lift", "Peso e izado", |ui| {
            let Some(c) = &check else {
                props::note(ui, "Está vacía: no hay nada que pesar.");
                return;
            };
            let w = &c.weight;
            text_row(
                ui,
                "Peso (t)",
                "Chasis, muros, forjados, techos y carpinterías",
                &n(w.tonnes(), 2),
            );
            let chassis: f64 = w
                .parts
                .iter()
                .filter(|p| p.what == "Chasis")
                .map(|p| p.kg)
                .sum();
            text_row(ui, "Chasis (kg)", "", &n(chassis, 0));
            text_row(
                ui,
                "Centro de gravedad (m)",
                "A lo largo, de través y en altura, desde el origen de la caja y la planta",
                &format!(
                    "{} · {} · {}",
                    n(w.centre[0], 2),
                    n(w.centre[1], 2),
                    n(w.centre[2], 2)
                ),
            );
            let pulls: Vec<String> = c.points.iter().map(|(_, f)| n(*f, 1)).collect();
            text_row(
                ui,
                "Por punto (kN)",
                "Eslingas verticales, con balancín",
                &pulls.join(" · "),
            );
            text_row(
                ui,
                "Borde del chasis",
                "Flexión del borde largo al colgarla (peso × 1,35 × 1,20)",
                &format!(
                    "{} de {} kNm · {} %",
                    n(c.moment, 1),
                    n(c.resistance, 1),
                    (c.usage() * 100.0).round()
                ),
            );
            text_row(
                ui,
                "Flecha",
                "Colgada, sin mayorar; límite luz / 250",
                &format!("{} de {} mm", n(c.deflection, 1), n(c.deflection_limit, 1)),
            );
            if c.passes() {
                props::note(ui, "· El chasis aguanta el izado.");
            } else {
                props::note(
                    ui,
                    "✗ El borde del chasis no aguanta el izado: cuélgala de seis puntos, sube el tubo del marco o cuenta con el muro como viga (a comprobar).",
                );
            }
        });
        if let Some(pos) = order.iter().position(|b| *b == id) {
            props::note(
                ui,
                &format!(
                    "Orden de montaje: {}ª de {} (por plantas, de una esquina y siguiendo las que tocan a las ya puestas).",
                    pos + 1,
                    order.len()
                ),
            );
        }
    }
}

/// Where a box is between the drawing board and the site (M9).
fn progress_group(ui: &mut Ui, id: BoxId, p: &mut archisim_core::BoxProgress) {
    group(ui, "box-progress", "Fabricación", |ui| {
        row(ui, "Estado", "Dónde está la caja", |ui, w| {
            ComboBox::from_id_salt(("box-stage", id))
                .width(w - 8.0)
                .selected_text(p.stage.label())
                .show_ui(ui, |ui| {
                    for st in archisim_core::BoxStage::ALL {
                        ui.selectable_value(&mut p.stage, st, st.label());
                    }
                });
        });
        if p.stage == archisim_core::BoxStage::Factory {
            row(ui, "Puesto", "En qué puesto de la fábrica", |ui, w| {
                ui.add(egui::TextEdit::singleline(&mut p.station).desired_width(w - 8.0));
            });
        }
        row(ui, "Fecha", "Desde cuándo está así", |ui, w| {
            ui.horizontal(|ui| {
                ui.add(egui::TextEdit::singleline(&mut p.date).desired_width(w - 60.0));
                if ui.button("Hoy").clicked() {
                    p.date = super::catalogue_ui::today();
                }
            });
        });
    });
}

impl ArchiSimApp {
    /// The crane of the site and this box's lift.
    fn crane_group(&mut self, ui: &mut Ui, id: BoxId) {
        let model = self.ed.model();
        let mut site = model.modular_site();
        let plan = archisim_struct::boxes::crane_plan(model);
        let n = |v: f64, d: usize| format!("{v:.d$}").replace('.', ",");
        let before = site.clone();
        group(ui, "box-crane", "Plan de grúa", |ui| {
            props::note(ui, &site.crane_chart.name);
            if let Some(p) = &plan {
                if let Some(l) = p.lifts.iter().find(|l| l.id == id) {
                    text_row(
                        ui,
                        "Esta caja",
                        "Carga con el aparejo, radio desde la grúa y qué parte de lo que admite allí usa",
                        &format!(
                            "{} t a {} m · {} %",
                            n(l.load, 1),
                            n(l.radius, 1),
                            (l.usage() * 100.0).min(999.0).round()
                        ),
                    );
                    if !l.passes() {
                        props::note(
                            ui,
                            "✗ La grúa no la levanta a esa distancia: acércala o elige una grúa mayor.",
                        );
                    }
                }
            }
            match site.crane {
                Some(mut c) => {
                    row(ui, "Grúa X (mm)", "Centro de giro", |ui, w| {
                        if let Some(x) = length_field(ui, Id::unique(("crane-x", id)), c.x, w - 8.0)
                        {
                            c.x = x;
                        }
                    });
                    row(ui, "Grúa Y (mm)", "", |ui, w| {
                        if let Some(y) = length_field(ui, Id::unique(("crane-y", id)), c.y, w - 8.0)
                        {
                            c.y = y;
                        }
                    });
                    site.crane = Some(c);
                    if ui.button("Quitar la grúa").clicked() {
                        site.crane = None;
                    }
                }
                None => {
                    props::note(
                        ui,
                        "Sin grúa colocada: se calcula con ella en el centro de las cajas.",
                    );
                    if ui.button("Colocarla ahí").clicked() {
                        site.crane = plan.as_ref().map(|p| p.at);
                    }
                }
            }
            row(
                ui,
                "Aparejo (t)",
                "Balancín, eslingas y grilletes",
                |ui, _| {
                    ui.add(
                        egui::DragValue::new(&mut site.rigging)
                            .range(0.0..=10.0)
                            .speed(0.05)
                            .max_decimals(2)
                            .custom_formatter(|v, _| format!("{v:.2}").replace('.', ","))
                            .custom_parser(|t| t.replace(',', ".").trim().parse().ok()),
                    );
                },
            );
        });
        if site != before {
            let res = self
                .ed
                .transact("Plan de grúa", move |m| m.set_modular_site(site));
            if let Err(e) = res {
                self.complain(format!("No se puede cambiar la grúa: {e}."));
            }
        }
    }

    /// The work on site under the boxes (M10): its sizes, and the button
    /// that makes (or makes again) the slab, the supports and the plinth.
    fn base_group(&mut self, ui: &mut Ui, id: BoxId) {
        let model = self.ed.model();
        let supports = model.box_supports();
        let built = model
            .levels()
            .into_iter()
            .find(|l| l.name == archisim_core::BASE_LEVEL)
            .map(|l| l.id);
        let check = archisim_struct::boxes::base_check(model);
        let mine: Vec<String> = check
            .as_ref()
            .map(|c| {
                c.supports
                    .iter()
                    .filter(|s| s.boxes.contains(&id))
                    .map(|s| format!("{}{}", s.mark, if s.passes() { "" } else { " ✗" }))
                    .collect()
            })
            .unwrap_or_default();
        let n = |v: f64, d: usize| format!("{v:.d$}").replace('.', ",");
        let mut o = self.box_settings().base.clone();
        let mut build = false;
        group(ui, "box-base", "Obra previa", |ui| {
            if supports.is_empty() {
                props::note(ui, "Solo las cajas sobre el terreno llevan apoyos.");
                return;
            }
            text_row(
                ui,
                "Apoyos",
                "Bajo las esquinas y el medio de los lados largos; los de cajas juntas, uno",
                &format!("{} en total", supports.len()),
            );
            if !mine.is_empty() {
                text_row(ui, "Bajo esta", "Sus apoyos", &mine.join(", "));
            }
            if let Some(c) = &check {
                text_row(
                    ui,
                    "Terreno",
                    "Presión bajo la losa y lo que admite (Estructura → Datos del sitio)",
                    &format!(
                        "{} de {} kN/m²{}",
                        n(c.pressure, 1),
                        n(c.bearing, 0),
                        if c.ground_passes() { "" } else { " ✗" }
                    ),
                );
            }
            let field = |ui: &mut Ui, key: &str, label: &str, hint: &str, v: &mut f64| {
                row(ui, label, hint, |ui, w| {
                    if let Some(x) = length_field(ui, Id::unique(("box-base", key)), *v, w - 8.0) {
                        *v = x;
                    }
                });
            };
            field(
                ui,
                "pier-h",
                "Alto de los apoyos (mm)",
                "De la losa a la base del chasis",
                &mut o.pier_height,
            );
            field(
                ui,
                "pier",
                "Pilar (mm)",
                "Lado del pilar bajo una caja",
                &mut o.pier,
            );
            field(
                ui,
                "shared",
                "Pilar compartido (mm)",
                "Bajo dos cajas o más",
                &mut o.shared_pier,
            );
            field(ui, "slab", "Canto de la losa (mm)", "", &mut o.slab);
            field(
                ui,
                "margin",
                "Vuelo de la losa (mm)",
                "Lo que sale por fuera de las cajas",
                &mut o.margin,
            );
            ui.switch(&mut o.plinth, "Zócalo de bloque");
            if o.plinth {
                field(
                    ui,
                    "plinth",
                    "Zócalo (mm)",
                    "Grueso, a haces con las cajas",
                    &mut o.plinth_thickness,
                );
            }
            props::note(
                ui,
                if built.is_some() {
                    "Ya hecha en la planta «Cimentación de las cajas». Volver a hacerla quita la anterior."
                } else {
                    "Se hace en una planta nueva bajo las cajas, «Cimentación de las cajas»."
                },
            );
            build = ui
                .button(if built.is_some() {
                    "Volver a hacer la obra previa"
                } else {
                    "Hacer la obra previa"
                })
                .clicked();
        });
        if o != self.box_settings().base {
            self.box_settings().base = o.clone();
        }
        if build {
            let res = self
                .ed
                .transact("Obra previa", move |m| m.build_box_base(&o));
            match res {
                Ok(made) => self.say(format!(
                    "Obra previa: losa, {} apoyos{}.",
                    made.piers.len(),
                    if made.plinth.is_empty() {
                        String::new()
                    } else {
                        format!(" y {} tramos de zócalo", made.plinth.len())
                    }
                )),
                Err(e) => self.complain(format!("No se puede hacer la obra previa: {e}.")),
            }
        }
    }
}

/// The chassis of a box: its tubes, joists and how it is lifted.
fn chassis_group(ui: &mut Ui, id: BoxId, c: &mut archisim_core::Chassis) {
    group(ui, "box-chassis", "Chasis", |ui| {
        props::note(
            ui,
            "Tubo soldado: marco de suelo y de techo con sus viguetas, pilares en las esquinas (y en el medio de los lados largos abiertos) y, en los lados libres, el peto: montantes sobre el marco de techo y la coronación arriba.",
        );
        let tubes = archisim_core::box_tubes();
        for (name, hint, slot) in [
            ("Marco de suelo", "Bordes del suelo", &mut c.edge),
            ("Pilares", "De esquina e intermedios", &mut c.post),
            ("Viguetas de suelo", "Y la viga central", &mut c.joist),
            ("Marco de techo", "En el plano del techo", &mut c.roof_edge),
            ("Viguetas de techo", "Y la viga central", &mut c.roof_joist),
            (
                "Coronación",
                "Arriba del peto, en los lados libres",
                &mut c.crown,
            ),
            (
                "Montantes del peto",
                "Entre el marco de techo y la coronación",
                &mut c.parapet_post,
            ),
        ] {
            row(ui, name, hint, |ui, w| {
                ComboBox::from_id_salt(("box-tube", id, name))
                    .width(w - 8.0)
                    .selected_text(slot.label())
                    .show_ui(ui, |ui| {
                        let mut list = tubes.clone();
                        if !list.contains(slot) {
                            list.push(*slot);
                        }
                        for t in list {
                            ui.selectable_value(slot, t, t.label());
                        }
                    });
            });
        }
        row(ui, "Separación (mm)", "Entre viguetas, a ejes", |ui, w| {
            if let Some(x) = length_field(ui, Id::unique(("box-spacing", id)), c.spacing, w - 8.0) {
                c.spacing = x;
            }
        });
        row(
            ui,
            "Montantes del peto (mm)",
            "Separación máxima entre ellos",
            |ui, w| {
                if let Some(x) = length_field(
                    ui,
                    Id::unique(("box-parapet-spacing", id)),
                    c.parapet_spacing,
                    w - 8.0,
                ) {
                    c.parapet_spacing = x;
                }
            },
        );
        row(
            ui,
            "Marco de techo (mm)",
            "Su cara de abajo sobre la planta: el plano del techo",
            |ui, w| {
                if let Some(x) =
                    length_field(ui, Id::unique(("box-roof-at", id)), c.roof_at, w - 8.0)
                {
                    c.roof_at = x;
                }
            },
        );
        ui.switch(
            &mut c.middle_beam,
            "Viga central en suelo y techo (dos vanos de viguetas)",
        );
        row(ui, "Izado", "De dónde se cuelga", |ui, w| {
            ComboBox::from_id_salt(("box-lift", id))
                .width(w - 8.0)
                .selected_text(c.lift.label())
                .show_ui(ui, |ui| {
                    for l in archisim_core::LiftPoints::ALL {
                        ui.selectable_value(&mut c.lift, l, l.label());
                    }
                });
        });
    });
}

fn sizes(
    ui: &mut Ui,
    key: &str,
    length: &mut f64,
    width: &mut f64,
    height: &mut f64,
    bottom: &mut f64,
) {
    for (name, hint, v) in [
        (
            "Largo (mm)",
            "Lados A y C, por la cara exterior de la estructura",
            length,
        ),
        ("Ancho (mm)", "Testeros B y D", width),
        (
            "Alto (mm)",
            "Del chasis de suelo a lo alto de la cubierta",
            height,
        ),
        ("Base (mm)", "Bajo el suelo terminado de la planta", bottom),
    ] {
        row(ui, name, hint, |ui, w| {
            if let Some(x) = length_field(ui, Id::unique((key, name)), *v, w - 8.0) {
                *v = x;
            }
        });
    }
}

/// The types to fill a box with.
fn fill_rows(ui: &mut Ui, model: &Model, key: &str, s: &mut BoxSettings) {
    let none = "— nada —";
    row(
        ui,
        "Muros",
        "En los lados libres y en los que llevan la junta",
        |ui, w| {
            let name = s
                .wall_type
                .and_then(|t| model.wall_type(t))
                .map_or(none.to_string(), |t| t.name.clone());
            ComboBox::from_id_salt((key, "walls"))
                .width(w - 8.0)
                .selected_text(name)
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut s.wall_type, None, none);
                    for t in model.wall_types() {
                        ui.selectable_value(&mut s.wall_type, Some(t.id), &t.name);
                    }
                });
        },
    );
    for (name, hint, wanted, slot) in [
        (
            "Suelo",
            "Arriba a la cota de la planta",
            SlabFunction::Floor,
            &mut s.floor_type,
        ),
        (
            "Cubierta",
            "Arriba a lo alto de la caja",
            SlabFunction::Roof,
            &mut s.roof_type,
        ),
        (
            "Falso techo",
            "",
            SlabFunction::Ceiling,
            &mut s.ceiling_type,
        ),
    ] {
        row(ui, name, hint, |ui, w| {
            let current = slot
                .and_then(|t| model.slab_type(t))
                .map_or(none.to_string(), |t| t.name.clone());
            ComboBox::from_id_salt((key, name))
                .width(w - 8.0)
                .selected_text(current)
                .show_ui(ui, |ui| {
                    ui.selectable_value(slot, None, none);
                    for t in model.slab_types().filter(|t| {
                        (t.function == SlabFunction::Ceiling) == (wanted == SlabFunction::Ceiling)
                    }) {
                        ui.selectable_value(slot, Some(t.id), &t.name);
                    }
                });
        });
    }
    if s.ceiling_type.is_some() {
        row(ui, "Techo a (mm)", "Cara vista sobre la planta", |ui, w| {
            if let Some(x) =
                length_field(ui, Id::unique((key, "ceiling")), s.ceiling_height, w - 8.0)
            {
                s.ceiling_height = x;
            }
        });
    }
}

impl ArchiSimApp {
    /// The workshop drawings of the boxes (M9), A3, to a PDF.
    pub(super) fn export_box_drawings(&mut self) {
        let model = self.ed.shown();
        let pages = archisim_plan::box_sheets::pages(&model);
        if pages.is_empty() {
            self.complain("El proyecto no tiene cajas modulares.".to_string());
            return;
        }
        let mut d = rfd::FileDialog::new()
            .set_title("Planos de taller de las cajas (PDF)")
            .add_filter("PDF", &["pdf"])
            .set_file_name(format!("{}-taller.pdf", self.file_name_stem()));
        if let Some(dir) = self.start_dir() {
            d = d.set_directory(dir);
        }
        let Some(path) = d.save_file() else {
            return;
        };
        let codes = archisim_plan::box_sheets::codes(pages.len());
        let sheets: Vec<(String, String)> = pages.into_iter().zip(codes).collect();
        let r = crate::documents::pdf_of(&sheets, "Planos de taller de las cajas")
            .and_then(|bytes| std::fs::write(&path, bytes).map_err(|e| e.to_string()));
        match r {
            Ok(()) => self.say(format!(
                "Planos de taller: {} hojas A3 en {}.",
                sheets.len(),
                path.display()
            )),
            Err(e) => self.complain(format!("No se pudo exportar: {e}.")),
        }
    }

    /// The shopping list of the boxes (M14), and their boards piece by
    /// piece beside it, as CSV.
    pub(super) fn export_shopping_list(&mut self) {
        let model = self.ed.shown();
        if model.boxes().next().is_none() {
            self.complain("El proyecto no tiene cajas modulares.".to_string());
            return;
        }
        let mut d = rfd::FileDialog::new()
            .set_title("Lista de compra de las cajas (CSV)")
            .add_filter("CSV", &["csv"])
            .set_file_name(format!("{}-compra.csv", self.file_name_stem()));
        if let Some(dir) = self.start_dir() {
            d = d.set_directory(dir);
        }
        let Some(path) = d.save_file() else {
            return;
        };
        let boards = path.with_file_name(format!(
            "{}-tableros.csv",
            path.file_stem().map_or("cajas".into(), |s| s
                .to_string_lossy()
                .trim_end_matches("-compra")
                .to_string())
        ));
        let r = std::fs::write(&path, archisim_geom::boards::shopping_csv(&model))
            .and_then(|_| std::fs::write(&boards, archisim_geom::boards::boards_csv(&model)));
        match r {
            Ok(()) => self.say(format!(
                "Lista de compra en {} y placas pieza a pieza en {}.",
                path.display(),
                boards.display()
            )),
            Err(e) => self.complain(format!("No se pudo exportar: {e}.")),
        }
    }
}
