//! Decals in plan (U8): one on a floor or a street drawn as its picture,
//! turned as it is; one on a wall, a fine dashed line along its trace, so
//! that it can be found and picked.

use std::fmt::Write as _;

use archisim_core::{LevelId, Model, Point2};

use crate::{num, pens, INK};

pub fn draw_decals(
    s: &mut String,
    model: &Model,
    level: LevelId,
    k: f64,
    visible: &dyn Fn(&[Point2]) -> bool,
) {
    let list: Vec<_> = model.decals_on_level(level).collect();
    if list.is_empty() {
        return;
    }
    s.push_str("<g id=\"decals\">\n");
    let base = model.level(level).map_or(0.0, |l| l.elevation);
    for d in list {
        let c = d.corners(base);
        let plan: Vec<Point2> = c.iter().map(|q| Point2::new(q[0], q[1])).collect();
        if !visible(&plan) {
            continue;
        }
        crate::dim::element(s, d.id.0, |s| {
            if d.lying() && d.normal[2] > 0.0 {
                let _ = writeln!(
                    s,
                    r#"<image x="{}" y="{}" width="{}" height="{}" preserveAspectRatio="none" transform="translate({},{}) rotate({})" href="data:image/png;base64,{}"/>"#,
                    num(-d.width / 2.0),
                    num(-d.height() / 2.0),
                    num(d.width),
                    num(d.height()),
                    num(d.centre[0]),
                    num(-d.centre[1]),
                    num(-d.rotation.to_degrees()),
                    crate::base64(&d.image.png)
                );
            } else {
                let _ = writeln!(
                    s,
                    r#"<path d="M{} L{}" stroke="{INK}" stroke-width="{}" stroke-dasharray="{} {}" fill="none"/>"#,
                    crate::pt(plan[0]),
                    crate::pt(plan[1]),
                    num(pens().thin * k),
                    num(1.5 * k),
                    num(0.8 * k)
                );
            }
        });
    }
    s.push_str("</g>\n");
}
