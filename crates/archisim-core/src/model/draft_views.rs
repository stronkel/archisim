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
}
