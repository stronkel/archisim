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
    signed_area, Model, ModelError, Point2, RoofId, SlabId, SurveyPoint, TerrainId,
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
}

/// Why a sketch cannot be finished or an operation done.
#[derive(Debug, Clone, PartialEq)]
pub enum SketchError {
    /// A loop would have fewer than three vertices.
    TooFew,
    /// The model refused it (a crossing outline, a hole outside…).
    Model(ModelError),
    /// The sketch lines do not close: where they are left open.
    Open(Vec<Point2>),
    /// A roof or a terrain takes one contour only.
    OneLoop,
    /// No sketch lines at all.
    Empty,
}

impl std::fmt::Display for SketchError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SketchError::TooFew => write!(f, "un contorno necesita al menos tres vértices"),
            SketchError::Model(e) => write!(f, "{e}"),
            SketchError::Open(at) => write!(
                f,
                "las líneas del boceto no se cierran ({} extremos sueltos, marcados en rojo)",
                at.len()
            ),
            SketchError::OneLoop => {
                write!(f, "aquí el boceto tiene que ser un solo contorno cerrado")
            }
            SketchError::Empty => write!(f, "el boceto no tiene líneas"),
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
        })
    }

    /// Whether the sketch can have holes (a slab's).
    pub fn takes_holes(&self) -> bool {
        matches!(self.target, SketchTarget::Slab(_))
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

/// How close two ends have to be to be joined (metres).
pub const JOIN: f64 = 0.002;

/// The closed contours made by sketch lines (straight or arcs, as `Seg`):
/// each one as a polygon, the arcs in short chords. Lines whose ends meet
/// no other, or three at one point, are not a contour: their ends come
/// back as the error.
pub fn loops_from_segs(segs: &[archisim_draft::Seg]) -> Result<Vec<Vec<Point2>>, SketchError> {
    let tol = 0.005;
    let mut loops = Vec::new();
    let mut open: Vec<&archisim_draft::Seg> = Vec::new();
    for s in segs {
        if s.is_full_circle() {
            let mut pts = s.tessellate(tol);
            pts.pop();
            loops.push(pts);
        } else if s.length() > 1e-6 {
            open.push(s);
        }
    }
    // Ends: (segment, at its end?) → the one point they share.
    let ends: Vec<Point2> = open.iter().flat_map(|s| [s.start(), s.end()]).collect();
    let partner = |k: usize| -> Vec<usize> {
        (0..ends.len())
            .filter(|&j| j != k && j / 2 != k / 2 && ends[j].distance(ends[k]) < JOIN)
            .collect()
    };
    let mut bad = Vec::new();
    for k in 0..ends.len() {
        let n = partner(k).len();
        if n != 1 {
            bad.push(ends[k]);
        }
    }
    if !bad.is_empty() {
        return Err(SketchError::Open(bad));
    }
    let mut used = vec![false; open.len()];
    for first in 0..open.len() {
        if used[first] {
            continue;
        }
        let mut pts: Vec<Point2> = Vec::new();
        let mut i = first;
        let mut forward = true;
        loop {
            used[i] = true;
            let s = if forward {
                (*open[i]).clone()
            } else {
                open[i].reversed()
            };
            let mut t = s.tessellate(tol);
            if !pts.is_empty() {
                t.remove(0);
            }
            pts.extend(t);
            // The end it reaches, and the segment that starts there.
            let at = if forward { 2 * i + 1 } else { 2 * i };
            let next = partner(at)[0];
            let j = next / 2;
            if j == first {
                break;
            }
            if used[j] {
                return Err(SketchError::Open(vec![ends[at]]));
            }
            forward = next % 2 == 0;
            i = j;
        }
        pts.pop();
        if pts.len() >= 3 {
            loops.push(pts);
        }
    }
    if loops.is_empty() {
        return Err(SketchError::Empty);
    }
    Ok(loops)
}

impl Sketch {
    /// The same element with new contours from sketch lines: a slab's
    /// largest is its outline and the rest its holes; a roof keeps the
    /// slope of each edge that lies on one of its old edges (the new ones
    /// take the nearest's); a terrain keeps the points it had where a
    /// vertex still is, and the new ones take the height of the nearest.
    pub fn with_loops(&self, mut loops: Vec<Vec<Point2>>) -> Result<Sketch, SketchError> {
        loops.sort_by(|a, b| signed_area(b).abs().total_cmp(&signed_area(a).abs()));
        if !self.takes_holes() && loops.len() != 1 {
            return Err(SketchError::OneLoop);
        }
        let mut out = self.clone();
        let old = self.loops[0].clone();
        let n_old = old.len();
        if matches!(self.target, SketchTarget::Roof(_)) {
            let lp = &loops[0];
            let n = lp.len();
            out.slopes = (0..n)
                .map(|i| {
                    let (a, b) = (lp[i], lp[(i + 1) % n]);
                    let mid = a + (b - a) * 0.5;
                    let best = (0..n_old).min_by(|&x, &y| {
                        let d = |k: usize| {
                            let (p, q) = (old[k], old[(k + 1) % n_old]);
                            segment_distance(mid, p, q)
                                + segment_distance(a, p, q)
                                + segment_distance(b, p, q)
                        };
                        d(x).total_cmp(&d(y))
                    });
                    best.and_then(|k| self.slopes.get(k).copied().flatten())
                })
                .collect();
        }
        if matches!(self.target, SketchTarget::Terrain(_)) && !self.terrain.is_empty() {
            out.terrain = loops[0]
                .iter()
                .map(|p| {
                    let near = (0..n_old)
                        .min_by(|&x, &y| old[x].distance(*p).total_cmp(&old[y].distance(*p)));
                    match near {
                        Some(k) if old[k].distance(*p) < JOIN => self.terrain[k],
                        Some(k) => (None, self.terrain[k].1),
                        None => (None, 0.0),
                    }
                })
                .collect();
        }
        out.loops = loops;
        Ok(out)
    }

    /// The element whose contour it is.
    pub fn element(&self) -> crate::pick::Element {
        match self.target {
            SketchTarget::Slab(id) => crate::pick::Element::Slab(id),
            SketchTarget::Roof(id) => crate::pick::Element::Roof(id),
            SketchTarget::Terrain(id) => crate::pick::Element::Terrain(id),
        }
    }
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

    fn line(a: Point2, b: Point2) -> archisim_draft::Seg {
        archisim_draft::Seg::from_bulge(a, b, 0.0)
    }

    /// Loose lines in any order and direction close into contours; the
    /// largest is the slab's outline and the one inside it a hole; nothing
    /// changes until it is applied.
    #[test]
    fn sketch_lines_make_the_contours() {
        let (mut m, id) = slab_model();
        let s = Sketch::of(&m, SketchTarget::Slab(id)).unwrap();
        // An L outline (one edge drawn backwards, one an arc) and a square
        // hole.
        let segs = vec![
            line(p(0.0, 0.0), p(7.0, 0.0)),
            line(p(7.0, 3.0), p(7.0, 0.0)),
            archisim_draft::Seg::from_bulge(p(7.0, 3.0), p(4.0, 3.0), 0.2),
            line(p(4.0, 3.0), p(4.0, 5.0)),
            line(p(4.0, 5.0), p(0.0, 5.0)),
            line(p(0.0, 5.0), p(0.0, 0.0)),
            line(p(1.0, 1.0), p(2.0, 1.0)),
            line(p(2.0, 1.0), p(2.0, 2.0)),
            line(p(2.0, 2.0), p(1.0, 2.0)),
            line(p(1.0, 2.0), p(1.0, 1.0)),
        ];
        let loops = loops_from_segs(&segs).unwrap();
        assert_eq!(loops.len(), 2);
        let s = s.with_loops(loops).unwrap();
        assert!(s.loops[0].len() > 6, "the arc in chords");
        assert_eq!(s.loops[1].len(), 4);
        assert_eq!(m.slab(id).unwrap().outline.len(), 4);
        s.apply(&mut m).unwrap();
        let slab = m.slab(id).unwrap();
        assert!(slab.outline.len() > 6);
        assert_eq!(slab.holes.len(), 1);
        // An open line: its ends come back.
        let open = vec![
            line(p(0.0, 0.0), p(5.0, 0.0)),
            line(p(5.0, 0.0), p(5.0, 4.0)),
        ];
        match loops_from_segs(&open) {
            Err(SketchError::Open(at)) => assert_eq!(at.len(), 2),
            other => panic!("{other:?}"),
        }
        // A circle is a contour on its own.
        let circle = archisim_draft::shape_segs(&archisim_core::Shape::Circle {
            centre: p(3.0, 2.0),
            radius: 1.0,
        });
        assert_eq!(loops_from_segs(&circle).unwrap().len(), 1);
        // A hole outside is refused, and the slab stays as it was.
        let before = m.clone();
        let mut bad = Sketch::of(&m, SketchTarget::Slab(id)).unwrap();
        bad.loops
            .push(vec![p(20.0, 20.0), p(21.0, 20.0), p(21.0, 21.0)]);
        assert!(matches!(
            bad.apply(&mut m.clone()),
            Err(SketchError::Model(_))
        ));
        assert_eq!(m, before);
    }

    /// A roof keeps the slope of each edge on an old one; the new edges
    /// take the nearest's. It takes one contour only.
    #[test]
    fn a_roof_keeps_its_slopes() {
        let s = Sketch {
            target: SketchTarget::Roof(RoofId(1)),
            loops: vec![vec![p(0.0, 0.0), p(6.0, 0.0), p(6.0, 4.0), p(0.0, 4.0)]],
            slopes: vec![Some(0.5), None, Some(0.5), None],
            terrain: Vec::new(),
        };
        // The east edge split in two.
        let r = s
            .with_loops(vec![vec![
                p(0.0, 0.0),
                p(6.0, 0.0),
                p(6.0, 2.0),
                p(6.0, 4.0),
                p(0.0, 4.0),
            ]])
            .unwrap();
        assert_eq!(r.slopes, vec![Some(0.5), None, None, Some(0.5), None]);
        assert_eq!(
            s.with_loops(vec![s.loops[0].clone(), s.loops[0].clone()]),
            Err(SketchError::OneLoop)
        );
    }
}
