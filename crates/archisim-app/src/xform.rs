//! Moving, turning, scaling and mirroring a selection of any kind of
//! element (AutoCAD's DESPLAZA, COPIA, GIRA, ESCALA and SIMETRIA work on
//! walls, slabs and annotations too, not only on 2D drafting).
//!
//! - Drafting elements: their geometry (see [`archisim_draft::transform_shape`]).
//! - Walls: their drawn line, with their doors and windows; mirrored, a
//!   wall is reversed so its exterior face is still the mirrored one, and
//!   its doors and windows change hand.
//! - Slabs: outline and holes.
//! - Dimensions and level marks: their points.
//! - A door or window selected without its wall stays (it lives in it).

use std::collections::BTreeSet;

use archisim_core::{
    Hand, Model, ModelError, NewDimension, NewDraft, NewOpening, NewSlab, NewWall, OpeningId,
};
use archisim_draft::{transform_shape, Transform};

use crate::pick::Element;

fn flip(h: Hand) -> Hand {
    match h {
        Hand::Left => Hand::Right,
        Hand::Right => Hand::Left,
    }
}

/// Applies `t` to the selection, or to copies of it (returned). Every
/// element keeps its identity and GUID when it is not copied.
pub fn transform(
    model: &mut Model,
    selection: &BTreeSet<Element>,
    t: &Transform,
    copy: bool,
) -> Result<BTreeSet<Element>, ModelError> {
    let mirror = t.is_mirror();
    let f = t.factor();
    let mut out = BTreeSet::new();
    for e in selection {
        match *e {
            Element::Draft(id) => {
                let d = model.draft(id).ok_or(ModelError::UnknownDraft(id))?.clone();
                let shape = transform_shape(&d.shape, t);
                if copy {
                    let new = model.add_draft(NewDraft {
                        view: d.view,
                        layer: d.layer,
                        shape,
                        color: d.color,
                        weight: d.weight,
                        line: d.line,
                    })?;
                    out.insert(Element::Draft(new));
                } else {
                    model.set_draft_shape(id, shape)?;
                    out.insert(*e);
                }
            }
            Element::Wall(id) => {
                let w = model.wall(id).ok_or(ModelError::UnknownWall(id))?.clone();
                let length = w.length();
                let (start, end) = if mirror {
                    (t.apply(w.end), t.apply(w.start))
                } else {
                    (t.apply(w.start), t.apply(w.end))
                };
                let hosted: Vec<(OpeningId, f64, Hand)> = model
                    .openings_in_wall(id)
                    .map(|o| {
                        if mirror {
                            (o.id, (length - o.along) * f, flip(o.hand))
                        } else {
                            (o.id, o.along * f, o.hand)
                        }
                    })
                    .collect();
                if copy {
                    let new = model.add_wall(NewWall {
                        wall_type: w.wall_type,
                        level: w.level,
                        start,
                        end,
                        reference: w.reference,
                        height: w.height,
                        base_offset: w.base_offset,
                        bulge: w.bulge,
                        top: w.top,
                    })?;
                    out.insert(Element::Wall(new));
                    for (oid, along, hand) in hosted {
                        let o = model
                            .opening(oid)
                            .ok_or(ModelError::UnknownOpening(oid))?
                            .clone();
                        let n = model.add_opening(NewOpening {
                            opening_type: o.opening_type,
                            wall: new,
                            along,
                            sill: o.sill,
                            opens_to: o.opens_to,
                            hand,
                            setback: o.setback,
                        })?;
                        out.insert(Element::Opening(n));
                    }
                } else {
                    model.place_wall(id, start, end, w.bulge, &hosted)?;
                    out.insert(*e);
                    for (oid, _, _) in hosted {
                        out.insert(Element::Opening(oid));
                    }
                }
            }
            Element::Slab(id) => {
                let s = model.slab(id).ok_or(ModelError::UnknownSlab(id))?.clone();
                let outline: Vec<_> = s.outline.iter().map(|p| t.apply(*p)).collect();
                let holes: Vec<Vec<_>> = s
                    .holes
                    .iter()
                    .map(|h| h.iter().map(|p| t.apply(*p)).collect())
                    .collect();
                if copy {
                    let new = model.add_slab(NewSlab {
                        slab_type: s.slab_type,
                        level: s.level,
                        outline,
                        top_offset: s.top_offset,
                    })?;
                    model.set_slab_holes(new, holes)?;
                    out.insert(Element::Slab(new));
                } else {
                    // Holes go first out of the way, then come back.
                    model.set_slab_holes(id, Vec::new())?;
                    model.set_slab_outline(id, outline)?;
                    model.set_slab_holes(id, holes)?;
                    out.insert(*e);
                }
            }
            Element::Dimension(id) => {
                let d = model
                    .dimension(id)
                    .ok_or(ModelError::UnknownDimension(id))?
                    .clone();
                let points: Vec<_> = d.points.iter().map(|p| t.apply(*p)).collect();
                let direction = t.linear(d.direction);
                let position = t.apply(d.position);
                if copy {
                    let new = model.add_dimension(NewDimension {
                        level: d.level,
                        points,
                        direction,
                        position,
                    })?;
                    out.insert(Element::Dimension(new));
                } else {
                    model.reshape_dimension(id, points, direction, position)?;
                    out.insert(*e);
                }
            }
            Element::Mark(id) => {
                let m = model
                    .elevation_mark(id)
                    .ok_or(ModelError::UnknownElevationMark(id))?
                    .clone();
                let p = t.apply(m.point);
                if copy {
                    out.insert(Element::Mark(model.add_elevation_mark(m.level, p)?));
                } else {
                    model.move_elevation_mark(id, p)?;
                    out.insert(*e);
                }
            }
            Element::Room(_) | Element::Tag(_) => {
                let walls_too = selection.iter().any(|e| matches!(e, Element::Wall(_)));
                out.insert(transform_one(model, *e, t, copy, walls_too)?);
            }
            Element::Opening(_) => {}
        }
    }
    Ok(out)
}

/// A room or a tag moved, turned or copied (`walls_too`: with walls).
pub fn transform_one(
    model: &mut Model,
    e: Element,
    t: &Transform,
    copy: bool,
    walls_too: bool,
) -> Result<Element, ModelError> {
    match e {
        Element::Room(id) => room(model, id, t, copy, walls_too),
        Element::Tag(id) => {
            let tag = model.tag(id).ok_or(ModelError::UnknownTag(id))?.clone();
            let at = t.apply(tag.at);
            if copy {
                Ok(Element::Tag(model.add_tag(tag.target, at, tag.leader)?))
            } else {
                model.move_tag(id, at)?;
                Ok(e)
            }
        }
        other => Ok(other),
    }
}

/// A room moved, turned or copied. A room bounded by walls moved without
/// them only moves its stamp, while it stays inside; moved out of its
/// walls, it is freed and goes whole.
fn room(
    model: &mut Model,
    id: archisim_core::RoomId,
    t: &Transform,
    copy: bool,
    walls_too: bool,
) -> Result<Element, ModelError> {
    let r = model.room(id).ok_or(ModelError::UnknownRoom(id))?.clone();
    let label = t.apply(r.label);
    let whole = archisim_core::Room {
        outline: r.outline.iter().map(|p| t.apply(*p)).collect(),
        holes: r
            .holes
            .iter()
            .map(|h| h.iter().map(|p| t.apply(*p)).collect())
            .collect(),
        label,
        ..r.clone()
    };
    if copy {
        let number = model.next_room_number(r.level);
        let new = model.add_room(archisim_core::NewRoom {
            level: r.level,
            number,
            name: r.name.clone(),
            outline: whole.outline,
            holes: whole.holes,
            label,
            bounded: r.bounded && walls_too,
        })?;
        if let Some(mut n) = model.room(new).cloned() {
            n.height = r.height;
            n.finishes = r.finishes.clone();
            model.set_room(n)?;
        }
        return Ok(Element::Room(new));
    }
    let moved = if !r.bounded || walls_too {
        whole
    } else if r.contains(label) {
        archisim_core::Room { label, ..r }
    } else {
        archisim_core::Room {
            bounded: false,
            ..whole
        }
    };
    model.set_room(moved)?;
    Ok(Element::Room(id))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pick::tests::room;
    use archisim_core::{Point2, Shape, ViewRef, LAYER_ZERO};
    use std::f64::consts::FRAC_PI_2;

    fn p(x: f64, y: f64) -> Point2 {
        Point2::new(x, y)
    }

    #[test]
    fn rotate_a_room_with_its_door() {
        let (mut m, _, _) = room();
        let all: BTreeSet<Element> = m
            .walls()
            .map(|w| Element::Wall(w.id))
            .chain(m.slabs().map(|s| Element::Slab(s.id)))
            .collect();
        let guids: Vec<_> = m.walls().map(|w| w.guid.clone()).collect();
        let door = m.openings().next().unwrap().clone();
        transform(
            &mut m,
            &all,
            &Transform::rotate(p(0.0, 0.0), FRAC_PI_2),
            false,
        )
        .unwrap();
        // Same walls, turned: the west wall now runs along −x.
        assert_eq!(m.walls().map(|w| w.guid.clone()).collect::<Vec<_>>(), guids);
        let west = m.walls().next().unwrap();
        assert!(west.end.approx_eq(p(-4.0, 0.0)), "{:?}", west.end);
        assert_eq!(m.opening(door.id).unwrap().along, door.along);
        assert!(m.slabs().next().unwrap().covers(p(-2.0, 3.0)));
    }

    #[test]
    fn mirror_keeps_exterior_faces_and_swaps_hands() {
        let (mut m, _, _) = room();
        let all: BTreeSet<Element> = m.walls().map(|w| Element::Wall(w.id)).collect();
        let door = m.openings().next().unwrap().clone();
        let out = transform(
            &mut m,
            &all,
            &Transform::mirror(p(10.0, 0.0), p(10.0, 1.0)),
            true,
        )
        .unwrap();
        // Four walls and the door copied.
        assert_eq!(out.len(), 5);
        let copy = m.openings().find(|o| o.id != door.id).unwrap();
        assert_ne!(copy.hand, door.hand);
        // The copied room is drawn clockwise again round its exterior: the
        // centre of the copy is inside every wall's interior side.
        let centre = p(17.0, 2.0);
        for w in m.walls().filter(|w| w.start.x > 10.0) {
            let d = (w.end - w.start).normalized().unwrap();
            // Exterior on the left: the centre is on the right.
            assert!(d.cross(centre - w.start) < 0.0, "{w:?}");
        }
    }

    #[test]
    fn copies_of_drafts_and_dimensions() {
        let (mut m, lvl, _) = room();
        let line = m
            .add_draft(NewDraft::new(
                ViewRef::Level(lvl),
                LAYER_ZERO,
                Shape::Line {
                    a: p(0.0, -1.0),
                    b: p(6.0, -1.0),
                },
            ))
            .unwrap();
        let dim = m
            .add_dimension(NewDimension {
                level: lvl,
                points: vec![p(0.0, 0.0), p(6.0, 0.0)],
                direction: p(1.0, 0.0),
                position: p(0.0, -2.0),
            })
            .unwrap();
        let sel: BTreeSet<Element> = [Element::Draft(line), Element::Dimension(dim)].into();
        let out = transform(&mut m, &sel, &Transform::scale(p(0.0, 0.0), 2.0), true).unwrap();
        assert_eq!(out.len(), 2);
        assert_eq!(m.drafts().count(), 2);
        let big = m.dimensions().last().unwrap();
        assert!((big.values()[0] - 12.0).abs() < 1e-9);
    }
}
