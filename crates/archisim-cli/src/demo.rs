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
    Document, Hand, Model, ModelError, NewDimension, NewOpening, NewSlab, NewWall, Point2,
    ReferenceLine, WallSide,
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
    let roof_depth = doc
        .model()
        .slab_type(lib.flat_roof)
        .map_or(0.0, |t| t.thickness());
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
                // Up to the underside of the roof slab: the façade follows
                // the roof level if it moves.
                bulge: 0.0,
                top: archisim_core::WallTop::Level {
                    level: cubierta,
                    offset: -roof_depth,
                },
            })?;
        }
        Ok(())
    })?;

    // Partitions run between faces (T-junctions are square-cut for now).
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
                top: archisim_core::WallTop::Unconnected,
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
        m.add_elevation_mark(pb, Point2::new(3.0, 6.5))?;
        m.add_elevation_mark(pb, Point2::new(9.0, 2.5))?;
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
                at: Point2::new(6.0, 10.2),
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

    Ok(doc)
}

#[cfg(test)]
mod tests {
    use archisim_core::SlabFunction;

    #[test]
    fn demo_builds() {
        let doc = super::build().unwrap();
        let m = doc.model();
        assert_eq!(m.walls().count(), 7);
        assert_eq!(m.slabs().count(), 2);
        assert_eq!(m.openings().count(), 10);
        assert_eq!(m.opening_types().count(), 9);
        assert_eq!(doc.undo_label(), Some("Dibujo 2D"));
        assert_eq!(m.drafts().count(), 7);
        assert_eq!(m.draw_layers().len(), 4);
        assert_eq!(m.dimensions().count(), 4);
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
