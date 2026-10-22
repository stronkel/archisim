//! Pipes, ducts and devices in plan, as installation drawings show them:
//! a pipe as a line of its system's colour (two lines, its diameter apart,
//! when it is wide enough to tell), a duct as its two sides with its
//! system's colour and its size written along it; a light as a circle with
//! a cross, a socket as a half circle with its two pins, a switch as a
//! circle with its lever, and equipment as its box crossed.

use std::fmt::Write as _;

use archisim_core::{DeviceKind, LevelId, Model, Point2};

use crate::lettering::{text_path, Align};
use crate::{num, path, pens, pt};

fn hex(c: [u8; 3]) -> String {
    format!("#{:02x}{:02x}{:02x}", c[0], c[1], c[2])
}

/// The pipes and ducts of a level, and its devices.
pub fn draw_services(
    s: &mut String,
    model: &Model,
    level: LevelId,
    k: f64,
    visible: &dyn Fn(&[Point2]) -> bool,
    walls: &[&[Point2]],
) {
    let runs: Vec<_> = model.runs_on_level(level).collect();
    let devices: Vec<_> = model.devices_on_level(level).collect();
    if runs.is_empty() && devices.is_empty() {
        return;
    }
    s.push_str("<g id=\"services\">\n");
    // The labels written, so that the next do not fall on them.
    let mut placed: Vec<[Point2; 4]> = Vec::new();
    let stretches: Vec<[Point2; 4]> = runs
        .iter()
        .flat_map(|r| {
            r.path.windows(2).filter_map(move |w| {
                let u = (w[1] - w[0]).normalized()?;
                let n = u.perp_left() * (r.width / 2.0);
                Some([w[0] - n, w[1] - n, w[1] + n, w[0] + n])
            })
        })
        .collect();
    for r in &runs {
        let o = r.outline();
        if o.len() < 3 || !visible(&o) {
            continue;
        }
        let colour = hex(r.system.color());
        crate::dim::element(s, r.id.0, |s| {
            // Two lines when the run is wider than a heavy pen on paper.
            if r.is_duct() || r.width > 1.2 * k {
                let _ = writeln!(
                    s,
                    r#"<path d="{}" fill="{colour}" fill-opacity="0.12" stroke="{colour}" stroke-width="{}"/>"#,
                    path(&o),
                    num(pens().view * k)
                );
            }
            let _ = writeln!(
                s,
                r#"<path d="M{}" fill="none" stroke="{colour}" stroke-width="{}"{}/>"#,
                r.path.iter().map(|p| pt(*p)).collect::<Vec<_>>().join(" L"),
                num(if r.is_duct() { pens().thin } else { pens().cut } * k),
                if r.is_duct() {
                    format!(r#" stroke-dasharray="{} {}""#, num(4.0 * k), num(2.0 * k))
                } else {
                    String::new()
                }
            );
            // Its system and size along the longest stretch, where it does
            // not fall on the label of another run.
            if let Some(w) = r
                .path
                .windows(2)
                .max_by(|a, b| a[0].distance(a[1]).total_cmp(&b[0].distance(b[1])))
            {
                if let Some(mut u) = (w[1] - w[0]).normalized() {
                    if u.x < -1e-9 || (u.x.abs() < 1e-9 && u.y < 0.0) {
                        u = u * -1.0;
                    }
                    let h = 2.0 * k;
                    let text = format!("{} {}", r.system.name(), r.size_text());
                    let half = archisim_draft::font::text_width(&text, h) / 2.0;
                    let n = u.perp_left();
                    let side = r.width / 2.0 + 1.0 * k + h / 2.0;
                    let label_box = |at: Point2| -> [Point2; 4] {
                        let (a, b) = (u * (half + 0.5 * k), n * (h / 2.0 + 0.5 * k));
                        [at - a - b, at + a - b, at + a + b, at - a + b]
                    };
                    let len = w[0].distance(w[1]);
                    let mut spots = Vec::new();
                    // Next to it, then farther out (beyond the runs
                    // alongside).
                    for j in 0..4 {
                        let off = side + f64::from(j) * (h + 1.0 * k);
                        for t in [0.5, 0.3, 0.7, 0.15, 0.85] {
                            for sign in [1.0, -1.0] {
                                let base = w[0] + (w[1] - w[0]) * t;
                                spots.push((base, base + n * (off * sign)));
                            }
                        }
                    }
                    let fits = len >= 2.0 * half;
                    let free = |at: &Point2, of: &[[Point2; 4]]| {
                        let bx = label_box(*at);
                        !of.iter().any(|q| boxes_overlap(q, &bx))
                    };
                    // Clear of other labels, of the pipes and of the walls;
                    // else, of the labels only.
                    let centre = spots
                        .iter()
                        .copied()
                        .find(|(base, at)| {
                            fits && free(at, &placed)
                                && free(at, &stretches)
                                && !on_walls(&label_box(*at), walls)
                                && !crosses_walls(*base, *at, walls)
                        })
                        .or_else(|| {
                            spots
                                .iter()
                                .copied()
                                .find(|(_, at)| fits && free(at, &placed))
                        })
                        .unwrap_or(spots[0])
                        .1;
                    placed.push(label_box(centre));
                    let at = centre - n * (h / 2.0);
                    let _ = writeln!(
                        s,
                        r#"<path d="{}" {}/>"#,
                        text_path(&text, at, u, h, Align::Centre),
                        crate::lettering::paint(h, &colour)
                    );
                }
            }
        });
    }
    for d in devices {
        let o = d.outline();
        if !visible(&o) {
            continue;
        }
        crate::dim::element(s, d.id.0, |s| draw_device(s, d, k));
    }
    s.push_str("</g>\n");
}

fn draw_device(s: &mut String, d: &archisim_core::ServiceDevice, k: f64) {
    let ink = "#202020";
    let w = num(pens().view * k);
    let c = d.position;
    // Symbols keep a size on paper (4 mm), bigger than the device.
    let r = (2.0 * k).max(d.width / 2.0);
    let (cs, sn) = (d.rotation.cos(), d.rotation.sin());
    let rot = |x: f64, y: f64| c + Point2::new(x * cs - y * sn, x * sn + y * cs);
    let line = |s: &mut String, a: Point2, b: Point2| {
        let _ = writeln!(
            s,
            r#"<path d="M{} L{}" fill="none" stroke="{ink}" stroke-width="{w}"/>"#,
            pt(a),
            pt(b)
        );
    };
    let circle = |s: &mut String, at: Point2, radius: f64, fill: &str| {
        let _ = writeln!(
            s,
            r#"<circle cx="{}" cy="{}" r="{}" fill="{fill}" stroke="{ink}" stroke-width="{w}"/>"#,
            num(at.x),
            num(-at.y),
            num(radius)
        );
    };
    match d.kind {
        DeviceKind::Light => {
            circle(s, c, r, "#fff7d6");
            let q = r * std::f64::consts::FRAC_1_SQRT_2;
            line(s, rot(-q, -q), rot(q, q));
            line(s, rot(-q, q), rot(q, -q));
        }
        DeviceKind::Socket => {
            // A half circle on the wall side, with its two pins.
            let arc: Vec<Point2> = (0..=12)
                .map(|i| {
                    let a = std::f64::consts::PI * i as f64 / 12.0;
                    rot(r * a.cos(), r * a.sin())
                })
                .collect();
            let _ = writeln!(
                s,
                r#"<path d="{} Z" fill="white" stroke="{ink}" stroke-width="{w}"/>"#,
                path(&arc).trim_end_matches('Z').trim_end()
            );
            line(s, rot(-r * 0.35, r * 0.2), rot(-r * 0.35, r * 0.6));
            line(s, rot(r * 0.35, r * 0.2), rot(r * 0.35, r * 0.6));
        }
        DeviceKind::Switch => {
            circle(s, c, r * 0.6, "white");
            line(s, rot(r * 0.4, r * 0.4), rot(r * 1.2, r * 1.2));
        }
        DeviceKind::Equipment => {
            let o = d.outline();
            let _ = writeln!(
                s,
                r#"<path d="{}" fill="white" stroke="{ink}" stroke-width="{}"/>"#,
                path(&o),
                num(pens().cut * k)
            );
            line(s, o[0], o[2]);
            line(s, o[1], o[3]);
            let h = 2.0 * k;
            let _ = writeln!(
                s,
                r#"<path d="{}" {}/>"#,
                text_path(
                    &d.name,
                    c - Point2::new(0.0, d.depth / 2.0 + 1.5 * h),
                    Point2::new(1.0, 0.0),
                    h,
                    Align::Centre
                ),
                crate::lettering::paint(h, ink)
            );
        }
    }
}

/// Whether two convex quadrilaterals overlap (separating axes).
fn boxes_overlap(a: &[Point2; 4], b: &[Point2; 4]) -> bool {
    for poly in [a, b] {
        for i in 0..4 {
            let e = poly[(i + 1) % 4] - poly[i];
            let axis = archisim_core::Vec2::new(-e.y, e.x);
            let span = |q: &[Point2; 4]| {
                q.iter().fold((f64::MAX, f64::MIN), |(lo, hi), p| {
                    let d = axis.x * p.x + axis.y * p.y;
                    (lo.min(d), hi.max(d))
                })
            };
            let ((a0, a1), (b0, b1)) = (span(a), span(b));
            if a1 < b0 || b1 < a0 {
                return false;
            }
        }
    }
    true
}

fn cross(a: Point2, b: Point2, c: Point2, d: Point2) -> bool {
    let side = |p: Point2, q: Point2, r: Point2| (q - p).cross(r - p);
    let (d1, d2) = (side(a, b, c), side(a, b, d));
    let (d3, d4) = (side(c, d, a), side(c, d, b));
    d1 * d2 < 0.0 && d3 * d4 < 0.0
}

/// Whether going from a run to its label crosses a wall.
fn crosses_walls(a: Point2, b: Point2, walls: &[&[Point2]]) -> bool {
    walls
        .iter()
        .any(|w| (0..w.len()).any(|j| cross(a, b, w[j], w[(j + 1) % w.len()])))
}

/// Whether a label's box falls on a wall: a corner of one inside the
/// other, or their edges crossing.
fn on_walls(b: &[Point2; 4], walls: &[&[Point2]]) -> bool {
    walls.iter().any(|w| {
        b.iter().any(|c| crate::contains(w, *c))
            || w.iter().any(|v| crate::contains(b, *v))
            || (0..4).any(|i| {
                let (p, q) = (b[i], b[(i + 1) % 4]);
                (0..w.len()).any(|j| cross(p, q, w[j], w[(j + 1) % w.len()]))
            })
    })
}
