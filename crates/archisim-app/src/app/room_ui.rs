//! Properties of rooms and tags.

use archisim_core::{RoomId, TagId, TagTarget};
use eframe::egui::{Id, Ui};

use super::icons::Icon;
use super::properties::view_card;
use super::props::{self, group, id_row, row, text_row};
use super::*;

impl ArchiSimApp {
    pub(super) fn room_properties(&mut self, ui: &mut Ui, id: RoomId) {
        let model = self.ed.model();
        let Some(r) = model.room(id).cloned() else {
            return;
        };
        let height = model.room_height(&r);
        let gross =
            archisim_geom::room_outline(model, r.level, r.label, archisim_geom::Contour::WallAxis)
                .map(|o| archisim_core::signed_area(&o).abs())
                .unwrap_or(0.0)
                .max(r.area());
        props::title(ui, "Local · 1 seleccionado");
        view_card(
            ui,
            Icon::Room,
            "Local",
            "IfcSpace: se exporta con su superficie, volumen y acabados",
        );
        let mut new = r.clone();
        group(ui, "room-id", "Identificación", |ui| {
            row(ui, "Número", "", |ui, w| {
                if let Some(t) =
                    text_edit_committed(ui, Id::unique(("room-number", id)), &r.number, w - 8.0)
                {
                    new.number = t;
                }
            });
            row(ui, "Nombre", "", |ui, w| {
                if let Some(t) =
                    text_edit_committed(ui, Id::unique(("room-name", id)), &r.name, w - 8.0)
                {
                    new.name = t;
                }
            });
        });
        self.usage_row(ui, &format!("room-usage-{}", id.0), Some(id));
        group(ui, "room-place", "Restricciones", |ui| {
            self.level_row(
                ui,
                ("room-level", id),
                "Planta",
                Some(r.level),
                &[Element::Room(id)],
            );
            id_row(ui, "GlobalId", &r.guid.to_string());
        });
        group(ui, "room-sizes", "Superficies", |ui| {
            text_row(
                ui,
                "Superficie útil",
                "Hasta las caras interiores, sin los huecos (pilares, patinillos)",
                &archisim_plan::rooms::format_area(r.area()),
            );
            text_row(
                ui,
                "Superficie construida",
                "Hasta los ejes de los muros que lo rodean",
                &archisim_plan::rooms::format_area(gross),
            );
            text_row(
                ui,
                "Perímetro (m)",
                "Longitud de rodapié",
                &fmt_m(r.perimeter()),
            );
            row(
                ui,
                "Altura libre (mm)",
                "Vacío: hasta la planta superior",
                |ui, w| {
                    if let Some(h) =
                        length_field(ui, Id::unique(("room-height", id)), height, w - 8.0)
                    {
                        if h > 0.0 {
                            new.height = Some(h);
                        }
                    }
                },
            );
            if r.height.is_some() && ui.small_button("Hasta la planta superior").clicked() {
                new.height = None;
            }
            text_row(
                ui,
                "Volumen",
                "",
                &format!("{:.2} m³", r.area() * height).replace('.', ","),
            );
            row(
                ui,
                "Sigue a los muros",
                "Su contorno se rehace desde su rótulo cuando cambian los muros que lo cierran",
                |ui, _| {
                    ui.checkbox(&mut new.bounded, "");
                },
            );
        });
        group(ui, "room-finishes", "Acabados", |ui| {
            for (name, which) in [("Suelo", 0), ("Paredes", 1), ("Techo", 2)] {
                let current = match which {
                    0 => r.finishes.floor.clone(),
                    1 => r.finishes.walls.clone(),
                    _ => r.finishes.ceiling.clone(),
                };
                row(ui, name, "", |ui, w| {
                    if let Some(t) = text_edit_committed(
                        ui,
                        Id::unique(("room-finish", id, which)),
                        &current,
                        w - 8.0,
                    ) {
                        match which {
                            0 => new.finishes.floor = t,
                            1 => new.finishes.walls = t,
                            _ => new.finishes.ceiling = t,
                        }
                    }
                });
            }
        });
        let center = archisim_core::label_center(&r.outline);
        if r.label.distance(center) > 1e-6 && ui.button("Rótulo al centro").clicked() {
            new.label = center;
        }
        if new != r {
            let r = self.ed.set_room(new, "Cambiar local");
            self.report(r, "Local cambiado.");
        }
        ui.add_space(4.0);
        props::note(
            ui,
            "Arrastre el rótulo por su asa para moverlo dentro del local. Para crear locales: herramienta Local (Z) o la orden LOCAL. La lista de usos está en las propiedades del proyecto (sin nada seleccionado).",
        );
    }

    pub(super) fn tag_properties(&mut self, ui: &mut Ui, id: TagId) {
        let model = self.ed.model();
        let Some(t) = model.tag(id).cloned() else {
            return;
        };
        let what = match t.target {
            TagTarget::Opening(_) => "Etiqueta de carpintería",
            TagTarget::Wall(_) => "Etiqueta de muro",
            TagTarget::Room(_) => "Etiqueta de local",
        };
        props::title(ui, "Etiqueta · 1 seleccionada");
        view_card(
            ui,
            Icon::Tag,
            what,
            "Escribe lo que es su elemento y cambia con él",
        );
        let lines = archisim_plan::rooms::tag_lines(model, &t);
        let mut leader = t.leader;
        group(ui, "tag", "Etiqueta", |ui| {
            text_row(ui, "Texto", "", &lines.join(" · "));
            row(
                ui,
                "Línea de referencia",
                "Una línea hasta el elemento",
                |ui, _| {
                    ui.checkbox(&mut leader, "");
                },
            );
        });
        if leader != t.leader {
            let r = self.ed.set_tag_leader(id, leader);
            self.report(r, "Etiqueta cambiada.");
        }
        ui.add_space(4.0);
        props::note(
            ui,
            "La marca y el tamaño salen del tipo de la puerta o ventana (Tipos y materiales). ETIQUETA y T etiqueta todas las de la planta.",
        );
    }
}
