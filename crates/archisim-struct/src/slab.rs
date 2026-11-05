//! Concrete slabs as a grillage (emparrillado): a grid of bars in the
//! directions of the slab's first edge and across, each standing for its
//! strip (I = s·h³/12, J = s·h³/6), held where the walls, beams and
//! columns under it are. Solved by the frame's solver; the moments per
//! metre at the nodes go to the reinforcement by Wood–Armer (m ± |mxy|),
//! and the deflection under the quasi-permanent loads, times 3 for
//! cracking and creep, against the span / 400.

use archisim_core::{bar_area, Spaced, Vec2};
use archisim_core::{LayerFunction, Model, Point2, Reinforcement, SlabId};

use crate::cases::{combinations, Case, Kind};
use crate::concrete;
use crate::frame::{Bar, Frame, Load, Section, Solver, PINNED};
use crate::geo;
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
        if pl.base || pl.span.is_some() || pl.pitch > 0.05 || pl.outline.len() < 3 {
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
    let (e, g) = MatKind::Concrete.moduli();
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
    let cell = s * s;
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
    let steel = |m: f64| concrete::bending_steel(m, 1.0, d).map_or(f64::INFINITY, |a| a.max(amin));
    Some(SlabResult {
        slab: SlabId(0),
        spacing: s,
        nodes: nn,
        bottom,
        top,
        need_bottom: [steel(bottom[0]), steel(bottom[1])],
        need_top: [steel(top[0]), steel(top[1])],
        deflection: 3.0 * defl,
        limit: span / 400.0,
        thickness: h,
    })
}

fn per_m(sp: &Spaced) -> f64 {
    bar_area(sp.diameter) / sp.spacing.max(0.01)
}

/// The mesh that gives `area` per metre: Ø8 to Ø16, spacing to 5 cm.
pub fn mesh_for(area: f64) -> Spaced {
    for dm in [0.008, 0.010, 0.012, 0.016] {
        let sp = ((bar_area(dm) / area.max(1e-9)) * 20.0).floor() / 20.0;
        if sp >= 0.10 {
            return Spaced::new(dm, sp.min(0.25));
        }
    }
    Spaced::new(0.016, 0.10)
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
