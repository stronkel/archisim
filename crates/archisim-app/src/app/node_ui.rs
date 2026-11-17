//! The nodes of the streets (N1, `docs/TERRITORIO.md` §2): the tool Nudo
//! picks the crossing under a click and its properties change it — what it
//! is (crossing, raised, shared surface, no node), how far each arm ends
//! from it, and each corner's radius, shape and sidewalk.

use archisim_core::{CornerShape, Model, NodeKey, NodeKind, NodeSettings, Point2};
use archisim_geom::crossroads::{self, Arm, Junction};
use eframe::egui::{self, Color32, ComboBox, Id, Rect, Shape, Stroke, Ui};

use super::props::{self, group, row};
use super::widgets::SwitchExt;
use super::*;

/// How far from a node's centre a click still picks it, metres.
const PICK_REACH: f64 = 15.0;

/// Where an arm goes, as a point of the compass.
fn heading(a: &Arm) -> &'static str {
    // 0 is east, counter-clockwise; north is +y.
    let t = a.dir.y.atan2(a.dir.x).to_degrees().rem_euclid(360.0);
    const NAMES: [&str; 8] = [
        "este", "noreste", "norte", "noroeste", "oeste", "suroeste", "sur", "sureste",
    ];
    NAMES[(((t + 22.5) / 45.0).floor() as usize) % 8]
}

/// An arm for Antonio: its street and where it goes.
fn arm_name(m: &Model, a: &Arm) -> String {
    let name = m.road(a.road).map_or("calle", |r| r.name.as_str());
    format!("{name}, hacia el {}", heading(a))
}

/// The names of a node's streets.
pub fn node_title(m: &Model, key: &NodeKey) -> String {
    let mut names: Vec<String> = key
        .road_ids()
        .filter_map(|r| m.road(r).map(|x| x.name.clone()))
        .collect();
    names.dedup();
    match names.len() {
        0 => "Nudo".into(),
        1 => format!("Nudo de {}", names[0]),
        _ => {
            let last = names.pop().unwrap_or_default();
            format!("Nudo de {} y {last}", names.join(", "))
        }
    }
}

impl ArchiSimApp {
    /// A click with the tool Nudo: picks the node under it.
    pub(super) fn node_click(&mut self, p: Point2) {
        let level = self.ed.level;
        match crossroads::node_at(self.ed.model(), level, p, PICK_REACH) {
            Some(j) => {
                let key = j.key.clone();
                if let Some(k) = &key {
                    let title = node_title(self.ed.model(), k);
                    self.say(format!("{title}: cámbialo en Propiedades."));
                }
                self.node_pick = key;
            }
            None => {
                self.node_pick = None;
                self.say("Aquí no se encuentran calles.");
            }
        }
    }

    /// The node picked, if it is still there.
    fn picked_node(&self) -> Option<(NodeKey, Junction)> {
        let key = self.node_pick.clone()?;
        let j = crossroads::node_junction(self.ed.model(), &key)?;
        Some((key, j))
    }

    /// The picked node's outline, over the plan.
    pub(super) fn draw_node_pick(&self, painter: &egui::Painter, rect: Rect) {
        if self.tool != Tool::Node {
            return;
        }
        let Some((_, j)) = self.picked_node() else {
            return;
        };
        let mut outline = j.plate.clone();
        if let Some(f) = outline.first().copied() {
            outline.push(f);
        }
        painter.add(Shape::line(
            self.polygon(rect, &outline),
            Stroke::new(2.5, ACCENT),
        ));
        // The arms' ends.
        for a in &j.arms {
            let n = a.dir.perp_left();
            let end = a.origin + a.dir * a.cut;
            let line = [end - n * a.edge_right, end + n * a.edge_left];
            painter.add(Shape::line(
                self.polygon(rect, &line),
                Stroke::new(1.5, Color32::from_rgb(230, 120, 40)),
            ));
        }
    }

    pub(super) fn node_tool_properties(&mut self, ui: &mut Ui) {
        let Some((key, j)) = self.picked_node() else {
            props::title(ui, "Herramienta Nudo");
            group(ui, "tool-node", "Nudo", |ui| {
                props::note(
                    ui,
                    "Haz clic en un cruce, una T o una L de calles para cambiarlo: tipo de nudo, retranqueo de cada calle y radio, forma y acera de cada esquina.",
                );
            });
            return;
        };
        let model = self.ed.model();
        props::title(ui, &node_title(model, &key));
        let before = model
            .node(&key)
            .cloned()
            .unwrap_or_else(|| NodeSettings::new(key.clone()));
        let mut n = before.clone();
        let radius_of = |id: archisim_core::RoadId| {
            model
                .road(id)
                .map_or(4.0, |r| r.section.corner_radius)
                .max(0.0)
        };
        group(ui, "node-kind", "Tipo de nudo", |ui| {
            ui.horizontal_wrapped(|ui| {
                for k in NodeKind::ALL {
                    if ui.selectable_label(n.kind() == k, k.label()).clicked() {
                        n.kind = (k != NodeKind::Crossing).then_some(k);
                    }
                }
            });
            props::note(
                ui,
                match n.kind() {
                    NodeKind::Crossing => "Las calles se unen con sus esquinas y bordillos.",
                    NodeKind::Raised => "La calzada sube a la cota de la acera, con una rampa de 2 m en cada calle.",
                    NodeKind::Shared => "Plataforma única: calzada a la cota de la acera, con su pavimento, sin bordillos y con bolardos en las esquinas.",
                    NodeKind::Open => "Las calles se cruzan sin unirse (como un paso a distinto nivel).",
                },
            );
        });
        if n.kind() != NodeKind::Open {
            group(ui, "node-arms", "Calles que llegan · retranqueo", |ui| {
                for a in &j.arms {
                    let r = Junction::arm_ref(a);
                    let mut s = n.arm(r);
                    let hint = "Hasta dónde llega el nudo por esta calle, desde su centro";
                    row(ui, &arm_name(model, a), hint, |ui, w| {
                        let shown = s.setback.unwrap_or(a.cut);
                        if let Some(v) =
                            length_field(ui, Id::unique(("node-arm", &key, r)), shown, w - 8.0)
                        {
                            s.setback = Some(v.max(0.0));
                        }
                    });
                    n.set_arm(r, s);
                }
            });
            group(ui, "node-corners", "Esquinas", |ui| {
                let count = j.arms.len();
                for i in 0..count {
                    let (a, b) = (&j.arms[i], &j.arms[(i + 1) % count]);
                    let r = Junction::arm_ref(a);
                    let mut c = n.corner(r);
                    ui.label(
                        egui::RichText::new(format!("Entre el {} y el {}", heading(a), heading(b)))
                            .strong(),
                    );
                    let usual = radius_of(a.road).min(radius_of(b.road));
                    row(
                        ui,
                        "Radio (mm)",
                        "El del bordillo de la esquina",
                        |ui, w| {
                            if let Some(v) = length_field(
                                ui,
                                Id::unique(("node-radius", &key, r)),
                                c.radius.unwrap_or(usual),
                                w - 8.0,
                            ) {
                                c.radius = Some(v.max(0.0));
                            }
                        },
                    );
                    row(ui, "Forma", "", |ui, w| {
                        let mut shape = c.shape.unwrap_or_default();
                        ComboBox::from_id_salt(("node-shape", &key, r))
                            .width(w - 8.0)
                            .selected_text(shape.label())
                            .show_ui(ui, |ui| {
                                for s in CornerShape::ALL {
                                    ui.selectable_value(&mut shape, s, s.label());
                                }
                            });
                        c.shape = (shape != CornerShape::Arc).then_some(shape);
                    });
                    let mut walk = c.sidewalk.unwrap_or(true);
                    ui.switch(&mut walk, "Acera en la esquina");
                    c.sidewalk = (!walk).then_some(false);
                    n.set_corner(r, c);
                }
            });
        }
        let mut reset = false;
        group(ui, "node-reset", "Nudo calculado", |ui| {
            props::note(
                ui,
                "Lo que no cambies se sigue calculando con las calles. Si quitas una calle del nudo, sus ajustes se pierden.",
            );
            reset = ui
                .add_enabled(!before.is_plain(), egui::Button::new("Restablecer"))
                .clicked();
        });
        if reset {
            n = NodeSettings::new(key.clone());
        }
        if n != before {
            let res = self.ed.transact("Nudo", |m| m.set_node(n.clone()));
            self.report(res, "Nudo cambiado.");
        }
    }
}

#[cfg(test)]
pub(super) fn first_node(m: &Model, level: archisim_core::LevelId) -> Option<NodeKey> {
    crossroads::node_keys(m, level).into_iter().next()
}
