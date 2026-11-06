//! The structure drawings (planos de estructura), A3 landscape pages (SVG
//! documents in millimetres) made from the model and its calculation:
//!
//! - E-01, the materials table of the Código Estructural (cuadro de
//!   características) and the columns table (cuadro de pilares): each
//!   column stack by its mark, level by level, with its section and bars;
//! - E-02, the foundations: the lowest level's plan with the marks of
//!   columns and footings, the setting out of the columns (replanteo) and
//!   the footings table (cuadro de zapatas);
//! - one page per floor: its plan with the marks of columns and beams,
//!   the one-way floors with their joists and the top bars over each
//!   support (negativos), and the heads of the waffle floors;
//! - the beams: each one in elevation with its bars and stirrups.
//!
//! Where an element has no reinforcement drawn, the one the calculation
//! asks for is shown, marked with an asterisk.

use std::collections::BTreeMap;
use std::fmt::Write as _;

use archisim_core::{
    ColumnId, ElementDiscipline, FootingKind, LevelId, Model, Point2, Profile, Reinforcement,
};
use archisim_struct::proposals::Proposal;

use crate::lettering::{paint, text_path, text_width, Align};
use crate::{render_plan, PlanOptions};

pub(crate) const W: f64 = 420.0;
pub(crate) const H: f64 = 297.0;
pub(crate) const MARGIN: f64 = 10.0;
pub(crate) const INK: &str = "#1a1a1a";
pub(crate) const GREY: &str = "#6b6b6b";
pub(crate) const MARK: &str = "#b3261e";
pub(crate) const RULE: &str = "#b9b9b9";
/// The title block at the bottom right, mm.
pub(crate) const BLOCK_W: f64 = 150.0;
pub(crate) const BLOCK_H: f64 = 28.0;

pub(crate) fn n(v: f64) -> String {
    let v = (v * 1000.0).round() / 1000.0;
    format!("{v}")
}

fn dec(v: f64, d: usize) -> String {
    format!("{v:.d$}").replace('.', ",")
}

pub(crate) fn text(s: &mut String, t: &str, x: f64, y: f64, h: f64, colour: &str) {
    if t.trim().is_empty() {
        return;
    }
    let _ = writeln!(
        s,
        r#"<path d="{}" {}/>"#,
        text_path(t, Point2::new(x, -y), Point2::new(1.0, 0.0), h, Align::Left),
        paint(h, colour)
    );
}

pub(crate) fn centred(s: &mut String, t: &str, x: f64, y: f64, h: f64, colour: &str) {
    if t.trim().is_empty() {
        return;
    }
    let _ = writeln!(
        s,
        r#"<path d="{}" {}/>"#,
        text_path(
            t,
            Point2::new(x, -y),
            Point2::new(1.0, 0.0),
            h,
            Align::Centre
        ),
        paint(h, colour)
    );
}

pub(crate) fn line(s: &mut String, a: (f64, f64), b: (f64, f64), w: f64, colour: &str) {
    let _ = writeln!(
        s,
        r#"<line x1="{}" y1="{}" x2="{}" y2="{}" stroke="{colour}" stroke-width="{}" stroke-linecap="round"/>"#,
        n(a.0),
        n(a.1),
        n(b.0),
        n(b.1),
        n(w)
    );
}

pub(crate) fn rect(s: &mut String, x: f64, y: f64, w: f64, h: f64, sw: f64, colour: &str) {
    let _ = writeln!(
        s,
        r#"<rect x="{}" y="{}" width="{}" height="{}" fill="none" stroke="{colour}" stroke-width="{}"/>"#,
        n(x),
        n(y),
        n(w),
        n(h),
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
    text(s, title, x + 4.0, y + 15.0, 3.0, INK);
    text(
        s,
        &format!(
            "Estructura{}",
            scale.map_or(String::new(), |k| format!(" · E 1:{k:.0}"))
        ),
        x + 4.0,
        y + 22.5,
        2.5,
        GREY,
    );
    centred(s, code, x + BLOCK_W - 15.0, y + 17.0, 6.0, INK);
}

pub(crate) fn page(body: &str) -> String {
    format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="{w}mm" height="{h}mm" viewBox="0 0 {w} {h}"><rect width="{w}" height="{h}" fill="white"/>
{body}</svg>"#,
        w = n(W),
        h = n(H)
    )
}

/// A table: columns' widths, a header and rows; drawn from (x, y), each
/// row `rh` high. Returns its height.
pub(crate) fn table(
    s: &mut String,
    x: f64,
    y: f64,
    widths: &[f64],
    header: &[&str],
    rows: &[Vec<String>],
    rh: f64,
) -> f64 {
    let total: f64 = widths.iter().sum();
    let h = rh * (rows.len() + 1) as f64;
    rect(s, x, y, total, h, 0.3, INK);
    line(s, (x, y + rh), (x + total, y + rh), 0.3, INK);
    for r in 1..rows.len() {
        let yy = y + rh * (r + 1) as f64;
        line(s, (x, yy), (x + total, yy), 0.1, RULE);
    }
    let mut cx = x;
    for (k, w) in widths.iter().enumerate() {
        if k > 0 {
            line(s, (cx, y), (cx, y + h), 0.15, RULE);
        }
        let th = (rh * 0.42).min(2.6);
        if let Some(t) = header.get(k) {
            text(s, t, cx + 1.5, y + rh * 0.7, th, INK);
        }
        for (r, row) in rows.iter().enumerate() {
            if let Some(t) = row.get(k) {
                let lines: Vec<&str> = t.split('\n').collect();
                let lh = rh / (lines.len() as f64 + 0.4);
                let th = th.min(lh * 0.75);
                for (i, l) in lines.iter().enumerate() {
                    let mut l = l.to_string();
                    while text_width(&l, th) > w - 3.0 && l.chars().count() > 3 {
                        l.pop();
                    }
                    let yy = y + rh * (r as f64 + 1.0) + lh * (i as f64 + 1.0);
                    text(s, &l, cx + 1.5, yy, th, INK);
                }
            }
        }
        cx += w;
    }
    h
}

/// The marks of the structure: columns by their place (a stack through
/// the levels is one mark), beams and footings, numbered from the top
/// left of the plan.
pub struct Marks {
    pub columns: BTreeMap<ColumnId, String>,
    pub beams: BTreeMap<u64, String>,
    pub footings: BTreeMap<u64, String>,
    /// Each column mark's place and its columns from the bottom up.
    pub stacks: Vec<(String, Point2, Vec<ColumnId>)>,
}

fn reading_order(a: Point2, b: Point2) -> std::cmp::Ordering {
    let ra = (-a.y * 2.0).round();
    let rb = (-b.y * 2.0).round();
    ra.total_cmp(&rb).then(a.x.total_cmp(&b.x))
}

pub fn marks(model: &Model) -> Marks {
    let elevation = |l: LevelId| model.level(l).map_or(0.0, |l| l.elevation);
    let mut stacks: Vec<(Point2, Vec<ColumnId>)> = Vec::new();
    for c in model.columns() {
        match stacks
            .iter_mut()
            .find(|(p, _)| p.distance(c.position) < 0.05)
        {
            Some((_, ids)) => ids.push(c.id),
            None => stacks.push((c.position, vec![c.id])),
        }
    }
    stacks.sort_by(|a, b| reading_order(a.0, b.0));
    let mut columns = BTreeMap::new();
    let mut out_stacks = Vec::new();
    for (k, (p, mut ids)) in stacks.into_iter().enumerate() {
        let mark = format!("P{}", k + 1);
        ids.sort_by(|a, b| {
            let za = model
                .column(*a)
                .map_or(0.0, |c| elevation(c.level) + c.base_offset);
            let zb = model
                .column(*b)
                .map_or(0.0, |c| elevation(c.level) + c.base_offset);
            za.total_cmp(&zb)
        });
        for id in &ids {
            columns.insert(*id, mark.clone());
        }
        out_stacks.push((mark, p, ids));
    }
    let mut beams: Vec<_> = model.beams().collect();
    beams.sort_by(|a, b| {
        elevation(a.level)
            .total_cmp(&elevation(b.level))
            .then(reading_order(a.start, b.start))
    });
    let beams = beams
        .iter()
        .enumerate()
        .map(|(k, b)| (b.id.0, format!("V{}", k + 1)))
        .collect();
    let mut footings: Vec<_> = model
        .footings()
        .filter(|f| !matches!(f.kind, FootingKind::TieBeam { .. }))
        .collect();
    let centre = |f: &archisim_core::Footing| {
        let o = f.outline();
        let k = o.len().max(1) as f64;
        Point2::new(
            o.iter().map(|p| p.x).sum::<f64>() / k,
            o.iter().map(|p| p.y).sum::<f64>() / k,
        )
    };
    footings.sort_by(|a, b| reading_order(centre(a), centre(b)));
    let mut fm: BTreeMap<u64, String> = footings
        .iter()
        .enumerate()
        .map(|(k, f)| (f.id.0, format!("Z{}", k + 1)))
        .collect();
    let mut ties: Vec<_> = model
        .footings()
        .filter(|f| matches!(f.kind, FootingKind::TieBeam { .. }))
        .collect();
    ties.sort_by(|a, b| reading_order(centre(a), centre(b)));
    for (k, t) in ties.iter().enumerate() {
        fm.insert(t.id.0, format!("VA{}", k + 1));
    }
    Marks {
        columns,
        beams,
        footings: fm,
        stacks: out_stacks,
    }
}

/// The reinforcement of an element: drawn, or the one proposed (then
/// `true`).
fn reinforcement_of(
    model: &Model,
    proposals: &[Proposal],
    id: u64,
) -> Option<(Reinforcement, bool)> {
    if let Some(r) = model.reinforcement(id) {
        return Some((*r, false));
    }
    proposals
        .iter()
        .find(|p| p.element == id)
        .map(|p| (p.reinforcement, true))
}

fn star(proposed: bool) -> &'static str {
    if proposed {
        " *"
    } else {
        ""
    }
}

/// Every page.
pub fn pages(model: &Model) -> Vec<String> {
    let model = &*archisim_struct::phases::as_built(model);
    let _font = crate::lettering::use_font(model.graphics.font);
    let an = archisim_struct::analysis::analyse(model);
    let proposals = archisim_struct::proposals::proposals(model, &an);
    let marks = marks(model);
    let mut out = Vec::new();
    let mut code = 1;
    let mut next = || {
        let c = format!("E-{code:02}");
        code += 1;
        c
    };
    out.push(page(&tables_page(model, &proposals, &marks, &next())));
    let levels = model.levels();
    // The foundations: the level with footings, or the lowest.
    let base = levels
        .iter()
        .find(|l| model.footings_on_level(l.id).next().is_some())
        .or(levels.first())
        .map(|l| l.id);
    if let Some(base) = base {
        out.push(page(&foundations_page(
            model,
            &proposals,
            &marks,
            base,
            &next(),
        )));
    }
    let floors = archisim_struct::floors::analyse(model, &an.takedown);
    let waffles = archisim_struct::slab::analyse(model, &an.takedown);
    for l in &levels {
        let has = model.slabs_on_level(l.id).next().is_some()
            || model.beams_on_level(l.id).next().is_some();
        if !has {
            continue;
        }
        out.push(page(&floor_page(
            model,
            &marks,
            l.id,
            &floors,
            &waffles,
            &next(),
        )));
    }
    for body in beams_pages(model, &proposals, &marks, &mut next) {
        out.push(page(&body));
    }
    out
}

/// E-01: the materials and the columns.
fn tables_page(model: &Model, proposals: &[Proposal], marks: &Marks, code: &str) -> String {
    let mut s = String::new();
    frame(
        &mut s,
        model,
        code,
        "Cuadro de características y cuadro de pilares",
        None,
    );
    let rc = archisim_struct::concrete::Rc::of(model);
    let mat = &model.structural_site().materials;
    let x0 = MARGIN + 8.0;
    let mut y = MARGIN + 12.0;
    text(
        &mut s,
        "CUADRO DE CARACTERÍSTICAS (Código Estructural)",
        x0,
        y,
        3.5,
        INK,
    );
    y += 5.0;
    let rows = vec![
        vec![
            "Hormigón".into(),
            format!("HA-{:.0}/B/20/IIa", rc.fck),
            "Estadístico".into(),
            "γc = 1,50".into(),
            "Cimentación: recubrimiento 5 cm (con hormigón de limpieza)".into(),
        ],
        vec![
            "Acero en barras".into(),
            format!("B{:.0}S", rc.fyk),
            "Normal".into(),
            "γs = 1,15".into(),
            "Pilares y vigas: 3 cm; forjados: 2,5 cm".into(),
        ],
        vec![
            "Acero laminado".into(),
            format!("S{:.0}", mat.steel_fy),
            "—".into(),
            "γM0 = 1,05".into(),
            if mat.steel_rigid {
                "Uniones rígidas (chapa de testa)".into()
            } else {
                "Uniones articuladas".into()
            },
        ],
        vec![
            "Madera".into(),
            mat.timber.code().into(),
            "—".into(),
            "γM = 1,30".into(),
            String::new(),
        ],
        vec![
            "Ejecución".into(),
            "Control normal".into(),
            String::new(),
            "γG = 1,35 · γQ = 1,50".into(),
            "Acciones: CTE DB-SE-AE; sismo: NCSE-02".into(),
        ],
    ];
    y += table(
        &mut s,
        x0,
        y,
        &[34.0, 34.0, 26.0, 40.0, 110.0],
        &["Material", "Tipo", "Control", "Coeficiente", "Notas"],
        &rows,
        6.0,
    );
    // The ground and the earthquake.
    let site = model.structural_site();
    y += 5.0;
    text(
        &mut s,
        &format!(
            "Terreno: tensión admisible {} kN/m² (según estudio geotécnico) · Sismo: ab/g {} · K {} · C {}",
            dec(site.bearing, 0),
            dec(site.seismic_ab, 2),
            dec(site.seismic_k, 2),
            dec(site.soil, 2)
        ),
        x0,
        y,
        2.5,
        GREY,
    );
    y += 10.0;
    // The columns table: a column per mark, a row per level.
    text(&mut s, "CUADRO DE PILARES", x0, y, 3.5, INK);
    y += 5.0;
    let levels = model.levels();
    let per = 10usize;
    let label_w = 34.0;
    let cell_w = ((W - 2.0 * MARGIN - 16.0 - label_w) / per as f64).min(36.0);
    let cell_w = if marks.stacks.len() < per {
        ((W - 2.0 * MARGIN - 16.0 - label_w) / marks.stacks.len().max(1) as f64).min(48.0)
    } else {
        cell_w
    };
    for chunk in marks.stacks.chunks(per) {
        let mut widths = vec![label_w];
        widths.extend(std::iter::repeat_n(cell_w, chunk.len()));
        let mut header: Vec<String> = vec!["Planta".into()];
        header.extend(chunk.iter().map(|(m, _, _)| m.clone()));
        let header: Vec<&str> = header.iter().map(String::as_str).collect();
        let mut rows = Vec::new();
        for l in levels.iter().rev() {
            let mut row = vec![l.name.clone()];
            let mut any = false;
            for (_, _, ids) in chunk {
                let cell = ids
                    .iter()
                    .filter_map(|id| model.column(*id))
                    .find(|c| c.level == l.id)
                    .map(|c| {
                        any = true;
                        let t = model.member_type(c.member_type);
                        let size = t.map_or(String::new(), |t| match t.profile {
                            Profile::Rectangle { width, depth } => {
                                format!("{:.0}×{:.0}", width * 100.0, depth * 100.0)
                            }
                            Profile::Circle { diameter } => format!("Ø{:.0}", diameter * 100.0),
                            _ => t.name.clone(),
                        });
                        match reinforcement_of(model, proposals, c.id.0) {
                            Some((Reinforcement::Column { bars, ties, .. }, p)) => format!(
                                "{size}{}\n{}\ncercos {}",
                                star(p),
                                bars.label(),
                                ties.label()
                            ),
                            _ => size,
                        }
                    })
                    .unwrap_or_default();
                row.push(cell);
            }
            if any {
                rows.push(row);
            }
        }
        if y + 13.0 * (rows.len() + 1) as f64 > H - MARGIN - BLOCK_H - 8.0 {
            text(
                &mut s,
                "(sigue en el anejo de cálculo)",
                x0,
                y + 4.0,
                2.5,
                GREY,
            );
            break;
        }
        y += table(&mut s, x0, y, &widths, &header, &rows, 13.0) + 5.0;
    }
    text(
        &mut s,
        "* armado propuesto por el cálculo (no dibujado en el modelo). Cercos de las zonas críticas, según la NCSE-02, en el anejo.",
        x0,
        H - MARGIN - BLOCK_H - 3.0,
        2.3,
        GREY,
    );
    s
}

/// A plan of a level, structure only, fitted into a box of the page: its
/// SVG fragment, the scale and how model points go to the page.
fn plan_in(
    model: &Model,
    level: LevelId,
    x: f64,
    y: f64,
    w: f64,
    h: f64,
) -> (String, f64, impl Fn(Point2) -> (f64, f64)) {
    let style = archisim_core::ViewStyle {
        discipline: Some(ElementDiscipline::Structure),
        ..Default::default()
    };
    // The extent: what the plan draws at 1:100.
    let probe = render_plan(
        model,
        level,
        &PlanOptions {
            scale: 100.0,
            margin_mm: 5.0,
            title: false,
            style: style.clone(),
            ..PlanOptions::default()
        },
    );
    let vb = view_box(&probe).unwrap_or((0.0, -10.0, 10.0, 10.0));
    let scale = [
        50.0, 75.0, 100.0, 125.0, 150.0, 200.0, 250.0, 300.0, 400.0, 500.0,
    ]
    .into_iter()
    .find(|k| vb.2 * 1000.0 / k <= w && vb.3 * 1000.0 / k <= h)
    .unwrap_or(500.0);
    let svg = render_plan(
        model,
        level,
        &PlanOptions {
            scale,
            margin_mm: 5.0,
            title: false,
            style,
            ..PlanOptions::default()
        },
    );
    let vb = view_box(&svg).unwrap_or(vb);
    let (pw, ph) = (vb.2 * 1000.0 / scale, vb.3 * 1000.0 / scale);
    let (px, py) = (x + (w - pw) / 2.0, y + (h - ph) / 2.0);
    let body_start = svg.find('>').map_or(0, |i| i + 1);
    let body_end = svg.rfind("</svg>").unwrap_or(svg.len());
    let frag = format!(
        r#"<svg x="{}" y="{}" width="{}" height="{}" viewBox="{} {} {} {}" overflow="hidden" preserveAspectRatio="none">{}</svg>"#,
        n(px),
        n(py),
        n(pw),
        n(ph),
        n(vb.0),
        n(vb.1),
        n(vb.2),
        n(vb.3),
        &svg[body_start..body_end]
    );
    let k = 1000.0 / scale;
    let to_page = move |p: Point2| (px + (p.x - vb.0) * k, py + (-p.y - vb.1) * k);
    (frag, scale, to_page)
}

fn view_box(svg: &str) -> Option<(f64, f64, f64, f64)> {
    let i = svg.find("viewBox=\"")? + 9;
    let j = svg[i..].find('"')? + i;
    let v: Vec<f64> = svg[i..j]
        .split_whitespace()
        .filter_map(|x| x.parse().ok())
        .collect();
    (v.len() == 4).then(|| (v[0], v[1], v[2], v[3]))
}

/// A mark in a circle at a point of the page.
fn bubble(s: &mut String, t: &str, at: (f64, f64), colour: &str) {
    let r = 1.2 + 0.9 * t.chars().count() as f64;
    let _ = writeln!(
        s,
        r#"<circle cx="{}" cy="{}" r="{}" fill="white" stroke="{colour}" stroke-width="0.25"/>"#,
        n(at.0),
        n(at.1),
        n(r)
    );
    centred(s, t, at.0, at.1 + 0.9, 2.2, colour);
}

/// E-02: the foundations, the setting out of the columns and the
/// footings table.
fn foundations_page(
    model: &Model,
    proposals: &[Proposal],
    marks: &Marks,
    level: LevelId,
    code: &str,
) -> String {
    let mut s = String::new();
    let plan_w = 260.0;
    let (frag, scale, at) = plan_in(
        model,
        level,
        MARGIN + 5.0,
        MARGIN + 5.0,
        plan_w,
        H - 2.0 * MARGIN - 10.0,
    );
    s.push_str(&frag);
    frame(
        &mut s,
        model,
        code,
        "Cimentación y replanteo de pilares",
        Some(scale),
    );
    for f in model.footings_on_level(level) {
        let Some(m) = marks.footings.get(&f.id.0) else {
            continue;
        };
        let o = f.outline();
        let k = o.len().max(1) as f64;
        let c = Point2::new(
            o.iter().map(|p| p.x).sum::<f64>() / k,
            o.iter().map(|p| p.y).sum::<f64>() / k,
        );
        let (x, y) = at(c);
        bubble(&mut s, m, (x + 5.0, y + 5.0), MARK);
    }
    for (m, p, _) in &marks.stacks {
        let (x, y) = at(*p);
        bubble(&mut s, m, (x - 4.0, y - 4.0), INK);
    }
    // The setting out: from the first grid crossing, or the origin.
    let origin = grid_origin(model).unwrap_or(Point2::new(0.0, 0.0));
    let x0 = MARGIN + plan_w + 12.0;
    let mut y = MARGIN + 10.0;
    text(&mut s, "REPLANTEO DE PILARES", x0, y, 3.2, INK);
    y += 3.5;
    text(
        &mut s,
        &format!(
            "Desde el punto ({}; {}) m del proyecto",
            dec(origin.x, 2),
            dec(origin.y, 2)
        ),
        x0,
        y,
        2.2,
        GREY,
    );
    y += 2.5;
    let rows: Vec<Vec<String>> = marks
        .stacks
        .iter()
        .map(|(m, p, _)| vec![m.clone(), dec(p.x - origin.x, 3), dec(p.y - origin.y, 3)])
        .collect();
    let max_rows = 30;
    let shown: Vec<Vec<String>> = rows.iter().take(max_rows).cloned().collect();
    y += table(
        &mut s,
        x0,
        y,
        &[18.0, 28.0, 28.0],
        &["Pilar", "X (m)", "Y (m)"],
        &shown,
        5.0,
    ) + 7.0;
    // The footings table.
    text(&mut s, "CUADRO DE ZAPATAS", x0, y, 3.2, INK);
    y += 2.5;
    let mut rows = Vec::new();
    for f in model.footings_on_level(level) {
        let Some(m) = marks.footings.get(&f.id.0) else {
            continue;
        };
        let size = match f.kind {
            FootingKind::Pad { length, .. } => {
                format!("{}×{}×{}", dec(f.width, 2), dec(length, 2), dec(f.depth, 2))
            }
            FootingKind::Strip { start, end, .. } | FootingKind::TieBeam { start, end } => {
                format!(
                    "{} × {} m, L {}",
                    dec(f.width, 2),
                    dec(f.depth, 2),
                    dec(start.distance(end), 2)
                )
            }
        };
        let piles = model.footing_piles(f.id).map_or(String::new(), |p| {
            format!(" · {} pil. Ø{:.0}", p.count, p.diameter * 100.0)
        });
        let steel = match reinforcement_of(model, proposals, f.id.0) {
            Some((Reinforcement::Footing { bottom, .. }, p)) => {
                format!("{}{}", bottom[0].label(), star(p))
            }
            _ => "—".into(),
        };
        rows.push(vec![m.clone(), format!("{size}{piles}"), steel]);
    }
    let key = |r: &Vec<String>| {
        let m = &r[0];
        let alpha: String = m.chars().take_while(|c| c.is_alphabetic()).collect();
        let num: u32 = m[alpha.len()..].parse().unwrap_or(0);
        (alpha, num)
    };
    rows.sort_by_key(key);
    rows.truncate(26);
    table(
        &mut s,
        x0,
        y,
        &[16.0, 48.0, 26.0],
        &["", "a × b × h (m)", "Armado"],
        &rows,
        5.0,
    );
    s
}

/// Where the first two grid lines cross, if there are grid lines.
fn grid_origin(model: &Model) -> Option<Point2> {
    let g: Vec<_> = model.grids().collect();
    for a in &g {
        for b in &g {
            let d1 = a.end - a.start;
            let d2 = b.end - b.start;
            let den = d1.x * d2.y - d1.y * d2.x;
            if den.abs() < 1e-6 {
                continue;
            }
            let w = b.start - a.start;
            let t = (w.x * d2.y - w.y * d2.x) / den;
            return Some(a.start + d1 * t);
        }
    }
    None
}

/// A floor: its plan with the marks, the joists and the top bars.
fn floor_page(
    model: &Model,
    marks: &Marks,
    level: LevelId,
    floors: &[archisim_struct::floors::OneWayResult],
    slabs: &[archisim_struct::slab::SlabResult],
    code: &str,
) -> String {
    let mut s = String::new();
    let name = model.level(level).map_or(String::new(), |l| l.name.clone());
    let plan_w = W - 2.0 * MARGIN - 10.0;
    let (frag, scale, at) = plan_in(
        model,
        level,
        MARGIN + 5.0,
        MARGIN + 5.0,
        plan_w,
        H - 2.0 * MARGIN - BLOCK_H - 8.0,
    );
    s.push_str(&frag);
    frame(
        &mut s,
        model,
        code,
        &format!("Forjado de {name}"),
        Some(scale),
    );
    let k = 1000.0 / scale;
    // The columns that hold this floor: those of the level below that
    // reach it, and its own.
    for (m, p, ids) in &marks.stacks {
        if ids.iter().any(|id| model.column(*id).is_some()) {
            let (x, y) = at(*p);
            bubble(&mut s, m, (x - 4.0, y - 4.0), INK);
        }
    }
    for b in model.beams_on_level(level) {
        if let Some(m) = marks.beams.get(&b.id.0) {
            let mid = b.start + (b.end - b.start) * 0.3;
            let (x, y) = at(mid);
            bubble(&mut s, m, (x, y - 3.5), INK);
        }
    }
    // The top bars of the one-way floors: one line per support, the
    // worst of its bands, as long as they go each way.
    for r in floors {
        if model.slab(r.slab).is_none_or(|sl| sl.level != level) {
            continue;
        }
        let d = r.direction;
        let nrm = archisim_core::Vec2::new(-d.y, d.x);
        let mut groups: Vec<(f64, Vec<&archisim_struct::floors::Bearing>)> = Vec::new();
        for b in &r.bearings {
            let a = b.at.x * d.x + b.at.y * d.y;
            match groups.iter_mut().find(|g| (g.0 - a).abs() < 0.3) {
                Some(g) => g.1.push(b),
                None => groups.push((a, vec![b])),
            }
        }
        let rc = archisim_struct::concrete::Rc::of(model);
        for (_, list) in groups {
            let Some(worst) = list.iter().max_by(|a, b| a.hogging.total_cmp(&b.hogging)) else {
                continue;
            };
            let across: Vec<f64> = list
                .iter()
                .map(|b| b.at.x * nrm.x + b.at.y * nrm.y)
                .collect();
            let mid_across = (across.iter().copied().fold(f64::MAX, f64::min)
                + across.iter().copied().fold(f64::MIN, f64::max))
                / 2.0;
            let base = worst.at + nrm * (mid_across - (worst.at.x * nrm.x + worst.at.y * nrm.y));
            let len = (0.25 * worst.span + 0.1).max(0.6);
            let outline = model
                .slab(r.slab)
                .map(|sl| sl.outline.clone())
                .unwrap_or_default();
            let inside = |p: Point2| archisim_struct::geo::inside(p, &outline);
            let back = if inside(base - d * (len * 0.5)) {
                len
            } else {
                0.0
            };
            let ahead = if inside(base + d * (len * 0.5)) {
                len
            } else {
                0.0
            };
            let m_rib = worst.hogging * r.system.spacing;
            let bars = rc
                .bending_steel(m_rib, r.system.rib, r.system.depth - 0.03)
                .map(archisim_struct::floors::rib_bars)
                .map_or("macizar".to_string(), |b| b.label());
            let (ax, ay) = at(base - d * back);
            let (bx, by) = at(base + d * ahead);
            line(&mut s, (ax, ay), (bx, by), 0.5, MARK);
            for (px, py) in [(ax, ay), (bx, by)] {
                let (nx, ny) = (nrm.x * 1.2, -nrm.y * 1.2);
                line(&mut s, (px, py), (px + nx, py + ny), 0.5, MARK);
            }
            let label = format!("{bars}/nervio L {}", dec(back + ahead, 2));
            let tip = if ahead > 0.0 {
                base + d * (ahead * 0.6)
            } else {
                base - d * (back * 0.6)
            };
            let (tx, ty) = at(tip);
            let _ = writeln!(
                s,
                r#"<path d="{}" {}/>"#,
                text_path(
                    &label,
                    Point2::new(tx + 1.5, -(ty - 1.5)),
                    Point2::new(1.0, 0.0),
                    2.2,
                    Align::Left
                ),
                paint(2.2, MARK)
            );
            let _ = k;
        }
    }
    // The heads of the waffle floors.
    for r in slabs {
        let Some(w) = &r.waffle else { continue };
        let Some(sl) = model.slab(r.slab) else {
            continue;
        };
        if sl.level != level {
            continue;
        }
        for p in &r.punching {
            let Some(c) = model.column(p.column) else {
                continue;
            };
            let h = w.head_reach;
            let corners = [
                Point2::new(c.position.x - h, c.position.y - h),
                Point2::new(c.position.x + h, c.position.y - h),
                Point2::new(c.position.x + h, c.position.y + h),
                Point2::new(c.position.x - h, c.position.y + h),
            ];
            let pts: Vec<(f64, f64)> = corners.iter().map(|q| at(*q)).collect();
            let mut d = String::new();
            for (i, (x, y)) in pts.iter().enumerate() {
                let _ = write!(d, "{}{} {} ", if i == 0 { "M" } else { "L" }, n(*x), n(*y));
            }
            let _ = writeln!(
                s,
                r#"<path d="{d}Z" fill="none" stroke="{MARK}" stroke-width="0.35" stroke-dasharray="1.5 1"/>"#
            );
        }
    }
    text(
        &mut s,
        "En rojo: armadura superior de los apoyos de los forjados unidireccionales (negativos) y ábacos de los reticulares.",
        MARGIN + 6.0,
        H - MARGIN - 4.0,
        2.3,
        GREY,
    );
    s
}

/// The beams, in elevation: a band each, as many as fit on a page.
fn beams_pages(
    model: &Model,
    proposals: &[Proposal],
    marks: &Marks,
    next: &mut impl FnMut() -> String,
) -> Vec<String> {
    let mut beams: Vec<_> = model
        .beams()
        .filter_map(|b| {
            let (r, p) = reinforcement_of(model, proposals, b.id.0)?;
            matches!(r, Reinforcement::Beam { .. }).then_some((b, r, p))
        })
        .collect();
    beams.sort_by_key(|(b, _, _)| {
        marks
            .beams
            .get(&b.id.0)
            .and_then(|m| m[1..].parse::<u32>().ok())
            .unwrap_or(0)
    });
    let band = 34.0;
    let per = ((H - 2.0 * MARGIN - BLOCK_H - 20.0) / band)
        .floor()
        .max(1.0) as usize;
    let mut out = Vec::new();
    for chunk in beams.chunks(per) {
        let mut s = String::new();
        frame(&mut s, model, &next(), "Despiece de vigas", None);
        let mut y = MARGIN + 12.0;
        text(&mut s, "DESPIECE DE VIGAS", MARGIN + 8.0, y, 3.5, INK);
        y += 6.0;
        for (b, r, proposed) in chunk {
            let Reinforcement::Beam {
                bottom,
                top,
                stirrups,
                cover,
            } = r
            else {
                continue;
            };
            let mark = marks.beams.get(&b.id.0).cloned().unwrap_or_default();
            let t = model.member_type(b.member_type);
            let (bw, bh) = t.map_or((0.3, 0.5), |t| t.profile.size());
            let length = b.start.distance(b.end);
            let x0 = MARGIN + 30.0;
            let span_w = W - 2.0 * MARGIN - 50.0;
            let k = span_w / length.max(0.1);
            let depth = (bh * k).clamp(8.0, 14.0);
            let top_y = y + 6.0;
            text(
                &mut s,
                &format!(
                    "{mark}{} · {}×{} cm · L {} m",
                    star(*proposed),
                    dec(bw * 100.0, 0),
                    dec(bh * 100.0, 0),
                    dec(length, 2)
                ),
                MARGIN + 8.0,
                y + 2.0,
                2.6,
                INK,
            );
            // The beam and its supports.
            rect(&mut s, x0, top_y, span_w, depth, 0.25, GREY);
            for x in [x0, x0 + span_w] {
                let _ = writeln!(
                    s,
                    r#"<path d="M{} {} l-2 3 h4 Z" fill="none" stroke="{INK}" stroke-width="0.25"/>"#,
                    n(x),
                    n(top_y + depth)
                );
            }
            // Stirrups, a tick each (as many as there are, to a point).
            let count = (length / stirrups.spacing).floor() as usize;
            let step = span_w / count.max(1) as f64;
            for i in 0..=count.min(80) {
                let x = x0 + i as f64 * step * (count as f64 / count.clamp(1, 80) as f64);
                if x > x0 + span_w + 0.1 {
                    break;
                }
                line(
                    &mut s,
                    (x, top_y + 1.0),
                    (x, top_y + depth - 1.0),
                    0.13,
                    GREY,
                );
            }
            // Bars: top and bottom, with their hooks (patillas).
            let hook = (0.15 * k).min(depth * 0.35);
            let anchor = (12.0 * bottom.diameter).max(0.15);
            let l_bar = length + 2.0 * anchor - 2.0 * cover;
            for (bars, yy, down) in [
                (top, top_y + 1.2, true),
                (bottom, top_y + depth - 1.2, false),
            ] {
                let hook_dir = if down { 1.0 } else { -1.0 };
                let _ = writeln!(
                    s,
                    r#"<path d="M{} {} L{} {} L{} {} L{} {}" fill="none" stroke="{MARK}" stroke-width="0.5"/>"#,
                    n(x0 - 1.5),
                    n(yy + hook_dir * hook),
                    n(x0 - 1.5),
                    n(yy),
                    n(x0 + span_w + 1.5),
                    n(yy),
                    n(x0 + span_w + 1.5),
                    n(yy + hook_dir * hook)
                );
                let label = format!("{} L {} m", bars.label(), dec(l_bar, 2));
                let ty = if down {
                    top_y - 1.0
                } else {
                    top_y + depth + 4.5
                };
                text(&mut s, &label, x0 + span_w / 2.0 - 15.0, ty, 2.4, MARK);
            }
            text(
                &mut s,
                &format!("Estribos {}", stirrups.label()),
                x0,
                top_y + depth + 8.5,
                2.4,
                INK,
            );
            y += band;
        }
        text(
            &mut s,
            "* armado propuesto por el cálculo. Longitudes con patillas de 15 cm; anclajes y solapes según el Código Estructural, art. 69.",
            MARGIN + 8.0,
            H - MARGIN - BLOCK_H - 3.0,
            2.3,
            GREY,
        );
        out.push(s);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use archisim_core::*;

    #[test]
    fn a_small_building_on_paper() {
        let mut m = Model::new("Estructura");
        let l0 = m.add_level("Planta baja", 0.0);
        let l1 = m.add_level("Planta 1", 3.0);
        let lib = archisim_library::add_library(&mut m).unwrap();
        let mut cols = Vec::new();
        for (x, y) in [(0.0, 0.0), (5.0, 0.0), (0.0, 4.0), (5.0, 4.0)] {
            cols.push(
                m.add_column(NewColumn {
                    member_type: lib.column,
                    level: l0,
                    position: Point2::new(x, y),
                    rotation: 0.0,
                    base_offset: 0.0,
                    top: WallTop::Unconnected,
                    height: 3.0,
                })
                .unwrap(),
            );
        }
        for (a, b) in [((0.0, 0.0), (5.0, 0.0)), ((0.0, 4.0), (5.0, 4.0))] {
            m.add_beam(NewBeam {
                member_type: lib.beam,
                level: l1,
                start: Point2::new(a.0, a.1),
                end: Point2::new(b.0, b.1),
                top_offset: 0.0,
            })
            .unwrap();
        }
        let one_way = m
            .slab_types()
            .find(|t| t.name.contains("unidireccional"))
            .unwrap()
            .id;
        let slab = m
            .add_slab(NewSlab {
                slab_type: one_way,
                level: l1,
                outline: vec![
                    Point2::new(0.0, 0.0),
                    Point2::new(5.0, 0.0),
                    Point2::new(5.0, 4.0),
                    Point2::new(0.0, 4.0),
                ],
                top_offset: 0.0,
            })
            .unwrap();
        m.set_slab_joists(slab, Some(std::f64::consts::FRAC_PI_2))
            .unwrap();
        let concrete = m
            .materials()
            .find(|x| x.name.contains("Hormigón"))
            .unwrap()
            .id;
        for c in &cols {
            let p = m.column(*c).unwrap().position;
            m.add_footing(NewFooting {
                level: l0,
                kind: FootingKind::Pad {
                    position: p,
                    length: 1.2,
                    rotation: 0.0,
                    column: Some(*c),
                },
                material: concrete,
                width: 1.2,
                depth: 0.5,
                top_offset: 0.0,
            })
            .unwrap();
        }
        let mk = marks(&m);
        assert_eq!(mk.stacks.len(), 4);
        // Read from the top left: P1 at (0, 4).
        assert!(mk.stacks[0].1.distance(Point2::new(0.0, 4.0)) < 1e-9);
        let pages = pages(&m);
        // Tables, foundations, the first floor and the beams.
        assert_eq!(pages.len(), 4, "{}", pages.len());
        assert!(pages.iter().all(|p| p.starts_with("<svg")));
        if let Ok(dir) = std::env::var("ARCHISIM_PLANOS_ESTRUCTURA_OUT") {
            for (i, p) in pages.iter().enumerate() {
                std::fs::write(format!("{dir}/estructura-{}.svg", i + 1), p).unwrap();
            }
        }
    }
}
