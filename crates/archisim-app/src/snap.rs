//! Point snapping while drawing ("puntos especiales" / "forzado a objeto").
//!
//! Points win over edges: wall ends and corners first, then midpoints, then
//! the nearest point on a wall face or slab edge. Anything else is rounded
//! to [`FREE_STEP`] so free coordinates stay readable.

use archisim_core::{LevelId, Model, Point2};
use archisim_geom::wall_geometry;

use std::collections::BTreeSet;

use crate::pick::{nearest_on_segment, Element};

/// Free points and typed lengths are rounded to 1 cm.
pub const FREE_STEP: f64 = 0.01;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SnapKind {
    /// End of a wall's drawn line, or a point of the wall being drawn.
    Endpoint,
    /// Corner of a wall outline or of a slab.
    Corner,
    /// Middle of a wall's drawn line.
    Midpoint,
    /// Somewhere on a wall face or slab edge.
    OnEdge,
    /// Along the constrained direction (Shift).
    Constrained,
    /// Locked onto a horizontal, vertical or 45° line through the previous
    /// point because the cursor was close to it.
    Tracked,
    /// Nothing nearby: rounded free point.
    Free,
}

impl SnapKind {
    pub fn label(self) -> &'static str {
        match self {
            SnapKind::Endpoint => "Extremo",
            SnapKind::Corner => "Esquina",
            SnapKind::Midpoint => "Punto medio",
            SnapKind::OnEdge => "Sobre arista",
            SnapKind::Constrained => "Ángulo forzado",
            SnapKind::Tracked => "Alineado",
            SnapKind::Free => "Libre",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Snap {
    pub point: Point2,
    pub kind: SnapKind,
}

fn round_to(v: f64, step: f64) -> f64 {
    (v / step).round() * step
}

/// Rounds a free point to the [`FREE_STEP`] grid.
pub fn round_point(p: Point2) -> Point2 {
    Point2::new(round_to(p.x, FREE_STEP), round_to(p.y, FREE_STEP))
}

/// Snaps `cursor` to the elements of `level` within `tolerance` metres.
/// `extra` are additional end points (the chain being drawn).
pub fn snap(
    model: &Model,
    level: LevelId,
    cursor: Point2,
    tolerance: f64,
    extra: &[Point2],
) -> Snap {
    snap_except(model, level, cursor, tolerance, extra, &BTreeSet::new())
}

/// Which object snaps are on (the toggles of the bottom bar). Points that
/// are switched off are not offered; the cursor then falls back to the
/// next kind, or to a free point.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SnapFilter {
    /// Ends of walls and corners of walls and slabs.
    pub points: bool,
    /// Middles of walls.
    pub midpoints: bool,
    /// Anywhere on a wall face or slab edge.
    pub edges: bool,
    /// Polar tracking of horizontal, vertical and 45° lines.
    pub tracking: bool,
}

impl Default for SnapFilter {
    fn default() -> Self {
        Self {
            points: true,
            midpoints: true,
            edges: true,
            tracking: true,
        }
    }
}

impl SnapFilter {
    fn allows(self, kind: SnapKind) -> bool {
        match kind {
            SnapKind::Endpoint | SnapKind::Corner => self.points,
            SnapKind::Midpoint => self.midpoints,
            SnapKind::OnEdge => self.edges,
            SnapKind::Constrained | SnapKind::Tracked | SnapKind::Free => true,
        }
    }
}

/// [`snap`], ignoring some walls and slabs (the ones being dragged, so a
/// point does not snap to where it already is).
pub fn snap_except(
    model: &Model,
    level: LevelId,
    cursor: Point2,
    tolerance: f64,
    extra: &[Point2],
    skip: &BTreeSet<Element>,
) -> Snap {
    snap_filtered(
        model,
        level,
        cursor,
        tolerance,
        extra,
        skip,
        SnapFilter::default(),
    )
}

/// [`snap_except`] with only the snaps `filter` lets through.
pub fn snap_filtered(
    model: &Model,
    level: LevelId,
    cursor: Point2,
    tolerance: f64,
    extra: &[Point2],
    skip: &BTreeSet<Element>,
    filter: SnapFilter,
) -> Snap {
    let mut points: Vec<(Point2, SnapKind)> =
        extra.iter().map(|p| (*p, SnapKind::Endpoint)).collect();
    let mut edges: Vec<(Point2, Point2)> = Vec::new();
    for w in model.walls_on_level(level) {
        if skip.contains(&Element::Wall(w.id)) {
            continue;
        }
        points.push((w.start, SnapKind::Endpoint));
        points.push((w.end, SnapKind::Endpoint));
        points.push(((w.start + w.end) * 0.5, SnapKind::Midpoint));
        if let Some(g) = wall_geometry(model, w.id) {
            let n = g.outline.len();
            for i in 0..n {
                points.push((g.outline[i], SnapKind::Corner));
                edges.push((g.outline[i], g.outline[(i + 1) % n]));
            }
        }
    }
    for s in model.slabs_on_level(level) {
        if skip.contains(&Element::Slab(s.id)) {
            continue;
        }
        let n = s.outline.len();
        for i in 0..n {
            points.push((s.outline[i], SnapKind::Corner));
            edges.push((s.outline[i], s.outline[(i + 1) % n]));
        }
    }

    // Closest point within tolerance; on a tie, the stronger kind (listed
    // first in `SnapKind`) wins, so a wall end beats the corner under it.
    let best_point = points
        .iter()
        .filter(|(_, k)| filter.allows(*k))
        .map(|(p, k)| (p.distance(cursor), *k, *p))
        .filter(|(d, _, _)| *d <= tolerance)
        .min_by(|a, b| {
            let close = (a.0 - b.0).abs() < 1e-6;
            if close {
                (a.1 as u8).cmp(&(b.1 as u8))
            } else {
                a.0.total_cmp(&b.0)
            }
        });
    if let Some((_, kind, point)) = best_point {
        return Snap { point, kind };
    }
    let edges = if filter.edges { edges } else { Vec::new() };
    let best_edge = edges
        .iter()
        .map(|(a, b)| nearest_on_segment(*a, *b, cursor))
        .map(|p| (p.distance(cursor), p))
        .filter(|(d, _)| *d <= tolerance)
        .min_by(|a, b| a.0.total_cmp(&b.0));
    if let Some((_, point)) = best_edge {
        return Snap {
            point,
            kind: SnapKind::OnEdge,
        };
    }
    Snap {
        point: round_point(cursor),
        kind: SnapKind::Free,
    }
}

/// Constrains `p` to the nearest multiple of 45° from `from` (Shift held),
/// keeping its distance along that direction rounded to [`FREE_STEP`]
/// unless it came from an object snap (then it is projected exactly, which
/// aligns with that point).
pub fn constrain(from: Point2, p: Snap) -> Snap {
    let d = p.point - from;
    if d.length() < 1e-9 {
        return p;
    }
    let step = std::f64::consts::FRAC_PI_4;
    let angle = (d.y.atan2(d.x) / step).round() * step;
    let dir = Point2::new(angle.cos(), angle.sin());
    // Exact axis directions avoid 1e-17 noise in coordinates.
    let dir = Point2::new(clean(dir.x), clean(dir.y));
    let mut along = d.dot(dir);
    if p.kind == SnapKind::Free {
        along = round_to(along, FREE_STEP);
    }
    Snap {
        point: from + dir * along,
        kind: if p.kind == SnapKind::Free {
            SnapKind::Constrained
        } else {
            p.kind
        },
    }
}

/// Polar tracking: a free point within `reach` metres of a horizontal,
/// vertical or 45° line through `from` is moved onto it, so walls come out
/// straight without holding Shift. Object snaps are left alone.
pub fn track(from: Point2, p: Snap, reach: f64) -> Snap {
    if p.kind != SnapKind::Free {
        return p;
    }
    let on_line = constrain(from, p);
    if on_line.point.distance(p.point) <= reach {
        Snap {
            point: on_line.point,
            kind: SnapKind::Tracked,
        }
    } else {
        p
    }
}

fn clean(v: f64) -> f64 {
    if v.abs() < 1e-12 {
        0.0
    } else if (v.abs() - 1.0).abs() < 1e-12 {
        v.signum()
    } else {
        v
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pick::tests::room;

    #[test]
    fn snaps_to_points_then_edges_then_rounds() {
        let (m, lvl, _) = room();
        // Near the north-west corner: the wall end (0, 4) wins over the
        // outline corner on the same spot.
        let s = snap(&m, lvl, Point2::new(0.03, 3.98), 0.1, &[]);
        assert_eq!(
            s,
            Snap {
                point: Point2::new(0.0, 4.0),
                kind: SnapKind::Endpoint
            }
        );
        // Midpoint of the west wall's drawn line.
        let s = snap(&m, lvl, Point2::new(0.05, 2.02), 0.1, &[]);
        assert_eq!(s.kind, SnapKind::Midpoint);
        assert_eq!(s.point, Point2::new(0.0, 2.0));
        // On the interior face of the west wall (x = 0.265), away from points.
        let s = snap(&m, lvl, Point2::new(0.3, 1.2), 0.1, &[]);
        assert_eq!(s.kind, SnapKind::OnEdge);
        assert!((s.point.x - 0.265).abs() < 1e-9 && (s.point.y - 1.2).abs() < 1e-9);
        // Free space: rounded to the centimetre.
        let s = snap(&m, lvl, Point2::new(2.3456, 2.1234), 0.1, &[]);
        assert_eq!(s.kind, SnapKind::Free);
        assert!(s.point.distance(Point2::new(2.35, 2.12)) < 1e-9);
        // Points of the chain being drawn.
        let s = snap(
            &m,
            lvl,
            Point2::new(10.02, 10.0),
            0.1,
            &[Point2::new(10.0, 10.0)],
        );
        assert_eq!(s.point, Point2::new(10.0, 10.0));
    }

    #[test]
    fn snaps_switched_off_are_not_offered() {
        let (m, lvl, _) = room();
        let skip = BTreeSet::new();
        let at = |p: Point2, f: SnapFilter| snap_filtered(&m, lvl, p, 0.1, &[], &skip, f);
        // Without points, the wall end falls back to the face under it.
        let no_points = SnapFilter {
            points: false,
            ..SnapFilter::default()
        };
        assert_eq!(
            at(Point2::new(0.03, 3.98), no_points).kind,
            SnapKind::OnEdge
        );
        // Without midpoints, the middle of the west wall is a free point.
        let no_mid = SnapFilter {
            midpoints: false,
            edges: false,
            ..SnapFilter::default()
        };
        assert_eq!(at(Point2::new(0.05, 2.02), no_mid).kind, SnapKind::Free);
        // Without edges, a point on a face is free (rounded).
        let no_edges = SnapFilter {
            edges: false,
            ..SnapFilter::default()
        };
        assert_eq!(at(Point2::new(0.3, 1.2), no_edges).kind, SnapKind::Free);
        // Everything on is plain snapping.
        let p = Point2::new(0.05, 2.02);
        assert_eq!(at(p, SnapFilter::default()), snap(&m, lvl, p, 0.1, &[]));
    }

    #[test]
    fn tracking_locks_near_straight_lines_only() {
        let from = Point2::new(1.0, 1.0);
        let free = |x, y| Snap {
            point: Point2::new(x, y),
            kind: SnapKind::Free,
        };
        // 3 cm off the horizontal with a 5 cm reach: locked.
        let s = track(from, free(5.0, 1.03), 0.05);
        assert_eq!(
            s,
            Snap {
                point: Point2::new(5.0, 1.0),
                kind: SnapKind::Tracked
            }
        );
        // 30 cm off: left alone.
        assert_eq!(track(from, free(5.0, 1.3), 0.05), free(5.0, 1.3));
        // Object snaps win.
        let corner = Snap {
            point: Point2::new(5.0, 1.03),
            kind: SnapKind::Corner,
        };
        assert_eq!(track(from, corner, 0.05), corner);
    }

    #[test]
    fn constrains_to_45_degrees() {
        let from = Point2::new(1.0, 1.0);
        let free = |x, y| Snap {
            point: Point2::new(x, y),
            kind: SnapKind::Free,
        };
        let s = constrain(from, free(4.004, 1.3));
        assert_eq!(s.kind, SnapKind::Constrained);
        assert_eq!(s.point, Point2::new(4.0, 1.0));
        let s = constrain(from, free(1.2, -2.0));
        assert_eq!(s.point, Point2::new(1.0, -2.0));
        let s = constrain(from, free(3.0, 3.1));
        assert!((s.point.x - s.point.y).abs() < 1e-9);
        // An object snap is projected exactly: aligning with that point.
        let s = constrain(
            from,
            Snap {
                point: Point2::new(5.0, 1.234567),
                kind: SnapKind::Endpoint,
            },
        );
        assert_eq!(s.point, Point2::new(5.0, 1.0));
    }
}
