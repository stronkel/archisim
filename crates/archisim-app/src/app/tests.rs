//! The whole window driven without a screen: mouse and keyboard events go
//! in, the model comes out. This is how an architect would draw, click by
//! click.

use super::*;
use crate::dim_tool::DimMode;
use archisim_core::SlabFunction;
use archisim_geom::{wall_geometry, EndCut};
use eframe::egui::{Event, Modifiers, PointerButton, RawInput};

/// The test window: a common laptop screen, with room for the toolbox,
/// the properties, the navigator and the drawing.
const SCREEN: (f32, f32) = (1600.0, 1000.0);

struct Harness {
    ctx: egui::Context,
    app: ArchiSimApp,
    time: f64,
    pos: Pos2,
    modifiers: Modifiers,
    events: Vec<Event>,
}

impl Harness {
    fn new(ed: Editor) -> Self {
        let mut h = Self {
            ctx: egui::Context::default(),
            app: ArchiSimApp::new(ed),
            time: 0.0,
            pos: Pos2::new(640.0, 400.0),
            modifiers: Modifiers::NONE,
            events: Vec::new(),
        };
        // The drawing tests were written with ArchiCAD's keys (M, F… at
        // once); the AutoCAD command line has its own tests.
        h.app.keys_autocad = false;
        h.frame();
        h.frame();
        h
    }

    fn frame(&mut self) {
        let input = RawInput {
            screen_rect: Some(Rect::from_min_size(
                Pos2::ZERO,
                Vec2::new(SCREEN.0, SCREEN.1),
            )),
            time: Some(self.time),
            events: std::mem::take(&mut self.events),
            ..Default::default()
        };
        let app = &mut self.app;
        let mut output = self.ctx.run_ui(input, |ui| app.show(ui));
        output.textures_delta.clear();
        self.time += 0.05;
    }

    /// A frame `seconds` later (for what happens now and then: looking at
    /// the shared folder of a team).
    fn frame_after(&mut self, seconds: f64) {
        self.time += seconds;
        self.frame();
    }

    fn screen(&self, p: Point2) -> Pos2 {
        self.app.view.to_screen(self.app.canvas, p)
    }

    fn move_to(&mut self, p: Point2) {
        self.pos = self.screen(p);
        self.events.push(Event::PointerMoved(self.pos));
        self.frame();
    }

    fn button(&mut self, button: PointerButton, pressed: bool) {
        self.events.push(Event::PointerButton {
            pos: self.pos,
            button,
            pressed,
            modifiers: self.modifiers,
        });
        self.frame();
    }

    fn click_at(&mut self, p: Point2) {
        self.move_to(p);
        self.button(PointerButton::Primary, true);
        self.button(PointerButton::Primary, false);
        // Never mistaken for a double click.
        self.time += 1.0;
        self.frame();
    }

    /// A click at a point of the screen (a label, an arrow).
    fn click_screen(&mut self, pos: Pos2) {
        self.pos = pos;
        self.events.push(Event::PointerMoved(pos));
        self.frame();
        self.button(PointerButton::Primary, true);
        self.button(PointerButton::Primary, false);
        self.time += 1.0;
        self.frame();
    }

    fn drag(&mut self, from: Point2, to: Point2) {
        self.move_to(from);
        self.button(PointerButton::Primary, true);
        for i in 1..=5 {
            let t = i as f64 / 5.0;
            self.move_to(from + (to - from) * t);
        }
        self.button(PointerButton::Primary, false);
        self.frame();
    }

    fn key(&mut self, key: Key) {
        for pressed in [true, false] {
            self.events.push(Event::Key {
                key,
                physical_key: None,
                pressed,
                repeat: false,
                modifiers: self.modifiers,
            });
        }
        self.frame();
    }

    fn type_text(&mut self, text: &str) {
        self.events.push(Event::Text(text.into()));
        self.frame();
    }

    fn set_modifiers(&mut self, m: Modifiers) {
        self.modifiers = m;
        self.events.push(Event::ModifiersChanged(m));
        self.frame();
    }

    fn model(&self) -> &Model {
        self.app.ed.model()
    }
}

fn p(x: f64, y: f64) -> Point2 {
    Point2::new(x, y)
}

/// Draws a 12 × 9 m house outline with the wall tool, like the demo house:
/// clockwise along the exterior faces, the north wall by typing its length.
fn draw_outline(h: &mut Harness) {
    h.type_text("m");
    h.key(Key::M);
    assert_eq!(h.app.tool, Tool::Wall);
    h.click_at(p(0.0, 0.0));
    h.click_at(p(0.0, 9.0));
    // North wall: cursor roughly east, Shift for a straight line, 12 m typed.
    h.set_modifiers(Modifiers::SHIFT);
    h.move_to(p(5.0, 9.3));
    h.type_text("12000");
    assert_eq!(h.app.wall_tool.typed, "12000");
    h.key(Key::Enter);
    h.set_modifiers(Modifiers::NONE);
    h.click_at(p(12.0, 0.0));
    // Back on the first point: the outline closes and the chain ends.
    h.click_at(p(0.002, -0.003));
}

/// Inside the outline drawn by [`draw_outline`]: a ground slab by its four
/// exterior corners, a door in the south façade and a window in the north
/// one.
fn furnish(h: &mut Harness) {
    h.key(Key::Escape);
    h.type_text("f");
    assert_eq!(h.app.tool, Tool::Slab);
    for c in [p(0.0, 0.0), p(0.0, 9.0), p(12.0, 9.0), p(12.0, 0.0)] {
        // A little off: the corners snap.
        h.click_at(c + Point2::new(0.03, -0.02));
    }
    h.click_at(p(0.02, 0.01));
    assert!(!h.app.slab_tool.is_drawing(), "{:?}", h.app.message);

    // Door: on the south façade (drawn east → west), cursor on the room side.
    h.type_text("p");
    assert_eq!(h.app.tool, Tool::Door);
    h.click_at(p(3.0, 0.2));
    // Window: on the north façade, 3 m from the west corner.
    h.type_text("v");
    assert_eq!(h.app.tool, Tool::Window);
    h.click_at(p(3.0, 8.8));
}

#[test]
fn slab_door_and_window_tools() {
    let mut h = Harness::new(Editor::new_project());
    draw_outline(&mut h);
    furnish(&mut h);
    let m = h.model();
    let slab = m.slabs().next().expect("a slab");
    assert_eq!(slab.area(), 108.0);
    assert_eq!(
        m.slab_type(slab.slab_type).unwrap().function,
        SlabFunction::BaseSlab
    );
    let (doors, windows): (Vec<_>, Vec<_>) = m
        .openings()
        .partition(|o| m.opening_type(o.opening_type).unwrap().is_door());
    assert_eq!((doors.len(), windows.len()), (1, 1), "{:?}", h.app.message);
    let door = doors[0];
    let south = m.walls().nth(3).unwrap();
    assert_eq!(door.wall, south.id);
    // The south wall runs from x = 12 to x = 0: 3 m from the west corner is
    // 9 m from its start.
    assert!((door.along - 9.0).abs() < 1e-9, "{}", door.along);
    assert_eq!((door.sill, door.opens_to), (0.0, WallSide::Interior));
    let window = windows[0];
    assert_eq!(window.wall, m.walls().nth(1).unwrap().id);
    assert!((window.along - 3.0).abs() < 1e-9);
    assert_eq!(window.sill, 0.9);
    assert_eq!(h.app.ed.undo_label(), Some("Colocar ventana"));

    // A second window on top of the first is refused and explained.
    let before = h.model().openings().count();
    h.click_at(p(3.2, 8.8));
    assert_eq!(h.model().openings().count(), before);
    let msg = h.app.message.clone().unwrap();
    assert!(msg.error && msg.text.contains("solaparía"), "{msg:?}");

    // Escape goes back to the arrow.
    h.key(Key::Escape);
    assert_eq!(h.app.tool, Tool::Arrow);
}

#[test]
fn slab_outline_with_typed_lengths_and_escape() {
    let mut h = Harness::new(Editor::new_project());
    h.type_text("f");
    h.click_at(p(0.0, 0.0));
    h.set_modifiers(Modifiers::SHIFT);
    h.move_to(p(2.0, 0.1));
    h.type_text("4000");
    h.key(Key::Enter);
    h.move_to(p(4.1, 2.0));
    h.type_text("3000");
    h.key(Key::Enter);
    h.set_modifiers(Modifiers::NONE);
    assert_eq!(h.app.slab_tool.corners().len(), 3);
    // A wrong corner, taken back with Escape.
    h.click_at(p(7.0, 7.0));
    h.key(Key::Escape);
    assert_eq!(h.app.slab_tool.corners().len(), 3);
    h.click_at(p(0.0, 3.0));
    // Enter closes it.
    h.key(Key::Enter);
    let m = h.model();
    assert_eq!(m.slabs().count(), 1, "{:?}", h.app.message);
    assert_eq!(m.slabs().next().unwrap().area(), 12.0);
}

#[test]
fn draws_a_closed_outline_with_mitred_corners() {
    let mut h = Harness::new(Editor::new_project());
    draw_outline(&mut h);
    let m = h.model();
    let walls: Vec<_> = m.walls().cloned().collect();
    assert_eq!(walls.len(), 4, "{:?}", h.app.message);
    let corners = [p(0.0, 0.0), p(0.0, 9.0), p(12.0, 9.0), p(12.0, 0.0)];
    for (i, w) in walls.iter().enumerate() {
        assert!(
            w.start.approx_eq(corners[i]),
            "wall {i} starts at {:?}",
            w.start
        );
        assert!(
            w.end.approx_eq(corners[(i + 1) % 4]),
            "wall {i} ends at {:?}",
            w.end
        );
        assert_eq!(w.reference, ReferenceLine::ExteriorFace);
        assert_eq!(w.height, 3.0);
        // Every corner is joined with a mitre at both ends.
        let g = wall_geometry(m, w.id).unwrap();
        assert!(matches!(g.start_cut, EndCut::Along(_)));
        assert!(matches!(g.end_cut, EndCut::Along(_)));
    }
    // Coordinates are exact, not 11.99999.
    assert_eq!(walls[1].end, p(12.0, 9.0));
    assert!(!h.app.wall_tool.is_drawing());
    assert_eq!(h.app.ed.undo_label(), Some("Crear muro"));
    assert!(h.app.ed.is_modified());
}

#[test]
fn partitions_snap_to_wall_faces_and_selection_edits() {
    let mut h = Harness::new(Editor::new_project());
    draw_outline(&mut h);
    let partition = h
        .model()
        .wall_types()
        .find(|t| t.name.starts_with("Tabique"))
        .unwrap()
        .id;
    h.app.wall_settings.wall_type = partition;
    h.app.wall_settings.reference = ReferenceLine::Center;
    // Near the interior faces of the south and north walls (26.5 cm in).
    h.click_at(p(6.0, 0.29));
    h.click_at(p(6.0, 8.71));
    h.key(Key::Escape);
    assert!(!h.app.wall_tool.is_drawing());
    h.key(Key::Escape);
    assert_eq!(h.app.tool, Tool::Arrow);

    let m = h.model();
    let w = m.walls().last().unwrap().clone();
    assert_eq!(w.wall_type, partition);
    assert!((w.start.y - 0.265).abs() < 1e-9, "{:?}", w.start);
    assert!((w.end.y - 8.735).abs() < 1e-9, "{:?}", w.end);

    // Click on it, delete it, undo.
    h.click_at(p(6.02, 4.5));
    assert_eq!(
        h.app.ed.selection.iter().copied().collect::<Vec<_>>(),
        vec![Element::Wall(w.id)]
    );
    h.key(Key::Delete);
    assert!(h.model().wall(w.id).is_none());
    h.set_modifiers(Modifiers::COMMAND);
    h.key(Key::Z);
    h.set_modifiers(Modifiers::NONE);
    assert!(h.model().wall(w.id).is_some());

    // A selection window around the whole house takes the five walls.
    h.drag(p(-1.0, -1.0), p(13.0, 10.0));
    assert_eq!(h.app.ed.selection.len(), 5);
    // Changing the height of all of them is one step.
    let walls: Vec<_> = h.model().walls().map(|w| w.id).collect();
    h.app.ed.set_walls_height(&walls, 2.8).unwrap();
    assert!(h.model().walls().all(|w| w.height == 2.8));
    h.frame();
}

#[test]
fn view_navigation_and_unsaved_changes() {
    let mut h = Harness::new(Editor::new_project());
    let zoom = h.app.view.zoom;
    h.move_to(p(3.0, 3.0));
    h.events.push(Event::MouseWheel {
        unit: egui::MouseWheelUnit::Point,
        delta: Vec2::new(0.0, 120.0),
        phase: egui::TouchPhase::Move,
        source: egui::MouseWheelSource::Wheel,
        modifiers: Modifiers::NONE,
    });
    for _ in 0..10 {
        h.frame();
    }
    assert!(h.app.view.zoom > zoom, "the wheel zooms in");
    // The point under the cursor stays put.
    let under = h.app.view.to_model(h.app.canvas, h.pos);
    assert!(under.distance(p(3.0, 3.0)) < 0.05, "{under:?}");

    // Ctrl+N on a modified project asks before discarding.
    draw_outline(&mut h);
    h.set_modifiers(Modifiers::COMMAND);
    h.key(Key::N);
    h.set_modifiers(Modifiers::NONE);
    assert_eq!(h.app.pending, Some(Pending::New));
    assert_eq!(h.model().walls().count(), 4);
    h.app.pending = None;
    h.app.run_pending(Pending::New);
    assert_eq!(h.model().walls().count(), 0);
    assert!(!h.app.ed.is_modified());
}

/// Draws the outline, saves it, opens it again and exports the IFC. With
/// `ARCHISIM_TEST_OUT` set, leaves `sesion.archisim` and `sesion.ifc` there
/// for IfcOpenShell to validate.
#[test]
fn drawn_project_saves_and_exports() {
    let mut h = Harness::new(Editor::new_project());
    draw_outline(&mut h);
    furnish(&mut h);
    h.app.ed.rename_project("Sesión de prueba").unwrap();
    let dir = match std::env::var_os("ARCHISIM_TEST_OUT") {
        Some(d) => PathBuf::from(d),
        None => std::env::temp_dir().join(format!("archisim-app-{}", std::process::id())),
    };
    std::fs::create_dir_all(&dir).unwrap();
    let path = h.app.ed.save(Some(&dir.join("sesion.archisim"))).unwrap();
    let reopened = Editor::open(&path).unwrap();
    assert_eq!(reopened.model(), h.model());
    reopened
        .export_ifc(&dir.join("sesion.ifc"), archisim_ifc::Schema::Ifc4x3)
        .unwrap();
    let ifc = std::fs::read_to_string(dir.join("sesion.ifc")).unwrap();
    assert_eq!(ifc.matches("IFCWALL(").count(), 4);
    assert_eq!(ifc.matches("IFCSLAB(").count(), 1);
    assert_eq!(ifc.matches("IFCDOOR(").count(), 1);
    assert_eq!(ifc.matches("IFCWINDOW(").count(), 1);
    assert!(ifc.contains("Fachada doble hoja 26,5"));
    // The plan drawn on screen is up to date and not empty.
    h.frame();
    assert!(h.app.plan.is_some());

    // Importing that IFC gives back the same building, as a new project
    // that still has to be saved.
    let mut h2 = Harness::new(Editor::new_project());
    h2.app.import_path(&dir.join("sesion.ifc"));
    h2.frame();
    let diffs = archisim_ifc::differences(reopened.model(), h2.model());
    assert!(diffs.is_empty(), "{}", diffs.join("\n"));
    assert!(h2.app.ed.is_modified());
    assert_eq!(h2.app.ed.display_name(), "sesion.ifc (importado)");
    assert!(h2.app.import_report.is_some());
    assert!(!h2.app.message.as_ref().unwrap().error);
}

/// An IFC written by another program, imported from the File menu path:
/// the report window opens, and the tools get the ArchiSim types when the
/// file has none of some kind.
#[test]
fn imports_an_ifc_file_from_another_program() {
    let fixture = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../archisim-ifc/tests/fixtures/ifcopenshell_ifc2x3_mm.ifc");
    let mut h = Harness::new(Editor::new_project());
    h.app.import_path(&fixture);
    h.frame();
    assert_eq!(h.model().walls().count(), 5);
    assert_eq!(h.model().project_name, "Casa de prueba");
    let report = h.app.import_report.clone().unwrap();
    assert!(report.contains("pilares"), "{report}");
    // The file brings door, window, wall and slab types: no template.
    assert!(!report.contains("plantilla"), "{report}");
    assert!(h
        .model()
        .wall_types()
        .all(|t| !t.name.starts_with("Fachada doble")));
    // Closing the report.
    h.app.import_report = None;
    h.frame();
    // A file that is not IFC is refused with a message.
    let bad = std::env::temp_dir().join(format!("archisim-no-ifc-{}.ifc", std::process::id()));
    std::fs::write(&bad, "esto no es un IFC").unwrap();
    h.app.import_path(&bad);
    assert!(h.app.message.as_ref().unwrap().error);
    assert_eq!(h.model().walls().count(), 5, "the project stays as it was");
    let _ = std::fs::remove_file(bad);
}

/// Runs `length_field` alone through a few frames of keyboard input and
/// returns what it reported in each.
fn drive_length_field(steps: Vec<Vec<Event>>) -> Vec<Option<f64>> {
    let ctx = egui::Context::default();
    let id = Id::unique("test-length");
    let mut out = Vec::new();
    for (i, events) in std::iter::once(Vec::new()).chain(steps).enumerate() {
        let input = RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(400.0, 200.0))),
            time: Some(i as f64 * 0.1),
            events,
            ..Default::default()
        };
        let mut got = None;
        let mut o = ctx.run_ui(input, |ui| {
            if i == 0 {
                ui.memory_mut(|m| m.request_focus(id));
            }
            got = length_field(ui, id, 3.0, 80.0);
        });
        o.textures_delta.clear();
        out.push(got);
    }
    out
}

fn key_event(key: Key) -> Vec<Event> {
    [true, false]
        .into_iter()
        .map(|pressed| Event::Key {
            key,
            physical_key: None,
            pressed,
            repeat: false,
            modifiers: Modifiers::NONE,
        })
        .collect()
}

#[test]
fn length_fields_commit_on_enter_and_cancel_on_escape() {
    let erase: Vec<Event> = (0..4).flat_map(|_| key_event(Key::Backspace)).collect();
    let typed = |t: &str| vec![Event::Text(t.into())];
    // Nothing is reported while typing; Enter reports the value once.
    let got = drive_length_field(vec![
        Vec::new(),
        erase.clone(),
        typed("2,7m"),
        key_event(Key::Enter),
        Vec::new(),
    ]);
    assert_eq!(got, vec![None, None, None, None, Some(2.7), None]);
    // Escape throws the typing away.
    let got = drive_length_field(vec![
        Vec::new(),
        erase,
        typed("2500"),
        key_event(Key::Escape),
        Vec::new(),
    ]);
    assert!(got.iter().all(Option::is_none), "{got:?}");
}

#[test]
fn the_3d_view_shows_the_model_and_selects_by_clicking() {
    let mut h = Harness::new(Editor::new_project());
    draw_outline(&mut h);
    // With the arrow (a tool would draw in the 3D view).
    h.key(Key::Escape);
    h.key(Key::Escape);
    h.key(Key::F3);
    assert_eq!(h.app.mode, ViewMode::ThreeD);
    h.frame();
    h.frame();
    let v = &h.app.v3d;
    assert!(!v.fit_requested, "framed on first sight");
    // Click on the top of the north wall, seen from the default view.
    let size = v.canvas.size();
    let vp = v.camera.view_proj(size.x / size.y);
    let target = glam::Vec3::new(6.0, 9.0 - 0.13, 3.0);
    let ndc = vp.project_point3(target);
    let pos = v.canvas.min + Vec2::new((ndc.x + 1.0) / 2.0 * size.x, (1.0 - ndc.y) / 2.0 * size.y);
    let north = h.model().walls().nth(1).unwrap().id;
    h.pos = pos;
    h.events.push(Event::PointerMoved(pos));
    h.frame();
    h.button(PointerButton::Primary, true);
    h.button(PointerButton::Primary, false);
    assert_eq!(
        h.app.ed.selection.iter().copied().collect::<Vec<_>>(),
        vec![Element::Wall(north)]
    );
    // Dragging orbits the camera; the selection stays.
    let yaw = h.app.v3d.camera.yaw;
    h.button(PointerButton::Primary, true);
    for i in 1..=5 {
        h.pos = pos + Vec2::new(20.0 * i as f32, 0.0);
        h.events.push(Event::PointerMoved(h.pos));
        h.frame();
    }
    h.button(PointerButton::Primary, false);
    assert!(h.app.v3d.camera.yaw < yaw, "turned");
    assert_eq!(h.app.ed.selection.len(), 1);
    // Supr deletes from the 3D view too; a tool key takes the tool there
    // (it draws in the 3D view, R14).
    h.key(Key::Delete);
    assert!(h.model().wall(north).is_none());
    h.type_text("m");
    assert_eq!(h.app.mode, ViewMode::ThreeD);
    assert_eq!(h.app.tool, Tool::Wall);
}

/// Presses the left button at `from`, moves to `to` in a few steps (so it
/// counts as a drag) and leaves the button down.
fn press_and_move(h: &mut Harness, from: Point2, to: Point2) {
    h.move_to(from);
    h.button(PointerButton::Primary, true);
    for i in 1..=4 {
        let t = i as f64 / 4.0;
        h.move_to(from + (to - from) * t);
    }
}

#[test]
fn moves_copies_and_stretches_with_the_mouse() {
    let mut h = Harness::new(Editor::new_project());
    draw_outline(&mut h);
    furnish(&mut h);
    h.key(Key::Escape);
    assert_eq!(h.app.tool, Tool::Arrow);
    let walls: Vec<_> = h.model().walls().map(|w| w.id).collect();
    let (west, north, east) = (walls[0], walls[1], walls[2]);
    let close = |a: Point2, b: Point2| a.distance(b) < 1e-6;

    // Select the north façade and drag it 1000 mm north, typed, with Shift
    // for a straight move.
    h.click_at(p(6.0, 8.87));
    assert_eq!(
        h.app.ed.selection.iter().copied().collect::<Vec<_>>(),
        vec![Element::Wall(north)]
    );
    h.set_modifiers(Modifiers::SHIFT);
    press_and_move(&mut h, p(6.0, 8.87), p(6.05, 9.6));
    assert!(h.app.drag.is_some(), "dragging the selected wall");
    h.type_text("1000");
    h.key(Key::Enter);
    h.button(PointerButton::Primary, false);
    h.set_modifiers(Modifiers::NONE);
    let m = h.model();
    assert!(
        close(m.wall(north).unwrap().start, p(0.0, 10.0)),
        "{:?} {:?}",
        m.wall(north).unwrap(),
        h.app.message
    );
    // The west and east façades followed: the corners are still closed.
    assert!(close(m.wall(west).unwrap().end, p(0.0, 10.0)));
    assert!(close(m.wall(east).unwrap().start, p(12.0, 10.0)));
    // The window went with its wall.
    assert_eq!(h.app.ed.undo_label(), Some("Mover"));
    assert_eq!(h.app.ed.selection.len(), 1);

    // Ctrl + drag copies it (with its window) 3000 mm further north.
    let (walls_before, openings_before) = (m.walls().count(), m.openings().count());
    h.set_modifiers(Modifiers::COMMAND | Modifiers::SHIFT);
    press_and_move(&mut h, p(6.0, 9.87), p(6.0, 11.0));
    h.type_text("3000");
    h.key(Key::Enter);
    h.button(PointerButton::Primary, false);
    h.set_modifiers(Modifiers::NONE);
    let m = h.model();
    assert_eq!(m.walls().count(), walls_before + 1, "{:?}", h.app.message);
    assert_eq!(m.openings().count(), openings_before + 1);
    assert_eq!(h.app.ed.undo_label(), Some("Copiar"));
    // The copy is selected and lies 3 m north of the original.
    let copy = match h
        .app
        .ed
        .selection
        .iter()
        .find(|e| matches!(e, Element::Wall(_)))
    {
        Some(Element::Wall(id)) => *id,
        _ => panic!("the copy is selected"),
    };
    assert!(close(m.wall(copy).unwrap().start, p(0.0, 13.0)));
    h.set_modifiers(Modifiers::COMMAND);
    h.key(Key::Z);
    h.set_modifiers(Modifiers::NONE);

    // Stretch: select the east façade and pull its north end (the corner
    // at (12, 10)) 2000 mm east. The north wall stretches with it.
    h.key(Key::Escape);
    h.click_at(p(11.87, 5.0));
    assert_eq!(
        h.app.ed.selection.iter().copied().collect::<Vec<_>>(),
        vec![Element::Wall(east)]
    );
    h.set_modifiers(Modifiers::SHIFT);
    press_and_move(&mut h, p(12.0, 10.0), p(13.0, 10.02));
    assert!(
        matches!(
            h.app.drag.as_ref().map(|d| d.grip),
            Some(Grip::WallEnd { .. })
        ),
        "grabbed by the end"
    );
    h.type_text("2000");
    h.key(Key::Enter);
    h.button(PointerButton::Primary, false);
    h.set_modifiers(Modifiers::NONE);
    let m = h.model();
    assert!(
        close(m.wall(east).unwrap().start, p(14.0, 10.0)),
        "{:?}",
        h.app.message
    );
    assert!(close(m.wall(north).unwrap().end, p(14.0, 10.0)));
    assert_eq!(h.app.ed.undo_label(), Some("Estirar muro"));

    // Escape cancels a drag: nothing changes.
    let before = h.model().clone();
    press_and_move(&mut h, p(14.0, 10.0), p(15.0, 11.0));
    assert!(h.app.drag.is_some());
    h.key(Key::Escape);
    h.button(PointerButton::Primary, false);
    assert_eq!(h.model(), &before);

    // Slab: select it by its edge and move the south edge 500 mm south by
    // dragging the diamond in its middle.
    h.key(Key::Escape);
    let slab = h.model().slabs().next().unwrap().id;
    h.app.ed.selection = [Element::Slab(slab)].into_iter().collect();
    h.frame();
    h.set_modifiers(Modifiers::SHIFT);
    press_and_move(&mut h, p(6.0, 0.0), p(6.0, -0.3));
    assert!(matches!(
        h.app.drag.as_ref().map(|d| d.grip),
        Some(Grip::SlabEdge { .. })
    ));
    h.type_text("500");
    h.key(Key::Enter);
    h.button(PointerButton::Primary, false);
    h.set_modifiers(Modifiers::NONE);
    let s = h.model().slab(slab).unwrap();
    assert!(
        s.outline
            .iter()
            .filter(|q| (q.y + 0.5).abs() < 1e-6)
            .count()
            == 2,
        "{:?} {:?}",
        s.outline,
        h.app.message
    );
    assert_eq!(h.app.ed.undo_label(), Some("Desplazar arista de forjado"));

    // A drag outside the selection is still a selection window.
    h.app.ed.selection.clear();
    h.drag(p(-1.0, -1.0), p(1.0, 11.0));
    assert!(h.app.drag.is_none());
    assert_eq!(h.app.ed.selection.len(), 1, "the west wall");

    // The edited house still exports; with `ARCHISIM_TEST_OUT` set it is
    // left there as `edicion.ifc` for IfcOpenShell to validate.
    let dir = match std::env::var_os("ARCHISIM_TEST_OUT") {
        Some(d) => PathBuf::from(d),
        None => std::env::temp_dir().join(format!("archisim-drag-{}", std::process::id())),
    };
    std::fs::create_dir_all(&dir).unwrap();
    h.app
        .ed
        .export_ifc(&dir.join("edicion.ifc"), archisim_ifc::Schema::Ifc4x3)
        .unwrap();
    let ifc = std::fs::read_to_string(dir.join("edicion.ifc")).unwrap();
    assert_eq!(ifc.matches("IFCWALL(").count(), 4);
    assert_eq!(ifc.matches("IFCWINDOW(").count(), 1);
}

#[test]
fn magic_wand_slabs() {
    let mut h = Harness::new(Editor::new_project());
    draw_outline(&mut h);
    h.key(Key::Escape);
    h.type_text("f");
    assert_eq!(h.app.tool, Tool::Slab);
    // The whole building at its exterior faces.
    h.app.slab_settings.wand = true;
    h.app.slab_settings.contour = Contour::ExteriorFace;
    h.click_at(p(6.0, 4.5));
    let m = h.model();
    assert_eq!(m.slabs().count(), 1, "{:?}", h.app.message);
    assert!((m.slabs().last().unwrap().area() - 108.0).abs() < 1e-9);
    assert!(h.app.message.as_ref().unwrap().text.contains("varita"));
    // The room inside the 26,5 cm façades.
    h.app.slab_settings.contour = Contour::InteriorFace;
    h.click_at(p(6.0, 4.5));
    let inner = (12.0 - 0.53) * (9.0 - 0.53);
    assert!((h.model().slabs().last().unwrap().area() - inner).abs() < 1e-6);
    // Clicking on a wall explains why not.
    let count = h.model().slabs().count();
    h.click_at(p(0.1, 4.5));
    assert_eq!(h.model().slabs().count(), count);
    assert!(h.app.message.as_ref().unwrap().error);
    // In polygon mode, Space + click uses the wand too.
    h.app.slab_settings.wand = false;
    h.app.slab_settings.contour = Contour::WallAxis;
    h.events.push(Event::Key {
        key: Key::Space,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers: Modifiers::NONE,
    });
    h.frame();
    h.click_at(p(6.0, 4.5));
    h.key(Key::Space);
    let axis = (12.0 - 0.265) * (9.0 - 0.265);
    assert_eq!(h.model().slabs().count(), count + 1, "{:?}", h.app.message);
    assert!((h.model().slabs().last().unwrap().area() - axis).abs() < 1e-6);
    assert!(!h.app.slab_tool.is_drawing());
}

#[test]
fn dimensions_and_level_marks_with_the_mouse() {
    let mut h = Harness::new(Editor::new_project());
    draw_outline(&mut h);
    furnish(&mut h);
    h.key(Key::Escape);
    h.type_text("c");
    assert_eq!(h.app.tool, Tool::Dimension);

    // Linear chain along the south façade: three corners and points
    // snapped, Enter, then the line 1 m below.
    h.click_at(p(0.01, 0.02));
    h.click_at(p(6.0, -0.02));
    h.click_at(p(11.98, 0.01));
    h.key(Key::Enter);
    assert!(h.app.dim_tool.is_placing());
    h.click_at(p(6.0, -1.0));
    let m = h.model();
    assert_eq!(m.dimensions().count(), 1, "{:?}", h.app.message);
    let d = m.dimensions().next().unwrap().clone();
    assert_eq!(d.direction, p(1.0, 0.0));
    let total: f64 = d.values().iter().sum();
    assert!((total - 12.0).abs() < 1e-9, "{:?}", d.values());
    assert_eq!(h.app.ed.undo_label(), Some("Acotar"));

    // The openings of the north façade, from outside.
    h.app.dim_settings.mode = DimMode::WallOpenings;
    h.click_at(p(6.0, 9.05));
    assert!(h.app.dim_tool.is_placing(), "{:?}", h.app.message);
    h.click_at(p(6.0, 10.0));
    let chain = h.model().dimensions().last().unwrap().clone();
    // Both ends of the façade and the jambs of the window.
    assert_eq!(chain.points.len(), 4, "{:?}", chain.points);

    // A level mark on the slab.
    h.app.dim_settings.mode = DimMode::LevelMark;
    h.click_at(p(4.0, 4.0));
    assert_eq!(h.model().elevation_marks().count(), 1);
    assert!(h.app.message.as_ref().unwrap().text.contains("±0,00"));

    // The plan draws them.
    let svg = archisim_plan::render_plan(h.model(), h.app.ed.level, &Default::default());
    assert_eq!(svg.matches("class=\"dimension\"").count(), 2);
    assert_eq!(svg.matches("class=\"level-mark\"").count(), 1);

    // Select the first dimension by clicking on its line and move the line.
    h.key(Key::Escape);
    h.key(Key::Escape);
    assert_eq!(h.app.tool, Tool::Arrow);
    h.click_at(p(3.0, -0.97));
    let id = d.id;
    assert_eq!(
        h.app.ed.selection.iter().copied().collect::<Vec<_>>(),
        vec![Element::Dimension(id)]
    );
    h.set_modifiers(Modifiers::SHIFT);
    press_and_move(&mut h, p(3.0, -0.97), p(3.0, -1.6));
    h.type_text("500");
    h.key(Key::Enter);
    h.button(PointerButton::Primary, false);
    h.set_modifiers(Modifiers::NONE);
    let moved = h.model().dimension(id).unwrap();
    assert!((moved.line_offset() - d.line_offset() + 0.5).abs() < 1e-6);
    assert_eq!(moved.points, d.points);
    // Delete it.
    h.key(Key::Delete);
    assert!(h.model().dimension(id).is_none());
}

#[test]
fn levels_are_managed_from_the_dialog() {
    let mut h = Harness::new(Editor::new_project());
    draw_outline(&mut h);
    let ground = h.app.ed.level;
    let before = h.model().levels().len();
    // Add a floor above: it becomes the active level.
    let first = h.app.ed.add_level("Planta primera", 3.2).unwrap();
    assert_eq!(h.app.ed.level, first);
    assert_eq!(h.model().levels().len(), before + 1);
    h.app.show_levels = true;
    h.frame();
    h.app.ed.rename_level(first, "Planta 1ª").unwrap();
    h.app.ed.set_level_elevation(first, 3.3).unwrap();
    assert_eq!(h.model().level(first).unwrap().elevation, 3.3);
    // Deleting the ground floor with walls asks first.
    h.app.level_to_delete = Some(ground);
    h.frame();
    assert!(h.app.level_to_delete.is_some(), "waits for an answer");
    h.app.level_to_delete = None;
    h.app.ed.remove_level(ground).unwrap();
    assert_eq!(h.model().walls().count(), 0);
    assert_eq!(h.app.ed.undo_label(), Some("Borrar planta"));
    h.set_modifiers(Modifiers::COMMAND);
    h.key(Key::Z);
    h.set_modifiers(Modifiers::NONE);
    assert_eq!(h.model().walls().count(), 4);
    // The new floor still exports as a storey.
    let dir = std::env::temp_dir().join(format!("archisim-levels-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    h.app
        .ed
        .export_ifc(&dir.join("plantas.ifc"), archisim_ifc::Schema::Ifc4x3)
        .unwrap();
    let ifc = std::fs::read_to_string(dir.join("plantas.ifc")).unwrap();
    assert!(ifc.contains("Planta 1"), "storey name");
    let _ = std::fs::remove_dir_all(dir);
}

/// The types and materials dialog on the drawn house: every tab is drawn
/// with its preview, types are duplicated and edited, the walls follow,
/// what does not fit is refused, and the result saves and exports. Its IFC
/// goes to `ARCHISIM_TEST_OUT` like the session of `drawn_project_saves_and_exports`.
#[test]
fn drawn_project_with_edited_types() {
    use super::types_ui::Action;
    use archisim_core::{CutPattern, LayerFunction};

    let mut h = Harness::new(Editor::new_project());
    draw_outline(&mut h);
    furnish(&mut h);
    h.key(Key::Escape);
    let wall = h.model().walls().next().unwrap().clone();
    let facade = h.model().wall_type(wall.wall_type).unwrap().clone();

    // «Editar tipo…» from the wall's properties opens the dialog on it.
    h.app.edit_type(TypesTab::Walls, facade.id.0);
    assert!(h.app.types.open);
    for tab in [
        TypesTab::Walls,
        TypesTab::Slabs,
        TypesTab::Doors,
        TypesTab::Windows,
        TypesTab::Materials,
    ] {
        h.app.types.tab = tab;
        // Every preview the list offers is drawn: plan, section, elevation
        // and 3D (without a graphics card, a notice in its place).
        for (view, _, _) in super::types_ui::views(tab) {
            h.app.types.view = *view;
            h.frame();
            h.frame();
            let shown = h.app.types.shown.clone();
            let prefix = match view {
                super::types_ui::PreviewView::Plan => "plan:",
                super::types_ui::PreviewView::Section => "section:",
                super::types_ui::PreviewView::Elevation => "elevation:",
                super::types_ui::PreviewView::ThreeD => "3d:",
            };
            assert!(shown.starts_with(prefix), "{tab:?} {view:?}: {shown}");
            let opening = matches!(tab, TypesTab::Doors | TypesTab::Windows);
            assert_eq!(shown.contains("ot:"), opening, "{tab:?} {view:?}");
        }
    }

    // Duplicate the façade, thicken its insulation to 8 cm and use it.
    h.app.apply_type_action(Action::DuplicateWall(facade.id));
    let copy = h.app.types.wall.unwrap();
    assert_ne!(copy, facade.id);
    let mut layers = facade.layers.clone();
    let insulation = layers
        .iter()
        .position(|l| l.function == LayerFunction::Insulation)
        .unwrap();
    layers[insulation].thickness = 0.08;
    h.app.apply_type_action(Action::EditWall(
        copy,
        "Fachada doble hoja 29,5".into(),
        facade.function,
        layers,
    ));
    let walls: Vec<_> = h.model().walls().map(|w| w.id).collect();
    h.app.ed.set_walls_type(&walls, copy).unwrap();
    assert!((h.model().wall_type(copy).unwrap().thickness() - 0.295).abs() < 1e-9);
    h.app.types.tab = TypesTab::Walls;
    h.frame();

    // A layer in a new material, hatched like concrete.
    let brick = facade.layers[1].material;
    h.app.apply_type_action(Action::DuplicateMaterial(brick));
    let new_mat = h.app.types.material.unwrap();
    h.app.apply_type_action(Action::EditMaterial(
        new_mat,
        "Bloque de hormigón".into(),
        "block".into(),
        [190, 190, 185],
        CutPattern::Cross,
    ));
    let mut layers = h.model().wall_type(copy).unwrap().layers.clone();
    layers[1].material = new_mat;
    h.app.apply_type_action(Action::EditWall(
        copy,
        "Fachada doble hoja 29,5".into(),
        facade.function,
        layers,
    ));
    assert_eq!(
        h.model().material_users(new_mat),
        vec!["Fachada doble hoja 29,5".to_string()]
    );
    // In use: refused, and the dialog says why.
    h.app.apply_type_action(Action::RemoveMaterial(new_mat));
    assert!(h.app.message.as_ref().unwrap().error);
    assert!(h.model().material(new_mat).is_some());

    // The placed door's type: 10 cm wider fits; a 40 cm deep frame does not.
    let door = h
        .model()
        .openings()
        .find(|o| h.model().opening_type(o.opening_type).unwrap().is_door())
        .unwrap()
        .clone();
    let dt = h.model().opening_type(door.opening_type).unwrap().clone();
    h.app.apply_type_action(Action::EditOpeningValue(
        dt.id,
        "width".into(),
        archisim_core::Value::Number(dt.width + 0.1),
    ));
    assert!(
        !h.app.message.as_ref().unwrap().error,
        "{:?}",
        h.app.message
    );
    assert!((h.model().opening_type(dt.id).unwrap().width - dt.width - 0.1).abs() < 1e-9);
    assert_eq!(h.model().opening(door.id).unwrap().along, door.along);
    h.app.apply_type_action(Action::EditOpeningValue(
        dt.id,
        "frame_depth".into(),
        archisim_core::Value::Number(0.4),
    ));
    assert!(h.app.message.as_ref().unwrap().text.contains("marco"));
    assert_eq!(
        h.model().opening_type(dt.id).unwrap().frame_depth,
        dt.frame_depth
    );

    // The wall tool uses the copy; undoing back past the duplicate makes it
    // fall back to an existing type.
    h.app.wall_settings.wall_type = copy;
    let saved_model = h.model().clone();
    let mut redo = 0;
    while h.model().wall_type(copy).is_some() {
        h.app.ed.undo();
        redo += 1;
    }
    h.frame();
    assert!(h.model().wall_type(h.app.wall_settings.wall_type).is_some());
    for _ in 0..redo {
        h.app.ed.redo();
    }
    assert_eq!(h.model(), &saved_model);

    // Save, reopen, export: the IFC carries the new type and material, and
    // imports back the same.
    let dir = match std::env::var_os("ARCHISIM_TEST_OUT") {
        Some(d) => PathBuf::from(d),
        None => std::env::temp_dir().join(format!("archisim-types-{}", std::process::id())),
    };
    std::fs::create_dir_all(&dir).unwrap();
    let path = h.app.ed.save(Some(&dir.join("tipos.archisim"))).unwrap();
    let reopened = Editor::open(&path).unwrap();
    assert_eq!(reopened.model(), h.model());
    reopened
        .export_ifc(&dir.join("tipos.ifc"), archisim_ifc::Schema::Ifc4x3)
        .unwrap();
    let ifc = std::fs::read_to_string(dir.join("tipos.ifc")).unwrap();
    assert!(ifc.contains("Fachada doble hoja 29,5"));
    assert!(ifc.contains("Bloque de hormig"));
    assert!(ifc.contains("0.295"), "wall width in the IFC");
    let mut h2 = Harness::new(Editor::new_project());
    h2.app.import_path(&dir.join("tipos.ifc"));
    let diffs = archisim_ifc::differences(reopened.model(), h2.model());
    assert!(diffs.is_empty(), "{}", diffs.join("\n"));
}

/// A file dropped on the window, as the windowing system hands it over.
#[derive(Debug)]
struct TestDrop(PathBuf);

impl egui::DroppedFile for TestDrop {
    fn path(&self) -> &Path {
        &self.0
    }
    fn bytes(&self) -> Result<Vec<u8>, String> {
        std::fs::read(&self.0).map_err(|e| e.to_string())
    }
}

/// Files dropped on the window (and, on macOS, opened from the Finder) are
/// opened like from the File menu: a project replaces the open one, an IFC
/// is imported, anything else is refused, and unsaved changes ask first.
#[test]
fn opens_files_dropped_on_the_window() {
    let dir = std::env::temp_dir().join(format!("archisim-drop-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let mut demo = Editor::new_project();
    demo.rename_project("Soltado").unwrap();
    let project = demo.save(Some(&dir.join("soltado.archisim"))).unwrap();
    let fixture = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../archisim-ifc/tests/fixtures/ifcopenshell_ifc2x3_mm.ifc");

    let mut h = Harness::new(Editor::new_project());
    let drop = |h: &mut Harness, path: PathBuf| {
        let input = RawInput {
            screen_rect: Some(Rect::from_min_size(
                Pos2::ZERO,
                Vec2::new(SCREEN.0, SCREEN.1),
            )),
            time: Some(h.time),
            dropped_files: vec![std::sync::Arc::new(TestDrop(path))],
            ..Default::default()
        };
        let app = &mut h.app;
        let mut output = h.ctx.run_ui(input, |ui| app.show(ui));
        output.textures_delta.clear();
        h.time += 0.05;
    };
    drop(&mut h, project.clone());
    assert_eq!(h.model().project_name, "Soltado");
    drop(&mut h, fixture.clone());
    assert!(h.app.import_report.is_some());
    assert!(h.model().walls().count() > 0);
    // A PDF is not opened.
    drop(&mut h, dir.join("plano.pdf"));
    assert!(h.app.message.as_ref().unwrap().error);
    // With changes, it asks before replacing the project.
    h.app.ed.rename_project("Con cambios").unwrap();
    drop(&mut h, project);
    assert!(h.app.pending.is_some());
    assert_eq!(h.model().project_name, "Con cambios");
    let _ = std::fs::remove_dir_all(dir);
}

/// Views open in tabs, each with its own level, framing and scale; F2 and
/// F3 go to the plan and the 3D view; a level added opens in a tab of its
/// own and closes with it.
#[test]
fn view_tabs_keep_their_level_framing_and_scale() {
    let mut h = Harness::new(Editor::new_project());
    draw_outline(&mut h);
    let ground = h.app.ed.level;
    assert_eq!(h.app.tabs.len(), 1);
    // The plan is drawn at the scale of its tab.
    h.frame();
    assert_eq!(h.app.plan.as_ref().unwrap().0.scale, 50.0);
    h.app.set_scale(100.0);
    h.frame();
    assert_eq!(h.app.plan.as_ref().unwrap().0.scale, 100.0);
    assert_eq!(h.app.tabs[0].kind, TabKind::Plan(ground));

    // A level added becomes current, in a new tab beside.
    let first = h.app.ed.add_level("Planta primera", 3.0).unwrap();
    h.frame();
    assert_eq!(h.app.tabs.len(), 2);
    assert_eq!(h.app.tabs[h.app.active_tab].kind, TabKind::Plan(first));
    assert_eq!(h.app.ed.level, first);
    assert_eq!(h.app.plan_scale, 50.0, "a new tab starts at 1:50");
    h.app.view.center = Point2::new(40.0, 40.0);

    // Back to the ground floor: its own framing and scale.
    h.app.activate_tab(0);
    h.frame();
    assert_eq!(h.app.ed.level, ground);
    assert_eq!(h.app.plan_scale, 100.0);
    assert_ne!(h.app.view.center, Point2::new(40.0, 40.0));
    h.app.activate_tab(1);
    assert_eq!(h.app.view.center, Point2::new(40.0, 40.0));
    assert_eq!(h.app.ed.level, first);

    // F3 opens the 3D view in a tab; F2 goes back to the last plan.
    h.key(Key::F3);
    assert_eq!(h.app.mode, ViewMode::ThreeD);
    assert_eq!(h.app.tabs.len(), 3);
    h.key(Key::F2);
    assert_eq!(h.app.mode, ViewMode::Plan);
    assert_eq!(h.app.ed.level, first);
    // A tool key from the 3D view draws there (R14).
    h.key(Key::F3);
    h.type_text("m");
    assert_eq!((h.app.mode, h.app.tool), (ViewMode::ThreeD, Tool::Wall));
    h.key(Key::Escape);
    h.key(Key::F2);
    // Ctrl+Tab goes round the tabs.
    let before = h.app.active_tab;
    h.set_modifiers(Modifiers::CTRL);
    h.key(Key::Tab);
    h.set_modifiers(Modifiers::NONE);
    assert_eq!(h.app.active_tab, (before + 1) % 3);

    // Closing the 3D tab; the last tab cannot be closed.
    let three_d = h
        .app
        .tabs
        .iter()
        .position(|t| t.kind == TabKind::ThreeD)
        .unwrap();
    h.app.close_tab(three_d);
    h.frame();
    assert_eq!(h.app.tabs.len(), 2);
    assert_eq!(h.app.mode, ViewMode::Plan);

    // Deleting the first floor closes its tab and shows the ground floor.
    h.app.activate_tab(
        h.app
            .tabs
            .iter()
            .position(|t| t.kind == TabKind::Plan(first))
            .unwrap(),
    );
    h.app.ed.remove_level(first).unwrap();
    h.frame();
    assert_eq!(h.app.tabs.len(), 1);
    assert_eq!(h.app.ed.level, ground);
    assert_eq!(h.app.plan_scale, 100.0);
    h.app.close_tab(0);
    assert_eq!(h.app.tabs.len(), 1, "the last tab stays");
    // Drawing still works where the canvas is now.
    assert_eq!(h.model().walls().count(), 4);
}

/// Keys and tiles of the toolbox: N is the level mark, C the dimension;
/// tools still to come say when they arrive and change nothing.
#[test]
fn toolbox_keys_and_tools_to_come() {
    let mut h = Harness::new(Editor::new_project());
    h.type_text("n");
    assert_eq!(h.app.tool, Tool::Dimension);
    assert_eq!(h.app.dim_settings.mode, DimMode::LevelMark);
    h.key(Key::Escape);
    h.type_text("c");
    assert_eq!(h.app.tool, Tool::Dimension);
    assert_eq!(h.app.dim_settings.mode, DimMode::Linear);
    h.key(Key::Escape);
    assert_eq!(h.app.tool, Tool::Arrow);
    h.app.pick(super::toolbox::Pick::Later("el punto 2.5"));
    assert_eq!(h.app.tool, Tool::Arrow);
    assert!(h.app.message.as_ref().unwrap().text.contains("2.5"));
    // The hidden properties and navigator still leave a working plan.
    h.app.show_properties = false;
    h.app.show_navigator = false;
    h.frame();
    h.frame();
    draw_outline(&mut h);
    assert_eq!(h.model().walls().count(), 4);
}

/// The disciplines: a tool key brings the discipline of its tool (M is
/// always the wall, I the column), the rail changes the tiles, and the
/// command search finds a tool by its name and takes it with Intro.
#[test]
fn disciplines_keys_and_command_search() {
    use super::toolbox::Discipline;
    let mut h = Harness::new(Editor::new_project());
    assert_eq!(h.app.discipline, Discipline::Architecture);
    h.type_text("i");
    assert_eq!(h.app.tool, Tool::Column);
    assert_eq!(h.app.discipline, Discipline::Structure);
    // The slab is in Structure too: it stays there.
    h.type_text("f");
    assert_eq!(h.app.tool, Tool::Slab);
    assert_eq!(h.app.discipline, Discipline::Structure);
    h.type_text("n");
    assert_eq!(h.app.discipline, Discipline::Documentation);
    h.key(Key::Escape);
    h.key(Key::Escape);
    // The search: Ctrl+Espacio, «viga», Intro.
    h.set_modifiers(Modifiers::CTRL);
    h.key(Key::Space);
    h.set_modifiers(Modifiers::NONE);
    assert!(h.app.command_search.is_some());
    h.frame();
    h.type_text("viga");
    assert_eq!(h.app.command_search.as_ref().unwrap().0, "viga");
    h.key(Key::Enter);
    assert!(h.app.command_search.is_none());
    assert_eq!(h.app.tool, Tool::Beam);
    assert_eq!(h.app.discipline, Discipline::Structure);
    // Orders that are not tiles are found too, and Esc closes the search
    // without taking anything.
    h.key(Key::Escape);
    h.app.open_command_search();
    h.frame();
    h.type_text("zapata");
    h.key(Key::Escape);
    assert!(h.app.command_search.is_none());
    assert_eq!(h.app.tool, Tool::Arrow);
    // Every discipline draws its tiles; Properties keep working below.
    for d in Discipline::ALL {
        h.app.discipline = d;
        h.frame();
    }
    h.app.nav_tab = super::navigator::NavTab::Model;
    h.frame();
    h.app.discipline = Discipline::Architecture;
    draw_outline(&mut h);
    assert_eq!(h.model().walls().count(), 4);
}

/// Working in Structure greys the rest of the plan; a discipline hidden
/// in the navigator leaves the views; one locked cannot be selected; an
/// element can be given another discipline in Properties.
#[test]
fn disciplines_grey_hide_and_lock() {
    use super::toolbox::Discipline;
    use archisim_core::{DisciplineFocus, ElementDiscipline};
    let mut h = Harness::new(Editor::new_project());
    draw_outline(&mut h);
    h.key(Key::Escape);
    assert_eq!(h.app.view_style.focus, DisciplineFocus::None);
    h.app.set_discipline(Discipline::Structure);
    h.frame();
    assert_eq!(
        h.app.view_style.focus,
        DisciplineFocus::Only(ElementDiscipline::Structure)
    );
    h.app.set_discipline(Discipline::Concept);
    assert_eq!(h.app.view_style.focus, DisciplineFocus::Volumes);
    h.app.set_discipline(Discipline::Architecture);
    assert_eq!(h.app.view_style.focus, DisciplineFocus::None);
    // Attenuation off: the discipline does not change the view.
    h.app.attenuate = false;
    h.app.set_discipline(Discipline::Structure);
    assert_eq!(h.app.view_style.focus, DisciplineFocus::None);
    h.app.attenuate = true;
    h.app.set_discipline(Discipline::Architecture);
    // The walls of the template's façade: architecture (brick), so the
    // eye of Architecture hides them in every tab.
    let w = h.model().walls().next().unwrap().id;
    assert_eq!(
        h.model().discipline(w.0),
        Some(ElementDiscipline::Architecture)
    );
    h.app
        .toggle_hidden_discipline(ElementDiscipline::Architecture);
    assert!(h
        .app
        .view_style
        .hidden_disciplines
        .contains(&ElementDiscipline::Architecture));
    h.frame();
    h.app
        .toggle_hidden_discipline(ElementDiscipline::Architecture);
    // Locked: a click selects nothing of it.
    h.app
        .locked_disciplines
        .insert(ElementDiscipline::Architecture);
    h.app.ed.selection = [Element::Wall(w)].into_iter().collect();
    h.frame();
    assert!(h.app.ed.selection.is_empty());
    h.app.locked_disciplines.clear();
    // By hand: a bearing brick wall.
    h.app
        .ed
        .set_discipline(&[w.0], Some(ElementDiscipline::Structure))
        .unwrap();
    assert_eq!(
        h.model().discipline(w.0),
        Some(ElementDiscipline::Structure)
    );
    h.app.ed.selection = [Element::Wall(w)].into_iter().collect();
    h.frame();
    h.app.ed.undo();
    assert_eq!(
        h.model().discipline(w.0),
        Some(ElementDiscipline::Architecture)
    );
}

/// The command line is hidden until an order is typed; the step of a
/// running order is in the contextual bar, and the bar's methods change
/// how the tool draws.
#[test]
fn command_line_hidden_and_contextual_bar() {
    let mut h = Harness::new(Editor::new_project());
    assert!(!h.app.command_line_shown());
    h.app.keys_autocad = true;
    h.type_text("l");
    assert_eq!(h.app.cmdline.buffer, "l", "written beside the cursor");
    assert!(!h.app.command_line_shown());
    h.key(Key::Enter);
    assert_eq!(h.app.tool, Tool::Cad);
    assert!(
        h.app.cad_prompt().is_some(),
        "its step is in the contextual bar"
    );
    h.key(Key::Escape);
    h.key(Key::Escape);
    h.app.pin_command_line = true;
    assert!(h.app.command_line_shown());
    h.app.pin_command_line = false;
    h.app.keys_autocad = false;
    // Methods: the wall in a rectangle; the slab with the wand.
    h.type_text("m");
    h.app.choose_method("Rectángulo");
    assert_eq!(h.app.wall_tool.mode, crate::wall_tool::WallMode::Rectangle);
    let states = h.app.method_states();
    assert!(states.contains(&("Rectángulo", true, true)));
    // Spline and Elegir línea are there for walls; Polígono, faint.
    assert!(states.contains(&("Spline", true, false)));
    assert!(
        states.contains(&("Polígono", false, false)),
        "faint, not gone"
    );
    h.frame();
    h.type_text("f");
    h.app.choose_method("Varita");
    assert!(h.app.slab_settings.wand);
    h.app.choose_method("Polígono");
    assert!(!h.app.slab_settings.wand);
    h.frame();
    h.key(Key::Escape);
    // Nothing selected: «Modificar» in grey; a wall selected, its orders.
    h.type_text("m");
    h.app.choose_method("Cadena");
    h.key(Key::Escape);
    draw_outline(&mut h);
    h.key(Key::Escape);
    let w = h.model().walls().next().unwrap().id;
    h.app.ed.selection = [Element::Wall(w)].into_iter().collect();
    h.frame();
    assert_eq!(
        super::context_bar::selection_label(&h.app.ed.selection),
        "Muro (1)"
    );
}

/// Snaps switched off in the bottom bar are not offered while drawing.
#[test]
fn snaps_follow_the_bottom_bar() {
    let mut h = Harness::new(Editor::new_project());
    draw_outline(&mut h);
    h.key(Key::Escape);
    h.type_text("m");
    h.move_to(p(0.04, 8.97));
    let kind = h.app.cursor.unwrap().kind;
    assert!(
        matches!(kind, SnapKind::Endpoint | SnapKind::Corner),
        "{kind:?}"
    );
    h.app.snap_filter.points = false;
    h.app.snap_filter.edges = false;
    h.app.snap_filter.midpoints = false;
    h.move_to(p(0.041, 8.971));
    assert_eq!(h.app.cursor.unwrap().kind, SnapKind::Free);
    // Without tracking, a point near the horizontal is not pulled onto it.
    h.app.snap_filter = SnapFilter::default();
    h.click_at(p(3.0, 3.0));
    h.move_to(p(8.0, 3.05));
    assert_eq!(h.app.cursor.unwrap().kind, SnapKind::Tracked);
    h.app.snap_filter.tracking = false;
    h.move_to(p(8.0, 3.051));
    assert_ne!(h.app.cursor.unwrap().kind, SnapKind::Tracked);
}

/// The properties palette draws every state: a tool, each kind of
/// element, several, nothing (the view, in plan and in 3D), in both
/// themes, and the type selector changes the type of the selection.
#[test]
fn properties_palette_shows_every_state() {
    let mut h = Harness::new(Editor::new_project());
    draw_outline(&mut h);
    furnish(&mut h);
    h.key(Key::Escape);
    // Annotations to show too.
    h.type_text("n");
    h.click_at(p(6.0, 4.0));
    h.key(Key::Escape);
    for dark in [false, true] {
        h.app.dark = dark;
        theme::apply(&h.ctx, dark);
        for key in ["m", "f", "p", "v", "c", "n"] {
            h.type_text(key);
            h.frame();
            h.key(Key::Escape);
        }
        // The type pictures of walls, slabs, doors and windows were drawn
        // (one per type of the template) and are kept.
        let types = h.model().wall_types().count()
            + h.model().slab_types().count()
            + h.model().opening_types().count();
        assert!(h.app.thumbs.count() >= types, "{}", h.app.thumbs.count());
        let m = h.model();
        let all: Vec<Element> = crate::pick::elements_on_level(m, h.app.ed.level);
        for e in &all {
            h.app.ed.selection = [*e].into_iter().collect();
            h.frame();
        }
        h.app.ed.selection = all.iter().copied().collect();
        h.frame();
        let walls: Vec<Element> = all
            .iter()
            .copied()
            .filter(|e| matches!(e, Element::Wall(_)))
            .collect();
        h.app.ed.selection = walls.iter().copied().collect();
        h.frame();
        h.app.ed.selection.clear();
        h.frame();
        h.key(Key::F3);
        h.frame();
        h.key(Key::F2);
    }
    // Same-type selection from the palette's helper, as the button does.
    let west = Element::Wall(h.model().walls().next().unwrap().id);
    let t = crate::pick::type_of(h.model(), west).unwrap();
    assert_eq!(
        crate::pick::of_type_on_level(h.model(), h.app.ed.level, t).len(),
        4
    );
}

/// Walls drawn with "Planta superior" go up to the level above (free 3 m
/// when there is none) and follow it when its elevation changes; the
/// palette shows them; undo puts them back.
#[test]
fn walls_drawn_up_to_the_floor_above_follow_it() {
    use archisim_core::WallTop;
    let mut h = Harness::new(Editor::new_project());
    // A single level: nothing above, the walls get their own 3 m.
    draw_outline(&mut h);
    assert!(h
        .model()
        .walls()
        .all(|w| w.top == WallTop::Unconnected && w.height == 3.0));
    let ground = h.app.ed.level;
    let first = h.app.ed.add_level("Planta primera", 3.2).unwrap();
    h.frame();
    h.app.activate_tab(0);
    h.frame();
    assert_eq!(h.app.ed.level, ground);

    // Up to the floor above, 30 cm under it (the slab).
    h.app.wall_settings.top_offset = -0.3;
    h.key(Key::Escape);
    h.type_text("m");
    h.frame();
    h.click_at(p(3.0, 3.0));
    h.click_at(p(3.0, 6.0));
    h.key(Key::Escape);
    let w = h.model().walls().last().unwrap().clone();
    assert_eq!(
        w.top,
        WallTop::Level {
            level: first,
            offset: -0.3
        }
    );
    assert!((w.height - 2.9).abs() < 1e-9, "{}", w.height);

    // The floor above goes up 30 cm: the wall follows, the free ones not.
    h.app.ed.set_level_elevation(first, 3.5).unwrap();
    assert!((h.model().wall(w.id).unwrap().height - 3.2).abs() < 1e-9);
    assert_eq!(
        h.model().walls().filter(|x| x.height == 3.0).count(),
        4,
        "the outline keeps its own height"
    );
    // The palette shows the tied wall, alone and with the others.
    h.key(Key::Escape);
    h.app.ed.selection = [Element::Wall(w.id)].into_iter().collect();
    h.frame();
    h.app.ed.selection = h.model().walls().map(|w| Element::Wall(w.id)).collect();
    h.frame();
    // Freed, it keeps its height; undo ties it again.
    h.app
        .ed
        .set_walls_top(&[w.id], WallTop::Unconnected)
        .unwrap();
    assert!((h.model().wall(w.id).unwrap().height - 3.2).abs() < 1e-9);
    h.set_modifiers(Modifiers::COMMAND);
    h.key(Key::Z);
    h.key(Key::Z);
    h.set_modifiers(Modifiers::NONE);
    assert!((h.model().wall(w.id).unwrap().height - 2.9).abs() < 1e-9);
}

/// Walls come out square without Shift, unless the cursor leaves the axis
/// clearly, and a partition drawn to a face ends square on it.
#[test]
fn walls_are_square_by_default() {
    let mut h = Harness::new(Editor::new_project());
    h.type_text("m");
    h.click_at(p(0.0, 0.0));
    // 4° off the horizontal: straight.
    h.move_to(p(4.0, 0.28));
    assert_eq!(h.app.cursor.unwrap().kind, SnapKind::Tracked);
    assert!(is_tracking_line(p(0.0, 0.0), h.app.cursor.unwrap().point));
    h.click_at(p(4.0, 0.28));
    // 2° off the vertical: straight.
    h.click_at(p(4.1, 3.0));
    // 37°: the wall follows the cursor (inclined on purpose).
    h.click_at(p(6.0, 4.5));
    h.key(Key::Escape);
    let ends: Vec<(Point2, Point2)> = h.model().walls().map(|w| (w.start, w.end)).collect();
    assert_eq!(
        ends,
        vec![
            (p(0.0, 0.0), p(4.0, 0.0)),
            (p(4.0, 0.0), p(4.0, 3.0)),
            (p(4.0, 3.0), p(6.0, 4.5)),
        ]
    );
    // The last wall drawn stays selected, with its temporary dimensions.
    let last = h.model().walls().last().unwrap().id;
    assert_eq!(
        h.app.ed.selection,
        [Element::Wall(last)].into_iter().collect()
    );
    assert!(!h.app.shown_temp_dims().is_empty());
}

/// Revit's temporary dimensions and flip controls on a door: click the
/// blue value, type, Enter; Space and the arrows turn it.
#[test]
fn temporary_dimensions_place_and_turn_a_door() {
    let mut h = Harness::new(Editor::new_project());
    draw_outline(&mut h);
    furnish(&mut h);
    // The window just placed is selected, with its dimensions.
    let window = h.model().openings().last().unwrap().id;
    assert_eq!(
        h.app.ed.selection,
        [Element::Opening(window)].into_iter().collect()
    );
    assert_eq!(h.app.shown_temp_dims().len(), 2);
    h.key(Key::Escape);
    assert_eq!(h.app.tool, Tool::Arrow);
    let door = h.model().openings().next().unwrap().id;
    h.app.ed.selection = [Element::Opening(door)].into_iter().collect();
    h.frame();
    let dims = h.app.shown_temp_dims();
    assert_eq!(dims.len(), 2, "{dims:?}");

    // Click the value of the first one and type 500.
    let label = h.app.temp_layout(h.app.canvas, &dims[0]).4;
    h.click_screen(label.center());
    assert!(h.app.temp_edit.is_some(), "{:?}", h.app.message);
    h.frame();
    h.type_text("500");
    h.key(Key::Enter);
    h.frame();
    assert!(h.app.temp_edit.is_none());
    let after = h.app.shown_temp_dims();
    assert!(
        after
            .iter()
            .any(|d| d.target == dims[0].target && (d.value - 0.5).abs() < 1e-9),
        "{after:?} {:?}",
        h.app.message
    );
    assert_eq!(h.app.ed.undo_label(), Some("Mover"));
    // Esc while typing leaves it as it was.
    let label = h.app.temp_layout(h.app.canvas, &after[1]).4;
    let before = h.model().clone();
    h.click_screen(label.center());
    h.frame();
    h.type_text("900");
    h.key(Key::Escape);
    h.frame();
    assert!(h.app.temp_edit.is_none());
    assert_eq!(h.model(), &before);
    assert!(
        h.app.ed.selection.contains(&Element::Opening(door)),
        "Esc closes the field only"
    );

    // Space: the hinges go to the other jamb, then the leaf to the other
    // side… four times round.
    let o = h.model().opening(door).unwrap().clone();
    h.key(Key::Space);
    let turned = h.model().opening(door).unwrap().clone();
    assert_eq!(
        (turned.opens_to, turned.hand),
        crate::flip::next(o.opens_to, o.hand)
    );
    for _ in 0..3 {
        h.key(Key::Space);
    }
    let back = h.model().opening(door).unwrap();
    assert_eq!((back.opens_to, back.hand), (o.opens_to, o.hand));

    // The arrows: across the wall and along it.
    let controls = h.app.flip_controls(h.app.canvas);
    assert_eq!(controls.len(), 2);
    let side = controls
        .iter()
        .find(|c| c.1 == temp_ui::FlipControl::Side)
        .unwrap()
        .0;
    h.click_screen(side);
    let flipped = h.model().opening(door).unwrap().clone();
    assert_ne!(flipped.opens_to, o.opens_to, "{:?}", h.app.message);
    assert_eq!(
        crate::flip::hinge_at_start(flipped.opens_to, flipped.hand),
        crate::flip::hinge_at_start(o.opens_to, o.hand),
        "the hinges stay on their jamb"
    );
    // Still selected: the arrows act again.
    assert!(h.app.ed.selection.contains(&Element::Opening(door)));
    let hand = h
        .app
        .flip_controls(h.app.canvas)
        .iter()
        .find(|c| c.1 == temp_ui::FlipControl::Hand)
        .unwrap()
        .0;
    h.click_screen(hand);
    assert_ne!(h.model().opening(door).unwrap().hand, flipped.hand);

    // A wall selected: Space flips it; its dimensions are its length and
    // the clear distance to the façade opposite.
    let west = h.model().walls().next().unwrap().clone();
    h.app.ed.selection = [Element::Wall(west.id)].into_iter().collect();
    h.frame();
    let dims = h.app.shown_temp_dims();
    assert!(
        dims.iter().any(|d| (d.value - 9.0).abs() < 1e-9),
        "{dims:?}"
    );
    h.key(Key::Space);
    assert_eq!(h.model().wall(west.id).unwrap().start, west.end);
}

/// A wall or slab goes to another level from its properties; the plan of
/// a level shows the walls of other levels its cut plane crosses, and the
/// level below in grey, which snaps.
#[test]
fn elements_change_level_and_other_levels_show() {
    let mut h = Harness::new(Editor::new_project());
    draw_outline(&mut h);
    h.key(Key::Escape);
    let ground = h.app.ed.level;
    let walls: Vec<_> = h.model().walls().map(|w| w.id).collect();
    let first = h.app.ed.add_level("Planta primera", 3.0).unwrap();
    h.frame();
    assert_eq!(h.app.ed.level, first);
    // The ground floor walls (3 m) end under the cut plane of the first
    // floor: they show as its reference, and their corners snap.
    assert_eq!(h.app.other_walls.len(), 4);
    h.type_text("m");
    h.move_to(p(0.03, 0.02));
    assert_eq!(h.app.cursor.unwrap().point, p(0.0, 0.0));
    h.key(Key::Escape);
    h.key(Key::Escape);
    // Without the reference, nothing of the ground floor.
    h.app.show_underlay = false;
    h.frame();
    assert!(h.app.other_walls.is_empty());
    // A ground-floor wall 5 m high is cut by the first floor's plane: it is
    // shown (and snaps) anyway.
    h.app.ed.set_walls_height(&walls[..1], 5.0).unwrap();
    h.frame();
    assert_eq!(h.app.other_walls, vec![walls[0]]);
    let svg = archisim_plan::render_plan(h.model(), first, &archisim_plan::PlanOptions::default());
    assert!(svg.contains("<path"), "the cut wall is drawn");

    // Move the other three walls up to the first floor in one step.
    let three: Vec<Element> = walls[1..].iter().map(|w| Element::Wall(*w)).collect();
    h.app.ed.set_elements_level(&three, first).unwrap();
    assert_eq!(h.app.ed.undo_label(), Some("Cambiar de planta"));
    assert!(walls[1..]
        .iter()
        .all(|w| h.model().wall(*w).unwrap().level == first));
    assert_eq!(h.model().wall(walls[0]).unwrap().level, ground);
    // From the first floor, back down: they leave the selection.
    h.app.ed.selection = three.iter().copied().collect();
    let n = h.app.ed.set_elements_level(&three, ground).unwrap();
    assert_eq!(n, 3);
    assert!(h.app.ed.selection.is_empty());
    h.frame();
}

/// The geometry methods of the wall and slab tools, click by click: a
/// rectangle of walls, a circle, an arc, a rectangle by its typed size and
/// a rectangular slab; then a curved wall bent with its handle, and a door
/// refused in a curved wall. With `ARCHISIM_TEST_OUT` set, leaves
/// `modos.archisim` and `modos.ifc` there.
#[test]
fn construction_modes_by_clicks() {
    use crate::slab_tool::SlabMode;
    use crate::wall_tool::WallMode;
    let mut h = Harness::new(Editor::new_project());
    h.type_text("m");
    assert_eq!(h.app.tool, Tool::Wall);

    // Rectangle: two opposite corners, four walls closed at the corners.
    h.app.wall_tool.set_mode(WallMode::Rectangle);
    h.click_at(p(0.0, 0.0));
    h.click_at(p(5.0, 4.0));
    assert_eq!(h.model().walls().count(), 4, "{:?}", h.app.message);
    assert_eq!(h.app.ed.undo_label(), Some("Crear muros"));
    assert!(!h.app.wall_tool.is_drawing());
    for w in h.model().walls() {
        let g = wall_geometry(h.model(), w.id).unwrap();
        assert!(g.start_cut.is_mitre() && g.end_cut.is_mitre());
    }
    let ring: Vec<Point2> = h.model().walls().map(|w| w.start).collect();
    assert!(archisim_core::signed_area(&ring) < 0.0, "clockwise");

    // Circle: centre and a point of the drawn line.
    h.app.wall_tool.set_mode(WallMode::Circle);
    h.click_at(p(9.0, 3.0));
    h.click_at(p(11.0, 3.0));
    let curved: Vec<archisim_core::Wall> = h
        .model()
        .walls()
        .filter(|w| w.is_curved())
        .cloned()
        .collect();
    assert_eq!(curved.len(), 2, "{:?}", h.app.message);
    for w in &curved {
        let a = w.arc().unwrap();
        assert!(a.centre.approx_eq(p(9.0, 3.0)));
        assert!((a.radius - 2.0).abs() < 1e-9);
    }
    let msg = h.app.message.clone().unwrap();
    assert!(msg.text.contains("radio 2000"), "{msg:?}");

    // Arc: start, end and a point it passes through; Escape ends it.
    h.app.wall_tool.set_mode(WallMode::Arc);
    h.click_at(p(0.0, 7.0));
    h.click_at(p(5.0, 7.0));
    h.click_at(p(2.5, 8.5));
    h.key(Key::Escape);
    let arc_wall = h
        .model()
        .walls()
        .find(|w| w.is_curved() && w.start.approx_eq(p(0.0, 7.0)))
        .cloned()
        .expect("the arc");
    assert!(arc_wall.midpoint().distance(p(2.5, 8.5)) < 1e-6);

    // Rectangle by its typed size, towards the cursor.
    h.app.wall_tool.set_mode(WallMode::Rectangle);
    h.click_at(p(6.5, 6.5));
    h.move_to(p(8.0, 8.0));
    h.type_text("3000x2000");
    h.key(Key::Enter);
    assert_eq!(h.model().walls().count(), 11, "{:?}", h.app.message);
    assert!(h
        .model()
        .walls()
        .any(|w| w.end.approx_eq(p(9.5, 8.5)) || w.start.approx_eq(p(9.5, 8.5))));

    // A rectangular slab on the first rectangle.
    h.key(Key::Escape);
    h.type_text("f");
    h.app.slab_tool.set_mode(SlabMode::Rectangle);
    h.click_at(p(0.0, 0.0));
    h.click_at(p(5.0, 4.0));
    let slab = h.model().slabs().next().expect("a slab");
    assert!((slab.area() - 20.0).abs() < 1e-9);

    // Bending the arc with its handle at the middle.
    h.key(Key::Escape);
    h.key(Key::Escape);
    assert_eq!(h.app.tool, Tool::Arrow);
    h.click_at(p(2.5, 8.5));
    assert!(h.app.ed.selection.contains(&Element::Wall(arc_wall.id)));
    h.drag(p(2.5, 8.5), p(2.5, 9.5));
    let bent = h.model().wall(arc_wall.id).unwrap().clone();
    assert!(
        bent.midpoint().distance(p(2.5, 9.5)) < 0.02,
        "{:?} {:?}",
        bent.midpoint(),
        h.app.message
    );
    assert_eq!(h.app.ed.undo_label(), Some("Curvar muro"));
    assert_eq!((bent.start, bent.end), (arc_wall.start, arc_wall.end));

    // No doors in curved walls yet: the click says why.
    h.type_text("p");
    let before = h.model().openings().count();
    h.click_at(p(9.0, 5.0));
    assert_eq!(h.model().openings().count(), before);
    let msg = h.app.message.clone().unwrap();
    assert!(msg.error && msg.text.contains("curvos"), "{msg:?}");
    h.key(Key::Escape);

    // Saved, opened and exported as it is; the IFC comes back the same.
    let dir = match std::env::var_os("ARCHISIM_TEST_OUT") {
        Some(d) => PathBuf::from(d),
        None => std::env::temp_dir().join(format!("archisim-modos-{}", std::process::id())),
    };
    std::fs::create_dir_all(&dir).unwrap();
    let path = h.app.ed.save(Some(&dir.join("modos.archisim"))).unwrap();
    let reopened = Editor::open(&path).unwrap();
    assert_eq!(reopened.model(), h.model());
    reopened
        .export_ifc(&dir.join("modos.ifc"), archisim_ifc::Schema::Ifc4x3)
        .unwrap();
    let ifc = std::fs::read_to_string(dir.join("modos.ifc")).unwrap();
    assert_eq!(ifc.matches("IFCTRIMMEDCURVE(").count(), 3);
    let back = archisim_ifc::import_ifc(&ifc).unwrap();
    let diffs = archisim_ifc::differences(reopened.model(), &back.model);
    assert!(diffs.is_empty(), "{}", diffs.join("\n"));
    // The plan draws.
    h.frame();
    assert!(h.app.plan.is_some());
}

/// A stair hole cut through a slab with the slab tool, the slab moved with
/// its hole, the hole closed again from Properties; and a void ("hueco")
/// placed with the window tool.
#[test]
fn slab_holes_and_voids_by_clicks() {
    use crate::slab_tool::SlabMode;
    let mut h = Harness::new(Editor::new_project());
    draw_outline(&mut h);
    h.key(Key::Escape);
    h.type_text("f");
    h.app.slab_tool.set_mode(SlabMode::Rectangle);
    h.click_at(p(0.0, 0.0));
    h.click_at(p(12.0, 9.0));
    let slab = h.model().slabs().next().unwrap().id;
    // The hole: same tool, «Hueco» ticked.
    h.app.slab_settings.hole = true;
    h.click_at(p(2.0, 2.0));
    h.click_at(p(3.0, 5.0));
    let s = h.model().slab(slab).unwrap().clone();
    assert_eq!(s.holes.len(), 1, "{:?}", h.app.message);
    assert!((s.area() - (108.0 - 3.0)).abs() < 1e-9);
    assert_eq!(h.app.ed.undo_label(), Some("Hueco en forjado"));
    // Across the edge of the slab: refused.
    h.click_at(p(11.5, 2.0));
    h.click_at(p(12.5, 3.0));
    let msg = h.app.message.clone().unwrap();
    assert!(
        msg.error && msg.text.contains("dentro de un forjado"),
        "{msg:?}"
    );
    h.app.slab_settings.hole = false;
    h.key(Key::Escape);
    h.key(Key::Escape);
    // A click in the hole selects nothing; on the slab, the slab.
    h.click_at(p(2.5, 3.5));
    assert!(!h.app.ed.selection.contains(&Element::Slab(slab)));
    h.click_at(p(6.5, 7.0));
    assert!(h.app.ed.selection.contains(&Element::Slab(slab)));
    // Moving the slab takes its hole along.
    h.drag(p(6.5, 7.0), p(6.5, 8.0));
    let moved = h.model().slab(slab).unwrap().clone();
    assert!(
        (moved.holes[0][0].y - s.holes[0][0].y - 1.0).abs() < 1e-9,
        "{:?}",
        h.app.message
    );
    h.app.ed.undo();
    // Closing it again.
    h.app.ed.remove_slab_hole(slab, 0).unwrap();
    assert!(h.model().slab(slab).unwrap().holes.is_empty());

    // A void in the north façade, with the window tool.
    let void = h
        .model()
        .opening_types()
        .find(|t| t.is_void())
        .expect("the template has a void")
        .id;
    h.type_text("v");
    h.app.window_settings.opening_type = void;
    h.app.window_settings.sill = 0.0;
    h.click_at(p(6.0, 8.8));
    let o = h
        .model()
        .openings()
        .find(|o| o.opening_type == void)
        .cloned()
        .expect("a void");
    assert!((o.along - 6.0).abs() < 1e-9 && o.sill == 0.0, "{o:?}");
    // It exports as an opening alone, and comes back the same.
    let ifc = archisim_ifc::export_ifc(h.model(), &Default::default());
    assert!(ifc.contains("'ArchiSim_Void'"));
    let back = archisim_ifc::import_ifc(&ifc).unwrap();
    let diffs = archisim_ifc::differences(h.model(), &back.model);
    assert!(diffs.is_empty(), "{}", diffs.join("\n"));
}

/// Columns and beams with the mouse (2.5): `i` places columns that reach
/// the level above, `g` chains beams hanging from it; Space turns a
/// column; the type's section is edited in place; and they move, copy
/// and delete like the rest.
#[test]
fn column_and_beam_tools() {
    let mut ed = Editor::new_project();
    let pb = ed.level;
    ed.add_level("Planta 1", 3.0).unwrap();
    ed.level = pb;
    let mut h = Harness::new(ed);
    h.type_text("i");
    assert_eq!(h.app.tool, Tool::Column);
    h.click_at(p(0.0, 0.0));
    // Space turns the next one a quarter.
    h.key(Key::Space);
    h.click_at(p(5.0, 0.0));
    assert_eq!(h.model().columns().count(), 2, "{:?}", h.app.message);
    let cols: Vec<_> = h.model().columns().cloned().collect();
    // Snapped to the project origin, up to the floor above.
    assert_eq!(cols[0].position, p(0.0, 0.0));
    assert!((cols[0].height - 3.0).abs() < 1e-9);
    assert!((cols[1].rotation - std::f64::consts::FRAC_PI_2).abs() < 1e-12);

    h.key(Key::Escape);
    h.type_text("g");
    assert_eq!(h.app.tool, Tool::Beam);
    // From centre to centre of the columns (they snap), then on.
    h.click_at(p(0.02, 0.01));
    h.click_at(p(4.99, -0.02));
    h.click_at(p(5.0, 4.0));
    h.key(Key::Escape);
    let m = h.model();
    assert_eq!(m.beams().count(), 2, "{:?}", h.app.message);
    let b = m.beams().next().unwrap();
    assert_eq!((b.start, b.end), (p(0.0, 0.0), p(5.0, 0.0)));
    // Drawn on the ground floor, it belongs to the floor it carries.
    assert_eq!(m.level(b.level).unwrap().name, "Planta 1");

    // Select the first column and move it 1 m north.
    h.key(Key::Escape);
    assert_eq!(h.app.tool, Tool::Arrow);
    let first = cols[0].id;
    h.click_at(p(0.05, 0.05));
    assert_eq!(
        h.app.ed.selection.iter().copied().collect::<Vec<_>>(),
        vec![Element::Column(first)]
    );
    h.drag(p(0.05, 0.05), p(0.0, 1.0));
    assert_eq!(h.model().column(first).unwrap().position, p(0.0, 1.0));
    // Space turns it.
    h.key(Key::Space);
    assert!(h.model().column(first).unwrap().rotation.abs() > 1.0);
    // Delete.
    h.key(Key::Delete);
    assert_eq!(h.model().columns().count(), 1);
    // The plan draws the remaining column cut and the beams dashed.
    let svg = archisim_plan::render_plan(h.model(), pb, &archisim_plan::PlanOptions::default());
    assert!(svg.contains("cut-column-"), "{svg}");
}

/// The witness end of a temporary dimension dragged to another point (the
/// project origin): the dimension measures to it, and typing a value
/// places the door from there.
#[test]
fn temporary_dimension_witness_dragged_to_the_origin() {
    let mut h = Harness::new(Editor::new_project());
    draw_outline(&mut h);
    furnish(&mut h);
    h.key(Key::Escape);
    let door = h.model().openings().next().unwrap().id;
    h.app.ed.selection = [Element::Opening(door)].into_iter().collect();
    h.frame();
    let dims = h.app.shown_temp_dims();
    // The one towards the west (x = 0).
    let d = *dims
        .iter()
        .find(|d| d.a.x < d.b.x)
        .expect("a dimension to the west");
    h.drag(d.a, p(0.0, 0.0));
    let after = h.app.shown_temp_dims();
    let moved = after.iter().find(|x| x.slot == d.slot).unwrap();
    assert!(moved.a.x.abs() < 1e-9, "{moved:?} {:?}", h.app.message);
    // Its own end to the door's centre.
    let o = h.model().opening(door).unwrap().clone();
    let w = h.model().wall(o.wall).unwrap().clone();
    let u = (w.end - w.start).normalized().unwrap();
    let mid = w.start + u * o.along;
    let own = Point2::new(mid.x, moved.b.y);
    h.drag(moved.b, own);
    let after = h.app.shown_temp_dims();
    let moved = *after.iter().find(|x| x.slot == d.slot).unwrap();
    assert!((moved.value - mid.x).abs() < 1e-6, "{moved:?}");
    // Type 2000: the door's centre 2 m from the origin.
    let label = h.app.temp_layout(h.app.canvas, &moved).4;
    h.click_screen(label.center());
    h.frame();
    h.type_text("2000");
    h.key(Key::Enter);
    h.frame();
    let o = h.model().opening(door).unwrap().clone();
    let w = h.model().wall(o.wall).unwrap().clone();
    let u = (w.end - w.start).normalized().unwrap();
    assert!(((w.start + u * o.along).x - 2.0).abs() < 1e-6);
}

/// Rooms (2.9): `z`, a click inside the outline makes the room with its
/// usable area; the next one gets the next number; clicking its stamp
/// selects it; its stamp can be dragged; and it is deleted like the rest.
#[test]
fn room_tool() {
    let mut h = Harness::new(Editor::new_project());
    draw_outline(&mut h);
    h.key(Key::Escape);
    h.type_text("z");
    assert_eq!(h.app.tool, Tool::Zone);
    h.click_at(p(6.0, 4.5));
    let m = h.model();
    assert_eq!(m.rooms().count(), 1, "{:?}", h.app.message);
    let s = m.rooms().next().unwrap().clone();
    assert!((s.area() - (12.0 - 0.53) * (9.0 - 0.53)).abs() < 1e-6);
    assert_eq!(s.number, "1");
    // A second one in the same room is refused.
    h.click_at(p(3.0, 3.0));
    assert_eq!(h.model().rooms().count(), 1);
    // By hand, a rectangle (a part of the room): number 2.
    h.app.room_settings.wand = false;
    h.app.zone_tool.set_mode(SlabMode::Rectangle);
    h.click_at(p(1.0, 1.0));
    h.click_at(p(4.0, 3.0));
    assert_eq!(h.model().rooms().count(), 2, "{:?}", h.app.message);
    assert!(h.model().rooms().any(|s| s.number == "2"));
    // Select the first by its stamp, drag the stamp.
    h.key(Key::Escape);
    h.key(Key::Escape);
    assert_eq!(h.app.tool, Tool::Arrow);
    let at = s.label;
    h.click_at(at);
    assert_eq!(
        h.app.ed.selection.iter().copied().collect::<Vec<_>>(),
        vec![Element::Room(s.id)]
    );
    h.drag(at, at + Point2::new(1.0, 1.0));
    let moved = h.model().room(s.id).unwrap().label;
    assert!(
        moved.distance(at + Point2::new(1.0, 1.0)) < 0.02,
        "{moved:?}"
    );
    h.key(Key::Delete);
    assert_eq!(h.model().rooms().count(), 1);
}

/// Phases of a refurbishment: walls drawn "as existing", one set to be
/// demolished in Properties, and the plan of each phase.
#[test]
fn phases_of_a_refurbishment() {
    let mut h = Harness::new(Editor::new_project());
    h.app.ed.set_draw_phase(archisim_core::Phase::Existing);
    draw_outline(&mut h);
    h.app.ed.set_draw_phase(archisim_core::Phase::New);
    let m = h.model();
    assert!(m
        .walls()
        .all(|w| m.phase(w.id.0) == archisim_core::Phase::Existing));
    // A new partition.
    h.key(Key::Escape);
    h.type_text("m");
    h.click_at(p(6.0, 0.265));
    h.click_at(p(6.0, 8.735));
    h.key(Key::Escape);
    h.key(Key::Escape);
    let partition = h.model().walls().last().unwrap().id;
    assert_eq!(h.model().phase(partition.0), archisim_core::Phase::New);
    // The north façade to be demolished.
    let north = h.model().walls().nth(1).unwrap().id;
    h.app.ed.selection = [Element::Wall(north)].into_iter().collect();
    assert_eq!(
        h.app
            .ed
            .set_phase(&[Element::Wall(north)], archisim_core::Phase::Demolished),
        Ok(1)
    );
    let level = h.app.ed.level;
    let plan = |view| {
        archisim_plan::render_plan(
            h.model(),
            level,
            &archisim_plan::PlanOptions {
                phase_view: view,
                ..Default::default()
            },
        )
    };
    use archisim_plan::phases::PhaseView;
    let yellow = "rgb(246,214,40)";
    let red = "rgb(222,74,60)";
    assert!(plan(PhaseView::Demolition).contains(yellow));
    assert!(!plan(PhaseView::Demolition).contains(red));
    assert!(plan(PhaseView::Proposed).contains(red));
    assert!(!plan(PhaseView::Proposed).contains(yellow));
    assert!(!plan(PhaseView::All).contains(red));
    // Undo keeps the drawing phase setting.
    h.app.ed.set_draw_phase(archisim_core::Phase::Existing);
    h.app.ed.undo();
    assert_eq!(h.app.ed.draw_phase(), archisim_core::Phase::Existing);
}

/// Sections and elevations: the section tool (S) draws a line with two
/// clicks; a double click on its mark opens its view in a tab, drawn from
/// the model; the elevations are made from the toolbox; deleting a section
/// closes its tab.
#[test]
fn sections_and_elevations() {
    let mut h = Harness::new(Editor::new_project());
    draw_outline(&mut h);
    h.key(Key::Escape);
    h.key(Key::Escape);
    h.type_text("s");
    assert_eq!(h.app.tool, Tool::Section);
    h.click_at(p(-0.8, 4.0));
    h.click_at(p(12.8, 4.0));
    let m = h.model();
    assert_eq!(m.sections().count(), 1, "{:?}", h.app.message);
    let sec = m.sections().next().unwrap().clone();
    assert_eq!(sec.title(), "Sección A");
    assert_eq!(sec.looks(), Some(Point2::new(0.0, 1.0)));
    h.key(Key::Escape);
    assert_eq!(h.app.tool, Tool::Arrow);
    // Its view: double click on the line.
    let on_line = p(3.0, 4.0);
    h.click_at(on_line);
    assert_eq!(
        h.app.ed.selection.iter().copied().collect::<Vec<_>>(),
        vec![Element::Section(sec.id)]
    );
    h.button(PointerButton::Primary, true);
    h.button(PointerButton::Primary, false);
    h.button(PointerButton::Primary, true);
    h.button(PointerButton::Primary, false);
    h.frame();
    assert_eq!(h.app.mode, ViewMode::Section);
    assert_eq!(h.app.active_section(), Some(sec.id));
    h.frame();
    assert!(h.app.section_cache.is_some(), "drawn");
    // Elements are selected in the section (R5): the west wall where it is
    // cut, the north wall seen beyond; Esc clears.
    h.click_at(p(0.8, 1.5));
    let picked: Vec<Element> = h.app.ed.selection.iter().copied().collect();
    let [Element::Wall(west)] = picked.as_slice() else {
        panic!("{picked:?}");
    };
    assert!(h.model().wall(*west).unwrap().start.x.abs() < 0.2);
    h.click_at(p(6.8, 1.5));
    let picked: Vec<Element> = h.app.ed.selection.iter().copied().collect();
    let [Element::Wall(north)] = picked.as_slice() else {
        panic!("{picked:?}");
    };
    assert_ne!(north, west);
    h.set_modifiers(Modifiers::SHIFT);
    h.click_at(p(0.8, 1.5));
    h.set_modifiers(Modifiers::NONE);
    assert_eq!(h.app.ed.selection.len(), 2);
    h.key(Key::Escape);
    assert!(h.app.ed.selection.is_empty());
    h.click_at(p(0.8, 1.5));
    h.click_at(p(6.8, 4.5));
    assert!(h.app.ed.selection.is_empty());
    // Back to the plan, flip it from Properties' editor and drag an end.
    h.key(Key::F2);
    assert_eq!(h.app.mode, ViewMode::Plan);
    h.app.ed.flip_section(sec.id).unwrap();
    assert_eq!(
        h.model().section(sec.id).unwrap().looks(),
        Some(Point2::new(0.0, -1.0))
    );
    // The four elevations.
    h.app.pick(super::toolbox::Pick::Action(
        super::toolbox::Action::Elevations,
    ));
    assert_eq!(h.model().sections().count(), 5, "{:?}", h.app.message);
    h.app.pick(super::toolbox::Pick::Action(
        super::toolbox::Action::Elevations,
    ));
    assert_eq!(h.model().sections().count(), 5);
    // Deleting the section closes its tab.
    let tabs = h.app.tabs.len();
    h.app.ed.selection = [Element::Section(sec.id)].into_iter().collect();
    h.key(Key::Delete);
    h.frame();
    assert_eq!(h.model().sections().count(), 4);
    assert_eq!(h.app.tabs.len(), tabs - 1);
    // Saved and opened again with the project.
    let text = archisim_file::to_string(h.model());
    let back = archisim_file::from_str(&text).unwrap();
    assert_eq!(back.sections().count(), 4);
}

/// An inclined slab: switched on in Properties (through the editor), its
/// arrow dragged in plan to turn it, and the level mark on it follows.
#[test]
fn inclined_slab_with_its_arrow() {
    let mut h = Harness::new(Editor::new_project());
    draw_outline(&mut h);
    h.key(Key::Escape);
    h.type_text("f");
    h.app.slab_settings.wand = true;
    h.app.slab_settings.contour = Contour::InteriorFace;
    h.click_at(p(6.0, 4.5));
    let slab = h.model().slabs().last().unwrap().clone();
    h.key(Key::Escape);
    h.key(Key::Escape);
    let a = super::properties::default_slope(&slab, Point2::new(1.0, 0.0), 0.05);
    h.app.ed.set_slab_slope(slab.id, Some(a)).unwrap();
    let s = h.model().slab(slab.id).unwrap().clone();
    let (lo, hi) = s.top_range();
    assert!((hi - lo - 0.05 * (12.0 - 0.53)).abs() < 1e-6, "{lo} {hi}");
    // Select it by its edge and drag the head of the arrow to the north.
    h.app.ed.selection = [Element::Slab(slab.id)].into_iter().collect();
    h.frame();
    let to = a.to;
    let north = Point2::new(a.from.x, a.from.y + 4.0);
    h.drag(to, north);
    let turned = h.model().slab(slab.id).unwrap().slope.unwrap();
    assert!(
        turned.to.distance(north) < 0.05,
        "{turned:?} {:?}",
        h.app.message
    );
    assert!((turned.slope - 0.05).abs() < 1e-12);
    // Undo, and level again.
    h.app.ed.undo();
    assert_eq!(h.model().slab(slab.id).unwrap().slope.unwrap().to, to);
    h.app.ed.set_slab_slope(slab.id, None).unwrap();
    assert!(h.model().slab(slab.id).unwrap().slope.is_none());
}

/// The roof tool (T): a click inside the building makes a hip roof over
/// the outer faces of its walls, on the level above; Properties turn it
/// into a gable roof; it is selected, moved and deleted like the rest.
#[test]
fn roof_tool() {
    let mut h = Harness::new(Editor::new_project());
    draw_outline(&mut h);
    let pb = h.app.ed.level;
    let top = h.app.ed.add_level("Cubierta", 3.0).unwrap();
    h.app.ed.level = pb;
    h.frame();
    h.key(Key::Escape);
    h.key(Key::Escape);
    h.type_text("t");
    assert_eq!(h.app.tool, Tool::Roof);
    h.click_at(p(6.0, 4.5));
    let m = h.model();
    assert_eq!(m.roofs().count(), 1, "{:?}", h.app.message);
    let r = m.roofs().next().unwrap().clone();
    assert_eq!(r.level, top);
    assert_eq!(r.planes().len(), 4);
    // Over the outer faces of the 12 × 9 outline, 50 cm of eaves.
    assert!((archisim_core::signed_area(&r.eaves()) - 13.0 * 10.0).abs() < 1e-6);
    assert_eq!(
        h.app.ed.selection.iter().copied().collect::<Vec<_>>(),
        vec![Element::Roof(r.id)]
    );
    h.app
        .ed
        .set_roof_kind(r.id, crate::editor::RoofKind::Gable, 0.3)
        .unwrap();
    assert_eq!(h.model().roof(r.id).unwrap().planes().len(), 2);
    // The walls under it go up to it: the gables come by themselves.
    assert_eq!(h.app.ed.trim_walls_under(r.id), Ok(4));
    let gable = h
        .model()
        .walls()
        .map(|w| h.model().wall_top_z(w))
        .fold(f64::MIN, f64::max);
    assert!(gable > 3.0, "{gable}");
    // It shows in the plan of its level and in 3D.
    let svg = archisim_plan::render_plan(h.model(), top, &archisim_plan::PlanOptions::default());
    assert!(svg.contains("class=\"roof\""));
    assert!(archisim_geom::solid::model_solids(h.model())
        .iter()
        .any(|s| s.owner == archisim_geom::solid::SolidOwner::Roof(r.id)));
    // Deleted with Supr.
    h.key(Key::Escape);
    h.app.ed.selection = [Element::Roof(r.id)].into_iter().collect();
    h.key(Key::Delete);
    assert_eq!(h.model().roofs().count(), 0);
    // The trims go with it.
    assert!(h
        .model()
        .walls()
        .all(|w| h.model().wall_roof(w.id).is_none()));
}

/// The stair tool (E): the walking line by clicks, Enter to finish; up to
/// the level above, checked against the code; its points dragged.
#[test]
fn stair_tool() {
    let mut h = Harness::new(Editor::new_project());
    draw_outline(&mut h);
    let pb = h.app.ed.level;
    h.app.ed.add_level("Planta 1", 2.97).unwrap();
    h.app.ed.level = pb;
    h.frame();
    h.key(Key::Escape);
    h.key(Key::Escape);
    h.type_text("e");
    assert_eq!(h.app.tool, Tool::Stair);
    // An L: 2,8 m up, then 2 m to the right, inside the room.
    h.click_at(p(1.0, 1.0));
    h.click_at(p(1.0, 4.3));
    h.click_at(p(3.8, 4.3));
    h.key(Key::Enter);
    let m = h.model();
    assert_eq!(m.stairs().count(), 1, "{:?}", h.app.message);
    let s = m.stairs().next().unwrap().clone();
    assert_eq!(s.path.len(), 3);
    let l = s.layout();
    assert_eq!(l.flights.len(), 2);
    assert_eq!(l.landings.len(), 1);
    assert!((s.height - 2.97).abs() < 1e-9);
    assert_eq!(
        h.app.ed.selection.iter().copied().collect::<Vec<_>>(),
        vec![Element::Stair(s.id)]
    );
    // Drawn in plan, and in 3D.
    let svg = archisim_plan::render_plan(h.model(), pb, &archisim_plan::PlanOptions::default());
    assert!(svg.contains("class=\"stair\""));
    // Its head dragged further: longer second flight.
    h.key(Key::Escape);
    h.app.ed.selection = [Element::Stair(s.id)].into_iter().collect();
    h.frame();
    h.drag(p(3.8, 4.3), p(4.6, 4.3));
    let moved = h.model().stair(s.id).unwrap().path[2];
    assert!(
        moved.distance(p(4.6, 4.3)) < 0.05,
        "{moved:?} {:?}",
        h.app.message
    );
    h.key(Key::Delete);
    assert_eq!(h.model().stairs().count(), 0);
}

/// The terrain tool (R): points with their heights, Enter to make the
/// terrain; the true north in the project's properties.
#[test]
fn terrain_tool_and_north() {
    let mut h = Harness::new(Editor::new_project());
    draw_outline(&mut h);
    h.key(Key::Escape);
    h.key(Key::Escape);
    h.type_text("r");
    assert_eq!(h.app.tool, Tool::Terrain);
    for (x, y, z) in [
        (-1.0, -1.0, -0.4),
        (13.0, -1.0, -0.6),
        (13.0, 10.0, 0.2),
        (-1.0, 10.0, 0.1),
    ] {
        h.app.terrain_settings.z = z;
        h.click_at(p(x, y));
    }
    assert_eq!(h.app.terrain_points.len(), 4);
    h.key(Key::Enter);
    let m = h.model();
    assert_eq!(m.terrains().count(), 1, "{:?}", h.app.message);
    let t = m.terrains().next().unwrap();
    assert_eq!(t.points.len(), 4);
    assert!(!t.contours().is_empty());
    h.app.ed.set_true_north(20.0).unwrap();
    let ifc = archisim_ifc::export_ifc(
        h.model(),
        &archisim_ifc::ExportOptions {
            file_name: "t.ifc".into(),
            author: "t".into(),
            organization: String::new(),
            timestamp: None,
            schema: Default::default(),
        },
    );
    assert!(ifc.contains("IFCGEOGRAPHICELEMENT"));
    assert!(ifc.contains("IFCDIRECTION((0.34202"));
}

/// The terrain in plan (R4): contour lines drawn at a height make the
/// terrain; clicks on its points give them the height written; with the
/// arrow, a point or a whole line is picked and given a height at once,
/// and Delete takes the picked points away.
#[test]
fn terrain_lines_and_picked_points() {
    let mut h = Harness::new(Editor::new_project());
    h.type_text("r");
    assert_eq!(h.app.tool, Tool::Terrain);
    h.app.terrain_settings.line = true;
    // Two contour lines: 0 m to the south, +1 m to the north.
    h.app.terrain_settings.z = 0.0;
    h.click_at(p(0.0, 0.0));
    h.click_at(p(10.0, 0.0));
    h.key(Key::Enter);
    assert!(h.model().terrains().next().is_none());
    assert_eq!(h.app.terrain_lines.len(), 1);
    h.app.terrain_settings.z = 1.0;
    h.click_at(p(0.0, 8.0));
    h.click_at(p(10.0, 8.0));
    h.key(Key::Enter);
    let t = h.model().terrains().next().expect("terrain").clone();
    assert_eq!(t.lines.len(), 2);
    // Points every metre at most: 11 on each line.
    assert_eq!(t.points.len(), 22);
    assert!((t.height_at(p(5.0, 4.0)).unwrap() - 0.5).abs() < 1e-9);
    // A third line goes into it; a point click on a point sets its height.
    h.app.terrain_settings.z = 0.5;
    h.click_at(p(0.0, 4.0));
    h.click_at(p(10.0, 4.0));
    h.key(Key::Enter);
    h.app.terrain_settings.line = false;
    h.app.terrain_settings.z = 2.0;
    h.click_at(p(10.0, 8.0));
    let t = h.model().terrains().next().unwrap().clone();
    assert_eq!(t.lines.len(), 3);
    assert_eq!(t.points.len(), 33);
    let corner = t.points.iter().position(|q| q.at == p(10.0, 8.0)).unwrap();
    assert_eq!(t.points[corner].z, 2.0);
    // With the arrow: click the middle line, all its points are picked.
    h.key(Key::Escape);
    assert_eq!(h.app.tool, Tool::Arrow);
    h.app.ed.selection = [Element::Terrain(t.id)].into_iter().collect();
    h.click_at(p(5.5, 4.0));
    assert_eq!(h.app.terrain_pick.len(), 11, "{:?}", h.app.message);
    assert!(h.app.ed.selection.contains(&Element::Terrain(t.id)));
    // A click on one point picks only it.
    h.click_at(p(0.0, 0.0));
    assert_eq!(h.app.terrain_pick.len(), 1);
    // Delete takes the picked point, not the terrain.
    h.key(Key::Delete);
    let t = h.model().terrains().next().expect("still there").clone();
    assert_eq!(t.points.len(), 32);
    assert_eq!(t.lines[0].len(), 10);
    // Away from points and lines, the click selects as usual.
    h.click_at(p(50.0, 50.0));
    assert!(h.app.terrain_pick.is_empty());
    // The lines are saved with the project.
    let back = archisim_file::from_str(&archisim_file::to_string(h.model())).unwrap();
    assert_eq!(back.terrains().next().unwrap().lines, t.lines);
}

#[test]
fn curtain_wall_tool() {
    let mut h = Harness::new(Editor::new_project());
    h.type_text("u");
    assert_eq!(h.app.tool, Tool::CurtainWall);
    h.click_at(p(0.0, 0.0));
    h.click_at(p(4.8, 0.0));
    let m = h.model();
    assert_eq!(m.curtain_walls().count(), 1, "{:?}", h.app.message);
    let c = m.curtain_walls().next().unwrap().clone();
    assert!((c.length() - 4.8).abs() < 1e-9);
    assert_eq!(h.app.ed.selection.len(), 1);
    // Its ends can be stretched, and it goes to the 3D and the IFC.
    let solids = archisim_geom::solid::model_solids(h.model());
    assert_eq!(solids.len(), 2);
    let ifc = archisim_ifc::export_ifc(
        h.model(),
        &archisim_ifc::ExportOptions {
            file_name: "t.ifc".into(),
            author: "t".into(),
            organization: String::new(),
            timestamp: None,
            schema: Default::default(),
        },
    );
    assert!(ifc.contains("IFCCURTAINWALL"));
    assert!(ifc.contains(".CURTAIN_PANEL."));
    h.key(Key::Escape);
    h.key(Key::Delete);
    assert_eq!(h.model().curtain_walls().count(), 0);
}

#[test]
fn railing_tool() {
    let mut h = Harness::new(Editor::new_project());
    h.type_text("b");
    assert_eq!(h.app.tool, Tool::Railing);
    h.click_at(p(0.0, 0.0));
    h.click_at(p(4.0, 0.0));
    h.click_at(p(4.0, 3.0));
    h.key(Key::Enter);
    let m = h.model();
    assert_eq!(m.railings().count(), 1, "{:?}", h.app.message);
    let r = m.railings().next().unwrap().clone();
    assert_eq!(r.path.len(), 3);
    assert!((r.height - 1.1).abs() < 1e-9);
    let ifc = archisim_ifc::export_ifc(
        h.model(),
        &archisim_ifc::ExportOptions {
            file_name: "t.ifc".into(),
            author: "t".into(),
            organization: String::new(),
            timestamp: None,
            schema: Default::default(),
        },
    );
    assert!(ifc.contains(".GUARDRAIL."));
    h.key(Key::Escape);
    h.key(Key::Delete);
    assert_eq!(h.model().railings().count(), 0);
}

#[test]
fn object_tool() {
    let mut h = Harness::new(Editor::new_project());
    h.type_text("o");
    assert_eq!(h.app.tool, Tool::Object);
    h.key(Key::Space);
    h.click_at(p(2.0, 2.0));
    let m = h.model();
    assert_eq!(m.objects().count(), 1, "{:?}", h.app.message);
    let o = m.objects().next().unwrap().clone();
    assert!((o.rotation - std::f64::consts::FRAC_PI_2).abs() < 1e-9);
    assert!(o.position.distance(archisim_core::Point2::new(2.0, 2.0)) < 0.05);
    // In the plan, the family's symbol; in the IFC, furniture.
    let svg = archisim_plan::render_plan(
        h.model(),
        h.app.ed.level,
        &archisim_plan::PlanOptions::default(),
    );
    assert!(svg.contains(r#"<g id="objects">"#));
    let ifc = archisim_ifc::export_ifc(
        h.model(),
        &archisim_ifc::ExportOptions {
            file_name: "t.ifc".into(),
            author: "t".into(),
            organization: String::new(),
            timestamp: None,
            schema: Default::default(),
        },
    );
    assert!(ifc.contains("IFCFURNITURE(") || ifc.contains("IFCSANITARYTERMINAL("));
    h.key(Key::Escape);
    h.key(Key::Delete);
    assert_eq!(h.model().objects().count(), 0);
}

/// Alignment guides: closing a house, the last corner lines up with the
/// first one without any wall there to snap to.
#[test]
fn alignment_guides_close_the_outline() {
    let mut h = Harness::new(Editor::new_project());
    h.type_text("m");
    h.click_at(p(2.0, 2.0));
    h.click_at(p(12.0, 2.0));
    h.click_at(p(12.0, 10.0));
    // Going west along y = 10, near x = 2: the vertical through the first
    // corner catches it.
    h.move_to(p(2.04, 10.02));
    let c = h.app.cursor.unwrap();
    assert_eq!(c.kind, SnapKind::Aligned, "{c:?}");
    assert_eq!(c.point, p(2.0, 10.0));
    assert_eq!(h.app.guides.len(), 1);
    // With tracking off in the bottom bar, no guides.
    h.app.snap_filter.tracking = false;
    h.move_to(p(2.04, 10.02));
    assert!(h.app.guides.is_empty());
}

/// The intersection priorities window (R7): it opens from the project's
/// properties, lists the materials of the layers; a priority set on one
/// and "Todas según la función de la capa" in one undoable step.
#[test]
fn intersection_priorities_window() {
    let mut h = Harness::new(Editor::new_project());
    h.app.show_priorities = true;
    h.frame();
    assert!(h.app.show_priorities);
    let ids: Vec<_> = h.model().materials().map(|m| m.id).take(2).collect();
    for id in &ids {
        h.app.ed.set_material_priority(*id, Some(950)).unwrap();
    }
    h.frame();
    assert_eq!(
        h.model()
            .materials()
            .filter(|m| m.priority.is_some())
            .count(),
        2
    );
    h.app.ed.reset_material_priorities().unwrap();
    assert!(h.model().materials().all(|m| m.priority.is_none()));
    h.app.ed.undo();
    assert_eq!(
        h.model()
            .materials()
            .filter(|m| m.priority.is_some())
            .count(),
        2
    );
    h.frame();
}

/// The object gallery (R8): the library's types the project lacks are
/// offered and loaded when chosen (one undoable step); trees and people
/// placed with the tool stand on the terrain under them.
#[test]
fn object_gallery_loads_from_the_library() {
    use super::object_gallery::ObjectPick;
    // A project with no objects, as an imported one.
    let mut model = archisim_core::Model::new("sin objetos");
    model.add_level("PB", 0.0);
    model.add_material(
        "Tierra",
        "earth",
        [150, 120, 90],
        archisim_core::CutPattern::Dots,
    );
    let mut h = Harness::new(Editor::with_model(model, None));
    assert_eq!(h.model().object_types().count(), 0);
    let t = h
        .app
        .object_pick_type(ObjectPick::Library("Árbol de hoja caduca 6 m"))
        .expect("loaded");
    assert_eq!(h.model().object_types().count(), 1);
    assert!(h.model().materials().any(|m| m.name == "Vegetación"));
    // Choosing it again does not load it twice.
    assert_eq!(
        h.app
            .object_pick_type(ObjectPick::Library("Árbol de hoja caduca 6 m")),
        Some(t)
    );
    assert_eq!(h.model().object_types().count(), 1);
    h.app.ed.undo();
    assert_eq!(h.model().object_types().count(), 0);
    let t = h
        .app
        .object_pick_type(ObjectPick::Library("Árbol de hoja caduca 6 m"))
        .unwrap();
    // A sloping terrain: the tree stands on it.
    let sp = |x: f64, y: f64, z: f64| archisim_core::SurveyPoint { at: p(x, y), z };
    h.app
        .ed
        .add_terrain(
            &crate::editor::TerrainSettings::default(),
            vec![sp(0.0, 0.0, 0.0), sp(20.0, 0.0, 2.0), sp(0.0, 20.0, 0.0)],
            Vec::new(),
        )
        .unwrap();
    h.app.object_settings.object_type = Some(t);
    let o = h
        .app
        .ed
        .add_object(&h.app.object_settings, p(5.0, 5.0))
        .unwrap();
    assert!((h.model().object(o).unwrap().base_offset - 0.5).abs() < 1e-9);
    // The gallery and its card draw with the tool.
    h.type_text("o");
    h.frame();
}

/// Importing a topographic survey (R9): a DXF in UTM coordinates with
/// points, contour lines, a breakline, height labels and a building at
/// height 0. The survey's layers are chosen (not the building's), it is
/// centred on the origin, with its heights from the chosen ±0,00.
#[test]
fn survey_import_from_dxf() {
    let path = std::path::PathBuf::from(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../archisim-dxf/tests/fixtures/levantamiento_r12.dxf"
    ));
    let mut h = Harness::new(Editor::new_project());
    h.app.open_survey(&path);
    let s = h.app.survey_import.as_mut().expect("window open");
    let on: Vec<&str> = s
        .layers
        .iter()
        .filter(|(_, on)| *on)
        .map(|(l, _)| l.name.as_str())
        .collect();
    assert_eq!(on, ["CURVAS", "PUNTOS", "RUPTURA"]);
    assert_eq!(s.contour, 1.0);
    // The lowest point is at 637,8: the ±0,00 at 637.
    assert_eq!(s.datum, 637.0);
    s.datum = 640.0;
    h.frame();
    let s = h.app.survey_import.take().unwrap();
    h.app.finish_survey(&s, None);
    let t = h.model().terrains().next().expect("terrain").clone();
    assert_eq!(t.lines.len(), 4);
    assert_eq!(t.contour, 1.0);
    // Centred: the top of the hill (645) at the origin, 5 m over ±0,00.
    let top = t.height_at(p(0.0, 0.0)).unwrap();
    assert!((top - 5.0).abs() < 0.05, "{top}");
    assert!(t
        .points
        .iter()
        .all(|q| q.at.x.abs() < 60.0 && q.at.y.abs() < 60.0));
    // One undoable step.
    h.app.ed.undo();
    assert!(h.model().terrains().next().is_none());
    // A DWG is not read: it says how to make a DXF of it.
    let dwg = std::env::temp_dir().join("archisim-test.dwg");
    std::fs::write(&dwg, b"AC1032").unwrap();
    h.app.open_survey(&dwg);
    assert!(h.app.survey_import.is_none());
    assert!(h.app.message.as_ref().unwrap().text.contains("DXF"));
}

/// One roof over an L-shaped house (R10): the magic wand takes the outer
/// faces of the walls; six planes with a valley; dragging where planes
/// meet changes their pitches.
#[test]
fn l_shaped_roof_and_its_valleys() {
    let mut h = Harness::new(Editor::new_project());
    h.type_text("m");
    for q in [
        p(0.0, 0.0),
        p(10.0, 0.0),
        p(10.0, 4.0),
        p(4.0, 4.0),
        p(4.0, 10.0),
        p(0.0, 10.0),
        p(0.0, 0.0),
    ] {
        h.click_at(q);
    }
    h.key(Key::Escape);
    let pb = h.app.ed.level;
    h.app.ed.add_level("Cubierta", 3.0).unwrap();
    h.app.ed.level = pb;
    h.frame();
    h.key(Key::Escape);
    h.type_text("t");
    assert_eq!(h.app.tool, Tool::Roof);
    h.click_at(p(2.0, 2.0));
    let r = h.model().roofs().next().expect("roof").clone();
    assert!(!r.is_convex());
    assert_eq!(r.planes().len(), 6, "{:?}", h.app.message);
    // The valley meets the ridges inside.
    let nodes = r.nodes();
    assert!(nodes.iter().any(|(_, sides)| sides.len() >= 3));
    let (at, sides) = nodes[0].clone();
    let before: Vec<Option<f64>> = sides.iter().map(|&i| r.slopes[i]).collect();
    h.key(Key::Escape);
    h.app.ed.selection = [Element::Roof(r.id)].into_iter().collect();
    h.frame();
    h.drag(at, at + archisim_core::Vec2::new(0.4, 0.0));
    let after = h.model().roof(r.id).unwrap().clone();
    let now: Vec<Option<f64>> = sides.iter().map(|&i| after.slopes[i]).collect();
    assert_ne!(before, now, "{:?}", h.app.message);
    assert!(after
        .nodes()
        .iter()
        .any(|(q, _)| q.distance(at + archisim_core::Vec2::new(0.4, 0.0)) < 1e-6));
    // In plan, 3D and the IFC.
    let solids = archisim_geom::solid::model_solids(h.model());
    assert!(
        solids
            .iter()
            .filter(|s| s.owner == archisim_geom::solid::SolidOwner::Roof(r.id))
            .count()
            >= 6
    );
}

/// Curtain wall types (R11): the tool draws with a type of the library;
/// the types window opens on the selected wall's type with its length and
/// height as the sample, shows it in every view, and a change to the type
/// (an opaque spandrel row, caps) is one undoable step every wall of the
/// type follows.
#[test]
fn curtain_wall_types_window() {
    use super::curtain_types_ui::CurtainView;
    use archisim_core::{CurtainCaps, CurtainDivision, CurtainPieceKind, CurtainRest};
    let mut h = Harness::new(Editor::new_project());
    h.app.set_tool(Tool::CurtainWall);
    h.frame();
    h.click_at(Point2::new(0.0, 0.0));
    h.click_at(Point2::new(4.8, 0.0));
    let id = h.model().curtain_walls().next().expect("drawn").id;
    let t = h.model().curtain_wall(id).unwrap().curtain_type;
    assert_eq!(
        h.model().curtain_type(t).unwrap().name,
        "Montante y travesaño 120 × 150"
    );
    h.app.set_tool(Tool::Arrow);
    h.app.ed.selection = [Element::CurtainWall(id)].into_iter().collect();
    h.frame();
    h.app.open_curtain_types(Some(t), Some((4.8, 2.7)));
    for view in [
        CurtainView::Elevation,
        CurtainView::Plan,
        CurtainView::Section,
        CurtainView::ThreeD,
    ] {
        h.app.curtain_types.view = view;
        h.frame();
    }
    assert!(h.app.curtain_types.open);
    assert_eq!(h.app.curtain_types.selected, Some(t));
    // A spandrel row and caps.
    let opaque = h
        .model()
        .materials()
        .find(|m| m.name == "Panel sándwich lacado")
        .unwrap()
        .id;
    let mut d = h.model().curtain_type(t).unwrap().design.clone();
    d.rows = CurtainDivision {
        fixed: vec![0.9],
        rest: CurtainRest::MaxSpacing(2.0),
    };
    d.panels.push(archisim_core::CurtainPanel {
        name: "Antepecho".into(),
        thickness: 0.05,
        material: opaque,
    });
    d.row_panels = vec![1, 0];
    d.caps = Some(CurtainCaps {
        width: 0.06,
        depth: 0.02,
        material: d.mullion.material,
    });
    let name = h.model().curtain_type(t).unwrap().name.clone();
    h.app
        .ed
        .edit_curtain_type("Antepecho", t, &name, d)
        .unwrap();
    h.app.curtain_types.view = CurtainView::Elevation;
    h.frame();
    let pieces = h.model().curtain_wall_pieces(id);
    assert!(pieces.iter().any(|p| p.kind == CurtainPieceKind::Panel(1)));
    assert!(pieces.iter().any(|p| p.kind == CurtainPieceKind::Cap));
    h.app.ed.undo();
    h.frame();
    let pieces = h.model().curtain_wall_pieces(id);
    assert!(!pieces.iter().any(|p| p.kind == CurtainPieceKind::Cap));
    // A type in use cannot be deleted; a duplicate can.
    assert!(h.app.ed.remove_curtain_type(t).is_err());
    let copy = h.app.ed.duplicate_curtain_type(t).unwrap();
    h.app.curtain_types.selected = Some(copy);
    h.frame();
    h.app.ed.remove_curtain_type(copy).unwrap();
    h.frame();
    assert_eq!(h.app.curtain_types.selected, Some(t));
    if let Ok(dir) = std::env::var("ARCHISIM_TEST_OUT") {
        let dir = std::path::PathBuf::from(dir);
        std::fs::create_dir_all(&dir).unwrap();
        archisim_file::save(h.model(), dir.join("muro_cortina.archisim")).unwrap();
    }
}

// ---- 2D drafting with the command line (AutoCAD) -----------------------

/// Types a command or value and presses Enter, as in AutoCAD.
fn command(h: &mut Harness, text: &str) {
    h.type_text(text);
    h.key(Key::Enter);
}

fn drafts(h: &Harness) -> Vec<archisim_core::Draft> {
    h.model().drafts().cloned().collect()
}

#[test]
fn lines_rectangles_and_typed_coordinates() {
    let mut h = Harness::new(Editor::new_project());
    h.app.keys_autocad = true;
    // LINEA with a click, relative and polar coordinates, and Cerrar.
    command(&mut h, "l");
    assert_eq!(h.app.tool, Tool::Cad);
    h.click_at(p(0.0, 0.0));
    command(&mut h, "@3000,0");
    command(&mut h, "@2000<90");
    command(&mut h, "c");
    let d = drafts(&h);
    assert_eq!(d.len(), 3, "{d:?}");
    assert_eq!(h.app.tool, Tool::Arrow);
    assert!(d
        .iter()
        .any(|x| matches!(x.shape, archisim_core::Shape::Line { a, b }
        if a.approx_eq(p(3.0, 0.0)) && b.approx_eq(p(3.0, 2.0)))));
    // Enter on an empty line repeats LINEA; Esc keeps what was drawn.
    h.key(Key::Enter);
    assert_eq!(h.app.cad.as_ref().map(|c| c.name()), Some("LINEA"));
    command(&mut h, "10000,0");
    command(&mut h, "12000,0");
    h.key(Key::Escape);
    assert_eq!(drafts(&h).len(), 4);
    assert!(h.app.cad.is_none());
    // A rectangle by its size.
    command(&mut h, "rec");
    command(&mut h, "0,5000");
    command(&mut h, "@4000,3000");
    let rect = drafts(&h).last().unwrap().shape.clone();
    assert!((archisim_draft::area(&rect).unwrap() - 12.0).abs() < 1e-9);
    // An unknown command is said, nothing changes.
    command(&mut h, "zzz");
    assert!(h.app.cmdline.history.last().unwrap().1);
    // M + Enter takes the wall tool.
    command(&mut h, "m");
    assert_eq!(h.app.tool, Tool::Wall);
    h.key(Key::Escape);
    // Undo removes the rectangle.
    h.set_modifiers(Modifiers::COMMAND);
    h.key(Key::Z);
    h.set_modifiers(Modifiers::NONE);
    assert_eq!(drafts(&h).len(), 4);
}

/// ArchiCAD's vertices: a double click on an edge of a selected fill or
/// line adds a vertex there, which drags like any other; a click on it
/// picks it and Supr takes it away (not the whole fill).
/// A 3D model from SketchUp (COLLADA, in inches) and a chair from a
/// catalogue (OBJ with its texture) brought in with File → Importar modelo
/// 3D: the dialog reads them, the object tool takes the new type, a click
/// places it; the 3D view shows its triangles with the colours of its
/// vertices, the plan its outline, and the trees of the library keep their
/// own drawing in a technical view.
#[test]
fn imported_models_are_placed_and_drawn() {
    let mut h = Harness::new(Editor::new_project());
    let fixtures =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../archisim-mesh/tests/fixtures");
    let level = h.app.ed.level;
    // Dropped on the window: the dialog, with what it read.
    h.app.open_files(vec![fixtures.join("arbol.dae")]);
    let imp = h.app.mesh_import.as_ref().expect("the dialog");
    assert_eq!(imp.name, "arbol");
    let mesh = imp.read.as_ref().unwrap();
    let (lo, hi) = mesh.bounds();
    assert!((hi[2] - lo[2] - 4.0).abs() < 1e-3, "inches read as such");
    h.app.mesh_import.as_mut().unwrap().kind = archisim_library::imported::ModelKind::Vegetation;
    h.frame();
    h.app.import_mesh_now();
    assert_eq!(h.app.tool, Tool::Object);
    let t = h.app.object_settings.object_type.unwrap();
    assert!(h.model().object_type(t).unwrap().mesh.is_some());
    h.click_at(p(3.0, 3.0));
    let o = h.model().objects().next().expect("placed").id;
    // In 3D: the model's own triangles, coloured vertex by vertex.
    let solids = archisim_geom::solid::object_solids(h.model(), o);
    let tris: usize = solids.iter().map(|s| s.triangles.len()).sum();
    assert!(tris > 300, "{tris}");
    let scene = crate::view3d::scene::Scene::build(h.model(), &BTreeSet::new());
    assert!(scene.opaque.len() >= tris * 3);
    // In plan: its outline (the canopy, about 2.5 m across), drawn with
    // its colour as the trees are.
    let svg = archisim_plan::render_plan(h.model(), level, &h.app.plan_export_options(50.0));
    assert!(svg.contains("fill-opacity"));
    let outline = h.model().object_outline(o);
    assert!((outline[0].distance(outline[1]) - 2.5).abs() < 0.05);
    // In the IFC: its triangles, once in the type.
    let ifc = archisim_ifc::export_ifc(h.model(), &Default::default());
    assert_eq!(ifc.matches("IFCREPRESENTATIONMAP(").count(), 1);
    assert!(ifc.contains("IFCTRIANGULATEDFACESET("));
    // Saved and opened, the model comes back.
    let text = archisim_file::to_string(h.model());
    let back = archisim_file::from_str(&text).unwrap();
    assert_eq!(back.meshes().count(), 1);
    // The chair with its texture: wood colours at its vertices.
    h.key(Key::Escape);
    h.app.open_files(vec![fixtures.join("silla.obj")]);
    h.app.import_mesh_now();
    let chair = h.app.object_settings.object_type.unwrap();
    let m = h.model();
    let mesh = m.mesh(m.object_type(chair).unwrap().mesh.unwrap()).unwrap();
    let c = mesh.levels[0].colors[0];
    assert!(c[0] > c[2] + 40, "{c:?}");
    assert_eq!(
        m.family(m.object_type(chair).unwrap().family)
            .unwrap()
            .def()
            .category,
        "furniture"
    );
}

#[test]
fn vertices_added_dragged_and_removed() {
    let mut h = Harness::new(Editor::new_project());
    h.app.keys_autocad = true;
    command(&mut h, "rellrect");
    command(&mut h, "0,0");
    command(&mut h, "4000,3000");
    let fill = drafts(&h)[0].clone();
    let area = |h: &Harness| archisim_draft::area(&drafts(h)[0].shape).unwrap();
    assert!((area(&h) - 12.0).abs() < 1e-9);
    h.key(Key::Escape);
    h.click_at(p(2.0, 1.0));
    assert!(h.app.ed.selection.contains(&Element::Draft(fill.id)));
    // Double click on the top edge: a fifth corner, the area unchanged.
    h.move_to(p(2.0, 3.0));
    for _ in 0..2 {
        h.button(PointerButton::Primary, true);
        h.button(PointerButton::Primary, false);
    }
    h.frame();
    let corners = |h: &Harness| match &drafts(h)[0].shape {
        archisim_core::Shape::Hatch(x) => x.loops[0].points.len(),
        _ => 0,
    };
    assert_eq!(corners(&h), 5);
    assert!((area(&h) - 12.0).abs() < 1e-9);
    // Dragged up a metre: a gable, two square metres more.
    h.time += 1.0;
    h.drag(p(2.0, 3.0), p(2.0, 4.0));
    assert!((area(&h) - 14.0).abs() < 1e-6, "{}", area(&h));
    // Picked with a click and taken away with Supr: back to the rectangle,
    // still there and still selected.
    h.click_at(p(2.0, 4.0));
    assert!(h.app.vertex.is_some());
    h.key(Key::Delete);
    assert_eq!(drafts(&h).len(), 1);
    assert_eq!(corners(&h), 4);
    assert!((area(&h) - 12.0).abs() < 1e-9);
    // With no vertex picked, Supr deletes the fill as before.
    h.click_at(p(2.0, 1.0));
    h.key(Key::Delete);
    assert!(drafts(&h).is_empty());
}

#[test]
fn offset_trim_hatch_and_move() {
    let mut h = Harness::new(Editor::new_project());
    h.app.keys_autocad = true;
    command(&mut h, "rec");
    command(&mut h, "0,0");
    command(&mut h, "4000,3000");
    // EQUIDIST 500 outwards.
    command(&mut h, "eq");
    command(&mut h, "500");
    h.click_at(p(2.0, 0.0));
    h.click_at(p(2.0, -2.0));
    h.key(Key::Enter);
    let d = drafts(&h);
    assert_eq!(d.len(), 2);
    assert!((archisim_draft::area(&d[1].shape).unwrap() - 20.0).abs() < 1e-9);
    // A line across both, trimmed between them on the right.
    command(&mut h, "l");
    command(&mut h, "-2000,1500");
    command(&mut h, "6000,1500");
    h.key(Key::Enter);
    command(&mut h, "rr");
    h.click_at(p(4.25, 1.5));
    h.key(Key::Enter);
    let line = drafts(&h)
        .into_iter()
        .filter(|x| matches!(x.shape, archisim_core::Shape::Line { .. }))
        .count();
    // Cut in two: left part and right part.
    assert_eq!(line, 2);
    // A hatch with a click inside the inner rectangle (it has the line
    // pieces through it: the lower half).
    command(&mut h, "sb");
    h.click_at(p(1.0, 1.0));
    h.key(Key::Enter);
    let hatch = drafts(&h)
        .into_iter()
        .find(|x| matches!(x.shape, archisim_core::Shape::Hatch(_)))
        .expect("a hatch");
    let a = archisim_draft::area(&hatch.shape).unwrap();
    assert!((a - 6.0).abs() < 1e-6, "{a}");
    // Move everything 1 m right: window from right to left (crossing).
    h.drag(p(7.0, 4.0), p(-1.5, -1.0));
    assert_eq!(h.app.ed.selection.len(), drafts(&h).len());
    command(&mut h, "d");
    h.click_at(p(0.0, 0.0));
    command(&mut h, "@1000,0");
    assert!(h.app.ed.selection.is_empty());
    let moved = drafts(&h)
        .into_iter()
        .find(|x| matches!(x.shape, archisim_core::Shape::Hatch(_)))
        .unwrap();
    let (lo, _) = archisim_draft::bounds(&moved.shape, 0.05);
    assert!((lo.x - 1.0).abs() < 1e-9, "{lo:?}");
}

#[test]
fn texts_layers_and_drafting_properties() {
    let mut h = Harness::new(Editor::new_project());
    h.app.keys_autocad = true;
    // A layer for texts, made current.
    let textos = h
        .app
        .ed
        .add_draw_layer(archisim_core::DrawLayer::new(
            "Textos",
            [0, 0, 200],
            0.25,
            archisim_core::LineStyle::Continuous,
        ))
        .unwrap();
    h.app.cad_settings.layer = textos;
    command(&mut h, "tx");
    h.click_at(p(1.0, 1.0));
    command(&mut h, "3,5");
    h.key(Key::Enter);
    assert_eq!(h.app.cad_want(), Some(crate::cad::Want::Text));
    h.frame();
    h.app
        .cad_feed(crate::cad::Input::Value("Salón\n25,40 m²".into()));
    h.key(Key::Escape);
    let t = drafts(&h).pop().expect("a text");
    assert_eq!(t.layer, textos);
    let archisim_core::Shape::Text(text) = &t.shape else {
        panic!()
    };
    assert_eq!(text.height, 3.5);
    assert_eq!(text.content.lines().count(), 2);
    // Selected, the palette shows it; the dialog of layers opens with CAPA.
    h.app.ed.selection = [Element::Draft(t.id)].into_iter().collect();
    h.frame();
    command(&mut h, "capa");
    assert!(h.app.show_layers);
    h.frame();
    // Hidden layer: its texts are not picked.
    let mut l = h.model().draw_layer(textos).unwrap().clone();
    l.visible = false;
    h.app.ed.set_draw_layer(l).unwrap();
    h.app.ed.selection.clear();
    assert!(crate::pick::elements_on_level(h.model(), h.app.ed.level)
        .iter()
        .all(|e| !matches!(e, Element::Draft(_))));
    // Every command starts and is left with Esc without trouble.
    for (name, _) in crate::cad::ALIASES {
        if let crate::cad::Launch::Command(c) = crate::cad::launch(name).unwrap() {
            h.app.start_command(c);
            h.move_to(p(2.0, 2.0));
            h.frame();
            h.key(Key::Escape);
            assert!(h.app.cad.is_none(), "{c}");
        }
    }
}

#[test]
fn sheets_views_and_drafting_on_paper() {
    let mut h = Harness::new(Editor::new_project());
    h.frame();
    draw_outline(&mut h);
    h.key(Key::Escape);
    h.app.keys_autocad = true;
    let level = h.app.ed.level;
    // A new sheet opens in its tab, as paper fitted to the window.
    h.app.new_sheet();
    h.frame();
    h.frame();
    let sheet = h.model().sheets()[0].id;
    assert_eq!(h.app.mode, ViewMode::Sheet(sheet));
    assert_eq!(h.model().sheets()[0].number, "A-01");
    let paper = h.screen(p(0.841, 0.594)) - h.screen(p(0.0, 0.0));
    assert!(paper.x > 500.0, "{paper:?}");
    // A view of the level at 1:100, then moved by dragging it.
    let v = h
        .app
        .ed
        .add_viewport(archisim_core::NewViewport {
            sheet,
            level,
            section: None,
            axo: None,
            scale: 100.0,
            cut_plane: 1.0,
            crop: None,
            centre: p(0.3, 0.3),
        })
        .unwrap();
    h.frame();
    h.drag(p(0.3, 0.3), p(0.35, 0.32));
    let c = h.model().viewport(v).unwrap().centre;
    assert!(c.distance(p(0.35, 0.32)) < 0.002, "{c:?}");
    assert_eq!(h.app.sheet_sel, Some(super::sheet_ui::Placed::View(v)));
    // A line drawn on the paper belongs to the sheet, in paper metres.
    command(&mut h, "l");
    assert_eq!(h.app.mode, ViewMode::Sheet(sheet));
    h.click_at(p(0.62, 0.40));
    command(&mut h, "@100,0");
    h.key(Key::Escape);
    let line = drafts(&h).pop().expect("a line");
    assert_eq!(line.view, archisim_core::ViewRef::Sheet(sheet));
    let archisim_core::Shape::Line { a, b } = line.shape else {
        panic!("{:?}", line.shape)
    };
    assert!((b.x - a.x - 0.1).abs() < 1e-9, "{a:?} {b:?}");
    // Picked on the sheet and deleted with Supr.
    h.click_at(p(0.67, 0.40));
    assert_eq!(
        h.app.ed.selection.iter().next(),
        Some(&Element::Draft(line.id))
    );
    h.key(Key::Delete);
    assert!(h.model().draft(line.id).is_none());
    // Supr on a selected view takes it off the sheet; double click on it
    // opens its plan.
    h.click_at(p(0.35, 0.32));
    assert_eq!(h.app.sheet_sel, Some(super::sheet_ui::Placed::View(v)));
    h.move_to(p(0.35, 0.32));
    h.button(PointerButton::Primary, true);
    h.button(PointerButton::Primary, false);
    h.button(PointerButton::Primary, true);
    h.button(PointerButton::Primary, false);
    h.frame();
    assert_eq!(h.app.mode, ViewMode::Plan);
    assert_eq!(h.app.ed.level, level);
    // Back to the sheet from its tab; its framing was kept.
    h.app.open_view(TabKind::Sheet(sheet));
    h.frame();
    h.click_at(p(0.35, 0.32));
    h.key(Key::Delete);
    assert!(h.model().viewport(v).is_none());
    // Deleting the sheet closes its tab.
    h.app.ed.remove_sheet(sheet).unwrap();
    h.frame();
    h.frame();
    assert_eq!(h.app.mode, ViewMode::Plan);
    assert!(h.app.tabs.iter().all(|t| t.kind != TabKind::Sheet(sheet)));
}

#[test]
fn dxf_out_and_back_in_and_pdf_of_the_sheets() {
    let mut h = Harness::new(Editor::new_project());
    h.frame();
    h.app.keys_autocad = true;
    command(&mut h, "l");
    h.click_at(p(0.0, 0.0));
    command(&mut h, "@4000,0");
    h.key(Key::Escape);
    let dir = std::env::temp_dir().join(format!("archisim-dxf-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let dxf = dir.join("planta.dxf");
    let options = h.app.plan_export_options(50.0);
    h.app.ed.export_plan(&dxf, &options).unwrap();
    // Into a sheet: the line comes back at paper size (4 m → 4000 mm of
    // paper would be huge, but it is what the DXF says).
    h.app.new_sheet();
    h.frame();
    let sheet = h.model().sheets()[0].id;
    h.app.import_dxf_from(&dxf);
    // The line; the north arrow of the plan comes on its own layer.
    let on_sheet: Vec<_> = h
        .model()
        .drafts_in(archisim_core::ViewRef::Sheet(sheet))
        .filter(|d| {
            h.model()
                .draw_layer(d.layer)
                .is_none_or(|l| l.name != "A-NORTE")
        })
        .cloned()
        .collect();
    assert_eq!(
        on_sheet.len(),
        1,
        "{:?}",
        h.app.message.as_ref().map(|m| &m.text)
    );
    assert_eq!(h.app.ed.undo_label(), Some("Importar DXF «planta.dxf»"));
    // Every sheet in one PDF.
    let pdf = dir.join("planos.pdf");
    h.app.ed.export_sheets(&pdf, &[sheet]).unwrap();
    assert!(std::fs::read(&pdf).unwrap().starts_with(b"%PDF"));
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn angular_radius_and_arc_dimensions() {
    let mut h = Harness::new(Editor::new_project());
    h.frame();
    h.app.keys_autocad = true;
    for second in ["@4000,0", "@0,3000"] {
        command(&mut h, "l");
        h.click_at(p(0.0, 0.0));
        command(&mut h, second);
        h.key(Key::Escape);
    }
    command(&mut h, "dan");
    h.click_at(p(2.0, 0.0));
    h.click_at(p(0.0, 1.5));
    h.click_at(p(1.0, 1.0));
    let dims: Vec<archisim_core::Measure> = drafts(&h)
        .iter()
        .filter_map(|d| match d.shape {
            archisim_core::Shape::Dim(m) => Some(m),
            _ => None,
        })
        .collect();
    assert_eq!(dims.len(), 1, "{:?}", h.app.cmdline.history);
    assert_eq!(archisim_draft::dims::label(&dims[0]), "90°");
    // A circle of 1,25 m: its radius and diameter.
    command(&mut h, "c");
    h.click_at(p(8.0, 2.0));
    command(&mut h, "1250");
    for (cmd, text) in [("dra", "R 1,25"), ("ddi", "Ø 2,50")] {
        command(&mut h, cmd);
        h.click_at(p(9.25, 2.0));
        h.click_at(p(10.5, 3.0));
        let last = drafts(&h).pop().unwrap();
        let archisim_core::Shape::Dim(m) = last.shape else {
            panic!("{:?}", last.shape)
        };
        assert_eq!(archisim_draft::dims::label(&m), text);
    }
    // Scaled ×2, the diameter dimension reads twice as much.
    let id = drafts(&h).pop().unwrap().id;
    h.app.ed.selection = [Element::Draft(id)].into_iter().collect();
    command(&mut h, "es");
    command(&mut h, "8000,2000");
    command(&mut h, "2");
    let archisim_core::Shape::Dim(m) = h.model().draft(id).unwrap().shape else {
        panic!()
    };
    assert_eq!(archisim_draft::dims::label(&m), "Ø 5,00");
}

#[test]
fn rooms_follow_walls_and_tags_follow_doors() {
    let (m, lvl, _) = crate::pick::tests::room();
    let mut h = Harness::new(Editor::with_model(m, None));
    h.app.ed.level = lvl;
    h.frame();
    h.app.keys_autocad = true;
    command(&mut h, "loc");
    h.click_at(p(3.0, 2.0));
    h.key(Key::Escape);
    let r = h.model().rooms().next().cloned().expect("a room");
    assert_eq!(r.number, "1");
    assert!(r.bounded);
    let before = r.area();
    assert!(before > 15.0 && before < 24.0, "{before}");
    // Every door and window tagged at once.
    command(&mut h, "eti");
    command(&mut h, "t");
    assert_eq!(h.model().tags().count(), 1);
    // The door slides half a metre: its tag goes with it.
    let tag_before = h.model().tags().next().unwrap().at;
    let door = h.model().openings().next().unwrap().clone();
    h.app
        .ed
        .move_opening(door.id, door.along + 0.5, door.sill)
        .unwrap();
    let tag_after = h.model().tags().next().unwrap().at;
    assert!(
        (tag_after.distance(tag_before) - 0.5).abs() < 1e-9,
        "{tag_before:?} → {tag_after:?}"
    );
    // The east wall moves out a metre: the room grows with it.
    let east = h
        .model()
        .walls()
        .find(|w| w.start.x > 5.9 && w.end.x > 5.9)
        .unwrap()
        .id;
    h.app.ed.selection = [Element::Wall(east)].into_iter().collect();
    let drag = crate::drag::Drag {
        grip: crate::drag::Grip::Body,
        from: p(6.0, 2.0),
        to: p(7.0, 2.0),
        variant: false,
    };
    h.app.ed.drag(&drag).unwrap();
    let after = h.model().room(r.id).unwrap().area();
    assert!(after > before + 3.0, "{before} → {after}");
    // The door goes: its tag too.
    let door = h.model().openings().next().unwrap().id;
    h.app.ed.selection = [Element::Opening(door)].into_iter().collect();
    h.app.ed.delete_selection().unwrap();
    assert_eq!(h.model().tags().count(), 0);
    // Picked by its stamp, its name changed in the palette.
    h.app.ed.selection.clear();
    h.click_at(r.label);
    assert_eq!(h.app.ed.selection.iter().next(), Some(&Element::Room(r.id)));
    let mut renamed = h.model().room(r.id).unwrap().clone();
    renamed.name = "Salón".into();
    h.app.ed.set_room(renamed, "Cambiar local").unwrap();
    h.frame();
    // In the IFC, as a space with its quantities.
    let ifc = archisim_ifc::export_ifc(h.model(), &Default::default());
    assert!(ifc.contains("IFCSPACE("), "no space");
    assert!(ifc.contains("'Salón'") || ifc.contains("Sal\\X2\\00F3\\X0\\n"));
    assert!(ifc.contains("'Qto_SpaceBaseQuantities'"));
}

#[test]
fn schedules_on_sheets_and_bc3() {
    let (m, lvl, _) = crate::pick::tests::room();
    let mut h = Harness::new(Editor::with_model(m, None));
    h.app.ed.level = lvl;
    h.frame();
    h.app.new_sheet();
    h.frame();
    let sheet = h.model().sheets()[0].id;
    // Dropped from the navigator onto the sheet shown.
    use super::sheet_drop::Drop;
    h.app.place_dropped(
        Drop::Table(archisim_core::ScheduleKind::Walls),
        "Muros",
        p(0.1, 0.5),
    );
    h.frame();
    let t = h.model().tables_on(sheet).next().unwrap().id;
    assert_eq!(h.model().table(t).unwrap().at, p(0.1, 0.5));
    let table =
        archisim_plan::schedules::table(h.model(), archisim_core::ScheduleKind::Walls, None);
    assert_eq!(table.rows.len(), 1);
    // Dragged by its middle.
    let (lo, hi) = super::sheet_ui::table_rect(h.model(), h.model().table(t).unwrap());
    let mid = (lo + hi) * 0.5;
    h.drag(mid, mid + p(0.05, -0.02));
    let at = h.model().table(t).unwrap().at;
    assert!(at.distance(p(0.15, 0.48)) < 0.002, "{at:?}");
    assert_eq!(h.app.sheet_sel, Some(super::sheet_ui::Placed::Table(t)));
    h.key(Key::Delete);
    assert!(h.model().table(t).is_none());
    // A plan at the scale of its tab, and the 3D view as an axonometric.
    h.app.place_dropped(Drop::Level(lvl), "Planta", p(0.2, 0.2));
    let plan = h.model().viewports_on(sheet).next().unwrap().clone();
    assert_eq!(plan.scale, h.app.tabs[0].scale);
    assert!(plan.axo.is_none());
    h.app
        .place_dropped(Drop::ThreeD, "La vista 3D", p(0.4, 0.2));
    let axo = h
        .model()
        .viewports_on(sheet)
        .find(|v| v.axo.is_some())
        .unwrap()
        .clone();
    // At a scale that fits the paper.
    assert!([50.0, 100.0, 200.0, 250.0, 500.0, 1000.0].contains(&axo.scale));
    assert!(axo.crop.is_some());
    h.frame();
    let svg = archisim_plan::sheet::render_sheet(h.model(), sheet, &Default::default());
    assert!(svg.contains("class=\"axo\""));
    assert_eq!(
        archisim_plan::sheet::viewport_title(h.model(), &axo),
        "VISTA 3D"
    );
    // Schedules and measurements in tabs of their own.
    use super::tabs::{SchedTab, TabKind};
    h.app.pick(super::toolbox::Pick::Action(
        super::toolbox::Action::Takeoff,
    ));
    h.frame();
    assert_eq!(h.app.mode, ViewMode::Schedule);
    assert_eq!(
        h.app.tabs[h.app.active_tab].kind,
        TabKind::Schedule(SchedTab::Takeoff)
    );
    h.app.open_view(TabKind::Schedule(SchedTab::Kind(
        archisim_core::ScheduleKind::Walls,
    )));
    h.frame();
    assert_eq!(h.app.tab_title(h.app.tabs[h.app.active_tab].kind), "Muros");
    // The budget.
    let dir = std::env::temp_dir().join(format!("archisim-bc3-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    h.app.ed.export_bc3(&dir.join("m.bc3")).unwrap();
    let text = std::fs::read(dir.join("m.bc3")).unwrap();
    assert!(text.starts_with(b"~V|ArchiSim|FIEBDC-3/2016"));
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn escape_then_a_new_command_in_the_same_instant() {
    let mut h = Harness::new(Editor::new_project());
    h.frame();
    h.app.keys_autocad = true;
    command(&mut h, "l");
    h.click_at(p(0.0, 0.0));
    // Esc and «c» arrive together: the line ends and CIRCULO starts.
    h.events.push(Event::Key {
        key: Key::Escape,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers: Modifiers::NONE,
    });
    h.events.push(Event::Text("c".into()));
    h.frame();
    h.key(Key::Enter);
    assert_eq!(h.app.cad.as_ref().map(|c| c.name()), Some("CIRCULO"));
}

/// Sections and elevations on sheets: placed from the sheet's panel with
/// their crop worked out once, drawn on the sheet, opened with a double
/// click; and exported on their own to PDF, DXF and SVG.
#[test]
fn sections_on_sheets_and_exported() {
    let mut h = Harness::new(Editor::new_project());
    draw_outline(&mut h);
    h.key(Key::Escape);
    h.key(Key::Escape);
    h.type_text("s");
    h.click_at(p(-0.8, 4.0));
    h.click_at(p(12.8, 4.0));
    h.key(Key::Escape);
    let sec = h.model().sections().next().unwrap().id;
    h.app.new_sheet();
    h.frame();
    let sheet = h.model().sheets()[0].id;
    let v = h
        .app
        .place_view(
            sheet,
            super::sheet_ui::ViewChoice::Section(sec),
            100.0,
            Point2::new(0.2, 0.2),
        )
        .unwrap();
    let vp = h.model().viewport(v).unwrap().clone();
    assert_eq!(vp.section, Some(sec));
    let (lo, hi) = vp.crop.expect("a section view gets its crop");
    // The 12 m of the outline, and the level labels on the right.
    assert!(hi.x - lo.x > 12.0, "{lo:?} {hi:?}");
    assert_eq!(
        archisim_plan::sheet::viewport_title(h.model(), &vp),
        "SECCIÓN A"
    );
    let svg = archisim_plan::sheet::render_sheet(h.model(), sheet, &Default::default());
    assert!(
        svg.contains("id=\"cut\""),
        "the section is drawn on the sheet"
    );
    h.frame();
    // Exported on its own.
    let dir = std::env::temp_dir().join(format!("archisim-sec-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let options = h.app.section_export_options(50.0);
    for ext in ["pdf", "dxf", "svg"] {
        let path = dir.join(format!("seccion.{ext}"));
        h.app.ed.export_section(&path, sec, &options).unwrap();
        let bytes = std::fs::read(&path).unwrap();
        match ext {
            "pdf" => assert!(bytes.starts_with(b"%PDF")),
            "dxf" => assert!(String::from_utf8_lossy(&bytes).contains("AC1021")),
            _ => assert!(String::from_utf8_lossy(&bytes).contains("<svg")),
        }
    }
    // Deleting the section takes its view off the sheet.
    h.app.ed.selection = [Element::Section(sec)].into_iter().collect();
    h.app.ed.delete_selection().unwrap();
    assert_eq!(h.model().viewports().count(), 0);
    let _ = std::fs::remove_dir_all(&dir);
}

/// The project's properties show where it is on the map; set, the IFC
/// carries it.
#[test]
fn georeference_in_the_project() {
    let mut h = Harness::new(Editor::new_project());
    h.frame();
    h.app
        .ed
        .set_georeference(Some(archisim_core::Georeference {
            crs: "EPSG:25829".into(),
            eastings: 680_000.0,
            northings: 4_119_000.0,
            height: 3.0,
        }))
        .unwrap();
    // Nothing selected: the project's properties, with the map group.
    h.app.ed.selection.clear();
    h.frame();
    h.frame();
    let dir = std::env::temp_dir().join(format!("archisim-geo-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("geo.ifc");
    h.app
        .ed
        .export_ifc(&path, archisim_ifc::Schema::Ifc4x3)
        .unwrap();
    let text = std::fs::read_to_string(&path).unwrap();
    assert!(text.contains("IFCMAPCONVERSION("));
    h.app.ed.undo();
    assert!(h.model().georeference().is_none());
    let _ = std::fs::remove_dir_all(&dir);
}

/// The library window: a door type saved from the project into a library
/// folder, loaded into another project, and the question when the project
/// has the same type with other values.
#[test]
fn library_window_loads_types() {
    use crate::editor::library::{save_type, scan};
    use archisim_core::{FamilyTypeRef, OnConflict};
    let dir = std::env::temp_dir().join(format!("archisim-libui-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let a = Harness::new(Editor::new_project());
    let t = a.model().opening_types().find(|t| t.is_door()).unwrap().id;
    let (entry, version) = a.app.ed.catalog_entry(FamilyTypeRef::Opening(t)).unwrap();
    save_type(&scan(&dir), &entry, version).unwrap();
    let guid = a.model().opening_type(t).unwrap().guid.clone();
    // Another project: the window lists it and loads it.
    let mut b = Harness::new(Editor::new_project());
    b.app.library.set(scan(&dir));
    b.app.library.open = true;
    b.frame();
    // Every project's types have their own GlobalIds: this is a new one,
    // with a free name; loading it again changes nothing.
    let before = b.model().opening_types().count() + 1;
    b.app.load_from_library(0, 0, OnConflict::Ask);
    assert_eq!(b.model().opening_types().count(), before);
    b.app.load_from_library(0, 0, OnConflict::Ask);
    assert_eq!(b.model().opening_types().count(), before);
    // Changed in the project: the window asks.
    let bt = b
        .model()
        .opening_types()
        .find(|x| x.guid == guid)
        .unwrap()
        .id;
    b.app
        .ed
        .transact("w", |m| {
            let ty = m.opening_type(bt).unwrap().clone();
            let mut v = ty.values.clone();
            v.insert("width".into(), archisim_core::Value::Number(1.234));
            m.edit_family_type(bt, ty.family, v)
        })
        .unwrap();
    b.app.load_from_library(0, 0, OnConflict::Ask);
    assert_eq!(b.app.library.conflict(), Some((0, 0)));
    b.frame();
    b.app.load_from_library(0, 0, OnConflict::Rename);
    assert_eq!(b.model().opening_types().count(), before + 1);
    let _ = std::fs::remove_dir_all(&dir);
}

/// The code check window: the building's use is set there (one undoable
/// step), and a clic on a result selects its element in its plan.
#[test]
fn cte_window_checks_the_model() {
    use archisim_cte::{Status, Subject};
    let mut h = Harness::new(Editor::new_project());
    let level = h.app.ed.level;
    let room = h
        .app
        .ed
        .transact("r", |m| {
            m.add_room(archisim_core::NewRoom {
                level,
                number: "1".into(),
                name: "Oficina".into(),
                outline: vec![
                    Point2::new(0.0, 0.0),
                    Point2::new(6.0, 0.0),
                    Point2::new(6.0, 4.0),
                    Point2::new(0.0, 4.0),
                ],
                holes: Vec::new(),
                label: Point2::new(3.0, 2.0),
                bounded: false,
            })
        })
        .unwrap();
    h.app
        .ed
        .set_code_settings(archisim_core::CodeSettings {
            building_use: archisim_core::BuildingUse::Administrative,
            ..Default::default()
        })
        .unwrap();
    assert_eq!(h.app.ed.undo_label(), Some("Uso del edificio (CTE)"));
    h.app.cte.open = true;
    h.frame();
    // A room with no door: it cannot be left.
    let report = h.app.cte.current(&h.app.ed);
    let fail = report
        .items
        .iter()
        .find(|i| i.status == Status::Fails && i.subject == Subject::Room(room))
        .expect("no way out")
        .clone();
    h.app.cte_pick(&fail);
    assert!(h.app.ed.selection.contains(&Element::Room(room)));
    h.frame();
    // Its occupancy, fixed in its properties.
    let mut r = h.model().room(room).unwrap().clone();
    r.occupancy = Some(0.0);
    h.app.ed.set_room(r, "o").unwrap();
    let report = h.app.cte.current(&h.app.ed);
    assert_eq!(report.levels[0].people, 0);
}

/// Grid lines by two clicks, named 1, 2… and A, B…; a pad where it is
/// clicked, a strip under the wall clicked, and the rest of the walls
/// founded at once from the contextual bar.
#[test]
fn grids_and_footings_are_drawn() {
    use archisim_core::FootingKind;
    let mut h = Harness::new(Editor::new_project());
    draw_outline(&mut h);
    h.key(Key::Escape);
    h.key(Key::Escape);
    h.type_text("x");
    assert_eq!(h.app.tool, Tool::Grid);
    h.click_at(p(-1.0, -1.5));
    h.click_at(p(-1.0, 10.5));
    h.click_at(p(-2.0, 4.5));
    h.click_at(p(14.0, 4.5));
    let names: Vec<String> = h.model().grids().map(|g| g.name.clone()).collect();
    assert_eq!(names, ["1", "A"]);
    h.key(Key::Escape);
    assert_eq!(h.app.tool, Tool::Arrow);
    h.type_text("a");
    assert_eq!(h.app.tool, Tool::Footing);
    h.click_at(p(6.0, 4.5));
    assert!(matches!(
        h.model().footings().next().unwrap().kind,
        FootingKind::Pad { column: None, .. }
    ));
    h.app.choose_method("Corrida");
    h.click_at(p(6.0, 0.0));
    let strips = |h: &Harness| {
        h.model()
            .footings()
            .filter(|f| matches!(f.kind, FootingKind::Strip { wall: Some(_), .. }))
            .count()
    };
    assert_eq!(strips(&h), 1);
    // The same wall again: refused.
    h.click_at(p(7.0, 0.0));
    assert_eq!(strips(&h), 1);
    h.app.choose_method("Cimentar planta");
    assert_eq!(strips(&h), 4);
    // A tie beam by two points, chained.
    h.app.choose_method("Viga de atado");
    h.click_at(p(3.0, -3.0));
    h.click_at(p(9.0, -3.0));
    h.key(Key::Escape);
    assert_eq!(h.model().footings().count(), 6);
    // Selected, it shows its properties; moved, it goes.
    h.key(Key::Escape);
    h.click_at(p(6.0, -3.0));
    assert!(matches!(
        h.app.ed.selection.iter().next(),
        Some(Element::Footing(_))
    ));
    h.frame();
}

/// A masterplan: the house drawn here saved as a project, loaded as a
/// module and placed three times (turned, and mirrored); the copies are
/// drawn and exported but are not elements of the masterplan; one is
/// selected by clicking inside it. With `ARCHISIM_TEST_OUT` set, leaves
/// `vivienda.archisim`, `masterplan.archisim` and `masterplan.ifc` there.
#[test]
fn masterplan_places_modules() {
    let dir = match std::env::var_os("ARCHISIM_TEST_OUT") {
        Some(d) => PathBuf::from(d),
        None => std::env::temp_dir().join(format!("archisim-modules-{}", std::process::id())),
    };
    std::fs::create_dir_all(&dir).unwrap();
    let mut house = Harness::new(Editor::new_project());
    draw_outline(&mut house);
    furnish(&mut house);
    let house_path = house
        .app
        .ed
        .save(Some(&dir.join("vivienda.archisim")))
        .unwrap();

    let mut h = Harness::new(Editor::new_project());
    h.app.ed.path = Some(dir.join("masterplan.archisim"));
    h.app.load_module_from(&house_path);
    assert_eq!(h.app.tool, Tool::Module);
    let canvas = h.app.canvas;
    h.app
        .view
        .fit_exact(canvas, p(-15.0, -15.0), p(65.0, 25.0), 0.0);
    h.click_at(p(0.0, 0.0));
    // Space: the next one turned a quarter.
    h.key(Key::Space);
    h.click_at(p(30.0, 0.0));
    h.app.module_tool.rotation = 0.0;
    h.app.module_tool.mirror = true;
    h.click_at(p(50.0, 0.0));
    assert_eq!(h.model().modules().count(), 3);
    assert_eq!(h.model().walls().count(), 0);
    assert_eq!(h.app.ed.shown().walls().count(), 12);
    assert_eq!(h.app.ed.shown().openings().count(), 6);
    h.frame();
    h.key(Key::Escape);
    assert_eq!(h.app.tool, Tool::Arrow);
    // Inside the first copy: the copy is selected, with its properties.
    h.click_at(p(6.0, 4.5));
    assert!(matches!(
        h.app.ed.selection.iter().next(),
        Some(Element::Module(_))
    ));
    h.frame();
    let path = h
        .app
        .ed
        .save(Some(&dir.join("masterplan.archisim")))
        .unwrap();
    let reopened = Editor::open(&path).unwrap();
    assert_eq!(reopened.model(), h.model());
    reopened
        .export_ifc(&dir.join("masterplan.ifc"), archisim_ifc::Schema::Ifc4x3)
        .unwrap();
    let ifc = std::fs::read_to_string(dir.join("masterplan.ifc")).unwrap();
    assert_eq!(ifc.matches("IFCWALL(").count(), 12);
    assert_eq!(ifc.matches("IFCDOOR(").count(), 3);
    // Deleted, the copy goes; the module stays, to place it again.
    h.key(Key::Delete);
    assert_eq!(h.model().modules().count(), 2);
    assert_eq!(h.model().module_sources().count(), 1);
}

/// Concepto: a rectangle face pushed up with the mouse, a mass by floors
/// typed, a parallel contour on top, a band along a path, a guide, a
/// material, and the mass turned into a building.
#[test]
fn concept_volumes_are_drawn_pushed_and_converted() {
    use super::concept_ui::ConceptPick;
    use super::toolbox::{Action, Discipline, Pick};
    let mut h = Harness::new(Editor::new_project());
    let canvas = h.app.canvas;
    h.app
        .view
        .fit_exact(canvas, p(-5.0, -12.0), p(30.0, 16.0), 0.0);
    h.app.set_discipline(Discipline::Concept);
    // Arista, with the rectangle of the contextual bar: a face at once.
    h.app.pick(Pick::Concept(ConceptPick::Line));
    assert_eq!(h.app.tool, Tool::Mass);
    h.app.choose_method("Rectángulo");
    h.click_at(p(0.0, 0.0));
    h.click_at(p(10.0, 8.0));
    let face = h.model().masses().next().unwrap().id;
    assert!(h.model().mass(face).unwrap().is_face());
    // Empujar: taken, the mouse 4 m up the plan, clicked.
    h.app.pick(Pick::Concept(ConceptPick::Push));
    h.click_at(p(5.0, 4.0));
    h.move_to(p(5.0, 8.0));
    h.frame();
    h.click_at(p(5.0, 8.0));
    assert!((h.model().mass(face).unwrap().height - 4.0).abs() < 1e-6);
    // Masa: four corners, then pushed to 9 m typed (whole floors).
    h.app.pick(Pick::Concept(ConceptPick::Mass));
    for c in [
        p(14.0, 0.0),
        p(24.0, 0.0),
        p(24.0, 10.0),
        p(14.0, 10.0),
        p(14.0, 0.0),
    ] {
        h.click_at(c);
    }
    let mass = h
        .model()
        .masses()
        .find(|m| m.floor_height.is_some())
        .unwrap()
        .id;
    assert_eq!(h.model().mass(mass).unwrap().floors(), 3);
    h.app.pick(Pick::Concept(ConceptPick::Push));
    h.click_at(p(19.0, 5.0));
    h.type_text("12000");
    h.key(Key::Enter);
    assert_eq!(h.model().mass(mass).unwrap().floors(), 4);
    // Equidistancia: 1 m inside, on top.
    h.app.pick(Pick::Concept(ConceptPick::Offset));
    h.click_at(p(19.0, 5.0));
    h.type_text("-1000");
    h.key(Key::Enter);
    assert_eq!(h.model().masses().count(), 3);
    // Seguir: a band along two segments.
    h.app.pick(Pick::Concept(ConceptPick::Follow));
    h.click_at(p(0.0, -4.0));
    h.click_at(p(10.0, -4.0));
    h.click_at(p(10.0, -8.0));
    h.key(Key::Enter);
    assert!(h
        .model()
        .masses()
        .any(|m| matches!(m.shape, archisim_core::MassShape::Sweep { .. })));
    // Cinta: a guide.
    h.app.pick(Pick::Concept(ConceptPick::Tape));
    h.click_at(p(0.0, 12.0));
    h.click_at(p(24.0, 12.0));
    assert_eq!(h.model().drafts().count(), 1);
    // Pintar.
    let brick = h.model().materials().last().unwrap().id;
    h.app.mass_settings.material = brick;
    h.app.pick(Pick::Concept(ConceptPick::Paint));
    h.click_at(p(2.0, 2.0));
    // In plan, its top face (R19: one face, not the whole volume).
    assert_eq!(
        h.model().face_paint(face.0, archisim_core::FaceRef::Top),
        Some(brick)
    );
    h.frame();
    // Convertir: the mass selected becomes walls, slabs and a roof.
    h.key(Key::Escape);
    h.key(Key::Escape);
    h.app.ed.selection = [Element::Mass(mass)].into_iter().collect();
    h.frame();
    h.app.pick(Pick::Action(Action::Convert));
    assert_eq!(h.model().walls().count(), 16);
    assert_eq!(h.model().levels().len(), 5);
    assert!(!h.model().mass(mass).unwrap().building_outdated());
    h.frame();
    // The 3D draws the volumes.
    assert!(archisim_geom::solid::model_solids(h.model())
        .iter()
        .any(|s| matches!(s.owner, archisim_geom::solid::SolidOwner::Mass(_))));
}

/// Urbanismo: a plot by its corners and a road by its axis, with their
/// figures; a mass on the plot takes the plot's area and is checked
/// against its setback.
#[test]
fn plots_and_roads_are_drawn() {
    use super::concept_ui::ConceptPick;
    use super::toolbox::{Discipline, Pick};
    let mut h = Harness::new(Editor::new_project());
    let canvas = h.app.canvas;
    h.app
        .view
        .fit_exact(canvas, p(-10.0, -20.0), p(50.0, 30.0), 0.0);
    h.app.set_discipline(Discipline::Site);
    h.type_text("y");
    assert_eq!(h.app.tool, Tool::Plot);
    for c in [
        p(0.0, 0.0),
        p(40.0, 0.0),
        p(40.0, 25.0),
        p(0.0, 25.0),
        p(0.0, 0.0),
    ] {
        h.click_at(c);
    }
    let plot = h.model().plots().next().unwrap();
    assert!((plot.area() - 1000.0).abs() < 1e-6);
    assert!((plot.setback - 3.0).abs() < 1e-9);
    h.frame();
    h.key(Key::Escape);
    h.type_text("w");
    assert_eq!(h.app.tool, Tool::Road);
    h.click_at(p(-5.0, -10.0));
    h.click_at(p(45.0, -10.0));
    h.key(Key::Enter);
    let road = h.model().roads().next().unwrap();
    assert!((road.length() - 50.0).abs() < 1e-6);
    assert_eq!(road.bands().len(), 7);
    h.frame();
    // A mass on the plot: its figures use the plot's area; one that
    // reaches the boundary leaves the setback.
    h.key(Key::Escape);
    h.app.pick(Pick::Concept(ConceptPick::Mass));
    for c in [
        p(1.0, 1.0),
        p(21.0, 1.0),
        p(21.0, 11.0),
        p(1.0, 11.0),
        p(1.0, 1.0),
    ] {
        h.click_at(c);
    }
    let m = h.model().masses().next().unwrap().clone();
    let f = m.figures(h.model().plot_area());
    assert!((f.plot_ratio.unwrap() - 0.6).abs() < 1e-9);
    assert_eq!(
        h.app.mass_setback(&m.outline()),
        Some("Se sale del retranqueo")
    );
    h.app.ed.selection = [Element::Mass(m.id)].into_iter().collect();
    h.frame();
    // Plot and road selected show their properties.
    h.app.ed.selection = [Element::Road(road_id(&h))].into_iter().collect();
    h.frame();
}

fn road_id(h: &Harness) -> archisim_core::RoadId {
    h.model().roads().next().unwrap().id
}

/// Spline: curved walls through clicked points and a slab with a free
/// edge. Elegir línea: a room on a closed polyline of the 2D drawing, a
/// railing along a line. The length and angle fields place the next point.
#[test]
fn splines_picked_lines_and_typed_points() {
    use super::methods_ui::DrawMethod;
    use archisim_core::{NewDraft, Polyline, Shape as DShape, ViewRef};
    let mut h = Harness::new(Editor::new_project());
    let canvas = h.app.canvas;
    h.app
        .view
        .fit_exact(canvas, p(-5.0, -5.0), p(30.0, 25.0), 0.0);
    // Walls along a spline: two curved walls.
    h.type_text("m");
    assert_eq!(h.app.tool, Tool::Wall);
    h.app.set_draw_method(DrawMethod::Spline);
    for c in [p(0.0, 0.0), p(5.0, 3.0), p(10.0, 0.0)] {
        h.click_at(c);
    }
    h.key(Key::Enter);
    assert_eq!(h.model().walls().count(), 2);
    assert!(h.model().walls().all(|w| w.bulge.abs() > 1e-6));
    // A slab with a free edge (closed spline).
    h.key(Key::Escape);
    h.key(Key::Escape);
    h.type_text("f");
    h.app.set_draw_method(DrawMethod::Spline);
    for c in [p(14.0, 0.0), p(24.0, 0.0), p(24.0, 8.0), p(14.0, 8.0)] {
        h.click_at(c);
    }
    h.key(Key::Enter);
    let slab = h.model().slabs().next().unwrap();
    assert!(slab.outline.len() >= 24);
    // A closed polyline of the drawing becomes a room; a line a railing.
    let level = h.app.ed.level;
    h.app
        .ed
        .transact("Dibujo", |m| {
            let layer = m.draw_layers()[0].id;
            m.add_draft(NewDraft::new(
                ViewRef::Level(level),
                layer,
                DShape::Polyline(Polyline {
                    points: vec![p(0.0, 10.0), p(8.0, 10.0), p(8.0, 16.0), p(0.0, 16.0)],
                    bulges: vec![0.0; 4],
                    closed: true,
                    width: 0.0,
                }),
            ))?;
            m.add_draft(NewDraft::new(
                ViewRef::Level(level),
                layer,
                DShape::Line {
                    a: p(12.0, 12.0),
                    b: p(22.0, 12.0),
                },
            ))
            .map(drop)
        })
        .unwrap();
    h.key(Key::Escape);
    h.key(Key::Escape);
    h.type_text("z");
    assert_eq!(h.app.tool, Tool::Zone);
    h.app.room_settings.wand = false;
    h.app.set_draw_method(DrawMethod::PickLine);
    h.click_at(p(4.0, 10.0));
    assert_eq!(h.model().rooms().count(), 1);
    h.key(Key::Escape);
    h.key(Key::Escape);
    h.type_text("b");
    assert_eq!(h.app.tool, Tool::Railing);
    h.app.set_draw_method(DrawMethod::PickLine);
    h.click_at(p(17.0, 12.0));
    assert_eq!(h.model().railings().count(), 1);
    // The length and angle fields: a wall of exactly 4 m at 90°.
    h.key(Key::Escape);
    h.key(Key::Escape);
    h.type_text("m");
    h.click_at(p(0.0, 20.0));
    h.app.point_at(p(0.0, 24.0));
    h.key(Key::Escape);
    assert!(h
        .model()
        .walls()
        .any(|w| w.start.approx_eq(p(0.0, 20.0)) && w.end.approx_eq(p(0.0, 24.0))));
    h.frame();
}

/// Instalaciones: a cold water pipe and a supply duct drawn by their
/// points, a light and a socket placed with a click; moved, copied,
/// deleted, and written to the IFC as segments of their systems.
#[test]
fn services_are_drawn() {
    use super::toolbox::{Discipline, Pick};
    use archisim_core::{DeviceKind, ServiceSystem};
    let mut h = Harness::new(Editor::new_project());
    let canvas = h.app.canvas;
    h.app
        .view
        .fit_exact(canvas, p(-2.0, -2.0), p(12.0, 10.0), 0.0);
    h.app.set_discipline(Discipline::Services);
    h.app.pick(Pick::Tool(Tool::Pipe));
    assert_eq!(h.app.tool, Tool::Pipe);
    h.click_at(p(0.0, 0.0));
    h.click_at(p(6.0, 0.0));
    h.click_at(p(6.0, 4.0));
    h.key(Key::Enter);
    let pipe = h.model().runs().next().unwrap().clone();
    assert_eq!(pipe.system, ServiceSystem::ColdWater);
    assert!((pipe.length() - 10.0).abs() < 1e-6);
    assert!((pipe.height - pipe.width).abs() < 1e-12);
    h.frame();
    h.key(Key::Escape);
    h.app.pick(Pick::Tool(Tool::Duct));
    h.app.service_settings.duct_system = ServiceSystem::ExhaustAir;
    h.click_at(p(0.0, 8.0));
    h.click_at(p(10.0, 8.0));
    h.key(Key::Enter);
    let duct = h.model().runs().find(|r| r.is_duct()).unwrap().clone();
    assert_eq!(duct.system, ServiceSystem::ExhaustAir);
    assert!((duct.elevation - 2.6).abs() < 1e-9);
    h.frame();
    h.key(Key::Escape);
    h.app.pick(Pick::Device(DeviceKind::Light));
    assert_eq!(h.app.tool, Tool::Device);
    h.click_at(p(3.0, 3.0));
    h.app.pick(Pick::Device(DeviceKind::Socket));
    h.key(Key::Space);
    h.click_at(p(9.0, 0.0));
    let light = h
        .model()
        .devices()
        .find(|d| d.kind == DeviceKind::Light)
        .unwrap()
        .clone();
    assert!((light.elevation - 2.6).abs() < 1e-9);
    let socket = h
        .model()
        .devices()
        .find(|d| d.kind == DeviceKind::Socket)
        .unwrap()
        .clone();
    assert!((socket.elevation - 0.3).abs() < 1e-9);
    assert!((socket.rotation - std::f64::consts::FRAC_PI_2).abs() < 1e-9);
    h.frame();
    // Selected, they show their properties and are picked in the plan.
    h.key(Key::Escape);
    h.key(Key::Escape);
    assert_eq!(
        crate::pick::pick(h.model(), h.app.ed.level, p(3.0, 0.0), 0.05),
        Some(Element::Run(pipe.id))
    );
    for e in [
        Element::Run(pipe.id),
        Element::Run(duct.id),
        Element::Device(light.id),
    ] {
        h.app.ed.selection = [e].into_iter().collect();
        h.frame();
    }
    // Moved and copied as everything else.
    let t = archisim_draft::Transform::translate(archisim_core::Vec2::new(1.0, 0.0));
    let sel: std::collections::BTreeSet<Element> =
        [Element::Run(pipe.id), Element::Device(light.id)]
            .into_iter()
            .collect();
    let mut moved = h.model().clone();
    crate::xform::transform(&mut moved, &sel, &t, false).unwrap();
    assert!((moved.run(pipe.id).unwrap().path[0].x - 1.0).abs() < 1e-9);
    let r = h
        .app
        .ed
        .transact("Copiar", |m| crate::xform::transform(m, &sel, &t, true));
    assert_eq!(r.unwrap().len(), 2);
    assert_eq!(h.model().runs().count(), 3);
    assert_eq!(h.model().devices().count(), 3);
    // The IFC carries them in their systems, and reads them back.
    let ifc = archisim_ifc::export_ifc(h.model(), &archisim_ifc::ExportOptions::default());
    for class in [
        "IFCPIPESEGMENT(",
        "IFCDUCTSEGMENT(",
        "IFCLIGHTFIXTURE(",
        "IFCOUTLET(",
        "IFCDISTRIBUTIONSYSTEM(",
    ] {
        assert!(ifc.contains(class), "{class}");
    }
    let back = archisim_ifc::import_ifc(&ifc).unwrap();
    assert_eq!(back.model.runs().count(), 3, "{:?}", back.report);
    assert_eq!(back.model.devices().count(), 3);
    assert!(archisim_ifc::differences(h.model(), &back.model).is_empty());
    // Deleted with Supr.
    h.app.ed.selection = [Element::Device(socket.id)].into_iter().collect();
    h.key(Key::Delete);
    assert_eq!(h.model().devices().count(), 2);
}

/// Arista (R12): lines of the model that stay open, and close into a face
/// on their first point or with the edges already there; Spline open by
/// default (R16) and in the 2D drawing; the drawing orders of the bottom
/// bar show their variants in the contextual bar (R15).
#[test]
fn concept_edges_close_into_faces() {
    use super::concept_ui::ConceptPick;
    use super::toolbox::{Discipline, Pick};
    let mut h = Harness::new(Editor::new_project());
    let canvas = h.app.canvas;
    h.app
        .view
        .fit_exact(canvas, p(-5.0, -12.0), p(30.0, 16.0), 0.0);
    h.app.set_discipline(Discipline::Concept);
    h.app.pick(Pick::Concept(ConceptPick::Line));
    assert_eq!(h.app.method_states()[0].0, "Línea");
    // An open line: an edge, not a face.
    h.click_at(p(0.0, 0.0));
    h.click_at(p(6.0, 0.0));
    h.click_at(p(6.0, 4.0));
    h.key(Key::Enter);
    assert_eq!(h.model().edges().count(), 1);
    assert_eq!(h.model().masses().count(), 0);
    // A second line from its end back to its start: the edges close a
    // face, and go.
    h.click_at(p(6.0, 4.0));
    h.click_at(p(0.0, 4.0));
    h.click_at(p(0.0, 0.0));
    h.key(Key::Enter);
    assert_eq!(h.model().edges().count(), 0);
    let face = h.model().masses().next().unwrap();
    assert!(face.is_face());
    assert!((archisim_core::signed_area(&face.outline()).abs() - 24.0).abs() < 1e-6);
    // Back on its first point: a face at once.
    h.click_at(p(10.0, 0.0));
    h.click_at(p(14.0, 0.0));
    h.click_at(p(14.0, 3.0));
    h.click_at(p(10.0, 0.0));
    assert_eq!(h.model().masses().count(), 2);
    h.frame();
    // Spline: open by default, an edge; on its first point, a face.
    h.app.choose_method("Spline");
    h.click_at(p(0.0, 8.0));
    h.click_at(p(4.0, 10.0));
    h.click_at(p(8.0, 8.0));
    h.key(Key::Enter);
    assert_eq!(h.model().edges().count(), 1);
    h.app.choose_method("Spline");
    h.click_at(p(12.0, 8.0));
    h.click_at(p(16.0, 10.0));
    h.click_at(p(20.0, 8.0));
    h.click_at(p(12.0, 8.0));
    assert_eq!(h.model().masses().count(), 3);
    // An edge selected shows its properties; deleted with Supr.
    h.key(Key::Escape);
    h.key(Key::Escape);
    let edge = h.model().edges().next().unwrap().id;
    h.app.ed.selection = [Element::Edge(edge)].into_iter().collect();
    h.frame();
    h.key(Key::Delete);
    assert_eq!(h.model().edges().count(), 0);
    // The 2D Spline of the bottom bar: a polyline of arcs through the
    // points; while it runs, its variants are in the contextual bar.
    h.app.keys_autocad = true;
    h.app.pick(Pick::Command("SPLINE"));
    assert_eq!(h.app.tool, Tool::Cad);
    h.frame();
    h.click_at(p(0.0, -6.0));
    h.click_at(p(5.0, -4.0));
    h.click_at(p(10.0, -6.0));
    h.key(Key::Enter);
    let d = h.model().drafts().last().unwrap();
    match &d.shape {
        archisim_core::Shape::Polyline(pl) => {
            assert!(!pl.closed);
            // Through the three points, with many in between.
            assert!(pl.points.len() > 10);
            for q in [p(0.0, -6.0), p(5.0, -4.0), p(10.0, -6.0)] {
                assert!(pl.points.iter().any(|x| x.distance(q) < 1e-6));
            }
        }
        other => panic!("{other:?}"),
    }
}

/// Where a model point is on the screen, in the 3D view.
fn screen_3d(h: &Harness, x: f64, y: f64, z: f64) -> Pos2 {
    let r = h.app.v3d.canvas;
    let s = h
        .app
        .v3d
        .camera
        .project_to_screen(
            glam::Vec3::new(x as f32, y as f32, z as f32),
            glam::Vec2::new(r.width(), r.height()),
        )
        .unwrap();
    Pos2::new(r.min.x + s.x, r.min.y + s.y)
}

/// R14: the tools work in the 3D view. A wall drawn by two clicks on the
/// ground; a face by its rectangle, pushed up with the mouse; a column
/// placed; Esc takes the arrow back without leaving the 3D view.
#[test]
fn tools_work_in_the_3d_view() {
    use super::concept_ui::ConceptPick;
    use super::toolbox::Pick;
    let mut h = Harness::new(Editor::new_project());
    draw_outline(&mut h);
    h.key(Key::Escape);
    h.key(Key::Escape);
    h.key(Key::F3);
    h.frame();
    h.frame();
    assert_eq!(h.app.mode, ViewMode::ThreeD);
    // Looking down steeply, so the ground inside is not behind the walls.
    h.app.v3d.camera.pitch = 1.35;
    h.frame();
    let walls = h.model().walls().count();
    // A wall inside the outline, from (3, 3) to (9, 3), on the level.
    h.type_text("m");
    assert_eq!((h.app.mode, h.app.tool), (ViewMode::ThreeD, Tool::Wall));
    h.app.wall_tool.set_mode(crate::wall_tool::WallMode::Single);
    let a = screen_3d(&h, 3.0, 3.0, 0.0);
    let b = screen_3d(&h, 9.0, 3.0, 0.0);
    assert_eq!(super::ghost3d::ghost_triangles(&h.app), 0);
    h.click_screen(a);
    // R21: before the second click, the wall is seen in 3D as it will be.
    h.pos = b;
    h.events.push(Event::PointerMoved(b));
    h.frame();
    assert!(super::ghost3d::ghost_triangles(&h.app) >= 12);
    h.click_screen(b);
    assert_eq!(h.model().walls().count(), walls + 1);
    let w = h.model().walls().last().unwrap().clone();
    // A window over that wall is seen before it is placed, too.
    h.type_text("v");
    assert_eq!((h.app.mode, h.app.tool), (ViewMode::ThreeD, Tool::Window));
    let on = screen_3d(&h, 6.0, 3.0, 1.0);
    h.pos = on;
    h.events.push(Event::PointerMoved(on));
    h.frame();
    assert!(h.app.placement.is_some());
    assert!(super::ghost3d::ghost_triangles(&h.app) > 0);
    h.key(Key::Escape);
    let near = |q: Point2, x: f64, y: f64| (q.x - x).abs() < 0.1 && (q.y - y).abs() < 0.1;
    assert!(
        (near(w.start, 3.0, 3.0) && near(w.end, 9.0, 3.0))
            || (near(w.end, 3.0, 3.0) && near(w.start, 9.0, 3.0)),
        "{:?} {:?}",
        w.start,
        w.end
    );
    // A face, by its rectangle, then pushed up.
    h.app.pick(Pick::Concept(ConceptPick::Line));
    assert_eq!(h.app.mode, ViewMode::ThreeD);
    h.app.choose_method("Rectángulo");
    h.click_screen(screen_3d(&h, 1.0, 5.0, 0.0));
    h.click_screen(screen_3d(&h, 4.0, 8.0, 0.0));
    let face = h.model().masses().next().unwrap().id;
    assert!(h.model().mass(face).unwrap().is_face());
    h.app.pick(Pick::Concept(ConceptPick::Push));
    h.click_screen(screen_3d(&h, 2.5, 6.5, 0.0));
    assert!(h.app.concept.push.is_some());
    // Up the screen: higher.
    let up = screen_3d(&h, 2.5, 6.5, 3.0);
    h.pos = up;
    h.events.push(Event::PointerMoved(up));
    h.frame();
    h.click_screen(up);
    let height = h.model().mass(face).unwrap().height;
    assert!((height - 3.0).abs() < 0.3, "{height}");
    // A column on the level.
    let columns = h.model().columns().count();
    h.type_text("i");
    assert_eq!((h.app.mode, h.app.tool), (ViewMode::ThreeD, Tool::Column));
    h.click_screen(screen_3d(&h, 8.0, 6.0, 0.0));
    assert_eq!(h.model().columns().count(), columns + 1);
    h.key(Key::Escape);
    assert_eq!((h.app.mode, h.app.tool), (ViewMode::ThreeD, Tool::Arrow));
}

/// R14, R18–R20: Pintar paints the face clicked: a side of a wall (in
/// plan, by the side clicked; in 3D, the face seen), one face of a volume,
/// or the zone of a façade between edges drawn up it in 3D; Quitar
/// pintura takes it off. The 3D shows the paint on that face only.
#[test]
fn faces_are_painted() {
    use super::concept_ui::ConceptPick;
    use super::toolbox::Pick;
    use archisim_core::{FaceRef, FaceSide};
    let mut h = Harness::new(Editor::new_project());
    draw_outline(&mut h);
    h.key(Key::Escape);
    h.key(Key::Escape);
    let brick = h.model().materials().last().unwrap().id;
    h.app.mass_settings.material = brick;
    h.app.pick(Pick::Concept(ConceptPick::Paint));
    // In plan: the west wall, clicked just east of its line.
    let west = h.model().walls().next().unwrap().clone();
    let mid = west.point_at(0.5, 0.0);
    h.click_at(mid + archisim_core::Vec2::new(0.1, 0.0));
    let painted: Vec<_> = h.model().face_paints().collect();
    assert_eq!(painted.len(), 1, "{painted:?}");
    let (e, face, m) = painted[0];
    assert_eq!((e, m), (west.id.0, brick));
    let inside = (west.end - west.start).cross(archisim_core::Vec2::new(0.1, 0.0)) > 0.0;
    let side = if inside {
        FaceSide::Exterior
    } else {
        FaceSide::Interior
    };
    assert_eq!(face, FaceRef::Wall(side));
    // The 3D shows the brick on that side only.
    let model = h.model().clone();
    let colour = model.material(brick).unwrap().cut_color;
    let mut painted_tris = 0;
    let mut plain = 0;
    for s in archisim_geom::solid::model_solids(&model)
        .iter()
        .filter(|s| s.owner == archisim_geom::solid::SolidOwner::Wall(west.id))
    {
        for t in &s.triangles {
            match archisim_geom::solid::painted_color(&model, s, t) {
                Some(c) => {
                    assert_eq!(c, colour);
                    painted_tris += 1;
                }
                None => plain += 1,
            }
        }
    }
    assert!(painted_tris > 0 && plain > painted_tris);
    // Quitar pintura, in plan, on the same side.
    h.app.paint_erase = true;
    h.click_at(mid + archisim_core::Vec2::new(0.1, 0.0));
    assert_eq!(h.model().face_paints().count(), 0);
    h.app.paint_erase = false;

    // In the 3D view, from the south: the south wall's outer face.
    h.key(Key::F3);
    h.frame();
    h.frame();
    assert_eq!((h.app.mode, h.app.tool), (ViewMode::ThreeD, Tool::Paint));
    h.app.v3d.camera.yaw = -1.3;
    h.app.v3d.camera.pitch = 0.35;
    h.frame();
    h.click_screen(screen_3d(&h, 6.0, -0.01, 1.5));
    let south = h
        .model()
        .walls()
        .find(|w| w.start.y.abs() < 1e-6 && w.end.y.abs() < 1e-6)
        .unwrap()
        .clone();
    assert_eq!(
        h.model()
            .face_paint(south.id.0, FaceRef::Wall(FaceSide::Exterior)),
        Some(brick)
    );
    // An edge drawn up the south façade, at x = 4: Arista, two clicks with
    // Mayús for the vertical.
    h.app.pick(Pick::Concept(ConceptPick::Line));
    h.click_screen(screen_3d(&h, 4.0, -0.001, 0.05));
    h.set_modifiers(Modifiers::SHIFT);
    let top = screen_3d(&h, 4.0, -0.001, 2.95);
    h.pos = top;
    h.events.push(Event::PointerMoved(top));
    h.frame();
    h.click_screen(top);
    h.set_modifiers(Modifiers::NONE);
    h.key(Key::Enter);
    let edge = h.model().edges().next().unwrap().clone();
    assert!(!edge.is_flat(), "{edge:?}");
    assert!((edge.points[0].x - 4.0).abs() < 0.05);
    // Painting west of it paints only that zone, in a second material.
    let stone = h.model().materials().find(|m| m.id != brick).unwrap().id;
    h.app.mass_settings.material = stone;
    h.app.pick(Pick::Concept(ConceptPick::Paint));
    h.click_screen(screen_3d(&h, 9.0, -0.01, 1.5));
    let zones: Vec<_> = h.model().paint_zones().cloned().collect();
    assert_eq!(zones.len(), 1, "{zones:?}");
    let area = archisim_core::signed_area(&zones[0].outline).abs();
    assert!(area > 1.0 && area < south.length() * south.height, "{area}");
    assert!(!archisim_geom::solid::paint_zone_triangles(h.model()).is_empty());
    // A volume: only its top is painted.
    h.key(Key::F2);
    let settings = crate::editor::MassSettings::new(h.model());
    let mass = h
        .app
        .ed
        .add_mass_outline(
            &settings,
            vec![p(2.0, 2.0), p(5.0, 2.0), p(5.0, 5.0), p(2.0, 5.0)],
        )
        .unwrap();
    assert_eq!(h.app.tool, Tool::Paint);
    h.click_at(p(3.5, 3.5));
    assert_eq!(
        h.model().face_paint(mass.0, FaceRef::Top),
        Some(stone),
        "{:?} {:?}",
        h.app.message.as_ref().map(|m| &m.text),
        h.app.mode
    );
    assert_ne!(h.model().mass(mass).unwrap().material, stone);
}

/// P1: a presentation style on the active plan (walls solid in its
/// colour, its paper) and a room's floor drawn with a pattern; the
/// «Estilos de presentación» window shows and edits them.
#[test]
fn presentation_styles_and_floor_fills() {
    let mut h = Harness::new(Editor::new_project());
    draw_outline(&mut h);
    h.key(Key::Escape);
    h.key(Key::Escape);
    let bosque = h
        .model()
        .presentation_styles()
        .find(|s| s.name == "Bosque")
        .unwrap()
        .clone();
    h.app.view_style.presentation = Some(bosque.id);
    let level = h.app.ed.level;
    let svg = archisim_plan::render_plan(h.model(), level, &h.app.plan_export_options(50.0));
    let [r, g, b] = bosque.poche.unwrap();
    assert!(
        svg.contains(&format!("#{r:02x}{g:02x}{b:02x}")),
        "solid walls"
    );
    let [r, g, b] = bosque.background;
    assert!(
        svg.contains(&format!("#{r:02x}{g:02x}{b:02x}")),
        "the paper"
    );
    h.frame();
    // The window, with the style chosen, draws and edits it.
    h.app.presentation.open = true;
    h.app.presentation.selected = Some(bosque.id);
    h.frame();
    h.frame();
    let id = h
        .app
        .ed
        .add_presentation_style(archisim_core::PresentationStyle::new("Bosque"))
        .unwrap();
    assert_eq!(h.model().presentation_style(id).unwrap().name, "Bosque 2");
    h.frame();
    // A room with a wooden floor.
    let room = h
        .app
        .ed
        .transact("Local", |m| {
            m.add_room(archisim_core::NewRoom {
                level,
                number: "1".into(),
                name: "Salón".into(),
                outline: vec![p(1.0, 1.0), p(5.0, 1.0), p(5.0, 4.0), p(1.0, 4.0)],
                holes: Vec::new(),
                label: p(3.0, 2.5),
                bounded: false,
            })
        })
        .unwrap();
    let mut r = h.model().room(room).unwrap().clone();
    r.floor_fill = Some(archisim_core::FloorFill {
        pattern: archisim_core::HatchPattern::Stone,
        color: [90, 90, 90],
        background: Some([210, 205, 195]),
        scale: 1.0,
        angle: 0.0,
    });
    h.app.ed.set_room(r, "Pavimento").unwrap();
    h.app.ed.selection = [Element::Room(room)].into_iter().collect();
    h.frame();
    h.app.view_style.presentation = None;
    let svg = archisim_plan::render_plan(h.model(), level, &h.app.plan_export_options(50.0));
    assert!(svg.contains("floor-fills"));
    assert!(svg.contains("#d2cdc3"));
    // P2: trees drawn as see-through canopies in a presentation view, each
    // its own; in the technical one too since v0.36 (their own drawing by
    // default), unless the type says «según el estilo».
    h.app
        .ed
        .transact("Árboles", |m| {
            let t = archisim_library::add_object_entry(m, "Árbol en flor 5 m")?;
            let mut v = m.object_type(t).unwrap().values.clone();
            v.insert(
                "plan_drawing".into(),
                archisim_core::Value::Text("según el estilo".into()),
            );
            m.set_object_type_values(t, v)?;
            for x in [20.0, 30.0] {
                m.add_object(archisim_core::NewObject {
                    level,
                    object_type: t,
                    position: p(x, 3.0),
                    rotation: 0.0,
                    base_offset: 0.0,
                })?;
            }
            Ok(())
        })
        .unwrap();
    let technical = archisim_plan::render_plan(h.model(), level, &h.app.plan_export_options(50.0));
    assert!(!technical.contains("fill-opacity"));
    let ad5 = h
        .model()
        .presentation_styles()
        .find(|s| s.name == "AD5")
        .unwrap()
        .id;
    h.app.view_style.presentation = Some(ad5);
    let shown = archisim_plan::render_plan(h.model(), level, &h.app.plan_export_options(50.0));
    assert!(shown.contains("fill-opacity"), "see-through canopies");
    assert!(shown.contains("#e2a0b6"), "the blossom's colour");
    let ids: Vec<_> = h.model().objects().map(|o| o.id).collect();
    let canopy = |id| match &h
        .model()
        .object_presentation_symbol(id, 50.0)
        .unwrap()
        .unwrap()[0]
    {
        archisim_core::Mark::Polygon { points, .. } => points.clone(),
        other => panic!("{other:?}"),
    };
    assert_ne!(canopy(ids[0]), canopy(ids[1]), "each tree its own canopy");
    h.frame();
}

/// P3–P4: a presentation board of the level (title, plan, 3D view,
/// areas and the sun's path) in the view's style, with shadows; it goes
/// out as PNG.
#[test]
fn presentation_boards() {
    let mut h = Harness::new(Editor::new_project());
    draw_outline(&mut h);
    h.key(Key::Escape);
    h.key(Key::Escape);
    let bosque = h
        .model()
        .presentation_styles()
        .find(|s| s.name == "Bosque")
        .unwrap()
        .clone();
    assert!(bosque.shadows);
    h.app.view_style.presentation = Some(bosque.id);
    h.frame();
    let sheets = h.model().sheets().len();
    h.app.new_presentation_sheet();
    h.frame();
    assert_eq!(h.model().sheets().len(), sheets + 1);
    let sheet = (*h.model().sheets().last().unwrap()).clone();
    assert_eq!(sheet.presentation, Some(bosque.id));
    assert_eq!(h.app.mode, ViewMode::Sheet(sheet.id));
    let views: Vec<_> = h.model().viewports_on(sheet.id).collect();
    assert!(views.len() >= 2, "plan and 3D");
    assert!(views
        .iter()
        .all(|v| v.style.presentation == Some(bosque.id)));
    assert!(h
        .model()
        .drafts_in(archisim_core::ViewRef::Sheet(sheet.id))
        .any(|d| matches!(&d.shape, archisim_core::Shape::Text(t) if t.content == "N")));
    let svg = archisim_plan::sheet::render_sheet(
        h.model(),
        sheet.id,
        &archisim_plan::sheet::SheetOptions::default(),
    );
    let [r, g, b] = bosque.background;
    assert!(
        svg.contains(&format!("rgb({r},{g},{b})")),
        "the sheet's paper"
    );
    assert!(svg.contains("fill-opacity"), "shadows");
    // As an image.
    let dir = std::env::temp_dir().join(format!("archisim-png-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let png = dir.join("lamina.png");
    h.app.ed.export_sheets(&png, &[sheet.id]).unwrap();
    let bytes = std::fs::read(&png).unwrap();
    assert_eq!(&bytes[1..4], b"PNG");
    // A3 at 200 dpi.
    assert_eq!(u32::from_be_bytes(bytes[16..20].try_into().unwrap()), 3307);
    let _ = std::fs::remove_dir_all(&dir);
    // The sun is the project's: day and hour.
    h.app
        .ed
        .set_sun(archisim_core::Sun {
            day: 355,
            hour: 12.0,
        })
        .unwrap();
    assert_eq!(h.model().sun().day, 355);
}

/// Antonio's review of the P block: Vegetación and Mobiliario urbano open
/// on their own part of the library; a tree's canopy is chosen tree by
/// tree; fills are drawn directly (rectangle, polygon, circle…) with
/// their opacity.
#[test]
fn vegetation_street_furniture_and_direct_fills() {
    use super::object_gallery::ObjectScope;
    use super::toolbox::Pick;
    let mut h = Harness::new(Editor::new_project());
    // Vegetación: the object tool with a tree, not furniture.
    h.app.pick(Pick::Objects(ObjectScope::Vegetation));
    h.frame();
    assert_eq!(h.app.tool, Tool::Object);
    let t = h.app.object_settings.object_type.unwrap();
    // A plant grown by ArchiSim (v0.37): an olive tree.
    assert_eq!(h.model().object_type(t).unwrap().name, "Olivo");
    h.click_at(p(2.0, 2.0));
    let tree = h.model().objects().last().unwrap().id;
    // Mobiliario urbano: a street lamp.
    h.app.pick(Pick::Objects(ObjectScope::Street));
    h.frame();
    let t = h.app.object_settings.object_type.unwrap();
    assert_eq!(h.model().object_type(t).unwrap().name, "Farola 4,5 m");
    assert_eq!(h.app.object_type_category(t), Some("Mobiliario urbano"));
    h.click_at(p(6.0, 2.0));
    assert_eq!(h.model().objects().count(), 2);
    // The tree drawn its own way, in a technical plan too.
    let own: archisim_core::Values = [
        (
            "drawing".to_string(),
            archisim_core::Value::Text("presentación".into()),
        ),
        (
            "style_own".to_string(),
            archisim_core::Value::Text("acuarela".into()),
        ),
        (
            "opacity_own".to_string(),
            archisim_core::Value::Number(0.35),
        ),
    ]
    .into_iter()
    .collect();
    h.app.ed.set_object_values(tree, own).unwrap();
    h.key(Key::Escape);
    h.app.ed.selection = [Element::Object(tree)].into_iter().collect();
    h.frame();
    let level = h.app.ed.level;
    let svg = archisim_plan::render_plan(h.model(), level, &h.app.plan_export_options(50.0));
    assert!(svg.contains("fill-opacity"), "washes");
    h.key(Key::Escape);
    h.key(Key::Escape);
    // A fill drawn as a rectangle, half see-through, in one go.
    h.app.keys_autocad = true;
    h.app.cad_settings.hatch_pattern = archisim_core::HatchPattern::Stone;
    h.app.cad_settings.hatch_opacity = 0.5;
    h.app.start_command("RELLRECT");
    h.click_at(p(10.0, 0.0));
    h.click_at(p(14.0, 3.0));
    h.frame();
    let fill = h
        .model()
        .drafts()
        .find_map(|d| match &d.shape {
            archisim_core::Shape::Hatch(x) => Some(x.clone()),
            _ => None,
        })
        .expect("a fill");
    assert_eq!(fill.pattern, archisim_core::HatchPattern::Stone);
    assert!((fill.opacity - 0.5).abs() < 1e-9);
    assert!((archisim_core::signed_area(&fill.loops[0].points).abs() - 12.0).abs() < 1e-6);
    // A polygon fill: three clicks and Enter.
    h.app.start_command("RELLPOLIG");
    h.click_at(p(10.0, 5.0));
    h.click_at(p(14.0, 5.0));
    h.click_at(p(12.0, 8.0));
    h.key(Key::Enter);
    h.frame();
    let fills = h
        .model()
        .drafts()
        .filter(|d| matches!(d.shape, archisim_core::Shape::Hatch(_)))
        .count();
    assert_eq!(fills, 2);
    let svg = archisim_plan::render_plan(h.model(), level, &h.app.plan_export_options(50.0));
    assert!(svg.contains("fill-opacity=\"0.5\""));
}

/// Urbanismo → Vegetación places a plant grown by ArchiSim (an olive by
/// default); «Otra planta» grows another; Plantación plants along a line
/// drawn with clicks and inside an outline, and changing the planting
/// makes its plants again; the vivero opens and shows a species.
#[test]
fn plants_and_plantings() {
    use super::object_gallery::ObjectScope;
    use super::toolbox::{Discipline, Pick};
    use archisim_core::Arrangement;
    let mut h = Harness::new(Editor::new_project());
    let canvas = h.app.canvas;
    h.app
        .view
        .fit_exact(canvas, p(-10.0, -20.0), p(60.0, 30.0), 0.0);
    h.app.set_discipline(Discipline::Site);
    h.app.pick(Pick::Objects(ObjectScope::Vegetation));
    assert_eq!(h.app.tool, Tool::Object);
    let olive = h.app.object_settings.object_type.unwrap();
    assert_eq!(h.model().object_type(olive).unwrap().name, "Olivo");
    assert!(h.model().type_grows(olive));
    h.click_at(p(5.0, 5.0));
    let tree = h
        .model()
        .objects()
        .find(|o| o.object_type == olive)
        .unwrap()
        .id;
    // Its plan and 3D come from the plant grown.
    assert!(h.model().object_plant(tree).is_some());
    assert!(archisim_geom::solid::object_solids(h.model(), tree).len() >= 2);
    let before = h.model().plant_look(tree).unwrap().seed;
    h.key(Key::Escape);
    h.app.ed.selection = [Element::Object(tree)].into_iter().collect();
    h.frame();
    // Another plant of the same species.
    let mut v = h.model().object(tree).unwrap().values.clone();
    v.insert("plant_seed".into(), archisim_core::Value::Number(4242.0));
    h.app.ed.set_object_values(tree, v).unwrap();
    assert_ne!(h.model().plant_look(tree).unwrap().seed, before);

    // A planting along a line: twelve plants, seen before planting.
    h.key(Key::Escape);
    h.app.set_tool(Tool::Planting);
    h.app.planting_settings.count = 12;
    h.click_at(p(0.0, -10.0));
    h.click_at(p(20.0, -14.0));
    h.move_to(p(40.0, -10.0));
    h.click_at(p(40.0, -10.0));
    h.key(Key::Enter);
    let pl = h.model().plantings().next().unwrap().clone();
    assert_eq!(pl.members.len(), 12);
    assert_eq!(pl.arrangement, Arrangement::Line);
    assert!(h.app.ed.selection.contains(&Element::Planting(pl.id)));
    h.frame();
    // Fewer plants, more irregular: made again.
    h.app
        .ed
        .transact("t", |m| {
            m.edit_planting(pl.id, |x| {
                x.count = 5;
                x.spread = 0.8;
            })
        })
        .unwrap();
    assert_eq!(h.model().planting(pl.id).unwrap().members.len(), 5);
    assert_eq!(
        h.model()
            .objects()
            .filter(|o| h.model().planting_of(o.id) == Some(pl.id))
            .count(),
        5
    );
    // Moving the planting moves its plants.
    let first = h.model().planting(pl.id).unwrap().members[0];
    let at = h.model().object(first).unwrap().position;
    h.app.ed.selection = [Element::Planting(pl.id)].into_iter().collect();
    h.app
        .ed
        .transact("mover", |m| {
            crate::xform::transform(
                m,
                &[Element::Planting(pl.id)].into_iter().collect(),
                &archisim_draft::Transform::translate(p(0.0, 30.0)),
                false,
            )
        })
        .unwrap();
    let first = h.model().planting(pl.id).unwrap().members[0];
    let moved = h.model().object(first).unwrap().position;
    assert!((moved.y - at.y - 30.0).abs() < 1e-6, "{at:?} {moved:?}");
    // Deleting it deletes its plants.
    let n = h.model().objects().count();
    h.app
        .ed
        .transact("borrar", |m| m.remove_planting(pl.id).map(drop))
        .unwrap();
    assert_eq!(h.model().objects().count(), n - 5);

    // Inside an outline.
    h.app.set_tool(Tool::Planting);
    h.app.planting_settings.arrangement = Arrangement::Area;
    h.app.planting_settings.count = 20;
    h.app.planting_settings.smooth = false;
    for c in [p(30.0, 0.0), p(50.0, 0.0), p(50.0, 15.0), p(30.0, 15.0)] {
        h.click_at(c);
    }
    h.key(Key::Enter);
    let area = h.model().plantings().next().unwrap().clone();
    assert!(area.closed);
    assert_eq!(area.members.len(), 20);
    for m in &area.members {
        let q = h.model().object(*m).unwrap().position;
        assert!(
            q.x > 29.9 && q.x < 50.1 && q.y > -0.1 && q.y < 15.1,
            "{q:?}"
        );
    }
    // Saved and opened again: the same planting and plants.
    let text = archisim_file::to_string(h.model());
    let back = archisim_file::from_str(&text).unwrap();
    assert_eq!(back.plantings().count(), 1);
    assert_eq!(back.objects().count(), h.model().objects().count());

    // The vivero.
    h.app.open_vivero(Some(olive), Default::default());
    h.frame();
    h.frame();
    assert!(h.app.vivero.open);
    h.app
        .ed
        .set_object_type_value(
            olive,
            "crown_shape",
            archisim_core::Value::Text("columnar".into()),
        )
        .unwrap();
    h.frame();
}

/// Two people on one shared folder, each with their own window: what one
/// reserves the other can't touch, new walls are their author's, and both
/// end up with the same project after publishing and receiving.
#[test]
fn team_work_in_a_shared_folder() {
    let dir = std::env::temp_dir().join(format!("archisim-equipo-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let wall_at = |h: &mut Harness, a: Point2, b: Point2| {
        h.key(Key::Escape);
        h.app.set_tool(Tool::Wall);
        h.click_at(a);
        h.click_at(b);
        h.key(Key::Escape);
        h.key(Key::Escape);
    };

    // Antonio draws a wall and shares the project.
    let mut a = Harness::new(Editor::new_project());
    let canvas = a.app.canvas;
    a.app
        .view
        .fit_exact(canvas, p(-5.0, -5.0), p(25.0, 15.0), 0.0);
    wall_at(&mut a, p(0.0, 0.0), p(10.0, 0.0));
    let w1 = a.model().walls().next().unwrap().id;
    a.app.start_team(&dir, "Antonio", true);
    assert!(a.app.team.is_some(), "{:?}", a.app.message);
    assert!(dir.join(archisim_team::TEAM_FILE).is_file());
    a.app.team_reserve(&[w1.0]);
    a.frame();

    // Ana opens it: she has the wall, but can't delete Antonio's.
    let mut b = Harness::new(Editor::new_project());
    b.app
        .view
        .fit_exact(canvas, p(-5.0, -5.0), p(25.0, 15.0), 0.0);
    b.app.start_team(&dir, "Ana", false);
    assert!(b.app.team.is_some(), "{:?}", b.app.message);
    assert!(b.model().wall(w1).is_some());
    // The Equipo window opens on joining; out of the way of the clicks.
    b.app.team_ui.open = false;
    a.app.team_ui.open = false;
    for h in [&mut a, &mut b] {
        h.frame();
        h.app
            .view
            .fit_exact(canvas, p(-5.0, -5.0), p(25.0, 15.0), 0.0);
    }
    b.frame();
    b.frame_after(4.0);
    b.app.ed.selection = [Element::Wall(w1)].into_iter().collect();
    b.frame();
    b.key(Key::Delete);
    assert!(
        b.model().wall(w1).is_some(),
        "Antonio's wall is still there"
    );
    assert!(
        b.app
            .message
            .as_ref()
            .is_some_and(|m| m.error && m.text.contains("Antonio")),
        "{:?}",
        b.app.message
    );
    // She draws her own wall: it is hers.
    wall_at(&mut b, p(0.0, 8.0), p(10.0, 8.0));
    let w2 = b
        .model()
        .walls()
        .map(|w| w.id)
        .find(|w| *w != w1)
        .unwrap_or_else(|| panic!("{:?} {:?}", b.app.message, b.app.tool));
    b.frame();
    let team = b.app.team.as_ref().unwrap();
    assert!(team.session.me.owns_id(w2.0));
    assert!(team
        .session
        .reserved_by()
        .get("ana")
        .is_some_and(|s| s.contains(&w2.0)));
    b.app.team_ui.note = "Muro norte".into();
    b.app.team_sync(true);
    assert!(
        b.app.message.as_ref().is_some_and(|m| !m.error),
        "{:?}",
        b.app.message
    );

    // Antonio receives it, draws another wall and publishes.
    a.frame_after(4.0);
    assert_eq!(a.app.team.as_ref().unwrap().session.unreceived(), 1);
    a.app.team_sync(false);
    assert!(a.model().wall(w2).is_some());
    // He can't move Ana's wall either.
    let before = a.model().wall(w2).unwrap().start;
    let r = a.app.ed.transact("Mover", |m| {
        m.move_wall(w2, Point2::new(0.0, 9.0), Point2::new(10.0, 9.0))
    });
    assert!(r.is_err());
    assert_eq!(a.model().wall(w2).unwrap().start, before);
    wall_at(&mut a, p(10.0, 0.0), p(10.0, 8.0));
    a.frame();
    assert_eq!(a.model().walls().count(), 3);
    a.app.team_sync(true);
    b.app.team_sync(false);
    assert_eq!(
        archisim_team::doc_of(a.model()),
        archisim_team::doc_of(b.model())
    );
    // The window, the strip in Properties and the reservations in plan.
    b.app.team_ui.open = true;
    b.app.ed.selection = [Element::Wall(w1)].into_iter().collect();
    b.frame();
    b.frame();
    b.app.team_ui.open = false;
    // Ana asks for the wall; Antonio gets the request and releases it.
    b.app
        .team_send("antonio", "¿Me dejas el muro sur?", &[w1.0]);
    a.frame_after(4.0);
    assert_eq!(a.app.team.as_ref().unwrap().session.unread(), 1);
    assert!(a.app.team_release(Some(&[w1.0])));
    b.frame_after(4.0);
    b.app.team_reserve(&[w1.0]);
    b.key(Key::Delete);
    assert!(b.model().wall(w1).is_none(), "now it is hers to delete");
    // Saving keeps her working copy in the folder.
    b.app.save(false);
    assert!(dir.join("trabajo").join("ana.archisim").is_file());
    let _ = std::fs::remove_dir_all(&dir);
}

/// Formas: a cylinder and a sphere drawn by clicks, a block, and the
/// operations offered with several volumes selected.
#[test]
fn round_forms_drawn_and_combined() {
    use super::concept_ui::ConceptPick;
    use super::toolbox::Pick;
    use crate::editor::MassMode;
    let mut h = Harness::new(Editor::new_project());
    let canvas = h.app.canvas;
    h.app
        .view
        .fit_exact(canvas, p(-5.0, -5.0), p(25.0, 15.0), 0.0);
    h.app.pick(Pick::Concept(ConceptPick::Form));
    assert_eq!(h.app.tool, Tool::Mass);
    assert_eq!(h.app.mass_settings.mode, MassMode::Cylinder);
    h.click_at(p(5.0, 5.0));
    h.click_at(p(7.0, 5.0));
    let cyl = h.model().masses().next().unwrap().id;
    assert!(matches!(
        h.model().mass(cyl).unwrap().shape,
        archisim_core::MassShape::Primitive(archisim_core::Primitive::Cylinder { .. })
    ));
    h.app.set_mass_mode(MassMode::Sphere);
    h.click_at(p(15.0, 5.0));
    h.click_at(p(16.0, 5.0));
    assert_eq!(h.model().masses().count(), 2);
    // A revolution: the axis, then the profile laid flat.
    h.app.set_mass_mode(MassMode::Revolution);
    h.click_at(p(20.0, 0.0));
    h.click_at(p(21.0, 0.0));
    h.click_at(p(21.5, 2.0));
    h.click_at(p(21.5, 2.0));
    assert_eq!(h.model().masses().count(), 3);
    // Both round forms selected: the operations show; joined, one solid.
    h.key(Key::Escape);
    let ids: Vec<_> = h.model().masses().map(|m| m.id).collect();
    h.app.ed.selection = [Element::Mass(ids[0]), Element::Mass(ids[1])]
        .into_iter()
        .collect();
    h.frame();
    h.frame();
    h.app
        .ed
        .combine_masses(ids[0], &[ids[1]], archisim_geom::csg::Op::Union)
        .unwrap();
    assert_eq!(h.model().masses().count(), 2);
    h.app.ed.selection = [Element::Mass(ids[0])].into_iter().collect();
    h.frame();
    h.app.mode = ViewMode::ThreeD;
    h.frame();
    h.frame();
}

/// The onboarding missions, done by clicking the control each step rings:
/// the first wall, doors and windows, a slab, the 3D, a level and a sheet.
/// Each step moves on by itself and the missions done are kept.
#[test]
fn onboarding_missions_by_clicks() {
    use super::missions_ui::marked;
    let mut h = Harness::new(Editor::new_project());
    let click_mark = |h: &mut Harness, key: &str| {
        h.frame();
        let r = marked(&h.ctx, key).unwrap_or_else(|| panic!("no control marked «{key}»"));
        h.click_screen(r.center());
    };
    let step = |h: &Harness| h.app.missions.progress.as_ref().map(|p| p.step);
    let target = |h: &Harness| {
        h.app
            .missions
            .progress
            .as_ref()
            .and_then(|p| p.current())
            .map(|s| s.target)
    };
    // The first wall: Arquitectura is already chosen, so it starts at the
    // tile.
    h.app.start_mission(0);
    h.frame();
    assert_eq!(target(&h), Some("tile:Muro"));
    click_mark(&mut h, "tile:Muro");
    assert_eq!(h.app.tool, Tool::Wall);
    assert_eq!(target(&h), Some("method:Rectángulo"));
    click_mark(&mut h, "method:Rectángulo");
    assert_eq!(step(&h), Some(3));
    h.click_at(p(0.0, 0.0));
    h.click_at(p(8.0, 6.0));
    assert_eq!(h.model().walls().count(), 4);
    assert!(h.app.missions.progress.as_ref().unwrap().finished());
    assert_eq!(h.app.prefs.missions_done, ["muro"]);
    // Doors and windows.
    h.app.start_mission(1);
    click_mark(&mut h, "tile:Puerta");
    h.click_at(p(4.0, 0.0));
    click_mark(&mut h, "tile:Ventana");
    h.click_at(p(4.0, 6.0));
    assert!(
        h.app.missions.progress.as_ref().unwrap().finished(),
        "{:?}",
        step(&h)
    );
    // A slab with the wand.
    h.app.start_mission(2);
    click_mark(&mut h, "tile:Forjado");
    click_mark(&mut h, "method:Varita");
    h.click_at(p(3.0, 3.0));
    assert_eq!(h.model().slabs().count(), 1);
    assert!(h.app.missions.progress.as_ref().unwrap().finished());
    // The 3D, from the Project tab of the navigator, and back.
    h.key(Key::Escape);
    h.app.nav_tab = super::navigator::NavTab::Model;
    h.app.start_mission(3);
    click_mark(&mut h, "seg:Proyecto");
    click_mark(&mut h, "nav:Vista 3D");
    assert_eq!(h.app.mode, ViewMode::ThreeD);
    click_mark(&mut h, "tab:plan");
    assert!(h.app.missions.progress.as_ref().unwrap().finished());
    // A level above, opened from the navigator.
    h.app.start_mission(4);
    click_mark(&mut h, "button:Plantas…");
    assert!(h.app.show_levels);
    click_mark(&mut h, "button:Añadir planta encima");
    assert_eq!(h.model().levels().len(), 2);
    h.app.show_levels = false;
    click_mark(&mut h, "nav:planta-alta");
    assert!(
        h.app.missions.progress.as_ref().unwrap().finished(),
        "{:?}",
        step(&h)
    );
    // A sheet, with the ground floor dropped on it.
    h.app.start_mission(5);
    click_mark(&mut h, "button:+ Nuevo plano");
    assert_eq!(step(&h), Some(1));
    let ground = h.model().levels()[0].id;
    h.app.place_dropped(
        super::sheet_drop::Drop::Level(ground),
        "Planta baja",
        p(0.3, 0.3),
    );
    h.frame();
    assert!(h.app.missions.progress.as_ref().unwrap().finished());
    assert_eq!(
        h.app.prefs.missions_done.len(),
        crate::missions::MISSIONS.len()
    );
    // The card offers no next one; closing it ends the missions.
    h.app.missions.progress = None;
    h.frame();
    // A bigger text grows the whole window.
    h.app.prefs.text_size = crate::prefs::TextSize::Large;
    h.frame();
    h.frame();
    assert!((h.ctx.zoom_factor() - 1.2).abs() < 1e-3);
}

/// A road drawn by the edge of its sidewalk (chosen in the bar under the
/// tabs), its axis edited afterwards by dragging, adding and removing
/// points; a selected tree changes only among trees.
#[test]
fn roads_by_their_edge_edited_after_and_trees_among_trees() {
    use super::missions_ui::marked;
    use super::urban_ui::RoadReference;
    let mut h = Harness::new(Editor::new_project());
    let canvas = h.app.canvas;
    h.app
        .view
        .fit_exact(canvas, p(-10.0, -30.0), p(60.0, 20.0), 0.0);
    h.type_text("w");
    assert_eq!(h.app.tool, Tool::Road);
    h.frame();
    let left = marked(&h.ctx, "reference:0").expect("reference buttons in the bar");
    h.click_screen(left.center());
    assert_eq!(h.app.road_settings.reference, RoadReference::LeftEdge);
    h.click_at(p(0.0, 0.0));
    h.move_to(p(40.0, 0.0));
    h.click_at(p(40.0, 0.0));
    h.key(Key::Enter);
    let r = h.model().roads().next().unwrap().clone();
    let half = r.section.width() / 2.0;
    // Drawn left to right along its left edge: the road lies below.
    assert!((r.axis[0].y + half).abs() < 1e-6, "{:?}", r.axis);
    assert!((r.length() - 40.0).abs() < 1e-6);
    // Edited after: the first point of the axis dragged.
    h.key(Key::Escape);
    h.app.ed.selection = [Element::Road(r.id)].into_iter().collect();
    h.frame();
    let a0 = r.axis[0];
    h.drag(a0, a0 + archisim_core::Vec2::new(0.0, -5.0));
    let axis = h.model().road(r.id).unwrap().axis.clone();
    assert!((axis[0].y - (a0.y - 5.0)).abs() < 1e-6, "{axis:?}");
    // A double click on the axis adds a point; picked, Supr takes it away.
    let mid = p(25.0, -half);
    h.move_to(mid);
    for _ in 0..2 {
        h.button(PointerButton::Primary, true);
        h.button(PointerButton::Primary, false);
    }
    h.time += 1.0;
    h.frame();
    assert_eq!(h.model().road(r.id).unwrap().axis.len(), 3);
    h.key(Key::Delete);
    assert_eq!(h.model().road(r.id).unwrap().axis.len(), 2);
    assert!(h.model().road(r.id).is_some(), "the road stays");
    // A tree: its gallery keeps to vegetation, people and vehicles.
    h.key(Key::Escape);
    h.app.pick(super::toolbox::Pick::Objects(
        super::object_gallery::ObjectScope::Vegetation,
    ));
    h.click_at(p(10.0, 10.0));
    let tree = h.model().objects().next().unwrap().id;
    h.key(Key::Escape);
    h.app.ed.selection = [Element::Object(tree)].into_iter().collect();
    h.frame();
    let t = h.model().object(tree).unwrap().object_type;
    assert_eq!(h.app.object_type_category(t), Some("Entorno"));
    assert_eq!(
        super::object_gallery::ObjectScope::of_category("Entorno"),
        Some(super::object_gallery::ObjectScope::Vegetation)
    );
    assert_eq!(
        super::object_gallery::ObjectScope::of_category("Mobiliario"),
        None
    );
}

/// Block 2 of Antonio's review: a level's look kept in the model (and
/// undone), a saved ceiling plan, a reference plane walls snap to, the
/// window «Visibilidad y gráficos» and the AD5 template.
#[test]
fn views_graphics_and_reference_planes() {
    use archisim_core::{Category, GraphicOverride, LayerLines, PlanKind};
    let mut h = Harness::new(Editor::new_project());
    let canvas = h.app.canvas;
    h.app
        .view
        .fit_exact(canvas, p(-2.0, -2.0), p(14.0, 10.0), 0.0);
    let level = h.app.ed.level;
    // Walls white with their cavity only: kept with the level's plan.
    let mut style = h.app.view_style.clone();
    style.set_graphics(
        Category::Walls,
        GraphicOverride {
            cut_fill: Some([255, 255, 255]),
            patterns: false,
            layer_lines: LayerLines::Cavity,
            ..Default::default()
        },
    );
    h.app.view_style = style;
    h.frame();
    h.frame();
    assert_eq!(
        h.model()
            .level_style(level)
            .graphics(Category::Walls)
            .layer_lines,
        LayerLines::Cavity
    );
    // Undone, the view goes back too.
    h.app.ed.undo();
    h.frame();
    h.frame();
    assert!(h.app.view_style.overrides.is_empty());
    // The AD5 template from the starter library.
    let ad5 = h
        .model()
        .view_templates()
        .find(|t| t.name == "AD5")
        .map(|t| t.id)
        .expect("AD5 template");
    h.app.apply_template(ad5);
    h.frame();
    h.frame();
    assert_eq!(
        h.model()
            .level_style(level)
            .graphics(Category::Slabs)
            .surface_fill,
        Some([255, 255, 255])
    );
    // The window opens and draws every category.
    h.app.views.vg_open = true;
    h.frame();
    h.frame();
    h.app.views.vg_open = false;
    // A ceiling plan, saved and shown in its own tab.
    h.app.duplicate_view(PlanKind::Ceiling);
    h.frame();
    let id = h.app.active_saved().expect("a saved view");
    assert!(h.app.active_ceiling());
    assert_eq!(
        h.model().plan_view(id).unwrap().name,
        "Planta baja — techos"
    );
    // Its own scale, kept in the model.
    h.app.set_scale(100.0);
    h.frame();
    h.frame();
    assert_eq!(h.model().plan_view(id).unwrap().scale, 100.0);
    // Back to the level's own plan: its scale was 1:50.
    h.app.open_view(super::tabs::TabKind::Plan(level));
    h.frame();
    assert!(h.app.active_saved().is_none());
    // A reference plane, drawn with its tool.
    h.app.set_tool(Tool::RefPlane);
    h.click_at(p(4.0, -1.0));
    h.click_at(p(4.0, 9.0));
    assert_eq!(h.model().ref_planes().count(), 1);
    h.key(Key::Escape);
    h.key(Key::Escape);
    // A wall started near it starts on it.
    h.type_text("m");
    h.click_at(p(4.03, 2.0));
    h.click_at(p(10.0, 2.0));
    h.key(Key::Escape);
    let w = h.model().walls().next().expect("a wall");
    assert!((w.start.x - 4.0).abs() < 1e-6, "{:?}", w.start);
    // The sans lettering and the technical one both draw.
    h.app
        .ed
        .transact("Letra", |m| {
            m.graphics.font = archisim_core::TextFont::Technical;
            Ok(())
        })
        .unwrap();
    h.frame();
}

/// Bloque 3 de la revisión del 22/10/2026: before the click, the plan
/// shows the real drawing of what would be placed (a door in its cut
/// wall), and while dragging, what moves where it goes; the start screen
/// keeps the files opened and saved, the last first.
#[test]
fn plan_shows_what_is_placed_or_dragged() {
    let mut h = Harness::new(Editor::new_project());
    assert!(
        !h.app.start.open,
        "the tests start without the start screen"
    );
    draw_outline(&mut h);
    h.key(Key::Escape);
    h.key(Key::Escape);
    h.move_to(p(6.0, 4.0));
    assert!(super::ghost3d::plan_ghost_region(&h.app).is_none());
    // A door over the south façade.
    h.type_text("p");
    assert_eq!(h.app.tool, Tool::Door);
    h.move_to(p(6.0, 0.2));
    let (lo, hi) = super::ghost3d::plan_ghost_region(&h.app).expect("the door's ghost");
    assert!(
        lo.x < 5.5 && hi.x > 6.5 && lo.y < 0.0 && hi.y > 0.2,
        "{lo:?} {hi:?}"
    );
    assert!(super::ghost3d::plan_ghost_drawn(&h.app));
    h.click_at(p(6.0, 0.2));
    assert_eq!(h.model().openings().count(), 1);
    h.key(Key::Escape);
    h.key(Key::Escape);
    h.move_to(p(6.0, 4.0));
    assert!(super::ghost3d::plan_ghost_region(&h.app).is_none());
    // Dragging the door along its wall: the ghost covers where it was and
    // where it goes.
    h.click_at(p(6.0, 0.0));
    let door = h.model().openings().next().unwrap().id;
    assert!(h.app.ed.selection.contains(&Element::Opening(door)));
    h.move_to(p(6.0, 0.0));
    h.button(PointerButton::Primary, true);
    for x in [6.5, 7.5, 8.5, 9.0] {
        h.move_to(p(x, 0.0));
    }
    let (lo, hi) = super::ghost3d::plan_ghost_region(&h.app).expect("the drag's ghost");
    assert!(lo.x < 5.6 && hi.x > 9.4, "{lo:?} {hi:?}");
    h.button(PointerButton::Primary, false);
    h.frame();
    assert!(super::ghost3d::plan_ghost_region(&h.app).is_none());
    // The recent files.
    let mut recent = Vec::new();
    for f in ["/a.archisim", "/b.ifc", "/a.archisim"] {
        super::start_ui::remember(&mut recent, std::path::Path::new(f));
    }
    assert_eq!(recent, ["/a.archisim", "/b.ifc"]);
}

/// Bloque 4 de la revisión del 22/10/2026: a side of a volume pushed out
/// in plan (it stays a contour), its top divided and one part pushed up,
/// the volume twisted with a typed angle, a band with a tube's profile,
/// edges rounded from the contextual bar; all saved and opened again.
#[test]
fn concept_faces_divided_pushed_twisted_and_profiles() {
    use super::concept_ui::ConceptPick;
    use super::toolbox::{Discipline, Pick};
    use archisim_core::MassShape;
    let mut h = Harness::new(Editor::new_project());
    let canvas = h.app.canvas;
    h.app
        .view
        .fit_exact(canvas, p(-5.0, -12.0), p(30.0, 16.0), 0.0);
    h.app.set_discipline(Discipline::Concept);
    h.app.pick(Pick::Concept(ConceptPick::Line));
    h.app.choose_method("Rectángulo");
    h.click_at(p(0.0, 0.0));
    h.click_at(p(10.0, 8.0));
    let id = h.model().masses().next().unwrap().id;
    h.app.pick(Pick::Concept(ConceptPick::Push));
    h.click_at(p(5.0, 4.0));
    h.move_to(p(5.0, 8.0));
    h.click_at(p(5.0, 8.0));
    assert!((h.model().mass(id).unwrap().height - 4.0).abs() < 1e-6);
    // The east side, taken by its edge and pulled 2 m out.
    h.click_at(p(10.0, 4.0));
    assert!(h.app.concept.face_push.is_some(), "{:?}", h.app.message);
    h.move_to(p(12.0, 4.0));
    h.click_at(p(12.0, 4.0));
    let m = h.model().mass(id).unwrap();
    assert!(matches!(m.shape, MassShape::Prism(_)));
    assert!((archisim_core::signed_area(&m.outline()) - 96.0).abs() < 1e-6);
    // Dividir: a line across the top at x = 3.
    h.app.pick(Pick::Concept(ConceptPick::Divide));
    assert_eq!(h.app.tool, Tool::DivideFace);
    h.click_at(p(3.0, 1.0));
    h.click_at(p(3.0, 7.0));
    let m = h.model().mass(id).unwrap();
    let MassShape::Mesh(mesh) = &m.shape else {
        panic!("a solid now: {:?}", h.app.message);
    };
    assert_eq!(mesh.face_ids().len(), 7);
    // The eastern part pushed up 1 m.
    h.app.pick(Pick::Concept(ConceptPick::Push));
    h.click_at(p(8.0, 4.0));
    assert!(h.app.concept.face_push.is_some());
    h.move_to(p(8.0, 5.0));
    h.click_at(p(8.0, 5.0));
    assert!((h.model().mass(id).unwrap().height - 5.0).abs() < 1e-6);
    // Deformar: 45° typed.
    h.app.pick(Pick::Concept(ConceptPick::Deform));
    h.click_at(p(5.0, 4.0));
    h.type_text("45");
    h.key(Key::Enter);
    let d = h.model().mass(id).unwrap().deform.expect("twisted");
    assert!((d.twist.to_degrees() - 45.0).abs() < 1e-6);
    // Seguir with a tube.
    h.app.pick(Pick::Concept(ConceptPick::Follow));
    h.app.mass_settings.profile = crate::editor::SweepProfile::Preset(0);
    h.click_at(p(0.0, -4.0));
    h.click_at(p(10.0, -4.0));
    h.click_at(p(10.0, -8.0));
    h.key(Key::Enter);
    assert!(h.model().masses().any(|m| matches!(
        &m.shape,
        MassShape::Sweep {
            profile: Some(_),
            ..
        }
    )));
    // A face pushed into a volume, its edges rounded.
    h.app.pick(Pick::Concept(ConceptPick::Line));
    h.app.choose_method("Rectángulo");
    h.click_at(p(16.0, 0.0));
    h.click_at(p(22.0, 6.0));
    let block = h.model().masses().last().unwrap().id;
    h.app
        .ed
        .edit_mass("Altura", block, |x| x.height = 3.0)
        .unwrap();
    h.key(Key::Escape);
    h.key(Key::Escape);
    h.app.ed.selection = [Element::Mass(block)].into_iter().collect();
    h.app.round_selected_edges();
    assert!(h.model().mass(block).unwrap().finish.is_some());
    // Saved and opened, the same.
    let dir = std::env::temp_dir().join(format!("archisim-sculpt-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = h.app.ed.save(Some(&dir.join("formas.archisim"))).unwrap();
    let reopened = Editor::open(&path).unwrap();
    assert_eq!(reopened.model(), h.model());
    let _ = std::fs::remove_dir_all(&dir);
}

/// Bloque 5: a wardrobe stretched by its handles in the 3D view, wider
/// and taller; its drawers and handles come from its family.
#[test]
fn objects_are_stretched_in_the_3d_view() {
    use super::grips3d::SizeKey;
    let mut h = Harness::new(Editor::new_project());
    let level = h.app.ed.level;
    let t = h
        .model()
        .object_types()
        .find(|t| t.name == "Armario 120")
        .expect("the library's wardrobe")
        .id;
    let id = h
        .app
        .ed
        .transact("Armario", |m| {
            m.add_object(archisim_core::NewObject {
                level,
                object_type: t,
                position: p(2.0, 2.0),
                rotation: 0.0,
                base_offset: 0.0,
            })
        })
        .unwrap();
    h.app.ed.selection = [Element::Object(id)].into_iter().collect();
    h.key(Key::F3);
    h.frame();
    h.frame();
    assert_eq!(h.app.mode, ViewMode::ThreeD);
    let handles = h.app.object_handles_3d();
    assert_eq!(handles.len(), 4, "{handles:?}");
    let drag = |h: &mut Harness, key: SizeKey, to: glam::DVec3| {
        let at = h
            .app
            .object_handles_3d()
            .into_iter()
            .find(|x| x.1 == key)
            .unwrap()
            .2;
        let from = screen_3d(h, at.x, at.y, at.z);
        let end = screen_3d(h, to.x, to.y, to.z);
        h.pos = from;
        h.events.push(Event::PointerMoved(from));
        h.frame();
        h.button(PointerButton::Primary, true);
        for k in 1..=6 {
            let q = from + (end - from) * (k as f32 / 6.0);
            h.pos = q;
            h.events.push(Event::PointerMoved(q));
            h.frame();
        }
        h.button(PointerButton::Primary, false);
        h.frame();
        at
    };
    // The right side 0,3 m further: 0,6 m wider (it stays centred).
    let at = h
        .app
        .object_handles_3d()
        .into_iter()
        .find(|x| x.1 == SizeKey::Width(true))
        .unwrap()
        .2;
    drag(
        &mut h,
        SizeKey::Width(true),
        at + glam::DVec3::new(0.3, 0.0, 0.0),
    );
    let ty = h
        .model()
        .object_type(h.model().object(id).unwrap().object_type)
        .unwrap();
    let width = match ty.values.get("width") {
        Some(archisim_core::Value::Number(v)) => *v,
        other => panic!("{other:?}"),
    };
    assert!((width - 1.8).abs() < 0.03, "{width} {:?}", h.app.message);
    // The top 0,2 m up.
    let at = h
        .app
        .object_handles_3d()
        .into_iter()
        .find(|x| x.1 == SizeKey::Height)
        .unwrap()
        .2;
    drag(
        &mut h,
        SizeKey::Height,
        at + glam::DVec3::new(0.0, 0.0, 0.2),
    );
    let ty = h
        .model()
        .object_type(h.model().object(id).unwrap().object_type)
        .unwrap();
    let height = match ty.values.get("height") {
        Some(archisim_core::Value::Number(v)) => *v,
        other => panic!("{other:?}"),
    };
    assert!((height - 2.6).abs() < 0.03, "{height}");
}
