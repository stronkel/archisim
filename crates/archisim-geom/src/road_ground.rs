//! How a street meets the ground when its grade does not follow it (as a
//! city builder does): the ground itself is shaped round it — lowered
//! under it, and meeting its edges in smooth slopes of fill (3 across, 2
//! down) or of cut (1 to 1) — and, where it runs high over the ground, a
//! viaduct on piers with its deck and parapets. Worked out from the
//! street's profile; none of it is stored.

use archisim_core::{MaterialId, Model, Point2, Road, RoadId};

use crate::solid::{prism, Solid, SolidOwner, P3};

/// Spacing of the stations along the street, metres.
pub const STEP: f64 = 4.0;
/// Over the ground by more than this, the street is a viaduct (the
/// ground is not raised to it).
pub const VIADUCT_FROM: f64 = 6.0;
/// The slope of an embankment: across per metre down.
pub const FILL_SLOPE: f64 = 1.5;
/// The slope of a cut: across per metre up.
pub const CUT_SLOPE: f64 = 1.0;
/// How far from a street's edge the ground is shaped, metres.
pub const REACH: f64 = 30.0;
/// The longest side of the ground's triangles near a street, metres.
pub const FINE_EDGE: f64 = 2.0;
/// Spacing of the piers of a viaduct, metres.
pub const PIER_SPAN: f64 = 24.0;
/// The depth of the deck of a viaduct under the street's pavement.
pub const DECK: f64 = 1.1;

/// What one station of a street sees.
#[derive(Debug, Clone, Copy)]
struct Station {
    left: Point2,
    right: Point2,
    out_left: Point2,
    out_right: Point2,
    /// The top of the street (its sidewalks) and the ground at each edge.
    top: f64,
    ground_left: Option<f64>,
    ground_right: Option<f64>,
    ground_mid: Option<f64>,
}

fn ground_at(model: &Model, r: &Road, p: Point2) -> Option<f64> {
    let base = model.level(r.level).map_or(0.0, |l| l.elevation);
    model
        .terrains_on_level(r.level)
        .find_map(|t| t.height_at(p))
        .map(|z| z + base)
}

fn stations(model: &Model, r: &Road) -> Vec<Station> {
    let base = model.level(r.level).map_or(0.0, |l| l.elevation);
    let len = r.length();
    let n = (len / STEP).ceil().max(1.0) as usize;
    let kh = if r.section.single_level {
        0.0
    } else {
        r.section.kerb_height
    };
    (0..=n)
        .map(|k| {
            let s = len * k as f64 / n as f64;
            let (p, dir) = r.axis_at(s);
            let nrm = dir.perp_left();
            let (l, rr) = r.extent_at(s);
            let left = p + nrm * l;
            let right = p - nrm * rr;
            Station {
                left,
                right,
                out_left: left + nrm,
                out_right: right - nrm,
                top: base + r.top_at(s) + kh,
                ground_left: ground_at(model, r, left),
                ground_right: ground_at(model, r, right),
                ground_mid: ground_at(model, r, p),
            }
        })
        .collect()
}

/// A material of the model of one of these categories, or `or`.
fn material_of(model: &Model, cats: &[&str], or: MaterialId) -> MaterialId {
    cats.iter()
        .find_map(|c| model.materials().find(|m| m.category == *c))
        .map_or(or, |m| m.id)
}

/// What a street needs where it runs high over the ground: its viaduct,
/// deck, piers and parapets (the rest of the ground is shaped to it,
/// [`Shaper`]).
pub fn road_ground_solids(model: &Model, id: RoadId) -> Vec<Solid> {
    let Some(r) = model.road(id) else {
        return Vec::new();
    };
    if !r.sloped() || model.terrains_on_level(r.level).next().is_none() {
        return Vec::new();
    }
    let st = stations(model, r);
    if st.len() < 2 {
        return Vec::new();
    }
    let concrete = material_of(model, &["concrete"], r.kerb_material);
    let metal = material_of(model, &["metal", "steel"], concrete);
    let fill_of = |x: &Station, g: Option<f64>| g.map_or(0.0, |g| x.top - g);
    let viaduct = |x: &Station| {
        fill_of(x, x.ground_mid).max(fill_of(x, x.ground_left).min(fill_of(x, x.ground_right)))
            > VIADUCT_FROM
    };
    let mut walls: Vec<[P3; 3]> = Vec::new();
    let mut deck: Vec<[P3; 3]> = Vec::new();
    let mut rails: Vec<[P3; 3]> = Vec::new();
    let depth = r.section.depth;
    for w in st.windows(2) {
        let (a, b) = (&w[0], &w[1]);
        let high = viaduct(a) || viaduct(b);
        for left in [true, false] {
            let (ea, eb) = if left {
                (a.left, b.left)
            } else {
                (a.right, b.right)
            };
            let (oa, ob) = if left {
                (a.out_left, b.out_left)
            } else {
                (a.out_right, b.out_right)
            };
            if high {
                // A parapet along the deck's edge: a low wall and a rail.
                let (ia, ib) = (ea + (oa - ea) * -0.25, eb + (ob - eb) * -0.25);
                walls.extend(strip_wall(ia, ib, a.top, b.top, 0.25, 0.75));
                rails.extend(strip_wall(ia, ib, a.top + 0.75, b.top + 0.75, 0.06, 0.3));
            }
        }
        if high {
            // The deck under the pavement, between the edges.
            let poly = [a.right, b.right, b.left, a.left];
            deck.extend(between(
                poly[0],
                poly[1],
                poly[2],
                poly[3],
                a.top - depth - DECK,
                b.top - depth - DECK,
                a.top - depth,
                b.top - depth,
            ));
        }
    }
    // The piers of the viaducts: one each span, under the middle.
    let len = r.length();
    let mut s = PIER_SPAN / 2.0;
    while s < len {
        let i = ((s / len) * (st.len() - 1) as f64).round() as usize;
        let x = &st[i.min(st.len() - 1)];
        if viaduct(x) {
            let (p, dir) = r.axis_at(s);
            let nrm = dir.perp_left();
            let half_w = (x.left.distance(x.right) * 0.3).max(1.0);
            let bottom = x.ground_mid.unwrap_or(x.top - 10.0) - 0.5;
            let top = x.top - depth - DECK;
            let poly = [
                p - nrm * half_w - dir * 0.6,
                p - nrm * half_w + dir * 0.6,
                p + nrm * half_w + dir * 0.6,
                p + nrm * half_w - dir * 0.6,
            ];
            deck.extend(prism(&poly, bottom, top));
        }
        s += PIER_SPAN;
    }
    let mut out = Vec::new();
    for (material, triangles) in [(concrete, walls), (concrete, deck), (metal, rails)] {
        if !triangles.is_empty() {
            out.push(Solid {
                owner: SolidOwner::Road(id),
                material,
                triangles,
            });
        }
    }
    out
}

/// A thin wall standing on the line `a b` (its bottom at `za`, `zb`),
/// `thick` across and `high` tall.
fn strip_wall(a: Point2, b: Point2, za: f64, zb: f64, thick: f64, high: f64) -> Vec<[P3; 3]> {
    let Some(d) = (b - a).normalized() else {
        return Vec::new();
    };
    let n = d.perp_left() * (thick / 2.0);
    between(a - n, b - n, b + n, a + n, za, zb, za + high, zb + high)
}

/// A closed solid over the quad `p0 p1 p2 p3` (p0–p3 at station a, p1–p2
/// at station b), from `za0`/`zb0` up to `za1`/`zb1`.
#[allow(clippy::too_many_arguments)]
fn between(
    p0: Point2,
    p1: Point2,
    p2: Point2,
    p3: Point2,
    za0: f64,
    zb0: f64,
    za1: f64,
    zb1: f64,
) -> Vec<[P3; 3]> {
    let poly = crate::ensure_ccw(vec![p0, p1, p2, p3]);
    // Which of the corners are at station b (p1 and p2).
    let at_b = |q: Point2| q.distance(p1) < 1e-9 || q.distance(p2) < 1e-9;
    crate::solid::draped_prism(&poly, |q| {
        if at_b(q) {
            (zb0, zb1.max(zb0 + 0.01))
        } else {
            (za0, za1.max(za0 + 0.01))
        }
    })
}

/// A street as the ground sees it: its axis and, station by station, its
/// top (the sidewalks'), the bottom of its pavement, its edges and whether
/// it runs on a viaduct there.
struct RoadLine {
    axis: Vec<Point2>,
    /// Distance along the axis of each of its points.
    at: Vec<f64>,
    len: f64,
    samples: Vec<Sample>,
    lo: Point2,
    hi: Point2,
}

#[derive(Debug, Clone, Copy)]
struct Sample {
    top: f64,
    under: f64,
    left: f64,
    right: f64,
    viaduct: bool,
}

/// How the ground is shaped round the streets of a level: lowered under
/// them and meeting their edges in slopes of fill or cut.
pub struct Shaper {
    roads: Vec<RoadLine>,
}

impl Shaper {
    /// The streets of `level` (heights over the model's zero).
    pub fn of(model: &Model, level: archisim_core::LevelId) -> Shaper {
        let roads = model
            .roads_on_level(level)
            .filter(|r| r.axis.len() >= 2)
            .map(|r| {
                let st = stations(model, r);
                let kh = if r.section.single_level {
                    0.0
                } else {
                    r.section.kerb_height
                };
                let fill_of = |x: &Station, g: Option<f64>| g.map_or(0.0, |g| x.top - g);
                let samples = st
                    .iter()
                    .map(|x| Sample {
                        top: x.top,
                        under: x.top - kh - r.section.depth - 0.12,
                        left: x.left.distance(x.right) / 2.0,
                        right: x.left.distance(x.right) / 2.0,
                        viaduct: fill_of(x, x.ground_mid)
                            .max(fill_of(x, x.ground_left).min(fill_of(x, x.ground_right)))
                            > VIADUCT_FROM,
                    })
                    .collect::<Vec<_>>();
                // The real extents to each side (the axis need not be in
                // the middle).
                let samples = samples
                    .into_iter()
                    .zip(&st)
                    .enumerate()
                    .map(|(k, (mut x, _))| {
                        let s = r.length() * k as f64 / (st.len() - 1).max(1) as f64;
                        let (l, rr) = r.extent_at(s);
                        x.left = l;
                        x.right = rr;
                        x
                    })
                    .collect();
                let mut at = vec![0.0];
                for w in r.axis.windows(2) {
                    at.push(at.last().copied().unwrap_or(0.0) + w[0].distance(w[1]));
                }
                let (mut lo, mut hi) = (
                    Point2::new(f64::MAX, f64::MAX),
                    Point2::new(f64::MIN, f64::MIN),
                );
                for p in &r.axis {
                    lo = Point2::new(lo.x.min(p.x), lo.y.min(p.y));
                    hi = Point2::new(hi.x.max(p.x), hi.y.max(p.y));
                }
                let pad = r.section.width() + REACH;
                RoadLine {
                    axis: r.axis.clone(),
                    len: *at.last().unwrap_or(&0.0),
                    at,
                    samples,
                    lo: Point2::new(lo.x - pad, lo.y - pad),
                    hi: Point2::new(hi.x + pad, hi.y + pad),
                }
            })
            .collect();
        Shaper { roads }
    }

    pub fn is_empty(&self) -> bool {
        self.roads.is_empty()
    }

    /// Whether `p` is near a street (where the ground may change).
    pub fn near(&self, p: Point2) -> bool {
        self.roads
            .iter()
            .any(|r| p.x >= r.lo.x && p.x <= r.hi.x && p.y >= r.lo.y && p.y <= r.hi.y)
            && !self.nearby(p).is_empty()
    }

    /// Each street near `p`: its sample at the point of its axis nearest
    /// `p` and the distance of `p` out of its edge (negative: under it).
    /// Past a street's ends the distance is measured from its end, so the
    /// ground rounds them off.
    fn nearby(&self, p: Point2) -> Vec<(Sample, f64)> {
        let mut out = Vec::new();
        for r in &self.roads {
            if p.x < r.lo.x || p.x > r.hi.x || p.y < r.lo.y || p.y > r.hi.y {
                continue;
            }
            let mut best: Option<(Sample, f64)> = None;
            for (i, w) in r.axis.windows(2).enumerate() {
                let ab = w[1] - w[0];
                let l2 = ab.dot(ab);
                if l2 < 1e-12 {
                    continue;
                }
                let t = ((p - w[0]).dot(ab) / l2).clamp(0.0, 1.0);
                let q = w[0] + ab * t;
                let d = q.distance(p);
                let s = r.at[i] + t * l2.sqrt();
                let sample = sample_at(r, s);
                let left_side = ab.x * (p.y - w[0].y) - ab.y * (p.x - w[0].x) > 0.0;
                let half = if left_side { sample.left } else { sample.right };
                let e = d - half;
                if best.is_none_or(|b| e < b.1) {
                    best = Some((sample, e));
                }
            }
            if let Some(b) = best {
                if b.1 <= REACH {
                    out.push(b);
                }
            }
        }
        out
    }

    /// The height of the ground at `p` that was `z`, shaped to the
    /// streets: below the pavement of any street over it (but a viaduct),
    /// and between the slopes of fill and cut of the nearest one.
    pub fn shape(&self, p: Point2, z: f64) -> f64 {
        let near = self.nearby(p);
        if near.is_empty() {
            return z;
        }
        let mut under = false;
        let mut z1 = z;
        for (x, e) in &near {
            if *e <= 0.0 && !x.viaduct {
                z1 = z1.min(x.under);
                under = true;
            }
        }
        if under {
            return z1;
        }
        // The nearest street that the ground meets (not over a viaduct's
        // deck).
        let Some((x, e)) = near
            .iter()
            .filter(|(x, e)| !(x.viaduct && *e <= 0.0))
            .min_by(|a, b| a.1.total_cmp(&b.1))
            .copied()
        else {
            return z;
        };
        let e = e.max(0.0);
        let high = x.top + e / CUT_SLOPE;
        let low = if x.viaduct {
            f64::MIN
        } else {
            x.top - e / FILL_SLOPE
        };
        z.clamp(low, high)
    }
}

fn sample_at(r: &RoadLine, s: f64) -> Sample {
    let n = r.samples.len();
    if n == 1 || r.len <= 0.0 {
        return r.samples[0];
    }
    let f = (s / r.len).clamp(0.0, 1.0) * (n - 1) as f64;
    let i = (f.floor() as usize).min(n - 2);
    let t = f - i as f64;
    let (a, b) = (r.samples[i], r.samples[i + 1]);
    let mix = |u: f64, v: f64| u + (v - u) * t;
    Sample {
        top: mix(a.top, b.top),
        under: mix(a.under, b.under),
        left: mix(a.left, b.left),
        right: mix(a.right, b.right),
        viaduct: a.viaduct || b.viaduct,
    }
}

/// The triangles of the ground made finer near the streets (sides of at
/// most [`FINE_EDGE`]) and shaped to them. An edge is split by itself
/// alone, so two triangles that share it split it alike: no cracks.
pub fn shaped_ground(tris: &[[P3; 3]], shaper: &Shaper) -> Vec<[P3; 3]> {
    if shaper.is_empty() {
        return tris.to_vec();
    }
    let split = |a: P3, b: P3| -> bool {
        let d = ((a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2)).sqrt();
        d > FINE_EDGE && shaper.near(Point2::new((a[0] + b[0]) / 2.0, (a[1] + b[1]) / 2.0))
    };
    let mid = |a: P3, b: P3| {
        [
            (a[0] + b[0]) / 2.0,
            (a[1] + b[1]) / 2.0,
            (a[2] + b[2]) / 2.0,
        ]
    };
    let mut flat = Vec::new();
    let mut stack: Vec<([P3; 3], u32)> = tris.iter().map(|t| (*t, 0)).collect();
    while let Some((t, depth)) = stack.pop() {
        let [a, b, c] = t;
        if depth > 12 {
            flat.push(t);
            continue;
        }
        let (sab, sbc, sca) = (split(a, b), split(b, c), split(c, a));
        let d = depth + 1;
        match (sab, sbc, sca) {
            (false, false, false) => flat.push(t),
            (true, true, true) => {
                let (m, n, o) = (mid(a, b), mid(b, c), mid(c, a));
                stack.extend([
                    ([a, m, o], d),
                    ([m, b, n], d),
                    ([o, n, c], d),
                    ([m, n, o], d),
                ]);
            }
            // One edge: two triangles from its midpoint to the far corner.
            (true, false, false) => {
                let m = mid(a, b);
                stack.extend([([a, m, c], d), ([m, b, c], d)]);
            }
            (false, true, false) => {
                let m = mid(b, c);
                stack.extend([([b, m, a], d), ([m, c, a], d)]);
            }
            (false, false, true) => {
                let m = mid(c, a);
                stack.extend([([c, m, b], d), ([m, a, b], d)]);
            }
            // Two edges: the corner they share cut off, the rest halved.
            (true, true, false) => {
                let (m, n) = (mid(a, b), mid(b, c));
                stack.extend([([m, b, n], d), ([a, m, n], d), ([a, n, c], d)]);
            }
            (false, true, true) => {
                let (m, n) = (mid(b, c), mid(c, a));
                stack.extend([([n, m, c], d), ([b, m, n], d), ([b, n, a], d)]);
            }
            (true, false, true) => {
                let (m, n) = (mid(c, a), mid(a, b));
                stack.extend([([m, a, n], d), ([c, m, n], d), ([c, n, b], d)]);
            }
        }
    }
    let shape = |p: P3| [p[0], p[1], shaper.shape(Point2::new(p[0], p[1]), p[2])];
    flat.into_iter().map(|t| t.map(shape)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use archisim_core::{CutPattern, NewTerrain, RoadSection, RoadStretch, SurveyPoint};

    #[test]
    fn a_street_over_and_under_the_ground() {
        let mut m = Model::new("T");
        let l = m.add_level("Planta baja", 0.0);
        let earth = m.add_material("Tierra", "earth", [120, 100, 80], CutPattern::Solid);
        let conc = m.add_material("Hormigón", "concrete", [180, 180, 175], CutPattern::Solid);
        // A valley 12 m deep in the middle, between two hills.
        let sp = |x: f64, y: f64, z: f64| SurveyPoint {
            at: Point2::new(x, y),
            z,
        };
        let mut pts = Vec::new();
        for x in [-10.0, 40.0, 60.0, 80.0, 130.0] {
            let z = if x == 60.0 {
                -12.0
            } else if x == 40.0 || x == 80.0 {
                -6.0
            } else {
                3.0
            };
            pts.push(sp(x, -40.0, z));
            pts.push(sp(x, 40.0, z));
        }
        m.add_terrain(NewTerrain {
            level: l,
            points: pts,
            material: earth,
            depth: 1.0,
            contour: 1.0,
            lines: Vec::new(),
        })
        .unwrap();
        let id = m
            .add_road_stretch(RoadStretch {
                level: l,
                points: vec![Point2::new(0.0, 0.0), Point2::new(120.0, 0.0)],
                continues: None,
                section: RoadSection::default(),
                top_offset: 0.0,
                pavement: conc,
                kerb_material: conc,
                sidewalk_material: conc,
                // Nearly level: over the valley, in the hills.
                follow_ground: Some(0.02),
            })
            .unwrap();
        let s = road_ground_solids(&m, id);
        let by = |mat: MaterialId| -> usize {
            s.iter()
                .filter(|x| x.material == mat)
                .map(|x| x.triangles.len())
                .sum()
        };
        // Concrete: deck, piers and parapets.
        assert!(by(conc) > 0, "no concrete");
        // Piers reach down near the valley's floor.
        let low = s
            .iter()
            .flat_map(|x| x.triangles.iter().flatten())
            .map(|p| p[2])
            .fold(f64::MAX, f64::min);
        assert!(low < -8.0, "{low}");
        // The ground is shaped to it: under it, below its pavement; at
        // its edge, at its top; far away, as it was.
        let shaper = Shaper::of(&m, l);
        let r = m.road(id).unwrap();
        let top = r.top_at(20.0);
        assert!(shaper.shape(Point2::new(20.0, 0.0), 50.0) < top);
        let at_edge = shaper.shape(Point2::new(20.0, r.extent_at(20.0).0 + 0.01), 50.0);
        assert!(
            (at_edge - top - r.section.kerb_height).abs() < 0.2,
            "{at_edge} {top}"
        );
        assert_eq!(shaper.shape(Point2::new(20.0, 39.0), 3.0), 3.0);
        // The terrain's own triangles are made fine round it.
        let t = m.terrains().next().unwrap();
        let p3 = |i: usize| [t.points[i].at.x, t.points[i].at.y, t.points[i].z];
        let tris: Vec<[P3; 3]> = t
            .triangles()
            .iter()
            .map(|[a, b, c]| [p3(*a), p3(*b), p3(*c)])
            .collect();
        let fine = shaped_ground(&tris, &shaper);
        assert!(
            fine.len() > tris.len() * 20,
            "{} {}",
            fine.len(),
            tris.len()
        );
        let _ = earth;
    }
}
