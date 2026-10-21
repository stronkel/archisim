//! The plant as coloured triangles, in three levels of detail: the plant
//! itself, a lighter one (clumps merged, twigs left out) and a very light
//! one for plans with hundreds of trees (a few masses and the trunk).

use crate::grow::{
    add, cross, dist, dot, len, lerp3, mul, norm, perpendicular, sub, Branch, Clump, ClumpKind,
    Frond, FrondKind, Plant, P3,
};
use crate::rng::Rng;
use crate::{Colors, Foliage, Habit, Rgb};
use std::collections::HashMap;
use std::f64::consts::{PI, TAU};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Lod {
    /// Every branch and clump.
    Full,
    /// Merged clumps, no twigs: a few thousand triangles.
    Medium,
    /// A few masses and the trunk: a few hundred.
    Low,
}

impl Lod {
    pub const ALL: [Lod; 3] = [Lod::Full, Lod::Medium, Lod::Low];
}

/// A plant as triangles: wood, leaves, and flowers and fruit, each its own
/// list (each will be a material), with a colour per corner.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct FloraMesh {
    pub positions: Vec<[f32; 3]>,
    pub colors: Vec<Rgb>,
    pub bark: Vec<[u32; 3]>,
    pub leaves: Vec<[u32; 3]>,
    pub flowers: Vec<[u32; 3]>,
}

#[derive(Clone, Copy)]
enum Part {
    Bark,
    Leaves,
    Flowers,
}

impl FloraMesh {
    pub fn triangle_count(&self) -> usize {
        self.bark.len() + self.leaves.len() + self.flowers.len()
    }

    fn vertex(&mut self, p: P3, c: Rgb) -> u32 {
        self.positions.push([p[0] as f32, p[1] as f32, p[2] as f32]);
        self.colors.push(c);
        (self.positions.len() - 1) as u32
    }

    fn tri(&mut self, part: Part, t: [u32; 3]) {
        match part {
            Part::Bark => self.bark.push(t),
            Part::Leaves => self.leaves.push(t),
            Part::Flowers => self.flowers.push(t),
        }
    }

    /// A triangle seen from both sides (blades, fronds).
    fn both(&mut self, part: Part, t: [u32; 3]) {
        self.tri(part, t);
        self.tri(part, [t[0], t[2], t[1]]);
    }

    /// Highest and lowest corners.
    pub fn bounds(&self) -> ([f32; 3], [f32; 3]) {
        let mut lo = [f32::INFINITY; 3];
        let mut hi = [f32::NEG_INFINITY; 3];
        for p in &self.positions {
            for k in 0..3 {
                lo[k] = lo[k].min(p[k]);
                hi[k] = hi[k].max(p[k]);
            }
        }
        (lo, hi)
    }
}

fn shade(c: Rgb, k: f64) -> Rgb {
    c.map(|v| (v as f64 * k).round().clamp(0.0, 255.0) as u8)
}

/// A colour in its own shade: lighter and warmer for `tint` above 0,
/// darker and cooler below.
fn tinted(c: Rgb, tint: f64, light: f64) -> Rgb {
    let t = tint.clamp(-1.0, 1.0);
    let warm = [
        c[0] as f64 * 1.16 + 14.0,
        c[1] as f64 * 1.12 + 10.0,
        c[2] as f64 * 0.95,
    ];
    let cool = [
        c[0] as f64 * 0.78,
        c[1] as f64 * 0.86,
        c[2] as f64 * 0.92 + 8.0,
    ];
    let base = [c[0] as f64, c[1] as f64, c[2] as f64];
    let mixed: [f64; 3] = if t >= 0.0 {
        [0, 1, 2].map(|k| base[k] + (warm[k] - base[k]) * t)
    } else {
        [0, 1, 2].map(|k| base[k] + (cool[k] - base[k]) * -t)
    };
    let k = 0.62 + 0.48 * light.clamp(0.0, 1.0);
    mixed.map(|v| (v * k).round().clamp(0.0, 255.0) as u8)
}

impl Plant {
    /// The plant as triangles at a level of detail.
    pub fn mesh(&self, colors: &Colors, lod: Lod) -> FloraMesh {
        let mut m = FloraMesh::default();
        let palm = self.species.habit == Habit::Palm;
        for b in &self.branches {
            // Under a dense crown of needles or scales the small
            // branches do not show.
            let hidden = b.level >= 2
                && matches!(
                    self.species.foliage,
                    crate::Foliage::Needles | crate::Foliage::Scales
                )
                && self.species.density >= 0.7;
            if hidden {
                continue;
            }
            let (keep, sides) = match lod {
                Lod::Full => (true, [10, 7, 5, 4, 4][b.level.min(4) as usize]),
                Lod::Medium => (
                    b.level <= 1 || (b.green && b.level <= 2),
                    [8, 6, 4, 4, 4][b.level.min(4) as usize],
                ),
                Lod::Low => (b.level == 0, 6),
            };
            if !keep {
                continue;
            }
            // Twigs thinner than a finger are left to the leaves, but in
            // a leafless broom they are the plant.
            if lod != Lod::Full && b.radii[0] < 0.006 && !b.green {
                continue;
            }
            let color = if b.green {
                shade(colors.leaves, 0.85)
            } else {
                colors.bark
            };
            let rings = palm && b.level == 0 && lod == Lod::Full;
            // Twigs: three sides and half the points are enough.
            let thin = b.radii[0] < 0.012 && b.level >= 2;
            if thin {
                let pts: Vec<usize> = (0..b.points.len())
                    .step_by(2)
                    .chain([b.points.len() - 1])
                    .collect();
                let mut pts = pts;
                pts.dedup();
                let twig = Branch {
                    points: pts.iter().map(|&i| b.points[i]).collect(),
                    radii: pts.iter().map(|&i| b.radii[i]).collect(),
                    level: b.level,
                    green: b.green,
                };
                tube(&mut m, &twig, 3, color, false);
            } else {
                tube(&mut m, b, sides, color, rings);
            }
        }
        let clumps: Vec<Clump> = match lod {
            Lod::Full => self.clumps.clone(),
            Lod::Medium => merge(&self.clumps, self.species.clump * 2.6, 1.05),
            Lod::Low => {
                let cell = (self.height.max(self.radius * 2.0) / 2.6).max(self.species.clump);
                merge(&self.clumps, cell, 0.92)
            }
        };
        for c in &clumps {
            let (part, base) = match c.kind {
                ClumpKind::Leaves => (Part::Leaves, colors.leaves),
                ClumpKind::Flowers => (Part::Flowers, colors.flowers),
                ClumpKind::Fruit => (Part::Flowers, colors.fruit),
            };
            let subdiv = c.radius > self.species.clump * 3.0;
            let color = tinted(base, c.tint, c.light);
            blob(&mut m, c, part, color, subdiv);
            // Up close, leaves stand out of the clumps: the crown's edge
            // is ragged, not a row of balls.
            if lod == Lod::Full && c.kind == ClumpKind::Leaves {
                fringe(&mut m, c, part, color, self.species.foliage);
            }
        }
        for f in &self.fronds {
            frond(&mut m, f, colors, lod);
        }
        m
    }
}

/// Clumps of a grid cell merged into one of about the same leaf area.
pub(crate) fn merge(clumps: &[Clump], cell: f64, grow: f64) -> Vec<Clump> {
    let cell = cell.max(1e-3);
    let mut cells: HashMap<(i32, i32, i32, u8), Vec<&Clump>> = HashMap::new();
    for c in clumps {
        let k = (
            (c.center[0] / cell).floor() as i32,
            (c.center[1] / cell).floor() as i32,
            (c.center[2] / cell).floor() as i32,
            match c.kind {
                ClumpKind::Leaves => 0,
                ClumpKind::Flowers => 1,
                ClumpKind::Fruit => 2,
            },
        );
        cells.entry(k).or_default().push(c);
    }
    let mut keys: Vec<_> = cells.keys().copied().collect();
    keys.sort();
    keys.into_iter()
        .map(|k| {
            let v = &cells[&k];
            if v.len() == 1 {
                return v[0].clone();
            }
            let w: f64 = v.iter().map(|c| c.radius * c.radius).sum();
            let center = v.iter().fold([0.0; 3], |a, c| {
                add(a, mul(c.center, c.radius * c.radius / w))
            });
            // As big as the area of its clumps, but no bigger than their
            // spread.
            let spread = v
                .iter()
                .map(|c| dist(c.center, center) + c.radius)
                .fold(0.0, f64::max);
            let radius = (w.sqrt() * grow).min(spread * 1.05).max(v[0].radius);
            Clump {
                center,
                radius,
                axis: [0.0, 0.0, 1.0],
                stretch: 1.0,
                flat: v.iter().map(|c| c.flat).sum::<f64>() / v.len() as f64,
                kind: v[0].kind,
                light: v.iter().map(|c| c.light).sum::<f64>() / v.len() as f64,
                tint: v.iter().map(|c| c.tint).sum::<f64>() / v.len() as f64 * 0.6,
                seed: v[0].seed,
            }
        })
        .collect()
}

/// A tube along a branch, closed at both ends.
fn tube(m: &mut FloraMesh, b: &Branch, sides: usize, color: Rgb, rings: bool) {
    let (points, radii): (Vec<P3>, Vec<f64>) = if rings {
        // A palm trunk: the scars of old fronds as rings.
        let total = b.length();
        let n = ((total / 0.14).round() as usize).clamp(8, 160);
        (0..=n)
            .map(|i| {
                let (p, _, r) = b.at(i as f64 / n as f64);
                let bump = if i % 2 == 0 { 1.0 } else { 1.07 };
                (p, r * bump)
            })
            .unzip()
    } else {
        (b.points.clone(), b.radii.clone())
    };
    if points.len() < 2 {
        return;
    }
    let mut frame = perpendicular(norm(sub(points[1], points[0])));
    let mut prev_dir = norm(sub(points[1], points[0]));
    let mut ring_start = Vec::with_capacity(points.len());
    let mut rng = Rng::new(points.len() as u64 ^ (radii[0].to_bits()));
    for i in 0..points.len() {
        let d = if i + 1 < points.len() {
            norm(sub(points[i + 1], points[i]))
        } else {
            prev_dir
        };
        // Parallel transport: turn the frame as little as the line turns.
        let ax = cross(prev_dir, d);
        let s = len(ax);
        if s > 1e-9 {
            let a = s.asin().clamp(-PI, PI);
            frame = crate::grow::rotate(frame, norm(ax), a);
        }
        frame = norm(sub(frame, mul(d, dot(frame, d))));
        let other = cross(d, frame);
        prev_dir = d;
        let start = m.positions.len() as u32;
        let k = if rings && i % 2 == 1 { 0.88 } else { 1.0 } * rng.range(0.93, 1.03);
        // Darker at the foot.
        let foot = if b.level == 0 {
            0.82 + 0.18 * (i as f64 / points.len() as f64).min(1.0)
        } else {
            1.0
        };
        let c = shade(color, k * foot);
        for j in 0..sides {
            let a = TAU * j as f64 / sides as f64;
            let off = add(
                mul(frame, a.cos() * radii[i]),
                mul(other, a.sin() * radii[i]),
            );
            m.vertex(add(points[i], off), c);
        }
        ring_start.push(start);
    }
    let s = sides as u32;
    for w in ring_start.windows(2) {
        let (a, b) = (w[0], w[1]);
        for j in 0..s {
            let j1 = (j + 1) % s;
            m.tri(Part::Bark, [a + j, a + j1, b + j1]);
            m.tri(Part::Bark, [a + j, b + j1, b + j]);
        }
    }
    // Caps.
    let first = ring_start[0];
    let last = *ring_start.last().expect("two rings at least");
    let c0 = m.vertex(points[0], shade(color, 0.7));
    let c1 = m.vertex(*points.last().expect("points"), shade(color, 0.9));
    for j in 0..s {
        let j1 = (j + 1) % s;
        m.tri(Part::Bark, [c0, first + j1, first + j]);
        m.tri(Part::Bark, [c1, last + j, last + j1]);
    }
}

fn icosahedron() -> Body {
    // The golden ratio, computed so that it builds on every toolchain.
    let phi = (1.0 + 5.0_f64.sqrt()) / 2.0;
    let v = [
        [-1.0, phi, 0.0],
        [1.0, phi, 0.0],
        [-1.0, -phi, 0.0],
        [1.0, -phi, 0.0],
        [0.0, -1.0, phi],
        [0.0, 1.0, phi],
        [0.0, -1.0, -phi],
        [0.0, 1.0, -phi],
        [phi, 0.0, -1.0],
        [phi, 0.0, 1.0],
        [-phi, 0.0, -1.0],
        [-phi, 0.0, 1.0],
    ]
    .map(norm)
    .to_vec();
    let f = vec![
        [0, 11, 5],
        [0, 5, 1],
        [0, 1, 7],
        [0, 7, 10],
        [0, 10, 11],
        [1, 5, 9],
        [5, 11, 4],
        [11, 10, 2],
        [10, 7, 6],
        [7, 1, 8],
        [3, 9, 4],
        [3, 4, 2],
        [3, 2, 6],
        [3, 6, 8],
        [3, 8, 9],
        [4, 9, 5],
        [2, 4, 11],
        [6, 2, 10],
        [8, 6, 7],
        [9, 8, 1],
    ];
    (v, f)
}

/// The icosahedron split once: 80 faces, rounder.
fn sphere80() -> Body {
    let (mut v, f) = icosahedron();
    let mut mid: HashMap<(usize, usize), usize> = HashMap::new();
    let mut out = Vec::new();
    let mut half = |a: usize, b: usize, v: &mut Vec<P3>| -> usize {
        let k = (a.min(b), a.max(b));
        *mid.entry(k).or_insert_with(|| {
            v.push(norm(lerp3(v[a], v[b], 0.5)));
            v.len() - 1
        })
    };
    for [a, b, c] in f {
        let ab = half(a, b, &mut v);
        let bc = half(b, c, &mut v);
        let ca = half(c, a, &mut v);
        out.extend([[a, ab, ca], [b, bc, ab], [c, ca, bc], [ab, bc, ca]]);
    }
    (v, out)
}

/// Corners and faces of a unit body.
type Body = (Vec<P3>, Vec<[usize; 3]>);

fn unit_shapes() -> &'static [Body; 2] {
    static S: std::sync::OnceLock<[Body; 2]> = std::sync::OnceLock::new();
    S.get_or_init(|| [icosahedron(), sphere80()])
}

/// A clump: a lumpy rounded body, drawn out along its axis.
fn blob(m: &mut FloraMesh, c: &Clump, part: Part, color: Rgb, fine: bool) {
    let (v, f) = &unit_shapes()[fine as usize];
    let mut rng = Rng::new(c.seed);
    let ax = norm(c.axis);
    let start = m.positions.len() as u32;
    let lumpy = if fine { 0.12 } else { 0.22 };
    for p in v {
        let k = 1.0 + rng.signed() * lumpy;
        let mut q = mul(*p, c.radius * k);
        // Drawn out along the axis.
        if (c.stretch - 1.0).abs() > 1e-6 {
            let along = dot(q, ax);
            q = add(q, mul(ax, along * (c.stretch - 1.0)));
        }
        q[2] *= c.flat;
        // Lighter on top, darker underneath.
        let up = 0.86 + 0.22 * p[2];
        let jitter = rng.range(0.95, 1.05);
        m.vertex(add(c.center, q), shade(color, up * jitter));
    }
    for t in f {
        m.tri(
            part,
            [
                start + t[0] as u32,
                start + t[1] as u32,
                start + t[2] as u32,
            ],
        );
    }
}

/// Leaves standing out of a clump: thin pointed blades, both sides seen.
fn fringe(m: &mut FloraMesh, c: &Clump, part: Part, color: Rgb, foliage: Foliage) {
    let (count, length, width) = match foliage {
        Foliage::Broad => (9, 0.5, 0.45),
        Foliage::Fine => (12, 0.42, 0.28),
        Foliage::Needles => (14, 0.6, 0.1),
        _ => return,
    };
    let mut rng = Rng::new(c.seed ^ 0xF00D);
    for _ in 0..count {
        // Outwards and a little upwards.
        let d = norm([rng.signed(), rng.signed(), rng.signed() * 0.7 + 0.3]);
        let r = c.radius;
        let mut base = mul(d, r * 0.72);
        base[2] *= c.flat;
        let base = add(c.center, base);
        let len = r * length * rng.range(0.7, 1.3);
        let bend = [rng.signed() * 0.3, rng.signed() * 0.3, -0.25];
        let tip = add(base, mul(norm(add(d, bend)), len));
        let side = crate::grow::rotate(perpendicular(d), d, rng.range(0.0, TAU));
        let w = len * width * 0.5;
        let a = m.vertex(add(base, mul(side, w)), shade(color, rng.range(0.92, 1.12)));
        let b = m.vertex(add(base, mul(side, -w)), shade(color, rng.range(0.85, 1.0)));
        let t = m.vertex(tip, shade(color, 1.1));
        m.both(part, [a, b, t]);
    }
}

/// A frond, blade or sword.
fn frond(m: &mut FloraMesh, f: &Frond, colors: &Colors, lod: Lod) {
    let n = f.rachis.len();
    if n < 2 {
        return;
    }
    let color = tinted(colors.leaves, f.tint, f.light);
    let dirs: Vec<P3> = (0..n)
        .map(|i| {
            let a = f.rachis[i.saturating_sub(1)];
            let b = f.rachis[(i + 1).min(n - 1)];
            norm(sub(b, a))
        })
        .collect();
    // Across the blade at each point: square to the rachis, kept level.
    let sides: Vec<P3> = dirs
        .iter()
        .map(|d| {
            let s = sub(f.side, mul(*d, dot(f.side, *d)));
            if len(s) < 1e-6 {
                perpendicular(*d)
            } else {
                norm(s)
            }
        })
        .collect();
    match f.kind {
        FrondKind::Pinnate => {
            let rib = shade(color, 1.1);
            let step = match lod {
                Lod::Full => 1,
                Lod::Medium => 3,
                Lod::Low => 6,
            };
            let mut i = 0;
            while i + step < n {
                let j = i + step;
                let (a, b) = (f.rachis[i], f.rachis[j]);
                let s = (i as f64 + step as f64 * 0.5) / (n - 1) as f64;
                let w = f.width * (PI * s.powf(0.75)).sin().max(0.0).powf(0.6);
                let down = norm(cross(sides[i], dirs[i]));
                let fold = mul(down, -w * 0.35);
                for sign in [-1.0, 1.0] {
                    // A leaflet pointing forwards, folded into a V.
                    let tip = add(add(lerp3(a, b, 0.9), mul(sides[i], sign * w)), fold);
                    let ia = m.vertex(a, rib);
                    let ib = m.vertex(b, rib);
                    let it = m.vertex(tip, color);
                    m.both(Part::Leaves, [ia, ib, it]);
                    if lod == Lod::Full {
                        // A second, shorter leaflet between, for a fuller
                        // feather.
                        let mid = lerp3(a, b, 0.5);
                        let tip2 = add(
                            add(
                                lerp3(b, f.rachis[(j + 1).min(n - 1)], 0.5),
                                mul(sides[j], sign * w * 0.85),
                            ),
                            fold,
                        );
                        let im = m.vertex(mid, rib);
                        let ib2 = m.vertex(b, rib);
                        let it2 = m.vertex(tip2, shade(color, 0.92));
                        m.both(Part::Leaves, [im, ib2, it2]);
                    }
                }
                i = j;
            }
        }
        FrondKind::Fan => {
            // The stalk, then the fan.
            if lod != Lod::Low {
                let pick: Vec<usize> = if lod == Lod::Full {
                    (0..n).step_by(3).chain([n - 1]).collect()
                } else {
                    vec![0, n / 2, n - 1]
                };
                let mut pick = pick;
                pick.dedup();
                let stalk = Branch {
                    points: pick.iter().map(|&i| f.rachis[i]).collect(),
                    radii: vec![f.width * 0.035; pick.len()],
                    level: 3,
                    green: true,
                };
                tube(m, &stalk, 3, shade(colors.leaves, 0.8), false);
            }
            let c = f.rachis[n - 1];
            let d = dirs[n - 1];
            let s = sides[n - 1];
            // The fan faces up, a little towards where it points.
            let up = norm(cross(s, d));
            let segs = if lod == Lod::Full { 18 } else { 7 };
            let center = m.vertex(c, shade(color, 0.85));
            let mut prev: Option<u32> = None;
            for k in 0..=segs {
                let a = (-80.0 + 160.0 * k as f64 / segs as f64).to_radians();
                let pleat = if k % 2 == 0 { 0.07 } else { -0.07 } * f.width;
                let r = f.width * if k % 2 == 0 { 1.0 } else { 0.9 };
                let e = add(
                    add(c, add(mul(d, a.cos() * r), mul(s, a.sin() * r))),
                    mul(up, pleat + f.width * 0.12 * a.cos()),
                );
                let ie = m.vertex(e, shade(color, if k % 2 == 0 { 1.05 } else { 0.9 }));
                if let Some(p) = prev {
                    m.both(Part::Leaves, [center, p, ie]);
                }
                prev = Some(ie);
            }
        }
        FrondKind::Blade => {
            let mut prev: Option<(u32, u32)> = None;
            let step = match lod {
                Lod::Full => 1,
                Lod::Medium => 2,
                Lod::Low => 4,
            };
            let mut i = 0;
            loop {
                let s = i as f64 / (n - 1) as f64;
                let w = f.width * (1.0 - s).powf(0.8);
                let p = f.rachis[i];
                let l = m.vertex(add(p, mul(sides[i], w)), color);
                let r = m.vertex(add(p, mul(sides[i], -w)), shade(color, 0.9));
                if let Some((pl, pr)) = prev {
                    m.both(Part::Leaves, [pl, pr, r]);
                    m.both(Part::Leaves, [pl, r, l]);
                }
                prev = Some((l, r));
                if i == n - 1 {
                    break;
                }
                i = (i + step).min(n - 1);
            }
        }
        FrondKind::Sword => {
            // Three-sided, thick at the foot, to a point.
            let mut rings: Vec<u32> = Vec::new();
            for i in 0..n {
                let s = i as f64 / (n - 1) as f64;
                let w = f.width * (1.0 - s).powf(0.7);
                let up = norm(cross(sides[i], dirs[i]));
                let p = f.rachis[i];
                let a = m.vertex(add(p, mul(sides[i], w)), color);
                m.vertex(add(p, mul(sides[i], -w)), shade(color, 0.9));
                m.vertex(add(p, mul(up, -w * 0.45)), shade(color, 0.75));
                rings.push(a);
            }
            for w in rings.windows(2) {
                let (a, b) = (w[0], w[1]);
                for j in 0..3u32 {
                    let j1 = (j + 1) % 3;
                    m.tri(Part::Leaves, [a + j, a + j1, b + j1]);
                    m.tri(Part::Leaves, [a + j, b + j1, b + j]);
                }
            }
            let a = rings[0];
            m.tri(Part::Leaves, [a, a + 2, a + 1]);
        }
    }
}
