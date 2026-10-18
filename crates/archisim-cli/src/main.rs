//! `archisim` command line.
//!
//! ```text
//! archisim demo [carpeta]                    vivienda de prueba: .archisim, IFC y plantas
//! archisim exportar <proyecto.archisim> [carpeta]   IFC y plantas de un proyecto
//! archisim importar <fichero.ifc> [proyecto.archisim]   proyecto a partir de un IFC
//! archisim --version
//! ```

mod demo;

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use archisim_core::Model;
use archisim_ifc::{export_ifc, import_ifc, ExportOptions};
use archisim_plan::{render_plan, PlanOptions};

const HELP: &str = "\
ArchiSim — herramienta BIM (versión de desarrollo)

Uso:
  archisim demo [carpeta]
      Genera una vivienda de prueba, la guarda como proyecto demo.archisim
      y la exporta a demo.ifc (IFC4) y a una planta 1:50 por nivel:
      demo.svg (planta baja), demo-cubierta.svg…, y otra 1:100
      (demo-1a100.svg…), con los símbolos de puertas y ventanas de esa
      escala.
      Carpeta por defecto: ./out

  archisim exportar <proyecto.archisim> [carpeta]
      Abre un proyecto y lo exporta a IFC4 y a plantas 1:50 y 1:100 con el
      mismo nombre (casa.archisim → casa.ifc, casa.svg, casa-1a100.svg…). Por defecto, en la
      misma carpeta del proyecto.

  archisim importar <fichero.ifc> [proyecto.archisim]
      Lee un IFC (IFC2X3 o IFC4, de ArchiCAD, Revit u otro programa) y lo
      guarda como proyecto de ArchiSim: plantas, muros con sus capas,
      forjados, puertas y ventanas, con sus GlobalId. Explica qué ha
      quedado fuera. Por defecto, casa.ifc → casa.archisim en la misma
      carpeta.

  archisim --version        Muestra la versión
  archisim --help           Muestra esta ayuda
";

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        Some("demo") => {
            let dir = PathBuf::from(args.get(1).map(String::as_str).unwrap_or("out"));
            report(run_demo(&dir))
        }
        Some("exportar") | Some("export") => match args.get(1) {
            Some(project) => {
                let project = PathBuf::from(project);
                let dir = match args.get(2) {
                    Some(d) => PathBuf::from(d),
                    None => project.parent().map(Path::to_path_buf).unwrap_or_default(),
                };
                report(run_export(&project, &dir))
            }
            None => {
                eprintln!("Falta el proyecto: archisim exportar <proyecto.archisim> [carpeta]");
                ExitCode::FAILURE
            }
        },
        Some("importar") | Some("import") => match args.get(1) {
            Some(ifc) => {
                let ifc = PathBuf::from(ifc);
                let project = match args.get(2) {
                    Some(p) => PathBuf::from(p),
                    None => ifc.with_extension(archisim_file::EXTENSION),
                };
                report(run_import(&ifc, &project))
            }
            None => {
                eprintln!("Falta el fichero: archisim importar <fichero.ifc> [proyecto.archisim]");
                ExitCode::FAILURE
            }
        },
        Some("--version") | Some("-V") => {
            println!("archisim {}", env!("CARGO_PKG_VERSION"));
            ExitCode::SUCCESS
        }
        Some("--help") | Some("-h") | None => {
            print!("{HELP}");
            ExitCode::SUCCESS
        }
        Some(other) => {
            eprintln!("Orden desconocida: {other}\n\n{HELP}");
            ExitCode::FAILURE
        }
    }
}

fn report(result: Result<(), Box<dyn std::error::Error>>) -> ExitCode {
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("Error: {e}");
            ExitCode::FAILURE
        }
    }
}

fn run_demo(dir: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let doc = demo::build()?;
    std::fs::create_dir_all(dir)?;
    let project = dir.join(format!("demo.{}", archisim_file::EXTENSION));
    archisim_file::save(doc.model(), &project)?;
    summary(doc.model());
    println!("  Proyecto: {}", project.display());
    export_all(doc.model(), dir, "demo")
}

fn run_export(project: &Path, dir: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let model = archisim_file::open(project)
        .map_err(|e| format!("no se puede abrir {}: {e}", project.display()))?;
    let stem = project
        .file_stem()
        .and_then(|s| s.to_str())
        .filter(|s| !s.is_empty())
        .unwrap_or("proyecto")
        .to_string();
    if !dir.as_os_str().is_empty() {
        std::fs::create_dir_all(dir)?;
    }
    summary(&model);
    export_all(&model, dir, &stem)
}

fn run_import(ifc: &Path, project: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let bytes =
        std::fs::read(ifc).map_err(|e| format!("no se puede leer {}: {e}", ifc.display()))?;
    let text = String::from_utf8_lossy(&bytes);
    let imported = import_ifc(&text).map_err(|e| format!("{}: {e}", ifc.display()))?;
    print!("{}", imported.report);
    if let Some(dir) = project.parent().filter(|d| !d.as_os_str().is_empty()) {
        std::fs::create_dir_all(dir)?;
    }
    archisim_file::save(&imported.model, project)?;
    summary(&imported.model);
    println!("  Proyecto: {}", project.display());
    Ok(())
}

fn summary(model: &Model) {
    let count = |f: fn(&archisim_core::OpeningType) -> bool| {
        model
            .openings()
            .filter(|o| model.opening_type(o.opening_type).is_some_and(f))
            .count()
    };
    let (doors, windows, voids) = (
        count(|t| t.is_door()),
        count(|t| t.is_window()),
        count(|t| t.is_void()),
    );
    let voids = if voids > 0 {
        format!(", {voids} huecos")
    } else {
        String::new()
    };
    println!(
        "Proyecto «{}»: {} muros, {} forjados, {} puertas y {} ventanas{voids} en {} planta(s).",
        model.project_name,
        model.walls().count(),
        model.slabs().count(),
        doors,
        windows,
        model.levels().len()
    );
}

/// IFC file of a model, named after the project file.
fn ifc_text(model: &Model, stem: &str, timestamp: Option<String>) -> String {
    export_ifc(
        model,
        &ExportOptions {
            file_name: format!("{stem}.ifc"),
            author: "ArchiSim".into(),
            organization: "AD5".into(),
            timestamp,
        },
    )
}

/// Plan file names: the first level keeps the plain name; the others get a
/// suffix ("casa.svg", "casa-cubierta.svg").
fn plan_files(model: &Model, stem: &str) -> Vec<(archisim_core::LevelId, String, String)> {
    model
        .levels()
        .iter()
        .enumerate()
        .map(|(i, level)| {
            let file = if i == 0 {
                format!("{stem}.svg")
            } else {
                format!("{stem}-{}.svg", slug(&level.name))
            };
            (level.id, level.name.clone(), file)
        })
        .collect()
}

fn export_all(model: &Model, dir: &Path, stem: &str) -> Result<(), Box<dyn std::error::Error>> {
    let ifc_path = dir.join(format!("{stem}.ifc"));
    std::fs::write(&ifc_path, ifc_text(model, stem, None))?;
    println!("  IFC4:   {}", ifc_path.display());
    for (level, name, file) in plan_files(model, stem) {
        let svg_path = dir.join(&file);
        std::fs::write(
            &svg_path,
            render_plan(model, level, &PlanOptions::default()),
        )?;
        println!("  Planta «{name}»: {}", svg_path.display());
        let file_100 = format!("{}-1a100.svg", file.trim_end_matches(".svg"));
        let svg_100 = dir.join(&file_100);
        let options = PlanOptions {
            scale: 100.0,
            ..PlanOptions::default()
        };
        std::fs::write(&svg_100, render_plan(model, level, &options))?;
        println!("  Planta «{name}» 1:100: {}", svg_100.display());
    }
    Ok(())
}

/// A file-name friendly version of a level name: "Planta 1ª" → "planta-1a".
fn slug(name: &str) -> String {
    let mut out = String::new();
    for c in name.to_lowercase().chars() {
        let c = match c {
            'á' | 'à' | 'ä' | 'â' | 'ª' => 'a',
            'é' | 'è' | 'ë' | 'ê' => 'e',
            'í' | 'ì' | 'ï' | 'î' => 'i',
            'ó' | 'ò' | 'ö' | 'ô' | 'º' => 'o',
            'ú' | 'ù' | 'ü' | 'û' => 'u',
            'ñ' => 'n',
            'ç' => 'c',
            c if c.is_ascii_alphanumeric() => c,
            _ => '-',
        };
        if c != '-' || !(out.is_empty() || out.ends_with('-')) {
            out.push(c);
        }
    }
    out.trim_end_matches('-').to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Saving the demo house and opening it again must give exactly the same
    /// IFC (every GlobalId kept) and the same plans.
    #[test]
    fn demo_survives_save_and_open() {
        let original = demo::build().unwrap().model().clone();
        let text = archisim_file::to_string(&original);
        let reopened = archisim_file::from_str(&text).unwrap();
        assert_eq!(reopened, original);
        let stamp = || Some("2026-10-17T00:00:00".to_string());
        assert_eq!(
            ifc_text(&reopened, "demo", stamp()),
            ifc_text(&original, "demo", stamp())
        );
        for (level, _, _) in plan_files(&original, "demo") {
            assert_eq!(
                render_plan(&reopened, level, &PlanOptions::default()),
                render_plan(&original, level, &PlanOptions::default())
            );
        }
        assert_eq!(archisim_file::to_string(&reopened), text);
    }

    /// The demo exported to IFC and imported again is the same building,
    /// with the same GlobalIds, and exports to an equivalent IFC.
    #[test]
    fn demo_survives_ifc_round_trip() {
        let original = demo::build().unwrap().model().clone();
        let ifc = ifc_text(&original, "demo", None);
        let imported = import_ifc(&ifc).unwrap();
        let diffs = archisim_ifc::differences(&original, &imported.model);
        assert!(diffs.is_empty(), "{}", diffs.join("\n"));
        assert!(imported.report.skipped.is_empty(), "{}", imported.report);
        assert!(imported.report.warnings.is_empty(), "{}", imported.report);
        // Saved as a project and opened again, still the same.
        let text = archisim_file::to_string(&imported.model);
        let reopened = archisim_file::from_str(&text).unwrap();
        assert!(archisim_ifc::differences(&original, &reopened).is_empty());
    }

    #[test]
    fn slugs() {
        assert_eq!(super::slug("Cubierta"), "cubierta");
        assert_eq!(super::slug("Planta 1ª (vivienda)"), "planta-1a-vivienda");
    }
}
