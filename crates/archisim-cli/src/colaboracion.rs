//! Commands of phase 4 (collaboration): checking requirements (IDS),
//! comparing versions of a model, clash detection and BCF issues.
//! More commands join here as they are built, so that `main.rs` only
//! hands over the ones it does not know.

use std::path::Path;
use std::process::ExitCode;

pub const HELP: &str = "\
Colaboración (un modelo puede ser un proyecto .archisim, que se usa tal
como sale en su IFC, o un IFC de cualquier programa):
  archisim comprobar-ids <modelo> <requisitos.ids> [incidencias.bcf]
      Comprueba el modelo contra los requisitos de información del cliente
      (IDS 1.0 de buildingSMART): qué especificaciones cumple y qué
      elementos fallan, y por qué. Con un .bcf, deja una incidencia por
      especificación incumplida con sus elementos seleccionados. Termina
      con error si alguna no se cumple.

  archisim comparar <version-anterior> <version-nueva>
      Elementos nuevos, borrados, movidos, que cambian de forma o de datos
      entre dos versiones del mismo modelo, por su GlobalId.

  archisim interferencias <modelo> <vinculado.ifc>... [--tolerancia mm] [--bcf incidencias.bcf]
      Choques entre el modelo y cada modelo vinculado (estructura,
      instalaciones…), sin contar contactos ni lo que pasa por un hueco.
      Tolerancia por defecto: 10 mm. Con --bcf, una incidencia por choque;
      si el fichero ya existe se actualiza: las que siguen reciben un
      comentario, las que ya no aparecen pasan a «Resuelta».

  archisim bcf <incidencias.bcf>
      Lista las incidencias de un fichero BCF (2.1 o 3.0).

  archisim demo-coordinacion [carpeta]
      Junto a la vivienda de prueba (demo.archisim), un modelo de
      estructura (demo-estructura.ifc: pilares y vigas, IFC en milímetros
      como lo exporta Revit) y otro de instalaciones (demo-instalaciones.ifc:
      un conducto y una bajante), con interferencias a propósito, y unos
      requisitos IDS de ejemplo (demo-requisitos.ids). Carpeta: ./out
";

/// Runs a collaboration command, or `None` if `args` is not one.
pub fn run(args: &[String]) -> Option<ExitCode> {
    let a = |i: usize| args.get(i).map(String::as_str);
    match a(0)? {
        "comprobar-ids" | "ids" => Some(match (a(1), a(2)) {
            (Some(model), Some(ids)) => {
                match check_ids(Path::new(model), Path::new(ids), a(3).map(Path::new)) {
                    Ok(true) => ExitCode::SUCCESS,
                    Ok(false) => ExitCode::from(1),
                    Err(e) => {
                        eprintln!("Error: {e}");
                        ExitCode::from(2)
                    }
                }
            }
            _ => {
                eprintln!(
                    "Uso: archisim comprobar-ids <modelo.archisim|modelo.ifc> <requisitos.ids>"
                );
                ExitCode::from(2)
            }
        }),
        "comparar" => Some(match (a(1), a(2)) {
            (Some(old), Some(new)) => done(compare_versions(Path::new(old), Path::new(new))),
            _ => usage("archisim comparar <version-anterior> <version-nueva>"),
        }),
        "interferencias" | "clash" => {
            let mut models = Vec::new();
            let mut tolerance = 0.01;
            let mut bcf = None;
            let mut i = 1;
            while let Some(arg) = a(i) {
                match arg {
                    "--tolerancia" => {
                        i += 1;
                        match a(i).and_then(|v| v.replace(',', ".").parse::<f64>().ok()) {
                            Some(mm) if mm >= 0.0 => tolerance = mm / 1000.0,
                            _ => return Some(usage("--tolerancia <milímetros>")),
                        }
                    }
                    "--bcf" => {
                        i += 1;
                        bcf = a(i).map(Path::new);
                    }
                    m => models.push(Path::new(m)),
                }
                i += 1;
            }
            Some(if models.len() < 2 {
                usage("archisim interferencias <modelo> <vinculado.ifc>... [--tolerancia mm] [--bcf incidencias.bcf]")
            } else {
                match clashes(models[0], &models[1..], tolerance, bcf) {
                    Ok(0) => ExitCode::SUCCESS,
                    Ok(_) => ExitCode::from(1),
                    Err(e) => {
                        eprintln!("Error: {e}");
                        ExitCode::from(2)
                    }
                }
            })
        }
        "demo-coordinacion" => Some(done(demo_files(Path::new(a(1).unwrap_or("out"))))),
        "bcf" => Some(match a(1) {
            Some(f) => done(list_bcf(Path::new(f))),
            None => usage("archisim bcf <incidencias.bcf>"),
        }),
        _ => None,
    }
}

fn usage(u: &str) -> ExitCode {
    eprintln!("Uso: {u}");
    ExitCode::from(2)
}

fn done(r: Result<(), Box<dyn std::error::Error>>) -> ExitCode {
    match r {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("Error: {e}");
            ExitCode::from(2)
        }
    }
}

type R<T> = Result<T, Box<dyn std::error::Error>>;

/// Who signs the issues: the person's name if the system knows it.
fn author() -> archisim_coord::Author {
    let user = std::env::var("ARCHISIM_AUTOR")
        .or_else(|_| std::env::var("USER"))
        .or_else(|_| std::env::var("USERNAME"))
        .unwrap_or_else(|_| "ArchiSim".into());
    archisim_coord::Author(user)
}

fn file_ref(path: &Path) -> archisim_bcf::FileRef {
    archisim_bcf::FileRef {
        ifc_project: None,
        ifc_spatial_element: None,
        is_external: true,
        filename: path.file_name().map(|n| n.to_string_lossy().into_owned()),
        date: Some(archisim_bcf::now()),
        reference: None,
    }
}

fn load(path: &Path) -> R<archisim_coord::ModelElements> {
    let text = model_ifc(path)?;
    Ok(archisim_coord::read_elements(&text).map_err(|e| format!("{}: {e}", path.display()))?)
}

fn describe(path: &Path, m: &archisim_coord::ModelElements) {
    println!(
        "  {}: {} elementos ({}{})",
        path.display(),
        m.elements.len(),
        m.main_discipline().name(),
        if m.application.is_empty() {
            String::new()
        } else {
            format!(", {}", m.application)
        }
    );
    if !m.without_body.is_empty() {
        let list: Vec<String> = m
            .without_body
            .iter()
            .map(|(c, n)| format!("{n} {}", archisim_ids::model::class_display(c)))
            .collect();
        println!("    sin geometría legible: {}", list.join(", "));
    }
    if m.approximate > 0 {
        println!(
            "    {} con geometría aproximada (recortes, superficies curvas o abiertas)",
            m.approximate
        );
    }
}

fn compare_versions(old: &Path, new: &Path) -> R<()> {
    let a = load(old)?;
    let b = load(new)?;
    println!("Comparación de versiones por GlobalId:");
    describe(old, &a);
    describe(new, &b);
    print!("\n{}", archisim_coord::compare(&a, &b).summary());
    Ok(())
}

fn clashes(model: &Path, links: &[&Path], tolerance: f64, bcf_path: Option<&Path>) -> R<usize> {
    let me = load(model)?;
    println!("Interferencias (tolerancia {:.0} mm):", tolerance * 1000.0);
    describe(model, &me);
    let mut bcf = match bcf_path {
        Some(p) if p.exists() => archisim_bcf::read_bcf(&std::fs::read(p)?)?,
        _ => archisim_bcf::Bcf::default(),
    };
    let opts = archisim_coord::ClashOptions {
        tolerance,
        ..Default::default()
    };
    let mut total = 0;
    let (mut added, mut kept, mut resolved) = (0, 0, 0);
    for link in links {
        let other = load(link)?;
        describe(link, &other);
        let found = archisim_coord::find_clashes(&me, &other, &opts);
        for c in &found {
            let (x, y) = (&me.elements[c.a], &other.elements[c.b]);
            println!(
                "    ✘ {} × {}: {} ({}) en X {:.2}, Y {:.2}, Z {:.2}",
                label(x),
                label(y),
                archisim_coord::versions::metres(c.depth),
                x.storey
                    .as_deref()
                    .or(y.storey.as_deref())
                    .unwrap_or("sin planta"),
                c.point[0],
                c.point[1],
                c.point[2]
            );
        }
        if found.is_empty() {
            println!("    ✔ sin interferencias");
        }
        total += found.len();
        if bcf_path.is_some() {
            let files = [file_ref(model), file_ref(link)];
            let (a, k, r) =
                archisim_coord::clashes_to_bcf(&mut bcf, &found, &me, &other, &files, &author());
            added += a;
            kept += k;
            resolved += r;
        }
    }
    if let Some(p) = bcf_path {
        std::fs::write(p, archisim_bcf::write_bcf(&bcf))?;
        println!(
            "\nIncidencias en {}: {added} nuevas, {kept} siguen, {resolved} resueltas.",
            p.display()
        );
    }
    println!("\n{total} interferencias.");
    Ok(total)
}

fn label(e: &archisim_coord::Element) -> String {
    let name = if e.name.is_empty() {
        &e.type_name
    } else {
        &e.name
    };
    format!("{} «{name}»", archisim_ids::model::class_display(&e.class))
}

fn list_bcf(path: &Path) -> R<()> {
    let bcf = archisim_bcf::read_bcf(&std::fs::read(path)?)?;
    println!(
        "{} — BCF {}, {} incidencias",
        path.display(),
        bcf.version.id(),
        bcf.topics.len()
    );
    for t in &bcf.topics {
        println!(
            "  {} [{}] {}{}",
            t.index.map(|i| format!("#{i}")).unwrap_or_default(),
            t.status.as_deref().unwrap_or("?"),
            t.title,
            t.assigned_to
                .as_deref()
                .map(|a| format!(" → {a}"))
                .unwrap_or_default()
        );
        let vp = t.viewpoints.first();
        println!(
            "      {} · {} comentarios · {} vistas{}",
            t.topic_type.as_deref().unwrap_or("sin tipo"),
            t.comments.len(),
            t.viewpoints.len(),
            vp.map(|v| format!(", {} elementos seleccionados", v.selection.len()))
                .unwrap_or_default()
        );
    }
    Ok(())
}

/// A box element for the example models: class, GlobalId, name, lower
/// and upper corner in metres.
struct Piece(&'static str, &'static str, &'static str, [f64; 3], [f64; 3]);

/// A small IFC4 file in millimetres (as Revit writes them) with one storey
/// and the given boxes, and pipes (GlobalId, name, polyline, radius).
fn example_ifc(
    project: &str,
    app: &str,
    boxes: &[Piece],
    pipes: &[(&str, &str, Vec<[f64; 3]>, f64)],
) -> String {
    let mm = |v: f64| format!("{:.1}", v * 1000.0);
    let mut s = format!(
        "ISO-10303-21;\nHEADER;\nFILE_DESCRIPTION(('ViewDefinition [ReferenceView_V1.2]'),'2;1');\n\
         FILE_NAME('{project}.ifc','2026-10-19T12:00:00',('Ingeniería'),('Ejemplo'),'{app}','{app}','');\n\
         FILE_SCHEMA(('IFC4'));\nENDSEC;\nDATA;\n\
         #1=IFCPROJECT('3Kq8sM1rT0dOoN0r9pG0a1',$,'{project}',$,$,$,$,(#6),#2);\n\
         #2=IFCUNITASSIGNMENT((#3));\n#3=IFCSIUNIT(*,.LENGTHUNIT.,.MILLI.,.METRE.);\n\
         #4=IFCDIRECTION((0.,0.,1.));\n#5=IFCAXIS2PLACEMENT3D(#7,$,$);\n\
         #6=IFCGEOMETRICREPRESENTATIONCONTEXT($,'Model',3,1.E-05,#5,$);\n#7=IFCCARTESIANPOINT((0.,0.,0.));\n\
         #8=IFCLOCALPLACEMENT($,#5);\n\
         #9=IFCSITE('3Kq8sM1rT0dOoN0r9pG0a2',$,'Parcela',$,$,#8,$,$,.ELEMENT.,$,$,$,$,$);\n\
         #10=IFCBUILDING('3Kq8sM1rT0dOoN0r9pG0a3',$,'Vivienda',$,$,#8,$,$,.ELEMENT.,$,$,$);\n\
         #11=IFCBUILDINGSTOREY('3Kq8sM1rT0dOoN0r9pG0a4',$,'Planta baja',$,$,#8,$,$,.ELEMENT.,0.);\n\
         #12=IFCRELAGGREGATES('3Kq8sM1rT0dOoN0r9pG0a5',$,$,$,#1,(#9));\n\
         #13=IFCRELAGGREGATES('3Kq8sM1rT0dOoN0r9pG0a6',$,$,$,#9,(#10));\n\
         #14=IFCRELAGGREGATES('3Kq8sM1rT0dOoN0r9pG0a7',$,$,$,#10,(#11));\n"
    );
    let mut n = 100;
    let mut ids = Vec::new();
    for Piece(class, guid, name, lo, hi) in boxes {
        let b = n;
        n += 10;
        let (w, d, h) = (hi[0] - lo[0], hi[1] - lo[1], hi[2] - lo[2]);
        s += &format!(
            "#{b}=IFCCARTESIANPOINT(({},{},{}));\n#{}=IFCAXIS2PLACEMENT3D(#{b},$,$);\n#{}=IFCLOCALPLACEMENT(#8,#{});\n\
             #{}=IFCCARTESIANPOINT(({},{}));\n#{}=IFCAXIS2PLACEMENT2D(#{},$);\n\
             #{}=IFCRECTANGLEPROFILEDEF(.AREA.,$,#{},{},{});\n#{}=IFCEXTRUDEDAREASOLID(#{},$,#4,{});\n\
             #{}=IFCSHAPEREPRESENTATION(#6,'Body','SweptSolid',(#{}));\n#{}=IFCPRODUCTDEFINITIONSHAPE($,$,(#{}));\n\
             #{}={class}('{guid}',$,'{name}',$,$,#{},#{},$,$);\n",
            mm(lo[0]), mm(lo[1]), mm(lo[2]),
            b + 1, b + 2, b + 1,
            b + 3, mm(w / 2.0), mm(d / 2.0), b + 4, b + 3,
            b + 5, b + 4, mm(w), mm(d), b + 6, b + 5, mm(h),
            b + 7, b + 6, b + 8, b + 7,
            b + 9, b + 2, b + 8,
        );
        ids.push(format!("#{}", b + 9));
    }
    for (guid, name, path, r) in pipes {
        let b = n;
        n += 10 + path.len() as u64;
        let pts: Vec<String> = (0..path.len())
            .map(|i| format!("#{}", b + 10 + i as u64))
            .collect();
        for (i, p) in path.iter().enumerate() {
            s += &format!(
                "#{}=IFCCARTESIANPOINT(({},{},{}));\n",
                b + 10 + i as u64,
                mm(p[0]),
                mm(p[1]),
                mm(p[2])
            );
        }
        s += &format!(
            "#{b}=IFCPOLYLINE(({}));\n#{}=IFCSWEPTDISKSOLID(#{b},{},$,$,$);\n\
             #{}=IFCSHAPEREPRESENTATION(#6,'Body','AdvancedSweptSolid',(#{}));\n#{}=IFCPRODUCTDEFINITIONSHAPE($,$,(#{}));\n\
             #{}=IFCPIPESEGMENT('{guid}',$,'{name}',$,$,#8,#{},$,.RIGIDSEGMENT.);\n",
            pts.join(","),
            b + 1,
            mm(*r),
            b + 2,
            b + 1,
            b + 3,
            b + 2,
            b + 4,
            b + 3
        );
        ids.push(format!("#{}", b + 4));
    }
    s += &format!(
        "#{n}=IFCRELCONTAINEDINSPATIALSTRUCTURE('3Kq8sM1rT0dOoN0r9pG0a8',$,$,$,({}),#11);\nENDSEC;\nEND-ISO-10303-21;\n",
        ids.join(",")
    );
    s
}

/// The example models of `demo-coordinacion`, matching the demo house
/// (12 × 9 m, walls 3 m high, roof slab from 3,00 to 3,55).
pub fn example_models() -> (String, String) {
    let structure = example_ifc(
        "Estructura",
        "Ejemplo de estructura",
        &[
            // Columns: in the façade corners (embedded in the walls, as
            // usual: a clash the architect wants to see) and one in the
            // middle of the hall door, by mistake.
            Piece(
                "IFCCOLUMN",
                "1Pil00000000000000P001",
                "P-1 30×30",
                [0.0, 0.0, -0.35],
                [0.3, 0.3, 3.0],
            ),
            Piece(
                "IFCCOLUMN",
                "1Pil00000000000000P002",
                "P-2 30×30",
                [11.7, 0.0, -0.35],
                [12.0, 0.3, 3.0],
            ),
            Piece(
                "IFCCOLUMN",
                "1Pil00000000000000P003",
                "P-3 30×30",
                [4.65, 3.35, -0.35],
                [4.95, 3.65, 3.0],
            ),
            // A beam whose underside is 10 cm below the partition heads.
            Piece(
                "IFCBEAM",
                "1Vig00000000000000V001",
                "V-1 30×40",
                [0.26, 4.35, 2.6],
                [11.74, 4.65, 3.0],
            ),
            // A beam embedded in the façade (its outer leaf and insulation).
            Piece(
                "IFCBEAM",
                "1Vig00000000000000V002",
                "V-2 26×40",
                [0.3, 0.0, 2.6],
                [11.7, 0.26, 3.0],
            ),
        ],
        &[],
    );
    let services = example_ifc(
        "Instalaciones",
        "Ejemplo de instalaciones",
        &[
            // A duct through the corridor partition, without a hole.
            Piece(
                "IFCDUCTSEGMENT",
                "1Con00000000000000C001",
                "Conducto 40×30",
                [1.0, 5.8, 2.25],
                [11.0, 6.2, 2.55],
            ),
        ],
        &[(
            "1Baj00000000000000B001",
            "Bajante Ø110",
            vec![[11.5, 8.5, -0.6], [11.5, 8.5, 3.8]],
            0.055,
        )],
    );
    (structure, services)
}

fn demo_files(dir: &Path) -> R<()> {
    std::fs::create_dir_all(dir)?;
    let doc = crate::demo::build()?;
    let project = dir.join(format!("demo.{}", archisim_file::EXTENSION));
    archisim_file::save(doc.model(), &project)?;
    let (structure, services) = example_models();
    let s = dir.join("demo-estructura.ifc");
    let i = dir.join("demo-instalaciones.ifc");
    let r = dir.join("demo-requisitos.ids");
    std::fs::write(&s, structure)?;
    std::fs::write(&i, services)?;
    std::fs::write(
        &r,
        include_str!("../../archisim-ids/tests/fixtures/vivienda.ids"),
    )?;
    println!("Ficheros de coordinación de ejemplo:");
    for p in [&project, &s, &i, &r] {
        println!("  {}", p.display());
    }
    println!();
    clashes(&project, &[&s, &i], 0.01, None)?;
    Ok(())
}

/// The IFC text of a model: read as is, or exported from a project.
pub fn model_ifc(path: &Path) -> Result<String, Box<dyn std::error::Error>> {
    let is_project = path
        .extension()
        .is_some_and(|e| e.eq_ignore_ascii_case(archisim_file::EXTENSION));
    if is_project {
        let model = archisim_file::open(path).map_err(|e| format!("{}: {e}", path.display()))?;
        Ok(crate::ifc_text(&model, &stem(path), None))
    } else {
        Ok(std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?)
    }
}

fn stem(path: &Path) -> String {
    path.file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_default()
}

fn check_ids(model: &Path, ids: &Path, bcf: Option<&Path>) -> R<bool> {
    let ifc = model_ifc(model)?;
    let xml = std::fs::read_to_string(ids).map_err(|e| format!("{}: {e}", ids.display()))?;
    let report = archisim_ids::check_texts(&xml, &ifc)?;
    print!("{}", report.summary());
    if let Some(out) = bcf {
        let issues = archisim_coord::ids_to_bcf(&report, &file_ref(model), &author());
        std::fs::write(out, archisim_bcf::write_bcf(&issues))?;
        println!("\n{} incidencias en {}", issues.topics.len(), out.display());
    }
    Ok(report.passed())
}

#[cfg(test)]
mod tests {
    #[test]
    fn demo_against_sample_requirements() {
        let doc = crate::demo::build().unwrap();
        let ifc = crate::ifc_text(doc.model(), "demo", None);
        let ids = include_str!("../../archisim-ids/tests/fixtures/vivienda.ids");
        let report = archisim_ids::check_texts(ids, &ifc).unwrap();
        let by_name = |n: &str| {
            report
                .specifications
                .iter()
                .find(|s| s.name.starts_with(n))
                .unwrap_or_else(|| panic!("{n}"))
        };
        // What ArchiSim exports today.
        for ok in [
            "Muros: exterior",
            "Muros: superficie",
            "Puertas",
            "Forjados",
            "Todo elemento",
        ] {
            let s = by_name(ok);
            assert!(s.passed, "{ok}: {}", report.summary());
            assert!(s.applicable > 0, "{ok}");
        }
        // The person and the car of the garden have no IFC class of their
        // own: they go as generic elements, which these requirements
        // forbid.
        let generic = by_name("Sin elementos");
        assert!(!generic.passed);
        assert_eq!(generic.failures.len(), 2, "{}", report.summary());
        // No U-values yet: every exterior wall fails, and says why.
        let u = by_name("Muros exteriores");
        assert!(!u.passed);
        assert!(u.applicable > 0);
        assert_eq!(u.failures.len(), u.applicable);
        assert!(u.failures[0].reasons[0].contains("ThermalTransmittance"));
        assert!(u.failures[0].global_id.is_some());
    }
}
