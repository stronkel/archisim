//! The carpentry sheets (plano de carpinterías, concepto de interfaz del
//! 24/10/2026): every door and window type of the project on A3 paper, in
//! elevation at 1:50 with its overall measurements, its vertical section,
//! its mark and a few lines of data — what it is, how it opens, its
//! frame and how many there are. Doors first, then windows, by mark; six
//! to a sheet.
//!
//! The elevations and sections are the same drawings as the types
//! dialog's (`type_drawing`, from the recipes), so nothing is drawn twice.
//! The sheets are SVG in millimetres of paper with ArchiSim's lettering,
//! so they print at their scale and turn into PDF like the plans.

use std::fmt::Write as _;

use archisim_core::{Model, OpeningType, Point2};
use archisim_plan::lettering::{self, Align};

use crate::paper::{self, metres, text, INK, MARGIN, PAPER};

use crate::type_drawing::{opening_elevation, opening_section, Drawing, Item, Pen};

/// Types on a sheet: columns and rows.
const COLS: usize = 3;
const ROWS: usize = 2;
/// Paper millimetres per metre at 1:50.
const MM_PER_M: f64 = 20.0;

/// One type on the sheets, with where it is used.
pub(crate) struct Entry<'a> {
    pub t: &'a OpeningType,
    pub count: usize,
    pub family: String,
    /// Units by level, in the order of the levels.
    pub levels: Vec<(String, usize)>,
    /// The sills it is placed at, from the lowest, without repeats.
    pub sills: Vec<f64>,
    /// Where it is: "Cocina", "Cocina / Salón" for a door between two
    /// rooms, and how many there.
    pub rooms: Vec<(String, usize)>,
}

/// The door and window types of the project, doors first, then windows,
/// by mark: the ones it uses, or all of them if it uses none.
pub(crate) fn entries(model: &Model) -> Vec<Entry<'_>> {
    let mut entries: Vec<Entry> = model
        .opening_types()
        .filter(|t| !t.is_void())
        .map(|t| entry(model, t))
        .collect();
    if entries.iter().any(|e| e.count > 0) {
        entries.retain(|e| e.count > 0);
    }
    entries.sort_by(|a, b| {
        (a.t.is_window(), natural(&a.t.mark), &a.t.name).cmp(&(
            b.t.is_window(),
            natural(&b.t.mark),
            &b.t.name,
        ))
    });
    entries
}

fn entry<'a>(model: &'a Model, t: &'a OpeningType) -> Entry<'a> {
    let mut by_level: Vec<(f64, String, usize)> = Vec::new();
    let mut sills: Vec<f64> = Vec::new();
    let mut rooms: Vec<(String, usize)> = Vec::new();
    let mut count = 0;
    for o in model.openings().filter(|o| o.opening_type == t.id) {
        count += 1;
        let Some(w) = model.wall(o.wall) else {
            continue;
        };
        if let Some(l) = model.level(w.level) {
            match by_level.iter_mut().find(|(_, n, _)| *n == l.name) {
                Some(e) => e.2 += 1,
                None => by_level.push((l.elevation, l.name.clone(), 1)),
            }
        }
        let sill = (o.sill * 100.0).round() / 100.0;
        if !sills.iter().any(|s| (s - sill).abs() < 1e-6) {
            sills.push(sill);
        }
        // The rooms a step to each side of its centre.
        let thickness = model
            .wall_type(w.wall_type)
            .map_or(0.2, |wt| wt.thickness());
        let (ext, int) = w.face_offsets(thickness);
        let names: Vec<String> = match w.opening_frame(o.along, t.width) {
            Some((p, u)) => {
                let n = u.perp_left();
                [ext + 0.15, int - 0.15]
                    .iter()
                    .filter_map(|off| {
                        let q = p + n * *off;
                        model
                            .rooms_on_level(w.level)
                            .find(|r| r.contains(q))
                            .map(|r| r.name.clone())
                    })
                    .collect()
            }
            None => Vec::new(),
        };
        let mut names = names;
        names.sort();
        names.dedup();
        if names.is_empty() {
            continue;
        }
        let place = names.join(" / ");
        match rooms.iter_mut().find(|(n, _)| *n == place) {
            Some(e) => e.1 += 1,
            None => rooms.push((place, 1)),
        }
    }
    by_level.sort_by(|a, b| a.0.total_cmp(&b.0));
    sills.sort_by(f64::total_cmp);
    rooms.sort();
    Entry {
        t,
        count,
        family: model
            .family(t.family)
            .map(|f| f.def().name.clone())
            .unwrap_or_default(),
        levels: by_level.into_iter().map(|(_, n, c)| (n, c)).collect(),
        sills,
        rooms,
    }
}

/// The sills in words: "0,90 m", "0,90 y 1,10 m", "sin antepecho".
pub(crate) fn sills_text(sills: &[f64]) -> String {
    match sills {
        [] => String::new(),
        [s] if s.abs() < 0.005 => "sin antepecho".to_string(),
        [s] => format!("{} m", metres(*s)),
        _ => {
            let v: Vec<String> = sills.iter().map(|s| metres(*s)).collect();
            let (last, rest) = v.split_last().unwrap();
            format!("{} y {last} m", rest.join(", "))
        }
    }
}

/// Units by level in words: "Planta baja 3, Planta primera 2".
pub(crate) fn levels_text(levels: &[(String, usize)]) -> String {
    levels
        .iter()
        .map(|(n, c)| format!("{n} {c}"))
        .collect::<Vec<_>>()
        .join(", ")
}

/// The sheets, as SVG documents, and their titles.
pub fn carpentry_sheets(model: &Model) -> Vec<(String, String)> {
    let entries = entries(model);
    let per = COLS * ROWS;
    let pages = entries.len().div_ceil(per).max(1);
    let project = model.project_name.clone();
    entries
        .chunks(per)
        .enumerate()
        .map(|(i, chunk)| {
            let title = format!("Carpinterías {}/{}", i + 1, pages);
            (sheet(model, chunk, &project, i + 1, pages), title)
        })
        .collect()
}

/// A mark's letters and number, for sorting P-2 before P-10.
pub(crate) fn natural(mark: &str) -> (String, u64) {
    let letters: String = mark.chars().filter(|c| c.is_alphabetic()).collect();
    let digits: String = mark.chars().filter(|c| c.is_ascii_digit()).collect();
    (letters, digits.parse().unwrap_or(0))
}

fn sheet(model: &Model, entries: &[Entry], project: &str, n: usize, of: usize) -> String {
    let (w, _) = PAPER;
    let (mut s, by) = paper::sheet(&paper::TitleBlock {
        project,
        title: "Carpinterías: alzados desde el exterior y secciones",
        scale: "E 1:50",
        sheet: n,
        sheets: of,
    });
    // The cells.
    let cw = (w - 2.0 * MARGIN) / COLS as f64;
    let ch = (by - MARGIN) / ROWS as f64;
    for (i, e) in entries.iter().enumerate() {
        let (c, r) = (i % COLS, i / COLS);
        let x0 = MARGIN + c as f64 * cw;
        let y0 = MARGIN + r as f64 * ch;
        cell(&mut s, model, e, x0, y0, cw, ch);
        if c > 0 {
            let _ = write!(
                s,
                r#"<line x1="{x0}" y1="{y0}" x2="{x0}" y2="{}" stroke="{INK}" stroke-width="0.18"/>"#,
                y0 + ch
            );
        }
        if r > 0 {
            let _ = write!(
                s,
                r#"<line x1="{x0}" y1="{y0}" x2="{}" y2="{y0}" stroke="{INK}" stroke-width="0.18"/>"#,
                x0 + cw
            );
        }
    }
    s.push_str("</svg>");
    s
}

/// One type: its mark and name above, its elevation with its measurements
/// and its section beside it, its data below.
fn cell(s: &mut String, model: &Model, e: &Entry, x0: f64, y0: f64, cw: f64, ch: f64) {
    let t = e.t;
    // The mark in a circle, the name beside it.
    let (mx, my) = (x0 + 12.0, y0 + 11.0);
    let _ = write!(
        s,
        r#"<circle cx="{mx}" cy="{my}" r="6" fill="none" stroke="{INK}" stroke-width="0.35"/>"#
    );
    let mark = if t.mark.is_empty() {
        "—"
    } else {
        t.mark.as_str()
    };
    text(s, mark, mx, my + 1.4, 3.2, Align::Centre);
    text(s, &t.name, x0 + 22.0, my + 1.4, 3.0, Align::Left);
    // The drawings' room: under the title, over the data.
    let data_h = 41.0;
    let (ax, ay) = (x0 + 14.0, y0 + 22.0);
    let room_h = ch - 22.0 - data_h - 6.0;
    let elev = opening_elevation(model, t);
    let sect = opening_section(model, t);
    let mut right = ax;
    if let Some(d) = &elev {
        // At 1:50, or smaller if it does not fit.
        let k = MM_PER_M
            .min(room_h / d.height())
            .min((cw - 60.0) / d.width());
        let (dw, dh) = (d.width() * k, d.height() * k);
        let (gx, gy) = (ax + 6.0, ay + (room_h - dh) / 2.0);
        drawing(s, d, gx, gy, k);
        // Overall width below and height left, of the door or window
        // itself (the drawing shows some wall round it).
        let cx = gx + dw / 2.0;
        let ow = t.width * k;
        let ohh = t.height * k;
        // The drawing's y = 0 is the sill.
        let sill = gy + dh - (0.0 - d.lo[1]) * k;
        dim_h(
            s,
            cx - ow / 2.0,
            cx + ow / 2.0,
            gy + dh + 5.0,
            &metres(t.width),
        );
        dim_v(s, gx - 5.0, sill - ohh, sill, &metres(t.height));
        right = gx + dw;
    }
    if let Some(d) = &sect {
        let k = MM_PER_M
            .min(room_h / d.height())
            .min(30.0 / d.width().max(0.01));
        let (dw, dh) = (d.width() * k, d.height() * k);
        let gx = (right + 8.0).min(x0 + cw - dw - 6.0);
        drawing(s, d, gx, ay + (room_h - dh) / 2.0, k);
        text(
            s,
            "Sección",
            gx + dw / 2.0,
            ay + (room_h + dh) / 2.0 + 5.0,
            2.2,
            Align::Centre,
        );
    }
    // The data.
    let ty = y0 + ch - data_h;
    let how = how_it_opens(t);
    let frame = model
        .material(t.frame_material)
        .map(|m| m.name.clone())
        .unwrap_or_default();
    let rows = [
        ("Tipo", e.family.clone()),
        (
            "Medidas",
            format!("{} × {} m", metres(t.width), metres(t.height)),
        ),
        ("Apertura", how),
        ("Carpintería", frame),
        (
            "Antepecho",
            if t.is_door() {
                String::new()
            } else {
                sills_text(&e.sills)
            },
        ),
        ("Unidades", e.count.to_string()),
        ("Situación", levels_text(&e.levels)),
    ];
    let shown = rows.iter().filter(|(_, v)| !v.is_empty());
    for (i, (k, v)) in shown.enumerate() {
        let y = ty + 4.0 + i as f64 * 5.2;
        text(s, k, x0 + 8.0, y, 2.4, Align::Left);
        // Long texts (where it is) cut to the cell.
        let first = paper::wrap(v, 2.4, cw - 40.0);
        let v = match first.as_slice() {
            [] => String::new(),
            [one] => one.clone(),
            [one, ..] => format!("{one}…"),
        };
        text(s, &v, x0 + 34.0, y, 2.4, Align::Left);
    }
}

/// How a door or window opens, in words.
pub(crate) fn how_it_opens(t: &OpeningType) -> String {
    use archisim_core::{DoorOperation as D, OpeningKind};
    if let OpeningKind::Door(op) = t.kind {
        return match op {
            D::SingleSwing => "1 hoja abatible",
            D::DoubleSwing => "2 hojas abatibles",
            D::Sliding => "corredera",
            D::Folding => "plegable",
            D::Pivot => "pivotante",
            D::Overhead => "seccional o basculante",
            D::Rolling => "enrollable",
        }
        .to_string();
    }
    if t.panels.is_empty() {
        return String::new();
    }
    let word = |p: &str| match p {
        "SIDEHUNGLEFTHAND" | "SIDEHUNGRIGHTHAND" => "practicable",
        "TILTANDTURNLEFTHAND" | "TILTANDTURNRIGHTHAND" => "oscilobatiente",
        "SLIDINGHORIZONTAL" => "corredera",
        "SLIDINGVERTICAL" => "guillotina",
        "FIXEDCASEMENT" => "fija",
        "TOPHUNG" | "BOTTOMHUNG" => "abatible",
        "PIVOTVERTICAL" | "PIVOTHORIZONTAL" => "pivotante",
        _ => "otra",
    };
    let words: Vec<&str> = t.panels.iter().map(|p| word(p)).collect();
    let n = words.len();
    let first = words[0];
    if words.iter().all(|w| *w == first) {
        if n == 1 {
            format!("1 hoja {first}")
        } else if first == "guillotina" {
            format!("{n} hojas de guillotina")
        } else {
            format!("{n} hojas {first}s")
        }
    } else {
        words.join(" + ")
    }
}

/// A horizontal dimension from `a` to `b` at height `y`.
fn dim_h(s: &mut String, a: f64, b: f64, y: f64, label: &str) {
    let _ = write!(
        s,
        r#"<path d="M{a},{y} L{b},{y} M{a},{} L{a},{} M{b},{} L{b},{} M{},{} L{},{} M{},{} L{},{}" stroke="{INK}" stroke-width="0.18" fill="none"/>"#,
        y - 2.0,
        y + 1.5,
        y - 2.0,
        y + 1.5,
        a - 1.0,
        y + 1.0,
        a + 1.0,
        y - 1.0,
        b - 1.0,
        y + 1.0,
        b + 1.0,
        y - 1.0
    );
    text(s, label, (a + b) / 2.0, y - 1.2, 2.2, Align::Centre);
}

/// A vertical dimension at `x` from `top` to `bottom`, its text upright.
fn dim_v(s: &mut String, x: f64, top: f64, bottom: f64, label: &str) {
    let _ = write!(
        s,
        r#"<path d="M{x},{top} L{x},{bottom} M{},{top} L{},{top} M{},{bottom} L{},{bottom} M{},{} L{},{} M{},{} L{},{}" stroke="{INK}" stroke-width="0.18" fill="none"/>"#,
        x - 1.5,
        x + 2.0,
        x - 1.5,
        x + 2.0,
        x - 1.0,
        top + 1.0,
        x + 1.0,
        top - 1.0,
        x - 1.0,
        bottom + 1.0,
        x + 1.0,
        bottom - 1.0
    );
    let mid = (top + bottom) / 2.0;
    let d = lettering::text_path(
        label,
        Point2::new(x - 1.2, -mid),
        Point2::new(0.0, 1.0),
        2.2,
        Align::Centre,
    );
    let _ = write!(s, r#"<path d="{d}" {}/>"#, lettering::paint(2.2, INK));
}

/// A drawing of the types dialog at `(x, y)` of the paper (its top left),
/// `k` millimetres per metre, with the pens of paper.
pub(crate) fn drawing(s: &mut String, d: &Drawing, x: f64, y: f64, k: f64) {
    let px = |v: f64| x + (v - d.lo[0]) * k;
    let py = |v: f64| y + (d.hi[1] - v) * k;
    let pen = |p: Pen| match p {
        Pen::Thin => 0.13,
        Pen::Medium => 0.25,
        Pen::Thick => 0.5,
    };
    for item in &d.items {
        match item {
            Item::Rect {
                min,
                max,
                fill,
                pen: p,
                ..
            } => {
                let [r, g, b, a] = *fill;
                let _ = write!(
                    s,
                    r#"<rect x="{:.2}" y="{:.2}" width="{:.2}" height="{:.2}" fill="rgb({r},{g},{b})" fill-opacity="{:.3}" stroke="{INK}" stroke-width="{}"/>"#,
                    px(min[0]),
                    py(max[1]),
                    (max[0] - min[0]) * k,
                    (max[1] - min[1]) * k,
                    a as f64 / 255.0,
                    pen(*p)
                );
            }
            Item::Line {
                from,
                to,
                pen: p,
                dashed,
            } => {
                let dash = if *dashed {
                    r#" stroke-dasharray="1.2 0.8""#
                } else {
                    ""
                };
                let _ = write!(
                    s,
                    r#"<line x1="{:.2}" y1="{:.2}" x2="{:.2}" y2="{:.2}" stroke="{INK}" stroke-width="{}"{dash}/>"#,
                    px(from[0]),
                    py(from[1]),
                    px(to[0]),
                    py(to[1]),
                    pen(*p)
                );
            }
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn every_type_has_its_place_on_the_sheets() {
        // A new project uses none: all its types are drawn.
        let m = archisim_library::new_project("Carpinterías");
        let sheets = super::carpentry_sheets(&m);
        assert!(!sheets.is_empty());
        let types = m.opening_types().filter(|t| !t.is_void()).count();
        assert_eq!(sheets.len(), types.div_ceil(6));
        assert!(sheets[0].0.contains("width=\"420mm\""));
        let pdf = crate::documents::pdf_of(&super::carpentry_sheets(&m), "Carpinterías").unwrap();
        assert!(pdf.starts_with(b"%PDF"));
        // To look at them: CARPINTERIAS_OUT=folder (the demo's, if made).
        if let Ok(dir) = std::env::var("CARPINTERIAS_OUT") {
            let demo =
                std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../out/demo.archisim");
            let m = archisim_file::open(&demo).unwrap_or(m);
            std::fs::create_dir_all(&dir).unwrap();
            for (i, (svg, _)) in super::carpentry_sheets(&m).iter().enumerate() {
                std::fs::write(format!("{dir}/carpinterias-{}.svg", i + 1), svg).unwrap();
            }
            std::fs::write(
                format!("{dir}/carpinterias.pdf"),
                crate::documents::pdf_of(&super::carpentry_sheets(&m), "Carpinterías").unwrap(),
            )
            .unwrap();
        }
    }
}
