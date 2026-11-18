//! `archisim territorio-grande`: the reference site of the territory
//! workshop (N0, `docs/TERRITORIO.md` §4): a square kilometre of a made-up
//! coastal town on gently rolling ground, with a grid of avenues and
//! streets that follow the ground, short side streets ending on them,
//! roundabouts, several hundred buildings of the world facing the streets
//! and the street life (trees, lights, benches, cars, people). It measures
//! what the territory costs: the junctions, the streets' solids, the
//! ground shaped by the streets, the whole 3D, the plan, and what moving
//! one street costs, which is what the application does at every touch.

use std::path::Path;
use std::time::Instant;

use archisim_core::{
    IslandFinish, LevelId, Model, NewRoundabout, NewTerrain, NewWorldBuilding, Point2, RoadSection,
    RoadStretch, SurveyPoint, Vec2, WorldPick, WorldTypology,
};
use archisim_plan::{render_plan, PlanOptions};

type Res<T = ()> = Result<T, Box<dyn std::error::Error>>;

/// Half the side of the site, metres.
pub const HALF: f64 = 500.0;
/// Between the north–south streets and between the east–west ones.
const STEP_X: f64 = 100.0;
const STEP_Y: f64 = 70.0;

/// The ground: a few gentle hills and a slope down to the sea at the
/// south.
fn ground(p: Point2) -> f64 {
    let hill = |cx: f64, cy: f64, h: f64, r: f64| {
        h * (-((p.x - cx).powi(2) + (p.y - cy).powi(2)) / (r * r)).exp()
    };
    0.008 * (p.y + HALF)
        + hill(-220.0, 180.0, 9.0, 160.0)
        + hill(260.0, -60.0, 6.0, 120.0)
        + hill(80.0, 330.0, 5.0, 90.0)
        + 0.6 * (p.x / 37.0).sin() * (p.y / 53.0).cos()
}

fn preset(name: &str) -> RoadSection {
    archisim_core::road_presets()
        .into_iter()
        .find(|(n, _)| *n == name)
        .map(|(_, s)| s)
        .unwrap_or_default()
}

/// The site, with its street life when `life`.
pub fn build(life: bool) -> Res<(Model, LevelId)> {
    let mut m = Model::new("Territorio de prueba 1 km²");
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
    let green = m
        .materials()
        .find(|x| x.category == "vegetation")
        .map_or(earth, |x| x.id);
    // The survey: a point every 25 m, and a margin round the site.
    let mut points = Vec::new();
    let n = ((HALF + 50.0) / 25.0) as i32;
    for i in -n..=n {
        for j in -n..=n {
            let at = Point2::new(i as f64 * 25.0, j as f64 * 25.0);
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
    let lay = |m: &mut Model, section: &RoadSection, a: Point2, b: Point2| {
        m.add_road_stretch(RoadStretch {
            level: pb,
            points: vec![a, b],
            continues: None,
            section: section.clone(),
            top_offset: 0.0,
            pavement: asphalt,
            kerb_material: stone,
            sidewalk_material: paving,
            follow_ground: Some(0.08),
        })
    };
    let avenue = preset("Avenida con mediana arbolada");
    let street = preset("Calle de doble sentido con aparcamiento");
    let lane = preset("Calle de un sentido sin aparcamiento");
    let xs: Vec<f64> = (0..)
        .map(|i| -HALF + i as f64 * STEP_X)
        .take_while(|x| *x <= HALF + 1e-6)
        .collect();
    let ys: Vec<f64> = (0..)
        .map(|j| -HALF + 30.0 + j as f64 * STEP_Y)
        .take_while(|y| *y <= HALF + 1e-6)
        .collect();
    let reach = HALF + 20.0;
    for (i, &x) in xs.iter().enumerate() {
        let s = if i % 4 == 0 { &avenue } else { &street };
        lay(&mut m, s, Point2::new(x, -reach), Point2::new(x, reach))?;
    }
    for (j, &y) in ys.iter().enumerate() {
        let s = if j % 4 == 2 { &avenue } else { &street };
        lay(&mut m, s, Point2::new(-reach, y), Point2::new(reach, y))?;
    }
    // A short street into every other block, ending on the street to its
    // south (a T).
    for (i, w) in xs.windows(2).enumerate() {
        for (j, h) in ys.windows(2).enumerate() {
            if (i + j) % 2 == 1 {
                continue;
            }
            let cx = (w[0] + w[1]) / 2.0;
            lay(
                &mut m,
                &lane,
                Point2::new(cx, h[0]),
                Point2::new(cx, h[0] + 0.55 * (h[1] - h[0])),
            )?;
        }
    }
    // The street's top under a point, as the roundabout tool takes it.
    let top_at = |m: &Model, c: Point2| {
        m.roads_on_level(pb)
            .map(|r| (r.station_of(c), r))
            .min_by(|a, b| {
                a.1.axis_at(a.0)
                    .0
                    .distance(c)
                    .total_cmp(&b.1.axis_at(b.0).0.distance(c))
            })
            .map_or(0.0, |(s, r)| r.top_at(s))
    };
    // Roundabouts on four crossings of the avenues.
    let avenue_x: Vec<f64> = xs.iter().step_by(4).copied().collect();
    let avenue_y: Vec<f64> = ys.iter().skip(2).step_by(4).copied().collect();
    for (k, (&x, &y)) in avenue_x
        .iter()
        .flat_map(|x| avenue_y.iter().map(move |y| (x, y)))
        .enumerate()
    {
        if k % 2 == 1 {
            continue;
        }
        m.add_roundabout(NewRoundabout {
            name: m.next_roundabout_name(),
            level: pb,
            centre: Point2::new(x, y),
            radius: 22.0,
            island: 12.0,
            apron: 1.5,
            top_offset: top_at(&m, Point2::new(x, y)),
            plaza: false,
            island_finish: IslandFinish::Garden,
            pavement: asphalt,
            kerb_material: stone,
            island_material: green,
        })?;
    }
    // Buildings of the world facing the east–west streets, two to each
    // side of a block, of the place's street houses, blocks and detached
    // houses.
    let kinds = [
        WorldTypology::Street,
        WorldTypology::Block,
        WorldTypology::House,
        WorldTypology::Street,
    ];
    let mut k = 0usize;
    for w in xs.windows(2) {
        for (j, &y) in ys.iter().enumerate() {
            for side in [-1.0, 1.0] {
                // No buildings outside the site.
                let back = y + side * 30.0;
                if back.abs() > HALF {
                    continue;
                }
                let half = if j % 4 == 2 { 17.0 } else { 12.0 };
                for cx in [w[0] + 26.0, w[1] - 26.0] {
                    let pick = WorldPick {
                        place: 0,
                        typology: kinds[k % kinds.len()],
                        programme: 0,
                    };
                    k += 1;
                    m.add_world_building(NewWorldBuilding {
                        level: pb,
                        front: Point2::new(cx, y + side * half),
                        facing: Vec2::new(0.0, -side),
                        width: 20.0,
                        depth: 14.0,
                        floors: None,
                        pick,
                    })?;
                }
            }
        }
    }
    if life {
        archisim_library::street_life::add_street_life(
            &mut m,
            pb,
            &archisim_library::street_life::LifeOptions::default(),
        )?;
    }
    Ok((m, pb))
}

/// Runs `f` and prints how long it took; gives back what it made and the
/// milliseconds.
fn timed<T>(what: &str, f: impl FnOnce() -> T) -> (T, f64) {
    let t = Instant::now();
    let out = f();
    let ms = t.elapsed().as_secs_f64() * 1000.0;
    println!("  {what:<56} {ms:>10.1} ms");
    (out, ms)
}

/// Builds the site, measures it and leaves `territorio.archisim` in
/// `dir`, and the times in `territorio-medidas.txt`.
pub fn measure(dir: &Path, life: bool) -> Res {
    println!("Territorio de prueba, 1 × 1 km:");
    let (built, _) = timed("Crear el sitio", || build(life));
    let (m, pb) = built?;
    let mut lines: Vec<(String, f64)> = Vec::new();
    let mut note = |what: &str, ms: f64| lines.push((what.to_string(), ms));
    println!(
        "  {} calles, {} glorietas, {} edificios, {} objetos",
        m.roads().count(),
        m.roundabouts().count(),
        m.masses().count(),
        m.objects().count()
    );
    let (js, ms) = timed("Cruces de la planta (la primera vez)", || {
        archisim_geom::crossroads::junctions(&m, pb)
    });
    note("Cruces, la primera vez", ms);
    println!("  {:<56} {:>10}", "  cruces", js.len());
    let (_, ms) = timed("Cruces de la planta (otra vez, sin cambios)", || {
        archisim_geom::crossroads::junctions(&m, pb)
    });
    note("Cruces, otra vez", ms);
    let (roads, ms) = timed("Sólidos de las calles", || {
        archisim_geom::solid::level_road_solids(&m, pb)
    });
    note("Sólidos de las calles", ms);
    let road_tris: usize = roads.iter().map(|s| s.triangles.len()).sum();
    println!("  {:<56} {:>10}", "  triángulos", road_tris);
    let terrain = m.terrains().next().map(|t| t.id).ok_or("sin terreno")?;
    let (ground, ms) = timed("Terreno modelado por las calles", || {
        archisim_geom::solid::terrain_solids(&m, terrain)
    });
    note("Terreno modelado por las calles", ms);
    let ground_tris: usize = ground.iter().map(|s| s.triangles.len()).sum();
    println!("  {:<56} {:>10}", "  triángulos", ground_tris);
    // Counted as they are made, not kept: millions of triangles.
    archisim_geom::meter::take();
    let mut by_kind: std::collections::BTreeMap<String, usize> = Default::default();
    let (_, ms) = timed("Sólidos 3D de todo el sitio", || {
        archisim_geom::solid::model_solids_each(
            &m,
            &Default::default(),
            &Default::default(),
            &mut |s| {
                let kind = format!("{:?}", s.owner);
                let kind = kind.split('(').next().unwrap_or("").to_string();
                *by_kind.entry(kind).or_default() += s.triangles.len();
            },
        )
    });
    note("Sólidos 3D de todo el sitio", ms);
    for (part, ms) in archisim_geom::meter::take() {
        println!("  {:<56} {:>10.1} ms", format!("    {part}"), ms);
    }
    let all: usize = by_kind.values().sum();
    println!("  {:<56} {:>10}", "  triángulos", all);
    for (k, n) in &by_kind {
        println!("  {:<56} {:>10}", format!("    {k}"), n);
    }
    // Moving one street a metre, as dragging it does: what has to be done
    // again before the screen can show it.
    let mut moved = m.clone();
    let id = moved
        .roads()
        .nth(moved.roads().count() / 2)
        .map(|r| r.id)
        .ok_or("sin calles")?;
    let axis: Vec<Point2> = moved
        .road(id)
        .map(|r| r.axis.iter().map(|p| *p + Vec2::new(1.0, 0.0)).collect())
        .unwrap_or_default();
    moved.edit_road(id, |r| r.axis = axis)?;
    let (_, ms) = timed("Al mover una calle: cruces", || {
        archisim_geom::crossroads::junctions(&moved, pb)
    });
    note("Mover una calle: cruces", ms);
    let (_, ms) = timed("Al mover una calle: sólidos de las calles", || {
        archisim_geom::solid::level_road_solids(&moved, pb)
    });
    note("Mover una calle: sólidos de las calles", ms);
    let (_, ms) = timed("Al mover una calle: terreno", || {
        archisim_geom::solid::terrain_solids(&moved, terrain)
    });
    note("Mover una calle: terreno", ms);
    // The plan as the screen draws it: a 200 × 120 m view at 1:500 and
    // the whole site at 1:2000.
    let plan = |viewport: (Point2, Point2), scale: f64| {
        let w = 1600.0;
        let px_per_mm = w / (viewport.1.x - viewport.0.x) * scale / 1000.0;
        render_plan(
            &m,
            pb,
            &PlanOptions {
                viewport: Some(viewport),
                scale,
                px_per_mm: Some(px_per_mm),
                ..PlanOptions::default()
            },
        )
    };
    archisim_geom::meter::take();
    let (_, ms) = timed("Planta en pantalla, 200 × 120 m a 1:500", || {
        plan(
            (Point2::new(-100.0, -60.0), Point2::new(100.0, 60.0)),
            500.0,
        )
    });
    note("Planta, zona de 200 × 120 m", ms);
    for (part, ms) in archisim_geom::meter::take().into_iter().take(6) {
        println!("  {:<56} {:>10.1} ms", format!("    {part}"), ms);
    }
    archisim_geom::meter::take();
    let (_, ms) = timed("Planta en pantalla, todo el sitio a 1:2000", || {
        plan((Point2::new(-HALF, -HALF), Point2::new(HALF, HALF)), 2000.0)
    });
    note("Planta, todo el sitio", ms);
    for (part, ms) in archisim_geom::meter::take().into_iter().take(6) {
        println!("  {:<56} {:>10.1} ms", format!("    {part}"), ms);
    }
    // What N1–N9 added: nodes with lights and raised crossings, squares
    // and car parks, the blocks cut into plots, the map and the traffic.
    let mut n = m.clone();
    let (_, ms) = timed("N1: cambiar 60 nudos (semáforos y elevados)", || {
        let keys = archisim_geom::crossroads::all_node_keys(&n);
        for (i, key) in keys.into_iter().take(60).enumerate() {
            let mut s = archisim_core::NodeSettings::new(key);
            s.kind = Some(if i % 3 == 0 {
                archisim_core::NodeKind::Raised
            } else {
                archisim_core::NodeKind::Crossing
            });
            let _ = n.set_node(s);
        }
    });
    note("N1: cambiar 60 nudos", ms);
    let (_, ms) = timed("N1: cruces con los nudos cambiados", || {
        archisim_geom::crossroads::junctions(&n, pb)
    });
    note("N1: cruces con nudos", ms);
    let (_, ms) = timed("N1: sólidos de las calles con los nudos", || {
        archisim_geom::solid::level_road_solids(&n, pb)
    });
    note("N1: sólidos de las calles con nudos", ms);
    let paving_mat = n.materials().next().map(|x| x.id).ok_or("sin materiales")?;
    let (made, ms) = timed("N5: 40 plazas y aparcamientos", || {
        let mut ids = Vec::new();
        for i in 0..40 {
            let c = Point2::new(
                -400.0 + (i % 8) as f64 * 100.0,
                -380.0 + (i / 8) as f64 * 160.0,
            );
            let kind = if i % 2 == 0 {
                archisim_core::SurfaceKind::Square
            } else {
                archisim_core::SurfaceKind::Parking
            };
            let outline = vec![
                c + Vec2::new(-12.0, -8.0),
                c + Vec2::new(12.0, -8.0),
                c + Vec2::new(12.0, 8.0),
                c + Vec2::new(-12.0, 8.0),
            ];
            if let Ok(id) = n.add_surface(archisim_core::NewSurface {
                name: format!("S{i}"),
                level: pb,
                kind,
                outline,
                material: paving_mat,
                edge_material: paving_mat,
            }) {
                ids.push(id);
            }
        }
        ids
    });
    note("N5: añadir 40 superficies", ms);
    let (_, ms) = timed("N5: sólidos de las superficies", || {
        made.iter()
            .map(|id| archisim_geom::urban_surfaces::surface_solids(&n, *id).len())
            .sum::<usize>()
    });
    note("N5: sólidos de las superficies", ms);
    let (blocks, ms) = timed("N7: buscar las manzanas", || {
        archisim_geom::blocks::city_blocks(&n, pb)
    });
    note("N7: buscar las manzanas", ms);
    println!("  {:<56} {:>10}", "  manzanas", blocks.len());
    let (lots, ms) = timed("N7: parcelar todas las manzanas", || {
        blocks
            .iter()
            .map(|b| archisim_geom::blocks::block_lots(b, 12.0, 20.0).len())
            .sum::<usize>()
    });
    note("N7: parcelar todas", ms);
    println!("  {:<56} {:>10}", "  parcelas", lots);
    let (map, ms) = timed("N8: mapa de todo el sitio, 1600 px", || {
        archisim_plan::map::render_map(&n, pb, 1600.0)
    });
    note("N8: mapa del sitio", ms);
    println!(
        "  {:<56} {:>10}",
        "  tamaño del mapa (kB)",
        map.map_or(0, |s| s.len() / 1024)
    );
    let (cars, people) = archisim_geom::traffic::usual_counts(&n, pb);
    let (mut traffic, ms) = timed("N9: preparar el tráfico", || {
        archisim_geom::traffic::Traffic::new(&n, pb, cars, people, 7)
    });
    note("N9: preparar el tráfico", ms);
    println!(
        "  {:<56} {:>10}",
        "  coches y personas",
        format!("{cars} y {people}")
    );
    let (_, ms) = timed("N9: 250 pasos de 1/25 s (10 s de tráfico)", || {
        for _ in 0..250 {
            traffic.step(&n, 0.04);
        }
    });
    note("N9: un paso del tráfico", ms / 250.0);
    let mat = paving_mat;
    let (_, ms) = timed("N9: cuerpos y sólidos de un fotograma", || {
        let bodies = traffic.bodies(&n);
        archisim_geom::traffic::traffic_solids(&bodies, mat).len()
    });
    note("N9: un fotograma del tráfico", ms);
    std::fs::create_dir_all(dir)?;
    let path = dir.join(format!("territorio.{}", archisim_file::EXTENSION));
    let (saved, ms) = timed("Guardar el proyecto (.archisim)", || {
        archisim_file::save(&m, &path)
    });
    saved?;
    note("Guardar", ms);
    let (opened, ms) = timed("Abrirlo", || archisim_file::open(&path));
    opened?;
    note("Abrir", ms);
    let mut text = String::from("Territorio de prueba, 1 × 1 km (archisim territorio-grande)\n");
    text += &format!(
        "{} calles, {} glorietas, {} cruces, {} edificios, {} objetos, {} triángulos\n\n",
        m.roads().count(),
        m.roundabouts().count(),
        js.len(),
        m.masses().count(),
        m.objects().count(),
        all
    );
    for (what, ms) in &lines {
        text += &format!("{what:<48} {ms:>10.1} ms\n");
    }
    std::fs::write(dir.join("territorio-medidas.txt"), text)?;
    println!("{} y territorio-medidas.txt", path.display());
    Ok(())
}
