//! Stairs in plan, as drawn in Spain: the outline of flights and landings,
//! a line at every riser, what lies above the cut plane dashed beyond a
//! zigzag break line, and the walking line from a dot at the foot to an
//! arrow at the head (the way up).

use std::fmt::Write as _;

use archisim_core::{Point2, Stair};

use crate::{num, path, pens, pt, INK, SLAB_COVER};

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

fn polyline(s: &mut String, pts: &[Point2], w: f64, dashed: bool, k: f64) {
    if pts.len() < 2 {
        return;
    }
    let dash = if dashed {
        format!(r#" stroke-dasharray="{} {}""#, num(2.0 * k), num(1.2 * k))
    } else {
        String::new()
    };
    let d = pts.iter().map(|p| pt(*p)).collect::<Vec<_>>().join(" L");
    let _ = writeln!(
        s,
        r#"<path d="M{d}" fill="none" stroke="{INK}" stroke-width="{}" stroke-linejoin="round"{dash}/>"#,
        num(w * k)
    );
}

/// A line of points split where it passes the cut plane: the stretches
/// over what one stands on below it (solid) and above it (dashed).
fn split_line(
    s: &mut String,
    line: &[archisim_core::SidePoint],
    above: impl Fn(f64) -> bool,
    w: f64,
    k: f64,
) {
    let mut run: Vec<Point2> = Vec::new();
    let mut dashed = None;
    for win in line.windows(2) {
        let d = above(win[0].floor);
        if dashed != Some(d) {
            if let Some(was) = dashed {
                polyline(s, &run, w, was, k);
            }
            run = vec![win[0].p];
            dashed = Some(d);
        }
        run.push(win[1].p);
    }
    if let Some(was) = dashed {
        polyline(s, &run, w, was, k);
    }
}

pub fn draw_stair(s: &mut String, stair: &Stair, cut_plane: f64, k: f64) {
    let l = stair.layout();
    let r = l.riser;
    // Beyond the cut: what one would stand on from a riser that starts
    // above the cut plane.
    let above = |floor: f64| floor - r > cut_plane + 1e-9;
    let _ = writeln!(s, r#"<g class="stair" stroke-linecap="round">"#);
    // Landings: light, as floors seen from above.
    for landing in &l.landings {
        let up = above(landing.top);
        let _ = writeln!(
            s,
            r#"<path d="{}" fill="{}" stroke="none"/>"#,
            path(&landing.outline),
            if up { "none" } else { SLAB_COVER }
        );
        let n = landing.outline.len();
        for i in 0..n {
            line(
                s,
                landing.outline[i],
                landing.outline[(i + 1) % n],
                pens().view,
                up,
                k,
            );
        }
    }
    // The treads, and a line at every riser (the last of each run is the
    // edge of its landing or of the floor above).
    for st in &l.steps {
        let _ = writeln!(
            s,
            r#"<path d="{}" fill="{SLAB_COVER}" stroke="none"/>"#,
            path(&st.outline())
        );
    }
    for run in &l.runs {
        for kk in run.steps.clone() {
            let st = &l.steps[kk];
            line(s, st.front[0], st.front[1], pens().view, above(st.top), k);
        }
        if let Some(last) = run.steps.clone().last().map(|i| &l.steps[i]) {
            line(
                s,
                last.back[0],
                last.back[1],
                pens().view,
                above(last.top + r),
                k,
            );
        }
    }
    // The sides, solid up to the cut.
    for left in [true, false] {
        split_line(s, &l.side(left), above, pens().view, k);
    }
    // The railings, just inside the sides.
    for (on, left) in [
        (stair.railings.left(), true),
        (stair.railings.right(), false),
    ] {
        if on && !archisim_geom::stairs::skip_inner_rail(stair, left) {
            let rail = archisim_geom::stairs::railing_line(&l, left);
            split_line(s, &rail, above, pens().hatch, k);
        }
    }
    // A spiral's newel.
    if stair.design.form == archisim_core::StairForm::Spiral
        && stair.design.structure == archisim_core::StairStructure::Newel
    {
        let c = stair.path[0];
        let _ = writeln!(
            s,
            r#"<circle cx="{}" cy="{}" r="{}" fill="{INK}" stroke="none"/>"#,
            num(c.x),
            num(-c.y),
            num(stair.design.core.clamp(0.03, 0.3))
        );
    }
    // The break line, once, across the tread the plane cuts.
    if let Some(st) = l
        .steps
        .iter()
        .find(|st| st.top > cut_plane + 1e-9 && st.top - r <= cut_plane + 1e-9)
    {
        // From the right side, a little up the tread, to the left side,
        // further up, with a zigzag in the middle.
        let a = st.front[1] + (st.back[1] - st.front[1]) * 0.2;
        let b = st.front[0] + (st.back[0] - st.front[0]) * 0.8;
        let m = (a + b) * 0.5;
        let ab = b - a;
        let side = ab.perp_left() * 0.12;
        let z1 = m - ab * 0.06 + side;
        let z2 = m + ab * 0.06 - side;
        let _ = writeln!(
            s,
            r#"<path d="M{} L{} L{} L{}" fill="none" stroke="{INK}" stroke-width="{}" stroke-linejoin="miter"/>"#,
            pt(a),
            pt(z1),
            pt(z2),
            pt(b),
            num(pens().view * 1.3 * k)
        );
    }
    // The walking line: from a dot at the foot to an arrow at the head.
    let walk = &l.walk;
    if walk.len() >= 2 {
        polyline(s, walk, pens().hatch, false, k);
        let p0 = walk[0];
        let _ = writeln!(
            s,
            r#"<circle cx="{}" cy="{}" r="{}" fill="{INK}"/>"#,
            num(p0.x),
            num(-p0.y),
            num(0.8 * k)
        );
        let n = walk.len();
        let (a, b) = (walk[n - 2], walk[n - 1]);
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
            design: Default::default(),
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
