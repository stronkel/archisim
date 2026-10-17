//! Floor plan views rendered to SVG.
//!
//! The SVG is drawn at a real drawing scale: its width and height are in
//! paper millimetres, and line weights and hatch spacing are specified in
//! paper millimetres too, so printing it at 100 % gives a correct 1:50 (or
//! whatever scale is chosen) drawing.
//!
//! Drawing conventions (MVP):
//! - every cut layer is filled with its material colour and pattern;
//! - wall faces and free ends are drawn with the heavy cut line;
//! - layer boundaries are thin; mitred joins draw no line, so the poché of
//!   connected walls reads as continuous;
//! - insulation is drawn with the usual zigzag, cut to its layer; all
//!   hatching is real geometry cut to each layer (no SVG clip paths);
//! - slabs are drawn under the walls according to the cut plane: below it
//!   (the usual case: the floor you stand on) with a light cover fill and a
//!   thin projection line; crossing it as a cut, with the heavy line and the
//!   fill of its structural layer; above it with a thin dashed line.
//! - doors and windows crossed by the cut plane interrupt the wall: every
//!   layer stops at the jambs, which are drawn with the heavy cut line. The
//!   frame is drawn in the gap with a medium line; doors show the leaf open
//!   at 90° and its swing; windows show the glazing line, the sill board
//!   outside and the edge of the sill wall inside. Windows entirely above
//!   the cut plane are drawn dashed and do not interrupt the wall.

pub mod annotate;
pub mod lettering;

use std::fmt::Write as _;

use archisim_core::{
    CutPattern, DoorOperation, LayerFunction, LevelId, Model, OpeningKind, Point2, Slab,
    WallFunction, WindowOperation, EPSILON,
};
use archisim_geom::{
    clip_span, openings_in_wall, wall_geometries, wall_pieces, EndCut, OpeningGeometry,
    WallGeometry, WallPiece,
};

/// Options for a plan view.
#[derive(Debug, Clone)]
pub struct PlanOptions {
    /// Drawing scale denominator (50 means 1:50).
    pub scale: f64,
    /// Paper margin around the drawing, in millimetres.
    pub margin_mm: f64,
    /// Draw the walls' reference lines (dashed).
    pub show_reference_lines: bool,
    /// Height of the horizontal cut plane above the level, in metres.
    pub cut_plane: f64,
    /// Draw exactly this region of the plan (lower-left and upper-right
    /// corners, in model metres) instead of fitting the drawing to the
    /// elements. Used by on-screen views: no margin, title or background.
    pub viewport: Option<(Point2, Point2)>,
}

impl Default for PlanOptions {
    fn default() -> Self {
        Self {
            scale: 50.0,
            margin_mm: 15.0,
            show_reference_lines: false,
            cut_plane: 1.1,
            viewport: None,
        }
    }
}

/// Line weights in paper millimetres.
const CUT_LINE: f64 = 0.5;
const LAYER_LINE: f64 = 0.13;
const HATCH_LINE: f64 = 0.09;
const REF_LINE: f64 = 0.13;
const VIEW_LINE: f64 = 0.18;
const FRAME_LINE: f64 = 0.25;
/// Thickness of a door leaf drawn in plan, in metres.
const LEAF_THICKNESS: f64 = 0.04;
/// How far the window sill board sticks out of the façade, in metres.
const SILL_PROJECTION: f64 = 0.03;
const SLAB_COVER: &str = "#f1f1ee";
const HATCH_SPACING: f64 = 1.5;
const TEXT_SIZE: f64 = 3.0;
const INK: &str = "#1a1a1a";

/// Renders the plan of one level as an SVG document.
pub fn render_plan(model: &Model, level: LevelId, options: &PlanOptions) -> String {
    // Paper millimetres → model metres.
    let k = options.scale / 1000.0;
    let geos = wall_geometries(model, level);
    let slabs: Vec<&Slab> = model.slabs_on_level(level).collect();

    let mut min = Point2::new(f64::INFINITY, f64::INFINITY);
    let mut max = Point2::new(f64::NEG_INFINITY, f64::NEG_INFINITY);
    let all_points = geos
        .iter()
        .flat_map(|g| g.outline.iter())
        .chain(slabs.iter().flat_map(|s| s.outline.iter()));
    let dimensions: Vec<_> = model.dimensions_on_level(level).collect();
    let marks: Vec<_> = model.elevation_marks_on_level(level).collect();
    let annotation_points: Vec<Point2> = dimensions
        .iter()
        .flat_map(|d| {
            let g = annotate::dimension_lines(d, k);
            let mut v = vec![g.line.0, g.line.1];
            v.extend(g.extensions.iter().map(|e| e.1));
            v
        })
        .chain(marks.iter().map(|m| m.point))
        .collect();
    let all_points = all_points.chain(annotation_points.iter());
    for p in all_points {
        min = Point2::new(min.x.min(p.x), min.y.min(p.y));
        max = Point2::new(max.x.max(p.x), max.y.max(p.y));
    }
    if geos.is_empty() && slabs.is_empty() && annotation_points.is_empty() {
        min = Point2::new(0.0, 0.0);
        max = Point2::new(1.0, 1.0);
    }
    let (vb_x, vb_y, vb_w, vb_h) = match options.viewport {
        Some((lo, hi)) => (
            lo.x,
            -hi.y,
            (hi.x - lo.x).max(EPSILON),
            (hi.y - lo.y).max(EPSILON),
        ),
        None => {
            let margin = options.margin_mm * k;
            let title_space = 4.0 * TEXT_SIZE * k;
            // SVG y grows downwards: plot (x, -y).
            (
                min.x - margin,
                -max.y - margin,
                (max.x - min.x) + 2.0 * margin,
                (max.y - min.y) + 2.0 * margin + title_space,
            )
        }
    };
    let margin = options.margin_mm * k;

    let mut s = String::new();
    let _ = writeln!(
        s,
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="{:.2}mm" height="{:.2}mm" viewBox="{} {} {} {}">"#,
        vb_w / k,
        vb_h / k,
        num(vb_x),
        num(vb_y),
        num(vb_w),
        num(vb_h)
    );
    if options.viewport.is_none() {
        let _ = writeln!(
            s,
            r#"<rect x="{}" y="{}" width="{}" height="{}" fill="white"/>"#,
            num(vb_x),
            num(vb_y),
            num(vb_w),
            num(vb_h)
        );
    }

    s.push_str("<g id=\"slabs\">\n");
    for (si, slab) in slabs.iter().enumerate() {
        draw_slab(&mut s, model, slab, si, options.cut_plane, k);
    }
    s.push_str("</g>\n");

    // Openings of each wall, and the solid pieces left by those that are cut.
    let openings: Vec<Vec<OpeningGeometry>> = geos
        .iter()
        .map(|g| openings_in_wall(model, g.wall))
        .collect();
    let pieces: Vec<Vec<WallPiece>> = openings
        .iter()
        .map(|ops| {
            let spans: Vec<(f64, f64)> = ops
                .iter()
                .filter(|o| opening_view(o, options.cut_plane) == OpeningView::Cut)
                .map(|o| (o.t0, o.t1))
                .collect();
            wall_pieces(&spans)
        })
        .collect();

    s.push_str("<g id=\"poche\">\n");
    for (gi, g) in geos.iter().enumerate() {
        let cut: Vec<OpeningGeometry> = openings[gi]
            .iter()
            .filter(|o| opening_view(o, options.cut_plane) == OpeningView::Cut)
            .cloned()
            .collect();
        draw_layers(&mut s, model, g, gi, &pieces[gi], &cut, k);
    }
    s.push_str("</g>\n<g id=\"openings\" stroke-linejoin=\"miter\">\n");
    for o in openings.iter().flatten() {
        match opening_view(o, options.cut_plane) {
            OpeningView::Cut => draw_opening(&mut s, o, k),
            OpeningView::Above => draw_opening_above(&mut s, o, k),
            OpeningView::Below => {}
        }
    }
    // Cut lines go last so frames and sill boards never cover the jambs.
    s.push_str("</g>\n<g id=\"cut-lines\" fill=\"none\" stroke-linecap=\"square\">\n");
    for (g, p) in geos.iter().zip(&pieces) {
        draw_cut_lines(&mut s, g, p, k);
    }
    s.push_str("</g>\n");

    s.push_str("<g id=\"annotations\">\n");
    for d in &dimensions {
        annotate::draw_dimension(&mut s, d, k);
    }
    for m in &marks {
        annotate::draw_elevation_mark(&mut s, model, m, k);
    }
    s.push_str("</g>\n");

    if options.show_reference_lines {
        let _ = writeln!(
            s,
            r##"<g id="reference-lines" stroke="#c0392b" stroke-width="{}" stroke-dasharray="{} {}">"##,
            num(REF_LINE * k),
            num(4.0 * k),
            num(1.5 * k)
        );
        for g in &geos {
            let (a, b) = g.axis;
            let _ = writeln!(s, "<path d=\"M{} L{}\"/>", pt(a), pt(b));
        }
        s.push_str("</g>\n");
    }

    if options.viewport.is_none() {
        // Title.
        let level_name = model.level(level).map(|l| l.name.as_str()).unwrap_or("");
        let tx = min.x;
        let ty = -min.y + margin * 0.6;
        let _ = writeln!(
            s,
            r#"<text x="{}" y="{}" font-family="Helvetica, Arial, sans-serif" font-size="{}" fill="{INK}"><tspan font-weight="bold">{}</tspan> · {} · E 1:{}</text>"#,
            num(tx),
            num(ty + TEXT_SIZE * k),
            num(TEXT_SIZE * k),
            escape(&model.project_name),
            escape(level_name),
            options.scale
        );
    }
    s.push_str("</svg>\n");
    s
}

/// How the layers of a wall type are finished at the jambs of a door or
/// window, as usually built in a Spanish double-leaf façade.
#[derive(Debug, Clone, PartialEq)]
struct JambDetail {
    /// The outer leaf that turns at the jamb to close the cavity, and the
    /// cavity layers it closes (they stop short of the jamb).
    turn: Option<(usize, std::ops::Range<usize>)>,
    /// The exterior finish (render) that wraps the reveal up to the frame.
    wrap: Option<usize>,
}

impl JambDetail {
    fn for_type(wtype: &archisim_core::WallType) -> Self {
        use LayerFunction::*;
        let layers = &wtype.layers;
        let is_cavity = |f: LayerFunction| matches!(f, Insulation | AirGap | Membrane);
        // The first leaf (from outside) followed by a cavity with something
        // behind it: that leaf returns across the cavity.
        let turn = (0..layers.len()).find_map(|i| {
            if !matches!(layers[i].function, Structure | Substrate) {
                return None;
            }
            let start = i + 1;
            let mut end = start;
            while end < layers.len() && is_cavity(layers[end].function) {
                end += 1;
            }
            (end > start && end < layers.len()).then_some((i, start..end))
        });
        let wrap = (wtype.function == WallFunction::Exterior
            && layers.len() > 1
            && layers[0].function == Finish)
            .then_some(0);
        Self { turn, wrap }
    }

    fn cut_back(&self, layer: usize) -> bool {
        self.turn
            .as_ref()
            .is_some_and(|(_, cavity)| cavity.contains(&layer))
    }
}

/// Fills a cut polygon with its material colour and hatch. `id` names the
/// fill so each cut piece can be found in the drawing.
fn fill_cut(s: &mut String, mat: &archisim_core::Material, polygon: &[Point2], id: &str, k: f64) {
    let d = path(polygon);
    let [r, gg, bb] = mat.cut_color;
    // A hairline in the fill colour hides anti-aliasing seams at mitres.
    let _ = writeln!(
        s,
        r#"<path id="{id}" d="{d}" fill="rgb({r},{gg},{bb})" stroke="rgb({r},{gg},{bb})" stroke-width="{}"/>"#,
        num(HATCH_LINE * k)
    );
    // Hatching is real geometry cut to the layer (not SVG patterns or clip
    // paths), so it prints identically everywhere, carries over to PDF/DXF
    // and draws fast on screen.
    let sp = HATCH_SPACING * k;
    let hatch = match mat.cut_pattern {
        CutPattern::Diagonal => hatch_lines(polygon, &[45.0], sp),
        CutPattern::Cross => hatch_lines(polygon, &[45.0, 135.0], sp),
        _ => String::new(),
    };
    if !hatch.is_empty() {
        let _ = writeln!(
            s,
            r#"<path d="{hatch}" fill="none" stroke="{INK}" stroke-width="{}"/>"#,
            num(HATCH_LINE * k)
        );
    }
    if mat.cut_pattern == CutPattern::Dots {
        let dots = hatch_dots(polygon, sp, HATCH_LINE * k * 1.2);
        if !dots.is_empty() {
            let _ = writeln!(s, r#"<path d="{dots}" fill="{INK}"/>"#);
        }
    }
}

fn draw_layers(
    s: &mut String,
    model: &Model,
    g: &WallGeometry,
    gi: usize,
    pieces: &[WallPiece],
    cut: &[OpeningGeometry],
    k: f64,
) {
    let (a, b) = g.axis;
    let dir = b - a;
    let len = dir.length();
    let (u, n) = match dir.normalized() {
        Some(u) => (u, u.perp_left()),
        None => return,
    };
    let wall_t = g.exterior_offset - g.interior_offset;
    let Some(wtype) = model
        .wall(g.wall)
        .and_then(|w| model.wall_type(w.wall_type))
    else {
        return;
    };
    let jamb = JambDetail::for_type(wtype);
    let turn_width = jamb
        .turn
        .as_ref()
        .map_or(0.0, |(i, _)| wtype.layers[*i].thickness);

    // Each layer clipped to a piece, stopping short of the jambs if it is
    // inside the cavity that the outer leaf closes.
    let layer_piece = |li: usize, piece: &WallPiece| -> Vec<Point2> {
        let back = if jamb.cut_back(li) { turn_width } else { 0.0 };
        clip_span(
            &g.layers[li].polygon,
            a,
            u,
            piece.from.map(|f| f + back),
            piece.to.map(|t| t - back),
        )
    };

    // Offsets of each layer's exterior face.
    let mut tops = Vec::with_capacity(wtype.layers.len() + 1);
    let mut high = g.exterior_offset;
    for layer in &wtype.layers {
        tops.push(high);
        high -= layer.thickness;
    }
    tops.push(high);

    for ((li, band), layer) in g.layers.iter().enumerate().zip(&wtype.layers) {
        let Some(mat) = model.material(band.material) else {
            continue;
        };
        let thickness = layer.thickness;
        let (high, low) = (tops[li], tops[li + 1]);
        for (pi, piece) in pieces.iter().enumerate() {
            let polygon = layer_piece(li, piece);
            if polygon.is_empty() {
                continue;
            }
            fill_cut(s, mat, &polygon, &format!("cut-{gi}-{li}-{pi}"), k);

            if mat.cut_pattern == CutPattern::Insulation
                || band.function == LayerFunction::Insulation
            {
                // Zigzag along the layer, overshooting the ends and cut to it.
                let mid = (high + low) / 2.0;
                let amp = thickness / 2.0;
                let step = thickness.max(0.01);
                let mut zigzag = Vec::new();
                let mut t = -wall_t * 2.0;
                let mut up = true;
                while t <= len + wall_t * 2.0 {
                    let off = mid + if up { amp } else { -amp };
                    zigzag.push(a + u * t + n * off);
                    t += step / 2.0;
                    up = !up;
                }
                let zz = clip_polyline(&zigzag, &polygon);
                if !zz.is_empty() {
                    let _ = writeln!(
                        s,
                        r#"<path d="{zz}" fill="none" stroke="{INK}" stroke-width="{}" stroke-linejoin="miter"/>"#,
                        num(HATCH_LINE * k)
                    );
                }
            }
        }
    }

    // Thin boundaries between layers, along the faces only. Where a cavity
    // layer stops short of a jamb, the boundary stops with it.
    for li in 0..wtype.layers.len().saturating_sub(1) {
        let owner = if jamb.cut_back(li + 1) { li + 1 } else { li };
        for piece in pieces {
            let polygon = layer_piece(owner, piece);
            if let Some((p, q)) = face_edge_at(&polygon, a, n, tops[li + 1]) {
                line(s, p, q, LAYER_LINE, k);
            }
        }
    }

    // Jamb details: (jamb position, +1 if the wall continues towards
    // growing t, the opening whose jamb it is).
    let mut jambs = Vec::new();
    for piece in pieces {
        if let Some(t) = piece.from {
            if let Some(o) = cut.iter().find(|o| (o.t1 - t).abs() < 1e-9) {
                jambs.push((t, 1.0, o));
            }
        }
        if let Some(t) = piece.to {
            if let Some(o) = cut.iter().find(|o| (o.t0 - t).abs() < 1e-9) {
                jambs.push((t, -1.0, o));
            }
        }
    }
    let local = |t: f64, off: f64| a + u * t + n * off;
    let rect = |t0: f64, t1: f64, y0: f64, y1: f64| {
        vec![local(t0, y0), local(t1, y0), local(t1, y1), local(t0, y1)]
    };
    for (ji, &(t, side, o)) in jambs.iter().enumerate() {
        // The outer leaf turns across the cavity, as wide as it is thick.
        if let Some((leaf, cavity)) = &jamb.turn {
            if let Some(mat) = model.material(wtype.layers[*leaf].material) {
                let (y_out, y_in) = (tops[*leaf + 1], tops[cavity.end]);
                let back = t + side * turn_width;
                fill_cut(
                    s,
                    mat,
                    &rect(t, back, y_in, y_out),
                    &format!("cut-{gi}-turn-{ji}"),
                    k,
                );
                // Against the cavity and against the inner leaf.
                line(s, local(back, y_out), local(back, y_in), LAYER_LINE, k);
                line(s, local(t, y_in), local(back, y_in), LAYER_LINE, k);
            }
        }
        // The render wraps the reveal from the façade to the frame.
        if let Some(fin) = jamb.wrap {
            let f = wtype.layers[fin].thickness;
            let (y_out, y_in) = (tops[fin + 1], o.frame_exterior);
            if let (Some(mat), true) = (
                model.material(wtype.layers[fin].material),
                y_in < y_out - EPSILON,
            ) {
                let back = t + side * f;
                fill_cut(
                    s,
                    mat,
                    &rect(t, back, y_in, y_out),
                    &format!("cut-{gi}-wrap-{ji}"),
                    k,
                );
                line(s, local(back, y_out), local(back, y_in), LAYER_LINE, k);
                line(s, local(t, y_in), local(back, y_in), LAYER_LINE, k);
            }
        }
    }
}

/// How a slab relates to the cut plane of the view.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SlabView {
    Below,
    Cut,
    Above,
}

fn slab_view(model: &Model, slab: &Slab, cut_plane: f64) -> SlabView {
    let thickness = model
        .slab_type(slab.slab_type)
        .map_or(0.0, |t| t.thickness());
    let (bottom, top) = (slab.top_offset - thickness, slab.top_offset);
    if top <= cut_plane {
        SlabView::Below
    } else if bottom < cut_plane {
        SlabView::Cut
    } else {
        SlabView::Above
    }
}

fn draw_slab(s: &mut String, model: &Model, slab: &Slab, si: usize, cut_plane: f64, k: f64) {
    let d = path(&slab.outline);
    match slab_view(model, slab, cut_plane) {
        SlabView::Below => {
            let _ = writeln!(
                s,
                r#"<path d="{d}" fill="{SLAB_COVER}" stroke="{INK}" stroke-width="{}" stroke-linejoin="round"/>"#,
                num(VIEW_LINE * k)
            );
        }
        SlabView::Above => {
            let _ = writeln!(
                s,
                r#"<path d="{d}" fill="none" stroke="{INK}" stroke-width="{}" stroke-dasharray="{} {}"/>"#,
                num(VIEW_LINE * k),
                num(3.0 * k),
                num(1.5 * k)
            );
        }
        SlabView::Cut => {
            // Filled with the structural layer (or the thickest one).
            let mat = model.slab_type(slab.slab_type).and_then(|t| {
                t.layers
                    .iter()
                    .find(|l| l.function == LayerFunction::Structure)
                    .or_else(|| {
                        t.layers
                            .iter()
                            .max_by(|a, b| a.thickness.total_cmp(&b.thickness))
                    })
                    .and_then(|l| model.material(l.material))
            });
            let (fill, pattern) = mat.map_or(([255, 255, 255], CutPattern::Solid), |m| {
                (m.cut_color, m.cut_pattern)
            });
            let [r, g, b] = fill;
            let _ = writeln!(
                s,
                r#"<path id="cut-slab-{si}" d="{d}" fill="rgb({r},{g},{b})"/>"#
            );
            let angles: &[f64] = match pattern {
                CutPattern::Diagonal => &[45.0],
                CutPattern::Cross => &[45.0, 135.0],
                _ => &[],
            };
            let hatch = hatch_lines(&slab.outline, angles, HATCH_SPACING * k);
            if !hatch.is_empty() {
                let _ = writeln!(
                    s,
                    r#"<path d="{hatch}" fill="none" stroke="{INK}" stroke-width="{}"/>"#,
                    num(HATCH_LINE * k)
                );
            }
            let _ = writeln!(
                s,
                r#"<path d="{d}" fill="none" stroke="{INK}" stroke-width="{}" stroke-linejoin="miter"/>"#,
                num(CUT_LINE * k)
            );
        }
    }
}

/// Parallel hatch lines cut to a polygon, anchored to the project origin so
/// that hatching continues seamlessly across elements. Angles are in
/// degrees, measured counter-clockwise from the X axis.
fn hatch_lines(poly: &[Point2], angles: &[f64], spacing: f64) -> String {
    let mut d = String::new();
    for &deg in angles {
        let rad = deg.to_radians();
        let dir = Point2::new(rad.cos(), rad.sin());
        let m = dir.perp_left();
        let (mut lo, mut hi) = (f64::INFINITY, f64::NEG_INFINITY);
        let (mut t0, mut t1) = (f64::INFINITY, f64::NEG_INFINITY);
        for p in poly {
            lo = lo.min(p.dot(m));
            hi = hi.max(p.dot(m));
            t0 = t0.min(p.dot(dir));
            t1 = t1.max(p.dot(dir));
        }
        let first = (lo / spacing).ceil() as i64;
        let last = (hi / spacing).floor() as i64;
        for i in first..=last {
            let c = i as f64 * spacing;
            let p = m * c + dir * (t0 - spacing);
            let q = m * c + dir * (t1 + spacing);
            for (a, b) in clip_segment(p, q, poly) {
                let _ = write!(d, "M{} L{} ", pt(a), pt(b));
            }
        }
    }
    d.trim_end().to_string()
}

/// A grid of small dots inside a polygon, anchored to the project origin.
fn hatch_dots(poly: &[Point2], spacing: f64, radius: f64) -> String {
    let (mut min, mut max) = (
        Point2::new(f64::INFINITY, f64::INFINITY),
        Point2::new(f64::NEG_INFINITY, f64::NEG_INFINITY),
    );
    for p in poly {
        min = Point2::new(min.x.min(p.x), min.y.min(p.y));
        max = Point2::new(max.x.max(p.x), max.y.max(p.y));
    }
    let mut d = String::new();
    let (i0, i1) = (
        (min.x / spacing).ceil() as i64,
        (max.x / spacing).floor() as i64,
    );
    let (j0, j1) = (
        (min.y / spacing).ceil() as i64,
        (max.y / spacing).floor() as i64,
    );
    for i in i0..=i1 {
        for j in j0..=j1 {
            // Stagger alternate rows for a more even texture.
            let x = (i as f64 + if j % 2 == 0 { 0.0 } else { 0.5 }) * spacing;
            let c = Point2::new(x, j as f64 * spacing);
            // Only whole dots: none sticks out of the layer.
            if contains(poly, c) && distance_to_outline(poly, c) >= radius {
                let _ = write!(
                    d,
                    "M{} a{r},{r} 0 1,0 {dd},0 a{r},{r} 0 1,0 -{dd},0 ",
                    pt(c - Point2::new(radius, 0.0)),
                    r = num(radius),
                    dd = num(2.0 * radius)
                );
            }
        }
    }
    d.trim_end().to_string()
}

/// The parts of segment `p`–`q` inside a simple polygon (any shape).
fn clip_segment(p: Point2, q: Point2, poly: &[Point2]) -> Vec<(Point2, Point2)> {
    let d = q - p;
    let n = poly.len();
    let mut ts = vec![0.0, 1.0];
    for i in 0..n {
        let (a, b) = (poly[i], poly[(i + 1) % n]);
        let e = b - a;
        let den = d.cross(e);
        if den.abs() < 1e-15 {
            continue;
        }
        let t = (a - p).cross(e) / den;
        let s = (a - p).cross(d) / den;
        if (-1e-12..=1.0 + 1e-12).contains(&s) && t > 0.0 && t < 1.0 {
            ts.push(t);
        }
    }
    ts.sort_by(f64::total_cmp);
    let mut out: Vec<(f64, f64)> = Vec::new();
    for w in ts.windows(2) {
        let (t0, t1) = (w[0], w[1]);
        if (t1 - t0) * d.length() < 1e-9 || !contains(poly, p + d * ((t0 + t1) / 2.0)) {
            continue;
        }
        match out.last_mut() {
            Some(last) if (last.1 - t0).abs() < 1e-12 => last.1 = t1,
            _ => out.push((t0, t1)),
        }
    }
    out.into_iter()
        .map(|(a, b)| (p + d * a, p + d * b))
        .collect()
}

/// A polyline cut to a polygon, as SVG path data: each run inside the
/// polygon stays one continuous line so its corners keep their joins.
fn clip_polyline(points: &[Point2], poly: &[Point2]) -> String {
    let mut d = String::new();
    let mut pen: Option<Point2> = None;
    for w in points.windows(2) {
        for (a, b) in clip_segment(w[0], w[1], poly) {
            match pen {
                Some(at) if at.approx_eq(a) => {}
                _ => {
                    let _ = write!(d, "M{} ", pt(a));
                }
            }
            let _ = write!(d, "L{} ", pt(b));
            pen = Some(b);
        }
    }
    d.trim_end().to_string()
}

/// Distance from a point to the outline of a polygon.
fn distance_to_outline(poly: &[Point2], p: Point2) -> f64 {
    let n = poly.len();
    (0..n)
        .map(|i| {
            let (a, b) = (poly[i], poly[(i + 1) % n]);
            let e = b - a;
            let len2 = e.dot(e);
            let t = if len2 < 1e-18 {
                0.0
            } else {
                ((p - a).dot(e) / len2).clamp(0.0, 1.0)
            };
            (a + e * t).distance(p)
        })
        .fold(f64::INFINITY, f64::min)
}

/// Point-in-polygon test (even-odd rule).
fn contains(poly: &[Point2], p: Point2) -> bool {
    let mut inside = false;
    let n = poly.len();
    for i in 0..n {
        let (a, b) = (poly[i], poly[(i + n - 1) % n]);
        if (a.y > p.y) != (b.y > p.y) && p.x < (b.x - a.x) * (p.y - a.y) / (b.y - a.y) + a.x {
            inside = !inside;
        }
    }
    inside
}

/// The edge of a layer polygon lying on the given offset from the axis.
fn face_edge_at(
    poly: &[Point2],
    origin: Point2,
    n: Point2,
    offset: f64,
) -> Option<(Point2, Point2)> {
    let on = |p: Point2| ((p - origin).dot(n) - offset).abs() < 1e-6;
    (0..poly.len()).find_map(|i| {
        let (p, q) = (poly[i], poly[(i + 1) % poly.len()]);
        (on(p) && on(q)).then_some((p, q))
    })
}

fn draw_cut_lines(s: &mut String, g: &WallGeometry, pieces: &[WallPiece], k: f64) {
    let (a, b) = g.axis;
    let Some(u) = (b - a).normalized() else {
        return;
    };
    let n = u.perp_left();
    let half = a.distance(b) / 2.0;
    let t = |p: Point2| (p - a).dot(u);
    let off = |p: Point2| (p - a).dot(n);
    let near = |x: f64, y: f64| (x - y).abs() < 1e-6;

    let mut d = String::new();
    for piece in pieces {
        let poly = clip_span(&g.outline, a, u, piece.from, piece.to);
        for i in 0..poly.len() {
            let (p, q) = (poly[i], poly[(i + 1) % poly.len()]);
            let on_face = [g.exterior_offset, g.interior_offset]
                .iter()
                .any(|&f| near(off(p), f) && near(off(q), f));
            let on_jamb = [piece.from, piece.to]
                .iter()
                .flatten()
                .any(|&j| near(t(p), j) && near(t(q), j));
            let heavy = on_face
                || on_jamb
                || if (t(p) + t(q)) / 2.0 < half {
                    g.start_cut == EndCut::Square
                } else {
                    g.end_cut == EndCut::Square
                };
            if heavy {
                let _ = write!(d, "M{} L{} ", pt(p), pt(q));
            }
        }
    }
    let _ = writeln!(
        s,
        r#"<path d="{}" stroke="{INK}" stroke-width="{}"/>"#,
        d.trim_end(),
        num(CUT_LINE * k)
    );
}

/// How a door or window relates to the cut plane of the view.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum OpeningView {
    Below,
    Cut,
    Above,
}

fn opening_view(o: &OpeningGeometry, cut_plane: f64) -> OpeningView {
    if o.head <= cut_plane + EPSILON {
        OpeningView::Below
    } else if o.sill < cut_plane - EPSILON {
        OpeningView::Cut
    } else {
        OpeningView::Above
    }
}

/// A closed rectangle from four corners, filled white.
fn quad(s: &mut String, pts: [Point2; 4], weight: f64, k: f64) {
    let _ = writeln!(
        s,
        r#"<path d="{}" fill="white" stroke="{INK}" stroke-width="{}"/>"#,
        path(&pts),
        num(weight * k)
    );
}

fn line(s: &mut String, p: Point2, q: Point2, weight: f64, k: f64) {
    let _ = writeln!(
        s,
        r#"<path d="M{} L{}" fill="none" stroke="{INK}" stroke-width="{}"/>"#,
        pt(p),
        pt(q),
        num(weight * k)
    );
}

/// A rectangle in the opening's own coordinates: `t` along the wall and
/// offsets across it.
fn rect(o: &OpeningGeometry, t0: f64, t1: f64, y0: f64, y1: f64) -> [Point2; 4] {
    [
        o.point(t0, y0),
        o.point(t1, y0),
        o.point(t1, y1),
        o.point(t0, y1),
    ]
}

/// A door or window crossed by the cut plane.
fn draw_opening(s: &mut String, o: &OpeningGeometry, k: f64) {
    let (fe, fi, fw) = (o.frame_exterior, o.frame_interior, o.frame_width);
    match o.kind {
        OpeningKind::Door(op) => {
            // Jambs of the frame ("cerco").
            quad(s, rect(o, o.t0, o.t0 + fw, fi, fe), FRAME_LINE, k);
            quad(s, rect(o, o.t1 - fw, o.t1, fi, fe), FRAME_LINE, k);
            let (c0, c1) = (o.t0 + fw, o.t1 - fw);
            match op {
                DoorOperation::SingleSwing => {
                    let (hinge, dir) = if o.hinge_at_start() {
                        (c0, 1.0)
                    } else {
                        (c1, -1.0)
                    };
                    draw_leaf(s, o, hinge, dir, c1 - c0, Leaf::Door, k);
                }
                DoorOperation::DoubleSwing => {
                    let w = (c1 - c0) / 2.0;
                    draw_leaf(s, o, c0, 1.0, w, Leaf::Door, k);
                    draw_leaf(s, o, c1, -1.0, w, Leaf::Door, k);
                }
                DoorOperation::Sliding => draw_sliding_leaf(s, o, c0, c1, k),
            }
        }
        OpeningKind::Window(op) => {
            let mid = (fe + fi) / 2.0;
            // Sill wall seen below the cut: sill board outside, edge inside.
            // The board runs from the frame to past the façade, and only
            // beyond the façade does it reach past the jambs (behind them
            // it is hidden by the cut wall).
            if o.sill > EPSILON {
                let (ext, out, lap) =
                    (o.exterior_offset, o.exterior_offset + SILL_PROJECTION, 0.02);
                let board = [
                    o.point(o.t0, fe),
                    o.point(o.t1, fe),
                    o.point(o.t1, ext),
                    o.point(o.t1 + lap, ext),
                    o.point(o.t1 + lap, out),
                    o.point(o.t0 - lap, out),
                    o.point(o.t0 - lap, ext),
                    o.point(o.t0, ext),
                ];
                let _ = writeln!(
                    s,
                    r#"<path d="{}" fill="white" stroke="{INK}" stroke-width="{}"/>"#,
                    path(&board),
                    num(LAYER_LINE * k)
                );
                line(
                    s,
                    o.point(o.t0, o.interior_offset),
                    o.point(o.t1, o.interior_offset),
                    LAYER_LINE,
                    k,
                );
            }
            quad(s, rect(o, o.t0, o.t1, fi, fe), FRAME_LINE, k);
            let (c0, c1) = (o.t0 + fw, o.t1 - fw);
            match op {
                WindowOperation::Fixed => {
                    line(s, o.point(c0, mid), o.point(c1, mid), LAYER_LINE, k);
                }
                WindowOperation::SingleCasement => {
                    line(s, o.point(c0, mid), o.point(c1, mid), LAYER_LINE, k);
                    let (hinge, dir) = if o.hinge_at_start() {
                        (c0, 1.0)
                    } else {
                        (c1, -1.0)
                    };
                    draw_leaf(s, o, hinge, dir, c1 - c0, Leaf::Window, k);
                }
                WindowOperation::DoubleCasement => {
                    let tm = (o.t0 + o.t1) / 2.0;
                    line(s, o.point(c0, mid), o.point(c1, mid), LAYER_LINE, k);
                    line(s, o.point(tm, fi), o.point(tm, fe), FRAME_LINE, k);
                    // Each leaf reaches the centre of the mullion.
                    draw_leaf(s, o, c0, 1.0, tm - c0, Leaf::Window, k);
                    draw_leaf(s, o, c1, -1.0, c1 - tm, Leaf::Window, k);
                }
                WindowOperation::Sliding => {
                    // Two leaves on separate tracks, overlapping at the centre.
                    let tm = (o.t0 + o.t1) / 2.0;
                    let q = (fe - fi) / 4.0;
                    let lap = fw / 2.0;
                    quad(s, rect(o, c0, tm + lap, mid, mid + q), LAYER_LINE, k);
                    quad(s, rect(o, tm - lap, c1, mid - q, mid), LAYER_LINE, k);
                }
            }
        }
    }
}

/// How an open leaf is drawn.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Leaf {
    /// A door leaf: a rectangle as thick as the leaf, medium line.
    Door,
    /// A window leaf: a single thin line.
    Window,
}

/// A hinged leaf drawn open at 90°, with its swing. `dir` is +1 when the
/// leaf closes towards growing `t` from its hinge, −1 otherwise.
fn draw_leaf(
    s: &mut String,
    o: &OpeningGeometry,
    hinge_t: f64,
    dir: f64,
    width: f64,
    leaf: Leaf,
    k: f64,
) {
    let h = o.point(hinge_t, o.swing_frame_face());
    let out = o.swing_dir();
    let along = o.u * dir;
    let tip_open = h + out * width;
    let tip_closed = h + along * width;
    match leaf {
        Leaf::Door => {
            let lt = LEAF_THICKNESS.min(width / 4.0);
            quad(
                s,
                [h, tip_open, tip_open + along * lt, h + along * lt],
                FRAME_LINE,
                k,
            );
        }
        Leaf::Window => line(s, h, tip_open, LAYER_LINE, k),
    }
    // Model counter-clockwise is screen counter-clockwise (sweep 0).
    let sweep = if out.cross(along) > 0.0 { 0 } else { 1 };
    let _ = writeln!(
        s,
        r#"<path d="M{} A{r},{r} 0 0,{sweep} {}" fill="none" stroke="{INK}" stroke-width="{}"/>"#,
        pt(tip_open),
        pt(tip_closed),
        num(LAYER_LINE * k),
        r = num(width)
    );
}

/// A sliding leaf hung on the face of the wall, drawn half open towards the
/// side it parks on, with an arrow.
fn draw_sliding_leaf(s: &mut String, o: &OpeningGeometry, c0: f64, c1: f64, k: f64) {
    let clear = c1 - c0;
    let lap = 0.05;
    let shift = if o.hinge_at_start() {
        -clear / 2.0
    } else {
        clear / 2.0
    };
    let face = o.swing_face();
    let sign = if o.swing_dir().dot(o.n) > 0.0 {
        1.0
    } else {
        -1.0
    };
    let (y0, y1) = (face + sign * 0.01, face + sign * (0.01 + LEAF_THICKNESS));
    quad(
        s,
        rect(o, c0 - lap + shift, c1 + lap + shift, y0, y1),
        FRAME_LINE,
        k,
    );
    // Arrow pointing where the leaf slides to open.
    let ya = face + sign * (0.01 + LEAF_THICKNESS + 0.08);
    let tm = (c0 + c1) / 2.0;
    let tip = tm + shift.signum() * clear * 0.3;
    let head = 0.06 * shift.signum();
    line(s, o.point(tm, ya), o.point(tip, ya), LAYER_LINE, k);
    line(
        s,
        o.point(tip, ya),
        o.point(tip - head, ya + 0.03),
        LAYER_LINE,
        k,
    );
    line(
        s,
        o.point(tip, ya),
        o.point(tip - head, ya - 0.03),
        LAYER_LINE,
        k,
    );
}

/// A window above the cut plane: its outline, dashed, over the wall.
fn draw_opening_above(s: &mut String, o: &OpeningGeometry, k: f64) {
    let _ = writeln!(
        s,
        r#"<path d="{}" fill="none" stroke="{INK}" stroke-width="{}" stroke-dasharray="{} {}"/>"#,
        path(&o.footprint()),
        num(VIEW_LINE * k),
        num(2.0 * k),
        num(1.0 * k)
    );
    let mid = (o.frame_exterior + o.frame_interior) / 2.0;
    let _ = writeln!(
        s,
        r#"<path d="M{} L{}" fill="none" stroke="{INK}" stroke-width="{}" stroke-dasharray="{} {}"/>"#,
        pt(o.point(o.t0, mid)),
        pt(o.point(o.t1, mid)),
        num(LAYER_LINE * k),
        num(2.0 * k),
        num(1.0 * k)
    );
}

fn num(v: f64) -> String {
    let v = (v * 1e6).round() / 1e6;
    if v == 0.0 {
        "0".into()
    } else {
        format!("{v}")
    }
}

fn pt(p: Point2) -> String {
    format!("{},{}", num(p.x), num(-p.y))
}

fn path(poly: &[Point2]) -> String {
    let mut d = String::new();
    for (i, p) in poly.iter().enumerate() {
        let _ = write!(d, "{}{} ", if i == 0 { "M" } else { "L" }, pt(*p));
    }
    d.push('Z');
    d
}

fn escape(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hatching_is_cut_to_any_polygon() {
        let p = |x, y| Point2::new(x, y);
        // An L: the line y = 1 crosses the lower leg, the gap and nothing else.
        let l = [
            p(0.0, 0.0),
            p(4.0, 0.0),
            p(4.0, 2.0),
            p(2.0, 2.0),
            p(2.0, 4.0),
            p(0.0, 4.0),
        ];
        assert_eq!(
            clip_segment(p(-1.0, 1.0), p(5.0, 1.0), &l),
            vec![(p(0.0, 1.0), p(4.0, 1.0))]
        );
        // The line y = 3 crosses the upper leg only.
        assert_eq!(
            clip_segment(p(-1.0, 3.0), p(5.0, 3.0), &l),
            vec![(p(0.0, 3.0), p(2.0, 3.0))]
        );
        // Through the notch of a U: two pieces.
        let u = [
            p(0.0, 0.0),
            p(6.0, 0.0),
            p(6.0, 4.0),
            p(4.0, 4.0),
            p(4.0, 2.0),
            p(2.0, 2.0),
            p(2.0, 4.0),
            p(0.0, 4.0),
        ];
        assert_eq!(clip_segment(p(-1.0, 3.0), p(7.0, 3.0), &u).len(), 2);
        // A zigzag cut to a band stays one continuous line inside it.
        let band = [p(0.0, 0.0), p(2.0, 0.0), p(2.0, 1.0), p(0.0, 1.0)];
        let zz = [
            p(-1.0, 0.2),
            p(0.5, 0.8),
            p(1.0, 0.2),
            p(1.5, 0.8),
            p(3.0, 0.2),
        ];
        let d = clip_polyline(&zz, &band);
        assert_eq!(d.matches('M').count(), 1, "{d}");
        assert!(hatch_lines(&band, &[45.0], 0.3).split('M').count() > 3);
    }
    use archisim_core::{Layer, NewSlab, NewWall, ReferenceLine, SlabFunction, WallFunction};

    #[test]
    fn renders_scaled_svg() {
        let mut m = Model::new("Casa <A&B>");
        let lvl = m.add_level("Planta baja", 0.0);
        let mat = m.add_material("Ladrillo", "brick", [230, 200, 190], CutPattern::Diagonal);
        let ins = m.add_material(
            "Lana mineral",
            "insulation",
            [250, 245, 200],
            CutPattern::Insulation,
        );
        let t = m
            .add_wall_type(
                "Fachada",
                WallFunction::Exterior,
                vec![
                    Layer {
                        material: mat,
                        thickness: 0.115,
                        function: LayerFunction::Structure,
                    },
                    Layer {
                        material: ins,
                        thickness: 0.05,
                        function: LayerFunction::Insulation,
                    },
                ],
            )
            .unwrap();
        m.add_wall(NewWall {
            wall_type: t,
            level: lvl,
            start: Point2::new(0.0, 0.0),
            end: Point2::new(5.0, 0.0),
            reference: ReferenceLine::Center,
            height: 3.0,
            base_offset: 0.0,
        })
        .unwrap();
        let svg = render_plan(&m, lvl, &PlanOptions::default());
        assert!(svg.starts_with("<svg"));
        assert!(svg.contains("id=\"cut-0-0-0\""));
        assert!(!svg.contains("clip-path="), "hatching is real geometry");
        assert!(svg.contains("Casa &lt;A&amp;B&gt;"));
        assert!(!svg.contains(" A0.8"), "no door yet");
        // 5 m at 1:50 = 100 mm plus two 15 mm margins.
        assert!(svg.contains("width=\"130.00mm\""), "{}", &svg[..200]);

        // An on-screen view draws exactly the requested region, with no
        // title or paper background, and the same poché.
        let screen = render_plan(
            &m,
            lvl,
            &PlanOptions {
                viewport: Some((Point2::new(-1.0, -2.0), Point2::new(3.0, 1.0))),
                ..PlanOptions::default()
            },
        );
        assert!(
            screen.contains("viewBox=\"-1 -1 4 3\""),
            "{}",
            &screen[..200]
        );
        assert!(screen.contains("width=\"80.00mm\" height=\"60.00mm\""));
        assert!(!screen.contains("<text"));
        assert!(!screen.contains("fill=\"white\""));
        assert!(screen.contains("id=\"cut-0-0-0\""));

        // A door cuts the wall in two pieces and draws its swing; a high
        // window above the cut plane is dashed and does not cut the wall.
        let wood = m.add_material("Madera", "wood", [180, 140, 90], CutPattern::Solid);
        let mut kind = |kind, width, height| {
            m.add_opening_type(archisim_core::NewOpeningType {
                name: "T".into(),
                kind,
                width,
                height,
                frame_width: 0.05,
                frame_depth: 0.07,
                frame_material: wood,
                panel_material: wood,
            })
            .unwrap()
        };
        let door = kind(OpeningKind::Door(DoorOperation::SingleSwing), 0.9, 2.1);
        let high = kind(OpeningKind::Window(WindowOperation::Fixed), 0.6, 0.6);
        let wall = m.walls().next().unwrap().id;
        for (t, along, sill) in [(door, 1.5, 0.0), (high, 3.5, 1.8)] {
            m.add_opening(archisim_core::NewOpening {
                opening_type: t,
                wall,
                along,
                sill,
                opens_to: archisim_core::WallSide::Interior,
                hand: archisim_core::Hand::Left,
                setback: None,
            })
            .unwrap();
        }
        let svg = render_plan(&m, lvl, &PlanOptions::default());
        // Leaf width 0.9 − 2 × 0.05 = 0.8: one swing arc of that radius.
        assert_eq!(svg.matches(" A0.8,0.8 ").count(), 1);
        // Two pieces per layer instead of one.
        assert!(svg.contains("cut-0-1-1"));
        assert!(!svg.contains("cut-0-1-2"));
        assert_eq!(svg.matches("stroke-dasharray").count(), 2);
    }

    #[test]
    fn facade_jambs_return_and_casements_swing() {
        let mut m = Model::new("Jambas");
        let lvl = m.add_level("PB", 0.0);
        let mat = |m: &mut Model, name: &str, pattern| {
            m.add_material(name, "x", [200, 200, 200], pattern)
        };
        let render = mat(&mut m, "Monocapa", CutPattern::Dots);
        let brick = mat(&mut m, "Ladrillo", CutPattern::Diagonal);
        let wool = mat(&mut m, "Lana", CutPattern::Insulation);
        let inner = mat(&mut m, "LHD", CutPattern::Diagonal);
        let glass = mat(&mut m, "Vidrio", CutPattern::Solid);
        let layer = |material, thickness, function| Layer {
            material,
            thickness,
            function,
        };
        let facade = m
            .add_wall_type(
                "Fachada",
                WallFunction::Exterior,
                vec![
                    layer(render, 0.015, LayerFunction::Finish),
                    layer(brick, 0.115, LayerFunction::Structure),
                    layer(wool, 0.05, LayerFunction::Insulation),
                    layer(inner, 0.07, LayerFunction::Substrate),
                ],
            )
            .unwrap();
        let partition = m
            .add_wall_type(
                "Tabique",
                WallFunction::Interior,
                vec![layer(inner, 0.07, LayerFunction::Substrate)],
            )
            .unwrap();
        let detail = JambDetail::for_type(m.wall_type(facade).unwrap());
        assert_eq!(detail.turn, Some((1, 2..3)));
        assert_eq!(detail.wrap, Some(0));
        let none = JambDetail::for_type(m.wall_type(partition).unwrap());
        assert_eq!(
            none,
            JambDetail {
                turn: None,
                wrap: None
            }
        );

        let wall = m
            .add_wall(NewWall {
                wall_type: facade,
                level: lvl,
                start: Point2::new(0.0, 0.0),
                end: Point2::new(5.0, 0.0),
                reference: ReferenceLine::ExteriorFace,
                height: 3.0,
                base_offset: 0.0,
            })
            .unwrap();
        let window = m
            .add_opening_type(archisim_core::NewOpeningType {
                name: "V".into(),
                kind: OpeningKind::Window(WindowOperation::DoubleCasement),
                width: 1.2,
                height: 1.2,
                frame_width: 0.06,
                frame_depth: 0.07,
                frame_material: brick,
                panel_material: glass,
            })
            .unwrap();
        m.add_opening(archisim_core::NewOpening {
            opening_type: window,
            wall,
            along: 2.5,
            sill: 0.9,
            opens_to: archisim_core::WallSide::Interior,
            hand: archisim_core::Hand::Left,
            setback: Some(0.13),
        })
        .unwrap();
        let svg = render_plan(&m, lvl, &PlanOptions::default());
        // The brick returns and the render wraps at both jambs.
        assert_eq!(svg.matches("id=\"cut-0-turn-").count(), 2);
        assert_eq!(svg.matches("id=\"cut-0-wrap-").count(), 2);
        // Two leaves of (1.2 − 2 × 0.06) / 2 = 0.54 m swing into the room.
        assert_eq!(svg.matches(" A0.54,0.54 ").count(), 2);
    }

    #[test]
    fn slabs_follow_the_cut_plane() {
        let mut m = Model::new("Forjados");
        let lvl = m.add_level("Planta baja", 0.0);
        let hormigon = m.add_material("Hormigón", "concrete", [200, 200, 200], CutPattern::Cross);
        let st = m
            .add_slab_type(
                "Losa 20",
                SlabFunction::Floor,
                vec![Layer {
                    material: hormigon,
                    thickness: 0.2,
                    function: LayerFunction::Structure,
                }],
            )
            .unwrap();
        let square = |x: f64| {
            vec![
                Point2::new(x, 0.0),
                Point2::new(x + 4.0, 0.0),
                Point2::new(x + 4.0, 4.0),
                Point2::new(x, 4.0),
            ]
        };
        for (x, top) in [(0.0, 0.0), (5.0, 1.2), (10.0, 2.5)] {
            m.add_slab(NewSlab {
                slab_type: st,
                level: lvl,
                outline: square(x),
                top_offset: top,
            })
            .unwrap();
        }
        let svg = render_plan(&m, lvl, &PlanOptions::default());
        // Below: cover fill. Cut: structural fill with cross hatch. Above: dashed.
        assert!(svg.contains(SLAB_COVER));
        assert!(svg.contains("fill=\"rgb(200,200,200)\""));
        assert!(svg.contains("cut-slab-1"));
        assert!(svg.contains("stroke-dasharray"));
        // The drawing spans the three slabs: 14 m at 1:50 + margins.
        assert!(svg.contains("width=\"310.00mm\""), "{}", &svg[..200]);
    }
}
