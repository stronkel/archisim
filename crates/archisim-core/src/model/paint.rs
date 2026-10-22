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

impl Model {
    fn check_face(&self, element: u64, face: FaceRef) -> Result<(), ModelError> {
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
