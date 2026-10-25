//! `archisim figuras`: what C1 draws (24/10/2026): trees, palms and
//! shrubs, people and cars in a row in front of a wall, seen from the
//! south as an elevation (technical lines and presentation colour) and
//! in axonometry, to see them drawn as architects draw them.

use std::path::Path;

use archisim_core::{
    Model, NewObject, NewSection, NewWall, Point2, ReferenceLine, SectionKind, Value, WallTop,
};

type Res<T = ()> = Result<T, Box<dyn std::error::Error>>;

/// The examples of `docs/ejemplos` (drawn for ArchiSim by
/// `scripts/make_elevation_examples.py`).
const TREE_DXF: &[u8] = include_bytes!("../../../docs/ejemplos/arbol-alzado.dxf");
const PERSON_PNG: &[u8] = include_bytes!("../../../docs/ejemplos/persona-alzado.png");
const TREE_PLAN_PNG: &[u8] = include_bytes!("../../../docs/ejemplos/arbol-planta.png");

/// The row, from the west: the library's type and the room it takes.
const ROW: &[(&str, f64)] = &[
    ("Ciprés", 2.4),
    ("Olivo", 5.5),
    ("Persona", 1.0),
    ("Mujer", 1.0),
    ("Turismo", 2.6),
    ("Palmera datilera", 5.0),
    ("Niño", 1.0),
    ("Pino piñonero", 7.5),
    ("Todoterreno", 2.8),
    ("Adelfa", 2.6),
    ("Almendro en flor", 5.0),
    ("Bicicleta", 1.2),
    ("Persona", 1.0),
    ("Romero", 1.4),
];

pub fn build() -> Res<Model> {
    let mut m = archisim_library::new_project("Figuras en alzado");
    let level = m.levels()[0].id;
    let wall = m
        .wall_types()
        .find(|t| t.name.contains("Tabique"))
        .or_else(|| m.wall_types().next())
        .map(|t| t.id)
        .ok_or("sin muros")?;
    let total: f64 = ROW.iter().map(|(_, w)| w).sum::<f64>() + 5.2;
    // A low wall behind them, to see what they hide.
    m.add_wall(NewWall {
        wall_type: wall,
        level,
        start: Point2::new(-1.0, 3.0),
        end: Point2::new(total + 1.0, 3.0),
        reference: ReferenceLine::InteriorFace,
        height: 2.4,
        base_offset: 0.0,
        bulge: 0.0,
        top: WallTop::Unconnected,
    })?;
    let mut x = 0.0;
    let mut walkers = 0;
    for (name, room) in ROW {
        let t = archisim_library::add_object_entry(&mut m, name)?;
        // Cars stand across the view, people look at it; the second
        // «Persona» walks.
        let (rotation, at) = if name.contains("Turismo") || name.contains("Todoterreno") {
            (-std::f64::consts::FRAC_PI_2, Point2::new(x, 0.0))
        } else {
            (0.0, Point2::new(x + room / 2.0, 0.0))
        };
        let t = if *name == "Persona" {
            walkers += 1;
            if walkers == 2 {
                let ty = m.object_type(t).cloned().ok_or("sin tipo")?;
                let mut v = ty.values.clone();
                v.insert("model3d".into(), Value::Text("Caminando".into()));
                let walking = format!("{} caminando", ty.name);
                let found = m.object_types().find(|x| x.name == walking).map(|x| x.id);
                match found {
                    Some(id) => id,
                    None => m.add_object_type(walking, ty.family, v)?,
                }
            } else {
                t
            }
        } else {
            t
        };
        m.add_object(NewObject {
            level,
            object_type: t,
            position: at,
            rotation,
            base_offset: 0.0,
        })?;
        x += room;
    }
    // At the end, an olive with a drawing of its own (DXF) and a person
    // with a picture of their own (PNG): an office's drawings.
    let own = [
        ("Olivo", "Olivo dibujado", false, 4.0),
        ("Persona", "Persona en imagen", true, 1.2),
    ];
    for (base, name, image, room) in own {
        let t = archisim_library::add_object_entry(&mut m, base)?;
        let ty = m.object_type(t).cloned().ok_or("sin tipo")?;
        let nt = m.add_object_type(name, ty.family, ty.values.clone())?;
        let figure = if image {
            archisim_export::symbol::elevation_from_png(PERSON_PNG, "persona-alzado.png", false)?
        } else {
            archisim_export::symbol::elevation_from_dxf(TREE_DXF, "arbol-alzado.dxf", false)?
        };
        m.set_object_type_elevation(nt, vec![figure])?;
        if !image {
            // And seen from above, a picture for presentation plans.
            if let archisim_core::ElevationArt::Image { png, aspect } =
                archisim_export::symbol::elevation_from_png(
                    TREE_PLAN_PNG,
                    "arbol-planta.png",
                    false,
                )?
                .art
            {
                m.set_object_type_plan_image(
                    nt,
                    Some(archisim_core::PlanImage {
                        source: "arbol-planta.png".into(),
                        png,
                        aspect,
                    }),
                )?;
            }
        }
        m.add_object(NewObject {
            level,
            object_type: nt,
            position: Point2::new(x + room / 2.0, 0.0),
            rotation: 0.0,
            base_offset: 0.0,
        })?;
        x += room;
    }
    let total = x;
    m.add_section(NewSection {
        name: "Sur".into(),
        kind: SectionKind::Elevation,
        start: Point2::new(-2.0, -4.0),
        end: Point2::new(total + 2.0, -4.0),
        depth: 9.0,
    })?;
    Ok(m)
}

pub fn write(dir: &Path) -> Res {
    let m = build()?;
    std::fs::create_dir_all(dir)?;
    let project = dir.join(format!("figuras.{}", archisim_file::EXTENSION));
    archisim_file::save(&m, &project)?;
    std::fs::write(
        dir.join("figuras.ifc"),
        crate::ifc_text(&m, "figuras", None),
    )?;
    let sec = m.sections().next().ok_or("sin alzado")?;
    let drawing = archisim_plan::section::section_drawing(&m, sec);
    let presentation = m.presentation_styles().next().map(|p| p.id);
    for (suffix, style) in [("tecnico", None), ("color", presentation)] {
        let mut options = archisim_plan::PlanOptions {
            scale: 50.0,
            ..Default::default()
        };
        options.style.presentation = style;
        std::fs::write(
            dir.join(format!("figuras-alzado-{suffix}.svg")),
            archisim_plan::section::render_section(&m, &drawing, &options),
        )?;
    }
    // The row in plan, in a presentation style (the olive by its picture).
    let mut plan = archisim_plan::PlanOptions {
        scale: 100.0,
        ..Default::default()
    };
    plan.style.presentation = presentation;
    std::fs::write(
        dir.join("figuras-planta-color.svg"),
        archisim_plan::render_plan(&m, m.levels()[0].id, &plan),
    )?;
    let axo = archisim_core::Axo::default();
    let drawing = archisim_plan::axo::axo_drawing(&m, &axo, Default::default());
    let crop = archisim_plan::axo::axo_extent(&m, &axo, 50.0);
    std::fs::write(
        dir.join("figuras-3d.svg"),
        archisim_plan::axo::render_axo(&drawing, crop, 50.0),
    )?;
    println!(
        "Figuras: {} árboles, personas y coches delante de un muro.",
        m.objects().count()
    );
    println!("  Proyecto: {}", project.display());
    println!(
        "  Alzado 1:50 técnico y en color: figuras-alzado-tecnico.svg, figuras-alzado-color.svg"
    );
    println!("  Planta 1:100 de presentación: figuras-planta-color.svg");
    println!("  Axonometría: figuras-3d.svg");
    Ok(())
}

#[cfg(test)]
mod tests {
    /// Every figure is drawn in the elevation (none left out), and the
    /// plants are not drawn by their mesh: their drawing is light.
    #[test]
    fn the_row_is_seen_from_the_south() {
        let m = super::build().unwrap();
        assert_eq!(m.objects().count(), super::ROW.len() + 2);
        let sec = m.sections().next().unwrap();
        let d = archisim_plan::section::section_drawing(&m, sec);
        let svg = archisim_plan::section::render_section(&m, &d, &Default::default());
        assert!(svg.len() < 4_000_000, "{} bytes", svg.len());
        // The office's drawings: the person has only a picture, so it is
        // drawn by it in technical views too; in a presentation, as well.
        assert_eq!(svg.matches("<image").count(), 1);
        let mut options = archisim_plan::PlanOptions::default();
        options.style.presentation = m.presentation_styles().next().map(|p| p.id);
        let colour = archisim_plan::section::render_section(&m, &d, &options);
        assert_eq!(colour.matches("<image").count(), 1);
        // In a presentation plan, the olive by its picture.
        let mut plan = archisim_plan::PlanOptions::default();
        let level = m.levels()[0].id;
        plan.style.presentation = options.style.presentation;
        assert!(archisim_plan::render_plan(&m, level, &plan).contains("<image"));
        for o in m.objects() {
            assert!(
                d.pick_object(o.id),
                "{:?} no se ve",
                m.object_type(o.object_type).map(|t| &t.name)
            );
        }
    }
}
