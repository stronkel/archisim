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
    /// Areas painted over it (lawn, gravel…) with the Exterior brushes
    /// (MQ3, C5): a material and the regions it covers in plan, each an
    /// outline and its holes. Two patches never overlap.
    pub patches: Vec<TerrainPatch>,
    /// Its edge drawn by hand (E5): the surface stops there, even where it
    /// goes in (with none, its edge is the hull of its points).
    pub outline: Option<Vec<Point2>>,
    /// Heights written beside the survey points in plan (E5: hidden by
    /// default; the contour lines carry theirs).
    pub show_heights: bool,
    /// Slope arrows in plan, downhill, with their gradient (E5).
    pub slope_arrows: bool,
    /// Platforms dug or filled into it (E8: "explanaciones"), under the
    /// building or by hand. `points` is the ground already graded.
    pub pads: Vec<TerrainPad>,
    /// The ground as surveyed, before the platforms (kept while there are
    /// any, to grade again and to measure cut and fill).
    pub ground: Option<Vec<SurveyPoint>>,
}

/// A platform in the ground (E8): flat at `z` inside `outline`, with a
/// bank ("talud") around it that rises or falls `slope` metres per metre
/// until it meets the ground.
#[derive(Debug, Clone, PartialEq)]
pub struct TerrainPad {
    /// Counter-clockwise, without repeating the first point.
    pub outline: Vec<Point2>,
    pub z: f64,
    /// Rise over run of the bank (1 = 1:1, 45°; 0.5 = 1:2).
    pub slope: f64,
}

/// Bank slope proposed for new platforms (1:1).
pub const PAD_SLOPE: f64 = 1.0;
/// Margin around the building proposed for its platforms.
pub const PAD_MARGIN: f64 = 0.5;

/// An area of a terrain covered by another material.
#[derive(Debug, Clone, PartialEq)]
pub struct TerrainPatch {
    pub material: MaterialId,
    /// Outlines (counter-clockwise) and their holes.
    pub regions: Vec<(Vec<Point2>, Vec<Vec<Point2>>)>,
}

impl TerrainPatch {
    /// Its area in plan.
    pub fn area(&self) -> f64 {
        self.regions
            .iter()
            .map(|(o, holes)| {
                signed_area(o).abs() - holes.iter().map(|h| signed_area(h).abs()).sum::<f64>()
            })
            .sum()
    }
}

/// What an Exterior brush does to the ground under it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Sculpt {
    /// Up by the amount (at the middle; less towards the edge).
    Raise,
    Lower,
    /// Towards the mean of the points round each one.
    Smooth,
    /// Towards this height (from project zero).
    Flatten(f64),
}

/// How far apart the points a brush adds are, at least (m).
pub const SCULPT_STEP: f64 = 0.5;

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
        if let Some(ground) = &mut self.ground {
            let n = ground.len();
            *ground = std::mem::take(ground)
                .into_iter()
                .enumerate()
                .filter(|(i, _)| !gone.contains(i) || *i >= n)
                .map(|(_, p)| p)
                .collect();
        }
        self.lines = std::mem::take(&mut self.lines)
            .into_iter()
            .map(|l| l.into_iter().filter_map(|i| map[i]).collect::<Vec<_>>())
            .filter(|l| l.len() >= 2)
            .collect();
        if !self.pads.is_empty() && self.ground.is_some() {
            let pads = std::mem::take(&mut self.pads);
            self.set_pads(pads);
        }
    }

    /// Works the ground with a brush of `radius` at `center`: raises,
    /// lowers, smooths or flattens it by `amount` (metres up or down at
    /// its middle for raising and lowering; between 0 and 1, how much,
    /// for the others), less towards its edge. Where its points are
    /// sparse, it first adds some under the brush, on the surface as it
    /// is, so there is ground to shape.
    pub fn sculpt(&mut self, center: Point2, radius: f64, how: Sculpt, amount: f64) {
        if radius <= 1e-6 {
            return;
        }
        self.densify(center, radius);
        let weight = |p: Point2| {
            let d = p.distance(center) / radius;
            if d >= 1.0 {
                0.0
            } else {
                let w = 1.0 - d * d;
                w * w
            }
        };
        let near: Vec<usize> = (0..self.points.len())
            .filter(|&i| weight(self.points[i].at) > 0.0)
            .collect();
        let before: Vec<SurveyPoint> = self.points.clone();
        for &i in &near {
            let p = before[i];
            let w = weight(p.at);
            let z = match how {
                Sculpt::Raise => p.z + amount * w,
                Sculpt::Lower => p.z - amount * w,
                Sculpt::Flatten(h) => p.z + (h - p.z) * (amount.clamp(0.0, 1.0) * w),
                Sculpt::Smooth => {
                    let reach = (radius * 0.35).max(SCULPT_STEP * 1.6);
                    let (sum, n) = before
                        .iter()
                        .filter(|q| q.at.distance(p.at) <= reach)
                        .fold((0.0, 0.0), |(s, n), q| (s + q.z, n + 1.0));
                    let mean = if n > 0.0 { sum / n } else { p.z };
                    p.z + (mean - p.z) * (amount.clamp(0.0, 1.0) * w)
                }
            };
            self.points[i].z = z;
        }
    }

    /// Adds points under a brush, on a grid, where there are none near:
    /// at the height the surface has there (inside it) or the nearest
    /// point's (outside, which widens the terrain).
    fn densify(&mut self, center: Point2, radius: f64) {
        let step = (radius / 4.0).max(SCULPT_STEP);
        let tris = self.triangles();
        let height = |pts: &[SurveyPoint], p: Point2| -> f64 {
            for [a, b, c] in &tris {
                let (pa, pb, pc) = (pts[*a], pts[*b], pts[*c]);
                if let Some((u, v, w)) = barycentric(p, pa.at, pb.at, pc.at) {
                    return u * pa.z + v * pb.z + w * pc.z;
                }
            }
            pts.iter()
                .min_by(|x, y| x.at.distance(p).total_cmp(&y.at.distance(p)))
                .map_or(0.0, |q| q.z)
        };
        // A ring of points just outside the brush too, at the height they
        // have: what is raised does not slope the ground beyond.
        let reach = radius + 1.5 * step;
        let n = (reach / step).ceil() as i64;
        let mut added = Vec::new();
        for j in -n..=n {
            for i in -n..=n {
                // On a grid of the project's, not of the brush, so strokes
                // over the same ground share their points.
                let g = Point2::new(
                    ((center.x / step).round() + i as f64) * step,
                    ((center.y / step).round() + j as f64) * step,
                );
                if g.distance(center) > reach {
                    continue;
                }
                let free = self
                    .points
                    .iter()
                    .chain(added.iter())
                    .all(|q: &SurveyPoint| q.at.distance(g) > step * 0.7);
                if free {
                    added.push(SurveyPoint {
                        at: g,
                        z: height(&self.points, g),
                    });
                }
            }
        }
        self.points.extend(added);
    }

    /// Paints `material` over the regions `shapes` (outlines and holes,
    /// already joined with what that material covered): the other
    /// patches give way where they meet. Patches left empty go.
    pub fn set_patches(&mut self, patches: Vec<TerrainPatch>) {
        self.patches = patches
            .into_iter()
            .filter(|p| p.regions.iter().any(|(o, _)| o.len() >= 3))
            .collect();
    }

    /// The surface's triangles, counter-clockwise in plan, by point index;
    /// with an edge drawn by hand, only those inside it.
    pub fn triangles(&self) -> Vec<[usize; 3]> {
        let all = delaunay(&self.points.iter().map(|p| p.at).collect::<Vec<_>>());
        match self.outline.as_deref().filter(|o| o.len() >= 3) {
            Some(o) => all
                .into_iter()
                .filter(|[a, b, c]| {
                    let m = (self.points[*a].at + self.points[*b].at + self.points[*c].at)
                        * (1.0 / 3.0);
                    super::roofs::point_in_polygon(m, o)
                })
                .collect(),
            None => all,
        }
    }

    /// Gives the terrain an edge drawn by hand: its points are kept, and
    /// points are added along the edge (every metre at most) at the height
    /// of the ground there (or of the nearest point, outside it), so that
    /// the surface reaches the edge.
    pub fn set_outline(&mut self, outline: Vec<Point2>) {
        let before = Terrain {
            outline: None,
            ..self.clone()
        };
        let mut add = Vec::new();
        let n = outline.len();
        for i in 0..n {
            let (a, b) = (outline[i], outline[(i + 1) % n]);
            let steps = (a.distance(b) / CONTOUR_POINT_SPACING).ceil().max(1.0) as usize;
            for k in 0..steps {
                let q = a + (b - a) * (k as f64 / steps as f64);
                if self
                    .points
                    .iter()
                    .chain(add.iter())
                    .any(|p: &SurveyPoint| p.at.approx_eq(q))
                {
                    continue;
                }
                let z = before.height_at(q).unwrap_or_else(|| {
                    self.points
                        .iter()
                        .min_by(|x, y| x.at.distance(q).total_cmp(&y.at.distance(q)))
                        .map_or(0.0, |p| p.z)
                });
                add.push(SurveyPoint { at: q, z });
            }
        }
        if let Some(ground) = &mut self.ground {
            // The platforms' points come after the ground's: keep it so.
            let n = ground.len();
            ground.extend(add.iter().copied());
            let graded: Vec<SurveyPoint> = self.points.drain(n..).collect();
            self.points.extend(add);
            self.points.extend(graded);
        } else {
            self.points.extend(add);
        }
        self.outline = Some(outline);
    }

    /// The ground as surveyed (without the platforms).
    pub fn survey(&self) -> &[SurveyPoint] {
        self.ground.as_deref().unwrap_or(&self.points)
    }

    /// Sets its platforms and grades the ground for them, from the ground
    /// as surveyed: inside each, flat at its height; around it, a bank
    /// to the ground. The lower ones go last, so a dig wins over a fill
    /// where they meet. With none, the survey comes back.
    ///
    /// The surveyed points keep their indices (the contour lines drawn
    /// by hand still name them); the platforms' points go after them.
    pub fn set_pads(&mut self, pads: Vec<TerrainPad>) {
        let ground = self.ground.take().unwrap_or_else(|| self.points.clone());
        let pads: Vec<TerrainPad> = pads
            .into_iter()
            .filter(|p| p.outline.len() >= 3 && p.slope > 1e-6)
            .map(|mut p| {
                if signed_area(&p.outline) < 0.0 {
                    p.outline.reverse();
                }
                p
            })
            .collect();
        if pads.is_empty() {
            self.points = ground;
            self.pads = Vec::new();
            return;
        }
        let mut order: Vec<&TerrainPad> = pads.iter().collect();
        order.sort_by(|a, b| b.z.total_cmp(&a.z));
        let mut points = ground.clone();
        for pad in order {
            points = grade(&points, pad, self.outline.as_deref());
        }
        self.points = points;
        self.pads = pads;
        self.ground = Some(ground);
    }

    /// Changes its surveyed points (heights, places): with platforms, the
    /// survey kept apart, and the ground is graded again.
    pub fn edit_survey(&mut self, edit: impl FnOnce(&mut Vec<SurveyPoint>)) {
        match self.ground.take() {
            Some(mut ground) if !self.pads.is_empty() => {
                edit(&mut ground);
                self.ground = Some(ground);
                let pads = std::mem::take(&mut self.pads);
                self.set_pads(pads);
            }
            other => {
                self.ground = other;
                edit(&mut self.points);
            }
        }
    }

    /// Earthworks for its platforms: the volume dug ("desmonte") and the
    /// volume filled ("terraplén"), in cubic metres, between the ground
    /// as surveyed and as graded.
    pub fn earthworks(&self) -> (f64, f64) {
        let Some(ground) = &self.ground else {
            return (0.0, 0.0);
        };
        if self.pads.is_empty() {
            return (0.0, 0.0);
        }
        let surveyed = Terrain {
            points: ground.clone(),
            pads: Vec::new(),
            ground: None,
            ..self.clone()
        };
        // Only near the platforms does the ground change.
        let (mut lo, mut hi) = (
            Point2::new(f64::MAX, f64::MAX),
            Point2::new(f64::MIN, f64::MIN),
        );
        for p in self.points.iter().skip(ground.len()) {
            lo = Point2::new(lo.x.min(p.at.x), lo.y.min(p.at.y));
            hi = Point2::new(hi.x.max(p.at.x), hi.y.max(p.at.y));
        }
        for pad in &self.pads {
            for p in &pad.outline {
                lo = Point2::new(lo.x.min(p.x), lo.y.min(p.y));
                hi = Point2::new(hi.x.max(p.x), hi.y.max(p.y));
            }
        }
        if lo.x > hi.x {
            return (0.0, 0.0);
        }
        let area = (hi.x - lo.x) * (hi.y - lo.y);
        let step = (area / 250_000.0).sqrt().max(EARTHWORKS_STEP);
        let nx = ((hi.x - lo.x) / step).ceil().max(1.0) as usize;
        let ny = ((hi.y - lo.y) / step).ceil().max(1.0) as usize;
        let before = surveyed.sample(lo, step, nx, ny);
        let after = self.sample(lo, step, nx, ny);
        let (mut cut, mut fill) = (0.0, 0.0);
        for (b, a) in before.iter().zip(&after) {
            if let (Some(b), Some(a)) = (b, a) {
                let d = a - b;
                if d < 0.0 {
                    cut -= d * step * step;
                } else {
                    fill += d * step * step;
                }
            }
        }
        (cut, fill)
    }

    /// Heights of the surface at the middle of each cell of a grid from
    /// `lo`, `step` wide, `nx` by `ny` (by rows), where it lies over it.
    fn sample(&self, lo: Point2, step: f64, nx: usize, ny: usize) -> Vec<Option<f64>> {
        let mut out = vec![None; nx * ny];
        for [a, b, c] in self.triangles() {
            let (pa, pb, pc) = (self.points[a], self.points[b], self.points[c]);
            let x0 = pa.at.x.min(pb.at.x).min(pc.at.x);
            let x1 = pa.at.x.max(pb.at.x).max(pc.at.x);
            let y0 = pa.at.y.min(pb.at.y).min(pc.at.y);
            let y1 = pa.at.y.max(pb.at.y).max(pc.at.y);
            let i0 = (((x0 - lo.x) / step - 0.5).ceil().max(0.0)) as usize;
            let j0 = (((y0 - lo.y) / step - 0.5).ceil().max(0.0)) as usize;
            let i1 = ((x1 - lo.x) / step - 0.5).floor();
            let j1 = ((y1 - lo.y) / step - 0.5).floor();
            if i1 < 0.0 || j1 < 0.0 {
                continue;
            }
            let (i1, j1) = ((i1 as usize).min(nx - 1), (j1 as usize).min(ny - 1));
            for j in j0..=j1 {
                for i in i0..=i1 {
                    let q = Point2::new(
                        lo.x + (i as f64 + 0.5) * step,
                        lo.y + (j as f64 + 0.5) * step,
                    );
                    if let Some((u, v, w)) = barycentric(q, pa.at, pb.at, pc.at) {
                        out[j * nx + i] = Some(u * pa.z + v * pb.z + w * pc.z);
                    }
                }
            }
        }
        out
    }

    /// Slope arrows: on a grid `step` apart over the surface, where the
    /// ground falls, the point, the downhill direction and the gradient
    /// (rise over run).
    pub fn slope_marks(&self, step: f64) -> Vec<(Point2, Vec2, f64)> {
        let mut out = Vec::new();
        let tris = self.triangles();
        if tris.is_empty() || step <= 0.0 {
            return out;
        }
        let (mut lo, mut hi) = (
            Point2::new(f64::MAX, f64::MAX),
            Point2::new(f64::MIN, f64::MIN),
        );
        for p in &self.points {
            lo = Point2::new(lo.x.min(p.at.x), lo.y.min(p.at.y));
            hi = Point2::new(hi.x.max(p.at.x), hi.y.max(p.at.y));
        }
        let nx = ((hi.x - lo.x) / step).floor() as i64;
        let ny = ((hi.y - lo.y) / step).floor() as i64;
        if nx * ny > 4000 {
            return out;
        }
        for i in 0..=nx {
            for j in 0..=ny {
                let q = Point2::new(
                    lo.x + (i as f64 + 0.5) * step,
                    lo.y + (j as f64 + 0.5) * step,
                );
                for [a, b, c] in &tris {
                    let (pa, pb, pc) = (self.points[*a], self.points[*b], self.points[*c]);
                    if barycentric(q, pa.at, pb.at, pc.at).is_none() {
                        continue;
                    }
                    // The plane's gradient.
                    let (u, v) = (pb.at - pa.at, pc.at - pa.at);
                    let (du, dv) = (pb.z - pa.z, pc.z - pa.z);
                    let det = u.x * v.y - u.y * v.x;
                    if det.abs() < 1e-12 {
                        break;
                    }
                    let gx = (du * v.y - dv * u.y) / det;
                    let gy = (dv * u.x - du * v.x) / det;
                    let g = (gx * gx + gy * gy).sqrt();
                    if g > 0.005 {
                        out.push((q, Vec2::new(-gx / g, -gy / g), g));
                    }
                    break;
                }
            }
        }
        out
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
/// Grid of the earthworks measure (metres; wider on large sites).
pub const EARTHWORKS_STEP: f64 = 0.5;

/// Distances from a platform's edge at which its bank gets points.
const BANK_RINGS: [f64; 12] = [
    0.5, 1.0, 1.5, 2.0, 3.0, 4.0, 5.5, 7.0, 9.0, 12.0, 16.0, 21.0,
];

/// `poly` (counter-clockwise) moved out by `d`, with mitred corners (a
/// sharp one cut at twice `d`).
pub fn grow_polygon(poly: &[Point2], d: f64) -> Vec<Point2> {
    let n = poly.len();
    if d.abs() < 1e-9 || n < 3 {
        return poly.to_vec();
    }
    let normal = |i: usize| {
        let v = poly[(i + 1) % n] - poly[i];
        let l = (v.x * v.x + v.y * v.y).sqrt().max(1e-12);
        Vec2::new(v.y / l, -v.x / l)
    };
    (0..n)
        .map(|i| {
            let (a, b) = (normal((i + n - 1) % n), normal(i));
            let m = a + b;
            let c = (1.0 + a.x * b.x + a.y * b.y) / 2.0;
            let k = if c > 0.125 { d / (2.0 * c) } else { d * 2.0 };
            let ml = (m.x * m.x + m.y * m.y).sqrt();
            if ml < 1e-9 {
                poly[i] + b * d
            } else {
                poly[i] + m * (k * 2.0 * c.sqrt() / ml)
            }
        })
        .collect()
}

/// Distance from `p` to the edge of `poly`; zero inside it.
fn distance_out(p: Point2, poly: &[Point2]) -> f64 {
    if super::roofs::point_in_polygon(p, poly) {
        return 0.0;
    }
    let n = poly.len();
    (0..n)
        .map(|i| segment_distance(p, poly[i], poly[(i + 1) % n]))
        .fold(f64::MAX, f64::min)
}

fn segment_distance(p: Point2, a: Point2, b: Point2) -> f64 {
    let ab = b - a;
    let len2 = ab.x * ab.x + ab.y * ab.y;
    let t = if len2 < 1e-18 {
        0.0
    } else {
        (((p - a).x * ab.x + (p - a).y * ab.y) / len2).clamp(0.0, 1.0)
    };
    p.distance(a + ab * t)
}

/// The ground `points` graded for one platform: points inside it go to
/// its height, those around it are held by its bank, and points are added
/// along its edge and in rings around it (where there is ground) so the
/// surface follows the platform and its bank.
fn grade(points: &[SurveyPoint], pad: &TerrainPad, outline: Option<&[Point2]>) -> Vec<SurveyPoint> {
    let before = Terrain {
        id: TerrainId(0),
        guid: GlobalId::new(),
        level: LevelId(0),
        points: points.to_vec(),
        material: MaterialId(0),
        depth: 1.0,
        contour: 0.0,
        lines: Vec::new(),
        patches: Vec::new(),
        outline: outline.map(|o| o.to_vec()),
        show_heights: false,
        slope_arrows: false,
        pads: Vec::new(),
        ground: None,
    };
    let held = |q: Point2, z: f64| {
        let d = distance_out(q, &pad.outline);
        z.clamp(pad.z - d * pad.slope, pad.z + d * pad.slope)
    };
    let mut out: Vec<SurveyPoint> = points
        .iter()
        .map(|p| SurveyPoint {
            at: p.at,
            z: held(p.at, p.z),
        })
        .collect();
    let edge = &pad.outline;
    let n = edge.len();
    // The edge, every metre at most, and its outward directions.
    let mut samples: Vec<(Point2, Vec<Vec2>)> = Vec::new();
    let normal = |i: usize| {
        let d = edge[(i + 1) % n] - edge[i];
        let l = (d.x * d.x + d.y * d.y).sqrt().max(1e-12);
        Vec2::new(d.y / l, -d.x / l)
    };
    for i in 0..n {
        let (a, b) = (edge[i], edge[(i + 1) % n]);
        let (prev, here) = (normal((i + n - 1) % n), normal(i));
        // A corner that turns outward gets a fan of directions.
        let turn = prev.x * here.y - prev.y * here.x;
        let mut dirs = vec![here];
        if turn > 1e-9 || (prev.x * here.x + prev.y * here.y) < 0.999 {
            let a0 = prev.y.atan2(prev.x);
            let mut a1 = here.y.atan2(here.x);
            while a1 < a0 {
                a1 += std::f64::consts::TAU;
            }
            if a1 - a0 < std::f64::consts::PI {
                let k = ((a1 - a0) / 0.4).ceil() as usize;
                dirs = (0..=k)
                    .map(|j| {
                        let t = a0 + (a1 - a0) * j as f64 / k as f64;
                        Vec2::new(t.cos(), t.sin())
                    })
                    .collect();
            }
        }
        samples.push((a, dirs));
        let steps = (a.distance(b) / CONTOUR_POINT_SPACING).ceil().max(1.0) as usize;
        for k in 1..steps {
            samples.push((a + (b - a) * (k as f64 / steps as f64), vec![here]));
        }
    }
    let mut added: Vec<SurveyPoint> = Vec::new();
    let near = |pts: &[SurveyPoint], added: &[SurveyPoint], q: Point2, gap: f64| {
        pts.iter().chain(added).any(|p| p.at.distance(q) < gap)
    };
    for (q, _) in &samples {
        if before.height_at(*q).is_some() && !near(&out, &added, *q, 0.05) {
            added.push(SurveyPoint { at: *q, z: pad.z });
        }
    }
    // Points well inside go (the platform is flat); those near its edge
    // already moved to its height.
    for ring in BANK_RINGS {
        let mut any_held = false;
        for (q, dirs) in &samples {
            for d in dirs {
                let r = *q + *d * ring;
                let real = distance_out(r, &pad.outline);
                if real < ring * 0.6 {
                    continue;
                }
                let Some(g) = before.height_at(r) else {
                    continue;
                };
                let z = held(r, g);
                any_held |= (z - g).abs() > 1e-3;
                let gap = (ring * 0.3).clamp(0.3, 1.2);
                if !near(&out, &added, r, gap) {
                    added.push(SurveyPoint { at: r, z });
                }
            }
        }
        if !any_held {
            break;
        }
    }
    out.extend(added);
    out
}

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
                patches: Vec::new(),
                outline: None,
                show_heights: false,
                slope_arrows: false,
                pads: Vec::new(),
                ground: None,
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
        check_pads(&t)?;
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

    /// Platforms for the building on a terrain (E8): one under each slab
    /// of its level and the ones below it (ground floor and basements),
    /// widened by `margin`, at the slab's underside, with banks of
    /// `slope`. Roof slabs are left out.
    pub fn building_pads(&self, terrain: TerrainId, margin: f64, slope: f64) -> Vec<TerrainPad> {
        let Some(t) = self.terrains.get(&terrain) else {
            return Vec::new();
        };
        let top = self.levels.get(&t.level).map_or(0.0, |l| l.elevation);
        let mut pads = Vec::new();
        for s in self.slabs.values() {
            let Some(level) = self.levels.get(&s.level) else {
                continue;
            };
            let Some(ty) = self.slab_types.get(&s.slab_type) else {
                continue;
            };
            if level.elevation > top + 0.05 || ty.function == SlabFunction::Roof {
                continue;
            }
            let lowest = s.slope.as_ref().map_or(0.0, |sl| {
                s.outline.iter().map(|p| sl.rise_at(*p)).fold(0.0, f64::min)
            });
            let z = level.elevation + s.top_offset + lowest - ty.thickness();
            let mut outline = s.outline.clone();
            if signed_area(&outline) < 0.0 {
                outline.reverse();
            }
            pads.push(TerrainPad {
                outline: grow_polygon(&outline, margin),
                z,
                slope,
            });
        }
        pads
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
                .and_then(|_| check_pads(&t))
                .map_err(|error| RestoreError {
                    element: name,
                    error,
                })?;
            self.terrains.insert(t.id, t);
        }
        Ok(())
    }
}

fn check_pads(t: &Terrain) -> Result<(), ModelError> {
    for pad in &t.pads {
        for p in &pad.outline {
            finite_point(*p, "terrain platform")?;
        }
        finite(pad.z, "terrain platform height")?;
        finite(pad.slope, "terrain platform bank")?;
        if pad.outline.len() < 3 || pad.slope <= 0.0 {
            return Err(ModelError::InvalidTerrain);
        }
    }
    for p in t.ground.iter().flatten() {
        finite_point(p.at, "terrain point")?;
        finite(p.z, "terrain point height")?;
    }
    Ok(())
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

    /// E5: an L-shaped edge drawn by hand keeps the surface out of the
    /// missing corner; the slope arrows point downhill with the gradient.
    #[test]
    fn platforms_dig_and_fill_with_banks() {
        let mut m = Model::new("t");
        let l = m.add_level("PB", 0.0);
        let earth = m.add_material("Tierra", "earth", [150, 120, 90], CutPattern::Dots);
        let mut points = Vec::new();
        for i in 0..=8 {
            for j in 0..=8 {
                let (x, y) = (i as f64 * 5.0, j as f64 * 5.0);
                points.push(sp(x, y, y / 10.0));
            }
        }
        let n = points.len();
        let id = m
            .add_terrain(NewTerrain {
                level: l,
                points,
                material: earth,
                depth: 2.0,
                contour: 0.5,
                lines: vec![vec![0, 1]],
            })
            .unwrap();
        let square = |x0: f64, y0: f64, s: f64| {
            vec![
                Point2::new(x0, y0),
                Point2::new(x0 + s, y0),
                Point2::new(x0 + s, y0 + s),
                Point2::new(x0, y0 + s),
            ]
        };
        // Dug 1 m below the middle of a 10 × 10 m square, ground 1,5–2,5.
        let dig = TerrainPad {
            outline: square(15.0, 15.0, 10.0),
            z: 1.0,
            slope: 1.0,
        };
        m.edit_terrain(id, |t| t.set_pads(vec![dig.clone()]))
            .unwrap();
        let t = m.terrain(id).unwrap();
        assert_eq!(t.survey().len(), n);
        assert_eq!(
            t.lines,
            vec![vec![0, 1]],
            "the surveyed points keep their place"
        );
        for q in [(16.0, 16.0), (20.0, 20.0), (24.5, 24.5)] {
            let z = t.height_at(Point2::new(q.0, q.1)).unwrap();
            assert!((z - 1.0).abs() < 1e-6, "{q:?}: {z}");
        }
        // North of it, the bank: 1 m out, at most 1 m up.
        let z = t.height_at(Point2::new(20.0, 26.0)).unwrap();
        assert!(z <= 2.0 + 1e-6 && z > 1.5, "{z}");
        // Far away, the ground as it was.
        let z = t.height_at(Point2::new(20.0, 39.0)).unwrap();
        assert!((z - 3.9).abs() < 1e-6, "{z}");
        let (cut, fill) = t.earthworks();
        // 100 m³ under the square, plus the banks.
        assert!(cut > 100.0 && cut < 130.0, "{cut}");
        assert!(fill < 1.0, "{fill}");
        // A raised one fills.
        let raise = TerrainPad {
            outline: square(5.0, 5.0, 5.0),
            z: 1.5,
            slope: 0.5,
        };
        m.edit_terrain(id, |t| t.set_pads(vec![raise])).unwrap();
        let (cut, fill) = m.terrain(id).unwrap().earthworks();
        // Ground 0,5–1,0 under it: 25 × 0,75 = 18,75 m³ and the banks.
        assert!(fill > 18.0 && fill < 40.0, "{fill}");
        assert!(cut < 0.5, "{cut}");
        // A survey point changed: the ground is graded again.
        m.edit_terrain(id, |t| t.edit_survey(|g| g[0].z = -1.0))
            .unwrap();
        let t = m.terrain(id).unwrap();
        assert_eq!(t.survey()[0].z, -1.0);
        assert_eq!(t.points[0].z, -1.0);
        assert!(!t.pads.is_empty());
        // None: the survey comes back.
        m.edit_terrain(id, |t| t.set_pads(Vec::new())).unwrap();
        let t = m.terrain(id).unwrap();
        assert_eq!(t.points.len(), n);
        assert!(t.ground.is_none() && t.pads.is_empty());
        assert_eq!(t.earthworks(), (0.0, 0.0));
    }

    #[test]
    fn platforms_under_the_building() {
        let mut m = Model::new("t");
        let sotano = m.add_level("Sótano", -3.0);
        let pb = m.add_level("PB", 0.0);
        let p1 = m.add_level("P1", 3.0);
        let earth = m.add_material("Tierra", "earth", [150, 120, 90], CutPattern::Dots);
        let id = m
            .add_terrain(NewTerrain {
                level: pb,
                points: vec![
                    sp(-20.0, -20.0, 0.0),
                    sp(20.0, -20.0, 0.0),
                    sp(20.0, 20.0, 0.0),
                    sp(-20.0, 20.0, 0.0),
                ],
                material: earth,
                depth: 2.0,
                contour: 0.5,
                lines: Vec::new(),
            })
            .unwrap();
        let ty = m
            .add_slab_type(
                "Forjado 30",
                SlabFunction::Floor,
                vec![Layer {
                    material: earth,
                    thickness: 0.3,
                    function: LayerFunction::Structure,
                }],
            )
            .unwrap();
        let sq = vec![
            Point2::new(0.0, 0.0),
            Point2::new(10.0, 0.0),
            Point2::new(10.0, 8.0),
            Point2::new(0.0, 8.0),
        ];
        for l in [sotano, pb, p1] {
            m.add_slab(NewSlab {
                slab_type: ty,
                level: l,
                outline: sq.clone(),
                top_offset: 0.0,
            })
            .unwrap();
        }
        let pads = m.building_pads(id, 0.5, 1.0);
        assert_eq!(
            pads.len(),
            2,
            "basement and ground floor, not the first floor"
        );
        let mut zs: Vec<f64> = pads.iter().map(|p| p.z).collect();
        zs.sort_by(f64::total_cmp);
        assert!(
            (zs[0] + 3.3).abs() < 1e-9 && (zs[1] + 0.3).abs() < 1e-9,
            "{zs:?}"
        );
        let o = &pads[0].outline;
        assert!(o[0].approx_eq(Point2::new(-0.5, -0.5)), "{o:?}");
        assert!(o[2].approx_eq(Point2::new(10.5, 8.5)), "{o:?}");
        m.edit_terrain(id, |t| t.set_pads(pads)).unwrap();
        let t = m.terrain(id).unwrap();
        // The dig of the basement wins.
        let z = t.height_at(Point2::new(5.0, 4.0)).unwrap();
        assert!((z + 3.3).abs() < 1e-6, "{z}");
        let (cut, _) = t.earthworks();
        // 11 × 9 × 3,3 = 326,7 m³, and the banks: 40 m round × 3,3² / 2
        // = 218 m³, and their corners.
        assert!(cut > 540.0 && cut < 620.0, "{cut}");
    }

    #[test]
    fn an_edge_by_hand_and_slope_arrows() {
        let mut m = Model::new("t");
        let l = m.add_level("PB", 0.0);
        let earth = m.add_material("Tierra", "earth", [150, 120, 90], CutPattern::Dots);
        let points = vec![
            sp(0.0, 0.0, 0.0),
            sp(10.0, 0.0, 0.0),
            sp(10.0, 10.0, 1.0),
            sp(0.0, 10.0, 1.0),
        ];
        let id = m
            .add_terrain(NewTerrain {
                level: l,
                points,
                material: earth,
                depth: 2.0,
                contour: 0.5,
                lines: Vec::new(),
            })
            .unwrap();
        let ell = vec![
            Point2::new(0.0, 0.0),
            Point2::new(10.0, 0.0),
            Point2::new(10.0, 5.0),
            Point2::new(5.0, 5.0),
            Point2::new(5.0, 10.0),
            Point2::new(0.0, 10.0),
        ];
        m.edit_terrain(id, |t| t.set_outline(ell)).unwrap();
        let t = m.terrain(id).unwrap();
        assert!(t.height_at(Point2::new(2.0, 8.0)).is_some());
        assert!(
            t.height_at(Point2::new(8.0, 8.0)).is_none(),
            "the corner left out"
        );
        // The height along the edge follows the ground: at (5, 5), 0,5.
        let mid = t
            .points
            .iter()
            .find(|p| p.at.approx_eq(Point2::new(5.0, 5.0)))
            .unwrap();
        assert!((mid.z - 0.5).abs() < 1e-9);
        // Downhill is south, at 10 %.
        let marks = t.slope_marks(2.0);
        assert!(!marks.is_empty());
        for (_, dir, g) in marks {
            assert!(dir.y < -0.99, "{dir:?}");
            assert!((g - 0.1).abs() < 1e-9);
        }
    }
}
