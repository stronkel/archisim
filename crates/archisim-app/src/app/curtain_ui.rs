//! The curtain wall tool ("Muro cortina"): two clicks give its line (the
//! outside to the left, as walls); what it looks like is its type, chosen
//! in Properties and designed in the curtain wall types window
//! (`curtain_types_ui`).

use archisim_core::{CurtainTypeId, CurtainWallId};
use eframe::egui::{self, Id, Ui};

use super::icons::Icon;
use super::props::{self, group, row, text_row, type_card, TypeItem};
use super::*;

/// Size of the elevations of the type card.
const CARD_PICTURE: egui::Vec2 = egui::Vec2::new(260.0, 130.0);

/// Bays × rows and the largest bay, as the type's note says it.
pub(super) fn grid_note(d: &archisim_core::CurtainDesign) -> String {
    let rest = |r: archisim_core::CurtainRest| match r {
        archisim_core::CurtainRest::MaxSpacing(v) => format!("≤ {}", fmt_mm(v)),
        archisim_core::CurtainRest::Count(n) => format!("{n} partes"),
    };
    let fixed = |f: &[f64]| {
        if f.is_empty() {
            String::new()
        } else {
            let v: Vec<String> = f.iter().map(|x| fmt_mm(*x)).collect();
            format!("{} + ", v.join(" + "))
        }
    };
    format!(
        "Paños {}{} · filas {}{}",
        fixed(&d.columns.fixed),
        rest(d.columns.rest),
        fixed(&d.rows.fixed),
        rest(d.rows.rest)
    )
}

impl ArchiSimApp {
    pub(super) fn curtain_click(&mut self, p: Point2) {
        match self.curtain_anchor {
            None => self.curtain_anchor = Some(p),
            Some(a) if a.approx_eq(p) => {}
            Some(a) => {
                self.curtain_anchor = None;
                match self.ed.add_curtain_wall(&self.curtain_settings, a, p) {
                    Ok(id) => {
                        self.ed.selection = [Element::CurtainWall(id)].into_iter().collect();
                        self.say("Muro cortina creado: su retícula, marcos y paneles son los de su tipo.");
                    }
                    Err(e) => self.complain(format!("No se puede crear el muro cortina: {e}.")),
                }
            }
        }
    }

    /// The type the tool draws with.
    fn curtain_tool_type(&self) -> Option<CurtainTypeId> {
        let m = self.ed.model();
        self.curtain_settings
            .curtain_type
            .filter(|t| m.curtain_type(*t).is_some())
            .or_else(|| m.curtain_types().next().map(|t| t.id))
    }

    /// The line being drawn, with its mullions where the type puts them.
    pub(super) fn draw_curtain_preview(&self, painter: &egui::Painter, rect: Rect) {
        let (Tool::CurtainWall, Some(c), Some(a)) = (self.tool, self.cursor, self.curtain_anchor)
        else {
            return;
        };
        let ink = Stroke::new(1.5, ACCENT);
        let (pa, pb) = (
            self.view.to_screen(rect, a),
            self.view.to_screen(rect, c.point),
        );
        painter.line_segment([pa, pb], ink);
        let len = a.distance(c.point);
        let model = self.ed.model();
        let design = self
            .curtain_tool_type()
            .and_then(|t| model.curtain_type(t))
            .map(|t| t.design.clone());
        let mut bays = 0;
        if let (Some(u), Some(d)) = ((c.point - a).normalized(), design) {
            let nrm = u.perp_left() * (d.frame_depth() / 2.0);
            let lines = d.columns.lines(len);
            bays = lines.len() - 1;
            for x in lines {
                let p = a + u * x;
                painter.line_segment(
                    [
                        self.view.to_screen(rect, p - nrm),
                        self.view.to_screen(rect, p + nrm),
                    ],
                    Stroke::new(3.0, ACCENT),
                );
            }
        }
        self.draw_label(
            painter,
            rect,
            c.point,
            format!(
                "Muro cortina   L {}   {bays} paños",
                crate::units::fmt_len(len)
            ),
        );
    }

    fn curtain_length_row(ui: &mut Ui, label: &str, hint: &str, id: Id, value: f64) -> Option<f64> {
        let mut out = None;
        row(ui, label, hint, |ui, w| {
            out = length_field(ui, id, value, w - 8.0);
        });
        out
    }

    /// The project's curtain wall types for the type card, each with its
    /// elevation.
    pub(super) fn curtain_type_items(
        &mut self,
        ctx: &egui::Context,
    ) -> Vec<TypeItem<CurtainTypeId>> {
        let model = self.ed.model();
        let thumbs = &mut self.thumbs;
        model
            .curtain_types()
            .map(|t| {
                let key = format!(
                    "curtain:{:?}:{:?}",
                    t.design,
                    super::curtain_types_ui::materials_signature(model, &t.design)
                );
                let thumb = thumbs.get_drawing(ctx, key, CARD_PICTURE, || {
                    let m = crate::type_preview::curtain_model(model, &t.design, 3.6, 3.0)?;
                    crate::type_drawing::curtain_elevation(&m).map(|x| x.0)
                });
                TypeItem {
                    id: t.id,
                    name: t.name.clone(),
                    note: grid_note(&t.design),
                    thumb,
                    caption: "Alzado exterior de 3,60 × 3,00 m",
                    folder: String::new(),
                }
            })
            .collect()
    }

    pub(super) fn curtain_tool_properties(&mut self, ui: &mut Ui) {
        props::title(ui, "Herramienta Muro cortina · lo que vas a dibujar");
        let items = self.curtain_type_items(ui.ctx());
        let mut chosen = None;
        let current = self.curtain_tool_type();
        let card = type_card(
            ui,
            "tool-curtain",
            Icon::CurtainWall,
            "Cortina",
            "IfcCurtainWall",
            current,
            &items,
            &mut chosen,
            None,
        );
        if let Some(t) = chosen {
            self.curtain_settings.curtain_type = Some(t);
        }
        if card.edit {
            self.open_curtain_types(current, None);
        }
        if items.is_empty() {
            props::note(
                ui,
                "El proyecto no tiene tipos de muro cortina: al dibujar el primero se cargan los de la biblioteca.",
            );
        }
        group(ui, "tool-curtain", "Restricciones", |ui| {
            let s = &mut self.curtain_settings;
            let fields: [(&str, &str, &mut f64); 2] = [
                ("Altura (mm)", "Desde su base", &mut s.height),
                (
                    "Desfase base (mm)",
                    "Respecto a la planta",
                    &mut s.base_offset,
                ),
            ];
            for (i, (label, hint, v)) in fields.into_iter().enumerate() {
                if let Some(x) =
                    Self::curtain_length_row(ui, label, hint, Id::unique(("tool-curtain", i)), *v)
                {
                    *v = x;
                }
            }
        });
        ui.add_space(6.0);
        props::note(
            ui,
            "Dos clics: inicio y final. El exterior queda a la izquierda del trazo, como en los muros. La retícula, los marcos, los paneles y los accesorios son los del tipo: «Editar tipo» abre el editor de tipos de muro cortina.",
        );
    }

    pub(super) fn curtain_wall_properties(&mut self, ui: &mut Ui, id: CurtainWallId) {
        let Some(c) = self.ed.model().curtain_wall(id).cloned() else {
            return;
        };
        let Some(design) = self.ed.model().curtain_design(id).cloned() else {
            return;
        };
        props::title(ui, "Muro cortina · 1 seleccionado");
        let items = self.curtain_type_items(ui.ctx());
        let mut chosen = None;
        let users = self.ed.model().curtain_type_users(c.curtain_type);
        let card = type_card(
            ui,
            "curtain-type",
            Icon::CurtainWall,
            "Cortina",
            "IfcCurtainWall",
            Some(c.curtain_type),
            &items,
            &mut chosen,
            (users > 1).then_some(users),
        );
        if let Some(t) = chosen {
            let r = self
                .ed
                .edit_curtain_wall("Tipo del muro cortina", id, |c| c.curtain_type = t);
            self.report(r, "Tipo cambiado.");
        }
        if card.edit {
            self.open_curtain_types(Some(c.curtain_type), Some((c.length(), c.height)));
        }
        if card.select_all {
            let t = c.curtain_type;
            let level = self.ed.level;
            self.ed.selection = self
                .ed
                .model()
                .curtain_walls()
                .filter(|x| x.curtain_type == t && x.level == level)
                .map(|x| Element::CurtainWall(x.id))
                .collect();
        }
        let g = c.grid(&design);
        let above = self.levels_above(c.level);
        group(ui, "curtain-top", "Restricciones", |ui| {
            row(
                ui,
                "Llega a",
                "Hasta una planta (sigue sus cambios de cota) o altura propia",
                |ui, w| {
                    let current = match c.top {
                        archisim_core::WallTop::Level { level, .. } => Some(level),
                        archisim_core::WallTop::Unconnected => None,
                    };
                    let mut opts: Vec<(Option<archisim_core::LevelId>, String)> =
                        vec![(None, "Altura propia".into())];
                    opts.extend(above.iter().map(|(l, n)| (Some(*l), format!("Hasta {n}"))));
                    let o: Vec<_> = opts.iter().map(|(v, n)| (*v, n.as_str())).collect();
                    let picked = super::props::choice(ui, ("curtain-top", id.0), w, current, &o);
                    if picked != current {
                        let r =
                            self.ed
                                .edit_curtain_wall("Coronación del muro cortina", id, |c| {
                                    c.top = match picked {
                                        Some(level) => {
                                            archisim_core::WallTop::Level { level, offset: 0.0 }
                                        }
                                        None => archisim_core::WallTop::Unconnected,
                                    }
                                });
                        self.report(r, "Coronación cambiada.");
                    }
                },
            );
            type Set = fn(&mut archisim_core::CurtainWall, f64);
            let fields: [(&str, &str, f64, Set); 2] = [
                ("Altura (mm)", "Desde su base", c.height, |c, v| {
                    c.height = v;
                    c.top = archisim_core::WallTop::Unconnected;
                }),
                (
                    "Desfase base (mm)",
                    "Respecto a la planta",
                    c.base_offset,
                    |c, v| c.base_offset = v,
                ),
            ];
            for (i, (label, hint, value, set)) in fields.into_iter().enumerate() {
                if let Some(v) = Self::curtain_length_row(
                    ui,
                    label,
                    hint,
                    Id::unique(("curtain", id.0, i)),
                    value,
                ) {
                    let r = self.ed.edit_curtain_wall(label, id, |c| set(c, v));
                    self.report(r, "Muro cortina cambiado.");
                }
            }
        });
        group(ui, "curtain-data", "Retícula de este muro", |ui| {
            text_row(
                ui,
                "Longitud (mm)",
                "De inicio a final",
                &fmt_mm(c.length()),
            );
            text_row(
                ui,
                "Paños × filas",
                "Lo que da el tipo en esta longitud y altura",
                &format!("{} × {}", g.mullions.len() - 1, g.transoms.len() - 1),
            );
            let sizes = |v: &[f64]| {
                let mut s: Vec<String> = v.windows(2).map(|w| fmt_mm(w[1] - w[0])).collect();
                s.dedup();
                s.join(" · ")
            };
            text_row(
                ui,
                "Paños (mm)",
                "Entre ejes, desde el inicio",
                &sizes(&g.mullions),
            );
            text_row(
                ui,
                "Filas (mm)",
                "Entre ejes, desde abajo",
                &sizes(&g.transoms),
            );
        });
        self.curtain_doors_group(ui, id, &c, &design);
        group(ui, "curtain-identity", "Datos de identidad", |ui| {
            props::id_row(ui, "GlobalId", &c.guid.to_string());
        });
    }

    /// The doors of a curtain wall: each in the bottom cell of a bay, of a
    /// door type of the project, with its hand and the side it opens to;
    /// add one in a free bay, take one out (its glass comes back).
    fn curtain_doors_group(
        &mut self,
        ui: &mut Ui,
        id: CurtainWallId,
        c: &archisim_core::CurtainWall,
        design: &archisim_core::CurtainDesign,
    ) {
        use archisim_core::{CurtainDoor, Hand, OpeningKind, WallSide};
        let bays = c.bays(design);
        let door_types: Vec<(archisim_core::OpeningTypeId, String)> = self
            .ed
            .model()
            .opening_types()
            .filter(|t| matches!(t.kind, OpeningKind::Door(_)))
            .map(|t| (t.id, t.name.clone()))
            .collect();
        if door_types.is_empty() {
            return;
        }
        group(ui, "curtain-doors", "Puertas", |ui| {
            let bay_names: Vec<(usize, String)> =
                (0..bays).map(|b| (b, format!("Paño {}", b + 1))).collect();
            let bay_opts: Vec<(usize, &str)> =
                bay_names.iter().map(|(b, n)| (*b, n.as_str())).collect();
            let type_opts: Vec<(archisim_core::OpeningTypeId, &str)> =
                door_types.iter().map(|(t, n)| (*t, n.as_str())).collect();
            let mut change: Option<(usize, CurtainDoor)> = None;
            let mut remove = None;
            for (i, door) in c.doors.iter().enumerate() {
                let mut d = door.clone();
                row(
                    ui,
                    &format!("Puerta {}", i + 1),
                    "El paño que ocupa",
                    |ui, w| {
                        d.bay = props::choice(ui, ("cw-door-bay", id.0, i), w, d.bay, &bay_opts);
                    },
                );
                row(ui, "Tipo", "Un tipo de puerta del proyecto", |ui, w| {
                    d.opening_type =
                        props::choice(ui, ("cw-door-type", id.0, i), w, d.opening_type, &type_opts);
                });
                row(
                    ui,
                    "Mano",
                    "Vista desde el lado hacia el que abre",
                    |ui, w| {
                        d.hand = props::choice(
                            ui,
                            ("cw-door-hand", id.0, i),
                            w,
                            d.hand,
                            &[(Hand::Left, "Izquierda"), (Hand::Right, "Derecha")],
                        );
                    },
                );
                row(ui, "Abre hacia", "El lado al que van sus hojas", |ui, w| {
                    d.opens_to = props::choice(
                        ui,
                        ("cw-door-side", id.0, i),
                        w,
                        d.opens_to,
                        &[
                            (WallSide::Interior, "Interior"),
                            (WallSide::Exterior, "Exterior"),
                        ],
                    );
                });
                if d != *door {
                    change = Some((i, d));
                }
                if ui.button("Quitar esta puerta").clicked() {
                    remove = Some(i);
                }
                ui.separator();
            }
            let free = (0..bays).find(|b| !c.doors.iter().any(|d| d.bay == *b));
            if let Some(bay) = free {
                if ui
                    .button(format!("Añadir puerta en el paño {}", bay + 1))
                    .on_hover_text("En la celda de abajo del paño, en lugar de su vidrio")
                    .clicked()
                {
                    let t = door_types[0].0;
                    let r = self
                        .ed
                        .edit_curtain_wall("Puerta en el muro cortina", id, |c| {
                            c.doors.push(CurtainDoor {
                                guid: archisim_core::GlobalId::new(),
                                bay,
                                opening_type: t,
                                hand: Hand::Left,
                                opens_to: WallSide::Interior,
                            })
                        });
                    self.report(r, "Puerta añadida.");
                }
            }
            if let Some((i, d)) = change {
                let r = self
                    .ed
                    .edit_curtain_wall("Puerta del muro cortina", id, |c| c.doors[i] = d);
                self.report(r, "Puerta cambiada.");
            }
            if let Some(i) = remove {
                let r = self
                    .ed
                    .edit_curtain_wall("Quitar puerta del muro cortina", id, |c| {
                        c.doors.remove(i);
                    });
                self.report(r, "Puerta quitada.");
            }
        });
    }
}
