//! Dimensions and level marks of a plan.
//!
//! Drawn as is usual in Spanish architectural drawings (and UNE-EN ISO
//! 129-1): thin dimension and extension lines, oblique ticks at 45° as
//! terminators, the dimension line running a little past the outer ticks,
//! and the value above the line, readable from the bottom or the right of
//! the sheet, in metres with two decimals and a decimal comma (`3,45`).
//! Level marks are a triangle half filled, its apex on the point, with the
//! elevation from project zero (`+0,35`, `±0,00`).

use std::fmt::Write as _;

use archisim_core::{Dimension, ElevationMark, Model, Point2};

use crate::lettering::{text_path, text_width, Align, STROKE};

/// Text height, paper millimetres.
pub const TEXT_HEIGHT: f64 = 2.5;
/// Half length of a tick, paper millimetres.
const TICK_HALF: f64 = 1.25;
/// Gap between the measured point and its extension line, paper mm.
const EXT_GAP: f64 = 1.5;
/// How far extension lines and the dimension line run past each other.
const OVERSHOOT: f64 = 2.0;
/// Space between the dimension line and the text, paper mm.
const TEXT_LIFT: f64 = 0.8;
/// Side of the level mark triangle, paper mm.
const MARK_SIDE: f64 = 3.0;
const INK: &str = "#1a1a1a";

/// A length in metres as written on a dimension: `3,45`.
pub fn format_dimension(v: f64) -> String {
    let v = (v * 100.0).round() / 100.0;
    let v = if v == 0.0 { 0.0 } else { v };
    format!("{v:.2}").replace('.', ",")
}

/// An elevation from project zero as written on a level mark: `+0,35`,
/// `-0,20`, `±0,00`.
pub fn format_elevation(v: f64) -> String {
    let r = (v * 100.0).round() / 100.0;
    if r == 0.0 {
        "±0,00".into()
    } else if r > 0.0 {
        format!("+{}", format_dimension(r))
    } else {
        format!("-{}", format_dimension(-r))
    }
}

/// The direction a dimension is read in: pointing right, or up when
/// vertical, so the text reads from the bottom or the right of the sheet.
pub fn reading_direction(d: Point2) -> Point2 {
    if d.x < -1e-9 || (d.x.abs() <= 1e-9 && d.y < 0.0) {
        -d
    } else {
        d
    }
}

/// The parts of a drawn dimension, in model metres.
#[derive(Debug, Clone, PartialEq)]
pub struct DimensionLines {
    /// The dimension line, including its overshoot past the outer ticks.
    pub line: (Point2, Point2),
    /// Extension lines, one per measured position.
    pub extensions: Vec<(Point2, Point2)>,
    /// Where each tick crosses the dimension line.
    pub ticks: Vec<Point2>,
    /// For each segment: the text, its anchor (middle of the baseline) and
    /// the reading direction.
    pub texts: Vec<(String, Point2, Point2)>,
}

/// Lays out a dimension at a scale where 1 paper mm is `k` metres.
pub fn dimension_lines(d: &Dimension, k: f64) -> DimensionLines {
    let u = reading_direction(d.direction);
    let n = u.perp_left();
    let c = d.position.dot(n);
    let at = |t: f64| u * t + n * c;
    let stations = crate::annotate::stations(&d.points, u);
    let mut extensions = Vec::new();
    let mut ticks = Vec::new();
    for &t in &stations {
        let foot = at(t);
        ticks.push(foot);
        // The point at this station nearest to the line.
        let nearest = d
            .points
            .iter()
            .filter(|p| (p.dot(u) - t).abs() <= 1e-6)
            .min_by(|a, b| (a.dot(n) - c).abs().total_cmp(&(b.dot(n) - c).abs()));
        if let Some(p) = nearest {
            let gap = c - p.dot(n);
            if gap.abs() > EXT_GAP * k {
                let e = n * gap.signum();
                extensions.push((*p + e * (EXT_GAP * k), foot + e * (OVERSHOOT * k)));
            }
        }
    }
    let (first, last) = match (stations.first(), stations.last()) {
        (Some(f), Some(l)) => (*f, *l),
        _ => (0.0, 0.0),
    };
    let line = (
        at(first) - u * (OVERSHOOT * k),
        at(last) + u * (OVERSHOOT * k),
    );
    let texts = stations
        .windows(2)
        .map(|w| {
            let mid = at((w[0] + w[1]) / 2.0) + n * (TEXT_LIFT * k);
            (format_dimension(w[1] - w[0]), mid, u)
        })
        .collect();
    DimensionLines {
        line,
        extensions,
        ticks,
        texts,
    }
}

/// Sorted positions of points along `u`, without repeats.
fn stations(points: &[Point2], u: Point2) -> Vec<f64> {
    let mut v: Vec<f64> = points.iter().map(|p| p.dot(u)).collect();
    v.sort_by(f64::total_cmp);
    v.dedup_by(|a, b| (*a - *b).abs() <= 1e-6);
    v
}

fn seg(s: &mut String, a: Point2, b: Point2) {
    let _ = write!(s, "M{} L{} ", pt(a), pt(b));
}

/// Draws a dimension.
pub fn draw_dimension(s: &mut String, d: &Dimension, k: f64) {
    let g = dimension_lines(d, k);
    let u = reading_direction(d.direction);
    let n = u.perp_left();
    let mut thin = String::new();
    seg(&mut thin, g.line.0, g.line.1);
    for (a, b) in &g.extensions {
        seg(&mut thin, *a, *b);
    }
    let mut ticks = String::new();
    let diag = (u + n) * (std::f64::consts::FRAC_1_SQRT_2 * TICK_HALF * k);
    for t in &g.ticks {
        seg(&mut ticks, *t - diag, *t + diag);
    }
    let h = crate::pens().text * k;
    let mut text = String::new();
    for (value, at, dir) in &g.texts {
        let _ = write!(text, "{} ", text_path(value, *at, *dir, h, Align::Centre));
    }
    let _ = writeln!(
        s,
        r#"<g class="dimension" fill="none" stroke="{INK}" stroke-linecap="round" stroke-linejoin="round"><path d="{}" stroke-width="{}"/><path d="{}" stroke-width="{}"/><path d="{}" stroke-width="{}"/></g>"#,
        thin.trim_end(),
        num(crate::pens().dimension * k),
        ticks.trim_end(),
        num(crate::pens().tick * k),
        text.trim_end(),
        num(STROKE * h)
    );
}

/// Draws a level mark with the floor elevation at its point.
pub fn draw_elevation_mark(s: &mut String, model: &Model, m: &ElevationMark, k: f64) {
    let value = model.floor_elevation_at(m.level, m.point).unwrap_or(0.0);
    let text = format_elevation(value);
    let a = MARK_SIDE * k;
    let h = a * 0.866;
    let p = m.point;
    let left = p + Point2::new(-a / 2.0, h);
    let right = p + Point2::new(a / 2.0, h);
    let top = p + Point2::new(0.0, h);
    let th = crate::pens().text * k;
    let text_at = right + Point2::new(TEXT_LIFT * k, TEXT_LIFT * k);
    let rule_end = text_at + Point2::new(text_width(&text, th), -TEXT_LIFT * k);
    let _ = writeln!(
        s,
        r#"<g class="level-mark" stroke="{INK}" stroke-linejoin="round" stroke-linecap="round"><path d="M{} L{} L{} Z" fill="{INK}" stroke-width="{}"/><path d="M{} L{} L{} Z M{} L{}" fill="none" stroke-width="{}"/><path d="{}" fill="none" stroke-width="{}"/></g>"#,
        pt(p),
        pt(left),
        pt(top),
        num(crate::pens().dimension * k),
        pt(p),
        pt(left),
        pt(right),
        pt(right),
        pt(rule_end),
        num(crate::pens().dimension * k),
        text_path(&text, text_at, Point2::new(1.0, 0.0), th, Align::Left),
        num(STROKE * th)
    );
}

fn num(v: f64) -> String {
    let v = (v * 1e6).round() / 1e6;
    if v == 0.0 {
        "0".into()
    } else {
        format!("{v}")
    }
}

fn pt(p: Point2) -> String {
    format!("{},{}", num(p.x), num(-p.y))
}

#[cfg(test)]
mod tests {
    use super::*;
    use archisim_core::{DimensionId, LevelId};

    #[test]
    fn formats() {
        assert_eq!(format_dimension(3.449), "3,45");
        assert_eq!(format_dimension(0.266), "0,27");
        assert_eq!(format_dimension(12.0), "12,00");
        assert_eq!(format_elevation(0.0), "±0,00");
        assert_eq!(format_elevation(0.001), "±0,00");
        assert_eq!(format_elevation(0.35), "+0,35");
        assert_eq!(format_elevation(-0.347), "-0,35");
    }

    #[test]
    fn a_chain_reads_from_the_bottom_whichever_way_it_was_drawn() {
        // A chain measured right to left, line below the points.
        let d = Dimension {
            id: DimensionId(1),
            level: LevelId(1),
            points: vec![
                Point2::new(6.0, 0.0),
                Point2::new(2.0, 0.0),
                Point2::new(0.0, 0.0),
            ],
            direction: Point2::new(-1.0, 0.0),
            position: Point2::new(3.0, -1.0),
            anchors: Vec::new(),
        };
        let g = dimension_lines(&d, 0.05);
        assert_eq!(g.ticks.len(), 3);
        assert_eq!(g.extensions.len(), 3);
        let texts: Vec<&str> = g.texts.iter().map(|t| t.0.as_str()).collect();
        assert_eq!(texts, vec!["2,00", "4,00"]);
        // Text above the line, written to the right.
        assert!(g
            .texts
            .iter()
            .all(|t| t.1.y > -1.0 && t.2 == Point2::new(1.0, 0.0)));
        // The line runs 2 mm (0.1 m at 1:50) past the outer ticks.
        assert!((g.line.0.x + 0.1).abs() < 1e-9 && (g.line.1.x - 6.1).abs() < 1e-9);
        // Extension lines leave a gap at the wall and pass the line.
        let (a, b) = g.extensions[0];
        assert!(
            (a.y + 0.075).abs() < 1e-9 && (b.y + 1.1).abs() < 1e-9,
            "{a:?} {b:?}"
        );
    }
}
