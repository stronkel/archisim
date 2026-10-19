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
    /// Where the type is edited (columns and beams: in Propiedades).
    tab: Option<TypesTab>,
    id: u64,
    badge: Option<String>,
    name: String,
    /// Elements of this type in the whole project and on the level.
    count: usize,
    here: Vec<Element>,
    /// Shown instead of the count (the area of a room).
    value: Option<String>,
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
        let mut make_elevations = false;
        let mut sections: Vec<(archisim_core::SectionId, String, bool)> = self
            .ed
            .model()
            .sections()
            .map(|s| {
                (
                    s.id,
                    s.title(),
                    s.kind == archisim_core::SectionKind::Elevation,
                )
            })
            .collect();
        // Sections first, then elevations, each by name.
        sections.sort_by(|a, b| a.2.cmp(&b.2).then(a.1.cmp(&b.1)));
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
            sub_header(
                ui,
                "Secciones y alzados",
                Some(("Crear alzados", &mut make_elevations)),
            );
            if sections.is_empty() {
                later_row(
                    ui,
                    Icon::Section,
                    "Herramienta Sección (S) o «Crear alzados»",
                );
            }
            for (id, title, elevation) in &sections {
                let kind = TabKind::Section(*id);
                let r = list_row(
                    ui,
                    Some(if *elevation {
                        Icon::Elevation
                    } else {
                        Icon::Section
                    }),
                    None,
                    title,
                    "",
                    active == Some(kind),
                    10.0,
                )
                .on_hover_text("Abrir la vista");
                if r.clicked() {
                    open = Some(kind);
                }
            }
        });
        if levels_dialog {
            self.show_levels = true;
        }
        if make_elevations {
            self.create_elevations();
        }
        if let Some(kind) = open {
            self.open_view(kind);
        }
    }

    fn nav_sheets(&mut self, ui: &mut Ui) {
        nav_section(ui, "nav-sheets", "PLANOS", |ui| {
            later_row(ui, Icon::Sheet, "Planos y maquetación: fase 3 (3.5)");
        });
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
                    tab: Some(TypesTab::Walls),
                    id: t.id.0,
                    badge: None,
                    name: t.name.clone(),
                    count: m.walls().filter(|w| w.wall_type == t.id).count(),
                    here,
                    value: None,
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
                    tab: Some(TypesTab::Slabs),
                    id: t.id.0,
                    badge: None,
                    name: t.name.clone(),
                    count: m.slabs().filter(|s| s.slab_type == t.id).count(),
                    here,
                    value: None,
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
                        tab: Some(if door {
                            TypesTab::Doors
                        } else {
                            TypesTab::Windows
                        }),
                        id: t.id.0,
                        badge: (!t.mark.is_empty()).then(|| t.mark.clone()),
                        name: t.name.clone(),
                        count: m.openings().filter(|o| o.opening_type == t.id).count(),
                        here,
                        value: None,
                    }
                })
                .collect()
        };
        let members = |usage: archisim_core::MemberUsage| -> Vec<TypeRow> {
            m.member_types()
                .filter(|t| t.usage == usage)
                .map(|t| {
                    let here: Vec<Element> = on_level
                        .iter()
                        .copied()
                        .filter(|e| {
                            crate::pick::type_of(m, *e) == Some(crate::pick::TypeOf::Member(t.id))
                        })
                        .collect();
                    TypeRow {
                        tab: None,
                        id: t.id.0,
                        badge: None,
                        name: t.name.clone(),
                        count: m.member_type_users(t.id),
                        here,
                        value: None,
                    }
                })
                .collect()
        };
        // Stairs, each with its steps.
        let stairs = m
            .stairs()
            .map(|s| {
                let l = s.layout();
                TypeRow {
                    tab: None,
                    id: s.id.0,
                    badge: None,
                    name: format!("Escalera {}", s.id.0),
                    count: 1,
                    here: if s.level == self.ed.level {
                        vec![Element::Stair(s.id)]
                    } else {
                        Vec::new()
                    },
                    value: Some(format!("{} tabicas", l.risers())),
                }
            })
            .collect();
        // Curtain walls and railings, one row each with their length.
        let curtains = m
            .curtain_walls()
            .map(|c| TypeRow {
                tab: None,
                id: c.id.0,
                badge: None,
                name: format!("Muro cortina {}", c.id.0),
                count: 1,
                here: if c.level == self.ed.level {
                    vec![Element::CurtainWall(c.id)]
                } else {
                    Vec::new()
                },
                value: Some(format!("{} m", fmt_m(c.length()))),
            })
            .collect();
        let railings = m
            .railings()
            .map(|r| TypeRow {
                tab: None,
                id: r.id.0,
                badge: None,
                name: format!("Barandilla {}", r.id.0),
                count: 1,
                here: if r.level == self.ed.level {
                    vec![Element::Railing(r.id)]
                } else {
                    Vec::new()
                },
                value: Some(format!("{} m", fmt_m(r.length()))),
            })
            .collect();
        // Objects, by type.
        let objects = m
            .object_types()
            .filter(|t| m.object_type_users(t.id) > 0)
            .map(|t| TypeRow {
                tab: None,
                id: t.id.0,
                badge: None,
                name: t.name.clone(),
                count: m.object_type_users(t.id),
                here: m
                    .objects_on_level(self.ed.level)
                    .filter(|o| o.object_type == t.id)
                    .map(|o| Element::Object(o.id))
                    .collect(),
                value: None,
            })
            .collect();
        // Roofs, by type.
        let roofs = m
            .slab_types()
            .filter(|t| m.roofs().any(|r| r.roof_type == t.id))
            .map(|t| TypeRow {
                tab: Some(TypesTab::Slabs),
                id: t.id.0,
                badge: None,
                name: t.name.clone(),
                count: m.roofs().filter(|r| r.roof_type == t.id).count(),
                here: m
                    .roofs_on_level(self.ed.level)
                    .filter(|r| r.roof_type == t.id)
                    .map(|r| Element::Roof(r.id))
                    .collect(),
                value: None,
            })
            .collect();
        // Rooms of the active level, by number, with their usable area.
        let mut rooms: Vec<&archisim_core::Space> = m.spaces_on_level(self.ed.level).collect();
        rooms.sort_by(|a, b| a.number.cmp(&b.number).then(a.name.cmp(&b.name)));
        let rooms = rooms
            .into_iter()
            .map(|s| TypeRow {
                tab: None,
                id: s.id.0,
                badge: (!s.number.is_empty()).then(|| s.number.clone()),
                name: s.name.clone(),
                count: 1,
                here: vec![Element::Space(s.id)],
                value: Some(fmt_area(s.area())),
            })
            .collect();
        vec![
            ("Locales (planta)", Icon::Room, rooms),
            ("Muros", Icon::Wall, walls),
            (
                "Pilares",
                Icon::Column,
                members(archisim_core::MemberUsage::Column),
            ),
            (
                "Vigas",
                Icon::Beam,
                members(archisim_core::MemberUsage::Beam),
            ),
            ("Forjados", Icon::Slab, slabs),
            ("Cubiertas", Icon::Roof, roofs),
            ("Escaleras", Icon::Stair, stairs),
            ("Muros cortina", Icon::CurtainWall, curtains),
            ("Barandillas", Icon::Railing, railings),
            ("Objetos", Icon::Object, objects),
            ("Puertas", Icon::Door, openings(true)),
            ("Ventanas", Icon::Window, openings(false)),
        ]
    }

    fn nav_model(&mut self, ui: &mut Ui) {
        let p = Palette::current(ui.ctx());
        let categories = self.type_rows();
        let materials = self.ed.model().materials().count();
        let mut select: Option<(Vec<Element>, String)> = None;
        let mut edit: Option<(Option<TypesTab>, u64, Vec<Element>)> = None;
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
                            &r.value.clone().unwrap_or_else(|| r.count.to_string()),
                            false,
                            26.0,
                        );
                        let resp = resp.on_hover_text(format!(
                            "{} en el proyecto · {} en esta planta\nClic: seleccionarlos en la planta · doble clic: editar el tipo",
                            r.count,
                            r.here.len()
                        ));
                        if resp.double_clicked() {
                            edit = Some((r.tab, r.id, r.here.clone()));
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
                                edit = Some((r.tab, r.id, r.here.clone()));
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
        match edit {
            Some((Some(tab), id, _)) => self.edit_type(tab, id),
            Some((None, _, here)) => {
                // Columns and beams: their type is edited in Propiedades.
                self.set_tool(Tool::Arrow);
                self.show_plan();
                let n = here.len();
                self.ed.selection = here.into_iter().collect();
                self.say(if n == 0 {
                    "No hay elementos de este tipo en esta planta: elige la herramienta Pilar o Viga para editar el tipo en Propiedades.".to_string()
                } else {
                    format!("{n} seleccionados: la sección y el material del tipo están en Propiedades → Tipo.")
                });
            }
            None => {}
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
    let hint = if title == "Plantas" {
        "Añadir, renombrar, cambiar la cota o borrar plantas"
    } else {
        "Los cuatro alzados alrededor del edificio (los que falten)"
    };
    ui.horizontal(|ui| {
        ui.set_min_height(22.0);
        ui.add_space(10.0);
        ui.label(egui::RichText::new(title).small().color(p.muted));
        if let Some((text, clicked)) = button {
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.small_button(text).on_hover_text(hint).clicked() {
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
