//! `archisim area-grande`: a large area of the surroundings (N8,
//! `docs/TERRITORIO.md` §2.11) — 2 × 2 km of a made-up town on a grid of
//! avenues every 400 m and streets every 200 m, which run on outside the
//! area and are cut at its edge with their whole section (its connections
//! with the outside); the blocks of the middle cut into plots and filled
//! with blocks of flats and shops. It writes the project, its IFC, the
//! map of the area (SVG and PNG), the plan, and how long each takes.

use std::path::Path;
use std::time::Instant;

use archisim_core::{BlockZone, MaterialId, Model, NewRoad, Point2, RoadSection};
use archisim_geom::blocks;

type Res<T = ()> = Result<T, Box<dyn std::error::Error>>;

/// Half the side of the area, metres.
pub const HALF: f64 = 1000.0;
/// Between streets, and between avenues.
const STEP: f64 = 200.0;
/// How far the streets would run outside the area before they are cut.
const BEYOND: f64 = 150.0;

fn material(m: &Model, category: &str) -> Option<MaterialId> {
    m.materials().find(|x| x.category == category).map(|x| x.id)
}

pub fn build() -> Res<Model> {
    let mut m = Model::new("Área grande de ArchiSim");
    let pb = m.add_level("Planta baja", 0.0);
    archisim_library::add_library(&mut m)?;
    let asphalt = material(&m, "asphalt")
        .or_else(|| material(&m, "concrete"))
        .ok_or("sin asfalto")?;
    let stone = material(&m, "stone").unwrap_or(asphalt);
    let paving = material(&m, "ceramic").unwrap_or(stone);
    let area = archisim_core::square_area(Point2::new(0.0, 0.0), HALF);
    m.set_surroundings(Some(area.clone()))?;
    let avenue = RoadSection::classic(14.0, 2.2, 0.15, 5.0);
    let street = RoadSection::default();
    let count = (2.0 * HALF / STEP).round() as i32;
    for k in 1..count {
        let t = -HALF + k as f64 * STEP;
        let wide = k % 2 == 0;
        let section = if wide { &avenue } else { &street };
        for (name, a, b) in [
            (
                if wide {
                    format!("Avenida {}", k / 2)
                } else {
                    format!("Calle {k} Este")
                },
                Point2::new(-HALF - BEYOND, t),
                Point2::new(HALF + BEYOND, t),
            ),
            (
                if wide {
                    format!("Avenida {} Norte", k / 2)
                } else {
                    format!("Calle {k} Norte")
                },
                Point2::new(t, -HALF - BEYOND),
                Point2::new(t, HALF + BEYOND),
            ),
        ] {
            // Cut at the edge of the area, with its whole section.
            for axis in archisim_core::clip_line_to_area(&[a, b], &area) {
                m.add_road(NewRoad {
                    name: name.clone(),
                    level: pb,
                    axis,
                    section: section.clone(),
                    top_offset: 0.0,
                    profile: Vec::new(),
                    changes: Vec::new(),
                    pavement: asphalt,
                    kerb_material: stone,
                    sidewalk_material: paving,
                })?;
            }
        }
    }
    // The blocks of the middle: flats round the centre, shops on the four
    // in the middle.
    let found = blocks::city_blocks(&m, pb);
    for (k, block) in found.iter().enumerate() {
        let c = archisim_core::centroid(block);
        if c.x.abs() > 400.0 || c.y.abs() > 400.0 {
            continue;
        }
        let zone = if c.x.abs() < STEP && c.y.abs() < STEP {
            BlockZone::Shops
        } else {
            BlockZone::Flats
        };
        let frontage = if zone == BlockZone::Flats {
            Some(30.0)
        } else {
            None
        };
        blocks::add_block_plots(&mut m, pb, block, k + 1, zone, frontage, None)?;
    }
    blocks::fill_plots(&mut m, pb, None, None)?;
    Ok(m)
}

pub fn write(dir: &Path) -> Res {
    let t = Instant::now();
    let m = build()?;
    let built = t.elapsed();
    std::fs::create_dir_all(dir)?;
    let pb = m.levels()[0].id;
    let project = dir.join(format!("area-grande.{}", archisim_file::EXTENSION));
    let t = Instant::now();
    archisim_file::save(&m, &project)?;
    let saved = t.elapsed();
    let t = Instant::now();
    std::fs::write(
        dir.join("area-grande.ifc"),
        crate::ifc_text(&m, "area-grande", None),
    )?;
    let ifc = t.elapsed();
    let t = Instant::now();
    let map = archisim_plan::map::render_map(&m, pb, 1600.0).ok_or("sin mapa")?;
    let mapped = t.elapsed();
    std::fs::write(dir.join("area-grande-mapa.svg"), &map)?;
    std::fs::write(
        dir.join("area-grande-mapa.png"),
        archisim_export::png::png(&map, 96.0)?,
    )?;
    let t = Instant::now();
    let options = archisim_plan::PlanOptions {
        scale: 5000.0,
        ..Default::default()
    };
    std::fs::write(
        dir.join("area-grande-planta.svg"),
        archisim_plan::render_plan(&m, pb, &options),
    )?;
    let planned = t.elapsed();
    let ends = m.outside_ends(pb);
    println!(
        "Área grande: {} × {} km, {} calles, {} conexiones con fuera, {} manzanas, {} parcelas y {} edificios.",
        2.0 * HALF / 1000.0,
        2.0 * HALF / 1000.0,
        m.roads().count(),
        ends.len(),
        blocks::city_blocks(&m, pb).len(),
        m.plots().count(),
        m.masses().filter(|x| x.dress.is_some()).count()
    );
    let s = |d: std::time::Duration| format!("{:.2} s", d.as_secs_f64()).replace('.', ",");
    println!("  Montarlo: {}", s(built));
    println!("  Guardarlo: {}", s(saved));
    println!("  IFC: {}", s(ifc));
    println!("  Mapa: {}", s(mapped));
    println!("  Planta: {}", s(planned));
    println!("  Proyecto: {}", project.display());
    println!("  Mapa: {}", dir.join("area-grande-mapa.png").display());
    println!("  Planta: {}", dir.join("area-grande-planta.svg").display());
    println!("  IFC: {}", dir.join("area-grande.ifc").display());
    Ok(())
}

/// `archisim mapa <proyecto> [carpeta]`: the map of a project's first level
/// with something on it, as SVG and PNG.
pub fn map_of(file: &Path, dir: &Path) -> Res {
    let m = archisim_file::open(file)?;
    let level = m
        .levels()
        .iter()
        .map(|l| l.id)
        .find(|l| archisim_plan::map::map_frame(&m, *l).is_some())
        .ok_or("el proyecto no tiene nada que poner en un mapa")?;
    let svg = archisim_plan::map::render_map(&m, level, 1600.0).ok_or("sin mapa")?;
    std::fs::create_dir_all(dir)?;
    let stem = file
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("proyecto");
    let out = dir.join(format!("{stem}-mapa.svg"));
    std::fs::write(&out, &svg)?;
    let png = dir.join(format!("{stem}-mapa.png"));
    std::fs::write(&png, archisim_export::png::png(&svg, 96.0)?)?;
    println!("Mapa: {} y {}", out.display(), png.display());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The area keeps its connections with the outside (one at each end
    /// of every street) through the file and the IFC, and its map draws.
    #[test]
    fn the_large_area_and_its_connections() {
        let m = build().unwrap();
        let pb = m.levels()[0].id;
        let ends = m.outside_ends(pb).len();
        let lines = (2.0 * HALF / STEP).round() as usize - 1;
        assert_eq!(ends, lines * 4, "both ends of every street");
        let back = archisim_file::from_str(&archisim_file::to_string(&m)).unwrap();
        assert_eq!(back.surroundings(), m.surroundings());
        let ifc = crate::ifc_text(&m, "area-grande", None);
        let imported = archisim_ifc::import_ifc(&ifc).unwrap().model;
        assert_eq!(imported.surroundings(), m.surroundings());
        assert_eq!(imported.outside_ends(imported.levels()[0].id).len(), ends);
        assert!(m.masses().count() > 100);
        assert!(archisim_plan::map::render_map(&m, pb, 800.0).is_some());
    }
}
