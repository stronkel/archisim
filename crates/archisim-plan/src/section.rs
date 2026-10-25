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
use archisim_geom::solid::{model_solids, SolidOwner, P3};

use crate::lettering::{text_path, Align};
use crate::{annotate, fill_cut, num, path, pens, pt, PlanOptions, INK, TEXT_SIZE};

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
    /// Height dimensions: the points measured (vertical, along x = the
    /// first point's) and where the dimension line goes (x).
    heights: Vec<(Vec<Point2>, f64)>,
    /// Grid lines it crosses: distance along its line and name.
    grids: Vec<(f64, String)>,
    pub lo: Point2,
    pub hi: Point2,
    pub title: String,
}

/// A cut solid: its material, loops, which loop edges are heavy, and the
/// element it belongs to.
type CutPiece = (MaterialId, Vec<Vec<Point2>>, Vec<Vec<bool>>, SolidOwner);
/// An edge as the keys of its two ends.
type EdgeKey = ((i64, i64, i64), (i64, i64, i64));

#[derive(Debug, Clone)]
struct Face {
    owner: SolidOwner,
    poly: Vec<Point2>,
    fill: [u8; 3],
    edges: Vec<(Point2, Point2)>,
    /// An opening line of a door or window (thin): `Some(true)` dashed, as
    /// when its leaves open away from the viewer.
    opening_line: Option<bool>,
    /// A plant drawn as architects draw them seen from the side, not by
    /// its mesh: its marks are made when the drawing is written, of
    /// presentation or technical as the view is (`poly` is only the box
    /// round it, to pick it).
    figure: Option<Figure>,
}

/// A plant, a person or a car seen from the side in a section or
/// elevation, drawn whole rather than face by face.
#[derive(Debug, Clone)]
enum Figure {
    /// A plant: its marks are made when the drawing is written.
    Plant {
        object: archisim_core::ObjectId,
        /// Its foot, in the drawing.
        at: Point2,
        /// The direction the section looks, in plan.
        toward: [f64; 2],
        /// The drawing's `x` runs to the viewer's left (marks are drawn
        /// to the viewer's right).
        flip: bool,
    },
    /// A person or a car: what each of its materials covers, the largest
    /// first, and its silhouette.
    Body {
        pieces: Vec<(MaterialId, Vec<archisim_geom::junctions::Shape>)>,
        outline: Vec<archisim_geom::junctions::Shape>,
    },
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

/// Whether `p` lies inside the polygon (even–odd).
fn inside(p: Point2, poly: &[Point2]) -> bool {
    let mut odd = false;
    let n = poly.len();
    for i in 0..n {
        let (a, b) = (poly[i], poly[(i + n - 1) % n]);
        if (a.y > p.y) != (b.y > p.y) && p.x < a.x + (p.y - a.y) / (b.y - a.y) * (b.x - a.x) {
            odd = !odd;
        }
    }
    odd
}

impl SectionDrawing {
    /// The element seen at `p` (drawing coordinates): one cut by the
    /// section there, or else the nearest one seen beyond it.
    pub fn pick(&self, p: Point2) -> Option<SolidOwner> {
        for (_, loops, _, owner) in self.cuts.iter().rev() {
            let n = loops.iter().filter(|l| inside(p, l)).count();
            if n % 2 == 1 {
                return Some(*owner);
            }
        }
        self.faces
            .iter()
            .rev()
            .find(|f| f.poly.len() >= 3 && inside(p, &f.poly))
            .map(|f| f.owner)
    }

    /// Whether an object is seen in the section (cut or beyond).
    pub fn pick_object(&self, id: archisim_core::ObjectId) -> bool {
        let me = SolidOwner::Object(id);
        self.cuts.iter().any(|c| c.3 == me) || self.faces.iter().any(|f| f.owner == me)
    }

    /// What an element looks like in the section, to highlight it: the
    /// loops of its cut, and its faces seen beyond (pieces of polygons).
    pub fn shape_of(&self, owner: SolidOwner) -> (Vec<Vec<Point2>>, Vec<Vec<Point2>>) {
        let cut = self
            .cuts
            .iter()
            .filter(|c| c.3 == owner)
            .flat_map(|c| c.1.iter().cloned())
            .collect();
        let faces = self
            .faces
            .iter()
            .filter(|f| f.owner == owner && f.poly.len() >= 3)
            .map(|f| f.poly.clone())
            .collect();
        (cut, faces)
    }
}

/// Works out what a section draws.
impl SectionDrawing {
    /// How far right of the drawing its level labels reach at a scale
    /// (they start a metre past it), metres.
    /// How far above its top the grid bubbles reach, model metres.
    pub fn grids_reach(&self, scale: f64) -> f64 {
        if self.grids.is_empty() {
            0.0
        } else {
            (2.0 * crate::grids::BUBBLE_RADIUS + 1.0) * scale / 1000.0
        }
    }

    pub fn labels_reach(&self, scale: f64) -> f64 {
        let k = scale / 1000.0;
        let h = annotate::TEXT_HEIGHT * k;
        let widest = self
            .levels
            .iter()
            .map(|(name, z)| {
                crate::lettering::text_width(
                    &format!("{} {}", annotate::format_elevation(*z), name),
                    h,
                )
            })
            .fold(0.0, f64::max);
        1.0 + 0.5 * k + widest + 2.0 * k
    }
}

/// The path of shapes with their holes (to fill with `evenodd`).
fn shapes_path(shapes: &[archisim_geom::junctions::Shape]) -> String {
    let mut d = String::new();
    for (outer, holes) in shapes {
        d.push_str(&path(outer));
        for h in holes {
            d.push(' ');
            d.push_str(&path(h));
        }
        d.push(' ');
    }
    d.trim_end().to_string()
}

/// A person or a car as architects draw them seen from the side: in a
/// presentation, each material in its colour; otherwise white, its
/// pieces (glass, wheels, face) in thin lines. Its outline over all.
fn draw_body(
    out: &mut String,
    model: &Model,
    pieces: &[(MaterialId, Vec<archisim_geom::junctions::Shape>)],
    outline: &[archisim_geom::junctions::Shape],
    presenting: bool,
    k: f64,
) {
    let whole = shapes_path(outline);
    if presenting {
        for (m, shapes) in pieces {
            let [r, g, b] = model.material(*m).map_or([200, 200, 200], |m| m.cut_color);
            let _ = writeln!(
                out,
                r#"<path d="{}" fill="rgb({r},{g},{b})" fill-rule="evenodd" stroke="none"/>"#,
                shapes_path(shapes)
            );
        }
    } else {
        let _ = writeln!(
            out,
            r#"<path d="{whole}" fill="white" fill-rule="evenodd" stroke="none"/>"#
        );
        for (_, shapes) in pieces.iter().skip(1) {
            let _ = writeln!(
                out,
                r#"<path d="{}" fill="none" stroke="{INK}" stroke-width="{}"/>"#,
                shapes_path(shapes),
                num(pens().thin * k)
            );
        }
    }
    let _ = writeln!(
        out,
        r#"<path d="{whole}" fill="none" stroke="{INK}" stroke-width="{}"/>"#,
        num(pens().view * k)
    );
}

/// The points of a mark (an arc by its ends and centre).
fn mark_points(m: &archisim_core::family::Mark) -> Vec<[f64; 2]> {
    use archisim_core::family::Mark;
    match m {
        Mark::Line { from, to, .. } => vec![*from, *to],
        Mark::Polygon { points, .. } => points.clone(),
        Mark::Arc {
            center, from, to, ..
        } => vec![*center, *from, *to],
    }
}

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
    let all = archisim_geom::solid::without_plants(|| model_solids(model));
    // Plants are drawn as figures (below), not by their mesh.
    let plants: std::collections::HashSet<archisim_core::ObjectId> = model
        .objects()
        .filter(|o| model.type_grows(o.object_type))
        .map(|o| o.id)
        .collect();
    // People and vehicles: drawn by their silhouette (below).
    let bodies: std::collections::HashSet<archisim_core::ObjectId> = model
        .objects()
        .filter(|o| !plants.contains(&o.id))
        .filter(|o| {
            model
                .object_type(o.object_type)
                .and_then(|t| model.family(t.family))
                .is_some_and(|f| matches!(f.def().category.as_str(), "people" | "vehicle"))
        })
        .map(|o| o.id)
        .collect();
    // Their faces seen from the section, flat on the drawing, by
    // material; and how deep they are.
    #[allow(clippy::type_complexity)]
    let mut body_faces: HashMap<
        archisim_core::ObjectId,
        (Vec<(MaterialId, Vec<[Point2; 3]>)>, f64, usize),
    > = HashMap::new();
    for solid in all {
        if matches!(solid.owner, SolidOwner::Object(o) if plants.contains(&o)) {
            continue;
        }
        if let SolidOwner::Object(o) = solid.owner {
            if bodies.contains(&o) {
                let entry = body_faces.entry(o).or_default();
                let mut tris = Vec::new();
                for t in &solid.triangles {
                    let n = cross(sub(t[1], t[0]), sub(t[2], t[0]));
                    if dot(n, view) > -1e-12 {
                        continue;
                    }
                    let ds = t.map(depth_of);
                    if ds.iter().all(|v| *v < plane) || ds.iter().all(|v| *v > sec.depth) {
                        continue;
                    }
                    entry.1 += ds.iter().sum::<f64>() / 3.0;
                    entry.2 += 1;
                    tris.push(t.map(to2));
                }
                if !tris.is_empty() {
                    match entry.0.iter_mut().find(|(m, _)| *m == solid.material) {
                        Some((_, v)) => v.extend(tris),
                        None => entry.0.push((solid.material, tris)),
                    }
                }
                continue;
            }
        }
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
        let crease = CREASE_COS;
        // A corner, or the outline of a rounded body (a crown, a car, a
        // person): the face on its other side looks away from the viewer.
        let is_corner = |a: P3, b: P3, me: usize| {
            let (a, b) = (key(a), key(b));
            let k = if a <= b { (a, b) } else { (b, a) };
            match edge_faces.get(&k) {
                Some(list) if list.len() >= 2 => list.iter().any(|&j| {
                    j != me
                        && (dot(normals[j], normals[me]) < crease || dot(normals[j], view) > -1e-3)
                }),
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
            d.cuts
                .push((solid.material, loops, solid_edges, solid.owner));
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
                        owner: solid.owner,
                        poly: poly2,
                        fill: if glass { GLASS } else { [255, 255, 255] },
                        edges: Vec::new(),
                        opening_line: None,
                        figure: None,
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
                            owner: solid.owner,
                            poly: Vec::new(),
                            fill: [0, 0, 0],
                            edges: vec![(to2(a), to2(b))],
                            opening_line: None,
                            figure: None,
                        },
                    ));
                }
            }
        }
    }
    // Plants behind the line and within the depth, as figures: standing
    // at their foot, at its depth among the faces.
    let right = Point2::new(look.y, -look.x);
    for &id in &plants {
        let Some(obj) = model.object(id) else {
            continue;
        };
        let z0 = model.level(obj.level).map_or(0.0, |l| l.elevation) + obj.base_offset;
        let foot = [obj.position.x, obj.position.y, z0];
        let depth = depth_of(foot);
        if depth < plane || depth > sec.depth {
            continue;
        }
        let toward = [look.x, look.y];
        let flip = right.dot(u) < 0.0;
        let at = to2(foot);
        // The box round its technical drawing, to pick it and to frame
        // the drawing.
        let Some(marks) = model.plant_side_marks(id, toward, false, 100.0) else {
            continue;
        };
        let (mut a, mut b) = ([f64::MAX; 2], [f64::MIN; 2]);
        for p in marks.iter().flat_map(mark_points) {
            for k in 0..2 {
                a[k] = a[k].min(p[k]);
                b[k] = b[k].max(p[k]);
            }
        }
        if a[0] > b[0] {
            continue;
        }
        let x = |v: f64| at.x + if flip { -v } else { v };
        let (x0, x1) = (x(a[0]).min(x(b[0])), x(a[0]).max(x(b[0])));
        let poly = vec![
            Point2::new(x0, at.y + a[1]),
            Point2::new(x1, at.y + a[1]),
            Point2::new(x1, at.y + b[1]),
            Point2::new(x0, at.y + b[1]),
        ];
        for q in &poly {
            grow_to(&mut lo, &mut hi, *q);
        }
        faces.push((
            depth,
            Face {
                owner: SolidOwner::Object(id),
                poly,
                fill: [255, 255, 255],
                edges: Vec::new(),
                opening_line: None,
                figure: Some(Figure::Plant {
                    object: id,
                    at,
                    toward,
                    flip,
                }),
            },
        ));
    }
    for (id, (by_material, depth_sum, count)) in body_faces {
        if count == 0 {
            continue;
        }
        // Traced on a centimetre grid: a fifth of a millimetre at 1:50.
        let cell = 0.01;
        let all: Vec<[Point2; 3]> = by_material.iter().flat_map(|(_, t)| t.clone()).collect();
        let outline = archisim_geom::cover::cover(&all, cell);
        if outline.is_empty() {
            continue;
        }
        let mut pieces: Vec<(MaterialId, Vec<archisim_geom::junctions::Shape>)> = by_material
            .iter()
            .map(|(m, t)| (*m, archisim_geom::cover::cover(t, cell)))
            .filter(|(_, s)| !s.is_empty())
            .collect();
        pieces.sort_by(|a, b| {
            archisim_geom::junctions::area(&b.1).total_cmp(&archisim_geom::junctions::area(&a.1))
        });
        let poly: Vec<Point2> = outline.iter().flat_map(|(o, _)| o.clone()).collect();
        let (mut a, mut b) = (
            Point2::new(f64::MAX, f64::MAX),
            Point2::new(f64::MIN, f64::MIN),
        );
        for q in &poly {
            a = Point2::new(a.x.min(q.x), a.y.min(q.y));
            b = Point2::new(b.x.max(q.x), b.y.max(q.y));
        }
        grow_to(&mut lo, &mut hi, a);
        grow_to(&mut lo, &mut hi, b);
        faces.push((
            depth_sum / count as f64,
            Face {
                owner: SolidOwner::Object(id),
                poly: vec![a, Point2::new(b.x, a.y), b, Point2::new(a.x, b.y)],
                fill: [255, 255, 255],
                edges: Vec::new(),
                opening_line: None,
                figure: Some(Figure::Body { pieces, outline }),
            },
        ));
    }
    // Opening lines of the doors and windows seen beyond the cut, on the
    // face that looks at the section: as drawn if their leaves open
    // towards it, dashed if they open away.
    for o in model.openings() {
        let (lines, opens) = archisim_geom::solid::opening_elevation_lines(model, o.id);
        let towards = opens[0] * look.x + opens[1] * look.y < 0.0;
        for l in lines {
            let (a, b) = if towards { l.front } else { l.back };
            let (da, db) = (depth_of(a), depth_of(b));
            if da.min(db) < plane || da.max(db) > sec.depth {
                continue;
            }
            faces.push((
                da.min(db) - EDGE_LEAD,
                Face {
                    owner: SolidOwner::Opening(o.id),
                    poly: Vec::new(),
                    fill: [0, 0, 0],
                    edges: vec![(to2(a), to2(b))],
                    opening_line: Some(l.dashed || !towards),
                    figure: None,
                },
            ));
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
        if sec.heights {
            d.heights = heights(model, sec, &d.levels, &d.cuts, lo.x);
            // Room for the chain of levels on the left.
            if d.heights.iter().any(|(_, x)| *x < lo.x) {
                lo = Point2::new(lo.x - 1.5, lo.y);
            }
        }
        d.lo = lo;
        d.hi = hi;
    } else {
        d.lo = Point2::new(0.0, 0.0);
        d.hi = Point2::new(o.distance(sec.end), 3.0);
    }
    d.grids = crate::grids::grids_crossing(model, sec.start, sec.end)
        .into_iter()
        .filter(|(x, _)| *x >= d.lo.x - 1e-6 && *x <= d.hi.x + 1e-6)
        .collect();
    d
}

/// The heights a section dimensions: the floor-to-floor heights of its
/// levels, in a chain a metre left of the drawing (`left`), and, in a
/// section (not an elevation), the clear height of each room the line cuts,
/// in the middle of the room: from the top of what is cut under its floor
/// to the underside of what is cut above (a room with its own height, a
/// false ceiling, up to it).
fn heights(
    model: &Model,
    sec: &SectionLine,
    levels: &[(String, f64)],
    cuts: &[CutPiece],
    left: f64,
) -> Vec<(Vec<Point2>, f64)> {
    // Heights at which the cut pieces cross the vertical through x.
    let crossings = |x: f64| -> Vec<f64> {
        let mut v = Vec::new();
        for (_, loops, _, _) in cuts {
            for l in loops {
                let n = l.len();
                for i in 0..n {
                    let (a, b) = (l[i], l[(i + 1) % n]);
                    if (a.x <= x) != (b.x <= x) && (b.x - a.x).abs() > 1e-12 {
                        v.push(a.y + (x - a.x) / (b.x - a.x) * (b.y - a.y));
                    }
                }
            }
        }
        v
    };
    let mut out = Vec::new();
    let mut zs: Vec<f64> = levels.iter().map(|(_, z)| *z).collect();
    zs.sort_by(f64::total_cmp);
    zs.dedup_by(|a, b| (*a - *b).abs() < 1e-6);
    if zs.len() >= 2 {
        let x = left - 1.0;
        out.push((zs.iter().map(|z| Point2::new(x, *z)).collect(), x));
    }
    if sec.kind != archisim_core::SectionKind::Section {
        return out;
    }
    let (Some(u), len) = (sec.along(), sec.start.distance(sec.end)) else {
        return out;
    };
    for r in model.rooms() {
        let Some(level) = model.level(r.level) else {
            continue;
        };
        // Where the line crosses the room's outline, along the line.
        let n = r.outline.len();
        let mut ts: Vec<f64> = Vec::new();
        for i in 0..n {
            let (a, b) = (r.outline[i], r.outline[(i + 1) % n]);
            let e = b - a;
            let denom = u.x * e.y - u.y * e.x;
            if denom.abs() < 1e-12 {
                continue;
            }
            let w = a - sec.start;
            let t = (w.x * e.y - w.y * e.x) / denom;
            let s = (w.x * u.y - w.y * u.x) / denom;
            if (0.0..1.0).contains(&s) {
                ts.push(t.clamp(0.0, len));
            }
        }
        ts.sort_by(f64::total_cmp);
        for pair in ts.chunks(2) {
            let [t0, t1] = pair else { continue };
            if t1 - t0 < 0.6 {
                continue;
            }
            let mid = (t0 + t1) / 2.0;
            if !r.contains(sec.start + u * mid) {
                continue;
            }
            let ys = crossings(mid);
            let base = level.elevation;
            // The floor: the top of what is cut just under the level.
            let z0 = ys
                .iter()
                .copied()
                .filter(|y| (*y - base).abs() <= 0.3)
                .fold(f64::NEG_INFINITY, f64::max);
            let z0 = if z0.is_finite() { z0 } else { base };
            let z1 = match r.height {
                Some(h) => base + h,
                None => {
                    let up = ys
                        .iter()
                        .copied()
                        .filter(|y| *y > z0 + 0.5)
                        .fold(f64::INFINITY, f64::min);
                    if up.is_finite() {
                        up
                    } else {
                        base + model.room_height(r)
                    }
                }
            };
            if z1 - z0 > 0.5 {
                out.push((vec![Point2::new(mid, z0), Point2::new(mid, z1)], mid));
            }
        }
    }
    out
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
                    num(pens().thin * k),
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
                    num(pens().cut * k)
                );
            }
            SectionKind::Elevation => {
                let _ = writeln!(
                    s,
                    r#"<path d="M{} L{}" fill="none" stroke="{INK}" stroke-width="{}" stroke-dasharray="{} {}"/>"#,
                    pt(a),
                    pt(b),
                    num(pens().thin * k),
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
                r#"<path d="M{} L{} L{} Z" fill="{INK}" stroke="{INK}" stroke-width="{}"/><circle cx="{}" cy="{}" r="{}" fill="white" stroke="{INK}" stroke-width="{}"/><path d="{}" {}/>"#,
                pt(h + u * r),
                pt(apex),
                pt(h - u * r),
                num(pens().thin * k),
                num(h.x),
                num(-h.y),
                num(r),
                num(pens().view * k),
                text_path(
                    &label,
                    h - Point2::new(0.0, th / 2.0),
                    Point2::new(1.0, 0.0),
                    th,
                    Align::Centre
                ),
                crate::lettering::paint(th, INK)
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
    // The building, not the plot.
    let solids = archisim_geom::solid::with_plant_detail(archisim_core::flora::Lod::Medium, || {
        model_solids(model)
    })
    .into_iter()
    .filter(|s| !matches!(s.owner, archisim_geom::solid::SolidOwner::Terrain(_)));
    for solid in solids {
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
    let _font = crate::lettering::use_font(model.graphics.font);
    let pens = options
        .pens
        .unwrap_or_else(|| model.graphics.pens(options.scale));
    crate::set_pens(pens);
    let _greyed = crate::dim::begin(model, &options.style);
    let look = crate::presentation::style_of(model, &options.style);
    let _poche = crate::presentation::begin(look);
    let mut svg = render_section_inner(model, d, options);
    crate::set_pens(archisim_core::Pens::FINE);
    if let Some(p) = look {
        svg = crate::presentation::apply(&svg, p);
    }
    svg
}

fn render_section_inner(model: &Model, d: &SectionDrawing, options: &PlanOptions) -> String {
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
            let extra = d.labels_reach(options.scale); // the level labels
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

    // Plants drawn in colour in a presentation view, in lines otherwise.
    let presenting = crate::presentation::style_of(model, &options.style).is_some();
    // Beyond: far to near.
    s.push_str("<g id=\"beyond\" stroke-linejoin=\"round\" stroke-linecap=\"round\">\n");
    let mut edges = String::new();
    let flush = |s: &mut String, edges: &mut String| {
        if !edges.is_empty() {
            let _ = writeln!(
                s,
                r#"<path d="{}" fill="none" stroke="{INK}" stroke-width="{}"/>"#,
                edges.trim_end(),
                num(pens().view * k)
            );
            edges.clear();
        }
    };
    // One face of what is seen beyond, into `out` (its loose edges into
    // `edges`, drawn together).
    let face = |out: &mut String, edges: &mut String, f: &Face| {
        if let Some(fig) = &f.figure {
            if !visible(&f.poly) {
                return;
            }
            flush(out, edges);
            match fig {
                Figure::Plant {
                    object,
                    at,
                    toward,
                    flip,
                } => {
                    if let Some(marks) =
                        model.plant_side_marks(*object, *toward, presenting, options.scale)
                    {
                        let world = |p: [f64; 2]| {
                            Point2::new(at.x + if *flip { -p[0] } else { p[0] }, at.y + p[1])
                        };
                        crate::draw_marks(out, model, marks, &world, k);
                    }
                }
                Figure::Body { pieces, outline } => {
                    draw_body(out, model, pieces, outline, presenting, k);
                }
            }
            return;
        }
        if let Some(dashed) = f.opening_line {
            flush(out, edges);
            let mut dashes = String::new();
            for (a, b) in &f.edges {
                if visible(&[*a, *b]) {
                    let _ = write!(dashes, "M{} L{} ", pt(*a), pt(*b));
                }
            }
            if !dashes.is_empty() {
                let pattern = if dashed {
                    format!(r#" stroke-dasharray="{} {}""#, num(2.0 * k), num(1.0 * k))
                } else {
                    String::new()
                };
                let _ = writeln!(
                    out,
                    r#"<path d="{}" fill="none" stroke="{INK}" stroke-width="{}"{pattern}/>"#,
                    dashes.trim_end(),
                    num(pens().thin * k),
                );
            }
            return;
        }
        if f.poly.is_empty() {
            for (a, b) in &f.edges {
                if visible(&[*a, *b]) {
                    let _ = write!(edges, "M{} L{} ", pt(*a), pt(*b));
                }
            }
            return;
        }
        if !visible(&f.poly) {
            return;
        }
        flush(out, edges);
        let [r, g, b] = f.fill;
        let _ = writeln!(
            out,
            r#"<path d="{}" fill="rgb({r},{g},{b})" stroke="rgb({r},{g},{b})" stroke-width="{}"/>"#,
            path(&f.poly),
            num(0.15 * k)
        );
    };
    for f in &d.faces {
        if crate::dim::is_hidden(f.owner.id()) {
            continue;
        }
        if crate::dim::is_dimmed(f.owner.id()) {
            // In grey, on its own.
            flush(&mut s, &mut edges);
            let (mut t, mut te) = (String::new(), String::new());
            face(&mut t, &mut te, f);
            flush(&mut t, &mut te);
            s.push_str(&crate::dim::grey(&t));
        } else {
            face(&mut s, &mut edges, f);
        }
    }
    flush(&mut s, &mut edges);
    s.push_str("</g>\n");

    // The cut, over everything.
    s.push_str("<g id=\"cut\" stroke-linejoin=\"miter\">\n");
    for (ci, (mat, loops, _, owner)) in d.cuts.iter().enumerate() {
        let Some(material) = model.material(*mat) else {
            continue;
        };
        crate::dim::element(&mut s, owner.id(), |s| {
            for (li, l) in loops.iter().enumerate() {
                if visible(l) {
                    fill_cut(s, material, l, &format!("cut-{ci}-{li}"), k);
                }
            }
        });
    }
    for (_, loops, flags, owner) in &d.cuts {
        if crate::dim::is_hidden(owner.id()) {
            continue;
        }
        let greyed = crate::dim::is_dimmed(owner.id());
        let mut t = String::new();
        let out = if greyed { &mut t } else { &mut s };
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
            for (d_, w) in [(thin, pens().thin), (heavy, pens().cut)] {
                if !d_.is_empty() {
                    let _ = writeln!(
                        out,
                        r#"<path d="{}" fill="none" stroke="{INK}" stroke-width="{}" stroke-linecap="round"/>"#,
                        d_.trim_end(),
                        num(w * k)
                    );
                }
            }
        }
        if greyed {
            s.push_str(&crate::dim::grey(&t));
        }
    }
    s.push_str("</g>\n");

    crate::grids::draw_section_grids(&mut s, &d.grids, d.lo.y, d.hi.y, k);
    // Levels.
    if !d.heights.is_empty() {
        s.push_str("<g id=\"heights\">\n");
        for (points, x) in &d.heights {
            let mid = (points[0].y + points[points.len() - 1].y) / 2.0;
            let dim = archisim_core::Dimension {
                id: archisim_core::DimensionId(0),
                level: archisim_core::LevelId(0),
                points: points.clone(),
                direction: Point2::new(0.0, 1.0),
                position: Point2::new(*x, mid),
                anchors: vec![None; points.len()],
            };
            annotate::draw_dimension(&mut s, &dim, k);
        }
        s.push_str("</g>\n");
    }
    s.push_str("<g id=\"levels\">\n");
    let (x0, x1) = (d.lo.x - 1.0, d.hi.x + 1.0);
    for (name, z) in &d.levels {
        let _ = writeln!(
            s,
            r#"<path d="M{} L{}" fill="none" stroke="{INK}" stroke-width="{}" stroke-dasharray="{} {} {} {}"/>"#,
            pt(Point2::new(x0, *z)),
            pt(Point2::new(x1, *z)),
            num(pens().thin * k),
            num(8.0 * k),
            num(1.5 * k),
            num(1.0 * k),
            num(1.5 * k)
        );
        let label = format!("{} {}", annotate::format_elevation(*z), name);
        let h = annotate::TEXT_HEIGHT * k;
        let _ = writeln!(
            s,
            r#"<path d="{}" {}/>"#,
            text_path(
                &label,
                Point2::new(x1 + 0.5 * k, *z + 0.8 * k),
                Point2::new(1.0, 0.0),
                h,
                Align::Left
            ),
            crate::lettering::paint(h, INK)
        );
    }
    s.push_str("</g>\n");

    if options.viewport.is_none() && options.title {
        // Title, in the technical lettering (no fonts needed).
        let h = TEXT_SIZE * k;
        let text = format!(
            "{} · {} · E 1:{}",
            model.project_name, d.title, options.scale
        );
        let _ = writeln!(
            s,
            r#"<path class="title" d="{}" {}/>"#,
            crate::lettering::text_path(
                &text,
                Point2::new(d.lo.x, d.lo.y - margin * 0.6 - h),
                Point2::new(1.0, 0.0),
                h,
                Align::Left
            ),
            crate::lettering::paint(h, INK)
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
        let mut walls = Vec::new();
        for (a, b) in [
            (Point2::new(0.0, 0.0), Point2::new(0.0, 5.0)),
            (Point2::new(4.0, 5.0), Point2::new(4.0, 0.0)),
            (Point2::new(10.0, 6.0), Point2::new(-2.0, 6.0)),
        ] {
            let w = m
                .add_wall(NewWall {
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
            walls.push(w);
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
        for (_, loops, _, _) in &brick_cuts {
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
        // Picked: the first wall where it is cut, the far wall between the
        // two, nothing above them.
        let wall = |i: usize| Some(SolidOwner::Wall(walls[i]));
        assert_eq!(d.pick(Point2::new(1.0, 1.5)), wall(0));
        assert_eq!(d.pick(Point2::new(3.0, 1.5)), wall(2));
        assert_eq!(d.pick(Point2::new(3.0, 3.5)), None);
        let (cut, faces) = d.shape_of(SolidOwner::Wall(walls[0]));
        assert_eq!(cut.len(), 2);
        assert!(faces.is_empty());
        assert!(!d.shape_of(SolidOwner::Wall(walls[2])).1.is_empty());

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
