//! AD5's own tree drawings in plan (the studio's library, 26/10/2026), as
//! the styles «AD5 …»: each drawing is kept at a crown of radius 1 about
//! its centre and is laid on a plant turned at its own angle and stretched
//! to the outline of its crown, so every tree of a project looks like
//! AD5's drawing and none like its neighbour.
//!
//! The drawings are in `ad5_trees.txt`, written from AD5's DXF by
//! `scripts/make_ad5_trees.py` (the DXF is not in the repository).

use std::sync::OnceLock;

type P2 = [f64; 2];

/// How a line of the drawing is drawn.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum LineKind {
    Open,
    Closed,
    /// A closed line round the crown: drawn with the outline's pen.
    Outline,
}

/// One drawing of AD5, in units of its crown's radius.
#[derive(Debug, Default)]
pub(crate) struct Drawing {
    pub lines: Vec<(LineKind, Vec<P2>)>,
    /// Washes of colour, with AD5's opacity.
    pub washes: Vec<(f64, Vec<P2>)>,
    /// Where AD5 has a texture (a grass pattern): ArchiSim draws its own
    /// tufts there.
    pub patches: Vec<Vec<P2>>,
}

const DATA: &str = include_str!("ad5_trees.txt");

fn points(words: &[&str]) -> Vec<P2> {
    words
        .as_chunks::<2>()
        .0
        .iter()
        .filter_map(|w| Some([w[0].parse().ok()?, w[1].parse().ok()?]))
        .collect()
}

fn parse(text: &str) -> Vec<(String, Drawing)> {
    let mut out: Vec<(String, Drawing)> = Vec::new();
    for row in text.lines() {
        let row = row.trim();
        if row.is_empty() || row.starts_with('#') {
            continue;
        }
        let words: Vec<&str> = row.split_whitespace().collect();
        if words[0] == "symbol" {
            out.push((words[1..].join(" "), Drawing::default()));
            continue;
        }
        let Some((_, d)) = out.last_mut() else {
            continue;
        };
        match words[0] {
            "line" if words.len() > 1 => {
                let kind = match words[1] {
                    "outline" => LineKind::Outline,
                    "closed" => LineKind::Closed,
                    _ => LineKind::Open,
                };
                let pts = points(&words[2..]);
                if pts.len() >= 2 {
                    d.lines.push((kind, pts));
                }
            }
            "wash" if words.len() > 1 => {
                let op = words[1].parse().unwrap_or(0.3);
                let pts = points(&words[2..]);
                if pts.len() >= 3 {
                    d.washes.push((op, pts));
                }
            }
            "patch" => {
                let pts = points(&words[1..]);
                if pts.len() >= 3 {
                    d.patches.push(pts);
                }
            }
            _ => {}
        }
    }
    out
}

/// AD5's drawing of a name («círculos», «racimos», «estrella», «copa»).
pub(crate) fn drawing(name: &str) -> Option<&'static Drawing> {
    static ALL: OnceLock<Vec<(String, Drawing)>> = OnceLock::new();
    ALL.get_or_init(|| parse(DATA))
        .iter()
        .find(|(n, _)| n == name)
        .map(|(_, d)| d)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_four_drawings_are_there() {
        for name in ["círculos", "racimos", "estrella", "copa"] {
            let d = drawing(name).unwrap_or_else(|| panic!("falta «{name}»"));
            assert!(!d.lines.is_empty(), "{name}");
            // Within the crown (the copa's outline reaches a little past
            // its ring, as AD5 drew it).
            for (_, pts) in &d.lines {
                for p in pts {
                    assert!(p[0].hypot(p[1]) < 1.25, "{name}: {p:?}");
                }
            }
        }
        assert_eq!(drawing("círculos").unwrap().washes.len(), 3);
        assert!(!drawing("racimos").unwrap().patches.is_empty());
        assert!(drawing("estrella").unwrap().washes.is_empty());
    }
}
