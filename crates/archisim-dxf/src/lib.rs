//! Reading DXF drawings (the ASCII format every CAD program writes), for
//! what ArchiSim takes from them: the points, lines and texts of a
//! topographic survey, with their layers. No dependencies; knows nothing
//! of the model.
//!
//! A DXF file is a list of pairs: a group code on one line and its value
//! on the next. Only the model space entities of the `ENTITIES` section are
//! read: `POINT`, `LINE`, `LWPOLYLINE` (with its elevation), `POLYLINE`
//! with its `VERTEX`es (2D with elevation, or 3D), `3DFACE`, `INSERT` (its
//! insertion point and the `ATTRIB`s after it), `TEXT` and `MTEXT`. Blocks
//! are not exploded: a survey point drawn as a block is its insertion
//! point. Coordinates are taken as world coordinates (the object
//! coordinate system of a polyline not facing up is not applied).
//!
//! [`survey`] turns a drawing into survey points and contour lines;
//! [`points_from_text`] reads a plain list of points (CSV or TXT: X, Y, Z,
//! as is prepared for other programs).

mod survey;

pub use survey::{layers, number, survey, LayerSummary, Survey, SurveyOptions};

/// A point in the drawing's units.
pub type P3 = [f64; 3];

/// What a DXF entity is, as far as ArchiSim reads it.
#[derive(Debug, Clone, PartialEq)]
pub enum Shape {
    Point(P3),
    Line(P3, P3),
    /// A polyline: its vertices in order; closed or not.
    Polyline(Vec<P3>, bool),
    /// A face of three or four corners.
    Face(Vec<P3>),
    /// A block placed: its name, insertion point and attributes (tag,
    /// value).
    Insert(String, P3, Vec<(String, String)>),
    /// A text: its insertion point and contents (formatting removed).
    Text(P3, String),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Entity {
    pub layer: String,
    pub shape: Shape,
}

/// A drawing as read.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Drawing {
    pub entities: Vec<Entity>,
    /// Drawing units (`$INSUNITS`): metres per unit, if the file says.
    pub unit: Option<f64>,
    /// Entities of kinds not read, by name, with how many.
    pub skipped: Vec<(String, usize)>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum DxfError {
    /// Not an ASCII DXF (a binary DXF, a DWG…).
    NotDxf,
    /// A group code that is not a number, at this line.
    BadCode(usize),
}

impl std::fmt::Display for DxfError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DxfError::NotDxf => write!(f, "no es un DXF de texto"),
            DxfError::BadCode(line) => write!(f, "código de grupo ilegible en la línea {line}"),
        }
    }
}

impl std::error::Error for DxfError {}

/// The text of a file: UTF-8 (DXF 2007 and later), or else one byte per
/// character (the Windows code page of older files, read as Latin-1).
pub fn decode(bytes: &[u8]) -> String {
    match std::str::from_utf8(bytes) {
        Ok(s) => s.to_string(),
        Err(_) => bytes.iter().map(|&b| b as char).collect(),
    }
}

/// Reads a DXF file's bytes.
pub fn read_bytes(bytes: &[u8]) -> Result<Drawing, DxfError> {
    if bytes.starts_with(b"AutoCAD Binary DXF") || bytes.starts_with(b"AC10") {
        return Err(DxfError::NotDxf);
    }
    read(&decode(bytes))
}

/// Metres per unit of a `$INSUNITS` code.
fn unit_of(code: i64) -> Option<f64> {
    Some(match code {
        1 => 0.0254,
        2 => 0.3048,
        4 => 0.001,
        5 => 0.01,
        6 => 1.0,
        7 => 1000.0,
        14 => 0.1,
        _ => return None,
    })
}

/// Reads a DXF drawing from its text.
pub fn read(text: &str) -> Result<Drawing, DxfError> {
    let lines: Vec<&str> = text.lines().collect();
    let mut pairs: Vec<(i32, &str)> = Vec::with_capacity(lines.len() / 2);
    let mut i = 0;
    while i + 1 < lines.len() {
        let code = lines[i].trim();
        if code.is_empty() && pairs.is_empty() {
            i += 1;
            continue;
        }
        let code: i32 = code.parse().map_err(|_| DxfError::BadCode(i + 1))?;
        pairs.push((code, lines[i + 1].trim_end_matches('\r')));
        i += 2;
    }
    if !pairs.iter().any(|(c, v)| *c == 0 && v.trim() == "SECTION") {
        return Err(DxfError::NotDxf);
    }
    let mut drawing = Drawing::default();
    // The header: drawing units.
    if let Some(k) = pairs
        .iter()
        .position(|(c, v)| *c == 9 && v.trim() == "$INSUNITS")
    {
        if let Some((70, v)) = pairs.get(k + 1) {
            drawing.unit = v.trim().parse().ok().and_then(unit_of);
        }
    }
    // The entities section.
    let Some(start) = pairs.windows(2).position(|w| {
        w[0].0 == 0 && w[0].1.trim() == "SECTION" && w[1].0 == 2 && w[1].1.trim() == "ENTITIES"
    }) else {
        return Ok(drawing);
    };
    let body = &pairs[start + 2..];
    let end = body
        .iter()
        .position(|(c, v)| *c == 0 && v.trim() == "ENDSEC")
        .unwrap_or(body.len());
    let body = &body[..end];
    // Split into entities: each starts at a code 0.
    let mut records: Vec<&[(i32, &str)]> = Vec::new();
    let mut s = 0;
    for k in 1..=body.len() {
        if k == body.len() || body[k].0 == 0 {
            if s < k {
                records.push(&body[s..k]);
            }
            s = k;
        }
    }
    let mut skipped: std::collections::BTreeMap<String, usize> = Default::default();
    let mut k = 0;
    while k < records.len() {
        let rec = records[k];
        let kind = rec[0].1.trim().to_string();
        k += 1;
        let r = Record(rec);
        // Paper space entities are not part of the drawing.
        if r.int(67) == Some(1) {
            // A polyline's vertices or an insert's attributes go with it.
            if matches!(kind.as_str(), "POLYLINE" | "INSERT") {
                while k < records.len() && !Record(records[k]).is("SEQEND") {
                    k += 1;
                }
                k += 1;
            }
            continue;
        }
        let layer = r.text(8).unwrap_or("0").to_string();
        let shape = match kind.as_str() {
            "POINT" => Some(Shape::Point(r.p3(10))),
            "LINE" => Some(Shape::Line(r.p3(10), r.p3(11))),
            "LWPOLYLINE" => {
                let z = r.num(38).unwrap_or(0.0);
                let xs = r.all(10);
                let ys = r.all(20);
                let pts: Vec<P3> = xs.iter().zip(&ys).map(|(x, y)| [*x, *y, z]).collect();
                Some(Shape::Polyline(pts, r.int(70).unwrap_or(0) & 1 == 1))
            }
            "POLYLINE" => {
                let flags = r.int(70).unwrap_or(0);
                // 2D: the elevation is the z of its own point.
                let elevation = r.num(30).unwrap_or(0.0);
                let three_d = flags & 8 != 0;
                let mesh = flags & (16 | 64) != 0;
                let mut pts = Vec::new();
                while k < records.len() && Record(records[k]).is("VERTEX") {
                    let v = Record(records[k]);
                    let vflags = v.int(70).unwrap_or(0);
                    // Polyface records that only index vertices carry no
                    // point.
                    if !(mesh && vflags & 128 != 0 && vflags & 64 == 0) {
                        let p = v.p3(10);
                        pts.push(if three_d || mesh {
                            p
                        } else {
                            [p[0], p[1], elevation]
                        });
                    }
                    k += 1;
                }
                if k < records.len() && Record(records[k]).is("SEQEND") {
                    k += 1;
                }
                Some(Shape::Polyline(pts, flags & 1 == 1))
            }
            "3DFACE" => {
                let mut pts = vec![r.p3(10), r.p3(11), r.p3(12), r.p3(13)];
                if pts[3] == pts[2] {
                    pts.pop();
                }
                Some(Shape::Face(pts))
            }
            "INSERT" => {
                let name = r.text(2).unwrap_or("").to_string();
                let at = r.p3(10);
                let mut attribs = Vec::new();
                if r.int(66) == Some(1) {
                    while k < records.len() && Record(records[k]).is("ATTRIB") {
                        let a = Record(records[k]);
                        attribs.push((
                            a.text(2).unwrap_or("").to_string(),
                            a.text(1).unwrap_or("").trim().to_string(),
                        ));
                        k += 1;
                    }
                    if k < records.len() && Record(records[k]).is("SEQEND") {
                        k += 1;
                    }
                }
                Some(Shape::Insert(name, at, attribs))
            }
            "TEXT" => Some(Shape::Text(r.p3(10), r.text(1).unwrap_or("").to_string())),
            "MTEXT" => {
                let mut s: String = r.texts(3).concat();
                s.push_str(r.text(1).unwrap_or(""));
                Some(Shape::Text(r.p3(10), plain_mtext(&s)))
            }
            _ => {
                *skipped.entry(kind).or_default() += 1;
                None
            }
        };
        if let Some(shape) = shape {
            drawing.entities.push(Entity { layer, shape });
        }
    }
    drawing.skipped = skipped.into_iter().collect();
    Ok(drawing)
}

/// The group codes of one entity.
struct Record<'a>(&'a [(i32, &'a str)]);

impl Record<'_> {
    fn is(&self, kind: &str) -> bool {
        self.0
            .first()
            .is_some_and(|(c, v)| *c == 0 && v.trim() == kind)
    }
    fn text(&self, code: i32) -> Option<&str> {
        self.0.iter().find(|(c, _)| *c == code).map(|(_, v)| *v)
    }
    fn texts(&self, code: i32) -> Vec<&str> {
        self.0
            .iter()
            .filter(|(c, _)| *c == code)
            .map(|(_, v)| *v)
            .collect()
    }
    fn num(&self, code: i32) -> Option<f64> {
        self.text(code).and_then(|v| v.trim().parse().ok())
    }
    fn int(&self, code: i32) -> Option<i64> {
        self.text(code).and_then(|v| v.trim().parse().ok())
    }
    fn all(&self, code: i32) -> Vec<f64> {
        self.0
            .iter()
            .filter(|(c, _)| *c == code)
            .filter_map(|(_, v)| v.trim().parse().ok())
            .collect()
    }
    /// The point of codes `c`, `c + 10`, `c + 20`.
    fn p3(&self, c: i32) -> P3 {
        [
            self.num(c).unwrap_or(0.0),
            self.num(c + 10).unwrap_or(0.0),
            self.num(c + 20).unwrap_or(0.0),
        ]
    }
}

/// The plain text of an `MTEXT`: its formatting codes (`\P`, `{\fArial;…}`)
/// taken out.
fn plain_mtext(s: &str) -> String {
    let mut out = String::new();
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '{' | '}' => {}
            '\\' => match chars.next() {
                Some('P') | Some('~') => out.push(' '),
                Some('\\') => out.push('\\'),
                Some('{') => out.push('{'),
                Some('}') => out.push('}'),
                // Switches with no value (underline, overline, strike).
                Some('L' | 'l' | 'O' | 'o' | 'K' | 'k') => {}
                // A code with a value up to ';'.
                Some(_) => {
                    for d in chars.by_ref() {
                        if d == ';' {
                            break;
                        }
                    }
                }
                None => {}
            },
            _ => out.push(c),
        }
    }
    out.trim().to_string()
}

/// A list of points written as text, one per line: `X Y Z`, separated by
/// commas, semicolons, tabs or spaces, perhaps after a point number and
/// followed by a code (`12;431250,10;4581300,25;643,18;TN`). Decimal
/// commas are understood when the fields are separated otherwise. Lines
/// that are not points (headings, blanks) are skipped; `swap` reads `Y X Z`
/// (northing first). Returns the points and the lines skipped.
pub fn points_from_text(text: &str, swap: bool) -> (Vec<P3>, usize) {
    let mut out = Vec::new();
    let mut skipped = 0;
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        // Fields: by ';' or tab if there are any (then ',' may be a
        // decimal comma), else by ',' or spaces.
        let fields: Vec<&str> = if line.contains(';') || line.contains('\t') {
            line.split([';', '\t']).collect()
        } else if line.contains(',') && !line.contains(' ') {
            line.split(',').collect()
        } else {
            line.split([' ', ',']).filter(|f| !f.is_empty()).collect()
        };
        let nums: Vec<Option<f64>> = fields.iter().map(|f| number(f)).collect();
        // The three numbers after the point number, if the first is a
        // whole number and there are four or more.
        let values: Vec<f64> = nums.iter().map_while(|n| *n).collect();
        let xyz = match values.len() {
            3 => Some([values[0], values[1], values[2]]),
            n if n >= 4 && values[0].fract() == 0.0 => Some([values[1], values[2], values[3]]),
            n if n >= 4 => Some([values[0], values[1], values[2]]),
            _ => None,
        };
        match xyz {
            Some([a, b, z]) => out.push(if swap { [b, a, z] } else { [a, b, z] }),
            None => skipped += 1,
        }
    }
    (out, skipped)
}

#[cfg(test)]
mod tests;
