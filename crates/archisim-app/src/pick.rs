//! Finding elements under the cursor or inside a selection window.

use archisim_core::{LevelId, Model, OpeningId, Point2, SlabId, WallId};
use archisim_geom::{opening_geometry, wall_geometry};

/// A selectable element of the plan.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Element {
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
pub fn outline(model: &Model, element: Element) -> Option<Vec<Point2>> {
    match element {
        Element::Wall(id) => wall_geometry(model, id).map(|g| g.outline),
        Element::Opening(id) => opening_geometry(model, id).map(|g| g.footprint()),
        Element::Slab(id) => model.slab(id).map(|s| s.outline.clone()),
    }
}

/// Every selectable element on a level, in picking priority: doors and
/// windows first (they sit inside walls), then walls, then slabs.
pub fn elements_on_level(model: &Model, level: LevelId) -> Vec<Element> {
    let mut v: Vec<Element> = Vec::new();
    for w in model.walls_on_level(level) {
        v.extend(model.openings_in_wall(w.id).map(|o| Element::Opening(o.id)));
    }
    v.extend(model.walls_on_level(level).map(|w| Element::Wall(w.id)));
    v.extend(model.slabs_on_level(level).map(|s| Element::Slab(s.id)));
    v
}

/// The element under `p`, within `tolerance` metres of its outline.
///
/// Doors and windows win over their walls, and walls over slabs. Among
/// elements of the same kind, the one whose outline is closest wins.
pub fn pick(model: &Model, level: LevelId, p: Point2, tolerance: f64) -> Option<Element> {
    let rank = |e: &Element| match e {
        Element::Opening(_) => 0,
        Element::Wall(_) => 1,
        Element::Slab(_) => 2,
    };
    elements_on_level(model, level)
        .into_iter()
        .filter_map(|e| {
            let poly = outline(model, e)?;
            let inside = contains(&poly, p);
            let d = distance_to_outline(&poly, p);
            (inside || d <= tolerance).then_some((rank(&e), if inside { 0.0 } else { d }, e))
        })
        .min_by(|a, b| a.0.cmp(&b.0).then(a.1.total_cmp(&b.1)))
        .map(|(_, _, e)| e)
}

/// Elements entirely inside the window `a`–`b` (any two opposite corners).
pub fn in_window(model: &Model, level: LevelId, a: Point2, b: Point2) -> Vec<Element> {
    let (lo, hi) = (
        Point2::new(a.x.min(b.x), a.y.min(b.y)),
        Point2::new(a.x.max(b.x), a.y.max(b.y)),
    );
    let inside = |p: &Point2| p.x >= lo.x && p.x <= hi.x && p.y >= lo.y && p.y <= hi.y;
    elements_on_level(model, level)
        .into_iter()
        .filter(|e| outline(model, *e).is_some_and(|poly| poly.iter().all(inside)))
        .collect()
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
}
