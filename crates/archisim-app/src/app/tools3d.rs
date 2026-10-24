//! Drawing and placing in the 3D view (R14): the tools of the plan work
//! here too. The cursor lands on what is under it (a face of a wall, a
//! volume, a slab) or, over nothing, on the plane of the active level; it
//! snaps to the corners and edge midpoints of the element under it, and
//! to the plan's points. Its plan position then goes through the same
//! snaps, clicks and keys as in the plan; Empujar takes the height from
//! the mouse going up and down the screen.

use eframe::egui::{self, Color32, Pos2, Rect, Shape, Stroke, Vec2};

use super::*;
use crate::view::View;
use crate::view3d::scene::Owner;

/// How close to a corner of the element under the cursor it snaps, in
/// screen points.
const VERTEX_REACH: f32 = 10.0;

/// Where the 3D cursor is.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Cursor3d {
    /// The point in the model.
    pub world: glam::Vec3,
    /// Metres per screen point there.
    pub scale: f32,
    /// What it is on, and the normal of the face there.
    pub owner: Option<Owner>,
    pub normal: Option<glam::Vec3>,
}

impl ArchiSimApp {
    /// Whether the tool in use works in the 3D view.
    pub(super) fn tool_in_3d(&self) -> bool {
        !matches!(self.tool, Tool::Arrow | Tool::Cad | Tool::Dimension)
    }

    /// The plane of the active level.
    fn level_z(&self) -> f32 {
        self.ed
            .model()
            .level(self.ed.level)
            .map_or(0.0, |l| l.elevation) as f32
    }

    /// The cursor, clicks and keys of the tool in use, in the 3D view.
    pub(super) fn tools_in_3d(
        &mut self,
        ctx: &egui::Context,
        response: &egui::Response,
        rect: Rect,
    ) {
        let Some(pos) = response.hover_pos() else {
            self.cursor = None;
            self.cursor3d = None;
            if !crate::app::typing_in_text(ctx) {
                self.keyboard_in_view(ctx);
            }
            return;
        };
        let size = rect.size();
        let local = pos - rect.min;
        let ray = self.v3d.camera.ray(
            glam::Vec2::new(local.x, local.y),
            glam::Vec2::new(size.x, size.y),
        );
        let z = self.level_z();
        // On the level's plane.
        let on_plane = (ray.dir.z.abs() > 1e-6)
            .then(|| (z - ray.origin.z) / ray.dir.z)
            .filter(|t| *t > 0.0)
            .map(|t| ray.origin + ray.dir * t);
        let hit = self.v3d.hit(local, size);
        let mut owner = None;
        let mut normal = None;
        let mut world = match (hit, on_plane) {
            (Some((p, n, o)), Some(q)) if p.distance(ray.origin) <= q.distance(ray.origin) => {
                owner = Some(o);
                normal = Some(n);
                p
            }
            (Some((p, n, o)), None) => {
                owner = Some(o);
                normal = Some(n);
                p
            }
            (_, Some(q)) => q,
            (None, None) => {
                self.cursor = None;
                self.cursor3d = None;
                return;
            }
        };
        // Empujar, with a volume taken: the height from the mouse, on an
        // upright plane through the point taken, facing the camera.
        // Empujar on any face: the distance along its normal from the ray.
        let face_pushing = self.tool == Tool::PushPull && self.concept.face_push.is_some();
        if face_pushing {
            self.face_push_from_ray(ray.origin, ray.dir);
        }
        let pushing = self.tool == Tool::PushPull && self.concept.push.is_some();
        if let (true, Some((id, from, h0))) = (pushing, self.concept.push) {
            let base = self
                .ed
                .model()
                .mass(id)
                .map_or(0.0, |m| m.base_offset as f32);
            let top = glam::Vec3::new(from.x as f32, from.y as f32, z + base + h0 as f32);
            let mut n = self.v3d.camera.forward();
            n.z = 0.0;
            let n = n.normalize_or(glam::Vec3::X);
            let denom = ray.dir.dot(n);
            if denom.abs() > 1e-6 {
                let t = (top - ray.origin).dot(n) / denom;
                if t > 0.0 {
                    world = ray.origin + ray.dir * t;
                }
            }
        } else if let (true, true, Some(prev)) = (
            self.drawing_edges(),
            ctx.input(|i| i.modifiers.shift),
            self.concept.path.last().copied(),
        ) {
            // Arista with Mayús: straight up or down from the last point.
            let h = self.concept.path_z.last().copied().unwrap_or(0.0) as f32;
            let from = glam::Vec3::new(prev.x as f32, prev.y as f32, z + h);
            let mut n = self.v3d.camera.forward();
            n.z = 0.0;
            let n = n.normalize_or(glam::Vec3::X);
            let denom = ray.dir.dot(n);
            if denom.abs() > 1e-6 {
                let t = (from - ray.origin).dot(n) / denom;
                if t > 0.0 {
                    let q = ray.origin + ray.dir * t;
                    world = glam::Vec3::new(from.x, from.y, q.z);
                }
            }
        } else if let Some(o) = owner {
            // The corners and edge midpoints of what is under the cursor.
            if let Some((v, _)) = self.v3d.snap_vertex(o, local, size, VERTEX_REACH) {
                world = v;
            }
        }
        // Metres per screen point at the cursor.
        let (_, right, _) = self.v3d.camera.screen_axes();
        let gsize = glam::Vec2::new(size.x, size.y);
        let scale = match (
            self.v3d.camera.project_to_screen(world, gsize),
            self.v3d.camera.project_to_screen(world + right, gsize),
        ) {
            (Some(a), Some(b)) if a.distance(b) > 1e-3 => 1.0 / a.distance(b),
            _ => 0.05,
        };
        let raw = if let (true, Some((_, from, h0))) = (pushing, self.concept.push) {
            let base = self
                .ed
                .model()
                .mass(self.concept.push.unwrap().0)
                .map_or(0.0, |m| m.base_offset);
            let dz = world.z as f64 - (z as f64 + base + h0);
            Point2::new(from.x, from.y + dz)
        } else {
            let p = Point2::new(world.x as f64, world.y as f64);
            // On the ground with the grid on: on the grid (Maqueta).
            if self.ground_grid && owner.is_none() {
                let q = super::maquette_ui::snap_to_grid(p);
                world = glam::Vec3::new(q.x as f32, q.y as f32, world.z);
                q
            } else {
                p
            }
        };
        // A plan view that puts `raw` under the cursor at this scale: the
        // plan's snaps, clicks and keys work through it.
        let zoom = 1.0 / scale as f64;
        let c = rect.center();
        let local_view = View {
            center: Point2::new(
                raw.x - (pos.x - c.x) as f64 / zoom,
                raw.y + (pos.y - c.y) as f64 / zoom,
            ),
            zoom,
        };
        // What the click is on, for the tools that look at it (Pintar).
        self.cursor3d = Some(Cursor3d {
            world,
            scale,
            owner,
            normal,
        });
        let plan_view = std::mem::replace(&mut self.view, local_view);
        // Empujar, and Arista (whose points keep their height, up a face):
        // the point under the cursor as it is.
        if pushing || face_pushing || self.drawing_edges() {
            self.cursor = Some(crate::snap::Snap::at(raw, crate::snap::SnapKind::Free));
        } else {
            self.track_cursor(ctx, response, rect);
        }
        self.sculpt_follow(ctx);
        if !crate::app::typing_in_text(ctx) {
            self.keyboard_in_view(ctx);
        }
        self.clicks(ctx, response, rect);
        self.view = plan_view;
        // The point drawn on: at the snapped plan position, on the level
        // (or where the cursor is, on a face).
        if let Some(s) = self.cursor {
            if !pushing {
                let on_face = (s.point.x as f32 - world.x).abs() < 1e-3
                    && (s.point.y as f32 - world.y).abs() < 1e-3;
                world = glam::Vec3::new(
                    s.point.x as f32,
                    s.point.y as f32,
                    if on_face { world.z } else { z },
                );
            }
        }
        self.cursor3d = Some(Cursor3d {
            world,
            scale,
            owner,
            normal,
        });
    }

    /// What is being drawn, over the 3D picture: the points so far and the
    /// cursor on the level, and the snap mark.
    pub(super) fn draw_tool_3d(&self, painter: &egui::Painter, rect: Rect) {
        let Some(c) = self.cursor3d else {
            return;
        };
        self.draw_ghost_3d(painter, rect);
        self.draw_sculpt_3d(painter, rect);
        let size = glam::Vec2::new(rect.width(), rect.height());
        let to_screen = |p: glam::Vec3| {
            self.v3d
                .camera
                .project_to_screen(p, size)
                .map(|s| Pos2::new(rect.min.x + s.x, rect.min.y + s.y))
        };
        let ink = Stroke::new(2.0, ACCENT);
        let z = self.level_z();
        // Empujar: the volume's outline at its new height.
        if let (Tool::PushPull, Some((id, from, h0))) = (self.tool, self.concept.push) {
            if let Some(m) = self.ed.model().mass(id) {
                let typed = crate::units::parse_length(&self.concept.typed);
                let h = crate::editor::snap_height(
                    self.ed.model(),
                    id,
                    typed.unwrap_or(h0 + self.cursor.map_or(0.0, |s| s.point.y - from.y)),
                );
                let top = z + (m.base_offset + h) as f32;
                let base = z + m.base_offset as f32;
                let o = m.outline();
                let ring = |zz: f32| -> Vec<Pos2> {
                    o.iter()
                        .filter_map(|p| to_screen(glam::Vec3::new(p.x as f32, p.y as f32, zz)))
                        .collect()
                };
                painter.add(Shape::closed_line(ring(top), ink));
                for p in &o {
                    if let (Some(a), Some(b)) = (
                        to_screen(glam::Vec3::new(p.x as f32, p.y as f32, base)),
                        to_screen(glam::Vec3::new(p.x as f32, p.y as f32, top)),
                    ) {
                        painter.line_segment([a, b], Stroke::new(1.0, ACCENT));
                    }
                }
                if let Some(s) = to_screen(c.world) {
                    label(
                        painter,
                        s,
                        &format!("H {} m", format!("{h:.2}").replace('.', ",")),
                    );
                }
            }
            return;
        }
        // The points of what is being drawn, and the cursor.
        let mut pts: Vec<glam::Vec3> = self
            .drawing_points()
            .into_iter()
            .map(|p| glam::Vec3::new(p.x as f32, p.y as f32, z))
            .collect();
        if self.drawing_edges() {
            pts = self
                .concept
                .path
                .iter()
                .enumerate()
                .map(|(i, p)| {
                    let h = self.concept.path_z.get(i).copied().unwrap_or(0.0) as f32;
                    glam::Vec3::new(p.x as f32, p.y as f32, z + h)
                })
                .collect();
            if !pts.is_empty() {
                pts.push(c.world);
                let screen: Vec<Pos2> = pts.iter().filter_map(|p| to_screen(*p)).collect();
                painter.add(Shape::line(screen, ink));
                pts.clear();
            }
        }
        if self.draw_method == super::methods_ui::DrawMethod::Spline {
            pts = self
                .spline_points
                .iter()
                .map(|p| glam::Vec3::new(p.x as f32, p.y as f32, z))
                .collect();
        }
        if !pts.is_empty() {
            pts.push(glam::Vec3::new(c.world.x, c.world.y, z));
            let screen: Vec<Pos2> = pts.iter().filter_map(|p| to_screen(*p)).collect();
            painter.add(Shape::line(screen, ink));
            // Up to where the cursor is, if on a face above the level.
            if (c.world.z - z).abs() > 1e-3 {
                if let (Some(a), Some(b)) = (
                    to_screen(c.world),
                    to_screen(glam::Vec3::new(c.world.x, c.world.y, z)),
                ) {
                    dashed(painter, a, b, ACCENT);
                }
            }
        }
        if let Some(s) = to_screen(c.world) {
            let kind = self.cursor.map(|s| s.kind);
            let stroke = Stroke::new(1.5, Color32::from_rgb(0, 150, 60));
            match kind {
                Some(crate::snap::SnapKind::Free) | None => {
                    painter
                        .line_segment([s - Vec2::new(8.0, 0.0), s + Vec2::new(8.0, 0.0)], stroke);
                    painter
                        .line_segment([s - Vec2::new(0.0, 8.0), s + Vec2::new(0.0, 8.0)], stroke);
                }
                _ => {
                    painter.rect_stroke(
                        Rect::from_center_size(s, Vec2::splat(11.0)),
                        0.0,
                        stroke,
                        egui::StrokeKind::Middle,
                    );
                }
            }
        }
    }
}

impl ArchiSimApp {
    /// The loose edges of Concepto, as lines in the 3D view.
    pub(super) fn draw_edges_3d(&self, painter: &egui::Painter, rect: Rect) {
        let model = self.ed.model();
        if model.edges().next().is_none() {
            return;
        }
        let size = glam::Vec2::new(rect.width(), rect.height());
        for e in model.edges() {
            let z = model.level(e.level).map_or(0.0, |l| l.elevation) + e.elevation;
            let selected = self.ed.selection.contains(&Element::Edge(e.id));
            let stroke = Stroke::new(
                if selected { 2.5 } else { 1.5 },
                if selected {
                    ACCENT
                } else {
                    Color32::from_gray(40)
                },
            );
            let level_z = model.level(e.level).map_or(0.0, |l| l.elevation);
            let _ = z;
            let pts: Vec<Pos2> = (0..e.points.len())
                .filter_map(|i| {
                    let (p, h) = e.point_3d(i);
                    self.v3d
                        .camera
                        .project_to_screen(
                            glam::Vec3::new(p.x as f32, p.y as f32, (level_z + h) as f32),
                            size,
                        )
                        .map(|s| Pos2::new(rect.min.x + s.x, rect.min.y + s.y))
                })
                .collect();
            if pts.len() == e.points.len() {
                painter.add(Shape::line(pts, stroke));
            }
        }
    }
}

/// A label next to a point of the screen.
fn label(painter: &egui::Painter, at: Pos2, text: &str) {
    let galley = painter.layout_no_wrap(
        text.to_string(),
        egui::FontId::monospace(12.0),
        Color32::WHITE,
    );
    let r = Rect::from_min_size(at + Vec2::new(14.0, 10.0), galley.size() + Vec2::splat(8.0));
    painter.rect_filled(r, 3.0, Color32::from_rgb(20, 60, 140));
    painter.galley(r.min + Vec2::splat(4.0), galley, Color32::WHITE);
}
