//! The sans lettering: IBM Plex Sans (SIL Open Font License, in
//! `fonts/`), drawn as filled outlines. The same outlines on screen, in
//! SVG, PDF and DXF, whatever fonts the computer has. Heights are cap
//! heights, as in the technical lettering, so a text keeps its size when
//! the lettering changes.

use std::sync::OnceLock;

use archisim_core::Point2;

static FONT: &[u8] = include_bytes!("../fonts/IBMPlexSans-Regular.ttf");

fn face() -> &'static ttf_parser::Face<'static> {
    static FACE: OnceLock<ttf_parser::Face<'static>> = OnceLock::new();
    FACE.get_or_init(|| ttf_parser::Face::parse(FONT, 0).expect("IBM Plex Sans"))
}

/// Font units per cap height.
fn cap() -> f64 {
    let f = face();
    f.capital_height()
        .filter(|c| *c > 0)
        .map_or(f.units_per_em() as f64 * 0.7, |c| c as f64)
}

/// A piece of an outline, in drawing coordinates.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Seg {
    Move(Point2),
    Line(Point2),
    Quad(Point2, Point2),
    Cubic(Point2, Point2, Point2),
    Close,
}

/// Width of one line of text of cap height `h`.
pub fn width(text: &str, h: f64) -> f64 {
    let f = face();
    let s = h / cap();
    text.chars()
        .map(|c| {
            f.glyph_index(c)
                .and_then(|g| f.glyph_hor_advance(g))
                .unwrap_or(0) as f64
                * s
        })
        .sum()
}

struct Builder<'a, F: Fn(f64, f64) -> Point2> {
    out: &'a mut Vec<Seg>,
    place: F,
    x: f64,
    s: f64,
}

impl<F: Fn(f64, f64) -> Point2> Builder<'_, F> {
    fn p(&self, x: f32, y: f32) -> Point2 {
        (self.place)(self.x + x as f64 * self.s, y as f64 * self.s)
    }
}

impl<F: Fn(f64, f64) -> Point2> ttf_parser::OutlineBuilder for Builder<'_, F> {
    fn move_to(&mut self, x: f32, y: f32) {
        let p = self.p(x, y);
        self.out.push(Seg::Move(p));
    }
    fn line_to(&mut self, x: f32, y: f32) {
        let p = self.p(x, y);
        self.out.push(Seg::Line(p));
    }
    fn quad_to(&mut self, x1: f32, y1: f32, x: f32, y: f32) {
        let (c, p) = (self.p(x1, y1), self.p(x, y));
        self.out.push(Seg::Quad(c, p));
    }
    fn curve_to(&mut self, x1: f32, y1: f32, x2: f32, y2: f32, x: f32, y: f32) {
        let (a, b, p) = (self.p(x1, y1), self.p(x2, y2), self.p(x, y));
        self.out.push(Seg::Cubic(a, b, p));
    }
    fn close(&mut self) {
        self.out.push(Seg::Close);
    }
}

/// The outlines of one line of text of cap height `h`, each point placed
/// by `place(x along the baseline from its start, y up)`.
pub fn line(text: &str, h: f64, place: impl Fn(f64, f64) -> Point2) -> Vec<Seg> {
    let f = face();
    let s = h / cap();
    let mut out = Vec::new();
    let mut b = Builder {
        out: &mut out,
        place,
        x: 0.0,
        s,
    };
    for c in text.chars() {
        let Some(g) = f.glyph_index(c) else {
            continue;
        };
        f.outline_glyph(g, &mut b);
        b.x += f.glyph_hor_advance(g).unwrap_or(0) as f64 * s;
    }
    out
}

/// The outlines flattened to closed polylines (for DXF and hit tests):
/// curves cut into `steps` straight pieces.
pub fn polylines(segs: &[Seg], steps: usize) -> Vec<Vec<Point2>> {
    let mut out: Vec<Vec<Point2>> = Vec::new();
    let mut cur: Vec<Point2> = Vec::new();
    let steps = steps.max(1);
    for s in segs {
        match *s {
            Seg::Move(p) => {
                if cur.len() > 1 {
                    out.push(std::mem::take(&mut cur));
                }
                cur = vec![p];
            }
            Seg::Line(p) => cur.push(p),
            Seg::Quad(c, p) => {
                let a = *cur.last().unwrap_or(&p);
                for i in 1..=steps {
                    let t = i as f64 / steps as f64;
                    let u = 1.0 - t;
                    cur.push(a * (u * u) + c * (2.0 * u * t) + p * (t * t));
                }
            }
            Seg::Cubic(c1, c2, p) => {
                let a = *cur.last().unwrap_or(&p);
                for i in 1..=steps {
                    let t = i as f64 / steps as f64;
                    let u = 1.0 - t;
                    cur.push(
                        a * (u * u * u)
                            + c1 * (3.0 * u * u * t)
                            + c2 * (3.0 * u * t * t)
                            + p * (t * t * t),
                    );
                }
            }
            Seg::Close => {
                if let Some(&first) = cur.first() {
                    cur.push(first);
                }
                if cur.len() > 1 {
                    out.push(std::mem::take(&mut cur));
                }
            }
        }
    }
    if cur.len() > 1 {
        out.push(cur);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sans_outlines_have_the_cap_height() {
        let segs = line("H", 1.0, Point2::new);
        let pts: Vec<Point2> = polylines(&segs, 4).into_iter().flatten().collect();
        let top = pts.iter().map(|p| p.y).fold(f64::MIN, f64::max);
        let bottom = pts.iter().map(|p| p.y).fold(f64::MAX, f64::min);
        assert!(
            (top - 1.0).abs() < 0.02 && bottom.abs() < 0.02,
            "{bottom} {top}"
        );
        // Every Spanish character has an outline.
        for c in "ÁÉÍÓÚÑñáéíóúüç¿¡ºª²€·…«»0123456789".chars() {
            assert!(!line(&c.to_string(), 1.0, Point2::new).is_empty(), "{c}");
        }
        assert!(width("Salón", 2.5) > width("Sal", 2.5));
    }
}
