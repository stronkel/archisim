//! Sculpting the concept phase's volumes (bloque 4 de la revisión del
//! 22/10/2026): dividing a face along a line so each part can be painted
//! or pushed apart, pushing any face (or part of one) along its normal as
//! SketchUp's push/pull does, bending and twisting a volume, and sweeping
//! any profile along a path. Everything works on the closed meshes of
//! `forms::mass_body`, keeping the face of each triangle.

use archisim_core::{Deform, FaceRef, MassId, MassMesh, MassShape, MaterialId, Model, Point2};

use crate::forms::{mass_body, FormError, BOTTOM, SIDE, TOP};
use std::collections::HashMap;

type P3 = [f64; 3];

fn sub(a: P3, b: P3) -> P3 {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

fn add(a: P3, b: P3) -> P3 {
    [a[0] + b[0], a[1] + b[1], a[2] + b[2]]
}

fn scale(a: P3, k: f64) -> P3 {
    [a[0] * k, a[1] * k, a[2] * k]
}

fn dot(a: P3, b: P3) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

fn cross(a: P3, b: P3) -> P3 {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

fn norm(a: P3) -> Option<P3> {
    let l = dot(a, a).sqrt();
    (l > 1e-12).then(|| scale(a, 1.0 / l))
}

/// The normal of a face (its triangles' area-weighted normal), if it has
/// one.
pub fn face_normal(mesh: &MassMesh, face: u32) -> Option<P3> {
    let mut n = [0.0; 3];
    for (t, f) in mesh.triangles.iter().zip(&mesh.faces) {
        if *f == face {
            let [a, b, c] = t.map(|i| mesh.positions[i as usize]);
            n = add(n, cross(sub(b, a), sub(c, a)));
        }
    }
    norm(n)
}

/// Whether a face is flat: all its triangles face the same way.
pub fn face_is_flat(mesh: &MassMesh, face: u32) -> bool {
    let Some(n) = face_normal(mesh, face) else {
        return false;
    };
    mesh.triangles.iter().zip(&mesh.faces).all(|(t, f)| {
        if *f != face {
            return true;
        }
        let [a, b, c] = t.map(|i| mesh.positions[i as usize]);
        norm(cross(sub(b, a), sub(c, a))).is_none_or(|m| dot(m, n) > 0.999)
    })
}

/// Splits triangles by a plane: those of face `only` (or all of them).
/// Returns the mesh and, for each triangle, whether it lies on the side
/// the normal points to. Points cut on an edge are shared by the
/// triangles on both sides of it, so the mesh stays closed: where only one
/// face is cut, the triangles of the other faces along the cut edges are
/// split at the same points.
pub fn split(mesh: &MassMesh, point: P3, normal: P3, only: Option<u32>) -> (MassMesh, Vec<bool>) {
    let mut out = MassMesh {
        positions: mesh.positions.clone(),
        triangles: Vec::new(),
        faces: Vec::new(),
    };
    let mut sides = Vec::new();
    let dist: Vec<f64> = mesh
        .positions
        .iter()
        .map(|p| {
            let d = dot(sub(*p, point), normal);
            if d.abs() < 1e-9 {
                0.0
            } else {
                d
            }
        })
        .collect();
    let mut cuts: HashMap<(u32, u32), u32> = HashMap::new();
    let mut cut = |out: &mut MassMesh, a: u32, b: u32| -> u32 {
        let key = (a.min(b), a.max(b));
        if let Some(i) = cuts.get(&key) {
            return *i;
        }
        let (pa, pb) = (
            mesh.positions[key.0 as usize],
            mesh.positions[key.1 as usize],
        );
        let (da, db) = (dist[key.0 as usize], dist[key.1 as usize]);
        let t = da / (da - db);
        out.positions.push(add(pa, scale(sub(pb, pa), t)));
        let i = (out.positions.len() - 1) as u32;
        cuts.insert(key, i);
        i
    };
    for (t, f) in mesh.triangles.iter().zip(&mesh.faces) {
        let d = t.map(|i| dist[i as usize]);
        let crosses = d.iter().any(|x| *x > 0.0) && d.iter().any(|x| *x < 0.0);
        if !crosses || only.is_some_and(|o| o != *f) {
            out.triangles.push(*t);
            out.faces.push(*f);
            let s: f64 = d.iter().sum();
            sides.push(s > 0.0);
            continue;
        }
        let mut pos = Vec::new();
        let mut neg = Vec::new();
        for k in 0..3 {
            let (i, j) = (t[k], t[(k + 1) % 3]);
            let (di, dj) = (d[k], d[(k + 1) % 3]);
            if di >= 0.0 {
                pos.push(i);
            }
            if di <= 0.0 {
                neg.push(i);
            }
            if di * dj < 0.0 {
                let c = cut(&mut out, i, j);
                pos.push(c);
                neg.push(c);
            }
        }
        for (poly, side) in [(pos, true), (neg, false)] {
            for k in 1..poly.len().saturating_sub(1) {
                out.triangles.push([poly[0], poly[k], poly[k + 1]]);
                out.faces.push(*f);
                sides.push(side);
            }
        }
    }
    if only.is_some() && !cuts.is_empty() {
        // The other faces' triangles along a cut edge: split at its point.
        let mut k = 0;
        while k < out.triangles.len() {
            let t = out.triangles[k];
            let found = (0..3).find_map(|e| {
                let (a, b) = (t[e], t[(e + 1) % 3]);
                cuts.get(&(a.min(b), a.max(b))).map(|c| (e, *c))
            });
            if let Some((e, c)) = found {
                if !t.contains(&c) {
                    let (a, b, o) = (t[e], t[(e + 1) % 3], t[(e + 2) % 3]);
                    out.triangles[k] = [a, c, o];
                    out.triangles.push([c, b, o]);
                    out.faces.push(out.faces[k]);
                    sides.push(sides[k]);
                    continue;
                }
            }
            k += 1;
        }
    }
    (out, sides)
}

/// Divides a face by a plane through a line on it: the part on the side
/// the plane's normal points to becomes a new face (numbered after the
/// last). `None` if the plane leaves the face whole.
pub fn divide_face(mesh: &MassMesh, face: u32, point: P3, normal: P3) -> Option<(MassMesh, u32)> {
    let (mut m, sides) = split(mesh, point, normal, Some(face));
    let new = m.faces.iter().max().map_or(0, |f| f + 1);
    let mut moved = 0;
    let mut kept = 0;
    for (f, s) in m.faces.iter_mut().zip(&sides) {
        if *f == face {
            if *s {
                *f = new;
                moved += 1;
            } else {
                kept += 1;
            }
        }
    }
    (moved > 0 && kept > 0).then_some((m, new))
}

/// Pushes a flat face along its normal by `d` (out if positive), as
/// SketchUp's push/pull: the face moves and walls join it to where it
/// was. A wall in line with a face beside it joins that face; the others
/// are new faces (numbered after the last). Returns the mesh and the new
/// faces made.
pub fn push_face(mesh: &MassMesh, face: u32, d: f64) -> Option<(MassMesh, Vec<u32>)> {
    if d.abs() < 1e-6 || !face_is_flat(mesh, face) {
        return None;
    }
    let n = face_normal(mesh, face)?;
    let offset = scale(n, d);
    let mut out = mesh.clone();
    // Each corner of the face, moved.
    let mut moved: HashMap<u32, u32> = HashMap::new();
    for (t, f) in mesh.triangles.iter().zip(&mesh.faces) {
        if *f == face {
            for i in t {
                moved.entry(*i).or_insert_with(|| {
                    out.positions.push(add(mesh.positions[*i as usize], offset));
                    (out.positions.len() - 1) as u32
                });
            }
        }
    }
    // The edges of the face's border, with the face across each.
    let mut edges: HashMap<(u32, u32), u32> = HashMap::new();
    for (t, f) in mesh.triangles.iter().zip(&mesh.faces) {
        for e in 0..3 {
            edges.insert((t[e], t[(e + 1) % 3]), *f);
        }
    }
    let mut next = mesh.faces.iter().max().map_or(0, |f| f + 1);
    let mut made: Vec<u32> = Vec::new();
    // New walls by plane, to give the walls of one plane one face.
    let mut walls: Vec<(P3, f64, u32)> = Vec::new();
    let mut sides = Vec::new();
    for (t, f) in mesh.triangles.iter().zip(&mesh.faces) {
        if *f != face {
            continue;
        }
        for e in 0..3 {
            let (a, b) = (t[e], t[(e + 1) % 3]);
            let across = edges.get(&(b, a)).copied();
            if across == Some(face) {
                continue;
            }
            sides.push((a, b, across));
        }
    }
    for (a, b, across) in sides {
        let (pa, pb) = (mesh.positions[a as usize], mesh.positions[b as usize]);
        let Some(wn) = norm(cross(sub(pb, pa), offset)) else {
            continue;
        };
        // In line with the face across: the same face.
        let beside = across.filter(|o| {
            face_normal(mesh, *o).is_some_and(|on| dot(on, wn) > 0.999) && face_is_flat(mesh, *o)
        });
        let id = match beside {
            Some(o) => o,
            None => {
                let off = dot(wn, pa);
                match walls
                    .iter()
                    .find(|(m, o, _)| dot(*m, wn) > 0.999 && (o - off).abs() < 1e-6)
                {
                    Some((_, _, id)) => *id,
                    None => {
                        walls.push((wn, off, next));
                        made.push(next);
                        next += 1;
                        next - 1
                    }
                }
            }
        };
        let (a2, b2) = (moved[&a], moved[&b]);
        out.triangles.push([a, b, b2]);
        out.faces.push(id);
        out.triangles.push([a, b2, a2]);
        out.faces.push(id);
    }
    // The face itself, moved.
    for (t, f) in out
        .triangles
        .iter_mut()
        .zip(&out.faces)
        .take(mesh.triangles.len())
    {
        if *f == face {
            *t = t.map(|i| moved[&i]);
        }
    }
    Some((out, made))
}

/// How many level bands a twist and a bend need to read smooth.
pub fn deform_steps(d: &Deform) -> usize {
    ((d.twist.abs().to_degrees() / 2.5).max(d.bend.abs().to_degrees() / 2.0))
        .ceil()
        .clamp(8.0, 144.0) as usize
}

/// A volume bent and twisted up its height: sliced in level bands thin
/// enough to read curved, each turned round `centre` (twist) and bowed in
/// the direction given (bend), as if it were a soft bar.
pub fn deform(mesh: &MassMesh, d: &Deform, centre: Point2) -> MassMesh {
    let h = mesh.top();
    if h < 1e-6 || (d.twist.abs() < 1e-9 && d.bend.abs() < 1e-9) {
        return mesh.clone();
    }
    let steps = deform_steps(d);
    let mut m = mesh.clone();
    for k in 1..steps {
        let z = h * k as f64 / steps as f64;
        m = split(&m, [0.0, 0.0, z], [0.0, 0.0, 1.0], None).0;
    }
    let dir = [d.bend_direction.cos(), d.bend_direction.sin()];
    let perp = [-dir[1], dir[0]];
    for p in &mut m.positions {
        let t = (p[2] / h).clamp(0.0, 1.0);
        let (mut x, mut y) = (p[0] - centre.x, p[1] - centre.y);
        let mut z = p[2];
        if d.twist.abs() > 1e-9 {
            let a = d.twist * t;
            let (s, c) = a.sin_cos();
            (x, y) = (x * c - y * s, x * s + y * c);
        }
        if d.bend.abs() > 1e-9 {
            let u = x * dir[0] + y * dir[1];
            let w = x * perp[0] + y * perp[1];
            let r = h / d.bend;
            let th = d.bend * t;
            let nu = r - (r - u) * th.cos();
            z = (r - u) * th.sin();
            x = dir[0] * nu + perp[0] * w;
            y = dir[1] * nu + perp[1] * w;
        }
        *p = [x + centre.x, y + centre.y, z];
    }
    m
}

/// A profile swept along a path in plan, mitred at its corners. Each
/// point of the profile is (across the path, to its left; height). Faces:
/// the start 0, the end 1, then one along each side of the profile.
pub fn sweep(path: &[Point2], profile: &[Point2]) -> MassMesh {
    let mut prof = profile.to_vec();
    if path.len() < 2 || prof.len() < 3 {
        return MassMesh::default();
    }
    if archisim_core::signed_area(&prof) < 0.0 {
        prof.reverse();
    }
    let n = path.len();
    let dirs: Vec<Point2> = path
        .windows(2)
        .map(|w| (w[1] - w[0]).normalized().unwrap_or(Point2::new(1.0, 0.0)))
        .collect();
    let left = |d: Point2| Point2::new(-d.y, d.x);
    let mut m = MassMesh::default();
    let np = prof.len();
    for (i, at) in path.iter().enumerate() {
        let (l, k) = match (i.checked_sub(1).map(|j| dirs[j]), dirs.get(i).copied()) {
            (Some(a), Some(b)) => {
                let mid = left(a) + left(b);
                let l = mid.normalized().unwrap_or(left(b));
                // Longer across the corner, so the sides stay parallel.
                let c = (l.x * left(b).x + l.y * left(b).y).max(0.2);
                (l, 1.0 / c)
            }
            (Some(a), None) => (left(a), 1.0),
            (None, Some(b)) => (left(b), 1.0),
            (None, None) => (Point2::new(0.0, 1.0), 1.0),
        };
        for q in &prof {
            let p = *at + l * (q.x * k);
            m.positions.push([p.x, p.y, q.y]);
        }
    }
    let at = |i: usize, j: usize| (i * np + j % np) as u32;
    for i in 0..n - 1 {
        for j in 0..np {
            let face = 2 + j as u32;
            m.triangles.push([at(i, j), at(i, j + 1), at(i + 1, j + 1)]);
            m.faces.push(face);
            m.triangles.push([at(i, j), at(i + 1, j + 1), at(i + 1, j)]);
            m.faces.push(face);
        }
    }
    let caps = crate::solid::triangulate(&prof);
    for t in caps {
        let [a, b, c] = t;
        m.triangles.push([at(0, a), at(0, c), at(0, b)]);
        m.faces.push(0);
        m.triangles.push([at(n - 1, a), at(n - 1, b), at(n - 1, c)]);
        m.faces.push(1);
    }
    if crate::forms::mesh_volume(&m) < 0.0 {
        for t in &mut m.triangles {
            t.swap(1, 2);
        }
    }
    m
}

/// A profile swept round a closed path, mitred at every corner: a ring
/// with no ends. Faces: one along each side of the profile, from 2.
pub fn sweep_closed(path: &[Point2], profile: &[Point2]) -> MassMesh {
    let mut prof = profile.to_vec();
    let n = path.len();
    if n < 3 || prof.len() < 3 {
        return MassMesh::default();
    }
    if archisim_core::signed_area(&prof) < 0.0 {
        prof.reverse();
    }
    let dir = |i: usize| {
        (path[(i + 1) % n] - path[i])
            .normalized()
            .unwrap_or(Point2::new(1.0, 0.0))
    };
    let left = |d: Point2| Point2::new(-d.y, d.x);
    let np = prof.len();
    let mut m = MassMesh::default();
    for (i, at) in path.iter().enumerate() {
        let (a, b) = (dir((i + n - 1) % n), dir(i));
        let l = (left(a) + left(b)).normalized().unwrap_or(left(b));
        let k = 1.0 / (l.x * left(b).x + l.y * left(b).y).max(0.2);
        for q in &prof {
            let p = *at + l * (q.x * k);
            m.positions.push([p.x, p.y, q.y]);
        }
    }
    let at = |i: usize, j: usize| ((i % n) * np + j % np) as u32;
    for i in 0..n {
        for j in 0..np {
            let face = 2 + j as u32;
            m.triangles.push([at(i, j), at(i, j + 1), at(i + 1, j + 1)]);
            m.faces.push(face);
            m.triangles.push([at(i, j), at(i + 1, j + 1), at(i + 1, j)]);
            m.faces.push(face);
        }
    }
    if crate::forms::mesh_volume(&m) < 0.0 {
        for t in &mut m.triangles {
            t.swap(1, 2);
        }
    }
    m
}

/// A prism's contour with edge `i` moved out by `d` (in, if negative):
/// the edges beside it stretch or shrink to meet it. `None` if it would
/// turn inside out.
pub fn move_edge(outline: &[Point2], i: usize, d: f64) -> Option<Vec<Point2>> {
    let n = outline.len();
    if n < 3 || i >= n {
        return None;
    }
    let mut poly = outline.to_vec();
    let ccw = archisim_core::signed_area(&poly) > 0.0;
    let (a, b) = (poly[i], poly[(i + 1) % n]);
    let dir = (b - a).normalized()?;
    // Out of a counter-clockwise contour is to the right of its edges.
    let out = if ccw {
        Point2::new(dir.y, -dir.x)
    } else {
        Point2::new(-dir.y, dir.x)
    };
    let (a2, b2) = (a + out * d, b + out * d);
    // Where the moved edge meets the lines of the edges beside it.
    let meet = |p: Point2, q: Point2, fallback: Point2| -> Point2 {
        let r = q - p;
        let den = r.cross(dir);
        if den.abs() < 1e-9 {
            return fallback;
        }
        let t = (a2 - p).cross(dir) / den;
        p + r * t
    };
    let prev = poly[(i + n - 1) % n];
    let next = poly[(i + 2) % n];
    let na = meet(prev, a, a2);
    let nb = meet(next, b, b2);
    poly[i] = na;
    poly[(i + 1) % n] = nb;
    let area = archisim_core::signed_area(&poly);
    (area.abs() > 1e-6 && (area > 0.0) == ccw).then_some(poly)
}

/// Ready profiles for Seguir, `width` across and `height` up (the
/// rectangle is the band's own, without a profile).
pub fn preset_profile(name: &str, width: f64, height: f64) -> Option<Vec<Point2>> {
    let (w, h) = (width / 2.0, height);
    let p = |x: f64, y: f64| Point2::new(x * w, y * h);
    Some(match name {
        "tubo" => (0..24)
            .map(|k| {
                let a = std::f64::consts::TAU * k as f64 / 24.0;
                p(a.cos(), 0.5 + 0.5 * a.sin())
            })
            .collect(),
        "L" => vec![
            p(-1.0, 0.0),
            p(1.0, 0.0),
            p(1.0, 0.25),
            p(-0.5, 0.25),
            p(-0.5, 1.0),
            p(-1.0, 1.0),
        ],
        "U" => vec![
            p(-1.0, 0.0),
            p(1.0, 0.0),
            p(1.0, 1.0),
            p(0.7, 1.0),
            p(0.7, 0.25),
            p(-0.7, 0.25),
            p(-0.7, 1.0),
            p(-1.0, 1.0),
        ],
        "doble T" => vec![
            p(-1.0, 0.0),
            p(1.0, 0.0),
            p(1.0, 0.12),
            p(0.15, 0.12),
            p(0.15, 0.88),
            p(1.0, 0.88),
            p(1.0, 1.0),
            p(-1.0, 1.0),
            p(-1.0, 0.88),
            p(-0.15, 0.88),
            p(-0.15, 0.12),
            p(-1.0, 0.12),
        ],
        "cornisa" => {
            // Against the wall at the right; a cove under the top's drip.
            let mut v = vec![p(1.0, 0.2), p(1.0, 1.0), p(-1.0, 1.0)];
            for k in 0..=8 {
                let a = std::f64::consts::FRAC_PI_2 * k as f64 / 8.0;
                v.push(p(-1.0 + 1.6 * (1.0 - a.cos()), 0.8 - 0.6 * a.sin()));
            }
            v
        }
        "media caña" => {
            let mut v = vec![p(-1.0, 0.0), p(1.0, 0.0)];
            // The arc between the two ends of the base.
            for k in 1..12 {
                let a = std::f64::consts::PI * k as f64 / 12.0;
                v.push(p(a.cos(), a.sin()));
            }
            v
        }
        _ => return None,
    })
}

/// The names of the ready profiles, as shown.
pub const PRESETS: [&str; 6] = ["tubo", "L", "U", "doble T", "cornisa", "media caña"];

/// The face looking up under a point in plan, highest first, and its
/// height there (over the solid's base).
pub fn top_face_at(mesh: &MassMesh, p: Point2) -> Option<(u32, f64)> {
    let mut best: Option<(u32, f64)> = None;
    for (t, f) in mesh.triangles.iter().zip(&mesh.faces) {
        let [a, b, c] = t.map(|i| mesh.positions[i as usize]);
        let n = cross(sub(b, a), sub(c, a));
        if n[2] <= 1e-12 {
            continue;
        }
        // Barycentric coordinates in plan.
        let den = (b[1] - c[1]) * (a[0] - c[0]) + (c[0] - b[0]) * (a[1] - c[1]);
        if den.abs() < 1e-15 {
            continue;
        }
        let l1 = ((b[1] - c[1]) * (p.x - c[0]) + (c[0] - b[0]) * (p.y - c[1])) / den;
        let l2 = ((c[1] - a[1]) * (p.x - c[0]) + (a[0] - c[0]) * (p.y - c[1])) / den;
        let l3 = 1.0 - l1 - l2;
        if l1 < -1e-9 || l2 < -1e-9 || l3 < -1e-9 {
            continue;
        }
        let z = l1 * a[2] + l2 * b[2] + l3 * c[2];
        if best.is_none_or(|(_, bz)| z > bz) {
            best = Some((*f, z));
        }
    }
    best
}

/// The edges round a face: those of its triangles that no other of its
/// triangles shares.
pub fn face_border(mesh: &MassMesh, face: u32) -> Vec<[P3; 2]> {
    let mut edges: std::collections::HashSet<(u32, u32)> = std::collections::HashSet::new();
    for (t, f) in mesh.triangles.iter().zip(&mesh.faces) {
        if *f == face {
            for e in 0..3 {
                edges.insert((t[e], t[(e + 1) % 3]));
            }
        }
    }
    edges
        .iter()
        .filter(|(a, b)| !edges.contains(&(*b, *a)))
        .map(|(a, b)| [mesh.positions[*a as usize], mesh.positions[*b as usize]])
        .collect()
}

/// The upright face nearest a point in plan, within `reach`: its number,
/// the point on it nearest (at mid height), its normal and how far.
pub fn side_face_near(mesh: &MassMesh, p: Point2, reach: f64) -> Option<(u32, P3, P3, f64)> {
    let mut best: Option<(u32, P3, f64)> = None;
    for (t, f) in mesh.triangles.iter().zip(&mesh.faces) {
        let [a, b, c] = t.map(|i| mesh.positions[i as usize]);
        let Some(n) = norm(cross(sub(b, a), sub(c, a))) else {
            continue;
        };
        if n[2].abs() > 0.3 {
            continue;
        }
        // Seen from above, the triangle is a segment.
        let pts = [a, b, c].map(|q| Point2::new(q[0], q[1]));
        let (mut i, mut j, mut far) = (0, 1, -1.0);
        for x in 0..3 {
            for y in x + 1..3 {
                let d = pts[x].distance(pts[y]);
                if d > far {
                    (i, j, far) = (x, y, d);
                }
            }
        }
        if far < 1e-9 {
            continue;
        }
        let (s0, s1) = (pts[i], pts[j]);
        let v = s1 - s0;
        let t = ((p - s0).x * v.x + (p - s0).y * v.y) / (v.x * v.x + v.y * v.y);
        let q = s0 + v * t.clamp(0.0, 1.0);
        let d = q.distance(p);
        if d <= reach && best.is_none_or(|b| d < b.2) {
            let z = (a[2] + b[2] + c[2]) / 3.0;
            best = Some((*f, [q.x, q.y, z], d));
        }
    }
    let (face, point, d) = best?;
    Some((face, point, face_normal(mesh, face)?, d))
}

/// Turns a volume into a solid of triangles as it is seen (its twist,
/// edges and profile kept in the shape), each face keeping its paint.
/// Returns the solid.
pub fn mass_to_mesh(model: &mut Model, id: MassId) -> Result<MassMesh, FormError> {
    let m = model.mass(id).ok_or(FormError::Missing)?.clone();
    if m.building.is_some() {
        return Err(FormError::Building);
    }
    let body = mass_body(&m);
    let plain_mesh = matches!(m.shape, MassShape::Mesh(_)) && m.deform.is_none_or(|d| d.is_none());
    if plain_mesh {
        return Ok(body);
    }
    let paints: Vec<(u32, MaterialId)> = body
        .face_ids()
        .into_iter()
        .filter_map(|f| model.face_paint(id.0, m.face_ref(f)).map(|p| (f, p)))
        .collect();
    let old: Vec<FaceRef> = model
        .face_paints()
        .filter(|(e, _, _)| *e == id.0)
        .map(|(_, f, _)| f)
        .collect();
    for f in old {
        model.set_face_paint(id.0, f, None)?;
    }
    let height = body.top();
    let shape = MassShape::Mesh(body.clone());
    model.edit_mass(id, |x| {
        x.shape = shape;
        x.height = height;
        x.finish = None;
        x.deform = None;
        x.floor_height = None;
    })?;
    for (f, p) in paints {
        model.set_face_paint(id.0, FaceRef::Part(f), Some(p))?;
    }
    Ok(body)
}

/// Puts a solid in a volume, raising it so it starts at its base (a face
/// pushed down lowers the base instead).
fn set_mesh(model: &mut Model, id: MassId, mut mesh: MassMesh) -> Result<(), FormError> {
    let low = mesh.positions.iter().map(|p| p[2]).fold(f64::MAX, f64::min);
    let low = if low.is_finite() { low } else { 0.0 };
    if low.abs() > 1e-9 {
        for p in &mut mesh.positions {
            p[2] -= low;
        }
    }
    let height = mesh.top();
    model.edit_mass(id, |x| {
        x.shape = MassShape::Mesh(mesh);
        x.height = height;
        x.base_offset += low;
    })?;
    Ok(())
}

/// Divides a face of a volume along a line on it: `point` on the line
/// (`z` over the volume's base) and `normal`, the normal of the plane
/// through the line that cuts the face. The part on the normal's side is
/// a new face, painted as the face was. Returns its number.
pub fn divide_mass_face(
    model: &mut Model,
    id: MassId,
    face: u32,
    point: P3,
    normal: P3,
) -> Result<u32, FormError> {
    let m = model.mass(id).ok_or(FormError::Missing)?;
    let paint = model.face_paint(id.0, m.face_ref(face));
    let (divided, new) =
        divide_face(&mass_body(m), face, point, normal).ok_or(FormError::NotDivided)?;
    mass_to_mesh(model, id)?;
    set_mesh(model, id, divided)?;
    if let Some(p) = paint {
        model.set_face_paint(id.0, FaceRef::Part(new), Some(p))?;
    }
    Ok(new)
}

/// Pushes a face of a volume out along its normal by `d` (in, if
/// negative). A contour's top changes its height, its base its base, and
/// a side moves its edge, so it stays a contour; any other face, or a
/// part of one, makes it a solid of triangles.
pub fn push_mass_face(model: &mut Model, id: MassId, face: u32, d: f64) -> Result<(), FormError> {
    let m = model.mass(id).ok_or(FormError::Missing)?.clone();
    if m.building.is_some() {
        return Err(FormError::Building);
    }
    let contour = matches!(m.shape, MassShape::Prism(_)) && m.deform.is_none_or(|x| x.is_none());
    if contour && face == TOP {
        if m.height + d < -1e-9 {
            return Err(FormError::NothingLeft);
        }
        model.edit_mass(id, |x| x.height = (x.height + d).max(0.0))?;
        return Ok(());
    }
    if contour && face == BOTTOM && m.finish.is_none() {
        if m.height + d < 1e-6 {
            return Err(FormError::NothingLeft);
        }
        model.edit_mass(id, |x| {
            x.base_offset -= d;
            x.height += d;
        })?;
        return Ok(());
    }
    if let (true, MassShape::Prism(outline), None) = (contour, &m.shape, m.finish) {
        if face >= SIDE {
            let moved =
                move_edge(outline, (face - SIDE) as usize, d).ok_or(FormError::NothingLeft)?;
            model.edit_mass(id, |x| x.shape = MassShape::Prism(moved))?;
            return Ok(());
        }
    }
    let (pushed, _) = push_face(&mass_body(&m), face, d).ok_or(FormError::NotFlat)?;
    mass_to_mesh(model, id)?;
    set_mesh(model, id, pushed)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::forms::mesh_volume;

    fn closed(m: &MassMesh) -> bool {
        let mut count: HashMap<(u32, u32), i32> = HashMap::new();
        for t in &m.triangles {
            for (a, b) in [(t[0], t[1]), (t[1], t[2]), (t[2], t[0])] {
                *count.entry((a, b)).or_default() += 1;
                *count.entry((b, a)).or_default() -= 1;
            }
        }
        count.values().all(|v| *v == 0)
    }

    fn cube(s: f64) -> MassMesh {
        let sq = vec![
            Point2::new(0.0, 0.0),
            Point2::new(s, 0.0),
            Point2::new(s, s),
            Point2::new(0.0, s),
        ];
        crate::forms::prism_body(&sq, s, None)
    }

    #[test]
    fn faces_divided_and_pushed() {
        let c = cube(4.0);
        assert!(closed(&c));
        // The top divided across at x = 1: two faces, still closed.
        let (d, new) =
            divide_face(&c, crate::forms::TOP, [1.0, 0.0, 4.0], [-1.0, 0.0, 0.0]).expect("divided");
        assert!(closed(&d), "the cut edge is shared");
        assert!(d.faces.contains(&new) && d.faces.contains(&crate::forms::TOP));
        // The strip pushed up 2 m: the volume grows by 1 × 4 × 2.
        let (p, made) = push_face(&d, new, 2.0).expect("pushed");
        assert!(closed(&p));
        assert!(
            (mesh_volume(&p) - (64.0 + 8.0)).abs() < 1e-9,
            "{}",
            mesh_volume(&p)
        );
        // The walls in line with the cube's sides join them; the one over
        // the cut is new.
        assert_eq!(made.len(), 1, "{made:?}");
        // Pushed in: a recess.
        let (r, _) = push_face(&d, new, -1.0).expect("pushed in");
        assert!(closed(&r));
        assert!((mesh_volume(&r) - (64.0 - 4.0)).abs() < 1e-9);
        // A side pushed out.
        let (s, made) = push_face(&c, crate::forms::SIDE, 1.0).expect("side");
        assert!(closed(&s));
        assert!((mesh_volume(&s) - 80.0).abs() < 1e-9);
        assert!(
            made.is_empty(),
            "its walls are in line with the cube's faces"
        );
    }

    #[test]
    fn bent_and_twisted() {
        let c = cube(2.0);
        let d = Deform {
            twist: 90f64.to_radians(),
            bend: 0.0,
            bend_direction: 0.0,
        };
        let t = deform(&c, &d, Point2::new(1.0, 1.0));
        assert!(closed(&t));
        // Twisting keeps the volume (nearly: the bands are flat).
        assert!((mesh_volume(&t) - 8.0).abs() < 0.3, "{}", mesh_volume(&t));
        // The top is turned a quarter: its corner (2, 2) went to (0, 2).
        let top: Vec<_> = t
            .positions
            .iter()
            .filter(|p| (p[2] - 2.0).abs() < 1e-9)
            .collect();
        assert!(top
            .iter()
            .any(|p| p[0].abs() < 1e-9 && (p[1] - 2.0).abs() < 1e-9));
        let b = Deform {
            twist: 0.0,
            bend: 90f64.to_radians(),
            bend_direction: 0.0,
        };
        let bent = deform(&c, &b, Point2::new(1.0, 1.0));
        assert!(closed(&bent));
        // Bent a quarter towards +x: the top lies on its side.
        let max_x = bent.positions.iter().map(|p| p[0]).fold(f64::MIN, f64::max);
        assert!(max_x > 2.0, "{max_x}");
    }

    #[test]
    fn profiles_swept_along_paths() {
        let path = [
            Point2::new(0.0, 0.0),
            Point2::new(10.0, 0.0),
            Point2::new(10.0, 6.0),
        ];
        let tube = preset_profile("tubo", 1.0, 1.0).unwrap();
        let m = sweep(&path, &tube);
        assert!(closed(&m));
        // About π·0.5²·16 m (a bit less: the polygon, and the mitre).
        let v = mesh_volume(&m);
        assert!(v > 11.0 && v < 13.0, "{v}");
        for name in PRESETS {
            let p = preset_profile(name, 0.6, 0.4).unwrap();
            let m = sweep(&path, &p);
            assert!(closed(&m), "{name}");
            assert!(mesh_volume(&m) > 0.0, "{name}");
        }
    }

    #[test]
    fn volumes_divided_and_pushed_in_the_model() {
        let mut m = Model::new("Prueba");
        let level = m.add_level("Planta baja", 0.0);
        let mat = m.add_material(
            "Hormigón",
            "Estructura",
            [200, 200, 200],
            archisim_core::CutPattern::Solid,
        );
        let id = m
            .add_mass(archisim_core::NewMass {
                name: "Bloque".into(),
                level,
                shape: MassShape::Prism(vec![
                    Point2::new(0.0, 0.0),
                    Point2::new(4.0, 0.0),
                    Point2::new(4.0, 4.0),
                    Point2::new(0.0, 4.0),
                ]),
                base_offset: 0.0,
                height: 3.0,
                floor_height: None,
                material: mat,
            })
            .unwrap();
        // A side pushed out: still a contour, 1 m longer.
        push_mass_face(&mut m, id, SIDE + 1, 1.0).unwrap();
        let x = m.mass(id).unwrap();
        assert!(matches!(x.shape, MassShape::Prism(_)));
        assert!((archisim_core::signed_area(&x.outline()) - 20.0).abs() < 1e-9);
        // The top divided at x = 2 and its west part pushed up 2 m.
        let new = divide_mass_face(&mut m, id, TOP, [2.0, 0.0, 3.0], [-1.0, 0.0, 0.0]).unwrap();
        assert!(matches!(m.mass(id).unwrap().shape, MassShape::Mesh(_)));
        push_mass_face(&mut m, id, new, 2.0).unwrap();
        let x = m.mass(id).unwrap();
        assert!((x.height - 5.0).abs() < 1e-9);
        let v = crate::forms::mesh_volume(&mass_body(x));
        assert!((v - (60.0 + 2.0 * 4.0 * 2.0)).abs() < 1e-6, "{v}");
        // Its base pushed down 1 m: the base lowers.
        let low = crate::forms::face_at(x, [1.0, 1.0, 0.0], [0.0, 0.0, -1.0]).unwrap();
        push_mass_face(&mut m, id, low, 1.0).unwrap();
        let x = m.mass(id).unwrap();
        assert!((x.base_offset + 1.0).abs() < 1e-9);
        assert!((x.height - 6.0).abs() < 1e-9);
    }

    #[test]
    fn edges_of_prisms_moved() {
        let sq = vec![
            Point2::new(0.0, 0.0),
            Point2::new(4.0, 0.0),
            Point2::new(4.0, 4.0),
            Point2::new(0.0, 4.0),
        ];
        // The east edge (1 → 2) out 1 m.
        let m = move_edge(&sq, 1, 1.0).unwrap();
        assert!((archisim_core::signed_area(&m) - 20.0).abs() < 1e-9);
        assert!(m[1].approx_eq(Point2::new(5.0, 0.0)));
        // In more than its width: no.
        assert!(move_edge(&sq, 1, -5.0).is_none());
    }
}
