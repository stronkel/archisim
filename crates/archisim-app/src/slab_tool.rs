//! The slab tool: click the corners of the outline, then close it.
//!
//! As in ArchiCAD's polygonal slab: each click adds a corner; clicking the
//! first corner again (or pressing Enter with three or more corners) closes
//! the outline and asks for the slab to be built. While drawing, a typed
//! length places the next corner along the direction of the cursor.

use archisim_core::{is_simple_polygon, Point2};

use crate::units::parse_length;

#[derive(Debug, Clone, Default, PartialEq)]
pub struct SlabTool {
    points: Vec<Point2>,
    /// Length being typed, in millimetres (or with a unit).
    pub typed: String,
}

/// What the tool wants done after an input.
#[derive(Debug, Clone, PartialEq)]
pub enum Outcome {
    Nothing,
    /// Build a slab with this outline, then call [`SlabTool::accept`].
    Build(Vec<Point2>),
    /// The outline cannot close as it is (fewer than three corners, or it
    /// crosses itself).
    Invalid,
}

impl SlabTool {
    pub fn corners(&self) -> &[Point2] {
        &self.points
    }

    pub fn anchor(&self) -> Option<Point2> {
        self.points.last().copied()
    }

    pub fn is_drawing(&self) -> bool {
        !self.points.is_empty()
    }

    fn close(&self) -> Outcome {
        if self.points.len() >= 3 && is_simple_polygon(&self.points) {
            Outcome::Build(self.points.clone())
        } else {
            Outcome::Invalid
        }
    }

    /// A click at an already snapped point.
    pub fn click(&mut self, p: Point2) -> Outcome {
        if let Some(first) = self.points.first() {
            if self.points.len() >= 3 && first.approx_eq(p) {
                return self.close();
            }
        }
        if self.anchor().is_some_and(|a| a.approx_eq(p)) {
            return Outcome::Nothing;
        }
        self.points.push(p);
        self.typed.clear();
        Outcome::Nothing
    }

    /// The slab was built: start again.
    pub fn accept(&mut self) {
        self.points.clear();
        self.typed.clear();
    }

    /// The corner the typed length gives towards `towards`.
    pub fn typed_end(&self, towards: Point2) -> Option<Point2> {
        let a = self.anchor()?;
        let len = parse_length(&self.typed).filter(|l| *l > 0.0)?;
        let dir = (towards - a).normalized()?;
        Some(a + dir * len)
    }

    pub fn type_char(&mut self, c: char) -> bool {
        let fits = c.is_ascii_digit() || matches!(c, ',' | '.' | 'm' | 'c' | 'M' | 'C');
        if self.is_drawing() && fits {
            self.typed.push(c);
            true
        } else {
            false
        }
    }

    pub fn backspace(&mut self) -> bool {
        self.typed.pop().is_some()
    }

    /// Enter: place the typed corner, or close the outline.
    pub fn enter(&mut self, towards: Point2) -> Outcome {
        if !self.typed.is_empty() {
            if let Some(p) = self.typed_end(towards) {
                return self.click(p);
            }
            return Outcome::Nothing;
        }
        if self.is_drawing() {
            self.close()
        } else {
            Outcome::Nothing
        }
    }

    /// Escape: clear the typed length, then the last corner, then idle.
    /// Returns `false` when there was nothing to undo.
    pub fn escape(&mut self) -> bool {
        if !self.typed.is_empty() {
            self.typed.clear();
            true
        } else {
            self.points.pop().is_some()
        }
    }

    pub fn finish(&mut self) {
        self.accept();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(x: f64, y: f64) -> Point2 {
        Point2::new(x, y)
    }

    #[test]
    fn closes_on_first_corner_or_enter() {
        let mut t = SlabTool::default();
        for q in [p(0.0, 0.0), p(4.0, 0.0), p(4.0, 3.0)] {
            assert_eq!(t.click(q), Outcome::Nothing);
        }
        // Repeated click ignored.
        assert_eq!(t.click(p(4.0, 3.0)), Outcome::Nothing);
        assert_eq!(t.corners().len(), 3);
        t.click(p(0.0, 3.0));
        let closed = t.click(p(0.0, 0.0));
        assert_eq!(
            closed,
            Outcome::Build(vec![p(0.0, 0.0), p(4.0, 0.0), p(4.0, 3.0), p(0.0, 3.0)])
        );
        t.accept();
        assert!(!t.is_drawing());

        // Enter closes a triangle; with two corners it cannot.
        t.click(p(0.0, 0.0));
        t.click(p(2.0, 0.0));
        assert_eq!(t.enter(p(9.0, 9.0)), Outcome::Invalid);
        t.click(p(1.0, 2.0));
        assert!(matches!(t.enter(p(9.0, 9.0)), Outcome::Build(v) if v.len() == 3));
    }

    #[test]
    fn crossing_outlines_are_refused() {
        let mut t = SlabTool::default();
        for q in [p(0.0, 0.0), p(2.0, 2.0), p(2.0, 0.0), p(0.0, 2.0)] {
            t.click(q);
        }
        assert_eq!(t.click(p(0.0, 0.0)), Outcome::Invalid);
        assert!(t.is_drawing(), "the corners stay so the user can fix them");
    }

    #[test]
    fn typed_lengths_and_escape() {
        let mut t = SlabTool::default();
        t.click(p(1.0, 1.0));
        for c in "5000".chars() {
            t.type_char(c);
        }
        assert_eq!(t.enter(p(3.0, 1.0)), Outcome::Nothing);
        assert_eq!(t.anchor(), Some(p(6.0, 1.0)));
        assert!(t.typed.is_empty());
        // Escape removes the last corner, then the first, then nothing.
        assert!(t.escape());
        assert_eq!(t.anchor(), Some(p(1.0, 1.0)));
        assert!(t.escape());
        assert!(!t.escape());
    }
}
