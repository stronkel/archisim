//! `archisim entorno <carpeta>`: a street of the surroundings with every
//! style of building (U6), so that Antonio can see them all together: a
//! row of Andalusian houses between party walls, the corner of an
//! «ensanche», a 1960s block, offices, a coastal hotel, a school, a
//! warehouse, terraced and semi-detached houses, a chalet and a courtyard
//! house, along two streets.

use std::path::Path;

use archisim_core::{
    BuildingStyle, Model, NewEnvBuilding, NewRoad, NewWall, Point2, ReferenceLine, WallTop,
};

type Res<T = ()> = Result<T, Box<dyn std::error::Error>>;

fn rect(x0: f64, y0: f64, x1: f64, y1: f64) -> Vec<Point2> {
    vec![
        Point2::new(x0, y0),
        Point2::new(x1, y0),
        Point2::new(x1, y1),
        Point2::new(x0, y1),
    ]
}

/// The pictures of the decals, drawn here (no files of anyone else's).
const GRAFFITI: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" width="300mm" height="120mm" viewBox="0 0 300 120">
<path d="M10 90 C40 10 70 10 90 80 S150 120 170 40 S240 0 290 70" fill="none" stroke="#e8326b" stroke-width="22" stroke-linecap="round"/>
<path d="M20 70 C50 30 80 40 100 90 S160 100 180 30 S250 20 280 90" fill="none" stroke="#2fb7e8" stroke-width="10" stroke-linecap="round"/>
<circle cx="60" cy="40" r="14" fill="#ffd23a"/><circle cx="230" cy="40" r="10" fill="#7be06a"/>
<path d="M120 20 l20 30 l20 -30 l20 30" fill="none" stroke="#1d1d1d" stroke-width="6" stroke-linejoin="round"/>
</svg>"##;
const SIGN: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" width="400mm" height="40mm" viewBox="0 0 400 40">
<rect width="400" height="40" rx="4" fill="#7a1f2b"/>
<rect x="8" y="6" width="384" height="28" rx="3" fill="none" stroke="#f2d48a" stroke-width="2"/>
<g fill="#f2d48a"><rect x="40" y="13" width="60" height="14"/><rect x="115" y="13" width="40" height="14"/><rect x="170" y="13" width="90" height="14"/><rect x="275" y="13" width="80" height="14"/></g>
</svg>"##;
const STAIN: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" width="160mm" height="100mm" viewBox="0 0 160 100">
<path d="M10 100 C10 60 30 70 40 40 S70 30 80 50 S110 20 130 45 S150 70 150 100 Z" fill="#9b8a6a" fill-opacity="0.75"/>
<path d="M25 100 C30 75 45 80 55 60 S85 55 95 70 S120 55 135 75 L140 100 Z" fill="#7d6c4e" fill-opacity="0.8"/>
</svg>"##;
const ARROW: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" width="60mm" height="250mm" viewBox="0 0 60 250">
<path d="M22 250 V90 H5 L30 0 L55 90 H38 V250 Z" fill="#f4f1e8"/>
<g fill="#000" fill-opacity="0"><rect width="60" height="250"/></g>
</svg>"##;

pub fn build() -> Res<Model> {
    let mut m = Model::new("Entorno de ArchiSim");
    let pb = m.add_level("Planta baja", 0.0);
    let p1 = m.add_level("Planta primera", 3.0);
    let lib = archisim_library::add_library(&mut m)?;
    let find = |m: &Model, c: &str| m.materials().find(|x| x.category == c).map(|x| x.id);
    let asphalt = find(&m, "asphalt")
        .or_else(|| find(&m, "concrete"))
        .ok_or("sin asfalto")?;
    let stone = find(&m, "stone").unwrap_or(asphalt);
    let paving = find(&m, "ceramic").unwrap_or(stone);
    let section = archisim_core::street_types()
        .into_iter()
        .find(|t| t.name.starts_with("Calle de barrio"))
        .or_else(|| archisim_core::street_types().into_iter().next())
        .ok_or("sin tipos de calle")?
        .section;
    let half = section.width() / 2.0;
    let p = Point2::new;
    for (name, axis) in [
        ("Calle Mayor", vec![p(-10.0, 0.0), p(170.0, 0.0)]),
        ("Avenida del Mar", vec![p(-10.0, 60.0), p(170.0, 60.0)]),
    ] {
        m.add_road(NewRoad {
            name: name.into(),
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
    let put = |m: &mut Model, style: BuildingStyle, outline: Vec<Point2>, floors| {
        let name = format!("{} {}", style.name(), m.env_buildings().count() + 1);
        m.add_env_building(NewEnvBuilding {
            name,
            level: pb,
            outline,
            style,
            floors,
            floor_height: None,
            base_offset: 0.0,
        })
    };
    // North of the Calle Mayor: the old town, houses between party walls.
    let y0 = half + 0.5;
    let mut x = 0.0;
    for (style, w, d, floors) in [
        (BuildingStyle::Andalusian, 6.0, 14.0, Some(2)),
        (BuildingStyle::Andalusian, 5.5, 14.0, Some(1)),
        (BuildingStyle::Andalusian, 7.0, 14.0, Some(2)),
        (BuildingStyle::Andalusian, 6.5, 14.0, Some(2)),
        (BuildingStyle::HistoricCentre, 9.0, 16.0, Some(3)),
        (BuildingStyle::HistoricCentre, 8.0, 16.0, Some(4)),
        (BuildingStyle::ShopAndFlats, 12.0, 16.0, None),
        (BuildingStyle::Ensanche, 20.0, 18.0, None),
    ] {
        put(&mut m, style, rect(x, y0, x + w, y0 + d), floors)?;
        x += w;
    }
    // South of it: the newer blocks.
    let y1 = -half - 0.5;
    x = 0.0;
    for (style, w, d, gap) in [
        (BuildingStyle::Block1960s, 24.0, 14.0, 6.0),
        (BuildingStyle::Offices, 26.0, 18.0, 8.0),
        (BuildingStyle::CoastalHotel, 34.0, 16.0, 8.0),
        (BuildingStyle::Facility, 30.0, 18.0, 8.0),
        (BuildingStyle::Warehouse, 36.0, 24.0, 0.0),
    ] {
        put(&mut m, style, rect(x, y1 - d, x + w, y1), None)?;
        x += w + gap;
    }
    // South of the Avenida del Mar: houses.
    let y2 = 60.0 - half - 4.0;
    x = 0.0;
    for w in [6.0, 6.0, 6.0, 6.0] {
        put(
            &mut m,
            BuildingStyle::Terraced,
            rect(x, y2 - 10.0, x + w, y2),
            None,
        )?;
        x += w;
    }
    x += 8.0;
    for w in [8.0, 8.0] {
        put(
            &mut m,
            BuildingStyle::SemiDetached,
            rect(x, y2 - 10.0, x + w, y2),
            None,
        )?;
        x += w;
    }
    x += 10.0;
    put(
        &mut m,
        BuildingStyle::Detached,
        rect(x, y2 - 11.0, x + 12.0, y2),
        None,
    )?;
    x += 22.0;
    put(
        &mut m,
        BuildingStyle::Courtyard,
        rect(x, y2 - 18.0, x + 18.0, y2),
        None,
    )?;
    // An L-shaped chalet, for the roof over a re-entrant corner.
    x += 26.0;
    put(
        &mut m,
        BuildingStyle::Detached,
        vec![
            p(x, y2 - 14.0),
            p(x + 14.0, y2 - 14.0),
            p(x + 14.0, y2 - 7.0),
            p(x + 7.0, y2 - 7.0),
            p(x + 7.0, y2),
            p(x, y2),
        ],
        None,
    )?;
    // Decals (U8): a graffiti on the warehouse, a shop's sign, a damp
    // stain on a house and a worn arrow on the street.
    let x_shop = 6.0 + 5.5 + 7.0 + 6.5 + 9.0 + 8.0;
    let x_store = 24.0 + 6.0 + 26.0 + 8.0 + 34.0 + 8.0 + 30.0 + 8.0;
    let floor = m.surface_at(pb, p(20.0, 0.0));
    for (svg, source, centre, normal, width) in [
        (
            GRAFFITI,
            "grafiti.png",
            [x_store + 18.0, y1, 2.2],
            [0.0, 1.0, 0.0],
            9.0,
        ),
        (
            SIGN,
            "rotulo.png",
            [x_shop + 6.0, y0, 2.46],
            [0.0, -1.0, 0.0],
            7.0,
        ),
        (STAIN, "humedad.png", [3.0, y0, 0.7], [0.0, -1.0, 0.0], 1.6),
        (
            ARROW,
            "flecha.png",
            [20.0, -1.6, floor],
            [0.0, 0.0, 1.0],
            1.2,
        ),
    ] {
        let png = archisim_export::png::png(svg, 96.0)?;
        let image = archisim_core::decal_image(source, png).ok_or("PNG no válido")?;
        let name = m.next_decal_name();
        m.add_decal(archisim_core::NewDecal {
            name,
            level: pb,
            centre,
            normal,
            rotation: 0.0,
            width,
            image,
            host: None,
        })?;
    }
    // A kiosk on the corner, the project among its surroundings.
    let corners = [p(-12.0, 8.0), p(-8.0, 8.0), p(-8.0, 11.0), p(-12.0, 11.0)];
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

pub fn write(dir: &Path) -> Res {
    let m = build()?;
    std::fs::create_dir_all(dir)?;
    let project = dir.join(format!("entorno.{}", archisim_file::EXTENSION));
    archisim_file::save(&m, &project)?;
    std::fs::write(
        dir.join("entorno.ifc"),
        crate::ifc_text(&m, "entorno", None),
    )?;
    let pb = m.levels()[0].id;
    let options = archisim_plan::PlanOptions {
        scale: 500.0,
        ..Default::default()
    };
    let plan = archisim_plan::render_plan(&m, pb, &options);
    std::fs::write(dir.join("entorno-planta.svg"), &plan)?;
    std::fs::write(
        dir.join("entorno-planta.png"),
        archisim_export::png::png(&plan, 200.0)?,
    )?;
    println!(
        "Edificios del entorno: {} de {} estilos.",
        m.env_buildings().count(),
        BuildingStyle::ALL.len()
    );
    println!("  Proyecto: {}", project.display());
    println!("  Planta: {}", dir.join("entorno-planta.svg").display());
    println!("  IFC: {}", dir.join("entorno.ifc").display());
    Ok(())
}

#[cfg(test)]
mod tests {
    /// Every style is there, the old town's houses share party walls, and
    /// the project and its IFC read back the same.
    #[test]
    fn a_street_of_every_style() {
        let m = super::build().unwrap();
        let styles: std::collections::BTreeSet<_> =
            m.env_buildings().map(|b| b.dress.unwrap().style).collect();
        assert_eq!(styles.len(), archisim_core::BuildingStyle::ALL.len());
        assert_eq!(m.decals().count(), 4);
        let first = m.env_buildings().next().unwrap();
        assert_eq!(m.party_sides(first.id), vec![false, true, false, false]);
        let back = archisim_file::from_str(&archisim_file::to_string(&m)).unwrap();
        assert_eq!(
            back.env_buildings().map(|b| b.dress).collect::<Vec<_>>(),
            m.env_buildings().map(|b| b.dress).collect::<Vec<_>>()
        );
        assert_eq!(
            back.decals().map(|d| &d.image).collect::<Vec<_>>(),
            m.decals().map(|d| &d.image).collect::<Vec<_>>()
        );
        let ifc = crate::ifc_text(&m, "entorno", None);
        let imported = archisim_ifc::import_ifc(&ifc).unwrap();
        assert_eq!(
            imported.model.env_buildings().count(),
            m.env_buildings().count()
        );
        let diffs = archisim_ifc::differences(&m, &imported.model);
        assert!(
            !diffs
                .iter()
                .any(|d| d.contains("entorno") || d.contains("pegatina")),
            "{diffs:#?}"
        );
        assert_eq!(imported.model.decals().count(), 4);
    }
}
