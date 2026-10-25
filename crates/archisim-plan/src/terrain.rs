//! The terrain in plan, as a topographic plan: contour lines every
//! interval (each fifth one heavier, with its height), the survey points
//! as small crosses with their heights, and the edge of the surveyed
//! ground dashed. And the north arrow of the drawings.

use std::fmt::Write as _;

use archisim_core::{Model, Point2, Terrain};

use crate::lettering::{text_path, Align};
use crate::{annotate, num, pens, pt, INK};

/// Colour of the contour lines.
const CONTOUR: &str = "#8a7a62";

pub fn terrain_points(t: &Terrain) -> Vec<Point2> {
    t.points.iter().map(|p| p.at).collect()
}

/// A height as written on a topographic plan: "+12,50".
fn height(z: f64) -> String {
    let s = format!("{:.2}", z.abs()).replace('.', ",");
    if z < -0.0005 {
        format!("-{s}")
    } else {
        format!("+{s}")
    }
}

pub fn draw_terrain(s: &mut String, model: &Model, t: &Terrain, k: f64) {
    let _ = writeln!(s, r#"<g class="terrain" stroke-linecap="round">"#);
    // The areas painted over it (lawn, gravel…), in their colour, under
    // the contour lines.
    for patch in &t.patches {
        let [r, g, b] = model.surface_color(patch.material);
        let mut d = String::new();
        for (outer, holes) in &patch.regions {
            for ring in std::iter::once(outer).chain(holes.iter()) {
                if ring.len() < 3 {
                    continue;
                }
                let pts: Vec<String> = ring.iter().map(|q| pt(*q)).collect();
                let _ = write!(d, "M{} Z ", pts.join(" L"));
            }
        }
        if d.is_empty() {
            continue;
        }
        let _ = writeln!(
            s,
            r##"<path class="terrain-patch" d="{}" fill="#{r:02x}{g:02x}{b:02x}" fill-opacity="0.55" fill-rule="evenodd" stroke="{CONTOUR}" stroke-width="{}"/>"##,
            d.trim_end(),
            num(pens().hatch * k)
        );
    }
    let th = annotate::TEXT_HEIGHT * 0.8 * k;
    let step = t.contour;
    let lines = t.contours();
    // Few lines: every one with its height; many: every fifth, heavier.
    let few = lines.len() <= 12;
    for (z, pieces) in lines {
        let index = step > 0.0 && ((z / step).round() as i64) % 5 == 0;
        let labelled = few || index;
        let mut d = String::new();
        for (a, b) in &pieces {
            let _ = write!(d, "M{} L{} ", pt(*a), pt(*b));
        }
        let _ = writeln!(
            s,
            r#"<path d="{}" fill="none" stroke="{CONTOUR}" stroke-width="{}"/>"#,
            d.trim_end(),
            num(if index { pens().view } else { pens().hatch } * k)
        );
        if labelled {
            // The height on the longest piece, along it.
            if let Some((a, b)) = pieces
                .iter()
                .max_by(|x, y| x.0.distance(x.1).total_cmp(&y.0.distance(y.1)))
            {
                let mut dir = (*b - *a).normalized().unwrap_or(Point2::new(1.0, 0.0));
                if dir.x < 0.0 {
                    dir = dir * -1.0;
                }
                let mid = (*a + *b) * 0.5 + dir.perp_left() * (0.6 * k);
                let _ = writeln!(
                    s,
                    r#"<path d="{}" {}/>"#,
                    text_path(&height(z), mid, dir, th, Align::Centre),
                    crate::lettering::paint(th, CONTOUR)
                );
            }
        }
    }
    // The edge of the surveyed ground.
    let ring = t.boundary();
    if ring.len() >= 3 {
        let pts: Vec<String> = ring.iter().map(|i| pt(t.points[*i].at)).collect();
        let _ = writeln!(
            s,
            r#"<path d="M{} Z" fill="none" stroke="{CONTOUR}" stroke-width="{}" stroke-dasharray="{} {}"/>"#,
            pts.join(" L"),
            num(pens().hatch * k),
            num(3.0 * k),
            num(1.5 * k)
        );
    }
    // Survey points.
    let c = 0.8 * k;
    for p in &t.points {
        let q = p.at;
        let _ = writeln!(
            s,
            r#"<path d="M{} L{} M{} L{}" fill="none" stroke="{INK}" stroke-width="{}"/><path d="{}" {}/>"#,
            pt(q - Point2::new(c, 0.0)),
            pt(q + Point2::new(c, 0.0)),
            pt(q - Point2::new(0.0, c)),
            pt(q + Point2::new(0.0, c)),
            num(pens().hatch * k),
            text_path(
                &height(p.z),
                q + Point2::new(c, c * 0.6),
                Point2::new(1.0, 0.0),
                th,
                Align::Left
            ),
            crate::lettering::paint(th, INK)
        );
    }
    s.push_str("</g>\n");
}

/// The north arrow, centred at `at`: a circle with an arrow towards true
/// north (`angle` radians clockwise from up) and an N.
pub fn draw_north(s: &mut String, at: Point2, angle: f64, k: f64) {
    let r = 6.0 * k;
    let up = Point2::new(angle.sin(), angle.cos());
    let side = Point2::new(up.y, -up.x);
    let tip = at + up * r;
    let tail = at - up * r;
    let _ = writeln!(
        s,
        r#"<g class="north"><circle cx="{}" cy="{}" r="{}" fill="none" stroke="{INK}" stroke-width="{}"/><path d="M{} L{} L{} Z" fill="{INK}" stroke="none"/><path d="M{} L{}" fill="none" stroke="{INK}" stroke-width="{}"/><path d="{}" {}/></g>"#,
        num(at.x),
        num(-at.y),
        num(r),
        num(pens().hatch * k),
        pt(tip),
        pt(at + side * (r * 0.3) - up * (r * 0.2)),
        pt(at - side * (r * 0.3) - up * (r * 0.2)),
        pt(at),
        pt(tail),
        num(pens().hatch * k),
        text_path(
            "N",
            tip + up * (2.4 * k),
            Point2::new(1.0, 0.0),
            2.5 * k,
            Align::Centre
        ),
        crate::lettering::paint(2.5 * k, INK)
    );
}
