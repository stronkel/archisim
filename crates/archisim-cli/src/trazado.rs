//! `archisim trazado`: streets laid stretch by stretch over a valley
//! between two hills, as the road tool lays them: an avenue nearly level
//! that crosses the valley on a viaduct and runs between retaining walls
//! through the hills, a street that climbs the hill at no more than 8 %
//! (on embankment and in cut), and a street along the valley's floor that
//! passes under the viaduct without a crossing. To look at in 3D and in
//! the IFC.

use std::path::Path;

use archisim_core::{Model, NewTerrain, Point2, RoadSection, RoadStretch, SurveyPoint};

type Res<T = ()> = Result<T, Box<dyn std::error::Error>>;

/// The ground: a valley 12 m deep along x = 60, hills either side and a
/// rise to the north.
fn ground(p: Point2) -> f64 {
    let valley = -12.0 * (-((p.x - 60.0).powi(2)) / 600.0).exp();
    valley + 0.04 * p.y + 4.0 * (-((p.x - 130.0).powi(2) + (p.y - 30.0).powi(2)) / 1200.0).exp()
}

fn preset(name: &str) -> RoadSection {
    archisim_core::road_presets()
        .into_iter()
        .find(|(n, _)| *n == name)
        .map(|(_, s)| s)
        .unwrap_or_default()
}

pub fn build() -> Res<Model> {
    let mut m = Model::new("Trazado de calles");
    let pb = m.add_level("Planta baja", 0.0);
    archisim_library::add_library(&mut m)?;
    let mat = |m: &Model, c: &str| m.materials().find(|x| x.category == c).map(|x| x.id);
    let earth = mat(&m, "earth").unwrap_or_else(|| {
        m.add_material(
            "Terreno natural",
            "earth",
            [150, 160, 110],
            archisim_core::CutPattern::Dots,
        )
    });
    let asphalt = mat(&m, "asphalt")
        .or_else(|| mat(&m, "concrete"))
        .ok_or("sin asfalto")?;
    let stone = mat(&m, "stone").unwrap_or(asphalt);
    let paving = mat(&m, "ceramic").unwrap_or(stone);
    let mut points = Vec::new();
    for i in -2..=20 {
        for j in -8..=10 {
            let at = Point2::new(i as f64 * 10.0, j as f64 * 10.0);
            points.push(SurveyPoint { at, z: ground(at) });
        }
    }
    m.add_terrain(NewTerrain {
        level: pb,
        points,
        material: earth,
        depth: 1.0,
        contour: 1.0,
        lines: Vec::new(),
    })?;
    let lay = |m: &mut Model,
               section: &RoadSection,
               grade: f64,
               clicks: &[Point2],
               curve: Option<Point2>|
     -> Res<archisim_core::RoadId> {
        // As the tool does: one stretch per click, going on from the last.
        let mut chain = None;
        let mut id = None;
        for w in clicks.windows(2) {
            let points = match curve {
                Some(c) if w[1] == *clicks.last().unwrap_or(&w[1]) => {
                    archisim_core::curve_points(w[0], c, w[1], 2.0)
                }
                _ => w.to_vec(),
            };
            let r = m.add_road_stretch(RoadStretch {
                level: pb,
                points,
                continues: chain,
                section: section.clone(),
                top_offset: 0.0,
                pavement: asphalt,
                kerb_material: stone,
                sidewalk_material: paving,
                follow_ground: Some(grade),
            })?;
            chain = Some((r, true));
            id = Some(r);
        }
        id.ok_or_else(|| "sin tramos".into())
    };
    // The avenue across the valley, nearly level.
    let avenue = preset("Avenida con mediana arbolada");
    lay(
        &mut m,
        &avenue,
        0.015,
        &[
            Point2::new(-10.0, 0.0),
            Point2::new(40.0, 0.0),
            Point2::new(90.0, 0.0),
            Point2::new(150.0, 0.0),
            Point2::new(190.0, 0.0),
        ],
        None,
    )?;
    // A street up the hill, at 8 %, with a curve at its top.
    let street = preset("Calle de doble sentido con aparcamiento");
    lay(
        &mut m,
        &street,
        0.08,
        &[
            Point2::new(150.0, 0.0),
            Point2::new(150.0, 40.0),
            Point2::new(110.0, 90.0),
        ],
        Some(Point2::new(150.0, 85.0)),
    )?;
    // Along the valley's floor, under the viaduct.
    lay(
        &mut m,
        &preset("Calle de un sentido sin aparcamiento"),
        0.06,
        &[Point2::new(60.0, -75.0), Point2::new(60.0, 95.0)],
        None,
    )?;
    Ok(m)
}

pub fn write(dir: &Path) -> Res {
    let m = build()?;
    std::fs::create_dir_all(dir)?;
    let project = dir.join(format!("trazado.{}", archisim_file::EXTENSION));
    archisim_file::save(&m, &project)?;
    std::fs::write(
        dir.join("trazado.ifc"),
        crate::ifc_text(&m, "trazado", None),
    )?;
    let pb = m.levels()[0].id;
    let junctions = archisim_geom::crossroads::junctions(&m, pb);
    println!(
        "Trazado: {} calles, {} cruces.",
        m.roads().count(),
        junctions.len()
    );
    for r in m.roads() {
        let g: Vec<String> = r
            .grades()
            .iter()
            .map(|g| format!("{:.1} %", g.2 * 100.0).replace('.', ","))
            .collect();
        println!(
            "  {}: {:.0} m, rasante {}",
            r.name,
            r.length(),
            g.join(" · ")
        );
    }
    println!("  Proyecto: {}", project.display());
    println!("  IFC: {}", dir.join("trazado.ifc").display());
    Ok(())
}
