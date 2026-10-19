//! Finding elements under the cursor or inside a selection window.

use crate::geo::Geo;
use archisim_core::{
    DimensionId, DraftId, ElevationMarkId, LevelId, Model, OpeningId, Point2, Shape, SlabId,
    ViewRef, WallId,
};
use archisim_geom::opening_geometry;

/// A selectable element of the plan.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Element {
    /// A dimension (annotation of the plan).
    Dimension(DimensionId),
    /// A level mark (annotation of the plan).
    Mark(ElevationMarkId),
    /// A drafting element: line, arc, text, hatch…
    Draft(DraftId),
    Opening(OpeningId),
    Wall(WallId),
    Slab(SlabId),
}

/// Whether a closed polygon contains a point (ray casting).
pub fn contains(poly: &[Point2], p: Point2) -> bool {
    let mut inside = false;
    let n = poly.len();
    for i in 0..n {
        let (a, b) = (poly[i], poly[(i + 1) % n]);
        if (a.y > p.y) != (b.y > p.y) {
            let x = a.x + (p.y - a.y) / (b.y - a.y) * (b.x - a.x);
            if p.x < x {
                inside = !inside;
            }
        }
    }
    inside
}

/// The point of segment `a`–`b` closest to `p`.
pub fn nearest_on_segment(a: Point2, b: Point2, p: Point2) -> Point2 {
    let d = b - a;
    let len2 = d.dot(d);
    if len2 < 1e-18 {
        return a;
    }
    let t = ((p - a).dot(d) / len2).clamp(0.0, 1.0);
    a + d * t
}

/// Distance from `p` to the outline of a closed polygon.
pub fn distance_to_outline(poly: &[Point2], p: Point2) -> f64 {
    let n = poly.len();
    (0..n)
        .map(|i| nearest_on_segment(poly[i], poly[(i + 1) % n], p).distance(p))
        .fold(f64::INFINITY, f64::min)
}

/// Plan outline of an element, as drawn: walls with their joins, doors and
/// windows by their rough opening, slabs by their contour.
pub fn outline<G: Geo + ?Sized>(geo: &G, element: Element) -> Option<Vec<Point2>> {
    let model = geo.model();
    match element {
        Element::Wall(id) => geo.wall_geometry(id).map(|g| g.outline.clone()),
        Element::Opening(id) => opening_geometry(model, id).map(|g| g.footprint()),
        Element::Slab(id) => model.slab(id).map(|s| s.outline.clone()),
        Element::Dimension(id) => model.dimension(id).map(dimension_outline),
        Element::Mark(id) => model.elevation_mark(id).map(|m| mark_outline(model, m)),
        Element::Draft(id) => model.draft(id).map(|d| {
            let (lo, hi) = archisim_draft::bounds(&d.shape, paper_k());
            vec![lo, Point2::new(hi.x, lo.y), hi, Point2::new(lo.x, hi.y)]
        }),
    }
}

thread_local! {
    /// Metres of the plan per paper millimetre in the view being used.
    static PAPER_K: std::cell::Cell<f64> = const { std::cell::Cell::new(0.05) };
}

/// Sets the scale of the active plan view (50 for 1:50), so annotations
/// and texts are picked by their size on screen.
pub fn set_scale(scale: f64) {
    PAPER_K.with(|k| k.set(scale / 1000.0));
}

/// Metres of the plan per paper millimetre in the active view.
pub fn paper_k() -> f64 {
    PAPER_K.with(|k| k.get())
}

/// What is drawn of a drafting element, as polylines in the plan: its
/// lines, the boxes of texts, the boundaries of hatches.
pub fn draft_paths(model: &Model, id: DraftId) -> Vec<Vec<Point2>> {
    let Some(d) = model.draft(id) else {
        return Vec::new();
    };
    shape_paths(&d.shape, paper_k())
}

/// The polylines of a shape (texts as their box; points as a cross).
pub fn shape_paths(shape: &Shape, k: f64) -> Vec<Vec<Point2>> {
    match shape {
        Shape::Text(t) => {
            let mut f = archisim_draft::font::text_frame(t, k).to_vec();
            f.push(f[0]);
            vec![f]
        }
        Shape::Point { at } => {
            let r = 0.8 * k;
            vec![
                vec![*at - Point2::new(r, 0.0), *at + Point2::new(r, 0.0)],
                vec![*at - Point2::new(0.0, r), *at + Point2::new(0.0, r)],
            ]
        }
        Shape::Ellipse {
            centre,
            axis,
            ratio,
            start,
            sweep,
        } => vec![archisim_draft::ellipse_points(
            *centre, *axis, *ratio, *start, *sweep, 1e-3,
        )],
        Shape::Hatch(h) => h
            .loops
            .iter()
            .map(|l| {
                let mut p = archisim_draft::loop_polygon(l, 1e-3);
                if let Some(f) = p.first().copied() {
                    p.push(f);
                }
                p
            })
            .collect(),
        _ => {
            let segs = archisim_draft::shape_segs(shape);
            let mut out: Vec<Vec<Point2>> = Vec::new();
            for s in segs {
                let pts = s.tessellate(1e-3);
                match out.last_mut() {
                    Some(last) if last.last().is_some_and(|q| q.approx_eq(pts[0])) => {
                        last.extend_from_slice(&pts[1..]);
                    }
                    _ => out.push(pts),
                }
            }
            out
        }
    }
}

/// Paper millimetres in metres at the active view's scale.
#[allow(non_snake_case)]
fn MM() -> f64 {
    paper_k()
}

/// A thin band around the dimension line and its texts.
fn dimension_outline(d: &archisim_core::Dimension) -> Vec<Point2> {
    let g = archisim_plan::annotate::dimension_lines(d, MM());
    let (a, b) = g.line;
    let u = (b - a).normalized().unwrap_or(Point2::new(1.0, 0.0));
    let n = u.perp_left();
    let below = 1.0 * MM();
    let above = (archisim_plan::annotate::TEXT_HEIGHT + 1.5) * MM();
    vec![a - n * below, b - n * below, b + n * above, a + n * above]
}

/// The box of a level mark: its triangle and its text.
fn mark_outline(model: &Model, m: &archisim_core::ElevationMark) -> Vec<Point2> {
    let value = model.floor_elevation_at(m.level, m.point).unwrap_or(0.0);
    let text = archisim_plan::annotate::format_elevation(value);
    let w =
        archisim_plan::lettering::text_width(&text, archisim_plan::annotate::TEXT_HEIGHT * MM());
    let p = m.point;
    let (x0, x1) = (p.x - 1.6 * MM(), p.x + 2.5 * MM() + w);
    let (y0, y1) = (p.y - 0.3 * MM(), p.y + 6.0 * MM());
    vec![
        Point2::new(x0, y0),
        Point2::new(x1, y0),
        Point2::new(x1, y1),
        Point2::new(x0, y1),
    ]
}

/// Every selectable element on a level, in picking priority: doors and
/// windows first (they sit inside walls), then walls, then slabs.
pub fn elements_on_level(model: &Model, level: LevelId) -> Vec<Element> {
    let mut v: Vec<Element> = Vec::new();
    v.extend(
        model
            .drafts_in(ViewRef::Level(level))
            .filter(|d| model.draft_editable(d))
            .map(|d| Element::Draft(d.id)),
    );
    v.extend(
        model
            .elevation_marks_on_level(level)
            .map(|m| Element::Mark(m.id)),
    );
    v.extend(
        model
            .dimensions_on_level(level)
            .map(|d| Element::Dimension(d.id)),
    );
    // Doors and windows by wall (going through them once: there may be
    // thousands).
    let mut by_wall: std::collections::HashMap<WallId, Vec<Element>> = Default::default();
    for o in model.openings() {
        by_wall
            .entry(o.wall)
            .or_default()
            .push(Element::Opening(o.id));
    }
    for w in model.walls_on_level(level) {
        v.extend(by_wall.remove(&w.id).unwrap_or_default());
    }
    v.extend(model.walls_on_level(level).map(|w| Element::Wall(w.id)));
    v.extend(model.slabs_on_level(level).map(|s| Element::Slab(s.id)));
    v
}

/// What type an element is of, to find the others of the same type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TypeOf {
    Wall(archisim_core::WallTypeId),
    Slab(archisim_core::SlabTypeId),
    Opening(archisim_core::OpeningTypeId),
}

/// The type of a wall, slab, door or window (annotations have none).
pub fn type_of(model: &Model, e: Element) -> Option<TypeOf> {
    match e {
        Element::Wall(id) => model.wall(id).map(|w| TypeOf::Wall(w.wall_type)),
        Element::Slab(id) => model.slab(id).map(|s| TypeOf::Slab(s.slab_type)),
        Element::Opening(id) => model.opening(id).map(|o| TypeOf::Opening(o.opening_type)),
        Element::Dimension(_) | Element::Mark(_) | Element::Draft(_) => None,
    }
}

/// The elements of a level of type `t` (Revit's "select all instances
/// visible in view").
pub fn of_type_on_level(model: &Model, level: LevelId, t: TypeOf) -> Vec<Element> {
    elements_on_level(model, level)
        .into_iter()
        .filter(|e| type_of(model, *e) == Some(t))
        .collect()
}

/// The element under `p`, within `tolerance` metres of its outline.
///
/// Doors and windows win over their walls, and walls over slabs. Among
/// elements of the same kind, the one whose outline is closest wins.
pub fn pick<G: Geo + ?Sized>(
    geo: &G,
    level: LevelId,
    p: Point2,
    tolerance: f64,
) -> Option<Element> {
    let model = geo.model();
    let rank = |e: &Element| match e {
        Element::Dimension(_) | Element::Mark(_) | Element::Draft(_) => 0,
        Element::Opening(_) => 1,
        Element::Wall(_) => 2,
        Element::Slab(_) => 4,
    };
    let k = paper_k();
    elements_on_level(model, level)
        .into_iter()
        .filter_map(|e| {
            if let Element::Draft(id) = e {
                // Lines by their lines; a hatch also from inside (after
                // walls, before slabs).
                let d = model.draft(id)?;
                let dist = archisim_draft::distance(&d.shape, p, k);
                let edge = match &d.shape {
                    Shape::Hatch(h) => h
                        .loops
                        .iter()
                        .flat_map(archisim_draft::polyline_segs)
                        .map(|s| s.distance(p))
                        .fold(f64::INFINITY, f64::min),
                    _ => dist,
                };
                if edge <= tolerance {
                    return Some((0, edge, e));
                }
                return (dist <= tolerance).then_some((3, 0.0, e));
            }
            let poly = outline(geo, e)?;
            // A hole of a slab is not the slab.
            let inside = match e {
                Element::Slab(id) => model.slab(id).is_some_and(|s| s.covers(p)),
                _ => contains(&poly, p),
            };
            let d = distance_to_outline(&poly, p);
            (inside || d <= tolerance).then_some((rank(&e), if inside { 0.0 } else { d }, e))
        })
        .min_by(|a, b| a.0.cmp(&b.0).then(a.1.total_cmp(&b.1)))
        .map(|(_, _, e)| e)
}

/// Elements found by a selection window from `a` to `b`, as in AutoCAD:
/// dragged from left to right, those entirely inside it; from right to
/// left (a crossing window), also those it touches.
pub fn in_window<G: Geo + ?Sized>(geo: &G, level: LevelId, a: Point2, b: Point2) -> Vec<Element> {
    let model = geo.model();
    let crossing = b.x < a.x;
    let (lo, hi) = (
        Point2::new(a.x.min(b.x), a.y.min(b.y)),
        Point2::new(a.x.max(b.x), a.y.max(b.y)),
    );
    let k = paper_k();
    let inside = |p: &Point2| p.x >= lo.x && p.x <= hi.x && p.y >= lo.y && p.y <= hi.y;
    elements_on_level(model, level)
        .into_iter()
        .filter(|e| {
            if let Element::Draft(id) = e {
                return model.draft(*id).is_some_and(|d| {
                    if crossing {
                        archisim_draft::crosses_window(&d.shape, lo, hi, k)
                    } else {
                        archisim_draft::inside_window(&d.shape, lo, hi, k)
                    }
                });
            }
            outline(geo, *e).is_some_and(|poly| {
                poly.iter().all(inside) || (crossing && polygon_touches(&poly, lo, hi))
            })
        })
        .collect()
}

/// The drafting element of a sheet under `p` (texts, lines and hatches
/// drawn on the paper).
pub fn pick_on_sheet(
    model: &Model,
    sheet: archisim_core::SheetId,
    p: Point2,
    tolerance: f64,
) -> Option<Element> {
    let k = paper_k();
    model
        .drafts_in(ViewRef::Sheet(sheet))
        .filter(|d| model.draft_editable(d))
        .filter_map(|d| {
            let dist = archisim_draft::distance(&d.shape, p, k);
            let edge = match &d.shape {
                Shape::Hatch(h) => h
                    .loops
                    .iter()
                    .flat_map(archisim_draft::polyline_segs)
                    .map(|s| s.distance(p))
                    .fold(f64::INFINITY, f64::min),
                _ => dist,
            };
            if edge <= tolerance {
                Some((0, edge, Element::Draft(d.id)))
            } else {
                (dist <= tolerance).then_some((1, 0.0, Element::Draft(d.id)))
            }
        })
        .min_by(|a, b| a.0.cmp(&b.0).then(a.1.total_cmp(&b.1)))
        .map(|(_, _, e)| e)
}

/// [`in_window`] for what is drawn on a sheet.
pub fn in_window_on_sheet(
    model: &Model,
    sheet: archisim_core::SheetId,
    a: Point2,
    b: Point2,
) -> Vec<Element> {
    let crossing = b.x < a.x;
    let (lo, hi) = (
        Point2::new(a.x.min(b.x), a.y.min(b.y)),
        Point2::new(a.x.max(b.x), a.y.max(b.y)),
    );
    let k = paper_k();
    model
        .drafts_in(ViewRef::Sheet(sheet))
        .filter(|d| model.draft_editable(d))
        .filter(|d| {
            if crossing {
                archisim_draft::crosses_window(&d.shape, lo, hi, k)
            } else {
                archisim_draft::inside_window(&d.shape, lo, hi, k)
            }
        })
        .map(|d| Element::Draft(d.id))
        .collect()
}

/// Whether a closed polygon touches a box: a corner inside it, the box
/// inside the polygon, or an edge crossing it.
fn polygon_touches(poly: &[Point2], lo: Point2, hi: Point2) -> bool {
    let inside = |p: &Point2| p.x >= lo.x && p.x <= hi.x && p.y >= lo.y && p.y <= hi.y;
    if poly.iter().any(inside) || contains(poly, (lo + hi) * 0.5) {
        return true;
    }
    let rect = [lo, Point2::new(hi.x, lo.y), hi, Point2::new(lo.x, hi.y)];
    let n = poly.len();
    (0..n).any(|i| {
        let e = archisim_draft::Seg::Line {
            a: poly[i],
            b: poly[(i + 1) % n],
        };
        (0..4).any(|j| {
            let r = archisim_draft::Seg::Line {
                a: rect[j],
                b: rect[(j + 1) % 4],
            };
            !archisim_draft::intersect(&e, &r).is_empty()
        })
    })
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use archisim_core::{Hand, NewOpening, NewSlab, NewWall, ReferenceLine, WallSide};
    use archisim_library::{add_library, Library};

    /// A 6 × 4 m room drawn clockwise along its exterior faces, a door in
    /// the south wall and a slab under it.
    pub(crate) fn room() -> (Model, LevelId, Library) {
        let mut m = Model::new("Prueba");
        let lvl = m.add_level("Planta baja", 0.0);
        let lib = add_library(&mut m).unwrap();
        let c = [
            Point2::new(0.0, 0.0),
            Point2::new(0.0, 4.0),
            Point2::new(6.0, 4.0),
            Point2::new(6.0, 0.0),
        ];
        for i in 0..4 {
            m.add_wall(NewWall {
                wall_type: lib.facade,
                level: lvl,
                start: c[i],
                end: c[(i + 1) % 4],
                reference: ReferenceLine::ExteriorFace,
                height: 3.0,
                base_offset: 0.0,
                bulge: 0.0,
                top: archisim_core::WallTop::Unconnected,
            })
            .unwrap();
        }
        let south = m.walls().last().unwrap().id;
        m.add_opening(NewOpening {
            opening_type: lib.interior_door,
            wall: south,
            along: 3.0,
            sill: 0.0,
            opens_to: WallSide::Interior,
            hand: Hand::Left,
            setback: None,
        })
        .unwrap();
        m.add_slab(NewSlab {
            slab_type: lib.ground_slab,
            level: lvl,
            outline: c.to_vec(),
            top_offset: 0.0,
        })
        .unwrap();
        (m, lvl, lib)
    }

    #[test]
    fn picking_priority() {
        let (m, lvl, _) = room();
        let walls: Vec<_> = m.walls().map(|w| w.id).collect();
        let door = m.openings().next().unwrap().id;
        let slab = m.slabs().next().unwrap().id;
        // Inside the west wall (exterior face at x = 0, 26.5 cm thick).
        assert_eq!(
            pick(&m, lvl, Point2::new(0.1, 2.0), 0.01),
            Some(Element::Wall(walls[0]))
        );
        // On the door in the south wall (the south wall runs east → west).
        assert_eq!(
            pick(&m, lvl, Point2::new(3.0, 0.1), 0.01),
            Some(Element::Opening(door))
        );
        // In the middle of the room: the slab.
        assert_eq!(
            pick(&m, lvl, Point2::new(3.0, 2.0), 0.01),
            Some(Element::Slab(slab))
        );
        // Just outside the building, within tolerance of the west wall.
        assert_eq!(
            pick(&m, lvl, Point2::new(-0.05, 2.0), 0.1),
            Some(Element::Wall(walls[0]))
        );
        assert_eq!(pick(&m, lvl, Point2::new(-1.0, 2.0), 0.1), None);
    }

    #[test]
    fn window_selection_takes_whole_elements() {
        let (m, lvl, _) = room();
        // The west wall and nothing else.
        let got = in_window(&m, lvl, Point2::new(-0.5, -0.5), Point2::new(0.5, 4.5));
        assert_eq!(got, vec![Element::Wall(m.walls().next().unwrap().id)]);
        // Everything.
        let all = in_window(&m, lvl, Point2::new(7.0, 5.0), Point2::new(-1.0, -1.0));
        assert_eq!(all.len(), 6);
    }

    #[test]
    fn polygon_helpers() {
        let sq = [
            Point2::new(0.0, 0.0),
            Point2::new(2.0, 0.0),
            Point2::new(2.0, 2.0),
            Point2::new(0.0, 2.0),
        ];
        assert!(contains(&sq, Point2::new(1.0, 1.0)));
        assert!(!contains(&sq, Point2::new(3.0, 1.0)));
        assert!((distance_to_outline(&sq, Point2::new(3.0, 1.0)) - 1.0).abs() < 1e-12);
        assert_eq!(
            nearest_on_segment(sq[0], sq[1], Point2::new(5.0, 3.0)),
            Point2::new(2.0, 0.0)
        );
    }

    #[test]
    fn elements_of_the_same_type() {
        let (mut m, lvl, lib) = room();
        // A partition across the room: another wall type.
        let partition = m
            .add_wall(NewWall {
                wall_type: lib.partition,
                level: lvl,
                start: Point2::new(3.0, 0.265),
                end: Point2::new(3.0, 3.735),
                reference: ReferenceLine::Center,
                height: 3.0,
                base_offset: 0.0,
                bulge: 0.0,
                top: archisim_core::WallTop::Unconnected,
            })
            .unwrap();
        let west = m.walls().next().unwrap().id;
        let facades = of_type_on_level(&m, lvl, type_of(&m, Element::Wall(west)).unwrap());
        assert_eq!(facades.len(), 4);
        assert!(!facades.contains(&Element::Wall(partition)));
        assert_eq!(
            of_type_on_level(&m, lvl, TypeOf::Wall(lib.partition)),
            vec![Element::Wall(partition)]
        );
        let door = m.openings().next().unwrap().id;
        assert_eq!(
            of_type_on_level(&m, lvl, type_of(&m, Element::Opening(door)).unwrap()),
            vec![Element::Opening(door)]
        );
        assert_eq!(
            of_type_on_level(&m, lvl, TypeOf::Slab(lib.ground_slab)).len(),
            1
        );
        assert_eq!(
            type_of(&m, Element::Dimension(archisim_core::DimensionId(1))),
            None
        );
    }
}
