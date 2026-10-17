//! The plan view's window onto the model: where it looks and how close.
//!
//! Model coordinates are metres with Y pointing north (up); screen
//! coordinates are egui points with Y pointing down.

use archisim_core::Point2;
use eframe::egui::{Pos2, Rect, Vec2};

/// Closest and furthest zoom, in screen points per metre.
pub const MIN_ZOOM: f64 = 1.0;
pub const MAX_ZOOM: f64 = 20_000.0;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct View {
    /// Model point shown at the centre of the view.
    pub center: Point2,
    /// Screen points per metre.
    pub zoom: f64,
}

impl Default for View {
    fn default() -> Self {
        // About 20 m across a typical window.
        Self {
            center: Point2::new(6.0, 4.5),
            zoom: 50.0,
        }
    }
}

impl View {
    pub fn to_screen(self, rect: Rect, p: Point2) -> Pos2 {
        let c = rect.center();
        Pos2::new(
            c.x + ((p.x - self.center.x) * self.zoom) as f32,
            c.y - ((p.y - self.center.y) * self.zoom) as f32,
        )
    }

    pub fn to_model(self, rect: Rect, pos: Pos2) -> Point2 {
        let c = rect.center();
        Point2::new(
            self.center.x + (pos.x - c.x) as f64 / self.zoom,
            self.center.y - (pos.y - c.y) as f64 / self.zoom,
        )
    }

    /// A screen distance in metres.
    pub fn to_metres(self, points: f32) -> f64 {
        points as f64 / self.zoom
    }

    /// The model region visible in `rect`: lower-left and upper-right.
    pub fn visible(self, rect: Rect) -> (Point2, Point2) {
        let a = self.to_model(rect, rect.left_bottom());
        let b = self.to_model(rect, rect.right_top());
        (a, b)
    }

    /// Moves the view by a screen displacement (dragging the drawing).
    pub fn pan(&mut self, delta: Vec2) {
        self.center.x -= delta.x as f64 / self.zoom;
        self.center.y += delta.y as f64 / self.zoom;
    }

    /// Zooms by `factor` keeping the model point under `pos` in place.
    pub fn zoom_at(&mut self, rect: Rect, pos: Pos2, factor: f64) {
        let anchor = self.to_model(rect, pos);
        self.zoom = (self.zoom * factor).clamp(MIN_ZOOM, MAX_ZOOM);
        let moved = self.to_model(rect, pos);
        self.center = self.center + (anchor - moved);
    }

    /// Frames the model region `lo`–`hi` in `rect` with a margin in points.
    pub fn fit(&mut self, rect: Rect, lo: Point2, hi: Point2, margin: f32) {
        let w = (hi.x - lo.x).max(1.0);
        let h = (hi.y - lo.y).max(1.0);
        let avail_w = (rect.width() - 2.0 * margin).max(10.0) as f64;
        let avail_h = (rect.height() - 2.0 * margin).max(10.0) as f64;
        self.zoom = (avail_w / w).min(avail_h / h).clamp(MIN_ZOOM, MAX_ZOOM);
        self.center = Point2::new((lo.x + hi.x) / 2.0, (lo.y + hi.y) / 2.0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rect() -> Rect {
        Rect::from_min_size(Pos2::new(100.0, 50.0), Vec2::new(800.0, 600.0))
    }

    #[test]
    fn round_trip_and_y_up() {
        let v = View {
            center: Point2::new(10.0, 5.0),
            zoom: 40.0,
        };
        let r = rect();
        assert_eq!(v.to_screen(r, Point2::new(10.0, 5.0)), r.center());
        // North is up on screen.
        let north = v.to_screen(r, Point2::new(10.0, 6.0));
        assert_eq!(north, r.center() - Vec2::new(0.0, 40.0));
        let p = Point2::new(3.25, -1.5);
        let back = v.to_model(r, v.to_screen(r, p));
        assert!(back.distance(p) < 1e-4);
        let (lo, hi) = v.visible(r);
        assert!((hi.x - lo.x - 20.0).abs() < 1e-9 && (hi.y - lo.y - 15.0).abs() < 1e-9);
    }

    #[test]
    fn zoom_keeps_point_under_cursor() {
        let mut v = View::default();
        let r = rect();
        let cursor = Pos2::new(250.0, 120.0);
        let before = v.to_model(r, cursor);
        v.zoom_at(r, cursor, 2.0);
        assert_eq!(v.zoom, 100.0);
        assert!(v.to_model(r, cursor).distance(before) < 1e-9);
        v.zoom_at(r, cursor, 1e9);
        assert_eq!(v.zoom, MAX_ZOOM);
    }

    #[test]
    fn pan_and_fit() {
        let mut v = View::default();
        let r = rect();
        let p = Point2::new(7.0, 3.0);
        let before = v.to_screen(r, p);
        v.pan(Vec2::new(30.0, -10.0));
        assert!((v.to_screen(r, p) - before - Vec2::new(30.0, -10.0)).length() < 1e-3);
        v.fit(r, Point2::new(0.0, 0.0), Point2::new(12.0, 9.0), 50.0);
        assert_eq!(v.center, Point2::new(6.0, 4.5));
        // 700 × 500 points available: height decides.
        assert!((v.zoom - 500.0 / 9.0).abs() < 1e-9);
    }
}
