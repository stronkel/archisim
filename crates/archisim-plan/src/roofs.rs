//! Pitched roofs in plan, as a roof plan ("planta de cubiertas"): each
//! plane in the light tone of what is seen below the cut, its ridges, hips
//! and eaves as view lines, the footprint (the walls under the eaves)
//! dashed, and on each plane an arrow down the slope (where the water
//! runs) with its pitch in per cent.

use std::fmt::Write as _;

use archisim_core::{Point2, Roof};

use crate::lettering::{text_path, Align, STROKE};
use crate::{annotate, num, path, pt, HATCH_LINE, INK, SLAB_COVER, VIEW_LINE};

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
            num(VIEW_LINE * k)
        );
    }
    // The walls under the eaves.
    if roof.overhang > 1e-6 {
        let _ = writeln!(
            s,
            r#"<path d="{}" fill="none" stroke="{INK}" stroke-width="{}" stroke-dasharray="{} {}"/>"#,
            path(&roof.outline),
            num(HATCH_LINE * k),
            num(2.0 * k),
            num(1.2 * k)
        );
    }
    // The eaves, a little heavier.
    let _ = writeln!(
        s,
        r#"<path d="{}" fill="none" stroke="{INK}" stroke-width="{}"/>"#,
        path(&roof.eaves()),
        num(VIEW_LINE * 1.5 * k)
    );
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
            num(VIEW_LINE * k),
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
            r#"<path d="{}" fill="none" stroke="{INK}" stroke-width="{}" stroke-linecap="round" stroke-linejoin="round"/>"#,
            text_path(&text, at, read, th, Align::Centre),
            num(STROKE * th)
        );
    }
    s.push_str("</g>\n");
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
