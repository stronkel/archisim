//! The single-line diagram of each electrical board (esquema unifilar),
//! an A3 landscape page (SVG in millimetres) drawn from the calculation
//! of `archisim-mep`: the supply, the main fuse box and the meter, the
//! feeder with its wires, the main switch, the surge protector, the
//! residual current devices and, under each, its circuits with their
//! breakers; and under the circuits, the table of each one (what it
//! feeds, points, power, current, breaker, wires, tube, length and
//! voltage drop).

use std::fmt::Write as _;

use archisim_core::{Electrification, Model};
use archisim_mep::electrical::{self as el, fmt, ka, section_text, Building, Installation};

use crate::structure_sheets::{
    centred, line, page, rect, text, BLOCK_H, BLOCK_W, GREY, H, INK, MARGIN, MARK, RULE, W,
};

const STROKE: f64 = 0.35;

/// What a row of the circuits' table writes for each circuit.
type Cell = Box<dyn Fn(&el::Circuit) -> String>;

/// A circuit breaker on a vertical line from `top` to `top + 12`: the
/// fixed contact with its cross, the moving one slanted.
fn breaker(s: &mut String, x: f64, top: f64) {
    line(s, (x, top), (x, top + 4.0), STROKE, INK);
    line(s, (x - 1.0, top + 3.0), (x + 1.0, top + 5.0), STROKE, INK);
    line(s, (x + 1.0, top + 3.0), (x - 1.0, top + 5.0), STROKE, INK);
    line(s, (x, top + 12.0), (x - 3.0, top + 5.0), STROKE, INK);
}

/// A residual current device: a breaker with its toroid and «Δ».
fn differential(s: &mut String, x: f64, top: f64) {
    breaker(s, x, top);
    let _ = writeln!(
        s,
        r#"<ellipse cx="{}" cy="{}" rx="3.2" ry="1.3" fill="none" stroke="{INK}" stroke-width="{STROKE}"/>"#,
        x,
        top + 9.5
    );
    line(s, (x + 3.2, top + 9.5), (x + 5.5, top + 9.5), 0.25, INK);
    text(s, "Δ", x + 5.8, top + 10.6, 2.6, INK);
}

/// A fuse: a box through the line.
fn fuse(s: &mut String, x: f64, top: f64) {
    rect(s, x - 1.5, top, 3.0, 7.0, STROKE, INK);
    line(s, (x, top - 1.5), (x, top + 8.5), STROKE, INK);
}

/// An earth: three bars getting shorter.
fn earth(s: &mut String, x: f64, y: f64) {
    for (i, w) in [3.0, 2.0, 1.0].into_iter().enumerate() {
        let yy = y + i as f64 * 1.0;
        line(s, (x - w, yy), (x + w, yy), STROKE, INK);
    }
}

fn frame(s: &mut String, model: &Model, code: &str, title: &str) {
    rect(
        s,
        MARGIN,
        MARGIN,
        W - 2.0 * MARGIN,
        H - 2.0 * MARGIN,
        0.5,
        INK,
    );
    let (x, y) = (W - MARGIN - BLOCK_W, H - MARGIN - BLOCK_H);
    rect(s, x, y, BLOCK_W, BLOCK_H, 0.35, INK);
    line(
        s,
        (x + BLOCK_W - 30.0, y),
        (x + BLOCK_W - 30.0, y + BLOCK_H),
        0.25,
        INK,
    );
    text(s, &model.project_name, x + 4.0, y + 7.0, 3.5, INK);
    text(s, title, x + 4.0, y + 15.0, 3.0, INK);
    text(
        s,
        "Instalación eléctrica · sin escala",
        x + 4.0,
        y + 22.5,
        2.5,
        GREY,
    );
    centred(s, code, x + BLOCK_W - 15.0, y + 17.0, 6.0, INK);
}

/// One page per board; first, with the meters centralised, the page of
/// the general feeder and the feeders.
pub fn pages(model: &Model) -> Vec<String> {
    let _font = crate::lettering::use_font(model.graphics.font);
    let e = el::analysis(model);
    let mut out = Vec::new();
    if let Some(b) = e.building.as_ref().filter(|b| b.general.is_some()) {
        out.push(building_page(model, b, &e.installations, "IE-01"));
    }
    let first = out.len();
    for (i, inst) in e.installations.iter().enumerate() {
        out.push(board_page(model, inst, &format!("IE-{:02}", first + i + 1)));
    }
    out
}

/// The building's page: the main fuse box, the general feeder, the
/// meters and the table of the feeders.
fn building_page(model: &Model, b: &Building, all: &[Installation], code: &str) -> String {
    let mut s = String::new();
    let site = *model.electrical_site();
    frame(
        &mut s,
        model,
        code,
        "Línea general y derivaciones individuales",
    );
    text(
        &mut s,
        "ESQUEMA DE LA LÍNEA GENERAL DE ALIMENTACIÓN",
        MARGIN + 6.0,
        MARGIN + 10.0,
        4.5,
        INK,
    );
    text(
        &mut s,
        &format!(
            "Previsión de cargas (ITC-BT-10): {} W · {}",
            fmt(b.total, 0),
            if b.three_phase {
                "trifásica 3×400/230 V"
            } else {
                "monofásica 230 V"
            }
        ),
        MARGIN + 6.0,
        MARGIN + 16.0,
        2.8,
        GREY,
    );
    let x0 = 40.0;
    text(&mut s, "Red de distribución", x0 + 4.0, 38.0, 2.5, GREY);
    line(&mut s, (x0, 34.0), (x0, 42.0), STROKE, INK);
    fuse(&mut s, x0, 43.5);
    text(
        &mut s,
        &format!("CGP · Icc {}", ka(b.fault_supply)),
        x0 + 4.0,
        48.5,
        2.5,
        INK,
    );
    line(&mut s, (x0, 51.5), (x0, 76.0), STROKE, INK);
    if let Some(g) = &b.general {
        text(
            &mut s,
            &format!(
                "Línea general de alimentación: {} mm² Cu, tubo Ø{} mm",
                g.wires(),
                g.tube
            ),
            x0 + 4.0,
            60.0,
            2.5,
            INK,
        );
        let colour = if g.passes() { GREY } else { MARK };
        text(
            &mut s,
            &format!(
                "{} m · {} A · ΔU {} % (≤ {} %)",
                fmt(g.length, 1),
                fmt(g.current, 1),
                fmt(g.drop * 100.0, 2),
                fmt(g.limit * 100.0, 1)
            ),
            x0 + 4.0,
            64.5,
            2.5,
            colour,
        );
    }
    // The meters: a busbar with a meter for each board.
    let n = all.len().max(1) as f64;
    let right = W - MARGIN - 20.0;
    let step = ((right - x0) / n).min(14.0);
    line(&mut s, (x0, 76.0), (x0 + step * (n - 1.0), 76.0), 0.6, INK);
    text(
        &mut s,
        &format!(
            "Centralización de contadores{} · Icc {}",
            if site.partly_centralised {
                " (en más de un lugar)"
            } else {
                ""
            },
            ka(b.fault_meters)
        ),
        x0 + 4.0,
        72.0,
        2.5,
        INK,
    );
    for i in 0..all.len() {
        let x = x0 + step * i as f64;
        line(&mut s, (x, 76.0), (x, 80.0), STROKE, INK);
        let _ = writeln!(
            s,
            r#"<circle cx="{x}" cy="83" r="3" fill="none" stroke="{INK}" stroke-width="{STROKE}"/>"#
        );
        line(&mut s, (x, 86.0), (x, 92.0), STROKE, INK);
        centred(&mut s, &format!("{}", i + 1), x, 96.0, 2.4, INK);
    }
    if b.three_phase {
        text(
            &mut s,
            &format!(
                "Reparto por fases: L1 {} W · L2 {} W · L3 {} W · desequilibrio {} %",
                fmt(b.phase_loads[0], 0),
                fmt(b.phase_loads[1], 0),
                fmt(b.phase_loads[2], 0),
                fmt(b.unbalance * 100.0, 0)
            ),
            x0,
            103.0,
            2.5,
            GREY,
        );
    }
    // The table of the feeders.
    let top = 110.0;
    let rh = 4.6;
    let cols: [(&str, f64); 10] = [
        ("Nº", 10.0),
        ("Cuadro", 60.0),
        ("Uso", 40.0),
        ("Potencia (W)", 26.0),
        ("Suministro", 26.0),
        ("Longitud (m)", 24.0),
        ("Conductores (mm²)", 34.0),
        ("Tubo", 16.0),
        ("ΔU (%)", 30.0),
        ("Icc cuadro", 24.0),
    ];
    let left = MARGIN + 6.0;
    let width: f64 = cols.iter().map(|c| c.1).sum();
    let rows = all
        .len()
        .min(((H - MARGIN - BLOCK_H - 6.0 - top) / rh) as usize - 1);
    rect(&mut s, left, top, width, rh * (rows + 1) as f64, 0.3, INK);
    let mut x = left;
    for (name, w) in cols {
        text(&mut s, name, x + 1.2, top + 3.3, 2.2, INK);
        line(
            &mut s,
            (x, top),
            (x, top + rh * (rows + 1) as f64),
            0.15,
            RULE,
        );
        x += w;
    }
    for (i, inst) in all.iter().take(rows).enumerate() {
        let y = top + rh * (i + 1) as f64;
        line(&mut s, (left, y), (left + width, y), 0.1, RULE);
        let name = inst
            .board
            .and_then(|d| model.device(d))
            .map_or("Cuadro".to_string(), |d| d.name.clone());
        let level = model
            .level(inst.level)
            .map_or(String::new(), |l| l.name.clone());
        let f = inst.feeder;
        let cells = [
            (i + 1).to_string(),
            format!("{name} · {level}"),
            inst.use_.name().to_string(),
            fmt(inst.power, 0),
            if inst.three_phase {
                "3×400 V".to_string()
            } else {
                "230 V".to_string()
            },
            fmt(f.length, 1),
            f.wires(),
            format!("Ø{}", f.tube),
            format!("{} (≤ {})", fmt(f.drop * 100.0, 2), fmt(f.limit * 100.0, 1)),
            ka(inst.fault),
        ];
        let mut x = left;
        for (k, (c, (_, w))) in cells.iter().zip(cols).enumerate() {
            let red = (k == 8 && !f.passes()) || (k == 9 && !inst.breaks());
            text(
                &mut s,
                c,
                x + 1.2,
                y + 3.3,
                2.1,
                if red { MARK } else { INK },
            );
            x += w;
        }
    }
    let notes = [
        "Líneas de cobre H07Z1-K (AS) bajo tubo. ΔU de la línea general ≤ 0,5 % con los contadores centralizados en un solo lugar y ≤ 1 % si no (ITC-BT-14); de cada derivación individual ≤ 1 % o 0,5 % (ITC-BT-15).".to_string(),
        "Cortocircuito por el método simplificado de la guía técnica: Icc = 0,8·U/R, con la resistencia de los conductores de fase a 20 °C. Previsión de cargas por ITC-BT-10.".to_string(),
    ];
    for (i, t) in notes.iter().enumerate() {
        text(
            &mut s,
            t,
            left,
            top + rh * (rows + 1) as f64 + 6.0 + 4.0 * i as f64,
            2.2,
            GREY,
        );
    }
    page(&s)
}

fn board_page(model: &Model, inst: &Installation, code: &str) -> String {
    let mut s = String::new();
    let name = inst
        .board
        .and_then(|b| model.device(b))
        .map_or("Cuadro general".to_string(), |d| d.name.clone());
    frame(&mut s, model, code, &format!("Esquema unifilar · {name}"));
    let grade = match inst.grade {
        Electrification::High => "elevada",
        _ => "básica",
    };
    text(
        &mut s,
        &format!("ESQUEMA UNIFILAR · {}", name.to_uppercase()),
        MARGIN + 6.0,
        MARGIN + 10.0,
        4.5,
        INK,
    );
    let what = match inst.use_ {
        el::BoardUse::Dwelling => format!("Vivienda de electrificación {grade}"),
        u => {
            let n = u.name();
            let mut c = n.chars();
            c.next().map_or(String::new(), |f| {
                f.to_uppercase().collect::<String>() + c.as_str()
            })
        }
    };
    let curve = model.electrical_site().curve.name();
    text(
        &mut s,
        &format!(
            "{what} · {} W · {} · Icc en el cuadro {} · poder de corte {}",
            fmt(inst.power, 0),
            if inst.three_phase {
                "trifásica 3×400/230 V"
            } else {
                "monofásica 230 V"
            },
            ka(inst.fault),
            ka(inst.breaking_capacity)
        ),
        MARGIN + 6.0,
        MARGIN + 16.0,
        2.8,
        if inst.breaks() { GREY } else { MARK },
    );
    if inst.three_phase {
        text(
            &mut s,
            &format!(
                "Fases: L1 {} W · L2 {} W · L3 {} W · desequilibrio {} %",
                fmt(inst.phase_loads[0], 0),
                fmt(inst.phase_loads[1], 0),
                fmt(inst.phase_loads[2], 0),
                fmt(inst.unbalance * 100.0, 0)
            ),
            MARGIN + 6.0,
            MARGIN + 20.5,
            2.6,
            GREY,
        );
    }

    // The supply down the left: fuse box, meter, feeder, main switch.
    let x0 = 40.0;
    text(&mut s, "Red de distribución", x0 + 4.0, 38.0, 2.5, GREY);
    line(&mut s, (x0, 34.0), (x0, 42.0), STROKE, INK);
    fuse(&mut s, x0, 43.5);
    text(&mut s, "CGP", x0 + 4.0, 48.5, 2.5, INK);
    line(&mut s, (x0, 51.5), (x0, 56.0), STROKE, INK);
    let _ = writeln!(
        s,
        r#"<circle cx="{x0}" cy="61" r="5" fill="none" stroke="{INK}" stroke-width="{STROKE}"/>"#
    );
    centred(&mut s, "kWh", x0, 62.0, 2.4, INK);
    text(&mut s, "Contador", x0 + 7.0, 62.0, 2.5, INK);
    line(&mut s, (x0, 66.0), (x0, 82.0), STROKE, INK);
    let f = inst.feeder;
    text(
        &mut s,
        &format!(
            "Derivación individual: {} mm² Cu H07Z1-K (AS), tubo Ø{} mm",
            f.wires(),
            f.tube
        ),
        x0 + 4.0,
        72.0,
        2.5,
        INK,
    );
    text(
        &mut s,
        &format!(
            "{} m · {} A · ΔU {} % (≤ {} %)",
            fmt(f.length, 1),
            fmt(f.current, 1),
            fmt(f.drop * 100.0, 2),
            fmt(f.limit * 100.0, 1)
        ),
        x0 + 4.0,
        76.5,
        2.5,
        if f.passes() { GREY } else { MARK },
    );
    // The board, dashed round what is in it.
    let n = inst.circuits.len().max(1) as f64;
    let first = 95.0;
    let step = ((W - MARGIN - 20.0 - first) / n).clamp(22.0, 34.0);
    let last = first + step * (n - 1.0);
    let right = (last + step * 0.6).max(160.0);
    let _ = writeln!(
        s,
        r#"<rect x="{}" y="80" width="{}" height="108" fill="none" stroke="{GREY}" stroke-width="0.3" stroke-dasharray="3 1.5"/>"#,
        x0 - 14.0,
        right - x0 + 14.0
    );
    text(&mut s, &name, x0 + 4.0, 84.5, 2.5, GREY);
    breaker(&mut s, x0, 86.0);
    line(&mut s, (x0, 98.0), (x0, 104.0), STROKE, INK);
    text(
        &mut s,
        &format!(
            "IGA {} A · {} · {}",
            inst.main_switch,
            inst.poles(),
            ka(inst.breaking_capacity)
        ),
        x0 + 4.0,
        94.0,
        2.6,
        INK,
    );
    // The busbar.
    line(&mut s, (x0, 104.0), (right - 6.0, 104.0), 0.6, INK);
    // The surge protector, to earth.
    let xs = x0 + 22.0;
    line(&mut s, (xs, 104.0), (xs, 110.0), STROKE, INK);
    rect(&mut s, xs - 2.5, 110.0, 5.0, 7.0, STROKE, INK);
    line(&mut s, (xs - 1.0, 111.5), (xs + 1.0, 115.5), 0.25, INK);
    line(&mut s, (xs, 117.0), (xs, 121.0), STROKE, INK);
    earth(&mut s, xs, 121.5);
    text(&mut s, "Protector de", xs - 9.0, 129.0, 2.2, GREY);
    text(&mut s, "sobretensiones", xs - 9.0, 132.0, 2.2, GREY);
    text(&mut s, "(ITC-BT-23)", xs - 9.0, 135.0, 2.2, GREY);

    // The residual current devices and their circuits.
    let x_of = |k: usize| first + step * k as f64;
    let mut k = 0usize;
    for d in &inst.differentials {
        let from = k;
        let to = k + d.circuits.len().saturating_sub(1);
        let xd = (x_of(from) + x_of(to)) / 2.0;
        line(&mut s, (xd, 104.0), (xd, 112.0), STROKE, INK);
        differential(&mut s, xd, 112.0);
        line(&mut s, (xd, 124.0), (xd, 132.0), STROKE, INK);
        text(
            &mut s,
            &format!("ID {} A · {} mA · {}P", d.rating, d.sensitivity, d.poles),
            xd + 4.0,
            118.0,
            2.4,
            INK,
        );
        line(&mut s, (x_of(from), 132.0), (x_of(to), 132.0), 0.5, INK);
        k = to + 1;
    }
    for (i, c) in inst.circuits.iter().enumerate() {
        let x = x_of(i);
        line(&mut s, (x, 132.0), (x, 142.0), STROKE, INK);
        breaker(&mut s, x, 142.0);
        line(&mut s, (x, 154.0), (x, 176.0), STROKE, INK);
        // The load: an arrow down.
        let _ = writeln!(
            s,
            r#"<path d="M {} 176 L {} 176 L {} 179.5 Z" fill="{INK}"/>"#,
            x - 1.4,
            x + 1.4,
            x
        );
        text(
            &mut s,
            &format!("{} A {curve}", c.breaker),
            x + 2.5,
            149.0,
            2.4,
            INK,
        );
        if c.phase.is_some() {
            text(&mut s, &c.phase_text(), x + 1.5, 138.5, 2.2, GREY);
        }
        text(&mut s, &section_text(c.chosen), x + 1.5, 165.0, 2.2, GREY);
        let colour = if c.drop > el::CIRCUIT_DROP + 1e-9 || !c.protected() || !c.trips() {
            MARK
        } else {
            INK
        };
        centred(&mut s, &c.code(), x, 185.0, 3.2, colour);
    }

    // The table of the circuits.
    let top = 194.0;
    let three = inst.three_phase;
    let rows: [(&str, Cell); 12] = [
        ("Circuito", Box::new(|c| c.code())),
        ("Uso", Box::new(|c| short(c.spec.name))),
        (
            "Fase",
            Box::new(move |c| if three { c.phase_text() } else { "L".into() }),
        ),
        ("Puntos", Box::new(|c| c.points.len().to_string())),
        ("Potencia (W)", Box::new(|c| fmt(c.power, 0))),
        ("Intensidad (A)", Box::new(|c| fmt(c.current, 1))),
        (
            "PIA (A)",
            Box::new(move |c| format!("{} · 2P · {curve}", c.breaker)),
        ),
        (
            "Conductores (mm²)",
            Box::new(|c| {
                let t = section_text(c.chosen);
                format!("2×{t}+TT{}", if c.bigger_section() { " ↑" } else { "" })
            }),
        ),
        ("Tubo (mm)", Box::new(|c| format!("Ø{}", c.tube))),
        ("Longitud (m)", Box::new(|c| fmt(c.length, 1))),
        ("ΔU (%)", Box::new(|c| fmt(c.drop * 100.0, 2))),
        ("Icc final (kA)", Box::new(|c| fmt(c.fault_end, 2))),
    ];
    let rh = 4.4;
    let left = MARGIN + 4.0;
    let label_w = first - step / 2.0 - left;
    let right_edge = last + step / 2.0;
    rect(
        &mut s,
        left,
        top,
        right_edge - left,
        rh * rows.len() as f64,
        0.3,
        INK,
    );
    for (r, (label, value)) in rows.iter().enumerate() {
        let y = top + rh * r as f64;
        if r > 0 {
            line(&mut s, (left, y), (right_edge, y), 0.1, RULE);
        }
        text(&mut s, label, left + 1.5, y + 3.6, 2.3, INK);
        for (i, c) in inst.circuits.iter().enumerate() {
            let x = x_of(i);
            let v = value(c);
            let red = (r == 10 && c.drop > el::CIRCUIT_DROP + 1e-9) || (r == 11 && !c.trips());
            centred(&mut s, &v, x, y + 3.6, 2.2, if red { MARK } else { INK });
        }
    }
    for i in 0..inst.circuits.len() {
        let x = x_of(i) - step / 2.0;
        line(
            &mut s,
            (x, top),
            (x, top + rh * rows.len() as f64),
            0.15,
            RULE,
        );
    }
    let _ = label_w;
    // Notes.
    let notes = [
        "Conductores de cobre H07V-K 450/750 V bajo tubo empotrado (método B1, UNE-HD 60364-5-52). ΔU ≤ 3 % desde el cuadro, sumada tramo a tramo por el recorrido de los cables (ITC-BT-19).".to_string(),
        format!(
            "Un diferencial de 30 mA por cada cinco circuitos (ITC-BT-25). ↑: sección mayor que la de la tabla 1 por la caída de tensión o para que la curva {curve} dispare con el cortocircuito al final de la línea (Icc = 0,8·U/R, conductores a 20 °C)."
        ),
        if inst.use_ != el::BoardUse::Dwelling {
            "Potencia prevista según ITC-BT-10.".to_string()
        } else if inst.reasons.is_empty() {
            "Grado de electrificación según ITC-BT-10.".to_string()
        } else {
            format!("Electrificación elevada: {}.", inst.reasons.join("; "))
        },
    ];
    for (i, t) in notes.iter().enumerate() {
        text(
            &mut s,
            t,
            left,
            top + rh * rows.len() as f64 + 6.0 + 4.0 * i as f64,
            2.2,
            GREY,
        );
    }
    page(&s)
}

/// What a circuit feeds, short enough for its column.
fn short(name: &str) -> String {
    match name {
        "Tomas de uso general y frigorífico" => "Tomas".into(),
        "Lavadora, lavavajillas y termo eléctrico" => "Lavad., lavavaj., termo".into(),
        "Baño y cuarto de cocina" => "Baño y cocina".into(),
        "Iluminación (circuito adicional)" => "Iluminación 2".into(),
        "Tomas de uso general (circuito adicional)" => "Tomas 2".into(),
        "Calefacción eléctrica" => "Calefacción".into(),
        "Aire acondicionado" => "Aire acond.".into(),
        other => other.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use archisim_core::{DeviceKind, NewDevice, Point2};

    #[test]
    fn a_board_draws_its_diagram() {
        let mut m = Model::new("Vivienda");
        let l = m.add_level("Planta baja", 0.0);
        for (kind, x, y) in [
            (DeviceKind::Panel, 0.0, 0.0),
            (DeviceKind::Light, 3.0, 2.0),
            (DeviceKind::Light, 6.0, 2.0),
            (DeviceKind::Socket, 4.0, 0.2),
            (DeviceKind::PowerSocket, 8.0, 3.0),
        ] {
            let (w, d, h, z) = kind.defaults();
            m.add_device(NewDevice {
                level: l,
                kind,
                name: if kind == DeviceKind::Panel {
                    "Cuadro vivienda".into()
                } else {
                    kind.name().into()
                },
                position: Point2::new(x, y),
                rotation: 0.0,
                elevation: z,
                width: w,
                depth: d,
                height: h,
            })
            .unwrap();
        }
        let pages = pages(&m);
        assert_eq!(pages.len(), 1);
        let svg = &pages[0];
        assert!(svg.starts_with("<svg"));
        // Five circuits: five breakers, one differential.
        assert_eq!(svg.matches("<ellipse").count(), 1);
        assert!(svg.len() > 5000);
        // In three phase: the phases over the breakers, four poles.
        let mut site = *m.electrical_site();
        site.supply = archisim_core::Supply::Three;
        m.set_electrical_site(site).unwrap();
        let three = super::pages(&m);
        assert_eq!(three.len(), 1);
        assert_ne!(&three[0], svg);
        // With centralised meters, the general feeder's page first.
        site.single_user = false;
        m.set_electrical_site(site).unwrap();
        let central = super::pages(&m);
        assert_eq!(central.len(), 2);
        if let Ok(dir) = std::env::var("ARCHISIM_UNIFILAR_OUT") {
            std::fs::create_dir_all(&dir).unwrap();
            std::fs::write(format!("{dir}/unifilar.svg"), svg).unwrap();
            std::fs::write(format!("{dir}/unifilar-trifasico.svg"), &three[0]).unwrap();
            std::fs::write(format!("{dir}/unifilar-linea-general.svg"), &central[0]).unwrap();
        }
    }
}
