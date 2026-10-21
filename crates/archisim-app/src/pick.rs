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
    Column(archisim_core::ColumnId),
    Beam(archisim_core::BeamId),
    /// A section or elevation line (seen on every level).
    Section(archisim_core::SectionId),
    /// A pitched roof, picked by its eaves.
    Roof(archisim_core::RoofId),
    /// A stair, picked by its flights and landings.
    Stair(archisim_core::StairId),
    /// The terrain, picked anywhere on it if nothing else is there.
    Terrain(archisim_core::TerrainId),
    /// A curtain wall, picked by its line.
    CurtainWall(archisim_core::CurtainWallId),
    /// A loose railing, picked by its line.
    Railing(archisim_core::RailingId),
    /// Furniture, sanitary ware…, picked by its box.
    Object(archisim_core::ObjectId),
    /// A room ("local").
    Room(archisim_core::RoomId),
    /// A tag of a door, window, wall or room.
    Tag(archisim_core::TagId),
    /// A footing or tie beam.
    Footing(archisim_core::FootingId),
    /// A setting-out grid line (seen on every level).
    Grid(archisim_core::GridId),
    /// A copy of a module, picked anywhere inside its box.
    Module(archisim_core::ModuleId),
    /// A volume of the concept phase.
    Mass(archisim_core::MassId),
    /// A plot, picked by its boundary or inside it if nothing else is.
    Plot(archisim_core::PlotId),
    Road(archisim_core::RoadId),
    /// A pipe or duct, picked by its band.
    Run(archisim_core::RunId),
    /// A light, socket, switch or piece of equipment, picked by its box.
    Device(archisim_core::DeviceId),
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
        Element::Column(id) => model
            .column(id)
            .map(|c| archisim_plan::members::column_outline(model, c)),
        Element::Beam(id) => model
            .beam(id)
            .map(|b| archisim_plan::members::beam_outline(model, b)),
        Element::Section(id) => model.section(id).map(section_outline),
        Element::Roof(id) => model.roof(id).map(|r| r.eaves()),
        Element::Stair(id) => model.stair(id).map(stair_outline),
        Element::Terrain(id) => model
            .terrain(id)
            .map(|t| t.boundary().iter().map(|i| t.points[*i].at).collect()),
        Element::CurtainWall(id) => model
            .curtain_wall(id)
            .zip(model.curtain_design(id))
            .map(|(c, d)| c.outline(d)),
        Element::Railing(id) => model.railing(id).map(railing_outline),
        Element::Object(id) => Some(model.object_outline(id)).filter(|o| !o.is_empty()),
        Element::Draft(id) => model.draft(id).map(|d| {
            let (lo, hi) = archisim_draft::bounds(&d.shape, paper_k());
            vec![lo, Point2::new(hi.x, lo.y), hi, Point2::new(lo.x, hi.y)]
        }),
        Element::Room(id) => model.room(id).map(|r| r.outline.clone()),
        Element::Tag(id) => model.tag(id).map(|t| tag_box(model, t, paper_k()).to_vec()),
        Element::Footing(id) => model.footing(id).map(|f| f.outline()),
        Element::Grid(id) => model.grid(id).map(grid_outline),
        Element::Module(id) => Some(model.module_outline(id)).filter(|o| o.len() > 2),
        Element::Mass(id) => model.mass(id).map(|m| m.outline()),
        Element::Plot(id) => model.plot(id).map(|p| p.outline.clone()),
        Element::Road(id) => model.road(id).map(|r| r.outline()),
        Element::Run(id) => model.run(id).map(|r| r.outline()),
        Element::Device(id) => model.device(id).map(|d| d.outline()),
    }
}

/// A band along a grid line, with its bubbles at both ends.
fn grid_outline(g: &archisim_core::GridLine) -> Vec<Point2> {
    let r = archisim_plan::grids::BUBBLE_RADIUS * MM();
    let u = (g.end - g.start)
        .normalized()
        .unwrap_or(Point2::new(1.0, 0.0));
    let n = u.perp_left() * r;
    let (a, b) = (g.start - u * (2.0 * r), g.end + u * (2.0 * r));
    vec![a - n, b - n, b + n, a + n]
}

/// A thin band round a railing's line, there and back.
fn railing_outline(r: &archisim_core::Railing) -> Vec<Point2> {
    let half = archisim_core::HANDRAIL_SIZE / 2.0;
    let side = |sign: f64| -> Vec<Point2> {
        r.path
            .windows(2)
            .flat_map(|w| {
                let n = (w[1] - w[0])
                    .normalized()
                    .map_or(archisim_core::Vec2::new(0.0, 0.0), |u| {
                        u.perp_left() * (half * sign)
                    });
                [w[0] + n, w[1] + n]
            })
            .collect()
    };
    let mut v = side(1.0);
    v.extend(side(-1.0).into_iter().rev());
    v
}

/// The box round a stair's flights and landings.
fn stair_outline(s: &archisim_core::Stair) -> Vec<Point2> {
    let pts: Vec<Point2> = s.outlines().into_iter().flatten().collect();
    let lo = pts.iter().fold(Point2::new(f64::MAX, f64::MAX), |a, p| {
        Point2::new(a.x.min(p.x), a.y.min(p.y))
    });
    let hi = pts.iter().fold(Point2::new(f64::MIN, f64::MIN), |a, p| {
        Point2::new(a.x.max(p.x), a.y.max(p.y))
    });
    vec![lo, Point2::new(hi.x, lo.y), hi, Point2::new(lo.x, hi.y)]
}

/// A band along a section line, wide enough to take its heads.
fn section_outline(s: &archisim_core::SectionLine) -> Vec<Point2> {
    let r = archisim_plan::section::MARKER_RADIUS * MM();
    let u = s.along().unwrap_or(Point2::new(1.0, 0.0));
    let n = u.perp_left();
    let (a, b) = match s.kind {
        archisim_core::SectionKind::Section => (s.start - u * (2.0 * r), s.end + u * (2.0 * r)),
        archisim_core::SectionKind::Elevation => (s.start, s.end),
    };
    // On the side it looks to, as far as the arrows go.
    vec![a - n * r, b - n * r, b + n * (2.0 * r), a + n * (2.0 * r)]
}

fn box_of(pts: impl IntoIterator<Item = Point2>) -> [Point2; 4] {
    let (mut lo, mut hi) = (
        Point2::new(f64::INFINITY, f64::INFINITY),
        Point2::new(f64::NEG_INFINITY, f64::NEG_INFINITY),
    );
    for p in pts {
        lo = Point2::new(lo.x.min(p.x), lo.y.min(p.y));
        hi = Point2::new(hi.x.max(p.x), hi.y.max(p.y));
    }
    [lo, Point2::new(hi.x, lo.y), hi, Point2::new(lo.x, hi.y)]
}

/// The box around a room's stamp (name, number, area).
pub fn stamp_box(model: &Model, r: &archisim_core::Room, k: f64) -> [Point2; 4] {
    box_of(
        archisim_plan::rooms::room_stamp(model, r, k)
            .iter()
            .flat_map(|t| archisim_draft::font::text_frame(t, k)),
    )
}

/// The box around a tag.
pub fn tag_box(model: &Model, t: &archisim_core::Tag, k: f64) -> [Point2; 4] {
    let l = archisim_plan::rooms::tag_layout(model, t, k);
    match l.frame {
        Some(f) => f,
        None if l.texts.is_empty() => box_of([t.at, t.at]),
        None => box_of(
            l.texts
                .iter()
                .flat_map(|x| archisim_draft::font::text_frame(x, k)),
        ),
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
        Shape::Dim(m) => {
            let d = archisim_draft::dims::drawing(m, k, archisim_draft::dims::DIM_TEXT);
            let mut out: Vec<Vec<Point2>> = d.segs.iter().map(|s| s.tessellate(1e-3)).collect();
            out.extend(d.arrows.iter().map(|a| vec![a[0], a[1], a[2], a[0]]));
            out.extend(archisim_draft::font::text_strokes(&d.text, k));
            out
        }
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
    v.extend(model.sections().map(|s| Element::Section(s.id)));
    v.extend(model.grids().map(|g| Element::Grid(g.id)));
    v.extend(model.columns_on_level(level).map(|c| Element::Column(c.id)));
    v.extend(model.walls_on_level(level).map(|w| Element::Wall(w.id)));
    v.extend(
        model
            .curtain_walls_on_level(level)
            .map(|c| Element::CurtainWall(c.id)),
    );
    v.extend(
        model
            .railings_on_level(level)
            .map(|r| Element::Railing(r.id)),
    );
    v.extend(model.objects_on_level(level).map(|o| Element::Object(o.id)));
    v.extend(model.beams_on_level(level).map(|b| Element::Beam(b.id)));
    v.extend(model.stairs_on_level(level).map(|s| Element::Stair(s.id)));
    v.extend(model.roofs_on_level(level).map(|r| Element::Roof(r.id)));
    v.extend(model.slabs_on_level(level).map(|s| Element::Slab(s.id)));
    v.extend(
        model
            .footings_on_level(level)
            .map(|f| Element::Footing(f.id)),
    );
    v.extend(
        model
            .terrains_on_level(level)
            .map(|t| Element::Terrain(t.id)),
    );
    v.extend(model.tags_on_level(level).map(|t| Element::Tag(t.id)));
    v.extend(model.rooms_on_level(level).map(|r| Element::Room(r.id)));
    v.extend(model.modules_on_level(level).map(|m| Element::Module(m.id)));
    v.extend(model.masses_on_level(level).map(|m| Element::Mass(m.id)));
    v.extend(model.roads_on_level(level).map(|r| Element::Road(r.id)));
    v.extend(model.runs_on_level(level).map(|r| Element::Run(r.id)));
    v.extend(model.devices_on_level(level).map(|d| Element::Device(d.id)));
    v.extend(model.plots_on_level(level).map(|p| Element::Plot(p.id)));
    v
}

/// What type an element is of, to find the others of the same type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TypeOf {
    Wall(archisim_core::WallTypeId),
    Slab(archisim_core::SlabTypeId),
    Opening(archisim_core::OpeningTypeId),
    Member(archisim_core::MemberTypeId),
}

/// The type of a wall, slab, door or window (annotations have none).
pub fn type_of(model: &Model, e: Element) -> Option<TypeOf> {
    match e {
        Element::Wall(id) => model.wall(id).map(|w| TypeOf::Wall(w.wall_type)),
        Element::Slab(id) => model.slab(id).map(|s| TypeOf::Slab(s.slab_type)),
        Element::Opening(id) => model.opening(id).map(|o| TypeOf::Opening(o.opening_type)),
        Element::Column(id) => model.column(id).map(|c| TypeOf::Member(c.member_type)),
        Element::Beam(id) => model.beam(id).map(|b| TypeOf::Member(b.member_type)),
        Element::Roof(id) => model.roof(id).map(|r| TypeOf::Slab(r.roof_type)),
        Element::Dimension(_)
        | Element::Mark(_)
        | Element::Room(_)
        | Element::Draft(_)
        | Element::Tag(_)
        | Element::Section(_)
        | Element::Stair(_)
        | Element::Terrain(_)
        | Element::CurtainWall(_)
        | Element::Railing(_)
        | Element::Object(_)
        | Element::Footing(_)
        | Element::Grid(_)
        | Element::Module(_)
        | Element::Mass(_)
        | Element::Plot(_)
        | Element::Road(_)
        | Element::Run(_)
        | Element::Device(_) => None,
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
        Element::Dimension(_)
        | Element::Mark(_)
        | Element::Section(_)
        | Element::Grid(_)
        | Element::Draft(_)
        | Element::Tag(_) => 0,
        Element::Opening(_) => 1,
        // Columns stand in front of the walls they go through.
        Element::Column(_) => 1,
        Element::Wall(_) | Element::CurtainWall(_) => 2,
        Element::Railing(_) | Element::Object(_) => 1,
        Element::Beam(_) => 3,
        Element::Roof(_) => 3,
        Element::Stair(_) => 1,
        Element::Room(_) => 3,
        Element::Slab(_) => 4,
        // Under the floor: after the slab.
        Element::Footing(_) => 5,
        // Its elements are not the project's: inside its box, if nothing
        // of the project is there.
        Element::Module(_) => 5,
        // In the concept phase the volumes are what is worked on.
        Element::Mass(_) => 2,
        Element::Road(_) => 5,
        // Under the ceiling, in front of the slab.
        Element::Run(_) => 2,
        Element::Device(_) => 1,
        // The ground everything stands on: last.
        Element::Plot(_) => 7,
        Element::Terrain(_) => 6,
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
            if let Element::Room(id) = e {
                // By its stamp first; from inside after walls, before
                // slabs.
                let r = model.room(id)?;
                if contains(&stamp_box(model, r, k), p) {
                    return Some((0, 0.0, e));
                }
                return r.contains(p).then_some((3, 0.0, e));
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

/// The element a solid of the 3D (or of a section drawn from them) belongs
/// to.
pub fn of_solid(owner: archisim_geom::solid::SolidOwner) -> Element {
    use archisim_geom::solid::SolidOwner;
    match owner {
        SolidOwner::Wall(id) => Element::Wall(id),
        SolidOwner::Slab(id) => Element::Slab(id),
        SolidOwner::Opening(id) => Element::Opening(id),
        SolidOwner::Column(id) => Element::Column(id),
        SolidOwner::Beam(id) => Element::Beam(id),
        SolidOwner::Roof(id) => Element::Roof(id),
        SolidOwner::Stair(id) => Element::Stair(id),
        SolidOwner::Terrain(id) => Element::Terrain(id),
        SolidOwner::CurtainWall(id) => Element::CurtainWall(id),
        SolidOwner::Railing(id) => Element::Railing(id),
        SolidOwner::Object(id) => Element::Object(id),
        SolidOwner::Footing(id) => Element::Footing(id),
        SolidOwner::Mass(id) => Element::Mass(id),
        SolidOwner::Road(id) => Element::Road(id),
        SolidOwner::Run(id) => Element::Run(id),
        SolidOwner::Device(id) => Element::Device(id),
    }
}

/// The solid owner of an element that has a body (not dimensions, marks,
/// rooms or sections).
pub fn solid_of(e: Element) -> Option<archisim_geom::solid::SolidOwner> {
    use archisim_geom::solid::SolidOwner;
    Some(match e {
        Element::Wall(id) => SolidOwner::Wall(id),
        Element::Slab(id) => SolidOwner::Slab(id),
        Element::Opening(id) => SolidOwner::Opening(id),
        Element::Column(id) => SolidOwner::Column(id),
        Element::Beam(id) => SolidOwner::Beam(id),
        Element::Roof(id) => SolidOwner::Roof(id),
        Element::Stair(id) => SolidOwner::Stair(id),
        Element::Terrain(id) => SolidOwner::Terrain(id),
        Element::CurtainWall(id) => SolidOwner::CurtainWall(id),
        Element::Railing(id) => SolidOwner::Railing(id),
        Element::Object(id) => SolidOwner::Object(id),
        Element::Footing(id) => SolidOwner::Footing(id),
        Element::Mass(id) => SolidOwner::Mass(id),
        Element::Road(id) => SolidOwner::Road(id),
        Element::Run(id) => SolidOwner::Run(id),
        Element::Device(id) => SolidOwner::Device(id),
        _ => return None,
    })
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
