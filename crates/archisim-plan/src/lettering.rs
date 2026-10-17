//! Single-stroke technical lettering for dimensions and level marks.
//!
//! Text is drawn as stroked paths, in the spirit of the normalised
//! technical lettering of drawings (ISO 3098, upright): the same strokes on
//! screen, in the exported SVG and later in PDF, with no dependency on the
//! fonts installed on the machine. The glyphs are ArchiSim's own design.
//!
//! Glyphs are polylines on a grid where the capital height is 1 and the
//! baseline is 0; only the characters dimensions need are defined (digits,
//! decimal comma, signs and a few letters for units).

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
        assert!(glyph('ñ').is_none());
    }

    /// With `ARCHISIM_GLYPHS=file.svg`, writes every glyph to look at them.
    #[test]
    fn glyph_sheet() {
        let Some(path) = std::env::var_os("ARCHISIM_GLYPHS") else {
            return;
        };
        let d = text_path(
            "0123456789 ±0,00 +3,45 -1.5m",
            Point2::new(0.0, 0.0),
            Point2::new(1.0, 0.0),
            10.0,
            Align::Left,
        );
        let svg = format!(
            r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="-5 -15 260 25"><rect x="-5" y="-15" width="260" height="25" fill="white"/><path d="{d}" fill="none" stroke="black" stroke-width="1" stroke-linecap="round" stroke-linejoin="round"/></svg>"#
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
        assert_eq!(text_width("ñ", 1.0), 0.0);
    }
}
