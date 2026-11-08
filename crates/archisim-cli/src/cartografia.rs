//! `archisim cartografia`: the surroundings of a project from the
//! cadastre, OpenStreetMap and a terrain grid (H7).

use std::io::Read as _;
use std::path::Path;
use std::time::Duration;

use archisim_core::{Model, Point2};
use archisim_library::carto::{import_carto, CartoOptions};
use archisim_library::geocode;

/// Opens a project (or starts one from the template).
pub fn open(project: Option<&Path>) -> Result<Model, Box<dyn std::error::Error>> {
    Ok(match project {
        Some(p) => archisim_file::open(p)?,
        None => archisim_library::new_project("Entorno"),
    })
}

/// Looks `text` up (an address, a site, a municipality, a cadastral
/// reference or coordinates) with `fetch`, writes what was found and takes
/// the place number `pick` (from 1): a project without a georeference
/// takes one with its origin there (and its ±0,00 on the terrain); one
/// that has it brings round the place, or round its origin if
/// `keep_origin`.
pub fn place(
    m: &mut Model,
    text: &str,
    pick: usize,
    keep_origin: bool,
    opts: &mut CartoOptions,
    fetch: &mut dyn FnMut(&str) -> Result<Vec<u8>, String>,
) -> Result<(), String> {
    let places = geocode::search(text, fetch)?;
    println!("  Encontrado:");
    for (i, p) in places.iter().enumerate() {
        println!(
            "  {} {}. {} ({}, {}) {}",
            if i + 1 == pick { "→" } else { " " },
            i + 1,
            p.line(),
            p.kind,
            p.origin.name(),
            if p.origin == geocode::Origin::Written {
                String::new()
            } else {
                p.coordinates_text()
            }
        );
    }
    let chosen = places.get(pick.saturating_sub(1)).ok_or_else(|| {
        format!(
            "no hay resultado {pick}: se han encontrado {}",
            places.len()
        )
    })?;
    let (lat, lon) = chosen
        .lat_lon
        .ok_or("el lugar elegido no tiene coordenadas")?;
    opts.centre = Point2::new(0.0, 0.0);
    opts.level_to_ground = false;
    if m.georeference().is_none() {
        let g = geocode::georeference_at(chosen).ok_or("el lugar no tiene coordenadas")?;
        println!(
            "  El proyecto se georreferencia con su origen ahí: {} X {:.0} · Y {:.0}",
            g.crs, g.eastings, g.northings
        );
        m.set_georeference(Some(g)).map_err(|e| e.to_string())?;
        opts.level_to_ground = true;
    } else if !keep_origin {
        let c = geocode::plan_point(m, lat, lon)
            .ok_or("el emplazamiento del proyecto no está en UTM: no se sabe dónde cae el lugar")?;
        println!(
            "  Alrededor del lugar, a {:.0} m del origen del proyecto (el proyecto no se mueve)",
            c.distance(Point2::new(0.0, 0.0))
        );
        opts.centre = c;
    } else {
        println!("  Alrededor del origen del proyecto");
    }
    Ok(())
}

/// One address by HTTP GET, through the system's proxy if it has one.
pub fn http_get(url: &str) -> Result<Vec<u8>, String> {
    let agent = ureq::AgentBuilder::new()
        .timeout_connect(Duration::from_secs(20))
        .timeout(Duration::from_secs(60))
        .user_agent(concat!(
            "ArchiSim/",
            env!("CARGO_PKG_VERSION"),
            " (+https://github.com/stronkel/archisim)"
        ))
        .try_proxy_from_env(true)
        .build();
    let res = agent.get(url).call().map_err(|e| match e {
        ureq::Error::Status(code, _) => format!("el servicio responde {code}"),
        ureq::Error::Transport(t) => format!("no se llega al servicio ({})", t.kind()),
    })?;
    let mut bytes = Vec::new();
    res.into_reader()
        .take(16 * 1024 * 1024)
        .read_to_end(&mut bytes)
        .map_err(|e| format!("la respuesta se ha cortado ({e})"))?;
    Ok(bytes)
}

/// Opens a project (or starts one from the template), brings the files'
/// cartography within `radius` of its origin and saves it as `out`.
pub fn run(
    project: Option<&Path>,
    files: &[&Path],
    radius: f64,
    recentre: bool,
    out: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let m = open(project)?;
    let opts = CartoOptions {
        radius,
        recentre,
        ..CartoOptions::default()
    };
    run_with(m, files, &opts, out)
}

/// Brings the files' cartography into `m` as `opts` say and saves it as
/// `out`.
pub fn run_with(
    mut m: Model,
    files: &[&Path],
    opts: &CartoOptions,
    out: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
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
    let r = import_carto(&mut m, &texts, opts)?;
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

#[cfg(test)]
mod tests {
    use super::*;

    const FOUND: &str = r#"[{"id":"1","province":"HUELVA","muni":"PUNTA UMBRÍA","type":"portal","address":"ANCHA","tip_via":"CALLE","lat":37.1801,"lng":-6.9662,"portalNumber":12},
        {"id":"2","province":"HUELVA","muni":"HUELVA","type":"portal","address":"ANCHA","tip_via":"CALLE","lat":37.2610,"lng":-6.9480,"portalNumber":12}]"#;

    #[test]
    fn a_place_georeferences_or_centres_what_is_brought() {
        let mut fetch = |url: &str| -> Result<Vec<u8>, String> {
            assert!(url.contains("cartociudad"));
            Ok(FOUND.as_bytes().to_vec())
        };
        let mut m = Model::new("T");
        let mut o = CartoOptions::default();
        place(&mut m, "Calle Ancha 12", 1, false, &mut o, &mut fetch).unwrap();
        assert_eq!(m.georeference().unwrap().crs, "EPSG:25829");
        assert!(o.level_to_ground && o.centre == Point2::new(0.0, 0.0));
        // Now georeferenced: the second, some 9 km north.
        place(&mut m, "Calle Ancha 12", 2, false, &mut o, &mut fetch).unwrap();
        assert!(!o.level_to_ground && (o.centre.y - 9_000.0).abs() < 300.0);
        place(&mut m, "Calle Ancha 12", 2, true, &mut o, &mut fetch).unwrap();
        assert_eq!(o.centre, Point2::new(0.0, 0.0));
        // A result that is not there, a bad reference, no network.
        let e = place(&mut m, "Calle Ancha 12", 5, false, &mut o, &mut fetch).unwrap_err();
        assert!(e.contains("no hay resultado 5"), "{e}");
        let e = place(&mut m, "1234567AB12", 1, false, &mut o, &mut fetch).unwrap_err();
        assert!(e.contains("11 caracteres"), "{e}");
        let e = place(
            &mut m,
            "Calle Ancha 12",
            1,
            false,
            &mut o,
            &mut |_: &str| Err("no se llega al servicio".into()),
        )
        .unwrap_err();
        assert!(e.contains("Comprueba la conexión"), "{e}");
    }
}
