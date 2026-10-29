//! `archisim modelos`: every object of the library whose 3D ArchiSim makes
//! from its measurements, in rows by kind (living and dining room,
//! bedroom, kitchen, bathroom, people and cars), to see them in 3D, in an
//! axonometry, in plan and in the IFC.

use std::path::Path;

use archisim_core::{Model, NewObject, Point2};

type Res<T = ()> = Result<T, Box<dyn std::error::Error>>;

/// The pieces shown in each collection (a row each).
const IN_COLLECTIONS: &[&str] = &[
    "Sofá acogedor de 3 plazas",
    "Butaca de orejas",
    "Mesa auxiliar redonda",
    "Alfombra 200×140",
    "Aparador 160",
    "Lámpara de pie",
    "Cama doble 150×200",
    "Mesita de noche",
    "Taburete de madera",
    "Cojín de suelo",
    "Macetón con olivo",
];

const ROWS: &[&[&str]] = &[
    // Street furniture in more models and more of a house (v0.52).
    &[
        "Banco con respaldo",
        "Banco de hormigón y madera",
        "Banco de acero sin respaldo",
        "Farola 4,5 m",
        "Farola fernandina 3,5 m",
        "Farola de globo 4 m",
        "Baliza de jardín",
        "Papelera",
        "Papelera de listones",
        "Papelera doble de reciclaje",
        "Bolardo",
        "Bolardo de acero",
        "Bolardo cónico de hormigón",
        "Buzón",
        "Buzón sobre poste",
        "Buzones de portal",
    ],
    &[
        "Contenedor de residuos",
        "Iglú de vidrio",
        "Contenedor soterrado",
        "Fuente de beber",
        "Fuente de piedra",
        "Estanque con surtidor",
        "Jardinera 120",
        "Jardinera de madera con flores",
        "Jardinera de acero corten",
        "Alcorque 120",
        "Alcorque de pletinas",
        "Alcorque de piedra y grava",
        "Aparcabicis 5 plazas",
        "Aparcabicis de onda",
        "Aparcabicis de rueda",
        "Señal de tráfico",
        "Señal de peligro",
        "Panel informativo",
        "Bicicleta",
        "Bicicleta de carretera",
        "Bicicleta de carga",
    ],
    &[
        "Chimenea de obra",
        "Estufa de leña",
        "Chimenea moderna lineal",
        "Perchero de pie",
        "Recibidor con banco",
        "Silla de oficina",
        "Silla de oficina de malla",
        "Mesa de bistró con dos sillas",
        "Mesa de teca para seis",
        "Sofá de exterior con mesa",
        "Silla colgante de ratán",
        "Hamaca con bastidor",
    ],
    &[
        "Radiador de aluminio 80",
        "Radiador de panel 120",
        "Aire acondicionado de pared",
        "Ventilador de techo",
        "Ventilador de techo de ratán",
        "Piano de pared",
        "Piano de cola",
        "Cama infantil 90×190",
        "Litera 90×190",
        "Estantería con objetos 120",
    ],
    // The collections' pieces (v0.50).
    &[
        "Cama con cabecero tapizado 160",
        "Cama de lamas 160",
        "Mesita de patas cónicas",
        "Mesita flotante",
        "Taburete de madera",
        "Taburete tapizado",
        "Taburete de enea",
        "Isla de cocina 180",
        "Mesa de trabajo de cocina 140",
        "Isla con fregadero 240",
        "Tablas y aceite",
        "Botes de cristal",
        "Cafetera y tazas",
        "Hierbas aromáticas",
    ],
    &[
        "Toallas dobladas",
        "Toallas colgadas",
        "Toallas enrolladas",
        "Cesto de ropa",
        "Cesta con rollos",
        "Cesta con asas",
        "Cuadro 50×70",
        "Tríptico",
        "Espejo de arco",
        "Espejo sol",
        "Reloj de pared",
        "Cojín de suelo",
        "Zafu",
        "Pila de cojines de suelo",
        "Manta doblada",
        "Pila de mantas",
        "Escalera con mantas",
    ],
    &[
        "Tumbona de teca",
        "Tumbona de aluminio",
        "Cama de exterior",
        "Sombrilla redonda 250",
        "Sombrilla descentrada 300",
        "Sombrilla de paja 200",
        "Macetón con olivo",
        "Ánfora de barro",
        "Macetón con gramíneas",
        "Cuenco con lavanda",
        "Pérgola de madera 3×3",
        "Pérgola de lamas 4×3",
        "Pérgola con cañizo 3×3",
    ],
    // The cosy pieces (v0.49).
    &[
        "Sofá acogedor de 3 plazas",
        "Sofá nórdico con cojines",
        "Butaca de orejas",
        "Butaca redonda",
        "Butaca de ratán",
        "Butaca baja de patas",
        "Puf redondo",
        "Puf de punto",
        "Puf cubo",
        "Mesa auxiliar redonda",
        "Mesa auxiliar de tres patas",
        "Mesas nido",
        "Aparador 160",
        "Aparador de lamas 180",
    ],
    &[
        "Alfombra 200×140",
        "Alfombra redonda 160",
        "Alfombra de rayas 240×170",
        "Alfombra bereber 300×200",
        "Lámpara de pie",
        "Lámpara de mesa",
        "Lámpara colgante",
        "Lámpara de arco",
        "Costilla de Adán",
        "Ficus en maceta",
        "Kentia",
        "Helecho",
        "Sansevieria",
        "Cortinas de lino 160",
        "Visillo 160",
        "Jarrón con ramas",
        "Frutero",
        "Libros y vela",
        "Cerámicas",
        "Tetera y cuencos",
    ],
    &[
        "Sofá de 3 plazas",
        "Sofá rinconera 260×170",
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
    let mut rows: Vec<Vec<archisim_core::ObjectTypeId>> = Vec::new();
    for c in archisim_library::collections::COLLECTIONS {
        rows.push(
            IN_COLLECTIONS
                .iter()
                .map(|name| archisim_library::collections::add_collection_entry(&mut m, name, c))
                .collect::<Result<_, _>>()?,
        );
    }
    for row in ROWS {
        rows.push(
            row.iter()
                .map(|name| archisim_library::add_object_entry(&mut m, name))
                .collect::<Result<_, _>>()?,
        );
    }
    for types in rows {
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
        let collections = archisim_library::collections::COLLECTIONS.len();
        assert_eq!(
            n,
            super::ROWS.iter().map(|r| r.len()).sum::<usize>()
                + collections * super::IN_COLLECTIONS.len()
        );
        assert!(m.objects().all(|o| m.type_has_prop(o.object_type)));
        let solids = archisim_geom::solid::model_solids(&m);
        assert!(solids.len() > n * 2);
    }
}
