//! Schedules ("tablas de planificación"): doors and windows, walls and
//! slabs by type, and the rooms with their areas and finishes, read from
//! the model as Revit's schedules and ArchiCAD's interactive schedules
//! are. They are drawn on sheets and written as CSV; the measurements go
//! to a BC3 budget (`archisim-export`).

use std::collections::BTreeMap;
use std::fmt::Write;

use archisim_core::{LevelId, Model, OpeningKind, Point2, ScheduleKind, Text, TextAlign};
use archisim_draft::font::{text_strokes, text_width, STROKE};

use crate::{num, pt};

/// A column of a schedule.
#[derive(Debug, Clone, PartialEq)]
pub struct Column {
    pub name: &'static str,
    /// Numbers align to the right.
    pub numeric: bool,
}

/// A schedule: its rows as written, and the totals row if it has one.
#[derive(Debug, Clone, PartialEq)]
pub struct Table {
    pub title: String,
    pub columns: Vec<Column>,
    pub rows: Vec<Vec<String>>,
    pub totals: Option<Vec<String>>,
}

const fn col(name: &'static str, numeric: bool) -> Column {
    Column { name, numeric }
}

fn dec(v: f64, places: usize) -> String {
    let f = 10f64.powi(places as i32);
    let v = (v * f).round() / f;
    let v = if v == 0.0 { 0.0 } else { v };
    format!("{v:.places$}").replace('.', ",")
}

/// One quantity of a type, element by element, for budgets.
#[derive(Debug, Clone, PartialEq)]
pub struct Measured {
    /// What the line is: "Muro 12 · Planta baja".
    pub comment: String,
    pub units: f64,
    pub length: f64,
    pub width: f64,
    pub height: f64,
    /// The quantity it adds (units × the dimensions that apply).
    pub quantity: f64,
}

/// A line of a budget: a type and its measurements.
#[derive(Debug, Clone, PartialEq)]
pub struct Item {
    /// Short code: the type mark, or one made up ("M01").
    pub code: String,
    /// `m2`, `m3`, `m`, `ud`.
    pub unit: &'static str,
    pub summary: String,
    pub lines: Vec<Measured>,
}

impl Item {
    pub fn total(&self) -> f64 {
        self.lines.iter().map(|l| l.quantity).sum()
    }
}

/// The chapters of a budget's measurements: (code, title, items).
pub fn measurements(model: &Model) -> Vec<(&'static str, &'static str, Vec<Item>)> {
    let level_name = |l: LevelId| model.level(l).map_or(String::new(), |l| l.name.clone());
    // Walls by type, m² of one face less the openings.
    let mut walls: BTreeMap<String, Item> = BTreeMap::new();
    for (i, t) in model.wall_types().enumerate() {
        let mut lines: Vec<Measured> = Vec::new();
        for w in model.walls().filter(|w| w.wall_type == t.id) {
            lines.push(Measured {
                comment: format!("Muro {} · {}", w.id.0, level_name(w.level)),
                units: 1.0,
                length: w.length(),
                width: 0.0,
                height: w.height,
                quantity: w.length() * w.height,
            });
            // Its doors and windows, taken off.
            for o in model.openings_in_wall(w.id) {
                if let Some(ot) = model.opening_type(o.opening_type) {
                    let name = if ot.mark.trim().is_empty() {
                        ot.name.clone()
                    } else {
                        ot.mark.clone()
                    };
                    lines.push(Measured {
                        comment: format!("A deducir: hueco {name}"),
                        units: -1.0,
                        length: 0.0,
                        width: ot.width,
                        height: ot.height,
                        quantity: -ot.width * ot.height,
                    });
                }
            }
        }
        if !lines.is_empty() {
            walls.insert(
                format!("{:03}", i),
                Item {
                    code: format!("M{:02}", i + 1),
                    unit: "m2",
                    summary: format!("{} ({} cm)", t.name, dec(t.thickness() * 100.0, 1)),
                    lines,
                },
            );
        }
    }
    let mut slabs: BTreeMap<String, Item> = BTreeMap::new();
    for (i, t) in model.slab_types().enumerate() {
        let lines: Vec<Measured> = model
            .slabs()
            .filter(|s| s.slab_type == t.id)
            .map(|s| Measured {
                comment: format!("Forjado {} · {}", s.id.0, level_name(s.level)),
                units: 1.0,
                length: 0.0,
                width: 0.0,
                height: 0.0,
                quantity: s.area(),
            })
            .collect();
        if !lines.is_empty() {
            slabs.insert(
                format!("{:03}", i),
                Item {
                    code: format!("F{:02}", i + 1),
                    unit: "m2",
                    summary: format!("{} ({} cm)", t.name, dec(t.thickness() * 100.0, 1)),
                    lines,
                },
            );
        }
    }
    let mut joinery: BTreeMap<String, Item> = BTreeMap::new();
    for (i, t) in model.opening_types().enumerate() {
        if t.kind == OpeningKind::Void {
            continue;
        }
        let lines: Vec<Measured> = model
            .openings()
            .filter(|o| o.opening_type == t.id)
            .map(|o| {
                let level = model.wall(o.wall).map(|w| w.level);
                Measured {
                    comment: format!(
                        "{} {} · {}",
                        if t.is_door() { "Puerta" } else { "Ventana" },
                        o.id.0,
                        level.map(level_name).unwrap_or_default()
                    ),
                    units: 1.0,
                    length: 0.0,
                    width: t.width,
                    height: t.height,
                    quantity: 1.0,
                }
            })
            .collect();
        if !lines.is_empty() {
            let code = code_of(&t.mark).unwrap_or_else(|| format!("C{:02}", i + 1));
            joinery.insert(
                format!("{code}{i:03}"),
                Item {
                    code,
                    unit: "ud",
                    summary: format!("{} {}×{}", t.name, dec(t.width, 2), dec(t.height, 2)),
                    lines,
                },
            );
        }
    }
    // Finishes of the rooms: floors and ceilings by area, walls by
    // perimeter times height.
    let mut finishes: BTreeMap<(u8, String), Item> = BTreeMap::new();
    for r in model.rooms() {
        let h = model.room_height(r);
        for (kind, name, unit_area) in [
            (0u8, &r.finishes.floor, r.area()),
            (1, &r.finishes.walls, r.perimeter() * h),
            (2, &r.finishes.ceiling, r.area()),
        ] {
            if name.trim().is_empty() {
                continue;
            }
            let what = ["Suelo", "Paredes", "Techo"][kind as usize];
            let n = finishes.len() + 1;
            let item = finishes
                .entry((kind, name.clone()))
                .or_insert_with(|| Item {
                    code: format!("A{n:02}"),
                    unit: "m2",
                    summary: format!("{what}: {name}"),
                    lines: Vec::new(),
                });
            item.lines.push(Measured {
                comment: format!("{} {} · {}", r.number, r.name, level_name(r.level)),
                units: 1.0,
                length: if kind == 1 { r.perimeter() } else { 0.0 },
                width: 0.0,
                height: if kind == 1 { h } else { 0.0 },
                quantity: unit_area,
            });
        }
    }
    vec![
        ("01", "Fábricas y muros", walls.into_values().collect()),
        ("02", "Forjados", slabs.into_values().collect()),
        ("03", "Carpintería", joinery.into_values().collect()),
        (
            "04",
            "Acabados de locales",
            finishes.into_values().collect(),
        ),
    ]
}

/// A type mark usable as a BC3 code (letters, digits, `.`, `-`, `_`).
fn code_of(mark: &str) -> Option<String> {
    let m = mark.trim();
    (!m.is_empty()
        && m.len() <= 18
        && m.chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '_')))
    .then(|| m.to_string())
}

/// A schedule of the model (of one level, or of all).
pub fn table(model: &Model, kind: ScheduleKind, level: Option<LevelId>) -> Table {
    let on = |l: Option<LevelId>| level.is_none() || l == level;
    let level_name = |l: LevelId| model.level(l).map_or(String::new(), |l| l.name.clone());
    let scope = level
        .map(|l| format!(" · {}", level_name(l)))
        .unwrap_or_default();
    match kind {
        ScheduleKind::Openings => {
            let mut rows = Vec::new();
            let mut total = 0;
            for t in model.opening_types() {
                if t.kind == OpeningKind::Void {
                    continue;
                }
                let mut levels: Vec<String> = Vec::new();
                let mut n = 0;
                for o in model.openings().filter(|o| o.opening_type == t.id) {
                    let l = model.wall(o.wall).map(|w| w.level);
                    if !on(l) {
                        continue;
                    }
                    n += 1;
                    if let Some(l) = l {
                        let name = level_name(l);
                        if !levels.contains(&name) {
                            levels.push(name);
                        }
                    }
                }
                if n == 0 {
                    continue;
                }
                total += n;
                rows.push(vec![
                    t.mark.clone(),
                    t.name.clone(),
                    if t.is_door() { "Puerta" } else { "Ventana" }.into(),
                    dec(t.width, 2),
                    dec(t.height, 2),
                    n.to_string(),
                    levels.join(", "),
                ]);
            }
            Table {
                title: format!("CARPINTERÍAS{scope}"),
                columns: vec![
                    col("Marca", false),
                    col("Tipo", false),
                    col("Clase", false),
                    col("Ancho (m)", true),
                    col("Alto (m)", true),
                    col("Uds.", true),
                    col("Plantas", false),
                ],
                rows,
                totals: Some(vec![
                    "Total".into(),
                    String::new(),
                    String::new(),
                    String::new(),
                    String::new(),
                    total.to_string(),
                    String::new(),
                ]),
            }
        }
        ScheduleKind::Walls => {
            let mut rows = Vec::new();
            let (mut tl, mut ta, mut tv) = (0.0, 0.0, 0.0);
            for t in model.wall_types() {
                let (mut n, mut l, mut a, mut v) = (0, 0.0, 0.0, 0.0);
                for w in model
                    .walls()
                    .filter(|w| w.wall_type == t.id && on(Some(w.level)))
                {
                    let cut: f64 = model
                        .openings_in_wall(w.id)
                        .filter_map(|o| model.opening_type(o.opening_type))
                        .map(|ot| ot.width * ot.height)
                        .sum();
                    let area = w.length() * w.height - cut;
                    n += 1;
                    l += w.length();
                    a += area;
                    v += area * t.thickness();
                }
                if n == 0 {
                    continue;
                }
                tl += l;
                ta += a;
                tv += v;
                rows.push(vec![
                    t.name.clone(),
                    dec(t.thickness() * 100.0, 1),
                    n.to_string(),
                    dec(l, 2),
                    dec(a, 2),
                    dec(v, 2),
                ]);
            }
            Table {
                title: format!("MUROS{scope}"),
                columns: vec![
                    col("Tipo", false),
                    col("Espesor (cm)", true),
                    col("Nº", true),
                    col("Longitud (m)", true),
                    col("Superficie (m²)", true),
                    col("Volumen (m³)", true),
                ],
                rows,
                totals: Some(vec![
                    "Total".into(),
                    String::new(),
                    String::new(),
                    dec(tl, 2),
                    dec(ta, 2),
                    dec(tv, 2),
                ]),
            }
        }
        ScheduleKind::Slabs => {
            let mut rows = Vec::new();
            let (mut ta, mut tv) = (0.0, 0.0);
            for t in model.slab_types() {
                let (mut n, mut a) = (0, 0.0);
                for s in model
                    .slabs()
                    .filter(|s| s.slab_type == t.id && on(Some(s.level)))
                {
                    n += 1;
                    a += s.area();
                }
                if n == 0 {
                    continue;
                }
                let v = a * t.thickness();
                ta += a;
                tv += v;
                rows.push(vec![
                    t.name.clone(),
                    dec(t.thickness() * 100.0, 1),
                    n.to_string(),
                    dec(a, 2),
                    dec(v, 2),
                ]);
            }
            Table {
                title: format!("FORJADOS{scope}"),
                columns: vec![
                    col("Tipo", false),
                    col("Espesor (cm)", true),
                    col("Nº", true),
                    col("Superficie (m²)", true),
                    col("Volumen (m³)", true),
                ],
                rows,
                totals: Some(vec![
                    "Total".into(),
                    String::new(),
                    String::new(),
                    dec(ta, 2),
                    dec(tv, 2),
                ]),
            }
        }
        ScheduleKind::Rooms => {
            let mut rooms: Vec<&archisim_core::Room> =
                model.rooms().filter(|r| on(Some(r.level))).collect();
            let elev = |l: LevelId| model.level(l).map_or(0.0, |l| l.elevation);
            rooms.sort_by(|a, b| {
                elev(a.level)
                    .total_cmp(&elev(b.level))
                    .then_with(|| natural(&a.number).cmp(&natural(&b.number)))
            });
            let total: f64 = rooms.iter().map(|r| r.area()).sum();
            let rows = rooms
                .iter()
                .map(|r| {
                    let h = model.room_height(r);
                    vec![
                        r.number.clone(),
                        r.name.clone(),
                        level_name(r.level),
                        dec(r.area(), 2),
                        dec(r.perimeter(), 2),
                        dec(h, 2),
                        dec(r.area() * h, 2),
                        r.finishes.floor.clone(),
                        r.finishes.walls.clone(),
                        r.finishes.ceiling.clone(),
                    ]
                })
                .collect();
            Table {
                title: format!("CUADRO DE SUPERFICIES{scope}"),
                columns: vec![
                    col("Nº", false),
                    col("Local", false),
                    col("Planta", false),
                    col("Sup. útil (m²)", true),
                    col("Perímetro (m)", true),
                    col("Altura (m)", true),
                    col("Volumen (m³)", true),
                    col("Suelo", false),
                    col("Paredes", false),
                    col("Techo", false),
                ],
                rows,
                totals: Some(vec![
                    "Total".into(),
                    String::new(),
                    String::new(),
                    dec(total, 2),
                    String::new(),
                    String::new(),
                    String::new(),
                    String::new(),
                    String::new(),
                    String::new(),
                ]),
            }
        }
    }
}

/// "0.10" after "0.9": digits compared as numbers.
fn natural(s: &str) -> Vec<(String, u64)> {
    let mut out = Vec::new();
    let mut text = String::new();
    let mut digits = String::new();
    for c in s.chars() {
        if c.is_ascii_digit() {
            digits.push(c);
        } else {
            if !digits.is_empty() {
                out.push((std::mem::take(&mut text), digits.parse().unwrap_or(0)));
                digits.clear();
            }
            text.push(c);
        }
    }
    out.push((text, digits.parse().unwrap_or(0)));
    out
}

impl Table {
    /// As CSV for a spreadsheet: semicolons and decimal commas, as Spanish
    /// spreadsheets read it.
    pub fn csv(&self) -> String {
        let cell = |s: &str| {
            if s.contains([';', '"', '\n']) {
                format!("\"{}\"", s.replace('"', "\"\""))
            } else {
                s.to_string()
            }
        };
        let mut out = String::new();
        let line = |out: &mut String, cells: Vec<String>| {
            let v: Vec<String> = cells.iter().map(|c| cell(c)).collect();
            out.push_str(&v.join(";"));
            out.push_str("\r\n");
        };
        line(&mut out, vec![self.title.clone()]);
        line(
            &mut out,
            self.columns.iter().map(|c| c.name.to_string()).collect(),
        );
        for r in &self.rows {
            line(&mut out, r.clone());
        }
        if let Some(t) = &self.totals {
            line(&mut out, t.clone());
        }
        out
    }

    /// Width of each column and height of a row, paper millimetres.
    pub fn layout(&self) -> (Vec<f64>, f64) {
        let pad = 2.0;
        let widths = (0..self.columns.len())
            .map(|i| {
                let head = text_width(self.columns[i].name, HEAD);
                let cells = self
                    .rows
                    .iter()
                    .chain(self.totals.iter())
                    .filter_map(|r| r.get(i))
                    .map(|c| text_width(c, BODY))
                    .fold(0.0, f64::max);
                head.max(cells) + 2.0 * pad
            })
            .collect();
        (widths, ROW)
    }

    /// Size on paper, millimetres.
    pub fn size(&self) -> (f64, f64) {
        let (w, h) = self.layout();
        let rows = self.rows.len() + 2 + usize::from(self.totals.is_some());
        (w.iter().sum(), h * rows as f64)
    }
}

const HEAD: f64 = 2.0;
const BODY: f64 = 2.0;
const ROW: f64 = 6.0;

/// Draws a schedule on a sheet with its top-left corner at `at` (paper
/// metres).
pub fn draw_table(s: &mut String, t: &Table, at: Point2) {
    let k = 0.001;
    let (widths, row) = t.layout();
    let total_w: f64 = widths.iter().sum();
    let n_rows = t.rows.len() + 2 + usize::from(t.totals.is_some());
    let p = |x: f64, y: f64| Point2::new(at.x + x * k, at.y - y * k);
    let ink = "#1a1a1a";
    let mut d = String::new();
    let mut thick = String::new();
    // Outline and the line under the headings, thicker.
    let _ = write!(
        thick,
        "M{} L{} L{} L{} Z M{} L{} ",
        pt(p(0.0, 0.0)),
        pt(p(total_w, 0.0)),
        pt(p(total_w, row * n_rows as f64)),
        pt(p(0.0, row * n_rows as f64)),
        pt(p(0.0, 2.0 * row)),
        pt(p(total_w, 2.0 * row)),
    );
    for i in 3..n_rows {
        let y = row * i as f64;
        let _ = write!(d, "M{} L{} ", pt(p(0.0, y)), pt(p(total_w, y)));
    }
    let _ = write!(d, "M{} L{} ", pt(p(0.0, row)), pt(p(total_w, row)));
    let mut x = 0.0;
    for w in &widths[..widths.len().saturating_sub(1)] {
        x += w;
        let _ = write!(d, "M{} L{} ", pt(p(x, row)), pt(p(x, row * n_rows as f64)));
    }
    let _ = writeln!(
        s,
        r#"<path d="{}" stroke="{ink}" stroke-width="{}" fill="none"/>"#,
        d.trim_end(),
        num(0.13 * k)
    );
    let _ = writeln!(
        s,
        r#"<path d="{}" stroke="{ink}" stroke-width="{}" fill="none"/>"#,
        thick.trim_end(),
        num(0.35 * k)
    );
    let mut texts: Vec<Text> = Vec::new();
    texts.push(Text {
        at: p(total_w / 2.0, row / 2.0),
        content: t.title.clone(),
        height: 2.5,
        angle: 0.0,
        align: TextAlign::Middle,
    });
    let cell = |texts: &mut Vec<Text>, i: usize, r: usize, content: &str, h: f64| {
        let x0: f64 = widths[..i].iter().sum();
        let y = row * r as f64 + row / 2.0;
        let numeric = t.columns[i].numeric && r >= 2;
        let (x, align) = if numeric {
            (x0 + widths[i] - 2.0, TextAlign::MiddleRight)
        } else {
            (x0 + 2.0, TextAlign::MiddleLeft)
        };
        if !content.is_empty() {
            texts.push(Text {
                at: p(x, y),
                content: content.to_string(),
                height: h,
                angle: 0.0,
                align,
            });
        }
    };
    for (i, c) in t.columns.iter().enumerate() {
        cell(&mut texts, i, 1, c.name, HEAD);
    }
    for (r, rowv) in t.rows.iter().chain(t.totals.iter()).enumerate() {
        for (i, v) in rowv.iter().enumerate().take(t.columns.len()) {
            cell(&mut texts, i, r + 2, v, BODY);
        }
    }
    for tx in &texts {
        let mut d = String::new();
        for st in text_strokes(tx, k) {
            for (i, q) in st.iter().enumerate() {
                let _ = write!(d, "{}{} ", if i == 0 { "M" } else { "L" }, pt(*q));
            }
        }
        let _ = writeln!(
            s,
            r#"<path d="{}" stroke="{ink}" stroke-width="{}" stroke-linecap="round" fill="none"/>"#,
            d.trim_end(),
            num(STROKE * tx.height * k)
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn numbers_in_order_and_csv() {
        let mut v = vec!["0.10", "0.9", "0.02"];
        v.sort_by_key(|s| natural(s));
        assert_eq!(v, ["0.02", "0.9", "0.10"]);
        let t = Table {
            title: "T".into(),
            columns: vec![col("A", false), col("B", true)],
            rows: vec![vec!["x;y".into(), "1,50".into()]],
            totals: None,
        };
        assert_eq!(t.csv(), "T\r\nA;B\r\n\"x;y\";1,50\r\n");
        let (w, h) = t.size();
        assert!(w > 10.0 && (h - 18.0).abs() < 1e-9);
    }
}
