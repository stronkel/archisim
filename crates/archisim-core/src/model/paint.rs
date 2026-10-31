//! Paint on faces: a material for one face of a wall or of a concept
//! volume, over the element's own (as painting a face in SketchUp or
//! overriding a surface in ArchiCAD); and painted zones of a wall's face,
//! bounded by edges drawn on it (two textures on one façade). It changes
//! how the element is seen in 3D, not what it is made of: its layers,
//! measurements and IFC materials stay the type's.

use super::*;

/// A side of a wall: to the left of its line (start → end) is the
/// exterior, to the right the interior.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum FaceSide {
    Exterior,
    Interior,
}

impl FaceSide {
    pub fn code(self) -> &'static str {
        match self {
            FaceSide::Exterior => "exterior",
            FaceSide::Interior => "interior",
        }
    }

    pub fn from_code(c: &str) -> Option<Self> {
        match c {
            "exterior" => Some(FaceSide::Exterior),
            "interior" => Some(FaceSide::Interior),
            _ => None,
        }
    }
}

/// A face of an element: a side of a wall; the top, the bottom or a side
/// of a volume (by the edge of its outline it stands on).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum FaceRef {
    Wall(FaceSide),
    Top,
    Bottom,
    /// The side of a volume on the edge from corner `i` to `i + 1` (of a
    /// round form, its curved surface, or a piece of its profile).
    Side(usize),
    /// A face of a solid made by joining or cutting volumes, by number.
    Part(u32),
}

impl FaceRef {
    /// «exterior», «interior», «top», «bottom», «side 3».
    pub fn code(self) -> String {
        match self {
            FaceRef::Wall(s) => s.code().to_string(),
            FaceRef::Top => "top".into(),
            FaceRef::Bottom => "bottom".into(),
            FaceRef::Side(i) => format!("side {i}"),
            FaceRef::Part(n) => format!("part {n}"),
        }
    }

    pub fn from_code(c: &str) -> Option<Self> {
        match c {
            "top" => Some(FaceRef::Top),
            "bottom" => Some(FaceRef::Bottom),
            _ => match (c.strip_prefix("side "), c.strip_prefix("part ")) {
                (Some(i), _) => i.trim().parse().ok().map(FaceRef::Side),
                (_, Some(n)) => n.trim().parse().ok().map(FaceRef::Part),
                _ => FaceSide::from_code(c).map(FaceRef::Wall),
            },
        }
    }
}

/// A painted zone of a wall's face: a polygon on it, in metres along the
/// wall's line from its start (`x`) and up from the level (`y`).
#[derive(Debug, Clone, PartialEq)]
pub struct PaintZone {
    pub wall: WallId,
    pub side: FaceSide,
    pub outline: Vec<Point2>,
    pub material: MaterialId,
}

/// A zone of a flat face looking up or down (the top or the bottom of a
/// slab, a footing, a column…), drawn on it with «Dividir cara» (G3): its
/// outline in plan, and its paint (`None`: only divided, the face's own
/// paint in sight).
#[derive(Debug, Clone, PartialEq)]
pub struct FaceZone {
    pub element: u64,
    pub face: FaceRef,
    pub outline: Vec<Point2>,
    pub material: Option<MaterialId>,
}

/// The number of sides by direction the faces of a column, a beam or a
/// footing are told apart by (every 22,5°).
pub const MEMBER_SIDES: usize = 16;

impl Model {
    /// Whether an element has any face painted (to skip the rest quickly).
    pub fn has_face_paint(&self, element: u64) -> bool {
        self.face_paints
            .range((element, FaceRef::Wall(FaceSide::Exterior))..)
            .next()
            .is_some_and(|((e, _), _)| *e == element)
            || self.face_zones.iter().any(|z| z.element == element)
    }

    fn check_face(&self, element: u64, face: FaceRef) -> Result<(), ModelError> {
        // Slabs, roofs, columns, beams and footings (G3): their top, their
        // bottom and their sides.
        if let Some(sides) = self.plain_faces(element) {
            return match face {
                FaceRef::Top | FaceRef::Bottom => Ok(()),
                FaceRef::Side(i) if i < sides => Ok(()),
                // A roof's edges.
                FaceRef::Part(n)
                    if (n as usize) < sides && self.roofs.contains_key(&RoofId(element)) =>
                {
                    Ok(())
                }
                _ => Err(ModelError::InvalidShape("no such face of the element")),
            };
        }
        match face {
            FaceRef::Wall(_) => {
                if !self.walls.contains_key(&WallId(element)) {
                    return Err(ModelError::UnknownWall(WallId(element)));
                }
            }
            FaceRef::Top | FaceRef::Bottom | FaceRef::Side(_) | FaceRef::Part(_) => {
                let m = self
                    .masses
                    .get(&MassId(element))
                    .ok_or(ModelError::UnknownElement(element))?;
                let ok = match (face, &m.shape) {
                    (FaceRef::Part(n), MassShape::Mesh(mesh)) => mesh.faces.contains(&n),
                    (FaceRef::Part(_), _) | (_, MassShape::Mesh(_)) => false,
                    (FaceRef::Side(i), _) => i < m.side_count(),
                    _ => true,
                };
                if !ok {
                    return Err(ModelError::InvalidShape("no such face of the volume"));
                }
            }
        }
        Ok(())
    }

    /// Paints a face (`None` takes the paint off).
    pub fn set_face_paint(
        &mut self,
        element: u64,
        face: FaceRef,
        material: Option<MaterialId>,
    ) -> Result<(), ModelError> {
        self.check_face(element, face)?;
        match material {
            Some(m) => {
                if !self.materials.contains_key(&m) {
                    return Err(ModelError::UnknownMaterial(m));
                }
                self.face_paints.insert((element, face), m);
            }
            None => {
                self.face_paints.remove(&(element, face));
            }
        }
        Ok(())
    }

    /// The paint on a face.
    pub fn face_paint(&self, element: u64, face: FaceRef) -> Option<MaterialId> {
        self.face_paints.get(&(element, face)).copied()
    }

    /// Every painted face, of elements that are there.
    pub fn face_paints(&self) -> impl Iterator<Item = (u64, FaceRef, MaterialId)> + '_ {
        self.face_paints
            .iter()
            .filter(|((e, f), _)| self.check_face(*e, *f).is_ok())
            .map(|((e, f), m)| (*e, *f, *m))
    }

    /// How many sides an element painted face by face has (a slab's edges,
    /// a roof's slopes, a member's directions), if it is one of those.
    fn plain_faces(&self, element: u64) -> Option<usize> {
        if let Some(s) = self.slabs.get(&SlabId(element)) {
            return Some(s.outline.len() + s.holes.iter().map(|h| h.len()).sum::<usize>());
        }
        if let Some(r) = self.roofs.get(&RoofId(element)) {
            return Some(r.outline.len());
        }
        if self.columns.contains_key(&ColumnId(element))
            || self.beams.contains_key(&BeamId(element))
            || self.footings.contains_key(&FootingId(element))
        {
            return Some(MEMBER_SIDES);
        }
        None
    }

    /// Divides a flat face looking up or down with a zone (`material`
    /// `None`: divided, not painted). A zone with the same outline is
    /// replaced.
    pub fn set_face_zone(
        &mut self,
        element: u64,
        face: FaceRef,
        outline: Vec<Point2>,
        material: Option<MaterialId>,
    ) -> Result<(), ModelError> {
        if !matches!(face, FaceRef::Top | FaceRef::Bottom) {
            return Err(ModelError::InvalidShape(
                "only faces looking up or down are divided",
            ));
        }
        self.check_face(element, face)?;
        if outline.len() < 3 || outline.iter().any(|p| !p.x.is_finite() || !p.y.is_finite()) {
            return Err(ModelError::InvalidShape("a zone needs three corners"));
        }
        if let Some(m) = material {
            if !self.materials.contains_key(&m) {
                return Err(ModelError::UnknownMaterial(m));
            }
        }
        let same = |z: &FaceZone| {
            z.element == element
                && z.face == face
                && z.outline.len() == outline.len()
                && z.outline.iter().zip(&outline).all(|(a, b)| a.approx_eq(*b))
        };
        match self.face_zones.iter_mut().find(|z| same(z)) {
            Some(z) => z.material = material,
            None => self.face_zones.push(FaceZone {
                element,
                face,
                outline,
                material,
            }),
        }
        Ok(())
    }

    /// Takes away the zones of a face.
    pub fn clear_face_zones(&mut self, element: u64, face: FaceRef) {
        self.face_zones
            .retain(|z| !(z.element == element && z.face == face));
    }

    /// The zones of faces, of elements that are there.
    pub fn face_zones(&self) -> impl Iterator<Item = &FaceZone> {
        self.face_zones
            .iter()
            .filter(|z| self.check_face(z.element, z.face).is_ok())
    }

    /// The zone of a face a point in plan is in (the last drawn wins).
    pub fn face_zone_at(&self, element: u64, face: FaceRef, at: Point2) -> Option<&FaceZone> {
        self.face_zones()
            .filter(|z| z.element == element && z.face == face)
            .filter(|z| point_in(&z.outline, at))
            .last()
    }

    pub(super) fn restore_face_zones(&mut self, zones: Vec<FaceZone>) -> Result<(), RestoreError> {
        for z in zones {
            let e = z.element;
            self.set_face_zone(z.element, z.face, z.outline, z.material)
                .map_err(|error| RestoreError {
                    element: format!("zone of a face of element {e}"),
                    error,
                })?;
        }
        Ok(())
    }

    /// Paints a zone of a wall's face. A zone with the same outline is
    /// painted again; `None` takes it off.
    pub fn set_paint_zone(
        &mut self,
        wall: WallId,
        side: FaceSide,
        outline: Vec<Point2>,
        material: Option<MaterialId>,
    ) -> Result<(), ModelError> {
        if !self.walls.contains_key(&wall) {
            return Err(ModelError::UnknownWall(wall));
        }
        if outline.len() < 3 || outline.iter().any(|p| !p.x.is_finite() || !p.y.is_finite()) {
            return Err(ModelError::InvalidShape(
                "a painted zone needs three corners",
            ));
        }
        let same = |z: &PaintZone| {
            z.wall == wall
                && z.side == side
                && z.outline.len() == outline.len()
                && z.outline.iter().zip(&outline).all(|(a, b)| a.approx_eq(*b))
        };
        self.paint_zones.retain(|z| !same(z));
        if let Some(m) = material {
            if !self.materials.contains_key(&m) {
                return Err(ModelError::UnknownMaterial(m));
            }
            self.paint_zones.push(PaintZone {
                wall,
                side,
                outline,
                material: m,
            });
        }
        Ok(())
    }

    /// Takes off every zone painted on a face of a wall.
    pub fn clear_paint_zones(&mut self, wall: WallId, side: FaceSide) {
        self.paint_zones
            .retain(|z| !(z.wall == wall && z.side == side));
    }

    /// The painted zones, of walls that are there.
    pub fn paint_zones(&self) -> impl Iterator<Item = &PaintZone> {
        self.paint_zones
            .iter()
            .filter(|z| self.walls.contains_key(&z.wall))
    }

    pub(super) fn restore_paint(
        &mut self,
        paints: Vec<(u64, FaceRef, MaterialId)>,
        zones: Vec<PaintZone>,
    ) -> Result<(), RestoreError> {
        for (e, face, m) in paints {
            self.set_face_paint(e, face, Some(m))
                .map_err(|error| RestoreError {
                    element: format!("paint of element {e}"),
                    error,
                })?;
        }
        for z in zones {
            let w = z.wall;
            self.set_paint_zone(z.wall, z.side, z.outline, Some(z.material))
                .map_err(|error| RestoreError {
                    element: format!("painted zone of wall {}", w.0),
                    error,
                })?;
        }
        Ok(())
    }
}

/// The part of a polygon with `x` at least `at` (`keep_after`) or at most
/// `at`: Sutherland–Hodgman against one vertical line.
pub(super) fn clip_x(poly: &[Point2], at: f64, keep_after: bool) -> Vec<Point2> {
    let inside = |p: Point2| if keep_after { p.x >= at } else { p.x <= at };
    let mut out = Vec::new();
    let n = poly.len();
    for i in 0..n {
        let (a, b) = (poly[i], poly[(i + 1) % n]);
        let (ia, ib) = (inside(a), inside(b));
        if ia {
            out.push(a);
        }
        if ia != ib && (b.x - a.x).abs() > 1e-12 {
            let t = (at - a.x) / (b.x - a.x);
            out.push(Point2::new(at, a.y + (b.y - a.y) * t));
        }
    }
    out.dedup_by(|a, b| a.approx_eq(*b));
    out
}

impl Model {
    /// Splits a straight wall in two at `t` metres from its start (Revit's
    /// «Dividir elemento»): the wall keeps the first part and a new wall,
    /// of the same type, height, constraints and phase, takes the rest
    /// with its doors and windows (those beyond `t`), its paint and the
    /// part of its painted zones beyond `t`. The two ends at the cut join
    /// each other. Returns the new wall.
    pub fn split_wall(&mut self, id: WallId, t: f64) -> Result<WallId, ModelError> {
        let w = self
            .walls
            .get(&id)
            .cloned()
            .ok_or(ModelError::UnknownWall(id))?;
        if w.bulge != 0.0 {
            return Err(ModelError::InvalidShape("a curved wall is not split yet"));
        }
        let len = w.start.distance(w.end);
        finite(t, "split point")?;
        if t < 0.05 || t > len - 0.05 {
            return Err(ModelError::InvalidShape(
                "the cut must leave both parts at least 5 cm long",
            ));
        }
        // No door or window across the cut.
        for o in self.openings.values().filter(|o| o.wall == id) {
            let width = self
                .opening_types
                .get(&o.opening_type)
                .map_or(0.0, |ot| ot.width);
            let (a, b) = o.span(width);
            if a < t && b > t {
                return Err(ModelError::InvalidShape(
                    "a door or window is where the wall would be cut",
                ));
            }
        }
        let cut = w.start + (w.end - w.start) * (t / len);
        let new_id = WallId(self.take_id());
        let mut second = Wall {
            id: new_id,
            guid: GlobalId::new(),
            start: cut,
            ..w.clone()
        };
        second.allow_join = [true, w.allow_join[1]];
        // Its profile (E3) is split too.
        let len = w.length();
        second.profile = w.profile.as_ref().and_then(|p| p.part(t, len));
        let phase = self.phase(id.0);
        if let Some(first) = self.walls.get_mut(&id) {
            first.end = cut;
            first.allow_join = [w.allow_join[0], true];
            first.profile = w.profile.as_ref().and_then(|p| p.part(0.0, t));
        }
        self.walls.insert(new_id, second);
        self.stamp_phase(new_id.0);
        let _ = self.set_phase(new_id.0, phase);
        // Doors and windows beyond the cut, to the new wall.
        for o in self.openings.values_mut().filter(|o| o.wall == id) {
            if o.along > t {
                o.wall = new_id;
                o.along -= t;
            }
        }
        // Paint: the faces as they were; the zones cut at `t`.
        for side in [FaceSide::Exterior, FaceSide::Interior] {
            if let Some(m) = self.face_paint(id.0, FaceRef::Wall(side)) {
                self.set_face_paint(new_id.0, FaceRef::Wall(side), Some(m))?;
            }
        }
        let zones: Vec<PaintZone> = self
            .paint_zones
            .iter()
            .filter(|z| z.wall == id)
            .cloned()
            .collect();
        self.paint_zones.retain(|z| z.wall != id);
        for z in zones {
            let before = clip_x(&z.outline, t, false);
            if before.len() >= 3 {
                self.paint_zones.push(PaintZone {
                    outline: before,
                    ..z.clone()
                });
            }
            let after: Vec<Point2> = clip_x(&z.outline, t, true)
                .into_iter()
                .map(|p| Point2::new(p.x - t, p.y))
                .collect();
            if after.len() >= 3 {
                self.paint_zones.push(PaintZone {
                    wall: new_id,
                    outline: after,
                    ..z
                });
            }
        }
        Ok(new_id)
    }
}


/// Whether a point is inside a polygon (even–odd).
fn point_in(poly: &[Point2], p: Point2) -> bool {
    let mut inside = false;
    let n = poly.len();
    for i in 0..n {
        let (a, b) = (poly[i], poly[(i + 1) % n]);
        if (a.y > p.y) != (b.y > p.y) && p.x < a.x + (p.y - a.y) / (b.y - a.y) * (b.x - a.x) {
            inside = !inside;
        }
    }
    inside
}

#[cfg(test)]
mod split_tests {
    use super::*;
    use crate::{CutPattern, Layer, LayerFunction, NewWall, ReferenceLine, WallFunction};

    /// A wall split in two: the second part takes the windows beyond the
    /// cut and the part of a painted band beyond it.
    #[test]
    fn a_wall_splits_with_its_openings_and_paint() {
        let mut m = Model::new("t");
        let l = m.add_level("PB", 0.0);
        let mat = m.add_material("Ladrillo", "brick", [180, 90, 60], CutPattern::Solid);
        let paint = m.add_material("Pintura", "paint", [240, 240, 240], CutPattern::Solid);
        let t = m
            .add_wall_type(
                "M",
                WallFunction::Exterior,
                vec![Layer {
                    material: mat,
                    thickness: 0.2,
                    function: LayerFunction::Structure,
                }],
            )
            .unwrap();
        let w = m
            .add_wall(NewWall {
                wall_type: t,
                level: l,
                start: Point2::new(0.0, 0.0),
                end: Point2::new(8.0, 0.0),
                reference: ReferenceLine::Center,
                height: 3.0,
                base_offset: 0.0,
                bulge: 0.0,
                top: WallTop::Unconnected,
            })
            .unwrap();
        let ot = m
            .add_opening_type(NewOpeningType {
                name: "V".into(),
                kind: OpeningKind::Window(WindowOperation::DoubleCasement),
                width: 1.0,
                height: 1.2,
                frame_width: 0.05,
                frame_depth: 0.07,
                frame_material: mat,
                panel_material: mat,
            })
            .unwrap();
        let place = |along| NewOpening {
            opening_type: ot,
            wall: w,
            along,
            sill: 0.9,
            opens_to: WallSide::Interior,
            hand: Hand::Left,
            setback: None,
        };
        let a = m.add_opening(place(2.0)).unwrap();
        let b = m.add_opening(place(6.0)).unwrap();
        m.set_paint_zone(
            w,
            FaceSide::Interior,
            vec![
                Point2::new(0.0, 0.0),
                Point2::new(8.0, 0.0),
                Point2::new(8.0, 1.0),
                Point2::new(0.0, 1.0),
            ],
            Some(paint),
        )
        .unwrap();
        // Not through a window.
        assert!(m.split_wall(w, 2.2).is_err());
        let n = m.split_wall(w, 5.0).unwrap();
        assert_eq!(m.wall(w).unwrap().end, Point2::new(5.0, 0.0));
        assert_eq!(m.wall(n).unwrap().start, Point2::new(5.0, 0.0));
        assert_eq!(m.opening(a).unwrap().wall, w);
        let ob = m.opening(b).unwrap();
        assert_eq!((ob.wall, ob.along), (n, 1.0));
        let zones: Vec<_> = m.paint_zones().filter(|z| z.wall == n).collect();
        assert_eq!(zones.len(), 1);
        let xmax = zones[0].outline.iter().map(|p| p.x).fold(0.0, f64::max);
        assert!((xmax - 3.0).abs() < 1e-9);
    }
}
