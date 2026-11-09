//! The geometry of walls of variable section (batters and footing, see
//! `archisim_core::WallSection`): the wall's plan at any height, with its
//! battered faces leaning out by what they lean there; the footing's
//! outline; the section across, as a polygon.
//!
//! The wall at a height is [`crate::wall_geometry_with`] with each layer
//! boundary moved across: those on the exterior side of the layer that
//! grows by what the exterior face leans out, those on the interior side
//! by what the interior face does. Its corners slide along the lines its
//! ends are cut along (a mitre, a tee), so two battered walls meeting at a
//! corner still meet at every height, and since each corner moves along a
//! straight line as the height changes, the solid between two heights is
//! a loft with plane faces.

use archisim_core::{Model, Point2, WallId, WallSection};

use crate::{wall_geometry_in, wall_geometry_with, WallGeometry, WallIndex};

/// What the geometry of a wall of variable section needs.
#[derive(Debug, Clone, PartialEq)]
pub struct SectionFrame {
    pub section: WallSection,
    /// The layer that grows.
    pub layer: usize,
    pub layers: usize,
    /// The wall's height (the crown is there).
    pub height: f64,
    /// Thickness at the crown (the type's).
    pub crown: f64,
    /// The base of the wall, above project zero.
    pub z0: f64,
}

impl SectionFrame {
    /// The frame of a wall of variable section (`None` if it is plain).
    pub fn of(model: &Model, id: WallId) -> Option<Self> {
        let section = model.wall_section(id)?;
        let w = model.wall(id)?;
        let t = model.wall_type(w.wall_type)?;
        let level = model.level(w.level).map_or(0.0, |l| l.elevation);
        Some(SectionFrame {
            layer: section.varying_layer(&t.layers),
            layers: t.layers.len(),
            section,
            height: w.height,
            crown: t.thickness(),
            z0: level + w.base_offset,
        })
    }

    /// How each layer boundary moves across (left positive) at `z` above
    /// the base.
    pub fn shifts_at(&self, z: f64) -> Vec<f64> {
        let (e, i) = self.section.leans_at(self.height, z);
        self.shifts(e, i)
    }

    fn shifts(&self, e: f64, i: f64) -> Vec<f64> {
        (0..=self.layers)
            .map(|b| if b <= self.layer { e } else { -i })
            .collect()
    }

    /// The boundaries' shifts of the footing's sides (`None`: no footing).
    pub fn footing_shifts(&self) -> Option<Vec<f64>> {
        let f = self.section.footing?;
        let (e, i) = self.section.leans_at(self.height, 0.0);
        Some(self.shifts(e + f.toe_ext, i + f.toe_int))
    }

    /// The height above the base where the faces start (the top of the
    /// footing, or the base).
    pub fn foot(&self) -> f64 {
        self.section.footing_depth()
    }

    /// Thickness at the foot of the faces.
    pub fn base_thickness(&self) -> f64 {
        self.section.base_thickness(self.crown, self.height)
    }

    /// The section across, `(w, z)`: `w` from the crown's exterior face,
    /// inwards positive (towards the interior), `z` above the base;
    /// counter-clockwise.
    pub fn polygon(&self) -> Vec<Point2> {
        let h = self.height;
        let d = self.foot();
        let (e, i) = self.section.leans_at(h, 0.0);
        let mut pts = Vec::new();
        match self.section.footing {
            Some(f) => {
                let (x0, x1) = (-e - f.toe_ext, self.crown + i + f.toe_int);
                pts.extend([
                    Point2::new(x0, 0.0),
                    Point2::new(x1, 0.0),
                    Point2::new(x1, d),
                    Point2::new(self.crown + i, d),
                ]);
                pts.extend([
                    Point2::new(self.crown, h),
                    Point2::new(0.0, h),
                    Point2::new(-e, d),
                    Point2::new(x0, d),
                ]);
            }
            None => pts.extend([
                Point2::new(-e, 0.0),
                Point2::new(self.crown + i, 0.0),
                Point2::new(self.crown, h),
                Point2::new(0.0, h),
            ]),
        }
        pts.dedup_by(|a, b| a.approx_eq(*b));
        if pts.len() > 1 && pts[0].approx_eq(pts[pts.len() - 1]) {
            pts.pop();
        }
        pts
    }
}

/// The wall at `z` above its base: with its battered faces where they are
/// there (its own geometry if it is plain).
pub fn geometry_at(idx: &WallIndex, id: WallId, z: f64) -> Option<WallGeometry> {
    match SectionFrame::of(idx.model, id) {
        Some(f) => wall_geometry_with(idx, id, Some(&f.shifts_at(z))),
        None => wall_geometry_in(idx, id),
    }
}

/// A wall's plan geometry as the plan cuts it, `cut` above its level:
/// `g` itself unless the wall is of variable section.
pub fn cut_geometry(idx: &WallIndex, g: WallGeometry, cut: f64) -> WallGeometry {
    let Some(w) = idx.model.wall(g.wall) else {
        return g;
    };
    let Some(f) = SectionFrame::of(idx.model, g.wall) else {
        return g;
    };
    let z = (cut - w.base_offset).clamp(0.0, f.height);
    wall_geometry_with(idx, g.wall, Some(&f.shifts_at(z))).unwrap_or(g)
}

/// The corners of a geometry's outline without what other walls take:
/// exterior and interior corner of its start and of its end,
/// counter-clockwise.
pub fn corners(g: &WallGeometry) -> Option<[Point2; 4]> {
    let (s, e) = g.end_points();
    let q = [*s.first()?, *s.last()?, *e.last()?, *e.first()?];
    let area = archisim_core::signed_area(&q);
    Some(if area < 0.0 {
        [q[3], q[2], q[1], q[0]]
    } else {
        q
    })
}

/// The outline of a wall's footing in plan (`None`: it has none).
pub fn footing_outline(idx: &WallIndex, id: WallId) -> Option<[Point2; 4]> {
    let f = SectionFrame::of(idx.model, id)?;
    let g = wall_geometry_with(idx, id, Some(&f.footing_shifts()?))?;
    corners(&g)
}

/// What the plan shows of a wall of variable section below its cut, as
/// hidden lines: the foot of its battered faces (on the footing or at the
/// base) and the outline of its footing. Each a polyline.
pub fn hidden_lines(idx: &WallIndex, id: WallId) -> Vec<Vec<Point2>> {
    let Some(f) = SectionFrame::of(idx.model, id) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    if let Some(g) = wall_geometry_with(idx, id, Some(&f.shifts_at(f.foot()))) {
        let (s, e) = g.end_points();
        let (lean_e, lean_i) = f.section.leans_at(f.height, 0.0);
        if lean_e > 1e-6 {
            if let (Some(a), Some(b)) = (s.first(), e.first()) {
                out.push(vec![*a, *b]);
            }
        }
        if lean_i > 1e-6 {
            if let (Some(a), Some(b)) = (s.last(), e.last()) {
                out.push(vec![*a, *b]);
            }
        }
    }
    if let Some(q) = footing_outline(idx, id) {
        out.push(vec![q[0], q[1], q[2], q[3], q[0]]);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use archisim_core::{
        CutPattern, Layer, LayerFunction, NewWall, ReferenceLine, WallFooting, WallFunction,
        WallTop,
    };

    pub(crate) fn retaining() -> (Model, WallId) {
        let mut m = Model::new("Muro");
        let level = m.add_level("Planta baja", 0.0);
        let c = m.add_material("Hormigón", "concrete", [200, 200, 200], CutPattern::Cross);
        let t = m
            .add_wall_type(
                "Muro de contención",
                WallFunction::Exterior,
                vec![Layer {
                    material: c,
                    thickness: 0.3,
                    function: LayerFunction::Structure,
                }],
            )
            .unwrap();
        let w = m
            .add_wall(NewWall {
                wall_type: t,
                level,
                start: Point2::new(0.0, 0.0),
                end: Point2::new(5.0, 0.0),
                reference: ReferenceLine::Center,
                height: 3.0,
                base_offset: 0.0,
                bulge: 0.0,
                top: WallTop::Unconnected,
            })
            .unwrap();
        m.set_wall_section(
            w,
            Some(WallSection {
                batter_ext: 0.1,
                batter_int: 0.0,
                layer: None,
                footing: Some(WallFooting {
                    toe_ext: 0.6,
                    toe_int: 0.4,
                    depth: 0.5,
                }),
            }),
        )
        .unwrap();
        (m, w)
    }

    #[test]
    fn the_plan_at_a_height_and_the_footing() {
        let (m, w) = retaining();
        let idx = WallIndex::new(&m);
        // The wall runs along +x: its exterior face is on +y.
        let top = geometry_at(&idx, w, 3.0).unwrap();
        assert!((top.exterior_offset - 0.15).abs() < 1e-9);
        let foot = geometry_at(&idx, w, 0.5).unwrap();
        assert!(
            (foot.exterior_offset - 0.4).abs() < 1e-9,
            "{}",
            foot.exterior_offset
        );
        assert!((foot.interior_offset + 0.15).abs() < 1e-9);
        // In the footing it is the foot of the faces.
        let low = geometry_at(&idx, w, 0.2).unwrap();
        assert!((low.exterior_offset - 0.4).abs() < 1e-9);
        let q = footing_outline(&idx, w).unwrap();
        let ys: Vec<f64> = q.iter().map(|p| p.y).collect();
        let (lo, hi) = (
            ys.iter().copied().fold(f64::INFINITY, f64::min),
            ys.iter().copied().fold(f64::NEG_INFINITY, f64::max),
        );
        assert!((hi - 1.0).abs() < 1e-9 && (lo + 0.55).abs() < 1e-9, "{q:?}");
        // Hidden: the foot of the exterior face and the footing.
        assert_eq!(hidden_lines(&idx, w).len(), 2);
        let f = SectionFrame::of(&m, w).unwrap();
        let area = archisim_core::signed_area(&f.polygon());
        assert!((area - f.section.area(0.3, 3.0)).abs() < 1e-9);
    }

    #[test]
    fn the_solid_is_the_section_along_the_wall() {
        let (m, w) = retaining();
        let solids = crate::solid::wall_solids(&m, w);
        let v: f64 = solids.iter().map(|s| s.volume()).sum();
        let f = SectionFrame::of(&m, w).unwrap();
        let area = f.section.area(0.3, 3.0);
        // A free wall with square ends: the section times its length.
        assert!((v - area * 5.0).abs() < 1e-6, "{v} vs {}", area * 5.0);
        // The stem leans: at the top its exterior face is at 0,15.
        let top_y = solids
            .iter()
            .flat_map(|s| s.triangles.iter().flatten())
            .filter(|p| (p[2] - 3.0).abs() < 1e-9)
            .map(|p| p[1])
            .fold(f64::NEG_INFINITY, f64::max);
        assert!((top_y - 0.15).abs() < 1e-9, "{top_y}");
    }

    #[test]
    fn two_battered_walls_meet_at_their_corner() {
        let (mut m, w) = retaining();
        let a = m.wall(w).unwrap().clone();
        // Clockwise outline: from (5,0) down to (5,-4); its exterior on
        // the left (+x), as the first one's (+y).
        let w2 = m
            .add_wall(NewWall {
                wall_type: a.wall_type,
                level: a.level,
                start: Point2::new(5.0, 0.0),
                end: Point2::new(5.0, -4.0),
                reference: ReferenceLine::Center,
                height: 3.0,
                base_offset: 0.0,
                bulge: 0.0,
                top: WallTop::Unconnected,
            })
            .unwrap();
        m.set_wall_section(w2, a.section).unwrap();
        let idx = WallIndex::new(&m);
        for z in [0.5, 1.7, 3.0] {
            let g1 = geometry_at(&idx, w, z).unwrap();
            let g2 = geometry_at(&idx, w2, z).unwrap();
            // The exterior corner of the end of the first is the one of
            // the start of the second.
            let c1 = *g1.end_points().1.first().unwrap();
            let c2 = *g2.end_points().0.first().unwrap();
            assert!(c1.distance(c2) < 1e-9, "{z}: {c1:?} {c2:?}");
            let lean = 0.1 * (3.0 - z.max(0.5));
            assert!((c1.x - (5.15 + lean)).abs() < 1e-9 && (c1.y - (0.15 + lean)).abs() < 1e-9);
        }
        let q1 = footing_outline(&idx, w).unwrap();
        let q2 = footing_outline(&idx, w2).unwrap();
        assert!(q1.iter().any(|p| q2.iter().any(|q| p.distance(*q) < 1e-9)));
    }
}
