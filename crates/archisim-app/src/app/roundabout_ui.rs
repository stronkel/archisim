//! Roundabouts and squares (U3): the tool (a click at the centre, which
//! goes to the junction under it) and their properties.

use archisim_core::{IslandFinish, MaterialId, Model, Point2, RoundaboutId};
use eframe::egui::{self, ComboBox, Id, Ui};

use super::props::{self, group, row, text_row};
use super::widgets::SwitchExt;
use super::*;

/// What the tool makes.
#[derive(Debug, Clone, PartialEq)]
pub struct RoundaboutSettings {
    pub radius: f64,
    pub island: f64,
    pub apron: f64,
    pub plaza: bool,
    pub finish: IslandFinish,
}

impl Default for RoundaboutSettings {
    fn default() -> Self {
        RoundaboutSettings {
            radius: archisim_core::ROUNDABOUT_RADIUS,
            island: archisim_core::ROUNDABOUT_ISLAND,
            apron: archisim_core::ROUNDABOUT_APRON,
            plaza: false,
            finish: IslandFinish::Garden,
        }
    }
}

/// Where a roundabout clicked at `p` goes: the junction it is in (within
/// 12 m), or `p`.
pub fn roundabout_centre(m: &Model, level: archisim_core::LevelId, p: Point2) -> Point2 {
    archisim_geom::crossroads::junctions(m, level)
        .into_iter()
        .filter(|j| j.ring.is_none())
        .map(|j| j.at)
        .filter(|x| x.distance(p) < 12.0)
        .min_by(|a, b| a.distance(p).total_cmp(&b.distance(p)))
        .unwrap_or(p)
}

/// The materials of a roundabout: those of the nearest road, and grass
/// (or the sidewalk's) for the island.
fn materials_near(m: &Model, p: Point2) -> Option<(MaterialId, MaterialId, MaterialId)> {
    let road = m.roads().min_by(|a, b| {
        a.axis[0]
            .distance(p)
            .min(a.axis[a.axis.len() - 1].distance(p))
            .total_cmp(
                &b.axis[0]
                    .distance(p)
                    .min(b.axis[b.axis.len() - 1].distance(p)),
            )
    });
    let green = m
        .materials()
        .find(|x| x.category == "vegetation")
        .map(|x| x.id);
    match road {
        Some(r) => Some((
            r.pavement,
            r.kerb_material,
            r.section.planting.or(green).unwrap_or(r.sidewalk_material),
        )),
        None => {
            let any = m.materials().next()?.id;
            Some((any, any, green.unwrap_or(any)))
        }
    }
}

impl ArchiSimApp {
    pub(super) fn roundabout_click(&mut self, p: Point2) {
        let level = self.ed.level;
        let s = self.roundabout_settings.clone();
        let centre = roundabout_centre(self.ed.model(), level, p);
        let Some((pavement, kerb, island)) = materials_near(self.ed.model(), centre) else {
            self.complain("El proyecto no tiene materiales.");
            return;
        };
        let top = self
            .ed
            .model()
            .roads_on_level(level)
            .map(|r| (r.station_of(centre), r))
            .min_by(|a, b| {
                a.1.axis_at(a.0)
                    .0
                    .distance(centre)
                    .total_cmp(&b.1.axis_at(b.0).0.distance(centre))
            })
            .map_or(0.0, |(s, r)| r.top_at(s));
        let mut id = None;
        let res = self.ed.transact("Glorieta", |m| {
            let name = m.next_roundabout_name();
            id = Some(m.add_roundabout(archisim_core::NewRoundabout {
                name,
                level,
                centre,
                radius: s.radius,
                island: if s.plaza { s.island.min(3.0) } else { s.island },
                apron: if s.plaza { 0.0 } else { s.apron },
                top_offset: top,
                plaza: s.plaza,
                island_finish: s.finish,
                pavement,
                kerb_material: kerb,
                island_material: island,
            })?);
            Ok(())
        });
        match (res, id) {
            (Ok(()), Some(id)) => {
                self.ed.selection = [Element::Roundabout(id)].into_iter().collect();
                self.say("Glorieta colocada.");
            }
            (Err(e), _) => self.complain(format!("No se puede poner la glorieta: {e}.")),
            _ => {}
        }
    }

    pub(super) fn roundabout_tool_properties(&mut self, ui: &mut Ui) {
        props::title(ui, "Herramienta Glorieta · lo que vas a poner");
        let mut s = self.roundabout_settings.clone();
        group(ui, "tool-roundabout", "Glorieta", |ui| {
            props::note(
                ui,
                "Un clic en un cruce pone la glorieta en su centro; las calles que llegan se recortan en su círculo y se unen con su radio de esquina.",
            );
            ui.switch(
                &mut s.plaza,
                "Plaza (pavimentada entera, al nivel de la calle)",
            );
            sizes(
                ui,
                "tool-rb",
                &mut s.radius,
                &mut s.island,
                &mut s.apron,
                s.plaza,
            );
            finish_row(ui, "tool-rb-finish", &mut s.finish);
        });
        self.roundabout_settings = s;
    }

    pub(super) fn roundabout_properties(&mut self, ui: &mut Ui, id: RoundaboutId) {
        let Some(c) = self.ed.model().roundabout(id).cloned() else {
            return;
        };
        props::title(ui, &format!("Glorieta · {}", c.name));
        let mut n = c.clone();
        group(ui, "rb", "Glorieta", |ui| {
            row(ui, "Nombre", "", |ui, w| {
                ui.add(egui::TextEdit::singleline(&mut n.name).desired_width(w - 8.0));
            });
            ui.switch(
                &mut n.plaza,
                "Plaza (pavimentada entera, al nivel de la calle)",
            );
            sizes(
                ui,
                &format!("rb-{}", id.0),
                &mut n.radius,
                &mut n.island,
                &mut n.apron,
                n.plaza,
            );
            finish_row(ui, &format!("rb-finish-{}", id.0), &mut n.island_finish);
            row(ui, "Rasante (mm)", "La calzada sobre la planta", |ui, w| {
                if let Some(v) = length_field(ui, Id::unique(("rb-top", id)), n.top_offset, w - 8.0)
                {
                    n.top_offset = v;
                }
            });
            text_row(
                ui,
                "Anillo (mm)",
                "Ancho de la calzada entre el bordillo y el borde montable de la isleta",
                &fmt_mm(c.ring().max(0.0)),
            );
        });
        group(ui, "rb-mat", "Materiales", |ui| {
            for (i, name) in ["Calzada", "Bordillo y borde montable", "Isleta"]
                .into_iter()
                .enumerate()
            {
                let v = match i {
                    0 => n.pavement,
                    1 => n.kerb_material,
                    _ => n.island_material,
                };
                row(ui, name, "", |ui, w| {
                    let x = urban_ui::material_combo(
                        ui,
                        Id::unique(("rb-mat", id, i)),
                        self.ed.model(),
                        v,
                        w,
                    );
                    match i {
                        0 => n.pavement = x,
                        1 => n.kerb_material = x,
                        _ => n.island_material = x,
                    }
                });
            }
        });
        group(ui, "rb-identity", "Datos de identidad", |ui| {
            text_row(
                ui,
                "Clase IFC",
                "",
                "IfcBuildingElementProxy (las piezas, con el vial: IfcPavement e IfcKerb)",
            );
        });
        if n != c {
            let res = self
                .ed
                .transact("Glorieta", |m| m.edit_roundabout(id, |x| *x = n));
            self.report(res, "Glorieta cambiada.");
        }
    }
}

/// Radius, island and apron.
fn sizes(
    ui: &mut Ui,
    salt: &str,
    radius: &mut f64,
    island: &mut f64,
    apron: &mut f64,
    plaza: bool,
) {
    let rows = [
        ("Radio exterior (mm)", "Hasta el bordillo de fuera", *radius),
        (
            if plaza {
                "Radio de la fuente o jardinera (mm)"
            } else {
                "Radio de la isleta (mm)"
            },
            "0: sin isleta",
            *island,
        ),
        (
            "Borde montable (mm)",
            "Anillo de adoquín alrededor de la isleta, para camiones",
            *apron,
        ),
    ];
    for (i, (name, hint, value)) in rows.into_iter().enumerate() {
        if plaza && i == 2 {
            continue;
        }
        row(ui, name, hint, |ui, w| {
            if let Some(v) = length_field(ui, Id::unique((salt, i)), value, w - 8.0) {
                let v = v.max(0.0);
                match i {
                    0 => *radius = v.max(3.0),
                    1 => *island = v,
                    _ => *apron = v,
                }
            }
        });
    }
}

fn finish_row(ui: &mut Ui, salt: &str, finish: &mut IslandFinish) {
    row(ui, "Isleta", "", |ui, w| {
        ComboBox::from_id_salt(salt)
            .width(w - 8.0)
            .selected_text(finish.name())
            .show_ui(ui, |ui| {
                for f in IslandFinish::ALL {
                    ui.selectable_value(finish, f, f.name());
                }
            });
    });
}
