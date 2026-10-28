//! Pools built in the ground, in plan (paquete 8, tanda 8d).
//!
//! Seen from above: the coping as a band with its outer and inner edges,
//! the water in a light blue, the outside of the shell under the coping
//! dashed (it is hidden), an arrow down the slope from the shallow end to
//! the deep one with both depths, and its name with the water's area.

use std::fmt::Write as _;

use archisim_core::{offset_contour, LevelId, Model, Point2, Pool};

use crate::lettering::{text_path, Align};
use crate::{num, path, pens, pt, INK};

/// Light blue of the water (a presentation style may change it).
const WATER: &str = "#e2f0f7";
/// Height of the labels, mm of paper.
const LABEL: f64 = 2.0;

fn label(s: &mut String, at: Point2, text: &str, k: f64) {
    let h = LABEL * k;
    let _ = writeln!(
        s,
        r#"<path d="{}" {}/>"#,
        text_path(
            text,
            at - Point2::new(0.0, h / 2.0),
            Point2::new(1.0, 0.0),
            h,
            Align::Centre
        ),
        crate::lettering::paint(h, INK)
    );
}

/// Metres with two decimals and a comma.
fn metres(v: f64) -> String {
    format!("{v:.2}").replace('.', ",")
}

/// The points that mark a pool, for the snaps.
pub fn pool_points(p: &Pool) -> Vec<Point2> {
    let mut out = p.outline.clone();
    out.extend(p.coping_outline());
    out
}

/// One pool.
pub fn draw_pool(s: &mut String, p: &Pool, k: f64) {
    let coping = p.coping_outline();
    let _ = writeln!(
        s,
        r#"<path d="{}" fill="white" stroke="{INK}" stroke-width="{}"/>"#,
        path(&coping),
        num(pens().view * k)
    );
    let _ = writeln!(
        s,
        r#"<path d="{}" fill="{WATER}" stroke="{INK}" stroke-width="{}"/>"#,
        path(&p.outline),
        num(pens().view * k)
    );
    // The outside of the shell, under the coping.
    if p.wall < p.coping - 0.01 {
        if let Some(shell) = offset_contour(&p.outline, p.wall) {
            let _ = writeln!(
                s,
                r#"<path d="{}" fill="none" stroke="{INK}" stroke-width="{}" stroke-dasharray="{} {}"/>"#,
                path(&shell),
                num(pens().thin * k),
                num(2.0 * k),
                num(1.0 * k)
            );
        }
    }
    // The slope: an arrow from the shallow end to the deep one.
    let (c, sn) = (p.slope.cos(), p.slope.sin());
    let along = |q: &Point2| q.x * c + q.y * sn;
    let (lo, hi) = p
        .outline
        .iter()
        .map(along)
        .fold((f64::MAX, f64::MIN), |(a, b), u| (a.min(u), b.max(u)));
    let mid = p.outline.iter().fold(Point2::new(0.0, 0.0), |a, q| a + *q)
        * (1.0 / p.outline.len().max(1) as f64);
    let dir = Point2::new(c, sn);
    let span = (hi - lo) * 0.3;
    let (a, b) = (mid - dir * span, mid + dir * span);
    if (p.deep - p.shallow).abs() > 1e-6 && span > 0.2 {
        let head = 2.0 * k;
        let side = Point2::new(-sn, c);
        let (w1, w2) = (
            b - dir * head + side * head * 0.4,
            b - dir * head - side * head * 0.4,
        );
        let _ = writeln!(
            s,
            r#"<path d="M{} L{} M{} L{} L{}" fill="none" stroke="{INK}" stroke-width="{}"/>"#,
            pt(a),
            pt(b),
            pt(w1),
            pt(b),
            pt(w2),
            num(pens().thin * k)
        );
        label(s, a - dir * (3.0 * k), &metres(p.shallow), k);
        label(s, b + dir * (3.0 * k), &metres(p.deep), k);
    }
    let area = format!("{} m²", metres(p.area()));
    label(s, mid + Point2::new(0.0, 3.0 * k), &p.name, k);
    label(s, mid - Point2::new(0.0, 3.0 * k), &area, k);
}

/// The pools of a level.
pub fn draw_pools(
    s: &mut String,
    model: &Model,
    level: LevelId,
    k: f64,
    visible: &dyn Fn(&[Point2]) -> bool,
) {
    if model.pools_on_level(level).next().is_none() {
        return;
    }
    s.push_str("<g id=\"pools\">\n");
    for p in model.pools_on_level(level) {
        let o = p.coping_outline();
        if !visible(&o) {
            continue;
        }
        crate::dim::element(s, p.id.0, |s| draw_pool(s, p, k));
    }
    s.push_str("</g>\n");
}
