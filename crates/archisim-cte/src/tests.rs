//! An office floor: three offices on a corridor with an exit at one end.

use archisim_core::*;

use crate::*;

struct Office {
    m: Model,
    level: LevelId,
    corridor_wall: WallId,
    west: WallId,
    east: WallId,
    lib: archisim_library::Library,
}

fn wall(m: &mut Model, t: WallTypeId, level: LevelId, a: (f64, f64), b: (f64, f64)) -> WallId {
    m.add_wall(NewWall {
        wall_type: t,
        level,
        start: Point2::new(a.0, a.1),
        end: Point2::new(b.0, b.1),
        bulge: 0.0,
        reference: ReferenceLine::Center,
        height: 3.0,
        base_offset: 0.0,
        top: WallTop::Unconnected,
    })
    .unwrap()
}

#[allow(clippy::too_many_arguments)]
fn room(
    m: &mut Model,
    level: LevelId,
    n: &str,
    usage: &str,
    x0: f64,
    y0: f64,
    x1: f64,
    y1: f64,
) -> RoomId {
    let id = m
        .add_room(NewRoom {
            level,
            number: n.into(),
            name: usage.into(),
            outline: vec![
                Point2::new(x0, y0),
                Point2::new(x1, y0),
                Point2::new(x1, y1),
                Point2::new(x0, y1),
            ],
            holes: Vec::new(),
            label: Point2::new((x0 + x1) / 2.0, (y0 + y1) / 2.0),
            bounded: false,
        })
        .unwrap();
    let mut r = m.room(id).unwrap().clone();
    r.usage = usage.into();
    m.set_room(r).unwrap();
    id
}

fn door(
    m: &mut Model,
    t: OpeningTypeId,
    wall: WallId,
    along: f64,
    opens_to: WallSide,
) -> OpeningId {
    m.add_opening(NewOpening {
        opening_type: t,
        wall,
        along,
        sill: 0.0,
        opens_to,
        hand: Hand::Left,
        setback: None,
    })
    .unwrap()
}

/// 24 × 6 m: offices of 8 × 4 to the south, a corridor of 2 m to the
/// north, a door out at the west end of the corridor.
fn office() -> Office {
    let mut m = Model::new("Oficinas");
    let level = m.add_level("Planta baja", 0.0);
    let lib = archisim_library::add_library(&mut m).unwrap();
    m.set_code_settings(CodeSettings {
        building_use: BuildingUse::Administrative,
        accessibility: Accessibility::Routes,
        sprinklers: false,
    });
    let t = lib.partition;
    wall(&mut m, t, level, (0.0, 0.0), (24.0, 0.0));
    let east = wall(&mut m, t, level, (24.0, 0.0), (24.0, 6.0));
    wall(&mut m, t, level, (24.0, 6.0), (0.0, 6.0));
    let west = wall(&mut m, t, level, (0.0, 6.0), (0.0, 0.0));
    let corridor_wall = wall(&mut m, t, level, (0.0, 4.0), (24.0, 4.0));
    wall(&mut m, t, level, (8.0, 0.0), (8.0, 4.0));
    wall(&mut m, t, level, (16.0, 0.0), (16.0, 4.0));
    let h = 0.05;
    for (i, x) in [0.0, 8.0, 16.0].iter().enumerate() {
        room(
            &mut m,
            level,
            &format!("{}", i + 1),
            "Despacho",
            x + h,
            h,
            x + 8.0 - h,
            4.0 - h,
        );
        door(
            &mut m,
            lib.interior_door,
            corridor_wall,
            x + 4.0,
            WallSide::Interior,
        );
    }
    room(&mut m, level, "4", "Pasillo", h, 4.0 + h, 24.0 - h, 6.0 - h);
    door(&mut m, lib.entrance_door, west, 1.0, WallSide::Interior);
    Office {
        m,
        level,
        corridor_wall,
        west,
        east,
        lib,
    }
}

fn find<'a>(r: &'a Report, rule: &str, subject: Subject) -> Vec<&'a Item> {
    r.items
        .iter()
        .filter(|i| i.rule.starts_with(rule) && i.subject == subject)
        .collect()
}

#[test]
fn occupancy_and_areas() {
    let o = office();
    let r = check(&o.m);
    // Three offices of 7,9 × 3,9 at 10 m²/person; the corridor counts none.
    let offices: Vec<_> = r.rooms.iter().filter(|x| x.usage == "Despacho").collect();
    assert_eq!(offices.len(), 3);
    assert!((offices[0].people - 7.9 * 3.9 / 10.0).abs() < 1e-9);
    let corridor = r.rooms.iter().find(|x| x.usage == "Pasillo").unwrap();
    assert_eq!(corridor.per_person, None);
    assert_eq!(
        r.levels[0].people,
        (3.0f64 * 7.9 * 3.9 / 10.0).ceil() as u32
    );
    // Built area: inside the outer faces of the walls, 24,1 × 6,1.
    assert!(
        (r.levels[0].built - 24.1 * 6.1).abs() < 1e-6,
        "{}",
        r.levels[0].built
    );
    let text = r.to_text(&o.m);
    assert!(text.contains("Evacuación (DB-SI 3)"), "{text}");
}

#[test]
fn one_exit_too_far_then_two() {
    let mut o = office();
    let r = check(&o.m);
    let route = r
        .items
        .iter()
        .find(|i| i.rule == "SI 3, tabla 3.1" && !i.route.is_empty())
        .expect("a route");
    // From the far corner of the east office: its door at x = 20, then
    // 20 m along the corridor.
    assert_eq!(route.status, Status::Fails, "{route:?}");
    assert!(route.text.contains("1 salida"), "{}", route.text);
    // A second exit at the east end: two exits, 50 m.
    door(
        &mut o.m,
        o.lib.entrance_door,
        o.east,
        5.0,
        WallSide::Interior,
    );
    let r = check(&o.m);
    let route = r
        .items
        .iter()
        .find(|i| i.rule == "SI 3, tabla 3.1" && !i.route.is_empty())
        .unwrap();
    assert_eq!(route.status, Status::Passes, "{route:?}");
    assert!(route.text.contains("2 salidas"), "{}", route.text);
    let _ = (o.level, o.west, o.corridor_wall);
}

#[test]
fn doors_corridors_and_turning() {
    let o = office();
    let r = check(&o.m);
    // Every door on the route is checked for its width.
    let widths: Vec<_> = r
        .items
        .iter()
        .filter(|i| i.rule == "SI 3, tabla 4.1 (puertas y pasos)")
        .collect();
    assert_eq!(widths.len(), 4, "{widths:#?}");
    // The corridor: 1,90 m, enough for evacuation and accessibility.
    let corridor = o.m.rooms().find(|x| x.usage == "Pasillo").unwrap().id;
    let c = find(&r, "SUA, Anejo A (pasillos)", Subject::Room(corridor));
    assert_eq!(c.len(), 1);
    assert_eq!(c[0].status, Status::Passes, "{:?}", c[0]);
    assert!(c[0].value.starts_with("1,90"), "{}", c[0].value);
    let c = find(&r, "SI 3, tabla 4.1 (pasillos)", Subject::Room(corridor));
    assert_eq!(c[0].status, Status::Passes);
    // Doors of an accessible route: a hollow-core door of the template.
    let accessible: Vec<_> = r
        .items
        .iter()
        .filter(|i| i.rule == "SUA, Anejo A (puertas)")
        .collect();
    assert!(!accessible.is_empty());
}

#[test]
fn a_room_with_no_door_cannot_be_left() {
    let mut o = office();
    let l = o.level;
    room(&mut o.m, l, "5", "Despacho", 30.0, 0.0, 34.0, 4.0);
    let r = check(&o.m);
    assert!(r
        .items
        .iter()
        .any(|i| i.status == Status::Fails && i.text.contains("No hay recorrido")));
}

#[test]
fn turning_circles_count_the_furniture() {
    let mut m = archisim_library::new_project("Vivienda");
    let level = m.levels()[0].id;
    m.set_code_settings(CodeSettings {
        building_use: BuildingUse::Dwelling,
        accessibility: Accessibility::WheelchairDwelling,
        sprinklers: false,
    });
    let bed = room(&mut m, level, "1", "Dormitorio", 0.0, 0.0, 3.0, 3.0);
    let r = check(&m);
    let t = find(&r, "SUA, Anejo A", Subject::Room(bed));
    assert_eq!(t[0].status, Status::Passes, "{:?}", t[0]);
    // A double bed in the middle of the room leaves no circle of 1,50.
    let objs = archisim_library::add_objects(&mut m).unwrap();
    let (_, bed_type) = objs
        .iter()
        .find(|(n, _)| n.to_lowercase().contains("cama"))
        .expect("a bed in the library");
    let (lo, hi) = m.object_type_box(*bed_type).unwrap();
    let centre = Point2::new((lo[0] + hi[0]) / 2.0, (lo[1] + hi[1]) / 2.0);
    m.add_object(NewObject {
        level,
        object_type: *bed_type,
        position: Point2::new(1.5, 1.5) - centre,
        rotation: 0.0,
        base_offset: 0.0,
    })
    .unwrap();
    let r = check(&m);
    let t = find(&r, "SUA, Anejo A", Subject::Room(bed));
    assert_eq!(
        t[0].status,
        Status::Fails,
        "a bed in the middle: {:?}",
        t[0]
    );
}

/// A room for more than 50 people: its door must be hinged and open the
/// way out (SI 3, 6).
#[test]
fn doors_of_crowded_rooms_open_outwards() {
    let results: Vec<bool> = [WallSide::Interior, WallSide::Exterior]
        .into_iter()
        .map(|side| {
            let mut m = Model::new("Sala");
            let level = m.add_level("Planta baja", 0.0);
            let lib = archisim_library::add_library(&mut m).unwrap();
            m.set_code_settings(CodeSettings {
                building_use: BuildingUse::Assembly,
                ..Default::default()
            });
            let t = lib.partition;
            let south = wall(&mut m, t, level, (0.0, 0.0), (12.0, 0.0));
            wall(&mut m, t, level, (12.0, 0.0), (12.0, 8.0));
            wall(&mut m, t, level, (12.0, 8.0), (0.0, 8.0));
            wall(&mut m, t, level, (0.0, 8.0), (0.0, 0.0));
            room(&mut m, level, "1", "Comedor", 0.05, 0.05, 11.95, 7.95);
            door(&mut m, lib.double_door, south, 6.0, side);
            let r = check(&m);
            let people = r.levels[0].people;
            assert!(people > 50, "{people}");
            !r.items.iter().any(|i| i.rule == "SI 3, 6.3")
        })
        .collect();
    // The walls go round anticlockwise, so their «exterior» face is the
    // room's side: opening to the walls' interior side opens out.
    assert_eq!(results, vec![true, false]);
}

/// On an upper floor the exit is the head of the stair down.
#[test]
fn stairs_are_the_exits_of_upper_floors() {
    let mut o = office();
    let first = o.m.add_level("Planta primera", 3.5);
    let material = o.m.materials().next().unwrap().id;
    // A stair in the corridor, from x = 2 up to x = 8.
    o.m.add_stair(NewStair {
        level: o.level,
        path: vec![Point2::new(2.0, 5.0), Point2::new(8.0, 5.0)],
        width: 1.0,
        going: 0.28,
        base_offset: 0.0,
        top: WallTop::Level {
            level: first,
            offset: 0.0,
        },
        height: 3.5,
        waist: 0.15,
        material,
        usage: StairUse::General,
        design: Default::default(),
    })
    .unwrap();
    // Upstairs, one office of 16 × 6 over the corridor and offices.
    room(&mut o.m, first, "11", "Despacho", 0.05, 0.05, 15.95, 5.95);
    let r = check(&o.m);
    let up = r
        .items
        .iter()
        .find(|i| i.rule == "SI 3, tabla 3.1" && i.level == Some(first))
        .expect("a route upstairs");
    assert_eq!(up.status, Status::Passes, "{up:?}");
    assert!(up.text.contains("Escalera"), "{}", up.text);
    // The stair itself is checked.
    assert!(r
        .items
        .iter()
        .any(|i| i.section == Section::StairsAndRamps && matches!(i.subject, Subject::Stair(_))));
}

#[test]
fn sanitary_pipes_by_hs4_and_hs5() {
    let mut m = archisim_library::new_project("Baño");
    let l = m.levels()[0].id;
    let wc = archisim_library::add_object_entry(&mut m, "Inodoro").unwrap();
    let basin = archisim_library::add_object_entry(&mut m, "Lavabo 60").unwrap();
    let place = |m: &mut Model, t, x: f64| {
        m.add_object(NewObject {
            level: l,
            object_type: t,
            position: Point2::new(x, 0.5),
            rotation: 0.0,
            base_offset: 0.0,
        })
        .unwrap()
    };
    let wc_id = place(&mut m, wc, 1.0);
    let basin_id = place(&mut m, basin, 4.0);
    assert_eq!(
        services::Appliance::of(&m, wc_id),
        Some(services::Appliance::Wc)
    );
    assert_eq!(
        services::Appliance::of(&m, basin_id),
        Some(services::Appliance::Washbasin)
    );
    let run = |m: &mut Model, system, x: f64, y: f64, width: f64, slope: f64| {
        m.add_run(NewRun {
            level: l,
            system,
            path: vec![Point2::new(x, y), Point2::new(x + 0.6, y)],
            elevation: 0.3,
            width,
            height: width,
            slope,
        })
        .unwrap()
    };
    // The WC drains through a 90 (too small); the basin through a 40 hung
    // flat (no slope).
    run(&mut m, ServiceSystem::Drainage, 0.7, 0.2, 0.09, 0.02);
    let flat = run(&mut m, ServiceSystem::Drainage, 3.7, 0.2, 0.04, 0.0);
    // Water: a 16 to the WC, a 10 (too small) to the basin, hot to neither.
    run(&mut m, ServiceSystem::ColdWater, 0.7, 0.9, 0.016, 0.0);
    run(&mut m, ServiceSystem::ColdWater, 3.7, 0.9, 0.010, 0.0);
    let r = check(&m);
    let of = |s: Subject, rule: &str| -> Vec<Status> {
        r.items
            .iter()
            .filter(|i| i.subject == s && i.rule == rule)
            .map(|i| i.status)
            .collect()
    };
    assert_eq!(
        of(Subject::Object(wc_id), "HS 5, tabla 4.1"),
        [Status::Fails]
    );
    assert_eq!(
        of(Subject::Object(basin_id), "HS 5, tabla 4.1"),
        [Status::Passes]
    );
    assert_eq!(
        of(Subject::Object(wc_id), "HS 4, tabla 4.2"),
        [Status::Passes]
    );
    assert_eq!(
        of(Subject::Object(basin_id), "HS 4, tabla 4.2"),
        [Status::Fails]
    );
    assert_eq!(of(Subject::Run(flat), "HS 5, 3.3.1.1"), [Status::Fails]);
    assert!(r.to_text(&m).contains("Agua y saneamiento"));
}

fn fire_device(m: &mut Model, level: LevelId, kind: DeviceKind, x: f64, y: f64) {
    let (width, depth, height, elevation) = kind.defaults();
    m.add_device(NewDevice {
        level,
        kind,
        name: kind.name().into(),
        position: Point2::new(x, y),
        rotation: 0.0,
        elevation,
        width,
        depth,
        height,
    })
    .unwrap();
}

/// DB-SI 4: no extinguisher fails the level; one at the west end leaves
/// the far office out of reach; a second one at the east end covers it.
/// Sprinklers declared need sprinklers drawn, enough for each room.
#[test]
fn extinguishers_and_sprinklers_by_si4() {
    let Office { mut m, level, .. } = office();
    let fire = |r: &Report| -> Vec<Item> {
        r.items
            .iter()
            .filter(|i| i.section == Section::Fire)
            .cloned()
            .collect()
    };
    let r = check(&m);
    assert!(fire(&r)
        .iter()
        .any(|i| i.subject == Subject::Level(level) && i.status == Status::Fails));
    fire_device(&mut m, level, DeviceKind::Extinguisher, 1.0, 5.8);
    let far = |r: &Report| {
        fire(r)
            .into_iter()
            .filter(|i| i.rule == "SI 4, tabla 1.1" && i.text.starts_with("Extintor"))
            .map(|i| i.status)
            .collect::<Vec<_>>()
    };
    assert!(far(&check(&m)).contains(&Status::Fails));
    fire_device(&mut m, level, DeviceKind::Extinguisher, 23.0, 5.8);
    let s = far(&check(&m));
    assert!(
        s.len() == 4 && s.iter().all(|x| *x == Status::Passes),
        "{s:?}"
    );
    // Sprinklers declared, none drawn.
    let mut c = m.code_settings();
    c.sprinklers = true;
    m.set_code_settings(c);
    assert!(fire(&check(&m))
        .iter()
        .any(|i| i.text.contains("extinción automática") && i.status == Status::Warning));
    // One in the first office (31 m²): two are needed.
    fire_device(&mut m, level, DeviceKind::Sprinkler, 4.0, 2.0);
    let counts = |r: &Report| {
        fire(r)
            .into_iter()
            .filter(|i| i.text.starts_with("Rociadores en") && i.required.starts_with('≥'))
            .map(|i| i.status)
            .collect::<Vec<_>>()
    };
    assert_eq!(counts(&check(&m)), [Status::Fails]);
    fire_device(&mut m, level, DeviceKind::Sprinkler, 2.0, 2.0);
    fire_device(&mut m, level, DeviceKind::Sprinkler, 6.0, 2.0);
    assert_eq!(counts(&check(&m)), [Status::Passes]);
}
