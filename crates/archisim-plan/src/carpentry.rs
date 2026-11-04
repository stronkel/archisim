//! Doors and windows on the sheets (the carpentry sheet, «plano de
//! carpinterías»): a view of one door or window type — its elevation from
//! outside with its overall width and height, its vertical section beside
//! it, and its data under it (what it is, how it opens, its frame, its
//! sills, how many there are and where). The drawings are the types
//! dialog's ([`crate::type_drawing`], from the recipes), so what the sheet
//! shows is what is built; the data follow the model.
//!
//! A card is drawn in metres of the drawing, like a plan, so a view on a
//! sheet scales and places it like any other.

use std::fmt::Write as _;

use archisim_core::{Model, OpeningType, OpeningTypeId, Point2};

use crate::lettering::{paint, text_path, text_width, Align};
use crate::num;
use crate::type_drawing::{opening_elevation, opening_section, Drawing, Item, Pen};

const INK: &str = "#1a1a1a";
const GREY: &str = "#6b6b6b";

/// One type, with where it is used.
pub struct Entry<'a> {
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
pub fn entries(model: &Model) -> Vec<Entry<'_>> {
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

/// One type with its units, levels, sills and rooms.
pub fn entry<'a>(model: &'a Model, t: &'a OpeningType) -> Entry<'a> {
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
        let mut names: Vec<String> = match w.opening_frame(o.along, t.width) {
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

/// Metres with two decimals and a comma.
pub fn metres(v: f64) -> String {
    format!("{v:.2}").replace('.', ",")
}

/// The sills in words: "0,90 m", "0,90 y 1,10 m", "sin antepecho".
pub fn sills_text(sills: &[f64]) -> String {
    match sills {
        [] => String::new(),
        [s] if s.abs() < 0.005 => "sin antepecho".to_string(),
        [s] => format!("{} m", metres(*s)),
        _ => {
            let v: Vec<String> = sills.iter().map(|s| metres(*s)).collect();
            let (last, rest) = v.split_last().expect("two or more");
            format!("{} y {last} m", rest.join(", "))
        }
    }
}

/// Units by level in words: "Planta baja 3, Planta primera 2".
pub fn levels_text(levels: &[(String, usize)]) -> String {
    levels
        .iter()
        .map(|(n, c)| format!("{n} {c}"))
        .collect::<Vec<_>>()
        .join(", ")
}

/// A mark's letters and number, for sorting P-2 before P-10.
pub fn natural(mark: &str) -> (String, u64) {
    let letters: String = mark.chars().filter(|c| c.is_alphabetic()).collect();
    let digits: String = mark.chars().filter(|c| c.is_ascii_digit()).collect();
    (letters, digits.parse().unwrap_or(0))
}

/// How a door or window opens, in words.
pub fn how_it_opens(t: &OpeningType) -> String {
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

/// The rows of data of a type: label and value (the empty ones left out).
pub fn data_rows(model: &Model, e: &Entry) -> Vec<(&'static str, String)> {
    let t = e.t;
    let frame = model
        .material(t.frame_material)
        .map(|m| m.name.clone())
        .unwrap_or_default();
    let rows = vec![
        ("Tipo", e.family.clone()),
        (
            "Medidas",
            format!("{} × {} m", metres(t.width), metres(t.height)),
        ),
        ("Apertura", how_it_opens(t)),
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
    rows.into_iter().filter(|(_, v)| !v.is_empty()).collect()
}

/// Layout of a card, paper millimetres.
const ROW_MM: f64 = 4.4;
const TEXT_MM: f64 = 2.2;
const NAME_MM: f64 = 2.8;
const VALUE_AT_MM: f64 = 19.0;
const DATA_W_MM: f64 = 72.0;
const GAP_MM: f64 = 6.0;
const DIM_MM: f64 = 6.0;

/// The card of a door or window type at 1:`scale`: its elevation with its
/// overall width and height, its section beside it and, to the right, its
/// name and data. Returns its region (drawing metres, lower-left and
/// upper-right) and its SVG body (y flipped, as plans); `None` if the
/// type has no drawing.
pub fn card(model: &Model, id: OpeningTypeId, scale: f64) -> Option<((Point2, Point2), String)> {
    let t = model.opening_type(id)?;
    let elev = opening_elevation(model, t)?;
    let sect = opening_section(model, t);
    let e = entry(model, t);
    let rows = data_rows(model, &e);
    let k = scale / 1000.0; // drawing metres per paper millimetre
                            // The elevation's lower left over the room for its width dimension.
    let base = DIM_MM * 2.0 * k;
    let ox = -elev.lo[0];
    let oy = base - elev.lo[1];
    let mut s = String::new();
    draw(&mut s, &elev, ox, oy, k);
    let mut right = elev.width();
    let mut top = base + elev.height();
    if let Some(d) = &sect {
        let sx = elev.width() + GAP_MM * k;
        // Its sill level with the elevation's.
        draw(&mut s, d, sx - d.lo[0], oy, k);
        let under = oy + d.lo[1] - 3.5 * k;
        label(
            &mut s,
            "Sección",
            Point2::new(sx + d.width() / 2.0, under),
            k,
            Align::Centre,
            GREY,
        );
        right = sx + d.width();
        top = top.max(oy + d.hi[1]);
    }
    // Overall width under it and height on its left, of the door or window
    // itself (the drawing shows some wall round it); the drawing's y = 0
    // is the sill.
    let (w, h) = (t.width, t.height);
    dim_h(
        &mut s,
        ox - w / 2.0,
        ox + w / 2.0,
        base - DIM_MM * k,
        &metres(w),
        k,
    );
    dim_v(&mut s, -DIM_MM * k, oy, oy + h, &metres(h), k);
    // The name and the data, to the right, from the top.
    let dx = right + GAP_MM * k;
    let name = fit_at(&t.name, NAME_MM, DATA_W_MM);
    let hn = NAME_MM * k;
    let _ = writeln!(
        s,
        r#"<path d="{}" {}/>"#,
        text_path(
            &name,
            Point2::new(dx, top - hn),
            Point2::new(1.0, 0.0),
            hn,
            Align::Left
        ),
        paint(hn, INK)
    );
    let first = top - hn - 3.0 * k;
    for (i, (label_text, value)) in rows.iter().enumerate() {
        let y = first - (i as f64 + 1.0) * ROW_MM * k + 1.2 * k;
        label(&mut s, label_text, Point2::new(dx, y), k, Align::Left, GREY);
        let v = fit(value, DATA_W_MM - VALUE_AT_MM);
        label(
            &mut s,
            &v,
            Point2::new(dx + VALUE_AT_MM * k, y),
            k,
            Align::Left,
            INK,
        );
    }
    let data_lo = first - rows.len() as f64 * ROW_MM * k;
    let lo = Point2::new(-(DIM_MM + 4.0) * k, data_lo.min(0.0) - 1.0 * k);
    let hi = Point2::new(dx + DATA_W_MM * k, top + 2.0 * k);
    Some(((lo, hi), s))
}

/// The title of a type's card: its mark (its name if it has none).
pub fn card_title(model: &Model, id: OpeningTypeId) -> String {
    match model.opening_type(id) {
        Some(t) if t.mark.trim().is_empty() => t.name.to_uppercase(),
        Some(t) => t.mark.clone(),
        None => String::new(),
    }
}

/// A text shortened with "…" to fit a width (millimetres).
fn fit(t: &str, width_mm: f64) -> String {
    fit_at(t, TEXT_MM, width_mm)
}

fn fit_at(t: &str, h_mm: f64, width_mm: f64) -> String {
    if text_width(t, h_mm) <= width_mm {
        return t.to_string();
    }
    let mut out = t.to_string();
    while !out.is_empty() && text_width(&format!("{out}…"), h_mm) > width_mm {
        out.pop();
    }
    format!("{}…", out.trim_end())
}

fn label(s: &mut String, t: &str, at: Point2, k: f64, align: Align, color: &str) {
    if t.trim().is_empty() {
        return;
    }
    let h = TEXT_MM * k;
    let _ = writeln!(
        s,
        r#"<path d="{}" {}/>"#,
        text_path(t, at, Point2::new(1.0, 0.0), h, align),
        paint(h, color)
    );
}

/// A drawing of a type, its own origin at `(ox, oy)`; pens in paper
/// millimetres, `k` drawing metres per millimetre.
fn draw(s: &mut String, d: &Drawing, ox: f64, oy: f64, k: f64) {
    let pen = |p: Pen| {
        k * match p {
            Pen::Thin => 0.13,
            Pen::Medium => 0.25,
            Pen::Thick => 0.5,
        }
    };
    let x = |v: f64| num(ox + v);
    let y = |v: f64| num(-(oy + v));
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
                let _ = writeln!(
                    s,
                    r#"<path d="M{},{} L{},{} L{},{} L{},{} Z" fill="rgb({r},{g},{b})" fill-opacity="{}" stroke="{INK}" stroke-width="{}"/>"#,
                    x(min[0]),
                    y(min[1]),
                    x(max[0]),
                    y(min[1]),
                    x(max[0]),
                    y(max[1]),
                    x(min[0]),
                    y(max[1]),
                    num(a as f64 / 255.0),
                    num(pen(*p))
                );
            }
            Item::Polygon {
                points,
                fill,
                pen: p,
            } => {
                let [r, g, b, a] = *fill;
                let mut d = String::new();
                for (i, q) in points.iter().enumerate() {
                    let _ = write!(
                        d,
                        "{}{},{} ",
                        if i == 0 { "M" } else { "L" },
                        x(q[0]),
                        y(q[1])
                    );
                }
                d.push('Z');
                let _ = writeln!(
                    s,
                    r#"<path d="{d}" fill="rgb({r},{g},{b})" fill-opacity="{}" stroke="{}" stroke-width="{}"/>"#,
                    num(a as f64 / 255.0),
                    if p.is_some() { INK } else { "none" },
                    num(p.map_or(0.0, pen))
                );
            }
            Item::Line {
                from,
                to,
                pen: p,
                dashed,
            } => {
                let dash = if *dashed {
                    format!(r#" stroke-dasharray="{} {}""#, num(1.2 * k), num(0.8 * k))
                } else {
                    String::new()
                };
                let _ = writeln!(
                    s,
                    r#"<path d="M{},{} L{},{}" stroke="{INK}" stroke-width="{}"{dash}/>"#,
                    x(from[0]),
                    y(from[1]),
                    x(to[0]),
                    y(to[1]),
                    num(pen(*p))
                );
            }
        }
    }
}

/// A line in drawing metres (y up).
fn seg(s: &mut String, a: Point2, b: Point2, w: f64) {
    let _ = writeln!(
        s,
        r#"<path d="M{},{} L{},{}" stroke="{INK}" stroke-width="{}"/>"#,
        num(a.x),
        num(-a.y),
        num(b.x),
        num(-b.y),
        num(w)
    );
}

/// A horizontal dimension from `a` to `b` at height `y`, ticks at 45°.
fn dim_h(s: &mut String, a: f64, b: f64, y: f64, text: &str, k: f64) {
    let w = 0.18 * k;
    seg(s, Point2::new(a, y), Point2::new(b, y), w);
    for x in [a, b] {
        seg(
            s,
            Point2::new(x, y - 1.5 * k),
            Point2::new(x, y + 2.0 * k),
            w,
        );
        seg(
            s,
            Point2::new(x - k, y - k),
            Point2::new(x + k, y + k),
            0.35 * k,
        );
    }
    label(
        s,
        text,
        Point2::new((a + b) / 2.0, y + 1.0 * k),
        k,
        Align::Centre,
        INK,
    );
}

/// A vertical dimension at `x` from `lo` to `hi`, its text upright.
fn dim_v(s: &mut String, x: f64, lo: f64, hi: f64, text: &str, k: f64) {
    let w = 0.18 * k;
    seg(s, Point2::new(x, lo), Point2::new(x, hi), w);
    for y in [lo, hi] {
        seg(
            s,
            Point2::new(x - 2.0 * k, y),
            Point2::new(x + 1.5 * k, y),
            w,
        );
        seg(
            s,
            Point2::new(x - k, y - k),
            Point2::new(x + k, y + k),
            0.35 * k,
        );
    }
    let h = TEXT_MM * k;
    let _ = writeln!(
        s,
        r#"<path d="{}" {}/>"#,
        text_path(
            text,
            Point2::new(x - 1.0 * k, (lo + hi) / 2.0),
            Point2::new(0.0, 1.0),
            h,
            Align::Centre
        ),
        paint(h, INK)
    );
}

/// Paper of the carpentry sheets: A3 landscape, millimetres.
const SHEET_MM: (f64, f64) = (420.0, 297.0);
/// Cards per sheet: two columns, and as many rows as fit of this height
/// (millimetres).
const COLS: usize = 2;
const ROW_H_MM: f64 = 82.0;
/// Room under each card for its title, millimetres.
const TITLE_ROOM_MM: f64 = 14.0;

/// Adds the carpentry sheets: every door and window type the project uses
/// (or all, if it uses none) as a card, in two columns on A3 sheets clear
/// of the default title block, each sheet at one scale (1:50, or smaller
/// if a card does not fit). Returns the sheets made.
pub fn add_carpentry_sheets(
    model: &mut Model,
) -> Result<Vec<archisim_core::SheetId>, archisim_core::ModelError> {
    let types: Vec<OpeningTypeId> = entries(model).iter().map(|e| e.t.id).collect();
    if types.is_empty() {
        return Ok(Vec::new());
    }
    let (w, h) = SHEET_MM;
    // Where the title block is not: a strip on the right, or the bottom.
    let probe = archisim_core::Sheet {
        id: archisim_core::SheetId(0),
        number: String::new(),
        name: String::new(),
        width: w,
        height: h,
        title_block: true,
        revision: String::new(),
        presentation: None,
        restyle_views: false,
        discipline: None,
        title_type: None,
    };
    let tb = model.sheet_title_block(&probe).into_owned();
    let (mut x0, mut y0, mut x1, y1): (f64, f64, f64, f64) = (26.0, 14.0, w - 14.0, h - 16.0);
    match tb.design {
        archisim_core::TitleBlockDesign::Classic => y0 = 10.0 + 56.0 + 14.0,
        _ => x1 = w - 10.0 - tb.width - 6.0,
    }
    x0 = x0.min(x1 - 10.0);
    let rows = (((y1 - y0) / ROW_H_MM).floor() as usize).max(1);
    let (cw, ch) = ((x1 - x0) / COLS as f64, (y1 - y0) / rows as f64);
    let per = COLS * rows;
    let pages = types.len().div_ceil(per);
    let mut made = Vec::new();
    for (p, chunk) in types.chunks(per).enumerate() {
        let taken = |m: &Model, n: &str| m.sheets().iter().any(|s| s.number == n);
        let number = (1..)
            .map(|i| format!("C-{i:02}"))
            .find(|n| !taken(model, n))
            .unwrap_or_default();
        let name = if pages > 1 {
            format!("Carpinterías ({}/{pages})", p + 1)
        } else {
            "Carpinterías".to_string()
        };
        let sheet = model.add_sheet(archisim_core::NewSheet {
            number,
            name,
            width: w,
            height: h,
        })?;
        if let Some(mut s) = model.sheet(sheet).cloned() {
            s.discipline = Some(archisim_core::ElementDiscipline::Architecture);
            model.set_sheet(s)?;
        }
        // One scale for the sheet: the largest at which every card fits.
        let room = (cw - 4.0, ch - TITLE_ROOM_MM - 4.0);
        let fits = |m: &Model, t: OpeningTypeId, sc: f64| {
            card(m, t, sc).is_none_or(|((lo, hi), _)| {
                (hi.x - lo.x) / sc * 1000.0 <= room.0 && (hi.y - lo.y) / sc * 1000.0 <= room.1
            })
        };
        let scale = [50.0, 100.0, 150.0, 200.0]
            .into_iter()
            .find(|sc| chunk.iter().all(|t| fits(model, *t, *sc)))
            .unwrap_or(200.0);
        for (i, t) in chunk.iter().enumerate() {
            let (c, r) = (i % COLS, i / COLS);
            let Some(((lo, hi), _)) = card(model, *t, scale) else {
                continue;
            };
            let size = (
                (hi.x - lo.x) / scale * 1000.0,
                (hi.y - lo.y) / scale * 1000.0,
            );
            // Left in its cell, from the top of its row.
            let cx = x0 + c as f64 * cw + 2.0 + size.0 / 2.0;
            let top = y1 - r as f64 * ch;
            let cy = top - 2.0 - size.1 / 2.0;
            model.add_opening_viewport(sheet, *t, Point2::new(cx * 0.001, cy * 0.001), scale)?;
        }
        made.push(sheet);
    }
    Ok(made)
}
