//! The handles of a street in the 3D view, as in the plan: with a street
//! selected, the squares on its nodes move them (the streets there follow
//! it, a loose end is pulled onto the street near it), the circle in the
//! middle of each stretch bends it and the arrows on its kerbs widen or
//! narrow that stretch. They are dragged on the street's own height and
//! the street is drawn as it will be, with its crossings.

use archisim_core::Point2;
use eframe::egui::{self, Color32, PointerButton, Pos2, Rect};

use super::*;
use crate::drag::{Drag, Grip};

/// A street handle being dragged in 3D.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RoadGrip3d {
    pub grip: Grip,
    /// Where it was taken, in plan, and the height of the plane it moves
    /// on.
    pub from: Point2,
    pub z: f64,
    pub to: Point2,
}

/// How near the cursor takes a handle, in pixels.
const REACH: f32 = 12.0;

/// Where along its street a handle is.
fn station_of(g: Grip) -> f64 {
    match g {
        Grip::RoadNode { mm, .. } | Grip::RoadBend { mm, .. } | Grip::RoadWidth { mm, .. } => {
            mm as f64 / 1000.0
        }
        _ => 0.0,
    }
}

impl ArchiSimApp {
    /// The handles of the streets selected, at the height of their
    /// carriageway.
    pub(super) fn road_handles_3d(&self) -> Vec<(Grip, glam::DVec3)> {
        if self.tool != Tool::Arrow {
            return Vec::new();
        }
        let model = self.ed.model();
        crate::drag::hotspots(model, &self.ed.selection)
            .into_iter()
            .filter_map(|h| {
                let r = model.road(crate::drag::road_of(h.grip)?)?;
                let z = model.level(r.level).map_or(0.0, |l| l.elevation)
                    + r.top_at(station_of(h.grip))
                    + 0.05;
                Some((h.grip, glam::DVec3::new(h.point.x, h.point.y, z)))
            })
            .collect()
    }

    fn road_handle_at(&self, rect: Rect, pos: Pos2) -> Option<(Grip, glam::DVec3)> {
        self.road_handles_3d()
            .into_iter()
            .filter_map(|h| Some((h, self.screen_3d(rect, h.1)?.distance(pos))))
            .filter(|(_, d)| *d <= crate::touch::reach(REACH))
            .min_by(|a, b| a.1.total_cmp(&b.1))
            .map(|(h, _)| h)
    }

    /// Hover, grab, drag and let go of a street's handles. Returns whether
    /// one is taken (the camera then stays still).
    pub(super) fn road_grips_3d(
        &mut self,
        ctx: &egui::Context,
        response: &egui::Response,
        rect: Rect,
    ) -> bool {
        if self.road_grip.is_none() {
            self.road_grip_hover = response
                .hover_pos()
                .and_then(|p| self.road_handle_at(rect, p))
                .map(|h| h.0);
        }
        if response.drag_started_by(PointerButton::Primary) {
            let press = ctx.input(|i| i.pointer.press_origin());
            self.road_grip = press
                .and_then(|p| self.road_handle_at(rect, p))
                .map(|(grip, at)| {
                    let from = Point2::new(at.x, at.y);
                    RoadGrip3d {
                        grip,
                        from,
                        z: at.z,
                        to: from,
                    }
                });
        }
        let Some(mut g) = self.road_grip else {
            return false;
        };
        if response.dragged_by(PointerButton::Primary) {
            if let Some(p) = response
                .interact_pointer_pos()
                .and_then(|pos| self.mass_on_plane(rect, pos, g.z))
            {
                g.to = Point2::new(p.x, p.y);
            }
            self.road_grip = Some(g);
            self.road_preview(&g);
            ctx.request_repaint();
        }
        if response.drag_stopped() {
            self.road_grip = None;
            self.v3d.set_overlay(None, || None);
            if !g.to.approx_eq(g.from) {
                let drag = Drag {
                    grip: g.grip,
                    from: g.from,
                    to: g.to,
                    variant: false,
                };
                let dist = crate::units::fmt_len(g.from.distance(g.to));
                match self.ed.drag(&drag) {
                    Ok(label) => self.say(format!("{label}: {dist}.")),
                    Err(e) => self.complain(format!("No se puede: {e}.")),
                }
            }
        }
        true
    }

    /// The street as it will be, with the streets it meets, drawn in its
    /// place while its handle is dragged.
    fn road_preview(&mut self, g: &RoadGrip3d) {
        let Some(road) = crate::drag::road_of(g.grip) else {
            return;
        };
        if g.to.approx_eq(g.from) {
            self.v3d.set_overlay(None, || None);
            return;
        }
        let key = format!(
            "calle-asa:{}:{:?}:{:.3}:{:.3}",
            self.ed.revision(),
            g.grip,
            g.to.x,
            g.to.y
        );
        let model = self.ed.model();
        let selection = self.ed.selection.clone();
        let drag = Drag {
            grip: g.grip,
            from: g.from,
            to: g.to,
            variant: false,
        };
        self.v3d.set_overlay(Some(key), || {
            let mut m = model.clone();
            let kept = crate::drag::apply(&mut m, &selection, &drag).ok()?;
            let road = kept
                .and_then(|k| {
                    k.into_iter().find_map(|e| match e {
                        Element::Road(r) => Some(r),
                        _ => None,
                    })
                })
                .unwrap_or(road);
            let (new, met): (Vec<archisim_geom::solid::Solid>, Vec<_>) =
                super::ghost3d::road_with_met(&m, road)
                    .into_iter()
                    .partition(|s| s.owner == archisim_geom::solid::SolidOwner::Road(road));
            Some(
                crate::view3d::scene::Scene::of_solids(&m, new)
                    .joined(crate::view3d::scene::Scene::of_solids_unmarked(&m, met)),
            )
        });
    }

    /// The handles of the streets selected: squares on the nodes, circles
    /// to bend, arrows on the kerbs.
    pub(super) fn draw_road_grips_3d(&self, painter: &egui::Painter, rect: Rect) {
        let model = self.ed.model();
        for (grip, at) in self.road_handles_3d() {
            let Some(c) = self.screen_3d(rect, at) else {
                continue;
            };
            let hot = self.road_grip_hover == Some(grip)
                || self.road_grip.is_some_and(|g| g.grip == grip);
            match grip {
                Grip::RoadBend { .. } => {
                    let r = if hot { 6.5 } else { 5.0 };
                    let inside = if hot { ACCENT } else { Color32::WHITE };
                    painter.circle(c, r, inside, egui::Stroke::new(1.5, Color32::BLACK));
                }
                Grip::RoadWidth { road, mm, .. } => {
                    let Some(r) = model.road(road) else {
                        continue;
                    };
                    let axis = r.axis_at(mm as f64 / 1000.0).0;
                    let out = (Point2::new(at.x, at.y) - axis)
                        .normalized()
                        .unwrap_or(Point2::new(1.0, 0.0));
                    let dir = self
                        .screen_3d(rect, at + glam::DVec3::new(out.x, out.y, 0.0) * 0.5)
                        .map_or(egui::Vec2::X, |q| q - c);
                    let dir = if dir.length() > 1e-3 {
                        dir.normalized()
                    } else {
                        egui::Vec2::X
                    };
                    super::handles::arrow(painter, c, dir, ACCENT, hot);
                }
                _ => {
                    let s = if hot { 6.0 } else { 4.5 };
                    let r = Rect::from_center_size(c, egui::vec2(2.0 * s, 2.0 * s));
                    painter.rect(
                        r,
                        1.0,
                        if hot { ACCENT } else { Color32::BLACK },
                        egui::Stroke::new(1.5, Color32::WHITE),
                        egui::StrokeKind::Middle,
                    );
                }
            }
        }
        if let Some(g) = self.road_grip {
            if let Some(p) = painter.ctx().pointer_latest_pos() {
                let what = crate::drag::label(
                    model,
                    &Drag {
                        grip: g.grip,
                        from: g.from,
                        to: g.to,
                        variant: false,
                    },
                );
                let text = format!("{what}: {}", crate::units::fmt_len(g.from.distance(g.to)));
                super::handles::tag(painter, p, text, ACCENT);
            }
        }
    }
}
