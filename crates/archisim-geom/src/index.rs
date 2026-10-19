//! Finding the walls near a point quickly, for the joins of every wall of
//! a large model (thousands of walls): a grid of square cells over the plan,
//! each with the walls whose outline may reach it, and the walls by their
//! end points.
//!
//! Build one [`WallIndex`] for a batch of work (a whole plan, the IFC, the
//! 3D view) and pass it to the `*_in` functions; the functions without an
//! index build one each time, which is fine for one wall.

use std::collections::HashMap;

use archisim_core::{LevelId, Model, OpeningId, Point2, Slab, SlabId, Wall, WallId};

use crate::OpeningGeometry;

/// Side of a cell of the grid, in metres.
const CELL: f64 = 4.0;

type Key = (i64, i64);

fn cell(p: Point2) -> Key {
    ((p.x / CELL).floor() as i64, (p.y / CELL).floor() as i64)
}

/// End points closer than this are the same point: the key is the point
/// rounded to a tenth of a millimetre, looked up with its neighbours.
const END_STEP: f64 = 1e-4;

fn end_key(p: Point2) -> Key {
    (
        (p.x / END_STEP).round() as i64,
        (p.y / END_STEP).round() as i64,
    )
}

/// The walls of a model by where they are.
pub struct WallIndex<'a> {
    pub model: &'a Model,
    cells: HashMap<Key, Vec<WallId>>,
    ends: HashMap<Key, Vec<WallId>>,
    /// Doors, windows and voids by wall.
    openings: HashMap<WallId, Vec<OpeningId>>,
    /// The slabs whose outline may reach each cell (any level).
    slab_cells: HashMap<Key, Vec<SlabId>>,
}

impl<'a> WallIndex<'a> {
    pub fn new(model: &'a Model) -> Self {
        let mut cells: HashMap<Key, Vec<WallId>> = HashMap::new();
        let mut ends: HashMap<Key, Vec<WallId>> = HashMap::new();
        for w in model.walls() {
            let (lo, hi) = bounds(model, w);
            let (a, b) = (cell(lo), cell(hi));
            for i in a.0..=b.0 {
                for j in a.1..=b.1 {
                    cells.entry((i, j)).or_default().push(w.id);
                }
            }
            for p in [w.start, w.end] {
                ends.entry(end_key(p)).or_default().push(w.id);
            }
        }
        let mut openings: HashMap<WallId, Vec<OpeningId>> = HashMap::new();
        for o in model.openings() {
            openings.entry(o.wall).or_default().push(o.id);
        }
        let mut slab_cells: HashMap<Key, Vec<SlabId>> = HashMap::new();
        for s in model.slabs() {
            let (lo, hi) = box_of(&s.outline);
            let (a, b) = (cell(lo), cell(hi));
            for i in a.0..=b.0 {
                for j in a.1..=b.1 {
                    slab_cells.entry((i, j)).or_default().push(s.id);
                }
            }
        }
        Self {
            model,
            slab_cells,
            cells,
            ends,
            openings,
        }
    }

    /// The doors, windows and voids of a wall, along it (as
    /// [`crate::openings_in_wall`], without going through all of them).
    pub fn openings_in_wall(&self, wall: WallId) -> Vec<OpeningGeometry> {
        let mut v: Vec<_> = self
            .openings
            .get(&wall)
            .into_iter()
            .flatten()
            .filter_map(|id| crate::opening_geometry(self.model, *id))
            .collect();
        v.sort_by(|a, b| a.t0.total_cmp(&b.t0));
        v
    }

    /// Walls of `level` whose outline may come within `r` of `p`.
    pub fn near(&self, level: LevelId, p: Point2, r: f64) -> Vec<&'a Wall> {
        let lo = cell(Point2::new(p.x - r, p.y - r));
        let hi = cell(Point2::new(p.x + r, p.y + r));
        let mut ids: Vec<WallId> = Vec::new();
        for i in lo.0..=hi.0 {
            for j in lo.1..=hi.1 {
                if let Some(v) = self.cells.get(&(i, j)) {
                    ids.extend(v.iter().copied());
                }
            }
        }
        self.walls(level, ids)
    }

    /// Walls of `level` whose outline may reach the box `lo`–`hi`.
    pub fn in_box(&self, level: LevelId, lo: Point2, hi: Point2) -> Vec<&'a Wall> {
        let (a, b) = (cell(lo), cell(hi));
        let mut ids: Vec<WallId> = Vec::new();
        for i in a.0..=b.0 {
            for j in a.1..=b.1 {
                if let Some(v) = self.cells.get(&(i, j)) {
                    ids.extend(v.iter().copied());
                }
            }
        }
        self.walls(level, ids)
    }

    /// Walls of any level whose outline may reach the box `lo`–`hi`.
    pub fn in_box_any_level(&self, lo: Point2, hi: Point2) -> Vec<&'a Wall> {
        let (a, b) = (cell(lo), cell(hi));
        let mut ids: Vec<WallId> = Vec::new();
        for i in a.0..=b.0 {
            for j in a.1..=b.1 {
                if let Some(v) = self.cells.get(&(i, j)) {
                    ids.extend(v.iter().copied());
                }
            }
        }
        ids.sort_unstable();
        ids.dedup();
        ids.into_iter()
            .filter_map(|id| self.model.wall(id))
            .collect()
    }

    /// Slabs of any level whose outline may reach the box `lo`–`hi`.
    pub fn slabs_in_box(&self, lo: Point2, hi: Point2) -> Vec<&'a Slab> {
        let (a, b) = (cell(lo), cell(hi));
        let mut ids: Vec<SlabId> = Vec::new();
        for i in a.0..=b.0 {
            for j in a.1..=b.1 {
                if let Some(v) = self.slab_cells.get(&(i, j)) {
                    ids.extend(v.iter().copied());
                }
            }
        }
        ids.sort_unstable();
        ids.dedup();
        ids.into_iter()
            .filter_map(|id| self.model.slab(id))
            .collect()
    }

    /// Walls of `level` with an end at `p` (within the model's tolerance).
    pub fn at_end(&self, level: LevelId, p: Point2) -> Vec<&'a Wall> {
        let k = end_key(p);
        let reach = (archisim_core::EPSILON / END_STEP).ceil() as i64;
        let mut ids: Vec<WallId> = Vec::new();
        for i in -reach..=reach {
            for j in -reach..=reach {
                if let Some(v) = self.ends.get(&(k.0 + i, k.1 + j)) {
                    ids.extend(v.iter().copied());
                }
            }
        }
        self.walls(level, ids)
            .into_iter()
            .filter(|w| w.start.approx_eq(p) || w.end.approx_eq(p))
            .collect()
    }

    /// The walls of those ids on `level`, once each, by id.
    fn walls(&self, level: LevelId, mut ids: Vec<WallId>) -> Vec<&'a Wall> {
        ids.sort_unstable();
        ids.dedup();
        ids.into_iter()
            .filter_map(|id| self.model.wall(id))
            .filter(|w| w.level == level)
            .collect()
    }
}

/// The box a wall's outline stays within: its drawn line (or arc) widened
/// by its thickness on both sides.
pub fn bounds(model: &Model, w: &Wall) -> (Point2, Point2) {
    let t = model.wall_type(w.wall_type).map_or(0.0, |t| t.thickness());
    let mut pts = vec![w.start, w.end];
    if let Some(a) = w.arc() {
        pts.extend(a.polyline(0.0, 0.05));
    }
    let m = t + 0.01;
    let lo = pts
        .iter()
        .fold(Point2::new(f64::INFINITY, f64::INFINITY), |a, p| {
            Point2::new(a.x.min(p.x), a.y.min(p.y))
        });
    let hi = pts
        .iter()
        .fold(Point2::new(f64::NEG_INFINITY, f64::NEG_INFINITY), |a, p| {
            Point2::new(a.x.max(p.x), a.y.max(p.y))
        });
    (lo - Point2::new(m, m), hi + Point2::new(m, m))
}

/// The box a polygon stays within.
pub fn box_of(poly: &[Point2]) -> (Point2, Point2) {
    poly.iter().fold(
        (
            Point2::new(f64::INFINITY, f64::INFINITY),
            Point2::new(f64::NEG_INFINITY, f64::NEG_INFINITY),
        ),
        |(lo, hi), p| {
            (
                Point2::new(lo.x.min(p.x), lo.y.min(p.y)),
                Point2::new(hi.x.max(p.x), hi.y.max(p.y)),
            )
        },
    )
}
