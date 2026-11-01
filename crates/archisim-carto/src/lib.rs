//! Free cartography to fill the surroundings of a project (H7): the
//! Spanish cadastre's INSPIRE buildings and parcels ([`catastro`]),
//! OpenStreetMap's streets, buildings and trees ([`osm`]) and terrain
//! grids such as the CNIG's MDT ([`asc`]); and the projection from
//! latitude and longitude to UTM ([`lat_lon_to_utm`]) that OSM needs.
//!
//! No dependencies; it knows nothing of the model: `archisim-library`'s
//! `carto` turns what it reads into masses, plots, roads, trees and
//! terrain.

pub mod asc;
pub mod catastro;
pub mod osm;
pub mod xml;

/// What kind of file a text is, by its content.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileKind {
    Cadastre,
    Osm,
    Grid,
}

/// Guesses the kind of a cartographic file from its first bytes.
pub fn kind_of(text: &str) -> Option<FileKind> {
    let head: String = text.chars().take(4000).collect();
    let lower = head.to_ascii_lowercase();
    if lower.contains("<osm") {
        Some(FileKind::Osm)
    } else if lower.contains("featurecollection") || lower.contains("gml:") {
        Some(FileKind::Cadastre)
    } else if lower.trim_start().starts_with("ncols") || lower.contains("\nncols") {
        Some(FileKind::Grid)
    } else {
        None
    }
}

/// Direct UTM (transverse Mercator on GRS80/WGS 84): eastings and
/// northings of a latitude and longitude (degrees) in a zone.
pub fn lat_lon_to_utm(lat: f64, lon: f64, zone: u32, north: bool) -> (f64, f64) {
    let a = 6_378_137.0_f64;
    let f: f64 = 1.0 / 298.257_222_101;
    let k0 = 0.9996;
    let e2 = f * (2.0 - f);
    let ep2 = e2 / (1.0 - e2);
    let phi = lat.to_radians();
    let lon0 = (zone as f64 * 6.0 - 183.0).to_radians();
    let dl = lon.to_radians() - lon0;
    let (s, c) = phi.sin_cos();
    let t = s / c;
    let n = a / (1.0 - e2 * s * s).sqrt();
    let t2 = t * t;
    let c2 = ep2 * c * c;
    let aa = c * dl;
    let e4 = e2 * e2;
    let e6 = e4 * e2;
    let m = a
        * ((1.0 - e2 / 4.0 - 3.0 * e4 / 64.0 - 5.0 * e6 / 256.0) * phi
            - (3.0 * e2 / 8.0 + 3.0 * e4 / 32.0 + 45.0 * e6 / 1024.0) * (2.0 * phi).sin()
            + (15.0 * e4 / 256.0 + 45.0 * e6 / 1024.0) * (4.0 * phi).sin()
            - (35.0 * e6 / 3072.0) * (6.0 * phi).sin());
    let x = k0
        * n
        * (aa
            + (1.0 - t2 + c2) * aa.powi(3) / 6.0
            + (5.0 - 18.0 * t2 + t2 * t2 + 72.0 * c2 - 58.0 * ep2) * aa.powi(5) / 120.0);
    let y = k0
        * (m + n
            * t
            * (aa * aa / 2.0
                + (5.0 - t2 + 9.0 * c2 + 4.0 * c2 * c2) * aa.powi(4) / 24.0
                + (61.0 - 58.0 * t2 + t2 * t2 + 600.0 * c2 - 330.0 * ep2) * aa.powi(6) / 720.0));
    let e = x + 500_000.0;
    let n_ = if north { y } else { y + 10_000_000.0 };
    (e, n_)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn projects_to_utm() {
        // Punta Umbría, UTM 29N: some 680 km east, 4 116 km north.
        let (e, n) = lat_lon_to_utm(37.18, -6.97, 29, true);
        assert!((670_000.0..690_000.0).contains(&e), "{e}");
        assert!((4_105_000.0..4_125_000.0).contains(&n), "{n}");
        // On the central meridian, the eastings are 500 km.
        let (e, _) = lat_lon_to_utm(40.0, -3.0, 30, true);
        assert!((e - 500_000.0).abs() < 1e-6);
        assert_eq!(kind_of("<?xml?><osm version=\"0.6\">"), Some(FileKind::Osm));
        assert_eq!(kind_of("ncols 3\nnrows 2"), Some(FileKind::Grid));
    }
}
