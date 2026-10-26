//! The nursery ("vivero"): every species ArchiSim grows, planted in rows
//! by groups, with an avenue and a planted bed, to see them in plan (in
//! presentation and technical), in 3D and in the IFC.

use std::path::Path;

use archisim_core::{Arrangement, Model, NewObject, NewPlanting, NewSection, Point2, SectionKind};

/// Room each plant is given in its row: its width and this.
const GAP: f64 = 1.5;

pub fn build() -> Result<Model, Box<dyn std::error::Error>> {
    let mut m = archisim_library::new_project("Vivero de ArchiSim");
    let level = m.levels()[0].id;
    let mut y = 0.0;
    let mut group = "";
    let mut x = 0.0;
    let mut row_depth: f64 = 0.0;
    // Each row seen from the south, as an elevation of its own.
    let elevation = |m: &mut Model, group: &str, y: f64, depth: f64, x_end: f64| {
        m.add_section(NewSection {
            name: group.to_string(),
            kind: SectionKind::Elevation,
            start: Point2::new(-2.0, y - depth - 1.0),
            end: Point2::new(x_end + 2.0, y - depth - 1.0),
            depth: depth + 2.0,
        })
        .map(|_| ())
    };
    for e in archisim_library::flora::FLORA_CATALOGUE {
        if e.group != group {
            if !group.is_empty() {
                elevation(&mut m, group, y, row_depth, x)?;
                y -= row_depth + GAP * 2.0;
            }
            group = e.group;
            x = 0.0;
            row_depth = 0.0;
        }
        let t = archisim_library::add_object_entry(&mut m, e.name)?;
        let w = m
            .object_type_box(t)
            .map(|(lo, hi)| hi[0] - lo[0])
            .unwrap_or(2.0);
        x += w / 2.0;
        m.add_object(NewObject {
            level,
            object_type: t,
            position: Point2::new(x, y - w / 2.0),
            rotation: 0.0,
            base_offset: 0.0,
        })?;
        x += w / 2.0 + GAP;
        row_depth = row_depth.max(w);
    }
    elevation(&mut m, group, y, row_depth, x)?;
    y -= row_depth + GAP * 3.0 + 8.0;
    // An avenue of plane trees, a little irregular, and a bed of
    // rosemary and lavender.
    let plane = archisim_library::add_object_entry(&mut m, "Plátano de sombra")?;
    m.add_planting(NewPlanting {
        name: "Paseo de plátanos".into(),
        level,
        path: vec![
            Point2::new(0.0, y),
            Point2::new(40.0, y - 6.0),
            Point2::new(80.0, y),
        ],
        closed: false,
        smooth: true,
        arrangement: Arrangement::Line,
        count: 9,
        species: vec![plane],
        size_variation: 0.4,
        shape_variation: 0.6,
        spread: 0.15,
        seed: 7,
        base_offset: 0.0,
    })?;
    let rosemary = archisim_library::add_object_entry(&mut m, "Romero")?;
    let lavender = archisim_library::add_object_entry(&mut m, "Lavanda")?;
    let y2 = y - 16.0;
    m.add_planting(NewPlanting {
        name: "Arriate de aromáticas".into(),
        level,
        path: vec![
            Point2::new(0.0, y2),
            Point2::new(14.0, y2 + 1.0),
            Point2::new(16.0, y2 - 6.0),
            Point2::new(2.0, y2 - 7.0),
        ],
        closed: true,
        smooth: true,
        arrangement: Arrangement::Area,
        count: 40,
        species: vec![rosemary, lavender],
        size_variation: 0.5,
        shape_variation: 0.7,
        spread: 0.4,
        seed: 3,
        base_offset: 0.0,
    })?;
    Ok(m)
}

/// Writes the nursery as a project, an IFC and two 1:200 plans
/// (presentation and technical).
pub fn write(dir: &Path) -> Result<(), Box<dyn std::error::Error>> {
    std::fs::create_dir_all(dir)?;
    let m = build()?;
    let project = dir.join("vivero.archisim");
    archisim_file::save(&m, &project)?;
    println!("  Proyecto: {}", project.display());
    let ifc = dir.join("vivero.ifc");
    std::fs::write(&ifc, crate::ifc_text(&m, "vivero", None))?;
    println!("  IFC 4.3: {}", ifc.display());
    let level = m.levels()[0].id;
    let mut options = archisim_plan::PlanOptions {
        scale: 200.0,
        ..Default::default()
    };
    let svg = dir.join("vivero-presentacion.svg");
    std::fs::write(&svg, archisim_plan::render_plan(&m, level, &options))?;
    println!("  Planta 1:200 de presentación: {}", svg.display());
    // The same, every plant drawn technical.
    let mut t = m.clone();
    let ids: Vec<_> = t.object_types().map(|x| x.id).collect();
    for id in ids {
        let Some(mut v) = t.object_type(id).map(|x| x.values.clone()) else {
            continue;
        };
        if t.type_grows(id) {
            v.insert(
                "plan_drawing".into(),
                archisim_core::Value::Text("técnico".into()),
            );
            t.set_object_type_values(id, v)?;
        }
    }
    options.scale = 200.0;
    let svg = dir.join("vivero-tecnico.svg");
    std::fs::write(&svg, archisim_plan::render_plan(&t, level, &options))?;
    println!("  Planta 1:200 técnica: {}", svg.display());
    // Each row seen from the south: technical, and in a presentation
    // style (colour).
    let presentation = m.presentation_styles().next().map(|p| p.id);
    for sec in m.sections() {
        let drawing = archisim_plan::section::section_drawing(&m, sec);
        let slug: String = sec
            .name
            .to_lowercase()
            .chars()
            .map(|c| if c.is_alphanumeric() { c } else { '-' })
            .collect();
        for (suffix, style) in [("tecnico", None), ("color", presentation)] {
            let mut options = archisim_plan::PlanOptions {
                scale: 100.0,
                ..Default::default()
            };
            options.style.presentation = style;
            let svg = dir.join(format!("vivero-alzado-{slug}-{suffix}.svg"));
            std::fs::write(
                &svg,
                archisim_plan::section::render_section(&m, &drawing, &options),
            )?;
        }
        println!(
            "  Alzado de «{}» 1:100: vivero-alzado-{slug}-….svg",
            sec.name
        );
    }
    // The line drawings of trees (25/10/2026): every plant in each of the
    // four styles, in plan and in the elevations, lines only.
    // AD5's own drawings (26/10/2026), in colour as the studio uses them.
    for (name, slug, look) in [
        ("corona", "corona", "líneas"),
        ("silueta", "silueta", "líneas"),
        ("ramitos", "ramitos", "líneas"),
        ("ramas y borde", "ramas-y-borde", "líneas"),
        ("AD5 círculos", "ad5-circulos", "transparente"),
        ("AD5 racimos", "ad5-racimos", "transparente"),
        ("AD5 estrella", "ad5-estrella", "transparente"),
        ("AD5 copa", "ad5-copa", "transparente"),
    ] {
        let mut t = m.clone();
        let ids: Vec<_> = t.object_types().map(|x| x.id).collect();
        for id in ids {
            if !t.type_grows(id) {
                continue;
            }
            let Some(mut v) = t.object_type(id).map(|x| x.values.clone()) else {
                continue;
            };
            for (k, val) in [
                ("style", name),
                ("tech_style", name),
                ("look", look),
                ("plan_drawing", "presentación"),
            ] {
                v.insert(k.into(), archisim_core::Value::Text(val.into()));
            }
            t.set_object_type_values(id, v)?;
        }
        let plan = archisim_plan::PlanOptions {
            scale: 200.0,
            ..Default::default()
        };
        std::fs::write(
            dir.join(format!("vivero-{slug}-planta.svg")),
            archisim_plan::render_plan(&t, level, &plan),
        )?;
        let mut rows = String::new();
        for (i, sec) in t.sections().enumerate() {
            let drawing = archisim_plan::section::section_drawing(&t, sec);
            let options = archisim_plan::PlanOptions {
                scale: 100.0,
                ..Default::default()
            };
            std::fs::write(
                dir.join(format!("vivero-{slug}-alzado-{}.svg", i + 1)),
                archisim_plan::section::render_section(&t, &drawing, &options),
            )?;
            rows = format!("{} alzados", i + 1);
        }
        println!("  Estilo «{name}»: vivero-{slug}-planta.svg y {rows}");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    #[test]
    fn every_species_is_planted() {
        let m = super::build().unwrap();
        let species = archisim_library::flora::FLORA_CATALOGUE.len();
        // One of each, nine plane trees and forty aromatic plants.
        assert_eq!(
            m.objects().filter(|o| m.type_grows(o.object_type)).count(),
            species + 9 + 40
        );
        assert_eq!(m.plantings().count(), 2);
    }
}
