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
}

/// What a piece of a railing is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RailingPieceKind {
    Post,
    Handrail,
    BottomRail,
    Baluster,
}

/// A straight bar of a railing: from `a` to `b` (plan points, heights
/// from the railing's foot), `size` square.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RailingPiece {
    pub kind: RailingPieceKind,
    pub a: (Point2, f64),
    pub b: (Point2, f64),
    pub size: f64,
}

/// Sizes of the pieces (metres).
pub const POST_SIZE: f64 = 0.04;
pub const HANDRAIL_SIZE: f64 = 0.05;
pub const RAIL_SIZE: f64 = 0.03;
pub const BALUSTER_SIZE: f64 = 0.016;
/// Height of the bottom rail's centre above the foot.
pub const BOTTOM_RAIL: f64 = 0.10;

impl Railing {
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
            // Bars between posts, evenly, no gap over 10 cm.
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
        if !self.materials.contains_key(&r.material) {
            return Err(ModelError::UnknownMaterial(r.material));
        }
        for p in &r.path {
            finite_point(*p, "railing point")?;
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
        assert_eq!(
            m.edit_railing(id, |r| r.path.truncate(1)),
            Err(ModelError::InvalidRailing)
        );
        let back = Model::from_parts(m.to_parts()).unwrap();
        assert_eq!(back, m);
    }
}
