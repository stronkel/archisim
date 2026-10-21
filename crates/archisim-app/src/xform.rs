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
    let mut out = transform_phase_2(model, selection, t, copy)?;
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
                // The slope arrow of an inclined slab goes with it.
                let slope = s.slope.map(|sl| archisim_core::SlabSlope {
                    from: t.apply(sl.from),
                    to: t.apply(sl.to),
                    slope: sl.slope,
                });
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
                    model.set_slab_slope(new, slope)?;
                    out.insert(Element::Slab(new));
                } else {
                    // Holes go first out of the way, then come back.
                    model.set_slab_holes(id, Vec::new())?;
                    model.set_slab_outline(id, outline)?;
                    model.set_slab_holes(id, holes)?;
                    model.set_slab_slope(id, slope)?;
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
            Element::Opening(_)
            | Element::Column(_)
            | Element::Beam(_)
            | Element::Section(_)
            | Element::Roof(_)
            | Element::Stair(_)
            | Element::Terrain(_)
            | Element::CurtainWall(_)
            | Element::Railing(_)
            | Element::Object(_)
            | Element::Footing(_)
            | Element::Grid(_)
            | Element::Module(_) => {}
            Element::Plot(id) => {
                let x = model
                    .plot(id)
                    .ok_or(ModelError::UnknownElement(id.0))?
                    .clone();
                let outline: Vec<_> = x.outline.iter().map(|p| t.apply(*p)).collect();
                if copy {
                    let new = model.add_plot(archisim_core::NewPlot {
                        name: model.next_plot_name(),
                        level: x.level,
                        outline,
                        setback: x.setback,
                        reference: String::new(),
                    })?;
                    out.insert(Element::Plot(new));
                } else {
                    model.edit_plot(id, |p| p.outline = outline)?;
                    out.insert(*e);
                }
            }
            Element::Run(id) => {
                let x = model
                    .run(id)
                    .ok_or(ModelError::UnknownElement(id.0))?
                    .clone();
                if copy {
                    let new = model.add_run(run_spec(&x, |p| t.apply(p)))?;
                    out.insert(Element::Run(new));
                } else {
                    model.edit_run(id, |r| r.path.iter_mut().for_each(|p| *p = t.apply(*p)))?;
                    out.insert(*e);
                }
            }
            Element::Device(id) => {
                let x = model
                    .device(id)
                    .ok_or(ModelError::UnknownElement(id.0))?
                    .clone();
                let turn = t.angle(0.0);
                if copy {
                    let new = model.add_device(device_spec(&x, |p| t.apply(p), turn))?;
                    out.insert(Element::Device(new));
                } else {
                    model.edit_device(id, |d| {
                        d.position = t.apply(d.position);
                        d.rotation += turn;
                    })?;
                    out.insert(*e);
                }
            }
            Element::Road(id) => {
                let x = model
                    .road(id)
                    .ok_or(ModelError::UnknownElement(id.0))?
                    .clone();
                let axis: Vec<_> = x.axis.iter().map(|p| t.apply(*p)).collect();
                if copy {
                    let new = model.add_road(archisim_core::NewRoad {
                        name: model.next_road_name(),
                        level: x.level,
                        axis,
                        section: x.section,
                        top_offset: x.top_offset,
                        pavement: x.pavement,
                        kerb_material: x.kerb_material,
                        sidewalk_material: x.sidewalk_material,
                    })?;
                    out.insert(Element::Road(new));
                } else {
                    model.edit_road(id, |r| r.axis = axis)?;
                    out.insert(*e);
                }
            }
            Element::Mass(id) => {
                let mut x = model
                    .mass(id)
                    .ok_or(ModelError::UnknownElement(id.0))?
                    .clone();
                x.map_points(|p| t.apply(p));
                if let archisim_core::MassShape::Sweep { width, .. } = &mut x.shape {
                    *width *= f;
                }
                if copy {
                    let new = model.add_mass(archisim_core::NewMass {
                        name: model.next_mass_name(x.floor_height.is_some()),
                        level: x.level,
                        shape: x.shape,
                        base_offset: x.base_offset,
                        height: x.height,
                        floor_height: x.floor_height,
                        material: x.material,
                    })?;
                    out.insert(Element::Mass(new));
                } else {
                    model.edit_mass(id, |m| m.shape = x.shape)?;
                    out.insert(*e);
                }
            }
        }
    }
    Ok(out)
}

/// The elements of phase 2 (columns, beams, footings, grid lines, roofs, stairs…) moved, copied
/// or turned with the 2D editing commands; they are not scaled or
/// mirrored.
fn transform_phase_2(
    model: &mut Model,
    selection: &BTreeSet<Element>,
    t: &Transform,
    copy: bool,
) -> Result<BTreeSet<Element>, ModelError> {
    use archisim_core::{Point2, Vec2};
    let items: BTreeSet<Element> = selection
        .iter()
        .copied()
        .filter(|e| {
            matches!(
                e,
                Element::Column(_)
                    | Element::Beam(_)
                    | Element::Section(_)
                    | Element::Roof(_)
                    | Element::Stair(_)
                    | Element::Terrain(_)
                    | Element::CurtainWall(_)
                    | Element::Railing(_)
                    | Element::Object(_)
                    | Element::Footing(_)
                    | Element::Grid(_)
                    | Element::Module(_)
            )
        })
        .collect();
    if items.is_empty() {
        return Ok(BTreeSet::new());
    }
    let (ex, ey) = (t.linear(Vec2::new(1.0, 0.0)), t.linear(Vec2::new(0.0, 1.0)));
    let d = t.apply(Point2::new(0.0, 0.0));
    let same = |a: Vec2, b: Vec2| (a - b).length() < 1e-9;
    if same(ex, Vec2::new(1.0, 0.0)) && same(ey, Vec2::new(0.0, 1.0)) {
        return if copy {
            crate::drag::copy(model, &items, d)
        } else {
            crate::drag::move_selection(model, &items, d)?;
            Ok(items)
        };
    }
    if t.is_mirror() || (t.factor() - 1.0).abs() > 1e-9 {
        return Err(ModelError::InvalidShape(
            "columns, beams, footings, grid lines, roofs, stairs, curtain walls, railings, objects, terrains and sections can be moved, copied or rotated, not scaled or mirrored",
        ));
    }
    // A turn: the copies are made in place and then turned.
    let targets = if copy {
        crate::drag::copy(model, &items, Vec2::new(0.0, 0.0))?
    } else {
        items
    };
    let turn = t.angle(0.0);
    let pts = |v: &mut Vec<Point2>| {
        for p in v.iter_mut() {
            *p = t.apply(*p);
        }
    };
    for e in &targets {
        match *e {
            Element::Column(id) => model.edit_column(id, |c| {
                c.position = t.apply(c.position);
                c.rotation += turn;
            })?,
            Element::Object(id) => model.edit_object(id, |o| {
                o.position = t.apply(o.position);
                o.rotation += turn;
            })?,
            Element::Beam(id) => model.edit_beam(id, |b| {
                b.start = t.apply(b.start);
                b.end = t.apply(b.end);
            })?,
            Element::Section(id) => model.edit_section(id, |x| {
                x.start = t.apply(x.start);
                x.end = t.apply(x.end);
            })?,
            Element::CurtainWall(id) => model.edit_curtain_wall(id, |c| {
                c.start = t.apply(c.start);
                c.end = t.apply(c.end);
            })?,
            Element::Railing(id) => model.edit_railing(id, |r| pts(&mut r.path))?,
            Element::Footing(id) => {
                crate::drag::place_footing(model, id, &|p| t.apply(p), turn, selection)?
            }
            Element::Grid(id) => model.edit_grid(id, |g| {
                g.start = t.apply(g.start);
                g.end = t.apply(g.end);
            })?,
            Element::Module(id) => model.edit_module(id, |m| {
                m.position = t.apply(m.position);
                m.rotation += turn;
            })?,
            Element::Stair(id) => model.edit_stair(id, |x| pts(&mut x.path))?,
            Element::Roof(id) => model.edit_roof(id, |r| pts(&mut r.outline))?,
            Element::Terrain(id) => model.edit_terrain(id, |x| {
                for p in x.points.iter_mut() {
                    p.at = t.apply(p.at);
                }
            })?,
            _ => {}
        }
    }
    Ok(targets)
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

/// What makes a copy of a pipe or duct, its path moved by `f`.
pub fn run_spec(
    x: &archisim_core::ServiceRun,
    f: impl Fn(archisim_core::Point2) -> archisim_core::Point2,
) -> archisim_core::NewRun {
    archisim_core::NewRun {
        level: x.level,
        system: x.system,
        path: x.path.iter().map(|p| f(*p)).collect(),
        elevation: x.elevation,
        width: x.width,
        height: x.height,
    }
}

/// What makes a copy of a device, moved by `f` and turned by `turn`.
pub fn device_spec(
    x: &archisim_core::ServiceDevice,
    f: impl Fn(archisim_core::Point2) -> archisim_core::Point2,
    turn: f64,
) -> archisim_core::NewDevice {
    archisim_core::NewDevice {
        level: x.level,
        kind: x.kind,
        name: x.name.clone(),
        position: f(x.position),
        rotation: x.rotation + turn,
        elevation: x.elevation,
        width: x.width,
        depth: x.depth,
        height: x.height,
    }
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

    /// Phase 2 elements go with the 2D commands: moved, copied, turned;
    /// not mirrored.
    #[test]
    fn columns_and_beams_move_copy_and_turn() {
        let (mut m, lvl, _) = room();
        let mt = m
            .member_types()
            .find(|t| t.usage == archisim_core::MemberUsage::Column)
            .unwrap()
            .id;
        let c = m
            .add_column(archisim_core::NewColumn {
                member_type: mt,
                level: lvl,
                position: p(2.0, 1.0),
                rotation: 0.0,
                base_offset: 0.0,
                top: archisim_core::WallTop::Unconnected,
                height: 2.7,
            })
            .unwrap();
        let sel: BTreeSet<Element> = [Element::Column(c)].into_iter().collect();
        let guid = m.column(c).unwrap().guid.clone();
        transform(&mut m, &sel, &Transform::translate(p(1.0, 0.0)), false).unwrap();
        assert!(m.column(c).unwrap().position.distance(p(3.0, 1.0)) < 1e-9);
        let copies = transform(&mut m, &sel, &Transform::translate(p(0.0, 2.0)), true).unwrap();
        assert_eq!(m.columns().count(), 2);
        assert_eq!(copies.len(), 1);
        assert!(!copies.contains(&Element::Column(c)));
        transform(
            &mut m,
            &sel,
            &Transform::rotate(p(0.0, 0.0), FRAC_PI_2),
            false,
        )
        .unwrap();
        let col = m.column(c).unwrap();
        assert!(
            col.position.distance(p(-1.0, 3.0)) < 1e-9,
            "{:?}",
            col.position
        );
        assert!((col.rotation - FRAC_PI_2).abs() < 1e-9);
        assert_eq!(col.guid, guid);
        // A turned copy: in place, then turned.
        transform(
            &mut m,
            &sel,
            &Transform::rotate(p(-1.0, 3.0), FRAC_PI_2),
            true,
        )
        .unwrap();
        assert_eq!(m.columns().count(), 3);
        assert!(transform(
            &mut m,
            &sel,
            &Transform::mirror(p(0.0, 0.0), p(0.0, 1.0)),
            false
        )
        .is_err());
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
