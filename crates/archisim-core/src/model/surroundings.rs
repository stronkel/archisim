//! The area of the surroundings (N8, `docs/TERRITORIO.md` §2.11): the
//! outline of what was brought round the project (a circle or a square of
//! up to 2 × 2 km). The streets that reach its edge are its connections
//! with the outside (as Outside Connections in Cities: Skylines): they are
//! cut there with their whole section and drawn with an arrow that says
//! where they go on. The map view frames it.

use super::*;

/// The most points an area keeps.
pub const AREA_POINTS: usize = 256;

/// How near the area's edge a street's end is one of its connections with
/// the outside, metres.
pub const OUTSIDE_REACH: f64 = 1.5;

/// A square area of half side `half` about `centre`, counter-clockwise.
pub fn square_area(centre: Point2, half: f64) -> Vec<Point2> {
    vec![
        Point2::new(centre.x - half, centre.y - half),
        Point2::new(centre.x + half, centre.y - half),
        Point2::new(centre.x + half, centre.y + half),
        Point2::new(centre.x - half, centre.y + half),
    ]
}

/// A round area of radius `r` about `centre`, as a polygon of 96 sides,
/// counter-clockwise.
pub fn circle_area(centre: Point2, r: f64) -> Vec<Point2> {
    (0..96)
        .map(|k| {
            let a = k as f64 / 96.0 * std::f64::consts::TAU;
            Point2::new(centre.x + r * a.cos(), centre.y + r * a.sin())
        })
        .collect()
}

/// The pieces of a line inside a convex polygon (counter-clockwise), cut
/// where it crosses the polygon's sides.
pub fn clip_line_to_area(line: &[Point2], poly: &[Point2]) -> Vec<Vec<Point2>> {
    let n = poly.len();
    if n < 3 || line.len() < 2 {
        return Vec::new();
    }
    // Each segment by Cyrus–Beck: the part of [0, 1] inside every side.
    let part = |a: Point2, b: Point2| -> Option<(f64, f64)> {
        let (mut t0, mut t1) = (0.0f64, 1.0f64);
        let d = b - a;
        for i in 0..n {
            let (p, q) = (poly[i], poly[(i + 1) % n]);
            // Inside: to the left of the side.
            let normal = (q - p).perp_left();
            let num = (a - p).dot(normal);
            let den = d.dot(normal);
            if den.abs() < 1e-12 {
                if num < 0.0 {
                    return None;
                }
                continue;
            }
            let t = -num / den;
            if den > 0.0 {
                t0 = t0.max(t);
            } else {
                t1 = t1.min(t);
            }
            if t0 > t1 {
                return None;
            }
        }
        Some((t0, t1))
    };
    let mut out: Vec<Vec<Point2>> = Vec::new();
    let mut cur: Vec<Point2> = Vec::new();
    for w in line.windows(2) {
        let (a, b) = (w[0], w[1]);
        match part(a, b) {
            Some((t0, t1)) => {
                let (p0, p1) = (a + (b - a) * t0, a + (b - a) * t1);
                if t0 > 1e-9 && !cur.is_empty() {
                    out.push(std::mem::take(&mut cur));
                }
                if cur.is_empty() {
                    cur.push(p0);
                }
                cur.push(p1);
                if t1 < 1.0 - 1e-9 {
                    out.push(std::mem::take(&mut cur));
                }
            }
            None => {
                if cur.len() >= 2 {
                    out.push(std::mem::take(&mut cur));
                }
                cur.clear();
            }
        }
    }
    if cur.len() >= 2 {
        out.push(cur);
    }
    out.retain(|p| p.len() >= 2);
    out
}

/// A connection of a street with the outside: the end of a street on the
/// area's edge.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct OutsideEnd {
    pub road: RoadId,
    /// Its end: the start of the axis (false) or its end (true).
    pub at_end: bool,
    pub point: Point2,
    /// Out of the area, along the street.
    pub out: Vec2,
}

impl Model {
    /// The area of the surroundings, if one was brought.
    /// The streets cut to the area of the surroundings (N8), when it
    /// changes: what is out of it goes; a street that leaves it and comes
    /// back in is kept in pieces. Streets with stretches, changes of
    /// section, a profile or hidden pieces of their own, or drawn freely,
    /// are left as they are. Returns how many were cut or taken away.
    pub fn clip_roads_to_surroundings(&mut self) -> Result<usize, ModelError> {
        let Some(area) = self.surroundings.clone() else {
            return Ok(0);
        };
        let roads: Vec<crate::Road> = self
            .roads()
            .filter(|r| {
                !r.free
                    && r.changes.is_empty()
                    && r.profile.is_empty()
                    && r.skins.is_empty()
                    && r.hidden.is_empty()
            })
            .cloned()
            .collect();
        let mut n = 0;
        for r in roads {
            let parts: Vec<Vec<Point2>> = clip_line_to_area(&r.axis, &area)
                .into_iter()
                .filter(|p| p.windows(2).map(|w| w[0].distance(w[1])).sum::<f64>() > 1.0)
                .collect();
            if parts.len() == 1
                && parts[0].len() == r.axis.len()
                && parts[0]
                    .iter()
                    .zip(&r.axis)
                    .all(|(a, b)| a.distance(*b) < 1e-6)
            {
                continue;
            }
            n += 1;
            let mut parts = parts.into_iter();
            match parts.next() {
                None => {
                    self.remove_road(r.id)?;
                }
                Some(first) => {
                    self.edit_road(r.id, |x| x.axis = first)?;
                    for axis in parts {
                        let id = self.add_road(crate::NewRoad {
                            name: r.name.clone(),
                            level: r.level,
                            axis,
                            section: r.section.clone(),
                            top_offset: r.top_offset,
                            profile: Vec::new(),
                            changes: Vec::new(),
                            pavement: r.pavement,
                            kerb_material: r.kerb_material,
                            sidewalk_material: r.sidewalk_material,
                        })?;
                        let _ = id;
                    }
                }
            }
        }
        Ok(n)
    }

    pub fn surroundings(&self) -> Option<&[Point2]> {
        self.surroundings.as_deref()
    }

    /// Sets (or takes away) the area of the surroundings.
    pub fn set_surroundings(&mut self, outline: Option<Vec<Point2>>) -> Result<(), ModelError> {
        if let Some(o) = &outline {
            if o.len() < 3 || o.len() > AREA_POINTS {
                return Err(ModelError::InvalidShape(
                    "the area of the surroundings needs 3 to 256 points",
                ));
            }
            if o.iter().any(|p| !p.x.is_finite() || !p.y.is_finite()) {
                return Err(ModelError::NotFinite("area of the surroundings"));
            }
            if signed_area(o).abs() < 1.0 {
                return Err(ModelError::InvalidShape(
                    "the area of the surroundings is empty",
                ));
            }
        }
        self.surroundings = outline.map(|mut o| {
            if signed_area(&o) < 0.0 {
                o.reverse();
            }
            o
        });
        Ok(())
    }

    /// The connections of a level's streets with the outside: their ends
    /// on the edge of the area of the surroundings.
    pub fn outside_ends(&self, level: LevelId) -> Vec<OutsideEnd> {
        let Some(area) = self.surroundings() else {
            return Vec::new();
        };
        let near_edge = |p: Point2| {
            (0..area.len()).any(|i| {
                let (a, b) = (area[i], area[(i + 1) % area.len()]);
                distance_to_segment(p, a, b) <= OUTSIDE_REACH
            })
        };
        let mut out = Vec::new();
        for r in self.roads_on_level(level) {
            let n = r.axis.len();
            if n < 2 {
                continue;
            }
            for (at_end, p, q) in [
                (false, r.axis[0], r.axis[1]),
                (true, r.axis[n - 1], r.axis[n - 2]),
            ] {
                if !near_edge(p) {
                    continue;
                }
                let Some(dir) = (p - q).normalized() else {
                    continue;
                };
                // Pointing out of the area.
                if crate::point_in_polygon(p + dir * (OUTSIDE_REACH + 1.0), area) {
                    continue;
                }
                out.push(OutsideEnd {
                    road: r.id,
                    at_end,
                    point: p,
                    out: dir,
                });
            }
        }
        out
    }
}

fn distance_to_segment(p: Point2, a: Point2, b: Point2) -> f64 {
    let d = b - a;
    let len2 = d.dot(d);
    if len2 < 1e-12 {
        return p.distance(a);
    }
    let t = ((p - a).dot(d) / len2).clamp(0.0, 1.0);
    p.distance(a + d * t)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn streets_already_there_are_cut_to_a_new_area() {
        let mut m = Model::new("Área");
        let level = m.add_level("Planta baja", 0.0);
        let mat = m.add_material("Asfalto", "asphalt", [70, 70, 70], crate::CutPattern::Solid);
        let road = |m: &mut Model, axis: Vec<Point2>| {
            m.add_road(crate::NewRoad {
                name: "Calle".into(),
                level,
                axis,
                section: crate::RoadSection::default(),
                top_offset: 0.0,
                profile: Vec::new(),
                changes: Vec::new(),
                pavement: mat,
                kerb_material: mat,
                sidewalk_material: mat,
            })
            .unwrap()
        };
        let across = road(
            &mut m,
            vec![Point2::new(-300.0, 0.0), Point2::new(300.0, 0.0)],
        );
        let out = road(
            &mut m,
            vec![Point2::new(400.0, 400.0), Point2::new(500.0, 400.0)],
        );
        let inside = road(
            &mut m,
            vec![Point2::new(-10.0, 10.0), Point2::new(10.0, 10.0)],
        );
        m.set_surroundings(Some(square_area(Point2::new(0.0, 0.0), 100.0)))
            .unwrap();
        assert_eq!(m.clip_roads_to_surroundings().unwrap(), 2);
        let a = m.road(across).unwrap();
        assert!((a.axis[0].x + 100.0).abs() < 1e-6 && (a.axis[1].x - 100.0).abs() < 1e-6);
        assert!(m.road(out).is_none());
        assert_eq!(m.road(inside).unwrap().axis.len(), 2);
        assert_eq!(m.clip_roads_to_surroundings().unwrap(), 0);
    }

    #[test]
    fn lines_are_cut_at_the_area() {
        let area = square_area(Point2::new(0.0, 0.0), 100.0);
        // Through, from outside to outside.
        let pieces = clip_line_to_area(
            &[Point2::new(-150.0, 10.0), Point2::new(150.0, 10.0)],
            &area,
        );
        assert_eq!(pieces.len(), 1);
        assert!((pieces[0][0].x + 100.0).abs() < 1e-9);
        assert!((pieces[0][1].x - 100.0).abs() < 1e-9);
        // Out and back in: two pieces.
        let line = [
            Point2::new(0.0, 0.0),
            Point2::new(0.0, 150.0),
            Point2::new(50.0, 150.0),
            Point2::new(50.0, 0.0),
        ];
        let pieces = clip_line_to_area(&line, &area);
        assert_eq!(pieces.len(), 2);
        assert!((pieces[0][1].y - 100.0).abs() < 1e-9);
        assert!((pieces[1][0].y - 100.0).abs() < 1e-9);
        // Wholly outside: none.
        assert!(
            clip_line_to_area(&[Point2::new(200.0, 0.0), Point2::new(300.0, 0.0)], &area)
                .is_empty()
        );
        // A circle.
        let round = circle_area(Point2::new(0.0, 0.0), 50.0);
        let pieces = clip_line_to_area(&[Point2::new(-80.0, 0.0), Point2::new(80.0, 0.0)], &round);
        assert!((pieces[0][0].x + 50.0).abs() < 0.1);
    }

    #[test]
    fn streets_on_the_edge_go_on_outside() {
        let mut m = Model::new("Área");
        let level = m.add_level("Planta baja", 0.0);
        let mat = m.add_material("Asfalto", "asphalt", [60, 60, 60], CutPattern::Solid);
        let road = |m: &mut Model, a: Point2, b: Point2| {
            m.add_road(NewRoad {
                name: "Calle".into(),
                level,
                axis: vec![a, b],
                section: RoadSection::default(),
                top_offset: 0.0,
                profile: Vec::new(),
                changes: Vec::new(),
                pavement: mat,
                kerb_material: mat,
                sidewalk_material: mat,
            })
            .unwrap()
        };
        let through = road(&mut m, Point2::new(-100.0, 0.0), Point2::new(100.0, 0.0));
        let inside = road(&mut m, Point2::new(0.0, -50.0), Point2::new(0.0, 50.0));
        assert!(m.outside_ends(level).is_empty(), "no area, no connections");
        assert!(m
            .set_surroundings(Some(vec![Point2::new(0.0, 0.0)]))
            .is_err());
        m.set_surroundings(Some(square_area(Point2::new(0.0, 0.0), 100.0)))
            .unwrap();
        let ends = m.outside_ends(level);
        assert_eq!(ends.len(), 2);
        assert!(ends.iter().all(|e| e.road == through));
        assert!(!ends.iter().any(|e| e.road == inside));
        let east = ends.iter().find(|e| e.at_end).unwrap();
        assert!(east.out.x > 0.99);
    }
}
