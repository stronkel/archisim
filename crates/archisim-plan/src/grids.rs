//! Setting-out grid lines and footings in plan and section.
//!
//! A grid line is drawn on every level as a thin dash-dot line with a
//! bubble at each end holding its name; in a section, every grid line its
//! line crosses is a vertical dash-dot line with its bubble on top.
//!
//! Footings are under the floor: on their level's plan they are drawn
//! dashed, as hidden lines.

use std::fmt::Write as _;

use archisim_core::{LevelId, Model, Point2};

use crate::lettering::{text_path, Align};
use crate::{num, pens, pt, INK};

/// Radius of a grid bubble, paper millimetres.
pub const BUBBLE_RADIUS: f64 = 4.0;
/// Height of the name in a bubble, paper millimetres.
const NAME_SIZE: f64 = 3.5;

fn dash_dot(k: f64) -> String {
    format!(
        r#" stroke-dasharray="{} {} {} {}""#,
        num(12.0 * k),
        num(1.5 * k),
        num(1.5 * k),
        num(1.5 * k)
    )
}

/// A bubble with a name, centred at `c`.
fn bubble(s: &mut String, c: Point2, name: &str, k: f64) {
    let r = BUBBLE_RADIUS * k;
    let _ = writeln!(
        s,
        r#"<circle cx="{}" cy="{}" r="{}" fill="white" stroke="{INK}" stroke-width="{}"/>"#,
        num(c.x),
        num(-c.y),
        num(r),
        num(pens().view * k)
    );
    let h = NAME_SIZE * k;
    let _ = writeln!(
        s,
        r#"<path d="{}" {}/>"#,
        text_path(
            name,
            c - Point2::new(0.0, h / 2.0),
            Point2::new(1.0, 0.0),
            h,
            Align::Centre
        ),
        crate::lettering::paint(h, INK)
    );
}

/// The grid lines of the project, on any plan.
pub fn draw_grids(s: &mut String, model: &Model, k: f64) {
    if model.grids().next().is_none() {
        return;
    }
    let r = BUBBLE_RADIUS * k;
    s.push_str("<g id=\"grids\">\n");
    for g in model.grids() {
        let Some(u) = (g.end - g.start).normalized() else {
            continue;
        };
        let _ = writeln!(
            s,
            r#"<path d="M{} L{}" fill="none" stroke="{INK}" stroke-width="{}"{}/>"#,
            pt(g.start),
            pt(g.end),
            num(pens().thin * k),
            dash_dot(k)
        );
        bubble(s, g.start - u * r, &g.name, k);
        bubble(s, g.end + u * r, &g.name, k);
    }
    s.push_str("</g>\n");
}

/// The points of the grid lines (for the extent of a plan).
pub fn grid_points(model: &Model, k: f64) -> Vec<Point2> {
    let r = 2.0 * BUBBLE_RADIUS * k;
    model
        .grids()
        .flat_map(|g| {
            let u = (g.end - g.start)
                .normalized()
                .unwrap_or(Point2::new(1.0, 0.0));
            [g.start - u * r, g.end + u * r]
        })
        .collect()
}

/// Where a section line from `a` to `b` crosses each grid line: the
/// distance along the section and the grid's name.
pub fn grids_crossing(model: &Model, a: Point2, b: Point2) -> Vec<(f64, String)> {
    let d = b - a;
    let len = d.length();
    if len < 1e-9 {
        return Vec::new();
    }
    let mut out = Vec::new();
    for g in model.grids() {
        let e = g.end - g.start;
        let den = d.x * e.y - d.y * e.x;
        if den.abs() < 1e-12 {
            continue;
        }
        let w = g.start - a;
        let t = (w.x * e.y - w.y * e.x) / den;
        let s = (w.x * d.y - w.y * d.x) / den;
        if (0.0..=1.0).contains(&s) {
            out.push((t * len, g.name.clone()));
        }
    }
    out
}

/// The grid lines a section crosses: vertical dash-dot lines from `lo` to
/// `hi` (section coordinates) with their bubbles on top.
pub fn draw_section_grids(s: &mut String, crossings: &[(f64, String)], lo: f64, hi: f64, k: f64) {
    if crossings.is_empty() {
        return;
    }
    let r = BUBBLE_RADIUS * k;
    s.push_str("<g id=\"grids\">\n");
    for (x, name) in crossings {
        let _ = writeln!(
            s,
            r#"<path d="M{} L{}" fill="none" stroke="{INK}" stroke-width="{}"{}/>"#,
            pt(Point2::new(*x, lo)),
            pt(Point2::new(*x, hi)),
            num(pens().thin * k),
            dash_dot(k)
        );
        bubble(s, Point2::new(*x, hi + r), name, k);
    }
    s.push_str("</g>\n");
}

/// The footings of a level, dashed: they are under the floor.
pub fn draw_footings(
    s: &mut String,
    model: &Model,
    level: LevelId,
    k: f64,
    visible: &dyn Fn(&[Point2]) -> bool,
) {
    if model.footings_on_level(level).next().is_none() {
        return;
    }
    s.push_str("<g id=\"footings\">\n");
    for f in model.footings_on_level(level) {
        let o = f.outline();
        if o.len() < 3 || !visible(&o) {
            continue;
        }
        crate::dim::element(s, f.id.0, |s| {
            let _ = writeln!(
                s,
                r#"<path d="{}" fill="none" stroke="{INK}" stroke-width="{}" stroke-dasharray="{} {}"/>"#,
                crate::path(&o),
                num(pens().view * k),
                num(3.0 * k),
                num(1.5 * k)
            );
        });
    }
    s.push_str("</g>\n");
}

#[cfg(test)]
mod tests {
    use super::*;
    use archisim_core::NewGrid;

    #[test]
    fn sections_find_the_grids_they_cross() {
        let mut m = Model::new("Ejes");
        for (name, x) in [("1", 0.0), ("2", 5.0)] {
            m.add_grid(NewGrid {
                name: name.into(),
                start: Point2::new(x, -2.0),
                end: Point2::new(x, 10.0),
            })
            .unwrap();
        }
        // Parallel to the section: not crossed.
        m.add_grid(NewGrid {
            name: "A".into(),
            start: Point2::new(-2.0, 0.0),
            end: Point2::new(8.0, 0.0),
        })
        .unwrap();
        let c = grids_crossing(&m, Point2::new(-1.0, 4.0), Point2::new(9.0, 4.0));
        assert_eq!(c.len(), 2);
        assert!((c[0].0 - 1.0).abs() < 1e-9 && c[0].1 == "1");
        assert!((c[1].0 - 6.0).abs() < 1e-9 && c[1].1 == "2");
        let mut svg = String::new();
        draw_grids(&mut svg, &m, 0.05);
        assert_eq!(svg.matches("<circle").count(), 6);
    }
}
