//! The room tool ("Local", ArchiCAD's zone, Revit's room): its settings
//! before placing a room, its clicks and the outline being drawn; and the
//! project's list of room uses. The properties of a selected room are in
//! `room_ui`.

use archisim_core::RoomId;
use archisim_geom::Contour;
use eframe::egui::{self, Id, Ui};

use super::props::{self, choice, group, row, text_row};
use super::*;
use crate::editor::{usages_of, USAGE_SETS};
use crate::slab_tool::Outcome as PolyOutcome;

impl ArchiSimApp {
    /// Whether a click with the room tool takes the room around it.
    pub(super) fn zone_wand(&self) -> bool {
        self.room_settings.wand && !self.zone_tool.is_drawing()
    }

    pub(super) fn room_tool_properties(&mut self, ui: &mut Ui) {
        props::title(ui, "Herramienta Local · lo que vas a crear");
        let level = self.ed.level;
        let next = self.ed.model().next_room_number(level);
        group(ui, "tool-room-method", "Contorno", |ui| {
            if self.room_settings.wand {
                row(
                    ui,
                    "Contorno",
                    "Superficie útil: caras interiores",
                    |ui, w| {
                        self.room_settings.contour = choice(
                            ui,
                            "tool-room-contour",
                            w,
                            self.room_settings.contour,
                            &[
                                (Contour::InteriorFace, "Caras interiores"),
                                (Contour::WallAxis, "Ejes de muros"),
                            ],
                        );
                    },
                );
            }
        });
        group(ui, "tool-room-data", "Datos", |ui| {
            text_row(ui, "Número", "El siguiente libre de esta planta", &next);
            row(ui, "Nombre", "Dormitorio 1, Cocina…", |ui, w| {
                let shown = self.room_settings.name.clone();
                if let Some(v) =
                    text_edit_committed(ui, Id::unique("tool-room-name"), &shown, w - 8.0)
                {
                    self.room_settings.name = v;
                }
            });
            self.usage_row(ui, "tool-room-usage", None);
            row(
                ui,
                "Altura libre (mm)",
                "Vacío: de la planta a la de encima",
                |ui, w| {
                    let shown = self.room_settings.height.unwrap_or(0.0);
                    if let Some(v) =
                        length_field(ui, Id::unique("tool-room-height"), shown, w - 8.0)
                    {
                        self.room_settings.height = (v > 0.0).then_some(v);
                    }
                },
            );
            if self.room_settings.height.is_some()
                && ui.small_button("Hasta la planta superior").clicked()
            {
                self.room_settings.height = None;
            }
        });
    }

    /// The use of a room, from the project's list (the name follows it if
    /// it was the default one). `room`: a placed room; `None`: the tool's.
    pub(super) fn usage_row(&mut self, ui: &mut Ui, salt: &str, room: Option<RoomId>) {
        let current = match room {
            Some(id) => self
                .ed
                .model()
                .room(id)
                .map(|r| r.usage.clone())
                .unwrap_or_default(),
            None => self.room_settings.usage.clone(),
        };
        let usages = usages_of(self.ed.model());
        let mut options: Vec<(&str, &str)> = vec![("", "—")];
        options.extend(usages.iter().map(|u| (u.as_str(), u.as_str())));
        if !current.is_empty() && !usages.contains(&current) {
            options.push((current.as_str(), current.as_str()));
        }
        let picked = row(
            ui,
            "Uso",
            "El uso del local (IfcSpace.ObjectType); la lista, en las propiedades del proyecto",
            |ui, w| choice(ui, salt, w, current.as_str(), &options).to_string(),
        );
        if picked == current {
            return;
        }
        match room {
            Some(id) => {
                let r = self.ed.edit_room("Uso del local", id, |r| {
                    if r.name == r.usage || r.name == "Local" {
                        r.name = picked.clone();
                    }
                    r.usage = picked.clone();
                });
                self.report(r, "Uso cambiado.");
            }
            None => {
                let s = &mut self.room_settings;
                if s.name == s.usage || s.name == "Local" {
                    s.name = picked.clone();
                }
                s.usage = picked;
            }
        }
    }

    /// The project's list of room uses, in the project's properties: each
    /// use can be removed, new ones typed, and whole sets for other kinds
    /// of building added (offices, schools, health…).
    pub(super) fn usages_group(&mut self, ui: &mut Ui) {
        let usages = usages_of(self.ed.model());
        let mut changed: Option<Vec<String>> = None;
        props::group(ui, "project-usages", "Usos de los locales", |ui| {
            for (i, u) in usages.iter().enumerate() {
                ui.horizontal(|ui| {
                    ui.add_space(8.0);
                    ui.label(u);
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui
                            .small_button("✕")
                            .on_hover_text("Quitar de la lista")
                            .clicked()
                        {
                            let mut v = usages.clone();
                            v.remove(i);
                            changed = Some(v);
                        }
                    });
                });
            }
            row(ui, "Nuevo uso", "Escríbelo y pulsa Intro", |ui, w| {
                if let Some(new) =
                    text_edit_committed(ui, egui::Id::unique("project-usage-new"), "", w - 8.0)
                {
                    let new = new.trim().to_string();
                    if !new.is_empty() && !usages.contains(&new) {
                        let mut v = usages.clone();
                        v.push(new);
                        changed = Some(v);
                    }
                }
            });
            row(
                ui,
                "Añadir conjunto",
                "Usos habituales de otro tipo de edificio",
                |ui, w| {
                    let mut opts: Vec<(usize, &str)> = vec![(usize::MAX, "—")];
                    opts.extend(USAGE_SETS.iter().enumerate().map(|(i, (n, _))| (i, *n)));
                    let picked = choice(ui, "project-usage-set", w, usize::MAX, &opts);
                    if let Some((_, set)) = USAGE_SETS.get(picked) {
                        let mut v = usages.clone();
                        for u in set.iter() {
                            if !v.iter().any(|x| x == u) {
                                v.push(u.to_string());
                            }
                        }
                        changed = Some(v);
                    }
                },
            );
        });
        if let Some(v) = changed {
            let r = self.ed.set_room_usages(v);
            self.report(r, "Usos del proyecto cambiados.");
        }
    }

    /// A click of the room tool.
    pub(super) fn space_click(&mut self, p: Point2) {
        if self.zone_wand() {
            match self.ed.add_room_in_room(&self.room_settings, p) {
                Ok(id) => self.room_created(id),
                Err(e) => self.complain(format!("No se puede crear el local: {e}.")),
            }
            return;
        }
        match self.zone_tool.click(p) {
            PolyOutcome::Build(outline) => self.build_space(outline),
            PolyOutcome::Invalid => {
                self.complain("El contorno no puede cruzarse: corrige la última esquina.")
            }
            PolyOutcome::Nothing => {}
        }
    }

    pub(super) fn build_space(&mut self, outline: Vec<Point2>) {
        match self.ed.add_room_outline(&self.room_settings, outline) {
            Ok(id) => {
                self.zone_tool.accept();
                self.room_created(id);
            }
            Err(e) => self.complain(format!("No se puede crear el local: {e}.")),
        }
    }

    fn room_created(&mut self, id: RoomId) {
        let (num, name, area) = self
            .ed
            .model()
            .room(id)
            .map(|r| (r.number.clone(), r.name.clone(), r.area()))
            .unwrap_or_default();
        self.ed.selection = [Element::Room(id)].into_iter().collect();
        self.say(format!(
            "Local {num} «{name}» de {} útiles. Siguiente: clic en otro recinto.",
            fmt_area(area)
        ));
    }

    /// The outline being drawn with the room tool.
    pub(super) fn draw_space_preview(&self, painter: &egui::Painter, rect: Rect) {
        let (Tool::Zone, Some(c)) = (self.tool, self.cursor) else {
            return;
        };
        let corners = self.zone_tool.corners();
        if corners.is_empty() {
            return;
        }
        let ink = Stroke::new(1.5, ACCENT);
        if self.zone_tool.mode == SlabMode::Rectangle {
            if let Some(outline) = self.zone_tool.rectangle_to(c.point) {
                painter.add(Shape::closed_line(self.polygon(rect, &outline), ink));
                self.draw_label(
                    painter,
                    rect,
                    c.point,
                    format!("S {}", fmt_area(archisim_core::signed_area(&outline).abs())),
                );
            }
            return;
        }
        let mut pts = corners.to_vec();
        pts.push(self.zone_tool.typed_end(c.point).unwrap_or(c.point));
        let screen = self.polygon(rect, &pts);
        painter.add(Shape::line(screen.clone(), ink));
        if pts.len() >= 3 {
            dashed(painter, screen[screen.len() - 1], screen[0], ACCENT);
            self.draw_label(
                painter,
                rect,
                c.point,
                format!("S {}", fmt_area(archisim_core::signed_area(&pts).abs())),
            );
        }
    }
}
