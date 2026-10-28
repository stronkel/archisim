//! Swimming pools built in the ground (paquete 8, tanda 8d): the pool as a
//! constructive element, drawn by its outline like a slab, which takes its
//! room out of the terrain.
//!
//! A [`Pool`]'s outline is the inside face of its shell (the edge of the
//! water, counter-clockwise). Its bottom slopes from the shallow end to the
//! deep one along a direction in plan; its coping sits `top_offset` over
//! its level and the water `freeboard` under the coping. Round the outline
//! go the shell (`wall` thick, its inside lined) and the coping (`coping`
//! wide). [`Model::excavations`] gives what every pool takes out of the
//! ground, the pools that are objects of the library too, so the terrains
//! are cut by them. IFC: an `IfcElementAssembly` «Piscina» with its floor
//! (`IfcSlab`), its walls (`IfcWall`), its coping (`IfcSlab`) and the
//! water as an `IfcSpace`.

use super::*;

/// A pool built in the ground.
#[derive(Debug, Clone, PartialEq)]
pub struct Pool {
    pub id: PoolId,
    pub guid: GlobalId,
    pub name: String,
    pub level: LevelId,
    /// The inside face of its shell, counter-clockwise.
    pub outline: Vec<Point2>,
    /// The coping's top over the level.
    pub top_offset: f64,
    /// The water's depth at the shallow end and at the deep end.
    pub shallow: f64,
    pub deep: f64,
    /// Direction from the shallow end to the deep one, radians from the
    /// plan's `x`.
    pub slope: f64,
    /// How far the water is under the coping's top.
    pub freeboard: f64,
    /// Thickness of the shell and width of the coping.
    pub wall: f64,
    pub coping: f64,
    pub lining: MaterialId,
    pub shell: MaterialId,
    pub coping_material: MaterialId,
    pub water: MaterialId,
}

/// Data needed to make a pool.
#[derive(Debug, Clone, PartialEq)]
pub struct NewPool {
    pub name: String,
    pub level: LevelId,
    pub outline: Vec<Point2>,
    pub top_offset: f64,
    pub shallow: f64,
    pub deep: f64,
    pub slope: f64,
    pub freeboard: f64,
    pub wall: f64,
    pub coping: f64,
    pub lining: MaterialId,
    pub shell: MaterialId,
    pub coping_material: MaterialId,
    pub water: MaterialId,
}

impl NewPool {
    /// A pool with the usual measurements: water from 1,00 to 2,00 m deep,
    /// 12 cm under the coping, a shell of 25 cm and a coping of 30 cm.
    pub fn new(level: LevelId, outline: Vec<Point2>, material: MaterialId) -> Self {
        NewPool {
            name: String::new(),
            level,
            outline,
            top_offset: 0.0,
            shallow: 1.0,
            deep: 2.0,
            slope: 0.0,
            freeboard: 0.12,
            wall: 0.25,
            coping: 0.3,
            lining: material,
            shell: material,
            coping_material: material,
            water: material,
        }
    }
}

/// What a pool takes out of the ground: the outside of its shell in plan
/// and the level of its bottom (absolute).
#[derive(Debug, Clone, PartialEq)]
pub struct Excavation {
    pub outline: Vec<Point2>,
    pub bottom: f64,
}

impl Pool {
    pub fn spec(&self) -> NewPool {
        NewPool {
            name: self.name.clone(),
            level: self.level,
            outline: self.outline.clone(),
            top_offset: self.top_offset,
            shallow: self.shallow,
            deep: self.deep,
            slope: self.slope,
            freeboard: self.freeboard,
            wall: self.wall,
            coping: self.coping,
            lining: self.lining,
            shell: self.shell,
            coping_material: self.coping_material,
            water: self.water,
        }
    }

    /// The area of its water.
    pub fn area(&self) -> f64 {
        signed_area(&self.outline).abs()
    }

    pub fn perimeter(&self) -> f64 {
        let n = self.outline.len();
        (0..n)
            .map(|i| self.outline[i].distance(self.outline[(i + 1) % n]))
            .sum()
    }

    /// Where the outline is along the slope: its least and greatest.
    fn span(&self) -> (f64, f64) {
        let (c, s) = (self.slope.cos(), self.slope.sin());
        self.outline
            .iter()
            .map(|p| p.x * c + p.y * s)
            .fold((f64::MAX, f64::MIN), |(lo, hi), u| (lo.min(u), hi.max(u)))
    }

    /// The water's depth at a point in plan.
    pub fn depth_at(&self, p: Point2) -> f64 {
        let (lo, hi) = self.span();
        let u = p.x * self.slope.cos() + p.y * self.slope.sin();
        let t = if hi - lo > EPSILON {
            ((u - lo) / (hi - lo)).clamp(0.0, 1.0)
        } else {
            0.0
        };
        self.shallow + (self.deep - self.shallow) * t
    }

    /// The water's face over the level.
    pub fn water_level(&self) -> f64 {
        self.top_offset - self.freeboard
    }

    /// The finished floor of the pool over the level at a point in plan.
    pub fn floor_at(&self, p: Point2) -> f64 {
        self.water_level() - self.depth_at(p)
    }

    /// The volume of its water, m³: its area by its mean depth (the depth
    /// varies along the slope, linearly, so the mean over the area is the
    /// depth at its centroid).
    pub fn water_volume(&self) -> f64 {
        let c = centroid(&self.outline);
        self.area() * self.depth_at(c)
    }

    /// The outside of its shell.
    pub fn outer(&self) -> Vec<Point2> {
        offset_contour(&self.outline, self.wall).unwrap_or_else(|| self.outline.clone())
    }

    /// The outside of its coping.
    pub fn coping_outline(&self) -> Vec<Point2> {
        offset_contour(&self.outline, self.coping).unwrap_or_else(|| self.outline.clone())
    }
}

/// The centroid of a polygon.
fn centroid(poly: &[Point2]) -> Point2 {
    let a = signed_area(poly);
    if a.abs() <= EPSILON {
        let n = poly.len().max(1) as f64;
        return poly.iter().fold(Point2::new(0.0, 0.0), |s, p| s + *p) * (1.0 / n);
    }
    let n = poly.len();
    let (mut cx, mut cy) = (0.0, 0.0);
    for i in 0..n {
        let (p, q) = (poly[i], poly[(i + 1) % n]);
        let k = p.cross(q);
        cx += (p.x + q.x) * k;
        cy += (p.y + q.y) * k;
    }
    Point2::new(cx / (6.0 * a), cy / (6.0 * a))
}

impl Model {
    fn validate_pool(&self, p: &NewPool) -> Result<(), ModelError> {
        if !self.levels.contains_key(&p.level) {
            return Err(ModelError::UnknownLevel(p.level));
        }
        for m in [p.lining, p.shell, p.coping_material, p.water] {
            if !self.materials.contains_key(&m) {
                return Err(ModelError::UnknownMaterial(m));
            }
        }
        for q in &p.outline {
            finite_point(*q, "pool outline")?;
        }
        if p.outline.len() < 3 || signed_area(&p.outline).abs() <= EPSILON {
            return Err(ModelError::InvalidShape(
                "a pool needs an outline of three points or more",
            ));
        }
        for (v, what) in [
            (p.shallow, "pool shallow depth"),
            (p.deep, "pool deep depth"),
            (p.wall, "pool shell"),
        ] {
            finite(v, what)?;
            if v <= EPSILON {
                return Err(ModelError::NonPositive(what));
            }
        }
        for (v, what) in [(p.freeboard, "pool freeboard"), (p.coping, "pool coping")] {
            finite(v, what)?;
            if v < 0.0 {
                return Err(ModelError::NonPositive(what));
            }
        }
        finite(p.top_offset, "pool top offset")?;
        finite(p.slope, "pool slope")?;
        Ok(())
    }

    pub fn add_pool(&mut self, spec: NewPool) -> Result<PoolId, ModelError> {
        let mut spec = spec;
        if signed_area(&spec.outline) < 0.0 {
            spec.outline.reverse();
        }
        self.validate_pool(&spec)?;
        let id = PoolId(self.take_id());
        let name = if spec.name.is_empty() {
            self.next_pool_name()
        } else {
            spec.name
        };
        self.pools.insert(
            id,
            Pool {
                id,
                guid: GlobalId::new(),
                name,
                level: spec.level,
                outline: spec.outline,
                top_offset: spec.top_offset,
                shallow: spec.shallow,
                deep: spec.deep,
                slope: spec.slope,
                freeboard: spec.freeboard,
                wall: spec.wall,
                coping: spec.coping,
                lining: spec.lining,
                shell: spec.shell,
                coping_material: spec.coping_material,
                water: spec.water,
            },
        );
        Ok(id)
    }

    pub fn edit_pool(
        &mut self,
        id: PoolId,
        edit: impl FnOnce(&mut Pool),
    ) -> Result<(), ModelError> {
        let mut p = self
            .pools
            .get(&id)
            .cloned()
            .ok_or(ModelError::UnknownElement(id.0))?;
        edit(&mut p);
        if signed_area(&p.outline) < 0.0 {
            p.outline.reverse();
        }
        self.validate_pool(&p.spec())?;
        self.pools.insert(id, p);
        Ok(())
    }

    pub fn remove_pool(&mut self, id: PoolId) -> Result<Pool, ModelError> {
        self.pools
            .remove(&id)
            .ok_or(ModelError::UnknownElement(id.0))
    }

    pub fn pool(&self, id: PoolId) -> Option<&Pool> {
        self.pools.get(&id)
    }

    pub fn pools(&self) -> impl Iterator<Item = &Pool> {
        self.pools.values()
    }

    pub fn pools_on_level(&self, level: LevelId) -> impl Iterator<Item = &Pool> {
        self.pools.values().filter(move |p| p.level == level)
    }

    /// A name for a new pool: «Piscina 1».
    pub fn next_pool_name(&self) -> String {
        (1..)
            .map(|n: usize| format!("Piscina {n}"))
            .find(|n| !self.pools.values().any(|p| p.name == *n))
            .unwrap_or_default()
    }

    /// What every pool takes out of the ground: the pools built here and
    /// the pools that are objects of the library (`props:fine_pool`).
    pub fn excavations(&self) -> Vec<Excavation> {
        let mut out: Vec<Excavation> = self
            .pools
            .values()
            .map(|p| {
                let z0 = self.level(p.level).map_or(0.0, |l| l.elevation);
                let deepest = p.deep.max(p.shallow);
                Excavation {
                    outline: p.outer(),
                    bottom: z0 + p.water_level() - deepest - p.wall,
                }
            })
            .collect();
        for o in self.objects.values() {
            let Some((kind, params)) = self.prop_params(o.object_type) else {
                continue;
            };
            if kind != "fine_pool" {
                continue;
            }
            let (hole, bottom) = archisim_props::fine::pool::excavation(&params);
            let (c, s) = (o.rotation.cos(), o.rotation.sin());
            let outline = hole
                .iter()
                .map(|q| {
                    Point2::new(
                        o.position.x + q[0] * c - q[1] * s,
                        o.position.y + q[0] * s + q[1] * c,
                    )
                })
                .collect();
            let z0 = self.level(o.level).map_or(0.0, |l| l.elevation) + o.base_offset;
            out.push(Excavation {
                outline,
                bottom: z0 + bottom,
            });
        }
        out
    }

    pub(super) fn restore_pools(
        &mut self,
        pools: Vec<Pool>,
        claim: &mut structure::Claim<'_>,
    ) -> Result<(), RestoreError> {
        for p in pools {
            claim(p.id.0, Some(&p.guid))
                .and_then(|_| self.validate_pool(&p.spec()))
                .map_err(|error| RestoreError {
                    element: format!("pool {} «{}»", p.id.0, p.name),
                    error,
                })?;
            self.pools.insert(p.id, p);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn square(m: &mut Model, side: f64) -> PoolId {
        let level = m.add_level("Planta baja", 0.0);
        let mat = m.add_material("Gresite", "ceramic", [80, 160, 200], CutPattern::Solid);
        let o = vec![
            Point2::new(0.0, 0.0),
            Point2::new(side, 0.0),
            Point2::new(side, side / 2.0),
            Point2::new(0.0, side / 2.0),
        ];
        m.add_pool(NewPool::new(level, o, mat)).unwrap()
    }

    #[test]
    fn a_pool_slopes_from_shallow_to_deep() {
        let mut m = Model::new("Piscinas");
        let id = square(&mut m, 8.0);
        let p = m.pool(id).unwrap();
        assert_eq!(p.name, "Piscina 1");
        assert!((p.depth_at(Point2::new(0.0, 1.0)) - 1.0).abs() < 1e-9);
        assert!((p.depth_at(Point2::new(8.0, 1.0)) - 2.0).abs() < 1e-9);
        assert!((p.water_volume() - 32.0 * 1.5).abs() < 1e-9);
        assert!((p.floor_at(Point2::new(8.0, 1.0)) - (-2.12)).abs() < 1e-9);
        // Turned the other way, the deep end is at the left.
        m.edit_pool(id, |p| p.slope = std::f64::consts::PI).unwrap();
        let p = m.pool(id).unwrap();
        assert!((p.depth_at(Point2::new(0.0, 1.0)) - 2.0).abs() < 1e-9);
    }

    #[test]
    fn pools_take_their_room_out_of_the_ground() {
        let mut m = Model::new("Piscinas");
        square(&mut m, 8.0);
        let e = m.excavations();
        assert_eq!(e.len(), 1);
        assert!((e[0].bottom - (-0.12 - 2.0 - 0.25)).abs() < 1e-9);
        assert!(signed_area(&e[0].outline) > 32.0);
    }

    #[test]
    fn a_pool_needs_room_and_depth() {
        let mut m = Model::new("Piscinas");
        let id = square(&mut m, 8.0);
        assert!(m.edit_pool(id, |p| p.deep = 0.0).is_err());
        assert!(m.edit_pool(id, |p| p.outline.truncate(2)).is_err());
        assert!(m.pool(id).unwrap().deep > 1.0);
    }
}
