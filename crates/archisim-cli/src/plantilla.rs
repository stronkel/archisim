//! `archisim plantilla`: a project template from a project (Revit's «Guardar
//! como plantilla»): the same levels, materials, types, view templates,
//! title blocks, sheets, tables and calculation data, with nothing drawn.
//! With a catalogue of frame sections (a file of the studio's library:
//! name; material; depth; flange; lip; thickness), the framed wall and slab
//! types take its studs and tracks by the depth of their frame layer, and
//! with `--entramado` every wall type with insulation between boards is
//! framed too.

use std::path::Path;

use archisim_core::{CatalogueScope, FrameMaterial};

type Res<T = ()> = Result<T, Box<dyn std::error::Error>>;

/// Makes the template; returns what it says.
pub fn run(input: &Path, output: &Path, catalogue: Option<&Path>, cavities: bool) -> Res<String> {
    let model = archisim_file::open(input)?;
    let mut t = archisim_file::template_of(&model)?;
    let mut text = String::new();
    use std::fmt::Write as _;
    let _ = writeln!(
        text,
        "Plantilla de «{}»: {} plantas, {} materiales, {} tipos de muro, {} de forjado, {} de puerta o ventana, {} de objeto, {} plantillas de vista.",
        model.project_name,
        t.levels().len(),
        t.materials().count(),
        t.wall_types().count(),
        t.slab_types().count(),
        t.opening_types().count(),
        t.object_types().count(),
        t.view_templates().count()
    );
    if let Some(path) = catalogue {
        let list = archisim_core::parse_section_catalogue(&std::fs::read_to_string(path)?);
        if list.is_empty() {
            return Err(format!("{} no tiene perfiles que se puedan leer", path.display()).into());
        }
        archisim_library::frame_material(&mut t, FrameMaterial::Steel);
        let scope = if cavities {
            CatalogueScope::Cavities
        } else {
            CatalogueScope::Framed
        };
        let fits = t.frame_types_from_catalogue(&list, scope);
        let _ = writeln!(
            text,
            "\nPerfiles de {} ({} perfiles):",
            path.file_name()
                .map(|n| n.to_string_lossy())
                .unwrap_or_default(),
            list.len()
        );
        if fits.is_empty() {
            let _ = writeln!(
                text,
                "  Ningún tipo con entramado. Con --entramado se ponen en los muros con aislamiento entre placas."
            );
        }
        for f in &fits {
            let what = match &f.framing {
                Some((stud, track)) => format!("montantes {} y canales {}", stud.name, track.name),
                None => "ningún perfil del catálogo cabe: se queda como estaba".into(),
            };
            let _ = writeln!(
                text,
                "  {} (capa de {:.0} mm): {}",
                f.name,
                f.depth * 1000.0,
                what
            );
        }
    }
    if let Some(dir) = output.parent().filter(|d| !d.as_os_str().is_empty()) {
        std::fs::create_dir_all(dir)?;
    }
    archisim_file::save(&t, output)?;
    let _ = writeln!(text, "\nPlantilla: {}", output.display());
    Ok(text)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_template_of_the_boxes_with_a_catalogue() {
        let dir = std::env::temp_dir().join(format!("archisim-plantilla-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let project = dir.join("cajas.archisim");
        archisim_file::save(&crate::cajas::build().unwrap(), &project).unwrap();
        let csv = dir.join("perfiles.csv");
        std::fs::write(
            &csv,
            "C100;acero;100;43;15;1,2\nU103;acero;103;43;0;1,2\nC150;acero;150;43;15;1,5\nU153;acero;153;43;0;1,5\n",
        )
        .unwrap();
        let out = dir.join("plantilla.archisim");
        let text = run(&project, &out, Some(&csv), true).unwrap();
        let t = archisim_file::open(&out).unwrap();
        assert_eq!(t.walls().count(), 0);
        assert_eq!(t.boxes().count(), 0);
        assert!(t.wall_types().count() > 0);
        assert!(text.contains("C100"), "{text}");
        assert!(t.framings().any(|(_, f)| f.stud.name == "C100"), "{text}");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
