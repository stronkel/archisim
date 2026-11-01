//! The longitudinal profile of a run of an urban network (U5): the street
//! above it and its pipe, with the manholes, at 1:500 along and ten times
//! that across (1:50 in height), and the guitar of numbers under it:
//! distances, street and pipe heights, depth and slope of each stretch.

use std::fmt::Write;

use archisim_core::{Model, RunId};

/// The run's profile as an SVG drawing (millimetres of paper).
pub fn run_profile_svg(model: &Model, run: RunId) -> Option<String> {
    let r = model.run(run)?;
    let st = model.run_profile(run);
    if st.len() < 2 {
        return None;
    }
    let base = model.level(r.level).map_or(0.0, |l| l.elevation);
    let len = st[st.len() - 1].at;
    // Scales: 1:500 along (2 mm a metre), 1:50 in height (20 mm a metre).
    let (kx, ky) = (2.0, 20.0);
    let lo = st.iter().map(|s| s.invert).fold(f64::MAX, f64::min).floor() - 1.0;
    let hi = st.iter().map(|s| s.ground).fold(f64::MIN, f64::max).ceil() + 1.0;
    let left = 40.0;
    let plot_h = (hi - lo) * ky;
    let rows = [
        "Distancia (m)",
        "Cota de calle",
        "Cota de solera",
        "Profundidad",
        "Pendiente",
    ];
    let row_h = 9.0;
    let width = left + len * kx + 20.0;
    let height = 20.0 + plot_h + rows.len() as f64 * row_h + 20.0;
    let x = |at: f64| left + at * kx;
    let y = |z: f64| 15.0 + (hi - z) * ky;
    let mut s = String::new();
    let _ = writeln!(
        s,
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="{w}mm" height="{h}mm" viewBox="0 0 {w} {h}" font-family="IBM Plex Sans, sans-serif">"#,
        w = width,
        h = height
    );
    let _ = writeln!(
        s,
        r#"<rect width="{width}" height="{height}" fill="white"/>"#
    );
    let _ = writeln!(
        s,
        r#"<text x="{left}" y="8" font-size="4">Perfil longitudinal · {} {} · H 1:500 · V 1:50</text>"#,
        r.system.name(),
        r.size_text()
    );
    // Height lines every metre.
    let mut z = lo;
    while z <= hi + 1e-9 {
        let _ = writeln!(
            s,
            r##"<path d="M{} {} H{}" stroke="#d8d8d8" stroke-width="0.15"/><text x="{}" y="{}" font-size="2.6" text-anchor="end">{}</text>"##,
            left,
            y(z),
            x(len),
            left - 2.0,
            y(z) + 1.0,
            format!("{:.2}", z + base).replace('.', ",")
        );
        z += 1.0;
    }
    // The street.
    let ground: Vec<String> = st
        .iter()
        .map(|p| format!("{:.2},{:.2}", x(p.at), y(p.ground)))
        .collect();
    let _ = writeln!(
        s,
        r##"<polyline points="{}" fill="none" stroke="#202020" stroke-width="0.5"/>"##,
        ground.join(" ")
    );
    // The pipe: its bottom and its top.
    let [cr, cg, cb] = r.system.color();
    let colour = format!("#{cr:02x}{cg:02x}{cb:02x}");
    for off in [0.0, r.width] {
        let pts: Vec<String> = st
            .iter()
            .map(|p| format!("{:.2},{:.2}", x(p.at), y(p.invert + off)))
            .collect();
        let _ = writeln!(
            s,
            r#"<polyline points="{}" fill="none" stroke="{colour}" stroke-width="0.4"/>"#,
            pts.join(" ")
        );
    }
    // The manholes, from the pipe's bottom up to the street.
    for p in st.iter().filter(|p| p.manhole) {
        let w = 1.2 * kx;
        let _ = writeln!(
            s,
            r##"<rect x="{:.2}" y="{:.2}" width="{:.2}" height="{:.2}" fill="none" stroke="#202020" stroke-width="0.3"/>"##,
            x(p.at) - w / 2.0,
            y(p.ground),
            w,
            (p.ground - (p.invert - 0.2)) * ky
        );
    }
    // The guitar.
    let top = 20.0 + plot_h;
    for (i, name) in rows.iter().enumerate() {
        let yy = top + i as f64 * row_h;
        let _ = writeln!(
            s,
            r##"<path d="M2 {yy} H{}" stroke="#808080" stroke-width="0.2"/><text x="3" y="{}" font-size="2.6">{name}</text>"##,
            x(len),
            yy + row_h * 0.65
        );
    }
    let _ = writeln!(
        s,
        r##"<path d="M2 {} H{}" stroke="#808080" stroke-width="0.2"/>"##,
        top + rows.len() as f64 * row_h,
        x(len)
    );
    // Numbers at the manholes and the ends; the slope between them.
    let marks: Vec<&archisim_core::ProfileStation> = st
        .iter()
        .enumerate()
        .filter(|(i, p)| p.manhole || *i == 0 || *i == st.len() - 1)
        .map(|(_, p)| p)
        .collect();
    let vertical = |s: &mut String, at: f64, row: usize, text: String| {
        let yy = top + row as f64 * row_h + row_h - 1.5;
        let _ = writeln!(
            s,
            r#"<text x="{:.2}" y="{:.2}" font-size="2.4" transform="rotate(-90 {:.2} {:.2})">{text}</text>"#,
            x(at) + 0.8,
            yy,
            x(at) + 0.8,
            yy
        );
    };
    let comma = |v: f64, d: usize| format!("{v:.d$}").replace('.', ",");
    for p in &marks {
        let _ = writeln!(
            s,
            r##"<path d="M{:.2} {:.2} V{:.2}" stroke="#b0b0b0" stroke-width="0.15" stroke-dasharray="1 1"/>"##,
            x(p.at),
            y(p.ground),
            top + 4.0 * row_h
        );
        vertical(&mut s, p.at, 0, comma(p.at, 2));
        vertical(&mut s, p.at, 1, comma(p.ground + base, 2));
        vertical(&mut s, p.at, 2, comma(p.invert + base, 2));
        vertical(&mut s, p.at, 3, comma(p.ground - p.invert, 2));
    }
    for w in marks.windows(2) {
        let (a, b) = (w[0], w[1]);
        let l = b.at - a.at;
        if l < 1e-6 {
            continue;
        }
        let slope = (a.invert - b.invert) / l * 100.0;
        let yy = top + 4.0 * row_h + row_h * 0.65;
        let _ = writeln!(
            s,
            r#"<text x="{:.2}" y="{yy:.2}" font-size="2.4" text-anchor="middle">{} % · {} m</text>"#,
            (x(a.at) + x(b.at)) / 2.0,
            comma(slope, 2),
            comma(l, 2)
        );
    }
    s.push_str("</svg>\n");
    Some(s)
}
