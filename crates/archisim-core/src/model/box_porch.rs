//! The porch of a box as a light steel module (M17, `docs/MODULAR.md`):
//! along a side of a box, posts of square tube at its outer corners (and
//! between them when the front is long), beams of rectangular tube round
//! its edge (the two returns bolted to the box's ceiling frame) and a light
//! roof of steel deck at the height of the box's ceiling frame. They are
//! ordinary columns, beams and a roof slab of their own types, so they are
//! drawn, measured, exported and calculated like any other; the porch is
//! made in the factory as one frame and lifted with the box.

use super::*;

/// The types the porch is made of (made again when not found).
pub const PORCH_POST: &str = "Pilar de porche";
pub const PORCH_BEAM: &str = "Viga de porche";
pub const PORCH_ROOF: &str = "Cubierta de porche";
/// Posts at most this far apart along the front, m.
pub const PORCH_SPAN: f64 = 3.6;

/// How to make a porch.
#[derive(Debug, Clone, PartialEq)]
pub struct PorchOptions {
    /// The side of the box (0 A, 1 B, 2 C, 3 D).
    pub side: usize,
    /// How far out from the box, m.
    pub depth: f64,
    /// Along the side from its start, m; `to` past the side's length is
    /// cut to it.
    pub from: f64,
    pub to: f64,
    /// Square tube of the posts, m.
    pub post: f64,
    pub post_wall: f64,
    /// Rectangular tube of the beams (depth × width), m.
    pub beam_depth: f64,
    pub beam_width: f64,
    pub beam_wall: f64,
    /// The steel deck of the roof, m.
    pub deck: f64,
}

impl Default for PorchOptions {
    fn default() -> Self {
        PorchOptions {
            side: 0,
            depth: 2.0,
            from: 0.0,
            to: f64::INFINITY,
            post: 0.10,
            post_wall: 0.004,
            beam_depth: 0.16,
            beam_width: 0.08,
            beam_wall: 0.005,
            deck: 0.05,
        }
    }
}

/// What was made.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct PorchMade {
    pub posts: Vec<ColumnId>,
    pub beams: Vec<BeamId>,
    pub roof: Option<SlabId>,
    /// Its outline in plan.
    pub outline: Vec<Point2>,
    /// The frame's steel, kg (posts and beams; the deck apart).
    pub steel: f64,
    /// Elements of an earlier porch in the same place, taken away.
    pub replaced: usize,
}

fn hollow_kg(width: f64, depth: f64, wall: f64) -> f64 {
    let inner = (width - 2.0 * wall).max(0.0) * (depth - 2.0 * wall).max(0.0);
    (width * depth - inner) * STEEL_DENSITY
}

impl Model {
    /// Makes a porch on a side of a box (an earlier porch in the same place
    /// is taken away first).
    pub fn build_porch(&mut self, id: BoxId, o: &PorchOptions) -> Result<PorchMade, ModelError> {
        let b = self
            .modular_box(id)
            .cloned()
            .ok_or(ModelError::InvalidShape("no such box"))?;
        if !(o.depth > 0.3 && o.depth <= 6.0) || o.side > 3 {
            return Err(ModelError::InvalidParameter(
                "a porch from 0,3 to 6 m deep on a side A–D".into(),
            ));
        }
        let (a, z) = b.side(o.side);
        let side = a.distance(z);
        let u = (z - a)
            .normalized()
            .ok_or(ModelError::InvalidShape("a side without length"))?;
        let from = o.from.clamp(0.0, side);
        let to = o.to.min(side);
        if to - from < 0.5 {
            return Err(ModelError::InvalidShape("a porch shorter than 0,5 m"));
        }
        // Outwards: away from the box's middle.
        let centre = b
            .outline()
            .iter()
            .fold(Point2::new(0.0, 0.0), |s, p| s + *p)
            * 0.25;
        let mut n = Point2::new(-u.y, u.x);
        if (a - centre).dot(n) < 0.0 {
            n = n * -1.0;
        }
        let p0 = a + u * from;
        let p1 = a + u * to;
        let p2 = p1 + n * o.depth;
        let p3 = p0 + n * o.depth;
        let outline = vec![p0, p1, p2, p3];
        // Heights over the level: the top of the box's ceiling frame.
        let roof_z = b.bottom + b.chassis.roof_at + b.chassis.roof_edge.depth;
        let mut made = PorchMade {
            outline: outline.clone(),
            ..Default::default()
        };
        // Away with an earlier porch here.
        let mid = outline.iter().fold(Point2::new(0.0, 0.0), |s, p| s + *p) * 0.25;
        let grown: Vec<Point2> = outline
            .iter()
            .map(|p| *p + (*p - mid).normalized().unwrap_or(Point2::new(0.0, 0.0)) * 0.42)
            .collect();
        let inside = |p: Point2| point_in_polygon(p, &grown);
        let posts_old: Vec<ColumnId> = self
            .columns()
            .filter(|c| c.level == b.level && inside(c.position))
            .filter(|c| {
                self.member_type(c.member_type)
                    .is_some_and(|t| t.name.starts_with(PORCH_POST))
            })
            .map(|c| c.id)
            .collect();
        let beams_old: Vec<BeamId> = self
            .beams()
            .filter(|x| x.level == b.level && inside((x.start + x.end) * 0.5))
            .filter(|x| {
                self.member_type(x.member_type)
                    .is_some_and(|t| t.name.starts_with(PORCH_BEAM))
            })
            .map(|x| x.id)
            .collect();
        let roofs_old: Vec<SlabId> = self
            .slabs()
            .filter(|s| s.level == b.level && s.outline.iter().all(|p| inside(*p)))
            .filter(|s| {
                self.slab_type(s.slab_type)
                    .is_some_and(|t| t.name.starts_with(PORCH_ROOF))
            })
            .map(|s| s.id)
            .collect();
        made.replaced = posts_old.len() + beams_old.len() + roofs_old.len();
        for c in posts_old {
            self.remove_column(c)?;
        }
        for x in beams_old {
            self.remove_beam(x)?;
        }
        for s in roofs_old {
            self.remove_slab(s)?;
        }
        // Types.
        let found = self
            .materials()
            .find(|m| m.name.contains("S275"))
            .map(|m| m.id);
        let steel = match found {
            Some(m) => m,
            None => self.add_material("Acero S275", "steel", [120, 126, 134], CutPattern::Solid),
        };
        let post_name = format!(
            "{PORCH_POST} {:.0}×{:.0}×{:.0}",
            o.post * 1000.0,
            o.post * 1000.0,
            o.post_wall * 1000.0
        );
        let found = self
            .member_types()
            .find(|t| t.name == post_name)
            .map(|t| t.id);
        let post_type = match found {
            Some(t) => t,
            None => self.add_member_type(
                post_name,
                MemberUsage::Column,
                steel,
                Profile::RectHollow {
                    width: o.post,
                    depth: o.post,
                    wall: o.post_wall,
                },
            )?,
        };
        let beam_name = format!(
            "{PORCH_BEAM} {:.0}×{:.0}×{:.0}",
            o.beam_depth * 1000.0,
            o.beam_width * 1000.0,
            o.beam_wall * 1000.0
        );
        let found = self
            .member_types()
            .find(|t| t.name == beam_name)
            .map(|t| t.id);
        let beam_type = match found {
            Some(t) => t,
            None => self.add_member_type(
                beam_name,
                MemberUsage::Beam,
                steel,
                Profile::RectHollow {
                    width: o.beam_width,
                    depth: o.beam_depth,
                    wall: o.beam_wall,
                },
            )?,
        };
        let roof_name = format!("{PORCH_ROOF} (chapa {:.0} mm)", o.deck * 1000.0);
        let found = self
            .slab_types()
            .find(|t| t.name == roof_name)
            .map(|t| t.id);
        let roof_type = match found {
            Some(t) => t,
            None => self.add_slab_type(
                roof_name,
                SlabFunction::Roof,
                vec![Layer {
                    material: steel,
                    thickness: o.deck,
                    function: LayerFunction::Structure,
                }],
            )?,
        };
        // Posts: at the outer corners, half a tube in, and between them.
        let half = o.post / 2.0;
        let front = to - from;
        let bays = (front / PORCH_SPAN).ceil().max(1.0) as usize;
        let level = b.level;
        for i in 0..=bays {
            let t = (front - o.post) * i as f64 / bays as f64 + half;
            let at = p0 + u * t + n * (o.depth - half);
            made.posts.push(self.add_column(NewColumn {
                member_type: post_type,
                level,
                position: at,
                rotation: u.y.atan2(u.x),
                base_offset: 0.0,
                top: WallTop::Unconnected,
                height: roof_z - o.beam_depth,
            })?);
        }
        // Beams: along the front over the posts, and the two returns to the
        // box's frame.
        let front_line = |t: f64| p0 + u * t + n * (o.depth - half);
        for (s, e) in [
            (front_line(0.0), front_line(front)),
            (p0 + u * half, front_line(half)),
            (p1 - u * half, front_line(front - half)),
        ] {
            made.beams.push(self.add_beam(NewBeam {
                member_type: beam_type,
                level,
                start: s,
                end: e,
                top_offset: roof_z,
            })?);
        }
        // The deck on the beams.
        made.roof = Some(self.add_slab(NewSlab {
            slab_type: roof_type,
            level,
            outline,
            top_offset: roof_z + o.deck,
        })?);
        let post_kg = hollow_kg(o.post, o.post, o.post_wall);
        let beam_kg = hollow_kg(o.beam_width, o.beam_depth, o.beam_wall);
        made.steel = post_kg * (roof_z - o.beam_depth) * made.posts.len() as f64
            + beam_kg * (front + 2.0 * (o.depth - half));
        Ok(made)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_porch_along_the_long_side() {
        let mut m = Model::new("T");
        let lv = m.add_level("PB", 0.0);
        let b = m
            .add_box(NewBox::usual(lv, Point2::new(0.0, 0.0), 0.0))
            .unwrap();
        let made = m
            .build_porch(
                b,
                &PorchOptions {
                    side: 0,
                    depth: 2.0,
                    ..Default::default()
                },
            )
            .unwrap();
        // 6,20 m of front: two bays, three posts; three beams; a roof.
        assert_eq!(made.posts.len(), 3);
        assert_eq!(made.beams.len(), 3);
        assert!(made.roof.is_some());
        assert!(made.steel > 50.0 && made.steel < 400.0, "{}", made.steel);
        // Outside the box.
        let bx = m.modular_box(b).unwrap().clone();
        for c in &made.posts {
            let p = m.column(*c).unwrap().position;
            assert!(!point_in_polygon(p, &bx.outline()), "{p:?}");
        }
        // Again: the same, the old one taken away.
        let again = m.build_porch(b, &PorchOptions::default()).unwrap();
        assert_eq!(again.replaced, 3 + 3 + 1);
        assert_eq!(m.columns().count(), 3);
    }
}
