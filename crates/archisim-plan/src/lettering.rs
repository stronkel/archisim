//! Single-stroke technical lettering for dimensions and level marks.
//!
//! Text is drawn as stroked paths, in the spirit of the normalised
//! technical lettering of drawings (ISO 3098, upright): the same strokes on
//! screen, in the exported SVG and later in PDF, with no dependency on the
//! fonts installed on the machine. The glyphs are ArchiSim's own design.
//!
//! The glyphs are those of [`archisim_draft::font`], which has every
//! character of Spanish texts; this module keeps the simple one-line API
//! used by dimensions and level marks.

use std::fmt::Write as _;

use archisim_core::Point2;

use archisim_core::TextFont;
use archisim_draft::font::{glyph, GAP};
pub use archisim_draft::font::{text_width, use_font, STROKE};
use archisim_draft::outline::Seg;

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
    if archisim_draft::font::current() == TextFont::Sans {
        let start = match align {
            Align::Left => 0.0,
            Align::Centre => -text_width(text, h) / 2.0,
        };
        let segs = archisim_draft::outline::line(text, h, |x, y| at + dir * (start + x) + up * y);
        return outline_path(&segs);
    }
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

/// SVG path data of a text of the drawing (several lines, any angle and
/// alignment), in the lettering in use: `k` turns paper millimetres into
/// drawing units. Paint it with [`paint`] at `t.height * k`.
pub fn text_d(t: &archisim_core::Text, k: f64) -> String {
    if archisim_draft::font::current() == TextFont::Sans {
        return outline_path(&archisim_draft::font::text_outline(t, k));
    }
    let mut d = String::new();
    for st in archisim_draft::font::text_strokes(t, k) {
        for (i, p) in st.iter().enumerate() {
            let _ = write!(
                d,
                "{}{},{} ",
                if i == 0 { "M" } else { "L" },
                num(p.x),
                num(-p.y)
            );
        }
    }
    d.trim_end().to_string()
}

/// SVG path data of outlines (y flipped, like every plan coordinate).
pub fn outline_path(segs: &[Seg]) -> String {
    let mut d = String::new();
    let p = |q: Point2| format!("{},{}", num(q.x), num(-q.y));
    for s in segs {
        let _ = match *s {
            Seg::Move(a) => write!(d, "M{} ", p(a)),
            Seg::Line(a) => write!(d, "L{} ", p(a)),
            Seg::Quad(c, a) => write!(d, "Q{} {} ", p(c), p(a)),
            Seg::Cubic(c1, c2, a) => write!(d, "C{} {} {} ", p(c1), p(c2), p(a)),
            Seg::Close => write!(d, "Z "),
        };
    }
    d.trim_end().to_string()
}

/// The paint of a text of height `h` in `color`: filled outlines in the
/// sans lettering, round strokes in the technical one.
pub fn paint(h: f64, color: &str) -> String {
    match archisim_draft::font::current() {
        TextFont::Sans => format!(r#"fill="{color}" stroke="none""#),
        TextFont::Technical => format!(
            r#"fill="none" stroke="{color}" stroke-width="{}" stroke-linecap="round" stroke-linejoin="round""#,
            num(STROKE * h)
        ),
    }
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
        assert!(glyph('ñ').is_some());
    }

    /// With `ARCHISIM_GLYPHS=file.svg`, writes every glyph to look at them.
    #[test]
    fn glyph_sheet() {
        let _f = use_font(TextFont::Technical);
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
        let _f = use_font(TextFont::Technical);
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
        assert_eq!(text_width("\u{1F600}", 1.0), 0.0);
        // The sans lettering: filled outlines, about as wide.
        drop(_f);
        let _s = use_font(TextFont::Sans);
        let sans = text_width("3,45", 0.1);
        assert!((sans - w).abs() < w * 0.5, "{sans} {w}");
        let d = text_path(
            "88",
            Point2::new(10.0, 0.0),
            Point2::new(1.0, 0.0),
            0.1,
            Align::Left,
        );
        assert!(d.contains('Q') && d.contains('Z'), "{d}");
        assert!(paint(0.1, "#000").starts_with("fill=\"#000\""));
    }
}
