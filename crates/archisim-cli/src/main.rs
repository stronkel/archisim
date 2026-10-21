//! `archisim` command line.
//!
//! ```text
//! archisim demo [carpeta]                    vivienda de prueba: .archisim, IFC y plantas
//! archisim exportar <proyecto.archisim> [carpeta]   IFC y plantas de un proyecto
//! archisim importar <fichero.ifc> [proyecto.archisim]   proyecto a partir de un IFC
//! archisim medir [n] [carpeta]             rendimiento con n × n viviendas
//! archisim png <dibujo.svg> [imagen.png] [ppp]   imagen de un SVG
//! archisim comprobar-ids | comparar | interferencias | bcf   colaboración (fase 4, `colaboracion`)
//! archisim --version
//! ```

mod colaboracion;
mod demo;
mod grande;
mod muestrario;

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
      escala, y cada plano con su cajetín (demo-plano-a-01.svg).
      Carpeta por defecto: ./out

  archisim exportar <proyecto.archisim> [carpeta]
      Abre un proyecto y lo exporta a IFC4 y a plantas 1:50 y 1:100 con el
      mismo nombre (casa.archisim → casa.ifc, casa.svg, casa-1a100.svg…) y sus
      planos (casa-plano-a-01.svg…). Por defecto, en la
      misma carpeta del proyecto.

  archisim importar <fichero.ifc> [proyecto.archisim]
      Lee un IFC (IFC2X3 o IFC4, de ArchiCAD, Revit u otro programa) y lo
      guarda como proyecto de ArchiSim: plantas, muros con sus capas,
      forjados, puertas y ventanas, con sus GlobalId. Explica qué ha
      quedado fuera. Por defecto, casa.ifc → casa.archisim en la misma
      carpeta.

  archisim biblioteca [carpeta] [proyecto.archisim]
      Escribe los tipos de puerta, ventana y objeto de la plantilla (o de
      un proyecto) como ficheros de biblioteca: una familia por fichero
      .asfam con su catálogo de tipos, en subcarpetas por categoría
      (Puertas, Ventanas, Mobiliario…). Es lo que lee la ventana
      Biblioteca de la aplicación. Carpeta por defecto: ./out/biblioteca

  archisim biblioteca --ampliada [carpeta]
      Escribe la biblioteca ampliada (más de ochenta árboles, arbustos,
      mobiliario urbano, personas y vehículos) en la carpeta, por defecto
      ./biblioteca-ampliada. Se copia dentro de la carpeta de la
      biblioteca de la aplicación.

  archisim muestrario [carpeta]
      Un muro con todas las puertas y ventanas de la plantilla, para verlas
      en planta, en 3D y en IFC: muestrario.archisim, muestrario.ifc y
      muestrario.svg. Carpeta por defecto: ./out

  archisim cte <proyecto.archisim> [informe.csv]
      Comprueba el proyecto con el Código Técnico: ocupación y evacuación
      (DB-SI 3), accesibilidad (DB-SUA 9), escaleras y rampas (DB-SUA 1) y
      superficies, según el uso del edificio guardado en el proyecto.
      Escribe el informe en pantalla y, si se da, en CSV.

  archisim medir [n] [carpeta]
      Repite la vivienda de prueba en una rejilla de n × n (20 por defecto:
      400 viviendas, 2800 muros) y mide cuánto tarda cada trabajo: planta,
      3D, varita mágica, guardar y abrir, exportar e importar el IFC. Deja
      grande.archisim y grande.ifc en la carpeta (./out por defecto).

  archisim png <dibujo.svg> [imagen.png] [ppp]
      Convierte una planta o un plano en SVG en una imagen PNG, a 200
      puntos por pulgada si no se dice otra cosa.

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
        Some("png") => match args.get(1) {
            Some(svg) => {
                let svg = PathBuf::from(svg);
                let png = args
                    .get(2)
                    .map(PathBuf::from)
                    .unwrap_or_else(|| svg.with_extension("png"));
                let dpi: f32 = args.get(3).and_then(|d| d.parse().ok()).unwrap_or(200.0);
                report(svg_to_png(&svg, &png, dpi))
            }
            None => {
                eprintln!("Falta el dibujo: archisim png <dibujo.svg> [imagen.png] [ppp]");
                ExitCode::FAILURE
            }
        },
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
        Some("muestrario") => {
            let dir = PathBuf::from(args.get(1).map(String::as_str).unwrap_or("out"));
            report(muestrario::write(&dir))
        }
        Some("biblioteca") if args.iter().any(|a| a == "--ampliada") => {
            let rest: Vec<&String> = args[1..].iter().filter(|a| *a != "--ampliada").collect();
            let dir = PathBuf::from(rest.first().map_or("biblioteca-ampliada", |s| s.as_str()));
            report(run_extended_library(&dir))
        }
        Some("biblioteca") => {
            let dir = PathBuf::from(args.get(1).map(String::as_str).unwrap_or("out/biblioteca"));
            report(run_library(&dir, args.get(2).map(Path::new)))
        }
        Some("cte") => match args.get(1) {
            Some(project) => report(run_cte(Path::new(project), args.get(2).map(Path::new))),
            None => {
                eprintln!("Falta el proyecto: archisim cte <proyecto.archisim> [informe.csv]");
                ExitCode::FAILURE
            }
        },
        Some("medir") => {
            let n = args.get(1).and_then(|a| a.parse().ok()).unwrap_or(20);
            let dir = PathBuf::from(args.get(2).map(String::as_str).unwrap_or("out"));
            report(
                std::fs::create_dir_all(&dir)
                    .map_err(Into::into)
                    .and_then(|()| grande::measure(n, &dir)),
            )
        }
        Some("--version") | Some("-V") => {
            println!("archisim {}", env!("CARGO_PKG_VERSION"));
            ExitCode::SUCCESS
        }
        Some("--help") | Some("-h") | None => {
            print!("{HELP}\n{}", colaboracion::HELP);
            ExitCode::SUCCESS
        }
        Some(other) => colaboracion::run(&args).unwrap_or_else(|| {
            eprintln!("Orden desconocida: {other}\n\n{HELP}");
            ExitCode::FAILURE
        }),
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
    export_all(doc.model(), dir, "demo")?;
    // The house as a module of an urbanisation (I5).
    let master = demo::masterplan(&project)?;
    let master_path = dir.join(format!("demo-masterplan.{}", archisim_file::EXTENSION));
    archisim_file::save(&master, &master_path)?;
    let ifc_path = dir.join("demo-masterplan.ifc");
    std::fs::write(&ifc_path, ifc_text(&master, "demo-masterplan", None))?;
    std::fs::write(
        dir.join("demo-masterplan-ifc4.ifc"),
        ifc_text_in(&master, "demo-masterplan", None, archisim_ifc::Schema::Ifc4),
    )?;
    println!(
        "  Urbanización con {} copias de la vivienda: {} y {}",
        master.modules().count(),
        master_path.display(),
        ifc_path.display()
    );
    Ok(())
}

fn run_extended_library(dir: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let mut model = archisim_core::Model::new("Biblioteca ampliada");
    model.add_level("Planta baja", 0.0);
    let n = archisim_library::extended::add_extended_library(&mut model)?;
    let files = write_library_files(&model, dir)?;
    println!(
        "Biblioteca ampliada: {n} tipos en {files} ficheros, en {}",
        dir.display()
    );
    Ok(())
}

/// Writes the family types of a model as library files in folders by
/// category; returns how many files.
fn write_library_files(
    model: &archisim_core::Model,
    dir: &Path,
) -> Result<usize, Box<dyn std::error::Error>> {
    let mut n = 0;
    for (category, family, text) in model.catalog_files() {
        let folder = library_folder(&category);
        let sub = dir.join(folder);
        std::fs::create_dir_all(&sub)?;
        let path = sub.join(format!("{}.asfam", family.replace(['/', '\\', ':'], "-")));
        std::fs::write(&path, text)?;
        println!("  {}", path.display());
        n += 1;
    }
    Ok(n)
}

fn library_folder(category: &str) -> &'static str {
    match category {
        "door" => "Puertas",
        "window" => "Ventanas",
        "void" => "Huecos",
        "furniture" => "Mobiliario",
        "sanitary" => "Sanitarios",
        "kitchen" => "Cocina",
        "site" => "Entorno",
        "street" => "Mobiliario urbano",
        "people" => "Personas",
        "vehicle" => "Vehículos",
        _ => "Otros",
    }
}

fn run_library(dir: &Path, project: Option<&Path>) -> Result<(), Box<dyn std::error::Error>> {
    let model = match project {
        Some(p) => {
            archisim_file::open(p).map_err(|e| format!("no se puede abrir {}: {e}", p.display()))?
        }
        None => archisim_library::new_project("Plantilla"),
    };
    let n = write_library_files(&model, dir)?;
    // Wall and slab types, one file for each.
    for (file, walls) in [("Muros.astipos", true), ("Forjados.astipos", false)] {
        let mut cat = archisim_core::LayeredCatalog::default();
        let refs: Vec<archisim_core::LayeredRef> = if walls {
            model
                .wall_types()
                .map(|t| archisim_core::LayeredRef::Wall(t.id))
                .collect()
        } else {
            model
                .slab_types()
                .map(|t| archisim_core::LayeredRef::Slab(t.id))
                .collect()
        };
        for r in refs {
            let (entry, materials) = model.layered_entry(r)?;
            for m in materials {
                if !cat.materials.iter().any(|x| x.name == m.name) {
                    cat.materials.push(m);
                }
            }
            cat.types.push(entry);
        }
        let sub = dir.join("Multicapas");
        std::fs::create_dir_all(&sub)?;
        let path = sub.join(file);
        std::fs::write(&path, archisim_core::layered_catalog_text(&cat))?;
        println!("  {} ({} tipos)", path.display(), cat.types.len());
    }
    println!(
        "{n} familias con sus tipos y los tipos de muro y forjado en {}",
        dir.display()
    );
    Ok(())
}

fn run_cte(project: &Path, csv: Option<&Path>) -> Result<(), Box<dyn std::error::Error>> {
    let model = archisim_file::open(project)
        .map_err(|e| format!("no se puede abrir {}: {e}", project.display()))?;
    let r = archisim_cte::check(&model);
    print!("{}", r.to_text(&model));
    if let Some(csv) = csv {
        std::fs::write(csv, r.to_csv(&model))?;
        println!("\nInforme CSV: {}", csv.display());
    }
    Ok(())
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

/// IFC 4.3 file of a model, named after the project file.
fn ifc_text(model: &Model, stem: &str, timestamp: Option<String>) -> String {
    ifc_text_in(model, stem, timestamp, archisim_ifc::Schema::Ifc4x3)
}

/// IFC file of a model in a schema.
fn ifc_text_in(
    model: &Model,
    stem: &str,
    timestamp: Option<String>,
    schema: archisim_ifc::Schema,
) -> String {
    export_ifc(
        model,
        &ExportOptions {
            file_name: format!("{stem}.ifc"),
            author: "ArchiSim".into(),
            organization: "AD5".into(),
            timestamp,
            schema,
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
    // Views and exports show the copies of the modules as elements.
    let flat = model.flattened()?;
    let model = &flat;
    let ifc_path = dir.join(format!("{stem}.ifc"));
    std::fs::write(&ifc_path, ifc_text(model, stem, None))?;
    println!("  IFC 4.3: {}", ifc_path.display());
    // The same in IFC4, for programs that do not read 4.3 yet.
    let ifc4_path = dir.join(format!("{stem}-ifc4.ifc"));
    std::fs::write(
        &ifc4_path,
        ifc_text_in(model, stem, None, archisim_ifc::Schema::Ifc4),
    )?;
    println!("  IFC4:    {}", ifc4_path.display());
    let cte_path = dir.join(format!("{stem}-cte.txt"));
    std::fs::write(&cte_path, archisim_cte::check(model).to_text(model))?;
    println!("  Comprobación CTE: {}", cte_path.display());
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
        let dxf_path = dir.join(file.replace(".svg", ".dxf"));
        std::fs::write(
            &dxf_path,
            archisim_export::dxf::plan_dxf(model, level, &PlanOptions::default())?,
        )?;
        println!(
            "  Planta «{name}» en DXF (mm, 1:50): {}",
            dxf_path.display()
        );
    }
    for sheet in model.sheets() {
        let path = dir.join(format!("{stem}-plano-{}.svg", slug(&sheet.number)));
        let svg = archisim_plan::sheet::render_sheet(
            model,
            sheet.id,
            &archisim_plan::sheet::SheetOptions::default(),
        );
        std::fs::write(&path, svg)?;
        println!(
            "  Plano {} «{}» ({}): {}",
            sheet.number,
            sheet.name,
            sheet.size_name(),
            path.display()
        );
        if sheet.presentation.is_some() {
            // Presentation boards also as an image, for screens and the web.
            let png = dir.join(format!("{stem}-plano-{}.png", slug(&sheet.number)));
            let svg = std::fs::read_to_string(&path)?;
            std::fs::write(&png, archisim_export::png::png(&svg, 200.0)?)?;
            println!(
                "  Plano {} en PNG (200 ppp): {}",
                sheet.number,
                png.display()
            );
        }
        let dxf = dir.join(format!("{stem}-plano-{}.dxf", slug(&sheet.number)));
        std::fs::write(&dxf, archisim_export::dxf::sheet_dxf(model, sheet.id)?)?;
        println!(
            "  Plano {} en DXF (mm de papel): {}",
            sheet.number,
            dxf.display()
        );
    }
    // Schedules as spreadsheets, and the measurements as a budget.
    for kind in archisim_core::ScheduleKind::ALL {
        let t = archisim_plan::schedules::table(model, kind, None);
        if t.rows.is_empty() {
            continue;
        }
        let path = dir.join(format!("{stem}-tabla-{}.csv", kind.code()));
        std::fs::write(&path, t.csv())?;
        println!("  Tabla «{}»: {}", kind.label(), path.display());
    }
    let bc3 = dir.join(format!("{stem}.bc3"));
    let date = archisim_export::bc3::bc3_date(&archisim_ifc::utc_now());
    std::fs::write(
        &bc3,
        archisim_export::bc3::bc3(model, &date, env!("CARGO_PKG_VERSION")),
    )?;
    println!("  Mediciones en BC3 (FIEBDC-3): {}", bc3.display());
    if !model.sheets().is_empty() {
        let pdf = dir.join(format!("{stem}-planos.pdf"));
        std::fs::write(&pdf, sheets_pdf(model)?)?;
        println!("  Planos en PDF: {}", pdf.display());
    }
    for sec in model.sections() {
        let file = format!("{stem}-{}.svg", slug(&sec.title()));
        let path = dir.join(&file);
        let drawing = archisim_plan::section::section_drawing(model, sec);
        let options = PlanOptions {
            scale: 100.0,
            ..PlanOptions::default()
        };
        std::fs::write(
            &path,
            archisim_plan::section::render_section(model, &drawing, &options),
        )?;
        println!("  {} 1:100: {}", sec.title(), path.display());
    }
    Ok(())
}

/// Every sheet of the project, a page each, in order.
fn sheets_pdf(model: &Model) -> Result<Vec<u8>, String> {
    let svgs: Vec<(String, String)> = model
        .sheets()
        .iter()
        .map(|s| {
            (
                archisim_plan::sheet::render_sheet(model, s.id, &Default::default()),
                format!("{} {}", s.number, s.name),
            )
        })
        .collect();
    let pages: Vec<archisim_export::pdf::Page> = svgs
        .iter()
        .map(|(svg, title)| archisim_export::pdf::Page { svg, title })
        .collect();
    archisim_export::pdf::pdf(&pages, &model.project_name)
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

/// Rasterizes an SVG drawing to a PNG at `dpi` dots per inch.
fn svg_to_png(svg: &Path, png: &Path, dpi: f32) -> Result<(), Box<dyn std::error::Error>> {
    let text = std::fs::read_to_string(svg).map_err(|e| format!("{}: {e}", svg.display()))?;
    let tree = resvg::usvg::Tree::from_str(&text, &resvg::usvg::Options::default())
        .map_err(|e| format!("{}: {e}", svg.display()))?;
    // usvg sizes are in CSS pixels (96 per inch).
    let k = dpi / 96.0;
    let size = tree.size();
    let w = (size.width() * k).ceil().max(1.0) as u32;
    let h = (size.height() * k).ceil().max(1.0) as u32;
    let mut pixmap = resvg::tiny_skia::Pixmap::new(w, h).ok_or("imagen demasiado grande")?;
    pixmap.fill(resvg::tiny_skia::Color::WHITE);
    resvg::render(
        &tree,
        resvg::tiny_skia::Transform::from_scale(k, k),
        &mut pixmap.as_mut(),
    );
    pixmap
        .save_png(png)
        .map_err(|e| format!("{}: {e}", png.display()))?;
    println!("{} ({w} × {h} px)", png.display());
    Ok(())
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
