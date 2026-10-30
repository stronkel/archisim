//! Built-in wardrobes (E10, D25): the same family between partitions or
//! in a hole in a wall. Between partitions it takes the room from wall
//! face to wall face ([`Model::fit_between_walls`]); in a wall, the wall
//! gets a hole («Hueco», no joinery) its size where it stands
//! ([`Model::hole_for_object`]).

use super::*;

impl Model {
    /// The faces of the walls of its level that a line from the middle of
    /// an object's depth meets along its width, to each side: the
    /// distances to the left (`-x`) and right (`+x`) of its origin.
    /// `None` on a side with no wall within 8 m.
    pub fn room_between_walls(&self, id: ObjectId) -> Option<(Option<f64>, Option<f64>)> {
        let o = self.objects.get(&id)?;
        let depth = self.object_number(id, "depth").unwrap_or(0.6);
        let (c, s) = (o.rotation.cos(), o.rotation.sin());
        let ux = Point2::new(c, s);
        let uy = Point2::new(-s, c);
        let from = o.position + uy * (depth / 2.0);
        let mut left: Option<f64> = None;
        let mut right: Option<f64> = None;
        for w in self
            .walls
            .values()
            .filter(|w| w.level == o.level && !w.is_curved())
        {
            let Some(t) = self.wall_types.get(&w.wall_type) else {
                continue;
            };
            let (ext, int) = w.face_offsets(t.thickness());
            let Some(dir) = (w.end - w.start).normalized() else {
                continue;
            };
            let n = dir.perp_left();
            for off in [ext, int] {
                let (a, b) = (w.start + n * off, w.end + n * off);
                // The ray from `from` along ±ux against the segment a–b.
                let e = b - a;
                let den = ux.x * e.y - ux.y * e.x;
                if den.abs() < 1e-12 {
                    continue;
                }
                let q = a - from;
                let t_ray = (q.x * e.y - q.y * e.x) / den;
                let t_seg = (q.x * ux.y - q.y * ux.x) / den;
                if !(-1e-9..=1.0 + 1e-9).contains(&t_seg) || t_ray.abs() > 8.0 {
                    continue;
                }
                if t_ray > 1e-6 {
                    right = Some(right.map_or(t_ray, |r: f64| r.min(t_ray)));
                } else if t_ray < -1e-6 {
                    left = Some(left.map_or(-t_ray, |l: f64| l.min(-t_ray)));
                }
            }
        }
        Some((left, right))
    }

    /// A number of an object's values (its type's, its own's).
    fn object_number(&self, id: ObjectId, key: &str) -> Option<f64> {
        let o = self.objects.get(&id)?;
        let t = self.object_types.get(&o.object_type)?;
        let mut values = self.family_defaults(t.family, ParamScope::Type).ok()?;
        values.extend(t.values.clone());
        match values.get(key) {
            Some(Value::Number(n)) => Some(*n),
            _ => None,
        }
    }

    /// Makes a hole in the wall an object stands in (its middle inside
    /// the wall's outline in plan), as wide and high as the object, with
    /// a type of the system family «Hueco» of those sizes. Returns the
    /// opening.
    pub fn hole_for_object(&mut self, id: ObjectId) -> Result<OpeningId, ModelError> {
        let o = self
            .objects
            .get(&id)
            .cloned()
            .ok_or(ModelError::UnknownObject(id))?;
        let width = self.object_number(id, "width").unwrap_or(1.0);
        let height = self.object_number(id, "height").unwrap_or(2.0);
        let depth = self.object_number(id, "depth").unwrap_or(0.6);
        let (c, s) = (o.rotation.cos(), o.rotation.sin());
        let centre = o.position + Point2::new(-s, c) * (depth / 2.0);
        // The wall whose band (between its faces) the object's middle
        // line crosses: nearest by its axis.
        let mut best: Option<(WallId, f64, f64)> = None;
        for w in self
            .walls
            .values()
            .filter(|w| w.level == o.level && !w.is_curved())
        {
            let Some(t) = self.wall_types.get(&w.wall_type) else {
                continue;
            };
            let Some(dir) = (w.end - w.start).normalized() else {
                continue;
            };
            let (ext, int) = w.face_offsets(t.thickness());
            let mid = (ext + int) / 2.0;
            let along = (centre - w.start).dot(dir);
            let across = (centre - w.start).dot(dir.perp_left()) - mid;
            let reach = (ext - int).abs() / 2.0 + depth / 2.0 + 0.05;
            if along < 0.0 || along > w.length() || across.abs() > reach {
                continue;
            }
            if best.is_none_or(|b| across.abs() < b.2) {
                best = Some((w.id, along, across.abs()));
            }
        }
        let (wall, along, _) = best.ok_or(ModelError::NotAnObject(
            "no wall where the wardrobe stands".into(),
        ))?;
        let family = self.system_family("Hueco")?;
        let mut values = self.family_defaults(family, ParamScope::Type)?;
        values.insert("width".into(), Value::Number(width));
        values.insert("height".into(), Value::Number(height));
        let name = format!(
            "Hueco de armario {:.0}×{:.0}",
            width * 100.0,
            height * 100.0
        );
        let opening_type = match self
            .opening_types
            .values()
            .find(|t| t.family == family && t.name == name)
        {
            Some(t) => t.id,
            None => self.add_family_type(NewFamilyType {
                family,
                name,
                mark: None,
                description: "Hueco en el muro para un armario empotrado".into(),
                values,
            })?,
        };
        let sill = o.base_offset;
        self.add_opening(NewOpening {
            opening_type,
            wall,
            along,
            sill,
            opens_to: WallSide::Interior,
            hand: Hand::Left,
            setback: None,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_wardrobe_between_two_partitions_and_in_a_wall() {
        let mut m = Model::new("t");
        let l = m.add_level("PB", 0.0);
        let mat = m.add_material("M", "brick", [200, 100, 80], CutPattern::Solid);
        let wt = m
            .add_wall_type(
                "T",
                WallFunction::Interior,
                vec![Layer {
                    material: mat,
                    thickness: 0.1,
                    function: LayerFunction::Structure,
                }],
            )
            .unwrap();
        let wall = |m: &mut Model, a: Point2, b: Point2| {
            m.add_wall(NewWall {
                wall_type: wt,
                level: l,
                start: a,
                end: b,
                reference: ReferenceLine::Center,
                height: 2.6,
                base_offset: 0.0,
                bulge: 0.0,
                top: WallTop::Unconnected,
            })
            .unwrap()
        };
        // Two partitions 2,1 m apart (faces 2,0 m apart), a back wall.
        wall(&mut m, Point2::new(0.0, 0.0), Point2::new(0.0, 2.0));
        wall(&mut m, Point2::new(2.1, 0.0), Point2::new(2.1, 2.0));
        let back = wall(&mut m, Point2::new(-1.0, -0.05), Point2::new(4.0, -0.05));
        let f = m.system_family("Armario empotrado").unwrap();
        let values = m.family_defaults(f, ParamScope::Type).unwrap();
        let t = m.add_object_type("A", f, values).unwrap();
        let o = m
            .add_object(NewObject {
                object_type: t,
                level: l,
                position: Point2::new(0.8, 0.0),
                rotation: 0.0,
                base_offset: 0.0,
            })
            .unwrap();
        let (left, right) = m.room_between_walls(o).unwrap();
        assert!((left.unwrap() - 0.75).abs() < 1e-9, "{left:?}");
        assert!((right.unwrap() - 1.25).abs() < 1e-9, "{right:?}");
        // Into the back wall: a hole of its size.
        m.edit_object(o, |x| x.position = Point2::new(3.0, -0.35))
            .unwrap();
        let hole = m.hole_for_object(o).unwrap();
        let h = m.opening(hole).unwrap();
        assert_eq!(h.wall, back);
        assert!((h.along - 4.0).abs() < 1e-9, "{}", h.along);
        let ty = m.opening_type(h.opening_type).unwrap();
        assert!((ty.width - 1.8).abs() < 1e-9 && (ty.height - 2.5).abs() < 1e-9);
    }
}
