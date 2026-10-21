//! Drafting elements of a view in SVG: lines, polylines, circles, arcs,
//! ellipses, points, texts and hatches, each with its colour, weight and
//! line style (its own or its layer's).

use std::fmt::Write as _;

use archisim_core::{Draft, Model, Point2, Shape, ViewRef};
use archisim_draft::{ellipse_points, hatch, polyline_segs, Seg, CURVE_TOL};

use crate::{dash_attr, num, pt};

fn hex(c: [u8; 3]) -> String {
    format!("#{:02x}{:02x}{:02x}", c[0], c[1], c[2])
}

/// SVG path data of a list of segments, in plan coordinates (y flipped).
pub fn segs_path(segs: &[Seg], close: bool) -> String {
    let mut d = String::new();
    let mut last: Option<Point2> = None;
    for s in segs {
        let a = s.start();
        if last.is_none_or(|l| !l.approx_eq(a)) {
            let _ = write!(d, "M{} ", pt(a));
        }
        match *s {
            Seg::Line { b, .. } => {
                let _ = write!(d, "L{} ", pt(b));
            }
            Seg::Arc { radius, sweep, .. } => {
                if sweep.abs() >= std::f64::consts::TAU - 1e-9 {
                    // A full circle in two halves.
                    let m = s.point_at(0.5);
                    let flag = if sweep > 0.0 { 0 } else { 1 };
                    let _ = write!(
                        d,
                        "A{r},{r} 0 0 {flag} {} A{r},{r} 0 0 {flag} {} ",
                        pt(m),
                        pt(s.end()),
                        r = num(radius)
                    );
                } else {
                    let large = if sweep.abs() > std::f64::consts::PI {
                        1
                    } else {
                        0
                    };
                    // y is flipped: counter-clockwise in plan is clockwise
                    // on the SVG page (sweep flag 0).
                    let flag = if sweep > 0.0 { 0 } else { 1 };
                    let _ = write!(
                        d,
                        "A{r},{r} 0 {large} {flag} {} ",
                        pt(s.end()),
                        r = num(radius)
                    );
                }
            }
        }
        last = Some(s.end());
    }
    if close {
        d.push('Z');
    }
    d.trim_end().to_string()
}

fn points_path(pts: &[Point2], close: bool) -> String {
    let mut d = String::new();
    for (i, p) in pts.iter().enumerate() {
        let _ = write!(d, "{}{} ", if i == 0 { "M" } else { "L" }, pt(*p));
    }
    if close {
        d.push('Z');
    }
    d.trim_end().to_string()
}

/// Whether a shape's bounds touch the region drawn (`None`: everything).
fn in_view(shape: &Shape, k: f64, viewport: Option<(Point2, Point2)>) -> bool {
    let Some((lo, hi)) = viewport else {
        return true;
    };
    let (a, b) = archisim_draft::bounds(shape, k);
    !(a.x > hi.x || a.y > hi.y || b.x < lo.x || b.y < lo.y)
}

/// Draws the drafting elements of a view. `k`: drawing units per paper
/// millimetre. `printing`: leave out layers that are not printed. Hatches
/// go first so lines and texts are drawn over them. `hatches`: only the
/// hatches (`Some(true)`, drawn under the elements, as pavings), only the
/// rest (`Some(false)`) or everything.
#[allow(clippy::too_many_arguments)]
pub fn draw_drafts(
    s: &mut String,
    model: &Model,
    view: ViewRef,
    k: f64,
    viewport: Option<(Point2, Point2)>,
    printing: bool,
    hidden_layers: &std::collections::BTreeSet<archisim_core::DrawLayerId>,
    hide_dims: bool,
    hatches: Option<bool>,
) {
    let shown: Vec<&Draft> = model
        .drafts_in(view)
        .filter(|d| hatches.is_none_or(|h| h == matches!(d.shape, Shape::Hatch(_))))
        .filter(|d| model.draft_visible(d) && !hidden_layers.contains(&d.layer))
        .filter(|d| !(hide_dims && matches!(d.shape, Shape::Dim(_))))
        .filter(|d| !printing || model.draw_layer(d.layer).is_none_or(|l| l.printable))
        .filter(|d| in_view(&d.shape, k, viewport))
        .collect();
    if shown.is_empty() {
        return;
    }
    s.push_str("<g id=\"drafting\" fill=\"none\" stroke-linejoin=\"round\">\n");
    for d in shown.iter().filter(|d| matches!(d.shape, Shape::Hatch(_))) {
        draw_draft(s, model, d, k);
    }
    for d in shown.iter().filter(|d| !matches!(d.shape, Shape::Hatch(_))) {
        draw_draft(s, model, d, k);
    }
    s.push_str("</g>\n");
}

/// Draws one drafting element.
pub fn draw_draft(s: &mut String, model: &Model, d: &Draft, k: f64) {
    let look = model.draft_look(d);
    let color = hex(look.color);
    let width = num(look.weight * k);
    let dash = dash_attr(look.line, k);
    // Dotted styles bring their own round caps.
    let cap = if dash.contains("stroke-linecap") {
        ""
    } else {
        r#" stroke-linecap="round""#
    };
    let stroke = |s: &mut String, path: &str| {
        let _ = writeln!(
            s,
            r#"<path d="{path}" stroke="{color}" stroke-width="{width}"{cap}{dash}/>"#
        );
    };
    match &d.shape {
        Shape::Line { a, b } => stroke(s, &format!("M{} L{}", pt(*a), pt(*b))),
        Shape::Polyline(p) => {
            let segs = polyline_segs(p);
            let path = segs_path(&segs, p.closed);
            if p.width > 0.0 {
                let _ = writeln!(
                    s,
                    r#"<path d="{path}" stroke="{color}" stroke-width="{}" stroke-linejoin="miter"{dash}/>"#,
                    num(p.width)
                );
            } else {
                stroke(s, &path);
            }
        }
        Shape::Circle { centre, radius } => {
            let seg = Seg::Arc {
                centre: *centre,
                radius: *radius,
                start: 0.0,
                sweep: std::f64::consts::TAU,
            };
            stroke(s, &segs_path(&[seg], true));
        }
        Shape::Arc {
            centre,
            radius,
            start,
            sweep,
        } => {
            let seg = Seg::Arc {
                centre: *centre,
                radius: *radius,
                start: *start,
                sweep: *sweep,
            };
            stroke(s, &segs_path(&[seg], false));
        }
        Shape::Ellipse {
            centre,
            axis,
            ratio,
            start,
            sweep,
        } => {
            let pts = ellipse_points(*centre, *axis, *ratio, *start, *sweep, CURVE_TOL);
            let full = *sweep >= std::f64::consts::TAU - 1e-9;
            stroke(s, &points_path(&pts, full));
        }
        Shape::Point { at } => {
            // A small cross with a dot, 1 mm on paper.
            let r = 0.5 * k;
            let _ = writeln!(
                s,
                r#"<path d="M{} L{} M{} L{}" stroke="{color}" stroke-width="{width}"/>"#,
                pt(*at - Point2::new(r, 0.0)),
                pt(*at + Point2::new(r, 0.0)),
                pt(*at - Point2::new(0.0, r)),
                pt(*at + Point2::new(0.0, r)),
            );
        }
        Shape::Text(t) => {
            let mut path = String::new();
            for st in archisim_draft::font::text_strokes(t, k) {
                path.push_str(&points_path(&st, false));
                path.push(' ');
            }
            let _ = writeln!(
                s,
                r#"<path d="{}" stroke="{color}" stroke-width="{}" stroke-linecap="round"/>"#,
                path.trim_end(),
                num(archisim_draft::font::STROKE * t.height * k)
            );
        }
        Shape::Hatch(h) => draw_hatch(s, h, &color, &width, k),
        Shape::Dim(m) => {
            let dd = archisim_draft::dims::drawing(m, k, crate::pens().text);
            stroke(s, &segs_path(&dd.segs, false));
            let mut arrows = String::new();
            for a in &dd.arrows {
                arrows.push_str(&points_path(a, true));
                arrows.push(' ');
            }
            if !arrows.is_empty() {
                let _ = writeln!(
                    s,
                    r#"<path d="{}" fill="{color}" stroke="none"/>"#,
                    arrows.trim_end()
                );
            }
            let mut path = String::new();
            for st in archisim_draft::font::text_strokes(&dd.text, k) {
                path.push_str(&points_path(&st, false));
                path.push(' ');
            }
            let _ = writeln!(
                s,
                r#"<path d="{}" stroke="{color}" stroke-width="{}" stroke-linecap="round"/>"#,
                path.trim_end(),
                num(archisim_draft::font::STROKE * dd.text.height * k)
            );
        }
    }
}

/// Draws a hatch: its background, its bands, its pattern lines (or a
/// solid fill) in `color`, lines `width` wide.
pub(crate) fn draw_hatch(
    s: &mut String,
    h: &archisim_core::Hatch,
    color: &str,
    width: &str,
    k: f64,
) {
    if h.opacity < 0.999 {
        // See-through: every path of the hatch at its opacity.
        let mut t = String::new();
        draw_hatch_paths(&mut t, h, color, width, k);
        let a = num(h.opacity.clamp(0.05, 1.0));
        s.push_str(&t.replace(
            "/>\n",
            &format!(r#" fill-opacity="{a}" stroke-opacity="{a}"/>"#).replace("/>", "/>\n"),
        ));
        return;
    }
    draw_hatch_paths(s, h, color, width, k);
}

fn draw_hatch_paths(s: &mut String, h: &archisim_core::Hatch, color: &str, width: &str, k: f64) {
    let polys = hatch::hatch_polygons(h);
    let mut outline = String::new();
    for p in &polys {
        outline.push_str(&points_path(p, true));
        outline.push(' ');
    }
    let outline = outline.trim_end();
    if let Some(bg) = h.background {
        let _ = writeln!(
            s,
            r#"<path d="{outline}" fill="{}" fill-rule="evenodd" stroke="none"/>"#,
            hex(bg)
        );
    }
    let bands = hatch::hatch_bands(h);
    if !bands.is_empty() {
        let mut d = String::new();
        for b in &bands {
            d.push_str(&points_path(b, true));
            d.push(' ');
        }
        let _ = writeln!(
            s,
            r#"<path d="{}" fill="{color}" fill-rule="evenodd" stroke="none"/>"#,
            d.trim_end()
        );
    }
    let lines = if h.pattern == archisim_core::HatchPattern::Solid {
        None
    } else {
        hatch::hatch_lines(h, k)
    };
    match lines {
        Some(lines) => {
            let mut path = String::new();
            for (a, b) in lines {
                let b = if a.approx_eq(b) {
                    // A dot: a very short line with round caps.
                    b + Point2::new(k * 0.01, 0.0)
                } else {
                    b
                };
                let _ = write!(path, "M{} L{} ", pt(a), pt(b));
            }
            if !path.is_empty() {
                let _ = writeln!(
                    s,
                    r#"<path d="{}" stroke="{color}" stroke-width="{width}" stroke-linecap="round"/>"#,
                    path.trim_end()
                );
            }
        }
        None => {
            // Solid, or too dense to draw line by line.
            let _ = writeln!(
                s,
                r#"<path d="{outline}" fill="{color}" fill-rule="evenodd" stroke="none"/>"#
            );
        }
    }
}
