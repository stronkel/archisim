//! `archisim taller`: the workshop's models (M1), each new piece in front
//! of the library's piece it is to replace, to look at in 3D and judge
//! before they enter the library.

use std::path::Path;

type Res<T = ()> = Result<T, Box<dyn std::error::Error>>;

pub fn write(dir: &Path) -> Res {
    let m = archisim_library::workshop::project()?;
    std::fs::create_dir_all(dir)?;
    archisim_file::save(&m, dir.join(format!("taller.{}", archisim_file::EXTENSION)))?;
    std::fs::write(dir.join("taller.ifc"), crate::ifc_text(&m, "taller", None))?;
    let axo = archisim_core::Axo::default();
    let drawing = archisim_plan::axo::axo_drawing(&m, &axo, Default::default());
    let crop = archisim_plan::axo::axo_extent(&m, &axo, 20.0);
    std::fs::write(
        dir.join("taller-3d.svg"),
        archisim_plan::axo::render_axo(&drawing, crop, 20.0),
    )?;
    let level = m.levels()[0].id;
    let options = archisim_plan::PlanOptions {
        scale: 50.0,
        ..Default::default()
    };
    std::fs::write(
        dir.join("taller-planta.svg"),
        archisim_plan::render_plan(&m, level, &options),
    )?;
    for piece in archisim_library::workshop::PIECES {
        let t = m
            .object_types()
            .find(|t| t.name == piece.name)
            .ok_or("falta un tipo del taller")?;
        let tris = m.type_prop(t.id).map_or(0, |p| p.0.triangle_count());
        println!("{}: {tris} triángulos", piece.name);
    }
    println!(
        "Taller: {} piezas nuevas delante de las que sustituirían, en {}",
        archisim_library::workshop::PIECES.len(),
        dir.display()
    );
    Ok(())
}

/// `archisim edificios`: the workshop of buildings (tanda 1), each new
/// building on the south side of a street facing today's of the same
/// kind: edificios.archisim and edificios.ifc.
pub fn write_buildings(dir: &Path) -> Res {
    let m = archisim_library::building_workshop::project()?;
    std::fs::create_dir_all(dir)?;
    archisim_file::save(
        &m,
        dir.join(format!("edificios.{}", archisim_file::EXTENSION)),
    )?;
    std::fs::write(
        dir.join("edificios.ifc"),
        crate::ifc_text(&m, "edificios", None),
    )?;
    for b in archisim_library::building_workshop::BUILDINGS {
        let t = m
            .object_types()
            .find(|t| t.name == b.name)
            .ok_or("falta un edificio del taller")?;
        let tris = m.type_prop(t.id).map_or(0, |p| p.0.triangle_count());
        println!("{}: {tris} triángulos", b.name);
    }
    println!(
        "Taller de edificios: {} edificios nuevos frente a los de hoy, en {}",
        archisim_library::building_workshop::BUILDINGS.len(),
        dir.join(format!("edificios.{}", archisim_file::EXTENSION))
            .display()
    );
    Ok(())
}

/// The buildings of the world (tanda 6): every place's house of the
/// street, block and detached house, a row each (`only`: some places by
/// their key).
/// The facilities of the places asked for (by key), a project each.
pub fn write_facilities(dir: &Path, only: &[&str]) -> Res {
    use archisim_core::props::fine::buildings::world::SKINS;
    std::fs::create_dir_all(dir)?;
    for (i, sk) in SKINS.iter().enumerate() {
        if !only.is_empty() && !only.contains(&sk.key) {
            continue;
        }
        if only.is_empty() && i > 0 && SKINS[..i].iter().any(|s| s.key == sk.key) {
            continue;
        }
        let m = archisim_library::building_workshop::facility_project(i)?;
        let name = format!("equipamientos-{}", sk.key);
        let path = dir.join(format!("{name}.{}", archisim_file::EXTENSION));
        archisim_file::save(&m, &path)?;
        std::fs::write(
            dir.join(format!("{name}.ifc")),
            crate::ifc_text(&m, &name, None),
        )?;
        let total: usize = m
            .object_types()
            .filter_map(|t| m.type_prop(t.id).map(|p| p.0.triangle_count()))
            .sum();
        println!(
            "{}: {} edificios ({total} triángulos), en {}",
            sk.name,
            m.objects().count(),
            path.display()
        );
    }
    Ok(())
}

pub fn write_world(dir: &Path, only: &[&str]) -> Res {
    let m = archisim_library::building_workshop::world_project(only)?;
    std::fs::create_dir_all(dir)?;
    let path = dir.join(format!("mundo.{}", archisim_file::EXTENSION));
    archisim_file::save(&m, &path)?;
    std::fs::write(dir.join("mundo.ifc"), crate::ifc_text(&m, "mundo", None))?;
    let mut total = 0;
    for t in m.object_types().filter(|t| t.name.ends_with("(taller)")) {
        let tris = m.type_prop(t.id).map_or(0, |p| p.0.triangle_count());
        total += tris;
        println!("{}: {tris} triángulos", t.name);
    }
    println!(
        "Edificios del mundo: {} edificios ({total} triángulos), en {}",
        m.objects().count(),
        path.display()
    );
    Ok(())
}
