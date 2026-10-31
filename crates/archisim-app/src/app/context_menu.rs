//! The menu of a single right click (concepto de interfaz del 24/10/2026),
//! the same in the plan and in 3D: on an element, what can be done with it
//! (properties, cut, copy, paste, duplicate, hide, isolate, select the
//! others of its type, take it to another level, delete); on nothing,
//! paste, select all and show what was hidden. Holding the right button
//! and dragging turns the 3D view instead, so the menu only opens on a
//! click.

use std::collections::BTreeSet;

use archisim_core::{LevelId, Point2};
use eframe::egui::{self, Pos2, RichText};

use super::*;

/// The menu open: where, on what, and the point of the model under the
/// cursor (where Pegar puts things).
#[derive(Debug, Clone)]
pub struct ContextMenu {
    pub pos: Pos2,
    pub element: Option<Element>,
    pub at: Option<Point2>,
    /// The frame it opened in: the click that opened it does not close it.
    opened: u64,
}

/// What Cortar or Copiar kept: the elements and the point they are taken
/// by (the middle of their outline). After Cortar, Pegar moves them; after
/// Copiar, it puts a copy.
#[derive(Debug, Clone, PartialEq)]
pub struct Clipboard {
    pub items: BTreeSet<Element>,
    pub base: Point2,
    pub cut: bool,
}

/// What the menu asked for.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MenuAct {
    Properties,
    Cut,
    Copy,
    Paste,
    Duplicate,
    Hide,
    Isolate,
    ShowAll,
    SameType,
    SelectAll,
    ToLevel(LevelId),
    Delete,
    /// Revit's «Permitir unión» / «No permitir unión» at one end of a wall.
    WallJoin {
        wall: archisim_core::WallId,
        start: bool,
        allow: bool,
    },
    /// The drafting elements selected in one group, or out of theirs.
    Group,
    Ungroup,
}

impl ContextMenu {
    /// A menu on nothing opened at a point of the plan.
    #[cfg(test)]
    pub fn at_for_tests(at: Point2) -> Self {
        Self {
            pos: Pos2::ZERO,
            element: None,
            at: Some(at),
            opened: 0,
        }
    }
}

impl ArchiSimApp {
    /// Opens the menu at `pos` for the element under the cursor (it joins
    /// the selection if it was not in it).
    pub(super) fn open_context_menu(
        &mut self,
        pos: Pos2,
        element: Option<Element>,
        at: Option<Point2>,
        frame: u64,
    ) {
        if let Some(e) = element {
            if !self.ed.selection.contains(&e) {
                self.ed.selection.clear();
                self.ed.selection.insert(e);
            }
        }
        self.context_menu = Some(ContextMenu {
            pos,
            element,
            at,
            opened: frame,
        });
    }

    /// Draws the menu open, if any, and does what is picked.
    pub(super) fn context_menu_ui(&mut self, ctx: &egui::Context) {
        let Some(menu) = self.context_menu.clone() else {
            return;
        };
        let frame = ctx.cumulative_frame_nr();
        let mut act = None;
        let area = egui::Area::new(egui::Id::unique("context-menu"))
            .order(egui::Order::Foreground)
            .fixed_pos(menu.pos)
            .constrain(true)
            .show(ctx, |ui| {
                egui::Frame::popup(ui.style()).show(ui, |ui| {
                    ui.set_min_width(230.0);
                    ui.spacing_mut().item_spacing.y = 0.0;
                    let w = &mut ui.style_mut().visuals.widgets.inactive;
                    w.weak_bg_fill = egui::Color32::TRANSPARENT;
                    w.bg_fill = egui::Color32::TRANSPARENT;
                    w.bg_stroke = egui::Stroke::NONE;
                    act = self.context_menu_items(ui, &menu);
                })
            });
        let outside = ctx.input(|i| {
            i.pointer.any_pressed()
                && i.pointer
                    .interact_pos()
                    .is_some_and(|p| !area.response.rect.contains(p))
        });
        let escape = ctx.input(|i| i.key_pressed(egui::Key::Escape));
        if act.is_some() || escape || (outside && frame > menu.opened) {
            self.context_menu = None;
        }
        if let Some(a) = act {
            self.menu_act(a, &menu);
        }
    }

    fn context_menu_items(&self, ui: &mut Ui, menu: &ContextMenu) -> Option<MenuAct> {
        let mut act = None;
        let item = |ui: &mut Ui, text: &str, hint: &str, enabled: bool| -> bool {
            let b = egui::Button::new(text)
                .shortcut_text(hint)
                .frame(false)
                .min_size(egui::vec2(220.0, 24.0));
            ui.add_enabled(enabled, b).clicked()
        };
        let model = self.ed.model();
        let paste = self.clipboard.is_some();
        match menu.element {
            Some(e) => {
                let n = self.ed.selection.len();
                let title = if n > 1 {
                    super::context_bar::selection_label(self.ed.selection.iter())
                } else {
                    self.menu_title(e)
                };
                ui.label(RichText::new(title).small().weak());
                ui.separator();
                if item(ui, "Propiedades…", "", true) {
                    act = Some(MenuAct::Properties);
                }
                ui.separator();
                if item(ui, "Cortar", "Ctrl X", true) {
                    act = Some(MenuAct::Cut);
                }
                if item(ui, "Copiar", "Ctrl C", true) {
                    act = Some(MenuAct::Copy);
                }
                if item(ui, "Pegar", "Ctrl V", paste) {
                    act = Some(MenuAct::Paste);
                }
                if item(ui, "Duplicar", "", true) {
                    act = Some(MenuAct::Duplicate);
                }
                ui.separator();
                if item(ui, "Ocultar", "", true) {
                    act = Some(MenuAct::Hide);
                }
                if item(ui, "Aislar", "", true) {
                    act = Some(MenuAct::Isolate);
                }
                if !self.ed.hidden().is_empty() && item(ui, "Mostrar todo", "", true) {
                    act = Some(MenuAct::ShowAll);
                }
                // Near an end of a wall: allow or disallow its join there.
                if let (Element::Wall(id), Some(at)) = (e, menu.at) {
                    if let Some((start, allowed)) = wall_end_near(model, id, at) {
                        let text = if allowed {
                            "No permitir unión en este extremo"
                        } else {
                            "Permitir unión en este extremo"
                        };
                        if item(ui, text, "", true) {
                            act = Some(MenuAct::WallJoin {
                                wall: id,
                                start,
                                allow: !allowed,
                            });
                        }
                    }
                }
                // Drafting: group what is selected, or take it apart.
                if matches!(e, Element::Draft(_)) {
                    let drafts: Vec<_> = self
                        .ed
                        .selection
                        .iter()
                        .filter_map(|x| match x {
                            Element::Draft(d) => model.draft(*d),
                            _ => None,
                        })
                        .collect();
                    let grouped = drafts.iter().any(|d| d.group.is_some());
                    if drafts.len() > 1 && item(ui, "Agrupar", "", true) {
                        act = Some(MenuAct::Group);
                    }
                    if grouped && item(ui, "Desagrupar", "", true) {
                        act = Some(MenuAct::Ungroup);
                    }
                }
                let typed = crate::pick::type_of(model, e).is_some();
                if item(ui, "Seleccionar todos los de este tipo", "", typed) {
                    act = Some(MenuAct::SameType);
                }
                let mut levels: Vec<_> = model
                    .levels()
                    .into_iter()
                    .map(|l| (l.id, l.name.clone(), l.elevation))
                    .collect();
                levels.sort_by(|a, b| b.2.total_cmp(&a.2));
                ui.menu_button("Llevar a la planta  ›", |ui| {
                    for (id, name, _) in &levels {
                        let here = *id == self.ed.level;
                        if ui.add_enabled(!here, egui::Button::new(name)).clicked() {
                            act = Some(MenuAct::ToLevel(*id));
                        }
                    }
                });
                ui.separator();
                let del = egui::Button::new(
                    RichText::new("Borrar").color(egui::Color32::from_rgb(194, 65, 45)),
                )
                .shortcut_text("Supr")
                .frame(false)
                .min_size(egui::vec2(220.0, 24.0));
                if ui.add(del).clicked() {
                    act = Some(MenuAct::Delete);
                }
            }
            None => {
                if item(ui, "Pegar", "Ctrl V", paste) {
                    act = Some(MenuAct::Paste);
                }
                if item(ui, "Seleccionar todo", "Ctrl A", true) {
                    act = Some(MenuAct::SelectAll);
                }
                if item(ui, "Mostrar todo", "", !self.ed.hidden().is_empty()) {
                    act = Some(MenuAct::ShowAll);
                }
            }
        }
        act
    }

    /// «Sofá 3 plazas · Objeto», «Fachada 30 · Muro».
    fn menu_title(&self, e: Element) -> String {
        let kind = super::context_bar::element_name(&e);
        let name = self.element_title(e);
        if name.is_empty() {
            kind.to_string()
        } else {
            format!("{name} · {kind}")
        }
    }

    /// The middle of the outlines of some elements, in plan.
    fn middle_of(&self, items: &BTreeSet<Element>) -> Option<Point2> {
        let pts: Vec<Point2> = items
            .iter()
            .filter_map(|e| crate::pick::outline(&self.ed, *e))
            .flatten()
            .collect();
        if pts.is_empty() {
            return None;
        }
        let (lo, hi) = pts.iter().fold((pts[0], pts[0]), |(lo, hi), p| {
            (
                Point2::new(lo.x.min(p.x), lo.y.min(p.y)),
                Point2::new(hi.x.max(p.x), hi.y.max(p.y)),
            )
        });
        Some(Point2::new((lo.x + hi.x) / 2.0, (lo.y + hi.y) / 2.0))
    }

    /// Keeps the selection to paste it later (moved, after Cortar).
    pub(super) fn copy_selection(&mut self, cut: bool) {
        let items = self.ed.selection.clone();
        let Some(base) = self.middle_of(&items) else {
            return;
        };
        let n = items.len();
        self.clipboard = Some(Clipboard { items, base, cut });
        self.say(if cut {
            format!("Cortado ({n}): clic derecho → Pegar donde vaya.")
        } else {
            format!("Copiado ({n}): clic derecho → Pegar donde vaya.")
        });
    }

    /// Puts what was copied (or moves what was cut) with its middle at
    /// `at` (else, 50 cm aside), on the level shown.
    pub(super) fn paste_at(&mut self, at: Option<Point2>) {
        let Some(c) = self.clipboard.clone() else {
            return;
        };
        let items: BTreeSet<Element> = c
            .items
            .iter()
            .copied()
            .filter(|e| crate::drag::exists(self.ed.model(), *e))
            .collect();
        if items.is_empty() {
            self.clipboard = None;
            self.complain("Lo copiado ya no está en el proyecto.");
            return;
        }
        let to = at.unwrap_or(Point2::new(c.base.x + 0.5, c.base.y - 0.5));
        let t = archisim_draft::Transform::translate(to - c.base);
        let level = self.ed.level;
        let list: Vec<Element> = items.iter().copied().collect();
        let res = self
            .ed
            .transact(if c.cut { "Pegar (mover)" } else { "Pegar" }, |m| {
                let out = crate::xform::transform(m, &items, &t, !c.cut)?;
                let _ = list;
                Ok(out)
            });
        match res {
            Ok(sel) => {
                self.ed.selection = sel;
                let moved: Vec<Element> = self.ed.selection.iter().copied().collect();
                // On the level shown, if it was copied from another one.
                let _ = self.ed.set_elements_level(&moved, level);
                if c.cut {
                    self.clipboard = None;
                } else {
                    self.clipboard = Some(Clipboard { base: to, ..c });
                }
                self.say("Pegado.");
            }
            Err(e) => self.complain(format!("No se puede pegar: {e}.")),
        }
    }

    pub(super) fn menu_act(&mut self, act: MenuAct, menu: &ContextMenu) {
        match act {
            MenuAct::Properties => {
                if self.maquette {
                    self.maquette_props = true;
                } else {
                    self.show_properties = true;
                }
            }
            MenuAct::Cut => self.copy_selection(true),
            MenuAct::Copy => self.copy_selection(false),
            MenuAct::Paste => self.paste_at(menu.at),
            MenuAct::Duplicate => self.card_duplicate_pub(),
            MenuAct::Hide => {
                let n = self.ed.selection.len();
                let items: Vec<Element> = self.ed.selection.iter().copied().collect();
                self.ed.hide(items);
                self.say(format!(
                    "Ocultado ({n}). Clic derecho en el vacío → Mostrar todo."
                ));
            }
            MenuAct::Isolate => {
                let keep = self.ed.selection.clone();
                let model = self.ed.model();
                let others: Vec<Element> = model
                    .levels()
                    .into_iter()
                    .flat_map(|l| crate::pick::elements_on_level(model, l.id))
                    .filter(|e| !keep.contains(e) && !matches!(e, Element::Opening(_)))
                    .collect();
                self.ed.hide(others);
                self.ed.selection = keep;
                self.say("Aislado. Clic derecho en el vacío → Mostrar todo.");
            }
            MenuAct::ShowAll => {
                self.ed.show_all();
                self.say("Se ve todo otra vez.");
            }
            MenuAct::SameType => {
                if let Some(e) = menu.element {
                    let found = self.same_type(e);
                    let what = plural(super::context_bar::element_name(&e));
                    self.select_instances(found, &what);
                }
            }
            MenuAct::SelectAll => self.select_all_pub(),
            MenuAct::ToLevel(level) => {
                let items: Vec<Element> = self.ed.selection.iter().copied().collect();
                let name = self
                    .ed
                    .model()
                    .level(level)
                    .map(|l| l.name.clone())
                    .unwrap_or_default();
                match self.ed.set_elements_level(&items, level) {
                    Ok(0) => self.complain("Eso no puede cambiar de planta."),
                    Ok(n) => self.say(format!("{n} elemento(s) llevados a {name}.")),
                    Err(e) => self.complain(format!("No se puede: {e}.")),
                }
            }
            MenuAct::Delete => self.delete_pub(),
            MenuAct::Group | MenuAct::Ungroup => {
                let ids: Vec<archisim_core::DraftId> = self
                    .ed
                    .selection
                    .iter()
                    .filter_map(|x| match x {
                        Element::Draft(d) => Some(*d),
                        _ => None,
                    })
                    .collect();
                let r = if act == MenuAct::Group {
                    self.ed
                        .transact("Agrupar", |m| m.group_drafts(&ids, "Grupo").map(|_| ()))
                } else {
                    self.ed.transact("Desagrupar", |m| m.ungroup_drafts(&ids))
                };
                match r {
                    Ok(()) => {
                        if act == MenuAct::Ungroup {
                            // Its lines stay selected, each on its own.
                            self.ed.selection.clear();
                        }
                        self.say(if act == MenuAct::Group {
                            "Agrupado: un clic elige el grupo entero."
                        } else {
                            "Desagrupado: cada línea va ya por su cuenta."
                        })
                    }
                    Err(e) => self.complain(format!("No se puede: {e}.")),
                }
            }
            MenuAct::WallJoin { wall, start, allow } => {
                let label = if allow {
                    "Permitir unión"
                } else {
                    "No permitir unión"
                };
                match self
                    .ed
                    .transact(label, |m| m.set_wall_join(wall, start, allow))
                {
                    Ok(()) => self.say(if allow {
                        "Unión permitida: el extremo vuelve a unirse con los muros que toca."
                    } else {
                        "Unión no permitida: el extremo queda libre aunque toque otro muro."
                    }),
                    Err(e) => self.complain(format!("No se puede: {e}.")),
                }
            }
        }
    }
}

/// The end of a wall within a metre of `at` (the nearer one), and whether
/// it may join now.
pub(super) fn wall_end_near(
    model: &archisim_core::Model,
    wall: archisim_core::WallId,
    at: Point2,
) -> Option<(bool, bool)> {
    let w = model.wall(wall)?;
    let (ds, de) = (w.start.distance(at), w.end.distance(at));
    let start = ds <= de;
    let reach = 1.0_f64.min(w.length() / 3.0);
    (ds.min(de) <= reach).then(|| (start, w.allow_join[if start { 0 } else { 1 }]))
}

/// «Muro» → «muros», «Pilar» → «pilares».
fn plural(name: &str) -> String {
    let n = name.to_lowercase();
    match n.chars().last() {
        Some('a' | 'e' | 'i' | 'o' | 'u') => format!("{n}s"),
        _ => format!("{n}es"),
    }
}
