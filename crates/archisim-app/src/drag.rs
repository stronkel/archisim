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
    /// The stamp of a room (its name and area), moved on its own
    /// inside the room.
    RoomLabel { room: archisim_core::RoomId },
    /// A survey point of a terrain.
    TerrainPoint {
        terrain: archisim_core::TerrainId,
        index: usize,
    },
    /// A point of a railing's line.
    RailingPoint {
        railing: archisim_core::RailingId,
        index: usize,
    },
    /// An end of a reference plane.
    RefPlaneEnd {
        plane: archisim_core::RefPlaneId,
        start: bool,
    },
    /// A point of a road's axis.
    RoadPoint {
        road: archisim_core::RoadId,
        index: usize,
    },
    /// A corner of a pool's outline (the edge of its water).
    PoolCorner {
        pool: archisim_core::PoolId,
        index: usize,
    },
    /// A size handle of an object (an arrow on its right, left or front
    /// side): the object grows only towards the handle, the opposite side
    /// staying put (with Alt, both sides about its middle).
    ObjectSize {
        object: archisim_core::ObjectId,
        side: ObjectSide,
    },
    /// The turning handle of an object (the ring in front of it): dragged
    /// round its middle, in steps of 15° (1° when the cursor is far).
    ObjectTurn { object: archisim_core::ObjectId },
    /// A jamb of a door or window (ArchiCAD's hotspots on its rough
    /// opening): dragged along the wall, it widens or narrows the opening
    /// with the other jamb staying put. `start`: the jamb nearer the wall's
    /// start.
    OpeningJamb {
        opening: archisim_core::OpeningId,
        start: bool,
    },
    /// A measured point of a dimension: dragged to another point, the
    /// dimension measures to it (no need to delete and draw it again).
    DimPoint {
        dimension: archisim_core::DimensionId,
        index: usize,
    },
    /// One end of a curtain wall.
    CurtainEnd {
        curtain: archisim_core::CurtainWallId,
        start: bool,
    },
    /// A point of a stair's walking line.
    StairPoint {
        stair: archisim_core::StairId,
        index: usize,
    },
    /// A corner of a roof's footprint.
    RoofCorner {
        roof: archisim_core::RoofId,
        index: usize,
    },
    /// Where planes of a roof meet (an end of a ridge, hip or valley): moved,
    /// the planes there change pitch.
    RoofNode {
        roof: archisim_core::RoofId,
        index: usize,
    },
    /// One end of the slope arrow of an inclined slab.
    SlopeEnd { slab: SlabId, start: bool },
    /// One end of a section or elevation line.
    SectionEnd {
        section: archisim_core::SectionId,
        start: bool,
    },
    /// A grip of a drafting element (its end, centre, middle…), by its
    /// index in [`archisim_draft::grips::grips`].
    DraftPoint {
        draft: archisim_core::DraftId,
        index: usize,
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

/// The side of an object a size handle is on, in its own axes: right and
/// left change its width, the front its depth (the back usually leans on a
/// wall).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ObjectSide {
    Right,
    Left,
    Front,
}

impl ObjectSide {
    /// The measurement it changes.
    pub fn key(self) -> &'static str {
        match self {
            ObjectSide::Right | ObjectSide::Left => "width",
            ObjectSide::Front => "depth",
        }
    }

    /// The name of that measurement.
    pub fn label(self) -> &'static str {
        match self {
            ObjectSide::Right | ObjectSide::Left => "Ancho",
            ObjectSide::Front => "Fondo",
        }
    }

    /// Which side stays put when it grows only towards this one.
    pub fn keep(self) -> Keep {
        match self {
            ObjectSide::Right => Keep::Low(0),
            ObjectSide::Left => Keep::High(0),
            ObjectSide::Front => Keep::Low(1),
        }
    }

    /// The direction it points, in the object's axes.
    pub fn dir(self) -> (f64, f64) {
        match self {
            ObjectSide::Right => (1.0, 0.0),
            ObjectSide::Left => (-1.0, 0.0),
            ObjectSide::Front => (0.0, 1.0),
        }
    }
}

/// What stays where it was when an object changes size: its low or high
/// face on an axis (0 width, 1 depth, 2 height) or its middle.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Keep {
    Low(usize),
    High(usize),
    Middle(usize),
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
                    // The arrow first: its ends may lie on the edges.
                    if let Some(a) = s.slope {
                        for (start, point) in [(true, a.from), (false, a.to)] {
                            v.push(Hotspot {
                                grip: Grip::SlopeEnd { slab: *id, start },
                                point,
                            });
                        }
                    }
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
            Element::Room(id) => {
                if let Some(r) = model.room(*id) {
                    v.push(Hotspot {
                        grip: Grip::RoomLabel { room: *id },
                        point: r.label,
                    });
                }
            }
            Element::Section(id) => {
                if let Some(s) = model.section(*id) {
                    for (start, point) in [(true, s.start), (false, s.end)] {
                        v.push(Hotspot {
                            grip: Grip::SectionEnd {
                                section: *id,
                                start,
                            },
                            point,
                        });
                    }
                }
            }
            // Moved whole, turned with Space; width and depth by their
            // handles if the family has them.
            Element::Object(id) => {
                for (side, point) in object_handles(model, *id) {
                    v.push(Hotspot {
                        grip: Grip::ObjectSize { object: *id, side },
                        point,
                    });
                }
                if let Some(point) = turn_handle(model, *id) {
                    v.push(Hotspot {
                        grip: Grip::ObjectTurn { object: *id },
                        point,
                    });
                }
            }
            Element::Railing(id) => {
                if let Some(r) = model.railing(*id) {
                    for (index, point) in r.path.iter().enumerate() {
                        v.push(Hotspot {
                            grip: Grip::RailingPoint {
                                railing: *id,
                                index,
                            },
                            point: *point,
                        });
                    }
                }
            }
            Element::RefPlane(id) => {
                if let Some(r) = model.ref_plane(*id) {
                    for (start, point) in [(true, r.start), (false, r.end)] {
                        v.push(Hotspot {
                            grip: Grip::RefPlaneEnd { plane: *id, start },
                            point,
                        });
                    }
                }
            }
            Element::Road(id) => {
                if let Some(r) = model.road(*id) {
                    for (index, point) in r.axis.iter().enumerate() {
                        v.push(Hotspot {
                            grip: Grip::RoadPoint { road: *id, index },
                            point: *point,
                        });
                    }
                }
            }
            Element::Pool(id) => {
                if let Some(p) = model.pool(*id) {
                    for (index, point) in p.outline.iter().enumerate() {
                        v.push(Hotspot {
                            grip: Grip::PoolCorner { pool: *id, index },
                            point: *point,
                        });
                    }
                }
            }
            Element::CurtainWall(id) => {
                if let Some(c) = model.curtain_wall(*id) {
                    for (start, point) in [(true, c.start), (false, c.end)] {
                        v.push(Hotspot {
                            grip: Grip::CurtainEnd {
                                curtain: *id,
                                start,
                            },
                            point,
                        });
                    }
                }
            }
            Element::Terrain(id) => {
                if let Some(t) = model.terrain(*id) {
                    // The surveyed points (not those of its platforms).
                    for (index, p) in t.points.iter().take(t.survey().len()).enumerate() {
                        v.push(Hotspot {
                            grip: Grip::TerrainPoint {
                                terrain: *id,
                                index,
                            },
                            point: p.at,
                        });
                    }
                }
            }
            Element::Stair(id) => {
                if let Some(s) = model.stair(*id) {
                    for (index, point) in s.path.iter().enumerate() {
                        v.push(Hotspot {
                            grip: Grip::StairPoint { stair: *id, index },
                            point: *point,
                        });
                    }
                }
            }
            Element::Roof(id) => {
                if let Some(r) = model.roof(*id) {
                    for (index, point) in r.outline.iter().enumerate() {
                        v.push(Hotspot {
                            grip: Grip::RoofCorner { roof: *id, index },
                            point: *point,
                        });
                    }
                    for (index, (point, _)) in r.nodes().iter().enumerate() {
                        v.push(Hotspot {
                            grip: Grip::RoofNode { roof: *id, index },
                            point: *point,
                        });
                    }
                }
            }
            Element::Dimension(id) => {
                if let Some(d) = model.dimension(*id) {
                    for (index, point) in d.points.iter().enumerate() {
                        v.push(Hotspot {
                            grip: Grip::DimPoint {
                                dimension: *id,
                                index,
                            },
                            point: *point,
                        });
                    }
                }
            }
            Element::Draft(id) => {
                if let Some(d) = model.draft(*id) {
                    for (index, (point, _)) in archisim_draft::grips::grips(&d.shape)
                        .into_iter()
                        .enumerate()
                    {
                        v.push(Hotspot {
                            grip: Grip::DraftPoint { draft: *id, index },
                            point,
                        });
                    }
                }
            }
            // Its two jambs, on the face of the wall it does not open to
            // (the temporary dimensions measure on the other): they
            // stretch the width.
            Element::Opening(id) => {
                if let Some(og) = archisim_geom::opening_geometry(model, *id) {
                    let face = if (og.swing_face() - og.exterior_offset).abs() < 1e-9 {
                        og.interior_offset
                    } else {
                        og.exterior_offset
                    };
                    for (start, t) in [(true, og.t0), (false, og.t1)] {
                        v.push(Hotspot {
                            grip: Grip::OpeningJamb {
                                opening: *id,
                                start,
                            },
                            point: og.point(t, face),
                        });
                    }
                }
            }
            Element::Mark(_)
            | Element::Column(_)
            | Element::Dormer(..)
            | Element::Tag(_)
            | Element::Footing(_)
            | Element::Grid(_)
            | Element::Module(_)
            | Element::Mass(_)
            | Element::Plot(_)
            | Element::Planting(_)
            | Element::Run(_)
            | Element::Edge(_)
            | Element::Device(_) => {}
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
    let k = crate::pick::paper_k();
    let on_selection = selection.iter().any(|e| {
        if let Element::Draft(id) = e {
            return model
                .draft(*id)
                .is_some_and(|d| archisim_draft::distance(&d.shape, p, k) <= tolerance);
        }
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
        Grip::SlabCorner { slab, .. }
        | Grip::SlabEdge { slab, .. }
        | Grip::SlopeEnd { slab, .. } => [Element::Slab(slab)].into_iter().collect(),
        Grip::BeamEnd { beam, .. } => [Element::Beam(beam)].into_iter().collect(),
        Grip::RoomLabel { room } => [Element::Room(room)].into_iter().collect(),
        Grip::SectionEnd { section, .. } => [Element::Section(section)].into_iter().collect(),
        Grip::RoofCorner { roof, .. } | Grip::RoofNode { roof, .. } => {
            [Element::Roof(roof)].into_iter().collect()
        }
        Grip::StairPoint { stair, .. } => [Element::Stair(stair)].into_iter().collect(),
        Grip::CurtainEnd { curtain, .. } => [Element::CurtainWall(curtain)].into_iter().collect(),
        Grip::RailingPoint { railing, .. } => [Element::Railing(railing)].into_iter().collect(),
        Grip::RoadPoint { road, .. } => [Element::Road(road)].into_iter().collect(),
        Grip::PoolCorner { pool, .. } => [Element::Pool(pool)].into_iter().collect(),
        Grip::RefPlaneEnd { plane, .. } => [Element::RefPlane(plane)].into_iter().collect(),
        Grip::DimPoint { dimension, .. } => [Element::Dimension(dimension)].into_iter().collect(),
        Grip::ObjectSize { object, .. } | Grip::ObjectTurn { object } => {
            [Element::Object(object)].into_iter().collect()
        }
        Grip::OpeningJamb { opening, .. } => [Element::Opening(opening)].into_iter().collect(),
        Grip::TerrainPoint { terrain, .. } => [Element::Terrain(terrain)].into_iter().collect(),
        Grip::DraftPoint { draft, .. } => [Element::Draft(draft)].into_iter().collect(),
    }
}

/// The size handles of an object whose type has a `width` or a `depth`:
/// arrows in the middle of its right and left sides, and of its front (a plant: one,
/// on the edge of its crown).
fn object_handles(model: &Model, id: archisim_core::ObjectId) -> Vec<(ObjectSide, Point2)> {
    let Some(o) = model.object(id) else {
        return Vec::new();
    };
    if let Some(p) = model
        .type_grows(o.object_type)
        .then(|| plant_handle(model, id))
        .flatten()
    {
        return vec![(ObjectSide::Right, p)];
    }
    let Some((lo, hi)) = object_box(model, id) else {
        return Vec::new();
    };
    let (mx, my) = ((lo[0] + hi[0]) / 2.0, (lo[1] + hi[1]) / 2.0);
    let mut v = Vec::new();
    if type_number(model, id, "width").is_some() {
        v.push((ObjectSide::Right, o.to_plan(hi[0], my)));
        v.push((ObjectSide::Left, o.to_plan(lo[0], my)));
    }
    if type_number(model, id, "depth").is_some() {
        v.push((ObjectSide::Front, o.to_plan(mx, hi[1])));
    }
    v
}

/// A measurement of an object's type: its own value or its family's.
pub(crate) fn type_number(model: &Model, id: archisim_core::ObjectId, key: &str) -> Option<f64> {
    use archisim_core::Value;
    let t = model.object_type(model.object(id)?.object_type)?;
    if let Some(Value::Number(v)) = t.values.get(key) {
        return Some(*v);
    }
    let d = model
        .family_defaults(t.family, archisim_family::ParamScope::Type)
        .ok()?;
    match d.get(key) {
        Some(Value::Number(v)) => Some(*v),
        _ => None,
    }
}

/// The box an object fills, in its own axes (from its family's 3D, or
/// else its width centred and its depth forward from its point).
pub(crate) fn object_box(
    model: &Model,
    id: archisim_core::ObjectId,
) -> Option<([f64; 3], [f64; 3])> {
    use archisim_core::Value;
    let o = model.object(id)?;
    if let Some(b) = model.object_type_box_3d(o.object_type) {
        return Some(b);
    }
    let t = model.object_type(o.object_type)?;
    let num = |k: &str| match t.values.get(k) {
        Some(Value::Number(v)) => Some(*v),
        _ => None,
    };
    let (w, d) = (num("width")?, num("depth")?);
    let h = num("height").unwrap_or(0.0);
    Some(([-w / 2.0, 0.0, 0.0], [w / 2.0, d, h]))
}

/// The middle of the side that stays put while `side` stretches (with
/// Alt, none: the object grows about its middle).
pub(crate) fn fixed_side(
    model: &Model,
    id: archisim_core::ObjectId,
    side: ObjectSide,
) -> Option<Point2> {
    let o = model.object(id)?;
    let (lo, hi) = object_box(model, id)?;
    let (mx, my) = ((lo[0] + hi[0]) / 2.0, (lo[1] + hi[1]) / 2.0);
    Some(match side {
        ObjectSide::Right => o.to_plan(lo[0], my),
        ObjectSide::Left => o.to_plan(hi[0], my),
        ObjectSide::Front => o.to_plan(mx, lo[1]),
    })
}

/// How far in front of an object its turning handle is, in metres.
pub(crate) const TURN_GAP: f64 = 0.35;

/// Where the turning handle of an object is: in front of its middle.
pub(crate) fn turn_handle(model: &Model, id: archisim_core::ObjectId) -> Option<Point2> {
    let o = model.object(id)?;
    let (lo, hi) = object_box(model, id)?;
    Some(o.to_plan((lo[0] + hi[0]) / 2.0, hi[1] + TURN_GAP))
}

/// The middle of an object in plan, what it turns about.
pub(crate) fn object_middle(model: &Model, id: archisim_core::ObjectId) -> Option<Point2> {
    let o = model.object(id)?;
    let (lo, hi) = object_box(model, id)?;
    Some(o.to_plan((lo[0] + hi[0]) / 2.0, (lo[1] + hi[1]) / 2.0))
}

/// The turn an object takes when its handle is dragged to `to`: pointing
/// from its middle to the cursor, in steps of 15° (1° when the cursor is
/// further than three times the handle's distance).
pub(crate) fn turn_angle(model: &Model, id: archisim_core::ObjectId, to: Point2) -> Option<f64> {
    let c = object_middle(model, id)?;
    let h = turn_handle(model, id)?;
    let reach = h.distance(c).max(0.1);
    let rel = to - c;
    if rel.length() < 1e-6 {
        return None;
    }
    // The handle sits on the object's +y axis.
    let raw = rel.y.atan2(rel.x) - std::f64::consts::FRAC_PI_2;
    let step = f64::to_radians(if rel.length() > 3.0 * reach {
        1.0
    } else {
        15.0
    });
    let a = (raw / step).round() * step;
    Some(a.rem_euclid(std::f64::consts::TAU))
}

/// Turns an object to `angle` about its middle.
pub(crate) fn turn_object(
    model: &mut Model,
    id: archisim_core::ObjectId,
    angle: f64,
) -> Result<(), ModelError> {
    let c = object_middle(model, id).ok_or(ModelError::UnknownObject(id))?;
    let o = model.object(id).ok_or(ModelError::UnknownObject(id))?;
    let d = angle - o.rotation;
    let rel = o.position - c;
    let (cs, sn) = (d.cos(), d.sin());
    let position = Point2::new(c.x + rel.x * cs - rel.y * sn, c.y + rel.x * sn + rel.y * cs);
    model.edit_object(id, |o| {
        o.rotation = angle;
        o.position = position;
    })
}

/// The edge of a plant's crown, east of its trunk (turned with it): its
/// width handle.
fn plant_handle(model: &Model, id: archisim_core::ObjectId) -> Option<Point2> {
    let o = model.object(id)?;
    let look = model.plant_look(id)?;
    Some(o.to_plan(look.species.width / 2.0, 0.0))
}

/// A plant made wider or narrower from its handle, by itself (its own
/// width; with Ctrl, its height too, in proportion). Its type, the
/// species, does not change.
fn stretch_plant(
    model: &mut Model,
    id: archisim_core::ObjectId,
    to: Point2,
    both: bool,
) -> Result<(), ModelError> {
    use archisim_core::Value;
    let o = model
        .object(id)
        .ok_or(ModelError::UnknownObject(id))?
        .clone();
    let look = model.plant_look(id).ok_or(ModelError::UnknownObject(id))?;
    let width = ((to.distance(o.position) * 2.0).max(0.1) * 1000.0).round() / 1000.0;
    let mut values = o.values.clone();
    values.insert("crown_own".into(), Value::Number(width));
    if both {
        let k = width / look.species.width.max(1e-3);
        let h = ((look.species.height * k).max(0.1) * 1000.0).round() / 1000.0;
        values.insert("height_own".into(), Value::Number(h));
    }
    model.set_object_values(id, values)
}

/// Stretches an object from its handle on `side` to `to`: it grows only
/// towards that side, the opposite one staying put; with `both`, about its
/// middle. A type shared with other objects is copied first, so only this
/// one changes.
fn stretch_object(
    model: &mut Model,
    id: archisim_core::ObjectId,
    side: ObjectSide,
    to: Point2,
    both: bool,
) -> Result<(), ModelError> {
    let o = model
        .object(id)
        .ok_or(ModelError::UnknownObject(id))?
        .clone();
    let (lo, hi) = object_box(model, id).ok_or(ModelError::UnknownObject(id))?;
    let rel = to - o.position;
    let (c, s) = (o.rotation.cos(), o.rotation.sin());
    let (x, y) = (rel.x * c + rel.y * s, -rel.x * s + rel.y * c);
    let (axis, at) = match side {
        ObjectSide::Right | ObjectSide::Left => (0, x),
        ObjectSide::Front => (1, y),
    };
    let value = if both {
        2.0 * (at - (lo[axis] + hi[axis]) / 2.0).abs()
    } else {
        match side.keep() {
            Keep::Low(_) => at - lo[axis],
            _ => hi[axis] - at,
        }
    };
    let value = (value.max(0.05) * 1000.0).round() / 1000.0;
    let keep = if both {
        Keep::Middle(axis)
    } else {
        side.keep()
    };
    resize_object(model, id, side.key(), value, keep)
}

/// Gives an object a new `width`, `depth` or `height`, moving it so that
/// `keep` stays where it was. A type shared with other objects is copied
/// first, so only this one changes.
pub(crate) fn resize_object(
    model: &mut Model,
    id: archisim_core::ObjectId,
    key: &str,
    value: f64,
    keep: Keep,
) -> Result<(), ModelError> {
    let before = object_box(model, id);
    set_object_size(model, id, key, value)?;
    let after = object_box(model, id);
    let (Some((lo0, hi0)), Some((lo1, hi1))) = (before, after) else {
        return Ok(());
    };
    let shift = |a: usize| match keep {
        Keep::Low(k) if k == a => lo0[a] - lo1[a],
        Keep::High(k) if k == a => hi0[a] - hi1[a],
        Keep::Middle(k) if k == a => (lo0[a] + hi0[a] - lo1[a] - hi1[a]) / 2.0,
        _ => 0.0,
    };
    let (dx, dy, dz) = (shift(0), shift(1), shift(2));
    if dx.abs() < 1e-9 && dy.abs() < 1e-9 && dz.abs() < 1e-9 {
        return Ok(());
    }
    let o = model.object(id).ok_or(ModelError::UnknownObject(id))?;
    let (c, s) = (o.rotation.cos(), o.rotation.sin());
    let position = Point2::new(
        o.position.x + dx * c - dy * s,
        o.position.y + dx * s + dy * c,
    );
    model.edit_object(id, |o| {
        o.position = position;
        o.base_offset += dz;
    })
}

/// A measurement of an object's type (`width`, `depth`, `height`): a type
/// shared with other objects is copied first, so only this one changes.
pub(crate) fn set_object_size(
    model: &mut Model,
    id: archisim_core::ObjectId,
    key: &str,
    value: f64,
) -> Result<(), ModelError> {
    set_object_value(model, id, key, archisim_core::Value::Number(value))
}

/// Sets a value (a measurement, a material, a choice) of this object
/// only: it goes to the object's own type ([`Model::own_object_type`]),
/// and the type it came from, and the others of it, stay as they were.
pub(crate) fn set_object_value(
    model: &mut Model,
    id: archisim_core::ObjectId,
    key: &str,
    value: archisim_core::Value,
) -> Result<(), ModelError> {
    let own = model.own_object_type(id)?;
    let t = model
        .object_type(own)
        .ok_or(ModelError::UnknownObjectType(own))?
        .clone();
    let mut values = t.values.clone();
    values.insert(key.into(), value);
    // What follows the measurement: a longer sofa gains a seat.
    model.follow_stretch(t.family, &mut values);
    model.set_object_type_values(own, values)
}

/// Moves one jamb of a door or window along its wall to `to` (projected
/// on the wall): its type's width changes and its centre moves so the
/// other jamb stays where it was. A type shared with other doors or windows
/// is copied first (with a new mark), so only this one changes.
fn stretch_opening(
    model: &mut Model,
    id: archisim_core::OpeningId,
    start: bool,
    to: Point2,
) -> Result<(), ModelError> {
    use archisim_core::Value;
    let og = archisim_geom::opening_geometry(model, id).ok_or(ModelError::UnknownOpening(id))?;
    let o = model
        .opening(id)
        .ok_or(ModelError::UnknownOpening(id))?
        .clone();
    let t = (to - og.origin).dot(og.u);
    let (fixed, moved) = if start { (og.t1, t) } else { (og.t0, t) };
    // Too narrow, the family's range refuses it with its own message.
    let width = ((fixed - moved).abs() * 1000.0).round().max(1.0) / 1000.0;
    let centre = o.along + ((fixed + moved) / 2.0 - (og.t0 + og.t1) / 2.0);
    let ot = model
        .opening_type(o.opening_type)
        .ok_or(ModelError::UnknownOpeningType(o.opening_type))?
        .clone();
    let mut values = ot.values.clone();
    values.insert("width".into(), Value::Number(width));
    let ty = if model.opening_type_users(ot.id) > 1 {
        let taken: Vec<String> = model.opening_types().map(|t| t.name.clone()).collect();
        let name = crate::editor::copy_name(&ot.name, &taken);
        let new = model.duplicate_opening_type(ot.id, name)?;
        model.set_opening_type(id, new)?;
        new
    } else {
        ot.id
    };
    // Narrowing first and moving after, or moving first and widening after,
    // so it never overlaps its neighbours on the way.
    if width < ot.width {
        model.edit_family_type(ty, ot.family, values)?;
        model.move_opening(id, centre, o.sill)
    } else {
        // May not fit yet with the old width; the widening decides.
        let _ = model.move_opening(id, centre, o.sill);
        model.edit_family_type(ty, ot.family, values)?;
        model.move_opening(id, centre, o.sill)
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
    // An end that may not join drags alone.
    if !w.joins_at(p) {
        return v;
    }
    for o in model.walls_on_level(w.level) {
        if o.id == wall || !o.joins_at(p) {
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
        Grip::RoomLabel { .. } => "Mover rótulo de local",
        Grip::SectionEnd { .. } => "Estirar sección",
        Grip::SlopeEnd { .. } => "Mover flecha de pendiente",
        Grip::RoofCorner { .. } => "Mover esquina de cubierta",
        Grip::RoofNode { .. } => "Mover limatesa o limahoya",
        Grip::StairPoint { .. } => "Mover punto de la escalera",
        Grip::CurtainEnd { .. } => "Estirar muro cortina",
        Grip::RailingPoint { .. } => "Mover punto de la barandilla",
        Grip::RoadPoint { .. } => "Mover punto del eje del vial",
        Grip::PoolCorner { .. } => "Mover esquina de piscina",
        Grip::RefPlaneEnd { .. } => "Estirar plano de referencia",
        Grip::DimPoint { .. } => "Cambiar punto de cota",
        Grip::ObjectSize { .. } => "Estirar objeto",
        Grip::ObjectTurn { .. } => "Girar objeto",
        Grip::OpeningJamb { .. } => "Estirar hueco",
        Grip::TerrainPoint { .. } => "Mover punto del terreno",
        Grip::DraftPoint { .. } => "Estirar",
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
        Grip::RoomLabel { room } => {
            let mut r = model
                .room(room)
                .ok_or(ModelError::UnknownRoom(room))?
                .clone();
            let at = r.label + d;
            // A room that follows its walls is found again from its label.
            if !r.contains(at) {
                return Err(ModelError::InvalidShape(
                    "the stamp of a room stays inside it",
                ));
            }
            r.label = at;
            model.set_room(r).map(|_| None)
        }
        Grip::TerrainPoint { terrain, index } => model
            .edit_terrain(terrain, |t| {
                t.edit_survey(|points| {
                    if let Some(p) = points.get_mut(index) {
                        p.at = drag.to;
                    }
                })
            })
            .map(|_| None),
        Grip::ObjectSize { object, .. }
            if model
                .object(object)
                .is_some_and(|o| model.type_grows(o.object_type)) =>
        {
            stretch_plant(model, object, drag.to, drag.variant).map(|_| None)
        }
        Grip::ObjectSize { object, side } => {
            stretch_object(model, object, side, drag.to, drag.variant).map(|_| None)
        }
        Grip::ObjectTurn { object } => match turn_angle(model, object, drag.to) {
            Some(a) => turn_object(model, object, a).map(|_| None),
            None => Ok(None),
        },
        Grip::OpeningJamb { opening, start } => {
            stretch_opening(model, opening, start, drag.to).map(|_| None)
        }
        Grip::DimPoint { dimension, index } => {
            let d = model
                .dimension(dimension)
                .ok_or(ModelError::UnknownDimension(dimension))?;
            let mut points = d.points.clone();
            if let Some(p) = points.get_mut(index) {
                *p = drag.to;
            }
            let position = d.position;
            model
                .set_dimension(dimension, points, position)
                .map(|_| None)
        }
        Grip::RailingPoint { railing, index } => model
            .edit_railing(railing, |r| {
                if let Some(p) = r.path.get_mut(index) {
                    *p = drag.to;
                }
            })
            .map(|_| None),
        Grip::RefPlaneEnd { plane, start } => model
            .edit_ref_plane(plane, |r| {
                if start {
                    r.start = drag.to;
                } else {
                    r.end = drag.to;
                }
            })
            .map(|_| None),
        Grip::RoadPoint { road, index } => model
            .edit_road(road, |r| {
                if let Some(p) = r.axis.get_mut(index) {
                    *p = drag.to;
                }
            })
            .map(|_| None),
        Grip::PoolCorner { pool, index } => model
            .edit_pool(pool, |x| {
                if let Some(p) = x.outline.get_mut(index) {
                    *p = drag.to;
                }
            })
            .map(|_| None),
        Grip::CurtainEnd { curtain, start } => model
            .edit_curtain_wall(curtain, |c| {
                if start {
                    c.start = drag.to;
                } else {
                    c.end = drag.to;
                }
            })
            .map(|_| None),
        Grip::StairPoint { stair, index } => model
            .edit_stair(stair, |s| {
                // A spiral's centre takes its first riser with it.
                if index == 0 && s.design.form == archisim_core::StairForm::Spiral {
                    let d = drag.to - s.path[0];
                    for p in &mut s.path {
                        *p = *p + d;
                    }
                } else if let Some(p) = s.path.get_mut(index) {
                    *p = drag.to;
                }
            })
            .map(|_| None),
        Grip::RoofCorner { roof, index } => model
            .edit_roof(roof, |r| {
                if let Some(p) = r.outline.get_mut(index) {
                    *p = drag.to;
                }
            })
            .map(|_| None),
        Grip::RoofNode { roof, index } => model.move_roof_node(roof, index, drag.to).map(|_| None),
        Grip::SlopeEnd { slab, start } => {
            let s = model.slab(slab).ok_or(ModelError::UnknownSlab(slab))?;
            let Some(mut a) = s.slope else {
                return Ok(None);
            };
            if start {
                a.from = drag.to;
            } else {
                a.to = drag.to;
            }
            model.set_slab_slope(slab, Some(a)).map(|_| None)
        }
        Grip::SectionEnd { section, start } => model
            .edit_section(section, |s| {
                if start {
                    s.start = drag.to;
                } else {
                    s.end = drag.to;
                }
            })
            .map(|_| None),
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
        Grip::DraftPoint { draft, index } => {
            let d = model.draft(draft).ok_or(ModelError::UnknownDraft(draft))?;
            let shape = archisim_draft::grips::move_grip(&d.shape, index, drag.to);
            model.set_draft_shape(draft, shape).map(|_| None)
        }
    }
}

/// Moves the selection by `d`.
pub(crate) fn move_selection(
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
    let only_annotations = selection.iter().all(|e| {
        matches!(
            e,
            Element::Dimension(_)
                | Element::Mark(_)
                | Element::Draft(_)
                | Element::Room(_)
                | Element::Tag(_)
        )
    });
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
            Element::Section(id) => {
                model.edit_section(*id, |s| {
                    s.start = s.start + d;
                    s.end = s.end + d;
                })?;
            }
            Element::Roof(id) => {
                model.edit_roof(*id, |r| {
                    for p in &mut r.outline {
                        *p = *p + d;
                    }
                })?;
            }
            Element::Stair(id) => {
                model.edit_stair(*id, |s| {
                    for p in &mut s.path {
                        *p = *p + d;
                    }
                })?;
            }
            Element::Terrain(id) => {
                model.edit_terrain(*id, |t| {
                    for p in &mut t.points {
                        p.at = p.at + d;
                    }
                })?;
            }
            Element::Object(id) => {
                model.edit_object(*id, |o| o.position = o.position + d)?;
            }
            Element::Railing(id) => {
                model.edit_railing(*id, |r| {
                    for p in &mut r.path {
                        *p = *p + d;
                    }
                })?;
            }
            Element::CurtainWall(id) => {
                model.edit_curtain_wall(*id, |c| {
                    c.start = c.start + d;
                    c.end = c.end + d;
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
            Element::Draft(id) => {
                let shape = model
                    .draft(*id)
                    .ok_or(ModelError::UnknownDraft(*id))?
                    .shape
                    .clone();
                let moved = archisim_draft::transform_shape(
                    &shape,
                    &archisim_draft::Transform::translate(d),
                );
                model.set_draft_shape(*id, moved)?;
            }
            Element::Footing(id) => {
                place_footing(model, *id, &|p| p + d, 0.0, selection)?;
            }
            Element::Grid(id) => {
                model.edit_grid(*id, |g| {
                    g.start = g.start + d;
                    g.end = g.end + d;
                })?;
            }
            Element::RefPlane(id) => {
                model.edit_ref_plane(*id, |r| {
                    r.start = r.start + d;
                    r.end = r.end + d;
                })?;
            }
            Element::Module(id) => {
                model.edit_module(*id, |m| m.position = m.position + d)?;
            }
            Element::Mass(id) => {
                model.edit_mass(*id, |m| m.map_points(|p| p + d))?;
            }
            Element::Plot(id) => {
                model.edit_plot(*id, |x| x.outline.iter_mut().for_each(|p| *p = *p + d))?;
            }
            Element::Pool(id) => {
                model.edit_pool(*id, |x| x.outline.iter_mut().for_each(|p| *p = *p + d))?;
            }
            Element::Road(id) => {
                model.edit_road(*id, |x| x.axis.iter_mut().for_each(|p| *p = *p + d))?;
            }
            Element::Planting(id) => {
                model.edit_planting(*id, |x| x.path.iter_mut().for_each(|p| *p = *p + d))?;
            }
            Element::Run(id) => {
                model.edit_run(*id, |x| x.path.iter_mut().for_each(|p| *p = *p + d))?;
            }
            Element::Edge(id) => {
                model.edit_edge(*id, |x| x.points.iter_mut().for_each(|p| *p = *p + d))?;
            }
            Element::Device(id) => {
                model.edit_device(*id, |x| x.position = x.position + d)?;
            }
            Element::Room(_) | Element::Tag(_) => {
                // Moved with walls, a room goes whole.
                let walls_too = selection.iter().any(|x| matches!(x, Element::Wall(_)));
                let t = archisim_draft::Transform::translate(d);
                crate::xform::transform_one(model, *e, &t, false, walls_too)?;
            }
            Element::Dormer(r, i) if !selection.contains(&Element::Roof(*r)) => {
                let i = usize::from(*i);
                let at = model
                    .roof(*r)
                    .and_then(|x| x.dormers.get(i).and_then(|dm| x.dormer_frame(dm)))
                    .ok_or(ModelError::UnknownDormer(i))?
                    .origin;
                model.move_dormer(*r, i, at + d)?;
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
/// go. As in Revit, only the selected walls move: a neighbour keeps its
/// line and its end slides along its own axis to stay against the moved
/// wall, at a corner (L) or butting against it anywhere between its faces
/// (T: a partition stays against the wall it meets). Only where it cannot
/// slide (it runs parallel to the moved wall, or it is curved) does the
/// joined end move with the wall.
fn followers(model: &Model, moved: &[WallId], d: Point2) -> Vec<(WallId, Point2, Point2)> {
    use archisim_geom::{wall_curve, Curve, Line};
    /// A moved wall: its reference curve and the band between its faces
    /// (offsets from the reference for a straight wall, radii for a
    /// curved one).
    struct Host<'a> {
        wall: &'a archisim_core::Wall,
        curve: Curve,
        band: Option<(f64, f64)>,
    }
    let mut hosts: Vec<Host> = Vec::new();
    let mut level = None;
    for id in moved {
        let Some(w) = model.wall(*id) else { continue };
        level = Some(w.level);
        let Some(curve) = wall_curve(w, 0.0) else {
            continue;
        };
        let band = model.wall_type(w.wall_type).and_then(|t| {
            let (ext, int) = w.face_offsets(t.thickness());
            let measure = |c: Curve| match c {
                Curve::Circle { radius, .. } => radius,
                Curve::Line(l) => {
                    let n = (w.end - w.start)
                        .normalized()
                        .map_or(Point2::default(), Point2::perp_left);
                    (l.p - w.start).dot(n)
                }
            };
            let a = measure(wall_curve(w, ext)?);
            let b = measure(wall_curve(w, int)?);
            Some((a.min(b), a.max(b)))
        });
        hosts.push(Host {
            wall: w,
            curve,
            band,
        });
    }
    let Some(level) = level else {
        return Vec::new();
    };
    const EPS: f64 = 1e-5;
    // The curve through `p` that runs with a moved wall (its reference
    // line at a corner, a parallel one or a concentric circle at a T), in
    // its new place.
    let shifted = |h: &Host, p: Point2| match h.curve {
        Curve::Line(l) => Curve::Line(Line::new(p + d, l.d)),
        Curve::Circle { centre, .. } => Curve::Circle {
            centre: centre + d,
            radius: p.distance(centre),
        },
    };
    // Whether `p` lies in a moved wall's band, between its ends.
    let in_band = |h: &Host, p: Point2| -> bool {
        let Some((lo, hi)) = h.band else { return false };
        match (h.curve, h.wall.arc()) {
            (Curve::Circle { centre, .. }, Some(arc)) => {
                let (f, _) = arc.project(p);
                let r = p.distance(centre);
                f > 0.0 && f < 1.0 && r > lo - EPS && r < hi + EPS
            }
            (Curve::Line(_), _) => {
                let v = h.wall.end - h.wall.start;
                let len = v.length();
                let Some(u) = v.normalized() else {
                    return false;
                };
                let t = (p - h.wall.start).dot(u) / len;
                let off = (p - h.wall.start).dot(u.perp_left());
                t > EPS && t < 1.0 - EPS && off > lo - EPS && off < hi + EPS
            }
            _ => false,
        }
    };
    // Slides `p` along the neighbour's axis onto the moved curve; `None`
    // when they run (nearly) parallel.
    let slide = |c: Curve, p: Point2, axis: Point2| -> Option<Point2> {
        if let Curve::Line(l) = c {
            let (Some(a), Some(b)) = (axis.normalized(), l.d.normalized()) else {
                return None;
            };
            // Under about 3°, the meeting point runs off: not a corner.
            if a.cross(b).abs() < 0.05 {
                return None;
            }
        }
        Curve::Line(Line::new(p, axis)).intersect_near(&c, p + d)
    };
    // `joins`: whether the neighbour's end may join; a disallowed end (on
    // either wall) stays where it is.
    let follow = |p: Point2, axis: Option<Point2>, joins: bool| -> Option<Point2> {
        if !joins {
            return None;
        }
        if let Some(h) = hosts
            .iter()
            .find(|h| h.wall.start.approx_eq(p) || h.wall.end.approx_eq(p))
        {
            if !h.wall.joins_at(p) {
                return None;
            }
            return Some(
                axis.and_then(|a| slide(shifted(h, p), p, a))
                    .unwrap_or(p + d),
            );
        }
        let axis = axis?;
        hosts
            .iter()
            .find(|h| in_band(h, p))
            .and_then(|h| slide(shifted(h, p), p, axis))
    };
    model
        .walls_on_level(level)
        .filter(|w| !moved.contains(&w.id))
        .filter_map(|w| {
            // A curved wall has no axis to slide along: only its joined
            // ends follow.
            let axis = (!w.is_curved()).then(|| w.end - w.start);
            let s = follow(w.start, axis, w.allow_join[0]);
            let e = follow(w.end, axis, w.allow_join[1]);
            (s.is_some() || e.is_some()).then(|| (w.id, s.unwrap_or(w.start), e.unwrap_or(w.end)))
        })
        .collect()
}

/// Copies the selection by `d` and returns the copies.
pub(crate) fn copy(
    model: &mut Model,
    selection: &BTreeSet<Element>,
    d: Point2,
) -> Result<BTreeSet<Element>, ModelError> {
    let mut copies = BTreeSet::new();
    let mut draft_copies = Vec::new();
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
            Element::Room(_) | Element::Tag(_) => {
                let t = archisim_draft::Transform::translate(d);
                copies.insert(crate::xform::transform_one(
                    model,
                    *e,
                    &t,
                    true,
                    !copied_walls.is_empty(),
                )?);
            }
            Element::Draft(id) => {
                let dr = model
                    .draft(*id)
                    .ok_or(ModelError::UnknownDraft(*id))?
                    .clone();
                let new = model.add_draft(archisim_core::NewDraft {
                    view: dr.view,
                    layer: dr.layer,
                    shape: archisim_draft::transform_shape(
                        &dr.shape,
                        &archisim_draft::Transform::translate(d),
                    ),
                    color: dr.color,
                    weight: dr.weight,
                    line: dr.line,
                })?;
                draft_copies.push((*id, new));
                copies.insert(Element::Draft(new));
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
                if let Some(mut a) = s.slope {
                    a.from = a.from + d;
                    a.to = a.to + d;
                    model.set_slab_slope(new, Some(a))?;
                }
                copies.insert(Element::Slab(new));
            }
            Element::Terrain(id) => {
                let t = model
                    .terrain(*id)
                    .ok_or(ModelError::UnknownTerrain(*id))?
                    .clone();
                let new = model.add_terrain(archisim_core::NewTerrain {
                    level: t.level,
                    points: t
                        .points
                        .iter()
                        .map(|p| archisim_core::SurveyPoint {
                            at: p.at + d,
                            z: p.z,
                        })
                        .collect(),
                    material: t.material,
                    depth: t.depth,
                    contour: t.contour,
                    lines: t.lines,
                })?;
                copies.insert(Element::Terrain(new));
            }
            Element::CurtainWall(id) => {
                let c = model
                    .curtain_wall(*id)
                    .ok_or(ModelError::UnknownCurtainWall(*id))?
                    .clone();
                let new = model.add_curtain_wall(archisim_core::NewCurtainWall {
                    curtain_type: c.curtain_type,
                    level: c.level,
                    start: c.start + d,
                    end: c.end + d,
                    base_offset: c.base_offset,
                    top: c.top,
                    height: c.height,
                })?;
                copies.insert(Element::CurtainWall(new));
            }
            Element::Object(id) => {
                let o = model
                    .object(*id)
                    .ok_or(ModelError::UnknownObject(*id))?
                    .clone();
                let new = model.add_object(archisim_core::NewObject {
                    level: o.level,
                    object_type: o.object_type,
                    position: o.position + d,
                    rotation: o.rotation,
                    base_offset: o.base_offset,
                })?;
                copies.insert(Element::Object(new));
            }
            Element::Railing(id) => {
                let r = model
                    .railing(*id)
                    .ok_or(ModelError::UnknownRailing(*id))?
                    .clone();
                let new = model.add_railing(archisim_core::NewRailing {
                    level: r.level,
                    path: r.path.iter().map(|p| *p + d).collect(),
                    base_offset: r.base_offset,
                    height: r.height,
                    post_spacing: r.post_spacing,
                    material: r.material,
                    glass: r.glass,
                    railing_type: r.railing_type,
                })?;
                copies.insert(Element::Railing(new));
            }
            Element::Stair(id) => {
                let s = model
                    .stair(*id)
                    .ok_or(ModelError::UnknownStair(*id))?
                    .clone();
                let new = model.add_stair(archisim_core::NewStair {
                    level: s.level,
                    path: s.path.iter().map(|p| *p + d).collect(),
                    width: s.width,
                    going: s.going,
                    base_offset: s.base_offset,
                    top: s.top,
                    height: s.height,
                    waist: s.waist,
                    material: s.material,
                    usage: s.usage,
                    design: s.design,
                })?;
                model.edit_stair(new, |x| x.railings = s.railings)?;
                copies.insert(Element::Stair(new));
            }
            Element::Roof(id) => {
                let r = model.roof(*id).ok_or(ModelError::UnknownRoof(*id))?.clone();
                let new = model.add_roof(archisim_core::NewRoof {
                    roof_type: r.roof_type,
                    level: r.level,
                    outline: r.outline.iter().map(|p| *p + d).collect(),
                    slopes: r.slopes.clone(),
                    base_offset: r.base_offset,
                    overhang: r.overhang,
                })?;
                // Its dormers and covering go with it, each dormer new.
                model.set_roof_covering(new, r.covering.clone())?;
                model.edit_roof(new, |x| {
                    x.dormers = r
                        .dormers
                        .iter()
                        .map(|dm| archisim_core::Dormer {
                            guid: archisim_core::GlobalId::new(),
                            ..dm.clone()
                        })
                        .collect();
                })?;
                copies.insert(Element::Roof(new));
            }
            Element::Dormer(r, i) => {
                let i = usize::from(*i);
                let roof = model.roof(*r).ok_or(ModelError::UnknownRoof(*r))?;
                let dm = roof
                    .dormers
                    .get(i)
                    .ok_or(ModelError::UnknownDormer(i))?
                    .clone();
                let at = roof
                    .dormer_frame(&dm)
                    .ok_or(ModelError::DormerDoesNotFit)?
                    .origin;
                let copy = archisim_core::Dormer {
                    guid: archisim_core::GlobalId::new(),
                    ..dm
                };
                let new = model.insert_dormer(*r, copy, at + d)?;
                copies.insert(Element::Dormer(*r, new as u16));
            }
            Element::Footing(id) => {
                let f = model
                    .footing(*id)
                    .ok_or(ModelError::UnknownElement(id.0))?
                    .clone();
                let new = model.add_footing(archisim_core::NewFooting {
                    level: f.level,
                    kind: f.kind,
                    material: f.material,
                    width: f.width,
                    depth: f.depth,
                    top_offset: f.top_offset,
                })?;
                // The copy is free: it does not go under the same column.
                place_footing(model, new, &|p| p + d, 0.0, &BTreeSet::new())?;
                copies.insert(Element::Footing(new));
            }
            Element::Grid(id) => {
                let g = model
                    .grid(*id)
                    .ok_or(ModelError::UnknownElement(id.0))?
                    .clone();
                let (start, end) = (g.start + d, g.end + d);
                let name = model.next_grid_name(start, end);
                let new = model.add_grid(archisim_core::NewGrid { name, start, end })?;
                copies.insert(Element::Grid(new));
            }
            Element::RefPlane(id) => {
                let r = model
                    .ref_plane(*id)
                    .ok_or(ModelError::UnknownElement(id.0))?
                    .clone();
                let new = model.add_ref_plane("", r.start + d, r.end + d)?;
                copies.insert(Element::RefPlane(new));
            }
            Element::Module(id) => {
                let p = model
                    .module(*id)
                    .ok_or(ModelError::UnknownElement(id.0))?
                    .clone();
                let new = model.add_module(archisim_core::NewModulePlacement {
                    source: p.source,
                    level: p.level,
                    position: p.position + d,
                    rotation: p.rotation,
                    mirror: p.mirror,
                })?;
                copies.insert(Element::Module(new));
            }
            Element::Mass(id) => {
                let mut x = model
                    .mass(*id)
                    .ok_or(ModelError::UnknownElement(id.0))?
                    .clone();
                x.map_points(|p| p + d);
                let new = model.add_mass(archisim_core::NewMass {
                    name: model.next_mass_name(x.floor_height.is_some()),
                    level: x.level,
                    shape: x.shape,
                    base_offset: x.base_offset,
                    height: x.height,
                    floor_height: x.floor_height,
                    material: x.material,
                })?;
                copies.insert(Element::Mass(new));
            }
            Element::Plot(id) => {
                let x = model
                    .plot(*id)
                    .ok_or(ModelError::UnknownElement(id.0))?
                    .clone();
                let new = model.add_plot(archisim_core::NewPlot {
                    name: model.next_plot_name(),
                    level: x.level,
                    outline: x.outline.iter().map(|p| *p + d).collect(),
                    setback: x.setback,
                    reference: String::new(),
                })?;
                copies.insert(Element::Plot(new));
            }
            Element::Pool(id) => {
                let x = model
                    .pool(*id)
                    .ok_or(ModelError::UnknownElement(id.0))?
                    .clone();
                let mut spec = x.spec();
                spec.name = model.next_pool_name();
                spec.outline = x.outline.iter().map(|p| *p + d).collect();
                let new = model.add_pool(spec)?;
                copies.insert(Element::Pool(new));
            }
            Element::Road(id) => {
                let x = model
                    .road(*id)
                    .ok_or(ModelError::UnknownElement(id.0))?
                    .clone();
                let new = model.add_road(archisim_core::NewRoad {
                    name: model.next_road_name(),
                    level: x.level,
                    axis: x.axis.iter().map(|p| *p + d).collect(),
                    section: x.section,
                    top_offset: x.top_offset,
                    pavement: x.pavement,
                    kerb_material: x.kerb_material,
                    sidewalk_material: x.sidewalk_material,
                })?;
                copies.insert(Element::Road(new));
            }
            Element::Planting(id) => {
                let new = crate::xform::copy_planting(model, *id, |p| p + d)?;
                copies.insert(Element::Planting(new));
            }
            Element::Run(id) => {
                let x = model
                    .run(*id)
                    .ok_or(ModelError::UnknownElement(id.0))?
                    .clone();
                let new = model.add_run(crate::xform::run_spec(&x, |p| p + d))?;
                copies.insert(Element::Run(new));
            }
            Element::Edge(id) => {
                let x = model
                    .edge(*id)
                    .ok_or(ModelError::UnknownElement(id.0))?
                    .clone();
                let new = model.add_edge(
                    x.level,
                    x.points.iter().map(|p| *p + d).collect(),
                    x.elevation,
                )?;
                copies.insert(Element::Edge(new));
            }
            Element::Device(id) => {
                let x = model
                    .device(*id)
                    .ok_or(ModelError::UnknownElement(id.0))?
                    .clone();
                let new = model.add_device(crate::xform::device_spec(&x, |p| p + d, 0.0))?;
                copies.insert(Element::Device(new));
            }
            Element::Section(id) => {
                let s = model
                    .section(*id)
                    .ok_or(ModelError::UnknownSection(*id))?
                    .clone();
                let name = match s.kind {
                    archisim_core::SectionKind::Section => model.next_section_name(),
                    archisim_core::SectionKind::Elevation => format!("{} (copia)", s.name),
                };
                let new = model.add_section(archisim_core::NewSection {
                    name,
                    kind: s.kind,
                    start: s.start + d,
                    end: s.end + d,
                    depth: s.depth,
                })?;
                copies.insert(Element::Section(new));
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
    model.regroup_copies(&draft_copies)?;
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
        if let Element::Draft(id) = e {
            // Drafting elements are shown as drawn, not by their box.
            let k = crate::pick::paper_k();
            let new = after.draft(id).map(|d| &d.shape);
            let old = model.draft(id).map(|d| &d.shape);
            if let Some(shape) = new {
                if old != Some(shape) {
                    out.extend(crate::pick::shape_paths(shape, k));
                }
            }
            continue;
        }
        let new = outline(&later, e);
        let old = if exists(model, e) {
            outline(geo, e)
        } else {
            None
        };
        if let Some(mut poly) = new {
            if old.as_ref() != Some(&poly) {
                // Closed, to be drawn as a line.
                if let Some(first) = poly.first().copied() {
                    poly.push(first);
                }
                out.push(poly);
            }
        }
    }
    Ok(out)
}

/// The model after a drag, the level it happens on (`None`: elements seen
/// on every level, like reference planes) and the region of that level it
/// changes: where the selection was and where it goes, with what joins to
/// it. The plan draws that region of `after` over the plan, so what is
/// dragged is seen as it will be drawn. `base` is the model the plan shows
/// (with the copies of modules).
pub fn ghost<G: Geo + ?Sized>(
    geo: &G,
    base: &Model,
    selection: &BTreeSet<Element>,
    drag: &Drag,
) -> Option<(Model, Option<archisim_core::LevelId>, (Point2, Point2))> {
    let mut after = base.clone();
    apply(&mut after, selection, drag).ok()?;
    let level = level_of(geo.model(), selection, drag.grip);
    let later = crate::geo::Indexed::new(&after);
    let mut boxes: Vec<(Point2, Point2)> = Vec::new();
    for e in selection {
        boxes.extend(outline(geo, *e).and_then(|p| bounds(&p)));
        boxes.extend(outline(&later, *e).and_then(|p| bounds(&p)));
    }
    if let Some(l) = level {
        for e in crate::pick::elements_on_level(&after, l) {
            if !exists(base, e) {
                boxes.extend(outline(&later, e).and_then(|p| bounds(&p)));
            }
        }
    }
    let (mut lo, mut hi) = *boxes.first()?;
    for (a, b) in &boxes {
        lo = Point2::new(lo.x.min(a.x), lo.y.min(a.y));
        hi = Point2::new(hi.x.max(b.x), hi.y.max(b.y));
    }
    // Walls joined to what moves stretch or lose their mitre near it.
    let margin = Point2::new(0.8, 0.8);
    Some((after, level, (lo - margin, hi + margin)))
}

/// A footing moved or turned by `map` (and `turn`, radians): it leaves
/// its column or wall unless that goes with it in `selection`.
pub(crate) fn place_footing(
    model: &mut Model,
    id: archisim_core::FootingId,
    map: &dyn Fn(Point2) -> Point2,
    turn: f64,
    selection: &BTreeSet<Element>,
) -> Result<(), ModelError> {
    use archisim_core::FootingKind;
    model.edit_footing(id, |f| {
        f.kind = match f.kind {
            FootingKind::Pad {
                position,
                length,
                rotation,
                column,
            } => FootingKind::Pad {
                position: map(position),
                length,
                rotation: rotation + turn,
                column: column.filter(|c| selection.contains(&Element::Column(*c))),
            },
            FootingKind::Strip { start, end, wall } => FootingKind::Strip {
                start: map(start),
                end: map(end),
                wall: wall.filter(|w| selection.contains(&Element::Wall(*w))),
            },
            FootingKind::TieBeam { start, end } => FootingKind::TieBeam {
                start: map(start),
                end: map(end),
            },
        };
    })
}

/// Whether the element is in the model.
pub(crate) fn exists(model: &Model, e: Element) -> bool {
    match e {
        Element::Wall(id) => model.wall(id).is_some(),
        Element::Opening(id) => model.opening(id).is_some(),
        Element::Slab(id) => model.slab(id).is_some(),
        Element::Dimension(id) => model.dimension(id).is_some(),
        Element::Mark(id) => model.elevation_mark(id).is_some(),
        Element::Column(id) => model.column(id).is_some(),
        Element::Beam(id) => model.beam(id).is_some(),
        Element::Section(id) => model.section(id).is_some(),
        Element::Roof(id) => model.roof(id).is_some(),
        Element::Stair(id) => model.stair(id).is_some(),
        Element::Terrain(id) => model.terrain(id).is_some(),
        Element::CurtainWall(id) => model.curtain_wall(id).is_some(),
        Element::Railing(id) => model.railing(id).is_some(),
        Element::Object(id) => model.object(id).is_some(),
        Element::Draft(id) => model.draft(id).is_some(),
        Element::Room(id) => model.room(id).is_some(),
        Element::Tag(id) => model.tag(id).is_some(),
        Element::Footing(id) => model.footing(id).is_some(),
        Element::Grid(id) => model.grid(id).is_some(),
        Element::RefPlane(id) => model.ref_plane(id).is_some(),
        Element::Module(id) => model.module(id).is_some(),
        Element::Mass(id) => model.mass(id).is_some(),
        Element::Plot(id) => model.plot(id).is_some(),
        Element::Pool(id) => model.pool(id).is_some(),
        Element::Road(id) => model.road(id).is_some(),
        Element::Planting(id) => model.planting(id).is_some(),
        Element::Run(id) => model.run(id).is_some(),
        Element::Edge(id) => model.edge(id).is_some(),
        Element::Device(id) => model.device(id).is_some(),
        Element::Dormer(r, i) => model
            .roof(r)
            .is_some_and(|x| usize::from(i) < x.dormers.len()),
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
        Grip::SlabCorner { slab, .. }
        | Grip::SlabEdge { slab, .. }
        | Grip::SlopeEnd { slab, .. } => model.slab(slab).map(|s| s.level),
        Grip::BeamEnd { beam, .. } => model.beam(beam).map(|b| b.level),
        Grip::RoomLabel { room } => model.room(room).map(|r| r.level),
        Grip::SectionEnd { .. } => None,
        Grip::RoofCorner { roof, .. } | Grip::RoofNode { roof, .. } => {
            model.roof(roof).map(|r| r.level)
        }
        Grip::StairPoint { stair, .. } => model.stair(stair).map(|s| s.level),
        Grip::TerrainPoint { terrain, .. } => model.terrain(terrain).map(|t| t.level),
        Grip::CurtainEnd { curtain, .. } => model.curtain_wall(curtain).map(|c| c.level),
        Grip::RailingPoint { railing, .. } => model.railing(railing).map(|r| r.level),
        Grip::RoadPoint { road, .. } => model.road(road).map(|r| r.level),
        Grip::PoolCorner { pool, .. } => model.pool(pool).map(|p| p.level),
        Grip::RefPlaneEnd { .. } => None,
        Grip::DimPoint { dimension, .. } => model.dimension(dimension).map(|d| d.level),
        Grip::ObjectSize { object, .. } | Grip::ObjectTurn { object } => {
            model.object(object).map(|o| o.level)
        }
        Grip::OpeningJamb { opening, .. } => {
            model.opening(opening).and_then(|o| wall_level(o.wall))
        }
        Grip::DraftPoint { draft, .. } => draft_level(model, draft),
        Grip::Body => selection.iter().find_map(|e| match e {
            Element::Wall(id) => wall_level(*id),
            Element::Slab(id) => model.slab(*id).map(|s| s.level),
            Element::Opening(id) => model.opening(*id).and_then(|o| wall_level(o.wall)),
            Element::Dimension(id) => model.dimension(*id).map(|d| d.level),
            Element::Mark(id) => model.elevation_mark(*id).map(|m| m.level),
            Element::Column(id) => model.column(*id).map(|c| c.level),
            Element::Beam(id) => model.beam(*id).map(|b| b.level),
            Element::Section(_) => None,
            Element::Roof(id) | Element::Dormer(id, _) => model.roof(*id).map(|r| r.level),
            Element::Stair(id) => model.stair(*id).map(|s| s.level),
            Element::Terrain(id) => model.terrain(*id).map(|t| t.level),
            Element::CurtainWall(id) => model.curtain_wall(*id).map(|c| c.level),
            Element::Railing(id) => model.railing(*id).map(|r| r.level),
            Element::Object(id) => model.object(*id).map(|o| o.level),
            Element::Draft(id) => draft_level(model, *id),
            Element::Room(id) => model.room(*id).map(|r| r.level),
            Element::Tag(id) => model.tag(*id).map(|t| t.level),
            Element::Footing(id) => model.footing(*id).map(|f| f.level),
            Element::Grid(_) | Element::RefPlane(_) => None,
            Element::Module(id) => model.module(*id).map(|m| m.level),
            Element::Mass(id) => model.mass(*id).map(|m| m.level),
            Element::Plot(id) => model.plot(*id).map(|p| p.level),
            Element::Pool(id) => model.pool(*id).map(|p| p.level),
            Element::Road(id) => model.road(*id).map(|r| r.level),
            Element::Planting(id) => model.planting(*id).map(|r| r.level),
            Element::Run(id) => model.run(*id).map(|r| r.level),
            Element::Edge(id) => model.edge(*id).map(|e| e.level),
            Element::Device(id) => model.device(*id).map(|d| d.level),
        }),
    }
}

/// The level whose plan a drafting element is drawn on.
fn draft_level(model: &Model, id: archisim_core::DraftId) -> Option<archisim_core::LevelId> {
    match model.draft(id)?.view {
        archisim_core::ViewRef::Level(l) => Some(l),
        archisim_core::ViewRef::Sheet(_) | archisim_core::ViewRef::Drafting(_) => None,
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

    /// Curtain walls, railings and objects move, copy and stretch like
    /// the rest, keeping their GUIDs when moved.
    #[test]
    fn new_elements_move_copy_and_stretch() {
        let (mut m, lvl, _) = room();
        let al = m.materials().next().unwrap().id;
        let frame = archisim_core::CurtainProfile {
            width: 0.05,
            depth: 0.12,
            material: al,
        };
        let t = m
            .add_curtain_type(
                "Muro cortina",
                archisim_core::CurtainDesign::simple(1.0, 1.2, frame, 0.024, al),
            )
            .unwrap();
        let cw = m
            .add_curtain_wall(archisim_core::NewCurtainWall {
                curtain_type: t,
                level: lvl,
                start: p(1.0, 1.0),
                end: p(4.0, 1.0),
                base_offset: 0.0,
                top: archisim_core::WallTop::Unconnected,
                height: 2.5,
            })
            .unwrap();
        let rail = m
            .add_railing(archisim_core::NewRailing {
                level: lvl,
                path: vec![p(1.0, 2.0), p(4.0, 2.0), p(4.0, 3.0)],
                base_offset: 0.0,
                height: 1.0,
                post_spacing: 1.5,
                material: al,
                glass: None,
                railing_type: None,
            })
            .unwrap();
        let ot = m.object_types().next().unwrap().id;
        let obj = m
            .add_object(archisim_core::NewObject {
                level: lvl,
                object_type: ot,
                position: p(2.0, 3.0),
                rotation: 0.0,
                base_offset: 0.0,
            })
            .unwrap();
        let all = sel(&[
            Element::CurtainWall(cw),
            Element::Railing(rail),
            Element::Object(obj),
        ]);
        let guid = m.object(obj).unwrap().guid.clone();
        let mv = Drag {
            grip: Grip::Body,
            from: p(0.0, 0.0),
            to: p(0.5, 0.25),
            variant: false,
        };
        apply(&mut m, &all, &mv).unwrap();
        assert_eq!(m.curtain_wall(cw).unwrap().start, p(1.5, 1.25));
        assert_eq!(m.railing(rail).unwrap().path[2], p(4.5, 3.25));
        assert_eq!(m.object(obj).unwrap().position, p(2.5, 3.25));
        assert_eq!(m.object(obj).unwrap().guid, guid);
        // Ctrl: copies.
        let copy = Drag {
            variant: true,
            ..mv
        };
        apply(&mut m, &all, &copy).unwrap();
        assert_eq!(m.curtain_walls().count(), 2);
        assert_eq!(m.railings().count(), 2);
        assert_eq!(m.objects().count(), 2);
        // An object's width by its handle: a shared type is copied.
        let one = sel(&[Element::Object(obj)]);
        let handles = hotspots(&m, &one);
        // Right, left, front and the turning handle.
        assert_eq!(handles.len(), 4);
        assert!(matches!(
            handles[0].grip,
            Grip::ObjectSize {
                side: ObjectSide::Right,
                ..
            }
        ));
        let left = handles[1].point;
        let w0 = handles[0].point;
        let stretch = Drag {
            grip: handles[0].grip,
            from: w0,
            to: w0 + archisim_core::Vec2::new(0.2, 0.0),
            variant: false,
        };
        let before = m.object(obj).unwrap().object_type;
        apply(&mut m, &one, &stretch).unwrap();
        let after = m.object(obj).unwrap().object_type;
        assert_ne!(before, after, "the type was shared with the copy");
        let w = m.object_type(after).unwrap().values["width"].clone();
        let w0v = m.object_type(before).unwrap().values["width"].clone();
        match (w, w0v) {
            (archisim_core::Value::Number(a), archisim_core::Value::Number(b)) => {
                // It grows only towards the handle: the left side stays.
                assert!((a - b - 0.2).abs() < 1e-6, "{a} {b}")
            }
            _ => panic!(),
        }
        let left_now = hotspots(&m, &one)[1].point;
        assert!(left_now.distance(left) < 1e-6, "{left:?} {left_now:?}");
        // With Alt (the variant), on both sides about its middle.
        let middle = object_middle(&m, obj).unwrap();
        let w1 = hotspots(&m, &one)[0].point;
        let both = Drag {
            grip: Grip::ObjectSize {
                object: obj,
                side: ObjectSide::Right,
            },
            from: w1,
            to: w1 + archisim_core::Vec2::new(0.1, 0.0),
            variant: true,
        };
        apply(&mut m, &one, &both).unwrap();
        assert!(object_middle(&m, obj).unwrap().distance(middle) < 1e-6);
        // The turning handle, dragged to the right of its middle: a
        // quarter turn clockwise, about its middle.
        let turn = hotspots(&m, &one)[3];
        assert!(matches!(turn.grip, Grip::ObjectTurn { .. }));
        let middle = object_middle(&m, obj).unwrap();
        let turned = Drag {
            grip: turn.grip,
            from: turn.point,
            to: middle + archisim_core::Vec2::new(1.0, 0.02),
            variant: false,
        };
        apply(&mut m, &one, &turned).unwrap();
        let r = m.object(obj).unwrap().rotation;
        assert!((r - 1.5 * std::f64::consts::PI).abs() < 1e-6, "{r}");
        assert!(object_middle(&m, obj).unwrap().distance(middle) < 1e-6);
        // Stretch a curtain wall by its end and move a railing point.
        let ends = sel(&[Element::CurtainWall(cw), Element::Railing(rail)]);
        assert_eq!(hotspots(&m, &ends).len(), 2 + 3);
        let stretch = Drag {
            grip: Grip::CurtainEnd {
                curtain: cw,
                start: false,
            },
            from: p(4.5, 1.25),
            to: p(5.0, 1.25),
            variant: false,
        };
        apply(&mut m, &ends, &stretch).unwrap();
        assert!((m.curtain_wall(cw).unwrap().length() - 3.5).abs() < 1e-9);
        let point = Drag {
            grip: Grip::RailingPoint {
                railing: rail,
                index: 0,
            },
            from: p(1.5, 2.25),
            to: p(1.0, 2.25),
            variant: false,
        };
        apply(&mut m, &ends, &point).unwrap();
        assert_eq!(m.railing(rail).unwrap().path[0], p(1.0, 2.25));
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
    fn moving_a_wall_moves_only_it_and_the_corners_slide_on_their_walls() {
        let (mut m, _, _) = room();
        let w = walls(&m);
        // The north wall 1 m north and 1 m east: it moves; the west and
        // east walls keep their line and only lengthen to meet it.
        let drag = Drag {
            grip: Grip::Body,
            from: p(3.0, 4.0),
            to: p(4.0, 5.0),
            variant: false,
        };
        apply(&mut m, &sel(&[Element::Wall(w[1])]), &drag).unwrap();
        assert!(m.wall(w[1]).unwrap().start.approx_eq(p(1.0, 5.0)));
        assert!(m.wall(w[0]).unwrap().end.approx_eq(p(0.0, 5.0)));
        assert!(m.wall(w[0]).unwrap().start.approx_eq(p(0.0, 0.0)));
        assert!(m.wall(w[2]).unwrap().start.approx_eq(p(6.0, 5.0)));
        assert!(m.wall(w[2]).unwrap().end.approx_eq(p(6.0, 0.0)));
    }

    #[test]
    fn a_partition_on_the_axis_of_a_moved_wall_follows_it() {
        let (mut m, lvl, lib) = room();
        let w = walls(&m);
        // A partition drawn up to the axis of the north wall (y = 4), inside
        // its thickness: it is a T-join, and it follows the wall.
        let part = m
            .add_wall(NewWall {
                wall_type: lib.partition,
                level: lvl,
                start: p(3.0, 0.265),
                end: p(3.0, 4.0),
                reference: archisim_core::ReferenceLine::Center,
                height: 2.5,
                base_offset: 0.0,
                bulge: 0.0,
                top: archisim_core::WallTop::Unconnected,
            })
            .unwrap();
        let drag = Drag {
            grip: Grip::Body,
            from: p(2.0, 4.0),
            to: p(2.0, 4.5),
            variant: false,
        };
        apply(&mut m, &sel(&[Element::Wall(w[1])]), &drag).unwrap();
        assert!(m.wall(part).unwrap().end.approx_eq(p(3.0, 4.5)));
        assert!(m.wall(part).unwrap().start.approx_eq(p(3.0, 0.265)));
    }

    /// A measured point of a dimension is a handle: dragged to another
    /// point, the dimension measures to it; its line stays.
    #[test]
    fn dimension_points_are_handles() {
        let (mut m, lvl, _) = room();
        let d = m
            .add_dimension(archisim_core::NewDimension {
                level: lvl,
                points: vec![p(0.0, 0.0), p(6.0, 0.0)],
                direction: p(1.0, 0.0),
                position: p(0.0, -1.0),
            })
            .unwrap();
        let s = sel(&[Element::Dimension(d)]);
        let (grip, at) = grip_at(&m, &s, p(5.98, 0.01), 0.1).unwrap();
        assert_eq!(at, p(6.0, 0.0));
        assert_eq!(
            label(
                &m,
                &Drag {
                    grip,
                    from: at,
                    to: at,
                    variant: false
                }
            ),
            "Cambiar punto de cota"
        );
        let drag = Drag {
            grip,
            from: at,
            to: p(5.735, 0.265),
            variant: false,
        };
        apply(&mut m, &s, &drag).unwrap();
        let dim = m.dimension(d).unwrap();
        assert_eq!(dim.points[1], p(5.735, 0.265));
        assert!((dim.values()[0] - 5.735).abs() < 1e-9);
        assert!((dim.line_offset() + 1.0).abs() < 1e-12);
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
    fn dragging_a_jamb_widens_the_door_and_keeps_the_other_jamb() {
        let (mut m, _, _) = room();
        let door = m.openings().next().unwrap().id;
        let before = archisim_geom::opening_geometry(&m, door).unwrap();
        let hs = hotspots(&m, &sel(&[Element::Opening(door)]));
        assert_eq!(hs.len(), 2);
        let jamb = hs
            .iter()
            .find(|h| {
                h.grip
                    == Grip::OpeningJamb {
                        opening: door,
                        start: false,
                    }
            })
            .unwrap();
        // 20 cm further along the wall: 20 cm wider, the start jamb fixed.
        let along = before.u * 0.2;
        let drag = Drag {
            grip: jamb.grip,
            from: jamb.point,
            to: Point2::new(jamb.point.x + along.x, jamb.point.y + along.y + 0.3),
            variant: false,
        };
        let w0 = before.t1 - before.t0;
        // The door type is shared with another door: a copy is made.
        let t0 = m.opening(door).unwrap().opening_type;
        let wall = m.opening(door).unwrap().wall;
        let other = m
            .add_opening(NewOpening {
                opening_type: t0,
                wall,
                along: 0.6,
                sill: 0.0,
                opens_to: archisim_core::WallSide::Interior,
                hand: archisim_core::Hand::Left,
                setback: None,
            })
            .unwrap();
        apply(&mut m, &sel(&[Element::Opening(door)]), &drag).unwrap();
        let after = archisim_geom::opening_geometry(&m, door).unwrap();
        assert!((after.t1 - after.t0 - w0 - 0.2).abs() < 1e-9);
        assert!((after.t0 - before.t0).abs() < 1e-9);
        assert_ne!(m.opening(door).unwrap().opening_type, t0);
        assert_eq!(m.opening(other).unwrap().opening_type, t0);
        assert!((m.opening_type(t0).unwrap().width - w0).abs() < 1e-9);
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
