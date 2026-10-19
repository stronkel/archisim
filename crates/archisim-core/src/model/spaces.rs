//! Rooms ("locales", ArchiCAD's zones, Revit's rooms). IFC: `IfcSpace`.
//!
//! A space is a closed outline on a level (usually found by clicking inside
//! a room: the interior faces of its walls) with a number, a name and a
//! use, and a height for its volume. Its area is the usable area
//! ("superficie útil"); the built area ("superficie construida") is worked
//! out in plan, to the axes of the walls around it.

use super::*;

/// A room. IFC: `IfcSpace` (`Name` = number, `LongName` = name).
#[derive(Debug, Clone, PartialEq)]
pub struct Space {
    pub id: SpaceId,
    pub guid: GlobalId,
    pub level: LevelId,
    /// Room number ("1.03"), unique on its level or empty.
    pub number: String,
    pub name: String,
    /// Use ("Dormitorio", "Baño", "Cocina"…): `IfcSpace.ObjectType`.
    pub usage: String,
    /// Outline in plan, counter-clockwise.
    pub outline: Vec<Point2>,
    /// Height of the room above its level (floor to ceiling).
    pub height: f64,
    /// Where its label is drawn; `None`: in the middle of the outline.
    pub label: Option<Point2>,
}

impl Space {
    /// Usable floor area of the outline.
    pub fn area(&self) -> f64 {
        signed_area(&self.outline).abs()
    }

    pub fn perimeter(&self) -> f64 {
        crate::math::perimeter(&self.outline)
    }

    /// Where the label goes: the point given, or the visual centre of the
    /// outline (its centroid, if it lies inside; otherwise the middle of
    /// its widest horizontal run through the centroid).
    pub fn label_point(&self) -> Point2 {
        if let Some(p) = self.label {
            return p;
        }
        let c = centroid(&self.outline);
        if polygon_contains(&self.outline, c) {
            return c;
        }
        // The widest piece of the horizontal line through the centroid.
        let n = self.outline.len();
        let mut xs: Vec<f64> = (0..n)
            .filter_map(|i| {
                let (a, b) = (self.outline[i], self.outline[(i + 1) % n]);
                ((a.y > c.y) != (b.y > c.y)).then(|| a.x + (c.y - a.y) / (b.y - a.y) * (b.x - a.x))
            })
            .collect();
        xs.sort_by(f64::total_cmp);
        xs.chunks(2)
            .filter(|p| p.len() == 2)
            .max_by(|a, b| (a[1] - a[0]).total_cmp(&(b[1] - b[0])))
            .map_or(c, |p| Point2::new((p[0] + p[1]) / 2.0, c.y))
    }
}

/// Centroid of a polygon's area.
pub fn centroid(poly: &[Point2]) -> Point2 {
    let n = poly.len();
    let (mut cx, mut cy, mut a2) = (0.0, 0.0, 0.0);
    for i in 0..n {
        let (p, q) = (poly[i], poly[(i + 1) % n]);
        let cross = p.x * q.y - q.x * p.y;
        a2 += cross;
        cx += (p.x + q.x) * cross;
        cy += (p.y + q.y) * cross;
    }
    if a2.abs() < 1e-12 {
        let s = poly.iter().fold(Point2::new(0.0, 0.0), |acc, p| acc + *p);
        return s * (1.0 / n.max(1) as f64);
    }
    Point2::new(cx / (3.0 * a2), cy / (3.0 * a2))
}

/// Data needed to create a space.
#[derive(Debug, Clone, PartialEq)]
pub struct NewSpace {
    pub level: LevelId,
    pub number: String,
    pub name: String,
    pub usage: String,
    pub outline: Vec<Point2>,
    pub height: f64,
}

impl Model {
    fn validate_space(&self, s: &NewSpace, me: Option<SpaceId>) -> Result<(), ModelError> {
        if !self.levels.contains_key(&s.level) {
            return Err(ModelError::UnknownLevel(s.level));
        }
        finite(s.height, "space height")?;
        if s.height <= EPSILON {
            return Err(ModelError::NonPositive("space height"));
        }
        let number = s.number.trim();
        if !number.is_empty()
            && self
                .spaces
                .values()
                .any(|o| Some(o.id) != me && o.level == s.level && o.number.trim() == number)
        {
            return Err(ModelError::DuplicateNumber(number.to_string()));
        }
        Ok(())
    }

    /// The next free room number on a level: one more than the highest
    /// number ending in digits there ("1.03" → "1.04"), or "1".
    pub fn next_space_number(&self, level: LevelId) -> String {
        let mut best: Option<(String, u64, usize)> = None;
        for s in self.spaces.values().filter(|s| s.level == level) {
            let t = s.number.trim();
            let digits = t.chars().rev().take_while(|c| c.is_ascii_digit()).count();
            if digits == 0 {
                continue;
            }
            let (prefix, tail) = t.split_at(t.len() - digits);
            let Ok(n) = tail.parse::<u64>() else {
                continue;
            };
            if best.as_ref().is_none_or(|b| n > b.1) {
                best = Some((prefix.to_string(), n, digits));
            }
        }
        match best {
            Some((prefix, n, width)) => format!("{prefix}{:0width$}", n + 1),
            None => "1".to_string(),
        }
    }

    pub fn add_space(&mut self, spec: NewSpace) -> Result<SpaceId, ModelError> {
        self.validate_space(&spec, None)?;
        let outline = Self::normalized_outline(spec.outline)?;
        let id = SpaceId(self.take_id());
        self.spaces.insert(
            id,
            Space {
                id,
                guid: GlobalId::new(),
                level: spec.level,
                number: spec.number.trim().to_string(),
                name: spec.name,
                usage: spec.usage,
                outline,
                height: spec.height,
                label: None,
            },
        );
        Ok(id)
    }

    /// Edits a space; nothing changes if the result is not valid.
    pub fn edit_space(
        &mut self,
        id: SpaceId,
        edit: impl FnOnce(&mut Space),
    ) -> Result<(), ModelError> {
        let mut s = self
            .spaces
            .get(&id)
            .cloned()
            .ok_or(ModelError::UnknownSpace(id))?;
        edit(&mut s);
        self.validate_space(
            &NewSpace {
                level: s.level,
                number: s.number.clone(),
                name: s.name.clone(),
                usage: s.usage.clone(),
                outline: s.outline.clone(),
                height: s.height,
            },
            Some(id),
        )?;
        s.outline = Self::normalized_outline(std::mem::take(&mut s.outline))?;
        s.number = s.number.trim().to_string();
        if let Some(p) = s.label {
            finite_point(p, "space label")?;
        }
        self.spaces.insert(id, s);
        Ok(())
    }

    pub fn remove_space(&mut self, id: SpaceId) -> Result<Space, ModelError> {
        self.spaces.remove(&id).ok_or(ModelError::UnknownSpace(id))
    }

    pub fn space(&self, id: SpaceId) -> Option<&Space> {
        self.spaces.get(&id)
    }

    pub fn spaces(&self) -> impl Iterator<Item = &Space> {
        self.spaces.values()
    }

    pub fn spaces_on_level(&self, level: LevelId) -> impl Iterator<Item = &Space> {
        self.spaces.values().filter(move |s| s.level == level)
    }

    pub(super) fn restore_spaces(
        &mut self,
        spaces: Vec<Space>,
        claim: &mut structure::Claim<'_>,
    ) -> Result<(), RestoreError> {
        for mut s in spaces {
            let name = format!("space {} ({})", s.id.0, s.guid);
            let spec = NewSpace {
                level: s.level,
                number: s.number.clone(),
                name: s.name.clone(),
                usage: s.usage.clone(),
                outline: s.outline.clone(),
                height: s.height,
            };
            claim(s.id.0, Some(&s.guid))
                .and_then(|_| self.validate_space(&spec, Some(s.id)))
                .and_then(|_| {
                    s.outline = Self::normalized_outline(std::mem::take(&mut s.outline))?;
                    if let Some(p) = s.label {
                        finite_point(p, "space label")?;
                    }
                    Ok(())
                })
                .map_err(|error| RestoreError {
                    element: name,
                    error,
                })?;
            self.spaces.insert(s.id, s);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn square(x: f64, y: f64, w: f64) -> Vec<Point2> {
        vec![
            Point2::new(x, y),
            Point2::new(x + w, y),
            Point2::new(x + w, y + w),
            Point2::new(x, y + w),
        ]
    }

    #[test]
    fn numbers_areas_and_labels() {
        let mut m = Model::new("t");
        let l = m.add_level("PB", 0.0);
        let spec = |number: &str, outline| NewSpace {
            level: l,
            number: number.into(),
            name: "Dormitorio".into(),
            usage: "Dormitorio".into(),
            outline,
            height: 2.5,
        };
        assert_eq!(m.next_space_number(l), "1");
        let a = m.add_space(spec("1.08", square(0.0, 0.0, 3.0))).unwrap();
        assert_eq!(m.next_space_number(l), "1.09");
        assert_eq!(
            m.add_space(spec("1.08", square(5.0, 0.0, 2.0))),
            Err(ModelError::DuplicateNumber("1.08".into()))
        );
        let s = m.space(a).unwrap();
        assert!((s.area() - 9.0).abs() < 1e-12);
        assert!((s.label_point().x - 1.5).abs() < 1e-12);
        // An L: its centroid falls outside, the label does not.
        let l_shape = vec![
            Point2::new(0.0, 0.0),
            Point2::new(4.0, 0.0),
            Point2::new(4.0, 0.5),
            Point2::new(0.5, 0.5),
            Point2::new(0.5, 4.0),
            Point2::new(0.0, 4.0),
        ];
        let b = m.add_space(spec("", l_shape)).unwrap();
        let s = m.space(b).unwrap();
        assert!(polygon_contains(&s.outline, s.label_point()));
        let back = Model::from_parts(m.to_parts()).unwrap();
        assert_eq!(back, m);
    }
}
