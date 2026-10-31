//! `archisim armado`: reinforced concrete (E11), a small frame — four
//! columns on pad footings, beams between their heads, a strip footing and
//! a slab over them — all with their reinforcement, to see the bars in 3D
//! (with the section box), the columns' bars in plan at 1:20, the steel in
//! the measurements, the checks of the Código Estructural and the bars in
//! the IFC.

use std::path::Path;

use archisim_core::{
    Counted, FootingKind, Model, NewBeam, NewColumn, NewFooting, NewSlab, Point2, RebarKind,
    Reinforcement, Spaced, WallTop,
};

type Res<T = ()> = Result<T, Box<dyn std::error::Error>>;

pub fn build() -> Res<Model> {
    let mut m = Model::new("Armado de ArchiSim");
    let ground = m.add_level("Planta baja", 0.0);
    let first = m.add_level("Planta primera", 3.0);
    let lib = archisim_library::add_library(&mut m)?;
    let concrete = m
        .materials()
        .find(|x| x.name == "Hormigón armado")
        .map(|x| x.id)
        .ok_or("sin hormigón")?;
    let (sx, sy) = (5.0, 4.0);
    let corners = [
        Point2::new(0.0, 0.0),
        Point2::new(sx, 0.0),
        Point2::new(sx, sy),
        Point2::new(0.0, sy),
    ];
    let mut columns = Vec::new();
    for (i, p) in corners.iter().enumerate() {
        let c = m.add_column(NewColumn {
            member_type: lib.column,
            level: ground,
            position: *p,
            rotation: 0.0,
            base_offset: 0.0,
            top: WallTop::Unconnected,
            height: 3.0,
        })?;
        columns.push(c);
        let f = m.add_footing(NewFooting {
            level: ground,
            kind: FootingKind::Pad {
                position: *p,
                length: 1.4,
                rotation: 0.0,
                column: Some(c),
            },
            material: concrete,
            width: 1.4,
            depth: 0.6,
            top_offset: -0.05,
        })?;
        // A column with eight bars, the others with four.
        let mut rc = Reinforcement::default_for(RebarKind::Column);
        if i == 0 {
            if let Reinforcement::Column { bars, .. } = &mut rc {
                *bars = Counted::new(8, 0.016);
            }
        }
        m.set_reinforcement(c.0, Some(rc))?;
        m.set_reinforcement(f.0, Some(Reinforcement::default_for(RebarKind::Footing)))?;
    }
    // Beams round the heads of the columns.
    for i in 0..4 {
        let b = m.add_beam(NewBeam {
            member_type: lib.beam,
            level: first,
            start: corners[i],
            end: corners[(i + 1) % 4],
            top_offset: 0.0,
        })?;
        m.set_reinforcement(b.0, Some(Reinforcement::default_for(RebarKind::Beam)))?;
    }
    // A wall on the strip footing, between the back columns.
    m.add_wall(archisim_core::NewWall {
        wall_type: lib.bearing_wall,
        level: ground,
        start: Point2::new(sx, sy + 1.5),
        end: Point2::new(0.0, sy + 1.5),
        reference: archisim_core::ReferenceLine::Center,
        height: 3.0,
        base_offset: 0.0,
        bulge: 0.0,
        top: WallTop::Unconnected,
    })?;
    // A strip footing along the back, with a mesh on top too.
    let strip = m.add_footing(NewFooting {
        level: ground,
        kind: FootingKind::Strip {
            start: Point2::new(-1.5, sy + 1.5),
            end: Point2::new(sx + 1.5, sy + 1.5),
            wall: None,
        },
        material: concrete,
        width: 0.8,
        depth: 0.5,
        top_offset: -0.05,
    })?;
    m.set_reinforcement(
        strip.0,
        Some(Reinforcement::Footing {
            cover: 0.05,
            bottom: [Spaced::new(0.012, 0.15), Spaced::new(0.012, 0.25)],
            top: Some([Spaced::new(0.010, 0.25); 2]),
        }),
    )?;
    // The slab over the beams, with a hole.
    let slab = m.add_slab(NewSlab {
        slab_type: lib.floor_slab,
        level: first,
        outline: corners.to_vec(),
        top_offset: 0.0,
    })?;
    m.add_slab_hole(
        slab,
        vec![
            Point2::new(3.4, 2.4),
            Point2::new(4.4, 2.4),
            Point2::new(4.4, 3.4),
            Point2::new(3.4, 3.4),
        ],
    )?;
    m.set_reinforcement(slab.0, Some(Reinforcement::default_for(RebarKind::Slab)))?;
    // A section along the front row, looking north.
    m.add_section(archisim_core::NewSection {
        name: "A".into(),
        kind: archisim_core::SectionKind::Section,
        start: Point2::new(-1.5, 0.0),
        end: Point2::new(sx + 1.5, 0.0),
        depth: 2.0,
    })?;
    Ok(m)
}

pub fn write(dir: &Path) -> Res {
    let m = build()?;
    std::fs::create_dir_all(dir)?;
    let project = dir.join(format!("armado.{}", archisim_file::EXTENSION));
    archisim_file::save(&m, &project)?;
    std::fs::write(dir.join("armado.ifc"), crate::ifc_text(&m, "armado", None))?;
    let options = archisim_plan::PlanOptions {
        scale: 20.0,
        ..Default::default()
    };
    // A section along the front row of columns, at 1:20: the bars cut and
    // seen (E17).
    if let Some(sec) = m.sections().find(|s| s.name == "A") {
        let d = archisim_plan::section::section_drawing(&m, sec);
        std::fs::write(
            dir.join("armado-seccion.svg"),
            archisim_plan::section::render_section(&m, &d, &options),
        )?;
    }
    let level = m.levels()[0].id;
    std::fs::write(
        dir.join("armado-planta.svg"),
        archisim_plan::render_plan(&m, level, &options),
    )?;
    // Both plans at 1:50: the bars of the footings and the slab and the
    // labels of beams and columns (G1).
    let fifty = archisim_plan::PlanOptions {
        scale: 50.0,
        ..Default::default()
    };
    for (i, l) in m.levels().iter().enumerate() {
        std::fs::write(
            dir.join(format!("armado-planta-{i}-50.svg")),
            archisim_plan::render_plan(&m, l.id, &fifty),
        )?;
    }
    let mut kg = 0.0;
    let mut bars = 0;
    for (id, _) in m.reinforcements() {
        for g in archisim_geom::rebar::element_bars(&m, id) {
            kg += g.weight();
            bars += g.bars.len();
        }
    }
    let mut report = archisim_cte::Report::default();
    archisim_cte::structure::check(&m, &mut report);
    let fails = report
        .items
        .iter()
        .filter(|i| i.status == archisim_cte::Status::Fails)
        .count();
    println!(
        "Armado: {} elementos armados, {bars} barras, {kg:.0} kg de acero; {} comprobaciones, {fails} no cumplen.",
        m.reinforcements().count(),
        report.items.len()
    );
    println!("  Proyecto: {}", project.display());
    println!(
        "  Planta a 1:20: {}",
        dir.join("armado-planta.svg").display()
    );
    println!(
        "  Sección A a 1:20: {}",
        dir.join("armado-seccion.svg").display()
    );
    println!("  IFC: {}", dir.join("armado.ifc").display());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_element_has_its_bars() {
        let m = build().unwrap();
        assert_eq!(m.reinforcements().count(), 14);
        for (id, _) in m.reinforcements() {
            let groups = archisim_geom::rebar::element_bars(&m, id);
            assert!(groups.iter().all(|g| !g.bars.is_empty()), "{id}");
            assert!(!archisim_geom::rebar::rebar_solids(&m, id).is_empty());
        }
        // The slab's bars stop at its hole.
        let slab = m.slabs().next().unwrap().id.0;
        for g in archisim_geom::rebar::element_bars(&m, slab) {
            for b in &g.bars {
                let mid = Point2::new((b[0][0] + b[1][0]) / 2.0, (b[0][1] + b[1][1]) / 2.0);
                let inside_hole = mid.x > 3.45 && mid.x < 4.35 && mid.y > 2.45 && mid.y < 3.35;
                assert!(!inside_hole);
            }
        }
        // The defaults meet the code.
        let mut report = archisim_cte::Report::default();
        archisim_cte::structure::check(&m, &mut report);
        let fails: Vec<_> = report
            .items
            .iter()
            .filter(|i| i.status == archisim_cte::Status::Fails)
            .map(|i| (archisim_cte::subject_name(&m, i.subject), i.text.clone()))
            .collect();
        assert!(fails.is_empty(), "{fails:#?}");
    }
}
