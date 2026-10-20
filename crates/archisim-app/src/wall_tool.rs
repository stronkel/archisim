//! The wall tool, with ArchiCAD's geometry methods ("método geométrico"):
//!
//! - **Recto**: two clicks, one wall.
//! - **Cadena** (the default): every click adds a wall from the previous
//!   point, so consecutive walls share their end points and join with a
//!   mitre. Clicking the first point again closes the outline and ends the
//!   chain.
//! - **Rectángulo**: two opposite corners give four walls, drawn clockwise
//!   so the exterior faces are outside.
//! - **Arco**: start, end and a point the wall passes through (Revit's
//!   "inicio-fin-radio"); arcs chain like walls in a chain.
//! - **Círculo**: the centre and a point of the drawn line; the circle is
//!   two half circles, clockwise.
//!
//! While drawing, typing a number sets the length of the next wall along
//! the direction of the cursor, the radius (arc and circle), or the width
//! and depth of a rectangle (`6000x4000`); Enter places it.
//!
//! The tool only decides what to build; the editor builds it and reports
//! back with [`WallTool::accept`], so a refused wall leaves the tool as it
//! was.

use archisim_core::{Arc, Point2};

use crate::units::{parse_length, parse_size};

/// How the wall tool draws (ArchiCAD's geometry method).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum WallMode {
    Single,
    #[default]
    Chain,
    Rectangle,
    Arc,
    Circle,
}

/// One wall to build: its drawn line and how much it bends (0: straight).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Piece {
    pub start: Point2,
    pub end: Point2,
    pub bulge: f64,
}

impl Piece {
    pub fn straight(start: Point2, end: Point2) -> Self {
        Self {
            start,
            end,
            bulge: 0.0,
        }
    }

    pub fn arc(&self) -> Option<Arc> {
        Arc::from_bulge(self.start, self.end, self.bulge)
    }

    pub fn length(&self) -> f64 {
        self.arc()
            .map_or(self.start.distance(self.end), |a| a.length())
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct WallTool {
    /// Points so far: the chain (the last one is where the next wall
    /// starts), the first corner of a rectangle, or the centre of a circle.
    points: Vec<Point2>,
    /// Arc mode: the end of the arc being drawn, waiting for the point it
    /// passes through.
    arc_end: Option<Point2>,
    /// Length (or radius, or size) being typed, in millimetres or with a
    /// unit.
    pub typed: String,
    pub mode: WallMode,
}

/// What the tool wants done after an input.
#[derive(Debug, Clone, PartialEq)]
pub enum Outcome {
    Nothing,
    /// Build these walls (one step), then call [`WallTool::accept`].
    Build(Vec<Piece>),
}

/// What Escape did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Escape {
    ClearedLength,
    /// Took back the end of the arc being drawn.
    SteppedBack,
    EndedChain,
    /// Nothing to cancel: the application may leave the tool.
    Idle,
}

/// The four walls of a rectangle with opposite corners `a` and `b`,
/// clockwise from `a`, or `None` if it has no width or depth.
pub fn rectangle(a: Point2, b: Point2) -> Option<Vec<Piece>> {
    if (a.x - b.x).abs() < 1e-6 || (a.y - b.y).abs() < 1e-6 {
        return None;
    }
    let c = rectangle_corners(a, b);
    Some(
        (0..4)
            .map(|i| Piece::straight(c[i], c[(i + 1) % 4]))
            .collect(),
    )
}

/// The corners of the rectangle with opposite corners `a` and `b`,
/// clockwise from `a`.
pub fn rectangle_corners(a: Point2, b: Point2) -> [Point2; 4] {
    let p = Point2::new(a.x, b.y);
    let q = Point2::new(b.x, a.y);
    // Clockwise: the turn a → p → b is to the right.
    if (p - a).cross(b - p) < 0.0 {
        [a, p, b, q]
    } else {
        [a, q, b, p]
    }
}

/// The opposite corner of a rectangle of `width` × `depth` from `a`, in the
/// quadrant of `towards`.
pub fn sized_corner(a: Point2, towards: Point2, width: f64, depth: f64) -> Point2 {
    let sx = if towards.x < a.x { -1.0 } else { 1.0 };
    let sy = if towards.y < a.y { -1.0 } else { 1.0 };
    Point2::new(a.x + sx * width, a.y + sy * depth)
}

/// A circle of walls around `centre` through `p`: two half circles,
/// clockwise.
pub fn circle(centre: Point2, p: Point2) -> Option<Vec<Piece>> {
    if centre.approx_eq(p) {
        return None;
    }
    let q = centre * 2.0 - p;
    Some(vec![
        Piece {
            start: p,
            end: q,
            bulge: -1.0,
        },
        Piece {
            start: q,
            end: p,
            bulge: -1.0,
        },
    ])
}

/// The arc from `a` to `b` of radius `r`, bowing out to the side of
/// `towards` (the shorter of the two arcs), or `None` if it cannot reach.
pub fn arc_with_radius(a: Point2, b: Point2, r: f64, towards: Point2) -> Option<Piece> {
    let c = a.distance(b);
    if c < 1e-9 || r < c / 2.0 - 1e-9 {
        return None;
    }
    let sweep = 2.0 * (c / (2.0 * r)).min(1.0).asin();
    // A positive bulge bows out to the right of a → b.
    let right = (b - a).cross(towards - a) < 0.0;
    let k = (sweep / 4.0).tan();
    Some(Piece {
        start: a,
        end: b,
        bulge: if right { k } else { -k },
    })
}

impl WallTool {
    /// Where the next wall starts (chain), the first corner (rectangle) or
    /// the centre (circle), if drawing.
    pub fn anchor(&self) -> Option<Point2> {
        self.points.last().copied()
    }

    /// The end of the arc being drawn, once clicked.
    pub fn arc_end(&self) -> Option<Point2> {
        self.arc_end
    }

    /// Points placed so far (for snapping and display).
    pub fn chain(&self) -> Vec<Point2> {
        let mut v = self.points.clone();
        v.extend(self.arc_end);
        v
    }

    pub fn is_drawing(&self) -> bool {
        !self.points.is_empty()
    }

    /// The point the cursor is tracked and constrained from (orthogonal
    /// lines, Shift for 45°), if any: not while choosing an arc's bend.
    pub fn tracking_from(&self) -> Option<Point2> {
        if self.arc_end.is_some() {
            None
        } else {
            self.anchor()
        }
    }

    pub fn set_mode(&mut self, mode: WallMode) {
        if mode != self.mode {
            self.finish();
            self.mode = mode;
        }
    }

    /// What a click at `p` would build now (the preview), if anything.
    pub fn preview(&self, p: Point2) -> Vec<Piece> {
        let Some(a) = self.anchor() else {
            return Vec::new();
        };
        let p = self.typed_point(p).unwrap_or(p);
        match self.mode {
            WallMode::Single | WallMode::Chain => {
                if a.approx_eq(p) {
                    Vec::new()
                } else {
                    vec![Piece::straight(a, p)]
                }
            }
            WallMode::Rectangle => rectangle(a, p).unwrap_or_default(),
            WallMode::Circle => circle(a, p).unwrap_or_default(),
            WallMode::Arc => match self.arc_end {
                None if !a.approx_eq(p) => vec![Piece::straight(a, p)],
                None => Vec::new(),
                Some(e) => self.bend(a, e, p).into_iter().collect(),
            },
        }
    }

    /// The arc from `a` to `e` through `t` (or of the typed radius towards
    /// `t`); straight if `t` is in line.
    fn bend(&self, a: Point2, e: Point2, t: Point2) -> Option<Piece> {
        if !self.typed.is_empty() {
            return arc_with_radius(a, e, self.typed_length()?, t);
        }
        Some(match Arc::bulge_through(a, t, e) {
            Some(k) => Piece {
                start: a,
                end: e,
                bulge: k,
            },
            None => Piece::straight(a, e),
        })
    }

    /// A click at an already snapped point.
    pub fn click(&mut self, p: Point2) -> Outcome {
        let Some(a) = self.anchor() else {
            self.points.push(p);
            return Outcome::Nothing;
        };
        if a.approx_eq(p) {
            return Outcome::Nothing;
        }
        match self.mode {
            WallMode::Single | WallMode::Chain => Outcome::Build(vec![Piece::straight(a, p)]),
            WallMode::Rectangle => rectangle(a, p).map_or(Outcome::Nothing, Outcome::Build),
            WallMode::Circle => circle(a, p).map_or(Outcome::Nothing, Outcome::Build),
            WallMode::Arc => match self.arc_end {
                None => {
                    self.arc_end = Some(p);
                    self.typed.clear();
                    Outcome::Nothing
                }
                Some(e) if e.approx_eq(p) => Outcome::Nothing,
                Some(e) => self
                    .bend(a, e, p)
                    .map_or(Outcome::Nothing, |w| Outcome::Build(vec![w])),
            },
        }
    }

    /// The walls asked for were built: continue the chain from the end of
    /// the last one (or stop if it closed the outline), or start again.
    pub fn accept(&mut self, pieces: &[Piece]) {
        self.typed.clear();
        self.arc_end = None;
        let chains = matches!(self.mode, WallMode::Chain | WallMode::Arc);
        let Some(end) = pieces.last().map(|w| w.end) else {
            return;
        };
        let closes = self.points.len() > 1 && self.points[0].approx_eq(end);
        if chains && !closes {
            self.points.push(end);
        } else {
            self.points.clear();
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

    /// The point the typed value stands for, towards the cursor: the end
    /// of the next wall, the radius point of a circle or the opposite
    /// corner of a rectangle. `None` while bending an arc (the typed value
    /// is its radius) or with nothing typed.
    fn typed_point(&self, towards: Point2) -> Option<Point2> {
        match self.mode {
            WallMode::Rectangle => {
                let a = self.anchor()?;
                let (w, d) = parse_size(&self.typed)?;
                Some(sized_corner(a, towards, w, d))
            }
            WallMode::Arc if self.arc_end.is_some() => None,
            _ => self.typed_end(towards),
        }
    }

    /// A character typed while drawing. Returns whether it was used.
    pub fn type_char(&mut self, c: char) -> bool {
        let size = self.mode == WallMode::Rectangle && matches!(c, 'x' | 'X' | '*' | ';');
        let fits = c.is_ascii_digit() || matches!(c, ',' | '.' | 'm' | 'c' | 'M' | 'C') || size;
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

    /// Enter: place what was typed towards the cursor, or end the chain.
    pub fn enter(&mut self, towards: Point2) -> Outcome {
        if self.typed.is_empty() {
            self.points.clear();
            self.arc_end = None;
            return Outcome::Nothing;
        }
        if let (WallMode::Arc, Some(a), Some(e)) = (self.mode, self.anchor(), self.arc_end) {
            return self
                .bend(a, e, towards)
                .map_or(Outcome::Nothing, |w| Outcome::Build(vec![w]));
        }
        match self.typed_point(towards) {
            Some(p) => {
                self.typed.clear();
                self.click(p)
            }
            None => Outcome::Nothing,
        }
    }

    pub fn escape(&mut self) -> Escape {
        if !self.typed.is_empty() {
            self.typed.clear();
            Escape::ClearedLength
        } else if self.arc_end.take().is_some() {
            Escape::SteppedBack
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
        self.arc_end = None;
        self.typed.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(x: f64, y: f64) -> Point2 {
        Point2::new(x, y)
    }

    fn one(o: Outcome) -> Piece {
        match o {
            Outcome::Build(v) if v.len() == 1 => v[0],
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn chained_walls_and_closing() {
        let mut t = WallTool::default();
        assert_eq!(t.click(p(0.0, 0.0)), Outcome::Nothing);
        // Clicking the same point again does nothing.
        assert_eq!(t.click(p(0.0, 0.0)), Outcome::Nothing);
        let w = one(t.click(p(0.0, 4.0)));
        assert_eq!(w, Piece::straight(p(0.0, 0.0), p(0.0, 4.0)));
        t.accept(&[w]);
        let w = one(t.click(p(6.0, 4.0)));
        // A refused wall: no accept, the chain stays at (0, 4).
        assert_eq!(t.anchor(), Some(p(0.0, 4.0)));
        t.accept(&[w]);
        t.accept(&[Piece::straight(p(6.0, 4.0), p(6.0, 0.0))]);
        let w = one(t.click(p(0.0, 0.0)));
        assert_eq!(w, Piece::straight(p(6.0, 0.0), p(0.0, 0.0)));
        t.accept(&[w]);
        assert!(!t.is_drawing(), "closing the outline ends the chain");
    }

    #[test]
    fn single_walls_start_again() {
        let mut t = WallTool::default();
        t.set_mode(WallMode::Single);
        t.click(p(0.0, 0.0));
        let w = one(t.click(p(3.0, 0.0)));
        t.accept(&[w]);
        assert!(!t.is_drawing());
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
        let w = one(t.enter(p(1.0, 2.0)));
        assert_eq!(w, Piece::straight(p(1.0, 1.0), p(1.0, 4.5)));
        t.accept(&[w]);
        assert!(t.typed.is_empty());
        // Enter with nothing typed ends the chain.
        assert_eq!(t.enter(p(9.0, 9.0)), Outcome::Nothing);
        assert!(!t.is_drawing());
    }

    #[test]
    fn rectangles_go_clockwise() {
        let mut t = WallTool::default();
        t.set_mode(WallMode::Rectangle);
        t.click(p(0.0, 0.0));
        // No depth: nothing.
        assert_eq!(t.click(p(4.0, 0.0)), Outcome::Nothing);
        let Outcome::Build(walls) = t.click(p(4.0, 3.0)) else {
            panic!()
        };
        assert_eq!(walls.len(), 4);
        assert_eq!(walls[0].start, p(0.0, 0.0));
        // Clockwise: up the left side first.
        assert_eq!(walls[0].end, p(0.0, 3.0));
        let ring: Vec<Point2> = walls.iter().map(|w| w.start).collect();
        assert!(archisim_core::signed_area(&ring) < 0.0);
        for i in 0..4 {
            assert_eq!(walls[i].end, walls[(i + 1) % 4].start);
        }
        t.accept(&walls);
        assert!(!t.is_drawing());
        // Typed size, towards the cursor's quadrant.
        t.click(p(10.0, 10.0));
        for c in "6000x4000".chars() {
            assert!(t.type_char(c));
        }
        let Outcome::Build(walls) = t.enter(p(5.0, 5.0)) else {
            panic!()
        };
        let xs: Vec<f64> = walls.iter().map(|w| w.start.x).collect();
        let ys: Vec<f64> = walls.iter().map(|w| w.start.y).collect();
        assert!(xs.iter().any(|x| (*x - 4.0).abs() < 1e-9));
        assert!(ys.iter().any(|y| (*y - 6.0).abs() < 1e-9));
    }

    #[test]
    fn arcs_by_three_points_and_by_radius() {
        let mut t = WallTool::default();
        t.set_mode(WallMode::Arc);
        t.click(p(0.0, 0.0));
        assert_eq!(t.click(p(4.0, 0.0)), Outcome::Nothing);
        assert_eq!(t.arc_end(), Some(p(4.0, 0.0)));
        // Escape takes back the end.
        assert_eq!(t.escape(), Escape::SteppedBack);
        t.click(p(4.0, 0.0));
        // Through (2, -2): the half circle below.
        let w = one(t.click(p(2.0, -2.0)));
        assert!((w.bulge - 1.0).abs() < 1e-9);
        assert!((w.length() - 2.0 * std::f64::consts::PI).abs() < 1e-9);
        t.accept(&[w]);
        // Arcs chain from the end.
        assert_eq!(t.anchor(), Some(p(4.0, 0.0)));
        assert_eq!(t.arc_end(), None);
        // In line: a straight wall.
        t.click(p(8.0, 0.0));
        let w = one(t.click(p(6.0, 0.0)));
        assert_eq!(w.bulge, 0.0);
        t.accept(&[w]);
        // By radius: 2500 towards the side of the cursor.
        t.click(p(8.0, 4.0));
        for c in "2500".chars() {
            t.type_char(c);
        }
        let w = one(t.enter(p(10.0, 2.0)));
        let a = w.arc().unwrap();
        assert!((a.radius - 2.5).abs() < 1e-9);
        // It bows out towards the cursor (to the right of going north).
        assert!(a.point(0.5, 0.0).x > 8.0);
        // A radius smaller than half the span is refused.
        let mut t2 = WallTool {
            mode: WallMode::Arc,
            ..Default::default()
        };
        t2.click(p(0.0, 0.0));
        t2.click(p(4.0, 0.0));
        t2.type_char('1');
        assert_eq!(t2.enter(p(2.0, 1.0)), Outcome::Nothing);
    }

    #[test]
    fn circles_are_two_half_circles_clockwise() {
        let mut t = WallTool::default();
        t.set_mode(WallMode::Circle);
        t.click(p(1.0, 1.0));
        let Outcome::Build(walls) = t.click(p(4.0, 1.0)) else {
            panic!()
        };
        assert_eq!(walls.len(), 2);
        for w in &walls {
            let a = w.arc().unwrap();
            assert!(a.centre.approx_eq(p(1.0, 1.0)));
            assert!((a.radius - 3.0).abs() < 1e-9);
            assert!(a.sweep < 0.0, "clockwise");
        }
        assert_eq!(walls[0].end, walls[1].start);
        assert_eq!(walls[1].end, walls[0].start);
        t.accept(&walls);
        assert!(!t.is_drawing());
        // Typed radius.
        t.click(p(0.0, 0.0));
        t.type_char('2');
        t.type_char('m');
        let Outcome::Build(walls) = t.enter(p(0.0, 5.0)) else {
            panic!()
        };
        assert!(walls[0].start.approx_eq(p(0.0, 2.0)));
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
