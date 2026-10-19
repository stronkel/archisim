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
}

/// Data needed to create a terrain.
#[derive(Debug, Clone, PartialEq)]
pub struct NewTerrain {
    pub level: LevelId,
    pub points: Vec<SurveyPoint>,
    pub material: MaterialId,
    pub depth: f64,
    pub contour: f64,
}

impl Terrain {
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

/// Delaunay triangulation (Bowyer–Watson) of points in plan; triangles
/// counter-clockwise, by index. Points repeated are ignored.
pub fn delaunay(points: &[Point2]) -> Vec<[usize; 3]> {
    let n = points.len();
    if n < 3 {
        return Vec::new();
    }
    let (mut lo, mut hi) = (points[0], points[0]);
    for p in points {
        lo = Point2::new(lo.x.min(p.x), lo.y.min(p.y));
        hi = Point2::new(hi.x.max(p.x), hi.y.max(p.y));
    }
    let size = (hi.x - lo.x).max(hi.y - lo.y).max(1.0) * 20.0;
    let mid = (lo + hi) * 0.5;
    let mut pts: Vec<Point2> = points.to_vec();
    pts.push(Point2::new(mid.x - size, mid.y - size));
    pts.push(Point2::new(mid.x + size, mid.y - size));
    pts.push(Point2::new(mid.x, mid.y + size));
    let mut tris: Vec<[usize; 3]> = vec![[n, n + 1, n + 2]];
    let in_circle = |t: &[usize; 3], p: Point2, pts: &[Point2]| {
        let (a, b, c) = (pts[t[0]] - p, pts[t[1]] - p, pts[t[2]] - p);
        let det = (a.x * a.x + a.y * a.y) * (b.x * c.y - c.x * b.y)
            - (b.x * b.x + b.y * b.y) * (a.x * c.y - c.x * a.y)
            + (c.x * c.x + c.y * c.y) * (a.x * b.y - b.x * a.y);
        det > 1e-12
    };
    for i in 0..n {
        let p = pts[i];
        if pts[..i].iter().any(|q| q.approx_eq(p)) {
            continue;
        }
        let (bad, good): (Vec<[usize; 3]>, Vec<[usize; 3]>) =
            tris.into_iter().partition(|t| in_circle(t, p, &pts));
        tris = good;
        // The hole's edges: those of one bad triangle only.
        let mut edges: Vec<(usize, usize)> = Vec::new();
        for t in &bad {
            for k in 0..3 {
                let e = (t[k], t[(k + 1) % 3]);
                if let Some(j) = edges.iter().position(|f| *f == (e.1, e.0)) {
                    edges.remove(j);
                } else {
                    edges.push(e);
                }
            }
        }
        for (a, b) in edges {
            tris.push([a, b, i]);
        }
    }
    tris.retain(|t| t.iter().all(|&k| k < n));
    // Counter-clockwise, and no slivers of zero area.
    tris.into_iter()
        .filter_map(|t| {
            let area = (pts[t[1]] - pts[t[0]]).cross(pts[t[2]] - pts[t[0]]);
            if area.abs() < 1e-12 {
                None
            } else if area > 0.0 {
                Some(t)
            } else {
                Some([t[0], t[2], t[1]])
            }
        })
        .collect()
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
}
