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
