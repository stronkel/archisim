//! Placements, curves, profiles and bodies of an IFC file, evaluated in
//! world coordinates (metres) as far as the importer needs them: points and
//! planar polygons, not exact solids.

use std::f64::consts::TAU;

use crate::step_read::{Entity, StepFile, Value};

pub type P3 = [f64; 3];

pub fn sub(a: P3, b: P3) -> P3 {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

pub fn dot(a: P3, b: P3) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

pub fn cross(a: P3, b: P3) -> P3 {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

pub fn scale(a: P3, k: f64) -> P3 {
    [a[0] * k, a[1] * k, a[2] * k]
}

pub fn normalize(a: P3) -> Option<P3> {
    let l = dot(a, a).sqrt();
    (l > 1e-12 && l.is_finite()).then(|| scale(a, 1.0 / l))
}

/// A rigid transform (with an optional uniform scale folded into the axes):
/// `p ↦ origin + x·px + y·py + z·pz`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Xf {
    pub x: P3,
    pub y: P3,
    pub z: P3,
    pub origin: P3,
}

impl Xf {
    pub const IDENTITY: Xf = Xf {
        x: [1.0, 0.0, 0.0],
        y: [0.0, 1.0, 0.0],
        z: [0.0, 0.0, 1.0],
        origin: [0.0, 0.0, 0.0],
    };

    pub fn apply(&self, p: P3) -> P3 {
        let v = self.vector(p);
        [
            self.origin[0] + v[0],
            self.origin[1] + v[1],
            self.origin[2] + v[2],
        ]
    }

    pub fn vector(&self, v: P3) -> P3 {
        [
            self.x[0] * v[0] + self.y[0] * v[1] + self.z[0] * v[2],
            self.x[1] * v[0] + self.y[1] * v[1] + self.z[1] * v[2],
            self.x[2] * v[0] + self.y[2] * v[1] + self.z[2] * v[2],
        ]
    }

    /// `self ∘ inner`: first `inner`, then `self`.
    pub fn then(&self, inner: &Xf) -> Xf {
        Xf {
            x: self.vector(inner.x),
            y: self.vector(inner.y),
            z: self.vector(inner.z),
            origin: self.apply(inner.origin),
        }
    }
}

/// Reads geometry from a parsed file with a length unit.
pub struct Geo<'a> {
    pub file: &'a StepFile,
    /// Metres per file length unit.
    pub length: f64,
    /// Radians per file plane-angle unit.
    pub angle: f64,
}

/// A planar polygon of a body, in world coordinates.
#[derive(Debug, Clone, PartialEq)]
pub struct Polygon {
    pub points: Vec<P3>,
}

impl Polygon {
    /// Area vector (normal × area) by Newell's method.
    pub fn area_vector(&self) -> P3 {
        let mut n = [0.0; 3];
        let k = self.points.len();
        for i in 0..k {
            let a = self.points[i];
            let b = self.points[(i + 1) % k];
            n[0] += (a[1] - b[1]) * (a[2] + b[2]);
            n[1] += (a[2] - b[2]) * (a[0] + b[0]);
            n[2] += (a[0] - b[0]) * (a[1] + b[1]);
        }
        scale(n, 0.5)
    }
}

/// What a body is made of, as far as the importer is concerned.
#[derive(Debug, Clone, Default)]
pub struct Body {
    /// Faces (or profile outlines at both ends of extrusions).
    pub polygons: Vec<Polygon>,
    /// Loose points (triangulated meshes).
    pub points: Vec<P3>,
    /// Vertical extrusions found: the outline in plan (world XY), the
    /// bottom and the top height. Used for slabs.
    pub extrusions: Vec<Extrusion>,
    /// Representation items that could not be read, by entity name.
    pub unsupported: Vec<String>,
    /// Part of the body was cut away (boolean clipping), so its outline
    /// and height are only an envelope.
    pub clipped: bool,
}

/// A closed loop of plan points.
pub type Ring = Vec<[f64; 2]>;

/// A piece of a wall axis: a straight piece from `a` to `b`, or an arc
/// through `mid`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AxisSeg {
    pub a: P3,
    pub mid: Option<P3>,
    pub b: P3,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Extrusion {
    pub outline: Vec<[f64; 2]>,
    /// The voids of the profile, in plan.
    pub holes: Vec<Vec<[f64; 2]>>,
    pub bottom: f64,
    pub top: f64,
}

impl Body {
    pub fn all_points(&self) -> impl Iterator<Item = P3> + '_ {
        self.polygons
            .iter()
            .flat_map(|p| p.points.iter().copied())
            .chain(self.points.iter().copied())
    }

    pub fn is_empty(&self) -> bool {
        self.polygons.is_empty() && self.points.is_empty()
    }

    /// Lowest and highest corner.
    pub fn bounds(&self) -> Option<(P3, P3)> {
        let mut lo = [f64::INFINITY; 3];
        let mut hi = [f64::NEG_INFINITY; 3];
        let mut any = false;
        for p in self.all_points() {
            any = true;
            for i in 0..3 {
                lo[i] = lo[i].min(p[i]);
                hi[i] = hi[i].max(p[i]);
            }
        }
        any.then_some((lo, hi))
    }
}

/// The outline of the top of a body: the boundary of its horizontal faces
/// looking up at its highest level (they may be split in triangles). The
/// largest loop and the other loops (holes).
pub fn top_outline(body: &Body, top: f64) -> Option<(Ring, Vec<Ring>)> {
    plan_outline(body, &|poly| {
        let n = poly.area_vector();
        let len = dot(n, n).sqrt();
        len >= 1e-9 && n[2] / len >= 0.9999 && poly.points.iter().all(|q| (q[2] - top).abs() < 1e-4)
    })
}

/// The top of an inclined body (a sloping slab or ramp): its outline in
/// plan (largest loop and holes), the plane of its top face as
/// `z = a·x + b·y + c`, and the vertical height down to its bottom face.
#[derive(Debug, Clone, PartialEq)]
pub struct SlopedTop {
    pub outline: Ring,
    pub holes: Vec<Ring>,
    pub plane: [f64; 3],
    pub vertical: f64,
}

/// [`SlopedTop`] of a body whose largest faces lie on two parallel
/// inclined planes (not level, not steeper than 80°): the top face on the
/// higher one.
pub fn sloped_top(body: &Body) -> Option<SlopedTop> {
    // The plane of each face, z = a·x + b·y + c, with its area in plan
    // (whichever way the face looks: the caps of an extrusion may both
    // look up).
    let plane_of = |poly: &Polygon| -> Option<([f64; 3], f64)> {
        let n = poly.area_vector();
        let len = dot(n, n).sqrt();
        if len < 1e-9 || (n[2] / len).abs() < 0.17 {
            return None;
        }
        let p = *poly.points.first()?;
        let (a, b) = (-n[0] / n[2], -n[1] / n[2]);
        Some(([a, b, p[2] - a * p[0] - b * p[1]], n[2].abs()))
    };
    let q = |v: f64| (v * 1e4).round() as i64;
    // Area by slope, then the planes of the largest slope.
    let mut by_slope: std::collections::HashMap<(i64, i64), f64> = Default::default();
    let planes: Vec<([f64; 3], f64)> = body.polygons.iter().filter_map(plane_of).collect();
    for (pl, area) in &planes {
        *by_slope.entry((q(pl[0]), q(pl[1]))).or_default() += area;
    }
    let (&slope, _) = by_slope.iter().max_by(|x, y| x.1.total_cmp(y.1))?;
    if slope.0.abs() + slope.1.abs() == 0 {
        return None;
    }
    let mut cs: Vec<[f64; 3]> = planes
        .iter()
        .filter(|(pl, _)| (q(pl[0]), q(pl[1])) == slope)
        .map(|(pl, _)| *pl)
        .collect();
    cs.sort_by(|x, y| x[2].total_cmp(&y[2]));
    let (bottom, top) = (*cs.first()?, *cs.last()?);
    if top[2] - bottom[2] < 1e-4 {
        return None;
    }
    let key = (q(top[0]), q(top[1]), q(top[2]));
    let (outline, holes) = plan_outline(body, &|poly| {
        plane_of(poly).is_some_and(|(pl, _)| (q(pl[0]), q(pl[1]), q(pl[2])) == key)
    })?;
    Some(SlopedTop {
        outline,
        holes,
        plane: top,
        vertical: top[2] - bottom[2],
    })
}

/// The outline in plan of the faces `keep` picks (they may be split in
/// triangles): the largest loop and the others (holes).
fn plan_outline(body: &Body, keep: &dyn Fn(&Polygon) -> bool) -> Option<(Ring, Vec<Ring>)> {
    use std::collections::HashMap;
    type Key = (i64, i64);
    let key = |p: P3| ((p[0] * 1e6).round() as i64, (p[1] * 1e6).round() as i64);
    let mut raw: Vec<(Key, Key)> = Vec::new();
    let mut coords: HashMap<(i64, i64), [f64; 2]> = HashMap::new();
    for poly in &body.polygons {
        if !keep(poly) {
            continue;
        }
        let m = poly.points.len();
        for i in 0..m {
            let (a, b) = (key(poly.points[i]), key(poly.points[(i + 1) % m]));
            if a == b {
                continue;
            }
            coords.insert(a, [poly.points[i][0], poly.points[i][1]]);
            raw.push((a, b));
        }
    }
    // Split edges at vertices lying on them (faces meeting in a T), so
    // shared edges match.
    let verts: Vec<(i64, i64)> = coords.keys().copied().collect();
    let mut edges: HashMap<(Key, Key), usize> = HashMap::new();
    for (a, b) in raw {
        let (pa, pb) = (coords[&a], coords[&b]);
        let d = [pb[0] - pa[0], pb[1] - pa[1]];
        let l2 = d[0] * d[0] + d[1] * d[1];
        let mut cuts: Vec<(f64, (i64, i64))> = verts
            .iter()
            .filter(|&&v| v != a && v != b)
            .filter_map(|&v| {
                let p = coords[&v];
                let w = [p[0] - pa[0], p[1] - pa[1]];
                let t = (w[0] * d[0] + w[1] * d[1]) / l2;
                let off = (w[0] * d[1] - w[1] * d[0]).abs() / l2.sqrt();
                (t > 1e-9 && t < 1.0 - 1e-9 && off < 1e-6).then_some((t, v))
            })
            .collect();
        cuts.sort_by(|x, y| x.0.total_cmp(&y.0));
        let mut prev = a;
        for v in cuts.into_iter().map(|c| c.1).chain([b]) {
            *edges
                .entry(if prev < v { (prev, v) } else { (v, prev) })
                .or_default() += 1;
            prev = v;
        }
    }
    // Edges used once are on the boundary; walk them into loops.
    let mut next: HashMap<(i64, i64), Vec<(i64, i64)>> = HashMap::new();
    for (&(a, b), &count) in &edges {
        if count == 1 {
            next.entry(a).or_default().push(b);
            next.entry(b).or_default().push(a);
        }
    }
    let mut loops: Vec<Vec<[f64; 2]>> = Vec::new();
    let mut starts: Vec<(i64, i64)> = next.keys().copied().collect();
    starts.sort_unstable();
    for start in starts {
        if next.get(&start).is_none_or(|v| v.is_empty()) {
            continue;
        }
        let mut lp = vec![coords[&start]];
        let mut prev = start;
        let mut cur = next.get_mut(&start)?.pop()?;
        next.get_mut(&cur)?.retain(|&x| x != prev);
        while cur != start {
            lp.push(coords[&cur]);
            let Some(n) = next.get_mut(&cur).and_then(|v| v.pop()) else {
                break;
            };
            next.get_mut(&n)?.retain(|&x| x != cur);
            prev = cur;
            cur = n;
            if lp.len() > 100_000 {
                break;
            }
        }
        let _ = prev;
        if lp.len() >= 3 {
            loops.push(lp);
        }
    }
    let area = |l: &Vec<[f64; 2]>| {
        let m = l.len();
        (0..m)
            .map(|i| l[i][0] * l[(i + 1) % m][1] - l[(i + 1) % m][0] * l[i][1])
            .sum::<f64>()
            .abs()
            / 2.0
    };
    let best = (0..loops.len()).max_by(|&a, &b| area(&loops[a]).total_cmp(&area(&loops[b])))?;
    let outer = loops.swap_remove(best);
    Some((outer, loops))
}

impl<'a> Geo<'a> {
    fn ent(&self, v: &Value) -> Option<&'a Entity> {
        self.file.deref(v)
    }

    /// An `IfcCartesianPoint`, scaled to metres (2D points get z = 0).
    pub fn point(&self, v: &Value) -> Option<P3> {
        let e = self.ent(v)?;
        if e.kind != "IFCCARTESIANPOINT" {
            return None;
        }
        let c = e.arg(0).numbers();
        let k = self.length;
        match c.len() {
            2 => Some([c[0] * k, c[1] * k, 0.0]),
            3 => Some([c[0] * k, c[1] * k, c[2] * k]),
            _ => None,
        }
    }

    /// An `IfcDirection` (unitless, normalised).
    pub fn direction(&self, v: &Value) -> Option<P3> {
        let e = self.ent(v)?;
        if e.kind != "IFCDIRECTION" {
            return None;
        }
        let c = e.arg(0).numbers();
        let d = match c.len() {
            2 => [c[0], c[1], 0.0],
            3 => [c[0], c[1], c[2]],
            _ => return None,
        };
        normalize(d)
    }

    /// `IfcAxis2Placement3D` or `IfcAxis2Placement2D` as a transform.
    pub fn axis_placement(&self, v: &Value) -> Xf {
        let Some(e) = self.ent(v) else {
            return Xf::IDENTITY;
        };
        let origin = self.point(e.arg(0)).unwrap_or([0.0; 3]);
        match e.kind.as_str() {
            "IFCAXIS2PLACEMENT3D" => {
                let z = self.direction(e.arg(1)).unwrap_or([0.0, 0.0, 1.0]);
                let x_hint = self.direction(e.arg(2)).unwrap_or_else(|| {
                    if z[0].abs() < 0.9 {
                        [1.0, 0.0, 0.0]
                    } else {
                        [0.0, 1.0, 0.0]
                    }
                });
                // Make X perpendicular to Z.
                let x = normalize(sub(x_hint, scale(z, dot(x_hint, z)))).unwrap_or([1.0, 0.0, 0.0]);
                let y = cross(z, x);
                Xf { x, y, z, origin }
            }
            "IFCAXIS2PLACEMENT2D" => {
                let x = self.direction(e.arg(1)).unwrap_or([1.0, 0.0, 0.0]);
                let x = normalize([x[0], x[1], 0.0]).unwrap_or([1.0, 0.0, 0.0]);
                Xf {
                    x,
                    y: [-x[1], x[0], 0.0],
                    z: [0.0, 0.0, 1.0],
                    origin,
                }
            }
            _ => Xf::IDENTITY,
        }
    }

    /// World transform of an `IfcObjectPlacement` (chains of
    /// `IfcLocalPlacement`; anything else counts as the origin).
    pub fn placement(&self, v: &Value) -> Xf {
        let mut chain = Vec::new();
        let mut cur = self.ent(v);
        while let Some(e) = cur {
            if e.kind != "IFCLOCALPLACEMENT" || chain.len() > 64 {
                break;
            }
            chain.push(self.axis_placement(e.arg(1)));
            cur = self.ent(e.arg(0));
        }
        chain
            .iter()
            .rev()
            .fold(Xf::IDENTITY, |acc, local| acc.then(local))
    }

    /// `IfcCartesianTransformationOperator3D/2D` (mapping targets).
    fn operator(&self, v: &Value) -> Xf {
        let Some(e) = self.ent(v) else {
            return Xf::IDENTITY;
        };
        let origin = self.point(e.arg(2)).unwrap_or([0.0; 3]);
        let s = e.arg(3).as_f64().unwrap_or(1.0);
        let x = self.direction(e.arg(0)).unwrap_or([1.0, 0.0, 0.0]);
        let (y, z) = if e.kind.starts_with("IFCCARTESIANTRANSFORMATIONOPERATOR3D") {
            let z = self.direction(e.arg(4)).unwrap_or_else(|| {
                let y = self.direction(e.arg(1)).unwrap_or([0.0, 1.0, 0.0]);
                normalize(cross(x, y)).unwrap_or([0.0, 0.0, 1.0])
            });
            let y = self.direction(e.arg(1)).unwrap_or_else(|| cross(z, x));
            (y, z)
        } else {
            let y = self.direction(e.arg(1)).unwrap_or([-x[1], x[0], 0.0]);
            (y, [0.0, 0.0, 1.0])
        };
        Xf {
            x: scale(x, s),
            y: scale(y, s),
            z: scale(z, s),
            origin,
        }
    }

    /// The pieces of a wall axis in its own coordinates: straight pieces
    /// and circular arcs (with a point of each arc between its ends).
    /// Polylines, indexed poly curves, trimmed lines and circles, whole
    /// circles (as two halves) and composite curves of those. `None` for
    /// other curves (ellipses, splines…).
    pub fn axis_segments(&self, v: &Value) -> Option<Vec<AxisSeg>> {
        self.axis_segments_depth(v, 0)
    }

    fn axis_segments_depth(&self, v: &Value, depth: usize) -> Option<Vec<AxisSeg>> {
        if depth > 16 {
            return None;
        }
        let e = self.ent(v)?;
        let lines = |pts: &[P3]| -> Vec<AxisSeg> {
            pts.windows(2)
                .filter(|w| !same(w[0], w[1]))
                .map(|w| AxisSeg {
                    a: w[0],
                    mid: None,
                    b: w[1],
                })
                .collect()
        };
        // Arcs from points sampled along one circle (32 to a turn): split
        // in two when it sweeps more than about 300° (a closed circle).
        let arcs = |pts: &[P3]| -> Vec<AxisSeg> {
            let n = pts.len();
            if n < 3 {
                return lines(pts);
            }
            let long = n > 28 || same(pts[0], pts[n - 1]);
            let parts: Vec<(usize, usize)> = if long {
                vec![(0, (n - 1) / 2), ((n - 1) / 2, n - 1)]
            } else {
                vec![(0, n - 1)]
            };
            parts
                .into_iter()
                .filter(|(i, j)| j > i)
                .map(|(i, j)| AxisSeg {
                    a: pts[i],
                    mid: Some(pts[(i + j) / 2]),
                    b: pts[j],
                })
                .collect()
        };
        Some(match e.kind.as_str() {
            "IFCPOLYLINE" => {
                let pts: Vec<P3> = e
                    .arg(0)
                    .as_list()
                    .iter()
                    .filter_map(|p| self.point(p))
                    .collect();
                lines(&pts)
            }
            "IFCINDEXEDPOLYCURVE" => {
                let list = self.ent(e.arg(0))?;
                let k = self.length;
                let coords: Vec<P3> = list
                    .arg(0)
                    .as_list()
                    .iter()
                    .map(|c| {
                        let n = c.numbers();
                        [
                            n.first().copied().unwrap_or(0.0) * k,
                            n.get(1).copied().unwrap_or(0.0) * k,
                            n.get(2).copied().unwrap_or(0.0) * k,
                        ]
                    })
                    .collect();
                let segments = e.arg(1).as_list();
                if segments.is_empty() {
                    return Some(lines(&coords));
                }
                let at = |i: f64| {
                    let i = i as usize;
                    (i >= 1 && i <= coords.len()).then(|| coords[i - 1])
                };
                let mut out = Vec::new();
                for seg in segments {
                    let Value::Typed(kind, inner) = seg else {
                        return None;
                    };
                    let idx: Vec<P3> = inner.numbers().into_iter().filter_map(at).collect();
                    match kind.as_str() {
                        "IFCLINEINDEX" => out.extend(lines(&idx)),
                        "IFCARCINDEX" if idx.len() == 3 => out.push(AxisSeg {
                            a: idx[0],
                            mid: Some(idx[1]),
                            b: idx[2],
                        }),
                        _ => return None,
                    }
                }
                out
            }
            "IFCCOMPOSITECURVE" | "IFCCOMPOSITECURVEONSURFACE" => {
                let mut out = Vec::new();
                for seg in e.arg(0).as_list() {
                    let s = self.ent(seg)?;
                    let same_sense = s.arg(1).as_bool().unwrap_or(true);
                    let mut part = self.axis_segments_depth(s.arg(2), depth + 1)?;
                    if !same_sense {
                        part.reverse();
                        for p in &mut part {
                            std::mem::swap(&mut p.a, &mut p.b);
                        }
                    }
                    out.extend(part);
                }
                out
            }
            "IFCTRIMMEDCURVE" => {
                let basis = self.ent(e.arg(0))?;
                let mut curved = false;
                let pts = self.trimmed(e, &mut curved)?;
                match basis.kind.as_str() {
                    "IFCLINE" => lines(&[pts[0], pts[pts.len() - 1]]),
                    "IFCCIRCLE" => arcs(&pts),
                    _ => return None,
                }
            }
            "IFCCIRCLE" => {
                let mut curved = false;
                let mut pts = self.curve(v, &mut curved)?;
                pts.push(pts[0]);
                arcs(&pts)
            }
            _ => return None,
        })
    }

    /// Points of a curve in its own coordinates (polylines, indexed poly
    /// curves, composite curves, trimmed lines and arcs, circles). Arcs are
    /// approximated by short segments. `curved` is set when the curve has
    /// arcs.
    pub fn curve(&self, v: &Value, curved: &mut bool) -> Option<Vec<P3>> {
        self.curve_depth(v, curved, 0)
    }

    fn curve_depth(&self, v: &Value, curved: &mut bool, depth: usize) -> Option<Vec<P3>> {
        if depth > 16 {
            return None;
        }
        let e = self.ent(v)?;
        match e.kind.as_str() {
            "IFCPOLYLINE" => {
                let pts: Vec<P3> = e
                    .arg(0)
                    .as_list()
                    .iter()
                    .filter_map(|p| self.point(p))
                    .collect();
                (pts.len() >= 2).then_some(pts)
            }
            "IFCINDEXEDPOLYCURVE" => {
                let list = self.ent(e.arg(0))?;
                let coords: Vec<P3> = list
                    .arg(0)
                    .as_list()
                    .iter()
                    .map(|c| {
                        let n = c.numbers();
                        let k = self.length;
                        [
                            n.first().copied().unwrap_or(0.0) * k,
                            n.get(1).copied().unwrap_or(0.0) * k,
                            n.get(2).copied().unwrap_or(0.0) * k,
                        ]
                    })
                    .collect();
                let segments = e.arg(1).as_list();
                if segments.is_empty() {
                    return (coords.len() >= 2).then_some(coords);
                }
                let mut pts = Vec::new();
                for seg in segments {
                    if let Value::Typed(kind, inner) = seg {
                        if kind == "IFCARCINDEX" {
                            *curved = true;
                        }
                        for i in inner.numbers() {
                            let i = i as usize;
                            if i >= 1 && i <= coords.len() {
                                push_unique(&mut pts, coords[i - 1]);
                            }
                        }
                    }
                }
                (pts.len() >= 2).then_some(pts)
            }
            "IFCCOMPOSITECURVE" | "IFCCOMPOSITECURVEONSURFACE" => {
                let mut pts = Vec::new();
                for seg in e.arg(0).as_list() {
                    let s = self.ent(seg)?;
                    let same_sense = s.arg(1).as_bool().unwrap_or(true);
                    let mut part = self.curve_depth(s.arg(2), curved, depth + 1)?;
                    if !same_sense {
                        part.reverse();
                    }
                    for p in part {
                        push_unique(&mut pts, p);
                    }
                }
                (pts.len() >= 2).then_some(pts)
            }
            "IFCTRIMMEDCURVE" => self.trimmed(e, curved),
            "IFCCIRCLE" => {
                *curved = true;
                let xf = self.axis_placement(e.arg(0));
                let r = e.arg(1).as_f64()? * self.length;
                let n = 32;
                Some(
                    (0..n)
                        .map(|i| {
                            let a = TAU * i as f64 / n as f64;
                            xf.apply([r * a.cos(), r * a.sin(), 0.0])
                        })
                        .collect(),
                )
            }
            _ => None,
        }
    }

    fn trimmed(&self, e: &Entity, curved: &mut bool) -> Option<Vec<P3>> {
        let basis = self.ent(e.arg(0))?;
        let sense = e.arg(3).as_bool().unwrap_or(true);
        // A trim is a list with a point and/or a parameter value.
        let trim = |v: &Value| -> (Option<P3>, Option<f64>) {
            let mut point = None;
            let mut param = None;
            for item in v.as_list() {
                match item {
                    Value::Ref(_) => point = point.or(self.point(item)),
                    Value::Typed(k, inner) if k == "IFCPARAMETERVALUE" => {
                        param = inner.as_f64();
                    }
                    other => param = param.or(other.as_f64()),
                }
            }
            (point, param)
        };
        let (p1, t1) = trim(e.arg(1));
        let (p2, t2) = trim(e.arg(2));
        let mut pts = match basis.kind.as_str() {
            "IFCLINE" => {
                let origin = self.point(basis.arg(0))?;
                let vector = self.ent(basis.arg(1))?;
                let dir = self.direction(vector.arg(0))?;
                let mag = vector.arg(1).as_f64().unwrap_or(1.0) * self.length;
                let at = |p: Option<P3>, t: Option<f64>| {
                    p.or_else(|| t.map(|t| [0, 1, 2].map(|i| origin[i] + dir[i] * mag * t)))
                };
                vec![at(p1, t1)?, at(p2, t2)?]
            }
            "IFCCIRCLE" | "IFCELLIPSE" => {
                *curved = true;
                let xf = self.axis_placement(basis.arg(0));
                let r = basis.arg(1).as_f64()? * self.length;
                let r2 = if basis.kind == "IFCELLIPSE" {
                    basis.arg(2).as_f64()? * self.length
                } else {
                    r
                };
                let angle_of = |p: Option<P3>, t: Option<f64>| -> Option<f64> {
                    if let Some(p) = p {
                        let d = sub(p, xf.origin);
                        return Some((dot(d, xf.y) / r2).atan2(dot(d, xf.x) / r));
                    }
                    t.map(|t| t * self.angle)
                };
                let a0 = angle_of(p1, t1)?;
                let mut a1 = angle_of(p2, t2)?;
                if sense {
                    while a1 <= a0 {
                        a1 += TAU;
                    }
                } else {
                    while a1 >= a0 {
                        a1 -= TAU;
                    }
                }
                let n = (((a1 - a0).abs() / TAU * 32.0).ceil() as usize).max(2);
                return Some(
                    (0..=n)
                        .map(|i| {
                            let a = a0 + (a1 - a0) * i as f64 / n as f64;
                            xf.apply([r * a.cos(), r2 * a.sin(), 0.0])
                        })
                        .collect(),
                );
            }
            _ => {
                let mut c = false;
                let all = self.curve_depth(e.arg(0), &mut c, 1)?;
                *curved |= c;
                all
            }
        };
        if !sense && basis.kind == "IFCLINE" {
            pts.reverse();
        }
        Some(pts)
    }

    /// Outer outline and holes of a profile, in profile coordinates.
    pub fn profile(&self, v: &Value) -> Option<(Vec<P3>, Vec<Vec<P3>>)> {
        let e = self.ent(v)?;
        let mut curved = false;
        let closed = |pts: Vec<P3>| -> Vec<P3> {
            let mut pts = pts;
            if pts.len() > 2 && same(pts[0], pts[pts.len() - 1]) {
                pts.pop();
            }
            pts
        };
        match e.kind.as_str() {
            "IFCARBITRARYCLOSEDPROFILEDEF" => {
                Some((closed(self.curve(e.arg(2), &mut curved)?), Vec::new()))
            }
            "IFCARBITRARYPROFILEDEFWITHVOIDS" => {
                let outer = closed(self.curve(e.arg(2), &mut curved)?);
                let holes = e
                    .arg(3)
                    .as_list()
                    .iter()
                    .filter_map(|c| self.curve(c, &mut curved).map(closed))
                    .collect();
                Some((outer, holes))
            }
            "IFCRECTANGLEPROFILEDEF" | "IFCROUNDEDRECTANGLEPROFILEDEF" => {
                let xf = self.axis_placement(e.arg(2));
                let w = e.arg(3).as_f64()? * self.length / 2.0;
                let h = e.arg(4).as_f64()? * self.length / 2.0;
                Some((
                    [[-w, -h], [w, -h], [w, h], [-w, h]]
                        .iter()
                        .map(|&[x, y]| xf.apply([x, y, 0.0]))
                        .collect(),
                    Vec::new(),
                ))
            }
            "IFCCIRCLEPROFILEDEF" => {
                let xf = self.axis_placement(e.arg(2));
                let r = e.arg(3).as_f64()? * self.length;
                let n = 32;
                Some((
                    (0..n)
                        .map(|i| {
                            let a = TAU * i as f64 / n as f64;
                            xf.apply([r * a.cos(), r * a.sin(), 0.0])
                        })
                        .collect(),
                    Vec::new(),
                ))
            }
            "IFCISHAPEPROFILEDEF" | "IFCASYMMETRICISHAPEPROFILEDEF" => {
                let xf = self.axis_placement(e.arg(2));
                let a = e.arg(3).as_f64()? * self.length / 2.0;
                let b = e.arg(4).as_f64()? * self.length / 2.0;
                let w = e.arg(5).as_f64()? * self.length / 2.0;
                let f = e.arg(6).as_f64()? * self.length;
                Some((
                    [
                        [-a, -b],
                        [a, -b],
                        [a, -b + f],
                        [w, -b + f],
                        [w, b - f],
                        [a, b - f],
                        [a, b],
                        [-a, b],
                        [-a, b - f],
                        [-w, b - f],
                        [-w, -b + f],
                        [-a, -b + f],
                    ]
                    .iter()
                    .map(|&[x, y]| xf.apply([x, y, 0.0]))
                    .collect(),
                    Vec::new(),
                ))
            }
            "IFCRECTANGLEHOLLOWPROFILEDEF" | "IFCCIRCLEHOLLOWPROFILEDEF" => {
                let xf = self.axis_placement(e.arg(2));
                let v = |i: usize| e.arg(i).as_f64().map(|x| x * self.length);
                let profile = if e.kind == "IFCCIRCLEHOLLOWPROFILEDEF" {
                    archisim_core::Profile::CircleHollow {
                        diameter: 2.0 * v(3)?,
                        wall: v(4)?,
                    }
                } else {
                    archisim_core::Profile::RectHollow {
                        width: v(3)?,
                        depth: v(4)?,
                        wall: v(5)?,
                    }
                };
                let put = |pts: Vec<archisim_core::Point2>| -> Vec<_> {
                    pts.into_iter().map(|q| xf.apply([q.x, q.y, 0.0])).collect()
                };
                Some((
                    put(profile.outline(32)),
                    profile.hole(32).map(put).into_iter().collect(),
                ))
            }
            "IFCUSHAPEPROFILEDEF" | "IFCTSHAPEPROFILEDEF" | "IFCLSHAPEPROFILEDEF" => {
                let xf = self.axis_placement(e.arg(2));
                let v = |i: usize| e.arg(i).as_f64().map(|x| x * self.length);
                let (depth, width) = (v(3)?, v(4));
                let profile = match e.kind.as_str() {
                    "IFCUSHAPEPROFILEDEF" => archisim_core::Profile::UShape {
                        width: width?,
                        depth,
                        web: v(5)?,
                        flange: v(6)?,
                    },
                    "IFCTSHAPEPROFILEDEF" => archisim_core::Profile::TShape {
                        width: width?,
                        depth,
                        web: v(5)?,
                        flange: v(6)?,
                    },
                    _ => archisim_core::Profile::LShape {
                        width: width.unwrap_or(depth),
                        depth,
                        thickness: v(5)?,
                    },
                };
                Some((
                    profile
                        .outline(0)
                        .into_iter()
                        .map(|q| xf.apply([q.x, q.y, 0.0]))
                        .collect(),
                    Vec::new(),
                ))
            }
            "IFCDERIVEDPROFILEDEF" => {
                let (outer, holes) = self.profile(e.arg(2))?;
                let op = self.operator(e.arg(3));
                Some((
                    outer.into_iter().map(|p| op.apply(p)).collect(),
                    holes
                        .into_iter()
                        .map(|h| h.into_iter().map(|p| op.apply(p)).collect())
                        .collect(),
                ))
            }
            _ => None,
        }
    }

    /// Adds the representation items of a shape to `body`, in world
    /// coordinates.
    pub fn items(&self, items: &[Value], xf: &Xf, body: &mut Body) {
        for item in items {
            self.item(item, xf, body, 0);
        }
    }

    fn item(&self, v: &Value, xf: &Xf, body: &mut Body, depth: usize) {
        let Some(e) = self.ent(v) else {
            return;
        };
        if depth > 16 {
            return;
        }
        match e.kind.as_str() {
            "IFCEXTRUDEDAREASOLID" | "IFCEXTRUDEDAREASOLIDTAPERED" => {
                let Some((outer, holes)) = self.profile(e.arg(0)) else {
                    body.unsupported.push(
                        self.ent(e.arg(0))
                            .map_or("perfil".to_string(), |p| p.kind.clone()),
                    );
                    return;
                };
                let pos = xf.then(&self.axis_placement(e.arg(1)));
                let dir = self.direction(e.arg(2)).unwrap_or([0.0, 0.0, 1.0]);
                let d = e.arg(3).as_f64().unwrap_or(0.0) * self.length;
                let shift = scale(dir, d);
                let base: Vec<P3> = outer.iter().map(|&p| pos.apply(p)).collect();
                let top: Vec<P3> = outer
                    .iter()
                    .map(|&p| pos.apply([p[0] + shift[0], p[1] + shift[1], p[2] + shift[2]]))
                    .collect();
                // Vertical in the world: usable as a plan outline.
                let world_dir = pos.vector(dir);
                let flat = base.iter().all(|p| (p[2] - base[0][2]).abs() < 1e-6);
                if flat
                    && world_dir[2].abs() > 1.0 - 1e-6
                    && world_dir[0].hypot(world_dir[1]) < 1e-5
                {
                    let z0 = base[0][2];
                    let z1 = top[0][2];
                    body.extrusions.push(Extrusion {
                        outline: base.iter().map(|p| [p[0], p[1]]).collect(),
                        holes: holes
                            .iter()
                            .map(|h| {
                                h.iter()
                                    .map(|&p| {
                                        let q = pos.apply(p);
                                        [q[0], q[1]]
                                    })
                                    .collect()
                            })
                            .collect(),
                        bottom: z0.min(z1),
                        top: z0.max(z1),
                    });
                }
                body.polygons.push(Polygon { points: base });
                body.polygons.push(Polygon { points: top });
            }
            "IFCBOOLEANRESULT" | "IFCBOOLEANCLIPPINGRESULT" => {
                // Only the first operand: clippings only remove material.
                let op = e.arg(0).as_enum().unwrap_or("DIFFERENCE");
                if op != "UNION" {
                    body.clipped = true;
                }
                self.item(e.arg(1), xf, body, depth + 1);
                if op == "UNION" {
                    self.item(e.arg(2), xf, body, depth + 1);
                }
            }
            "IFCFACETEDBREP" | "IFCFACETEDBREPWITHVOIDS" => {
                self.shell(e.arg(0), xf, body);
            }
            "IFCSHELLBASEDSURFACEMODEL" | "IFCFACEBASEDSURFACEMODEL" => {
                for s in e.arg(0).as_list() {
                    self.shell(s, xf, body);
                }
            }
            "IFCCLOSEDSHELL" | "IFCOPENSHELL" | "IFCCONNECTEDFACESET" => {
                self.shell(v, xf, body);
            }
            "IFCTRIANGULATEDFACESET" | "IFCPOLYGONALFACESET" => {
                let Some(list) = self.ent(e.arg(0)) else {
                    return;
                };
                let k = self.length;
                let coords: Vec<P3> = list
                    .arg(0)
                    .as_list()
                    .iter()
                    .map(|c| {
                        let n = c.numbers();
                        let at = |i: usize| n.get(i).copied().unwrap_or(0.0) * k;
                        xf.apply([at(0), at(1), at(2)])
                    })
                    .collect();
                let remap = if e.kind == "IFCTRIANGULATEDFACESET" {
                    e.arg(4).numbers()
                } else {
                    e.arg(3).numbers()
                };
                let point = |i: f64| -> Option<P3> {
                    let i = i as usize;
                    let i = if remap.is_empty() {
                        i
                    } else {
                        *remap.get(i.checked_sub(1)?)? as usize
                    };
                    coords.get(i.checked_sub(1)?).copied()
                };
                let mut faces: Vec<Vec<f64>> = Vec::new();
                if e.kind == "IFCTRIANGULATEDFACESET" {
                    faces.extend(e.arg(3).as_list().iter().map(Value::numbers));
                } else {
                    for face in e.arg(2).as_list() {
                        if let Some(face) = self.ent(face) {
                            faces.push(face.arg(0).numbers());
                            for hole in face.arg(1).as_list() {
                                faces.push(hole.numbers());
                            }
                        }
                    }
                }
                for f in faces {
                    let pts: Vec<P3> = f.iter().filter_map(|&i| point(i)).collect();
                    if pts.len() >= 3 {
                        body.polygons.push(Polygon { points: pts });
                    }
                }
                if body.polygons.is_empty() {
                    body.points.extend(coords);
                }
            }
            "IFCMAPPEDITEM" => {
                let Some(source) = self.ent(e.arg(0)) else {
                    return;
                };
                let origin = self.axis_placement(source.arg(0));
                let target = self.operator(e.arg(1));
                let inner = xf.then(&target).then(&origin);
                if let Some(rep) = self.ent(source.arg(1)) {
                    for it in rep.arg(3).as_list() {
                        self.item(it, &inner, body, depth + 1);
                    }
                }
            }
            "IFCBOUNDINGBOX" => {
                let Some(c) = self.point(e.arg(0)) else {
                    return;
                };
                let k = self.length;
                let d = [1, 2, 3].map(|i| e.arg(i).as_f64().unwrap_or(0.0) * k);
                for i in 0..8 {
                    let p = [
                        c[0] + if i & 1 != 0 { d[0] } else { 0.0 },
                        c[1] + if i & 2 != 0 { d[1] } else { 0.0 },
                        c[2] + if i & 4 != 0 { d[2] } else { 0.0 },
                    ];
                    body.points.push(xf.apply(p));
                }
            }
            "IFCSTYLEDITEM" => {}
            other => body.unsupported.push(other.to_string()),
        }
    }

    fn shell(&self, v: &Value, xf: &Xf, body: &mut Body) {
        let Some(shell) = self.ent(v) else {
            return;
        };
        for face in shell.arg(0).as_list() {
            let Some(face) = self.ent(face) else {
                continue;
            };
            for bound in face.arg(0).as_list() {
                let Some(bound) = self.ent(bound) else {
                    continue;
                };
                let Some(lp) = self.ent(bound.arg(0)) else {
                    continue;
                };
                if lp.kind != "IFCPOLYLOOP" {
                    continue;
                }
                let pts: Vec<P3> = lp
                    .arg(0)
                    .as_list()
                    .iter()
                    .filter_map(|p| self.point(p))
                    .map(|p| xf.apply(p))
                    .collect();
                if pts.len() >= 3 {
                    body.polygons.push(Polygon { points: pts });
                }
            }
        }
    }
}

fn same(a: P3, b: P3) -> bool {
    (0..3).all(|i| (a[i] - b[i]).abs() < 1e-9)
}

fn push_unique(pts: &mut Vec<P3>, p: P3) {
    if pts.last().is_none_or(|&q| !same(q, p)) {
        pts.push(p);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::step_read::parse;

    fn file(data: &str) -> StepFile {
        parse(&format!(
            "ISO-10303-21;HEADER;ENDSEC;DATA;{data}ENDSEC;END-ISO-10303-21;"
        ))
        .unwrap()
    }

    #[test]
    fn chains_placements() {
        let f = file(
            "#1=IFCCARTESIANPOINT((0.,0.,3000.));
             #2=IFCAXIS2PLACEMENT3D(#1,$,$);
             #3=IFCLOCALPLACEMENT($,#2);
             #4=IFCCARTESIANPOINT((1000.,0.,0.));
             #5=IFCDIRECTION((0.,1.,0.));
             #6=IFCDIRECTION((0.,0.,1.));
             #7=IFCAXIS2PLACEMENT3D(#4,#6,#5);
             #8=IFCLOCALPLACEMENT(#3,#7);",
        );
        let g = Geo {
            file: &f,
            length: 0.001,
            angle: 1.0,
        };
        let xf = g.placement(&Value::Ref(8));
        let p = xf.apply([1.0, 0.0, 0.0]);
        // Origin at (1, 0, 3) m, local X along world Y.
        assert!((p[0] - 1.0).abs() < 1e-12 && (p[1] - 1.0).abs() < 1e-12);
        assert!((p[2] - 3.0).abs() < 1e-12);
        assert_eq!(xf.y, [-1.0, 0.0, 0.0]);
    }

    #[test]
    fn reads_profiles_and_extrusions() {
        let f = file(
            "#1=IFCCARTESIANPOINT((0.,0.));
             #2=IFCAXIS2PLACEMENT2D(#1,$);
             #3=IFCRECTANGLEPROFILEDEF(.AREA.,$,#2,4.,2.);
             #4=IFCCARTESIANPOINT((0.,0.,1.));
             #5=IFCAXIS2PLACEMENT3D(#4,$,$);
             #6=IFCDIRECTION((0.,0.,1.));
             #7=IFCEXTRUDEDAREASOLID(#3,#5,#6,0.3);
             #8=IFCCARTESIANPOINT((5.,0.,0.));
             #9=IFCCARTESIANPOINT((0.,5.,0.));
             #10=IFCTRIMMEDCURVE(#11,(#8),(#9),.T.,.CARTESIAN.);
             #11=IFCCIRCLE(#5,5.);",
        );
        let g = Geo {
            file: &f,
            length: 1.0,
            angle: 1.0,
        };
        let mut body = Body::default();
        g.items(&[Value::Ref(7)], &Xf::IDENTITY, &mut body);
        let (lo, hi) = body.bounds().unwrap();
        assert_eq!(lo, [-2.0, -1.0, 1.0]);
        assert!((hi[2] - 1.3).abs() < 1e-12 && hi[0] == 2.0);
        assert_eq!(body.extrusions.len(), 1);
        assert_eq!(body.extrusions[0].outline.len(), 4);
        let mut curved = false;
        let arc = g.curve(&Value::Ref(10), &mut curved).unwrap();
        assert!(curved && arc.len() > 3);
        let last = arc[arc.len() - 1];
        assert!(last[0].abs() < 1e-9 && (last[1] - 5.0).abs() < 1e-9);
    }
}
