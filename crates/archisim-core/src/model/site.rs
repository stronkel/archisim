//! The site: the terrain, from survey points, and the true north.
//!
//! A terrain is a set of survey points (plan position and elevation from
//! project zero), joined in triangles (Delaunay) into a surface; the
//! ground under it is a solid down to `depth` below its lowest point, for
//! sections. Its contour lines ("curvas de nivel") are drawn in plan
//! every `contour` metres.
//!
//! True north is the angle from the plan's up (+Y) to the geographic
//! north, clockwise (positive: north lies to the right of up).
//!
//! IFC: `IfcGeographicElement` (`TERRAIN`) with a triangulated surface;
//! true north in the model's geometric context.

use super::*;

/// A survey point: where and how high (from project zero).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SurveyPoint {
    pub at: Point2,
    pub z: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Terrain {
    pub id: TerrainId,
    pub guid: GlobalId,
    /// The level it is drawn on (usually the ground floor).
    pub level: LevelId,
    pub points: Vec<SurveyPoint>,
    pub material: MaterialId,
    /// How far the ground goes below its lowest point.
    pub depth: f64,
    /// Interval between contour lines.
    pub contour: f64,
    /// Contour lines drawn by hand ("líneas de nivel"): runs of points,
    /// by index, each two points at least. They pick their points
    /// together, to give them a height at once.
    pub lines: Vec<Vec<usize>>,
}

/// Data needed to create a terrain.
#[derive(Debug, Clone, PartialEq)]
pub struct NewTerrain {
    pub level: LevelId,
    pub points: Vec<SurveyPoint>,
    pub material: MaterialId,
    pub depth: f64,
    pub contour: f64,
    pub lines: Vec<Vec<usize>>,
}

/// Largest gap between the points of a contour line drawn by hand.
pub const CONTOUR_POINT_SPACING: f64 = 1.0;

impl Terrain {
    /// Adds a contour line through `path` at height `z`: points no more
    /// than `CONTOUR_POINT_SPACING` apart along it (reusing points already there).
    /// Returns the index of the new line.
    pub fn add_line(&mut self, path: &[Point2], z: f64) -> usize {
        let line = push_line(&mut self.points, path, z);
        self.lines.push(line);
        self.lines.len() - 1
    }

    /// Removes the points with these indices, and them from the lines (a
    /// line left with fewer than two points goes).
    pub fn remove_points(&mut self, gone: &[usize]) {
        let mut map = Vec::with_capacity(self.points.len());
        let mut kept = Vec::with_capacity(self.points.len());
        for (i, p) in self.points.iter().enumerate() {
            if gone.contains(&i) {
                map.push(None);
            } else {
                map.push(Some(kept.len()));
                kept.push(*p);
            }
        }
        self.points = kept;
        self.lines = std::mem::take(&mut self.lines)
            .into_iter()
            .map(|l| l.into_iter().filter_map(|i| map[i]).collect::<Vec<_>>())
            .filter(|l| l.len() >= 2)
            .collect();
    }

    /// The surface's triangles, counter-clockwise in plan, by point index.
    pub fn triangles(&self) -> Vec<[usize; 3]> {
        delaunay(&self.points.iter().map(|p| p.at).collect::<Vec<_>>())
    }

    /// Height of the surface at `p`, if it lies over it.
    pub fn height_at(&self, p: Point2) -> Option<f64> {
        for [a, b, c] in self.triangles() {
            let (pa, pb, pc) = (self.points[a], self.points[b], self.points[c]);
            if let Some((u, v, w)) = barycentric(p, pa.at, pb.at, pc.at) {
                return Some(u * pa.z + v * pb.z + w * pc.z);
            }
        }
        None
    }

    /// The outline of the surface in plan (its convex hull),
    /// counter-clockwise, by point index.
    pub fn boundary(&self) -> Vec<usize> {
        // Edges of one triangle only.
        let tris = self.triangles();
        let mut count: std::collections::HashMap<(usize, usize), usize> = Default::default();
        for t in &tris {
            for i in 0..3 {
                let (a, b) = (t[i], t[(i + 1) % 3]);
                *count.entry((a.min(b), a.max(b))).or_default() += 1;
            }
        }
        let mut next: std::collections::HashMap<usize, usize> = Default::default();
        for t in &tris {
            for i in 0..3 {
                let (a, b) = (t[i], t[(i + 1) % 3]);
                if count[&(a.min(b), a.max(b))] == 1 {
                    next.insert(a, b);
                }
            }
        }
        let Some(&start) = next.keys().min() else {
            return Vec::new();
        };
        let mut out = vec![start];
        let mut cur = start;
        while let Some(&n) = next.get(&cur) {
            if n == start || out.len() > next.len() {
                break;
            }
            out.push(n);
            cur = n;
        }
        out
    }

    /// Lowest and highest point.
    pub fn range(&self) -> (f64, f64) {
        self.points
            .iter()
            .fold((f64::INFINITY, f64::NEG_INFINITY), |(lo, hi), p| {
                (lo.min(p.z), hi.max(p.z))
            })
    }

    /// The contour lines: for each height (a multiple of `contour`), the
    /// pieces of the line in plan.
    pub fn contours(&self) -> Vec<(f64, Vec<(Point2, Point2)>)> {
        let step = self.contour;
        if step <= 0.0 {
            return Vec::new();
        }
        let (lo, hi) = self.range();
        let tris = self.triangles();
        let first = (lo / step).ceil() as i64;
        let last = (hi / step).floor() as i64;
        let mut out = Vec::new();
        for k in first..=last {
            let z = k as f64 * step;
            let mut pieces = Vec::new();
            for [a, b, c] in &tris {
                let pts = [self.points[*a], self.points[*b], self.points[*c]];
                let mut cross: Vec<Point2> = Vec::new();
                for i in 0..3 {
                    let (p, q) = (pts[i], pts[(i + 1) % 3]);
                    let (dp, dq) = (p.z - z, q.z - z);
                    if (dp < 0.0 && dq >= 0.0) || (dp >= 0.0 && dq < 0.0) {
                        let t = dp / (dp - dq);
                        cross.push(p.at + (q.at - p.at) * t);
                    }
                }
                if cross.len() == 2 && cross[0].distance(cross[1]) > 1e-9 {
                    pieces.push((cross[0], cross[1]));
                }
            }
            if !pieces.is_empty() {
                out.push((z, pieces));
            }
        }
        out
    }
}

/// Puts the points of a contour line through `path` at height `z` into
/// `points` (no more than `CONTOUR_POINT_SPACING` apart; a point already there is
/// reused and takes the height), and returns their indices in order.
pub fn push_line(points: &mut Vec<SurveyPoint>, path: &[Point2], z: f64) -> Vec<usize> {
    let mut at: Vec<Point2> = Vec::new();
    for (k, w) in path.windows(2).enumerate() {
        let n = (w[0].distance(w[1]) / CONTOUR_POINT_SPACING - 1e-9)
            .ceil()
            .max(1.0) as usize;
        let first = if k == 0 { 0 } else { 1 };
        for j in first..=n {
            at.push(w[0] + (w[1] - w[0]) * (j as f64 / n as f64));
        }
    }
    if path.len() == 1 {
        at.push(path[0]);
    }
    let mut line: Vec<usize> = Vec::new();
    for p in at {
        let i = match points.iter().position(|q| q.at.approx_eq(p)) {
            Some(i) => {
                points[i].z = z;
                i
            }
            None => {
                points.push(SurveyPoint { at: p, z });
                points.len() - 1
            }
        };
        if line.last() != Some(&i) {
            line.push(i);
        }
    }
    line
}

/// Barycentric coordinates of `p` in the triangle `a b c`, if inside.
fn barycentric(p: Point2, a: Point2, b: Point2, c: Point2) -> Option<(f64, f64, f64)> {
    let d = (b - a).cross(c - a);
    if d.abs() < 1e-12 {
        return None;
    }
    let u = (b - p).cross(c - p) / d;
    let v = (c - p).cross(a - p) / d;
    let w = 1.0 - u - v;
    let e = -1e-9;
    (u >= e && v >= e && w >= e).then_some((u, v, w))
}

/// Delaunay triangulation of points in plan; triangles counter-clockwise,
/// by index. Points repeated are left out, and so are triangles of no area.
///
/// A sweep: points are added in order of distance from a first small
/// triangle, each joined to the edges of the hull it sees (a point on a
/// hull edge splits it), and then edges are flipped until every triangle's
/// circle is empty (Lawson). Close to `n log n` for survey data, where
/// adding points one at a time into one big triangle was `n²`.
pub fn delaunay(points: &[Point2]) -> Vec<[usize; 3]> {
    let n = points.len();
    if n < 3 {
        return Vec::new();
    }
    // Worked about the middle of the points, so that survey coordinates
    // (hundreds of kilometres) keep their precision.
    let (mut lo, mut hi) = (points[0], points[0]);
    for p in points {
        lo = Point2::new(lo.x.min(p.x), lo.y.min(p.y));
        hi = Point2::new(hi.x.max(p.x), hi.y.max(p.y));
    }
    let mid = (lo + hi) * 0.5;
    let pts: Vec<Point2> = points
        .iter()
        .map(|p| Point2::new(p.x - mid.x, p.y - mid.y))
        .collect();
    let size = (hi.x - lo.x).max(hi.y - lo.y).max(1e-9);
    // Lengths below this are the same point.
    let eps = size * 1e-10;
    let cross = |a: usize, b: usize, c: usize| (pts[b] - pts[a]).cross(pts[c] - pts[a]);
    let dist2 = |a: Point2, b: Point2| (a.x - b.x).powi(2) + (a.y - b.y).powi(2);

    // The first triangle: the point nearest the middle, its nearest
    // neighbour, and the point making the smallest circle with them.
    let centre = Point2::new(0.0, 0.0);
    let i0 = (0..n)
        .min_by(|&a, &b| dist2(pts[a], centre).total_cmp(&dist2(pts[b], centre)))
        .expect("points");
    let Some(i1) = (0..n)
        .filter(|&i| dist2(pts[i], pts[i0]) > eps * eps)
        .min_by(|&a, &b| dist2(pts[a], pts[i0]).total_cmp(&dist2(pts[b], pts[i0])))
    else {
        return Vec::new();
    };
    let radius2 = |a: usize, b: usize, c: usize| -> f64 {
        circumcentre(pts[a], pts[b], pts[c]).map_or(f64::INFINITY, |o| dist2(o, pts[a]))
    };
    let Some(mut i2) = (0..n)
        .filter(|&i| i != i0 && i != i1 && cross(i0, i1, i).abs() > eps * size)
        .min_by(|&a, &b| radius2(i0, i1, a).total_cmp(&radius2(i0, i1, b)))
    else {
        // All in a line: no surface.
        return Vec::new();
    };
    let mut i1 = i1;
    if cross(i0, i1, i2) < 0.0 {
        std::mem::swap(&mut i1, &mut i2);
    }
    let centre = circumcentre(pts[i0], pts[i1], pts[i2]).unwrap_or(centre);
    let mut order: Vec<usize> = (0..n).collect();
    order.sort_by(|&a, &b| dist2(pts[a], centre).total_cmp(&dist2(pts[b], centre)));

    let mut mesh = Mesh::default();
    // The hull, counter-clockwise: next and previous vertex, and the
    // half-edge running along the hull from each vertex to the next.
    const NONE: usize = usize::MAX;
    let mut next = vec![NONE; n];
    let mut prev = vec![NONE; n];
    let mut hull_edge = vec![NONE; n];
    let t = mesh.add(i0, i1, i2, NONE, NONE, NONE);
    next[i0] = i1;
    next[i1] = i2;
    next[i2] = i0;
    prev[i1] = i0;
    prev[i2] = i1;
    prev[i0] = i2;
    hull_edge[i0] = t;
    hull_edge[i1] = t + 1;
    hull_edge[i2] = t + 2;
    let mut start = i0;
    let mut inside: Vec<usize> = Vec::new();
    let mut last: Option<usize> = None;
    for &p in &order {
        if p == i0 || p == i1 || p == i2 {
            continue;
        }
        if last.is_some_and(|q| dist2(pts[p], pts[q]) <= eps * eps) {
            continue;
        }
        last = Some(p);
        let visible = |a: usize, b: usize| cross(a, b, p) < -eps * size;
        // A hull edge it sees: walk round the hull (from where the
        // previous point was added: points come in order round it).
        let mut e = start;
        let mut found = false;
        loop {
            if visible(e, next[e]) {
                found = true;
                break;
            }
            e = next[e];
            if e == start {
                break;
            }
        }
        if !found {
            inside.push(p);
            continue;
        }
        // Go back to the first edge it sees (those it sees are together:
        // the hull is convex).
        let first = e;
        while visible(prev[e], e) {
            e = prev[e];
            if e == first {
                break;
            }
        }
        let q = next[e];
        let t = mesh.add(e, p, q, NONE, NONE, hull_edge[e]);
        hull_edge[e] = t;
        hull_edge[p] = t + 1;
        // Forwards: the next edges it sees.
        let mut m = q;
        while visible(m, next[m]) {
            let r = next[m];
            let t = mesh.add(m, p, r, hull_edge[p], NONE, hull_edge[m]);
            hull_edge[p] = t + 1;
            next[m] = NONE;
            prev[m] = NONE;
            m = r;
        }
        next[e] = p;
        prev[p] = e;
        next[p] = m;
        prev[m] = p;
        start = p;
    }
    // Points not outside the hull when their turn came: on a hull edge,
    // or inside (near the first triangle).
    for p in inside {
        mesh.insert_inside(p, &pts, eps, &mut next, &mut prev, &mut hull_edge);
    }
    mesh.flip_all(&pts);
    mesh.tris
        .chunks(3)
        .filter(|t| cross(t[0], t[1], t[2]) > eps * eps)
        .map(|t| [t[0], t[1], t[2]])
        .collect()
}

/// Survey points and contour lines in bulk (an imported survey), after the
/// points a terrain has (kept as they are, so its lines stay right): the
/// new points, each once (one at the same place as another is left out), and
/// each contour line through `paths` at its height, its points no more than
/// `CONTOUR_POINT_SPACING` apart and shared with the points already there (they take
/// the line's height). Fast for thousands of points.
pub fn survey_points(
    existing: Vec<SurveyPoint>,
    points: impl IntoIterator<Item = SurveyPoint>,
    paths: &[(Vec<Point2>, f64)],
) -> (Vec<SurveyPoint>, Vec<Vec<usize>>) {
    use std::collections::HashMap;
    let key = |p: Point2| ((p.x * 1e4).round() as i64, (p.y * 1e4).round() as i64);
    let mut index: HashMap<(i64, i64), usize> = HashMap::new();
    for (i, p) in existing.iter().enumerate() {
        index.entry(key(p.at)).or_insert(i);
    }
    let mut out = existing;
    for p in points {
        index.entry(key(p.at)).or_insert_with(|| {
            out.push(p);
            out.len() - 1
        });
    }
    let mut lines = Vec::new();
    for (path, z) in paths {
        let mut line: Vec<usize> = Vec::new();
        let mut add = |p: Point2, out: &mut Vec<SurveyPoint>| {
            let i = *index.entry(key(p)).or_insert_with(|| {
                out.push(SurveyPoint { at: p, z: *z });
                out.len() - 1
            });
            out[i].z = *z;
            if line.last() != Some(&i) {
                line.push(i);
            }
        };
        if let Some(first) = path.first() {
            add(*first, &mut out);
        }
        for w in path.windows(2) {
            let n = (w[0].distance(w[1]) / CONTOUR_POINT_SPACING - 1e-9)
                .ceil()
                .max(1.0) as usize;
            for j in 1..=n {
                add(w[0] + (w[1] - w[0]) * (j as f64 / n as f64), &mut out);
            }
        }
        if line.len() >= 2 {
            lines.push(line);
        }
    }
    (out, lines)
}

/// Centre of the circle through three points.
fn circumcentre(a: Point2, b: Point2, c: Point2) -> Option<Point2> {
    let (bx, by) = (b.x - a.x, b.y - a.y);
    let (cx, cy) = (c.x - a.x, c.y - a.y);
    let d = 2.0 * (bx * cy - by * cx);
    if d.abs() < 1e-300 {
        return None;
    }
    let b2 = bx * bx + by * by;
    let c2 = cx * cx + cy * cy;
    Some(Point2::new(
        a.x + (cy * b2 - by * c2) / d,
        a.y + (bx * c2 - cx * b2) / d,
    ))
}

/// Whether `d` lies inside the circle through the counter-clockwise
/// triangle `a b c`.
fn in_circle(a: Point2, b: Point2, c: Point2, d: Point2) -> bool {
    let (ax, ay) = (a.x - d.x, a.y - d.y);
    let (bx, by) = (b.x - d.x, b.y - d.y);
    let (cx, cy) = (c.x - d.x, c.y - d.y);
    let det = (ax * ax + ay * ay) * (bx * cy - cx * by) - (bx * bx + by * by) * (ax * cy - cx * ay)
        + (cx * cx + cy * cy) * (ax * by - bx * ay);
    let scale = (ax * ax + ay * ay) + (bx * bx + by * by) + (cx * cx + cy * cy);
    det > 1e-12 * scale * scale
}

/// Triangles as half-edges: half-edge `h` runs from `tris[h]` to the next
/// corner of its triangle; `twin[h]` is the half-edge the other way, if
/// another triangle is there.
#[derive(Default)]
struct Mesh {
    tris: Vec<usize>,
    twin: Vec<usize>,
}

impl Mesh {
    const NONE: usize = usize::MAX;

    fn link(&mut self, a: usize, b: usize) {
        if a != Self::NONE {
            self.twin[a] = b;
        }
        if b != Self::NONE {
            self.twin[b] = a;
        }
    }

    /// A triangle `a b c` whose edges face half-edges `ta`, `tb`, `tc`;
    /// returns its first half-edge.
    fn add(&mut self, a: usize, b: usize, c: usize, ta: usize, tb: usize, tc: usize) -> usize {
        let t = self.tris.len();
        self.tris.extend([a, b, c]);
        self.twin.extend([Self::NONE; 3]);
        self.link(t, ta);
        self.link(t + 1, tb);
        self.link(t + 2, tc);
        t
    }

    /// Adds a point that is not outside the hull: on a hull edge (the
    /// triangle there splits in two), on an edge inside (both triangles
    /// split), or inside a triangle (it splits in three).
    fn insert_inside(
        &mut self,
        p: usize,
        pts: &[Point2],
        eps: f64,
        next: &mut [usize],
        prev: &mut [usize],
        hull_edge: &mut [usize],
    ) {
        let n = self.tris.len() / 3;
        let at = pts[p];
        for t in 0..n {
            let h = 3 * t;
            let (a, b, c) = (self.tris[h], self.tris[h + 1], self.tris[h + 2]);
            let side = |u: usize, v: usize| (pts[v] - pts[u]).cross(at - pts[u]);
            let len = |u: usize, v: usize| pts[u].distance(pts[v]).max(1e-300);
            // Distance to each edge's line (inside: positive).
            let d = [
                side(a, b) / len(a, b),
                side(b, c) / len(b, c),
                side(c, a) / len(c, a),
            ];
            if d.iter().any(|x| *x < -eps) {
                continue;
            }
            if [a, b, c].iter().any(|v| pts[*v].distance(at) <= eps) {
                return; // the same point
            }
            match d.iter().position(|x| *x <= eps) {
                None => self.split3(t, p, hull_edge),
                Some(k) => {
                    let e = h + k;
                    if self.twin[e] == Self::NONE {
                        self.split_hull(e, p, next, prev, hull_edge);
                    } else {
                        self.split_edge(e, p, hull_edge);
                    }
                }
            }
            return;
        }
    }

    /// Triangle `t` split in three round `p` inside it.
    fn split3(&mut self, t: usize, p: usize, hull_edge: &mut [usize]) {
        let h = 3 * t;
        let (a, b, c) = (self.tris[h], self.tris[h + 1], self.tris[h + 2]);
        let (x1, x2) = (self.twin[h + 1], self.twin[h + 2]);
        // t becomes (a, b, p); then (b, c, p) and (c, a, p).
        self.tris[h + 2] = p;
        let n1 = self.add(b, c, p, x1, Self::NONE, h + 1);
        let n2 = self.add(c, a, p, x2, h + 2, n1 + 1);
        if x1 == Self::NONE {
            hull_edge[b] = n1;
        }
        if x2 == Self::NONE {
            hull_edge[c] = n2;
        }
    }

    /// The triangle of hull half-edge `e` split at `p`, on that edge.
    fn split_hull(
        &mut self,
        e: usize,
        p: usize,
        next: &mut [usize],
        prev: &mut [usize],
        hull_edge: &mut [usize],
    ) {
        let t = e - e % 3;
        let (h1, h2) = (t + (e - t + 1) % 3, t + (e - t + 2) % 3);
        let (a, b, r) = (self.tris[e], self.tris[h1], self.tris[h2]);
        let x = self.twin[h1];
        // (a, b, r) becomes (a, p, r) and (p, b, r).
        self.tris[h1] = p;
        let m = self.add(p, b, r, Self::NONE, x, h1);
        if x == Self::NONE {
            hull_edge[b] = m + 1;
        }
        hull_edge[a] = e;
        hull_edge[p] = m;
        next[a] = p;
        prev[p] = a;
        next[p] = b;
        prev[b] = p;
    }

    /// The two triangles of inner half-edge `e` split at `p`, on it.
    fn split_edge(&mut self, e: usize, p: usize, hull_edge: &mut [usize]) {
        let f = self.twin[e];
        let t = e - e % 3;
        let (e1, e2) = (t + (e - t + 1) % 3, t + (e - t + 2) % 3);
        let s = f - f % 3;
        let (f1, f2) = (s + (f - s + 1) % 3, s + (f - s + 2) % 3);
        // e: a → b with c opposite; f: b → a with d opposite.
        let (a, b, c) = (self.tris[e], self.tris[e1], self.tris[e2]);
        let d = self.tris[f2];
        let (xe1, xf1) = (self.twin[e1], self.twin[f1]);
        // (a, b, c) → (a, p, c) + (p, b, c); (b, a, d) → (b, p, d) + (p, a, d).
        self.tris[e1] = p;
        self.tris[f1] = p;
        let m = self.add(p, b, c, Self::NONE, xe1, e1);
        let k = self.add(p, a, d, Self::NONE, xf1, f1);
        // a → p (e) faces p → a (k); p → b (m) faces b → p (f).
        self.link(e, k);
        self.link(m, f);
        if xe1 == Self::NONE {
            hull_edge[b] = m + 1;
        }
        if xf1 == Self::NONE {
            hull_edge[a] = k + 1;
        }
    }

    /// Flips edges until every triangle's circle is empty.
    fn flip_all(&mut self, pts: &[Point2]) {
        let mut stack: Vec<usize> = (0..self.tris.len())
            .filter(|&h| self.twin[h] != Self::NONE && h < self.twin[h])
            .collect();
        let mut guard = 0usize;
        let limit = 50 * self.tris.len() + 1000;
        while let Some(a) = stack.pop() {
            guard += 1;
            if guard > limit {
                break;
            }
            let b = self.twin[a];
            if b == Self::NONE {
                continue;
            }
            let ta = a - a % 3;
            let tb = b - b % 3;
            let al = ta + (a - ta + 1) % 3;
            let ar = ta + (a - ta + 2) % 3;
            let br = tb + (b - tb + 1) % 3;
            let bl = tb + (b - tb + 2) % 3;
            let (s, q, p0) = (self.tris[a], self.tris[al], self.tris[ar]);
            let p1 = self.tris[bl];
            if !in_circle(pts[s], pts[q], pts[p0], pts[p1]) {
                continue;
            }
            // Only a convex quadrilateral can be flipped.
            let c1 = (pts[p1] - pts[p0]).cross(pts[s] - pts[p0]);
            let c2 = (pts[p1] - pts[p0]).cross(pts[q] - pts[p0]);
            if c1 * c2 >= 0.0 {
                continue;
            }
            let (hbl, har) = (self.twin[bl], self.twin[ar]);
            self.tris[a] = p1;
            self.tris[b] = p0;
            self.link(a, hbl);
            self.link(b, har);
            self.link(ar, bl);
            for h in [a, al, b, br] {
                if self.twin[h] != Self::NONE {
                    stack.push(h);
                }
            }
        }
    }
}

impl Model {
    fn validate_terrain(&self, t: &NewTerrain) -> Result<(), ModelError> {
        if !self.levels.contains_key(&t.level) {
            return Err(ModelError::UnknownLevel(t.level));
        }
        if !self.materials.contains_key(&t.material) {
            return Err(ModelError::UnknownMaterial(t.material));
        }
        for p in &t.points {
            finite_point(p.at, "terrain point")?;
            finite(p.z, "terrain point height")?;
        }
        finite(t.depth, "terrain depth")?;
        finite(t.contour, "terrain contour interval")?;
        if t.depth <= EPSILON || t.contour < 0.0 {
            return Err(ModelError::InvalidTerrain);
        }
        let n = t.points.len();
        if t.lines
            .iter()
            .any(|l| l.len() < 2 || l.iter().any(|&i| i >= n))
        {
            return Err(ModelError::InvalidTerrain);
        }
        let pts: Vec<Point2> = t.points.iter().map(|p| p.at).collect();
        if delaunay(&pts).is_empty() {
            return Err(ModelError::InvalidTerrain);
        }
        Ok(())
    }

    pub fn add_terrain(&mut self, spec: NewTerrain) -> Result<TerrainId, ModelError> {
        self.validate_terrain(&spec)?;
        let id = TerrainId(self.take_id());
        self.terrains.insert(
            id,
            Terrain {
                id,
                guid: GlobalId::new(),
                level: spec.level,
                points: spec.points,
                material: spec.material,
                depth: spec.depth,
                contour: spec.contour,
                lines: spec.lines,
            },
        );
        Ok(id)
    }

    /// Changes a terrain; nothing changes if the result is not valid.
    pub fn edit_terrain(
        &mut self,
        id: TerrainId,
        edit: impl FnOnce(&mut Terrain),
    ) -> Result<(), ModelError> {
        let mut t = self
            .terrains
            .get(&id)
            .cloned()
            .ok_or(ModelError::UnknownTerrain(id))?;
        edit(&mut t);
        self.validate_terrain(&spec_of(&t))?;
        self.terrains.insert(id, t);
        Ok(())
    }

    pub fn remove_terrain(&mut self, id: TerrainId) -> Result<Terrain, ModelError> {
        self.terrains
            .remove(&id)
            .ok_or(ModelError::UnknownTerrain(id))
    }

    pub fn terrain(&self, id: TerrainId) -> Option<&Terrain> {
        self.terrains.get(&id)
    }

    pub fn terrains(&self) -> impl Iterator<Item = &Terrain> {
        self.terrains.values()
    }

    pub fn terrains_on_level(&self, level: LevelId) -> impl Iterator<Item = &Terrain> {
        self.terrains.values().filter(move |t| t.level == level)
    }

    /// True north: radians, clockwise from the plan's up.
    pub fn true_north(&self) -> f64 {
        self.true_north
    }

    /// Sets the true north (radians, clockwise from the plan's up).
    pub fn set_true_north(&mut self, angle: f64) -> Result<(), ModelError> {
        finite(angle, "true north")?;
        let t = std::f64::consts::TAU;
        self.true_north = angle.rem_euclid(t);
        if self.true_north > std::f64::consts::PI {
            self.true_north -= t;
        }
        // Saved in degrees: round off what the conversion leaves.
        self.true_north = (self.true_north * 1e12).round() / 1e12;
        Ok(())
    }

    pub(super) fn restore_terrains(
        &mut self,
        terrains: Vec<Terrain>,
        claim: &mut structure::Claim<'_>,
    ) -> Result<(), RestoreError> {
        for t in terrains {
            let name = format!("terrain {} ({})", t.id.0, t.guid);
            claim(t.id.0, Some(&t.guid))
                .and_then(|_| self.validate_terrain(&spec_of(&t)))
                .map_err(|error| RestoreError {
                    element: name,
                    error,
                })?;
            self.terrains.insert(t.id, t);
        }
        Ok(())
    }
}

fn spec_of(t: &Terrain) -> NewTerrain {
    NewTerrain {
        level: t.level,
        points: t.points.clone(),
        material: t.material,
        depth: t.depth,
        contour: t.contour,
        lines: t.lines.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sp(x: f64, y: f64, z: f64) -> SurveyPoint {
        SurveyPoint {
            at: Point2::new(x, y),
            z,
        }
    }

    /// A slope rising 1 m every 10 m to the north, and a bump.
    #[test]
    fn terrain_from_survey_points() {
        let mut m = Model::new("t");
        let l = m.add_level("PB", 0.0);
        let earth = m.add_material("Tierra", "earth", [150, 120, 90], CutPattern::Dots);
        let mut points = Vec::new();
        for i in 0..=4 {
            for j in 0..=4 {
                let (x, y) = (i as f64 * 10.0, j as f64 * 10.0);
                points.push(sp(x, y, y / 10.0));
            }
        }
        let id = m
            .add_terrain(NewTerrain {
                level: l,
                points,
                material: earth,
                depth: 2.0,
                contour: 1.0,
                lines: Vec::new(),
            })
            .unwrap();
        let t = m.terrain(id).unwrap();
        assert_eq!(t.triangles().len(), 32);
        assert!((t.height_at(Point2::new(15.0, 25.0)).unwrap() - 2.5).abs() < 1e-9);
        assert!(t.height_at(Point2::new(-1.0, 5.0)).is_none());
        assert_eq!(t.boundary().len(), 16);
        let c = t.contours();
        // 1, 2, 3 and 4 m (the lowest edge, at 0, is not a line).
        assert_eq!(c.len(), 4);
        // The 2 m line runs east–west at y = 20.
        let (z, pieces) = &c[1];
        assert_eq!(*z, 2.0);
        assert!(pieces
            .iter()
            .all(|(a, b)| (a.y - 20.0).abs() < 1e-9 && (b.y - 20.0).abs() < 1e-9));
        let len: f64 = pieces.iter().map(|(a, b)| a.distance(*b)).sum();
        assert!((len - 40.0).abs() < 1e-9);
        // In a line: no surface.
        assert_eq!(
            m.edit_terrain(id, |t| t.points =
                vec![sp(0.0, 0.0, 0.0), sp(1.0, 0.0, 0.0), sp(2.0, 0.0, 0.0)]),
            Err(ModelError::InvalidTerrain)
        );
        m.set_true_north(-0.3).unwrap();
        let back = Model::from_parts(m.to_parts()).unwrap();
        assert_eq!(back, m);
    }

    /// Contour lines drawn by hand: their points no more than a metre
    /// apart, sharing the points already there; removing points keeps the
    /// lines right.
    #[test]
    fn contour_lines_by_hand() {
        let mut m = Model::new("t");
        let l = m.add_level("PB", 0.0);
        let earth = m.add_material("Tierra", "earth", [150, 120, 90], CutPattern::Dots);
        let id = m
            .add_terrain(NewTerrain {
                level: l,
                points: vec![sp(0.0, 0.0, 0.0), sp(10.0, 0.0, 0.0), sp(0.0, 10.0, 0.0)],
                material: earth,
                depth: 1.0,
                contour: 0.5,
                lines: Vec::new(),
            })
            .unwrap();
        m.edit_terrain(id, |t| {
            // From an existing corner, 2.5 m east then 1 m north.
            t.add_line(
                &[
                    Point2::new(0.0, 0.0),
                    Point2::new(2.5, 0.0),
                    Point2::new(2.5, 1.0),
                ],
                1.5,
            );
        })
        .unwrap();
        let t = m.terrain(id).unwrap().clone();
        // 0, 0.83, 1.67, 2.5 and (2.5, 1): the first one reused.
        assert_eq!(t.lines, vec![vec![0, 3, 4, 5, 6]]);
        assert_eq!(t.points.len(), 7);
        assert!(t.lines[0].iter().all(|&i| t.points[i].z == 1.5));
        assert_eq!(
            m.edit_terrain(id, |t| t.lines.push(vec![1])),
            Err(ModelError::InvalidTerrain)
        );
        m.edit_terrain(id, |t| t.remove_points(&[3, 4, 5])).unwrap();
        let t = m.terrain(id).unwrap();
        assert_eq!(t.lines, vec![vec![0, 3]]);
        assert_eq!(t.points[3].at, Point2::new(2.5, 1.0));
        m.edit_terrain(id, |t| t.remove_points(&[3])).unwrap();
        assert!(m.terrain(id).unwrap().lines.is_empty());
        let back = Model::from_parts(m.to_parts()).unwrap();
        assert_eq!(back, m);
    }

    /// Area of the convex hull (monotone chain).
    fn hull_area(points: &[Point2]) -> f64 {
        let mut p: Vec<Point2> = points.to_vec();
        p.sort_by(|a, b| a.x.total_cmp(&b.x).then(a.y.total_cmp(&b.y)));
        let mut hull: Vec<Point2> = Vec::new();
        for pass in 0..2 {
            let start = hull.len();
            let iter: Box<dyn Iterator<Item = &Point2>> = if pass == 0 {
                Box::new(p.iter())
            } else {
                Box::new(p.iter().rev())
            };
            for &q in iter {
                while hull.len() >= start + 2 {
                    let (a, b) = (hull[hull.len() - 2], hull[hull.len() - 1]);
                    if (b - a).cross(q - a) <= 0.0 {
                        hull.pop();
                    } else {
                        break;
                    }
                }
                hull.push(q);
            }
            hull.pop();
        }
        crate::signed_area(&hull)
    }

    /// Every triangle's circle is empty, and they cover the convex hull.
    fn check(points: &[Point2]) {
        let tris = delaunay(points);
        let area = |ts: &[[usize; 3]]| -> f64 {
            ts.iter()
                .map(|t| (points[t[1]] - points[t[0]]).cross(points[t[2]] - points[t[0]]) / 2.0)
                .sum()
        };
        assert!(tris
            .iter()
            .all(|t| (points[t[1]] - points[t[0]]).cross(points[t[2]] - points[t[0]]) > 0.0));
        // They cover the convex hull (the old triangulation could miss
        // thin triangles on it).
        let (a, b) = (area(&tris), hull_area(points));
        assert!(
            (a - b).abs() <= 1e-6 * b.abs().max(1.0),
            "area {a} vs hull {b}"
        );
        // Every point used (but repeated ones).
        let mut used = vec![false; points.len()];
        for t in &tris {
            for &i in t {
                used[i] = true;
            }
        }
        for (i, p) in points.iter().enumerate() {
            let repeated = points[..i].iter().any(|q| q.approx_eq(*p));
            assert!(used[i] || repeated, "point {i} {p:?} left out");
        }
        for t in &tris {
            let (pa, pb, pc) = (points[t[0]], points[t[1]], points[t[2]]);
            let o = circumcentre(pa, pb, pc).unwrap();
            let r = o.distance(pa);
            for (i, p) in points.iter().enumerate() {
                if !t.contains(&i) {
                    assert!(o.distance(*p) >= r - 1e-6 * r.max(1.0), "{t:?} holds {i}");
                }
            }
        }
    }

    #[test]
    fn sweep_triangulation_is_delaunay() {
        let mut seed = 99u64;
        let mut rnd = || {
            seed ^= seed << 13;
            seed ^= seed >> 7;
            seed ^= seed << 17;
            (seed % 100_000) as f64 / 1000.0
        };
        // Scattered points.
        let pts: Vec<Point2> = (0..400).map(|_| Point2::new(rnd(), rnd())).collect();
        check(&pts);
        // A grid (every four corners on one circle), with repeated points.
        let mut grid = Vec::new();
        for i in 0..12 {
            for j in 0..9 {
                grid.push(Point2::new(i as f64 * 2.0, j as f64 * 2.0));
            }
        }
        grid.push(Point2::new(4.0, 4.0));
        grid.push(Point2::new(0.0, 0.0));
        check(&grid);
        // Survey coordinates (UTM), and points on lines.
        let utm: Vec<Point2> = (0..300)
            .map(|k| {
                let (x, y) = (rnd(), rnd());
                if k % 3 == 0 {
                    Point2::new(431_000.0 + x, 4_581_000.0)
                } else {
                    Point2::new(431_000.0 + x, 4_581_000.0 + y)
                }
            })
            .collect();
        check(&utm);
        // In a line: nothing.
        let line: Vec<Point2> = (0..10)
            .map(|i| Point2::new(i as f64, 2.0 * i as f64))
            .collect();
        assert!(delaunay(&line).is_empty());
    }
}
