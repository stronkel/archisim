//! Concrete slabs as a grillage (emparrillado): a grid of bars in the
//! directions of the slab's first edge and across, each standing for its
//! strip (I = s·h³/12, J = s·h³/6), held where the walls, beams and
//! columns under it are. Solved by the frame's solver; the moments per
//! metre at the nodes go to the reinforcement by Wood–Armer (m ± |mxy|),
//! and the deflection under the quasi-permanent loads, times 3 for
//! cracking and creep, against the span / 400.
//!
//! A foundation slab (raft) is the same grillage on springs (Winkler):
//! the modulus of subgrade reaction from the admissible pressure
//! (ks = 120·σadm, Bowles), its loads the walls and columns that stand on
//! it, and the pressure under it in service against σadm.
//!
//! Punching at each column (Código Estructural, art. 46; EN 1992-1-1,
//! 6.4): the control perimeter u1 at 2d, β 1,15 inside, 1,4 at an edge,
//! 1,5 at a corner, against vRd,c with the top steel; and the face of the
//! column against 0,5·ν·fcd.

use archisim_core::{bar_area, Spaced, Vec2};
use archisim_core::{LayerFunction, Model, Point2, Reinforcement, SlabId};

use crate::cases::{combinations, Case, Kind};
use crate::concrete::Rc;
use crate::frame::{Bar, Frame, Load, Section, Solver, PINNED};
use crate::geo;
use crate::gravity::Carrier;
use crate::gravity::{self, PlateSource, Takedown};
use crate::lightframe::{Check, Status, Subject};
use crate::sections::Kind as MatKind;
use crate::text::dec;

/// Most nodes of a grillage: the spacing grows to keep it under this.
const MAX_NODES: usize = 1800;

/// What a slab's grillage gives.
#[derive(Debug, Clone, PartialEq)]
pub struct SlabResult {
    pub slab: SlabId,
    pub spacing: f64,
    pub nodes: usize,
    /// Design moments per metre (kN·m/m), along the first edge and
    /// across: the largest sagging (bottom) and hogging (top).
    pub bottom: [f64; 2],
    pub top: [f64; 2],
    /// Steel they ask for, m²/m.
    pub need_bottom: [f64; 2],
    pub need_top: [f64; 2],
    /// Long-term deflection estimate and its limit, m.
    pub deflection: f64,
    pub limit: f64,
    pub thickness: f64,
    /// A raft: the largest pressure on the ground in service, kN/m².
    pub raft: Option<f64>,
    pub punching: Vec<Punch>,
}

/// Punching at one column.
#[derive(Debug, Clone, PartialEq)]
pub struct Punch {
    pub column: archisim_core::ColumnId,
    /// Design load through the slab, kN.
    pub ved: f64,
    pub beta: f64,
    /// Control perimeter at 2d, m.
    pub u1: f64,
    /// Stresses at u1 and the strength without reinforcement, N/mm².
    pub v_ed: f64,
    pub v_rd: f64,
    /// At the face of the column, utilisation of 0,5·ν·fcd.
    pub face: f64,
    /// Inside, edge or corner.
    pub place: &'static str,
    /// Punching reinforcement when vRd,c is short and 1,5·vRd,c is not
    /// (Código Estructural, art. 46.4; EN 1992-1-1, 6.4.5): legs per
    /// perimeter, their diameter (m), perimeters and spacing (m).
    pub links: Option<(u32, f64, u32, f64)>,
}

/// The punching reinforcement of a column: (legs per perimeter, bar
/// diameter, perimeters, spacing) for `v_ed`, `v_rd` (N/mm²) at the
/// perimeter `u1` (m), with `ved` (kN), `beta`, depth `d` (m) and the
/// column `c1` × `c2` at `near` free edges.
#[allow(clippy::too_many_arguments)]
pub fn punching_links(
    rc: &Rc,
    v_ed: f64,
    v_rd: f64,
    u1: f64,
    ved: f64,
    beta: f64,
    d: f64,
    c1: f64,
    c2: f64,
    near: usize,
) -> (u32, f64, u32, f64) {
    let dmm = d * 1000.0;
    let sr = (0.75 * d * 20.0).floor() / 20.0;
    let sr = sr.max(0.05);
    let fywd = (250.0 + 0.25 * dmm).min(rc.fyk / crate::concrete::GAMMA_S);
    // mm²: (vEd − 0,75·vRd,c)·u1·d / (1,5·(d/sr)·fywd,ef).
    let asw = (v_ed - 0.75 * v_rd).max(0.0) * u1 * 1000.0 * dmm / (1.5 * (d / sr) * fywd);
    let (dia, legs) = [0.008f64, 0.010, 0.012]
        .into_iter()
        .map(|db| {
            let a = std::f64::consts::PI * (db * 1000.0).powi(2) / 4.0;
            (db, ((asw / a).ceil() as u32).max(4))
        })
        .find(|(_, n)| *n <= 16)
        .unwrap_or((0.012, ((asw / 113.1).ceil() as u32).max(4)));
    // Out to where no reinforcement is needed: uout = β·VEd/(vRd,c·d),
    // the last perimeter within 1,5·d of it.
    let uout = beta * ved / (v_rd * 1000.0 * d);
    let pi = std::f64::consts::PI;
    let r = match near {
        0 => (uout - 2.0 * (c1 + c2)) / (2.0 * pi),
        1 => (uout - c1 - 2.0 * c2) / pi,
        _ => (uout - c1 - c2) / (pi / 2.0),
    };
    let rows = (((r - 1.5 * d - 0.5 * d) / sr).ceil() as i64 + 1).max(2) as u32;
    (legs, dia, rows, sr)
}

/// The modulus of subgrade reaction from the admissible pressure, kN/m³.
pub fn subgrade(bearing: f64) -> f64 {
    120.0 * bearing
}

fn concrete_slab(model: &Model, id: SlabId) -> Option<f64> {
    let s = model.slab(id)?;
    if model.type_framing(s.slab_type.0).is_some() {
        return None;
    }
    let t = model.slab_type(s.slab_type)?;
    let h: f64 = t
        .layers
        .iter()
        .filter(|l| {
            l.function == LayerFunction::Structure
                && MatKind::of(model.material(l.material)) == MatKind::Concrete
        })
        .map(|l| l.thickness)
        .sum();
    (h > 0.05).then_some(h)
}

/// The grillage of every concrete slab on supports.
pub fn analyse(model: &Model, td: &Takedown) -> Vec<SlabResult> {
    let plates = gravity::plates(model);
    let altitude = model.site_altitude();
    let mut out = Vec::new();
    for (i, pl) in plates.iter().enumerate() {
        let PlateSource::Slab(id) = pl.source else {
            continue;
        };
        if (pl.base && !pl.raft) || pl.span.is_some() || pl.pitch > 0.05 || pl.outline.len() < 3 {
            continue;
        }
        let Some(h) = concrete_slab(model, id) else {
            continue;
        };
        if let Some(r) = grillage(model, td, &plates, i, h, altitude) {
            out.push(SlabResult { slab: id, ..r });
        }
    }
    out
}

fn grillage(
    model: &Model,
    td: &Takedown,
    plates: &[gravity::Plate],
    i: usize,
    h: f64,
    altitude: f64,
) -> Option<SlabResult> {
    let pl = &plates[i];
    // Local axes: u along the first edge.
    let u = (pl.outline[1] - pl.outline[0]).normalized()?;
    let v = Vec2::new(-u.y, u.x);
    let o = pl.outline[0];
    let to_l = |p: Point2| {
        let d = p - o;
        (d.x * u.x + d.y * u.y, d.x * v.x + d.y * v.y)
    };
    let (mut a0, mut a1, mut b0, mut b1) = (f64::MAX, f64::MIN, f64::MAX, f64::MIN);
    for p in &pl.outline {
        let (a, b) = to_l(*p);
        a0 = a0.min(a);
        a1 = a1.max(a);
        b0 = b0.min(b);
        b1 = b1.max(b);
    }
    let area = geo::area(&pl.outline);
    let mut s = 0.5f64.max((area / MAX_NODES as f64).sqrt());
    s = (s * 20.0).ceil() / 20.0;
    let na = ((a1 - a0) / s).floor() as i64 + 1;
    let nb = ((b1 - b0) / s).floor() as i64 + 1;
    let at = |ia: i64, ib: i64| -> Point2 {
        let a = a0 + s / 2.0 + ia as f64 * s;
        let b = b0 + s / 2.0 + ib as f64 * s;
        o + u * a + v * b
    };
    let mut index = std::collections::HashMap::new();
    let mut frame = Frame::default();
    let z = (pl.top)(pl.outline[0]);
    for ia in 0..na {
        for ib in 0..nb {
            let p = at(ia, ib);
            if geo::inside(p, &pl.outline) && !pl.holes.iter().any(|hl| geo::inside(p, hl)) {
                index.insert((ia, ib), frame.nodes.len());
                frame.nodes.push([p.x, p.y, z]);
            }
        }
    }
    if frame.nodes.len() < 4 {
        return None;
    }
    let e = Rc::of(model).ecm();
    let g = e / 2.4;
    let section = Section {
        a: s * h,
        iy: s * h.powi(3) / 12.0,
        iz: h * s.powi(3) / 12.0,
        j: s * h.powi(3) / 6.0,
        e,
        g,
    };
    // Bars: along u (dir 0) and along v (dir 1).
    let mut dirs: Vec<usize> = Vec::new();
    let mut ends: Vec<[usize; 2]> = Vec::new();
    for (&(ia, ib), &n) in &index {
        for (d, (ja, jb)) in [(0usize, (ia + 1, ib)), (1, (ia, ib + 1))] {
            if let Some(&m) = index.get(&(ja, jb)) {
                frame.bars.push(Bar {
                    i: n,
                    j: m,
                    section,
                    up: [0.0, 0.0, 1.0],
                    hinge_i: false,
                    hinge_j: false,
                });
                dirs.push(d);
                ends.push([n, m]);
            }
        }
    }
    // Supports: the nodes on the walls and beams under it, the nearest to
    // each column.
    let supports = gravity::plate_supports(model, td, pl);
    let mut held = vec![false; frame.nodes.len()];
    for sp in &supports {
        if sp.path.len() == 1 {
            let c = sp.path[0];
            if let Some((n, d)) = frame
                .nodes
                .iter()
                .enumerate()
                .map(|(n, p)| (n, Point2::new(p[0], p[1]).distance(c)))
                .min_by(|a, b| a.1.total_cmp(&b.1))
            {
                if d < s {
                    held[n] = true;
                }
            }
        } else {
            for (n, p) in frame.nodes.iter().enumerate() {
                if geo::to_path(Point2::new(p[0], p[1]), &sp.path).0 < 0.75 * s {
                    held[n] = true;
                }
            }
        }
    }
    let cell = s * s;
    let ks = pl.raft.then(|| subgrade(model.structural_site().bearing));
    if let Some(ks) = ks {
        // A raft lies on the ground: a spring under each node (and weak
        // ones in its plane).
        held.iter_mut().for_each(|h| *h = false);
        frame.springs = (0..frame.nodes.len())
            .map(|n| (n, [1.0, 1.0, ks * cell, 0.0, 0.0, 1.0]))
            .collect();
    } else {
        let count = held.iter().filter(|h| **h).count();
        if count < 3 {
            return None;
        }
        frame.supports = held
            .iter()
            .enumerate()
            .filter(|(_, h)| **h)
            .map(|(n, _)| (n, PINNED))
            .collect();
    }
    // The columns: under it (reactions) or on it, for a raft (their loads).
    let mut columns: Vec<(archisim_core::ColumnId, usize)> = Vec::new();
    if pl.raft {
        let z = (pl.top)(pl.outline[0]);
        for c in model.columns() {
            let base = model.level(c.level).map_or(0.0, |l| l.elevation) + c.base_offset;
            if (base - z).abs() < 0.3 && geo::inside(c.position, &pl.outline) {
                if let Some(n) = nearest_node(&frame, c.position, s) {
                    columns.push((c.id, n));
                }
            }
        }
    } else {
        for sp in &supports {
            if let (Carrier::Column(id), 1) = (sp.carrier, sp.path.len()) {
                if let Some(n) = nearest_node(&frame, sp.path[0], s) {
                    if held[n] {
                        columns.push((id, n));
                    }
                }
            }
        }
    }
    // In-plane movements and the turn about z of the free nodes are held
    // by the bars; springs keep the solver steady.
    // Loads: each node takes its cell.
    let nearest = |p: Point2| -> usize {
        let (a, b) = to_l(p);
        let ia = (((a - a0 - s / 2.0) / s).round() as i64).clamp(0, na - 1);
        let ib = (((b - b0 - s / 2.0) / s).round() as i64).clamp(0, nb - 1);
        if let Some(&n) = index.get(&(ia, ib)) {
            return n;
        }
        frame
            .nodes
            .iter()
            .enumerate()
            .min_by(|x, y| {
                Point2::new(x.1[0], x.1[1])
                    .distance(p)
                    .total_cmp(&Point2::new(y.1[0], y.1[1]).distance(p))
            })
            .map_or(0, |(n, _)| n)
    };
    let rooms: Vec<_> = model.rooms_on_level(pl.level).collect();
    let mut by_case: std::collections::BTreeMap<Case, Vec<Load>> = Default::default();
    for (n, p) in frame.nodes.iter().enumerate() {
        let pt = Point2::new(p[0], p[1]);
        for (case, q) in gravity::area_loads(model, plates, i, &rooms, pt) {
            by_case.entry(case).or_default().push(Load::Node {
                node: n,
                f: [0.0, 0.0, -q * cell, 0.0, 0.0, 0.0],
            });
        }
    }
    if let Some(inc) = td.plate_incoming.get(i) {
        for (p, case, kn) in inc {
            by_case.entry(*case).or_default().push(Load::Node {
                node: nearest(*p),
                f: [0.0, 0.0, -kn, 0.0, 0.0, 0.0],
            });
        }
    }
    let cases: Vec<Case> = by_case.keys().copied().collect();
    let solver = Solver::new(&frame);
    let sols: Vec<_> = cases.iter().map(|c| solver.solve(&by_case[c])).collect();
    // Moments per metre at the nodes, by case: [m along u, m along v, mxy].
    let nn = frame.nodes.len();
    let node_moments = |sol: &crate::frame::Solution| -> Vec<[f64; 3]> {
        let mut sum = vec![[0.0f64; 3]; nn];
        let mut cnt = vec![[0usize; 2]; nn];
        for (b, [ni, nj]) in ends.iter().enumerate() {
            let ef = &sol.end_forces[b];
            let fi = crate::frame::internal(ef, &[], 0.0);
            let l = s;
            let fj = crate::frame::internal(ef, &[], l);
            let d = dirs[b];
            for (n, f) in [(*ni, fi), (*nj, fj)] {
                sum[n][d] += f[4] / s;
                sum[n][2] += f[3].abs() / s;
                cnt[n][d] += 1;
            }
        }
        sum.iter()
            .zip(&cnt)
            .map(|(m, c)| {
                let k = (c[0] + c[1]).max(1) as f64;
                [
                    m[0] / c[0].max(1) as f64,
                    m[1] / c[1].max(1) as f64,
                    m[2] / k,
                ]
            })
            .collect()
    };
    let per_case: Vec<Vec<[f64; 3]>> = sols.iter().map(node_moments).collect();
    let combos = combinations(&cases, altitude);
    let mut bottom = [0.0f64; 2];
    let mut top = [0.0f64; 2];
    let mut defl = 0.0f64;
    let mut pressure = 0.0f64;
    // The load each column puts through the slab, ultimate, kN.
    let mut ved = vec![0.0f64; columns.len()];
    for combo in combos.iter().filter(|c| c.kind == Kind::Ultimate) {
        for (k, (id, n)) in columns.iter().enumerate() {
            let here = model
                .column(*id)
                .map_or(Point2::new(frame.nodes[*n][0], frame.nodes[*n][1]), |c| {
                    c.position
                });
            let v: f64 = cases
                .iter()
                .enumerate()
                .map(|(ci, c)| {
                    let f = combo.factor(*c);
                    if ks.is_some() {
                        // On a raft: what reaches the node of the column.
                        f * td.plate_incoming.get(i).map_or(0.0, |inc| {
                            inc.iter()
                                .filter(|(p, cs, _)| cs == c && p.distance(here) < 0.3)
                                .map(|(_, _, kn)| kn)
                                .sum::<f64>()
                        })
                    } else {
                        f * sols[ci]
                            .reactions
                            .iter()
                            .find(|(m, _)| m == n)
                            .map_or(0.0, |(_, r)| r[2])
                    }
                })
                .sum();
            ved[k] = ved[k].max(v);
        }
    }
    for combo in &combos {
        match combo.kind {
            Kind::Ultimate => {
                for n in 0..nn {
                    let mut m = [0.0; 3];
                    for (ci, c) in cases.iter().enumerate() {
                        let f = combo.factor(*c);
                        for k in 0..3 {
                            m[k] += f * per_case[ci][n][k];
                        }
                    }
                    for d in 0..2 {
                        bottom[d] = bottom[d].max(m[d] + m[2].abs());
                        top[d] = top[d].max(-(m[d] - m[2].abs()));
                    }
                }
            }
            Kind::Characteristic if ks.is_some() => {
                let ks = ks.unwrap_or(0.0);
                for n in 0..nn {
                    let w: f64 = cases
                        .iter()
                        .enumerate()
                        .map(|(ci, c)| combo.factor(*c) * sols[ci].displacements[n][2])
                        .sum();
                    pressure = pressure.max(-w * ks);
                }
            }
            Kind::QuasiPermanent => {
                for n in 0..nn {
                    let w: f64 = cases
                        .iter()
                        .enumerate()
                        .map(|(ci, c)| combo.factor(*c) * sols[ci].displacements[n][2])
                        .sum();
                    defl = defl.max(-w);
                }
            }
            _ => {}
        }
    }
    // The span: twice the farthest a node is from what holds it.
    let held_pts: Vec<Point2> = frame
        .supports
        .iter()
        .map(|(n, _)| Point2::new(frame.nodes[*n][0], frame.nodes[*n][1]))
        .collect();
    let far = frame
        .nodes
        .iter()
        .map(|p| {
            let q = Point2::new(p[0], p[1]);
            held_pts
                .iter()
                .map(|h| h.distance(q))
                .fold(f64::INFINITY, f64::min)
        })
        .fold(0.0, f64::max);
    let span = (2.0 * far).max(s);
    let d = h - 0.035;
    let amin = 0.0018 * h;
    let rc = Rc::of(model);
    let steel = |m: f64| {
        rc.bending_steel(m, 1.0, d)
            .map_or(f64::INFINITY, |a| a.max(amin))
    };
    let need_top = [steel(top[0]), steel(top[1])];
    // Punching.
    let punching = columns
        .iter()
        .zip(&ved)
        .filter(|(_, v)| **v > 0.0)
        .filter_map(|((id, _), v)| {
            let c = model.column(*id)?;
            let (c1, c2) = model.member_type(c.member_type)?.profile.size();
            let n = pl.outline.len();
            let near = (0..n)
                .filter(|&k| {
                    geo::to_segment(c.position, pl.outline[k], pl.outline[(k + 1) % n]).0
                        < c1.max(c2) / 2.0 + 2.0 * d
                })
                .count();
            let rho = (need_top[0].max(need_top[1]) / d).clamp(0.002, 0.02);
            Some(punch(&rc, *id, *v, c1, c2, d, near, rho))
        })
        .collect();
    Some(SlabResult {
        slab: SlabId(0),
        spacing: s,
        nodes: nn,
        bottom,
        top,
        need_bottom: [steel(bottom[0]), steel(bottom[1])],
        need_top,
        deflection: 3.0 * defl,
        limit: span / 400.0,
        thickness: h,
        raft: ks.map(|_| pressure),
        punching,
    })
}

fn nearest_node(frame: &Frame, p: Point2, s: f64) -> Option<usize> {
    frame
        .nodes
        .iter()
        .enumerate()
        .map(|(n, q)| (n, Point2::new(q[0], q[1]).distance(p)))
        .min_by(|a, b| a.1.total_cmp(&b.1))
        .filter(|(_, d)| *d < s)
        .map(|(n, _)| n)
}

/// Punching of a column `c1` × `c2` through a slab of depth `d` with `near`
/// free edges close by and top steel ratio `rho`.
#[allow(clippy::too_many_arguments)]
pub fn punch(
    rc: &Rc,
    column: archisim_core::ColumnId,
    ved: f64,
    c1: f64,
    c2: f64,
    d: f64,
    near: usize,
    rho: f64,
) -> Punch {
    let pi = std::f64::consts::PI;
    let (beta, u1, u0, place) = match near {
        0 => (
            1.15,
            2.0 * (c1 + c2) + 4.0 * pi * d,
            2.0 * (c1 + c2),
            "interior",
        ),
        1 => (
            1.4,
            c1 + 2.0 * c2 + 2.0 * pi * d,
            c2 + 3.0 * d.min(c1 / 2.0) + c1,
            "de borde",
        ),
        _ => (1.5, c1 + c2 + pi * d, (3.0 * d).min(c1 + c2), "de esquina"),
    };
    let dmm = d * 1000.0;
    let xi = (1.0 + (200.0 / dmm).sqrt()).min(2.0);
    let g = crate::concrete::GAMMA_C;
    let vrd = (0.18 / g * xi * (100.0 * rho * rc.fck).cbrt())
        .max(0.075 / g * xi.powf(1.5) * rc.fck.sqrt());
    // kN/m² → N/mm².
    let v_ed = beta * ved / (u1 * d) / 1000.0;
    let nu = 0.6 * (1.0 - rc.fck / 250.0);
    let face = beta * ved / (u0 * d) / (0.5 * nu * rc.fcd());
    let links = (v_ed > vrd).then(|| punching_links(rc, v_ed, vrd, u1, ved, beta, d, c1, c2, near));
    Punch {
        column,
        ved,
        beta,
        u1,
        v_ed,
        v_rd: vrd,
        face,
        place,
        links,
    }
}

fn per_m(sp: &Spaced) -> f64 {
    bar_area(sp.diameter) / sp.spacing.max(0.01)
}

/// The mesh that gives `area` per metre: Ø8 to Ø20, at 15 cm or more if
/// it can, else at 10 cm or more; spacing to 5 cm, 25 cm at most.
pub fn mesh_for(area: f64) -> Spaced {
    const D: [f64; 5] = [0.008, 0.010, 0.012, 0.016, 0.020];
    for least in [0.15, 0.10] {
        for dm in D {
            let sp = ((bar_area(dm) / area.max(1e-9)) * 20.0).floor() / 20.0;
            if sp >= least {
                return Spaced::new(dm, sp.min(0.25));
            }
        }
    }
    Spaced::new(0.020, 0.10)
}

pub fn check(model: &Model, td: &Takedown) -> Vec<Check> {
    let mut out = Vec::new();
    for r in analyse(model, td) {
        let subject = Subject::Floor(r.slab);
        let name = model
            .slab(r.slab)
            .and_then(|s| model.slab_type(s.slab_type))
            .map_or(String::new(), |t| t.name.clone());
        let dir = ["paralelo al primer lado", "perpendicular"];
        let need = format!(
            "{name} (emparrillado de {} m, {} nudos): m+ {} / {} kN·m/m, m− {} / {} kN·m/m ({} y {})",
            dec(r.spacing, 2),
            r.nodes,
            dec(r.bottom[0], 1),
            dec(r.bottom[1], 1),
            dec(r.top[0], 1),
            dec(r.top[1], 1),
            dir[0],
            dir[1]
        );
        let fails_concrete = r
            .need_bottom
            .iter()
            .chain(&r.need_top)
            .any(|a| !a.is_finite());
        if fails_concrete {
            out.push(Check {
                subject,
                what: format!("{need}: el canto no basta"),
                rule: "Flexión de la losa (Código Estructural, art. 42)",
                ratio: 2.0,
                status: Status::Fail,
                advice: Some("más canto".into()),
            });
            continue;
        }
        let proposal = format!(
            "abajo {} y {}, arriba {} y {}",
            mesh_for(r.need_bottom[0]).label(),
            mesh_for(r.need_bottom[1]).label(),
            mesh_for(r.need_top[0]).label(),
            mesh_for(r.need_top[1]).label()
        );
        match model.reinforcement(r.slab.0) {
            Some(Reinforcement::Slab { bottom, top, .. }) => {
                let tops = top.map_or([0.0; 2], |t| [per_m(&t[0]), per_m(&t[1])]);
                let mut u = 0.0f64;
                for k in 0..2 {
                    u = u.max(r.need_bottom[k] / per_m(&bottom[k]).max(1e-9));
                    if r.top[k] > 1.0 {
                        u = u.max(r.need_top[k] / tops[k].max(1e-9));
                    }
                }
                out.push(Check {
                    subject,
                    what: format!(
                        "{need}; tiene abajo {} y {}{}",
                        bottom[0].label(),
                        bottom[1].label(),
                        top.map_or(String::new(), |t| format!(
                            ", arriba {} y {}",
                            t[0].label(),
                            t[1].label()
                        ))
                    ),
                    rule: "Armado de la losa (Código Estructural, art. 42; Wood–Armer)",
                    ratio: u,
                    status: if u <= 1.0 { Status::Pass } else { Status::Fail },
                    advice: (u > 1.0).then(|| format!("{proposal} (con refuerzos donde haga falta)")),
                });
            }
            _ => out.push(Check {
                subject,
                what: format!("{need}: pide {proposal}"),
                rule: "Armado de la losa (Código Estructural, art. 42; Wood–Armer)",
                ratio: 0.0,
                status: Status::Info,
                advice: Some(
                    "la losa no tiene armado dibujado: es el armado de los puntos más cargados, una base menor con refuerzos ahorra acero".into(),
                ),
            }),
        }
        for p in &r.punching {
            let plain = p.v_ed / p.v_rd.max(1e-9);
            // With reinforcement up to 1,5·vRd,c (kmax), and the face.
            let reinforced = p.v_ed / (1.5 * p.v_rd).max(1e-9);
            let (ratio, status, advice) = if plain <= 1.0 && p.face <= 1.0 {
                (plain.max(p.face), Status::Pass, None)
            } else if reinforced <= 1.0 && p.face <= 1.0 {
                let (legs, dia, rows, sr) = p.links.unwrap_or((0, 0.0, 0, 0.0));
                (
                    reinforced,
                    Status::Warn,
                    Some(format!(
                        "armadura de punzonamiento: {rows} perímetros de {legs} ramas Ø{} cada {} cm, el primero a menos de 0,5·d de la cara del pilar",
                        dec(dia * 1000.0, 0),
                        dec(sr * 100.0, 0)
                    )),
                )
            } else {
                (
                    reinforced.max(p.face),
                    Status::Fail,
                    Some(if p.face > 1.0 {
                        "más canto o un capitel: la biela de la cara del pilar no basta".into()
                    } else {
                        "más canto o un ábaco: ni con armadura de punzonamiento (1,5·vRd,c) llega"
                            .into()
                    }),
                )
            };
            out.push(Check {
                subject,
                what: format!(
                    "{name}, pilar {} ({}): VEd {} kN, β {}, u1 {} m, vEd {} N/mm² sobre vRd,c {} N/mm²; en la cara, {} de 0,5·ν·fcd",
                    p.column.0,
                    p.place,
                    dec(p.ved, 1),
                    dec(p.beta, 2),
                    dec(p.u1, 2),
                    dec(p.v_ed, 3),
                    dec(p.v_rd, 3),
                    dec(p.face, 2)
                ),
                rule: "Punzonamiento (Código Estructural, art. 46)",
                ratio,
                status,
                advice,
            });
        }
        if let Some(pressure) = r.raft {
            let sigma = model.structural_site().bearing;
            let u = pressure / sigma.max(1e-6);
            out.push(Check {
                subject,
                what: format!(
                    "{name} (losa de cimentación, balasto {} kN/m³): presión máxima en servicio {} kN/m² sobre {} admisibles",
                    dec(subgrade(sigma), 0),
                    dec(pressure, 0),
                    dec(sigma, 0)
                ),
                rule: "Presión de la losa sobre el terreno (DB-SE-C, 4.3)",
                ratio: u,
                status: if u <= 1.0 { Status::Pass } else { Status::Fail },
                advice: (u > 1.0).then(|| "más canto (reparte mejor), más losa o mejorar el terreno".into()),
            });
            continue;
        }
        let u = r.deflection / r.limit.max(1e-6);
        out.push(Check {
            subject,
            what: format!(
                "{name}: flecha a largo plazo {} mm (elástica × 3 por fisuración y fluencia), límite {} mm",
                dec(r.deflection * 1000.0, 1),
                dec(r.limit * 1000.0, 1)
            ),
            rule: "Flecha de la losa (DB-SE 4.3.3.1, L/400)",
            ratio: u,
            status: if u <= 1.0 { Status::Pass } else { Status::Fail },
            advice: (u > 1.0).then(|| format!("más canto (h {} m)", dec(r.thickness + 0.05, 2))),
        });
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use archisim_core::*;

    /// A flat slab on nine columns 6 m apart, on a raft.
    fn flat_slab(m: &mut Model, raft: bool) {
        let l0 = m.add_level("Planta baja", 0.0);
        let l1 = m.add_level("Planta 1", 3.0);
        let lib = archisim_library::add_library(m).unwrap();
        let concrete = m
            .materials()
            .find(|x| x.name.to_lowercase().contains("hormigón armado"))
            .or_else(|| {
                m.materials()
                    .find(|x| x.name.to_lowercase().contains("hormigón"))
            })
            .unwrap()
            .id;
        let flat = m
            .add_slab_type(
                "Losa maciza 25",
                SlabFunction::Floor,
                vec![Layer {
                    material: concrete,
                    thickness: 0.25,
                    function: LayerFunction::Structure,
                }],
            )
            .unwrap();
        for i in 0..3 {
            for j in 0..3 {
                m.add_column(NewColumn {
                    member_type: lib.column,
                    level: l0,
                    position: Point2::new(i as f64 * 6.0, j as f64 * 6.0),
                    rotation: 0.0,
                    base_offset: 0.0,
                    top: WallTop::Unconnected,
                    height: 3.0,
                })
                .unwrap();
            }
        }
        let square = |a: f64| {
            vec![
                Point2::new(-a, -a),
                Point2::new(12.0 + a, -a),
                Point2::new(12.0 + a, 12.0 + a),
                Point2::new(-a, 12.0 + a),
            ]
        };
        m.add_slab(NewSlab {
            slab_type: flat,
            level: l1,
            outline: square(0.5),
            top_offset: 0.0,
        })
        .unwrap();
        if raft {
            let rt = m
                .add_slab_type(
                    "Losa de cimentación 60",
                    SlabFunction::BaseSlab,
                    vec![Layer {
                        material: concrete,
                        thickness: 0.6,
                        function: LayerFunction::Structure,
                    }],
                )
                .unwrap();
            m.add_slab(NewSlab {
                slab_type: rt,
                level: l0,
                outline: square(1.0),
                top_offset: 0.0,
            })
            .unwrap();
        }
    }

    #[test]
    fn a_flat_slab_punches_and_a_raft_bears() {
        let mut m = Model::new("Losa sobre pilares");
        flat_slab(&mut m, true);
        let td = crate::takedown(&m);
        let r = analyse(&m, &td);
        let floor = r.iter().find(|r| r.raft.is_none()).expect("the floor");
        // Nine columns: one inside, four at edges, four at corners.
        assert_eq!(floor.punching.len(), 9, "{:?}", floor.punching);
        let inside = floor
            .punching
            .iter()
            .find(|p| p.place == "interior")
            .unwrap();
        // The inner column takes some 6 × 6 m of floor: 300–700 kN.
        assert!((200.0..900.0).contains(&inside.ved), "{inside:?}");
        assert!(inside.v_ed > 0.3 && inside.v_rd > 0.3, "{inside:?}");
        let raft = r.iter().find(|r| r.raft.is_some()).expect("the raft");
        let p = raft.raft.unwrap();
        // The building's weight over 196 m² of raft: tens of kN/m².
        assert!((5.0..200.0).contains(&p), "{p}");
        assert_eq!(raft.punching.len(), 9);
        let c = check(&m, &td);
        assert!(c.iter().any(|c| c.rule.starts_with("Punzonamiento")));
        assert!(c.iter().any(|c| c.rule.starts_with("Presión de la losa")));
        // The columns stand on the raft: no footings asked for them.
        let f = crate::foundations::check(&m, &td);
        assert!(f.is_empty(), "{f:?}");
    }

    #[test]
    fn punching_reinforcement() {
        // An inner 30 × 30 column under 25 cm of slab (d 0,215 m) with
        // 600 kN: u1 = 1,2 + 4π·0,215 = 3,90 m, vEd = 1,15·600/(3,90·0,215)
        // = 0,82 N/mm² over some 0,55: reinforcement, less than 1,5·vRd,c.
        let rc = Rc::default();
        let p = punch(&rc, ColumnId(1), 600.0, 0.3, 0.3, 0.215, 0, 0.01);
        assert!((p.u1 - (1.2 + 4.0 * std::f64::consts::PI * 0.215)).abs() < 1e-9);
        assert!(p.v_ed > p.v_rd && p.v_ed < 1.5 * p.v_rd, "{p:?}");
        let (legs, dia, rows, sr) = p.links.unwrap();
        // Asw = (0,82 − 0,75·vRd,c)·u1·d / (1,5·(d/sr)·fywd,ef), fywd,ef
        // = 250 + 0,25·215 = 304 N/mm².
        let asw = (p.v_ed - 0.75 * p.v_rd) * p.u1 * 1000.0 * 215.0 / (1.5 * (0.215 / sr) * 303.75);
        let has = f64::from(legs) * std::f64::consts::PI * (dia * 1000.0).powi(2) / 4.0;
        assert!(has >= asw && has < asw + 120.0, "{has} {asw}");
        assert!((2..12).contains(&rows), "{rows}");
        assert!((sr - 0.15).abs() < 1e-9);
    }

    #[test]
    fn a_slab_on_four_walls() {
        let mut m = Model::new("Losa");
        let l0 = m.add_level("Planta baja", 0.0);
        let l1 = m.add_level("Planta 1", 3.0);
        let lib = archisim_library::add_library(&mut m).unwrap();
        let pts = [
            Point2::new(0.0, 0.0),
            Point2::new(5.0, 0.0),
            Point2::new(5.0, 5.0),
            Point2::new(0.0, 5.0),
        ];
        for k in 0..4 {
            m.add_wall(NewWall {
                wall_type: lib.facade,
                level: l0,
                start: pts[k],
                end: pts[(k + 1) % 4],
                bulge: 0.0,
                reference: ReferenceLine::Center,
                height: 3.0,
                base_offset: 0.0,
                top: WallTop::Unconnected,
            })
            .unwrap();
        }
        m.add_slab(NewSlab {
            slab_type: lib.floor_slab,
            level: l1,
            outline: pts.to_vec(),
            top_offset: 0.0,
        })
        .unwrap();
        let td = crate::takedown(&m);
        let r = analyse(&m, &td);
        assert_eq!(r.len(), 1, "the template's floor is concrete");
        let r = &r[0];
        // A square simply supported plate (ν 0): m ≈ 0,0479·q·a² at the
        // centre, with torsion; the grillage gives the same order.
        assert!(r.bottom[0] > 5.0 && r.bottom[0] < 80.0, "{r:?}");
        assert!(
            (r.bottom[0] - r.bottom[1]).abs() < 0.15 * r.bottom[0],
            "{r:?}"
        );
        assert!(r.deflection > 0.0, "{r:?}");
        let c = check(&m, &td);
        assert!(c.iter().any(|c| c.rule.starts_with("Armado")));
    }
}
