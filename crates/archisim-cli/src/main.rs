//! `archisim` command line.
//!
//! ```text
//! archisim demo [carpeta]   genera demo.ifc y demo.svg (por defecto en ./out)
//! archisim --version
//! ```

mod demo;

use std::path::PathBuf;
use std::process::ExitCode;

use archisim_ifc::{export_ifc, ExportOptions};
use archisim_plan::{render_plan, PlanOptions};

const HELP: &str = "\
ArchiSim — herramienta BIM (versión de desarrollo)

Uso:
  archisim demo [carpeta]   Genera una vivienda de prueba y la exporta a
                            demo.ifc (IFC4) y a una planta 1:50 por nivel:
                            demo.svg (planta baja), demo-cubierta.svg…
                            Carpeta por defecto: ./out
  archisim --version        Muestra la versión
  archisim --help           Muestra esta ayuda
";

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        Some("demo") => {
            let dir = PathBuf::from(args.get(1).map(String::as_str).unwrap_or("out"));
            match run_demo(&dir) {
                Ok(()) => ExitCode::SUCCESS,
                Err(e) => {
                    eprintln!("Error: {e}");
                    ExitCode::FAILURE
                }
            }
        }
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

fn run_demo(dir: &std::path::Path) -> Result<(), Box<dyn std::error::Error>> {
    let doc = demo::build()?;
    let model = doc.model();
    std::fs::create_dir_all(dir)?;

    let ifc_path = dir.join("demo.ifc");
    let ifc = export_ifc(
        model,
        &ExportOptions {
            file_name: "demo.ifc".into(),
            author: "ArchiSim".into(),
            organization: "AD5".into(),
            timestamp: None,
        },
    );
    std::fs::write(&ifc_path, ifc)?;

    let doors = model
        .openings()
        .filter(|o| {
            model
                .opening_type(o.opening_type)
                .is_some_and(|t| t.is_door())
        })
        .count();
    println!(
        "Proyecto «{}»: {} muros, {} forjados, {} puertas y {} ventanas en {} planta(s).",
        model.project_name,
        model.walls().count(),
        model.slabs().count(),
        doors,
        model.openings().count() - doors,
        model.levels().len()
    );
    println!("  IFC4:   {}", ifc_path.display());

    // The first level keeps the plain name; the others get a suffix.
    for (i, level) in model.levels().iter().enumerate() {
        let file = if i == 0 {
            "demo.svg".to_string()
        } else {
            format!("demo-{}.svg", slug(&level.name))
        };
        let svg_path = dir.join(file);
        std::fs::write(
            &svg_path,
            render_plan(model, level.id, &PlanOptions::default()),
        )?;
        println!("  Planta «{}»: {}", level.name, svg_path.display());
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
    #[test]
    fn slugs() {
        assert_eq!(super::slug("Cubierta"), "cubierta");
        assert_eq!(super::slug("Planta 1ª (vivienda)"), "planta-1a-vivienda");
    }
}
