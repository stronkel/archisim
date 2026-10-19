//! A topographic survey from a drawing: its points with their heights and
//! its contour lines, from the layers chosen.
//!
//! - A `POINT` is a survey point; so is a block placed (`INSERT`), at its
//!   insertion point, with the height of its insertion or, if that is 0,
//!   of an attribute that is a number (`COTA`, `Z`, `ELEV`… first).
//! - The corners of a `3DFACE` (a triangulated survey) are points.
//! - The vertices of lines and polylines are points; a polyline all at one
//!   height (not 0) is also a contour line.
//! - Points at height 0 are taken as having none (drawn flat): they are
//!   left out, or, if asked, take the height of the nearest text that is a
//!   number («643,25»), as many surveys label their points.

use crate::{Drawing, Shape, P3};

/// What the survey takes from a drawing.
#[derive(Debug, Clone, PartialEq)]
pub struct SurveyOptions {
    /// The layers to read; empty: all.
    pub layers: Vec<String>,
    /// Points with no height take that of the nearest number written
    /// within `text_reach`.
    pub heights_from_texts: bool,
    pub text_reach: f64,
    /// Points closer than this to one already taken are left out (and the
    /// vertices of contour lines thinned likewise); 0: all.
    pub min_spacing: f64,
}

impl Default for SurveyOptions {
    fn default() -> Self {
        Self {
            layers: Vec::new(),
            heights_from_texts: false,
            text_reach: 2.0,
            min_spacing: 0.0,
        }
    }
}

/// A survey: points with their heights, and contour lines (each at one
/// height), in the drawing's units.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Survey {
    pub points: Vec<P3>,
    pub lines: Vec<Vec<P3>>,
}

impl Survey {
    /// Lowest and highest height, of points and lines.
    pub fn z_range(&self) -> Option<(f64, f64)> {
        self.all().fold(None, |acc, p| match acc {
            None => Some((p[2], p[2])),
            Some((lo, hi)) => Some((lo.min(p[2]), hi.max(p[2]))),
        })
    }

    /// The box in plan: lower-left and upper-right corners.
    pub fn bounds(&self) -> Option<([f64; 2], [f64; 2])> {
        self.all().fold(None, |acc, p| match acc {
            None => Some(([p[0], p[1]], [p[0], p[1]])),
            Some((lo, hi)) => Some((
                [lo[0].min(p[0]), lo[1].min(p[1])],
                [hi[0].max(p[0]), hi[1].max(p[1])],
            )),
        })
    }

    /// The interval between contour lines, if they have one: the most
    /// usual difference between consecutive heights.
    pub fn contour_interval(&self) -> Option<f64> {
        let mut zs: Vec<f64> = self
            .lines
            .iter()
            .filter_map(|l| l.first())
            .map(|p| p[2])
            .collect();
        zs.sort_by(f64::total_cmp);
        zs.dedup_by(|a, b| (*a - *b).abs() < 1e-6);
        let mut steps: Vec<f64> = zs
            .windows(2)
            .map(|w| ((w[1] - w[0]) * 1000.0).round() / 1000.0)
            .collect();
        steps.sort_by(f64::total_cmp);
        let mut best: Option<(f64, usize)> = None;
        for s in &steps {
            let n = steps.iter().filter(|t| (*t - s).abs() < 1e-9).count();
            if best.is_none_or(|b| n > b.1) {
                best = Some((*s, n));
            }
        }
        best.map(|b| b.0).filter(|s| *s > 0.0)
    }

    fn all(&self) -> impl Iterator<Item = &P3> {
        self.points.iter().chain(self.lines.iter().flatten())
    }
}

/// What a layer of the drawing has for a survey.
#[derive(Debug, Clone, PartialEq)]
pub struct LayerSummary {
    pub name: String,
    /// Points (points, blocks, corners of faces, vertices) with a height.
    pub points: usize,
    /// Points with no height (at 0).
    pub flat: usize,
    /// Polylines all at one height (contour lines).
    pub contours: usize,
    /// Texts that are a number (height labels).
    pub numbers: usize,
    pub z: Option<(f64, f64)>,
}

impl LayerSummary {
    /// Whether it looks like part of the survey: points or contour lines
    /// with a height.
    pub fn looks_topographic(&self) -> bool {
        self.points > 0 || self.contours > 0
    }
}

/// A number as surveys write them: «643.25», «643,25», «+643,25 m».
pub fn number(text: &str) -> Option<f64> {
    let t = text.trim().trim_end_matches('m').trim().replace(',', ".");
    let t = t.strip_prefix('+').unwrap_or(&t);
    let v: f64 = t.parse().ok()?;
    v.is_finite().then_some(v)
}

/// Height of a placed block: its own, or a numeric attribute's.
fn insert_height(at: P3, attribs: &[(String, String)]) -> P3 {
    if at[2] != 0.0 {
        return at;
    }
    let preferred = ["COTA", "Z", "ELEV", "ALT", "H"];
    let pick = attribs
        .iter()
        .filter(|(t, _)| preferred.iter().any(|p| t.to_uppercase().contains(p)))
        .chain(attribs.iter())
        .find_map(|(_, v)| number(v));
    [at[0], at[1], pick.unwrap_or(0.0)]
}

/// The points and the one-height polylines of an entity.
fn parts(shape: &Shape) -> (Vec<P3>, Option<Vec<P3>>) {
    match shape {
        Shape::Point(p) => (vec![*p], None),
        Shape::Insert(_, at, attribs) => (vec![insert_height(*at, attribs)], None),
        Shape::Face(ps) => (ps.clone(), None),
        Shape::Line(a, b) => (vec![*a, *b], None),
        Shape::Polyline(ps, closed) => {
            let level = ps.first().map(|p| p[2]).filter(|z| *z != 0.0);
            let flat = level.is_some_and(|z| ps.iter().all(|p| (p[2] - z).abs() < 1e-6));
            let line = (flat && ps.len() >= 2).then(|| {
                let mut l = ps.clone();
                if *closed {
                    l.push(ps[0]);
                }
                l
            });
            (ps.clone(), line)
        }
        Shape::Text(..) => (Vec::new(), None),
    }
}

/// What each layer of a drawing has, by name.
pub fn layers(drawing: &Drawing) -> Vec<LayerSummary> {
    let mut out: Vec<LayerSummary> = Vec::new();
    for e in &drawing.entities {
        let i = match out.iter().position(|l| l.name == e.layer) {
            Some(i) => i,
            None => {
                out.push(LayerSummary {
                    name: e.layer.clone(),
                    points: 0,
                    flat: 0,
                    contours: 0,
                    numbers: 0,
                    z: None,
                });
                out.len() - 1
            }
        };
        let l = &mut out[i];
        if let Shape::Text(_, t) = &e.shape {
            if number(t).is_some() {
                l.numbers += 1;
            }
            continue;
        }
        let (pts, line) = parts(&e.shape);
        if line.is_some() {
            l.contours += 1;
        }
        for p in pts {
            if p[2] == 0.0 {
                l.flat += 1;
            } else {
                l.points += 1;
                l.z = Some(match l.z {
                    None => (p[2], p[2]),
                    Some((lo, hi)) => (lo.min(p[2]), hi.max(p[2])),
                });
            }
        }
    }
    out.sort_by(|a, b| a.name.cmp(&b.name));
    out
}

/// The survey of a drawing, from the layers asked for.
pub fn survey(drawing: &Drawing, options: &SurveyOptions) -> Survey {
    let wanted =
        |layer: &str| options.layers.is_empty() || options.layers.iter().any(|l| l == layer);
    let mut points: Vec<P3> = Vec::new();
    let mut flat: Vec<P3> = Vec::new();
    let mut lines: Vec<Vec<P3>> = Vec::new();
    let mut labels: Vec<([f64; 2], f64)> = Vec::new();
    for e in drawing.entities.iter().filter(|e| wanted(&e.layer)) {
        if let Shape::Text(at, t) = &e.shape {
            if let Some(v) = number(t) {
                labels.push(([at[0], at[1]], v));
            }
            continue;
        }
        let (pts, line) = parts(&e.shape);
        if let Some(l) = line {
            lines.push(l);
            continue;
        }
        for p in pts {
            if p[2] == 0.0 {
                flat.push(p);
            } else {
                points.push(p);
            }
        }
    }
    // Heights from the labels next to points drawn flat.
    if options.heights_from_texts && !labels.is_empty() {
        let reach2 = options.text_reach * options.text_reach;
        for p in flat {
            let near = labels
                .iter()
                .map(|(at, v)| ((at[0] - p[0]).powi(2) + (at[1] - p[1]).powi(2), *v))
                .filter(|(d, _)| *d <= reach2)
                .min_by(|a, b| a.0.total_cmp(&b.0));
            if let Some((_, z)) = near {
                points.push([p[0], p[1], z]);
            }
        }
    }
    let spacing = options.min_spacing;
    if spacing > 0.0 {
        points = thin(points, spacing);
        lines = lines
            .into_iter()
            .map(|l| {
                let mut out: Vec<P3> = Vec::new();
                let n = l.len();
                for (i, p) in l.into_iter().enumerate() {
                    let far = out
                        .last()
                        .is_none_or(|q: &P3| (q[0] - p[0]).hypot(q[1] - p[1]) >= spacing);
                    if far || i + 1 == n {
                        out.push(p);
                    }
                }
                out
            })
            .filter(|l| l.len() >= 2)
            .collect();
    }
    Survey { points, lines }
}

/// Points no closer than `spacing` to each other (the first one kept).
fn thin(points: Vec<P3>, spacing: f64) -> Vec<P3> {
    use std::collections::HashMap;
    let key = |p: &P3| {
        (
            (p[0] / spacing).floor() as i64,
            (p[1] / spacing).floor() as i64,
        )
    };
    let mut grid: HashMap<(i64, i64), Vec<P3>> = HashMap::new();
    let mut out = Vec::new();
    for p in points {
        let (i, j) = key(&p);
        let close = (-1..=1).any(|di| {
            (-1..=1).any(|dj| {
                grid.get(&(i + di, j + dj))
                    .is_some_and(|v| v.iter().any(|q| (q[0] - p[0]).hypot(q[1] - p[1]) < spacing))
            })
        });
        if !close {
            grid.entry((i, j)).or_default().push(p);
            out.push(p);
        }
    }
    out
}
