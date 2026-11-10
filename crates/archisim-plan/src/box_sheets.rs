//! The workshop drawings of the modular boxes (M9, `docs/MODULAR.md`),
//! A3 landscape pages (SVG documents in millimetres):
//!
//! - M-00, assembly and crane: the boxes in plan with their order, the
//!   crane with the reach of its chart, and a table with each lift (order,
//!   box, stage, weight, reach, what the crane takes there);
//! - one page per box, in the order they go up: its chassis at 1:60 (the
//!   floor's and the ceiling's with the parapet in plan, the four sides
//!   in elevation), its cut list (the chassis' tubes and the light frame
//!   of what it holds), its materials, its weight and lifting, and its
//!   stage.
//!
//! - last, the work on site before the boxes come (M10): the supports in
//!   plan, measured from the first box's outer corner along the boxes'
//!   axes, with their level, pier, plate, bolts and load; the slab and the
//!   plinth.
//!
//! The chassis is drawn in the box's own axes (side A at the bottom of the
//! plan, as the box is laid out in the factory); the elevations are seen
//! from outside, each side named by its letter, with what is in its half
//! of the box.

use std::fmt::Write as _;

use archisim_core::{BoxId, Model, ModularBox, Point2, SIDE_NAMES};
use archisim_geom::boxes::{chassis_members, lift_points, ChassisMember, ChassisRole};
use archisim_struct::boxes::{assembly_order, chassis_cut_list, crane_plan, lift_check};

/// A chassis tube in elevation: its depth and its two ends on paper.
type Bar = (f64, (f64, f64), (f64, f64));

use crate::structure_sheets::{
    centred, line, page, rect, table, text, BLOCK_H, BLOCK_W, GREY, H, INK, MARGIN, MARK, W,
};

/// The scale of a box's sheet.
const BOX_SCALE: f64 = 60.0;
/// Fill of the cut tubes and of the parts nearest the eye.
const CUT: &str = "#3a3a3a";

fn dec(v: f64, d: usize) -> String {
    format!("{v:.d$}").replace('.', ",")
}

fn n(v: f64) -> String {
    crate::structure_sheets::n(v)
}

/// A filled or outlined rectangle from two corners.
fn bar(s: &mut String, a: (f64, f64), b: (f64, f64), fill: &str, sw: f64) {
    let (x0, x1) = (a.0.min(b.0), a.0.max(b.0));
    let (y0, y1) = (a.1.min(b.1), a.1.max(b.1));
    let _ = writeln!(
        s,
        r#"<rect x="{}" y="{}" width="{}" height="{}" fill="{fill}" stroke="{INK}" stroke-width="{}"/>"#,
        n(x0),
        n(y0),
        n((x1 - x0).max(0.05)),
        n((y1 - y0).max(0.05)),
        n(sw)
    );
}

/// A page's frame and title block.
fn frame(s: &mut String, model: &Model, code: &str, title: &str, scale: Option<f64>) {
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
    let mut t = title.to_string();
    while crate::lettering::text_width(&t, 3.0) > BLOCK_W - 40.0 && t.chars().count() > 4 {
        t.pop();
    }
    text(s, &t, x + 4.0, y + 15.0, 3.0, INK);
    text(
        s,
        &format!(
            "Taller de cajas{}",
            scale.map_or(String::new(), |k| format!(" · E 1:{k:.0}"))
        ),
        x + 4.0,
        y + 22.5,
        2.5,
        GREY,
    );
    centred(s, code, x + BLOCK_W - 15.0, y + 17.0, 6.0, INK);
}

/// The workshop sheets: the assembly and crane sheet, then one per box in
/// the order they go up. Empty without boxes.
pub fn pages(model: &Model) -> Vec<String> {
    let order = assembly_order(model);
    if order.is_empty() {
        return Vec::new();
    }
    let mut out = vec![assembly_page(model, &order)];
    for (i, id) in order.iter().enumerate() {
        if let Some(b) = model.modular_box(*id) {
            out.push(box_page(model, b, i + 1));
        }
    }
    if let Some(p) = base_page(model, out.len()) {
        out.push(p);
    }
    out
}

/// The codes of the pages, «M-00», «M-01»…
pub fn codes(count: usize) -> Vec<String> {
    (0..count).map(|i| format!("M-{i:02}")).collect()
}

fn assembly_page(model: &Model, order: &[BoxId]) -> String {
    let mut s = String::new();
    frame(&mut s, model, "M-00", "Montaje y plan de grúa", None);
    text(&mut s, "MONTAJE Y PLAN DE GRÚA", 20.0, 22.0, 4.0, INK);
    let plan = crane_plan(model);
    // The plan: the boxes (and the crane) fitted into the left part.
    let (x0, y0, w, h) = (20.0, 30.0, 245.0, 225.0);
    let mut pts: Vec<Point2> = order
        .iter()
        .filter_map(|id| model.modular_box(*id))
        .flat_map(|b| b.corners())
        .collect();
    if let Some(p) = &plan {
        pts.push(p.at);
    }
    let (lo, hi) = pts.iter().fold(
        (
            Point2::new(f64::MAX, f64::MAX),
            Point2::new(f64::MIN, f64::MIN),
        ),
        |(lo, hi), p| {
            (
                Point2::new(lo.x.min(p.x), lo.y.min(p.y)),
                Point2::new(hi.x.max(p.x), hi.y.max(p.y)),
            )
        },
    );
    let span = Point2::new((hi.x - lo.x).max(1.0) + 4.0, (hi.y - lo.y).max(1.0) + 4.0);
    let scale = [50.0, 100.0, 150.0, 200.0, 250.0, 500.0, 1000.0]
        .into_iter()
        .find(|k| span.x * 1000.0 / k <= w && span.y * 1000.0 / k <= h)
        .unwrap_or(1000.0);
    let k = 1000.0 / scale;
    let mid = (lo + hi) * 0.5;
    let at = |p: Point2| {
        (
            x0 + w / 2.0 + (p.x - mid.x) * k,
            y0 + h / 2.0 - (p.y - mid.y) * k,
        )
    };
    text(
        &mut s,
        &format!("Planta · E 1:{scale:.0} · a trazos, las cajas de las plantas altas"),
        x0,
        y0 + h + 5.0,
        2.5,
        GREY,
    );
    // What is drawn of the plan stays in its part of the sheet.
    let _ = writeln!(
        s,
        r#"<clipPath id="plan-area"><rect x="{}" y="{}" width="{}" height="{}"/></clipPath><g clip-path="url(#plan-area)">"#,
        n(x0 - 8.0),
        n(y0 - 6.0),
        n(w + 16.0),
        n(h + 6.0)
    );
    // The crane's reach: a circle at each radius of its chart that a box
    // needs, with what it takes there.
    if let Some(p) = &plan {
        let c = at(p.at);
        let chart = &model.modular_site().crane_chart;
        let far = p.lifts.iter().map(|l| l.radius).fold(0.0, f64::max);
        for (r, t) in chart.rows.iter().filter(|(r, _)| *r >= 5.0) {
            if *r > far + 6.0 {
                break;
            }
            let _ = writeln!(
                s,
                r#"<circle cx="{}" cy="{}" r="{}" fill="none" stroke="{GREY}" stroke-width="0.2" stroke-dasharray="2 1.5"/>"#,
                n(c.0),
                n(c.1),
                n(r * k)
            );
            text(
                &mut s,
                &format!("{} m · {} t", dec(*r, 0), dec(*t, 1)),
                c.0 + r * k * 0.707 + 0.8,
                c.1 - r * k * 0.707,
                2.0,
                GREY,
            );
        }
        // The crane: a ring with a cross.
        let _ = writeln!(
            s,
            r#"<circle cx="{}" cy="{}" r="3" fill="white" stroke="{MARK}" stroke-width="0.5"/>"#,
            n(c.0),
            n(c.1)
        );
        line(&mut s, (c.0 - 3.0, c.1), (c.0 + 3.0, c.1), 0.4, MARK);
        line(&mut s, (c.0, c.1 - 3.0), (c.0, c.1 + 3.0), 0.4, MARK);
        centred(
            &mut s,
            if p.suggested {
                "Grúa (propuesta)"
            } else {
                "Grúa"
            },
            c.0,
            c.1 + 7.0,
            2.5,
            MARK,
        );
    }
    // The boxes, their order and mark, and where they hang from; those of
    // an upper floor dashed, their tag lower down.
    let ground = order
        .iter()
        .filter_map(|id| model.modular_box(*id))
        .filter_map(|b| model.level(b.level).map(|l| l.elevation))
        .fold(f64::INFINITY, f64::min);
    for (i, id) in order.iter().enumerate() {
        let Some(b) = model.modular_box(*id) else {
            continue;
        };
        let upper = model
            .level(b.level)
            .is_some_and(|l| l.elevation > ground + 0.5);
        let dash = if upper {
            r#" stroke-dasharray="2 1""#
        } else {
            ""
        };
        let corners: Vec<(f64, f64)> = b.corners().iter().map(|p| at(*p)).collect();
        let d: String = corners
            .iter()
            .enumerate()
            .map(|(j, p)| format!("{}{} {} ", if j == 0 { "M" } else { "L" }, n(p.0), n(p.1)))
            .collect();
        let [r, g, bl] = b.progress.stage.rgb();
        let _ = writeln!(
            s,
            r##"<path d="{d}Z" fill="#{r:02x}{g:02x}{bl:02x}" fill-opacity="0.08" stroke="{INK}" stroke-width="0.35"{dash}/>"##
        );
        let c = at(b.centre());
        let c = if upper { (c.0, c.1 + 9.0) } else { c };
        centred(&mut s, &format!("{}º", i + 1), c.0, c.1 - 0.5, 4.0, INK);
        centred(&mut s, &b.mark, c.0, c.1 + 4.0, 2.2, GREY);
        if let Some(p) = &plan {
            if let Some(l) = p.lifts.iter().find(|l| l.id == *id) {
                let g = at(l.at);
                let g = if upper { (g.0, g.1 + 9.0) } else { g };
                let _ = writeln!(
                    s,
                    r#"<circle cx="{}" cy="{}" r="0.8" fill="{}"/>"#,
                    n(g.0),
                    n(g.1),
                    if l.passes() { INK } else { MARK }
                );
            }
        }
    }
    let _ = writeln!(s, "</g>");
    // The table of lifts.
    let rows: Vec<Vec<String>> = plan
        .as_ref()
        .map(|p| {
            p.lifts
                .iter()
                .map(|l| {
                    let b = model.modular_box(l.id).expect("box");
                    vec![
                        format!("{}º", l.order),
                        b.mark.clone(),
                        b.progress.stage.short().to_string(),
                        dec(l.load, 2),
                        dec(l.radius, 1),
                        dec(l.capacity, 1),
                        format!(
                            "{} %{}",
                            (l.usage() * 100.0).min(999.0).round(),
                            if l.passes() { "" } else { " ✗" }
                        ),
                    ]
                })
                .collect()
        })
        .unwrap_or_default();
    let tx = 275.0;
    text(&mut s, "IZADAS, POR ORDEN", tx, 34.0, 3.0, INK);
    let rh = 5.5_f64.min(170.0 / (rows.len() as f64 + 1.0)).max(3.2);
    let th = table(
        &mut s,
        tx,
        37.0,
        &[10.0, 20.0, 18.0, 17.0, 17.0, 18.0, 25.0],
        &[
            "Nº", "Caja", "Estado", "Carga t", "Radio m", "Admite t", "Uso",
        ],
        &rows,
        rh,
    );
    let mut y = 37.0 + th + 6.0;
    if let Some(p) = &plan {
        let site = model.modular_site();
        for l in [
            p.chart.clone(),
            if p.suggested {
                "Grúa sin colocar: propuesta en el centro de las cajas.".to_string()
            } else {
                format!("Grúa en ({}, {}).", dec(p.at.x, 2), dec(p.at.y, 2))
            },
            format!(
                "Carga: caja terminada y {} t de balancín y eslingas.",
                dec(site.rigging, 2)
            ),
            "Radio: del centro de la grúa al centro de gravedad de la caja.".to_string(),
        ] {
            text(&mut s, &l, tx, y, 2.2, GREY);
            y += 4.0;
        }
        if let Some(worst) = p.worst() {
            let b = model.modular_box(worst.id).expect("box");
            text(
                &mut s,
                &format!(
                    "La más exigente: {} a {} m ({} %).",
                    b.mark,
                    dec(worst.radius, 1),
                    (worst.usage() * 100.0).min(999.0).round()
                ),
                tx,
                y,
                2.5,
                if worst.passes() { INK } else { MARK },
            );
        }
    }
    page(&s)
}

/// The setting out of the supports (M10): the last sheet.
fn base_page(model: &Model, code: usize) -> Option<String> {
    let check = archisim_struct::boxes::base_check(model)?;
    let ground = model.ground_boxes();
    let first = *ground.iter().min_by(|a, b| a.mark.cmp(&b.mark))?;
    let (u, v) = (first.along(), first.across());
    // The origin: the boxes' corner where both measures are least, so
    // that every one is positive.
    let along = |p: Point2| (p.x * u.x + p.y * u.y, p.x * v.x + p.y * v.y);
    let corners: Vec<Point2> = ground.iter().flat_map(|b| b.corners()).collect();
    let min_x = corners.iter().map(|p| along(*p).0).fold(f64::MAX, f64::min);
    let min_y = corners.iter().map(|p| along(*p).1).fold(f64::MAX, f64::min);
    let o = u * min_x + v * min_y;
    let o = Point2::new(o.x, o.y);
    let local = |p: Point2| {
        let d = p - o;
        (d.x * u.x + d.y * u.y, d.x * v.x + d.y * v.y)
    };
    let owner = ground
        .iter()
        .min_by(|a, b| {
            let (pa, pb) = (local(a.centre()), local(b.centre()));
            (pa.0 + pa.1).total_cmp(&(pb.0 + pb.1))
        })
        .map_or(first.mark.as_str(), |b| b.mark.as_str());
    let code = format!("M-{code:02}");
    let mut s = String::new();
    frame(
        &mut s,
        model,
        &code,
        "Obra previa: replanteo de apoyos",
        None,
    );
    text(
        &mut s,
        "OBRA PREVIA: REPLANTEO DE APOYOS",
        20.0,
        22.0,
        4.0,
        INK,
    );
    let slabs: Vec<Vec<Point2>> = {
        let built: Vec<Vec<Point2>> = model
            .levels()
            .into_iter()
            .find(|l| l.name == archisim_core::BASE_LEVEL)
            .map(|lv| {
                model
                    .slabs()
                    .filter(|sl| sl.level == lv.id)
                    .map(|sl| sl.outline.clone())
                    .collect()
            })
            .unwrap_or_default();
        if built.is_empty() {
            model.ground_box_outlines(archisim_core::BaseOptions::default().margin)
        } else {
            built
        }
    };
    let pts: Vec<(f64, f64)> = slabs.iter().flatten().map(|p| local(*p)).collect();
    let (mut lo, mut hi) = ((f64::MAX, f64::MAX), (f64::MIN, f64::MIN));
    for p in &pts {
        lo = (lo.0.min(p.0), lo.1.min(p.1));
        hi = (hi.0.max(p.0), hi.1.max(p.1));
    }
    let (x0, y0, w, h) = (20.0, 32.0, 235.0, 215.0);
    let span = ((hi.0 - lo.0).max(1.0) + 3.0, (hi.1 - lo.1).max(1.0) + 3.0);
    let scale = [50.0, 75.0, 100.0, 150.0, 200.0, 250.0, 500.0]
        .into_iter()
        .find(|k| span.0 * 1000.0 / k <= w && span.1 * 1000.0 / k <= h)
        .unwrap_or(500.0);
    let k = 1000.0 / scale;
    let mid = ((lo.0 + hi.0) / 2.0, (lo.1 + hi.1) / 2.0);
    let at = |p: Point2| {
        let (x, y) = local(p);
        (
            x0 + w / 2.0 + (x - mid.0) * k,
            y0 + h / 2.0 - (y - mid.1) * k,
        )
    };
    let path = |pts: &[Point2]| -> String {
        let mut d: String = pts
            .iter()
            .enumerate()
            .map(|(j, p)| {
                let q = at(*p);
                format!("{}{} {} ", if j == 0 { "M" } else { "L" }, n(q.0), n(q.1))
            })
            .collect();
        d.push('Z');
        d
    };
    text(
        &mut s,
        &format!("Planta · E 1:{scale:.0} · losa, apoyos y, a trazos, las cajas"),
        x0,
        y0 + h + 5.0,
        2.5,
        GREY,
    );
    for sl in &slabs {
        let _ = writeln!(
            s,
            r##"<path d="{}" fill="#e6e6e6" stroke="{INK}" stroke-width="0.4"/>"##,
            path(sl)
        );
    }
    for b in &ground {
        let _ = writeln!(
            s,
            r#"<path d="{}" fill="none" stroke="{GREY}" stroke-width="0.25" stroke-dasharray="2 1"/>"#,
            path(&b.corners())
        );
        let c = at(b.centre());
        centred(&mut s, &b.mark, c.0, c.1, 2.5, GREY);
    }
    for sp in &check.supports {
        let c = at(sp.at);
        let half = (sp.pier * k / 2.0).max(0.8);
        let _ = writeln!(
            s,
            r#"<rect x="{}" y="{}" width="{}" height="{}" fill="{}" stroke="{INK}" stroke-width="0.3"/>"#,
            n(c.0 - half),
            n(c.1 - half),
            n(2.0 * half),
            n(2.0 * half),
            if sp.passes() { CUT } else { MARK }
        );
        text(
            &mut s,
            &sp.mark,
            c.0 + half + 0.6,
            c.1 - half - 0.4,
            2.2,
            INK,
        );
    }
    // The origin of the setting out, and its axes.
    let c = at(o);
    let _ = writeln!(
        s,
        r#"<circle cx="{}" cy="{}" r="1.2" fill="none" stroke="{MARK}" stroke-width="0.4"/>"#,
        n(c.0),
        n(c.1)
    );
    line(&mut s, c, (c.0 + 10.0, c.1), 0.35, MARK);
    line(&mut s, c, (c.0, c.1 - 10.0), 0.35, MARK);
    text(&mut s, "X", c.0 + 11.0, c.1 + 1.0, 2.2, MARK);
    text(&mut s, "Y", c.0 - 1.0, c.1 - 11.0, 2.2, MARK);
    // The table.
    let rows: Vec<Vec<String>> = check
        .supports
        .iter()
        .map(|sp| {
            let (x, y) = local(sp.at);
            // «MOD-06» → «06», to fit.
            let marks: Vec<&str> = sp
                .boxes
                .iter()
                .filter_map(|b| model.modular_box(*b).map(|b| b.mark.as_str()))
                .map(|m| m.rsplit('-').next().unwrap_or(m))
                .collect();
            vec![
                sp.mark.clone(),
                dec(x, 3),
                dec(y, 3),
                dec(sp.z, 3),
                format!("{:.0}×{:.0}", sp.pier * 100.0, sp.pier * 100.0),
                sp.plate.clone(),
                dec(sp.design, 0),
                format!(
                    "{}{}",
                    marks.join(" · "),
                    if sp.passes() { "" } else { " ✗" }
                ),
            ]
        })
        .collect();
    let tx = 262.0;
    text(&mut s, "APOYOS", tx, 34.0, 3.0, INK);
    let rh = 5.0_f64.min(185.0 / (rows.len() as f64 + 1.0)).max(2.6);
    let th = table(
        &mut s,
        tx,
        37.0,
        &[10.0, 15.0, 15.0, 15.0, 14.0, 25.0, 13.0, 34.0],
        &["", "X m", "Y m", "Cota", "Pilar", "Placa", "Nd kN", "Cajas"],
        &rows,
        rh,
    );
    let mut y = 37.0 + th + 5.0;
    let base_z = model
        .levels()
        .into_iter()
        .find(|l| l.name == archisim_core::BASE_LEVEL)
        .map(|l| l.elevation);
    let mut notes = vec![
        format!(
            "Origen: la esquina de las cajas junto a {owner}; X por el largo de las cajas, Y por su ancho."
        ),
        "Cajas: su número de marca (06 es MOD-06).".to_string(),
        "Cota: cara superior de la placa, la base del chasis.".to_string(),
        "Placas de acero S275 con pernos de anclaje que resisten también el levantamiento.".to_string(),
        "Calzos de nivelación y banda elastomérica bajo cada caja.".to_string(),
    ];
    if let Some(z) = base_z {
        notes.push(format!("Cara superior de la losa a {}.", dec(z, 3)));
    } else {
        notes.push("Aún sin construir: Propiedades de la caja → Obra previa.".to_string());
    }
    notes.push(format!(
        "Losa: {} m²; terreno a {} kN/m² de {} admisibles{}.",
        dec(check.area, 1),
        dec(check.pressure, 1),
        dec(check.bearing, 0),
        if check.ground_passes() { "" } else { " ✗" }
    ));
    for l in notes {
        text(&mut s, &l, tx, y, 2.1, GREY);
        y += 3.8;
    }
    Some(page(&s))
}

/// Where a point of the box's axes lands on a view: plan (x along, y
/// across) or one of the sides seen from outside (x along that side, the
/// depth from the eye).
fn side_view(b: &ModularBox, side: usize, p: [f64; 3]) -> (f64, f64) {
    let (l, w) = (b.length, b.width);
    match side {
        0 => (p[0], p[1]),
        1 => (p[1], l - p[0]),
        2 => (l - p[0], w - p[1]),
        _ => (w - p[1], p[0]),
    }
}

/// The chassis seen from outside a side, at `k` mm per metre, its bottom
/// left at (x0, y0 + height).
fn elevation(
    s: &mut String,
    b: &ModularBox,
    members: &[ChassisMember],
    side: usize,
    x0: f64,
    y0: f64,
    k: f64,
) {
    let top = b.bottom + b.height;
    // The half of the box on this side: what is beyond belongs to the
    // opposite side's elevation.
    let half = if side.is_multiple_of(2) {
        b.width
    } else {
        b.length
    } / 2.0;
    let mut v: Vec<Bar> = members
        .iter()
        .filter(|m| {
            let a = side_view(b, side, m.lo);
            let z = side_view(b, side, m.hi);
            a.1.min(z.1) < half
        })
        .map(|m| {
            let a = side_view(b, side, m.lo);
            let z = side_view(b, side, m.hi);
            let (h0, h1) = (a.0.min(z.0), a.0.max(z.0));
            let near = a.1.min(z.1);
            (
                near,
                (x0 + h0 * k, y0 + (top - m.hi[2]) * k),
                (x0 + h1 * k, y0 + (top - m.lo[2]) * k),
            )
        })
        .collect();
    // Far first, so the near ones hide them.
    v.sort_by(|a, b| b.0.total_cmp(&a.0));
    let thin = 0.12;
    for (depth, a, z) in v {
        let fill = if depth < 0.13 { "#d9d9d9" } else { "white" };
        bar(s, a, z, fill, thin);
    }
    let along = if side.is_multiple_of(2) {
        b.length
    } else {
        b.width
    };
    text(
        s,
        &format!("Lado {} · {} m", SIDE_NAMES[side], dec(along, 2)),
        x0,
        y0 - 2.0,
        2.5,
        INK,
    );
    text(
        s,
        b.sides[side].label(),
        x0,
        y0 + b.height * k + 4.0,
        2.0,
        GREY,
    );
}

/// The chassis in plan: the pieces `keep` takes, lowest first; the posts
/// cut, filled.
fn plan_view(
    s: &mut String,
    b: &ModularBox,
    members: &[ChassisMember],
    keep: impl Fn(&ChassisMember) -> bool,
    (x0, y0): (f64, f64),
    k: f64,
    title: &str,
) {
    let w = b.width;
    let mut v: Vec<&ChassisMember> = members.iter().filter(|m| keep(m)).collect();
    v.sort_by(|a, b| a.hi[2].total_cmp(&b.hi[2]));
    for m in v {
        let a = (x0 + m.lo[0] * k, y0 + (w - m.hi[1]) * k);
        let z = (x0 + m.hi[0] * k, y0 + (w - m.lo[1]) * k);
        let fill = if m.role.is_post() { CUT } else { "white" };
        bar(s, a, z, fill, 0.12);
    }
    text(s, title, x0, y0 - 2.0, 2.5, INK);
    // Its sides' letters and the overall sizes.
    centred(s, "A", x0 + b.length * k / 2.0, y0 + w * k + 3.5, 2.2, GREY);
    centred(s, "C", x0 + b.length * k / 2.0, y0 - 5.5, 2.2, GREY);
    text(s, "D", x0 - 3.5, y0 + w * k / 2.0 + 1.0, 2.2, GREY);
    text(
        s,
        "B",
        x0 + b.length * k + 1.5,
        y0 + w * k / 2.0 + 1.0,
        2.2,
        GREY,
    );
    let yd = y0 + w * k + 7.5;
    line(s, (x0, yd), (x0 + b.length * k, yd), 0.15, INK);
    for x in [x0, x0 + b.length * k] {
        line(s, (x, yd - 1.0), (x, yd + 1.0), 0.15, INK);
    }
    centred(
        s,
        &dec(b.length, 2),
        x0 + b.length * k / 2.0,
        yd - 0.8,
        2.0,
        INK,
    );
}

fn box_page(model: &Model, b: &ModularBox, order: usize) -> String {
    let mut s = String::new();
    let code = format!("M-{order:02}");
    let title = if b.kind.is_empty() {
        b.mark.clone()
    } else {
        format!("{} · {}", b.mark, b.kind)
    };
    frame(&mut s, model, &code, &title, Some(BOX_SCALE));
    text(&mut s, &title.to_uppercase(), 20.0, 21.0, 4.0, INK);
    text(
        &mut s,
        &format!("{}º en montar · {}", order, b.progress.label()),
        20.0,
        26.5,
        2.5,
        GREY,
    );
    let k = 1000.0 / BOX_SCALE;
    let members = chassis_members(b);
    let roof = b.chassis.roof_at - 0.01;
    // Row 1: the floor's chassis and the ceiling's with the parapet.
    let y1 = 38.0;
    plan_view(
        &mut s,
        b,
        &members,
        |m| m.lo[2] < roof && !matches!(m.role, ChassisRole::ParapetPost),
        (20.0, y1),
        k,
        "Chasis de suelo",
    );
    let x2 = 20.0 + b.length * k + 18.0;
    plan_view(
        &mut s,
        b,
        &members,
        |m| m.hi[2] > roof || m.role.is_post(),
        (x2, y1),
        k,
        "Chasis de techo y peto",
    );
    // Where it hangs from, on the floor's plan.
    for p in lift_points(b) {
        let c = (20.0 + p[0] * k, y1 + (b.width - p[1]) * k);
        let _ = writeln!(
            s,
            r#"<circle cx="{}" cy="{}" r="1.3" fill="white" stroke="{MARK}" stroke-width="0.35"/>"#,
            n(c.0),
            n(c.1)
        );
    }
    // Row 2: the four sides.
    let y2 = y1 + b.width * k + 22.0;
    let mut x = 20.0;
    for side in 0..4 {
        elevation(&mut s, b, &members, side, x, y2, k);
        x += if side % 2 == 0 { b.length } else { b.width } * k + 8.0;
    }
    // Row 3: the cut list and the materials.
    let y3 = y2 + b.height * k + 14.0;
    let mut rows: Vec<Vec<String>> = chassis_cut_list(b)
        .into_iter()
        .map(|c| {
            vec![
                c.role.label().to_string(),
                c.tube.label(),
                c.count.to_string(),
                c.length_mm.to_string(),
                dec(c.kg, 1),
            ]
        })
        .collect();
    let frames = archisim_geom::framing::members_by_box(model);
    if let Some(m) = frames.get(&b.id) {
        let list = archisim_geom::framing::cut_list(model, m);
        for t in &list.totals {
            rows.push(vec![
                "Entramado".to_string(),
                t.section.clone(),
                t.pieces.to_string(),
                format!("{} m", dec(t.metres, 1)),
                dec(t.kg, 1),
            ]);
        }
    }
    text(&mut s, "DESPIECE", 20.0, y3 - 2.0, 3.0, INK);
    let rh = (H - MARGIN - 6.0 - y3) / (rows.len() as f64 + 1.0);
    table(
        &mut s,
        20.0,
        y3,
        &[42.0, 34.0, 12.0, 20.0, 18.0],
        &["Pieza", "Perfil", "Nº", "Largo mm", "kg"],
        &rows,
        rh.clamp(2.6, 5.0),
    );
    // Materials and weight on the right.
    let mx = 160.0;
    let mats = materials(model, b.id);
    text(&mut s, "MATERIALES", mx, y3 - 2.0, 3.0, INK);
    let rh = ((H - MARGIN - BLOCK_H - 6.0 - y3) / (mats.len() as f64 + 1.0)).clamp(2.6, 5.0);
    table(
        &mut s,
        mx,
        y3,
        &[22.0, 58.0, 20.0],
        &["Elemento", "Tipo", "Cantidad"],
        &mats,
        rh,
    );
    let wx = 270.0;
    text(&mut s, "PESO E IZADO", wx, y3 - 2.0, 3.0, INK);
    let lines: Vec<Vec<String>> = match lift_check(model, b.id) {
        None => vec![vec!["Peso".into(), "vacía".into()]],
        Some(c) => {
            let w = &c.weight;
            let chassis: f64 = w
                .parts
                .iter()
                .filter(|p| p.what == "Chasis")
                .map(|p| p.kg)
                .sum();
            vec![
                vec!["Peso".into(), format!("{} t", dec(w.tonnes(), 2))],
                vec!["Chasis".into(), format!("{} kg", dec(chassis, 0))],
                vec![
                    "C. de gravedad".into(),
                    format!(
                        "{} · {} · {} m",
                        dec(w.centre[0], 2),
                        dec(w.centre[1], 2),
                        dec(w.centre[2], 2)
                    ),
                ],
                vec!["Izado".into(), b.chassis.lift.label().to_string()],
                vec![
                    "Por punto".into(),
                    format!(
                        "{} kN",
                        c.points
                            .iter()
                            .map(|(_, f)| dec(*f, 1))
                            .collect::<Vec<_>>()
                            .join(" · ")
                    ),
                ],
                vec![
                    "Borde".into(),
                    format!(
                        "{} %{}",
                        (c.usage() * 100.0).round(),
                        if c.passes() { "" } else { " ✗" }
                    ),
                ],
            ]
        }
    };
    table(&mut s, wx, y3, &[28.0, 102.0], &["", ""], &lines, 4.4);
    page(&s)
}

/// What a box is made of: its walls, floors, ceilings and roofs by type
/// with their area, and its doors and windows by type with how many.
fn materials(model: &Model, id: BoxId) -> Vec<Vec<String>> {
    let held = model.box_members(id);
    let mut rows: Vec<(String, String, f64, &str)> = Vec::new();
    let mut add = |what: &str, name: String, amount: f64, unit: &'static str| match rows
        .iter_mut()
        .find(|r| r.0 == what && r.1 == name)
    {
        Some(r) => r.2 += amount,
        None => rows.push((what.to_string(), name, amount, unit)),
    };
    for w in held.walls.iter().filter_map(|w| model.wall(*w)) {
        let holes: f64 = archisim_geom::openings_in_wall(model, w.id)
            .iter()
            .map(|o| (o.t1 - o.t0).max(0.0) * (o.head - o.sill).max(0.0))
            .sum();
        let name = model
            .wall_type(w.wall_type)
            .map_or_else(String::new, |t| t.name.clone());
        add("Muro", name, (w.length() * w.height - holes).max(0.0), "m²");
    }
    for sl in held.slabs.iter().filter_map(|s| model.slab(*s)) {
        let t = model.slab_type(sl.slab_type);
        let what = match t.map(|t| t.function) {
            Some(archisim_core::SlabFunction::Roof) => "Cubierta",
            _ => "Suelo",
        };
        add(
            what,
            t.map_or_else(String::new, |t| t.name.clone()),
            archisim_core::signed_area(&sl.outline).abs(),
            "m²",
        );
    }
    for c in held.ceilings.iter().filter_map(|c| model.ceiling(*c)) {
        add(
            "Techo",
            model
                .slab_type(c.ceiling_type)
                .map_or_else(String::new, |t| t.name.clone()),
            archisim_core::signed_area(&c.outline).abs(),
            "m²",
        );
    }
    for o in held.openings.iter().filter_map(|o| model.opening(*o)) {
        let name = model
            .opening_type(o.opening_type)
            .map_or_else(String::new, |t| {
                if t.mark.is_empty() {
                    t.name.clone()
                } else {
                    format!("{} {}", t.mark, t.name)
                }
            });
        add("Carpintería", name, 1.0, "ud");
    }
    rows.into_iter()
        .map(|(what, name, amount, unit)| {
            vec![
                what,
                name,
                if unit == "ud" {
                    format!("{amount:.0} ud")
                } else {
                    format!("{} {unit}", dec(amount, 2))
                },
            ]
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use archisim_core::{NewBox, SideState};

    #[test]
    fn a_sheet_for_the_assembly_and_one_per_box() {
        let mut m = Model::new("Casa");
        let lv = m.add_level("PB", 0.0);
        let a = m
            .add_box(NewBox::usual(lv, Point2::new(0.0, 0.0), 0.0))
            .unwrap();
        let b = m
            .add_box(NewBox::usual(lv, Point2::new(0.0, 3.62), 0.0))
            .unwrap();
        m.edit_box(a, |x| x.sides[2] = SideState::Wall).unwrap();
        m.edit_box(b, |x| {
            x.sides[0] = SideState::Open;
            x.kind = "Caja 6,20 · estar".into();
        })
        .unwrap();
        let p = pages(&m);
        // Assembly, one per box and the setting out of the supports.
        assert_eq!(p.len(), 4);
        assert_eq!(codes(p.len()), vec!["M-00", "M-01", "M-02", "M-03"]);
        // Nine supports, each a square on the plan.
        let piers = |page: &str| page.matches(&format!(r#"fill="{CUT}""#)).count();
        assert_eq!(piers(&p[3]), 9);
        // Built, the same supports with the slab that was made.
        m.build_box_base(&archisim_core::BaseOptions::default())
            .unwrap();
        let built = pages(&m);
        assert_eq!(piers(&built[3]), 9);
        assert_ne!(built[3], p[3]);
        assert!(p
            .iter()
            .all(|x| x.starts_with("<svg") && x.ends_with("</svg>")));
        // The boxes have no fill, so they weigh only their chassis: the
        // crane is placed in the middle and takes them.
        assert!(p[0].contains("<circle"));
        // A box's sheet: its four sides and its tubes.
        assert!(p[2].matches("<rect").count() > 100);
        assert!(pages(&Model::new("Vacío")).is_empty());
    }
}
