//! `archisim diseno <modelo> [carpeta] [Fabricante]`: a manufacturer's 3D
//! model (FBX, OBJ, glTF, COLLADA, 3DS, STL or the download's `.zip`)
//! brought in as a design piece the way the application does, placed on
//! its own, saved as a project and exported to IFC; says what was read
//! (format, units, faces before and after lightening it, materials and
//! what each was taken for) so a real download can be tried without the
//! application.

use std::path::Path;

use archisim_library::imported::{
    finish_of, import_mesh_with, ImportOptions, Maker, ModelKind, DEFAULT_DETAIL, DETAILS,
};

type Res<T = ()> = Result<T, Box<dyn std::error::Error>>;

pub fn write(model: &Path, dir: &Path, manufacturer: Option<&str>) -> Res {
    let loaded = archisim_mesh::load(model, None, Default::default())?;
    let name = model
        .file_stem()
        .map(|s| s.to_string_lossy().replace(['_', '-'], " "))
        .unwrap_or_else(|| "Pieza de diseño".into());
    if let Some(e) = &loaded.entry {
        println!(
            "Dentro del .zip: {e} (de {} modelos legibles)",
            loaded.models.len()
        );
    }
    if !loaded.others.is_empty() {
        println!("Trae también, sin leer: {}", loaded.others.join(", "));
    }
    let mesh = &loaded.mesh;
    let (lo, hi) = mesh.bounds();
    println!(
        "Medidas: {:.3} × {:.3} m de planta, {:.3} m de alto · {} caras",
        hi[0] - lo[0],
        hi[1] - lo[1],
        hi[2] - lo[2],
        mesh.triangle_count()
    );
    if let Some(k) = loaded.guessed_unit {
        println!("Unidades supuestas por su tamaño: {k} m por unidad");
    }
    for m in &mesh.materials {
        let (category, texture) = finish_of(&m.name);
        println!(
            "  material «{}»: {category}{}{}",
            m.name,
            texture.map_or(String::new(), |t| format!(", textura {t:?}")),
            m.texture
                .as_ref()
                .map_or(String::new(), |f| format!(", imagen {f}"))
        );
    }
    let options = ImportOptions {
        turns: 0,
        detail: DETAILS[DEFAULT_DETAIL].1,
        maker: Some(Maker {
            manufacturer: manufacturer.unwrap_or("Fabricante").to_string(),
            product: name.clone(),
            ..Default::default()
        }),
    };
    let mut m = archisim_library::new_project("Pieza de diseño");
    let level = m.levels()[0].id;
    let source = model
        .file_name()
        .map(|f| f.to_string_lossy().into_owned())
        .unwrap_or_default();
    let started = std::time::Instant::now();
    let r = import_mesh_with(
        &mut m,
        loaded.mesh,
        &name,
        &source,
        ModelKind::Furniture,
        &options,
    )?;
    let far = if r.triangles.len() > 1 {
        r.triangles[1..]
            .iter()
            .map(|t| t.to_string())
            .collect::<Vec<_>>()
            .join(" y ")
            + " de lejos"
    } else {
        "ya es ligera: la misma de lejos".into()
    };
    println!(
        "Importada en {:.1} s: {} caras de cerca, {far}",
        started.elapsed().as_secs_f64(),
        r.triangles[0],
    );
    m.add_object(archisim_core::NewObject {
        level,
        object_type: r.object_type,
        position: archisim_core::Point2::new(0.0, 0.0),
        rotation: 0.0,
        base_offset: 0.0,
    })?;
    std::fs::create_dir_all(dir)?;
    archisim_file::save(&m, dir.join(format!("diseno.{}", archisim_file::EXTENSION)))?;
    std::fs::write(dir.join("diseno.ifc"), crate::ifc_text(&m, "diseno", None))?;
    let options = archisim_plan::PlanOptions {
        scale: 20.0,
        ..Default::default()
    };
    std::fs::write(
        dir.join("diseno-planta.svg"),
        archisim_plan::render_plan(&m, level, &options),
    )?;
    println!(
        "Pieza de diseño en {}: diseno.archisim, diseno.ifc y diseno-planta.svg",
        dir.display()
    );
    Ok(())
}
