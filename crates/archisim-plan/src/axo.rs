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
}

/// The picture: faces from far to near, and its extent (picture metres).
pub struct AxoDrawing {
    faces: Vec<Face>,
    pub lo: Point2,
    pub hi: Point2,
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
    let mut lo = Point2::new(f64::INFINITY, f64::INFINITY);
    let mut hi = Point2::new(f64::NEG_INFINITY, f64::NEG_INFINITY);
    for solid in model_solids(model) {
        let mat = model.material(solid.material);
        if mat.is_some_and(|m| m.category == "glass") {
            // Glass would hide what is behind it.
            continue;
        }
        let base = archisim_geom::solid::solid_color(model, &solid);
        let grey = model.dimmed(focus, solid.owner.id());
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
                        dot(m, f.toward) <= 1e-9 || dot(m, n) < 0.985
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
            });
        }
    }
    if faces.is_empty() {
        lo = Point2::new(0.0, 0.0);
        hi = Point2::new(5.0, 5.0);
    }
    faces.sort_by(|a, b| a.depth.total_cmp(&b.depth));
    AxoDrawing { faces, lo, hi }
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
    for face in &d.faces {
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
    s.push_str("</g>\n</svg>\n");
    s
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
