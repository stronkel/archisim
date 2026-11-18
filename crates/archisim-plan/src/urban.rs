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
            // Its zone tinted, as the zoning maps paint it (N7).
            if let Some(z) = p.zone {
                let [r, g, b] = z.rgb();
                let _ = writeln!(
                    s,
                    r#"<path class="plot-zone" d="{}" fill="rgb({r},{g},{b})" fill-opacity="0.22" stroke="none"/>"#,
                    path(&p.outline)
                );
            }
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
            // Only where the text fits in the plot: a block cut into narrow
            // plots would be covered with names.
            let (lo, hi) = p.outline.iter().fold(
                (
                    Point2::new(f64::MAX, f64::MAX),
                    Point2::new(f64::MIN, f64::MIN),
                ),
                |(lo, hi), q| {
                    (
                        Point2::new(lo.x.min(q.x), lo.y.min(q.y)),
                        Point2::new(hi.x.max(q.x), hi.y.max(q.y)),
                    )
                },
            );
            let wide = p.name.chars().count().max(area.len() + 3) as f64 * LABEL * k * 0.6;
            if hi.x - lo.x < wide || hi.y - lo.y < 7.0 * LABEL * k {
                return;
            }
            label(s, c + Point2::new(0.0, 2.0 * LABEL * k), &p.name, k);
            label(s, c, &format!("{area} m²"), k);
            let mut below = 1.0;
            if !p.reference.is_empty() {
                label(s, c - Point2::new(0.0, 2.0 * LABEL * k), &p.reference, k);
                below += 1.0;
            }
            if p.locked {
                label(
                    s,
                    c - Point2::new(0.0, 2.0 * below * LABEL * k),
                    "bloqueada",
                    k,
                );
            }
        });
    }
    s.push_str("</g>\n");
}

/// The edge of the area of the surroundings (N8), a long dash, and the
/// connections of the streets with the outside: an arrow out of the area
/// at each street cut by its edge, with the street's name.
pub fn draw_outside(s: &mut String, model: &Model, level: LevelId, k: f64) {
    let Some(area) = model.surroundings() else {
        return;
    };
    let _ = writeln!(
        s,
        r#"<path class="surroundings" d="{}" fill="none" stroke="{INK}" stroke-width="{}" stroke-dasharray="{} {}" stroke-opacity="0.6"/>"#,
        path(area),
        num(pens().thin * k),
        num(12.0 * k),
        num(3.0 * k)
    );
    for e in model.outside_ends(level) {
        let Some(r) = model.road(e.road) else {
            continue;
        };
        let side = e.out.perp_left();
        let w = (r.section.width() / 2.0).min(6.0 * k);
        // An arrow out of the area, as long as the street is wide.
        let tip = e.point + e.out * (w * 2.0 + 4.0 * k);
        let base = e.point + e.out * (2.0 * k);
        let arrow = [base + side * (w * 0.5), tip, base - side * (w * 0.5)];
        let _ = writeln!(
            s,
            r#"<path class="outside-connection" d="M{} L{} L{} Z" fill="{INK}" fill-opacity="0.75" stroke="none"/>"#,
            pt(arrow[0]),
            pt(arrow[1]),
            pt(arrow[2])
        );
        if !r.name.is_empty() {
            let h = LABEL * k;
            // Along the street, reading from left to right.
            let dir = if e.out.x < -1e-9 { e.out * -1.0 } else { e.out };
            let half = r.name.chars().count() as f64 * h * 0.3;
            let at = tip + e.out * (h + half) - dir.perp_left() * (h / 2.0);
            let _ = writeln!(
                s,
                r#"<path class="outside-connection" d="{}" {}/>"#,
                text_path(&r.name, at, dir, h, Align::Centre),
                crate::lettering::paint(h, INK)
            );
        }
    }
}

/// The roads of a level.
pub fn draw_roads(
    s: &mut String,
    model: &Model,
    level: LevelId,
    k: f64,
    visible: &dyn Fn(&[Point2]) -> bool,
) {
    if model.roads_on_level(level).next().is_none()
        && model.markings_on_level(level).next().is_none()
        && model.surfaces_on_level(level).next().is_none()
    {
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
            for j in junctions
                .iter()
                .filter(|j| j.owner == r.id && visible(&j.plate))
            {
                let plate_fill = if j
                    .ring
                    .and_then(|c| model.roundabout(c))
                    .is_some_and(|c| c.plaza)
                {
                    "#ece6d8"
                } else if j.kind == archisim_core::NodeKind::Shared {
                    // A shared surface: paved as the sidewalks (N1).
                    "#f1eee6"
                } else {
                    "#dcdcdc"
                };
                let parts = std::iter::once((&j.plate, plate_fill, pens().view))
                    // The ramps of a raised node, a little darker.
                    .chain(j.ramps.iter().map(|w| (w, "#cfcfcf", pens().thin)))
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
                // Its bump-outs' kerbs, stop and give-way lines and signs
                // (N1) lie on its roads, so they go over them.
                for b in &j.bump_kerbs {
                    let _ = writeln!(
                        over,
                        r##"<path d="{}" fill="#ffffff" stroke="{INK}" stroke-width="{}"/>"##,
                        path(b),
                        num(pens().cut * k)
                    );
                }
                for m in j.marks.iter().chain(j.arm_paint.iter().map(|(_, p)| p)) {
                    let _ = writeln!(
                        over,
                        r#"<path d="{}" fill="white" stroke="{INK}" stroke-width="{}"/>"#,
                        path(m),
                        num(pens().thin * k * 0.5)
                    );
                }
                for (at, dir, sign) in &j.signs {
                    draw_sign(&mut over, *at, *dir, *sign, k);
                }
            }
            for p in r.plan_pieces_except(&gaps) {
                if !visible(&p.outline) {
                    continue;
                }
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
                // Lowered at a crossing (U4): the kerb flush with the road
                // and the sidewalk with tactile paving.
                let (fill, width) = match p.lowered {
                    archisim_core::Lowered::No => (fill, width),
                    _ if p.band == RoadBand::Kerb => ("#ffffff", pens().view),
                    archisim_core::Lowered::Flush => ("#e2bf8f", pens().view),
                    archisim_core::Lowered::Ramp { .. } => ("#ecdcc2", pens().view),
                };
                let _ = writeln!(
                    s,
                    r#"<path d="{}" fill="{fill}" stroke="{INK}" stroke-width="{}"/>"#,
                    path(&p.outline),
                    num(width * k)
                );
            }
            // The tree pits of its sidewalks (U3).
            for pit in r.visible_tree_pits(&gaps) {
                if !visible(&pit) {
                    continue;
                }
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
            // The road markings: lane lines, parking bays, zebra crossings
            // (not near a node that hides them, N1).
            let mark_gaps = archisim_geom::crossroads::marking_gaps(&junctions, r.id);
            for m in r.visible_markings(&mark_gaps) {
                if !visible(&m) {
                    continue;
                }
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
    // The urban surfaces (N5): their paving, their edge, their joints
    // (from 1:200) and their bays, over the streets.
    for x in model.surfaces_on_level(level) {
        if !visible(&x.outline) {
            continue;
        }
        use archisim_core::SurfaceKind as K;
        let fill = match x.kind {
            K::Square => "#ebe5d8",
            K::Island | K::Median => "#dcdcdc",
            K::Parking => "#d2d2d2",
            K::Garden => "#d3e6bb",
        };
        crate::dim::element(s, x.id.0, |s| {
            let inner = x.inner();
            let edged =
                x.edge != archisim_core::SurfaceEdge::None && inner.len() == x.outline.len();
            if edged {
                // The edge: the outline with the inner one as a hole.
                let mut back = inner.clone();
                back.reverse();
                let _ = writeln!(
                    s,
                    r##"<path d="{} {}" fill="#bdb6a8" fill-rule="evenodd" stroke="none"/>"##,
                    path(&x.outline),
                    path(&back)
                );
            }
            let _ = writeln!(
                s,
                r#"<path d="{}" fill="{fill}" stroke="{INK}" stroke-width="{}"/>"#,
                path(if edged { &inner } else { &x.outline }),
                num(pens().thin * k)
            );
            if k <= 0.2 {
                for (a, b) in x.joint_lines() {
                    let _ = writeln!(
                        s,
                        r##"<path d="M{},{} L{},{}" stroke="#9c9486" stroke-width="{}" fill="none"/>"##,
                        num(a.x),
                        num(-a.y),
                        num(b.x),
                        num(-b.y),
                        num(pens().thin * k * 0.5)
                    );
                }
            }
            let _ = writeln!(
                s,
                r#"<path d="{}" fill="none" stroke="{INK}" stroke-width="{}"/>"#,
                path(&x.outline),
                num(pens().view * k)
            );
            for b in x.bay_lines() {
                let _ = writeln!(
                    s,
                    r#"<path d="{}" fill="white" stroke="{INK}" stroke-width="{}"/>"#,
                    path(&b),
                    num(pens().thin * k * 0.5)
                );
            }
        });
    }
    // The road markings at will (U4), over the roads.
    for m in model.markings_on_level(level) {
        let shapes = m.shapes();
        if !shapes.iter().any(|x| visible(x)) {
            continue;
        }
        let rgb = model
            .material(m.material)
            .map_or([245, 245, 240], |x| x.cut_color);
        let colour = format!("#{:02x}{:02x}{:02x}", rgb[0], rgb[1], rgb[2]);
        crate::dim::element(s, m.id.0, |s| {
            if let Some((plateau, ramps)) = m.raised_plateau() {
                let _ = writeln!(
                    s,
                    r##"<path d="{}" fill="#d6d6d6" stroke="{INK}" stroke-width="{}"/>"##,
                    path(&plateau),
                    num(pens().view * k)
                );
                for r in ramps {
                    let _ = writeln!(
                        s,
                        r##"<path d="{}" fill="#c8c8c8" stroke="{INK}" stroke-width="{}" stroke-dasharray="{} {}"/>"##,
                        path(&r),
                        num(pens().thin * k),
                        num(0.3 * k),
                        num(0.3 * k)
                    );
                }
            }
            let fill = m.fill();
            if let Some(f) = &fill {
                let _ = writeln!(
                    s,
                    r#"<path d="{}" fill="{colour}" stroke="none"/>"#,
                    path(f)
                );
            }
            let paint = if fill.is_some() {
                "#f5f5f0".to_string()
            } else {
                colour.clone()
            };
            for x in &shapes {
                let _ = writeln!(
                    s,
                    r#"<path d="{}" fill="{paint}" stroke="{INK}" stroke-width="{}"/>"#,
                    path(x),
                    num(pens().thin * k * 0.5)
                );
            }
        });
    }
    s.push_str(&over);
    s.push_str("</g>\n");
}

/// A sign of a node seen from above (N1): a triangle (give way), an octagon
/// (stop) or a circle (traffic lights), 0,9 m across on paper's scale,
/// beside its post.
fn draw_sign(
    s: &mut String,
    at: Point2,
    dir: archisim_core::Vec2,
    sign: archisim_core::ArmSign,
    k: f64,
) {
    use archisim_core::ArmSign;
    // At least 3 mm on paper so it reads at 1:500.
    let r = (0.45_f64).max(1.5 * k);
    let c = at + dir * r;
    let ring = |n: usize, turn: f64| -> Vec<Point2> {
        (0..n)
            .map(|i| {
                let t = turn + std::f64::consts::TAU * i as f64 / n as f64;
                c + Point2::new(t.cos(), t.sin()) * r
            })
            .collect()
    };
    let (poly, fill) = match sign {
        ArmSign::None => return,
        // Pointing down the street, towards the traffic.
        ArmSign::Yield => (
            ring(3, dir.y.atan2(dir.x) + std::f64::consts::PI),
            "#ffffff",
        ),
        ArmSign::Stop => (ring(8, std::f64::consts::PI / 8.0), "#d9534f"),
        ArmSign::Lights => (ring(16, 0.0), "#5a5a5a"),
    };
    let _ = writeln!(
        s,
        r#"<path d="{}" fill="{fill}" stroke="{INK}" stroke-width="{}"/>"#,
        path(&poly),
        num(pens().thin * k)
    );
}
