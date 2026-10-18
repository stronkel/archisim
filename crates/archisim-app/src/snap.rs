//! Point snapping while drawing ("puntos especiales" / "forzado a objeto").
//!
//! Points win over edges: wall ends and corners first, then midpoints, then
//! the nearest point on a wall face or slab edge. Anything else is rounded
//! to [`FREE_STEP`] so free coordinates stay readable.

use archisim_core::{LevelId, Model, Point2, WallId};
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
    /// Centre of a curved wall.
    Centre,
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
            SnapKind::Centre => "Centro",
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
    /// For [`SnapKind::OnEdge`], the edge the point is on (so tracking can
    /// slide the point along it to stay square).
    pub edge: Option<(Point2, Point2)>,
}

impl Snap {
    /// A snapped point that is not on an edge.
    pub fn at(point: Point2, kind: SnapKind) -> Self {
        Self {
            point,
            kind,
            edge: None,
        }
    }
}

/// How far from horizontal or vertical the cursor may be, seen from the
/// previous point, for the line to lock square (Revit's and ArchiCAD's
/// orthogonal tracking, without holding Shift).
pub const ORTHO_LOCK: f64 = 8.0 * std::f64::consts::PI / 180.0;

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
            SnapKind::Endpoint | SnapKind::Corner | SnapKind::Centre => self.points,
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
        &[],
    )
}

/// Pushes the points and edges of a closed outline: its corners, the
/// middle of every edge and the edges themselves.
fn outline_snaps(
    outline: &[Point2],
    points: &mut Vec<(Point2, SnapKind)>,
    edges: &mut Vec<(Point2, Point2)>,
) {
    let n = outline.len();
    for i in 0..n {
        let (a, b) = (outline[i], outline[(i + 1) % n]);
        points.push((a, SnapKind::Corner));
        if a.distance(b) > 1e-6 {
            points.push(((a + b) * 0.5, SnapKind::Midpoint));
            edges.push((a, b));
        }
    }
}

/// [`snap_except`] with only the snaps `filter` lets through. `others` are
/// walls of other levels shown in this plan (cut by its cut plane, or the
/// reference of the level below): they snap too, so new walls can be drawn
/// on top of them.
#[allow(clippy::too_many_arguments)]
pub fn snap_filtered(
    model: &Model,
    level: LevelId,
    cursor: Point2,
    tolerance: f64,
    extra: &[Point2],
    skip: &BTreeSet<Element>,
    filter: SnapFilter,
    others: &[WallId],
) -> Snap {
    let mut points: Vec<(Point2, SnapKind)> =
        extra.iter().map(|p| (*p, SnapKind::Endpoint)).collect();
    let mut edges: Vec<(Point2, Point2)> = Vec::new();
    let walls = model
        .walls_on_level(level)
        .map(|w| w.id)
        .chain(others.iter().copied());
    for id in walls {
        if skip.contains(&Element::Wall(id)) {
            continue;
        }
        let Some(w) = model.wall(id) else {
            continue;
        };
        points.push((w.start, SnapKind::Endpoint));
        points.push((w.end, SnapKind::Endpoint));
        points.push((w.midpoint(), SnapKind::Midpoint));
        let Some(g) = wall_geometry(model, id) else {
            continue;
        };
        match &g.arc {
            Some(arc) => {
                // A curve: its centre, the corners at its ends and the
                // middle of each face; anywhere along the faces.
                points.push((arc.centre, SnapKind::Centre));
                let (s, e) = g.end_points();
                for p in s.iter().chain(e) {
                    points.push((*p, SnapKind::Corner));
                }
                for off in [g.exterior_offset, g.interior_offset] {
                    points.push((arc.point(0.5, off), SnapKind::Midpoint));
                }
                let n = g.outline.len();
                for i in 0..n {
                    let (a, b) = (g.outline[i], g.outline[(i + 1) % n]);
                    if a.distance(b) > 1e-6 {
                        edges.push((a, b));
                    }
                }
            }
            None => outline_snaps(&g.outline, &mut points, &mut edges),
        }
    }
    for s in model.slabs_on_level(level) {
        if skip.contains(&Element::Slab(s.id)) {
            continue;
        }
        outline_snaps(&s.outline, &mut points, &mut edges);
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
        return Snap::at(point, kind);
    }
    let edges = if filter.edges { edges } else { Vec::new() };
    let best_edge = edges
        .iter()
        .map(|(a, b)| (nearest_on_segment(*a, *b, cursor), (*a, *b)))
        .map(|(p, e)| (p.distance(cursor), p, e))
        .filter(|(d, _, _)| *d <= tolerance)
        .min_by(|a, b| a.0.total_cmp(&b.0));
    if let Some((_, point, edge)) = best_edge {
        return Snap {
            point,
            kind: SnapKind::OnEdge,
            edge: Some(edge),
        };
    }
    Snap::at(round_point(cursor), SnapKind::Free)
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
    Snap::at(
        from + dir * along,
        if p.kind == SnapKind::Free {
            SnapKind::Constrained
        } else {
            p.kind
        },
    )
}

/// The horizontal or vertical direction within [`ORTHO_LOCK`] of `d`.
fn ortho_dir(d: Point2) -> Option<Point2> {
    let step = std::f64::consts::FRAC_PI_2;
    let angle = d.y.atan2(d.x);
    let square = (angle / step).round() * step;
    ((angle - square).abs() <= ORTHO_LOCK).then(|| {
        let dir = Point2::new(square.cos(), square.sin());
        Point2::new(clean(dir.x), clean(dir.y))
    })
}

/// Polar tracking, so walls come out square without holding Shift:
/// - a free point seen from `from` within [`ORTHO_LOCK`] of horizontal or
///   vertical goes onto that line, and within `reach` metres of a 45° line,
///   onto that one;
/// - a point on a wall face or slab edge slides along it to where the
///   horizontal or vertical line through `from` meets it, so a partition
///   drawn to a face ends square.
///
/// Points and corners are left alone: an off-square point snapped on
/// purpose wins.
pub fn track(from: Point2, p: Snap, reach: f64) -> Snap {
    let d = p.point - from;
    if d.length() < 1e-9 {
        return p;
    }
    match p.kind {
        SnapKind::Free => {
            if let Some(dir) = ortho_dir(d) {
                let along = round_to(d.dot(dir), FREE_STEP);
                return Snap::at(from + dir * along, SnapKind::Tracked);
            }
            let on_line = constrain(from, p);
            if on_line.point.distance(p.point) <= reach {
                Snap::at(on_line.point, SnapKind::Tracked)
            } else {
                p
            }
        }
        SnapKind::OnEdge => {
            let (Some(dir), Some((a, b))) = (ortho_dir(d), p.edge) else {
                return p;
            };
            match line_meets_segment(from, dir, a, b) {
                Some(x) if x.distance(p.point) <= reach * 5.0 => Snap { point: x, ..p },
                _ => p,
            }
        }
        _ => p,
    }
}

/// Where the line through `p` along `dir` crosses the segment `a`–`b`.
fn line_meets_segment(p: Point2, dir: Point2, a: Point2, b: Point2) -> Option<Point2> {
    let e = b - a;
    let den = dir.x * e.y - dir.y * e.x;
    if den.abs() < 1e-9 {
        return None;
    }
    let w = a - p;
    // p + dir·t = a + e·s
    let s = -(dir.x * w.y - dir.y * w.x) / den;
    (-1e-9..=1.0 + 1e-9)
        .contains(&s)
        .then(|| a + e * s.clamp(0.0, 1.0))
}

/// Whether the line from `from` to `to` runs exactly horizontal, vertical
/// or at 45°: then the plan shows it as a blue tracking line.
pub fn is_tracking_line(from: Point2, to: Point2) -> bool {
    let d = to - from;
    if d.length() < 1e-9 {
        return false;
    }
    d.x.abs() < 1e-9 || d.y.abs() < 1e-9 || (d.x.abs() - d.y.abs()).abs() < 1e-9
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
        assert_eq!(s, Snap::at(Point2::new(0.0, 4.0), SnapKind::Endpoint));
        // Midpoint of the west wall's drawn line.
        let s = snap(&m, lvl, Point2::new(0.05, 2.02), 0.1, &[]);
        assert_eq!(s.kind, SnapKind::Midpoint);
        assert_eq!(s.point, Point2::new(0.0, 2.0));
        // On the interior face of the west wall (x = 0.265), away from points:
        // the point carries the face it is on.
        let s = snap(&m, lvl, Point2::new(0.3, 1.2), 0.1, &[]);
        assert_eq!(s.kind, SnapKind::OnEdge);
        assert!((s.point.x - 0.265).abs() < 1e-9 && (s.point.y - 1.2).abs() < 1e-9);
        let (a, b) = s.edge.expect("the face");
        assert!((a.x - 0.265).abs() < 1e-9 && (b.x - 0.265).abs() < 1e-9);
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
    fn midpoints_of_faces_and_slab_edges() {
        let (m, lvl, _) = room();
        // The interior face of the west wall runs between the inner corners
        // of the room: its middle is at mid-height of the room.
        let g = wall_geometry(&m, m.walls_on_level(lvl).next().unwrap().id).unwrap();
        let face: Vec<Point2> = g
            .outline
            .iter()
            .copied()
            .filter(|p| (p.x - 0.265).abs() < 1e-9)
            .collect();
        assert_eq!(face.len(), 2, "{:?}", g.outline);
        let mid = (face[0] + face[1]) * 0.5;
        let s = snap(&m, lvl, mid + Point2::new(0.04, 0.03), 0.1, &[]);
        assert_eq!(s, Snap::at(mid, SnapKind::Midpoint));
        // The middle of a slab edge.
        let slab = m.slabs_on_level(lvl).next().expect("the room has a slab");
        let (a, b) = (slab.outline[0], slab.outline[1]);
        let s = snap(&m, lvl, (a + b) * 0.5 + Point2::new(0.02, 0.02), 0.1, &[]);
        assert_eq!(s.kind, SnapKind::Midpoint);
        assert!(s.point.distance((a + b) * 0.5) < 1e-9);
    }

    #[test]
    fn snaps_switched_off_are_not_offered() {
        let (m, lvl, _) = room();
        let skip = BTreeSet::new();
        let at = |p: Point2, f: SnapFilter| snap_filtered(&m, lvl, p, 0.1, &[], &skip, f, &[]);
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
    fn walls_come_out_square_unless_the_cursor_leaves_the_axis() {
        let from = Point2::new(1.0, 1.0);
        let free = |x, y| Snap::at(Point2::new(x, y), SnapKind::Free);
        // 3 cm off the horizontal: locked.
        let s = track(from, free(5.0, 1.03), 0.05);
        assert_eq!(s, Snap::at(Point2::new(5.0, 1.0), SnapKind::Tracked));
        // 30 cm off over 4 m (4°): still square, by angle, not distance.
        assert_eq!(
            track(from, free(5.0, 1.3), 0.05),
            Snap::at(Point2::new(5.0, 1.0), SnapKind::Tracked)
        );
        // Vertical, downwards, also 4° off.
        assert_eq!(
            track(from, free(1.2, -2.0), 0.05).point,
            Point2::new(1.0, -2.0)
        );
        // 15° off: the cursor is followed (an inclined wall on purpose).
        assert_eq!(track(from, free(5.0, 2.07), 0.05), free(5.0, 2.07));
        // Close to 45°: locks only within the reach.
        let s = track(from, free(4.0, 4.03), 0.05);
        assert_eq!(s.kind, SnapKind::Tracked);
        assert!((s.point.x - s.point.y).abs() < 1e-9);
        // Object snaps win.
        let corner = Snap::at(Point2::new(5.0, 1.03), SnapKind::Corner);
        assert_eq!(track(from, corner, 0.05), corner);
        // On a face: the point slides along it to stay square.
        let on_face = Snap {
            point: Point2::new(5.0, 1.2),
            kind: SnapKind::OnEdge,
            edge: Some((Point2::new(4.0, 0.0), Point2::new(6.0, 3.0))),
        };
        let s = track(from, on_face, 0.1);
        assert_eq!(s.kind, SnapKind::OnEdge);
        assert!((s.point.y - 1.0).abs() < 1e-9, "{:?}", s.point);
        assert!(is_tracking_line(from, s.point));
        assert!(!is_tracking_line(from, Point2::new(5.0, 2.07)));
    }

    #[test]
    fn constrains_to_45_degrees() {
        let from = Point2::new(1.0, 1.0);
        let free = |x, y| Snap::at(Point2::new(x, y), SnapKind::Free);
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
            Snap::at(Point2::new(5.0, 1.234567), SnapKind::Endpoint),
        );
        assert_eq!(s.point, Point2::new(5.0, 1.0));
    }
}
