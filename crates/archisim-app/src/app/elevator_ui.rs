//! The lift tool (H4): a lift placed with a click by the centre of its
//! shaft, its doors looking down the screen (Espacio turns it), from the
//! level being drawn up to the highest one (or the one chosen); and the
//! properties of a lift.

use archisim_core::{ElevatorId, LevelId, NewElevator, Point2, ELEVATOR_PRESETS};
use eframe::egui::{ComboBox, Id, Ui};

use super::props::{self, group, row, text_row};
use super::structure_ui::angle_field;
use super::*;

/// What the lift tool places.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ElevatorSettings {
    /// Which of [`ELEVATOR_PRESETS`].
    pub preset: usize,
    /// Its highest stop; `None`, the highest level of the project.
    pub top: Option<LevelId>,
    pub wall: f64,
    pub pit: f64,
    pub overrun: f64,
    pub rotation: f64,
    pub cut_slabs: bool,
}

impl Default for ElevatorSettings {
    fn default() -> Self {
        ElevatorSettings {
            preset: 1,
            top: None,
            wall: 0.2,
            pit: 1.2,
            overrun: 3.6,
            rotation: 0.0,
            cut_slabs: true,
        }
    }
}

fn comma(v: f64, d: usize) -> String {
    format!("{v:.d$}").replace('.', ",")
}

impl ArchiSimApp {
    /// The highest level of the project: the lift's top by default.
    fn default_top(&self) -> LevelId {
        let mut levels = self.ed.model().levels();
        levels.sort_by(|a, b| a.elevation.total_cmp(&b.elevation));
        levels.last().map_or(self.ed.level, |l| l.id)
    }

    pub(super) fn elevator_click(&mut self, p: Point2) {
        let s = self.elevator_settings;
        let level = self.ed.level;
        let top = s.top.unwrap_or_else(|| self.default_top());
        let mut spec = NewElevator::new(level, top, p)
            .with_preset(&ELEVATOR_PRESETS[s.preset.min(ELEVATOR_PRESETS.len() - 1)]);
        spec.wall = s.wall;
        spec.pit = s.pit;
        spec.overrun = s.overrun;
        spec.rotation = s.rotation;
        spec.cut_slabs = s.cut_slabs;
        match self
            .ed
            .transact("Colocar ascensor", |m| m.add_elevator(spec))
        {
            Ok(id) => {
                self.ed.selection = [Element::Elevator(id)].into_iter().collect();
                let stops = self.ed.model().elevator_stops(id).len();
                self.say(format!("Ascensor con {stops} paradas."));
            }
            Err(e) => self.complain(format!("No se puede colocar el ascensor: {e}.")),
        }
    }

    /// Espacio turns the lift to be placed a quarter.
    pub(super) fn elevator_turn(&mut self) {
        self.elevator_settings.rotation = crate::editor::normal_angle(
            self.elevator_settings.rotation + std::f64::consts::FRAC_PI_2,
        );
    }

    pub(super) fn elevator_hint(&self) -> &'static str {
        "Clic en el centro del hueco del ascensor · Espacio gira las puertas"
    }

    fn preset_combo(ui: &mut Ui, id: Id, value: usize, w: f32) -> usize {
        let mut v = value;
        ComboBox::from_id_salt(id)
            .width(w - 8.0)
            .selected_text(ELEVATOR_PRESETS.get(value).map_or("A medida", |p| p.name))
            .show_ui(ui, |ui| {
                for (i, p) in ELEVATOR_PRESETS.iter().enumerate() {
                    ui.selectable_value(&mut v, i, p.name);
                }
            });
        v
    }

    fn level_combo(
        &self,
        ui: &mut Ui,
        id: Id,
        value: Option<LevelId>,
        none: &str,
        w: f32,
    ) -> Option<LevelId> {
        let mut v = value;
        let mut levels = self.ed.model().levels();
        levels.sort_by(|a, b| a.elevation.total_cmp(&b.elevation));
        let shown = value
            .and_then(|l| levels.iter().find(|x| x.id == l))
            .map_or(none.to_string(), |l| l.name.clone());
        ComboBox::from_id_salt(id)
            .width(w - 8.0)
            .selected_text(shown)
            .show_ui(ui, |ui| {
                ui.selectable_value(&mut v, None, none);
                for l in &levels {
                    ui.selectable_value(&mut v, Some(l.id), l.name.as_str());
                }
            });
        v
    }

    pub(super) fn elevator_tool_properties(&mut self, ui: &mut Ui) {
        props::title(ui, "Herramienta Ascensor · lo que vas a colocar");
        let mut s = self.elevator_settings;
        group(ui, "tool-lift", "Ascensor", |ui| {
            row(
                ui,
                "Modelo",
                "Carga, cabina, puerta y hueco (EN 81-20 y EN 81-70)",
                |ui, w| {
                    s.preset = Self::preset_combo(ui, Id::unique("tool-lift-preset"), s.preset, w);
                },
            );
            row(
                ui,
                "Última parada",
                "La planta más alta a la que llega",
                |ui, w| {
                    s.top =
                        self.level_combo(ui, Id::unique("tool-lift-top"), s.top, "La más alta", w);
                },
            );
            for (i, (name, hint, value)) in [
                (
                    "Muros del hueco (mm)",
                    "0: el hueco se dibuja con muros aparte",
                    s.wall,
                ),
                ("Foso (mm)", "Bajo la parada más baja", s.pit),
                (
                    "Huida (mm)",
                    "Desde el suelo de la última parada hasta el techo del hueco",
                    s.overrun,
                ),
            ]
            .into_iter()
            .enumerate()
            {
                row(ui, name, hint, |ui, w| {
                    if let Some(v) = length_field(ui, Id::unique(("tool-lift", i)), value, w - 8.0)
                    {
                        match i {
                            0 => s.wall = v.max(0.0),
                            1 => s.pit = v.max(0.0),
                            _ => s.overrun = v.max(0.0),
                        }
                    }
                });
            }
            ui.checkbox(&mut s.cut_slabs, "Abrir los huecos en los forjados");
            props::note(
                ui,
                "Clic en el centro del hueco; Espacio gira las puertas. Para usar un hueco ya dibujado con muros, pon a 0 sus muros.",
            );
        });
        self.elevator_settings = s;
    }

    pub(super) fn elevator_properties(&mut self, ui: &mut Ui, id: ElevatorId) {
        let Some(e) = self.ed.model().elevator(id).cloned() else {
            return;
        };
        props::title(ui, &format!("{} · 1 seleccionado", e.name));
        let edit = |app: &mut Self, what: &str, f: &dyn Fn(&mut archisim_core::Elevator)| {
            let r = app.ed.transact(what, |m| m.edit_elevator(id, |x| f(x)));
            app.report(r, "Cambiado.");
        };
        group(ui, "lift-data", "Ascensor", |ui| {
            row(ui, "Nombre", "", |ui, w| {
                if let Some(t) =
                    text_edit_committed(ui, Id::unique(("lift-name", id)), &e.name, w - 8.0)
                {
                    let t = t.trim().to_string();
                    edit(self, "Nombre del ascensor", &|x| x.name = t.clone());
                }
            });
            row(
                ui,
                "Modelo",
                "Cambia cabina, puerta, carga y hueco",
                |ui, w| {
                    let now = ELEVATOR_PRESETS
                        .iter()
                        .position(|p| {
                            p.load == e.load
                                && (p.car.0 - e.car_w).abs() < 1e-6
                                && (p.car.1 - e.car_d).abs() < 1e-6
                        })
                        .unwrap_or(usize::MAX);
                    let n = Self::preset_combo(ui, Id::unique(("lift-preset", id)), now, w);
                    if n != now && n < ELEVATOR_PRESETS.len() {
                        let p = &ELEVATOR_PRESETS[n];
                        edit(self, "Modelo de ascensor", &|x| {
                            x.car_w = p.car.0;
                            x.car_d = p.car.1;
                            x.door_w = p.door;
                            x.shaft_w = p.shaft.0;
                            x.shaft_d = p.shaft.1;
                            x.load = p.load;
                        });
                    }
                },
            );
            text_row(
                ui,
                "Carga",
                "",
                &format!("{} kg · {} personas", e.load, e.persons()),
            );
            row(ui, "Primera parada", "", |ui, w| {
                if let Some(l) =
                    self.level_combo(ui, Id::unique(("lift-level", id)), Some(e.level), "—", w)
                {
                    if l != e.level {
                        edit(self, "Primera parada del ascensor", &|x| x.level = l);
                    }
                }
            });
            row(ui, "Última parada", "", |ui, w| {
                if let Some(l) =
                    self.level_combo(ui, Id::unique(("lift-top", id)), Some(e.top), "—", w)
                {
                    if l != e.top {
                        edit(self, "Última parada del ascensor", &|x| x.top = l);
                    }
                }
            });
            row(
                ui,
                "Cabina en",
                "La planta en la que se dibuja la cabina",
                |ui, w| {
                    let v = self.level_combo(
                        ui,
                        Id::unique(("lift-car", id)),
                        e.car_at,
                        "La primera parada",
                        w,
                    );
                    if v != e.car_at {
                        edit(self, "Cabina del ascensor", &|x| x.car_at = v);
                    }
                },
            );
            let stops = self.ed.model().elevator_stops(id);
            text_row(ui, "Paradas", "", &stops.len().to_string());
            row(
                ui,
                "Giro (°)",
                "Las puertas miran a su +Y girado",
                |ui, w| {
                    if let Some(a) =
                        angle_field(ui, Id::unique(("lift-rot", id)), e.rotation, w - 8.0)
                    {
                        edit(self, "Giro del ascensor", &|x| x.rotation = a);
                    }
                },
            );
        });
        group(ui, "lift-sizes", "Medidas", |ui| {
            let fields: [(&str, &str, f64); 10] = [
                ("Hueco, ancho (mm)", "Por dentro", e.shaft_w),
                ("Hueco, fondo (mm)", "Por dentro", e.shaft_d),
                (
                    "Muros del hueco (mm)",
                    "0: el hueco se dibuja con muros aparte",
                    e.wall,
                ),
                ("Foso (mm)", "", e.pit),
                ("Huida (mm)", "", e.overrun),
                ("Cabina, ancho (mm)", "Por dentro", e.car_w),
                ("Cabina, fondo (mm)", "Por dentro", e.car_d),
                ("Cabina, alto (mm)", "", e.car_h),
                ("Puerta, paso (mm)", "", e.door_w),
                ("Puerta, alto (mm)", "", e.door_h),
            ];
            for (i, (name, hint, value)) in fields.into_iter().enumerate() {
                row(ui, name, hint, |ui, w| {
                    if let Some(v) = length_field(ui, Id::unique(("lift-f", id, i)), value, w - 8.0)
                    {
                        edit(self, "Medidas del ascensor", &|x| match i {
                            0 => x.shaft_w = v,
                            1 => x.shaft_d = v,
                            2 => x.wall = v.max(0.0),
                            3 => x.pit = v.max(0.0),
                            4 => x.overrun = v.max(0.0),
                            5 => x.car_w = v,
                            6 => x.car_d = v,
                            7 => x.car_h = v,
                            8 => x.door_w = v,
                            _ => x.door_h = v,
                        });
                    }
                });
            }
            let mut cut = e.cut_slabs;
            if ui
                .checkbox(&mut cut, "Abrir los huecos en los forjados")
                .changed()
            {
                edit(self, "Huecos del ascensor", &|x| x.cut_slabs = cut);
            }
        });
        group(ui, "lift-identity", "Datos de identidad", |ui| {
            text_row(ui, "Clase IFC", "", "IfcTransportElement (ELEVATOR)");
            text_row(
                ui,
                "Recorrido",
                "",
                &format!(
                    "{} m",
                    comma(
                        self.ed
                            .model()
                            .elevator_stop_heights(id)
                            .last()
                            .copied()
                            .unwrap_or(0.0),
                        2
                    )
                ),
            );
        });
    }
}
