//! Commands of phase 4 (collaboration): checking requirements (IDS).
//! More commands join here as they are built, so that `main.rs` only
//! hands over the ones it does not know.

use std::path::Path;
use std::process::ExitCode;

pub const HELP: &str = "\
Colaboración:
  archisim comprobar-ids <modelo.archisim|modelo.ifc> <requisitos.ids>
      Comprueba el modelo contra los requisitos de información del cliente
      (IDS 1.0 de buildingSMART): qué especificaciones cumple y qué
      elementos fallan, y por qué. Un proyecto .archisim se comprueba tal
      como sale en su IFC. Termina con error si alguna no se cumple.
";

/// Runs a collaboration command, or `None` if `args` is not one.
pub fn run(args: &[String]) -> Option<ExitCode> {
    let a = |i: usize| args.get(i).map(String::as_str);
    match a(0)? {
        "comprobar-ids" | "ids" => Some(match (a(1), a(2)) {
            (Some(model), Some(ids)) => match check_ids(Path::new(model), Path::new(ids)) {
                Ok(true) => ExitCode::SUCCESS,
                Ok(false) => ExitCode::from(1),
                Err(e) => {
                    eprintln!("Error: {e}");
                    ExitCode::from(2)
                }
            },
            _ => {
                eprintln!(
                    "Uso: archisim comprobar-ids <modelo.archisim|modelo.ifc> <requisitos.ids>"
                );
                ExitCode::from(2)
            }
        }),
        _ => None,
    }
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

fn check_ids(model: &Path, ids: &Path) -> Result<bool, Box<dyn std::error::Error>> {
    let ifc = model_ifc(model)?;
    let xml = std::fs::read_to_string(ids).map_err(|e| format!("{}: {e}", ids.display()))?;
    let report = archisim_ids::check_texts(&xml, &ifc)?;
    print!("{}", report.summary());
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
            "Sin elementos",
        ] {
            let s = by_name(ok);
            assert!(s.passed, "{ok}: {}", report.summary());
            assert!(s.applicable > 0 || ok == "Sin elementos", "{ok}");
        }
        // No U-values yet: every exterior wall fails, and says why.
        let u = by_name("Muros exteriores");
        assert!(!u.passed);
        assert!(u.applicable > 0);
        assert_eq!(u.failures.len(), u.applicable);
        assert!(u.failures[0].reasons[0].contains("ThermalTransmittance"));
        assert!(u.failures[0].global_id.is_some());
    }
}
