//! Finding a place without its coordinates, for «Traer el entorno»: an
//! address, a site or a municipality (CartoCiudad, the IGN's geocoder for
//! Spain, and OpenStreetMap's Nominatim when it finds nothing), a
//! cadastral reference (the cadastre's OVCCoordenadas) or coordinates
//! pasted as they are. No network here: these are the addresses to ask
//! and the readers of what the services answer; [`search`] runs them
//! with the fetch it is given (the application's, on a thread).

use archisim_core::{Georeference, Model, Point2};

use crate::carto::{utm_of, Frame};
use crate::download::{encode, Bbox, PNOA};

/// CartoCiudad's geocoder: candidates for a text, and where one is.
pub const CARTOCIUDAD: &str = "https://www.cartociudad.es/geocoder/api/geocoder";
/// OpenStreetMap's geocoder (its policy: a User-Agent that names the
/// program and no more than one request a second).
pub const NOMINATIM: &str = "https://nominatim.openstreetmap.org/search";
/// The cadastre's coordinates of a cadastral reference.
pub const CADASTRE_COORDINATES: &str =
    "https://ovc.catastro.meh.es/ovcservweb/OVCSWLocalizacionRC/OVCCoordenadas.asmx/Consulta_CPMRC";
/// How many results are asked for.
pub const RESULTS: usize = 10;
/// How many CartoCiudad candidates without coordinates are looked up.
const FIND_AT_MOST: usize = 6;

/// What was written in the search field.
#[derive(Debug, Clone, PartialEq)]
pub enum Query {
    /// Latitude and longitude, degrees.
    Coordinates(f64, f64),
    /// A cadastral reference: the parcel's 14 characters, and whether a
    /// property's 20 were written.
    Cadastre { parcel: String, property: bool },
    /// An address, a site or a municipality.
    Text(String),
}

/// Who found a place.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Origin {
    CartoCiudad,
    Nominatim,
    Cadastre,
    /// The coordinates written.
    Written,
}

impl Origin {
    pub fn name(self) -> &'static str {
        match self {
            Origin::CartoCiudad => "CartoCiudad (IGN)",
            Origin::Nominatim => "OpenStreetMap (Nominatim)",
            Origin::Cadastre => "Catastro",
            Origin::Written => "Coordenadas escritas",
        }
    }
}

/// A place found.
#[derive(Debug, Clone, PartialEq)]
pub struct Place {
    /// The address or the name: «Calle Ancha, 12».
    pub name: String,
    /// The municipality and its province: «Punta Umbría (Huelva)».
    pub municipality: String,
    /// What it is: «Portal», «Calle», «Municipio»…
    pub kind: String,
    pub origin: Origin,
    /// Latitude and longitude (ETRS89 or WGS 84, the same here), degrees;
    /// none until `find` is asked.
    pub lat_lon: Option<(f64, f64)>,
    /// CartoCiudad's address that gives where it is, when the candidate
    /// did not say.
    pub find: Option<String>,
}

impl Place {
    /// One line for the list: name, municipality and kind.
    pub fn line(&self) -> String {
        let mut s = self.name.clone();
        if !self.municipality.is_empty() && !self.name.contains(&self.municipality) {
            s.push_str(" · ");
            s.push_str(&self.municipality);
        }
        s
    }

    /// Its latitude and longitude written as in Spain: «37,180000°, −6,930000°».
    pub fn coordinates_text(&self) -> String {
        match self.lat_lon {
            Some((lat, lon)) => format!("{lat:.6}°, {lon:.6}°").replace('.', ","),
            None => "sin coordenadas".into(),
        }
    }
}

/// Reads what was written: coordinates, a cadastral reference or a text
/// to look up. An empty field, a cadastral reference of the wrong length
/// or UTM coordinates are told in a sentence.
pub fn read_query(text: &str) -> Result<Query, String> {
    let t = text.trim();
    if t.is_empty() {
        return Err("Escribe una dirección, un sitio, un municipio, una referencia catastral o unas coordenadas.".into());
    }
    if let Some(c) = read_coordinates(t)? {
        return Ok(Query::Coordinates(c.0, c.1));
    }
    if let Some(rc) = cadastral_reference(t)? {
        return Ok(rc);
    }
    Ok(Query::Text(t.to_string()))
}

/// A cadastral reference, if the text looks like one: tokens of letters
/// and digits that all carry a digit (but the two control letters), with
/// at least five digits in all. Of 14 characters (a parcel) or 20 (a
/// property); any other length is an error.
fn cadastral_reference(t: &str) -> Result<Option<Query>, String> {
    let tokens: Vec<&str> = t.split_whitespace().collect();
    let looks = tokens.iter().all(|w| {
        w.chars().all(|c| c.is_ascii_alphanumeric() || c == '-')
            && (w.chars().any(|c| c.is_ascii_digit()) || w.len() <= 2)
    });
    let compact: String = t
        .chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .collect::<String>()
        .to_ascii_uppercase();
    let digits = compact.chars().filter(|c| c.is_ascii_digit()).count();
    let letters = compact.len() - digits;
    if !looks || digits < 5 || letters == 0 || !(10..=24).contains(&compact.len()) {
        return Ok(None);
    }
    match compact.len() {
        14 => Ok(Some(Query::Cadastre {
            parcel: compact,
            property: false,
        })),
        20 => Ok(Some(Query::Cadastre {
            parcel: compact[..14].to_string(),
            property: true,
        })),
        n => Err(format!(
            "«{t}» parece una referencia catastral, pero tiene {n} caracteres: la de una parcela tiene 14 y la de un inmueble 20 (sin contar los espacios)."
        )),
    }
}

/// Latitude and longitude, if the text is two numbers: «37.18, -6.93»,
/// «37,18, -6,93», «37,18 -6,93», «37.18;-6.93», «37,18,-6,93», with or
/// without «°» and N, S, E, W (O) after them. Longitude and latitude the
/// other way round are turned when only that falls in Spain. Two big
/// numbers (UTM) are an error.
fn read_coordinates(t: &str) -> Result<Option<(f64, f64)>, String> {
    let s: String = t.replace(['°', 'º'], " ");
    if !s
        .chars()
        .all(|c| c.is_ascii_digit() || " \t,.;-+−NSEWOnsewo".contains(c))
        || !s.chars().any(|c| c.is_ascii_digit())
    {
        return Ok(None);
    }
    let s = s.replace('−', "-");
    // Tokens split by blanks and semicolons; a comma that ends a token is
    // a separator.
    let mut parts: Vec<String> = s
        .split(|c: char| c.is_whitespace() || c == ';')
        .map(|w| w.trim_end_matches(',').to_string())
        .filter(|w| !w.is_empty())
        .collect();
    // Hemisphere letters written apart («37.18 N 6.93 W»).
    let mut joined: Vec<String> = Vec::new();
    for p in parts.drain(..) {
        if p.chars().all(|c| c.is_ascii_alphabetic()) {
            match joined.last_mut() {
                Some(last) => last.push_str(&p),
                None => return Ok(None),
            }
        } else {
            joined.push(p);
        }
    }
    let mut parts = joined;
    if parts.len() == 1 {
        let one = parts.remove(0);
        let commas = one.matches(',').count();
        parts = if one.contains('.') || commas == 1 {
            one.split(',').map(str::to_string).collect()
        } else if commas == 3 {
            let p: Vec<&str> = one.split(',').collect();
            vec![format!("{}.{}", p[0], p[1]), format!("{}.{}", p[2], p[3])]
        } else {
            return Ok(None);
        };
    }
    if parts.len() != 2 {
        return Ok(None);
    }
    let number = |w: &str| -> Option<f64> {
        let w = w.trim();
        let (body, sign) = match w.chars().last()? {
            'N' | 'n' | 'E' | 'e' => (&w[..w.len() - 1], 1.0),
            'S' | 's' | 'W' | 'w' | 'O' | 'o' => (&w[..w.len() - 1], -1.0),
            _ => (w, 1.0),
        };
        body.trim()
            .replace(',', ".")
            .parse::<f64>()
            .ok()
            .map(|v| v * sign)
    };
    let (Some(a), Some(b)) = (number(&parts[0]), number(&parts[1])) else {
        return Ok(None);
    };
    if a.abs() > 1000.0 && b.abs() > 1000.0 {
        return Err("Parecen coordenadas UTM. Escribe la latitud y la longitud (por ejemplo «37,18, -6,93», como las copia un mapa web), o pon las UTM en el emplazamiento del proyecto.".into());
    }
    let spain = |lat: f64, lon: f64| (27.0..44.5).contains(&lat) && (-19.0..5.0).contains(&lon);
    let (lat, lon) = if !spain(a, b) && spain(b, a) {
        (b, a)
    } else {
        (a, b)
    };
    if lat.abs() > 90.0 || lon.abs() > 180.0 {
        return Err(format!(
            "«{t}» no son una latitud y una longitud: la latitud va de -90 a 90 y la longitud de -180 a 180."
        ));
    }
    Ok(Some((lat, lon)))
}

/// CartoCiudad's candidates for a text.
pub fn cartociudad_url(text: &str) -> String {
    format!(
        "{CARTOCIUDAD}/candidates?q={}&limit={RESULTS}",
        encode(text)
    )
}

/// Nominatim's places for a text.
pub fn nominatim_url(text: &str) -> String {
    format!(
        "{NOMINATIM}?q={}&format=jsonv2&addressdetails=1&limit={RESULTS}&accept-language=es",
        encode(text)
    )
}

/// The cadastre's coordinates of a parcel, in ETRS89 latitude and
/// longitude.
pub fn cadastre_url(parcel: &str) -> String {
    format!(
        "{CADASTRE_COORDINATES}?Provincia=&Municipio=&SRS=EPSG:4258&RC={}",
        encode(parcel)
    )
}

/// A JSON answer, without the callback round it if it came as JSONP.
fn json_of(text: &str) -> Result<serde_json::Value, String> {
    let t = text.trim();
    let t = if !t.starts_with(['[', '{']) {
        match (t.find('('), t.rfind(')')) {
            (Some(a), Some(b)) if a < b => &t[a + 1..b],
            _ => t,
        }
    } else {
        t
    };
    if t.is_empty() {
        return Ok(serde_json::Value::Null);
    }
    serde_json::from_str(t).map_err(|_| {
        let start: String = t.chars().take(80).collect();
        format!("la respuesta no se entiende («{}»)", start.trim())
    })
}

fn number(v: &serde_json::Value) -> Option<f64> {
    match v {
        serde_json::Value::Number(n) => n.as_f64(),
        serde_json::Value::String(s) => s.trim().replace(',', ".").parse().ok(),
        _ => None,
    }
    .filter(|x: &f64| x.is_finite())
}

fn text_of<'a>(v: &'a serde_json::Value, key: &str) -> &'a str {
    v.get(key).and_then(|x| x.as_str()).unwrap_or("").trim()
}

/// A name as people write it: «CALLE ANCHA» → «Calle Ancha», with «de»,
/// «del», «la»… in small letters (but the first word). Names that are
/// not all in capitals are kept.
pub fn as_written(s: &str) -> String {
    if s.chars().any(|c| c.is_lowercase()) {
        return s.to_string();
    }
    let small = [
        "de", "del", "la", "las", "los", "el", "y", "e", "en", "a", "al", "o",
    ];
    s.split(' ')
        .enumerate()
        .map(|(i, w)| {
            let low = w.to_lowercase();
            if i > 0 && small.contains(&low.as_str()) {
                low
            } else {
                let mut c = low.chars();
                match c.next() {
                    Some(f) => f.to_uppercase().chain(c).collect(),
                    None => String::new(),
                }
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// What a CartoCiudad type is, in a word.
fn cartociudad_kind(t: &str) -> &'static str {
    match t.to_lowercase().as_str() {
        "portal" => "Portal",
        "callejero" => "Calle",
        "carretera" | "carreteras" => "Carretera",
        "municipio" => "Municipio",
        "poblacion" | "población" => "Población",
        "provincia" => "Provincia",
        "toponimo" | "topónimo" | "ngbe" => "Topónimo",
        "codpost" => "Código postal",
        "refcatastral" => "Referencia catastral",
        "expendeduria" | "expendeduría" => "Estanco",
        "punto_recarga_electrica" => "Punto de recarga",
        _ => "Lugar",
    }
}

/// The places of CartoCiudad's candidates (a JSON list, or JSONP). Those
/// without coordinates (streets, municipalities) carry the address that
/// gives them (`find`).
pub fn read_cartociudad(text: &str) -> Result<Vec<Place>, String> {
    let v = json_of(text)?;
    let list = match &v {
        serde_json::Value::Array(a) => a.as_slice(),
        serde_json::Value::Null => &[],
        serde_json::Value::Object(_) => std::slice::from_ref(&v),
        _ => return Err("la respuesta no es una lista de lugares".into()),
    };
    let mut out = Vec::new();
    for c in list {
        let kind_code = text_of(c, "type");
        let address = text_of(c, "address");
        let muni = text_of(c, "muni");
        let province = text_of(c, "province");
        if address.is_empty() && muni.is_empty() {
            continue;
        }
        let mut name = as_written(if address.is_empty() { muni } else { address });
        let road = as_written(text_of(c, "tip_via"));
        if !road.is_empty() && !name.to_lowercase().starts_with(&road.to_lowercase()) {
            name = format!("{road} {name}");
        }
        let portal = c.get("portalNumber").and_then(number).unwrap_or(0.0);
        if portal > 0.0 && kind_code.eq_ignore_ascii_case("portal") {
            name.push_str(&format!(", {}", portal as i64));
            let ext = text_of(c, "extension");
            if !ext.is_empty() {
                name.push(' ');
                name.push_str(ext);
            }
        }
        let municipality = match (muni.is_empty(), province.is_empty()) {
            (false, false) if !muni.eq_ignore_ascii_case(province) => {
                format!("{} ({})", as_written(muni), as_written(province))
            }
            (false, _) => as_written(muni),
            (true, false) => as_written(province),
            (true, true) => String::new(),
        };
        let lat_lon = match (c.get("lat").and_then(number), c.get("lng").and_then(number)) {
            (Some(lat), Some(lng)) if lat != 0.0 || lng != 0.0 => Some((lat, lng)),
            _ => None,
        };
        let id = c.get("id").map_or(String::new(), |x| match x {
            serde_json::Value::String(s) => s.clone(),
            other => other.to_string(),
        });
        let find = (lat_lon.is_none() && !id.is_empty()).then(|| {
            let mut u = format!(
                "{CARTOCIUDAD}/find?id={}&type={}",
                encode(&id),
                encode(kind_code)
            );
            if portal > 0.0 {
                u.push_str(&format!("&portal={}", portal as i64));
            }
            u
        });
        out.push(Place {
            name,
            municipality,
            kind: cartociudad_kind(kind_code).into(),
            origin: Origin::CartoCiudad,
            lat_lon,
            find,
        });
    }
    Ok(out)
}

/// Where CartoCiudad's `find` says a candidate is: its `lat` and `lng`.
pub fn read_cartociudad_find(text: &str) -> Option<(f64, f64)> {
    let v = json_of(text).ok()?;
    let v = match &v {
        serde_json::Value::Array(a) => a.first()?.clone(),
        _ => v,
    };
    let lat = v.get("lat").and_then(number)?;
    let lng = v.get("lng").and_then(number)?;
    (lat != 0.0 || lng != 0.0).then_some((lat, lng))
}

/// What a Nominatim class is, in a word.
fn nominatim_kind(category: &str, kind: &str) -> String {
    let k = match (category, kind) {
        (_, "house") | ("building", _) => "Edificio",
        ("highway", _) => "Calle",
        ("place", "city" | "town" | "village" | "hamlet") => "Población",
        ("place", "suburb" | "neighbourhood" | "quarter") => "Barrio",
        ("boundary", "administrative") => "Municipio",
        ("amenity", _) => "Equipamiento",
        ("tourism", _) | ("historic", _) => "Lugar de interés",
        ("leisure", _) => "Ocio",
        ("natural", _) => "Paraje",
        ("shop", _) => "Comercio",
        _ => "Lugar",
    };
    k.to_string()
}

/// The places of Nominatim's answer (`format=jsonv2&addressdetails=1`).
pub fn read_nominatim(text: &str) -> Result<Vec<Place>, String> {
    let v = json_of(text)?;
    let Some(list) = v.as_array() else {
        return match v {
            serde_json::Value::Null => Ok(Vec::new()),
            _ => Err("la respuesta no es una lista de lugares".into()),
        };
    };
    let mut out = Vec::new();
    for p in list {
        let (Some(lat), Some(lon)) = (p.get("lat").and_then(number), p.get("lon").and_then(number))
        else {
            continue;
        };
        let a = p.get("address");
        let field = |k: &str| a.map_or("", |a| text_of(a, k));
        let road = field("road");
        let number_ = field("house_number");
        let name = text_of(p, "name");
        let display = text_of(p, "display_name");
        let label = if !road.is_empty() {
            let mut s = road.to_string();
            if !number_.is_empty() {
                s.push_str(", ");
                s.push_str(number_);
            }
            if !name.is_empty() && name != road {
                s = format!("{name} · {s}");
            }
            s
        } else if !name.is_empty() {
            name.to_string()
        } else {
            display.split(',').next().unwrap_or("").trim().to_string()
        };
        let town = ["city", "town", "village", "municipality", "hamlet"]
            .iter()
            .map(|k| field(k))
            .find(|s| !s.is_empty())
            .unwrap_or("");
        let region = ["province", "county", "state"]
            .iter()
            .map(|k| field(k))
            .find(|s| !s.is_empty())
            .unwrap_or("");
        let municipality = match (town.is_empty(), region.is_empty()) {
            (false, false) if town != region => format!("{town} ({region})"),
            (false, _) => town.to_string(),
            (true, _) => region.to_string(),
        };
        let category = p
            .get("category")
            .or_else(|| p.get("class"))
            .and_then(|x| x.as_str())
            .unwrap_or("");
        out.push(Place {
            name: label,
            municipality,
            kind: nominatim_kind(category, text_of(p, "type")),
            origin: Origin::Nominatim,
            lat_lon: Some((lat, lon)),
            find: None,
        });
    }
    Ok(out)
}

/// The places of the cadastre's answer to `Consulta_CPMRC` (asked in
/// latitude and longitude), or what it says is wrong.
pub fn read_cadastre(text: &str) -> Result<Vec<Place>, String> {
    use archisim_carto::xml::{events, local, Event};
    let mut path: Vec<&str> = Vec::new();
    let mut out = Vec::new();
    let (mut x, mut y, mut ldt) = (None, None, String::new());
    let mut pc = (String::new(), String::new());
    let mut error = String::new();
    let mut srs = String::new();
    for e in events(text) {
        match e {
            Event::Start { name, empty, .. } => {
                if !empty {
                    path.push(local(name));
                }
                if local(name) == "coord" {
                    (x, y, ldt) = (None, None, String::new());
                    pc = (String::new(), String::new());
                }
            }
            Event::End(name) => {
                if local(name) == "coord" {
                    if let (Some(lon), Some(lat)) = (x, y) {
                        out.push((lat, lon, ldt.clone(), format!("{}{}", pc.0, pc.1)));
                    }
                }
                path.pop();
            }
            Event::Text(t) => {
                let t = t.trim().to_string();
                match path.last().copied() {
                    Some("xcen") => x = t.replace(',', ".").parse().ok(),
                    Some("ycen") => y = t.replace(',', ".").parse().ok(),
                    Some("ldt") => ldt = t,
                    Some("pc1") => pc.0 = t,
                    Some("pc2") => pc.1 = t,
                    Some("srs") => srs = t,
                    Some("des") if path.contains(&"err") => {
                        if !error.is_empty() {
                            error.push_str("; ");
                        }
                        error.push_str(&t);
                    }
                    _ => {}
                }
            }
        }
    }
    if out.is_empty() {
        if !error.is_empty() {
            return Err(format!("el Catastro responde: «{}»", sentence(&error)));
        }
        if text.trim().is_empty() || !text.contains("consulta_coordenadas") {
            return Err("la respuesta del Catastro no se entiende".into());
        }
        return Err("el Catastro no da coordenadas para esa referencia".into());
    }
    if !srs.is_empty() && !srs.contains("4258") && !srs.contains("4326") {
        return Err(format!(
            "el Catastro ha respondido en {srs}, no en latitud y longitud"
        ));
    }
    Ok(out
        .into_iter()
        .map(|(lat, lon, ldt, rc)| {
            // «CL ANCHA 12 PUNTA UMBRIA (HUELVA)»: the address and, in
            // brackets, the province.
            let (address, province) = match (ldt.rfind('('), ldt.rfind(')')) {
                (Some(a), Some(b)) if a < b => (
                    ldt[..a].trim().to_string(),
                    ldt[a + 1..b].trim().to_string(),
                ),
                _ => (ldt.clone(), String::new()),
            };
            Place {
                name: if address.is_empty() {
                    format!("Parcela {rc}")
                } else {
                    as_written(&street_type(&address))
                },
                municipality: as_written(&province),
                kind: format!("Parcela {rc}"),
                origin: Origin::Cadastre,
                lat_lon: Some((lat, lon)),
                find: None,
            }
        })
        .collect())
}

/// The cadastre's address with its kind of street written whole:
/// «CL ANCHA 12» → «CALLE ANCHA 12».
fn street_type(address: &str) -> String {
    let Some((first, rest)) = address.split_once(' ') else {
        return address.to_string();
    };
    let whole = match first {
        "CL" => "CALLE",
        "AV" => "AVENIDA",
        "PZ" => "PLAZA",
        "PS" => "PASEO",
        "CR" => "CARRETERA",
        "CM" => "CAMINO",
        "TR" => "TRAVESÍA",
        "PJ" => "PASAJE",
        "UR" => "URBANIZACIÓN",
        "BO" => "BARRIO",
        "LG" => "LUGAR",
        "DS" => "DISEMINADO",
        "PG" => "POLÍGONO",
        "GL" => "GLORIETA",
        "RD" => "RONDA",
        _ => return address.to_string(),
    };
    format!("{whole} {rest}")
}

/// «LA REFERENCIA CATASTRAL NO EXISTE» → «La referencia catastral no existe».
fn sentence(s: &str) -> String {
    if s.chars().any(|c| c.is_lowercase()) {
        return s.to_string();
    }
    let low = s.to_lowercase();
    let mut c = low.chars();
    match c.next() {
        Some(f) => f.to_uppercase().chain(c).collect(),
        None => String::new(),
    }
}

/// Looks a place up, asking with `fetch` (an address → its bytes, or why
/// not): coordinates are taken as they are; a cadastral reference is
/// asked to the cadastre; a text to CartoCiudad and, if it finds nothing
/// or does not answer, to Nominatim. Every failure is a sentence in
/// Spanish for the window.
pub fn search(
    text: &str,
    fetch: &mut dyn FnMut(&str) -> Result<Vec<u8>, String>,
) -> Result<Vec<Place>, String> {
    let get = |fetch: &mut dyn FnMut(&str) -> Result<Vec<u8>, String>, url: &str| {
        fetch(url).map(|b| String::from_utf8_lossy(&b).into_owned())
    };
    match read_query(text)? {
        Query::Coordinates(lat, lon) => Ok(vec![Place {
            name: format!("{lat:.6}°, {lon:.6}°").replace('.', ","),
            municipality: String::new(),
            kind: "Punto".into(),
            origin: Origin::Written,
            lat_lon: Some((lat, lon)),
            find: None,
        }]),
        Query::Cadastre { parcel, property } => {
            let answer = get(fetch, &cadastre_url(&parcel)).map_err(|e| {
                format!("No se ha podido preguntar al Catastro por la referencia {parcel}: {e}. Comprueba la conexión, o escribe una dirección o unas coordenadas.")
            })?;
            let mut places = read_cadastre(&answer).map_err(|e| {
                format!("No se encuentra la referencia catastral {parcel}: {e}. Comprueba que está bien escrita (14 caracteres la de una parcela, 20 la de un inmueble).")
            })?;
            if property {
                for p in &mut places {
                    p.kind = format!("{} (del inmueble)", p.kind);
                }
            }
            Ok(places)
        }
        Query::Text(t) => {
            let mut why = Vec::new();
            let mut places =
                match get(fetch, &cartociudad_url(&t)).and_then(|a| read_cartociudad(&a)) {
                    Ok(p) => p,
                    Err(e) => {
                        why.push(format!("CartoCiudad: {e}"));
                        Vec::new()
                    }
                };
            // Where the streets and municipalities are.
            let mut asked = 0;
            for p in &mut places {
                if p.lat_lon.is_some() || asked >= FIND_AT_MOST {
                    continue;
                }
                if let Some(url) = p.find.clone() {
                    asked += 1;
                    p.lat_lon = get(fetch, &url)
                        .ok()
                        .and_then(|a| read_cartociudad_find(&a));
                }
            }
            places.retain(|p| p.lat_lon.is_some());
            if places.is_empty() {
                match get(fetch, &nominatim_url(&t)).and_then(|a| read_nominatim(&a)) {
                    Ok(p) => places = p,
                    Err(e) => why.push(format!("OpenStreetMap: {e}")),
                }
            }
            if !places.is_empty() {
                return Ok(places);
            }
            if why.len() == 2 {
                Err(format!(
                    "No se ha podido buscar «{t}» ({}). Comprueba la conexión a internet, o escribe unas coordenadas (por ejemplo «37,18, -6,93»).",
                    why.join("; ")
                ))
            } else {
                Err(format!(
                    "No se ha encontrado «{t}». Prueba con la calle y el municipio («Calle Ancha 12, Punta Umbría»), solo el municipio, una referencia catastral o unas coordenadas."
                ))
            }
        }
    }
}

/// A georeference with the project's origin at a place.
pub fn georeference_at(place: &Place) -> Option<Georeference> {
    let (lat, lon) = place.lat_lon?;
    Some(crate::carto::georeference_at(lat, lon))
}

/// Where a latitude and longitude fall in the plan of a georeferenced
/// project (UTM), by its georeference and its north.
pub fn plan_point(m: &Model, lat: f64, lon: f64) -> Option<Point2> {
    let g = m.georeference()?;
    g.utm_zone()?;
    Some(Frame::of(m, g).to_plan(utm_of(g, lat, lon)))
}

/// Latitude and longitude of a point of the plan of a georeferenced
/// project (UTM).
pub fn lat_lon_of(m: &Model, p: Point2) -> Option<(f64, f64)> {
    let g = m.georeference()?;
    let (zone, north) = g.utm_zone()?;
    let c = Frame::of(m, g).to_map(p);
    Some(archisim_core::utm_to_lat_lon(c[0], c[1], zone, north))
}

/// Pixels a side of the little map of the window.
pub const PREVIEW_PIXELS: u32 = 320;

/// The little map round a place: the PNOA's orthophoto of a square of
/// `half` metres each way, in the UTM zone of the place. Its address and
/// rectangle (west, south, east, north, in that zone's system).
pub fn preview(lat: f64, lon: f64, half: f64) -> (String, Bbox) {
    let g = crate::carto::georeference_at(lat, lon);
    let (zone, north) = g.utm_zone().unwrap_or((30, true));
    let (e, n) = archisim_carto::lat_lon_to_utm(lat, lon, zone, north);
    let b = [e - half, n - half, e + half, n + half];
    (
        format!(
            "{PNOA}?SERVICE=WMS&VERSION=1.3.0&REQUEST=GetMap&LAYERS=OI.OrthoimageCoverage&STYLES=&CRS={}&BBOX={:.0},{:.0},{:.0},{:.0}&WIDTH={PREVIEW_PIXELS}&HEIGHT={PREVIEW_PIXELS}&FORMAT=image/png",
            g.crs, b[0], b[1], b[2], b[3]
        ),
        b,
    )
}

/// Where a latitude and longitude fall in a little map of [`preview`]
/// made round (`lat0`, `lon0`): from 0 to 1 across, and from the top.
pub fn preview_spot(lat0: f64, lon0: f64, b: Bbox, lat: f64, lon: f64) -> (f64, f64) {
    let g = crate::carto::georeference_at(lat0, lon0);
    let (zone, north) = g.utm_zone().unwrap_or((30, true));
    let (e, n) = archisim_carto::lat_lon_to_utm(lat, lon, zone, north);
    ((e - b[0]) / (b[2] - b[0]), (b[3] - n) / (b[3] - b[1]))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn coordinates_are_read_as_people_paste_them() {
        for t in [
            "37.18, -6.93",
            "37,18, -6,93",
            "37,18 -6,93",
            "37.18;-6.93",
            "37.18,-6.93",
            "37,18,-6,93",
            "37.18° N, 6.93° W",
            "37,18N 6,93O",
            " 37.18  -6.93 ",
            // The other way round: only that falls in Spain.
            "-6.93, 37.18",
        ] {
            match read_query(t) {
                Ok(Query::Coordinates(lat, lon)) => {
                    assert!(
                        (lat - 37.18).abs() < 1e-9 && (lon + 6.93).abs() < 1e-9,
                        "{t}"
                    )
                }
                other => panic!("{t}: {other:?}"),
            }
        }
        // UTM, and out of range.
        assert!(read_query("682000, 4115000").is_err());
        assert!(read_query("95.0, 200.0").is_err());
        // A street number alone is not coordinates.
        assert!(matches!(read_query("Calle Ancha 12"), Ok(Query::Text(_))));
        assert!(read_query("   ").is_err());
    }

    #[test]
    fn cadastral_references_are_known_by_their_shape() {
        assert_eq!(
            read_query("1234567AB1234C").unwrap(),
            Query::Cadastre {
                parcel: "1234567AB1234C".into(),
                property: false
            }
        );
        // With spaces and small letters; a property's 20 give its parcel.
        assert_eq!(
            read_query("1234567 ab1234c 0001 xy").unwrap(),
            Query::Cadastre {
                parcel: "1234567AB1234C".into(),
                property: true
            }
        );
        // A rural one.
        assert!(matches!(
            read_query("21099A00200017").unwrap(),
            Query::Cadastre { .. }
        ));
        // One character short: said, not looked up as an address.
        let e = read_query("1234567AB1234").unwrap_err();
        assert!(e.contains("13 caracteres") && e.contains("14"), "{e}");
        // Words with numbers are addresses.
        assert!(matches!(
            read_query("Avenida 28 de Febrero 12345").unwrap(),
            Query::Text(_)
        ));
    }

    // Answers made up for these tests, in the services' shape.
    const CARTO: &str = r#"[
      {"id":"210600000123","province":"HUELVA","comunidadAutonoma":"Andalucía","muni":"PUNTA UMBRÍA","type":"portal","address":"ANCHA","postalCode":"21100","poblacion":"Punta Umbría","geom":null,"tip_via":"CALLE","lat":37.1801,"lng":-6.9662,"portalNumber":12,"noNumber":false,"stateMsg":"","extension":"","state":0,"countryCode":"011","refCatastral":null},
      {"id":"21060000077","province":"HUELVA","muni":"PUNTA UMBRÍA","type":"callejero","address":"ANCHA DE LA PLAYA","tip_via":"AVENIDA","lat":null,"lng":null,"portalNumber":0},
      {"id":"21060","province":"HUELVA","muni":"PUNTA UMBRÍA","type":"Municipio","address":"PUNTA UMBRÍA","tip_via":null,"portalNumber":0}
    ]"#;

    #[test]
    fn cartociudad_candidates_are_read() {
        let p = read_cartociudad(CARTO).unwrap();
        assert_eq!(p.len(), 3);
        assert_eq!(p[0].name, "Calle Ancha, 12");
        assert_eq!(p[0].municipality, "Punta Umbría (Huelva)");
        assert_eq!(p[0].kind, "Portal");
        assert_eq!(p[0].lat_lon, Some((37.1801, -6.9662)));
        assert_eq!(p[1].name, "Avenida Ancha de la Playa");
        assert_eq!(p[1].kind, "Calle");
        assert!(p[1].lat_lon.is_none());
        let find = p[1].find.as_deref().unwrap();
        assert!(
            find.contains("/find?id=21060000077&type=callejero"),
            "{find}"
        );
        assert_eq!(p[2].name, "Punta Umbría");
        assert_eq!(p[2].kind, "Municipio");
        // JSONP too, and nothing found.
        assert_eq!(
            read_cartociudad(&format!("callback({CARTO})"))
                .unwrap()
                .len(),
            3
        );
        assert!(read_cartociudad("[]").unwrap().is_empty());
        assert!(read_cartociudad("<html>error</html>").is_err());
        assert_eq!(
            read_cartociudad_find(r#"{"id":"21060000077","lat":37.1795,"lng":-6.9601}"#),
            Some((37.1795, -6.9601))
        );
        assert_eq!(read_cartociudad_find("{}"), None);
    }

    const NOMI: &str = r#"[{"place_id":1,"lat":"37.1810","lon":"-6.9655","category":"amenity","type":"townhall","name":"Ayuntamiento","display_name":"Ayuntamiento, Plaza Pérez Pastor, Punta Umbría, Huelva, Andalucía, España","address":{"amenity":"Ayuntamiento","road":"Plaza Pérez Pastor","town":"Punta Umbría","province":"Huelva","state":"Andalucía","country":"España"}},
      {"place_id":2,"lat":"40.4168","lon":"-3.7038","category":"boundary","type":"administrative","name":"Madrid","display_name":"Madrid, Comunidad de Madrid, España","address":{"city":"Madrid","state":"Comunidad de Madrid"}}]"#;

    #[test]
    fn nominatim_places_are_read() {
        let p = read_nominatim(NOMI).unwrap();
        assert_eq!(p.len(), 2);
        assert_eq!(p[0].name, "Ayuntamiento · Plaza Pérez Pastor");
        assert_eq!(p[0].municipality, "Punta Umbría (Huelva)");
        assert_eq!(p[0].kind, "Equipamiento");
        assert_eq!(p[0].lat_lon, Some((37.181, -6.9655)));
        assert_eq!(p[1].name, "Madrid");
        assert_eq!(p[1].kind, "Municipio");
        assert!(read_nominatim("[]").unwrap().is_empty());
    }

    const RC_OK: &str = r#"<?xml version="1.0" encoding="utf-8"?>
<consulta_coordenadas xmlns="http://www.catastro.meh.es/"><control><cucoor>1</cucoor><cuerr>0</cuerr></control>
<coordenadas><coord><pc><pc1>1234567</pc1><pc2>AB1234C</pc2></pc><geo><xcen>-6.9650</xcen><ycen>37.1805</ycen><srs>EPSG:4258</srs></geo><ldt>CL ANCHA 12 PUNTA UMBRIA (HUELVA)</ldt></coord></coordenadas></consulta_coordenadas>"#;
    const RC_BAD: &str = r#"<?xml version="1.0" encoding="utf-8"?>
<consulta_coordenadas xmlns="http://www.catastro.meh.es/"><control><cuerr>1</cuerr></control><lerr><err><cod>11</cod><des>LA REFERENCIA CATASTRAL NO EXISTE</des></err></lerr></consulta_coordenadas>"#;

    #[test]
    fn the_cadastre_gives_a_parcel_or_says_why_not() {
        let p = read_cadastre(RC_OK).unwrap();
        assert_eq!(p.len(), 1);
        assert_eq!(p[0].name, "Calle Ancha 12 Punta Umbria");
        assert_eq!(p[0].municipality, "Huelva");
        assert_eq!(p[0].kind, "Parcela 1234567AB1234C");
        assert_eq!(p[0].lat_lon, Some((37.1805, -6.965)));
        let e = read_cadastre(RC_BAD).unwrap_err();
        assert!(e.contains("La referencia catastral no existe"), "{e}");
        assert!(read_cadastre("").is_err());
        assert!(cadastre_url("1234567AB1234C").ends_with("SRS=EPSG:4258&RC=1234567AB1234C"));
    }

    #[test]
    fn a_search_asks_in_turn_and_says_what_failed() {
        // CartoCiudad finds it; the street without coordinates is asked
        // with `find`.
        let mut asked = Vec::new();
        let p = search("Calle Ancha 12, Punta Umbría", &mut |url: &str| {
            asked.push(url.to_string());
            if url.contains("/candidates?") {
                Ok(CARTO.as_bytes().to_vec())
            } else if url.contains("/find?") && url.contains("callejero") {
                Ok(br#"{"lat":37.1795,"lng":-6.9601}"#.to_vec())
            } else {
                Ok(b"{}".to_vec())
            }
        })
        .unwrap();
        assert_eq!(
            p.len(),
            2,
            "the municipality without coordinates is left out"
        );
        assert!(asked[0].contains("q=Calle%20Ancha%2012%2C%20Punta%20Umbr%C3%ADa"));
        assert!(!asked.iter().any(|u| u.contains("nominatim")));
        // Nothing in CartoCiudad: Nominatim.
        let p = search("Ayuntamiento de Punta Umbría", &mut |url: &str| {
            Ok(if url.contains("nominatim") {
                NOMI.as_bytes().to_vec()
            } else {
                b"[]".to_vec()
            })
        })
        .unwrap();
        assert_eq!(p[0].origin, Origin::Nominatim);
        // Nothing anywhere, and no network.
        let e = search("Xyzzy", &mut |_: &str| Ok(b"[]".to_vec())).unwrap_err();
        assert!(e.contains("No se ha encontrado «Xyzzy»"), "{e}");
        let e = search("Xyzzy", &mut |_: &str| {
            Err("no se llega al servicio (Dns Failed)".into())
        })
        .unwrap_err();
        assert!(e.contains("Comprueba la conexión"), "{e}");
        // Coordinates need no network.
        let p = search("37,18, -6,93", &mut |_: &str| panic!("no network")).unwrap();
        assert_eq!(p[0].origin, Origin::Written);
        // The cadastre, and its refusal.
        let p = search("1234567AB1234C", &mut |url: &str| {
            assert!(url.starts_with(CADASTRE_COORDINATES));
            Ok(RC_OK.as_bytes().to_vec())
        })
        .unwrap();
        assert_eq!(p[0].origin, Origin::Cadastre);
        let e = search("1234567AB1234C", &mut |_: &str| {
            Ok(RC_BAD.as_bytes().to_vec())
        })
        .unwrap_err();
        assert!(e.contains("no existe") && e.contains("bien escrita"), "{e}");
        let e = search("1234567AB1234C", &mut |_: &str| Err("sin red".into())).unwrap_err();
        assert!(e.contains("No se ha podido preguntar al Catastro"), "{e}");
    }

    #[test]
    fn a_place_georeferences_a_project_or_falls_in_its_plan() {
        let place = &search("37.18, -6.93", &mut |_: &str| Err(String::new())).unwrap()[0];
        let g = georeference_at(place).unwrap();
        assert_eq!(g.crs, "EPSG:25829");
        let mut m = Model::new("T");
        m.set_georeference(Some(g.clone())).unwrap();
        // The origin is the place (to the metre).
        let p = plan_point(&m, 37.18, -6.93).unwrap();
        assert!(p.x.abs() < 1.0 && p.y.abs() < 1.0, "{p:?}");
        // 0,001° north is some 111 m up the plan (north is up).
        let q = plan_point(&m, 37.181, -6.93).unwrap();
        assert!((q.y - 111.0).abs() < 2.0 && q.x.abs() < 5.0, "{q:?}");
        let (lat, lon) = lat_lon_of(&m, q).unwrap();
        assert!((lat - 37.181).abs() < 1e-6 && (lon + 6.93).abs() < 1e-6);
        // The little map: the place in its middle.
        let (url, b) = preview(37.18, -6.93, 200.0);
        assert!(url.contains("CRS=EPSG:25829") && url.contains("WIDTH=320"));
        let (u, v) = preview_spot(37.18, -6.93, b, 37.18, -6.93);
        assert!((u - 0.5).abs() < 1e-6 && (v - 0.5).abs() < 1e-6);
    }
}
