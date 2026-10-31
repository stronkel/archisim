//! Keyboard shortcuts as in Revit, the industry's standard (pedido por
//! Antonio el 26/10/2026): two letters typed one after the other start the
//! tool or the order at once, without Enter (WA muro, DR puerta, MV mover,
//! MM reflejar, AL alinear, TR esquina…). The codes are Revit's own where
//! Revit has one; a few that Revit lacks are ArchiSim's, marked as such in
//! the list. With Revit keys on, the letters are Revit's: the AutoCAD names
//! typed with Enter (LINEA, REC…) need «Teclas: AutoCAD».

use eframe::egui;

use super::*;

/// What a shortcut does.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum KeyAct {
    Tool(Tool),
    Command(&'static str),
    LevelMark,
    ZoomAll,
    ThinLines,
    HideSelection,
    ShowAll,
    Shortcuts,
}

/// A shortcut: its two letters, what it does, its name in the list and
/// whether Revit has it with the same letters.
pub struct Shortcut {
    pub code: &'static str,
    pub act: KeyAct,
    pub name: &'static str,
    pub revit: bool,
}

const fn key(code: &'static str, act: KeyAct, name: &'static str, revit: bool) -> Shortcut {
    Shortcut {
        code,
        act,
        name,
        revit,
    }
}

/// Every shortcut, grouped as in the list.
pub const SHORTCUTS: &[(&str, &[Shortcut])] = &[
    (
        "Arquitectura",
        &[
            key("WA", KeyAct::Tool(Tool::Wall), "Muro", true),
            key("DR", KeyAct::Tool(Tool::Door), "Puerta", true),
            key("WN", KeyAct::Tool(Tool::Window), "Ventana", true),
            key(
                "CM",
                KeyAct::Tool(Tool::Object),
                "Componente (objeto)",
                true,
            ),
            key("SB", KeyAct::Tool(Tool::Slab), "Suelo / forjado", true),
            key("RF", KeyAct::Tool(Tool::Roof), "Cubierta", false),
            key("ST", KeyAct::Tool(Tool::Stair), "Escalera", false),
            key("RA", KeyAct::Tool(Tool::Railing), "Barandilla", false),
            key("CW", KeyAct::Tool(Tool::CurtainWall), "Muro cortina", false),
            key("RM", KeyAct::Command("LOCAL"), "Habitación (local)", true),
            key("PT", KeyAct::Tool(Tool::Paint), "Pintar", true),
            key("SF", KeyAct::Tool(Tool::DivideFace), "Dividir cara", true),
        ],
    ),
    (
        "Estructura y referencias",
        &[
            key("CL", KeyAct::Tool(Tool::Column), "Pilar", true),
            key("BM", KeyAct::Tool(Tool::Beam), "Viga", true),
            key("GR", KeyAct::Tool(Tool::Grid), "Rejilla (eje)", true),
            key(
                "RP",
                KeyAct::Tool(Tool::RefPlane),
                "Plano de referencia",
                true,
            ),
            key("FT", KeyAct::Tool(Tool::Footing), "Cimentación", false),
        ],
    ),
    (
        "Anotar",
        &[
            key("DI", KeyAct::Tool(Tool::Dimension), "Cota alineada", true),
            key("EL", KeyAct::LevelMark, "Cota de nivel", true),
            key("TX", KeyAct::Command("TEXTO"), "Texto", true),
            key("TG", KeyAct::Command("ETIQUETA"), "Etiquetar", true),
            key(
                "RT",
                KeyAct::Command("ETIQUETA"),
                "Etiqueta de habitación",
                true,
            ),
            key("DL", KeyAct::Command("LINEA"), "Línea de detalle", true),
            key("LI", KeyAct::Command("LINEA"), "Línea de modelo", true),
            key("SE", KeyAct::Tool(Tool::Section), "Sección", false),
        ],
    ),
    (
        "Modificar",
        &[
            key("MV", KeyAct::Command("DESPLAZA"), "Mover", true),
            key("CO", KeyAct::Command("COPIA"), "Copiar", true),
            key("CC", KeyAct::Command("COPIA"), "Copiar", true),
            key("RO", KeyAct::Command("GIRA"), "Rotar", true),
            key("RE", KeyAct::Command("ESCALA"), "Escala", true),
            key(
                "MM",
                KeyAct::Command("REFLEJAR"),
                "Reflejar - elegir eje",
                true,
            ),
            key(
                "DM",
                KeyAct::Command("REFLEJARDIB"),
                "Reflejar - dibujar eje",
                true,
            ),
            key("AR", KeyAct::Command("MATRIZ"), "Matriz", true),
            key("AL", KeyAct::Command("ALINEACARA"), "Alinear", true),
            key(
                "TR",
                KeyAct::Command("ESQUINA"),
                "Recortar/extender para formar esquina",
                true,
            ),
            key("OF", KeyAct::Command("EQUIDIST"), "Desfase", true),
            key(
                "MA",
                KeyAct::Command("IGUALARPROP"),
                "Igualar propiedades",
                true,
            ),
            key("DE", KeyAct::Command("BORRA"), "Borrar", true),
        ],
    ),
    (
        "Vista",
        &[
            key("ZA", KeyAct::ZoomAll, "Zoom ajustar todo", true),
            key("ZE", KeyAct::ZoomAll, "Zoom extensión", true),
            key("ZF", KeyAct::ZoomAll, "Zoom ajustar", true),
            key("TL", KeyAct::ThinLines, "Líneas finas", true),
            key("HH", KeyAct::HideSelection, "Ocultar temporalmente", true),
            key("EH", KeyAct::HideSelection, "Ocultar elemento", true),
            key("HR", KeyAct::ShowAll, "Restablecer ocultos", true),
            key("KS", KeyAct::Shortcuts, "Lista de atajos", true),
        ],
    ),
];

/// The other keys of the program, for the list.
pub const OTHER_KEYS: &[(&str, &str)] = &[
    (
        "Tab / Mayús+Tab",
        "Elemento siguiente / anterior bajo el cursor (clic para elegirlo)",
    ),
    ("Esc", "Terminar la orden; otra vez: soltar la selección"),
    ("Supr", "Borrar lo seleccionado"),
    (
        "Espacio",
        "Girar la puerta o ventana que se coloca, o el objeto 90°",
    ),
    ("Intro", "Repetir la última orden"),
    ("F2 / F3 / F4", "Planta / 3D / Técnica ↔ Maqueta"),
    ("F8", "Orto"),
    (
        "Ctrl+Z / Ctrl+Y",
        "Deshacer / rehacer (también U dentro de una orden)",
    ),
    ("Ctrl+C / X / V", "Copiar / cortar / pegar"),
    ("Ctrl+A", "Seleccionar todo"),
    ("Ctrl+S", "Guardar"),
    ("Ctrl+Espacio", "Buscar una orden por su nombre"),
    ("Ctrl+Tab", "Pestaña de vista siguiente"),
    ("Inicio", "Encuadrar todo"),
];

/// The shortcut for two letters, if any.
pub fn find(code: &str) -> Option<&'static Shortcut> {
    let code = code.to_uppercase();
    SHORTCUTS
        .iter()
        .flat_map(|(_, list)| list.iter())
        .find(|s| s.code == code)
}

impl ArchiSimApp {
    /// Runs a Revit shortcut; returns whether `code` was one.
    pub(super) fn revit_key(&mut self, code: &str) -> bool {
        let Some(s) = find(code) else {
            return false;
        };
        if self.tool == Tool::Cad {
            self.end_command(true);
        }
        match s.act {
            KeyAct::Tool(t) => self.pick(toolbox::Pick::Tool(t)),
            KeyAct::Command(c) => self.start_command(c),
            KeyAct::LevelMark => self.pick(toolbox::Pick::LevelMark),
            KeyAct::ZoomAll => self.fit_active_view(),
            KeyAct::ThinLines => {
                self.thin_lines = !self.thin_lines;
                self.say(if self.thin_lines {
                    "Líneas finas"
                } else {
                    "Líneas con su grosor"
                });
            }
            KeyAct::HideSelection => {
                let items: Vec<Element> = self.ed.selection.iter().copied().collect();
                if !items.is_empty() {
                    let n = items.len();
                    self.ed.hide(items);
                    self.say(format!("Ocultado ({n}). HR para verlo todo otra vez."));
                }
            }
            KeyAct::ShowAll => {
                self.ed.show_all();
                self.say("Se ve todo otra vez.");
            }
            KeyAct::Shortcuts => self.show_shortcuts = true,
        }
        self.cmdline.log(format!("{} · {}", s.code, s.name), false);
        true
    }

    /// The list of shortcuts (KS, or Vista → Atajos de teclado).
    pub(super) fn shortcuts_dialog(&mut self, ctx: &egui::Context) {
        if !self.show_shortcuts {
            return;
        }
        let mut open = true;
        super::widgets::dialog("Atajos de teclado")
            .open(&mut open)
            .pivot(egui::Align2::CENTER_CENTER)
            .default_pos(ctx.content_rect().center())
            .default_size([620.0, 560.0])
            .show(ctx, |ui| {
                ui.label(if self.keys_revit {
                    "Teclas de dos letras: escribe las dos letras y la herramienta empieza, sin Intro. Para escribir las órdenes de dibujo (LINEA, REC…) con Intro, Vista → Teclas: Órdenes."
                } else {
                    "Las teclas de dos letras están apagadas: actívalas en Vista → Teclas: Dos letras."
                });
                ui.add_space(6.0);
                egui::ScrollArea::vertical().show(ui, |ui| {
                    for (group, list) in SHORTCUTS {
                        ui.label(egui::RichText::new(*group).strong());
                        egui::Grid::new(("atajos", *group))
                            .num_columns(3)
                            .spacing([16.0, 2.0])
                            .show(ui, |ui| {
                                for s in *list {
                                    ui.label(egui::RichText::new(s.code).monospace().strong());
                                    ui.label(s.name);
                                    ui.label(
                                        egui::RichText::new(if s.revit { "" } else { "propio de ArchiSim" })
                                            .small()
                                            .weak(),
                                    );
                                    ui.end_row();
                                }
                            });
                        ui.add_space(6.0);
                    }
                    for (title, list) in [("Otras teclas", OTHER_KEYS), ("Ratón y dibujo", super::MOUSE_KEYS)] {
                        ui.label(egui::RichText::new(title).strong());
                        egui::Grid::new(("atajos-otras", title))
                            .num_columns(2)
                            .spacing([16.0, 2.0])
                            .show(ui, |ui| {
                                for (k, what) in list {
                                    ui.label(egui::RichText::new(*k).monospace().strong());
                                    ui.label(*what);
                                    ui.end_row();
                                }
                            });
                        ui.add_space(6.0);
                    }
                });
            });
        if !open {
            self.show_shortcuts = false;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every code is two letters (or digits) and appears once.
    #[test]
    fn codes_are_two_characters_and_unique() {
        let mut seen = std::collections::BTreeSet::new();
        for (_, list) in SHORTCUTS {
            for s in *list {
                assert_eq!(s.code.chars().count(), 2, "{}", s.code);
                assert!(seen.insert(s.code), "{} twice", s.code);
                if let KeyAct::Command(c) = s.act {
                    assert!(crate::cad::new_command(c).is_some(), "{c}");
                }
            }
        }
        assert!(find("wa").is_some());
        assert!(find("XX").is_none());
    }
}
