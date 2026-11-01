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
    // The lamps placed as objects are light points too (C1): drawn with
    // the light's symbol and joined to their circuit.
    let lamps: Vec<archisim_core::ServiceDevice> = model
        .lamp_points()
        .into_iter()
        .filter(|(_, l)| *l == level)
        .filter_map(|(o, _)| {
            let ob = model.object(o)?;
            Some(archisim_core::ServiceDevice {
                id: archisim_core::DeviceId(o.0),
                guid: ob.guid.clone(),
                level,
                kind: DeviceKind::Light,
                name: String::new(),
                position: ob.position,
                rotation: 0.0,
                elevation: ob.base_offset,
                width: 0.3,
                depth: 0.3,
                height: 0.1,
                fixture: None,
            })
        })
        .collect();
    let lamp_ids: Vec<archisim_core::DeviceId> = lamps.iter().map(|d| d.id).collect();
    devices.extend(lamps.iter());
    let circuit = |d: &archisim_core::ServiceDevice| {
        if lamp_ids.contains(&d.id) {
            Some(1)
        } else {
            model.device_circuit(d.id)
        }
    };
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
    draw_circuits(s, &devices, &circuit, k);
    for d in &devices {
        let o = d.outline();
        if !visible(&o) {
            continue;
        }
        crate::dim::element(s, d.id.0, |s| draw_device(s, d, k));
        if let Some(n) = circuit(d) {
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
fn draw_circuits(
    s: &mut String,
    devices: &[&archisim_core::ServiceDevice],
    circuit: &dyn Fn(&archisim_core::ServiceDevice) -> Option<u8>,
    k: f64,
) {
    let board = devices
        .iter()
        .find(|d| d.kind == DeviceKind::Panel)
        .map(|d| d.position);
    let mut by: std::collections::BTreeMap<u8, Vec<Point2>> = Default::default();
    for d in devices {
        if let Some(n) = circuit(d) {
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
        kind @ (DeviceKind::Sprinkler
        | DeviceKind::HoseReel
        | DeviceKind::Extinguisher
        | DeviceKind::SmokeDetector
        | DeviceKind::CallPoint
        | DeviceKind::Siren
        | DeviceKind::EmergencyLight
        | DeviceKind::ExitSign
        | DeviceKind::Valve
        | DeviceKind::WaterMeter
        | DeviceKind::Tank
        | DeviceKind::Pump
        | DeviceKind::WaterHeater
        | DeviceKind::SolarPanel
        | DeviceKind::SolarCollector) => {
            fire_water_symbol(s, d, kind, r, ink, k);
        }
        kind @ (DeviceKind::Manhole
        | DeviceKind::Gully
        | DeviceKind::Hydrant
        | DeviceKind::Chamber
        | DeviceKind::Transformer
        | DeviceKind::MainFuseBox
        | DeviceKind::StreetLight) => {
            urban_symbol(s, d, kind, r, ink, k);
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

/// The red of the fire protection's symbols (UNE 23032) and the green of
/// the evacuation's.
const FIRE_INK: &str = "#c8142a";
const EXIT_INK: &str = "#14915a";

/// The symbols of the fire protection, the water's equipment and the
/// solar panels (H3): the fire's in red as UNE 23032 draws them, with a
/// letter where a shape alone would not say what they are; the water's in
/// the colour of their system; the panels as their row of modules.
/// The symbols of the urban networks' devices (U5), at their real size
/// (but never smaller than a symbol).
fn urban_symbol(
    s: &mut String,
    d: &archisim_core::ServiceDevice,
    kind: DeviceKind,
    r: f64,
    ink: &str,
    k: f64,
) {
    let c = d.position;
    let w = num(pens().view * k);
    let system = hex(kind.system().map_or([90, 90, 90], |s| s.color()));
    let poly = |s: &mut String, pts: &[Point2], fill: &str, stroke: &str| {
        let _ = writeln!(
            s,
            r#"<path d="{}" fill="{fill}" stroke="{stroke}" stroke-width="{w}" stroke-linejoin="round"/>"#,
            path(pts)
        );
    };
    let disc = |radius: f64| -> Vec<Point2> {
        (0..32)
            .map(|i| {
                let a = std::f64::consts::TAU * i as f64 / 32.0;
                c + Point2::new(radius * a.cos(), radius * a.sin())
            })
            .collect()
    };
    let letter = |s: &mut String, text: &str, h: f64, colour: &str| {
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
            crate::lettering::paint(h, colour)
        );
    };
    let line = |s: &mut String, a: Point2, b: Point2| {
        let _ = writeln!(
            s,
            r#"<path d="M{} L{}" fill="none" stroke="{system}" stroke-width="{w}"/>"#,
            pt(a),
            pt(b)
        );
    };
    let size = (d.width.max(d.depth) / 2.0).max(r);
    match kind {
        DeviceKind::Manhole => {
            // Its cover: a circle with a cross.
            poly(s, &disc(size), "white", ink);
            poly(s, &disc(size * 0.55), "white", &system);
            line(
                s,
                c + Point2::new(-size * 0.55, 0.0),
                c + Point2::new(size * 0.55, 0.0),
            );
            line(
                s,
                c + Point2::new(0.0, -size * 0.55),
                c + Point2::new(0.0, size * 0.55),
            );
        }
        DeviceKind::Gully => {
            let o = d.outline();
            poly(s, &o, "white", ink);
            // Its grating's slots.
            let (a, b) = (o[0], o[1]);
            let (dd, e) = (o[3], o[2]);
            for i in 1..5 {
                let t = i as f64 / 5.0;
                line(s, a + (b - a) * t, dd + (e - dd) * t);
            }
        }
        DeviceKind::Hydrant => {
            poly(s, &disc(size), "#ffe2e2", "#c81e1e");
            letter(s, "H", size * 1.1, "#c81e1e");
        }
        DeviceKind::Chamber => {
            let o = d.outline();
            poly(s, &o, "white", ink);
            line(s, o[0], o[2]);
            line(s, o[1], o[3]);
        }
        DeviceKind::Transformer => {
            poly(s, &d.outline(), "#fff1e6", ink);
            letter(s, "CT", size * 0.5, &system);
        }
        DeviceKind::MainFuseBox => {
            poly(s, &d.outline(), "white", ink);
            letter(s, "CGP", size * 0.5, &system);
        }
        DeviceKind::StreetLight => {
            poly(s, &disc(size.max(0.25)), "#fff7d6", ink);
            for i in 0..8 {
                let a = std::f64::consts::TAU * i as f64 / 8.0;
                let u = Point2::new(a.cos(), a.sin());
                line(s, c + u * (size * 1.3), c + u * (size * 2.0));
            }
        }
        _ => {}
    }
}

fn fire_water_symbol(
    s: &mut String,
    d: &archisim_core::ServiceDevice,
    kind: DeviceKind,
    r: f64,
    ink: &str,
    k: f64,
) {
    let c = d.position;
    let (cs, sn) = (d.rotation.cos(), d.rotation.sin());
    let rot = |x: f64, y: f64| c + Point2::new(x * cs - y * sn, x * sn + y * cs);
    let w = num(pens().view * k);
    let poly = |s: &mut String, pts: &[Point2], fill: &str, stroke: &str| {
        let _ = writeln!(
            s,
            r#"<path d="{}" fill="{fill}" stroke="{stroke}" stroke-width="{w}" stroke-linejoin="round"/>"#,
            path(pts)
        );
    };
    let square = |half: f64| -> Vec<Point2> {
        [(-1.0, -1.0), (1.0, -1.0), (1.0, 1.0), (-1.0, 1.0)]
            .into_iter()
            .map(|(x, y)| rot(x * half, y * half))
            .collect()
    };
    let disc = |radius: f64| -> Vec<Point2> {
        (0..24)
            .map(|i| {
                let a = std::f64::consts::TAU * i as f64 / 24.0;
                c + Point2::new(radius * a.cos(), radius * a.sin())
            })
            .collect()
    };
    let letter = |s: &mut String, text: &str, h: f64, colour: &str, at: Point2| {
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
            crate::lettering::paint(h, colour)
        );
    };
    let line = |s: &mut String, a: Point2, b: Point2, colour: &str| {
        let _ = writeln!(
            s,
            r#"<path d="M{} L{}" fill="none" stroke="{colour}" stroke-width="{w}"/>"#,
            pt(a),
            pt(b)
        );
    };
    let system = hex(kind.system().map_or([90, 90, 90], |s| s.color()));
    match kind {
        DeviceKind::Sprinkler => {
            // Its head, with its spray.
            poly(s, &disc(r * 0.45), FIRE_INK, FIRE_INK);
            for i in 0..4 {
                let a = std::f64::consts::FRAC_PI_4 + std::f64::consts::FRAC_PI_2 * i as f64;
                let u = Point2::new(a.cos(), a.sin());
                line(s, c + u * (r * 0.6), c + u * r, FIRE_INK);
            }
        }
        DeviceKind::HoseReel => {
            // Its cabinet, with the reel.
            poly(s, &square(r), "white", FIRE_INK);
            poly(s, &disc(r * 0.6), "none", FIRE_INK);
            poly(s, &disc(r * 0.18), FIRE_INK, FIRE_INK);
        }
        DeviceKind::Extinguisher => {
            poly(s, &square(r * 0.8), FIRE_INK, FIRE_INK);
            letter(s, "E", r, "white", c);
        }
        DeviceKind::SmokeDetector => {
            poly(s, &disc(r * 0.8), "white", FIRE_INK);
            letter(s, "D", r * 0.9, FIRE_INK, c);
        }
        DeviceKind::CallPoint => {
            poly(s, &square(r * 0.7), "white", FIRE_INK);
            poly(s, &disc(r * 0.28), FIRE_INK, FIRE_INK);
        }
        DeviceKind::Siren => {
            // A horn opening out from the wall.
            let horn = [
                rot(-r * 0.3, 0.0),
                rot(r * 0.3, 0.0),
                rot(r * 0.8, r * 0.9),
                rot(-r * 0.8, r * 0.9),
            ];
            poly(s, &horn, FIRE_INK, FIRE_INK);
        }
        DeviceKind::EmergencyLight => {
            let o = [rot(-r, 0.0), rot(r, 0.0), rot(r, r * 0.7), rot(-r, r * 0.7)];
            poly(s, &o, "white", ink);
            let half = [
                rot(0.0, 0.0),
                rot(r, 0.0),
                rot(r, r * 0.7),
                rot(0.0, r * 0.7),
            ];
            poly(s, &half, ink, ink);
        }
        DeviceKind::ExitSign => {
            let o = [
                rot(-r * 1.2, 0.0),
                rot(r * 1.2, 0.0),
                rot(r * 1.2, r * 0.8),
                rot(-r * 1.2, r * 0.8),
            ];
            poly(s, &o, EXIT_INK, EXIT_INK);
            // Its arrow.
            line(s, rot(-r * 0.7, r * 0.4), rot(r * 0.7, r * 0.4), "white");
            line(s, rot(r * 0.7, r * 0.4), rot(r * 0.35, r * 0.65), "white");
            line(s, rot(r * 0.7, r * 0.4), rot(r * 0.35, r * 0.15), "white");
        }
        DeviceKind::Valve => {
            // Two triangles tip to tip, across the pipe; its stem.
            let a = r * 0.8;
            poly(
                s,
                &[rot(-a, -a * 0.6), rot(0.0, 0.0), rot(-a, a * 0.6)],
                &system,
                &system,
            );
            poly(
                s,
                &[rot(a, -a * 0.6), rot(0.0, 0.0), rot(a, a * 0.6)],
                &system,
                &system,
            );
            line(s, rot(0.0, 0.0), rot(0.0, a * 0.9), &system);
            line(s, rot(-a * 0.4, a * 0.9), rot(a * 0.4, a * 0.9), &system);
        }
        DeviceKind::WaterMeter => {
            poly(s, &d.outline(), "white", ink);
            poly(s, &disc(r * 0.6), "white", &system);
            letter(s, "C", r * 0.7, &system, c);
        }
        DeviceKind::Tank => {
            let rr = (d.width.min(d.depth) / 2.0).max(r);
            poly(s, &disc(rr), "white", &system);
            let q = rr * 0.55;
            line(
                s,
                c + Point2::new(-q, -q * 0.3),
                c + Point2::new(q, -q * 0.3),
                &system,
            );
            line(
                s,
                c + Point2::new(-q * 0.7, -q * 0.6),
                c + Point2::new(q * 0.7, -q * 0.6),
                &system,
            );
            letter(s, &d.name, r * 0.8, ink, c + Point2::new(0.0, rr * 0.3));
        }
        DeviceKind::Pump => {
            poly(s, &d.outline(), "white", ink);
            let rr = r.min(d.width.min(d.depth) / 2.0).max(r * 0.6);
            poly(s, &disc(rr), "white", &system);
            poly(
                s,
                &[
                    rot(-rr * 0.5, -rr * 0.6),
                    rot(rr * 0.75, 0.0),
                    rot(-rr * 0.5, rr * 0.6),
                ],
                &system,
                &system,
            );
        }
        DeviceKind::WaterHeater => {
            let rr = (d.width.min(d.depth) / 2.0).max(r);
            poly(s, &disc(rr), "white", &system);
            letter(s, "ACS", rr * 0.55, &system, c);
        }
        DeviceKind::SolarPanel | DeviceKind::SolarCollector => {
            // Its modules along the row, with their cells or tubes.
            let (hx, hy) = (d.width / 2.0, d.depth / 2.0);
            let n = archisim_core::solar_modules(d);
            let step = d.width / n as f64;
            let colour = if kind == DeviceKind::SolarPanel {
                "#1d3a6b"
            } else {
                &system
            };
            for i in 0..n {
                let x0 = -hx + step * i as f64 + 0.01;
                let x1 = x0 + step - 0.02;
                let m = [rot(x0, -hy), rot(x1, -hy), rot(x1, hy), rot(x0, hy)];
                poly(s, &m, "white", colour);
                let lines = if kind == DeviceKind::SolarPanel { 5 } else { 8 };
                for j in 1..lines {
                    if kind == DeviceKind::SolarPanel {
                        let y = -hy + d.depth * j as f64 / lines as f64;
                        line(s, rot(x0, y), rot(x1, y), colour);
                    } else {
                        let x = x0 + (x1 - x0) * j as f64 / lines as f64;
                        line(s, rot(x, -hy), rot(x, hy), colour);
                    }
                }
            }
        }
        _ => {}
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
