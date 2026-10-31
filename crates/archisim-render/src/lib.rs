//! Rendering the model to a picture (bloque 7 de la revisión del
//! 22/10/2026): a ray tracer of its own, on the processor, for still
//! images of presentation — the sun of the project with its shadows, the
//! light of the sky with the shade of corners and eaves (ambient
//! occlusion), the colours of the materials and glass that lets the light
//! through. No dependencies besides the model and its solids: the same
//! triangles as the 3D view and the IFC.
//!
//! The scene is the model's solids (with the faces painted), and a ground
//! under it when the project has no terrain. A bounding volume hierarchy
//! finds what each ray meets; rows are shared among the processor's
//! threads.

pub mod glb;
pub mod textures;

use archisim_core::Model;
pub(crate) use archisim_geom::solid::texture_of;
use archisim_geom::solid::{model_solids, painted_color, solid_color, solid_vertex_colors};
use std::sync::atomic::{AtomicUsize, Ordering};

type V3 = [f64; 3];

fn add(a: V3, b: V3) -> V3 {
    [a[0] + b[0], a[1] + b[1], a[2] + b[2]]
}
fn sub(a: V3, b: V3) -> V3 {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}
fn mul(a: V3, k: f64) -> V3 {
    [a[0] * k, a[1] * k, a[2] * k]
}
fn dot(a: V3, b: V3) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}
fn cross(a: V3, b: V3) -> V3 {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}
fn norm(a: V3) -> V3 {
    let l = dot(a, a).sqrt();
    if l < 1e-15 {
        [0.0, 0.0, 1.0]
    } else {
        mul(a, 1.0 / l)
    }
}
fn had(a: V3, b: V3) -> V3 {
    [a[0] * b[0], a[1] * b[1], a[2] * b[2]]
}

/// Where the picture is taken from: as the 3D view's camera.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct View {
    pub eye: V3,
    pub target: V3,
    /// Vertical field of view, radians (perspective).
    pub fov_y: f64,
    pub perspective: bool,
    /// Half the height of the picture at the target, metres (parallel).
    pub half_height: f64,
}

impl View {
    /// Seen from the south-west, from above, the whole model in the
    /// picture.
    pub fn fit(model: &Model) -> View {
        let (lo, hi) = scene_box(model);
        let c = mul(add(lo, hi), 0.5);
        let r = (dot(sub(hi, lo), sub(hi, lo)).sqrt() / 2.0).max(1.0);
        let (yaw, pitch) = ((-125f64).to_radians(), 28f64.to_radians());
        let back = [
            pitch.cos() * yaw.cos(),
            pitch.cos() * yaw.sin(),
            pitch.sin(),
        ];
        let fov: f64 = 0.7;
        let d = r / (fov / 2.0).sin() * 0.85;
        View {
            eye: add(c, mul(back, d)),
            target: c,
            fov_y: fov,
            perspective: true,
            half_height: r,
        }
    }
}

/// The light of the picture, as the 3D view's: day, evening (a low,
/// warm sun and the lamps beginning to glow) or night (the lamps alone,
/// under a dark sky).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Light {
    #[default]
    Day,
    Evening,
    Night,
}

impl Light {
    pub const ALL: [Light; 3] = [Light::Day, Light::Evening, Light::Night];

    pub fn name(self) -> &'static str {
        match self {
            Light::Day => "Día",
            Light::Evening => "Atardecer",
            Light::Night => "Noche",
        }
    }

    /// The sun's light (none at night).
    fn sun(self) -> Option<V3> {
        match self {
            Light::Day => Some(SUN),
            Light::Evening => Some([2.5, 1.55, 0.85]),
            Light::Night => None,
        }
    }

    /// The sky's light on a face open to all of it.
    fn sky_light(self) -> V3 {
        match self {
            Light::Day => SKY_LIGHT,
            Light::Evening => [0.42, 0.40, 0.50],
            Light::Night => [0.045, 0.06, 0.11],
        }
    }

    /// How much the lamps are lit.
    fn lamps(self) -> f64 {
        match self {
            Light::Day => 0.0,
            Light::Evening => 0.55,
            Light::Night => 1.0,
        }
    }

    /// The sky's colour at the horizon and at the zenith.
    fn sky_colours(self) -> (V3, V3) {
        match self {
            Light::Day => ([0.78, 0.84, 0.92], [0.36, 0.52, 0.80]),
            Light::Evening => ([0.98, 0.72, 0.50], [0.32, 0.38, 0.62]),
            Light::Night => ([0.07, 0.09, 0.16], [0.015, 0.025, 0.06]),
        }
    }
}

/// How the picture is made.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Options {
    pub width: usize,
    pub height: usize,
    /// Rays per pixel, along each side (2: four rays, smooth edges).
    pub samples: usize,
    /// Rays to the sky from each point, for the shade of corners.
    pub sky_rays: usize,
    pub light: Light,
    /// The materials' textures (as the 3D view's).
    pub textures: bool,
}

impl Default for Options {
    fn default() -> Self {
        Options {
            width: 1600,
            height: 1000,
            samples: 2,
            sky_rays: 12,
            light: Light::Day,
            textures: true,
        }
    }
}

/// A picture: rows from the top, three bytes (sRGB) a pixel.
#[derive(Debug, Clone, PartialEq)]
pub struct Image {
    pub width: usize,
    pub height: usize,
    pub rgb: Vec<u8>,
}

#[derive(Debug, Clone, Copy)]
struct Tri {
    a: V3,
    e1: V3,
    e2: V3,
    n: V3,
    /// Linear colour.
    albedo: V3,
    /// Its colour as given (sRGB, 0–1), for its texture.
    srgb: V3,
    glass: bool,
    /// Its texture (`TextureKind::number`, 0 none) and the size of its
    /// piece, metres.
    tex: u8,
    scale: f32,
    /// The object it belongs to (0: not an object), so that a lamp's
    /// light is not stopped by its own shade.
    owner: u64,
    /// It glows at night (a lampshade, a bulb, a flame).
    glow: bool,
}

/// Where a lamp's light comes from, how far it reaches, and its object.
#[derive(Debug, Clone, Copy)]
struct Lamp {
    at: V3,
    reach: f64,
    owner: u64,
}

#[derive(Debug, Clone, Copy)]
struct Node {
    lo: V3,
    hi: V3,
    /// Leaf: first triangle and count; inner: the right child (the left
    /// one is next), count 0.
    start: u32,
    count: u32,
}

/// What the rays meet.
struct Scene {
    tris: Vec<Tri>,
    nodes: Vec<Node>,
    sun: Option<V3>,
    light: Light,
    lamps: Vec<Lamp>,
    /// The pictures of the textures used, by `TextureKind::number`.
    tiles: Vec<Option<std::sync::Arc<textures::Tile>>>,
}

fn srgb_to_linear(c: u8) -> f64 {
    let c = c as f64 / 255.0;
    if c <= 0.04045 {
        c / 12.92
    } else {
        ((c + 0.055) / 1.055).powf(2.4)
    }
}

fn linear_to_srgb(v: f64) -> u8 {
    let v = v.clamp(0.0, 1.0);
    let s = if v <= 0.003_130_8 {
        v * 12.92
    } else {
        1.055 * v.powf(1.0 / 2.4) - 0.055
    };
    (s * 255.0).round() as u8
}

fn scene_box(model: &Model) -> (V3, V3) {
    let mut lo = [f64::MAX; 3];
    let mut hi = [f64::MIN; 3];
    for s in model_solids(model) {
        for t in &s.triangles {
            for p in t {
                for k in 0..3 {
                    lo[k] = lo[k].min(p[k]);
                    hi[k] = hi[k].max(p[k]);
                }
            }
        }
    }
    if lo[0] > hi[0] {
        return ([0.0; 3], [10.0, 10.0, 3.0]);
    }
    (lo, hi)
}

impl Scene {
    fn build(model: &Model, opt: &Options) -> Scene {
        let mut tris = Vec::new();
        let (mut lo, mut hi) = ([f64::MAX; 3], [f64::MIN; 3]);
        let mut tiles: Vec<Option<std::sync::Arc<textures::Tile>>> = vec![None; 32];
        let mut lamps: std::collections::BTreeMap<u64, (V3, f64, f64)> = Default::default();
        for s in model_solids(model) {
            let glass = model
                .material(s.material)
                .is_some_and(|m| m.category == "glass");
            let base = solid_color(model, &s);
            let per_vertex = solid_vertex_colors(model, &s);
            let (owner, is_object) = match s.owner {
                archisim_geom::solid::SolidOwner::Object(o) => (o.0 | 1 << 63, true),
                _ => (0, false),
            };
            let glow = archisim_geom::solid::solid_glow(model, &s);
            for (i, t) in s.triangles.iter().enumerate() {
                let painted = archisim_geom::solid::painted_material(model, &s, t);
                let rgb = per_vertex
                    .as_ref()
                    .and_then(|v| v.get(i))
                    .map(|c| {
                        let avg = |k: usize| {
                            ((c[0][k] as u32 + c[1][k] as u32 + c[2][k] as u32) / 3) as u8
                        };
                        [avg(0), avg(1), avg(2)]
                    })
                    .or_else(|| painted_color(model, &s, t))
                    .unwrap_or(base);
                let e1 = sub(t[1], t[0]);
                let e2 = sub(t[2], t[0]);
                let n = cross(e1, e2);
                if dot(n, n) < 1e-18 {
                    continue;
                }
                for p in t {
                    for k in 0..3 {
                        lo[k] = lo[k].min(p[k]);
                        hi[k] = hi[k].max(p[k]);
                    }
                }
                // Its texture, unless its colours come from its vertices.
                let piece = matches!(s.owner, archisim_geom::solid::SolidOwner::Facade(_))
                    && painted.is_none();
                let texture = (opt.textures && !glass && per_vertex.is_none() && !piece)
                    .then(|| texture_of(model, painted.unwrap_or(s.material), is_object))
                    .flatten();
                let (tex, scale) = match texture {
                    Some((kind, scale)) => {
                        let k = kind.number() as usize;
                        if k < tiles.len() && tiles[k].is_none() {
                            tiles[k] = Some(textures::tile(kind));
                        }
                        (k as u8, scale as f32)
                    }
                    None => (0, 0.0),
                };
                if let Some(reach) = glow {
                    let e = lamps.entry(owner).or_insert(([0.0; 3], 0.0, reach));
                    for p in t {
                        e.0 = add(e.0, *p);
                        e.1 += 1.0;
                    }
                }
                tris.push(Tri {
                    a: t[0],
                    e1,
                    e2,
                    n: norm(n),
                    albedo: rgb.map(srgb_to_linear),
                    srgb: rgb.map(|c| c as f64 / 255.0),
                    glass,
                    tex,
                    scale,
                    owner,
                    glow: glow.is_some(),
                });
            }
        }
        // A ground where there is no terrain: under the lowest point, far
        // round the model.
        if model.terrains().next().is_none() && lo[0] <= hi[0] {
            let z = lo[2].min(0.0) - 0.001;
            let r = (hi[0] - lo[0]).max(hi[1] - lo[1]) * 6.0 + 50.0;
            let c = [(lo[0] + hi[0]) / 2.0, (lo[1] + hi[1]) / 2.0];
            let p = |x: f64, y: f64| [c[0] + x * r, c[1] + y * r, z];
            let albedo = [0.42, 0.46, 0.36];
            for (a, b, d) in [
                (p(-1.0, -1.0), p(1.0, -1.0), p(1.0, 1.0)),
                (p(-1.0, -1.0), p(1.0, 1.0), p(-1.0, 1.0)),
            ] {
                tris.push(Tri {
                    a,
                    e1: sub(b, a),
                    e2: sub(d, a),
                    n: [0.0, 0.0, 1.0],
                    albedo,
                    srgb: [0.68, 0.71, 0.63],
                    glass: false,
                    tex: 0,
                    scale: 0.0,
                    owner: 0,
                    glow: false,
                });
            }
        }
        let lamps = if opt.light.lamps() > 0.0 {
            lamps
                .into_iter()
                .filter(|(_, (_, n, _))| *n > 0.0)
                .map(|(owner, (sum, n, reach))| Lamp {
                    at: mul(sum, 1.0 / n),
                    reach,
                    owner,
                })
                .collect()
        } else {
            Vec::new()
        };
        let mut scene = Scene {
            tris,
            nodes: Vec::new(),
            sun: model.sun_direction().filter(|_| opt.light.sun().is_some()),
            light: opt.light,
            lamps,
            tiles,
        };
        scene.build_bvh();
        scene
    }

    fn tri_box(t: &Tri) -> (V3, V3) {
        let b = add(t.a, t.e1);
        let c = add(t.a, t.e2);
        let mut lo = t.a;
        let mut hi = t.a;
        for p in [b, c] {
            for k in 0..3 {
                lo[k] = lo[k].min(p[k]);
                hi[k] = hi[k].max(p[k]);
            }
        }
        (lo, hi)
    }

    fn build_bvh(&mut self) {
        if self.tris.is_empty() {
            return;
        }
        let n = self.tris.len();
        let boxes: Vec<(V3, V3)> = self.tris.iter().map(Self::tri_box).collect();
        let centres: Vec<V3> = boxes.iter().map(|(l, h)| mul(add(*l, *h), 0.5)).collect();
        let mut order: Vec<usize> = (0..n).collect();
        let mut nodes = Vec::with_capacity(2 * n);
        // (node index, range)
        fn build(
            nodes: &mut Vec<Node>,
            order: &mut [usize],
            offset: usize,
            boxes: &[(V3, V3)],
            centres: &[V3],
        ) -> usize {
            let (mut lo, mut hi) = ([f64::MAX; 3], [f64::MIN; 3]);
            for &i in order.iter() {
                for k in 0..3 {
                    lo[k] = lo[k].min(boxes[i].0[k]);
                    hi[k] = hi[k].max(boxes[i].1[k]);
                }
            }
            let me = nodes.len();
            nodes.push(Node {
                lo,
                hi,
                start: offset as u32,
                count: order.len() as u32,
            });
            if order.len() <= 4 {
                return me;
            }
            let ext = sub(hi, lo);
            let axis = if ext[0] >= ext[1] && ext[0] >= ext[2] {
                0
            } else if ext[1] >= ext[2] {
                1
            } else {
                2
            };
            let mid = order.len() / 2;
            order.select_nth_unstable_by(mid, |a, b| {
                centres[*a][axis].total_cmp(&centres[*b][axis])
            });
            let (left, right) = order.split_at_mut(mid);
            build(nodes, left, offset, boxes, centres);
            let r = build(nodes, right, offset + mid, boxes, centres);
            nodes[me].start = r as u32;
            nodes[me].count = 0;
            me
        }
        build(&mut nodes, &mut order, 0, &boxes, &centres);
        let tris = order.iter().map(|i| self.tris[*i]).collect();
        self.tris = tris;
        self.nodes = nodes;
    }

    /// The nearest triangle a ray meets beyond `tmin` and before `tmax`.
    fn hit(&self, o: V3, d: V3, tmin: f64, tmax: f64) -> Option<(f64, usize)> {
        if self.nodes.is_empty() {
            return None;
        }
        let inv = d.map(|v| if v.abs() < 1e-15 { 1e15 } else { 1.0 / v });
        let mut best: Option<(f64, usize)> = None;
        let mut far = tmax;
        let mut stack = [0usize; 64];
        let mut sp = 1;
        while sp > 0 {
            sp -= 1;
            let idx = stack[sp];
            let node = &self.nodes[idx];
            // Slab test.
            let mut t0 = tmin;
            let mut t1 = far;
            let mut miss = false;
            for k in 0..3 {
                let a = (node.lo[k] - o[k]) * inv[k];
                let b = (node.hi[k] - o[k]) * inv[k];
                let (a, b) = if a < b { (a, b) } else { (b, a) };
                t0 = t0.max(a);
                t1 = t1.min(b);
                if t0 > t1 {
                    miss = true;
                    break;
                }
            }
            if miss {
                continue;
            }
            if node.count > 0 {
                for i in node.start as usize..(node.start + node.count) as usize {
                    let t = &self.tris[i];
                    // Möller–Trumbore.
                    let p = cross(d, t.e2);
                    let det = dot(t.e1, p);
                    if det.abs() < 1e-12 {
                        continue;
                    }
                    let inv_det = 1.0 / det;
                    let s = sub(o, t.a);
                    let u = dot(s, p) * inv_det;
                    if !(0.0..=1.0).contains(&u) {
                        continue;
                    }
                    let q = cross(s, t.e1);
                    let v = dot(d, q) * inv_det;
                    if v < 0.0 || u + v > 1.0 {
                        continue;
                    }
                    let tt = dot(t.e2, q) * inv_det;
                    if tt > tmin && tt < far {
                        far = tt;
                        best = Some((tt, i));
                    }
                }
            } else if sp + 2 <= stack.len() {
                // The right child, and the left one, which follows its
                // parent.
                stack[sp] = node.start as usize;
                stack[sp + 1] = idx + 1;
                sp += 2;
            }
        }
        best
    }

    /// How much light gets through to `o` along `d` (glass lets most of
    /// it, anything else none).
    fn transmit(&self, o: V3, d: V3, tmax: f64, skip: u64) -> f64 {
        let mut o = o;
        let mut left = tmax;
        let mut light = 1.0;
        for _ in 0..8 {
            match self.hit(o, d, 1e-4, left) {
                None => return light,
                Some((t, i)) => {
                    let tri = &self.tris[i];
                    if skip != 0 && tri.owner == skip {
                        // The lamp's own shade or bulb: on through it.
                    } else if !tri.glass {
                        return 0.0;
                    } else {
                        light *= 0.75;
                    }
                    o = add(o, mul(d, t));
                    left -= t;
                }
            }
        }
        light
    }
}

/// A small, fast random number generator (one per row).
struct Rng(u64);

impl Rng {
    fn next(&mut self) -> f64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        (self.0 >> 11) as f64 / (1u64 << 53) as f64
    }
}

/// The colour of the sky looking along `d`.
fn sky(light: Light, d: V3) -> V3 {
    let t = d[2].clamp(-1.0, 1.0);
    let (horizon, zenith) = light.sky_colours();
    if t < 0.0 {
        // Below the horizon, where nothing was modelled: a pale haze, as
        // the backdrop of a model photographed on a table.
        let k = (-t).powf(0.5);
        let floor = match light {
            Light::Night => [0.05, 0.05, 0.06],
            _ => [0.86, 0.86, 0.83],
        };
        return add(mul(horizon, 1.0 - k), mul(floor, k));
    }
    let k = t.powf(0.6);
    add(mul(horizon, 1.0 - k), mul(zenith, k))
}

/// The warm light of a lamp.
const LAMP: V3 = [1.0, 0.72, 0.42];
/// How bright a lamp is near it.
const LAMP_POWER: f64 = 2.2;

const SUN: V3 = [3.1, 2.95, 2.7];
const SKY_LIGHT: V3 = [0.62, 0.70, 0.82];

impl Scene {
    /// The light coming back along a ray.
    fn trace(&self, o: V3, d: V3, rng: &mut Rng, sky_rays: usize) -> V3 {
        let mut o = o;
        let mut throughput = [1.0, 1.0, 1.0];
        let mut out = [0.0; 3];
        for _ in 0..6 {
            let Some((t, i)) = self.hit(o, d, 1e-4, 1e9) else {
                return add(out, had(throughput, sky(self.light, d)));
            };
            let tri = &self.tris[i];
            let p = add(o, mul(d, t));
            let mut n = tri.n;
            if dot(n, d) > 0.0 {
                n = mul(n, -1.0);
            }
            if tri.glass {
                // A little of the sky reflected, the rest goes through,
                // tinted.
                let r = 0.08 + 0.3 * (1.0 + dot(d, n)).powi(5);
                let refl = sub(d, mul(n, 2.0 * dot(d, n)));
                out = add(out, had(throughput, mul(sky(self.light, refl), r)));
                throughput = had(throughput, mul([0.86, 0.92, 0.92], 1.0 - r));
                o = p;
                continue;
            }
            let lift = add(p, mul(n, 1e-4));
            // The sun, if this point sees it.
            let mut light = [0.0; 3];
            if let (Some(s), Some(sun)) = (self.sun, self.light.sun()) {
                let ns = dot(n, s);
                if ns > 0.0 {
                    let pass = self.transmit(lift, s, 1e9, 0);
                    light = add(light, mul(sun, ns * pass));
                }
            }
            // The lamps that reach it and that it sees (their shadows).
            let lit = self.light.lamps();
            for lamp in &self.lamps {
                let to = sub(lamp.at, p);
                let dist = dot(to, to).sqrt().max(0.05);
                if dist > lamp.reach {
                    continue;
                }
                let dir = mul(to, 1.0 / dist);
                let face = dot(n, dir);
                if face <= 0.0 && tri.owner != lamp.owner {
                    continue;
                }
                let fall = 1.0 - dist / lamp.reach;
                let pass = if tri.owner == lamp.owner {
                    1.0
                } else {
                    self.transmit(lift, dir, dist, lamp.owner)
                };
                let k = LAMP_POWER * lit * fall * fall * face.max(0.25) * pass;
                light = add(light, mul(LAMP, k));
            }
            // The sky, less what the surroundings hide (cosine-weighted).
            let (tx, ty) = basis(n);
            let mut open = 0.0;
            for _ in 0..sky_rays {
                let (u, v) = (rng.next(), rng.next());
                let r = u.sqrt();
                let phi = std::f64::consts::TAU * v;
                let dir = add(
                    add(mul(tx, r * phi.cos()), mul(ty, r * phi.sin())),
                    mul(n, (1.0 - u).max(0.0).sqrt()),
                );
                if self.hit(lift, dir, 1e-4, 25.0).is_none() {
                    open += 1.0;
                }
            }
            let ao = if sky_rays == 0 {
                1.0
            } else {
                open / sky_rays as f64
            };
            light = add(light, mul(self.light.sky_light(), 0.25 + 0.75 * ao));
            let albedo = self.albedo(tri, p, d);
            out = add(out, had(throughput, had(albedo, light)));
            if tri.glow && lit > 0.0 {
                // A lampshade, a bulb, a flame: it glows.
                let glow = mul(add(mul(albedo, 0.6), [0.55, 0.42, 0.25]), 1.6 * lit);
                out = add(out, had(throughput, glow));
            }
            return out;
        }
        out
    }
}

impl Scene {
    /// The colour of a triangle at `p`, with its texture, seen along `d`.
    fn albedo(&self, tri: &Tri, p: V3, d: V3) -> V3 {
        if tri.tex == 0 {
            return tri.albedo;
        }
        let Some(tile) = self.tiles.get(tri.tex as usize).and_then(|t| t.as_ref()) else {
            return tri.albedo;
        };
        let q = textures::texture_point(p, tri.n, tri.scale as f64);
        let mut c = tile.apply(tri.srgb, q);
        if tile.kind == archisim_core::TextureKind::Velvet {
            let k = textures::velvet_sheen(dot(tri.n, d));
            c = c.map(|v| v * (1.0 + k));
        }
        c.map(|v| srgb_to_linear((v.clamp(0.0, 1.0) * 255.0).round() as u8))
    }
}

/// Two directions square to `n` and to each other.
fn basis(n: V3) -> (V3, V3) {
    let a = if n[0].abs() > 0.9 {
        [0.0, 1.0, 0.0]
    } else {
        [1.0, 0.0, 0.0]
    };
    let t = norm(cross(n, a));
    (t, cross(n, t))
}

/// Renders the model. `progress` hears how much is done (0 to 1), from
/// any thread.
pub fn render(model: &Model, view: &View, opt: &Options, progress: &(dyn Fn(f64) + Sync)) -> Image {
    let scene = Scene::build(model, opt);
    let (w, h) = (opt.width.max(1), opt.height.max(1));
    let forward = norm(sub(view.target, view.eye));
    let right = norm(cross(forward, [0.0, 0.0, 1.0]));
    let up = cross(right, forward);
    let aspect = w as f64 / h as f64;
    let dist = dot(sub(view.target, view.eye), forward);
    let half = if view.perspective {
        dist * (view.fov_y / 2.0).tan()
    } else {
        view.half_height
    };
    let samples = opt.samples.max(1);
    let mut rgb = vec![0u8; w * h * 3];
    let next = AtomicUsize::new(0);
    let done = AtomicUsize::new(0);
    let threads = std::thread::available_parallelism().map_or(4, |n| n.get());
    let rows: Vec<std::sync::Mutex<Option<Vec<u8>>>> =
        (0..h).map(|_| std::sync::Mutex::new(None)).collect();
    std::thread::scope(|sc| {
        for _ in 0..threads {
            sc.spawn(|| loop {
                let y = next.fetch_add(1, Ordering::Relaxed);
                if y >= h {
                    break;
                }
                let mut rng =
                    Rng(0x9E37_79B9_7F4A_7C15 ^ (y as u64 + 1).wrapping_mul(0x2545_F491_4F6C_DD1D));
                let mut row = vec![0u8; w * 3];
                for x in 0..w {
                    let mut acc = [0.0; 3];
                    for sy in 0..samples {
                        for sx in 0..samples {
                            let fx =
                                (x as f64 + (sx as f64 + rng.next()) / samples as f64) / w as f64;
                            let fy =
                                (y as f64 + (sy as f64 + rng.next()) / samples as f64) / h as f64;
                            let ndc = [2.0 * fx - 1.0, 1.0 - 2.0 * fy];
                            let offset =
                                add(mul(right, ndc[0] * half * aspect), mul(up, ndc[1] * half));
                            let (o, d) = if view.perspective {
                                (view.eye, norm(sub(add(view.target, offset), view.eye)))
                            } else {
                                (
                                    add(sub(view.target, mul(forward, dist * 4.0)), offset),
                                    forward,
                                )
                            };
                            acc = add(acc, scene.trace(o, d, &mut rng, opt.sky_rays));
                        }
                    }
                    let c = mul(acc, 1.0 / (samples * samples) as f64);
                    // A gentle tone curve, so the sun does not burn.
                    let tone = |v: f64| {
                        let v = v * 0.85;
                        v / (1.0 + v * 0.35)
                    };
                    for k in 0..3 {
                        row[x * 3 + k] = linear_to_srgb(tone(c[k]));
                    }
                }
                if let Ok(mut slot) = rows[y].lock() {
                    *slot = Some(row);
                }
                let n = done.fetch_add(1, Ordering::Relaxed) + 1;
                if n.is_multiple_of(8) || n == h {
                    progress(n as f64 / h as f64);
                }
            });
        }
    });
    for (y, slot) in rows.into_iter().enumerate() {
        if let Some(row) = slot.into_inner().ok().flatten() {
            rgb[y * w * 3..(y + 1) * w * 3].copy_from_slice(&row);
        }
    }
    Image {
        width: w,
        height: h,
        rgb,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use archisim_core::*;

    #[test]
    fn a_box_in_the_sun_casts_a_shadow() {
        let mut m = Model::new("t");
        let pb = m.add_level("PB", 0.0);
        let mat = m.add_material("Blanco", "plaster", [240, 240, 240], CutPattern::Solid);
        let wt = m
            .add_wall_type(
                "Muro",
                WallFunction::Exterior,
                vec![Layer {
                    material: mat,
                    thickness: 0.3,
                    function: LayerFunction::Structure,
                }],
            )
            .unwrap();
        m.add_wall(NewWall {
            wall_type: wt,
            level: pb,
            start: Point2::new(0.0, 0.0),
            end: Point2::new(6.0, 0.0),
            reference: ReferenceLine::Center,
            height: 3.0,
            base_offset: 0.0,
            bulge: 0.0,
            top: WallTop::Unconnected,
        })
        .unwrap();
        let view = View::fit(&m);
        let img = render(
            &m,
            &view,
            &Options {
                width: 80,
                height: 50,
                samples: 1,
                sky_rays: 4,
                ..Default::default()
            },
            &|_| {},
        );
        assert_eq!(img.rgb.len(), 80 * 50 * 3);
        // Not all one colour: a wall, its shadow, the ground and the sky.
        let first = &img.rgb[0..3];
        assert!(img.rgb.chunks(3).any(|p| p != first));
        // Somewhere darker than the sunlit ground: the shadow or the wall.
        let lum = |p: &[u8]| p.iter().map(|&v| v as u32).sum::<u32>();
        let max = img.rgb.chunks(3).map(lum).max().unwrap();
        let min = img.rgb.chunks(3).map(lum).min().unwrap();
        assert!(max > min + 120, "{min} {max}");
    }

    /// A corner of two walls and a floor lamp in front of it.
    fn lamp_corner() -> Model {
        let mut m = Model::new("Lámpara");
        let pb = m.add_level("PB", 0.0);
        let lib = archisim_library::add_library(&mut m).unwrap();
        for (a, b) in [((0.0, 0.0), (4.0, 0.0)), ((0.0, 0.0), (0.0, 4.0))] {
            m.add_wall(NewWall {
                wall_type: lib.facade,
                level: pb,
                start: Point2::new(a.0, a.1),
                end: Point2::new(b.0, b.1),
                reference: ReferenceLine::Center,
                height: 2.7,
                base_offset: 0.0,
                bulge: 0.0,
                top: WallTop::Unconnected,
            })
            .unwrap();
        }
        let t = archisim_library::add_object_entry(&mut m, "Lámpara de pie").unwrap();
        m.add_object(NewObject {
            level: pb,
            object_type: t,
            position: Point2::new(1.0, 1.0),
            rotation: 0.0,
            base_offset: 0.0,
        })
        .unwrap();
        m
    }

    #[test]
    fn at_night_the_lamps_light_the_walls() {
        let m = lamp_corner();
        let view = View {
            eye: [4.0, 4.0, 1.6],
            target: [0.5, 0.5, 1.0],
            fov_y: 1.0,
            perspective: true,
            half_height: 2.0,
        };
        let opt = |light| Options {
            width: 60,
            height: 40,
            samples: 1,
            sky_rays: 2,
            light,
            textures: true,
        };
        let lum = |img: &Image| -> f64 {
            img.rgb.iter().map(|&v| v as f64).sum::<f64>() / img.rgb.len() as f64
        };
        let day = render(&m, &view, &opt(Light::Day), &|_| {});
        let night = render(&m, &view, &opt(Light::Night), &|_| {});
        // Darker than the day, but the lamp and the walls near it glow:
        // some pixels are well lit and warm (more red than blue).
        assert!(
            lum(&night) < lum(&day) * 0.7,
            "{} {}",
            lum(&night),
            lum(&day)
        );
        let warm = night
            .rgb
            .chunks(3)
            .filter(|p| p[0] as i32 > 120 && p[0] as i32 > p[2] as i32 + 25)
            .count();
        assert!(warm > 20, "{warm}");
        // Without the lamp, night is nearly black.
        let mut dark = m.clone();
        let lamp = dark.objects().next().unwrap().id;
        dark.remove_object(lamp).unwrap();
        let none = render(&dark, &view, &opt(Light::Night), &|_| {});
        assert!(
            lum(&none) < lum(&night) * 0.6,
            "{} {}",
            lum(&none),
            lum(&night)
        );
    }

    #[test]
    fn textures_show_in_the_picture() {
        let mut m = lamp_corner();
        // The façade's outer leaf in brick.
        let brick = m
            .materials()
            .find(|x| x.name == "Ladrillo perforado")
            .map(|x| x.id)
            .unwrap();
        m.set_appearance(
            brick,
            archisim_core::Appearance {
                color: None,
                texture: Some(TextureKind::Brick),
                scale: None,
                growth: None,
            },
        )
        .unwrap();
        let view = View::fit(&m);
        let opt = |textures| Options {
            width: 60,
            height: 40,
            samples: 1,
            sky_rays: 0,
            light: Light::Day,
            textures,
        };
        let a = render(&m, &view, &opt(true), &|_| {});
        let b = render(&m, &view, &opt(false), &|_| {});
        assert_ne!(a.rgb, b.rgb);
    }
}
