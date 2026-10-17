//! The ArchiSim window: menus, toolbox, info box, plan view, properties and
//! status bar, laid out like ArchiCAD (tools on the left, settings of the
//! active tool on top) with Revit's properties palette on the right.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use archisim_core::{
    Hand, LevelId, Model, OpeningKind, OpeningTypeId, Point2, ReferenceLine, SlabFunction,
    SlabTypeId, WallFunction, WallSide, WallTypeId,
};
use archisim_geom::{opening_geometry, room_outline, Contour};
use eframe::egui::{
    self, Align2, Color32, ComboBox, CornerRadius, CursorIcon, FontId, Id, Key, KeyboardShortcut,
    Modifiers, PointerButton, Pos2, Rect, Sense, Shape, Stroke, StrokeKind, TextEdit,
    TextureHandle, TextureOptions, Ui, Vec2,
};

use crate::dim_tool::{DimSettings, DimTool};
use crate::drag::{grip_at, hotspots, Drag, Grip};
use crate::editor::{Editor, WallSettings};
use crate::opening_tool::{placement, OpeningSettings, Placement};
use crate::pick::{in_window, outline, pick, Element};
use crate::plan_image::{rasterize, PlanKey};
use crate::slab_tool::{Outcome as SlabOutcome, SlabTool};
use crate::snap::{constrain, snap, snap_except, track, Snap, SnapKind};
use crate::units::{explain_room, fmt_angle, fmt_area, fmt_m, fmt_mm, parse_length};
use crate::view::View;
use crate::wall_tool::{Escape, Outcome, WallTool};

const ACCENT: Color32 = Color32::from_rgb(30, 120, 220);
const PAPER: Color32 = Color32::WHITE;
const GRID_MINOR: Color32 = Color32::from_rgb(236, 238, 242);
const GRID_MAJOR: Color32 = Color32::from_rgb(214, 218, 226);
const REF_LINE: Color32 = Color32::from_rgb(192, 57, 43);
/// Snapping and picking reach, in screen points.
const REACH: f32 = 10.0;
/// How close to a horizontal, vertical or 45° direction the cursor must be
/// for the wall to lock onto it, in screen points.
const TRACK_REACH: f32 = 6.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tool {
    Arrow,
    Wall,
    Slab,
    Door,
    Window,
    Dimension,
}

impl Tool {
    /// Tools that draw by clicking points (snaps and the crosshair apply).
    fn draws_points(self) -> bool {
        matches!(self, Tool::Wall | Tool::Slab | Tool::Dimension)
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
    /// The levels dialog is open.
    pub show_levels: bool,
    /// A level waiting for confirmation before being deleted with its
    /// contents.
    pub level_to_delete: Option<LevelId>,
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
    /// Plan or 3D in the central area.
    pub mode: ViewMode,
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
            show_levels: false,
            level_to_delete: None,
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
            v3d: View3dState::default(),
            gpu: None,
        }
    }

    fn say(&mut self, text: impl Into<String>) {
        self.message = Some(Message {
            text: text.into(),
            error: false,
        });
    }

    fn complain(&mut self, text: impl Into<String>) {
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
            self.placement = None;
            self.window_start = None;
            self.drag = None;
            self.tool = tool;
        }
        // Drawing happens in plan.
        if tool != Tool::Arrow {
            self.mode = ViewMode::Plan;
        }
    }

    fn replace_editor(&mut self, ed: Editor) {
        self.wall_settings = default_wall_settings(ed.model());
        self.slab_settings = default_slab_settings(ed.model());
        self.door_settings = default_opening_settings(ed.model(), true);
        self.window_settings = default_opening_settings(ed.model(), false);
        self.ed = ed;
        self.drag = None;
        self.wall_tool.finish();
        self.slab_tool.finish();
        self.plan = None;
        self.fit_requested = true;
        self.v3d.reset();
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

    fn export_plan(&mut self) {
        let level = self
            .ed
            .model()
            .level(self.ed.level)
            .map(|l| l.name.clone())
            .unwrap_or_default();
        let mut d = rfd::FileDialog::new()
            .set_title("Exportar planta 1:50 (SVG)")
            .add_filter("SVG", &["svg"])
            .set_file_name(format!("{} - {}.svg", self.file_name_stem(), level));
        if let Some(dir) = self.start_dir() {
            d = d.set_directory(dir);
        }
        if let Some(path) = d.save_file() {
            match self.ed.export_plan(&path, self.show_reference_lines) {
                Ok(()) => self.say(format!("Planta exportada en {}.", path.display())),
                Err(e) => self.complain(format!("No se pudo exportar: {e}.")),
            }
        }
    }

    // ---- Commands ------------------------------------------------------------

    fn undo(&mut self) {
        self.drag = None;
        self.wall_tool.finish();
        self.slab_tool.finish();
        match self.ed.undo() {
            Some(l) => self.say(format!("Deshecho: {l}.")),
            None => self.say("No hay nada que deshacer."),
        }
    }

    fn redo(&mut self) {
        self.drag = None;
        self.wall_tool.finish();
        self.slab_tool.finish();
        match self.ed.redo() {
            Some(l) => self.say(format!("Rehecho: {l}.")),
            None => self.say("No hay nada que rehacer."),
        }
    }

    fn delete(&mut self) {
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

    /// Builds a wall asked for by the wall tool.
    fn build_wall(&mut self, start: Point2, end: Point2) {
        match self.ed.add_wall(&self.wall_settings, start, end) {
            Ok(_) => {
                self.wall_tool.accept(end);
                let len = start.distance(end);
                self.say(format!("Muro de {} mm creado.", fmt_mm(len)));
                if !self.wall_tool.is_drawing() {
                    self.say("Contorno cerrado.");
                }
            }
            Err(e) => self.complain(format!("No se puede crear el muro: {e}.")),
        }
    }

    /// Builds the slab closed by the slab tool.
    fn build_slab(&mut self, outline: Vec<Point2>) {
        let s = self.slab_settings;
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
            self.complain("Haz clic sobre un muro para colocar la puerta o ventana.");
            return;
        };
        if let Some(problem) = pl.problem {
            self.complain(format!("No cabe ahí: {problem}."));
            return;
        }
        let door = self.tool == Tool::Door;
        match self.ed.add_opening(pl.spec) {
            Ok(_) => self.say(if door {
                "Puerta colocada."
            } else {
                "Ventana colocada."
            }),
            Err(e) => self.complain(format!("No se puede colocar: {e}.")),
        }
    }

    // ---- Frame -------------------------------------------------------------

    /// Draws the whole window. Called by eframe every frame (and by tests).
    pub fn show(&mut self, ui: &mut Ui) {
        let ctx = ui.ctx().clone();
        self.handle_close(&ctx);
        self.handle_shortcuts(&ctx);
        self.update_title(&ctx);

        egui::Panel::top("menu").show(ui, |ui| self.menu_bar(ui));
        egui::Panel::top("info").show(ui, |ui| self.info_box(ui));
        egui::Panel::top("views").show(ui, |ui| self.view_tabs(ui));
        egui::Panel::bottom("status").show(ui, |ui| self.status_bar(ui));
        egui::Panel::left("tools")
            .resizable(false)
            .exact_size(118.0)
            .show(ui, |ui| self.toolbox(ui));
        egui::Panel::right("properties")
            .resizable(true)
            .default_size(360.0)
            .min_size(300.0)
            .max_size(560.0)
            .show(ui, |ui| {
                egui::ScrollArea::vertical().show(ui, |ui| self.properties(ui))
            });
        egui::CentralPanel::no_frame().show(ui, |ui| match self.mode {
            ViewMode::Plan => self.plan_view(ui),
            ViewMode::ThreeD => self.view_3d(ui),
        });

        if let Some(text) = ctx.data_mut(|d| d.remove_temp::<String>(Id::unique("length-error"))) {
            self.complain(format!(
                "«{text}» no es una medida válida: escribe milímetros (2700) o con unidad (2,7m · 270cm)."
            ));
        }
        self.dialogs(&ctx);
    }

    fn handle_close(&mut self, ctx: &egui::Context) {
        if ctx.input(|i| i.viewport().close_requested()) && !self.allow_close {
            if self.ed.is_modified() {
                ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
                self.pending = Some(Pending::Quit);
            } else {
                self.allow_close = true;
            }
        }
        if self.allow_close && self.pending.is_none() {
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }
    }

    fn handle_shortcuts(&mut self, ctx: &egui::Context) {
        let cmd = Modifiers::COMMAND;
        let cmd_shift = Modifiers::COMMAND | Modifiers::SHIFT;
        let typing = ctx.memory(|m| m.focused().is_some());
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
            match self.mode {
                ViewMode::Plan => self.fit_requested = true,
                ViewMode::ThreeD => self.v3d.fit_requested = true,
            }
        }
        if consume(Modifiers::NONE, Key::F2) {
            self.set_mode(ViewMode::Plan);
        }
        if consume(Modifiers::NONE, Key::F3) {
            self.set_mode(ViewMode::ThreeD);
        }
        if self.mode == ViewMode::ThreeD {
            self.keyboard_in_3d(ctx);
        }
    }

    /// Keys while the 3D view is shown: Esc, Supr, and the tool keys,
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
                let tool = Tool::from_key(t);
                if let Some(tool) = tool {
                    self.set_tool(tool);
                }
            }
        }
    }

    /// Tabs to switch between the plan and the 3D view, with the 3D view's
    /// own settings.
    fn view_tabs(&mut self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            let level = self
                .ed
                .model()
                .level(self.ed.level)
                .map(|l| l.name.clone())
                .unwrap_or_default();
            if ui
                .selectable_label(self.mode == ViewMode::Plan, format!("Planta · {level}"))
                .on_hover_text("F2")
                .clicked()
            {
                self.set_mode(ViewMode::Plan);
            }
            if ui
                .selectable_label(self.mode == ViewMode::ThreeD, "3D")
                .on_hover_text("F3")
                .clicked()
            {
                self.set_mode(ViewMode::ThreeD);
            }
            if self.mode == ViewMode::ThreeD {
                ui.separator();
                ui.radio_value(&mut self.v3d.camera.perspective, true, "Perspectiva");
                ui.radio_value(&mut self.v3d.camera.perspective, false, "Axonometría");
                ui.separator();
                if ui.button("Ajustar").on_hover_text("Inicio").clicked() {
                    self.v3d.fit_requested = true;
                }
                if ui.button("Guardar imagen…").clicked() {
                    self.export_3d_picture();
                }
            }
        });
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

    fn menu_bar(&mut self, ui: &mut Ui) {
        egui::MenuBar::new().ui(ui, |ui| {
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
                if ui.button("Exportar IFC4…").clicked() {
                    self.export_ifc();
                }
                if ui.button("Exportar planta 1:50 (SVG)…").clicked() {
                    self.export_plan();
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
                ui.checkbox(&mut self.v3d.camera.perspective, "Perspectiva en 3D");
                ui.separator();
                if ui
                    .add(shortcut_button("Ajustar a la ventana", "Inicio"))
                    .clicked()
                {
                    match self.mode {
                        ViewMode::Plan => self.fit_requested = true,
                        ViewMode::ThreeD => self.v3d.fit_requested = true,
                    }
                }
                ui.checkbox(&mut self.show_reference_lines, "Líneas de referencia");
                ui.separator();
                if ui.button("Plantas…").clicked() {
                    self.show_levels = true;
                }
            });
            ui.menu_button("Ayuda", |ui| {
                if ui.button("Ratón y teclado").clicked() {
                    self.show_shortcuts = true;
                }
                if ui.button("Acerca de ArchiSim").clicked() {
                    self.show_about = true;
                }
            });
        });
    }

    /// ArchiCAD's info box: the settings of the active tool, and the level.
    fn info_box(&mut self, ui: &mut Ui) {
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing.x = 10.0;
            match self.tool {
                Tool::Arrow => {
                    ui.strong("Flecha");
                    ui.label("Clic: seleccionar · Mayús+clic: añadir o quitar · arrastrar lo seleccionado: mover (Ctrl: copiar) · arrastrar en vacío: ventana de selección");
                }
                Tool::Wall => {
                    ui.strong("Muro");
                    self.wall_settings_ui(ui);
                }
                Tool::Slab => {
                    ui.strong("Forjado");
                    self.slab_settings_ui(ui);
                }
                Tool::Door => {
                    ui.strong("Puerta");
                    self.opening_settings_ui(ui, true);
                }
                Tool::Window => {
                    ui.strong("Ventana");
                    self.opening_settings_ui(ui, false);
                }
                Tool::Dimension => {
                    ui.strong("Cota");
                    self.dim_settings_ui(ui);
                }
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui
                    .small_button("Plantas…")
                    .on_hover_text("Añadir, renombrar, cambiar la cota o borrar plantas")
                    .clicked()
                {
                    self.show_levels = true;
                }
                self.level_picker(ui);
                ui.label("Planta:");
            });
        });
    }

    fn wall_settings_ui(&mut self, ui: &mut Ui) {
        let model = self.ed.model();
        let types: Vec<(WallTypeId, String, f64, WallFunction)> = model
            .wall_types()
            .map(|t| (t.id, t.name.clone(), t.thickness(), t.function))
            .collect();
        let current = types.iter().find(|t| t.0 == self.wall_settings.wall_type);
        ui.label("Tipo:");
        let mut chosen = self.wall_settings.wall_type;
        ComboBox::from_id_salt("tool-wall-type")
            .width(240.0)
            .selected_text(current.map_or("—".into(), |t| t.1.clone()))
            .show_ui(ui, |ui| {
                for (id, name, thickness, _) in &types {
                    ui.selectable_value(
                        &mut chosen,
                        *id,
                        format!("{name} · {} mm", fmt_mm(*thickness)),
                    );
                }
            });
        if chosen != self.wall_settings.wall_type {
            self.wall_settings.wall_type = chosen;
            // Façades are usually drawn by their outside face, partitions
            // by their axis.
            if let Some(t) = types.iter().find(|t| t.0 == chosen) {
                self.wall_settings.reference = default_reference(t.3);
            }
        }
        if let Some(t) = current {
            ui.label(format!("Espesor {} mm", fmt_mm(t.2)));
        }
        ui.label("Línea de referencia:");
        reference_combo(ui, "tool-wall-ref", &mut self.wall_settings.reference);
        ui.label("Altura:");
        if let Some(v) = length_field(
            ui,
            Id::unique("tool-wall-height"),
            self.wall_settings.height,
            70.0,
        ) {
            if v > 0.0 {
                self.wall_settings.height = v;
            } else {
                self.complain("La altura debe ser mayor que cero.");
            }
        }
        ui.label("Desfase de base:");
        if let Some(v) = length_field(
            ui,
            Id::unique("tool-wall-base"),
            self.wall_settings.base_offset,
            60.0,
        ) {
            self.wall_settings.base_offset = v;
        }
    }

    fn slab_settings_ui(&mut self, ui: &mut Ui) {
        ui.label("Método:").on_hover_text(
            "Polígono: esquina a esquina. Varita mágica: clic dentro de un recinto de muros (también con Espacio + clic)",
        );
        ComboBox::from_id_salt("tool-slab-method")
            .selected_text(if self.slab_settings.wand {
                "Varita mágica"
            } else {
                "Polígono"
            })
            .show_ui(ui, |ui| {
                ui.selectable_value(&mut self.slab_settings.wand, false, "Polígono");
                ui.selectable_value(&mut self.slab_settings.wand, true, "Varita mágica");
            });
        ui.label("Contorno:").on_hover_text(
            "Hasta dónde llega el forjado de la varita mágica: la cara de los muros del recinto, su eje, o la cara exterior de todo el edificio",
        );
        ComboBox::from_id_salt("tool-slab-contour")
            .selected_text(contour_name(self.slab_settings.contour))
            .show_ui(ui, |ui| {
                for (c, hint) in [
                    (
                        Contour::InteriorFace,
                        "El recinto hasta la cara de sus muros",
                    ),
                    (Contour::WallAxis, "El recinto hasta el eje de sus muros"),
                    (
                        Contour::ExteriorFace,
                        "Todo el edificio hasta la cara exterior de sus muros",
                    ),
                ] {
                    ui.selectable_value(&mut self.slab_settings.contour, c, contour_name(c))
                        .on_hover_text(hint);
                }
            });
        ui.separator();
        let types: Vec<(SlabTypeId, String, f64)> = self
            .ed
            .model()
            .slab_types()
            .map(|t| (t.id, t.name.clone(), t.thickness()))
            .collect();
        let current = types.iter().find(|t| t.0 == self.slab_settings.slab_type);
        ui.label("Tipo:");
        let mut chosen = self.slab_settings.slab_type;
        ComboBox::from_id_salt("tool-slab-type")
            .width(220.0)
            .selected_text(current.map_or("—".into(), |t| t.1.clone()))
            .show_ui(ui, |ui| {
                for (id, name, thickness) in &types {
                    ui.selectable_value(
                        &mut chosen,
                        *id,
                        format!("{name} · {} mm", fmt_mm(*thickness)),
                    );
                }
            });
        self.slab_settings.slab_type = chosen;
        if let Some(t) = current {
            ui.label(format!("Canto {} mm", fmt_mm(t.2)));
        }
        ui.label("Cara superior:").on_hover_text(
            "Altura de la cara superior sobre la planta. Referencia en la cara superior, como en ArchiCAD y Revit; las capas cuelgan hacia abajo",
        );
        if let Some(v) = length_field(
            ui,
            Id::unique("tool-slab-top"),
            self.slab_settings.top_offset,
            60.0,
        ) {
            self.slab_settings.top_offset = v;
        }
    }

    fn opening_settings_ui(&mut self, ui: &mut Ui, door: bool) {
        let types: Vec<(OpeningTypeId, String)> = self
            .ed
            .model()
            .opening_types()
            .filter(|t| t.is_door() == door)
            .map(|t| (t.id, t.name.clone()))
            .collect();
        let settings = if door {
            &mut self.door_settings
        } else {
            &mut self.window_settings
        };
        let current = types
            .iter()
            .find(|t| t.0 == settings.opening_type)
            .map_or("—".to_string(), |t| t.1.clone());
        ui.label("Tipo:");
        ComboBox::from_id_salt(if door {
            "tool-door-type"
        } else {
            "tool-window-type"
        })
        .width(250.0)
        .selected_text(current)
        .show_ui(ui, |ui| {
            for (id, name) in &types {
                ui.selectable_value(&mut settings.opening_type, *id, name);
            }
        });
        ui.label("Mano:")
            .on_hover_text("UNE-EN 12519: mirando desde el lado hacia el que abre, bisagras a la izquierda o a la derecha");
        ComboBox::from_id_salt(if door {
            "tool-door-hand"
        } else {
            "tool-window-hand"
        })
        .selected_text(hand_name(settings.hand))
        .show_ui(ui, |ui| {
            ui.selectable_value(&mut settings.hand, Hand::Left, hand_name(Hand::Left));
            ui.selectable_value(&mut settings.hand, Hand::Right, hand_name(Hand::Right));
        });
        ui.label("Antepecho:");
        let sill_id = Id::unique(if door {
            "tool-door-sill"
        } else {
            "tool-window-sill"
        });
        if let Some(v) = length_field(ui, sill_id, settings.sill, 60.0) {
            settings.sill = v;
        }
        ui.label("Retranqueo:").on_hover_text(
            "Desde la cara exterior del muro hasta la cara exterior del marco. Vacío: centrado en el muro",
        );
        let setback_id = Id::unique(if door {
            "tool-door-setback"
        } else {
            "tool-window-setback"
        });
        let shown = settings.setback.map(fmt_mm).unwrap_or_default();
        if let Some(text) = text_edit_committed(ui, setback_id, &shown, 60.0) {
            if text.trim().is_empty() {
                settings.setback = None;
            } else if let Some(v) = parse_length(&text) {
                settings.setback = Some(v);
            } else {
                ui.ctx()
                    .data_mut(|d| d.insert_temp(Id::unique("length-error"), text));
            }
        }
        if settings.setback.is_none() {
            ui.label(egui::RichText::new("(centrado)").weak());
        }
    }

    fn level_picker(&mut self, ui: &mut Ui) {
        let levels: Vec<(LevelId, String)> = self
            .ed
            .model()
            .levels()
            .iter()
            .rev()
            .map(|l| (l.id, format!("{} ({})", l.name, fmt_level(l.elevation))))
            .collect();
        let current = levels
            .iter()
            .find(|l| l.0 == self.ed.level)
            .map_or("—".to_string(), |l| l.1.clone());
        let mut chosen = self.ed.level;
        ComboBox::from_id_salt("level")
            .selected_text(current)
            .show_ui(ui, |ui| {
                for (id, name) in &levels {
                    ui.selectable_value(&mut chosen, *id, name);
                }
            });
        if chosen != self.ed.level {
            self.ed.level = chosen;
            self.ed.selection.clear();
            self.wall_tool.finish();
            self.slab_tool.finish();
            self.dim_tool.finish();
        }
    }

    fn toolbox(&mut self, ui: &mut Ui) {
        ui.add_space(6.0);
        ui.label(egui::RichText::new("HERRAMIENTAS").small().weak());
        ui.add_space(2.0);
        let size = Vec2::new(ui.available_width(), 30.0);
        for (tool, label, key) in [
            (Tool::Arrow, "↖  Flecha", "Esc"),
            (Tool::Wall, "▭  Muro", "M"),
            (Tool::Slab, "▱  Forjado", "F"),
            (Tool::Door, "◜  Puerta", "P"),
            (Tool::Window, "⊟  Ventana", "V"),
            (Tool::Dimension, "↔  Cota", "C"),
        ] {
            let r = ui
                .add_sized(size, egui::Button::selectable(self.tool == tool, label))
                .on_hover_text(format!("Atajo: {key}"));
            if r.clicked() {
                self.set_tool(tool);
            }
        }
    }

    fn status_bar(&mut self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            let hint = if self.mode == ViewMode::ThreeD {
                "Arrastrar: girar · rueda: zoom · arrastrar con la rueda o el botón derecho: encuadre · clic: seleccionar · Inicio: ajustar · F2: planta"
            } else {
                match self.tool {
                Tool::Arrow if self.drag.is_some() => "Suelta para terminar · Ctrl: copiar (o, en un extremo de muro, soltarlo de la esquina; en una arista, nueva esquina) · Mayús: ángulos de 45° · distancia en mm + Intro · Esc: cancelar",
                Tool::Arrow if !self.ed.selection.is_empty() => "Arrastrar lo seleccionado: mover (Ctrl: copiar) · arrastrar un extremo o esquina: estirar · un rombo: mover la arista · Rueda: zoom · Inicio: ajustar",
                Tool::Arrow => "Clic: seleccionar · arrastrar: ventana de selección · Rueda: zoom · arrastrar con la rueda o el botón derecho: encuadre · Inicio: ajustar",
                Tool::Wall if !self.wall_tool.is_drawing() => "Clic: inicio del muro",
                Tool::Wall => "Clic: siguiente punto · escribe la longitud en mm + Intro · Mayús: ángulos de 45° · Intro/Esc/botón derecho: terminar",
                Tool::Slab if !self.slab_tool.is_drawing() && self.slab_settings.wand => "Varita mágica: clic dentro de un recinto de muros",
                Tool::Slab if !self.slab_tool.is_drawing() => "Clic: primera esquina del forjado · Espacio + clic dentro de un recinto: varita mágica",
                Tool::Slab => "Clic: siguiente esquina · clic en la primera o Intro: cerrar · longitud en mm + Intro · Esc: quitar la última esquina",
                Tool::Door | Tool::Window => "Clic sobre un muro: se coloca centrada en el cursor y abre hacia el lado del muro donde está el cursor · Esc: flecha",
                Tool::Dimension => self.dim_hint(),
                }
            };
            // Position and message on the right; the hint takes what is
            // left and is cut short rather than overlapping them.
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if let (Some(c), ViewMode::Plan) = (self.cursor, self.mode) {
                    ui.monospace(format!(
                        "X {}  Y {} m  ·  {}",
                        fmt_m(c.point.x),
                        fmt_m(c.point.y),
                        c.kind.label()
                    ));
                }
                if let Some(m) = &self.message {
                    let color = if m.error {
                        Color32::from_rgb(200, 40, 40)
                    } else {
                        Color32::from_rgb(30, 130, 70)
                    };
                    ui.separator();
                    ui.colored_label(color, &m.text);
                }
                ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                    ui.add(egui::Label::new(egui::RichText::new(hint).weak()).truncate());
                });
            });
        });
    }

    // ---- Plan view -----------------------------------------------------------

    fn plan_view(&mut self, ui: &mut Ui) {
        let (rect, response) = ui.allocate_exact_size(ui.available_size(), Sense::click_and_drag());
        let ctx = ui.ctx().clone();
        let painter = ui.painter_at(rect);
        self.canvas = rect;

        if self.fit_requested && rect.width() > 50.0 {
            self.fit_requested = false;
            match self.ed.level_bounds() {
                Some((lo, hi)) => self.view.fit(rect, lo, hi, 40.0),
                None => self
                    .view
                    .fit(rect, Point2::new(-2.0, -2.0), Point2::new(18.0, 12.0), 20.0),
            }
        }

        self.navigate(&ctx, &response, rect);
        self.track_cursor(&ctx, &response, rect);
        if ctx.memory(|m| m.focused().is_none()) {
            self.keyboard_in_view(&ctx);
        }
        self.clicks(&ctx, &response, rect);

        // Paper, grid, the plan itself, then what is being done on top.
        painter.rect_filled(rect, CornerRadius::ZERO, PAPER);
        self.draw_grid(&painter, rect);
        self.draw_plan(&ctx, &painter, rect);
        self.draw_origin(&painter, rect);
        self.draw_selection(&painter, rect);
        self.draw_hotspots(&painter, rect);
        self.draw_wall_preview(&painter, rect);
        self.draw_slab_preview(&painter, rect);
        self.draw_opening_preview(&painter, rect);
        self.draw_wand_preview(&painter, rect);
        self.draw_dim_preview(&painter, rect);
        self.draw_drag_preview(&painter, rect);
        self.draw_cursor(&painter, rect);
        if let (Some(a), Some(c)) = (self.window_start, self.cursor) {
            let r = Rect::from_two_pos(
                self.view.to_screen(rect, a),
                self.view.to_screen(rect, c.point),
            );
            painter.rect_filled(r, CornerRadius::ZERO, ACCENT.gamma_multiply(0.08));
            painter.rect_stroke(
                r,
                CornerRadius::ZERO,
                Stroke::new(1.0, ACCENT),
                StrokeKind::Inside,
            );
        }

        let icon = match self.tool {
            Tool::Wall | Tool::Slab | Tool::Door | Tool::Window | Tool::Dimension => {
                CursorIcon::Crosshair
            }
            Tool::Arrow if self.drag.is_some() => CursorIcon::Grabbing,
            Tool::Arrow if matches!(self.hover_grip, Some(Grip::Body)) => CursorIcon::Move,
            Tool::Arrow if self.hover_grip.is_some() => CursorIcon::Grab,
            Tool::Arrow if self.hover.is_some() => CursorIcon::PointingHand,
            Tool::Arrow => CursorIcon::Default,
        };
        if response.hovered() {
            ctx.set_cursor_icon(icon);
        }
    }

    fn navigate(&mut self, ctx: &egui::Context, response: &egui::Response, rect: Rect) {
        if response.dragged_by(PointerButton::Middle)
            || response.dragged_by(PointerButton::Secondary)
        {
            self.view.pan(response.drag_delta());
        }
        if response.double_clicked_by(PointerButton::Middle) {
            self.fit_requested = true;
        }
        if let Some(pos) = response.hover_pos() {
            let (scroll, zoom) = ctx.input(|i| (i.smooth_scroll_delta, i.zoom_delta()));
            let factor = zoom as f64 * (scroll.y as f64 / 300.0).exp();
            if (factor - 1.0).abs() > 1e-6 {
                self.view.zoom_at(rect, pos, factor);
            }
        }
    }

    fn track_cursor(&mut self, ctx: &egui::Context, response: &egui::Response, rect: Rect) {
        let Some(pos) = response.hover_pos() else {
            self.cursor = None;
            self.hover = None;
            return;
        };
        self.space_held = ctx.input(|i| i.key_down(Key::Space));
        let raw = self.view.to_model(rect, pos);
        let reach = self.view.to_metres(REACH);
        let model = self.ed.model();
        match self.tool {
            Tool::Wall => {
                let mut s = snap(model, self.ed.level, raw, reach, self.wall_tool.chain());
                if let Some(a) = self.wall_tool.anchor() {
                    if ctx.input(|i| i.modifiers.shift) {
                        s = constrain(a, s);
                    } else {
                        s = track(a, s, self.view.to_metres(TRACK_REACH));
                    }
                }
                self.cursor = Some(s);
                self.hover = None;
            }
            Tool::Slab if self.wand_active(ctx) => {
                let p = crate::snap::round_point(raw);
                self.cursor = Some(Snap {
                    point: p,
                    kind: SnapKind::Free,
                });
                self.hover = None;
                let contour = self.slab_settings.contour;
                let key = (self.ed.revision(), p, contour);
                let fresh = self
                    .wand_preview
                    .as_ref()
                    .is_some_and(|w| (w.0, w.1, w.2) == key);
                if !fresh {
                    let found = room_outline(model, self.ed.level, p, contour)
                        .map_err(|e| explain_room(&e));
                    self.wand_preview = Some((key.0, key.1, key.2, found));
                }
            }
            Tool::Slab => {
                let mut s = snap(model, self.ed.level, raw, reach, self.slab_tool.corners());
                if let Some(a) = self.slab_tool.anchor() {
                    if ctx.input(|i| i.modifiers.shift) {
                        s = constrain(a, s);
                    } else {
                        s = track(a, s, self.view.to_metres(TRACK_REACH));
                    }
                }
                self.cursor = Some(s);
                self.hover = None;
            }
            Tool::Dimension => {
                let mut s = snap(model, self.ed.level, raw, reach, self.dim_tool.points());
                if let (false, Some(a)) =
                    (self.dim_tool.is_placing(), self.dim_tool.points().last())
                {
                    if ctx.input(|i| i.modifiers.shift) {
                        s = constrain(*a, s);
                    } else {
                        s = track(*a, s, self.view.to_metres(TRACK_REACH));
                    }
                }
                self.cursor = Some(s);
                self.hover = None;
            }
            Tool::Door | Tool::Window => {
                let settings = if self.tool == Tool::Door {
                    self.door_settings
                } else {
                    self.window_settings
                };
                self.placement = placement(model, self.ed.level, raw, reach, &settings);
                self.cursor = Some(Snap {
                    point: crate::snap::round_point(raw),
                    kind: SnapKind::Free,
                });
                self.hover = None;
            }
            Tool::Arrow if self.drag.is_some() => {
                let Some(d) = &self.drag else {
                    return;
                };
                let mut s = snap_except(model, self.ed.level, raw, reach, &[], &d.skip);
                if ctx.input(|i| i.modifiers.shift) {
                    s = constrain(d.from, s);
                } else {
                    s = track(d.from, s, self.view.to_metres(TRACK_REACH));
                }
                self.cursor = Some(s);
                self.hover = None;
                self.hover_grip = None;
                let variant = ctx.input(|i| i.modifiers.command);
                let to = d.target(s.point);
                let fresh = self
                    .drag_preview
                    .as_ref()
                    .is_some_and(|p| p.0 == to && p.1 == variant);
                if !fresh {
                    let drag = Drag {
                        grip: d.grip,
                        from: d.from,
                        to,
                        variant,
                    };
                    let ghost = crate::drag::preview(model, &self.ed.selection, &drag)
                        .map_err(|e| crate::units::explain(&e));
                    self.drag_preview = Some((to, variant, ghost));
                }
            }
            Tool::Arrow => {
                self.cursor = Some(Snap {
                    point: crate::snap::round_point(raw),
                    kind: SnapKind::Free,
                });
                self.hover_grip = if self.window_start.is_some() {
                    None
                } else {
                    grip_at(model, &self.ed.selection, raw, reach).map(|g| g.0)
                };
                self.hover = if self.window_start.is_some() {
                    None
                } else {
                    pick(model, self.ed.level, raw, reach)
                };
            }
        }
    }

    fn keyboard_in_view(&mut self, ctx: &egui::Context) {
        let (events, enter, escape, backspace, delete) = ctx.input(|i| {
            (
                i.events.clone(),
                i.key_pressed(Key::Enter),
                i.key_pressed(Key::Escape),
                i.key_pressed(Key::Backspace),
                i.key_pressed(Key::Delete),
            )
        });
        if self.drag.is_some() {
            self.keyboard_while_dragging(ctx, &events, enter, escape, backspace);
            return;
        }
        let drawing =
            self.wall_tool.is_drawing() || self.slab_tool.is_drawing() || self.dim_tool.is_active();
        let mut used_text = false;
        for e in &events {
            if let egui::Event::Text(t) = e {
                for c in t.chars() {
                    used_text |= match self.tool {
                        Tool::Wall => self.wall_tool.type_char(c),
                        Tool::Slab => self.slab_tool.type_char(c),
                        _ => false,
                    };
                }
            }
        }
        if !used_text && !drawing {
            // Tool keys: M muro, F forjado, P puerta, V ventana.
            for e in &events {
                if let egui::Event::Text(t) = e {
                    let tool = Tool::from_key(t);
                    if let Some(tool) = tool {
                        self.set_tool(tool);
                    }
                }
            }
        }
        let edited_length = backspace
            && match self.tool {
                Tool::Wall => self.wall_tool.backspace(),
                Tool::Slab => self.slab_tool.backspace(),
                _ => false,
            };
        if !edited_length && (delete || backspace) && !drawing {
            self.delete();
        }
        if enter {
            let towards = self.cursor.map(|c| c.point).unwrap_or_default();
            match self.tool {
                Tool::Wall => {
                    if let Outcome::Build(a, b) = self.wall_tool.enter(towards) {
                        self.build_wall(a, b);
                    }
                }
                Tool::Slab => match self.slab_tool.enter(towards) {
                    SlabOutcome::Build(outline) => self.build_slab(outline),
                    SlabOutcome::Invalid => self.complain(
                        "El contorno necesita al menos tres esquinas y no puede cruzarse.",
                    ),
                    SlabOutcome::Nothing => {}
                },
                Tool::Dimension if !self.dim_tool.enough() => {
                    self.complain("Marca al menos dos puntos para acotar.");
                }
                _ => {}
            }
        }
        if escape {
            match self.tool {
                Tool::Wall => {
                    if self.wall_tool.escape() == Escape::Idle {
                        self.set_tool(Tool::Arrow);
                    }
                }
                Tool::Slab => {
                    if !self.slab_tool.escape() {
                        self.set_tool(Tool::Arrow);
                    }
                }
                Tool::Door | Tool::Window => self.set_tool(Tool::Arrow),
                Tool::Dimension => {
                    if !self.dim_tool.escape() {
                        self.set_tool(Tool::Arrow);
                    }
                }
                Tool::Arrow => {
                    self.window_start = None;
                    self.ed.selection.clear();
                }
            }
        }
    }

    /// While dragging: a typed distance and Enter drop the selection that
    /// far towards the cursor; Esc cancels.
    fn keyboard_while_dragging(
        &mut self,
        ctx: &egui::Context,
        events: &[egui::Event],
        enter: bool,
        escape: bool,
        backspace: bool,
    ) {
        if escape {
            self.drag = None;
            self.drag_preview = None;
            self.say("Cancelado.");
            return;
        }
        if let Some(d) = &mut self.drag {
            for e in events {
                if let egui::Event::Text(t) = e {
                    for c in t.chars() {
                        if c.is_ascii_digit() || matches!(c, ',' | '.' | 'm' | 'c' | 'M' | 'C') {
                            d.typed.push(c);
                        }
                    }
                }
            }
            if backspace {
                d.typed.pop();
            }
        }
        if enter {
            if let Some(c) = self.cursor {
                let variant = ctx.input(|i| i.modifiers.command);
                self.finish_drag(c.point, variant);
            }
        }
    }

    /// Whether a click with the slab tool uses the magic wand: the wand is
    /// chosen, or Space is held, and no outline is being drawn.
    fn wand_active(&self, ctx: &egui::Context) -> bool {
        !self.slab_tool.is_drawing()
            && (self.slab_settings.wand || ctx.input(|i| i.key_down(Key::Space)))
    }

    fn clicks(&mut self, ctx: &egui::Context, response: &egui::Response, rect: Rect) {
        let shift = ctx.input(|i| i.modifiers.shift || i.modifiers.command);
        match self.tool {
            Tool::Wall => {
                if response.clicked_by(PointerButton::Primary) {
                    if let Some(c) = self.cursor {
                        if let Outcome::Build(a, b) = self.wall_tool.click(c.point) {
                            self.build_wall(a, b);
                        }
                    }
                }
                if response.double_clicked_by(PointerButton::Primary)
                    || response.clicked_by(PointerButton::Secondary)
                {
                    self.wall_tool.finish();
                }
            }
            Tool::Slab if self.wand_active(ctx) => {
                if response.clicked_by(PointerButton::Primary) {
                    if let Some(c) = self.cursor {
                        self.build_slab_in_room(c.point);
                    }
                }
            }
            Tool::Slab => {
                if response.clicked_by(PointerButton::Primary) {
                    if let Some(c) = self.cursor {
                        match self.slab_tool.click(c.point) {
                            SlabOutcome::Build(outline) => self.build_slab(outline),
                            SlabOutcome::Invalid => self.complain(
                                "El contorno no puede cruzarse: corrige la última esquina.",
                            ),
                            SlabOutcome::Nothing => {}
                        }
                    }
                }
                if response.clicked_by(PointerButton::Secondary) {
                    self.slab_tool.finish();
                }
            }
            Tool::Door | Tool::Window => {
                if response.clicked_by(PointerButton::Primary) {
                    self.place_opening();
                }
            }
            Tool::Dimension => {
                if response.double_clicked_by(PointerButton::Primary)
                    || response.clicked_by(PointerButton::Secondary)
                {
                    self.dim_tool.enough();
                } else if response.clicked_by(PointerButton::Primary) {
                    if let Some(c) = self.cursor {
                        let reach = self.view.to_metres(REACH);
                        let out = self.dim_tool.click(
                            self.ed.model(),
                            self.ed.level,
                            self.dim_settings,
                            c.point,
                            reach,
                        );
                        self.dim_outcome(out);
                    }
                }
            }
            Tool::Arrow => {
                if response.clicked_by(PointerButton::Primary) {
                    let hit = self.hover;
                    match (hit, shift) {
                        (Some(e), true) => {
                            if !self.ed.selection.remove(&e) {
                                self.ed.selection.insert(e);
                            }
                        }
                        (Some(e), false) => {
                            self.ed.selection.clear();
                            self.ed.selection.insert(e);
                        }
                        (None, false) => self.ed.selection.clear(),
                        (None, true) => {}
                    }
                }
                if response.drag_started_by(PointerButton::Primary) {
                    let origin = ctx
                        .input(|i| i.pointer.press_origin())
                        .map(|p| self.view.to_model(rect, p));
                    let reach = self.view.to_metres(REACH);
                    let grip =
                        origin.and_then(|o| grip_at(self.ed.model(), &self.ed.selection, o, reach));
                    match grip {
                        Some((grip, from)) => {
                            // The selection is grabbed by the nearest
                            // special point, so it can be dropped exactly on
                            // another one.
                            let from = if grip == Grip::Body {
                                snap(self.ed.model(), self.ed.level, from, reach, &[]).point
                            } else {
                                from
                            };
                            let skip =
                                crate::drag::moving(self.ed.model(), &self.ed.selection, grip);
                            self.drag = Some(DragState {
                                grip,
                                from,
                                typed: String::new(),
                                skip,
                            });
                            self.drag_preview = None;
                        }
                        None => self.window_start = origin,
                    }
                }
                if response.drag_stopped() && self.drag.is_some() {
                    match self.cursor {
                        Some(c) => {
                            let variant = ctx.input(|i| i.modifiers.command);
                            self.finish_drag(c.point, variant);
                        }
                        None => self.say("Cancelado: se ha soltado fuera de la planta."),
                    }
                    self.drag = None;
                }
                if response.drag_stopped() {
                    if let (Some(a), Some(c)) = (self.window_start.take(), self.cursor) {
                        let found = in_window(self.ed.model(), self.ed.level, a, c.point);
                        if !shift {
                            self.ed.selection.clear();
                        }
                        self.ed.selection.extend(found);
                    }
                }
            }
        }
    }

    fn draw_grid(&self, painter: &egui::Painter, rect: Rect) {
        let (lo, hi) = self.view.visible(rect);
        for (step, color) in [(1.0, GRID_MINOR), (5.0, GRID_MAJOR)] {
            if step * self.view.zoom < 8.0 {
                continue;
            }
            let mut x = (lo.x / step).ceil() * step;
            while x <= hi.x {
                let sx = self.view.to_screen(rect, Point2::new(x, 0.0)).x;
                painter.line_segment(
                    [Pos2::new(sx, rect.top()), Pos2::new(sx, rect.bottom())],
                    Stroke::new(1.0, color),
                );
                x += step;
            }
            let mut y = (lo.y / step).ceil() * step;
            while y <= hi.y {
                let sy = self.view.to_screen(rect, Point2::new(0.0, y)).y;
                painter.line_segment(
                    [Pos2::new(rect.left(), sy), Pos2::new(rect.right(), sy)],
                    Stroke::new(1.0, color),
                );
                y += step;
            }
        }
    }

    fn draw_plan(&mut self, ctx: &egui::Context, painter: &egui::Painter, rect: Rect) {
        let ppp = ctx.pixels_per_point();
        let size = [
            (rect.width() * ppp).round().max(1.0) as usize,
            (rect.height() * ppp).round().max(1.0) as usize,
        ];
        let (lo, hi) = self.view.visible(rect);
        let key = PlanKey {
            revision: self.ed.revision(),
            level: self.ed.level,
            lo,
            hi,
            size,
            show_reference_lines: self.show_reference_lines,
        };
        if self.plan.as_ref().map(|p| &p.0) != Some(&key) {
            let started = std::time::Instant::now();
            let image = rasterize(
                self.ed.model(),
                self.ed.level,
                lo,
                hi,
                size,
                self.show_reference_lines,
            );
            self.last_plan_ms = started.elapsed().as_secs_f64() * 1000.0;
            match &mut self.plan {
                Some((k, tex)) => {
                    tex.set(image, TextureOptions::LINEAR);
                    *k = key;
                }
                None => {
                    let tex = ctx.load_texture("plan", image, TextureOptions::LINEAR);
                    self.plan = Some((key, tex));
                }
            }
        }
        if let Some((_, tex)) = &self.plan {
            painter.image(
                tex.id(),
                rect,
                Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)),
                Color32::WHITE,
            );
        }
    }

    fn draw_origin(&self, painter: &egui::Painter, rect: Rect) {
        let o = self.view.to_screen(rect, Point2::new(0.0, 0.0));
        let s = Stroke::new(1.0, Color32::from_rgb(120, 120, 130));
        painter.line_segment([o - Vec2::new(8.0, 0.0), o + Vec2::new(8.0, 0.0)], s);
        painter.line_segment([o - Vec2::new(0.0, 8.0), o + Vec2::new(0.0, 8.0)], s);
        painter.circle_stroke(o, 4.0, s);
    }

    fn polygon(&self, rect: Rect, poly: &[Point2]) -> Vec<Pos2> {
        poly.iter().map(|p| self.view.to_screen(rect, *p)).collect()
    }

    fn draw_selection(&self, painter: &egui::Painter, rect: Rect) {
        let model = self.ed.model();
        if let Some(h) = self.hover {
            if !self.ed.selection.contains(&h) {
                if let Some(poly) = outline(model, h) {
                    painter.add(Shape::closed_line(
                        self.polygon(rect, &poly),
                        Stroke::new(2.0, ACCENT.gamma_multiply(0.5)),
                    ));
                }
            }
        }
        for e in &self.ed.selection {
            let Some(poly) = outline(model, *e) else {
                continue;
            };
            let pts = self.polygon(rect, &poly);
            painter.add(Shape::closed_line(pts.clone(), Stroke::new(2.0, ACCENT)));
            // A door or window is dragged by its body; mark its middle.
            let spots: Vec<Point2> = match e {
                Element::Opening(id) => opening_geometry(model, *id)
                    .map(|g| vec![g.point((g.t0 + g.t1) / 2.0, g.exterior_offset)])
                    .unwrap_or_default(),
                _ => Vec::new(),
            };
            for p in spots {
                let c = self.view.to_screen(rect, p);
                painter.rect_filled(
                    Rect::from_center_size(c, Vec2::splat(7.0)),
                    CornerRadius::ZERO,
                    Color32::BLACK,
                );
                painter.rect_stroke(
                    Rect::from_center_size(c, Vec2::splat(7.0)),
                    CornerRadius::ZERO,
                    Stroke::new(1.0, Color32::WHITE),
                    StrokeKind::Inside,
                );
            }
            if let Element::Wall(id) = e {
                if let Some(w) = model.wall(*id) {
                    dashed(
                        painter,
                        self.view.to_screen(rect, w.start),
                        self.view.to_screen(rect, w.end),
                        REF_LINE,
                    );
                }
            }
        }
    }

    /// Handles of the selection: wall ends and slab corners (black
    /// squares, to stretch) and slab edges (white diamonds, to move the
    /// edge); the one under the cursor is highlighted.
    fn draw_hotspots(&self, painter: &egui::Painter, rect: Rect) {
        for h in hotspots(self.ed.model(), &self.ed.selection) {
            let c = self.view.to_screen(rect, h.point);
            let hot = self.hover_grip == Some(h.grip)
                || self.drag.as_ref().is_some_and(|d| d.grip == h.grip);
            let fill = if hot { ACCENT } else { Color32::BLACK };
            match h.grip {
                Grip::SlabEdge { .. } => {
                    let r = if hot { 6.0 } else { 4.5 };
                    let pts = vec![
                        c + Vec2::new(0.0, -r),
                        c + Vec2::new(r, 0.0),
                        c + Vec2::new(0.0, r),
                        c + Vec2::new(-r, 0.0),
                    ];
                    let inside = if hot { ACCENT } else { Color32::WHITE };
                    painter.add(Shape::convex_polygon(
                        pts,
                        inside,
                        Stroke::new(1.0, Color32::BLACK),
                    ));
                }
                _ => {
                    let size = Vec2::splat(if hot { 9.0 } else { 7.0 });
                    painter.rect_filled(Rect::from_center_size(c, size), CornerRadius::ZERO, fill);
                    painter.rect_stroke(
                        Rect::from_center_size(c, size),
                        CornerRadius::ZERO,
                        Stroke::new(1.0, Color32::WHITE),
                        StrokeKind::Inside,
                    );
                }
            }
        }
    }

    /// The selection being dragged: the outlines it would have, the line
    /// from where it was grabbed to the cursor, and the distance.
    fn draw_drag_preview(&self, painter: &egui::Painter, rect: Rect) {
        let (Some(d), Some(c)) = (&self.drag, self.cursor) else {
            return;
        };
        let to = d.target(c.point);
        let (a, b) = (
            self.view.to_screen(rect, d.from),
            self.view.to_screen(rect, to),
        );
        dashed(painter, a, b, Color32::from_gray(60));
        let variant = self.drag_preview.as_ref().is_some_and(|p| p.1);
        let mut problem = None;
        if let Some((_, _, ghost)) = &self.drag_preview {
            match ghost {
                Ok(polys) => {
                    for poly in polys {
                        let pts = self.polygon(rect, poly);
                        painter.add(Shape::closed_line(pts, Stroke::new(1.5, ACCENT)));
                    }
                }
                Err(e) => problem = Some(e.clone()),
            }
        }
        let action = match (d.grip, variant) {
            (Grip::Body, true) => "Copiar",
            (Grip::Body, false) => "Mover",
            (Grip::WallEnd { .. }, false) => "Estirar",
            (Grip::WallEnd { .. }, true) => "Estirar solo este muro",
            (Grip::SlabCorner { .. }, _) => "Esquina",
            (Grip::SlabEdge { .. }, false) => "Arista",
            (Grip::SlabEdge { .. }, true) => "Nueva esquina",
        };
        let delta = to - d.from;
        let mut text = if d.typed.is_empty() {
            format!(
                "{action}  {} mm   {}",
                fmt_mm(delta.length()),
                fmt_angle(delta.y.atan2(delta.x))
            )
        } else {
            format!(
                "{action}  {}▏   {}",
                d.typed,
                fmt_angle(delta.y.atan2(delta.x))
            )
        };
        if let Some(p) = problem {
            text.push_str(&format!("\nNo se puede: {p}"));
        }
        self.draw_label(painter, rect, c.point, text);
    }

    /// The room the magic wand would turn into a slab.
    fn draw_wand_preview(&self, painter: &egui::Painter, rect: Rect) {
        let (Tool::Slab, Some(c), Some((_, _, _, found))) =
            (self.tool, self.cursor, &self.wand_preview)
        else {
            return;
        };
        if self.slab_tool.is_drawing() || !(self.slab_settings.wand || self.space_held) {
            return;
        }
        match found {
            Ok(poly) => {
                let pts = self.polygon(rect, poly);
                painter.add(Shape::closed_line(pts, Stroke::new(2.0, ACCENT)));
                self.draw_label(
                    painter,
                    rect,
                    c.point,
                    format!(
                        "Varita mágica   S {}",
                        fmt_area(archisim_core::signed_area(poly).abs())
                    ),
                );
            }
            Err(e) => self.draw_label(painter, rect, c.point, format!("Varita mágica: {e}")),
        }
    }

    /// The wall about to be built, with its layers, its drawn line and a
    /// mark on its exterior side.
    fn draw_wall_preview(&self, painter: &egui::Painter, rect: Rect) {
        let (Tool::Wall, Some(a), Some(c)) = (self.tool, self.wall_tool.anchor(), self.cursor)
        else {
            return;
        };
        let b = self.wall_tool.typed_end(c.point).unwrap_or(c.point);
        let model = self.ed.model();
        let Some(wtype) = model.wall_type(self.wall_settings.wall_type) else {
            return;
        };
        let Some(u) = (b - a).normalized() else {
            return;
        };
        let n = u.perp_left();
        let probe = archisim_core::Wall {
            id: archisim_core::WallId(0),
            guid: archisim_core::GlobalId::new(),
            wall_type: wtype.id,
            level: self.ed.level,
            start: a,
            end: b,
            reference: self.wall_settings.reference,
            height: self.wall_settings.height,
            base_offset: self.wall_settings.base_offset,
        };
        let (ext, int) = probe.face_offsets(wtype.thickness());
        let band = |hi: f64, lo: f64| -> Vec<Pos2> {
            [a + n * hi, b + n * hi, b + n * lo, a + n * lo]
                .iter()
                .map(|p| self.view.to_screen(rect, *p))
                .collect()
        };
        let mut high = ext;
        for layer in &wtype.layers {
            let low = high - layer.thickness;
            let [r, g, bl] = model
                .material(layer.material)
                .map_or([200, 200, 200], |m| m.cut_color);
            painter.add(Shape::convex_polygon(
                band(high, low),
                Color32::from_rgba_unmultiplied(r, g, bl, 170),
                Stroke::new(0.5, Color32::from_gray(90)),
            ));
            high = low;
        }
        painter.add(Shape::closed_line(
            band(ext, int),
            Stroke::new(1.5, Color32::from_gray(30)),
        ));
        dashed(
            painter,
            self.view.to_screen(rect, a),
            self.view.to_screen(rect, b),
            REF_LINE,
        );

        // "ext." on the exterior side of façade walls, by the middle.
        if wtype.function == WallFunction::Exterior {
            let mid = (a + b) * 0.5;
            let out =
                (self.view.to_screen(rect, mid + n) - self.view.to_screen(rect, mid)).normalized();
            let label_pos = self.view.to_screen(rect, mid + n * ext) + out * 14.0;
            painter.text(
                label_pos,
                Align2::CENTER_CENTER,
                "ext.",
                FontId::proportional(11.0),
                Color32::from_gray(90),
            );
        }

        // Length and angle by the cursor.
        let len = a.distance(b);
        let angle = (b - a).y.atan2((b - a).x);
        let text = if self.wall_tool.typed.is_empty() {
            format!("L {} mm   {}", fmt_mm(len), fmt_angle(angle))
        } else {
            format!("L: {}▏   {}", self.wall_tool.typed, fmt_angle(angle))
        };
        self.draw_label(painter, rect, c.point, text);
    }

    /// A small label beside the cursor, flipped near the view's edges.
    fn draw_label(&self, painter: &egui::Painter, rect: Rect, at: Point2, text: String) {
        let cursor = self.view.to_screen(rect, at);
        let galley = painter.layout_no_wrap(text, FontId::monospace(12.0), Color32::from_gray(20));
        let mut pos = cursor + Vec2::new(16.0, 18.0);
        if pos.x + galley.size().x + 8.0 > rect.right() {
            pos.x = cursor.x - 16.0 - galley.size().x;
        }
        if pos.y + galley.size().y + 8.0 > rect.bottom() {
            pos.y = cursor.y - 18.0 - galley.size().y;
        }
        let bg = Rect::from_min_size(pos, galley.size()).expand(4.0);
        painter.rect_filled(
            bg,
            CornerRadius::same(3),
            Color32::from_rgba_unmultiplied(255, 255, 230, 235),
        );
        painter.rect_stroke(
            bg,
            CornerRadius::same(3),
            Stroke::new(1.0, Color32::from_gray(150)),
            StrokeKind::Inside,
        );
        painter.galley(pos, galley, Color32::BLACK);
    }

    /// The slab outline being drawn: placed corners, the edge to the cursor
    /// and, dashed, the edge that will close it.
    fn draw_slab_preview(&self, painter: &egui::Painter, rect: Rect) {
        let (Tool::Slab, Some(c)) = (self.tool, self.cursor) else {
            return;
        };
        let corners = self.slab_tool.corners();
        let Some(&last) = corners.last() else {
            return;
        };
        let next = self.slab_tool.typed_end(c.point).unwrap_or(c.point);
        let mut pts: Vec<Point2> = corners.to_vec();
        pts.push(next);
        let screen = self.polygon(rect, &pts);
        let ink = Stroke::new(1.5, ACCENT);
        painter.add(Shape::line(screen.clone(), ink));
        if pts.len() >= 3 {
            dashed(painter, screen[screen.len() - 1], screen[0], ACCENT);
        }
        for p in &screen[..screen.len() - 1] {
            painter.rect_filled(
                Rect::from_center_size(*p, Vec2::splat(6.0)),
                CornerRadius::ZERO,
                ACCENT,
            );
        }
        let len = last.distance(next);
        let angle = (next - last).y.atan2((next - last).x);
        let mut text = if self.slab_tool.typed.is_empty() {
            format!("L {} mm   {}", fmt_mm(len), fmt_angle(angle))
        } else {
            format!("L: {}▏   {}", self.slab_tool.typed, fmt_angle(angle))
        };
        if pts.len() >= 3 {
            text.push_str(&format!(
                "   S {}",
                fmt_area(archisim_core::signed_area(&pts).abs())
            ));
        }
        self.draw_label(painter, rect, c.point, text);
    }

    /// Where the door or window would go: its rough opening across the
    /// wall, the frame, and for doors the leaf and its swing.
    fn draw_opening_preview(&self, painter: &egui::Painter, rect: Rect) {
        if !matches!(self.tool, Tool::Door | Tool::Window) {
            return;
        }
        let (Some(pl), Some(c)) = (&self.placement, self.cursor) else {
            return;
        };
        let g = &pl.geometry;
        let color = if pl.problem.is_some() {
            Color32::from_rgb(210, 40, 40)
        } else {
            ACCENT
        };
        let to = |t: f64, off: f64| self.view.to_screen(rect, g.point(t, off));
        let foot = self.polygon(rect, &g.footprint());
        painter.add(Shape::convex_polygon(
            foot,
            color.gamma_multiply(0.18),
            Stroke::new(1.5, color),
        ));
        // Frame faces.
        let stroke = Stroke::new(1.0, color);
        for off in [g.frame_exterior, g.frame_interior] {
            painter.line_segment([to(g.t0, off), to(g.t1, off)], stroke);
        }
        match g.kind {
            OpeningKind::Door(op) => {
                use archisim_core::DoorOperation as D;
                let face = g.swing_frame_face();
                let out = g.swing_dir();
                let mut leaves: Vec<(f64, f64)> = Vec::new(); // (hinge t, free t)
                match op {
                    D::SingleSwing => {
                        if g.hinge_at_start() {
                            leaves.push((g.t0, g.t1));
                        } else {
                            leaves.push((g.t1, g.t0));
                        }
                    }
                    D::DoubleSwing => {
                        let mid = (g.t0 + g.t1) / 2.0;
                        leaves.push((g.t0, mid));
                        leaves.push((g.t1, mid));
                    }
                    D::Sliding => {}
                }
                for (hinge, free) in leaves {
                    let r = (free - hinge).abs();
                    let h = g.point(hinge, face);
                    let along = g.u * (free - hinge).signum();
                    // Leaf open at 90°, and the arc its edge sweeps.
                    painter.line_segment(
                        [
                            self.view.to_screen(rect, h),
                            self.view.to_screen(rect, h + out * r),
                        ],
                        Stroke::new(2.0, color),
                    );
                    let arc: Vec<Pos2> = (0..=24)
                        .map(|i| {
                            let a = i as f64 / 24.0 * std::f64::consts::FRAC_PI_2;
                            self.view
                                .to_screen(rect, h + along * (r * a.cos()) + out * (r * a.sin()))
                        })
                        .collect();
                    painter.add(Shape::line(arc, Stroke::new(1.0, color)));
                }
            }
            OpeningKind::Window(_) => {
                let mid = (g.frame_exterior + g.frame_interior) / 2.0;
                painter.line_segment([to(g.t0, mid), to(g.t1, mid)], Stroke::new(2.0, color));
            }
        }
        let side = match g.opens_to {
            WallSide::Interior => "abre al interior",
            WallSide::Exterior => "abre al exterior",
        };
        let text = match &pl.problem {
            Some(p) => format!("No cabe: {p}"),
            None => format!(
                "{} mm ◂ ▸ {} mm   {side}",
                fmt_mm(pl.before),
                fmt_mm(pl.after)
            ),
        };
        self.draw_label(painter, rect, c.point, text);
    }

    fn draw_cursor(&self, painter: &egui::Painter, rect: Rect) {
        let points =
            self.tool.draws_points() && !(self.tool == Tool::Slab && self.slab_settings.wand);
        let (true, Some(c)) = (points || self.drag.is_some(), self.cursor) else {
            return;
        };
        let p = self.view.to_screen(rect, c.point);
        let stroke = Stroke::new(1.5, Color32::from_rgb(0, 150, 60));
        match c.kind {
            SnapKind::Endpoint | SnapKind::Corner => {
                painter.rect_stroke(
                    Rect::from_center_size(p, Vec2::splat(11.0)),
                    CornerRadius::ZERO,
                    stroke,
                    StrokeKind::Middle,
                );
            }
            SnapKind::Midpoint => {
                let pts = vec![
                    p + Vec2::new(0.0, -7.0),
                    p + Vec2::new(6.5, 5.0),
                    p + Vec2::new(-6.5, 5.0),
                ];
                painter.add(Shape::closed_line(pts, stroke));
            }
            SnapKind::OnEdge => {
                painter.line_segment([p + Vec2::new(-5.0, -5.0), p + Vec2::new(5.0, 5.0)], stroke);
                painter.line_segment([p + Vec2::new(-5.0, 5.0), p + Vec2::new(5.0, -5.0)], stroke);
            }
            SnapKind::Constrained | SnapKind::Tracked | SnapKind::Free => {}
        }
        let ink = Stroke::new(1.0, Color32::from_gray(40));
        painter.line_segment([p - Vec2::new(12.0, 0.0), p + Vec2::new(12.0, 0.0)], ink);
        painter.line_segment([p - Vec2::new(0.0, 12.0), p + Vec2::new(0.0, 12.0)], ink);
    }

    // ---- Properties ------------------------------------------------------------

    fn properties(&mut self, ui: &mut Ui) {
        ui.add_space(4.0);
        ui.heading("Propiedades");
        ui.separator();
        let selection: Vec<Element> = self.ed.selection.iter().copied().collect();
        match selection.as_slice() {
            [] => self.project_properties(ui),
            [Element::Wall(id)] => self.wall_properties(ui, *id),
            [Element::Opening(id)] => self.opening_properties(ui, *id),
            [Element::Slab(id)] => self.slab_properties(ui, *id),
            [Element::Dimension(id)] => self.dimension_properties(ui, *id),
            [Element::Mark(id)] => self.mark_properties(ui, *id),
            many => self.many_properties(ui, many),
        }
    }

    fn project_properties(&mut self, ui: &mut Ui) {
        let model = self.ed.model();
        let name = model.project_name.clone();
        let counts = (
            model.walls().count(),
            model.slabs().count(),
            model.openings().count(),
        );
        let guid = model.project_guid.to_string();
        egui::Grid::new("project-props")
            .num_columns(2)
            .spacing([10.0, 6.0])
            .show(ui, |ui| {
                ui.label("Proyecto");
                if let Some(new) = text_field(ui, Id::unique("project-name"), &name) {
                    let r = self.ed.rename_project(&new);
                    self.report(r, "Proyecto renombrado.");
                }
                ui.end_row();
                ui.label("Fichero");
                ui.label(self.ed.display_name());
                ui.end_row();
                ui.label("Contenido");
                ui.label(format!(
                    "{} muros · {} forjados · {} huecos",
                    counts.0, counts.1, counts.2
                ));
                ui.end_row();
                ui.label("IfcProject");
                guid_label(ui, &guid);
                ui.end_row();
            });
        ui.add_space(8.0);
        ui.label(
            egui::RichText::new("Selecciona un elemento para ver y cambiar sus propiedades.")
                .weak(),
        );
    }

    fn wall_properties(&mut self, ui: &mut Ui, id: archisim_core::WallId) {
        let model = self.ed.model();
        let Some(w) = model.wall(id).cloned() else {
            return;
        };
        let wtype = model.wall_type(w.wall_type).cloned();
        let types: Vec<(WallTypeId, String)> =
            model.wall_types().map(|t| (t.id, t.name.clone())).collect();
        let openings = model.openings_in_wall(id).count();
        let thickness = wtype.as_ref().map_or(0.0, |t| t.thickness());
        let level = model
            .level(w.level)
            .map(|l| l.name.clone())
            .unwrap_or_default();
        ui.label(egui::RichText::new("Muro · IfcWall").strong());
        ui.add_space(4.0);
        let walls = [id];
        egui::Grid::new("wall-props")
            .num_columns(2)
            .spacing([10.0, 6.0])
            .show(ui, |ui| {
                ui.label("Tipo");
                let mut t = w.wall_type;
                ComboBox::from_id_salt("wall-type")
                    .width(165.0)
                    .truncate()
                    .selected_text(wtype.as_ref().map_or("—".into(), |t| t.name.clone()))
                    .show_ui(ui, |ui| {
                        for (tid, name) in &types {
                            ui.selectable_value(&mut t, *tid, name);
                        }
                    });
                if t != w.wall_type {
                    let r = self.ed.set_walls_type(&walls, t);
                    self.report(r, "Tipo de muro cambiado.");
                }
                ui.end_row();
                ui.label("Espesor");
                ui.label(format!("{} mm", fmt_mm(thickness)));
                ui.end_row();
                ui.label("Línea de referencia");
                let mut r = w.reference;
                reference_combo(ui, "wall-ref", &mut r);
                if r != w.reference {
                    let res = self.ed.set_walls_reference(&walls, r);
                    self.report(
                        res,
                        "Línea de referencia cambiada (la línea se queda en su sitio).",
                    );
                }
                ui.end_row();
                ui.label("Longitud (mm)");
                if let Some(v) = length_field(ui, Id::unique(("wall-len", id)), w.length(), 90.0) {
                    let r = self.ed.set_wall_length(id, v);
                    self.report(r, "Longitud cambiada desde el inicio del muro.");
                }
                ui.end_row();
                ui.label("Altura (mm)");
                if let Some(v) = length_field(ui, Id::unique(("wall-h", id)), w.height, 90.0) {
                    let r = self.ed.set_walls_height(&walls, v);
                    self.report(r, "Altura cambiada.");
                }
                ui.end_row();
                ui.label("Desfase de base (mm)");
                if let Some(v) = length_field(ui, Id::unique(("wall-b", id)), w.base_offset, 90.0) {
                    let r = self.ed.set_walls_base_offset(&walls, v);
                    self.report(r, "Desfase de base cambiado.");
                }
                ui.end_row();
                ui.label("Planta");
                ui.label(level);
                ui.end_row();
                ui.label("Inicio (m)");
                ui.label(format!("{}; {}", fmt_m(w.start.x), fmt_m(w.start.y)));
                ui.end_row();
                ui.label("Fin (m)");
                ui.label(format!("{}; {}", fmt_m(w.end.x), fmt_m(w.end.y)));
                ui.end_row();
                ui.label("Puertas y ventanas");
                ui.label(openings.to_string());
                ui.end_row();
                ui.label("GlobalId");
                guid_label(ui, &w.guid.to_string());
                ui.end_row();
            });
        ui.add_space(6.0);
        if ui
            .button("⇄ Invertir (cara exterior al otro lado)")
            .clicked()
        {
            let r = self.ed.flip_walls(&walls);
            self.report(r, "Muro invertido.");
        }
        if let Some(t) = wtype {
            ui.add_space(8.0);
            ui.label(egui::RichText::new("Capas (de exterior a interior)").strong());
            for l in &t.layers {
                let name = self
                    .ed
                    .model()
                    .material(l.material)
                    .map(|m| m.name.clone())
                    .unwrap_or_default();
                ui.horizontal(|ui| {
                    let [r, g, b] = self
                        .ed
                        .model()
                        .material(l.material)
                        .map_or([200; 3], |m| m.cut_color);
                    let (rr, _) = ui.allocate_exact_size(Vec2::splat(12.0), Sense::hover());
                    ui.painter()
                        .rect_filled(rr, CornerRadius::ZERO, Color32::from_rgb(r, g, b));
                    ui.painter().rect_stroke(
                        rr,
                        CornerRadius::ZERO,
                        Stroke::new(1.0, Color32::GRAY),
                        StrokeKind::Inside,
                    );
                    ui.label(format!("{} mm  {}", fmt_mm(l.thickness), name));
                });
            }
        }
    }

    fn opening_properties(&mut self, ui: &mut Ui, id: archisim_core::OpeningId) {
        let model = self.ed.model();
        let Some(o) = model.opening(id).cloned() else {
            return;
        };
        let Some(ot) = model.opening_type(o.opening_type).cloned() else {
            return;
        };
        let is_door = ot.is_door();
        let types: Vec<(archisim_core::OpeningTypeId, String)> = model
            .opening_types()
            .filter(|t| t.is_door() == is_door)
            .map(|t| (t.id, t.name.clone()))
            .collect();
        let host = model
            .wall(o.wall)
            .map(|w| w.guid.to_string())
            .unwrap_or_default();
        let operation = match ot.kind {
            OpeningKind::Door(d) => match d {
                archisim_core::DoorOperation::SingleSwing => "Abatible 1 hoja",
                archisim_core::DoorOperation::DoubleSwing => "Abatible 2 hojas",
                archisim_core::DoorOperation::Sliding => "Corredera",
            },
            OpeningKind::Window(w) => match w {
                archisim_core::WindowOperation::Fixed => "Fija",
                archisim_core::WindowOperation::SingleCasement => "Practicable 1 hoja",
                archisim_core::WindowOperation::DoubleCasement => "Practicable 2 hojas",
                archisim_core::WindowOperation::Sliding => "Corredera 2 hojas",
            },
        };
        ui.label(
            egui::RichText::new(if is_door {
                "Puerta · IfcDoor"
            } else {
                "Ventana · IfcWindow"
            })
            .strong(),
        );
        ui.add_space(4.0);
        egui::Grid::new("opening-props")
            .num_columns(2)
            .spacing([10.0, 6.0])
            .show(ui, |ui| {
                ui.label("Tipo");
                let mut t = o.opening_type;
                ComboBox::from_id_salt("opening-type")
                    .width(165.0)
                    .truncate()
                    .selected_text(ot.name.clone())
                    .show_ui(ui, |ui| {
                        for (tid, name) in &types {
                            ui.selectable_value(&mut t, *tid, name);
                        }
                    });
                if t != o.opening_type {
                    let r = self.ed.set_opening_type(id, t);
                    self.report(r, "Tipo cambiado.");
                }
                ui.end_row();
                ui.label("Funcionamiento");
                ui.label(operation);
                ui.end_row();
                ui.label("Hueco de obra");
                ui.label(format!("{} × {} mm", fmt_mm(ot.width), fmt_mm(ot.height)));
                ui.end_row();
                ui.label("Posición (mm)")
                    .on_hover_text("Del centro del hueco, medida desde el inicio del muro");
                if let Some(v) = length_field(ui, Id::unique(("op-along", id)), o.along, 90.0) {
                    let r = self.ed.move_opening(id, v, o.sill);
                    self.report(r, "Posición cambiada.");
                }
                ui.end_row();
                ui.label("Antepecho (mm)");
                if let Some(v) = length_field(ui, Id::unique(("op-sill", id)), o.sill, 90.0) {
                    let r = self.ed.move_opening(id, o.along, v);
                    self.report(r, "Antepecho cambiado.");
                }
                ui.end_row();
                ui.label("Abre hacia");
                let mut side = o.opens_to;
                ComboBox::from_id_salt("op-side")
                    .selected_text(side_name(side))
                    .show_ui(ui, |ui| {
                        ui.selectable_value(
                            &mut side,
                            WallSide::Interior,
                            side_name(WallSide::Interior),
                        );
                        ui.selectable_value(
                            &mut side,
                            WallSide::Exterior,
                            side_name(WallSide::Exterior),
                        );
                    });
                ui.end_row();
                ui.label("Mano (UNE-EN 12519)");
                let mut hand = o.hand;
                ComboBox::from_id_salt("op-hand")
                    .selected_text(hand_name(hand))
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut hand, Hand::Left, hand_name(Hand::Left));
                        ui.selectable_value(&mut hand, Hand::Right, hand_name(Hand::Right));
                    });
                if side != o.opens_to || hand != o.hand {
                    let r = self.ed.set_opening_handing(id, side, hand);
                    self.report(r, "Apertura cambiada.");
                }
                ui.end_row();
                ui.label("Muro anfitrión");
                guid_label(ui, &host);
                ui.end_row();
                ui.label("GlobalId");
                guid_label(ui, &o.guid.to_string());
                ui.end_row();
            });
    }

    fn slab_properties(&mut self, ui: &mut Ui, id: archisim_core::SlabId) {
        let model = self.ed.model();
        let Some(s) = model.slab(id).cloned() else {
            return;
        };
        let st = model.slab_type(s.slab_type).cloned();
        let types: Vec<(archisim_core::SlabTypeId, String)> =
            model.slab_types().map(|t| (t.id, t.name.clone())).collect();
        ui.label(egui::RichText::new("Forjado · IfcSlab").strong());
        ui.add_space(4.0);
        egui::Grid::new("slab-props")
            .num_columns(2)
            .spacing([10.0, 6.0])
            .show(ui, |ui| {
                ui.label("Tipo");
                let mut t = s.slab_type;
                ComboBox::from_id_salt("slab-type")
                    .width(165.0)
                    .truncate()
                    .selected_text(st.as_ref().map_or("—".into(), |t| t.name.clone()))
                    .show_ui(ui, |ui| {
                        for (tid, name) in &types {
                            ui.selectable_value(&mut t, *tid, name);
                        }
                    });
                if t != s.slab_type {
                    let r = self.ed.set_slab_type(id, t);
                    self.report(r, "Tipo de forjado cambiado.");
                }
                ui.end_row();
                ui.label("Canto");
                ui.label(format!(
                    "{} mm",
                    fmt_mm(st.as_ref().map_or(0.0, |t| t.thickness()))
                ));
                ui.end_row();
                ui.label("Cara superior (mm)");
                if let Some(v) = length_field(ui, Id::unique(("slab-top", id)), s.top_offset, 90.0)
                {
                    let r = self.ed.set_slab_offset(id, v);
                    self.report(r, "Desfase cambiado.");
                }
                ui.end_row();
                ui.label("Superficie");
                ui.label(fmt_area(s.area()));
                ui.end_row();
                ui.label("Vértices");
                ui.label(s.outline.len().to_string());
                ui.end_row();
                ui.label("GlobalId");
                guid_label(ui, &s.guid.to_string());
                ui.end_row();
            });
    }

    fn many_properties(&mut self, ui: &mut Ui, many: &[Element]) {
        let walls: Vec<archisim_core::WallId> = many
            .iter()
            .filter_map(|e| match e {
                Element::Wall(id) => Some(*id),
                _ => None,
            })
            .collect();
        let openings = many
            .iter()
            .filter(|e| matches!(e, Element::Opening(_)))
            .count();
        let slabs = many
            .iter()
            .filter(|e| matches!(e, Element::Slab(_)))
            .count();
        let annotations = many
            .iter()
            .filter(|e| matches!(e, Element::Dimension(_) | Element::Mark(_)))
            .count();
        ui.label(egui::RichText::new(format!("{} elementos seleccionados", many.len())).strong());
        ui.label(format!(
            "{} muros · {} puertas y ventanas · {} forjados · {} cotas",
            walls.len(),
            openings,
            slabs,
            annotations
        ));
        ui.add_space(6.0);
        if walls.len() == many.len() {
            let model = self.ed.model();
            let types: Vec<(WallTypeId, String)> =
                model.wall_types().map(|t| (t.id, t.name.clone())).collect();
            let first = model.wall(walls[0]).cloned();
            let same = |f: &dyn Fn(&archisim_core::Wall) -> String| {
                let vals: Vec<String> =
                    walls.iter().filter_map(|w| model.wall(*w)).map(f).collect();
                if vals.windows(2).all(|p| p[0] == p[1]) {
                    vals.first().cloned()
                } else {
                    None
                }
            };
            let type_text = same(&|w| {
                model
                    .wall_type(w.wall_type)
                    .map(|t| t.name.clone())
                    .unwrap_or_default()
            });
            let height = same(&|w| fmt_mm(w.height));
            ui.label("Cambiar en todos los muros seleccionados:");
            egui::Grid::new("walls-props")
                .num_columns(2)
                .spacing([10.0, 6.0])
                .show(ui, |ui| {
                    ui.label("Tipo");
                    let mut t = WallTypeId(0);
                    ComboBox::from_id_salt("walls-type")
                        .width(165.0)
                        .truncate()
                        .selected_text(type_text.unwrap_or_else(|| "(varios)".into()))
                        .show_ui(ui, |ui| {
                            for (tid, name) in &types {
                                ui.selectable_value(&mut t, *tid, name);
                            }
                        });
                    if t != WallTypeId(0) {
                        let r = self.ed.set_walls_type(&walls, t);
                        self.report(r, "Tipo cambiado en los muros seleccionados.");
                    }
                    ui.end_row();
                    ui.label("Altura (mm)");
                    let shown = first.map_or(0.0, |w| w.height);
                    if height.is_none() {
                        ui.label("(varias)");
                        ui.end_row();
                        ui.label("Nueva altura (mm)");
                    }
                    if let Some(v) = length_field(ui, Id::unique("walls-h"), shown, 90.0) {
                        let r = self.ed.set_walls_height(&walls, v);
                        self.report(r, "Altura cambiada en los muros seleccionados.");
                    }
                    ui.end_row();
                });
            if ui.button("⇄ Invertir").clicked() {
                let r = self.ed.flip_walls(&walls);
                self.report(r, "Muros invertidos.");
            }
        }
        ui.add_space(6.0);
        if ui.button("🗑 Borrar selección").clicked() {
            self.delete();
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
        "M · F · P · V · C",
        "Muro · Forjado · Puerta · Ventana · Cota",
    ),
    ("Esc", "Terminar / herramienta flecha / anular selección"),
    ("Mayús (dibujando)", "Ángulos múltiplos de 45°"),
    (
        "Números + Intro (dibujando)",
        "Longitud del muro en mm (o 3,5m · 350cm)",
    ),
    (
        "Intro / doble clic / botón derecho",
        "Terminar la cadena de muros",
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
    }
}

/// Slab tool settings for a model: its first type, top face on the level.
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

fn contour_name(c: Contour) -> &'static str {
    match c {
        Contour::InteriorFace => "Cara interior",
        Contour::WallAxis => "Eje de muros",
        Contour::ExteriorFace => "Cara exterior",
    }
}

fn side_name(s: WallSide) -> &'static str {
    match s {
        WallSide::Interior => "Cara interior",
        WallSide::Exterior => "Cara exterior",
    }
}

fn hand_name(h: Hand) -> &'static str {
    match h {
        Hand::Left => "Izquierda",
        Hand::Right => "Derecha",
    }
}

fn reference_name(r: ReferenceLine) -> &'static str {
    match r {
        ReferenceLine::ExteriorFace => "Cara exterior",
        ReferenceLine::Center => "Eje",
        ReferenceLine::InteriorFace => "Cara interior",
    }
}

fn reference_combo(ui: &mut Ui, id: &str, r: &mut ReferenceLine) {
    ComboBox::from_id_salt(id)
        .selected_text(reference_name(*r))
        .show_ui(ui, |ui| {
            for v in [
                ReferenceLine::ExteriorFace,
                ReferenceLine::Center,
                ReferenceLine::InteriorFace,
            ] {
                ui.selectable_value(r, v, reference_name(v));
            }
        });
}

fn shortcut_button<'a>(text: &'a str, shortcut: &'a str) -> egui::Button<'a> {
    egui::Button::new(text).shortcut_text(shortcut)
}

fn guid_label(ui: &mut Ui, guid: &str) {
    ui.add(egui::Label::new(egui::RichText::new(guid).monospace().small()).selectable(true));
}

fn dashed(painter: &egui::Painter, a: Pos2, b: Pos2, color: Color32) {
    painter.extend(Shape::dashed_line(
        &[a, b],
        Stroke::new(1.0, color),
        8.0,
        4.0,
    ));
}

/// A text field that edits a length in millimetres and only reports a value
/// when the user presses Enter or leaves the field (Escape cancels), so
/// each change is one undo step.
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

fn text_field(ui: &mut Ui, id: Id, value: &str) -> Option<String> {
    text_edit_committed(ui, id, value, 170.0).filter(|t| t != value)
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
mod levels_ui;
mod view3d_ui;
pub use view3d_ui::{View3dState, ViewMode};

#[cfg(test)]
mod tests;
