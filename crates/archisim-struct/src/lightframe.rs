//! The checks of the light frames (steel frame to EN 1993-1-3, timber
//! frame to DB-SE-M and EN 1995-1-1), member by member, with the loads of
//! the takedown and the combinations of DB-SE:
//!
//! - joists and rafters: each one a continuous beam over the walls and
//!   beams it crosses (the frame solver), with its strip of floor or
//!   roof, the walls and loads standing on it; bending, shear, bearing
//!   (web crippling in steel), deflection and, in floors, vibration;
//!   rafters with the thrust of their tie, and the ties in tension;
//! - studs: the load of the wall's top line over their spacing and, in
//!   exterior walls, the wind on their face: compression with buckling
//!   and bending (EN 1993-1-3 6.3 or EN 1995-1-1 6.3.2), and in timber
//!   the bearing on the sole plate;
//! - headers and the jack studs under them;
//! - the walls as shear walls (EN 1995-1-1, 9.2.4.2, method A, for both
//!   systems) against the wind of each storey, with the tension at the
//!   ends of each panel for its hold-downs;
//! - the fire resistance their boards give, against DB-SI 6.

use archisim_core::{FrameMaterial, Framing, Model, Point2, RoofId, SlabId, WallId};
use archisim_geom::framing::{roof_members, slab_members, wall_members, Member, Role};

use crate::cases::{combinations, Case, Combination, Kind, WindDir};
use crate::cold::{self, Cold};
use crate::frame::{internal, Bar, Frame, Load, Section, Solver};
use crate::gravity::{self, area_loads, Carrier, PlateSource, Takedown, BIN};
use crate::timber::{self, Piece};
use crate::wind::{wall_pressure, BuildingWind};
use crate::{geo, weights};

/// What a check is about.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Subject {
    Wall(WallId),
    Floor(SlabId),
    Roof(RoofId),
    Building,
}

/// How a check came out.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Status {
    Pass,
    /// Passes but needs something checked or decided by hand.
    Warn,
    Fail,
    /// For the record.
    Info,
}

/// One check.
#[derive(Debug, Clone, PartialEq)]
pub struct Check {
    pub subject: Subject,
    /// The member and what it carries, written out.
    pub what: String,
    /// The rule (and its clause).
    pub rule: &'static str,
    /// Utilisation: 1,00 is the limit.
    pub ratio: f64,
    pub status: Status,
    /// What would make it pass, or what to do.
    pub advice: Option<String>,
}

fn status(ratio: f64) -> Status {
    if ratio <= 1.0 {
        Status::Pass
    } else {
        Status::Fail
    }
}

fn dec(v: f64, p: usize) -> String {
    crate::text::dec(v, p)
}

/// The variable cases of a combination's wind: the largest factor of any
/// wind direction in it.
fn wind_factor(c: &Combination) -> f64 {
    c.factors
        .iter()
        .filter(|(k, _)| matches!(k, Case::Wind(_)))
        .map(|(_, f)| *f)
        .fold(0.0, f64::max)
}

/// The resistances of a frame's section.
#[derive(Debug, Clone, Copy)]
enum Mat {
    Steel(Cold),
    Timber(Piece),
}

impl Mat {
    fn of(f: &Framing, track: bool) -> Mat {
        let s = if track { &f.track } else { &f.stud };
        match f.material {
            FrameMaterial::Steel => Mat::Steel(Cold::from_section(s)),
            FrameMaterial::Timber => Mat::Timber(Piece::from_section(s)),
        }
    }

    /// E·I about the strong axis, kN·m².
    fn ei(&self) -> f64 {
        match self {
            Mat::Steel(c) => cold::E * 1000.0 * c.gross().iy * 1e-12,
            Mat::Timber(p) => timber::E0_MEAN * 1000.0 * p.i_strong() * 1e-12,
        }
    }

    fn section(&self) -> Section {
        let ei = self.ei();
        let (e, g) = match self {
            Mat::Steel(_) => (210e6, 81e6),
            Mat::Timber(_) => (11e6, 0.69e6),
        };
        let iy = ei / e;
        Section {
            a: 1e-3,
            iy,
            iz: iy * 0.2,
            j: iy * 0.05,
            e,
            g,
        }
    }
}

/// A beam on supports: positions of the supports (m along it) and point
/// loads by case (m, kN down). Returns, for each combination: the
/// largest |M| in span (sagging), the largest hogging, |V| max, the
/// largest reaction at an end and inside, and the relative deflection at
/// the nodes (m), per case.
struct BeamResult {
    length: f64,
    /// Per combination: (M sag, M hog, V, R end, R inside).
    per_combo: Vec<[f64; 5]>,
    /// Per case: the largest relative deflection of any span, m, and the
    /// span it is in.
    deflection: std::collections::BTreeMap<Case, f64>,
    /// The longest span between supports, m.
    span: f64,
    cantilever: bool,
}

fn continuous_beam(
    length: f64,
    supports: &[f64],
    loads: &[(f64, Case, f64)],
    sec: Section,
    combos: &[Combination],
    cases: &[Case],
) -> Option<BeamResult> {
    if length < 0.2 {
        return None;
    }
    let mut sup: Vec<f64> = supports.iter().map(|s| s.clamp(0.0, length)).collect();
    sup.sort_by(f64::total_cmp);
    sup.dedup_by(|a, b| (*a - *b).abs() < 0.1);
    if sup.is_empty() {
        return None;
    }
    if sup.len() == 1 {
        // One support only: hold it at its ends too (a joist held at
        // one wall and its rim).
        let far = if sup[0] < length / 2.0 { length } else { 0.0 };
        sup.push(far);
        sup.sort_by(f64::total_cmp);
    }
    // Nodes: the ends, the supports and quarter points of each stretch.
    let mut xs: Vec<f64> = vec![0.0, length];
    xs.extend(sup.iter().copied());
    let mut cuts = xs.clone();
    cuts.sort_by(f64::total_cmp);
    cuts.dedup_by(|a, b| (*a - *b).abs() < 1e-6);
    let mut nodes_x: Vec<f64> = Vec::new();
    for w in cuts.windows(2) {
        for k in 0..4 {
            nodes_x.push(w[0] + (w[1] - w[0]) * k as f64 / 4.0);
        }
    }
    nodes_x.push(length);
    nodes_x.dedup_by(|a, b| (*a - *b).abs() < 1e-6);
    let frame = Frame {
        nodes: nodes_x.iter().map(|x| [*x, 0.0, 0.0]).collect(),
        bars: (0..nodes_x.len() - 1)
            .map(|i| Bar {
                i,
                j: i + 1,
                section: sec,
                up: [0.0, 0.0, 1.0],
                hinge_i: false,
                hinge_j: false,
            })
            .collect(),
        supports: sup
            .iter()
            .enumerate()
            .filter_map(|(k, s)| {
                let n = nodes_x.iter().position(|x| (x - s).abs() < 1e-6)?;
                Some((
                    n,
                    if k == 0 {
                        [true, true, true, true, false, false]
                    } else {
                        [false, true, true, false, false, false]
                    },
                ))
            })
            .collect(),
        springs: Vec::new(),
    };
    let solver = Solver::new(&frame);
    let bar_of = |x: f64| -> (usize, f64) {
        let i = nodes_x
            .windows(2)
            .position(|w| x >= w[0] - 1e-9 && x <= w[1] + 1e-9)
            .unwrap_or(nodes_x.len() - 2);
        (i, (x - nodes_x[i]).max(0.0))
    };
    let mut per_case = std::collections::BTreeMap::new();
    let mut deflection = std::collections::BTreeMap::new();
    for case in cases {
        let ls: Vec<Load> = loads
            .iter()
            .filter(|(_, c, _)| c == case)
            .map(|(x, _, kn)| {
                let (bar, at) = bar_of(*x);
                Load::Point {
                    bar,
                    at,
                    f: [0.0, 0.0, -kn],
                }
            })
            .collect();
        let sol = solver.solve(&ls);
        // Internal forces at every node (from the bar starting there,
        // and the last bar's end).
        let mut f = Vec::new();
        for b in 0..frame.bars.len() {
            let bl = solver.bar_loads(&ls, b);
            let l = solver.bar_length(b);
            f.push(internal(&sol.end_forces[b], &bl, 1e-9));
            f.push(internal(&sol.end_forces[b], &bl, l / 2.0));
            if b + 1 == frame.bars.len() {
                f.push(internal(&sol.end_forces[b], &bl, l - 1e-9));
            }
        }
        let r: Vec<(usize, f64)> = sol.reactions.iter().map(|(n, r)| (*n, r[2])).collect();
        // Deflection relative to the line between each span's supports.
        let mut worst: f64 = 0.0;
        let sup_nodes: Vec<usize> = frame.supports.iter().map(|s| s.0).collect();
        for (n, x) in nodes_x.iter().enumerate() {
            let w = sol.displacements[n][2];
            let left = sup_nodes.iter().rev().find(|&&s| nodes_x[s] <= *x);
            let right = sup_nodes.iter().find(|&&s| nodes_x[s] >= *x);
            let base = match (left, right) {
                (Some(&a), Some(&b)) if a != b => {
                    let (xa, xb) = (nodes_x[a], nodes_x[b]);
                    let (wa, wb) = (sol.displacements[a][2], sol.displacements[b][2]);
                    wa + (wb - wa) * (x - xa) / (xb - xa)
                }
                (Some(&a), _) | (_, Some(&a)) => sol.displacements[a][2],
                _ => 0.0,
            };
            worst = worst.max((w - base).abs());
        }
        deflection.insert(*case, worst);
        per_case.insert(*case, (f, r));
    }
    let ends: Vec<usize> = frame
        .supports
        .iter()
        .map(|s| s.0)
        .filter(|&n| {
            n == 0 || n + 1 == nodes_x.len() || {
                let x = nodes_x[n];
                x <= sup[0] + 1e-6 || x >= sup[sup.len() - 1] - 1e-6
            }
        })
        .collect();
    let mut per_combo = Vec::new();
    for combo in combos {
        let mut m_sag: f64 = 0.0;
        let mut m_hog: f64 = 0.0;
        let mut v: f64 = 0.0;
        let mut r_end: f64 = 0.0;
        let mut r_in: f64 = 0.0;
        let nf = per_case
            .values()
            .next()
            .map_or(0, |(f, _): &(Vec<[f64; 6]>, _)| f.len());
        for i in 0..nf {
            let mut s = [0.0; 6];
            for (case, k) in &combo.factors {
                if let Some((f, _)) = per_case.get(case) {
                    for d in 0..6 {
                        s[d] += k * f[i][d];
                    }
                }
            }
            m_sag = m_sag.max(s[4]);
            m_hog = m_hog.max(-s[4]);
            v = v.max(s[2].abs());
        }
        for (si, (n, _)) in frame.supports.iter().enumerate() {
            let mut r = 0.0;
            for (case, k) in &combo.factors {
                if let Some((_, rs)) = per_case.get(case) {
                    r += k * rs[si].1;
                }
            }
            if ends.contains(n) {
                r_end = r_end.max(r);
            } else {
                r_in = r_in.max(r);
            }
        }
        per_combo.push([m_sag, m_hog, v, r_end, r_in]);
    }
    let span = sup.windows(2).map(|w| w[1] - w[0]).fold(0.0, f64::max);
    let overhang = sup[0].max(length - sup[sup.len() - 1]);
    Some(BeamResult {
        length,
        per_combo,
        deflection,
        span: span.max(2.0 * overhang),
        cantilever: overhang > 0.3,
    })
}

/// The worst check of a kind among the members of one element.
struct Worst {
    best: Option<Check>,
}

impl Worst {
    fn new() -> Self {
        Worst { best: None }
    }
    fn take(&mut self, c: Check) {
        if self.best.as_ref().is_none_or(|b| c.ratio > b.ratio) {
            self.best = Some(c);
        }
    }
}

/// The joists or rafters of one framed floor or roof plane.
#[allow(clippy::too_many_arguments)]
fn spans_of(
    model: &Model,
    td: &Takedown,
    plates: &[gravity::Plate],
    i: usize,
    framing: &Framing,
    members: &[Member],
    subject: Subject,
    combos: &[Combination],
    cases: &[Case],
    out: &mut Vec<Check>,
) {
    let pl = &plates[i];
    let supports = gravity::plate_supports(model, td, pl);
    if supports.is_empty() && !pl.base {
        out.push(Check {
            subject,
            what: format!(
                "{}: las viguetas no se apoyan en ningún muro ni viga",
                pl.name
            ),
            rule: "Apoyos de las viguetas",
            ratio: 9.99,
            status: Status::Fail,
            advice: Some(
                "un muro portante o una viga bajo ellas (un apoyo intermedio si la luz es grande)"
                    .into(),
            ),
        });
        return;
    }
    let rooms: Vec<_> = model.rooms_on_level(pl.level).collect();
    let altitude = model.site_altitude();
    let mat = Mat::of(framing, false);
    let sec = mat.section();
    let s = framing.spacing;
    let roof = pl.roof;
    let rafters = matches!(pl.source, PlateSource::Roof(..));
    let floor_name = if rafters { "Cabio" } else { "Vigueta" };
    let mut worst: Vec<Worst> = (0..8).map(|_| Worst::new()).collect();
    let cos = pl.pitch.cos().max(0.2);
    for m in members {
        if !matches!(m.role, Role::Joist | Role::Rafter) {
            continue;
        }
        let a = Point2::new(m.start[0], m.start[1]);
        let b = Point2::new(m.end[0], m.end[1]);
        let mid = a + (b - a) * 0.5;
        if !geo::inside(mid, &pl.outline) {
            continue;
        }
        let lp = a.distance(b);
        if lp < 0.3 {
            continue;
        }
        let dir = (b - a) * (1.0 / lp);
        // Supports along it (plan).
        let mut sup: Vec<f64> = Vec::new();
        for sp in &supports {
            if sp.path.len() < 2 {
                let (d, t) = geo::to_segment(sp.path[0], a, b);
                if d < 0.3 {
                    sup.push(t * lp);
                }
                continue;
            }
            if let Some((t, _)) = geo::ray_hit(a - dir * 0.01, dir, &sp.path, lp + 0.02) {
                sup.push((t - 0.01).clamp(0.0, lp));
            }
            for (e, s_end) in [(a, 0.0), (b, lp)] {
                if geo::to_path(e, &sp.path).0 < 0.3 {
                    sup.push(s_end);
                }
            }
        }
        if rafters {
            // The ridge end holds it (its pair and the tie, or a ridge
            // beam): the higher end.
            let up = if m.end[2] > m.start[2] { lp } else { 0.0 };
            sup.push(up);
        }
        sup.sort_by(f64::total_cmp);
        sup.dedup_by(|x, y| (*x - *y).abs() < 0.15);
        // Loads: its strip and what stands on it.
        let mut loads: Vec<(f64, Case, f64)> = Vec::new();
        let n = (lp / BIN).ceil().max(1.0) as usize;
        for k in 0..n {
            let x = (k as f64 + 0.5) * lp / n as f64;
            let p = a + dir * x;
            for (case, q) in area_loads(model, plates, i, &rooms, p) {
                // Per true length, square to the rafter: × cos.
                loads.push((x / cos, case, q * s * lp / n as f64 * cos));
            }
        }
        for (p, case, kn) in &td.plate_incoming[i] {
            let (d, t) = geo::to_segment(*p, a, b);
            // What stands right over a wall or beam goes into it, not into
            // the joist.
            let over = supports
                .iter()
                .any(|sp| sp.path.len() > 1 && geo::to_path(*p, &sp.path).0 < 0.12);
            if d <= s / 2.0 && !over {
                loads.push((t * lp / cos, *case, kn * cos));
            }
        }
        let sup_true: Vec<f64> = sup.iter().map(|x| x / cos).collect();
        let Some(r) = continuous_beam(lp / cos, &sup_true, &loads, sec, combos, cases) else {
            continue;
        };
        let level = model
            .level(pl.level)
            .map_or(String::new(), |l| l.name.clone());
        let label = format!(
            "{floor_name} {} a {} mm, {} m de luz ({})",
            framing.stud.name,
            dec(s * 1000.0, 0),
            dec(r.span, 2),
            level
        );
        // Thrust of the rafters held by a tie: H = w a / (2 tan α).
        let thrust = if rafters && pl.pitch > 0.05 {
            Some(|combo: &Combination| {
                let total: f64 = loads.iter().map(|(_, c, kn)| combo.factor(*c) * kn).sum();
                total / 2.0 / pl.pitch.tan()
            })
        } else {
            None
        };
        for (ci, combo) in combos.iter().enumerate() {
            if !matches!(combo.kind, Kind::Ultimate) {
                continue;
            }
            let [m_sag, m_hog, v, r_end, r_in] = r.per_combo[ci];
            let n_axial = thrust.map_or(0.0, |t| t(combo) / cos);
            match mat {
                Mat::Steel(c) => {
                    let mc = c.mc_rd();
                    // Hogging: the bottom flange in compression, held by
                    // the blocking (or every 1,2 m).
                    let lb = if framing.noggins {
                        r.span / 2.0
                    } else {
                        r.span.min(1.2)
                    };
                    let mb = c.mb_rd(lb.max(0.3));
                    let u_m = (m_sag / mc).max(m_hog / mb);
                    let (nb, _) = c.nb_rd(r.span, None);
                    let u = if n_axial > 0.0 {
                        (n_axial / nb).powf(0.8) + u_m.powf(0.8)
                    } else {
                        u_m
                    };
                    worst[0].take(Check {
                        subject,
                        what: format!(
                            "{label}: M {} kN·m en el vano (Mc,Rd {}), {} sobre el apoyo (Mb,Rd {}){}",
                            dec(m_sag, 2),
                            dec(mc, 2),
                            dec(m_hog, 2),
                            dec(mb, 2),
                            if n_axial > 0.0 {
                                format!(", N {} kN por el tirante", dec(n_axial, 1))
                            } else {
                                String::new()
                            }
                        ),
                        rule: "Flexión (EN 1993-1-3, 6.1.4 y 6.3)",
                        ratio: u,
                        status: status(u),
                        advice: (u > 1.0).then(|| {
                            format!("más canto o espesor, menos separación o un apoyo intermedio (luz de {} m)", dec(r.span, 2))
                        }),
                    });
                    let vb = c.vb_rd();
                    let u = v / vb;
                    worst[1].take(Check {
                        subject,
                        what: format!("{label}: V {} kN (Vb,Rd {})", dec(v, 1), dec(vb, 1)),
                        rule: "Cortante del alma (EN 1993-1-3, 6.1.5)",
                        ratio: u,
                        status: status(u),
                        advice: None,
                    });
                    // Web crippling at the supports over 40 mm of bearing.
                    let re = c.rw_rd(40.0, true);
                    let ri = c.rw_rd(40.0, false);
                    let u_end = r_end / re;
                    let u_in = (r_in / ri).max(if r_in > 0.0 {
                        (m_hog / mc + r_in / ri) / 1.25
                    } else {
                        0.0
                    });
                    let u = u_end.max(u_in);
                    worst[2].take(Check {
                        subject,
                        what: format!(
                            "{label}: reacción {} kN en el extremo (Rw,Rd {}){}",
                            dec(r_end, 1),
                            dec(re, 1),
                            if r_in > 0.0 {
                                format!(
                                    ", {} kN en el apoyo intermedio (Rw,Rd {})",
                                    dec(r_in, 1),
                                    dec(ri, 1)
                                )
                            } else {
                                String::new()
                            }
                        ),
                        rule: "Abolladura del alma en los apoyos (EN 1993-1-3, 6.1.7 y 6.1.11)",
                        ratio: u,
                        status: status(u),
                        advice: (u > 1.0)
                            .then(|| "un rigidizador o un clip de apoyo en el alma".to_string()),
                    });
                    let screws = (r_end.max(r_in) / cold::screw_bearing(4.2, c.t))
                        .ceil()
                        .max(2.0);
                    worst[3].take(Check {
                        subject,
                        what: format!(
                            "{label}: apoyo de {} kN, {} tornillos Ø4,2 en el angular de apoyo",
                            dec(r_end.max(r_in), 1),
                            screws
                        ),
                        rule: "Unión de apoyo (EN 1993-1-3, tabla 8.4)",
                        ratio: 0.0,
                        status: Status::Info,
                        advice: None,
                    });
                }
                Mat::Timber(p) => {
                    let kmod = timber::kmod_of(combo, altitude);
                    let (um, uv) = p.bending_shear(m_sag.max(m_hog), v, kmod);
                    let u = if n_axial > 0.0 {
                        p.compression_bending(n_axial, m_sag.max(m_hog), r.span, None, kmod)
                    } else {
                        um
                    };
                    worst[0].take(Check {
                        subject,
                        what: format!(
                            "{label}: M {} kN·m, kmod {}{}",
                            dec(m_sag.max(m_hog), 2),
                            dec(kmod, 1),
                            if n_axial > 0.0 {
                                format!(", N {} kN por el tirante", dec(n_axial, 1))
                            } else {
                                String::new()
                            }
                        ),
                        rule: "Flexión (EN 1995-1-1, 6.1.6 y 6.3.2)",
                        ratio: u,
                        status: status(u),
                        advice: (u > 1.0).then(|| {
                            format!(
                                "más canto, menos separación o un apoyo intermedio (luz de {} m)",
                                dec(r.span, 2)
                            )
                        }),
                    });
                    worst[1].take(Check {
                        subject,
                        what: format!("{label}: V {} kN", dec(v, 1)),
                        rule: "Cortante (EN 1995-1-1, 6.1.7)",
                        ratio: uv,
                        status: status(uv),
                        advice: None,
                    });
                    let u = p.bearing(r_end.max(r_in), 90.0, kmod);
                    worst[2].take(Check {
                        subject,
                        what: format!(
                            "{label}: reacción {} kN sobre 90 mm de apoyo",
                            dec(r_end.max(r_in), 1)
                        ),
                        rule: "Compresión perpendicular en el apoyo (EN 1995-1-1, 6.1.5)",
                        ratio: u,
                        status: status(u),
                        advice: None,
                    });
                }
            }
            if let Some(t) = thrust {
                let h = t(combo);
                let tie = Mat::of(framing, true);
                let u = match tie {
                    Mat::Steel(c) => h / (c.gross().a * cold::FYB / cold::GAMMA_M0 / 1000.0),
                    Mat::Timber(p) => p.tension(h, timber::kmod_of(combo, altitude)),
                };
                worst[4].take(Check {
                    subject,
                    what: format!(
                        "Tirante {} de la cercha: {} kN de tracción",
                        framing.track.name,
                        dec(h, 1)
                    ),
                    rule: "Tracción del tirante (EN 1993-1-3, 6.1.2 / EN 1995-1-1, 6.1.2)",
                    ratio: u,
                    status: status(u),
                    advice: None,
                });
            }
        }
        // Deflections (characteristic G + Q; Q alone; and long term in
        // timber).
        let l = r.span;
        let limit_total = if roof { l / 300.0 } else { l / 400.0 };
        let w_g = r.deflection.get(&Case::Dead).copied().unwrap_or(0.0);
        let w_q: f64 = cases
            .iter()
            .filter(|c| matches!(c, Case::Live(_) | Case::Snow))
            .map(|c| r.deflection.get(c).copied().unwrap_or(0.0))
            .fold(0.0, f64::max);
        let creep = match mat {
            Mat::Timber(_) => timber::K_DEF,
            Mat::Steel(_) => 0.0,
        };
        let psi2 = if roof { 0.0 } else { 0.3 };
        let total = w_g * (1.0 + creep) + w_q * (1.0 + psi2 * creep);
        let u = (total / limit_total).max(w_q / (l / 350.0));
        worst[5].take(Check {
            subject,
            what: format!(
                "{label}: flecha {} mm (permanente {}, variable {}{})",
                dec(total * 1000.0, 1),
                dec(w_g * 1000.0, 1),
                dec(w_q * 1000.0, 1),
                if creep > 0.0 { ", con fluencia" } else { "" }
            ),
            rule: if roof {
                "Flecha L/300 (DB-SE 4.3.3)"
            } else {
                "Flecha L/400 con tabiques, L/350 de confort (DB-SE 4.3.3)"
            },
            ratio: u,
            status: status(u),
            advice: None,
        });
        // Vibration of floors (EN 1995-1-1, 7.3): frequency over 8 Hz and
        // under 1,5 mm for 1 kN.
        if !roof && !r.cantilever {
            let ei_m = mat.ei() / s;
            let g: f64 = loads
                .iter()
                .filter(|(_, c, _)| *c == Case::Dead)
                .map(|(_, _, kn)| kn)
                .sum::<f64>()
                / (r.length * s);
            let q: f64 = loads
                .iter()
                .filter(|(_, c, _)| matches!(c, Case::Live(_)))
                .map(|(_, _, kn)| kn)
                .sum::<f64>()
                / (r.length * s);
            let mass = (g + 0.3 * q) * 1000.0 / 9.81;
            let f1 = std::f64::consts::PI / (2.0 * l * l) * (ei_m * 1000.0 / mass).sqrt();
            let w1 = 0.45 * 1.0 * l.powi(3) / (48.0 * mat.ei()) * 1000.0;
            let u = (8.0 / f1).max(w1 / 1.5);
            worst[6].take(Check {
                subject,
                what: format!(
                    "{label}: frecuencia propia {} Hz, {} mm con 1 kN",
                    dec(f1, 1),
                    dec(w1, 2)
                ),
                rule: "Vibraciones del forjado (EN 1995-1-1, 7.3; f1 ≥ 8 Hz, w ≤ 1,5 mm/kN)",
                ratio: u,
                status: if u <= 1.0 { Status::Pass } else { Status::Warn },
                advice: (u > 1.0)
                    .then(|| "más canto, menos separación o un tablero más rígido".into()),
            });
        }
    }
    for w in worst {
        if let Some(c) = w.best {
            out.push(c);
        }
    }
}

/// The studs, headers and jack studs of one framed wall.
#[allow(clippy::too_many_arguments)]
fn wall_studs(
    model: &Model,
    td: &Takedown,
    wind: Option<&BuildingWind>,
    id: WallId,
    framing: &Framing,
    combos: &[Combination],
    cases: &[Case],
    out: &mut Vec<Check>,
) {
    let Some(w) = model.wall(id) else { return };
    let Some(c) = td.carriers.get(&Carrier::Wall(id)) else {
        return;
    };
    let altitude = model.site_altitude();
    let members = wall_members(model, id);
    let pressure = wall_pressure(model, id, wind);
    let mat = Mat::of(framing, false);
    let sheathed = framing.sheathing.is_some();
    let name = model
        .wall_type(w.wall_type)
        .map_or(String::new(), |t| t.name.clone());
    let level = model
        .level(w.level)
        .map_or(String::new(), |l| l.name.clone());
    let wall_weight = weights::wall_type_weight(model, w.wall_type);
    let mut studs: Vec<&Member> = members
        .iter()
        .filter(|m| matches!(m.role, Role::Stud | Role::KingStud))
        .collect();
    studs.sort_by(|a, b| a.along[0].total_cmp(&b.along[0]));
    let mut worst_stud = Worst::new();
    let mut worst_bearing = Worst::new();
    for (k, st) in studs.iter().enumerate() {
        let x = st.along[0];
        let left = if k > 0 { studs[k - 1].along[0] } else { x };
        let right = if k + 1 < studs.len() {
            studs[k + 1].along[0]
        } else {
            x
        };
        let trib = ((right - left) / 2.0).clamp(0.05, framing.spacing * 1.5);
        let h = st.length();
        let n_case = |case: Case| -> f64 {
            let mut v = c.received_per_m(case, x, (trib / 2.0).max(BIN)) * trib;
            if case == Case::Dead {
                v += wall_weight * h / 2.0 * trib;
            }
            v
        };
        let lcr_z = if sheathed {
            None
        } else if framing.noggins {
            Some(h / 2.0)
        } else {
            Some(h)
        };
        let m_wind = pressure * trib * h * h / 8.0;
        for combo in combos.iter().filter(|c| c.kind == Kind::Ultimate) {
            let n: f64 = cases.iter().map(|cs| combo.factor(*cs) * n_case(*cs)).sum();
            let m = wind_factor(combo) * m_wind;
            if n <= 0.0 && m <= 0.0 {
                continue;
            }
            let (u, detail) = match mat {
                Mat::Steel(cs) => {
                    let (nb, _) = cs.nb_rd(h, lcr_z);
                    let mb = if sheathed {
                        cs.mc_rd()
                    } else {
                        cs.mb_rd(if framing.noggins { h / 2.0 } else { h })
                    };
                    let u = (n.max(0.0) / nb).powf(0.8) + (m / mb).powf(0.8);
                    (
                        if m > 0.0 { u } else { n / nb },
                        format!("Nb,Rd {} kN, Mb,Rd {} kN·m", dec(nb, 1), dec(mb, 2)),
                    )
                }
                Mat::Timber(p) => {
                    let kmod = timber::kmod_of(combo, altitude);
                    (
                        p.compression_bending(n.max(0.0), m, h, lcr_z, kmod),
                        format!("kmod {}", dec(kmod, 1)),
                    )
                }
            };
            worst_stud.take(Check {
                subject: Subject::Wall(id),
                what: format!(
                    "{name} ({level}): montante {} de {} m, N {} kN{}; {detail}",
                    framing.stud.name,
                    dec(h, 2),
                    dec(n, 1),
                    if m > 0.0 {
                        format!(", M de viento {} kN·m", dec(m, 2))
                    } else {
                        String::new()
                    }
                ),
                rule: match mat {
                    Mat::Steel(_) => "Montantes a compresión y flexión (EN 1993-1-3, 6.2 y 6.3)",
                    Mat::Timber(_) => "Montantes a compresión y flexión (EN 1995-1-1, 6.3.2)",
                },
                ratio: u,
                status: status(u),
                advice: None,
            });
            if let Mat::Timber(p) = mat {
                let kmod = timber::kmod_of(combo, altitude);
                let u = p.bearing(n.max(0.0), framing.track.depth * 1000.0, kmod);
                worst_bearing.take(Check {
                    subject: Subject::Wall(id),
                    what: format!(
                        "{name}: montante sobre el durmiente {}, N {} kN",
                        framing.track.name,
                        dec(n, 1)
                    ),
                    rule: "Compresión perpendicular del durmiente (EN 1995-1-1, 6.1.5)",
                    ratio: u,
                    status: status(u),
                    advice: (u > 1.0).then(|| {
                        "montantes dobles en ese tramo, menos separación o un durmiente de madera de más clase".into()
                    }),
                });
            }
        }
    }
    for wst in [worst_stud, worst_bearing] {
        if let Some(c) = wst.best {
            out.push(c);
        }
    }
    // Headers: two pieces in a box over each opening.
    let mut worst_header = Worst::new();
    let mut worst_jack = Worst::new();
    let mut seen: Vec<(f64, f64)> = Vec::new();
    for hd in members.iter().filter(|m| m.role == Role::Header) {
        let (a0, a1) = (hd.along[0].min(hd.along[1]), hd.along[0].max(hd.along[1]));
        if seen
            .iter()
            .any(|s| (s.0 - a0).abs() < 0.01 && (s.1 - a1).abs() < 0.01)
        {
            continue;
        }
        seen.push((a0, a1));
        let l = (a1 - a0).max(0.1);
        let top_z = hd.start[2].max(hd.end[2]);
        let above = (c.top - top_z).max(0.0);
        let mid = (a0 + a1) / 2.0;
        let w_case = |case: Case| -> f64 {
            let mut v = c.received_per_m(case, mid, l / 2.0);
            if case == Case::Dead {
                v += wall_weight * above + 2.0 * weights::member_weight(&framing.stud);
            }
            v
        };
        let hmat = Mat::of(
            &Framing {
                stud: hd.section.clone(),
                ..framing.clone()
            },
            false,
        );
        let ei2 = 2.0 * hmat.ei();
        for combo in combos.iter().filter(|c| c.kind == Kind::Ultimate) {
            let wd: f64 = cases.iter().map(|cs| combo.factor(*cs) * w_case(*cs)).sum();
            let m = wd * l * l / 8.0;
            let v = wd * l / 2.0;
            let u = match hmat {
                Mat::Steel(cs) => (m / (2.0 * cs.mc_rd())).max(v / (2.0 * cs.vb_rd())),
                Mat::Timber(p) => {
                    let kmod = timber::kmod_of(combo, altitude);
                    let (um, uv) = p.bending_shear(m / 2.0, v / 2.0, kmod);
                    um.max(uv)
                }
            };
            worst_header.take(Check {
                subject: Subject::Wall(id),
                what: format!(
                    "{name}: dintel de 2 × {} sobre {} m, {} kN/m, M {} kN·m",
                    hd.section.name,
                    dec(l, 2),
                    dec(wd, 1),
                    dec(m, 2)
                ),
                rule: "Dinteles a flexión y cortante",
                ratio: u,
                status: status(u),
                advice: None,
            });
            // The jack stud beside it takes half.
            let jh = (top_z - c.base).max(0.5);
            let u = match mat {
                Mat::Steel(cs) => v / cs.nb_rd(jh, None).0,
                Mat::Timber(p) => {
                    p.compression_bending(v, 0.0, jh, None, timber::kmod_of(combo, altitude))
                }
            };
            worst_jack.take(Check {
                subject: Subject::Wall(id),
                what: format!(
                    "{name}: montante de apoyo del dintel de {} m, N {} kN",
                    dec(l, 2),
                    dec(v, 1)
                ),
                rule: "Montantes de apoyo de los dinteles",
                ratio: u,
                status: status(u),
                advice: None,
            });
        }
        // Deflection of the header under the characteristic load.
        let wk: f64 = cases
            .iter()
            .filter(|c| !matches!(c, Case::Wind(_)))
            .map(|c| w_case(*c))
            .sum();
        let d = 5.0 * wk * l.powi(4) / (384.0 * ei2);
        let u = d / (l / 400.0);
        worst_header.take(Check {
            subject: Subject::Wall(id),
            what: format!(
                "{name}: flecha del dintel de {} m, {} mm",
                dec(l, 2),
                dec(d * 1000.0, 1)
            ),
            rule: "Flecha de los dinteles L/400",
            ratio: u,
            status: status(u),
            advice: None,
        });
    }
    for wst in [worst_header, worst_jack] {
        if let Some(c) = wst.best {
            out.push(c);
        }
    }
}

/// Edge spacing of the board's fasteners, m.
pub const EDGE_SPACING: f64 = 0.15;

/// The racking resistance of a framed wall (EN 1995-1-1, 9.2.4.2, method
/// A): Σ Ff,Rd · bi · ci / s over its full-height panels between openings,
/// kN; with the panels' widths.
fn racking(model: &Model, id: WallId, f: &Framing, kmod: f64) -> (f64, Vec<f64>, f64) {
    let Some(w) = model.wall(id) else {
        return (0.0, Vec::new(), 0.0);
    };
    let Some(board) = &f.sheathing else {
        return (0.0, Vec::new(), 0.0);
    };
    let len = geo::path_length(&geo::wall_path(w));
    let h = w.height;
    // The gaps of the openings along it.
    let mut gaps: Vec<(f64, f64)> = model
        .openings_in_wall(id)
        .filter_map(|o| {
            let t = model.opening_type(o.opening_type)?;
            Some(o.span(t.width))
        })
        .collect();
    gaps.sort_by(|a, b| a.0.total_cmp(&b.0));
    let mut panels = Vec::new();
    let mut at = 0.0;
    for (a, b) in gaps {
        if a > at {
            panels.push(a - at);
        }
        at = at.max(b);
    }
    if len > at {
        panels.push(len - at);
    }
    panels.retain(|b| *b >= 0.3);
    let t_board = board.thickness * 1000.0;
    let ff = match f.material {
        FrameMaterial::Steel => {
            // Screw Ø4,2 through the board into the stud: the steel's
            // bearing or the board's embedment.
            let steel = cold::screw_bearing(4.2, f.stud.thickness * 1000.0);
            let fh1 = 65.0 * 4.2f64.powf(-0.7) * t_board.powf(0.1);
            let board = kmod * fh1 * t_board * 4.2 / timber::GAMMA_M / 1000.0;
            steel.min(board)
        }
        FrameMaterial::Timber => timber::fastener_board(2.8, t_board, 50.0, kmod),
    };
    let b0 = h / 2.0;
    let total: f64 = panels
        .iter()
        .map(|b| ff * b * (b / b0).min(1.0) / EDGE_SPACING)
        .sum();
    (total, panels, ff)
}

/// The shear walls of each storey against the wind.
#[allow(clippy::too_many_arguments)]
fn shear_walls(model: &Model, td: &Takedown, wind: &BuildingWind, out: &mut Vec<Check>) {
    let kmod = timber::Duration::Short.kmod();
    let levels: Vec<(archisim_core::LevelId, f64)> =
        model.levels().iter().map(|l| (l.id, l.elevation)).collect();
    for (level, z) in levels {
        let walls: Vec<_> = model.walls().filter(|w| w.level == level).collect();
        if walls.is_empty() {
            continue;
        }
        for dir in [WindDir::PosX, WindDir::PosY] {
            let [dx, dy] = dir.vector();
            // The wind above the base of this storey, either way: the
            // larger.
            let v_k: f64 = [
                dir,
                if dir == WindDir::PosX {
                    WindDir::NegX
                } else {
                    WindDir::NegY
                },
            ]
            .iter()
            .map(|d| {
                wind.storeys
                    .iter()
                    .filter(|s| s.dir == *d && s.to > z + 0.2)
                    .map(|s| {
                        let part = ((s.to - z.max(s.from)) / (s.to - s.from)).clamp(0.0, 1.0);
                        s.force * part
                    })
                    .sum::<f64>()
            })
            .fold(0.0, f64::max);
            if v_k <= 0.0 {
                continue;
            }
            let v_d = 1.5 * v_k;
            // The walls along it and their share (by stiffness: framed by
            // their racking resistance, masonry and concrete far more).
            let mut parts: Vec<(WallId, f64, f64, Option<Framing>)> = Vec::new();
            for w in &walls {
                let d = w.end - w.start;
                let l = d.length();
                if l < 0.5 || (d.x * dx + d.y * dy).abs() / l < 0.94 {
                    continue;
                }
                let Some(t) = model.wall_type(w.wall_type) else {
                    continue;
                };
                match model.type_framing(w.wall_type.0) {
                    Some(f) => {
                        let (cap, _, _) = racking(model, w.id, f, kmod);
                        if cap > 0.0 {
                            parts.push((w.id, cap, cap, Some(f.clone())));
                        }
                    }
                    None => {
                        if let Some(k) = Model::frame_layer(&t.layers).filter(|&k| {
                            t.layers[k].function == archisim_core::LayerFunction::Structure
                        }) {
                            let th = t.layers[k].thickness;
                            parts.push((w.id, l * th * 2000.0, 0.0, None));
                        }
                    }
                }
            }
            let framed: Vec<_> = parts.iter().filter(|p| p.3.is_some()).collect();
            if framed.is_empty() {
                continue;
            }
            let stiff: f64 = parts.iter().map(|p| p.1).sum();
            for (id, k, cap, f) in &parts {
                let Some(f) = f else { continue };
                let share = v_d * k / stiff;
                let u = share / cap;
                let wall = model.wall(*id);
                let name = wall
                    .and_then(|w| model.wall_type(w.wall_type))
                    .map_or(String::new(), |t| t.name.clone());
                let (_, panels, ff) = racking(model, *id, f, kmod);
                let h = wall.map_or(2.7, |w| w.height);
                out.push(Check {
                    subject: Subject::Wall(*id),
                    what: format!(
                        "{name}: {} kN de viento {} (de {} kN en la planta), resiste {} kN con {} paneles, {} kN por fijación a {} mm",
                        dec(share, 1),
                        if dir == WindDir::PosX { "en X" } else { "en Y" },
                        dec(v_d, 1),
                        dec(*cap, 1),
                        panels.len(),
                        dec(ff, 2),
                        dec(EDGE_SPACING * 1000.0, 0)
                    ),
                    rule: "Arriostramiento por el tablero (EN 1995-1-1, 9.2.4.2, método A)",
                    ratio: u,
                    status: status(u),
                    advice: (u > 1.0).then(|| {
                        "fijaciones más juntas en los bordes, tablero por las dos caras o más muro en esa dirección".into()
                    }),
                });
                // Hold-downs: the tension at the end of the widest panel.
                if let (Some(b), Some(c)) = (
                    panels
                        .iter()
                        .copied()
                        .fold(None, |m: Option<f64>, b| Some(m.map_or(b, |m| m.max(b)))),
                    td.carriers.get(&Carrier::Wall(*id)),
                ) {
                    let total_b: f64 = panels.iter().sum();
                    let v_panel = share * b / total_b.max(b);
                    let g_line = c.delivered_total(Case::Dead) / c.length.max(0.1);
                    let t = v_panel * h / b - 0.8 * g_line * b / 2.0;
                    let shear_anchors = (share / total_b.max(0.3) / 5.0).ceil().max(1.0);
                    out.push(Check {
                        subject: Subject::Wall(*id),
                        what: if t > 0.0 {
                            format!(
                                "{name}: panel de {} m: tracción en su extremo {} kN; anclarlo al cimiento con un anclaje de tracción de al menos {} kN en cada extremo, y {} anclajes de cortante por metro",
                                dec(b, 2),
                                dec(t, 1),
                                dec(t.ceil(), 0),
                                dec(shear_anchors, 0)
                            )
                        } else {
                            format!(
                                "{name}: panel de {} m: su peso lo sujeta contra el vuelco; {} anclajes de cortante por metro",
                                dec(b, 2),
                                dec(shear_anchors, 0)
                            )
                        },
                        rule: "Anclajes de los paneles (vuelco y deslizamiento)",
                        ratio: 0.0,
                        status: if t > 0.0 { Status::Warn } else { Status::Info },
                        advice: None,
                    });
                }
            }
        }
    }
}

/// The fire resistance DB-SI 6 (table 3.1) asks of the structure, minutes,
/// for a building of `height` (evacuation height, m).
pub fn fire_requirement(use_: archisim_core::BuildingUse, height: f64) -> u32 {
    use archisim_core::BuildingUse as U;
    match use_ {
        U::Dwelling | U::PublicResidential | U::Administrative | U::Teaching => {
            if height < 15.0 {
                60
            } else if height < 28.0 {
                90
            } else {
                120
            }
        }
        U::Commercial | U::Assembly | U::Hospital => {
            if height < 15.0 {
                90
            } else if height < 28.0 {
                120
            } else {
                180
            }
        }
        U::Parking => 90,
    }
}

/// What the boards of a type give, orientatively: the gypsum boards on
/// its least protected face (mm) and the minutes they usually reach in
/// tested systems.
fn board_protection(model: &Model, layers: &[archisim_core::Layer], frame: usize) -> (f64, u32) {
    let gypsum = |l: &archisim_core::Layer| {
        model.material(l.material).is_some_and(|m| {
            let n = m.name.to_lowercase();
            n.contains("yeso")
                || n.contains("pladur")
                || n.contains("cartón")
                || n.contains("carton")
        })
    };
    let side = |r: std::ops::Range<usize>| -> f64 {
        layers[r]
            .iter()
            .filter(|l| gypsum(l))
            .map(|l| l.thickness * 1000.0)
            .sum()
    };
    let a = side(0..frame);
    let b = side(frame + 1..layers.len());
    let t = a.max(b).min(if a > 0.0 && b > 0.0 {
        a.min(b)
    } else {
        a.max(b)
    });
    let minutes = if t >= 36.0 {
        90
    } else if t >= 25.0 {
        60
    } else if t >= 12.5 {
        30
    } else {
        0
    };
    (t, minutes)
}

/// Every check of the light frames.
pub fn check(model: &Model, td: &Takedown, wind: Option<&BuildingWind>) -> Vec<Check> {
    let mut out = Vec::new();
    let mut cases = td.cases.clone();
    if wind.is_some() {
        for d in WindDir::ALL {
            if !cases.contains(&Case::Wind(d)) {
                cases.push(Case::Wind(d));
            }
        }
    }
    let combos = combinations(&cases, model.site_altitude());
    let plates = gravity::plates(model);
    for (i, pl) in plates.iter().enumerate() {
        let (type_id, members, subject) = match pl.source {
            PlateSource::Slab(id) => {
                let Some(s) = model.slab(id) else { continue };
                (s.slab_type.0, slab_members(model, id), Subject::Floor(id))
            }
            PlateSource::Roof(id, _) => {
                let Some(r) = model.roof(id) else { continue };
                (r.roof_type.0, roof_members(model, id), Subject::Roof(id))
            }
        };
        let Some(f) = model.type_framing(type_id) else {
            continue;
        };
        spans_of(
            model, td, &plates, i, f, &members, subject, &combos, &cases, &mut out,
        );
    }
    for w in model.walls() {
        if let Some(f) = model.type_framing(w.wall_type.0) {
            wall_studs(model, td, wind, w.id, f, &combos, &cases, &mut out);
        }
    }
    if let Some(wind) = wind {
        shear_walls(model, td, wind, &mut out);
    }
    // Fire: the framed types.
    let top = model
        .levels()
        .iter()
        .map(|l| l.elevation)
        .fold(0.0, f64::max);
    let ground = model
        .levels()
        .iter()
        .map(|l| l.elevation)
        .fold(f64::INFINITY, f64::min)
        .min(top);
    let need = fire_requirement(model.code_settings().building_use, top - ground);
    let mut seen = std::collections::BTreeSet::new();
    for w in model.walls() {
        if model.type_framing(w.wall_type.0).is_none() || !seen.insert(w.wall_type.0) {
            continue;
        }
        let Some(t) = model.wall_type(w.wall_type) else {
            continue;
        };
        let Some(k) = Model::frame_layer(&t.layers) else {
            continue;
        };
        let (mm, minutes) = board_protection(model, &t.layers, k);
        let passes = minutes >= need
            || (minutes >= 30
                && model.code_settings().building_use == archisim_core::BuildingUse::Dwelling);
        out.push(Check {
            subject: Subject::Wall(w.id),
            what: format!(
                "{}: {} mm de placa de yeso en la cara menos protegida, del orden de R {} en los sistemas ensayados; se exige R {}{}",
                t.name,
                dec(mm, 1),
                minutes,
                need,
                if model.code_settings().building_use == archisim_core::BuildingUse::Dwelling {
                    " (R 30 si es vivienda unifamiliar)"
                } else {
                    ""
                }
            ),
            rule: "Resistencia al fuego (DB-SI 6, tabla 3.1)",
            ratio: if passes { 0.0 } else { need as f64 / (minutes.max(1) as f64) },
            status: if passes { Status::Warn } else { Status::Fail },
            advice: Some(
                "justificarla con el ensayo o el certificado del sistema (UNE-EN 1365-1); la de los forjados, con su techo".into(),
            ),
        });
    }
    // One line per element and rule: the worst.
    let mut kept: Vec<Check> = Vec::new();
    for c in out {
        match kept
            .iter_mut()
            .find(|k| k.subject == c.subject && k.rule == c.rule)
        {
            Some(k) => {
                if c.ratio > k.ratio || (c.status > k.status && c.status != Status::Info) {
                    *k = c;
                }
            }
            None => kept.push(c),
        }
    }
    kept
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_continuous_joist_gives_its_reactions() {
        // Two spans of 3 m, 1 kN/m: the middle reaction is 5wL/4 = 3,75.
        let sec = Section {
            a: 1e-3,
            iy: 1e-6,
            iz: 2e-7,
            j: 5e-8,
            e: 210e6,
            g: 81e6,
        };
        let loads: Vec<(f64, Case, f64)> = (0..60)
            .map(|k| ((k as f64 + 0.5) * 0.1, Case::Dead, 0.1))
            .collect();
        let combos = combinations(&[Case::Dead], 0.0);
        let r =
            continuous_beam(6.0, &[0.0, 3.0, 6.0], &loads, sec, &combos, &[Case::Dead]).unwrap();
        let ult = &r.per_combo[0];
        // Factored by 1,35.
        assert!((ult[4] - 1.35 * 3.75).abs() < 0.05, "{ult:?}");
        assert!((ult[1] - 1.35 * 9.0 / 8.0).abs() < 0.05, "{ult:?}");
        assert!((r.span - 3.0).abs() < 1e-9);
    }

    #[test]
    fn fire_by_use_and_height() {
        use archisim_core::BuildingUse;
        assert_eq!(fire_requirement(BuildingUse::Dwelling, 6.0), 60);
        assert_eq!(fire_requirement(BuildingUse::Commercial, 20.0), 120);
    }
}
