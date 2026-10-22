//! Single-stroke technical lettering (in the spirit of UNE-EN ISO 3098,
//! upright): ArchiSim's own design, drawn as lines so texts look the same
//! on screen, in SVG, PDF and DXF without depending on installed fonts.
//!
//! Glyphs are polylines on a grid where the capital height is 1 and the
//! baseline 0 (lower case: x-height 0.7, descenders to −0.3). Every
//! character of Spanish texts has one: letters with accents, ñ, ç, ü,
//! opening question and exclamation marks, º and ª, ², ³, Ø, °, ±, €…

use std::cell::Cell;
use std::collections::HashMap;
use std::sync::OnceLock;

use archisim_core::{Point2, Text, TextFont, LINE_SPACING};

thread_local! {
    /// The lettering texts are measured and drawn with on this thread.
    static CURRENT: Cell<TextFont> = const { Cell::new(TextFont::Sans) };
}

/// The lettering in use: the project's, set by whoever draws.
pub fn current() -> TextFont {
    CURRENT.with(|c| c.get())
}

/// Uses a lettering until the guard is dropped (then the previous one).
pub fn use_font(f: TextFont) -> FontGuard {
    let previous = CURRENT.with(|c| c.replace(f));
    FontGuard(previous)
}

/// Sets the lettering for good (the app, once per frame).
pub fn set_font(f: TextFont) {
    CURRENT.with(|c| c.set(f));
}

pub struct FontGuard(TextFont);

impl Drop for FontGuard {
    fn drop(&mut self) {
        CURRENT.with(|c| c.set(self.0));
    }
}

/// Stroke width as a fraction of the text height (ISO 3098 type B: h/10).
pub const STROKE: f64 = 0.1;
/// Space between characters, as a fraction of the text height.
pub const GAP: f64 = 0.22;

/// One pen stroke of a glyph: a polyline in glyph units.
pub type Stroke = Vec<(f64, f64)>;

/// Points of an ellipse arc from angle `a0` to `a1` (degrees, counter-
/// clockwise from +x), centred on `(cx, cy)`.
fn arc(cx: f64, cy: f64, rx: f64, ry: f64, a0: f64, a1: f64) -> Vec<(f64, f64)> {
    let n = (((a1 - a0).abs() / 15.0).ceil() as usize).max(2);
    (0..=n)
        .map(|i| {
            let a = (a0 + (a1 - a0) * i as f64 / n as f64).to_radians();
            (cx + rx * a.cos(), cy + ry * a.sin())
        })
        .collect()
}

/// The digits and signs of dimensions (the first glyphs of ArchiSim,
/// kept exactly as they were).
fn dimension_glyph(c: char) -> Option<(Vec<Stroke>, f64)> {
    let w = 0.6;
    let strokes: Vec<Stroke> = match c {
        '0' => vec![arc(0.3, 0.5, 0.3, 0.5, 0.0, 360.0)],
        '1' => vec![vec![(0.12, 0.78), (0.38, 1.0), (0.38, 0.0)]],
        '2' => {
            let mut s = arc(0.3, 0.72, 0.3, 0.28, 165.0, -20.0);
            s.extend([(0.0, 0.0), (0.6, 0.0)]);
            vec![s]
        }
        '3' => {
            let mut s = arc(0.29, 0.77, 0.27, 0.23, 155.0, -90.0);
            s.extend(arc(0.3, 0.27, 0.3, 0.27, 90.0, -160.0));
            vec![s]
        }
        '4' => vec![vec![(0.46, 0.0), (0.46, 1.0), (0.0, 0.3), (0.62, 0.3)]],
        '5' => {
            let mut s = vec![(0.56, 1.0), (0.08, 1.0), (0.04, 0.56)];
            s.extend(arc(0.3, 0.32, 0.3, 0.32, 140.0, -160.0));
            vec![s]
        }
        '6' => {
            let mut s = arc(0.3, 0.6, 0.29, 0.4, 50.0, 180.0);
            s.extend(arc(0.3, 0.3, 0.3, 0.3, 180.0, 540.0));
            vec![s]
        }
        '7' => vec![vec![(0.0, 1.0), (0.6, 1.0), (0.18, 0.0)]],
        '8' => vec![
            arc(0.3, 0.77, 0.25, 0.23, 0.0, 360.0),
            arc(0.3, 0.27, 0.3, 0.27, 0.0, 360.0),
        ],
        '9' => {
            // The 6 turned upside down.
            let six = dimension_glyph('6')?.0;
            six.into_iter()
                .map(|s| s.into_iter().map(|(x, y)| (0.6 - x, 1.0 - y)).collect())
                .collect()
        }
        ',' => return Some((vec![vec![(0.08, 0.06), (0.08, 0.0), (0.0, -0.16)]], 0.12)),
        '.' => return Some((vec![vec![(0.04, 0.0), (0.04, 0.04)]], 0.08)),
        '+' => vec![vec![(0.3, 0.2), (0.3, 0.8)], vec![(0.0, 0.5), (0.6, 0.5)]],
        '-' | '−' => vec![vec![(0.08, 0.5), (0.52, 0.5)]],
        '±' => vec![
            vec![(0.3, 0.38), (0.3, 0.98)],
            vec![(0.0, 0.68), (0.6, 0.68)],
            vec![(0.0, 0.12), (0.6, 0.12)],
        ],
        'm' => vec![
            vec![(0.0, 0.0), (0.0, 0.7)],
            {
                let mut s = arc(0.15, 0.5, 0.15, 0.2, 180.0, 0.0);
                s.push((0.3, 0.0));
                s
            },
            {
                let mut s = arc(0.45, 0.5, 0.15, 0.2, 180.0, 0.0);
                s.push((0.6, 0.0));
                s
            },
        ],
        ' ' => return Some((Vec::new(), 0.4)),
        _ => return None,
    };
    Some((strokes, w))
}

/// The rest of the glyphs, in a small notation: `w` is the advance width;
/// strokes are separated by `;`, and each is a list of points `x,y` and
/// arcs `a(cx,cy,rx,ry,from,to)` (degrees), joined in order.
const GLYPHS: &[(char, f64, &str)] = &[
    // Capitals.
    ('A', 0.62, "0,0 0.31,1 0.62,0; 0.1,0.33 0.52,0.33"),
    (
        'B',
        0.58,
        "0,0.5 0,1 0.34,1 a(0.34,0.75,0.22,0.25,90,-90) 0,0.5; 0.36,0.5 a(0.36,0.25,0.24,0.25,90,-90) 0,0 0,0.5",
    ),
    ('C', 0.62, "a(0.32,0.5,0.32,0.5,48,312)"),
    ('D', 0.6, "0,0 0,1 0.25,1 a(0.25,0.5,0.35,0.5,90,-90) 0,0"),
    ('E', 0.54, "0.54,1 0,1 0,0 0.54,0; 0,0.5 0.42,0.5"),
    ('F', 0.54, "0.54,1 0,1 0,0; 0,0.5 0.42,0.5"),
    ('G', 0.64, "a(0.32,0.5,0.32,0.5,45,360) 0.38,0.5"),
    ('H', 0.6, "0,0 0,1; 0.6,0 0.6,1; 0,0.5 0.6,0.5"),
    ('I', 0.04, "0.02,0 0.02,1"),
    ('J', 0.46, "0.46,1 a(0.23,0.25,0.23,0.25,0,-180)"),
    ('K', 0.6, "0,0 0,1; 0.58,1 0,0.38; 0.18,0.56 0.6,0"),
    ('L', 0.52, "0,1 0,0 0.52,0"),
    ('M', 0.72, "0,0 0,1 0.36,0.32 0.72,1 0.72,0"),
    ('N', 0.6, "0,0 0,1 0.6,0 0.6,1"),
    ('O', 0.66, "a(0.33,0.5,0.33,0.5,0,360)"),
    ('P', 0.58, "0,0 0,1 0.36,1 a(0.36,0.73,0.22,0.27,90,-90) 0,0.46"),
    (
        'Q',
        0.66,
        "a(0.33,0.5,0.33,0.5,0,360); 0.4,0.24 0.68,-0.04",
    ),
    (
        'R',
        0.6,
        "0,0 0,1 0.36,1 a(0.36,0.73,0.22,0.27,90,-90) 0,0.46; 0.3,0.46 0.6,0",
    ),
    (
        'S',
        0.6,
        "a(0.3,0.75,0.28,0.25,20,270) a(0.3,0.25,0.3,0.25,90,-160)",
    ),
    ('T', 0.62, "0,1 0.62,1; 0.31,1 0.31,0"),
    ('U', 0.6, "0,1 a(0.3,0.3,0.3,0.3,180,360) 0.6,1"),
    ('V', 0.62, "0,1 0.31,0 0.62,1"),
    ('W', 0.8, "0,1 0.19,0 0.4,0.74 0.61,0 0.8,1"),
    ('X', 0.6, "0,0 0.6,1; 0,1 0.6,0"),
    ('Y', 0.62, "0,1 0.31,0.5 0.62,1; 0.31,0.5 0.31,0"),
    ('Z', 0.6, "0,1 0.6,1 0,0 0.6,0"),
    // Lower case.
    ('a', 0.5, "a(0.25,0.35,0.25,0.35,0,360); 0.5,0.7 0.5,0"),
    ('b', 0.5, "0,1 0,0; a(0.25,0.35,0.25,0.35,0,360)"),
    ('c', 0.5, "a(0.26,0.35,0.26,0.35,45,315)"),
    ('d', 0.5, "a(0.25,0.35,0.25,0.35,0,360); 0.5,1 0.5,0"),
    ('e', 0.5, "0,0.35 0.5,0.35 a(0.25,0.35,0.25,0.35,0,320)"),
    ('f', 0.42, "0.46,0.93 a(0.35,0.83,0.15,0.15,40,180) 0.2,0; 0.04,0.7 0.4,0.7"),
    (
        'g',
        0.5,
        "a(0.25,0.35,0.25,0.35,0,360); 0.5,0.7 0.5,-0.1 a(0.25,-0.1,0.25,0.2,0,-180)",
    ),
    ('h', 0.5, "0,1 0,0; 0,0.45 a(0.25,0.45,0.25,0.25,180,0) 0.5,0"),
    ('i', 0.04, "0.02,0 0.02,0.7; 0.02,0.92 0.02,0.96"),
    (
        'j',
        0.3,
        "0.28,0.7 0.28,-0.1 a(0.13,-0.1,0.15,0.2,0,-160); 0.28,0.92 0.28,0.96",
    ),
    ('k', 0.48, "0,1 0,0; 0.45,0.7 0,0.26; 0.15,0.4 0.48,0"),
    ('l', 0.04, "0.02,1 0.02,0"),
    (
        'n',
        0.5,
        "0,0 0,0.7; 0,0.45 a(0.25,0.45,0.25,0.25,180,0) 0.5,0",
    ),
    ('o', 0.5, "a(0.25,0.35,0.25,0.35,0,360)"),
    ('p', 0.5, "0,0.7 0,-0.3; a(0.25,0.35,0.25,0.35,0,360)"),
    ('q', 0.5, "a(0.25,0.35,0.25,0.35,0,360); 0.5,0.7 0.5,-0.3"),
    ('r', 0.4, "0,0 0,0.7; 0,0.4 a(0.3,0.4,0.3,0.3,180,72)"),
    (
        's',
        0.48,
        "a(0.24,0.53,0.22,0.17,20,270) a(0.24,0.18,0.25,0.18,90,-160)",
    ),
    ('t', 0.38, "0.15,0.95 0.15,0.12 a(0.3,0.12,0.15,0.12,180,320); 0,0.7 0.36,0.7"),
    ('u', 0.5, "0,0.7 0,0.25 a(0.25,0.25,0.25,0.25,180,360); 0.5,0.7 0.5,0"),
    ('v', 0.5, "0,0.7 0.25,0 0.5,0.7"),
    ('w', 0.68, "0,0.7 0.16,0 0.34,0.56 0.52,0 0.68,0.7"),
    ('x', 0.5, "0,0 0.5,0.7; 0,0.7 0.5,0"),
    ('y', 0.5, "0,0.7 0.26,0.04; 0.5,0.7 0.12,-0.3"),
    ('z', 0.5, "0,0.7 0.5,0.7 0,0 0.5,0"),
    // Punctuation and signs.
    (':', 0.08, "0.04,0 0.04,0.04; 0.04,0.5 0.04,0.54"),
    (';', 0.12, "0.08,0.06 0.08,0 0,-0.16; 0.08,0.5 0.08,0.54"),
    ('!', 0.04, "0.02,1 0.02,0.3; 0.02,0 0.02,0.04"),
    ('¡', 0.04, "0.02,-0.3 0.02,0.4; 0.02,0.66 0.02,0.7"),
    (
        '?',
        0.5,
        "0.02,0.85 a(0.25,0.75,0.25,0.25,155,-90) 0.25,0.3; 0.25,0 0.25,0.04",
    ),
    (
        '¿',
        0.5,
        "0.48,-0.15 a(0.25,-0.05,0.25,0.25,-25,90) 0.25,0.4; 0.25,0.66 0.25,0.7",
    ),
    ('(', 0.22, "a(0.35,0.45,0.35,0.65,118,242)"),
    (')', 0.22, "a(-0.13,0.45,0.35,0.65,62,-62)"),
    ('[', 0.2, "0.2,1.1 0,1.1 0,-0.15 0.2,-0.15"),
    (']', 0.2, "0,1.1 0.2,1.1 0.2,-0.15 0,-0.15"),
    ('{', 0.26, "0.26,1.1 0.13,1.02 0.13,0.55 0,0.48 0.13,0.41 0.13,-0.07 0.26,-0.15"),
    ('}', 0.26, "0,1.1 0.13,1.02 0.13,0.55 0.26,0.48 0.13,0.41 0.13,-0.07 0,-0.15"),
    ('/', 0.46, "0,-0.1 0.46,1.1"),
    ('\\', 0.46, "0,1.1 0.46,-0.1"),
    ('|', 0.04, "0.02,1.1 0.02,-0.2"),
    ('=', 0.56, "0.02,0.34 0.54,0.34; 0.02,0.66 0.54,0.66"),
    ('<', 0.5, "0.5,0.85 0,0.5 0.5,0.15"),
    ('>', 0.5, "0,0.85 0.5,0.5 0,0.15"),
    (
        '*',
        0.5,
        "0.25,1 0.25,0.5; 0.03,0.88 0.47,0.62; 0.03,0.62 0.47,0.88",
    ),
    (
        '%',
        0.66,
        "a(0.14,0.84,0.12,0.16,0,360); 0.62,1 0.04,0; a(0.52,0.16,0.12,0.16,0,360)",
    ),
    (
        '#',
        0.62,
        "0.18,-0.05 0.3,1.05; 0.4,-0.05 0.52,1.05; 0.02,0.33 0.62,0.33; 0.06,0.67 0.66,0.67",
    ),
    (
        '&',
        0.64,
        "0.64,0 0.16,0.6 a(0.28,0.8,0.16,0.2,210,-30) 0.06,0.33 a(0.26,0.22,0.22,0.22,170,330) 0.6,0.36",
    ),
    (
        '@',
        0.86,
        "a(0.43,0.45,0.17,0.21,0,360); 0.6,0.66 0.6,0.3 a(0.7,0.3,0.1,0.12,180,360) a(0.43,0.45,0.43,0.55,10,330)",
    ),
    ('_', 0.6, "0,-0.15 0.6,-0.15"),
    ('\'', 0.04, "0.02,1 0.02,0.72"),
    ('"', 0.2, "0.02,1 0.02,0.72; 0.18,1 0.18,0.72"),
    ('`', 0.16, "0,1 0.16,0.82"),
    ('^', 0.4, "0,0.7 0.2,1 0.4,0.7"),
    ('~', 0.56, "0,0.42 0.14,0.55 0.42,0.45 0.56,0.58"),
    ('·', 0.08, "0.04,0.48 0.04,0.52"),
    ('°', 0.3, "a(0.15,0.85,0.13,0.15,0,360)"),
    ('º', 0.3, "a(0.15,0.84,0.12,0.16,0,360); 0.02,0.58 0.28,0.58"),
    ('ª', 0.3, "a(0.14,0.84,0.12,0.16,0,360); 0.28,0.99 0.28,0.68; 0.02,0.58 0.28,0.58"),
    ('–', 0.6, "0,0.5 0.6,0.5"),
    ('—', 1.0, "0,0.5 1,0.5"),
    ('Ø', 0.7, "a(0.35,0.5,0.32,0.5,0,360); -0.02,-0.06 0.72,1.06"),
    ('⌒', 0.7, "a(0.35,0.35,0.32,0.32,15,165)"),
    ('ø', 0.54, "a(0.27,0.35,0.25,0.35,0,360); 0,-0.04 0.54,0.74"),
    ('€', 0.66, "a(0.38,0.5,0.32,0.5,50,310); 0,0.6 0.46,0.6; 0,0.4 0.46,0.4"),
    (
        '$',
        0.6,
        "a(0.3,0.75,0.28,0.25,20,270) a(0.3,0.25,0.3,0.25,90,-160); 0.3,1.12 0.3,-0.12",
    ),
    ('×', 0.5, "0.05,0.25 0.45,0.75; 0.05,0.75 0.45,0.25"),
    ('÷', 0.56, "0.02,0.5 0.54,0.5; 0.28,0.78 0.28,0.82; 0.28,0.18 0.28,0.22"),
    ('≤', 0.5, "0.5,0.95 0,0.62 0.5,0.29; 0,0.08 0.5,0.08"),
    ('≥', 0.5, "0,0.95 0.5,0.62 0,0.29; 0,0.08 0.5,0.08"),
    ('≈', 0.56, "0,0.3 0.14,0.43 0.42,0.33 0.56,0.46; 0,0.58 0.14,0.71 0.42,0.61 0.56,0.74"),
    ('Δ', 0.62, "0,0 0.31,1 0.62,0 0,0"),
    ('Ω', 0.66, "0,0 0.18,0 a(0.33,0.55,0.32,0.45,240,-60) 0.48,0 0.66,0"),
    ('µ', 0.5, "0,0.7 0,-0.3; 0,0.25 a(0.25,0.25,0.25,0.25,180,360); 0.5,0.7 0.5,0"),
    ('π', 0.56, "0,0.7 0.56,0.7; 0.14,0.7 0.14,0; 0.42,0.7 0.42,0"),
    ('…', 0.6, "0.04,0 0.04,0.04; 0.3,0 0.3,0.04; 0.56,0 0.56,0.04"),
    ('«', 0.46, "0.22,0.62 0,0.36 0.22,0.1; 0.46,0.62 0.24,0.36 0.46,0.1"),
    ('»', 0.46, "0,0.62 0.22,0.36 0,0.1; 0.24,0.62 0.46,0.36 0.24,0.1"),
    ('‘', 0.08, "0.08,1 0,0.84 0,0.78"),
    ('’', 0.08, "0,0.78 0.08,0.94 0.08,1"),
    ('“', 0.24, "0.08,1 0,0.84 0,0.78; 0.24,1 0.16,0.84 0.16,0.78"),
    ('”', 0.24, "0,0.78 0.08,0.94 0.08,1; 0.16,0.78 0.24,0.94 0.24,1"),
];

fn parse_glyph(src: &str) -> Vec<Stroke> {
    let mut strokes = Vec::new();
    for part in src.split(';') {
        let mut stroke: Stroke = Vec::new();
        let mut rest = part.trim();
        while !rest.is_empty() {
            if let Some(r) = rest.strip_prefix("a(") {
                let end = r.find(')').expect("arc without )");
                let v: Vec<f64> = r[..end]
                    .split(',')
                    .map(|n| n.trim().parse().expect("arc number"))
                    .collect();
                stroke.extend(arc(v[0], v[1], v[2], v[3], v[4], v[5]));
                rest = r[end + 1..].trim_start();
            } else {
                let end = rest.find(' ').unwrap_or(rest.len());
                let (x, y) = rest[..end].split_once(',').expect("point x,y");
                stroke.push((x.parse().expect("x"), y.parse().expect("y")));
                rest = rest[end..].trim_start();
            }
        }
        if !stroke.is_empty() {
            strokes.push(stroke);
        }
    }
    strokes
}

fn table() -> &'static HashMap<char, (Vec<Stroke>, f64)> {
    static TABLE: OnceLock<HashMap<char, (Vec<Stroke>, f64)>> = OnceLock::new();
    TABLE.get_or_init(|| {
        GLYPHS
            .iter()
            .map(|(c, w, src)| (*c, (parse_glyph(src), *w)))
            .collect()
    })
}

/// Marks drawn over a base letter.
#[derive(Clone, Copy)]
enum Mark {
    Acute,
    Grave,
    Circumflex,
    Diaeresis,
    Tilde,
    Cedilla,
    Ring,
}

fn decompose(c: char) -> Option<(char, Mark)> {
    use Mark::*;
    Some(match c {
        'á' => ('a', Acute),
        'é' => ('e', Acute),
        'í' => ('ı', Acute),
        'ó' => ('o', Acute),
        'ú' => ('u', Acute),
        'ý' => ('y', Acute),
        'à' => ('a', Grave),
        'è' => ('e', Grave),
        'ì' => ('ı', Grave),
        'ò' => ('o', Grave),
        'ù' => ('u', Grave),
        'â' => ('a', Circumflex),
        'ê' => ('e', Circumflex),
        'î' => ('ı', Circumflex),
        'ô' => ('o', Circumflex),
        'û' => ('u', Circumflex),
        'ä' => ('a', Diaeresis),
        'ë' => ('e', Diaeresis),
        'ï' => ('ı', Diaeresis),
        'ö' => ('o', Diaeresis),
        'ü' => ('u', Diaeresis),
        'ÿ' => ('y', Diaeresis),
        'ñ' => ('n', Tilde),
        'ã' => ('a', Tilde),
        'õ' => ('o', Tilde),
        'ç' => ('c', Cedilla),
        'å' => ('a', Ring),
        'Á' => ('A', Acute),
        'É' => ('E', Acute),
        'Í' => ('I', Acute),
        'Ó' => ('O', Acute),
        'Ú' => ('U', Acute),
        'Ý' => ('Y', Acute),
        'À' => ('A', Grave),
        'È' => ('E', Grave),
        'Ì' => ('I', Grave),
        'Ò' => ('O', Grave),
        'Ù' => ('U', Grave),
        'Â' => ('A', Circumflex),
        'Ê' => ('E', Circumflex),
        'Î' => ('I', Circumflex),
        'Ô' => ('O', Circumflex),
        'Û' => ('U', Circumflex),
        'Ä' => ('A', Diaeresis),
        'Ë' => ('E', Diaeresis),
        'Ï' => ('I', Diaeresis),
        'Ö' => ('O', Diaeresis),
        'Ü' => ('U', Diaeresis),
        'Ñ' => ('N', Tilde),
        'Ã' => ('A', Tilde),
        'Õ' => ('O', Tilde),
        'Ç' => ('C', Cedilla),
        'Å' => ('A', Ring),
        _ => return None,
    })
}

fn mark_strokes(mark: Mark, cx: f64, capital: bool, w: f64) -> Vec<Stroke> {
    // Lower-case marks sit over the x-height, capital ones over the caps.
    let y = if capital { 1.1 } else { 0.82 };
    match mark {
        Mark::Acute => vec![vec![(cx - 0.06, y), (cx + 0.1, y + 0.16)]],
        Mark::Grave => vec![vec![(cx + 0.06, y), (cx - 0.1, y + 0.16)]],
        Mark::Circumflex => vec![vec![(cx - 0.13, y), (cx, y + 0.15), (cx + 0.13, y)]],
        Mark::Diaeresis => vec![
            vec![(cx - 0.12, y + 0.06), (cx - 0.12, y + 0.1)],
            vec![(cx + 0.12, y + 0.06), (cx + 0.12, y + 0.1)],
        ],
        Mark::Tilde => vec![vec![
            (cx - 0.18, y + 0.02),
            (cx - 0.09, y + 0.12),
            (cx + 0.09, y + 0.04),
            (cx + 0.18, y + 0.14),
        ]],
        Mark::Ring => vec![arc(cx, y + 0.08, 0.08, 0.08, 0.0, 360.0)],
        Mark::Cedilla => {
            let x = (w / 2.0).max(0.1);
            vec![vec![
                (x, 0.0),
                (x, -0.1),
                (x + 0.1, -0.17),
                (x - 0.06, -0.3),
            ]]
        }
    }
}

/// Strokes and advance width of a character, or `None` if it has no
/// glyph (it is then left out).
pub fn glyph(c: char) -> Option<(Vec<Stroke>, f64)> {
    if let Some(g) = dimension_glyph(c) {
        return Some(g);
    }
    if c == 'ı' {
        // Dotless i, the base of accented i.
        return Some((vec![vec![(0.02, 0.0), (0.02, 0.7)]], 0.04));
    }
    if let Some(g) = table().get(&c) {
        return Some(g.clone());
    }
    if let Some((base, mark)) = decompose(c) {
        let (mut strokes, w) = glyph(base)?;
        strokes.extend(mark_strokes(mark, w / 2.0, base.is_uppercase(), w));
        return Some((strokes, w));
    }
    // Superscript two and three: the digits, smaller and raised.
    if let Some(d) = match c {
        '²' => Some('2'),
        '³' => Some('3'),
        '¹' => Some('1'),
        _ => None,
    } {
        let (s, w) = dimension_glyph(d)?;
        let k = 0.55;
        let s = s
            .into_iter()
            .map(|st| st.into_iter().map(|(x, y)| (x * k, y * k + 0.48)).collect())
            .collect();
        return Some((s, w * k));
    }
    None
}

/// Width of a single line of text of height `h`, in the same units, in
/// the lettering in use.
pub fn text_width(text: &str, h: f64) -> f64 {
    match current() {
        TextFont::Sans => crate::outline::width(text, h),
        TextFont::Technical => stroke_width(text, h),
    }
}

/// Width of a line in the technical lettering.
pub fn stroke_width(text: &str, h: f64) -> f64 {
    let widths: Vec<f64> = text.chars().filter_map(glyph).map(|g| g.1).collect();
    if widths.is_empty() {
        return 0.0;
    }
    (widths.iter().sum::<f64>() + GAP * (widths.len() - 1) as f64) * h
}

/// The strokes of one line of text of height `h`: x from 0 along the
/// baseline, y up.
pub fn line_strokes(text: &str, h: f64) -> Vec<Vec<(f64, f64)>> {
    let mut out = Vec::new();
    let mut x = 0.0;
    for c in text.chars() {
        let Some((strokes, w)) = glyph(c) else {
            continue;
        };
        for s in strokes {
            out.push(s.iter().map(|(gx, gy)| (x + gx * h, gy * h)).collect());
        }
        x += (w + GAP) * h;
    }
    out
}

/// Where the lines of a text go, in its own frame (x along the baseline,
/// y up, the insertion point at the origin): `(line, x, baseline y)`.
fn line_origins(t: &Text, h: f64) -> Vec<(&str, f64, f64)> {
    let lines: Vec<&str> = t.content.split('\n').collect();
    let n = lines.len();
    let step = h * LINE_SPACING;
    // Height from the last baseline to the first cap line.
    let block = h + step * (n as f64 - 1.0);
    let base0 = block * t.align.vertical() - h;
    lines
        .into_iter()
        .enumerate()
        .map(|(i, line)| {
            let w = text_width(line, h);
            (line, -w * t.align.horizontal(), base0 - step * i as f64)
        })
        .collect()
}

/// The strokes of a text, in drawing coordinates: `k` turns paper
/// millimetres into drawing units (the view scale / 1000).
pub fn text_strokes(t: &Text, k: f64) -> Vec<Vec<Point2>> {
    let h = t.height * k;
    let (s, c) = t.angle.sin_cos();
    let along = Point2::new(c, s);
    let up = along.perp_left();
    let mut out = Vec::new();
    for (line, x0, y0) in line_origins(t, h) {
        for st in line_strokes(line, h) {
            out.push(
                st.into_iter()
                    .map(|(x, y)| t.at + along * (x0 + x) + up * (y0 + y))
                    .collect(),
            );
        }
    }
    out
}

/// The outlines of a text in the sans lettering, in drawing coordinates.
pub fn text_outline(t: &Text, k: f64) -> Vec<crate::outline::Seg> {
    let h = t.height * k;
    let (s, c) = t.angle.sin_cos();
    let along = Point2::new(c, s);
    let up = along.perp_left();
    let mut out = Vec::new();
    for (line, x0, y0) in line_origins(t, h) {
        out.extend(crate::outline::line(line, h, |x, y| {
            t.at + along * (x0 + x) + up * (y0 + y)
        }));
    }
    out
}

/// The box around a text (cap height of the first line to the baseline of
/// the last, plus descenders), as four corners.
pub fn text_frame(t: &Text, k: f64) -> [Point2; 4] {
    let h = t.height * k;
    let origins = line_origins(t, h);
    let mut lo = (f64::INFINITY, f64::INFINITY);
    let mut hi = (f64::NEG_INFINITY, f64::NEG_INFINITY);
    for (line, x0, y0) in &origins {
        let w = text_width(line, h).max(h * 0.3);
        lo = (lo.0.min(*x0), lo.1.min(y0 - 0.3 * h));
        hi = (hi.0.max(x0 + w), hi.1.max(y0 + h));
    }
    let (s, c) = t.angle.sin_cos();
    let along = Point2::new(c, s);
    let up = along.perp_left();
    let p = |x: f64, y: f64| t.at + along * x + up * y;
    [p(lo.0, lo.1), p(hi.0, lo.1), p(hi.0, hi.1), p(lo.0, hi.1)]
}

#[cfg(test)]
mod tests {
    use super::*;
    use archisim_core::TextAlign;

    #[test]
    fn every_spanish_character_has_a_glyph() {
        let text = "ABCDEFGHIJKLMNÑOPQRSTUVWXYZ abcdefghijklmnñopqrstuvwxyz \
                    áéíóúÁÉÍÓÚüÜçÇàè 0123456789 ¿?¡!.,;:()[]{}/\\-+±=<>*%#&@_'\"ºª°²³Ø€$×·…«»";
        for c in text.chars() {
            assert!(glyph(c).is_some(), "falta {c:?}");
        }
        // Accented letters keep the width of their base letter.
        assert_eq!(glyph('á').unwrap().1, glyph('a').unwrap().1);
        assert!(glyph('\u{1F600}').is_none());
    }

    #[test]
    fn glyphs_stay_in_their_box() {
        for (c, w, _) in GLYPHS {
            let (strokes, _) = glyph(*c).unwrap();
            for s in strokes {
                for (x, y) in s {
                    assert!(
                        x >= -0.15 && x <= w + 0.15 && (-0.35..=1.3).contains(&y),
                        "{c}: ({x}, {y})"
                    );
                }
            }
        }
    }

    #[test]
    fn alignment_and_frame() {
        let t = Text {
            at: Point2::new(10.0, 5.0),
            content: "Dormitorio 1\nS = 12,40 m²".into(),
            height: 2.5,
            angle: 0.0,
            align: TextAlign::Middle,
        };
        let f = text_frame(&t, 0.05);
        let cx = (f[0].x + f[2].x) / 2.0;
        assert!((cx - 10.0).abs() < 1e-9, "{cx}");
        // Two lines of 2,5 mm at 1:50, 1,6 apart, plus descenders.
        let height = f[2].y - f[0].y;
        assert!((height - (2.5 + 2.5 * 1.6 + 0.75) * 0.05).abs() < 1e-9);
        // Bottom-left: the frame starts at the insertion point.
        let t = Text {
            align: TextAlign::BottomLeft,
            content: "A".into(),
            ..t
        };
        let f = text_frame(&t, 0.05);
        assert!((f[0].x - 10.0).abs() < 1e-9);
        assert!(!text_strokes(&t, 0.05).is_empty());
    }

    /// With `ARCHISIM_GLYPHS=file.svg`, writes every glyph to look at them.
    #[test]
    fn glyph_sheet() {
        let Some(path) = std::env::var_os("ARCHISIM_GLYPHS") else {
            return;
        };
        let lines = [
            "ABCDEFGHIJKLMNÑOPQRSTUVWXYZ",
            "abcdefghijklmnñopqrstuvwxyz",
            "áéíóú ÁÉÍÓÚ üÜ çÇ àèìòù âê ãõ",
            "0123456789 ±0,00 +3,45 25,40 m² 1,5 m³",
            "¿Qué? ¡Sí! (a) [b] {c} 1/2 a-b a+b=c <> * % # & @ _",
            "'x' \"y\" 45° 1º 2ª Ø16 12 € $ × ÷ · … «a» ≤ ≥ ≈ Δ µ π",
            "Salón-comedor  Dormitorio principal  Baño  Cocina",
        ];
        let mut d = String::new();
        for (i, l) in lines.iter().enumerate() {
            for s in line_strokes(l, 10.0) {
                for (j, (x, y)) in s.iter().enumerate() {
                    d.push_str(&format!(
                        "{}{:.3},{:.3} ",
                        if j == 0 { "M" } else { "L" },
                        x,
                        -y + 20.0 * i as f64
                    ));
                }
            }
        }
        let svg = format!(
            r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="-5 -15 330 150"><rect x="-5" y="-15" width="330" height="150" fill="white"/><path d="{d}" fill="none" stroke="black" stroke-width="1" stroke-linecap="round" stroke-linejoin="round"/></svg>"#
        );
        std::fs::write(path, svg).unwrap();
    }
}
