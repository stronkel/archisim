//! The analytical model of the frame of columns and beams: its nodes,
//! bars, supports and loads, from the drawing and the takedown.
//!
//! - Each column is a bar from its base to its top; one standing on
//!   another, or on a beam, shares their node.
//! - Each beam runs along the middle of its depth; it is cut where other
//!   beams land on it, where columns stand on it or hold it from below,
//!   and the pieces share those nodes.
//! - Each piece of beam is cut in four and each column in two, so that
//!   the movements of the nodes give the deflections.
//! - Concrete joints are rigid; steel and timber beams are pinned at
//!   their ends (simple connections), continuous across the cuts.
//! - The base of a column that stands on nothing in the frame is fixed
//!   (its footing); a beam end on a wall is held there (pinned).
//! - The loads of the floors, roofs and walls that reach the beams and
//!   columns come from the takedown; the frame passes them on itself.
//! - The wind of each storey goes, in equal parts, to the frame's nodes
//!   at the top of the storey band.

use std::collections::BTreeMap;

use archisim_core::{BeamId, ColumnId, Model, Point2};

use crate::cases::{Case, WindDir};
use crate::frame::{Bar, Fixity, Frame, Load, FIXED, PINNED, V3};
use crate::geo;
use crate::gravity::{Carrier, Takedown, BIN};
use crate::sections::{section_of, Kind};

/// A section with the project's concrete.
fn section_in(model: &Model, p: archisim_core::Profile, kind: Kind) -> crate::frame::Section {
    let mut s = section_of(p, kind);
    if kind == Kind::Concrete {
        s.e = crate::concrete::Rc::of(model).ecm();
        s.g = s.e / 2.4;
    }
    s
}
use crate::wind::BuildingWind;

/// Which element a bar is a piece of.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Owner {
    Column(ColumnId),
    Beam(BeamId),
}

/// One bar of the analytical model and what it stands for.
#[derive(Debug, Clone, PartialEq)]
pub struct Piece {
    pub owner: Owner,
    /// Where it runs along its element, metres from its start.
    pub from: f64,
    pub to: f64,
    /// The span it belongs to: between two cuts of a beam (0, 1…), for
    /// the deflection.
    pub span: usize,
}

/// A span of a beam between two cuts, for its deflection.
#[derive(Debug, Clone, PartialEq)]
pub struct Span {
    pub owner: Owner,
    pub length: f64,
    /// The nodes along it, in order (ends included).
    pub nodes: Vec<usize>,
    /// Whether an end is free (a cantilever).
    pub cantilever: bool,
}

/// The analytical model.
#[derive(Debug, Clone, Default)]
pub struct Analytical {
    pub frame: Frame,
    pub pieces: Vec<Piece>,
    pub spans: Vec<Span>,
    pub loads: BTreeMap<Case, Vec<Load>>,
    /// The material of each element.
    pub kinds: BTreeMap<Owner, Kind>,
    /// The wind directions the frame takes (the others go to the walls).
    pub wind_on_frame: Vec<WindDir>,
    /// The modal spectral analysis, when the frame takes the earthquake.
    pub modal: Option<crate::seismic::Modal>,
    pub notes: Vec<String>,
}

fn level_z(model: &Model, l: archisim_core::LevelId) -> f64 {
    model.level(l).map_or(0.0, |l| l.elevation)
}

fn kind_of(model: &Model, t: archisim_core::MemberTypeId) -> Kind {
    model
        .member_type(t)
        .map_or(Kind::Other, |mt| Kind::of(model.material(mt.material)))
}

struct Nodes {
    at: Vec<V3>,
}

impl Nodes {
    fn add(&mut self, p: V3) -> usize {
        self.at.push(p);
        self.at.len() - 1
    }
}

/// Builds the analytical model of the frame.
pub fn build(model: &Model, td: &Takedown, wind: Option<&BuildingWind>) -> Analytical {
    let mut out = Analytical::default();
    let mut nodes = Nodes { at: Vec::new() };
    // Columns: base and top nodes.
    struct Col {
        id: ColumnId,
        pos: Point2,
        base: f64,
        top: f64,
        half: f64,
        base_node: usize,
        top_node: usize,
        kind: Kind,
    }
    let mut cols: Vec<Col> = Vec::new();
    for c in model.columns() {
        let base = level_z(model, c.level) + c.base_offset;
        let top = base + c.height;
        let half = model
            .member_type(c.member_type)
            .map_or(0.15, |t| t.profile.size().0.max(t.profile.size().1) / 2.0);
        let b = nodes.add([c.position.x, c.position.y, base]);
        let t = nodes.add([c.position.x, c.position.y, top]);
        let kind = kind_of(model, c.member_type);
        out.kinds.insert(Owner::Column(c.id), kind);
        cols.push(Col {
            id: c.id,
            pos: c.position,
            base,
            top,
            half,
            base_node: b,
            top_node: t,
            kind,
        });
    }
    // A column on another: share the node (the lower one's top).
    for i in 0..cols.len() {
        let below = (0..cols.len()).find(|&j| {
            j != i
                && cols[j].pos.distance(cols[i].pos) < 0.15 + cols[j].half
                && (cols[j].top - cols[i].base).abs() < 0.35
        });
        if let Some(j) = below {
            cols[i].base_node = cols[j].top_node;
        }
    }
    // Beams: cuts.
    struct Bm {
        id: BeamId,
        a: Point2,
        b: Point2,
        top: f64,
        axis: f64,
        kind: Kind,
        /// Cuts: along, node.
        cuts: Vec<(f64, usize)>,
    }
    let mut beams: Vec<Bm> = Vec::new();
    for b in model.beams() {
        let top = level_z(model, b.level) + b.top_offset;
        let depth = model
            .member_type(b.member_type)
            .map_or(0.4, |t| t.profile.size().1);
        let kind = kind_of(model, b.member_type);
        out.kinds.insert(Owner::Beam(b.id), kind);
        beams.push(Bm {
            id: b.id,
            a: b.start,
            b: b.end,
            top,
            axis: top - depth / 2.0,
            kind,
            cuts: Vec::new(),
        });
    }
    let point_on = |b: &Bm, s: f64| -> Point2 {
        let l = b.a.distance(b.b);
        if l <= 0.0 {
            b.a
        } else {
            b.a + (b.b - b.a) * (s / l)
        }
    };
    // Columns under or over a beam.
    for bi in 0..beams.len() {
        let len = beams[bi].a.distance(beams[bi].b);
        for c in cols.iter_mut() {
            let (d, t) = geo::to_segment(c.pos, beams[bi].a, beams[bi].b);
            if d > c.half + 0.1 {
                continue;
            }
            let s = t * len;
            let holds = c.top >= beams[bi].axis - 0.6 && c.top <= beams[bi].top + 0.35;
            let stands = (c.base - beams[bi].top).abs() < 0.35;
            if holds {
                beams[bi].cuts.push((s, c.top_node));
            } else if stands {
                // The column stands on the beam: a node on the beam.
                let p = point_on(&beams[bi], s);
                let n = nodes.add([p.x, p.y, beams[bi].axis]);
                beams[bi].cuts.push((s, n));
                c.base_node = n;
            }
        }
    }
    // Beams' ends that land on other beams.
    for bi in 0..beams.len() {
        for end in 0..2 {
            let e = if end == 0 { beams[bi].a } else { beams[bi].b };
            let len_bi = beams[bi].a.distance(beams[bi].b);
            let s_end = if end == 0 { 0.0 } else { len_bi };
            if beams[bi].cuts.iter().any(|(s, _)| (s - s_end).abs() < 0.3) {
                continue;
            }
            for oj in 0..beams.len() {
                if oj == bi || (beams[oj].top - beams[bi].top).abs() > 0.4 {
                    continue;
                }
                let (d, t) = geo::to_segment(e, beams[oj].a, beams[oj].b);
                if d > 0.3 {
                    continue;
                }
                let lo = beams[oj].a.distance(beams[oj].b);
                let s = t * lo;
                let existing = beams[oj]
                    .cuts
                    .iter()
                    .find(|(x, _)| (x - s).abs() < 0.3)
                    .map(|c| c.1);
                let n = existing.unwrap_or_else(|| {
                    let p = point_on(&beams[oj], s);
                    nodes.add([p.x, p.y, beams[oj].axis])
                });
                if existing.is_none() {
                    beams[oj].cuts.push((s, n));
                }
                beams[bi].cuts.push((s_end, n));
                break;
            }
        }
    }
    // Free beam ends: a node of their own (held by a wall or free).
    let mut supports: Vec<(usize, Fixity)> = Vec::new();
    let walls: Vec<(Vec<Point2>, f64)> = model
        .walls()
        .map(|w| {
            let base = level_z(model, w.level) + w.base_offset;
            (geo::wall_path(w), base + w.height)
        })
        .collect();
    // Walls under a beam hold it: where they cross it, and along it
    // where they run under it.
    for b in beams.iter_mut() {
        let len = b.a.distance(b.b);
        if len < 1e-6 {
            continue;
        }
        let dir = (b.b - b.a) * (1.0 / len);
        let mut held: Vec<f64> = Vec::new();
        for (path, wtop) in &walls {
            if !(*wtop >= b.axis - 0.8 && *wtop <= b.top + 0.1) {
                continue;
            }
            for seg in path.windows(2) {
                let (p0, p1) = (seg[0], seg[1]);
                let e = p1 - p0;
                let el = e.length();
                if el < 1e-6 {
                    continue;
                }
                let cross = dir.x * e.y - dir.y * e.x;
                if (cross / el).abs() > 0.34 {
                    // Across: where they meet.
                    let ap = p0 - b.a;
                    let t = (ap.x * e.y - ap.y * e.x) / cross;
                    let u = (ap.x * dir.y - ap.y * dir.x) / cross;
                    if (-0.2..=len + 0.2).contains(&t) && (-0.02..=1.02).contains(&u) {
                        held.push(t.clamp(0.0, len));
                    }
                } else {
                    // Along: under it for a stretch.
                    let d0 = geo::to_segment(p0, b.a, b.b);
                    let d1 = geo::to_segment(p1, b.a, b.b);
                    if d0.0 < 0.35 && d1.0 < 0.35 {
                        let (s0, s1) = (d0.1 * len, d1.1 * len);
                        let (lo, hi) = (s0.min(s1), s0.max(s1));
                        if hi - lo > 0.3 {
                            held.extend([lo, (lo + hi) / 2.0, hi]);
                        }
                    }
                }
            }
        }
        held.sort_by(f64::total_cmp);
        held.dedup_by(|x, y| (*x - *y).abs() < 0.3);
        for s in held {
            if b.cuts.iter().any(|(x, _)| (x - s).abs() < 0.3) {
                // Already a node there (a column, another beam).
                continue;
            }
            let p = b.a + dir * s;
            let n = nodes.add([p.x, p.y, b.axis]);
            b.cuts.push((s, n));
            supports.push((n, PINNED));
        }
    }
    for b in beams.iter_mut() {
        let len = b.a.distance(b.b);
        for (s_end, p) in [(0.0, b.a), (len, b.b)] {
            if b.cuts.iter().any(|(s, _)| (s - s_end).abs() < 0.3) {
                continue;
            }
            let n = nodes.add([p.x, p.y, b.axis]);
            b.cuts.push((s_end, n));
            let on_wall = walls.iter().any(|(path, wtop)| {
                geo::to_path(p, path).0 < 0.35 && *wtop >= b.axis - 0.8 && *wtop <= b.top + 0.1
            });
            if on_wall {
                supports.push((n, PINNED));
            }
        }
        b.cuts.sort_by(|x, y| x.0.total_cmp(&y.0));
        b.cuts.dedup_by(|x, y| (x.0 - y.0).abs() < 0.05);
    }
    // Columns' bases on nothing: fixed.
    for c in &cols {
        let shared = cols.iter().any(|o| o.top_node == c.base_node)
            || beams
                .iter()
                .any(|b| b.cuts.iter().any(|(_, n)| *n == c.base_node));
        if !shared {
            supports.push((c.base_node, FIXED));
        }
    }
    // Bars.
    let mut bars: Vec<Bar> = Vec::new();
    let mut pieces: Vec<Piece> = Vec::new();
    let mut spans: Vec<Span> = Vec::new();
    for c in &cols {
        let Some(col) = model.column(c.id) else {
            continue;
        };
        let Some(mt) = model.member_type(col.member_type) else {
            continue;
        };
        let sec = section_in(model, mt.profile, c.kind);
        // The section's depth (its y) turned by the column's rotation.
        let up = [-col.rotation.sin(), col.rotation.cos(), 0.0];
        let mid = nodes.add([c.pos.x, c.pos.y, (c.base + c.top) / 2.0]);
        let h = c.top - c.base;
        for (k, (i, j)) in [(c.base_node, mid), (mid, c.top_node)]
            .into_iter()
            .enumerate()
        {
            bars.push(Bar {
                i,
                j,
                section: sec,
                up,
                hinge_i: false,
                hinge_j: false,
            });
            pieces.push(Piece {
                owner: Owner::Column(c.id),
                from: h * k as f64 / 2.0,
                to: h * (k + 1) as f64 / 2.0,
                span: 0,
            });
        }
    }
    for b in &beams {
        let Some(beam) = model.beam(b.id) else {
            continue;
        };
        let Some(mt) = model.member_type(beam.member_type) else {
            continue;
        };
        let sec = section_in(model, mt.profile, b.kind);
        let pinned = matches!(b.kind, Kind::Steel | Kind::Timber | Kind::Aluminium);
        let last = b.cuts.len().saturating_sub(1);
        let supported = |n: usize| {
            supports.iter().any(|(s, _)| *s == n)
                || cols.iter().any(|c| c.top_node == n || c.base_node == n)
                || beams
                    .iter()
                    .any(|o| o.id != b.id && o.cuts.iter().any(|(_, m)| *m == n))
        };
        for (si, w) in b.cuts.windows(2).enumerate() {
            let (s0, n0) = w[0];
            let (s1, n1) = w[1];
            if s1 - s0 < 1e-3 {
                continue;
            }
            let mut span_nodes = vec![n0];
            let parts = 4;
            let mut prev = n0;
            for k in 1..=parts {
                let next = if k == parts {
                    n1
                } else {
                    let s = s0 + (s1 - s0) * k as f64 / parts as f64;
                    let p = point_on(b, s);
                    nodes.add([p.x, p.y, b.axis])
                };
                bars.push(Bar {
                    i: prev,
                    j: next,
                    section: sec,
                    up: [0.0, 0.0, 1.0],
                    hinge_i: pinned && k == 1 && si == 0 && supported(n0),
                    hinge_j: pinned && k == parts && si + 1 == last && supported(n1),
                });
                pieces.push(Piece {
                    owner: Owner::Beam(b.id),
                    from: s0 + (s1 - s0) * (k - 1) as f64 / parts as f64,
                    to: s0 + (s1 - s0) * k as f64 / parts as f64,
                    span: si,
                });
                span_nodes.push(next);
                prev = next;
            }
            let cantilever = (si == 0 && !supported(n0)) || (si + 1 == last && !supported(n1));
            spans.push(Span {
                owner: Owner::Beam(b.id),
                length: s1 - s0,
                nodes: span_nodes,
                cantilever,
            });
        }
    }
    out.frame = Frame {
        nodes: nodes.at,
        bars,
        supports,
        springs: Vec::new(),
    };
    // Loads.
    let mut loads: BTreeMap<Case, Vec<Load>> = BTreeMap::new();
    for (bar_i, piece) in pieces.iter().enumerate() {
        match piece.owner {
            Owner::Beam(id) => {
                let Some(c) = td.carriers.get(&Carrier::Beam(id)) else {
                    continue;
                };
                let mut add = |case: Case, bins: &[f64]| {
                    for (k, kn) in bins.iter().enumerate() {
                        if *kn == 0.0 {
                            continue;
                        }
                        let (b0, b1) = (k as f64 * BIN, ((k + 1) as f64 * BIN).min(c.length));
                        let (a, z) = (b0.max(piece.from), b1.min(piece.to));
                        if z <= a || b1 <= b0 {
                            continue;
                        }
                        let w = kn / (b1 - b0);
                        loads.entry(case).or_default().push(Load::Spread {
                            bar: bar_i,
                            from: a - piece.from,
                            to: z - piece.from,
                            w: [0.0, 0.0, -w],
                        });
                    }
                };
                add(Case::Dead, &c.own);
                for case in td.cases.iter().copied() {
                    let v = c.received_outside_frame(case);
                    add(case, &v);
                }
            }
            Owner::Column(id) => {
                let Some(c) = td.carriers.get(&Carrier::Column(id)) else {
                    continue;
                };
                let h = piece.to - piece.from;
                let col_h = c.top - c.base;
                if col_h > 0.0 {
                    let w = c.own[0] / col_h;
                    loads.entry(Case::Dead).or_default().push(Load::Spread {
                        bar: bar_i,
                        from: 0.0,
                        to: h,
                        w: [0.0, 0.0, -w],
                    });
                }
                // What reaches its top from outside the frame, on the
                // top piece's far node.
                if piece.to >= col_h - 1e-6 {
                    let node = out.frame.bars[bar_i].j;
                    for case in td.cases.iter().copied() {
                        let v: f64 = c.received_outside_frame(case).iter().sum();
                        if v.abs() > 1e-9 {
                            loads.entry(case).or_default().push(Load::Node {
                                node,
                                f: [0.0, 0.0, -v, 0.0, 0.0, 0.0],
                            });
                        }
                    }
                }
            }
        }
    }
    // The wind on the frame: where the building has load-bearing walls
    // along the wind, they take it (they are far stiffer); otherwise the
    // frame does.
    if let Some(w) = wind {
        if !out.frame.bars.is_empty() {
            for dir in WindDir::ALL {
                let [dx, dy] = dir.vector();
                let width = (w.hi.x - w.lo.x).max(w.hi.y - w.lo.y);
                if walls_along(model, dir) > 0.2 * width {
                    continue;
                }
                for st in w.storeys.iter().filter(|st| st.dir == dir) {
                    let free: Vec<usize> = (0..out.frame.nodes.len())
                        .filter(|&n| {
                            let z = out.frame.nodes[n][2];
                            z > st.from + 0.3
                                && z <= st.to + 0.6
                                && !out.frame.supports.iter().any(|(s, _)| *s == n)
                        })
                        .collect();
                    let top = free
                        .iter()
                        .map(|&n| out.frame.nodes[n][2])
                        .fold(f64::NEG_INFINITY, f64::max);
                    let at: Vec<usize> = free
                        .into_iter()
                        .filter(|&n| (out.frame.nodes[n][2] - top).abs() < 0.3)
                        .collect();
                    if at.is_empty() {
                        continue;
                    }
                    let each = st.force / at.len() as f64;
                    for n in at {
                        loads.entry(Case::Wind(dir)).or_default().push(Load::Node {
                            node: n,
                            f: [dx * each, dy * each, 0.0, 0.0, 0.0, 0.0],
                        });
                    }
                }
                if !loads.contains_key(&Case::Wind(dir)) {
                    continue;
                }
                out.wind_on_frame.push(dir);
            }
        }
    }
    // The earthquake (NCSE-02, 3.7): the force of each storey to the
    // frame's nodes at its floor, in X and in Y, where no load-bearing
    // walls take it.
    let quake = crate::seismic::quake(model, td);
    if quake.applies && !out.frame.bars.is_empty() {
        let takes = |dir: WindDir| walls_along(model, dir) <= 0.2 * building_width(model);
        if takes(WindDir::PosX) || takes(WindDir::PosY) {
            out.modal = crate::seismic::modal(model, &out.frame, &quake);
        }
        for (x, dir) in [(true, WindDir::PosX), (false, WindDir::PosY)] {
            if !takes(dir) {
                continue;
            }
            let [dx, dy] = dir.vector();
            for (si, &(z, _, simple)) in quake.storeys.iter().enumerate() {
                let force = out
                    .modal
                    .as_ref()
                    .map_or(simple, |m| m.forces[if x { 0 } else { 1 }][si]);
                let at: Vec<usize> = (0..out.frame.nodes.len())
                    .filter(|&n| {
                        (out.frame.nodes[n][2] - z).abs() < 0.6
                            && !out.frame.supports.iter().any(|(s, _)| *s == n)
                    })
                    .collect();
                if at.is_empty() || force <= 0.0 {
                    continue;
                }
                let each = force / at.len() as f64;
                for n in at {
                    loads.entry(Case::Quake(x)).or_default().push(Load::Node {
                        node: n,
                        f: [dx * each, dy * each, 0.0, 0.0, 0.0, 0.0],
                    });
                }
            }
        }
    }
    out.loads = loads;
    out.pieces = pieces;
    out.spans = spans;
    if !out.frame.bars.is_empty() && out.frame.supports.is_empty() {
        out.notes
            .push("El pórtico no tiene apoyos: no se puede calcular.".into());
    }
    out
}

/// Length of load-bearing walls along a direction, m.
fn walls_along(model: &Model, dir: WindDir) -> f64 {
    let [dx, dy] = dir.vector();
    model
        .walls()
        .filter(|wl| {
            model.wall_type(wl.wall_type).is_some_and(|t| {
                t.layers
                    .iter()
                    .any(|l| l.function == archisim_core::LayerFunction::Structure)
            })
        })
        .filter(|wl| {
            let d = wl.end - wl.start;
            let l = d.length();
            l > 0.5 && (d.x * dx + d.y * dy).abs() / l > 0.94
        })
        .map(|wl| wl.start.distance(wl.end))
        .sum()
}

/// The building's larger side in plan, m (the frame's when there is no
/// building wind).
fn building_width(model: &Model) -> f64 {
    crate::wind::building_wind(model).map_or(10.0, |w| (w.hi.x - w.lo.x).max(w.hi.y - w.lo.y))
}
