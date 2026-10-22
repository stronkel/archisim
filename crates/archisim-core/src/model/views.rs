//! Views kept with the project: saved plans of a level (ArchiCAD's view
//! map, Revit's duplicated views: «Planta baja — superficies»,
//! «Planta baja — albañilería»), each with its scale, cut plane and what
//! it shows; ceiling plans; the style of each level's own plan; view
//! templates to give several views the same look; and reference planes,
//! lines to draw against seen in every plan.

use super::*;
use crate::graphics::ViewStyle;

/// What a plan view looks at.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Hash)]
pub enum PlanKind {
    /// The floor plan: cut at the cut plane, looking down.
    #[default]
    Floor,
    /// The reflected ceiling plan: cut at the cut plane, looking up at
    /// the ceiling (the slab above, beams, lights); doors without swings,
    /// no furniture.
    Ceiling,
}

impl PlanKind {
    pub fn code(self) -> &'static str {
        match self {
            PlanKind::Floor => "floor",
            PlanKind::Ceiling => "ceiling",
        }
    }

    pub fn from_code(code: &str) -> Option<PlanKind> {
        [PlanKind::Floor, PlanKind::Ceiling]
            .into_iter()
            .find(|k| k.code() == code)
    }

    pub fn label(self) -> &'static str {
        match self {
            PlanKind::Floor => "Planta",
            PlanKind::Ceiling => "Planta de techos",
        }
    }
}

/// A saved plan of a level.
#[derive(Debug, Clone, PartialEq)]
pub struct PlanView {
    pub id: PlanViewId,
    pub name: String,
    pub level: LevelId,
    pub kind: PlanKind,
    /// Scale denominator (50 is 1:50).
    pub scale: f64,
    /// Height of the cut plane above the level, metres.
    pub cut_plane: f64,
    pub style: ViewStyle,
}

/// A named look given to views (Revit's view template): what they show
/// and how each category is drawn.
#[derive(Debug, Clone, PartialEq)]
pub struct ViewTemplate {
    pub id: ViewTemplateId,
    pub name: String,
    pub style: ViewStyle,
}

/// A reference plane: a named line to draw against (Revit's reference
/// plane), seen dashed in every plan and snapped to. Not built, not in
/// the IFC.
#[derive(Debug, Clone, PartialEq)]
pub struct RefPlane {
    pub id: RefPlaneId,
    pub name: String,
    pub start: Point2,
    pub end: Point2,
}

impl RefPlane {
    pub fn length(&self) -> f64 {
        self.start.distance(self.end)
    }
}

impl Model {
    // ---- Saved plans --------------------------------------------------------

    pub fn plan_views(&self) -> impl Iterator<Item = &PlanView> {
        self.plan_views.values()
    }

    pub fn plan_view(&self, id: PlanViewId) -> Option<&PlanView> {
        self.plan_views.get(&id)
    }

    /// The saved plans of a level, by name.
    pub fn plan_views_of(&self, level: LevelId) -> Vec<&PlanView> {
        let mut v: Vec<&PlanView> = self
            .plan_views
            .values()
            .filter(|p| p.level == level)
            .collect();
        v.sort_by(|a, b| a.name.cmp(&b.name));
        v
    }

    fn check_plan_view(&self, v: &PlanView, except: Option<PlanViewId>) -> Result<(), ModelError> {
        if !self.levels.contains_key(&v.level) {
            return Err(ModelError::UnknownLevel(v.level));
        }
        if v.name.trim().is_empty() {
            return Err(ModelError::EmptyName);
        }
        if !(v.scale.is_finite() && v.scale > 0.0) {
            return Err(ModelError::NonPositive("scale"));
        }
        if !v.cut_plane.is_finite() {
            return Err(ModelError::NonPositive("cut plane"));
        }
        if self
            .plan_views
            .values()
            .any(|o| Some(o.id) != except && o.name == v.name)
        {
            return Err(ModelError::DuplicateName(v.name.clone()));
        }
        Ok(())
    }

    /// Saves a plan view (its id is given here).
    pub fn add_plan_view(&mut self, mut v: PlanView) -> Result<PlanViewId, ModelError> {
        self.check_plan_view(&v, None)?;
        v.id = PlanViewId(self.take_id());
        let id = v.id;
        self.plan_views.insert(id, v);
        Ok(id)
    }

    /// Replaces a saved plan (keeps its id).
    pub fn set_plan_view(&mut self, v: PlanView) -> Result<(), ModelError> {
        if !self.plan_views.contains_key(&v.id) {
            return Err(ModelError::UnknownPlanView(v.id));
        }
        self.check_plan_view(&v, Some(v.id))?;
        self.plan_views.insert(v.id, v);
        Ok(())
    }

    pub fn remove_plan_view(&mut self, id: PlanViewId) -> Result<PlanView, ModelError> {
        self.plan_views
            .remove(&id)
            .ok_or(ModelError::UnknownPlanView(id))
    }

    /// A name for a new saved plan of a level not yet taken: «Planta baja
    /// — copia», «Planta baja — copia 2»…
    pub fn free_view_name(&self, base: &str) -> String {
        let taken = |n: &str| self.plan_views.values().any(|v| v.name == n);
        if !taken(base) {
            return base.to_string();
        }
        (2..)
            .map(|i| format!("{base} {i}"))
            .find(|n| !taken(n))
            .unwrap_or_default()
    }

    // ---- Styles of the levels' own plans ------------------------------------

    /// What a level's own plan shows and how (the default look if never
    /// changed).
    pub fn level_style(&self, level: LevelId) -> ViewStyle {
        self.level_styles.get(&level).cloned().unwrap_or_default()
    }

    pub fn set_level_style(&mut self, level: LevelId, style: ViewStyle) -> Result<(), ModelError> {
        if !self.levels.contains_key(&level) {
            return Err(ModelError::UnknownLevel(level));
        }
        if style == ViewStyle::default() {
            self.level_styles.remove(&level);
        } else {
            self.level_styles.insert(level, style);
        }
        Ok(())
    }

    // ---- View templates -----------------------------------------------------

    pub fn view_templates(&self) -> impl Iterator<Item = &ViewTemplate> {
        self.view_templates.values()
    }

    pub fn view_template(&self, id: ViewTemplateId) -> Option<&ViewTemplate> {
        self.view_templates.get(&id)
    }

    fn check_template(
        &self,
        t: &ViewTemplate,
        except: Option<ViewTemplateId>,
    ) -> Result<(), ModelError> {
        if t.name.trim().is_empty() {
            return Err(ModelError::EmptyName);
        }
        if self
            .view_templates
            .values()
            .any(|o| Some(o.id) != except && o.name == t.name)
        {
            return Err(ModelError::DuplicateName(t.name.clone()));
        }
        Ok(())
    }

    pub fn add_view_template(&mut self, mut t: ViewTemplate) -> Result<ViewTemplateId, ModelError> {
        self.check_template(&t, None)?;
        t.id = ViewTemplateId(self.take_id());
        let id = t.id;
        self.view_templates.insert(id, t);
        Ok(id)
    }

    pub fn set_view_template(&mut self, t: ViewTemplate) -> Result<(), ModelError> {
        if !self.view_templates.contains_key(&t.id) {
            return Err(ModelError::UnknownViewTemplate(t.id));
        }
        self.check_template(&t, Some(t.id))?;
        self.view_templates.insert(t.id, t);
        Ok(())
    }

    pub fn remove_view_template(&mut self, id: ViewTemplateId) -> Result<ViewTemplate, ModelError> {
        self.view_templates
            .remove(&id)
            .ok_or(ModelError::UnknownViewTemplate(id))
    }

    // ---- Reference planes ---------------------------------------------------

    pub fn ref_planes(&self) -> impl Iterator<Item = &RefPlane> {
        self.ref_planes.values()
    }

    pub fn ref_plane(&self, id: RefPlaneId) -> Option<&RefPlane> {
        self.ref_planes.get(&id)
    }

    fn check_ref_plane(&self, r: &RefPlane) -> Result<(), ModelError> {
        if !(r.start.x.is_finite()
            && r.start.y.is_finite()
            && r.end.x.is_finite()
            && r.end.y.is_finite())
            || r.length() < EPSILON
        {
            return Err(ModelError::NonPositive("reference plane length"));
        }
        Ok(())
    }

    /// Adds a reference plane from `start` to `end`; an empty name gets
    /// the next free one («Referencia 1»…).
    pub fn add_ref_plane(
        &mut self,
        name: &str,
        start: Point2,
        end: Point2,
    ) -> Result<RefPlaneId, ModelError> {
        let name = if name.trim().is_empty() {
            let taken = |n: &str| self.ref_planes.values().any(|r| r.name == n);
            (1..)
                .map(|i| format!("Referencia {i}"))
                .find(|n| !taken(n))
                .unwrap_or_default()
        } else {
            name.trim().to_string()
        };
        let mut r = RefPlane {
            id: RefPlaneId(0),
            name,
            start,
            end,
        };
        self.check_ref_plane(&r)?;
        r.id = RefPlaneId(self.take_id());
        let id = r.id;
        self.ref_planes.insert(id, r);
        Ok(id)
    }

    pub fn edit_ref_plane(
        &mut self,
        id: RefPlaneId,
        f: impl FnOnce(&mut RefPlane),
    ) -> Result<(), ModelError> {
        let mut r = self
            .ref_planes
            .get(&id)
            .cloned()
            .ok_or(ModelError::UnknownRefPlane(id))?;
        f(&mut r);
        r.id = id;
        self.check_ref_plane(&r)?;
        self.ref_planes.insert(id, r);
        Ok(())
    }

    pub fn remove_ref_plane(&mut self, id: RefPlaneId) -> Result<RefPlane, ModelError> {
        self.ref_planes
            .remove(&id)
            .ok_or(ModelError::UnknownRefPlane(id))
    }

    /// Takes a presentation style away from every saved view, level plan
    /// and template.
    pub(super) fn forget_presentation(&mut self, id: PresentationStyleId) {
        let clear = |s: &mut ViewStyle| {
            if s.presentation == Some(id) {
                s.presentation = None;
            }
        };
        self.plan_views
            .values_mut()
            .for_each(|v| clear(&mut v.style));
        self.level_styles.values_mut().for_each(clear);
        self.view_templates
            .values_mut()
            .for_each(|t| clear(&mut t.style));
    }

    pub(super) fn restore_views(
        &mut self,
        plan_views: Vec<PlanView>,
        level_styles: Vec<(LevelId, ViewStyle)>,
        templates: Vec<ViewTemplate>,
        ref_planes: Vec<RefPlane>,
        claim: &mut structure::Claim<'_>,
    ) -> Result<(), RestoreError> {
        for v in plan_views {
            claim(v.id.0, None)
                .and_then(|_| self.check_plan_view(&v, None))
                .map_err(|error| RestoreError {
                    element: format!("plan view {}", v.id.0),
                    error,
                })?;
            self.plan_views.insert(v.id, v);
        }
        for (level, style) in level_styles {
            self.set_level_style(level, style)
                .map_err(|error| RestoreError {
                    element: format!("style of level {}", level.0),
                    error,
                })?;
        }
        for t in templates {
            claim(t.id.0, None)
                .and_then(|_| self.check_template(&t, None))
                .map_err(|error| RestoreError {
                    element: format!("view template {}", t.id.0),
                    error,
                })?;
            self.view_templates.insert(t.id, t);
        }
        for r in ref_planes {
            claim(r.id.0, None)
                .and_then(|_| self.check_ref_plane(&r))
                .map_err(|error| RestoreError {
                    element: format!("reference plane {}", r.id.0),
                    error,
                })?;
            self.ref_planes.insert(r.id, r);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graphics::{Category, GraphicOverride};

    #[test]
    fn saved_views_templates_and_reference_planes() {
        let mut m = Model::new("T");
        let ground = m.add_level("Planta baja", 0.0);
        let mut style = ViewStyle::default();
        style.hidden.insert(Category::Objects);
        let v = m
            .add_plan_view(PlanView {
                id: PlanViewId(0),
                name: "Planta baja — albañilería".into(),
                level: ground,
                kind: PlanKind::Floor,
                scale: 50.0,
                cut_plane: 1.1,
                style: style.clone(),
            })
            .unwrap();
        // Names are unique.
        let mut dup = m.plan_view(v).unwrap().clone();
        assert!(m.add_plan_view(dup.clone()).is_err());
        assert_eq!(
            m.free_view_name("Planta baja — albañilería"),
            "Planta baja — albañilería 2"
        );
        dup.kind = PlanKind::Ceiling;
        m.set_plan_view(dup).unwrap();
        assert_eq!(m.plan_views_of(ground)[0].kind, PlanKind::Ceiling);
        // A level's own plan keeps its look.
        m.set_level_style(ground, ViewStyle::ad5()).unwrap();
        assert_eq!(
            m.level_style(ground).graphics(Category::Walls).layer_lines,
            crate::graphics::LayerLines::Cavity
        );
        let t = m
            .add_view_template(ViewTemplate {
                id: ViewTemplateId(0),
                name: "AD5".into(),
                style: ViewStyle::ad5(),
            })
            .unwrap();
        assert!(m.view_template(t).is_some());
        let r = m
            .add_ref_plane("", Point2::new(0.0, 0.0), Point2::new(0.0, 10.0))
            .unwrap();
        assert_eq!(m.ref_plane(r).unwrap().name, "Referencia 1");
        assert!(m
            .add_ref_plane("x", Point2::new(1.0, 1.0), Point2::new(1.0, 1.0))
            .is_err());
        m.edit_ref_plane(r, |p| p.end = Point2::new(0.0, 5.0))
            .unwrap();
        assert!((m.ref_plane(r).unwrap().length() - 5.0).abs() < 1e-12);
        // Removing the level takes its views with it.
        let first = m.add_level("Planta 1", 3.0);
        m.remove_level(ground).unwrap();
        assert_eq!(m.plan_views().count(), 0);
        assert_eq!(m.level_style(first), ViewStyle::default());
        // An override at its default changes nothing and is not kept.
        let mut s = ViewStyle::default();
        s.set_graphics(Category::Slabs, GraphicOverride::default());
        assert!(s.overrides.is_empty());
    }
}
