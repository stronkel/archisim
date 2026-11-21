//! `archisim manzanas`: blocks, plots and filling them by zone (N7,
//! `docs/TERRITORIO.md` §2.10) — a town of three streets each way closes
//! four blocks; each is cut into plots along its façade line and filled
//! with buildings of the place by its zone (old town, blocks of flats,
//! detached houses, shops with a school on one corner); a locked plot
//! keeps its building when the others are cleared and filled again. In
//! plan, in 3D and in the IFC (whose `ArchiSim_Plot` sets keep the zone
//! and the lock, read back on import).

use std::path::Path;

use archisim_core::{
    BlockZone, MaterialId, Model, NewRoad, NewWall, Point2, ReferenceLine, RoadSection, WallTop,
};
use archisim_geom::blocks;

type Res<T = ()> = Result<T, Box<dyn std::error::Error>>;

fn material(m: &Model, category: &str) -> Option<MaterialId> {
    m.materials().find(|x| x.category == category).map(|x| x.id)
}

/// The zones of the four blocks, from the south-west.
const ZONES: [BlockZone; 4] = [
    BlockZone::OldTown,
    BlockZone::Flats,
    BlockZone::Houses,
    BlockZone::Shops,
];

pub fn build() -> Res<Model> {
    let mut m = Model::new("Manzanas de ArchiSim");
    let pb = m.add_level("Planta baja", 0.0);
    let lib = archisim_library::add_library(&mut m)?;
    let asphalt = material(&m, "asphalt")
        .or_else(|| material(&m, "concrete"))
        .ok_or("sin asfalto")?;
    let stone = material(&m, "stone").unwrap_or(asphalt);
    let paving = material(&m, "ceramic").unwrap_or(stone);
    let names = [
        ("Calle del Sur", "Calle de Poniente"),
        ("Calle Mayor", "Calle del Medio"),
        ("Calle del Norte", "Calle de Levante"),
    ];
    for (k, (ew, ns)) in names.iter().enumerate() {
        let t = k as f64 * 100.0;
        for (name, axis) in [
            (ew, vec![Point2::new(-15.0, t), Point2::new(215.0, t)]),
            (ns, vec![Point2::new(t, -15.0), Point2::new(t, 215.0)]),
        ] {
            m.add_road(NewRoad {
                name: name.to_string(),
                level: pb,
                axis,
                section: RoadSection::default(),
                top_offset: 0.0,
                profile: Vec::new(),
                changes: Vec::new(),
                pavement: asphalt,
                kerb_material: stone,
                sidewalk_material: paving,
            })?;
        }
    }
    let found = blocks::city_blocks(&m, pb);
    if found.len() != 4 {
        return Err(format!("{} manzanas en lugar de 4", found.len()).into());
    }
    let mut shops = Vec::new();
    for (k, (block, zone)) in found.iter().zip(ZONES).enumerate() {
        let ids = blocks::add_block_plots(&mut m, pb, block, k + 1, zone, None, None)?;
        if zone == BlockZone::Shops {
            shops = ids;
        }
    }
    // A school on the first plot of the shops' block.
    if let Some(first) = shops.first() {
        m.edit_plot(*first, |p| p.zone = Some(BlockZone::Civic))?;
    }
    blocks::fill_plots(&mut m, pb, None, None)?;
    // The second plot of the old town is locked: clearing and filling
    // again leaves its house as it is.
    let old: Vec<_> = m
        .plots_on_level(pb)
        .filter(|p| p.zone == Some(BlockZone::OldTown))
        .map(|p| p.id)
        .collect();
    if let Some(id) = old.get(1) {
        m.edit_plot(*id, |p| p.locked = true)?;
    }
    blocks::clear_plots(&mut m, pb, None)?;
    blocks::fill_plots(&mut m, pb, None, None)?;
    // A pavilion in the courtyard of the blocks of flats.
    let corners = [
        Point2::new(140.0, 45.0),
        Point2::new(160.0, 45.0),
        Point2::new(160.0, 55.0),
        Point2::new(140.0, 55.0),
    ];
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
            top: WallTop::Unconnected,
        })?;
    }
    // In the same courtyard, a small square with a tree pit and its paving
    // in herringbone, and a sunken garden (remates del territorio).
    let r = |a: (f64, f64), b: (f64, f64)| {
        vec![
            Point2::new(a.0, a.1),
            Point2::new(b.0, a.1),
            Point2::new(b.0, b.1),
            Point2::new(a.0, b.1),
        ]
    };
    let stone_paving = paving;
    let square = m.add_surface(archisim_core::NewSurface {
        name: "Placita".into(),
        level: pb,
        kind: archisim_core::SurfaceKind::Square,
        outline: r((132.0, 58.0), (168.0, 70.0)),
        material: stone_paving,
        edge_material: stone,
    })?;
    m.edit_surface(square, |x| {
        x.holes = vec![r((148.0, 62.0), (152.0, 66.0))];
        x.pattern = archisim_core::JointPattern::Herringbone;
        x.joints = 0.4;
    })?;
    let garden = m.add_surface(archisim_core::NewSurface {
        name: "Jardín hundido".into(),
        level: pb,
        kind: archisim_core::SurfaceKind::Garden,
        outline: r((132.0, 30.0), (138.0, 42.0)),
        material: stone_paving,
        edge_material: stone,
    })?;
    m.edit_surface(garden, |x| x.raise = -1.0)?;
    Ok(m)
}

pub fn write(dir: &Path) -> Res {
    let m = build()?;
    std::fs::create_dir_all(dir)?;
    let project = dir.join(format!("manzanas.{}", archisim_file::EXTENSION));
    archisim_file::save(&m, &project)?;
    std::fs::write(
        dir.join("manzanas.ifc"),
        crate::ifc_text(&m, "manzanas", None),
    )?;
    let pb = m.levels()[0].id;
    let options = archisim_plan::PlanOptions {
        scale: 1000.0,
        ..Default::default()
    };
    std::fs::write(
        dir.join("manzanas-planta.svg"),
        archisim_plan::render_plan(&m, pb, &options),
    )?;
    let axo = archisim_core::Axo::default();
    let drawing = archisim_plan::axo::axo_drawing(&m, &axo, Default::default());
    let crop = archisim_plan::axo::axo_extent(&m, &axo, 1000.0);
    std::fs::write(
        dir.join("manzanas-3d.svg"),
        archisim_plan::axo::render_axo(&drawing, crop, 1000.0),
    )?;
    println!(
        "Manzanas: {} calles, {} parcelas y {} edificios.",
        m.roads().count(),
        m.plots().count(),
        m.masses().filter(|x| x.dress.is_some()).count()
    );
    for zone in BlockZone::ALL {
        let plots: Vec<_> = m.plots().filter(|p| p.zone == Some(zone)).collect();
        if plots.is_empty() {
            continue;
        }
        let built = plots.iter().filter(|p| blocks::plot_built(&m, p)).count();
        println!(
            "  {}: {} parcelas, {} con edificio",
            zone.label(),
            plots.len(),
            built
        );
    }
    let locked = m.plots().filter(|p| p.locked).count();
    println!("  Bloqueadas: {locked}");
    println!("  Proyecto: {}", project.display());
    println!("  Planta: {}", dir.join("manzanas-planta.svg").display());
    println!("  Axonometría: {}", dir.join("manzanas-3d.svg").display());
    println!("  IFC: {}", dir.join("manzanas.ifc").display());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Four blocks zoned and filled; the zones and the lock come back from
    /// the file and from the IFC, and the locked plot kept its building.
    #[test]
    fn the_blocks_come_back() {
        let m = build().unwrap();
        let pb = m.levels()[0].id;
        for zone in BlockZone::ALL {
            assert!(
                m.plots().any(|p| p.zone == Some(zone)),
                "{zone:?} has no plots"
            );
        }
        let built = m.plots().filter(|p| blocks::plot_built(&m, p)).count();
        assert!(built * 10 >= m.plots().count() * 7, "{built} built");
        let locked: Vec<_> = m.plots().filter(|p| p.locked).collect();
        assert_eq!(locked.len(), 1);
        assert!(blocks::plot_built(&m, locked[0]));
        let text = archisim_file::to_string(&m);
        let back = archisim_file::from_str(&text).unwrap();
        let zones = |m: &Model| {
            let mut z: Vec<_> = m.plots().map(|p| (p.zone, p.locked)).collect();
            z.sort_by_key(|(z, l)| (z.map(|z| z.code()), *l));
            z
        };
        assert_eq!(zones(&back), zones(&m));
        let ifc = crate::ifc_text(&m, "manzanas", None);
        let imported = archisim_ifc::import_ifc(&ifc).unwrap().model;
        assert_eq!(zones(&imported), zones(&m));
        // Filling again puts nothing: they are all built or too small.
        let mut again = m.clone();
        assert_eq!(blocks::fill_plots(&mut again, pb, None, None).unwrap(), 0);
    }
}
