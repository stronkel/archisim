//! The breakdown of the light frames on paper («Despiece de entramados»):
//! the cut list by section and length with the totals, the bars and
//! boards to buy and the fixings, and then every prefabricated wall panel
//! in elevation from outside at 1:50, its members drawn and numbered by
//! kind, its length, height and weight. One page wide as an A3, as long as
//! it needs; millimetres of paper.

use std::fmt::Write as _;

use archisim_core::{Model, Point2};
use archisim_geom::framing::{all_members, cut_list, panel_elevation, wall_panels, Role};

use crate::lettering::{paint, text_path, Align};

const INK: &str = "#1a1a1a";
/// The height of a page (A3 landscape), mm.
const PAGE_H: f64 = 297.0;
const GREY: &str = "#6b6b6b";
const WIDTH: f64 = 420.0;
const MARGIN: f64 = 15.0;
/// The panels' scale (1:50): millimetres of paper per metre.
const PANEL_K: f64 = 20.0;

fn n(v: f64) -> String {
    let v = (v * 100.0).round() / 100.0;
    format!("{v}")
}

fn dec(v: f64, places: usize) -> String {
    format!("{v:.places$}").replace('.', ",")
}

fn text(s: &mut String, t: &str, x: f64, y: f64, h: f64, align: Align, colour: &str) {
    if t.trim().is_empty() {
        return;
    }
    let _ = writeln!(
        s,
        r#"<path d="{}" {}/>"#,
        text_path(t, Point2::new(x, -y), Point2::new(1.0, 0.0), h, align),
        paint(h, colour)
    );
}

fn line(s: &mut String, a: (f64, f64), b: (f64, f64), w: f64, colour: &str) {
    let _ = writeln!(
        s,
        r#"<line x1="{}" y1="{}" x2="{}" y2="{}" stroke="{colour}" stroke-width="{}"/>"#,
        n(a.0),
        n(a.1),
        n(b.0),
        n(b.1),
        n(w)
    );
}

/// The fill of each kind of member in the panels.
fn fill(role: Role) -> &'static str {
    match role {
        Role::BottomTrack | Role::TopTrack | Role::TopPlate => "#9aa7b4",
        Role::KingStud | Role::JackStud => "#d9a441",
        Role::Header | Role::SillTrack => "#c8553d",
        Role::Cripple => "#e6c98a",
        Role::Noggin => "#7fa37a",
        _ => "#cfd6dd",
    }
}

/// Where the next thing `h` high goes: here, or at the top of the next
/// page if it does not fit on this one.
fn room(y: f64, h: f64) -> f64 {
    let page = (y / PAGE_H).floor();
    if y + h > (page + 1.0) * PAGE_H - MARGIN {
        (page + 1.0) * PAGE_H + MARGIN + 8.0
    } else {
        y
    }
}

/// The breakdown as A3 pages (SVG documents, millimetres), `None` if
/// nothing is framed.
pub fn breakdown_pages(model: &Model) -> Option<Vec<String>> {
    let (s, height) = breakdown(model)?;
    let pages = (height / PAGE_H).ceil().max(1.0) as usize;
    // Each page takes the drawing that starts on it.
    let mut parts = vec![String::new(); pages];
    for line in s.lines() {
        let y = first_y(line).unwrap_or(0.0);
        let k = ((y / PAGE_H).floor().max(0.0) as usize).min(pages - 1);
        parts[k].push_str(line);
        parts[k].push('\n');
    }
    Some(
        parts
            .into_iter()
            .enumerate()
            .map(|(k, body)| {
                let top = k as f64 * PAGE_H;
                let mut foot = String::new();
                text(
                    &mut foot,
                    &format!("{} · Despiece de entramados · hoja {}/{}", model.project_name, k + 1, pages),
                    MARGIN,
                    top + PAGE_H - 7.0,
                    2.5,
                    Align::Left,
                    GREY,
                );
                format!(
                    r#"<svg xmlns="http://www.w3.org/2000/svg" width="{w}mm" height="{h}mm" viewBox="0 {top} {w} {h}"><rect y="{top}" width="{w}" height="{h}" fill="white"/>
{body}{foot}</svg>"#,
                    w = n(WIDTH),
                    h = n(PAGE_H),
                    top = n(top),
                )
            })
            .collect(),
    )
}

/// The first y of an element of the drawing (its `y`, `y1` or the first
/// point of its path).
fn first_y(line: &str) -> Option<f64> {
    for key in [" y=\"", " y1=\""] {
        if let Some(i) = line.find(key) {
            let rest = &line[i + key.len()..];
            return rest.split('"').next()?.parse().ok();
        }
    }
    let i = line.find("d=\"M")?;
    let rest = &line[i + 4..];
    let pair = rest.split([' ', 'L', 'M']).next()?;
    pair.split(',').nth(1)?.parse::<f64>().ok()
}

/// The whole breakdown as one long SVG page, `None` if nothing is framed.
pub fn breakdown_svg(model: &Model) -> Option<String> {
    let (s, height) = breakdown(model)?;
    Some(format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="{w}mm" height="{h}mm" viewBox="0 0 {w} {h}"><rect width="{w}" height="{h}" fill="white"/>
{s}</svg>"#,
        w = n(WIDTH),
        h = n(height)
    ))
}

/// The drawing of the breakdown and how long it is.
fn breakdown(model: &Model) -> Option<(String, f64)> {
    let members = all_members(model);
    if members.is_empty() {
        return None;
    }
    let list = cut_list(model, &members);
    let panels = wall_panels(model);
    let mut s = String::new();
    let mut y = MARGIN + 6.0;
    text(
        &mut s,
        "DESPIECE DE ENTRAMADOS",
        MARGIN,
        y,
        5.0,
        Align::Left,
        INK,
    );
    y += 6.0;
    text(
        &mut s,
        &format!(
            "{} · {} piezas · {} paneles de muro",
            model.project_name,
            members.len(),
            panels.len()
        ),
        MARGIN,
        y,
        3.0,
        Align::Left,
        GREY,
    );
    y += 10.0;
    // The cut list.
    let cols = [MARGIN, 80.0, 175.0, 235.0, 265.0, 305.0, 350.0];
    let heads = [
        "Perfil",
        "Pieza",
        "Longitud (mm)",
        "Uds.",
        "Total (m)",
        "Peso (kg)",
        "Volumen (m³)",
    ];
    text(&mut s, "LISTA DE CORTE", MARGIN, y, 3.5, Align::Left, INK);
    y += 6.0;
    for (x, h) in cols.iter().zip(heads) {
        text(&mut s, h, *x, y, 2.5, Align::Left, GREY);
    }
    y += 1.5;
    line(&mut s, (MARGIN, y), (WIDTH - MARGIN, y), 0.25, INK);
    y += 4.5;
    let mut last = String::new();
    for l in &list.lines {
        let was = y;
        y = room(y, 4.0);
        if y != was {
            for (x, h) in cols.iter().zip(heads) {
                text(&mut s, h, *x, y, 2.5, Align::Left, GREY);
            }
            y += 6.0;
            last.clear();
        }
        if l.section != last && !last.is_empty() {
            line(
                &mut s,
                (MARGIN, y - 3.5),
                (WIDTH - MARGIN, y - 3.5),
                0.1,
                GREY,
            );
        }
        let first = l.section != last;
        last = l.section.clone();
        let cells = [
            if first {
                l.section.clone()
            } else {
                String::new()
            },
            l.role.name(l.material).to_string(),
            format!("{}", l.length_mm),
            format!("{}", l.count),
            dec(l.metres(), 2),
            if l.m3() > 0.0 {
                String::new()
            } else {
                dec(l.kg(), 1)
            },
            if l.m3() > 0.0 {
                dec(l.m3(), 3)
            } else {
                String::new()
            },
        ];
        for (x, c) in cols.iter().zip(&cells) {
            text(&mut s, c, *x, y, 2.5, Align::Left, INK);
        }
        y += 4.0;
    }
    line(
        &mut s,
        (MARGIN, y - 2.5),
        (WIDTH - MARGIN, y - 2.5),
        0.25,
        INK,
    );
    y += 6.0;
    // The totals and what to buy.
    y = room(
        y,
        12.0 + 4.5 * (list.totals.len() + list.boards.len() + 1) as f64,
    );
    text(&mut s, "TOTALES Y COMPRA", MARGIN, y, 3.5, Align::Left, INK);
    y += 6.0;
    for t in &list.totals {
        let bar = if t.m3 > 0.0 {
            archisim_geom::framing::BAR_TIMBER
        } else {
            archisim_geom::framing::BAR_STEEL
        };
        let amount = if t.m3 > 0.0 {
            format!("{} m³", dec(t.m3, 3))
        } else {
            format!("{} kg", dec(t.kg, 1))
        };
        text(
            &mut s,
            &format!(
                "{}: {} piezas, {} m, {} · {} barras de {} m",
                t.section,
                t.pieces,
                dec(t.metres, 1),
                amount,
                t.bars,
                dec(bar, 1)
            ),
            MARGIN,
            y,
            2.8,
            Align::Left,
            INK,
        );
        y += 4.5;
    }
    for b in &list.boards {
        text(
            &mut s,
            &format!(
                "{}: {} m² · {} tableros de {} × {} m (con un 10 % de recortes)",
                b.name,
                dec(b.area, 1),
                b.boards,
                dec(b.board.0, 2),
                dec(b.board.1, 2)
            ),
            MARGIN,
            y,
            2.8,
            Align::Left,
            INK,
        );
        y += 4.5;
    }
    let mut fix = Vec::new();
    if list.screws > 0 {
        fix.push(format!("{} tornillos autotaladrantes", list.screws));
    }
    if list.nails > 0 {
        fix.push(format!("{} clavos", list.nails));
    }
    if list.anchors > 0 {
        fix.push(format!("{} anclajes de los muros", list.anchors));
    }
    if !fix.is_empty() {
        text(
            &mut s,
            &format!("Fijaciones (estimación): {}", fix.join(", ")),
            MARGIN,
            y,
            2.8,
            Align::Left,
            INK,
        );
        y += 4.5;
    }
    y += 8.0;
    // The panels.
    if !panels.is_empty() {
        text(
            &mut s,
            "PANELES PREFABRICADOS (alzado desde el exterior, 1:50)",
            MARGIN,
            y,
            3.5,
            Align::Left,
            INK,
        );
        y += 8.0;
    }
    let mut x = MARGIN;
    let mut row_h: f64 = 0.0;
    for p in &panels {
        let (w, h) = (p.length() * PANEL_K, p.height() * PANEL_K);
        if x > MARGIN && x + w > WIDTH - MARGIN {
            x = MARGIN;
            y += row_h + 22.0;
            row_h = 0.0;
        }
        if x == MARGIN {
            y = room(y - 6.0, h + 14.0) + 6.0;
        }
        let bottom = y + h;
        for (role, a, b, width) in panel_elevation(model, p) {
            let (ax, ay) = (x + a[0] * PANEL_K, bottom - a[1] * PANEL_K);
            let (bx, by) = (x + b[0] * PANEL_K, bottom - b[1] * PANEL_K);
            let half = width * PANEL_K / 2.0;
            let (x0, x1, y0, y1) = if (ax - bx).abs() < 1e-6 {
                (ax - half, ax + half, ay.min(by), ay.max(by))
            } else {
                (ax.min(bx), ax.max(bx), ay - half, ay + half)
            };
            let _ = writeln!(
                s,
                r#"<rect x="{}" y="{}" width="{}" height="{}" fill="{}" stroke="{INK}" stroke-width="0.12"/>"#,
                n(x0),
                n(y0),
                n((x1 - x0).max(0.2)),
                n((y1 - y0).max(0.2)),
                fill(role)
            );
        }
        // Its mark, size and weight.
        text(&mut s, &p.mark, x, y - 2.0, 3.5, Align::Left, INK);
        let caption = format!(
            "{} × {} m · {} piezas · {} kg",
            dec(p.length(), 2),
            dec(p.height(), 2),
            p.members.len(),
            dec(p.weight(model), 0)
        );
        text(&mut s, &caption, x, bottom + 5.0, 2.5, Align::Left, GREY);
        line(&mut s, (x, bottom + 1.5), (x + w, bottom + 1.5), 0.15, GREY);
        x += w + 14.0;
        row_h = row_h.max(h);
    }
    if !panels.is_empty() {
        y = room(y + row_h + 20.0, 6.0);
        // The key.
        let mut kx = MARGIN;
        for (role, name) in [
            (Role::Stud, "Montante"),
            (Role::BottomTrack, "Canal / durmiente"),
            (Role::KingStud, "Refuerzo de hueco"),
            (Role::Header, "Dintel / antepecho"),
            (Role::Cripple, "Montante corto"),
            (Role::Noggin, "Travesaño"),
        ] {
            let _ = writeln!(
                s,
                r#"<rect x="{}" y="{}" width="6" height="3" fill="{}" stroke="{INK}" stroke-width="0.12"/>"#,
                n(kx),
                n(y - 2.8),
                fill(role)
            );
            text(&mut s, name, kx + 8.0, y, 2.5, Align::Left, INK);
            kx += 60.0;
        }
        y += 6.0;
    }
    Some((s, y + MARGIN))
}

#[cfg(test)]
mod tests {
    #[test]
    fn the_first_y_of_each_kind_of_line() {
        assert_eq!(
            super::first_y(r#"<line x1="1" y1="20.5" x2="3" y2="4"/>"#),
            Some(20.5)
        );
        assert_eq!(
            super::first_y(r#"<rect x="1" y="7" width="3"/>"#),
            Some(7.0)
        );
        assert_eq!(
            super::first_y(r#"<path d="M12,301.5 L3,4" fill="x"/>"#),
            Some(301.5)
        );
    }
}
