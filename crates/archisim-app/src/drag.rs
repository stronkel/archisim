//! Moving, copying and stretching the selection with the mouse, without
//! any user interface: which handle is under the cursor, and what dragging
//! it from one point to another does to the model.
//!
//! As in ArchiCAD, only selected elements can be dragged; dragging anywhere
//! else draws a selection window. Every drag is one undoable step.
//!
//! - **Body** (anywhere on a selected element): moves the whole selection.
//!   Walls joined to a moved wall but not selected stretch to keep the
//!   corner closed (as Revit does). A door or window selected without its
//!   wall slides along the wall. With Ctrl the selection is copied instead,
//!   doors and windows of copied walls included.
//! - **Wall end**: stretches the wall, and every wall joined at that corner
//!   with it, so the corner stays closed. With Ctrl only this wall moves
//!   (it comes off the corner). Doors and windows stay where they are.
//! - **Bend** (handle at the middle of a curved wall): the wall passes
//!   through the point it is dropped on, its ends staying put; dropped in
//!   line with them, it straightens.
//! - **Slab corner**: moves that corner; dropped on a neighbouring corner
//!   it removes it.
//! - **Slab edge** (handle at its middle): moves the edge parallel to
//!   itself; with Ctrl it adds a new corner there instead.

use crate::geo::Geo;
use std::collections::BTreeSet;

use crate::pick::{contains, distance_to_outline, outline, Element};
use archisim_core::{Model, ModelError, NewOpening, NewSlab, NewWall, Point2, SlabId, WallId};

/// The part of the selection being dragged.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Grip {
    /// The whole selection.
    Body,
    /// One end of a wall's drawn line.
    WallEnd { wall: WallId, start: bool },
    /// The middle of a curved wall's drawn line: bends it.
    WallBend { wall: WallId },
    /// A corner of a slab, by its index in the outline.
    SlabCorner { slab: SlabId, index: usize },
    /// The edge of a slab from corner `index` to the next one.
    SlabEdge { slab: SlabId, index: usize },
    /// One end of a beam.
    BeamEnd {
        beam: archisim_core::BeamId,
        start: bool,
    },
}

/// A handle drawn on the selection.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Hotspot {
    pub grip: Grip,
    pub point: Point2,
}

/// A finished drag: from where to where, and whether Ctrl was held.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Drag {
    pub grip: Grip,
    pub from: Point2,
    pub to: Point2,
    /// Ctrl: copy (body), leave the corner (wall end), add a corner (edge).
    pub variant: bool,
}

/// Handles of the selected walls and slabs.
pub fn hotspots(model: &Model, selection: &BTreeSet<Element>) -> Vec<Hotspot> {
    let mut v = Vec::new();
    for e in selection {
        match e {
            Element::Wall(id) => {
                if let Some(w) = model.wall(*id) {
                    v.push(Hotspot {
                        grip: Grip::WallEnd {
                            wall: *id,
                            start: true,
                        },
                        point: w.start,
                    });
                    v.push(Hotspot {
                        grip: Grip::WallEnd {
                            wall: *id,
                            start: false,
                        },
                        point: w.end,
                    });
                    if w.is_curved() {
                        v.push(Hotspot {
                            grip: Grip::WallBend { wall: *id },
                            point: w.midpoint(),
                        });
                    }
                }
            }
            Element::Slab(id) => {
                if let Some(s) = model.slab(*id) {
                    let n = s.outline.len();
                    for (i, p) in s.outline.iter().enumerate() {
                        v.push(Hotspot {
                            grip: Grip::SlabCorner {
                                slab: *id,
                                index: i,
                            },
                            point: *p,
                        });
                        v.push(Hotspot {
                            grip: Grip::SlabEdge {
                                slab: *id,
                                index: i,
                            },
                            point: (*p + s.outline[(i + 1) % n]) * 0.5,
                        });
                    }
                }
            }
            Element::Beam(id) => {
                if let Some(b) = model.beam(*id) {
                    v.push(Hotspot {
                        grip: Grip::BeamEnd {
                            beam: *id,
                            start: true,
                        },
                        point: b.start,
                    });
                    v.push(Hotspot {
                        grip: Grip::BeamEnd {
                            beam: *id,
                            start: false,
                        },
                        point: b.end,
                    });
                }
            }
            Element::Opening(_) | Element::Dimension(_) | Element::Mark(_) | Element::Column(_) => {
            }
        }
    }
    v
}

/// What dragging from `p` would grab: the nearest handle within
/// `tolerance`, or the selection itself when `p` is on a selected element.
/// Returns the grip and the point it is grabbed by.
pub fn grip_at<G: Geo + ?Sized>(
    geo: &G,
    selection: &BTreeSet<Element>,
    p: Point2,
    tolerance: f64,
) -> Option<(Grip, Point2)> {
    let model = geo.model();
    let spot = hotspots(model, selection)
        .into_iter()
        .map(|h| (h.point.distance(p), h))
        .filter(|(d, _)| *d <= tolerance)
        .min_by(|a, b| a.0.total_cmp(&b.0));
    if let Some((_, h)) = spot {
        return Some((h.grip, h.point));
    }
    let on_selection = selection.iter().any(|e| {
        outline(geo, *e)
            .is_some_and(|poly| contains(&poly, p) || distance_to_outline(&poly, p) <= tolerance)
    });
    on_selection.then_some((Grip::Body, p))
}

/// Walls and slabs whose geometry the drag changes, so snapping ignores
/// them (a point does not snap to itself).
pub fn moving(model: &Model, selection: &BTreeSet<Element>, grip: Grip) -> BTreeSet<Element> {
    match grip {
        Grip::Body => selection.clone(),
        Grip::WallEnd { wall, start } => joined_at(model, wall, start)
            .into_iter()
            .map(|(w, _)| Element::Wall(w))
            .collect(),
        Grip::WallBend { wall } => [Element::Wall(wall)].into_iter().collect(),
        Grip::SlabCorner { slab, .. } | Grip::SlabEdge { slab, .. } => {
            [Element::Slab(slab)].into_iter().collect()
        }
        Grip::BeamEnd { beam, .. } => [Element::Beam(beam)].into_iter().collect(),
    }
}

/// The walls with an end at the same point as the given end of `wall`
/// (itself included), and which of their ends it is.
fn joined_at(model: &Model, wall: WallId, start: bool) -> Vec<(WallId, bool)> {
    let Some(w) = model.wall(wall) else {
        return Vec::new();
    };
    let p = if start { w.start } else { w.end };
    let mut v = vec![(wall, start)];
    for o in model.walls_on_level(w.level) {
        if o.id == wall {
            continue;
        }
        if o.start.approx_eq(p) {
            v.push((o.id, true));
        } else if o.end.approx_eq(p) {
            v.push((o.id, false));
        }
    }
    v
}

/// Name of the step in the Edit menu.
pub fn label(model: &Model, drag: &Drag) -> &'static str {
    match drag.grip {
        Grip::Body if drag.variant => "Copiar",
        Grip::Body => "Mover",
        Grip::WallEnd { .. } => "Estirar muro",
        Grip::WallBend { .. } => "Curvar muro",
        Grip::SlabCorner { slab, index } => {
            if merges_corner(model, slab, index, drag.to) {
                "Quitar esquina de forjado"
            } else {
                "Mover esquina de forjado"
            }
        }
        Grip::SlabEdge { .. } if drag.variant => "Añadir esquina de forjado",
        Grip::SlabEdge { .. } => "Desplazar arista de forjado",
        Grip::BeamEnd { .. } => "Estirar viga",
    }
}

/// Whether moving a slab corner to `to` drops it on one of its neighbours.
fn merges_corner(model: &Model, slab: SlabId, index: usize, to: Point2) -> bool {
    let Some(s) = model.slab(slab) else {
        return false;
    };
    let n = s.outline.len();
    n > 3
        && (s.outline[(index + 1) % n].approx_eq(to)
            || s.outline[(index + n - 1) % n].approx_eq(to))
}

/// Applies a drag to the model. Returns the new selection when it changes
/// (after a copy, the copies are selected, as in Revit).
pub fn apply(
    model: &mut Model,
    selection: &BTreeSet<Element>,
    drag: &Drag,
) -> Result<Option<BTreeSet<Element>>, ModelError> {
    let d = drag.to - drag.from;
    match drag.grip {
        Grip::Body if drag.variant => copy(model, selection, d).map(Some),
        Grip::Body => move_selection(model, selection, d).map(|_| None),
        Grip::WallEnd { wall, start } => {
            let ends = if drag.variant {
                vec![(wall, start)]
            } else {
                joined_at(model, wall, start)
            };
            for (w, at_start) in ends {
                let wall = model.wall(w).ok_or(ModelError::UnknownWall(w))?;
                let (s, e) = if at_start {
                    (drag.to, wall.end)
                } else {
                    (wall.start, drag.to)
                };
                model.stretch_wall(w, s, e)?;
            }
            Ok(None)
        }
        Grip::WallBend { wall } => {
            let w = model.wall(wall).ok_or(ModelError::UnknownWall(wall))?;
            let bulge = archisim_core::Arc::bulge_through(w.start, drag.to, w.end).unwrap_or(0.0);
            model.set_wall_bulge(wall, bulge).map(|_| None)
        }
        Grip::SlabCorner { slab, index } => {
            let merge = merges_corner(model, slab, index, drag.to);
            let s = model.slab(slab).ok_or(ModelError::UnknownSlab(slab))?;
            let mut outline = s.outline.clone();
            if merge {
                outline.remove(index);
            } else {
                outline[index] = drag.to;
            }
            model.set_slab_outline(slab, outline).map(|_| None)
        }
        Grip::BeamEnd { beam, start } => model
            .edit_beam(beam, |b| {
                if start {
                    b.start = drag.to;
                } else {
                    b.end = drag.to;
                }
            })
            .map(|_| None),
        Grip::SlabEdge { slab, index } => {
            let s = model.slab(slab).ok_or(ModelError::UnknownSlab(slab))?;
            let mut outline = s.outline.clone();
            let n = outline.len();
            let (a, b) = (outline[index], outline[(index + 1) % n]);
            if drag.variant {
                outline.insert(index + 1, drag.to);
            } else {
                let normal = (b - a)
                    .normalized()
                    .ok_or(ModelError::InvalidOutline)?
                    .perp_left();
                let shift = normal * d.dot(normal);
                outline[index] = a + shift;
                outline[(index + 1) % n] = b + shift;
            }
            model.set_slab_outline(slab, outline).map(|_| None)
        }
    }
}

/// Moves the selection by `d`.
fn move_selection(
    model: &mut Model,
    selection: &BTreeSet<Element>,
    d: Point2,
) -> Result<(), ModelError> {
    let walls: Vec<WallId> = selection
        .iter()
        .filter_map(|e| match e {
            Element::Wall(id) => Some(*id),
            _ => None,
        })
        .collect();
    let followers = followers(model, &walls, d);
    for id in &walls {
        let w = model.wall(*id).ok_or(ModelError::UnknownWall(*id))?;
        let (s, e) = (w.start + d, w.end + d);
        model.move_wall(*id, s, e)?;
    }
    for (id, s, e) in followers {
        model.stretch_wall(id, s, e)?;
    }
    // Dimensions go with the building elements they are moved with; moved
    // on their own, only their line moves (it is how they are tidied up).
    let only_annotations = selection
        .iter()
        .all(|e| matches!(e, Element::Dimension(_) | Element::Mark(_)));
    for e in selection {
        match e {
            Element::Slab(id) => {
                model.move_slab(*id, d)?;
            }
            Element::Column(id) => {
                model.edit_column(*id, |c| c.position = c.position + d)?;
            }
            Element::Beam(id) => {
                model.edit_beam(*id, |b| {
                    b.start = b.start + d;
                    b.end = b.end + d;
                })?;
            }
            Element::Dimension(id) => {
                let dim = model
                    .dimension(*id)
                    .ok_or(ModelError::UnknownDimension(*id))?;
                if only_annotations {
                    let n = dim.direction.perp_left();
                    let (points, position) = (dim.points.clone(), dim.position + n * d.dot(n));
                    model.set_dimension(*id, points, position)?;
                } else {
                    let points = dim.points.iter().map(|p| *p + d).collect();
                    let position = dim.position + d;
                    model.set_dimension(*id, points, position)?;
                }
            }
            Element::Mark(id) => {
                let p = model
                    .elevation_mark(*id)
                    .ok_or(ModelError::UnknownElevationMark(*id))?
                    .point;
                model.move_elevation_mark(*id, p + d)?;
            }
            _ => {}
        }
    }
    // Doors and windows whose wall stays: they slide along it. The ones in
    // front go first so they never bump into the ones behind.
    let mut alone: Vec<(archisim_core::OpeningId, f64, f64)> = Vec::new();
    for e in selection {
        if let Element::Opening(id) = e {
            let o = model.opening(*id).ok_or(ModelError::UnknownOpening(*id))?;
            if walls.contains(&o.wall) {
                continue;
            }
            let w = model.wall(o.wall).ok_or(ModelError::UnknownWall(o.wall))?;
            let u = (w.end - w.start)
                .normalized()
                .ok_or(ModelError::ZeroLengthWall)?;
            alone.push((*id, o.along + d.dot(u), d.dot(u)));
        }
    }
    let front = |a: &(archisim_core::OpeningId, f64, f64)| a.1 * a.2.signum();
    alone.sort_by(|a, b| front(b).total_cmp(&front(a)));
    for (id, along, _) in alone {
        let sill = model
            .opening(id)
            .ok_or(ModelError::UnknownOpening(id))?
            .sill;
        model.move_opening(id, along, sill)?;
    }
    Ok(())
}

/// Walls that are not moved but touch the moved ones, and where their ends
/// go: an end joined at the end of a moved wall moves with it (the corner
/// stays closed); an end butting against a face of a moved wall slides
/// along its own axis to stay against that face (a partition stays
/// against the wall it meets).
fn followers(model: &Model, moved: &[WallId], d: Point2) -> Vec<(WallId, Point2, Point2)> {
    use archisim_geom::{wall_curve, Curve, Line};
    let mut ends: Vec<Point2> = Vec::new();
    // Faces of the moved walls: the line or circle, and the wall it is a
    // face of (to know whether a point lies on the face itself).
    let mut faces: Vec<(Curve, &archisim_core::Wall)> = Vec::new();
    let mut level = None;
    for id in moved {
        let Some(w) = model.wall(*id) else { continue };
        ends.push(w.start);
        ends.push(w.end);
        level = Some(w.level);
        let Some(t) = model.wall_type(w.wall_type).map(|t| t.thickness()) else {
            continue;
        };
        let (ext, int) = w.face_offsets(t);
        for off in [ext, int] {
            if let Some(c) = wall_curve(w, off) {
                faces.push((c, w));
            }
        }
    }
    let Some(level) = level else {
        return Vec::new();
    };
    let on_face = |c: &Curve, w: &archisim_core::Wall, p: Point2| match (c, w.arc()) {
        (Curve::Circle { centre, radius }, Some(arc)) => {
            let (f, _) = arc.project(p);
            (p.distance(*centre) - radius).abs() < 1e-5 && f > 0.0 && f < 1.0
        }
        (Curve::Line(l), _) => {
            let n = (w.end - w.start)
                .normalized()
                .map_or(Point2::default(), Point2::perp_left);
            let off = (l.p - w.start).dot(n);
            let (a, b) = (w.start + n * off, w.end + n * off);
            crate::pick::nearest_on_segment(a, b, p).distance(p) < 1e-5
        }
        _ => false,
    };
    let follow = |p: Point2, axis: Option<Point2>| -> Option<Point2> {
        if ends.iter().any(|e| e.approx_eq(p)) {
            return Some(p + d);
        }
        let axis = axis?;
        faces.iter().find_map(|(c, w)| {
            if !on_face(c, w, p) {
                return None;
            }
            let moved = match c {
                Curve::Line(l) => Curve::Line(Line::new(l.p + d, l.d)),
                Curve::Circle { centre, radius } => Curve::Circle {
                    centre: *centre + d,
                    radius: *radius,
                },
            };
            Curve::Line(Line::new(p, axis)).intersect_near(&moved, p + d)
        })
    };
    model
        .walls_on_level(level)
        .filter(|w| !moved.contains(&w.id))
        .filter_map(|w| {
            // A curved wall has no axis to slide along: only its joined
            // ends follow.
            let axis = (!w.is_curved()).then(|| w.end - w.start);
            let s = follow(w.start, axis);
            let e = follow(w.end, axis);
            (s.is_some() || e.is_some()).then(|| (w.id, s.unwrap_or(w.start), e.unwrap_or(w.end)))
        })
        .collect()
}

/// Copies the selection by `d` and returns the copies.
fn copy(
    model: &mut Model,
    selection: &BTreeSet<Element>,
    d: Point2,
) -> Result<BTreeSet<Element>, ModelError> {
    let mut copies = BTreeSet::new();
    let copied_walls: Vec<WallId> = selection
        .iter()
        .filter_map(|e| match e {
            Element::Wall(id) => Some(*id),
            _ => None,
        })
        .collect();
    for e in selection {
        match e {
            Element::Dimension(id) => {
                let dim = model
                    .dimension(*id)
                    .ok_or(ModelError::UnknownDimension(*id))?
                    .clone();
                let new = model.add_dimension(archisim_core::NewDimension {
                    level: dim.level,
                    points: dim.points.iter().map(|p| *p + d).collect(),
                    direction: dim.direction,
                    position: dim.position + d,
                })?;
                copies.insert(Element::Dimension(new));
            }
            Element::Mark(id) => {
                let m = model
                    .elevation_mark(*id)
                    .ok_or(ModelError::UnknownElevationMark(*id))?
                    .clone();
                let new = model.add_elevation_mark(m.level, m.point + d)?;
                copies.insert(Element::Mark(new));
            }
            Element::Wall(id) => {
                let w = model.wall(*id).ok_or(ModelError::UnknownWall(*id))?.clone();
                let hosted: Vec<NewOpening> = model
                    .openings_in_wall(*id)
                    .map(|o| NewOpening {
                        opening_type: o.opening_type,
                        wall: *id,
                        along: o.along,
                        sill: o.sill,
                        opens_to: o.opens_to,
                        hand: o.hand,
                        setback: o.setback,
                    })
                    .collect();
                let new = model.add_wall(NewWall {
                    wall_type: w.wall_type,
                    level: w.level,
                    start: w.start + d,
                    end: w.end + d,
                    reference: w.reference,
                    height: w.height,
                    base_offset: w.base_offset,
                    bulge: w.bulge,
                    top: w.top,
                })?;
                copies.insert(Element::Wall(new));
                for mut spec in hosted {
                    spec.wall = new;
                    copies.insert(Element::Opening(model.add_opening(spec)?));
                }
            }
            Element::Slab(id) => {
                let s = model.slab(*id).ok_or(ModelError::UnknownSlab(*id))?.clone();
                let new = model.add_slab(NewSlab {
                    slab_type: s.slab_type,
                    level: s.level,
                    outline: s.outline.iter().map(|p| *p + d).collect(),
                    top_offset: s.top_offset,
                })?;
                let holes = s
                    .holes
                    .iter()
                    .map(|h| h.iter().map(|p| *p + d).collect())
                    .collect();
                model.set_slab_holes(new, holes)?;
                copies.insert(Element::Slab(new));
            }
            Element::Column(id) => {
                let c = model
                    .column(*id)
                    .ok_or(ModelError::UnknownColumn(*id))?
                    .clone();
                let new = model.add_column(archisim_core::NewColumn {
                    member_type: c.member_type,
                    level: c.level,
                    position: c.position + d,
                    rotation: c.rotation,
                    base_offset: c.base_offset,
                    top: c.top,
                    height: c.height,
                })?;
                copies.insert(Element::Column(new));
            }
            Element::Beam(id) => {
                let b = model.beam(*id).ok_or(ModelError::UnknownBeam(*id))?.clone();
                let new = model.add_beam(archisim_core::NewBeam {
                    member_type: b.member_type,
                    level: b.level,
                    start: b.start + d,
                    end: b.end + d,
                    top_offset: b.top_offset,
                })?;
                copies.insert(Element::Beam(new));
            }
            Element::Opening(id) => {
                let o = model
                    .opening(*id)
                    .ok_or(ModelError::UnknownOpening(*id))?
                    .clone();
                if copied_walls.contains(&o.wall) {
                    continue;
                }
                let w = model.wall(o.wall).ok_or(ModelError::UnknownWall(o.wall))?;
                let u = (w.end - w.start)
                    .normalized()
                    .ok_or(ModelError::ZeroLengthWall)?;
                let new = model.add_opening(NewOpening {
                    opening_type: o.opening_type,
                    wall: o.wall,
                    along: o.along + d.dot(u),
                    sill: o.sill,
                    opens_to: o.opens_to,
                    hand: o.hand,
                    setback: o.setback,
                })?;
                copies.insert(Element::Opening(new));
            }
        }
    }
    Ok(copies)
}

/// Outlines that a drag would change or create, to draw them as it
/// follows the cursor: the model after the drag compared with before.
pub fn preview<G: Geo + ?Sized>(
    geo: &G,
    selection: &BTreeSet<Element>,
    drag: &Drag,
) -> Result<Vec<Vec<Point2>>, ModelError> {
    let model = geo.model();
    let mut after = model.clone();
    apply(&mut after, selection, drag)?;
    let Some(level) = level_of(model, selection, drag.grip) else {
        return Ok(Vec::new());
    };
    let later = crate::geo::Indexed::new(&after);
    // Only what is near the selection can change, where it was and where
    // it goes (walls joined to it stretch), and what the drag creates.
    let all = crate::pick::elements_on_level(&after, level);
    let created = all.iter().filter(|e| !exists(model, **e));
    let mut boxes: Vec<(Point2, Point2)> = Vec::new();
    for e in selection {
        boxes.extend(outline(geo, *e).and_then(|p| bounds(&p)));
        boxes.extend(outline(&later, *e).and_then(|p| bounds(&p)));
    }
    for e in created {
        boxes.extend(outline(&later, *e).and_then(|p| bounds(&p)));
    }
    let margin = Point2::new(1.0, 1.0);
    let mut near: BTreeSet<WallId> = BTreeSet::new();
    for (lo, hi) in boxes {
        near.extend(
            later
                .0
                .in_box(level, lo - margin, hi + margin)
                .iter()
                .map(|w| w.id),
        );
    }
    let mut out = Vec::new();
    for e in all {
        let candidate = match e {
            Element::Wall(w) => near.contains(&w),
            Element::Opening(o) => after.opening(o).is_some_and(|o| near.contains(&o.wall)),
            _ => true,
        };
        if !candidate {
            continue;
        }
        let new = outline(&later, e);
        let old = if exists(model, e) {
            outline(geo, e)
        } else {
            None
        };
        if let Some(poly) = new {
            if old.as_ref() != Some(&poly) {
                out.push(poly);
            }
        }
    }
    Ok(out)
}

/// Whether the element is in the model.
fn exists(model: &Model, e: Element) -> bool {
    match e {
        Element::Wall(id) => model.wall(id).is_some(),
        Element::Opening(id) => model.opening(id).is_some(),
        Element::Slab(id) => model.slab(id).is_some(),
        Element::Dimension(id) => model.dimension(id).is_some(),
        Element::Mark(id) => model.elevation_mark(id).is_some(),
        Element::Column(id) => model.column(id).is_some(),
        Element::Beam(id) => model.beam(id).is_some(),
    }
}

/// The box around a polygon.
fn bounds(poly: &[Point2]) -> Option<(Point2, Point2)> {
    let first = *poly.first()?;
    Some(poly.iter().fold((first, first), |(lo, hi), p| {
        (
            Point2::new(lo.x.min(p.x), lo.y.min(p.y)),
            Point2::new(hi.x.max(p.x), hi.y.max(p.y)),
        )
    }))
}

fn level_of(
    model: &Model,
    selection: &BTreeSet<Element>,
    grip: Grip,
) -> Option<archisim_core::LevelId> {
    let wall_level = |id: WallId| model.wall(id).map(|w| w.level);
    match grip {
        Grip::WallEnd { wall, .. } | Grip::WallBend { wall } => wall_level(wall),
        Grip::SlabCorner { slab, .. } | Grip::SlabEdge { slab, .. } => {
            model.slab(slab).map(|s| s.level)
        }
        Grip::BeamEnd { beam, .. } => model.beam(beam).map(|b| b.level),
        Grip::Body => selection.iter().find_map(|e| match e {
            Element::Wall(id) => wall_level(*id),
            Element::Slab(id) => model.slab(*id).map(|s| s.level),
            Element::Opening(id) => model.opening(*id).and_then(|o| wall_level(o.wall)),
            Element::Dimension(id) => model.dimension(*id).map(|d| d.level),
            Element::Mark(id) => model.elevation_mark(*id).map(|m| m.level),
            Element::Column(id) => model.column(*id).map(|c| c.level),
            Element::Beam(id) => model.beam(*id).map(|b| b.level),
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pick::tests::room;

    fn sel(items: &[Element]) -> BTreeSet<Element> {
        items.iter().copied().collect()
    }

    fn p(x: f64, y: f64) -> Point2 {
        Point2::new(x, y)
    }

    /// Walls of the test room in drawing order: west, north, east, south.
    fn walls(m: &Model) -> Vec<WallId> {
        m.walls().map(|w| w.id).collect()
    }

    #[test]
    fn handles_and_grips() {
        let (m, _, _) = room();
        let w = walls(&m);
        let slab = m.slabs().next().unwrap().id;
        let s = sel(&[Element::Wall(w[0]), Element::Slab(slab)]);
        // 2 wall ends + 4 corners + 4 edge middles.
        assert_eq!(hotspots(&m, &s).len(), 10);
        // On the end of the west wall at (0, 4).
        assert_eq!(
            grip_at(&m, &s, p(0.02, 3.99), 0.1),
            Some((
                Grip::WallEnd {
                    wall: w[0],
                    start: false
                },
                p(0.0, 4.0)
            ))
        );
        // On the middle of the slab's south edge.
        let (g, at) = grip_at(&m, &s, p(3.0, 0.05), 0.1).unwrap();
        assert!(matches!(g, Grip::SlabEdge { .. }));
        assert_eq!(at, p(3.0, 0.0));
        // In the middle of the selected wall: the whole selection.
        assert_eq!(
            grip_at(&m, &s, p(0.1, 1.0), 0.05),
            Some((Grip::Body, p(0.1, 1.0)))
        );
        // Nothing selected there.
        assert_eq!(grip_at(&m, &sel(&[]), p(0.1, 1.0), 0.05), None);
    }

    #[test]
    fn moving_a_wall_drags_the_joined_corners() {
        let (mut m, _, _) = room();
        let w = walls(&m);
        let guid = m.wall(w[1]).unwrap().guid.clone();
        // The north wall 1 m north: west and east walls stretch with it.
        let drag = Drag {
            grip: Grip::Body,
            from: p(3.0, 4.0),
            to: p(3.0, 5.0),
            variant: false,
        };
        assert_eq!(label(&m, &drag), "Mover");
        apply(&mut m, &sel(&[Element::Wall(w[1])]), &drag).unwrap();
        assert_eq!(m.wall(w[1]).unwrap().start, p(0.0, 5.0));
        assert_eq!(m.wall(w[1]).unwrap().guid, guid);
        assert_eq!(m.wall(w[0]).unwrap().end, p(0.0, 5.0));
        assert_eq!(m.wall(w[2]).unwrap().start, p(6.0, 5.0));
        // The south wall and its door did not move.
        let door = m.openings().next().unwrap();
        assert_eq!(door.along, 3.0);
        assert_eq!(m.wall(w[3]).unwrap().start, p(6.0, 0.0));
    }

    #[test]
    fn partitions_butting_against_a_moved_wall_follow_it() {
        let (mut m, lvl, lib) = room();
        let w = walls(&m);
        // A partition from the inner face of the west wall (x = 0.265) to
        // the east one (x = 5.735), and another from the south wall's inner
        // face up to the first one.
        let add = |m: &mut Model, a: Point2, b: Point2| {
            m.add_wall(NewWall {
                wall_type: lib.partition,
                level: lvl,
                start: a,
                end: b,
                reference: archisim_core::ReferenceLine::Center,
                height: 2.5,
                base_offset: 0.0,
                bulge: 0.0,
                top: archisim_core::WallTop::Unconnected,
            })
            .unwrap()
        };
        let across = add(&mut m, p(0.265, 2.0), p(5.735, 2.0));
        let below = add(&mut m, p(3.0, 0.265), p(3.0, 1.95));
        // Move the east façade 1 m east: the partition stretches to its new
        // inner face; the one below does not touch it and stays.
        let drag = Drag {
            grip: Grip::Body,
            from: p(6.0, 2.0),
            to: p(7.0, 2.0),
            variant: false,
        };
        apply(&mut m, &sel(&[Element::Wall(w[2])]), &drag).unwrap();
        assert!(m.wall(across).unwrap().end.approx_eq(p(6.735, 2.0)));
        assert_eq!(m.wall(across).unwrap().start, p(0.265, 2.0));
        assert_eq!(m.wall(below).unwrap().end, p(3.0, 1.95));
        // Move the partition across 0.5 m north: the one below follows its
        // south face.
        let drag = Drag {
            grip: Grip::Body,
            from: p(3.0, 2.0),
            to: p(3.0, 2.5),
            variant: false,
        };
        apply(&mut m, &sel(&[Element::Wall(across)]), &drag).unwrap();
        assert!(m.wall(below).unwrap().end.approx_eq(p(3.0, 2.45)));
        assert_eq!(m.wall(below).unwrap().start, p(3.0, 0.265));
    }

    #[test]
    fn dimensions_move_their_line_alone_or_go_with_the_walls() {
        let (mut m, lvl, _) = room();
        let w = walls(&m);
        let d = m
            .add_dimension(archisim_core::NewDimension {
                level: lvl,
                points: vec![p(0.0, 0.0), p(6.0, 0.0)],
                direction: p(1.0, 0.0),
                position: p(0.0, -1.0),
            })
            .unwrap();
        let mark = m.add_elevation_mark(lvl, p(3.0, 2.0)).unwrap();
        // Alone: only the line moves, across the direction.
        let drag = Drag {
            grip: Grip::Body,
            from: p(3.0, -1.0),
            to: p(4.0, -1.5),
            variant: false,
        };
        apply(&mut m, &sel(&[Element::Dimension(d)]), &drag).unwrap();
        let dim = m.dimension(d).unwrap();
        assert_eq!(dim.points[1], p(6.0, 0.0));
        assert!((dim.line_offset() + 1.5).abs() < 1e-12);
        // With the west wall: everything moves.
        let drag = Drag {
            grip: Grip::Body,
            from: p(0.0, 0.0),
            to: p(-1.0, 0.0),
            variant: false,
        };
        let s = sel(&[
            Element::Wall(w[0]),
            Element::Dimension(d),
            Element::Mark(mark),
        ]);
        apply(&mut m, &s, &drag).unwrap();
        assert_eq!(m.dimension(d).unwrap().points[0], p(-1.0, 0.0));
        assert_eq!(m.elevation_mark(mark).unwrap().point, p(2.0, 2.0));
        // Copied.
        let copy = Drag {
            variant: true,
            ..drag
        };
        let copies = apply(&mut m, &sel(&[Element::Dimension(d)]), &copy)
            .unwrap()
            .unwrap();
        assert_eq!(copies.len(), 1);
        assert_eq!(m.dimensions().count(), 2);
    }

    #[test]
    fn moving_the_south_wall_takes_its_door() {
        let (mut m, _, _) = room();
        let w = walls(&m);
        let drag = Drag {
            grip: Grip::Body,
            from: p(3.0, 0.0),
            to: p(3.0, -1.0),
            variant: false,
        };
        apply(&mut m, &sel(&[Element::Wall(w[3])]), &drag).unwrap();
        // Same distance from the start: it moved with the wall.
        assert_eq!(m.openings().next().unwrap().along, 3.0);
        assert_eq!(m.wall(w[0]).unwrap().start, p(0.0, -1.0));
    }

    #[test]
    fn a_door_alone_slides_along_its_wall() {
        let (mut m, _, _) = room();
        let door = m.openings().next().unwrap().id;
        // The south wall runs east → west: 1 m west is 1 m further along.
        let drag = Drag {
            grip: Grip::Body,
            from: p(3.0, 0.1),
            to: p(2.0, 0.6),
            variant: false,
        };
        apply(&mut m, &sel(&[Element::Opening(door)]), &drag).unwrap();
        assert!((m.opening(door).unwrap().along - 4.0).abs() < 1e-12);
        // Too far: it would leave the wall, and nothing changes.
        let too_far = Drag {
            to: p(-3.0, 0.1),
            ..drag
        };
        assert_eq!(
            apply(&mut m, &sel(&[Element::Opening(door)]), &too_far),
            Err(ModelError::OpeningOutsideWall)
        );
    }

    #[test]
    fn copying_walls_takes_their_doors_and_selects_the_copies() {
        let (mut m, _, _) = room();
        let w = walls(&m);
        let before = (m.walls().count(), m.openings().count());
        let drag = Drag {
            grip: Grip::Body,
            from: p(0.0, 0.0),
            to: p(10.0, 0.0),
            variant: true,
        };
        assert_eq!(label(&m, &drag), "Copiar");
        let all = sel(&w.iter().map(|id| Element::Wall(*id)).collect::<Vec<_>>());
        let copies = apply(&mut m, &all, &drag).unwrap().unwrap();
        assert_eq!(m.walls().count(), before.0 + 4);
        assert_eq!(m.openings().count(), before.1 + 1);
        assert_eq!(copies.len(), 5);
        // The originals did not move; the copies are joined among themselves.
        assert_eq!(m.wall(w[0]).unwrap().start, p(0.0, 0.0));
        for c in &copies {
            if let Element::Wall(id) = c {
                let g = archisim_geom::wall_geometry(&m, *id).unwrap();
                assert!(matches!(g.start_cut, archisim_geom::EndCut::Along(_)));
            }
        }
    }

    #[test]
    fn stretching_a_corner_moves_both_walls_unless_ctrl() {
        let (mut m, _, _) = room();
        let w = walls(&m);
        // The north-east corner (6, 4) pulled to (7, 5).
        let drag = Drag {
            grip: Grip::WallEnd {
                wall: w[1],
                start: false,
            },
            from: p(6.0, 4.0),
            to: p(7.0, 5.0),
            variant: false,
        };
        apply(&mut m, &sel(&[Element::Wall(w[1])]), &drag).unwrap();
        assert_eq!(m.wall(w[1]).unwrap().end, p(7.0, 5.0));
        assert_eq!(m.wall(w[2]).unwrap().start, p(7.0, 5.0));
        // With Ctrl only the selected wall's end moves.
        let detach = Drag {
            from: p(7.0, 5.0),
            to: p(7.0, 6.0),
            variant: true,
            ..drag
        };
        apply(&mut m, &sel(&[Element::Wall(w[1])]), &detach).unwrap();
        assert_eq!(m.wall(w[1]).unwrap().end, p(7.0, 6.0));
        assert_eq!(m.wall(w[2]).unwrap().start, p(7.0, 5.0));
    }

    #[test]
    fn stretching_the_south_wall_keeps_the_door_in_place() {
        let (mut m, _, _) = room();
        let w = walls(&m);
        let door = m.openings().next().unwrap().id;
        // South wall: (6, 0) → (0, 0); door 3 m from the start, at x = 3.
        // Pull the south-east corner 2 m east.
        let drag = Drag {
            grip: Grip::WallEnd {
                wall: w[3],
                start: true,
            },
            from: p(6.0, 0.0),
            to: p(8.0, 0.0),
            variant: false,
        };
        apply(&mut m, &sel(&[Element::Wall(w[3])]), &drag).unwrap();
        // Still at x = 3, now 5 m from the new start.
        assert!((m.opening(door).unwrap().along - 5.0).abs() < 1e-12);
        // The east wall now leans from (8, 0) to (6, 4).
        assert_eq!(m.wall(w[2]).unwrap().end, p(8.0, 0.0));
    }

    #[test]
    fn slab_corners_and_edges() {
        let (mut m, _, _) = room();
        let slab = m.slabs().next().unwrap().id;
        let s = sel(&[Element::Slab(slab)]);
        let corner = |m: &Model, i| m.slab(slab).unwrap().outline[i];
        let index = |m: &Model, q: Point2| {
            m.slab(slab)
                .unwrap()
                .outline
                .iter()
                .position(|c| c.approx_eq(q))
                .unwrap()
        };
        // Move the north-east corner.
        let ne = index(&m, p(6.0, 4.0));
        let d = Drag {
            grip: Grip::SlabCorner { slab, index: ne },
            from: p(6.0, 4.0),
            to: p(7.0, 4.0),
            variant: false,
        };
        assert_eq!(label(&m, &d), "Mover esquina de forjado");
        apply(&mut m, &s, &d).unwrap();
        assert_eq!(corner(&m, ne), p(7.0, 4.0));
        // Offset the south edge 0.5 m south, whatever the sideways movement
        // of the cursor. Counter-clockwise, it runs from (0, 0) to (6, 0).
        let sw = index(&m, p(0.0, 0.0));
        let e = Drag {
            grip: Grip::SlabEdge { slab, index: sw },
            from: p(3.0, 0.0),
            to: p(3.7, -0.5),
            variant: false,
        };
        apply(&mut m, &s, &e).unwrap();
        assert_eq!(corner(&m, sw), p(0.0, -0.5));
        assert_eq!(corner(&m, (sw + 1) % 4), p(6.0, -0.5));
        // Ctrl on the west edge, from (0, 4) down to (0, -0.5): a new corner.
        let nw = index(&m, p(0.0, 4.0));
        let add = Drag {
            grip: Grip::SlabEdge { slab, index: nw },
            from: p(0.0, 1.75),
            to: p(-1.0, 1.75),
            variant: true,
        };
        assert_eq!(label(&m, &add), "Añadir esquina de forjado");
        apply(&mut m, &s, &add).unwrap();
        assert_eq!(m.slab(slab).unwrap().outline.len(), 5);
        // Dropping that corner on its neighbour removes it.
        let new_index = m
            .slab(slab)
            .unwrap()
            .outline
            .iter()
            .position(|q| *q == p(-1.0, 1.75))
            .unwrap();
        let next = m.slab(slab).unwrap().outline[(new_index + 1) % 5];
        let drop = Drag {
            grip: Grip::SlabCorner {
                slab,
                index: new_index,
            },
            from: p(-1.0, 1.75),
            to: next,
            variant: false,
        };
        assert_eq!(label(&m, &drop), "Quitar esquina de forjado");
        apply(&mut m, &s, &drop).unwrap();
        assert_eq!(m.slab(slab).unwrap().outline.len(), 4);
        // A corner that would make the outline cross itself is refused.
        let cross = Drag {
            grip: Grip::SlabCorner { slab, index: 0 },
            from: corner(&m, 0),
            to: p(3.0, 8.0),
            variant: false,
        };
        assert_eq!(apply(&mut m, &s, &cross), Err(ModelError::InvalidOutline));
    }

    #[test]
    fn preview_shows_only_what_changes() {
        let (m, _, _) = room();
        let w = walls(&m);
        let drag = Drag {
            grip: Grip::Body,
            from: p(3.0, 4.0),
            to: p(3.0, 5.0),
            variant: false,
        };
        // North wall plus the west and east walls that follow it.
        let ghost = preview(&m, &sel(&[Element::Wall(w[1])]), &drag).unwrap();
        assert_eq!(ghost.len(), 3);
        // The model itself is untouched.
        assert_eq!(m.wall(w[1]).unwrap().start, p(0.0, 4.0));
    }
}
