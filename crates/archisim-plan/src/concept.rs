//! Volumes of the concept phase in plan: each mass on its level as its
//! outline over a light tint of its material, with its name and, for a
//! mass by floors, its floors and built area; once turned into a building,
//! only its outline, dashed.

use std::fmt::Write as _;

use archisim_core::{LevelId, Model, Point2};

use crate::lettering::{text_path, Align};
use crate::{num, pens, INK};

/// Height of the label, paper millimetres.
const LABEL: f64 = 2.5;

/// The label of a mass: «Masa 1 · 3 plantas · 300 m²», «Volumen 2 · 6,00 m».
pub fn mass_label(model: &Model, m: &archisim_core::Mass) -> String {
    let f = m.figures(model.plot_area());
    let comma = |v: f64, d: usize| format!("{v:.d$}").replace('.', ",");
    if m.is_face() {
        format!("{} · cara", m.name)
    } else if m.floor_height.is_some() {
        format!(
            "{} · {} {} · {} m²",
            m.name,
            f.floors,
            if f.floors == 1 { "planta" } else { "plantas" },
            comma(f.built, 0)
        )
    } else {
        format!("{} · {} m", m.name, comma(m.height, 2))
    }
}

/// The masses of a level.
pub fn draw_masses(
    s: &mut String,
    model: &Model,
    level: LevelId,
    k: f64,
    visible: &dyn Fn(&[Point2]) -> bool,
) {
    // Loose edges: lines of the model, with a dot at each end (as SketchUp
    // shows the ends of an edge that closes no face).
    let edges: Vec<_> = model
        .edges_on_level(level)
        .filter(|e| visible(&e.points))
        .collect();
    if !edges.is_empty() {
        s.push_str("<g id=\"edges\">\n");
        for e in edges {
            crate::dim::element(s, e.id.0, |s| {
                let d: Vec<String> = e.points.iter().map(|p| crate::pt(*p)).collect();
                let _ = writeln!(
                    s,
                    r#"<path d="M{}" fill="none" stroke="{INK}" stroke-width="{}" stroke-linecap="round" stroke-linejoin="round"/>"#,
                    d.join(" L"),
                    num(pens().view * k)
                );
                for p in [e.points[0], e.points[e.points.len() - 1]] {
                    let _ = writeln!(
                        s,
                        r#"<circle cx="{}" cy="{}" r="{}" fill="{INK}"/>"#,
                        num(p.x),
                        num(-p.y),
                        num(0.6 * k)
                    );
                }
            });
        }
        s.push_str("</g>\n");
    }
    if model.masses_on_level(level).next().is_none() {
        return;
    }
    s.push_str("<g id=\"masses\">\n");
    for m in model.masses_on_level(level) {
        let shapes = archisim_geom::forms::footprint(m);
        let Some(o) = shapes.first().map(|s| s.0.clone()) else {
            continue;
        };
        if !visible(&o) {
            continue;
        }
        // Every outline and hole of what the solid covers.
        let d: String = shapes
            .iter()
            .flat_map(|(outer, holes)| std::iter::once(outer).chain(holes))
            .map(|c| crate::path(c))
            .collect::<Vec<_>>()
            .join(" ");
        let [r, g, b] = model.surface_color(m.material);
        // A light tint: halfway to white.
        let tint = |c: u8| (c as u16 + 255) / 2;
        let label = mass_label(model, m);
        let centre = o.iter().fold(Point2::new(0.0, 0.0), |a, p| a + *p) * (1.0 / o.len() as f64);
        crate::dim::element(s, m.id.0, |s| {
            if m.building.is_some() {
                // A building now: only its outline, dashed, to pick it.
                let _ = writeln!(
                    s,
                    r#"<path d="{}" fill="none" stroke="{INK}" stroke-width="{}" stroke-dasharray="{} {}"/>"#,
                    d,
                    num(pens().thin * k),
                    num(2.0 * k),
                    num(1.0 * k)
                );
                return;
            }
            if let Some(dress) = &m.dress {
                env_building(s, model, m, dress, &d, centre, k);
                return;
            }
            let _ = writeln!(
                s,
                r#"<path d="{}" fill="rgb({},{},{})" fill-opacity="0.6" fill-rule="evenodd" stroke="{INK}" stroke-width="{}"/>"#,
                d,
                tint(r),
                tint(g),
                tint(b),
                num(pens().cut * k)
            );
            let h = LABEL * k;
            let _ = writeln!(
                s,
                r#"<path d="{}" {}/>"#,
                text_path(
                    &label,
                    centre - Point2::new(0.0, h / 2.0),
                    Point2::new(1.0, 0.0),
                    h,
                    Align::Centre
                ),
                crate::lettering::paint(h, INK)
            );
        });
    }
    s.push_str("</g>\n");
}

/// A building of the surroundings, as town plans draw them: its footprint
/// in grey with its outline, the party walls heavier, its courtyard, and
/// its floors in Roman numerals.
fn env_building(
    s: &mut String,
    model: &Model,
    m: &archisim_core::Mass,
    dress: &archisim_core::Dress,
    d: &str,
    centre: Point2,
    k: f64,
) {
    let _ = writeln!(
        s,
        r#"<path d="{}" fill="{ENV_FILL}" fill-rule="evenodd" stroke="{INK}" stroke-width="{}"/>"#,
        d,
        num(pens().cut * k)
    );
    if let archisim_core::MassShape::Prism(p) = &m.shape {
        let party = model.party_sides(m.id);
        let n = p.len();
        for (i, on) in party.iter().enumerate() {
            if *on {
                let _ = writeln!(
                    s,
                    r#"<path d="M{} L{}" stroke="{INK}" stroke-width="{}"/>"#,
                    crate::pt(p[i]),
                    crate::pt(p[(i + 1) % n]),
                    num(pens().cut * 2.0 * k)
                );
            }
        }
        if dress.patio > 0.0 {
            if let Some(inner) = archisim_core::offset_contour(p, -dress.patio)
                .filter(|q| q.len() == n && archisim_core::signed_area(q) > 4.0)
            {
                let _ = writeln!(
                    s,
                    r#"<path d="{}" fill="white" stroke="{INK}" stroke-width="{}"/>"#,
                    crate::path(&inner),
                    num(pens().view * k)
                );
            }
        }
    }
    let h = LABEL * 1.4 * k;
    let floors = archisim_core::roman(m.floors().max(1) as u32);
    let centre = match &m.shape {
        archisim_core::MassShape::Prism(p) => label_point(p).unwrap_or(centre),
        _ => centre,
    };
    let _ = writeln!(
        s,
        r#"<path d="{}" {}/>"#,
        text_path(
            &floors,
            centre - Point2::new(0.0, h / 2.0),
            Point2::new(1.0, 0.0),
            h,
            Align::Centre
        ),
        crate::lettering::paint(h, INK)
    );
}

/// The grey of the buildings of the surroundings.
const ENV_FILL: &str = "#dedbd5";

/// Where a label sits well inside a contour: of a grid of points over it,
/// the one inside farthest from its sides (the middle of an L's widest
/// arm, not its corner).
fn label_point(poly: &[Point2]) -> Option<Point2> {
    let (mut lo, mut hi) = (poly[0], poly[0]);
    for p in poly {
        lo = Point2::new(lo.x.min(p.x), lo.y.min(p.y));
        hi = Point2::new(hi.x.max(p.x), hi.y.max(p.y));
    }
    let n = poly.len();
    let clearance = |q: Point2| {
        (0..n)
            .map(|i| to_segment(q, poly[i], poly[(i + 1) % n]))
            .fold(f64::MAX, f64::min)
    };
    const STEPS: usize = 16;
    let mut best: Option<(f64, Point2)> = None;
    for i in 0..=STEPS {
        for j in 0..=STEPS {
            let q = Point2::new(
                lo.x + (hi.x - lo.x) * i as f64 / STEPS as f64,
                lo.y + (hi.y - lo.y) * j as f64 / STEPS as f64,
            );
            if !archisim_core::point_in_polygon(q, poly) {
                continue;
            }
            let c = clearance(q);
            if best.is_none_or(|b| c > b.0 + 1e-9) {
                best = Some((c, q));
            }
        }
    }
    best.map(|b| b.1)
}

fn to_segment(q: Point2, a: Point2, b: Point2) -> f64 {
    let ab = b - a;
    let l2 = ab.dot(ab);
    let t = if l2 <= 1e-18 {
        0.0
    } else {
        ((q - a).dot(ab) / l2).clamp(0.0, 1.0)
    };
    q.distance(a + ab * t)
}
