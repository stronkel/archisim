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

mod concept;
mod curtain;
mod foundations;
pub mod library;
mod modules;
mod objects;
mod phases;
mod presentation;
mod railings;
mod roofs;
mod room_tool;
mod sections;
mod stairs;
mod structure;
mod terrain;
pub(crate) mod types;

pub use concept::{add_mass_in, circle, snap_height, MassMode, MassSettings, MassWhat};
pub use curtain::CurtainSettings;
pub use foundations::{FootingMode, FootingSettings};
pub use modules::ModuleStatus;
pub use objects::ObjectSettings;
pub use railings::RailingSettings;
pub use roofs::{slopes_for, RoofKind, RoofSettings};
pub use room_tool::{usages_of, RoomSettings, USAGE_SETS};
pub use stairs::{default_stair_settings, StairSettings};
pub use structure::{default_member_type, normal_angle, BeamSettings, ColumnSettings};
pub use terrain::TerrainSettings;
pub use types::copy_name;

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
    /// The plan geometry of every wall, for this revision.
    geo: crate::geo::GeoCache,
    /// The model with the copies of its modules, for this revision.
    flat: modules::FlatCache,
    /// The level shown in the plan and where new elements go.
    pub level: LevelId,
    pub selection: BTreeSet<Element>,
    /// In a shared project, what the person has reserved: edits to
    /// anything else are refused.
    pub team: Option<crate::team::Guard>,
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
            geo: Default::default(),
            flat: Default::default(),
            level,
            selection: BTreeSet::new(),
            team: None,
        }
    }

    /// Puts another version of the project in place (received from the
    /// team): no undo across it, and it counts as saved.
    pub fn reset_model(&mut self, model: Model) {
        self.saved = model.clone();
        self.doc = Document::new(model);
        self.changed();
    }

    /// The project as it is now counts as saved (in a shared project, the
    /// working copy was written to the folder).
    pub fn mark_saved(&mut self) {
        self.saved = self.doc.model().clone();
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
        } else if model.member_types().next().is_none() {
            // Columns and beams to draw with, even if the file had none.
            archisim_library::add_structure(&mut model).map_err(|e| explain(&e))?;
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

    /// Writes the IFC file of the whole project, in IFC 4.3 or IFC4.
    pub fn export_ifc(&self, path: &Path, schema: archisim_ifc::Schema) -> EditResult {
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
                schema,
            },
        );
        std::fs::write(path, text).map_err(|e| format!("no se pudo escribir el IFC: {e}"))
    }

    /// Writes the plan of the active level as SVG at a scale (50 for
    /// 1:50): line weights are paper weights, and doors and windows use the
    /// plan symbol their family gives for that scale.
    /// Writes the current level's plan with these options, as the file's
    /// extension says: PDF (vector, at its scale), DXF (millimetres) or
    /// SVG.
    pub fn export_plan(&self, path: &Path, options: &PlanOptions) -> EditResult {
        let shown = self.shown();
        let model: &Model = &shown;
        let bytes = match extension(path).as_str() {
            "dxf" => archisim_export::dxf::plan_dxf(model, self.level, options)?.into_bytes(),
            "pdf" => {
                let svg = render_plan(model, self.level, options);
                let title = model.level(self.level).map_or("", |l| l.name.as_str());
                archisim_export::pdf::pdf(
                    &[archisim_export::pdf::Page { svg: &svg, title }],
                    &model.project_name,
                )?
            }
            _ => render_plan(model, self.level, options).into_bytes(),
        };
        std::fs::write(path, bytes).map_err(|e| format!("no se pudo escribir la planta: {e}"))
    }

    /// Writes a section or elevation, as the file's extension says: PDF
    /// (vector, at its scale), DXF (millimetres) or SVG. `options` gives
    /// the scale and the phases shown.
    pub fn export_section(
        &self,
        path: &Path,
        section: archisim_core::SectionId,
        options: &PlanOptions,
    ) -> EditResult {
        let shown = self.shown();
        let model: &Model = &shown;
        let sec = model.section(section).ok_or("la sección ya no existe")?;
        let phased = archisim_plan::phases::phase_model(model, options.phase_view);
        let shown = phased.as_ref().unwrap_or(model);
        let d = archisim_plan::section::section_drawing(shown, sec);
        let bytes = match extension(path).as_str() {
            "dxf" => archisim_export::dxf::section_dxf(shown, &d, options)?.into_bytes(),
            "pdf" => {
                let svg = archisim_plan::section::render_section(shown, &d, options);
                archisim_export::pdf::pdf(
                    &[archisim_export::pdf::Page {
                        svg: &svg,
                        title: &d.title,
                    }],
                    &model.project_name,
                )?
            }
            _ => archisim_plan::section::render_section(shown, &d, options).into_bytes(),
        };
        std::fs::write(path, bytes).map_err(|e| format!("no se pudo escribir la sección: {e}"))
    }

    /// Writes sheets: several in one PDF, or one as PDF, DXF or SVG.
    pub fn export_sheets(&self, path: &Path, sheets: &[archisim_core::SheetId]) -> EditResult {
        let shown = self.shown();
        let model: &Model = &shown;
        let svg = |id| {
            archisim_plan::sheet::render_sheet(
                model,
                id,
                &archisim_plan::sheet::SheetOptions::default(),
            )
        };
        let bytes = match (extension(path).as_str(), sheets) {
            ("dxf", [id]) => archisim_export::dxf::sheet_dxf(model, *id)?.into_bytes(),
            ("svg", [id]) => svg(*id).into_bytes(),
            ("png", [id]) => archisim_export::png::png(&svg(*id), 200.0)?,
            ("pdf", _) => {
                let pages: Vec<(String, String)> = sheets
                    .iter()
                    .filter_map(|id| {
                        let s = model.sheet(*id)?;
                        Some((svg(*id), format!("{} {}", s.number, s.name)))
                    })
                    .collect();
                let pages: Vec<archisim_export::pdf::Page> = pages
                    .iter()
                    .map(|(svg, title)| archisim_export::pdf::Page { svg, title })
                    .collect();
                archisim_export::pdf::pdf(&pages, &model.project_name)?
            }
            _ => return Err("los planos se exportan en PDF, DXF, SVG o PNG (uno a uno)".into()),
        };
        std::fs::write(path, bytes).map_err(|e| format!("no se pudo escribir el plano: {e}"))
    }

    /// Brings a DXF drawing into a view as 2D drafting, `k` metres per
    /// paper millimetre of the view (texts keep their size on paper).
    pub fn import_dxf(
        &mut self,
        path: &Path,
        view: archisim_core::ViewRef,
        k: f64,
    ) -> EditResult<archisim_export::dxf_read::Report> {
        let bytes = std::fs::read(path).map_err(|e| format!("no se pudo leer el DXF: {e}"))?;
        let drawing = archisim_export::dxf_read::read(&bytes)?;
        if drawing.items.is_empty() {
            return Err("el DXF no tiene nada que ArchiSim sepa dibujar en 2D".into());
        }
        let label = format!(
            "Importar DXF «{}»",
            path.file_name()
                .map(|n| n.to_string_lossy())
                .unwrap_or_default()
        );
        self.transact(&label, |m| {
            archisim_export::dxf_read::add_to(
                m,
                view,
                &drawing,
                archisim_core::Vec2::new(0.0, 0.0),
                k,
            )
        })
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
            Element::Column(id) => model.column(*id).is_some(),
            Element::Beam(id) => model.beam(*id).is_some(),
            Element::Section(id) => model.section(*id).is_some(),
            Element::Roof(id) => model.roof(*id).is_some(),
            Element::Stair(id) => model.stair(*id).is_some(),
            Element::Terrain(id) => model.terrain(*id).is_some(),
            Element::CurtainWall(id) => model.curtain_wall(*id).is_some(),
            Element::Railing(id) => model.railing(*id).is_some(),
            Element::Object(id) => model.object(*id).is_some(),
            Element::Draft(id) => model.draft(*id).is_some(),
            Element::Room(id) => model.room(*id).is_some(),
            Element::Tag(id) => model.tag(*id).is_some(),
            Element::Footing(id) => model.footing(*id).is_some(),
            Element::Grid(id) => model.grid(*id).is_some(),
            Element::Module(id) => model.module(*id).is_some(),
            Element::Mass(id) => model.mass(*id).is_some(),
            Element::Plot(id) => model.plot(*id).is_some(),
            Element::Road(id) => model.road(*id).is_some(),
            Element::Planting(id) => model.planting(*id).is_some(),
            Element::Run(id) => model.run(*id).is_some(),
            Element::Edge(id) => model.edge(*id).is_some(),
            Element::Device(id) => model.device(*id).is_some(),
        });
        if model.level(self.level).is_none() {
            if let Some(l) = model.levels().first() {
                self.level = l.id;
            }
        }
    }

    pub(crate) fn transact<T>(
        &mut self,
        label: &str,
        edit: impl FnOnce(&mut Model) -> Result<T, ModelError>,
    ) -> EditResult<T> {
        if let Some(g) = self.team.as_mut() {
            g.prepare(self.revision, self.doc.model());
        }
        let guard = &mut self.team;
        let mut refused = None;
        let result = self
            .doc
            .transact(label, |m| {
                // Dimensions follow what they measure (and new or moved
                // ones are tied to what is under their points).
                let before = archisim_geom::anchors::snapshot(m);
                // Rooms bounded by walls follow them too.
                let rooms = m.rooms().any(|r| r.bounded);
                let walls = if rooms {
                    archisim_geom::room::wall_signatures(m)
                } else {
                    Default::default()
                };
                // Tags go with their element.
                let tags: Vec<(archisim_core::TagId, Point2, Option<Point2>)> = m
                    .tags()
                    .map(|t| (t.id, t.at, archisim_plan::rooms::tag_anchor(m, t.target)))
                    .collect();
                let r = edit(m)?;
                // In a shared project, only what is reserved (what follows
                // below goes with it: dimensions, rooms, tags).
                if let Some(g) = guard.as_mut() {
                    if let Err(e) = g.check(m) {
                        refused = Some(e.clone());
                        return Err(ModelError::InUse(e));
                    }
                }
                if m.footings().next().is_some() {
                    m.footings_follow_hosts();
                }
                for (id, at, anchor) in tags {
                    let Some(t) = m.tag(id) else { continue };
                    if t.at != at {
                        continue;
                    }
                    let now = archisim_plan::rooms::tag_anchor(m, t.target);
                    if let (Some(a), Some(b)) = (anchor, now) {
                        if a.distance(b) > 1e-9 {
                            let _ = m.move_tag(id, at + (b - a));
                        }
                    }
                }
                if m.dimensions().next().is_some() {
                    archisim_geom::anchors::follow(m, &before);
                }
                if rooms {
                    archisim_geom::room::follow_walls(m, &walls);
                }
                if m.tags().next().is_some() {
                    m.remove_orphan_tags();
                }
                Ok(r)
            })
            .map_err(|e| refused.take().unwrap_or_else(|| explain(&e)))?;
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
                    e @ (Element::Column(_)
                    | Element::Beam(_)
                    | Element::Section(_)
                    | Element::Roof(_)
                    | Element::Stair(_)
                    | Element::Terrain(_)
                    | Element::CurtainWall(_)
                    | Element::Railing(_)
                    | Element::Object(_)
                    | Element::Footing(_)
                    | Element::Grid(_)
                    | Element::Module(_)
                    | Element::Mass(_)
                    | Element::Plot(_)
                    | Element::Road(_)
                    | Element::Planting(_)
                    | Element::Run(_)
                    | Element::Edge(_)
                    | Element::Device(_)) => remove_element(m, *e)?,
                    Element::Draft(id) => {
                        m.remove_draft(*id)?;
                    }
                    Element::Room(id) => {
                        m.remove_room(*id)?;
                    }
                    Element::Tag(id) => {
                        if m.tag(*id).is_some() {
                            m.remove_tag(*id)?;
                        }
                    }
                    Element::Opening(_) => {}
                }
            }
            Ok(())
        })?;
        self.selection.clear();
        Ok(n)
    }

    /// Applies the changes of a step of a 2D command as one undoable
    /// step. New drafting elements go to `view` with the layer and style
    /// of `st`. Returns the elements created or changed (the new
    /// selection after a copy).
    pub fn apply_cad(
        &mut self,
        view: archisim_core::ViewRef,
        st: &crate::cad::CadSettings,
        step: &crate::cad::Changes,
    ) -> EditResult<BTreeSet<Element>> {
        use crate::cad::Change;
        let changes = step.changes.clone();
        let out = self.transact(step.label, |m| {
            let mut out = BTreeSet::new();
            for c in changes {
                match c {
                    Change::Add(shape) => {
                        let id = m.add_draft(archisim_core::NewDraft {
                            view,
                            layer: st.layer,
                            shape,
                            color: st.color,
                            weight: st.weight,
                            line: st.line,
                        })?;
                        out.insert(Element::Draft(id));
                    }
                    Change::AddDraft(spec) => {
                        out.insert(Element::Draft(m.add_draft(spec)?));
                    }
                    Change::Reshape(id, shape) => {
                        m.set_draft_shape(id, shape)?;
                        out.insert(Element::Draft(id));
                    }
                    Change::Remove(e) => match e {
                        Element::Draft(id) => {
                            m.remove_draft(id)?;
                        }
                        Element::Wall(id) => {
                            if m.wall(id).is_some() {
                                m.remove_wall(id)?;
                            }
                        }
                        Element::Slab(id) => {
                            m.remove_slab(id)?;
                        }
                        Element::Opening(id) => {
                            if m.opening(id).is_some() {
                                m.remove_opening(id)?;
                            }
                        }
                        Element::Dimension(id) => {
                            m.remove_dimension(id)?;
                        }
                        Element::Mark(id) => {
                            m.remove_elevation_mark(id)?;
                        }
                        Element::Room(id) => {
                            m.remove_room(id)?;
                        }
                        Element::Tag(id) => {
                            if m.tag(id).is_some() {
                                m.remove_tag(id)?;
                            }
                        }
                        e @ (Element::Column(_)
                        | Element::Beam(_)
                        | Element::Section(_)
                        | Element::Roof(_)
                        | Element::Stair(_)
                        | Element::Terrain(_)
                        | Element::CurtainWall(_)
                        | Element::Railing(_)
                        | Element::Object(_)
                        | Element::Footing(_)
                        | Element::Grid(_)
                        | Element::Module(_)
                        | Element::Mass(_)
                        | Element::Plot(_)
                        | Element::Road(_)
                        | Element::Planting(_)
                        | Element::Run(_)
                        | Element::Edge(_)
                        | Element::Device(_)) => remove_element(m, e)?,
                    },
                    Change::Transform { items, t, copy } => {
                        out.extend(crate::xform::transform(m, &items, &t, copy)?);
                    }
                    Change::RoomAt(p) => {
                        let level = match view {
                            archisim_core::ViewRef::Level(l) => l,
                            _ => return Err(ModelError::InvalidShape("no room there")),
                        };
                        let outline = archisim_geom::room_outline(
                            m,
                            level,
                            p,
                            archisim_geom::Contour::InteriorFace,
                        )
                        .map_err(|_| ModelError::InvalidShape("no room there"))?;
                        out.insert(Element::Room(new_room(m, level, outline, p, true)?));
                    }
                    Change::Room(outline) => {
                        let level = match view {
                            archisim_core::ViewRef::Level(l) => l,
                            _ => return Err(ModelError::InvalidShape("no room there")),
                        };
                        let n = outline.len().max(1) as f64;
                        let c = outline
                            .iter()
                            .fold(Point2::new(0.0, 0.0), |a, p| a + *p * (1.0 / n));
                        out.insert(Element::Room(new_room(m, level, outline, c, false)?));
                    }
                    Change::Tag(e, at) => {
                        let target = tag_target(e)
                            .ok_or(ModelError::InvalidShape("nothing to tag there"))?;
                        let at = match at {
                            Some(p) => p,
                            None => default_tag_point(m, target)
                                .ok_or(ModelError::InvalidShape("nothing to tag there"))?,
                        };
                        out.insert(Element::Tag(m.add_tag(target, at, false)?));
                    }
                    Change::TagAll => {
                        let level = match view {
                            archisim_core::ViewRef::Level(l) => l,
                            _ => continue,
                        };
                        let tagged: BTreeSet<archisim_core::TagTarget> =
                            m.tags().map(|t| t.target).collect();
                        let todo: Vec<archisim_core::OpeningId> = m
                            .walls_on_level(level)
                            .flat_map(|w| m.openings_in_wall(w.id))
                            .filter(|o| {
                                m.opening_type(o.opening_type)
                                    .is_some_and(|t| t.kind != archisim_core::OpeningKind::Void)
                            })
                            .map(|o| o.id)
                            .filter(|o| !tagged.contains(&archisim_core::TagTarget::Opening(*o)))
                            .collect();
                        for o in todo {
                            let target = archisim_core::TagTarget::Opening(o);
                            if let Some(at) = default_tag_point(m, target) {
                                out.insert(Element::Tag(m.add_tag(target, at, false)?));
                            }
                        }
                    }
                    Change::MatchStyle { from, to } => {
                        let f = m.draft(from).ok_or(ModelError::UnknownDraft(from))?.clone();
                        for id in to {
                            m.set_draft_style(id, f.layer, f.color, f.weight, f.line)?;
                            out.insert(Element::Draft(id));
                        }
                    }
                }
            }
            Ok(out)
        })?;
        Ok(out)
    }

    // ---- 2D drafting ----------------------------------------------------

    /// Changes the layer, colour, weight and line style of drafting
    /// elements; `None` in an argument leaves that property as it is, and
    /// `Some(None)` sets it to "by layer".
    pub fn set_drafts_style(
        &mut self,
        ids: &[archisim_core::DraftId],
        layer: Option<archisim_core::DrawLayerId>,
        color: Option<Option<archisim_core::Rgb>>,
        weight: Option<Option<f64>>,
        line: Option<Option<archisim_core::LineStyle>>,
    ) -> EditResult {
        self.transact("Cambiar estilo", |m| {
            for id in ids {
                let d = m.draft(*id).ok_or(ModelError::UnknownDraft(*id))?.clone();
                m.set_draft_style(
                    *id,
                    layer.unwrap_or(d.layer),
                    color.unwrap_or(d.color),
                    weight.unwrap_or(d.weight),
                    line.unwrap_or(d.line),
                )?;
            }
            Ok(())
        })
    }

    /// New geometry for a drafting element (a text's words, a hatch's
    /// pattern…).
    pub fn set_draft_shape(
        &mut self,
        id: archisim_core::DraftId,
        shape: archisim_core::Shape,
        label: &str,
    ) -> EditResult {
        self.transact(label, |m| m.set_draft_shape(id, shape))
    }

    /// The project's line weights by scale.
    pub fn set_graphics(&mut self, g: archisim_core::GraphicStandard) -> EditResult {
        if !g.is_valid() {
            return Err("los grosores tienen que ser mayores que cero".into());
        }
        self.transact("Normas gráficas", |m| {
            m.graphics = g;
            Ok(())
        })
    }

    pub fn add_sheet(
        &mut self,
        spec: archisim_core::NewSheet,
    ) -> EditResult<archisim_core::SheetId> {
        self.transact("Nuevo plano", |m| m.add_sheet(spec))
    }

    pub fn set_sheet(&mut self, sheet: archisim_core::Sheet) -> EditResult {
        self.transact("Cambiar plano", |m| m.set_sheet(sheet))
    }

    /// Removes a sheet with its views and what is drawn on it.
    pub fn remove_sheet(&mut self, id: archisim_core::SheetId) -> EditResult {
        self.transact("Borrar plano", |m| m.remove_sheet(id).map(|_| ()))
    }

    pub fn add_viewport(
        &mut self,
        spec: archisim_core::NewViewport,
    ) -> EditResult<archisim_core::ViewportId> {
        self.transact("Colocar vista en el plano", |m| m.add_viewport(spec))
    }

    pub fn set_viewport(&mut self, v: archisim_core::Viewport, label: &str) -> EditResult {
        self.transact(label, |m| m.set_viewport(v))
    }

    pub fn remove_viewport(&mut self, id: archisim_core::ViewportId) -> EditResult {
        self.transact("Quitar vista del plano", |m| {
            m.remove_viewport(id).map(|_| ())
        })
    }

    pub fn set_room(&mut self, room: archisim_core::Room, label: &str) -> EditResult {
        self.transact(label, |m| m.set_room(room))
    }

    pub fn set_tag_leader(&mut self, id: archisim_core::TagId, leader: bool) -> EditResult {
        self.transact("Cambiar etiqueta", |m| m.set_tag_leader(id, leader))
    }

    pub fn add_table(
        &mut self,
        sheet: archisim_core::SheetId,
        kind: archisim_core::ScheduleKind,
        level: Option<LevelId>,
        at: Point2,
    ) -> EditResult<archisim_core::TableId> {
        self.transact("Colocar tabla en el plano", |m| {
            m.add_table(sheet, kind, level, at)
        })
    }

    pub fn set_table(&mut self, t: archisim_core::SheetTable, label: &str) -> EditResult {
        self.transact(label, |m| m.set_table(t))
    }

    pub fn remove_table(&mut self, id: archisim_core::TableId) -> EditResult {
        self.transact("Quitar tabla del plano", |m| m.remove_table(id).map(|_| ()))
    }

    /// The measurements as a BC3 budget.
    pub fn export_bc3(&self, path: &Path) -> EditResult {
        let date = archisim_export::bc3::bc3_date(&archisim_ifc::utc_now());
        let bytes = archisim_export::bc3::bc3(&self.shown(), &date, env!("CARGO_PKG_VERSION"));
        std::fs::write(path, bytes).map_err(|e| format!("no se pudo escribir el BC3: {e}"))
    }

    /// The project's data for the title blocks.
    /// The building's use, accessibility and sprinklers (code checks).
    pub fn set_code_settings(&mut self, settings: archisim_core::CodeSettings) -> EditResult {
        self.transact("Uso del edificio (CTE)", |m| {
            m.set_code_settings(settings);
            Ok(())
        })
    }

    pub fn set_info(&mut self, info: archisim_core::ProjectInfo) -> EditResult {
        self.transact("Datos del proyecto", |m| {
            m.set_info(info);
            Ok(())
        })
    }

    pub fn add_draw_layer(
        &mut self,
        layer: archisim_core::DrawLayer,
    ) -> EditResult<archisim_core::DrawLayerId> {
        self.transact("Nueva capa", |m| m.add_draw_layer(layer))
    }

    pub fn set_draw_layer(&mut self, layer: archisim_core::DrawLayer) -> EditResult {
        self.transact("Cambiar capa", |m| m.set_draw_layer(layer))
    }

    /// Removes a layer, moving its elements to layer "0".
    pub fn remove_draw_layer(&mut self, id: archisim_core::DrawLayerId) -> EditResult {
        self.transact("Borrar capa", |m| {
            m.remove_draw_layer(id, Some(archisim_core::LAYER_ZERO))
                .map(|_| ())
        })
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

    /// Changes one instance parameter of a door or window (the corner of a
    /// corner window…).
    pub fn set_opening_value(
        &mut self,
        o: OpeningId,
        param: &str,
        value: archisim_core::Value,
    ) -> EditResult {
        let mut values = self
            .model()
            .opening(o)
            .ok_or("la puerta o ventana no existe")?
            .values
            .clone();
        values.insert(param.to_string(), value);
        self.transact("Cambiar parámetro de la puerta o ventana", |m| {
            m.set_opening_values(o, values)
        })
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
            .filter(|e| {
                matches!(
                    e,
                    Element::Wall(_)
                        | Element::Slab(_)
                        | Element::Column(_)
                        | Element::Beam(_)
                        | Element::Room(_)
                        | Element::Roof(_)
                        | Element::Stair(_)
                        | Element::Terrain(_)
                        | Element::CurtainWall(_)
                        | Element::Railing(_)
                        | Element::Object(_)
                        | Element::Footing(_)
                        | Element::Module(_)
                        | Element::Mass(_)
                        | Element::Plot(_)
                        | Element::Road(_)
                        | Element::Planting(_)
                        | Element::Run(_)
                        | Element::Edge(_)
                        | Element::Device(_)
                )
            })
            .collect();
        if movable.is_empty() {
            return Ok(0);
        }
        self.transact("Cambiar de planta", |m| {
            for e in &movable {
                match e {
                    Element::Wall(id) => m.set_wall_level(*id, level)?,
                    Element::Slab(id) => m.set_slab_level(*id, level)?,
                    Element::Column(id) => m.set_column_level(*id, level)?,
                    Element::Beam(id) => m.edit_beam(*id, |b| b.level = level)?,
                    Element::Room(id) => m.set_room_level(*id, level)?,
                    Element::Roof(id) => m.edit_roof(*id, |r| r.level = level)?,
                    Element::Stair(id) => m.edit_stair(*id, |s| s.level = level)?,
                    Element::Terrain(id) => m.edit_terrain(*id, |t| t.level = level)?,
                    Element::CurtainWall(id) => m.edit_curtain_wall(*id, |c| c.level = level)?,
                    Element::Railing(id) => m.edit_railing(*id, |r| r.level = level)?,
                    Element::Object(id) => m.edit_object(*id, |o| o.level = level)?,
                    Element::Footing(id) => m.edit_footing(*id, |f| f.level = level)?,
                    Element::Module(id) => m.edit_module(*id, |p| p.level = level)?,
                    Element::Mass(id) => m.edit_mass(*id, |x| x.level = level)?,
                    Element::Plot(id) => m.edit_plot(*id, |x| x.level = level)?,
                    Element::Road(id) => m.edit_road(*id, |x| x.level = level)?,
                    Element::Planting(id) => m.edit_planting(*id, |x| x.level = level)?,
                    Element::Run(id) => m.edit_run(*id, |x| x.level = level)?,
                    Element::Edge(id) => m.edit_edge(*id, |x| x.level = level)?,
                    Element::Device(id) => m.edit_device(*id, |x| x.level = level)?,
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

    /// Makes a slab inclined (or level, with `None`).
    pub fn set_slab_slope(
        &mut self,
        s: SlabId,
        slope: Option<archisim_core::SlabSlope>,
    ) -> EditResult {
        let label = if slope.is_some() {
            "Inclinar forjado"
        } else {
            "Forjado horizontal"
        };
        self.transact(label, |m| m.set_slab_slope(s, slope))
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
            .filter_map(|e| crate::pick::outline(self, e))
            .flatten();
        for p in points {
            any = true;
            lo = Point2::new(lo.x.min(p.x), lo.y.min(p.y));
            hi = Point2::new(hi.x.max(p.x), hi.y.max(p.y));
        }
        any.then_some((lo, hi))
    }
}

impl crate::geo::Geo for Editor {
    fn model(&self) -> &Model {
        self.doc.model()
    }

    fn wall_geometry(&self, id: WallId) -> Option<std::rc::Rc<archisim_geom::WallGeometry>> {
        self.geo.wall(self.doc.model(), self.revision, id)
    }
}

/// A file's extension in lower case.
fn extension(path: &Path) -> String {
    path.extension()
        .map(|e| e.to_string_lossy().to_lowercase())
        .unwrap_or_default()
}

/// Removes an element of phase 2 (columns, beams, roofs, stairs…).
fn remove_element(m: &mut Model, e: Element) -> Result<(), archisim_core::ModelError> {
    match e {
        Element::Column(id) => m.remove_column(id).map(drop),
        Element::Beam(id) => m.remove_beam(id).map(drop),
        Element::Section(id) => m.remove_section(id).map(drop),
        Element::Roof(id) => m.remove_roof(id).map(drop),
        Element::Stair(id) => m.remove_stair(id).map(drop),
        Element::Terrain(id) => m.remove_terrain(id).map(drop),
        Element::CurtainWall(id) => m.remove_curtain_wall(id).map(drop),
        Element::Railing(id) => m.remove_railing(id).map(drop),
        Element::Object(id) => m.remove_object(id).map(drop),
        Element::Footing(id) => m.remove_footing(id).map(drop),
        Element::Grid(id) => m.remove_grid(id).map(drop),
        Element::Module(id) => m.remove_module(id).map(drop),
        Element::Mass(id) => m.remove_mass(id).map(drop),
        Element::Plot(id) => m.remove_plot(id).map(drop),
        Element::Road(id) => m.remove_road(id).map(drop),
        Element::Planting(id) => m.remove_planting(id).map(drop),
        Element::Run(id) => m.remove_run(id).map(drop),
        Element::Edge(id) => m.remove_edge(id).map(drop),
        Element::Device(id) => m.remove_device(id).map(drop),
        _ => Ok(()),
    }
}

/// A new room of a level with the next number.
fn new_room(
    m: &mut Model,
    level: LevelId,
    outline: Vec<Point2>,
    label: Point2,
    bounded: bool,
) -> Result<archisim_core::RoomId, ModelError> {
    let number = m.next_room_number(level);
    m.add_room(archisim_core::NewRoom {
        level,
        number,
        name: "Local".into(),
        outline,
        holes: Vec::new(),
        label,
        bounded,
    })
}

/// What a tag of an element points at.
pub fn tag_target(e: Element) -> Option<archisim_core::TagTarget> {
    match e {
        Element::Opening(o) => Some(archisim_core::TagTarget::Opening(o)),
        Element::Wall(w) => Some(archisim_core::TagTarget::Wall(w)),
        Element::Room(r) => Some(archisim_core::TagTarget::Room(r)),
        _ => None,
    }
}

/// Where a tag goes by default: outside the wall in front of a door or
/// window, beside a wall's middle, under a room's stamp.
pub fn default_tag_point(m: &Model, target: archisim_core::TagTarget) -> Option<Point2> {
    match target {
        archisim_core::TagTarget::Opening(o) => {
            let g = archisim_geom::opening_geometry(m, o)?;
            Some(g.point((g.t0 + g.t1) / 2.0, g.exterior_offset + 0.35))
        }
        archisim_core::TagTarget::Wall(w) => {
            let w = m.wall(w)?;
            let u = (w.end - w.start).normalized()?;
            let t = m.wall_type(w.wall_type).map_or(0.2, |t| t.thickness());
            Some(w.midpoint() + u.perp_left() * (t + 0.4))
        }
        archisim_core::TagTarget::Room(r) => m.room(r).map(|r| r.label - Point2::new(0.0, 0.6)),
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
        ed.export_ifc(&dir.join("casa.ifc"), archisim_ifc::Schema::Ifc4x3)
            .unwrap();
        for name in ["casa.svg", "casa.pdf", "casa.dxf"] {
            ed.export_plan(&dir.join(name), &PlanOptions::default())
                .unwrap();
            assert!(
                std::fs::metadata(dir.join(name)).unwrap().len() > 1000,
                "{name}"
            );
        }
        let ifc = std::fs::read_to_string(dir.join("casa.ifc")).unwrap();
        assert!(ifc.contains("IFCWALL("));
        std::fs::write(dir.join("roto.archisim"), "{").unwrap();
        let err = Editor::open(&dir.join("roto.archisim")).err().unwrap();
        assert!(err.starts_with("el fichero está mal escrito"), "{err}");
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn dimensions_follow_walls_and_doors() {
        let (m, lvl, _) = room();
        let south = m.walls().last().unwrap().id;
        let door = m.openings().next().unwrap().id;
        let g = archisim_geom::opening_geometry(&m, door).unwrap();
        let jamb = g.point(g.t0, g.exterior_offset);
        let mut ed = Editor::with_model(m, None);
        // The whole south façade, and from its corner to the door.
        let total = ed
            .add_dimension(NewDimension {
                level: lvl,
                points: vec![Point2::new(0.0, 0.0), Point2::new(6.0, 0.0)],
                direction: Point2::new(1.0, 0.0),
                position: Point2::new(0.0, -1.0),
            })
            .unwrap();
        let to_door = ed
            .add_dimension(NewDimension {
                level: lvl,
                points: vec![Point2::new(6.0, 0.0), jamb],
                direction: Point2::new(1.0, 0.0),
                position: Point2::new(0.0, -1.5),
            })
            .unwrap();
        let d = ed.model().dimension(to_door).unwrap();
        assert!(d.anchors.iter().all(Option::is_some), "{:?}", d.anchors);
        let before = ed.model().dimension(to_door).unwrap().values()[0];
        // The door slides 50 cm: its dimension changes, the façade's not.
        let along = ed.model().opening(door).unwrap().along;
        ed.move_opening(door, along + 0.5, 0.0).unwrap();
        let after = ed.model().dimension(to_door).unwrap().values()[0];
        assert!((after - before).abs() - 0.5 < 1e-9, "{before} → {after}");
        assert!((ed.model().dimension(total).unwrap().values()[0] - 6.0).abs() < 1e-9);
        // The façade is lengthened to 7 m: so is its dimension.
        ed.set_wall_length(south, 7.0).unwrap();
        let v = ed.model().dimension(total).unwrap().values()[0];
        assert!((v - 7.0).abs() < 1e-9, "{v}");
        // Undo brings both back in one step each.
        ed.undo();
        assert!((ed.model().dimension(total).unwrap().values()[0] - 6.0).abs() < 1e-9);
    }
}
