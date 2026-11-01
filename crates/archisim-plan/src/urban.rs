//! Plots and roads in plan.
//!
//! A plot's boundary is a thick dash-dot-dot line (the usual line of a
//! "linde"), its setback a thin dashed line inside, and its name, area
//! and cadastral reference in the middle. A road is its bands: the
//! carriageway and parking in a light grey with the parking bays marked,
//! the kerbs as cut lines, the sidewalks in a lighter tint, and its axis
//! dash-dot.

use std::fmt::Write as _;

use archisim_core::{LevelId, Model, Point2, RoadBand};

use crate::lettering::{text_path, Align};
use crate::{num, path, pens, pt, INK};

const LABEL: f64 = 2.5;

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

fn centre(o: &[Point2]) -> Point2 {
    o.iter().fold(Point2::new(0.0, 0.0), |a, p| a + *p) * (1.0 / o.len().max(1) as f64)
}

/// The plots of a level.
pub fn draw_plots(
    s: &mut String,
    model: &Model,
    level: LevelId,
    k: f64,
    visible: &dyn Fn(&[Point2]) -> bool,
) {
    if model.plots_on_level(level).next().is_none() {
        return;
    }
    s.push_str("<g id=\"plots\">\n");
    for p in model.plots_on_level(level) {
        if !visible(&p.outline) {
            continue;
        }
        crate::dim::element(s, p.id.0, |s| {
            let _ = writeln!(
                s,
                r#"<path d="{}" fill="none" stroke="{INK}" stroke-width="{}" stroke-dasharray="{} {} {} {} {} {}"/>"#,
                path(&p.outline),
                num(pens().cut * k),
                num(10.0 * k),
                num(1.5 * k),
                num(1.0 * k),
                num(1.5 * k),
                num(1.0 * k),
                num(1.5 * k)
            );
            if p.setback > 0.0 {
                if let Some(b) = p.buildable() {
                    let _ = writeln!(
                        s,
                        r#"<path d="{}" fill="none" stroke="{INK}" stroke-width="{}" stroke-dasharray="{} {}"/>"#,
                        path(&b),
                        num(pens().thin * k),
                        num(3.0 * k),
                        num(1.5 * k)
                    );
                }
            }
            let c = centre(&p.outline);
            let area = format!("{:.2}", p.area()).replace('.', ",");
            label(s, c + Point2::new(0.0, 2.0 * LABEL * k), &p.name, k);
            label(s, c, &format!("{area} m²"), k);
            if !p.reference.is_empty() {
                label(s, c - Point2::new(0.0, 2.0 * LABEL * k), &p.reference, k);
            }
        });
    }
    s.push_str("</g>\n");
}

/// The roads of a level.
pub fn draw_roads(
    s: &mut String,
    model: &Model,
    level: LevelId,
    k: f64,
    visible: &dyn Fn(&[Point2]) -> bool,
) {
    if model.roads_on_level(level).next().is_none() {
        return;
    }
    s.push_str("<g id=\"roads\">\n");
    for r in model.roads_on_level(level) {
        let outline = r.outline();
        if outline.len() < 3 || !visible(&outline) {
            continue;
        }
        crate::dim::element(s, r.id.0, |s| {
            for p in r.pieces() {
                let (fill, width) = match p.band {
                    RoadBand::Carriageway => ("#dcdcdc", pens().view),
                    RoadBand::Parking => ("#e6e6e6", pens().view),
                    RoadBand::Kerb => ("#ffffff", pens().cut),
                    RoadBand::Sidewalk => ("#f1eee6", pens().view),
                    RoadBand::BikeLane => ("#e9c9bf", pens().view),
                    RoadBand::Planting => ("#d4dfc0", pens().view),
                    RoadBand::Median => ("#e8e4d6", pens().view),
                };
                let _ = writeln!(
                    s,
                    r#"<path d="{}" fill="{fill}" stroke="{INK}" stroke-width="{}"/>"#,
                    path(&p.outline),
                    num(width * k)
                );
            }
            // The road markings: lane lines, parking bays, zebra crossings.
            for m in r.markings() {
                let _ = writeln!(
                    s,
                    r#"<path d="{}" fill="white" stroke="{INK}" stroke-width="{}"/>"#,
                    path(&m),
                    num(pens().thin * k * 0.5)
                );
            }
            let sec = &r.section;
            let _ = writeln!(
                s,
                r#"<path d="M{}" fill="none" stroke="{INK}" stroke-width="{}" stroke-dasharray="{} {} {} {}"/>"#,
                r.axis.iter().map(|p| pt(*p)).collect::<Vec<_>>().join(" L"),
                num(pens().thin * k),
                num(12.0 * k),
                num(1.5 * k),
                num(1.5 * k),
                num(1.5 * k)
            );
            // The name in the middle of the longest stretch, along it.
            if let Some(seg) = r
                .axis
                .windows(2)
                .max_by(|x, y| x[0].distance(x[1]).total_cmp(&y[0].distance(y[1])))
            {
                let (a, b) = (seg[0], seg[1]);
                if let Some(mut u) = (b - a).normalized() {
                    if u.x < -1e-9 || (u.x.abs() < 1e-9 && u.y < 0.0) {
                        u = u * -1.0;
                    }
                    let m =
                        (a + b) * 0.5 + u.perp_left() * (sec.width_of(RoadBand::Carriageway) / 4.0);
                    let h = LABEL * k;
                    let _ = writeln!(
                        s,
                        r#"<path d="{}" {}/>"#,
                        text_path(&r.name, m, u, h, Align::Centre),
                        crate::lettering::paint(h, INK)
                    );
                }
            }
        });
    }
    s.push_str("</g>\n");
}
