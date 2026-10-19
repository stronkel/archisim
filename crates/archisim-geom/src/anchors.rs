//! Associative dimensions: the points of a dimension tied to wall corners,
//! jambs of doors and windows and slab corners, so that the dimension
//! follows them when they move, as in Revit and ArchiCAD.

use std::collections::BTreeMap;

use archisim_core::{Anchor, AnchorSide, DimensionId, LevelId, Model, Point2, WallId};

use crate::{opening_geometry, wall_geometry, WallGeometry};

/// How close a dimension point must be to a corner or jamb to be tied to it.
pub const ANCHOR_TOL: f64 = 1e-4;

/// Wall geometries computed once per change.
#[derive(Default)]
struct Cache {
    walls: BTreeMap<WallId, Option<WallGeometry>>,
}

impl Cache {
    fn wall(&mut self, model: &Model, id: WallId) -> Option<&WallGeometry> {
        self.walls
            .entry(id)
            .or_insert_with(|| wall_geometry(model, id))
            .as_ref()
    }
}

/// Where an anchor is now; `None` if its element is gone.
pub fn resolve(model: &Model, anchor: Anchor) -> Option<Point2> {
    resolve_in(model, &mut Cache::default(), anchor)
}

fn resolve_in(model: &Model, cache: &mut Cache, anchor: Anchor) -> Option<Point2> {
    match anchor {
        Anchor::WallEnd { wall, start, side } => {
            let w = model.wall(wall)?;
            if side == AnchorSide::Axis {
                return Some(if start { w.start } else { w.end });
            }
            let g = cache.wall(model, wall)?;
            let (s, e) = g.end_points();
            let end = if start { s } else { e };
            match side {
                AnchorSide::Exterior => end.first().copied(),
                _ => end.last().copied(),
            }
        }
        Anchor::Jamb { opening, end, side } => {
            let g = opening_geometry(model, opening)?;
            let t = if end { g.t1 } else { g.t0 };
            let off = match side {
                AnchorSide::Exterior => g.exterior_offset,
                AnchorSide::Axis => 0.0,
                AnchorSide::Interior => g.interior_offset,
            };
            // Curved walls: jambs are on the arc; straight ones on the line.
            match model.wall(g.wall).and_then(|w| w.arc()) {
                Some(_) => None,
                None => Some(g.point(t, off)),
            }
        }
        Anchor::SlabCorner { slab, index } => model.slab(slab)?.outline.get(index).copied(),
    }
}

/// The anchor at a point of a level, if a wall corner, a jamb or a slab
/// corner is there. Where walls meet, the wall that runs along `direction`
/// (the one whose length the dimension measures) is preferred.
pub fn find(model: &Model, level: LevelId, p: Point2, direction: Point2) -> Option<Anchor> {
    find_in(model, &mut Cache::default(), level, p, direction)
}

fn find_in(
    model: &Model,
    cache: &mut Cache,
    level: LevelId,
    p: Point2,
    direction: Point2,
) -> Option<Anchor> {
    let near = |q: Point2| q.distance(p) <= ANCHOR_TOL;
    let mut walls: Vec<(f64, WallId)> = model
        .walls_on_level(level)
        .map(|w| {
            let d = w.end - w.start;
            let along = d.normalized().map_or(0.0, |u| u.dot(direction).abs());
            (along, w.id)
        })
        .collect();
    walls.sort_by(|a, b| b.0.total_cmp(&a.0).then(a.1.cmp(&b.1)));
    let walls: Vec<WallId> = walls.into_iter().map(|w| w.1).collect();
    for &id in &walls {
        for start in [true, false] {
            for side in [AnchorSide::Exterior, AnchorSide::Interior, AnchorSide::Axis] {
                let a = Anchor::WallEnd {
                    wall: id,
                    start,
                    side,
                };
                if resolve_in(model, cache, a).is_some_and(near) {
                    return Some(a);
                }
            }
        }
    }
    for &id in &walls {
        let ops: Vec<_> = model.openings_in_wall(id).map(|o| o.id).collect();
        for o in ops {
            for end in [false, true] {
                for side in [AnchorSide::Exterior, AnchorSide::Interior, AnchorSide::Axis] {
                    let a = Anchor::Jamb {
                        opening: o,
                        end,
                        side,
                    };
                    if resolve_in(model, cache, a).is_some_and(near) {
                        return Some(a);
                    }
                }
            }
        }
    }
    for s in model.slabs_on_level(level) {
        if let Some(index) = s.outline.iter().position(|q| near(*q)) {
            return Some(Anchor::SlabCorner { slab: s.id, index });
        }
    }
    None
}

/// After a change: dimensions created or changed by it are tied to what
/// is under their points; the others follow what they are tied to.
/// `before` holds every dimension's points before the change.
pub fn follow(model: &mut Model, before: &BTreeMap<DimensionId, Vec<Point2>>) {
    let mut cache = Cache::default();
    type Row = (
        DimensionId,
        LevelId,
        Vec<Point2>,
        Vec<Option<Anchor>>,
        Point2,
        Point2,
    );
    let dims: Vec<Row> = model
        .dimensions()
        .map(|d| {
            (
                d.id,
                d.level,
                d.points.clone(),
                d.anchors.clone(),
                d.position,
                d.direction,
            )
        })
        .collect();
    for (id, level, points, anchors, position, direction) in dims {
        let edited = before.get(&id) != Some(&points);
        if edited {
            let found: Vec<Option<Anchor>> = points
                .iter()
                .map(|p| find_in(model, &mut cache, level, *p, direction))
                .collect();
            if found != anchors {
                let _ = model.set_dimension_anchors(id, found);
            }
            continue;
        }
        if anchors.is_empty() {
            continue;
        }
        let mut moved = points.clone();
        let mut kept = anchors.clone();
        for (i, a) in anchors.iter().enumerate() {
            let Some(a) = a else { continue };
            match resolve_in(model, &mut cache, *a) {
                Some(q) => moved[i] = q,
                // Its element is gone: the point stays where it was.
                None => kept[i] = None,
            }
        }
        if moved != points && model.set_dimension(id, moved, position).is_err() {
            // The points fell onto each other: the dimension is freed.
            kept = Vec::new();
        }
        if kept != anchors {
            let _ = model.set_dimension_anchors(id, kept);
        }
    }
}

/// Every dimension's points, for [`follow`].
pub fn snapshot(model: &Model) -> BTreeMap<DimensionId, Vec<Point2>> {
    model
        .dimensions()
        .map(|d| (d.id, d.points.clone()))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use archisim_core::{
        CutPattern, Layer, LayerFunction, NewDimension, NewWall, ReferenceLine, WallFunction,
        WallTop,
    };

    #[test]
    fn a_dimension_follows_the_wall_it_measures() {
        let mut m = Model::new("P");
        let lvl = m.add_level("PB", 0.0);
        let a = m.add_material("A", "brick", [0, 0, 0], CutPattern::Solid);
        let t = m
            .add_wall_type(
                "Fachada",
                WallFunction::Exterior,
                vec![Layer {
                    material: a,
                    thickness: 0.3,
                    function: LayerFunction::Structure,
                }],
            )
            .unwrap();
        let w = m
            .add_wall(NewWall {
                wall_type: t,
                level: lvl,
                start: Point2::new(0.0, 0.0),
                end: Point2::new(5.0, 0.0),
                reference: ReferenceLine::ExteriorFace,
                height: 3.0,
                base_offset: 0.0,
                bulge: 0.0,
                top: WallTop::Unconnected,
            })
            .unwrap();
        let before = snapshot(&m);
        let d = m
            .add_dimension(NewDimension {
                level: lvl,
                points: vec![Point2::new(0.0, 0.0), Point2::new(5.0, 0.0)],
                direction: Point2::new(1.0, 0.0),
                position: Point2::new(0.0, 1.0),
            })
            .unwrap();
        follow(&mut m, &before);
        assert!(m.dimension(d).unwrap().anchors.iter().all(Option::is_some));
        // The wall is stretched: the dimension measures 7 m.
        let before = snapshot(&m);
        m.move_wall(w, Point2::new(0.0, 0.0), Point2::new(7.0, 0.0))
            .unwrap();
        follow(&mut m, &before);
        let v = m.dimension(d).unwrap().values();
        assert!((v[0] - 7.0).abs() < 1e-9, "{v:?}");
        // The wall is deleted: the dimension stays, free.
        let before = snapshot(&m);
        m.remove_wall(w).unwrap();
        follow(&mut m, &before);
        assert!(m.dimension(d).unwrap().anchors.is_empty());
    }
}
