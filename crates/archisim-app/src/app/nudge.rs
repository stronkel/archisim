//! Moving the selection with the arrow keys, as in Revit: up, down, left
//! and right in the view (plan, section or elevation, drafting view), by a
//! step that follows the zoom (closer in, finer); with Shift, ten times as
//! far.

use super::*;

/// The steps the arrows move by, m: the smallest that is at least
/// [`NUDGE_PIXELS`] on screen.
const STEPS: [f64; 13] = [
    0.001, 0.002, 0.005, 0.01, 0.02, 0.05, 0.1, 0.2, 0.5, 1.0, 2.0, 5.0, 10.0,
];
/// How far one press moves on screen, at least, in points.
const NUDGE_PIXELS: f32 = 4.0;

/// The step for a view where one screen point is `metres_per_point`.
pub fn nudge_step(metres_per_point: f64) -> f64 {
    let want = metres_per_point * NUDGE_PIXELS as f64;
    STEPS
        .iter()
        .copied()
        .find(|s| *s >= want)
        .unwrap_or(STEPS[STEPS.len() - 1])
}

/// The corners, middles of the sides and centre of an outline.
pub fn key_points(outline: &[Point2]) -> Vec<Point2> {
    let n = outline.len();
    if n == 0 {
        return Vec::new();
    }
    let mut out: Vec<Point2> = outline.to_vec();
    for i in 0..n {
        out.push((outline[i] + outline[(i + 1) % n]) * 0.5);
    }
    let c = outline.iter().fold(Point2::new(0.0, 0.0), |a, p| a + *p) * (1.0 / n as f64);
    out.push(c);
    out
}

/// How much to add to `delta` so that one of `moving` (moved by `delta`)
/// lines up in x, and one in y, with one of `around`, each within `reach`;
/// and the guides that show it (from the point around to the moved one).
pub fn align_points(
    moving: &[Point2],
    around: &[Point2],
    delta: Point2,
    reach: f64,
) -> (Point2, Vec<(Point2, Point2)>) {
    let mut best_x: Option<(f64, f64)> = None;
    let mut best_y: Option<(f64, f64)> = None;
    for m in moving {
        let q = *m + delta;
        for a in around {
            let dx = a.x - q.x;
            if dx.abs() < reach && best_x.is_none_or(|(b, _)| dx.abs() < b.abs()) {
                best_x = Some((dx, a.x));
            }
            let dy = a.y - q.y;
            if dy.abs() < reach && best_y.is_none_or(|(b, _)| dy.abs() < b.abs()) {
                best_y = Some((dy, a.y));
            }
        }
    }
    let fix = Point2::new(
        best_x.map_or(0.0, |(d, _)| d),
        best_y.map_or(0.0, |(d, _)| d),
    );
    let delta = delta + fix;
    // One guide per direction: the moved point nearest to a point around
    // on that line.
    let mut guides = Vec::new();
    let pair = |on: &dyn Fn(Point2) -> bool| {
        moving
            .iter()
            .map(|m| *m + delta)
            .filter(|q| on(*q))
            .flat_map(|q| around.iter().filter(|a| on(**a)).map(move |a| (*a, q)))
            .min_by(|x, y| x.0.distance(x.1).total_cmp(&y.0.distance(y.1)))
    };
    if let Some((_, x)) = best_x {
        guides.extend(pair(&|q: Point2| (q.x - x).abs() < 1e-9));
    }
    if let Some((_, y)) = best_y {
        guides.extend(pair(&|q: Point2| (q.y - y).abs() < 1e-9));
    }
    (fix, guides)
}

impl ArchiSimApp {
    /// The points of what is around the selection on this level that a
    /// moved or stretched element lines up with: the corners, middles and
    /// centres of objects, pillars and boxes, and the ends of walls,
    /// within `radius` of `near`.
    pub(super) fn points_around(&self, near: Point2, radius: f64) -> Vec<Point2> {
        let m = self.ed.model();
        let level = self.ed.level;
        let mut elements: Vec<Element> = Vec::new();
        elements.extend(
            m.objects_on_level(level)
                .filter(|o| o.position.distance(near) < radius)
                .map(|o| Element::Object(o.id)),
        );
        elements.extend(
            m.columns_on_level(level)
                .filter(|c| c.position.distance(near) < radius)
                .map(|c| Element::Column(c.id)),
        );
        elements.extend(
            m.boxes()
                .filter(|b| b.level == level && b.origin.distance(near) < radius + 15.0)
                .map(|b| Element::Box(b.id)),
        );
        let mut out = Vec::new();
        for e in elements {
            if self.ed.selection.contains(&e) {
                continue;
            }
            if let Some(o) = crate::pick::outline(&self.ed, e) {
                out.extend(key_points(&o));
            }
        }
        for w in m.walls_on_level(level) {
            if self.ed.selection.contains(&Element::Wall(w.id)) {
                continue;
            }
            for p in [w.start, w.end] {
                if p.distance(near) < radius {
                    out.push(p);
                }
            }
        }
        out
    }

    /// The arrows with something selected and the arrow tool: the
    /// selection moves one step that way. Whether a key was taken.
    pub(super) fn nudge_keys(&mut self, ctx: &egui::Context, section: Option<SectionId>) -> bool {
        if self.tool != Tool::Arrow
            || self.ed.selection.is_empty()
            || self.drag.is_some()
            || crate::app::typing_in_text(ctx)
        {
            return false;
        }
        let mut dir = Point2::new(0.0, 0.0);
        let mut far = false;
        for (key, d) in [
            (Key::ArrowLeft, Point2::new(-1.0, 0.0)),
            (Key::ArrowRight, Point2::new(1.0, 0.0)),
            (Key::ArrowUp, Point2::new(0.0, 1.0)),
            (Key::ArrowDown, Point2::new(0.0, -1.0)),
        ] {
            // Shift first: the plain one would take it too.
            if ctx.input_mut(|i| i.consume_key(egui::Modifiers::SHIFT, key)) {
                dir = dir + d;
                far = true;
            } else if ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, key)) {
                dir = dir + d;
            }
        }
        if dir.length() < 1e-9 {
            return false;
        }
        let step = nudge_step(self.view.to_metres(1.0)) * if far { 10.0 } else { 1.0 };
        let d = dir * step;
        match section {
            Some(id) => {
                let elements: Vec<Element> = self
                    .ed
                    .selection
                    .iter()
                    .copied()
                    .filter(|e| matches!(e, Element::Object(_) | Element::Opening(_)))
                    .collect();
                if elements.is_empty() {
                    return true;
                }
                self.move_in_section(id, &elements, d);
            }
            None => {
                let from = Point2::new(0.0, 0.0);
                let drag = crate::drag::Drag {
                    grip: crate::drag::Grip::Body,
                    from,
                    to: from + d,
                    variant: false,
                };
                match self.ed.drag(&drag) {
                    Ok(_) => self.say(format!(
                        "Movido {} (flechas; con Mayúsculas, ×10).",
                        crate::units::fmt_len(step)
                    )),
                    Err(e) => self.complain(format!("No se puede: {e}.")),
                }
            }
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn corners_line_up_with_what_is_around() {
        let p = Point2::new;
        let box_ = [p(0.0, 0.0), p(1.0, 0.0), p(1.0, 1.0), p(0.0, 1.0)];
        // Moved by (2,03; 0,98): its left side lines up with x = 2, its top
        // with y = 2.
        let (fix, guides) = align_points(&box_, &[p(2.0, 5.0), p(9.0, 2.0)], p(2.03, 0.98), 0.1);
        assert!(fix.distance(p(-0.03, 0.02)) < 1e-9, "{fix:?}");
        assert_eq!(guides.len(), 2);
    }

    #[test]
    fn the_step_follows_the_zoom() {
        // 1 mm per point: 5 mm; 5 cm per point: 0,20 m.
        assert_eq!(nudge_step(0.001), 0.005);
        assert_eq!(nudge_step(0.05), 0.2);
        assert_eq!(nudge_step(100.0), 10.0);
    }
}
