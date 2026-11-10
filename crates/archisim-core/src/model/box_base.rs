//! What is built on site before the boxes come (M10, `docs/MODULAR.md`):
//! the supports under the corners and the middles of each box, the slab
//! they stand on and the block plinth round the edge, made from the boxes
//! on the ground.
//!
//! The supports are worked out, not kept: [`Model::box_supports`]. What
//! [`Model::build_box_base`] makes is ordinary slabs, columns and walls on a
//! level of their own under the boxes («Cimentación de las cajas»), so they
//! are drawn, measured, exported and calculated as any other.

use std::collections::BTreeSet;

use super::*;

/// Supports closer than this are one (the corners of two boxes side by
/// side, 2 cm apart).
pub const SUPPORT_MERGE: f64 = 0.35;

/// How high its plan is cut over the slab: through the piers.
pub const BASE_CUT: f64 = 0.30;

/// The level the base is built on.
pub const BASE_LEVEL: &str = "Cimentación de las cajas";

/// Where a support goes under its box.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SupportPlace {
    Corner,
    /// Half way along a long side.
    Middle,
}

/// One support: a short pier with its plate and bolts, under one or more
/// boxes.
#[derive(Debug, Clone, PartialEq)]
pub struct BoxSupport {
    /// «A1», «A2»…, in order along the boxes.
    pub mark: String,
    pub at: Point2,
    /// Height of the boxes' chassis base over project zero (top of the
    /// plate).
    pub z: f64,
    pub place: SupportPlace,
    /// The boxes it carries, and what part of each one's weight: 3/32 at a
    /// corner and 5/16 half way along a long side (a frame on three
    /// supports per side, as a continuous beam of two spans).
    pub boxes: Vec<(BoxId, f64)>,
}

impl BoxSupport {
    /// Shared by more than one box: a bigger pier and plate.
    pub fn shared(&self) -> bool {
        self.boxes.len() > 1
    }
}

/// What a corner and a middle support take of a box on six of them.
pub const CORNER_SHARE: f64 = 3.0 / 32.0;
pub const MIDDLE_SHARE: f64 = 5.0 / 16.0;

/// How the base is made.
#[derive(Debug, Clone, PartialEq)]
pub struct BaseOptions {
    /// Pier height: from the slab's top to the boxes' chassis base.
    pub pier_height: f64,
    /// Pier side under one box and under two or more.
    pub pier: f64,
    pub shared_pier: f64,
    /// Slab thickness and how far it goes out past the boxes.
    pub slab: f64,
    pub margin: f64,
    /// The block plinth round the edge, flush with the boxes' faces.
    pub plinth: bool,
    pub plinth_thickness: f64,
}

impl Default for BaseOptions {
    fn default() -> Self {
        BaseOptions {
            pier_height: 0.60,
            pier: 0.25,
            shared_pier: 0.40,
            slab: 0.45,
            margin: 0.30,
            plinth: true,
            plinth_thickness: 0.20,
        }
    }
}

impl BaseOptions {
    pub fn is_valid(&self) -> bool {
        let ok = |v: f64, lo: f64, hi: f64| v.is_finite() && v >= lo && v <= hi;
        ok(self.pier_height, 0.10, 3.0)
            && ok(self.pier, 0.10, 1.5)
            && ok(self.shared_pier, 0.10, 1.5)
            && ok(self.slab, 0.10, 2.0)
            && ok(self.margin, 0.0, 3.0)
            && ok(self.plinth_thickness, 0.05, 0.60)
    }
}

/// What [`Model::build_box_base`] made.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct BaseMade {
    pub level: Option<LevelId>,
    pub slabs: Vec<SlabId>,
    pub piers: Vec<ColumnId>,
    pub plinth: Vec<WallId>,
    /// What was there from an earlier run and was taken away.
    pub replaced: usize,
}

/// Names of the types the base uses.
pub const PIER_TYPE: &str = "Apoyo de caja";
pub const SLAB_TYPE: &str = "Losa de cimentación de las cajas";
pub const PLINTH_TYPE: &str = "Zócalo de bloque de las cajas";

impl Model {
    /// The boxes standing on the ground: on no other box.
    pub fn ground_boxes(&self) -> Vec<&ModularBox> {
        self.boxes
            .values()
            .filter(|b| self.box_below(b.id).is_none())
            .filter(|b| {
                // On the lowest level that has boxes.
                let z = self.level(b.level).map_or(0.0, |l| l.elevation);
                !self
                    .boxes
                    .values()
                    .any(|o| self.level(o.level).is_some_and(|l| l.elevation < z - 1e-6))
            })
            .collect()
    }

    /// The supports under the boxes on the ground: one under each corner
    /// and half way along each long side, under the posts' line (half the
    /// floor frame's tube in from the faces); those of boxes side by side
    /// become one.
    pub fn box_supports(&self) -> Vec<BoxSupport> {
        let mut found: Vec<BoxSupport> = Vec::new();
        let mut boxes = self.ground_boxes();
        boxes.sort_by(|a, b| a.mark.cmp(&b.mark));
        for b in boxes {
            let z = self.level(b.level).map_or(0.0, |l| l.elevation) + b.bottom;
            let inset = b.chassis.edge.width / 2.0;
            let (u, v) = (b.along(), b.across());
            let at = |s: f64, t: f64| b.origin + u * s + v * t;
            let (l, w) = (b.length, b.width);
            let (s0, s1, t0, t1) = (inset, l - inset, inset, w - inset);
            let points = [
                (at(s0, t0), SupportPlace::Corner),
                (at(l / 2.0, t0), SupportPlace::Middle),
                (at(s1, t0), SupportPlace::Corner),
                (at(s1, t1), SupportPlace::Corner),
                (at(l / 2.0, t1), SupportPlace::Middle),
                (at(s0, t1), SupportPlace::Corner),
            ];
            for (p, place) in points {
                let share = match place {
                    SupportPlace::Corner => CORNER_SHARE,
                    SupportPlace::Middle => MIDDLE_SHARE,
                };
                if let Some(s) = found
                    .iter_mut()
                    .find(|s| s.at.distance(p) < SUPPORT_MERGE && (s.z - z).abs() < 0.05)
                {
                    let n = s.boxes.len() as f64;
                    s.at = (s.at * n + p) * (1.0 / (n + 1.0));
                    s.boxes.push((b.id, share));
                    if place == SupportPlace::Middle {
                        s.place = SupportPlace::Middle;
                    }
                } else {
                    found.push(BoxSupport {
                        mark: String::new(),
                        at: p,
                        z,
                        place,
                        boxes: vec![(b.id, share)],
                    });
                }
            }
        }
        // Marked in rows: by y, then by x, in the first box's axes.
        let (u, v) = self
            .ground_boxes()
            .first()
            .map_or((Point2::new(1.0, 0.0), Point2::new(0.0, 1.0)), |b| {
                (b.along(), b.across())
            });
        let key = |p: Point2| ((p.x * v.x + p.y * v.y) * 10.0).round() as i64;
        found.sort_by(|a, b| {
            key(a.at)
                .cmp(&key(b.at))
                .then((a.at.x * u.x + a.at.y * u.y).total_cmp(&(b.at.x * u.x + b.at.y * u.y)))
        });
        for (i, s) in found.iter_mut().enumerate() {
            s.mark = format!("A{}", i + 1);
        }
        found
    }

    /// The outline of the boxes on the ground, grown by `margin`: one
    /// polygon per group of boxes that touch (counter-clockwise). Boxes
    /// turned to each other give their joint outline's hull.
    pub fn ground_box_outlines(&self, margin: f64) -> Vec<Vec<Point2>> {
        let boxes = self.ground_boxes();
        let Some(first) = boxes.first() else {
            return Vec::new();
        };
        let (u, v) = (first.along(), first.across());
        let square = boxes.iter().all(|b| {
            let c = (b.along().x * u.x + b.along().y * u.y).abs();
            (c - 1.0).abs() < 1e-6 || c < 1e-6
        });
        // Rectangles in the first box's axes, grown by the margin and by
        // half the widest joint so that boxes side by side meet.
        let grow = margin + 0.05;
        let to = |p: Point2| (p.x * u.x + p.y * u.y, p.x * v.x + p.y * v.y);
        let rects: Vec<[f64; 4]> = boxes
            .iter()
            .map(|b| {
                let cs = b.corners().map(to);
                let (mut x0, mut y0, mut x1, mut y1) = (f64::MAX, f64::MAX, f64::MIN, f64::MIN);
                for (x, y) in cs {
                    x0 = x0.min(x);
                    y0 = y0.min(y);
                    x1 = x1.max(x);
                    y1 = y1.max(y);
                }
                [x0 - grow, y0 - grow, x1 + grow, y1 + grow]
            })
            .collect();
        let back = |x: f64, y: f64| Point2::new(u.x * x + v.x * y, u.y * x + v.y * y);
        if !square {
            let pts: Vec<Point2> = boxes
                .iter()
                .flat_map(|b| super::site::grow_polygon(&b.outline(), margin))
                .collect();
            return vec![hull(&pts)];
        }
        let loops = rect_union(&rects);
        loops
            .into_iter()
            .map(|l| {
                // Back from the joint's half to the margin.
                let p: Vec<Point2> = l.iter().map(|(x, y)| back(*x, *y)).collect();
                super::site::grow_polygon(&p, -0.05)
            })
            .collect()
    }

    /// Builds the base of the boxes on the ground: the level under them
    /// (made if missing), the slab, one pier per support and, if asked,
    /// the plinth. What an earlier run made on that level (elements of the
    /// base's types) is taken away first.
    pub fn build_box_base(&mut self, options: &BaseOptions) -> Result<BaseMade, ModelError> {
        if !options.is_valid() {
            return Err(ModelError::InvalidParameter(
                "obra previa de las cajas".into(),
            ));
        }
        let supports = self.box_supports();
        if supports.is_empty() {
            return Err(ModelError::InvalidParameter(
                "no hay cajas sobre el terreno".into(),
            ));
        }
        let mut made = BaseMade::default();
        let low = supports.iter().map(|s| s.z).fold(f64::MAX, f64::min);
        let top = low - options.pier_height;
        let level = match self
            .levels()
            .into_iter()
            .find(|l| l.name == BASE_LEVEL)
            .map(|l| l.id)
        {
            Some(l) => {
                self.set_level_elevation(l, top)?;
                l
            }
            None => self.add_level(BASE_LEVEL, top),
        };
        made.level = Some(level);
        // Its plan cut through the piers and the plinth, by a template of
        // its own.
        let cut = (options.pier_height / 2.0).min(BASE_CUT);
        let found = self
            .view_templates()
            .find(|t| t.name == BASE_LEVEL)
            .map(|t| t.id);
        let template = match found {
            Some(id) => {
                let mut t = self.view_template(id).cloned().expect("template");
                t.cut_plane = Some(cut);
                self.set_view_template(t)?;
                id
            }
            None => {
                let mut style = self.level_style(level);
                style.template = None;
                self.add_view_template(ViewTemplate {
                    id: ViewTemplateId(0),
                    name: BASE_LEVEL.into(),
                    style,
                    scale: None,
                    cut_plane: Some(cut),
                })?
            }
        };
        if let Some(t) = self.view_template(template).cloned() {
            let style = t.applied(&self.level_style(level));
            self.set_level_style(level, style)?;
        }
        // Types: the template's concrete and block if there are, or new.
        let concrete =
            self.base_material("hormig", "Hormigón HA-25", "concrete", CutPattern::Cross);
        let block = self.base_material(
            "bloque",
            "Bloque de hormigón",
            "masonry",
            CutPattern::Diagonal,
        );
        let pier_type = |m: &mut Model, side: f64| -> Result<MemberTypeId, ModelError> {
            let name = format!("{PIER_TYPE} {:.0}×{:.0}", side * 100.0, side * 100.0);
            if let Some(t) = m.member_types().find(|t| t.name == name) {
                return Ok(t.id);
            }
            m.add_member_type(
                name,
                MemberUsage::Column,
                concrete,
                Profile::Rectangle {
                    width: side,
                    depth: side,
                },
            )
        };
        let single = pier_type(self, options.pier)?;
        let shared = pier_type(self, options.shared_pier)?;
        let slab_name = format!("{SLAB_TYPE} {:.0}", options.slab * 100.0);
        let found = self
            .slab_types()
            .find(|t| t.name == slab_name)
            .map(|t| t.id);
        let slab_type = match found {
            Some(t) => t,
            None => self.add_slab_type(
                slab_name,
                SlabFunction::BaseSlab,
                vec![Layer {
                    material: concrete,
                    thickness: options.slab,
                    function: LayerFunction::Structure,
                }],
            )?,
        };
        let plinth_name = format!("{PLINTH_TYPE} {:.0}", options.plinth_thickness * 100.0);
        let found = self
            .wall_types()
            .find(|t| t.name == plinth_name)
            .map(|t| t.id);
        let plinth_type = match found {
            Some(t) => t,
            None => self.add_wall_type(
                plinth_name,
                WallFunction::Exterior,
                vec![Layer {
                    material: block,
                    thickness: options.plinth_thickness,
                    function: LayerFunction::Structure,
                }],
            )?,
        };
        // Away with an earlier base.
        let ours_member: BTreeSet<MemberTypeId> = self
            .member_types()
            .filter(|t| t.name.starts_with(PIER_TYPE))
            .map(|t| t.id)
            .collect();
        let old_columns: Vec<ColumnId> = self
            .columns()
            .filter(|c| c.level == level && ours_member.contains(&c.member_type))
            .map(|c| c.id)
            .collect();
        let old_slabs: Vec<SlabId> = self
            .slabs()
            .filter(|s| {
                s.level == level
                    && self
                        .slab_type(s.slab_type)
                        .is_some_and(|t| t.name.starts_with(SLAB_TYPE))
            })
            .map(|s| s.id)
            .collect();
        let old_walls: Vec<WallId> = self
            .walls()
            .filter(|w| {
                w.level == level
                    && self
                        .wall_type(w.wall_type)
                        .is_some_and(|t| t.name.starts_with(PLINTH_TYPE))
            })
            .map(|w| w.id)
            .collect();
        made.replaced = old_columns.len() + old_slabs.len() + old_walls.len();
        for c in old_columns {
            self.remove_column(c)?;
        }
        for s in old_slabs {
            self.remove_slab(s)?;
        }
        for w in old_walls {
            self.remove_wall(w)?;
        }
        // The slab, one per group of boxes.
        for outline in self.ground_box_outlines(options.margin) {
            made.slabs.push(self.add_slab(NewSlab {
                slab_type: slab_type,
                level,
                outline,
                top_offset: 0.0,
            })?);
        }
        // The piers, up to the boxes' chassis.
        let angle = self.ground_boxes().first().map_or(0.0, |b| b.angle);
        for s in &supports {
            made.piers.push(self.add_column(NewColumn {
                member_type: if s.shared() { shared } else { single },
                level,
                position: s.at,
                rotation: angle,
                base_offset: 0.0,
                top: WallTop::Unconnected,
                height: s.z - top,
            })?);
        }
        // The plinth, its outer face flush with the boxes' faces.
        if options.plinth {
            for outline in self.ground_box_outlines(0.0) {
                let n = outline.len();
                for i in 0..n {
                    // Clockwise, so that the outside is on the left.
                    let (a, b) = (outline[(i + 1) % n], outline[i]);
                    if a.distance(b) < 0.05 {
                        continue;
                    }
                    made.plinth.push(self.add_wall(NewWall {
                        wall_type: plinth_type,
                        level,
                        start: a,
                        end: b,
                        bulge: 0.0,
                        reference: ReferenceLine::ExteriorFace,
                        height: low - top,
                        base_offset: 0.0,
                        top: WallTop::Unconnected,
                    })?);
                }
            }
        }
        Ok(made)
    }

    /// A material of the project whose name has `key`, or a new one.
    fn base_material(
        &mut self,
        key: &str,
        name: &str,
        category: &str,
        pattern: CutPattern,
    ) -> MaterialId {
        if let Some(m) = self
            .materials()
            .find(|m| m.name.to_lowercase().contains(key))
            .map(|m| m.id)
        {
            return m;
        }
        self.add_material(name, category, [200, 200, 200], pattern)
    }
}

/// The union of axis-aligned rectangles `[x0, y0, x1, y1]` as its outer
/// loops, counter-clockwise, with no point in the middle of a straight run.
/// (Holes, a courtyard ringed by boxes, are left out.)
pub fn rect_union(rects: &[[f64; 4]]) -> Vec<Vec<(f64, f64)>> {
    let mut xs: Vec<f64> = rects.iter().flat_map(|r| [r[0], r[2]]).collect();
    let mut ys: Vec<f64> = rects.iter().flat_map(|r| [r[1], r[3]]).collect();
    let tidy = |v: &mut Vec<f64>| {
        v.sort_by(f64::total_cmp);
        v.dedup_by(|a, b| (*a - *b).abs() < 1e-6);
    };
    tidy(&mut xs);
    tidy(&mut ys);
    let (nx, ny) = (xs.len().saturating_sub(1), ys.len().saturating_sub(1));
    let full = |i: i64, j: i64| -> bool {
        if i < 0 || j < 0 || i >= nx as i64 || j >= ny as i64 {
            return false;
        }
        let (cx, cy) = (
            (xs[i as usize] + xs[i as usize + 1]) / 2.0,
            (ys[j as usize] + ys[j as usize + 1]) / 2.0,
        );
        rects
            .iter()
            .any(|r| cx > r[0] && cx < r[2] && cy > r[1] && cy < r[3])
    };
    // Directed edges with the filled cell on their left.
    let mut edges: Vec<((usize, usize), (usize, usize))> = Vec::new();
    for i in 0..nx as i64 {
        for j in 0..ny as i64 {
            if !full(i, j) {
                continue;
            }
            let (a, b) = (i as usize, j as usize);
            if !full(i, j - 1) {
                edges.push(((a, b), (a + 1, b)));
            }
            if !full(i + 1, j) {
                edges.push(((a + 1, b), (a + 1, b + 1)));
            }
            if !full(i, j + 1) {
                edges.push(((a + 1, b + 1), (a, b + 1)));
            }
            if !full(i - 1, j) {
                edges.push(((a, b + 1), (a, b)));
            }
        }
    }
    let mut used = vec![false; edges.len()];
    let mut loops = Vec::new();
    for start in 0..edges.len() {
        if used[start] {
            continue;
        }
        let mut chain = Vec::new();
        let mut k = start;
        loop {
            used[k] = true;
            chain.push(edges[k].0);
            let end = edges[k].1;
            // At a pinch (two cells touching by a corner) turn left first.
            let next = (0..edges.len())
                .filter(|e| !used[*e] && edges[*e].0 == end)
                .min_by_key(|e| {
                    let d0 = (
                        end.0 as i64 - edges[k].0 .0 as i64,
                        end.1 as i64 - edges[k].0 .1 as i64,
                    );
                    let d1 = (
                        edges[*e].1 .0 as i64 - end.0 as i64,
                        edges[*e].1 .1 as i64 - end.1 as i64,
                    );
                    // Left turn first, then straight, then right.
                    let cross = d0.0 * d1.1 - d0.1 * d1.0;
                    -cross.signum()
                });
            match next {
                Some(n) => k = n,
                None => break,
            }
            if edges[k].0 == edges[start].0 {
                break;
            }
        }
        let pts: Vec<(f64, f64)> = chain.iter().map(|(i, j)| (xs[*i], ys[*j])).collect();
        // Drop points in the middle of a straight run.
        let n = pts.len();
        let kept: Vec<(f64, f64)> = (0..n)
            .filter(|i| {
                let (a, b, c) = (pts[(i + n - 1) % n], pts[*i], pts[(i + 1) % n]);
                ((b.0 - a.0) * (c.1 - b.1) - (b.1 - a.1) * (c.0 - b.0)).abs() > 1e-12
            })
            .map(|i| pts[i])
            .collect();
        let area: f64 = (0..kept.len())
            .map(|i| {
                let (a, b) = (kept[i], kept[(i + 1) % kept.len()]);
                a.0 * b.1 - b.0 * a.1
            })
            .sum();
        if kept.len() >= 3 && area > 0.0 {
            loops.push(kept);
        }
    }
    loops
}

/// The convex hull of points, counter-clockwise.
fn hull(points: &[Point2]) -> Vec<Point2> {
    let mut p: Vec<Point2> = points.to_vec();
    p.sort_by(|a, b| a.x.total_cmp(&b.x).then(a.y.total_cmp(&b.y)));
    p.dedup_by(|a, b| a.distance(*b) < 1e-9);
    if p.len() < 3 {
        return p;
    }
    let cross =
        |o: Point2, a: Point2, b: Point2| (a.x - o.x) * (b.y - o.y) - (a.y - o.y) * (b.x - o.x);
    let mut lower: Vec<Point2> = Vec::new();
    for &q in &p {
        while lower.len() >= 2 && cross(lower[lower.len() - 2], lower[lower.len() - 1], q) <= 0.0 {
            lower.pop();
        }
        lower.push(q);
    }
    let mut upper: Vec<Point2> = Vec::new();
    for &q in p.iter().rev() {
        while upper.len() >= 2 && cross(upper[upper.len() - 2], upper[upper.len() - 1], q) <= 0.0 {
            upper.pop();
        }
        upper.push(q);
    }
    lower.pop();
    upper.pop();
    lower.extend(upper);
    lower
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Two boxes side by side and a third along the first: an L.
    fn ell() -> (Model, LevelId) {
        let mut m = Model::new("P");
        let lv = m.add_level("PB", 0.0);
        for o in [
            Point2::new(0.0, 0.0),
            Point2::new(0.0, BOX_WIDTH + BOX_JOINT),
            Point2::new(BOX_LENGTH + BOX_JOINT, 0.0),
        ] {
            m.add_box(NewBox::usual(lv, o, 0.0)).unwrap();
        }
        (m, lv)
    }

    #[test]
    fn supports_are_shared_by_boxes_side_by_side() {
        let (m, _) = ell();
        let s = m.box_supports();
        // 18 points: the joint along the long sides shares three, the
        // joint across the ends two; where the three boxes meet, one
        // support carries all three.
        assert_eq!(s.len(), 18 - 3 - 2, "{s:#?}");
        assert_eq!(s.iter().filter(|s| s.shared()).count(), 4);
        assert_eq!(s.iter().filter(|s| s.boxes.len() == 3).count(), 1);
        // Each box's shares add up to its whole weight.
        for b in m.boxes() {
            let sum: f64 = s
                .iter()
                .flat_map(|s| s.boxes.iter())
                .filter(|(id, _)| *id == b.id)
                .map(|(_, f)| f)
                .sum();
            assert!((sum - 1.0).abs() < 1e-9, "{sum}");
        }
        assert_eq!(s[0].mark, "A1");
        assert!((s[0].z - m.boxes().next().unwrap().bottom).abs() < 1e-9);
    }

    #[test]
    fn the_union_of_rectangles_is_its_outline() {
        // An L of two rectangles touching and a separate square.
        let l = rect_union(&[
            [0.0, 0.0, 4.0, 1.0],
            [0.0, 1.0, 1.0, 3.0],
            [6.0, 0.0, 7.0, 1.0],
        ]);
        assert_eq!(l.len(), 2);
        let mut sizes: Vec<usize> = l.iter().map(|p| p.len()).collect();
        sizes.sort();
        assert_eq!(sizes, vec![4, 6]);
        // Overlapping ones merge.
        let o = rect_union(&[[0.0, 0.0, 2.0, 2.0], [1.0, 1.0, 3.0, 3.0]]);
        assert_eq!(o.len(), 1);
        assert_eq!(o[0].len(), 8);
    }

    #[test]
    fn the_base_is_built_and_built_again() {
        let (mut m, lv) = ell();
        let made = m.build_box_base(&BaseOptions::default()).unwrap();
        let base = made.level.unwrap();
        assert_ne!(base, lv);
        let bottom = m.boxes().next().unwrap().bottom;
        assert!((m.level(base).unwrap().elevation - (bottom - 0.60)).abs() < 1e-9);
        assert_eq!(made.slabs.len(), 1);
        // Its plan follows a template that cuts through the piers.
        let t = m.level_style(base).template.unwrap();
        assert_eq!(m.view_template(t).unwrap().cut_plane, Some(BASE_CUT));
        assert_eq!(made.piers.len(), 13);
        // The L: six sides of plinth.
        assert_eq!(made.plinth.len(), 6);
        let slab = m.slab(made.slabs[0]).unwrap();
        assert_eq!(slab.outline.len(), 6);
        // The piers reach the chassis.
        for c in &made.piers {
            let c = m.column(*c).unwrap();
            assert!((c.height - 0.60).abs() < 1e-9);
        }
        // Not part of any box.
        for b in m.boxes() {
            assert!(m.box_members(b.id).columns.is_empty());
        }
        // Again: the same, with what was there taken away.
        let again = m
            .build_box_base(&BaseOptions {
                plinth: false,
                ..Default::default()
            })
            .unwrap();
        assert_eq!(again.level, Some(base));
        assert_eq!(again.replaced, 1 + 13 + 6);
        assert_eq!(m.columns().count(), 13);
        assert_eq!(m.walls().count(), 0);
    }

    #[test]
    fn only_boxes_on_the_ground_stand_on_supports() {
        let (mut m, lv) = ell();
        let up = m.add_level("P1", 3.6);
        m.add_box(NewBox::usual(up, Point2::new(0.0, 0.0), 0.0))
            .unwrap();
        assert_eq!(m.ground_boxes().len(), 3);
        assert!(m.ground_boxes().iter().all(|b| b.level == lv));
        assert_eq!(m.box_supports().len(), 13);
    }
}
