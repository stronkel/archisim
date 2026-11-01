//! `archisim calles`: a few streets on a slope that meet every way they
//! can (U1): an avenue with a tree-lined median crossed by a street that
//! climbs the hill (a crossing), a one-way street that ends on the avenue
//! (a T), a street between the two at the top (two T's) and two
//! pedestrian streets that meet at a corner (an L). The streets that
//! climb follow the terrain («rasante»); the crossing is a roundabout
//! and the two pedestrian streets meet in a square; the Calle Nueva's sidewalks
//! reach the façade line of its plots. To see the corners, the kerbs
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
    let nueva = road(
        &mut m,
        "Calle Nueva",
        preset("Calle de un sentido con aparcamiento"),
        vec![p(0.0, 60.0), p(60.0, 60.0)],
        ground(p(30.0, 60.0)),
    )?;
    // Plots along the Calle Nueva, with a façade line that steps back,
    // and its sidewalks taken out to them (U2).
    for (name, outline) in [
        (
            "Parcela 1",
            vec![p(9.0, 67.0), p(30.0, 67.0), p(30.0, 90.0), p(9.0, 90.0)],
        ),
        (
            "Parcela 2",
            vec![p(30.0, 68.5), p(51.0, 68.5), p(51.0, 90.0), p(30.0, 90.0)],
        ),
        (
            "Parcela 3",
            vec![p(9.0, 33.0), p(51.0, 33.0), p(51.0, 53.0), p(9.0, 53.0)],
        ),
    ] {
        m.add_plot(archisim_core::NewPlot {
            name: name.into(),
            level: pb,
            outline,
            setback: 3.0,
            reference: String::new(),
        })?;
    }
    let to_plots = m.sidewalks_to_plots(nueva).ok_or("sin parcelas")?;
    m.edit_road(nueva, |r| r.changes = to_plots)?;
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
    // A roundabout where the Calle Real crosses the avenue, and a square
    // with a fountain where the two pedestrian streets meet (U3).
    let grass = material(&m, "vegetation").unwrap_or(paving);
    let real_top = {
        let r = m.road(real).ok_or("sin calle")?;
        r.top_at(r.station_of(p(0.0, 0.0)))
    };
    m.add_roundabout(archisim_core::NewRoundabout {
        name: "Glorieta del Mar".into(),
        level: pb,
        centre: p(0.0, 0.0),
        radius: 16.0,
        island: 7.5,
        apron: 1.5,
        top_offset: real_top,
        plaza: false,
        island_finish: archisim_core::IslandFinish::Garden,
        pavement: asphalt,
        kerb_material: stone,
        island_material: grass,
    })?;
    m.add_roundabout(archisim_core::NewRoundabout {
        name: "Plaza del Pozo".into(),
        level: pb,
        centre: p(-35.0, 30.0),
        radius: 9.0,
        island: 2.5,
        apron: 0.0,
        top_offset: ground(p(-35.0, 30.0)),
        plaza: true,
        island_finish: archisim_core::IslandFinish::Paved,
        pavement: paving,
        kerb_material: stone,
        island_material: stone,
    })?;
    // Road markings at will (U4): crossings with their lowered kerbs on
    // the Calle Real and the Calle Nueva, a raised crossing, give-way lines
    // and triangles at the roundabout, a «STOP», arrows, «BUS», a hatched
    // island and bays for disabled people and for loading.
    m.edit_road(real, |r| {
        r.section.crossings = vec![archisim_core::Crossing {
            at: 115.0,
            width: 4.0,
        }];
    })?;
    m.edit_road(nueva, |r| {
        r.section.crossings = vec![archisim_core::Crossing {
            at: 15.0,
            width: 4.0,
        }];
    })?;
    use archisim_core::MarkingKind as K;
    let marks: Vec<(K, Vec<Point2>)> = vec![
        (K::RaisedCrossing, vec![p(32.0, 57.15), p(32.0, 62.85)]),
        (K::GiveWayLine, vec![p(-3.0, 24.5), p(0.0, 24.5)]),
        (K::GiveWay, vec![p(-1.5, 31.0), p(-1.5, 30.0)]),
        (K::GiveWayLine, vec![p(0.0, -24.5), p(3.0, -24.5)]),
        (K::GiveWay, vec![p(1.5, -31.0), p(1.5, -30.0)]),
        (K::StopLine, vec![p(53.0, 60.6), p(53.0, 57.2)]),
        (K::Stop, vec![p(46.0, 58.9), p(47.0, 58.9)]),
        (K::ArrowStraight, vec![p(1.5, -45.0), p(1.5, -44.0)]),
        (K::ArrowStraightRight, vec![p(1.5, -38.0), p(1.5, -37.0)]),
        (K::Bus, vec![p(-75.0, -5.5), p(-74.0, -5.5)]),
        (
            K::Chevrons,
            vec![p(0.0, -72.0), p(1.4, -62.0), p(-1.4, -62.0)],
        ),
        (
            K::DisabledBay,
            vec![p(40.0, 60.7), p(45.0, 60.7), p(45.0, 62.8), p(40.0, 62.8)],
        ),
        (K::LoadingBay, vec![p(14.0, 62.8), p(22.0, 62.8)]),
    ];
    for (kind, path) in marks {
        let material = m.marking_material(kind.colour());
        m.add_marking(archisim_core::NewMarking::usual(pb, kind, path, material))?;
    }
    // The networks under the street (U5): the sewer down the Calle Real
    // with its manholes and its connections, the water main, the street
    // lighting, low voltage and telecommunications along the Calle Nueva,
    // with hydrants, street lights and a gully.
    use archisim_core::{DeviceKind as D, ServiceSystem as S};
    let top_real = |m: &Model, y: f64| -> f64 {
        m.road(real)
            .map_or(0.0, |r| r.top_at(r.station_of(p(0.0, y))))
    };
    let (north, south) = (top_real(&m, 90.0), top_real(&m, -70.0));
    let sewer = m.add_run(archisim_core::NewRun {
        level: pb,
        system: S::Sewer,
        path: vec![p(3.0, 90.0), p(3.0, -70.0)],
        elevation: north - 2.2,
        width: 0.315,
        height: 0.315,
        slope: (north - south) / 160.0,
    })?;
    m.add_manholes(sewer)?;
    m.connect_plots(sewer)?;
    let top_nueva = m.road(nueva).map_or(0.0, |r| r.top_offset);
    for (system, y) in [
        (S::WaterMain, 64.0),
        (S::StreetLighting, 64.6),
        (S::PowerLow, 56.0),
        (S::Telecom, 55.4),
    ] {
        let (d, depth) = system.urban_defaults();
        let run = m.add_run(archisim_core::NewRun {
            level: pb,
            system,
            path: vec![p(9.0, y), p(51.0, y)],
            elevation: top_nueva - depth,
            width: d,
            height: d,
            slope: 0.0,
        })?;
        if system == S::WaterMain || system == S::PowerLow {
            m.connect_plots(run)?;
        }
    }
    let device = |m: &mut Model, kind: D, at: Point2| -> Res {
        let (w, d, h, z) = kind.defaults();
        let surface = m.surface_at(pb, at);
        m.add_device(archisim_core::NewDevice {
            level: pb,
            kind,
            name: kind.name().into(),
            position: at,
            rotation: 0.0,
            elevation: surface + z,
            width: w,
            depth: d,
            height: h,
        })?;
        Ok(())
    };
    device(&mut m, D::Hydrant, p(25.0, 65.6))?;
    for x in [14.0, 32.0, 50.0] {
        device(&mut m, D::StreetLight, p(x, 64.6))?;
    }
    device(&mut m, D::Gully, p(3.3, 70.0))?;
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
    // The sewer's longitudinal profile (U5).
    if let Some(sewer) = m
        .runs()
        .find(|r| r.system == archisim_core::ServiceSystem::Sewer && r.length() > 50.0)
    {
        if let Some(svg) = archisim_plan::profile::run_profile_svg(&m, sewer.id) {
            std::fs::write(dir.join("calles-perfil-saneamiento.svg"), svg)?;
        }
    }
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

/// Every street type (U3), one beside the other, 40 m long each, by
/// group; with a kiosk, so that the IFC has a building.
pub fn build_types() -> Res<Model> {
    let mut m = Model::new("Tipos de calle de ArchiSim");
    let pb = m.add_level("Planta baja", 0.0);
    let p1 = m.add_level("Planta primera", 3.0);
    let lib = archisim_library::add_library(&mut m)?;
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
    let grass = m.add_material(
        "Césped",
        "vegetation",
        [120, 160, 90],
        archisim_core::CutPattern::Solid,
    );
    let red = m.add_material(
        "Aglomerado rojo (carril bici)",
        "asphalt",
        [176, 88, 72],
        archisim_core::CutPattern::Solid,
    );
    let p = Point2::new;
    let mut y = 0.0;
    let mut x = 0.0;
    let mut last_group = None;
    for t in archisim_core::street_types() {
        // A column per group.
        if last_group.is_some_and(|g| g != t.group) {
            x += 60.0;
            y = 0.0;
        }
        last_group = Some(t.group);
        let w = t.section.width();
        y -= w / 2.0;
        let mut sec = t.section.clone();
        sec.markings = Some(paint);
        sec.planting = Some(grass);
        sec.bike = Some(red);
        m.add_road(NewRoad {
            name: t.name.into(),
            level: pb,
            axis: vec![p(x, y), p(x + 40.0, y)],
            section: sec,
            top_offset: 0.0,
            profile: Vec::new(),
            changes: Vec::new(),
            pavement: asphalt,
            kerb_material: stone,
            sidewalk_material: paving,
        })?;
        y -= w / 2.0 + 6.0;
    }
    // A kiosk.
    let corners = [p(-12.0, 0.0), p(-8.0, 0.0), p(-8.0, 3.0), p(-12.0, 3.0)];
    for i in 0..4 {
        m.add_wall(NewWall {
            wall_type: lib.bearing_wall,
            level: pb,
            start: corners[i],
            end: corners[(i + 1) % 4],
            reference: ReferenceLine::ExteriorFace,
            height: 3.0,
            base_offset: 0.0,
            bulge: 0.0,
            top: WallTop::Level {
                level: p1,
                offset: 0.0,
            },
        })?;
    }
    Ok(m)
}

/// `archisim tipos-calle`: [`build_types`] with its plan, axonometric and
/// IFC.
pub fn write_types(dir: &Path) -> Res {
    let m = build_types()?;
    std::fs::create_dir_all(dir)?;
    let project = dir.join(format!("tipos-calle.{}", archisim_file::EXTENSION));
    archisim_file::save(&m, &project)?;
    std::fs::write(
        dir.join("tipos-calle.ifc"),
        crate::ifc_text(&m, "tipos-calle", None),
    )?;
    let axo = archisim_core::Axo::default();
    let drawing = archisim_plan::axo::axo_drawing(&m, &axo, Default::default());
    let crop = archisim_plan::axo::axo_extent(&m, &axo, 500.0);
    std::fs::write(
        dir.join("tipos-calle-3d.svg"),
        archisim_plan::axo::render_axo(&drawing, crop, 500.0),
    )?;
    let pb = m.levels()[0].id;
    let options = archisim_plan::PlanOptions {
        scale: 500.0,
        ..Default::default()
    };
    std::fs::write(
        dir.join("tipos-calle-planta.svg"),
        archisim_plan::render_plan(&m, pb, &options),
    )?;
    println!("Tipos de calle: {}.", m.roads().count());
    println!("  Proyecto: {}", project.display());
    println!("  Planta: {}", dir.join("tipos-calle-planta.svg").display());
    println!(
        "  Axonometría: {}",
        dir.join("tipos-calle-3d.svg").display()
    );
    println!("  IFC: {}", dir.join("tipos-calle.ifc").display());
    Ok(())
}

#[cfg(test)]
mod tests {
    /// The sample's streets meet in a crossing, three T's and an L, the
    /// two that climb have a profile and the Calle Nueva's sidewalks reach
    /// its plots.
    #[test]
    fn streets_meet_every_way() {
        let m = super::build().unwrap();
        let pb = m.levels()[0].id;
        let js = archisim_geom::crossroads::junctions(&m, pb);
        let mut arms: Vec<usize> = js.iter().map(|j| j.arms.len()).collect();
        arms.sort_unstable();
        assert_eq!(arms, vec![2, 3, 3, 3, 4], "{arms:?}");
        // The crossing is a roundabout and the L a square.
        assert_eq!(js.iter().filter(|j| j.ring.is_some()).count(), 2);
        // Its markings, and the crossings' lowered kerbs.
        assert_eq!(m.markings().count(), 13);
        // Its networks: manholes, connections and the street lights.
        let count = |k: archisim_core::DeviceKind| m.devices().filter(|d| d.kind == k).count();
        assert!(count(archisim_core::DeviceKind::Manhole) >= 5);
        assert!(count(archisim_core::DeviceKind::Chamber) >= 4);
        assert_eq!(count(archisim_core::DeviceKind::StreetLight), 3);
        let real = m.roads().find(|r| r.name == "Calle Real").unwrap();
        assert!(real
            .pieces()
            .iter()
            .any(|p| p.lowered == archisim_core::Lowered::Flush));
        assert_eq!(m.roads().filter(|r| r.sloped()).count(), 2);
        assert_eq!(m.roads().filter(|r| !r.changes.is_empty()).count(), 3);
        let nueva = m.roads().find(|r| r.name == "Calle Nueva").unwrap();
        let (l, r) = nueva.extent_at(40.0);
        assert!((l - 8.5).abs() < 0.01 && (r - 7.0).abs() < 0.01, "{l} {r}");
    }

    /// Every street type is a road of the sample, and none meets another.
    #[test]
    fn every_street_type_side_by_side() {
        let m = super::build_types().unwrap();
        assert_eq!(m.roads().count(), archisim_core::street_types().len());
        let pb = m.levels()[0].id;
        assert!(archisim_geom::crossroads::junctions(&m, pb).is_empty());
    }
}
