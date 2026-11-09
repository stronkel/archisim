//! `archisim concepto-malla <fichero> [carpeta] [--edificios]
//! [--unidades m|cm|mm|in]`: a model of volumes (SketchUp's COLLADA, OBJ,
//! STL…, also inside a `.zip`) brought in as volumes of the concept phase
//! the way the application does: a project with them
//! (`concepto-malla.archisim`), the plan of the ground floor, the IFC and a
//! report of what each piece became (prism or mesh, closed or open, its
//! faces). With `--edificios` every prism is made a mass by floors of
//! 3 m and turned into a building, so the IFC carries its walls and slabs.

use std::path::Path;

use archisim_core::{SlabFunction, WallFunction};
use archisim_library::concept_mesh::{import_concept, ConceptOptions, PieceKind};

type Res<T = ()> = Result<T, Box<dyn std::error::Error>>;

/// The height of each floor of the masses made buildings.
const FLOOR: f64 = 3.0;

pub fn write(model: &Path, dir: &Path, buildings: bool, unit: Option<f64>) -> Res {
    let mut loaded = archisim_mesh::load(
        model,
        None,
        archisim_mesh::Options {
            unit,
            ..Default::default()
        },
    )?;
    loaded.guess_for_site();
    if let Some(k) = loaded.guessed_unit {
        println!("El fichero no dice sus unidades: leído a {k} m por unidad por su tamaño.");
    }
    let (lo, hi) = loaded.mesh.bounds();
    println!(
        "Medidas: {:.2} × {:.2} m de planta, {:.2} m de alto · {} triángulos",
        hi[0] - lo[0],
        hi[1] - lo[1],
        hi[2] - lo[2],
        loaded.mesh.triangle_count()
    );
    let name = model
        .file_stem()
        .map(|s| s.to_string_lossy().replace(['_', '-'], " "))
        .unwrap_or_else(|| "Volumen importado".into());
    let source = model
        .file_name()
        .map(|f| f.to_string_lossy().into_owned())
        .unwrap_or_default();
    let mut m = archisim_library::new_project("Volúmenes de Concepto");
    let level = m.levels()[0].id;
    let r = import_concept(&mut m, &loaded.mesh, &ConceptOptions::new(level, &name))?;
    let mut text = r.report(&source);
    if buildings {
        let wall = m
            .wall_types()
            .find(|t| t.function == WallFunction::Exterior)
            .or_else(|| m.wall_types().next())
            .ok_or("sin tipos de muro")?
            .id;
        let slab = |f: SlabFunction| {
            m.slab_types()
                .find(|t| t.function == f)
                .or_else(|| m.slab_types().next())
                .map(|t| t.id)
                .ok_or("sin tipos de forjado")
        };
        let (g, f, roof) = (
            slab(SlabFunction::BaseSlab)?,
            slab(SlabFunction::Floor)?,
            slab(SlabFunction::Roof)?,
        );
        text.push('\n');
        for p in r.pieces.iter().filter(|p| p.kind == PieceKind::Prism) {
            m.edit_mass(p.mass, |x| x.floor_height = Some(FLOOR))?;
            let n = m.mass_to_building(p.mass, wall, g, f, roof)?;
            let floors = m.mass(p.mass).map_or(1, |x| x.floors());
            text.push_str(&format!(
                "«{}» pasado a edificio: {floors} plantas de {} m, {n} muros y forjados.\n",
                p.name,
                format!("{FLOOR:.2}").replace('.', ",")
            ));
        }
    }
    print!("{text}");
    std::fs::create_dir_all(dir)?;
    archisim_file::save(
        &m,
        dir.join(format!("concepto-malla.{}", archisim_file::EXTENSION)),
    )?;
    std::fs::write(dir.join("concepto-malla-informe.txt"), &text)?;
    std::fs::write(
        dir.join("concepto-malla.ifc"),
        crate::ifc_text(&m, "concepto-malla", None),
    )?;
    let options = archisim_plan::PlanOptions {
        scale: 200.0,
        ..Default::default()
    };
    std::fs::write(
        dir.join("concepto-malla-planta.svg"),
        archisim_plan::render_plan(&m, level, &options),
    )?;
    println!(
        "En {}: concepto-malla.archisim, concepto-malla.ifc, concepto-malla-planta.svg y concepto-malla-informe.txt",
        dir.display()
    );
    Ok(())
}
