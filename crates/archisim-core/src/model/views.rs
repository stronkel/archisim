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
/// and how each category is drawn, its detail, presentation style and
/// discipline. The views that follow it (`ViewStyle::template`) take its
/// look whenever it changes.
#[derive(Debug, Clone, PartialEq)]
pub struct ViewTemplate {
    pub id: ViewTemplateId,
    pub name: String,
    pub style: ViewStyle,
}

impl ViewTemplate {
    /// The look of a view that follows it: the template's, with what
    /// belongs to the session (the discipline greyed and the disciplines
    /// hidden in the navigator) kept from `old`.
    pub fn applied(&self, old: &ViewStyle) -> ViewStyle {
        let mut s = self.style.clone();
        s.template = Some(self.id);
        s.focus = old.focus;
        s.hidden_disciplines = old.hidden_disciplines.clone();
        s
    }
}

/// A saved point of view of the 3D view (the Cámara tile of
/// Presentación): where it looks, from how far, how tilted and turned,
/// and whether in perspective; a walk at eye height is a camera close to
/// the point it looks at.
#[derive(Debug, Clone, PartialEq)]
pub struct SavedCamera {
    pub id: CameraId,
    pub name: String,
    /// The point looked at, metres.
    pub target: [f64; 3],
    /// Tilt over the horizontal and turn, radians (the 3D view's).
    pub pitch: f64,
    pub yaw: f64,
    /// From the eye to the point looked at, metres.
    pub distance: f64,
    pub perspective: bool,
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

    /// What a level's own plan shows and how: its own look, or else the
    /// plans' template's ([`Model::plan_template`]), or the default look.
    pub fn level_style(&self, level: LevelId) -> ViewStyle {
        self.level_styles
            .get(&level)
            .cloned()
            .unwrap_or_else(|| self.plain_level_style())
    }

    /// The look of a level's plan never changed.
    fn plain_level_style(&self) -> ViewStyle {
        self.plan_template
            .and_then(|t| self.view_templates.get(&t))
            .map_or_else(ViewStyle::default, |t| t.applied(&ViewStyle::default()))
    }

    pub fn set_level_style(&mut self, level: LevelId, style: ViewStyle) -> Result<(), ModelError> {
        if !self.levels.contains_key(&level) {
            return Err(ModelError::UnknownLevel(level));
        }
        if style == self.plain_level_style() {
            self.level_styles.remove(&level);
        } else {
            self.level_styles.insert(level, style);
        }
        Ok(())
    }

    /// The template the levels' plans follow unless given their own look
    /// (a new project's: «Arquitectura», without the installations).
    pub fn plan_template(&self) -> Option<ViewTemplateId> {
        self.plan_template
    }

    /// Sets the plans' template. The levels that were drawn as the old one
    /// keep that look as their own only if it differs.
    pub fn set_plan_template(&mut self, t: Option<ViewTemplateId>) -> Result<(), ModelError> {
        if let Some(id) = t {
            if !self.view_templates.contains_key(&id) {
                return Err(ModelError::UnknownViewTemplate(id));
            }
        }
        self.plan_template = t;
        // A level whose own look is now the plain one keeps nothing.
        let plain = self.plain_level_style();
        self.level_styles.retain(|_, s| *s != plain);
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
        let mut t = t;
        t.style.template = None;
        self.restyle_followers(t.id, |old| t.applied(old));
        self.view_templates.insert(t.id, t);
        Ok(())
    }

    /// Every view that follows template `id`: its look as `f` makes it.
    fn restyle_followers(&mut self, id: ViewTemplateId, f: impl Fn(&ViewStyle) -> ViewStyle) {
        let follows = |s: &ViewStyle| s.template == Some(id);
        for v in self.plan_views.values_mut() {
            if follows(&v.style) {
                v.style = f(&v.style);
            }
        }
        for s in self.level_styles.values_mut() {
            if follows(s) {
                *s = f(s);
            }
        }
        for v in self.viewports.values_mut() {
            if follows(&v.style) {
                v.style = f(&v.style);
            }
        }
    }

    /// How many views follow template `id` (plans, saved views and views
    /// placed on sheets).
    pub fn template_followers(&self, id: ViewTemplateId) -> usize {
        let follows = |s: &ViewStyle| s.template == Some(id);
        self.plan_views
            .values()
            .filter(|v| follows(&v.style))
            .count()
            + self.level_styles.values().filter(|s| follows(s)).count()
            + self
                .viewports
                .values()
                .filter(|v| follows(&v.style))
                .count()
    }

    /// Removes a template; the views that followed it keep its look as
    /// their own.
    pub fn remove_view_template(&mut self, id: ViewTemplateId) -> Result<ViewTemplate, ModelError> {
        // The plans that followed it as the plans' template keep its look.
        if self.plan_template == Some(id) {
            let plain = self.plain_level_style();
            let levels: Vec<LevelId> = self.levels.keys().copied().collect();
            for l in levels {
                self.level_styles.entry(l).or_insert_with(|| plain.clone());
            }
            self.plan_template = None;
        }
        let t = self
            .view_templates
            .remove(&id)
            .ok_or(ModelError::UnknownViewTemplate(id))?;
        self.restyle_followers(id, |old| {
            let mut s = old.clone();
            s.template = None;
            s
        });
        Ok(t)
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

    // ---- Saved cameras --------------------------------------------------------

    pub fn cameras(&self) -> impl Iterator<Item = &SavedCamera> {
        self.cameras.values()
    }

    pub fn camera(&self, id: CameraId) -> Option<&SavedCamera> {
        self.cameras.get(&id)
    }

    fn check_camera(&self, c: &SavedCamera) -> Result<(), ModelError> {
        let finite = c.target.iter().all(|v| v.is_finite())
            && c.pitch.is_finite()
            && c.yaw.is_finite()
            && c.distance.is_finite();
        if !finite {
            return Err(ModelError::NotFinite("camera"));
        }
        if c.distance <= 0.0 {
            return Err(ModelError::NonPositive("camera distance"));
        }
        if c.name.trim().is_empty() {
            return Err(ModelError::EmptyName);
        }
        Ok(())
    }

    /// Saves a point of view; an empty name gets the next free one
    /// («Cámara 1»…).
    pub fn add_camera(&mut self, mut c: SavedCamera) -> Result<CameraId, ModelError> {
        if c.name.trim().is_empty() {
            let taken = |n: &str| self.cameras.values().any(|k| k.name == n);
            c.name = (1..)
                .map(|i| format!("Cámara {i}"))
                .find(|n| !taken(n))
                .unwrap_or_default();
        }
        c.name = c.name.trim().to_string();
        c.id = CameraId(0);
        self.check_camera(&c)?;
        c.id = CameraId(self.take_id());
        let id = c.id;
        self.cameras.insert(id, c);
        Ok(id)
    }

    pub fn edit_camera(
        &mut self,
        id: CameraId,
        f: impl FnOnce(&mut SavedCamera),
    ) -> Result<(), ModelError> {
        let mut c = self
            .cameras
            .get(&id)
            .cloned()
            .ok_or(ModelError::UnknownCamera(id))?;
        f(&mut c);
        c.id = id;
        self.check_camera(&c)?;
        self.cameras.insert(id, c);
        Ok(())
    }

    pub fn remove_camera(&mut self, id: CameraId) -> Result<SavedCamera, ModelError> {
        self.cameras
            .remove(&id)
            .ok_or(ModelError::UnknownCamera(id))
    }

    pub(super) fn restore_cameras(
        &mut self,
        cameras: Vec<SavedCamera>,
        claim: &mut structure::Claim<'_>,
    ) -> Result<(), RestoreError> {
        for c in cameras {
            claim(c.id.0, None)
                .and_then(|_| self.check_camera(&c))
                .map_err(|error| RestoreError {
                    element: format!("camera {}", c.id.0),
                    error,
                })?;
            self.cameras.insert(c.id, c);
        }
        Ok(())
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
        self.sections.values_mut().for_each(|x| clear(&mut x.style));
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
        // As saved: a level's own look stays its own even when it is the
        // plain one (the plans' template comes after).
        for (level, style) in level_styles {
            if !self.levels.contains_key(&level) {
                return Err(RestoreError {
                    element: format!("style of level {}", level.0),
                    error: ModelError::UnknownLevel(level),
                });
            }
            self.level_styles.insert(level, style);
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

    /// The views that follow a template take its look whenever it
    /// changes; removing it leaves them their look as their own; the file
    /// keeps who follows what.
    #[test]
    fn views_follow_their_template() {
        let mut m = Model::new("T");
        let ground = m.add_level("Planta baja", 0.0);
        let first = m.add_level("Planta 1", 3.0);
        let mut look = ViewStyle::ad5();
        look.detail = crate::graphics::Detail::Medium;
        let t = m
            .add_view_template(ViewTemplate {
                id: ViewTemplateId(0),
                name: "Albañilería".into(),
                style: look.clone(),
            })
            .unwrap();
        let template = m.view_template(t).unwrap().clone();
        let own = ViewStyle {
            focus: crate::DisciplineFocus::Only(crate::ElementDiscipline::Structure),
            ..Default::default()
        };
        m.set_level_style(ground, template.applied(&own)).unwrap();
        let v = m
            .add_plan_view(PlanView {
                id: PlanViewId(0),
                name: "Planta 1 — albañilería".into(),
                level: first,
                kind: PlanKind::Floor,
                scale: 50.0,
                cut_plane: 1.1,
                style: template.applied(&ViewStyle::default()),
            })
            .unwrap();
        assert_eq!(m.template_followers(t), 2);
        // The template changes: both views with it, each keeping what is
        // of the session.
        let mut changed = template.clone();
        changed.style.detail = crate::graphics::Detail::Coarse;
        changed.style.hidden.insert(Category::Objects);
        m.set_view_template(changed).unwrap();
        for s in [m.level_style(ground), m.plan_view(v).unwrap().style.clone()] {
            assert_eq!(s.detail, crate::graphics::Detail::Coarse);
            assert!(!s.shows(Category::Objects));
            assert_eq!(s.template, Some(t));
        }
        assert_eq!(m.level_style(ground).focus, own.focus);
        assert_eq!(m.view_template(t).unwrap().style.template, None);
        // Kept in the file.
        let back = Model::from_parts(m.to_parts()).unwrap();
        assert_eq!(back.level_style(ground).template, Some(t));
        // Removed: the views keep the look, following nothing.
        m.remove_view_template(t).unwrap();
        let s = m.level_style(ground);
        assert_eq!(s.template, None);
        assert_eq!(s.detail, crate::graphics::Detail::Coarse);
        assert_eq!(m.template_followers(t), 0);
    }

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
        // A saved camera, named by itself.
        let cam = SavedCamera {
            id: CameraId(0),
            name: String::new(),
            target: [1.0, 2.0, 0.0],
            pitch: 0.5,
            yaw: -1.0,
            distance: 20.0,
            perspective: true,
        };
        let c = m.add_camera(cam.clone()).unwrap();
        assert_eq!(m.camera(c).unwrap().name, "Cámara 1");
        assert!(m
            .add_camera(SavedCamera {
                distance: 0.0,
                ..cam.clone()
            })
            .is_err());
        m.edit_camera(c, |k| k.name = "Fachada sur".into()).unwrap();
        assert_eq!(m.camera(c).unwrap().name, "Fachada sur");
        let back = Model::from_parts(m.to_parts()).unwrap();
        assert_eq!(back.camera(c), m.camera(c));
        m.remove_camera(c).unwrap();
        assert!(m.camera(c).is_none());
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
                name: "Blanco y negro".into(),
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

#[cfg(test)]
mod plan_template_tests {
    use super::*;

    #[test]
    fn the_plans_follow_their_template() {
        let mut m = Model::new("Plantas");
        let a = m.add_level("Baja", 0.0);
        let b = m.add_level("Primera", 3.0);
        let arq = m
            .add_view_template(ViewTemplate {
                id: ViewTemplateId(0),
                name: "Arquitectura".into(),
                style: ViewStyle {
                    discipline: Some(crate::ElementDiscipline::Architecture),
                    ..Default::default()
                },
            })
            .unwrap();
        assert_eq!(m.level_style(a).template, None);
        m.set_plan_template(Some(arq)).unwrap();
        // Both levels follow it without a look of their own.
        assert_eq!(m.level_style(a).template, Some(arq));
        assert!(m.level_styles.is_empty());
        // One level shown otherwise keeps it; set back, it follows again.
        let own = ViewStyle::default();
        m.set_level_style(b, own.clone()).unwrap();
        assert_eq!(m.level_style(b), own);
        let plain = m.level_style(a);
        m.set_level_style(b, plain).unwrap();
        assert!(m.level_styles.is_empty());
        // Removing the template leaves the plans with its look.
        m.remove_view_template(arq).unwrap();
        assert_eq!(m.plan_template(), None);
        assert_eq!(
            m.level_style(a).discipline,
            Some(crate::ElementDiscipline::Architecture)
        );
        assert_eq!(m.level_style(a).template, None);
    }
}
