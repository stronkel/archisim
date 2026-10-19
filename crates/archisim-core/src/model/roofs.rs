//! Pitched roofs by planes ("cubiertas por faldones", ArchiCAD's multi-plane
//! roof, Revit's roof by footprint).
//!
//! A roof is drawn by its footprint (usually the outer faces of the walls
//! under it). Each side of the footprint either slopes (a roof plane rises
//! from it at its pitch) or is a gable ("piñón", no plane). The planes meet
//! at ridges, hips and valleys where they cross; the eaves and verges stand
//! out from the footprint by the overhang. Its build-up is a slab type
//! (usually a roof type): the layers keep their thickness square to each
//! plane and the edges are plumb, as inclined slabs.
//!
//! The footprint must be convex for now (rectangles, any polygon without
//! re-entrant corners): an L-shaped house takes two roofs.
//!
//! IFC: `IfcRoof` made of one `IfcSlab` (`ROOF`) per plane.

use super::*;

/// A pitched roof.
#[derive(Debug, Clone, PartialEq)]
pub struct Roof {
    pub id: RoofId,
    pub guid: GlobalId,
    /// Its build-up: a slab type (the layers, from the top down).
    pub roof_type: SlabTypeId,
    pub level: LevelId,
    /// The footprint, counter-clockwise and convex: where the planes
    /// start, at `base_offset`.
    pub outline: Vec<Point2>,
    /// The pitch of each side (side `i` runs from corner `i` to the
    /// next), as rise per run (0.3 = 30 %); `None`: a gable.
    pub slopes: Vec<Option<f64>>,
    /// Height of the top face above the level along the footprint.
    pub base_offset: f64,
    /// How far the eaves and verges stand out from the footprint.
    pub overhang: f64,
}

/// Data needed to create a roof.
#[derive(Debug, Clone, PartialEq)]
pub struct NewRoof {
    pub roof_type: SlabTypeId,
    pub level: LevelId,
    pub outline: Vec<Point2>,
    /// One per side; or a single value for every side.
    pub slopes: Vec<Option<f64>>,
    pub base_offset: f64,
    pub overhang: f64,
}

/// One plane of a roof: the side it rises from, its outline in plan (the
/// part of the roof it covers, eaves included) and its slope, as that of
/// an inclined slab whose top face is at the roof's `base_offset` along
/// the footprint.
#[derive(Debug, Clone, PartialEq)]
pub struct RoofPlane {
    pub side: usize,
    pub outline: Vec<Point2>,
    pub slope: SlabSlope,
}

/// Largest overhang (metres).
pub const MAX_OVERHANG: f64 = 3.0;

impl Roof {
    /// The inward normal of side `i` and a point on it.
    fn side(&self, i: usize) -> Option<(Point2, Vec2)> {
        let n = self.outline.len();
        let (a, b) = (self.outline[i], self.outline[(i + 1) % n]);
        // Counter-clockwise: the inside is to the left.
        (b - a).normalized().map(|u| (a, u.perp_left()))
    }

    /// The top face of plane `i` at `p`, above the level (not limited to
    /// the part of the roof it covers).
    fn plane_at(&self, i: usize, p: Point2) -> Option<f64> {
        let s = self.slopes.get(i).copied().flatten()?;
        let (a, n) = self.side(i)?;
        Some(self.base_offset + s * (p - a).dot(n))
    }

    /// Height of the top face above the level at `p`: the lowest plane
    /// there (planes meet at ridges and hips).
    pub fn top_at(&self, p: Point2) -> f64 {
        (0..self.outline.len())
            .filter_map(|i| self.plane_at(i, p))
            .fold(f64::INFINITY, f64::min)
    }

    /// The outline of the eaves and verges: the footprint grown by the
    /// overhang, counter-clockwise.
    pub fn eaves(&self) -> Vec<Point2> {
        grow_convex(&self.outline, self.overhang)
    }

    /// The planes of the roof, each with the part of the roof it covers.
    pub fn planes(&self) -> Vec<RoofPlane> {
        let eaves = self.eaves();
        let n = self.outline.len();
        let mut out = Vec::new();
        for i in 0..n {
            let (Some(s), Some((a, nrm))) = (self.slopes[i], self.side(i)) else {
                continue;
            };
            // Where this plane is the lowest: h_i(p) <= h_j(p) for every
            // other plane j, each a half-plane in plan.
            let mut region = eaves.clone();
            for j in 0..n {
                if j == i {
                    continue;
                }
                let (Some(t), Some((b, m))) = (self.slopes[j], self.side(j)) else {
                    continue;
                };
                // h_i - h_j = (s·n_i - t·n_j)·p - (s·n_i·a - t·n_j·b) <= 0.
                let g = nrm * s - m * t;
                let c = s * nrm.dot(a) - t * m.dot(b);
                if g.length() < 1e-12 {
                    // The same plane (sides in line with the same pitch):
                    // the first one takes it.
                    if (c).abs() < 1e-9 && j < i {
                        region.clear();
                    }
                    continue;
                }
                region = clip_half_plane(&region, g, c);
                if region.len() < 3 {
                    break;
                }
            }
            if region.len() >= 3 && signed_area(&region).abs() > 1e-9 {
                out.push(RoofPlane {
                    side: i,
                    outline: region,
                    slope: SlabSlope {
                        from: a,
                        to: a + nrm,
                        slope: s,
                    },
                });
            }
        }
        out
    }

    /// Lowest and highest point of the top face above the level.
    pub fn top_range(&self) -> (f64, f64) {
        self.planes()
            .iter()
            .flat_map(|p| p.outline.iter().map(|q| self.top_at(*q)))
            .fold((f64::INFINITY, f64::NEG_INFINITY), |(lo, hi), z| {
                (lo.min(z), hi.max(z))
            })
    }

    /// Area of the roof surface (on the slope), eaves included.
    pub fn surface_area(&self) -> f64 {
        self.planes()
            .iter()
            .map(|p| signed_area(&p.outline).abs() * p.slope.vertical(1.0))
            .sum()
    }
}

/// A convex polygon (counter-clockwise) grown outwards by `d`: each side
/// moved out square to itself, the corners where they meet.
pub fn grow_convex(poly: &[Point2], d: f64) -> Vec<Point2> {
    let n = poly.len();
    if d.abs() < 1e-12 || n < 3 {
        return poly.to_vec();
    }
    let moved: Vec<(Point2, Vec2)> = (0..n)
        .map(|i| {
            let (a, b) = (poly[i], poly[(i + 1) % n]);
            let u = (b - a).normalized().unwrap_or(Vec2::new(1.0, 0.0));
            // Outside is to the right.
            (a - u.perp_left() * d, u)
        })
        .collect();
    (0..n)
        .map(|i| {
            let (p, u) = moved[(i + n - 1) % n];
            let (q, v) = moved[i];
            let den = u.x * v.y - u.y * v.x;
            if den.abs() < 1e-12 {
                // In line: the corner just moves out.
                q
            } else {
                let w = q - p;
                let t = (w.x * v.y - w.y * v.x) / den;
                p + u * t
            }
        })
        .collect()
}

/// The part of a convex polygon where `g·p <= c`.
fn clip_half_plane(poly: &[Point2], g: Vec2, c: f64) -> Vec<Point2> {
    let f = |p: Point2| g.dot(p) - c;
    let n = poly.len();
    let mut out = Vec::new();
    for i in 0..n {
        let (a, b) = (poly[i], poly[(i + 1) % n]);
        let (fa, fb) = (f(a), f(b));
        if fa <= 1e-12 {
            out.push(a);
        }
        if (fa < -1e-12 && fb > 1e-12) || (fa > 1e-12 && fb < -1e-12) {
            let t = fa / (fa - fb);
            out.push(a + (b - a) * t);
        }
    }
    // Drop points that repeat.
    out.dedup_by(|a, b| a.approx_eq(*b));
    if out.len() > 1 && out[0].approx_eq(out[out.len() - 1]) {
        out.pop();
    }
    out
}

/// Volume of a prism on `footprint` from `z_base` up to the lowest of
/// the planes `z = a·x + b·y + c` (a wall trimmed under a roof): over the
/// part of the footprint where each plane is the lowest, its area by the
/// height at its centroid.
pub fn volume_under_planes(footprint: &[Point2], planes: &[[f64; 3]], z_base: f64) -> f64 {
    let mut poly = footprint.to_vec();
    if signed_area(&poly) < 0.0 {
        poly.reverse();
    }
    let mut total = 0.0;
    for (i, p) in planes.iter().enumerate() {
        let mut region = poly.clone();
        for (j, q) in planes.iter().enumerate() {
            if i == j || region.len() < 3 {
                continue;
            }
            let g = Vec2::new(p[0] - q[0], p[1] - q[1]);
            let c = q[2] - p[2];
            if g.length() < 1e-12 {
                // The same slope: the lower one covers it all.
                if c < 0.0 || (c.abs() < 1e-12 && j < i) {
                    region.clear();
                }
                continue;
            }
            region = clip_half_plane(&region, g, c);
        }
        if region.len() >= 3 {
            let area = signed_area(&region).abs();
            let m = super::spaces::centroid(&region);
            total += area * (p[0] * m.x + p[1] * m.y + p[2] - z_base);
        }
    }
    total
}

/// Whether a counter-clockwise polygon has no re-entrant corner.
fn is_convex(poly: &[Point2]) -> bool {
    let n = poly.len();
    (0..n).all(|i| {
        let (a, b, c) = (poly[i], poly[(i + 1) % n], poly[(i + 2) % n]);
        let (u, v) = (b - a, c - b);
        u.x * v.y - u.y * v.x >= -1e-9 * (u.length() * v.length()).max(1e-12)
    })
}

impl Model {
    fn validate_roof(&self, r: &NewRoof) -> Result<(Vec<Point2>, Vec<Option<f64>>), ModelError> {
        if !self.levels.contains_key(&r.level) {
            return Err(ModelError::UnknownLevel(r.level));
        }
        if !self.slab_types.contains_key(&r.roof_type) {
            return Err(ModelError::UnknownSlabType(r.roof_type));
        }
        finite(r.base_offset, "roof base offset")?;
        finite(r.overhang, "roof overhang")?;
        if !(0.0..=MAX_OVERHANG).contains(&r.overhang) {
            return Err(ModelError::InvalidRoof);
        }
        // Counter-clockwise; the pitches follow the sides if it turns.
        let mut outline = r.outline.clone();
        let mut slopes = r.slopes.clone();
        let n = outline.len();
        if slopes.len() == 1 && n > 1 {
            slopes = vec![slopes[0]; n];
        }
        if slopes.len() != n {
            return Err(ModelError::InvalidRoof);
        }
        for p in &outline {
            finite_point(*p, "roof outline point")?;
        }
        if !is_simple_polygon(&outline) || signed_area(&outline).abs() <= EPSILON {
            return Err(ModelError::InvalidOutline);
        }
        if signed_area(&outline) < 0.0 {
            outline.reverse();
            // Side i of the reversed outline is side n-2-i of the original.
            let old = slopes.clone();
            slopes = (0..n).map(|i| old[(2 * n - 2 - i) % n]).collect();
        }
        if !is_convex(&outline) {
            return Err(ModelError::RoofNotConvex);
        }
        let mut any = false;
        for s in slopes.iter().flatten() {
            finite(*s, "roof slope")?;
            if *s <= 1e-6 || *s > MAX_SLAB_SLOPE {
                return Err(ModelError::InvalidRoof);
            }
            any = true;
        }
        if !any {
            return Err(ModelError::InvalidRoof);
        }
        Ok((outline, slopes))
    }

    pub fn add_roof(&mut self, spec: NewRoof) -> Result<RoofId, ModelError> {
        let (outline, slopes) = self.validate_roof(&spec)?;
        let id = RoofId(self.take_id());
        self.roofs.insert(
            id,
            Roof {
                id,
                guid: GlobalId::new(),
                roof_type: spec.roof_type,
                level: spec.level,
                outline,
                slopes,
                base_offset: spec.base_offset,
                overhang: spec.overhang,
            },
        );
        self.stamp_phase(id.0);
        Ok(id)
    }

    /// Changes a roof; nothing changes if the result is not valid.
    pub fn edit_roof(
        &mut self,
        id: RoofId,
        edit: impl FnOnce(&mut Roof),
    ) -> Result<(), ModelError> {
        let mut r = self
            .roofs
            .get(&id)
            .cloned()
            .ok_or(ModelError::UnknownRoof(id))?;
        edit(&mut r);
        let (outline, slopes) = self.validate_roof(&NewRoof {
            roof_type: r.roof_type,
            level: r.level,
            outline: r.outline.clone(),
            slopes: r.slopes.clone(),
            base_offset: r.base_offset,
            overhang: r.overhang,
        })?;
        r.outline = outline;
        r.slopes = slopes;
        self.roofs.insert(id, r);
        Ok(())
    }

    pub fn remove_roof(&mut self, id: RoofId) -> Result<Roof, ModelError> {
        let r = self.roofs.remove(&id).ok_or(ModelError::UnknownRoof(id))?;
        self.wall_roofs.retain(|_, x| *x != id);
        Ok(r)
    }

    /// Trims a wall under a roof (its top follows the underside of the
    /// roof's planes), or frees it.
    pub fn set_wall_roof(&mut self, wall: WallId, roof: Option<RoofId>) -> Result<(), ModelError> {
        if !self.walls.contains_key(&wall) {
            return Err(ModelError::UnknownWall(wall));
        }
        match roof {
            Some(r) => {
                if !self.roofs.contains_key(&r) {
                    return Err(ModelError::UnknownRoof(r));
                }
                self.wall_roofs.insert(wall, r);
            }
            None => {
                self.wall_roofs.remove(&wall);
            }
        }
        Ok(())
    }

    /// The roof a wall is trimmed under.
    pub fn wall_roof(&self, wall: WallId) -> Option<RoofId> {
        self.wall_roofs.get(&wall).copied()
    }

    /// The planes under which a wall is trimmed, as `z = a·x + b·y + c`
    /// (heights from project zero): the undersides of its roof's planes.
    pub fn wall_trim_planes(&self, wall: WallId) -> Vec<[f64; 3]> {
        let Some(roof) = self.wall_roof(wall).and_then(|r| self.roofs.get(&r)) else {
            return Vec::new();
        };
        let level = self.levels.get(&roof.level).map_or(0.0, |l| l.elevation);
        let thickness = self
            .slab_types
            .get(&roof.roof_type)
            .map_or(0.0, |t| t.thickness());
        roof.planes()
            .iter()
            .filter_map(|p| {
                let u = p.slope.direction()?;
                let s = p.slope.slope;
                let c = level + roof.base_offset
                    - s * p.slope.from.dot(u)
                    - p.slope.vertical(thickness);
                Some([s * u.x, s * u.y, c])
            })
            .collect()
    }

    /// The highest point a wall reaches: its own top, or the ridge of the
    /// roof it is trimmed under if that is higher (it is built that high
    /// and then trimmed).
    pub fn wall_top_z(&self, wall: &Wall) -> f64 {
        let level = self.levels.get(&wall.level).map_or(0.0, |l| l.elevation);
        let own = level + wall.base_offset + wall.height;
        match self.wall_roof(wall.id).and_then(|r| self.roofs.get(&r)) {
            Some(roof) => {
                let rl = self.levels.get(&roof.level).map_or(0.0, |l| l.elevation);
                own.max(rl + roof.top_range().1)
            }
            None => own,
        }
    }

    pub fn roof(&self, id: RoofId) -> Option<&Roof> {
        self.roofs.get(&id)
    }

    pub fn roofs(&self) -> impl Iterator<Item = &Roof> {
        self.roofs.values()
    }

    pub fn roofs_on_level(&self, level: LevelId) -> impl Iterator<Item = &Roof> {
        self.roofs.values().filter(move |r| r.level == level)
    }

    pub(super) fn restore_roofs(
        &mut self,
        roofs: Vec<Roof>,
        claim: &mut structure::Claim<'_>,
    ) -> Result<(), RestoreError> {
        for mut r in roofs {
            let name = format!("roof {} ({})", r.id.0, r.guid);
            let spec = NewRoof {
                roof_type: r.roof_type,
                level: r.level,
                outline: r.outline.clone(),
                slopes: r.slopes.clone(),
                base_offset: r.base_offset,
                overhang: r.overhang,
            };
            claim(r.id.0, Some(&r.guid))
                .and_then(|_| self.validate_roof(&spec))
                .map(|(outline, slopes)| {
                    r.outline = outline;
                    r.slopes = slopes;
                })
                .map_err(|error| RestoreError {
                    element: name,
                    error,
                })?;
            self.roofs.insert(r.id, r);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn setup() -> (Model, LevelId, SlabTypeId) {
        let mut m = Model::new("t");
        let l = m.add_level("Cubierta", 3.0);
        let tile = m.add_material("Teja", "tile", [180, 90, 60], CutPattern::Solid);
        let t = m
            .add_slab_type(
                "Cubierta de teja",
                SlabFunction::Roof,
                vec![Layer {
                    material: tile,
                    thickness: 0.2,
                    function: LayerFunction::Structure,
                }],
            )
            .unwrap();
        (m, l, t)
    }

    fn rect(w: f64, d: f64) -> Vec<Point2> {
        vec![
            Point2::new(0.0, 0.0),
            Point2::new(w, 0.0),
            Point2::new(w, d),
            Point2::new(0.0, d),
        ]
    }

    /// A hip roof over 12 × 8 at 50 %: four planes, the ridge 2 m up.
    #[test]
    fn hip_roof() {
        let (mut m, l, t) = setup();
        let r = m
            .add_roof(NewRoof {
                roof_type: t,
                level: l,
                outline: rect(12.0, 8.0),
                slopes: vec![Some(0.5)],
                base_offset: 0.0,
                overhang: 0.5,
            })
            .unwrap();
        let roof = m.roof(r).unwrap();
        let planes = roof.planes();
        assert_eq!(planes.len(), 4);
        // The planes cover the eaves exactly.
        let covered: f64 = planes.iter().map(|p| signed_area(&p.outline)).sum();
        assert!((covered - 13.0 * 9.0).abs() < 1e-9, "{covered}");
        assert!((roof.top_at(Point2::new(6.0, 4.0)) - 2.0).abs() < 1e-12);
        // The eaves drop below the footprint by the overhang's rise.
        assert!((roof.top_at(Point2::new(6.0, -0.5)) + 0.25).abs() < 1e-12);
        assert_eq!(roof.top_range(), (-0.25, 2.0));
        // The long sides make trapezia, the short ones triangles.
        let corners: Vec<usize> = planes.iter().map(|p| p.outline.len()).collect();
        assert_eq!(corners, vec![4, 3, 4, 3]);
        let back = Model::from_parts(m.to_parts()).unwrap();
        assert_eq!(back, m);
    }

    /// Gables at both ends: two planes and a ridge along the length.
    #[test]
    fn gable_roof_and_errors() {
        let (mut m, l, t) = setup();
        let spec = NewRoof {
            roof_type: t,
            level: l,
            outline: rect(10.0, 6.0),
            slopes: vec![Some(0.3), None, Some(0.3), None],
            base_offset: 0.0,
            overhang: 0.0,
        };
        let r = m.add_roof(spec.clone()).unwrap();
        let roof = m.roof(r).unwrap();
        assert_eq!(roof.planes().len(), 2);
        assert!((roof.top_at(Point2::new(1.0, 3.0)) - 0.9).abs() < 1e-12);
        // Drawn clockwise, the pitches stay on their sides.
        let mut cw = spec.clone();
        cw.outline.reverse();
        cw.slopes = vec![Some(0.3), None, Some(0.3), None];
        // (Its side 0 is the north one.)
        let r2 = m.add_roof(cw).unwrap();
        let roof2 = m.roof(r2).unwrap();
        assert!((roof2.top_at(Point2::new(5.0, 3.0)) - 0.9).abs() < 1e-12);
        // No plane at all, an L, a flat pitch, too much overhang.
        let mut bad = spec.clone();
        bad.slopes = vec![None];
        assert_eq!(m.add_roof(bad), Err(ModelError::InvalidRoof));
        let mut l_shape = spec.clone();
        l_shape.outline = vec![
            Point2::new(0.0, 0.0),
            Point2::new(6.0, 0.0),
            Point2::new(6.0, 3.0),
            Point2::new(3.0, 3.0),
            Point2::new(3.0, 6.0),
            Point2::new(0.0, 6.0),
        ];
        l_shape.slopes = vec![Some(0.3)];
        assert_eq!(m.add_roof(l_shape), Err(ModelError::RoofNotConvex));
        let mut flat = spec.clone();
        flat.slopes = vec![Some(0.0)];
        assert_eq!(m.add_roof(flat), Err(ModelError::InvalidRoof));
        let mut wide = spec;
        wide.overhang = 5.0;
        assert_eq!(m.add_roof(wide), Err(ModelError::InvalidRoof));
        // Edited whole or not at all.
        assert!(m.edit_roof(r, |r| r.slopes = vec![None; 4]).is_err());
        assert_eq!(m.roof(r).unwrap().planes().len(), 2);
    }

    #[test]
    fn grown_outline() {
        let g = grow_convex(&rect(4.0, 2.0), 1.0);
        assert_eq!(
            g,
            vec![
                Point2::new(-1.0, -1.0),
                Point2::new(5.0, -1.0),
                Point2::new(5.0, 3.0),
                Point2::new(-1.0, 3.0)
            ]
        );
    }
}
