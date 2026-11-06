//! One-way and waffle floors (`docs/ESTRUCTURA.md` §6): the «Sistema de
//! forjado» group of a slab type in Tipos y materiales, and the direction
//! of the joists of a slab in its properties.

use super::egui::{self, Ui};
use archisim_core::{FloorKind, FloorSystem, Infill, Joist, Model, SlabId, SlabTypeId};

use super::props;
use super::widgets::SwitchExt;
use super::ArchiSimApp;

/// Centimetres as a choice.
fn cm_choice(ui: &mut Ui, salt: (&str, u64), w: f32, now: f64, list: &[u32]) -> f64 {
    let v = (now * 100.0).round() as u32;
    let mut opts: Vec<(u32, String)> = list.iter().map(|c| (*c, format!("{c} cm"))).collect();
    if !list.contains(&v) {
        opts.push((v, format!("{v} cm")));
    }
    let refs: Vec<(u32, &str)> = opts.iter().map(|(c, s)| (*c, s.as_str())).collect();
    f64::from(props::choice(ui, salt, w, v, &refs)) / 100.0
}

/// The group for slab type `id`. Returns the new system (or `Some(None)`
/// for a solid slab) when something changed.
pub fn floor_system_group(
    ui: &mut Ui,
    model: &Model,
    id: SlabTypeId,
) -> Option<Option<FloorSystem>> {
    let mut out = None;
    let now = model.floor_system(id).copied();
    props::group(
        ui,
        &format!("types-floor-{}", id.0),
        "Sistema de forjado",
        |ui| {
            let kind = now.map(|f| f.kind);
            props::row(
            ui,
            "Forjado",
            "Cómo se construye la capa de estructura: losa maciza, unidireccional de viguetas y bovedillas o reticular. El cálculo de la estructura lo calcula según sea",
            |ui, w| {
                let v = props::choice(
                    ui,
                    ("floor-kind", id.0),
                    w,
                    kind,
                    &[
                        (None, "Losa maciza"),
                        (Some(FloorKind::OneWay), FloorKind::OneWay.label()),
                        (Some(FloorKind::Waffle), FloorKind::Waffle.label()),
                    ],
                );
                if v != kind {
                    out = Some(match v {
                        None => None,
                        Some(FloorKind::OneWay) => Some(FloorSystem::one_way()),
                        Some(FloorKind::Waffle) => Some(FloorSystem::waffle()),
                    });
                }
            },
        );
            if out.is_some() {
                return;
            }
            let Some(f) = now else {
                return;
            };
            let mut next = f;
            if f.kind == FloorKind::OneWay {
                props::row(ui, "Viguetas", "", |ui, w| {
                    next.joist = props::choice(
                        ui,
                        ("floor-joist", id.0),
                        w,
                        f.joist,
                        &[
                            (Joist::Precast, Joist::Precast.label()),
                            (Joist::InSitu, Joist::InSitu.label()),
                        ],
                    );
                });
            }
            props::row(
                ui,
                "Entrevigado",
                "Lo que rellena entre nervios",
                |ui, w| {
                    let opts: Vec<(Infill, &str)> = Infill::ALL
                        .iter()
                        .filter(|i| f.kind == FloorKind::Waffle || **i != Infill::Recoverable)
                        .map(|i| (*i, i.label()))
                        .collect();
                    next.infill = props::choice(ui, ("floor-infill", id.0), w, f.infill, &opts);
                },
            );
            props::row(
                ui,
                "Canto de la pieza",
                "Sin la capa de compresión",
                |ui, w| {
                    let h = f.depth - f.topping;
                    let v = cm_choice(
                        ui,
                        ("floor-h", id.0),
                        w,
                        h,
                        &[16, 18, 20, 22, 25, 28, 30, 35, 40],
                    );
                    next.depth = v + f.topping;
                },
            );
            props::row(ui, "Capa de compresión", "", |ui, w| {
                let v = cm_choice(ui, ("floor-t", id.0), w, f.topping, &[4, 5, 6, 7, 8, 10]);
                next.depth = f.depth - f.topping + v;
                next.topping = v;
            });
            let spacings: &[u32] = if f.kind == FloorKind::OneWay {
                &[60, 70, 72, 80]
            } else {
                &[72, 75, 80, 82, 85, 100]
            };
            props::row(ui, "Intereje", "Entre ejes de nervios", |ui, w| {
                next.spacing = cm_choice(ui, ("floor-s", id.0), w, f.spacing, spacings);
            });
            props::row(ui, "Ancho del nervio", "", |ui, w| {
                next.rib = cm_choice(ui, ("floor-r", id.0), w, f.rib, &[10, 11, 12, 14, 15, 16]);
            });
            ui.label(
            egui::RichText::new(format!(
                "{} · {:.2} kN/m² · {:.3} m³/m² de hormigón. Pon el espesor de la capa de estructura del tipo igual al canto total ({:.0} cm).",
                f.label(),
                f.weight(),
                f.concrete(),
                f.depth * 100.0
            ))
            .small()
            .color(super::theme::Palette::current(ui.ctx()).muted),
        );
            if next != f && next.is_valid() {
                out = Some(Some(next));
            }
        },
    );
    out
}

impl ArchiSimApp {
    /// The direction of the joists of a one-way floor.
    pub(super) fn slab_joists_group(&mut self, ui: &mut Ui, id: SlabId) {
        let model = self.ed.model();
        let Some(s) = model.slab(id) else {
            return;
        };
        let Some(f) = model.floor_system(s.slab_type).copied() else {
            return;
        };
        if f.kind != FloorKind::OneWay {
            return;
        }
        let given = model.slab_joists(id);
        let dir = model.joist_direction(id);
        props::group(ui, "slab-joists", "Viguetas", |ui| {
            let mut auto = given.is_none();
            props::row(
                ui,
                "Dirección automática",
                "Las viguetas van por el lado corto del forjado; quítalo para darles la dirección a mano",
                |ui, _| {
                    if ui.switch(&mut auto, "").changed() {
                        let a = (!auto).then(|| dir.map_or(0.0, |d| d.y.atan2(d.x)));
                        let r = self.ed.transact("Dirección de las viguetas", |m| m.set_slab_joists(id, a));
                        self.report(r, "Dirección de las viguetas cambiada.");
                    }
                },
            );
            let deg = dir.map_or(0.0, |d| d.y.atan2(d.x).to_degrees().rem_euclid(180.0));
            props::row(ui, "Ángulo", "Desde el eje x, en grados", |ui, w| {
                ui.horizontal(|ui| {
                    ui.label(format!("{deg:.0}°"));
                    if ui
                        .add_sized([(w - 60.0).max(60.0), 20.0], egui::Button::new("Girar 90°"))
                        .clicked()
                    {
                        let a = (deg + 90.0).to_radians();
                        let r = self
                            .ed
                            .transact("Girar las viguetas", |m| m.set_slab_joists(id, Some(a)));
                        self.report(r, "Viguetas giradas 90°.");
                    }
                });
            });
            ui.label(
                egui::RichText::new(format!(
                    "{} {} · el cálculo reparte su carga a los muros y vigas que cruzan las viguetas.",
                    f.joist.label(),
                    f.label()
                ))
                .small()
                .color(super::theme::Palette::current(ui.ctx()).muted),
            );
        });
    }
}
