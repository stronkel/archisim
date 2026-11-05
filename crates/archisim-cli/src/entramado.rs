//! `archisim entramado`: two small houses built of light frames side by
//! side, one in steel frame (C studs and U tracks of galvanised steel) and
//! one in timber frame (C24), each with its walls, doors and windows, its
//! first floor and its pitched roof framed; and their breakdown: the cut
//! list and the panels on paper (SVG and PNG), the workbook for Excel
//! (`.xlsx`), the BC3 for the budget and the IFC.

use std::path::Path;

use archisim_core::{
    FrameMaterial, Framing, Layer, LayerFunction, Model, NewOpening, NewRoof, NewSlab, NewWall,
    Point2, ReferenceLine, SlabFunction, WallFunction, WallSide, WallTop,
};

type Res<T = ()> = Result<T, Box<dyn std::error::Error>>;

fn find(m: &Model, name: &str) -> Option<archisim_core::MaterialId> {
    m.materials().find(|x| x.name == name).map(|x| x.id)
}

/// One framed house of `material` with its south-west corner at `at`.
fn house(
    m: &mut Model,
    at: Point2,
    material: FrameMaterial,
    lib: &archisim_library::Library,
) -> Res {
    let pb = m.levels()[0].id;
    let p1 = m.levels()[1].id;
    let wool = find(m, "Lana mineral").ok_or("sin lana")?;
    let plaster = find(m, "Enlucido de yeso").ok_or("sin yeso")?;
    let render = find(m, "Mortero monocapa").ok_or("sin mortero")?;
    let osb = match find(m, "Tablero OSB/3") {
        Some(id) => id,
        None => m.add_material(
            "Tablero OSB/3",
            "wood",
            [205, 170, 110],
            archisim_core::CutPattern::Solid,
        ),
    };
    archisim_library::frame_material(m, material);
    let tag = match material {
        FrameMaterial::Steel => "steel frame",
        FrameMaterial::Timber => "entramado de madera",
    };
    let layer = |material, thickness, function| Layer {
        material,
        thickness,
        function,
    };
    let wt = m.add_wall_type(
        format!("Fachada {tag} 150"),
        WallFunction::Exterior,
        vec![
            layer(render, 0.015, LayerFunction::Finish),
            layer(osb, 0.012, LayerFunction::Substrate),
            layer(wool, 0.15, LayerFunction::Structure),
            layer(plaster, 0.015, LayerFunction::Finish),
        ],
    )?;
    m.set_type_framing(wt.0, Some(Framing::for_depth(material, 0.15)))?;
    let st = m.add_slab_type(
        format!("Forjado {tag}"),
        SlabFunction::Floor,
        vec![
            layer(osb, 0.022, LayerFunction::Substrate),
            layer(wool, 0.22, LayerFunction::Structure),
            layer(plaster, 0.015, LayerFunction::Finish),
        ],
    )?;
    let mut floor = Framing::for_depth(material, 0.22);
    floor.spacing = 0.4;
    m.set_type_framing(st.0, Some(floor))?;
    let rt = m.add_slab_type(
        format!("Cubierta {tag}"),
        SlabFunction::Roof,
        vec![
            layer(osb, 0.018, LayerFunction::Substrate),
            layer(wool, 0.2, LayerFunction::Structure),
            layer(plaster, 0.015, LayerFunction::Finish),
        ],
    )?;
    m.set_type_framing(rt.0, Some(Framing::for_depth(material, 0.2)))?;
    let (w, d) = (9.0, 7.0);
    let corners = [
        at,
        at + Point2::new(w, 0.0),
        at + Point2::new(w, d),
        at + Point2::new(0.0, d),
    ];
    let mut walls = Vec::new();
    for level in [pb, p1] {
        for i in 0..4 {
            // Clockwise round the house: the exterior to the left.
            let (start, end) = (corners[(i + 1) % 4], corners[i]);
            walls.push((
                level,
                m.add_wall(NewWall {
                    wall_type: wt,
                    level,
                    start,
                    end,
                    bulge: 0.0,
                    reference: ReferenceLine::Center,
                    height: 2.8,
                    base_offset: 0.0,
                    top: WallTop::Unconnected,
                })?,
            ));
        }
    }
    let opening = |m: &mut Model, wall, t, along: f64, sill: f64| -> Res {
        m.add_opening(NewOpening {
            opening_type: t,
            wall,
            along,
            sill,
            opens_to: WallSide::Interior,
            hand: archisim_core::Hand::Left,
            setback: None,
        })?;
        Ok(())
    };
    for (i, (level, wall)) in walls.clone().into_iter().enumerate() {
        let long = i % 2 == 0;
        let ground = level == pb;
        if i % 4 == 0 && ground {
            opening(m, wall, lib.entrance_door, 2.0, 0.0)?;
            // A wide sliding door: a deeper header.
            opening(m, wall, lib.sliding_door, 6.0, 0.0)?;
        } else if long {
            opening(m, wall, lib.window_two_leaves, 2.4, 0.9)?;
            opening(m, wall, lib.window_one_leaf, 6.4, 0.9)?;
        } else {
            opening(m, wall, lib.window_one_leaf, 3.5, 0.9)?;
        }
    }
    let inside = corners.to_vec();
    m.add_slab(NewSlab {
        slab_type: st,
        level: p1,
        outline: inside.clone(),
        top_offset: 0.0,
    })?;
    let roof = m.add_roof(NewRoof {
        roof_type: rt,
        level: p1,
        outline: inside,
        slopes: vec![Some(0.5), None, Some(0.5), None],
        base_offset: 2.8,
        overhang: 0.4,
    })?;
    // The walls of the upper floor up to the roof: the gables.
    for (level, wall) in &walls {
        if *level == p1 {
            m.set_wall_roof(*wall, Some(roof))?;
        }
    }
    // A curved garden wall in front, of the same type.
    m.add_wall(NewWall {
        wall_type: wt,
        level: pb,
        start: at + Point2::new(9.0, -4.0),
        end: at + Point2::new(0.0, -4.0),
        bulge: -0.25,
        reference: ReferenceLine::Center,
        height: 2.2,
        base_offset: 0.0,
        top: WallTop::Unconnected,
    })?;
    Ok(())
}

pub fn build() -> Res<Model> {
    let mut m = Model::new("Entramados ligeros");
    m.add_level("Planta baja", 0.0);
    m.add_level("Planta primera", 3.06);
    let lib = archisim_library::add_library(&mut m)?;
    house(&mut m, Point2::new(0.0, 0.0), FrameMaterial::Steel, &lib)?;
    house(&mut m, Point2::new(14.0, 0.0), FrameMaterial::Timber, &lib)?;
    // A section across both houses, looking north.
    m.add_section(archisim_core::NewSection {
        name: "A".into(),
        kind: archisim_core::SectionKind::Section,
        start: Point2::new(-1.0, 3.3),
        end: Point2::new(24.0, 3.3),
        depth: 4.0,
    })?;
    Ok(m)
}

pub fn write(dir: &Path) -> Res {
    let m = build()?;
    std::fs::create_dir_all(dir)?;
    let project = dir.join(format!("entramado.{}", archisim_file::EXTENSION));
    archisim_file::save(&m, &project)?;
    std::fs::write(
        dir.join("entramado.ifc"),
        crate::ifc_text(&m, "entramado", None),
    )?;
    let svg = archisim_plan::framing::breakdown_svg(&m).ok_or("nada con entramado")?;
    std::fs::write(dir.join("entramado-despiece.svg"), &svg)?;
    std::fs::write(
        dir.join("entramado-despiece.png"),
        archisim_export::png::png(&svg, 100.0)?,
    )?;
    let book = archisim_export::xlsx::framing_workbook(&m).ok_or("nada con entramado")?;
    std::fs::write(dir.join("entramado.xlsx"), book)?;
    let date = archisim_export::bc3::bc3_date(&archisim_ifc::utc_now());
    std::fs::write(
        dir.join("entramado.bc3"),
        archisim_export::bc3::bc3(&m, &date, env!("CARGO_PKG_VERSION")),
    )?;
    // The ground floor at 1:20 and the section at 1:20: the studs, joists
    // and rafters cut.
    let options = archisim_plan::PlanOptions {
        scale: 20.0,
        ..Default::default()
    };
    let level = m.levels()[0].id;
    std::fs::write(
        dir.join("entramado-planta-20.svg"),
        archisim_plan::render_plan(&m, level, &options),
    )?;
    if let Some(sec) = m.sections().find(|s| s.name == "A") {
        let d = archisim_plan::section::section_drawing(&m, sec);
        std::fs::write(
            dir.join("entramado-seccion-20.svg"),
            archisim_plan::section::render_section(&m, &d, &options),
        )?;
    }
    let members = archisim_geom::framing::all_members(&m);
    let list = archisim_geom::framing::cut_list(&m, &members);
    let panels = archisim_geom::framing::wall_panels(&m);
    println!(
        "Entramados: {} piezas, {} paneles de muro.",
        members.len(),
        panels.len()
    );
    for t in &list.totals {
        let amount = if t.m3 > 0.0 {
            format!("{:.3} m³", t.m3)
        } else {
            format!("{:.1} kg", t.kg)
        };
        println!(
            "  {}: {} piezas, {:.1} m, {} ({} barras)",
            t.section, t.pieces, t.metres, amount, t.bars
        );
    }
    for b in &list.boards {
        println!("  {}: {:.1} m², {} tableros", b.name, b.area, b.boards);
    }
    println!("  Proyecto: {}", project.display());
    println!(
        "  Despiece: {}",
        dir.join("entramado-despiece.svg").display()
    );
    println!("  Excel: {}", dir.join("entramado.xlsx").display());
    println!("  BC3: {}", dir.join("entramado.bc3").display());
    println!("  IFC: {}", dir.join("entramado.ifc").display());
    Ok(())
}

#[cfg(test)]
mod tests {
    #[test]
    fn the_frames_are_drawn_cut_at_1_20() {
        let m = super::build().unwrap();
        let options = archisim_plan::PlanOptions {
            scale: 20.0,
            ..Default::default()
        };
        let plan = archisim_plan::render_plan(&m, m.levels()[0].id, &options);
        assert!(plan.contains("frame-cut"));
        let sec = m.sections().next().unwrap();
        let d = archisim_plan::section::section_drawing(&m, sec);
        let svg = archisim_plan::section::render_section(&m, &d, &options);
        assert!(svg.contains("frame-cut"));
        // Not at 1:100.
        let far = archisim_plan::PlanOptions {
            scale: 100.0,
            ..Default::default()
        };
        assert!(!archisim_plan::render_plan(&m, m.levels()[0].id, &far).contains("frame-cut"));
    }
}
