//! Drawing sheets in SVG: the frame and the title block, the views placed
//! on the sheet (each a plan at its scale, cut to its crop) with their
//! titles, and the sheet's own drafting.
//!
//! The SVG is in paper metres (1 mm = 0.001) with y flipped, like plans;
//! its size is in millimetres, so it prints at 100 %.

use std::fmt::Write as _;

use archisim_core::{LevelId, Model, Point2, SectionId, Sheet, SheetId, ViewRef, Viewport};

use crate::lettering::{text_path, text_width, Align, STROKE};
use crate::{num, render_plan, PlanOptions};

const INK: &str = "#1a1a1a";
const GREY: &str = "#6b6b6b";
/// Paper millimetres in paper metres.
const MM: f64 = 0.001;

/// Margins of the frame, millimetres: wider on the left for binding
/// (UNE 1-027).
pub const FRAME_LEFT: f64 = 20.0;
pub const FRAME: f64 = 10.0;
/// Size of the title block, millimetres.
pub const TITLE_W: f64 = 180.0;
pub const TITLE_H: f64 = 56.0;

/// Options of a sheet drawing.
#[derive(Debug, Clone, Default)]
pub struct SheetOptions {
    /// Draw only this region (paper metres, lower-left and upper-right), for
    /// the screen; `None` draws the whole sheet on white paper.
    pub region: Option<(Point2, Point2)>,
    /// Screen pixels per paper millimetre (on screen: details too small
    /// are left out).
    pub px_per_mm: Option<f64>,
    /// Leaves out what is drawn on the sheet itself (the DXF writes it
    /// natively).
    pub without_drafts: bool,
}

/// The extent of what a level shows, model metres: walls, slabs,
/// annotations and drafting. `None` for an empty level.
pub fn level_extent(model: &Model, level: LevelId, scale: f64) -> Option<(Point2, Point2)> {
    let k = scale / 1000.0;
    let mut pts: Vec<Point2> = Vec::new();
    for g in archisim_geom::wall_geometries(model, level) {
        pts.extend(g.outline);
    }
    for s in model.slabs_on_level(level) {
        pts.extend(s.outline.iter().copied());
    }
    for d in model.dimensions_on_level(level) {
        let g = crate::annotate::dimension_lines(d, k);
        pts.push(g.line.0);
        pts.push(g.line.1);
        pts.extend(g.extensions.iter().map(|e| e.1));
    }
    pts.extend(model.elevation_marks_on_level(level).map(|m| m.point));
    for d in model.drafts_in(ViewRef::Level(level)) {
        let (a, b) = archisim_draft::bounds(&d.shape, k);
        pts.push(a);
        pts.push(b);
    }
    let first = *pts.first()?;
    let (mut lo, mut hi) = (first, first);
    for p in pts {
        lo = Point2::new(lo.x.min(p.x), lo.y.min(p.y));
        hi = Point2::new(hi.x.max(p.x), hi.y.max(p.y));
    }
    // A little air around (5 mm of paper).
    let m = 5.0 * k;
    Some((lo - Point2::new(m, m), hi + Point2::new(m, m)))
}

/// The part of a section or elevation to show whole at a scale: what it
/// draws, the level labels on its right and a little air around (along
/// its line and in height, metres).
pub fn section_extent(model: &Model, section: SectionId, scale: f64) -> Option<(Point2, Point2)> {
    let sec = model.section(section)?;
    let d = crate::section::section_drawing(model, sec);
    Some(drawing_extent(&d, scale))
}

fn drawing_extent(d: &crate::section::SectionDrawing, scale: f64) -> (Point2, Point2) {
    let k = scale / 1000.0;
    let m = 5.0 * k;
    (
        d.lo - Point2::new(m, m),
        d.hi + Point2::new(m + d.labels_reach(scale), m),
    )
}

/// The part of the model a view shows (its crop, or its whole level or
/// section). A section view placed from the app always has its crop.
pub fn viewport_crop(model: &Model, v: &Viewport) -> (Point2, Point2) {
    let whole = || match (v.axo, v.section) {
        (Some(a), _) => Some(crate::axo::axo_extent(model, &a, v.scale)),
        (None, Some(s)) => section_extent(model, s, v.scale),
        (None, None) => level_extent(model, v.level, v.scale),
    };
    v.crop
        .or_else(whole)
        .unwrap_or((Point2::new(0.0, 0.0), Point2::new(5.0, 5.0)))
}

/// Where a view is on its sheet, paper metres (lower-left, upper-right).
pub fn viewport_rect(model: &Model, v: &Viewport) -> (Point2, Point2) {
    let (lo, hi) = viewport_crop(model, v);
    let half = Point2::new((hi.x - lo.x) / v.scale / 2.0, (hi.y - lo.y) / v.scale / 2.0);
    (v.centre - half, v.centre + half)
}

/// The point of the model under a point of the sheet inside a view.
pub fn sheet_to_model(model: &Model, v: &Viewport, p: Point2) -> Point2 {
    let (lo, hi) = viewport_crop(model, v);
    let c = (lo + hi) * 0.5;
    c + (p - v.centre) * v.scale
}

/// The title written under a view.
pub fn viewport_title(model: &Model, v: &Viewport) -> String {
    if v.title.trim().is_empty() {
        if v.axo.is_some() {
            return "VISTA 3D".into();
        }
        match v.section {
            Some(s) => model
                .section(s)
                .map(|s| s.title().to_uppercase())
                .unwrap_or_default(),
            None => model
                .level(v.level)
                .map(|l| l.name.to_uppercase())
                .unwrap_or_default(),
        }
    } else {
        v.title.clone()
    }
}

fn text(s: &mut String, t: &str, at: Point2, h_mm: f64, align: Align, color: &str) {
    if t.trim().is_empty() {
        return;
    }
    let h = h_mm * MM;
    let _ = writeln!(
        s,
        r#"<path d="{}" fill="none" stroke="{color}" stroke-width="{}" stroke-linecap="round" stroke-linejoin="round"/>"#,
        text_path(t, at, Point2::new(1.0, 0.0), h, align),
        num(STROKE * h)
    );
}

/// A text shortened with "…" to fit a width (millimetres).
fn fit(t: &str, h_mm: f64, width_mm: f64) -> String {
    if text_width(t, h_mm) <= width_mm {
        return t.to_string();
    }
    let mut out: String = t.to_string();
    while !out.is_empty() && text_width(&format!("{out}…"), h_mm) > width_mm {
        out.pop();
    }
    format!("{}…", out.trim_end())
}

fn line(s: &mut String, a: Point2, b: Point2, w_mm: f64) {
    let _ = writeln!(
        s,
        r#"<path d="M{},{} L{},{}" stroke="{INK}" stroke-width="{}" stroke-linecap="square"/>"#,
        num(a.x),
        num(-a.y),
        num(b.x),
        num(-b.y),
        num(w_mm * MM)
    );
}

fn rect(s: &mut String, a: Point2, b: Point2, w_mm: f64) {
    let _ = writeln!(
        s,
        r#"<path d="M{},{} L{},{} L{},{} L{},{} Z" fill="none" stroke="{INK}" stroke-width="{}"/>"#,
        num(a.x),
        num(-a.y),
        num(b.x),
        num(-a.y),
        num(b.x),
        num(-b.y),
        num(a.x),
        num(-b.y),
        num(w_mm * MM)
    );
}

/// The scale the title block shows: the views' if they share one, else
/// "Indicadas".
fn sheet_scale(model: &Model, id: SheetId) -> String {
    let mut scales: Vec<f64> = model.viewports_on(id).map(|v| v.scale).collect();
    scales.sort_by(f64::total_cmp);
    scales.dedup();
    match scales.as_slice() {
        [] => "—".into(),
        [s] => format!("1:{s}"),
        _ => "Indicadas".into(),
    }
}

/// The frame and the title block (cajetín) of a sheet.
fn draw_frame(s: &mut String, model: &Model, sheet: &Sheet) {
    let (w, h) = (sheet.width, sheet.height);
    let p = |x: f64, y: f64| Point2::new(x * MM, y * MM);
    rect(s, p(FRAME_LEFT, FRAME), p(w - FRAME, h - FRAME), 0.5);
    let x0 = w - FRAME - TITLE_W;
    let y0 = FRAME;
    let x1 = w - FRAME;
    let y1 = FRAME + TITLE_H;
    rect(s, p(x0, y0), p(x1, y1), 0.5);
    // Rows from the top: studio and project (16), site and phase (10),
    // sheet name (14), data cells (16).
    let r1 = y1 - 16.0;
    let r2 = r1 - 10.0;
    let r3 = r2 - 14.0;
    for y in [r1, r2, r3] {
        line(s, p(x0, y), p(x1, y), 0.25);
    }
    line(s, p(x0 + 55.0, y1), p(x0 + 55.0, r1), 0.25);
    let cells = [0.0, 30.0, 62.0, 112.0, 132.0, TITLE_W];
    for c in &cells[1..cells.len() - 1] {
        line(s, p(x0 + c, r3), p(x0 + c, y0), 0.25);
    }
    let info = model.info();
    let small = 1.8;
    let label = |s: &mut String, t: &str, x: f64, y: f64| {
        text(s, t, p(x + 1.5, y - 1.5 - small), small, Align::Left, GREY);
    };
    // Studio.
    let studio = if info.studio.trim().is_empty() {
        "ArchiSim"
    } else {
        info.studio.as_str()
    };
    // As large as fits (a long name gets smaller letters, down to 3 mm).
    let w = text_width(studio, 1.0).max(1e-9);
    let h = (50.0 / w).clamp(3.0, 6.0);
    text(
        s,
        &fit(studio, h, 50.0),
        p(x0 + 27.5, r1 + 5.0),
        h,
        Align::Centre,
        INK,
    );
    // Project and client.
    label(s, "PROYECTO", x0 + 55.0, y1);
    text(
        s,
        &fit(&model.project_name, 3.5, TITLE_W - 60.0),
        p(x0 + 57.0, r1 + 6.5),
        3.5,
        Align::Left,
        INK,
    );
    if !info.client.trim().is_empty() {
        text(
            s,
            &fit(&format!("Promotor: {}", info.client), 2.2, TITLE_W - 60.0),
            p(x0 + 57.0, r1 + 1.8),
            2.2,
            Align::Left,
            INK,
        );
    }
    // Site and phase.
    label(s, "EMPLAZAMIENTO", x0, r1);
    text(
        s,
        &fit(&info.site, 2.5, 110.0),
        p(x0 + 2.0, r2 + 2.0),
        2.5,
        Align::Left,
        INK,
    );
    label(s, "FASE", x0 + 120.0, r1);
    text(
        s,
        &fit(&info.phase, 2.5, 56.0),
        p(x0 + 122.0, r2 + 2.0),
        2.5,
        Align::Left,
        INK,
    );
    line(s, p(x0 + 118.0, r1), p(x0 + 118.0, r2), 0.25);
    // Sheet name.
    label(s, "PLANO", x0, r2);
    text(
        s,
        &fit(&sheet.name, 4.0, TITLE_W - 6.0),
        p(x0 + 2.0, r3 + 3.0),
        4.0,
        Align::Left,
        INK,
    );
    // Data cells.
    let scale = sheet_scale(model, sheet.id);
    let data = [
        ("ESCALA", scale.as_str(), 3.0),
        ("FECHA", info.date.as_str(), 2.5),
        ("AUTOR", info.author.as_str(), 2.5),
        ("REV.", sheet.revision.as_str(), 3.0),
        ("Nº PLANO", sheet.number.as_str(), 7.0),
    ];
    for (i, (name, value, th)) in data.iter().enumerate() {
        let cx0 = x0 + cells[i];
        let cw = cells[i + 1] - cells[i];
        label(s, name, cx0, r3);
        text(
            s,
            &fit(value, *th, cw - 3.0),
            p(cx0 + cw / 2.0, y0 + 3.5),
            *th,
            Align::Centre,
            INK,
        );
    }
    if !info.number.trim().is_empty() {
        text(
            s,
            &fit(&format!("Exp. {}", info.number), 1.8, 50.0),
            p(x0 + 27.5, r1 + 1.5),
            1.8,
            Align::Centre,
            GREY,
        );
    }
}

/// The title of a view: its name underlined, and its scale.
fn draw_view_title(s: &mut String, model: &Model, v: &Viewport) {
    let (lo, _) = viewport_rect(model, v);
    let t = viewport_title(model, v);
    let h = 4.0;
    let at = Point2::new(lo.x, lo.y - 8.0 * MM);
    text(s, &t, at, h, Align::Left, INK);
    let w = text_width(&t, h);
    line(
        s,
        at - Point2::new(0.0, 1.6 * MM),
        at + Point2::new(w * MM, -1.6 * MM),
        0.5,
    );
    text(
        s,
        &format!("E 1:{}", v.scale),
        at + Point2::new((w + 4.0) * MM, 0.0),
        2.5,
        Align::Left,
        INK,
    );
}

/// The plan, section or elevation of a view, as an SVG fragment placed
/// and cut on the sheet.
fn draw_viewport(s: &mut String, model: &Model, v: &Viewport, px_per_mm: Option<f64>) {
    if let Some(axo) = v.axo {
        let d = crate::axo::axo_drawing(model, &axo);
        let m = 5.0 * v.scale / 1000.0;
        let crop = v
            .crop
            .unwrap_or((d.lo - Point2::new(m, m), d.hi + Point2::new(m, m)));
        let svg = crate::axo::render_axo(&d, crop, v.scale);
        place_view_svg(s, &svg, v, crop);
        return;
    }
    let section = v
        .section
        .and_then(|id| model.section(id))
        .map(|sec| crate::section::section_drawing(model, sec));
    let (crop_lo, crop_hi) = match (&section, v.crop) {
        (_, Some(c)) => c,
        (Some(d), None) => drawing_extent(d, v.scale),
        (None, None) => viewport_crop(model, v),
    };
    let options = PlanOptions {
        scale: v.scale,
        cut_plane: v.cut_plane,
        viewport: Some((crop_lo, crop_hi)),
        style: v.style.clone(),
        px_per_mm,
        title: false,
        ..PlanOptions::default()
    };
    let plan = match &section {
        Some(d) => crate::section::render_section(model, d, &options),
        None => render_plan(model, v.level, &options),
    };
    place_view_svg(s, &plan, v, (crop_lo, crop_hi));
}

/// Puts the drawing of a view on the sheet, scaled and cut to its crop.
fn place_view_svg(s: &mut String, plan: &str, v: &Viewport, crop: (Point2, Point2)) {
    let (crop_lo, crop_hi) = crop;
    let half = Point2::new(
        (crop_hi.x - crop_lo.x) / v.scale / 2.0,
        (crop_hi.y - crop_lo.y) / v.scale / 2.0,
    );
    let (lo, hi) = (v.centre - half, v.centre + half);
    // The plan's own <svg> keeps its viewBox (the crop in model metres);
    // placed with x, y, width and height it scales and is cut to them.
    let body_start = plan.find('>').map_or(0, |i| i + 1);
    let body_end = plan.rfind("</svg>").unwrap_or(plan.len());
    let _ = writeln!(
        s,
        r#"<svg class="viewport" x="{}" y="{}" width="{}" height="{}" viewBox="{} {} {} {}" overflow="hidden" preserveAspectRatio="none">"#,
        num(lo.x),
        num(-hi.y),
        num(hi.x - lo.x),
        num(hi.y - lo.y),
        num(crop_lo.x),
        num(-crop_hi.y),
        num(crop_hi.x - crop_lo.x),
        num(crop_hi.y - crop_lo.y),
    );
    s.push_str(&plan[body_start..body_end]);
    s.push_str("</svg>\n");
}

/// Renders a sheet as an SVG document.
pub fn render_sheet(model: &Model, id: SheetId, options: &SheetOptions) -> String {
    let Some(sheet) = model.sheet(id) else {
        return String::from(r#"<svg xmlns="http://www.w3.org/2000/svg"/>"#);
    };
    let (w, h) = (sheet.width * MM, sheet.height * MM);
    let (vx, vy, vw, vh) = match options.region {
        Some((lo, hi)) => (
            lo.x,
            -hi.y,
            (hi.x - lo.x).max(1e-9),
            (hi.y - lo.y).max(1e-9),
        ),
        None => (0.0, -h, w, h),
    };
    let mut s = String::new();
    let _ = writeln!(
        s,
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="{:.2}mm" height="{:.2}mm" viewBox="{} {} {} {}">"#,
        vw / MM,
        vh / MM,
        num(vx),
        num(vy),
        num(vw),
        num(vh)
    );
    // The paper.
    let _ = writeln!(
        s,
        r#"<rect class="paper" x="0" y="{}" width="{}" height="{}" fill="white"/>"#,
        num(-h),
        num(w),
        num(h)
    );
    let visible = |a: Point2, b: Point2| match options.region {
        Some((lo, hi)) => !(a.x > hi.x || a.y > hi.y || b.x < lo.x || b.y < lo.y),
        None => true,
    };
    for v in model.viewports_on(id) {
        let (a, b) = viewport_rect(model, v);
        if visible(a, b - Point2::new(0.0, 0.0)) {
            draw_viewport(&mut s, model, v, options.px_per_mm);
        }
        if v.show_title {
            s.push_str("<g id=\"view-titles\">\n");
            draw_view_title(&mut s, model, v);
            s.push_str("</g>\n");
        }
    }
    let tables: Vec<&archisim_core::SheetTable> = model.tables_on(id).collect();
    if !tables.is_empty() {
        s.push_str("<g id=\"tables\">\n");
        for t in tables {
            let table = crate::schedules::table(model, t.kind, t.level);
            crate::schedules::draw_table(&mut s, &table, t.at);
        }
        s.push_str("</g>\n");
    }
    if sheet.title_block {
        s.push_str("<g id=\"title-block\">\n");
        draw_frame(&mut s, model, sheet);
        s.push_str("</g>\n");
    }
    if options.without_drafts {
        s.push_str("</svg>\n");
        return s;
    }
    crate::drafting::draw_drafts(
        &mut s,
        model,
        ViewRef::Sheet(id),
        MM,
        options.region,
        options.px_per_mm.is_none(),
        &Default::default(),
        false,
    );
    s.push_str("</svg>\n");
    s
}

#[cfg(test)]
mod tests {
    use super::*;
    use archisim_core::{NewSheet, NewViewport, NewWall, ReferenceLine, WallFunction, WallTop};

    fn house() -> (Model, LevelId) {
        let mut m = Model::new("Casa");
        let l = m.add_level("Planta baja", 0.0);
        let mat = m.add_material(
            "Ladrillo",
            "brick",
            [200, 100, 80],
            archisim_core::CutPattern::Diagonal,
        );
        let t = m
            .add_wall_type(
                "Muro",
                WallFunction::Exterior,
                vec![archisim_core::Layer {
                    material: mat,
                    thickness: 0.24,
                    function: archisim_core::LayerFunction::Structure,
                }],
            )
            .unwrap();
        let c = [
            Point2::new(0.0, 0.0),
            Point2::new(0.0, 6.0),
            Point2::new(8.0, 6.0),
            Point2::new(8.0, 0.0),
        ];
        for i in 0..4 {
            m.add_wall(NewWall {
                wall_type: t,
                level: l,
                start: c[i],
                end: c[(i + 1) % 4],
                bulge: 0.0,
                reference: ReferenceLine::ExteriorFace,
                height: 3.0,
                base_offset: 0.0,
                top: WallTop::Unconnected,
            })
            .unwrap();
        }
        (m, l)
    }

    #[test]
    fn a_sheet_with_a_view_and_its_title_block() {
        let (mut m, l) = house();
        let s = m
            .add_sheet(NewSheet {
                number: "A-01".into(),
                name: "Planta baja".into(),
                width: 420.0,
                height: 297.0,
            })
            .unwrap();
        let v = m
            .add_viewport(NewViewport {
                sheet: s,
                level: l,
                section: None,
                axo: None,
                scale: 50.0,
                cut_plane: 1.1,
                crop: None,
                centre: Point2::new(0.15, 0.17),
            })
            .unwrap();
        // 8 m at 1:50 is 160 mm of paper, plus 5 mm of air each side.
        let (a, b) = viewport_rect(&m, m.viewport(v).unwrap());
        assert!(((b.x - a.x) - 0.17).abs() < 0.01, "{a:?} {b:?}");
        // A point of the sheet at the view's centre is the model's centre.
        let c = sheet_to_model(&m, m.viewport(v).unwrap(), Point2::new(0.15, 0.17));
        assert!(c.distance(Point2::new(4.0, 3.0)) < 0.05, "{c:?}");
        let svg = render_sheet(&m, s, &SheetOptions::default());
        assert!(svg.starts_with("<svg") && svg.contains("width=\"420.00mm\""));
        assert!(svg.contains("class=\"viewport\""));
        assert!(svg.contains("id=\"poche\""));
        assert!(!svg.contains("<text"));
        // The SVG reads back (the PDF and the screen use it).
        if let Ok(path) = std::env::var("ARCHISIM_SHEET") {
            std::fs::write(path, &svg).unwrap();
        }
    }
}
