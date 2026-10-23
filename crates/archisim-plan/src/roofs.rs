//! Pitched roofs in plan, as a roof plan ("planta de cubiertas"): each
//! plane in the light tone of what is seen below the cut, its ridges, hips
//! and eaves as view lines, the footprint (the walls under the eaves)
//! dashed, and on each plane an arrow down the slope (where the water
//! runs) with its pitch in per cent.

use std::fmt::Write as _;

use archisim_core::{DormerKind, Point2, Roof, Vec2};

use crate::lettering::{text_path, Align};
use crate::{annotate, num, path, pens, pt, INK, SLAB_COVER};

/// Every point of the roof that the plan must show.
pub fn roof_points(roof: &Roof) -> Vec<Point2> {
    roof.eaves()
}

pub fn draw_roof(s: &mut String, roof: &Roof, k: f64) {
    let planes = roof.planes();
    let _ = writeln!(s, r#"<g class="roof" stroke-linejoin="round">"#);
    for plane in &planes {
        let _ = writeln!(
            s,
            r#"<path d="{}" fill="{SLAB_COVER}" stroke="{INK}" stroke-width="{}"/>"#,
            path(&plane.outline),
            num(pens().view * k)
        );
    }
    // The walls under the eaves.
    if roof.overhang > 1e-6 {
        let _ = writeln!(
            s,
            r#"<path d="{}" fill="none" stroke="{INK}" stroke-width="{}" stroke-dasharray="{} {}"/>"#,
            path(&roof.outline),
            num(pens().hatch * k),
            num(2.0 * k),
            num(1.2 * k)
        );
    }
    // The eaves, a little heavier.
    let _ = writeln!(
        s,
        r#"<path d="{}" fill="none" stroke="{INK}" stroke-width="{}"/>"#,
        path(&roof.eaves()),
        num(pens().view * 1.5 * k)
    );
    // The tiles and dormers under the arrows, so these stay readable.
    draw_tiles(s, roof, k);
    draw_dormers(s, roof, k);
    for plane in &planes {
        let Some(u) = plane.slope.direction() else {
            continue;
        };
        // Down the slope, from the middle of the plane.
        let down = u * -1.0;
        let c = archisim_core::centroid(&plane.outline);
        let len = 9.0 * k;
        let (a, b) = (c - down * (len / 2.0), c + down * (len / 2.0));
        let head = 2.2 * k;
        let n = down.perp_left();
        let base = b - down * head;
        let _ = writeln!(
            s,
            r#"<path d="M{} L{}" fill="none" stroke="{INK}" stroke-width="{}"/><path d="M{} L{} L{} Z" fill="{INK}" stroke="none"/>"#,
            pt(a),
            pt(base),
            num(pens().view * k),
            pt(b),
            pt(base + n * (head * 0.35)),
            pt(base - n * (head * 0.35))
        );
        let text = format!("{} %", percent(plane.slope.slope * 100.0));
        let th = annotate::TEXT_HEIGHT * k;
        // Beside the arrow, read from below or from the right.
        let read = if down.x < -1e-9 || (down.x.abs() <= 1e-9 && down.y < 0.0) {
            down * -1.0
        } else {
            down
        };
        let at = c + read.perp_left() * (1.2 * k);
        let _ = writeln!(
            s,
            r#"<path d="{}" {}/>"#,
            text_path(&text, at, read, th, Align::Centre),
            crate::lettering::paint(th, INK)
        );
    }
    draw_gutters(s, roof, k);
    s.push_str("</g>\n");
}

/// The tiles in plan: lines down the slope at each cover (curved tiles,
/// metal seams) or across it at each course (flat tiles, slate), round
/// the dormers.
fn draw_tiles(s: &mut String, roof: &Roof, k: f64) {
    let Some(kind) = roof.covering.tiles else {
        return;
    };
    if crate::coarse() {
        return;
    }
    let mut d = String::new();
    for plane in roof.planes() {
        let Some(u) = plane.slope.direction() else {
            continue;
        };
        let across = u.perp_left();
        let holes: Vec<Vec<Point2>> = roof
            .plane_holes(plane.side)
            .into_iter()
            .filter(|h| {
                h.iter()
                    .all(|q| archisim_core::point_in_polygon(*q, &plane.outline))
            })
            .collect();
        let rings: Vec<&Vec<Point2>> = std::iter::once(&plane.outline)
            .chain(holes.iter())
            .collect();
        let from = plane.slope.from;
        let (along, gap) = match (kind.ribs(), kind.course()) {
            (Some(g), _) => (true, g),
            (None, Some(c)) => (false, c / (1.0 + plane.slope.slope.powi(2)).sqrt()),
            (None, None) => continue,
        };
        // Lines along `dir`, one every `gap` across `normal`.
        let (dir, normal) = if along { (u, across) } else { (across, u) };
        let cs: Vec<f64> = plane
            .outline
            .iter()
            .map(|p| (*p - from).dot(normal))
            .collect();
        let (c0, c1) = cs
            .iter()
            .fold((f64::INFINITY, f64::NEG_INFINITY), |(a, b), &c| {
                (a.min(c), b.max(c))
            });
        let mut c = c0 + gap / 2.0;
        while c < c1 {
            let o = from + normal * c;
            for (ta, tb) in runs(o, dir, &rings) {
                let _ = write!(d, "M{} L{} ", pt(o + dir * ta), pt(o + dir * tb));
            }
            c += gap;
        }
    }
    if !d.is_empty() {
        let _ = writeln!(
            s,
            r#"<path class="pat" d="{d}" fill="none" stroke="{INK}" stroke-width="{}"/>"#,
            num(pens().hatch * k)
        );
    }
}

/// Where a line through `o` along `u` is inside the first ring and out of
/// the others.
fn runs(o: Point2, u: Vec2, rings: &[&Vec<Point2>]) -> Vec<(f64, f64)> {
    let n = u.perp_left();
    let mut ts = Vec::new();
    for ring in rings {
        let m = ring.len();
        for i in 0..m {
            let (a, b) = (ring[i], ring[(i + 1) % m]);
            let (da, db) = ((a - o).dot(n), (b - o).dot(n));
            if (da > 0.0) != (db > 0.0) {
                let t = da / (da - db);
                ts.push((a + (b - a) * t - o).dot(u));
            }
        }
    }
    ts.sort_by(f64::total_cmp);
    ts.chunks(2)
        .filter(|c| c.len() == 2)
        .map(|c| (c[0], c[1]))
        .collect()
}

/// Dormers in plan: their own roof as seen from above (its planes, the
/// ridge between them) or, for a roof window, its frame and glass.
fn draw_dormers(s: &mut String, roof: &Roof, k: f64) {
    for d in &roof.dormers {
        let Some(shape) = roof.dormer_shape(d) else {
            continue;
        };
        if d.kind == DormerKind::Skylight {
            let f = shape.frame;
            let (hw, len) = (d.width / 2.0, shape.back);
            let fw = archisim_core::DORMER_FRAME;
            let inner: Vec<Point2> = [
                (-hw + fw, fw),
                (hw - fw, fw),
                (hw - fw, len - fw),
                (-hw + fw, len - fw),
            ]
            .iter()
            .map(|&(x, y)| f.plan(x, y))
            .collect();
            let _ = writeln!(
                s,
                r#"<path d="{}" fill="{SLAB_COVER}" stroke="{INK}" stroke-width="{}"/><path d="{}" fill="rgb(214,232,240)" stroke="{INK}" stroke-width="{}"/>"#,
                path(&shape.footprint),
                num(pens().view * k),
                path(&inner),
                num(pens().hatch * k)
            );
            continue;
        }
        for plane in &shape.planes {
            let _ = writeln!(
                s,
                r#"<path d="{}" fill="{SLAB_COVER}" stroke="{INK}" stroke-width="{}"/>"#,
                path(&plane.outline),
                num(pens().view * k)
            );
        }
        // The front's line under its eave, dashed.
        let f = shape.frame;
        let hw = d.width / 2.0;
        let _ = writeln!(
            s,
            r#"<path d="M{} L{}" fill="none" stroke="{INK}" stroke-width="{}" stroke-dasharray="{} {}"/>"#,
            pt(f.plan(-hw, 0.0)),
            pt(f.plan(hw, 0.0)),
            num(pens().hatch * k),
            num(1.2 * k),
            num(0.8 * k)
        );
    }
}

/// Gutters as two lines along the eaves; downpipes as circles against the
/// walls.
fn draw_gutters(s: &mut String, roof: &Roof, k: f64) {
    let Some(g) = &roof.covering.gutter else {
        return;
    };
    let mut d = String::new();
    for run in roof.gutter_runs() {
        for off in [-g.size / 2.0, g.size / 2.0] {
            let _ = write!(
                d,
                "M{} L{} ",
                pt(run.start + run.out * off),
                pt(run.end + run.out * off)
            );
        }
        for p in &run.downpipes {
            let r = g.downpipe / 2.0;
            let _ = write!(
                d,
                "M{} A{} {} 0 1 0 {} A{} {} 0 1 0 {} ",
                pt(*p + Vec2::new(r, 0.0)),
                num(r),
                num(r),
                pt(*p - Vec2::new(r, 0.0)),
                num(r),
                num(r),
                pt(*p + Vec2::new(r, 0.0))
            );
        }
    }
    if !d.is_empty() {
        let _ = writeln!(
            s,
            r#"<path d="{d}" fill="none" stroke="{INK}" stroke-width="{}"/>"#,
            num(pens().view * k)
        );
    }
}

/// A percentage with up to one decimal and a decimal comma.
fn percent(v: f64) -> String {
    let r = (v * 10.0).round() / 10.0;
    if (r - r.round()).abs() < 1e-9 {
        format!("{}", r.round() as i64)
    } else {
        format!("{r:.1}").replace('.', ",")
    }
}
