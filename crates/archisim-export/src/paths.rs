//! An SVG drawing as flat lists of filled and stroked polylines in its own
//! user units (metres of the plan, or of paper on a sheet), with clipping
//! by the frames of nested views resolved.

use resvg::usvg::{self, tiny_skia_path::PathSegment, Node, Transform};

/// A point in the drawing's user units, y down as in SVG.
pub type P = (f64, f64);

/// One drawn path, flattened.
#[derive(Debug, Clone, PartialEq)]
pub struct Item {
    /// Subpaths: points and whether each is closed.
    pub subpaths: Vec<(Vec<P>, bool)>,
    /// Fill colour, if filled.
    pub fill: Option<[u8; 3]>,
    /// How opaque the fill is, 0 to 1 (see-through canopies).
    pub fill_opacity: f64,
    /// How opaque the stroke is.
    pub stroke_opacity: f64,
    /// Stroke colour and width (user units), if stroked.
    pub stroke: Option<([u8; 3], f64)>,
    /// Dash pattern in user units.
    pub dashes: Vec<f64>,
    /// Even-odd fill.
    pub even_odd: bool,
    /// The `id` of the outermost named group it is in ("slabs", "poche",
    /// "openings", "cut-lines", "annotations", "drafting"…).
    pub group: String,
    /// Inside a nested view (a viewport of a sheet).
    pub nested: bool,
}

/// Parses an SVG drawing.
pub fn parse(svg: &str) -> Result<usvg::Tree, String> {
    usvg::Tree::from_str(svg, &usvg::Options::default()).map_err(|e| e.to_string())
}

/// The drawing's paths in the tree's pixels. Paths outside the clip of a
/// nested view are cut to it.
pub fn items(tree: &usvg::Tree) -> Vec<Item> {
    let mut out = Vec::new();
    let root = tree.root();
    walk_group(root, Transform::identity(), &mut out);
    out
}

fn walk_group(root: &usvg::Group, t: Transform, out: &mut Vec<Item>) {
    let t = t.pre_concat(root.transform());
    for n in root.children() {
        walk(n, t, "", None, false, out);
    }
}

/// The drawing's paths in its own user units (those of its `viewBox`):
/// metres of the plan, or of paper on a sheet; y down.
pub fn user_items(svg: &str) -> Result<Vec<Item>, String> {
    let tree = parse(svg)?;
    let (vx, vy, vw, vh) = view_box(svg).ok_or("el dibujo no tiene viewBox")?;
    let size = tree.size();
    let (sx, sy) = (vw / size.width() as f64, vh / size.height() as f64);
    let mut v = items(&tree);
    for it in &mut v {
        for (pts, _) in &mut it.subpaths {
            for p in pts.iter_mut() {
                *p = (vx + p.0 * sx, vy + p.1 * sy);
            }
        }
        if let Some((_, w)) = &mut it.stroke {
            *w *= sx;
        }
        for d in &mut it.dashes {
            *d *= sx;
        }
    }
    Ok(v)
}

/// The paper size of a drawing, millimetres (its `width` and `height`
/// in mm).
pub fn paper_mm(svg: &str) -> Option<(f64, f64)> {
    let start = svg.find("<svg")?;
    let tag = &svg[start..start + svg[start..].find('>')?];
    let attr = |name: &str| -> Option<f64> {
        let key = format!(" {name}=\"");
        let i = tag.find(&key)? + key.len();
        let j = tag[i..].find('"')? + i;
        tag[i..j].trim_end_matches("mm").parse().ok()
    };
    Some((attr("width")?, attr("height")?))
}

/// The `viewBox` of the outermost `<svg>`.
fn view_box(svg: &str) -> Option<(f64, f64, f64, f64)> {
    let start = svg.find("<svg")?;
    let tag = &svg[start..start + svg[start..].find('>')?];
    let i = tag.find("viewBox=\"")? + 9;
    let j = tag[i..].find('"')? + i;
    let v: Vec<f64> = tag[i..j]
        .split([' ', ','])
        .filter(|s| !s.is_empty())
        .filter_map(|s| s.parse().ok())
        .collect();
    (v.len() == 4).then(|| (v[0], v[1], v[2], v[3]))
}

fn walk(
    node: &Node,
    t: Transform,
    group: &str,
    clip: Option<(P, P)>,
    nested: bool,
    out: &mut Vec<Item>,
) {
    match node {
        Node::Group(g) => {
            let t = t.pre_concat(g.transform());
            let name = if group.is_empty() && !g.id().is_empty() {
                g.id()
            } else {
                group
            };
            let mut clip = clip;
            let mut nested = nested;
            if let Some(cp) = g.clip_path() {
                // The frame of a view: a rectangle; its box is enough.
                let ct = t.pre_concat(cp.transform());
                let mut pts = Vec::new();
                for n in cp.root().children() {
                    let mut sub = Vec::new();
                    walk(n, ct, "", None, false, &mut sub);
                    for it in sub {
                        for (p, _) in it.subpaths {
                            pts.extend(p);
                        }
                    }
                }
                if !pts.is_empty() {
                    let lo = pts.iter().fold((f64::INFINITY, f64::INFINITY), |a, p| {
                        (a.0.min(p.0), a.1.min(p.1))
                    });
                    let hi = pts
                        .iter()
                        .fold((f64::NEG_INFINITY, f64::NEG_INFINITY), |a, p| {
                            (a.0.max(p.0), a.1.max(p.1))
                        });
                    clip = Some(match clip {
                        Some((a, b)) => (
                            (a.0.max(lo.0), a.1.max(lo.1)),
                            (b.0.min(hi.0), b.1.min(hi.1)),
                        ),
                        None => (lo, hi),
                    });
                    nested = true;
                }
            }
            // A nested view starts its own naming.
            let name = if nested && g.clip_path().is_some() {
                ""
            } else {
                name
            };
            for c in g.children() {
                walk(c, t, name, clip, nested, out);
            }
        }
        Node::Path(p) => {
            if !p.is_visible() {
                return;
            }
            let scale = (t.sx as f64 * t.sy as f64 - t.kx as f64 * t.ky as f64)
                .abs()
                .sqrt();
            let fill = p.fill().map(|f| color(f.paint()));
            let fill_opacity = p.fill().map_or(1.0, |f| f.opacity().get() as f64);
            let stroke_opacity = p.stroke().map_or(1.0, |f| f.opacity().get() as f64);
            let even_odd = p
                .fill()
                .is_some_and(|f| f.rule() == usvg::FillRule::EvenOdd);
            let stroke = p
                .stroke()
                .map(|s| (color(s.paint()), s.width().get() as f64 * scale));
            let dashes = p
                .stroke()
                .and_then(|s| s.dasharray())
                .map(|d| d.iter().map(|v| *v as f64 * scale).collect())
                .unwrap_or_default();
            let subpaths = flatten(p.data(), t);
            let item = |subpaths, fill, stroke, dashes| Item {
                subpaths,
                fill,
                fill_opacity,
                stroke_opacity,
                stroke,
                dashes,
                even_odd,
                group: group.to_string(),
                nested,
            };
            match clip {
                None => out.push(item(subpaths, fill, stroke, dashes)),
                Some((lo, hi)) => {
                    // Cut to the frame: the fill as areas, the stroke as
                    // lines.
                    if fill.is_some() {
                        let a = clip_subpaths(subpaths.clone(), lo, hi, true);
                        if !a.is_empty() {
                            out.push(item(a, fill, None, Vec::new()));
                        }
                    }
                    if stroke.is_some() {
                        let l = clip_subpaths(subpaths, lo, hi, false);
                        if !l.is_empty() {
                            out.push(item(l, None, stroke, dashes));
                        }
                    }
                }
            }
        }
        Node::Image(_) | Node::Text(_) => {}
    }
}

fn color(p: &usvg::Paint) -> [u8; 3] {
    match p {
        usvg::Paint::Color(c) => [c.red, c.green, c.blue],
        _ => [128, 128, 128],
    }
}

fn apply(t: Transform, x: f32, y: f32) -> P {
    (
        (t.sx * x + t.kx * y + t.tx) as f64,
        (t.ky * x + t.sy * y + t.ty) as f64,
    )
}

/// A path as polylines (curves cut into short straight pieces).
fn flatten(path: &usvg::tiny_skia_path::Path, t: Transform) -> Vec<(Vec<P>, bool)> {
    let mut out: Vec<(Vec<P>, bool)> = Vec::new();
    let mut cur: Vec<P> = Vec::new();
    let mut last = (0.0f32, 0.0f32);
    let finish = |cur: &mut Vec<P>, closed: bool, out: &mut Vec<(Vec<P>, bool)>| {
        if cur.len() >= 2 || (cur.len() == 1 && !closed) {
            out.push((std::mem::take(cur), closed));
        } else {
            cur.clear();
        }
    };
    for seg in path.segments() {
        match seg {
            PathSegment::MoveTo(p) => {
                finish(&mut cur, false, &mut out);
                cur.push(apply(t, p.x, p.y));
                last = (p.x, p.y);
            }
            PathSegment::LineTo(p) => {
                cur.push(apply(t, p.x, p.y));
                last = (p.x, p.y);
            }
            PathSegment::QuadTo(c, p) => {
                let n = pieces(last, (c.x, c.y), (p.x, p.y), (p.x, p.y), t);
                for i in 1..=n {
                    let s = i as f32 / n as f32;
                    let u = 1.0 - s;
                    let x = u * u * last.0 + 2.0 * u * s * c.x + s * s * p.x;
                    let y = u * u * last.1 + 2.0 * u * s * c.y + s * s * p.y;
                    cur.push(apply(t, x, y));
                }
                last = (p.x, p.y);
            }
            PathSegment::CubicTo(c1, c2, p) => {
                let n = pieces(last, (c1.x, c1.y), (c2.x, c2.y), (p.x, p.y), t);
                for i in 1..=n {
                    let s = i as f32 / n as f32;
                    let u = 1.0 - s;
                    let x = u * u * u * last.0
                        + 3.0 * u * u * s * c1.x
                        + 3.0 * u * s * s * c2.x
                        + s * s * s * p.x;
                    let y = u * u * u * last.1
                        + 3.0 * u * u * s * c1.y
                        + 3.0 * u * s * s * c2.y
                        + s * s * s * p.y;
                    cur.push(apply(t, x, y));
                }
                last = (p.x, p.y);
            }
            PathSegment::Close => {
                if let Some(first) = cur.first().copied() {
                    if cur.len() > 1 && cur.last() == Some(&first) {
                        cur.pop();
                    }
                }
                finish(&mut cur, true, &mut out);
            }
        }
    }
    finish(&mut cur, false, &mut out);
    out
}

/// How many straight pieces a curve needs: about one per 2° of turn,
/// judged by the length of its control polygon against its chord.
fn pieces(a: (f32, f32), b: (f32, f32), c: (f32, f32), d: (f32, f32), t: Transform) -> usize {
    let l = |p: (f32, f32), q: (f32, f32)| ((p.0 - q.0).powi(2) + (p.1 - q.1).powi(2)).sqrt();
    let poly = l(a, b) + l(b, c) + l(c, d);
    let chord = l(a, d);
    let bend = if poly > 1e-9 { 1.0 - chord / poly } else { 0.0 };
    let _ = t;
    ((bend.sqrt() * 40.0).ceil() as usize).clamp(2, 32)
}

/// Cuts subpaths to a box: open lines piece by piece, filled areas
/// (`area`) as polygons.
fn clip_subpaths(subs: Vec<(Vec<P>, bool)>, lo: P, hi: P, area: bool) -> Vec<(Vec<P>, bool)> {
    let mut out = Vec::new();
    for (pts, closed) in subs {
        if area && closed {
            let poly = clip_polygon(&pts, lo, hi);
            if poly.len() >= 3 {
                out.push((poly, true));
            }
            continue;
        }
        let inside = |p: &P| p.0 >= lo.0 && p.0 <= hi.0 && p.1 >= lo.1 && p.1 <= hi.1;
        if pts.iter().all(inside) {
            out.push((pts, closed));
            continue;
        }
        let mut ring = pts.clone();
        if closed {
            ring.push(pts[0]);
        }
        let mut cur: Vec<P> = Vec::new();
        for w in ring.windows(2) {
            match clip_segment(w[0], w[1], lo, hi) {
                Some((a, b)) => {
                    if cur.last().is_some_and(|l| close(*l, a)) {
                        cur.push(b);
                    } else {
                        if cur.len() >= 2 {
                            out.push((std::mem::take(&mut cur), false));
                        }
                        cur = vec![a, b];
                    }
                }
                None => {
                    if cur.len() >= 2 {
                        out.push((std::mem::take(&mut cur), false));
                    }
                    cur.clear();
                }
            }
        }
        if cur.len() >= 2 {
            out.push((cur, false));
        }
    }
    out
}

fn close(a: P, b: P) -> bool {
    (a.0 - b.0).abs() < 1e-9 && (a.1 - b.1).abs() < 1e-9
}

/// Liang–Barsky.
fn clip_segment(a: P, b: P, lo: P, hi: P) -> Option<(P, P)> {
    let (dx, dy) = (b.0 - a.0, b.1 - a.1);
    let (mut t0, mut t1) = (0.0f64, 1.0f64);
    for (p, q) in [
        (-dx, a.0 - lo.0),
        (dx, hi.0 - a.0),
        (-dy, a.1 - lo.1),
        (dy, hi.1 - a.1),
    ] {
        if p.abs() < 1e-15 {
            if q < 0.0 {
                return None;
            }
        } else {
            let r = q / p;
            if p < 0.0 {
                t0 = t0.max(r);
            } else {
                t1 = t1.min(r);
            }
        }
    }
    (t0 <= t1).then_some((
        (a.0 + dx * t0, a.1 + dy * t0),
        (a.0 + dx * t1, a.1 + dy * t1),
    ))
}

/// Sutherland–Hodgman against a box.
fn clip_polygon(poly: &[P], lo: P, hi: P) -> Vec<P> {
    let mut v = poly.to_vec();
    let edges: [(usize, f64, bool); 4] = [
        (0, lo.0, true),
        (0, hi.0, false),
        (1, lo.1, true),
        (1, hi.1, false),
    ];
    for (axis, c, keep_above) in edges {
        if v.is_empty() {
            break;
        }
        let get = |p: &P| if axis == 0 { p.0 } else { p.1 };
        let ins = |p: &P| if keep_above { get(p) >= c } else { get(p) <= c };
        let mut out = Vec::new();
        for i in 0..v.len() {
            let (a, b) = (v[i], v[(i + 1) % v.len()]);
            let (ia, ib) = (ins(&a), ins(&b));
            if ia {
                out.push(a);
            }
            if ia != ib {
                let t = (c - get(&a)) / (get(&b) - get(&a));
                out.push((a.0 + (b.0 - a.0) * t, a.1 + (b.1 - a.1) * t));
            }
        }
        v = out;
    }
    v
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn paths_groups_and_clips() {
        let svg = r##"<svg xmlns="http://www.w3.org/2000/svg" width="100mm" height="50mm" viewBox="0 0 10 5">
<g id="walls"><path d="M0,0 L4,0 L4,2 Z" fill="#ff0000" stroke="#000000" stroke-width="0.1"/></g>
<svg x="5" y="0" width="2" height="2" viewBox="0 0 4 4" overflow="hidden">
<path d="M0,1 L10,1" stroke="#000000" stroke-width="0.2" fill="none"/>
</svg>
</svg>"##;
        let items = user_items(svg).unwrap();
        assert_eq!(items.len(), 2, "{items:?}");
        assert_eq!(items[0].group, "walls");
        assert_eq!(items[0].fill, Some([255, 0, 0]));
        let (pts, closed) = &items[0].subpaths[0];
        assert!(*closed);
        for (p, q) in pts.iter().zip([(0.0, 0.0), (4.0, 0.0), (4.0, 2.0)]) {
            assert!(
                (p.0 - q.0).abs() < 1e-5 && (p.1 - q.1).abs() < 1e-5,
                "{pts:?}"
            );
        }
        // The view is half size and cut at its frame (x 5…7).
        let line = &items[1];
        assert!(line.nested);
        let (pts, _) = &line.subpaths[0];
        assert!(
            (pts[0].0 - 5.0).abs() < 1e-6 && (pts[1].0 - 7.0).abs() < 1e-6,
            "{pts:?}"
        );
        assert!((pts[0].1 - 0.5).abs() < 1e-6);
        assert!((line.stroke.unwrap().1 - 0.1).abs() < 1e-6);
    }
}
