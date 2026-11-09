//! `archisim tierras`: earthworks on a hillside. A slope rising 12 % to
//! the north; a house with a garage in a basement under part of it, with
//! its platforms made from its slabs (the ground floor's and the dig of
//! the basement); a terrace platform drawn by hand to the west; a ramp
//! down from the street to the garage; a platform for the pool to the
//! east and the pool dug in it. The project, its IFC 4.3 (with every cut
//! and fill as `IfcEarthworksCut` and `IfcEarthworksFill`) and IFC4, the
//! plan with the cut and fill coloured, and the earthworks report.

use std::path::Path;

use archisim_core::{
    CutPattern, Model, NewPool, NewSlab, NewTerrain, NewWall, Point2, ReferenceLine, SurveyPoint,
    TerrainPad, WallTop,
};

type Res<T = ()> = Result<T, Box<dyn std::error::Error>>;

/// Height of the ground as surveyed: 12 % up to the north, a little to
/// the east.
pub fn ground_z(x: f64, y: f64) -> f64 {
    -2.0 + 0.12 * y + 0.01 * x
}

fn rect(x0: f64, y0: f64, x1: f64, y1: f64) -> Vec<Point2> {
    vec![
        Point2::new(x0, y0),
        Point2::new(x1, y0),
        Point2::new(x1, y1),
        Point2::new(x0, y1),
    ]
}

pub fn build() -> Res<Model> {
    let mut m = Model::new("Ladera con explanaciones");
    let ground = m.add_level("Planta baja", 0.0);
    let basement = m.add_level("Sótano", -3.0);
    let lib = archisim_library::add_library(&mut m)?;
    let earth = m
        .materials()
        .find(|x| x.name.to_lowercase().contains("tierra"))
        .map(|x| x.id);
    let earth = earth
        .unwrap_or_else(|| m.add_material("Tierra", "earth", [150, 120, 90], CutPattern::Dots));
    // The survey: every 5 m over 60 × 45 m.
    let mut points = Vec::new();
    for i in 0..=12 {
        for j in 0..=9 {
            let (x, y) = (i as f64 * 5.0, j as f64 * 5.0);
            points.push(SurveyPoint {
                at: Point2::new(x, y),
                z: ground_z(x, y),
            });
        }
    }
    let terrain = m.add_terrain(NewTerrain {
        level: ground,
        points,
        material: earth,
        depth: 3.0,
        contour: 0.5,
        lines: Vec::new(),
    })?;
    // The house: 14 × 10 m on the ground floor; the garage under its
    // east part, 8 × 10 m.
    m.add_slab(NewSlab {
        slab_type: lib.ground_slab,
        level: ground,
        outline: rect(18.0, 20.0, 32.0, 30.0),
        top_offset: 0.0,
    })?;
    m.add_slab(NewSlab {
        slab_type: lib.ground_slab,
        level: basement,
        outline: rect(24.0, 20.0, 32.0, 30.0),
        top_offset: 0.0,
    })?;
    // Their walls (the façade's outer face out: clockwise), 3 m high.
    for (level, wall_type, (x0, y0, x1, y1)) in [
        (ground, lib.facade, (18.0, 20.0, 32.0, 30.0)),
        (basement, lib.bearing_wall, (24.0, 20.0, 32.0, 30.0)),
    ] {
        let c = [(x0, y0), (x0, y1), (x1, y1), (x1, y0)];
        for i in 0..4 {
            let (a, b) = (c[i], c[(i + 1) % 4]);
            m.add_wall(NewWall {
                wall_type,
                level,
                start: Point2::new(a.0, a.1),
                end: Point2::new(b.0, b.1),
                reference: ReferenceLine::Center,
                height: 3.0,
                base_offset: 0.0,
                bulge: 0.0,
                top: WallTop::Unconnected,
            })?;
        }
    }
    let mut pads = m.building_pads(terrain, archisim_core::PAD_MARGIN, archisim_core::PAD_SLOPE);
    for p in &mut pads {
        p.name = match p.kind {
            archisim_core::PadKind::Basement => "Garaje".into(),
            _ => "Plataforma de la vivienda".into(),
        };
    }
    // The terrace to the west, at +0,60, with banks 3:2: dug uphill,
    // filled downhill.
    pads.push(TerrainPad {
        name: "Terraza".into(),
        ..TerrainPad::flat(rect(6.0, 16.0, 17.0, 30.0), 0.6, 1.0 / 1.5)
    });
    // The ramp from the street down to the garage, 3,50 m wide: from the
    // pavement (at the ground there) to the garage's floor.
    let (start, end) = (Point2::new(28.0, 3.0), Point2::new(28.0, 19.5));
    let mut ramp =
        TerrainPad::ramp(start, end, 3.5, ground_z(28.0, 3.0), -3.0 - 0.3, 1.0).ok_or("rampa")?;
    ramp.name = "Rampa de acceso al garaje".into();
    pads.push(ramp);
    // The pool's platform to the east, at +0,15.
    pads.push(TerrainPad {
        name: "Plataforma de la piscina".into(),
        ..TerrainPad::flat(rect(35.0, 17.0, 50.0, 29.0), 0.15, 1.0)
    });
    m.edit_terrain(terrain, |t| {
        t.set_pads(pads);
        t.show_earthworks = true;
    })?;
    // The pool: 8 × 4 m, its coping at the platform.
    let lining = m.add_material("Gresite azul", "ceramic", [70, 150, 200], CutPattern::Solid);
    let mut pool = NewPool::new(ground, rect(38.5, 21.0, 46.5, 25.0), lining);
    pool.name = "Piscina".into();
    pool.top_offset = 0.15;
    m.add_pool(pool)?;
    Ok(m)
}

pub fn write(dir: &Path) -> Res {
    let m = build()?;
    std::fs::create_dir_all(dir)?;
    let project = dir.join(format!("tierras.{}", archisim_file::EXTENSION));
    archisim_file::save(&m, &project)?;
    std::fs::write(
        dir.join("tierras.ifc"),
        crate::ifc_text(&m, "tierras", None),
    )?;
    std::fs::write(
        dir.join("tierras-ifc4.ifc"),
        crate::ifc_text_in(&m, "tierras-ifc4", None, archisim_ifc::Schema::Ifc4),
    )?;
    let level = m.terrains().next().ok_or("terreno")?.level;
    let options = archisim_plan::PlanOptions {
        scale: 200.0,
        ..Default::default()
    };
    let plan = dir.join("tierras-planta.svg");
    std::fs::write(&plan, archisim_plan::render_plan(&m, level, &options))?;
    let report = archisim_plan::terrain::earthworks_report(&m);
    std::fs::write(dir.join("tierras-informe.txt"), &report)?;
    print!("{report}");
    println!("  Proyecto: {}", project.display());
    println!("  IFC 4.3: {}", dir.join("tierras.ifc").display());
    println!("  IFC4: {}", dir.join("tierras-ifc4.ifc").display());
    println!(
        "  Planta a 1:200 con desmonte y terraplén: {}",
        plan.display()
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_hillside_is_graded_and_its_earthworks_written() {
        let m = build().unwrap();
        let t = m.terrains().next().unwrap();
        assert_eq!(t.pads.len(), 5);
        let works = m.earthworks();
        // The house cuts (uphill), the basement digs, the ramp digs, the
        // terrace and the pool's platform cut and fill, the pool digs.
        let names: Vec<&str> = works.iter().map(|e| e.name.as_str()).collect();
        for want in [
            "Desmonte · Plataforma de la vivienda",
            "Vaciado · Garaje",
            "Desmonte · Rampa de acceso al garaje",
            "Terraplén · Terraza",
            "Vaciado · Piscina",
        ] {
            assert!(names.contains(&want), "{want}: {names:?}");
        }
        for e in &works {
            assert_eq!(e.mesh.open_edges(), 0, "{}", e.name);
            let v = e.mesh.solid_volume();
            assert!(
                (v - e.mesh.volume).abs() < 1e-4 * v.max(1.0),
                "{} {v} {} {}",
                e.name,
                e.mesh.volume,
                e.mesh.triangles.len()
            );
        }
        // The garage's dig: about 9 × 11 m by 3 m and more, and its banks.
        let garage = works.iter().find(|e| e.name == "Vaciado · Garaje").unwrap();
        assert!(garage.mesh.volume > 150.0, "{}", garage.mesh.volume);
        // The ramp goes down to the garage: its graded ground at its ends.
        let z = t.height_at(Point2::new(28.0, 18.0)).unwrap();
        assert!(z < -2.9 && z > -3.4, "{z}");
        let ifc = crate::ifc_text(&m, "tierras", None);
        assert!(ifc.contains("IFCEARTHWORKSCUT(") && ifc.contains(".EXCAVATION.)"));
        assert!(ifc.contains("IFCEARTHWORKSFILL("));
        let report = archisim_plan::terrain::earthworks_report(&m);
        assert!(report.contains("Compensación"), "{report}");
        let svg = archisim_plan::render_plan(&m, t.level, &Default::default());
        assert!(svg.contains("terrain-ramp") && svg.contains("terrain-fill"));
    }
}
