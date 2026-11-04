//! «Descargar el entorno»: the addresses of the free services that give
//! the surroundings of a georeferenced project — the cadastre's buildings,
//! their parts and parcels (its INSPIRE WFS), OpenStreetMap's streets and
//! trees (Overpass, with other servers when one is busy) and the IGN's
//! terrain (its tiles of heights, `xyz-mdt`, in any UTM zone) — split in
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
    /// The IGN's surface model (MDS05, from the LiDAR): with the terrain,
    /// the floors of the buildings.
    Surface,
}

impl Source {
    /// Its name, for the window.
    pub fn name(self) -> &'static str {
        match self {
            Source::Buildings => "Catastro: edificios",
            Source::BuildingParts => "Catastro: partes de edificios (plantas)",
            Source::Parcels => "Catastro: parcelas",
            Source::Osm => "OpenStreetMap: calles y árboles",
            Source::Terrain => "IGN: terreno (MDT)",
            Source::Orthophoto => "IGN: ortofoto del PNOA",
            Source::Surface => "IGN: modelo de superficies (MDS, LiDAR)",
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
    /// When given, what is asked instead of `url`, in turn until one
    /// answers: an address and, for a POST, its form (`url` stays the
    /// address to open in a browser).
    pub attempts: Vec<(String, Option<String>)>,
    /// A tile of heights of the terrain and what it is for.
    pub tile: Option<TerrainTile>,
}

/// A tile of the IGN's heights (zoom, column, row of the web map) and the
/// rectangle of the map its grid is wanted for.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TerrainTile {
    pub z: u32,
    pub x: u32,
    pub y: u32,
    pub bbox: Bbox,
    pub zone: u32,
    pub north: bool,
}

/// The cadastre's services (INSPIRE WFS 2.0).
pub const CADASTRE_BU: &str = "http://ovc.catastro.meh.es/INSPIRE/wfsBU.aspx";
pub const CADASTRE_CP: &str = "http://ovc.catastro.meh.es/INSPIRE/wfsCP.aspx";
/// OpenStreetMap through Overpass, and other public servers of it for
/// when the main one is busy.
pub const OVERPASS: &str = "https://overpass-api.de/api/interpreter";
/// OpenStreetMap's own map by rectangle (up to 0,25 square degrees).
pub const OSM_MAP: &str = "https://api.openstreetmap.org/api/0.6/map";
pub const OVERPASS_MIRRORS: [&str; 2] = [
    "https://overpass.kumi.systems/api/interpreter",
    "https://overpass.private.coffee/api/interpreter",
];
/// The PNOA's most recent orthophoto (INSPIRE WMS 1.3.0).
pub const PNOA: &str = "https://www.ign.es/wms-inspire/pnoa-ma";
/// Pixels of the orthophoto: one each 25 cm (the PNOA's), up to this many
/// a side.
pub const ORTHO_PIXELS: u32 = 2048;
/// The IGN's tiles of heights of the whole of Spain (MDT, «Terrain RGB»:
/// height = −10000 + (R·65536 + G·256 + B) · 0,1 m), in the web map's
/// tiles.
pub const IGN_HEIGHTS: &str = "https://xyz-mdt.idee.es/1.0.0/raster-dem";
/// The IGN's surface model by rectangle (WCS of the MDS, 5 m, which it
/// reprojects to any UTM zone): its coverage of 5 m.
pub const IGN_SURFACE: &str = "https://wcs-mds.idee.es/mds";
pub const SURFACE_COVERAGE: &str = "MDS";
/// Zoom of those tiles: 15, its finest (some 4 m a pixel in Spain).
pub const TERRAIN_ZOOM: u32 = 15;
/// Spacing of the terrain grid made from them, metres.
pub const TERRAIN_CELL: f64 = 5.0;

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
                attempts: Vec::new(),
                tile: None,
                source: Source::Buildings,
                name: format!("catastro-edificios-{}.gml", i + 1),
                url: wfs(CADASTRE_BU, "BU.BUILDING", t, code),
            });
            out.push(Request {
                bbox: None,
                attempts: Vec::new(),
                tile: None,
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
                attempts: Vec::new(),
                tile: None,
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
            attempts: Vec::new(),
            tile: None,
            source: Source::Osm,
            name: "openstreetmap.osm".into(),
            url: format!(
                "{OVERPASS}?data={}",
                encode(&overpass_query(s, w, n, e, opts.buildings))
            ),
        });
        // Overpass asked by POST (as it prefers), its other servers when
        // it is busy, and last OpenStreetMap's own map (everything in the
        // rectangle; the importer takes what it wants).
        let form = format!(
            "data={}",
            encode(&overpass_query(s, w, n, e, opts.buildings))
        );
        if let Some(r) = out.last_mut() {
            r.attempts = std::iter::once(OVERPASS)
                .chain(OVERPASS_MIRRORS)
                .map(|m| (m.to_string(), Some(form.clone())))
                .chain(std::iter::once((
                    format!("{OSM_MAP}?bbox={w:.6},{s:.6},{e:.6},{n:.6}"),
                    None,
                )))
                .collect();
        }
    }
    if opts.orthophoto {
        let px = ((b[2] - b[0]) / 0.25)
            .ceil()
            .clamp(64.0, ORTHO_PIXELS as f64) as u32;
        out.push(Request {
            bbox: Some(b),
            attempts: Vec::new(),
            tile: None,
            source: Source::Orthophoto,
            name: "pnoa.png".into(),
            url: format!(
                "{PNOA}?SERVICE=WMS&VERSION=1.3.0&REQUEST=GetMap&LAYERS=OI.OrthoimageCoverage&STYLES=&CRS=EPSG:{code}&BBOX={:.0},{:.0},{:.0},{:.0}&WIDTH={px}&HEIGHT={px}&FORMAT=image/png",
                b[0], b[1], b[2], b[3]
            ),
        });
    }
    // The surface model, with the terrain: the height of what stands on
    // it gives the floors of the buildings (brought or already there).
    if opts.terrain {
        let (nx, ny) = (
            ((b[2] - b[0]) / TERRAIN_CELL).ceil().max(1.0) as u32,
            ((b[3] - b[1]) / TERRAIN_CELL).ceil().max(1.0) as u32,
        );
        let v1 = format!(
            "{IGN_SURFACE}?SERVICE=WCS&VERSION=1.0.0&REQUEST=GetCoverage&COVERAGE={SURFACE_COVERAGE}&CRS=EPSG:{code}&BBOX={:.0},{:.0},{:.0},{:.0}&WIDTH={nx}&HEIGHT={ny}&FORMAT=application/asc",
            b[0], b[1], b[2], b[3]
        );
        let v2 = format!(
            "{IGN_SURFACE}?SERVICE=WCS&VERSION=2.0.1&REQUEST=GetCoverage&COVERAGEID={SURFACE_COVERAGE}&SUBSETTINGCRS=http://www.opengis.net/def/crs/EPSG/0/{code}&OUTPUTCRS=http://www.opengis.net/def/crs/EPSG/0/{code}&SUBSET=x({:.0},{:.0})&SUBSET=y({:.0},{:.0})&FORMAT=application/asc",
            b[0], b[2], b[1], b[3]
        );
        out.push(Request {
            bbox: Some(b),
            attempts: vec![(v1.clone(), None), (v2, None)],
            tile: None,
            source: Source::Surface,
            // Named as the CNIG names it, so that it is read as a surface.
            name: "MDS05.asc".into(),
            url: v1,
        });
    }
    if opts.terrain {
        for (x, y) in tiles_over(b, zone, north, TERRAIN_ZOOM) {
            out.push(Request {
                bbox: Some(b),
                attempts: Vec::new(),
                tile: Some(TerrainTile {
                    z: TERRAIN_ZOOM,
                    x,
                    y,
                    bbox: b,
                    zone,
                    north,
                }),
                source: Source::Terrain,
                name: format!("mdt-{TERRAIN_ZOOM}-{x}-{y}.png"),
                url: format!("{IGN_HEIGHTS}/{TERRAIN_ZOOM}/{x}/{y}.png"),
            });
        }
    }
    Ok(out)
}

/// Where a latitude and longitude fall on the web map at zoom `z`: its
/// column and row, with their fractions.
pub fn web_tile(lat: f64, lon: f64, z: u32) -> (f64, f64) {
    let n = 2f64.powi(z as i32);
    let x = (lon + 180.0) / 360.0 * n;
    let r = lat.to_radians();
    let y = (1.0 - (r.tan() + 1.0 / r.cos()).ln() / std::f64::consts::PI) / 2.0 * n;
    (x, y)
}

/// The tiles of the web map at zoom `z` that hold a rectangle of the map.
pub fn tiles_over(b: Bbox, zone: u32, north: bool, z: u32) -> Vec<(u32, u32)> {
    let corners = [
        utm_to_lat_lon(b[0], b[1], zone, north),
        utm_to_lat_lon(b[2], b[1], zone, north),
        utm_to_lat_lon(b[2], b[3], zone, north),
        utm_to_lat_lon(b[0], b[3], zone, north),
    ];
    let pts: Vec<(f64, f64)> = corners.iter().map(|c| web_tile(c.0, c.1, z)).collect();
    let x0 = pts.iter().map(|p| p.0).fold(f64::MAX, f64::min).floor() as u32;
    let x1 = pts.iter().map(|p| p.0).fold(f64::MIN, f64::max).floor() as u32;
    let y0 = pts.iter().map(|p| p.1).fold(f64::MAX, f64::min).floor() as u32;
    let y1 = pts.iter().map(|p| p.1).fold(f64::MIN, f64::max).floor() as u32;
    let mut out = Vec::new();
    for y in y0..=y1 {
        for x in x0..=x1 {
            out.push((x, y));
        }
    }
    out
}

/// The height a pixel of the IGN's tiles of heights holds, metres; `None`
/// for no data (black).
pub fn tile_height(rgb: [u8; 3]) -> Option<f64> {
    let v = (rgb[0] as u32) << 16 | (rgb[1] as u32) << 8 | rgb[2] as u32;
    (v != 0).then(|| -10_000.0 + v as f64 * 0.1)
}

/// The terrain of a rectangle of the map as an ESRI ASCII grid (what the
/// importer reads), from the tiles of heights that arrived: `pixel` gives
/// the colour of a tile `(x, y)` at a point of it (both from 0 to 1).
pub fn terrain_grid(
    t: &TerrainTile,
    pixel: impl Fn(u32, u32, f64, f64) -> Option<[u8; 3]>,
) -> Option<String> {
    let b = t.bbox;
    let cols = ((b[2] - b[0]) / TERRAIN_CELL).floor() as usize + 1;
    let rows = ((b[3] - b[1]) / TERRAIN_CELL).floor() as usize + 1;
    let mut text = format!(
        "ncols {cols}\nnrows {rows}\nxllcenter {:.2}\nyllcenter {:.2}\ncellsize {TERRAIN_CELL}\nNODATA_value -9999\n",
        b[0], b[1]
    );
    let mut any = false;
    for r in (0..rows).rev() {
        let mut row = Vec::with_capacity(cols);
        for c in 0..cols {
            let (e, n) = (
                b[0] + c as f64 * TERRAIN_CELL,
                b[1] + r as f64 * TERRAIN_CELL,
            );
            let (lat, lon) = utm_to_lat_lon(e, n, t.zone, t.north);
            let (tx, ty) = web_tile(lat, lon, t.z);
            let z = pixel(tx as u32, ty as u32, tx.fract(), ty.fract()).and_then(tile_height);
            any |= z.is_some();
            row.push(z.map_or("-9999".to_string(), |z| format!("{z:.2}")));
        }
        text.push_str(&row.join(" "));
        text.push('\n');
    }
    any.then_some(text)
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
        Source::Terrain | Source::Surface => {
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
        // The terrain: the tiles of heights that hold the rectangle (some
        // 1,2 km at 4,8 m a pixel: two or three a side).
        let t = of(Source::Terrain);
        assert!((4..=9).contains(&t.len()), "{}", t.len());
        assert!(
            t[0].url.starts_with(&format!("{IGN_HEIGHTS}/15/")),
            "{}",
            t[0].url
        );
        // Overpass by POST, its other servers for when it is busy, and
        // last OpenStreetMap's own map.
        let a = &of(Source::Osm)[0].attempts;
        assert_eq!(a.len(), OVERPASS_MIRRORS.len() + 2);
        assert!(a[0].0 == OVERPASS && a[0].1.as_ref().is_some_and(|f| f.starts_with("data=")));
        assert!(a.last().unwrap().0.starts_with(OSM_MAP) && a.last().unwrap().1.is_none());
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
        assert!(
            !few.is_empty()
                && few
                    .iter()
                    .all(|r| matches!(r.source, Source::Terrain | Source::Surface))
        );
        // The surface model of the same rectangle, by its WCS (5 m), named
        // so that it is read as a surface.
        let mds = few.iter().find(|r| r.source == Source::Surface).unwrap();
        assert!(crate::carto::is_surface_model(&mds.name));
        assert!(
            mds.url.starts_with(IGN_SURFACE)
                && mds.url.contains("BBOX=439850,4473850,440150,4474150")
                && mds.url.contains("WIDTH=60&HEIGHT=60"),
            "{}",
            mds.url
        );
        assert_eq!(mds.attempts.len(), 2);
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

    /// The tiles of heights become a grid of the rectangle: a pixel of
    /// R 1, G 134, B 160 is −10000 + 100000 · 0,1 = 0 m… plus 3,2 m.
    #[test]
    fn tiles_of_heights_make_the_terrain() {
        assert_eq!(tile_height([0, 0, 0]), None);
        let h = tile_height([1, 134, 192]).unwrap();
        assert!((h - (-10_000.0 + (65_536.0 + 134.0 * 256.0 + 192.0) * 0.1)).abs() < 1e-9);
        // Madrid, Puerta del Sol: tile 16046, 12355 at zoom 15.
        let (x, y) = web_tile(40.4168, -3.7038, 15);
        assert_eq!((x as u32, y as u32), (16046, 12355));
        let t = TerrainTile {
            z: 15,
            x: 0,
            y: 0,
            bbox: [440_000.0, 4_474_000.0, 440_020.0, 4_474_010.0],
            zone: 30,
            north: true,
        };
        // Everything at 12,5 m: (10000 + 12,5) / 0,1 = 100125.
        let v: u32 = 100_125;
        let rgb = [(v >> 16) as u8, (v >> 8) as u8, v as u8];
        let text = terrain_grid(&t, |_, _, _, _| Some(rgb)).unwrap();
        let g = archisim_carto::asc::read(&text).unwrap();
        assert_eq!((g.cols, g.rows), (5, 3));
        assert!((g.at(2, 1).unwrap() - 12.5).abs() < 1e-6);
        assert!(terrain_grid(&t, |_, _, _, _| None).is_none());
    }

    #[test]
    fn encoding_keeps_only_safe_letters() {
        assert_eq!(encode("a b\"[x]"), "a%20b%22%5Bx%5D");
    }
}
