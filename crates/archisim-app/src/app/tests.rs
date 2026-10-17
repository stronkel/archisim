//! The whole window driven without a screen: mouse and keyboard events go
//! in, the model comes out. This is how an architect would draw, click by
//! click.

use super::*;
use archisim_core::SlabFunction;
use archisim_geom::{wall_geometry, EndCut};
use eframe::egui::{Event, Modifiers, PointerButton, RawInput};

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
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(1280.0, 800.0))),
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
