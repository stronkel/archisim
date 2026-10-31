//! The bar schedule (despiece de armaduras, E17): every bar of the
//! reinforcement drawn flat with the length of each stretch, its mark,
//! diameter, number, length of one and of all and its weight, element by
//! element; and at the end the steel by diameter. A3 sheets with the
//! carpentry sheets' title block, like the other documents.
//!
//! The bars are the ones `archisim_geom::rebar` lays, so the schedule, the
//! 3D, the sections, the measurements and the IFC count the same bars.
//! Bars of a group with the same shape (to the centimetre) share a mark;
//! a slab's bars cut short at a hole get a mark of their own.

use std::fmt::Write as _;

use archisim_core::{Model, RebarKind};
use archisim_geom::rebar::{bar_shape, element_bars, shape_stretches};
use archisim_plan::lettering::Align;

use crate::paper::{self, text, text_right, PaperTable, INK, MARGIN, PAPER};

/// One mark: a shape of bar repeated `count` times in an element.
#[derive(Debug, Clone)]
pub struct Mark {
    pub number: usize,
    /// The element it belongs to.
    pub element: String,
    /// Where in the element: «longitudinales», «cercos»…
    pub place: String,
    pub diameter: f64,
    pub count: usize,
    /// The bar laid flat, metres.
    pub shape: Vec<[f64; 2]>,
    /// Whether it closes on itself (ties, stirrups).
    pub closed: bool,
    /// Length of one bar, metres.
    pub length: f64,
}

impl Mark {
    pub fn total(&self) -> f64 {
        self.length * self.count as f64
    }
    pub fn weight(&self) -> f64 {
        self.total() * archisim_core::bar_weight(self.diameter)
    }
}

fn kind_order(k: Option<RebarKind>) -> u8 {
    match k {
        Some(RebarKind::Footing) => 0,
        Some(RebarKind::Column) => 1,
        Some(RebarKind::Beam) => 2,
        Some(RebarKind::Slab) => 3,
        None => 4,
    }
}

/// Every mark of the project: by element (footings, columns, beams,
/// slabs), and in each element by group and shape.
pub fn marks(model: &Model) -> Vec<Mark> {
    let mut ids: Vec<u64> = model.reinforcements().map(|(id, _)| id).collect();
    ids.sort_by_key(|id| (kind_order(model.rebar_kind(*id)), *id));
    let mut out: Vec<Mark> = Vec::new();
    for id in ids {
        let element = archisim_cte::structure::element_name(model, id);
        for g in element_bars(model, id) {
            // Same shape: the same stretches to the centimetre.
            let mut shapes: Vec<(Vec<i64>, Mark)> = Vec::new();
            for b in &g.bars {
                let shape = bar_shape(b);
                let stretches = shape_stretches(&shape);
                let key: Vec<i64> = stretches
                    .iter()
                    .map(|l| (l * 100.0).round() as i64)
                    .collect();
                if key.is_empty() {
                    continue;
                }
                if let Some((_, m)) = shapes.iter_mut().find(|(k, _)| *k == key) {
                    m.count += 1;
                    continue;
                }
                let closed = b.len() > 2 && {
                    let (a, z) = (b[0], b[b.len() - 1]);
                    (a[0] - z[0]).abs() + (a[1] - z[1]).abs() + (a[2] - z[2]).abs() < 1e-6
                };
                shapes.push((
                    key,
                    Mark {
                        number: 0,
                        element: element.clone(),
                        place: g.place.clone(),
                        diameter: g.diameter,
                        count: 1,
                        length: stretches.iter().sum(),
                        shape,
                        closed,
                    },
                ));
            }
            // The longest first: the whole bars before those cut short.
            shapes.sort_by(|a, b| b.1.length.total_cmp(&a.1.length));
            out.extend(shapes.into_iter().map(|(_, m)| m));
        }
    }
    for (i, m) in out.iter_mut().enumerate() {
        m.number = i + 1;
    }
    out
}

/// The steel by diameter: (diameter, metres, kg), thinnest first.
pub fn by_diameter(marks: &[Mark]) -> Vec<(f64, f64, f64)> {
    let mut out: Vec<(f64, f64, f64)> = Vec::new();
    for m in marks {
        match out
            .iter_mut()
            .find(|(d, _, _)| (d - m.diameter).abs() < 1e-6)
        {
            Some(e) => {
                e.1 += m.total();
                e.2 += m.weight();
            }
            None => out.push((m.diameter, m.total(), m.weight())),
        }
    }
    out.sort_by(|a, b| a.0.total_cmp(&b.0));
    out
}

/// Height of a row with its drawn bar, millimetres.
const ROW_H: f64 = 16.0;
const HEAD_H: f64 = 7.0;
const CELL: f64 = 2.4;
/// The columns: heading, width (mm), right-aligned.
const COLUMNS: &[(&str, f64, bool)] = &[
    ("Elemento", 92.0, false),
    ("Marca", 14.0, true),
    ("Forma (cm)", 130.0, false),
    ("Ø (mm)", 16.0, true),
    ("Nº", 14.0, true),
    ("Long. (m)", 22.0, true),
    ("Total (m)", 24.0, true),
    ("Peso (kg)", 24.0, true),
];

fn dec(v: f64, n: usize) -> String {
    format!("{v:.n$}").replace('.', ",")
}

/// The bar schedule's sheets, as SVG documents, and their titles.
pub fn bar_schedule_sheets(model: &Model) -> Vec<(String, String)> {
    let marks = marks(model);
    let (_, h) = PAPER;
    let top = MARGIN + 6.0;
    let bottom = h - MARGIN - paper::BLOCK - 6.0;
    let per = ((bottom - top - HEAD_H) / ROW_H).floor().max(1.0) as usize;
    let summary = summary_table(&marks);
    let mut pages: Vec<&[Mark]> = marks.chunks(per).collect();
    if pages.is_empty() {
        pages.push(&[]);
    }
    // The summary under the last rows, or on a sheet of its own.
    let last_rows = pages.last().map_or(0, |p| p.len());
    let summary_y = top + HEAD_H + ROW_H * last_rows as f64 + 10.0;
    let own_sheet = !marks.is_empty() && summary_y + summary.height() > bottom;
    let n = pages.len() + usize::from(own_sheet);
    let project = model.project_name.clone();
    let x0 = MARGIN + 6.0;
    let table_w: f64 = COLUMNS.iter().map(|c| c.1).sum();
    let mut out = Vec::new();
    for (i, chunk) in pages.iter().enumerate() {
        let (mut s, _) = paper::sheet(&paper::TitleBlock {
            project: &project,
            title: "Despiece de armaduras",
            scale: "Formas sin escala",
            sheet: i + 1,
            sheets: n,
        });
        if marks.is_empty() {
            text(
                &mut s,
                "El proyecto no tiene elementos armados.",
                x0,
                top + 8.0,
                3.0,
                Align::Left,
            );
        } else {
            rows(&mut s, chunk, x0, top, table_w);
            if i + 1 == pages.len() && !own_sheet {
                summary.draw(&mut s, x0, summary_y, 120.0);
            }
        }
        s.push_str("</svg>");
        out.push((s, format!("Despiece de armaduras {}/{}", i + 1, n)));
    }
    if own_sheet {
        let (mut s, _) = paper::sheet(&paper::TitleBlock {
            project: &project,
            title: "Despiece de armaduras",
            scale: "",
            sheet: n,
            sheets: n,
        });
        summary.draw(&mut s, x0, top, 120.0);
        s.push_str("</svg>");
        out.push((s, format!("Despiece de armaduras {n}/{n}")));
    }
    out
}

fn summary_table(marks: &[Mark]) -> PaperTable {
    let d = by_diameter(marks);
    let kg: f64 = d.iter().map(|x| x.2).sum();
    let m: f64 = d.iter().map(|x| x.1).sum();
    PaperTable {
        title: "ACERO B 500 S POR DIÁMETROS".into(),
        columns: vec![
            ("Diámetro".into(), false),
            ("Longitud (m)".into(), true),
            ("Peso (kg)".into(), true),
        ],
        rows: d
            .iter()
            .map(|(dd, l, k)| {
                vec![
                    format!("Ø{}", (dd * 1000.0).round() as i64),
                    dec(*l, 2),
                    dec(*k, 1),
                ]
            })
            .collect(),
        totals: Some(vec!["Total".into(), dec(m, 2), dec(kg, 1)]),
    }
}

/// The headings and the rows of marks, with their drawn bars.
fn rows(s: &mut String, chunk: &[Mark], x0: f64, top: f64, width: f64) {
    let bottom = top + HEAD_H + ROW_H * chunk.len() as f64;
    let _ = write!(
        s,
        r#"<rect x="{x0:.2}" y="{top:.2}" width="{width:.2}" height="{:.2}" fill="none" stroke="{INK}" stroke-width="0.35"/>"#,
        bottom - top
    );
    paper::line(s, x0, top + HEAD_H, x0 + width, top + HEAD_H, 0.35);
    let mut cx = x0;
    for (i, (name, cw, right)) in COLUMNS.iter().enumerate() {
        if i > 0 {
            paper::line(s, cx, top, cx, bottom, 0.13);
        }
        if *right {
            text_right(s, name, cx + cw - 2.0, top + HEAD_H - 2.2, CELL);
        } else {
            text(s, name, cx + 2.0, top + HEAD_H - 2.2, CELL, Align::Left);
        }
        cx += cw;
    }
    let mut previous: Option<&str> = None;
    for (r, m) in chunk.iter().enumerate() {
        let y = top + HEAD_H + ROW_H * r as f64;
        if r > 0 {
            // A thicker line between elements.
            let pen = if previous != Some(m.element.as_str()) {
                0.25
            } else {
                0.1
            };
            paper::line(s, x0, y, x0 + width, y, pen);
        }
        let base = y + ROW_H / 2.0 + 0.9;
        let mut cx = x0;
        for (i, (_, cw, right)) in COLUMNS.iter().enumerate() {
            let value = match i {
                0 => {
                    if previous != Some(m.element.as_str()) {
                        text(
                            s,
                            &fit(&m.element, cw - 3.0),
                            cx + 2.0,
                            base - 2.0,
                            CELL,
                            Align::Left,
                        );
                    }
                    text(
                        s,
                        &fit(&m.place, cw - 3.0),
                        cx + 2.0,
                        base + 2.4,
                        2.0,
                        Align::Left,
                    );
                    None
                }
                1 => Some(m.number.to_string()),
                2 => {
                    draw_shape(s, m, cx + 3.0, y + 2.5, cw - 6.0, ROW_H - 5.0);
                    None
                }
                3 => Some(((m.diameter * 1000.0).round() as i64).to_string()),
                4 => Some(m.count.to_string()),
                5 => Some(dec(m.length, 2)),
                6 => Some(dec(m.total(), 2)),
                _ => Some(dec(m.weight(), 1)),
            };
            if let Some(v) = value {
                if *right {
                    text_right(s, &v, cx + cw - 2.0, base, CELL);
                } else {
                    text(s, &v, cx + 2.0, base, CELL, Align::Left);
                }
            }
            cx += cw;
        }
        previous = Some(m.element.as_str());
    }
}

/// The bar turned so that its longest stretch lies along x, its bent ends
/// up (as a bar schedule draws them).
fn lay_down(shape: &[[f64; 2]]) -> Vec<[f64; 2]> {
    let Some((i, _)) = shape_stretches(shape)
        .iter()
        .enumerate()
        .max_by(|a, b| a.1.total_cmp(b.1))
    else {
        return shape.to_vec();
    };
    let (a, b) = (shape[i], shape[i + 1]);
    let ang = (b[1] - a[1]).atan2(b[0] - a[0]);
    let (c, sn) = (ang.cos(), ang.sin());
    let mut out: Vec<[f64; 2]> = shape
        .iter()
        .map(|p| {
            let (x, y) = (p[0] - a[0], p[1] - a[1]);
            [x * c + y * sn, -x * sn + y * c]
        })
        .collect();
    let mean: f64 = out.iter().map(|p| p[1]).sum::<f64>() / out.len() as f64;
    if mean < -1e-9 {
        for p in &mut out {
            p[1] = -p[1];
        }
    }
    out
}

fn fit(t: &str, width: f64) -> String {
    let mut v = t.to_string();
    while v.chars().count() > 1 && archisim_plan::lettering::text_width(&v, CELL) > width {
        v.pop();
    }
    v
}

/// The bar laid flat in the box, kept in proportion unless a straight bar
/// (drawn the box's width), with the length of each stretch in cm.
fn draw_shape(s: &mut String, m: &Mark, x: f64, y: f64, w: f64, h: f64) {
    let pts = &lay_down(&m.shape);
    if pts.len() < 2 {
        return;
    }
    let (mut lo, mut hi) = ([f64::MAX; 2], [f64::MIN; 2]);
    for p in pts {
        for k in 0..2 {
            lo[k] = lo[k].min(p[k]);
            hi[k] = hi[k].max(p[k]);
        }
    }
    let (sw, sh) = ((hi[0] - lo[0]).max(1e-6), (hi[1] - lo[1]).max(1e-6));
    // Room for the numbers round it.
    let (bw, bh) = (w - 16.0, h - 5.0);
    // Ties in proportion; open bars as long as the box, their bent ends
    // as tall as fits.
    let (k, ky) = if m.closed {
        let k = (bw / sw).min(bh / sh);
        (k, k)
    } else {
        let k = bw / sw;
        (k, k.min(bh / sh))
    };
    let (dw, dh) = (sw * k, sh * ky);
    let ox = x + 8.0 + (bw - dw) / 2.0;
    let oy = y + 2.5 + (bh + dh) / 2.0;
    let at = |p: &[f64; 2]| (ox + (p[0] - lo[0]) * k, oy - (p[1] - lo[1]) * ky);
    let mut d = String::new();
    for (i, p) in pts.iter().enumerate() {
        let (px, py) = at(p);
        let _ = write!(d, "{}{px:.2},{py:.2}", if i == 0 { "M" } else { " L" });
    }
    let _ = write!(
        s,
        r#"<path d="{d}" fill="none" stroke="{INK}" stroke-width="0.5" stroke-linejoin="round"/>"#
    );
    // The stretches: all of an open bar, two sides of a closed one.
    let stretches = shape_stretches(pts);
    let n = if m.closed {
        2.min(stretches.len())
    } else {
        stretches.len()
    };
    let (cx, cy) = (ox + dw / 2.0, oy - dh / 2.0);
    for (i, l) in stretches.iter().take(n).enumerate() {
        let (ax, ay) = at(&pts[i]);
        let (bx, by) = at(&pts[i + 1]);
        let (mx, my) = ((ax + bx) / 2.0, (ay + by) / 2.0);
        let label = ((l * 100.0).round() as i64).to_string();
        let horizontal = (by - ay).abs() < (bx - ax).abs();
        if horizontal {
            // Above the stretch, or below if it is the bottom of the shape.
            let below = my > cy + 0.1;
            let ty = if below { my + 3.0 } else { my - 1.5 };
            text(s, &label, mx, ty, 2.0, Align::Centre);
        } else {
            // Beside it, outside the shape.
            let left = mx < cx;
            if left {
                text_right(s, &label, mx - 1.0, my + 0.8, 2.0);
            } else {
                text(s, &label, mx + 1.0, my + 0.8, 2.0, Align::Left);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn armado() -> Model {
        let path =
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../out/armado.archisim");
        archisim_file::open(&path).unwrap_or_else(|_| {
            // A small one of our own: a column on its footing.
            let mut m = archisim_library::new_project("Despiece");
            let level = m.levels()[0].id;
            let ty = m.member_types().next().unwrap().id;
            let c = m
                .add_column(archisim_core::NewColumn {
                    member_type: ty,
                    level,
                    position: archisim_core::Point2::new(0.0, 0.0),
                    rotation: 0.0,
                    base_offset: 0.0,
                    top: archisim_core::WallTop::Unconnected,
                    height: 3.0,
                })
                .unwrap();
            m.set_reinforcement(
                c.0,
                Some(archisim_core::Reinforcement::default_for(RebarKind::Column)),
            )
            .unwrap();
            m
        })
    }

    #[test]
    fn every_bar_has_a_mark_and_the_steel_adds_up() {
        let m = armado();
        let marks = marks(&m);
        assert!(!marks.is_empty());
        // As many bars and as much steel as the elements lay.
        let mut bars = 0;
        let mut kg = 0.0;
        for (id, _) in m.reinforcements() {
            for g in element_bars(&m, id) {
                bars += g.bars.len();
                kg += g.weight();
            }
        }
        assert_eq!(marks.iter().map(|x| x.count).sum::<usize>(), bars);
        let total: f64 = by_diameter(&marks).iter().map(|d| d.2).sum();
        assert!((total - kg).abs() < 0.01 * kg.max(1.0), "{total} {kg}");
        // Marks numbered one after another; ties closed.
        assert!(marks.iter().enumerate().all(|(i, x)| x.number == i + 1));
        assert!(marks.iter().any(|x| x.closed));
        let sheets = bar_schedule_sheets(&m);
        assert!(crate::documents::pdf_of(&sheets, "Despiece")
            .unwrap()
            .starts_with(b"%PDF"));
        if let Ok(dir) = std::env::var("ARCHISIM_TEST_OUT") {
            let _ = std::fs::create_dir_all(&dir);
            for (i, (svg, _)) in sheets.iter().enumerate() {
                let _ = std::fs::write(format!("{dir}/despiece-{}.svg", i + 1), svg);
            }
        }
    }

    #[test]
    fn no_reinforcement_says_so() {
        let m = archisim_library::new_project("Vacío");
        let sheets = bar_schedule_sheets(&m);
        assert_eq!(sheets.len(), 1);
        assert!(marks(&m).is_empty());
    }
}
