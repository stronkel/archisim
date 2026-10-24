//! The 3D camera: an orbit around a target point, in perspective or
//! axonometric (parallel) projection. Model coordinates: metres, Z up.

use glam::camera::rh::{proj::directx, view::look_at_mat4};
use glam::{Mat4, Vec2, Vec3};

/// Vertical field of view in perspective, and the equivalent framing in
/// axonometry, in radians.
pub const FOV_Y: f32 = 0.70;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Camera {
    /// The point the camera orbits around and looks at.
    pub target: Vec3,
    /// Direction the camera stands in, around the vertical, from +X (east)
    /// counter-clockwise.
    pub yaw: f32,
    /// Height of the camera above the horizontal through the target.
    pub pitch: f32,
    pub distance: f32,
    pub perspective: bool,
}

impl Default for Camera {
    /// Seen from the south-east, from above, as ArchiCAD's default 3D.
    fn default() -> Self {
        Self {
            target: Vec3::new(6.0, 4.5, 1.5),
            yaw: (-55.0f32).to_radians(),
            pitch: 30.0f32.to_radians(),
            distance: 30.0,
            perspective: true,
        }
    }
}

/// A ray from the eye into the scene.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Ray {
    pub origin: Vec3,
    pub dir: Vec3,
}

impl Camera {
    /// Unit vector from the target towards the eye.
    fn back(&self) -> Vec3 {
        Vec3::new(
            self.pitch.cos() * self.yaw.cos(),
            self.pitch.cos() * self.yaw.sin(),
            self.pitch.sin(),
        )
    }

    pub fn eye(&self) -> Vec3 {
        self.target + self.back() * self.distance
    }

    /// Forward, screen-right and screen-up directions in the model.
    pub fn screen_axes(&self) -> (Vec3, Vec3, Vec3) {
        self.basis()
    }

    /// Screen-right and screen-up directions in the model.
    fn basis(&self) -> (Vec3, Vec3, Vec3) {
        let forward = -self.back();
        let right = forward.cross(Vec3::Z).normalize_or(Vec3::X);
        let up = right.cross(forward);
        (forward, right, up)
    }

    /// Half the height of the view at the target, in metres.
    fn half_height(&self) -> f32 {
        self.distance * (FOV_Y / 2.0).tan()
    }

    pub fn view(&self) -> Mat4 {
        look_at_mat4(self.eye(), self.target, Vec3::Z)
    }

    pub fn projection(&self, aspect: f32) -> Mat4 {
        let near = (self.distance * 0.002).max(0.01);
        let far = self.distance * 50.0 + 100.0;
        if self.perspective {
            directx::perspective(FOV_Y, aspect, near, far)
        } else {
            let h = self.half_height();
            let w = h * aspect;
            // Parallel projection: the near plane is pulled back so nothing
            // between the eye and the target is clipped.
            directx::orthographic(-w, w, -h, h, -far, far)
        }
    }

    pub fn view_proj(&self, aspect: f32) -> Mat4 {
        self.projection(aspect) * self.view()
    }

    /// Direction the camera looks in.
    pub fn forward(&self) -> Vec3 {
        -self.back()
    }

    /// Where a model point falls in a view of `size` points (from its
    /// top-left corner); `None` behind the eye.
    pub fn project_to_screen(&self, p: Vec3, size: Vec2) -> Option<Vec2> {
        let clip = self.view_proj(size.x / size.y.max(1.0)) * p.extend(1.0);
        if clip.w <= 1e-6 {
            return None;
        }
        let ndc = clip.truncate() / clip.w;
        Some(Vec2::new(
            (ndc.x + 1.0) / 2.0 * size.x,
            (1.0 - ndc.y) / 2.0 * size.y,
        ))
    }

    /// Turns the camera around the target (dragging by screen points).
    pub fn orbit(&mut self, delta: Vec2) {
        self.yaw -= delta.x * 0.008;
        self.pitch = (self.pitch + delta.y * 0.008).clamp(-1.5, 1.5);
    }

    /// Turns the camera around `pivot` (what is selected) instead of its
    /// target: the eye and what it looks at turn together about it, so
    /// the selection stays where it is on screen while the view goes
    /// round it.
    pub fn orbit_about(&mut self, delta: Vec2, pivot: Vec3) {
        let dy = -delta.x * 0.008;
        let pitch = (self.pitch + delta.y * 0.008).clamp(-1.5, 1.5);
        let dp = pitch - self.pitch;
        let (_, right, _) = self.basis();
        // About the screen's right: the sign that lifts the eye as the
        // pitch grows.
        let tilt = |a: f32| glam::Quat::from_axis_angle(right, a);
        let lifted = tilt(dp) * self.back();
        let turn_up = if (lifted.z - self.back().z) * dp >= 0.0 {
            tilt(dp)
        } else {
            tilt(-dp)
        };
        let r = glam::Quat::from_rotation_z(dy) * turn_up;
        self.target = pivot + r * (self.target - pivot);
        self.yaw += dy;
        self.pitch = pitch;
    }

    /// Moves the target with the drawing (dragging by screen points), in a
    /// view `height` points high.
    pub fn pan(&mut self, delta: Vec2, height: f32) {
        let (_, right, up) = self.basis();
        let k = 2.0 * self.half_height() / height.max(1.0);
        self.target += (-right * delta.x + up * delta.y) * k;
    }

    /// Walks the camera as in a game (W A S D): `forward` and `right`
    /// along the ground in the direction it looks, `up` straight up, in
    /// metres; the eye and what it looks at move together.
    pub fn walk(&mut self, forward: f32, right: f32, up: f32) {
        let f = Vec3::new(-self.yaw.cos(), -self.yaw.sin(), 0.0);
        let r = f.cross(Vec3::Z);
        self.target += f * forward + r * right + Vec3::Z * up;
    }

    /// Ray through a point of the view, in points from its top-left corner.
    pub fn ray(&self, pos: Vec2, size: Vec2) -> Ray {
        let (forward, right, up) = self.basis();
        let aspect = size.x / size.y.max(1.0);
        // -1..1 across the view, Y up.
        let ndc = Vec2::new(2.0 * pos.x / size.x - 1.0, 1.0 - 2.0 * pos.y / size.y);
        let h = self.half_height();
        let offset = right * ndc.x * h * aspect + up * ndc.y * h;
        if self.perspective {
            let through = self.target + offset;
            let origin = self.eye();
            Ray {
                origin,
                dir: (through - origin).normalize(),
            }
        } else {
            Ray {
                origin: self.eye() + offset - forward * self.distance * 50.0,
                dir: forward,
            }
        }
    }

    /// Zooms by `factor` (< 1 closer) keeping the point under the cursor,
    /// on the plane through the target, where it was.
    pub fn zoom_at(&mut self, factor: f32, pos: Vec2, size: Vec2) {
        let before = self.on_target_plane(pos, size);
        self.distance = (self.distance * factor).clamp(0.5, 5000.0);
        let after = self.on_target_plane(pos, size);
        if let (Some(b), Some(a)) = (before, after) {
            self.target += b - a;
        }
    }

    fn on_target_plane(&self, pos: Vec2, size: Vec2) -> Option<Vec3> {
        let r = self.ray(pos, size);
        let n = -self.basis().0;
        let den = r.dir.dot(n);
        if den.abs() < 1e-6 {
            return None;
        }
        let t = (self.target - r.origin).dot(n) / den;
        Some(r.origin + r.dir * t)
    }

    /// Frames a bounding box in a view of the given aspect ratio.
    pub fn fit(&mut self, lo: Vec3, hi: Vec3, aspect: f32) {
        let center = (lo + hi) / 2.0;
        let radius = ((hi - lo).length() / 2.0).max(1.0);
        let half = (FOV_Y / 2.0).tan() * aspect.min(1.0);
        self.target = center;
        self.distance = radius / half.atan().sin() * 1.05;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_view_is_from_the_south_east_above() {
        let c = Camera::default();
        let e = c.eye() - c.target;
        assert!(e.x > 0.0 && e.y < 0.0 && e.z > 0.0, "{e:?}");
        // The target projects to the centre of the view.
        let p = c.view_proj(1.5).project_point3(c.target);
        assert!(p.x.abs() < 1e-5 && p.y.abs() < 1e-5);
        assert!(p.z > 0.0 && p.z < 1.0, "inside the depth range: {}", p.z);
    }

    #[test]
    fn rays_go_through_what_is_drawn_there() {
        let size = Vec2::new(800.0, 600.0);
        for perspective in [true, false] {
            let c = Camera {
                perspective,
                ..Camera::default()
            };
            let vp = c.view_proj(size.x / size.y);
            let point = Vec3::new(3.0, 2.0, 0.5);
            let ndc = vp.project_point3(point);
            let pos = Vec2::new((ndc.x + 1.0) / 2.0 * size.x, (1.0 - ndc.y) / 2.0 * size.y);
            let r = c.ray(pos, size);
            // Distance from the point to the ray.
            let to = point - r.origin;
            let off = (to - r.dir * to.dot(r.dir)).length();
            assert!(off < 1e-3, "perspective {perspective}: {off}");
        }
    }

    #[test]
    fn zoom_keeps_the_point_under_the_cursor() {
        let size = Vec2::new(800.0, 600.0);
        for perspective in [true, false] {
            let mut c = Camera {
                perspective,
                ..Camera::default()
            };
            let pos = Vec2::new(200.0, 150.0);
            let before = c.on_target_plane(pos, size).unwrap();
            c.zoom_at(0.5, pos, size);
            assert!((c.distance - 15.0).abs() < 1e-4);
            let after = c.on_target_plane(pos, size).unwrap();
            assert!((after - before).length() < 1e-3, "{before:?} {after:?}");
        }
    }

    #[test]
    fn walks_where_it_looks() {
        let mut c = Camera::default();
        let (eye, target) = (c.eye(), c.target);
        c.walk(2.0, 0.0, 0.0);
        // Forward along the ground, towards where it looks.
        let moved = c.target - target;
        assert!(moved.z.abs() < 1e-6 && (moved.length() - 2.0).abs() < 1e-5);
        assert!(moved.dot(c.forward()) > 0.0);
        assert!(((c.eye() - eye) - moved).length() < 1e-5);
        // To the right, as on the screen; and up.
        let before = c.target;
        c.walk(0.0, 1.0, 0.5);
        let (_, right, _) = c.screen_axes();
        let d = c.target - before;
        assert!((d.truncate().dot(right.truncate()) - 1.0).abs() < 1e-4);
        assert!((d.z - 0.5).abs() < 1e-6);
    }

    /// Turning about a point keeps that point where it was on screen.
    #[test]
    fn orbit_about_a_selection() {
        let mut c = Camera::default();
        let pivot = c.target + Vec3::new(3.0, -2.0, 0.5);
        let size = Vec2::new(800.0, 600.0);
        let before = c.project_to_screen(pivot, size).unwrap();
        c.orbit_about(Vec2::new(60.0, 25.0), pivot);
        let after = c.project_to_screen(pivot, size).unwrap();
        assert!((after - before).length() < 1.0, "{before:?} {after:?}");
        // …and the view did turn.
        assert!((c.yaw - Camera::default().yaw).abs() > 0.3);
        assert!(c.pitch > Camera::default().pitch);
    }

    #[test]
    fn orbit_pan_and_fit() {
        let mut c = Camera::default();
        c.orbit(Vec2::new(0.0, 10_000.0));
        assert!(c.pitch <= 1.5, "never upside down");
        let t = c.target;
        c.pan(Vec2::new(100.0, 0.0), 600.0);
        assert!((c.target - t).length() > 0.0);
        assert!((c.target - t).z.abs() < 1e-3 || c.pitch.abs() > 0.1);
        c.fit(Vec3::ZERO, Vec3::new(12.0, 9.0, 3.5), 1.6);
        assert_eq!(c.target, Vec3::new(6.0, 4.5, 1.75));
        // The whole box is in front of the camera and inside the view.
        let vp = c.view_proj(1.6);
        for x in [0.0, 12.0] {
            for y in [0.0, 9.0] {
                for z in [0.0, 3.5] {
                    let p = vp.project_point3(Vec3::new(x, y, z));
                    assert!(p.x.abs() <= 1.0 && p.y.abs() <= 1.0, "{p:?}");
                }
            }
        }
    }
}
