//! The navigator (ArchiCAD's) or project browser (Revit's): the views of
//! the project, its sheets, and what the model is made of — each type
//! with how many elements use it, the unit of measurement.
//!
//! Clic on a view opens it in a tab. Clic on a type selects its elements
//! on the current level; double clic (or the context menu) edits it.

use archisim_core::LevelId;
use eframe::egui::{self, collapsing_header::CollapsingState, Align2, Pos2, Rect, Sense, Ui, Vec2};

use super::icons::{self, Icon};
use super::tabs::TabKind;
use super::theme::Palette;
use super::widgets::{list_row, strong_font};
use super::*;

/// A type row of the model tree.
struct TypeRow {
    tab: TypesTab,
    id: u64,
    badge: Option<String>,
    name: String,
    /// Elements of this type in the whole project and on the level.
    count: usize,
    here: Vec<Element>,
}

impl ArchiSimApp {
    pub(super) fn navigator(&mut self, ui: &mut Ui) {
        let p = Palette::current(ui.ctx());
        ui.add_space(8.0);
        ui.horizontal(|ui| {
            ui.label(
                egui::RichText::new("Navegador")
                    .font(strong_font(13.0))
                    .color(p.text),
            );
        });
        ui.add_space(4.0);
        egui::ScrollArea::vertical()
            .id_salt("navigator")
            .auto_shrink([false, false])
            .show(ui, |ui| {
                self.nav_views(ui);
                self.nav_sheets(ui);
                nav_section(ui, "nav-tables", "TABLAS", |ui| self.nav_table_rows(ui));
                self.nav_model(ui);
                ui.add_space(6.0);
            });
    }

    fn nav_views(&mut self, ui: &mut Ui) {
        let levels: Vec<(LevelId, String, f64)> = self
            .ed
            .model()
            .levels()
            .iter()
            .rev()
            .map(|l| (l.id, l.name.clone(), l.elevation))
            .collect();
        let active = self.tabs.get(self.active_tab).map(|t| t.kind);
        let mut open = None;
        let mut levels_dialog = false;
        nav_section(ui, "nav-views", "VISTAS", |ui| {
            sub_header(ui, "Plantas", Some(("Plantas…", &mut levels_dialog)));
            for (id, name, elevation) in &levels {
                let kind = TabKind::Plan(*id);
                let r = list_row(
                    ui,
                    Some(Icon::Plan),
                    None,
                    name,
                    &fmt_level(*elevation),
                    active == Some(kind),
                    10.0,
                )
                .on_hover_text("Abrir la planta");
                if r.clicked() {
                    open = Some(kind);
                }
            }
            sub_header(ui, "3D", None);
            if list_row(
                ui,
                Some(Icon::ThreeD),
                None,
                "Vista 3D",
                "F3",
                active == Some(TabKind::ThreeD),
                10.0,
            )
            .clicked()
            {
                open = Some(TabKind::ThreeD);
            }
            sub_header(ui, "Secciones y alzados", None);
            later_row(ui, Icon::Section, "Llegan con el punto 2.10");
        });
        if levels_dialog {
            self.show_levels = true;
        }
        if let Some(kind) = open {
            self.open_view(kind);
        }
    }

    fn nav_sheets(&mut self, ui: &mut Ui) {
        let active = self.tabs.get(self.active_tab).map(|t| t.kind);
        let mut new = false;
        let mut open = None;
        nav_section(ui, "nav-sheets", "PLANOS", |ui| {
            sub_header(ui, "Planos", Some(("+ Nuevo plano", &mut new)));
            open = self.nav_sheet_rows(ui, active);
        });
        if new {
            self.new_sheet();
        }
        if let Some(kind) = open {
            self.open_view(kind);
        }
    }

    fn type_rows(&self) -> Vec<(&'static str, Icon, Vec<TypeRow>)> {
        let m = self.ed.model();
        let on_level = crate::pick::elements_on_level(m, self.ed.level);
        let walls = m
            .wall_types()
            .map(|t| {
                let here: Vec<Element> = on_level
                    .iter()
                    .copied()
                    .filter(|e| matches!(e, Element::Wall(w) if m.wall(*w).is_some_and(|w| w.wall_type == t.id)))
                    .collect();
                TypeRow {
                    tab: TypesTab::Walls,
                    id: t.id.0,
                    badge: None,
                    name: t.name.clone(),
                    count: m.walls().filter(|w| w.wall_type == t.id).count(),
                    here,
                }
            })
            .collect();
        let slabs = m
            .slab_types()
            .map(|t| {
                let here: Vec<Element> = on_level
                    .iter()
                    .copied()
                    .filter(|e| matches!(e, Element::Slab(s) if m.slab(*s).is_some_and(|s| s.slab_type == t.id)))
                    .collect();
                TypeRow {
                    tab: TypesTab::Slabs,
                    id: t.id.0,
                    badge: None,
                    name: t.name.clone(),
                    count: m.slabs().filter(|s| s.slab_type == t.id).count(),
                    here,
                }
            })
            .collect();
        let openings = |door: bool| -> Vec<TypeRow> {
            m.opening_types()
                .filter(|t| t.is_door() == door)
                .map(|t| {
                    let here: Vec<Element> = on_level
                        .iter()
                        .copied()
                        .filter(|e| matches!(e, Element::Opening(o) if m.opening(*o).is_some_and(|o| o.opening_type == t.id)))
                        .collect();
                    TypeRow {
                        tab: if door {
                            TypesTab::Doors
                        } else {
                            TypesTab::Windows
                        },
                        id: t.id.0,
                        badge: (!t.mark.is_empty()).then(|| t.mark.clone()),
                        name: t.name.clone(),
                        count: m.openings().filter(|o| o.opening_type == t.id).count(),
                        here,
                    }
                })
                .collect()
        };
        vec![
            ("Muros", Icon::Wall, walls),
            ("Forjados", Icon::Slab, slabs),
            ("Puertas", Icon::Door, openings(true)),
            ("Ventanas", Icon::Window, openings(false)),
        ]
    }

    fn nav_model(&mut self, ui: &mut Ui) {
        let p = Palette::current(ui.ctx());
        let categories = self.type_rows();
        let materials = self.ed.model().materials().count();
        let mut select: Option<(Vec<Element>, String)> = None;
        let mut edit: Option<(TypesTab, u64)> = None;
        let mut show_types = None;
        nav_section(ui, "nav-model", "MODELO", |ui| {
            for (name, icon, rows) in &categories {
                let total: usize = rows.iter().map(|r| r.count).sum();
                let id = ui.make_persistent_id(("nav-cat", *name));
                let mut state = CollapsingState::load_with_default_open(ui.ctx(), id, false);
                let open = state.is_open();
                let header = category_row(ui, *icon, name, total, open);
                if header.clicked() {
                    state.toggle(ui);
                }
                state.show_body_unindented(ui, |ui| {
                    for r in rows {
                        let resp = list_row(
                            ui,
                            None,
                            r.badge.as_deref(),
                            &r.name,
                            &r.count.to_string(),
                            false,
                            26.0,
                        );
                        let resp = resp.on_hover_text(format!(
                            "{} en el proyecto · {} en esta planta\nClic: seleccionarlos en la planta · doble clic: editar el tipo",
                            r.count,
                            r.here.len()
                        ));
                        if resp.double_clicked() {
                            edit = Some((r.tab, r.id));
                        } else if resp.clicked() {
                            select = Some((r.here.clone(), r.name.clone()));
                        }
                        resp.context_menu(|ui| {
                            if ui
                                .button(format!("Seleccionar los {} de esta planta", r.here.len()))
                                .clicked()
                            {
                                select = Some((r.here.clone(), r.name.clone()));
                                ui.close();
                            }
                            if ui.button("✎ Editar tipo…").clicked() {
                                edit = Some((r.tab, r.id));
                                ui.close();
                            }
                        });
                    }
                });
            }
            let r = list_row(
                ui,
                Some(Icon::Hatch),
                None,
                "Materiales",
                &materials.to_string(),
                false,
                0.0,
            )
            .on_hover_text("Abrir los materiales de construcción");
            if r.clicked() {
                show_types = Some(TypesTab::Materials);
            }
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                ui.add_space(6.0);
                if ui
                    .button("Tipos y materiales…")
                    .on_hover_text("Multicapas, tipos de puerta y ventana, materiales")
                    .clicked()
                {
                    show_types = Some(TypesTab::Walls);
                }
            });
            let _ = p;
        });
        if let Some((found, name)) = select {
            let n = found.len();
            self.set_tool(Tool::Arrow);
            self.show_plan();
            self.ed.selection = found.into_iter().collect();
            if n == 0 {
                self.say(format!("No hay elementos «{name}» en esta planta."));
            } else {
                self.say(format!(
                    "Seleccionados {n} elementos «{name}» en la planta."
                ));
            }
        }
        if let Some((tab, id)) = edit {
            self.edit_type(tab, id);
        }
        if let Some(tab) = show_types {
            self.types.tab = tab;
            self.types.open = true;
        }
    }
}

/// A top-level section of the navigator (VISTAS, PLANOS, MODELO).
fn nav_section(ui: &mut Ui, id: &str, title: &str, add: impl FnOnce(&mut Ui)) {
    let p = Palette::current(ui.ctx());
    let id = ui.make_persistent_id(id);
    let mut state = CollapsingState::load_with_default_open(ui.ctx(), id, true);
    ui.add_space(6.0);
    let (rect, resp) =
        ui.allocate_exact_size(Vec2::new(ui.available_width(), 22.0), Sense::click());
    let chevron = if state.is_open() {
        Icon::ChevronDown
    } else {
        Icon::ChevronRight
    };
    let fg = if resp.hovered() { p.text } else { p.muted };
    icons::paint(
        ui.painter(),
        Rect::from_center_size(
            Pos2::new(rect.left() + 8.0, rect.center().y),
            Vec2::splat(11.0),
        ),
        chevron,
        fg,
    );
    ui.painter().text(
        Pos2::new(rect.left() + 18.0, rect.center().y),
        Align2::LEFT_CENTER,
        title,
        strong_font(10.5),
        fg,
    );
    if resp.clicked() {
        state.toggle(ui);
    }
    state.show_body_unindented(ui, add);
}

/// A small heading inside a section ("Plantas"), with an optional button.
fn sub_header(ui: &mut Ui, title: &str, button: Option<(&str, &mut bool)>) {
    let p = Palette::current(ui.ctx());
    ui.horizontal(|ui| {
        ui.set_min_height(22.0);
        ui.add_space(10.0);
        ui.label(egui::RichText::new(title).small().color(p.muted));
        if let Some((text, clicked)) = button {
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui
                    .small_button(text)
                    .on_hover_text(if title == "Plantas" {
                        "Añadir, renombrar, cambiar la cota o borrar plantas"
                    } else {
                        "Crear un plano (A1 apaisado) y abrirlo"
                    })
                    .clicked()
                {
                    *clicked = true;
                }
            });
        }
    });
}

/// A faint row for what is still to come.
fn later_row(ui: &mut Ui, icon: Icon, text: &str) {
    let p = Palette::current(ui.ctx());
    let (rect, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 24.0), Sense::hover());
    icons::paint(
        ui.painter(),
        Rect::from_center_size(
            Pos2::new(rect.left() + 25.0, rect.center().y),
            Vec2::splat(14.0),
        ),
        icon,
        p.faint,
    );
    ui.painter().text(
        Pos2::new(rect.left() + 40.0, rect.center().y),
        Align2::LEFT_CENTER,
        text,
        egui::FontId::proportional(11.5),
        p.faint,
    );
}

/// A category of the model tree (Muros, Forjados…) that folds its types.
fn category_row(ui: &mut Ui, icon: Icon, name: &str, count: usize, open: bool) -> egui::Response {
    let p = Palette::current(ui.ctx());
    let (rect, resp) =
        ui.allocate_exact_size(Vec2::new(ui.available_width(), 28.0), Sense::click());
    if resp.hovered() {
        ui.painter()
            .rect_filled(rect, egui::CornerRadius::same(6), p.raised);
    }
    let cy = rect.center().y;
    icons::paint(
        ui.painter(),
        Rect::from_center_size(Pos2::new(rect.left() + 8.0, cy), Vec2::splat(10.0)),
        if open {
            Icon::ChevronDown
        } else {
            Icon::ChevronRight
        },
        p.muted,
    );
    icons::paint(
        ui.painter(),
        Rect::from_center_size(Pos2::new(rect.left() + 25.0, cy), Vec2::splat(15.0)),
        icon,
        p.text,
    );
    ui.painter().text(
        Pos2::new(rect.left() + 40.0, cy),
        Align2::LEFT_CENTER,
        name,
        egui::FontId::proportional(12.5),
        p.text,
    );
    ui.painter().text(
        Pos2::new(rect.right() - 8.0, cy),
        Align2::RIGHT_CENTER,
        count.to_string(),
        egui::FontId::monospace(11.0),
        p.muted,
    );
    resp
}
