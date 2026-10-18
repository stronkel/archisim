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
    reopened.export_ifc(&dir.join("sesion.ifc")).unwrap();
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
    // Supr deletes from the 3D view too; a tool key goes back to the plan.
    h.key(Key::Delete);
    assert!(h.model().wall(north).is_none());
    h.type_text("m");
    assert_eq!(h.app.mode, ViewMode::Plan);
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
    h.app.ed.export_ifc(&dir.join("edicion.ifc")).unwrap();
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
    h.app.ed.export_ifc(&dir.join("plantas.ifc")).unwrap();
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
        h.frame();
        h.frame();
        let key = &h.app.types.preview.as_ref().expect("a preview").0;
        let opening = matches!(tab, TypesTab::Doors | TypesTab::Windows);
        assert_eq!(key.starts_with("ot:"), opening, "preview of {tab:?}");
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
    reopened.export_ifc(&dir.join("tipos.ifc")).unwrap();
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
    // A tool key from the 3D view draws in the last plan too.
    h.key(Key::F3);
    h.type_text("m");
    assert_eq!((h.app.mode, h.app.tool), (ViewMode::Plan, Tool::Wall));
    h.key(Key::Escape);
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
    h.app.pick(super::toolbox::Pick::Later("2.5"));
    assert_eq!(h.app.tool, Tool::Arrow);
    assert!(h.app.message.as_ref().unwrap().text.contains("2.5"));
    // Every tile of the box has a name and a hint; tools have their key.
    for (_, tiles) in super::toolbox::SECTIONS {
        for t in *tiles {
            assert!(!t.name.is_empty() && !t.hint.is_empty());
            if let super::toolbox::Pick::Tool(tool) = t.pick {
                if tool != Tool::Arrow {
                    assert_eq!(Tool::from_key(t.key), Some(tool), "{}", t.name);
                }
            }
        }
    }
    // The compact box and the hidden dock still leave a working plan.
    h.app.compact_toolbox = true;
    h.app.show_properties = false;
    h.app.show_navigator = false;
    h.frame();
    h.frame();
    draw_outline(&mut h);
    assert_eq!(h.model().walls().count(), 4);
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
