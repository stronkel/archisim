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
    CutPattern, Document, DoorOperation, Hand, Layer, LayerFunction, Model, ModelError, NewOpening,
    NewOpeningType, NewSlab, NewWall, OpeningKind, Point2, ReferenceLine, SlabFunction,
    WallFunction, WallSide, WindowOperation,
};

/// Height of the façade walls; the roof slab rests on them.
const WALL_HEIGHT: f64 = 3.0;

pub fn build() -> Result<Document, ModelError> {
    let mut doc = Document::new(Model::new("Vivienda de prueba"));

    let (pb, cubierta, fachada, tabique, solera, cubierta_plana) =
        doc.transact("Configurar proyecto", |m| {
            let pb = m.add_level("Planta baja", 0.0);

            let monocapa = m.add_material(
                "Mortero monocapa",
                "plaster",
                [236, 232, 222],
                CutPattern::Dots,
            );
            let perforado = m.add_material(
                "Ladrillo perforado",
                "brick",
                [232, 196, 178],
                CutPattern::Diagonal,
            );
            let lana = m.add_material(
                "Lana mineral",
                "insulation",
                [250, 243, 205],
                CutPattern::Insulation,
            );
            let lhd = m.add_material(
                "Ladrillo hueco doble",
                "brick",
                [240, 214, 200],
                CutPattern::Diagonal,
            );
            let yeso = m.add_material(
                "Enlucido de yeso",
                "plaster",
                [245, 245, 245],
                CutPattern::Solid,
            );

            let layer = |material, thickness, function| Layer {
                material,
                thickness,
                function,
            };
            let fachada = m.add_wall_type(
                "Fachada doble hoja 26,5",
                WallFunction::Exterior,
                vec![
                    layer(monocapa, 0.015, LayerFunction::Finish),
                    layer(perforado, 0.115, LayerFunction::Structure),
                    layer(lana, 0.050, LayerFunction::Insulation),
                    layer(lhd, 0.070, LayerFunction::Substrate),
                    layer(yeso, 0.015, LayerFunction::Finish),
                ],
            )?;
            let tabique = m.add_wall_type(
                "Tabique LHD 10",
                WallFunction::Interior,
                vec![
                    layer(yeso, 0.015, LayerFunction::Finish),
                    layer(lhd, 0.070, LayerFunction::Substrate),
                    layer(yeso, 0.015, LayerFunction::Finish),
                ],
            )?;
            let baldosa = m.add_material(
                "Baldosa cerámica",
                "ceramic",
                [214, 206, 190],
                CutPattern::Solid,
            );
            let mortero = m.add_material(
                "Mortero de cemento",
                "mortar",
                [222, 222, 214],
                CutPattern::Dots,
            );
            let hormigon = m.add_material(
                "Hormigón armado",
                "concrete",
                [205, 205, 205],
                CutPattern::Cross,
            );
            let lamina = m.add_material(
                "Lámina impermeabilizante",
                "membrane",
                [70, 70, 70],
                CutPattern::Solid,
            );
            let grava = m.add_material("Grava", "gravel", [210, 204, 190], CutPattern::Dots);
            let xps = m.add_material(
                "Poliestireno extruido (XPS)",
                "insulation",
                [214, 236, 214],
                CutPattern::Insulation,
            );
            let celular = m.add_material(
                "Hormigón celular",
                "concrete",
                [228, 224, 214],
                CutPattern::Dots,
            );

            let solera = m.add_slab_type(
                "Solera HA 15 con encachado",
                SlabFunction::BaseSlab,
                vec![
                    layer(baldosa, 0.015, LayerFunction::Finish),
                    layer(mortero, 0.030, LayerFunction::Substrate),
                    layer(hormigon, 0.150, LayerFunction::Structure),
                    layer(lamina, 0.002, LayerFunction::Membrane),
                    layer(grava, 0.150, LayerFunction::Substrate),
                ],
            )?;
            let cubierta_plana = m.add_slab_type(
                "Cubierta plana invertida con grava",
                SlabFunction::Roof,
                vec![
                    layer(grava, 0.050, LayerFunction::Finish),
                    layer(xps, 0.080, LayerFunction::Insulation),
                    layer(lamina, 0.003, LayerFunction::Membrane),
                    layer(celular, 0.100, LayerFunction::Substrate),
                    layer(hormigon, 0.300, LayerFunction::Structure),
                    layer(yeso, 0.015, LayerFunction::Finish),
                ],
            )?;
            let roof_thickness = m.slab_type(cubierta_plana).map_or(0.0, |t| t.thickness());
            let cubierta = m.add_level("Cubierta", WALL_HEIGHT + roof_thickness);
            Ok((pb, cubierta, fachada, tabique, solera, cubierta_plana))
        })?;

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
        let madera = m.add_material("Madera de pino", "wood", [196, 160, 112], CutPattern::Solid);
        let aluminio = m.add_material(
            "Aluminio lacado",
            "aluminium",
            [120, 124, 128],
            CutPattern::Solid,
        );
        let vidrio = m.add_material("Vidrio doble", "glass", [190, 222, 236], CutPattern::Solid);
        let mut kind = |name: &str, kind, width, height, frame_width, frame_depth, frame, panel| {
            m.add_opening_type(NewOpeningType {
                name: name.into(),
                kind,
                width,
                height,
                frame_width,
                frame_depth,
                frame_material: frame,
                panel_material: panel,
            })
        };
        let entrada = kind(
            "Puerta de entrada 1 hoja 100×215",
            OpeningKind::Door(DoorOperation::SingleSwing),
            1.00,
            2.15,
            0.07,
            0.10,
            madera,
            madera,
        )?;
        let paso = kind(
            "Puerta de paso 1 hoja 80×205",
            OpeningKind::Door(DoorOperation::SingleSwing),
            0.80,
            2.05,
            0.05,
            0.07,
            madera,
            madera,
        )?;
        let v2h = kind(
            "Ventana practicable 2 hojas 180×130",
            OpeningKind::Window(WindowOperation::DoubleCasement),
            1.80,
            1.30,
            0.06,
            0.07,
            aluminio,
            vidrio,
        )?;
        let v1h = kind(
            "Ventana practicable 1 hoja 100×120",
            OpeningKind::Window(WindowOperation::SingleCasement),
            1.00,
            1.20,
            0.06,
            0.07,
            aluminio,
            vidrio,
        )?;
        let balconera = kind(
            "Balconera corredera 2 hojas 240×220",
            OpeningKind::Window(WindowOperation::Sliding),
            2.40,
            2.20,
            0.06,
            0.07,
            aluminio,
            vidrio,
        )?;
        let fija = kind(
            "Ventana fija 60×60",
            OpeningKind::Window(WindowOperation::Fixed),
            0.60,
            0.60,
            0.05,
            0.07,
            aluminio,
            vidrio,
        )?;

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
        assert_eq!(m.openings().count(), 9);
        assert_eq!(m.opening_types().count(), 6);
        assert_eq!(doc.undo_label(), Some("Forjados"));
        // The roof slab rests on the façade walls.
        let roof = m
            .slabs()
            .find(|s| m.slab_type(s.slab_type).unwrap().function == SlabFunction::Roof)
            .unwrap();
        let thickness = m.slab_type(roof.slab_type).unwrap().thickness();
        let level = m.level(roof.level).unwrap().elevation;
        assert!((level + roof.top_offset - thickness - super::WALL_HEIGHT).abs() < 1e-9);
    }
}
