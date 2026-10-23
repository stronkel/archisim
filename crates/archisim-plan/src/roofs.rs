//! Pitched roofs in plan, as a roof plan ("planta de cubiertas"): each
//! plane in the light tone of what is seen below the cut, its ridges, hips
//! and eaves as view lines, the footprint (the walls under the eaves)
//! dashed, and on each plane an arrow down the slope (where the water
//! runs) with its pitch in per cent.

use std::fmt::Write as _;

use archisim_core::{Point2, Roof};

use crate::lettering::{text_path, Align};
use crate::{annotate, num, path, pens, pt, INK, SLAB_COVER};

/// Every point of the roof that the plan must show.
pub fn roof_points(roof: &Roof) -> Vec<Point2> {
    let mut v = roof.eaves();
    v.extend(roof.dormers.iter().map(|d| d.position));
    v
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
    draw_dormers(s, roof, k);
    s.push_str("</g>\n");
}

/// The dormers over the roof: the outline of their roof (with its eaves)
/// filled over the planes, their ridge or the top of their front, and
/// their front wall.
fn draw_dormers(s: &mut String, roof: &Roof, k: f64) {
    for d in &roof.dormers {
        let Ok(shape) = roof.dormer_shape(d) else {
            continue;
        };
        let f = &shape.frame;
        let w = d.width / 2.0;
        let e = w + 0.15;
        let outline: Vec<Point2> = match d.kind {
            archisim_core::DormerKind::Shed => vec![
                f.at(-e, -0.15),
                f.at(e, -0.15),
                f.at(e, shape.depth),
                f.at(-e, shape.depth),
            ],
            archisim_core::DormerKind::Gable => vec![
                f.at(-e, -0.15),
                f.at(e, -0.15),
                f.at(e, shape.eave_depth),
                f.at(0.0, shape.depth),
                f.at(-e, shape.eave_depth),
            ],
        };
        let _ = writeln!(
            s,
            r#"<path d="{}" fill="{SLAB_COVER}" stroke="{INK}" stroke-width="{}"/>"#,
            path(&outline),
            num(pens().view * 1.5 * k)
        );
        // The front wall, dashed under the eave.
        let _ = writeln!(
            s,
            r#"<path d="M{} L{}" fill="none" stroke="{INK}" stroke-width="{}" stroke-dasharray="{} {}"/>"#,
            pt(f.at(-w, 0.0)),
            pt(f.at(w, 0.0)),
            num(pens().hatch * k),
            num(1.0 * k),
            num(0.8 * k)
        );
        if d.kind == archisim_core::DormerKind::Gable {
            let _ = writeln!(
                s,
                r#"<path d="M{} L{}" fill="none" stroke="{INK}" stroke-width="{}"/>"#,
                pt(f.at(0.0, -0.15)),
                pt(f.at(0.0, shape.depth)),
                num(pens().view * k)
            );
        }
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
