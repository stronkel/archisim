//! Urban surfaces drawn by their outline (N5, `docs/TERRITORIO.md` §2.8):
//! the tool is the plot's, drawing a surface of a kind (square, island,
//! median, car park, garden); its properties change its paving, edge,
//! height, joints and bays.

use archisim_core::{Bays, MaterialId, Point2, SurfaceEdge, SurfaceId, SurfaceKind};
use eframe::egui::{self, ComboBox, Id, Ui};

use super::props::{self, group, row, text_row};
use super::*;

/// The materials a new surface of a kind takes: of the project's, the
/// first whose name fits.
fn usual_materials(m: &archisim_core::Model, kind: SurfaceKind) -> (MaterialId, MaterialId) {
    let find = |words: &[&str], cat: &[&str]| {
        m.materials()
            .find(|x| {
                let n = x.name.to_lowercase();
                words.iter().any(|w| n.contains(w))
            })
            .or_else(|| m.materials().find(|x| cat.contains(&x.category.as_str())))
            .map(|x| x.id)
    };
    let any = m.materials().next().map_or(MaterialId(0), |x| x.id);
    let stone = find(&["granito", "piedra", "bordillo"], &["stone"]).unwrap_or(any);
    let body = match kind {
        SurfaceKind::Square => find(&["adoqu", "baldos", "losa"], &["ceramic", "stone"]),
        SurfaceKind::Island | SurfaceKind::Median => find(&["hormig"], &["concrete"]),
        SurfaceKind::Parking => find(&["asfalt"], &["asphalt"]),
        SurfaceKind::Garden => find(&["tierra", "césped", "cesped", "jard"], &["soil", "grass"]),
    }
    .unwrap_or(stone);
    (body, stone)
}

impl ArchiSimApp {
    /// Builds the surface just drawn with the plot's tool.
    pub(super) fn build_surface(&mut self, kind: SurfaceKind, outline: Vec<Point2>) {
        let level = self.ed.level;
        let (material, edge_material) = usual_materials(self.ed.model(), kind);
        let r = self.ed.transact("Crear superficie", |m| {
            m.add_surface(archisim_core::NewSurface {
                name: m.next_surface_name(kind),
                level,
                kind,
                outline,
                material,
                edge_material,
            })
        });
        match r {
            Ok(id) => {
                self.plot_tool.accept();
                self.ed.selection = [Element::Surface(id)].into_iter().collect();
                let (name, area, bays) = self
                    .ed
                    .model()
                    .surface(id)
                    .map(|s| (s.name.clone(), s.area(), s.bay_count()))
                    .unwrap_or_default();
                let more = if bays > 0 {
                    format!(", {bays} plazas")
                } else {
                    String::new()
                };
                self.say(format!(
                    "{name}: {} m²{more}. El borde, el pavimento y las juntas, en Propiedades.",
                    super::urban_ui::comma(area, 2)
                ));
            }
            Err(e) => self.complain(format!("No se puede crear la superficie: {e}.")),
        }
    }

    /// What the tool draws as a surface: its kind.
    pub(super) fn surface_tool_properties(&mut self, ui: &mut Ui, mut kind: SurfaceKind) {
        props::title(ui, "Herramienta Superficie · lo que vas a dibujar");
        group(ui, "tool-surface", "Superficie", |ui| {
            ui.horizontal_wrapped(|ui| {
                for k in SurfaceKind::ALL {
                    ui.selectable_value(&mut kind, k, k.label());
                }
            });
            let (edge, w, raise, joints, bays) = kind.usual();
            let mut text = format!(
                "{} de {} cm",
                edge.label(),
                super::urban_ui::comma(w * 100.0, 0)
            );
            if raise > 0.0 {
                text += &format!(
                    ", {} cm sobre lo de debajo",
                    super::urban_ui::comma(raise * 100.0, 0)
                );
            }
            if joints > 0.0 {
                text += &format!(", juntas cada {} m", super::urban_ui::comma(joints, 2));
            }
            if bays.is_some() {
                text += ", plazas de 2,50 × 5,00 m en batería";
            }
            props::note(ui, &(text + "."));
            props::note(
                ui,
                "Dibuja el contorno con clics (o un rectángulo); se pega a los bordillos, las líneas de fachada y otras superficies. Se apoya en el terreno o en la calle que tenga debajo.",
            );
        });
        self.surface_draw = Some(kind);
    }

    pub(super) fn surface_properties(&mut self, ui: &mut Ui, id: SurfaceId) {
        let Some(s) = self.ed.model().surface(id).cloned() else {
            return;
        };
        props::title(ui, &format!("{} · {}", s.name, s.kind.label()));
        if super::sketch_ui::edit_contour_button(ui) {
            self.start_sketch(crate::sketch::SketchTarget::Surface(id));
            return;
        }
        let model = self.ed.model();
        if !s.holes.is_empty() {
            ui.label(
                egui::RichText::new(format!(
                    "{} hueco{} (alcorques, fuentes, parterres): se cambian con Editar contorno",
                    s.holes.len(),
                    if s.holes.len() == 1 { "" } else { "s" }
                ))
                .small(),
            );
        }
        let mut n = s.clone();
        group(ui, "surface", "Superficie", |ui| {
            row(ui, "Nombre", "", |ui, w| {
                if let Some(t) =
                    text_edit_committed(ui, Id::unique(("surface-name", id)), &n.name, w - 8.0)
                {
                    n.name = t.trim().to_string();
                }
            });
            row(ui, "Clase", "", |ui, w| {
                ComboBox::from_id_salt(("surface-kind", id))
                    .width(w - 8.0)
                    .selected_text(n.kind.label())
                    .show_ui(ui, |ui| {
                        for k in SurfaceKind::ALL {
                            if ui.selectable_label(n.kind == k, k.label()).clicked() && k != n.kind
                            {
                                let (edge, ew, raise, joints, bays) = k.usual();
                                n.kind = k;
                                n.edge = edge;
                                n.edge_width = ew;
                                n.raise = raise;
                                n.joints = joints;
                                n.bays = bays;
                            }
                        }
                    });
            });
            row(ui, "Pavimento", "", |ui, w| {
                super::cards::material_button(
                    ui,
                    Id::unique(("surface-mat", id)),
                    model,
                    &mut n.material,
                    w - 8.0,
                );
            });
            row(
                ui,
                "Altura (mm)",
                "Sobre el terreno o la calle de debajo; negativa, hundida (hasta 6 m): el terreno se recorta",
                |ui, w| {
                    if let Some(v) =
                        length_field(ui, Id::unique(("surface-raise", id)), n.raise, w - 8.0)
                    {
                        n.raise = v.clamp(-6.0, 3.0);
                    }
                },
            );
            text_row(
                ui,
                "Superficie (m²)",
                "",
                &super::urban_ui::comma(n.area(), 2),
            );
        });
        group(ui, "surface-edge", "Borde", |ui| {
            row(ui, "Borde", "Bordillo, rigola o encintado", |ui, w| {
                ComboBox::from_id_salt(("surface-edge", id))
                    .width(w - 8.0)
                    .selected_text(n.edge.label())
                    .show_ui(ui, |ui| {
                        for e in SurfaceEdge::ALL {
                            ui.selectable_value(&mut n.edge, e, e.label());
                        }
                    });
            });
            if n.edge != SurfaceEdge::None {
                row(ui, "Ancho (mm)", "", |ui, w| {
                    if let Some(v) = length_field(
                        ui,
                        Id::unique(("surface-edge-w", id)),
                        n.edge_width,
                        w - 8.0,
                    ) {
                        n.edge_width = v.clamp(0.02, 2.0);
                    }
                });
                row(ui, "Material", "", |ui, w| {
                    super::cards::material_button(
                        ui,
                        Id::unique(("surface-edge-mat", id)),
                        model,
                        &mut n.edge_material,
                        w - 8.0,
                    );
                });
            }
        });
        group(ui, "surface-joints", "Juntas del pavimento", |ui| {
            row(ui, "Aparejo", "Cómo van las juntas", |ui, w| {
                ComboBox::from_id_salt(("surface-pattern", id))
                    .width(w - 8.0)
                    .selected_text(n.pattern.label())
                    .show_ui(ui, |ui| {
                        for p in archisim_core::JointPattern::ALL {
                            ui.selectable_value(&mut n.pattern, p, p.label());
                        }
                    });
            });
            row(ui, "Cada (mm)", "0: sin juntas", |ui, w| {
                if let Some(v) =
                    length_field(ui, Id::unique(("surface-joints", id)), n.joints, w - 8.0)
                {
                    n.joints = if v < 0.1 { 0.0 } else { v };
                }
            });
            row(ui, "Giro (°)", "Desde el eje x de la planta", |ui, w| {
                let mut deg = n.joint_angle.to_degrees();
                if ui
                    .add_sized(
                        [w - 8.0, 20.0],
                        egui::DragValue::new(&mut deg)
                            .range(-90.0..=90.0)
                            .speed(1.0)
                            .suffix(" °"),
                    )
                    .changed()
                {
                    n.joint_angle = deg.to_radians();
                }
            });
            if ui
                .small_button("Al lado más largo")
                .on_hover_text("Las juntas paralelas a su lado más largo")
                .clicked()
            {
                let (a, b) = n.longest_side();
                n.joint_angle = (b.y - a.y).atan2(b.x - a.x);
            }
        });
        group(ui, "surface-bays", "Plazas de aparcamiento", |ui| {
            let mut on = n.bays.is_some();
            super::widgets::switch(ui, &mut on, "Plazas pintadas");
            if on != n.bays.is_some() {
                n.bays = on.then(Bays::default);
            }
            if let Some(b) = n.bays.as_mut() {
                for (label, v, key) in [
                    ("Ancho (mm)", &mut b.width, "w"),
                    ("Fondo (mm)", &mut b.depth, "d"),
                ] {
                    row(ui, label, "", |ui, w| {
                        if let Some(x) =
                            length_field(ui, Id::unique(("surface-bay", id, key)), *v, w - 8.0)
                        {
                            *v = x.clamp(0.6, 20.0);
                        }
                    });
                }
                row(
                    ui,
                    "Ángulo (°)",
                    "90: en batería; 45 o 60: en espiga",
                    |ui, w| {
                        ui.add_sized(
                            [w - 8.0, 20.0],
                            egui::DragValue::new(&mut b.angle)
                                .range(15.0..=90.0)
                                .speed(1.0)
                                .suffix(" °"),
                        );
                    },
                );
            }
            if n.bays.is_some() {
                props::note(
                    ui,
                    &format!(
                        "{} plazas, en filas a lo largo del lado más largo, con calle de 6 m entre cada dos.",
                        n.bay_count()
                    ),
                );
            }
        });
        group(ui, "surface-identity", "Datos de identidad", |ui| {
            text_row(ui, "Clase IFC", "", "IfcBuildingElementProxy");
        });
        if n != s {
            let res = self
                .ed
                .transact("Superficie", |m| m.edit_surface(id, |x| *x = n));
            self.report(res, "Superficie cambiada.");
        }
    }
}
