//! Plantings: plants (or any object: street lamps, bollards) spread along
//! a line or over an area, as many as asked, more or less regular, with
//! their size and shape more or less varied. The planting keeps its line
//! and its rules; its plants are ordinary objects that it makes again
//! whenever the planting changes, each with a `GlobalId` of its own that
//! stays the same as long as its place in the planting does.

use super::*;
use archisim_flora as flora;

/// How the plants of a planting are spread.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Arrangement {
    /// Along its line, evenly (an avenue, a hedge, a row of palms).
    Line,
    /// Over the area its closed line encloses (a grove, a planted bed).
    Area,
}

impl Arrangement {
    pub fn name(self) -> &'static str {
        match self {
            Arrangement::Line => "A lo largo de la línea",
            Arrangement::Area => "Dentro del contorno",
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Planting {
    pub id: PlantingId,
    pub guid: GlobalId,
    pub name: String,
    pub level: LevelId,
    /// The line the plants follow, or the outline of their area.
    pub path: Vec<Point2>,
    pub closed: bool,
    /// The line drawn as a curve through its points (spline).
    pub smooth: bool,
    pub arrangement: Arrangement,
    /// How many plants.
    pub count: u32,
    /// The types planted, mixed at random.
    pub species: Vec<ObjectTypeId>,
    /// How much their size varies, from 0 (all equal) to 1 (±40 %).
    pub size_variation: f64,
    /// How much their shape varies from their species, from 0 to 1.
    pub shape_variation: f64,
    /// How irregular their places are, from 0 (in line, even) to 1.
    pub spread: f64,
    /// The planting's own chance: another one, other plants.
    pub seed: u64,
    /// The objects it has made.
    pub members: Vec<ObjectId>,
    pub base_offset: f64,
}

/// Data needed to make a planting.
#[derive(Debug, Clone, PartialEq)]
pub struct NewPlanting {
    pub name: String,
    pub level: LevelId,
    pub path: Vec<Point2>,
    pub closed: bool,
    pub smooth: bool,
    pub arrangement: Arrangement,
    pub count: u32,
    pub species: Vec<ObjectTypeId>,
    pub size_variation: f64,
    pub shape_variation: f64,
    pub spread: f64,
    pub seed: u64,
    pub base_offset: f64,
}

impl Planting {
    fn spec(&self) -> NewPlanting {
        NewPlanting {
            name: self.name.clone(),
            level: self.level,
            path: self.path.clone(),
            closed: self.closed,
            smooth: self.smooth,
            arrangement: self.arrangement,
            count: self.count,
            species: self.species.clone(),
            size_variation: self.size_variation,
            shape_variation: self.shape_variation,
            spread: self.spread,
            seed: self.seed,
            base_offset: self.base_offset,
        }
    }
}

/// Where a plant of a planting goes and what it is.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PlantSpot {
    pub position: Point2,
    pub rotation: f64,
    pub object_type: ObjectTypeId,
    /// Its own seed (1 or more).
    pub seed: u64,
    /// Its size against its type's.
    pub size: f64,
}

/// The line through the points: as drawn, or as a curve through them
/// (Catmull-Rom), closed or not.
pub fn planting_line(path: &[Point2], closed: bool, smooth: bool) -> Vec<Point2> {
    let n = path.len();
    if !smooth || n < 3 {
        let mut v = path.to_vec();
        if closed && n >= 3 {
            v.push(path[0]);
        }
        return v;
    }
    let at = |i: isize| -> Point2 {
        if closed {
            path[i.rem_euclid(n as isize) as usize]
        } else {
            path[i.clamp(0, n as isize - 1) as usize]
        }
    };
    let segs = if closed { n } else { n - 1 };
    let mut out = Vec::with_capacity(segs * 12 + 1);
    for i in 0..segs as isize {
        let (p0, p1, p2, p3) = (at(i - 1), at(i), at(i + 1), at(i + 2));
        for k in 0..12 {
            let t = k as f64 / 12.0;
            let t2 = t * t;
            let t3 = t2 * t;
            let f = |a: f64, b: f64, c: f64, d: f64| {
                0.5 * (2.0 * b
                    + (-a + c) * t
                    + (2.0 * a - 5.0 * b + 4.0 * c - d) * t2
                    + (-a + 3.0 * b - 3.0 * c + d) * t3)
            };
            out.push(Point2::new(
                f(p0.x, p1.x, p2.x, p3.x),
                f(p0.y, p1.y, p2.y, p3.y),
            ));
        }
    }
    out.push(if closed { path[0] } else { path[n - 1] });
    out
}

fn point_inside(poly: &[Point2], p: Point2) -> bool {
    let mut c = false;
    let n = poly.len();
    let mut j = n - 1;
    for i in 0..n {
        let (a, b) = (poly[i], poly[j]);
        if (a.y > p.y) != (b.y > p.y) && p.x < (b.x - a.x) * (p.y - a.y) / (b.y - a.y) + a.x {
            c = !c;
        }
        j = i;
    }
    c
}

/// A small repeatable chance (the same as the plants').
struct Chance(u64);

impl Chance {
    fn f(&mut self) -> f64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        ((z ^ (z >> 31)) >> 11) as f64 / (1u64 << 53) as f64
    }
    fn signed(&mut self) -> f64 {
        self.f() * 2.0 - 1.0
    }
}

/// Where the plants of a planting go (also to see them before making it).
pub fn planting_spots(p: &NewPlanting) -> Vec<PlantSpot> {
    let line = planting_line(&p.path, p.closed, p.smooth);
    if line.len() < 2 || p.species.is_empty() || p.count == 0 {
        return Vec::new();
    }
    let mut rng = Chance(flora::seed_of(p.seed, 0x5eed));
    let n = p.count as usize;
    let spread = p.spread.clamp(0.0, 1.0);
    let mut spots: Vec<Point2> = Vec::with_capacity(n);
    match p.arrangement {
        Arrangement::Line => {
            let lengths: Vec<f64> = line.windows(2).map(|w| w[0].distance(w[1])).collect();
            let total: f64 = lengths.iter().sum();
            if total <= EPSILON {
                return Vec::new();
            }
            let closed = p.closed && p.path.len() >= 3;
            let step = if closed {
                total / n as f64
            } else if n > 1 {
                total / (n - 1) as f64
            } else {
                total
            };
            for i in 0..n {
                let mut s = if closed {
                    step * i as f64
                } else if n == 1 {
                    total / 2.0
                } else {
                    step * i as f64
                };
                s += rng.signed() * spread * step * 0.35;
                let s = if closed {
                    s.rem_euclid(total)
                } else {
                    s.clamp(0.0, total)
                };
                // The point at `s` and the direction there.
                let mut left = s;
                let mut at = (line[0], Point2::new(1.0, 0.0));
                for (k, l) in lengths.iter().enumerate() {
                    if left <= *l || k == lengths.len() - 1 {
                        let u = if *l > EPSILON {
                            (left / l).min(1.0)
                        } else {
                            0.0
                        };
                        let a = line[k];
                        let b = line[k + 1];
                        let d = (b - a) * (1.0 / l.max(EPSILON));
                        at = (a + (b - a) * u, d);
                        break;
                    }
                    left -= l;
                }
                let (q, d) = at;
                let side = Point2::new(-d.y, d.x);
                spots.push(q + side * (rng.signed() * spread * step * 0.45));
            }
        }
        Arrangement::Area => {
            let poly: Vec<Point2> = line[..line.len() - 1].to_vec();
            if poly.len() < 3 || signed_area(&poly).abs() <= EPSILON {
                return Vec::new();
            }
            let (mut lo, mut hi) = (
                Point2::new(f64::MAX, f64::MAX),
                Point2::new(f64::MIN, f64::MIN),
            );
            for q in &poly {
                lo = Point2::new(lo.x.min(q.x), lo.y.min(q.y));
                hi = Point2::new(hi.x.max(q.x), hi.y.max(q.y));
            }
            // Best of several tries (Mitchell): even when regular, at random
            // when irregular.
            let tries = (1.0 + (1.0 - spread) * 16.0).round() as usize;
            for _ in 0..n {
                let mut best: Option<(f64, Point2)> = None;
                for _ in 0..tries {
                    let mut q = None;
                    for _ in 0..200 {
                        let c = Point2::new(
                            lo.x + (hi.x - lo.x) * rng.f(),
                            lo.y + (hi.y - lo.y) * rng.f(),
                        );
                        if point_inside(&poly, c) {
                            q = Some(c);
                            break;
                        }
                    }
                    let Some(q) = q else { continue };
                    let d = spots.iter().map(|s| s.distance(q)).fold(f64::MAX, f64::min);
                    if best.is_none_or(|(bd, _)| d > bd) {
                        best = Some((d, q));
                    }
                }
                if let Some((_, q)) = best {
                    spots.push(q);
                }
            }
        }
    }
    spots
        .into_iter()
        .enumerate()
        .map(|(i, position)| {
            let pick = if p.species.len() == 1 {
                0
            } else {
                ((rng.f() * p.species.len() as f64) as usize).min(p.species.len() - 1)
            };
            PlantSpot {
                position,
                rotation: rng.f() * std::f64::consts::TAU,
                object_type: p.species[pick],
                seed: flora::seed_of(p.seed, i as u64) % 999_983 + 1,
                size: 1.0 + p.size_variation.clamp(0.0, 1.0) * 0.4 * rng.signed(),
            }
        })
        .collect()
}

impl Model {
    fn validate_planting(&self, p: &NewPlanting) -> Result<(), ModelError> {
        if !self.levels.contains_key(&p.level) {
            return Err(ModelError::UnknownLevel(p.level));
        }
        for q in &p.path {
            finite_point(*q, "planting line")?;
        }
        if p.path.len() < 2 || p.path.windows(2).all(|w| w[0].approx_eq(w[1])) {
            return Err(ModelError::InvalidShape(
                "a planting needs a line of two different points or more",
            ));
        }
        if p.arrangement == Arrangement::Area
            && (!p.closed || p.path.len() < 3 || signed_area(&p.path).abs() <= EPSILON)
        {
            return Err(ModelError::InvalidShape(
                "to plant inside an outline, the outline must be closed",
            ));
        }
        if p.count == 0 || p.count > 5000 {
            return Err(ModelError::InvalidShape(
                "a planting has from 1 to 5000 plants",
            ));
        }
        if p.species.is_empty() {
            return Err(ModelError::InvalidShape(
                "a planting needs something to plant",
            ));
        }
        for t in &p.species {
            if !self.object_types.contains_key(t) {
                return Err(ModelError::UnknownObjectType(*t));
            }
        }
        for (v, what) in [
            (p.size_variation, "size variation"),
            (p.shape_variation, "shape variation"),
            (p.spread, "spread"),
        ] {
            finite(v, what)?;
            if !(0.0..=1.0).contains(&v) {
                return Err(ModelError::InvalidShape(
                    "the variations and the spread go from 0 to 1",
                ));
            }
        }
        finite(p.base_offset, "planting base offset")?;
        Ok(())
    }

    /// Makes a planting and its plants.
    pub fn add_planting(&mut self, spec: NewPlanting) -> Result<PlantingId, ModelError> {
        self.validate_planting(&spec)?;
        let id = PlantingId(self.take_id());
        self.plantings.insert(
            id,
            Planting {
                id,
                guid: GlobalId::new(),
                name: spec.name,
                level: spec.level,
                path: spec.path,
                closed: spec.closed,
                smooth: spec.smooth,
                arrangement: spec.arrangement,
                count: spec.count,
                species: spec.species,
                size_variation: spec.size_variation,
                shape_variation: spec.shape_variation,
                spread: spec.spread,
                seed: spec.seed,
                members: Vec::new(),
                base_offset: spec.base_offset,
            },
        );
        if let Err(e) = self.replant(id) {
            self.plantings.remove(&id);
            return Err(e);
        }
        Ok(id)
    }

    /// Changes a planting; its plants are made again.
    pub fn edit_planting(
        &mut self,
        id: PlantingId,
        edit: impl FnOnce(&mut Planting),
    ) -> Result<(), ModelError> {
        let old = self
            .plantings
            .get(&id)
            .cloned()
            .ok_or(ModelError::UnknownElement(id.0))?;
        let mut p = old.clone();
        edit(&mut p);
        p.id = id;
        p.guid = old.guid.clone();
        p.members = old.members.clone();
        self.validate_planting(&p.spec())?;
        self.plantings.insert(id, p);
        if let Err(e) = self.replant(id) {
            self.plantings.insert(id, old);
            self.replant(id).ok();
            return Err(e);
        }
        Ok(())
    }

    /// Takes a planting out, with its plants.
    pub fn remove_planting(&mut self, id: PlantingId) -> Result<Planting, ModelError> {
        let p = self
            .plantings
            .remove(&id)
            .ok_or(ModelError::UnknownElement(id.0))?;
        for m in &p.members {
            self.objects.remove(m);
        }
        Ok(p)
    }

    pub fn planting(&self, id: PlantingId) -> Option<&Planting> {
        self.plantings.get(&id)
    }

    pub fn plantings(&self) -> impl Iterator<Item = &Planting> {
        self.plantings.values()
    }

    pub fn plantings_on_level(&self, level: LevelId) -> impl Iterator<Item = &Planting> {
        self.plantings.values().filter(move |p| p.level == level)
    }

    /// The planting an object belongs to, if it was made by one.
    pub fn planting_of(&self, object: ObjectId) -> Option<PlantingId> {
        self.plantings
            .values()
            .find(|p| p.members.contains(&object))
            .map(|p| p.id)
    }

    /// A name for a new planting: «Plantación 1».
    pub fn next_planting_name(&self) -> String {
        (1..)
            .map(|n: usize| format!("Plantación {n}"))
            .find(|n| !self.plantings.values().any(|p| p.name == *n))
            .unwrap_or_default()
    }

    /// Makes the plants of a planting again from its rules.
    fn replant(&mut self, id: PlantingId) -> Result<(), ModelError> {
        let p = self
            .plantings
            .get(&id)
            .cloned()
            .ok_or(ModelError::UnknownElement(id.0))?;
        for m in &p.members {
            self.objects.remove(m);
        }
        let mut members = Vec::new();
        for (i, spot) in planting_spots(&p.spec()).into_iter().enumerate() {
            let oid = ObjectId(self.take_id());
            let mut values = Values::new();
            if self.type_grows(spot.object_type) {
                if let Some(tv) = self.type_values_of(spot.object_type) {
                    let h = tv.get("height").and_then(|v| v.as_number()).unwrap_or(1.0);
                    let w = tv.get("crown").and_then(|v| v.as_number()).unwrap_or(1.0);
                    values.insert("plant_seed".into(), Value::Number(spot.seed as f64));
                    values.insert("variation".into(), Value::Number(p.shape_variation));
                    if (spot.size - 1.0).abs() > 1e-9 {
                        values.insert("height_own".into(), Value::Number(round_mm(h * spot.size)));
                        values.insert("crown_own".into(), Value::Number(round_mm(w * spot.size)));
                    }
                }
            }
            let o = Object {
                id: oid,
                guid: combined_guid(&p.guid, &GlobalId::from_u128(i as u128 + 1)),
                level: p.level,
                object_type: spot.object_type,
                position: spot.position,
                rotation: spot.rotation,
                base_offset: p.base_offset,
                values,
            };
            self.objects.insert(oid, o);
            members.push(oid);
        }
        if let Some(pl) = self.plantings.get_mut(&id) {
            pl.members = members;
        }
        Ok(())
    }

    /// A type's values over its family's defaults.
    pub(crate) fn type_values_of(&self, t: ObjectTypeId) -> Option<Values> {
        let ty = self.object_types.get(&t)?;
        let mut v = self.family_defaults(ty.family, ParamScope::Type).ok()?;
        v.extend(ty.values.clone());
        Some(v)
    }

    pub(super) fn restore_plantings(
        &mut self,
        plantings: Vec<Planting>,
        claim: &mut structure::Claim<'_>,
    ) -> Result<(), RestoreError> {
        for mut p in plantings {
            claim(p.id.0, Some(&p.guid))
                .and_then(|_| self.validate_planting(&p.spec()))
                .map_err(|error| RestoreError {
                    element: format!("planting {} «{}»", p.id.0, p.name),
                    error,
                })?;
            // Plants taken out by hand stay out.
            p.members.retain(|m| self.objects.contains_key(m));
            self.plantings.insert(p.id, p);
        }
        Ok(())
    }
}

fn round_mm(v: f64) -> f64 {
    (v * 1000.0).round() / 1000.0
}
