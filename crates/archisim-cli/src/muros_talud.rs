//! `archisim muros-talud`: walls of variable section. On the basement
//! level, a reinforced concrete retaining wall battered 1:10 on the earth
//! side with its footing (heel and toe), turning a corner in L with
//! another of the same, and a brick wall butting into it in T. On the
//! ground floor, a stone masonry wall battered on both faces, with a door,
//! meeting a plain brick wall in L. Sections across both, the plans, the
//! IFC and the stability checks of the retaining wall.

use std::path::Path;

use archisim_core::{
    CutPattern, Hand, Layer, LayerFunction, Model, NewOpening, NewSection, NewWall, Point2,
    ReferenceLine, SectionKind, WallFooting, WallFunction, WallSection, WallSide, WallTop,
};

type Res<T = ()> = Result<T, Box<dyn std::error::Error>>;

/// A number with a decimal comma.
fn dec(v: f64, places: usize) -> String {
    format!("{v:.places$}").replace('.', ",")
}

/// The material of that name, or a new one.
fn material(
    m: &mut Model,
    name: &str,
    category: &str,
    colour: [u8; 3],
    pattern: CutPattern,
) -> archisim_core::MaterialId {
    let found = m.materials().find(|x| x.name == name).map(|x| x.id);
    found.unwrap_or_else(|| m.add_material(name, category, colour, pattern))
}

pub fn build() -> Res<Model> {
    let mut m = Model::new("Muros en talud");
    let ground = m.add_level("Planta baja", 0.0);
    let basement = m.add_level("Sótano", -3.0);
    let lib = archisim_library::add_library(&mut m)?;
    let concrete = material(
        &mut m,
        "Hormigón armado",
        "concrete",
        [200, 200, 200],
        CutPattern::Cross,
    );
    let stone = material(
        &mut m,
        "Mampostería de piedra",
        "stone",
        [196, 186, 166],
        CutPattern::Diagonal,
    );
    let render = material(
        &mut m,
        "Enfoscado de cal",
        "plaster",
        [236, 230, 218],
        CutPattern::Dots,
    );

    // The retaining wall: 30 cm at the crown, 1:10 on the earth side
    // (the exterior), a footing 50 cm deep with 80 cm of heel and 50 of
    // toe.
    let retaining = m.add_wall_type(
        "Muro de contención HA 30",
        WallFunction::Exterior,
        vec![Layer {
            material: concrete,
            thickness: 0.30,
            function: LayerFunction::Structure,
        }],
    )?;
    m.set_wall_type_section(
        retaining,
        Some(WallSection {
            batter_ext: 0.1,
            batter_int: 0.0,
            layer: None,
            footing: Some(WallFooting {
                toe_ext: 0.8,
                toe_int: 0.5,
                depth: 0.5,
            }),
        }),
    )?;
    // The masonry wall: 45 cm of stone at the crown between a lime render
    // outside and plaster inside, 1:20 on both faces; the render follows
    // its face.
    let masonry = m.add_wall_type(
        "Muro de mampostería 50",
        WallFunction::Exterior,
        vec![
            Layer {
                material: render,
                thickness: 0.02,
                function: LayerFunction::Finish,
            },
            Layer {
                material: stone,
                thickness: 0.45,
                function: LayerFunction::Structure,
            },
            Layer {
                material: render,
                thickness: 0.015,
                function: LayerFunction::Finish,
            },
        ],
    )?;
    m.set_wall_type_section(
        masonry,
        Some(WallSection {
            batter_ext: 0.05,
            batter_int: 0.05,
            layer: None,
            footing: None,
        }),
    )?;

    let wall = |m: &mut Model, t, level, a: (f64, f64), b: (f64, f64), h: f64, base: f64| {
        m.add_wall(NewWall {
            wall_type: t,
            level,
            start: Point2::new(a.0, a.1),
            end: Point2::new(b.0, b.1),
            reference: ReferenceLine::Center,
            height: h,
            base_offset: base,
            bulge: 0.0,
            top: WallTop::Unconnected,
        })
    };
    // Basement: the earth to the north (+y) and to the east (+x): the
    // exterior face on the left of each wall.
    wall(
        &mut m,
        retaining,
        basement,
        (0.0, 0.0),
        (10.0, 0.0),
        3.5,
        0.0,
    )?;
    wall(
        &mut m,
        retaining,
        basement,
        (10.0, 0.0),
        (10.0, -6.0),
        3.5,
        0.0,
    )?;
    // A brick wall into the retaining one, on its footing.
    wall(
        &mut m,
        lib.bearing_wall,
        basement,
        (4.0, -6.0),
        (4.0, 0.0),
        3.0,
        0.5,
    )?;

    // Ground floor: the masonry wall and a brick one in L with it.
    let stone_wall = wall(&mut m, masonry, ground, (14.0, 0.0), (22.0, 0.0), 2.8, 0.0)?;
    wall(
        &mut m,
        lib.bearing_wall,
        ground,
        (22.0, 0.0),
        (22.0, -5.0),
        2.8,
        0.0,
    )?;
    m.add_opening(NewOpening {
        opening_type: lib.entrance_door,
        wall: stone_wall,
        along: 3.0,
        sill: 0.0,
        opens_to: WallSide::Interior,
        hand: Hand::Left,
        setback: None,
    })?;
    m.add_opening(NewOpening {
        opening_type: lib.window_one_leaf,
        wall: stone_wall,
        along: 6.0,
        sill: 0.9,
        opens_to: WallSide::Interior,
        hand: Hand::Left,
        setback: None,
    })?;

    // Sections across both, looking west.
    m.add_section(NewSection {
        name: "Contención".into(),
        kind: SectionKind::Section,
        start: Point2::new(2.0, -3.0),
        end: Point2::new(2.0, 3.0),
        depth: 2.0,
    })?;
    m.add_section(NewSection {
        name: "Mampostería".into(),
        kind: SectionKind::Section,
        start: Point2::new(15.5, -2.0),
        end: Point2::new(15.5, 2.0),
        depth: 1.0,
    })?;
    Ok(m)
}

pub fn write(dir: &Path) -> Res {
    let m = build()?;
    std::fs::create_dir_all(dir)?;
    let project = dir.join(format!("muros-talud.{}", archisim_file::EXTENSION));
    archisim_file::save(&m, &project)?;
    std::fs::write(
        dir.join("muros-talud.ifc"),
        crate::ifc_text(&m, "muros-talud", None),
    )?;
    println!("Muros de sección variable:");
    let idx = archisim_geom::WallIndex::new(&m);
    for w in m.walls() {
        let Some(f) = archisim_geom::wall_section::SectionFrame::of(&m, w.id) else {
            continue;
        };
        let name = m.wall_type(w.wall_type).map_or("", |t| t.name.as_str());
        let v: f64 = archisim_geom::solid::wall_solids_in(&idx, w.id)
            .iter()
            .map(|s| s.volume())
            .sum();
        println!(
            "  {name} ({} m): {} m en coronación, {} en la base del alzado, {} m³",
            dec(w.length(), 2),
            dec(f.crown, 2),
            dec(f.base_thickness(), 2),
            dec(v, 2)
        );
    }
    for c in archisim_struct::retaining::check(&m) {
        println!("  · {} — {}", c.what, c.rule);
    }
    let options = archisim_plan::PlanOptions {
        scale: 50.0,
        ..Default::default()
    };
    for level in m.levels() {
        let file = format!(
            "muros-talud-{}.svg",
            if level.elevation < 0.0 {
                "sotano"
            } else {
                "planta-baja"
            }
        );
        std::fs::write(
            dir.join(&file),
            archisim_plan::render_plan(&m, level.id, &options),
        )?;
        println!("  {} a 1:50: {}", level.name, dir.join(&file).display());
    }
    for (name, file) in [
        ("Contención", "muros-talud-seccion-contencion.svg"),
        ("Mampostería", "muros-talud-seccion-mamposteria.svg"),
    ] {
        if let Some(sec) = m.sections().find(|s| s.name == name) {
            let d = archisim_plan::section::section_drawing(&m, sec);
            std::fs::write(
                dir.join(file),
                archisim_plan::section::render_section(&m, &d, &options),
            )?;
            println!("  Sección {name} a 1:50: {}", dir.join(file).display());
        }
    }
    println!("  Proyecto: {}", project.display());
    println!("  IFC: {}", dir.join("muros-talud.ifc").display());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_walls_are_built_with_their_section() {
        let m = build().unwrap();
        let idx = archisim_geom::WallIndex::new(&m);
        let sectioned: Vec<_> = m
            .walls()
            .filter(|w| m.wall_section(w.id).is_some())
            .collect();
        assert_eq!(sectioned.len(), 3);
        for w in &sectioned {
            let f = archisim_geom::wall_section::SectionFrame::of(&m, w.id).unwrap();
            let v: f64 = archisim_geom::solid::wall_solids_in(&idx, w.id)
                .iter()
                .map(|s| s.volume())
                .sum();
            let whole = f.section.area(f.crown, f.height) * w.length();
            // Near the section times the length: less the doors and
            // windows, more or less at the corners.
            assert!(v > 0.6 * whole && v < 1.3 * whole, "{v} {whole}");
        }
        // The door of the stone wall goes right through it.
        let stone = sectioned
            .iter()
            .find(|w| {
                m.wall_type(w.wall_type)
                    .unwrap()
                    .name
                    .contains("mampostería")
            })
            .unwrap();
        let solids = archisim_geom::solid::wall_solids_in(&idx, stone.id);
        let at_door = solids.iter().flat_map(|s| s.triangles.iter()).any(|t| {
            t.iter()
                .all(|p| (p[0] - 14.0 - 3.0).abs() < 0.3 && p[2] < 1.0 && p[2] > 0.2)
        });
        assert!(!at_door, "nothing of the wall in the middle of the door");
        // The checks of the retaining wall use its own footing.
        let checks = archisim_struct::retaining::check(&m);
        assert!(
            checks.iter().any(|c| c.what.contains("zarpa de 1,9")),
            "{:#?}",
            checks.iter().map(|c| &c.what).collect::<Vec<_>>()
        );
        // The plans show the foot of the faces and the footing dashed.
        let basement = m.levels().into_iter().find(|l| l.elevation < 0.0).unwrap();
        let svg = archisim_plan::render_plan(&m, basement.id, &Default::default());
        assert!(svg.contains("wall-section-below"));
    }
}
