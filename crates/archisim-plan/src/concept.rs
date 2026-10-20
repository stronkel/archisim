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
    if model.masses_on_level(level).next().is_none() {
        return;
    }
    s.push_str("<g id=\"masses\">\n");
    for m in model.masses_on_level(level) {
        let o = m.outline();
        if o.len() < 3 || !visible(&o) {
            continue;
        }
        let [r, g, b] = model
            .material(m.material)
            .map_or([200, 200, 200], |x| x.cut_color);
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
                    crate::path(&o),
                    num(pens().thin * k),
                    num(2.0 * k),
                    num(1.0 * k)
                );
                return;
            }
            let _ = writeln!(
                s,
                r#"<path d="{}" fill="rgb({},{},{})" fill-opacity="0.6" stroke="{INK}" stroke-width="{}"/>"#,
                crate::path(&o),
                tint(r),
                tint(g),
                tint(b),
                num(pens().cut * k)
            );
            let h = LABEL * k;
            let _ = writeln!(
                s,
                r#"<path d="{}" fill="none" stroke="{INK}" stroke-width="{}" stroke-linecap="round" stroke-linejoin="round"/>"#,
                text_path(
                    &label,
                    centre - Point2::new(0.0, h / 2.0),
                    Point2::new(1.0, 0.0),
                    h,
                    Align::Centre
                ),
                num(crate::lettering::STROKE * h)
            );
        });
    }
    s.push_str("</g>\n");
}
