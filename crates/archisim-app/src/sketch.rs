//! The sketch of a slab's, a roof's or a terrain's contour, as Revit's
//! «Edit Boundary» (E4): the lines of its outline (and of a slab's holes)
//! are edited on their own — vertices moved, added and taken away, edges
//! moved parallel, the whole outline offset, an edge turned into an arc,
//! two edges trimmed or extended to meet — and nothing changes in the
//! model until the sketch is finished, in one step that can be undone;
//! cancelled, it leaves the element as it was.
//!
//! No interface here: `app/sketch_ui` draws it and takes the clicks.

use archisim_core::{
    signed_area, Model, ModelError, Point2, RoofId, SlabId, SurveyPoint, TerrainId, Vec2,
};

/// What is being sketched.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SketchTarget {
    Slab(SlabId),
    Roof(RoofId),
    Terrain(TerrainId),
}

/// A contour being edited.
#[derive(Debug, Clone, PartialEq)]
pub struct Sketch {
    pub target: SketchTarget,
    /// The outline first, then a slab's holes; each closed.
    pub loops: Vec<Vec<Point2>>,
    /// A roof's slope on each edge of its outline (`None`: a gable).
    slopes: Vec<Option<f64>>,
    /// A terrain's boundary: the point each vertex was (none: a new one)
    /// and its height.
    terrain: Vec<(Option<usize>, f64)>,
    /// The edge picked (loop, edge from vertex `i` to `i + 1`), for the
    /// operations on one edge.
    pub edge: Option<(usize, usize)>,
}

/// Why a sketch cannot be finished or an operation done.
#[derive(Debug, Clone, PartialEq)]
pub enum SketchError {
    /// A loop would have fewer than three vertices.
    TooFew,
    /// The offset or the corner does not give a contour.
    NoContour,
    /// The model refused it (a crossing outline, a hole outside…).
    Model(ModelError),
}

impl std::fmt::Display for SketchError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SketchError::TooFew => write!(f, "un contorno necesita al menos tres vértices"),
            SketchError::NoContour => write!(f, "así no queda un contorno"),
            SketchError::Model(e) => write!(f, "{e}"),
        }
    }
}

impl Sketch {
    /// The sketch of an element's contour as it is now.
    pub fn of(model: &Model, target: SketchTarget) -> Option<Sketch> {
        let (loops, slopes, terrain) = match target {
            SketchTarget::Slab(id) => {
                let s = model.slab(id)?;
                let mut loops = vec![s.outline.clone()];
                loops.extend(s.holes.iter().cloned());
                (loops, Vec::new(), Vec::new())
            }
            SketchTarget::Roof(id) => {
                let r = model.roof(id)?;
                (vec![r.outline.clone()], r.slopes.clone(), Vec::new())
            }
            SketchTarget::Terrain(id) => {
                let t = model.terrain(id)?;
                // An edge drawn by hand is edited as it is.
                if let Some(o) = t.outline.clone().filter(|o| o.len() >= 3) {
                    return Some(Sketch {
                        target,
                        loops: vec![o],
                        slopes: Vec::new(),
                        terrain: Vec::new(),
                        edge: None,
                    });
                }
                let ring = t.boundary();
                let outline = ring.iter().map(|&i| t.points[i].at).collect();
                let terrain = ring.iter().map(|&i| (Some(i), t.points[i].z)).collect();
                (vec![outline], Vec::new(), terrain)
            }
        };
        Some(Sketch {
            target,
            loops,
            slopes,
            terrain,
            edge: None,
        })
    }

    /// Whether the sketch can have holes (a slab's).
    pub fn takes_holes(&self) -> bool {
        matches!(self.target, SketchTarget::Slab(_))
    }

    /// The two ends of an edge.
    pub fn edge_ends(&self, l: usize, i: usize) -> Option<(Point2, Point2)> {
        let lp = self.loops.get(l)?;
        let n = lp.len();
        (i < n).then(|| (lp[i], lp[(i + 1) % n]))
    }

    /// Moves a vertex.
    pub fn move_vertex(&mut self, l: usize, i: usize, to: Point2) {
        if let Some(p) = self.loops.get_mut(l).and_then(|lp| lp.get_mut(i)) {
            *p = to;
        }
    }

    /// Moves an edge, both its ends, by `d` (Revit drags a sketch line
    /// parallel to itself).
    pub fn move_edge(&mut self, l: usize, i: usize, d: Vec2) {
        let Some(lp) = self.loops.get_mut(l) else {
            return;
        };
        let n = lp.len();
        if i >= n {
            return;
        }
        lp[i] = lp[i] + d;
        lp[(i + 1) % n] = lp[(i + 1) % n] + d;
    }

    /// Adds a vertex at `at` on the edge from vertex `i`; returns its
    /// index.
    pub fn insert_vertex(&mut self, l: usize, i: usize, at: Point2) -> Option<usize> {
        let n = self.loops.get(l)?.len();
        if i >= n {
            return None;
        }
        if l == 0 {
            // The new edge keeps its edge's slope; the new point of a
            // terrain, the height along its edge.
            if let Some(s) = self.slopes.get(i).copied() {
                self.slopes.insert(i + 1, s);
            }
            if !self.terrain.is_empty() {
                let (a, b) = self.edge_ends(l, i)?;
                let (za, zb) = (self.terrain[i].1, self.terrain[(i + 1) % n].1);
                let len = a.distance(b).max(1e-9);
                let f = (a.distance(at) / len).clamp(0.0, 1.0);
                self.terrain.insert(i + 1, (None, za + (zb - za) * f));
            }
        }
        self.loops[l].insert(i + 1, at);
        Some(i + 1)
    }

    /// Takes a vertex away (a loop keeps at least three).
    pub fn remove_vertex(&mut self, l: usize, i: usize) -> Result<(), SketchError> {
        let lp = self.loops.get_mut(l).ok_or(SketchError::NoContour)?;
        if lp.len() <= 3 {
            return Err(SketchError::TooFew);
        }
        lp.remove(i);
        if l == 0 {
            // The edges either side become one, with the first's slope.
            if i < self.slopes.len() {
                self.slopes.remove(i);
            }
            if i < self.terrain.len() {
                self.terrain.remove(i);
            }
        }
        self.edge = None;
        Ok(())
    }

    /// Offsets the outline `d` outwards (negative: inwards), its edges
    /// parallel and meeting at their corners.
    pub fn offset(&mut self, d: f64) -> Result<(), SketchError> {
        let lp = self.loops.first().ok_or(SketchError::NoContour)?;
        let out = offset_loop(lp, d).ok_or(SketchError::NoContour)?;
        self.loops[0] = out;
        Ok(())
    }

    /// Turns an edge into an arc through `bulge` of its length off its
    /// middle (positive: outwards), as `steps` straight pieces.
    pub fn arc_edge(&mut self, l: usize, i: usize, bulge: f64, steps: usize) -> Option<()> {
        let (a, b) = self.edge_ends(l, i)?;
        let chord = a.distance(b);
        let s = bulge * chord;
        if s.abs() < 1e-6 || chord < 1e-6 {
            return None;
        }
        // Outwards: to the right of the edge of a counter-clockwise outline.
        let ccw = signed_area(&self.loops[l]) > 0.0;
        let out = if ccw == (l == 0) { -1.0 } else { 1.0 };
        let s = s * out;
        // The circle through a, b and the point `s` off the middle.
        let r = (chord * chord / 4.0 + s * s) / (2.0 * s.abs());
        let u = (b - a) * (1.0 / chord);
        let left = Vec2::new(-u.y, u.x);
        let mid = a + (b - a) * 0.5;
        let centre = mid + left * (s.signum() * (s.abs() - r));
        let a0 = (a.y - centre.y).atan2(a.x - centre.x);
        let a1 = (b.y - centre.y).atan2(b.x - centre.x);
        let tau = std::f64::consts::TAU;
        // Round the side it bulges to: of the two ways round, the one
        // whose middle is on that side of the chord.
        let mut sweep = a1 - a0;
        while sweep > std::f64::consts::PI {
            sweep -= tau;
        }
        while sweep <= -std::f64::consts::PI {
            sweep += tau;
        }
        let side_of = |sw: f64| {
            let t = a0 + sw / 2.0;
            let q = centre + Vec2::new(t.cos(), t.sin()) * r;
            (b - a).cross(q - a)
        };
        if side_of(sweep).signum() != s.signum() {
            sweep -= sweep.signum() * tau;
        }
        let steps = steps.clamp(2, 64);
        for k in (1..steps).rev() {
            let t = a0 + sweep * k as f64 / steps as f64;
            let p = centre + Vec2::new(t.cos(), t.sin()) * r;
            self.insert_vertex(l, i, p)?;
        }
        self.edge = None;
        Some(())
    }

    /// Trims or extends the edges `i` and `j` of a loop so that they meet
    /// (Revit's «Trim/Extend to Corner»): the vertices between them go,
    /// and the corner is where their lines cross.
    pub fn corner(&mut self, l: usize, i: usize, j: usize) -> Result<(), SketchError> {
        let lp = self.loops.get(l).ok_or(SketchError::NoContour)?;
        let n = lp.len();
        if i == j || i >= n || j >= n {
            return Err(SketchError::NoContour);
        }
        let (a0, a1) = (lp[i], lp[(i + 1) % n]);
        let (b0, b1) = (lp[j], lp[(j + 1) % n]);
        let p = line_cross(a0, a1, b0, b1).ok_or(SketchError::NoContour)?;
        // Going round from edge i to edge j: the vertices after i up to j
        // become the one corner (the shorter way round).
        let forward = (j + n - i) % n;
        let (first, last) = if forward <= n / 2 { (i, j) } else { (j, i) };
        let between = (last + n - first) % n; // vertices first+1..=last
        if n - (between - 1) < 3 {
            return Err(SketchError::TooFew);
        }
        let mut idx: Vec<usize> = (1..=between).map(|k| (first + k) % n).collect();
        // Keep the first of them, now the corner; take the others away,
        // the highest index first.
        let keep = idx.remove(0);
        self.loops[l][keep] = p;
        idx.sort_unstable_by(|a, b| b.cmp(a));
        for k in idx {
            self.remove_vertex(l, k)?;
        }
        Ok(())
    }

    /// Adds a hole (of a slab): a rectangle `w` × `h` about `at`.
    pub fn add_hole(&mut self, at: Point2, w: f64, h: f64) -> bool {
        if !self.takes_holes() {
            return false;
        }
        let (hw, hh) = (w / 2.0, h / 2.0);
        self.loops.push(vec![
            Point2::new(at.x - hw, at.y - hh),
            Point2::new(at.x + hw, at.y - hh),
            Point2::new(at.x + hw, at.y + hh),
            Point2::new(at.x - hw, at.y + hh),
        ]);
        true
    }

    /// Takes a hole away.
    pub fn remove_hole(&mut self, l: usize) -> bool {
        if l == 0 || l >= self.loops.len() {
            return false;
        }
        self.loops.remove(l);
        self.edge = None;
        true
    }

    /// The vertex nearest `at` within `reach`: (loop, index).
    pub fn vertex_near(&self, at: Point2, reach: f64) -> Option<(usize, usize)> {
        let mut best: Option<(f64, usize, usize)> = None;
        for (l, lp) in self.loops.iter().enumerate() {
            for (i, p) in lp.iter().enumerate() {
                let d = p.distance(at);
                if d <= reach && best.is_none_or(|b| d < b.0) {
                    best = Some((d, l, i));
                }
            }
        }
        best.map(|(_, l, i)| (l, i))
    }

    /// The edge nearest `at` within `reach`: (loop, edge).
    pub fn edge_near(&self, at: Point2, reach: f64) -> Option<(usize, usize)> {
        let mut best: Option<(f64, usize, usize)> = None;
        for (l, lp) in self.loops.iter().enumerate() {
            let n = lp.len();
            for i in 0..n {
                let d = segment_distance(at, lp[i], lp[(i + 1) % n]);
                if d <= reach && best.is_none_or(|b| d < b.0) {
                    best = Some((d, l, i));
                }
            }
        }
        best.map(|(_, l, i)| (l, i))
    }

    /// Puts the sketch into the model: the element's contour as sketched.
    /// The model checks it (a crossing outline, a hole outside the slab).
    pub fn apply(&self, m: &mut Model) -> Result<(), SketchError> {
        if self.loops.iter().any(|l| l.len() < 3) {
            return Err(SketchError::TooFew);
        }
        let outline = self.loops[0].clone();
        match self.target {
            SketchTarget::Slab(id) => {
                m.set_slab_holes(id, Vec::new())
                    .map_err(SketchError::Model)?;
                m.set_slab_outline(id, outline)
                    .map_err(SketchError::Model)?;
                m.set_slab_holes(id, self.loops[1..].to_vec())
                    .map_err(SketchError::Model)?;
            }
            SketchTarget::Roof(id) => {
                let mut slopes = self.slopes.clone();
                slopes.resize(outline.len(), slopes.last().copied().flatten());
                // The model keeps outlines counter-clockwise; the slopes go
                // with their edges.
                let (outline, slopes) = if signed_area(&outline) < 0.0 {
                    let n = outline.len();
                    let rev: Vec<Point2> = outline.iter().rev().copied().collect();
                    // Edge k of the reversed outline is edge n-2-k of the
                    // original (taken the other way).
                    let s = (0..n).map(|k| slopes[(2 * n - 2 - k) % n]).collect();
                    (rev, s)
                } else {
                    (outline, slopes)
                };
                m.edit_roof(id, |r| {
                    r.outline = outline;
                    r.slopes = slopes;
                })
                .map_err(SketchError::Model)?;
            }
            SketchTarget::Terrain(id) if self.terrain.is_empty() => {
                // An edge drawn by hand (E5): the terrain takes the new one.
                m.edit_terrain(id, |t| t.set_outline(outline))
                    .map_err(SketchError::Model)?;
            }
            SketchTarget::Terrain(id) => {
                let t = m
                    .terrain(id)
                    .ok_or(SketchError::Model(ModelError::UnknownTerrain(id)))?;
                let mut points = t.points.clone();
                let mut lines = t.lines.clone();
                let kept: Vec<usize> = self.terrain.iter().filter_map(|v| v.0).collect();
                // The boundary points taken away, the highest first.
                let mut gone: Vec<usize> = t
                    .boundary()
                    .into_iter()
                    .filter(|i| !kept.contains(i))
                    .collect();
                // Those kept, moved; the new ones, added.
                for (p, (old, z)) in outline.iter().zip(&self.terrain) {
                    match old {
                        Some(i) => points[*i] = SurveyPoint { at: *p, z: *z },
                        None => points.push(SurveyPoint { at: *p, z: *z }),
                    }
                }
                gone.sort_unstable_by(|a, b| b.cmp(a));
                for g in gone {
                    points.remove(g);
                    lines.retain(|l| !l.contains(&g));
                    for l in &mut lines {
                        for i in l.iter_mut() {
                            if *i > g {
                                *i -= 1;
                            }
                        }
                    }
                }
                m.edit_terrain(id, |t| {
                    t.points = points;
                    t.lines = lines;
                })
                .map_err(SketchError::Model)?;
            }
        }
        Ok(())
    }
}

/// A closed loop moved `d` outwards (negative: inwards), its edges
/// parallel, meeting where their lines cross.
pub fn offset_loop(lp: &[Point2], d: f64) -> Option<Vec<Point2>> {
    let n = lp.len();
    if n < 3 {
        return None;
    }
    // Outwards is the right of each edge of a counter-clockwise loop.
    let s = if signed_area(lp) > 0.0 { -d } else { d };
    let line = |i: usize| -> Option<(Point2, Point2)> {
        let (a, b) = (lp[i], lp[(i + 1) % n]);
        let u = (b - a).normalized()?;
        let off = Vec2::new(-u.y, u.x) * s;
        Some((a + off, b + off))
    };
    let mut out = Vec::with_capacity(n);
    for i in 0..n {
        let (p0, p1) = line((i + n - 1) % n)?;
        let (q0, q1) = line(i)?;
        // Edges in line: the moved vertex itself.
        let p = line_cross(p0, p1, q0, q1).unwrap_or(q0);
        out.push(p);
    }
    (signed_area(&out).abs() > 1e-9 && signed_area(&out).signum() == signed_area(lp).signum())
        .then_some(out)
}

/// Where the lines through a0–a1 and b0–b1 cross.
fn line_cross(a0: Point2, a1: Point2, b0: Point2, b1: Point2) -> Option<Point2> {
    let (r, s) = (a1 - a0, b1 - b0);
    let den = r.cross(s);
    if den.abs() < 1e-12 {
        return None;
    }
    let t = (b0 - a0).cross(s) / den;
    Some(a0 + r * t)
}

fn segment_distance(p: Point2, a: Point2, b: Point2) -> f64 {
    let ab = b - a;
    let len2 = ab.dot(ab);
    if len2 < 1e-18 {
        return p.distance(a);
    }
    let t = ((p - a).dot(ab) / len2).clamp(0.0, 1.0);
    p.distance(a + ab * t)
}

#[cfg(test)]
mod tests {
    use super::*;
    use archisim_core::{CutPattern, NewSlab};

    fn p(x: f64, y: f64) -> Point2 {
        Point2::new(x, y)
    }

    fn slab_model() -> (Model, SlabId) {
        let mut m = Model::new("t");
        let l = m.add_level("PB", 0.0);
        let mat = m.add_material("HA", "concrete", [200, 200, 200], CutPattern::Solid);
        let t = m
            .add_slab_type(
                "Losa",
                archisim_core::SlabFunction::Floor,
                vec![archisim_core::Layer {
                    material: mat,
                    thickness: 0.25,
                    function: archisim_core::LayerFunction::Structure,
                }],
            )
            .unwrap();
        let id = m
            .add_slab(NewSlab {
                slab_type: t,
                level: l,
                outline: vec![p(0.0, 0.0), p(6.0, 0.0), p(6.0, 4.0), p(0.0, 4.0)],
                top_offset: 0.0,
            })
            .unwrap();
        (m, id)
    }

    /// Nothing changes until it is applied; then the slab takes the
    /// sketched outline and holes.
    #[test]
    fn a_slab_sketch_is_applied_whole() {
        let (mut m, id) = slab_model();
        let mut s = Sketch::of(&m, SketchTarget::Slab(id)).unwrap();
        s.move_edge(0, 1, Vec2::new(1.0, 0.0));
        let k = s.insert_vertex(0, 2, p(3.5, 4.0)).unwrap();
        s.move_vertex(0, k, p(3.5, 5.0));
        assert!(s.add_hole(p(2.0, 2.0), 1.0, 1.0));
        assert_eq!(m.slab(id).unwrap().outline.len(), 4);
        s.apply(&mut m).unwrap();
        let slab = m.slab(id).unwrap();
        assert_eq!(slab.outline.len(), 5);
        assert!(slab.outline.contains(&p(7.0, 4.0)));
        assert_eq!(slab.holes.len(), 1);
        // A hole outside it is refused, and the slab stays as it was.
        let before = m.clone();
        let mut bad = Sketch::of(&m, SketchTarget::Slab(id)).unwrap();
        bad.add_hole(p(20.0, 20.0), 1.0, 1.0);
        assert!(matches!(
            bad.apply(&mut m.clone()),
            Err(SketchError::Model(_))
        ));
        assert_eq!(m, before);
        // At least three vertices.
        let mut tri = Sketch::of(&m, SketchTarget::Slab(id)).unwrap();
        tri.loops[0].truncate(3);
        assert_eq!(tri.remove_vertex(0, 0), Err(SketchError::TooFew));
    }

    /// Offset, arc and corner.
    #[test]
    fn offset_arc_and_corner() {
        let (m, id) = slab_model();
        let mut s = Sketch::of(&m, SketchTarget::Slab(id)).unwrap();
        s.offset(0.5).unwrap();
        let a = signed_area(&s.loops[0]).abs();
        assert!((a - 7.0 * 5.0).abs() < 1e-9, "{a}");
        s.offset(-0.5).unwrap();
        assert!((signed_area(&s.loops[0]).abs() - 24.0).abs() < 1e-9);
        // The south edge as an arc bulging out by a quarter of 6 m.
        s.arc_edge(0, 0, 0.25, 8).unwrap();
        assert_eq!(s.loops[0].len(), 4 + 7);
        let low = s.loops[0].iter().map(|q| q.y).fold(f64::MAX, f64::min);
        assert!((low + 1.5).abs() < 1e-9, "{low}");
        // A corner cut off, then trimmed back: the edges either side of it
        // meet again at the old corner.
        let mut c = Sketch::of(&m, SketchTarget::Slab(id)).unwrap();
        c.insert_vertex(0, 1, p(6.0, 3.0)).unwrap();
        c.move_vertex(0, 2, p(6.0, 3.0));
        c.insert_vertex(0, 2, p(5.0, 4.0)).unwrap();
        // 0,0 · 6,0 · 6,3 · 5,4 · 6,4 · 0,4 → make it 6,3 · 5,4 a chamfer.
        c.loops[0].remove(4);
        assert_eq!(c.loops[0].len(), 5);
        c.corner(0, 1, 3).unwrap();
        assert_eq!(c.loops[0].len(), 4);
        assert!(c.loops[0].contains(&p(6.0, 4.0)));
    }

    /// A roof keeps a slope on each edge: a new vertex splits one, and
    /// taking one away joins two.
    #[test]
    fn a_roof_keeps_its_slopes() {
        let mut s = Sketch {
            target: SketchTarget::Roof(RoofId(1)),
            loops: vec![vec![p(0.0, 0.0), p(6.0, 0.0), p(6.0, 4.0), p(0.0, 4.0)]],
            slopes: vec![Some(0.5), None, Some(0.5), None],
            terrain: Vec::new(),
            edge: None,
        };
        s.insert_vertex(0, 1, p(6.0, 2.0)).unwrap();
        assert_eq!(s.slopes, vec![Some(0.5), None, None, Some(0.5), None]);
        s.remove_vertex(0, 0).unwrap();
        assert_eq!(s.slopes.len(), 4);
        assert!(!s.add_hole(p(1.0, 1.0), 0.5, 0.5));
    }
}
