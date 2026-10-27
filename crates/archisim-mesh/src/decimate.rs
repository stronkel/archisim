//! Lighter versions of a mesh that keep its shape: edges are collapsed
//! one by one, the one that moves the surface least first (each corner
//! carries the sum of the planes of its triangles, Garland and Heckbert's
//! quadric error). The edges between materials and the open borders of
//! the model hold their line, and no triangle may turn over. A corner
//! keeps its colour, so textures sampled at the corners stay where they
//! were.

use std::cmp::Ordering;
use std::collections::BinaryHeap;
use std::hash::{BuildHasherDefault, Hasher};

/// A quick hash for the numbers used as keys here (the standard one is
/// made to resist attacks, which millions of corners do not need).
#[derive(Default)]
struct Quick(u64);

impl Hasher for Quick {
    fn finish(&self) -> u64 {
        self.0
    }

    fn write(&mut self, bytes: &[u8]) {
        for b in bytes {
            self.write_u64(*b as u64);
        }
    }

    fn write_u8(&mut self, n: u8) {
        self.write_u64(n as u64);
    }

    fn write_u32(&mut self, n: u32) {
        self.write_u64(n as u64);
    }

    fn write_u64(&mut self, n: u64) {
        self.0 = (self.0.rotate_left(5) ^ n).wrapping_mul(0x51_7c_c1_b7_27_22_0a_95);
    }

    fn write_usize(&mut self, n: usize) {
        self.write_u64(n as u64);
    }
}

type HashMap<K, V> = std::collections::HashMap<K, V, BuildHasherDefault<Quick>>;

use crate::{Group, Mesh};

/// A symmetric 4×4 matrix in its ten numbers.
#[derive(Clone, Copy, Default)]
struct Quadric([f64; 10]);

impl Quadric {
    /// The plane `a x + b y + c z + d = 0` with weight `w`.
    fn plane(n: [f64; 3], d: f64, w: f64) -> Quadric {
        let [a, b, c] = n;
        Quadric(
            [
                a * a,
                a * b,
                a * c,
                a * d,
                b * b,
                b * c,
                b * d,
                c * c,
                c * d,
                d * d,
            ]
            .map(|v| v * w),
        )
    }

    fn add(&mut self, o: &Quadric) {
        for (a, b) in self.0.iter_mut().zip(o.0) {
            *a += b;
        }
    }

    /// The squared distance (weighted) of `p` to the planes.
    fn error(&self, p: [f64; 3]) -> f64 {
        let q = &self.0;
        let [x, y, z] = p;
        let e = q[0] * x * x
            + 2.0 * q[1] * x * y
            + 2.0 * q[2] * x * z
            + 2.0 * q[3] * x
            + q[4] * y * y
            + 2.0 * q[5] * y * z
            + 2.0 * q[6] * y
            + q[7] * z * z
            + 2.0 * q[8] * z
            + q[9];
        e.max(0.0)
    }
}

fn sub(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

fn cross(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

fn dot(a: [f64; 3], b: [f64; 3]) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

fn length(a: [f64; 3]) -> f64 {
    dot(a, a).sqrt()
}

/// A triangle being simplified: its corners, the mesh's vertices at them
/// (for the colour) and its group.
struct Tri {
    c: [u32; 3],
    v: [u32; 3],
    group: usize,
    alive: bool,
}

/// A collapse waiting: moving corner `from` onto `to`.
#[derive(PartialEq)]
struct Candidate {
    cost: f64,
    from: u32,
    to: u32,
    /// The stamp of `from` when it was weighed (stale if it changed).
    stamp: u32,
}

impl Eq for Candidate {}

impl Ord for Candidate {
    fn cmp(&self, o: &Self) -> Ordering {
        // The cheapest first out of the (max-)heap.
        o.cost
            .partial_cmp(&self.cost)
            .unwrap_or(Ordering::Equal)
            .then(o.from.cmp(&self.from))
    }
}

impl PartialOrd for Candidate {
    fn partial_cmp(&self, o: &Self) -> Option<Ordering> {
        Some(self.cmp(o))
    }
}

/// A version of `mesh` with about `target` triangles, by collapsing
/// edges; `None` if it cannot get there keeping its shape (what is left
/// are borders and seams).
pub fn decimate(mesh: &Mesh, target: usize) -> Option<Mesh> {
    let total = mesh.triangle_count();
    if total <= target {
        return Some(mesh.clone());
    }
    // Corners welded by position: a vertex of the mesh is a corner and a
    // colour.
    let mut corner_of: Vec<u32> = Vec::with_capacity(mesh.positions.len());
    let mut corners: Vec<[f64; 3]> = Vec::new();
    {
        let mut at: HashMap<[u32; 3], u32> = HashMap::default();
        for p in &mesh.positions {
            let id = *at.entry(p.map(f32::to_bits)).or_insert_with(|| {
                corners.push(p.map(|c| c as f64));
                (corners.len() - 1) as u32
            });
            corner_of.push(id);
        }
    }
    // Triangles: their corners, the mesh's vertices at them (for the
    // colour) and their material.
    let mut tris: Vec<Tri> = Vec::with_capacity(total);
    for (gi, g) in mesh.groups.iter().enumerate() {
        for t in &g.triangles {
            let c = t.map(|v| corner_of[v as usize]);
            if c[0] == c[1] || c[1] == c[2] || c[0] == c[2] {
                continue;
            }
            tris.push(Tri {
                c,
                v: *t,
                group: gi,
                alive: true,
            });
        }
    }
    let n = corners.len();
    let mut around: Vec<Vec<u32>> = vec![Vec::new(); n];
    for (i, t) in tris.iter().enumerate() {
        for c in t.c {
            around[c as usize].push(i as u32);
        }
    }
    // The size of the model, to weigh borders against faces.
    let (lo, hi) = mesh.bounds();
    let size = (0..3)
        .map(|k| (hi[k] - lo[k]) as f64)
        .fold(0.0f64, f64::max)
        .max(1e-6);
    // Planes of the triangles, by their area.
    let mut q: Vec<Quadric> = vec![Quadric::default(); n];
    for t in &tris {
        let [a, b, c] = t.c.map(|i| corners[i as usize]);
        let nrm = cross(sub(b, a), sub(c, a));
        let len = length(nrm);
        if len < 1e-18 {
            continue;
        }
        let u = nrm.map(|x| x / len);
        let plane = Quadric::plane(u, -dot(u, a), len / 2.0);
        for i in t.c {
            q[i as usize].add(&plane);
        }
    }
    // Borders and seams: an edge with one triangle, or between two
    // materials, holds by a plane across it, heavily weighted.
    {
        let mut edges: HashMap<(u32, u32), (u32, usize, bool)> = HashMap::default();
        for t in &tris {
            for k in 0..3 {
                let (a, b) = (t.c[k], t.c[(k + 1) % 3]);
                let key = (a.min(b), a.max(b));
                edges
                    .entry(key)
                    .and_modify(|e| {
                        e.0 += 1;
                        if e.1 != t.group {
                            e.2 = true;
                        }
                    })
                    .or_insert((1, t.group, false));
            }
        }
        let mut faces_of: HashMap<(u32, u32), usize> = HashMap::default();
        for (ti, t) in tris.iter().enumerate() {
            for k in 0..3 {
                let (a, b) = (t.c[k], t.c[(k + 1) % 3]);
                faces_of.entry((a.min(b), a.max(b))).or_insert(ti);
            }
        }
        for ((a, b), (count, _, seam)) in &edges {
            if *count != 1 && !seam {
                continue;
            }
            let t = &tris[faces_of[&(*a, *b)]];
            let [p0, p1, p2] = t.c.map(|i| corners[i as usize]);
            let face = cross(sub(p1, p0), sub(p2, p0));
            let pa = corners[*a as usize];
            let e = sub(corners[*b as usize], pa);
            let across = cross(e, face);
            let len = length(across);
            if len < 1e-18 {
                continue;
            }
            let u = across.map(|x| x / len);
            let w = length(e) * size * 10.0;
            let plane = Quadric::plane(u, -dot(u, pa), w);
            q[*a as usize].add(&plane);
            q[*b as usize].add(&plane);
        }
    }

    let mut stamp = vec![0u32; n];
    let mut heap: BinaryHeap<Candidate> = BinaryHeap::new();
    // The collapses out of corner `from`, cheapest first.
    fn weigh(
        from: u32,
        corners: &[[f64; 3]],
        tris: &[Tri],
        around: &[Vec<u32>],
        q: &[Quadric],
    ) -> Vec<(f64, u32)> {
        let mut out: Vec<(f64, u32)> = Vec::new();
        for &ti in &around[from as usize] {
            let t = &tris[ti as usize];
            if !t.alive {
                continue;
            }
            for &to in &t.c {
                if to == from || out.iter().any(|o| o.1 == to) {
                    continue;
                }
                let mut sum = q[from as usize];
                sum.add(&q[to as usize]);
                out.push((sum.error(corners[to as usize]), to));
            }
        }
        out.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(Ordering::Equal));
        out
    }
    // The cheapest collapse out of corner `from`.
    fn cheapest(
        from: u32,
        corners: &[[f64; 3]],
        tris: &[Tri],
        around: &[Vec<u32>],
        q: &[Quadric],
    ) -> Option<(f64, u32)> {
        let mut best: Option<(f64, u32)> = None;
        for &ti in &around[from as usize] {
            let t = &tris[ti as usize];
            if !t.alive {
                continue;
            }
            for &to in &t.c {
                if to == from || best.is_some_and(|b| b.1 == to) {
                    continue;
                }
                let mut sum = q[from as usize];
                sum.add(&q[to as usize]);
                let cost = sum.error(corners[to as usize]);
                if best.is_none_or(|b| cost < b.0) {
                    best = Some((cost, to));
                }
            }
        }
        best
    }
    // Whether moving `from` onto `to` keeps every moved triangle facing
    // the same way and not flat.
    fn valid(from: u32, to: u32, corners: &[[f64; 3]], tris: &[Tri], around: &[Vec<u32>]) -> bool {
        // How many live triangles share the edge `a`–`b`.
        let shared = |a: u32, b: u32| {
            around[a as usize]
                .iter()
                .filter(|&&ti| tris[ti as usize].alive && tris[ti as usize].c.contains(&b))
                .count()
        };
        let target = corners[to as usize];
        for &ti in &around[from as usize] {
            let t = &tris[ti as usize];
            if !t.alive {
                continue;
            }
            if t.c.contains(&to) {
                // A triangle on its own (a leaf) is not taken away.
                let x =
                    t.c.iter()
                        .copied()
                        .find(|c| *c != from && *c != to)
                        .unwrap_or(to);
                if shared(from, to) == 1 && shared(from, x) == 1 && shared(to, x) == 1 {
                    return false;
                }
                continue;
            }
            let p = t.c.map(|i| corners[i as usize]);
            let before = cross(sub(p[1], p[0]), sub(p[2], p[0]));
            let m = t.c.map(|i| {
                if i == from {
                    target
                } else {
                    corners[i as usize]
                }
            });
            let after = cross(sub(m[1], m[0]), sub(m[2], m[0]));
            let (lb, la) = (length(before), length(after));
            if la < 1e-14 || dot(before, after) < 0.2 * lb * la {
                return false;
            }
        }
        true
    }
    for c in 0..n as u32 {
        if let Some((cost, to)) = cheapest(c, &corners, &tris, &around, &q) {
            heap.push(Candidate {
                cost,
                from: c,
                to,
                stamp: 0,
            });
        }
    }

    let mut alive = tris.len();
    while alive > target {
        let Some(cand) = heap.pop() else { break };
        let from = cand.from;
        if stamp[from as usize] != cand.stamp {
            continue;
        }
        // The cheapest valid one now (the first may have turned invalid).
        let mut chosen = cheapest(from, &corners, &tris, &around, &q)
            .filter(|&(_, to)| valid(from, to, &corners, &tris, &around));
        if chosen.is_none() {
            for (cost, to) in weigh(from, &corners, &tris, &around, &q) {
                if valid(from, to, &corners, &tris, &around) {
                    chosen = Some((cost, to));
                    break;
                }
            }
        }
        let Some((cost, to)) = chosen else {
            // Stuck until something around it changes.
            continue;
        };
        if cost > cand.cost * (1.0 + 1e-9) + 1e-15 {
            heap.push(Candidate {
                cost,
                from,
                to,
                stamp: cand.stamp,
            });
            continue;
        }
        // Collapse.
        let moved: Vec<u32> = std::mem::take(&mut around[from as usize]);
        for &ti in &moved {
            let t = &mut tris[ti as usize];
            if !t.alive {
                continue;
            }
            if t.c.contains(&to) {
                t.alive = false;
                alive -= 1;
                continue;
            }
            for c in t.c.iter_mut() {
                if *c == from {
                    *c = to;
                }
            }
            around[to as usize].push(ti);
        }
        let qf = q[from as usize];
        q[to as usize].add(&qf);
        around[to as usize].retain(|&ti| tris[ti as usize].alive);
        stamp[from as usize] += 1;
        // Weigh again the corners around.
        let mut near: Vec<u32> = vec![to];
        for &ti in &around[to as usize] {
            near.extend(tris[ti as usize].c);
        }
        near.sort_unstable();
        near.dedup();
        for c in near {
            stamp[c as usize] += 1;
            if let Some((cost, t2)) = cheapest(c, &corners, &tris, &around, &q) {
                heap.push(Candidate {
                    cost,
                    from: c,
                    to: t2,
                    stamp: stamp[c as usize],
                });
            }
        }
    }
    if alive > target + target / 2 {
        return None;
    }
    // The mesh left: a vertex per (corner, colour) still used.
    let mut out = Mesh {
        materials: mesh.materials.clone(),
        ..Default::default()
    };
    let mut made: HashMap<(u32, [u8; 3]), u32> = HashMap::default();
    let mut groups: Vec<Vec<[u32; 3]>> = vec![Vec::new(); mesh.groups.len()];
    for t in tris.iter().filter(|t| t.alive) {
        let ids = [0, 1, 2].map(|k| {
            let color = mesh.colors[t.v[k] as usize];
            *made.entry((t.c[k], color)).or_insert_with(|| {
                out.positions
                    .push(corners[t.c[k] as usize].map(|x| x as f32));
                out.colors.push(color);
                (out.positions.len() - 1) as u32
            })
        });
        groups[t.group].push(ids);
    }
    for (gi, tris) in groups.into_iter().enumerate() {
        if !tris.is_empty() {
            out.groups.push(Group {
                material: mesh.groups[gi].material,
                triangles: tris,
            });
        }
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Material;

    /// A sphere of radius 1 (top half one material, bottom another),
    /// `n` rings and `2n` meridians.
    fn sphere(n: u32) -> Mesh {
        let mut m = Mesh::default();
        for name in ["arriba", "abajo"] {
            m.materials.push(Material {
                name: name.into(),
                color: [200, 200, 200],
                alpha: 1.0,
                texture: None,
            });
            m.groups.push(Group {
                material: m.groups.len(),
                triangles: Vec::new(),
            });
        }
        for i in 0..=n {
            let th = std::f32::consts::PI * i as f32 / n as f32;
            for j in 0..2 * n {
                let ph = std::f32::consts::PI * j as f32 / n as f32;
                m.positions
                    .push([th.sin() * ph.cos(), th.sin() * ph.sin(), th.cos()]);
                m.colors.push([200, 200, 200]);
            }
        }
        let at = |i: u32, j: u32| i * 2 * n + j % (2 * n);
        for i in 0..n {
            let g = if i < n / 2 { 0 } else { 1 };
            for j in 0..2 * n {
                let (a, b, c, d) = (at(i, j), at(i + 1, j), at(i + 1, j + 1), at(i, j + 1));
                m.groups[g].triangles.push([a, b, c]);
                m.groups[g].triangles.push([a, c, d]);
            }
        }
        m.clean();
        m
    }

    fn volume(m: &Mesh) -> f32 {
        m.groups
            .iter()
            .flat_map(|g| &g.triangles)
            .map(|t| {
                let [a, b, c] = t.map(|i| m.positions[i as usize]);
                (a[0] * (b[1] * c[2] - b[2] * c[1]) - a[1] * (b[0] * c[2] - b[2] * c[0])
                    + a[2] * (b[0] * c[1] - b[1] * c[0]))
                    / 6.0
            })
            .sum()
    }

    #[test]
    fn a_sphere_keeps_its_size_volume_and_seam() {
        let full = sphere(60);
        let v0 = volume(&full);
        assert!((v0 - 4.18879).abs() < 0.02, "{v0}");
        let light = decimate(&full, 300).unwrap();
        assert!(light.triangle_count() <= 300, "{}", light.triangle_count());
        let v = volume(&light);
        assert!((v - v0).abs() / v0 < 0.06, "volume {v} of {v0}");
        let (lo, hi) = light.bounds();
        for k in 0..3 {
            assert!(hi[k] - lo[k] > 1.9, "{lo:?} {hi:?}");
        }
        // Both materials, and the seam at the equator still level.
        assert_eq!(light.groups.len(), 2);
        let top = &light.groups[0];
        let lowest = top
            .triangles
            .iter()
            .flatten()
            .map(|&i| light.positions[i as usize][2])
            .fold(f32::INFINITY, f32::min);
        assert!(lowest.abs() < 0.02, "{lowest}");
    }

    #[test]
    fn loose_squares_do_not_go_down() {
        // Leaves: squares on their own, all border.
        let mut m = Mesh::default();
        m.materials.push(Material {
            name: "hoja".into(),
            color: [60, 120, 40],
            alpha: 1.0,
            texture: None,
        });
        let mut tris = Vec::new();
        for k in 0..100u32 {
            let x = k as f32;
            for p in [
                [x, 0.0, 0.0],
                [x + 0.5, 0.0, 0.0],
                [x + 0.5, 0.5, 0.0],
                [x, 0.5, 0.0],
            ] {
                m.positions.push(p);
                m.colors.push([60, 120, 40]);
            }
            let b = 4 * k;
            tris.push([b, b + 1, b + 2]);
            tris.push([b, b + 2, b + 3]);
        }
        m.groups.push(Group {
            material: 0,
            triangles: tris,
        });
        assert!(decimate(&m, 20).is_none());
    }
}
