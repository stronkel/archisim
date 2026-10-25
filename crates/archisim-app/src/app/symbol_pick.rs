//! Choosing a plan symbol among the drawings of a DXF that is a library
//! (A1, pedido por Antonio el 25/10/2026): a sheet of trees, people or
//! cars, one block each. Each one is shown as the plan would draw it on
//! the object's type, to pick one. The files stay in the user's own
//! library folder («Símbolos de planta»): they are often someone else's
//! drawings, kept on the user's computer, never in ArchiSim.

use std::path::{Path, PathBuf};

use archisim_core::ObjectTypeId;
use archisim_export::symbol::LibrarySymbol;
use eframe::egui::{self, Vec2};

use super::ArchiSimApp;

/// The subfolder of the library with the DXF drawings of symbols.
pub const FOLDER: &str = "Símbolos de planta";

/// Pictures made each frame at most (the rest come in the next ones).
const NEW_PER_FRAME: usize = 8;

/// Side of each picture of the choice, in points.
const TILE: f32 = 96.0;

/// The drawings of one DXF.
pub struct SymbolSet {
    /// The file's name.
    pub file: String,
    pub symbols: Vec<LibrarySymbol>,
}

/// The choice open: for which type, stretched to it or not, the
/// drawings to choose from and which file is shown.
pub struct SymbolPicker {
    pub target: ObjectTypeId,
    pub fit: bool,
    pub sets: Vec<SymbolSet>,
    pub shown: usize,
}

/// The folder of plan symbols in the user's library.
pub fn library_folder() -> PathBuf {
    crate::editor::library::library_dir().join(FOLDER)
}

/// The DXF files of the library's plan symbols, by name.
pub fn library_files() -> Vec<PathBuf> {
    let mut files: Vec<PathBuf> = std::fs::read_dir(library_folder())
        .into_iter()
        .flatten()
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|x| x.eq_ignore_ascii_case("dxf")))
        .collect();
    files.sort();
    files
}

/// Whether the library has drawings to choose from: looked at again
/// every few seconds, not at every frame (the folder may be on a shared
/// drive).
pub fn has_library_files() -> bool {
    use std::cell::Cell;
    use std::time::{Duration, Instant};
    thread_local! {
        static SEEN: Cell<Option<(Instant, bool)>> = const { Cell::new(None) };
    }
    SEEN.with(|c| match c.get() {
        Some((at, has)) if at.elapsed() < Duration::from_secs(5) => has,
        _ => {
            let has = !library_files().is_empty();
            c.set(Some((Instant::now(), has)));
            has
        }
    })
}

/// Keeps a copy of a library DXF in the user's library folder (unless
/// it is already there), so it can be chosen from later without looking
/// for the file. The tests never write there.
pub fn keep_in_library(path: &Path) -> Option<PathBuf> {
    if cfg!(test) {
        return None;
    }
    let dir = library_folder();
    let name = path.file_name()?;
    let to = dir.join(name);
    if path.parent() == Some(dir.as_path()) || to.exists() {
        return None;
    }
    std::fs::create_dir_all(&dir).ok()?;
    std::fs::copy(path, &to).ok()?;
    Some(to)
}

/// The library drawings of a file (none if it is one drawing).
pub fn read_set(path: &Path, fit: bool) -> Result<SymbolSet, String> {
    let bytes = std::fs::read(path).map_err(|e| e.to_string())?;
    let file = path.file_name().map_or_else(
        || path.display().to_string(),
        |n| n.to_string_lossy().into_owned(),
    );
    let symbols = archisim_export::symbol::symbols_from_dxf(&bytes, &file, fit)?;
    Ok(SymbolSet { file, symbols })
}

impl ArchiSimApp {
    /// Opens the choice among the drawings of the library folder.
    pub(super) fn pick_from_library(&mut self, t: ObjectTypeId) {
        let fit = self.symbol_fits(t);
        let sets: Vec<SymbolSet> = library_files()
            .iter()
            .filter_map(|p| read_set(p, fit).ok())
            .filter(|s| !s.symbols.is_empty())
            .collect();
        if sets.is_empty() {
            self.complain(format!(
                "No hay dibujos en la carpeta «{FOLDER}» de tu biblioteca. Importa un DXF con varios bloques y se guardará allí."
            ));
            return;
        }
        self.symbol_picker = Some(SymbolPicker {
            target: t,
            fit,
            sets,
            shown: 0,
        });
    }

    /// The window to choose a drawing, while it is open.
    pub(super) fn symbol_pick_window(&mut self, ctx: &egui::Context) {
        let Some(picker) = self.symbol_picker.as_ref() else {
            return;
        };
        let target = picker.target;
        let Some(ty) = self.ed.model().object_type(target).cloned() else {
            self.symbol_picker = None;
            return;
        };
        let mut open = true;
        let mut chosen: Option<(usize, usize)> = None;
        let mut shown = picker.shown;
        let p = super::theme::Palette::current(ctx);
        let mut new = 0usize;
        let mut waiting = false;
        super::widgets::dialog(format!("Símbolo de planta · {}", ty.name))
            .open(&mut open)
            .default_width(620.0)
            .default_height(560.0)
            .show(ctx, |ui| {
                let Some(picker) = self.symbol_picker.as_ref() else {
                    return;
                };
                if picker.sets.len() > 1 {
                    ui.horizontal_wrapped(|ui| {
                        for (i, set) in picker.sets.iter().enumerate() {
                            if ui.selectable_label(shown == i, &set.file).clicked() {
                                shown = i;
                            }
                        }
                    });
                    ui.separator();
                }
                let Some(set) = picker.sets.get(shown) else {
                    return;
                };
                ui.label(
                    egui::RichText::new(format!(
                        "{} dibujos de {}. Se ven como los dibujará la planta; haz clic en uno para usarlo.",
                        set.symbols.len(),
                        set.file
                    ))
                    .small()
                    .color(p.muted),
                );
                ui.add_space(4.0);
                let fit = picker.fit;
                let file = set.file.clone();
                let symbols: Vec<(usize, archisim_core::PlanSymbol, f64, String)> = set
                    .symbols
                    .iter()
                    .map(|s| (s.number, s.symbol.clone(), s.size, s.block.clone()))
                    .collect();
                egui::ScrollArea::vertical().show(ui, |ui| {
                    let per_row = ((ui.available_width() / (TILE + 10.0)).floor() as usize).max(1);
                    for (row_i, row) in symbols.chunks(per_row).enumerate() {
                        ui.horizontal(|ui| {
                            for (k, (number, symbol, size, block)) in row.iter().enumerate() {
                                let key = format!("symlib:{file}:{number}:{fit}:{}", target.0);
                                let ready = self.thumbs.has(&key);
                                let thumb = if ready || new < NEW_PER_FRAME {
                                    if !ready {
                                        new += 1;
                                    }
                                    let model = self.ed.model();
                                    let mut candidate = ty.clone();
                                    candidate.symbol = Some(symbol.clone());
                                    candidate.plan_image = None;
                                    self.thumbs.get(ctx, key, || {
                                        crate::type_preview::object_model(model, &candidate)
                                    })
                                } else {
                                    waiting = true;
                                    None
                                };
                                let (rect, resp) = ui.allocate_exact_size(
                                    Vec2::new(TILE, TILE + 16.0),
                                    egui::Sense::click(),
                                );
                                let pic = egui::Rect::from_min_size(rect.min, Vec2::splat(TILE));
                                let painter = ui.painter();
                                painter.rect_filled(pic, 6.0, egui::Color32::WHITE);
                                painter.rect_stroke(
                                    pic,
                                    6.0,
                                    egui::Stroke::new(
                                        if resp.hovered() { 2.0 } else { 1.0 },
                                        if resp.hovered() { p.accent } else { p.line },
                                    ),
                                    egui::StrokeKind::Inside,
                                );
                                if let Some(t) = thumb {
                                    let s = t.fit(Vec2::splat(TILE - 8.0));
                                    painter.image(
                                        t.tex,
                                        egui::Rect::from_center_size(pic.center(), s),
                                        egui::Rect::from_min_max(
                                            egui::Pos2::ZERO,
                                            egui::Pos2::new(1.0, 1.0),
                                        ),
                                        egui::Color32::WHITE,
                                    );
                                }
                                painter.text(
                                    egui::Pos2::new(pic.center().x, pic.bottom() + 8.0),
                                    egui::Align2::CENTER_CENTER,
                                    if fit {
                                        number.to_string()
                                    } else {
                                        format!("{number} · {:.2} m", size)
                                    },
                                    egui::FontId::proportional(11.0),
                                    p.muted,
                                );
                                if resp.on_hover_text(block.as_str()).clicked() {
                                    chosen = Some((shown, row_i * per_row + k));
                                }
                            }
                        });
                        ui.add_space(4.0);
                    }
                });
            });
        if waiting {
            ctx.request_repaint();
        }
        if let Some(picker) = self.symbol_picker.as_mut() {
            picker.shown = shown;
        }
        if let Some((s, i)) = chosen {
            self.use_library_symbol(s, i);
            return;
        }
        if !open {
            self.symbol_picker = None;
        }
    }

    /// Gives the type the drawing `i` of the set `s` of the open choice.
    pub(super) fn use_library_symbol(&mut self, s: usize, i: usize) {
        let Some(picker) = self.symbol_picker.take() else {
            return;
        };
        let Some(lib) = picker.sets.get(s).and_then(|set| set.symbols.get(i)) else {
            return;
        };
        let t = picker.target;
        let symbol = lib.symbol.clone();
        let source = symbol.source.clone();
        let r = self.ed.transact("Símbolo de planta", |m| {
            m.set_object_type_symbol(t, Some(symbol))
        });
        self.report(r, &format!("Símbolo de planta: dibujo {source}."));
    }
}
