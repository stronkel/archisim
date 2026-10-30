//! The elevation profile of a wall (E3, Revit's «Editar perfil»): the
//! outline of the wall seen from its exterior face, as `(s, z)` points,
//! `s` along its drawn line from its start and `z` above its base, and
//! holes of any shape through it. With none, the wall is its rectangle
//! (length by height). Straight walls only.
//!
//! The wall keeps its length, height and joins; the profile is cut out of
//! it: in 3D what lies outside the outline or inside a hole goes, in plan
//! the stretches where the cut plane misses the wall are left open, and
//! in IFC the cuts are openings (`IfcOpeningElement`) with their shape.

use super::*;

/// The outline and holes of a wall in elevation, `x` = along it from its
/// start, `y` = above its base.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct WallProfile {
    /// Counter-clockwise, without repeating the first point.
    pub outline: Vec<Point2>,
    pub holes: Vec<Vec<Point2>>,
}

impl WallProfile {
    /// The wall's own rectangle, to start editing from.
    pub fn rectangle(length: f64, height: f64) -> Self {
        WallProfile {
            outline: vec![
                Point2::new(0.0, 0.0),
                Point2::new(length, 0.0),
                Point2::new(length, height),
                Point2::new(0.0, height),
            ],
            holes: Vec::new(),
        }
    }

    /// A gable ("hastial"): up from `side` at both ends to `ridge` in the
    /// middle.
    pub fn gable(length: f64, side: f64, ridge: f64) -> Self {
        WallProfile {
            outline: vec![
                Point2::new(0.0, 0.0),
                Point2::new(length, 0.0),
                Point2::new(length, side),
                Point2::new(length / 2.0, ridge),
                Point2::new(0.0, side),
            ],
            holes: Vec::new(),
        }
    }

    /// A sloping top: `start` high at its start, `end` at its end.
    pub fn sloping(length: f64, start: f64, end: f64) -> Self {
        WallProfile {
            outline: vec![
                Point2::new(0.0, 0.0),
                Point2::new(length, 0.0),
                Point2::new(length, end),
                Point2::new(0.0, start),
            ],
            holes: Vec::new(),
        }
    }

    /// Area of the wall in elevation (outline less holes).
    pub fn area(&self) -> f64 {
        signed_area(&self.outline).abs()
            - self.holes.iter().map(|h| signed_area(h).abs()).sum::<f64>()
    }

    /// Highest point of the outline.
    pub fn top(&self) -> f64 {
        self.outline.iter().map(|p| p.y).fold(0.0, f64::max)
    }

    /// The stretches along the wall, `(s0, s1)`, where a horizontal line at
    /// `z` (above the base) is in the wall: inside the outline and out of
    /// the holes. Sorted.
    pub fn solid_at(&self, z: f64) -> Vec<(f64, f64)> {
        let inside = spans_at(&self.outline, z);
        let mut out = inside;
        for h in &self.holes {
            let cut = spans_at(h, z);
            out = subtract_spans(&out, &cut);
        }
        out
    }

    /// The stretches of `[0, length]` where the line at `z` misses the
    /// wall (open in plan).
    pub fn gaps_at(&self, z: f64, length: f64) -> Vec<(f64, f64)> {
        subtract_spans(&[(0.0, length)], &self.solid_at(z))
            .into_iter()
            .filter(|(a, b)| b - a > 1e-6)
            .collect()
    }

    /// The same profile with `x` stretched by `k` (a wall made longer or
    /// shorter keeps its shape).
    pub fn stretched(&self, k: f64) -> Self {
        let f = |p: &Point2| Point2::new(p.x * k, p.y);
        WallProfile {
            outline: self.outline.iter().map(f).collect(),
            holes: self
                .holes
                .iter()
                .map(|h| h.iter().map(f).collect())
                .collect(),
        }
    }

    /// The same profile seen from the other end (a wall turned round).
    pub fn reversed(&self, length: f64) -> Self {
        let f = |p: &Point2| Point2::new(length - p.x, p.y);
        let mut outline: Vec<Point2> = self.outline.iter().map(f).collect();
        outline.reverse();
        WallProfile {
            outline,
            holes: self
                .holes
                .iter()
                .map(|h| {
                    let mut h: Vec<Point2> = h.iter().map(f).collect();
                    h.reverse();
                    h
                })
                .collect(),
        }
    }

    /// The part from `x0` to `x1`, moved so it starts at 0 (a wall split
    /// in two). `None` if nothing is left.
    pub fn part(&self, x0: f64, x1: f64) -> Option<Self> {
        let keep = |poly: &[Point2]| {
            let a = clip_x(poly, x0, true);
            let b = clip_x(&a, x1, false);
            (b.len() >= 3 && signed_area(&b).abs() > 1e-9).then(|| {
                b.iter()
                    .map(|p| Point2::new(p.x - x0, p.y))
                    .collect::<Vec<_>>()
            })
        };
        let outline = keep(&self.outline)?;
        Some(WallProfile {
            outline,
            holes: self.holes.iter().filter_map(|h| keep(h)).collect(),
        })
    }

    /// Where a vertical line at `x` is inside the outline: pairs of
    /// heights.
    pub fn spans_up(&self, x: f64) -> Vec<(f64, f64)> {
        let turned: Vec<Point2> = self.outline.iter().map(|p| Point2::new(p.y, p.x)).collect();
        spans_at(&turned, x)
    }

    /// Whether it is just the wall's rectangle (no need to keep it).
    pub fn is_rectangle(&self, length: f64, height: f64) -> bool {
        self.holes.is_empty()
            && (self.area() - length * height).abs() < 1e-6
            && self
                .outline
                .iter()
                .all(|p| p.x > -1e-6 && p.x < length + 1e-6 && p.y > -1e-6 && p.y < height + 1e-6)
    }
}

/// The part of `poly` with `x >= at` (`keep_right`) or `x <= at`.
fn clip_x(poly: &[Point2], at: f64, keep_right: bool) -> Vec<Point2> {
    let inside = |p: &Point2| if keep_right { p.x >= at } else { p.x <= at };
    let n = poly.len();
    let mut out = Vec::new();
    for i in 0..n {
        let (p, q) = (poly[i], poly[(i + 1) % n]);
        if inside(&p) {
            out.push(p);
        }
        if inside(&p) != inside(&q) {
            let t = (at - p.x) / (q.x - p.x);
            out.push(Point2::new(at, p.y + (q.y - p.y) * t));
        }
    }
    out
}

/// Where a horizontal line at `z` is inside `poly`: pairs of crossings.
fn spans_at(poly: &[Point2], z: f64) -> Vec<(f64, f64)> {
    let n = poly.len();
    let mut xs = Vec::new();
    for i in 0..n {
        let (p, q) = (poly[i], poly[(i + 1) % n]);
        // Half-open, so a vertex on the line counts once.
        if (p.y <= z) != (q.y <= z) {
            let t = (z - p.y) / (q.y - p.y);
            xs.push(p.x + (q.x - p.x) * t);
        }
    }
    xs.sort_by(f64::total_cmp);
    xs.chunks(2)
        .filter(|c| c.len() == 2 && c[1] - c[0] > 1e-9)
        .map(|c| (c[0], c[1]))
        .collect()
}

/// `a` less `b`, both sorted lists of stretches.
fn subtract_spans(a: &[(f64, f64)], b: &[(f64, f64)]) -> Vec<(f64, f64)> {
    let mut out = Vec::new();
    for &(mut s0, s1) in a {
        for &(c0, c1) in b {
            if c1 <= s0 || c0 >= s1 {
                continue;
            }
            if c0 > s0 {
                out.push((s0, c0));
            }
            s0 = s0.max(c1);
        }
        if s1 > s0 {
            out.push((s0, s1));
        }
    }
    out
}

impl Model {
    /// Gives a wall an elevation profile (or takes it away with `None`).
    /// A profile that is just the wall's rectangle is not kept.
    pub fn set_wall_profile(
        &mut self,
        id: WallId,
        profile: Option<WallProfile>,
    ) -> Result<(), ModelError> {
        let w = self.walls.get(&id).ok_or(ModelError::UnknownWall(id))?;
        let profile = match profile {
            Some(p) => {
                if w.is_curved() {
                    return Err(ModelError::InvalidWallProfile(id));
                }
                check_profile(id, &p)?;
                let mut p = p;
                if signed_area(&p.outline) < 0.0 {
                    p.outline.reverse();
                }
                (!p.is_rectangle(w.length(), w.height)).then_some(p)
            }
            None => None,
        };
        // A profile above the wall raises it (a gable over a wall of its
        // own height); one tied to a level is cut at its top.
        let raise = profile
            .as_ref()
            .map(|p| p.top())
            .filter(|t| *t > w.height + 1e-9 && matches!(w.top, WallTop::Unconnected));
        self.edit_wall(id, |w| {
            w.profile = profile;
            if let Some(t) = raise {
                w.height = t;
            }
        })
    }
}

/// Whether a profile can be built: points finite, an outline of three
/// points at least, with area, and holes likewise.
pub(super) fn check_profile(id: WallId, p: &WallProfile) -> Result<(), ModelError> {
    for q in p.outline.iter().chain(p.holes.iter().flatten()) {
        finite_point(*q, "wall profile")?;
    }
    if p.outline.len() < 3 || signed_area(&p.outline).abs() < 1e-6 {
        return Err(ModelError::InvalidWallProfile(id));
    }
    if p.holes
        .iter()
        .any(|h| h.len() < 3 || signed_area(h).abs() < 1e-6)
    {
        return Err(ModelError::InvalidWallProfile(id));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(x: f64, y: f64) -> Point2 {
        Point2::new(x, y)
    }

    #[test]
    fn gaps_parts_and_turns() {
        let mut g = WallProfile::gable(6.0, 2.0, 4.0);
        g.holes
            .push(vec![p(2.0, 1.0), p(4.0, 1.0), p(4.0, 2.0), p(2.0, 2.0)]);
        assert!((g.area() - (12.0 + 6.0 - 2.0)).abs() < 1e-9);
        // At 1,5 m: the hole between 2 and 4.
        assert_eq!(g.gaps_at(1.5, 6.0), vec![(2.0, 4.0)]);
        // At 3 m: only the middle of the gable, 1,5 to 4,5.
        let gaps = g.gaps_at(3.0, 6.0);
        assert_eq!(gaps.len(), 2);
        assert!((gaps[0].1 - 1.5).abs() < 1e-9 && (gaps[1].0 - 4.5).abs() < 1e-9);
        // Split at 3: each half takes half the hole.
        let left = g.part(0.0, 3.0).unwrap();
        let right = g.part(3.0, 6.0).unwrap();
        assert!((left.area() + right.area() - g.area()).abs() < 1e-9);
        assert_eq!(right.holes.len(), 1);
        assert!(right.holes[0]
            .iter()
            .all(|q| q.x >= -1e-9 && q.x <= 1.0 + 1e-9));
        // Turned round and stretched.
        let r = WallProfile::sloping(4.0, 3.0, 2.0).reversed(4.0);
        assert_eq!(r.solid_at(2.5), vec![(2.0, 4.0)]);
        let s = r.stretched(2.0);
        assert_eq!(s.solid_at(2.5), vec![(4.0, 8.0)]);
        assert!(WallProfile::rectangle(5.0, 3.0).is_rectangle(5.0, 3.0));
        assert!(!g.is_rectangle(6.0, 4.0));
    }
}
