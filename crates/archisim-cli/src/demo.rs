//! A small single-storey house used to test exports end to end.
//!
//! 12 × 9 m outline drawn clockwise along the exterior faces, with a typical
//! Spanish double-leaf façade and plasterboard-free brick partitions.

use archisim_core::{
    CutPattern, Document, Layer, LayerFunction, Model, ModelError, NewWall, Point2, ReferenceLine,
    WallFunction,
};

pub fn build() -> Result<Document, ModelError> {
    let mut doc = Document::new(Model::new("Vivienda de prueba"));

    let (pb, fachada, tabique) = doc.transact("Configurar proyecto", |m| {
        let pb = m.add_level("Planta baja", 0.0);
        m.add_level("Cubierta", 3.0);

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
        Ok((pb, fachada, tabique))
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
                height: 3.0,
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

    Ok(doc)
}

#[cfg(test)]
mod tests {
    #[test]
    fn demo_builds() {
        let doc = super::build().unwrap();
        assert_eq!(doc.model().walls().count(), 7);
        assert_eq!(doc.undo_label(), Some("Tabiquería"));
    }
}
