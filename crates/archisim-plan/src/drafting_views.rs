//! Views only for drawing (E7) and underlays: a view of drafting alone,
//! in real metres at its scale, and any plan, section or view for drawing
//! drawn under another in one colour, see-through, moved and turned.

use std::fmt::Write as _;

use archisim_core::{DraftViewId, Model, Point2, Underlay, UnderlaySource, ViewRef};

use crate::{num, PlanOptions};

/// What the drafting of a view for drawing covers, or `None` if it has
/// none. `k`: metres per paper millimetre.
pub fn drafting_extent(model: &Model, id: DraftViewId, k: f64) -> Option<(Point2, Point2)> {
    let mut out: Option<(Point2, Point2)> = None;
    for d in model.drafts_in(ViewRef::Drafting(id)) {
        let (a, b) = archisim_draft::bounds(&d.shape, k);
        out = Some(match out {
            None => (a, b),
            Some((lo, hi)) => (
                Point2::new(lo.x.min(a.x), lo.y.min(a.y)),
                Point2::new(hi.x.max(b.x), hi.y.max(b.y)),
            ),
        });
    }
    // The cropped underlays where they are put.
    for u in model.draft_view(id).map_or(&[][..], |v| &v.underlays[..]) {
        let Some([a, b]) = u.crop else { continue };
        let (c, sn) = (u.rotation.cos(), u.rotation.sin());
        for p in [a, b, Point2::new(a.x, b.y), Point2::new(b.x, a.y)] {
            let q = Point2::new(
                p.x * c - p.y * sn + u.offset.x,
                p.x * sn + p.y * c + u.offset.y,
            );
            out = Some(match out {
                None => (q, q),
                Some((lo, hi)) => (
                    Point2::new(lo.x.min(q.x), lo.y.min(q.y)),
                    Point2::new(hi.x.max(q.x), hi.y.max(q.y)),
                ),
            });
        }
    }
    out
}

/// The region a view for drawing shows: its drafting with a margin (a
/// metre square at the origin when it has none).
pub fn drafting_region(model: &Model, id: DraftViewId, options: &PlanOptions) -> (Point2, Point2) {
    let k = options.scale / 1000.0;
    let m = options.margin_mm * k;
    let (lo, hi) =
        drafting_extent(model, id, k).unwrap_or((Point2::new(0.0, 0.0), Point2::new(1.0, 1.0)));
    (lo - Point2::new(m, m), hi + Point2::new(m, m))
}

/// Draws a view for drawing as an SVG document: its underlays, then its
/// drafting, at `options.scale`, in `options.viewport` (or around what it
/// has).
pub fn render_drafting_view(model: &Model, id: DraftViewId, options: &PlanOptions) -> String {
    let k = options.scale / 1000.0;
    let (lo, hi) = options
        .viewport
        .unwrap_or_else(|| drafting_region(model, id, options));
    let (vb_x, vb_y, vb_w, vb_h) = (lo.x, -hi.y, hi.x - lo.x, hi.y - lo.y);
    let mut s = String::new();
    let _ = writeln!(
        s,
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="{:.2}mm" height="{:.2}mm" viewBox="{} {} {} {}">"#,
        vb_w / k,
        vb_h / k,
        num(vb_x),
        num(vb_y),
        num(vb_w),
        num(vb_h)
    );
    if options.viewport.is_none() {
        let _ = writeln!(
            s,
            r#"<rect x="{}" y="{}" width="{}" height="{}" fill="white"/>"#,
            num(vb_x),
            num(vb_y),
            num(vb_w),
            num(vb_h)
        );
    }
    if !options.skip_underlays {
        if let Some(v) = model.draft_view(id) {
            draw_underlays(&mut s, model, &v.underlays, options);
        }
    }
    s.push_str("<g data-cat=\"drafting\">\n");
    crate::drafting::draw_drafts(
        &mut s,
        model,
        ViewRef::Drafting(id),
        k,
        Some((lo, hi)),
        options.viewport.is_none(),
        &options.style.hidden_layers,
        false,
        None,
    );
    s.push_str("</g>\n</svg>\n");
    if options.thin_lines {
        // One pixel on screen, a hairline on paper.
        let mm = options
            .px_per_mm
            .map_or(crate::HAIRLINE_MM, |px| 1.0 / px.max(1e-6));
        return crate::hairlines(&s, mm * k);
    }
    s
}

/// Draws `underlays` (first lowest) into `s`, at the scale of `options`.
pub fn draw_underlays(
    s: &mut String,
    model: &Model,
    underlays: &[Underlay],
    options: &PlanOptions,
) {
    for u in underlays.iter().filter(|u| u.opacity > 0.0) {
        let Some(svg) = underlay_svg(model, u.source, options) else {
            continue;
        };
        let body = body_of(&svg);
        let body = match u.color {
            Some(c) => recolor(body, c),
            None => body.to_string(),
        };
        // The drawings have y down (−y): turned the other way round.
        let _ = writeln!(
            s,
            r#"<g class="underlay" opacity="{}" transform="translate({} {}) rotate({})">"#,
            num(u.opacity),
            num(u.offset.x),
            num(-u.offset.y),
            num(-u.rotation.to_degrees())
        );
        match u.crop {
            // Only the rectangle (its y down, like the drawing).
            Some([a, b]) => {
                let id = format!("underlay-crop-{}", s.len());
                let (x0, x1) = (a.x.min(b.x), a.x.max(b.x));
                let (y0, y1) = (a.y.min(b.y), a.y.max(b.y));
                let _ = writeln!(
                    s,
                    r#"<clipPath id="{id}"><rect x="{}" y="{}" width="{}" height="{}"/></clipPath><g clip-path="url(#{id})">"#,
                    num(x0),
                    num(-y1),
                    num(x1 - x0),
                    num(y1 - y0)
                );
                s.push_str(&body);
                s.push_str("</g>");
            }
            None => s.push_str(&body),
        }
        s.push_str("</g>\n");
    }
}

/// The drawing of an underlay's source, without its own underlays (no
/// loops) and its paper.
fn underlay_svg(model: &Model, source: UnderlaySource, options: &PlanOptions) -> Option<String> {
    let far = Some((Point2::new(-1e5, -1e5), Point2::new(1e5, 1e5)));
    let o = PlanOptions {
        viewport: far,
        skip_underlays: true,
        title: false,
        shadows: None,
        ..options.clone()
    };
    match source {
        UnderlaySource::Level(l) => model.level(l).map(|_| crate::render_plan(model, l, &o)),
        UnderlaySource::Section(sid) => model.section(sid).map(|sec| {
            let d = crate::section::section_drawing(model, sec);
            crate::section::render_section(model, &d, &o)
        }),
        UnderlaySource::Drafting(d) => model
            .draft_view(d)
            .map(|_| render_drafting_view(model, d, &o)),
    }
}

/// What is inside the outer `<svg>`, less a white paper rectangle.
fn body_of(svg: &str) -> &str {
    let start = svg.find('>').map_or(0, |i| i + 1);
    let end = svg.rfind("</svg>").unwrap_or(svg.len());
    let mut body = &svg[start..end];
    let trimmed = body.trim_start();
    if trimmed.starts_with("<rect")
        && trimmed[..trimmed.find('>').unwrap_or(0)].contains("fill=\"white\"")
    {
        body = &trimmed[trimmed.find('>').map_or(0, |i| i + 1)..];
    }
    body
}

/// Every stroke and fill of an SVG fragment in one colour (fills faint,
/// so the drawing reads as lines); `none` stays.
pub fn recolor(svg: &str, c: [u8; 3]) -> String {
    let colour = format!("#{:02x}{:02x}{:02x}", c[0], c[1], c[2]);
    // Fills a quarter of the colour over white: the drawing reads as lines.
    let pale = c.map(|v| (v as f64 * 0.25 + 255.0 * 0.75).round() as u8);
    let pale = format!("#{:02x}{:02x}{:02x}", pale[0], pale[1], pale[2]);
    let mut out = String::with_capacity(svg.len());
    let mut rest = svg;
    loop {
        let next = ["stroke=\"", "fill=\""]
            .iter()
            .filter_map(|k| rest.find(k).map(|i| (i, *k)))
            .min_by_key(|(i, _)| *i);
        let Some((i, key)) = next else {
            out.push_str(rest);
            break;
        };
        // Not part of another name (stroke-width=, fill-opacity=…).
        let before = rest[..i].chars().last();
        out.push_str(&rest[..i + key.len()]);
        rest = &rest[i + key.len()..];
        let end = rest.find('"').unwrap_or(rest.len());
        let value = &rest[..end];
        let is_name_start = before.is_none_or(|ch| ch == ' ' || ch == '\n' || ch == '<');
        if !is_name_start || value == "none" {
            out.push_str(value);
        } else if key == "fill=\"" {
            out.push_str(&pale);
        } else {
            out.push_str(&colour);
        }
        rest = &rest[end..];
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use archisim_core::{NewDraft, Shape};

    #[test]
    fn a_view_for_drawing_with_an_underlay() {
        let mut m = Model::new("t");
        let pb = m.add_level("PB", 0.0);
        let layer = m.draw_layers()[0].id;
        // A line on the plan, a circle in the view for drawing.
        m.add_draft(NewDraft::new(
            ViewRef::Level(pb),
            layer,
            Shape::Line {
                a: Point2::new(0.0, 0.0),
                b: Point2::new(3.0, 0.0),
            },
        ))
        .unwrap();
        let d = m.add_draft_view("Esquema", 20.0).unwrap();
        m.add_draft(NewDraft::new(
            ViewRef::Drafting(d),
            layer,
            Shape::Circle {
                centre: Point2::new(1.0, 1.0),
                radius: 0.5,
            },
        ))
        .unwrap();
        let o = PlanOptions {
            scale: 20.0,
            ..PlanOptions::default()
        };
        let plain = render_drafting_view(&m, d, &o);
        assert!(plain.contains("<svg") && !plain.contains("underlay"));
        let mut v = m.draft_view(d).unwrap().clone();
        let mut u = Underlay::of(UnderlaySource::Level(pb));
        u.offset = Point2::new(2.0, 1.0);
        u.rotation = std::f64::consts::FRAC_PI_2;
        v.underlays.push(u);
        m.set_drafting_view(v).unwrap();
        let with = render_drafting_view(&m, d, &o);
        assert!(with.contains("class=\"underlay\""));
        assert!(with.contains("translate(2 -1) rotate(-90)"), "{with}");
        // In the underlay's colour.
        assert!(with.contains("#788caa"));
        // The plan shows the view under it, and not itself in a loop.
        m.set_level_underlays(pb, vec![Underlay::of(UnderlaySource::Drafting(d))])
            .unwrap();
        let plan = crate::render_plan(&m, pb, &PlanOptions::default());
        assert_eq!(plan.matches("class=\"underlay\"").count(), 1);
        // On a sheet, at its scale, with its name.
        let sheet = m
            .add_sheet(archisim_core::NewSheet {
                number: "D-01".into(),
                name: "Detalles".into(),
                width: 420.0,
                height: 297.0,
            })
            .unwrap();
        let vp = m
            .add_drafting_viewport(sheet, d, Point2::new(0.2, 0.15), None)
            .unwrap();
        assert_eq!(m.viewport(vp).unwrap().scale, 20.0);
        let paper = crate::sheet::render_sheet(&m, sheet, &Default::default());
        assert!(paper.contains("class=\"viewport\""));
        assert_eq!(
            crate::sheet::viewport_title(&m, m.viewport(vp).unwrap()),
            "ESQUEMA"
        );
        assert_eq!(
            recolor(
                r#"<path stroke="red" stroke-width="2" fill="none"/>"#,
                [1, 2, 3]
            ),
            r##"<path stroke="#010203" stroke-width="2" fill="none"/>"##
        );
    }

    #[test]
    fn a_detail_shows_only_its_piece_of_the_section() {
        let mut m = Model::new("t");
        let pb = m.add_level("PB", 0.0);
        let _ = pb;
        let sec = m
            .add_section(archisim_core::NewSection {
                name: "A".into(),
                kind: archisim_core::SectionKind::Section,
                start: Point2::new(0.0, -1.0),
                end: Point2::new(0.0, 5.0),
                depth: 2.0,
            })
            .unwrap();
        let d = m
            .add_section_detail(sec, Point2::new(2.0, 3.5), Point2::new(1.0, 2.5), 10.0)
            .unwrap();
        let v = m.draft_view(d).unwrap();
        assert_eq!(v.name, "Detalle de A");
        let u = v.underlays[0];
        assert_eq!(u.crop, Some([Point2::new(1.0, 2.5), Point2::new(2.0, 3.5)]));
        // Its lower left corner at the origin, and the view round it.
        assert_eq!(u.offset, Point2::new(-1.0, -2.5));
        let (lo, hi) = drafting_extent(&m, d, 0.01).unwrap();
        assert!(lo.distance(Point2::new(0.0, 0.0)) < 1e-9);
        assert!(hi.distance(Point2::new(1.0, 1.0)) < 1e-9);
        let o = PlanOptions {
            scale: 10.0,
            ..Default::default()
        };
        let svg = render_drafting_view(&m, d, &o);
        assert!(svg.contains("<clipPath id=\"underlay-crop-"));
        assert!(svg.contains("clip-path=\"url(#underlay-crop-"));
        // A second one is numbered.
        let d2 = m
            .add_section_detail(sec, Point2::new(0.0, 0.0), Point2::new(1.0, 1.0), 10.0)
            .unwrap();
        assert_eq!(m.draft_view(d2).unwrap().name, "Detalle de A (2)");
        // A rectangle with no area is refused.
        assert!(m
            .add_section_detail(sec, Point2::new(0.0, 0.0), Point2::new(1.0, 0.0), 10.0)
            .is_err());
    }
}
