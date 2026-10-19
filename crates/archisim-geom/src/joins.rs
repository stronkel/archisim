//! T-junctions between walls and the intersection priority of their layers
//! (ArchiCAD's "prioridad de intersección", Revit's layer join rules).
//!
//! A wall whose end lies inside another wall (on one of its faces, on its
//! axis or anywhere in its thickness), away from that wall's ends, butts
//! against it: a T-junction. Each layer of the butting wall goes into the
//! host through the host's layers of lower priority, from the face it
//! comes in by, and stops at the first one of the same or higher priority:
//! the structure goes through, the finishes stop. A partition's brick goes
//! through the plaster of a façade up to its inner leaf; a load-bearing wall
//! goes through the inner leaf and the insulation up to the outer leaf.
//! The host loses the part of its layers the other wall goes through.
//!
//! The butting wall never goes through the host's last layer (the far
//! face stays whole), and only goes in at all when it lies within the
//! host's height and away from its corners; otherwise it just stops at the
//! host's face.
//!
//! Curved walls neither butt into others nor take butting walls this way:
//! a wall against a curved one just stops at its face (see
//! [`crate::EndCut::Face`]).

use archisim_core::{LayerFunction, Model, Point2, Vec2, Wall, WallId, EPSILON};

use crate::index::{bounds, WallIndex};
use crate::Line;

/// Intersection priority of a layer by its function alone (its material
/// may have its own: [`Model::layer_priority`]).
pub fn priority(f: LayerFunction) -> u16 {
    f.default_priority()
}

/// The end of a wall that butts against another one.
#[derive(Debug, Clone, PartialEq)]
pub struct TeeEnd {
    pub host: WallId,
    /// The face of the host this wall comes in by.
    pub face: Line,
    /// Unit vector from that face into the host.
    pub inward: Vec2,
    /// How far each layer of this wall (exterior to interior) goes into the
    /// host from that face, in metres.
    pub reach: Vec<f64>,
}

impl TeeEnd {
    /// The line where layer `i` of the butting wall stops.
    pub fn layer_line(&self, i: usize) -> Line {
        let r = self.reach.get(i).copied().unwrap_or(0.0);
        Line::new(self.face.p + self.inward * r, self.face.d)
    }
}

/// Where another wall butts against this one and, layer by layer, what
/// it takes (nothing, if it just stops at the face).
#[derive(Debug, Clone, PartialEq)]
pub struct Notch {
    /// The wall that butts against this one.
    pub by: WallId,
    /// Bottom and top of the butting wall, above the level.
    pub z: (f64, f64),
    /// The face of this wall it comes in by (as an offset from the drawn
    /// line).
    pub face_offset: f64,
    /// One region per layer of the butting wall that goes in.
    pub regions: Vec<NotchRegion>,
    /// Every layer boundary line of the butting wall, exterior to interior.
    pub boundaries: Vec<Line>,
}

/// The part of this wall taken by one layer of the butting wall.
#[derive(Debug, Clone, PartialEq)]
pub struct NotchRegion {
    /// The two faces of that layer, as lines along the butting wall.
    pub sides: (Line, Line),
    /// How deep it goes from the face.
    pub depth: f64,
    /// The layers of this wall it goes right through (exterior to interior
    /// indices).
    pub layers: Vec<usize>,
}

impl NotchRegion {
    /// Signed distance across the region's sides, and its range: points
    /// with `f` in `lo..hi` are inside the strip.
    pub fn strip(&self) -> (impl Fn(Point2) -> f64, f64, f64) {
        let (s1, s2) = self.sides;
        let nl = s1.d.normalized().map_or(Vec2::default(), Vec2::perp_left);
        let w = (s2.p - s1.p).dot(nl);
        let p0 = s1.p;
        (move |p: Point2| (p - p0).dot(nl), w.min(0.0), w.max(0.0))
    }
}

/// Basic plan data of a wall: drawn line, unit vectors, faces and layers.
struct Frame {
    start: Point2,
    u: Vec2,
    n: Vec2,
    len: f64,
    /// Offsets of every layer boundary, exterior to interior.
    tops: Vec<f64>,
    priorities: Vec<u16>,
    thicknesses: Vec<f64>,
}

impl Frame {
    fn of(model: &Model, w: &Wall) -> Option<Self> {
        let wtype = model.wall_type(w.wall_type)?;
        let u = (w.end - w.start).normalized()?;
        let (ext, _) = w.face_offsets(wtype.thickness());
        let mut tops = vec![ext];
        for l in &wtype.layers {
            let last = *tops.last()?;
            tops.push(last - l.thickness);
        }
        Some(Self {
            start: w.start,
            u,
            n: u.perp_left(),
            len: w.length(),
            tops,
            priorities: wtype
                .layers
                .iter()
                .map(|l| model.layer_priority(l))
                .collect(),
            thicknesses: wtype.layers.iter().map(|l| l.thickness).collect(),
        })
    }

    fn lo(&self) -> f64 {
        self.tops[0].min(self.tops[self.tops.len() - 1])
    }

    fn hi(&self) -> f64 {
        self.tops[0].max(self.tops[self.tops.len() - 1])
    }
}

/// The T-junction at one end of a wall, if it butts against another wall.
pub fn tee_end(model: &Model, wall: &Wall, at_start: bool) -> Option<TeeEnd> {
    tee_end_in(&WallIndex::new(model), wall, at_start)
}

/// [`tee_end`] with the walls found through an index.
pub fn tee_end_in(idx: &WallIndex, wall: &Wall, at_start: bool) -> Option<TeeEnd> {
    if wall.is_curved() {
        return None;
    }
    let p = if at_start { wall.start } else { wall.end };
    // Ends shared with another wall are corners, not T-junctions.
    let shared = idx.at_end(wall.level, p).iter().any(|o| o.id != wall.id);
    if shared {
        return None;
    }
    idx.near(wall.level, p, 0.0)
        .into_iter()
        .filter(|h| h.id != wall.id && !h.is_curved())
        .find_map(|h| tee_with(idx, wall, at_start, h))
}

/// Whether a point lies in a wall's band (between its faces, along it).
pub(crate) fn in_band(model: &Model, host: &Wall, p: Point2) -> bool {
    if host.is_curved() {
        return false;
    }
    let Some(f) = Frame::of(model, host) else {
        return false;
    };
    let t = (p - f.start).dot(f.u);
    let s = (p - f.start).dot(f.n);
    t > EPSILON && t < f.len - EPSILON && s >= f.lo() - 1e-6 && s <= f.hi() + 1e-6
}

/// The T-junction of one end of `wall` against `host`, if it butts there.
fn tee_with(idx: &WallIndex, wall: &Wall, at_start: bool, host: &Wall) -> Option<TeeEnd> {
    let model = idx.model;
    let (p, q) = if at_start {
        (wall.start, wall.end)
    } else {
        (wall.end, wall.start)
    };
    if !in_band(model, host, p) {
        return None;
    }
    let h = Frame::of(model, host)?;
    let w = Frame::of(model, wall)?;
    // Coming in at a clear angle (more than 15° off the host).
    let towards_q = (q - p).normalized()?;
    let side = towards_q.dot(h.n);
    if side.abs() < 0.26 {
        return None;
    }
    let face_offset = if side > 0.0 { h.hi() } else { h.lo() };
    let inward = if side > 0.0 { -h.n } else { h.n };
    let face = Line::new(h.start + h.n * face_offset, h.u);

    // The host's layers in the order they are met from that face.
    let n = h.priorities.len();
    let from_exterior = (h.tops[0] - face_offset).abs() < 1e-9;
    let order: Vec<usize> = if from_exterior {
        (0..n).collect()
    } else {
        (0..n).rev().collect()
    };
    let far = h.thicknesses[order[n - 1]];
    let total: f64 = h.thicknesses.iter().sum();

    // Only inside the host's height and away from its corners.
    let fits_height = wall.base_offset >= host.base_offset - EPSILON
        && wall.base_offset + wall.height <= host.base_offset + host.height + EPSILON;
    let reach: Vec<f64> = w
        .priorities
        .iter()
        .map(|prio| {
            let mut depth = 0.0;
            for &i in &order {
                if h.priorities[i] >= *prio {
                    break;
                }
                depth += h.thicknesses[i];
            }
            depth.min(total - far)
        })
        .collect();
    let mut tee = TeeEnd {
        host: host.id,
        face,
        inward,
        reach,
    };
    let deepest = tee.reach.iter().copied().fold(0.0, f64::max);
    let margin = corner_margin(idx, host);
    let clear = (0..w.tops.len()).all(|b| {
        let side_line = Line::new(w.start + w.n * w.tops[b], w.u);
        [0.0, deepest].iter().all(|d| {
            Line::new(face.p + inward * *d, face.d)
                .intersect(&side_line)
                .is_some_and(|x| {
                    let t = (x - h.start).dot(h.u);
                    t > margin.0 && t < h.len - margin.1
                })
        })
    });
    if !fits_height || !clear {
        tee.reach.iter_mut().for_each(|r| *r = 0.0);
    }
    Some(tee)
}

/// How far from each end of a wall its corners reach: the thickness of the
/// thickest wall joined there (zero at a free end).
fn corner_margin(idx: &WallIndex, w: &Wall) -> (f64, f64) {
    let at = |p: Point2| {
        idx.at_end(w.level, p)
            .into_iter()
            .filter(|o| o.id != w.id)
            .filter_map(|o| idx.model.wall_type(o.wall_type).map(|t| t.thickness()))
            .fold(0.0, f64::max)
            + EPSILON
    };
    (at(w.start), at(w.end))
}

/// Where other walls go into `host`.
pub fn notches(model: &Model, host: &Wall) -> Vec<Notch> {
    notches_in(&WallIndex::new(model), host)
}

/// [`notches`] with the walls found through an index.
pub fn notches_in(idx: &WallIndex, host: &Wall) -> Vec<Notch> {
    let model = idx.model;
    if host.is_curved() {
        return Vec::new();
    }
    let Some(h) = Frame::of(model, host) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    let (lo, hi) = bounds(model, host);
    for w in idx.in_box(host.level, lo, hi) {
        if w.id == host.id {
            continue;
        }
        for at_start in [true, false] {
            let p = if at_start { w.start } else { w.end };
            if !in_band(model, host, p) {
                continue;
            }
            let Some(tee) = tee_end_in(idx, w, at_start) else {
                continue;
            };
            if tee.host != host.id {
                continue;
            }
            let Some(f) = Frame::of(model, w) else {
                continue;
            };
            let boundaries: Vec<Line> = f
                .tops
                .iter()
                .map(|o| Line::new(f.start + f.n * *o, f.u))
                .collect();
            let face_offset = (tee.face.p - h.start).dot(h.n);
            let regions = tee
                .reach
                .iter()
                .enumerate()
                .filter(|(_, r)| **r > EPSILON)
                .map(|(j, r)| {
                    // Host layers wholly within that depth from the face.
                    let layers = (0..h.priorities.len())
                        .filter(|&i| {
                            let (a, b) = (h.tops[i], h.tops[i + 1]);
                            let far = (a - face_offset).abs().max((b - face_offset).abs());
                            far <= *r + 1e-9
                        })
                        .collect();
                    NotchRegion {
                        sides: (boundaries[j], boundaries[j + 1]),
                        depth: *r,
                        layers,
                    }
                })
                .collect();
            out.push(Notch {
                by: w.id,
                z: (w.base_offset, w.base_offset + w.height),
                face_offset,
                regions,
                boundaries,
            });
        }
    }
    out
}
