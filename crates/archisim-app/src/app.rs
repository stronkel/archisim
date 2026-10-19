//! The ArchiSim window, laid out like ArchiCAD with Revit's properties:
//! the top bar (menus, project, quick commands), the toolbox on the left,
//! the open views as tabs over the drawing, the navigator and the
//! properties palette on the right, and the bottom bar with the scale,
//! zoom, snaps and display of the active view.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use archisim_core::{
    Hand, LevelId, Model, OpeningKind, OpeningTypeId, Point2, ReferenceLine, SlabFunction,
    SlabTypeId, WallFunction, WallSide, WallTypeId,
};
use archisim_geom::{opening_geometry, room_outline, Contour};
use eframe::egui::{
    self, Align2, Color32, CornerRadius, CursorIcon, FontId, Id, Key, KeyboardShortcut, Modifiers,
    PointerButton, Pos2, Rect, Sense, Shape, Stroke, StrokeKind, TextEdit, TextureHandle,
    TextureOptions, Ui, Vec2,
};

use crate::dim_tool::{DimSettings, DimTool};
use crate::drag::{grip_at, hotspots, Drag, Grip};
use crate::editor::{BeamSettings, ColumnSettings, Editor, WallSettings};
use crate::opening_tool::{placement, OpeningSettings, Placement};
use crate::pick::{in_window, outline, pick, Element};
use crate::plan_image::{rasterize_with, PlanKey};
use crate::slab_tool::{Outcome as SlabOutcome, SlabMode, SlabTool};
use crate::snap::{
    constrain, is_tracking_line, snap, snap_filtered, track, Snap, SnapFilter, SnapKind,
};
use crate::units::{explain_room, fmt_angle, fmt_area, fmt_m, fmt_mm, parse_length};
use crate::view::View;
use crate::wall_tool::{Escape, Outcome, Piece, WallMode, WallTool};

const ACCENT: Color32 = theme::ACCENT;
const PAPER: Color32 = Color32::WHITE;
const GRID_MINOR: Color32 = Color32::from_rgb(236, 238, 242);
const GRID_MAJOR: Color32 = Color32::from_rgb(214, 218, 226);
const REF_LINE: Color32 = Color32::from_rgb(192, 57, 43);
/// Tracking lines and temporary dimensions (Revit's blue).
const TRACK_LINE: Color32 = Color32::from_rgb(30, 110, 230);
/// Snapping and picking reach, in screen points.
const REACH: f32 = 10.0;
/// How close to a 45° line the cursor must be for the wall to lock onto it,
/// in screen points (horizontal and vertical lock by angle, see
/// [`crate::snap::ORTHO_LOCK`]).
const TRACK_REACH: f32 = 6.0;
/// Default width of each side column (left: tools and properties; right:
/// navigator), in points.
const SIDE_WIDTH: f32 = 340.0;
const MIN_SIDE_WIDTH: f32 = 290.0;
const MAX_SIDE_WIDTH: f32 = 520.0;
/// Width of the toolbox, with names and with icons only.
const TOOLBOX_WIDTH: f32 = 124.0;
const TOOLBOX_COMPACT_WIDTH: f32 = 52.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tool {
    Arrow,
    Wall,
    Slab,
    Door,
    Window,
    Dimension,
    Column,
    Beam,
    /// Rooms ("locales").
    Zone,
    /// Section lines (elevations are made from the navigator).
    Section,
    /// Pitched roofs.
    Roof,
    /// Stairs, by their walking line.
    Stair,
    /// Terrain, by survey points.
    Terrain,
    /// Curtain walls, by their line.
    CurtainWall,
    /// Loose railings, by their line.
    Railing,
    /// Furniture, sanitary ware and kitchen units.
    Object,
}

impl Tool {
    /// Tools that draw by clicking points (snaps and the crosshair apply).
    fn draws_points(self) -> bool {
        matches!(
            self,
            Tool::Wall
                | Tool::Slab
                | Tool::Dimension
                | Tool::Column
                | Tool::Beam
                | Tool::Zone
                | Tool::Section
                | Tool::Roof
                | Tool::Stair
                | Tool::Terrain
                | Tool::CurtainWall
                | Tool::Railing
                | Tool::Object
        )
    }

    /// The tool a key selects: M muro, F forjado, P puerta, V ventana,
    /// C cota.
    fn from_key(text: &str) -> Option<Tool> {
        match text.to_lowercase().as_str() {
            "m" => Some(Tool::Wall),
            "f" => Some(Tool::Slab),
            "p" => Some(Tool::Door),
            "v" => Some(Tool::Window),
            "c" => Some(Tool::Dimension),
            "i" => Some(Tool::Column),
            "g" => Some(Tool::Beam),
            "z" => Some(Tool::Zone),
            "s" => Some(Tool::Section),
            "t" => Some(Tool::Roof),
            "e" => Some(Tool::Stair),
            "r" => Some(Tool::Terrain),
            "u" => Some(Tool::CurtainWall),
            "b" => Some(Tool::Railing),
            "o" => Some(Tool::Object),
            _ => None,
        }
    }
}

/// Settings of the slab tool.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SlabSettings {
    pub slab_type: SlabTypeId,
    /// Height of the top face above the level.
    pub top_offset: f64,
    /// Magic wand: a click inside a room makes the slab (also with Space
    /// held while drawing by corners).
    pub wand: bool,
    /// Which outline of the room the magic wand takes.
    pub contour: Contour,
    /// The outline drawn is a hole through the slab it is drawn on, not a
    /// new slab.
    pub hole: bool,
}

/// Outlines a drag would give, or why it cannot be done.
type Outlines = Result<Vec<Vec<Point2>>, String>;
/// The room the magic wand finds, or why there is none.
type RoomFound = Result<Vec<Point2>, String>;

/// A drag of the selection in progress, with the arrow tool.
#[derive(Debug, Clone, PartialEq)]
pub struct DragState {
    pub grip: Grip,
    /// The point the selection was grabbed by.
    pub from: Point2,
    /// Distance typed while dragging, in the interface's units.
    pub typed: String,
    /// Elements that move: snapping ignores them.
    skip: BTreeSet<Element>,
}

impl DragState {
    /// The point the grip goes to: the typed distance towards the cursor,
    /// or the cursor itself.
    pub fn target(&self, cursor: Point2) -> Point2 {
        match (parse_length(&self.typed), (cursor - self.from).normalized()) {
            (Some(len), Some(dir)) if !self.typed.is_empty() => self.from + dir * len,
            _ => cursor,
        }
    }
}

/// Something that would throw away unsaved changes, waiting for an answer.
#[derive(Debug, Clone, PartialEq)]
enum Pending {
    New,
    Open(Option<PathBuf>),
    Import(Option<PathBuf>),
    Quit,
}

/// A status-bar message: what just happened, or why it could not.
#[derive(Debug, Clone, PartialEq)]
pub struct Message {
    pub text: String,
    pub error: bool,
}

pub struct ArchiSimApp {
    pub ed: Editor,
    pub view: View,
    pub tool: Tool,
    pub wall_tool: WallTool,
    pub wall_settings: WallSettings,
    pub slab_tool: SlabTool,
    pub slab_settings: SlabSettings,
    pub door_settings: OpeningSettings,
    pub window_settings: OpeningSettings,
    pub dim_tool: DimTool,
    pub dim_settings: DimSettings,
    pub column_settings: ColumnSettings,
    pub beam_settings: BeamSettings,
    pub beam_tool: structure_ui::BeamTool,
    pub section_tool: section_ui::SectionTool,
    /// The section shown: its drawing (worked out once per revision) and
    /// its picture on screen.
    pub section_cache: Option<section_ui::SectionCache>,
    /// Where the pointer is over the section shown (along, height).
    pub section_hover: Option<Point2>,
    pub space_settings: crate::editor::SpaceSettings,
    pub roof_settings: crate::editor::RoofSettings,
    pub roof_tool: SlabTool,
    pub stair_settings: crate::editor::StairSettings,
    pub stair_tool: stair_ui::StairTool,
    pub terrain_settings: crate::editor::TerrainSettings,
    pub curtain_settings: crate::editor::CurtainSettings,
    /// The first point of the curtain wall being drawn.
    pub curtain_anchor: Option<Point2>,
    /// Points the alignment guides start from: the last ones the cursor
    /// snapped to (ArchiCAD's and Revit's acquired points).
    pub acquired: Vec<Point2>,
    /// Alignment guides through the cursor, to draw: (from, to).
    pub guides: Vec<(Point2, Point2)>,
    pub railing_settings: crate::editor::RailingSettings,
    pub object_settings: crate::editor::ObjectSettings,
    /// Points of the railing being drawn.
    pub railing_points: Vec<Point2>,
    /// Survey points of the terrain being drawn.
    pub terrain_points: Vec<archisim_core::SurveyPoint>,
    /// Contour lines of the terrain being drawn (indices into
    /// `terrain_points`), before there is a terrain to put them in.
    pub terrain_lines: Vec<Vec<usize>>,
    /// The contour line being drawn with the terrain tool.
    pub terrain_line: Vec<Point2>,
    /// Points of the selected terrain picked in plan (clicked one by one,
    /// or a whole contour line), to give them a height at once.
    pub terrain_pick: Vec<usize>,
    /// The outline of a room drawn by hand.
    pub zone_tool: SlabTool,
    /// The levels dialog is open.
    pub show_levels: bool,
    /// A level waiting for confirmation before being deleted with its
    /// contents.
    pub level_to_delete: Option<LevelId>,
    /// The types and materials dialog.
    pub types: TypesDialog,
    /// Where the door or window tool would place its element now.
    placement: Option<Placement>,
    pub show_reference_lines: bool,
    pub message: Option<Message>,
    /// Snapped point under the cursor, in model metres.
    cursor: Option<Snap>,
    hover: Option<Element>,
    /// The handle of the selection under the cursor, if any.
    hover_grip: Option<Grip>,
    /// Moving, copying or stretching the selection with the mouse.
    pub drag: Option<DragState>,
    /// What the drag would give at the last cursor position: the outlines
    /// that change, or why it cannot be done.
    drag_preview: Option<(Point2, bool, Outlines)>,
    /// Space is held down (magic wand while drawing slabs by corners).
    space_held: bool,
    /// The room the magic wand would take under the cursor.
    wand_preview: Option<(u64, Point2, Contour, RoomFound)>,
    /// Corner where a selection window started.
    window_start: Option<Point2>,
    /// A temporary dimension being typed.
    temp_edit: Option<temp_ui::TempEdit>,
    /// Witness ends of the temporary dimensions moved by hand, for the
    /// selection they were moved with.
    temp_anchors: (BTreeSet<Element>, crate::temp_dims::Anchors),
    /// The witness end being dragged.
    anchor_drag: Option<(crate::temp_dims::Slot, crate::temp_dims::End)>,
    plan: Option<(PlanKey, TextureHandle)>,
    pending: Option<Pending>,
    allow_close: bool,
    fit_requested: bool,
    title: String,
    show_shortcuts: bool,
    show_about: bool,
    /// What the last IFC import brought in and left out, until closed.
    pub import_report: Option<String>,
    /// Last time the plan picture took to draw, in milliseconds.
    pub last_plan_ms: f64,
    /// Screen area of the plan view in the last frame.
    pub canvas: Rect,
    /// Plan or 3D in the central area (what the active tab shows).
    pub mode: ViewMode,
    /// The open views, as tabs; the active one's state is in `view`,
    /// `plan_scale`, `cut_plane` and `ed.level`.
    pub tabs: Vec<ViewTab>,
    pub active_tab: usize,
    /// The plan tab shown last (F2 goes back to it).
    last_plan_tab: usize,
    /// Drawing scale of the active plan (50 means 1:50).
    pub plan_scale: f64,
    /// Cut plane of the active plan above its level, in metres.
    pub cut_plane: f64,
    pub show_grid: bool,
    /// Every plan line as a hairline (ArchiCAD's "Líneas finas"), instead
    /// of its real weight at the drawing scale.
    pub thin_lines: bool,
    /// Which phases of a refurbishment the plans show.
    pub phase_view: archisim_plan::phases::PhaseView,
    /// The level below is drawn in grey under the active plan (and snaps).
    pub show_underlay: bool,
    /// Walls of other levels shown in the active plan (cut by its cut plane,
    /// or of the level below as a reference): they snap too.
    other_walls: Vec<archisim_core::WallId>,
    /// Which snaps are on (bottom bar).
    pub snap_filter: SnapFilter,
    /// Dark theme for the window (the drawing stays white paper).
    pub dark: bool,
    /// The properties palette (left, beside the tools) is shown.
    pub show_properties: bool,
    /// The navigator (right) is shown.
    pub show_navigator: bool,
    /// Pictures of the types for the properties palette.
    thumbs: thumbs::Thumbs,
    /// The toolbox shows icons only.
    pub compact_toolbox: bool,
    /// Width of each side of the drawing: tools and properties on the left,
    /// navigator on the right, so the drawing is centred.
    pub side_width: f32,
    /// Counts messages, to know when the last one appeared.
    message_serial: u64,
    /// The message last aged and when it appeared (seconds).
    message_seen: (u64, f64),
    /// Fonts and theme installed in the context.
    styled: bool,
    pub v3d: View3dState,
    /// The graphics card, when the window has one (never in tests).
    gpu: Option<eframe::egui_wgpu::RenderState>,
}

impl ArchiSimApp {
    pub fn new(ed: Editor) -> Self {
        let wall_settings = default_wall_settings(ed.model());
        let slab_settings = default_slab_settings(ed.model());
        let door_settings = default_opening_settings(ed.model(), true);
        let window_settings = default_opening_settings(ed.model(), false);
        let (column_settings, beam_settings) = default_member_settings(ed.model());
        let roof_settings = roof_ui::default_roof_settings(ed.model());
        let stair_settings = crate::editor::default_stair_settings(ed.model());
        let level = ed.level;
        Self {
            ed,
            view: View::default(),
            tool: Tool::Arrow,
            wall_tool: WallTool::default(),
            wall_settings,
            slab_tool: SlabTool::default(),
            slab_settings,
            door_settings,
            window_settings,
            dim_tool: DimTool::default(),
            dim_settings: DimSettings::default(),
            column_settings,
            beam_settings,
            beam_tool: structure_ui::BeamTool::default(),
            section_tool: section_ui::SectionTool::default(),
            section_cache: None,
            section_hover: None,
            space_settings: Default::default(),
            roof_settings,
            stair_settings,
            stair_tool: Default::default(),
            terrain_settings: Default::default(),
            curtain_settings: Default::default(),
            curtain_anchor: None,
            acquired: Vec::new(),
            guides: Vec::new(),
            railing_settings: Default::default(),
            object_settings: Default::default(),
            railing_points: Vec::new(),
            terrain_points: Vec::new(),
            terrain_lines: Vec::new(),
            terrain_line: Vec::new(),
            terrain_pick: Vec::new(),
            roof_tool: SlabTool::default(),
            zone_tool: SlabTool::default(),
            show_levels: false,
            level_to_delete: None,
            types: TypesDialog::default(),
            placement: None,
            show_reference_lines: false,
            message: None,
            cursor: None,
            hover: None,
            hover_grip: None,
            drag: None,
            drag_preview: None,
            wand_preview: None,
            space_held: false,
            window_start: None,
            temp_edit: None,
            temp_anchors: Default::default(),
            anchor_drag: None,
            plan: None,
            pending: None,
            allow_close: false,
            fit_requested: true,
            title: String::new(),
            show_shortcuts: false,
            show_about: false,
            import_report: None,
            last_plan_ms: 0.0,
            canvas: Rect::NOTHING,
            mode: ViewMode::Plan,
            tabs: vec![ViewTab::plan(level)],
            active_tab: 0,
            last_plan_tab: 0,
            plan_scale: archisim_plan::PlanOptions::default().scale,
            cut_plane: archisim_plan::PlanOptions::default().cut_plane,
            show_grid: true,
            thin_lines: false,
            phase_view: Default::default(),
            show_underlay: true,
            other_walls: Vec::new(),
            snap_filter: SnapFilter::default(),
            dark: false,
            show_properties: true,
            show_navigator: true,
            thumbs: thumbs::Thumbs::default(),
            compact_toolbox: false,
            side_width: SIDE_WIDTH,
            message_serial: 0,
            message_seen: (0, 0.0),
            styled: false,
            v3d: View3dState::default(),
            gpu: None,
        }
    }

    fn say(&mut self, text: impl Into<String>) {
        self.message_serial += 1;
        self.message = Some(Message {
            text: text.into(),
            error: false,
        });
    }

    fn complain(&mut self, text: impl Into<String>) {
        self.message_serial += 1;
        self.message = Some(Message {
            text: text.into(),
            error: true,
        });
    }

    /// Reports the result of an edit in the status bar.
    fn report(&mut self, result: Result<(), String>, done: &str) {
        match result {
            Ok(()) => self.say(done),
            Err(e) => self.complain(format!("No se puede: {e}.")),
        }
    }

    fn set_tool(&mut self, tool: Tool) {
        if tool != self.tool {
            self.wall_tool.finish();
            self.slab_tool.finish();
            self.dim_tool.finish();
            self.beam_tool.finish();
            self.zone_tool.finish();
            self.roof_tool.finish();
            self.stair_tool.points.clear();
            self.terrain_points.clear();
            self.terrain_lines.clear();
            self.terrain_line.clear();
            self.section_tool.anchor = None;
            self.curtain_anchor = None;
            self.railing_points.clear();
            self.guides.clear();
            self.placement = None;
            self.window_start = None;
            self.drag = None;
            self.temp_edit = None;
            self.tool = tool;
        }
        // Drawing happens in plan.
        if tool != Tool::Arrow {
            self.show_plan();
        }
    }

    fn replace_editor(&mut self, ed: Editor) {
        self.wall_settings = default_wall_settings(ed.model());
        self.slab_settings = default_slab_settings(ed.model());
        self.door_settings = default_opening_settings(ed.model(), true);
        self.window_settings = default_opening_settings(ed.model(), false);
        (self.column_settings, self.beam_settings) = default_member_settings(ed.model());
        self.roof_settings = roof_ui::default_roof_settings(ed.model());
        self.stair_settings = crate::editor::default_stair_settings(ed.model());
        self.beam_tool.finish();
        self.ed = ed;
        self.drag = None;
        self.wall_tool.finish();
        self.slab_tool.finish();
        self.plan = None;
        self.fit_requested = true;
        self.v3d.reset();
        // A project opened starts with its lowest plan, as a new window.
        self.tabs = vec![ViewTab::plan(self.ed.level)];
        self.active_tab = 0;
        self.last_plan_tab = 0;
        self.mode = ViewMode::Plan;
        self.plan_scale = self.tabs[0].scale;
        self.cut_plane = self.tabs[0].cut_plane;
    }

    /// A tool key: M muro, F forjado, P puerta, V ventana, C cota, N cota
    /// de nivel. Returns whether it was one.
    fn tool_key(&mut self, text: &str) -> bool {
        let pick = match text.to_lowercase().as_str() {
            "n" => Some(toolbox::Pick::LevelMark),
            t => Tool::from_key(t).map(toolbox::Pick::Tool),
        };
        match pick {
            Some(p) => {
                self.pick(p);
                true
            }
            None => false,
        }
    }

    // ---- Files -------------------------------------------------------------

    fn start_dir(&self) -> Option<PathBuf> {
        self.ed
            .path
            .as_ref()
            .or(self.ed.imported_from.as_ref())
            .and_then(|p| p.parent())
            .map(Path::to_path_buf)
    }

    fn file_name_stem(&self) -> String {
        self.ed
            .path
            .as_ref()
            .or(self.ed.imported_from.as_ref())
            .and_then(|p| p.file_stem())
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_else(|| "proyecto".into())
    }

    /// Runs an action that would discard changes, asking first if needed.
    fn guarded(&mut self, action: Pending) {
        if self.ed.is_modified() {
            self.pending = Some(action);
        } else {
            self.run_pending(action);
        }
    }

    fn run_pending(&mut self, action: Pending) {
        match action {
            Pending::New => {
                self.replace_editor(Editor::new_project());
                self.say("Proyecto nuevo con la plantilla de ArchiSim.");
            }
            Pending::Open(path) => {
                let path = path.or_else(|| {
                    let mut d = rfd::FileDialog::new()
                        .set_title("Abrir proyecto")
                        .add_filter("Proyecto ArchiSim", &[archisim_file::EXTENSION]);
                    if let Some(dir) = self.start_dir() {
                        d = d.set_directory(dir);
                    }
                    d.pick_file()
                });
                if let Some(path) = path {
                    self.open_path(&path);
                }
            }
            Pending::Import(path) => {
                let path = path.or_else(|| {
                    let mut d = rfd::FileDialog::new()
                        .set_title("Importar IFC")
                        .add_filter("IFC", &["ifc", "IFC"]);
                    if let Some(dir) = self.start_dir() {
                        d = d.set_directory(dir);
                    }
                    d.pick_file()
                });
                if let Some(path) = path {
                    self.import_path(&path);
                }
            }
            Pending::Quit => {
                self.allow_close = true;
            }
        }
    }

    /// Files dropped on the window and, on macOS, opened from the Finder or
    /// dropped on the Dock icon: a project is opened, an IFC imported, as
    /// with the File menu (asking first if there are unsaved changes).
    fn receive_files(&mut self, ctx: &egui::Context) {
        let paths: Vec<PathBuf> = ctx.input(|i| {
            i.raw
                .dropped_files
                .iter()
                .map(|f| f.path().to_path_buf())
                .filter(|p| !p.as_os_str().is_empty())
                .collect()
        });
        // Tests run in parallel and never read the operating system's queue.
        #[cfg(not(test))]
        let paths = [paths, crate::open_files::take()].concat();
        self.open_files(paths);
        #[cfg(all(target_os = "macos", not(test)))]
        crate::open_files::macos::quit_for_check(ctx);
    }

    /// Opens the first project or IFC file of a list (one at a time: a
    /// project replaces the open one).
    pub fn open_files(&mut self, paths: Vec<PathBuf>) {
        if paths.is_empty() {
            return;
        }
        let mut openable = paths.iter().filter(|p| crate::open_files::is_openable(p));
        let Some(path) = openable.next().cloned() else {
            self.complain("ArchiSim abre proyectos .archisim y ficheros .ifc.");
            return;
        };
        let rest = openable.count();
        let is_ifc = path
            .extension()
            .is_some_and(|e| e.eq_ignore_ascii_case("ifc"));
        self.guarded(if is_ifc {
            Pending::Import(Some(path))
        } else {
            Pending::Open(Some(path))
        });
        if rest > 0 && self.message.as_ref().is_some_and(|m| !m.error) {
            let text = self
                .message
                .as_ref()
                .map(|m| m.text.clone())
                .unwrap_or_default();
            self.say(format!(
                "{text} Se abre un fichero cada vez: {rest} más sin abrir."
            ));
        }
    }

    /// Imports an IFC file as a new project and shows what came in.
    pub fn import_path(&mut self, path: &Path) {
        match Editor::import(path) {
            Ok((ed, report, library)) => {
                self.replace_editor(ed);
                let mut text = report.to_string();
                if library {
                    text.push_str(
                        "\nSe ha añadido la plantilla de ArchiSim porque el fichero no trae tipos de todas las herramientas.\n",
                    );
                }
                self.import_report = Some(text);
                let name = path.file_name().map_or_else(
                    || path.display().to_string(),
                    |n| n.to_string_lossy().into_owned(),
                );
                self.say(format!("Importado {name}: {}.", report.summary()));
            }
            Err(e) => self.complain(format!("No se puede importar {}: {e}.", path.display())),
        }
    }

    pub fn open_path(&mut self, path: &Path) {
        match Editor::open(path) {
            Ok(ed) => {
                self.replace_editor(ed);
                self.say(format!("Abierto {}.", path.display()));
            }
            Err(e) => self.complain(format!("No se puede abrir {}: {e}.", path.display())),
        }
    }

    /// Saves; with `ask` or without a file yet, asks where. Returns whether
    /// the project ended up saved.
    fn save(&mut self, ask: bool) -> bool {
        let target = if ask || self.ed.path.is_none() {
            let mut d = rfd::FileDialog::new()
                .set_title("Guardar proyecto")
                .add_filter("Proyecto ArchiSim", &[archisim_file::EXTENSION])
                .set_file_name(format!(
                    "{}.{}",
                    self.file_name_stem(),
                    archisim_file::EXTENSION
                ));
            if let Some(dir) = self.start_dir() {
                d = d.set_directory(dir);
            }
            match d.save_file() {
                Some(p) => Some(p),
                None => return false,
            }
        } else {
            None
        };
        match self.ed.save(target.as_deref()) {
            Ok(p) => {
                self.say(format!("Guardado en {}.", p.display()));
                true
            }
            Err(e) => {
                self.complain(format!("No se pudo guardar: {e}."));
                false
            }
        }
    }

    fn export_ifc(&mut self) {
        let mut d = rfd::FileDialog::new()
            .set_title("Exportar IFC4")
            .add_filter("IFC", &["ifc"])
            .set_file_name(format!("{}.ifc", self.file_name_stem()));
        if let Some(dir) = self.start_dir() {
            d = d.set_directory(dir);
        }
        if let Some(path) = d.save_file() {
            match self.ed.export_ifc(&path) {
                Ok(()) => self.say(format!("IFC4 exportado en {}.", path.display())),
                Err(e) => self.complain(format!("No se pudo exportar: {e}.")),
            }
        }
    }

    fn export_plan(&mut self, scale: f64) {
        let level = self
            .ed
            .model()
            .level(self.ed.level)
            .map(|l| l.name.clone())
            .unwrap_or_default();
        let mut d = rfd::FileDialog::new()
            .set_title(format!("Exportar planta 1:{scale} (SVG)"))
            .add_filter("SVG", &["svg"])
            .set_file_name(format!(
                "{} - {} 1-{scale}.svg",
                self.file_name_stem(),
                level
            ));
        if let Some(dir) = self.start_dir() {
            d = d.set_directory(dir);
        }
        if let Some(path) = d.save_file() {
            match self.ed.export_plan_phase(
                &path,
                self.show_reference_lines,
                scale,
                self.phase_view,
            ) {
                Ok(()) => self.say(format!("Planta exportada en {}.", path.display())),
                Err(e) => self.complain(format!("No se pudo exportar: {e}.")),
            }
        }
    }

    // ---- Commands ------------------------------------------------------------

    fn undo(&mut self) {
        self.drag = None;
        self.temp_edit = None;
        self.wall_tool.finish();
        self.slab_tool.finish();
        match self.ed.undo() {
            Some(l) => self.say(format!("Deshecho: {l}.")),
            None => self.say("No hay nada que deshacer."),
        }
    }

    fn redo(&mut self) {
        self.drag = None;
        self.temp_edit = None;
        self.wall_tool.finish();
        self.slab_tool.finish();
        match self.ed.redo() {
            Some(l) => self.say(format!("Rehecho: {l}.")),
            None => self.say("No hay nada que rehacer."),
        }
    }

    fn delete(&mut self) {
        if self.delete_terrain_points() {
            return;
        }
        match self.ed.delete_selection() {
            Ok(0) => {}
            Ok(n) => self.say(format!("Borrados {n} elemento(s).")),
            Err(e) => self.complain(format!("No se puede borrar: {e}.")),
        }
    }

    fn select_all(&mut self) {
        self.ed.selection = crate::pick::elements_on_level(self.ed.model(), self.ed.level)
            .into_iter()
            .collect();
    }

    /// Builds the walls asked for by the wall tool.
    fn build_walls(&mut self, pieces: Vec<Piece>) {
        match self.ed.add_walls(&self.wall_settings, &pieces) {
            Ok(ids) => {
                self.wall_tool.accept(&pieces);
                // The last walls drawn stay selected, with their temporary
                // dimensions once the chain ends (as Revit does).
                self.ed.selection = ids.iter().map(|id| Element::Wall(*id)).collect();
                let message = match (self.wall_tool.mode, pieces.as_slice()) {
                    (WallMode::Rectangle, [a, b, ..]) => format!(
                        "4 muros creados ({} × {} mm).",
                        fmt_mm(a.length()),
                        fmt_mm(b.length())
                    ),
                    (WallMode::Circle, [a, ..]) => format!(
                        "Muro circular de radio {} mm creado (dos medios círculos).",
                        fmt_mm(a.arc().map_or(0.0, |c| c.radius))
                    ),
                    (_, [w]) => match w.arc() {
                        Some(arc) => format!(
                            "Muro curvo de {} mm (radio {} mm) creado.",
                            fmt_mm(w.length()),
                            fmt_mm(arc.radius)
                        ),
                        None => format!("Muro de {} mm creado.", fmt_mm(w.length())),
                    },
                    _ => format!("{} muros creados.", pieces.len()),
                };
                self.say(message);
                let chains = matches!(self.wall_tool.mode, WallMode::Chain | WallMode::Arc);
                if chains && !self.wall_tool.is_drawing() {
                    self.say("Contorno cerrado.");
                }
            }
            Err(e) => self.complain(format!("No se puede crear el muro: {e}.")),
        }
    }

    /// Builds the slab closed by the slab tool.
    fn build_slab(&mut self, outline: Vec<Point2>) {
        let s = self.slab_settings;
        if s.hole {
            let area = archisim_core::signed_area(&outline).abs();
            match self.ed.add_slab_hole(outline) {
                Ok(_) => {
                    self.slab_tool.accept();
                    self.say(format!("Hueco de {} en el forjado.", fmt_area(area)));
                }
                Err(e) => self.complain(format!("No se puede hacer el hueco: {e}.")),
            }
            return;
        }
        match self.ed.add_slab(s.slab_type, outline, s.top_offset) {
            Ok(id) => {
                self.slab_tool.accept();
                let area = self.ed.model().slab(id).map_or(0.0, |s| s.area());
                self.say(format!("Forjado de {} creado.", fmt_area(area)));
            }
            Err(e) => self.complain(format!("No se puede crear el forjado: {e}.")),
        }
    }

    /// Builds the slab of the room around `p` (magic wand).
    fn build_slab_in_room(&mut self, p: Point2) {
        let s = self.slab_settings;
        match self
            .ed
            .add_slab_in_room(s.slab_type, p, s.contour, s.top_offset)
        {
            Ok(id) => {
                let area = self.ed.model().slab(id).map_or(0.0, |s| s.area());
                self.say(format!(
                    "Forjado de {} creado con la varita mágica.",
                    fmt_area(area)
                ));
            }
            Err(e) => self.complain(format!("No se puede crear el forjado: {e}.")),
        }
    }

    /// Drops the dragged selection at `cursor` (or at the typed distance).
    fn finish_drag(&mut self, cursor: Point2, variant: bool) {
        let Some(d) = self.drag.take() else {
            return;
        };
        self.drag_preview = None;
        let to = d.target(cursor);
        if to.approx_eq(d.from) {
            return;
        }
        let drag = Drag {
            grip: d.grip,
            from: d.from,
            to,
            variant,
        };
        let dist = fmt_mm(d.from.distance(to));
        match self.ed.drag(&drag) {
            Ok(label) => self.say(format!("{label}: {dist} mm.")),
            Err(e) => self.complain(format!("No se puede: {e}.")),
        }
    }

    /// Places the door or window shown under the cursor.
    fn place_opening(&mut self) {
        let Some(pl) = self.placement.clone() else {
            let reach = self.view.to_metres(REACH);
            let curved = self.cursor.and_then(|c| {
                crate::opening_tool::wall_at(&self.ed, self.ed.level, c.point, reach)
            });
            if curved.is_some_and(|w| self.ed.model().wall(w).is_some_and(|w| w.is_curved())) {
                self.complain("Los muros curvos aún no admiten puertas ni ventanas.");
            } else {
                self.complain("Haz clic sobre un muro para colocar la puerta o ventana.");
            }
            return;
        };
        if let Some(problem) = pl.problem {
            self.complain(format!("No cabe ahí: {problem}."));
            return;
        }
        let door = self.tool == Tool::Door;
        match self.ed.add_opening(pl.spec) {
            Ok(id) => {
                // Selected, with its temporary dimensions and arrows, to
                // place it exactly.
                self.ed.selection = [Element::Opening(id)].into_iter().collect();
                self.say(if door {
                    "Puerta colocada: clic en una cota azul para ajustarla; Espacio la gira."
                } else {
                    "Ventana colocada: clic en una cota azul para ajustarla; Espacio la gira."
                })
            }
            Err(e) => self.complain(format!("No se puede colocar: {e}.")),
        }
    }

    // ---- Frame -------------------------------------------------------------

    /// Draws the whole window. Called by eframe every frame (and by tests).
    pub fn show(&mut self, ui: &mut Ui) {
        let ctx = ui.ctx().clone();
        if !self.styled {
            self.styled = true;
            theme::install_fonts(&ctx);
            theme::apply(&ctx, self.dark);
        }
        self.handle_close(&ctx);
        self.handle_shortcuts(&ctx);
        self.keep_tool_types_valid();
        self.receive_files(&ctx);
        self.sync_tabs();
        self.update_title(&ctx);
        self.age_message(ctx.input(|i| i.time));
        let p = theme::Palette::current(&ctx);

        let bar = |fill| {
            egui::Frame::new()
                .fill(fill)
                .inner_margin(egui::Margin::symmetric(10, 3))
                .stroke(Stroke::new(1.0, p.line))
        };
        egui::Panel::top("top-bar")
            .frame(bar(p.panel))
            .show(ui, |ui| self.top_bar(ui));
        egui::Panel::bottom("bottom-bar")
            .frame(bar(p.panel))
            .show(ui, |ui| self.bottom_bar(ui));
        // The drawing stays in the middle of the window: the column on the
        // right (navigator) is as wide as the two on the left (tools and
        // properties). Dragging the edge of Properties widens both sides.
        let tools_w = self.toolbox_width();
        egui::Panel::left("tools")
            .resizable(false)
            .exact_size(tools_w)
            .frame(
                egui::Frame::new()
                    .fill(p.panel)
                    .inner_margin(egui::Margin::symmetric(6, 0))
                    .stroke(Stroke::new(1.0, p.line)),
            )
            .show(ui, |ui| self.toolbox(ui));
        let column = || {
            egui::Frame::new()
                .fill(p.panel)
                .inner_margin(egui::Margin::symmetric(10, 0))
                .stroke(Stroke::new(1.0, p.line))
        };
        // Properties beside the tools, as Revit's palette: choosing a tool
        // and its type is a short move of the mouse.
        if self.show_properties {
            let shown = egui::Panel::left("properties")
                .resizable(true)
                .default_size(SIDE_WIDTH - tools_w)
                .min_size(MIN_SIDE_WIDTH - tools_w)
                .max_size(MAX_SIDE_WIDTH - tools_w)
                .frame(column())
                .show(ui, |ui| {
                    egui::ScrollArea::vertical()
                        .id_salt("properties")
                        .auto_shrink([false, false])
                        .show(ui, |ui| {
                            self.properties(ui);
                            ui.add_space(12.0);
                        });
                });
            self.side_width = tools_w + shown.response.rect.width();
        }
        if self.show_navigator {
            let panel = egui::Panel::right("navigator").frame(column());
            let panel = if self.show_properties {
                // Mirrors the left side.
                panel.resizable(false).exact_size(self.side_width)
            } else {
                panel
                    .resizable(true)
                    .default_size(SIDE_WIDTH)
                    .min_size(MIN_SIDE_WIDTH)
                    .max_size(MAX_SIDE_WIDTH)
            };
            panel.show(ui, |ui| self.navigator(ui));
        }
        egui::CentralPanel::no_frame().show(ui, |ui| {
            ui.spacing_mut().item_spacing.y = 0.0;
            self.tab_strip(ui);
            match self.mode {
                ViewMode::Plan => self.plan_view(ui),
                ViewMode::ThreeD => self.view_3d(ui),
                ViewMode::Section => self.section_view(ui),
            }
        });

        if let Some(text) = ctx.data_mut(|d| d.remove_temp::<String>(Id::unique("length-error"))) {
            self.complain(format!(
                "«{text}» no es una medida válida: escribe milímetros (2700) o con unidad (2,7m · 270cm)."
            ));
        }
        self.dialogs(&ctx);
        self.thumbs.end_frame();
    }

    fn handle_close(&mut self, ctx: &egui::Context) {
        // Cmd+Q, «Salir» and closing the session (macOS) are handled like
        // the window's close button.
        let quit = crate::open_files::take_quit();
        if (quit || ctx.input(|i| i.viewport().close_requested())) && !self.allow_close {
            if self.ed.is_modified() {
                ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
                self.pending = Some(Pending::Quit);
            } else {
                self.allow_close = true;
            }
        }
        if self.allow_close && self.pending.is_none() {
            // macOS: leave at once, with the window still open. Letting
            // winit tear the window down crashes AppKit on Macs with a
            // Touch Bar (its Touch Bar lookup removes an observer from the
            // closed window and throws), and macOS then reports a crash.
            // Nothing is lost: the project was saved or discarded above.
            #[cfg(all(target_os = "macos", not(test)))]
            std::process::exit(0);
            #[allow(unreachable_code)]
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }
    }

    fn handle_shortcuts(&mut self, ctx: &egui::Context) {
        let cmd = Modifiers::COMMAND;
        let cmd_shift = Modifiers::COMMAND | Modifiers::SHIFT;
        let typing = ctx.memory(|m| m.focused().is_some()) || self.temp_edit.is_some();
        let consume = |m: Modifiers, k: Key| {
            ctx.input_mut(|i| i.consume_shortcut(&KeyboardShortcut::new(m, k)))
        };
        if consume(cmd_shift, Key::S) {
            self.save(true);
        } else if consume(cmd, Key::S) {
            self.save(false);
        }
        if consume(cmd, Key::N) {
            self.guarded(Pending::New);
        }
        if consume(cmd, Key::O) {
            self.guarded(Pending::Open(None));
        }
        if typing {
            return;
        }
        if consume(cmd_shift, Key::Z) || consume(cmd, Key::Y) {
            self.redo();
        } else if consume(cmd, Key::Z) {
            self.undo();
        }
        if consume(cmd, Key::A) {
            self.set_tool(Tool::Arrow);
            self.select_all();
        }
        if consume(Modifiers::NONE, Key::Home) {
            self.fit_active_view();
        }
        if consume(Modifiers::CTRL, Key::Tab) && !self.tabs.is_empty() {
            self.activate_tab((self.active_tab + 1) % self.tabs.len());
        }
        if consume(Modifiers::NONE, Key::F2) {
            self.set_mode(ViewMode::Plan);
        }
        if consume(Modifiers::NONE, Key::F3) {
            self.set_mode(ViewMode::ThreeD);
        }
        if matches!(self.mode, ViewMode::ThreeD | ViewMode::Section) {
            self.keyboard_in_3d(ctx);
        }
    }

    /// Keys while the 3D view or a section is shown: Esc, Supr, and the tool keys,
    /// which go back to the plan to draw.
    fn keyboard_in_3d(&mut self, ctx: &egui::Context) {
        let (events, escape, delete) = ctx.input(|i| {
            (
                i.events.clone(),
                i.key_pressed(Key::Escape),
                i.key_pressed(Key::Delete) || i.key_pressed(Key::Backspace),
            )
        });
        if escape {
            self.ed.selection.clear();
        }
        if delete {
            self.delete();
        }
        for e in &events {
            if let egui::Event::Text(t) = e {
                self.tool_key(t);
            }
        }
    }

    fn export_3d_picture(&mut self) {
        let Some((w, h, px)) = self.v3d.picture() else {
            self.complain(
                "No hay imagen 3D que guardar (la vista 3D necesita la tarjeta gráfica).",
            );
            return;
        };
        let mut d = rfd::FileDialog::new()
            .set_title("Guardar imagen 3D")
            .add_filter("PNG", &["png"])
            .set_file_name(format!("{} - 3D.png", self.file_name_stem()));
        if let Some(dir) = self.start_dir() {
            d = d.set_directory(dir);
        }
        let Some(path) = d.save_file() else {
            return;
        };
        let saved = resvg::tiny_skia::IntSize::from_wh(w, h)
            .and_then(|size| resvg::tiny_skia::Pixmap::from_vec(px, size))
            .map(|pixmap| pixmap.save_png(&path));
        match saved {
            Some(Ok(())) => self.say(format!("Imagen 3D guardada en {}.", path.display())),
            _ => self.complain("No se pudo guardar la imagen 3D."),
        }
    }

    fn toolbox_width(&self) -> f32 {
        if self.compact_toolbox {
            TOOLBOX_COMPACT_WIDTH
        } else {
            TOOLBOX_WIDTH
        }
    }

    fn update_title(&mut self, ctx: &egui::Context) {
        let title = format!(
            "{}{} — {} — ArchiSim",
            if self.ed.is_modified() { "● " } else { "" },
            self.ed.display_name(),
            self.ed.model().project_name
        );
        if title != self.title {
            ctx.send_viewport_cmd(egui::ViewportCommand::Title(title.clone()));
            self.title = title;
        }
    }

    // ---- Dialogs ---------------------------------------------------------------

    fn dialogs(&mut self, ctx: &egui::Context) {
        if let Some(action) = self.pending.clone() {
            let name = self.ed.display_name();
            let mut choice = None;
            egui::Modal::new(Id::unique("unsaved")).show(ctx, |ui| {
                ui.set_width(380.0);
                ui.heading("Cambios sin guardar");
                ui.add_space(6.0);
                ui.label(format!(
                    "«{name}» tiene cambios sin guardar. ¿Quieres guardarlos?"
                ));
                ui.add_space(10.0);
                ui.horizontal(|ui| {
                    if ui.button("Guardar").clicked() {
                        choice = Some(0);
                    }
                    if ui.button("No guardar").clicked() {
                        choice = Some(1);
                    }
                    if ui.button("Cancelar").clicked() {
                        choice = Some(2);
                    }
                });
            });
            match choice {
                Some(0) => {
                    self.pending = None;
                    if self.save(false) {
                        self.run_pending(action);
                    }
                }
                Some(1) => {
                    self.pending = None;
                    self.run_pending(action);
                }
                Some(_) => self.pending = None,
                None => {}
            }
        }
        if self.show_shortcuts {
            egui::Window::new("Ratón y teclado")
                .open(&mut self.show_shortcuts)
                .resizable(false)
                .show(ctx, |ui| {
                    egui::Grid::new("keys")
                        .num_columns(2)
                        .spacing([16.0, 4.0])
                        .show(ui, |ui| {
                            for (k, v) in SHORTCUTS {
                                ui.strong(*k);
                                ui.label(*v);
                                ui.end_row();
                            }
                        });
                });
        }
        if let Some(text) = &self.import_report {
            let mut open = true;
            let mut close = false;
            egui::Window::new("Informe de importación")
                .open(&mut open)
                .default_width(520.0)
                .resizable(true)
                .show(ctx, |ui| {
                    egui::ScrollArea::vertical()
                        .max_height(420.0)
                        .show(ui, |ui| {
                            ui.label(egui::RichText::new(text.as_str()).monospace());
                        });
                    ui.add_space(6.0);
                    if ui.button("Cerrar").clicked() {
                        close = true;
                    }
                });
            if !open || close {
                self.import_report = None;
            }
        }
        self.levels_dialog(ctx);
        self.types_dialog(ctx);
        if self.show_about {
            egui::Window::new("Acerca de ArchiSim")
                .open(&mut self.show_about)
                .resizable(false)
                .show(ctx, |ui| {
                    ui.heading(format!("ArchiSim {}", env!("CARGO_PKG_VERSION")));
                    ui.label("Herramienta BIM de autoría. IFC4 como modelo de datos.");
                    ui.label("Versión de desarrollo · AD5");
                });
        }
    }
}

impl eframe::App for ArchiSimApp {
    fn ui(&mut self, ui: &mut Ui, frame: &mut eframe::Frame) {
        if self.gpu.is_none() {
            self.gpu = frame.wgpu_render_state().cloned();
        }
        self.show(ui);
    }
}

const SHORTCUTS: &[(&str, &str)] = &[
    ("F2 · F3", "Planta · 3D"),
    (
        "3D: arrastrar (o Mayús + rueda)",
        "Girar alrededor del modelo",
    ),
    ("3D: clic", "Seleccionar (también Mayús+clic)"),
    ("Rueda", "Zoom en el cursor"),
    ("Arrastrar rueda / botón derecho", "Encuadre"),
    ("Doble clic rueda · Inicio", "Ajustar a la ventana"),
    (
        "M · F · P · V · C · N",
        "Muro · Forjado · Puerta · Ventana · Cota · Cota de nivel",
    ),
    ("Ctrl+Tab", "Siguiente pestaña de vista"),
    ("Clic con la rueda en una pestaña", "Cerrar la vista"),
    ("Esc", "Terminar / herramienta flecha / anular selección"),
    (
        "Dibujando",
        "Muros rectos por defecto (línea azul); sal del eje más de 8° para inclinarlo",
    ),
    ("Mayús (dibujando)", "Ángulos múltiplos de 45°"),
    (
        "Números + Intro (dibujando)",
        "Longitud del muro en mm (o 3,5m · 350cm)",
    ),
    (
        "Intro / doble clic / botón derecho",
        "Terminar la cadena de muros",
    ),
    (
        "Método (Propiedades del muro)",
        "Recto · Cadena · Rectángulo (dos esquinas) · Arco (inicio, fin y punto de paso) · Círculo (centro y punto)",
    ),
    (
        "Números + Intro (arco o círculo)",
        "Radio en mm; en un rectángulo, ancho x fondo (6000x4000)",
    ),
    (
        "Método (Propiedades del forjado)",
        "Polígono · Rectángulo (dos esquinas) · Varita mágica",
    ),
    (
        "Círculo blanco (muro curvo)",
        "Arrastrar: curvarlo por ese punto; en línea con sus extremos, queda recto",
    ),
    (
        "Clic en una cota azul",
        "Escribir la medida exacta (muro, puerta o ventana seleccionados)",
    ),
    (
        "Espacio (puerta o ventana)",
        "Girarla: otra jamba, otro lado… (también las flechas azules)",
    ),
    (
        "Espacio (muro seleccionado)",
        "Invertir: la cara exterior al otro lado",
    ),
    ("Mayús+clic", "Añadir o quitar de la selección"),
    ("Supr", "Borrar la selección"),
    ("Ctrl+Z · Ctrl+Y", "Deshacer · Rehacer"),
    ("Ctrl+A", "Seleccionar todo en la planta"),
    ("Ctrl+N · Ctrl+O · Ctrl+S", "Nuevo · Abrir · Guardar"),
];

fn default_reference(function: WallFunction) -> ReferenceLine {
    match function {
        WallFunction::Exterior => ReferenceLine::ExteriorFace,
        WallFunction::Interior => ReferenceLine::Center,
    }
}

/// Wall tool settings for a model: its first exterior type, 3 m high.
pub fn default_wall_settings(model: &Model) -> WallSettings {
    let t = model
        .wall_types()
        .find(|t| t.function == WallFunction::Exterior)
        .or_else(|| model.wall_types().next());
    WallSettings {
        wall_type: t.map_or(WallTypeId(0), |t| t.id),
        reference: t.map_or(ReferenceLine::Center, |t| default_reference(t.function)),
        height: 3.0,
        base_offset: 0.0,
        // Up to the floor above, as walls usually are; free 3 m when the
        // project has no level above.
        top: crate::editor::TopChoice::NextLevel,
        top_offset: 0.0,
    }
}

/// Slab tool settings for a model: its first type, top face on the level.
/// Column and beam tools start with the first type of each (the library's
/// concrete column and beam), columns up to the level above.
pub fn default_member_settings(model: &Model) -> (ColumnSettings, BeamSettings) {
    use archisim_core::MemberUsage;
    (
        ColumnSettings {
            member_type: crate::editor::default_member_type(model, MemberUsage::Column),
            rotation: 0.0,
            base_offset: 0.0,
            height: 3.0,
            top: crate::editor::TopChoice::NextLevel,
            top_offset: 0.0,
        },
        BeamSettings {
            member_type: crate::editor::default_member_type(model, MemberUsage::Beam),
            level_above: true,
            top_offset: 0.0,
        },
    )
}

pub fn default_slab_settings(model: &Model) -> SlabSettings {
    // A new project starts on the ground floor: a ground-bearing slab.
    let t = model
        .slab_types()
        .find(|t| t.function == SlabFunction::BaseSlab)
        .or_else(|| model.slab_types().next());
    SlabSettings {
        slab_type: t.map_or(SlabTypeId(0), |t| t.id),
        top_offset: 0.0,
        wand: false,
        contour: Contour::InteriorFace,
        hole: false,
    }
}

/// Door or window tool settings: the first type of that kind; doors from
/// the floor, windows with a 900 mm sill; frame centred in the wall.
pub fn default_opening_settings(model: &Model, door: bool) -> OpeningSettings {
    let t = model.opening_types().find(|t| t.is_door() == door);
    OpeningSettings {
        opening_type: t.map_or(OpeningTypeId(0), |t| t.id),
        sill: if door { 0.0 } else { 0.9 },
        hand: Hand::Left,
        setback: None,
    }
}

fn fmt_level(elevation: f64) -> String {
    if elevation.abs() < 0.0005 {
        "±0,00".into()
    } else {
        format!("{:+.2}", elevation).replace('.', ",")
    }
}

fn shortcut_button<'a>(text: &'a str, shortcut: &'a str) -> egui::Button<'a> {
    egui::Button::new(text).shortcut_text(shortcut)
}
fn dashed(painter: &egui::Painter, a: Pos2, b: Pos2, color: Color32) {
    dashed_path(painter, &[a, b], color);
}

/// A dashed polyline (the drawn line of a curved wall), with the dashes
/// running on from one piece to the next.
fn dashed_path(painter: &egui::Painter, pts: &[Pos2], color: Color32) {
    painter.extend(Shape::dashed_line(pts, Stroke::new(1.0, color), 8.0, 4.0));
}

/// A text field that edits a length in millimetres and only reports a value
/// when the user presses Enter or leaves the field (Escape cancels), so
/// each change is one undo step.
/// How each leaf of a window opens, for families with one setting per leaf
/// ("3 hojas: fija · oscilobatiente · fija"); `None` for the others.
fn leaves_summary(t: &archisim_core::OpeningType) -> Option<String> {
    let n = t.values.get("leaves")?.as_number()? as usize;
    let ops: Vec<&str> = (1..=n)
        .map(|i| t.values.get(&format!("leaf_{i}"))?.as_text())
        .collect::<Option<_>>()?;
    Some(if n == 1 {
        let mut c = ops[0].chars();
        c.next()
            .map(|f| f.to_uppercase().chain(c).collect())
            .unwrap_or_default()
    } else {
        format!("{n} hojas: {}", ops.join(" · "))
    })
}

pub fn length_field(ui: &mut Ui, id: Id, value: f64, width: f32) -> Option<f64> {
    let text = text_edit_committed(ui, id, &fmt_mm(value), width)?;
    match parse_length(&text) {
        Some(v) => Some(v),
        None => {
            ui.ctx()
                .data_mut(|d| d.insert_temp(Id::unique("length-error"), text));
            None
        }
    }
}

fn text_edit_committed(ui: &mut Ui, id: Id, shown: &str, width: f32) -> Option<String> {
    let buffer_id = id.with("buffer");
    let mut text = ui
        .ctx()
        .data(|d| d.get_temp::<String>(buffer_id))
        .unwrap_or_else(|| shown.to_string());
    let response = ui.add(TextEdit::singleline(&mut text).id(id).desired_width(width));
    if response.has_focus() {
        ui.ctx().data_mut(|d| d.insert_temp(buffer_id, text));
        None
    } else if response.lost_focus() {
        ui.ctx().data_mut(|d| d.remove::<String>(buffer_id));
        let cancelled = ui.input(|i| i.key_pressed(Key::Escape));
        (!cancelled && text != shown).then_some(text)
    } else {
        ui.ctx().data_mut(|d| d.remove::<String>(buffer_id));
        None
    }
}

mod annotation_ui;
mod chrome;
mod curtain_ui;
mod icons;
mod levels_ui;
mod navigator;
mod object_ui;
mod plan_ui;
mod properties;
mod props;
mod railing_ui;
mod roof_ui;
mod section_ui;
mod space_ui;
mod stair_ui;
mod structure_ui;
mod tabs;
mod temp_ui;
mod terrain_ui;
pub mod theme;
mod thumbs;
mod toolbox;
mod type_views;
mod types_ui;
mod view3d_ui;
mod widgets;
pub use tabs::{TabKind, ViewTab};
pub use types_ui::{TypesDialog, TypesTab};
pub use view3d_ui::{View3dState, ViewMode};

#[cfg(test)]
mod tests;
