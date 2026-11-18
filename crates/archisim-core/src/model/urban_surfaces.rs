//! Urban surfaces drawn by their outline (N5, `docs/TERRITORIO.md` §2.8):
//! squares, raised islands, medians, car parks and gardens, each with its
//! paving, its edge (kerb, gutter or band of stone) and, as it fits, its
//! joints or its parking bays — as Surface Painter and Parking Lot
//! Snapping do in Cities: Skylines. They sit on what is under them (the
//! ground or the street), raised by their own height.

use super::*;

/// What an urban surface is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Hash)]
pub enum SurfaceKind {
    /// A square or any paved area for people.
    #[default]
    Square,
    /// A traffic island, raised with its kerb.
    Island,
    /// A median between carriageways, raised with its kerb.
    Median,
    /// A car park with its bays painted.
    Parking,
    /// A garden: soil with its kerb.
    Garden,
}

impl SurfaceKind {
    pub const ALL: [SurfaceKind; 5] = [
        SurfaceKind::Square,
        SurfaceKind::Island,
        SurfaceKind::Median,
        SurfaceKind::Parking,
        SurfaceKind::Garden,
    ];

    pub fn label(self) -> &'static str {
        match self {
            SurfaceKind::Square => "Plaza",
            SurfaceKind::Island => "Isleta",
            SurfaceKind::Median => "Mediana",
            SurfaceKind::Parking => "Aparcamiento",
            SurfaceKind::Garden => "Jardín",
        }
    }

    pub fn code(self) -> &'static str {
        match self {
            SurfaceKind::Square => "square",
            SurfaceKind::Island => "island",
            SurfaceKind::Median => "median",
            SurfaceKind::Parking => "parking",
            SurfaceKind::Garden => "garden",
        }
    }

    pub fn from_code(c: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|k| k.code() == c)
    }

    /// Its usual edge, edge width (m), height over what is under it (m),
    /// joints (m between them; 0: none) and bays.
    pub fn usual(self) -> (SurfaceEdge, f64, f64, f64, Option<Bays>) {
        match self {
            SurfaceKind::Square => (SurfaceEdge::Band, 0.3, 0.0, 1.0, None),
            SurfaceKind::Island | SurfaceKind::Median => (SurfaceEdge::Kerb, 0.15, 0.15, 0.0, None),
            SurfaceKind::Parking => (SurfaceEdge::Gutter, 0.3, 0.0, 0.0, Some(Bays::default())),
            SurfaceKind::Garden => (SurfaceEdge::Kerb, 0.1, 0.1, 0.0, None),
        }
    }
}

/// The edge round an urban surface.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Hash)]
pub enum SurfaceEdge {
    None,
    /// A kerb, standing up with the surface.
    #[default]
    Kerb,
    /// A gutter («rigola»), flush, of another material.
    Gutter,
    /// A band of stone («encintado»), flush.
    Band,
}

impl SurfaceEdge {
    pub const ALL: [SurfaceEdge; 4] = [
        SurfaceEdge::None,
        SurfaceEdge::Kerb,
        SurfaceEdge::Gutter,
        SurfaceEdge::Band,
    ];

    pub fn label(self) -> &'static str {
        match self {
            SurfaceEdge::None => "Sin borde",
            SurfaceEdge::Kerb => "Bordillo",
            SurfaceEdge::Gutter => "Rigola",
            SurfaceEdge::Band => "Encintado",
        }
    }

    pub fn code(self) -> &'static str {
        match self {
            SurfaceEdge::None => "none",
            SurfaceEdge::Kerb => "kerb",
            SurfaceEdge::Gutter => "gutter",
            SurfaceEdge::Band => "band",
        }
    }

    pub fn from_code(c: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|k| k.code() == c)
    }
}

/// The bays of a car park: along its longest side, `width` wide, `depth`
/// deep, at `angle` to the side (90°: in battery; 0°: in line).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Bays {
    pub width: f64,
    pub depth: f64,
    /// Degrees.
    pub angle: f64,
}

impl Default for Bays {
    fn default() -> Self {
        Bays {
            width: 2.5,
            depth: 5.0,
            angle: 90.0,
        }
    }
}

/// An urban surface.
#[derive(Debug, Clone, PartialEq)]
pub struct UrbanSurface {
    pub id: SurfaceId,
    pub guid: GlobalId,
    pub name: String,
    pub level: LevelId,
    pub kind: SurfaceKind,
    /// Its outline, counter-clockwise.
    pub outline: Vec<Point2>,
    pub material: MaterialId,
    pub edge: SurfaceEdge,
    pub edge_width: f64,
    pub edge_material: MaterialId,
    /// How far it stands over what is under it.
    pub raise: f64,
    /// Its paving joints: metres between them (0: none) and their turn
    /// from the plan's x axis (radians).
    pub joints: f64,
    pub joint_angle: f64,
    /// A car park's bays.
    pub bays: Option<Bays>,
}

/// Data needed to make an urban surface.
#[derive(Debug, Clone, PartialEq)]
pub struct NewSurface {
    pub name: String,
    pub level: LevelId,
    pub kind: SurfaceKind,
    pub outline: Vec<Point2>,
    pub material: MaterialId,
    pub edge_material: MaterialId,
}

/// The width of the lines of the bays, metres.
pub const BAY_LINE: f64 = 0.1;

impl UrbanSurface {
    /// Its outline inside its edge.
    pub fn inner(&self) -> Vec<Point2> {
        if self.edge == SurfaceEdge::None || self.edge_width <= 0.0 {
            return self.outline.clone();
        }
        grow_polygon(&self.outline, -self.edge_width)
    }

    pub fn area(&self) -> f64 {
        crate::signed_area(&self.outline).abs()
    }

    /// Its longest side: its two ends.
    pub fn longest_side(&self) -> (Point2, Point2) {
        let n = self.outline.len();
        (0..n)
            .map(|i| (self.outline[i], self.outline[(i + 1) % n]))
            .max_by(|a, b| a.0.distance(a.1).total_cmp(&b.0.distance(b.1)))
            .unwrap_or((Point2::new(0.0, 0.0), Point2::new(1.0, 0.0)))
    }

    /// The lines of its parking bays, as thin polygons: rows along its
    /// longest side, one after another inwards as they fit, each bay
    /// marked by its sides (and the back of the row).
    pub fn bay_lines(&self) -> Vec<Vec<Point2>> {
        let Some(b) = self.bays else {
            return Vec::new();
        };
        let inner = self.inner();
        if inner.len() < 3 || b.width <= 0.5 || b.depth <= 1.0 {
            return Vec::new();
        }
        let (a, z) = self.longest_side();
        let Some(u) = (z - a).normalized() else {
            return Vec::new();
        };
        // Inwards: the outline is counter-clockwise.
        let n = u.perp_left();
        let t = b.angle.clamp(15.0, 90.0).to_radians();
        // A bay's side, from the row's front to its back.
        let side = u * t.cos() + n * t.sin();
        let across = b.depth * t.sin();
        let step = b.width / t.sin().max(0.2);
        // How far the surface reaches inwards and along.
        let (mut lo, mut hi, mut near, mut reach) = (f64::MAX, f64::MIN, f64::MAX, 0.0_f64);
        for q in &inner {
            let d = *q - a;
            lo = lo.min(d.dot(u));
            hi = hi.max(d.dot(u));
            near = near.min(d.dot(n));
            reach = reach.max(d.dot(n));
        }
        let inside = |p: Point2| point_in_polygon(p, &inner);
        // Rows: one open upwards, an aisle of 6 m, one open downwards,
        // and again back to back with the next.
        let mut rows: Vec<(f64, bool)> = Vec::new();
        let mut y = near.max(0.0) + 0.01;
        while rows.len() < 40 {
            if y + across > reach + 1e-6 {
                break;
            }
            rows.push((y, true));
            y += across + 6.0;
            if y + across > reach + 1e-6 {
                break;
            }
            rows.push((y + across, false));
            y += across;
        }
        let mut out = Vec::new();
        for (back, up) in rows {
            // From the back of the row to its front.
            let dir = if up { side } else { side * -1.0 };
            let base = a + n * back;
            let mut s = lo + 0.3;
            let mut last: Option<Point2> = None;
            while s <= hi + 1e-6 {
                let p = base + u * s;
                let q = p + dir * b.depth;
                let probe = p + n * if up { 0.05 } else { -0.05 };
                if inside(probe) && inside(q) {
                    out.push(strip(&[p, q], -BAY_LINE / 2.0, BAY_LINE / 2.0));
                    if let Some(l) = last {
                        // The back of the bay.
                        out.push(strip(&[l, p], -BAY_LINE / 2.0, BAY_LINE / 2.0));
                    }
                    last = Some(p);
                } else {
                    last = None;
                }
                s += step;
            }
        }
        out.retain(|p| p.len() >= 3);
        out
    }

    /// How many bays it has.
    pub fn bay_count(&self) -> usize {
        // Each bay has its back line (the first of a row has none).
        let lines = self.bay_lines();
        lines.len() / 2
    }

    /// The lines of its paving joints inside its inner outline, as
    /// segments.
    pub fn joint_lines(&self) -> Vec<(Point2, Point2)> {
        if self.joints <= 0.05 {
            return Vec::new();
        }
        let inner = self.inner();
        if inner.len() < 3 {
            return Vec::new();
        }
        let mut out = Vec::new();
        for turn in [
            self.joint_angle,
            self.joint_angle + std::f64::consts::FRAC_PI_2,
        ] {
            let u = Vec2::new(turn.cos(), turn.sin());
            let n = u.perp_left();
            let (mut lo, mut hi, mut a0, mut a1) = (f64::MAX, f64::MIN, f64::MAX, f64::MIN);
            for q in &inner {
                let v = Vec2::new(q.x, q.y);
                lo = lo.min(v.dot(n));
                hi = hi.max(v.dot(n));
                a0 = a0.min(v.dot(u));
                a1 = a1.max(v.dot(u));
            }
            let first = (lo / self.joints).ceil() * self.joints;
            let mut c = first;
            let mut count = 0;
            while c < hi && count < 2000 {
                let p = Point2::new(0.0, 0.0) + n * c + u * (a0 - 1.0);
                let q = Point2::new(0.0, 0.0) + n * c + u * (a1 + 1.0);
                for (t0, t1) in super::markings::inside_spans(&inner, p, q) {
                    out.push((p + (q - p) * t0, p + (q - p) * t1));
                }
                c += self.joints;
                count += 1;
            }
        }
        out
    }

    /// Its settings as a text, for the IFC: `kind edge edge_width raise
    /// joints joint_angle [bay_width bay_depth bay_angle]`.
    pub fn encode(&self) -> String {
        let mut s = format!(
            "{} {} {} {} {} {}",
            self.kind.code(),
            self.edge.code(),
            self.edge_width,
            self.raise,
            self.joints,
            self.joint_angle
        );
        if let Some(b) = self.bays {
            s += &format!(" {} {} {}", b.width, b.depth, b.angle);
        }
        s
    }

    /// Reads [`UrbanSurface::encode`] into this surface.
    pub fn decode_into(&mut self, text: &str) -> Option<()> {
        let f: Vec<&str> = text.split_whitespace().collect();
        if f.len() < 6 {
            return None;
        }
        self.kind = SurfaceKind::from_code(f[0])?;
        self.edge = SurfaceEdge::from_code(f[1])?;
        self.edge_width = f[2].parse().ok()?;
        self.raise = f[3].parse().ok()?;
        self.joints = f[4].parse().ok()?;
        self.joint_angle = f[5].parse().ok()?;
        self.bays = if f.len() >= 9 {
            Some(Bays {
                width: f[6].parse().ok()?,
                depth: f[7].parse().ok()?,
                angle: f[8].parse().ok()?,
            })
        } else {
            None
        };
        Some(())
    }

    fn check(&self) -> Result<(), ModelError> {
        if self.outline.len() < 3 || crate::signed_area(&self.outline).abs() < 0.01 {
            return Err(ModelError::InvalidOutline);
        }
        for q in &self.outline {
            finite_point(*q, "superficie")?;
        }
        let nums = [self.edge_width, self.raise, self.joints, self.joint_angle];
        if nums.iter().any(|x| !x.is_finite()) {
            return Err(ModelError::NotFinite("superficie"));
        }
        if self.edge_width < 0.0 || self.joints < 0.0 || self.raise < -1.0 {
            return Err(ModelError::NonPositive("superficie"));
        }
        if let Some(b) = self.bays {
            if !(b.width > 0.5 && b.depth > 1.0 && (0.0..=90.0).contains(&b.angle)) {
                return Err(ModelError::InvalidShape("plazas de aparcamiento"));
            }
        }
        Ok(())
    }
}

impl Model {
    fn validate_surface(&self, s: &UrbanSurface) -> Result<(), ModelError> {
        if self.level(s.level).is_none() {
            return Err(ModelError::UnknownLevel(s.level));
        }
        for m in [s.material, s.edge_material] {
            if self.material(m).is_none() {
                return Err(ModelError::UnknownMaterial(m));
            }
        }
        s.check()
    }

    /// A new urban surface with the usual edge, height, joints and bays of
    /// its kind.
    pub fn add_surface(&mut self, spec: NewSurface) -> Result<SurfaceId, ModelError> {
        let mut outline = spec.outline;
        if crate::signed_area(&outline) < 0.0 {
            outline.reverse();
        }
        let (edge, edge_width, raise, joints, bays) = spec.kind.usual();
        let s = UrbanSurface {
            id: SurfaceId(0),
            guid: GlobalId::new(),
            name: spec.name,
            level: spec.level,
            kind: spec.kind,
            outline,
            material: spec.material,
            edge,
            edge_width,
            edge_material: spec.edge_material,
            raise,
            joints,
            joint_angle: 0.0,
            bays,
        };
        self.validate_surface(&s)?;
        let id = SurfaceId(self.take_id());
        self.surfaces.insert(id, UrbanSurface { id, ..s });
        Ok(id)
    }

    /// A copy of a surface with another outline (moved, turned or
    /// mirrored): the same kind, materials, edge, joints and bays.
    pub fn copy_surface(
        &mut self,
        id: SurfaceId,
        outline: Vec<Point2>,
    ) -> Result<SurfaceId, ModelError> {
        let x = self
            .surface(id)
            .cloned()
            .ok_or(ModelError::UnknownElement(id.0))?;
        let new = self.add_surface(NewSurface {
            name: self.next_surface_name(x.kind),
            level: x.level,
            kind: x.kind,
            outline,
            material: x.material,
            edge_material: x.edge_material,
        })?;
        self.edit_surface(new, |s| {
            s.edge = x.edge;
            s.edge_width = x.edge_width;
            s.raise = x.raise;
            s.joints = x.joints;
            s.joint_angle = x.joint_angle;
            s.bays = x.bays;
        })?;
        Ok(new)
    }

    pub fn edit_surface(
        &mut self,
        id: SurfaceId,
        edit: impl FnOnce(&mut UrbanSurface),
    ) -> Result<(), ModelError> {
        let mut s = self
            .surfaces
            .get(&id)
            .cloned()
            .ok_or(ModelError::UnknownElement(id.0))?;
        edit(&mut s);
        if crate::signed_area(&s.outline) < 0.0 {
            s.outline.reverse();
        }
        self.validate_surface(&s)?;
        self.surfaces.insert(id, s);
        Ok(())
    }

    pub fn remove_surface(&mut self, id: SurfaceId) -> Result<UrbanSurface, ModelError> {
        self.surfaces
            .remove(&id)
            .ok_or(ModelError::UnknownElement(id.0))
    }

    pub fn surface(&self, id: SurfaceId) -> Option<&UrbanSurface> {
        self.surfaces.get(&id)
    }

    pub fn surfaces(&self) -> impl Iterator<Item = &UrbanSurface> {
        self.surfaces.values()
    }

    pub fn surfaces_on_level(&self, level: LevelId) -> impl Iterator<Item = &UrbanSurface> {
        self.surfaces.values().filter(move |s| s.level == level)
    }

    /// The next name for a surface of a kind: «Plaza 2».
    pub fn next_surface_name(&self, kind: SurfaceKind) -> String {
        let n = self.surfaces.values().filter(|s| s.kind == kind).count();
        format!("{} {}", kind.label(), n + 1)
    }

    pub(super) fn restore_surfaces(
        &mut self,
        list: Vec<UrbanSurface>,
        claim: &mut structure::Claim<'_>,
    ) -> Result<(), RestoreError> {
        for s in list {
            claim(s.id.0, Some(&s.guid))
                .and_then(|_| self.validate_surface(&s))
                .map_err(|error| RestoreError {
                    element: format!("surface {} «{}»", s.id.0, s.name),
                    error,
                })?;
            self.surfaces.insert(s.id, s);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn place() -> (Model, LevelId, MaterialId) {
        let mut m = Model::new("Superficies");
        let lv = m.add_level("PB", 0.0);
        let mat = m.add_material("Granito", "stone", [180, 180, 180], CutPattern::Solid);
        (m, lv, mat)
    }

    fn rect(x0: f64, y0: f64, x1: f64, y1: f64) -> Vec<Point2> {
        vec![
            Point2::new(x0, y0),
            Point2::new(x1, y0),
            Point2::new(x1, y1),
            Point2::new(x0, y1),
        ]
    }

    /// A car park of 40 × 17 m: two rows of bays in battery, back to back
    /// over an aisle; its settings come back from their text.
    #[test]
    fn a_car_park_has_its_bays() {
        let (mut m, lv, mat) = place();
        let id = m
            .add_surface(NewSurface {
                name: "Aparcamiento 1".into(),
                level: lv,
                kind: SurfaceKind::Parking,
                outline: rect(0.0, 0.0, 40.0, 17.0),
                material: mat,
                edge_material: mat,
            })
            .unwrap();
        let s = m.surface(id).unwrap().clone();
        assert_eq!(s.edge, SurfaceEdge::Gutter);
        // 39,4 m along, every 2,5 m: 15 bays a row, two rows.
        let n = s.bay_count();
        assert!((26..=32).contains(&n), "{n}");
        let mut back = s.clone();
        back.kind = SurfaceKind::Square;
        back.bays = None;
        back.decode_into(&s.encode()).unwrap();
        assert_eq!(back, s);
        // A square has its joints; drawn clockwise, it is turned round.
        let sq = m
            .add_surface(NewSurface {
                name: "Plaza 1".into(),
                level: lv,
                kind: SurfaceKind::Square,
                outline: rect(50.0, 0.0, 60.0, 10.0).into_iter().rev().collect(),
                material: mat,
                edge_material: mat,
            })
            .unwrap();
        let sq = m.surface(sq).unwrap();
        assert!(crate::signed_area(&sq.outline) > 0.0);
        // Inside the band of 0,3 m: 9,4 m, a joint every metre each way.
        let j = sq.joint_lines();
        assert!((16..=20).contains(&j.len()), "{}", j.len());
        assert!((sq.area() - 100.0).abs() < 1e-9);
        // A surface needs an outline.
        let bad = NewSurface {
            name: "x".into(),
            level: lv,
            kind: SurfaceKind::Island,
            outline: vec![Point2::new(0.0, 0.0), Point2::new(1.0, 0.0)],
            material: mat,
            edge_material: mat,
        };
        assert!(m.add_surface(bad).is_err());
    }
}
