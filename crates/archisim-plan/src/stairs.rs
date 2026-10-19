//! Stairs in plan, as drawn in Spain: the outline of flights and landings,
//! a line at every riser, what lies above the cut plane dashed beyond a
//! zigzag break line, and the walking line from a dot at the foot to an
//! arrow at the head (the way up).

use std::fmt::Write as _;

use archisim_core::{Point2, Stair};

use crate::{num, path, pt, HATCH_LINE, INK, SLAB_COVER, VIEW_LINE};

/// The stairs of other levels that arrive at `level` (their top is tied
/// to it), and how far it is above each one's level.
pub fn arriving(model: &archisim_core::Model, level: archisim_core::LevelId) -> Vec<(&Stair, f64)> {
    let Some(here) = model.level(level).map(|l| l.elevation) else {
        return Vec::new();
    };
    model
        .stairs()
        .filter(|s| {
            s.level != level
                && matches!(s.top, archisim_core::WallTop::Level { level: l, .. } if l == level)
        })
        .filter_map(|s| {
            let base = model.level(s.level)?.elevation;
            Some((s, here - base))
        })
        .collect()
}

/// Every point of the stair the plan must show.
pub fn stair_points(stair: &Stair) -> Vec<Point2> {
    stair.outlines().into_iter().flatten().collect()
}

fn line(s: &mut String, a: Point2, b: Point2, w: f64, dashed: bool, k: f64) {
    let dash = if dashed {
        format!(r#" stroke-dasharray="{} {}""#, num(2.0 * k), num(1.2 * k))
    } else {
        String::new()
    };
    let _ = writeln!(
        s,
        r#"<path d="M{} L{}" fill="none" stroke="{INK}" stroke-width="{}"{dash}/>"#,
        pt(a),
        pt(b),
        num(w * k)
    );
}

pub fn draw_stair(s: &mut String, stair: &Stair, cut_plane: f64, k: f64) {
    let l = stair.layout();
    let half = stair.width / 2.0;
    let _ = writeln!(s, r#"<g class="stair" stroke-linecap="round">"#);
    // Landings: light, as floors seen from above.
    for landing in &l.landings {
        let above = landing.top > cut_plane;
        let _ = writeln!(
            s,
            r#"<path d="{}" fill="{}" stroke="none"/>"#,
            path(&landing.outline),
            if above { "none" } else { SLAB_COVER }
        );
        let n = landing.outline.len();
        for i in 0..n {
            line(
                s,
                landing.outline[i],
                landing.outline[(i + 1) % n],
                VIEW_LINE,
                above,
                k,
            );
        }
    }
    let mut broken = false;
    for f in &l.flights {
        let u = f.direction();
        let nrm = u.perp_left() * half;
        let at = |d: f64| f.start + u * d;
        // Where the cut plane crosses the flight: the first tread above it.
        let tread_top = |i: usize| f.base + (i + 1) as f64 * l.riser;
        let cut_at = (0..f.treads)
            .find(|&i| tread_top(i) > cut_plane)
            .map(|i| (i as f64 + 0.5) * f.going);
        let below = |d: f64| cut_at.is_none_or(|c| d <= c);
        let _ = writeln!(
            s,
            r#"<path d="{}" fill="{SLAB_COVER}" stroke="none"/>"#,
            path(&f.outline(stair.width))
        );
        // A line at every riser (the last one is the landing's edge).
        for i in 0..=f.treads {
            let d = (i as f64 * f.going).min(f.length());
            line(s, at(d) - nrm, at(d) + nrm, VIEW_LINE, !below(d), k);
        }
        // The sides, solid up to the cut.
        for side in [-1.0, 1.0] {
            let off = nrm * side;
            match cut_at {
                Some(c) => {
                    line(s, f.start + off, at(c) + off, VIEW_LINE, false, k);
                    line(s, at(c) + off, f.end + off, VIEW_LINE, true, k);
                }
                None => line(s, f.start + off, f.end + off, VIEW_LINE, false, k),
            }
        }
        // The railings, just inside the sides.
        for (on, sign) in [(stair.railings.left(), 1.0), (stair.railings.right(), -1.0)] {
            if !on {
                continue;
            }
            let off = u.perp_left() * (sign * (half - 0.04));
            match cut_at {
                Some(c) => {
                    line(s, f.start + off, at(c) + off, HATCH_LINE, false, k);
                    line(s, at(c) + off, f.end + off, HATCH_LINE, true, k);
                }
                None => line(s, f.start + off, f.end + off, HATCH_LINE, false, k),
            }
        }
        // The break line, once, where the plane cuts.
        if let (Some(c), false) = (cut_at, broken) {
            broken = true;
            let g = f.going;
            let a = at(c - g * 0.6) - nrm;
            let b = at(c + g * 0.6) + nrm;
            let m = at(c);
            let zig = u.perp_left() * (0.12 * stair.width);
            let z1 = m - zig * 1.0 - u * (g * 0.25);
            let z2 = m + zig * 1.0 + u * (g * 0.25);
            let _ = writeln!(
                s,
                r#"<path d="M{} L{} L{} L{}" fill="none" stroke="{INK}" stroke-width="{}" stroke-linejoin="miter"/>"#,
                pt(a),
                pt(z1),
                pt(z2),
                pt(b),
                num(VIEW_LINE * 1.3 * k)
            );
        }
    }
    // The walking line: from a dot at the foot to an arrow at the head.
    if stair.path.len() >= 2 {
        let _ = writeln!(
            s,
            r#"<path d="M{}" fill="none" stroke="{INK}" stroke-width="{}"/>"#,
            stair
                .path
                .iter()
                .map(|p| pt(*p))
                .collect::<Vec<_>>()
                .join(" L"),
            num(HATCH_LINE * k)
        );
        let p0 = stair.path[0];
        let _ = writeln!(
            s,
            r#"<circle cx="{}" cy="{}" r="{}" fill="{INK}"/>"#,
            num(p0.x),
            num(-p0.y),
            num(0.8 * k)
        );
        let n = stair.path.len();
        let (a, b) = (stair.path[n - 2], stair.path[n - 1]);
        if let Some(u) = (b - a).normalized() {
            let head = 2.2 * k;
            let base = b - u * head;
            let w = u.perp_left() * (head * 0.35);
            let _ = writeln!(
                s,
                r#"<path d="M{} L{} L{} Z" fill="{INK}" stroke="none"/>"#,
                pt(b),
                pt(base + w),
                pt(base - w)
            );
        }
    }
    s.push_str("</g>\n");
}

#[cfg(test)]
mod tests {
    use archisim_core::{CutPattern, Model, NewStair, Point2, StairUse, WallTop};

    #[test]
    fn the_upper_plan_shows_the_stair_arriving() {
        let mut m = Model::new("t");
        let pb = m.add_level("PB", 0.0);
        let p1 = m.add_level("P1", 2.8);
        let p2 = m.add_level("P2", 5.6);
        let mat = m.add_material("HA", "concrete", [200, 200, 200], CutPattern::Solid);
        m.add_stair(NewStair {
            level: pb,
            path: vec![Point2::new(0.0, 0.0), Point2::new(0.0, 5.0)],
            width: 1.0,
            going: 0.28,
            base_offset: 0.0,
            top: WallTop::Level {
                level: p1,
                offset: 0.0,
            },
            height: 0.0,
            waist: 0.15,
            material: mat,
            usage: StairUse::General,
        })
        .unwrap();
        let o = crate::PlanOptions::default();
        let count = |l| {
            crate::render_plan(&m, l, &o)
                .matches(r#"class="stair""#)
                .count()
        };
        assert_eq!(count(pb), 1);
        assert_eq!(count(p1), 1);
        assert_eq!(count(p2), 0);
        // Seen from above: nothing dashed beyond a break line.
        let upper = crate::render_plan(&m, p1, &o);
        let stair = &upper[upper.find(r#"class="stair""#).unwrap()..];
        let stair = &stair[..stair.find("</g>").unwrap()];
        assert!(!stair.contains("stroke-dasharray"));
    }
}
