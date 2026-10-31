//! Instalaciones: the pipe and duct tools (a path by its points, at a
//! height, of a system and size), the device tool (equipment, lights,
//! sockets and switches, by a click) and their properties.

use archisim_core::{DeviceId, DeviceKind, NewDevice, NewRun, Point2, RunId, ServiceSystem};
use eframe::egui::{self, ComboBox, Id, Ui};

use super::props::{self, group, row, text_row};
use super::structure_ui::angle_field;
use super::*;

/// What the pipe, duct and device tools make.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ServiceSettings {
    pub pipe_system: ServiceSystem,
    /// A pipe's diameter.
    pub pipe_size: f64,
    /// Its axis over the level (at its first point).
    pub pipe_elevation: f64,
    /// The fall of a drain, from its first point to its last.
    pub drain_slope: f64,
    pub duct_system: ServiceSystem,
    pub duct_width: f64,
    pub duct_height: f64,
    pub duct_elevation: f64,
    pub device: DeviceKind,
    pub device_rotation: f64,
    /// The bottom of the device over the level.
    pub device_elevation: f64,
}

impl Default for ServiceSettings {
    fn default() -> Self {
        ServiceSettings {
            pipe_system: ServiceSystem::ColdWater,
            pipe_size: 0.025,
            pipe_elevation: 0.3,
            drain_slope: 0.02,
            duct_system: ServiceSystem::SupplyAir,
            duct_width: 0.3,
            duct_height: 0.2,
            duct_elevation: 2.6,
            device: DeviceKind::Light,
            device_rotation: 0.0,
            device_elevation: DeviceKind::Light.defaults().3,
        }
    }
}

impl ServiceSettings {
    /// Takes another kind of device, at its usual height.
    pub fn set_device(&mut self, kind: DeviceKind) {
        if self.device != kind {
            self.device = kind;
            self.device_elevation = kind.defaults().3;
        }
    }
}

fn comma(v: f64, d: usize) -> String {
    format!("{v:.d$}").replace('.', ",")
}

/// A slope as a percentage («2», «1,5»).
fn fmt_percent(slope: f64) -> String {
    let v = (slope * 1000.0).round() / 10.0;
    format!("{v}").replace('.', ",")
}

/// A field with a percentage: the slope it says, from 0 to 1.
fn percent_field(ui: &mut Ui, id: Id, slope: f64, width: f32) -> Option<f64> {
    let text = text_edit_committed(ui, id, &fmt_percent(slope), width)?;
    text.trim()
        .trim_end_matches('%')
        .trim()
        .replace(',', ".")
        .parse::<f64>()
        .ok()
        .filter(|v| v.is_finite())
        .map(|v| (v / 100.0).clamp(0.0, 1.0))
}

fn system_combo(ui: &mut Ui, id: Id, ducts: bool, value: ServiceSystem, w: f32) -> ServiceSystem {
    let mut v = value;
    let list: &[ServiceSystem] = if ducts {
        &ServiceSystem::DUCTS
    } else {
        &ServiceSystem::PIPES
    };
    ComboBox::from_id_salt(id)
        .width(w - 8.0)
        .selected_text(value.name())
        .show_ui(ui, |ui| {
            for s in list {
                ui.selectable_value(&mut v, *s, s.name());
            }
        });
    v
}

fn kind_combo(ui: &mut Ui, id: Id, value: DeviceKind, w: f32) -> DeviceKind {
    let mut v = value;
    ComboBox::from_id_salt(id)
        .width(w - 8.0)
        .selected_text(value.name())
        .show_ui(ui, |ui| {
            for k in DeviceKind::ALL {
                ui.selectable_value(&mut v, k, k.name());
            }
        });
    v
}

impl ArchiSimApp {
    // ---- Pipes and ducts ----------------------------------------------------

    pub(super) fn run_click(&mut self, p: Point2) {
        if self.run_points.last().is_some_and(|l| l.approx_eq(p)) {
            self.finish_run();
        } else {
            self.run_points.push(p);
        }
    }

    pub(super) fn finish_run(&mut self) {
        let path = std::mem::take(&mut self.run_points);
        if path.len() < 2 {
            return;
        }
        let s = self.service_settings;
        let duct = self.tool == Tool::Duct;
        let spec = NewRun {
            level: self.ed.level,
            system: if duct { s.duct_system } else { s.pipe_system },
            path,
            elevation: if duct {
                s.duct_elevation
            } else {
                s.pipe_elevation
            },
            width: if duct { s.duct_width } else { s.pipe_size },
            height: if duct { s.duct_height } else { s.pipe_size },
            slope: if !duct && s.pipe_system == ServiceSystem::Drainage {
                s.drain_slope
            } else {
                0.0
            },
        };
        let what = if duct { "conducto" } else { "tubería" };
        match self
            .ed
            .transact(&format!("Crear {what}"), |m| m.add_run(spec))
        {
            Ok(id) => {
                self.ed.selection = [Element::Run(id)].into_iter().collect();
                if let Some(r) = self.ed.model().run(id) {
                    let text = format!(
                        "{} {}: {} m.",
                        r.system.name(),
                        r.size_text(),
                        comma(r.length(), 2)
                    );
                    self.say(text);
                }
            }
            Err(e) => self.complain(format!("No se puede crear la {what}: {e}.")),
        }
    }

    // ---- Devices ------------------------------------------------------------

    pub(super) fn device_click(&mut self, p: Point2) {
        let s = self.service_settings;
        let (w, d, h, _) = s.device.defaults();
        let spec = NewDevice {
            level: self.ed.level,
            kind: s.device,
            name: s.device.name().to_string(),
            position: p,
            rotation: s.device_rotation,
            elevation: s.device_elevation,
            width: w,
            depth: d,
            height: h,
        };
        match self.ed.transact("Colocar aparato", |m| m.add_device(spec)) {
            Ok(id) => {
                self.ed.selection = [Element::Device(id)].into_iter().collect();
                self.say(format!(
                    "{} a {} del suelo.",
                    s.device.name(),
                    crate::units::fmt_len(s.device_elevation)
                ));
            }
            Err(e) => self.complain(format!("No se puede colocar: {e}.")),
        }
    }

    /// Takes the device tool with a kind of device.
    pub(super) fn device_pick(&mut self, kind: DeviceKind) {
        self.service_settings.set_device(kind);
        self.set_tool(Tool::Device);
    }

    // ---- Preview ------------------------------------------------------------

    pub(super) fn draw_services_preview(&self, painter: &egui::Painter, rect: Rect) {
        let Some(c) = self.cursor else {
            return;
        };
        let s = &self.service_settings;
        match self.tool {
            Tool::Pipe | Tool::Duct if !self.run_points.is_empty() => {
                let (system, width) = if self.tool == Tool::Duct {
                    (s.duct_system, s.duct_width)
                } else {
                    (s.pipe_system, s.pipe_size)
                };
                let [r, g, b] = system.color();
                let ink = egui::Color32::from_rgb(r, g, b);
                let mut path = self.run_points.clone();
                path.push(c.point);
                painter.add(Shape::line(
                    self.polygon(rect, &path),
                    Stroke::new(1.5, ink),
                ));
                let band = archisim_core::strip(&path, -width / 2.0, width / 2.0);
                if band.len() > 2 {
                    painter.add(Shape::closed_line(
                        self.polygon(rect, &band),
                        Stroke::new(1.0, ink.gamma_multiply(0.6)),
                    ));
                }
                let len: f64 = path.windows(2).map(|x| x[0].distance(x[1])).sum();
                self.draw_label(painter, rect, c.point, format!("L {} m", comma(len, 2)));
            }
            Tool::Device => {
                let (w, d, _, _) = s.device.defaults();
                let ghost = archisim_core::ServiceDevice {
                    id: DeviceId(0),
                    guid: archisim_core::GlobalId::new(),
                    level: self.ed.level,
                    kind: s.device,
                    name: String::new(),
                    position: c.point,
                    rotation: s.device_rotation,
                    elevation: 0.0,
                    width: w,
                    depth: d,
                    height: 0.0,
                };
                painter.add(Shape::closed_line(
                    self.polygon(rect, &ghost.outline()),
                    Stroke::new(1.5, ACCENT),
                ));
            }
            _ => {}
        }
    }

    pub(super) fn services_hint(&self) -> &'static str {
        match self.tool {
            Tool::Pipe | Tool::Duct if !self.run_points.is_empty() => {
                "Clic: siguiente punto · Doble clic, Intro o clic derecho: terminar · Esc: quitar el último"
            }
            Tool::Pipe => "Clic: comienzo de la tubería (sistema, diámetro y altura, en Propiedades) · Esc: flecha",
            Tool::Duct => "Clic: comienzo del conducto (sistema, sección y altura, en Propiedades) · Esc: flecha",
            Tool::Device => "Clic: donde va el aparato · Espacio: girar 90° · Esc: flecha",
            _ => "",
        }
    }

    // ---- Properties ---------------------------------------------------------

    pub(super) fn run_tool_properties(&mut self, ui: &mut Ui) {
        let duct = self.tool == Tool::Duct;
        let mut s = self.service_settings;
        if duct {
            props::title(ui, "Herramienta Conducto · lo que vas a dibujar");
            group(ui, "tool-duct", "Conducto", |ui| {
                row(ui, "Sistema", "", |ui, w| {
                    s.duct_system =
                        system_combo(ui, Id::unique("tool-duct-sys"), true, s.duct_system, w);
                });
                row(ui, "Ancho (mm)", "", |ui, w| {
                    if let Some(v) =
                        length_field(ui, Id::unique("tool-duct-w"), s.duct_width, w - 8.0)
                    {
                        s.duct_width = v.max(0.01);
                    }
                });
                row(ui, "Alto (mm)", "", |ui, w| {
                    if let Some(v) =
                        length_field(ui, Id::unique("tool-duct-h"), s.duct_height, w - 8.0)
                    {
                        s.duct_height = v.max(0.01);
                    }
                });
                row(ui, "Altura del eje (mm)", "Sobre la planta", |ui, w| {
                    if let Some(v) =
                        length_field(ui, Id::unique("tool-duct-z"), s.duct_elevation, w - 8.0)
                    {
                        s.duct_elevation = v;
                    }
                });
            });
        } else {
            props::title(ui, "Herramienta Tubería · lo que vas a dibujar");
            group(ui, "tool-pipe", "Tubería", |ui| {
                row(ui, "Sistema", "", |ui, w| {
                    s.pipe_system =
                        system_combo(ui, Id::unique("tool-pipe-sys"), false, s.pipe_system, w);
                });
                row(ui, "Diámetro (mm)", "Exterior", |ui, w| {
                    if let Some(v) =
                        length_field(ui, Id::unique("tool-pipe-d"), s.pipe_size, w - 8.0)
                    {
                        s.pipe_size = v.max(0.005);
                    }
                });
                row(
                    ui,
                    "Altura del eje (mm)",
                    "Sobre la planta, en el primer punto",
                    |ui, w| {
                        if let Some(v) =
                            length_field(ui, Id::unique("tool-pipe-z"), s.pipe_elevation, w - 8.0)
                        {
                            s.pipe_elevation = v;
                        }
                    },
                );
                if s.pipe_system == ServiceSystem::Drainage {
                    row(
                        ui,
                        "Pendiente (%)",
                        "Baja desde el primer punto hacia el último",
                        |ui, w| {
                            if let Some(v) = percent_field(
                                ui,
                                Id::unique("tool-pipe-slope"),
                                s.drain_slope,
                                w - 8.0,
                            ) {
                                s.drain_slope = v;
                            }
                        },
                    );
                }
            });
        }
        self.service_settings = s;
    }

    pub(super) fn device_tool_properties(&mut self, ui: &mut Ui) {
        props::title(ui, "Herramienta Aparato · lo que vas a colocar");
        let mut s = self.service_settings;
        group(ui, "tool-device", "Aparato", |ui| {
            row(ui, "Tipo", "", |ui, w| {
                let k = kind_combo(ui, Id::unique("tool-dev-kind"), s.device, w);
                s.set_device(k);
            });
            row(ui, "Altura (mm)", "De su base sobre la planta", |ui, w| {
                if let Some(v) =
                    length_field(ui, Id::unique("tool-dev-z"), s.device_elevation, w - 8.0)
                {
                    s.device_elevation = v;
                }
            });
            row(ui, "Giro (°)", "", |ui, w| {
                if let Some(v) =
                    angle_field(ui, Id::unique("tool-dev-rot"), s.device_rotation, w - 8.0)
                {
                    s.device_rotation = v;
                }
            });
        });
        self.service_settings = s;
    }

    pub(super) fn run_properties(&mut self, ui: &mut Ui, id: RunId) {
        let Some(r) = self.ed.model().run(id).cloned() else {
            return;
        };
        let duct = r.is_duct();
        props::title(
            ui,
            &format!(
                "{} · 1 seleccionad{}",
                if duct { "Conducto" } else { "Tubería" },
                if duct { "o" } else { "a" }
            ),
        );
        group(ui, "run-data", "Instalación", |ui| {
            row(ui, "Sistema", "", |ui, w| {
                let n = system_combo(ui, Id::unique(("run-sys", id)), duct, r.system, w);
                if n != r.system {
                    let res = self
                        .ed
                        .transact("Sistema", |m| m.edit_run(id, |x| x.system = n));
                    self.report(res, "Sistema cambiado.");
                }
            });
            self.level_row(
                ui,
                ("run-level", id),
                "Planta",
                Some(r.level),
                &[Element::Run(id)],
            );
            let fields: Vec<(&str, f64)> = if duct {
                vec![
                    ("Ancho (mm)", r.width),
                    ("Alto (mm)", r.height),
                    ("Altura del eje (mm)", r.elevation),
                ]
            } else {
                vec![
                    ("Diámetro (mm)", r.width),
                    ("Altura del eje (mm)", r.elevation),
                ]
            };
            for (i, (name, v)) in fields.into_iter().enumerate() {
                row(ui, name, "", |ui, w| {
                    if let Some(v) = length_field(ui, Id::unique(("run-f", id, i)), v, w - 8.0) {
                        let res = self.ed.transact(name, |m| {
                            m.edit_run(id, |x| match (duct, i) {
                                (_, 0) => {
                                    x.width = v.max(0.005);
                                    if !duct {
                                        x.height = x.width;
                                    }
                                }
                                (true, 1) => x.height = v.max(0.005),
                                _ => x.elevation = v,
                            })
                        });
                        self.report(res, "Cambiado.");
                    }
                });
            }
            if !duct {
                row(
                    ui,
                    "Pendiente (%)",
                    "Baja desde el primer punto hacia el último",
                    |ui, w| {
                        if let Some(v) =
                            percent_field(ui, Id::unique(("run-slope", id)), r.slope, w - 8.0)
                        {
                            let res = self
                                .ed
                                .transact("Pendiente", |m| m.edit_run(id, |x| x.slope = v));
                            self.report(res, "Pendiente cambiada.");
                        }
                    },
                );
            }
        });
        let fittings: Vec<_> = self
            .ed
            .model()
            .run_fittings()
            .into_iter()
            .filter(|f| f.run == id)
            .collect();
        group(ui, "run-figures", "Mediciones", |ui| {
            text_row(ui, "Longitud (m)", "", &comma(r.true_length(), 2));
            text_row(ui, "Tramos", "", &(r.path.len() - 1).to_string());
            if r.slope > 0.0 {
                text_row(
                    ui,
                    "Altura al final (mm)",
                    "Del eje, sobre la planta",
                    &crate::units::fmt_len(r.end_elevation()),
                );
            }
            for kind in [
                archisim_core::FittingKind::Elbow,
                archisim_core::FittingKind::Tee,
            ] {
                let n = fittings.iter().filter(|f| f.kind == kind).count();
                text_row(
                    ui,
                    if kind == archisim_core::FittingKind::Elbow {
                        "Codos"
                    } else {
                        "Tes"
                    },
                    "Salen del trazado: no se dibujan",
                    &n.to_string(),
                );
            }
        });
        group(ui, "run-identity", "Datos de identidad", |ui| {
            text_row(
                ui,
                "Clase IFC",
                "Un segmento por tramo, en su IfcDistributionSystem",
                if duct {
                    "IfcDuctSegment"
                } else {
                    "IfcPipeSegment"
                },
            );
        });
    }

    pub(super) fn device_properties(&mut self, ui: &mut Ui, id: DeviceId) {
        let Some(d) = self.ed.model().device(id).cloned() else {
            return;
        };
        props::title(ui, &format!("{} · 1 seleccionado", d.name));
        group(ui, "dev-data", "Aparato", |ui| {
            row(ui, "Nombre", "", |ui, w| {
                if let Some(t) =
                    text_edit_committed(ui, Id::unique(("dev-name", id)), &d.name, w - 8.0)
                {
                    let res = self.ed.transact("Nombre del aparato", |m| {
                        m.edit_device(id, |x| x.name = t.trim().to_string())
                    });
                    self.report(res, "Nombre cambiado.");
                }
            });
            row(ui, "Tipo", "", |ui, w| {
                let k = kind_combo(ui, Id::unique(("dev-kind", id)), d.kind, w);
                if k != d.kind {
                    let res = self.ed.transact("Tipo de aparato", |m| {
                        m.edit_device(id, |x| {
                            if x.name == x.kind.name() {
                                x.name = k.name().to_string();
                            }
                            x.kind = k;
                        })
                    });
                    self.report(res, "Tipo cambiado.");
                }
            });
            self.level_row(
                ui,
                ("dev-level", id),
                "Planta",
                Some(d.level),
                &[Element::Device(id)],
            );
            let fields = [
                ("Ancho (mm)", d.width),
                ("Fondo (mm)", d.depth),
                ("Alto (mm)", d.height),
                ("Altura (mm)", d.elevation),
            ];
            for (i, (name, v)) in fields.into_iter().enumerate() {
                row(ui, name, "", |ui, w| {
                    if let Some(v) = length_field(ui, Id::unique(("dev-f", id, i)), v, w - 8.0) {
                        let res = self.ed.transact(name, |m| {
                            m.edit_device(id, |x| match i {
                                0 => x.width = v.max(0.01),
                                1 => x.depth = v.max(0.01),
                                2 => x.height = v.max(0.01),
                                _ => x.elevation = v,
                            })
                        });
                        self.report(res, "Cambiado.");
                    }
                });
            }
            row(ui, "Giro (°)", "", |ui, w| {
                if let Some(v) = angle_field(ui, Id::unique(("dev-rot", id)), d.rotation, w - 8.0) {
                    let res = self
                        .ed
                        .transact("Giro", |m| m.edit_device(id, |x| x.rotation = v));
                    self.report(res, "Girado.");
                }
            });
        });
        group(ui, "dev-identity", "Datos de identidad", |ui| {
            text_row(
                ui,
                "Clase IFC",
                "",
                match d.kind {
                    DeviceKind::Equipment => "IfcUnitaryEquipment",
                    DeviceKind::Light => "IfcLightFixture",
                    DeviceKind::Socket => "IfcOutlet",
                    DeviceKind::Switch => "IfcSwitchingDevice",
                    DeviceKind::Radiator => "IfcSpaceHeater",
                    DeviceKind::Grille | DeviceKind::Diffuser => "IfcAirTerminal",
                    DeviceKind::TwoWaySwitch => "IfcSwitchingDevice",
                    DeviceKind::PowerSocket | DeviceKind::TvSocket | DeviceKind::DataSocket => {
                        "IfcOutlet"
                    }
                    DeviceKind::Panel => "IfcElectricDistributionBoard",
                    DeviceKind::Riser | DeviceKind::Downpipe => "IfcPipeSegment",
                },
            );
        });
        // The circuit of a mechanism (E13).
        if d.kind.is_electric() && d.kind != DeviceKind::Panel {
            group(ui, "dev-circuit", "Circuito (REBT, ITC-BT-25)", |ui| {
                let given = self
                    .ed
                    .model()
                    .device_circuits()
                    .find(|(x, _)| *x == id)
                    .map(|(_, n)| n);
                let now = self.ed.model().device_circuit(id);
                let names: Vec<String> = archisim_core::CIRCUITS
                    .iter()
                    .map(|c| format!("C{} · {}", c.number, c.name))
                    .collect();
                let mut options: Vec<(Option<u8>, &str)> = vec![(None, "El de su clase")];
                options.extend(
                    archisim_core::CIRCUITS
                        .iter()
                        .zip(&names)
                        .map(|(c, n)| (Some(c.number), n.as_str())),
                );
                options.push((Some(0), "Ninguno"));
                row(
                    ui,
                    "Circuito",
                    "Por defecto, el que le toca por su clase y el local donde está",
                    |ui, w| {
                        let v = super::props::choice(ui, ("dev-circuit", id), w, given, &options);
                        if v != given {
                            let res = self
                                .ed
                                .transact("Circuito", |m| m.set_device_circuit(id, v));
                            self.report(res, "Circuito cambiado.");
                        }
                    },
                );
                if let Some(c) = now.and_then(archisim_core::circuit_spec) {
                    text_row(
                        ui,
                        "Protección",
                        "",
                        &format!(
                            "{} A · {} mm²",
                            c.breaker,
                            format!("{}", c.section).replace('.', ",")
                        ),
                    );
                }
            });
        }
    }
}
