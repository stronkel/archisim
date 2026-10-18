//! The wall tool: click the start, click the end, and keep going.
//!
//! Modes (like ArchiCAD's construction methods):
//! - **Chain** (default): every click adds a wall from the previous point.
//! - **Rectangle**: two opposite corners define four walls.
//! - **Arc**: start, end and a point the arc passes through.
//! - **Circle**: centre and a point on the circumference.
//!
//! While drawing, typing a number sets the length of the next wall along
//! the direction of the cursor (Enter to place it).

use archisim_core::Point2;

use crate::units::parse_length;

/// How the wall tool builds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum WallMode {
    #[default]
    Chain,
    Rectangle,
    Arc,
    Circle,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct WallTool {
    pub mode: WallMode,
    /// Points collected so far for the current construction.
    points: Vec<Point2>,
    /// Length being typed, in millimetres (or with a unit).
    pub typed: String,
}

/// What the tool wants done after an input.
#[derive(Debug, Clone, PartialEq)]
pub enum Outcome {
    Nothing,
    /// Build a straight wall between these points.
    Build(Point2, Point2),
    /// Build an arc wall (start, end, through).
    BuildArc(Point2, Point2, Point2),
    /// Build a rectangle of four walls from two opposite corners.
    BuildRectangle(Point2, Point2),
    /// Build a circular wall (centre, radius point).
    BuildCircle(Point2, Point2),
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
        match self.mode {
            WallMode::Chain => self.click_chain(p),
            WallMode::Rectangle => self.click_rectangle(p),
            WallMode::Arc => self.click_arc(p),
            WallMode::Circle => self.click_circle(p),
        }
    }

    fn click_chain(&mut self, p: Point2) -> Outcome {
        match self.anchor() {
            None => {
                self.points.push(p);
                Outcome::Nothing
            }
            Some(a) if a.approx_eq(p) => Outcome::Nothing,
            Some(a) => Outcome::Build(a, p),
        }
    }

    fn click_rectangle(&mut self, p: Point2) -> Outcome {
        match self.points.len() {
            0 => {
                self.points.push(p);
                Outcome::Nothing
            }
            1 => {
                let a = self.points[0];
                self.points.clear();
                Outcome::BuildRectangle(a, p)
            }
            _ => Outcome::Nothing,
        }
    }

    fn click_arc(&mut self, p: Point2) -> Outcome {
        match self.points.len() {
            0 | 1 => {
                self.points.push(p);
                Outcome::Nothing
            }
            2 => {
                let a = self.points[0];
                let b = self.points[1];
                self.points.clear();
                Outcome::BuildArc(a, b, p)
            }
            _ => Outcome::Nothing,
        }
    }

    fn click_circle(&mut self, p: Point2) -> Outcome {
        match self.points.len() {
            0 => {
                self.points.push(p);
                Outcome::Nothing
            }
            1 => {
                let c = self.points[0];
                self.points.clear();
                Outcome::BuildCircle(c, p)
            }
            _ => Outcome::Nothing,
        }
    }

    /// The wall from the anchor to `end` was built: continue from `end`, or
    /// stop if it closed the outline.
    pub fn accept(&mut self, end: Point2) {
        self.typed.clear();
        if self.mode != WallMode::Chain {
            self.points.clear();
            return;
        }
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

    /// Backspace while typing a length.
    pub fn backspace(&mut self) -> bool {
        if self.typed.is_empty() {
            false
        } else {
            self.typed.pop();
            true
        }
    }

    /// Enter: place the typed length, or finish a rectangle/arc/circle with the current point.
    pub fn enter(&mut self, towards: Point2) -> Outcome {
        if let Some(end) = self.typed_end(towards) {
            return self.click(end);
        }
        self.click(towards)
    }

    /// Finish the current construction (e.g. on tool change).
    pub fn finish(&mut self) {
        self.points.clear();
        self.typed.clear();
    }

    /// Escape: clear a typed length, or end the current construction.
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
}