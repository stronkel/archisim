//! The bars around the drawing: the top bar (menus, project, quick
//! commands, theme) and the bottom bar (scale, zoom, snaps and display of
//! the active view, cursor position), plus what floats over the drawing:
//! the keys of the next step and the message of the last action.

use eframe::egui::{self, FontId, Rect, Sense, Ui, Vec2};

use super::icons::{self, Icon};
use super::tabs::SCALES;
use super::theme::Palette;
use super::widgets::{self, chip, icon_button, strong_font};
use super::*;

/// How long a message stays over the drawing, in seconds (errors longer).
const MESSAGE_SECONDS: f64 = 4.5;
const ERROR_SECONDS: f64 = 9.0;
const FADE_SECONDS: f64 = 0.6;

/// Screen points per paper millimetre at 100 % zoom (96 points per inch).
const POINTS_PER_MM: f64 = 96.0 / 25.4;

impl ArchiSimApp {
    // ---- Top bar ------------------------------------------------------------

    pub(super) fn top_bar(&mut self, ui: &mut Ui) {
        let p = Palette::current(ui.ctx());
        ui.horizontal(|ui| {
            ui.set_min_height(36.0);
            // The mark of the application.
            let (r, _) = ui.allocate_exact_size(Vec2::new(26.0, 26.0), Sense::hover());
            ui.painter()
                .rect_filled(r, egui::CornerRadius::same(7), p.accent);
            icons::paint(
                ui.painter(),
                r.shrink(5.0),
                Icon::Wall,
                egui::Color32::WHITE,
            );
            ui.label(
                egui::RichText::new("ArchiSim")
                    .font(strong_font(14.0))
                    .color(p.text),
            );
            ui.add_space(6.0);
            ui.scope(|ui| {
                // Menus as plain words, as in any desktop program.
                let w = &mut ui.visuals_mut().widgets;
                w.inactive.weak_bg_fill = egui::Color32::TRANSPARENT;
                w.inactive.bg_stroke = egui::Stroke::NONE;
                w.hovered.bg_stroke = egui::Stroke::NONE;
                w.open.bg_stroke = egui::Stroke::NONE;
                ui.spacing_mut().button_padding = Vec2::new(7.0, 4.0);
                self.menus(ui);
            });
            ui.add_space(10.0);
            // Project and file, in the middle.
            let modified = self.ed.is_modified();
            ui.label(
                egui::RichText::new(&self.ed.model().project_name)
                    .font(strong_font(12.5))
                    .color(p.text),
            );
            ui.label(
                egui::RichText::new(format!(
                    "{}{}",
                    self.ed.display_name(),
                    if modified { " · sin guardar" } else { "" }
                ))
                .small()
                .color(if modified { p.accent } else { p.muted }),
            );
            ui.add_space(14.0);
            self.search_field(ui);
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.spacing_mut().item_spacing.x = 4.0;
                let dark = ui.ctx().global_style().visuals.dark_mode;
                if icon_button(
                    ui,
                    Icon::Theme,
                    if dark { "Tema claro" } else { "Tema oscuro" },
                    false,
                )
                .clicked()
                {
                    self.dark = !dark;
                    theme::apply(ui.ctx(), self.dark);
                }
                if icon_button(
                    ui,
                    Icon::SidebarRight,
                    "Mostrar u ocultar el navegador",
                    self.show_navigator,
                )
                .clicked()
                {
                    self.show_navigator = !self.show_navigator;
                }
                if icon_button(
                    ui,
                    Icon::Sidebar,
                    "Mostrar u ocultar las propiedades",
                    self.show_properties,
                )
                .clicked()
                {
                    self.show_properties = !self.show_properties;
                }
                ui.separator();
                if ui
                    .add(egui::Button::new(
                        egui::RichText::new("Exportar IFC").font(strong_font(12.0)),
                    ))
                    .on_hover_text("Exportar el proyecto en IFC 4.3")
                    .clicked()
                {
                    self.export_ifc(archisim_ifc::Schema::Ifc4x3);
                }
                if icon_button(ui, Icon::Save, "Guardar (Ctrl+S)", false).clicked() {
                    self.save(false);
                }
                let redo = match self.ed.redo_label() {
                    Some(l) => format!("Rehacer «{l}» (Ctrl+Y)"),
                    None => "Nada que rehacer".into(),
                };
                if widgets::icon_button_sized(
                    ui,
                    Icon::Redo,
                    &redo,
                    false,
                    Vec2::new(28.0, 26.0),
                    self.ed.redo_label().is_some(),
                )
                .clicked()
                {
                    self.redo();
                }
                let undo = match self.ed.undo_label() {
                    Some(l) => format!("Deshacer «{l}» (Ctrl+Z)"),
                    None => "Nada que deshacer".into(),
                };
                if widgets::icon_button_sized(
                    ui,
                    Icon::Undo,
                    &undo,
                    false,
                    Vec2::new(28.0, 26.0),
                    self.ed.undo_label().is_some(),
                )
                .clicked()
                {
                    self.undo();
                }
            });
        });
    }

    fn menus(&mut self, ui: &mut Ui) {
        ui.menu_button("Archivo", |ui| {
            if ui.add(shortcut_button("Nuevo", "Ctrl+N")).clicked() {
                self.guarded(Pending::New);
            }
            if ui.add(shortcut_button("Abrir…", "Ctrl+O")).clicked() {
                self.guarded(Pending::Open(None));
            }
            ui.separator();
            if ui.add(shortcut_button("Guardar", "Ctrl+S")).clicked() {
                self.save(false);
            }
            if ui
                .add(shortcut_button("Guardar como…", "Ctrl+Mayús+S"))
                .clicked()
            {
                self.save(true);
            }
            ui.separator();
            if ui.button("Importar IFC…").clicked() {
                self.guarded(Pending::Import(None));
            }
            if ui
                .button("Importar topografía (DXF, CSV)…")
                .on_hover_text("Puntos con cota y curvas de nivel de un levantamiento, en el terreno de la planta")
                .clicked()
            {
                self.pick_survey();
            }
            if ui.button("Exportar IFC 4.3…").clicked() {
                self.export_ifc(archisim_ifc::Schema::Ifc4x3);
            }
            if ui
                .button("Exportar IFC4 (compatibilidad)…")
                .on_hover_text("Para programas que aún no leen IFC 4.3: la vegetación y los vehículos van como elementos genéricos")
                .clicked()
            {
                self.export_ifc(archisim_ifc::Schema::Ifc4);
            }
            let section = matches!(self.mode, ViewMode::Section);
            let what = if section {
                "Exportar sección o alzado (PDF, DXF, SVG)"
            } else {
                "Exportar planta (PDF, DXF, SVG)"
            };
            ui.menu_button(what, |ui| {
                for s in SCALES {
                    if ui.button(format!("A escala 1:{s}…")).clicked() {
                        if section {
                            self.export_section(s);
                        } else {
                            self.export_plan(s);
                        }
                    }
                }
            });
            let on_sheet = matches!(self.mode, ViewMode::Sheet(_));
            if ui
                .add_enabled(
                    on_sheet,
                    egui::Button::new("Exportar este plano (PDF, DXF, SVG)…"),
                )
                .clicked()
            {
                self.export_sheets(false);
            }
            if ui
                .add_enabled(
                    !self.ed.model().sheets().is_empty(),
                    egui::Button::new("Exportar todos los planos (PDF)…"),
                )
                .clicked()
            {
                self.export_sheets(true);
            }
            if ui
                .button("Exportar mediciones (BC3)…")
                .on_hover_text("Muros, forjados, carpinterías y acabados de locales, por tipo, para Presto, Arquímedes y otros")
                .clicked()
            {
                self.export_bc3();
            }
            if ui
                .button("Importar DXF en la vista…")
                .on_hover_text(
                    "Líneas, textos, sombreados y bloques del DXF, como dibujo 2D en sus capas",
                )
                .clicked()
            {
                self.import_dxf();
            }
            if ui
                .add_enabled(
                    self.mode == ViewMode::ThreeD,
                    egui::Button::new("Guardar imagen 3D (PNG)…"),
                )
                .clicked()
            {
                self.export_3d_picture();
            }
            ui.separator();
            if ui.button("Salir").clicked() {
                ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
            }
        });
        ui.menu_button("Edición", |ui| {
            let undo = match self.ed.undo_label() {
                Some(l) => format!("Deshacer «{l}»"),
                None => "Deshacer".into(),
            };
            if ui
                .add_enabled(
                    self.ed.undo_label().is_some(),
                    shortcut_button(&undo, "Ctrl+Z"),
                )
                .clicked()
            {
                self.undo();
            }
            let redo = match self.ed.redo_label() {
                Some(l) => format!("Rehacer «{l}»"),
                None => "Rehacer".into(),
            };
            if ui
                .add_enabled(
                    self.ed.redo_label().is_some(),
                    shortcut_button(&redo, "Ctrl+Y"),
                )
                .clicked()
            {
                self.redo();
            }
            ui.separator();
            if ui
                .add_enabled(
                    !self.ed.selection.is_empty(),
                    shortcut_button("Borrar", "Supr"),
                )
                .clicked()
            {
                self.delete();
            }
            if ui
                .add(shortcut_button("Seleccionar todo", "Ctrl+A"))
                .clicked()
            {
                self.set_tool(Tool::Arrow);
                self.select_all();
            }
            if ui.add(shortcut_button("Anular selección", "Esc")).clicked() {
                self.ed.selection.clear();
            }
        });
        ui.menu_button("Vista", |ui| {
            if ui.add(shortcut_button("Planta", "F2")).clicked() {
                self.set_mode(ViewMode::Plan);
            }
            if ui.add(shortcut_button("3D", "F3")).clicked() {
                self.set_mode(ViewMode::ThreeD);
            }
            if ui
                .add(shortcut_button("Ajustar a la ventana", "Inicio"))
                .clicked()
            {
                self.fit_active_view();
            }
            ui.separator();
            ui.checkbox(&mut self.show_reference_lines, "Líneas de referencia");
            ui.checkbox(&mut self.show_grid, "Rejilla");
            ui.checkbox(&mut self.thin_lines, "Líneas finas");
            ui.checkbox(&mut self.show_underlay, "Calco de la planta inferior");
            ui.checkbox(&mut self.v3d.camera.perspective, "Perspectiva en 3D");
            ui.separator();
            ui.checkbox(&mut self.show_properties, "Propiedades");
            ui.checkbox(&mut self.show_navigator, "Navegador");
            if ui
                .checkbox(&mut self.attenuate, "Atenuar por disciplina")
                .on_hover_text("En Estructura, Instalaciones, Urbanismo o Concepto, lo que no es de la disciplina se ve en gris")
                .changed()
            {
                if self.attenuate {
                    self.apply_focus();
                } else {
                    self.view_style.focus = archisim_core::DisciplineFocus::None;
                }
            }
            ui.checkbox(&mut self.pin_command_line, "Línea de órdenes fija")
                .on_hover_text("Sin marcar, lo que escribes sale junto al cursor y el paso de la orden en marcha, en la barra bajo las pestañas");
            ui.checkbox(&mut self.keys_autocad, "Teclas de AutoCAD")
                .on_hover_text("Marcado: se escribe la orden y se pulsa Intro (L, PL, M de muro…). Sin marcar: M, F, P, V, C y N cambian de herramienta al momento, como en ArchiCAD");
            ui.separator();
            if ui.button("Plantas…").clicked() {
                self.show_levels = true;
            }
            if ui.button("Tipos y materiales…").clicked() {
                self.types.open = true;
            }
            if ui.button("Tipos de muro cortina…").clicked() {
                self.open_curtain_types(None, None);
            }
            if ui
                .button("Biblioteca…")
                .on_hover_text(
                    "Cargar tipos de la biblioteca, actualizar familias, purgar tipos sin usar",
                )
                .clicked()
            {
                self.library.open = true;
            }
            if ui
                .button("Parámetros del proyecto…")
                .on_hover_text(
                    "Datos propios de puertas, muros, locales…: resistencia al fuego, acabados, códigos de medición (y compartidos entre proyectos)",
                )
                .clicked()
            {
                self.params.open = true;
            }
        });
        self.coord_menu(ui);
        ui.menu_button("Ayuda", |ui| {
            if ui.button("Ratón y teclado").clicked() {
                self.show_shortcuts = true;
            }
            if ui.button("Acerca de ArchiSim").clicked() {
                self.show_about = true;
            }
        });
    }

    // ---- Bottom bar ------------------------------------------------------------

    /// Zoom as a percentage of the paper at its real size on the screen.
    pub(super) fn zoom_percent(&self) -> f64 {
        let real = 1000.0 / self.plan_scale * POINTS_PER_MM;
        self.view.zoom / real * 100.0
    }

    pub(super) fn set_scale(&mut self, scale: f64) {
        if (scale - self.plan_scale).abs() > 1e-9 {
            // The model stays the same size on the screen; lines, texts
            // and symbols change (and the zoom, measured on the paper).
            self.plan_scale = scale;
            self.say(format!(
                "Escala 1:{scale}: grosores, textos y símbolos de puertas y ventanas a esa escala."
            ));
        }
    }

    pub(super) fn fit_active_view(&mut self) {
        match self.mode {
            ViewMode::Plan | ViewMode::Section | ViewMode::Sheet(_) => self.fit_requested = true,
            ViewMode::ThreeD => self.v3d.fit_requested = true,
            ViewMode::Schedule => {}
        }
    }

    pub(super) fn bottom_bar(&mut self, ui: &mut Ui) {
        let p = Palette::current(ui.ctx());
        ui.horizontal(|ui| {
            ui.set_min_height(30.0);
            ui.spacing_mut().item_spacing.x = 6.0;
            match self.mode {
                ViewMode::Plan => {
                    self.scale_and_zoom(ui);
                    ui.separator();
                    self.drawing_segments(ui);
                    ui.separator();
                    self.snap_toggles(ui);
                }
                ViewMode::Section => {
                    self.scale_and_zoom(ui);
                    ui.separator();
                    chip(
                        ui,
                        &mut self.thin_lines,
                        "Líneas finas",
                        "Todas las líneas a un píxel, sin grosor; apagado, cada línea con su grosor real a la escala de la vista",
                    );
                }
                ViewMode::Sheet(id) => {
                    let paper = self
                        .ed
                        .model()
                        .sheet(id)
                        .map(|s| format!("Papel {} · 1:1", s.size_name()))
                        .unwrap_or_default();
                    ui.label(egui::RichText::new(paper).small().color(p.muted));
                    ui.separator();
                    self.drawing_segments(ui);
                    ui.separator();
                    self.snap_toggles(ui);
                }
                ViewMode::Schedule => {
                    ui.label(
                        egui::RichText::new("Tabla · se rehace sola al cambiar el modelo")
                            .small()
                            .color(p.muted),
                    );
                }
                ViewMode::ThreeD => {
                    let cam = &mut self.v3d.camera;
                    let mut persp = cam.perspective;
                    ui.allocate_ui(Vec2::new(220.0, 26.0), |ui| {
                        widgets::segmented(
                            ui,
                            "bar-projection",
                            &mut persp,
                            &[
                                (true, "Perspectiva", ""),
                                (false, "Axonometría", "Proyección paralela"),
                            ],
                        );
                    });
                    cam.perspective = persp;
                    ui.separator();
                    if icon_button(ui, Icon::Fit, "Ajustar (Inicio)", false).clicked() {
                        self.v3d.fit_requested = true;
                    }
                    if icon_button(ui, Icon::Export, "Guardar imagen 3D (PNG)", false).clicked() {
                        self.export_3d_picture();
                    }
                    ui.separator();
                    let mut on = self.v3d.section.is_some();
                    chip(
                        ui,
                        &mut on,
                        "Caja de sección",
                        "Recorta el modelo con una caja, como en Revit: arrastra las flechas azules de sus caras para ver interiores o una sección en perspectiva. Medidas exactas en Propiedades",
                    );
                    if on != self.v3d.section.is_some() {
                        self.v3d.toggle_section();
                    }
                }
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let level = self
                    .ed
                    .model()
                    .level(self.ed.level)
                    .map(|l| format!("{} {}", l.name, fmt_level(l.elevation)))
                    .unwrap_or_default();
                if let ViewMode::Sheet(_) = self.mode {
                    if let Some(c) = self.cursor {
                        ui.label(
                            egui::RichText::new(format!(
                                "X {:.1}   Y {:.1} mm",
                                c.point.x * 1000.0,
                                c.point.y * 1000.0
                            ))
                            .monospace()
                            .color(p.text),
                        );
                        if c.kind != crate::snap::SnapKind::Free {
                            ui.label(egui::RichText::new(c.kind.label()).small().color(p.accent));
                        }
                    }
                } else if self.mode == ViewMode::Plan {
                    ui.label(egui::RichText::new(level).small().color(p.muted));
                    if let Some(c) = self.cursor {
                        ui.separator();
                        let kind = c.kind.label();
                        ui.label(
                            egui::RichText::new(format!(
                                "X {}   Y {}",
                                fmt_m(c.point.x),
                                fmt_m(c.point.y)
                            ))
                            .monospace()
                            .color(p.text),
                        );
                        if c.kind != crate::snap::SnapKind::Free {
                            ui.label(egui::RichText::new(kind).small().color(p.accent));
                        }
                    }
                } else if self.mode != ViewMode::Schedule {
                    ui.label(
                        egui::RichText::new(format!(
                            "{} elementos seleccionados",
                            self.ed.selection.len()
                        ))
                        .small()
                        .color(p.muted),
                    );
                }
            });
        });
    }

    /// The scale of the view and the zoom, at the left of the bottom bar.
    fn scale_and_zoom(&mut self, ui: &mut Ui) {
        let p = Palette::current(ui.ctx());
        let mut scale = self.plan_scale as u32;
        egui::ComboBox::from_id_salt("bar-scale")
            .width(70.0)
            .selected_text(egui::RichText::new(format!("1:{scale}")).font(strong_font(12.0)))
            .show_ui(ui, |ui| {
                for s in SCALES {
                    ui.selectable_value(&mut scale, s as u32, format!("1:{s}"));
                }
            })
            .response
            .on_hover_text("Escala del dibujo: grosores de línea, textos y símbolos");
        self.set_scale(scale as f64);
        let rect = self.canvas;
        if icon_button(ui, Icon::Minus, "Alejar", false).clicked() {
            self.view.zoom_at(rect, rect.center(), 1.0 / 1.25);
        }
        ui.add_sized(
            [42.0, 20.0],
            egui::Label::new(
                egui::RichText::new(format!("{:.0} %", self.zoom_percent()))
                    .monospace()
                    .color(p.text),
            ),
        )
        .on_hover_text("100 %: el papel a su tamaño real en la pantalla");
        if icon_button(ui, Icon::Plus, "Acercar", false).clicked() {
            self.view.zoom_at(rect, rect.center(), 1.25);
        }
        if icon_button(ui, Icon::Fit, "Ajustar a la ventana (Inicio)", false).clicked() {
            self.fit_requested = true;
        }
    }

    // ---- Over the drawing --------------------------------------------------------

    /// What the keys and the mouse do now, as "Clic: … · Esc: …".
    pub(super) fn hint_text(&self) -> &'static str {
        if self.mode == ViewMode::Section {
            return "Rueda: zoom · Botón derecho o central: desplazar · Inicio: ajustar · F2: planta · F3: 3D";
        }
        if self.mode == ViewMode::ThreeD {
            if self.v3d.section.is_some() {
                return "Flechas azules: mover las caras de la caja de sección · Arrastrar: girar · Rueda: zoom · Clic: seleccionar · F2: planta";
            }
            return "Arrastrar: girar · Rueda: zoom · Botón derecho: encuadre · Clic: seleccionar · Inicio: ajustar · F2: planta";
        }
        if matches!(self.mode, ViewMode::Sheet(_)) && self.tool != Tool::Cad {
            return "Clic: seleccionar vista o dibujo · Arrastrar vista: colocarla · Doble clic: abrir su planta · L, TX…: dibujar en el plano · Supr: quitar";
        }
        match self.tool {
            Tool::Arrow if self.drag.is_some() => "Soltar: terminar · Ctrl: copiar · Mayús: 45° · Distancia + Intro: exacto · Esc: cancelar",
            Tool::Arrow if self.temp_edit.is_some() => "Escribe la medida (mm, o 1,2m · 120cm) · Intro: aplicar · Esc: dejarla",
            Tool::Arrow if self.single_opening_selected() => "Clic en una cota azul: medida exacta · Espacio o flechas azules: girar · Arrastrar: deslizar por el muro · Supr: borrar",
            Tool::Arrow if self.single_curved_wall_selected() => "Círculo blanco: curvar · Espacio: invertir · Arrastrar: mover · Extremo: estirar · Propiedades: radio · Supr: borrar",
            Tool::Arrow if self.single_wall_selected() => "Clic en una cota azul: medida exacta · Espacio: invertir · Arrastrar: mover · Extremo: estirar · Supr: borrar",
            Tool::Arrow if !self.ed.selection.is_empty() => "Arrastrar: mover · Ctrl: copiar · Extremo o esquina: estirar · Rombo: mover la arista · Supr: borrar · Esc: anular",
            Tool::Arrow => "Clic: seleccionar · Arrastrar: ventana de selección · Rueda: zoom · Botón derecho: encuadre · M F P V C I G Z: herramientas · Orden (L, PL, C…) + Intro: dibujar y editar",
            Tool::Wall => self.wall_hint(),
            Tool::Slab if !self.slab_tool.is_drawing() && self.slab_settings.wand => "Clic: dentro de un recinto de muros · Esc: flecha",
            Tool::Slab if !self.slab_tool.is_drawing() => "Clic: primera esquina · Espacio + clic: varita mágica · Esc: flecha",
            Tool::Slab if self.slab_tool.mode == SlabMode::Rectangle => "Clic: esquina opuesta · Ancho x fondo (6000x4000) + Intro: exacto · Esc: anular",
            Tool::Slab => "Clic: siguiente esquina · Clic en la primera o Intro: cerrar · Longitud + Intro: exacto · Esc: quitar la última",
            Tool::Door | Tool::Window => "Clic sobre un muro: colocar, centrada en el cursor, abriendo hacia ese lado · Espacio: cambiar la mano · Esc: flecha",
            Tool::Dimension => self.dim_hint(),
            Tool::Column => "Clic: centro del pilar · Espacio: girar 90° · Esc: flecha",
            Tool::Beam if self.beam_tool.is_drawing() => "Clic: final de la viga (y comienzo de la siguiente) · Mayús: 45° · Clic derecho o Esc: terminar",
            Tool::Beam => "Clic: inicio de la viga · Esc: flecha",
            Tool::Zone if self.zone_wand() => "Clic dentro de un recinto de muros: crear el local · Esc: flecha",
            Tool::Zone if !self.zone_tool.is_drawing() => "Clic: primera esquina del local · Esc: flecha",
            Tool::Zone => "Clic: siguiente esquina · Clic en la primera o Intro: cerrar · Esc: quitar la última",
            Tool::Section if self.section_tool.anchor.is_some() => "Clic: final de la sección (mira hacia la izquierda del trazo) · Mayús: 45° · Esc: anular",
            Tool::Section => "Clic: inicio de la sección · Esc: flecha",
            Tool::CurtainWall if self.curtain_anchor.is_some() => "Clic: final del muro cortina (exterior a la izquierda) · Mayús: 45° · Esc: anular",
            Tool::CurtainWall => "Clic: inicio del muro cortina · Esc: flecha",
            Tool::Grid | Tool::Footing => self.foundation_hint(),
            Tool::Object => "Clic: colocar el objeto (lado de atrás en el punto) · Espacio: girar 90° · Esc: flecha",
            Tool::Railing if self.railing_points.is_empty() => "Clic: inicio de la barandilla · Esc: flecha",
            Tool::Railing => "Clic: siguiente punto · Intro, doble clic o clic derecho: terminar · Esc: quitar el último punto",
            Tool::Roof if self.roof_wand() => "Clic dentro del edificio: cubierta por las caras exteriores de sus muros · Esc: flecha",
            Tool::Roof if !self.roof_tool.is_drawing() => "Clic: primera esquina de la cubierta · Esc: flecha",
            Tool::Roof => "Clic: siguiente esquina · Clic en la primera o Intro: cerrar · Esc: quitar la última",
            Tool::Terrain if self.terrain_settings.line => "Escribe la cota en Propiedades y clic: vértices de la línea de nivel · Intro o doble clic: terminarla · Esc: quitar el último",
            Tool::Terrain => "Escribe la cota en Propiedades y clic: punto del terreno (sobre uno que ya existe: le pone la cota) · Intro o doble clic: crear · Esc: quitar el último",
            Tool::Stair if self.stair_tool.points.is_empty() => "Clic: pie de la escalera (inicio de la línea de huella) · Esc: flecha",
            Tool::Stair => "Clic: final del tramo o giro (meseta) · Intro, doble clic o clic derecho: terminar · Esc: quitar el último punto",
            Tool::Cad => "Clic o coordenadas + Intro: punto · Intro, Espacio o botón derecho: aceptar · Esc: terminar · F8: orto",
        }
    }

    fn wall_hint(&self) -> &'static str {
        let t = &self.wall_tool;
        if !t.is_drawing() {
            return match t.mode {
                WallMode::Rectangle => "Clic: primera esquina del rectángulo · Esc: flecha",
                WallMode::Circle => "Clic: centro del círculo · Esc: flecha",
                WallMode::Arc => "Clic: inicio del arco · Esc: flecha",
                WallMode::Single | WallMode::Chain => "Clic: inicio del muro · Esc: flecha",
            };
        }
        match t.mode {
            WallMode::Rectangle => "Clic: esquina opuesta · Ancho x fondo (6000x4000) + Intro: exacto · Esc: anular",
            WallMode::Circle => "Clic: un punto de la línea de referencia · Radio + Intro: exacto · Esc: anular",
            WallMode::Arc if t.arc_end().is_some() => "Clic: punto por el que pasa el arco · Radio + Intro: exacto · Esc: otro final",
            WallMode::Arc => "Clic: final del arco · Cuerda + Intro: exacta · Mayús: 45° · Intro/Esc: terminar",
            WallMode::Single => "Clic: final del muro · Longitud + Intro: exacto · Mayús: 45° · Esc: anular",
            WallMode::Chain => "Clic: siguiente punto · Recto por defecto (sal más de 8° para inclinarlo) · Longitud + Intro: exacto · Mayús: 45° · Intro/Esc: terminar",
        }
    }

    fn single_opening_selected(&self) -> bool {
        self.ed.selection.len() == 1
            && matches!(self.ed.selection.iter().next(), Some(Element::Opening(_)))
    }

    fn single_curved_wall_selected(&self) -> bool {
        match (self.ed.selection.len(), self.ed.selection.iter().next()) {
            (1, Some(Element::Wall(w))) => self.ed.model().wall(*w).is_some_and(|w| w.is_curved()),
            _ => false,
        }
    }

    fn single_wall_selected(&self) -> bool {
        self.ed.selection.len() == 1
            && matches!(self.ed.selection.iter().next(), Some(Element::Wall(_)))
    }

    /// Records when the current message appeared.
    pub(super) fn age_message(&mut self, now: f64) {
        let serial = self.message_serial;
        if self.message_seen.0 != serial {
            self.message_seen = (serial, now);
        }
    }

    /// The hint pill at the bottom and the last message at the top left.
    pub(super) fn draw_overlays(&self, ctx: &egui::Context, painter: &egui::Painter, rect: Rect) {
        let p = Palette::current(ctx);
        widgets::hud(painter, rect, &widgets::parse_hints(self.hint_text()), p);
        if let Some(m) = &self.message {
            let now = ctx.input(|i| i.time);
            let age = now - self.message_seen.1;
            let life = if m.error {
                ERROR_SECONDS
            } else {
                MESSAGE_SECONDS
            };
            let alpha = ((life - age) / FADE_SECONDS).clamp(0.0, 1.0) as f32;
            if alpha > 0.0 {
                widgets::toast(painter, rect, &m.text, m.error, alpha, p);
                ctx.request_repaint_after(std::time::Duration::from_millis(50));
            }
        }
    }

    /// A small label beside the cursor (lengths, angles, areas), flipped
    /// near the view's edges.
    pub(super) fn draw_label(&self, painter: &egui::Painter, rect: Rect, at: Point2, text: String) {
        let cursor = self.view.to_screen(rect, at);
        let light = egui::Color32::from_rgb(240, 242, 245);
        let galley = painter.layout_no_wrap(text, FontId::monospace(12.0), light);
        let mut pos = cursor + Vec2::new(18.0, 20.0);
        if pos.x + galley.size().x + 10.0 > rect.right() {
            pos.x = cursor.x - 18.0 - galley.size().x;
        }
        // Clear of the hint pill at the bottom of the view.
        if pos.y + galley.size().y + 10.0 > rect.bottom() - 56.0 {
            pos.y = cursor.y - 20.0 - galley.size().y;
        }
        let bg = Rect::from_min_size(pos, galley.size()).expand2(Vec2::new(7.0, 4.0));
        painter.rect_filled(
            bg,
            egui::CornerRadius::same(6),
            egui::Color32::from_rgba_unmultiplied(24, 27, 32, 232),
        );
        painter.galley(pos, galley, light);
    }
}
