//! The column and beam tools (fase 2, 2.5): what they draw, their
//! properties and those of selected columns and beams, and the type
//! settings (section and material) edited in place.

use archisim_core::{BeamId, ColumnId, MaterialId, MemberTypeId, MemberUsage, Profile, WallTop};
use eframe::egui::{self, Id, Ui};

use super::icons::Icon;
use super::props::{self, choice, group, id_row, row, text_row, type_card, TypeItem};
use super::*;
use crate::editor::{normal_angle, TopChoice};

/// The beam being drawn: its first point (the beam tool chains: each beam
/// starts where the last one ended).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct BeamTool {
    pub anchor: Option<Point2>,
}

impl BeamTool {
    pub fn is_drawing(&self) -> bool {
        self.anchor.is_some()
    }

    pub fn finish(&mut self) {
        self.anchor = None;
    }
}

/// Section kinds offered when editing a type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SectionShape {
    Rectangle,
    Circle,
    IShape,
    UShape,
    TShape,
    LShape,
    RectHollow,
    CircleHollow,
}

fn shape_of(p: Profile) -> SectionShape {
    match p {
        Profile::Rectangle { .. } => SectionShape::Rectangle,
        Profile::Circle { .. } => SectionShape::Circle,
        Profile::IShape { .. } => SectionShape::IShape,
        Profile::UShape { .. } => SectionShape::UShape,
        Profile::TShape { .. } => SectionShape::TShape,
        Profile::LShape { .. } => SectionShape::LShape,
        Profile::RectHollow { .. } => SectionShape::RectHollow,
        Profile::CircleHollow { .. } => SectionShape::CircleHollow,
    }
}

/// Section in words: «300 × 500», «Ø 300», «I 150 × 300».
pub fn profile_text(p: Profile) -> String {
    match p {
        Profile::Rectangle { width, depth } => format!("{} × {} mm", fmt_mm(width), fmt_mm(depth)),
        Profile::Circle { diameter } => format!("Ø {} mm", fmt_mm(diameter)),
        Profile::IShape { width, depth, .. } => {
            format!("I {} × {} mm", fmt_mm(width), fmt_mm(depth))
        }
        Profile::UShape { width, depth, .. } => {
            format!("U {} × {} mm", fmt_mm(width), fmt_mm(depth))
        }
        Profile::TShape { width, depth, .. } => {
            format!("T {} × {} mm", fmt_mm(width), fmt_mm(depth))
        }
        Profile::LShape {
            width,
            depth,
            thickness,
        } => format!(
            "L {} × {} × {} mm",
            fmt_mm(width),
            fmt_mm(depth),
            fmt_mm(thickness)
        ),
        Profile::RectHollow { width, depth, wall } => format!(
            "Tubo {} × {} × {} mm",
            fmt_mm(width),
            fmt_mm(depth),
            fmt_mm(wall)
        ),
        Profile::CircleHollow { diameter, wall } => {
            format!("Tubo Ø {} × {} mm", fmt_mm(diameter), fmt_mm(wall))
        }
    }
}

/// Degrees with a decimal comma.
fn fmt_deg(a: f64) -> String {
    let d = (a.to_degrees() * 100.0).round() / 100.0;
    format!("{d}").replace('.', ",")
}

pub(super) fn angle_field(ui: &mut Ui, id: Id, value: f64, width: f32) -> Option<f64> {
    let text = text_edit_committed(ui, id, &fmt_deg(value), width)?;
    text.trim()
        .trim_end_matches('°')
        .replace(',', ".")
        .parse::<f64>()
        .ok()
        .filter(|v| v.is_finite())
        .map(|d| normal_angle(d.to_radians()))
}

impl ArchiSimApp {
    pub(super) fn member_type_items(
        &mut self,
        ctx: &egui::Context,
        usage: MemberUsage,
    ) -> Vec<TypeItem<MemberTypeId>> {
        let sig = self.picture_signature(false);
        let model = self.ed.model();
        let thumbs = &mut self.thumbs;
        model
            .member_types()
            .filter(|t| t.usage == usage)
            .map(|t| {
                let mat = model.material(t.material).map_or("", |m| m.name.as_str());
                let thumb = thumbs.get(
                    ctx,
                    format!("member:{:?}:{:?}:{sig}", t.profile, t.material),
                    || crate::type_preview::member_model(model, t),
                );
                TypeItem {
                    id: t.id,
                    name: t.name.clone(),
                    note: format!("{} · {mat}", profile_text(t.profile)),
                    thumb,
                    caption: "Sección 1:20",
                }
            })
            .collect()
    }

    // ---- Tools ---------------------------------------------------------------

    pub(super) fn column_tool_properties(&mut self, ui: &mut Ui) {
        props::title(ui, "Herramienta Pilar · lo que vas a colocar");
        let items = self.member_type_items(ui.ctx(), MemberUsage::Column);
        if items.is_empty() {
            props::note(ui, "El proyecto no tiene tipos de pilar.");
            return;
        }
        let mut chosen = None;
        let card = type_card(
            ui,
            "tool-column",
            Icon::Column,
            "Pilar",
            "IfcColumn",
            Some(self.column_settings.member_type),
            &items,
            &mut chosen,
            None,
        );
        if let Some(t) = chosen {
            self.column_settings.member_type = t;
        }
        if card.edit {
            self.say("La sección y el material del tipo se editan en el grupo «Tipo», abajo.");
        }
        let base_name = self
            .ed
            .model()
            .level(self.ed.level)
            .map(|l| l.name.clone())
            .unwrap_or_default();
        let above = self.levels_above(self.ed.level);
        let next_name = self
            .ed
            .model()
            .level_above(self.ed.level)
            .and_then(|l| self.ed.model().level(l))
            .map(|l| l.name.clone());
        let resolved = self.column_settings.top_on(self.ed.model(), self.ed.level);
        group(ui, "tool-column-constraints", "Restricciones", |ui| {
            text_row(
                ui,
                "Restricción de base",
                "La planta en la que colocas",
                &base_name,
            );
            row(
                ui,
                "Desfase de base (mm)",
                "Altura del arranque del pilar sobre la planta (negativo: desde la cimentación)",
                |ui, w| {
                    if let Some(v) = length_field(
                        ui,
                        Id::unique("tool-col-base"),
                        self.column_settings.base_offset,
                        w - 8.0,
                    ) {
                        self.column_settings.base_offset = v;
                    }
                },
            );
            let next_label = match &next_name {
                Some(n) => format!("Planta superior ({n})"),
                None => "Planta superior (no hay)".to_string(),
            };
            let mut options: Vec<(TopChoice, String)> = vec![
                (TopChoice::Unconnected, "Desconectada".into()),
                (TopChoice::NextLevel, next_label),
            ];
            options.extend(
                above
                    .iter()
                    .map(|(id, name)| (TopChoice::Level(*id), format!("Hasta {name}"))),
            );
            row(
                ui,
                "Restricción superior",
                "Hasta una planta (sigue sus cambios de cota) o altura propia",
                |ui, w| {
                    let opts: Vec<(TopChoice, &str)> =
                        options.iter().map(|(v, n)| (*v, n.as_str())).collect();
                    self.column_settings.top =
                        choice(ui, "tool-col-top", w, self.column_settings.top, &opts);
                },
            );
            match resolved {
                WallTop::Level { .. } => row(
                    ui,
                    "Desfase superior (mm)",
                    "Negativo: por debajo de la planta (por ejemplo, el canto de la viga o del forjado)",
                    |ui, w| {
                        if let Some(v) = length_field(
                            ui,
                            Id::unique("tool-col-top-offset"),
                            self.column_settings.top_offset,
                            w - 8.0,
                        ) {
                            self.column_settings.top_offset = v;
                        }
                    },
                ),
                WallTop::Unconnected => row(
                    ui,
                    "Altura desconectada (mm)",
                    "Altura del pilar desde su base",
                    |ui, w| {
                        if let Some(v) = length_field(
                            ui,
                            Id::unique("tool-col-height"),
                            self.column_settings.height,
                            w - 8.0,
                        ) {
                            if v > 0.0 {
                                self.column_settings.height = v;
                            } else {
                                self.complain("La altura debe ser mayor que cero.");
                            }
                        }
                    },
                ),
            }
        });
        group(ui, "tool-column-placement", "Colocación", |ui| {
            row(
                ui,
                "Giro (°)",
                "Giro de la sección respecto al eje X del proyecto. Espacio: +90°",
                |ui, w| {
                    if let Some(v) = angle_field(
                        ui,
                        Id::unique("tool-col-rot"),
                        self.column_settings.rotation,
                        w - 8.0,
                    ) {
                        self.column_settings.rotation = v;
                    }
                },
            );
            props::note(ui, "El clic marca el centro de la sección.");
        });
        self.member_type_group(ui, self.column_settings.member_type);
    }

    pub(super) fn beam_tool_properties(&mut self, ui: &mut Ui) {
        props::title(ui, "Herramienta Viga · lo que vas a dibujar");
        let items = self.member_type_items(ui.ctx(), MemberUsage::Beam);
        if items.is_empty() {
            props::note(ui, "El proyecto no tiene tipos de viga.");
            return;
        }
        let mut chosen = None;
        let card = type_card(
            ui,
            "tool-beam",
            Icon::Beam,
            "Viga",
            "IfcBeam",
            Some(self.beam_settings.member_type),
            &items,
            &mut chosen,
            None,
        );
        if let Some(t) = chosen {
            self.beam_settings.member_type = t;
        }
        if card.edit {
            self.say("La sección y el material del tipo se editan en el grupo «Tipo», abajo.");
        }
        let target = self.ed.beam_level(&self.beam_settings);
        let target_name = self
            .ed
            .model()
            .level(target)
            .map(|l| l.name.clone())
            .unwrap_or_default();
        group(ui, "tool-beam-constraints", "Restricciones", |ui| {
            row(
                ui,
                "Planta de referencia",
                "Una viga dibujada en esta planta suele llevar el forjado de la de encima (como en Revit): su cara superior queda a la cota de esa planta",
                |ui, w| {
                    self.beam_settings.level_above = choice(
                        ui,
                        "tool-beam-level",
                        w,
                        self.beam_settings.level_above,
                        &[(true, "Planta superior"), (false, "Esta planta")],
                    );
                },
            );
            text_row(ui, "Planta de la viga", "", &target_name);
            row(
                ui,
                "Cara superior (mm)",
                "Sobre la planta de la viga; la sección cuelga hacia abajo",
                |ui, w| {
                    if let Some(v) = length_field(
                        ui,
                        Id::unique("tool-beam-top"),
                        self.beam_settings.top_offset,
                        w - 8.0,
                    ) {
                        self.beam_settings.top_offset = v;
                    }
                },
            );
            if target == self.ed.level && self.beam_settings.level_above {
                props::note(
                    ui,
                    "No hay planta encima de esta: la viga queda en esta planta.",
                );
            }
        });
        self.member_type_group(ui, self.beam_settings.member_type);
    }

    /// The type's section and material, edited in place (all elements of
    /// the type follow), with «Duplicar» to start a new type from it.
    fn member_type_group(&mut self, ui: &mut Ui, id: MemberTypeId) {
        let model = self.ed.model();
        let Some(t) = model.member_type(id).cloned() else {
            return;
        };
        let users = model.member_type_users(id);
        let materials: Vec<(MaterialId, String)> =
            model.materials().map(|m| (m.id, m.name.clone())).collect();
        let title = format!("Tipo · {users} en el proyecto");
        group(ui, &format!("member-type-{}", id.0), &title, |ui| {
            let mut name = t.name.clone();
            let mut material = t.material;
            let mut profile = t.profile;
            let mut changed = false;
            row(
                ui,
                "Nombre",
                "Único entre los tipos de pilar y viga",
                |ui, w| {
                    if let Some(v) =
                        text_edit_committed(ui, Id::unique(("member-name", id)), &t.name, w - 8.0)
                    {
                        name = v;
                        changed = true;
                    }
                },
            );
            row(ui, "Material", "Hormigón, acero, madera…", |ui, w| {
                let opts: Vec<(MaterialId, &str)> =
                    materials.iter().map(|(m, n)| (*m, n.as_str())).collect();
                let m = choice(ui, ("member-mat", id), w, material, &opts);
                if m != material {
                    material = m;
                    changed = true;
                }
            });
            row(ui, "Sección", "Forma de la sección", |ui, w| {
                let shape = shape_of(profile);
                let picked = choice(
                    ui,
                    ("member-shape", id),
                    w,
                    shape,
                    &[
                        (SectionShape::Rectangle, "Rectangular"),
                        (SectionShape::Circle, "Circular"),
                        (SectionShape::IShape, "Perfil I / H"),
                        (SectionShape::UShape, "Perfil U (UPN)"),
                        (SectionShape::TShape, "Perfil T"),
                        (SectionShape::LShape, "Angular L"),
                        (SectionShape::RectHollow, "Tubo rectangular"),
                        (SectionShape::CircleHollow, "Tubo circular"),
                    ],
                );
                if picked != shape {
                    let (a, b) = profile.size();
                    profile = match picked {
                        SectionShape::Rectangle => Profile::Rectangle { width: a, depth: b },
                        SectionShape::Circle => Profile::Circle { diameter: a.max(b) },
                        SectionShape::IShape => Profile::IShape {
                            width: a,
                            depth: b,
                            web: (a * 0.05).max(0.005),
                            flange: (b * 0.06).max(0.007),
                        },
                        SectionShape::UShape => Profile::UShape {
                            width: a,
                            depth: b,
                            web: (a * 0.1).max(0.005),
                            flange: (b * 0.06).max(0.007),
                        },
                        SectionShape::TShape => Profile::TShape {
                            width: a,
                            depth: b,
                            web: (a * 0.1).max(0.005),
                            flange: (b * 0.1).max(0.005),
                        },
                        SectionShape::RectHollow => Profile::RectHollow {
                            width: a,
                            depth: b,
                            wall: (a.min(b) * 0.06).max(0.003),
                        },
                        SectionShape::CircleHollow => Profile::CircleHollow {
                            diameter: a.max(b),
                            wall: (a.max(b) * 0.05).max(0.003),
                        },
                        SectionShape::LShape => Profile::LShape {
                            width: a,
                            depth: b,
                            thickness: (a.min(b) * 0.1).max(0.004),
                        },
                    };
                    changed = true;
                }
            });
            let mut dim = |ui: &mut Ui, label: &str, hint: &str, salt: &str, v: &mut f64| {
                row(ui, label, hint, |ui, w| {
                    if let Some(x) = length_field(ui, Id::unique((salt, id)), *v, w - 8.0) {
                        *v = x;
                        changed = true;
                    }
                });
            };
            match &mut profile {
                Profile::Rectangle { width, depth } => {
                    dim(ui, "Ancho (mm)", "Eje X de la sección", "member-w", width);
                    dim(
                        ui,
                        "Canto (mm)",
                        "Eje Y de la sección (en vigas, la altura)",
                        "member-d",
                        depth,
                    );
                }
                Profile::Circle { diameter } => {
                    dim(ui, "Diámetro (mm)", "", "member-dia", diameter);
                }
                Profile::IShape {
                    width,
                    depth,
                    web,
                    flange,
                } => {
                    dim(ui, "Ancho de ala (mm)", "", "member-w", width);
                    dim(ui, "Canto (mm)", "", "member-d", depth);
                    dim(ui, "Espesor del alma (mm)", "", "member-web", web);
                    dim(ui, "Espesor de ala (mm)", "", "member-fl", flange);
                }
                Profile::UShape {
                    width,
                    depth,
                    web,
                    flange,
                }
                | Profile::TShape {
                    width,
                    depth,
                    web,
                    flange,
                } => {
                    dim(ui, "Ancho de ala (mm)", "", "member-w", width);
                    dim(ui, "Canto (mm)", "", "member-d", depth);
                    dim(ui, "Espesor del alma (mm)", "", "member-web", web);
                    dim(ui, "Espesor de ala (mm)", "", "member-fl", flange);
                }
                Profile::LShape {
                    width,
                    depth,
                    thickness,
                } => {
                    dim(
                        ui,
                        "Ala horizontal (mm)",
                        "Eje X de la sección",
                        "member-w",
                        width,
                    );
                    dim(
                        ui,
                        "Ala vertical (mm)",
                        "Eje Y de la sección",
                        "member-d",
                        depth,
                    );
                    dim(ui, "Espesor (mm)", "", "member-t", thickness);
                }
                Profile::RectHollow { width, depth, wall } => {
                    dim(ui, "Ancho (mm)", "Eje X de la sección", "member-w", width);
                    dim(ui, "Canto (mm)", "Eje Y de la sección", "member-d", depth);
                    dim(ui, "Espesor de pared (mm)", "", "member-t", wall);
                }
                Profile::CircleHollow { diameter, wall } => {
                    dim(ui, "Diámetro (mm)", "", "member-dia", diameter);
                    dim(ui, "Espesor de pared (mm)", "", "member-t", wall);
                }
            }
            text_row(
                ui,
                "Área de la sección",
                "",
                &format!("{} cm²", fmt_number(profile.area() * 1e4)),
            );
            if changed {
                let r = self.ed.edit_member_type(id, &name, material, profile);
                self.report(r, "Tipo modificado: cambian todos sus elementos.");
            }
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                if ui.button("Duplicar tipo").clicked() {
                    match self.ed.duplicate_member_type(id) {
                        Ok(new) => {
                            match t.usage {
                                MemberUsage::Column => self.column_settings.member_type = new,
                                MemberUsage::Beam => self.beam_settings.member_type = new,
                            }
                            self.say("Tipo duplicado: cámbiale el nombre y la sección.");
                        }
                        Err(e) => self.complain(format!("No se puede duplicar: {e}.")),
                    }
                }
                if users == 0 && ui.button("Borrar tipo").clicked() {
                    let others: Vec<MemberTypeId> = self
                        .ed
                        .model()
                        .member_types()
                        .filter(|o| o.usage == t.usage && o.id != id)
                        .map(|o| o.id)
                        .collect();
                    if let Some(&other) = others.first() {
                        match self.ed.remove_member_type(id) {
                            Ok(()) => {
                                match t.usage {
                                    MemberUsage::Column => self.column_settings.member_type = other,
                                    MemberUsage::Beam => self.beam_settings.member_type = other,
                                }
                                self.say("Tipo borrado.");
                            }
                            Err(e) => self.complain(format!("No se puede borrar: {e}.")),
                        }
                    } else {
                        self.complain("Es el último tipo: no se puede borrar.");
                    }
                }
            });
        });
    }

    // ---- Selection -------------------------------------------------------------

    pub(super) fn column_properties(&mut self, ui: &mut Ui, id: ColumnId) {
        let model = self.ed.model();
        let Some(c) = model.column(id).cloned() else {
            return;
        };
        let same_type = self.same_type(Element::Column(id));
        props::title(ui, "Pilar · 1 seleccionado");
        let items = self.member_type_items(ui.ctx(), MemberUsage::Column);
        let mut chosen = None;
        let card = type_card(
            ui,
            "column",
            Icon::Column,
            "Pilar",
            "IfcColumn",
            Some(c.member_type),
            &items,
            &mut chosen,
            Some(same_type.len()),
        );
        if let Some(t) = chosen {
            let r = self.ed.set_members_type(&[Element::Column(id)], t);
            self.report(r, "Tipo de pilar cambiado.");
        }
        if card.edit {
            self.say("La sección y el material del tipo se editan en el grupo «Tipo», abajo.");
        }
        if card.select_all {
            self.select_instances(same_type, "pilares");
        }
        let above = self.levels_above(c.level);
        group(ui, "column-constraints", "Restricciones", |ui| {
            self.level_row(
                ui,
                ("column-level", id),
                "Restricción de base",
                Some(c.level),
                &[Element::Column(id)],
            );
            row(ui, "Desfase de base (mm)", "", |ui, w| {
                if let Some(v) =
                    length_field(ui, Id::unique(("col-base", id)), c.base_offset, w - 8.0)
                {
                    let r = self
                        .ed
                        .edit_columns("Desfase de base", &[id], |c| c.base_offset = v);
                    self.report(r, "Desfase cambiado.");
                }
            });
            let shown = match c.top {
                WallTop::Unconnected => None,
                WallTop::Level { level, .. } => Some(level),
            };
            let mut options: Vec<(Option<archisim_core::LevelId>, String)> =
                vec![(None, "Desconectada".to_string())];
            options.extend(above.iter().map(|(l, n)| (Some(*l), format!("Hasta {n}"))));
            let picked = row(
                ui,
                "Restricción superior",
                "Hasta una planta (sigue sus cambios de cota) o altura propia",
                |ui, w| {
                    let opts: Vec<(Option<archisim_core::LevelId>, &str)> =
                        options.iter().map(|(v, n)| (*v, n.as_str())).collect();
                    choice(ui, ("col-top", id), w, shown, &opts)
                },
            );
            if picked != shown {
                let offset = match c.top {
                    WallTop::Level { offset, .. } => offset,
                    WallTop::Unconnected => 0.0,
                };
                let top = match picked {
                    None => WallTop::Unconnected,
                    Some(level) => WallTop::Level { level, offset },
                };
                let r = self
                    .ed
                    .edit_columns("Restricción superior", &[id], |c| c.top = top);
                self.report(r, "Restricción superior cambiada.");
            }
            match c.top {
                WallTop::Level { level, offset } => row(
                    ui,
                    "Desfase superior (mm)",
                    "Negativo: por debajo de la planta",
                    |ui, w| {
                        if let Some(v) =
                            length_field(ui, Id::unique(("col-top-off", id)), offset, w - 8.0)
                        {
                            let r = self.ed.edit_columns("Desfase superior", &[id], |c| {
                                c.top = WallTop::Level { level, offset: v }
                            });
                            self.report(r, "Desfase superior cambiado.");
                        }
                    },
                ),
                WallTop::Unconnected => row(ui, "Altura (mm)", "", |ui, w| {
                    if let Some(v) = length_field(ui, Id::unique(("col-h", id)), c.height, w - 8.0)
                    {
                        let r = self
                            .ed
                            .edit_columns("Altura del pilar", &[id], |c| c.height = v);
                        self.report(r, "Altura cambiada.");
                    }
                }),
            }
        });
        group(ui, "column-placement", "Colocación", |ui| {
            row(ui, "Giro (°)", "Espacio: +90°", |ui, w| {
                if let Some(v) = angle_field(ui, Id::unique(("col-rot", id)), c.rotation, w - 8.0) {
                    let r = self
                        .ed
                        .edit_columns("Girar pilar", &[id], |c| c.rotation = v);
                    self.report(r, "Pilar girado.");
                }
            });
            text_row(
                ui,
                "Centro",
                "",
                &format!("{} ; {}", fmt_m(c.position.x), fmt_m(c.position.y)),
            );
        });
        group(ui, "column-dimensions", "Dimensiones", |ui| {
            text_row(ui, "Altura (mm)", "Resultante", &fmt_mm(c.height));
            if let Some(t) = self.ed.model().member_type(c.member_type) {
                text_row(ui, "Sección", "", &profile_text(t.profile));
                text_row(
                    ui,
                    "Volumen",
                    "",
                    &format!("{} m³", fmt_number(t.profile.area() * c.height)),
                );
            }
        });
        self.member_type_group(ui, c.member_type);
        self.phase_group(ui, "column-phase", &[Element::Column(id)]);
        group(ui, "column-identity", "Datos de identidad", |ui| {
            id_row(ui, "GlobalId", &c.guid.to_string());
        });
    }

    pub(super) fn beam_properties(&mut self, ui: &mut Ui, id: BeamId) {
        let model = self.ed.model();
        let Some(b) = model.beam(id).cloned() else {
            return;
        };
        let same_type = self.same_type(Element::Beam(id));
        props::title(ui, "Viga · 1 seleccionada");
        let items = self.member_type_items(ui.ctx(), MemberUsage::Beam);
        let mut chosen = None;
        let card = type_card(
            ui,
            "beam",
            Icon::Beam,
            "Viga",
            "IfcBeam",
            Some(b.member_type),
            &items,
            &mut chosen,
            Some(same_type.len()),
        );
        if let Some(t) = chosen {
            let r = self.ed.set_members_type(&[Element::Beam(id)], t);
            self.report(r, "Tipo de viga cambiado.");
        }
        if card.edit {
            self.say("La sección y el material del tipo se editan en el grupo «Tipo», abajo.");
        }
        if card.select_all {
            self.select_instances(same_type, "vigas");
        }
        group(ui, "beam-constraints", "Restricciones", |ui| {
            self.level_row(
                ui,
                ("beam-level", id),
                "Planta de referencia",
                Some(b.level),
                &[Element::Beam(id)],
            );
            row(
                ui,
                "Cara superior (mm)",
                "Sobre la planta; la sección cuelga hacia abajo",
                |ui, w| {
                    if let Some(v) =
                        length_field(ui, Id::unique(("beam-top", id)), b.top_offset, w - 8.0)
                    {
                        let r = self
                            .ed
                            .edit_beams("Desfase de viga", &[id], |b| b.top_offset = v);
                        self.report(r, "Desfase cambiado.");
                    }
                },
            );
        });
        group(ui, "beam-dimensions", "Dimensiones", |ui| {
            row(
                ui,
                "Longitud (mm)",
                "Desde el inicio; el final se desplaza",
                |ui, w| {
                    if let Some(v) =
                        length_field(ui, Id::unique(("beam-len", id)), b.length(), w - 8.0)
                    {
                        if v > 0.0 {
                            let r = self.ed.edit_beams("Longitud de viga", &[id], |b| {
                                if let Some(u) = (b.end - b.start).normalized() {
                                    b.end = b.start + u * v;
                                }
                            });
                            self.report(r, "Longitud cambiada.");
                        } else {
                            self.complain("La longitud debe ser mayor que cero.");
                        }
                    }
                },
            );
            if let Some(t) = self.ed.model().member_type(b.member_type) {
                text_row(ui, "Sección", "", &profile_text(t.profile));
                text_row(
                    ui,
                    "Volumen",
                    "",
                    &format!("{} m³", fmt_number(t.profile.area() * b.length())),
                );
            }
        });
        self.member_type_group(ui, b.member_type);
        self.phase_group(ui, "beam-phase", &[Element::Beam(id)]);
        group(ui, "beam-identity", "Datos de identidad", |ui| {
            id_row(ui, "GlobalId", &b.guid.to_string());
        });
    }

    // ---- Plan ---------------------------------------------------------------------

    /// A click of the column or beam tool at `p`.
    pub(super) fn member_click(&mut self, p: Point2) {
        match self.tool {
            Tool::Column => match self.ed.add_column(&self.column_settings, p) {
                Ok(id) => {
                    self.ed.selection = [Element::Column(id)].into_iter().collect();
                    self.say("Pilar colocado. Espacio gira 90° el siguiente.");
                }
                Err(e) => self.complain(format!("No se puede colocar el pilar: {e}.")),
            },
            Tool::Beam => match self.beam_tool.anchor {
                None => self.beam_tool.anchor = Some(p),
                Some(a) if a.approx_eq(p) => {}
                Some(a) => match self.ed.add_beam(&self.beam_settings, a, p) {
                    Ok(id) => {
                        self.beam_tool.anchor = Some(p);
                        self.ed.selection = [Element::Beam(id)].into_iter().collect();
                        self.say(format!(
                            "Viga de {} mm creada. Clic: la siguiente desde aquí · Esc: terminar.",
                            fmt_mm(a.distance(p))
                        ));
                    }
                    Err(e) => self.complain(format!("No se puede crear la viga: {e}.")),
                },
            },
            _ => {}
        }
    }

    /// The column that would be placed, or the beam being drawn.
    pub(super) fn draw_member_preview(&self, painter: &egui::Painter, rect: Rect) {
        let Some(c) = self.cursor else {
            return;
        };
        let model = self.ed.model();
        match self.tool {
            Tool::Column => {
                let Some(t) = model.member_type(self.column_settings.member_type) else {
                    return;
                };
                let probe = archisim_core::Column {
                    id: ColumnId(0),
                    guid: Default::default(),
                    member_type: t.id,
                    level: self.ed.level,
                    position: c.point,
                    rotation: self.column_settings.rotation,
                    base_offset: 0.0,
                    top: WallTop::Unconnected,
                    height: 1.0,
                };
                let outline = probe.plan_outline(t.profile, 48);
                let pts = self.polygon(rect, &outline);
                painter.add(Shape::convex_polygon(
                    pts.clone(),
                    ACCENT.gamma_multiply(0.15),
                    Stroke::NONE,
                ));
                painter.add(Shape::closed_line(pts, Stroke::new(1.5, ACCENT)));
                self.draw_label(
                    painter,
                    rect,
                    c.point,
                    format!(
                        "{}   {}°",
                        profile_text(t.profile),
                        fmt_deg(self.column_settings.rotation)
                    ),
                );
            }
            Tool::Beam => {
                let Some(a) = self.beam_tool.anchor else {
                    return;
                };
                let Some(t) = model.member_type(self.beam_settings.member_type) else {
                    return;
                };
                let probe = archisim_core::Beam {
                    id: BeamId(0),
                    guid: Default::default(),
                    member_type: t.id,
                    level: self.ed.level,
                    start: a,
                    end: c.point,
                    top_offset: 0.0,
                };
                let outline = archisim_plan::members::beam_outline(model, &probe);
                if outline.len() == 4 {
                    let pts = self.polygon(rect, &outline);
                    painter.add(Shape::closed_line(pts, Stroke::new(1.5, ACCENT)));
                }
                dashed(
                    painter,
                    self.view.to_screen(rect, a),
                    self.view.to_screen(rect, c.point),
                    ACCENT,
                );
                let d = c.point - a;
                self.draw_label(
                    painter,
                    rect,
                    c.point,
                    format!(
                        "L {} mm   {}",
                        fmt_mm(d.length()),
                        fmt_angle(d.y.atan2(d.x))
                    ),
                );
            }
            _ => {}
        }
    }
}

/// A number with two decimals and a decimal comma.
fn fmt_number(v: f64) -> String {
    format!("{v:.2}").replace('.', ",")
}
