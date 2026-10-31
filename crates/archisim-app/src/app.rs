//! The ArchiSim window, laid out like ArchiCAD with Revit's properties:
//! the top bar (menus, project, quick commands), the toolbox on the left,
//! the open views as tabs over the drawing, the navigator and the
//! properties palette on the right, and the bottom bar with the scale,
//! zoom, snaps and display of the active view.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use archisim_core::{
    Hand, LevelId, Model, OpeningKind, OpeningTypeId, Point2, ReferenceLine, SectionId,
    SlabFunction, SlabTypeId, WallFunction, WallSide, WallTypeId,
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
/// Height of the command line under the drawing (two lines said and the
/// prompt).
const COMMAND_LINE_HEIGHT: f32 = 62.0;
/// Width of the toolbox, with names and with icons only.
/// The rail of disciplines, on the far left.
const RAIL_WIDTH: f32 = 50.0;

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
    /// Setting-out grid lines, by two points.
    Grid,
    /// Reference planes, by two points.
    RefPlane,
    /// Pads, strips and tie beams.
    Footing,
    /// Copies of a module (another project).
    Module,
    /// Faces, volumes and masses of the concept phase.
    Mass,
    /// Pushing a face or volume up (SketchUp's push/pull).
    PushPull,
    /// Dividing a face of a volume along a line.
    DivideFace,
    /// Twisting and bending a volume.
    Deform,
    /// A parallel contour on a volume.
    MassOffset,
    /// The tape measure: measures and leaves guides.
    Tape,
    /// Gives a volume a material.
    Paint,
    /// Plots, by their boundary.
    Plot,
    /// Pools built in the ground, by the edge of their water.
    Pool,
    /// Roads, by their axis.
    Road,
    /// Plants along a line or over an area.
    Planting,
    /// The Exterior brushes: shape the ground, paint lawn or gravel,
    /// scatter plants (MQ3).
    Brush,
    /// Pipes, by their path.
    Pipe,
    /// Ducts, by their path.
    Duct,
    /// Equipment, lights, sockets and switches, by a click.
    Device,
    /// A zone reserved for team work, by two corners.
    TeamZone,
    /// A 2D drafting or editing command is running (AutoCAD's).
    Cad,
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
                | Tool::Grid
                | Tool::RefPlane
                | Tool::Footing
                | Tool::Module
                | Tool::Mass
                | Tool::PushPull
                | Tool::DivideFace
                | Tool::Deform
                | Tool::MassOffset
                | Tool::Tape
                | Tool::Paint
                | Tool::Plot
                | Tool::Pool
                | Tool::Road
                | Tool::Planting
                | Tool::Pipe
                | Tool::Duct
                | Tool::Device
                | Tool::TeamZone
        )
    }

    /// The tool a key selects (with the ArchiCAD keys): M muro, F forjado,
    /// P puerta, V ventana, C cota, I pilar, G viga, Z local, S sección,
    /// T cubierta, E escalera, R terreno, U muro cortina, B barandilla,
    /// O objeto, X ejes, A cimentación, K módulo, Y parcela, W vial,
    /// J plantación (jardín),
    /// H tubería, D conducto.
    pub(super) fn from_key(text: &str) -> Option<Tool> {
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
            "x" => Some(Tool::Grid),
            "a" => Some(Tool::Footing),
            "k" => Some(Tool::Module),
            "y" => Some(Tool::Plot),
            "w" => Some(Tool::Road),
            "j" => Some(Tool::Planting),
            "h" => Some(Tool::Pipe),
            "d" => Some(Tool::Duct),
            "q" => Some(Tool::RefPlane),
            "pi" => Some(Tool::Brush),
            "ps" => Some(Tool::Pool),
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
    /// What the tool would build, for the 3D view (R21).
    ghost: ghost3d::GhostCache,
    plan_ghost: ghost3d::PlanGhost,
    start: start_ui::StartScreen,
    /// Stretching an object by a handle in the 3D view.
    object_grip: Option<grips3d::ObjectGrip3d>,
    object_grip_hover: Option<grips3d::SizeKey>,
    /// The menu of a right click, while it is open.
    context_menu: Option<context_menu::ContextMenu>,
    /// The material of the object whose colours are open in its bar, and
    /// where its swatch is.
    bar_swatches: Option<String>,
    /// The colour being chosen on the free colour wheel of the swatches,
    /// for the piece it was opened on (applied with its button).
    bar_rgb: Option<(String, [u8; 3])>,
    bar_swatch_at: Option<eframe::egui::Rect>,
    /// A left drag in the 3D view (moving or a selection window).
    drag3d: Option<mouse3d::Drag3d>,
    /// What Cortar or Copiar kept.
    clipboard: Option<context_menu::Clipboard>,
    /// Furniture dragged along the floor in the Maqueta.
    object_move: Option<placing::ObjectMove3d>,
    /// A handle of a Concepto volume being dragged in 3D, and the one
    /// under the cursor.
    mass_grip: Option<mass_grips3d::MassGrip3d>,
    /// The Exterior brush and the stroke being made with it.
    pub(super) brush: crate::brush::BrushSettings,
    brush_stroke: Option<brush_ui::BrushStroke>,
    mass_grip_hover: Option<mass_grips3d::MassKey>,
    /// While something is moved in 3D: the wall faces it is pulled to,
    /// its plan points when taken, and its clear distances now (with the
    /// height they are drawn at).
    move_faces: Vec<crate::wall_faces::Face>,
    move_points: Vec<Point2>,
    move_axis: Point2,
    pub(super) move_clear: Vec<crate::wall_faces::Clearance>,
    move_clear_z: f64,
    render: render_ui::RenderState,
    budget: budget_ui::BudgetState,
    /// The first point of the grid line being drawn.
    pub grid_anchor: Option<Point2>,
    /// The first point of the strip or tie beam being drawn.
    pub footing_anchor: Option<Point2>,
    pub footing_settings: crate::editor::FootingSettings,
    pub module_tool: module_ui::ModuleTool,
    /// The contour of the volume tools, their settings and what the
    /// concept tools are doing.
    pub mass_tool: SlabTool,
    pub mass_settings: crate::editor::MassSettings,
    pub concept: concept_ui::ConceptState,
    /// The boundary of the plot being drawn and the setback of the next.
    pub plot_tool: SlabTool,
    pub plot_setback: f64,
    pub pool_tool: SlabTool,
    pub pool_settings: pool_ui::PoolSettings,
    /// The axis of the road being drawn and what the road tool draws.
    pub road_points: Vec<Point2>,
    pub road_settings: urban_ui::RoadSettings,
    /// The line of the planting being drawn and what the tool plants.
    pub planting_points: Vec<Point2>,
    pub planting_settings: planting_ui::PlantingSettings,
    /// The path of the pipe or duct being drawn.
    pub run_points: Vec<Point2>,
    pub service_settings: services_ui::ServiceSettings,
    /// How the drawing tools take their points (their own way, a spline
    /// or a picked line), and the spline's points.
    pub draw_method: methods_ui::DrawMethod,
    pub spline_points: Vec<Point2>,
    pub module_watch: module_ui::ModuleWatch,
    pub section_tool: section_ui::SectionTool,
    /// The section shown: its drawing (worked out once per revision) and
    /// its picture on screen.
    pub section_cache: Option<section_ui::SectionCache>,
    /// Drawings of the sections seen before, kept for going back to their
    /// tabs without working them out again (most recent last).
    section_stash: Vec<section_ui::SectionCache>,
    /// Pictures of a section and a sheet being drawn aside, and the model
    /// those threads draw from.
    section_pending: Option<background::Pending<section_ui::SectionKey>>,
    sheet_pending: Option<background::Pending<sheet_ui::SheetKey>>,
    shared_model: Option<(u64, std::sync::Arc<archisim_core::Model>)>,
    /// Where the pointer is over the section shown (along, height).
    pub section_hover: Option<Point2>,
    /// Objects, doors or windows being moved in a section or elevation.
    pub section_drag: Option<section_ui::SectionDrag>,
    /// Drawing the rectangle of a detail in a section (E17): the section
    /// and its first corner.
    pub detail_pick: Option<(SectionId, Option<Point2>)>,
    /// A DXF read, waiting for its unit and how its blocks come in.
    pub dxf_import: Option<dxf_import_ui::DxfImport>,
    /// The tile «Armado»: clicks arm what they hit.
    pub rebar_pick: bool,
    pub room_settings: crate::editor::RoomSettings,
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
    /// Part of the library the object tool offers (Vegetación, Mobiliario
    /// urbano); `None`: all of it.
    pub object_scope: Option<object_gallery::ObjectScope>,
    /// The object card being drawn is the tool's (it keeps to its scope).
    scoped_card: bool,
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
    /// A survey point's height being typed where it is written (E5):
    /// the terrain, the point and the text.
    terrain_z_edit: Option<(archisim_core::TerrainId, usize, String)>,
    /// Cut and fill of a terrain's platforms, by model revision (E8).
    earthworks_cache: Option<(u64, archisim_core::TerrainId, (f64, f64))>,
    /// The outline of a room drawn by hand.
    pub zone_tool: SlabTool,
    /// The «Maqueta» environment is on (`docs/MAQUETA.md`).
    pub maquette: bool,
    /// The first time in the Maqueta (it starts with the walls cut away
    /// and the grid).
    maquette_first: bool,
    /// The angle the card's «Girar» turns by, degrees.
    turn_by: f64,
    /// The tab to go back to on leaving the Maqueta.
    maquette_return: Option<usize>,
    /// The Maqueta's catalog: group and theme shown.
    catalog_group: maquette_ui::CatalogGroup,
    catalog_theme: &'static str,
    /// The kind chosen within the theme (None: all).
    catalog_sub: Option<String>,
    /// Properties of the selection in a window (Más…, in the Maqueta).
    maquette_props: bool,
    /// The wall tool's rectangle makes a room: its walls, floor and room.
    pub room_walls: bool,
    /// The ground grid of the 3D view, and clicks on it snapped to it.
    pub ground_grid: bool,
    /// The 2D command running, if any.
    pub cad: Option<Box<dyn crate::cad::Command>>,
    /// Layer, style and the values 2D commands remember.
    pub cad_settings: crate::cad::CadSettings,
    /// The command line under the drawing.
    pub cmdline: cad_ui::CmdLine,
    /// Keys as in AutoCAD (type the command and Enter) or as in ArchiCAD
    /// (M, F, P, V, C, N change tool at once).
    pub keys_autocad: bool,
    /// Revit's two-letter shortcuts (`revit_keys`).
    pub keys_revit: bool,
    /// The text being written for the text command.
    text_draft: Option<String>,
    /// The drawing layers dialog is open.
    pub show_layers: bool,
    /// The levels dialog is open.
    pub show_levels: bool,
    /// The bubble «Nueva planta», while open.
    new_level: Option<levels_ui::NewLevel>,
    /// How the type card of doors and windows shows them.
    opening_card_view: properties::CardView,
    /// The intersection priorities window, whether it lists unused
    /// materials too, and the bar being dragged (material, value).
    pub show_priorities: bool,
    /// The curtain wall types window.
    pub curtain_types: curtain_types_ui::CurtainTypesWindow,
    pub vivero: vivero_ui::ViveroWindow,
    /// The library window.
    pub library: library_ui::LibraryWindow,
    pub params: params_ui::ParamsWindow,
    /// The code check window.
    pub cte: cte_ui::CteWindow,
    /// A topographic survey being imported (its window is open).
    pub survey_import: Option<survey_ui::SurveyImport>,
    /// The whole object library, for the gallery's pictures of the types
    /// the project does not have yet (made the first time it is needed).
    pub object_library: Option<archisim_core::Model>,
    /// The Maqueta's collection: the catalog shows its pieces and puts
    /// them in its colours (`None`: the library as it is).
    pub collection: Option<&'static archisim_library::collections::Collection>,
    /// The library in each collection, for the catalog's pictures (made
    /// the first time one is chosen).
    pub collection_libraries: Vec<(&'static str, archisim_core::Model)>,
    /// Where the lamps of a light point come from in its properties:
    /// `None` the project's, `Some(i)` the collection `i` (G4).
    pub fixture_source: Option<usize>,
    /// «Dividir cara» in 3D: the next click on a face divides it.
    pub divide_face_pick: bool,
    /// Every species of plants of the library, for the gallery.
    pub flora_library: Option<std::sync::Arc<archisim_core::Model>>,
    /// The object types of the library folder (the studio's and the
    /// extended library), for the gallery: read again now and then.
    pub folder_objects: Option<object_gallery::FolderObjects>,
    pub priorities_all: bool,
    pub priority_drag: Option<(archisim_core::MaterialId, u16)>,
    /// A level waiting for confirmation before being deleted with its
    /// contents.
    pub level_to_delete: Option<LevelId>,
    /// The types and materials dialog.
    pub types: TypesDialog,
    /// Where the door or window tool would place its element now.
    placement: Option<Placement>,
    /// Door and window tools: the distance from the end of the wall
    /// («Desde un extremo») and how many («Repartidas»).
    pub place_gap: f64,
    pub place_count: usize,
    pub show_reference_lines: bool,
    pub message: Option<Message>,
    /// Snapped point under the cursor, in model metres.
    cursor: Option<Snap>,
    /// Where the cursor is in the 3D view, while a tool works there.
    cursor3d: Option<tools3d::Cursor3d>,
    /// Pintar takes the paint off instead (Quitar pintura).
    paint_erase: bool,
    hover: Option<Element>,
    /// Every element under the cursor in plan, first the one `hover`
    /// would take; and where Tab was pressed and how many times, to go
    /// through them (as in Revit).
    under_cursor: Vec<Element>,
    tab_pick: Option<(Point2, usize)>,
    /// The handle of the selection under the cursor, if any.
    hover_grip: Option<Grip>,
    /// A vertex picked with a click (Supr takes it away): `vertex_ui`.
    vertex: Option<Grip>,
    /// The contour being sketched (E4), instead of the usual clicks.
    sketch: Option<sketch_ui::SketchState>,
    /// The band and the split being prepared in a wall's properties (E3).
    wall_band: wall_tools_ui::WallBand,
    /// A wall's elevation profile being drawn in its window (E3).
    profile_edit: Option<wall_profile_ui::ProfileEdit>,
    /// A 3D model being brought in (its dialog): `mesh_ui`.
    pub mesh_import: Option<mesh_ui::MeshImport>,
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
    /// When the plan or section view last moved (panning or zooming)
    /// since its picture was last drawn in full: while it moves, the last
    /// picture is moved with it instead of being drawn again.
    view_moved_at: Option<std::time::Instant>,
    /// What [`ArchiSimApp::other_walls`] was worked out for: revision,
    /// level, cut plane and reference level.
    other_walls_key: Option<(u64, LevelId, u64, Option<LevelId>)>,
    /// The sun's shadows of the plan's level, while the model and the
    /// level do not change (worked out once, not at each redraw).
    shadow_cache: Option<(
        u64,
        LevelId,
        std::sync::Arc<Vec<archisim_geom::junctions::Shape>>,
    )>,
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
    /// What the active plan shows (categories, layers, detail).
    pub view_style: archisim_core::ViewStyle,
    /// The graphic standard dialog is open.
    pub show_graphics: bool,
    /// The «Estilos de presentación» window.
    pub presentation: presentation_ui::PresentationUi,
    /// The sheet picture and what it was drawn for.
    sheet_image: Option<(sheet_ui::SheetKey, egui::TextureHandle)>,
    /// The picture of the view for drawing shown (E7).
    drafting_image: Option<(drafting_view_ui::DraftingKey, egui::TextureHandle)>,
    /// The view selected on the sheet shown.
    pub sheet_sel: Option<sheet_ui::Placed>,
    /// The view of the sheet under the cursor.
    sheet_hover: Option<sheet_ui::Placed>,
    /// A view being moved on its sheet.
    sheet_drag: Option<sheet_ui::ViewportDrag>,
    /// Level and scale offered for the next view placed on a sheet.
    /// What «Añadir vista» places: a level's plan or a section.
    sheet_new_view: Option<sheet_ui::ViewChoice>,
    sheet_new_scale: f64,
    /// Schedule and level offered for the next schedule placed on a sheet.
    table_new_kind: archisim_core::ScheduleKind,
    table_new_level: Option<LevelId>,
    /// The schedule window, with its level filter.
    /// The level the schedule tabs list (`None`: the whole building).
    pub schedule_level: Option<LevelId>,
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
    /// The choice of a plan symbol among the drawings of a library DXF.
    pub symbol_picker: Option<symbol_pick::SymbolPicker>,
    /// The 3D pictures of the Maqueta's catalog.
    thumbs3d: thumbs3d::Thumbs3d,
    /// The pictures of the materials' swatches.
    swatches: cards::Swatches,
    /// The command line stays shown (Vista); otherwise it shows only while
    /// an order is typed or running.
    pub pin_command_line: bool,
    /// The discipline worked in greys the rest of the view (Vista).
    pub attenuate: bool,
    /// Disciplines hidden in every view, and those that cannot be
    /// selected (the navigator's eye and lock).
    pub hidden_disciplines: std::collections::BTreeSet<archisim_core::ElementDiscipline>,
    pub locked_disciplines: std::collections::BTreeSet<archisim_core::ElementDiscipline>,
    /// The discipline whose tools are shown.
    pub discipline: toolbox::Discipline,
    /// The tab of the navigator shown.
    pub nav_tab: navigator::NavTab,
    /// The command search, while open: what is typed and the result chosen.
    pub command_search: Option<(String, usize)>,
    /// Width of each side of the drawing: disciplines, tools and properties
    /// on the left, navigator on the right, so the drawing is centred.
    pub side_width: f32,
    /// Counts messages, to know when the last one appeared.
    message_serial: u64,
    /// The message last aged and when it appeared (seconds).
    message_seen: (u64, f64),
    /// The revision a message was said at, when it told of a change that
    /// can be undone (its toast offers «Deshacer»).
    message_undo: Option<u64>,
    /// The revision at the last message.
    said_rev: u64,
    /// Fonts and theme installed in the context.
    styled: bool,
    pub v3d: View3dState,
    /// The graphics card, when the window has one (never in tests).
    gpu: Option<eframe::egui_wgpu::RenderState>,
    /// Linked models, issues, clashes and requirements (phase 4).
    pub coord: crate::coord::Coord,
    pub coord_window: coord_ui::CoordWindow,
    /// The shared project, when working in a team.
    pub team: Option<crate::team::Team>,
    pub team_ui: team_ui::TeamUi,
    /// Preferences of this computer (size of the text, theme, keys,
    /// missions done) and the ones last written.
    pub prefs: crate::prefs::Prefs,
    saved_prefs: crate::prefs::Prefs,
    /// The zoom factor last given to egui (the size of the text).
    applied_zoom: f32,
    /// The onboarding mission in hand.
    pub missions: missions_ui::MissionsUi,
    /// Saved plans and the window «Visibilidad y gráficos».
    pub views: views_ui::ViewsUi,
}

impl ArchiSimApp {
    pub fn new(ed: Editor) -> Self {
        let wall_settings = default_wall_settings(ed.model());
        let slab_settings = default_slab_settings(ed.model());
        let door_settings = default_opening_settings(ed.model(), true);
        let window_settings = default_opening_settings(ed.model(), false);
        let (column_settings, beam_settings) = default_member_settings(ed.model());
        let footing_settings = crate::editor::FootingSettings::new(ed.model());
        let mass_settings = crate::editor::MassSettings::new(ed.model());
        let road_settings = urban_ui::RoadSettings::new(ed.model());
        let roof_settings = roof_ui::default_roof_settings(ed.model());
        let stair_settings = crate::editor::default_stair_settings(ed.model());
        let level = ed.level;
        let prefs = crate::prefs::Prefs::load();
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
            ghost: ghost3d::GhostCache::default(),
            plan_ghost: ghost3d::PlanGhost::default(),
            // The start screen greets the program opened without a file
            // (never in the tests).
            object_grip: None,
            object_grip_hover: None,
            context_menu: None,
            clipboard: None,
            drag3d: None,
            bar_swatches: None,
            bar_rgb: None,
            bar_swatch_at: None,
            object_move: None,
            mass_grip: None,
            brush: Default::default(),
            brush_stroke: None,
            mass_grip_hover: None,
            move_faces: Vec::new(),
            move_points: Vec::new(),
            move_axis: Point2::new(1.0, 0.0),
            move_clear: Vec::new(),
            move_clear_z: 0.0,
            render: Default::default(),
            budget: Default::default(),
            start: start_ui::StartScreen {
                open: !cfg!(test),
                ..Default::default()
            },
            grid_anchor: None,
            footing_anchor: None,
            footing_settings,
            module_tool: Default::default(),
            module_watch: Default::default(),
            mass_tool: SlabTool::default(),
            mass_settings,
            concept: Default::default(),
            plot_tool: SlabTool::default(),
            plot_setback: 3.0,
            pool_tool: SlabTool::default(),
            pool_settings: Default::default(),
            road_points: Vec::new(),
            road_settings,
            planting_points: Vec::new(),
            planting_settings: Default::default(),
            run_points: Vec::new(),
            service_settings: services_ui::ServiceSettings::default(),
            draw_method: Default::default(),
            spline_points: Vec::new(),
            section_tool: section_ui::SectionTool::default(),
            section_cache: None,
            section_stash: Vec::new(),
            section_pending: None,
            sheet_pending: None,
            shared_model: None,
            section_hover: None,
            section_drag: None,
            detail_pick: None,
            dxf_import: None,
            rebar_pick: false,
            room_settings: Default::default(),
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
            object_scope: None,
            scoped_card: false,
            railing_points: Vec::new(),
            terrain_points: Vec::new(),
            terrain_lines: Vec::new(),
            terrain_line: Vec::new(),
            terrain_pick: Vec::new(),
            terrain_z_edit: None,
            earthworks_cache: None,
            roof_tool: SlabTool::default(),
            zone_tool: SlabTool::default(),
            maquette: false,
            maquette_first: true,
            turn_by: 90.0,
            maquette_return: None,
            catalog_group: Default::default(),
            catalog_theme: "Habitaciones",
            catalog_sub: None,
            maquette_props: false,
            room_walls: false,
            ground_grid: false,
            cad: None,
            cad_settings: Default::default(),
            cmdline: Default::default(),
            keys_autocad: prefs.keys_autocad,
            keys_revit: prefs.keys_revit,
            text_draft: None,
            show_layers: false,
            show_levels: false,
            new_level: None,
            opening_card_view: Default::default(),
            show_priorities: false,
            curtain_types: Default::default(),
            vivero: Default::default(),
            library: Default::default(),
            params: Default::default(),
            cte: Default::default(),
            survey_import: None,
            object_library: None,
            collection: None,
            collection_libraries: Vec::new(),
            fixture_source: None,
            divide_face_pick: false,
            flora_library: None,
            folder_objects: None,
            priorities_all: false,
            priority_drag: None,
            level_to_delete: None,
            types: TypesDialog::default(),
            placement: None,
            place_gap: 0.30,
            place_count: 2,
            show_reference_lines: false,
            message: None,
            cursor: None,
            cursor3d: None,
            paint_erase: false,
            hover: None,
            under_cursor: Vec::new(),
            tab_pick: None,
            hover_grip: None,
            vertex: None,
            sketch: None,
            wall_band: Default::default(),
            profile_edit: None,
            mesh_import: None,
            drag: None,
            drag_preview: None,
            wand_preview: None,
            space_held: false,
            window_start: None,
            temp_edit: None,
            temp_anchors: Default::default(),
            anchor_drag: None,
            plan: None,
            view_moved_at: None,
            other_walls_key: None,
            shadow_cache: None,
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
            view_style: Default::default(),
            show_graphics: false,
            presentation: Default::default(),
            sheet_image: None,
            drafting_image: None,
            sheet_sel: None,
            sheet_hover: None,
            sheet_drag: None,
            sheet_new_view: None,
            sheet_new_scale: 50.0,
            table_new_kind: archisim_core::ScheduleKind::Rooms,
            table_new_level: None,
            schedule_level: None,
            show_grid: true,
            thin_lines: false,
            phase_view: Default::default(),
            show_underlay: true,
            other_walls: Vec::new(),
            snap_filter: SnapFilter::default(),
            dark: prefs.dark,
            show_properties: true,
            show_navigator: true,
            thumbs: thumbs::Thumbs::default(),
            symbol_picker: None,
            thumbs3d: Default::default(),
            swatches: Default::default(),
            pin_command_line: false,
            attenuate: true,
            hidden_disciplines: Default::default(),
            locked_disciplines: Default::default(),
            discipline: toolbox::Discipline::default(),
            nav_tab: navigator::NavTab::default(),
            command_search: None,
            side_width: SIDE_WIDTH,
            message_serial: 0,
            message_seen: (0, 0.0),
            message_undo: None,
            said_rev: 0,
            styled: false,
            v3d: View3dState::with_look(
                prefs
                    .textures_3d
                    .as_deref()
                    .and_then(crate::view3d::render::TextureMode::from_key)
                    .unwrap_or_else(view3d_ui::default_textures),
                prefs
                    .quality_3d
                    .unwrap_or_else(crate::view3d::look::default_quality),
            ),
            gpu: None,
            coord: crate::coord::Coord::new(),
            coord_window: coord_ui::CoordWindow::default(),
            team: None,
            team_ui: Default::default(),
            saved_prefs: prefs.clone(),
            prefs,
            applied_zoom: 1.0,
            missions: Default::default(),
            views: Default::default(),
        }
    }

    fn say(&mut self, text: impl Into<String>) {
        self.message_serial += 1;
        // «Deshacer» only for a message that follows a change.
        let rev = self.ed.revision();
        self.message_undo = (rev != self.said_rev).then_some(rev);
        self.said_rev = rev;
        self.message = Some(Message {
            text: text.into(),
            error: false,
        });
    }

    /// The last session ended with an error: says where its report is.
    pub fn previous_crash(&mut self, log: &std::path::Path) {
        self.complain(format!(
            "La última vez ArchiSim se cerró por un error. El informe está en {}: mándaselo a Claude para arreglarlo.",
            log.display()
        ));
    }

    fn complain(&mut self, text: impl Into<String>) {
        self.message_serial += 1;
        self.said_rev = self.ed.revision();
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
        self.rebar_pick = false;
        if tool != Tool::Cad && self.cad.is_some() {
            // Another tool ends the command (keeping what it drew).
            self.end_command(true);
        }
        if tool != self.tool {
            self.wall_tool.finish();
            self.slab_tool.finish();
            self.dim_tool.finish();
            self.beam_tool.finish();
            self.grid_anchor = None;
            self.footing_anchor = None;
            self.mass_tool.finish();
            self.concept.finish();
            self.plot_tool.finish();
            self.pool_tool.finish();
            self.pool_tool.finish();
            self.road_points.clear();
            self.planting_points.clear();
            self.draw_method = Default::default();
            self.spline_points.clear();
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
        // Drawing happens in plan (2D drafting also on sheets), and the
        // tools of the model also in the 3D view (R14).
        let on_sheet = matches!(self.mode, ViewMode::Sheet(_) | ViewMode::Drafting(_));
        let in_3d = self.mode == ViewMode::ThreeD && self.tool_in_3d();
        if tool != Tool::Arrow && !(tool == Tool::Cad && on_sheet) && !in_3d {
            self.show_plan();
        }
    }

    fn replace_editor(&mut self, ed: Editor) {
        self.start.open = false;
        // Another project: the shared one is left (its working copy kept).
        if self.team.is_some() {
            self.team_closing();
            self.team = None;
        }
        self.wall_settings = default_wall_settings(ed.model());
        self.slab_settings = default_slab_settings(ed.model());
        self.door_settings = default_opening_settings(ed.model(), true);
        self.window_settings = default_opening_settings(ed.model(), false);
        (self.column_settings, self.beam_settings) = default_member_settings(ed.model());
        self.roof_settings = roof_ui::default_roof_settings(ed.model());
        self.stair_settings = crate::editor::default_stair_settings(ed.model());
        self.footing_settings = crate::editor::FootingSettings::new(ed.model());
        self.beam_tool.finish();
        self.grid_anchor = None;
        self.footing_anchor = None;
        self.module_tool = Default::default();
        self.module_watch = Default::default();
        self.mass_settings = crate::editor::MassSettings::new(ed.model());
        self.mass_tool.finish();
        self.concept.finish();
        self.plot_tool.finish();
        self.road_points.clear();
        self.road_settings = urban_ui::RoadSettings::new(ed.model());
        self.planting_points.clear();
        self.planting_settings = Default::default();
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
        // Its links and issues, kept beside it.
        for problem in self.coord.project_changed(self.ed.path.as_deref()) {
            self.complain(problem);
        }
        self.coord_window = coord_ui::CoordWindow::default();
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
        // DXF drawings dropped on the window go into the view shown; with
        // the terrain tool (or a terrain selected) a DXF or DWG is a
        // survey for the terrain.
        let survey = self.tool == Tool::Terrain
            || self
                .ed
                .selection
                .iter()
                .any(|e| matches!(e, Element::Terrain(_)));
        // 3D models: the dialog to bring one in.
        let (models, paths): (Vec<PathBuf>, Vec<PathBuf>) = paths
            .into_iter()
            .partition(|p| archisim_mesh::is_model_file(p));
        if let Some(m) = models.first() {
            // Over the start screen: into the project behind it.
            self.start.open = false;
            self.mesh_import = Some(mesh_ui::MeshImport::new(m));
        }
        let (cad, paths): (Vec<PathBuf>, Vec<PathBuf>) = paths.into_iter().partition(|p| {
            p.extension()
                .is_some_and(|e| e.eq_ignore_ascii_case("dxf") || e.eq_ignore_ascii_case("dwg"))
        });
        for d in &cad {
            let dwg = d.extension().is_some_and(|e| e.eq_ignore_ascii_case("dwg"));
            if survey || dwg {
                self.open_survey(d);
            } else {
                self.import_dxf_from(d);
            }
        }
        if paths.is_empty() {
            return;
        }
        let mut openable = paths.iter().filter(|p| crate::open_files::is_openable(p));
        let Some(path) = openable.next().cloned() else {
            self.complain("ArchiSim abre proyectos .archisim y ficheros .ifc (y los .dxf se importan en la vista).");
            return;
        };
        let rest = openable.count();
        if path
            .extension()
            .is_some_and(|e| e.eq_ignore_ascii_case(archisim_team::EXTENSION))
        {
            self.open_team(&path);
            return;
        }
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
                self.note_recent(path);
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
                self.note_recent(path);
                self.say(format!("Abierto {}.", path.display()));
            }
            Err(e) => self.complain(format!("No se puede abrir {}: {e}.", path.display())),
        }
    }

    /// Saves; with `ask` or without a file yet, asks where. Returns whether
    /// the project ended up saved.
    fn save(&mut self, ask: bool) -> bool {
        // In a shared project, the working copy goes to the folder.
        if self.team.is_some() && !ask {
            return self.team_save();
        }
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
                self.note_recent(&p);
                self.say(format!("Guardado en {}.", p.display()));
                if let Err(e) = self.coord.save_beside(&p) {
                    self.complain(format!("No se pudieron guardar los vínculos: {e}."));
                }
                true
            }
            Err(e) => {
                self.complain(format!("No se pudo guardar: {e}."));
                false
            }
        }
    }

    fn export_ifc(&mut self, schema: archisim_ifc::Schema) {
        let (title, name) = match schema {
            archisim_ifc::Schema::Ifc4x3 => ("IFC 4.3", format!("{}.ifc", self.file_name_stem())),
            archisim_ifc::Schema::Ifc4 => ("IFC4", format!("{} (IFC4).ifc", self.file_name_stem())),
        };
        let mut d = rfd::FileDialog::new()
            .set_title(format!("Exportar {title}"))
            .add_filter("IFC", &["ifc"])
            .set_file_name(name);
        if let Some(dir) = self.start_dir() {
            d = d.set_directory(dir);
        }
        if let Some(path) = d.save_file() {
            match self.ed.export_ifc(&path, schema) {
                Ok(()) => self.say(format!("{title} exportado en {}.", path.display())),
                Err(e) => self.complain(format!("No se pudo exportar: {e}.")),
            }
        }
    }

    /// The project as a `.glb`, for D5, Twinmotion or Blender.
    fn export_glb(&mut self) {
        let mut d = rfd::FileDialog::new()
            .set_title("Exportar modelo 3D (glTF)")
            .add_filter("glTF", &["glb"])
            .set_file_name(format!("{}.glb", self.file_name_stem()));
        if let Some(dir) = self.start_dir() {
            d = d.set_directory(dir);
        }
        if let Some(path) = d.save_file() {
            let bytes = archisim_render::glb::project_glb(&self.ed.shown());
            match std::fs::write(&path, &bytes) {
                Ok(()) => self.say(format!(
                    "Modelo 3D exportado ({:.1} MB) en {}.",
                    bytes.len() as f64 / 1e6,
                    path.display()
                )),
                Err(e) => self.complain(format!("No se pudo exportar: {e}.")),
            }
        }
    }

    /// The plan of the current level as drawn now (cut plane, what the view
    /// shows), at a scale.
    fn plan_export_options(&self, scale: f64) -> archisim_plan::PlanOptions {
        archisim_plan::PlanOptions {
            show_reference_lines: self.show_reference_lines,
            scale,
            cut_plane: self.cut_plane,
            style: self.view_style.clone(),
            phase_view: self.phase_view,
            ceiling: self.active_ceiling(),
            ..archisim_plan::PlanOptions::default()
        }
    }

    /// Exports the current level's plan as PDF, DXF or SVG.
    fn export_plan(&mut self, scale: f64) {
        let level = self
            .ed
            .model()
            .level(self.ed.level)
            .map(|l| l.name.clone())
            .unwrap_or_default();
        let mut d = rfd::FileDialog::new()
            .set_title(format!("Exportar planta 1:{scale}"))
            .add_filter("PDF (para imprimir)", &["pdf"])
            .add_filter("DXF (para otros programas de CAD)", &["dxf"])
            .add_filter("SVG", &["svg"])
            .set_file_name(format!(
                "{} - {} 1-{scale}.pdf",
                self.file_name_stem(),
                level
            ));
        if let Some(dir) = self.start_dir() {
            d = d.set_directory(dir);
        }
        if let Some(path) = d.save_file() {
            let options = self.plan_export_options(scale);
            match self.ed.export_plan(&path, &options) {
                Ok(()) => self.say(format!("Planta exportada en {}.", path.display())),
                Err(e) => self.complain(format!("No se pudo exportar: {e}.")),
            }
        }
    }

    /// Exports the section or elevation shown as PDF, DXF or SVG, with the
    /// phases the view shows.
    fn export_section(&mut self, scale: f64) {
        let Some(id) = self.active_section() else {
            return;
        };
        let title = self
            .ed
            .model()
            .section(id)
            .map(|s| s.title())
            .unwrap_or_default();
        let mut d = rfd::FileDialog::new()
            .set_title(format!("Exportar {title} 1:{scale}"))
            .add_filter("PDF (para imprimir)", &["pdf"])
            .add_filter("DXF (para otros programas de CAD)", &["dxf"])
            .add_filter("SVG", &["svg"])
            .set_file_name(format!("{} - {title} 1-{scale}.pdf", self.file_name_stem()));
        if let Some(dir) = self.start_dir() {
            d = d.set_directory(dir);
        }
        if let Some(path) = d.save_file() {
            let options = self.section_export_options(scale);
            match self.ed.export_section(&path, id, &options) {
                Ok(()) => self.say(format!("{title} exportada en {}.", path.display())),
                Err(e) => self.complain(format!("No se pudo exportar: {e}.")),
            }
        }
    }

    pub(super) fn section_export_options(&self, scale: f64) -> archisim_plan::PlanOptions {
        archisim_plan::PlanOptions {
            scale,
            phase_view: self.phase_view,
            style: archisim_core::ViewStyle {
                focus: self.screen_focus(),
                hidden_disciplines: self.hidden_disciplines.clone(),
                ..Default::default()
            },
            ..archisim_plan::PlanOptions::default()
        }
    }

    /// Exports sheets: the one shown (PDF, DXF or SVG) or all (one PDF).
    fn export_sheets(&mut self, all: bool) {
        let model = self.ed.model();
        let ids: Vec<archisim_core::SheetId> = match (all, self.mode) {
            (false, ViewMode::Sheet(s)) => vec![s],
            _ => model.sheets().iter().map(|s| s.id).collect(),
        };
        if ids.is_empty() {
            self.complain("No hay planos: crea uno en el navegador (Planos › Nuevo plano).");
            return;
        }
        let name = match (all, model.sheet(ids[0])) {
            (false, Some(s)) => format!("{} - {} {}.pdf", self.file_name_stem(), s.number, s.name),
            _ => format!("{} - planos.pdf", self.file_name_stem()),
        };
        let mut d = rfd::FileDialog::new()
            .set_title(if all {
                "Exportar todos los planos"
            } else {
                "Exportar plano"
            })
            .add_filter("PDF (para imprimir)", &["pdf"]);
        if !all {
            d = d
                .add_filter("DXF (para otros programas de CAD)", &["dxf"])
                .add_filter("SVG", &["svg"])
                .add_filter("PNG (imagen a 200 ppp)", &["png"]);
        }
        d = d.set_file_name(name);
        if let Some(dir) = self.start_dir() {
            d = d.set_directory(dir);
        }
        if let Some(path) = d.save_file() {
            match self.ed.export_sheets(&path, &ids) {
                Ok(()) => self.say(format!(
                    "{} exportado(s) en {}.",
                    if ids.len() == 1 { "Plano" } else { "Planos" },
                    path.display()
                )),
                Err(e) => self.complain(format!("No se pudo exportar: {e}.")),
            }
        }
    }

    /// Brings a DXF into the view shown (plan or sheet) as 2D drafting.
    fn import_dxf(&mut self) {
        let mut d = rfd::FileDialog::new()
            .set_title("Importar DXF en la vista")
            .add_filter("DXF", &["dxf"]);
        if let Some(dir) = self.start_dir() {
            d = d.set_directory(dir);
        }
        if let Some(path) = d.pick_file() {
            self.import_dxf_from(&path);
        }
    }

    /// Reads a DXF and asks how to bring it in (its unit, its blocks):
    /// see `dxf_import_ui`.
    pub(crate) fn import_dxf_from(&mut self, path: &Path) {
        if self.mode == ViewMode::ThreeD {
            self.show_plan();
        }
        self.open_dxf_import(path);
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
        if self.delete_terrain_points() || self.delete_vertex() {
            return;
        }
        match self.ed.delete_selection() {
            Ok(0) => {}
            Ok(n) => self.say(format!("Borrados {n} elemento(s).")),
            Err(e) => self.complain(format!("No se puede borrar: {e}.")),
        }
    }

    pub(super) fn select_all_pub(&mut self) {
        self.select_all();
    }

    pub(super) fn delete_pub(&mut self) {
        self.delete();
    }

    fn select_all(&mut self) {
        self.ed.selection = crate::pick::elements_on_level(self.ed.model(), self.ed.level)
            .into_iter()
            .collect();
    }

    /// Builds the walls asked for by the wall tool.
    fn build_walls(&mut self, pieces: Vec<Piece>) {
        let mark = self.ed.undo_mark();
        match self.ed.add_walls(&self.wall_settings, &pieces) {
            Ok(ids) if self.room_walls && self.wall_tool.mode == WallMode::Rectangle => {
                // A room (Los Sims'): its floor and its room inside the
                // walls just drawn.
                self.wall_tool.accept(&pieces);
                let n = pieces.len().max(1) as f64;
                let c = pieces
                    .iter()
                    .fold(Point2::new(0.0, 0.0), |a, w| a + w.start)
                    * (1.0 / n);
                let floor = self.ed.add_slab_in_room(
                    self.slab_settings.slab_type,
                    c,
                    archisim_geom::Contour::InteriorFace,
                    self.slab_settings.top_offset,
                );
                let room = self.ed.add_room_in_room(&self.room_settings, c);
                // One step to undo: walls, floor and room.
                self.ed.join_since(mark, "Habitación");
                self.ed.selection = ids.iter().map(|id| Element::Wall(*id)).collect();
                match (floor, room) {
                    (Ok(_), Ok(_)) => {
                        self.say("Habitación hecha: sus 4 muros, su suelo y su local.")
                    }
                    (f, r) => self.complain(format!(
                        "Muros hechos; {}{}",
                        f.err()
                            .map_or(String::new(), |e| format!("sin suelo ({e}) ")),
                        r.err()
                            .map_or(String::new(), |e| format!("sin local ({e})"))
                    )),
                }
            }
            Ok(ids) => {
                self.wall_tool.accept(&pieces);
                // The last walls drawn stay selected, with their temporary
                // dimensions once the chain ends (as Revit does).
                self.ed.selection = ids.iter().map(|id| Element::Wall(*id)).collect();
                let message = match (self.wall_tool.mode, pieces.as_slice()) {
                    (WallMode::Rectangle, [a, b, ..]) => format!(
                        "4 muros creados ({} × {}).",
                        fmt_mm(a.length()),
                        crate::units::fmt_len(b.length())
                    ),
                    (WallMode::Circle, [a, ..]) => format!(
                        "Muro circular de radio {} creado (dos medios círculos).",
                        crate::units::fmt_len(a.arc().map_or(0.0, |c| c.radius))
                    ),
                    (_, [w]) => match w.arc() {
                        Some(arc) => format!(
                            "Muro curvo de {} (radio {}) creado.",
                            crate::units::fmt_len(w.length()),
                            crate::units::fmt_len(arc.radius)
                        ),
                        None => format!("Muro de {} creado.", crate::units::fmt_len(w.length())),
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

    /// Where a piece of furniture made to join others (`joins` in its
    /// recipe: kitchen modules, sofas, shelves…) dragged by its body in
    /// plan goes to touch the one beside it, as when it is placed; `to`
    /// itself for anything else, or a distance typed in.
    pub(super) fn join_snap(&self, d: &DragState, to: Point2) -> Point2 {
        if d.grip != Grip::Body || !d.typed.is_empty() || self.ed.selection.len() != 1 {
            return to;
        }
        let Some(crate::pick::Element::Object(id)) = self.ed.selection.first().copied() else {
            return to;
        };
        let m = self.ed.model();
        let Some(o) = m.object(id) else {
            return to;
        };
        let moved = o.position + (to - d.from);
        match crate::placing::snap_alongside(m, o.level, o.object_type, moved, o.rotation, Some(id))
        {
            Some(q) => d.from + (q - o.position),
            None => to,
        }
    }

    /// Drops the dragged selection at `cursor` (or at the typed distance).
    fn finish_drag(&mut self, cursor: Point2, variant: bool) {
        let Some(d) = self.drag.take() else {
            return;
        };
        self.drag_preview = None;
        let to = self.join_snap(&d, d.target(cursor));
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
        if !pl.more.is_empty() {
            // Spread out: all of them in one step.
            let specs = pl.specs();
            let n = specs.len();
            let mark = self.ed.undo_mark();
            let mut ids = Vec::new();
            for spec in specs {
                match self.ed.add_opening(spec) {
                    Ok(id) => ids.push(id),
                    Err(e) => {
                        self.complain(format!("No se puede colocar: {e}."));
                        break;
                    }
                }
            }
            let what = if door { "puertas" } else { "ventanas" };
            self.ed
                .join_since(mark, &format!("Colocar {n} {what} repartidas"));
            if ids.len() == n {
                self.ed.selection = ids.into_iter().map(Element::Opening).collect();
                self.say(format!("{n} {what} repartidas con machones iguales."));
            }
            return;
        }
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
        missions_ui::clear_marks(&ctx);
        widgets::note_click(&ctx);
        self.ed.select_whole_groups();
        // Sketching, Intro with no command running accepts the sketch
        // (before it can repeat the last command).
        if matches!(self.mode, ViewMode::Plan | ViewMode::Drafting(_)) {
            self.sketch_keys(&ctx);
        }
        // Texts are measured and drawn in the project's lettering.
        archisim_draft::font::set_font(self.ed.model().graphics.font);
        self.handle_close(&ctx);
        // Escape with the right click's menu open only closes it (the
        // selection stays).
        if self.context_menu.is_some()
            && ctx.input_mut(|i| i.consume_key(Modifiers::NONE, Key::Escape))
        {
            self.context_menu = None;
        }
        if self.maquette {
            // Before the shortcuts: W A S D Q E walk here.
            self.maquette_walk(&ctx);
        }
        self.handle_shortcuts(&ctx);
        self.command_search_window(&ctx);
        self.drop_locked_from_selection();
        self.keep_tool_types_valid();
        self.receive_files(&ctx);
        self.coord_poll(&ctx);
        self.team_poll(&ctx);
        self.sync_tabs();
        self.sync_views(&ctx);
        self.update_title(&ctx);
        self.age_message(ctx.input(|i| i.time));
        crate::pick::set_scale(match self.mode {
            ViewMode::Sheet(_) => 1.0,
            ViewMode::Drafting(d) => self.ed.model().draft_view(d).map_or(20.0, |v| v.scale),
            _ => self.plan_scale,
        });
        if self.maquette {
            self.maquette_frame(ui);
        } else {
            self.technical_frame(ui);
        }
        self.context_menu_ui(&ctx);
        self.toast_area(&ctx);
        self.new_level_bubble(&ctx);
        self.after_frame(&ctx);
    }

    /// The window of the technical environment: disciplines, tools and
    /// Properties on the left, the navigator on the right, tabs of views.
    fn technical_frame(&mut self, ui: &mut Ui) {
        let p = theme::Palette::current(ui.ctx());
        // Islands (concepto de interfaz del 24/10/2026): white rounded
        // cards with a soft shadow, a little apart, on warm paper.
        ui.painter()
            .rect_filled(ui.max_rect(), egui::CornerRadius::ZERO, p.ground);
        let gap = 5;
        let island = |m: egui::Margin| {
            egui::Frame::new()
                .fill(p.panel)
                .corner_radius(egui::CornerRadius::same(14))
                .stroke(Stroke::new(1.0, p.line))
                .shadow(egui::epaint::Shadow {
                    offset: [0, 2],
                    blur: 10,
                    spread: 0,
                    color: Color32::from_black_alpha(14),
                })
                .outer_margin(m)
        };
        let side = |l: i8, r: i8| egui::Margin {
            left: l,
            right: r,
            top: gap,
            bottom: gap,
        };
        egui::Panel::top("top-bar")
            .frame(
                island(egui::Margin {
                    left: gap * 2,
                    right: gap * 2,
                    top: gap * 2,
                    bottom: gap,
                })
                .inner_margin(egui::Margin::symmetric(10, 3)),
            )
            .show(ui, |ui| self.top_bar(ui));
        egui::Panel::bottom("bottom-bar")
            .frame(
                island(egui::Margin {
                    left: gap * 2,
                    right: gap * 2,
                    top: gap,
                    bottom: gap * 2,
                })
                .inner_margin(egui::Margin::symmetric(10, 3)),
            )
            .show(ui, |ui| self.bottom_bar(ui));
        // The drawing stays in the middle of the window: the column on the
        // right (navigator) is as wide as the two on the left (disciplines,
        // and tools with properties). Dragging the edge of the tools widens
        // both sides.
        egui::Panel::left("disciplines")
            .resizable(false)
            .exact_size(RAIL_WIDTH + (gap * 3) as f32)
            .frame(island(side(gap * 2, gap)).inner_margin(egui::Margin::symmetric(4, 6)))
            .show(ui, |ui| self.discipline_rail(ui));
        // The tools of the discipline and, below them, Properties, as
        // Revit's palette: choosing a tool and its type is a short move of
        // the mouse.
        let shown = egui::Panel::left("tools")
            .resizable(true)
            .default_size(SIDE_WIDTH - RAIL_WIDTH)
            .min_size(MIN_SIDE_WIDTH - RAIL_WIDTH)
            .max_size(MAX_SIDE_WIDTH - RAIL_WIDTH)
            .frame(island(side(gap, gap)).inner_margin(egui::Margin::symmetric(10, 6)))
            .show(ui, |ui| self.tool_column(ui));
        self.side_width = RAIL_WIDTH + shown.response.rect.width();
        if self.show_navigator {
            let panel = egui::Panel::right("navigator")
                .frame(island(side(gap, gap * 2)).inner_margin(egui::Margin::symmetric(10, 6)));
            // Mirrors the left side.
            panel
                .resizable(false)
                .exact_size(self.side_width)
                .show(ui, |ui| self.navigator(ui));
        }
        let centre = egui::Frame::new()
            .fill(p.panel)
            .corner_radius(egui::CornerRadius::same(14))
            .stroke(Stroke::new(1.0, p.line))
            .outer_margin(side(gap, gap))
            .inner_margin(egui::Margin::same(1));
        egui::CentralPanel::default().frame(centre).show(ui, |ui| {
            ui.spacing_mut().item_spacing.y = 0.0;
            self.tab_strip(ui);
            if self.mode != ViewMode::Schedule {
                egui::Panel::top("context-bar")
                    .resizable(false)
                    .exact_size(context_bar::CONTEXT_BAR_HEIGHT)
                    .frame(
                        egui::Frame::new()
                            .fill(p.panel)
                            .inner_margin(egui::Margin::symmetric(8, 0))
                            .stroke(Stroke::new(1.0, p.line)),
                    )
                    .show(ui, |ui| self.context_bar(ui));
            }
            if !matches!(self.mode, ViewMode::ThreeD | ViewMode::Schedule)
                && self.command_line_shown()
            {
                egui::Panel::bottom("command-line")
                    .resizable(false)
                    .exact_size(COMMAND_LINE_HEIGHT)
                    .frame(
                        egui::Frame::new()
                            .fill(p.panel)
                            .inner_margin(egui::Margin::symmetric(10, 3))
                            .stroke(Stroke::new(1.0, p.line)),
                    )
                    .show(ui, |ui| self.command_line(ui));
            }
            match self.mode {
                ViewMode::Plan => self.plan_view(ui),
                ViewMode::ThreeD => self.view_3d(ui),
                ViewMode::Section => self.section_view(ui),
                ViewMode::Schedule => {
                    if let Some(tabs::TabKind::Schedule(w)) =
                        self.tabs.get(self.active_tab).map(|t| t.kind)
                    {
                        self.schedule_view(ui, w);
                    }
                }
                ViewMode::Sheet(s) => self.sheet_view(ui, s),
                ViewMode::Drafting(d) => self.drafting_view(ui, d),
            }
        });
    }

    /// What every frame does after the window: dialogs and windows.
    fn after_frame(&mut self, ctx: &egui::Context) {
        let ctx = ctx.clone();
        if let Some(text) = ctx.data_mut(|d| d.remove_temp::<String>(Id::unique("length-error"))) {
            self.complain(format!(
                "«{text}» no es una medida válida: escribe un número en {} (Vista → Unidades) o con su unidad (2,7m · 270cm · 2700mm).",
                crate::units::unit().name().to_lowercase()
            ));
        }
        self.dialogs(&ctx);
        self.vg_window(&ctx);
        self.team_windows(&ctx);
        self.text_entry(&ctx);
        if !matches!(self.mode, ViewMode::Sheet(_)) {
            sheet_drop::paint_carried(&ctx, false);
        }
        self.layers_dialog(&ctx);
        self.shortcuts_dialog(&ctx);
        self.wall_profile_dialog(&ctx);
        self.graphics_dialog(&ctx);
        self.presentation_dialog(&ctx);
        self.render_window(&ctx);
        self.price_pick_window(&ctx);
        self.symbol_pick_window(&ctx);
        self.missions_frame(&ctx);
        self.start_screen(&ctx);
        self.sync_prefs(&ctx);
        self.thumbs.end_frame();
    }

    fn handle_close(&mut self, ctx: &egui::Context) {
        // Cmd+Q, «Salir» and closing the session (macOS) are handled like
        // the window's close button.
        let quit = crate::open_files::take_quit();
        if (quit || ctx.input(|i| i.viewport().close_requested())) && !self.allow_close {
            // A shared project keeps its working copy in the folder.
            self.team_closing();
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
        // Only a text field takes the keys: a button keeps the keyboard
        // focus after its click (the Esquina of the bar, say), and Ctrl+Z
        // must still undo then.
        let in_text = typing_in_text(ctx);
        let typing = in_text || self.temp_edit.is_some();
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
        if consume(Modifiers::CTRL, Key::Space) {
            self.open_command_search();
        }
        // Tab over several elements in plan, with the arrow: the next of
        // them (Shift+Tab, the one before); a click takes it.
        let overlapping = !self.maquette
            && self.tool == Tool::Arrow
            && self.hover.is_some()
            && self.under_cursor.len() > 1
            && self.temp_edit.is_none()
            && !in_text;
        if overlapping {
            let back = consume(Modifiers::SHIFT, Key::Tab);
            if back || consume(Modifiers::NONE, Key::Tab) {
                let n = self.under_cursor.len();
                let at = self.cursor.map_or(Point2::new(0.0, 0.0), |c| c.point);
                let i = self.tab_pick.map_or(0, |(_, i)| i % n);
                let i = if back { (i + n - 1) % n } else { (i + 1) % n };
                self.tab_pick = Some((at, i));
                self.hover = self.under_cursor.get(i).copied();
                if let Some(e) = self.hover {
                    let name = context_bar::element_name(&e);
                    self.say(format!(
                        "Tab: {name} ({} de {n}). Clic para elegirlo.",
                        i + 1
                    ));
                }
                ctx.request_repaint();
            }
        }
        // F4: Técnica ↔ Maqueta. It used to be Tab, which Antonio found
        // confusing next to Tab for the elements under the cursor (Revit's
        // Tab): Tab now only does that.
        if !in_text && consume(Modifiers::NONE, Key::F4) {
            let on = !self.maquette;
            self.set_maquette(on);
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
        // Cortar, Copiar and Pegar (also in the right click's menu); egui
        // turns the keys into its own events on most systems.
        let (copy, cut, paste) = ctx.input(|i| {
            let mut v = (false, false, false);
            for e in &i.events {
                match e {
                    egui::Event::Copy => v.0 = true,
                    egui::Event::Cut => v.1 = true,
                    egui::Event::Paste(_) => v.2 = true,
                    _ => {}
                }
            }
            v
        });
        let sheet = !matches!(self.mode, ViewMode::Plan | ViewMode::ThreeD);
        if !sheet && self.tool == Tool::Arrow {
            if (copy || consume(cmd, Key::C)) && !self.ed.selection.is_empty() {
                self.copy_selection(false);
            }
            if (cut || consume(cmd, Key::X)) && !self.ed.selection.is_empty() {
                self.copy_selection(true);
            }
            if (paste || consume(cmd, Key::V)) && self.clipboard.is_some() {
                let at = (self.mode == ViewMode::Plan)
                    .then(|| self.cursor.map(|c| c.point))
                    .flatten();
                self.paste_at(at);
            }
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
        // With a tool in the 3D view, its keys are the plan's (they come
        // with the cursor, in `tools_in_3d`).
        let tool3d = self.mode == ViewMode::ThreeD && self.tool_in_3d();
        if matches!(self.mode, ViewMode::ThreeD | ViewMode::Section) && !tool3d {
            self.keyboard_in_3d(ctx);
        }
    }

    /// Keys while the 3D view or a section is shown: Esc, Supr, and the tool
    /// keys (which draw in the 3D view itself; from a section, in the plan).
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

    /// The Maqueta's view as a big picture (3840 pixels wide), saved as a
    /// PNG: for a render with an AI, D5 or Twinmotion.
    pub(super) fn big_capture(&mut self) {
        let Some((w, h, px)) = self.v3d.capture(3840) else {
            self.complain(
                "No hay imagen 3D que guardar (la vista 3D necesita la tarjeta gráfica).",
            );
            return;
        };
        let mut d = rfd::FileDialog::new()
            .set_title("Guardar captura")
            .add_filter("PNG", &["png"])
            .set_file_name(format!("{} - captura.png", self.file_name_stem()));
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
            Some(Ok(())) => self.say(format!(
                "Captura de {w} × {h} guardada en {}.",
                path.display()
            )),
            _ => self.complain("No se pudo guardar la captura."),
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

    /// Whether the command line is shown: only when pinned (Vista). What
    /// is typed is written beside the cursor and the step of the order
    /// running is in the contextual bar, so the drawing does not jump.
    pub(super) fn command_line_shown(&self) -> bool {
        self.pin_command_line
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
        if let Some(text) = &self.import_report {
            let mut open = true;
            let mut close = false;
            widgets::decision(ctx, "Informe de importación")
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
        self.priorities_dialog(ctx);
        self.curtain_types_dialog(ctx);
        self.vivero_dialog(ctx);
        self.survey_dialog(ctx);
        self.types_dialog(ctx);
        self.library_dialog(ctx);
        self.mesh_import_dialog(ctx);
        self.dxf_import_dialog(ctx);
        self.params_dialog(ctx);
        self.cte_dialog(ctx);
        self.coord_dialog(ctx);
        if self.show_about {
            widgets::dialog("Acerca de ArchiSim")
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
            if let Some(g) = &self.gpu {
                crate::crash::watch_device(&g.device);
            }
        }
        crate::crash::frame_start();
        crate::crash::stage("ventana");
        self.show(ui);
        crate::crash::frame_done();
    }
}

/// The mouse and drawing keys, at the end of the list of shortcuts
/// (`revit_keys::shortcuts_dialog`).
const MOUSE_KEYS: &[(&str, &str)] = &[
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
        "M · F · P · V · C · N (teclas de una letra)",
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
        "Radio; en un rectángulo, ancho x fondo (6x4 en metros)",
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
        place: Default::default(),
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

pub(super) fn text_edit_committed(ui: &mut Ui, id: Id, shown: &str, width: f32) -> Option<String> {
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
mod background;
mod bottom_segments;
mod brush_ui;
mod budget_ui;
mod cad_ui;
mod cards;
mod chrome;
mod collection_ui;
mod concept_ui;
mod context_bar;
mod context_menu;
mod coord_ui;
mod cte_ui;
mod curtain_types_ui;
mod curtain_ui;
mod discipline_ui;
mod draft_props;
mod drafting_view_ui;
mod dxf_import_ui;
mod facade_ui;
mod foundation_ui;
mod ghost3d;
mod glyphs;
mod grips3d;
mod handles;
mod hatch_ui;
mod icons;
mod levels_ui;
mod library_ui;
mod maquette_ui;
mod mass_grips3d;
mod mesh_ui;
mod methods_ui;
mod missions_ui;
mod module_ui;
mod mouse3d;
mod navigator;
mod object_bar;
mod object_gallery;
mod object_ui;
mod paint_ui;
mod params_ui;
mod phases_ui;
mod placing;
mod plan_ui;
mod planting_ui;
mod pool_ui;
mod presentation_ui;
mod priorities_ui;
mod properties;
mod props;
mod railing_ui;
mod rebar_ui;
mod render_ui;
mod revit_keys;
mod roof_ui;
mod room_tool_ui;
mod room_ui;
mod schedule_ui;
mod sculpt_ui;
mod search_ui;
mod section_ui;
mod services_ui;
mod sheet_drop;
mod sheet_ui;
mod sketch_ui;
mod stair_ui;
mod start_ui;
mod structure_ui;
mod survey_ui;
mod symbol_pick;
mod tabs;
mod team_ui;
mod temp_ui;
mod terrain_ui;
pub mod theme;
mod thumbs;
mod thumbs3d;
mod toolbox;
mod tools3d;
mod type_views;
mod types_ui;
mod urban_ui;
mod vertex_ui;
mod view3d_ui;
mod views_ui;
mod vivero_ui;
mod wall_profile_ui;
mod wall_tools_ui;
mod widgets;
pub use tabs::{TabKind, ViewTab};
pub use types_ui::{TypesDialog, TypesTab};
pub use view3d_ui::{View3dState, ViewMode};

#[cfg(test)]
mod tests;

/// Whether a text field has the keyboard (the keys are for it). A button
/// or card just clicked keeps the focus too, but not the keys: the view's
/// keys (Esc, Espacio, Supr…) still work.
pub(crate) fn typing_in_text(ctx: &egui::Context) -> bool {
    let Some(id) = ctx.memory(|m| m.focused()) else {
        return false;
    };
    if egui::TextEdit::load_state(ctx, id).is_some() {
        return true;
    }
    // The button lets go of the keyboard, so Espacio or Intro do not
    // press it again.
    ctx.memory_mut(|m| m.surrender_focus(id));
    false
}
