//! What the tool in use would build, seen in the 3D view before the click
//! (R21): a wall with its layers and joins, a door or window in its wall,
//! a column, a beam, a slab or an object, drawn see-through in its own
//! colours. It is built on a copy of the model with the same functions the
//! click uses, so it shows exactly what the click will make; the copy is
//! only rebuilt when the cursor or the model change.
//!
//! The plan does the same with its real drawing (bloque 3 de la revisión
//! del 22/10/2026): the region of the model copy that the click or the
//! drag changes is drawn by `archisim-plan`, as it will look, over the
//! plan — an object with its symbol, a door with its swing in its cut
//! wall, a wall with its layers and joins, what is moved where it goes.

use std::cell::RefCell;

use archisim_core::{Model, NewBeam, NewColumn, NewObject, NewSlab, NewWall, Point2};
use archisim_geom::solid::{self, Solid};
use eframe::egui::{self, Color32, Pos2, Rect};

use super::*;

/// Triangles of the ghost and their colours, by what they were built from.
#[derive(Default)]
pub struct GhostCache {
    cell: RefCell<Option<(String, Vec<Solid>)>>,
}

/// What a click would build.
#[derive(Debug, Clone)]
enum Would {
    Walls(Vec<NewWall>),
    Openings(Vec<archisim_core::NewOpening>),
    Column(NewColumn),
    Beam(NewBeam),
    Slab(NewSlab),
    Object(NewObject),
    World(archisim_core::NewWorldBuilding),
    Road(archisim_core::RoadStretch),
    /// A modular box (M1): its outline and mark in plan, nothing in 3D.
    Box(archisim_core::NewBox),
    Elevator(archisim_core::NewElevator),
    Device(archisim_core::NewDevice),
    Stair(archisim_core::NewStair),
}

impl ArchiSimApp {
    /// What the tool in use would build at the cursor, if anything.
    fn would_build(&self) -> Option<Would> {
        let c = self.cursor?;
        let model = self.ed.model();
        let level = self.ed.level;
        Some(match self.tool {
            Tool::Wall => {
                self.wall_tool.anchor()?;
                let s = &self.wall_settings;
                let top = s.top_on(model, level);
                let pieces = self.wall_tool.preview(c.point);
                if pieces.is_empty() {
                    return None;
                }
                Would::Walls(
                    pieces
                        .iter()
                        .map(|w| {
                            let (start, end, bulge) = s.oriented(w);
                            NewWall {
                                wall_type: s.wall_type,
                                level,
                                start,
                                end,
                                reference: s.reference,
                                height: s.height,
                                base_offset: s.base_offset,
                                bulge,
                                top,
                            }
                        })
                        .collect(),
                )
            }
            Tool::Door | Tool::Window => {
                let pl = self.placement.as_ref()?;
                if pl.problem.is_some() {
                    return None;
                }
                Would::Openings(pl.specs())
            }
            Tool::Column => {
                let s = &self.column_settings;
                Would::Column(NewColumn {
                    member_type: s.member_type,
                    level,
                    position: c.point,
                    rotation: s.rotation,
                    base_offset: s.base_offset,
                    top: s.top_on(model, level),
                    height: s.height,
                })
            }
            Tool::Beam => {
                let a = self.beam_tool.anchor?;
                if a.distance(c.point) < 1e-3 {
                    return None;
                }
                let s = &self.beam_settings;
                Would::Beam(NewBeam {
                    member_type: s.member_type,
                    level: self.ed.beam_level(s),
                    start: a,
                    end: c.point,
                    top_offset: s.top_offset,
                })
            }
            Tool::Slab if !self.slab_settings.hole && !self.slab_settings.ceiling => {
                let outline = if self.slab_tool.mode == crate::slab_tool::SlabMode::Rectangle {
                    self.slab_tool.rectangle_to(c.point)?
                } else {
                    let mut pts: Vec<Point2> = self.slab_tool.corners().to_vec();
                    pts.push(self.slab_tool.typed_end(c.point).unwrap_or(c.point));
                    pts
                };
                if outline.len() < 3 || archisim_core::signed_area(&outline).abs() < 1e-4 {
                    return None;
                }
                Would::Slab(NewSlab {
                    slab_type: self.slab_settings.slab_type,
                    level,
                    outline,
                    top_offset: self.slab_settings.top_offset,
                })
            }
            Tool::Object => {
                let (position, rotation, base_offset) = self.placement_spot(c.point);
                Would::Object(NewObject {
                    level,
                    object_type: self.object_settings.object_type?,
                    position,
                    rotation,
                    base_offset,
                })
            }
            Tool::World => Would::World(self.world_would(c.point)?),
            Tool::Elevator => Would::Elevator(self.elevator_spec(c.point)),
            Tool::Device => Would::Device(self.device_spec(c.point)),
            // A ready-made stair before its first click: at the cursor,
            // going right (after it, `draw_stair_preview` shows it).
            Tool::Stair
                if self.stair_settings.preset != archisim_core::StairPreset::Free
                    && self.stair_tool.points.is_empty() =>
            {
                Would::Stair(self.ed.preset_stair_spec(
                    &self.stair_settings,
                    c.point,
                    archisim_core::Vec2::new(1.0, 0.0),
                )?)
            }
            Tool::ModularBox => Would::Box(super::box_ui::box_at(
                model,
                level,
                self.box_settings.as_ref()?,
                c.point,
            )),
            Tool::Road if self.road_settings.draw.by_stretches() => {
                if self.road_settings.draw == super::urban_ui::RoadDraw::Curve
                    && self.road_points.len() < 2
                {
                    return None;
                }
                let (p, _) = self.road_snap(c.point);
                let s = &self.road_settings;
                Would::Road(archisim_core::RoadStretch {
                    level,
                    points: self.road_stretch_points(p)?,
                    continues: self.road_chain,
                    section: s.section.clone(),
                    top_offset: s.top_offset,
                    pavement: s.pavement,
                    kerb_material: s.kerb,
                    sidewalk_material: s.sidewalk,
                    follow_ground: s.follow_ground.then_some(s.max_grade),
                })
            }
            _ => return None,
        })
    }

    /// The solids of what the tool would build, on a copy of the model.
    fn ghost_solids(&self) -> Option<std::cell::Ref<'_, Vec<Solid>>> {
        let would = self.would_build()?;
        let key = format!("{}:{would:?}", self.ed.revision());
        let fresh = self
            .ghost
            .cell
            .borrow()
            .as_ref()
            .is_some_and(|(k, _)| *k == key);
        if !fresh {
            let solids = build_ghost(self.ed.model(), would).unwrap_or_default();
            *self.ghost.cell.borrow_mut() = Some((key, solids));
        }
        let r = self.ghost.cell.borrow();
        std::cell::Ref::filter_map(r, |c| c.as_ref().map(|(_, s)| s)).ok()
    }

    /// Draws what the tool would build, see-through, over the 3D view.
    pub(super) fn draw_ghost_3d(&self, painter: &egui::Painter, rect: Rect) {
        // A street is drawn as it will be, in the scene (`road_overlay_3d`).
        if matches!(self.would_build(), Some(Would::Road(_))) {
            return;
        }
        let Some(solids) = self.ghost_solids() else {
            return;
        };
        let camera = &self.v3d.camera;
        let size = glam::Vec2::new(rect.width(), rect.height());
        let eye = camera.eye();
        let light = glam::Vec3::new(0.35, -0.5, 0.8).normalize();
        let model = self.ed.model();
        // Far triangles first, so the near ones are drawn over them.
        let mut tris: Vec<(f32, [Pos2; 3], Color32)> = Vec::new();
        for s in solids.iter() {
            let base = solid::solid_color(model, s);
            for t in &s.triangles {
                let p = t.map(|q| glam::Vec3::new(q[0] as f32, q[1] as f32, q[2] as f32));
                let n = (p[1] - p[0]).cross(p[2] - p[0]).normalize_or_zero();
                let centre = (p[0] + p[1] + p[2]) / 3.0;
                // Only the faces turned to the eye: a shell, not a tangle.
                if n.dot(eye - centre) <= 0.0 && camera.perspective {
                    continue;
                }
                if !camera.perspective && n.dot(-camera.forward()) <= 0.0 {
                    continue;
                }
                let Some(screen) = p
                    .iter()
                    .map(|q| {
                        camera
                            .project_to_screen(*q, size)
                            .map(|s| Pos2::new(rect.min.x + s.x, rect.min.y + s.y))
                    })
                    .collect::<Option<Vec<_>>>()
                else {
                    continue;
                };
                let shade = 0.65 + 0.35 * n.dot(light).max(0.0);
                let [r, g, b] = base.map(|c| (c as f32 * shade).min(255.0) as u8);
                tris.push((
                    centre.distance(eye),
                    [screen[0], screen[1], screen[2]],
                    Color32::from_rgba_unmultiplied(r, g, b, 150),
                ));
            }
        }
        tris.sort_by(|a, b| b.0.total_cmp(&a.0));
        let mut mesh = egui::Mesh::default();
        for (_, [a, b, c], colour) in tris {
            let i = mesh.vertices.len() as u32;
            for p in [a, b, c] {
                mesh.colored_vertex(p, colour);
            }
            mesh.add_triangle(i, i + 1, i + 2);
        }
        painter.add(egui::Shape::mesh(mesh));
    }
}

/// Builds `would` on a copy of `model` and returns the solids it makes
/// (a door or window: its own, without the wall).
fn build_ghost(model: &Model, would: Would) -> Option<Vec<Solid>> {
    let mut m = model.clone();
    Some(match would {
        Would::Walls(walls) => {
            let ids: Vec<_> = walls
                .into_iter()
                .map(|w| m.add_wall(w))
                .collect::<Result<_, _>>()
                .ok()?;
            ids.iter()
                .flat_map(|id| solid::wall_solids(&m, *id))
                .collect()
        }
        Would::Openings(specs) => {
            let mut v = Vec::new();
            for spec in specs {
                let id = m.add_opening(spec).ok()?;
                v.push(id);
            }
            v.iter()
                .flat_map(|id| solid::opening_solids(&m, *id))
                .collect()
        }
        Would::Column(spec) => {
            let id = m.add_column(spec).ok()?;
            solid::column_solids(&m, id)
        }
        Would::Beam(spec) => {
            let id = m.add_beam(spec).ok()?;
            solid::beam_solids(&m, id)
        }
        Would::Slab(spec) => {
            let id = m.add_slab(spec).ok()?;
            solid::slab_solids(&m, id)
        }
        Would::Object(spec) => {
            let id = m.add_object(spec).ok()?;
            solid::object_solids(&m, id)
        }
        Would::World(spec) => {
            // Its volume only: dressing it at each step of the cursor would
            // be slow.
            let id = m.add_world_building(spec).ok()?;
            m.set_mass_dress(id, None).ok()?;
            solid::mass_solids(&m, id)
        }
        Would::Box(_) => Vec::new(),
        Would::Elevator(spec) => {
            let id = m.add_elevator(spec).ok()?;
            archisim_geom::elevators::elevator_solids(&m, id)
        }
        Would::Device(spec) => {
            let id = m.add_device(spec).ok()?;
            solid::device_solids(&m, id)
        }
        Would::Stair(spec) => {
            let id = m.add_stair(spec).ok()?;
            archisim_geom::stairs::stair_solids(&m, id)
        }
        Would::Road(spec) => {
            let id = stretch_fused(&mut m, spec)?;
            solid::road_solids(&m, id)
        }
    })
}

/// Above this many walls, the plan shows only the outline of what a
/// click would place.
const GHOST_MAX_WALLS: usize = 3000;

/// The plan's ghost: the copy of the model after the click or drag, the
/// region it changes and its picture at the current zoom.
#[derive(Default)]
pub struct PlanGhost {
    /// What the copy was built for.
    what: String,
    after: Option<(Model, (Point2, Point2))>,
    /// What the picture was drawn for, and the picture with its region.
    raster: String,
    image: Option<(egui::TextureHandle, (Point2, Point2))>,
    /// A crossing the new street would make treads on another one.
    crowded: std::cell::Cell<bool>,
    /// The outlines of what a click would place, tinted over the picture
    /// so it reads as not there yet.
    made: std::cell::RefCell<Vec<Vec<Point2>>>,
}

impl ArchiSimApp {
    /// What the plan's ghost shows now, as a key, or nothing.
    fn plan_ghost_what(&self) -> Option<String> {
        if let Some(d) = &self.drag {
            let (to, variant, _) = self.drag_preview.as_ref()?;
            return Some(format!("drag {:?} {:?} {:?} {variant}", d.grip, d.from, to));
        }
        self.would_build().map(|w| format!("{w:?}"))
    }

    /// Builds the copy of the model the ghost draws, and its region.
    fn plan_ghost_build(&self) -> Option<(Model, (Point2, Point2))> {
        let shown = self.ed.shown();
        self.plan_ghost.made.borrow_mut().clear();
        if let Some(d) = &self.drag {
            let (to, variant, _) = self.drag_preview.as_ref()?;
            let drag = crate::drag::Drag {
                grip: d.grip,
                from: d.from,
                to: *to,
                variant: *variant,
            };
            let (after, level, region) =
                crate::drag::ghost(&self.ed, &shown, &self.ed.selection, &drag)?;
            if level.is_some_and(|l| l != self.ed.level) {
                return None;
            }
            return Some((after, region));
        }
        let would = self.would_build()?;
        // With very big models, copying the model at each step of the
        // cursor would slow it down: the outline is enough there.
        if shown.walls().count() > GHOST_MAX_WALLS {
            return None;
        }
        self.plan_ghost_crowded_set(false);
        let (after, made) = build_plan_ghost(&shown, would)?;
        self.plan_ghost_crowded_set(crowded(&after, &made));
        let geo = crate::geo::Indexed::new(&after);
        let outlines: Vec<Vec<Point2>> = made
            .iter()
            .filter(|e| !matches!(e, Element::Road(_)))
            .filter_map(|e| crate::pick::outline(&geo, *e))
            .collect();
        *self.plan_ghost.made.borrow_mut() = outlines;
        let mut pts: Vec<Point2> = made
            .iter()
            .filter_map(|e| crate::pick::outline(&geo, *e))
            .flatten()
            .collect();
        if pts.is_empty() {
            return None;
        }
        let first = pts.remove(0);
        let (lo, hi) = pts.iter().fold((first, first), |(lo, hi), p| {
            (
                Point2::new(lo.x.min(p.x), lo.y.min(p.y)),
                Point2::new(hi.x.max(p.x), hi.y.max(p.y)),
            )
        });
        // Walls it joins lose or gain their mitres nearby; a street changes
        // the crossings it makes and the streets it meets round them.
        let reach = if made.iter().any(|e| matches!(e, Element::Road(_))) {
            25.0
        } else {
            0.8
        };
        let margin = Point2::new(reach, reach);
        Some((after, (lo - margin, hi + margin)))
    }

    /// Draws the ghost over the plan: the region it changes, as it will
    /// be drawn.
    pub(super) fn draw_plan_ghost(
        &mut self,
        ctx: &egui::Context,
        painter: &egui::Painter,
        rect: Rect,
    ) {
        let Some(what) = self.plan_ghost_what() else {
            self.plan_ghost = PlanGhost::default();
            return;
        };
        let what = format!("{what} @{} {:?}", self.ed.revision(), self.ed.level);
        if self.plan_ghost.what != what {
            self.plan_ghost.after = self.plan_ghost_build();
            self.plan_ghost.what = what.clone();
        }
        let Some((after, (lo, hi))) = &self.plan_ghost.after else {
            return;
        };
        // Only what is on screen is drawn.
        let (vlo, vhi) = self.view.visible(rect);
        let lo = Point2::new(lo.x.max(vlo.x), lo.y.max(vlo.y));
        let hi = Point2::new(hi.x.min(vhi.x), hi.y.min(vhi.y));
        if hi.x <= lo.x || hi.y <= lo.y {
            return;
        }
        let screen =
            Rect::from_two_pos(self.view.to_screen(rect, lo), self.view.to_screen(rect, hi));
        let ppp = ctx.pixels_per_point();
        let size = [
            (screen.width() * ppp).round().max(1.0) as usize,
            (screen.height() * ppp).round().max(1.0) as usize,
        ];
        let raster = format!(
            "{what} {lo:?} {hi:?} {size:?} {} {} {} {:?}",
            self.plan_scale, self.cut_plane, self.thin_lines, self.view_style
        );
        if self.plan_ghost.raster != raster {
            let mut options = self.screen_plan_options(lo, hi);
            options.shadows = None;
            let image = crate::plan_image::rasterize_with(after, self.ed.level, size, &options);
            match &mut self.plan_ghost.image {
                Some((tex, region)) => {
                    tex.set(image, egui::TextureOptions::LINEAR);
                    *region = (lo, hi);
                }
                None => {
                    let tex = ctx.load_texture("plan-ghost", image, egui::TextureOptions::LINEAR);
                    self.plan_ghost.image = Some((tex, (lo, hi)));
                }
            }
            self.plan_ghost.raster = raster;
        }
        if let Some((tex, _)) = &self.plan_ghost.image {
            // The drawing under it is hidden (it is drawn again, changed),
            // but not the grid: without it the region was a white box.
            painter.rect_filled(screen, egui::CornerRadius::ZERO, PAPER);
            if self.show_grid {
                self.draw_grid(&painter.with_clip_rect(screen.intersect(rect)), rect);
            }
            painter.image(
                tex.id(),
                screen,
                Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)),
                Color32::WHITE,
            );
            // What is not there yet: a see-through tint over it.
            let tint = Color32::from_rgba_unmultiplied(30, 110, 230, 34);
            let edge = egui::Stroke::new(1.2, Color32::from_rgba_unmultiplied(30, 110, 230, 170));
            for o in self.plan_ghost.made.borrow().iter() {
                let pts: Vec<Pos2> = o.iter().map(|p| self.view.to_screen(rect, *p)).collect();
                if pts.len() >= 3 {
                    // egui fills only convex outlines well: the others,
                    // their edge alone.
                    let n = pts.len();
                    let turns: Vec<f32> = (0..n)
                        .map(|i| {
                            let (a, b, c) = (pts[i], pts[(i + 1) % n], pts[(i + 2) % n]);
                            (b - a).x * (c - b).y - (b - a).y * (c - b).x
                        })
                        .filter(|t| t.abs() > 1e-3)
                        .collect();
                    let convex = turns.iter().all(|t| *t > 0.0) || turns.iter().all(|t| *t < 0.0);
                    painter.add(egui::Shape::Path(egui::epaint::PathShape {
                        points: pts,
                        closed: true,
                        fill: if convex { tint } else { Color32::TRANSPARENT },
                        stroke: edge.into(),
                    }));
                }
            }
        }
    }
}

impl ArchiSimApp {
    /// Whether a crossing of the street being traced treads on another.
    pub(super) fn plan_ghost_crowded(&self) -> bool {
        self.plan_ghost.crowded.get() && self.plan_ghost.after.is_some()
    }

    fn plan_ghost_crowded_set(&self, on: bool) {
        self.plan_ghost.crowded.set(on);
    }

    /// Whether the plan shows the real ghost now (the tool's own sketch
    /// over it would hide it).
    pub(super) fn plan_ghost_shown(&self) -> bool {
        self.plan_ghost.after.is_some() && self.plan_ghost.image.is_some()
    }

    /// While a street is traced in 3D, the street as it will be, with its
    /// materials, in the scene: it and the streets it meets, their
    /// crossings made (as in a city builder, the junction forms as the
    /// cursor goes).
    pub(super) fn road_overlay_3d(&mut self) {
        let would = match self.would_build() {
            Some(Would::Road(spec)) => spec,
            _ => {
                if self.road_overlaid {
                    self.v3d.set_overlay(None, || None);
                    self.road_overlaid = false;
                }
                return;
            }
        };
        let key = format!("calle {} {would:?}", self.ed.revision());
        let model = self.ed.model();
        self.v3d.set_overlay(Some(key), || {
            let mut m = model.clone();
            let id = stretch_fused(&mut m, would)?;
            let (new, met): (Vec<Solid>, Vec<Solid>) = road_with_met(&m, id)
                .into_iter()
                .partition(|s| s.owner == solid::SolidOwner::Road(id));
            // The new street marked (outlined), the ones it meets as they
            // will be, with their crossings.
            Some(
                crate::view3d::scene::Scene::of_solids(&m, new)
                    .joined(crate::view3d::scene::Scene::of_solids_unmarked(&m, met)),
            )
        });
        self.road_overlaid = true;
    }
}

/// Whether a crossing of what was made is on top of another crossing:
/// their middles closer than the half widths of the streets there.
fn crowded(m: &Model, made: &[Element]) -> bool {
    let roads: Vec<archisim_core::RoadId> = made
        .iter()
        .filter_map(|e| match e {
            Element::Road(r) => Some(*r),
            _ => None,
        })
        .collect();
    archisim_geom::road_shaping::crowded(m, &roads)
}

/// A stretch of street laid on a copy of the model as the tool lays it:
/// with the crossings it makes too near another fused into that one.
fn stretch_fused(m: &mut Model, spec: archisim_core::RoadStretch) -> Option<archisim_core::RoadId> {
    let id = m.add_road_stretch(spec).ok()?;
    archisim_geom::road_shaping::fuse_crowded(m, id).ok()?;
    Some(id)
}

/// The solids of a new street and of the streets it meets, with their
/// crossings, a little over the ground (over the streets as they are).
pub(super) fn road_with_met(m: &Model, id: archisim_core::RoadId) -> Vec<Solid> {
    let Some(level) = m.road(id).map(|r| r.level) else {
        return Vec::new();
    };
    let junctions = archisim_geom::crossroads::junctions(m, level);
    let mut roads = vec![id];
    for j in &junctions {
        if j.arms.iter().any(|a| a.road == id) {
            for a in &j.arms {
                if !roads.contains(&a.road) {
                    roads.push(a.road);
                }
            }
        }
    }
    let mut solids: Vec<Solid> = roads
        .iter()
        .flat_map(|r| solid::road_solids_with(m, *r, &junctions))
        .collect();
    for s in &mut solids {
        for t in &mut s.triangles {
            for p in t.iter_mut() {
                p[2] += 0.03;
            }
        }
    }
    solids
}

/// Builds `would` on a copy of `model`: the copy and what it added.
fn build_plan_ghost(model: &Model, would: Would) -> Option<(Model, Vec<Element>)> {
    let mut m = model.clone();
    let made = match would {
        Would::Walls(walls) => walls
            .into_iter()
            .map(|w| m.add_wall(w).map(Element::Wall))
            .collect::<Result<Vec<_>, _>>()
            .ok()?,
        Would::Openings(specs) => {
            let mut made = Vec::new();
            for spec in specs {
                let wall = spec.wall;
                made.push(Element::Opening(m.add_opening(spec).ok()?));
                if !made.contains(&Element::Wall(wall)) {
                    made.push(Element::Wall(wall));
                }
            }
            made
        }
        Would::Column(spec) => vec![Element::Column(m.add_column(spec).ok()?)],
        Would::Beam(spec) => vec![Element::Beam(m.add_beam(spec).ok()?)],
        Would::Slab(spec) => vec![Element::Slab(m.add_slab(spec).ok()?)],
        Would::Object(spec) => vec![Element::Object(m.add_object(spec).ok()?)],
        Would::World(spec) => vec![Element::Mass(m.add_world_building(spec).ok()?)],
        Would::Road(spec) => vec![Element::Road(stretch_fused(&mut m, spec)?)],
        Would::Box(spec) => vec![Element::Box(m.add_box(spec).ok()?)],
        Would::Elevator(spec) => vec![Element::Elevator(m.add_elevator(spec).ok()?)],
        Would::Device(spec) => vec![Element::Device(m.add_device(spec).ok()?)],
        Would::Stair(spec) => vec![Element::Stair(m.add_stair(spec).ok()?)],
    };
    Some((m, made))
}

#[cfg(test)]
pub(super) fn plan_ghost_region(app: &ArchiSimApp) -> Option<(Point2, Point2)> {
    app.plan_ghost.after.as_ref().map(|(_, r)| *r)
}

#[cfg(test)]
pub(super) fn ghost_triangles(app: &ArchiSimApp) -> usize {
    app.ghost_solids()
        .map_or(0, |s| s.iter().map(|s| s.triangles.len()).sum())
}

#[cfg(test)]
pub(super) fn plan_ghost_drawn(app: &ArchiSimApp) -> bool {
    app.plan_ghost.image.is_some()
}
