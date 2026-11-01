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
            if !ducts {
                // The networks under the street (U5).
                ui.separator();
                for s in ServiceSystem::URBAN {
                    ui.selectable_value(&mut v, s, s.name());
                }
            }
        });
    v
}

/// «IFCFIRESUPPRESSIONTERMINAL» → «IfcFireSuppressionTerminal».
fn ifc_name(class: &str) -> String {
    const WORDS: [&str; 28] = [
        "UNITARY",
        "EQUIPMENT",
        "LIGHT",
        "FIXTURE",
        "OUTLET",
        "SWITCHING",
        "DEVICE",
        "SPACE",
        "HEATER",
        "AIR",
        "TERMINAL",
        "ELECTRIC",
        "DISTRIBUTION",
        "BOARD",
        "PIPE",
        "SEGMENT",
        "FIRE",
        "SUPPRESSION",
        "SENSOR",
        "ALARM",
        "VALVE",
        "FLOW",
        "METER",
        "TANK",
        "PUMP",
        "BOILER",
        "SOLAR",
        "IFC",
    ];
    let mut rest = class;
    let mut out = String::new();
    'next: while !rest.is_empty() {
        for w in WORDS {
            if let Some(r) = rest.strip_prefix(w) {
                let mut c = w.chars();
                if let Some(f) = c.next() {
                    out.push(f);
                    out.extend(c.flat_map(|x| x.to_lowercase()));
                }
                rest = r;
                continue 'next;
            }
        }
        out.push_str(&rest.to_lowercase());
        break;
    }
    out
}

fn kind_combo(ui: &mut Ui, id: Id, value: DeviceKind, w: f32) -> DeviceKind {
    let mut v = value;
    ComboBox::from_id_salt(id)
        .width(w - 8.0)
        .selected_text(value.name())
        .show_ui(ui, |ui| {
            // The kinds of its own family: fire protection, water,
            // solar, or the rest.
            let family = |k: DeviceKind| (k.is_fire(), k.is_water(), k.is_solar(), k.is_urban());
            for k in DeviceKind::ALL
                .into_iter()
                .filter(|k| family(*k) == family(value))
            {
                ui.selectable_value(&mut v, k, k.name());
            }
        });
    v
}

impl ArchiSimApp {
    /// A run of a network under the street (U5): its manholes, its
    /// connections to the plots beside it and its longitudinal profile.
    fn network_group(&mut self, ui: &mut Ui, id: RunId) {
        let Some(r) = self.ed.model().run(id).cloned() else {
            return;
        };
        group(ui, "run-network", "Red urbana", |ui| {
            props::note(
                ui,
                "Bajo la calle: la profundidad es la altura del eje, negativa, sobre la planta.",
            );
            ui.horizontal_wrapped(|ui| {
                if r.system.falls()
                    && ui
                        .button("Pozos de registro")
                        .on_hover_text("En los extremos, en cada quiebro y a no más de 40 m")
                        .clicked()
                {
                    let mut n = 0;
                    let res = self.ed.transact("Pozos de registro", |m| {
                        n = m.add_manholes(id)?.len();
                        Ok(())
                    });
                    self.report(res, &format!("{n} pozos de registro."));
                }
                if ui
                    .button("Acometidas a las parcelas")
                    .on_hover_text("Una arqueta en el frente de cada parcela a menos de 25 m y su tubería hasta la red, a escuadra")
                    .clicked()
                {
                    let mut n = 0;
                    let res = self.ed.transact("Acometidas", |m| {
                        n = m.connect_plots(id)?.len();
                        Ok(())
                    });
                    self.report(res, &format!("{n} acometidas."));
                }
                if ui
                    .button("Unir a los edificios")
                    .on_hover_text("Lleva cada acometida hasta la instalación del edificio de su parcela: bajante o saneamiento, contador o agua fría, caja general de protección o cuadro")
                    .clicked()
                {
                    let mut n = 0;
                    let res = self.ed.transact("Unir acometidas", |m| {
                        n = m.connect_buildings(id)?.len();
                        Ok(())
                    });
                    let text = if n == 0 {
                        "Ninguna acometida por unir: hacen falta acometidas y, en la parcela, la instalación del edificio (bajante, contador, cuadro…).".to_string()
                    } else {
                        format!("{n} acometidas unidas a su edificio.")
                    };
                    self.report(res, &text);
                }
                if ui
                    .button("Perfil longitudinal…")
                    .on_hover_text("Guarda el perfil (H 1:500, V 1:50) con su guitarra de cotas, como SVG")
                    .clicked()
                {
                    match archisim_plan::profile::run_profile_svg(self.ed.model(), id) {
                        Some(svg) => {
                            let file = rfd::FileDialog::new()
                                .set_title("Guardar el perfil longitudinal")
                                .set_file_name(format!("perfil-{}.svg", r.system.code()))
                                .add_filter("SVG", &["svg"])
                                .save_file();
                            if let Some(path) = file {
                                match std::fs::write(&path, svg) {
                                    Ok(()) => self.say(format!("Perfil guardado en {}.", path.display())),
                                    Err(e) => self.complain(format!("No se puede guardar el perfil: {e}.")),
                                }
                            }
                        }
                        None => self.complain("La tubería es demasiado corta para su perfil."),
                    }
                }
            });
        });
    }

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
            slope: if !duct && s.pipe_system.falls() {
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
        // The urban ones stand on the street or go down from it (U5).
        let elevation = if s.device.is_urban() {
            self.ed.model().surface_at(self.ed.level, p) + s.device_elevation
        } else {
            s.device_elevation
        };
        let spec = NewDevice {
            level: self.ed.level,
            kind: s.device,
            name: s.device.name().to_string(),
            position: p,
            rotation: s.device_rotation,
            elevation,
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

    /// Takes the pipe tool for a network under the street (U5).
    pub(super) fn network_pick(&mut self) {
        if !self.service_settings.pipe_system.is_urban() {
            let sys = ServiceSystem::Sewer;
            let (d, depth) = sys.urban_defaults();
            self.service_settings.pipe_system = sys;
            self.service_settings.pipe_size = d;
            self.service_settings.pipe_elevation = -depth;
            self.service_settings.drain_slope = 0.01;
        }
        self.set_tool(Tool::Pipe);
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
                    fixture: None,
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
                    let before = s.pipe_system;
                    s.pipe_system =
                        system_combo(ui, Id::unique("tool-pipe-sys"), false, s.pipe_system, w);
                    // A network under the street takes its usual pipe and
                    // depth (U5).
                    if s.pipe_system != before && s.pipe_system.is_urban() {
                        let (d, depth) = s.pipe_system.urban_defaults();
                        s.pipe_size = d;
                        s.pipe_elevation = -depth;
                    }
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
                if s.pipe_system.falls() {
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
        if r.system.is_urban() {
            self.network_group(ui, id);
        }
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
        // A row of solar panels: its modules, tilt and power (H3).
        if d.kind.is_solar() {
            group(ui, "dev-solar", "Captación solar", |ui| {
                let n = archisim_core::solar_modules(&d);
                text_row(ui, "Módulos", "", &n.to_string());
                text_row(
                    ui,
                    "Inclinación (°)",
                    "El alto de la caja sobre su fondo; plana si el alto es de 5 cm o menos",
                    &comma(archisim_core::solar_tilt(&d).to_degrees(), 0),
                );
                if d.kind == DeviceKind::SolarPanel {
                    text_row(
                        ui,
                        "Potencia pico (kWp)",
                        "",
                        &comma(n as f64 * archisim_core::MODULE_PEAK_KW, 2),
                    );
                } else {
                    text_row(
                        ui,
                        "Superficie de captación (m²)",
                        "",
                        &comma(n as f64 * archisim_core::COLLECTOR_AREA, 1),
                    );
                }
            });
        }
        // A light point's lamp (G4).
        if d.kind == DeviceKind::Light {
            self.fixture_group(ui, id, d.fixture);
        }
        group(ui, "dev-identity", "Datos de identidad", |ui| {
            text_row(ui, "Clase IFC", "", &ifc_name(d.kind.ifc_class().0));
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

/// The lamps of the collections a light point can take: the pieces of the
/// catalogue whose family is a pendant, a ceiling light or a wall light.
fn collection_lamps(lib: &archisim_core::Model) -> Vec<&'static str> {
    archisim_library::OBJECT_CATALOGUE
        .iter()
        .map(|(name, _, _)| *name)
        .filter(|name| {
            lib.object_types().any(|t| {
                t.name == *name
                    && lib.family_prop(t.family).is_some_and(|k| {
                        matches!(k.as_str(), "pendant" | "ceiling_light" | "wall_light")
                    })
            })
        })
        .collect()
}

impl ArchiSimApp {
    /// The lamp of a light point, chosen on cards drawn in 3D: the
    /// project's lamps or those of a collection, or the downlight.
    fn fixture_group(
        &mut self,
        ui: &mut Ui,
        id: DeviceId,
        now: Option<archisim_core::ObjectTypeId>,
    ) {
        use archisim_library::collections::{self, COLLECTIONS};
        let ctx = ui.ctx().clone();
        let p = super::theme::Palette::current(ui.ctx());
        let mut pick: Option<Pick> = None;
        enum Pick {
            Downlight,
            Project(archisim_core::ObjectTypeId),
            Collected(&'static str, usize),
        }
        let source = self.fixture_source.filter(|i| *i < COLLECTIONS.len());
        // The cards: (name, its picture, what a click does, on).
        let mut cards: Vec<(String, Option<super::thumbs::Thumb>, Pick, bool)> =
            vec![("Empotrable".into(), None, Pick::Downlight, now.is_none())];
        if let Some(i) = source {
            self.collection_library(&COLLECTIONS[i]);
        }
        self.thumbs3d.begin();
        let gpu = self.gpu.as_ref();
        match source {
            None => {
                let model = self.ed.model();
                for t in model.fixture_types() {
                    let Some(ty) = model.object_type(t) else {
                        continue;
                    };
                    let key = format!("p:{}:{:?}", ty.family.0, ty.values);
                    let thumb = self.thumbs3d.get(&ctx, gpu, key, || {
                        crate::type_preview::object_model(model, ty)
                    });
                    cards.push((ty.name.clone(), thumb, Pick::Project(t), now == Some(t)));
                }
            }
            Some(i) => {
                let c = &COLLECTIONS[i];
                let lib = &self
                    .collection_libraries
                    .iter()
                    .find(|(n, _)| *n == c.name)
                    .expect("just made")
                    .1;
                let current = now
                    .and_then(|t| self.ed.model().object_type(t))
                    .map(|t| t.name.clone());
                for name in collection_lamps(lib) {
                    let variant = collections::variant_name(name, c);
                    let Some(ty) = lib.object_types().find(|t| t.name == variant) else {
                        continue;
                    };
                    let thumb =
                        self.thumbs3d
                            .get(&ctx, gpu, format!("c:{}:{name}", c.name), || {
                                crate::type_preview::object_model(lib, ty)
                            });
                    let on = current.as_deref() == Some(variant.as_str());
                    cards.push((name.to_string(), thumb, Pick::Collected(name, i), on));
                }
            }
        }
        if self.thumbs3d.pending {
            ctx.request_repaint();
        }
        group(ui, "dev-fixture", "Lámpara", |ui| {
            ui.label(
                egui::RichText::new(
                    "La pieza que se ve en 3D en este punto de luz: un empotrable o una lámpara.",
                )
                .small()
                .color(p.muted),
            );
            ui.horizontal_wrapped(|ui| {
                let chip = |ui: &mut Ui, text: &str, on: bool| {
                    ui.add(egui::Button::new(egui::RichText::new(text).small()).selected(on))
                        .clicked()
                };
                if chip(ui, "Del proyecto", source.is_none()) {
                    self.fixture_source = None;
                }
                for (i, c) in COLLECTIONS.iter().enumerate() {
                    if chip(ui, c.name, source == Some(i)) {
                        self.fixture_source = Some(i);
                    }
                }
            });
            ui.add_space(4.0);
            let card = egui::vec2(86.0, 104.0);
            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing = egui::vec2(6.0, 6.0);
                for (name, thumb, act, on) in cards.drain(..) {
                    let (rect, resp) = ui.allocate_exact_size(card, egui::Sense::click());
                    let painter = ui.painter_at(rect);
                    let fill = if on {
                        p.accent_soft
                    } else if resp.hovered() {
                        p.raised
                    } else {
                        p.ground
                    };
                    painter.rect_filled(rect, 6.0, fill);
                    if on {
                        painter.rect_stroke(
                            rect,
                            6.0,
                            egui::Stroke::new(1.5, p.accent),
                            egui::StrokeKind::Inside,
                        );
                    }
                    let pic = egui::Rect::from_min_size(
                        rect.min + egui::vec2(4.0, 4.0),
                        egui::vec2(card.x - 8.0, card.x - 8.0),
                    );
                    match thumb {
                        Some(t) => {
                            let size = t.fit(pic.size());
                            painter.image(
                                t.tex,
                                egui::Rect::from_center_size(pic.center(), size),
                                egui::Rect::from_min_max(
                                    egui::pos2(0.0, 0.0),
                                    egui::pos2(1.0, 1.0),
                                ),
                                egui::Color32::WHITE,
                            );
                        }
                        None if matches!(act, Pick::Downlight) => {
                            // The downlight seen from below: its ring and lens.
                            let c = pic.center();
                            painter.circle_filled(c, 22.0, egui::Color32::from_rgb(235, 235, 230));
                            painter.circle_stroke(c, 22.0, egui::Stroke::new(1.0, p.muted));
                            painter.circle_filled(c, 15.0, egui::Color32::from_rgb(255, 238, 190));
                        }
                        None => {}
                    }
                    // A long name, cut to the card.
                    let shown = if name.chars().count() > 15 {
                        format!("{}…", name.chars().take(14).collect::<String>())
                    } else {
                        name.clone()
                    };
                    painter.text(
                        egui::pos2(rect.center().x, rect.max.y - 14.0),
                        egui::Align2::CENTER_CENTER,
                        shown,
                        egui::FontId::proportional(10.5),
                        p.text,
                    );
                    if resp.on_hover_text(&name).clicked() && !on {
                        pick = Some(act);
                    }
                }
            });
        });
        let Some(pick) = pick else {
            return;
        };
        let mark = self.ed.undo_mark();
        let fixture = match pick {
            Pick::Downlight => None,
            Pick::Project(t) => Some(t),
            Pick::Collected(name, i) => match self.ed.collection_object_type(name, &COLLECTIONS[i])
            {
                Ok(t) => Some(t),
                Err(e) => {
                    self.complain(format!("No se puede cargar «{name}»: {e}."));
                    return;
                }
            },
        };
        let res = self
            .ed
            .transact("Lámpara", |m| m.set_device_fixture(id, fixture));
        self.ed.join_since(mark, "Lámpara del punto de luz");
        self.report(res, "Lámpara cambiada.");
    }
}
