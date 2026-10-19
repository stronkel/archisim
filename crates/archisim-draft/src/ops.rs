//! The editing commands of AutoCAD on drafting shapes: offset (EQUIDIST),
//! trim (RECORTA), extend (ALARGA), fillet (EMPALME), chamfer (CHAFLAN),
//! break (PARTE), join (JUNTAR), explode (DESCOMP), arrays (MATRIZ),
//! stretch (ESTIRA), and helpers to build rectangles, polygons and
//! revision clouds.
//!
//! Every curve is handled through one parameter `u`: a line or an arc runs
//! from 0 to 1; a polyline from 0 to its number of segments (segment `i`
//! from `i` to `i + 1`); a circle or a full ellipse from 0 to 1 once round
//! (counter-clockwise from angle 0 for circles, from the parameter `start`
//! for ellipses).

use std::f64::consts::{PI, TAU};

use archisim_core::{Point2, Polyline, Shape, Vec2};

use crate::seg::{angle_of, dir, intersect, intersect_extended, norm_angle, Seg, TOL};
use crate::transform::{transform_shape, Transform};
use crate::{ellipse_param, ellipse_points, polyline_from_segs, polyline_segs, CURVE_TOL};

/// Why an edit could not be done.
#[derive(Debug, Clone, PartialEq)]
pub enum EditError {
    /// The command does not apply to this kind of element.
    NotApplicable,
    /// Nothing crosses the element where it was picked.
    NoIntersection,
    /// The lines are parallel.
    Parallel,
    /// The radius or distances are too large for the elements.
    TooLarge,
    /// The result would have no size.
    Degenerate,
}

impl std::fmt::Display for EditError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            EditError::NotApplicable => "no se puede aplicar a este elemento",
            EditError::NoIntersection => "no hay ningún borde que lo corte por ahí",
            EditError::Parallel => "las líneas son paralelas",
            EditError::TooLarge => "el radio o las distancias son demasiado grandes",
            EditError::Degenerate => "el resultado no tendría tamaño",
        })
    }
}

/// A shape for one segment: a line, or a counter-clockwise arc.
pub fn seg_shape(s: &Seg) -> Shape {
    match *s {
        Seg::Line { a, b } => Shape::Line { a, b },
        Seg::Arc {
            centre,
            radius,
            start,
            sweep,
        } => {
            if sweep.abs() >= TAU - 1e-9 {
                Shape::Circle { centre, radius }
            } else if sweep > 0.0 {
                Shape::Arc {
                    centre,
                    radius,
                    start: norm_angle(start),
                    sweep,
                }
            } else {
                Shape::Arc {
                    centre,
                    radius,
                    start: norm_angle(start + sweep),
                    sweep: -sweep,
                }
            }
        }
    }
}

// ---- One parameter along any curve --------------------------------------

/// A curve shape seen through its parameter.
enum Curve {
    Open(Vec<Seg>),
    /// A closed polyline.
    Closed(Vec<Seg>),
    Circle {
        centre: Point2,
        radius: f64,
    },
    Ellipse {
        centre: Point2,
        axis: Vec2,
        ratio: f64,
        start: f64,
        sweep: f64,
    },
}

impl Curve {
    fn of(shape: &Shape) -> Option<Curve> {
        Some(match shape {
            Shape::Line { a, b } => Curve::Open(vec![Seg::Line { a: *a, b: *b }]),
            Shape::Arc {
                centre,
                radius,
                start,
                sweep,
            } => Curve::Open(vec![Seg::Arc {
                centre: *centre,
                radius: *radius,
                start: *start,
                sweep: *sweep,
            }]),
            Shape::Polyline(p) => {
                let segs = polyline_segs(p);
                if segs.is_empty() {
                    return None;
                }
                if p.closed {
                    Curve::Closed(segs)
                } else {
                    Curve::Open(segs)
                }
            }
            Shape::Circle { centre, radius } => Curve::Circle {
                centre: *centre,
                radius: *radius,
            },
            Shape::Ellipse {
                centre,
                axis,
                ratio,
                start,
                sweep,
            } => Curve::Ellipse {
                centre: *centre,
                axis: *axis,
                ratio: *ratio,
                start: *start,
                sweep: *sweep,
            },
            _ => return None,
        })
    }

    fn is_closed(&self) -> bool {
        match self {
            Curve::Open(_) => false,
            Curve::Closed(_) | Curve::Circle { .. } => true,
            Curve::Ellipse { sweep, .. } => *sweep >= TAU - 1e-9,
        }
    }

    /// The parameter range: `[0, end]`.
    fn end(&self) -> f64 {
        match self {
            Curve::Open(s) | Curve::Closed(s) => s.len() as f64,
            _ => 1.0,
        }
    }

    /// Straight or circular pieces to intersect with (ellipses in straight
    /// pieces), each with the parameter range it covers.
    fn pieces(&self) -> Vec<(Seg, f64, f64)> {
        match self {
            Curve::Open(s) | Curve::Closed(s) => s
                .iter()
                .enumerate()
                .map(|(i, s)| (*s, i as f64, i as f64 + 1.0))
                .collect(),
            Curve::Circle { centre, radius } => vec![(
                Seg::Arc {
                    centre: *centre,
                    radius: *radius,
                    start: 0.0,
                    sweep: TAU,
                },
                0.0,
                1.0,
            )],
            Curve::Ellipse { .. } => {
                // Exact parameters are recovered from the points.
                Vec::new()
            }
        }
    }

    /// The parameter of a point on (or near) the curve.
    fn param(&self, p: Point2) -> f64 {
        match self {
            Curve::Open(segs) | Curve::Closed(segs) => {
                let mut best = (f64::INFINITY, 0.0);
                for (i, s) in segs.iter().enumerate() {
                    let (t, q) = s.nearest(p);
                    let d = q.distance(p);
                    if d < best.0 - 1e-12 {
                        best = (d, i as f64 + t);
                    }
                }
                best.1
            }
            Curve::Circle { centre, .. } => norm_angle(angle_of(p - *centre)) / TAU,
            Curve::Ellipse {
                centre,
                axis,
                ratio,
                start,
                sweep,
            } => {
                let t = ellipse_param(*centre, *axis, *ratio, p);
                (t - start).rem_euclid(TAU) / sweep
            }
        }
    }

    /// Parameters where the curve meets the cutting segments.
    fn crossings(&self, cutters: &[Seg]) -> Vec<f64> {
        let mut out = Vec::new();
        match self {
            Curve::Ellipse {
                centre,
                axis,
                ratio,
                start,
                sweep,
            } => {
                let pts = ellipse_points(*centre, *axis, *ratio, *start, *sweep, CURVE_TOL * 0.1);
                for w in pts.windows(2) {
                    let s = Seg::Line { a: w[0], b: w[1] };
                    for c in cutters {
                        for (_, _, p) in intersect(&s, c) {
                            out.push(self.param(p));
                        }
                    }
                }
            }
            _ => {
                for (s, u0, u1) in self.pieces() {
                    for c in cutters {
                        for (t, _, _) in intersect(&s, c) {
                            let u = if s.is_full_circle() {
                                norm_angle(t * TAU) / TAU
                            } else {
                                u0 + t.clamp(0.0, 1.0) * (u1 - u0)
                            };
                            out.push(u);
                        }
                    }
                }
            }
        }
        out.sort_by(f64::total_cmp);
        out.dedup_by(|a, b| (*a - *b).abs() < 1e-9);
        out
    }

    /// The part from `u0` to `u1` (wrapping round a closed curve when
    /// `u1 < u0`).
    fn part(&self, u0: f64, u1: f64, width: f64) -> Option<Shape> {
        match self {
            Curve::Circle { centre, radius } => {
                let mut sweep = (u1 - u0) * TAU;
                if sweep <= 0.0 {
                    sweep += TAU;
                }
                (sweep > 1e-9).then_some(Shape::Arc {
                    centre: *centre,
                    radius: *radius,
                    start: norm_angle(u0 * TAU),
                    sweep: sweep.min(TAU - 1e-6),
                })
            }
            Curve::Ellipse {
                centre,
                axis,
                ratio,
                start,
                sweep,
            } => {
                let mut du = u1 - u0;
                if du <= 0.0 {
                    du += 1.0;
                }
                (du > 1e-9).then_some(Shape::Ellipse {
                    centre: *centre,
                    axis: *axis,
                    ratio: *ratio,
                    start: start + sweep * u0,
                    sweep: (sweep * du).min(TAU),
                })
            }
            Curve::Open(segs) | Curve::Closed(segs) => {
                let n = segs.len() as f64;
                let mut out = Vec::new();
                let span = if u1 >= u0 { u1 - u0 } else { u1 + n - u0 };
                if span < 1e-9 {
                    return None;
                }
                let mut u = u0;
                let end = u0 + span;
                while u < end - 1e-12 {
                    let w = u.rem_euclid(n);
                    let i = (w.floor() as usize).min(segs.len() - 1);
                    let seg_end = (u - w) + i as f64 + 1.0;
                    let stop = seg_end.min(end);
                    let t0 = w - i as f64;
                    let t1 = t0 + (stop - u);
                    let piece = segs[i].sub(t0, t1);
                    if piece.length() > TOL {
                        out.push(piece);
                    }
                    u = stop;
                }
                if out.is_empty() {
                    return None;
                }
                if out.len() == 1 && width == 0.0 {
                    return Some(seg_shape(&out[0]));
                }
                polyline_from_segs(&out, false, width).map(Shape::Polyline)
            }
        }
    }
}

fn width_of(shape: &Shape) -> f64 {
    match shape {
        Shape::Polyline(p) => p.width,
        _ => 0.0,
    }
}

// ---- Trim and break ------------------------------------------------------

/// Trims a shape at the cutting edges around the point it was picked by:
/// the piece between the nearest crossings on either side is removed
/// (AutoCAD's RECORTA). Returns what is left (possibly nothing, when no
/// crossing bounds the picked piece on an open curve that... is never the
/// case: then it is an error).
pub fn trim(shape: &Shape, pick: Point2, cutters: &[Seg]) -> Result<Vec<Shape>, EditError> {
    let curve = Curve::of(shape).ok_or(EditError::NotApplicable)?;
    let cuts = curve.crossings(cutters);
    let up = curve.param(pick);
    let end = curve.end();
    let width = width_of(shape);
    if curve.is_closed() {
        let cuts: Vec<f64> = cuts.into_iter().filter(|u| *u < end - 1e-9).collect();
        if cuts.len() < 2 {
            return Err(EditError::NoIntersection);
        }
        // The crossing before the pick and the one after, round the loop.
        let before = cuts
            .iter()
            .rev()
            .find(|u| **u < up)
            .copied()
            .unwrap_or(*cuts.last().unwrap());
        let after = cuts.iter().find(|u| **u > up).copied().unwrap_or(cuts[0]);
        // What stays runs from `after` round to `before`.
        Ok(curve.part(after, before, width).into_iter().collect())
    } else {
        let inner: Vec<f64> = cuts
            .into_iter()
            .filter(|u| *u > 1e-9 && *u < end - 1e-9)
            .collect();
        if inner.is_empty() {
            return Err(EditError::NoIntersection);
        }
        let a = inner
            .iter()
            .rev()
            .find(|u| **u < up)
            .copied()
            .unwrap_or(0.0);
        let b = inner.iter().find(|u| **u > up).copied().unwrap_or(end);
        let mut out = Vec::new();
        if a > 1e-9 {
            out.extend(curve.part(0.0, a, width));
        }
        if b < end - 1e-9 {
            out.extend(curve.part(b, end, width));
        }
        Ok(out)
    }
}

/// Removes the piece between two points of a shape (AutoCAD's PARTE): on a
/// circle, counter-clockwise from the first to the second. With both
/// points equal, an open shape is split in two there.
pub fn break_between(shape: &Shape, p1: Point2, p2: Point2) -> Result<Vec<Shape>, EditError> {
    let curve = Curve::of(shape).ok_or(EditError::NotApplicable)?;
    let width = width_of(shape);
    let u1 = curve.param(p1);
    let u2 = curve.param(p2);
    let end = curve.end();
    if curve.is_closed() {
        if (u1 - u2).abs() < 1e-9 {
            return Err(EditError::Degenerate);
        }
        // What stays runs from the second point round to the first.
        return Ok(curve.part(u2, u1, width).into_iter().collect());
    }
    let (a, b) = if u1 <= u2 { (u1, u2) } else { (u2, u1) };
    let mut out = Vec::new();
    if a > 1e-9 {
        out.extend(curve.part(0.0, a, width));
    }
    if b < end - 1e-9 {
        out.extend(curve.part(b, end, width));
    }
    Ok(out)
}

// ---- Extend --------------------------------------------------------------

/// Extends the end of a line, arc or open polyline nearest to the pick up
/// to the nearest boundary edge it would meet (AutoCAD's ALARGA).
pub fn extend(shape: &Shape, pick: Point2, boundaries: &[Seg]) -> Result<Shape, EditError> {
    let curve = Curve::of(shape).ok_or(EditError::NotApplicable)?;
    let Curve::Open(mut segs) = curve else {
        return Err(EditError::NotApplicable);
    };
    let up = Curve::Open(segs.clone()).param(pick);
    let at_end = up > segs.len() as f64 / 2.0;
    let idx = if at_end { segs.len() - 1 } else { 0 };
    let s = segs[idx];
    // Look along the segment beyond its end (or before its start).
    let mut best: Option<f64> = None;
    for b in boundaries {
        for (t, tb, _) in intersect_extended(&s, b) {
            if !b.contains_param(tb) {
                continue;
            }
            let t = match s {
                Seg::Line { .. } => t,
                Seg::Arc { sweep, .. } => {
                    // Round the circle forwards from the start (for the
                    // end) or backwards (for the start).
                    let raw = s.param(s.point_at(t));
                    if at_end {
                        raw
                    } else {
                        raw - TAU / sweep.abs()
                    }
                }
            };
            let ok = if at_end { t > 1.0 + 1e-9 } else { t < -1e-9 };
            if ok {
                best = Some(match best {
                    None => t,
                    Some(c) if at_end => c.min(t),
                    Some(c) => c.max(t),
                });
            }
        }
    }
    let t = best.ok_or(EditError::NoIntersection)?;
    if let Seg::Arc { sweep, .. } = s {
        // An arc cannot grow into a full turn.
        let total = if at_end { t } else { 1.0 - t };
        if (sweep * total).abs() >= TAU - 1e-6 {
            return Err(EditError::NoIntersection);
        }
    }
    segs[idx] = if at_end { s.sub(0.0, t) } else { s.sub(t, 1.0) };
    Ok(rebuild_open(shape, &segs))
}

fn rebuild_open(original: &Shape, segs: &[Seg]) -> Shape {
    match original {
        Shape::Polyline(p) => {
            Shape::Polyline(polyline_from_segs(segs, false, p.width).expect("segments"))
        }
        _ => seg_shape(&segs[0]),
    }
}

/// Lengthens (positive) or shortens (negative) the end of a line or arc
/// nearest to the pick by a distance (AutoCAD's LENGTHEN, delta).
pub fn lengthen(shape: &Shape, pick: Point2, delta: f64) -> Result<Shape, EditError> {
    let Some(Curve::Open(mut segs)) = Curve::of(shape) else {
        return Err(EditError::NotApplicable);
    };
    let up = Curve::Open(segs.clone()).param(pick);
    let at_end = up > segs.len() as f64 / 2.0;
    let idx = if at_end { segs.len() - 1 } else { 0 };
    let s = segs[idx];
    let dt = delta / s.length();
    let new = if at_end {
        s.sub(0.0, 1.0 + dt)
    } else {
        s.sub(-dt, 1.0)
    };
    if new.length() < TOL || dt <= -1.0 {
        return Err(EditError::Degenerate);
    }
    if let Seg::Arc { sweep, .. } = new {
        if sweep.abs() >= TAU - 1e-6 {
            return Err(EditError::TooLarge);
        }
    }
    segs[idx] = new;
    Ok(rebuild_open(shape, &segs))
}

// ---- Offset --------------------------------------------------------------

/// Whether `p` is on the left of the segment's direction at its nearest
/// point.
fn left_of(s: &Seg, p: Point2) -> bool {
    let (t, q) = s.nearest(p);
    s.tangent(t).cross(p - q) > 0.0
}

/// A parallel copy at a distance, on the side of `side` (AutoCAD's
/// EQUIDIST). Polylines keep their corners sharp (segments extended or
/// trimmed to meet), rounding them only where the pieces no longer meet.
pub fn offset(shape: &Shape, distance: f64, side: Point2) -> Result<Shape, EditError> {
    if distance <= 0.0 {
        return Err(EditError::Degenerate);
    }
    match shape {
        Shape::Line { a, b } => {
            let s = Seg::Line { a: *a, b: *b };
            let d = if left_of(&s, side) {
                distance
            } else {
                -distance
            };
            Ok(seg_shape(&s.offset(d).ok_or(EditError::Degenerate)?))
        }
        Shape::Circle { centre, radius } => {
            let r = if side.distance(*centre) > *radius {
                radius + distance
            } else {
                radius - distance
            };
            if r <= TOL {
                return Err(EditError::TooLarge);
            }
            Ok(Shape::Circle {
                centre: *centre,
                radius: r,
            })
        }
        Shape::Arc {
            centre,
            radius,
            start,
            sweep,
        } => {
            let r = if side.distance(*centre) > *radius {
                radius + distance
            } else {
                radius - distance
            };
            if r <= TOL {
                return Err(EditError::TooLarge);
            }
            Ok(Shape::Arc {
                centre: *centre,
                radius: r,
                start: *start,
                sweep: *sweep,
            })
        }
        Shape::Polyline(p) => offset_polyline(p, distance, side).map(Shape::Polyline),
        Shape::Ellipse {
            centre,
            axis,
            ratio,
            start,
            sweep,
        } => {
            // Not an ellipse any more: a polyline following it.
            let pts = ellipse_points(*centre, *axis, *ratio, *start, *sweep, CURVE_TOL);
            let closed = *sweep >= TAU - 1e-9;
            let mut pts = pts;
            if closed {
                pts.pop();
            }
            let p = Polyline::straight(pts, closed);
            offset_polyline(&p, distance, side).map(Shape::Polyline)
        }
        _ => Err(EditError::NotApplicable),
    }
}

fn offset_polyline(p: &Polyline, distance: f64, side: Point2) -> Result<Polyline, EditError> {
    let segs = polyline_segs(p);
    if segs.is_empty() {
        return Err(EditError::Degenerate);
    }
    // The side is judged on the segment nearest to the point.
    let near = segs
        .iter()
        .min_by(|a, b| a.distance(side).total_cmp(&b.distance(side)))
        .expect("segments");
    let d = if left_of(near, side) {
        distance
    } else {
        -distance
    };
    let moved: Vec<Seg> = segs
        .iter()
        .map(|s| s.offset(d).ok_or(EditError::TooLarge))
        .collect::<Result<_, _>>()?;
    let n = moved.len();
    let pairs = if p.closed { n } else { n - 1 };
    // Where each pair of neighbours meets: the intersection nearest the
    // original corner, or a rounded join around it.
    let mut out: Vec<Seg> = moved.clone();
    let mut joins: Vec<Option<Seg>> = vec![None; n];
    for i in 0..pairs {
        let j = (i + 1) % n;
        let corner = segs[i].end();
        let (a, b) = (out[i], out[j]);
        if a.end().distance(b.start()) < 1e-9 {
            continue;
        }
        let hits = intersect_extended(&a, &b);
        let best = hits
            .into_iter()
            .min_by(|x, y| x.2.distance(corner).total_cmp(&y.2.distance(corner)));
        match best {
            Some((ta, tb, q)) if q.distance(corner) < distance * 4.0 + 1e-6 => {
                let ta = if let Seg::Arc { .. } = a {
                    a.param_near(q)
                } else {
                    ta
                };
                let tb = if let Seg::Arc { .. } = b {
                    b.param_near(q)
                } else {
                    tb
                };
                out[i] = a.sub(0.0, ta);
                out[j] = b.sub(tb, 1.0);
            }
            _ => {
                // Round the corner with an arc around it.
                let sa = angle_of(a.end() - corner);
                let sb = angle_of(b.start() - corner);
                let mut sweep = (sb - sa).rem_euclid(TAU);
                if sweep > PI {
                    sweep -= TAU;
                }
                joins[i] = Some(Seg::Arc {
                    centre: corner,
                    radius: distance,
                    start: sa,
                    sweep,
                });
            }
        }
    }
    let mut all = Vec::new();
    for i in 0..n {
        if out[i].length() > TOL {
            all.push(out[i]);
        }
        if let Some(j) = joins[i] {
            all.push(j);
        }
    }
    polyline_from_segs(&all, p.closed, p.width).ok_or(EditError::Degenerate)
}

// ---- Fillet and chamfer ----------------------------------------------------

/// A straight line of a shape for fillets: a line, or the segment of a
/// polyline nearest to the pick when it is straight.
fn pick_line(shape: &Shape, _pick: Point2) -> Result<(Point2, Point2), EditError> {
    match shape {
        Shape::Line { a, b } => Ok((*a, *b)),
        _ => Err(EditError::NotApplicable),
    }
}

/// Where two lines meet, and the unit directions from there towards the
/// part of each that is kept (the side of its pick).
fn corner(
    l1: (Point2, Point2),
    pick1: Point2,
    l2: (Point2, Point2),
    pick2: Point2,
) -> Result<(Point2, Vec2, Vec2), EditError> {
    let s1 = Seg::Line { a: l1.0, b: l1.1 };
    let s2 = Seg::Line { a: l2.0, b: l2.1 };
    let x = intersect_extended(&s1, &s2)
        .first()
        .map(|h| h.2)
        .ok_or(EditError::Parallel)?;
    let side = |l: (Point2, Point2), pick: Point2| -> Result<Vec2, EditError> {
        let d = (l.1 - l.0).normalized().ok_or(EditError::Degenerate)?;
        // The pick, or failing that the far end, says which way to keep.
        let s = (pick - x).dot(d);
        let s = if s.abs() > 1e-9 {
            s
        } else {
            let far = if l.0.distance(x) > l.1.distance(x) {
                l.0
            } else {
                l.1
            };
            (far - x).dot(d)
        };
        Ok(if s >= 0.0 { d } else { -d })
    };
    Ok((x, side(l1, pick1)?, side(l2, pick2)?))
}

/// The end of a line kept by a fillet: the one farther along `u` from the
/// corner.
fn kept_end(l: (Point2, Point2), x: Point2, u: Vec2) -> Point2 {
    if (l.0 - x).dot(u) >= (l.1 - x).dot(u) {
        l.0
    } else {
        l.1
    }
}

/// Fillets two lines with an arc of `radius` (0: they are trimmed or
/// extended to meet), keeping the sides they were picked by (AutoCAD's
/// EMPALME). Returns the two lines and the arc, if any.
pub fn fillet(
    a: &Shape,
    pick_a: Point2,
    b: &Shape,
    pick_b: Point2,
    radius: f64,
) -> Result<(Shape, Shape, Option<Shape>), EditError> {
    let l1 = pick_line(a, pick_a)?;
    let l2 = pick_line(b, pick_b)?;
    let (x, u1, u2) = corner(l1, pick_a, l2, pick_b)?;
    let e1 = kept_end(l1, x, u1);
    let e2 = kept_end(l2, x, u2);
    if radius <= 1e-9 {
        let na = Shape::Line { a: e1, b: x };
        let nb = Shape::Line { a: x, b: e2 };
        return Ok((na, nb, None));
    }
    let theta = u1.dot(u2).clamp(-1.0, 1.0).acos();
    if !(1e-6..=PI - 1e-6).contains(&theta) {
        return Err(EditError::Parallel);
    }
    let d = radius / (theta / 2.0).tan();
    let t1 = x + u1 * d;
    let t2 = x + u2 * d;
    if (e1 - x).dot(u1) < d - 1e-9 || (e2 - x).dot(u2) < d - 1e-9 {
        return Err(EditError::TooLarge);
    }
    let bis = (u1 + u2).normalized().ok_or(EditError::Parallel)?;
    let centre = x + bis * (radius / (theta / 2.0).sin());
    let a1 = angle_of(t1 - centre);
    let a2 = angle_of(t2 - centre);
    let mut sweep = (a2 - a1).rem_euclid(TAU);
    let (start, sweep) = if sweep > PI {
        sweep = TAU - sweep;
        (a2, sweep)
    } else {
        (a1, sweep)
    };
    Ok((
        Shape::Line { a: e1, b: t1 },
        Shape::Line { a: t2, b: e2 },
        Some(Shape::Arc {
            centre,
            radius,
            start: norm_angle(start),
            sweep,
        }),
    ))
}

/// Chamfers two lines: cut back `d1` along the first and `d2` along the
/// second from where they meet, joined by a line (AutoCAD's CHAFLAN).
pub fn chamfer(
    a: &Shape,
    pick_a: Point2,
    b: &Shape,
    pick_b: Point2,
    d1: f64,
    d2: f64,
) -> Result<(Shape, Shape, Option<Shape>), EditError> {
    if d1 <= 1e-9 && d2 <= 1e-9 {
        return fillet(a, pick_a, b, pick_b, 0.0);
    }
    let l1 = pick_line(a, pick_a)?;
    let l2 = pick_line(b, pick_b)?;
    let (x, u1, u2) = corner(l1, pick_a, l2, pick_b)?;
    let e1 = kept_end(l1, x, u1);
    let e2 = kept_end(l2, x, u2);
    if (e1 - x).dot(u1) < d1 - 1e-9 || (e2 - x).dot(u2) < d2 - 1e-9 {
        return Err(EditError::TooLarge);
    }
    let c1 = x + u1 * d1;
    let c2 = x + u2 * d2;
    Ok((
        Shape::Line { a: e1, b: c1 },
        Shape::Line { a: c2, b: e2 },
        Some(Shape::Line { a: c1, b: c2 }),
    ))
}

/// Rounds (or chamfers, with `chamfer`) every corner of a polyline between
/// two straight segments (AutoCAD's EMPALME with the Polilínea option).
pub fn fillet_polyline(p: &Polyline, radius: f64, chamfer: bool) -> Result<Polyline, EditError> {
    let segs = polyline_segs(p);
    let n = segs.len();
    if n < 2 {
        return Err(EditError::NotApplicable);
    }
    let corners = if p.closed { n } else { n - 1 };
    // Each segment cut back at its start and end.
    let mut cut0 = vec![0.0; n];
    let mut cut1 = vec![1.0; n];
    let mut extra: Vec<Option<Seg>> = vec![None; n];
    for i in 0..corners {
        let j = (i + 1) % n;
        let (Seg::Line { a: a0, b: x }, Seg::Line { b: b1, .. }) = (segs[i], segs[j]) else {
            continue;
        };
        let u1 = (a0 - x).normalized().ok_or(EditError::Degenerate)?;
        let u2 = (b1 - x).normalized().ok_or(EditError::Degenerate)?;
        let theta = u1.dot(u2).clamp(-1.0, 1.0).acos();
        if !(1e-6..=PI - 1e-6).contains(&theta) {
            continue;
        }
        let d = if chamfer {
            radius
        } else {
            radius / (theta / 2.0).tan()
        };
        let l1 = segs[i].length();
        let l2 = segs[j].length();
        let t1 = 1.0 - d / l1;
        let t2 = d / l2;
        if t1 < cut0[i] - 1e-9 || t2 > cut1[j] + 1e-9 {
            return Err(EditError::TooLarge);
        }
        cut1[i] = t1;
        cut0[j] = t2;
        let p1 = x + u1 * d;
        let p2 = x + u2 * d;
        extra[i] = Some(if chamfer {
            Seg::Line { a: p1, b: p2 }
        } else {
            let bis = (u1 + u2).normalized().ok_or(EditError::Degenerate)?;
            let c = x + bis * (radius / (theta / 2.0).sin());
            let s = angle_of(p1 - c);
            let e = angle_of(p2 - c);
            let mut sw = (e - s).rem_euclid(TAU);
            if sw > PI {
                sw -= TAU;
            }
            Seg::Arc {
                centre: c,
                radius,
                start: s,
                sweep: sw,
            }
        });
    }
    let mut out = Vec::new();
    for i in 0..n {
        let s = segs[i].sub(cut0[i], cut1[i]);
        if s.length() > TOL {
            out.push(s);
        }
        if let Some(e) = extra[i] {
            out.push(e);
        }
    }
    polyline_from_segs(&out, p.closed, p.width).ok_or(EditError::Degenerate)
}

// ---- Join and explode -------------------------------------------------------

/// Joins lines, arcs and open polylines that touch end to end into one
/// polyline (AutoCAD's JUNTAR), starting from the first. Returns the
/// polyline and the indices of the shapes it used.
pub fn join(shapes: &[Shape], tol: f64) -> Result<(Shape, Vec<usize>), EditError> {
    let chains: Vec<Option<Vec<Seg>>> = shapes
        .iter()
        .map(|s| match s {
            Shape::Line { .. } | Shape::Arc { .. } => Some(crate::shape_segs(s)),
            Shape::Polyline(p) if !p.closed => Some(polyline_segs(p)),
            _ => None,
        })
        .collect();
    let first = chains
        .iter()
        .position(|c| c.as_ref().is_some_and(|c| !c.is_empty()))
        .ok_or(EditError::NotApplicable)?;
    let mut chain: Vec<Seg> = chains[first].clone().unwrap_or_default();
    let mut used = vec![first];
    let rev = |c: &[Seg]| -> Vec<Seg> { c.iter().rev().map(Seg::reversed).collect() };
    loop {
        let head = chain[0].start();
        let tail = chain[chain.len() - 1].end();
        if head.distance(tail) <= tol && chain.len() > 1 {
            break;
        }
        let mut grown = false;
        for (i, c) in chains.iter().enumerate() {
            if used.contains(&i) {
                continue;
            }
            let Some(c) = c else { continue };
            if c.is_empty() {
                continue;
            }
            let (cs, ce) = (c[0].start(), c[c.len() - 1].end());
            if cs.distance(tail) <= tol {
                chain.extend_from_slice(c);
            } else if ce.distance(tail) <= tol {
                chain.extend(rev(c));
            } else if ce.distance(head) <= tol {
                let mut v = c.clone();
                v.extend(chain);
                chain = v;
            } else if cs.distance(head) <= tol {
                let mut v = rev(c);
                v.extend(chain);
                chain = v;
            } else {
                continue;
            }
            used.push(i);
            grown = true;
            break;
        }
        if !grown {
            break;
        }
    }
    if used.len() < 2 {
        return Err(EditError::NoIntersection);
    }
    // Snap the joints exactly together.
    let closed = chain.len() > 1 && chain[0].start().distance(chain[chain.len() - 1].end()) <= tol;
    let width = shapes
        .iter()
        .find_map(|s| match s {
            Shape::Polyline(p) => Some(p.width),
            _ => None,
        })
        .unwrap_or(0.0);
    let p = polyline_from_segs(&chain, closed, width).ok_or(EditError::Degenerate)?;
    Ok((Shape::Polyline(p), used))
}

/// Breaks a polyline into lines and arcs, a hatch into its boundaries, a
/// rectangle into its sides (AutoCAD's DESCOMP).
pub fn explode(shape: &Shape) -> Result<Vec<Shape>, EditError> {
    match shape {
        Shape::Polyline(p) => Ok(polyline_segs(p).iter().map(seg_shape).collect()),
        Shape::Hatch(h) => Ok(h.loops.iter().cloned().map(Shape::Polyline).collect()),
        _ => Err(EditError::NotApplicable),
    }
}

// ---- Arrays and stretch -----------------------------------------------------

/// Copies in rows and columns (AutoCAD's MATRIZ rectangular), not
/// counting the original: `rows × cols − 1` copies of each shape.
pub fn array_rect(shapes: &[Shape], rows: usize, cols: usize, dy: f64, dx: f64) -> Vec<Shape> {
    let mut out = Vec::new();
    for r in 0..rows.max(1) {
        for c in 0..cols.max(1) {
            if r == 0 && c == 0 {
                continue;
            }
            let t = Transform::translate(Vec2::new(dx * c as f64, dy * r as f64));
            out.extend(shapes.iter().map(|s| transform_shape(s, &t)));
        }
    }
    out
}

/// Copies round a centre (AutoCAD's MATRIZ polar): `count` items in all
/// (the original included) over `fill` radians (2π: a full turn, evenly
/// spaced), turned with the array or not.
pub fn array_polar(
    shapes: &[Shape],
    centre: Point2,
    count: usize,
    fill: f64,
    rotate: bool,
) -> Vec<Shape> {
    let n = count.max(1);
    let full = (fill.abs() - TAU).abs() < 1e-9;
    let step = if full {
        fill / n as f64
    } else if n > 1 {
        fill / (n - 1) as f64
    } else {
        0.0
    };
    let mut out = Vec::new();
    for i in 1..n {
        let a = step * i as f64;
        for s in shapes {
            if rotate {
                out.push(transform_shape(s, &Transform::rotate(centre, a)));
            } else {
                // Only the position turns: move by the turn of a key point.
                let key = crate::snaps::snap_points(s)
                    .first()
                    .map(|p| p.0)
                    .unwrap_or(centre);
                let moved = Transform::rotate(centre, a).apply(key);
                out.push(transform_shape(s, &Transform::translate(moved - key)));
            }
        }
    }
    out
}

/// Stretches a shape: the points inside the window move by `d`, the rest
/// stay (AutoCAD's ESTIRA, with a crossing window). A shape entirely inside
/// moves whole.
pub fn stretch(shape: &Shape, lo: Point2, hi: Point2, d: Vec2) -> Shape {
    let inside = |p: Point2| p.x >= lo.x && p.x <= hi.x && p.y >= lo.y && p.y <= hi.y;
    let mv = |p: Point2| if inside(p) { p + d } else { p };
    match shape {
        Shape::Line { a, b } => Shape::Line {
            a: mv(*a),
            b: mv(*b),
        },
        Shape::Polyline(p) => Shape::Polyline(Polyline {
            points: p.points.iter().map(|q| mv(*q)).collect(),
            ..p.clone()
        }),
        Shape::Arc {
            centre,
            radius,
            start,
            sweep,
        } => {
            let s = Seg::Arc {
                centre: *centre,
                radius: *radius,
                start: *start,
                sweep: *sweep,
            };
            let (a, b) = (s.start(), s.end());
            match (inside(a), inside(b)) {
                (true, true) => transform_shape(shape, &Transform::translate(d)),
                (false, false) => shape.clone(),
                _ => {
                    // Keep the bulge between the ends that are left.
                    seg_shape(&Seg::from_bulge(mv(a), mv(b), s.bulge()))
                }
            }
        }
        Shape::Circle { centre, .. } | Shape::Ellipse { centre, .. } => {
            if inside(*centre) {
                transform_shape(shape, &Transform::translate(d))
            } else {
                shape.clone()
            }
        }
        Shape::Point { at } => Shape::Point { at: mv(*at) },
        Shape::Text(t) => {
            if inside(t.at) {
                transform_shape(shape, &Transform::translate(d))
            } else {
                shape.clone()
            }
        }
        Shape::Hatch(h) => Shape::Hatch(archisim_core::Hatch {
            loops: h
                .loops
                .iter()
                .map(|l| Polyline {
                    points: l.points.iter().map(|q| mv(*q)).collect(),
                    ..l.clone()
                })
                .collect(),
            ..h.clone()
        }),
    }
}

// ---- Building shapes ------------------------------------------------------

/// A rectangle by two opposite corners, turned by `angle` about the first.
pub fn rectangle(a: Point2, b: Point2, angle: f64) -> Polyline {
    let u = dir(angle);
    let v = u.perp_left();
    let d = b - a;
    let w = d.dot(u);
    let h = d.dot(v);
    let pts = vec![a, a + u * w, a + u * w + v * h, a + v * h];
    // Counter-clockwise always.
    let mut pts = pts;
    if w * h < 0.0 {
        pts.reverse();
    }
    Polyline::straight(pts, true)
}

/// A regular polygon of `sides` around `centre`, with one vertex (or, if
/// circumscribed, the middle of one side) at `towards`.
pub fn polygon(centre: Point2, towards: Point2, sides: usize, inscribed: bool) -> Option<Polyline> {
    let n = sides.max(3);
    let r = centre.distance(towards);
    if r < TOL {
        return None;
    }
    let a0 = angle_of(towards - centre);
    let step = TAU / n as f64;
    let (r, a0) = if inscribed {
        (r, a0)
    } else {
        (r / (step / 2.0).cos(), a0 + step / 2.0)
    };
    Some(Polyline::straight(
        (0..n)
            .map(|i| centre + dir(a0 + step * i as f64) * r)
            .collect(),
        true,
    ))
}

/// A revision cloud along a polygon: arcs of about `arc` length bulging
/// outwards (AutoCAD's NUBEREV).
pub fn revision_cloud(outline: &[Point2], arc: f64) -> Option<Polyline> {
    if outline.len() < 3 || arc <= TOL {
        return None;
    }
    let ccw = archisim_core::signed_area(outline) > 0.0;
    let mut points = Vec::new();
    let n = outline.len();
    for i in 0..n {
        let (a, b) = (outline[i], outline[(i + 1) % n]);
        let l = a.distance(b);
        let k = (l / arc).round().max(1.0) as usize;
        for j in 0..k {
            points.push(a + (b - a) * (j as f64 / k as f64));
        }
    }
    // Bulging out: on a counter-clockwise outline, counter-clockwise arcs
    // bow out to the right of travel.
    let bulge = if ccw { 0.5 } else { -0.5 };
    let m = points.len();
    Some(Polyline {
        points,
        bulges: vec![bulge; m],
        closed: true,
        width: 0.0,
    })
}

/// Points dividing a shape into `n` equal parts (AutoCAD's DIVIDE): `n − 1`
/// points on an open shape, `n` on a closed one.
pub fn divide(shape: &Shape, n: usize) -> Vec<Point2> {
    let total = crate::length(shape);
    if n < 2 || total <= TOL {
        return Vec::new();
    }
    let closed = Curve::of(shape).is_some_and(|c| c.is_closed());
    let count = if closed { n } else { n - 1 };
    (0..count)
        .filter_map(|i| {
            let s = total * (i + usize::from(!closed)) as f64 / n as f64;
            point_at_length(shape, s)
        })
        .collect()
}

/// The point at a length along a shape.
pub fn point_at_length(shape: &Shape, s: f64) -> Option<Point2> {
    let segs = crate::shape_segs(shape);
    let mut left = s;
    for seg in &segs {
        let l = seg.length();
        if left <= l + 1e-12 {
            return Some(seg.point_at((left / l).clamp(0.0, 1.0)));
        }
        left -= l;
    }
    segs.last().map(|s| s.end())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::f64::consts::FRAC_PI_2;

    fn p(x: f64, y: f64) -> Point2 {
        Point2::new(x, y)
    }

    fn line(a: Point2, b: Point2) -> Shape {
        Shape::Line { a, b }
    }

    fn cut(a: Point2, b: Point2) -> Seg {
        Seg::Line { a, b }
    }

    #[test]
    fn trim_a_line_between_two_edges() {
        let l = line(p(0.0, 0.0), p(10.0, 0.0));
        let edges = [
            cut(p(3.0, -1.0), p(3.0, 1.0)),
            cut(p(7.0, -1.0), p(7.0, 1.0)),
        ];
        let out = trim(&l, p(5.0, 0.0), &edges).unwrap();
        assert_eq!(
            out,
            vec![
                line(p(0.0, 0.0), p(3.0, 0.0)),
                line(p(7.0, 0.0), p(10.0, 0.0))
            ]
        );
        // Picked beyond the last edge: only the end goes.
        let out = trim(&l, p(9.0, 0.0), &edges).unwrap();
        assert_eq!(out, vec![line(p(0.0, 0.0), p(7.0, 0.0))]);
        assert_eq!(trim(&l, p(5.0, 0.0), &[]), Err(EditError::NoIntersection));
    }

    #[test]
    fn trim_a_circle_leaves_an_arc() {
        let c = Shape::Circle {
            centre: p(0.0, 0.0),
            radius: 1.0,
        };
        let edge = [cut(p(0.0, -2.0), p(0.0, 2.0))];
        let out = trim(&c, p(1.0, 0.0), &edge).unwrap();
        let Shape::Arc { start, sweep, .. } = out[0] else {
            panic!("{out:?}")
        };
        // The right half goes: the left half from +y round to −y stays.
        assert!((start - FRAC_PI_2).abs() < 1e-9, "{start}");
        assert!((sweep - PI).abs() < 1e-9);
    }

    #[test]
    fn trim_a_polyline_and_a_closed_one() {
        let pl = Shape::Polyline(Polyline::straight(
            vec![p(0.0, 0.0), p(4.0, 0.0), p(4.0, 4.0)],
            false,
        ));
        let edge = [cut(p(2.0, -1.0), p(2.0, 1.0))];
        let out = trim(&pl, p(1.0, 0.0), &edge).unwrap();
        let Shape::Polyline(q) = &out[0] else {
            panic!()
        };
        assert_eq!(q.points, vec![p(2.0, 0.0), p(4.0, 0.0), p(4.0, 4.0)]);
        let sq = Shape::Polyline(Polyline::straight(
            vec![p(0.0, 0.0), p(4.0, 0.0), p(4.0, 4.0), p(0.0, 4.0)],
            true,
        ));
        let edge = [cut(p(2.0, -1.0), p(2.0, 5.0))];
        let out = trim(&sq, p(4.0, 2.0), &edge).unwrap();
        let Shape::Polyline(q) = &out[0] else {
            panic!()
        };
        // The left half, open: from the top cut round to the bottom one.
        assert_eq!(
            q.points,
            vec![p(2.0, 4.0), p(0.0, 4.0), p(0.0, 0.0), p(2.0, 0.0)]
        );
        assert!(!q.closed);
    }

    #[test]
    fn extend_lines_and_arcs() {
        let l = line(p(0.0, 0.0), p(2.0, 0.0));
        let wall = [
            cut(p(5.0, -1.0), p(5.0, 1.0)),
            cut(p(-3.0, -1.0), p(-3.0, 1.0)),
        ];
        assert_eq!(
            extend(&l, p(1.9, 0.0), &wall).unwrap(),
            line(p(0.0, 0.0), p(5.0, 0.0))
        );
        assert_eq!(
            extend(&l, p(0.1, 0.0), &wall).unwrap(),
            line(p(-3.0, 0.0), p(2.0, 0.0))
        );
        // A quarter arc extended to the x axis below its start.
        let arc = Shape::Arc {
            centre: p(0.0, 0.0),
            radius: 1.0,
            start: 0.0,
            sweep: FRAC_PI_2,
        };
        let edge = [cut(p(-2.0, 0.0), p(2.0, 0.0))];
        let Shape::Arc { start, sweep, .. } = extend(&arc, p(0.0, 1.0), &edge).unwrap() else {
            panic!()
        };
        assert!(
            start.abs() < 1e-9 && (sweep - PI).abs() < 1e-9,
            "{start} {sweep}"
        );
        assert_eq!(extend(&l, p(1.9, 0.0), &[]), Err(EditError::NoIntersection));
    }

    #[test]
    fn offset_lines_circles_and_polylines() {
        let l = line(p(0.0, 0.0), p(4.0, 0.0));
        assert_eq!(
            offset(&l, 1.0, p(2.0, 3.0)).unwrap(),
            line(p(0.0, 1.0), p(4.0, 1.0))
        );
        assert_eq!(
            offset(&l, 1.0, p(2.0, -3.0)).unwrap(),
            line(p(0.0, -1.0), p(4.0, -1.0))
        );
        let c = Shape::Circle {
            centre: p(0.0, 0.0),
            radius: 2.0,
        };
        assert!(
            matches!(offset(&c, 0.5, p(0.1, 0.0)).unwrap(), Shape::Circle { radius, .. } if (radius - 1.5).abs() < 1e-12)
        );
        // A square offset outwards keeps square corners.
        let sq = Shape::Polyline(Polyline::straight(
            vec![p(0.0, 0.0), p(4.0, 0.0), p(4.0, 4.0), p(0.0, 4.0)],
            true,
        ));
        let Shape::Polyline(o) = offset(&sq, 1.0, p(10.0, 2.0)).unwrap() else {
            panic!()
        };
        assert_eq!(o.points.len(), 4);
        assert!((crate::polyline_area(&o).abs() - 36.0).abs() < 1e-9);
        // Inwards.
        let Shape::Polyline(i) = offset(&sq, 1.0, p(2.0, 2.0)).unwrap() else {
            panic!()
        };
        assert!((crate::polyline_area(&i).abs() - 4.0).abs() < 1e-9);
        // An L: the outer corner stays sharp.
        let lshape = Shape::Polyline(Polyline::straight(
            vec![p(0.0, 0.0), p(4.0, 0.0), p(4.0, 3.0)],
            false,
        ));
        let Shape::Polyline(o) = offset(&lshape, 0.5, p(2.0, -1.0)).unwrap() else {
            panic!()
        };
        assert!(
            o.points.iter().any(|q| q.approx_eq(p(4.5, -0.5))),
            "{:?}",
            o.points
        );
    }

    #[test]
    fn fillet_and_chamfer_two_lines() {
        let a = line(p(0.0, 0.0), p(5.0, 0.0));
        let b = line(p(6.0, 1.0), p(6.0, 5.0));
        // Radius 0: they meet at (6, 0).
        let (na, nb, arc) = fillet(&a, p(1.0, 0.0), &b, p(6.0, 4.0), 0.0).unwrap();
        assert_eq!(na, line(p(0.0, 0.0), p(6.0, 0.0)));
        assert_eq!(nb, line(p(6.0, 0.0), p(6.0, 5.0)));
        assert!(arc.is_none());
        let (na, nb, arc) = fillet(&a, p(1.0, 0.0), &b, p(6.0, 4.0), 1.0).unwrap();
        let close = |s: &Shape, a: Point2, b: Point2| matches!(s, Shape::Line { a: x, b: y } if x.approx_eq(a) && y.approx_eq(b));
        assert!(close(&na, p(0.0, 0.0), p(5.0, 0.0)), "{na:?}");
        assert!(close(&nb, p(6.0, 1.0), p(6.0, 5.0)), "{nb:?}");
        let Some(Shape::Arc {
            centre,
            radius,
            sweep,
            ..
        }) = arc
        else {
            panic!()
        };
        assert!(centre.approx_eq(p(5.0, 1.0)) && radius == 1.0 && (sweep - FRAC_PI_2).abs() < 1e-9);
        let (_, _, l) = chamfer(&a, p(1.0, 0.0), &b, p(6.0, 4.0), 1.0, 2.0).unwrap();
        assert!(close(&l.unwrap(), p(5.0, 0.0), p(6.0, 2.0)));
        let par = line(p(0.0, 1.0), p(5.0, 1.0));
        assert_eq!(
            fillet(&a, p(1.0, 0.0), &par, p(1.0, 1.0), 1.0),
            Err(EditError::Parallel)
        );
    }

    #[test]
    fn fillet_every_corner_of_a_polyline() {
        let sq = Polyline::straight(
            vec![p(0.0, 0.0), p(4.0, 0.0), p(4.0, 4.0), p(0.0, 4.0)],
            true,
        );
        let f = fillet_polyline(&sq, 1.0, false).unwrap();
        // Four sides and four arcs; the area loses (4 − π) × 1².
        assert_eq!(f.points.len(), 8);
        assert!((crate::polyline_area(&f) - (16.0 - (4.0 - PI))).abs() < 1e-9);
        let c = fillet_polyline(&sq, 1.0, true).unwrap();
        assert!((crate::polyline_area(&c) - (16.0 - 2.0)).abs() < 1e-9);
    }

    #[test]
    fn join_and_explode() {
        let shapes = vec![
            line(p(0.0, 0.0), p(2.0, 0.0)),
            line(p(2.0, 2.0), p(2.0, 0.0)),
            Shape::Circle {
                centre: p(9.0, 9.0),
                radius: 1.0,
            },
            line(p(0.0, 0.0), p(2.0, 2.0)),
        ];
        let (j, used) = join(&shapes, 1e-6).unwrap();
        assert_eq!(used.len(), 3);
        let Shape::Polyline(pl) = &j else { panic!() };
        assert!(pl.closed);
        assert_eq!(explode(&j).unwrap().len(), 3);
    }

    #[test]
    fn break_and_arrays_and_stretch() {
        let l = line(p(0.0, 0.0), p(10.0, 0.0));
        let out = break_between(&l, p(2.0, 0.0), p(5.0, 0.0)).unwrap();
        assert_eq!(
            out,
            vec![
                line(p(0.0, 0.0), p(2.0, 0.0)),
                line(p(5.0, 0.0), p(10.0, 0.0))
            ]
        );
        let out = break_between(&l, p(4.0, 0.0), p(4.0, 0.0)).unwrap();
        assert_eq!(out.len(), 2);
        let copies = array_rect(std::slice::from_ref(&l), 2, 3, 1.0, 2.0);
        assert_eq!(copies.len(), 5);
        let polar = array_polar(&[line(p(1.0, 0.0), p(2.0, 0.0))], p(0.0, 0.0), 4, TAU, true);
        assert_eq!(polar.len(), 3);
        assert!(matches!(polar[0], Shape::Line { a, .. } if a.approx_eq(p(0.0, 1.0))));
        let s = stretch(&l, p(8.0, -1.0), p(12.0, 1.0), p(3.0, 0.0));
        assert_eq!(s, line(p(0.0, 0.0), p(13.0, 0.0)));
        assert_eq!(
            lengthen(&l, p(9.0, 0.0), -2.0).unwrap(),
            line(p(0.0, 0.0), p(8.0, 0.0))
        );
        assert_eq!(divide(&l, 4).len(), 3);
    }

    #[test]
    fn rectangles_polygons_and_clouds() {
        let r = rectangle(p(0.0, 0.0), p(4.0, -2.0), 0.0);
        assert!(archisim_core::signed_area(&r.points) > 0.0);
        let hex = polygon(p(0.0, 0.0), p(1.0, 0.0), 6, true).unwrap();
        assert!(hex.points[0].approx_eq(p(1.0, 0.0)));
        let sq = polygon(p(0.0, 0.0), p(1.0, 0.0), 4, false).unwrap();
        // Circumscribed: the side's middle at distance 1.
        assert!((sq.points[0].distance(p(0.0, 0.0)) - 2f64.sqrt()).abs() < 1e-9);
        let cloud =
            revision_cloud(&[p(0.0, 0.0), p(4.0, 0.0), p(4.0, 2.0), p(0.0, 2.0)], 1.0).unwrap();
        assert_eq!(cloud.points.len(), 12);
        assert!(crate::polyline_area(&cloud) > 8.0);
    }
}
