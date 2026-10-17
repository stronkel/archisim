//! The wall tool: click the start, click the end, and keep going.
//!
//! Like ArchiCAD's chained wall: every click adds a wall from the previous
//! point, so consecutive walls share their end points and join with a mitre.
//! Clicking the first point again closes the outline and ends the chain.
//! While drawing, typing a number sets the length of the next wall along
//! the direction of the cursor (Enter to place it).
//!
//! The tool only decides what to build; the editor builds it and reports
//! back with [`WallTool::accept`], so a refused wall leaves the chain as it
//! was.

use archisim_core::Point2;

use crate::units::parse_length;

#[derive(Debug, Clone, Default, PartialEq)]
pub struct WallTool {
    /// Points of the chain so far; the last one is where the next wall starts.
    points: Vec<Point2>,
    /// Length being typed, in millimetres (or with a unit).
    pub typed: String,
}

/// What the tool wants done after an input.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Outcome {
    Nothing,
    /// Build a wall between these points, then call [`WallTool::accept`].
    Build(Point2, Point2),
}

/// What Escape did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Escape {
    ClearedLength,
    EndedChain,
    /// Nothing to cancel: the application may leave the tool.
    Idle,
}

impl WallTool {
    /// Where the next wall starts, if a chain is in progress.
    pub fn anchor(&self) -> Option<Point2> {
        self.points.last().copied()
    }

    /// Points of the chain being drawn (for snapping and display).
    pub fn chain(&self) -> &[Point2] {
        &self.points
    }

    pub fn is_drawing(&self) -> bool {
        !self.points.is_empty()
    }

    /// A click at an already snapped point.
    pub fn click(&mut self, p: Point2) -> Outcome {
        match self.anchor() {
            None => {
                self.points.push(p);
                Outcome::Nothing
            }
            Some(a) if a.approx_eq(p) => Outcome::Nothing,
            Some(a) => Outcome::Build(a, p),
        }
    }

    /// The wall from the anchor to `end` was built: continue from `end`, or
    /// stop if it closed the outline.
    pub fn accept(&mut self, end: Point2) {
        self.typed.clear();
        let closes = self.points.len() > 1 && self.points[0].approx_eq(end);
        if closes {
            self.points.clear();
        } else {
            self.points.push(end);
        }
    }

    /// The typed length in metres, if it is a positive number.
    pub fn typed_length(&self) -> Option<f64> {
        parse_length(&self.typed).filter(|l| *l > 0.0)
    }

    /// The end point the typed length gives towards `towards`.
    pub fn typed_end(&self, towards: Point2) -> Option<Point2> {
        let a = self.anchor()?;
        let len = self.typed_length()?;
        let dir = (towards - a).normalized()?;
        Some(a + dir * len)
    }

    /// A character typed while drawing. Returns whether it was used.
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

    /// Enter: place the typed length towards the cursor, or end the chain.
    pub fn enter(&mut self, towards: Point2) -> Outcome {
        if self.typed.is_empty() {
            self.points.clear();
            return Outcome::Nothing;
        }
        match (self.anchor(), self.typed_end(towards)) {
            (Some(a), Some(end)) => Outcome::Build(a, end),
            _ => Outcome::Nothing,
        }
    }

    pub fn escape(&mut self) -> Escape {
        if !self.typed.is_empty() {
            self.typed.clear();
            Escape::ClearedLength
        } else if self.is_drawing() {
            self.points.clear();
            Escape::EndedChain
        } else {
            Escape::Idle
        }
    }

    /// Ends the chain without building anything (right click, tool change).
    pub fn finish(&mut self) {
        self.points.clear();
        self.typed.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(x: f64, y: f64) -> Point2 {
        Point2::new(x, y)
    }

    #[test]
    fn chained_walls_and_closing() {
        let mut t = WallTool::default();
        assert_eq!(t.click(p(0.0, 0.0)), Outcome::Nothing);
        // Clicking the same point again does nothing.
        assert_eq!(t.click(p(0.0, 0.0)), Outcome::Nothing);
        assert_eq!(
            t.click(p(0.0, 4.0)),
            Outcome::Build(p(0.0, 0.0), p(0.0, 4.0))
        );
        t.accept(p(0.0, 4.0));
        assert_eq!(
            t.click(p(6.0, 4.0)),
            Outcome::Build(p(0.0, 4.0), p(6.0, 4.0))
        );
        // A refused wall: no accept, the chain stays at (0, 4).
        assert_eq!(t.anchor(), Some(p(0.0, 4.0)));
        t.accept(p(6.0, 4.0));
        t.accept(p(6.0, 0.0));
        assert_eq!(
            t.click(p(0.0, 0.0)),
            Outcome::Build(p(6.0, 0.0), p(0.0, 0.0))
        );
        t.accept(p(0.0, 0.0));
        assert!(!t.is_drawing(), "closing the outline ends the chain");
    }

    #[test]
    fn typed_length_follows_the_cursor_direction() {
        let mut t = WallTool::default();
        assert!(!t.type_char('3'), "nothing typed before the first point");
        t.click(p(1.0, 1.0));
        for c in "3500".chars() {
            assert!(t.type_char(c));
        }
        assert!(!t.type_char('x'));
        assert_eq!(t.typed_length(), Some(3.5));
        // Cursor anywhere to the north: the wall goes 3.5 m north.
        assert_eq!(
            t.enter(p(1.0, 2.0)),
            Outcome::Build(p(1.0, 1.0), p(1.0, 4.5))
        );
        t.accept(p(1.0, 4.5));
        assert!(t.typed.is_empty());
        // Enter with nothing typed ends the chain.
        assert_eq!(t.enter(p(9.0, 9.0)), Outcome::Nothing);
        assert!(!t.is_drawing());
    }

    #[test]
    fn escape_steps_back() {
        let mut t = WallTool::default();
        assert_eq!(t.escape(), Escape::Idle);
        t.click(p(0.0, 0.0));
        t.type_char('2');
        assert!(t.backspace());
        assert!(!t.backspace());
        t.type_char('2');
        assert_eq!(t.escape(), Escape::ClearedLength);
        assert_eq!(t.escape(), Escape::EndedChain);
        assert_eq!(t.escape(), Escape::Idle);
    }
}
