//! Two more documents with the carpentry sheets' title block (C6): the
//! carpentry report (memoria de carpinterías), every door and window type
//! described in words beside a small elevation, and the table of areas
//! (cuadro de superficies), the usable area of every room by level, with
//! the built area of every level and the areas by use.
//!
//! Like the carpentry sheets they are A3 SVG documents in millimetres,
//! with ArchiSim's lettering, that turn into PDF the same way. The areas
//! are the ones the code check uses (`archisim_cte::surfaces`), so the
//! table and the check never disagree.

use archisim_core::Model;
use archisim_plan::lettering::Align;

use crate::carpentry::{drawing, entries, how_it_opens, levels_text, sills_text, Entry};
use crate::paper::{self, metres, text, wrap, PaperTable, MARGIN, PAPER};
use crate::type_drawing::opening_elevation;

/// The documents of A3 sheets the app exports.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Document {
    /// The carpentry sheets: elevations at 1:50 (`carpentry`).
    Carpentry,
    /// The carpentry report.
    CarpentryReport,
    /// The table of areas.
    Areas,
    /// The bar schedule of the reinforcement (`bar_schedule`).
    Bars,
}

impl Document {
    pub fn title(self) -> &'static str {
        match self {
            Document::Carpentry => "Plano de carpinterías",
            Document::CarpentryReport => "Memoria de carpinterías",
            Document::Areas => "Cuadro de superficies",
            Document::Bars => "Despiece de armaduras",
        }
    }

    /// What its file is called after the project's name.
    pub fn file(self) -> &'static str {
        match self {
            Document::Carpentry => "carpinterias",
            Document::CarpentryReport => "memoria-carpinterias",
            Document::Areas => "superficies",
            Document::Bars => "despiece-armaduras",
        }
    }

    /// Its sheets, as SVG documents, and their titles.
    pub fn sheets(self, model: &Model) -> Vec<(String, String)> {
        match self {
            Document::Carpentry => crate::carpentry::carpentry_sheets(model),
            Document::CarpentryReport => carpentry_report_sheets(model),
            Document::Areas => areas_sheets(model),
            Document::Bars => crate::bar_schedule::bar_schedule_sheets(model),
        }
    }
}

/// Types on a sheet of the report: columns and rows.
const COLS: usize = 2;
const ROWS: usize = 4;
/// Paper millimetres per metre of the small elevations (1:100).
const MM_PER_M: f64 = 10.0;
/// Lettering of the paragraphs and the space between their lines.
const BODY: f64 = 2.4;
const LEADING: f64 = 4.2;

/// The carpentry report's sheets, as SVG documents, and their titles.
pub fn carpentry_report_sheets(model: &Model) -> Vec<(String, String)> {
    let entries = entries(model);
    let per = COLS * ROWS;
    let pages = entries.len().div_ceil(per).max(1);
    let project = model.project_name.clone();
    let chunks: Vec<&[Entry]> = if entries.is_empty() {
        vec![&[]]
    } else {
        entries.chunks(per).collect()
    };
    chunks
        .into_iter()
        .enumerate()
        .map(|(i, chunk)| {
            let (mut s, by) = paper::sheet(&paper::TitleBlock {
                project: &project,
                title: "Memoria de carpinterías",
                scale: "Alzados E 1:100",
                sheet: i + 1,
                sheets: pages,
            });
            let (w, _) = PAPER;
            let cw = (w - 2.0 * MARGIN) / COLS as f64;
            let ch = (by - MARGIN) / ROWS as f64;
            for (j, e) in chunk.iter().enumerate() {
                let (c, r) = (j % COLS, j / COLS);
                let x0 = MARGIN + c as f64 * cw;
                let y0 = MARGIN + r as f64 * ch;
                report_cell(&mut s, model, e, x0, y0, cw, ch);
                if c > 0 {
                    paper::line(&mut s, x0, y0, x0, y0 + ch, 0.18);
                }
                if r > 0 {
                    paper::line(&mut s, x0, y0, x0 + cw, y0, 0.18);
                }
            }
            if chunk.is_empty() {
                text(
                    &mut s,
                    "El proyecto no tiene puertas ni ventanas.",
                    MARGIN + 8.0,
                    MARGIN + 14.0,
                    3.0,
                    Align::Left,
                );
            }
            s.push_str("</svg>");
            (s, format!("Memoria de carpinterías {}/{}", i + 1, pages))
        })
        .collect()
}

/// The paragraphs that describe a type.
pub fn description(model: &Model, e: &Entry) -> Vec<String> {
    let t = e.t;
    let what = if t.is_door() { "Puerta" } else { "Ventana" };
    let how = how_it_opens(t);
    let mut first = format!(
        "{what}{} de {} m de ancho por {} m de alto (medidas del hueco)",
        if how.is_empty() {
            String::new()
        } else {
            format!(" de {how},")
        },
        metres(t.width),
        metres(t.height)
    );
    if t.is_window() && !e.sills.is_empty() {
        let sills = sills_text(&e.sills);
        if sills == "sin antepecho" {
            first.push_str(", sin antepecho");
        } else {
            first.push_str(&format!(", con antepecho a {sills}"));
        }
    }
    first.push('.');
    let mut out = vec![first];
    let material = |id| model.material(id).map(|m| m.name.clone());
    let mut frame = String::new();
    if let Some(m) = material(t.frame_material) {
        frame = format!("Carpintería de {}", m.to_lowercase());
    }
    if t.frame_width > 0.0 {
        let face = format!(
            "{} marco de {} cm de cara vista y {} cm de fondo",
            if frame.is_empty() { "Marco" } else { ", con" },
            cm(t.frame_width),
            cm(t.frame_depth)
        );
        frame = if frame.is_empty() {
            face.trim_start_matches(", con ").to_string()
        } else {
            format!("{frame}{face}")
        };
    }
    if let Some(m) = material(t.panel_material) {
        let what = if t.is_door() {
            "hoja"
        } else {
            "acristalamiento"
        };
        if frame.is_empty() {
            frame = format!("{} de {}", capital(what), m.to_lowercase());
        } else {
            frame = format!("{frame}; {what} de {}", m.to_lowercase());
        }
    }
    if !frame.is_empty() {
        out.push(format!("{frame}."));
    }
    if !t.description.trim().is_empty() {
        let d = t.description.trim();
        out.push(if d.ends_with('.') {
            d.to_string()
        } else {
            format!("{d}.")
        });
    }
    if !e.family.is_empty() {
        out.push(format!("Familia: {}.", e.family));
    }
    let mut units = format!(
        "Unidades: {}",
        if e.count == 0 {
            "ninguna colocada".to_string()
        } else {
            e.count.to_string()
        }
    );
    if !e.levels.is_empty() {
        units.push_str(&format!(" ({})", levels_text(&e.levels)));
    }
    units.push('.');
    out.push(units);
    if !e.rooms.is_empty() {
        let v: Vec<String> = e
            .rooms
            .iter()
            .map(|(n, c)| {
                if *c > 1 {
                    format!("{n} ({c})")
                } else {
                    n.clone()
                }
            })
            .collect();
        out.push(format!("Situación: {}.", v.join("; ")));
    }
    out
}

fn cm(v: f64) -> String {
    let c = v * 100.0;
    if (c - c.round()).abs() < 0.05 {
        format!("{}", c.round())
    } else {
        format!("{c:.1}").replace('.', ",")
    }
}

fn capital(s: &str) -> String {
    let mut c = s.chars();
    c.next()
        .map(|f| f.to_uppercase().chain(c).collect())
        .unwrap_or_default()
}

/// One type of the report: its mark and name, a small elevation on the
/// left and its description on the right.
fn report_cell(s: &mut String, model: &Model, e: &Entry, x0: f64, y0: f64, cw: f64, ch: f64) {
    let t = e.t;
    let (mx, my) = (x0 + 11.0, y0 + 10.0);
    s.push_str(&format!(
        r#"<circle cx="{mx}" cy="{my}" r="5.5" fill="none" stroke="{}" stroke-width="0.35"/>"#,
        paper::INK
    ));
    let mark = if t.mark.is_empty() {
        "—"
    } else {
        t.mark.as_str()
    };
    text(s, mark, mx, my + 1.3, 3.0, Align::Centre);
    text(s, &t.name, x0 + 20.0, my + 1.3, 3.0, Align::Left);
    // The elevation, at 1:100 or smaller if it does not fit.
    let (bx, by, bw, bh) = (x0 + 6.0, y0 + 19.0, 52.0, ch - 24.0);
    if let Some(d) = opening_elevation(model, t) {
        let k = MM_PER_M
            .min(bh / d.height().max(0.01))
            .min(bw / d.width().max(0.01));
        let (dw, dh) = (d.width() * k, d.height() * k);
        drawing(s, &d, bx + (bw - dw) / 2.0, by + (bh - dh) / 2.0, k);
    }
    // The words.
    let (tx, width) = (x0 + 64.0, cw - 70.0);
    let mut y = y0 + 22.0;
    let bottom = y0 + ch - 3.0;
    'paragraphs: for p in description(model, e) {
        for line in wrap(&p, BODY, width) {
            if y > bottom {
                break 'paragraphs;
            }
            text(s, &line, tx, y, BODY, Align::Left);
            y += LEADING;
        }
        y += 1.2;
    }
}

/// The table of areas' sheets, as SVG documents, and their titles.
pub fn areas_sheets(model: &Model) -> Vec<(String, String)> {
    let (rooms, levels) = archisim_cte::surfaces::table(model);
    let mut tables: Vec<PaperTable> = Vec::new();
    for l in &levels {
        let here: Vec<_> = rooms.iter().filter(|r| r.level == l.level).collect();
        if here.is_empty() {
            continue;
        }
        tables.push(PaperTable {
            title: l.name.to_uppercase(),
            columns: vec![
                ("Nº".into(), false),
                ("Local".into(), false),
                ("Uso".into(), false),
                ("Útil (m²)".into(), true),
            ],
            rows: here
                .iter()
                .map(|r| {
                    vec![
                        r.number.clone(),
                        r.name.clone(),
                        r.usage.clone(),
                        metres(r.area),
                    ]
                })
                .collect(),
            totals: Some(vec![
                String::new(),
                "Superficie útil".into(),
                String::new(),
                metres(l.usable),
            ]),
        });
    }
    let (usable, built): (f64, f64) = levels
        .iter()
        .fold((0.0, 0.0), |(u, b), l| (u + l.usable, b + l.built));
    tables.push(PaperTable {
        title: "RESUMEN POR PLANTAS".into(),
        columns: vec![
            ("Planta".into(), false),
            ("Útil (m²)".into(), true),
            ("Construida (m²)".into(), true),
        ],
        rows: levels
            .iter()
            .map(|l| vec![l.name.clone(), metres(l.usable), metres(l.built)])
            .collect(),
        totals: Some(vec!["Total".into(), metres(usable), metres(built)]),
    });
    let mut uses: Vec<(String, f64)> = Vec::new();
    for l in &levels {
        for (u, a) in &l.by_use {
            match uses.iter_mut().find(|(n, _)| n == u) {
                Some(e) => e.1 += a,
                None => uses.push((u.clone(), *a)),
            }
        }
    }
    if !uses.is_empty() {
        tables.push(PaperTable {
            title: "SUPERFICIE ÚTIL POR USOS".into(),
            columns: vec![("Uso".into(), false), ("Útil (m²)".into(), true)],
            rows: uses
                .iter()
                .map(|(u, a)| vec![u.clone(), metres(*a)])
                .collect(),
            totals: Some(vec!["Total".into(), metres(usable)]),
        });
    }
    let project = model.project_name.clone();
    let pages = flow(tables);
    let n = pages.len();
    pages
        .into_iter()
        .enumerate()
        .map(|(i, placed)| {
            let (mut s, _) = paper::sheet(&paper::TitleBlock {
                project: &project,
                title: "Cuadro de superficies",
                scale: "",
                sheet: i + 1,
                sheets: n,
            });
            for (t, x, y, w) in &placed {
                t.draw(&mut s, *x, *y, *w);
            }
            s.push_str("</svg>");
            (s, format!("Cuadro de superficies {}/{}", i + 1, n))
        })
        .collect()
}

/// Columns of tables on the areas sheets.
const FLOW_COLS: usize = 3;
const GAP: f64 = 8.0;

/// The tables placed down the columns of as many sheets as they need, a
/// table too long for a column cut and continued in the next one: for
/// each sheet, each table (or piece) with its top left and width.
fn flow(tables: Vec<PaperTable>) -> Vec<Vec<(PaperTable, f64, f64, f64)>> {
    let (w, h) = PAPER;
    let top = MARGIN + 6.0;
    let bottom = h - MARGIN - paper::BLOCK - 6.0;
    let col_w = (w - 2.0 * MARGIN - 2.0 * 6.0 - (FLOW_COLS - 1) as f64 * GAP) / FLOW_COLS as f64;
    let x_of = |c: usize| MARGIN + 6.0 + c as f64 * (col_w + GAP);
    let mut pages: Vec<Vec<(PaperTable, f64, f64, f64)>> = vec![Vec::new()];
    let (mut col, mut y) = (0usize, top);
    let next_col = |pages: &mut Vec<Vec<_>>, col: &mut usize, y: &mut f64| {
        *col += 1;
        *y = top;
        if *col == FLOW_COLS {
            *col = 0;
            pages.push(Vec::new());
        }
    };
    for mut t in tables {
        loop {
            let room = bottom - y;
            // The whole table, if it fits.
            if t.height() <= room {
                let hgt = t.height();
                pages.last_mut().unwrap().push((t, x_of(col), y, col_w));
                y += hgt + GAP;
                break;
            }
            // How many rows fit here, with the title and the headings.
            let fit = ((room / paper::ROW).floor() as usize).saturating_sub(2);
            if fit < 3 {
                next_col(&mut pages, &mut col, &mut y);
                continue;
            }
            let rest = t.rows.split_off(fit.min(t.rows.len()));
            let piece = PaperTable {
                title: t.title.clone(),
                columns: t.columns.clone(),
                rows: std::mem::replace(&mut t.rows, rest),
                totals: None,
            };
            pages.last_mut().unwrap().push((piece, x_of(col), y, col_w));
            if !t.title.ends_with("(sigue)") {
                t.title = format!("{} (sigue)", t.title);
            }
            next_col(&mut pages, &mut col, &mut y);
        }
    }
    pages
}

/// Sheets as one PDF.
pub fn pdf_of(sheets: &[(String, String)], title: &str) -> Result<Vec<u8>, String> {
    let pages: Vec<archisim_export::pdf::Page> = sheets
        .iter()
        .map(|(svg, title)| archisim_export::pdf::Page { svg, title })
        .collect();
    archisim_export::pdf::pdf(&pages, title)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn demo() -> Option<Model> {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../out/demo.archisim");
        archisim_file::open(&path).ok()
    }

    #[test]
    fn every_type_is_described() {
        let m = archisim_library::new_project("Memoria");
        let sheets = carpentry_report_sheets(&m);
        let types = m.opening_types().filter(|t| !t.is_void()).count();
        assert_eq!(sheets.len(), types.div_ceil(COLS * ROWS));
        let e = &entries(&m)[0];
        let words = description(&m, e);
        assert!(words[0].starts_with("Puerta"), "{words:?}");
        assert!(words[0].contains("m de ancho por"));
        assert!(words.iter().any(|w| w.starts_with("Unidades: ninguna")));
        assert!(pdf_of(&sheets, "Memoria").unwrap().starts_with(b"%PDF"));
    }

    #[test]
    fn a_window_says_its_sill_and_its_room() {
        let mut m = archisim_library::new_project("Memoria");
        let level = m.levels()[0].id;
        let wt = m.wall_types().next().unwrap().id;
        let pts = [(0.0, 0.0), (5.0, 0.0), (5.0, 4.0), (0.0, 4.0)];
        use archisim_core::{
            Hand, NewOpening, NewRoom, NewWall, Point2, ReferenceLine, WallSide, WallTop,
        };
        let mut walls = Vec::new();
        for i in 0..4 {
            let (a, b) = (pts[i], pts[(i + 1) % 4]);
            walls.push(
                m.add_wall(NewWall {
                    wall_type: wt,
                    level,
                    start: Point2::new(a.0, a.1),
                    end: Point2::new(b.0, b.1),
                    bulge: 0.0,
                    reference: ReferenceLine::Center,
                    height: 2.7,
                    base_offset: 0.0,
                    top: WallTop::Unconnected,
                })
                .unwrap(),
            );
        }
        let window = m.opening_types().find(|t| t.is_window()).unwrap().id;
        m.add_opening(NewOpening {
            opening_type: window,
            wall: walls[0],
            along: 2.5,
            sill: 0.9,
            opens_to: WallSide::Interior,
            hand: Hand::Left,
            setback: None,
        })
        .unwrap();
        m.add_room(NewRoom {
            level,
            number: "1".into(),
            name: "Salón".into(),
            outline: pts.iter().map(|(x, y)| Point2::new(*x, *y)).collect(),
            holes: Vec::new(),
            label: Point2::new(2.5, 2.0),
            bounded: false,
        })
        .unwrap();
        let e = entries(&m).into_iter().find(|e| e.t.id == window).unwrap();
        assert_eq!(e.count, 1);
        assert_eq!(e.sills, vec![0.9]);
        assert_eq!(e.rooms, vec![("Salón".to_string(), 1)]);
        let words = description(&m, &e).join(" ");
        assert!(words.contains("antepecho a 0,90 m"), "{words}");
        assert!(words.contains("Situación: Salón."), "{words}");
    }

    #[test]
    fn the_areas_add_up_and_long_lists_go_on() {
        let m = archisim_library::new_project("Superficies");
        let sheets = areas_sheets(&m);
        assert_eq!(sheets.len(), 1);
        assert!(sheets[0].0.contains("width=\"420mm\""));
        // A list too long for a column goes on in the next one.
        let long = PaperTable {
            title: "PLANTA BAJA".into(),
            columns: vec![("Local".into(), false), ("Útil (m²)".into(), true)],
            rows: (0..120)
                .map(|i| vec![format!("Local {i}"), metres(10.0)])
                .collect(),
            totals: Some(vec!["Superficie útil".into(), metres(1200.0)]),
        };
        let pages = flow(vec![long]);
        let pieces: Vec<_> = pages.iter().flatten().collect();
        assert!(pieces.len() >= 3);
        let rows: usize = pieces.iter().map(|p| p.0.rows.len()).sum();
        assert_eq!(rows, 120);
        assert!(pieces.last().unwrap().0.totals.is_some());
        assert!(pieces[1].0.title.ends_with("(sigue)"));
        // To look at them: DOCUMENTOS_OUT=folder (the demo's, if made).
        if let Ok(dir) = std::env::var("DOCUMENTOS_OUT") {
            let m = demo().unwrap_or(m);
            std::fs::create_dir_all(&dir).unwrap();
            for (name, sheets) in [
                ("memoria", carpentry_report_sheets(&m)),
                ("superficies", areas_sheets(&m)),
            ] {
                for (i, (svg, _)) in sheets.iter().enumerate() {
                    std::fs::write(format!("{dir}/{name}-{}.svg", i + 1), svg).unwrap();
                }
                std::fs::write(format!("{dir}/{name}.pdf"), pdf_of(&sheets, name).unwrap())
                    .unwrap();
            }
        }
    }
}
