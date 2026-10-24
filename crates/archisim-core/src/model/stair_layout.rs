//! How the treads of a stair are laid out in plan, for every form:
//! - flights and landings ("escalera de tramos y mesetas"): straight
//!   flights along the walking line, a landing at every turn;
//! - winders ("peldaños compensados"): one run along the walking line,
//!   the turns made with treads that fan round the inner corner, spread
//!   over the treads before and after it ("compensados") so that none is
//!   left without depth on its narrow side;
//! - spiral ("de caracol"): treads round a centre, from a newel or an
//!   eye of the radius asked.
//!
//! Every form gives the same thing: its treads ([`Step`]) and landings
//! in walking order, the height of every tread and the lines of the
//! nosings along each side and along the middle ([`SidePoint`]), from
//! which the 3D, the plan, the railings and the IFC are drawn.
//!
//! The walking line of flights and winders is the middle of the stair.
//! The going of a curved tread is measured, as the code asks (CTE DB-SUA
//! 1, 4.2), 50 cm from its narrow side (on its axis when it is narrower
//! than 1 m in a dwelling).

use super::*;

/// One tread, seen from above: its nosing ("front"), the edge under the
/// next nosing ("back"), and the corners of its sides between them.
#[derive(Debug, Clone, PartialEq)]
pub struct Step {
    /// The nosing, its left end and its right end (walking up).
    pub front: [Point2; 2],
    /// The back edge, left end and right end.
    pub back: [Point2; 2],
    /// Corners of the left side between front and back, front first.
    pub left: Vec<Point2>,
    /// Corners of the right side between front and back, front first.
    pub right: Vec<Point2>,
    /// The middle of the stair across it, from the middle of the front to
    /// the middle of the back.
    pub centre: Vec<Point2>,
    /// Height of its top above the level.
    pub top: f64,
    /// Its depth along the middle of the stair.
    pub going: f64,
    /// Its front and back are not parallel (a winder or a spiral tread).
    pub turned: bool,
}

/// The fraction of the way from `a` to `b` along `a`, `corners`, `b` at
/// which each corner is.
fn fractions(a: Point2, corners: &[Point2], b: Point2) -> Vec<f64> {
    let mut d = Vec::with_capacity(corners.len());
    let mut acc = 0.0;
    let mut prev = a;
    for c in corners {
        acc += prev.distance(*c);
        d.push(acc);
        prev = *c;
    }
    let total = acc + prev.distance(b);
    d.into_iter()
        .map(|x| if total > EPSILON { x / total } else { 0.5 })
        .collect()
}

impl Step {
    /// One side, from the front to the back: its ends and corners.
    pub fn side(&self, left: bool) -> Vec<Point2> {
        let i = usize::from(!left);
        let corners = if left { &self.left } else { &self.right };
        let mut v = vec![self.front[i]];
        v.extend(corners.iter().copied());
        v.push(self.back[i]);
        v
    }

    /// The length of a side, from front to back.
    pub fn side_length(&self, left: bool) -> f64 {
        self.side(left)
            .windows(2)
            .map(|w| w[0].distance(w[1]))
            .sum()
    }

    /// Its outline, counter-clockwise, each corner with how far it is
    /// from the front (0) to the back (1) along its side: where the line
    /// of the nosings passes over it.
    pub fn outline_rising(&self) -> Vec<(Point2, f64)> {
        let mut v = vec![(self.front[1], 0.0)];
        let rf = fractions(self.front[1], &self.right, self.back[1]);
        v.extend(self.right.iter().copied().zip(rf));
        v.push((self.back[1], 1.0));
        v.push((self.back[0], 1.0));
        let lf = fractions(self.front[0], &self.left, self.back[0]);
        let left: Vec<(Point2, f64)> = self.left.iter().copied().zip(lf).collect();
        v.extend(left.into_iter().rev());
        v.push((self.front[0], 0.0));
        v.dedup_by(|a, b| a.0.approx_eq(b.0));
        while v.len() > 1 && v[0].0.approx_eq(v[v.len() - 1].0) {
            v.pop();
        }
        let pts: Vec<Point2> = v.iter().map(|x| x.0).collect();
        if signed_area(&pts) < 0.0 {
            v.reverse();
        }
        v
    }

    /// Its outline, counter-clockwise.
    pub fn outline(&self) -> Vec<Point2> {
        self.outline_rising().into_iter().map(|x| x.0).collect()
    }

    /// The middle of its nosing.
    pub fn nosing_middle(&self) -> Point2 {
        (self.front[0] + self.front[1]) * 0.5
    }
}

/// A point of the line of the nosings along a side (or the middle) of a
/// stair: where it is, how high the line passes over it, and the height
/// of what one walks on from it to the next point (a tread or a landing).
/// Heights above the level.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SidePoint {
    pub p: Point2,
    pub z: f64,
    pub floor: f64,
}

/// A run of treads, ended by one more riser up to a landing or the floor
/// above.
#[derive(Debug, Clone, PartialEq)]
pub struct Run {
    /// The treads of the run, as indices of [`StairLayout::steps`].
    pub steps: std::ops::Range<usize>,
    /// Height of its foot above the level.
    pub base: f64,
}

/// A polyline measured along its length.
pub(crate) struct Poly {
    pub pts: Vec<Point2>,
    cum: Vec<f64>,
}

impl Poly {
    pub fn new(mut pts: Vec<Point2>) -> Poly {
        pts.dedup_by(|a, b| a.approx_eq(*b));
        let mut cum = vec![0.0];
        for w in pts.windows(2) {
            let last = *cum.last().unwrap_or(&0.0);
            cum.push(last + w[0].distance(w[1]));
        }
        Poly { pts, cum }
    }

    pub fn len(&self) -> f64 {
        *self.cum.last().unwrap_or(&0.0)
    }

    fn segment(&self, t: f64) -> usize {
        let n = self.pts.len();
        if n < 2 {
            return 0;
        }
        (0..n - 1)
            .find(|&i| t <= self.cum[i + 1] + 1e-12)
            .unwrap_or(n - 2)
    }

    pub fn at(&self, t: f64) -> Point2 {
        if self.pts.len() < 2 {
            return self.pts.first().copied().unwrap_or(Point2::new(0.0, 0.0));
        }
        let i = self.segment(t);
        let (a, b) = (self.pts[i], self.pts[i + 1]);
        let len = self.cum[i + 1] - self.cum[i];
        let f = if len > EPSILON {
            ((t - self.cum[i]) / len).clamp(0.0, 1.0)
        } else {
            0.0
        };
        a + (b - a) * f
    }

    pub fn dir_at(&self, t: f64) -> Vec2 {
        if self.pts.len() < 2 {
            return Vec2::new(1.0, 0.0);
        }
        let i = self.segment(t);
        (self.pts[i + 1] - self.pts[i])
            .normalized()
            .unwrap_or(Vec2::new(1.0, 0.0))
    }

    /// Where `p` is along the line (its nearest point), looking from `from`
    /// on.
    pub fn param(&self, p: Point2, from: f64) -> f64 {
        let n = self.pts.len();
        if n < 2 {
            return 0.0;
        }
        let mut best = (f64::INFINITY, from);
        for i in 0..n - 1 {
            if self.cum[i + 1] < from - 1e-7 {
                continue;
            }
            let (a, b) = (self.pts[i], self.pts[i + 1]);
            let ab = b - a;
            let len2 = ab.dot(ab);
            let f = if len2 > 0.0 {
                ((p - a).dot(ab) / len2).clamp(0.0, 1.0)
            } else {
                0.0
            };
            let d = (a + ab * f).distance(p);
            let t = self.cum[i] + f * len2.sqrt();
            if d < best.0 - 1e-9 {
                best = (d, t.max(from));
            }
        }
        best.1
    }

    /// The corners strictly between `t0` and `t1`.
    pub fn between(&self, t0: f64, t1: f64) -> Vec<Point2> {
        let e = 1e-7;
        self.pts
            .iter()
            .zip(&self.cum)
            .filter(|(_, c)| **c > t0 + e && **c < t1 - e)
            .map(|(p, _)| *p)
            .collect()
    }

    /// The first point where the ray from `o` along `d` crosses the line
    /// further than `min` from `o`, and where it is along the line.
    fn hit(&self, o: Point2, d: Vec2, min: f64) -> Option<(Point2, f64)> {
        let mut best: Option<(f64, Point2, f64)> = None;
        for i in 0..self.pts.len().saturating_sub(1) {
            let (a, b) = (self.pts[i], self.pts[i + 1]);
            let e = b - a;
            let den = d.cross(e);
            if den.abs() < 1e-12 {
                continue;
            }
            let w = a - o;
            let lambda = w.cross(e) / den;
            let f = w.cross(d) / den;
            if lambda > min && (-1e-9..=1.0 + 1e-9).contains(&f) {
                let t = self.cum[i] + f.clamp(0.0, 1.0) * e.length();
                if best.is_none_or(|x| lambda < x.0) {
                    best = Some((lambda, o + d * lambda, t));
                }
            }
        }
        best.map(|(_, p, t)| (p, t))
    }
}

/// A stretch of the walking line where it turns, with winders: from `a`
/// to `b` along it, the inner side to the left or the right.
#[derive(Debug, Clone, Copy)]
struct Zone {
    a: f64,
    b: f64,
    left_inner: bool,
}

impl StairLayout {
    /// The treads.
    pub fn treads(&self) -> usize {
        self.steps.len()
    }

    /// The risers: one more than the treads in every run.
    pub fn risers(&self) -> usize {
        self.steps.len() + self.runs.len()
    }

    /// The shallowest tread, on the walking line (50 cm from the narrow
    /// side of curved treads, as the code measures them).
    pub fn going(&self) -> f64 {
        self.steps
            .iter()
            .map(|s| s.going)
            .fold(f64::INFINITY, f64::min)
    }

    /// The line of the nosings along a side, from the foot to the head:
    /// over the treads it rises a riser per tread; over the landings it
    /// is level with them.
    pub fn side(&self, left: bool) -> Vec<SidePoint> {
        self.line_along(|s| s.side(left), |l| if left { &l.left } else { &l.right })
    }

    /// The same along the middle of the stair.
    pub fn centre(&self) -> Vec<SidePoint> {
        self.line_along(|s| s.centre.clone(), |l| &l.centre)
    }

    fn line_along(
        &self,
        of_step: impl Fn(&Step) -> Vec<Point2>,
        of_landing: impl Fn(&Landing) -> &Vec<Point2>,
    ) -> Vec<SidePoint> {
        let mut out: Vec<SidePoint> = Vec::new();
        let mut push = |p: Point2, z: f64, floor: f64| match out.last_mut() {
            Some(last) if last.p.approx_eq(p) => *last = SidePoint { p, z, floor },
            _ => out.push(SidePoint { p, z, floor }),
        };
        for (ri, run) in self.runs.iter().enumerate() {
            for st in &self.steps[run.steps.clone()] {
                let pts = of_step(st);
                let (Some(a), Some(b)) = (pts.first(), pts.last()) else {
                    continue;
                };
                let f = fractions(*a, &pts[1..pts.len() - 1], *b);
                push(*a, st.top, st.top);
                for (p, f) in pts[1..pts.len() - 1].iter().zip(f) {
                    push(*p, st.top + f * self.riser, st.top);
                }
                push(*b, st.top + self.riser, st.top);
            }
            if ri + 1 < self.runs.len() {
                if let Some(l) = self.landings.get(ri) {
                    for p in of_landing(l) {
                        push(*p, l.top, l.top);
                    }
                }
            }
        }
        out
    }
}

/// The shortest piece of a turn's landing (`width` across the stair): a
/// piece of the walking line between two square turns the same way,
/// shorter than the stair is wide, is the landing of a U ("ojo").
fn across_piece(u0: Vec2, u1: Vec2, u2: Vec2) -> bool {
    let (a, b) = (u0.cross(u1), u1.cross(u2));
    a * b > 0.0 && u0.dot(u1).abs() < 0.2 && u1.dot(u2).abs() < 0.2
}

/// Lays out a stair of flights and landings along a walking line.
pub fn layout(path: &[Point2], width: f64, going: f64, height: f64, base: f64) -> StairLayout {
    let k = path.len();
    let half = width / 2.0;
    // Each piece of the walking line, shortened by half the width at the
    // turns (the landing takes it); those still long enough for a tread
    // are flights, the others belong to the landing.
    struct Piece {
        start: Point2,
        end: Point2,
        u: Vec2,
        flight: bool,
        /// Index of the path's point it starts from.
        from: usize,
    }
    let mut pieces: Vec<Piece> = Vec::new();
    for i in 0..k.saturating_sub(1) {
        let (a, b) = (path[i], path[i + 1]);
        let Some(u) = (b - a).normalized() else {
            continue;
        };
        let start = if i > 0 { a + u * half } else { a };
        let end = if i + 2 < k { b - u * half } else { b };
        let long = (end - start).dot(u) >= going * 0.999;
        pieces.push(Piece {
            start,
            end,
            u,
            flight: long,
            from: i,
        });
    }
    // The short piece across a U is part of its landing.
    for i in 1..pieces.len().saturating_sub(1) {
        let len = (pieces[i].end - pieces[i].start).dot(pieces[i].u);
        if len < width && across_piece(pieces[i - 1].u, pieces[i].u, pieces[i + 1].u) {
            pieces[i].flight = false;
        }
    }
    // Treads per flight, and the risers: one more than the treads per
    // flight (the last rises onto the landing or the floor).
    let mut flights: Vec<Flight> = pieces
        .iter()
        .filter(|p| p.flight)
        .map(|p| {
            let len = p.start.distance(p.end);
            let treads = ((len / going) + 1e-6).floor().max(1.0) as usize;
            Flight {
                start: p.start,
                end: p.end,
                treads,
                risers: treads + 1,
                going: len / treads as f64,
                base: 0.0,
            }
        })
        .collect();
    let risers: usize = flights.iter().map(|f| f.risers).sum();
    let riser = if risers > 0 {
        height / risers as f64
    } else {
        0.0
    };
    let mut z = base;
    for f in &mut flights {
        f.base = z;
        z += f.risers as f64 * riser;
    }
    // The treads of each flight.
    let mut steps = Vec::new();
    let mut runs = Vec::new();
    for f in &flights {
        let u = f.direction();
        let n = u.perp_left() * half;
        let first = steps.len();
        for i in 0..f.treads {
            let a = f.start + u * (i as f64 * f.going);
            let b = a + u * f.going;
            steps.push(Step {
                front: [a + n, a - n],
                back: [b + n, b - n],
                left: Vec::new(),
                right: Vec::new(),
                centre: vec![a, b],
                top: f.base + (i + 1) as f64 * riser,
                going: f.going,
                turned: false,
            });
        }
        runs.push(Run {
            steps: first..steps.len(),
            base: f.base,
        });
    }
    // Landings: between consecutive flights, everything the walking line
    // turns through, between its two sides.
    let mut landings = Vec::new();
    let (left, right) = (
        Poly::new(offset_path(path, half).unwrap_or_default()),
        Poly::new(offset_path(path, -half).unwrap_or_default()),
    );
    let flight_pieces: Vec<usize> = (0..pieces.len()).filter(|i| pieces[*i].flight).collect();
    for (fi, w) in flight_pieces.windows(2).enumerate() {
        let (i, j) = (w[0], w[1]);
        let (pi, pj) = (&pieces[i], &pieces[j]);
        let (ni, nj) = (pi.u.perp_left() * half, pj.u.perp_left() * half);
        let chain = |poly: &Poly, a: Point2, b: Point2| {
            let ta = poly.param(a, 0.0);
            let tb = poly.param(b, ta);
            let mut v = vec![a];
            v.extend(poly.between(ta, tb));
            v.push(b);
            v.dedup_by(|a, b| a.approx_eq(*b));
            v
        };
        let l = chain(&left, pi.end + ni, pj.start + nj);
        let r = chain(&right, pi.end - ni, pj.start - nj);
        let mut c = vec![pi.end];
        c.extend(path[pi.from + 1..=pj.from].iter().copied());
        c.push(pj.start);
        c.dedup_by(|a, b| a.approx_eq(*b));
        let mut outline: Vec<Point2> = r.clone();
        outline.extend(l.iter().rev().copied());
        outline.dedup_by(|a, b| a.approx_eq(*b));
        while outline.len() > 1 && outline[0].approx_eq(outline[outline.len() - 1]) {
            outline.pop();
        }
        if signed_area(&outline) < 0.0 {
            outline.reverse();
        }
        landings.push(Landing {
            outline,
            top: flights[fi + 1].base,
            left: l,
            right: r,
            centre: c,
        });
    }
    StairLayout {
        flights,
        landings,
        steps,
        riser,
        runs,
        walk: path.to_vec(),
    }
}

/// A polyline moved `d` to its left (negative: to its right), rounded
/// round the corners it passes outside of (as the walking line of a
/// winder, at a fixed distance from the inner corner).
fn round_offset(pts: &[Point2], d: f64) -> Vec<Point2> {
    let mut pts = pts.to_vec();
    pts.dedup_by(|a, b| a.approx_eq(*b));
    let n = pts.len();
    if n < 2 {
        return pts;
    }
    let dirs: Vec<Vec2> = pts
        .windows(2)
        .map(|w| (w[1] - w[0]).normalized().unwrap_or(Vec2::new(1.0, 0.0)))
        .collect();
    let mut out = vec![pts[0] + dirs[0].perp_left() * d];
    for j in 1..n - 1 {
        let (u1, u2) = (dirs[j - 1], dirs[j]);
        let (n1, n2) = (u1.perp_left(), u2.perp_left());
        let turn = u1.cross(u2);
        let angle = turn.atan2(u1.dot(u2));
        if turn * d < 0.0 {
            let v = n1 * d;
            let a1 = v.y.atan2(v.x);
            let r = d.abs();
            let steps = (angle.abs() / 8f64.to_radians()).ceil().max(1.0) as usize;
            for i in 0..=steps {
                let a = a1 + angle * i as f64 / steps as f64;
                out.push(pts[j] + Vec2::new(a.cos(), a.sin()) * r);
            }
        } else {
            let m = (n1 + n2).normalized().unwrap_or(n1);
            let cos = m.dot(n1).max(0.2);
            out.push(pts[j] + m * (d / cos));
        }
    }
    out.push(pts[n - 1] + dirs[n - 2].perp_left() * d);
    out
}

/// How a walking line with winders is laid out: its sides, its turns and
/// the walking line the treads are measured on, stretch by stretch.
struct WinderPlan {
    centre: Poly,
    left: Poly,
    right: Poly,
    zones: Vec<Zone>,
    /// Where each turn's ends are along its inner side.
    inner_ends: Vec<(f64, f64)>,
    /// Where each turn's ends are along its outer side.
    outer_ends: Vec<(f64, f64)>,
    /// Each turn's walking line.
    walks: Vec<Poly>,
    /// The stretches of the walking line: where each starts along it, its
    /// length, its turn (or none: straight along the middle from `c0`).
    stretches: Vec<(f64, f64, Option<usize>, f64)>,
    total: f64,
}

fn winder_plan(path: &[Point2], width: f64, edge: f64, going: f64) -> Option<WinderPlan> {
    let half = width / 2.0;
    let centre = Poly::new(path.to_vec());
    let length = centre.len();
    if centre.pts.len() < 2 || length < EPSILON {
        return None;
    }
    let left = Poly::new(offset_path(&centre.pts, half)?);
    let right = Poly::new(offset_path(&centre.pts, -half)?);
    // The turns, each the stretch of the middle its square takes, joined
    // when they are close and turn the same way (a U).
    let pts = &centre.pts;
    let mut zones: Vec<Zone> = Vec::new();
    for j in 1..pts.len().saturating_sub(1) {
        let (Some(u0), Some(u1)) = (
            (pts[j] - pts[j - 1]).normalized(),
            (pts[j + 1] - pts[j]).normalized(),
        ) else {
            continue;
        };
        let turn = u0.cross(u1);
        if turn.abs() < 0.05 && u0.dot(u1) > 0.0 {
            continue;
        }
        let s = centre.cum[j];
        let z = Zone {
            a: (s - half).max(0.0),
            b: (s + half).min(length),
            left_inner: turn > 0.0,
        };
        match zones.last_mut() {
            Some(last) if last.left_inner == z.left_inner && z.a - last.b < 2.0 * going => {
                last.b = z.b;
            }
            Some(last) if z.a < last.b => {
                let m = (z.a + last.b) / 2.0;
                last.b = m;
                zones.push(Zone { a: m, ..z });
            }
            _ => zones.push(z),
        }
    }
    // Spread each turn over up to four treads before and after it.
    let spread = 4.0 * going;
    let bounds: Vec<(f64, f64)> = zones.iter().map(|z| (z.a, z.b)).collect();
    for (i, z) in zones.iter_mut().enumerate() {
        let before = if i == 0 {
            bounds[i].0
        } else {
            (bounds[i].0 - bounds[i - 1].1) / 2.0
        };
        let after = if i + 1 == bounds.len() {
            length - bounds[i].1
        } else {
            (bounds[i + 1].0 - bounds[i].1) / 2.0
        };
        z.a -= spread.min(before.max(0.0));
        z.b += spread.min(after.max(0.0));
    }
    // Where each zone's ends are on its inner side, and its walking line,
    // `edge` from the inner side, round the inner corners.
    let square = |s: f64, side: f64| centre.at(s) + centre.dir_at(s).perp_left() * (side * half);
    let d = edge.clamp(0.01, width);
    let mut inner_ends = Vec::new();
    let mut outer_ends = Vec::new();
    let mut walks = Vec::new();
    for z in &zones {
        let (poly, other, side) = if z.left_inner {
            (&left, &right, 1.0)
        } else {
            (&right, &left, -1.0)
        };
        let ia = poly.param(square(z.a + 1e-9, side), 0.0);
        let ib = poly.param(square(z.b - 1e-9, side), ia);
        let oa = other.param(square(z.a + 1e-9, -side), 0.0);
        let ob = other.param(square(z.b - 1e-9, -side), oa);
        outer_ends.push((oa, ob));
        let mut chain = vec![poly.at(ia)];
        chain.extend(poly.between(ia, ib));
        chain.push(poly.at(ib));
        walks.push(Poly::new(round_offset(&chain, -side * d)));
        inner_ends.push((ia, ib));
    }
    let mut stretches = Vec::new();
    let (mut at, mut c) = (0.0, 0.0);
    for (i, z) in zones.iter().enumerate() {
        if z.a > c + 1e-9 {
            stretches.push((at, z.a - c, None, c));
            at += z.a - c;
        }
        let len = walks[i].len();
        stretches.push((at, len, Some(i), z.a));
        at += len;
        c = z.b;
    }
    if length > c + 1e-9 {
        stretches.push((at, length - c, None, c));
        at += length - c;
    }
    Some(WinderPlan {
        centre,
        left,
        right,
        zones,
        inner_ends,
        outer_ends,
        walks,
        stretches,
        total: at,
    })
}

/// The length of the walking line of a stair with winders: along its
/// middle where it is straight, `edge` from the inner side round its
/// turns.
pub fn walking_length(path: &[Point2], width: f64, edge: f64, going: f64) -> f64 {
    winder_plan(path, width, edge, going).map_or(0.0, |p| p.total)
}

/// Lays out a stair of one run along a walking line, its turns made with
/// winders spread over two treads before and after each turn; the treads
/// all the same depth on the walking line, `edge` from the inner side of
/// the turns (CTE: 50 cm).
pub fn winder_layout(
    path: &[Point2],
    width: f64,
    going: f64,
    height: f64,
    base: f64,
    edge: f64,
) -> StairLayout {
    let half = width / 2.0;
    let empty = StairLayout {
        flights: Vec::new(),
        landings: Vec::new(),
        steps: Vec::new(),
        riser: 0.0,
        runs: Vec::new(),
        walk: path.to_vec(),
    };
    let Some(plan) = winder_plan(path, width, edge, going) else {
        return empty;
    };
    if plan.total < going * 0.999 {
        return empty;
    }
    let WinderPlan {
        centre,
        left,
        right,
        zones,
        inner_ends,
        outer_ends,
        walks,
        stretches,
        total,
    } = plan;
    let treads = ((total / going) + 1e-6).floor().max(1.0) as usize;
    let g = total / treads as f64;
    let riser = height / (treads + 1) as f64;
    // The risers: across the stair where it is straight; in a turn, by the
    // walking line, spread evenly along it and along the outer side (the
    // inner side takes what is left) or, if that crosses them on the inner
    // side, from points of the inner side spread evenly.
    let risers_with = |even_outside: bool| {
        let mut lines: Vec<([Point2; 2], [f64; 2], f64)> = Vec::new();
        let (mut tl, mut tr, mut tc) = (0.0, 0.0, 0.0);
        for k in 0..=treads {
            let u = (k as f64 * g).min(total);
            let &(u0, len, zone, c0) = stretches
                .iter()
                .find(|x| u <= x.0 + x.1 + 1e-9)
                .unwrap_or(&stretches[stretches.len() - 1]);
            let ends = match zone {
                Some(zi) => {
                    let z = zones[zi];
                    let (ia, ib) = inner_ends[zi];
                    let (oa, ob) = outer_ends[zi];
                    let (inner, outer) = if z.left_inner {
                        (&left, &right)
                    } else {
                        (&right, &left)
                    };
                    let f = ((u - u0) / len.max(EPSILON)).clamp(0.0, 1.0);
                    let p = walks[zi].at(f * walks[zi].len());
                    let across = centre.dir_at(z.a).perp_left();
                    let (q, o) = if even_outside {
                        let o = outer.at(oa + f * (ob - oa));
                        let d = (p - o).normalized().unwrap_or(across);
                        let q = inner
                            .hit(o, d, o.distance(p) * 0.5)
                            .map(|x| x.0)
                            .unwrap_or_else(|| inner.at(ia + f * (ib - ia)));
                        (q, o)
                    } else {
                        let q = inner.at(ia + f * (ib - ia));
                        let d = (p - q).normalized().unwrap_or(across);
                        let o = outer
                            .hit(q, d, q.distance(p) * 0.5)
                            .map(|x| x.0)
                            .unwrap_or(p + d * half);
                        (q, o)
                    };
                    if z.left_inner {
                        [q, o]
                    } else {
                        [o, q]
                    }
                }
                None => {
                    let c = (c0 + (u - u0)).min(centre.len());
                    let p = centre.at(c);
                    let n = centre.dir_at(c).perp_left() * half;
                    [p + n, p - n]
                }
            };
            let (pl, pr) = (left.param(ends[0], 0.0), right.param(ends[1], 0.0));
            let ok = k == 0 || (pl >= tl + 0.02 && pr >= tr + 0.02);
            tl = left.param(ends[0], tl);
            tr = right.param(ends[1], tr);
            tc = centre.param((ends[0] + ends[1]) * 0.5, tc);
            lines.push((ends, [tl, tr], tc));
            if !ok && even_outside {
                return None;
            }
        }
        Some(lines)
    };
    let Some(lines) = risers_with(true).or_else(|| risers_with(false)) else {
        return empty;
    };
    let mut steps = Vec::with_capacity(treads);
    for k in 0..treads {
        let (f, tf, cf) = lines[k];
        let (b, tb, cb) = lines[k + 1];
        let mut c = vec![(f[0] + f[1]) * 0.5];
        c.extend(centre.between(cf, cb));
        c.push((b[0] + b[1]) * 0.5);
        c.dedup_by(|a, b| a.approx_eq(*b));
        let (uf, ub) = (f[1] - f[0], b[1] - b[0]);
        let turned = uf
            .normalized()
            .zip(ub.normalized())
            .is_some_and(|(x, y)| x.cross(y).abs() > 1e-4);
        steps.push(Step {
            front: f,
            back: b,
            left: left.between(tf[0], tb[0]),
            right: right.between(tf[1], tb[1]),
            centre: c,
            top: base + (k + 1) as f64 * riser,
            going: g,
            turned,
        });
    }
    StairLayout {
        flights: Vec::new(),
        landings: Vec::new(),
        steps,
        riser,
        runs: vec![Run {
            steps: 0..treads,
            base,
        }],
        walk: path.to_vec(),
    }
}

/// How far from the narrow side of a curved tread the code measures its
/// going: 50 cm, or the middle of a dwelling's stair narrower than 1 m.
pub fn walking_edge(usage: StairUse, width: f64) -> f64 {
    match usage {
        StairUse::Restricted if width < 1.0 => width / 2.0,
        _ => 0.5,
    }
}

/// The radius of a spiral stair's walking line: `edge` from its inner
/// edge.
pub fn spiral_walking_radius(core: f64, width: f64, edge: f64) -> f64 {
    core.max(0.0) + edge.min(width)
}

/// Points of the arc of `radius` round `c` strictly between two angles.
fn arc_between(c: Point2, radius: f64, a0: f64, a1: f64) -> Vec<Point2> {
    let n = ((a1 - a0).abs() / 8f64.to_radians()).ceil().max(1.0) as usize;
    (1..n)
        .map(|i| {
            let a = a0 + (a1 - a0) * i as f64 / n as f64;
            c + Vec2::new(a.cos(), a.sin()) * radius
        })
        .collect()
}

/// Lays out a spiral stair round `centre`, its first riser in the
/// direction of `start`, turning `sweep` (counter-clockwise if positive)
/// from its first riser to its last, its treads from `core` to `core +
/// width` from the centre.
#[allow(clippy::too_many_arguments)]
pub fn spiral_layout(
    centre: Point2,
    start: Vec2,
    sweep: f64,
    core: f64,
    width: f64,
    going: f64,
    height: f64,
    base: f64,
    edge: f64,
) -> StairLayout {
    let rw = spiral_walking_radius(core, width, edge);
    let a0 = start.y.atan2(start.x);
    let treads = ((sweep.abs() * rw / going) + 1e-6).floor().max(1.0) as usize;
    let phi = sweep / treads as f64;
    let riser = height / (treads + 1) as f64;
    let ccw = sweep >= 0.0;
    let (ri, ro, rc) = (
        core.max(0.0),
        core.max(0.0) + width,
        core.max(0.0) + width / 2.0,
    );
    let (rl, rr) = if ccw { (ri, ro) } else { (ro, ri) };
    let at = |a: f64, r: f64| centre + Vec2::new(a.cos(), a.sin()) * r;
    let mut steps = Vec::with_capacity(treads);
    for k in 0..treads {
        let (af, ab) = (a0 + k as f64 * phi, a0 + (k + 1) as f64 * phi);
        let mut c = vec![at(af, rc)];
        c.extend(arc_between(centre, rc, af, ab));
        c.push(at(ab, rc));
        steps.push(Step {
            front: [at(af, rl), at(af, rr)],
            back: [at(ab, rl), at(ab, rr)],
            left: arc_between(centre, rl, af, ab),
            right: arc_between(centre, rr, af, ab),
            centre: c,
            top: base + (k + 1) as f64 * riser,
            going: phi.abs() * rw,
            turned: true,
        });
    }
    let mut walk = vec![at(a0, rw)];
    walk.extend(arc_between(centre, rw, a0, a0 + sweep));
    walk.push(at(a0 + sweep, rw));
    StairLayout {
        flights: Vec::new(),
        landings: Vec::new(),
        steps,
        riser,
        runs: vec![Run {
            steps: 0..treads,
            base,
        }],
        walk,
    }
}

/// The ready-made shapes the stair tool offers: drawn by their foot (the
/// centre of a spiral) and the direction they set off in, their length
/// worked out from the height to climb.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum StairPreset {
    /// Any walking line, point by point.
    Free,
    #[default]
    Straight,
    /// Two flights at right angles ("en L").
    LShape,
    /// Two flights side by side, there and back ("en U", "de ida y vuelta").
    UShape,
    /// Round a newel ("de caracol").
    Spiral,
}

/// What a ready-made stair needs to be laid out.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PresetSpec {
    pub preset: StairPreset,
    /// The turns with winders instead of landings.
    pub winders: bool,
    /// Turning left (counter-clockwise) walking up.
    pub left: bool,
    pub width: f64,
    pub going: f64,
    /// The riser height aimed at; the risers are as many as needed not
    /// to pass it.
    pub riser: f64,
    pub height: f64,
    /// The gap between the flights of a U ("ojo").
    pub gap: f64,
    /// The radius of a spiral's newel or eye.
    pub core: f64,
    /// Where the going of curved treads is measured, from their narrow
    /// side ([`walking_edge`]).
    pub edge: f64,
}

/// A ready-made stair from its foot and the direction it sets off in:
/// its walking line, form and (for a spiral) the angle it turns. `None`
/// if it does not fit (a U too low for its width, say).
pub fn preset_stair(
    spec: &PresetSpec,
    foot: Point2,
    dir: Vec2,
) -> Option<(Vec<Point2>, StairForm, f64)> {
    let u = dir.normalized()?;
    let v = if spec.left {
        u.perp_left()
    } else {
        -u.perp_left()
    };
    let (w, g, h) = (spec.width, spec.going, spec.width / 2.0);
    if spec.riser <= EPSILON || spec.height <= EPSILON || g <= EPSILON {
        return None;
    }
    let n = ((spec.height / spec.riser) - 1e-9).ceil().max(2.0) as usize;
    let t = (n - 1) as f64;
    let form = if spec.winders {
        StairForm::Winders
    } else {
        StairForm::Landings
    };
    // Two flights with a landing: as many risers in the first as in the
    // second, or one more.
    let r1 = n.div_ceil(2);
    let (t1, t2) = (r1 as f64 - 1.0, (n - r1) as f64 - 1.0);
    match spec.preset {
        StairPreset::Free => None,
        StairPreset::Straight => Some((vec![foot, foot + u * (t * g)], form, 0.0)),
        StairPreset::LShape if spec.winders => {
            // Legs as long as the walking line round the corner needs.
            let make = |leg: f64| {
                let p1 = foot + u * leg;
                vec![foot, p1, p1 + v * leg]
            };
            let leg = fit(t * g, t * g / 2.0, 2.0, |leg| {
                walking_length(&make(leg), w, spec.edge, g)
            })?;
            (leg >= h).then(|| (make(leg), form, 0.0))
        }
        StairPreset::LShape => (t2 >= 1.0).then(|| {
            let p1 = foot + u * (t1 * g + h);
            (vec![foot, p1, p1 + v * (t2 * g + h)], form, 0.0)
        }),
        StairPreset::UShape => {
            let across = w + spec.gap.max(0.0);
            if spec.winders {
                let make = |leg: f64| {
                    let p1 = foot + u * leg;
                    let p2 = p1 + v * across;
                    vec![foot, p1, p2, p2 - u * leg]
                };
                let leg = fit(t * g, (t * g - across) / 2.0, 2.0, |leg| {
                    walking_length(&make(leg), w, spec.edge, g)
                })?;
                (leg >= h).then(|| (make(leg), form, 0.0))
            } else {
                (t2 >= 1.0).then(|| {
                    let p1 = foot + u * (t1 * g + h);
                    let p2 = p1 + v * across;
                    (vec![foot, p1, p2, p2 - u * (t2 * g + h)], form, 0.0)
                })
            }
        }
        StairPreset::Spiral => {
            let rw = spiral_walking_radius(spec.core, w, spec.edge);
            let sweep = t * g / rw * if spec.left { 1.0 } else { -1.0 };
            let rc = spec.core.max(0.0) + h;
            Some((vec![foot, foot + u * rc], StairForm::Spiral, sweep))
        }
    }
}

/// The length of the legs of a stair whose walking line, `length(leg)`
/// long, must be `target` long (a hair more, so that all its treads fit):
/// from a first guess, each leg changing as much as `per` legs would.
fn fit(target: f64, guess: f64, per: f64, length: impl Fn(f64) -> f64) -> Option<f64> {
    let goal = target + 1e-4;
    let mut leg = guess;
    for _ in 0..6 {
        let miss = goal - length(leg);
        if miss.abs() < 1e-7 {
            break;
        }
        leg += miss / per;
    }
    (leg.is_finite() && leg > 0.0).then_some(leg)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn spec(preset: StairPreset, winders: bool) -> PresetSpec {
        PresetSpec {
            preset,
            winders,
            left: true,
            width: 1.0,
            going: 0.28,
            riser: 0.175,
            height: 2.97,
            gap: 0.1,
            core: 0.1,
            edge: 0.5,
        }
    }

    fn check(l: &StairLayout) {
        // Every tread a proper outline, the treads one after the other,
        // their tops a riser apart within a run.
        for st in &l.steps {
            let o = st.outline();
            assert!(o.len() >= 3);
            assert!(signed_area(&o) > 1e-4, "{o:?}");
        }
        for run in &l.runs {
            for k in run.steps.start + 1..run.steps.end {
                let (a, b) = (&l.steps[k - 1], &l.steps[k]);
                assert!((b.top - a.top - l.riser).abs() < 1e-9);
                assert!(a.back[0].approx_eq(b.front[0]) && a.back[1].approx_eq(b.front[1]));
            }
        }
    }

    /// The ready-made stairs climb 2,97 m in 17 risers of 17,47 cm.
    #[test]
    fn ready_made_stairs() {
        for (preset, winders, runs) in [
            (StairPreset::Straight, false, 1),
            (StairPreset::LShape, false, 2),
            (StairPreset::UShape, false, 2),
            (StairPreset::LShape, true, 1),
            (StairPreset::UShape, true, 1),
            (StairPreset::Spiral, false, 1),
        ] {
            let s = spec(preset, winders);
            let (path, form, sweep) =
                preset_stair(&s, Point2::new(0.0, 0.0), Vec2::new(1.0, 0.0)).unwrap();
            let l = match form {
                StairForm::Landings => layout(&path, 1.0, 0.28, 2.97, 0.0),
                StairForm::Winders => winder_layout(&path, 1.0, 0.28, 2.97, 0.0, 0.5),
                StairForm::Spiral => spiral_layout(
                    path[0],
                    path[1] - path[0],
                    sweep,
                    0.1,
                    1.0,
                    0.28,
                    2.97,
                    0.0,
                    0.5,
                ),
            };
            assert_eq!(l.risers(), 17, "{preset:?} {winders}");
            assert_eq!(l.runs.len(), runs, "{preset:?} {winders}");
            assert!((l.riser - 2.97 / 17.0).abs() < 1e-9);
            check(&l);
            // The line of the nosings along each side rises from the
            // first tread to the floor above.
            for left in [true, false] {
                let side = l.side(left);
                assert!((side[0].z - l.riser).abs() < 1e-9);
                assert!((side.last().unwrap().z - 2.97).abs() < 1e-9, "{preset:?}");
                assert!(side.windows(2).all(|w| w[1].z >= w[0].z - 1e-9));
            }
        }
    }

    /// The winders of an L: one run round the corner, the narrow side of
    /// none left without depth, the treads by the corner turned.
    #[test]
    fn winders_round_a_corner() {
        let s = spec(StairPreset::LShape, true);
        let (path, ..) = preset_stair(&s, Point2::new(0.0, 0.0), Vec2::new(1.0, 0.0)).unwrap();
        let l = winder_layout(&path, 1.0, 0.28, 2.97, 0.0, 0.5);
        let turned: Vec<&Step> = l.steps.iter().filter(|s| s.turned).collect();
        assert!(turned.len() >= 4, "{}", turned.len());
        for st in &turned {
            // The inner side: at least 5 cm; the outer: at most 44 cm.
            let (a, b) = (st.side_length(true), st.side_length(false));
            assert!(a.min(b) > 0.05, "{a} {b}");
            assert!(a.max(b) < 0.6, "{a} {b}");
        }
        // All the same depth on the walking line, 28 cm as asked.
        assert!(l.steps.iter().all(|s| (s.going - 0.28).abs() < 1e-3));
        // All the stair is covered: as much as an L of 1 m.
        let area: f64 = l.steps.iter().map(|s| signed_area(&s.outline())).sum();
        let leg = path[0].distance(path[1]);
        let expected = 2.0 * (leg - 0.5) + 1.0;
        assert!((area - expected).abs() < 1e-6, "{area} {expected}");
    }

    /// A spiral of 1 m wide treads round a 10 cm newel turns, for 16
    /// treads of 28 cm at 50 cm from the newel, a little over 2 turns.
    #[test]
    fn a_spiral() {
        let s = spec(StairPreset::Spiral, false);
        let (path, form, sweep) =
            preset_stair(&s, Point2::new(2.0, 2.0), Vec2::new(0.0, 1.0)).unwrap();
        assert_eq!(form, StairForm::Spiral);
        assert!((sweep - 16.0 * 0.28 / 0.6).abs() < 1e-9);
        let l = spiral_layout(
            path[0],
            path[1] - path[0],
            sweep,
            0.1,
            1.0,
            0.28,
            2.97,
            0.0,
            0.5,
        );
        assert_eq!(l.treads(), 16);
        assert!((l.going() - 0.28).abs() < 1e-6, "{}", l.going());
        // Its treads ring the centre.
        for st in &l.steps {
            assert!((st.front[0].distance(path[0]) - 0.1).abs() < 1e-9);
            assert!((st.front[1].distance(path[0]) - 1.1).abs() < 1e-9);
        }
    }
}
