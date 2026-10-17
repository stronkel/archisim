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

use std::fmt::Write as _;

use archisim_core::{CutPattern, LayerFunction, LevelId, Model, Point2, Slab};
use archisim_geom::{wall_geometries, EndCut, WallGeometry};

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

    s.push_str("<g id=\"poche\">\n");
    for (gi, g) in geos.iter().enumerate() {
        draw_layers(&mut s, model, g, gi, k);
    }
    s.push_str("</g>\n<g id=\"cut-lines\" fill=\"none\" stroke-linecap=\"square\">\n");
    for g in &geos {
        draw_cut_lines(&mut s, g, k);
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

fn draw_layers(s: &mut String, model: &Model, g: &WallGeometry, gi: usize, k: f64) {
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
        let d = path(&band.polygon);
        let [r, gg, bb] = mat.cut_color;
        // A hairline in the fill colour hides anti-aliasing seams at mitres.
        let _ = writeln!(
            s,
            r#"<path d="{d}" fill="rgb({r},{gg},{bb})" stroke="rgb({r},{gg},{bb})" stroke-width="{}"/>"#,
            num(HATCH_LINE * k)
        );

        // Hatching is real geometry clipped to the layer (not SVG patterns),
        // so it prints identically everywhere and carries over to PDF/DXF.
        let clip = format!("clip-{gi}-{li}");
        let _ = writeln!(s, r#"<clipPath id="{clip}"><path d="{d}"/></clipPath>"#);
        let sp = HATCH_SPACING * k;
        let hatch = match mat.cut_pattern {
            CutPattern::Diagonal => hatch_lines(&band.polygon, &[45.0], sp),
            CutPattern::Cross => hatch_lines(&band.polygon, &[45.0, 135.0], sp),
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
            let dots = hatch_dots(&band.polygon, sp, HATCH_LINE * k * 1.2);
            let _ = writeln!(
                s,
                r#"<path d="{dots}" fill="{INK}" clip-path="url(#{clip})"/>"#
            );
        }

        if mat.cut_pattern == CutPattern::Insulation || band.function == LayerFunction::Insulation {
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
        if let Some((p, q)) = face_edge_at(&band.polygon, a, n, off) {
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

fn draw_cut_lines(s: &mut String, g: &WallGeometry, k: f64) {
    // Outline edges are, in order: face, end cut, face, start cut.
    let o = &g.outline;
    if o.len() != 4 {
        return;
    }
    let mut d = String::new();
    let mut edge = |i: usize| {
        let _ = write!(d, "M{} L{} ", pt(o[i]), pt(o[(i + 1) % 4]));
    };
    edge(0);
    edge(2);
    if g.end_cut == EndCut::Square {
        edge(1);
    }
    if g.start_cut == EndCut::Square {
        edge(3);
    }
    let _ = writeln!(
        s,
        r#"<path d="{}" stroke="{INK}" stroke-width="{}"/>"#,
        d.trim_end(),
        num(CUT_LINE * k)
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
        assert!(svg.contains("clip-path=\"url(#clip-0-0)\""));
        assert!(svg.contains("clip-path="));
        assert!(svg.contains("Casa &lt;A&amp;B&gt;"));
        // 5 m at 1:50 = 100 mm plus two 15 mm margins.
        assert!(svg.contains("width=\"130.00mm\""), "{}", &svg[..200]);
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
