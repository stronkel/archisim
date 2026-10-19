//! Georeferencing: where the project's ±0,00 is on the map, in a projected
//! coordinate system (UTM, as the Spanish cadastre and topographic surveys
//! use), and the latitude and longitude that follow.
//!
//! IFC: `IfcMapConversion` from the model's context to an `IfcProjectedCRS`
//! (eastings, northings, orthogonal height and the direction of the
//! model's x axis on the map), and `RefLatitude`/`RefLongitude` of the
//! site.
//!
//! The model's true north is geographic north; the map's axes point to
//! grid north. Between them is the grid convergence, worked out here from
//! the UTM zone.

use crate::model::{Model, ModelError};

/// Where the project is on the map.
#[derive(Debug, Clone, PartialEq)]
pub struct Georeference {
    /// Coordinate reference system, as `EPSG:<code>` ("EPSG:25830",
    /// ETRS89 / UTM zone 30N).
    pub crs: String,
    /// Map coordinates of the model's origin (the project's ±0,00 at plan
    /// 0,0), metres.
    pub eastings: f64,
    pub northings: f64,
    /// Height of the ±0,00 above the vertical datum (sea level), metres.
    pub height: f64,
}

/// Coordinate systems offered in the interface: (code, name).
pub const CRS_LIST: [(&str, &str); 10] = [
    (
        "EPSG:25829",
        "ETRS89 / UTM 29N (Galicia, oeste de Andalucía…)",
    ),
    ("EPSG:25830", "ETRS89 / UTM 30N (centro peninsular)"),
    ("EPSG:25831", "ETRS89 / UTM 31N (Cataluña, Baleares…)"),
    ("EPSG:4083", "REGCAN95 / UTM 28N (Canarias)"),
    ("EPSG:4082", "REGCAN95 / UTM 27N (El Hierro, La Palma)"),
    ("EPSG:32628", "WGS 84 / UTM 28N"),
    ("EPSG:32629", "WGS 84 / UTM 29N"),
    ("EPSG:32630", "WGS 84 / UTM 30N"),
    ("EPSG:32631", "WGS 84 / UTM 31N"),
    ("EPSG:3857", "WGS 84 / Pseudo-Mercator (web)"),
];

impl Georeference {
    /// The UTM zone and hemisphere of its system, if it is a UTM one this
    /// program knows: ETRS89 (258zz), WGS 84 (326zz north, 327zz south)
    /// and REGCAN95 (4082, 4083).
    pub fn utm_zone(&self) -> Option<(u32, bool)> {
        let code: u32 = self
            .crs
            .trim()
            .to_ascii_uppercase()
            .strip_prefix("EPSG:")?
            .parse()
            .ok()?;
        match code {
            25828..=25838 => Some((code - 25800, true)),
            32601..=32660 => Some((code - 32600, true)),
            32701..=32760 => Some((code - 32700, false)),
            4082 => Some((27, true)),
            4083 => Some((28, true)),
            _ => None,
        }
    }

    /// The geodetic datum of its system, for IFC.
    pub fn datum(&self) -> &'static str {
        let code = self.crs.trim().to_ascii_uppercase();
        if code.starts_with("EPSG:258") {
            "ETRS89"
        } else if code == "EPSG:4082" || code == "EPSG:4083" {
            "REGCAN95"
        } else {
            "WGS84"
        }
    }

    /// Latitude and longitude of the origin, degrees (north and east
    /// positive), for a UTM system.
    pub fn lat_lon(&self) -> Option<(f64, f64)> {
        let (zone, north) = self.utm_zone()?;
        Some(utm_to_lat_lon(self.eastings, self.northings, zone, north))
    }

    /// Grid convergence at the origin: the angle from true north to grid
    /// north, radians, clockwise (positive east of the central meridian in
    /// the northern hemisphere). Zero if the system is not UTM.
    pub fn convergence(&self) -> f64 {
        let (Some((zone, _)), Some((lat, lon))) = (self.utm_zone(), self.lat_lon()) else {
            return 0.0;
        };
        let lon0 = (zone as f64) * 6.0 - 183.0;
        let dl = (lon - lon0).to_radians();
        (dl.tan() * lat.to_radians().sin()).atan()
    }
}

/// Inverse UTM (transverse Mercator on GRS80/WGS 84, which differ by less
/// than a millimetre here): latitude and longitude in degrees.
pub fn utm_to_lat_lon(e: f64, n: f64, zone: u32, north: bool) -> (f64, f64) {
    let a = 6_378_137.0_f64;
    let f: f64 = 1.0 / 298.257_222_101;
    let k0 = 0.9996;
    let e2 = f * (2.0 - f);
    let ep2 = e2 / (1.0 - e2);
    let x = e - 500_000.0;
    let y = if north { n } else { n - 10_000_000.0 };
    let m = y / k0;
    let mu = m / (a * (1.0 - e2 / 4.0 - 3.0 * e2 * e2 / 64.0 - 5.0 * e2.powi(3) / 256.0));
    let e1 = (1.0 - (1.0 - e2).sqrt()) / (1.0 + (1.0 - e2).sqrt());
    let phi1 = mu
        + (3.0 * e1 / 2.0 - 27.0 * e1.powi(3) / 32.0) * (2.0 * mu).sin()
        + (21.0 * e1 * e1 / 16.0 - 55.0 * e1.powi(4) / 32.0) * (4.0 * mu).sin()
        + (151.0 * e1.powi(3) / 96.0) * (6.0 * mu).sin()
        + (1097.0 * e1.powi(4) / 512.0) * (8.0 * mu).sin();
    let (s, c) = phi1.sin_cos();
    let t = s / c;
    let n1 = a / (1.0 - e2 * s * s).sqrt();
    let t1 = t * t;
    let c1 = ep2 * c * c;
    let r1 = a * (1.0 - e2) / (1.0 - e2 * s * s).powf(1.5);
    let d = x / (n1 * k0);
    let lat = phi1
        - (n1 * t / r1)
            * (d * d / 2.0
                - (5.0 + 3.0 * t1 + 10.0 * c1 - 4.0 * c1 * c1 - 9.0 * ep2) * d.powi(4) / 24.0
                + (61.0 + 90.0 * t1 + 298.0 * c1 + 45.0 * t1 * t1 - 252.0 * ep2 - 3.0 * c1 * c1)
                    * d.powi(6)
                    / 720.0);
    let lon0 = ((zone as f64) * 6.0 - 183.0).to_radians();
    let lon = lon0
        + (d - (1.0 + 2.0 * t1 + c1) * d.powi(3) / 6.0
            + (5.0 - 2.0 * c1 + 28.0 * t1 - 3.0 * c1 * c1 + 8.0 * ep2 + 24.0 * t1 * t1)
                * d.powi(5)
                / 120.0)
            / c;
    (lat.to_degrees(), lon.to_degrees())
}

impl Model {
    pub fn georeference(&self) -> Option<&Georeference> {
        self.georeference.as_ref()
    }

    /// Sets (or clears) where the project is on the map.
    pub fn set_georeference(&mut self, g: Option<Georeference>) -> Result<(), ModelError> {
        if let Some(g) = &g {
            for v in [g.eastings, g.northings, g.height] {
                if !v.is_finite() {
                    return Err(ModelError::NotFinite("georeference"));
                }
            }
            if !g.crs.trim().to_ascii_uppercase().starts_with("EPSG:") {
                return Err(ModelError::InvalidShape(
                    "the coordinate system is written EPSG:<code>",
                ));
            }
        }
        self.georeference = g.map(|g| Georeference {
            crs: g.crs.trim().to_ascii_uppercase(),
            ..g
        });
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The Puerta del Sol (Madrid) and the town hall of Punta Umbría
    /// (Huelva), from their published UTM coordinates.
    #[test]
    fn utm_to_degrees_and_convergence() {
        let sol = Georeference {
            crs: "EPSG:25830".into(),
            eastings: 440_291.0,
            northings: 4_474_254.0,
            height: 650.0,
        };
        let (lat, lon) = sol.lat_lon().unwrap();
        assert!((lat - 40.4168).abs() < 0.001, "{lat}");
        assert!((lon - (-3.7038)).abs() < 0.001, "{lon}");
        // West of the central meridian (−3°): grid north west of true north.
        assert!(sol.convergence() < 0.0);
        let pu = Georeference {
            crs: "EPSG:25829".into(),
            eastings: 680_000.0,
            northings: 4_119_000.0,
            height: 3.0,
        };
        let (lat, lon) = pu.lat_lon().unwrap();
        assert!(
            (lat - 37.2).abs() < 0.1 && (lon - (-6.96)).abs() < 0.1,
            "{lat} {lon}"
        );
        assert!(pu.convergence() > 0.0);
        assert_eq!(pu.datum(), "ETRS89");
        let web = Georeference {
            crs: "EPSG:3857".into(),
            ..pu
        };
        assert_eq!(web.lat_lon(), None);
        assert_eq!(web.convergence(), 0.0);
    }

    #[test]
    fn set_and_clear() {
        let mut m = Model::new("P");
        assert!(m
            .set_georeference(Some(Georeference {
                crs: "25830".into(),
                eastings: 0.0,
                northings: 0.0,
                height: 0.0,
            }))
            .is_err());
        m.set_georeference(Some(Georeference {
            crs: " epsg:25830 ".into(),
            eastings: 1.0,
            northings: 2.0,
            height: 3.0,
        }))
        .unwrap();
        assert_eq!(m.georeference().unwrap().crs, "EPSG:25830");
        m.set_georeference(None).unwrap();
        assert!(m.georeference().is_none());
    }
}
