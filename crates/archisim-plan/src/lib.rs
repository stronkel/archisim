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
//! - insulation is drawn with the usual zigzag, clipped to its layer;
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

use std::fmt::Write as _;

use archisim_core::{
    CutPattern, DoorOperation, LayerFunction, LevelId, Model, OpeningKind, Point2, Slab,
    WindowOperation, EPSILON,
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
}

impl Default for PlanOptions {
    fn default() -> Self {
        Self {
            scale: 50.0,
            margin_mm: 15.0,
            show_reference_lines: false,
            cut_plane: 1.1,
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
    for p in all_points {
        min = Point2::new(min.x.min(p.x), min.y.min(p.y));
        max = Point2::new(max.x.max(p.x), max.y.max(p.y));
    }
    if geos.is_empty() && slabs.is_empty() {
        min = Point2::new(0.0, 0.0);
        max = Point2::new(1.0, 1.0);
    }
    let margin = options.margin_mm * k;
    let title_space = 4.0 * TEXT_SIZE * k;
    // SVG y grows downwards: plot (x, -y).
    let vb_x = min.x - margin;
    let vb_y = -max.y - margin;
    let vb_w = (max.x - min.x) + 2.0 * margin;
    let vb_h = (max.y - min.y) + 2.0 * margin + title_space;

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
    let _ = writeln!(
        s,
        r#"<rect x="{}" y="{}" width="{}" height="{}" fill="white"/>"#,
        num(vb_x),
        num(vb_y),
        num(vb_w),
        num(vb_h)
    );

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
        draw_layers(&mut s, model, g, gi, &pieces[gi], k);
    }
    s.push_str("</g>\n<g id=\"cut-lines\" fill=\"none\" stroke-linecap=\"square\">\n");
    for (g, p) in geos.iter().zip(&pieces) {
        draw_cut_lines(&mut s, g, p, k);
    }
    s.push_str("</g>\n<g id=\"openings\" stroke-linejoin=\"miter\">\n");
    for o in openings.iter().flatten() {
        match opening_view(o, options.cut_plane) {
            OpeningView::Cut => draw_opening(&mut s, o, k),
            OpeningView::Above => draw_opening_above(&mut s, o, k),
            OpeningView::Below => {}
        }
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
    s.push_str("</svg>\n");
    s
}

fn draw_layers(
    s: &mut String,
    model: &Model,
    g: &WallGeometry,
    gi: usize,
    pieces: &[WallPiece],
    k: f64,
) {
    let mut high = g.exterior_offset;
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

    for ((li, band), layer) in g.layers.iter().enumerate().zip(&wtype.layers) {
        let Some(mat) = model.material(band.material) else {
            continue;
        };
        let thickness = layer.thickness;
        let low = high - thickness;
        for (pi, piece) in pieces.iter().enumerate() {
            let polygon = clip_span(&band.polygon, a, u, piece.from, piece.to);
            if polygon.is_empty() {
                continue;
            }
            let d = path(&polygon);
            let [r, gg, bb] = mat.cut_color;
            // A hairline in the fill colour hides anti-aliasing seams at mitres.
            let _ = writeln!(
                s,
                r#"<path d="{d}" fill="rgb({r},{gg},{bb})" stroke="rgb({r},{gg},{bb})" stroke-width="{}"/>"#,
                num(HATCH_LINE * k)
            );

            // Hatching is real geometry clipped to the layer (not SVG patterns),
            // so it prints identically everywhere and carries over to PDF/DXF.
            let clip = format!("clip-{gi}-{li}-{pi}");
            let _ = writeln!(s, r#"<clipPath id="{clip}"><path d="{d}"/></clipPath>"#);
            let sp = HATCH_SPACING * k;
            let hatch = match mat.cut_pattern {
                CutPattern::Diagonal => hatch_lines(&polygon, &[45.0], sp),
                CutPattern::Cross => hatch_lines(&polygon, &[45.0, 135.0], sp),
                _ => String::new(),
            };
            if !hatch.is_empty() {
                let _ = writeln!(
                    s,
                    r#"<path d="{hatch}" fill="none" stroke="{INK}" stroke-width="{}" clip-path="url(#{clip})"/>"#,
                    num(HATCH_LINE * k)
                );
            }
            if mat.cut_pattern == CutPattern::Dots {
                let dots = hatch_dots(&polygon, sp, HATCH_LINE * k * 1.2);
                let _ = writeln!(
                    s,
                    r#"<path d="{dots}" fill="{INK}" clip-path="url(#{clip})"/>"#
                );
            }

            if mat.cut_pattern == CutPattern::Insulation
                || band.function == LayerFunction::Insulation
            {
                // Zigzag along the layer, overshooting the ends and clipped to it.
                let mid = (high + low) / 2.0;
                let amp = thickness / 2.0;
                let step = thickness.max(0.01);
                let mut zz = String::new();
                let mut t = -wall_t * 2.0;
                let mut up = true;
                while t <= len + wall_t * 2.0 {
                    let off = mid + if up { amp } else { -amp };
                    let p = a + u * t + n * off;
                    let _ = write!(zz, "{}{} ", if zz.is_empty() { "M" } else { "L" }, pt(p));
                    t += step / 2.0;
                    up = !up;
                }
                let _ = writeln!(
                    s,
                    r#"<path d="{}" fill="none" stroke="{INK}" stroke-width="{}" clip-path="url(#{clip})"/>"#,
                    zz.trim_end(),
                    num(HATCH_LINE * k)
                );
            }
        }
        high = low;
    }

    // Thin boundaries between layers, along the faces only: the edge of each
    // band (except the last) that lies on its interior side.
    let mut off = g.exterior_offset;
    for (band, layer) in g
        .layers
        .iter()
        .zip(&wtype.layers)
        .take(g.layers.len().saturating_sub(1))
    {
        off -= layer.thickness;
        for piece in pieces {
            let polygon = clip_span(&band.polygon, a, u, piece.from, piece.to);
            if let Some((p, q)) = face_edge_at(&polygon, a, n, off) {
                let _ = writeln!(
                    s,
                    r#"<path d="M{} L{}" stroke="{INK}" stroke-width="{}" fill="none"/>"#,
                    pt(p),
                    pt(q),
                    num(LAYER_LINE * k)
                );
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
            let _ = writeln!(s, r#"<path d="{d}" fill="rgb({r},{g},{b})"/>"#);
            let angles: &[f64] = match pattern {
                CutPattern::Diagonal => &[45.0],
                CutPattern::Cross => &[45.0, 135.0],
                _ => &[],
            };
            if !angles.is_empty() {
                let clip = format!("clip-slab-{si}");
                let _ = writeln!(s, r#"<clipPath id="{clip}"><path d="{d}"/></clipPath>"#);
                let _ = writeln!(
                    s,
                    r#"<path d="{}" fill="none" stroke="{INK}" stroke-width="{}" clip-path="url(#{clip})"/>"#,
                    hatch_lines(&slab.outline, angles, HATCH_SPACING * k),
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

/// Parallel hatch lines covering a polygon's bounding box, anchored to the
/// project origin so that hatching continues seamlessly across elements.
/// Angles are in degrees, measured counter-clockwise from the X axis.
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
            let p = m * c + dir * t0;
            let q = m * c + dir * t1;
            let _ = write!(d, "M{} L{} ", pt(p), pt(q));
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
            if contains(poly, c) {
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
                    draw_leaf(s, o, hinge, dir, c1 - c0, k);
                }
                DoorOperation::DoubleSwing => {
                    let w = (c1 - c0) / 2.0;
                    draw_leaf(s, o, c0, 1.0, w, k);
                    draw_leaf(s, o, c1, -1.0, w, k);
                }
                DoorOperation::Sliding => draw_sliding_leaf(s, o, c0, c1, k),
            }
        }
        OpeningKind::Window(op) => {
            let mid = (fe + fi) / 2.0;
            // Sill wall seen below the cut: sill board outside, edge inside.
            if o.sill > EPSILON {
                let out = o.exterior_offset + SILL_PROJECTION;
                quad(s, rect(o, o.t0 - 0.02, o.t1 + 0.02, fe, out), LAYER_LINE, k);
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
                WindowOperation::Fixed | WindowOperation::SingleCasement => {
                    line(s, o.point(c0, mid), o.point(c1, mid), LAYER_LINE, k);
                }
                WindowOperation::DoubleCasement => {
                    let tm = (o.t0 + o.t1) / 2.0;
                    line(s, o.point(c0, mid), o.point(c1, mid), LAYER_LINE, k);
                    line(s, o.point(tm, fi), o.point(tm, fe), FRAME_LINE, k);
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

/// A hinged leaf drawn open at 90°, with its swing. `dir` is +1 when the
/// leaf closes towards growing `t` from its hinge, −1 otherwise.
fn draw_leaf(s: &mut String, o: &OpeningGeometry, hinge_t: f64, dir: f64, width: f64, k: f64) {
    let h = o.point(hinge_t, o.swing_frame_face());
    let out = o.swing_dir();
    let along = o.u * dir;
    let lt = LEAF_THICKNESS.min(width / 4.0);
    let tip_open = h + out * width;
    let tip_closed = h + along * width;
    quad(
        s,
        [h, tip_open, tip_open + along * lt, h + along * lt],
        FRAME_LINE,
        k,
    );
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
        assert!(svg.contains("clip-path=\"url(#clip-0-0-0)\""));
        assert!(svg.contains("clip-path="));
        assert!(svg.contains("Casa &lt;A&amp;B&gt;"));
        assert!(!svg.contains(" A0.8"), "no door yet");
        // 5 m at 1:50 = 100 mm plus two 15 mm margins.
        assert!(svg.contains("width=\"130.00mm\""), "{}", &svg[..200]);

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
        assert!(svg.contains("clip-0-1-1"));
        assert!(!svg.contains("clip-0-1-2"));
        assert_eq!(svg.matches("stroke-dasharray").count(), 2);
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
        assert!(svg.contains("clip-slab-1"));
        assert!(svg.contains("stroke-dasharray"));
        // The drawing spans the three slabs: 14 m at 1:50 + margins.
        assert!(svg.contains("width=\"310.00mm\""), "{}", &svg[..200]);
    }
}
