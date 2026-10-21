//! A small single-storey house used to test exports end to end.
//!
//! 12 × 9 m outline drawn clockwise along the exterior faces, with a typical
//! Spanish double-leaf façade and plasterboard-free brick partitions, a
//! ground-bearing slab and a flat gravel roof.
//!
//! Heights: the façade walls are 3.00 m high and the roof slab sits on them,
//! so the "Cubierta" level (the slab's top face) is at 3.00 m plus the roof
//! thickness. Walls and slabs are not trimmed against each other yet.
//!
//! Rooms (not a real design, just enough to test every kind of opening):
//! living room to the north-west with the entrance and a sliding glass door,
//! bedroom to the south-west, kitchen to the north-east and a second room
//! to the south-east with a high fixed window above the cut plane.

use archisim_core::{
    Document, Hand, Model, ModelError, NewBeam, NewColumn, NewDimension, NewOpening, NewSlab,
    NewWall, Point2, ReferenceLine, WallSide,
};
use archisim_library::add_library;

/// Height of the façade walls; the roof slab rests on them.
const WALL_HEIGHT: f64 = 3.0;

pub fn build() -> Result<Document, ModelError> {
    let mut doc = Document::new(Model::new("Vivienda de prueba"));

    let (pb, cubierta, lib) = doc.transact("Configurar proyecto", |m| {
        let pb = m.add_level("Planta baja", 0.0);
        let lib = add_library(m)?;
        let roof_thickness = m.slab_type(lib.flat_roof).map_or(0.0, |t| t.thickness());
        let cubierta = m.add_level("Cubierta", WALL_HEIGHT + roof_thickness);
        Ok((pb, cubierta, lib))
    })?;
    let (fachada, tabique, solera, cubierta_plana) =
        (lib.facade, lib.partition, lib.ground_slab, lib.flat_roof);

    // Façade: clockwise along the exterior faces so the exterior is outside.
    let corners = [
        Point2::new(0.0, 0.0),
        Point2::new(0.0, 9.0),
        Point2::new(12.0, 9.0),
        Point2::new(12.0, 0.0),
    ];
    doc.transact("Muros de fachada", |m| {
        for i in 0..4 {
            m.add_wall(NewWall {
                wall_type: fachada,
                level: pb,
                start: corners[i],
                end: corners[(i + 1) % 4],
                reference: ReferenceLine::ExteriorFace,
                height: WALL_HEIGHT,
                base_offset: 0.0,
                // Up to the roof level: the roof slab takes the top of it
                // (junctions by priority), and the façade follows the roof
                // level if it moves.
                bulge: 0.0,
                top: archisim_core::WallTop::Level {
                    level: cubierta,
                    offset: 0.0,
                },
            })?;
        }
        Ok(())
    })?;

    // Partitions run between faces, up to the roof level: the roof slab
    // cuts them where its structure is, and their brick goes through its
    // ceiling plaster (junctions by priority, v0.26).
    let inner = 0.265;
    let partitions = [
        (Point2::new(6.0, inner), Point2::new(6.0, 9.0 - inner)),
        (Point2::new(6.05, 4.5), Point2::new(12.0 - inner, 4.5)),
        (Point2::new(inner, 3.5), Point2::new(5.95, 3.5)),
    ];
    doc.transact("Tabiquería", |m| {
        for (start, end) in partitions {
            m.add_wall(NewWall {
                wall_type: tabique,
                level: pb,
                start,
                end,
                reference: ReferenceLine::Center,
                height: 2.7,
                base_offset: 0.0,
                bulge: 0.0,
                top: archisim_core::WallTop::Level {
                    level: cubierta,
                    offset: 0.0,
                },
            })?;
        }
        Ok(())
    })?;

    // Doors and windows. Façade walls are drawn clockwise from the south-west
    // corner, so positions are measured: west wall from the south, north
    // wall from the west, east wall from the north, south wall from the east.
    let walls: Vec<_> = doc.model().walls().map(|w| w.id).collect();
    let (west, north, east, south) = (walls[0], walls[1], walls[2], walls[3]);
    let (living_kitchen, bedroom_living, kitchen_room) = (walls[4], walls[6], walls[5]);
    doc.transact("Puertas y ventanas", |m| {
        let entrada = lib.entrance_door;
        let paso = lib.interior_door;
        let v2h = lib.window_two_leaves;
        let v1h = lib.window_one_leaf;
        let balconera = lib.balcony_door;
        let fija = lib.fixed_window;

        // Façade carpentry sits behind the brick leaf, in the cavity.
        let facade = Some(0.13);
        let place = |opening_type, wall, along, sill, hand| NewOpening {
            opening_type,
            wall,
            along,
            sill,
            opens_to: WallSide::Interior,
            hand,
            setback: facade,
        };
        m.add_opening(place(entrada, west, 7.8, 0.0, Hand::Right))?;
        m.add_opening(place(balconera, north, 3.0, 0.0, Hand::Left))?;
        m.add_opening(place(v1h, north, 9.0, 1.0, Hand::Left))?;
        m.add_opening(place(fija, east, 6.75, 1.6, Hand::Left))?;
        m.add_opening(place(v1h, south, 3.0, 1.0, Hand::Right))?;
        m.add_opening(place(v2h, south, 9.0, 0.9, Hand::Left))?;

        // Interior doors, centred in the partitions, opening into the room
        // on the right of each partition's drawn line.
        let inner = |wall, along, hand| NewOpening {
            opening_type: paso,
            wall,
            along,
            sill: 0.0,
            opens_to: WallSide::Interior,
            hand,
            setback: None,
        };
        m.add_opening(inner(living_kitchen, 6.5 - 0.265, Hand::Left))?;
        m.add_opening(inner(bedroom_living, 4.8 - 0.265, Hand::Right))?;
        m.add_opening(inner(kitchen_room, 7.0 - 6.05, Hand::Left))?;
        // A passage with no door into the room off the kitchen.
        m.add_opening(NewOpening {
            opening_type: lib.void,
            ..inner(kitchen_room, 3.6, Hand::Left)
        })?;
        Ok(())
    })?;

    // Slabs over the whole footprint, with their top face on each level.
    doc.transact("Forjados", |m| {
        for (slab_type, level) in [(solera, pb), (cubierta_plana, cubierta)] {
            m.add_slab(NewSlab {
                slab_type,
                level,
                outline: corners.to_vec(),
                top_offset: 0.0,
            })?;
        }
        Ok(())
    })?;

    // A porch to the south (v0.25): a lean-to roof of tiles falling 10 %
    // away from the house, on a concrete beam carried by two columns, tied
    // to the roof level so they follow it. The roof meets the façade at
    // the top of the flat roof; the beam's top follows the roof's soffit.
    doc.transact("Porche", |m| {
        let beam_depth = m.member_type(lib.beam).map_or(0.0, |t| t.profile.size().1);
        let pitch = 0.1;
        let depth = 2.85;
        let base = -pitch * depth;
        let tiles = m.slab_type(lib.pitched_roof).map_or(0.0, |t| t.thickness());
        let soffit_at = |y: f64| base + pitch * (y + depth) - tiles * (1.0 + pitch * pitch).sqrt();
        let beam_top = soffit_at(-2.7);
        for x in [0.3, 11.7] {
            m.add_column(NewColumn {
                member_type: lib.column,
                level: pb,
                position: Point2::new(x, -2.7),
                rotation: 0.0,
                base_offset: 0.0,
                top: archisim_core::WallTop::Level {
                    level: cubierta,
                    offset: beam_top - beam_depth,
                },
                height: 0.0,
            })?;
        }
        m.add_beam(NewBeam {
            member_type: lib.beam,
            level: cubierta,
            start: Point2::new(0.15, -2.7),
            end: Point2::new(11.85, -2.7),
            top_offset: beam_top,
        })?;
        m.add_roof(archisim_core::NewRoof {
            roof_type: lib.pitched_roof,
            level: cubierta,
            outline: vec![
                Point2::new(0.0, -depth),
                Point2::new(12.0, -depth),
                Point2::new(12.0, 0.0),
                Point2::new(0.0, 0.0),
            ],
            // The south side slopes; the others are gables.
            slopes: vec![Some(pitch), None, None, None],
            base_offset: base,
            overhang: 0.0,
        })?;
        // A glazed screen closing the east side of the porch (v0.25), from
        // the façade to the column, up to the underside of the beam.
        m.add_curtain_wall(archisim_core::NewCurtainWall {
            curtain_type: lib.curtain,
            level: pb,
            start: Point2::new(11.75, 0.0),
            end: Point2::new(11.75, -2.55),
            base_offset: 0.0,
            top: archisim_core::WallTop::Level {
                level: cubierta,
                offset: beam_top - beam_depth,
            },
            height: 0.0,
        })?;
        Ok(())
    })?;

    // Setting-out grids and the foundations (v0.36): strip footings under
    // the façade walls, pads under the porch columns and a tie beam
    // between them, their tops under the ground slab.
    doc.transact("Ejes y cimentación", |m| {
        use archisim_core::{FootingKind, NewFooting, NewGrid};
        let c = 0.1325;
        for (name, a, b) in [
            ("1", Point2::new(c, -4.2), Point2::new(c, 10.4)),
            (
                "2",
                Point2::new(12.0 - c, -4.2),
                Point2::new(12.0 - c, 10.4),
            ),
            ("A", Point2::new(-1.5, -2.7), Point2::new(13.5, -2.7)),
            ("B", Point2::new(-1.5, c), Point2::new(13.5, c)),
            ("C", Point2::new(-1.5, 9.0 - c), Point2::new(13.5, 9.0 - c)),
        ] {
            m.add_grid(NewGrid {
                name: name.into(),
                start: a,
                end: b,
            })?;
        }
        let concrete = m
            .materials()
            .find(|x| x.category == "concrete")
            .map(|x| x.id)
            .ok_or(ModelError::InvalidShape("demo concrete missing"))?;
        let top = -0.35;
        let facade_walls: Vec<_> = m
            .walls_on_level(pb)
            .filter(|w| w.wall_type == fachada)
            .map(|w| w.id)
            .collect();
        for w in facade_walls {
            m.add_footing(NewFooting {
                level: pb,
                kind: FootingKind::Strip {
                    start: Point2::new(0.0, 0.0),
                    end: Point2::new(1.0, 0.0),
                    wall: Some(w),
                },
                material: concrete,
                width: 0.6,
                depth: 0.5,
                top_offset: top,
            })?;
        }
        let columns: Vec<_> = m.columns_on_level(pb).map(|c| c.id).collect();
        for col in &columns {
            m.add_footing(NewFooting {
                level: pb,
                kind: FootingKind::Pad {
                    position: Point2::new(0.0, 0.0),
                    length: 1.2,
                    rotation: 0.0,
                    column: Some(*col),
                },
                material: concrete,
                width: 1.2,
                depth: 0.6,
                top_offset: top,
            })?;
        }
        m.add_footing(NewFooting {
            level: pb,
            kind: FootingKind::TieBeam {
                start: Point2::new(0.3, -2.7),
                end: Point2::new(11.7, -2.7),
            },
            material: concrete,
            width: 0.4,
            depth: 0.4,
            top_offset: top,
        })?;
        Ok(())
    })?;

    // An access ramp to the west door (v0.25): an inclined ground slab
    // rising 10 % from the garden (−0,30) to the floor.
    doc.transact("Rampa", |m| {
        let ramp = m.add_slab(NewSlab {
            slab_type: solera,
            level: pb,
            outline: vec![
                Point2::new(-3.0, 7.1),
                Point2::new(0.0, 7.1),
                Point2::new(0.0, 8.5),
                Point2::new(-3.0, 8.5),
            ],
            top_offset: -0.3,
        })?;
        m.set_slab_slope(
            ramp,
            Some(archisim_core::SlabSlope {
                from: Point2::new(-3.0, 7.8),
                to: Point2::new(0.0, 7.8),
                slope: 0.1,
            }),
        )
    })?;

    // An outdoor stair up to the flat roof along the east façade (v0.25):
    // two flights of 10 and 8 treads of 28 cm with a landing between (a
    // flight may climb 3,20 m at most), 20 risers, in reinforced concrete.
    doc.transact("Escalera", |m| {
        let concrete = m
            .materials()
            .find(|x| x.category == "concrete")
            .map(|x| x.id)
            .ok_or(ModelError::InvalidOutline)?;
        m.add_stair(archisim_core::NewStair {
            level: pb,
            path: vec![
                Point2::new(12.8, 0.4),
                Point2::new(12.8, 3.7),
                Point2::new(12.8, 6.44),
            ],
            width: 1.0,
            going: 0.28,
            base_offset: 0.0,
            top: archisim_core::WallTop::Level {
                level: cubierta,
                offset: 0.0,
            },
            height: 0.0,
            waist: 0.15,
            material: concrete,
            usage: archisim_core::StairUse::General,
        })
    })?;

    // A garden shed (v0.25): four 10 cm walls under a gable roof of tiles
    // at 35 %, trimmed under it, so its gables rise to the ridge.
    doc.transact("Caseta", |m| {
        let c = [
            Point2::new(-5.0, 10.0),
            Point2::new(-5.0, 12.5),
            Point2::new(-2.0, 12.5),
            Point2::new(-2.0, 10.0),
        ];
        let mut walls = Vec::new();
        for i in 0..4 {
            walls.push(m.add_wall(NewWall {
                wall_type: tabique,
                level: pb,
                start: c[i],
                end: c[(i + 1) % 4],
                reference: ReferenceLine::ExteriorFace,
                height: 2.2,
                base_offset: 0.0,
                bulge: 0.0,
                top: archisim_core::WallTop::Unconnected,
            })?);
        }
        let roof = m.add_roof(archisim_core::NewRoof {
            roof_type: lib.pitched_roof,
            level: pb,
            outline: vec![c[0], c[3], c[2], c[1]],
            // Gables east and west; the slopes fall north and south.
            slopes: vec![Some(0.35), None, Some(0.35), None],
            base_offset: 2.2,
            overhang: 0.3,
        })?;
        for w in walls {
            m.set_wall_roof(w, Some(roof))?;
        }
        Ok(())
    })?;

    // The plot (v0.25): survey points falling gently to the south-east,
    // under the floor of the house; true north 12° to the east of up.
    // Furniture (v0.25): kitchen units along the north wall, a sofa and a
    // dining table in the living room, a bed in each bedroom.
    doc.transact("Mobiliario", |m| {
        let ty = |m: &archisim_core::Model, name: &str| {
            m.object_types()
                .find(|t| t.name == name)
                .map(|t| t.id)
                .ok_or_else(|| ModelError::NotAnObject(name.to_string()))
        };
        use std::f64::consts::{FRAC_PI_2, PI};
        let inner = 0.265;
        for (name, x, y, rotation) in [
            ("Muebles de cocina 240", 7.45, 9.0 - inner, PI),
            ("Sofá de 3 plazas", inner, 5.5, -FRAC_PI_2),
            ("Mesa de comedor 160×90", 4.3, 7.0, 0.0),
            ("Cama doble 150×200", 2.0, 3.45, PI),
            ("Cama individual 90×190", 12.0 - inner, 2.0, FRAC_PI_2),
            ("Armario 120", 5.95, 1.5, FRAC_PI_2),
            ("Frigorífico 60", 6.05, 7.5, -FRAC_PI_2),
            ("Muebles altos de cocina 120", 7.45, 9.0 - inner, PI),
        ] {
            let object_type = ty(m, name)?;
            m.add_object(archisim_core::NewObject {
                level: pb,
                object_type,
                position: Point2::new(x, y),
                rotation,
                base_offset: 0.0,
            })?;
        }
        Ok(())
    })?;

    // Instalaciones: water to the kitchen along the north wall, its
    // drain, the extract duct of the kitchen hood out through the north
    // façade, a light in each zone, sockets and a switch.
    doc.transact("Instalaciones", |m| {
        use archisim_core::{DeviceKind, NewDevice, NewRun, ServiceSystem};
        use std::f64::consts::PI;
        let inner = 0.265;
        for (system, gap, width, elevation) in [
            (ServiceSystem::ColdWater, 0.05, 0.025, 0.4),
            (ServiceSystem::HotWater, 0.1, 0.02, 0.4),
            (ServiceSystem::Drainage, 0.2, 0.11, 0.15),
        ] {
            let (x, y) = (12.0 - inner - gap, 9.0 - inner - gap);
            m.add_run(NewRun {
                level: pb,
                system,
                path: vec![Point2::new(x, 0.8), Point2::new(x, y), Point2::new(7.0, y)],
                elevation,
                width,
                height: width,
            })?;
        }
        m.add_run(NewRun {
            level: pb,
            system: ServiceSystem::ExhaustAir,
            path: vec![Point2::new(7.45, 8.2), Point2::new(7.45, 9.4)],
            elevation: 2.3,
            width: 0.2,
            height: 0.15,
        })?;
        let place = |m: &mut archisim_core::Model,
                     kind: DeviceKind,
                     name: &str,
                     at: Point2,
                     rotation: f64|
         -> Result<(), ModelError> {
            let (width, depth, height, elevation) = kind.defaults();
            m.add_device(NewDevice {
                level: pb,
                kind,
                name: name.to_string(),
                position: at,
                rotation,
                elevation,
                width,
                depth,
                height,
            })?;
            Ok(())
        };
        for (x, y) in [(2.5, 6.5), (8.5, 6.5), (2.5, 2.2), (9.5, 2.2)] {
            place(m, DeviceKind::Light, "Luminaria", Point2::new(x, y), 0.0)?;
        }
        for (x, y, r) in [
            (6.2, 9.0 - inner - 0.02, PI),
            (8.9, 9.0 - inner - 0.02, PI),
            (inner + 0.02, 7.6, -PI / 2.0),
        ] {
            place(m, DeviceKind::Socket, "Enchufe", Point2::new(x, y), r)?;
        }
        place(
            m,
            DeviceKind::Switch,
            "Interruptor",
            Point2::new(12.0 - inner - 0.02, 4.0),
            PI / 2.0,
        )?;
        place(
            m,
            DeviceKind::Equipment,
            "Caldera",
            Point2::new(11.4, 0.6),
            PI,
        )?;
        Ok(())
    })?;

    // A guard round the flat roof (v0.25), open where the outdoor stair
    // arrives on the east side.
    doc.transact("Barandilla", |m| {
        let steel = m
            .materials()
            .find(|x| x.category == "steel")
            .map(|x| x.id)
            .ok_or(ModelError::InvalidRailing)?;
        m.add_railing(archisim_core::NewRailing {
            level: cubierta,
            path: vec![
                Point2::new(11.9, 5.9),
                Point2::new(11.9, 0.1),
                Point2::new(0.1, 0.1),
                Point2::new(0.1, 8.9),
                Point2::new(11.9, 8.9),
                Point2::new(11.9, 7.0),
            ],
            base_offset: 0.0,
            height: 1.1,
            post_spacing: 1.5,
            material: steel,
            glass: None,
        })?;
        Ok(())
    })?;

    doc.transact("Terreno", |m| {
        let earth = m.add_material(
            "Terreno natural",
            "earth",
            [176, 156, 120],
            archisim_core::CutPattern::Dots,
        );
        let mut points = Vec::new();
        for i in 0..5 {
            for j in 0..5 {
                let (x, y) = (-6.0 + 6.0 * i as f64, -8.0 + 6.0 * j as f64);
                let z = -0.25 + 0.012 * y - 0.008 * x;
                points.push(archisim_core::SurveyPoint {
                    at: Point2::new(x, y),
                    z: (z * 100.0f64).round() / 100.0,
                });
            }
        }
        let terrain = m.add_terrain(archisim_core::NewTerrain {
            level: pb,
            points,
            material: earth,
            depth: 1.0,
            contour: 0.1,
            lines: Vec::new(),
        })?;

        // A contour line drawn by hand to the north, past the survey grid.
        m.edit_terrain(terrain, |t| {
            t.add_line(&[Point2::new(-6.0, 22.0), Point2::new(18.0, 22.0)], -0.05);
        })?;
        m.set_true_north(12f64.to_radians())
    })?;

    doc.transact("Jardín", |m| {
        let ty = |m: &archisim_core::Model, name: &str| {
            m.object_types()
                .find(|t| t.name == name)
                .map(|t| t.id)
                .ok_or_else(|| ModelError::NotAnObject(name.to_string()))
        };
        use std::f64::consts::FRAC_PI_2;
        // The garden (v0.26): trees, shrubs by the entrance, someone
        // walking to it and a car on the plot, on the ground.
        let ground = |m: &archisim_core::Model, p: Point2| {
            m.terrains_on_level(pb)
                .find_map(|t| t.height_at(p))
                .map_or(0.0, |z| (z * 1000.0).round() / 1000.0)
        };
        for (name, x, y, rotation) in [
            ("Árbol de hoja caduca 6 m", -3.6, 2.4, 0.0),
            ("Árbol grande 12 m", 15.5, 13.0, 0.0),
            ("Arbusto 120", -1.4, 5.6, 0.0),
            ("Arbusto 120", -1.4, 10.2, 0.0),
            ("Persona", -2.2, 7.6, -FRAC_PI_2),
            ("Turismo", 15.0, -6.5, 0.0),
        ] {
            let object_type = ty(m, name)?;
            let at = Point2::new(x, y);
            let base_offset = ground(m, at);
            m.add_object(archisim_core::NewObject {
                level: pb,
                object_type,
                position: at,
                rotation,
                base_offset,
            })?;
        }
        Ok(())
    })?;

    // Dimensions: the openings of the south and west façades, outside, the
    // overall size beyond them, and level marks on the ground slab.
    doc.transact("Cotas", |m| {
        let walls: Vec<_> = m.walls_on_level(pb).map(|w| w.id).collect();
        let (west, south) = (walls[0], walls[3]);
        for (wall, overall_from, overall_to) in [
            (south, Point2::new(0.0, 0.0), Point2::new(12.0, 0.0)),
            (west, Point2::new(0.0, 0.0), Point2::new(0.0, 9.0)),
        ] {
            let Some(chain) = archisim_geom::wall_face_chain(m, wall, true) else {
                continue;
            };
            let first = chain.points[0];
            m.add_dimension(NewDimension {
                level: pb,
                points: chain.points.clone(),
                direction: chain.direction,
                position: first + chain.outward * 0.8,
            })?;
            m.add_dimension(NewDimension {
                level: pb,
                points: vec![overall_from, overall_to],
                direction: chain.direction,
                position: first + chain.outward * 1.4,
            })?;
        }
        // Tied to the corners and jambs they measure.
        archisim_geom::anchors::follow(m, &Default::default());
        // Clear of the room stamps, in a corner of two rooms.
        m.add_elevation_mark(pb, Point2::new(1.0, 5.0))?;
        m.add_elevation_mark(pb, Point2::new(10.2, 1.0))?;
        Ok(())
    })?;

    // Sections and elevations (v0.25): A across the living room and the
    // kitchen, looking north; B through the porch, looking west; and the
    // four elevations.
    doc.transact("Secciones y alzados", |m| {
        for (name, start, end, depth) in [
            ("A", Point2::new(-1.5, 6.2), Point2::new(13.5, 6.2), 4.0),
            ("B", Point2::new(8.5, -4.5), Point2::new(8.5, 11.0), 9.0),
        ] {
            m.add_section(archisim_core::NewSection {
                name: name.into(),
                kind: archisim_core::SectionKind::Section,
                start,
                end,
                depth,
            })?;
        }
        for e in archisim_plan::section::elevations_around(m) {
            m.add_section(e)?;
        }
        Ok(())
    })?;

    // Drafting: structural grid lines, a paved terrace to the east and the
    // title of the plan.
    doc.transact("Dibujo 2D", |m| {
        use archisim_core::{
            DrawLayer, Hatch, HatchPattern, LineStyle, NewDraft, Polyline, Shape, Text, TextAlign,
            ViewRef,
        };
        let view = ViewRef::Level(pb);
        let ejes = m.add_draw_layer(DrawLayer::new(
            "Ejes",
            [200, 40, 40],
            0.13,
            LineStyle::DashDot,
        ))?;
        let urb = m.add_draw_layer(DrawLayer::new(
            "Urbanización",
            [90, 90, 90],
            0.18,
            LineStyle::Continuous,
        ))?;
        let textos = m.add_draw_layer(DrawLayer::new(
            "Textos",
            [0, 0, 0],
            0.25,
            LineStyle::Continuous,
        ))?;
        for x in [0.0, 6.0, 12.0] {
            m.add_draft(NewDraft::new(
                view,
                ejes,
                Shape::Line {
                    a: Point2::new(x, -2.5),
                    b: Point2::new(x, 11.0),
                },
            ))?;
        }
        let terrace = Polyline::straight(
            vec![
                Point2::new(12.0, 0.0),
                Point2::new(15.0, 0.0),
                Point2::new(15.0, 9.0),
                Point2::new(12.0, 9.0),
            ],
            true,
        );
        m.add_draft(NewDraft::new(
            view,
            urb,
            Shape::Hatch(Hatch {
                loops: vec![terrace.clone()],
                pattern: HatchPattern::Tiles,
                scale: 1.0,
                angle: 0.0,
                background: None,
            }),
        ))?;
        m.add_draft(NewDraft::new(view, urb, Shape::Polyline(terrace)))?;
        m.add_draft(NewDraft::new(
            view,
            textos,
            Shape::Text(Text {
                at: Point2::new(6.0, 11.6),
                content: "PLANTA BAJA".into(),
                height: 5.0,
                angle: 0.0,
                align: TextAlign::BottomCentre,
            }),
        ))?;
        m.add_draft(NewDraft::new(
            view,
            textos,
            Shape::Text(Text {
                at: Point2::new(13.5, 4.5),
                content: "Terraza\nbaldosa 30×30".into(),
                height: 2.5,
                angle: 0.0,
                align: TextAlign::Middle,
            }),
        ))?;
        Ok(())
    })?;

    // Rooms found inside the walls, and a tag on every door and window,
    // on the outer side of its wall.
    doc.transact("Locales y etiquetas", |m| {
        use archisim_core::{NewRoom, TagTarget};
        use archisim_geom::{opening_geometry, room_outline, Contour};
        // Named after the furniture: living and kitchen to the north,
        // bedrooms to the south.
        let wood = "Tarima flotante de roble";
        for (number, name, usage, at, floor) in [
            (
                "0.01",
                "Estar-comedor",
                "Estar",
                Point2::new(3.0, 6.0),
                wood,
            ),
            (
                "0.02",
                "Dormitorio",
                "Dormitorio",
                Point2::new(3.0, 1.8),
                wood,
            ),
            (
                "0.03",
                "Cocina",
                "Cocina",
                Point2::new(9.0, 6.8),
                "Gres porcelánico",
            ),
            (
                "0.04",
                "Dormitorio 2",
                "Dormitorio",
                Point2::new(9.0, 2.2),
                wood,
            ),
        ] {
            let outline = room_outline(m, pb, at, Contour::InteriorFace)
                .map_err(|_| ModelError::InvalidOutline)?;
            let id = m.add_room(NewRoom {
                level: pb,
                number: number.into(),
                name: name.into(),
                outline,
                holes: Vec::new(),
                label: at,
                bounded: true,
            })?;
            if let Some(mut r) = m.room(id).cloned() {
                r.usage = usage.into();
                r.finishes = archisim_core::Finishes {
                    floor: floor.into(),
                    walls: "Pintura plástica lisa".into(),
                    ceiling: "Falso techo de placa de yeso".into(),
                };
                m.set_room(r)?;
            }
        }
        let ops: Vec<_> = m
            .openings()
            .filter(|o| {
                m.opening_type(o.opening_type)
                    .is_some_and(|t| t.kind != archisim_core::OpeningKind::Void)
            })
            .map(|o| o.id)
            .collect();
        for o in ops {
            if let Some(g) = opening_geometry(m, o) {
                let at = g.point((g.t0 + g.t1) / 2.0, g.exterior_offset + 0.35);
                m.add_tag(TagTarget::Opening(o), at, false)?;
            }
        }
        Ok(())
    })?;

    // The ground floor plan on an A2 sheet at 1:50, with its title block.
    doc.transact("Plano", |m| {
        use archisim_core::{NewSheet, NewViewport, ProjectInfo};
        m.set_info(ProjectInfo {
            client: "Promotor de ejemplo".into(),
            site: "Calle Mayor 1, Madrid".into(),
            author: "ArchiSim".into(),
            studio: "Estudio de ejemplo".into(),
            number: "2026-001".into(),
            phase: "Proyecto básico".into(),
            date: "octubre 2026".into(),
        });
        // On the map: Calle Mayor 1, Madrid (ETRS89 / UTM 30N), the ±0,00
        // at 650 m above sea level.
        m.set_georeference(Some(archisim_core::Georeference {
            crs: "EPSG:25830".into(),
            eastings: 440_230.0,
            northings: 4_474_270.0,
            height: 650.0,
        }))?;
        // A dwelling for wheelchair users: the code check says what of
        // the house does not meet it.
        m.set_code_settings(archisim_core::CodeSettings {
            building_use: archisim_core::BuildingUse::Dwelling,
            accessibility: archisim_core::Accessibility::WheelchairDwelling,
            sprinklers: false,
        });
        let sheet = m.add_sheet(NewSheet {
            number: "A-01".into(),
            name: "Planta baja".into(),
            width: 594.0,
            height: 420.0,
        })?;
        m.add_viewport(NewViewport {
            sheet,
            level: pb,
            section: None,
            axo: None,
            scale: 50.0,
            cut_plane: 1.0,
            crop: None,
            centre: Point2::new(0.21, 0.24),
        })?;
        // The rooms' areas over the plan, against the right of the frame.
        let (w, _) =
            archisim_plan::schedules::table(m, archisim_core::ScheduleKind::Rooms, Some(pb)).size();
        m.add_table(
            sheet,
            archisim_core::ScheduleKind::Rooms,
            Some(pb),
            Point2::new(0.579 - w * 0.001, 0.405),
        )?;
        // Sections and elevations at 1:100 on A-02: the south elevation
        // above, section A below, one over the other on the left.
        let sheet2 = m.add_sheet(NewSheet {
            number: "A-02".into(),
            name: "Alzado sur y sección A".into(),
            width: 594.0,
            height: 420.0,
        })?;
        let find = |m: &archisim_core::Model, t: &str| {
            m.sections()
                .find(|s| s.title() == t)
                .map(|s| s.id)
                .ok_or(ModelError::InvalidShape("demo section missing"))
        };
        for (title, top) in [("Alzado Sur", 0.395), ("Sección A", 0.2)] {
            let id = find(m, title)?;
            let crop = archisim_plan::sheet::section_extent(m, id, 100.0)
                .ok_or(ModelError::InvalidShape("demo section missing"))?;
            let half = (crop.1 - crop.0) * (0.5 / 100.0);
            m.add_viewport(NewViewport {
                sheet: sheet2,
                level: pb,
                section: Some(id),
                axo: None,
                scale: 100.0,
                cut_plane: 1.0,
                crop: Some(crop),
                centre: Point2::new(0.03 + half.x, top - half.y),
            })?;
        }
        // The house in 3D, on the right, as the 3D view opens.
        let axo = archisim_core::Axo::default();
        let crop = archisim_plan::axo::axo_extent(m, &axo, 200.0);
        let half = (crop.1 - crop.0) * (0.5 / 200.0);
        m.add_viewport(NewViewport {
            sheet: sheet2,
            level: pb,
            section: None,
            axo: Some(axo),
            scale: 200.0,
            cut_plane: 1.0,
            crop: Some(crop),
            centre: Point2::new(0.42, 0.3 - half.y.min(0.1)),
        })?;
        Ok(())
    })?;

    // The studio's own data: a finish for each room and a measurement code
    // for each wall type (in Propiedades, the schedules and the IFC).
    doc.transact("Parámetros del proyecto", |m| {
        use archisim_core::{NewUserParameter, ParamCategory, UserKind, Value};
        let finish = m.add_user_parameter(NewUserParameter {
            name: "Acabado de suelo".into(),
            kind: UserKind::Choice(vec!["Gres".into(), "Tarima".into(), "Microcemento".into()]),
            group: "AD5_Acabados".into(),
            categories: vec![ParamCategory::Rooms],
            by_type: false,
            default: Value::Text("Gres".into()),
            description: "Pavimento del local".into(),
        })?;
        let code = m.add_user_parameter(NewUserParameter {
            name: "Código de medición".into(),
            kind: UserKind::Text,
            group: "AD5_Mediciones".into(),
            categories: vec![ParamCategory::Walls],
            by_type: true,
            default: Value::Text(String::new()),
            description: "Partida del presupuesto".into(),
        })?;
        let rooms: Vec<(u64, String)> = m.rooms().map(|r| (r.id.0, r.name.clone())).collect();
        for (id, name) in rooms {
            if name.contains("Dormitorio") || name.contains("Estar") {
                m.set_parameter_value(id, finish, Value::Text("Tarima".into()))?;
            }
        }
        let types: Vec<(u64, String)> = m.wall_types().map(|t| (t.id.0, t.name.clone())).collect();
        for (i, (id, _)) in types.iter().enumerate() {
            m.set_parameter_value(*id, code, Value::Text(format!("FAB0{}", i + 1)))?;
        }
        Ok(())
    })?;

    Ok(doc)
}

/// An urbanisation that places the demo house as a module three times:
/// two side by side (the second mirrored, so their entrances face each
/// other) and one turned, across the street (I5), the mass of a block
/// still to design (I4), and the plots and the street (Urbanismo).
pub fn masterplan(house: &std::path::Path) -> Result<Model, Box<dyn std::error::Error>> {
    let text = std::fs::read_to_string(house)?;
    let (text, stamp, module) = archisim_file::read_module(&text)?;
    let mut m = archisim_library::new_project("Urbanización de prueba");
    let level = m.levels()[0].id;
    let name = house
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    let src = m.add_module_source("Vivienda tipo", name, text.into(), stamp, module)?;
    m.add_levels_for_module(src, level)?;
    for (x, y, rotation, mirror) in [
        (0.0, 0.0, 0.0, false),
        (30.0, 0.0, 0.0, true),
        (6.0, -16.0, std::f64::consts::PI, false),
    ] {
        m.add_module(archisim_core::NewModulePlacement {
            source: src,
            level,
            position: Point2::new(x, y),
            rotation,
            mirror,
        })?;
    }
    // The two plots on each side of the street, and the street (with its
    // carriageway, kerbs and sidewalks, no parking).
    for (name, reference, outline, setback) in [
        (
            "Parcela A",
            "1234501VK4713S",
            [(-6.0, -2.85), (36.0, -2.85), (36.0, 14.0), (-6.0, 14.0)],
            3.0,
        ),
        (
            "Parcela B",
            "1234502VK4713S",
            [(-8.0, -34.0), (48.0, -34.0), (48.0, -12.15), (-8.0, -12.15)],
            3.0,
        ),
    ] {
        m.add_plot(archisim_core::NewPlot {
            name: name.into(),
            level,
            outline: outline.iter().map(|(x, y)| Point2::new(*x, *y)).collect(),
            setback,
            reference: reference.into(),
        })?;
    }
    let by_category = |m: &Model, c: &str| m.materials().find(|x| x.category == c).map(|x| x.id);
    let paving = by_category(&m, "ceramic")
        .or_else(|| by_category(&m, "concrete"))
        .ok_or(ModelError::InvalidShape("paving missing"))?;
    let kerb = by_category(&m, "concrete").ok_or(ModelError::InvalidShape("concrete missing"))?;
    let asphalt = m.add_material(
        "Aglomerado asfáltico",
        "asphalt",
        [96, 96, 100],
        archisim_core::CutPattern::Solid,
    );
    m.add_road(archisim_core::NewRoad {
        name: "Calle de la Huerta".into(),
        level,
        axis: vec![Point2::new(-12.0, -7.5), Point2::new(52.0, -7.5)],
        section: archisim_core::RoadSection {
            carriageway: 6.0,
            parking: 0.0,
            kerb: 0.15,
            sidewalk: 1.5,
            kerb_height: 0.15,
            depth: 0.4,
        },
        top_offset: -0.3,
        pavement: asphalt,
        kerb_material: kerb,
        sidewalk_material: paving,
    })?;
    // The concept of the block still to build across the street (I4): a
    // mass of three floors on plot B.
    let concrete = m
        .materials()
        .find(|x| x.category == "concept")
        .map(|x| x.id)
        .ok_or(ModelError::InvalidShape("concept material missing"))?;
    m.add_mass(archisim_core::NewMass {
        name: "Bloque de viviendas".into(),
        level,
        shape: archisim_core::MassShape::Prism(vec![
            Point2::new(20.0, -30.0),
            Point2::new(44.0, -30.0),
            Point2::new(44.0, -16.0),
            Point2::new(20.0, -16.0),
        ]),
        base_offset: 0.0,
        height: 9.0,
        floor_height: Some(3.0),
        material: concrete,
    })?;
    Ok(m)
}

#[cfg(test)]
mod tests {
    use archisim_core::SlabFunction;

    #[test]
    fn demo_builds() {
        let doc = super::build().unwrap();
        let m = doc.model();
        assert_eq!(m.walls().count(), 11);
        assert_eq!(m.slabs().count(), 3);
        assert_eq!(m.roofs().count(), 2);
        assert_eq!(m.stairs().count(), 1);
        assert_eq!(m.terrains().count(), 1);
        assert_eq!(m.curtain_walls().count(), 1);
        assert_eq!(m.railings().count(), 1);
        assert_eq!(m.objects().count(), 14);
        let stair = m.stairs().next().unwrap();
        assert!(stair.issues().is_empty(), "{:?}", stair.issues());
        assert_eq!(m.columns().count(), 2);
        assert_eq!(m.beams().count(), 1);
        assert_eq!(m.openings().count(), 10);
        assert_eq!(m.opening_types().count(), 28);
        assert_eq!(doc.undo_label(), Some("Parámetros del proyecto"));
        assert_eq!(m.user_parameters().count(), 2);
        assert_eq!(m.sections().count(), 6);
        assert_eq!(m.sheets().len(), 2);
        assert_eq!(m.viewports().filter(|v| v.section.is_some()).count(), 2);
        assert_eq!(m.rooms().count(), 4);
        assert_eq!(m.tags().count(), 9);
        let total: f64 = m.rooms().map(|r| r.area()).sum();
        assert!(total > 90.0 && total < 108.0, "{total}");
        assert_eq!(m.drafts().count(), 7);
        assert_eq!(m.draw_layers().len(), 4);
        assert_eq!(m.dimensions().count(), 4);
        assert!(m
            .dimensions()
            .all(|d| d.anchors.iter().any(Option::is_some)));
        assert_eq!(m.elevation_marks().count(), 2);
        // The roof slab rests on the façade walls.
        let roof = m
            .slabs()
            .find(|s| m.slab_type(s.slab_type).unwrap().function == SlabFunction::Roof)
            .unwrap();
        let thickness = m.slab_type(roof.slab_type).unwrap().thickness();
        let level = m.level(roof.level).unwrap().elevation;
        assert!((level + roof.top_offset - thickness - super::WALL_HEIGHT).abs() < 1e-9);
    }

    /// The partitions of the demo butt against the façades: their inner
    /// leaf goes through the plaster; a load-bearing wall instead goes up
    /// to the outer leaf. The IFC volumes still match (validated in CI).
    /// Section A dimensions the floor-to-floor height (3,55) and the clear
    /// height of the two rooms it cuts (3,00 from the floor to the slab);
    /// switched off, none.
    #[test]
    fn sections_dimension_their_heights() {
        let doc = super::build().unwrap();
        let m = doc.model();
        let a = m.sections().find(|s| s.title() == "Sección A").unwrap();
        let options = archisim_plan::PlanOptions {
            scale: 100.0,
            ..Default::default()
        };
        let d = archisim_plan::section::section_drawing(m, a);
        let svg = archisim_plan::section::render_section(m, &d, &options);
        assert!(svg.contains("<g id=\"heights\">"));
        let mut off = a.clone();
        off.heights = false;
        let d = archisim_plan::section::section_drawing(m, &off);
        let svg = archisim_plan::section::render_section(m, &d, &options);
        assert!(!svg.contains("<g id=\"heights\">"));
    }

    /// A view can hide what phase 2 added: the furniture, the columns and
    /// beams, the terrain…, as it hides walls or dimensions.
    #[test]
    fn views_hide_phase_2_elements() {
        use archisim_core::Category;
        let doc = super::build().unwrap();
        let m = doc.model();
        let pb = m.levels()[0].id;
        let mut o = archisim_plan::PlanOptions::default();
        let all = archisim_plan::render_plan(m, pb, &o);
        assert!(all.contains("<g id=\"objects\">"));
        assert!(all.contains("<g id=\"structure\">"));
        for c in [Category::Objects, Category::Structure, Category::Terrain] {
            o.style.hidden.insert(c);
        }
        let some = archisim_plan::render_plan(m, pb, &o);
        assert!(!some.contains("<g id=\"objects\">"));
        assert!(!some.contains("<g id=\"structure\">"));
        assert!(some.len() < all.len());
        // Walls and rooms are still there.
        assert!(some.contains("<g id=\"poche\">"));
        assert!(some.contains("<g id=\"rooms\">") == all.contains("<g id=\"rooms\">"));
    }

    #[test]
    fn partitions_and_bearing_walls_butt_into_the_facade() {
        use archisim_geom::{wall_geometry, EndCut};
        let doc = super::build().unwrap();
        let mut m = doc.model().clone();
        let central = m.walls().nth(4).unwrap().id;
        let g = wall_geometry(&m, central).unwrap();
        let EndCut::Tee(t) = &g.start_cut else {
            panic!("{:?}", g.start_cut)
        };
        // Plaster stops, LHD through the 15 mm of plaster.
        assert!((t.reach[1] - 0.015).abs() < 1e-9, "{:?}", t.reach);
        assert_eq!(t.reach[0], 0.0);
        let bearing = m
            .wall_types()
            .find(|w| w.name.starts_with("Muro de carga"))
            .unwrap()
            .id;
        m.set_wall_type(central, bearing).unwrap();
        let g = wall_geometry(&m, central).unwrap();
        let EndCut::Tee(t) = &g.start_cut else {
            panic!()
        };
        // Plaster, inner leaf and insulation of the façade: 0.135 m.
        assert!((t.reach[1] - 0.135).abs() < 1e-9, "{:?}", t.reach);
    }

    /// The magic wand finds the four rooms of the demo house (doors in the
    /// walls do not open them) and the whole building outline.
    #[test]
    fn magic_wand_finds_every_room() {
        use archisim_core::Point2;
        use archisim_geom::{room_outline, signed_area, Contour};
        let doc = super::build().unwrap();
        let m = doc.model();
        let ground = m.walls().next().unwrap().level;
        let mut total = 0.0;
        for p in [(3.0, 2.0), (3.0, 6.0), (9.0, 2.0), (9.0, 7.0)] {
            let room = room_outline(m, ground, Point2::new(p.0, p.1), Contour::InteriorFace)
                .unwrap_or_else(|e| panic!("{p:?}: {e}"));
            assert_eq!(room.len(), 4, "{p:?}: {room:?}");
            total += signed_area(&room);
        }
        let building =
            room_outline(m, ground, Point2::new(3.0, 2.0), Contour::ExteriorFace).unwrap();
        assert!((signed_area(&building) - 108.0).abs() < 1e-9);
        // The rooms fill the building but for the walls.
        assert!(total > 90.0 && total < 108.0, "{total}");
    }
}
