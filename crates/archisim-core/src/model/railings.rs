//! Loose railings ("barandillas"): a guard along a line of points on a
//! level, for terraces, flat roofs, balconies and voids. Posts at every
//! corner and in between no farther apart than asked, a handrail on top,
//! a bottom rail and vertical bars no more than 10 cm apart (CTE DB-SUA 1,
//! 3.2.3: no gap a 10 cm sphere can pass).
//!
//! IFC: `IfcRailing` (`GUARDRAIL`).

use super::*;

/// Gap between bars a railing must not exceed (CTE DB-SUA 1).
pub const MAX_BAR_GAP: f64 = 0.10;

#[derive(Debug, Clone, PartialEq)]
pub struct Railing {
    pub id: RailingId,
    pub guid: GlobalId,
    pub level: LevelId,
    /// The line it runs along, two points at least.
    pub path: Vec<Point2>,
    /// Height of its foot above the level.
    pub base_offset: f64,
    /// From its foot to the top of the handrail.
    pub height: f64,
    /// Largest distance between posts.
    pub post_spacing: f64,
    pub material: MaterialId,
    /// Glass panels between the posts instead of bars (their material).
    pub glass: Option<MaterialId>,
    /// Its type (E12): an object type of the «Barandilla» family, which
    /// says its posts, handrail, infill, end posts, anchors and
    /// materials. Without one, the plain railing of posts, rails and bars
    /// (`material`, `glass`).
    pub railing_type: Option<ObjectTypeId>,
}

/// Data needed to create a railing.
#[derive(Debug, Clone, PartialEq)]
pub struct NewRailing {
    pub level: LevelId,
    pub path: Vec<Point2>,
    pub base_offset: f64,
    pub height: f64,
    pub post_spacing: f64,
    pub material: MaterialId,
    pub glass: Option<MaterialId>,
    pub railing_type: Option<ObjectTypeId>,
}

/// One straight run of a typed railing: from `start` along `dir` for
/// `len`, with a free end (`0`) or a corner at each end (at its start, a
/// corner it draws the post of: `1`; at its end, one it leaves to the
/// next run: `1`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RailingRun {
    pub start: Point2,
    pub dir: Vec2,
    pub len: f64,
    pub start_end: (i32, i32),
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RailingPieceKind {
    Post,
    Handrail,
    BottomRail,
    Baluster,
    /// A glass panel between two posts, from the bottom rail to the
    /// handrail.
    Panel,
}

/// A straight bar of a railing: from `a` to `b` (plan points, heights
/// from the railing's foot), `size` square; a glass panel is `size` thick
/// and `tall` high about the line from `a` to `b`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RailingPiece {
    pub kind: RailingPieceKind,
    pub a: (Point2, f64),
    pub b: (Point2, f64),
    pub size: f64,
    pub tall: f64,
}

/// Thickness of a glass panel (laminated 6+6).
pub const GLASS_THICKNESS: f64 = 0.012;

/// Sizes of the pieces (metres).
pub const POST_SIZE: f64 = 0.04;
pub const HANDRAIL_SIZE: f64 = 0.05;
pub const RAIL_SIZE: f64 = 0.03;
pub const BALUSTER_SIZE: f64 = 0.016;
/// Height of the bottom rail's centre above the foot.
pub const BOTTOM_RAIL: f64 = 0.10;

impl Railing {
    /// The straight runs a typed railing is made of, one per segment.
    pub fn runs(&self) -> Vec<RailingRun> {
        let n = self.path.len();
        self.path
            .windows(2)
            .enumerate()
            .filter_map(|(i, w)| {
                let dir = (w[1] - w[0]).normalized()?;
                Some(RailingRun {
                    start: w[0],
                    dir,
                    len: w[0].distance(w[1]),
                    start_end: (if i == 0 { 0 } else { 1 }, if i + 2 == n { 0 } else { 1 }),
                })
            })
            .collect()
    }

    pub fn length(&self) -> f64 {
        self.path.windows(2).map(|w| w[0].distance(w[1])).sum()
    }

    /// Posts, handrail, bottom rail and bars, segment by segment.
    pub fn pieces(&self) -> Vec<RailingPiece> {
        let mut out = Vec::new();
        let h = self.height;
        let top = h - HANDRAIL_SIZE;
        let rail_top = BOTTOM_RAIL + RAIL_SIZE / 2.0;
        let piece = |kind, a: Point2, za: f64, b: Point2, zb: f64, size| RailingPiece {
            kind,
            a: (a, za),
            b: (b, zb),
            size,
            tall: size,
        };
        let n = self.path.len();
        for (i, w) in self.path.windows(2).enumerate() {
            let (p, q) = (w[0], w[1]);
            let len = p.distance(q);
            let Some(u) = (q - p).normalized() else {
                continue;
            };
            let at = |s: f64| p + u * s;
            // Rails run over the posts, from post centre to post centre.
            out.push(piece(
                RailingPieceKind::Handrail,
                p,
                h - HANDRAIL_SIZE / 2.0,
                q,
                h - HANDRAIL_SIZE / 2.0,
                HANDRAIL_SIZE,
            ));
            out.push(piece(
                RailingPieceKind::BottomRail,
                p,
                BOTTOM_RAIL,
                q,
                BOTTOM_RAIL,
                RAIL_SIZE,
            ));
            let bays = (len / self.post_spacing - 1e-9).ceil().max(1.0) as usize;
            let last = if i + 2 == n { bays } else { bays - 1 };
            for k in 0..=last {
                let s = len * k as f64 / bays as f64;
                out.push(piece(
                    RailingPieceKind::Post,
                    at(s),
                    0.0,
                    at(s),
                    top,
                    POST_SIZE,
                ));
            }
            // Glass panels between posts, or bars evenly, no gap over 10 cm.
            if self.glass.is_some() {
                for k in 0..bays {
                    let (s0, s1) = (
                        len * k as f64 / bays as f64 + POST_SIZE / 2.0,
                        len * (k + 1) as f64 / bays as f64 - POST_SIZE / 2.0,
                    );
                    let (za, zb) = (rail_top, top);
                    out.push(RailingPiece {
                        kind: RailingPieceKind::Panel,
                        a: (at(s0), (za + zb) / 2.0),
                        b: (at(s1), (za + zb) / 2.0),
                        size: GLASS_THICKNESS,
                        tall: zb - za,
                    });
                }
                continue;
            }
            for k in 0..bays {
                let (s0, s1) = (
                    len * k as f64 / bays as f64 + POST_SIZE / 2.0,
                    len * (k + 1) as f64 / bays as f64 - POST_SIZE / 2.0,
                );
                let clear = s1 - s0;
                let bars = ((clear - MAX_BAR_GAP) / (MAX_BAR_GAP + BALUSTER_SIZE))
                    .ceil()
                    .max(0.0) as usize;
                let step = clear / (bars + 1) as f64;
                for j in 1..=bars {
                    let s = s0 + step * j as f64;
                    out.push(piece(
                        RailingPieceKind::Baluster,
                        at(s),
                        rail_top,
                        at(s),
                        top,
                        BALUSTER_SIZE,
                    ));
                }
            }
        }
        out
    }
}

impl Model {
    fn validate_railing(&self, r: &NewRailing) -> Result<(), ModelError> {
        if !self.levels.contains_key(&r.level) {
            return Err(ModelError::UnknownLevel(r.level));
        }
        for m in std::iter::once(r.material).chain(r.glass) {
            if !self.materials.contains_key(&m) {
                return Err(ModelError::UnknownMaterial(m));
            }
        }
        for p in &r.path {
            finite_point(*p, "railing point")?;
        }
        if let Some(t) = r.railing_type {
            if !self.is_railing_type(t) {
                return Err(ModelError::InvalidRailing);
            }
        }
        for (v, what) in [
            (r.base_offset, "railing base offset"),
            (r.height, "railing height"),
            (r.post_spacing, "railing post spacing"),
        ] {
            finite(v, what)?;
        }
        let ok = r.path.len() >= 2
            && r.path.windows(2).all(|w| w[0].distance(w[1]) > 0.05)
            && (0.3..=3.0).contains(&r.height)
            && (0.3..=5.0).contains(&r.post_spacing);
        if !ok {
            return Err(ModelError::InvalidRailing);
        }
        Ok(())
    }

    pub fn add_railing(&mut self, spec: NewRailing) -> Result<RailingId, ModelError> {
        self.validate_railing(&spec)?;
        let id = RailingId(self.take_id());
        self.railings.insert(
            id,
            Railing {
                id,
                guid: GlobalId::new(),
                level: spec.level,
                path: spec.path,
                base_offset: spec.base_offset,
                height: spec.height,
                post_spacing: spec.post_spacing,
                material: spec.material,
                glass: spec.glass,
                railing_type: spec.railing_type,
            },
        );
        self.stamp_phase(id.0);
        Ok(id)
    }

    /// Changes a railing; nothing changes if the result is not valid.
    pub fn edit_railing(
        &mut self,
        id: RailingId,
        edit: impl FnOnce(&mut Railing),
    ) -> Result<(), ModelError> {
        let mut r = self
            .railings
            .get(&id)
            .cloned()
            .ok_or(ModelError::UnknownRailing(id))?;
        edit(&mut r);
        self.validate_railing(&spec_of(&r))?;
        self.railings.insert(id, r);
        Ok(())
    }

    pub fn remove_railing(&mut self, id: RailingId) -> Result<Railing, ModelError> {
        self.railings
            .remove(&id)
            .ok_or(ModelError::UnknownRailing(id))
    }

    pub fn railing(&self, id: RailingId) -> Option<&Railing> {
        self.railings.get(&id)
    }

    pub fn railings(&self) -> impl Iterator<Item = &Railing> {
        self.railings.values()
    }

    pub fn railings_on_level(&self, level: LevelId) -> impl Iterator<Item = &Railing> {
        self.railings.values().filter(move |r| r.level == level)
    }

    /// Whether an object type is a railing type (its family's category is
    /// `railing`).
    pub fn is_railing_type(&self, t: ObjectTypeId) -> bool {
        self.object_types
            .get(&t)
            .and_then(|t| self.families.get(&t.family))
            .is_some_and(|f| f.def().category == "railing")
    }

    /// The railing types of the project, by name.
    pub fn railing_types(&self) -> Vec<&ObjectType> {
        let mut v: Vec<&ObjectType> = self
            .object_types
            .values()
            .filter(|t| t.own.is_none() && self.is_railing_type(t.id))
            .collect();
        v.sort_by(|a, b| a.name.cmp(&b.name));
        v
    }

    /// What ArchiSim makes a run of a typed railing with: the generator
    /// and its measurements (the type's, the railing's height and post
    /// spacing, the run's length and ends).
    pub fn railing_run_params(
        &self,
        id: RailingId,
        run: &RailingRun,
    ) -> Option<(String, archisim_props::Params)> {
        let r = self.railings.get(&id)?;
        let (kind, mut p) = self.prop_params(r.railing_type?)?;
        p.set("run_len", run.len);
        p.set("height", r.height);
        p.set("post_spacing", r.post_spacing);
        p.set("start", run.start_end.0 as f64);
        p.set("end", run.start_end.1 as f64);
        Some((kind, p))
    }

    pub(super) fn restore_railings(
        &mut self,
        list: Vec<Railing>,
        claim: &mut structure::Claim<'_>,
    ) -> Result<(), RestoreError> {
        for r in list {
            let name = format!("railing {} ({})", r.id.0, r.guid);
            claim(r.id.0, Some(&r.guid))
                .and_then(|_| self.validate_railing(&spec_of(&r)))
                .map_err(|error| RestoreError {
                    element: name,
                    error,
                })?;
            self.railings.insert(r.id, r);
        }
        Ok(())
    }
}

fn spec_of(r: &Railing) -> NewRailing {
    NewRailing {
        level: r.level,
        path: r.path.clone(),
        base_offset: r.base_offset,
        height: r.height,
        post_spacing: r.post_spacing,
        material: r.material,
        glass: r.glass,
        railing_type: r.railing_type,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn railing_pieces_keep_bars_close() {
        let mut m = Model::new("t");
        let pb = m.add_level("PB", 0.0);
        let steel = m.add_material("Acero", "steel", [120, 126, 134], CutPattern::Solid);
        let id = m
            .add_railing(NewRailing {
                level: pb,
                path: vec![
                    Point2::new(0.0, 0.0),
                    Point2::new(3.0, 0.0),
                    Point2::new(3.0, 2.0),
                ],
                base_offset: 0.0,
                height: 1.1,
                post_spacing: 1.5,
                material: steel,
                glass: None,
                railing_type: None,
            })
            .unwrap();
        let r = m.railing(id).unwrap();
        assert!((r.length() - 5.0).abs() < 1e-12);
        let pieces = r.pieces();
        let posts: Vec<_> = pieces
            .iter()
            .filter(|p| p.kind == RailingPieceKind::Post)
            .collect();
        // 3 m in two bays and 2 m in two: posts at 0, 1.5, 3, 4, 5.
        assert_eq!(posts.len(), 5);
        // Bars along the first segment: no gap over 10 cm.
        let mut xs: Vec<f64> = pieces
            .iter()
            .filter(|p| {
                matches!(p.kind, RailingPieceKind::Baluster | RailingPieceKind::Post)
                    && p.a.0.y.abs() < 1e-9
            })
            .map(|p| p.a.0.x)
            .collect();
        xs.sort_by(f64::total_cmp);
        for w in xs.windows(2) {
            assert!(w[1] - w[0] - BALUSTER_SIZE.max(POST_SIZE) < MAX_BAR_GAP + 1e-9);
        }
        // With glass: a panel per bay, no bars.
        m.edit_railing(id, |r| r.glass = Some(steel)).unwrap();
        let pieces = m.railing(id).unwrap().pieces();
        let count = |k| pieces.iter().filter(|p| p.kind == k).count();
        assert_eq!(count(RailingPieceKind::Panel), 4);
        assert_eq!(count(RailingPieceKind::Baluster), 0);
        assert_eq!(
            m.edit_railing(id, |r| r.path.truncate(1)),
            Err(ModelError::InvalidRailing)
        );
        let back = Model::from_parts(m.to_parts()).unwrap();
        assert_eq!(back, m);
    }
}
