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
                            demo.ifc (IFC4) y demo.svg (planta 1:50).
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

    let level = model.levels()[0].id;
    let svg_path = dir.join("demo.svg");
    std::fs::write(
        &svg_path,
        render_plan(model, level, &PlanOptions::default()),
    )?;

    println!(
        "Proyecto «{}»: {} muros en {} planta(s).",
        model.project_name,
        model.walls().count(),
        model.levels().len()
    );
    println!("  IFC4:   {}", ifc_path.display());
    println!("  Planta: {}", svg_path.display());
    Ok(())
}
