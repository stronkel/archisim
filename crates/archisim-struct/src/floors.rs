//! One-way floors of joists and blocks (forjados unidireccionales).
//!
//! The floor is cut into bands one metre wide along its joists. Each band
//! is a continuous beam over the walls and beams it crosses, with
//! cantilevers where it runs past the last one, loaded with what lies on
//! its metre (the floor, the use, the snow and the walls standing on it).
//! The variable loads go span by span (alternancia de sobrecargas), so a
//! span's sagging and a support's hogging are the worst the patterns
//! give. The end supports take a hogging of a quarter of their span's
//! sagging at least (partial fixity of a floor built into a wall), as is
//! usual for these floors.
//!
//! Per rib (a joist and its share of topping):
//! - precast joists: the ultimate moment the joist must have, for the
//!   manufacturer's sheet (its strength is theirs, not ours);
//! - ribs cast in place: the bottom bars, the topping in compression;
//! - the top bars over each support, the rib's bottom in compression; if
//!   the rib is short, a solid strip (macizado) by the support;
//! - shear at d from the support against the rib without stirrups
//!   (Código Estructural, art. 44), and the solid strip it asks for;
//! - deflection: elastic with the rib's inertia (0,8 for precast, 0,5
//!   cast in place for cracking), the long-term one three times the
//!   quasi-permanent, against L/300; the active one (after the
//!   partitions: all but the floor's own weight at first) against L/400
//!   (DB-SE 4.3.3.1).

use archisim_core::{
    bar_area, Counted, FloorKind, FloorSystem, Joist, Model, Point2, SlabId, Vec2,
};

use crate::cases::{combinations, Case, Kind};
use crate::concrete::Rc;
use crate::geo;
use crate::gravity::{self, PlateSource, Takedown};
use crate::lightframe::{Check, Status, Subject};
use crate::text::dec;

/// Width of a band, m.
pub const BAND: f64 = 1.0;
/// Most bands per floor: wider ones past this.
const MAX_BANDS: usize = 60;
/// Longest piece of a band's beam, m.
const PIECE: f64 = 0.2;

/// The direction of a slab's joists (`Model::joist_direction`).
pub fn joist_direction(model: &Model, slab: SlabId) -> Option<Vec2> {
    model.joist_direction(slab)
}

/// The one-way system of a slab, if it has one.
pub fn one_way(model: &Model, slab: SlabId) -> Option<FloorSystem> {
    let s = model.slab(slab)?;
    model
        .floor_system(s.slab_type)
        .filter(|f| f.kind == FloorKind::OneWay)
        .copied()
}

/// A continuous beam on points: `x` of its nodes (rising), EI (kN·m²),
/// which nodes are held. Solved by stiffness, two freedoms a node.
struct BandBeam {
    x: Vec<f64>,
    ei: f64,
    held: Vec<bool>,
}

/// What a loading gives at each node: moment (sagging +), shear on the
/// left of the node and on its right, deflection (down +).
#[derive(Clone)]
struct Response {
    m: Vec<f64>,
    v: Vec<[f64; 2]>,
    w: Vec<f64>,
}

impl BandBeam {
    /// `q[e]`: uniform load on piece e (kN/m, down +); `p[n]`: load at
    /// node n (kN, down +).
    fn solve(&self, q: &[f64], p: &[f64]) -> Response {
        let n = self.x.len();
        let dof = 2 * n;
        let mut k = vec![vec![0.0f64; dof]; dof];
        let mut f = vec![0.0f64; dof];
        for e in 0..n - 1 {
            let l = self.x[e + 1] - self.x[e];
            let c = self.ei / l.powi(3);
            let ke = [
                [12.0, 6.0 * l, -12.0, 6.0 * l],
                [6.0 * l, 4.0 * l * l, -6.0 * l, 2.0 * l * l],
                [-12.0, -6.0 * l, 12.0, -6.0 * l],
                [6.0 * l, 2.0 * l * l, -6.0 * l, 4.0 * l * l],
            ];
            let idx = [2 * e, 2 * e + 1, 2 * e + 2, 2 * e + 3];
            for a in 0..4 {
                for b in 0..4 {
                    k[idx[a]][idx[b]] += c * ke[a][b];
                }
            }
            let w = q[e];
            let fe = [
                w * l / 2.0,
                w * l * l / 12.0,
                w * l / 2.0,
                -w * l * l / 12.0,
            ];
            for a in 0..4 {
                f[idx[a]] += fe[a];
            }
        }
        for (i, pi) in p.iter().enumerate() {
            f[2 * i] += pi;
        }
        // Held nodes: w = 0.
        for (i, h) in self.held.iter().enumerate() {
            if *h {
                let r = 2 * i;
                for c in 0..dof {
                    k[r][c] = 0.0;
                    k[c][r] = 0.0;
                }
                k[r][r] = 1.0;
                f[r] = 0.0;
            }
        }
        let u = solve_dense(k, f);
        // Element ends.
        let mut m = vec![0.0; n];
        let mut v = vec![[0.0f64; 2]; n];
        for e in 0..n - 1 {
            let l = self.x[e + 1] - self.x[e];
            let c = self.ei / l.powi(3);
            let (w1, t1, w2, t2) = (u[2 * e], u[2 * e + 1], u[2 * e + 2], u[2 * e + 3]);
            let qe = q[e];
            // End forces on the element (down + for shear, clockwise moment
            // at the ends as in the stiffness matrix), minus the fixed-end
            // forces of the load.
            let v1 = c * (12.0 * w1 + 6.0 * l * t1 - 12.0 * w2 + 6.0 * l * t2) - qe * l / 2.0;
            let m1 = c * (6.0 * l * w1 + 4.0 * l * l * t1 - 6.0 * l * w2 + 2.0 * l * l * t2)
                - qe * l * l / 12.0;
            let v2 = c * (-12.0 * w1 - 6.0 * l * t1 + 12.0 * w2 - 6.0 * l * t2) - qe * l / 2.0;
            let m2 = c * (6.0 * l * w1 + 2.0 * l * l * t1 - 6.0 * l * w2 + 4.0 * l * l * t2)
                + qe * l * l / 12.0;
            // With w down and these turns, sagging is m1 at the first end
            // and -m2 at the second.
            m[e] = m1;
            m[e + 1] = -m2;
            v[e][1] = -v1;
            v[e + 1][0] = v2;
        }
        Response {
            m,
            v,
            w: (0..n).map(|i| u[2 * i]).collect(),
        }
    }
}

fn solve_dense(mut a: Vec<Vec<f64>>, mut b: Vec<f64>) -> Vec<f64> {
    let n = b.len();
    for c in 0..n {
        let p = (c..n)
            .max_by(|x, y| a[*x][c].abs().total_cmp(&a[*y][c].abs()))
            .unwrap_or(c);
        a.swap(c, p);
        b.swap(c, p);
        let d = a[c][c];
        if d.abs() < 1e-14 {
            continue;
        }
        for r in c + 1..n {
            let f = a[r][c] / d;
            if f != 0.0 {
                for k in c..n {
                    a[r][k] -= f * a[c][k];
                }
                b[r] -= f * b[c];
            }
        }
    }
    let mut x = vec![0.0; n];
    for r in (0..n).rev() {
        let s: f64 = (r + 1..n).map(|k| a[r][k] * x[k]).sum();
        x[r] = if a[r][r].abs() < 1e-14 {
            0.0
        } else {
            (b[r] - s) / a[r][r]
        };
    }
    x
}

/// A span of a band: from support to support, or a cantilever.
#[derive(Debug, Clone, PartialEq)]
pub struct Span {
    pub length: f64,
    pub cantilever: bool,
    /// Sagging and hogging design moments per metre, kN·m/m.
    pub sagging: f64,
    /// Shear at d from its supports, per metre, kN/m.
    pub shear: f64,
    /// Long-term and active deflection, m.
    pub total: f64,
    pub active: f64,
    /// Where it lies: the middle of the span, for the plan.
    pub middle: Point2,
    /// How long the solid strip by each end must be for the shear, m.
    pub solid: f64,
}

/// A support of a band.
#[derive(Debug, Clone, PartialEq)]
pub struct Bearing {
    pub at: Point2,
    /// Hogging per metre, kN·m/m (positive).
    pub hogging: f64,
    /// Whether it is an end support (only the minimum).
    pub end: bool,
    /// The longer span by it, for the length of the top bars, m.
    pub span: f64,
}

/// What a one-way floor gives.
#[derive(Debug, Clone, PartialEq)]
pub struct OneWayResult {
    pub slab: SlabId,
    pub system: FloorSystem,
    pub direction: Vec2,
    pub spans: Vec<Span>,
    pub bearings: Vec<Bearing>,
    /// Parts of bands that nothing holds up, m of floor.
    pub loose: f64,
}

/// Every one-way floor.
pub fn analyse(model: &Model, td: &Takedown) -> Vec<OneWayResult> {
    let plates = gravity::plates(model);
    let rc = Rc::of(model);
    let altitude = model.site_altitude();
    let mut out = Vec::new();
    for (i, pl) in plates.iter().enumerate() {
        let PlateSource::Slab(id) = pl.source else {
            continue;
        };
        let Some(sys) = one_way(model, id) else {
            continue;
        };
        if pl.base || pl.boxed || pl.pitch > 0.05 || pl.outline.len() < 3 {
            continue;
        }
        let Some(dir) = joist_direction(model, id) else {
            continue;
        };
        out.push(band_analysis(
            model, td, &plates, i, id, sys, dir, &rc, altitude,
        ));
    }
    out
}

#[allow(clippy::too_many_arguments)]
fn band_analysis(
    model: &Model,
    td: &Takedown,
    plates: &[gravity::Plate],
    i: usize,
    id: SlabId,
    sys: FloorSystem,
    d: Vec2,
    rc: &Rc,
    altitude: f64,
) -> OneWayResult {
    let pl = &plates[i];
    let n = Vec2::new(-d.y, d.x);
    let o = pl.outline[0];
    let along = |p: Point2| (p - o).x * d.x + (p - o).y * d.y;
    let across = |p: Point2| (p - o).x * n.x + (p - o).y * n.y;
    let (mut t0, mut t1) = (f64::MAX, f64::MIN);
    for p in &pl.outline {
        t0 = t0.min(across(*p));
        t1 = t1.max(across(*p));
    }
    let count = (((t1 - t0) / BAND).ceil() as usize).clamp(1, MAX_BANDS);
    let width = (t1 - t0) / count as f64;
    let supports = gravity::plate_supports(model, td, pl);
    let rooms: Vec<_> = model.rooms_on_level(pl.level).collect();
    let (ig, _) = sys.rib_inertia();
    let crack = if sys.joist == Joist::Precast {
        0.8
    } else {
        0.5
    };
    let ei = rc.ecm() * ig * crack / sys.spacing;
    let mut spans = Vec::new();
    let mut bearings = Vec::new();
    let mut loose = 0.0;
    let dd = sys.depth - 0.03;
    for b in 0..count {
        let t = t0 + (b as f64 + 0.5) * width;
        let base = o + n * t;
        for (sa, sb) in line_inside(base, d, &pl.outline, &pl.holes) {
            if sb - sa < 0.5 {
                continue;
            }
            // The supports it crosses.
            let mut st: Vec<f64> = Vec::new();
            for sp in &supports {
                if sp.path.len() == 1 {
                    let c = sp.path[0];
                    if (across(c) - t).abs() < width / 2.0 {
                        let s = along(c) - along(base);
                        if s > sa - 0.2 && s < sb + 0.2 {
                            st.push(s.clamp(sa, sb));
                        }
                    }
                    continue;
                }
                for w in sp.path.windows(2) {
                    if let Some(s) = cross(base, d, w[0], w[1]) {
                        if s > sa - 0.2 && s < sb + 0.2 {
                            st.push(s.clamp(sa, sb));
                        }
                    }
                }
            }
            st.sort_by(f64::total_cmp);
            st.dedup_by(|a, b| (*a - *b).abs() < 0.15);
            if st.is_empty() {
                loose += sb - sa;
                continue;
            }
            // Nodes: the ends, the supports and pieces between.
            let mut keys = vec![sa];
            keys.extend(st.iter().copied());
            keys.push(sb);
            keys.dedup_by(|a, b| (*a - *b).abs() < 0.05);
            let mut x = Vec::new();
            for w in keys.windows(2) {
                let pieces = ((w[1] - w[0]) / PIECE).ceil().max(1.0) as usize;
                for k in 0..pieces {
                    x.push(w[0] + (w[1] - w[0]) * k as f64 / pieces as f64);
                }
            }
            x.push(*keys.last().unwrap());
            let held: Vec<bool> = x
                .iter()
                .map(|xi| st.iter().any(|s| (s - xi).abs() < 1e-6))
                .collect();
            if held.iter().filter(|h| **h).count() == 1 && x.len() > 2 {
                // One support only: a cantilever both ways is not a floor.
                loose += sb - sa;
                continue;
            }
            let beam = BandBeam {
                x: x.clone(),
                ei,
                held: held.clone(),
            };
            let ne = x.len() - 1;
            let at = |s: f64| base + d * s;
            // Loads per case: per piece (kN/m on the band's metre) and
            // per node.
            let mut q: std::collections::BTreeMap<Case, Vec<f64>> = Default::default();
            for e in 0..ne {
                let mid = at((x[e] + x[e + 1]) / 2.0);
                for (case, v) in gravity::area_loads(model, plates, i, &rooms, mid) {
                    q.entry(case).or_insert_with(|| vec![0.0; ne])[e] += v;
                }
            }
            let mut p: std::collections::BTreeMap<Case, Vec<f64>> = Default::default();
            if let Some(inc) = td.plate_incoming.get(i) {
                for (pt, case, kn) in inc {
                    if (across(*pt) - t).abs() > width / 2.0 {
                        continue;
                    }
                    let s = along(*pt) - along(base);
                    if s < sa - 0.05 || s > sb + 0.05 {
                        continue;
                    }
                    let k = x
                        .iter()
                        .enumerate()
                        .min_by(|a, b| (a.1 - s).abs().total_cmp(&(b.1 - s).abs()))
                        .map_or(0, |(k, _)| k);
                    p.entry(*case).or_insert_with(|| vec![0.0; x.len()])[k] += kn / width;
                }
            }
            let cases: Vec<Case> = q
                .keys()
                .chain(p.keys())
                .copied()
                .collect::<std::collections::BTreeSet<_>>()
                .into_iter()
                .collect();
            // Spans between held nodes (and cantilevers at the ends).
            let held_idx: Vec<usize> = (0..x.len()).filter(|k| held[*k]).collect();
            let mut ranges: Vec<(usize, usize, bool)> = Vec::new();
            if held_idx[0] > 0 {
                ranges.push((0, held_idx[0], true));
            }
            for w in held_idx.windows(2) {
                ranges.push((w[0], w[1], false));
            }
            if *held_idx.last().unwrap() < x.len() - 1 {
                ranges.push((*held_idx.last().unwrap(), x.len() - 1, true));
            }
            let zero_q = vec![0.0; ne];
            let zero_p = vec![0.0; x.len()];
            // Per case: the whole case, and for the variable ones each
            // range alone.
            let mut whole: std::collections::BTreeMap<Case, Response> = Default::default();
            let mut parts: std::collections::BTreeMap<Case, Vec<Response>> = Default::default();
            for c in &cases {
                let qc = q.get(c).unwrap_or(&zero_q);
                let pc = p.get(c).unwrap_or(&zero_p);
                whole.insert(*c, beam.solve(qc, pc));
                if *c != Case::Dead {
                    let mut list = Vec::new();
                    for (a, z, _) in &ranges {
                        let qk: Vec<f64> = (0..ne)
                            .map(|e| if e >= *a && e < *z { qc[e] } else { 0.0 })
                            .collect();
                        let pk: Vec<f64> = (0..x.len())
                            .map(|k| if k >= *a && k <= *z { pc[k] } else { 0.0 })
                            .collect();
                        list.push(beam.solve(&qk, &pk));
                    }
                    parts.insert(*c, list);
                }
            }
            let combos = combinations(&cases, altitude);
            let nn = x.len();
            let mut m_max = vec![0.0f64; nn];
            let mut m_min = vec![0.0f64; nn];
            let mut v_max = vec![0.0f64; nn];
            let mut w_qp = vec![0.0f64; nn];
            for combo in &combos {
                match combo.kind {
                    Kind::Ultimate => {
                        for k in 0..nn {
                            let mut hi = 0.0;
                            let mut lo = 0.0;
                            let mut vl = 0.0;
                            let mut vr = 0.0;
                            for c in &cases {
                                let f = combo.factor(*c);
                                if f == 0.0 {
                                    continue;
                                }
                                if *c == Case::Dead {
                                    let r = &whole[c];
                                    hi += f * r.m[k];
                                    lo += f * r.m[k];
                                    vl += f * r.v[k][0];
                                    vr += f * r.v[k][1];
                                } else {
                                    for r in &parts[c] {
                                        hi += f * r.m[k].max(0.0);
                                        lo += f * r.m[k].min(0.0);
                                        vl += f * r.v[k][0].abs();
                                        vr += f * r.v[k][1].abs();
                                    }
                                }
                            }
                            m_max[k] = m_max[k].max(hi);
                            m_min[k] = m_min[k].min(lo);
                            v_max[k] = v_max[k].max(vl.abs()).max(vr.abs());
                        }
                    }
                    Kind::QuasiPermanent => {
                        for k in 0..nn {
                            let w: f64 =
                                cases.iter().map(|c| combo.factor(*c) * whole[c].w[k]).sum();
                            w_qp[k] = w_qp[k].max(w);
                        }
                    }
                    _ => {}
                }
            }
            // The floor's own weight alone, before the partitions.
            let own = vec![sys.weight(); ne];
            let w_g0 = beam.solve(&own, &zero_p).w;
            // Spans.
            let mut sag_of = Vec::new();
            for (a, z, cant) in &ranges {
                let len = x[*z] - x[*a];
                let sag = if *cant {
                    0.0
                } else {
                    (*a..=*z).map(|k| m_max[k]).fold(0.0, f64::max)
                };
                sag_of.push(sag);
                // Shear at d (and a little for the support) from each held end.
                let mut shear = 0.0f64;
                let mut ends = Vec::new();
                if held[*a] {
                    ends.push((*a, 1.0));
                }
                if held[*z] {
                    ends.push((*z, -1.0));
                }
                let mut solid = 0.0f64;
                let vrd_rib = rc.vrd_c(sys.rib, dd, 0.01) / sys.spacing;
                for (h, sign) in &ends {
                    let s0 = x[*h] + sign * (dd + 0.1);
                    let k = (*a..=*z)
                        .min_by(|p, q| (x[*p] - s0).abs().total_cmp(&(x[*q] - s0).abs()))
                        .unwrap_or(*h);
                    shear = shear.max(v_max[k]);
                    // How far from the support the shear stays over the rib's.
                    let far = (*a..=*z)
                        .filter(|kk| v_max[*kk] > vrd_rib)
                        .map(|kk| (x[kk] - x[*h]).abs())
                        .fold(0.0, f64::max);
                    if far > 0.0 {
                        solid = solid.max(((far + 0.1) * 20.0).ceil() / 20.0);
                    }
                }
                if *cant {
                    shear = shear.max(v_max[if held[*a] { *a } else { *z }]);
                }
                let wmax = (*a..=*z)
                    .map(|k| {
                        let tot = 3.0 * w_qp[k];
                        (tot, tot - w_g0[k])
                    })
                    .fold((0.0f64, 0.0f64), |acc, v| (acc.0.max(v.0), acc.1.max(v.1)));
                // Relative to the supports: a cantilever's tip moves with
                // its support's turn, which the beam already gives.
                spans.push(Span {
                    length: len,
                    cantilever: *cant,
                    sagging: sag,
                    shear,
                    total: wmax.0,
                    active: wmax.1.max(0.0),
                    middle: at((x[*a] + x[*z]) / 2.0),
                    solid,
                });
            }
            // Bearings.
            for (r, h) in held_idx.iter().enumerate() {
                let left = ranges.iter().position(|(a, z, _)| z == h && a < z);
                let right = ranges.iter().position(|(a, z, _)| a == h && a < z);
                let interior =
                    left.is_some_and(|l| !ranges[l].2) && right.is_some_and(|q| !ranges[q].2);
                let lens: Vec<f64> = [left, right]
                    .iter()
                    .flatten()
                    .map(|k| x[ranges[*k].1] - x[ranges[*k].0])
                    .collect();
                let span_len = lens.iter().copied().fold(0.0, f64::max);
                let mut hog = (-m_min[*h]).max(0.0);
                let next_sag = [left, right]
                    .iter()
                    .flatten()
                    .map(|k| sag_of[*k])
                    .fold(0.0, f64::max);
                let end = !interior && ranges.iter().all(|(a, z, c)| !(*c && (a == h || z == h)));
                if end {
                    hog = hog.max(0.25 * next_sag);
                }
                let _ = r;
                bearings.push(Bearing {
                    at: at(x[*h]),
                    hogging: hog,
                    end,
                    span: span_len,
                });
            }
        }
    }
    OneWayResult {
        slab: id,
        system: sys,
        direction: d,
        spans,
        bearings,
        loose,
    }
}

/// Where the line `base + d·s` crosses the segment `a`–`b`: its `s`.
fn cross(base: Point2, d: Vec2, a: Point2, b: Point2) -> Option<f64> {
    let e = b - a;
    let den = d.x * e.y - d.y * e.x;
    if den.abs() < 1e-12 {
        return None;
    }
    let w = a - base;
    let s = (w.x * e.y - w.y * e.x) / den;
    let u = (w.x * d.y - w.y * d.x) / den;
    (-1e-9..=1.0 + 1e-9).contains(&u).then_some(s)
}

/// The stretches of the line `base + d·s` inside the outline and out of
/// its holes.
fn line_inside(
    base: Point2,
    d: Vec2,
    outline: &[Point2],
    holes: &[Vec<Point2>],
) -> Vec<(f64, f64)> {
    let mut cuts = Vec::new();
    for ring in std::iter::once(outline).chain(holes.iter().map(|h| h.as_slice())) {
        let n = ring.len();
        for k in 0..n {
            if let Some(s) = cross(base, d, ring[k], ring[(k + 1) % n]) {
                cuts.push(s);
            }
        }
    }
    cuts.sort_by(f64::total_cmp);
    cuts.dedup_by(|a, b| (*a - *b).abs() < 1e-7);
    let mut out = Vec::new();
    for w in cuts.windows(2) {
        let mid = base + d * ((w[0] + w[1]) / 2.0);
        if geo::inside(mid, outline) && !holes.iter().any(|h| geo::inside(mid, h)) {
            out.push((w[0], w[1]));
        }
    }
    out
}

/// The fewest bars per rib for `area` (m²): one or two of 8 to 20, else
/// three.
pub fn rib_bars(area: f64) -> Counted {
    for n in [1u32, 2] {
        for dm in [0.008, 0.010, 0.012, 0.016, 0.020] {
            if f64::from(n) * bar_area(dm) >= area {
                return Counted::new(n, dm);
            }
        }
    }
    for dm in [0.016, 0.020, 0.025] {
        if 3.0 * bar_area(dm) >= area {
            return Counted::new(3, dm);
        }
    }
    Counted::new(3, 0.025)
}

pub fn check(model: &Model, td: &Takedown) -> Vec<Check> {
    let rc = Rc::of(model);
    let mut out = Vec::new();
    for r in analyse(model, td) {
        let subject = Subject::Floor(r.slab);
        let sys = r.system;
        let name = model
            .slab(r.slab)
            .and_then(|s| model.slab_type(s.slab_type))
            .map_or(String::new(), |t| t.name.clone());
        let dd = sys.depth - 0.03;
        let head = format!(
            "{name} ({} {}, {}, viguetas a {}°)",
            match sys.joist {
                Joist::Precast => "viguetas pretensadas",
                Joist::InSitu => "nervios in situ",
            },
            sys.label(),
            sys.infill.label().to_lowercase(),
            dec(
                r.direction
                    .y
                    .atan2(r.direction.x)
                    .to_degrees()
                    .rem_euclid(180.0),
                0
            )
        );
        if r.spans.is_empty() {
            out.push(Check {
                subject,
                what: format!("{head}: las viguetas no encuentran apoyos"),
                rule: "Apoyos del forjado unidireccional",
                ratio: 2.0,
                status: Status::Fail,
                advice: Some("muros o vigas que crucen las viguetas, o girar su dirección".into()),
            });
            continue;
        }
        if r.loose > 0.5 {
            out.push(Check {
                subject,
                what: format!("{head}: {} m de banda sin apoyo", dec(r.loose, 1)),
                rule: "Apoyos del forjado unidireccional",
                ratio: 1.5,
                status: Status::Warn,
                advice: Some("una viga o un zuncho donde las viguetas se quedan sin apoyo".into()),
            });
        }
        // Sagging: the worst span.
        let worst = r
            .spans
            .iter()
            .filter(|s| !s.cantilever)
            .max_by(|a, b| a.sagging.total_cmp(&b.sagging));
        if let Some(s) = worst {
            let m_rib = s.sagging * sys.spacing;
            match sys.joist {
                Joist::Precast => out.push(Check {
                    subject,
                    what: format!(
                        "{head}: vano de {} m, M+ {} kN·m/m: cada vigueta debe resistir Mu ≥ {} kN·m",
                        dec(s.length, 2),
                        dec(s.sagging, 1),
                        dec(m_rib, 1)
                    ),
                    rule: "Momento positivo de la vigueta (Código Estructural, art. 42; ficha del fabricante)",
                    ratio: 0.0,
                    status: Status::Info,
                    advice: Some(format!(
                        "elige en la ficha de características técnicas del fabricante una vigueta con momento último de {} kN·m o más para este canto",
                        dec(m_rib, 1)
                    )),
                }),
                Joist::InSitu => {
                    let need = rc.bending_steel(m_rib, sys.spacing, dd);
                    // The compressed block must stay in the topping.
                    let x = need.map(|a| a * rc.fyd() / (0.8 * sys.spacing * rc.fcd()));
                    let (ratio, status, what) = match (need, x) {
                        (Some(a), Some(x)) if x <= sys.topping => {
                            let a = a.max(rc.min_beam_steel(sys.rib, sys.depth));
                            (
                                0.0,
                                Status::Info,
                                format!("{} por nervio ({} cm²)", rib_bars(a).label(), dec(a * 1e4, 2)),
                            )
                        }
                        _ => (1.5, Status::Fail, "la capa de compresión no basta".into()),
                    };
                    out.push(Check {
                        subject,
                        what: format!(
                            "{head}: vano de {} m, M+ {} kN·m/m ({} kN·m por nervio): {what}",
                            dec(s.length, 2),
                            dec(s.sagging, 1),
                            dec(m_rib, 1)
                        ),
                        rule: "Momento positivo del nervio (Código Estructural, art. 42)",
                        ratio,
                        status,
                        advice: (status == Status::Fail).then(|| "más canto o más capa de compresión".into()),
                    });
                }
            }
        }
        // Hogging: the worst support.
        if let Some(b) = r
            .bearings
            .iter()
            .max_by(|a, b| a.hogging.total_cmp(&b.hogging))
        {
            let m_rib = b.hogging * sys.spacing;
            let rib = rc.bending_steel(m_rib, sys.rib, dd);
            let solid = rc.bending_steel(m_rib, sys.spacing, dd);
            let length = (0.25 * b.span + 0.1).max(0.6);
            let (ratio, status, what, advice) = match (rib, solid) {
                (Some(a), _) => (
                    0.0,
                    Status::Info,
                    format!(
                        "{} por nervio arriba ({} cm²), {} m a cada lado del apoyo",
                        rib_bars(a).label(),
                        dec(a * 1e4, 2),
                        dec(length, 2)
                    ),
                    None,
                ),
                (None, Some(a)) => (
                    1.0,
                    Status::Warn,
                    format!(
                        "el nervio no basta a compresión: macizado junto al apoyo y {} por nervio arriba",
                        rib_bars(a).label()
                    ),
                    Some(format!("macizar {} m junto al apoyo (como el zuncho)", dec((0.15 * b.span).max(0.25), 2))),
                ),
                _ => (1.5, Status::Fail, "ni macizado basta".into(), Some("más canto".into())),
            };
            out.push(Check {
                subject,
                what: format!(
                    "{head}: apoyo {} con M− {} kN·m/m ({} kN·m por nervio){}: {what}",
                    if b.end { "extremo" } else { "intermedio" },
                    dec(b.hogging, 1),
                    dec(m_rib, 1),
                    if b.end {
                        ", al menos un cuarto del positivo"
                    } else {
                        ""
                    }
                ),
                rule: "Momento negativo del forjado (Código Estructural, art. 42)",
                ratio,
                status,
                advice,
            });
        }
        // Shear.
        if let Some(s) = r.spans.iter().max_by(|a, b| a.shear.total_cmp(&b.shear)) {
            let v_rib = s.shear * sys.spacing;
            let vrd = rc.vrd_c(sys.rib, dd, 0.01);
            let vrd_solid = rc.vrd_c(sys.spacing, dd, 0.005);
            let u = v_rib / vrd.max(1e-9);
            let (ratio, status, advice) = if u <= 1.0 {
                (u, Status::Pass, None)
            } else if v_rib <= vrd_solid {
                (
                    v_rib / vrd_solid,
                    Status::Warn,
                    Some(format!(
                        "macizado de {} m junto a los apoyos",
                        dec(s.solid.max(0.2), 2)
                    )),
                )
            } else {
                (
                    v_rib / vrd_solid,
                    Status::Fail,
                    Some("más canto o un apoyo intermedio".into()),
                )
            };
            out.push(Check {
                subject,
                what: format!(
                    "{head}: cortante a un canto del apoyo {} kN/m ({} kN por nervio) sobre Vrd,c {} kN del nervio",
                    dec(s.shear, 1),
                    dec(v_rib, 1),
                    dec(vrd, 1)
                ),
                rule: "Cortante del nervio (Código Estructural, art. 44)",
                ratio,
                status,
                advice,
            });
        }
        // Deflection: the worst against its limits.
        let mut worst: Option<(f64, &Span, f64, f64)> = None;
        for s in &r.spans {
            let l = if s.cantilever {
                2.0 * s.length
            } else {
                s.length
            };
            let u = (s.total / (l / 300.0)).max(s.active / (l / 400.0));
            if worst.is_none_or(|w| u > w.0) {
                worst = Some((u, s, l / 300.0, l / 400.0));
            }
        }
        if let Some((u, s, lt, la)) = worst {
            out.push(Check {
                subject,
                what: format!(
                    "{head}: {} de {} m, flecha total {} mm (límite {} mm) y activa {} mm (límite {} mm)",
                    if s.cantilever { "voladizo" } else { "vano" },
                    dec(s.length, 2),
                    dec(s.total * 1000.0, 1),
                    dec(lt * 1000.0, 1),
                    dec(s.active * 1000.0, 1),
                    dec(la * 1000.0, 1)
                ),
                rule: "Flecha del forjado (DB-SE 4.3.3; L/300 total, L/400 activa)",
                ratio: u,
                status: if u <= 1.0 { Status::Pass } else { Status::Fail },
                advice: (u > 1.0).then(|| format!("más canto (h {} m) o menos luz", dec(sys.depth + 0.05, 2))),
            });
        }
        out.push(Check {
            subject,
            what: format!(
                "{head}: mallazo de reparto en la capa de compresión de {} cm",
                dec(sys.topping * 100.0, 0)
            ),
            rule: "Armadura de reparto (Código Estructural)",
            ratio: 0.0,
            status: Status::Info,
            advice: Some(
                "malla electrosoldada en la capa de compresión, p. ej. ME 20×30 Ø5–5".into(),
            ),
        });
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use archisim_core::*;

    #[test]
    fn a_continuous_beam() {
        // Two equal spans of 4 m with 10 kN/m: hogging 20 kN·m over the
        // middle, sagging 0,0703·q·L² = 11,25, reactions 15 · 50 · 15.
        let x: Vec<f64> = (0..=40).map(|k| k as f64 * 0.2).collect();
        let held: Vec<bool> = x
            .iter()
            .map(|v| [0.0, 4.0, 8.0].iter().any(|s| (s - v).abs() < 1e-9))
            .collect();
        let beam = BandBeam {
            x: x.clone(),
            ei: 1e4,
            held,
        };
        let r = beam.solve(&[10.0; 40], &[0.0; 41]);
        assert!((r.m[20] + 20.0).abs() < 0.05, "{}", r.m[20]);
        let sag = r.m.iter().copied().fold(0.0, f64::max);
        assert!((sag - 11.25).abs() < 0.15, "{sag}");
        assert!((r.v[0][1] - 15.0).abs() < 0.1, "{:?}", r.v[0]);
        assert!(r.w[7] > 0.0);
        // A simple span: 5·q·L⁴/(384·EI).
        let x: Vec<f64> = (0..=20).map(|k| k as f64 * 0.2).collect();
        let held: Vec<bool> = (0..=20).map(|k| k == 0 || k == 20).collect();
        let r = BandBeam { x, ei: 1e4, held }.solve(&[10.0; 20], &[0.0; 21]);
        let w = 5.0 * 10.0 * 4f64.powi(4) / (384.0 * 1e4);
        assert!((r.w[10] - w).abs() < 1e-4, "{} {w}", r.w[10]);
        assert!((r.m[10] - 20.0).abs() < 0.05);
    }

    /// Two bays of 4,5 m between three walls, one-way floor across.
    fn house(m: &mut Model, sys: FloorSystem) -> SlabId {
        let l0 = m.add_level("Planta baja", 0.0);
        let l1 = m.add_level("Planta 1", 3.0);
        let lib = archisim_library::add_library(m).unwrap();
        for x in [0.0, 4.5, 9.0] {
            m.add_wall(NewWall {
                wall_type: lib.facade,
                level: l0,
                start: Point2::new(x, 0.0),
                end: Point2::new(x, 6.0),
                bulge: 0.0,
                reference: ReferenceLine::Center,
                height: 3.0,
                base_offset: 0.0,
                top: WallTop::Unconnected,
            })
            .unwrap();
        }
        let concrete = m
            .materials()
            .find(|x| x.name.to_lowercase().contains("hormigón"))
            .unwrap()
            .id;
        let t = m
            .add_slab_type(
                "Forjado unidireccional 25+5",
                SlabFunction::Floor,
                vec![Layer {
                    material: concrete,
                    thickness: sys.depth,
                    function: LayerFunction::Structure,
                }],
            )
            .unwrap();
        m.set_floor_system(t, Some(sys)).unwrap();
        m.add_slab(NewSlab {
            slab_type: t,
            level: l1,
            outline: vec![
                Point2::new(0.0, 0.0),
                Point2::new(9.0, 0.0),
                Point2::new(9.0, 6.0),
                Point2::new(0.0, 6.0),
            ],
            top_offset: 0.0,
        })
        .unwrap()
    }

    #[test]
    fn a_one_way_floor_over_three_walls() {
        let mut m = Model::new("Forjado");
        let slab = house(&mut m, FloorSystem::one_way());
        // The joists go the shorter way: along x? The floor is 9 × 6, so
        // along y by itself; across the walls only if given.
        m.set_slab_joists(slab, Some(0.0)).unwrap();
        let td = crate::takedown(&m);
        let r = analyse(&m, &td);
        assert_eq!(r.len(), 1);
        let r = &r[0];
        assert!(r.loose < 0.1, "{r:?}");
        let inner: Vec<_> = r.bearings.iter().filter(|b| !b.end).collect();
        assert!(!inner.is_empty());
        // Open to the sky, so a roof: q ≈ 1,35·3,8 + 1,5·1 ≈ 6,6 kN/m²,
        // hogging q·L²/8 ≈ 17 over the middle wall, sagging ≈ 0,07–0,1·q·L²
        // with the pattern.
        let hog = inner[0].hogging;
        assert!((14.0..20.0).contains(&hog), "{hog}");
        let sag = r.spans.iter().map(|s| s.sagging).fold(0.0, f64::max);
        assert!((8.0..14.0).contains(&sag), "{sag}");
        // The pattern makes the sagging more than with both bays loaded.
        let c = check(&m, &td);
        assert!(
            c.iter()
                .any(|c| c.rule.starts_with("Momento positivo de la vigueta")),
            "{c:?}"
        );
        assert!(c.iter().any(|c| c.rule.starts_with("Flecha del forjado")));
        // The takedown sends it to the three walls, the middle one most.
        let auto = joist_direction(&m, slab).unwrap();
        assert!(auto.x.abs() > 0.99);
        m.set_slab_joists(slab, None).unwrap();
        let auto = joist_direction(&m, slab).unwrap();
        assert!(auto.y.abs() > 0.99, "{auto:?}");
    }

    #[test]
    fn ribs_cast_in_place() {
        let mut m = Model::new("Forjado");
        let sys = FloorSystem {
            joist: Joist::InSitu,
            ..FloorSystem::one_way()
        };
        let slab = house(&mut m, sys);
        m.set_slab_joists(slab, Some(0.0)).unwrap();
        let td = crate::takedown(&m);
        let c = check(&m, &td);
        let pos = c
            .iter()
            .find(|c| c.rule.starts_with("Momento positivo del nervio"))
            .unwrap();
        assert!(pos.what.contains("por nervio"), "{}", pos.what);
        assert_ne!(pos.status, Status::Fail, "{}", pos.what);
    }
}
