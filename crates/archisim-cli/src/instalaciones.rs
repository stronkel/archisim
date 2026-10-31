//! `archisim instalaciones`: the second round of the services (E13) on a
//! small two-storey house — the electrical mechanisms of every kind with
//! their plan symbols and the circuits of the REBT (C1 lights, C2 sockets,
//! C3 the cooker, C5 the bathroom and kitchen), the board, water pipes
//! with bends of their radius, a riser and a downpipe through both floors
//! with the drains ending at it.

use std::path::Path;

use archisim_core::{
    DeviceKind, Model, NewDevice, NewRoom, NewRun, NewWall, Point2, ReferenceLine, ServiceSystem,
    WallTop,
};

type Res<T = ()> = Result<T, Box<dyn std::error::Error>>;

pub fn build() -> Res<Model> {
    let mut m = Model::new("Instalaciones de ArchiSim");
    let ground = m.add_level("Planta baja", 0.0);
    let first = m.add_level("Planta primera", 3.0);
    let lib = archisim_library::add_library(&mut m)?;
    let (w, d) = (8.0, 6.0);
    let c = [
        Point2::new(0.0, 0.0),
        Point2::new(0.0, d),
        Point2::new(w, d),
        Point2::new(w, 0.0),
    ];
    for level in [ground, first] {
        for k in 0..4 {
            m.add_wall(NewWall {
                wall_type: lib.facade,
                level,
                start: c[k],
                end: c[(k + 1) % 4],
                reference: ReferenceLine::ExteriorFace,
                height: 3.0,
                base_offset: 0.0,
                bulge: 0.0,
                top: WallTop::Unconnected,
            })?;
        }
    }
    // Two rooms downstairs: the kitchen and the living room.
    let room = |m: &mut Model, level, name: &str, a: Point2, b: Point2| -> Res {
        m.add_room(NewRoom {
            level,
            number: String::new(),
            name: name.into(),
            outline: vec![a, Point2::new(b.x, a.y), b, Point2::new(a.x, b.y)],
            holes: Vec::new(),
            label: a + (b - a) * 0.5,
            bounded: false,
        })?;
        Ok(())
    };
    room(
        &mut m,
        ground,
        "Cocina",
        Point2::new(0.3, 0.3),
        Point2::new(3.5, 5.7),
    )?;
    room(
        &mut m,
        ground,
        "Salón",
        Point2::new(3.6, 0.3),
        Point2::new(7.7, 5.7),
    )?;
    room(
        &mut m,
        first,
        "Baño",
        Point2::new(0.3, 0.3),
        Point2::new(3.0, 3.0),
    )?;
    let device = |m: &mut Model, level, kind: DeviceKind, x: f64, y: f64, rot: f64| -> Res {
        let (dw, dd, dh, z) = kind.defaults();
        m.add_device(NewDevice {
            level,
            kind,
            name: kind.name().into(),
            position: Point2::new(x, y),
            rotation: rot,
            elevation: z,
            width: dw,
            depth: dd,
            height: dh,
        })?;
        Ok(())
    };
    use DeviceKind as K;
    let up = std::f64::consts::FRAC_PI_2;
    device(&mut m, ground, K::Panel, 7.6, 0.4, 0.0)?;
    for (k, x, y, r) in [
        (K::Light, 1.8, 3.0, 0.0),
        (K::Light, 5.6, 3.0, 0.0),
        (K::Switch, 3.3, 0.5, 0.0),
        (K::TwoWaySwitch, 3.9, 0.5, 0.0),
        (K::TwoWaySwitch, 7.5, 5.5, 0.0),
        (K::Socket, 0.4, 1.5, -up),
        (K::Socket, 0.4, 4.5, -up),
        (K::PowerSocket, 1.5, 5.6, 0.0),
        (K::Socket, 4.5, 0.4, up),
        (K::Socket, 7.6, 2.5, up),
        (K::Socket, 6.0, 5.6, 0.0),
        (K::TvSocket, 7.6, 3.5, up),
        (K::DataSocket, 7.6, 4.0, up),
    ] {
        device(&mut m, ground, k, x, y, r)?;
    }
    for (k, x, y, r) in [
        (K::Light, 1.5, 1.5, 0.0),
        (K::Switch, 2.8, 0.5, 0.0),
        (K::Socket, 0.4, 2.0, -up),
        (K::Light, 5.5, 3.0, 0.0),
        (K::Socket, 7.6, 3.0, up),
    ] {
        device(&mut m, first, k, x, y, r)?;
    }
    // The washing machine's socket, on C4 by hand.
    let washer = m
        .devices()
        .find(|d| d.position.distance(Point2::new(0.4, 4.5)) < 1e-6)
        .map(|d| d.id);
    if let Some(d) = washer {
        m.set_device_circuit(d, Some(4))?;
    }
    // The riser and the downpipe go up through both floors.
    let pipe = |m: &mut Model, kind: DeviceKind, x: f64, y: f64| -> Res {
        let (dw, dd, _, _) = kind.defaults();
        m.add_device(NewDevice {
            level: ground,
            kind,
            name: kind.name().into(),
            position: Point2::new(x, y),
            rotation: 0.0,
            elevation: -0.5,
            width: dw,
            depth: dd,
            height: 6.3,
        })?;
        Ok(())
    };
    pipe(&mut m, K::Riser, 0.5, 5.4)?;
    pipe(&mut m, K::Downpipe, 0.5, 0.6)?;
    // Water from the riser to the kitchen and the bathroom, round corners.
    m.add_run(NewRun {
        level: ground,
        system: ServiceSystem::ColdWater,
        path: vec![
            Point2::new(0.5, 5.4),
            Point2::new(2.5, 5.4),
            Point2::new(2.5, 3.5),
            Point2::new(3.2, 3.5),
        ],
        elevation: 0.4,
        width: 0.025,
        height: 0.025,
        slope: 0.0,
    })?;
    m.add_run(NewRun {
        level: first,
        system: ServiceSystem::ColdWater,
        path: vec![
            Point2::new(0.5, 5.4),
            Point2::new(0.5, 2.0),
            Point2::new(2.0, 2.0),
        ],
        elevation: 0.4,
        width: 0.020,
        height: 0.020,
        slope: 0.0,
    })?;
    // The bathroom's drain into the downpipe, falling 2 %.
    m.add_run(NewRun {
        level: first,
        system: ServiceSystem::Drainage,
        path: vec![
            Point2::new(2.5, 1.8),
            Point2::new(2.5, 0.6),
            Point2::new(0.5, 0.6),
        ],
        elevation: -0.1,
        width: 0.09,
        height: 0.09,
        slope: 0.02,
    })?;
    // The living room's light point takes the dining pendant (G4).
    let pendant = m.fixture_types().into_iter().find(|t| {
        m.object_type(*t)
            .is_some_and(|x| x.name == "Colgante de comedor")
    });
    let living = m
        .devices()
        .find(|d| d.kind == K::Light && d.position.distance(Point2::new(5.6, 3.0)) < 1e-6)
        .map(|d| d.id);
    if let (Some(t), Some(d)) = (pendant, living) {
        m.set_device_fixture(d, Some(t))?;
    }
    Ok(m)
}

pub fn write(dir: &Path) -> Res {
    let m = build()?;
    std::fs::create_dir_all(dir)?;
    let project = dir.join(format!("instalaciones.{}", archisim_file::EXTENSION));
    archisim_file::save(&m, &project)?;
    std::fs::write(
        dir.join("instalaciones.ifc"),
        crate::ifc_text(&m, "instalaciones", None),
    )?;
    let options = archisim_plan::PlanOptions {
        scale: 50.0,
        ..Default::default()
    };
    for (i, l) in m.levels().iter().enumerate() {
        std::fs::write(
            dir.join(format!("instalaciones-planta-{i}.svg")),
            archisim_plan::render_plan(&m, l.id, &options),
        )?;
    }
    let mut report = archisim_cte::Report::default();
    archisim_cte::electrical::check(&m, &mut report);
    println!(
        "Instalaciones: {} mecanismos y aparatos, {} tramos; {} circuitos comprobados.",
        m.devices().count(),
        m.runs().count(),
        report.items.len()
    );
    println!("  Proyecto: {}", project.display());
    println!(
        "  Plantas: {}",
        dir.join("instalaciones-planta-0.svg").display()
    );
    println!("  IFC: {}", dir.join("instalaciones.ifc").display());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn circuits_by_kind_and_room() {
        let m = build().unwrap();
        let circuit_of = |k: DeviceKind, x: f64, y: f64| {
            let d = m
                .devices()
                .find(|d| d.kind == k && d.position.distance(Point2::new(x, y)) < 1e-6)
                .unwrap();
            m.device_circuit(d.id)
        };
        assert_eq!(circuit_of(DeviceKind::Light, 1.8, 3.0), Some(1));
        // The living room's light hangs a pendant, its top on the ceiling.
        let lamp = m
            .devices()
            .find(|d| d.fixture.is_some())
            .expect("a light point with its lamp");
        assert!(
            (lamp.elevation + lamp.height - 2.7).abs() < 1e-6,
            "{lamp:?}"
        );
        assert!(archisim_geom::solid::device_solids(&m, lamp.id)
            .iter()
            .any(|s| archisim_geom::solid::solid_glow(&m, s).is_some()));
        // A pendant placed as an object counts as a point of C1, unless a
        // light point is already under it.
        let mut with_lamp = m.clone();
        let level = lamp.level;
        for (x, y) in [(3.0, 1.0), (5.6, 3.0)] {
            with_lamp
                .add_object(archisim_core::NewObject {
                    level,
                    object_type: lamp.fixture.unwrap(),
                    position: Point2::new(x, y),
                    rotation: 0.0,
                    base_offset: 1.9,
                })
                .unwrap();
        }
        assert_eq!(with_lamp.lamp_points().len(), 1);
        // A socket in the kitchen: C5; in the living room: C2.
        assert_eq!(circuit_of(DeviceKind::Socket, 0.4, 1.5), Some(5));
        assert_eq!(circuit_of(DeviceKind::Socket, 4.5, 0.4), Some(2));
        assert_eq!(circuit_of(DeviceKind::PowerSocket, 1.5, 5.6), Some(3));
        assert_eq!(circuit_of(DeviceKind::TvSocket, 7.6, 3.5), None);
        let mut report = archisim_cte::Report::default();
        archisim_cte::electrical::check(&m, &mut report);
        assert!(report
            .items
            .iter()
            .all(|i| i.status == archisim_cte::Status::Passes));
        // The drain ends at the downpipe with a bend; the water bends at
        // its corners.
        let fittings = archisim_geom::solid::fitting_solids(&m);
        assert!(fittings.len() >= 4, "{}", fittings.len());
    }
}
