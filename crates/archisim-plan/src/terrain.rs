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
        draw_growth(s, model, patch, k);
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

/// The grass and flowers of a painted area, as the plan shows them: little
/// strokes of grass where it grows tall and dots of each flower's colour,
/// a few millimetres of paper apart (not on screen at small scales).
fn draw_growth(s: &mut String, model: &Model, patch: &archisim_core::TerrainPatch, k: f64) {
    let Some(g) = model.appearance(patch.material).growth else {
        return;
    };
    if crate::coarse() {
        return;
    }
    let inside = |p: Point2| {
        patch.regions.iter().any(|(o, holes)| {
            archisim_core::point_in_polygon(p, o)
                && !holes.iter().any(|h| archisim_core::point_in_polygon(p, h))
        })
    };
    let (mut lo, mut hi) = (
        Point2::new(f64::INFINITY, f64::INFINITY),
        Point2::new(f64::NEG_INFINITY, f64::NEG_INFINITY),
    );
    for (o, _) in &patch.regions {
        for q in o {
            lo = Point2::new(lo.x.min(q.x), lo.y.min(q.y));
            hi = Point2::new(hi.x.max(q.x), hi.y.max(q.y));
        }
    }
    if !lo.x.is_finite() {
        return;
    }
    let hash = |i: i64, j: i64, salt: u64| {
        let mut x = (i as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15)
            ^ (j as u64).wrapping_mul(0xC2B2_AE3D_27D4_EB4F)
            ^ salt;
        x ^= x >> 31;
        x = x.wrapping_mul(0xBF58_476D_1CE4_E5B9);
        x ^= x >> 29;
        (x >> 11) as f64 / (1u64 << 53) as f64
    };
    // Points a little apart on the paper, at most a few thousand.
    let points = |spacing: f64, salt: u64| -> Vec<(Point2, f64)> {
        let area = (hi.x - lo.x) * (hi.y - lo.y);
        let spacing = spacing.max((area / 4000.0).sqrt());
        let mut out = Vec::new();
        let (nx, ny) = (
            ((hi.x - lo.x) / spacing).ceil() as i64,
            ((hi.y - lo.y) / spacing).ceil() as i64,
        );
        for j in 0..ny {
            for i in 0..nx {
                let p = Point2::new(
                    lo.x + (i as f64 + hash(i, j, salt)) * spacing,
                    lo.y + (j as f64 + hash(j, i, salt ^ 7)) * spacing,
                );
                if inside(p) {
                    out.push((p, hash(i, j, salt ^ 13)));
                }
            }
        }
        out
    };
    if g.height >= 0.15 {
        let [r, gg, b] = model.surface_color(patch.material);
        let dark = [r, gg, b].map(|c| (c as f64 * 0.55) as u8);
        let mut d = String::new();
        let len = 1.2 * k;
        for (p, h) in points(3.0 * k, 1) {
            let a = 1.2 + h * 0.7;
            for side in [-1.0, 1.0] {
                let q = Point2::new(p.x + side * len * 0.35 * a.cos(), p.y + len * a.sin());
                let _ = write!(d, "M{} L{} ", pt(p), pt(q));
            }
        }
        if !d.is_empty() {
            let _ = writeln!(
                s,
                r##"<path class="pat" d="{}" fill="none" stroke="#{:02x}{:02x}{:02x}" stroke-width="{}"/>"##,
                d.trim_end(),
                dark[0],
                dark[1],
                dark[2],
                num(pens().hatch * k)
            );
        }
    }
    let colours = g.flower_colours();
    if g.bloom > 0.0 && !colours.is_empty() {
        let r = 0.45 * k;
        let mut by: Vec<String> = vec![String::new(); colours.len()];
        for (p, h) in points((2.2 * k).max(1.0 / g.bloom.sqrt()), 2) {
            let i = (h * colours.len() as f64) as usize % colours.len();
            let _ = write!(
                by[i],
                "M{} a{r} {r} 0 1 0 {} 0 a{r} {r} 0 1 0 {} 0 ",
                pt(Point2::new(p.x - r, p.y)),
                num(2.0 * r),
                num(-2.0 * r),
                r = num(r)
            );
        }
        for (c, d) in colours.iter().zip(by) {
            if d.is_empty() {
                continue;
            }
            let _ = writeln!(
                s,
                r##"<path class="pat" d="{}" fill="#{:02x}{:02x}{:02x}" stroke="none"/>"##,
                d.trim_end(),
                c[0],
                c[1],
                c[2]
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use archisim_core::{Appearance, Growth, NewTerrain, SurveyPoint, TerrainPatch};

    /// A meadow with flowers is drawn with its tufts and a dot of each
    /// flower's colour; plain earth, without.
    #[test]
    fn meadows_show_their_flowers() {
        let mut m = Model::new("t");
        let l = m.add_level("PB", 0.0);
        let earth = m.add_material(
            "Tierra",
            "earth",
            [150, 120, 90],
            archisim_core::CutPattern::Solid,
        );
        let meadow = m.add_material(
            "Pradera",
            "grass",
            [110, 160, 70],
            archisim_core::CutPattern::Solid,
        );
        m.set_appearance(
            meadow,
            Appearance {
                growth: Some(Growth::flowering(0.3, 12.0, &[[238, 112, 40]], 10.0)),
                ..Default::default()
            },
        )
        .unwrap();
        let sp = |x: f64, y: f64| SurveyPoint {
            at: Point2::new(x, y),
            z: 0.0,
        };
        let id = m
            .add_terrain(NewTerrain {
                level: l,
                points: vec![sp(0.0, 0.0), sp(10.0, 0.0), sp(10.0, 10.0), sp(0.0, 10.0)],
                material: earth,
                depth: 1.0,
                contour: 0.5,
                lines: Vec::new(),
            })
            .unwrap();
        let square = |x: f64| {
            vec![
                Point2::new(x, 1.0),
                Point2::new(x + 3.0, 1.0),
                Point2::new(x + 3.0, 4.0),
                Point2::new(x, 4.0),
            ]
        };
        m.edit_terrain(id, |t| {
            t.patches = vec![
                TerrainPatch {
                    material: meadow,
                    regions: vec![(square(1.0), Vec::new())],
                },
                TerrainPatch {
                    material: earth,
                    regions: vec![(square(5.0), Vec::new())],
                },
            ]
        })
        .unwrap();
        let mut s = String::new();
        draw_terrain(&mut s, &m, m.terrain(id).unwrap(), 0.05);
        assert!(s.contains("fill=\"#ee7028\""), "flower dots");
        assert_eq!(s.matches("class=\"pat\"").count(), 2, "tufts and flowers");
    }
}
