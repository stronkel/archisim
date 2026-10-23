//! `archisim cubiertas`: what a pitched roof carries (`docs/MAQUETA.md`
//! §5), on a small house with a gabled roof and a garage with a hipped
//! one: gabled, shed and flat dormers and roof windows, tiles of every
//! kind with their ridge tiles, gutters and downpipes, to see them in 3D,
//! in an axonometry, in the roof plan and in the IFC.

use std::path::Path;

use archisim_core::{
    DormerKind, Model, NewRoof, NewWall, Point2, ReferenceLine, RoofCovering, TileKind, WallTop,
};

type Res<T = ()> = Result<T, Box<dyn std::error::Error>>;

pub fn build() -> Res<Model> {
    let mut m = Model::new("Cubiertas de ArchiSim");
    let level = m.add_level("Planta baja", 0.0);
    let lib = archisim_library::add_library(&mut m)?;
    let walls = |m: &mut Model, c: &[Point2], h: f64| -> Res<Vec<archisim_core::WallId>> {
        let mut out = Vec::new();
        for i in 0..c.len() {
            out.push(m.add_wall(NewWall {
                wall_type: lib.facade,
                level,
                start: c[i],
                end: c[(i + 1) % c.len()],
                reference: ReferenceLine::ExteriorFace,
                height: h,
                base_offset: 0.0,
                bulge: 0.0,
                top: WallTop::Unconnected,
            })?);
        }
        Ok(out)
    };
    // The house: 10 × 8 m, gables east and west, 55 % north and south.
    let house = [
        Point2::new(0.0, 0.0),
        Point2::new(0.0, 8.0),
        Point2::new(10.0, 8.0),
        Point2::new(10.0, 0.0),
    ];
    let hw = walls(&mut m, &house, 3.0)?;
    let roof = m.add_roof(NewRoof {
        roof_type: lib.pitched_roof,
        level,
        outline: vec![house[0], house[3], house[2], house[1]],
        slopes: vec![Some(0.55), None, Some(0.55), None],
        base_offset: 3.0,
        overhang: 0.45,
    })?;
    for w in hw {
        m.set_wall_roof(w, Some(roof))?;
    }
    // Two gabled dormers and a roof window between them to the south, a
    // shed and a flat dormer to the north.
    m.add_dormer(roof, DormerKind::Gable, Point2::new(2.6, 0.5))?;
    m.add_dormer(roof, DormerKind::Gable, Point2::new(7.4, 0.5))?;
    m.add_dormer(roof, DormerKind::Skylight, Point2::new(5.0, 1.6))?;
    m.add_dormer(roof, DormerKind::Shed, Point2::new(3.2, 7.5))?;
    m.add_dormer(roof, DormerKind::Flat, Point2::new(7.3, 7.5))?;
    let gutter = m.default_gutter();
    m.set_roof_covering(
        roof,
        RoofCovering {
            tiles: Some(TileKind::Arabic),
            tile_material: None,
            gutter: gutter.clone(),
        },
    )?;
    // Garages with hipped roofs, one of each tile, each with a roof
    // window.
    for (k, tiles) in [
        TileKind::Mixed,
        TileKind::Flat,
        TileKind::Slate,
        TileKind::Metal,
    ]
    .into_iter()
    .enumerate()
    {
        let x = 13.0 + 7.0 * k as f64;
        let c = [
            Point2::new(x, 0.0),
            Point2::new(x, 6.0),
            Point2::new(x + 5.0, 6.0),
            Point2::new(x + 5.0, 0.0),
        ];
        let gw = walls(&mut m, &c, 2.6)?;
        let r = m.add_roof(NewRoof {
            roof_type: lib.pitched_roof,
            level,
            outline: vec![c[0], c[3], c[2], c[1]],
            slopes: vec![Some(0.6)],
            base_offset: 2.6,
            overhang: 0.35,
        })?;
        for w in gw {
            m.set_wall_roof(w, Some(r))?;
        }
        m.add_dormer(r, DormerKind::Skylight, Point2::new(x + 2.5, 0.9))?;
        m.set_roof_covering(
            r,
            RoofCovering {
                tiles: Some(tiles),
                tile_material: None,
                gutter: gutter.clone(),
            },
        )?;
    }
    Ok(m)
}

pub fn write(dir: &Path) -> Res {
    let m = build()?;
    std::fs::create_dir_all(dir)?;
    let project = dir.join(format!("cubiertas.{}", archisim_file::EXTENSION));
    archisim_file::save(&m, &project)?;
    std::fs::write(
        dir.join("cubiertas.ifc"),
        crate::ifc_text(&m, "cubiertas", None),
    )?;
    let axo = archisim_core::Axo::default();
    let drawing = archisim_plan::axo::axo_drawing(&m, &axo, Default::default());
    let crop = archisim_plan::axo::axo_extent(&m, &axo, 50.0);
    std::fs::write(
        dir.join("cubiertas-3d.svg"),
        archisim_plan::axo::render_axo(&drawing, crop, 50.0),
    )?;
    let options = archisim_plan::PlanOptions {
        scale: 100.0,
        ..Default::default()
    };
    let level = m.levels()[0].id;
    std::fs::write(
        dir.join("cubiertas-planta.svg"),
        archisim_plan::render_plan(&m, level, &options),
    )?;
    let dormers: usize = m.roofs().map(|r| r.dormers.len()).sum();
    println!(
        "Cubiertas: {} cubiertas, {dormers} buhardillas y ventanas de cubierta, tejas y canalones.",
        m.roofs().count()
    );
    println!("  Proyecto: {}", project.display());
    println!("  Axonometría: {}", dir.join("cubiertas-3d.svg").display());
    println!(
        "  Planta de cubiertas: {}",
        dir.join("cubiertas-planta.svg").display()
    );
    println!("  IFC: {}", dir.join("cubiertas.ifc").display());
    Ok(())
}

#[cfg(test)]
mod tests {
    #[test]
    fn every_dormer_is_built() {
        let m = super::build().unwrap();
        for r in m.roofs() {
            for d in &r.dormers {
                assert!(
                    r.dormer_shape(d).is_some(),
                    "{:?} on roof {}",
                    d.kind,
                    r.id.0
                );
            }
        }
        let text = archisim_file::to_string(&m);
        let back = archisim_file::from_str(&text).unwrap();
        assert_eq!(archisim_file::to_string(&back), text);
    }
}
