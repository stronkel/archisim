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
    let junctions = archisim_geom::crossroads::junctions(model, level);
    let base = model.level(level).map_or(0.0, |l| l.elevation);
    // The profiles' grades and heights go over every road.
    let mut over = String::new();
    for r in model.roads_on_level(level) {
        let gaps = archisim_geom::crossroads::road_gaps(&junctions, r.id);
        let outline = r.outline();
        if outline.len() < 3 || !visible(&outline) {
            continue;
        }
        crate::dim::element(s, r.id.0, |s| {
            // Its junctions: the plate, the kerbs round the corners and the
            // sidewalks at them (U1).
            for j in junctions.iter().filter(|j| j.owner == r.id) {
                let plate_fill = if j
                    .ring
                    .and_then(|c| model.roundabout(c))
                    .is_some_and(|c| c.plaza)
                {
                    "#ece6d8"
                } else {
                    "#dcdcdc"
                };
                let parts = std::iter::once((&j.plate, plate_fill, pens().view))
                    .chain(j.sidewalks.iter().map(|w| (w, "#f1eee6", pens().view)))
                    .chain(j.kerbs.iter().map(|w| (w, "#ffffff", pens().cut)))
                    .chain(j.parts.iter().map(|p| {
                        (
                            &p.outline,
                            match p.band {
                                RoadBand::Planting => "#d4dfc0",
                                RoadBand::Kerb => "#efefef",
                                _ => "#e8e4d6",
                            },
                            if p.band == RoadBand::Kerb {
                                pens().thin
                            } else {
                                pens().cut
                            },
                        )
                    }));
                for (poly, fill, width) in parts {
                    let _ = writeln!(
                        s,
                        r#"<path d="{}" fill="{fill}" stroke="{INK}" stroke-width="{}"/>"#,
                        path(poly),
                        num(width * k)
                    );
                }
            }
            for p in r.plan_pieces_except(&gaps) {
                let (fill, width) = match p.band {
                    RoadBand::Carriageway => ("#dcdcdc", pens().view),
                    RoadBand::Parking => ("#e6e6e6", pens().view),
                    RoadBand::Kerb => ("#ffffff", pens().cut),
                    RoadBand::Sidewalk => ("#f1eee6", pens().view),
                    RoadBand::BikeLane => ("#e9c9bf", pens().view),
                    RoadBand::Planting => ("#d4dfc0", pens().view),
                    RoadBand::Median => ("#e8e4d6", pens().view),
                    RoadBand::BusLane => ("#d9b3a8", pens().view),
                    RoadBand::Tram => ("#c9d8b0", pens().view),
                    RoadBand::Shoulder => ("#e3e3e3", pens().view),
                    RoadBand::Ditch => ("#c7d3b5", pens().view),
                    RoadBand::TreePits => ("#f1eee6", pens().view),
                    RoadBand::BikePath => ("#ead2c8", pens().view),
                };
                let _ = writeln!(
                    s,
                    r#"<path d="{}" fill="{fill}" stroke="{INK}" stroke-width="{}"/>"#,
                    path(&p.outline),
                    num(width * k)
                );
            }
            // The tree pits of its sidewalks (U3).
            for pit in r.tree_pits_except(&gaps) {
                let _ = writeln!(
                    s,
                    r##"<path d="{}" fill="#c9b48f" stroke="{INK}" stroke-width="{}"/>"##,
                    path(&pit),
                    num(pens().view * k)
                );
            }
            // The rails of its tram strips (U3).
            for rail in r.rail_lines() {
                let _ = writeln!(
                    s,
                    r#"<path d="M{}" fill="none" stroke="{INK}" stroke-width="{}"/>"#,
                    rail.iter().map(|p| pt(*p)).collect::<Vec<_>>().join(" L"),
                    num(pens().cut * k)
                );
            }
            // The road markings: lane lines, parking bays, zebra crossings.
            for m in r.markings_except(&gaps) {
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
            // Its profile: the grade of each stretch, with an arrow downhill,
            // and the height of each point (U1).
            if r.sloped() {
                let s = &mut over;
                let h = LABEL * k * 0.8;
                let off = r.section.width() / 2.0 + 1.2 * h;
                for (a, b, g) in r.grades() {
                    if g.abs() < 1e-4 {
                        continue;
                    }
                    let (p, u) = r.axis_at((a + b) / 2.0);
                    let down = if g > 0.0 { u * -1.0 } else { u };
                    let mut read = u;
                    if read.x < -1e-9 || (read.x.abs() < 1e-9 && read.y < 0.0) {
                        read = read * -1.0;
                    }
                    let at = p + u.perp_left() * off;
                    let text = format!("{} %", format!("{:.2}", g.abs() * 100.0).replace('.', ","));
                    let _ = writeln!(
                        s,
                        r#"<path d="{}" {}/>"#,
                        text_path(&text, at, read, h, Align::Centre),
                        crate::lettering::paint(h, INK)
                    );
                    // The arrow, under the text.
                    let c = at - read.perp_left() * (0.9 * h);
                    let (t0, t1) = (c - down * (2.5 * h), c + down * (2.5 * h));
                    let w = down.perp_left() * (0.4 * h);
                    let tip = t1 - down * (0.8 * h);
                    let _ = writeln!(
                        s,
                        r#"<path d="M{} L{} M{} L{} L{}" fill="none" stroke="{INK}" stroke-width="{}"/>"#,
                        pt(t0),
                        pt(t1),
                        pt(tip + w),
                        pt(t1),
                        pt(tip - w),
                        num(pens().thin * k)
                    );
                }
                for q in &r.profile {
                    let (p, u) = r.axis_at(q.at);
                    let z = base + r.top_at(q.at);
                    let text = format!(
                        "{}{}",
                        if z >= 0.0 { "+" } else { "" },
                        format!("{z:.2}").replace('.', ",")
                    );
                    let at = p - u.perp_left() * off;
                    let _ = writeln!(
                        s,
                        r#"<path d="M{} L{}" fill="none" stroke="{INK}" stroke-width="{}"/>"#,
                        pt(p - u.perp_left() * (off * 0.6)),
                        pt(p + u.perp_left() * (off * 0.6)),
                        num(pens().thin * k)
                    );
                    let _ = writeln!(
                        s,
                        r#"<path d="{}" {}/>"#,
                        text_path(&text, at, Point2::new(1.0, 0.0), h, Align::Centre),
                        crate::lettering::paint(h, INK)
                    );
                }
            }
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
    s.push_str(&over);
    s.push_str("</g>\n");
}
