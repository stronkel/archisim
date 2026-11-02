//! «Descargar el entorno»: the addresses of the free services that give
//! the surroundings of a georeferenced project — the cadastre's buildings,
//! their parts and parcels (its INSPIRE WFS), OpenStreetMap's streets and
//! trees (Overpass) and the IGN's terrain (its INSPIRE WCS) — split in
//! tiles the services accept. No network here: the application fetches
//! them (on a thread) and hands the answers to `carto::import_carto`;
//! `answer_text` cleans what a service answers.

use archisim_core::{utm_to_lat_lon, Model, Point2};

use crate::carto::{CartoOptions, Frame};

/// Where an address comes from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Source {
    /// The cadastre's buildings (their use).
    Buildings,
    /// The cadastre's parts of buildings (their floors).
    BuildingParts,
    /// The cadastre's parcels.
    Parcels,
    /// OpenStreetMap: streets, trees and, with no cadastre, buildings.
    Osm,
    /// The IGN's terrain (MDT05).
    Terrain,
    /// The PNOA's orthophoto (a PNG of the rectangle).
    Orthophoto,
}

impl Source {
    /// Its name, for the window.
    pub fn name(self) -> &'static str {
        match self {
            Source::Buildings => "Catastro: edificios",
            Source::BuildingParts => "Catastro: partes de edificios (plantas)",
            Source::Parcels => "Catastro: parcelas",
            Source::Osm => "OpenStreetMap: calles y árboles",
            Source::Terrain => "IGN: terreno (MDT05)",
            Source::Orthophoto => "IGN: ortofoto del PNOA",
        }
    }
}

/// One address to fetch.
#[derive(Debug, Clone, PartialEq)]
pub struct Request {
    pub source: Source,
    /// The name its answer is given (as if it were a file).
    pub name: String,
    pub url: String,
    /// The rectangle of the map a picture covers (the orthophoto).
    pub bbox: Option<Bbox>,
}

/// The cadastre's services (INSPIRE WFS 2.0).
pub const CADASTRE_BU: &str = "http://ovc.catastro.meh.es/INSPIRE/wfsBU.aspx";
pub const CADASTRE_CP: &str = "http://ovc.catastro.meh.es/INSPIRE/wfsCP.aspx";
/// OpenStreetMap through Overpass.
pub const OVERPASS: &str = "https://overpass-api.de/api/interpreter";
/// The PNOA's most recent orthophoto (INSPIRE WMS 1.3.0).
pub const PNOA: &str = "https://www.ign.es/wms-inspire/pnoa-ma";
/// Pixels of the orthophoto: one each 25 cm (the PNOA's), up to this many
/// a side.
pub const ORTHO_PIXELS: u32 = 2048;
/// The IGN's digital terrain models (INSPIRE WCS 2.0.1).
pub const IGN_MDT: &str = "https://servicios.idee.es/wcs-inspire/mdt";

/// Largest side of a tile of the cadastre's buildings (it serves up to
/// 4 km² a time) and of its parcels (1 km²), metres, with some room.
const BUILDINGS_TILE: f64 = 1_900.0;
const PARCELS_TILE: f64 = 950.0;

/// A rectangle of the map: west, south, east, north.
pub type Bbox = [f64; 4];

/// The rectangle of the map round the circle of `radius` about the plan's
/// `centre`, by the project's georeference; `None` if it has none or it
/// is not in UTM.
pub fn map_bbox(m: &Model, centre: Point2, radius: f64) -> Option<(Bbox, u32)> {
    let g = m.georeference()?;
    g.utm_zone()?;
    let code: u32 = g
        .crs
        .trim()
        .to_ascii_uppercase()
        .strip_prefix("EPSG:")?
        .parse()
        .ok()?;
    let c = Frame::of(m, g).to_map(centre);
    Some((
        [c[0] - radius, c[1] - radius, c[0] + radius, c[1] + radius],
        code,
    ))
}

/// A rectangle cut in tiles no wider than `side`.
pub fn tiles(b: Bbox, side: f64) -> Vec<Bbox> {
    let nx = ((b[2] - b[0]) / side).ceil().max(1.0) as usize;
    let ny = ((b[3] - b[1]) / side).ceil().max(1.0) as usize;
    let (w, h) = ((b[2] - b[0]) / nx as f64, (b[3] - b[1]) / ny as f64);
    let mut out = Vec::with_capacity(nx * ny);
    for j in 0..ny {
        for i in 0..nx {
            let x = b[0] + w * i as f64;
            let y = b[1] + h * j as f64;
            out.push([x, y, x + w, y + h]);
        }
    }
    out
}

fn wfs(base: &str, type_name: &str, b: Bbox, code: u32) -> String {
    format!(
        "{base}?service=wfs&version=2&request=getfeature&typenames={type_name}&bbox={:.0},{:.0},{:.0},{:.0}&srsname=EPSG::{code}",
        b[0], b[1], b[2], b[3]
    )
}

/// Percent-encodes a query value.
pub fn encode(s: &str) -> String {
    let mut out = String::with_capacity(s.len() * 3);
    for b in s.bytes() {
        if b.is_ascii_alphanumeric() || b"-_.~".contains(&b) {
            out.push(b as char);
        } else {
            out.push_str(&format!("%{b:02X}"));
        }
    }
    out
}

/// The Overpass query for a rectangle of latitudes and longitudes
/// (south, west, north, east).
pub fn overpass_query(s: f64, w: f64, n: f64, e: f64, buildings: bool) -> String {
    let bb = format!("{s:.6},{w:.6},{n:.6},{e:.6}");
    let mut q = format!(
        "[out:xml][timeout:120];(way[\"highway\"]({bb});node[\"natural\"=\"tree\"]({bb});way[\"natural\"=\"tree_row\"]({bb});"
    );
    if buildings {
        q.push_str(&format!("way[\"building\"]({bb});"));
    }
    q.push_str(");(._;>;);out body;");
    q
}

/// The addresses to fetch for what `opts` asks, round its centre.
pub fn plan(m: &Model, opts: &CartoOptions) -> Result<Vec<Request>, String> {
    let (b, code) = map_bbox(m, opts.centre, opts.radius).ok_or_else(|| {
        "el proyecto no está georreferenciado en UTM: pon su emplazamiento en las propiedades del proyecto («Emplazamiento (georreferencia)»)".to_string()
    })?;
    let g = m.georeference().expect("map_bbox checked it");
    let (zone, north) = g.utm_zone().expect("map_bbox checked it");
    let mut out = Vec::new();
    if opts.buildings {
        for (i, t) in tiles(b, BUILDINGS_TILE).into_iter().enumerate() {
            out.push(Request {
                bbox: None,
                source: Source::Buildings,
                name: format!("catastro-edificios-{}.gml", i + 1),
                url: wfs(CADASTRE_BU, "BU.BUILDING", t, code),
            });
            out.push(Request {
                bbox: None,
                source: Source::BuildingParts,
                name: format!("catastro-partes-{}.gml", i + 1),
                url: wfs(CADASTRE_BU, "BU.BUILDINGPART", t, code),
            });
        }
    }
    if opts.parcels {
        for (i, t) in tiles(b, PARCELS_TILE).into_iter().enumerate() {
            out.push(Request {
                bbox: None,
                source: Source::Parcels,
                name: format!("catastro-parcelas-{}.gml", i + 1),
                url: wfs(CADASTRE_CP, "CP.CADASTRALPARCEL", t, code),
            });
        }
    }
    if opts.streets || opts.trees || opts.buildings {
        // Latitudes and longitudes of the rectangle's corners.
        let corners = [
            utm_to_lat_lon(b[0], b[1], zone, north),
            utm_to_lat_lon(b[2], b[1], zone, north),
            utm_to_lat_lon(b[2], b[3], zone, north),
            utm_to_lat_lon(b[0], b[3], zone, north),
        ];
        let s = corners.iter().map(|c| c.0).fold(f64::MAX, f64::min);
        let n = corners.iter().map(|c| c.0).fold(f64::MIN, f64::max);
        let w = corners.iter().map(|c| c.1).fold(f64::MAX, f64::min);
        let e = corners.iter().map(|c| c.1).fold(f64::MIN, f64::max);
        out.push(Request {
            bbox: None,
            source: Source::Osm,
            name: "openstreetmap.osm".into(),
            url: format!(
                "{OVERPASS}?data={}",
                encode(&overpass_query(s, w, n, e, opts.buildings))
            ),
        });
    }
    if opts.orthophoto {
        let px = ((b[2] - b[0]) / 0.25)
            .ceil()
            .clamp(64.0, ORTHO_PIXELS as f64) as u32;
        out.push(Request {
            bbox: Some(b),
            source: Source::Orthophoto,
            name: "pnoa.png".into(),
            url: format!(
                "{PNOA}?SERVICE=WMS&VERSION=1.3.0&REQUEST=GetMap&LAYERS=OI.OrthoimageCoverage&STYLES=&CRS=EPSG:{code}&BBOX={:.0},{:.0},{:.0},{:.0}&WIDTH={px}&HEIGHT={px}&FORMAT=image/png",
                b[0], b[1], b[2], b[3]
            ),
        });
    }
    if opts.terrain {
        out.push(Request {
            bbox: None,
            source: Source::Terrain,
            name: "ign-mdt05.asc".into(),
            url: format!(
                "{IGN_MDT}?SERVICE=WCS&VERSION=2.0.1&REQUEST=GetCoverage&CoverageId=Elevacion{code}_5&SUBSET=x({:.0},{:.0})&SUBSET=y({:.0},{:.0})&FORMAT=application/asc",
                b[0], b[2], b[1], b[3]
            ),
        });
    }
    Ok(out)
}

/// Whether bytes are a PNG.
pub fn is_png(bytes: &[u8]) -> bool {
    bytes.starts_with(&[0x89, b'P', b'N', b'G'])
}

/// What a service answered, as the text the importer reads: the grid of a
/// coverage sent in several parts (`multipart/related`) without its
/// wrapping; `Err` with the service's own message when it answered an
/// error (an exception report), or when the answer is not what was asked.
pub fn answer_text(source: Source, body: &str) -> Result<String, String> {
    let low = body
        .get(..body.len().min(4096))
        .unwrap_or(body)
        .to_ascii_lowercase();
    if low.contains("exceptionreport") || low.contains("serviceexception") {
        let msg = between(body, "ExceptionText>", "</")
            .or_else(|| between(body, "ServiceException>", "</"))
            .unwrap_or("el servicio no da más detalle");
        return Err(msg.trim().to_string());
    }
    match source {
        Source::Terrain => {
            let at = body
                .find("ncols")
                .or_else(|| body.find("NCOLS"))
                .ok_or("no ha llegado una malla ASC")?;
            let grid = &body[at..];
            // A multipart answer ends the part with its boundary («--…»).
            let end = grid.find("\n--").unwrap_or(grid.len());
            Ok(grid[..end].trim_end().to_string() + "\n")
        }
        Source::Osm if !low.contains("<osm") => {
            Err("no ha llegado un fichero de OpenStreetMap".into())
        }
        Source::Buildings | Source::BuildingParts | Source::Parcels
            if !low.contains("featurecollection") =>
        {
            Err("no ha llegado un fichero GML del Catastro".into())
        }
        _ => Ok(body.to_string()),
    }
}

fn between<'a>(s: &'a str, a: &str, b: &str) -> Option<&'a str> {
    let i = s.find(a)? + a.len();
    let j = s[i..].find(b)? + i;
    Some(&s[i..j])
}

#[cfg(test)]
mod tests {
    use super::*;
    use archisim_core::Georeference;

    fn placed() -> Model {
        let mut m = Model::new("T");
        m.set_georeference(Some(Georeference {
            crs: "EPSG:25830".into(),
            eastings: 440_000.0,
            northings: 4_474_000.0,
            height: 650.0,
        }))
        .unwrap();
        m
    }

    #[test]
    fn tiles_cover_the_rectangle_within_the_limits() {
        let t = tiles([0.0, 0.0, 2_500.0, 1_000.0], 950.0);
        assert_eq!(t.len(), 3 * 2);
        for b in &t {
            assert!(b[2] - b[0] <= 950.0 && b[3] - b[1] <= 950.0);
        }
        let area: f64 = t.iter().map(|b| (b[2] - b[0]) * (b[3] - b[1])).sum();
        assert!((area - 2_500_000.0).abs() < 1e-6);
        assert_eq!(tiles([0.0, 0.0, 300.0, 300.0], 950.0).len(), 1);
    }

    #[test]
    fn the_addresses_ask_for_the_projects_surroundings() {
        let m = placed();
        let opts = CartoOptions {
            radius: 600.0,
            ..CartoOptions::default()
        };
        let p = plan(&m, &opts).unwrap();
        let of = |s: Source| p.iter().filter(|r| r.source == s).collect::<Vec<_>>();
        // 1.2 km a side: one tile of buildings, four of parcels.
        assert_eq!(of(Source::Buildings).len(), 1);
        assert_eq!(of(Source::BuildingParts).len(), 1);
        assert_eq!(of(Source::Parcels).len(), 4);
        let b = &of(Source::BuildingParts)[0].url;
        assert!(b.starts_with(CADASTRE_BU), "{b}");
        assert!(b.contains("typenames=BU.BUILDINGPART"), "{b}");
        assert!(b.contains("bbox=439400,4473400,440600,4474600"), "{b}");
        assert!(b.contains("srsname=EPSG::25830"), "{b}");
        // OpenStreetMap by latitude and longitude, round Madrid.
        let o = &of(Source::Osm)[0].url;
        assert!(o.starts_with(OVERPASS), "{o}");
        assert!(o.contains("highway") && o.contains("40.4"), "{o}");
        let t = &of(Source::Terrain)[0].url;
        assert!(t.contains("CoverageId=Elevacion25830_5"), "{t}");
        assert!(t.contains("SUBSET=x(439400,440600)"), "{t}");
        // The orthophoto: the rectangle at 25 cm a pixel, up to its limit.
        let o = of(Source::Orthophoto)[0];
        assert!(
            o.url.starts_with(PNOA) && o.url.contains("CRS=EPSG:25830"),
            "{}",
            o.url
        );
        assert!(o.url.contains("WIDTH=2048&HEIGHT=2048"), "{}", o.url);
        assert_eq!(
            o.bbox,
            Some([439_400.0, 4_473_400.0, 440_600.0, 4_474_600.0])
        );
        // Only what is asked.
        let few = plan(
            &m,
            &CartoOptions {
                buildings: false,
                parcels: false,
                streets: false,
                trees: false,
                orthophoto: false,
                ..CartoOptions::default()
            },
        )
        .unwrap();
        assert_eq!(few.len(), 1);
        assert_eq!(few[0].source, Source::Terrain);
        // Without a georeference there is nothing to ask.
        assert!(plan(&Model::new("T"), &opts).is_err());
    }

    #[test]
    fn the_answers_are_cleaned_or_explained() {
        let multi = "--wcs\r\nContent-Type: text/xml\r\n\r\n<gml/>\r\n--wcs\r\nContent-Type: application/asc\r\n\r\nncols 2\nnrows 1\nxllcorner 0\nyllcorner 0\ncellsize 5\n1 2\n--wcs--\r\n";
        let t = answer_text(Source::Terrain, multi).unwrap();
        assert!(
            t.starts_with("ncols 2") && t.trim_end().ends_with("1 2"),
            "{t:?}"
        );
        assert!(archisim_carto::asc::read(&t).is_ok());
        let err = "<ows:ExceptionReport><ows:Exception><ows:ExceptionText>Bbox too big</ows:ExceptionText></ows:Exception></ows:ExceptionReport>";
        assert_eq!(
            answer_text(Source::Parcels, err),
            Err("Bbox too big".into())
        );
        assert!(answer_text(Source::Osm, "<html>busy</html>").is_err());
        assert!(answer_text(Source::Osm, "<?xml?><osm version=\"0.6\"></osm>").is_ok());
    }

    #[test]
    fn encoding_keeps_only_safe_letters() {
        assert_eq!(encode("a b\"[x]"), "a%20b%22%5Bx%5D");
    }
}
