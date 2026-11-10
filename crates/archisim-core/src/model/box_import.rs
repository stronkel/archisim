//! Boxes over a project drawn without them (M12, `docs/MODULAR.md`): a
//! house modelled in another program and imported, whose floors are the
//! boxes' floors. Each floor slab becomes a box of its size (grown to the
//! system's module when it fits), and the sides between two boxes are
//! read from the walls along the joint: whose they are, or open.

use super::*;

/// How far a wall's line may be from a joint to count as the joint's wall.
pub const JOINT_REACH: f64 = 0.30;

/// How much of a side a joint's walls must cover for it to be walled.
pub const WALLED_SHARE: f64 = 0.30;

/// How much a floor may be grown to reach the module.
pub const MODULE_REACH: f64 = 0.30;

/// The floor's rectangle in its own axes: the direction of its longest
/// edge, the corner where both measures are least, and its length (along)
/// and width (across). `None` if it is not near a rectangle.
pub fn floor_rectangle(outline: &[Point2]) -> Option<(f64, Point2, f64, f64)> {
    let n = outline.len();
    if n < 3 {
        return None;
    }
    let (mut best, mut angle) = (0.0, 0.0);
    for i in 0..n {
        let d = outline[(i + 1) % n] - outline[i];
        let l = d.length();
        if l > best {
            best = l;
            angle = d.y.atan2(d.x);
        }
    }
    let (u, v) = (
        Point2::new(angle.cos(), angle.sin()),
        Point2::new(-angle.sin(), angle.cos()),
    );
    let along = |p: Point2| (p.x * u.x + p.y * u.y, p.x * v.x + p.y * v.y);
    let (mut a0, mut a1, mut c0, mut c1) = (f64::MAX, f64::MIN, f64::MAX, f64::MIN);
    for p in outline {
        let (a, c) = along(*p);
        a0 = a0.min(a);
        a1 = a1.max(a);
        c0 = c0.min(c);
        c1 = c1.max(c);
    }
    let (length, width) = (a1 - a0, c1 - c0);
    // Near a rectangle: its area is most of the bounding one.
    let area = crate::math::signed_area(outline).abs();
    if length < 1.0 || width < 1.0 || area < 0.85 * length * width {
        return None;
    }
    let o = u * a0 + v * c0;
    Some((angle, Point2::new(o.x, o.y), length, width))
}

/// Whether two boxes' rectangles overlap by more than `gap`.
fn overlap(a: &[Point2; 4], b: &[Point2; 4], gap: f64) -> bool {
    let axes = |r: &[Point2; 4]| [r[1] - r[0], r[3] - r[0]];
    for axis in axes(a).into_iter().chain(axes(b)) {
        let l = axis.length();
        if l < 1e-9 {
            continue;
        }
        let n = axis * (1.0 / l);
        let span = |r: &[Point2; 4]| {
            r.iter().fold((f64::MAX, f64::MIN), |(lo, hi), p| {
                let d = p.x * n.x + p.y * n.y;
                (lo.min(d), hi.max(d))
            })
        };
        let (s, t) = (span(a), span(b));
        if s.1 <= t.0 + gap || t.1 <= s.0 + gap {
            return false;
        }
    }
    true
}

impl Model {
    /// One box over each of these floor slabs, of the slab's size; each
    /// side grows to the module (`length` × `width`) when it is short by
    /// less than [`MODULE_REACH`] and growing does not run into another of
    /// them. The boxes take the slab's level and the usual base, height
    /// and chassis. Slabs that are not near a rectangle are left out.
    pub fn boxes_on_floors(
        &mut self,
        slabs: &[SlabId],
        module: Option<(f64, f64)>,
    ) -> Result<Vec<BoxId>, ModelError> {
        let mut plans: Vec<NewBox> = Vec::new();
        for id in slabs {
            let s = self.slab(*id).ok_or(ModelError::UnknownElement(id.0))?;
            let Some((angle, origin, length, width)) = floor_rectangle(&s.outline) else {
                continue;
            };
            let mut b = NewBox::usual(s.level, origin, angle);
            b.length = length;
            b.width = width;
            plans.push(b);
        }
        if let Some((ml, mw)) = module {
            // Grow each, the length first, where it does not run into the
            // others.
            for k in 0..plans.len() {
                for along in [true, false] {
                    let b = &plans[k];
                    let (have, want) = if along { (b.length, ml) } else { (b.width, mw) };
                    let more = want - have;
                    if more <= 1e-6 || more > MODULE_REACH {
                        continue;
                    }
                    let (u, v) = (
                        Point2::new(b.angle.cos(), b.angle.sin()),
                        Point2::new(-b.angle.sin(), b.angle.cos()),
                    );
                    let dir = if along { u } else { v };
                    let rect = |n: &NewBox| {
                        let (u, v) = (
                            Point2::new(n.angle.cos(), n.angle.sin()) * n.length,
                            Point2::new(-n.angle.sin(), n.angle.cos()) * n.width,
                        );
                        [n.origin, n.origin + u, n.origin + u + v, n.origin + v]
                    };
                    // Grown on both ends, else towards one, else the other.
                    let mut grown = None;
                    for back in [more / 2.0, 0.0, more] {
                        let mut g = b.clone();
                        if along {
                            g.length = want;
                        } else {
                            g.width = want;
                        }
                        g.origin = g.origin - dir * back;
                        let mine = rect(&g);
                        let clear = plans
                            .iter()
                            .enumerate()
                            .all(|(j, o)| j == k || !overlap(&mine, &rect(o), 0.005));
                        if clear {
                            grown = Some(g);
                            break;
                        }
                    }
                    if let Some(g) = grown {
                        plans[k] = g;
                    }
                }
            }
        }
        let mut made = Vec::new();
        for p in plans {
            made.push(self.add_box(p)?);
        }
        Ok(made)
    }

    /// The sides of these boxes read from the walls: a side against
    /// another box is the joint's wall when walls along the joint (within
    /// [`JOINT_REACH`], on its side of it) cover [`WALLED_SHARE`] of it,
    /// and open when not; a side against no box is free. A joint with
    /// walls on both sides is left with two (a box issue says so).
    pub fn box_sides_from_walls(&mut self, ids: &[BoxId]) -> Result<(), ModelError> {
        for id in ids {
            let b = self
                .modular_box(*id)
                .ok_or(ModelError::UnknownElement(id.0))?
                .clone();
            let neighbours = self.box_neighbours(*id);
            let mut sides = b.sides;
            for (i, n) in neighbours.iter().enumerate() {
                if n.is_none() {
                    sides[i] = SideState::Free;
                    continue;
                }
                let (s, e) = b.side(i);
                let len = s.distance(e);
                let dir = (e - s) * (1.0 / len);
                // Inwards, towards the box's centre.
                let inward = dir.perp_left();
                let mut covered = 0.0;
                for w in self.walls_on_level(b.level) {
                    let d = w.end - w.start;
                    let wl = d.length();
                    if wl < 0.05 {
                        continue;
                    }
                    // Parallel to the side.
                    if (d.x * dir.y - d.y * dir.x).abs() / wl > 0.05 {
                        continue;
                    }
                    let mid = (w.start + w.end) * 0.5;
                    let off = (mid - s).x * inward.x + (mid - s).y * inward.y;
                    // On this box's side of the joint, close to it.
                    if !(-0.005..=JOINT_REACH).contains(&off) {
                        continue;
                    }
                    let t = |p: Point2| ((p - s).x * dir.x + (p - s).y * dir.y).clamp(0.0, len);
                    covered += (t(w.end) - t(w.start)).abs();
                }
                sides[i] = if covered >= WALLED_SHARE * len {
                    SideState::Wall
                } else {
                    SideState::Open
                };
            }
            self.edit_box(*id, |x| x.sides = sides)?;
        }
        Ok(())
    }
}

impl Model {
    /// Whether a slab may be a box's floor: a floor at its level (not a
    /// roof over it), near a rectangle 4 to 9,5 m long and 2,4 to 3,8 m
    /// wide, closed by walls as a box is (three or more of the level's
    /// walls stand on it or at its edge; a terrace is not).
    pub fn is_box_floor(&self, id: SlabId) -> bool {
        let Some(s) = self.slab(id) else {
            return false;
        };
        let floor = s.top_offset.abs() < 0.5
            && self
                .slab_type(s.slab_type)
                .is_some_and(|t| !matches!(t.function, SlabFunction::Roof));
        let Some((angle, o, l, w)) = floor_rectangle(&s.outline) else {
            return false;
        };
        let (u, v) = (
            Point2::new(angle.cos(), angle.sin()),
            Point2::new(-angle.sin(), angle.cos()),
        );
        let inside = |p: Point2| {
            let d = p - o;
            let (a, c) = (d.x * u.x + d.y * u.y, d.x * v.x + d.y * v.y);
            (-0.3..=l + 0.3).contains(&a) && (-0.3..=w + 0.3).contains(&c)
        };
        let walls = self
            .walls_on_level(s.level)
            .filter(|x| inside((x.start + x.end) * 0.5))
            .count();
        floor && walls >= 3 && (4.0..=9.5).contains(&l) && (2.4..=3.8).contains(&w)
    }

    /// The floors the boxes of an imported house go on: the box floors of
    /// the level that has most of them, with no box on them yet.
    pub fn box_floor_slabs(&self) -> Vec<SlabId> {
        let mut best: Vec<SlabId> = Vec::new();
        for l in self.levels() {
            let here: Vec<SlabId> = self
                .slabs()
                .filter(|s| s.level == l.id)
                .map(|s| s.id)
                .filter(|id| self.is_box_floor(*id))
                .filter(|id| {
                    let s = self.slab(*id).unwrap();
                    let c = s.outline.iter().fold(Point2::new(0.0, 0.0), |a, p| a + *p)
                        * (1.0 / s.outline.len() as f64);
                    !self.boxes().any(|b| {
                        b.level == s.level && super::roofs::point_in_polygon(c, &b.outline())
                    })
                })
                .collect();
            if here.len() > best.len() {
                best = here;
            }
        }
        best
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rect(x0: f64, y0: f64, x1: f64, y1: f64) -> Vec<Point2> {
        vec![
            Point2::new(x0, y0),
            Point2::new(x1, y0),
            Point2::new(x1, y1),
            Point2::new(x0, y1),
        ]
    }

    #[test]
    fn floors_become_boxes_and_walls_their_sides() {
        let mut m = Model::new("P");
        let lv = m.add_level("PB", 0.0);
        let mat = m.add_material("Madera", "wood", [180; 3], CutPattern::Solid);
        let st = m
            .add_slab_type(
                "Suelo",
                SlabFunction::Floor,
                vec![Layer {
                    material: mat,
                    thickness: 0.02,
                    function: LayerFunction::Finish,
                }],
            )
            .unwrap();
        let wt = m
            .add_wall_type(
                "Tabique",
                WallFunction::Interior,
                vec![Layer {
                    material: mat,
                    thickness: 0.1,
                    function: LayerFunction::Structure,
                }],
            )
            .unwrap();
        // Three floors 3,6 wide and 6,0 long side by side, long side along
        // y; the module is 6,10.
        let mut slabs = Vec::new();
        for i in 0..3 {
            let x = i as f64 * 3.6;
            slabs.push(
                m.add_slab(NewSlab {
                    slab_type: st,
                    level: lv,
                    outline: rect(x, 0.0, x + 3.6, 6.0),
                    top_offset: 0.0,
                })
                .unwrap(),
            );
        }
        // A partition along the first joint, inside the second box.
        m.add_wall(NewWall {
            wall_type: wt,
            level: lv,
            start: Point2::new(3.65, 0.2),
            end: Point2::new(3.65, 5.8),
            bulge: 0.0,
            reference: ReferenceLine::Center,
            height: 2.5,
            base_offset: 0.0,
            top: WallTop::Unconnected,
        })
        .unwrap();
        let boxes = m.boxes_on_floors(&slabs, Some((6.10, 3.60))).unwrap();
        assert_eq!(boxes.len(), 3);
        let b = m.modular_box(boxes[0]).unwrap();
        assert!((b.length - 6.10).abs() < 1e-9, "grown to the module");
        assert!((b.width - 3.60).abs() < 1e-9);
        m.box_sides_from_walls(&boxes).unwrap();
        let sides = |i: usize| m.modular_box(boxes[i]).unwrap().sides;
        // The joint between the first two: the second has the wall.
        let s0 = sides(0);
        let s1 = sides(1);
        assert_eq!(
            s0.iter().filter(|s| **s == SideState::Open).count(),
            1,
            "{s0:?}"
        );
        assert_eq!(
            s1.iter().filter(|s| **s == SideState::Wall).count(),
            1,
            "{s1:?}"
        );
        // The joint between the second and the third: no wall, both open.
        assert_eq!(
            s1.iter().filter(|s| **s == SideState::Open).count(),
            1,
            "{s1:?}"
        );
        assert_eq!(
            sides(2).iter().filter(|s| **s == SideState::Free).count(),
            3
        );
        // The partition belongs to the second box.
        assert_eq!(m.box_members(boxes[1]).walls.len(), 1);
    }

    #[test]
    fn a_floor_must_be_near_a_rectangle() {
        let l = vec![
            Point2::new(0.0, 0.0),
            Point2::new(6.0, 0.0),
            Point2::new(6.0, 1.5),
            Point2::new(1.5, 1.5),
            Point2::new(1.5, 6.0),
            Point2::new(0.0, 6.0),
        ];
        assert!(floor_rectangle(&l).is_none());
        let (_, _, len, w) = floor_rectangle(&rect(0.0, 0.0, 3.6, 6.0)).unwrap();
        assert!((len - 6.0).abs() < 1e-9 && (w - 3.6).abs() < 1e-9);
    }
}
