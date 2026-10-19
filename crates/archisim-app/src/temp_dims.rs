//! Temporary dimensions (Revit's "cotas temporales"): while a wall or a
//! door or window is selected, blue dimensions to the nearest references
//! around it. Typing a value on one of them moves the element so that the
//! dimension becomes exactly that, as one undoable step.
//!
//! - **Door or window**: along its wall, from each jamb (the side of the
//!   rough opening) to the nearest reference on that side, measured on the
//!   face it opens to: the face of a wall that meets or crosses it, the end
//!   of the wall, or the jamb of the next door or window. Editing one slides
//!   the opening along the wall.
//! - **Wall**: the length of its drawn line (editing it moves its end, and
//!   the walls joined there follow, as when stretching it), and the clear
//!   distance from each face to the nearest parallel wall on that side
//!   (editing it moves the wall across, with the walls joined to it).
//!
//! Each dimension has two witness ends, as in Revit: the **reference**
//! (what it measures to) can be dragged to any other point (a corner, a
//! face, the end of a wall, the project origin…), and the **own** end can
//! be moved to another line of the element itself: a door or window's
//! other jamb or its centre; a wall's other face or its axis. These
//! choices ([`Anchors`]) last while the element stays selected.
//!
//! Nothing here knows about the screen: [`temp_dims`] says what to measure
//! and [`edit`] turns a typed value into the drag that does it.

use crate::geo::Geo;
use std::collections::BTreeSet;

use archisim_core::{Model, OpeningId, Point2, WallId, WallSide};
use archisim_geom::openings_in_wall;

use crate::drag::{Drag, Grip};
use crate::pick::Element;

/// What changes when a temporary dimension is edited.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Target {
    /// The distance from a jamb of a door or window to a reference along
    /// its wall. `sign` is +1 if the reference is towards the wall's end
    /// (a larger value moves the opening towards its start), −1 otherwise.
    Opening { opening: OpeningId, sign: f64 },
    /// The clear distance from a wall face to a parallel wall; `towards` is
    /// the unit vector from this wall to the other one.
    WallOffset { wall: WallId, towards: Point2 },
    /// The length of the wall's drawn line (from `from` metres along it
    /// to its end), changed from its end.
    WallLength { wall: WallId, from: f64 },
}

/// Which dimension of the selection a [`TempDim`] is: stays the same when
/// its ends are moved.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Slot {
    /// From a door or window to the reference before it along the wall.
    OpeningBefore(OpeningId),
    /// … and after it.
    OpeningAfter(OpeningId),
    WallLength(WallId),
    /// The clear distance on the wall's exterior side (0) or interior (1).
    WallSide(WallId, u8),
}

/// The witness ends of a dimension moved by hand.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Anchor {
    /// The point the dimension measures to, instead of the nearest one.
    pub reference: Option<Point2>,
    /// The element's own line it measures from: for a door or window, the
    /// fraction of its width from its first jamb (0, ½ or 1); for a wall,
    /// its offset to the left of the drawn line (a face or the axis).
    pub own: Option<f64>,
}

/// The anchors moved by hand, by dimension.
pub type Anchors = std::collections::HashMap<Slot, Anchor>;

/// Which end of a dimension is grabbed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum End {
    /// `a`, the reference.
    Reference,
    /// `b`, the element's own line.
    Own,
}

/// One temporary dimension: the two points it measures, the side its line
/// is drawn on (a unit vector, or zero to draw it between the points), its
/// value and what editing it changes.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TempDim {
    pub a: Point2,
    pub b: Point2,
    pub side: Point2,
    pub value: f64,
    pub target: Target,
    pub slot: Slot,
}

/// Parallel walls farther than this are not measured to.
const MAX_REACH: f64 = 30.0;
const EPS: f64 = 1e-6;

/// The temporary dimensions of a selection: only for a single wall or a
/// single door or window, as in Revit.
#[cfg(test)]
pub fn temp_dims<G: Geo + ?Sized>(geo: &G, selection: &BTreeSet<Element>) -> Vec<TempDim> {
    temp_dims_anchored(geo, selection, &Anchors::new())
}

/// [`temp_dims`] with some witness ends moved by hand.
pub fn temp_dims_anchored<G: Geo + ?Sized>(
    geo: &G,
    selection: &BTreeSet<Element>,
    anchors: &Anchors,
) -> Vec<TempDim> {
    let model = geo.model();
    if selection.len() != 1 {
        return Vec::new();
    }
    match selection.iter().next() {
        Some(Element::Opening(id)) => opening_dims(geo, *id, anchors),
        Some(Element::Wall(id)) => wall_dims(model, *id, anchors),
        _ => Vec::new(),
    }
}

/// Where the own end of a dimension goes when dropped at `p`: the nearest
/// jamb or centre of a door or window, or the nearest face or axis of a
/// wall.
pub fn own_choice(model: &Model, slot: Slot, p: Point2) -> Option<f64> {
    match slot {
        Slot::OpeningBefore(id) | Slot::OpeningAfter(id) => {
            let o = model.opening(id)?;
            let g = openings_in_wall(model, o.wall)
                .into_iter()
                .find(|g| g.opening == id)?;
            let t = (p - g.point(0.0, 0.0)).dot(g.u);
            let width = g.t1 - g.t0;
            [0.0, 0.5, 1.0].into_iter().min_by(|a, b| {
                (g.t0 + a * width - t)
                    .abs()
                    .total_cmp(&(g.t0 + b * width - t).abs())
            })
        }
        Slot::WallSide(id, _) => {
            let w = model.wall(id)?;
            let t = model.wall_type(w.wall_type)?;
            let n = (w.end - w.start).normalized()?.perp_left();
            let o = (p - w.start).dot(n);
            let (ext, int) = w.face_offsets(t.thickness());
            [ext, (ext + int) / 2.0, int]
                .into_iter()
                .min_by(|a, b| (a - o).abs().total_cmp(&(b - o).abs()))
        }
        Slot::WallLength(_) => None,
    }
}

/// Where the line through `p` along `u` crosses the segment `a`–`b`, as a
/// distance along `u` from `p`.
fn cross_line(p: Point2, u: Point2, a: Point2, b: Point2) -> Option<f64> {
    let e = b - a;
    let den = u.x * e.y - u.y * e.x;
    if den.abs() < 1e-12 {
        return None;
    }
    let w = a - p;
    let s = -(u.x * w.y - u.y * w.x) / den;
    let t = (w.x * e.y - w.y * e.x) / den;
    (-EPS..=1.0 + EPS).contains(&s).then_some(t)
}

fn opening_dims<G: Geo + ?Sized>(geo: &G, id: OpeningId, anchors: &Anchors) -> Vec<TempDim> {
    let model = geo.model();
    let Some(o) = model.opening(id) else {
        return Vec::new();
    };
    let Some(host) = model.wall(o.wall) else {
        return Vec::new();
    };
    let ops = openings_in_wall(model, host.id);
    let Some(g) = ops.iter().find(|g| g.opening == id) else {
        return Vec::new();
    };
    // Measured on the face the leaves open to (the room side), and drawn
    // a little off it, away from the wall.
    let face = g.swing_face();
    let away = match o.opens_to {
        WallSide::Exterior => g.n,
        WallSide::Interior => -g.n,
    };
    let origin = g.point(0.0, face);
    let mut refs: Vec<f64> = Vec::new();
    // The ends of this wall's face, and where other walls meet or cross it.
    let mut outlines: Vec<Vec<Point2>> = Vec::new();
    if let Some(hg) = geo.wall_geometry(host.id) {
        outlines.push(hg.outline_at(None));
    }
    for w in model.walls_on_level(host.level).filter(|w| w.id != host.id) {
        if let Some(wg) = geo.wall_geometry(w.id) {
            outlines.push(wg.outline.clone());
        }
    }
    let length = host.length();
    for poly in &outlines {
        let n = poly.len();
        for i in 0..n {
            if let Some(t) = cross_line(origin, g.u, poly[i], poly[(i + 1) % n]) {
                if t > -1.0 && t < length + 1.0 {
                    refs.push(t);
                }
            }
            // Corners lying on the face (edges along it do not cross it).
            let p = poly[i];
            if ((p - origin).dot(g.n)).abs() < EPS {
                refs.push((p - origin).dot(g.u));
            }
        }
    }
    // The jambs of the other doors and windows in the wall.
    for other in ops.iter().filter(|x| x.opening != id) {
        refs.push(other.t0);
        refs.push(other.t1);
    }
    let before = refs
        .iter()
        .copied()
        .filter(|t| *t <= g.t0 + EPS)
        .max_by(f64::total_cmp);
    let after = refs
        .iter()
        .copied()
        .filter(|t| *t >= g.t1 - EPS)
        .min_by(f64::total_cmp);
    let at = |t: f64| g.point(t, face);
    let width = g.t1 - g.t0;
    let mut v = Vec::new();
    for (slot, found, default_own) in [
        (Slot::OpeningBefore(id), before, 0.0),
        (Slot::OpeningAfter(id), after, 1.0),
    ] {
        let anchor = anchors.get(&slot).copied().unwrap_or_default();
        let reference = anchor.reference.map(|p| (p - origin).dot(g.u)).or(found);
        let Some(t_ref) = reference else {
            continue;
        };
        let t_own = g.t0 + anchor.own.unwrap_or(default_own) * width;
        // The reference is the dimension's `a`, the opening its `b`.
        let sign = if t_ref <= t_own { -1.0 } else { 1.0 };
        v.push(TempDim {
            a: at(t_ref),
            b: at(t_own),
            side: away,
            value: (t_own - t_ref).abs(),
            target: Target::Opening { opening: id, sign },
            slot,
        });
    }
    v.retain(|d| d.value > EPS);
    v
}

fn wall_dims(model: &Model, id: WallId, anchors: &Anchors) -> Vec<TempDim> {
    let Some(w) = model.wall(id) else {
        return Vec::new();
    };
    // Curved walls are measured in Properties (length and radius).
    if w.is_curved() {
        return Vec::new();
    }
    let Some(t) = model.wall_type(w.wall_type) else {
        return Vec::new();
    };
    let Some(u) = (w.end - w.start).normalized() else {
        return Vec::new();
    };
    let n = u.perp_left();
    let (ext, int) = w.face_offsets(t.thickness());
    let length = w.length();
    // The length, from the start or from a reference moved by hand.
    let from = anchors
        .get(&Slot::WallLength(id))
        .and_then(|a| a.reference)
        .map_or(0.0, |p| (p - w.start).dot(u).min(length - 0.01));
    let mut v = vec![TempDim {
        a: w.start + u * from + n * ext,
        b: w.end + n * ext,
        side: n,
        value: length - from,
        target: Target::WallLength { wall: id, from },
        slot: Slot::WallLength(id),
    }];
    // The nearest parallel wall on each side, overlapping this one.
    // (signed offset of its facing face, middle of the overlap)
    let mut best: [Option<(f64, f64)>; 2] = [None, None];
    for other in model
        .walls_on_level(w.level)
        .filter(|o| o.id != id && !o.is_curved())
    {
        let Some(ou) = (other.end - other.start).normalized() else {
            continue;
        };
        if u.x * ou.y - u.y * ou.x > 0.0175 || u.x * ou.y - u.y * ou.x < -0.0175 {
            continue;
        }
        let Some(ot) = model.wall_type(other.wall_type) else {
            continue;
        };
        let d = (other.start - w.start).dot(n);
        if d.abs() > MAX_REACH {
            continue;
        }
        let (a, b) = ((other.start - w.start).dot(u), (other.end - w.start).dot(u));
        let (lo, hi) = (a.min(b).max(0.0), a.max(b).min(length));
        if hi - lo < 0.05 {
            continue;
        }
        let (oe, oi) = other.face_offsets(ot.thickness());
        let along = other.end - other.start;
        let flip = if along.normalized().map_or(1.0, |x| x.perp_left().dot(n)) < 0.0 {
            -1.0
        } else {
            1.0
        };
        let faces = [d + oe * flip, d + oi * flip];
        let (k, mine, theirs) = if d > 0.0 {
            (0, ext, faces[0].min(faces[1]))
        } else {
            (1, int, faces[0].max(faces[1]))
        };
        let gap = (theirs - mine) * if d > 0.0 { 1.0 } else { -1.0 };
        if gap <= EPS {
            continue;
        }
        if best[k].is_none_or(|(g, _)| gap < (g - mine).abs()) {
            // A third of the way along the overlap, clear of the length
            // label, which sits in the middle of the wall.
            best[k] = Some((theirs, lo + (hi - lo) / 3.0));
        }
    }
    for (k, found) in best.iter().enumerate() {
        let slot = Slot::WallSide(id, k as u8);
        let anchor = anchors.get(&slot).copied().unwrap_or_default();
        // The reference: a point moved by hand (its offset across the
        // wall, and where it is along it), or the nearest parallel wall.
        let (theirs, mid) = match (anchor.reference, found) {
            (Some(p), _) => (
                (p - w.start).dot(n),
                (p - w.start).dot(u).clamp(0.0, length),
            ),
            (None, Some((theirs, mid))) => (*theirs, *mid),
            (None, None) => continue,
        };
        let mine = anchor.own.unwrap_or(if k == 0 { ext } else { int });
        let base = w.start + u * mid;
        let towards = if theirs >= mine { n } else { -n };
        let value = (theirs - mine).abs();
        if value <= EPS {
            continue;
        }
        v.push(TempDim {
            a: base + n * theirs,
            b: base + n * mine,
            side: Point2::new(0.0, 0.0),
            value,
            target: Target::WallOffset { wall: id, towards },
            slot,
        });
    }
    v
}

/// The drag that makes `dim` measure `value`, with the selection it acts
/// on. `None` if the value is not a positive number.
pub fn edit(model: &Model, dim: &TempDim, value: f64) -> Option<(BTreeSet<Element>, Drag)> {
    if !(value.is_finite() && value > 0.0) {
        return None;
    }
    let change = value - dim.value;
    match dim.target {
        Target::Opening { opening, sign } => {
            let o = model.opening(opening)?;
            let w = model.wall(o.wall)?;
            let u = (w.end - w.start).normalized()?;
            let from = w.start + u * o.along;
            Some((
                [Element::Opening(opening)].into_iter().collect(),
                Drag {
                    grip: Grip::Body,
                    from,
                    to: from - u * (sign * change),
                    variant: false,
                },
            ))
        }
        Target::WallOffset { wall, towards } => {
            let w = model.wall(wall)?;
            Some((
                [Element::Wall(wall)].into_iter().collect(),
                Drag {
                    grip: Grip::Body,
                    from: w.start,
                    to: w.start - towards * change,
                    variant: false,
                },
            ))
        }
        Target::WallLength { wall, from } => {
            let w = model.wall(wall)?;
            let u = (w.end - w.start).normalized()?;
            Some((
                [Element::Wall(wall)].into_iter().collect(),
                Drag {
                    grip: Grip::WallEnd { wall, start: false },
                    from: w.end,
                    to: w.start + u * (from + value),
                    variant: false,
                },
            ))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pick::tests::room;

    fn close(a: f64, b: f64) -> bool {
        (a - b).abs() < 1e-6
    }

    /// The door of the test room: 3 m from the start of the south wall
    /// (drawn east → west, x = 6 → 0), so its centre is at x = 3.
    #[test]
    fn door_jambs_to_the_inner_corners() {
        let (m, _, _) = room();
        let door = m.openings().next().unwrap();
        let width = m.opening_type(door.opening_type).unwrap().width;
        let sel = [Element::Opening(door.id)].into_iter().collect();
        let dims = temp_dims(&m, &sel);
        assert_eq!(dims.len(), 2, "{dims:?}");
        // Inner faces of the east and west walls at x = 5.735 and 0.265.
        let expect = 3.0 - width / 2.0 - 0.265;
        for d in &dims {
            assert!(close(d.value, expect), "{} ≠ {expect}", d.value);
            // On the interior face of the south wall (y = 0.265), drawn
            // towards the room (north).
            assert!(close(d.a.y, 0.265) && close(d.b.y, 0.265), "{d:?}");
            assert!(close(d.side.y, 1.0));
        }
    }

    #[test]
    fn typing_a_door_dimension_slides_it() {
        let (mut m, _, _) = room();
        let door = m.openings().next().unwrap().clone();
        let sel = [Element::Opening(door.id)].into_iter().collect();
        for dim in temp_dims(&m.clone(), &sel) {
            let mut m2 = m.clone();
            let (s, drag) = edit(&m2, &dim, 0.5).unwrap();
            crate::drag::apply(&mut m2, &s, &drag).unwrap();
            let again = temp_dims(&m2, &sel);
            // The edited one now says 500 mm exactly.
            assert!(
                again
                    .iter()
                    .any(|d| d.target == dim.target && close(d.value, 0.5)),
                "{again:?}"
            );
        }
        // Too far: refused by the wall, nothing changes.
        let dim = temp_dims(&m, &sel)[0];
        let (s, drag) = edit(&m, &dim, 9.0).unwrap();
        assert!(crate::drag::apply(&mut m, &s, &drag).is_err());
        assert!(edit(&m, &dim, 0.0).is_none());
    }

    #[test]
    fn wall_length_and_clear_distance_to_the_parallel_wall() {
        let (m, lvl, _) = room();
        // The west wall: (0,0) → (0,4), exterior face on the axis (left,
        // −x). Its parallel is the east wall at x = 6.
        let west = m.walls_on_level(lvl).next().unwrap().id;
        let sel = [Element::Wall(west)].into_iter().collect();
        let dims = temp_dims(&m, &sel);
        let length = dims
            .iter()
            .find(|d| matches!(d.target, Target::WallLength { .. }))
            .unwrap();
        assert!(close(length.value, 4.0));
        let gap = dims
            .iter()
            .find(|d| matches!(d.target, Target::WallOffset { .. }))
            .expect("the east wall");
        // Interior faces: x = 0.265 and x = 5.735.
        assert!(close(gap.value, 6.0 - 2.0 * 0.265), "{}", gap.value);
        // From the reference (the east wall's face) to the wall's own face.
        assert!(close(gap.b.x, 0.265) && close(gap.a.x, 5.735), "{gap:?}");

        // 5000 mm clear: the west wall moves 47 cm east, the north and
        // south walls stretch with it.
        let mut m2 = m.clone();
        let (s, drag) = edit(&m2, gap, 5.0).unwrap();
        crate::drag::apply(&mut m2, &s, &drag).unwrap();
        let w = m2.wall(west).unwrap();
        assert!(close(w.start.x, 0.47), "{:?}", w.start);
        let after = temp_dims(&m2, &sel);
        assert!(after
            .iter()
            .any(|d| matches!(d.target, Target::WallOffset { .. }) && close(d.value, 5.0)));
        // Length 3500: the end moves, the north wall follows the corner.
        let mut m3 = m.clone();
        let (s, drag) = edit(&m3, length, 3.5).unwrap();
        crate::drag::apply(&mut m3, &s, &drag).unwrap();
        assert!(close(m3.wall(west).unwrap().length(), 3.5));
    }

    /// Witness ends dragged by hand: the door measured from its centre to
    /// the project origin's x; the west wall's clear distance from its
    /// axis to a point 2 m east.
    #[test]
    fn witness_ends_moved_by_hand() {
        let (m, lvl, _) = room();
        let door = m.openings().next().unwrap().clone();
        let sel: BTreeSet<Element> = [Element::Opening(door.id)].into_iter().collect();
        let dims = temp_dims(&m, &sel);
        // The one on the west side (the wall runs east → west).
        let west_side = dims
            .iter()
            .find(|d| d.a.x < d.b.x)
            .copied()
            .expect("a dimension towards the west");
        let mut anchors = Anchors::new();
        anchors.insert(
            west_side.slot,
            Anchor {
                reference: Some(Point2::new(0.0, -3.0)),
                own: own_choice(&m, west_side.slot, Point2::new(3.01, 0.2)),
            },
        );
        let moved = temp_dims_anchored(&m, &sel, &anchors);
        let d = moved.iter().find(|d| d.slot == west_side.slot).unwrap();
        // From x = 0 to the door's centre at x = 3.
        assert!(close(d.value, 3.0), "{d:?}");
        // Typing 2500 puts the door's centre 2.5 m from x = 0.
        let mut m2 = m.clone();
        let (s, drag) = edit(&m2, d, 2.5).unwrap();
        crate::drag::apply(&mut m2, &s, &drag).unwrap();
        let again = temp_dims_anchored(&m2, &sel, &anchors);
        let d2 = again.iter().find(|x| x.slot == west_side.slot).unwrap();
        assert!(close(d2.value, 2.5), "{d2:?}");

        let west = m.walls_on_level(lvl).next().unwrap().id;
        let wsel: BTreeSet<Element> = [Element::Wall(west)].into_iter().collect();
        let side = temp_dims(&m, &wsel)
            .into_iter()
            .find(|d| matches!(d.slot, Slot::WallSide(..)))
            .unwrap();
        let mut anchors = Anchors::new();
        anchors.insert(
            side.slot,
            Anchor {
                reference: Some(Point2::new(2.0, 1.0)),
                // The axis of the 26.5 cm wall drawn by its exterior face.
                own: own_choice(&m, side.slot, Point2::new(0.14, 1.0)),
            },
        );
        let d = temp_dims_anchored(&m, &wsel, &anchors)
            .into_iter()
            .find(|d| d.slot == side.slot)
            .unwrap();
        assert!(close(d.value, 2.0 - 0.1325), "{d:?}");
        let mut m3 = m.clone();
        let (s, drag) = edit(&m3, &d, 1.0).unwrap();
        crate::drag::apply(&mut m3, &s, &drag).unwrap();
        let w = m3.wall(west).unwrap();
        // The axis is now 1 m from x = 2.
        assert!(close(w.start.x + 0.1325, 1.0), "{:?}", w.start);
    }

    #[test]
    fn only_for_one_wall_or_opening() {
        let (m, lvl, _) = room();
        let all: BTreeSet<Element> = m.walls_on_level(lvl).map(|w| Element::Wall(w.id)).collect();
        assert!(temp_dims(&m, &all).is_empty());
        let slab = m.slabs().next().unwrap().id;
        assert!(temp_dims(&m, &[Element::Slab(slab)].into_iter().collect()).is_empty());
    }
}
