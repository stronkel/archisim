//! Views only for drawing (E7, as AutoCAD's model space for a detail or
//! a diagram): a name, a scale and their drafting elements
//! ([`ViewRef::Drafting`], in real metres). They are placed on sheets
//! like plans ([`crate::Viewport::drafting`]).
//!
//! Underlays ("calcos"): any plan, section or view for drawing shown
//! under a view for drawing or under a level's plan, in one colour, see-
//! through, moved and turned, as a reference to draw over.

use super::*;
use crate::drafting::ViewRef;
use crate::ids::DraftViewId;

/// A view only for drawing.
#[derive(Debug, Clone, PartialEq)]
pub struct DraftView {
    pub id: DraftViewId,
    pub name: String,
    /// Scale denominator it is drawn and placed at (20 is 1:20).
    pub scale: f64,
    /// What is shown under it, first lowest.
    pub underlays: Vec<Underlay>,
}

/// What an underlay shows.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnderlaySource {
    Level(LevelId),
    Section(SectionId),
    Drafting(DraftViewId),
}

/// A view shown under another as a reference.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Underlay {
    pub source: UnderlaySource,
    /// Everything drawn in this colour; `None`: its own colours.
    pub color: Option<[u8; 3]>,
    /// 0 (not seen) to 1.
    pub opacity: f64,
    /// Moved by this much (metres)…
    pub offset: Point2,
    /// …after being turned this much about its origin (radians,
    /// counter-clockwise).
    pub rotation: f64,
    /// Only this rectangle of the source shown (its two corners, in the
    /// source's own metres: a plan's x and y, a section's distance along
    /// it and height), before it is turned and moved (E17).
    pub crop: Option<[Point2; 2]>,
    /// Drawn this much bigger, about its origin, before it is turned
    /// (version 112): fitted to a known length or by two points.
    pub scale: f64,
    /// Locked: not moved, turned or scaled by mistake.
    pub locked: bool,
    /// Switched off without losing where it goes.
    pub hidden: bool,
}

impl Underlay {
    /// A source in light grey, half see-through, where it is.
    pub fn of(source: UnderlaySource) -> Self {
        Underlay {
            source,
            color: Some([120, 140, 170]),
            opacity: 0.5,
            offset: Point2::new(0.0, 0.0),
            rotation: 0.0,
            crop: None,
            scale: 1.0,
            locked: false,
            hidden: false,
        }
    }

    /// Where a point of the source is drawn: scaled, turned and moved.
    pub fn place(&self, p: Point2) -> Point2 {
        let (c, s) = (self.rotation.cos(), self.rotation.sin());
        let (x, y) = (p.x * self.scale, p.y * self.scale);
        Point2::new(x * c - y * s + self.offset.x, x * s + y * c + self.offset.y)
    }

    /// The point of the source drawn at `q`.
    pub fn source_of(&self, q: Point2) -> Point2 {
        let (c, s) = (self.rotation.cos(), self.rotation.sin());
        let (x, y) = (q.x - self.offset.x, q.y - self.offset.y);
        let k = if self.scale.abs() > 1e-12 {
            self.scale
        } else {
            1.0
        };
        Point2::new((x * c + y * s) / k, (-x * s + y * c) / k)
    }

    /// Moved so that what is drawn at `a1` goes to `b1` and what is drawn
    /// at `a2` towards `b2`: turned to line up, and, with `scale`, made
    /// the size that puts `a2` on `b2` too.
    pub fn fitted(&self, a1: Point2, a2: Point2, b1: Point2, b2: Point2, scale: bool) -> Self {
        let (da, db) = (a2 - a1, b2 - b1);
        let turn = db.y.atan2(db.x) - da.y.atan2(da.x);
        let k = if scale && da.length() > 1e-9 && db.length() > 1e-9 {
            db.length() / da.length()
        } else {
            1.0
        };
        self.mapped(turn, k, a1, b1)
    }

    /// Scaled by `k` about what is drawn at `at`.
    pub fn scaled_about(&self, at: Point2, k: f64) -> Self {
        self.mapped(0.0, k, at, at)
    }

    /// Followed by the move that turns by `turn` and scales by `k` what is
    /// drawn about `from`, and puts it at `to`.
    fn mapped(&self, turn: f64, k: f64, from: Point2, to: Point2) -> Self {
        let (c, s) = (turn.cos(), turn.sin());
        let d = self.offset - from;
        let off = Point2::new(
            (d.x * c - d.y * s) * k + to.x,
            (d.x * s + d.y * c) * k + to.y,
        );
        Underlay {
            offset: off,
            rotation: self.rotation + turn,
            scale: self.scale * k,
            ..*self
        }
    }

    /// Only the rectangle between `a` and `b` of the source, moved so that
    /// its lower left corner is at the origin (E17: a detail drawn over a
    /// piece of a section).
    pub fn cropped(source: UnderlaySource, a: Point2, b: Point2) -> Self {
        let lo = Point2::new(a.x.min(b.x), a.y.min(b.y));
        let hi = Point2::new(a.x.max(b.x), a.y.max(b.y));
        Underlay {
            crop: Some([lo, hi]),
            offset: Point2::new(-lo.x, -lo.y),
            ..Underlay::of(source)
        }
    }
}

impl Model {
    /// A new view for drawing, at 1:`scale`.
    pub fn add_draft_view(
        &mut self,
        name: impl Into<String>,
        scale: f64,
    ) -> Result<DraftViewId, ModelError> {
        finite(scale, "drafting view scale")?;
        if scale <= 0.0 {
            return Err(ModelError::InvalidShape("drafting view scale"));
        }
        let id = DraftViewId(self.take_id());
        self.draft_views.insert(
            id,
            DraftView {
                id,
                name: name.into(),
                scale,
                underlays: Vec::new(),
            },
        );
        Ok(id)
    }

    /// A new view for drawing over a piece of a section or elevation (E17):
    /// named after it, at 1:`scale`, with the rectangle between `a` and `b`
    /// of the section (distance along it, height) under it, its lower left
    /// corner at the origin.
    pub fn add_section_detail(
        &mut self,
        section: SectionId,
        a: Point2,
        b: Point2,
        scale: f64,
    ) -> Result<DraftViewId, ModelError> {
        let name = match self.section(section) {
            Some(s) => s.name.clone(),
            None => return Err(ModelError::UnknownSection(section)),
        };
        let taken = |n: &str| self.draft_views.values().any(|v| v.name == n);
        let mut title = format!("Detalle de {name}");
        let mut k = 2;
        while taken(&title) {
            title = format!("Detalle de {name} ({k})");
            k += 1;
        }
        let u = Underlay::cropped(UnderlaySource::Section(section), a, b);
        self.check_underlay(&u)?;
        let id = self.add_draft_view(title, scale)?;
        if let Some(v) = self.draft_views.get_mut(&id) {
            v.underlays.push(u);
        }
        Ok(id)
    }

    pub fn draft_view(&self, id: DraftViewId) -> Option<&DraftView> {
        self.draft_views.get(&id)
    }

    /// Every view for drawing, by name.
    pub fn draft_views(&self) -> Vec<&DraftView> {
        let mut v: Vec<&DraftView> = self.draft_views.values().collect();
        v.sort_by(|a, b| a.name.cmp(&b.name).then(a.id.0.cmp(&b.id.0)));
        v
    }

    /// Changes a view for drawing (its name, scale and underlays).
    pub fn set_drafting_view(&mut self, v: DraftView) -> Result<(), ModelError> {
        if !self.draft_views.contains_key(&v.id) {
            return Err(ModelError::UnknownDraftView(v.id));
        }
        self.check_draft_view(&v)?;
        self.draft_views.insert(v.id, v);
        Ok(())
    }

    fn check_draft_view(&self, v: &DraftView) -> Result<(), ModelError> {
        finite(v.scale, "drafting view scale")?;
        if v.scale <= 0.0 {
            return Err(ModelError::InvalidShape("drafting view scale"));
        }
        for u in &v.underlays {
            if u.source == UnderlaySource::Drafting(v.id) {
                return Err(ModelError::InvalidShape("a view cannot be under itself"));
            }
            self.check_underlay(u)?;
        }
        Ok(())
    }

    fn check_underlay(&self, u: &Underlay) -> Result<(), ModelError> {
        finite_point(u.offset, "underlay offset")?;
        finite(u.rotation, "underlay rotation")?;
        if !(u.scale.is_finite() && u.scale > 0.0) {
            return Err(ModelError::InvalidParameter(
                "the scale of an underlay must be more than 0".into(),
            ));
        }
        if let Some([a, b]) = u.crop {
            finite_point(a, "underlay crop")?;
            finite_point(b, "underlay crop")?;
            if (b.x - a.x).abs() < 1e-6 || (b.y - a.y).abs() < 1e-6 {
                return Err(ModelError::InvalidShape("underlay crop"));
            }
        }
        finite(u.opacity, "underlay opacity")?;
        if !(0.0..=1.0).contains(&u.opacity) {
            return Err(ModelError::InvalidShape("underlay opacity"));
        }
        match u.source {
            UnderlaySource::Level(l) if !self.levels.contains_key(&l) => {
                Err(ModelError::UnknownLevel(l))
            }
            UnderlaySource::Section(s) if self.section(s).is_none() => {
                Err(ModelError::UnknownSection(s))
            }
            UnderlaySource::Drafting(d) if !self.draft_views.contains_key(&d) => {
                Err(ModelError::UnknownDraftView(d))
            }
            _ => Ok(()),
        }
    }

    /// Removes a view for drawing, with its drafting elements, the views
    /// of it on sheets and the underlays that show it.
    pub fn remove_draft_view(&mut self, id: DraftViewId) -> Result<DraftView, ModelError> {
        let v = self
            .draft_views
            .remove(&id)
            .ok_or(ModelError::UnknownDraftView(id))?;
        self.drafts.retain(|_, d| d.view != ViewRef::Drafting(id));
        self.viewports.retain(|_, vp| vp.drafting != Some(id));
        self.drop_underlays_of(UnderlaySource::Drafting(id));
        Ok(v)
    }

    /// The underlays of a level's plan.
    pub fn level_underlays(&self, level: LevelId) -> &[Underlay] {
        self.level_underlays
            .get(&level)
            .map_or(&[], |v| v.as_slice())
    }

    /// Sets the underlays of a level's plan.
    pub fn set_level_underlays(
        &mut self,
        level: LevelId,
        underlays: Vec<Underlay>,
    ) -> Result<(), ModelError> {
        if !self.levels.contains_key(&level) {
            return Err(ModelError::UnknownLevel(level));
        }
        for u in &underlays {
            if u.source == UnderlaySource::Level(level) {
                return Err(ModelError::InvalidShape("a plan cannot be under itself"));
            }
            self.check_underlay(u)?;
        }
        if underlays.is_empty() {
            self.level_underlays.remove(&level);
        } else {
            self.level_underlays.insert(level, underlays);
        }
        Ok(())
    }

    /// Every level's underlays.
    pub fn all_level_underlays(&self) -> impl Iterator<Item = (LevelId, &Vec<Underlay>)> {
        self.level_underlays.iter().map(|(l, u)| (*l, u))
    }

    /// Takes away the underlays that show `source` (it is gone).
    pub(crate) fn drop_underlays_of(&mut self, source: UnderlaySource) {
        for v in self.draft_views.values_mut() {
            v.underlays.retain(|u| u.source != source);
        }
        for u in self.level_underlays.values_mut() {
            u.retain(|u| u.source != source);
        }
        self.level_underlays.retain(|_, u| !u.is_empty());
        if let UnderlaySource::Level(l) = source {
            self.level_underlays.remove(&l);
        }
    }

    pub(crate) fn restore_draft_view(&mut self, v: DraftView) -> Result<(), ModelError> {
        // Underlays may name views restored after it: checked at the end.
        finite(v.scale, "drafting view scale")?;
        self.draft_views.insert(v.id, v);
        Ok(())
    }

    pub(crate) fn check_restored_underlays(&self) -> Result<(), ModelError> {
        for v in self.draft_views.values() {
            self.check_draft_view(v)?;
        }
        for u in self.level_underlays.values().flatten() {
            self.check_underlay(u)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn views_for_drawing_and_underlays() {
        let mut m = Model::new("t");
        let pb = m.add_level("PB", 0.0);
        let p1 = m.add_level("P1", 3.0);
        let d = m.add_draft_view("Detalle de alero", 5.0).unwrap();
        let layer = m.draw_layers()[0].id;
        m.add_draft(crate::drafting::NewDraft::new(
            ViewRef::Drafting(d),
            layer,
            crate::drafting::Shape::Line {
                a: Point2::new(0.0, 0.0),
                b: Point2::new(1.0, 0.0),
            },
        ))
        .unwrap();
        assert_eq!(m.drafts_in(ViewRef::Drafting(d)).count(), 1);
        // The detail under the first floor's plan, the ground floor under
        // the detail.
        m.set_level_underlays(p1, vec![Underlay::of(UnderlaySource::Drafting(d))])
            .unwrap();
        let mut v = m.draft_view(d).unwrap().clone();
        v.underlays.push(Underlay::of(UnderlaySource::Level(pb)));
        m.set_drafting_view(v.clone()).unwrap();
        // Not under itself, not see-through past 1.
        let mut bad = v.clone();
        bad.underlays
            .push(Underlay::of(UnderlaySource::Drafting(d)));
        assert!(m.set_drafting_view(bad).is_err());
        let mut bad = v.clone();
        bad.underlays[0].opacity = 1.5;
        assert!(m.set_drafting_view(bad).is_err());
        // Gone: its drafting and the underlays of it too.
        m.remove_draft_view(d).unwrap();
        assert_eq!(m.drafts_in(ViewRef::Drafting(d)).count(), 0);
        assert!(m.level_underlays(p1).is_empty());
    }

    #[test]
    fn an_underlay_is_fitted_by_two_points() {
        let p = Point2::new;
        let u = Underlay::of(UnderlaySource::Level(crate::LevelId(1)));
        // Its points (0,0) and (2,0) go to (5,5) and (5,9): moved, turned
        // a quarter and twice the size.
        let f = u.fitted(p(0.0, 0.0), p(2.0, 0.0), p(5.0, 5.0), p(5.0, 9.0), true);
        assert!(f.place(p(0.0, 0.0)).distance(p(5.0, 5.0)) < 1e-9);
        assert!(f.place(p(2.0, 0.0)).distance(p(5.0, 9.0)) < 1e-9);
        assert!((f.scale - 2.0).abs() < 1e-9);
        assert!(f.source_of(p(5.0, 9.0)).distance(p(2.0, 0.0)) < 1e-9);
        // Without scaling, only lined up.
        let g = f.fitted(p(5.0, 5.0), p(5.0, 9.0), p(0.0, 0.0), p(1.0, 0.0), false);
        assert!(g.place(p(0.0, 0.0)).distance(p(0.0, 0.0)) < 1e-9);
        assert!((g.scale - 2.0).abs() < 1e-9);
        // A known length: 4 m drawn that are really 6.
        let h = f.scaled_about(p(5.0, 5.0), 1.5);
        assert!(h.place(p(2.0, 0.0)).distance(p(5.0, 11.0)) < 1e-9);
    }
}
