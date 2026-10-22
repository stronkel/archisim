//! `archisim modelos`: every object of the library whose 3D ArchiSim makes
//! from its measurements, in rows by kind (living and dining room,
//! bedroom, kitchen, bathroom, people and cars), to see them in 3D, in an
//! axonometry, in plan and in the IFC.

use std::path::Path;

use archisim_core::{Model, NewObject, Point2};

type Res<T = ()> = Result<T, Box<dyn std::error::Error>>;

const ROWS: &[&[&str]] = &[
    &[
        "Sofá de 3 plazas",
        "Sofá clásico de 3 plazas",
        "Sofá nórdico de 2 plazas",
        "Sillón",
        "Mesa de centro",
        "Mesa de centro de vidrio",
        "Mueble TV 180",
        "Estantería 80",
        "Librería 160",
    ],
    &[
        "Mesa de comedor 160×90",
        "Mesa de caballetes 200×100",
        "Mesa de patas metálicas 140×80",
        "Silla",
        "Silla metálica",
        "Silla tapizada",
    ],
    &[
        "Cama doble 150×200",
        "Cama de madera 160×200",
        "Cama tarima 150×200",
        "Cama individual 90×190",
        "Mesita de noche",
        "Armario 120",
        "Cómoda 4 cajones",
        "Cómoda de patas 2×3 cajones",
        "Escritorio 140",
        "Escritorio metálico 120",
        "Balda 80",
    ],
    &[
        "Muebles de cocina 240",
        "Muebles altos de cocina 120",
        "Frigorífico 60",
        "Lavadora 60",
        "Encimera con cajonera 300",
        "Campana decorativa 60",
        "Campana integrada 60",
    ],
    &[
        "Inodoro",
        "Inodoro suspendido",
        "Bidé",
        "Bidé suspendido",
        "Lavabo 60",
        "Lavabo con mueble 80",
        "Lavabo suspendido 60",
        "Bañera 170×70",
        "Plato de ducha 90×90",
        "Ducha con mampara 120×80",
    ],
    &[
        "Espejo 80×100",
        "Espejo redondo 60",
        "Toallero 60",
        "Radiador toallero 50",
        "Portarrollos",
        "Barra de apoyo fija 70",
        "Barra de apoyo abatible 75",
    ],
    &[
        "Persona",
        "Mujer",
        "Niño",
        "Turismo",
        "Todoterreno",
        "Furgoneta",
    ],
];

pub fn build() -> Res<Model> {
    let mut m = archisim_library::new_project("Modelos 3D de ArchiSim");
    let level = m.levels()[0].id;
    let mut y = 0.0;
    for row in ROWS {
        let types: Vec<_> = row
            .iter()
            .map(|name| archisim_library::add_object_entry(&mut m, name))
            .collect::<Result<_, _>>()?;
        let sizes: Vec<(f64, f64)> = types
            .iter()
            .map(|t| {
                m.object_type_box(*t)
                    .map(|b| (b.1[0] - b.0[0], b.1[1] - b.0[1]))
                    .unwrap_or((1.0, 1.0))
            })
            .collect();
        // Each row in front of the last, with room for its deepest object
        // (a car runs forwards from its back by its length).
        let depth = sizes.iter().map(|s| s.1).fold(0.0, f64::max);
        y -= depth + 2.0;
        let mut x = 0.0;
        for (t, (w, _)) in types.iter().zip(&sizes) {
            m.add_object(NewObject {
                object_type: *t,
                level,
                // Turned round: their fronts (drawers, doors, handles) look
                // towards the axonometry's viewer, in the south.
                position: Point2::new(x + w / 2.0, y + 0.5 * depth),
                rotation: std::f64::consts::PI,
                base_offset: 0.0,
            })?;
            x += w + 0.8;
        }
    }
    Ok(m)
}

pub fn write(dir: &Path) -> Res {
    let m = build()?;
    std::fs::create_dir_all(dir)?;
    let project = dir.join(format!("modelos.{}", archisim_file::EXTENSION));
    archisim_file::save(&m, &project)?;
    let made = m
        .objects()
        .filter(|o| m.type_has_prop(o.object_type))
        .count();
    std::fs::write(
        dir.join("modelos.ifc"),
        crate::ifc_text(&m, "modelos", None),
    )?;
    let axo = archisim_core::Axo::default();
    let drawing = archisim_plan::axo::axo_drawing(&m, &axo, Default::default());
    let crop = archisim_plan::axo::axo_extent(&m, &axo, 50.0);
    std::fs::write(
        dir.join("modelos-3d.svg"),
        archisim_plan::axo::render_axo(&drawing, crop, 50.0),
    )?;
    let options = archisim_plan::PlanOptions {
        scale: 100.0,
        ..Default::default()
    };
    let level = m.levels()[0].id;
    std::fs::write(
        dir.join("modelos-planta.svg"),
        archisim_plan::render_plan(&m, level, &options),
    )?;
    println!(
        "Modelos: {} objetos, {made} con su 3D hecho por ArchiSim.",
        m.objects().count()
    );
    println!("  Proyecto: {}", project.display());
    println!("  Axonometría: {}", dir.join("modelos-3d.svg").display());
    println!("  IFC: {}", dir.join("modelos.ifc").display());
    Ok(())
}

#[cfg(test)]
mod tests {
    #[test]
    fn every_model_is_placed_with_its_3d() {
        let m = super::build().unwrap();
        let n = m.objects().count();
        assert_eq!(n, super::ROWS.iter().map(|r| r.len()).sum::<usize>());
        assert!(m.objects().all(|o| m.type_has_prop(o.object_type)));
        let solids = archisim_geom::solid::model_solids(&m);
        assert!(solids.len() > n * 2);
    }
}
