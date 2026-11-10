//! Walls drawn layer by layer (M4, `docs/MODULAR.md`): some models, often
//! from Revit, draw each layer of a wall build-up (cladding, membrane,
//! insulation, lining) as a wall of its own, pressed face to face against
//! the next. ArchiSim wants one wall with a layered type, so its framing,
//! its checks and its measurements work. Here such stacks are found among
//! the walls read from the file and joined: walls that are parallel, touch
//! face to face, overlap along their length and in height, and at least one
//! of them is thin (a layer, not a wall).
//!
//! Along the stack, the joined wall is split where the set of layers
//! changes (a lining that stops at a corner, a cladding that runs on);
//! each piece takes the lowest base and the highest top of its layers.

use std::collections::{BTreeMap, HashMap};

use archisim_core::{Layer, Point2};

use super::{piece_guid, snap, snap2, WallCandidate};

/// A wall this thin (or thinner) is a layer drawn as a wall.
pub(super) const THIN_LAYER: f64 = 0.030;
/// Faces closer than this (or overlapping by less) touch.
const TOUCH: f64 = 0.003;
/// Walls this far from parallel (sine of the angle) are not a stack.
const PARALLEL: f64 = 0.01;
/// Pieces of a joined wall shorter than this are left out.
const SHORTEST: f64 = 0.05;
/// Faces closer than this across the stack are the same place.
const SAME_PLACE: f64 = 0.002;
/// Ends closer than this along the stack are the same end.
const SAME_END: f64 = 0.005;

/// What joining did: how many walls of the file went into how many walls.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub(super) struct Joined {
    pub layers: usize,
    pub walls: usize,
    pub dropped: usize,
}

/// A candidate seen from the frame of its stack.
struct Placed {
    idx: usize,
    t0: f64,
    t1: f64,
    /// Its layers across the frame's line (left positive).
    layers: Vec<(f64, f64, Layer)>,
    s0: f64,
    s1: f64,
}

/// The candidates with their stacks joined, and what was done.
pub(super) fn join_stacks(cands: Vec<WallCandidate>) -> (Vec<WallCandidate>, Joined) {
    let n = cands.len();
    // Straight walls with their direction and their band across.
    let info: Vec<Option<(Point2, f64, f64, f64)>> = cands
        .iter()
        .map(|c| {
            let len = c.a.distance(c.b);
            if c.bulge != 0.0 || len < 1e-6 || c.layers.is_empty() {
                return None;
            }
            let u = (c.b - c.a) * (1.0 / len);
            let lo = c.layers.iter().map(|l| l.0).fold(f64::INFINITY, f64::min);
            let hi = c
                .layers
                .iter()
                .map(|l| l.1)
                .fold(f64::NEG_INFINITY, f64::max);
            Some((u, len, lo, hi))
        })
        .collect();

    // Neighbours by a grid of their boxes in plan.
    const CELL: f64 = 2.0;
    let mut grid: HashMap<(i64, i64), Vec<usize>> = HashMap::new();
    for (i, c) in cands.iter().enumerate() {
        let Some((_, _, lo, hi)) = info[i] else {
            continue;
        };
        let pad = lo.abs().max(hi.abs()) + TOUCH;
        let (x0, x1) = (c.a.x.min(c.b.x) - pad, c.a.x.max(c.b.x) + pad);
        let (y0, y1) = (c.a.y.min(c.b.y) - pad, c.a.y.max(c.b.y) + pad);
        let cell = |v: f64| (v / CELL).floor() as i64;
        for gx in cell(x0)..=cell(x1) {
            for gy in cell(y0)..=cell(y1) {
                grid.entry((gx, gy)).or_default().push(i);
            }
        }
    }

    let mut parent: Vec<usize> = (0..n).collect();
    fn root(p: &mut [usize], mut i: usize) -> usize {
        while p[i] != i {
            p[i] = p[p[i]];
            i = p[i];
        }
        i
    }
    let mut seen = std::collections::HashSet::new();
    for list in grid.values() {
        for (k, &i) in list.iter().enumerate() {
            for &j in &list[k + 1..] {
                let pair = (i.min(j), i.max(j));
                if !seen.insert(pair) {
                    continue;
                }
                if touching(&cands, &info, i, j) {
                    let (ri, rj) = (root(&mut parent, i), root(&mut parent, j));
                    if ri != rj {
                        parent[ri] = rj;
                    }
                }
            }
        }
    }
    let mut groups: BTreeMap<usize, Vec<usize>> = BTreeMap::new();
    for i in 0..n {
        let r = root(&mut parent, i);
        groups.entry(r).or_default().push(i);
    }

    let mut joined = Joined::default();
    let mut out: Vec<Option<WallCandidate>> = cands.into_iter().map(Some).collect();
    let mut made: Vec<WallCandidate> = Vec::new();
    let mut guid_uses: HashMap<String, usize> = HashMap::new();
    for members in groups.values().filter(|m| m.len() > 1) {
        let thin = members
            .iter()
            .any(|&i| info[i].is_some_and(|(_, _, lo, hi)| hi - lo <= THIN_LAYER + 1e-9));
        if !thin {
            continue;
        }
        let taken: Vec<&WallCandidate> = members
            .iter()
            .map(|&i| out[i].as_ref().expect("each candidate in one group"))
            .collect();
        let Some((walls, dropped)) = join_group(&taken, &mut guid_uses) else {
            continue;
        };
        joined.layers += members.len();
        joined.walls += walls.len();
        joined.dropped += dropped;
        for &i in members {
            out[i] = None;
        }
        made.extend(walls);
    }
    let mut result: Vec<WallCandidate> = out.into_iter().flatten().collect();
    result.extend(made);
    result.sort_by_key(|c| c.ifc);
    (result, joined)
}

/// Whether walls i and j are layers of one stack.
fn touching(
    cands: &[WallCandidate],
    info: &[Option<(Point2, f64, f64, f64)>],
    i: usize,
    j: usize,
) -> bool {
    let (Some((ui, li, lo_i, hi_i)), Some((uj, _, lo_j, hi_j))) = (info[i], info[j]) else {
        return false;
    };
    if ui.cross(uj).abs() > PARALLEL {
        return false;
    }
    let (ci, cj) = (&cands[i], &cands[j]);
    // In height.
    if ci.top.min(cj.top) - ci.base.max(cj.base) < 0.01 {
        return false;
    }
    let ni = ui.perp_left();
    let d = (cj.a - ci.a).dot(ni);
    let (s0, s1) = if ui.dot(uj) > 0.0 {
        (d + lo_j, d + hi_j)
    } else {
        (d - hi_j, d - lo_j)
    };
    // Face to face, not one inside the other.
    let gap = (s0 - hi_i).max(lo_i - s1);
    if !(-TOUCH..=TOUCH).contains(&gap) {
        return false;
    }
    // Along.
    let ta = (cj.a - ci.a).dot(ui);
    let tb = (cj.b - ci.a).dot(ui);
    ta.max(tb).min(li) - ta.min(tb).max(0.0) > 0.01
}

/// The walls one stack becomes, and how many short pieces were left out;
/// `None` if its layers do not make a clean build-up.
fn join_group(
    walls: &[&WallCandidate],
    guid_uses: &mut HashMap<String, usize>,
) -> Option<(Vec<WallCandidate>, usize)> {
    // The frame: the longest wall (the thickest of equals).
    let thickness = |c: &WallCandidate| c.layers.iter().map(|l| l.1 - l.0).sum::<f64>();
    let frame = walls.iter().copied().max_by(|a, b| {
        let la = a.a.distance(a.b);
        let lb = b.a.distance(b.b);
        la.total_cmp(&lb)
            .then(thickness(a).total_cmp(&thickness(b)))
            .then(b.ifc.cmp(&a.ifc))
    })?;
    let o = frame.a;
    let u = (frame.b - frame.a).normalized()?;
    let n = u.perp_left();

    let placed: Vec<Placed> = walls
        .iter()
        .enumerate()
        .map(|(k, c)| {
            let ta = (c.a - o).dot(u);
            let tb = (c.b - o).dot(u);
            let d = (c.a - o).dot(n);
            let same = (c.b - c.a).dot(u) > 0.0;
            let layers: Vec<(f64, f64, Layer)> = c
                .layers
                .iter()
                .map(|(lo, hi, l)| {
                    if same {
                        (d + lo, d + hi, l.clone())
                    } else {
                        (d - hi, d - lo, l.clone())
                    }
                })
                .collect();
            let s0 = layers.iter().map(|l| l.0).fold(f64::INFINITY, f64::min);
            let s1 = layers.iter().map(|l| l.1).fold(f64::NEG_INFINITY, f64::max);
            Placed {
                idx: k,
                t0: ta.min(tb),
                t1: ta.max(tb),
                layers,
                s0,
                s1,
            }
        })
        .collect();

    // Places across (slots), outside in, alike within a couple of
    // millimetres; and bands: a slot with its materials (a cladding of
    // ceramic plates here and of timber slats there share a slot).
    let mut slots: Vec<(f64, f64)> = Vec::new();
    let slot_of: Vec<usize> = placed
        .iter()
        .map(|p| {
            match slots
                .iter()
                .position(|s| (s.0 - p.s0).abs() <= SAME_PLACE && (s.1 - p.s1).abs() <= SAME_PLACE)
            {
                Some(k) => k,
                None => {
                    slots.push((p.s0, p.s1));
                    slots.len() - 1
                }
            }
        })
        .collect();
    let mut order: Vec<usize> = (0..slots.len()).collect();
    order.sort_by(|&x, &y| slots[y].1.total_cmp(&slots[x].1));
    // A clean build-up: slots one after the other, none inside another.
    for w in order.windows(2) {
        if slots[w[1]].1 > slots[w[0]].0 + TOUCH {
            return None;
        }
    }
    let rank: Vec<usize> = (0..slots.len())
        .map(|k| order.iter().position(|&o| o == k).unwrap())
        .collect();
    let mut kinds: Vec<(usize, Vec<u64>)> = Vec::new();
    let band_of: Vec<usize> = placed
        .iter()
        .enumerate()
        .map(|(k, p)| {
            let kind = (
                slot_of[k],
                p.layers
                    .iter()
                    .map(|l| l.2.material.0)
                    .collect::<Vec<u64>>(),
            );
            match kinds.iter().position(|x| *x == kind) {
                Some(i) => i,
                None => {
                    kinds.push(kind);
                    kinds.len() - 1
                }
            }
        })
        .collect();

    // Ends along the stack.
    let mut ends: Vec<f64> = placed.iter().flat_map(|p| [p.t0, p.t1]).collect();
    ends.sort_by(f64::total_cmp);
    let mut stops: Vec<f64> = Vec::new();
    for e in ends {
        if stops.last().is_none_or(|l| e - l > SAME_END) {
            stops.push(e);
        }
    }

    // Pieces between stops with the bands present along them.
    struct Piece {
        t0: f64,
        t1: f64,
        bands: Vec<usize>,
        members: Vec<usize>,
    }
    let mut pieces: Vec<Piece> = Vec::new();
    for w in stops.windows(2) {
        let (p, q) = (w[0], w[1]);
        let covering: Vec<usize> = placed
            .iter()
            .filter(|m| m.t0 <= p + SAME_END && m.t1 >= q - SAME_END)
            .map(|m| m.idx)
            .collect();
        if covering.is_empty() {
            continue;
        }
        // One band per slot: the longest wall's there.
        let mut by_length = covering.clone();
        by_length.sort_by(|&x, &y| {
            (placed[y].t1 - placed[y].t0).total_cmp(&(placed[x].t1 - placed[x].t0))
        });
        let mut present: Vec<usize> = Vec::new();
        for &k in &by_length {
            let slot = kinds[band_of[k]].0;
            if !present.iter().any(|&b| kinds[b].0 == slot) {
                present.push(band_of[k]);
            }
        }
        present.sort_by_key(|&b| rank[kinds[b].0]);
        match pieces.last_mut() {
            Some(last) if (last.t1 - p).abs() <= SAME_END && last.bands == present => {
                last.t1 = q;
                for k in covering {
                    if !last.members.contains(&k) {
                        last.members.push(k);
                    }
                }
            }
            _ => pieces.push(Piece {
                t0: p,
                t1: q,
                bands: present,
                members: covering,
            }),
        }
    }

    let mut result = Vec::new();
    let mut dropped = 0;
    for piece in pieces {
        if piece.t1 - piece.t0 < SHORTEST {
            dropped += 1;
            continue;
        }
        // Each band's layers from the first wall that has it, with that
        // wall's type to name the joined type outside in.
        let mut named: Vec<((f64, f64, Layer), String)> = Vec::new();
        for &b in &piece.bands {
            let k = *piece
                .members
                .iter()
                .find(|&&k| band_of[k] == b)
                .expect("a band comes from a wall");
            let name = short_name(&walls[k].type_key_name);
            named.extend(placed[k].layers.iter().map(|l| (l.clone(), name.clone())));
        }
        // Face to face: the hairline gaps and overlaps between the layers'
        // walls are closed, keeping the outer face where it is.
        named.sort_by(|x, y| y.0 .1.total_cmp(&x.0 .1));
        let mut face = named.first().map_or(0.0, |l| l.0 .1);
        for (l, _) in &mut named {
            l.1 = face;
            l.0 = face - l.2.thickness;
            face = l.0;
        }
        let (layers, layer_names): (Vec<(f64, f64, Layer)>, Vec<String>) =
            named.into_iter().unzip();
        let ms: Vec<&WallCandidate> = piece.members.iter().map(|&k| walls[k]).collect();
        // The wall that gives its identity: the thickest.
        let main = ms.iter().copied().max_by(|a, b| {
            thickness(a)
                .total_cmp(&thickness(b))
                .then(b.ifc.cmp(&a.ifc))
        })?;
        let base = ms.iter().map(|c| c.base).fold(f64::INFINITY, f64::min);
        let top = ms.iter().map(|c| c.top).fold(f64::NEG_INFINITY, f64::max);
        let uses = guid_uses.entry(main.guid.clone()).or_insert(0);
        let guid = if *uses == 0 {
            main.guid.clone()
        } else {
            piece_guid(&main.guid, 100 + *uses)
        };
        *uses += 1;
        let one = piece.bands.len() == 1;
        let (type_key_name, ifc_type) = if one {
            (main.type_key_name.clone(), main.ifc_type)
        } else {
            // Named once its sides are settled, from its layers' walls.
            (String::new(), None)
        };
        let also: Vec<u64> = ms
            .iter()
            .map(|c| c.ifc)
            .chain(ms.iter().flat_map(|c| c.also.iter().copied()))
            .filter(|&id| id != main.ifc)
            .collect();
        result.push(WallCandidate {
            ifc: main.ifc,
            also,
            guid,
            a: snap2(o + u * piece.t0),
            b: snap2(o + u * piece.t1),
            bulge: 0.0,
            layers,
            base: snap(base),
            top: snap(top),
            level: main.level,
            external: ms.iter().any(|c| c.external),
            type_key_name,
            ifc_type,
            layer_names: if one { Vec::new() } else { layer_names },
        });
    }
    (!result.is_empty()).then_some((result, dropped))
}

/// The name of a joined wall's type: its thickness and its layers' wall
/// types, outside in, each once.
pub(super) fn joined_name(thickness: f64, names: &[String]) -> String {
    let mut list: Vec<&str> = Vec::new();
    for n in names {
        if list.last() != Some(&n.as_str()) {
            list.push(n);
        }
    }
    format!("{:.0} mm · {}", thickness * 1000.0, list.join(" + "))
}

/// A type name without its family («Muro básico:Fachada» → «Fachada»).
fn short_name(name: &str) -> String {
    let short = name.rsplit(':').next().unwrap_or(name).trim();
    if short.is_empty() {
        name.trim().to_string()
    } else {
        short.to_string()
    }
}
