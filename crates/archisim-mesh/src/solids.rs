//! The solids of a mesh, for volumes of the concept phase: a SketchUp
//! model of volumes is a soup of triangles, and each building in it is a
//! closed piece. [`solids`] welds the points that coincide (within a
//! tolerance), drops the faces exported twice (SketchUp's «two-sided
//! faces»), splits the triangles into pieces joined by their edges, turns
//! every triangle of a piece the same way (outwards), closes the flat
//! holes it can (the missing base of a volume) and gathers the triangles
//! of each flat face of one material into one face. [`extrusion`] tells
//! whether a solid is a contour pushed up, and [`flat_face`] whether an
//! open piece is a single horizontal face.

use std::collections::HashMap;

use crate::Mesh;

/// A piece of a mesh: closed (a solid) or open (a surface).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Solid {
    /// Metres, `z` up, as the mesh.
    pub positions: Vec<[f64; 3]>,
    /// Outwards counter-clockwise.
    pub triangles: Vec<[u32; 3]>,
    /// The face of each triangle: the flat faces of one material.
    pub faces: Vec<u32>,
    /// The material of each face (index into [`Mesh::materials`]).
    pub face_materials: Vec<usize>,
    /// Every edge between two of its triangles.
    pub closed: bool,
    /// Flat holes closed with new faces.
    pub holes_closed: usize,
    /// Edges of a single triangle left (zero when closed).
    pub open_edges: usize,
}

/// What [`solids`] found.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Solids {
    pub solids: Vec<Solid>,
    /// Triangles exported twice (both sides of a face) that were dropped.
    pub doubled: usize,
    /// Points joined to another within the tolerance.
    pub welded: usize,
}

/// A solid that is a contour pushed up: vertical sides, horizontal top and
/// bottom.
#[derive(Debug, Clone, PartialEq)]
pub struct Extrusion {
    /// Counter-clockwise, without points on a straight run.
    pub outline: Vec<[f64; 2]>,
    /// The height of its bottom.
    pub base: f64,
    pub height: f64,
    /// The face of the solid on each side (`outline[i]` → `outline[i + 1]`).
    pub side_faces: Vec<u32>,
    pub top_face: u32,
    pub bottom_face: u32,
}

/// Material names that are no material: what a program writes for a face
/// left with its default colour.
pub fn is_plain_material(name: &str) -> bool {
    let n = name
        .trim()
        .to_ascii_lowercase()
        .replace([' ', '_', '-'], "");
    n.is_empty()
        || matches!(
            n.as_str(),
            "default"
                | "material"
                | "defaultmaterial"
                | "frontcolor"
                | "backcolor"
                | "none"
                | "0"
                | "mat"
                | "surface"
        )
        || is_back_material(name)
}

/// SketchUp's colour of the back of the faces.
fn is_back_material(name: &str) -> bool {
    let n = name.to_ascii_lowercase();
    n.contains("backcolor") || n.contains("back_color") || n.starts_with("back")
}

fn sub(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

fn cross(u: [f64; 3], v: [f64; 3]) -> [f64; 3] {
    [
        u[1] * v[2] - u[2] * v[1],
        u[2] * v[0] - u[0] * v[2],
        u[0] * v[1] - u[1] * v[0],
    ]
}

fn dot(u: [f64; 3], v: [f64; 3]) -> f64 {
    u[0] * v[0] + u[1] * v[1] + u[2] * v[2]
}

fn length(u: [f64; 3]) -> f64 {
    dot(u, u).sqrt()
}

/// The normal of a triangle, as long as twice its area.
fn tri_cross(p: &[[f64; 3]], t: [u32; 3]) -> [f64; 3] {
    let [a, b, c] = t.map(|i| p[i as usize]);
    cross(sub(b, a), sub(c, a))
}

fn unit(u: [f64; 3]) -> [f64; 3] {
    let l = length(u);
    if l < 1e-300 {
        [0.0; 3]
    } else {
        [u[0] / l, u[1] / l, u[2] / l]
    }
}

/// An edge whatever its direction.
fn key(a: u32, b: u32) -> (u32, u32) {
    if a < b {
        (a, b)
    } else {
        (b, a)
    }
}

fn edges(t: [u32; 3]) -> [(u32, u32); 3] {
    [(t[0], t[1]), (t[1], t[2]), (t[2], t[0])]
}

/// Six times the signed volume a set of triangles closes (about the
/// origin of `p`).
fn signed_volume6(p: &[[f64; 3]], tris: &[[u32; 3]]) -> f64 {
    let o = p.first().copied().unwrap_or([0.0; 3]);
    tris.iter()
        .map(|t| {
            let [a, b, c] = t.map(|i| sub(p[i as usize], o));
            dot(a, cross(b, c))
        })
        .sum()
}

/// The volume of a closed solid, m³ (positive outwards).
pub fn volume(s: &Solid) -> f64 {
    signed_volume6(&s.positions, &s.triangles) / 6.0
}

/// The pieces of a mesh as solids, its points welded within `tolerance`
/// metres (a millimetre is usual).
pub fn solids(mesh: &Mesh, tolerance: f64) -> Solids {
    let tol = tolerance.max(1e-9);
    // ---- Weld --------------------------------------------------------------
    let mut points: Vec<[f64; 3]> = Vec::new();
    let mut cells: HashMap<(i64, i64, i64), Vec<u32>> = HashMap::new();
    let cell = |p: [f64; 3]| {
        (
            (p[0] / tol).floor() as i64,
            (p[1] / tol).floor() as i64,
            (p[2] / tol).floor() as i64,
        )
    };
    let mut welded = 0;
    let mut map: Vec<u32> = Vec::with_capacity(mesh.positions.len());
    for q in &mesh.positions {
        let p = q.map(f64::from);
        let c = cell(p);
        let mut found = None;
        'look: for dx in -1..=1 {
            for dy in -1..=1 {
                for dz in -1..=1 {
                    if let Some(list) = cells.get(&(c.0 + dx, c.1 + dy, c.2 + dz)) {
                        for &i in list {
                            if length(sub(points[i as usize], p)) <= tol {
                                found = Some(i);
                                break 'look;
                            }
                        }
                    }
                }
            }
        }
        let i = match found {
            Some(i) => {
                welded += 1;
                i
            }
            None => {
                points.push(p);
                let i = (points.len() - 1) as u32;
                cells.entry(c).or_default().push(i);
                i
            }
        };
        map.push(i);
    }
    // ---- Triangles, without the degenerate and the doubled ------------------
    let mut tris: Vec<([u32; 3], usize)> = Vec::new();
    let mut seen: HashMap<[u32; 3], usize> = HashMap::new();
    let mut doubled = 0;
    let min_cross = tol * tol * 1e-3;
    for g in &mesh.groups {
        let back = mesh
            .materials
            .get(g.material)
            .is_some_and(|m| is_back_material(&m.name));
        for t in &g.triangles {
            let t = t.map(|i| map[i as usize]);
            if t[0] == t[1] || t[1] == t[2] || t[0] == t[2] {
                continue;
            }
            if length(tri_cross(&points, t)) < min_cross {
                continue;
            }
            let mut k = t;
            k.sort_unstable();
            match seen.get(&k) {
                Some(&j) => {
                    doubled += 1;
                    // Keep the front of the face, not its back colour.
                    let kept_back = mesh
                        .materials
                        .get(tris[j].1)
                        .is_some_and(|m| is_back_material(&m.name));
                    if kept_back && !back {
                        tris[j] = (t, g.material);
                    }
                }
                None => {
                    seen.insert(k, tris.len());
                    tris.push((t, g.material));
                }
            }
        }
    }
    // ---- Pieces joined by edges of two triangles ----------------------------
    let mut by_edge: HashMap<(u32, u32), Vec<usize>> = HashMap::new();
    for (i, (t, _)) in tris.iter().enumerate() {
        for (a, b) in edges(*t) {
            by_edge.entry(key(a, b)).or_default().push(i);
        }
    }
    let mut parent: Vec<usize> = (0..tris.len()).collect();
    fn find(p: &mut [usize], mut i: usize) -> usize {
        while p[i] != i {
            p[i] = p[p[i]];
            i = p[i];
        }
        i
    }
    for list in by_edge.values() {
        if list.len() == 2 {
            let (a, b) = (find(&mut parent, list[0]), find(&mut parent, list[1]));
            if a != b {
                parent[a] = b;
            }
        }
    }
    let mut groups: HashMap<usize, Vec<usize>> = HashMap::new();
    for i in 0..tris.len() {
        let r = find(&mut parent, i);
        groups.entry(r).or_default().push(i);
    }
    let mut pieces: Vec<Vec<usize>> = groups.into_values().collect();
    // In the order of the file.
    pieces.sort_by_key(|p| p.iter().copied().min().unwrap_or(0));
    let solids = pieces
        .into_iter()
        .map(|p| {
            let local: Vec<([u32; 3], usize)> = p.iter().map(|&i| tris[i]).collect();
            piece(&points, local, tol)
        })
        .collect();
    Solids {
        solids,
        doubled,
        welded,
    }
}

/// One piece: its own points, turned outwards, holes closed, faces found.
fn piece(points: &[[f64; 3]], tris: Vec<([u32; 3], usize)>, tol: f64) -> Solid {
    // Its own points.
    let mut index: HashMap<u32, u32> = HashMap::new();
    let mut positions: Vec<[f64; 3]> = Vec::new();
    let mut triangles: Vec<[u32; 3]> = Vec::with_capacity(tris.len());
    let mut materials: Vec<usize> = Vec::with_capacity(tris.len());
    for (t, m) in tris {
        triangles.push(t.map(|i| {
            *index.entry(i).or_insert_with(|| {
                positions.push(points[i as usize]);
                (positions.len() - 1) as u32
            })
        }));
        materials.push(m);
    }
    let edge_map = |tris: &[[u32; 3]]| {
        let mut m: HashMap<(u32, u32), Vec<usize>> = HashMap::new();
        for (i, t) in tris.iter().enumerate() {
            for (a, b) in edges(*t) {
                m.entry(key(a, b)).or_default().push(i);
            }
        }
        m
    };
    // ---- The same way round, from the first triangle ------------------------
    let by_edge = edge_map(&triangles);
    let n = triangles.len();
    let mut done = vec![false; n];
    for start in 0..n {
        if done[start] {
            continue;
        }
        done[start] = true;
        let mut stack = vec![start];
        while let Some(i) = stack.pop() {
            let t = triangles[i];
            for (a, b) in edges(t) {
                let Some(list) = by_edge.get(&key(a, b)) else {
                    continue;
                };
                if list.len() != 2 {
                    continue;
                }
                let j = if list[0] == i { list[1] } else { list[0] };
                if done[j] {
                    continue;
                }
                // The neighbour runs the shared edge the other way.
                if edges(triangles[j]).contains(&(a, b)) {
                    triangles[j].swap(1, 2);
                }
                done[j] = true;
                stack.push(j);
            }
        }
    }
    // ---- Close the flat holes ------------------------------------------------
    let open = |tris: &[[u32; 3]]| {
        let m = edge_map(tris);
        let mut out: Vec<(u32, u32)> = Vec::new();
        for (i, t) in tris.iter().enumerate() {
            for (a, b) in edges(*t) {
                if m.get(&key(a, b)).is_some_and(|l| l.len() == 1 && l[0] == i) {
                    out.push((a, b));
                }
            }
        }
        out
    };
    let manifold = |tris: &[[u32; 3]]| edge_map(tris).values().all(|l| l.len() == 2);
    let mut holes_closed = 0;
    let boundary = open(&triangles);
    if !boundary.is_empty() {
        let mut filled = triangles.clone();
        let mut fill_mats = materials.clone();
        let mut closed_now = 0;
        for lp in loops(&boundary) {
            if let Some(new) = fill_flat(&positions, &lp, tol) {
                // The new face takes the material of the faces round it.
                let m = boundary_material(&triangles, &materials, &lp);
                fill_mats.extend(std::iter::repeat_n(m, new.len()));
                filled.extend(new);
                closed_now += 1;
            }
        }
        // A flat sheet closed on itself is still a sheet: leave it open.
        let bounds = bounds_of(&positions);
        let flat = signed_volume6(&positions, &filled).abs() / 6.0 < 1e-6_f64.max(bounds * 1e-9);
        if closed_now > 0 && !flat {
            triangles = filled;
            materials = fill_mats;
            holes_closed = closed_now;
        }
    }
    let open_edges = open(&triangles).len();
    let closed = open_edges == 0 && manifold(&triangles);
    // ---- Outwards --------------------------------------------------------------
    let v = signed_volume6(&positions, &triangles);
    let inwards = if v.abs() > 1e-9 {
        v < 0.0
    } else {
        // A sheet: its face up.
        let up: f64 = triangles.iter().map(|t| tri_cross(&positions, *t)[2]).sum();
        up < 0.0
    };
    if inwards {
        for t in &mut triangles {
            t.swap(1, 2);
        }
    }
    // ---- Flat faces of one material ------------------------------------------
    let by_edge = edge_map(&triangles);
    let normals: Vec<[f64; 3]> = triangles
        .iter()
        .map(|t| unit(tri_cross(&positions, *t)))
        .collect();
    let mut faces = vec![u32::MAX; triangles.len()];
    let mut face_materials: Vec<usize> = Vec::new();
    // Half a degree from the first triangle of the face.
    let same = (0.5f64).to_radians().cos();
    for seed in 0..triangles.len() {
        if faces[seed] != u32::MAX {
            continue;
        }
        let f = face_materials.len() as u32;
        face_materials.push(materials[seed]);
        faces[seed] = f;
        let n0 = normals[seed];
        let mut stack = vec![seed];
        while let Some(i) = stack.pop() {
            for (a, b) in edges(triangles[i]) {
                let Some(list) = by_edge.get(&key(a, b)) else {
                    continue;
                };
                if list.len() != 2 {
                    continue;
                }
                let j = if list[0] == i { list[1] } else { list[0] };
                if faces[j] == u32::MAX
                    && materials[j] == materials[seed]
                    && dot(normals[j], n0) >= same
                {
                    faces[j] = f;
                    stack.push(j);
                }
            }
        }
    }
    Solid {
        positions,
        triangles,
        faces,
        face_materials,
        closed,
        holes_closed,
        open_edges,
    }
}

fn bounds_of(p: &[[f64; 3]]) -> f64 {
    let mut lo = [f64::INFINITY; 3];
    let mut hi = [f64::NEG_INFINITY; 3];
    for q in p {
        for k in 0..3 {
            lo[k] = lo[k].min(q[k]);
            hi[k] = hi[k].max(q[k]);
        }
    }
    (0..3).map(|k| hi[k] - lo[k]).fold(0.0, f64::max)
}

/// The material most of the triangles round a hole have.
fn boundary_material(tris: &[[u32; 3]], mats: &[usize], lp: &[u32]) -> usize {
    let on: std::collections::HashSet<u32> = lp.iter().copied().collect();
    let mut count: HashMap<usize, usize> = HashMap::new();
    for (t, m) in tris.iter().zip(mats) {
        if t.iter().filter(|i| on.contains(i)).count() >= 2 {
            *count.entry(*m).or_default() += 1;
        }
    }
    count
        .into_iter()
        .max_by_key(|(m, c)| (*c, usize::MAX - *m))
        .map_or(mats.first().copied().unwrap_or(0), |(m, _)| m)
}

/// The loops of the open edges, each the way its new face runs (against
/// the triangles beside it). A point where two loops touch is left: those
/// loops are not closed.
fn loops(open: &[(u32, u32)]) -> Vec<Vec<u32>> {
    let mut next: HashMap<u32, Vec<u32>> = HashMap::new();
    for &(a, b) in open {
        // The hole runs b → a.
        next.entry(b).or_default().push(a);
    }
    let mut used: std::collections::HashSet<(u32, u32)> = std::collections::HashSet::new();
    let mut out = Vec::new();
    for &(a, b) in open {
        if used.contains(&(b, a)) {
            continue;
        }
        let start = b;
        let mut lp = vec![start];
        let mut cur = start;
        let mut ok = true;
        loop {
            let Some(n) = next.get(&cur) else {
                ok = false;
                break;
            };
            if n.len() != 1 {
                ok = false;
                break;
            }
            let nx = n[0];
            used.insert((cur, nx));
            if nx == start {
                break;
            }
            if lp.len() > open.len() {
                ok = false;
                break;
            }
            lp.push(nx);
            cur = nx;
        }
        if ok && lp.len() >= 3 {
            out.push(lp);
        }
    }
    out
}

/// Triangles closing a flat loop (each point within a centimetre of its
/// plane), or `None`.
fn fill_flat(p: &[[f64; 3]], lp: &[u32], tol: f64) -> Option<Vec<[u32; 3]>> {
    // Newell's normal: the loop's way round.
    let mut n = [0.0; 3];
    for k in 0..lp.len() {
        let a = p[lp[k] as usize];
        let b = p[lp[(k + 1) % lp.len()] as usize];
        n[0] += (a[1] - b[1]) * (a[2] + b[2]);
        n[1] += (a[2] - b[2]) * (a[0] + b[0]);
        n[2] += (a[0] - b[0]) * (a[1] + b[1]);
    }
    let n = unit(n);
    if length(n) < 0.5 {
        return None;
    }
    let o = p[lp[0] as usize];
    let reach = (10.0 * tol).max(0.01);
    if lp
        .iter()
        .any(|&i| dot(sub(p[i as usize], o), n).abs() > reach)
    {
        return None;
    }
    // Axes on the plane with u × v = n.
    let helper = if n[2].abs() < 0.9 {
        [0.0, 0.0, 1.0]
    } else {
        [1.0, 0.0, 0.0]
    };
    let u = unit(cross(helper, n));
    let v = cross(n, u);
    let pts: Vec<[f64; 2]> = lp
        .iter()
        .map(|&i| {
            let d = sub(p[i as usize], o);
            [dot(d, u), dot(d, v)]
        })
        .collect();
    let tris = ear_clip(&pts)?;
    Some(tris.into_iter().map(|t| t.map(|k| lp[k])).collect())
}

/// Triangles of a simple counter-clockwise polygon (ear clipping).
pub(crate) fn ear_clip(pts: &[[f64; 2]]) -> Option<Vec<[usize; 3]>> {
    let cross2 = |o: [f64; 2], a: [f64; 2], b: [f64; 2]| {
        (a[0] - o[0]) * (b[1] - o[1]) - (a[1] - o[1]) * (b[0] - o[0])
    };
    let area: f64 = (0..pts.len())
        .map(|i| {
            let (a, b) = (pts[i], pts[(i + 1) % pts.len()]);
            a[0] * b[1] - a[1] * b[0]
        })
        .sum();
    if area <= 0.0 {
        return None;
    }
    let mut idx: Vec<usize> = (0..pts.len()).collect();
    let mut out = Vec::new();
    let mut guard = 0;
    while idx.len() > 3 {
        guard += 1;
        if guard > 4 * pts.len() * pts.len() + 16 {
            return None;
        }
        let m = idx.len();
        let mut cut = None;
        for k in 0..m {
            let (ia, ib, ic) = (idx[(k + m - 1) % m], idx[k], idx[(k + 1) % m]);
            let (a, b, c) = (pts[ia], pts[ib], pts[ic]);
            if cross2(a, b, c) <= 1e-14 {
                continue;
            }
            let inside = idx.iter().any(|&j| {
                if j == ia || j == ib || j == ic {
                    return false;
                }
                let q = pts[j];
                cross2(a, b, q) >= 0.0 && cross2(b, c, q) >= 0.0 && cross2(c, a, q) >= 0.0
            });
            if !inside {
                cut = Some(k);
                break;
            }
        }
        // Points on a straight run: drop the middle one.
        let k = match cut {
            Some(k) => {
                out.push([idx[(k + m - 1) % m], idx[k], idx[(k + 1) % m]]);
                k
            }
            None => (0..m)
                .find(|&k| {
                    cross2(
                        pts[idx[(k + m - 1) % m]],
                        pts[idx[k]],
                        pts[idx[(k + 1) % m]],
                    )
                    .abs()
                        <= 1e-12
                })
                .ok_or(())
                .ok()?,
        };
        idx.remove(k);
    }
    if idx.len() == 3 && cross2(pts[idx[0]], pts[idx[1]], pts[idx[2]]) > 1e-14 {
        out.push([idx[0], idx[1], idx[2]]);
    }
    Some(out)
}

/// The outline of a set of horizontal triangles seen from above, if it is
/// a single loop: counter-clockwise, without the points on straight runs.
fn single_outline(p: &[[f64; 3]], tris: &[[u32; 3]], tol: f64) -> Option<Vec<[f64; 2]>> {
    let mut count: HashMap<(u32, u32), usize> = HashMap::new();
    for t in tris {
        for (a, b) in edges(*t) {
            *count.entry(key(a, b)).or_default() += 1;
        }
    }
    let mut next: HashMap<u32, u32> = HashMap::new();
    for t in tris {
        for (a, b) in edges(*t) {
            if count[&key(a, b)] == 1 && next.insert(a, b).is_some() {
                return None;
            }
        }
    }
    let &start = next.keys().min()?;
    let mut lp = vec![start];
    let mut cur = start;
    loop {
        let nx = *next.get(&cur)?;
        if nx == start {
            break;
        }
        if lp.len() > next.len() {
            return None;
        }
        lp.push(nx);
        cur = nx;
    }
    // Another loop: a hole or a second part.
    if lp.len() != next.len() {
        return None;
    }
    let mut pts: Vec<[f64; 2]> = lp
        .iter()
        .map(|&i| [p[i as usize][0], p[i as usize][1]])
        .collect();
    let area: f64 = (0..pts.len())
        .map(|i| {
            let (a, b) = (pts[i], pts[(i + 1) % pts.len()]);
            a[0] * b[1] - a[1] * b[0]
        })
        .sum();
    if area < 0.0 {
        pts.reverse();
    }
    // Without the points on a straight run.
    let mut changed = true;
    while changed && pts.len() > 3 {
        changed = false;
        let m = pts.len();
        for i in 0..m {
            let (a, q, b) = (pts[(i + m - 1) % m], pts[i], pts[(i + 1) % m]);
            let len = ((b[0] - a[0]).powi(2) + (b[1] - a[1]).powi(2)).sqrt();
            let off = if len < 1e-12 {
                0.0
            } else {
                ((b[0] - a[0]) * (q[1] - a[1]) - (b[1] - a[1]) * (q[0] - a[0])).abs() / len
            };
            if off <= tol.max(1e-6) {
                pts.remove(i);
                changed = true;
                break;
            }
        }
    }
    (pts.len() >= 3).then_some(pts)
}

/// Whether a closed solid is a contour pushed up: every point on its
/// bottom or its top, its sides vertical and its bottom a single outline
/// without holes.
pub fn extrusion(s: &Solid, tol: f64) -> Option<Extrusion> {
    if !s.closed || s.triangles.len() < 4 {
        return None;
    }
    let tol = tol.max(1e-6);
    let (lo, hi) = s
        .positions
        .iter()
        .fold((f64::INFINITY, f64::NEG_INFINITY), |(lo, hi), p| {
            (lo.min(p[2]), hi.max(p[2]))
        });
    let height = hi - lo;
    if height <= 10.0 * tol {
        return None;
    }
    let at = |z: f64| {
        if (z - lo).abs() <= tol {
            Some(false)
        } else if (z - hi).abs() <= tol {
            Some(true)
        } else {
            None
        }
    };
    if s.positions.iter().any(|p| at(p[2]).is_none()) {
        return None;
    }
    let mut bottom: Vec<[u32; 3]> = Vec::new();
    let mut top_area: HashMap<u32, f64> = HashMap::new();
    let mut bottom_area: HashMap<u32, f64> = HashMap::new();
    for (t, f) in s.triangles.iter().zip(&s.faces) {
        let c = tri_cross(&s.positions, *t);
        let area = length(c) / 2.0;
        let n = unit(c);
        let level: Vec<bool> = t
            .iter()
            .map(|&i| at(s.positions[i as usize][2]).unwrap_or(false))
            .collect();
        if level.iter().all(|&x| !x) {
            if n[2] > -0.999 {
                return None;
            }
            bottom.push(*t);
            *bottom_area.entry(*f).or_default() += area;
        } else if level.iter().all(|&x| x) {
            if n[2] < 0.999 {
                return None;
            }
            *top_area.entry(*f).or_default() += area;
        } else if n[2].abs() > 0.01 {
            return None;
        }
    }
    // Seen from above the bottom faces turn clockwise: the outline is
    // turned round in `single_outline`.
    let outline = single_outline(&s.positions, &bottom, tol)?;
    let most = |m: &HashMap<u32, f64>| {
        m.iter()
            .max_by(|a, b| a.1.total_cmp(b.1).then(b.0.cmp(a.0)))
            .map(|(f, _)| *f)
    };
    let top_face = most(&top_area)?;
    let bottom_face = most(&bottom_area)?;
    // The face on each side: the side triangles whose bottom edge lies on
    // it, the largest.
    let k = outline.len();
    let mut side_area: Vec<HashMap<u32, f64>> = vec![HashMap::new(); k];
    for (t, f) in s.triangles.iter().zip(&s.faces) {
        let low: Vec<[f64; 2]> = t
            .iter()
            .map(|&i| s.positions[i as usize])
            .filter(|p| at(p[2]) == Some(false))
            .map(|p| [p[0], p[1]])
            .collect();
        let high = t
            .iter()
            .filter(|&&i| at(s.positions[i as usize][2]) == Some(true))
            .count();
        if high == 0 || high == 3 {
            continue;
        }
        let area = length(tri_cross(&s.positions, *t)) / 2.0;
        // The middle of what it touches below (an edge, or a corner of a
        // triangle standing on its point: then its middle in plan).
        let mid = if low.len() == 2 {
            [(low[0][0] + low[1][0]) / 2.0, (low[0][1] + low[1][1]) / 2.0]
        } else {
            let ps: Vec<[f64; 3]> = t.iter().map(|&i| s.positions[i as usize]).collect();
            [
                (ps[0][0] + ps[1][0] + ps[2][0]) / 3.0,
                (ps[0][1] + ps[1][1] + ps[2][1]) / 3.0,
            ]
        };
        let best = (0..k)
            .map(|i| (i, seg_distance(mid, outline[i], outline[(i + 1) % k])))
            .min_by(|a, b| a.1.total_cmp(&b.1))?;
        if best.1 <= 10.0 * tol + 1e-6 {
            *side_area[best.0].entry(*f).or_default() += area;
        }
    }
    let side_faces = side_area
        .iter()
        .map(|m| most(m).unwrap_or(top_face))
        .collect();
    Some(Extrusion {
        outline,
        base: lo,
        height,
        side_faces,
        top_face,
        bottom_face,
    })
}

fn seg_distance(p: [f64; 2], a: [f64; 2], b: [f64; 2]) -> f64 {
    let d = [b[0] - a[0], b[1] - a[1]];
    let l2 = d[0] * d[0] + d[1] * d[1];
    let t = if l2 < 1e-24 {
        0.0
    } else {
        (((p[0] - a[0]) * d[0] + (p[1] - a[1]) * d[1]) / l2).clamp(0.0, 1.0)
    };
    let q = [a[0] + d[0] * t, a[1] + d[1] * t];
    ((p[0] - q[0]).powi(2) + (p[1] - q[1]).powi(2)).sqrt()
}

/// Whether an open piece is a single horizontal face: its outline
/// (counter-clockwise) and its height.
pub fn flat_face(s: &Solid, tol: f64) -> Option<(Vec<[f64; 2]>, f64)> {
    if s.closed {
        return None;
    }
    let z = s.positions.first()?[2];
    if s.positions.iter().any(|p| (p[2] - z).abs() > tol.max(1e-6)) {
        return None;
    }
    Some((single_outline(&s.positions, &s.triangles, tol)?, z))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Group, Material};

    fn mesh(points: &[[f32; 3]], tris: &[[u32; 3]]) -> Mesh {
        Mesh {
            positions: points.to_vec(),
            colors: vec![[200; 3]; points.len()],
            materials: vec![Material {
                name: "Blanco".into(),
                color: [240; 3],
                alpha: 1.0,
                texture: None,
            }],
            groups: vec![Group {
                material: 0,
                triangles: tris.to_vec(),
            }],
        }
    }

    /// A box made of separate triangles (each with its own points), some
    /// of them the wrong way round, and without its bottom.
    fn loose_box(x: f32, w: f32, h: f32) -> Mesh {
        let c = |i: u32| -> [f32; 3] {
            [
                x + if i & 1 != 0 { w } else { 0.0 },
                if i & 2 != 0 { w } else { 0.0 },
                if i & 4 != 0 { h } else { 0.0 },
            ]
        };
        let quads: [[u32; 4]; 5] = [
            [4, 5, 7, 6], // top
            [0, 1, 5, 4],
            [1, 3, 7, 5],
            [3, 2, 6, 7],
            [2, 0, 4, 6],
        ];
        let mut points = Vec::new();
        let mut tris = Vec::new();
        for (k, q) in quads.iter().enumerate() {
            for t in [[q[0], q[1], q[2]], [q[0], q[2], q[3]]] {
                let base = points.len() as u32;
                points.extend(t.map(c));
                // Every other face turned round.
                tris.push(if k % 2 == 0 {
                    [base, base + 1, base + 2]
                } else {
                    [base, base + 2, base + 1]
                });
            }
        }
        mesh(&points, &tris)
    }

    #[test]
    fn a_box_of_loose_triangles_becomes_a_closed_prism() {
        let m = loose_box(0.0, 4.0, 3.0);
        let s = solids(&m, 0.001);
        assert_eq!(s.solids.len(), 1);
        let b = &s.solids[0];
        assert!(b.closed);
        assert_eq!(b.holes_closed, 1);
        assert_eq!(b.positions.len(), 8);
        // Six flat faces.
        assert_eq!(b.face_materials.len(), 6);
        assert!((volume(b) - 48.0).abs() < 1e-6);
        let e = extrusion(b, 0.001).unwrap();
        assert_eq!(e.outline.len(), 4);
        assert!((e.height - 3.0).abs() < 1e-9);
        let area: f64 = (0..4)
            .map(|i| {
                let (a, b) = (e.outline[i], e.outline[(i + 1) % 4]);
                a[0] * b[1] - a[1] * b[0]
            })
            .sum::<f64>()
            / 2.0;
        assert!((area - 16.0).abs() < 1e-9);
        // Four different side faces, and top and bottom.
        let mut f = e.side_faces.clone();
        f.extend([e.top_face, e.bottom_face]);
        f.sort_unstable();
        f.dedup();
        assert_eq!(f.len(), 6);
    }

    #[test]
    fn two_apart_are_two_solids_and_doubled_faces_go() {
        let mut m = loose_box(0.0, 2.0, 2.0);
        let other = loose_box(5.0, 2.0, 2.0);
        let n = m.positions.len() as u32;
        m.positions.extend(&other.positions);
        m.colors.extend(&other.colors);
        let more: Vec<[u32; 3]> = other.groups[0]
            .triangles
            .iter()
            .map(|t| t.map(|i| i + n))
            .collect();
        // Both sides of the second box, as SketchUp exports them.
        let back: Vec<[u32; 3]> = more.iter().map(|t| [t[0], t[2], t[1]]).collect();
        m.groups[0].triangles.extend(more);
        m.groups[0].triangles.extend(back);
        let s = solids(&m, 0.001);
        assert_eq!(s.solids.len(), 2);
        assert_eq!(s.doubled, 10);
        assert!(s.solids.iter().all(|x| x.closed));
    }

    #[test]
    fn a_bent_open_piece_stays_open_and_a_flat_one_is_a_face() {
        // A box without its top and one side: its edge is not flat.
        let mut m = loose_box(0.0, 2.0, 2.0);
        m.groups[0].triangles.drain(0..4);
        let s = solids(&m, 0.001);
        assert_eq!(s.solids.len(), 1);
        assert!(!s.solids[0].closed);
        assert!(s.solids[0].open_edges > 0);
        assert!(extrusion(&s.solids[0], 0.001).is_none());
        // A single square: a face, not closed on itself.
        let sq = mesh(
            &[
                [0.0, 0.0, 1.0],
                [3.0, 0.0, 1.0],
                [3.0, 3.0, 1.0],
                [0.0, 3.0, 1.0],
            ],
            &[[0, 2, 1], [0, 3, 2]],
        );
        let s = solids(&sq, 0.001);
        assert!(!s.solids[0].closed);
        let (o, z) = flat_face(&s.solids[0], 0.001).unwrap();
        assert_eq!(o.len(), 4);
        assert!((z - 1.0).abs() < 1e-9);
    }

    fn fixture(name: &str) -> crate::Loaded {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures")
            .join(name);
        let mut l = crate::load(&path, None, crate::Options::default()).unwrap();
        l.guess_for_site();
        l
    }

    #[test]
    fn the_sketchup_volumes_are_two_prisms_and_a_house() {
        let l = fixture("volumenes.dae");
        let s = solids(&l.mesh, 0.001);
        assert_eq!(s.solids.len(), 3, "{s:?}");
        // The first block was exported with both sides of its faces.
        assert_eq!(s.doubled, 12);
        assert!(s.solids.iter().all(|x| x.closed));
        let a = extrusion(&s.solids[0], 0.001).expect("a prism");
        assert!((a.height - 9.0).abs() < 1e-3);
        assert_eq!(a.outline.len(), 4);
        // The tower had no bottom: closed.
        let b = &s.solids[1];
        assert_eq!(b.holes_closed, 1);
        let eb = extrusion(b, 0.001).expect("a prism");
        assert!((eb.height - 15.0).abs() < 1e-3);
        assert!(eb.outline.iter().any(|p| (p[0] - 28.0).abs() < 1e-3));
        // The house with its gabled roofs is not.
        let c = &s.solids[2];
        assert!(extrusion(c, 0.001).is_none());
        // The two wings (60 m² of section, 16 and 14 m long) less what
        // they share over their 8 × 8 m corner (384 m³ below the eaves and
        // 64 m³ where the two roofs cross).
        let v = volume(c);
        assert!((v - (60.0 * 30.0 - 448.0)).abs() < 0.5, "{v}");
        // Walls and roofs of different materials: more faces than the box.
        assert!(c.face_materials.len() >= 10);
        let names: Vec<&str> = c
            .face_materials
            .iter()
            .map(|m| l.mesh.materials[*m].name.as_str())
            .collect();
        assert!(names.iter().any(|n| n.contains("Teja")));
        assert!(names.iter().any(|n| n.contains("Fachada")));
    }

    #[test]
    fn a_curved_volume_and_an_open_piece() {
        let l = fixture("volumenes.obj");
        assert_eq!(l.guessed_unit, Some(1.0));
        let s = solids(&l.mesh, 0.001);
        assert_eq!(s.solids.len(), 2);
        let dome = &s.solids[0];
        assert!(dome.closed);
        assert!(extrusion(dome, 0.001).is_none());
        let open = &s.solids[1];
        assert!(!open.closed);
        assert_eq!(open.holes_closed, 0);
    }
}
