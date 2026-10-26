//! Axonometric 3D views for sheets: the whole model in parallel projection,
//! at a scale, drawn as vectors (so it prints, goes to PDF and to DXF like
//! any other view). Faces are filled with their material's colour, lit
//! from above the left shoulder, and drawn from the farthest to the
//! nearest; the edges between faces that turn (and the outline of each
//! element) are drawn with each face, so nearer faces cover them.

use std::collections::HashMap;
use std::fmt::Write as _;

use archisim_core::{Axo, DisciplineFocus, Model, Point2};
use archisim_geom::solid::{model_solids, P3};

use crate::num;

/// Where the picture is: its axes in the model and the direction it looks
/// from.
struct Frame {
    right: P3,
    up: P3,
    /// From the model towards the eye.
    toward: P3,
}

fn dot(a: P3, b: P3) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

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

fn unit(a: P3) -> P3 {
    let l = dot(a, a).sqrt();
    if l < 1e-12 {
        [0.0, 0.0, 0.0]
    } else {
        [a[0] / l, a[1] / l, a[2] / l]
    }
}

fn frame(axo: &Axo) -> Frame {
    let (cy, sy) = (axo.yaw.cos(), axo.yaw.sin());
    let (cp, sp) = (axo.pitch.cos(), axo.pitch.sin());
    let toward = [cp * cy, cp * sy, sp];
    // Right of the picture: horizontal, at right angles to the view.
    let right = [-sy, cy, 0.0];
    let up = cross(toward, right);
    Frame { right, up, toward }
}

impl Frame {
    fn project(&self, p: P3) -> Point2 {
        Point2::new(dot(p, self.right), dot(p, self.up))
    }
}

/// One face to draw.
struct Face {
    pts: [Point2; 3],
    depth: f64,
    fill: [u8; 3],
    /// Which of its three edges (a→b, b→c, c→a) are drawn.
    edges: [bool; 3],
    /// Of a discipline the view leaves in grey.
    grey: bool,
    /// Lies on the ground (no higher than the lowest level and flat).
    on_ground: bool,
    /// A plant standing up to the viewer, drawn as architects draw trees
    /// (its marks, in the picture), instead of the triangle `pts`.
    figure: Option<Vec<FigureMark>>,
}

/// A mark of a plant in the picture: a shape filled (colour and opacity)
/// and or drawn (paper millimetres).
struct FigureMark {
    pts: Vec<Point2>,
    closed: bool,
    fill: Option<([u8; 3], f64)>,
    stroke: Option<f64>,
    /// A picture instead: its bytes, its size in its own units and the
    /// SVG matrix that puts it in the picture.
    image: Option<Picture>,
}

/// A picture's bytes, width, height and SVG matrix.
type Picture = (std::sync::Arc<Vec<u8>>, f64, f64, [f64; 6]);

/// The picture: faces from far to near, and its extent (picture metres).
pub struct AxoDrawing {
    faces: Vec<Face>,
    pub lo: Point2,
    pub hi: Point2,
    /// The sun's shadows on the ground, in the picture (rings, even-odd),
    /// drawn after the first `ground` faces (those lying on the ground).
    shadows: Vec<Vec<Point2>>,
    ground: usize,
    shadow_rgb: [u8; 3],
}

impl AxoDrawing {
    /// Adds the sun's shadows on the ground of the lowest level: between
    /// what lies on the ground and what stands on it.
    pub fn with_shadows(mut self, model: &Model, axo: &Axo, rgb: [u8; 3]) -> Self {
        let levels = model.levels();
        let (Some(sun), Some(level)) = (model.sun_direction(), levels.first()) else {
            return self;
        };
        let f = frame(axo);
        let z = level.elevation;
        self.shadows = archisim_geom::shadow::shadows(model, level.id, sun)
            .into_iter()
            .flat_map(|(o, hs)| std::iter::once(o).chain(hs))
            .map(|ring| ring.iter().map(|p| f.project([p.x, p.y, z])).collect())
            .collect();
        self.shadow_rgb = rgb;
        self
    }
}

/// A vertex rounded to a tenth of a millimetre, to find shared edges.
fn key(p: P3) -> [i64; 3] {
    [
        (p[0] * 1e4).round() as i64,
        (p[1] * 1e4).round() as i64,
        (p[2] * 1e4).round() as i64,
    ]
}

/// The axonometric of every element of the model; what `focus` leaves as
/// reference, in grey.
pub fn axo_drawing(model: &Model, axo: &Axo, focus: DisciplineFocus) -> AxoDrawing {
    let f = frame(axo);
    // From above, on the left of the eye.
    let light = unit([
        f.toward[0] * 0.4 - f.right[0] * 0.5,
        f.toward[1] * 0.4 - f.right[1] * 0.5,
        0.9,
    ]);
    let mut faces = Vec::new();
    let ground_z = model.levels().first().map_or(0.0, |l| l.elevation) + 0.02;
    let mut lo = Point2::new(f64::INFINITY, f64::INFINITY);
    let mut hi = Point2::new(f64::NEG_INFINITY, f64::NEG_INFINITY);
    let solids = archisim_geom::solid::without_plants(|| model_solids(model));
    // Objects with their own drawing for elevations, and plants, stand up
    // to the viewer as figures (below), not by their mesh.
    let toward = [-f.toward[0], -f.toward[1]];
    let customs: std::collections::HashSet<archisim_core::ObjectId> = model
        .objects()
        .filter(|o| model.shown_figure(o.id, toward, true).is_some())
        .map(|o| o.id)
        .collect();
    let plants: std::collections::HashSet<archisim_core::ObjectId> = model
        .objects()
        .filter(|o| model.type_grows(o.object_type) || customs.contains(&o.id))
        .map(|o| o.id)
        .collect();
    for solid in solids {
        if matches!(solid.owner, archisim_geom::solid::SolidOwner::Object(o) if plants.contains(&o))
        {
            continue;
        }
        let mat = model.material(solid.material);
        if mat.is_some_and(|m| m.category == "glass") {
            // Glass would hide what is behind it.
            continue;
        }
        let base = archisim_geom::solid::solid_color(model, &solid);
        let grey = model.dimmed(focus, solid.owner.id());
        // The leaves of a plant: their outline, not every facet.
        let soft = match solid.owner {
            archisim_geom::solid::SolidOwner::Object(o) => model
                .plant_look(o)
                .is_some_and(|l| solid.material != l.materials.bark),
            _ => false,
        };
        // A twisted or bent volume is curved: only its real edges.
        let crease = match solid.owner {
            archisim_geom::solid::SolidOwner::Mass(id)
                if model
                    .mass(id)
                    .and_then(|m| m.deform)
                    .is_some_and(|d| !d.is_none()) =>
            {
                0.85
            }
            _ => 0.985,
        };
        // Normals of every triangle, and who shares each edge.
        let normals: Vec<P3> = solid
            .triangles
            .iter()
            .map(|[a, b, c]| unit(cross(sub(*b, *a), sub(*c, *a))))
            .collect();
        let mut by_edge: HashMap<([i64; 3], [i64; 3]), Vec<usize>> = HashMap::new();
        for (i, t) in solid.triangles.iter().enumerate() {
            for e in 0..3 {
                let (a, b) = (key(t[e]), key(t[(e + 1) % 3]));
                let k = if a <= b { (a, b) } else { (b, a) };
                by_edge.entry(k).or_default().push(i);
            }
        }
        for (i, t) in solid.triangles.iter().enumerate() {
            let n = normals[i];
            if dot(n, f.toward) <= 1e-9 {
                continue;
            }
            let mut edges = [false; 3];
            for (e, drawn) in edges.iter_mut().enumerate() {
                let (a, b) = (key(t[e]), key(t[(e + 1) % 3]));
                let k = if a <= b { (a, b) } else { (b, a) };
                // An edge is drawn where the surface turns: on the outline
                // (the other face looks away) or at a crease.
                *drawn = by_edge.get(&k).is_none_or(|others| {
                    others.iter().filter(|&&j| j != i).all(|&j| {
                        let m = normals[j];
                        dot(m, f.toward) <= 1e-9 || (!soft && dot(m, n) < crease)
                    })
                });
            }
            let lit = 0.62 + 0.38 * dot(n, light).max(0.0);
            let base = archisim_geom::solid::painted_color(model, &solid, t).unwrap_or(base);
            let fill = base.map(|c| ((c as f64) * lit).round().clamp(0.0, 255.0) as u8);
            let pts = [f.project(t[0]), f.project(t[1]), f.project(t[2])];
            for p in pts {
                lo = Point2::new(lo.x.min(p.x), lo.y.min(p.y));
                hi = Point2::new(hi.x.max(p.x), hi.y.max(p.y));
            }
            let depth = (dot(t[0], f.toward) + dot(t[1], f.toward) + dot(t[2], f.toward)) / 3.0;
            faces.push(Face {
                pts,
                depth,
                fill,
                edges,
                grey,
                on_ground: t.iter().all(|p| p[2] <= ground_z) && n[2] > 0.9,
                figure: None,
            });
        }
    }
    // Each plant seen from where the picture looks, standing at its foot:
    // its marks (in its presentation style) lifted into the picture; an
    // object with its own drawing, that one.
    for &id in &plants {
        let Some(o) = model.object(id) else {
            continue;
        };
        if let Some(shown) = model.shown_figure(id, toward, true) {
            let z0 = model.level(o.level).map_or(0.0, |l| l.elevation) + o.base_offset;
            let foot = [o.position.x, o.position.y, z0];
            let figure = custom_marks(&f, foot, &shown);
            for m in &figure {
                for p in &m.pts {
                    lo = Point2::new(lo.x.min(p.x), lo.y.min(p.y));
                    hi = Point2::new(hi.x.max(p.x), hi.y.max(p.y));
                }
            }
            let middle = [foot[0], foot[1], foot[2] + shown.height / 2.0];
            let base = f.project(foot);
            faces.push(Face {
                pts: [base, base, base],
                depth: dot(middle, f.toward),
                fill: [0, 0, 0],
                edges: [false; 3],
                grey: model.dimmed(focus, id.0),
                on_ground: false,
                figure: Some(figure),
            });
            continue;
        }
        let Some(marks) = model.plant_side_marks(id, toward, true, 100.0) else {
            continue;
        };
        let z0 = model.level(o.level).map_or(0.0, |l| l.elevation) + o.base_offset;
        let foot = [o.position.x, o.position.y, z0];
        let at = |p: [f64; 2]| {
            f.project([
                foot[0] + f.right[0] * p[0],
                foot[1] + f.right[1] * p[0],
                foot[2] + p[1],
            ])
        };
        let mut figure = Vec::new();
        let mut top: f64 = 0.0;
        for m in marks {
            use archisim_core::family::{Mark, Pen};
            let width = |pen: Pen| match pen {
                Pen::View => Some(0.18),
                Pen::None => None,
                _ => Some(0.1),
            };
            match m {
                Mark::Polygon {
                    points,
                    pen,
                    filled,
                    tint,
                    opacity,
                    ..
                } => {
                    let fill = match tint.and_then(|t| model.material(archisim_core::MaterialId(t)))
                    {
                        _ if tint == Some(archisim_core::family::INK_TINT) => {
                            Some(([26, 26, 26], 1.0))
                        }
                        Some(m) => Some((m.cut_color, opacity)),
                        None if filled => Some(([255, 255, 255], 1.0)),
                        None => None,
                    };
                    for q in &points {
                        top = top.max(q[1]);
                    }
                    figure.push(FigureMark {
                        pts: points.into_iter().map(at).collect(),
                        closed: true,
                        fill,
                        stroke: width(pen),
                        image: None,
                    });
                }
                Mark::Line { from, to, pen, .. } => figure.push(FigureMark {
                    pts: vec![at(from), at(to)],
                    closed: false,
                    fill: None,
                    stroke: width(pen),
                    image: None,
                }),
                Mark::Arc { .. } => {}
            }
        }
        for m in &figure {
            for p in &m.pts {
                lo = Point2::new(lo.x.min(p.x), lo.y.min(p.y));
                hi = Point2::new(hi.x.max(p.x), hi.y.max(p.y));
            }
        }
        let middle = [foot[0], foot[1], foot[2] + top / 2.0];
        let base = f.project(foot);
        faces.push(Face {
            pts: [base, base, base],
            depth: dot(middle, f.toward),
            fill: [0, 0, 0],
            edges: [false; 3],
            grey: model.dimmed(focus, id.0),
            on_ground: false,
            figure: Some(figure),
        });
    }
    if faces.is_empty() {
        lo = Point2::new(0.0, 0.0);
        hi = Point2::new(5.0, 5.0);
    }
    faces.sort_by(|a, b| a.depth.total_cmp(&b.depth));
    // What lies on the ground goes first, so shadows fall on it and under
    // what stands on it.
    let (mut low, high): (Vec<Face>, Vec<Face>) = faces.into_iter().partition(|f| f.on_ground);
    let ground = low.len();
    low.extend(high);
    AxoDrawing {
        faces: low,
        lo,
        hi,
        shadows: Vec::new(),
        ground,
        shadow_rgb: [40, 40, 46],
    }
}

/// The part of the picture to show whole at a scale, with some air.
pub fn axo_extent(model: &Model, axo: &Axo, scale: f64) -> (Point2, Point2) {
    let d = axo_drawing(model, axo, DisciplineFocus::None);
    let m = 5.0 * scale / 1000.0;
    (d.lo - Point2::new(m, m), d.hi + Point2::new(m, m))
}

/// The picture as an SVG document whose coordinates are picture metres
/// (y up, written as -y), cut to `crop`, for a sheet at `scale`.
pub fn render_axo(d: &AxoDrawing, crop: (Point2, Point2), scale: f64) -> String {
    let (lo, hi) = crop;
    let k = scale / 1000.0;
    // Paper millimetres to picture metres.
    let edge = 0.18 * k;
    let seam = 0.04 * k;
    let mut s = String::new();
    let _ = writeln!(
        s,
        r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="{} {} {} {}">"#,
        num(lo.x),
        num(-hi.y),
        num(hi.x - lo.x),
        num(hi.y - lo.y)
    );
    let _ = writeln!(
        s,
        r#"<g class="axo" stroke-linejoin="round" stroke-linecap="round">"#
    );
    let inside = |p: &Point2| p.x >= lo.x && p.x <= hi.x && p.y >= lo.y && p.y <= hi.y;
    for (i, face) in d.faces.iter().enumerate() {
        if i == d.ground && !d.shadows.is_empty() {
            draw_axo_shadows(&mut s, d);
        }
        if let Some(marks) = &face.figure {
            draw_figure(&mut s, marks, face.grey, k);
            continue;
        }
        if !face.pts.iter().any(inside) {
            // Wholly outside the crop (a tree of the far corner).
            let (a, b) = (
                Point2::new(
                    face.pts.iter().map(|p| p.x).fold(f64::INFINITY, f64::min),
                    face.pts.iter().map(|p| p.y).fold(f64::INFINITY, f64::min),
                ),
                Point2::new(
                    face.pts
                        .iter()
                        .map(|p| p.x)
                        .fold(f64::NEG_INFINITY, f64::max),
                    face.pts
                        .iter()
                        .map(|p| p.y)
                        .fold(f64::NEG_INFINITY, f64::max),
                ),
            );
            if b.x < lo.x || a.x > hi.x || b.y < lo.y || a.y > hi.y {
                continue;
            }
        }
        let [r, g, b] = face.fill;
        let colour = if face.grey {
            let v = (255.0 - (255.0 - (0.3 * r as f64 + 0.59 * g as f64 + 0.11 * b as f64)) * 0.38)
                as u8;
            format!("#{v:02x}{v:02x}{v:02x}")
        } else {
            format!("#{r:02x}{g:02x}{b:02x}")
        };
        let ink = if face.grey { "#a9a9a9" } else { "#222" };
        let [p0, p1, p2] = face.pts;
        // The same colour as a thin outline, so the triangles of a face
        // leave no gap between them.
        let _ = writeln!(
            s,
            r#"<path d="M{} {}L{} {}L{} {}Z" fill="{colour}" stroke="{colour}" stroke-width="{}"/>"#,
            num(p0.x),
            num(-p0.y),
            num(p1.x),
            num(-p1.y),
            num(p2.x),
            num(-p2.y),
            num(seam)
        );
        let mut d = String::new();
        for (e, drawn) in face.edges.iter().enumerate() {
            if *drawn {
                let (a, b) = (face.pts[e], face.pts[(e + 1) % 3]);
                let _ = write!(d, "M{} {}L{} {}", num(a.x), num(-a.y), num(b.x), num(-b.y));
            }
        }
        if !d.is_empty() {
            let _ = writeln!(
                s,
                r##"<path d="{d}" fill="none" stroke="{ink}" stroke-width="{}"/>"##,
                num(edge)
            );
        }
    }
    if d.ground >= d.faces.len() && !d.shadows.is_empty() {
        draw_axo_shadows(&mut s, d);
    }
    s.push_str("</g>\n</svg>\n");
    s
}

/// An object's own drawing standing at `foot`, facing the picture.
fn custom_marks(f: &Frame, foot: P3, shown: &archisim_core::ShownFigure) -> Vec<FigureMark> {
    use archisim_core::{ElevationArt, Shape};
    let h = shown.height;
    let sx = if shown.flip { -1.0 } else { 1.0 };
    let at = |x: f64, up: f64| {
        f.project([
            foot[0] + f.right[0] * x * sx,
            foot[1] + f.right[1] * x * sx,
            foot[2] + up,
        ])
    };
    let mut out = Vec::new();
    match &shown.figure.art {
        ElevationArt::Lines(items) => {
            for item in items {
                let colour = item.color.unwrap_or([26, 26, 26]);
                let pts = |p: &Point2| at(p.x * h, p.y * h);
                if let Shape::Hatch(hatch) = &item.shape {
                    for l in &hatch.loops {
                        let ring: Vec<Point2> = archisim_draft::loop_polygon(l, 1e-3)
                            .iter()
                            .map(pts)
                            .collect();
                        out.push(FigureMark {
                            pts: ring,
                            closed: true,
                            fill: Some((colour, 1.0)),
                            stroke: None,
                            image: None,
                        });
                    }
                    continue;
                }
                let closed = matches!(&item.shape, Shape::Polyline(p) if p.closed)
                    || matches!(&item.shape, Shape::Circle { .. });
                let mut runs: Vec<Vec<Point2>> = Vec::new();
                for seg in archisim_draft::shape_segs(&item.shape) {
                    let q = seg.tessellate(1e-3);
                    match runs.last_mut() {
                        Some(last) if last.last().is_some_and(|x| x.approx_eq(q[0])) => {
                            last.extend_from_slice(&q[1..]);
                        }
                        _ => runs.push(q),
                    }
                }
                for r in runs {
                    out.push(FigureMark {
                        pts: r.iter().map(pts).collect(),
                        closed,
                        fill: closed.then_some(([255, 255, 255], 1.0)),
                        stroke: Some(item.weight.unwrap_or(0.1)),
                        image: None,
                    });
                }
            }
        }
        ElevationArt::Image { png, aspect } => {
            let w = h * aspect;
            let p0 = f.project(foot);
            let o = f.project([0.0, 0.0, 0.0]);
            let r = f.project(f.right) - o;
            let z = f.project([0.0, 0.0, 1.0]) - o;
            // Its pixels (x right, y down, in metres here) into the
            // picture (y up, written as -y).
            let (a, b) = (r.x * sx, -r.y * sx);
            let (c, d) = (-z.x, z.y);
            let e = p0.x - (w / 2.0) * r.x * sx + h * z.x;
            let ff = -(p0.y - (w / 2.0) * r.y * sx + h * z.y);
            let corners = [
                at(-w / 2.0, 0.0),
                at(w / 2.0, 0.0),
                at(w / 2.0, h),
                at(-w / 2.0, h),
            ];
            out.push(FigureMark {
                pts: corners.to_vec(),
                closed: true,
                fill: None,
                stroke: None,
                image: Some((png.clone(), w, h, [a, b, c, d, e, ff])),
            });
        }
    }
    out
}

/// A plant's marks in the picture (grey when its discipline is).
fn draw_figure(s: &mut String, marks: &[FigureMark], grey: bool, k: f64) {
    let tone = |[r, g, b]: [u8; 3]| -> String {
        if grey {
            let v = (255.0 - (255.0 - (0.3 * r as f64 + 0.59 * g as f64 + 0.11 * b as f64)) * 0.38)
                as u8;
            format!("#{v:02x}{v:02x}{v:02x}")
        } else {
            format!("#{r:02x}{g:02x}{b:02x}")
        }
    };
    let ink = if grey { "#a9a9a9" } else { "#222" };
    for m in marks {
        if let Some((png, w, h, t)) = &m.image {
            let _ = writeln!(
                s,
                r#"<image width="{}" height="{}" preserveAspectRatio="none" transform="matrix({} {} {} {} {} {})" href="data:image/png;base64,{}"/>"#,
                num(*w),
                num(*h),
                num(t[0]),
                num(t[1]),
                num(t[2]),
                num(t[3]),
                num(t[4]),
                num(t[5]),
                crate::base64(png)
            );
            continue;
        }
        if m.pts.len() < 2 {
            continue;
        }
        let mut d = String::new();
        for (i, p) in m.pts.iter().enumerate() {
            let _ = write!(
                d,
                "{}{} {}",
                if i == 0 { "M" } else { "L" },
                num(p.x),
                num(-p.y)
            );
        }
        if m.closed {
            d.push('Z');
        }
        let fill = match m.fill {
            Some((c, op)) if op < 1.0 => {
                format!(r#"fill="{}" fill-opacity="{}""#, tone(c), num(op))
            }
            Some((c, _)) => format!(r#"fill="{}""#, tone(c)),
            None => r#"fill="none""#.to_string(),
        };
        let stroke = match m.stroke {
            Some(w) => format!(r#"stroke="{ink}" stroke-width="{}""#, num(w * k)),
            None => r#"stroke="none""#.to_string(),
        };
        let _ = writeln!(s, r#"<path d="{d}" {fill} {stroke}/>"#);
    }
}

fn draw_axo_shadows(s: &mut String, d: &AxoDrawing) {
    let mut path = String::new();
    for ring in &d.shadows {
        for (i, p) in ring.iter().enumerate() {
            let _ = write!(
                path,
                "{}{} {}",
                if i == 0 { "M" } else { "L" },
                num(p.x),
                num(-p.y)
            );
        }
        path.push('Z');
    }
    let [r, g, b] = d.shadow_rgb;
    let _ = writeln!(
        s,
        r#"<path d="{path}" fill="rgb({r},{g},{b})" fill-opacity="{}" fill-rule="evenodd" stroke="none"/>"#,
        num(crate::presentation::SHADOW_OPACITY)
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frame_is_right_handed_and_looks_down() {
        let f = frame(&Axo::default());
        assert!((dot(f.right, f.up)).abs() < 1e-12);
        assert!((dot(f.toward, f.up)).abs() < 1e-12);
        // Up in the picture goes up in the model.
        assert!(f.up[2] > 0.0);
        // Seen from above.
        assert!(f.toward[2] > 0.0);
    }

    #[test]
    fn a_box_shows_three_faces_with_its_outline() {
        use archisim_core::{
            CutPattern, Layer, LayerFunction, NewWall, ReferenceLine, WallFunction, WallTop,
        };
        let mut m = Model::new("Prueba");
        let l = m.add_level("Planta baja", 0.0);
        let brick = m.add_material("Ladrillo", "brick", [230, 200, 190], CutPattern::Diagonal);
        let t = m
            .add_wall_type(
                "Muro",
                WallFunction::Exterior,
                vec![Layer {
                    material: brick,
                    thickness: 0.2,
                    function: LayerFunction::Structure,
                }],
            )
            .unwrap();
        m.add_wall(NewWall {
            wall_type: t,
            level: l,
            start: Point2::new(0.0, 0.0),
            end: Point2::new(4.0, 0.0),
            bulge: 0.0,
            reference: ReferenceLine::Center,
            height: 2.5,
            base_offset: 0.0,
            top: WallTop::Unconnected,
        })
        .unwrap();
        let d = axo_drawing(&m, &Axo::default(), DisciplineFocus::None);
        assert!(!d.faces.is_empty());
        assert!(d.hi.x > d.lo.x && d.hi.y > d.lo.y);
        let svg = render_axo(&d, (d.lo, d.hi), 50.0);
        assert!(svg.contains("stroke=\"#222\""));
        // Far to near.
        assert!(d.faces.windows(2).all(|w| w[0].depth <= w[1].depth));
    }
}
