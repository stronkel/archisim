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

/// «2», «1,5».
fn percent(slope: f64) -> String {
    let v = (slope * 1000.0).round() / 10.0;
    if (v - v.round()).abs() < 1e-9 {
        format!("{v:.0}")
    } else {
        format!("{v:.1}").replace('.', ",")
    }
}

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
    let mut devices: Vec<_> = model.devices_on_level(level).collect();
    // Risers and downpipes of the levels below that pass through this one.
    let elevation = model.level(level).map_or(0.0, |l| l.elevation);
    devices.extend(model.devices().filter(|d| {
        d.level != level
            && d.kind.is_vertical()
            && model.level(d.level).is_some_and(|l| {
                let bottom = l.elevation + d.elevation;
                bottom < elevation + 1.0 && bottom + d.height > elevation + 1.0
            })
    }));
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
                    let text = if r.slope > 0.0 {
                        format!(
                            "{} {} · {} %",
                            r.system.name(),
                            r.size_text(),
                            percent(r.slope)
                        )
                    } else {
                        format!("{} {}", r.system.name(), r.size_text())
                    };
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
    // Arrows the way a sloping run falls, in the middle of each stretch.
    for r in runs.iter().filter(|r| r.slope > 0.0) {
        let colour = hex(r.system.color());
        for w in r.path.windows(2) {
            let Some(u) = (w[1] - w[0]).normalized() else {
                continue;
            };
            if w[0].distance(w[1]) < 6.0 * k {
                continue;
            }
            let m = w[0] + (w[1] - w[0]) * 0.5;
            let n = u.perp_left() * (1.0 * k);
            let tip = m + u * (1.2 * k);
            let back = m - u * (1.2 * k);
            let _ = writeln!(
                s,
                r#"<path d="M{} L{} L{} Z" fill="{colour}" stroke="none"/>"#,
                pt(tip),
                pt(back + n),
                pt(back - n)
            );
        }
    }
    // Tees of pipes: a dot where a branch joins.
    for f in model.run_fittings() {
        if f.kind != archisim_core::FittingKind::Tee {
            continue;
        }
        let Some(r) = runs.iter().find(|r| r.id == f.run) else {
            continue;
        };
        if r.is_duct() || !visible(&[f.position]) {
            continue;
        }
        let _ = writeln!(
            s,
            r#"<circle cx="{}" cy="{}" r="{}" fill="{}" stroke="none"/>"#,
            num(f.position.x),
            num(-f.position.y),
            num((r.width / 2.0 * 1.4).max(0.7 * k)),
            hex(r.system.color())
        );
    }
    // The circuits: each mechanism's number, and a line joining those of
    // a circuit, nearest first, from the board if there is one (E13).
    draw_circuits(s, model, &devices, k);
    for d in &devices {
        let o = d.outline();
        if !visible(&o) {
            continue;
        }
        crate::dim::element(s, d.id.0, |s| draw_device(s, d, k));
        if let Some(n) = model.device_circuit(d.id) {
            let h = 1.6 * k;
            let at = d.position + Point2::new((2.0 * k).max(d.width / 2.0) + 0.6 * k, 0.4 * k);
            let _ = writeln!(
                s,
                r#"<path d="{}" {}/>"#,
                text_path(&format!("C{n}"), at, Point2::new(1.0, 0.0), h, Align::Left),
                crate::lettering::paint(h, CIRCUIT_INK)
            );
        }
    }
    s.push_str("</g>\n");
}

/// The colour of the circuits in plan.
const CIRCUIT_INK: &str = "#b0302a";

/// A line through the mechanisms of each circuit of a level: from the
/// board (or the first), always to the nearest not yet joined, drawn as a
/// curve as an electrician sketches it.
fn draw_circuits(s: &mut String, model: &Model, devices: &[&archisim_core::ServiceDevice], k: f64) {
    let board = devices
        .iter()
        .find(|d| d.kind == DeviceKind::Panel)
        .map(|d| d.position);
    let mut by: std::collections::BTreeMap<u8, Vec<Point2>> = Default::default();
    for d in devices {
        if let Some(n) = model.device_circuit(d.id) {
            by.entry(n).or_default().push(d.position);
        }
    }
    for points in by.values() {
        let mut left = points.clone();
        let mut at = match board {
            Some(b) => b,
            None => left.remove(0),
        };
        let mut d = String::new();
        let _ = write!(d, "M{}", pt(at));
        while !left.is_empty() {
            let (i, _) = left
                .iter()
                .enumerate()
                .min_by(|a, b| at.distance(*a.1).total_cmp(&at.distance(*b.1)))
                .expect("not empty");
            let next = left.remove(i);
            // A gentle bow to the left of the way.
            let mid = at + (next - at) * 0.5;
            let bow = (next - at).perp_left() * 0.08;
            let _ = write!(d, " Q{} {}", pt(mid + bow), pt(next));
            at = next;
        }
        let _ = writeln!(
            s,
            r#"<path d="{d}" fill="none" stroke="{CIRCUIT_INK}" stroke-width="{}" stroke-dasharray="{} {}"/>"#,
            num(pens().thin * k),
            num(2.0 * k),
            num(1.0 * k)
        );
    }
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
            line(s, rot(r * 1.2, r * 1.2), rot(r * 1.45, r * 0.95));
        }
        DeviceKind::TwoWaySwitch => {
            // Its arm both ways, each with its tick.
            circle(s, c, r * 0.6, "white");
            line(s, rot(r * 0.4, r * 0.4), rot(r * 1.2, r * 1.2));
            line(s, rot(r * 1.2, r * 1.2), rot(r * 1.45, r * 0.95));
            line(s, rot(-r * 0.4, -r * 0.4), rot(-r * 1.2, -r * 1.2));
            line(s, rot(-r * 1.2, -r * 1.2), rot(-r * 1.45, -r * 0.95));
        }
        DeviceKind::PowerSocket => {
            // The socket, filled: a power base.
            let arc: Vec<Point2> = (0..=12)
                .map(|i| {
                    let a = std::f64::consts::PI * i as f64 / 12.0;
                    rot(r * a.cos(), r * a.sin())
                })
                .collect();
            let _ = writeln!(
                s,
                r#"<path d="{} Z" fill="{ink}" stroke="{ink}" stroke-width="{w}"/>"#,
                path(&arc).trim_end_matches('Z').trim_end()
            );
            line(s, rot(-r * 1.2, 0.0), rot(r * 1.2, 0.0));
        }
        DeviceKind::TvSocket | DeviceKind::DataSocket => {
            let text = if d.kind == DeviceKind::TvSocket {
                "TV"
            } else {
                "TD"
            };
            circle(s, c, r * 0.8, "white");
            let h = r * 0.7;
            let _ = writeln!(
                s,
                r#"<path d="{}" {}/>"#,
                text_path(
                    text,
                    c - Point2::new(0.0, h / 2.0),
                    Point2::new(1.0, 0.0),
                    h,
                    Align::Centre
                ),
                crate::lettering::paint(h, ink)
            );
        }
        DeviceKind::Panel => {
            // The board: its box, half filled.
            let o = d.outline();
            let _ = writeln!(
                s,
                r#"<path d="{}" fill="white" stroke="{ink}" stroke-width="{}"/>"#,
                path(&o),
                num(pens().cut * k)
            );
            let half = vec![o[0], o[1], o[2]];
            let _ = writeln!(
                s,
                r#"<path d="{}" fill="{ink}" stroke="none"/>"#,
                path(&half)
            );
        }
        DeviceKind::Riser | DeviceKind::Downpipe => {
            // The pipe, cut, in the colour of its system, with its name
            // and size.
            let colour = hex(d.kind.system().map_or([90, 90, 90], |s| s.color()));
            let rr = (d.width / 2.0).max(1.0 * k);
            let _ = writeln!(
                s,
                r#"<circle cx="{}" cy="{}" r="{}" fill="white" stroke="{colour}" stroke-width="{}"/>"#,
                num(c.x),
                num(-c.y),
                num(rr),
                num(pens().cut * k)
            );
            let q = rr * std::f64::consts::FRAC_1_SQRT_2;
            let _ = writeln!(
                s,
                r#"<path d="M{} L{} M{} L{}" stroke="{colour}" stroke-width="{w}"/>"#,
                pt(c + Point2::new(-q, -q)),
                pt(c + Point2::new(q, q)),
                pt(c + Point2::new(-q, q)),
                pt(c + Point2::new(q, -q))
            );
            let h = 1.8 * k;
            let text = format!(
                "{} Ø{}",
                if d.kind == DeviceKind::Riser {
                    "M"
                } else {
                    "B"
                },
                (d.width * 1000.0).round() as i64
            );
            let _ = writeln!(
                s,
                r#"<path d="{}" {}/>"#,
                text_path(
                    &text,
                    c + Point2::new(rr + 0.8 * k, -rr - h),
                    Point2::new(1.0, 0.0),
                    h,
                    Align::Left
                ),
                crate::lettering::paint(h, &colour)
            );
        }
        DeviceKind::Radiator | DeviceKind::Grille | DeviceKind::Diffuser => {
            let o = d.outline();
            let _ = writeln!(
                s,
                r#"<path d="{}" fill="white" stroke="{ink}" stroke-width="{}"/>"#,
                path(&o),
                num(pens().cut * k)
            );
            let (hx, hy) = (d.width / 2.0, d.depth / 2.0);
            match d.kind {
                // Its elements, across.
                DeviceKind::Radiator => {
                    let n = ((d.width / 0.1).round() as usize).clamp(3, 20);
                    for i in 1..n {
                        let x = -hx + d.width * i as f64 / n as f64;
                        line(s, rot(x, -hy), rot(x, hy));
                    }
                }
                // Its slats, along.
                DeviceKind::Grille => {
                    for i in 1..4 {
                        let y = -hy + d.depth * i as f64 / 4.0;
                        line(s, rot(-hx, y), rot(hx, y));
                    }
                }
                // Its cross and its inner square.
                _ => {
                    line(s, o[0], o[2]);
                    line(s, o[1], o[3]);
                    let q: Vec<Point2> = [(-0.5, -0.5), (0.5, -0.5), (0.5, 0.5), (-0.5, 0.5)]
                        .into_iter()
                        .map(|(x, y)| rot(x * hx, y * hy))
                        .collect();
                    let _ = writeln!(
                        s,
                        r#"<path d="{}" fill="none" stroke="{ink}" stroke-width="{w}"/>"#,
                        path(&q)
                    );
                }
            }
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
