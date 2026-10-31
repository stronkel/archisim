//! A large model to measure ArchiSim with ("archisim medir"): the test
//! house repeated on an n × n grid (n = 20: 400 houses, 2800 walls, 4000
//! doors, windows and voids, 800 slabs), and the time each job takes on it:
//! the plan (whole and as the screen draws it), the 3D solids, snapping and
//! picking under the cursor, saving and opening the project, and exporting
//! and importing the IFC.

use std::path::Path;
use std::time::Instant;

use archisim_core::{Model, ModelError, NewOpening, NewSlab, NewWall, Point2};
use archisim_ifc::{export_ifc, import_ifc, ExportOptions};
use archisim_plan::{render_plan, PlanOptions};

/// Distance between the houses of the grid, in metres.
const STEP: (f64, f64) = (14.0, 11.0);

/// The test house repeated on an `n` × `n` grid.
pub fn build(n: usize) -> Result<Model, ModelError> {
    let demo = crate::demo::build()?;
    let base = demo.model().clone();
    let mut m = base.clone();
    let walls: Vec<_> = base.walls().cloned().collect();
    let slabs: Vec<_> = base.slabs().cloned().collect();
    for i in 0..n {
        for j in 0..n {
            if i == 0 && j == 0 {
                continue;
            }
            let d = Point2::new(i as f64 * STEP.0, j as f64 * STEP.1);
            for w in &walls {
                let id = m.add_wall(NewWall {
                    wall_type: w.wall_type,
                    level: w.level,
                    start: w.start + d,
                    end: w.end + d,
                    bulge: w.bulge,
                    reference: w.reference,
                    height: w.height,
                    base_offset: w.base_offset,
                    top: w.top,
                })?;
                for o in base.openings_in_wall(w.id) {
                    m.add_opening(NewOpening {
                        opening_type: o.opening_type,
                        wall: id,
                        along: o.along,
                        sill: o.sill,
                        opens_to: o.opens_to,
                        hand: o.hand,
                        setback: o.setback,
                    })?;
                }
            }
            for s in &slabs {
                m.add_slab(NewSlab {
                    slab_type: s.slab_type,
                    level: s.level,
                    outline: s.outline.iter().map(|p| *p + d).collect(),
                    top_offset: s.top_offset,
                })?;
            }
        }
    }
    m.project_name = format!("Prueba de rendimiento {n} × {n}");
    Ok(m)
}

/// Runs `f` and prints how long it took.
fn timed<T>(what: &str, f: impl FnOnce() -> T) -> T {
    let t = Instant::now();
    let out = f();
    let ms = t.elapsed().as_secs_f64() * 1000.0;
    println!("  {what:<52} {ms:>10.1} ms");
    out
}

/// Draws `svg` into a `w` × `h` image, as the screen does.
fn rasterise(svg: &str, w: u32, h: u32) -> resvg::tiny_skia::Pixmap {
    let mut pixmap = resvg::tiny_skia::Pixmap::new(w.max(1), h.max(1)).expect("image");
    if let Ok(tree) = resvg::usvg::Tree::from_str(svg, &resvg::usvg::Options::default()) {
        let s = tree.size();
        let t =
            resvg::tiny_skia::Transform::from_scale(w as f32 / s.width(), h as f32 / s.height());
        resvg::render(&tree, t, &mut pixmap.as_mut());
    }
    pixmap
}

/// Measures what the screen does with the views of a project
/// ("archisim medir vistas proyecto.archisim"): every section and
/// elevation (working it out, and its picture of the whole and of a
/// close-up, as when moving around in it) and each plan.
pub fn measure_views(path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let m = timed("Abrir el proyecto", || archisim_file::open(path))?;
    // A 1600 × 1000 px view, as the application draws it.
    let (w, h) = (1600u32, 1000u32);
    let picture = |options: &PlanOptions, svg: &dyn Fn(&PlanOptions) -> String, name: &str| {
        let (lo, hi) = options.viewport.unwrap_or_default();
        let px_per_mm = w as f64 / (hi.x - lo.x) * options.scale / 1000.0;
        let options = PlanOptions {
            px_per_mm: Some(px_per_mm),
            ..options.clone()
        };
        let t = Instant::now();
        let s = svg(&options);
        let svg_ms = t.elapsed().as_secs_f64() * 1000.0;
        let t = Instant::now();
        let pixmap = rasterise(&s, w, h);
        let raster_ms = t.elapsed().as_secs_f64() * 1000.0;
        println!(
            "  {name:<40} dibujo {svg_ms:>8.1} ms · imagen {raster_ms:>8.1} ms · {:>6.2} MB",
            s.len() as f64 / 1e6
        );
        if let Some(dir) = std::env::var_os("ARCHISIM_MEDIR_IMAGENES") {
            let file = name.replace([' ', ',', '/'], "-");
            let _ = pixmap.save_png(Path::new(&dir).join(format!("{file}.png")));
        }
    };
    // A view of the region lo–hi with the window's proportions.
    let framed = |lo: Point2, hi: Point2| {
        let c = (lo + hi) * 0.5;
        let half_w = ((hi.x - lo.x) / 2.0).max((hi.y - lo.y) / 2.0 * w as f64 / h as f64);
        let half_h = half_w * h as f64 / w as f64;
        (
            c - Point2::new(half_w, half_h),
            c + Point2::new(half_w, half_h),
        )
    };
    for sec in m.sections() {
        let d = timed(&format!("{}: calcularlo", sec.title()), || {
            archisim_plan::section::section_drawing(&m, sec)
        });
        let render = |o: &PlanOptions| archisim_plan::section::render_section(&m, &d, o);
        let all = PlanOptions {
            viewport: Some(framed(d.lo, d.hi)),
            ..PlanOptions::default()
        };
        picture(&all, &render, &format!("{} entero", sec.title()));
        let mid = (d.lo + d.hi) * 0.5;
        let close = PlanOptions {
            viewport: Some(framed(
                mid - Point2::new(4.0, 2.5),
                mid + Point2::new(4.0, 2.5),
            )),
            ..PlanOptions::default()
        };
        picture(&close, &render, &format!("{} de cerca", sec.title()));
    }
    for level in m.levels() {
        let (lo, hi) = archisim_plan::sheet::level_extent(&m, level.id, 50.0)
            .unwrap_or((Point2::new(-10.0, -10.0), Point2::new(30.0, 30.0)));
        let render = |o: &PlanOptions| render_plan(&m, level.id, o);
        let all = PlanOptions {
            viewport: Some(framed(lo, hi)),
            ..PlanOptions::default()
        };
        picture(&all, &render, &format!("Planta {} entera", level.name));
    }
    Ok(())
}

/// Measures the jobs on a project of its own (`archisim medir proyecto
/// <fichero>`): open and save it, its 3D, the IFC both ways, the code
/// check, and then every view as `measure_views` does.
pub fn measure_project(path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let m = timed("Abrir el proyecto", || archisim_file::open(path))?;
    println!(
        "  {} muros, {} forjados, {} huecos, {} objetos, {} locales",
        m.walls().count(),
        m.slabs().count(),
        m.openings().count(),
        m.objects().count(),
        m.rooms().count()
    );
    let flat = timed("Modelo con los módulos desplegados", || m.flattened())?;
    let solids = timed("Sólidos 3D de todo el modelo", || {
        archisim_geom::solid::model_solids(&flat)
    });
    let triangles: usize = solids.iter().map(|s| s.triangles.len()).sum();
    println!("  {:<52} {:>10}", "  triángulos", triangles);
    let out = std::env::temp_dir().join("archisim-medir.archisim");
    timed("Guardar el proyecto (.archisim)", || {
        archisim_file::save(&m, &out)
    })?;
    let ifc = timed("Exportar el IFC", || {
        export_ifc(
            &flat,
            &ExportOptions {
                timestamp: Some("2026-10-18T12:00:00".into()),
                ..Default::default()
            },
        )
    });
    println!(
        "  {:<52} {:>10.1} MB",
        "  tamaño del IFC",
        ifc.len() as f64 / 1e6
    );
    timed("Importar el IFC", || import_ifc(&ifc))?;
    timed("Comprobación CTE", || archisim_cte::check(&flat));
    timed("Mediciones", || {
        archisim_plan::schedules::measurements(&flat)
    });
    measure_views(path)
}

/// Builds the grid, measures every job and leaves the files in `dir`
/// (grande.archisim and grande.ifc, to open them in the application).
pub fn measure(n: usize, dir: &Path) -> Result<(), Box<dyn std::error::Error>> {
    println!("Rejilla de {n} × {n} viviendas de prueba:");
    let m = timed("Crear el modelo", || build(n))?;
    let level = m.levels()[0].id;
    println!(
        "  {} muros, {} forjados, {} puertas, ventanas y huecos",
        m.walls().count(),
        m.slabs().count(),
        m.openings().count()
    );
    let svg = timed("Planta baja entera, 1:50 (SVG)", || {
        render_plan(&m, level, &PlanOptions::default())
    });
    println!(
        "  {:<52} {:>10.1} MB",
        "  tamaño del SVG",
        svg.len() as f64 / 1e6
    );
    // What the screen draws: a view about 30 × 20 m wide.
    let mid = Point2::new(n as f64 * STEP.0 / 2.0, n as f64 * STEP.1 / 2.0);
    let view = PlanOptions {
        viewport: Some((mid - Point2::new(15.0, 10.0), mid + Point2::new(15.0, 10.0))),
        ..PlanOptions::default()
    };
    let screen = |options: &PlanOptions, name: &str| {
        // A 1600 × 1000 px view, as the application draws it.
        let (lo, hi) = options.viewport.unwrap_or_default();
        let w = 1600u32;
        let h = ((hi.y - lo.y) / (hi.x - lo.x) * w as f64).max(1.0) as u32;
        let px_per_mm = w as f64 / (hi.x - lo.x) * options.scale / 1000.0;
        let options = PlanOptions {
            px_per_mm: Some(px_per_mm),
            ..options.clone()
        };
        let t = Instant::now();
        let svg = render_plan(&m, level, &options);
        let svg_ms = t.elapsed().as_secs_f64() * 1000.0;
        let mut pixmap = resvg::tiny_skia::Pixmap::new(w, h).expect("image");
        if let Ok(tree) = resvg::usvg::Tree::from_str(&svg, &resvg::usvg::Options::default()) {
            let s = tree.size();
            let t = resvg::tiny_skia::Transform::from_scale(
                w as f32 / s.width(),
                h as f32 / s.height(),
            );
            resvg::render(&tree, t, &mut pixmap.as_mut());
        }
        if std::env::var_os("ARCHISIM_MEDIR_IMAGENES").is_some() {
            // To look at what the screen would show.
            let _ = pixmap.save_png(dir.join(format!("{name}.png")));
        }
        (svg.len(), svg_ms)
    };
    let (_, svg_ms) = timed(
        "Planta en pantalla, zona de 30 × 20 m (con imagen)",
        || screen(&view, "pantalla-zona"),
    );
    println!("  {:<52} {svg_ms:>10.1} ms", "  de ello, el dibujo (SVG)");
    let zoomed_out = PlanOptions {
        viewport: Some((Point2::new(-5.0, -5.0), mid * 2.0 + Point2::new(5.0, 5.0))),
        ..PlanOptions::default()
    };
    let (len, svg_ms) = timed("Planta en pantalla, todo a la vista (con imagen)", || {
        screen(&zoomed_out, "pantalla-todo")
    });
    println!("  {:<52} {svg_ms:>10.1} ms", "  de ello, el dibujo (SVG)");
    println!("  {:<52} {:>10.1} MB", "  tamaño del SVG", len as f64 / 1e6);
    let solids = timed("Sólidos 3D de todo el modelo", || {
        archisim_geom::solid::model_solids(&m)
    });
    let triangles: usize = solids.iter().map(|s| s.triangles.len()).sum();
    println!("  {:<52} {:>10}", "  triángulos", triangles);
    timed("Geometría de un muro (uniones)", || {
        let w = m.walls().nth(m.walls().count() / 2).map(|w| w.id);
        w.and_then(|w| archisim_geom::wall_geometry(&m, w))
    });
    let room = mid + Point2::new(-STEP.0 / 2.0 + 3.0, 3.0);
    let _ = timed("Varita mágica en un recinto", || {
        archisim_geom::room_outline(&m, level, room, archisim_geom::Contour::InteriorFace)
    });
    let path = dir.join(format!("grande.{}", archisim_file::EXTENSION));
    timed("Guardar el proyecto (.archisim)", || {
        archisim_file::save(&m, &path)
    })?;
    let size = std::fs::metadata(&path).map_or(0, |f| f.len());
    println!(
        "  {:<52} {:>10.1} MB",
        "  tamaño del proyecto",
        size as f64 / 1e6
    );
    timed("Abrir el proyecto", || archisim_file::open(&path))?;
    let ifc = timed("Exportar el IFC", || {
        export_ifc(
            &m,
            &ExportOptions {
                timestamp: Some("2026-10-18T12:00:00".into()),
                ..Default::default()
            },
        )
    });
    println!(
        "  {:<52} {:>10.1} MB",
        "  tamaño del IFC",
        ifc.len() as f64 / 1e6
    );
    std::fs::write(dir.join("grande.ifc"), &ifc)?;
    let imported = timed("Importar el IFC", || import_ifc(&ifc))?;
    println!(
        "  {:<52} {:>10}",
        "  muros importados",
        imported.model.walls().count()
    );
    Ok(())
}
