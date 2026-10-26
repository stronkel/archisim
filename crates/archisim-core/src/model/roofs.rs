//! Pitched roofs by planes ("cubiertas por faldones", ArchiCAD's multi-plane
//! roof, Revit's roof by footprint).
//!
//! A roof is drawn by its footprint (usually the outer faces of the walls
//! under it). Each side of the footprint either slopes (a roof plane rises
//! from it at its pitch) or is a gable ("piñón", no plane). The planes meet
//! at ridges, hips and valleys where they cross; the eaves and verges stand
//! out from the footprint by the overhang. Its build-up is a slab type
//! (usually a roof type): the layers keep their thickness square to each
//! plane and the edges are plumb, as inclined slabs.
//!
//! Any footprint that does not cross itself: over a convex one each plane
//! covers where it is the lowest (hips over every corner); over one with
//! re-entrant corners (an L, a T, a U…) the planes are found by the
//! weighted straight skeleton ([`crate::skeleton`]): each side moves in at
//! the run of its pitch and sweeps its plane, so the planes meet at hips
//! over the outer corners and at valleys ("limahoyas") from the inner
//! ones, and a gable stays put. The eaves and verges stand out from the
//! footprint with the planes that reach them (at an outer corner the lower
//! of the two, at an inner one the higher).
//!
//! IFC: `IfcRoof` made of one `IfcSlab` (`ROOF`) per plane.

use super::*;

/// A pitched roof.
#[derive(Debug, Clone, PartialEq)]
pub struct Roof {
    pub id: RoofId,
    pub guid: GlobalId,
    /// Its build-up: a slab type (the layers, from the top down).
    pub roof_type: SlabTypeId,
    pub level: LevelId,
    /// The footprint, counter-clockwise and convex: where the planes
    /// start, at `base_offset`.
    pub outline: Vec<Point2>,
    /// The pitch of each side (side `i` runs from corner `i` to the
    /// next), as rise per run (0.3 = 30 %); `None`: a gable.
    pub slopes: Vec<Option<f64>>,
    /// Height of the top face above the level along the footprint.
    pub base_offset: f64,
    /// How far the eaves and verges stand out from the footprint.
    pub overhang: f64,
    /// Dormers and roof windows on its planes.
    pub dormers: Vec<Dormer>,
    /// Its tiles and gutters.
    pub covering: RoofCovering,
}

/// Data needed to create a roof.
#[derive(Debug, Clone, PartialEq)]
pub struct NewRoof {
    pub roof_type: SlabTypeId,
    pub level: LevelId,
    pub outline: Vec<Point2>,
    /// One per side; or a single value for every side.
    pub slopes: Vec<Option<f64>>,
    pub base_offset: f64,
    pub overhang: f64,
}

/// One plane of a roof: the side it rises from, its outline in plan (the
/// part of the roof it covers, eaves included) and its slope, as that of
/// an inclined slab whose top face is at the roof's `base_offset` along
/// the footprint.
#[derive(Debug, Clone, PartialEq)]
pub struct RoofPlane {
    pub side: usize,
    pub outline: Vec<Point2>,
    pub slope: SlabSlope,
}

/// Largest overhang (metres).
pub const MAX_OVERHANG: f64 = 3.0;

impl Roof {
    /// The inward normal of side `i` and a point on it.
    fn side(&self, i: usize) -> Option<(Point2, Vec2)> {
        let n = self.outline.len();
        let (a, b) = (self.outline[i], self.outline[(i + 1) % n]);
        // Counter-clockwise: the inside is to the left.
        (b - a).normalized().map(|u| (a, u.perp_left()))
    }

    /// The top face of plane `i` at `p`, above the level (not limited to
    /// the part of the roof it covers).
    fn plane_at(&self, i: usize, p: Point2) -> Option<f64> {
        let s = self.slopes.get(i).copied().flatten()?;
        let (a, n) = self.side(i)?;
        Some(self.base_offset + s * (p - a).dot(n))
    }

    /// Height of the top face above the level at `p`: the plane over it
    /// (over a convex footprint, the lowest one: planes meet at ridges and
    /// hips).
    pub fn top_at(&self, p: Point2) -> f64 {
        let lowest = || {
            (0..self.outline.len())
                .filter_map(|i| self.plane_at(i, p))
                .fold(f64::INFINITY, f64::min)
        };
        if is_convex(&self.outline) {
            return lowest();
        }
        self.planes()
            .iter()
            .find(|pl| point_in_polygon(p, &pl.outline))
            .and_then(|pl| self.plane_at(pl.side, p))
            .unwrap_or_else(lowest)
    }

    /// The outline of the eaves and verges: the footprint grown by the
    /// overhang, counter-clockwise.
    pub fn eaves(&self) -> Vec<Point2> {
        grow_convex(&self.outline, self.overhang)
    }

    /// Whether its footprint has no re-entrant corner.
    pub fn is_convex(&self) -> bool {
        is_convex(&self.outline)
    }

    /// The planes of the roof, each with the part of the roof it covers.
    pub fn planes(&self) -> Vec<RoofPlane> {
        if !is_convex(&self.outline) {
            return self.skeleton_planes().unwrap_or_default();
        }
        let eaves = self.eaves();
        let n = self.outline.len();
        let mut out = Vec::new();
        for i in 0..n {
            let (Some(s), Some((a, nrm))) = (self.slopes[i], self.side(i)) else {
                continue;
            };
            // Where this plane is the lowest: h_i(p) <= h_j(p) for every
            // other plane j, each a half-plane in plan.
            let mut region = eaves.clone();
            for j in 0..n {
                if j == i {
                    continue;
                }
                let (Some(t), Some((b, m))) = (self.slopes[j], self.side(j)) else {
                    continue;
                };
                // h_i - h_j = (s·n_i - t·n_j)·p - (s·n_i·a - t·n_j·b) <= 0.
                let g = nrm * s - m * t;
                let c = s * nrm.dot(a) - t * m.dot(b);
                if g.length() < 1e-12 {
                    // The same plane (sides in line with the same pitch):
                    // the first one takes it.
                    if (c).abs() < 1e-9 && j < i {
                        region.clear();
                    }
                    continue;
                }
                region = clip_half_plane(&region, g, c);
                if region.len() < 3 {
                    break;
                }
            }
            if region.len() >= 3 && signed_area(&region).abs() > 1e-9 {
                out.push(RoofPlane {
                    side: i,
                    outline: region,
                    slope: SlabSlope {
                        from: a,
                        to: a + nrm,
                        slope: s,
                    },
                });
            }
        }
        out
    }

    /// The planes over a footprint with re-entrant corners: the faces of
    /// the weighted straight skeleton, each with its strip of eaves and its
    /// part of the verges over gables.
    fn skeleton_planes(&self) -> Option<Vec<RoofPlane>> {
        let n = self.outline.len();
        let speeds: Vec<f64> = self
            .slopes
            .iter()
            .map(|s| s.map_or(0.0, |s| 1.0 / s))
            .collect();
        let faces = crate::skeleton::faces(&self.outline, &speeds)?;
        let d = self.overhang;
        let eaves = self.eaves();
        // A box holding everything, to clip half-planes from.
        let (mut lo, mut hi) = (eaves[0], eaves[0]);
        for q in &eaves {
            lo = Point2::new(lo.x.min(q.x), lo.y.min(q.y));
            hi = Point2::new(hi.x.max(q.x), hi.y.max(q.y));
        }
        let pad = 1.0 + d;
        let big = vec![
            Point2::new(lo.x - pad, lo.y - pad),
            Point2::new(hi.x + pad, lo.y - pad),
            Point2::new(hi.x + pad, hi.y + pad),
            Point2::new(lo.x - pad, hi.y + pad),
        ];
        let sides: Vec<Option<(Point2, Vec2)>> = (0..n).map(|i| self.side(i)).collect();
        let corner_convex = |k: usize| {
            let (a, b, c) = (
                self.outline[(k + n - 1) % n],
                self.outline[k],
                self.outline[(k + 1) % n],
            );
            (b - a).cross(c - b) >= 0.0
        };
        // Keeps the part where `g·p <= c`.
        let keep = |poly: Vec<Point2>, g: Vec2, c: f64| -> Vec<Point2> {
            if poly.len() < 3 {
                return poly;
            }
            clip_half_plane(&poly, g, c)
        };
        // Where plane i is over plane j (rises above it): g·p <= c means
        // h_i <= h_j.
        let below = |i: usize, j: usize| -> Option<(Vec2, f64)> {
            let (si, sj) = (self.slopes[i]?, self.slopes[j]?);
            let ((a, ni), (b, nj)) = (sides[i]?, sides[j]?);
            let g = ni * si - nj * sj;
            let c = si * ni.dot(a) - sj * nj.dot(b);
            Some((g, c))
        };
        let mut pieces: Vec<Vec<Vec<Point2>>> = vec![Vec::new(); n];
        for i in 0..n {
            let (Some(_), Some((a, ni))) = (self.slopes[i], sides[i]) else {
                continue;
            };
            pieces[i].push(faces[i].clone());
            if d <= 1e-9 {
                continue;
            }
            // Its eaves: outside its side, inside its eave line.
            let mut strip = keep(big.clone(), ni, ni.dot(a));
            strip = keep(strip, -ni, -(ni.dot(a) - d));
            for (k, other) in [(i, (i + n - 1) % n), ((i + 1) % n, (i + 1) % n)] {
                // At corner k, against the side before or after.
                match below(i, other) {
                    Some((g, c)) => {
                        if corner_convex(k) {
                            strip = keep(strip, g, c);
                        } else {
                            strip = keep(strip, -g, -c);
                        }
                    }
                    None => {
                        // A gable: up to its line.
                        if let Some((b, nb)) = sides[other] {
                            strip = keep(strip, -nb, -nb.dot(b));
                        }
                    }
                }
            }
            if strip.len() >= 3 && signed_area(&strip).abs() > 1e-9 {
                pieces[i].push(strip);
            }
        }
        // The verges over each gable: the planes either side reach over
        // it, the lower of the two where they meet.
        for g in 0..n {
            if self.slopes[g].is_some() || d <= 1e-9 {
                continue;
            }
            let Some((a, ng)) = sides[g] else { continue };
            let (h, k) = ((g + n - 1) % n, (g + 1) % n);
            let mut verge = keep(big.clone(), ng, ng.dot(a));
            verge = keep(verge, -ng, -(ng.dot(a) - d));
            for j in [h, k] {
                if let Some((b, nj)) = sides[j] {
                    verge = keep(verge, -nj, -(nj.dot(b) - d));
                }
            }
            match below(h, k) {
                Some((gv, c)) => {
                    let mine = keep(verge.clone(), gv, c);
                    let theirs = keep(verge, -gv, -c);
                    for (j, part) in [(h, mine), (k, theirs)] {
                        if part.len() >= 3 && signed_area(&part).abs() > 1e-9 {
                            pieces[j].push(part);
                        }
                    }
                }
                None => {
                    for j in [h, k] {
                        if self.slopes[j].is_some() && verge.len() >= 3 {
                            pieces[j].push(verge.clone());
                            break;
                        }
                    }
                }
            }
        }
        let mut out = Vec::new();
        for (i, parts) in pieces.into_iter().enumerate() {
            let (Some(s), Some((a, ni))) = (self.slopes[i], sides[i]) else {
                continue;
            };
            let slope = SlabSlope {
                from: a,
                to: a + ni,
                slope: s,
            };
            match merge_pieces(&parts) {
                Some(outline) => out.push(RoofPlane {
                    side: i,
                    outline,
                    slope,
                }),
                // Not one outline: each piece on its own.
                None => out.extend(parts.into_iter().filter(|p| p.len() >= 3).map(|outline| {
                    RoofPlane {
                        side: i,
                        outline,
                        slope,
                    }
                })),
            }
        }
        Some(out)
    }

    /// Where planes meet inside the footprint (or on a gable): the ends of
    /// ridges, hips and valleys, each with the sides whose planes meet
    /// there. Moving one ([`Model::move_roof_node`]) changes their pitches.
    pub fn nodes(&self) -> Vec<(Point2, Vec<usize>)> {
        let n = self.outline.len();
        let on_sloping_side = |q: Point2| {
            (0..n).any(|i| {
                self.slopes[i].is_some() && {
                    let (a, b) = (self.outline[i], self.outline[(i + 1) % n]);
                    let d = b - a;
                    let t = ((q - a).dot(d) / d.dot(d)).clamp(0.0, 1.0);
                    (a + d * t).distance(q) <= 1e-7
                }
            })
        };
        let mut out: Vec<(Point2, Vec<usize>)> = Vec::new();
        for pl in self.planes() {
            for q in &pl.outline {
                if self.outline.iter().any(|c| c.distance(*q) <= 1e-7)
                    || !point_in_polygon(*q, &self.outline)
                    || on_sloping_side(*q)
                {
                    continue;
                }
                match out.iter_mut().find(|(p, _)| p.distance(*q) <= 1e-6) {
                    Some((_, sides)) => {
                        if !sides.contains(&pl.side) {
                            sides.push(pl.side);
                        }
                    }
                    None => out.push((*q, vec![pl.side])),
                }
            }
        }
        out.retain(|(_, s)| s.len() >= 2);
        out
    }

    /// Lowest and highest point of the top face above the level.
    pub fn top_range(&self) -> (f64, f64) {
        self.planes()
            .iter()
            .flat_map(|p| p.outline.iter().map(|q| self.top_at(*q)))
            .fold((f64::INFINITY, f64::NEG_INFINITY), |(lo, hi), z| {
                (lo.min(z), hi.max(z))
            })
    }

    /// Area of the roof surface (on the slope), eaves included, less the
    /// holes of its dormers.
    pub fn surface_area(&self) -> f64 {
        self.planes()
            .iter()
            .map(|p| {
                let (inside, crossing) = self.plane_cuts(p);
                let holes: f64 =
                    inside
                        .iter()
                        .map(|h| signed_area(h).abs())
                        .chain(crossing.iter().map(|h| {
                            signed_area(&super::dormers::clip_to_convex(&p.outline, h)).abs()
                        }))
                        .sum();
                (signed_area(&p.outline).abs() - holes) * p.slope.vertical(1.0)
            })
            .sum()
    }
}

/// Whether `p` lies inside a polygon (even–odd), or on its edge.
pub fn point_in_polygon(p: Point2, poly: &[Point2]) -> bool {
    let n = poly.len();
    let mut odd = false;
    for i in 0..n {
        let (a, b) = (poly[i], poly[(i + n - 1) % n]);
        // On the edge.
        let d = b - a;
        let len2 = d.dot(d);
        if len2 > 0.0 {
            let t = ((p - a).dot(d) / len2).clamp(0.0, 1.0);
            if (a + d * t).distance(p) <= 1e-9 {
                return true;
            }
        }
        if (a.y > p.y) != (b.y > p.y) && p.x < a.x + (p.y - a.y) / (b.y - a.y) * (b.x - a.x) {
            odd = !odd;
        }
    }
    odd
}

/// One counter-clockwise outline from polygons that touch along their
/// edges (and do not overlap): the edges they share go, the rest are
/// joined. `None` if they do not make one outline.
pub fn merge_pieces(pieces: &[Vec<Point2>]) -> Option<Vec<Point2>> {
    let tol = 1e-7;
    let pieces: Vec<Vec<Point2>> = pieces
        .iter()
        .filter(|p| p.len() >= 3 && signed_area(p).abs() > 1e-12)
        .map(|p| {
            let mut p = p.clone();
            if signed_area(&p) < 0.0 {
                p.reverse();
            }
            p
        })
        .collect();
    if pieces.len() == 1 {
        return Some(pieces[0].clone());
    }
    if pieces.is_empty() {
        return None;
    }
    // Every corner, once.
    let mut pts: Vec<Point2> = Vec::new();
    let id = |pts: &mut Vec<Point2>, q: Point2| -> usize {
        match pts.iter().position(|r| r.distance(q) <= tol) {
            Some(k) => k,
            None => {
                pts.push(q);
                pts.len() - 1
            }
        }
    };
    let loops: Vec<Vec<usize>> = pieces
        .iter()
        .map(|p| {
            let mut l: Vec<usize> = p.iter().map(|q| id(&mut pts, *q)).collect();
            l.dedup();
            if l.len() > 1 && l[0] == l[l.len() - 1] {
                l.pop();
            }
            l
        })
        .collect();
    // Edges, split at corners lying on them.
    let mut edges: Vec<(usize, usize)> = Vec::new();
    for l in &loops {
        for k in 0..l.len() {
            let (a, b) = (l[k], l[(k + 1) % l.len()]);
            let (pa, pb) = (pts[a], pts[b]);
            let d = pb - pa;
            let len2 = d.dot(d);
            let mut on: Vec<(f64, usize)> = (0..pts.len())
                .filter(|&c| c != a && c != b)
                .filter_map(|c| {
                    let t = (pts[c] - pa).dot(d) / len2;
                    let off = (pts[c] - pa).cross(d).abs() / len2.sqrt();
                    (t > 1e-9 && t < 1.0 - 1e-9 && off <= tol).then_some((t, c))
                })
                .collect();
            on.sort_by(|x, y| x.0.total_cmp(&y.0));
            let mut prev = a;
            for (_, c) in on {
                edges.push((prev, c));
                prev = c;
            }
            edges.push((prev, b));
        }
    }
    // Shared edges (one each way) go.
    let mut keep = vec![true; edges.len()];
    for i in 0..edges.len() {
        if !keep[i] {
            continue;
        }
        if let Some(j) =
            (0..edges.len()).find(|&j| j != i && keep[j] && edges[j] == (edges[i].1, edges[i].0))
        {
            keep[i] = false;
            keep[j] = false;
        }
    }
    let rest: Vec<(usize, usize)> = edges
        .into_iter()
        .zip(keep)
        .filter(|(_, k)| *k)
        .map(|(e, _)| e)
        .collect();
    if rest.is_empty() {
        return None;
    }
    // Joined into one loop.
    let mut out = vec![rest[0].0];
    let mut used = vec![false; rest.len()];
    used[0] = true;
    let mut at = rest[0].1;
    while at != out[0] {
        let k = (0..rest.len()).find(|&k| !used[k] && rest[k].0 == at)?;
        used[k] = true;
        out.push(at);
        at = rest[k].1;
    }
    if used.iter().any(|u| !u) {
        return None;
    }
    let mut poly: Vec<Point2> = out.into_iter().map(|k| pts[k]).collect();
    // Corners in line go.
    let mut i = 0;
    while poly.len() > 3 && i < poly.len() {
        let m = poly.len();
        let (a, b, c) = (poly[(i + m - 1) % m], poly[i], poly[(i + 1) % m]);
        if (b - a).cross(c - b).abs() <= 1e-9 * (b - a).length().max(1.0)
            && (b - a).dot(c - b) > 0.0
        {
            poly.remove(i);
        } else {
            i += 1;
        }
    }
    Some(poly)
}

/// A polygon (counter-clockwise) grown outwards by `d`: each side moved
/// out square to itself, the corners where they meet (convex or not).
pub fn grow_convex(poly: &[Point2], d: f64) -> Vec<Point2> {
    let n = poly.len();
    if d.abs() < 1e-12 || n < 3 {
        return poly.to_vec();
    }
    let moved: Vec<(Point2, Vec2)> = (0..n)
        .map(|i| {
            let (a, b) = (poly[i], poly[(i + 1) % n]);
            let u = (b - a).normalized().unwrap_or(Vec2::new(1.0, 0.0));
            // Outside is to the right.
            (a - u.perp_left() * d, u)
        })
        .collect();
    (0..n)
        .map(|i| {
            let (p, u) = moved[(i + n - 1) % n];
            let (q, v) = moved[i];
            let den = u.x * v.y - u.y * v.x;
            if den.abs() < 1e-12 {
                // In line: the corner just moves out.
                q
            } else {
                let w = q - p;
                let t = (w.x * v.y - w.y * v.x) / den;
                p + u * t
            }
        })
        .collect()
}

/// The part of a convex polygon where `g·p <= c`.
pub(crate) fn clip_half_plane(poly: &[Point2], g: Vec2, c: f64) -> Vec<Point2> {
    let f = |p: Point2| g.dot(p) - c;
    let n = poly.len();
    let mut out = Vec::new();
    for i in 0..n {
        let (a, b) = (poly[i], poly[(i + 1) % n]);
        let (fa, fb) = (f(a), f(b));
        if fa <= 1e-12 {
            out.push(a);
        }
        if (fa < -1e-12 && fb > 1e-12) || (fa > 1e-12 && fb < -1e-12) {
            let t = fa / (fa - fb);
            out.push(a + (b - a) * t);
        }
    }
    // Drop points that repeat.
    out.dedup_by(|a, b| a.approx_eq(*b));
    if out.len() > 1 && out[0].approx_eq(out[out.len() - 1]) {
        out.pop();
    }
    out
}

/// Volume of a prism on `footprint` from `z_base` up to the lowest of
/// the planes `z = a·x + b·y + c` (a wall trimmed under a roof): over the
/// part of the footprint where each plane is the lowest, its area by the
/// height at its centroid.
pub fn volume_under_planes(footprint: &[Point2], planes: &[[f64; 3]], z_base: f64) -> f64 {
    let mut poly = footprint.to_vec();
    if signed_area(&poly) < 0.0 {
        poly.reverse();
    }
    let mut total = 0.0;
    for (i, p) in planes.iter().enumerate() {
        let mut region = poly.clone();
        for (j, q) in planes.iter().enumerate() {
            if i == j || region.len() < 3 {
                continue;
            }
            let g = Vec2::new(p[0] - q[0], p[1] - q[1]);
            let c = q[2] - p[2];
            if g.length() < 1e-12 {
                // The same slope: the lower one covers it all.
                if c < 0.0 || (c.abs() < 1e-12 && j < i) {
                    region.clear();
                }
                continue;
            }
            region = clip_half_plane(&region, g, c);
        }
        if region.len() >= 3 {
            let area = signed_area(&region).abs();
            let m = crate::rooms::centroid(&region);
            total += area * (p[0] * m.x + p[1] * m.y + p[2] - z_base);
        }
    }
    total
}

/// Whether a counter-clockwise polygon has no re-entrant corner.
fn is_convex(poly: &[Point2]) -> bool {
    let n = poly.len();
    (0..n).all(|i| {
        let (a, b, c) = (poly[i], poly[(i + 1) % n], poly[(i + 2) % n]);
        let (u, v) = (b - a, c - b);
        u.x * v.y - u.y * v.x >= -1e-9 * (u.length() * v.length()).max(1e-12)
    })
}

impl Model {
    fn validate_roof(&self, r: &NewRoof) -> Result<(Vec<Point2>, Vec<Option<f64>>), ModelError> {
        if !self.levels.contains_key(&r.level) {
            return Err(ModelError::UnknownLevel(r.level));
        }
        if !self.slab_types.contains_key(&r.roof_type) {
            return Err(ModelError::UnknownSlabType(r.roof_type));
        }
        finite(r.base_offset, "roof base offset")?;
        finite(r.overhang, "roof overhang")?;
        if !(0.0..=MAX_OVERHANG).contains(&r.overhang) {
            return Err(ModelError::InvalidRoof);
        }
        // Counter-clockwise; the pitches follow the sides if it turns.
        let mut outline = r.outline.clone();
        let mut slopes = r.slopes.clone();
        let n = outline.len();
        if slopes.len() == 1 && n > 1 {
            slopes = vec![slopes[0]; n];
        }
        if slopes.len() != n {
            return Err(ModelError::InvalidRoof);
        }
        for p in &outline {
            finite_point(*p, "roof outline point")?;
        }
        if !is_simple_polygon(&outline) || signed_area(&outline).abs() <= EPSILON {
            return Err(ModelError::InvalidOutline);
        }
        if signed_area(&outline) < 0.0 {
            outline.reverse();
            // Side i of the reversed outline is side n-2-i of the original.
            let old = slopes.clone();
            slopes = (0..n).map(|i| old[(2 * n - 2 - i) % n]).collect();
        }
        let mut any = false;
        for s in slopes.iter().flatten() {
            finite(*s, "roof slope")?;
            if *s <= 1e-6 || *s > MAX_SLAB_SLOPE {
                return Err(ModelError::InvalidRoof);
            }
            any = true;
        }
        if !any {
            return Err(ModelError::InvalidRoof);
        }
        if !is_convex(&outline) {
            // Re-entrant corners: the planes must be found, the eaves must
            // not cross themselves.
            let speeds: Vec<f64> = slopes.iter().map(|s| s.map_or(0.0, |s| 1.0 / s)).collect();
            let eaves = grow_convex(&outline, r.overhang);
            if crate::skeleton::faces(&outline, &speeds).is_none() || !is_simple_polygon(&eaves) {
                return Err(ModelError::RoofNotConvex);
            }
        }
        Ok((outline, slopes))
    }

    pub fn add_roof(&mut self, spec: NewRoof) -> Result<RoofId, ModelError> {
        let (outline, slopes) = self.validate_roof(&spec)?;
        let id = RoofId(self.take_id());
        self.roofs.insert(
            id,
            Roof {
                id,
                guid: GlobalId::new(),
                roof_type: spec.roof_type,
                level: spec.level,
                outline,
                slopes,
                base_offset: spec.base_offset,
                overhang: spec.overhang,
                dormers: Vec::new(),
                covering: RoofCovering::default(),
            },
        );
        self.stamp_phase(id.0);
        Ok(id)
    }

    /// Changes a roof; nothing changes if the result is not valid.
    pub fn edit_roof(
        &mut self,
        id: RoofId,
        edit: impl FnOnce(&mut Roof),
    ) -> Result<(), ModelError> {
        let mut r = self
            .roofs
            .get(&id)
            .cloned()
            .ok_or(ModelError::UnknownRoof(id))?;
        // Where its dormers are, in plan, in case the sides change.
        let old_sides = r.outline.len();
        let spots: Vec<Option<Point2>> = r
            .dormers
            .iter()
            .map(|d| r.dormer_frame(d).map(|f| f.origin))
            .collect();
        edit(&mut r);
        let (outline, slopes) = self.validate_roof(&NewRoof {
            roof_type: r.roof_type,
            level: r.level,
            outline: r.outline.clone(),
            slopes: r.slopes.clone(),
            base_offset: r.base_offset,
            overhang: r.overhang,
        })?;
        // Drawn the other way round (a mirrored roof): the sides are
        // renumbered and each runs the other way.
        let reversed = signed_area(&r.outline) < 0.0;
        let n = outline.len();
        r.outline = outline;
        r.slopes = slopes;
        if reversed && n == old_sides {
            for d in &mut r.dormers {
                d.side = (2 * n - 2 - d.side) % n;
                d.along = -d.along;
            }
        } else if n != old_sides {
            // Sides added or removed: each dormer goes back on whatever
            // plane is where it was.
            let found: Vec<Option<(usize, f64, f64)>> = spots
                .iter()
                .map(|p| p.and_then(|p| r.locate_on_plane(p)))
                .collect();
            for (d, f) in r.dormers.iter_mut().zip(found) {
                if let Some((side, along, up)) = f {
                    d.side = side;
                    d.along = along;
                    d.up = up;
                }
            }
        }
        self.roofs.insert(id, r);
        Ok(())
    }

    pub fn remove_roof(&mut self, id: RoofId) -> Result<Roof, ModelError> {
        let r = self.roofs.remove(&id).ok_or(ModelError::UnknownRoof(id))?;
        self.wall_roofs.retain(|_, x| *x != id);
        Ok(r)
    }

    /// Trims a wall under a roof (its top follows the underside of the
    /// roof's planes), or frees it.
    pub fn set_wall_roof(&mut self, wall: WallId, roof: Option<RoofId>) -> Result<(), ModelError> {
        if !self.walls.contains_key(&wall) {
            return Err(ModelError::UnknownWall(wall));
        }
        match roof {
            Some(r) => {
                if !self.roofs.contains_key(&r) {
                    return Err(ModelError::UnknownRoof(r));
                }
                self.wall_roofs.insert(wall, r);
            }
            None => {
                self.wall_roofs.remove(&wall);
            }
        }
        Ok(())
    }

    /// Moves the point where planes meet (`node`, from [`Roof::nodes`]) to
    /// `to`, at the same height: the planes meeting there take the pitch
    /// that brings them through it (a hip, valley or ridge moved by hand,
    /// as for a roof of special geometry).
    pub fn move_roof_node(
        &mut self,
        id: RoofId,
        node: usize,
        to: Point2,
    ) -> Result<(), ModelError> {
        let roof = self.roofs.get(&id).ok_or(ModelError::UnknownRoof(id))?;
        let nodes = roof.nodes();
        let (at, sides) = nodes.get(node).cloned().ok_or(ModelError::InvalidRoof)?;
        finite_point(to, "roof node")?;
        let rise = |i: usize, p: Point2| -> Option<(f64, f64)> {
            let s = roof.slopes[i]?;
            let (a, nrm) = roof.side(i)?;
            Some((s, (p - a).dot(nrm)))
        };
        // Its height: as each plane there gives it.
        let hs: Vec<f64> = sides
            .iter()
            .filter_map(|&i| rise(i, at).map(|(s, d)| s * d))
            .collect();
        if hs.is_empty() {
            return Err(ModelError::InvalidRoof);
        }
        let z = hs.iter().sum::<f64>() / hs.len() as f64;
        let mut new = Vec::new();
        for &i in &sides {
            let Some((_, d)) = rise(i, to) else { continue };
            if d <= 0.05 {
                return Err(ModelError::InvalidRoof);
            }
            new.push((i, z / d));
        }
        self.edit_roof(id, |r| {
            for (i, s) in new {
                r.slopes[i] = Some(s);
            }
        })
    }

    /// The roof a wall is trimmed under.
    pub fn wall_roof(&self, wall: WallId) -> Option<RoofId> {
        self.wall_roofs.get(&wall).copied()
    }

    /// The planes under which a wall is trimmed, as `z = a·x + b·y + c`
    /// (heights from project zero): the undersides of its roof's planes.
    pub fn wall_trim_planes(&self, wall: WallId) -> Vec<[f64; 3]> {
        let Some(roof) = self.wall_roof(wall).and_then(|r| self.roofs.get(&r)) else {
            return Vec::new();
        };
        let level = self.levels.get(&roof.level).map_or(0.0, |l| l.elevation);
        let thickness = self
            .slab_types
            .get(&roof.roof_type)
            .map_or(0.0, |t| t.thickness());
        let mut planes = roof.planes();
        // Over a footprint with inner corners, only the planes over the
        // wall (the far wing's would cut it where it is not).
        if !roof.is_convex() {
            if let Some(w) = self.walls.get(&wall) {
                // Over its footprint: along it, across its thickness (just
                // inside its faces).
                let t = self
                    .wall_types
                    .get(&w.wall_type)
                    .map_or(0.3, |t| t.thickness());
                let (ext, int) = w.face_offsets(t);
                let across = (w.end - w.start)
                    .normalized()
                    .map_or(Vec2::new(0.0, 0.0), |u| u.perp_left());
                let steps = ((w.start.distance(w.end) / 0.1).ceil() as usize).max(1);
                let over: Vec<usize> = (1..steps)
                    .flat_map(|k| {
                        let p = w.start + (w.end - w.start) * (k as f64 / steps as f64);
                        [0.02, 0.5, 0.98].map(|f| p + across * (ext + (int - ext) * f))
                    })
                    .filter_map(|p| {
                        planes
                            .iter()
                            .find(|pl| point_in_polygon(p, &pl.outline))
                            .map(|pl| pl.side)
                    })
                    .collect();
                planes.retain(|pl| over.contains(&pl.side));
            }
        }
        planes
            .iter()
            .filter_map(|p| {
                let u = p.slope.direction()?;
                let s = p.slope.slope;
                let c = level + roof.base_offset
                    - s * p.slope.from.dot(u)
                    - p.slope.vertical(thickness);
                Some([s * u.x, s * u.y, c])
            })
            .collect()
    }

    /// The highest point a wall reaches: its own top, or the ridge of the
    /// roof it is trimmed under if that is higher (it is built that high
    /// and then trimmed).
    pub fn wall_top_z(&self, wall: &Wall) -> f64 {
        let level = self.levels.get(&wall.level).map_or(0.0, |l| l.elevation);
        let own = level + wall.base_offset + wall.height;
        match self.wall_roof(wall.id).and_then(|r| self.roofs.get(&r)) {
            Some(roof) => {
                let rl = self.levels.get(&roof.level).map_or(0.0, |l| l.elevation);
                own.max(rl + roof.top_range().1)
            }
            None => own,
        }
    }

    pub fn roof(&self, id: RoofId) -> Option<&Roof> {
        self.roofs.get(&id)
    }

    pub fn roofs(&self) -> impl Iterator<Item = &Roof> {
        self.roofs.values()
    }

    pub fn roofs_on_level(&self, level: LevelId) -> impl Iterator<Item = &Roof> {
        self.roofs.values().filter(move |r| r.level == level)
    }

    pub(super) fn restore_roofs(
        &mut self,
        roofs: Vec<Roof>,
        claim: &mut structure::Claim<'_>,
    ) -> Result<(), RestoreError> {
        for mut r in roofs {
            let name = format!("roof {} ({})", r.id.0, r.guid);
            let spec = NewRoof {
                roof_type: r.roof_type,
                level: r.level,
                outline: r.outline.clone(),
                slopes: r.slopes.clone(),
                base_offset: r.base_offset,
                overhang: r.overhang,
            };
            let parts = r
                .dormers
                .iter()
                .try_for_each(|d| self.check_dormer_parts(d))
                .and_then(|_| self.check_covering(&r.covering));
            claim(r.id.0, Some(&r.guid))
                .and(parts)
                .and_then(|_| self.validate_roof(&spec))
                .map(|(outline, slopes)| {
                    r.outline = outline;
                    r.slopes = slopes;
                })
                .map_err(|error| RestoreError {
                    element: name,
                    error,
                })?;
            self.roofs.insert(r.id, r);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn setup() -> (Model, LevelId, SlabTypeId) {
        let mut m = Model::new("t");
        let l = m.add_level("Cubierta", 3.0);
        let tile = m.add_material("Teja", "tile", [180, 90, 60], CutPattern::Solid);
        let t = m
            .add_slab_type(
                "Cubierta de teja",
                SlabFunction::Roof,
                vec![Layer {
                    material: tile,
                    thickness: 0.2,
                    function: LayerFunction::Structure,
                }],
            )
            .unwrap();
        (m, l, t)
    }

    fn rect(w: f64, d: f64) -> Vec<Point2> {
        vec![
            Point2::new(0.0, 0.0),
            Point2::new(w, 0.0),
            Point2::new(w, d),
            Point2::new(0.0, d),
        ]
    }

    /// A hip roof over 12 × 8 at 50 %: four planes, the ridge 2 m up.
    #[test]
    fn hip_roof() {
        let (mut m, l, t) = setup();
        let r = m
            .add_roof(NewRoof {
                roof_type: t,
                level: l,
                outline: rect(12.0, 8.0),
                slopes: vec![Some(0.5)],
                base_offset: 0.0,
                overhang: 0.5,
            })
            .unwrap();
        let roof = m.roof(r).unwrap();
        let planes = roof.planes();
        assert_eq!(planes.len(), 4);
        // The planes cover the eaves exactly.
        let covered: f64 = planes.iter().map(|p| signed_area(&p.outline)).sum();
        assert!((covered - 13.0 * 9.0).abs() < 1e-9, "{covered}");
        assert!((roof.top_at(Point2::new(6.0, 4.0)) - 2.0).abs() < 1e-12);
        // The eaves drop below the footprint by the overhang's rise.
        assert!((roof.top_at(Point2::new(6.0, -0.5)) + 0.25).abs() < 1e-12);
        assert_eq!(roof.top_range(), (-0.25, 2.0));
        // The long sides make trapezia, the short ones triangles.
        let corners: Vec<usize> = planes.iter().map(|p| p.outline.len()).collect();
        assert_eq!(corners, vec![4, 3, 4, 3]);
        let back = Model::from_parts(m.to_parts()).unwrap();
        assert_eq!(back, m);
    }

    /// Gables at both ends: two planes and a ridge along the length.
    #[test]
    fn gable_roof_and_errors() {
        let (mut m, l, t) = setup();
        let spec = NewRoof {
            roof_type: t,
            level: l,
            outline: rect(10.0, 6.0),
            slopes: vec![Some(0.3), None, Some(0.3), None],
            base_offset: 0.0,
            overhang: 0.0,
        };
        let r = m.add_roof(spec.clone()).unwrap();
        let roof = m.roof(r).unwrap();
        assert_eq!(roof.planes().len(), 2);
        assert!((roof.top_at(Point2::new(1.0, 3.0)) - 0.9).abs() < 1e-12);
        // Drawn clockwise, the pitches stay on their sides.
        let mut cw = spec.clone();
        cw.outline.reverse();
        cw.slopes = vec![Some(0.3), None, Some(0.3), None];
        // (Its side 0 is the north one.)
        let r2 = m.add_roof(cw).unwrap();
        let roof2 = m.roof(r2).unwrap();
        assert!((roof2.top_at(Point2::new(5.0, 3.0)) - 0.9).abs() < 1e-12);
        // No plane at all, an L, a flat pitch, too much overhang.
        let mut bad = spec.clone();
        bad.slopes = vec![None];
        assert_eq!(m.add_roof(bad), Err(ModelError::InvalidRoof));
        let mut l_shape = spec.clone();
        l_shape.outline = vec![
            Point2::new(0.0, 0.0),
            Point2::new(6.0, 0.0),
            Point2::new(6.0, 3.0),
            Point2::new(3.0, 3.0),
            Point2::new(3.0, 6.0),
            Point2::new(0.0, 6.0),
        ];
        l_shape.slopes = vec![Some(0.3)];
        // An L is resolved (see `l_roof`).
        assert!(m.add_roof(l_shape).is_ok());
        let mut flat = spec.clone();
        flat.slopes = vec![Some(0.0)];
        assert_eq!(m.add_roof(flat), Err(ModelError::InvalidRoof));
        let mut wide = spec;
        wide.overhang = 5.0;
        assert_eq!(m.add_roof(wide), Err(ModelError::InvalidRoof));
        // Edited whole or not at all.
        assert!(m.edit_roof(r, |r| r.slopes = vec![None; 4]).is_err());
        assert_eq!(m.roof(r).unwrap().planes().len(), 2);
    }

    #[test]
    fn grown_outline() {
        let g = grow_convex(&rect(4.0, 2.0), 1.0);
        assert_eq!(
            g,
            vec![
                Point2::new(-1.0, -1.0),
                Point2::new(5.0, -1.0),
                Point2::new(5.0, 3.0),
                Point2::new(-1.0, 3.0)
            ]
        );
    }

    /// One roof over an L (v0.26): six planes, a valley from the inner
    /// corner, hips at the outer ones; eaves all round; the top face
    /// continuous across every hip and valley.
    #[test]
    fn l_roof() {
        let (mut m, l, t) = setup();
        let outline = vec![
            Point2::new(0.0, 0.0),
            Point2::new(10.0, 0.0),
            Point2::new(10.0, 4.0),
            Point2::new(4.0, 4.0),
            Point2::new(4.0, 10.0),
            Point2::new(0.0, 10.0),
        ];
        let r = m
            .add_roof(NewRoof {
                roof_type: t,
                level: l,
                outline: outline.clone(),
                slopes: vec![Some(0.5)],
                base_offset: 0.0,
                overhang: 0.5,
            })
            .unwrap();
        let roof = m.roof(r).unwrap().clone();
        assert!(!roof.is_convex());
        let planes = roof.planes();
        assert_eq!(planes.len(), 6);
        // They cover the eaves, without overlapping.
        let area: f64 = planes.iter().map(|p| signed_area(&p.outline)).sum();
        let eaves = signed_area(&roof.eaves());
        assert!((area - eaves).abs() < 1e-6, "{area} vs {eaves}");
        // The valley: at (3, 3) the roof is 1 m × 50 % up (the higher of
        // the two planes there); the ridges 1 m up; eaves 0.25 m down.
        assert!((roof.top_at(Point2::new(3.0, 3.0)) - 0.5).abs() < 1e-9);
        assert!((roof.top_at(Point2::new(7.0, 2.0)) - 1.0).abs() < 1e-9);
        assert!((roof.top_at(Point2::new(2.0, 7.0)) - 1.0).abs() < 1e-9);
        assert!((roof.top_at(Point2::new(4.2, 4.3)) - (-0.1)).abs() < 1e-9);
        assert!((roof.top_at(Point2::new(-0.5, 5.0)) + 0.25).abs() < 1e-9);
        // Continuous: at every corner of every plane, every plane through
        // that point gives the same height.
        for pl in &planes {
            for q in &pl.outline {
                let h = |p: &RoofPlane| {
                    let s = p.slope;
                    s.slope * (*q - s.from).dot((s.to - s.from).normalized().unwrap())
                };
                for other in &planes {
                    if other.outline.iter().any(|x| x.distance(*q) < 1e-7) {
                        assert!((h(pl) - h(other)).abs() < 1e-6, "{q:?}");
                    }
                }
            }
        }
        // Gable ends on both wings: four planes, verges over the gables.
        m.edit_roof(r, |r| {
            r.slopes = vec![Some(0.5), None, Some(0.5), Some(0.5), None, Some(0.5)]
        })
        .unwrap();
        let roof = m.roof(r).unwrap().clone();
        let planes = roof.planes();
        assert_eq!(planes.len(), 4);
        let area: f64 = planes.iter().map(|p| signed_area(&p.outline)).sum();
        assert!((area - signed_area(&roof.eaves())).abs() < 1e-6);
        // The ridge runs out to the verge: 1 m up at the gable, still over
        // the overhang.
        assert!((roof.top_at(Point2::new(10.4, 2.0)) - 1.0).abs() < 1e-9);
        let back = Model::from_parts(m.to_parts()).unwrap();
        assert_eq!(back, m);
    }

    /// Hips moved by hand: the end of the ridge of a hip roof dragged
    /// along it makes that hip end steeper; dragged across, the ridge goes
    /// off centre (one side steeper, the other gentler).
    #[test]
    fn moving_where_planes_meet() {
        let (mut m, l, t) = setup();
        let r = m
            .add_roof(NewRoof {
                roof_type: t,
                level: l,
                outline: rect(12.0, 8.0),
                slopes: vec![Some(0.5)],
                base_offset: 0.0,
                overhang: 0.0,
            })
            .unwrap();
        let nodes = m.roof(r).unwrap().nodes();
        assert_eq!(nodes.len(), 2);
        let k = nodes
            .iter()
            .position(|(p, _)| p.distance(Point2::new(8.0, 4.0)) < 1e-9)
            .unwrap();
        assert_eq!(nodes[k].1.len(), 3);
        m.move_roof_node(r, k, Point2::new(9.0, 4.0)).unwrap();
        let roof = m.roof(r).unwrap();
        assert!((roof.slopes[1].unwrap() - 2.0 / 3.0).abs() < 1e-12);
        assert_eq!(roof.slopes[0], Some(0.5));
        assert!(roof
            .nodes()
            .iter()
            .any(|(p, _)| p.distance(Point2::new(9.0, 4.0)) < 1e-9));
        // Across: the ridge at y = 5, still 2 m up.
        let k = roof
            .nodes()
            .iter()
            .position(|(p, _)| p.distance(Point2::new(4.0, 4.0)) < 1e-9)
            .unwrap();
        m.move_roof_node(r, k, Point2::new(4.0, 5.0)).unwrap();
        let roof = m.roof(r).unwrap();
        assert!((roof.slopes[0].unwrap() - 0.4).abs() < 1e-12);
        assert!((roof.slopes[2].unwrap() - 2.0 / 3.0).abs() < 1e-12);
        assert!((roof.top_at(Point2::new(6.0, 5.0)) - 2.0).abs() < 1e-9);
        // Not past the side.
        assert!(m.move_roof_node(r, 0, Point2::new(4.0, -1.0)).is_err());
    }
}
