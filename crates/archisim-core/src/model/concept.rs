//! The concept phase (`docs/INTERFAZ.md` §4, Concepto): volumes drawn
//! freely to study a building before it is one, as in SketchUp.
//!
//! A [`Mass`] is a prism on a level: a contour pushed up to a height (a
//! flat face while its height is zero), a band along a path with a
//! rectangular profile ("Seguir"), or a mass by floors, whose height is a
//! number of floors and which tells its built area, plot ratio and site
//! coverage as it is pushed. A mass can be turned into a building (walls on
//! its faces, slabs on its floors, a flat roof on top) and remembers it:
//! when the mass changes, the building can be made again from it.
//!
//! Masses are a design aid: they are drawn in plan, section and 3D, but
//! not exported to IFC (as Revit's masses), and not measured.

use super::*;

/// What a mass is.
#[derive(Debug, Clone, PartialEq)]
pub enum MassShape {
    /// A contour (counter-clockwise), pushed up.
    Prism(Vec<Point2>),
    /// A rectangular profile `width` wide along a path.
    Sweep { path: Vec<Point2>, width: f64 },
}

/// How a mass was turned into a building, and what it made.
#[derive(Debug, Clone, PartialEq)]
pub struct MassBuilding {
    pub wall_type: WallTypeId,
    /// The slab of the ground floor (on the ground) and of the others.
    pub ground_type: SlabTypeId,
    pub floor_type: SlabTypeId,
    pub roof_type: SlabTypeId,
    /// The elements it made (walls and slabs), by id.
    pub elements: Vec<u64>,
    /// The mass as it was then: its contour, base and height.
    pub made_from: (Vec<Point2>, f64, f64),
}

/// A volume of the concept phase.
#[derive(Debug, Clone, PartialEq)]
pub struct Mass {
    pub id: MassId,
    pub guid: GlobalId,
    pub name: String,
    pub level: LevelId,
    pub shape: MassShape,
    /// Its base over the level.
    pub base_offset: f64,
    /// Zero: a flat face, to push.
    pub height: f64,
    /// A mass by floors: the height of each (its height is a whole number
    /// of them).
    pub floor_height: Option<f64>,
    pub material: MaterialId,
    pub building: Option<MassBuilding>,
}

/// Data needed to make a mass.
#[derive(Debug, Clone, PartialEq)]
pub struct NewMass {
    pub name: String,
    pub level: LevelId,
    pub shape: MassShape,
    pub base_offset: f64,
    pub height: f64,
    pub floor_height: Option<f64>,
    pub material: MaterialId,
}

/// What a mass gives: floors, footprint and built area, and, with the plot
/// area, plot ratio and coverage.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MassFigures {
    pub floors: usize,
    /// m².
    pub footprint: f64,
    /// m² built (footprint × floors).
    pub built: f64,
    /// m² built per m² of plot.
    pub plot_ratio: Option<f64>,
    /// Footprint over plot, 0–1.
    pub coverage: Option<f64>,
}

impl Mass {
    /// The mass moved, turned, scaled or mirrored point by point.
    pub fn map_points(&mut self, f: impl Fn(Point2) -> Point2) {
        match &mut self.shape {
            MassShape::Prism(p) => p.iter_mut().for_each(|q| *q = f(*q)),
            MassShape::Sweep { path, .. } => path.iter_mut().for_each(|q| *q = f(*q)),
        }
    }

    /// Its outline in plan, counter-clockwise (a sweep's band, mitred at
    /// its corners).
    pub fn outline(&self) -> Vec<Point2> {
        match &self.shape {
            MassShape::Prism(p) => p.clone(),
            MassShape::Sweep { path, width } => band(path, *width),
        }
    }

    pub fn is_face(&self) -> bool {
        self.height <= EPSILON
    }

    /// Floors of a mass by floors (one for any other volume).
    pub fn floors(&self) -> usize {
        match self.floor_height {
            Some(f) if f > EPSILON => ((self.height / f).round() as usize).max(1),
            _ => 1,
        }
    }

    pub fn figures(&self, plot_area: Option<f64>) -> MassFigures {
        let footprint = signed_area(&self.outline()).abs();
        let floors = if self.is_face() { 0 } else { self.floors() };
        let built = footprint * floors as f64;
        let plot = plot_area.filter(|a| *a > EPSILON);
        MassFigures {
            floors,
            footprint,
            built,
            plot_ratio: plot.map(|a| built / a),
            coverage: plot.map(|a| footprint / a),
        }
    }

    /// Whether the mass changed since it was turned into a building.
    pub fn building_outdated(&self) -> bool {
        self.building.as_ref().is_some_and(|b| {
            let (o, base, h) = &b.made_from;
            *o != self.outline()
                || (base - self.base_offset).abs() > 1e-9
                || (h - self.height).abs() > 1e-9
        })
    }
}

/// A band `width` wide along a path, mitred at its corners.
fn band(path: &[Point2], width: f64) -> Vec<Point2> {
    if path.len() < 2 {
        return Vec::new();
    }
    let h = width / 2.0;
    let n = path.len();
    let dir = |i: usize| (path[i + 1] - path[i]).normalized();
    let offset = |side: f64| -> Option<Vec<Point2>> {
        let mut out = Vec::with_capacity(n);
        for (i, &p) in path.iter().enumerate() {
            let q = match (
                i.checked_sub(1).and_then(dir),
                (i + 1 < n).then(|| dir(i)).flatten(),
            ) {
                (None, Some(d)) | (Some(d), None) => p + d.perp_left() * (h * side),
                (Some(a), Some(b)) => {
                    let m = (a.perp_left() + b.perp_left()).normalized()?;
                    let cos = m.x * a.perp_left().x + m.y * a.perp_left().y;
                    p + m * (h * side / cos.max(0.2))
                }
                (None, None) => return None,
            };
            out.push(q);
        }
        Some(out)
    };
    let (Some(left), Some(mut right)) = (offset(1.0), offset(-1.0)) else {
        return Vec::new();
    };
    right.reverse();
    let mut out = right;
    out.extend(left);
    if signed_area(&out) < 0.0 {
        out.reverse();
    }
    out
}

/// The same contour `d` further out (negative: inwards), its sides moved
/// in parallel and meeting again at their corners.
pub fn offset_contour(poly: &[Point2], d: f64) -> Option<Vec<Point2>> {
    let n = poly.len();
    if n < 3 {
        return None;
    }
    let mut p: Vec<Point2> = poly.to_vec();
    if signed_area(&p) < 0.0 {
        p.reverse();
    }
    // Outwards is to the right of a counter-clockwise contour.
    let line = |i: usize| {
        let (a, b) = (p[i], p[(i + 1) % n]);
        let u = (b - a).normalized()?;
        let out = Point2::new(u.y, -u.x);
        Some((a + out * d, u))
    };
    let mut res = Vec::with_capacity(n);
    for i in 0..n {
        let (a0, u0) = line((i + n - 1) % n)?;
        let (a1, u1) = line(i)?;
        let den = u0.x * u1.y - u0.y * u1.x;
        if den.abs() < 1e-12 {
            res.push(a1);
            continue;
        }
        let w = a1 - a0;
        let t = (w.x * u1.y - w.y * u1.x) / den;
        res.push(a0 + u0 * t);
    }
    // Each side keeps its direction, or the contour turned inside out.
    for i in 0..n {
        let (_, u) = line(i)?;
        let e = res[(i + 1) % n] - res[i];
        if e.x * u.x + e.y * u.y <= EPSILON {
            return None;
        }
    }
    let area = signed_area(&res);
    (area > EPSILON).then_some(res)
}

impl Model {
    fn validate_mass(&self, m: &NewMass) -> Result<(), ModelError> {
        if !self.levels.contains_key(&m.level) {
            return Err(ModelError::UnknownLevel(m.level));
        }
        if !self.materials.contains_key(&m.material) {
            return Err(ModelError::UnknownMaterial(m.material));
        }
        finite(m.base_offset, "mass base")?;
        finite(m.height, "mass height")?;
        if m.height < 0.0 {
            return Err(ModelError::NonPositive("mass height"));
        }
        if let Some(f) = m.floor_height {
            finite(f, "floor height")?;
            if f <= EPSILON {
                return Err(ModelError::NonPositive("floor height"));
            }
        }
        match &m.shape {
            MassShape::Prism(p) => {
                for q in p {
                    finite_point(*q, "mass contour")?;
                }
                if p.len() < 3 || signed_area(p).abs() <= EPSILON {
                    return Err(ModelError::InvalidShape(
                        "a mass needs a contour of three points or more",
                    ));
                }
            }
            MassShape::Sweep { path, width } => {
                for q in path {
                    finite_point(*q, "mass path")?;
                }
                finite(*width, "mass width")?;
                if *width <= EPSILON {
                    return Err(ModelError::NonPositive("mass width"));
                }
                if path.len() < 2 || path.windows(2).any(|w| w[0].approx_eq(w[1])) {
                    return Err(ModelError::InvalidShape(
                        "a mass along a path needs two different points",
                    ));
                }
            }
        }
        Ok(())
    }

    /// Makes a mass. A contour drawn clockwise is turned round.
    pub fn add_mass(&mut self, spec: NewMass) -> Result<MassId, ModelError> {
        let mut spec = spec;
        if let MassShape::Prism(p) = &mut spec.shape {
            if signed_area(p) < 0.0 {
                p.reverse();
            }
        }
        self.validate_mass(&spec)?;
        let id = MassId(self.take_id());
        self.masses.insert(
            id,
            Mass {
                id,
                guid: GlobalId::new(),
                name: spec.name,
                level: spec.level,
                shape: spec.shape,
                base_offset: spec.base_offset,
                height: spec.height,
                floor_height: spec.floor_height,
                material: spec.material,
                building: None,
            },
        );
        Ok(id)
    }

    pub fn edit_mass(
        &mut self,
        id: MassId,
        edit: impl FnOnce(&mut Mass),
    ) -> Result<(), ModelError> {
        let mut m = self
            .masses
            .get(&id)
            .cloned()
            .ok_or(ModelError::UnknownElement(id.0))?;
        edit(&mut m);
        if let MassShape::Prism(p) = &mut m.shape {
            if signed_area(p) < 0.0 {
                p.reverse();
            }
        }
        self.validate_mass(&NewMass {
            name: m.name.clone(),
            level: m.level,
            shape: m.shape.clone(),
            base_offset: m.base_offset,
            height: m.height,
            floor_height: m.floor_height,
            material: m.material,
        })?;
        self.masses.insert(id, m);
        Ok(())
    }

    pub fn remove_mass(&mut self, id: MassId) -> Result<Mass, ModelError> {
        self.masses
            .remove(&id)
            .ok_or(ModelError::UnknownElement(id.0))
    }

    pub fn mass(&self, id: MassId) -> Option<&Mass> {
        self.masses.get(&id)
    }

    pub fn masses(&self) -> impl Iterator<Item = &Mass> {
        self.masses.values()
    }

    pub fn masses_on_level(&self, level: LevelId) -> impl Iterator<Item = &Mass> {
        self.masses.values().filter(move |m| m.level == level)
    }

    /// A name for a new mass: «Volumen 3», «Masa 2».
    pub fn next_mass_name(&self, by_floors: bool) -> String {
        let stem = if by_floors { "Masa" } else { "Volumen" };
        (1..)
            .map(|n: usize| format!("{stem} {n}"))
            .find(|n| !self.masses.values().any(|m| m.name == *n))
            .unwrap_or_default()
    }

    /// The plot area the plot ratio and coverage are worked out with, m².
    pub fn plot_area(&self) -> Option<f64> {
        self.plot_area
    }

    pub fn set_plot_area(&mut self, area: Option<f64>) -> Result<(), ModelError> {
        if let Some(a) = area {
            finite(a, "plot area")?;
            if a <= EPSILON {
                return Err(ModelError::NonPositive("plot area"));
            }
        }
        self.plot_area = area;
        Ok(())
    }

    // ---- To a building -----------------------------------------------------

    /// The level at an elevation (within a centimetre), made if missing.
    fn level_at(&mut self, elevation: f64, name: &str) -> LevelId {
        if let Some(l) = self
            .levels
            .values()
            .find(|l| (l.elevation - elevation).abs() < 0.01)
        {
            return l.id;
        }
        let mut n = name.to_string();
        let mut k = 2;
        while self.levels.values().any(|l| l.name == n) {
            n = format!("{name} ({k})");
            k += 1;
        }
        self.add_level(n, elevation)
    }

    /// Turns a mass into a building: walls of `wall_type` along its faces
    /// (inside them, the exterior face on the contour), a slab on each
    /// floor (the lowest of `ground_type`, the others of `floor_type`) and
    /// a flat roof of `roof_type` on top, with the levels each floor needs.
    /// A building made before from the mass is made again. Returns how
    /// many elements it made.
    pub fn mass_to_building(
        &mut self,
        id: MassId,
        wall_type: WallTypeId,
        ground_type: SlabTypeId,
        floor_type: SlabTypeId,
        roof_type: SlabTypeId,
    ) -> Result<usize, ModelError> {
        let m = self
            .masses
            .get(&id)
            .cloned()
            .ok_or(ModelError::UnknownElement(id.0))?;
        if !matches!(m.shape, MassShape::Prism(_)) || m.is_face() {
            return Err(ModelError::InvalidShape(
                "only volumes and masses with a height become buildings",
            ));
        }
        for t in [ground_type, floor_type, roof_type] {
            if !self.slab_types.contains_key(&t) {
                return Err(ModelError::UnknownSlabType(t));
            }
        }
        if !self.wall_types.contains_key(&wall_type) {
            return Err(ModelError::UnknownWallType(wall_type));
        }
        // The building made before goes.
        if let Some(b) = &m.building {
            self.remove_made(&b.elements);
        }
        let outline = m.outline();
        let floors = m.floors();
        let floor_height = m.height / floors as f64;
        let base = self.level(m.level).map_or(0.0, |l| l.elevation) + m.base_offset;
        let levels: Vec<LevelId> = (0..=floors)
            .map(|i| {
                let name = if i == floors {
                    "Cubierta".to_string()
                } else if i == 0 {
                    "Planta baja".to_string()
                } else {
                    format!("Planta {i}")
                };
                if i == 0 && m.base_offset.abs() < 0.01 {
                    m.level
                } else {
                    self.level_at(base + i as f64 * floor_height, &name)
                }
            })
            .collect();
        let mut made = Vec::new();
        let n = outline.len();
        for f in 0..floors {
            // Clockwise, so the exterior (to the left of each wall) is out.
            for i in 0..n {
                let (a, b) = (outline[(i + 1) % n], outline[i]);
                let w = self.add_wall(NewWall {
                    wall_type,
                    level: levels[f],
                    start: a,
                    end: b,
                    reference: ReferenceLine::ExteriorFace,
                    height: floor_height,
                    base_offset: 0.0,
                    bulge: 0.0,
                    top: WallTop::Level {
                        level: levels[f + 1],
                        offset: 0.0,
                    },
                })?;
                made.push(w.0);
            }
            let s = self.add_slab(NewSlab {
                slab_type: if f == 0 { ground_type } else { floor_type },
                level: levels[f],
                outline: outline.clone(),
                top_offset: 0.0,
            })?;
            made.push(s.0);
        }
        let roof = self.add_slab(NewSlab {
            slab_type: roof_type,
            level: levels[floors],
            outline: outline.clone(),
            top_offset: 0.0,
        })?;
        made.push(roof.0);
        let count = made.len();
        let mass = self.masses.get_mut(&id).expect("checked");
        mass.building = Some(MassBuilding {
            wall_type,
            ground_type,
            floor_type,
            roof_type,
            elements: made,
            made_from: (outline, m.base_offset, m.height),
        });
        Ok(count)
    }

    /// Removes the walls and slabs a mass made that are still there.
    fn remove_made(&mut self, ids: &[u64]) {
        for id in ids {
            if self.walls.contains_key(&WallId(*id)) {
                let _ = self.remove_wall(WallId(*id));
            } else if self.slabs.contains_key(&SlabId(*id)) {
                let _ = self.remove_slab(SlabId(*id));
            }
        }
    }

    pub(super) fn restore_masses(
        &mut self,
        masses: Vec<Mass>,
        claim: &mut structure::Claim<'_>,
    ) -> Result<(), RestoreError> {
        for m in masses {
            let spec = NewMass {
                name: m.name.clone(),
                level: m.level,
                shape: m.shape.clone(),
                base_offset: m.base_offset,
                height: m.height,
                floor_height: m.floor_height,
                material: m.material,
            };
            claim(m.id.0, Some(&m.guid))
                .and_then(|_| self.validate_mass(&spec))
                .map_err(|error| RestoreError {
                    element: format!("mass {} «{}»", m.id.0, m.name),
                    error,
                })?;
            self.masses.insert(m.id, m);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn project() -> (Model, LevelId, MaterialId, WallTypeId, SlabTypeId) {
        let mut m = Model::new("Concepto");
        let l = m.add_level("Planta baja", 0.0);
        let mat = m.add_material("Hormigón", "concrete", [180, 180, 180], CutPattern::Cross);
        let wt = m
            .add_wall_type(
                "Fachada",
                WallFunction::Exterior,
                vec![Layer {
                    material: mat,
                    thickness: 0.3,
                    function: LayerFunction::Structure,
                }],
            )
            .unwrap();
        let st = m
            .add_slab_type(
                "Forjado",
                SlabFunction::Floor,
                vec![Layer {
                    material: mat,
                    thickness: 0.3,
                    function: LayerFunction::Structure,
                }],
            )
            .unwrap();
        (m, l, mat, wt, st)
    }

    fn square(s: f64) -> Vec<Point2> {
        vec![
            Point2::new(0.0, 0.0),
            Point2::new(s, 0.0),
            Point2::new(s, s),
            Point2::new(0.0, s),
        ]
    }

    #[test]
    fn a_mass_by_floors_tells_its_figures() {
        let (mut m, l, mat, _, _) = project();
        m.set_plot_area(Some(400.0)).unwrap();
        let id = m
            .add_mass(NewMass {
                name: m.next_mass_name(true),
                level: l,
                // Drawn clockwise: turned round.
                shape: MassShape::Prism(square(10.0).into_iter().rev().collect()),
                base_offset: 0.0,
                height: 9.0,
                floor_height: Some(3.0),
                material: mat,
            })
            .unwrap();
        let mass = m.mass(id).unwrap();
        assert_eq!(mass.name, "Masa 1");
        assert!(signed_area(&mass.outline()) > 0.0);
        let f = mass.figures(m.plot_area());
        assert_eq!(f.floors, 3);
        assert!((f.built - 300.0).abs() < 1e-9);
        assert!((f.plot_ratio.unwrap() - 0.75).abs() < 1e-9);
        assert!((f.coverage.unwrap() - 0.25).abs() < 1e-9);
    }

    #[test]
    fn a_mass_becomes_a_building_and_again_when_it_changes() {
        let (mut m, l, mat, wt, st) = project();
        let id = m
            .add_mass(NewMass {
                name: "Bloque".into(),
                level: l,
                shape: MassShape::Prism(square(10.0)),
                base_offset: 0.0,
                height: 6.0,
                floor_height: Some(3.0),
                material: mat,
            })
            .unwrap();
        let n = m.mass_to_building(id, wt, st, st, st).unwrap();
        // Two floors of four walls and a slab, and the roof.
        assert_eq!(n, 11);
        assert_eq!(m.levels().len(), 3);
        assert!(!m.mass(id).unwrap().building_outdated());
        // The walls are inside the contour: their exterior face on it.
        let w = m.walls().next().unwrap();
        assert_eq!(w.reference, ReferenceLine::ExteriorFace);
        m.edit_mass(id, |x| x.height = 9.0).unwrap();
        assert!(m.mass(id).unwrap().building_outdated());
        let n = m.mass_to_building(id, wt, st, st, st).unwrap();
        assert_eq!(n, 16);
        assert_eq!(m.walls().count(), 12);
        assert_eq!(m.slabs().count(), 4);
    }

    #[test]
    fn contours_are_offset_and_paths_make_bands() {
        let out = offset_contour(&square(10.0), 1.0).unwrap();
        assert!((signed_area(&out) - 144.0).abs() < 1e-9);
        let inside = offset_contour(&square(10.0), -1.0).unwrap();
        assert!((signed_area(&inside) - 64.0).abs() < 1e-9);
        assert!(offset_contour(&square(10.0), -6.0).is_none());
        let b = band(
            &[
                Point2::new(0.0, 0.0),
                Point2::new(10.0, 0.0),
                Point2::new(10.0, 10.0),
            ],
            2.0,
        );
        assert_eq!(b.len(), 6);
        // 2 m wide, 20 m along its centre line, corner mitred.
        assert!((signed_area(&b) - 40.0).abs() < 1e-9);
    }
}
