//! Changes along a street and replacing in bulk (N4, `docs/TERRITORIO.md`
//! §2.7): a stretch of a street with other paving, kerbs, sidewalk or
//! planting soil than the rest of it (as Network Skins and Repaint do in
//! Cities: Skylines), and «Sustituir todos»: every object of a type (a tree,
//! a lamp, a bench) or every street material changed for another, in the
//! whole project, on a level, inside an area or along a stretch of a street
//! (BOB).

use super::point_in_polygon;
use super::{axis_point, LevelId, MaterialId, Model, ModelError, ObjectId, ObjectTypeId, RoadId};
use crate::{Point2, EPSILON};

/// A stretch of a street with its own materials: those not given are the
/// street's.
#[derive(Debug, Clone, PartialEq)]
pub struct RoadSkin {
    /// From where to where along its axis.
    pub from: f64,
    pub to: f64,
    /// Of the carriageway and parking.
    pub pavement: Option<MaterialId>,
    pub kerb: Option<MaterialId>,
    pub sidewalk: Option<MaterialId>,
    /// Of the planting strips, tram tracks and ditches.
    pub planting: Option<MaterialId>,
}

impl RoadSkin {
    /// A stretch with nothing changed yet.
    pub fn new(from: f64, to: f64) -> Self {
        RoadSkin {
            from,
            to,
            pavement: None,
            kerb: None,
            sidewalk: None,
            planting: None,
        }
    }

    /// Its materials, as `(what, material)`.
    pub fn materials(&self) -> [Option<MaterialId>; 4] {
        [self.pavement, self.kerb, self.sidewalk, self.planting]
    }

    fn slots(&mut self) -> [&mut Option<MaterialId>; 4] {
        [
            &mut self.pavement,
            &mut self.kerb,
            &mut self.sidewalk,
            &mut self.planting,
        ]
    }
}

/// Where a replacement reaches.
#[derive(Debug, Clone, PartialEq)]
pub enum ReplaceScope {
    /// The whole project.
    All,
    /// A level.
    Level(LevelId),
    /// Inside an outline on a level.
    Area {
        level: LevelId,
        outline: Vec<Point2>,
    },
    /// Along a stretch of a street (from, to along its axis), and what
    /// stands on it.
    Road { road: RoadId, from: f64, to: f64 },
}

/// How far beyond a street's edges an object still stands «on» it (a lamp
/// at the back of the sidewalk), metres.
pub const ON_ROAD_MARGIN: f64 = 1.0;

impl super::Road {
    /// The stretch with its own materials at `s` along the axis (the last
    /// given, if they overlap).
    pub fn skin_at(&self, s: f64) -> Option<&RoadSkin> {
        self.skins
            .iter()
            .rev()
            .find(|k| k.from - EPSILON <= s && s <= k.to + EPSILON)
    }

    /// Where its pieces are cut for its stretches.
    pub fn skin_cuts(&self) -> Vec<f64> {
        let len = self.length();
        self.skins
            .iter()
            .flat_map(|k| [k.from.clamp(0.0, len), k.to.clamp(0.0, len)])
            .collect()
    }

    /// Its materials at `s`: of the carriageway, kerbs, sidewalks and
    /// planting (none: the sidewalk's unless its section says).
    pub fn materials_at(&self, s: f64) -> (MaterialId, MaterialId, MaterialId, Option<MaterialId>) {
        let k = self.skin_at(s);
        (
            k.and_then(|k| k.pavement).unwrap_or(self.pavement),
            k.and_then(|k| k.kerb).unwrap_or(self.kerb_material),
            k.and_then(|k| k.sidewalk).unwrap_or(self.sidewalk_material),
            k.and_then(|k| k.planting).or(self.section.planting),
        )
    }

    /// Its stretches as a line of text (for the IFC), the materials named
    /// by `name`: `from;to;pavement;kerb;sidewalk;planting|…`.
    pub fn encode_skins(&self, name: impl Fn(MaterialId) -> String) -> String {
        self.skins
            .iter()
            .map(|k| {
                let m = |x: Option<MaterialId>| x.map(&name).unwrap_or_default();
                format!(
                    "{};{};{};{};{};{}",
                    k.from,
                    k.to,
                    m(k.pavement),
                    m(k.kerb),
                    m(k.sidewalk),
                    m(k.planting)
                )
            })
            .collect::<Vec<_>>()
            .join("|")
    }

    /// Reads [`Road::encode_skins`], the materials found by `material` (an
    /// unknown one is left as the street's).
    pub fn decode_skins(
        text: &str,
        material: impl Fn(&str) -> Option<MaterialId>,
    ) -> Option<Vec<RoadSkin>> {
        let mut out = Vec::new();
        for t in text.split('|').filter(|t| !t.is_empty()) {
            let f: Vec<&str> = t.split(';').collect();
            if f.len() < 6 {
                return None;
            }
            let m = |x: &str| if x.is_empty() { None } else { material(x) };
            out.push(RoadSkin {
                from: f[0].parse().ok()?,
                to: f[1].parse().ok()?,
                pavement: m(f[2]),
                kerb: m(f[3]),
                sidewalk: m(f[4]),
                planting: m(f[5]),
            });
        }
        Some(out)
    }
}

/// The stretches of an axis inside an outline: (from, to) along it,
/// sampled every metre.
pub fn stretches_inside(axis: &[Point2], outline: &[Point2]) -> Vec<(f64, f64)> {
    let len: f64 = axis.windows(2).map(|w| w[0].distance(w[1])).sum();
    if len < EPSILON || outline.len() < 3 {
        return Vec::new();
    }
    let n = (len / 1.0).ceil().max(1.0) as usize;
    let mut out = Vec::new();
    let mut start: Option<f64> = None;
    for i in 0..=n {
        let s = len * i as f64 / n as f64;
        let (p, _) = axis_point(axis, s);
        let inside = point_in_polygon(p, outline);
        match (inside, start) {
            (true, None) => start = Some(s),
            (false, Some(a)) => {
                out.push((a, s));
                start = None;
            }
            _ => {}
        }
    }
    if let Some(a) = start {
        out.push((a, len));
    }
    out.retain(|(a, b)| b - a > 0.5);
    out
}

impl Model {
    /// Gives a street its stretches with their own materials.
    pub fn set_road_skins(&mut self, id: RoadId, skins: Vec<RoadSkin>) -> Result<(), ModelError> {
        let len = self
            .road(id)
            .ok_or(ModelError::UnknownElement(id.0))?
            .length();
        for k in &skins {
            if !k.from.is_finite() || !k.to.is_finite() {
                return Err(ModelError::NotFinite("tramo"));
            }
            if k.to - k.from < EPSILON || k.from < -EPSILON || k.to > len + 0.01 {
                return Err(ModelError::NonPositive("tramo"));
            }
            for m in k.materials().into_iter().flatten() {
                if self.material(m).is_none() {
                    return Err(ModelError::UnknownMaterial(m));
                }
            }
        }
        self.edit_road(id, |r| r.skins = skins)
    }

    /// The objects a replacement reaches.
    pub fn objects_in(&self, scope: &ReplaceScope) -> Vec<ObjectId> {
        let mut out: Vec<ObjectId> = self
            .objects()
            .filter(|o| match scope {
                ReplaceScope::All => true,
                ReplaceScope::Level(l) => o.level == *l,
                ReplaceScope::Area { level, outline } => {
                    o.level == *level && point_in_polygon(o.position, outline)
                }
                ReplaceScope::Road { road, from, to } => {
                    let Some(r) = self.road(*road) else {
                        return false;
                    };
                    if o.level != r.level {
                        return false;
                    }
                    let s = r.station_of(o.position);
                    if s < from - EPSILON || s > to + EPSILON {
                        return false;
                    }
                    let (q, _) = axis_point(&r.axis, s);
                    let (left, right) = r.extent_at(s);
                    q.distance(o.position) <= left.max(right) + ON_ROAD_MARGIN
                }
            })
            .map(|o| o.id)
            .collect();
        out.sort();
        out
    }

    /// Every object of type `from` in `scope` changed to type `to` (its
    /// place, turn and own values kept). How many.
    pub fn replace_object_type(
        &mut self,
        from: ObjectTypeId,
        to: ObjectTypeId,
        scope: &ReplaceScope,
    ) -> Result<usize, ModelError> {
        if self.object_type(to).is_none() {
            return Err(ModelError::UnknownObjectType(to));
        }
        let ids: Vec<ObjectId> = self
            .objects_in(scope)
            .into_iter()
            .filter(|id| self.object(*id).is_some_and(|o| o.object_type == from))
            .collect();
        for id in &ids {
            self.edit_object(*id, |o| o.object_type = to)?;
        }
        Ok(ids.len())
    }

    /// Every use of material `from` by the streets in `scope` changed to
    /// `to`: whole streets on the project or a level; inside an area or
    /// along a stretch, a stretch of its own of each street. How many
    /// streets changed.
    pub fn replace_road_material(
        &mut self,
        from: MaterialId,
        to: MaterialId,
        scope: &ReplaceScope,
    ) -> Result<usize, ModelError> {
        if self.material(to).is_none() {
            return Err(ModelError::UnknownMaterial(to));
        }
        let swap = |m: &mut MaterialId| {
            if *m == from {
                *m = to;
                true
            } else {
                false
            }
        };
        let mut changed = 0;
        match scope {
            ReplaceScope::All | ReplaceScope::Level(_) => {
                let ids: Vec<RoadId> = self
                    .roads()
                    .filter(|r| match scope {
                        ReplaceScope::Level(l) => r.level == *l,
                        _ => true,
                    })
                    .map(|r| r.id)
                    .collect();
                for id in ids {
                    let mut any = false;
                    self.edit_road(id, |r| {
                        any |= swap(&mut r.pavement);
                        any |= swap(&mut r.kerb_material);
                        any |= swap(&mut r.sidewalk_material);
                        for m in [&mut r.section.planting, &mut r.section.bike]
                            .into_iter()
                            .flatten()
                        {
                            any |= swap(m);
                        }
                        for k in &mut r.skins {
                            for m in k.slots().into_iter().flatten() {
                                any |= swap(m);
                            }
                        }
                    })?;
                    changed += usize::from(any);
                }
            }
            ReplaceScope::Area { level, outline } => {
                let runs: Vec<(RoadId, f64, f64)> = self
                    .roads()
                    .filter(|r| r.level == *level)
                    .flat_map(|r| {
                        stretches_inside(&r.axis, outline)
                            .into_iter()
                            .map(move |(a, b)| (r.id, a, b))
                    })
                    .collect();
                let mut roads = Vec::new();
                for (road, a, b) in runs {
                    if self.skin_stretch(road, a, b, from, to)? && !roads.contains(&road) {
                        roads.push(road);
                    }
                }
                changed = roads.len();
            }
            ReplaceScope::Road {
                road,
                from: a,
                to: b,
            } => {
                changed = usize::from(self.skin_stretch(*road, *a, *b, from, to)?);
            }
        }
        Ok(changed)
    }

    /// A stretch of its own on a street over `[a, b]` where each material
    /// equal to `from` there is `to`. Whether it changed anything.
    fn skin_stretch(
        &mut self,
        road: RoadId,
        a: f64,
        b: f64,
        from: MaterialId,
        to: MaterialId,
    ) -> Result<bool, ModelError> {
        let r = self.road(road).ok_or(ModelError::UnknownElement(road.0))?;
        let len = r.length();
        let (a, b) = (a.clamp(0.0, len), b.clamp(0.0, len));
        if b - a < EPSILON {
            return Ok(false);
        }
        let (pave, kerb, walk, plant) = r.materials_at((a + b) / 2.0);
        let mut skin = RoadSkin::new(a, b);
        if let Some(k) = r.skin_at((a + b) / 2.0) {
            skin.pavement = k.pavement;
            skin.kerb = k.kerb;
            skin.sidewalk = k.sidewalk;
            skin.planting = k.planting;
        }
        let mut any = false;
        for (now, slot) in [
            (Some(pave), &mut skin.pavement),
            (Some(kerb), &mut skin.kerb),
            (Some(walk), &mut skin.sidewalk),
            (plant, &mut skin.planting),
        ] {
            if now == Some(from) {
                *slot = Some(to);
                any = true;
            }
        }
        if !any {
            return Ok(false);
        }
        let mut skins = r.skins.clone();
        skins.push(skin);
        self.set_road_skins(road, skins)?;
        Ok(true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{NewObject, NewRoad, RoadSection};

    fn street() -> (Model, LevelId, RoadId, [MaterialId; 3]) {
        let mut m = Model::new("Tramos");
        let level = m.add_level("Planta baja", 0.0);
        let mat = |m: &mut Model, n: &str| {
            m.add_material(n, "stone", [120, 120, 120], crate::CutPattern::Solid)
        };
        let asphalt = mat(&mut m, "Asfalto");
        let granite = mat(&mut m, "Granito");
        let setts = mat(&mut m, "Adoquín");
        let road = m
            .add_road(NewRoad {
                name: "Calle".into(),
                level,
                axis: vec![Point2::new(0.0, 0.0), Point2::new(100.0, 0.0)],
                section: RoadSection::default(),
                top_offset: 0.0,
                profile: Vec::new(),
                changes: Vec::new(),
                pavement: asphalt,
                kerb_material: granite,
                sidewalk_material: granite,
            })
            .unwrap();
        (m, level, road, [asphalt, granite, setts])
    }

    /// A stretch of setts over the middle of the street: its pieces there
    /// take them, and the rest keep the asphalt; the stretch comes back
    /// from its text.
    #[test]
    fn a_stretch_with_its_own_paving() {
        let (mut m, _, road, [asphalt, granite, setts]) = street();
        let mut k = RoadSkin::new(40.0, 60.0);
        k.pavement = Some(setts);
        m.set_road_skins(road, vec![k]).unwrap();
        let r = m.road(road).unwrap();
        let carriageway: Vec<_> = r
            .pieces()
            .into_iter()
            .filter(|p| p.band == crate::RoadBand::Carriageway)
            .collect();
        assert!(carriageway
            .iter()
            .any(|p| p.material == setts && (p.span.0 - 40.0).abs() < 1e-9));
        assert!(carriageway
            .iter()
            .filter(|p| p.span.1 <= 40.0 + 1e-9)
            .all(|p| p.material == asphalt));
        let text = r.encode_skins(|id| m.material(id).unwrap().name.clone());
        let back =
            crate::Road::decode_skins(&text, |n| m.materials().find(|x| x.name == n).map(|x| x.id))
                .unwrap();
        assert_eq!(back, r.skins);
        // Beyond its length, refused.
        let bad = RoadSkin::new(90.0, 130.0);
        assert!(m.set_road_skins(road, vec![bad]).is_err());
        let _ = granite;
    }

    /// «Sustituir todos»: the asphalt of a stretch inside an area becomes
    /// setts, and the lamps on the first half of the street become benches.
    #[test]
    fn replace_in_an_area_and_along_a_stretch() {
        let (mut m, level, road, [asphalt, _, setts]) = street();
        let area = vec![
            Point2::new(20.0, -20.0),
            Point2::new(30.0, -20.0),
            Point2::new(30.0, 20.0),
            Point2::new(20.0, 20.0),
        ];
        let n = m
            .replace_road_material(
                asphalt,
                setts,
                &ReplaceScope::Area {
                    level,
                    outline: area,
                },
            )
            .unwrap();
        assert_eq!(n, 1);
        let r = m.road(road).unwrap();
        assert_eq!(r.materials_at(25.0).0, setts);
        assert_eq!(r.materials_at(50.0).0, asphalt);
        // All of it, on the level: the stretch too.
        m.replace_road_material(setts, asphalt, &ReplaceScope::Level(level))
            .unwrap();
        assert_eq!(m.road(road).unwrap().materials_at(25.0).0, asphalt);
        // Objects along the street.
        let plain = |m: &mut Model, name: &str| {
            let f = m.system_family("Armario empotrado").unwrap();
            let values = m.family_defaults(f, crate::ParamScope::Type).unwrap();
            m.add_object_type(name, f, values).unwrap()
        };
        let lamp = plain(&mut m, "Farola");
        let bench = plain(&mut m, "Banco");
        for x in [10.0, 30.0, 70.0, 90.0] {
            m.add_object(NewObject {
                level,
                object_type: lamp,
                position: Point2::new(x, 6.0),
                rotation: 0.0,
                base_offset: 0.0,
            })
            .unwrap();
        }
        // One far from the street.
        m.add_object(NewObject {
            level,
            object_type: lamp,
            position: Point2::new(20.0, 40.0),
            rotation: 0.0,
            base_offset: 0.0,
        })
        .unwrap();
        let n = m
            .replace_object_type(
                lamp,
                bench,
                &ReplaceScope::Road {
                    road,
                    from: 0.0,
                    to: 50.0,
                },
            )
            .unwrap();
        assert_eq!(n, 2);
        assert_eq!(m.objects().filter(|o| o.object_type == bench).count(), 2);
        let n = m
            .replace_object_type(lamp, bench, &ReplaceScope::All)
            .unwrap();
        assert_eq!(n, 3);
    }
}
