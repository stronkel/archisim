//! Grass and flowers on the areas painted over a terrain (C8): where the
//! material painted has something growing on it ([`Growth`]), tufts of
//! blades and flowers among them, scattered at random but the same each
//! time, standing on the ground. Each blade and flower carries its colour
//! at its corners (darker at the foot, lighter at the tip), so the 3D view,
//! the rendered pictures and the `.glb` draw them alike. They are kept
//! once made, by what they depend on (the ground and the painted areas),
//! so turning the camera does not grow them again.

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::sync::{Arc, Mutex, OnceLock};

use archisim_core::{Growth, Model, Point2, TerrainId};

use crate::solid::{Solid, SolidOwner, P3, PATCH_THICKNESS};

/// The grass of one painted area: its triangles and the colour at each
/// corner.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct GrowthMesh {
    pub triangles: Vec<[P3; 3]>,
    pub colors: Vec<[[u8; 3]; 3]>,
}

/// At most this many tufts of grass on one terrain (and as many flowers):
/// beyond it, they are spread wider.
pub const MAX_TUFTS: usize = 16_000;

/// The grass and flowers of every painted area of a terrain that grows
/// something: one solid each, owned by [`SolidOwner::Growth`] and of the
/// painted material.
pub fn growth_solids(model: &Model, id: TerrainId) -> Vec<Solid> {
    let Some(t) = model.terrain(id) else {
        return Vec::new();
    };
    t.patches
        .iter()
        .enumerate()
        .filter(|(_, p)| model.appearance(p.material).growth.is_some())
        .filter_map(|(i, p)| {
            let mesh = growth_mesh(model, id, i)?;
            (!mesh.triangles.is_empty()).then(|| Solid {
                owner: SolidOwner::Growth(id),
                material: p.material,
                triangles: mesh.triangles.clone(),
            })
        })
        .collect()
}

/// The colours of the corners of a solid of [`growth_solids`].
pub fn growth_colors(model: &Model, s: &Solid) -> Option<Vec<[[u8; 3]; 3]>> {
    let SolidOwner::Growth(id) = s.owner else {
        return None;
    };
    let t = model.terrain(id)?;
    let i = t.patches.iter().position(|p| p.material == s.material)?;
    let mesh = growth_mesh(model, id, i)?;
    (mesh.colors.len() == s.triangles.len()).then(|| mesh.colors.clone())
}

/// The grass made, by what it depends on.
type Kept = Mutex<Vec<(u64, Arc<GrowthMesh>)>>;

/// The grass of painted area `index` of a terrain, made once and kept.
pub fn growth_mesh(model: &Model, id: TerrainId, index: usize) -> Option<Arc<GrowthMesh>> {
    let t = model.terrain(id)?;
    let patch = t.patches.get(index)?;
    let growth = model.appearance(patch.material).growth?;
    let colour = model.surface_color(patch.material);
    // What it depends on.
    let mut h = DefaultHasher::new();
    id.0.hash(&mut h);
    for p in &t.points {
        p.at.x.to_bits().hash(&mut h);
        p.at.y.to_bits().hash(&mut h);
        p.z.to_bits().hash(&mut h);
    }
    // The painted area less the room the pools take out of the ground.
    let boundary: Vec<archisim_core::Point2> =
        t.boundary().iter().map(|i| t.points[*i].at).collect();
    let dug = crate::solid::terrain_holes(model, &boundary);
    let regions = if dug.is_empty() {
        patch.regions.clone()
    } else {
        let cut: Vec<crate::junctions::Shape> = dug
            .iter()
            .map(|e| (e.outline.clone(), Vec::new()))
            .collect();
        crate::junctions::difference(&patch.regions, &cut)
    };
    for (o, holes) in &regions {
        for q in o.iter().chain(holes.iter().flatten()) {
            q.x.to_bits().hash(&mut h);
            q.y.to_bits().hash(&mut h);
        }
        holes.len().hash(&mut h);
    }
    // The whole terrain's areas share its tufts.
    let total: f64 = t
        .patches
        .iter()
        .filter_map(|p| {
            model
                .appearance(p.material)
                .growth
                .map(|g| p.area() * g.density.max(g.bloom))
        })
        .sum();
    total.to_bits().hash(&mut h);
    [growth.height, growth.density, growth.bloom]
        .iter()
        .for_each(|v| v.to_bits().hash(&mut h));
    growth.flowers.hash(&mut h);
    growth.colours.hash(&mut h);
    colour.hash(&mut h);
    let key = h.finish();
    static CACHE: OnceLock<Kept> = OnceLock::new();
    let cache = CACHE.get_or_init(|| Mutex::new(Vec::new()));
    if let Some(m) = cache
        .lock()
        .ok()
        .and_then(|c| c.iter().find(|(k, _)| *k == key).map(|(_, m)| m.clone()))
    {
        return Some(m);
    }
    let ground = Ground::new(t);
    // Spread wider when the terrain would have too many.
    let thin = if total > MAX_TUFTS as f64 {
        (total / MAX_TUFTS as f64).sqrt()
    } else {
        1.0
    };
    let mesh = Arc::new(grow(
        &ground,
        &regions,
        &growth,
        colour,
        thin,
        key ^ index as u64,
    ));
    if let Ok(mut c) = cache.lock() {
        if c.len() > 24 {
            c.remove(0);
        }
        c.push((key, mesh.clone()));
    }
    Some(mesh)
}

/// The ground under the grass: its triangles in a grid of cells, to find
/// the height under a point quickly.
struct Ground {
    tris: Vec<[P3; 3]>,
    lo: [f64; 2],
    cell: f64,
    n: [usize; 2],
    cells: Vec<Vec<u32>>,
}

impl Ground {
    fn new(t: &archisim_core::Terrain) -> Ground {
        let tris: Vec<[P3; 3]> = t
            .triangles()
            .into_iter()
            .map(|tri| tri.map(|i| [t.points[i].at.x, t.points[i].at.y, t.points[i].z]))
            .collect();
        let (mut lo, mut hi) = ([f64::INFINITY; 2], [f64::NEG_INFINITY; 2]);
        for tri in &tris {
            for p in tri {
                for k in 0..2 {
                    lo[k] = lo[k].min(p[k]);
                    hi[k] = hi[k].max(p[k]);
                }
            }
        }
        let span = (hi[0] - lo[0]).max(hi[1] - lo[1]).max(1.0);
        let cell = (span / 64.0).max(0.5);
        let n = [
            (((hi[0] - lo[0]) / cell).ceil() as usize).max(1),
            (((hi[1] - lo[1]) / cell).ceil() as usize).max(1),
        ];
        let mut cells = vec![Vec::new(); n[0] * n[1]];
        for (i, tri) in tris.iter().enumerate() {
            let (mut a, mut b) = ([usize::MAX; 2], [0usize; 2]);
            for p in tri {
                for k in 0..2 {
                    let c = (((p[k] - lo[k]) / cell).floor().max(0.0) as usize).min(n[k] - 1);
                    a[k] = a[k].min(c);
                    b[k] = b[k].max(c);
                }
            }
            for y in a[1]..=b[1] {
                for x in a[0]..=b[0] {
                    cells[y * n[0] + x].push(i as u32);
                }
            }
        }
        Ground {
            tris,
            lo,
            cell,
            n,
            cells,
        }
    }

    fn height(&self, p: Point2) -> Option<f64> {
        let c = |v: f64, k: usize| ((v - self.lo[k]) / self.cell).floor();
        let (x, y) = (c(p.x, 0), c(p.y, 1));
        if x < 0.0 || y < 0.0 || x as usize >= self.n[0] || y as usize >= self.n[1] {
            return None;
        }
        for &i in &self.cells[y as usize * self.n[0] + x as usize] {
            let [a, b, c] = self.tris[i as usize];
            let d = (b[1] - c[1]) * (a[0] - c[0]) + (c[0] - b[0]) * (a[1] - c[1]);
            if d.abs() < 1e-12 {
                continue;
            }
            let u = ((b[1] - c[1]) * (p.x - c[0]) + (c[0] - b[0]) * (p.y - c[1])) / d;
            let v = ((c[1] - a[1]) * (p.x - c[0]) + (a[0] - c[0]) * (p.y - c[1])) / d;
            let w = 1.0 - u - v;
            if u >= -1e-9 && v >= -1e-9 && w >= -1e-9 {
                return Some(u * a[2] + v * b[2] + w * c[2]);
            }
        }
        None
    }
}

/// A small, repeatable random number generator.
struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Rng {
        Rng(seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1)
    }
    fn next(&mut self) -> f64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        (self.0 >> 11) as f64 / (1u64 << 53) as f64
    }
}

fn inside(regions: &[(Vec<Point2>, Vec<Vec<Point2>>)], p: Point2) -> bool {
    regions.iter().any(|(o, holes)| {
        archisim_core::point_in_polygon(p, o)
            && !holes.iter().any(|h| archisim_core::point_in_polygon(p, h))
    })
}

fn shade(c: [u8; 3], k: f64, jitter: [f64; 3]) -> [u8; 3] {
    [0, 1, 2].map(|i| (c[i] as f64 * k * (1.0 + jitter[i])).clamp(0.0, 255.0) as u8)
}

/// Points scattered over `regions`, about one per `spacing` square, each
/// moved at random within its square.
fn scatter(
    regions: &[(Vec<Point2>, Vec<Vec<Point2>>)],
    spacing: f64,
    rng: &mut Rng,
) -> Vec<Point2> {
    let (mut lo, mut hi) = (
        Point2::new(f64::INFINITY, f64::INFINITY),
        Point2::new(f64::NEG_INFINITY, f64::NEG_INFINITY),
    );
    for (o, _) in regions {
        for q in o {
            lo = Point2::new(lo.x.min(q.x), lo.y.min(q.y));
            hi = Point2::new(hi.x.max(q.x), hi.y.max(q.y));
        }
    }
    if !lo.x.is_finite() || spacing <= 0.0 {
        return Vec::new();
    }
    let (nx, ny) = (
        ((hi.x - lo.x) / spacing).ceil() as usize,
        ((hi.y - lo.y) / spacing).ceil() as usize,
    );
    let mut out = Vec::new();
    for j in 0..ny {
        for i in 0..nx {
            let p = Point2::new(
                lo.x + (i as f64 + rng.next()) * spacing,
                lo.y + (j as f64 + rng.next()) * spacing,
            );
            if inside(regions, p) {
                out.push(p);
            }
        }
    }
    out
}

/// Grows the grass and flowers of one area.
fn grow(
    ground: &Ground,
    regions: &[(Vec<Point2>, Vec<Vec<Point2>>)],
    g: &Growth,
    colour: [u8; 3],
    thin: f64,
    seed: u64,
) -> GrowthMesh {
    let mut mesh = GrowthMesh::default();
    let mut rng = Rng::new(seed);
    let lift = PATCH_THICKNESS * 0.5;
    // Both faces of a triangle, so it is seen from either side.
    let both = |m: &mut GrowthMesh, t: [P3; 3], c: [[u8; 3]; 3]| {
        m.triangles.push(t);
        m.colors.push(c);
        m.triangles.push([t[0], t[2], t[1]]);
        m.colors.push([c[0], c[2], c[1]]);
    };
    if g.height > 1e-3 && g.density > 1e-3 {
        let spacing = thin / g.density.sqrt();
        for p in scatter(regions, spacing, &mut rng) {
            let Some(z) = ground.height(p) else {
                continue;
            };
            let z = z + lift;
            for _ in 0..3 {
                let a = rng.next() * std::f64::consts::TAU;
                let (dx, dy) = (a.cos(), a.sin());
                let off = 0.03 * rng.next();
                let foot = Point2::new(p.x + dx * off, p.y + dy * off);
                let h = g.height * (0.55 + 0.55 * rng.next());
                let w = 0.01 + g.height * 0.035;
                let lean = h * (0.15 + 0.3 * rng.next());
                let b = rng.next() * std::f64::consts::TAU;
                let (sx, sy) = (b.cos() * w * 0.5, b.sin() * w * 0.5);
                let tip = [foot.x + dx * lean, foot.y + dy * lean, z + h];
                let j = [
                    (rng.next() - 0.5) * 0.25,
                    (rng.next() - 0.5) * 0.15,
                    (rng.next() - 0.5) * 0.25,
                ];
                let dark = shade(colour, 0.55, j);
                let light = shade(colour, 1.15, j);
                both(
                    &mut mesh,
                    [
                        [foot.x - sx, foot.y - sy, z],
                        [foot.x + sx, foot.y + sy, z],
                        tip,
                    ],
                    [dark, dark, light],
                );
            }
        }
    }
    let colours = g.flower_colours();
    if g.bloom > 1e-3 && !colours.is_empty() {
        let spacing = thin / g.bloom.sqrt();
        let stem_h = g.height.max(0.12);
        for p in scatter(regions, spacing, &mut rng) {
            let Some(z) = ground.height(p) else {
                continue;
            };
            let z = z + lift;
            let h = stem_h * (0.7 + 0.45 * rng.next());
            let petal = colours[(rng.next() * colours.len() as f64) as usize % colours.len()];
            let j = [
                (rng.next() - 0.5) * 0.12,
                (rng.next() - 0.5) * 0.12,
                (rng.next() - 0.5) * 0.12,
            ];
            let petal = shade(petal, 1.0, j);
            let stem = shade(colour, 0.7, [0.0; 3]);
            let a = rng.next() * std::f64::consts::TAU;
            let w = 0.006;
            let (sx, sy) = (a.cos() * w, a.sin() * w);
            let top = [p.x, p.y, z + h];
            both(
                &mut mesh,
                [[p.x - sx, p.y - sy, z], [p.x + sx, p.y + sy, z], top],
                [stem, stem, stem],
            );
            // The head: a low cone of petals round a lighter middle.
            let r = 0.022 + 0.022 * rng.next();
            let heart = [top[0], top[1], top[2] + r * 0.35];
            let centre = [
                petal[0].saturating_add(40).max(200),
                petal[1].saturating_add(30).max(170),
                petal[2] / 2,
            ];
            let ring: Vec<P3> = (0..5)
                .map(|k| {
                    let b = a + k as f64 / 5.0 * std::f64::consts::TAU;
                    [top[0] + r * b.cos(), top[1] + r * b.sin(), top[2]]
                })
                .collect();
            for k in 0..5 {
                let (u, v) = (ring[k], ring[(k + 1) % 5]);
                mesh.triangles.push([u, v, heart]);
                mesh.colors.push([petal, petal, centre]);
                mesh.triangles.push([v, u, top]);
                mesh.colors.push([petal, petal, petal]);
            }
        }
    }
    mesh
}

// ---- Green walls (H5) -------------------------------------------------------

/// Leaves per m² of a green wall at most, and its leaf's length.
const GREEN_LEAVES_MAX: f64 = 700.0;
const GREEN_LEAF: f64 = 0.07;

/// The leaves of a green wall: on the faces of a wall (`solids`, its
/// layers) whose material — painted or of the layer — is a plant's that
/// grows (a «Fachada vegetal»), small leaves standing out of
/// the face in their thousands, the same each time, in that material.
/// They belong to the wall's façade (`SolidOwner::Facade`), so sections
/// and elevations leave them out as they do the bricks.
pub fn green_wall_solids(
    model: &Model,
    wall: archisim_core::WallId,
    solids: &[Solid],
) -> Vec<Solid> {
    let mut by_material: std::collections::BTreeMap<archisim_core::MaterialId, Vec<[P3; 3]>> =
        Default::default();
    for s in solids {
        for tri in &s.triangles {
            let u = [
                tri[1][0] - tri[0][0],
                tri[1][1] - tri[0][1],
                tri[1][2] - tri[0][2],
            ];
            let v = [
                tri[2][0] - tri[0][0],
                tri[2][1] - tri[0][1],
                tri[2][2] - tri[0][2],
            ];
            let n = [
                u[1] * v[2] - u[2] * v[1],
                u[2] * v[0] - u[0] * v[2],
                u[0] * v[1] - u[1] * v[0],
            ];
            let len = (n[0] * n[0] + n[1] * n[1] + n[2] * n[2]).sqrt();
            // Faces that stand up, big enough to grow on.
            if len < 1e-9 || (n[2] / len).abs() > 0.2 || len / 2.0 < 0.01 {
                continue;
            }
            let m = crate::solid::painted_material(model, s, tri).unwrap_or(s.material);
            let a = model.appearance(m);
            let Some(g) = a.growth else {
                continue;
            };
            if model.material(m).is_none_or(|x| x.category != "vegetation") {
                continue;
            }
            let normal = [n[0] / len, n[1] / len, n[2] / len];
            let area = len / 2.0;
            let density = g.density.clamp(10.0, GREEN_LEAVES_MAX);
            let depth = g.height.clamp(0.02, 0.4);
            // The same each time: seeded by where the triangle is.
            let mut h = DefaultHasher::new();
            for p in tri {
                for c in p {
                    ((c * 1000.0).round() as i64).hash(&mut h);
                }
            }
            let mut seed = h.finish() | 1;
            let mut rnd = move || {
                seed ^= seed << 13;
                seed ^= seed >> 7;
                seed ^= seed << 17;
                (seed >> 11) as f64 / (1u64 << 53) as f64
            };
            let count = (area * density) as usize + usize::from(rnd() < (area * density).fract());
            let out = by_material.entry(m).or_default();
            for _ in 0..count {
                // A point of the triangle.
                let (mut a1, mut b1) = (rnd(), rnd());
                if a1 + b1 > 1.0 {
                    a1 = 1.0 - a1;
                    b1 = 1.0 - b1;
                }
                let off = depth * (0.15 + 0.85 * rnd());
                let c = [
                    tri[0][0] + u[0] * a1 + v[0] * b1 + normal[0] * off,
                    tri[0][1] + u[1] * a1 + v[1] * b1 + normal[1] * off,
                    tri[0][2] + u[2] * a1 + v[2] * b1 + normal[2] * off,
                ];
                // A leaf: a rhombus turned at random in the face's plane,
                // tilted out of it.
                let ang = rnd() * std::f64::consts::TAU;
                let side = [normal[1], -normal[0], 0.0];
                let up = [0.0, 0.0, 1.0];
                let d = [
                    side[0] * ang.cos() + up[0] * ang.sin(),
                    side[1] * ang.cos() + up[1] * ang.sin(),
                    ang.sin(),
                ];
                let tilt = 0.3 + 0.5 * rnd();
                let along = [
                    d[0] * (1.0 - tilt) + normal[0] * tilt,
                    d[1] * (1.0 - tilt) + normal[1] * tilt,
                    d[2] * (1.0 - tilt) + normal[2] * tilt,
                ];
                let w = [
                    along[1] * normal[2] - along[2] * normal[1],
                    along[2] * normal[0] - along[0] * normal[2],
                    along[0] * normal[1] - along[1] * normal[0],
                ];
                let l = GREEN_LEAF * (0.7 + 0.6 * rnd());
                let half = l * 0.35;
                let tip = [
                    c[0] + along[0] * l,
                    c[1] + along[1] * l,
                    c[2] + along[2] * l,
                ];
                let mid = [
                    c[0] + along[0] * l * 0.4,
                    c[1] + along[1] * l * 0.4,
                    c[2] + along[2] * l * 0.4,
                ];
                let left = [
                    mid[0] + w[0] * half,
                    mid[1] + w[1] * half,
                    mid[2] + w[2] * half,
                ];
                let right = [
                    mid[0] - w[0] * half,
                    mid[1] - w[1] * half,
                    mid[2] - w[2] * half,
                ];
                out.push([c, left, tip]);
                out.push([c, tip, right]);
            }
        }
    }
    by_material
        .into_iter()
        .map(|(material, triangles)| Solid {
            owner: SolidOwner::Facade(wall),
            material,
            triangles,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use archisim_core::{NewTerrain, SurveyPoint, TerrainPatch};

    fn site(growth: Option<Growth>) -> (Model, TerrainId) {
        let mut m = Model::new("t");
        let l = m.add_level("PB", 0.0);
        let earth = m.add_material(
            "Tierra",
            "earth",
            [150, 120, 90],
            archisim_core::CutPattern::Solid,
        );
        let lawn = m.add_material(
            "Pradera",
            "grass",
            [110, 160, 70],
            archisim_core::CutPattern::Solid,
        );
        m.set_appearance(
            lawn,
            archisim_core::Appearance {
                growth,
                ..Default::default()
            },
        )
        .unwrap();
        let sp = |x: f64, y: f64, z: f64| SurveyPoint {
            at: Point2::new(x, y),
            z,
        };
        let t = m
            .add_terrain(NewTerrain {
                level: l,
                points: vec![
                    sp(0.0, 0.0, 0.0),
                    sp(10.0, 0.0, 0.0),
                    sp(10.0, 10.0, 1.0),
                    sp(0.0, 10.0, 1.0),
                ],
                material: earth,
                depth: 1.0,
                contour: 0.5,
                lines: Vec::new(),
            })
            .unwrap();
        let square = vec![
            Point2::new(2.0, 2.0),
            Point2::new(6.0, 2.0),
            Point2::new(6.0, 6.0),
            Point2::new(2.0, 6.0),
        ];
        m.edit_terrain(t, |t| {
            t.patches = vec![TerrainPatch {
                material: lawn,
                regions: vec![(square, Vec::new())],
            }]
        })
        .unwrap();
        (m, t)
    }

    #[test]
    fn grass_and_flowers_where_it_is_painted() {
        let g = Growth::flowering(0.3, 12.0, &[[240, 200, 40]], 9.0);
        let (m, t) = site(Some(g));
        let solids = growth_solids(&m, t);
        assert_eq!(solids.len(), 1);
        let s = &solids[0];
        // About 16 m² × 12 tufts × 3 blades × 2 faces, and the flowers.
        let tufts = 16.0 * 12.0 * 6.0;
        assert!(
            s.triangles.len() as f64 > tufts * 0.7,
            "{}",
            s.triangles.len()
        );
        // All on the painted square, on the slope of the ground, no higher
        // than the grass and the flowers.
        for tri in &s.triangles {
            for p in tri {
                assert!(
                    p[0] > 1.8 && p[0] < 6.2 && p[1] > 1.8 && p[1] < 6.2,
                    "{p:?}"
                );
                let ground = p[1] / 10.0;
                assert!(p[2] >= ground - 0.05 && p[2] <= ground + 0.6, "{p:?}");
            }
        }
        let colours = growth_colors(&m, s).unwrap();
        assert_eq!(colours.len(), s.triangles.len());
        assert!(colours
            .iter()
            .flatten()
            .any(|c| c[0] > 200 && c[1] > 160 && c[2] < 90));
        // The same each time, and kept.
        let again = growth_solids(&m, t);
        assert_eq!(again[0].triangles, s.triangles);
        // Nothing grows where nothing is said to.
        let (m, t) = site(None);
        assert!(growth_solids(&m, t).is_empty());
    }

    #[test]
    fn a_large_meadow_is_thinned() {
        let g = Growth::grass(0.3, 400.0);
        let (m, t) = site(Some(g));
        let s = &growth_solids(&m, t)[0];
        // 16 m² × 400 would be 6 400 tufts; still under the cap.
        assert!(s.triangles.len() <= MAX_TUFTS * 6 + 600);
    }

    /// A wall painted with a green finish grows leaves out of that face,
    /// and only there; unpainted, none.
    #[test]
    fn green_walls_grow_leaves() {
        let mut m = Model::new("t");
        let l = m.add_level("PB", 0.0);
        let mat = m.add_material(
            "Revoco",
            "plaster",
            [230, 228, 220],
            archisim_core::CutPattern::Solid,
        );
        let wt = m
            .add_wall_type(
                "Muro",
                archisim_core::WallFunction::Exterior,
                vec![archisim_core::Layer {
                    material: mat,
                    thickness: 0.2,
                    function: archisim_core::LayerFunction::Structure,
                }],
            )
            .unwrap();
        let w = m
            .add_wall(archisim_core::NewWall {
                wall_type: wt,
                level: l,
                start: Point2::new(0.0, 0.0),
                end: Point2::new(4.0, 0.0),
                bulge: 0.0,
                reference: archisim_core::ReferenceLine::Center,
                height: 3.0,
                base_offset: 0.0,
                top: archisim_core::WallTop::Unconnected,
            })
            .unwrap();
        let solids = crate::solid::wall_solids(&m, w);
        assert!(green_wall_solids(&m, w, &solids).is_empty());
        let green = m.add_material(
            "Hiedra",
            "vegetation",
            [52, 88, 44],
            archisim_core::CutPattern::Solid,
        );
        m.set_appearance(
            green,
            archisim_core::Appearance {
                growth: Some(Growth::grass(0.12, 400.0)),
                ..Default::default()
            },
        )
        .unwrap();
        m.set_face_paint(
            w.0,
            archisim_core::FaceRef::Wall(archisim_core::FaceSide::Exterior),
            Some(green),
        )
        .unwrap();
        let solids = crate::solid::wall_solids(&m, w);
        let leaves = green_wall_solids(&m, w, &solids);
        assert_eq!(leaves.len(), 1);
        let n = leaves[0].triangles.len();
        // 12 m² at 400 a m², two triangles each.
        assert!((8000..11000).contains(&n), "{n}");
        // All on the exterior side (the left of the wall, +y), out of it.
        assert!(leaves[0].triangles.iter().flatten().all(|p| p[1] > 0.09));
        // The same each time.
        assert_eq!(leaves, green_wall_solids(&m, w, &solids));
    }
}
