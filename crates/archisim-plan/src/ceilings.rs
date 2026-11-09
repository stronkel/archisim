//! False ceilings (M7) in a reflected ceiling plan: its outline over a light
//! tint, its holes crossed, a grid of its boards when its type says it is
//! one of panels («registrable», 60 × 60) and, high in it, the height of
//! its face over the level («FT +2,50»). Floor plans do not show them: they
//! are above the cut.

use std::fmt::Write as _;

use archisim_core::{Ceiling, LevelId, Model, Point2};

use crate::lettering::{text_path, Align};
use crate::{bounds, clip_segment, num, path, pens, pt, INK};

/// Light tint of a false ceiling seen from below.
const CEILING_TINT: &str = "#f4f1ea";
/// Height of its label, mm of paper.
const LABEL: f64 = 2.0;
/// Side of the boards of a ceiling of panels.
const PANEL: f64 = 0.6;

/// Whether a ceiling's type is one of panels laid in a grid.
fn of_panels(model: &Model, c: &Ceiling) -> bool {
    model.slab_type(c.ceiling_type).is_some_and(|t| {
        let n = t.name.to_lowercase();
        n.contains("registrable") || n.contains("60×60") || n.contains("60x60")
    })
}

/// «FT +2,50»: its face over the level, with two decimals.
pub fn ceiling_label(c: &Ceiling) -> String {
    let sign = if c.height < 0.0 { "−" } else { "+" };
    format!("FT {sign}{:.2}", c.height.abs()).replace('.', ",")
}

/// The false ceilings of a level, as in its ceiling plan.
pub fn draw_ceilings(s: &mut String, model: &Model, level: LevelId, k: f64) {
    for c in model.ceilings_on_level(level) {
        crate::dim::element(s, c.id.0, |s| draw_ceiling(s, model, c, k));
    }
}

fn draw_ceiling(s: &mut String, model: &Model, c: &Ceiling, k: f64) {
    let mut d = path(&c.outline);
    for h in &c.holes {
        d.push(' ');
        d.push_str(&path(h));
    }
    let _ = writeln!(
        s,
        r#"<path d="{d}" fill="{CEILING_TINT}" fill-rule="evenodd" stroke="{INK}" stroke-width="{}" stroke-linejoin="round"/>"#,
        num(pens().view * k)
    );
    // The grid of its boards, from the corner of its box.
    if of_panels(model, c) {
        let (lo, hi) = bounds(&c.outline);
        let mut grid = String::new();
        let mut line = |a: Point2, b: Point2| {
            for (p, q) in clip_segment(a, b, &c.outline) {
                let _ = write!(grid, "M{} L{} ", pt(p), pt(q));
            }
        };
        let mut x = lo.x + PANEL;
        while x < hi.x - 1e-6 {
            line(Point2::new(x, lo.y), Point2::new(x, hi.y));
            x += PANEL;
        }
        let mut y = lo.y + PANEL;
        while y < hi.y - 1e-6 {
            line(Point2::new(lo.x, y), Point2::new(hi.x, y));
            y += PANEL;
        }
        if !grid.is_empty() {
            let _ = writeln!(
                s,
                r#"<path d="{}" fill="none" stroke="{INK}" stroke-width="{}"/>"#,
                grid.trim_end(),
                num(pens().hatch * k)
            );
        }
    }
    // A cross in each hole.
    for h in &c.holes {
        let (lo, hi) = bounds(h);
        let mut x = String::new();
        for (a, b) in [(lo, hi), (Point2::new(lo.x, hi.y), Point2::new(hi.x, lo.y))] {
            for (p, q) in clip_segment(a, b, h) {
                let _ = write!(x, "M{} L{} ", pt(p), pt(q));
            }
        }
        let _ = writeln!(
            s,
            r#"<path d="{}" fill="none" stroke="{INK}" stroke-width="{}"/>"#,
            x.trim_end(),
            num(pens().hatch * k)
        );
    }
    // Its height, high in its box, clear of the room's name in the middle
    // (inside it if it is not convex).
    let (lo, hi) = bounds(&c.outline);
    let mut at = Point2::new((lo.x + hi.x) * 0.5, lo.y + (hi.y - lo.y) * 0.8);
    if !archisim_core::point_in_polygon(at, &c.outline) {
        at = c.outline[0];
    }
    let h = LABEL * k;
    let _ = writeln!(
        s,
        r#"<path d="{}" {}/>"#,
        text_path(
            &ceiling_label(c),
            at - Point2::new(0.0, h / 2.0),
            Point2::new(1.0, 0.0),
            h,
            Align::Centre
        ),
        crate::lettering::paint(h, INK)
    );
}
