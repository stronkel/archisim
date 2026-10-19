//! Single-stroke technical lettering for dimensions and level marks.
//!
//! Text is drawn as stroked paths, in the spirit of the normalised
//! technical lettering of drawings (ISO 3098, upright): the same strokes on
//! screen, in the exported SVG and later in PDF, with no dependency on the
//! fonts installed on the machine. The glyphs are ArchiSim's own design.
//!
//! Glyphs are polylines on a grid where the capital height is 1 and the
//! baseline is 0. Digits, decimal comma and signs for dimensions; the
//! Latin alphabet with the Spanish letters (ñ, accents) for the names of
//! rooms and labels; lowercase letters have their x-height at 0.7 and
//! their descenders down to −0.3.

use std::fmt::Write as _;

use archisim_core::Point2;

/// Stroke width as a fraction of the text height (ISO 3098 type B: h/10).
pub const STROKE: f64 = 0.1;
/// Space between characters, as a fraction of the text height.
const GAP: f64 = 0.22;

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

/// One pen stroke of a glyph: a polyline in glyph units.
type Stroke = Vec<(f64, f64)>;

/// Strokes and advance width of a character, or `None` if it has no glyph.
fn glyph(c: char) -> Option<(Vec<Stroke>, f64)> {
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
            let six = glyph('6')?.0;
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
        _ => return letter(c),
    };
    Some((strokes, w))
}

/// A polyline from a list of points.
fn pl(points: &[(f64, f64)]) -> Stroke {
    points.to_vec()
}

/// An arc followed by straight segments.
fn arc_then(mut a: Stroke, then: &[(f64, f64)]) -> Stroke {
    a.extend_from_slice(then);
    a
}

/// Straight segments followed by an arc.
fn then_arc(first: &[(f64, f64)], a: Stroke) -> Stroke {
    let mut s = first.to_vec();
    s.extend(a);
    s
}

/// The acute accent over a letter `w` wide whose top is at `top`.
fn acute(w: f64, top: f64) -> Stroke {
    pl(&[(w / 2.0 - 0.05, top + 0.13), (w / 2.0 + 0.09, top + 0.3)])
}

/// The tilde of ñ / Ñ.
fn tilde(w: f64, top: f64) -> Stroke {
    pl(&[
        (w * 0.15, top + 0.14),
        (w * 0.32, top + 0.22),
        (w * 0.66, top + 0.12),
        (w * 0.85, top + 0.2),
    ])
}

/// Letters and the remaining signs.
fn letter(c: char) -> Option<(Vec<Stroke>, f64)> {
    // Lowercase bowl: the 'o', 0.5 wide and x-height high.
    let bowl = || arc(0.25, 0.35, 0.25, 0.35, 0.0, 360.0);
    let (strokes, w): (Vec<Stroke>, f64) = match c {
        'A' => (
            vec![
                pl(&[(0.0, 0.0), (0.3, 1.0), (0.6, 0.0)]),
                pl(&[(0.1, 0.33), (0.5, 0.33)]),
            ],
            0.6,
        ),
        'B' => (
            vec![
                then_arc(
                    &[(0.0, 0.5), (0.0, 1.0), (0.33, 1.0)],
                    arc_then(arc(0.33, 0.75, 0.22, 0.25, 90.0, -90.0), &[(0.0, 0.5)]),
                ),
                then_arc(
                    &[(0.0, 0.5), (0.0, 0.0), (0.35, 0.0)],
                    arc_then(arc(0.35, 0.25, 0.25, 0.25, -90.0, 90.0), &[(0.0, 0.5)]),
                ),
            ],
            0.6,
        ),
        'C' => (vec![arc(0.32, 0.5, 0.32, 0.5, 50.0, 310.0)], 0.6),
        'D' => (
            vec![then_arc(
                &[(0.25, 0.0), (0.0, 0.0), (0.0, 1.0), (0.25, 1.0)],
                arc(0.25, 0.5, 0.35, 0.5, 90.0, -90.0),
            )],
            0.6,
        ),
        'E' => (
            vec![
                pl(&[(0.55, 1.0), (0.0, 1.0), (0.0, 0.0), (0.55, 0.0)]),
                pl(&[(0.0, 0.5), (0.45, 0.5)]),
            ],
            0.55,
        ),
        'F' => (
            vec![
                pl(&[(0.55, 1.0), (0.0, 1.0), (0.0, 0.0)]),
                pl(&[(0.0, 0.5), (0.45, 0.5)]),
            ],
            0.55,
        ),
        'G' => (
            vec![arc_then(
                arc(0.32, 0.5, 0.32, 0.5, 50.0, 320.0),
                &[(0.6, 0.45), (0.36, 0.45)],
            )],
            0.62,
        ),
        'H' => (
            vec![
                pl(&[(0.0, 0.0), (0.0, 1.0)]),
                pl(&[(0.6, 0.0), (0.6, 1.0)]),
                pl(&[(0.0, 0.5), (0.6, 0.5)]),
            ],
            0.6,
        ),
        'I' => (vec![pl(&[(0.05, 0.0), (0.05, 1.0)])], 0.1),
        'J' => (
            vec![then_arc(
                &[(0.45, 1.0)],
                arc(0.24, 0.26, 0.21, 0.26, 0.0, -180.0),
            )],
            0.48,
        ),
        'K' => (
            vec![
                pl(&[(0.0, 0.0), (0.0, 1.0)]),
                pl(&[(0.55, 1.0), (0.0, 0.38)]),
                pl(&[(0.17, 0.57), (0.58, 0.0)]),
            ],
            0.58,
        ),
        'L' => (vec![pl(&[(0.0, 1.0), (0.0, 0.0), (0.5, 0.0)])], 0.5),
        'M' => (
            vec![pl(&[
                (0.0, 0.0),
                (0.0, 1.0),
                (0.35, 0.3),
                (0.7, 1.0),
                (0.7, 0.0),
            ])],
            0.7,
        ),
        'N' => (
            vec![pl(&[(0.0, 0.0), (0.0, 1.0), (0.6, 0.0), (0.6, 1.0)])],
            0.6,
        ),
        'Ñ' => (
            vec![
                pl(&[(0.0, 0.0), (0.0, 1.0), (0.6, 0.0), (0.6, 1.0)]),
                tilde(0.6, 1.0),
            ],
            0.6,
        ),
        'O' => (vec![arc(0.32, 0.5, 0.32, 0.5, 0.0, 360.0)], 0.64),
        'P' => (
            vec![then_arc(
                &[(0.0, 0.0), (0.0, 1.0), (0.33, 1.0)],
                arc_then(arc(0.33, 0.73, 0.25, 0.27, 90.0, -90.0), &[(0.0, 0.46)]),
            )],
            0.58,
        ),
        'Q' => (
            vec![
                arc(0.32, 0.5, 0.32, 0.5, 0.0, 360.0),
                pl(&[(0.38, 0.24), (0.66, -0.06)]),
            ],
            0.66,
        ),
        'R' => (
            vec![
                then_arc(
                    &[(0.0, 0.0), (0.0, 1.0), (0.33, 1.0)],
                    arc_then(arc(0.33, 0.73, 0.25, 0.27, 90.0, -90.0), &[(0.0, 0.46)]),
                ),
                pl(&[(0.3, 0.46), (0.6, 0.0)]),
            ],
            0.6,
        ),
        'S' => (
            vec![{
                let mut s = arc(0.3, 0.75, 0.27, 0.25, 20.0, 270.0);
                s.extend(arc(0.3, 0.25, 0.3, 0.25, 90.0, -160.0));
                s
            }],
            0.6,
        ),
        'T' => (
            vec![pl(&[(0.0, 1.0), (0.6, 1.0)]), pl(&[(0.3, 1.0), (0.3, 0.0)])],
            0.6,
        ),
        'U' => (
            vec![then_arc(
                &[(0.0, 1.0)],
                arc_then(arc(0.3, 0.3, 0.3, 0.3, 180.0, 360.0), &[(0.6, 1.0)]),
            )],
            0.6,
        ),
        'V' => (vec![pl(&[(0.0, 1.0), (0.3, 0.0), (0.6, 1.0)])], 0.6),
        'W' => (
            vec![pl(&[
                (0.0, 1.0),
                (0.18, 0.0),
                (0.35, 0.7),
                (0.52, 0.0),
                (0.7, 1.0),
            ])],
            0.7,
        ),
        'X' => (
            vec![pl(&[(0.0, 0.0), (0.6, 1.0)]), pl(&[(0.0, 1.0), (0.6, 0.0)])],
            0.6,
        ),
        'Y' => (
            vec![
                pl(&[(0.0, 1.0), (0.3, 0.5), (0.6, 1.0)]),
                pl(&[(0.3, 0.5), (0.3, 0.0)]),
            ],
            0.6,
        ),
        'Z' => (
            vec![pl(&[(0.0, 1.0), (0.6, 1.0), (0.0, 0.0), (0.6, 0.0)])],
            0.6,
        ),
        'Á' | 'É' | 'Í' | 'Ó' | 'Ú' => {
            let base = match c {
                'Á' => 'A',
                'É' => 'E',
                'Í' => 'I',
                'Ó' => 'O',
                _ => 'U',
            };
            let (mut s, w) = letter(base)?;
            s.push(acute(w, 1.0));
            (s, w)
        }
        'a' => (vec![bowl(), pl(&[(0.5, 0.7), (0.5, 0.0)])], 0.5),
        'b' => (vec![bowl(), pl(&[(0.0, 1.0), (0.0, 0.0)])], 0.5),
        'c' => (vec![arc(0.26, 0.35, 0.26, 0.35, 45.0, 315.0)], 0.48),
        'd' => (vec![bowl(), pl(&[(0.5, 1.0), (0.5, 0.0)])], 0.5),
        'e' => (
            vec![then_arc(
                &[(0.02, 0.36), (0.5, 0.36)],
                arc(0.26, 0.35, 0.25, 0.35, 0.0, 320.0),
            )],
            0.5,
        ),
        'f' => (
            vec![
                arc_then(arc(0.3, 0.85, 0.15, 0.15, 30.0, 180.0), &[(0.15, 0.0)]),
                pl(&[(0.0, 0.65), (0.34, 0.65)]),
            ],
            0.38,
        ),
        'g' => (
            vec![
                bowl(),
                then_arc(
                    &[(0.5, 0.7), (0.5, -0.08)],
                    arc(0.25, -0.08, 0.25, 0.2, 0.0, -160.0),
                ),
            ],
            0.5,
        ),
        'h' => (
            vec![
                pl(&[(0.0, 1.0), (0.0, 0.0)]),
                then_arc(
                    &[(0.0, 0.45)],
                    arc_then(arc(0.25, 0.45, 0.25, 0.25, 180.0, 0.0), &[(0.5, 0.0)]),
                ),
            ],
            0.5,
        ),
        'i' => (
            vec![
                pl(&[(0.05, 0.7), (0.05, 0.0)]),
                pl(&[(0.05, 0.9), (0.05, 0.94)]),
            ],
            0.1,
        ),
        'í' => (
            vec![
                pl(&[(0.05, 0.7), (0.05, 0.0)]),
                pl(&[(0.0, 0.84), (0.14, 1.0)]),
            ],
            0.12,
        ),
        'j' => (
            vec![
                pl(&[(0.2, 0.7), (0.2, -0.16), (0.12, -0.27), (0.0, -0.27)]),
                pl(&[(0.2, 0.9), (0.2, 0.94)]),
            ],
            0.25,
        ),
        'k' => (
            vec![
                pl(&[(0.0, 1.0), (0.0, 0.0)]),
                pl(&[(0.44, 0.7), (0.0, 0.28)]),
                pl(&[(0.14, 0.41), (0.47, 0.0)]),
            ],
            0.47,
        ),
        'l' => (vec![pl(&[(0.05, 1.0), (0.05, 0.0)])], 0.1),
        'n' => (
            vec![
                pl(&[(0.0, 0.7), (0.0, 0.0)]),
                then_arc(
                    &[(0.0, 0.45)],
                    arc_then(arc(0.25, 0.45, 0.25, 0.25, 180.0, 0.0), &[(0.5, 0.0)]),
                ),
            ],
            0.5,
        ),
        'ñ' => (
            vec![
                pl(&[(0.0, 0.7), (0.0, 0.0)]),
                then_arc(
                    &[(0.0, 0.45)],
                    arc_then(arc(0.25, 0.45, 0.25, 0.25, 180.0, 0.0), &[(0.5, 0.0)]),
                ),
                tilde(0.5, 0.72),
            ],
            0.5,
        ),
        'o' => (vec![bowl()], 0.5),
        'p' => (vec![bowl(), pl(&[(0.0, 0.7), (0.0, -0.3)])], 0.5),
        'q' => (vec![bowl(), pl(&[(0.5, 0.7), (0.5, -0.3)])], 0.5),
        'r' => (
            vec![
                pl(&[(0.0, 0.7), (0.0, 0.0)]),
                then_arc(&[(0.0, 0.4)], arc(0.28, 0.42, 0.28, 0.28, 180.0, 60.0)),
            ],
            0.38,
        ),
        's' => (
            vec![{
                let mut s = arc(0.23, 0.53, 0.2, 0.17, 20.0, 270.0);
                s.extend(arc(0.24, 0.18, 0.24, 0.18, 90.0, -160.0));
                s
            }],
            0.48,
        ),
        't' => (
            vec![
                pl(&[(0.15, 0.95), (0.15, 0.1), (0.22, 0.0), (0.35, 0.0)]),
                pl(&[(0.0, 0.7), (0.33, 0.7)]),
            ],
            0.36,
        ),
        'u' => (
            vec![
                then_arc(
                    &[(0.0, 0.7)],
                    arc_then(arc(0.25, 0.25, 0.25, 0.25, 180.0, 360.0), &[]),
                ),
                pl(&[(0.5, 0.7), (0.5, 0.0)]),
            ],
            0.5,
        ),
        'v' => (vec![pl(&[(0.0, 0.7), (0.25, 0.0), (0.5, 0.7)])], 0.5),
        'w' => (
            vec![pl(&[
                (0.0, 0.7),
                (0.15, 0.0),
                (0.3, 0.5),
                (0.45, 0.0),
                (0.6, 0.7),
            ])],
            0.6,
        ),
        'x' => (
            vec![pl(&[(0.0, 0.0), (0.5, 0.7)]), pl(&[(0.0, 0.7), (0.5, 0.0)])],
            0.5,
        ),
        'y' => (
            vec![
                pl(&[(0.0, 0.7), (0.25, 0.0)]),
                pl(&[(0.5, 0.7), (0.14, -0.3), (0.04, -0.3)]),
            ],
            0.5,
        ),
        'z' => (
            vec![pl(&[(0.0, 0.7), (0.5, 0.7), (0.0, 0.0), (0.5, 0.0)])],
            0.5,
        ),
        'á' | 'é' | 'ó' | 'ú' => {
            let base = match c {
                'á' => 'a',
                'é' => 'e',
                'ó' => 'o',
                _ => 'u',
            };
            let (mut s, w) = letter(base)?;
            s.push(acute(w, 0.7));
            (s, w)
        }
        'ü' => {
            let (mut s, w) = letter('u')?;
            s.push(pl(&[(0.12, 0.88), (0.12, 0.92)]));
            s.push(pl(&[(0.38, 0.88), (0.38, 0.92)]));
            (s, w)
        }
        '²' => {
            let (two, _) = glyph('2')?;
            (
                two.into_iter()
                    .map(|s| {
                        s.into_iter()
                            .map(|(x, y)| (x * 0.55, 0.5 + y * 0.55))
                            .collect()
                    })
                    .collect(),
                0.36,
            )
        }
        'º' | '°' => (vec![arc(0.14, 0.82, 0.14, 0.16, 0.0, 360.0)], 0.3),
        '/' => (vec![pl(&[(0.0, -0.1), (0.4, 1.05)])], 0.4),
        '(' => (vec![arc(0.32, 0.45, 0.28, 0.65, 120.0, 240.0)], 0.22),
        ')' => (vec![arc(-0.1, 0.45, 0.28, 0.65, 60.0, -60.0)], 0.22),
        ':' => (
            vec![
                pl(&[(0.04, 0.0), (0.04, 0.04)]),
                pl(&[(0.04, 0.5), (0.04, 0.54)]),
            ],
            0.08,
        ),
        '·' => (vec![pl(&[(0.04, 0.45), (0.04, 0.49)])], 0.08),
        '_' => (vec![pl(&[(0.0, -0.1), (0.6, -0.1)])], 0.6),
        '\'' | '’' => (vec![pl(&[(0.04, 1.0), (0.04, 0.75)])], 0.08),
        '"' => (
            vec![
                pl(&[(0.04, 1.0), (0.04, 0.75)]),
                pl(&[(0.2, 1.0), (0.2, 0.75)]),
            ],
            0.24,
        ),
        '&' => (
            vec![{
                let mut s = vec![(0.6, 0.0)];
                s.extend(arc(0.25, 0.8, 0.15, 0.2, -40.0, 220.0));
                s.extend(arc(0.27, 0.27, 0.27, 0.27, 140.0, 330.0));
                s.push((0.6, 0.4));
                s
            }],
            0.6,
        ),
        _ => return None,
    };
    Some((strokes, w))
}

/// Width of a text of height `h`, in the same units.
pub fn text_width(text: &str, h: f64) -> f64 {
    let widths: Vec<f64> = text.chars().filter_map(glyph).map(|g| g.1).collect();
    if widths.is_empty() {
        return 0.0;
    }
    (widths.iter().sum::<f64>() + GAP * (widths.len() - 1) as f64) * h
}

/// How a text sits on its anchor point.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Align {
    /// The anchor is the left end of the baseline.
    Left,
    /// The anchor is the middle of the baseline.
    Centre,
}

/// SVG path data (`M… L…`) of `text` of height `h`, written along the unit
/// vector `dir` (its left normal is "up"), anchored at `at` in model
/// coordinates. The y axis is flipped for SVG, like every plan coordinate.
pub fn text_path(text: &str, at: Point2, dir: Point2, h: f64, align: Align) -> String {
    let up = dir.perp_left();
    let start = match align {
        Align::Left => 0.0,
        Align::Centre => -text_width(text, h) / 2.0,
    };
    let mut d = String::new();
    let mut x = start;
    for c in text.chars() {
        let Some((strokes, w)) = glyph(c) else {
            continue;
        };
        for s in strokes {
            for (i, (gx, gy)) in s.iter().enumerate() {
                let p = at + dir * (x + gx * h) + up * (gy * h);
                let _ = write!(
                    d,
                    "{}{},{} ",
                    if i == 0 { "M" } else { "L" },
                    num(p.x),
                    num(-p.y)
                );
            }
        }
        x += (w + GAP) * h;
    }
    d.trim_end().to_string()
}

fn num(v: f64) -> String {
    let v = (v * 1e6).round() / 1e6;
    if v == 0.0 {
        "0".into()
    } else {
        format!("{v}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_dimension_character_has_a_glyph() {
        for c in "0123456789,.+-±m ".chars() {
            assert!(glyph(c).is_some(), "{c}");
        }
        assert!(glyph('€').is_none());
    }

    /// With `ARCHISIM_GLYPHS=file.svg`, writes every glyph to look at them.
    #[test]
    fn glyph_sheet() {
        let Some(path) = std::env::var_os("ARCHISIM_GLYPHS") else {
            return;
        };
        let d = text_path(
            "0123456789 ±0,00 +3,45 -1.5m ABCDEFGHIJKLMNÑOPQRSTUVWXYZ abcdefghijklmnñopqrstuvwxyz áéíóú ÁÉÍÓÚ m² (1/2): Baño",
            Point2::new(0.0, 0.0),
            Point2::new(1.0, 0.0),
            10.0,
            Align::Left,
        );
        let svg = format!(
            r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="-5 -15 1300 25"><rect x="-5" y="-15" width="1300" height="25" fill="white"/><path d="{d}" fill="none" stroke="black" stroke-width="1" stroke-linecap="round" stroke-linejoin="round"/></svg>"#
        );
        std::fs::write(path, svg).unwrap();
    }

    #[test]
    fn widths_and_alignment() {
        let w = text_width("3,45", 0.1);
        assert!(w > 0.15 && w < 0.3, "{w}");
        // Centred text is symmetric about its anchor.
        let d = text_path(
            "88",
            Point2::new(10.0, 0.0),
            Point2::new(1.0, 0.0),
            0.1,
            Align::Centre,
        );
        let xs: Vec<f64> = d
            .split_whitespace()
            .map(|t| t[1..].split(',').next().unwrap().parse().unwrap())
            .collect();
        let (lo, hi) = xs
            .iter()
            .fold((f64::MAX, f64::MIN), |a, x| (a.0.min(*x), a.1.max(*x)));
        assert!(((lo + hi) / 2.0 - 10.0).abs() < 1e-3, "{lo} {hi}");
        // Unknown characters are skipped.
        assert_eq!(text_width("€", 1.0), 0.0);
    }
}

#[cfg(test)]
mod letter_tests {
    use super::*;

    #[test]
    fn spanish_names_have_every_glyph() {
        for name in [
            "Dormitorio principal",
            "Baño",
            "Cocina-comedor",
            "Salón",
            "Almacén",
            "Terraza (cubierta)",
            "S útil: 12,45 m²",
            "ÁÉÍÓÚÑ áéíóúñ ü",
            "ABCDEFGHIJKLMNOPQRSTUVWXYZ",
            "abcdefghijklmnopqrstuvwxyz",
        ] {
            for c in name.chars() {
                assert!(glyph(c).is_some(), "no glyph for {c:?} in «{name}»");
            }
            assert!(text_width(name, 1.0) > 0.0);
        }
    }
}
