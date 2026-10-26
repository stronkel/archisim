//! Modules (I5): the module tool (load a project as a module, place its
//! copies with a turn and, if wanted, mirrored), the properties of a
//! selected copy and the Modules section of the navigator, which says
//! when a module's file has changed and updates it.

use super::widgets::SwitchExt as _;
use std::time::{Duration, Instant};

use archisim_core::{ModuleId, ModuleSourceId, ModuleTransform, Point2};
use eframe::egui::{self, Id, Ui};

use super::icons::Icon;
use super::props::{self, group, row, text_row};
use super::*;
use crate::editor::ModuleStatus;

/// What the module tool places.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct ModuleTool {
    pub source: Option<ModuleSourceId>,
    /// Turn of the next copy, radians.
    pub rotation: f64,
    pub mirror: bool,
}

/// The status of the module files, checked now and then (it reads them
/// from disk).
#[derive(Default)]
pub struct ModuleWatch {
    pub checked: Option<Instant>,
    pub status: Vec<(ModuleSourceId, ModuleStatus)>,
}

impl ArchiSimApp {
    /// The status of a module's file (checked at most every few seconds).
    fn module_status_of(&mut self, id: ModuleSourceId) -> ModuleStatus {
        let stale = self
            .module_watch
            .checked
            .is_none_or(|t| t.elapsed() > Duration::from_secs(3));
        if stale {
            self.module_watch.status = self.ed.module_status();
            self.module_watch.checked = Some(Instant::now());
        }
        self.module_watch
            .status
            .iter()
            .find(|(s, _)| *s == id)
            .map_or(ModuleStatus::UpToDate, |(_, st)| *st)
    }

    /// Checks the module files again on the next look.
    fn recheck_modules(&mut self) {
        self.module_watch.checked = None;
    }

    /// Asks for a project to load as a module.
    pub(super) fn load_module_dialog(&mut self) {
        let mut d = rfd::FileDialog::new()
            .set_title("Cargar un proyecto como módulo")
            .add_filter("ArchiSim", &[archisim_file::EXTENSION]);
        if let Some(dir) = self.start_dir() {
            d = d.set_directory(dir);
        }
        if let Some(path) = d.pick_file() {
            self.load_module_from(&path);
        }
    }

    pub(super) fn load_module_from(&mut self, path: &std::path::Path) {
        match self.ed.load_module(path) {
            Ok(id) => {
                self.module_tool.source = Some(id);
                self.recheck_modules();
                let name = self
                    .ed
                    .model()
                    .module_source(id)
                    .map(|s| s.name.clone())
                    .unwrap_or_default();
                self.set_tool(Tool::Module);
                self.say(format!(
                    "Módulo «{name}» cargado con sus tipos. Clic: colocar una copia · Espacio: girar 90°."
                ));
            }
            Err(e) => self.complain(format!("No se puede cargar el módulo: {e}.")),
        }
    }

    /// The module the tool places: the chosen one, or the first.
    fn tool_module(&self) -> Option<ModuleSourceId> {
        let m = self.ed.model();
        self.module_tool
            .source
            .filter(|s| m.module_source(*s).is_some())
            .or_else(|| m.module_sources().next().map(|s| s.id))
    }

    /// A click of the module tool: a copy there.
    pub(super) fn module_click(&mut self, p: Point2) {
        let Some(source) = self.tool_module() else {
            self.complain("Carga primero un módulo (Propiedades → Cargar módulo…).");
            return;
        };
        let t = self.module_tool;
        match self.ed.place_module(source, p, t.rotation, t.mirror) {
            Ok(id) => {
                self.ed.selection = [Element::Module(id)].into_iter().collect();
                let n = self.ed.model().module_copies(source);
                self.say(format!(
                    "Copia {n} colocada. Clic: otra · Espacio: girar 90° · Esc: flecha."
                ));
            }
            Err(e) => self.complain(format!("No se puede colocar el módulo: {e}.")),
        }
    }

    /// Space with the module tool: the next copy turns 90°.
    pub(super) fn turn_module_tool(&mut self) {
        let r =
            crate::editor::normal_angle(self.module_tool.rotation + std::f64::consts::FRAC_PI_2);
        self.module_tool.rotation = r;
        self.say(format!("Giro {}°.", r.to_degrees().round()));
    }

    /// The box of the copy that would be placed.
    pub(super) fn draw_module_preview(&self, painter: &egui::Painter, rect: Rect) {
        if self.tool != Tool::Module {
            return;
        }
        let (Some(c), Some(source)) = (self.cursor, self.tool_module()) else {
            return;
        };
        let model = self.ed.model();
        let Some(s) = model.module_source(source) else {
            return;
        };
        let t = ModuleTransform {
            position: c.point,
            rotation: self.module_tool.rotation,
            mirror: self.module_tool.mirror,
        };
        // The module's walls, placed, and its box.
        for w in s.model.walls() {
            painter.line_segment(
                [
                    self.view.to_screen(rect, t.apply(w.start)),
                    self.view.to_screen(rect, t.apply(w.end)),
                ],
                Stroke::new(1.5, ACCENT),
            );
        }
        let probe = s.model.plan_box().map(|(lo, hi)| {
            [lo, Point2::new(hi.x, lo.y), hi, Point2::new(lo.x, hi.y)]
                .into_iter()
                .map(|q| t.apply(q))
                .collect::<Vec<_>>()
        });
        if let Some(outline) = probe.filter(|o| o.len() > 2) {
            let pts = self.polygon(rect, &outline);
            dashed_path(painter, &[pts.clone(), vec![pts[0]]].concat(), ACCENT);
        }
        self.draw_label(
            painter,
            rect,
            c.point,
            format!(
                "{}   {}°{}",
                s.name,
                self.module_tool.rotation.to_degrees().round(),
                if self.module_tool.mirror {
                    "   simetría"
                } else {
                    ""
                }
            ),
        );
    }

    // ---- Properties ---------------------------------------------------------

    pub(super) fn module_tool_properties(&mut self, ui: &mut Ui) {
        props::title(ui, "Herramienta Módulo · lo que vas a colocar");
        let sources: Vec<(ModuleSourceId, String)> = self
            .ed
            .model()
            .module_sources()
            .map(|s| (s.id, s.name.clone()))
            .collect();
        let current = self.tool_module();
        group(ui, "tool-module", "Módulo", |ui| {
            if sources.is_empty() {
                props::note(
                    ui,
                    "Un módulo es otro proyecto de ArchiSim (una vivienda tipo) que se coloca aquí tantas veces como haga falta. Cárgalo primero.",
                );
            } else {
                row(ui, "Módulo", "El proyecto que se coloca", |ui, w| {
                    let options: Vec<(ModuleSourceId, &str)> =
                        sources.iter().map(|(id, n)| (*id, n.as_str())).collect();
                    let v = props::choice(
                        ui,
                        "tool-module-source",
                        w,
                        current.unwrap_or(sources[0].0),
                        &options,
                    );
                    self.module_tool.source = Some(v);
                });
                row(ui, "Giro", "Espacio: 90° más", |ui, w| {
                    if let Some(v) = super::structure_ui::angle_field(
                        ui,
                        Id::unique("tool-module-rot"),
                        self.module_tool.rotation,
                        w - 8.0,
                    ) {
                        self.module_tool.rotation = v;
                    }
                });
                row(
                    ui,
                    "Simetría",
                    "Para pareadas y adosadas: el módulo reflejado",
                    |ui, _| {
                        ui.switch(&mut self.module_tool.mirror, "");
                    },
                );
                props::note(ui, "El clic marca dónde va el origen (0,0) del módulo.");
            }
        });
        ui.add_space(4.0);
        if ui
            .button("Cargar módulo…")
            .on_hover_text("Elegir un proyecto .archisim para colocarlo como módulo")
            .clicked()
        {
            self.load_module_dialog();
        }
    }

    pub(super) fn module_properties(&mut self, ui: &mut Ui, id: ModuleId) {
        let model = self.ed.model();
        let Some(p) = model.module(id).cloned() else {
            return;
        };
        let Some((name, path)) = model
            .module_source(p.source)
            .map(|s| (s.name.clone(), s.path.clone()))
        else {
            return;
        };
        let copies = model.module_copies(p.source);
        props::title(ui, &format!("Módulo «{name}» · 1 seleccionado"));
        group(ui, "module-data", "Módulo", |ui| {
            text_row(ui, "Proyecto", "El fichero del módulo", &path);
            text_row(ui, "Copias", "En todo el proyecto", &copies.to_string());
            let status = match self.module_status_of(p.source) {
                ModuleStatus::UpToDate => "Al día",
                ModuleStatus::Changed => "El fichero ha cambiado",
                ModuleStatus::Missing => "No se encuentra el fichero (se usa la copia guardada)",
            };
            text_row(ui, "Estado", "", status);
        });
        group(ui, "module-place", "Colocación", |ui| {
            self.level_row(
                ui,
                ("module-level", id),
                "Planta",
                Some(p.level),
                &[Element::Module(id)],
            );
            for (axis, value) in [("X (mm)", p.position.x), ("Y (mm)", p.position.y)] {
                row(ui, axis, "Del origen del módulo", |ui, w| {
                    if let Some(v) =
                        length_field(ui, Id::unique(("module-pos", axis, id)), value, w - 8.0)
                    {
                        let r = self.ed.edit_module("Mover módulo", id, |m| {
                            if axis.starts_with('X') {
                                m.position.x = v;
                            } else {
                                m.position.y = v;
                            }
                        });
                        self.report(r, "Módulo movido.");
                    }
                });
            }
            row(ui, "Giro", "", |ui, w| {
                if let Some(v) = super::structure_ui::angle_field(
                    ui,
                    Id::unique(("module-rot", id)),
                    p.rotation,
                    w - 8.0,
                ) {
                    let r = self.ed.edit_module("Girar módulo", id, |m| m.rotation = v);
                    self.report(r, "Módulo girado.");
                }
            });
            row(
                ui,
                "Simetría",
                "El módulo reflejado (pareadas, adosadas)",
                |ui, _| {
                    let mut mirror = p.mirror;
                    if ui.switch(&mut mirror, "").changed() {
                        let r = self
                            .ed
                            .edit_module("Simetría del módulo", id, |m| m.mirror = mirror);
                        self.report(r, "Simetría cambiada.");
                    }
                },
            );
        });
        ui.add_space(4.0);
        ui.horizontal(|ui| {
            if ui
                .button("Actualizar")
                .on_hover_text("Leer otra vez el fichero del módulo: cambian todas sus copias")
                .clicked()
            {
                self.update_module(p.source);
            }
            if ui
                .button(format!("Seleccionar las {copies}"))
                .on_hover_text("Todas las copias de este módulo")
                .clicked()
            {
                let all: Vec<Element> = self
                    .ed
                    .model()
                    .modules()
                    .filter(|m| m.source == p.source)
                    .map(|m| Element::Module(m.id))
                    .collect();
                self.ed.selection = all.into_iter().collect();
            }
        });
        if let Some(problem) = self.ed.modules_problem() {
            props::note(ui, &format!("Aviso: {problem}"));
        }
        props::note(
            ui,
            "Una copia no se edita suelta: se cambia el proyecto del módulo y se actualiza. Para algo distinto, otro módulo.",
        );
    }

    pub(super) fn update_module(&mut self, id: ModuleSourceId) {
        let name = self
            .ed
            .model()
            .module_source(id)
            .map(|s| s.name.clone())
            .unwrap_or_default();
        match self.ed.update_module(id) {
            Ok(()) => {
                self.recheck_modules();
                let n = self.ed.model().module_copies(id);
                self.say(format!("Módulo «{name}» actualizado: {n} copias."));
            }
            Err(e) => self.complain(format!("No se puede actualizar «{name}»: {e}.")),
        }
    }

    // ---- Navigator ----------------------------------------------------------

    /// The Modules section of the navigator (only when the project has
    /// modules).
    pub(super) fn nav_modules(&mut self, ui: &mut Ui) {
        let sources: Vec<(ModuleSourceId, String)> = self
            .ed
            .model()
            .module_sources()
            .map(|s| (s.id, s.name.clone()))
            .collect();
        if sources.is_empty() {
            return;
        }
        let mut update = None;
        let mut pick = None;
        let mut remove = None;
        let rows: Vec<(ModuleSourceId, String, usize, ModuleStatus)> = sources
            .into_iter()
            .map(|(id, name)| {
                let n = self.ed.model().module_copies(id);
                (id, name, n, self.module_status_of(id))
            })
            .collect();
        super::navigator::nav_section_pub(ui, "nav-modules", "MÓDULOS", |ui| {
            for (id, name, n, status) in &rows {
                let note = match status {
                    ModuleStatus::UpToDate => format!("{n} copias"),
                    ModuleStatus::Changed => "ha cambiado".to_string(),
                    ModuleStatus::Missing => "sin fichero".to_string(),
                };
                let r =
                    super::widgets::list_row(ui, Some(Icon::Folder), None, name, &note, false, 0.0)
                        .on_hover_text("Clic: colocar copias · Menú: actualizar");
                if r.clicked() {
                    pick = Some(*id);
                }
                r.context_menu(|ui| {
                    if ui.button("Actualizar").clicked() {
                        update = Some(*id);
                        ui.close();
                    }
                    if ui.button("Quitar del proyecto (con sus copias)").clicked() {
                        remove = Some(*id);
                        ui.close();
                    }
                });
                if *status == ModuleStatus::Changed {
                    ui.horizontal(|ui| {
                        ui.add_space(30.0);
                        if ui
                            .small_button(format!("Actualizar «{name}»"))
                            .on_hover_text("El fichero del módulo ha cambiado: leerlo otra vez")
                            .clicked()
                        {
                            update = Some(*id);
                        }
                    });
                }
            }
        });
        if let Some(id) = update {
            self.update_module(id);
        }
        if let Some(id) = remove {
            let r = self.ed.remove_module_source(id);
            self.recheck_modules();
            self.report(r, "Módulo quitado con sus copias.");
        }
        if let Some(id) = pick {
            self.module_tool.source = Some(id);
            self.set_tool(Tool::Module);
        }
    }
}
