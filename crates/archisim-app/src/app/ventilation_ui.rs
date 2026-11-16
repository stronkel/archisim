//! The ventilation window (Colaboración → Cálculo de ventilación): the
//! system (mechanical or hybrid) and the zone of table 4.4, the
//! dwellings' ventilation by the DB-HS 3 as text, and «Poner los tamaños
//! calculados», which takes every inlet, extract valve, duct and shaft
//! drawn smaller up to what it needs, in one undoable step. Also the
//! garages' system (natural or mechanical) and, for the rooms of the RITE,
//! the outdoor air's class, the ducts' speed and each room's IDA.

use eframe::egui::{self, Ui};

use super::*;

/// The window's state.
#[derive(Default)]
pub struct VentilationWindow {
    pub open: bool,
    /// The calculation of a revision of the model: (revision, text, the
    /// changes, what fails).
    results: Option<(u64, String, Grow, usize)>,
    /// The calculation drawn (`report_view`), and whether shown as typed
    /// text.
    blocks: Vec<archisim_struct::report::Block>,
    as_text: bool,
    /// The rooms of the RITE of the last calculation: room, name, IDA and
    /// whether set by hand.
    rite: Vec<(archisim_core::RoomId, String, u8, bool)>,
}

/// What grows: ducts (their width) and devices (width and depth).
#[derive(Clone, Default)]
struct Grow {
    runs: Vec<(archisim_core::RunId, f64)>,
    devices: Vec<(archisim_core::DeviceId, f64, f64)>,
}

impl Grow {
    fn len(&self) -> usize {
        self.runs.len() + self.devices.len()
    }

    fn of(model: &archisim_core::Model, v: &archisim_mep::ventilation::Ventilation) -> Self {
        Grow {
            runs: v
                .sized_runs(model)
                .into_iter()
                .filter(|(r, w)| model.run(*r).is_some_and(|x| x.width + 1e-4 < *w))
                .collect(),
            devices: v.sized_devices(model),
        }
    }
}

impl ArchiSimApp {
    pub(super) fn ventilation_dialog(&mut self, ctx: &egui::Context) {
        if !self.ventilation.open {
            return;
        }
        let mut open = true;
        super::widgets::dialog("Cálculo de ventilación (DB-HS 3 y RITE)")
            .open(&mut open)
            .default_width(860.0)
            .default_height(560.0)
            .show(ctx, |ui| self.ventilation_contents(ui));
        self.ventilation.open = open;
    }

    fn ventilation_contents(&mut self, ui: &mut Ui) {
        ui.label(
            egui::RichText::new(
                "Viviendas: los locales de cada planta unidos por puertas, sin los comunes. Entra el aire por los aireadores de los locales secos, pasa bajo las puertas y sale por las bocas de extracción de los húmedos, por los conductos del sistema Extracción hasta el extractor.",
            )
            .small(),
        );
        self.ventilation_site_row(ui);
        self.rite_rows(ui);
        ui.separator();
        let rev = self.ed.revision();
        if self.ventilation.results.as_ref().is_none_or(|r| r.0 != rev) {
            let model = self.ed.model();
            let v = archisim_mep::ventilation::ventilation(model);
            let text = archisim_mep::ventilation::report(model, &v);
            let fails = v.fails();
            let mut blocks = archisim_plan::services_report::ventilation(model, &v);
            if v.rite.applies() {
                blocks.push(archisim_struct::report::Block::Heading(
                    1,
                    "Locales del RITE (IT 1.1.4.2)".into(),
                ));
                blocks.extend(archisim_plan::services_report::rite(model, &v));
            }
            self.ventilation.blocks = blocks;
            self.ventilation.rite = v
                .rite
                .rooms
                .iter()
                .map(|r| (r.room, r.name.clone(), r.ida, r.by_hand))
                .collect();
            self.ventilation.results = Some((rev, text, Grow::of(model, &v), fails));
        }
        let (_, text, grow, fails) = self.ventilation.results.clone().expect("just made");
        let (mut apply, mut pdf, mut dxf) = (false, false, false);
        ui.horizontal(|ui| {
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui
                    .button("Esquemas (DXF)…")
                    .on_hover_text("El esquema vertical y la red de cada planta, un DXF por hoja")
                    .clicked()
                {
                    dxf = true;
                }
                if ui
                    .button("Esquemas (PDF)…")
                    .on_hover_text("El esquema vertical de la extracción y las campanas, y la red de cada planta, en hojas A3")
                    .clicked()
                {
                    pdf = true;
                }
                if ui
                    .add_enabled(
                        grow.len() > 0,
                        egui::Button::new(format!("Poner los tamaños calculados ({})", grow.len())),
                    )
                    .on_hover_text("Los aireadores, bocas, conductos y conductos verticales dibujados más pequeños toman el tamaño calculado, en un paso de Deshacer (los pasos de las puertas se quedan: holgura o rejilla, a decidir)")
                    .clicked()
                {
                    apply = true;
                }
            });
        });
        let blocks = std::mem::take(&mut self.ventilation.blocks);
        super::report_view::result(
            ui,
            &blocks,
            &text,
            &mut self.ventilation.as_text,
            "la ventilación",
            "ventilation",
            Some(fails),
        );
        self.ventilation.blocks = blocks;
        if apply {
            self.apply_ventilation_sizes();
        }
        if pdf {
            self.export_ventilation_pdf();
        }
        if dxf {
            self.export_ventilation_dxf();
        }
    }

    /// The system (mechanical or hybrid) and the zone of table 4.4, set
    /// by hand or from the province.
    fn ventilation_site_row(&mut self, ui: &mut Ui) {
        use archisim_core::GarageVent;
        use archisim_core::{DraftZone, VentSystem, DRAFT_ZONES};
        let site = self.ed.model().ventilation_site().clone();
        let mut new = site.clone();
        ui.horizontal(|ui| {
            ui.label("Sistema");
            egui::ComboBox::from_id_salt("vent-system")
                .selected_text(new.system.name())
                .show_ui(ui, |ui| {
                    for s in VentSystem::ALL {
                        ui.selectable_value(&mut new.system, s, s.name());
                    }
                })
                .response
                .on_hover_text("HS 3, 3.1.1: híbrida (el conducto tira solo y un aspirador híbrido le ayuda) o mecánica (extractor). Los garajes y las campanas son siempre mecánicos");
            ui.add_space(12.0);
            ui.add_enabled_ui(new.system == VentSystem::Hybrid, |ui| {
                ui.label("Zona térmica");
                egui::ComboBox::from_id_salt("vent-zone")
                    .selected_text(new.zone.code())
                    .width(40.0)
                    .show_ui(ui, |ui| {
                        for z in DraftZone::ALL {
                            ui.selectable_value(&mut new.zone, z, z.code());
                        }
                    })
                    .response
                    .on_hover_text("HS 3, tabla 4.4: da con las plantas el tiro de los conductos (tabla 4.3)");
                egui::ComboBox::from_id_salt("vent-province")
                    .selected_text("Por provincia…")
                    .height(360.0)
                    .show_ui(ui, |ui| {
                        for (p, low, high) in DRAFT_ZONES {
                            if ui
                                .selectable_label(false, format!("{p} (hasta 800 m): {}", low.code()))
                                .clicked()
                            {
                                new.zone = low;
                            }
                            if let Some(h) = high {
                                if ui
                                    .selectable_label(false, format!("{p} (más de 800 m): {}", h.code()))
                                    .clicked()
                                {
                                    new.zone = h;
                                }
                            }
                        }
                    });
            });
        });
        ui.horizontal(|ui| {
            ui.label("Garajes");
            egui::ComboBox::from_id_salt("vent-garage")
                .selected_text(new.garage.name())
                .show_ui(ui, |ui| {
                    for g in GarageVent::ALL {
                        ui.selectable_value(&mut new.garage, g, g.name());
                    }
                })
                .response
                .on_hover_text("HS 3, 3.1.4: natural, por aberturas mixtas en dos zonas opuestas de la fachada (se leen las ventanas, huecos y rejillas de sus muros), o mecánica, 120 l/s por plaza con bocas de extracción. «Según el dibujo»: mecánica si tiene bocas dibujadas");
            ui.add_space(12.0);
            ui.label("Aire exterior (RITE)");
            egui::ComboBox::from_id_salt("vent-oda")
                .selected_text(format!("ODA {}", new.oda))
                .width(60.0)
                .show_ui(ui, |ui| {
                    for (k, what) in [
                        (1u8, "ODA 1: aire puro, se ensucia solo de vez en cuando (polen)"),
                        (2, "ODA 2: muchas partículas o gases (ciudad)"),
                        (3, "ODA 3: muchísimos gases o partículas"),
                    ] {
                        ui.selectable_value(&mut new.oda, k, what);
                    }
                })
                .response
                .on_hover_text("RITE IT 1.1.4.2.4: con la IDA de cada local da los filtros (tabla 1.4.2.5)");
            ui.add_space(12.0);
            ui.label("Velocidad en conductos");
            ui.add(
                egui::DragValue::new(&mut new.duct_speed)
                    .range(1.0..=12.0)
                    .speed(0.1)
                    .suffix(" m/s"),
            )
            .on_hover_text("Los conductos de los locales del RITE se dimensionan por la velocidad del aire: esta en los principales y el 60 % en los ramales a un solo terminal");
        });
        if new != site {
            let r = self
                .ed
                .transact("Sistema de ventilación", |m| m.set_ventilation_site(new));
            if let Err(e) = r {
                self.complain(format!("No se puede cambiar: {e}."));
            }
        }
    }

    /// The rooms of the RITE with their indoor air class, which can be set
    /// by hand.
    fn rite_rows(&mut self, ui: &mut Ui) {
        let rooms = self.ventilation.rite.clone();
        if rooms.is_empty() {
            return;
        }
        let mut change: Option<(archisim_core::RoomId, Option<u8>)> = None;
        egui::CollapsingHeader::new(format!(
            "Calidad del aire interior de los locales del RITE ({})",
            rooms.len()
        ))
        .id_salt("rite-ida")
        .show(ui, |ui| {
            egui::Grid::new("rite-ida-grid")
                .striped(true)
                .show(ui, |ui| {
                    for (id, name, ida, by_hand) in &rooms {
                        ui.label(name);
                        let shown = if *by_hand {
                            format!("IDA {ida} (a mano)")
                        } else {
                            format!("IDA {ida} (por su uso)")
                        };
                        egui::ComboBox::from_id_salt(("rite-ida", id.0))
                            .selected_text(shown)
                            .show_ui(ui, |ui| {
                                if ui.selectable_label(!by_hand, "Por su uso").clicked() {
                                    change = Some((*id, None));
                                }
                                for k in 1..=4u8 {
                                    let what = match k {
                                        1 => "IDA 1: óptima (hospitales, guarderías)",
                                        2 => "IDA 2: buena (oficinas, aulas)",
                                        3 => "IDA 3: media (comercios, bares)",
                                        _ => "IDA 4: baja",
                                    };
                                    if ui.selectable_label(*by_hand && *ida == k, what).clicked() {
                                        change = Some((*id, Some(k)));
                                    }
                                }
                            });
                        ui.end_row();
                    }
                });
        });
        if let Some((room, ida)) = change {
            let r = self
                .ed
                .transact("Calidad del aire interior", |m| m.set_room_ida(room, ida));
            if let Err(e) = r {
                self.complain(format!("No se puede cambiar: {e}."));
            }
        }
    }

    /// The ventilation diagrams as a PDF of A3 pages.
    pub(super) fn export_ventilation_pdf(&mut self) {
        let mut d = crate::file_dialog::FileDialog::new()
            .set_title("Esquemas de ventilación (PDF)")
            .add_filter("PDF", &["pdf"])
            .set_file_name(format!("{}-ventilacion.pdf", self.file_name_stem()));
        if let Some(dir) = self.start_dir() {
            d = d.set_directory(dir);
        }
        let Some(path) = d.save_file() else {
            return;
        };
        let model = self.ed.shown();
        let sheets: Vec<(String, String)> =
            archisim_plan::water_schematic::ventilation_pages(&model)
                .into_iter()
                .enumerate()
                .map(|(i, svg)| (svg, format!("VE-{:02}", i + 1)))
                .collect();
        if sheets.is_empty() {
            self.complain("No hay conductos de extracción que lleguen a una boca o a una campana.");
            return;
        }
        let r = crate::documents::pdf_of(&sheets, "Esquemas de ventilación")
            .and_then(|bytes| std::fs::write(&path, bytes).map_err(|e| e.to_string()));
        match r {
            Ok(()) => self.say(format!(
                "Esquemas de ventilación: {} hojas A3 en {}.",
                sheets.len(),
                path.display()
            )),
            Err(e) => self.complain(format!("No se pudo exportar: {e}.")),
        }
    }

    /// The ventilation diagrams in DXF: one file per page.
    pub(super) fn export_ventilation_dxf(&mut self) {
        let mut d = crate::file_dialog::FileDialog::new()
            .set_title("Esquemas de ventilación (DXF)")
            .add_filter("DXF", &["dxf"])
            .set_file_name(format!("{}-ventilacion.dxf", self.file_name_stem()));
        if let Some(dir) = self.start_dir() {
            d = d.set_directory(dir);
        }
        let Some(path) = d.save_file() else {
            return;
        };
        let model = self.ed.shown();
        let sheets = archisim_plan::water_schematic::ventilation_pages(&model);
        if sheets.is_empty() {
            self.complain("No hay conductos de extracción que lleguen a una boca o a una campana.");
            return;
        }
        let stem = path
            .file_stem()
            .map_or("ventilacion".into(), |s| s.to_string_lossy().to_string());
        for (i, svg) in sheets.iter().enumerate() {
            let file = path.with_file_name(format!("{stem}-ve-{:02}.dxf", i + 1));
            if let Err(e) = archisim_export::dxf::page_dxf(&model, svg)
                .and_then(|t| std::fs::write(&file, t).map_err(|e| e.to_string()))
            {
                self.complain(format!("No se pudo exportar: {e}."));
                return;
            }
        }
        self.say(format!(
            "Esquemas de ventilación en DXF: {} archivos junto a {}.",
            sheets.len(),
            path.display()
        ));
    }

    /// Gives every inlet, valve, duct and shaft drawn smaller than it
    /// needs the size of the calculation, in one undoable step; how many
    /// change.
    pub(super) fn apply_ventilation_sizes(&mut self) -> usize {
        let model = self.ed.model();
        let v = archisim_mep::ventilation::ventilation(model);
        let grow = Grow::of(model, &v);
        let n = grow.len();
        if n == 0 {
            return 0;
        }
        let r = self.ed.transact("Tamaños del cálculo de ventilación", |m| {
            for (run, w) in &grow.runs {
                m.edit_run(*run, |r| r.width = *w)?;
            }
            for (d, w, dp) in &grow.devices {
                m.edit_device(*d, |x| {
                    x.width = *w;
                    x.depth = *dp;
                })?;
            }
            Ok(())
        });
        self.report(
            r,
            &format!(
                "Tamaños puestos en {n} elemento{} de la ventilación. Deshacer los quita.",
                if n == 1 { "" } else { "s" }
            ),
        );
        n
    }
}
