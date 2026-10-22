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
    Opening(archisim_core::NewOpening),
    Column(NewColumn),
    Beam(NewBeam),
    Slab(NewSlab),
    Object(NewObject),
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
                        .map(|w| NewWall {
                            wall_type: s.wall_type,
                            level,
                            start: w.start,
                            end: w.end,
                            reference: s.reference,
                            height: s.height,
                            base_offset: s.base_offset,
                            bulge: w.bulge,
                            top,
                        })
                        .collect(),
                )
            }
            Tool::Door | Tool::Window => {
                let pl = self.placement.as_ref()?;
                if pl.problem.is_some() {
                    return None;
                }
                Would::Opening(pl.spec.clone())
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
            Tool::Slab if !self.slab_settings.hole => {
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
            Tool::Object => Would::Object(NewObject {
                level,
                object_type: self.object_settings.object_type?,
                position: c.point,
                rotation: self.object_settings.rotation,
                base_offset: self.object_settings.base_offset,
            }),
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
        Would::Opening(spec) => {
            let id = m.add_opening(spec).ok()?;
            solid::opening_solids(&m, id)
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
        let (after, made) = build_plan_ghost(&shown, would)?;
        let geo = crate::geo::Indexed::new(&after);
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
        // Walls it joins lose or gain their mitres nearby.
        let margin = Point2::new(0.8, 0.8);
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
            painter.rect_filled(screen, egui::CornerRadius::ZERO, PAPER);
            painter.image(
                tex.id(),
                screen,
                Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)),
                Color32::WHITE,
            );
        }
    }
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
        Would::Opening(spec) => {
            let wall = spec.wall;
            let id = m.add_opening(spec).ok()?;
            vec![Element::Opening(id), Element::Wall(wall)]
        }
        Would::Column(spec) => vec![Element::Column(m.add_column(spec).ok()?)],
        Would::Beam(spec) => vec![Element::Beam(m.add_beam(spec).ok()?)],
        Would::Slab(spec) => vec![Element::Slab(m.add_slab(spec).ok()?)],
        Would::Object(spec) => vec![Element::Object(m.add_object(spec).ok()?)],
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
