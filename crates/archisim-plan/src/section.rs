//! Sections and elevations, drawn from the same 3D solids as the 3D view
//! and the IFC.
//!
//! - **Cut**: where the section's vertical plane crosses a solid, its cut
//!   is filled with the material's colour and hatch (the poché), outlined
//!   with the heavy cut line; where two layers of a wall or slab meet
//!   inside the cut, a thin line, as in plan.
//! - **Beyond**: what the section sees beyond its plane, up to its depth,
//!   is drawn with its visible edges only (hidden lines removed): faces are
//!   painted far to near, each white with the edges of its solid that are
//!   corners (not the diagonals of the triangles), so nearer faces cover
//!   what lies behind them.
//! - **Levels**: a thin dash-dot line at each level, with its name and
//!   elevation, as is usual in Spanish sections.
//!
//! Coordinates of the drawing: `x` along the section line from its start,
//! `y` the height above project zero (metres), so an elevation can be
//! measured like a plan.

use std::collections::HashMap;
use std::fmt::Write as _;

use archisim_core::{MaterialId, Model, NewSection, Point2, SectionKind, SectionLine};
use archisim_geom::solid::{model_solids, P3};

use crate::lettering::{text_path, Align};
use crate::{
    annotate, fill_cut, num, path, pt, PlanOptions, CUT_LINE, INK, LAYER_LINE, TEXT_SIZE, VIEW_LINE,
};

/// What a section draws, worked out once for a model and a section line;
/// [`render_section`] writes it to SVG for any view of it.
#[derive(Debug, Clone, Default)]
pub struct SectionDrawing {
    /// Cut pieces: material, the loops of its cut, and for each loop which
    /// of its edges are faces of the element (heavy) or joins between
    /// layers (thin).
    cuts: Vec<CutPiece>,
    /// Faces beyond the cut, far to near.
    faces: Vec<Face>,
    /// Levels: name and elevation.
    levels: Vec<(String, f64)>,
    pub lo: Point2,
    pub hi: Point2,
    pub title: String,
}

/// A cut solid: its material, loops, and which loop edges are heavy.
type CutPiece = (MaterialId, Vec<Vec<Point2>>, Vec<Vec<bool>>);
/// An edge as the keys of its two ends.
type EdgeKey = ((i64, i64, i64), (i64, i64, i64));

#[derive(Debug, Clone)]
struct Face {
    poly: Vec<Point2>,
    fill: [u8; 3],
    edges: Vec<(Point2, Point2)>,
}

/// Faces are split until no edge is longer than this, so that painting
/// them far to near hides correctly (metres).
const MAX_EDGE: f64 = 0.6;
/// How far ahead of their place edges are painted (metres).
const EDGE_LEAD: f64 = 0.03;
/// Edges between faces turning more than this are drawn.
const CREASE_COS: f64 = 0.966; // 15°

fn sub(a: P3, b: P3) -> P3 {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}
fn cross(a: P3, b: P3) -> P3 {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}
fn dot(a: P3, b: P3) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}
fn len(a: P3) -> f64 {
    dot(a, a).sqrt()
}
fn lerp(a: P3, b: P3, t: f64) -> P3 {
    [
        a[0] + (b[0] - a[0]) * t,
        a[1] + (b[1] - a[1]) * t,
        a[2] + (b[2] - a[2]) * t,
    ]
}

/// A key for a point, to match ends of edges and segments.
fn key(p: P3) -> (i64, i64, i64) {
    let q = |v: f64| (v * 1e5).round() as i64;
    (q(p[0]), q(p[1]), q(p[2]))
}

fn key2(p: Point2) -> (i64, i64) {
    let q = |v: f64| (v * 1e5).round() as i64;
    (q(p.x), q(p.y))
}

/// A triangle with, for each of its edges (0: a–b, 1: b–c, 2: c–a),
/// whether it is a corner of its solid.
type Tri = ([P3; 3], [bool; 3]);

/// Splits a triangle on its longest edge until every edge is short.
fn subdivide(t: Tri, out: &mut Vec<Tri>, depth: usize) {
    let (p, f) = t;
    let l = [
        len(sub(p[1], p[0])),
        len(sub(p[2], p[1])),
        len(sub(p[0], p[2])),
    ];
    let k = (0..3).max_by(|a, b| l[*a].total_cmp(&l[*b])).unwrap_or(0);
    if l[k] <= MAX_EDGE || depth > 10 {
        out.push(t);
        return;
    }
    // Rotate so that the edge to split is the first one (a–b).
    let (a, b, c) = (p[k], p[(k + 1) % 3], p[(k + 2) % 3]);
    let (fab, fbc, fca) = (f[k], f[(k + 1) % 3], f[(k + 2) % 3]);
    let m = lerp(a, b, 0.5);
    subdivide(([a, m, c], [fab, false, fca]), out, depth + 1);
    subdivide(([m, b, c], [fab, fbc, false]), out, depth + 1);
}

/// Works out what a section draws.
pub fn section_drawing(model: &Model, sec: &SectionLine) -> SectionDrawing {
    let mut d = SectionDrawing {
        title: sec.title(),
        ..Default::default()
    };
    let (Some(u), Some(look)) = (sec.along(), sec.looks()) else {
        return d;
    };
    let o = sec.start;
    let view: P3 = [look.x, look.y, 0.0];
    // A hair in front of the line, so faces lying on it are not cut.
    let plane = 1e-5;
    let to2 = |p: P3| {
        let rel = Point2::new(p[0] - o.x, p[1] - o.y);
        Point2::new(rel.dot(u), p[2])
    };
    let depth_of = |p: P3| Point2::new(p[0] - o.x, p[1] - o.y).dot(look);

    let mut lo = Point2::new(f64::INFINITY, f64::INFINITY);
    let mut hi = Point2::new(f64::NEG_INFINITY, f64::NEG_INFINITY);
    fn grow_to(lo: &mut Point2, hi: &mut Point2, p: Point2) {
        *lo = Point2::new(lo.x.min(p.x), lo.y.min(p.y));
        *hi = Point2::new(hi.x.max(p.x), hi.y.max(p.y));
    }

    let mut faces: Vec<(f64, Face)> = Vec::new();
    for solid in model_solids(model) {
        let glass = model
            .material(solid.material)
            .is_some_and(|m| m.category == "glass");
        // Corners of the solid: edges of one face only, or between faces
        // that turn.
        let normals: Vec<P3> = solid
            .triangles
            .iter()
            .map(|t| {
                let n = cross(sub(t[1], t[0]), sub(t[2], t[0]));
                let l = len(n).max(1e-30);
                [n[0] / l, n[1] / l, n[2] / l]
            })
            .collect();
        let mut edge_faces: HashMap<EdgeKey, Vec<usize>> = HashMap::new();
        for (i, t) in solid.triangles.iter().enumerate() {
            for e in 0..3 {
                let (a, b) = (key(t[e]), key(t[(e + 1) % 3]));
                let k = if a <= b { (a, b) } else { (b, a) };
                edge_faces.entry(k).or_default().push(i);
            }
        }
        let is_corner = |a: P3, b: P3, me: usize| {
            let (a, b) = (key(a), key(b));
            let k = if a <= b { (a, b) } else { (b, a) };
            match edge_faces.get(&k) {
                Some(list) if list.len() >= 2 => list
                    .iter()
                    .any(|&j| j != me && dot(normals[j], normals[me]) < CREASE_COS),
                _ => true,
            }
        };

        // The cut: one segment per triangle crossing the plane, oriented
        // with the solid's outside on its right.
        let mut segments: Vec<(Point2, Point2)> = Vec::new();
        for (i, t) in solid.triangles.iter().enumerate() {
            let s: Vec<f64> = t.iter().map(|p| depth_of(*p) - plane).collect();
            let mut pts: Vec<P3> = Vec::new();
            for e in 0..3 {
                let (a, b) = (e, (e + 1) % 3);
                if (s[a] < 0.0) != (s[b] < 0.0) {
                    let k = s[a] / (s[a] - s[b]);
                    pts.push(lerp(t[a], t[b], k));
                }
            }
            if pts.len() != 2 {
                continue;
            }
            let (p, q) = (to2(pts[0]), to2(pts[1]));
            if p.distance(q) < 1e-7 {
                continue;
            }
            let n = normals[i];
            let n2 = Point2::new(n[0] * u.x + n[1] * u.y, n[2]);
            let dir = q - p;
            let right = Point2::new(dir.y, -dir.x);
            if right.dot(n2) >= 0.0 {
                segments.push((p, q));
            } else {
                segments.push((q, p));
            }
        }
        if !segments.is_empty() {
            let (loops, solid_edges) = chain(segments);
            for l in &loops {
                for p in l {
                    grow_to(&mut lo, &mut hi, *p);
                }
            }
            d.cuts.push((solid.material, loops, solid_edges));
        }

        // Beyond the cut: faces seen from the section, clipped to its
        // depth.
        for (i, t) in solid.triangles.iter().enumerate() {
            let n = normals[i];
            if dot(n, view) > -1e-3 {
                continue;
            }
            let ds: Vec<f64> = t.iter().map(|p| depth_of(*p)).collect();
            if ds.iter().all(|v| *v < plane) || ds.iter().all(|v| *v > sec.depth) {
                continue;
            }
            let flags = [
                is_corner(t[0], t[1], i),
                is_corner(t[1], t[2], i),
                is_corner(t[2], t[0], i),
            ];
            let mut small = Vec::new();
            subdivide((*t, flags), &mut small, 0);
            for (p, f) in small {
                let Some((poly, edges)) = clip(&p, &f, &depth_of, plane, sec.depth) else {
                    continue;
                };
                let centre = poly.iter().map(|q| depth_of(*q)).sum::<f64>() / poly.len() as f64;
                let poly2: Vec<Point2> = poly.iter().map(|q| to2(*q)).collect();
                for q in &poly2 {
                    grow_to(&mut lo, &mut hi, *q);
                }
                faces.push((
                    centre,
                    Face {
                        poly: poly2,
                        fill: if glass { GLASS } else { [255, 255, 255] },
                        edges: Vec::new(),
                    },
                ));
                // Each edge is painted on its own, a little ahead of where
                // it is: faces level with it (the next layer, the next
                // triangle of the same face) do not cover it.
                for (a, b) in edges {
                    let at = depth_of(a).min(depth_of(b)) - EDGE_LEAD;
                    faces.push((
                        at,
                        Face {
                            poly: Vec::new(),
                            fill: [0, 0, 0],
                            edges: vec![(to2(a), to2(b))],
                        },
                    ));
                }
            }
        }
    }
    faces.sort_by(|a, b| b.0.total_cmp(&a.0));
    d.faces = faces.into_iter().map(|(_, f)| f).collect();
    d.levels = model
        .levels()
        .iter()
        .map(|l| (l.name.clone(), l.elevation))
        .collect();
    if lo.x.is_finite() {
        for (_, z) in &d.levels {
            let x = lo.x;
            grow_to(&mut lo, &mut hi, Point2::new(x, *z));
        }
        d.lo = lo;
        d.hi = hi;
    } else {
        d.lo = Point2::new(0.0, 0.0);
        d.hi = Point2::new(o.distance(sec.end), 3.0);
    }
    d
}

/// Glass seen beyond the cut.
const GLASS: [u8; 3] = [222, 236, 243];

/// Clips a triangle to `plane <= depth <= far`; returns the polygon and
/// the edges of it that are corners of the solid.
#[allow(clippy::type_complexity)]
fn clip(
    p: &[P3; 3],
    f: &[bool; 3],
    depth_of: &dyn Fn(P3) -> f64,
    plane: f64,
    far: f64,
) -> Option<(Vec<P3>, Vec<(P3, P3)>)> {
    // Polygon as (point, flag of the edge from this point to the next).
    let mut poly: Vec<(P3, bool)> = (0..3).map(|i| (p[i], f[i])).collect();
    for (limit, keep_above) in [(plane, true), (far, false)] {
        let inside = |q: P3| {
            let v = depth_of(q);
            if keep_above {
                v >= limit
            } else {
                v <= limit
            }
        };
        let mut out: Vec<(P3, bool)> = Vec::new();
        let n = poly.len();
        for i in 0..n {
            let (a, fa) = poly[i];
            let (b, _) = poly[(i + 1) % n];
            let (ia, ib) = (inside(a), inside(b));
            if ia {
                out.push((a, fa));
            }
            if ia != ib {
                let da = depth_of(a) - limit;
                let db = depth_of(b) - limit;
                let t = da / (da - db);
                let m = lerp(a, b, t);
                // Leaving: the new edge along the limit is not drawn;
                // entering: the rest of the original edge keeps its flag.
                out.push((m, if ia { false } else { fa }));
            }
        }
        if out.len() < 3 {
            return None;
        }
        poly = out;
    }
    let n = poly.len();
    let edges = (0..n)
        .filter(|i| poly[*i].1)
        .map(|i| (poly[i].0, poly[(i + 1) % n].0))
        .collect();
    Some((poly.into_iter().map(|(q, _)| q).collect(), edges))
}

/// Chains oriented segments into closed loops. Segments that end where no
/// other starts (a layer whose inner faces are shared with the next layer)
/// are joined to the nearest free start; those joins are marked `false`
/// (a thin line between layers), the segments themselves `true`.
fn chain(segments: Vec<(Point2, Point2)>) -> (Vec<Vec<Point2>>, Vec<Vec<bool>>) {
    let mut by_start: HashMap<(i64, i64), Vec<usize>> = HashMap::new();
    for (i, (a, _)) in segments.iter().enumerate() {
        by_start.entry(key2(*a)).or_default().push(i);
    }
    let ends: std::collections::HashSet<(i64, i64)> =
        segments.iter().map(|(_, b)| key2(*b)).collect();
    // Open chains are started at their beginning (a start no segment ends
    // at), so that none is split in two.
    let mut order: Vec<usize> = (0..segments.len())
        .filter(|i| !ends.contains(&key2(segments[*i].0)))
        .collect();
    order.extend((0..segments.len()).filter(|i| ends.contains(&key2(segments[*i].0))));
    let mut used = vec![false; segments.len()];
    let mut loops = Vec::new();
    let mut flags = Vec::new();
    for first in order {
        if used[first] {
            continue;
        }
        let mut pts = vec![segments[first].0];
        let mut fl = Vec::new();
        let mut cur = first;
        let start_key = key2(segments[first].0);
        loop {
            used[cur] = true;
            let end = segments[cur].1;
            fl.push(true);
            if key2(end) == start_key {
                break;
            }
            let next = by_start
                .get(&key2(end))
                .and_then(|v| v.iter().copied().find(|j| !used[*j]));
            match next {
                Some(j) => {
                    pts.push(end);
                    cur = j;
                }
                None => {
                    // Jump to the nearest free start (or close the loop).
                    pts.push(end);
                    // Only to the start of another open chain: closed
                    // loops are whole on their own.
                    let jump = (0..segments.len())
                        .filter(|j| !used[*j] && !ends.contains(&key2(segments[*j].0)))
                        .map(|j| (segments[j].0.distance(end), j))
                        .min_by(|a, b| a.0.total_cmp(&b.0));
                    match jump {
                        Some((_, j)) => {
                            fl.push(false);
                            pts.push(segments[j].0);
                            cur = j;
                        }
                        _ => {
                            fl.push(false);
                            break;
                        }
                    }
                }
            }
        }
        if pts.len() >= 3 {
            // `fl[i]` is the edge from pts[i] to pts[i + 1]; the last one
            // closes the loop.
            fl.resize(pts.len(), false);
            loops.push(pts);
            flags.push(fl);
        }
    }
    (loops, flags)
}

/// Radius of the head of a section or elevation marker, on paper (mm).
pub const MARKER_RADIUS: f64 = 4.0;

/// The heads of a section marker (where its circles are) and the
/// direction its arrows point; an elevation has one head, in the middle.
pub fn marker_heads(sec: &SectionLine, k: f64) -> Vec<Point2> {
    let Some(u) = sec.along() else {
        return Vec::new();
    };
    let r = MARKER_RADIUS * k;
    match sec.kind {
        SectionKind::Section => vec![sec.start - u * r, sec.end + u * r],
        SectionKind::Elevation => vec![sec.start + (sec.end - sec.start) * 0.5],
    }
}

/// Points a plan must include to show the markers whole.
pub fn marker_points(model: &Model, k: f64) -> Vec<Point2> {
    let r = MARKER_RADIUS * k * 2.0;
    let mut v = Vec::new();
    for sec in model.sections() {
        v.push(sec.start);
        v.push(sec.end);
        for h in marker_heads(sec, k) {
            v.extend([h + Point2::new(r, r), h + Point2::new(-r, -r)]);
        }
    }
    v
}

/// Draws the section and elevation markers in a plan: a section, a
/// dash-dot line with heavy ends and, beyond each end, a circle with its
/// name and an arrow pointing where it looks; an elevation, a thin line
/// with one circle in its middle and its arrow.
pub fn draw_markers(s: &mut String, model: &Model, k: f64) {
    let r = MARKER_RADIUS * k;
    s.push_str("<g id=\"section-marks\" stroke-linejoin=\"round\">\n");
    for sec in model.sections() {
        let (Some(u), Some(look)) = (sec.along(), sec.looks()) else {
            continue;
        };
        let (a, b) = (sec.start, sec.end);
        match sec.kind {
            SectionKind::Section => {
                let _ = writeln!(
                    s,
                    r#"<path d="M{} L{}" fill="none" stroke="{INK}" stroke-width="{}" stroke-dasharray="{} {} {} {}"/>"#,
                    pt(a),
                    pt(b),
                    num(LAYER_LINE * k),
                    num(10.0 * k),
                    num(1.5 * k),
                    num(1.0 * k),
                    num(1.5 * k)
                );
                let end = (6.0 * k).min(a.distance(b) / 2.0);
                let _ = writeln!(
                    s,
                    r#"<path d="M{} L{} M{} L{}" fill="none" stroke="{INK}" stroke-width="{}"/>"#,
                    pt(a),
                    pt(a + u * end),
                    pt(b),
                    pt(b - u * end),
                    num(CUT_LINE * k)
                );
            }
            SectionKind::Elevation => {
                let _ = writeln!(
                    s,
                    r#"<path d="M{} L{}" fill="none" stroke="{INK}" stroke-width="{}" stroke-dasharray="{} {}"/>"#,
                    pt(a),
                    pt(b),
                    num(LAYER_LINE * k),
                    num(2.0 * k),
                    num(1.5 * k)
                );
            }
        }
        let label: String = match sec.kind {
            SectionKind::Section => sec.name.clone(),
            SectionKind::Elevation => sec.name.chars().take(1).collect(),
        };
        let th = 2.5 * k;
        for h in marker_heads(sec, k) {
            let apex = h + look * (r * 1.9);
            let _ = writeln!(
                s,
                r#"<path d="M{} L{} L{} Z" fill="{INK}" stroke="{INK}" stroke-width="{}"/><circle cx="{}" cy="{}" r="{}" fill="white" stroke="{INK}" stroke-width="{}"/><path d="{}" fill="none" stroke="{INK}" stroke-width="{}" stroke-linecap="round" stroke-linejoin="round"/>"#,
                pt(h + u * r),
                pt(apex),
                pt(h - u * r),
                num(LAYER_LINE * k),
                num(h.x),
                num(-h.y),
                num(r),
                num(VIEW_LINE * k),
                text_path(
                    &label,
                    h - Point2::new(0.0, th / 2.0),
                    Point2::new(1.0, 0.0),
                    th,
                    Align::Centre
                ),
                num(crate::lettering::STROKE * th)
            );
        }
    }
    s.push_str("</g>\n");
}

/// The four elevations of a model (north, south, east and west), each a
/// few metres outside it, looking at it, wide and deep enough to see it
/// whole. Names that are taken are skipped.
pub fn elevations_around(model: &Model) -> Vec<NewSection> {
    let mut lo = Point2::new(f64::INFINITY, f64::INFINITY);
    let mut hi = Point2::new(f64::NEG_INFINITY, f64::NEG_INFINITY);
    for solid in model_solids(model) {
        for t in &solid.triangles {
            for p in t {
                lo = Point2::new(lo.x.min(p[0]), lo.y.min(p[1]));
                hi = Point2::new(hi.x.max(p[0]), hi.y.max(p[1]));
            }
        }
    }
    if !lo.x.is_finite() {
        return Vec::new();
    }
    let gap = 3.0;
    let side = 1.0;
    let w = hi.x - lo.x;
    let d = hi.y - lo.y;
    let p = Point2::new;
    let all = [
        // Seen from the north, looking south: east is on the left.
        (
            "Norte",
            p(hi.x + side, hi.y + gap),
            p(lo.x - side, hi.y + gap),
            d + 2.0 * gap,
        ),
        (
            "Sur",
            p(lo.x - side, lo.y - gap),
            p(hi.x + side, lo.y - gap),
            d + 2.0 * gap,
        ),
        (
            "Este",
            p(hi.x + gap, lo.y - side),
            p(hi.x + gap, hi.y + side),
            w + 2.0 * gap,
        ),
        (
            "Oeste",
            p(lo.x - gap, hi.y + side),
            p(lo.x - gap, lo.y - side),
            w + 2.0 * gap,
        ),
    ];
    all.into_iter()
        .filter(|(name, ..)| {
            !model
                .sections()
                .any(|s| s.kind == SectionKind::Elevation && s.name == *name)
        })
        .map(|(name, start, end, depth)| NewSection {
            name: name.into(),
            kind: SectionKind::Elevation,
            start,
            end,
            depth,
        })
        .collect()
}

/// Writes a section drawing to SVG, with the same options as a plan
/// (scale, viewport for the screen, thin lines).
pub fn render_section(model: &Model, d: &SectionDrawing, options: &PlanOptions) -> String {
    let k = options.scale / 1000.0;
    let margin = options.margin_mm * k;
    let (vb_x, vb_y, vb_w, vb_h) = match options.viewport {
        Some((lo, hi)) => (
            lo.x,
            -hi.y,
            (hi.x - lo.x).max(1e-9),
            (hi.y - lo.y).max(1e-9),
        ),
        None => {
            let extra = 30.0 * k; // the level labels on the right
            (
                d.lo.x - margin,
                -d.hi.y - margin,
                (d.hi.x - d.lo.x) + 2.0 * margin + extra,
                (d.hi.y - d.lo.y) + 2.0 * margin + 4.0 * TEXT_SIZE * k,
            )
        }
    };
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
    let visible = |poly: &[Point2]| match options.viewport {
        Some((lo, hi)) => poly.iter().any(|p| {
            p.x >= lo.x - 1.0 && p.x <= hi.x + 1.0 && p.y >= lo.y - 1.0 && p.y <= hi.y + 1.0
        }),
        None => true,
    };

    // Beyond: far to near.
    s.push_str("<g id=\"beyond\" stroke-linejoin=\"round\" stroke-linecap=\"round\">\n");
    let mut edges = String::new();
    let flush = |s: &mut String, edges: &mut String| {
        if !edges.is_empty() {
            let _ = writeln!(
                s,
                r#"<path d="{}" fill="none" stroke="{INK}" stroke-width="{}"/>"#,
                edges.trim_end(),
                num(VIEW_LINE * k)
            );
            edges.clear();
        }
    };
    for f in &d.faces {
        if f.poly.is_empty() {
            for (a, b) in &f.edges {
                if visible(&[*a, *b]) {
                    let _ = write!(edges, "M{} L{} ", pt(*a), pt(*b));
                }
            }
            continue;
        }
        if !visible(&f.poly) {
            continue;
        }
        flush(&mut s, &mut edges);
        let [r, g, b] = f.fill;
        let _ = writeln!(
            s,
            r#"<path d="{}" fill="rgb({r},{g},{b})" stroke="rgb({r},{g},{b})" stroke-width="{}"/>"#,
            path(&f.poly),
            num(0.15 * k)
        );
    }
    flush(&mut s, &mut edges);
    s.push_str("</g>\n");

    // The cut, over everything.
    s.push_str("<g id=\"cut\" stroke-linejoin=\"miter\">\n");
    for (ci, (mat, loops, _)) in d.cuts.iter().enumerate() {
        let Some(material) = model.material(*mat) else {
            continue;
        };
        for (li, l) in loops.iter().enumerate() {
            if !visible(l) {
                continue;
            }
            fill_cut(&mut s, material, l, &format!("cut-{ci}-{li}"), k);
        }
    }
    for (_, loops, flags) in &d.cuts {
        for (l, fl) in loops.iter().zip(flags) {
            if !visible(l) {
                continue;
            }
            let (mut heavy, mut thin) = (String::new(), String::new());
            let n = l.len();
            for i in 0..n {
                let (a, b) = (l[i], l[(i + 1) % n]);
                let target = if fl[i] { &mut heavy } else { &mut thin };
                let _ = write!(target, "M{} L{} ", pt(a), pt(b));
            }
            for (d_, w) in [(thin, LAYER_LINE), (heavy, CUT_LINE)] {
                if !d_.is_empty() {
                    let _ = writeln!(
                        s,
                        r#"<path d="{}" fill="none" stroke="{INK}" stroke-width="{}" stroke-linecap="round"/>"#,
                        d_.trim_end(),
                        num(w * k)
                    );
                }
            }
        }
    }
    s.push_str("</g>\n");

    // Levels.
    s.push_str("<g id=\"levels\">\n");
    let (x0, x1) = (d.lo.x - 1.0, d.hi.x + 1.0);
    for (name, z) in &d.levels {
        let _ = writeln!(
            s,
            r#"<path d="M{} L{}" fill="none" stroke="{INK}" stroke-width="{}" stroke-dasharray="{} {} {} {}"/>"#,
            pt(Point2::new(x0, *z)),
            pt(Point2::new(x1, *z)),
            num(LAYER_LINE * k),
            num(8.0 * k),
            num(1.5 * k),
            num(1.0 * k),
            num(1.5 * k)
        );
        let label = format!("{} {}", annotate::format_elevation(*z), name);
        let h = annotate::TEXT_HEIGHT * k;
        let _ = writeln!(
            s,
            r#"<path d="{}" fill="none" stroke="{INK}" stroke-width="{}" stroke-linecap="round" stroke-linejoin="round"/>"#,
            text_path(
                &label,
                Point2::new(x1 + 0.5 * k, *z + 0.8 * k),
                Point2::new(1.0, 0.0),
                h,
                Align::Left
            ),
            num(crate::lettering::STROKE * h)
        );
    }
    s.push_str("</g>\n");

    if options.viewport.is_none() {
        let _ = writeln!(
            s,
            r#"<text x="{}" y="{}" font-family="Helvetica, Arial, sans-serif" font-size="{}" fill="{INK}"><tspan font-weight="bold">{}</tspan> · {} · E 1:{}</text>"#,
            num(d.lo.x),
            num(-d.lo.y + margin * 0.6 + TEXT_SIZE * k),
            num(TEXT_SIZE * k),
            crate::escape(&model.project_name),
            crate::escape(&d.title),
            options.scale
        );
    }
    s.push_str("</svg>\n");
    if options.thin_lines {
        let mm = options
            .px_per_mm
            .map_or(crate::HAIRLINE_MM, |px| 1.0 / px.max(1e-6));
        return crate::hairlines(&s, mm * k);
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;
    use archisim_core::{
        CutPattern, Layer, LayerFunction, NewWall, ReferenceLine, WallFunction, WallTop,
    };

    /// A two-layer wall cut square: two pieces of poché with the thin line
    /// between them, and a wall beyond drawn with its edges.
    #[test]
    fn cuts_and_sees_beyond() {
        let mut m = Model::new("t");
        let l = m.add_level("PB", 0.0);
        let brick = m.add_material("Ladrillo", "brick", [200, 120, 90], CutPattern::Diagonal);
        let plaster = m.add_material("Yeso", "plaster", [240, 240, 240], CutPattern::Solid);
        let wt = m
            .add_wall_type(
                "Muro",
                WallFunction::Exterior,
                vec![
                    Layer {
                        material: brick,
                        thickness: 0.2,
                        function: LayerFunction::Structure,
                    },
                    Layer {
                        material: plaster,
                        thickness: 0.02,
                        function: LayerFunction::Finish,
                    },
                ],
            )
            .unwrap();
        for (a, b) in [
            (Point2::new(0.0, 0.0), Point2::new(0.0, 5.0)),
            (Point2::new(4.0, 5.0), Point2::new(4.0, 0.0)),
            (Point2::new(10.0, 6.0), Point2::new(-2.0, 6.0)),
        ] {
            m.add_wall(NewWall {
                wall_type: wt,
                level: l,
                start: a,
                end: b,
                reference: ReferenceLine::Center,
                height: 3.0,
                base_offset: 0.0,
                bulge: 0.0,
                top: WallTop::Unconnected,
            })
            .unwrap();
        }
        let sec = m
            .add_section(NewSection {
                name: "A".into(),
                kind: SectionKind::Section,
                start: Point2::new(-1.0, 2.0),
                end: Point2::new(6.0, 2.0),
                depth: 10.0,
            })
            .unwrap();
        let d = section_drawing(&m, m.section(sec).unwrap());
        // Two walls cut, two layers each.
        assert_eq!(d.cuts.len(), 4, "{:?}", d.cuts.len());
        let brick_cuts: Vec<_> = d.cuts.iter().filter(|c| c.0 == brick).collect();
        assert_eq!(brick_cuts.len(), 2);
        for (_, loops, _) in &brick_cuts {
            assert_eq!(loops.len(), 1);
            let area = archisim_core::signed_area(&loops[0]).abs();
            assert!((area - 0.2 * 3.0).abs() < 1e-6, "{area}");
        }
        // The wall beyond is seen: faces with edges.
        assert!(d.faces.iter().any(|f| !f.edges.is_empty()));
        assert!(d.faces.iter().any(|f| !f.poly.is_empty()));
        assert!(d.hi.y >= 3.0 - 1e-9 && d.lo.y <= 0.0 + 1e-9);
        let svg = render_section(&m, &d, &PlanOptions::default());
        assert!(svg.contains("id=\"cut\"") && svg.contains("id=\"levels\""));

        // Four elevations around, each looking at the building.
        let els = elevations_around(&m);
        assert_eq!(els.len(), 4);
        let middle = Point2::new(4.0, 3.0);
        for e in els {
            let id = m.add_section(e).unwrap();
            let sec = m.section(id).unwrap();
            let look = sec.looks().unwrap();
            assert!((middle - sec.start).dot(look) > 0.0, "{}", sec.title());
            let d = section_drawing(&m, sec);
            assert!(d.cuts.is_empty() && !d.faces.is_empty(), "{}", sec.title());
        }
        assert!(elevations_around(&m).is_empty());
        let mut plan = String::new();
        draw_markers(&mut plan, &m, 0.05);
        assert_eq!(plan.matches("<circle").count(), 6);
    }
}
