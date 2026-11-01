//! `archisim calles`: a few streets on a slope that meet every way they
//! can (U1): an avenue with a tree-lined median crossed by a street that
//! climbs the hill (a crossing), a one-way street that ends on the avenue
//! (a T), a street between the two at the top (two T's) and two
//! pedestrian streets that meet at a corner (an L). The streets that
//! climb follow the terrain («rasante»). To see the corners, the kerbs
//! and the grades in plan, in 3D and in the IFC (with its alignments).

use std::path::Path;

use archisim_core::{
    MaterialId, Model, NewRoad, NewTerrain, NewWall, Point2, ReferenceLine, RoadId, RoadSection,
    SurveyPoint, WallTop,
};

type Res<T = ()> = Result<T, Box<dyn std::error::Error>>;

fn material(m: &Model, category: &str) -> Option<MaterialId> {
    m.materials().find(|x| x.category == category).map(|x| x.id)
}

fn preset(name: &str) -> RoadSection {
    archisim_core::road_presets()
        .into_iter()
        .find(|(n, _)| *n == name)
        .map(|(_, s)| s)
        .unwrap_or_default()
}

/// The height of the hill: 3 % up to the north and a rise to the
/// north-west.
fn ground(p: Point2) -> f64 {
    0.03 * p.y + 2.0 * (-((p.x + 60.0).powi(2) + (p.y - 60.0).powi(2)) / 1500.0).exp()
}

pub fn build() -> Res<Model> {
    let mut m = Model::new("Calles de ArchiSim");
    let pb = m.add_level("Planta baja", 0.0);
    let p1 = m.add_level("Planta primera", 3.0);
    let lib = archisim_library::add_library(&mut m)?;
    let earth = material(&m, "earth").unwrap_or_else(|| {
        m.add_material(
            "Terreno natural",
            "earth",
            [176, 156, 120],
            archisim_core::CutPattern::Dots,
        )
    });
    let asphalt = material(&m, "asphalt")
        .or_else(|| material(&m, "concrete"))
        .ok_or("sin asfalto")?;
    let stone = material(&m, "stone").unwrap_or(asphalt);
    let paving = material(&m, "ceramic").unwrap_or(stone);
    let paint = m.add_material(
        "Pintura vial blanca",
        "paint",
        [245, 245, 240],
        archisim_core::CutPattern::Solid,
    );
    // The hill, a point every 10 m.
    let mut points = Vec::new();
    for i in -10..=10 {
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
        contour: 0.5,
        lines: Vec::new(),
    })?;
    let road = |m: &mut Model, name: &str, sec: RoadSection, axis: Vec<Point2>, top: f64| {
        let mut sec = sec;
        sec.markings = Some(paint);
        m.add_road(NewRoad {
            name: name.into(),
            level: pb,
            axis,
            section: sec,
            top_offset: top,
            profile: Vec::new(),

            changes: Vec::new(),
            pavement: asphalt,
            kerb_material: stone,
            sidewalk_material: paving,
        })
    };
    let p = Point2::new;
    let avenue = road(
        &mut m,
        "Avenida del Mar",
        preset("Avenida con mediana arbolada"),
        vec![p(-95.0, 0.0), p(95.0, 0.0)],
        0.0,
    )?;
    let climbing: Vec<RoadId> = vec![
        road(
            &mut m,
            "Calle Real",
            preset("Calle de doble sentido con aparcamiento"),
            vec![p(0.0, -75.0), p(0.0, 95.0)],
            0.0,
        )?,
        road(
            &mut m,
            "Calle del Agua",
            preset("Calle de un sentido con aparcamiento"),
            vec![p(60.0, 0.0), p(60.0, 95.0)],
            0.0,
        )?,
    ];
    road(
        &mut m,
        "Calle Nueva",
        preset("Calle de un sentido con aparcamiento"),
        vec![p(0.0, 60.0), p(60.0, 60.0)],
        ground(p(30.0, 60.0)),
    )?;
    road(
        &mut m,
        "Calle de la Plaza",
        preset("Calle de plataforma única"),
        vec![p(-90.0, 30.0), p(-35.0, 30.0)],
        ground(p(-60.0, 30.0)),
    )?;
    road(
        &mut m,
        "Calle del Pozo",
        preset("Calle de plataforma única"),
        vec![p(-35.0, 30.0), p(-35.0, 80.0)],
        ground(p(-35.0, 55.0)),
    )?;
    // The streets that climb follow the hill.
    for &id in &climbing {
        let profile = m
            .road_profile_from_terrain(id, 20.0)
            .ok_or("sin terreno bajo la calle")?;
        m.edit_road(id, |r| r.profile = profile)?;
    }
    let _ = avenue;
    // Where the sections change (U2): the Calle Real widens its sidewalks
    // south of the avenue, over 10 m; the Calle del Agua becomes a
    // single-level street at the top.
    let real = climbing[0];
    let wide: Vec<archisim_core::RoadStrip> = m
        .road(real)
        .ok_or("sin calle")?
        .section
        .strips
        .iter()
        .map(|x| match x.band {
            archisim_core::RoadBand::Sidewalk => archisim_core::strip_of(x.band, 4.5),
            archisim_core::RoadBand::Parking => archisim_core::strip_of(x.band, 1.0),
            _ => *x,
        })
        .collect();
    m.edit_road(real, |r| {
        // From the start (the south end) up to 40 m wide, then back.
        r.changes = vec![archisim_core::SectionChange {
            at: 40.0,
            transition: 10.0,
            strips: r.section.strips.clone(),
        }];
        r.section.strips = wide;
    })?;
    let single = preset("Calle de plataforma única").strips;
    m.edit_road(climbing[1], |r| {
        r.changes = vec![archisim_core::SectionChange {
            at: 75.0,
            transition: 6.0,
            strips: single,
        }];
    })?;
    // A house on the block between them.
    let corners = [p(15.0, 15.0), p(35.0, 15.0), p(35.0, 28.0), p(15.0, 28.0)];
    let base = ground(p(25.0, 20.0));
    for i in 0..4 {
        m.add_wall(NewWall {
            wall_type: lib.bearing_wall,
            level: pb,
            start: corners[i],
            end: corners[(i + 1) % 4],
            reference: ReferenceLine::ExteriorFace,
            height: 3.0,
            base_offset: base,
            bulge: 0.0,
            top: WallTop::Level {
                level: p1,
                offset: base,
            },
        })?;
    }
    Ok(m)
}

pub fn write(dir: &Path) -> Res {
    let m = build()?;
    std::fs::create_dir_all(dir)?;
    let project = dir.join(format!("calles.{}", archisim_file::EXTENSION));
    archisim_file::save(&m, &project)?;
    std::fs::write(dir.join("calles.ifc"), crate::ifc_text(&m, "calles", None))?;
    let axo = archisim_core::Axo::default();
    let drawing = archisim_plan::axo::axo_drawing(&m, &axo, Default::default());
    let crop = archisim_plan::axo::axo_extent(&m, &axo, 500.0);
    std::fs::write(
        dir.join("calles-3d.svg"),
        archisim_plan::axo::render_axo(&drawing, crop, 500.0),
    )?;
    let pb = m.levels()[0].id;
    let options = archisim_plan::PlanOptions {
        scale: 500.0,
        ..Default::default()
    };
    std::fs::write(
        dir.join("calles-planta.svg"),
        archisim_plan::render_plan(&m, pb, &options),
    )?;
    let junctions = archisim_geom::crossroads::junctions(&m, pb);
    println!(
        "Calles: {} viales, {} cruces ({}).",
        m.roads().count(),
        junctions.len(),
        junctions
            .iter()
            .map(|j| match j.arms.len() {
                4 => "en cruz",
                3 => "en T",
                _ => "en L",
            })
            .collect::<Vec<_>>()
            .join(", ")
    );
    for r in m.roads() {
        let grades: Vec<String> = r
            .grades()
            .iter()
            .map(|g| format!("{:.1} %", g.2 * 100.0).replace('.', ","))
            .collect();
        if !grades.is_empty() {
            println!("  {}: rasante {}", r.name, grades.join(" · "));
        }
    }
    println!("  Proyecto: {}", project.display());
    println!("  Planta: {}", dir.join("calles-planta.svg").display());
    println!("  Axonometría: {}", dir.join("calles-3d.svg").display());
    println!("  IFC: {}", dir.join("calles.ifc").display());
    Ok(())
}

#[cfg(test)]
mod tests {
    /// The sample's streets meet in a crossing, three T's and an L, and
    /// the two that climb have a profile.
    #[test]
    fn streets_meet_every_way() {
        let m = super::build().unwrap();
        let pb = m.levels()[0].id;
        let js = archisim_geom::crossroads::junctions(&m, pb);
        let mut arms: Vec<usize> = js.iter().map(|j| j.arms.len()).collect();
        arms.sort_unstable();
        assert_eq!(arms, vec![2, 3, 3, 3, 4], "{arms:?}");
        assert_eq!(m.roads().filter(|r| r.sloped()).count(), 2);
        assert_eq!(m.roads().filter(|r| !r.changes.is_empty()).count(), 2);
    }
}
