//! `archisim cartografia`: the surroundings of a project from the
//! cadastre, OpenStreetMap and a terrain grid (H7).

use std::path::Path;

use archisim_library::carto::{import_carto, CartoOptions};

/// Opens a project (or starts one from the template), brings the files'
/// cartography within `radius` of its origin and saves it as `out`.
pub fn run(
    project: Option<&Path>,
    files: &[&Path],
    radius: f64,
    recentre: bool,
    out: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut m = match project {
        Some(p) => archisim_file::open(p)?,
        None => archisim_library::new_project("Entorno"),
    };
    let mut texts = Vec::new();
    for f in files {
        let bytes =
            std::fs::read(f).map_err(|e| format!("no se puede leer {}: {e}", f.display()))?;
        let name = f.file_name().map_or_else(
            || f.display().to_string(),
            |n| n.to_string_lossy().into_owned(),
        );
        texts.push((name, String::from_utf8_lossy(&bytes).into_owned()));
    }
    let opts = CartoOptions {
        radius,
        recentre,
        ..CartoOptions::default()
    };
    let r = import_carto(&mut m, &texts, &opts)?;
    for l in r.lines() {
        println!("  {l}");
    }
    if let Some(g) = m.georeference() {
        println!(
            "  Origen: {} X {:.0} · Y {:.0}",
            g.crs, g.eastings, g.northings
        );
    }
    if let Some(dir) = out.parent().filter(|d| !d.as_os_str().is_empty()) {
        std::fs::create_dir_all(dir)?;
    }
    archisim_file::save(&m, out)?;
    println!("  Proyecto: {}", out.display());
    Ok(())
}
