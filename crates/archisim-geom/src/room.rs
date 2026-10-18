//! Spaces enclosed by walls, for the slab tool's magic wand ("varita
//! mágica"): a click inside a room gives the outline of that room.
//!
//! The plan outlines of every wall on the level (with their mitred
//! corners) are cut into a planar arrangement: every edge is split where it
//! meets another one, so a partition that butts against the inner face of a
//! façade splits that face. The room is the face of the arrangement that
//! contains the clicked point, found by casting a ray from it and walking
//! the boundary of the face the ray started in.
//!
//! Three contours can be asked for:
//! - [`Contour::InteriorFace`]: the room itself, up to the faces of its
//!   walls (a floor finish, a ceiling).
//! - [`Contour::WallAxis`]: each side of the room pushed out to the axis of
//!   the wall it runs along (a structural slab between centre lines).
//! - [`Contour::ExteriorFace`]: the outer contour of the whole building the
//!   room belongs to, at the exterior faces of its walls (a ground slab or
//!   a roof under or over everything).

use archisim_core::{LevelId, Model, Point2, Vec2, WallId};

use crate::{wall_geometry, Line};

/// Which outline the magic wand returns.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Contour {
    /// The room up to the faces of its walls.
    #[default]
    InteriorFace,
    /// The room up to the axis (centre line) of its walls.
    WallAxis,
    /// The outer contour of the building, at the exterior faces.
    ExteriorFace,
}

/// Why no outline could be found.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RoomError {
    /// The point is inside a wall, not in a room.
    InsideWall,
    /// The walls around the point do not close a room.
    NotEnclosed,
}

impl std::fmt::Display for RoomError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RoomError::InsideWall => write!(f, "the point is inside a wall"),
            RoomError::NotEnclosed => write!(f, "the walls around the point do not close a room"),
        }
    }
}

impl std::error::Error for RoomError {}

/// Vertices closer than this are the same vertex (metres).
const MERGE: f64 = 1e-6;

/// Outline of the space around `p` on `level`, counter-clockwise, in the
/// requested contour.
pub fn room_outline(
    model: &Model,
    level: LevelId,
    p: Point2,
    contour: Contour,
) -> Result<Vec<Point2>, RoomError> {
    let walls: Vec<(WallId, Vec<Point2>)> = model
        .walls_on_level(level)
        .filter_map(|w| wall_geometry(model, w.id).map(|g| (w.id, g.outline)))
        .collect();
    if walls.iter().any(|(_, o)| strictly_inside(o, p)) {
        return Err(RoomError::InsideWall);
    }
    let arr = Arrangement::build(&walls);
    let crossings = arr.ray_crossings(p);

    // The first boundary met by the ray that goes around `p`
    // counter-clockwise is the outer boundary of the room.
    let mut room: Option<Vec<usize>> = None;
    for c in &crossings {
        let cycle = arr.trace(c.half_edge);
        let poly = arr.polygon(&cycle);
        if crate::signed_area(&poly) > 0.0 && contains(&poly, p) {
            room = Some(cycle);
            break;
        }
    }
    let room = room.ok_or(RoomError::NotEnclosed)?;

    let outline = match contour {
        Contour::InteriorFace => arr.polygon(&room),
        Contour::WallAxis => {
            let lines = arr.side_lines(&room);
            let shifted: Vec<(Line, f64)> = lines
                .iter()
                .map(|s| {
                    let shift = s
                        .walls
                        .iter()
                        .find_map(|w| shift_to_axis(model, *w, s.line))
                        .unwrap_or(0.0);
                    (s.line, shift)
                })
                .collect();
            offset_polygon(&shifted)
        }
        Contour::ExteriorFace => {
            // The farthest crossing with the building this room belongs to
            // lies on its outer contour; the face beyond it is outside.
            let component = arr.component(arr.half_edges[room[0]].from);
            let far = crossings
                .iter()
                .rev()
                .find(|c| component[arr.half_edges[c.half_edge].from])
                .ok_or(RoomError::NotEnclosed)?;
            let cycle = arr.trace(far.half_edge ^ 1);
            let mut poly = arr.polygon(&cycle);
            if crate::signed_area(&poly) >= 0.0 {
                return Err(RoomError::NotEnclosed);
            }
            poly.reverse();
            poly
        }
    };
    let outline = clean(outline);
    if outline.len() < 3 || crate::signed_area(&outline) <= 0.0 {
        return Err(RoomError::NotEnclosed);
    }
    Ok(outline)
}

/// Inside a polygon and not on its boundary.
fn strictly_inside(poly: &[Point2], p: Point2) -> bool {
    let n = poly.len();
    let on_edge = (0..n).any(|i| {
        let (a, b) = (poly[i], poly[(i + 1) % n]);
        let d = b - a;
        let t = ((p - a).dot(d) / d.dot(d)).clamp(0.0, 1.0);
        (a + d * t).distance(p) < MERGE
    });
    !on_edge && contains(poly, p)
}

/// Whether a closed polygon contains a point (ray casting).
fn contains(poly: &[Point2], p: Point2) -> bool {
    let mut inside = false;
    let n = poly.len();
    for i in 0..n {
        let (a, b) = (poly[i], poly[(i + 1) % n]);
        if (a.y > p.y) != (b.y > p.y) {
            let x = a.x + (p.y - a.y) / (b.y - a.y) * (b.x - a.x);
            if p.x < x {
                inside = !inside;
            }
        }
    }
    inside
}

/// How far the side line of a room has to move (along the line's left
/// normal) to reach the axis of `wall`, if the line runs along one of the
/// wall's faces.
fn shift_to_axis(model: &Model, wall: WallId, side: Line) -> Option<f64> {
    let w = model.wall(wall)?;
    let t = model.wall_type(w.wall_type)?.thickness();
    let (ext, int) = w.face_offsets(t);
    let dir = side.d.normalized()?;
    if let Some(arc) = w.arc() {
        // A piece of a curved face: moved radially to the axis.
        let mid = side.p + side.d * 0.5;
        let (f, offset) = arc.project(mid);
        let n = arc.tangent(f).perp_left();
        if dir.cross(arc.tangent(f)).abs() > 0.1 {
            return None;
        }
        let on_face = [ext, int].iter().any(|f| (offset - f).abs() < 1e-3);
        if !on_face {
            return None;
        }
        return Some(((ext + int) / 2.0 - offset) * n.dot(dir.perp_left()));
    }
    let u = (w.end - w.start).normalized()?;
    let n = u.perp_left();
    if dir.cross(u).abs() > 1e-6 {
        return None;
    }
    let offset = (side.p - w.start).dot(n);
    let on_face = [ext, int].iter().any(|f| (offset - f).abs() < 1e-5);
    if !on_face {
        return None;
    }
    let centre = (ext + int) / 2.0;
    // From the face to the centre, expressed along the side's left normal.
    Some((centre - offset) * n.dot(dir.perp_left()))
}

/// Rebuilds a polygon from its side lines, each moved along its left normal
/// by the given amount. Consecutive parallel sides are joined by a short
/// perpendicular step.
fn offset_polygon(sides: &[(Line, f64)]) -> Vec<Point2> {
    let moved: Vec<Line> = sides
        .iter()
        .map(|(l, s)| {
            let n = l.d.normalized().map_or(Vec2::default(), Vec2::perp_left);
            Line::new(l.p + n * *s, l.d)
        })
        .collect();
    let n = moved.len();
    let mut out = Vec::with_capacity(n);
    for i in 0..n {
        let prev = moved[(i + n - 1) % n];
        let next = moved[i];
        // The original corner, where the side `i` starts.
        let corner = sides[i].0.p;
        match prev.intersect(&next) {
            Some(q) if q.distance(corner) < 1e3 => out.push(q),
            _ => {
                out.push(project(prev, corner));
                out.push(project(next, corner));
            }
        }
    }
    out
}

fn project(l: Line, p: Point2) -> Point2 {
    let d = l.d.dot(l.d);
    if d < 1e-18 {
        return l.p;
    }
    l.p + l.d * ((p - l.p).dot(l.d) / d)
}

/// Rounds to the micrometre and removes repeated and collinear points (and
/// zero-width spikes, which are collinear too).
fn clean(poly: Vec<Point2>) -> Vec<Point2> {
    let round = |v: f64| (v * 1e6).round() / 1e6 + 0.0;
    let mut pts: Vec<Point2> = poly
        .into_iter()
        .map(|p| Point2::new(round(p.x), round(p.y)))
        .collect();
    loop {
        let n = pts.len();
        if n < 3 {
            return pts;
        }
        let mut removed = false;
        for i in 0..n {
            let (a, b, c) = (pts[(i + n - 1) % n], pts[i], pts[(i + 1) % n]);
            let repeated = a.distance(b) < MERGE;
            let collinear = (b - a).cross(c - b).abs() < 1e-9;
            if repeated || collinear {
                pts.remove(i);
                removed = true;
                break;
            }
        }
        if !removed {
            return pts;
        }
    }
}

/// One half of an edge of the arrangement, from `from` to `to`.
#[derive(Debug, Clone)]
struct HalfEdge {
    from: usize,
    to: usize,
    /// Walls whose outline contains this edge.
    edge: usize,
}

/// A crossing of the ray cast from the clicked point: the half-edge that
/// has the ray's start on its left, at distance `s`.
#[derive(Debug, Clone, Copy)]
struct Crossing {
    s: f64,
    half_edge: usize,
}

/// A side of a room: its line and the walls it runs along.
struct Side {
    line: Line,
    walls: Vec<WallId>,
}

struct Arrangement {
    vertices: Vec<Point2>,
    /// Half-edges in pairs: `2k` and `2k + 1` are the two halves of edge
    /// `k`, so the twin of `h` is `h ^ 1`.
    half_edges: Vec<HalfEdge>,
    /// Walls along each edge.
    edge_walls: Vec<Vec<WallId>>,
    /// Outgoing half-edges of each vertex, sorted by angle.
    outgoing: Vec<Vec<usize>>,
}

impl Arrangement {
    fn build(walls: &[(WallId, Vec<Point2>)]) -> Self {
        // Every outline edge, with the wall it belongs to.
        let mut segs: Vec<(Point2, Point2, WallId)> = Vec::new();
        for (id, o) in walls {
            let n = o.len();
            for i in 0..n {
                let (a, b) = (o[i], o[(i + 1) % n]);
                if a.distance(b) > MERGE {
                    segs.push((a, b, *id));
                }
            }
        }
        // Where each segment is cut by the others, as parameters 0..1.
        let mut cuts: Vec<Vec<f64>> = vec![vec![0.0, 1.0]; segs.len()];
        let boxes: Vec<(Point2, Point2)> = segs
            .iter()
            .map(|(a, b, _)| {
                (
                    Point2::new(a.x.min(b.x) - MERGE, a.y.min(b.y) - MERGE),
                    Point2::new(a.x.max(b.x) + MERGE, a.y.max(b.y) + MERGE),
                )
            })
            .collect();
        for i in 0..segs.len() {
            for j in (i + 1)..segs.len() {
                let (bi, bj) = (boxes[i], boxes[j]);
                if bi.1.x < bj.0.x || bj.1.x < bi.0.x || bi.1.y < bj.0.y || bj.1.y < bi.0.y {
                    continue;
                }
                let (a, b, _) = segs[i];
                let (c, d, _) = segs[j];
                for (ti, tj) in intersections(a, b, c, d) {
                    cuts[i].push(ti);
                    cuts[j].push(tj);
                }
            }
        }

        let mut arr = Arrangement {
            vertices: Vec::new(),
            half_edges: Vec::new(),
            edge_walls: Vec::new(),
            outgoing: Vec::new(),
        };
        let mut index: std::collections::HashMap<(i64, i64), Vec<usize>> = Default::default();
        let mut edge_of: std::collections::HashMap<(usize, usize), usize> = Default::default();
        for (k, (a, b, wall)) in segs.iter().enumerate() {
            let mut ts = cuts[k].clone();
            ts.sort_by(f64::total_cmp);
            let pts: Vec<usize> = ts
                .iter()
                .map(|t| arr.vertex(&mut index, *a + (*b - *a) * *t))
                .collect();
            for w in pts.windows(2) {
                let (u, v) = (w[0], w[1]);
                if u == v {
                    continue;
                }
                let key = (u.min(v), u.max(v));
                let e = *edge_of.entry(key).or_insert_with(|| {
                    let e = arr.edge_walls.len();
                    arr.edge_walls.push(Vec::new());
                    arr.half_edges.push(HalfEdge {
                        from: u,
                        to: v,
                        edge: e,
                    });
                    arr.half_edges.push(HalfEdge {
                        from: v,
                        to: u,
                        edge: e,
                    });
                    e
                });
                if !arr.edge_walls[e].contains(wall) {
                    arr.edge_walls[e].push(*wall);
                }
            }
        }
        arr.outgoing = vec![Vec::new(); arr.vertices.len()];
        for (h, he) in arr.half_edges.iter().enumerate() {
            arr.outgoing[he.from].push(h);
        }
        let angle = |arr: &Arrangement, h: usize| {
            let he = &arr.half_edges[h];
            let d = arr.vertices[he.to] - arr.vertices[he.from];
            d.y.atan2(d.x)
        };
        for v in 0..arr.outgoing.len() {
            let mut out = std::mem::take(&mut arr.outgoing[v]);
            out.sort_by(|a, b| angle(&arr, *a).total_cmp(&angle(&arr, *b)));
            arr.outgoing[v] = out;
        }
        arr
    }

    /// The vertex at `p`, merging points closer than [`MERGE`].
    fn vertex(
        &mut self,
        index: &mut std::collections::HashMap<(i64, i64), Vec<usize>>,
        p: Point2,
    ) -> usize {
        let key = |v: f64| (v / (MERGE * 10.0)).floor() as i64;
        let (kx, ky) = (key(p.x), key(p.y));
        for dx in -1..=1 {
            for dy in -1..=1 {
                if let Some(list) = index.get(&(kx + dx, ky + dy)) {
                    if let Some(&v) = list.iter().find(|&&v| self.vertices[v].distance(p) < MERGE) {
                        return v;
                    }
                }
            }
        }
        let v = self.vertices.len();
        self.vertices.push(p);
        index.entry((kx, ky)).or_default().push(v);
        v
    }

    /// The half-edge that follows `h` around the face on its left: at the
    /// end vertex, the outgoing edge just clockwise from the way back.
    fn next(&self, h: usize) -> usize {
        let he = &self.half_edges[h];
        let out = &self.outgoing[he.to];
        let back = h ^ 1;
        let i = out.iter().position(|&x| x == back).unwrap_or(0);
        out[(i + out.len() - 1) % out.len()]
    }

    /// The cycle of half-edges around the face on the left of `start`.
    fn trace(&self, start: usize) -> Vec<usize> {
        let mut cycle = vec![start];
        let mut h = self.next(start);
        while h != start && cycle.len() <= self.half_edges.len() {
            cycle.push(h);
            h = self.next(h);
        }
        cycle
    }

    fn polygon(&self, cycle: &[usize]) -> Vec<Point2> {
        cycle
            .iter()
            .map(|h| self.vertices[self.half_edges[*h].from])
            .collect()
    }

    /// The sides of a cycle, merging consecutive collinear edges that run
    /// along the same walls.
    fn side_lines(&self, cycle: &[usize]) -> Vec<Side> {
        let mut sides: Vec<Side> = Vec::new();
        let mut ends: Vec<Point2> = Vec::new();
        for &h in cycle {
            let he = &self.half_edges[h];
            let (a, b) = (self.vertices[he.from], self.vertices[he.to]);
            let walls = self.edge_walls[he.edge].clone();
            if let (Some(last), Some(end)) = (sides.last_mut(), ends.last_mut()) {
                let same_dir = (*end - last.line.p).cross(b - a).abs() < 1e-9
                    && (*end - last.line.p).dot(b - a) > 0.0;
                if same_dir && last.walls == walls {
                    *end = b;
                    last.line.d = b - last.line.p;
                    continue;
                }
            }
            sides.push(Side {
                line: Line::new(a, b - a),
                walls,
            });
            ends.push(b);
        }
        // The last side may continue the first one.
        if sides.len() > 1 {
            let (first, last) = (&sides[0], &sides[sides.len() - 1]);
            let continues = first.walls == last.walls
                && first.line.d.cross(last.line.d).abs() < 1e-9
                && first.line.d.dot(last.line.d) > 0.0;
            if continues {
                let end = ends[0];
                let start = last.line.p;
                sides.pop();
                sides[0].line = Line::new(start, end - start);
            }
        }
        sides
    }

    /// Vertices connected to `v`.
    fn component(&self, v: usize) -> Vec<bool> {
        let mut seen = vec![false; self.vertices.len()];
        let mut stack = vec![v];
        seen[v] = true;
        while let Some(x) = stack.pop() {
            for &h in &self.outgoing[x] {
                let to = self.half_edges[h].to;
                if !seen[to] {
                    seen[to] = true;
                    stack.push(to);
                }
            }
        }
        seen
    }

    /// Edges crossed by a ray from `p`, nearest first. The ray leaves at an
    /// odd angle so it does not run along walls or through their corners.
    fn ray_crossings(&self, p: Point2) -> Vec<Crossing> {
        let dir = Point2::new(0.913_545_457_642_600_9, 0.406_736_643_075_800_2);
        let mut out = Vec::new();
        for k in 0..self.edge_walls.len() {
            let h = 2 * k;
            let (a, b) = (
                self.vertices[self.half_edges[h].from],
                self.vertices[self.half_edges[h].to],
            );
            let e = b - a;
            let den = dir.cross(e);
            if den.abs() < 1e-15 {
                continue;
            }
            let s = (a - p).cross(e) / den;
            let t = (a - p).cross(dir) / den;
            if s > 0.0 && (0.0..=1.0).contains(&t) {
                // The half-edge that has `p` on its left.
                let half_edge = if e.cross(p - a) > 0.0 { h } else { h + 1 };
                out.push(Crossing { s, half_edge });
            }
        }
        out.sort_by(|a, b| a.s.total_cmp(&b.s));
        out
    }
}

/// Parameters on `a`–`b` and on `c`–`d` where the two segments meet:
/// a crossing or touch, or the ends of their overlap when collinear.
fn intersections(a: Point2, b: Point2, c: Point2, d: Point2) -> Vec<(f64, f64)> {
    let r = b - a;
    let s = d - c;
    let den = r.cross(s);
    let param = |p: Point2, o: Point2, v: Vec2| (p - o).dot(v) / v.dot(v);
    let near = |p: Point2, o: Point2, v: Vec2| {
        let t = param(p, o, v).clamp(0.0, 1.0);
        (o + v * t).distance(p) < MERGE
    };
    let mut out = Vec::new();
    let tol_r = MERGE / r.length();
    let tol_s = MERGE / s.length();
    if den.abs() > 1e-12 * r.length() * s.length() {
        let t = (c - a).cross(s) / den;
        let u = (c - a).cross(r) / den;
        if t >= -tol_r && t <= 1.0 + tol_r && u >= -tol_s && u <= 1.0 + tol_s {
            out.push((t.clamp(0.0, 1.0), u.clamp(0.0, 1.0)));
        }
    } else {
        // Parallel: only overlapping collinear segments meet.
        for (p, on_ab) in [(c, true), (d, true), (a, false), (b, false)] {
            let touches = if on_ab { near(p, a, r) } else { near(p, c, s) };
            if touches {
                out.push((
                    param(p, a, r).clamp(0.0, 1.0),
                    param(p, c, s).clamp(0.0, 1.0),
                ));
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use archisim_core::{CutPattern, Layer, LayerFunction, NewWall, ReferenceLine, WallFunction};

    /// A 12 × 9 m house drawn clockwise along its exterior faces with
    /// 0.3 m façades, and a 0.1 m partition on its axis at x = 5 that butts
    /// against the inner faces of the north and south walls.
    fn house() -> (Model, LevelId) {
        let mut m = Model::new("Casa");
        let lvl = m.add_level("PB", 0.0);
        let mat = m.add_material("Ladrillo", "brick", [200, 100, 80], CutPattern::Solid);
        let layer = |t| Layer {
            material: mat,
            thickness: t,
            function: LayerFunction::Structure,
        };
        let facade = m
            .add_wall_type("Fachada", WallFunction::Exterior, vec![layer(0.3)])
            .unwrap();
        let partition = m
            .add_wall_type("Tabique", WallFunction::Interior, vec![layer(0.1)])
            .unwrap();
        let c = [
            Point2::new(0.0, 0.0),
            Point2::new(0.0, 9.0),
            Point2::new(12.0, 9.0),
            Point2::new(12.0, 0.0),
        ];
        for i in 0..4 {
            m.add_wall(NewWall {
                wall_type: facade,
                level: lvl,
                start: c[i],
                end: c[(i + 1) % 4],
                reference: ReferenceLine::ExteriorFace,
                height: 3.0,
                base_offset: 0.0,
                bulge: 0.0,
                top: archisim_core::WallTop::Unconnected,
            })
            .unwrap();
        }
        m.add_wall(NewWall {
            wall_type: partition,
            level: lvl,
            start: Point2::new(5.0, 0.3),
            end: Point2::new(5.0, 8.7),
            reference: ReferenceLine::Center,
            height: 3.0,
            base_offset: 0.0,
            bulge: 0.0,
            top: archisim_core::WallTop::Unconnected,
        })
        .unwrap();
        (m, lvl)
    }

    fn same_polygon(got: &[Point2], want: &[(f64, f64)]) {
        assert_eq!(got.len(), want.len(), "{got:?}");
        let start = got
            .iter()
            .position(|p| p.distance(Point2::new(want[0].0, want[0].1)) < 1e-9)
            .unwrap_or_else(|| panic!("{got:?}"));
        for (i, w) in want.iter().enumerate() {
            let g = got[(start + i) % got.len()];
            assert!(
                g.distance(Point2::new(w.0, w.1)) < 1e-9,
                "{got:?} should be {want:?}"
            );
        }
    }

    #[test]
    fn rooms_on_either_side_of_a_partition() {
        let (m, lvl) = house();
        let west = room_outline(&m, lvl, Point2::new(2.0, 4.0), Contour::InteriorFace).unwrap();
        same_polygon(&west, &[(0.3, 0.3), (4.95, 0.3), (4.95, 8.7), (0.3, 8.7)]);
        let east = room_outline(&m, lvl, Point2::new(9.0, 4.0), Contour::InteriorFace).unwrap();
        same_polygon(&east, &[(5.05, 0.3), (11.7, 0.3), (11.7, 8.7), (5.05, 8.7)]);
        // To the wall axes: the façades are 0.3 thick, the partition 0.1.
        let axis = room_outline(&m, lvl, Point2::new(2.0, 4.0), Contour::WallAxis).unwrap();
        same_polygon(
            &axis,
            &[(0.15, 0.15), (5.0, 0.15), (5.0, 8.85), (0.15, 8.85)],
        );
        // The whole building, whichever room is clicked.
        for p in [Point2::new(2.0, 4.0), Point2::new(9.0, 4.0)] {
            let ext = room_outline(&m, lvl, p, Contour::ExteriorFace).unwrap();
            same_polygon(&ext, &[(0.0, 0.0), (12.0, 0.0), (12.0, 9.0), (0.0, 9.0)]);
            assert!(crate::signed_area(&ext) > 0.0);
        }
    }

    #[test]
    fn refuses_points_in_walls_or_outside() {
        let (m, lvl) = house();
        assert_eq!(
            room_outline(&m, lvl, Point2::new(0.1, 4.0), Contour::InteriorFace),
            Err(RoomError::InsideWall)
        );
        assert_eq!(
            room_outline(&m, lvl, Point2::new(-3.0, 4.0), Contour::InteriorFace),
            Err(RoomError::NotEnclosed)
        );
        assert_eq!(
            room_outline(&m, lvl, Point2::new(20.0, 4.0), Contour::ExteriorFace),
            Err(RoomError::NotEnclosed)
        );
    }

    #[test]
    fn an_open_outline_is_not_a_room() {
        let (mut m, lvl) = house();
        // Remove the east façade: the house is open.
        let east = m.walls().nth(2).unwrap().id;
        m.remove_wall(east).unwrap();
        assert_eq!(
            room_outline(&m, lvl, Point2::new(9.0, 4.0), Contour::InteriorFace),
            Err(RoomError::NotEnclosed)
        );
        // The west room is still closed by the partition.
        assert!(room_outline(&m, lvl, Point2::new(2.0, 4.0), Contour::InteriorFace).is_ok());
    }

    #[test]
    fn a_free_standing_wall_inside_a_room_is_skipped() {
        let (mut m, lvl) = house();
        let partition = m.walls().last().unwrap().wall_type;
        // A short free-standing wall in the east room, right of the click.
        m.add_wall(NewWall {
            wall_type: partition,
            level: lvl,
            start: Point2::new(9.0, 3.0),
            end: Point2::new(9.0, 6.0),
            reference: ReferenceLine::Center,
            height: 1.0,
            base_offset: 0.0,
            bulge: 0.0,
            top: archisim_core::WallTop::Unconnected,
        })
        .unwrap();
        let east = room_outline(&m, lvl, Point2::new(7.0, 4.0), Contour::InteriorFace).unwrap();
        same_polygon(&east, &[(5.05, 0.3), (11.7, 0.3), (11.7, 8.7), (5.05, 8.7)]);
    }

    #[test]
    fn an_l_shaped_room_with_a_t_junction() {
        let (mut m, lvl) = house();
        let partition = m.walls().last().unwrap().wall_type;
        // A partition from the inner face of the east façade to the first
        // partition, closing a room in the north-east corner.
        m.add_wall(NewWall {
            wall_type: partition,
            level: lvl,
            start: Point2::new(5.05, 5.0),
            end: Point2::new(11.7, 5.0),
            reference: ReferenceLine::Center,
            height: 3.0,
            base_offset: 0.0,
            bulge: 0.0,
            top: archisim_core::WallTop::Unconnected,
        })
        .unwrap();
        let ne = room_outline(&m, lvl, Point2::new(8.0, 7.0), Contour::InteriorFace).unwrap();
        same_polygon(&ne, &[(5.05, 5.05), (11.7, 5.05), (11.7, 8.7), (5.05, 8.7)]);
        let ne_axis = room_outline(&m, lvl, Point2::new(8.0, 7.0), Contour::WallAxis).unwrap();
        same_polygon(
            &ne_axis,
            &[(5.0, 5.0), (11.85, 5.0), (11.85, 8.85), (5.0, 8.85)],
        );
    }
}
