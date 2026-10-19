//! Where walls and slabs meet ("encuentros"), as ArchiCAD resolves them:
//! layer by layer, by intersection priority. Where a layer of a slab and a
//! layer of a wall would take the same room, the one of higher priority
//! keeps it ([`Model::layer_priority`]); on a tie, the slab. So with the
//! usual priorities the structure of the slab cuts the wall (the wall stops
//! under it and starts again over it), while the structure of the wall goes
//! through the slab's ceiling plaster, and its screed and floor finish if
//! the wall goes on above (they stop at its faces).
//!
//! A wall layer goes into a slab from where the wall is: from below if the
//! wall is under the slab, from above if it goes on over it, layer after
//! layer while it wins, and stops at the first one it loses to (so a wall up
//! to the top of a roof slab stops under its concrete: it does not come out
//! again through the insulation and the gravel above). It goes through a
//! slab layer only if it covers that layer's whole thickness.
//!
//! Only straight walls and level slabs meet this way (curved walls and
//! inclined slabs overlap as before). The same decisions build the 3D
//! solids ([`crate::solid`]) and the IFC, so both show the same junction.

use archisim_core::{signed_area, Layer, Model, Point2, Slab, SlabId, Wall, WallId, EPSILON};
use i_overlay::core::fill_rule::FillRule;
use i_overlay::core::overlay_rule::OverlayRule;
use i_overlay::float::single::SingleFloatOverlay;

use crate::index::{bounds, box_of, WallIndex};
use std::collections::HashMap;

use crate::{ensure_ccw, wall_geometry_in, WallGeometry};

/// A polygon with holes: the outline (counter-clockwise), then the holes.
pub type Shape = (Vec<Point2>, Vec<Vec<Point2>>);

/// A layer of a slab where it meets a wall.
#[derive(Debug, Clone, PartialEq)]
pub struct SlabCut {
    pub slab: SlabId,
    pub slab_layer: usize,
    /// Its bottom and top, from project zero.
    pub z: (f64, f64),
    /// The wall's layers it takes (exterior first): none of them is there.
    pub taken: Vec<usize>,
    /// The wall's layers that go through it (it loses their room).
    pub through: Vec<usize>,
    /// The slab in plan.
    pub outline: Vec<Point2>,
    pub holes: Vec<Vec<Point2>>,
}

impl SlabCut {
    /// Whether `p` lies on the slab (in its outline, not in a hole).
    pub fn contains(&self, p: Point2) -> bool {
        inside(p, &self.outline) && !self.holes.iter().any(|h| inside(p, h))
    }
}

/// Whether `p` lies inside the polygon (even–odd).
pub fn inside(p: Point2, poly: &[Point2]) -> bool {
    let mut odd = false;
    let n = poly.len();
    for i in 0..n {
        let (a, b) = (poly[i], poly[(i + n - 1) % n]);
        if (a.y > p.y) != (b.y > p.y) && p.x < a.x + (p.y - a.y) / (b.y - a.y) * (b.x - a.x) {
            odd = !odd;
        }
    }
    odd
}

/// Bottom and top of a wall from project zero, as its solids are built
/// (up to the ridge when it is trimmed under a roof).
pub fn wall_z_range(model: &Model, wall: &Wall) -> (f64, f64) {
    let level = model.level(wall.level).map_or(0.0, |l| l.elevation);
    let base = level + wall.base_offset;
    let trimmed = wall.arc().is_none() && !model.wall_trim_planes(wall.id).is_empty();
    let top = if trimmed {
        model.wall_top_z(wall)
    } else {
        base + wall.height
    };
    (base, top)
}

/// The layers of a level slab from project zero, top first: (bottom, top)
/// of each. `None` for an inclined slab.
pub fn slab_bands(model: &Model, slab: &Slab) -> Option<Vec<(f64, f64)>> {
    if slab.slope.is_some() {
        return None;
    }
    let stype = model.slab_type(slab.slab_type)?;
    let mut top = model.level(slab.level)?.elevation + slab.top_offset;
    Some(
        stype
            .layers
            .iter()
            .map(|l| {
                let band = (top - l.thickness, top);
                top -= l.thickness;
                band
            })
            .collect(),
    )
}

/// Whether a wall layer beats a slab layer it covers.
fn wall_wins(model: &Model, wall_layer: &Layer, slab_layer: &Layer) -> bool {
    model.layer_priority(wall_layer) > model.layer_priority(slab_layer)
}

/// For a wall (its layers and heights) and a level slab (its layers and
/// their bands, top first): for each slab layer the wall overlaps, which
/// wall layers go through it (see the module's rule). `None` where they do
/// not overlap.
fn through(
    model: &Model,
    wall_layers: &[Layer],
    range: (f64, f64),
    bands: &[(f64, f64)],
    slab_layers: &[Layer],
) -> Vec<Option<Vec<bool>>> {
    let n = bands.len().min(slab_layers.len());
    let mut out: Vec<Option<Vec<bool>>> = (0..n)
        .map(|j| overlap(range, bands[j]).then(|| vec![false; wall_layers.len()]))
        .collect();
    if n == 0 {
        return out;
    }
    let (bottom, top) = (bands[n - 1].0, bands[0].1);
    // Where the wall has a body to go in from: below the slab (or starting
    // inside it), above it (or ending inside it).
    let from_below = (range.0 - bottom).abs() > EPSILON;
    let from_above = (range.1 - top).abs() > EPSILON;
    let order_up: Vec<usize> = (0..n).rev().filter(|&j| out[j].is_some()).collect();
    let order_down: Vec<usize> = (0..n).filter(|&j| out[j].is_some()).collect();
    for (k, wl) in wall_layers.iter().enumerate() {
        for (walk, on) in [(&order_up, from_below), (&order_down, from_above)] {
            if !on {
                continue;
            }
            for &j in walk.iter() {
                if spans(range, bands[j]) && wall_wins(model, wl, &slab_layers[j]) {
                    if let Some(flags) = out[j].as_mut() {
                        flags[k] = true;
                    }
                } else {
                    break;
                }
            }
        }
    }
    out
}

fn overlap(a: (f64, f64), b: (f64, f64)) -> bool {
    a.1.min(b.1) - a.0.max(b.0) > EPSILON
}

fn spans(wall: (f64, f64), band: (f64, f64)) -> bool {
    wall.0 <= band.0 + EPSILON && wall.1 >= band.1 - EPSILON
}

fn boxes_meet(a: (Point2, Point2), b: (Point2, Point2)) -> bool {
    a.0.x <= b.1.x && b.0.x <= a.1.x && a.0.y <= b.1.y && b.0.y <= a.1.y
}

/// The layers of the slabs a straight wall meets, with the wall layers
/// each one takes and those going through it.
pub fn wall_slab_cuts(idx: &WallIndex, wall: WallId) -> Vec<SlabCut> {
    let model = idx.model;
    let Some(w) = model.wall(wall) else {
        return Vec::new();
    };
    let Some(wtype) = model.wall_type(w.wall_type) else {
        return Vec::new();
    };
    if w.arc().is_some() {
        return Vec::new();
    }
    let range = wall_z_range(model, w);
    let wbox = bounds(model, w);
    let mut out = Vec::new();
    for slab in idx.slabs_in_box(wbox.0, wbox.1) {
        if !boxes_meet(wbox, box_of(&slab.outline)) {
            continue;
        }
        let (Some(bands), Some(stype)) = (slab_bands(model, slab), model.slab_type(slab.slab_type))
        else {
            continue;
        };
        let passing = through(model, &wtype.layers, range, &bands, &stype.layers);
        for (j, flags) in passing.iter().enumerate() {
            let Some(flags) = flags else { continue };
            let band = &bands[j];
            let (through, taken): (Vec<usize>, Vec<usize>) =
                (0..wtype.layers.len()).partition(|&k| flags[k]);
            out.push(SlabCut {
                slab: slab.id,
                slab_layer: j,
                z: *band,
                taken,
                through,
                outline: slab.outline.clone(),
                holes: slab.holes.clone(),
            });
        }
    }
    out
}

/// For each layer of a level slab (top first), the plan pieces of the
/// wall layers that go through it.
pub fn slab_wall_cuts(idx: &WallIndex, slab: SlabId) -> Vec<Vec<Vec<Point2>>> {
    slab_wall_cuts_with(idx, slab, &HashMap::new())
}

/// [`slab_wall_cuts`] with the geometry of walls already worked out.
pub fn slab_wall_cuts_with(
    idx: &WallIndex,
    slab: SlabId,
    known: &HashMap<WallId, WallGeometry>,
) -> Vec<Vec<Vec<Point2>>> {
    let model = idx.model;
    let Some(s) = model.slab(slab) else {
        return Vec::new();
    };
    let (Some(bands), Some(stype)) = (slab_bands(model, s), model.slab_type(s.slab_type)) else {
        return Vec::new();
    };
    let mut out: Vec<Vec<Vec<Point2>>> = vec![Vec::new(); bands.len()];
    let sbox = box_of(&s.outline);
    for w in idx.in_box_any_level(sbox.0, sbox.1) {
        if w.arc().is_some() || !boxes_meet(sbox, bounds(model, w)) {
            continue;
        }
        let Some(wtype) = model.wall_type(w.wall_type) else {
            continue;
        };
        let range = wall_z_range(model, w);
        let level = model.level(w.level).map_or(0.0, |l| l.elevation);
        let mut geo = None;
        let passing = through(model, &wtype.layers, range, &bands, &stype.layers);
        for (j, flags) in passing.iter().enumerate() {
            let Some(flags) = flags else { continue };
            let band = &bands[j];
            for (k, _) in flags.iter().enumerate().filter(|(_, on)| **on) {
                let g = geo.get_or_insert_with(|| match known.get(&w.id) {
                    Some(g) => Some(std::borrow::Cow::Borrowed(g)),
                    None => wall_geometry_in(idx, w.id).map(std::borrow::Cow::Owned),
                });
                let Some(g) = g else { continue };
                let z = (band.0 + band.1) / 2.0 - level;
                out[j].extend(g.layer_pieces(k, Some(z)));
            }
        }
    }
    out
}

/// A prism a wall or slab loses at a junction: its plan shapes, and its
/// bottom and top from project zero.
pub type Void = (Vec<Shape>, f64, f64);

fn slab_shape(outline: &[Point2], holes: &[Vec<Point2>]) -> Vec<Shape> {
    vec![(ensure_ccw(outline.to_vec()), holes.to_vec())]
}

fn pieces_as_shapes(pieces: &[Vec<Point2>]) -> Vec<Shape> {
    pieces
        .iter()
        .filter(|p| p.len() >= 3)
        .map(|p| (ensure_ccw(p.clone()), Vec::new()))
        .collect()
}

/// What a straight wall loses to the slabs it meets (the room of the
/// slab layers that take its layers), as prisms: for the IFC.
pub fn wall_slab_voids(idx: &WallIndex, g: &crate::WallGeometry) -> Vec<Void> {
    let model = idx.model;
    let wall = g.wall;
    let Some(w) = model.wall(wall) else {
        return Vec::new();
    };
    let range = wall_z_range(model, w);
    let level = model.level(w.level).map_or(0.0, |l| l.elevation);
    let mut out = Vec::new();
    for cut in wall_slab_cuts(idx, wall) {
        if cut.taken.is_empty() {
            continue;
        }
        let z = (cut.z.0 + cut.z.1) / 2.0 - level;
        let pieces: Vec<Vec<Point2>> = cut
            .taken
            .iter()
            .flat_map(|&k| g.layer_pieces(k, Some(z)))
            .collect();
        let shapes = intersection(
            &slab_shape(&cut.outline, &cut.holes),
            &pieces_as_shapes(&pieces),
        );
        let (z0, z1) = (cut.z.0.max(range.0), cut.z.1.min(range.1));
        if !shapes.is_empty() && z1 - z0 > EPSILON {
            out.push((shapes, z0, z1));
        }
    }
    out
}

/// What a level slab loses to the walls going through its layers, as
/// prisms: for the IFC.
pub fn slab_wall_voids(
    idx: &WallIndex,
    slab: SlabId,
    known: &HashMap<WallId, WallGeometry>,
) -> Vec<Void> {
    let model = idx.model;
    let Some(s) = model.slab(slab) else {
        return Vec::new();
    };
    let Some(bands) = slab_bands(model, s) else {
        return Vec::new();
    };
    let whole = slab_shape(&s.outline, &s.holes);
    slab_wall_cuts_with(idx, slab, known)
        .iter()
        .zip(bands)
        .filter(|(pieces, _)| !pieces.is_empty())
        .map(|(pieces, (z0, z1))| (intersection(&whole, &pieces_as_shapes(pieces)), z0, z1))
        .filter(|(shapes, _, _)| !shapes.is_empty())
        .collect()
}

fn to_xy(poly: &[Point2]) -> Vec<[f64; 2]> {
    poly.iter().map(|p| [p.x, p.y]).collect()
}

fn from_xy(c: &[[f64; 2]]) -> Vec<Point2> {
    c.iter().map(|p| Point2::new(p[0], p[1])).collect()
}

/// Shapes of i_overlay as ours: outline counter-clockwise, holes after.
fn shapes(result: Vec<Vec<Vec<[f64; 2]>>>) -> Vec<Shape> {
    result
        .into_iter()
        .filter_map(|shape| {
            let mut it = shape.into_iter();
            let outer = ensure_ccw(from_xy(&it.next()?));
            if outer.len() < 3 || signed_area(&outer).abs() < 1e-10 {
                return None;
            }
            let holes = it
                .map(|h| from_xy(&h))
                .filter(|h| h.len() >= 3 && signed_area(h).abs() >= 1e-10)
                .collect();
            Some((outer, holes))
        })
        .collect()
}

/// Contours of shapes, outlines counter-clockwise and holes clockwise, so
/// that "non-zero" fills them as they are.
fn contours(list: &[Shape]) -> Vec<Vec<[f64; 2]>> {
    let mut out = Vec::new();
    for (outer, holes) in list {
        out.push(to_xy(&ensure_ccw(outer.clone())));
        for h in holes {
            let mut h = ensure_ccw(h.clone());
            h.reverse();
            out.push(to_xy(&h));
        }
    }
    out
}

/// `a` minus `b`.
pub fn difference(a: &[Shape], b: &[Shape]) -> Vec<Shape> {
    if b.is_empty() {
        return a.to_vec();
    }
    shapes(contours(a).overlay(&contours(b), OverlayRule::Difference, FillRule::NonZero))
}

/// The part of `a` inside `b`.
pub fn intersection(a: &[Shape], b: &[Shape]) -> Vec<Shape> {
    shapes(contours(a).overlay(&contours(b), OverlayRule::Intersect, FillRule::NonZero))
}

/// Area of shapes.
pub fn area(list: &[Shape]) -> f64 {
    list.iter()
        .map(|(o, hs)| signed_area(o).abs() - hs.iter().map(|h| signed_area(h).abs()).sum::<f64>())
        .sum()
}

/// What is left of each layer of a slab (its outline less its holes) once
/// the wall layers going through it take their room: `cuts[j]` are their
/// plan pieces for layer `j`.
pub fn slab_layer_regions(
    outline: &[Point2],
    holes: &[Vec<Point2>],
    cuts: &[Vec<Vec<Point2>>],
) -> Vec<Vec<Shape>> {
    let whole = vec![(ensure_ccw(outline.to_vec()), holes.to_vec())];
    cuts.iter()
        .map(|c| {
            let pieces: Vec<Shape> = c
                .iter()
                .filter(|p| p.len() >= 3)
                .map(|p| (ensure_ccw(p.clone()), Vec::new()))
                .collect();
            difference(&whole, &pieces)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sq(x0: f64, y0: f64, x1: f64, y1: f64) -> Vec<Point2> {
        vec![
            Point2::new(x0, y0),
            Point2::new(x1, y0),
            Point2::new(x1, y1),
            Point2::new(x0, y1),
        ]
    }

    #[test]
    fn layers_lose_the_room_of_the_walls_through_them() {
        // A 10 × 10 slab with a 1 × 1 hole; a wall strip 0.2 wide across
        // it and a second one overlapping the first at a corner.
        let regions = slab_layer_regions(
            &sq(0.0, 0.0, 10.0, 10.0),
            &[sq(6.0, 6.0, 7.0, 7.0)],
            &[
                vec![],
                vec![sq(-0.1, 4.9, 10.1, 5.1), sq(4.9, -0.1, 5.1, 5.1)],
            ],
        );
        assert_eq!(regions.len(), 2);
        assert!((area(&regions[0]) - 99.0).abs() < 1e-6);
        // 99 − 10 × 0.2 − 4.9 × 0.2 (the corner counted once).
        assert!(
            (area(&regions[1]) - (99.0 - 2.0 - 0.98)).abs() < 1e-6,
            "{}",
            area(&regions[1])
        );
        assert_eq!(regions[1].len(), 3);
    }
}
