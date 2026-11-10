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
        self.catalogue_group(ui);
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
        chassis_group(ui, id, &mut n.chassis);
        self.lift_group(ui, id);
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
                "Flecha (mm)",
                "Colgada, sin mayorar; límite luz / 250",
                &format!("{} de {}", n(c.deflection, 1), n(c.deflection_limit, 1)),
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
