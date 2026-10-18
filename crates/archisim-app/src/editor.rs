//! The open project and everything done to it, without any user interface:
//! the document with its undo history, the file it lives in, the active
//! level and the selection. Every edit is one undoable step with a Spanish
//! name, as it appears in the Edit menu.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use archisim_core::{
    DimensionId, Document, ElevationMarkId, Hand, LevelId, Model, ModelError, NewDimension,
    NewOpening, NewSlab, NewWall, OpeningId, OpeningTypeId, Point2, ReferenceLine, SlabId,
    SlabTypeId, WallId, WallSide, WallTop, WallTypeId,
};
use archisim_ifc::{export_ifc, import_ifc, ExportOptions, ImportReport};
use archisim_plan::{render_plan, PlanOptions};

use crate::pick::Element;
use crate::units::{explain, explain_file};
use crate::wall_tool::Piece;

mod types;

/// Settings of the wall tool (ArchiCAD "Cuadro de información").
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WallSettings {
    pub wall_type: WallTypeId,
    pub reference: ReferenceLine,
    /// Height of walls without a top level (or with no level above).
    pub height: f64,
    pub base_offset: f64,
    /// Where new walls end at the top.
    pub top: TopChoice,
    /// Offset from the top level (negative: below it).
    pub top_offset: f64,
}

/// The top constraint of the wall tool: a level relative to the one being
/// drawn on (ArchiCAD's "planta inicial + 1"), a given level, or none.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TopChoice {
    /// Own height (Revit's "Desconectada").
    Unconnected,
    /// The level right above the active one; with none above, the wall
    /// gets its own height.
    NextLevel,
    /// A given level (it must be above the active one).
    Level(LevelId),
}

impl WallSettings {
    /// The constraint a new wall on `level` gets.
    pub fn top_on(&self, model: &Model, level: LevelId) -> WallTop {
        let target = match self.top {
            TopChoice::Unconnected => None,
            TopChoice::NextLevel => next_level_above(model, level),
            TopChoice::Level(l) => Some(l),
        };
        match target {
            Some(l) => WallTop::Level {
                level: l,
                offset: self.top_offset,
            },
            None => WallTop::Unconnected,
        }
    }
}

/// The lowest level above `level`, if any.
pub fn next_level_above(model: &Model, level: LevelId) -> Option<LevelId> {
    model.level_above(level)
}

pub struct Editor {
    doc: Document,
    /// Where the project was opened from or last saved to.
    pub path: Option<PathBuf>,
    /// The IFC file the project was imported from, until it is saved: it
    /// suggests the folder and name to save it with.
    pub imported_from: Option<PathBuf>,
    /// The model as last saved (or opened), to know if there are changes.
    saved: Model,
    /// Grows with every change to the model, so views know when to redraw.
    revision: u64,
    /// The level shown in the plan and where new elements go.
    pub level: LevelId,
    pub selection: BTreeSet<Element>,
}

/// Result of an edit: `Err` carries a Spanish explanation.
pub type EditResult<T = ()> = Result<T, String>;

impl Editor {
    /// A new project from the starter library.
    pub fn new_project() -> Self {
        Self::with_model(archisim_library::new_project("Proyecto sin título"), None)
    }

    pub fn with_model(model: Model, path: Option<PathBuf>) -> Self {
        let level = model
            .levels()
            .iter()
            .min_by(|a, b| a.elevation.abs().total_cmp(&b.elevation.abs()))
            .map(|l| l.id)
            .unwrap_or(LevelId(0));
        Self {
            saved: model.clone(),
            doc: Document::new(model),
            path,
            imported_from: None,
            revision: 0,
            level,
            selection: BTreeSet::new(),
        }
    }

    /// Opens a `.archisim` project.
    pub fn open(path: &Path) -> EditResult<Self> {
        let model = archisim_file::open(path).map_err(|e| explain_file(&e))?;
        Ok(Self::with_model(model, Some(path.to_path_buf())))
    }

    /// Imports an IFC file as a new, unsaved project. When the file brings
    /// no wall, slab, door or window types of some kind, the ArchiSim
    /// starter library is added so every tool has something to draw with.
    /// Returns the report of what was imported and whether the library was
    /// added.
    pub fn import(path: &Path) -> EditResult<(Self, ImportReport, bool)> {
        let bytes = std::fs::read(path).map_err(|e| format!("no se puede leer el fichero: {e}"))?;
        let text = String::from_utf8_lossy(&bytes);
        let imported = import_ifc(&text).map_err(|e| e.to_string())?;
        let mut model = imported.model;
        let lacks_types = model.wall_types().next().is_none()
            || model.slab_types().next().is_none()
            || !model.opening_types().any(|t| t.is_door())
            || !model.opening_types().any(|t| t.is_window());
        if lacks_types {
            archisim_library::add_library(&mut model).map_err(|e| explain(&e))?;
        }
        let mut ed = Self::with_model(model, None);
        // Not saved anywhere yet: closing asks first.
        ed.saved = Model::new("");
        ed.imported_from = Some(path.to_path_buf());
        Ok((ed, imported.report, lacks_types))
    }

    pub fn model(&self) -> &Model {
        self.doc.model()
    }

    pub fn revision(&self) -> u64 {
        self.revision
    }

    /// Whether there are changes since the project was opened or saved.
    pub fn is_modified(&self) -> bool {
        self.doc.model() != &self.saved
    }

    /// File name to show in the title bar.
    pub fn display_name(&self) -> String {
        self.path
            .as_ref()
            .and_then(|p| p.file_name())
            .map(|n| n.to_string_lossy().into_owned())
            .or_else(|| {
                self.imported_from
                    .as_ref()
                    .and_then(|p| p.file_name())
                    .map(|n| format!("{} (importado)", n.to_string_lossy()))
            })
            .unwrap_or_else(|| "Sin título".into())
    }

    /// Saves to `path` (or to the current file) and remembers it.
    pub fn save(&mut self, path: Option<&Path>) -> EditResult<PathBuf> {
        let mut target = match path.or(self.path.as_deref()) {
            Some(p) => p.to_path_buf(),
            None => return Err("el proyecto todavía no tiene fichero".into()),
        };
        if target.extension().is_none() {
            target.set_extension(archisim_file::EXTENSION);
        }
        archisim_file::save(self.model(), &target).map_err(|e| explain_file(&e))?;
        self.saved = self.model().clone();
        self.path = Some(target.clone());
        self.imported_from = None;
        Ok(target)
    }

    /// Writes the IFC4 file of the whole project.
    pub fn export_ifc(&self, path: &Path) -> EditResult {
        let file_name = path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| "proyecto.ifc".into());
        let text = export_ifc(
            self.model(),
            &ExportOptions {
                file_name,
                author: "ArchiSim".into(),
                organization: String::new(),
                timestamp: None,
            },
        );
        std::fs::write(path, text).map_err(|e| format!("no se pudo escribir el IFC: {e}"))
    }

    /// Writes the plan of the active level as SVG at a scale (50 for
    /// 1:50): line weights are paper weights, and doors and windows use the
    /// plan symbol their family gives for that scale.
    pub fn export_plan(&self, path: &Path, show_reference_lines: bool, scale: f64) -> EditResult {
        let svg = render_plan(
            self.model(),
            self.level,
            &PlanOptions {
                show_reference_lines,
                scale,
                ..PlanOptions::default()
            },
        );
        std::fs::write(path, svg).map_err(|e| format!("no se pudo escribir la planta: {e}"))
    }

    // ---- History ---------------------------------------------------------

    fn changed(&mut self) {
        self.revision += 1;
        let model = self.doc.model();
        self.selection.retain(|e| match e {
            Element::Wall(id) => model.wall(*id).is_some(),
            Element::Opening(id) => model.opening(*id).is_some(),
            Element::Slab(id) => model.slab(*id).is_some(),
            Element::Dimension(id) => model.dimension(*id).is_some(),
            Element::Mark(id) => model.elevation_mark(*id).is_some(),
        });
        if model.level(self.level).is_none() {
            if let Some(l) = model.levels().first() {
                self.level = l.id;
            }
        }
    }

    fn transact<T>(
        &mut self,
        label: &str,
        edit: impl FnOnce(&mut Model) -> Result<T, ModelError>,
    ) -> EditResult<T> {
        let result = self.doc.transact(label, edit).map_err(|e| explain(&e))?;
        self.changed();
        Ok(result)
    }

    pub fn undo_label(&self) -> Option<&str> {
        self.doc.undo_label()
    }

    pub fn redo_label(&self) -> Option<&str> {
        self.doc.redo_label()
    }

    pub fn undo(&mut self) -> Option<String> {
        let label = self.doc.undo()?;
        self.changed();
        Some(label)
    }

    pub fn redo(&mut self) -> Option<String> {
        let label = self.doc.redo()?;
        self.changed();
        Some(label)
    }

    // ---- Drawing and editing ------------------------------------------------

    #[cfg(test)]
    pub fn add_wall(&mut self, s: &WallSettings, start: Point2, end: Point2) -> EditResult<WallId> {
        let ids = self.add_walls(s, &[Piece::straight(start, end)])?;
        Ok(ids[0])
    }

    /// Draws walls on the active level in one step (a rectangle, a circle):
    /// all of them or none.
    pub fn add_walls(&mut self, s: &WallSettings, pieces: &[Piece]) -> EditResult<Vec<WallId>> {
        let level = self.level;
        let top = s.top_on(self.model(), level);
        let label = if pieces.len() == 1 {
            "Crear muro"
        } else {
            "Crear muros"
        };
        self.transact(label, |m| {
            pieces
                .iter()
                .map(|w| {
                    m.add_wall(NewWall {
                        wall_type: s.wall_type,
                        level,
                        start: w.start,
                        end: w.end,
                        reference: s.reference,
                        height: s.height,
                        base_offset: s.base_offset,
                        bulge: w.bulge,
                        top,
                    })
                })
                .collect()
        })
    }

    /// Bends the selected walls (bulge 0 straightens them), keeping their
    /// ends.
    pub fn set_walls_bulge(&mut self, walls: &[WallId], bulge: f64) -> EditResult {
        self.transact("Curvar muro", |m| {
            walls.iter().try_for_each(|w| m.set_wall_bulge(*w, bulge))
        })
    }

    /// Draws a slab on the active level.
    pub fn add_slab(
        &mut self,
        slab_type: SlabTypeId,
        outline: Vec<Point2>,
        top_offset: f64,
    ) -> EditResult<SlabId> {
        let level = self.level;
        self.transact("Crear forjado", |m| {
            m.add_slab(NewSlab {
                slab_type,
                level,
                outline,
                top_offset,
            })
        })
    }

    /// Cuts a hole through the slab of the active level that holds the
    /// outline (the topmost, if several do).
    pub fn add_slab_hole(&mut self, outline: Vec<Point2>) -> EditResult<SlabId> {
        let level = self.level;
        let holds = |s: &archisim_core::Slab| {
            outline
                .iter()
                .all(|p| crate::pick::contains(&s.outline, *p))
        };
        let slab = self
            .model()
            .slabs_on_level(level)
            .filter(|s| holds(s))
            .max_by(|a, b| a.top_offset.total_cmp(&b.top_offset))
            .map(|s| s.id)
            .ok_or("dibuja el hueco dentro de un forjado de esta planta")?;
        self.transact("Hueco en forjado", |m| m.add_slab_hole(slab, outline))?;
        Ok(slab)
    }

    /// Fills a hole of a slab again.
    pub fn remove_slab_hole(&mut self, slab: SlabId, index: usize) -> EditResult {
        self.transact("Quitar hueco de forjado", |m| {
            m.remove_slab_hole(slab, index)
        })
    }

    /// Places a door or window in a wall.
    pub fn add_opening(&mut self, spec: NewOpening) -> EditResult<OpeningId> {
        let label = match self.model().opening_type(spec.opening_type) {
            Some(t) if t.is_door() => "Colocar puerta",
            _ => "Colocar ventana",
        };
        self.transact(label, |m| m.add_opening(spec))
    }

    /// Deletes the selected elements in one step. A wall takes its doors and
    /// windows with it.
    pub fn delete_selection(&mut self) -> EditResult<usize> {
        let items: Vec<Element> = self.selection.iter().copied().collect();
        if items.is_empty() {
            return Ok(0);
        }
        let n = items.len();
        self.transact("Borrar", |m| {
            // Doors and windows first: their wall may be deleted too.
            for e in &items {
                if let Element::Opening(id) = e {
                    if m.opening(*id).is_some() {
                        m.remove_opening(*id)?;
                    }
                }
            }
            for e in &items {
                match e {
                    Element::Wall(id) => {
                        m.remove_wall(*id)?;
                    }
                    Element::Slab(id) => {
                        m.remove_slab(*id)?;
                    }
                    Element::Dimension(id) => {
                        m.remove_dimension(*id)?;
                    }
                    Element::Mark(id) => {
                        m.remove_elevation_mark(*id)?;
                    }
                    Element::Opening(_) => {}
                }
            }
            Ok(())
        })?;
        self.selection.clear();
        Ok(n)
    }

    pub fn set_walls_type(&mut self, walls: &[WallId], t: WallTypeId) -> EditResult {
        self.transact("Cambiar tipo de muro", |m| {
            walls.iter().try_for_each(|w| m.set_wall_type(*w, t))
        })
    }

    pub fn set_walls_reference(&mut self, walls: &[WallId], r: ReferenceLine) -> EditResult {
        self.transact("Cambiar línea de referencia", |m| {
            walls.iter().try_for_each(|w| m.set_wall_reference(*w, r))
        })
    }

    pub fn set_walls_height(&mut self, walls: &[WallId], height: f64) -> EditResult {
        self.transact("Cambiar altura de muro", |m| {
            walls.iter().try_for_each(|w| {
                let base = m.wall(*w).map_or(0.0, |x| x.base_offset);
                m.set_wall_height(*w, height, base)
            })
        })
    }

    pub fn set_walls_base_offset(&mut self, walls: &[WallId], base: f64) -> EditResult {
        self.transact("Cambiar desfase de base", |m| {
            walls
                .iter()
                .try_for_each(|w| m.set_wall_base_offset(*w, base))
        })
    }

    /// Ties walls to a top level, or frees them (they keep their height).
    pub fn set_walls_top(&mut self, walls: &[WallId], top: WallTop) -> EditResult {
        let label = match top {
            WallTop::Unconnected => "Desconectar muro de la planta superior",
            WallTop::Level { .. } => "Cambiar restricción superior",
        };
        self.transact(label, |m| {
            walls.iter().try_for_each(|w| m.set_wall_top(*w, top))
        })
    }

    /// Sets a wall's length keeping its start and direction.
    pub fn set_wall_length(&mut self, wall: WallId, length: f64) -> EditResult {
        if length <= 0.0 {
            return Err("la longitud debe ser mayor que cero".into());
        }
        self.transact("Cambiar longitud de muro", |m| {
            let w = m.wall(wall).ok_or(ModelError::UnknownWall(wall))?;
            let start = w.start;
            if w.is_curved() {
                // The same curve, longer or shorter: the bend stays.
                let end = start + (w.end - w.start) * (length / w.length());
                return m.move_wall(wall, start, end);
            }
            let dir = (w.end - w.start)
                .normalized()
                .ok_or(ModelError::ZeroLengthWall)?;
            m.move_wall(wall, start, start + dir * length)
        })
    }

    /// Sets the radius of a curved wall, keeping its ends, the side it
    /// bows out to and whether it sweeps more than half a turn.
    pub fn set_wall_radius(&mut self, wall: WallId, radius: f64) -> EditResult {
        let w = self
            .model()
            .wall(wall)
            .ok_or_else(|| explain(&ModelError::UnknownWall(wall)))?;
        let arc = w.arc().ok_or("el muro es recto")?;
        let chord = w.start.distance(w.end);
        if radius < chord / 2.0 - 1e-9 {
            return Err(format!(
                "el radio debe ser al menos la mitad de la distancia entre los extremos ({} mm)",
                crate::units::fmt_mm(chord / 2.0)
            ));
        }
        let minor = 2.0 * (chord / (2.0 * radius)).min(1.0).asin();
        let sweep = if arc.sweep.abs() > std::f64::consts::PI + 1e-9 {
            std::f64::consts::TAU - minor
        } else {
            minor
        };
        let bulge = (sweep / 4.0).tan() * arc.sweep.signum();
        self.transact("Cambiar radio de muro", |m| m.set_wall_bulge(wall, bulge))
    }

    pub fn flip_walls(&mut self, walls: &[WallId]) -> EditResult {
        self.transact("Invertir muro", |m| {
            walls.iter().try_for_each(|w| m.flip_wall(*w))
        })
    }

    pub fn set_opening_type(&mut self, o: OpeningId, t: OpeningTypeId) -> EditResult {
        self.transact("Cambiar tipo de puerta o ventana", |m| {
            m.set_opening_type(o, t)
        })
    }

    pub fn move_opening(&mut self, o: OpeningId, along: f64, sill: f64) -> EditResult {
        self.transact("Mover puerta o ventana", |m| m.move_opening(o, along, sill))
    }

    pub fn set_opening_handing(
        &mut self,
        o: OpeningId,
        opens_to: WallSide,
        hand: Hand,
    ) -> EditResult {
        self.transact("Cambiar apertura", |m| {
            m.set_opening_handing(o, opens_to, hand)
        })
    }

    /// Moves walls (with their doors and windows) and slabs to another
    /// level, in one step, keeping their offsets from the level. Doors,
    /// windows and annotations in `items` are left alone (doors and
    /// windows go with their wall). Returns how many elements moved; those
    /// that leave the active level leave the selection too.
    pub fn set_elements_level(&mut self, items: &[Element], level: LevelId) -> EditResult<usize> {
        let movable: Vec<Element> = items
            .iter()
            .copied()
            .filter(|e| matches!(e, Element::Wall(_) | Element::Slab(_)))
            .collect();
        if movable.is_empty() {
            return Ok(0);
        }
        self.transact("Cambiar de planta", |m| {
            for e in &movable {
                match e {
                    Element::Wall(id) => m.set_wall_level(*id, level)?,
                    Element::Slab(id) => m.set_slab_level(*id, level)?,
                    _ => {}
                }
            }
            Ok(())
        })?;
        if level != self.level {
            for e in &movable {
                self.selection.remove(e);
            }
            // Their doors and windows are no longer on this level either.
            let model = self.doc.model();
            self.selection.retain(|e| match e {
                Element::Opening(o) => model
                    .opening(*o)
                    .and_then(|o| model.wall(o.wall))
                    .is_some_and(|w| w.level == self.level),
                _ => true,
            });
        }
        Ok(movable.len())
    }

    pub fn set_slab_type(&mut self, s: SlabId, t: SlabTypeId) -> EditResult {
        self.transact("Cambiar tipo de forjado", |m| m.set_slab_type(s, t))
    }

    pub fn set_slab_offset(&mut self, s: SlabId, offset: f64) -> EditResult {
        self.transact("Cambiar desfase de forjado", |m| {
            m.set_slab_offset(s, offset)
        })
    }

    /// Moves, copies or stretches the selection as dragged with the mouse,
    /// in one step. After a copy the copies are selected.
    pub fn drag(&mut self, drag: &crate::drag::Drag) -> EditResult<&'static str> {
        let label = crate::drag::label(self.model(), drag);
        let selection = self.selection.clone();
        let new_selection = self.transact(label, |m| crate::drag::apply(m, &selection, drag))?;
        if let Some(s) = new_selection {
            self.selection = s;
        }
        Ok(label)
    }

    /// A drag applied to given elements instead of the selection (a
    /// temporary dimension typed in), in one step.
    pub fn drag_elements(
        &mut self,
        elements: &BTreeSet<Element>,
        drag: &crate::drag::Drag,
    ) -> EditResult<&'static str> {
        let label = crate::drag::label(self.model(), drag);
        self.transact(label, |m| crate::drag::apply(m, elements, drag))?;
        Ok(label)
    }

    /// Draws a slab on the active level from the room around `p`, as the
    /// magic wand does.
    pub fn add_slab_in_room(
        &mut self,
        slab_type: SlabTypeId,
        p: Point2,
        contour: archisim_geom::Contour,
        top_offset: f64,
    ) -> EditResult<SlabId> {
        let outline = archisim_geom::room_outline(self.model(), self.level, p, contour)
            .map_err(|e| crate::units::explain_room(&e))?;
        self.add_slab(slab_type, outline, top_offset)
    }

    /// Draws a dimension on the active level.
    pub fn add_dimension(&mut self, mut spec: NewDimension) -> EditResult<DimensionId> {
        spec.level = self.level;
        self.transact("Acotar", |m| m.add_dimension(spec))
    }

    /// Places a level mark on the active level.
    pub fn add_elevation_mark(&mut self, p: Point2) -> EditResult<ElevationMarkId> {
        let level = self.level;
        self.transact("Cota de nivel", |m| m.add_elevation_mark(level, p))
    }

    // ---- Levels -------------------------------------------------------------

    /// Adds a level and makes it the active one.
    pub fn add_level(&mut self, name: &str, elevation: f64) -> EditResult<LevelId> {
        let name = name.trim().to_string();
        if name.is_empty() {
            return Err("el nombre no puede quedar vacío".into());
        }
        if !elevation.is_finite() {
            return Err("la cota no es un número válido".into());
        }
        let id = self.transact("Añadir planta", |m| Ok(m.add_level(name, elevation)))?;
        self.level = id;
        self.selection.clear();
        Ok(id)
    }

    pub fn rename_level(&mut self, id: LevelId, name: &str) -> EditResult {
        let name = name.trim().to_string();
        if name.is_empty() {
            return Err("el nombre no puede quedar vacío".into());
        }
        self.transact("Renombrar planta", |m| m.rename_level(id, name))
    }

    pub fn set_level_elevation(&mut self, id: LevelId, elevation: f64) -> EditResult {
        self.transact("Cambiar cota de planta", |m| {
            m.set_level_elevation(id, elevation)
        })
    }

    /// Removes a level with everything on it. If it was the active level,
    /// the nearest remaining one becomes active.
    pub fn remove_level(&mut self, id: LevelId) -> EditResult {
        let elevation = self.model().level(id).map(|l| l.elevation);
        self.transact("Borrar planta", |m| m.remove_level(id).map(|_| ()))?;
        if self.level == id {
            let e = elevation.unwrap_or(0.0);
            if let Some(l) = self
                .model()
                .levels()
                .into_iter()
                .min_by(|a, b| (a.elevation - e).abs().total_cmp(&(b.elevation - e).abs()))
            {
                self.level = l.id;
            }
            self.selection.clear();
        }
        Ok(())
    }

    pub fn rename_project(&mut self, name: &str) -> EditResult {
        let name = name.trim().to_string();
        if name.is_empty() {
            return Err("el nombre no puede quedar vacío".into());
        }
        self.transact("Renombrar proyecto", |m| {
            m.project_name = name;
            Ok(())
        })
    }

    /// Bounds of everything on the active level, to frame the view.
    pub fn level_bounds(&self) -> Option<(Point2, Point2)> {
        let m = self.model();
        let mut lo = Point2::new(f64::INFINITY, f64::INFINITY);
        let mut hi = Point2::new(f64::NEG_INFINITY, f64::NEG_INFINITY);
        let mut any = false;
        let points = crate::pick::elements_on_level(m, self.level)
            .into_iter()
            .filter_map(|e| crate::pick::outline(m, e))
            .flatten();
        for p in points {
            any = true;
            lo = Point2::new(lo.x.min(p.x), lo.y.min(p.y));
            hi = Point2::new(hi.x.max(p.x), hi.y.max(p.y));
        }
        any.then_some((lo, hi))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pick::tests::room;

    fn settings(m: &Model) -> WallSettings {
        WallSettings {
            wall_type: m.wall_types().next().unwrap().id,
            reference: ReferenceLine::ExteriorFace,
            height: 3.0,
            base_offset: 0.0,
            top: TopChoice::NextLevel,
            top_offset: 0.0,
        }
    }

    #[test]
    fn new_project_is_unmodified_until_edited() {
        let mut ed = Editor::new_project();
        assert!(!ed.is_modified());
        assert_eq!(ed.display_name(), "Sin título");
        let s = settings(ed.model());
        let r0 = ed.revision();
        ed.add_wall(&s, Point2::new(0.0, 0.0), Point2::new(5.0, 0.0))
            .unwrap();
        assert!(ed.is_modified());
        assert!(ed.revision() > r0);
        assert_eq!(ed.undo_label(), Some("Crear muro"));
        ed.undo();
        // Back to the saved state.
        assert!(!ed.is_modified());
        ed.redo();
        assert_eq!(ed.model().walls().count(), 1);
        // Refused edits change nothing and explain why in Spanish.
        let err = ed
            .add_wall(&s, Point2::new(1.0, 1.0), Point2::new(1.0, 1.0))
            .unwrap_err();
        assert!(err.contains("dos puntos"), "{err}");
        assert_eq!(ed.model().walls().count(), 1);
    }

    #[test]
    fn delete_and_undo_keep_selection_consistent() {
        let (m, _, _) = room();
        let mut ed = Editor::with_model(m, None);
        let walls = ed.model().walls().map(|w| w.id).collect::<Vec<_>>();
        let south = *walls.last().unwrap();
        let door = ed.model().openings().next().unwrap().id;
        ed.selection.insert(Element::Wall(south));
        ed.selection.insert(Element::Opening(door));
        assert_eq!(ed.delete_selection().unwrap(), 2);
        assert!(ed.model().wall(south).is_none());
        assert!(ed.model().opening(door).is_none());
        assert_eq!(ed.undo_label(), Some("Borrar"));
        ed.undo();
        assert!(ed.model().opening(door).is_some());
        // A selection pointing at something undone away is dropped.
        ed.selection.insert(Element::Wall(WallId(9999)));
        ed.redo();
        assert!(ed.selection.is_empty());
    }

    #[test]
    fn property_edits_are_single_steps() {
        let (m, _, lib) = room();
        let mut ed = Editor::with_model(m, None);
        let walls = ed.model().walls().map(|w| w.id).collect::<Vec<_>>();
        ed.set_walls_height(&walls, 2.8).unwrap();
        assert!(ed.model().walls().all(|w| w.height == 2.8));
        assert_eq!(ed.undo_label(), Some("Cambiar altura de muro"));
        // Too low for the 2.05 m door: refused for all of them.
        assert!(ed.set_walls_height(&walls, 2.0).is_err());
        assert!(ed.model().walls().all(|w| w.height == 2.8));
        ed.set_walls_type(&walls[..1], lib.bearing_wall).unwrap();
        assert_eq!(
            ed.model().wall(walls[0]).unwrap().wall_type,
            lib.bearing_wall
        );
        ed.set_wall_length(walls[0], 5.0).unwrap();
        assert_eq!(
            ed.model().wall(walls[0]).unwrap().end,
            Point2::new(0.0, 5.0)
        );
        assert!(ed.set_wall_length(walls[0], 0.0).is_err());
        let door = ed.model().openings().next().unwrap().id;
        ed.move_opening(door, 2.0, 0.0).unwrap();
        ed.set_opening_handing(door, WallSide::Exterior, Hand::Right)
            .unwrap();
        assert!(ed.set_opening_type(door, lib.window_one_leaf).is_err());
        ed.set_opening_type(door, lib.double_door).unwrap();
        let slab = ed.model().slabs().next().unwrap().id;
        ed.set_slab_offset(slab, -0.02).unwrap();
        ed.set_slab_type(slab, lib.floor_slab).unwrap();
        ed.rename_project("Casa Poetas").unwrap();
        assert!(ed.rename_project("  ").is_err());
        assert_eq!(ed.model().project_name, "Casa Poetas");
    }

    #[test]
    fn save_open_and_export() {
        let dir = std::env::temp_dir().join(format!("archisim-editor-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let (m, _, _) = room();
        let mut ed = Editor::with_model(m, None);
        assert!(ed.save(None).is_err(), "no file yet");
        let path = ed.save(Some(&dir.join("casa"))).unwrap();
        assert_eq!(path.extension().unwrap(), "archisim");
        assert!(!ed.is_modified());
        assert_eq!(ed.display_name(), "casa.archisim");
        let reopened = Editor::open(&path).unwrap();
        assert_eq!(reopened.model(), ed.model());
        ed.export_ifc(&dir.join("casa.ifc")).unwrap();
        ed.export_plan(&dir.join("casa.svg"), false, 50.0).unwrap();
        let ifc = std::fs::read_to_string(dir.join("casa.ifc")).unwrap();
        assert!(ifc.contains("IFCWALL("));
        std::fs::write(dir.join("roto.archisim"), "{").unwrap();
        let err = Editor::open(&dir.join("roto.archisim")).err().unwrap();
        assert!(err.starts_with("el fichero está mal escrito"), "{err}");
        std::fs::remove_dir_all(&dir).ok();
    }
}
