//! Growing a plant: trunk, branches and clumps of leaves for trees,
//! conifers and shrubs; trunk and fronds for palms; blades for grasses and
//! rosettes. Everything depends only on the species, the seed and the
//! variation.

use crate::envelope::Envelope;
use crate::rng::Rng;
use crate::{CrownShape, Foliage, Habit, Species};
use std::f64::consts::{PI, TAU};

pub type P3 = [f64; 3];

/// The golden angle: leaves and branches round a stem.
const GOLDEN: f64 = 2.399_963_229_728_653;

/// A trunk, stem or branch: a line of points with its radius at each.
#[derive(Debug, Clone, PartialEq)]
pub struct Branch {
    pub points: Vec<P3>,
    pub radii: Vec<f64>,
    /// 0 trunk, 1 limbs, 2 and more, smaller branches.
    pub level: u8,
    /// Green twigs (broom), coloured as the leaves.
    pub green: bool,
}

impl Branch {
    pub fn length(&self) -> f64 {
        self.points.windows(2).map(|w| dist(w[0], w[1])).sum()
    }

    /// The point and the direction at a share of the length.
    pub fn at(&self, s: f64) -> (P3, P3, f64) {
        let total = self.length().max(1e-9);
        let mut want = s.clamp(0.0, 1.0) * total;
        for (i, w) in self.points.windows(2).enumerate() {
            let l = dist(w[0], w[1]);
            if want <= l || i == self.points.len() - 2 {
                let u = if l > 1e-12 { (want / l).min(1.0) } else { 0.0 };
                let p = lerp3(w[0], w[1], u);
                let r = self.radii[i] + (self.radii[i + 1] - self.radii[i]) * u;
                return (p, norm(sub(w[1], w[0])), r);
            }
            want -= l;
        }
        let n = self.points.len();
        (self.points[n - 1], [0.0, 0.0, 1.0], self.radii[n - 1])
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClumpKind {
    Leaves,
    Flowers,
    Fruit,
}

/// A clump of leaves (or of flowers, or a fruit): a rounded body.
#[derive(Debug, Clone, PartialEq)]
pub struct Clump {
    pub center: P3,
    pub radius: f64,
    /// The direction it is drawn out along, and how much (1: round).
    pub axis: P3,
    pub stretch: f64,
    /// Height against width (below 1, flattened).
    pub flat: f64,
    pub kind: ClumpKind,
    /// Light on it: 0 deep in the crown, 1 on top in the sun.
    pub light: f64,
    /// Its own shade of the colour, from -1 to 1.
    pub tint: f64,
    /// A number of its own, for the shape of its body.
    pub seed: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FrondKind {
    /// A feather: leaflets both sides of the rachis.
    Pinnate,
    /// A stalk ending in a pleated fan.
    Fan,
    /// A narrow blade (grasses).
    Blade,
    /// A thick, stiff sword (agave, yucca).
    Sword,
}

/// A frond of a palm, a blade of a grass or a leaf of a rosette.
#[derive(Debug, Clone, PartialEq)]
pub struct Frond {
    /// Its midrib, from where it starts to its tip.
    pub rachis: Vec<P3>,
    /// Half width of the blade at its widest (of the fan, its radius).
    pub width: f64,
    pub kind: FrondKind,
    /// Across the blade at its start, level.
    pub side: P3,
    pub light: f64,
    pub tint: f64,
}

/// A grown plant.
#[derive(Debug, Clone, PartialEq)]
pub struct Plant {
    pub species: Species,
    pub branches: Vec<Branch>,
    pub clumps: Vec<Clump>,
    pub fronds: Vec<Frond>,
    /// The crown's axis in plan (the top of the trunk).
    pub center: [f64; 2],
    /// Highest point.
    pub height: f64,
    /// Furthest point from the foot, in plan.
    pub radius: f64,
}

// --- Small vector help -------------------------------------------------

pub fn add(a: P3, b: P3) -> P3 {
    [a[0] + b[0], a[1] + b[1], a[2] + b[2]]
}
pub fn sub(a: P3, b: P3) -> P3 {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}
pub fn mul(a: P3, k: f64) -> P3 {
    [a[0] * k, a[1] * k, a[2] * k]
}
pub fn dot(a: P3, b: P3) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}
pub fn cross(a: P3, b: P3) -> P3 {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}
pub fn len(a: P3) -> f64 {
    dot(a, a).sqrt()
}
pub fn dist(a: P3, b: P3) -> f64 {
    len(sub(a, b))
}
pub fn norm(a: P3) -> P3 {
    let l = len(a);
    if l < 1e-12 {
        [0.0, 0.0, 1.0]
    } else {
        mul(a, 1.0 / l)
    }
}
pub fn lerp3(a: P3, b: P3, u: f64) -> P3 {
    add(a, mul(sub(b, a), u))
}

/// Any unit vector square to `d`.
pub fn perpendicular(d: P3) -> P3 {
    let h = if d[2].abs() < 0.9 {
        [0.0, 0.0, 1.0]
    } else {
        [1.0, 0.0, 0.0]
    };
    norm(cross(d, h))
}

/// `v` turned `a` radians round the unit `axis` (Rodrigues).
pub fn rotate(v: P3, axis: P3, a: f64) -> P3 {
    let (s, c) = a.sin_cos();
    add(
        add(mul(v, c), mul(cross(axis, v), s)),
        mul(axis, dot(axis, v) * (1.0 - c)),
    )
}

/// A direction from its azimuth and its angle from the vertical.
fn dir(az: f64, from_vertical: f64) -> P3 {
    let (s, c) = from_vertical.sin_cos();
    [s * az.cos(), s * az.sin(), c]
}

// --- Growing ----------------------------------------------------------

pub fn grow(s: &Species, seed: u64, variation: f64) -> Plant {
    let mut rng = Rng::new(seed);
    let mut g = Grower {
        s: *s,
        var: variation,
        branches: Vec::new(),
        clumps: Vec::new(),
        fronds: Vec::new(),
        center: [0.0, 0.0],
    };
    match s.habit {
        Habit::Tree | Habit::Shrub | Habit::Cushion => g.branched(&mut rng),
        Habit::Conifer => g.conifer(&mut rng),
        Habit::Palm => g.palm(&mut rng),
        Habit::Grass | Habit::Rosette => g.tuft(&mut rng),
    }
    g.finish()
}

struct Grower {
    s: Species,
    var: f64,
    branches: Vec<Branch>,
    clumps: Vec<Clump>,
    fronds: Vec<Frond>,
    center: [f64; 2],
}

impl Grower {
    fn finish(self) -> Plant {
        let mut height: f64 = 0.0;
        let mut radius: f64 = 0.0;
        let mut see = |p: P3, r: f64| {
            height = height.max(p[2] + r);
            radius = radius.max((p[0] * p[0] + p[1] * p[1]).sqrt() + r);
        };
        for b in &self.branches {
            for (p, r) in b.points.iter().zip(&b.radii) {
                see(*p, *r);
            }
        }
        for c in &self.clumps {
            see(c.center, c.radius * c.stretch.max(1.0));
        }
        for f in &self.fronds {
            for p in &f.rachis {
                see(*p, f.width * 0.5);
            }
            if f.kind == FrondKind::Fan {
                if let Some(p) = f.rachis.last() {
                    see(*p, f.width);
                }
            }
        }
        Plant {
            species: self.s,
            branches: self.branches,
            clumps: self.clumps,
            fronds: self.fronds,
            center: self.center,
            height,
            radius,
        }
    }

    /// A trunk or stem from `base` to `top`, bent by the lean and
    /// twisted by the gnarl.
    fn stem(&self, base: P3, top: P3, r0: f64, r1: f64, rng: &mut Rng, level: u8) -> Branch {
        let n = 10;
        let axis = sub(top, base);
        let length = len(axis);
        let d = norm(axis);
        let u = perpendicular(d);
        let v = cross(d, u);
        let g = self.s.gnarl;
        let amp = g * (r0 * 2.2 + length * 0.05);
        let (f1, f2) = (rng.range(0.6, 1.6), rng.range(1.5, 3.2));
        let (p1, p2, p3) = (
            rng.range(0.0, TAU),
            rng.range(0.0, TAU),
            rng.range(0.0, TAU),
        );
        let mut points = Vec::with_capacity(n + 1);
        let mut radii = Vec::with_capacity(n + 1);
        for i in 0..=n {
            let t = i as f64 / n as f64;
            // The wobble fades at both ends so the stem meets its places.
            let fade = (PI * t).sin();
            let a = amp * fade * (TAU * f1 * t + p1).sin();
            let b = amp * 0.7 * fade * (TAU * f2 * t + p2).sin();
            let p = add(lerp3(base, top, t), add(mul(u, a), mul(v, b)));
            points.push(p);
            // A flare at the foot, a taper to the top, knots if gnarled.
            let flare = if level == 0 {
                1.0 + 0.35 * (1.0 - t * 6.0).max(0.0)
            } else {
                1.0
            };
            let knots = 1.0 + g * 0.12 * (TAU * 3.0 * t + p3).sin();
            radii.push((r0 + (r1 - r0) * t) * flare * knots);
        }
        Branch {
            points,
            radii,
            level,
            green: false,
        }
    }

    /// A branch from `origin` along `d` for `length`, bending as the
    /// species says.
    fn limb(&self, origin: P3, d: P3, length: f64, r0: f64, level: u8, rng: &mut Rng) -> Branch {
        let n = 6;
        let step = length / n as f64;
        // Hanging branches bend down, upright ones up; the deeper the
        // branch, the more.
        let deep = 1.0 + 0.35 * (level as f64 - 1.0);
        let pull = if self.s.droop >= 0.0 {
            [0.0, 0.0, -self.s.droop * 0.55 * deep]
        } else {
            [0.0, 0.0, -self.s.droop * 0.35]
        };
        let wobble = (self.s.gnarl * 0.45 + 0.06) * (0.5 + self.var);
        let mut points = vec![origin];
        let mut radii = vec![r0];
        let mut p = origin;
        let mut dir = d;
        for i in 1..=n {
            let k = i as f64 / n as f64;
            let side = [rng.signed(), rng.signed(), rng.signed() * 0.5];
            dir = norm(add(
                add(dir, mul(pull, 1.0 / n as f64 * 2.0)),
                mul(side, wobble / n as f64 * 2.0),
            ));
            p = add(p, mul(dir, step));
            points.push(p);
            radii.push(r0 * (1.0 - 0.72 * k).max(0.12));
        }
        Branch {
            points,
            radii,
            level,
            green: self.s.foliage == Foliage::Broom && level >= 1,
        }
    }

    // --- Trees and shrubs ---------------------------------------------

    fn branched(&mut self, rng: &mut Rng) {
        let s = self.s;
        let h = s.height;
        let r = s.width / 2.0;
        let shrub = matches!(s.habit, Habit::Shrub | Habit::Cushion);
        // Lean: the species' and a little of chance.
        let lean_az = rng.range(0.0, TAU);
        let lean = (s.lean * rng.range(0.55, 1.0) + self.var * 0.06 * rng.f()).min(1.0);
        let z0 = if shrub { 0.0 } else { s.clear.min(h * 0.85) };
        // Where the trunk splits into limbs.
        let split = match s.crown {
            CrownShape::Vase => 0.04,
            CrownShape::Umbrella => 0.1,
            CrownShape::Spreading => 0.12,
            CrownShape::Columnar => 0.45,
            CrownShape::Conical => 0.5,
            CrownShape::Weeping => 0.2,
            _ => 0.2,
        };
        let zs = if shrub {
            0.0
        } else {
            z0 + (h - z0) * split * rng.range(0.85, 1.15)
        };
        let drift = lean * zs * 0.32;
        let top_xy = [drift * lean_az.cos(), drift * lean_az.sin()];
        self.center = top_xy;
        let mut env_rng = rng.fork(1);
        let env = Envelope::new(
            s.crown,
            top_xy,
            z0,
            h,
            r * (1.0 + self.var * 0.08 * env_rng.signed()),
            self.var,
            &mut env_rng,
        );

        // Trunks, and where the limbs start.
        let mut starts: Vec<(P3, P3, f64)> = Vec::new();
        if shrub {
            let n = s.stems.max(s.limbs).max(3);
            let foot = s.trunk * 0.5 * (n as f64).sqrt();
            for i in 0..n {
                let az = i as f64 * GOLDEN + rng.signed() * 0.4;
                let rr = foot * rng.f().sqrt();
                let base = [rr * az.cos(), rr * az.sin(), 0.0];
                let up = (s.angle * 0.6 + rng.range(5.0, 25.0) * (0.6 + self.var)).to_radians();
                starts.push((
                    base,
                    dir(az, up.min(1.4)),
                    s.trunk * 0.5 / (n as f64).sqrt(),
                ));
            }
        } else {
            let n = s.stems.max(1);
            let r_stem =
                s.trunk * 0.5 / (n as f64).sqrt().max(1.0) * if n > 1 { 1.15 } else { 1.0 };
            let per = (s.limbs as f64 / n as f64).ceil().max(1.0) as u32;
            for k in 0..n {
                let (base, top) = if n == 1 {
                    ([0.0, 0.0, 0.0], [top_xy[0], top_xy[1], zs])
                } else {
                    let az = k as f64 * TAU / n as f64 + rng.signed() * 0.5;
                    let b = s.trunk * 0.35 * (n as f64).sqrt();
                    let splay = r * rng.range(0.15, 0.4);
                    (
                        [b * az.cos(), b * az.sin(), 0.0],
                        [
                            top_xy[0] + splay * az.cos(),
                            top_xy[1] + splay * az.sin(),
                            zs * rng.range(0.8, 1.1),
                        ],
                    )
                };
                let trunk = self.stem(base, top, r_stem, r_stem * 0.62, rng, 0);
                // Limbs from the upper part of this trunk.
                for i in 0..per {
                    let at = if per == 1 {
                        1.0
                    } else {
                        0.72 + 0.28 * i as f64 / (per - 1) as f64
                    };
                    let (p, _, rr) = trunk.at(at);
                    let az = (k * per + i) as f64 * GOLDEN + rng.signed() * 0.5 * (0.4 + self.var);
                    let up = (s.angle + rng.signed() * 14.0 * (0.4 + self.var)).clamp(0.0, 105.0);
                    starts.push((p, dir(az, up.to_radians()), rr * rng.range(0.55, 0.75)));
                }
                // The trunk goes on a little above the split as a leader.
                self.branches.push(trunk);
            }
        }

        // Limbs and their branches.
        let mut tips: Vec<usize> = Vec::new();
        for (origin, d, r0) in starts {
            let reach = env.ray(origin, d);
            let length = (reach * rng.range(0.72, 0.95)).max(r * 0.2);
            let b = self.limb(origin, d, length, r0.max(0.004), 1, rng);
            self.branches.push(b);
            let idx = self.branches.len() - 1;
            self.children(idx, 2, &env, rng, &mut tips);
        }

        self.leaves(&env, &tips, rng);
    }

    /// The branches of branch `parent`, and theirs, down to the species'
    /// last order. Branches with no children are tips: leaves grow there.
    fn children(
        &mut self,
        parent: usize,
        level: u8,
        env: &Envelope,
        rng: &mut Rng,
        tips: &mut Vec<usize>,
    ) {
        let s = self.s;
        if level as u32 > s.levels {
            tips.push(parent);
            return;
        }
        let plen = self.branches[parent].length();
        let base = match level {
            2 => 5.0,
            3 => 4.0,
            _ => 3.0,
        };
        let broom = s.foliage == Foliage::Broom;
        let kind = if broom {
            1.2
        } else if s.habit == Habit::Conifer {
            0.45
        } else {
            1.0
        };
        let count = (base * (0.55 + 0.8 * s.density) * kind).round() as u32;
        let mut made = 0;
        for i in 0..count {
            let at = 0.25 + 0.72 * (i as f64 + rng.f() * 0.8) / count as f64;
            let (p, pd, pr) = self.branches[parent].at(at);
            let roll = rng.range(0.0, TAU);
            let axis = rotate(perpendicular(pd), pd, roll);
            let spread = if broom {
                rng.range(10.0, 28.0)
            } else {
                rng.range(28.0, 58.0)
            };
            let mut d = rotate(pd, axis, spread.to_radians());
            if broom {
                d = norm(add(d, [0.0, 0.0, 0.5]));
            }
            let reach = env.ray(p, d);
            let length = (reach * rng.range(0.5, 0.92)).min(plen * rng.range(0.5, 0.75));
            if length < s.clump * 0.4 {
                continue;
            }
            let b = self.limb(p, d, length, (pr * 0.62).max(0.003), level, rng);
            self.branches.push(b);
            let idx = self.branches.len() - 1;
            made += 1;
            self.children(idx, level + 1, env, rng, tips);
        }
        if made == 0 {
            tips.push(parent);
        }
    }

    /// Leaves: clumps along the tips, then over the crown's skin wherever
    /// it would be bare, as the species' density asks.
    fn leaves(&mut self, env: &Envelope, tips: &[usize], rng: &mut Rng) {
        let s = self.s;
        let (size, along, stretch, flat) = match s.foliage {
            Foliage::Broad => (1.0, 1.0, 1.0, 0.72),
            Foliage::Fine => (0.72, 1.4, 1.0, 0.8),
            Foliage::Needles => (0.8, 1.2, 1.55, 0.9),
            Foliage::Scales => (0.7, 1.6, 1.0, 1.0),
            Foliage::Broom => (0.32, 0.5, 1.6, 1.0),
            Foliage::Succulent => (0.55, 1.4, 1.7, 1.0),
            _ => (1.0, 1.0, 1.0, 0.75),
        };
        // Never so small that a plant needs thousands of clumps.
        let cr = (s.clump * size).max(s.width.max(s.height) / 40.0);
        let mut grid = Grid::new(cr * 1.1);
        // Along the tips.
        for &t in tips {
            let b = self.branches[t].clone();
            let l = b.length();
            let n = ((l * 0.62 / cr) * (0.45 + s.density) * along)
                .ceil()
                .max(1.0) as usize;
            for i in 0..n {
                let at = 0.4 + 0.6 * (i as f64 + rng.f()) / n as f64;
                let (p, d, _) = b.at(at);
                let off = mul(
                    norm([rng.signed(), rng.signed(), rng.signed() * 0.6 + 0.25]),
                    cr * 0.45,
                );
                let c = add(p, off);
                let axis = if s.foliage == Foliage::Succulent {
                    norm(add(d, [0.0, 0.0, 0.8]))
                } else {
                    d
                };
                self.clump(
                    c,
                    cr * rng.range(0.75, 1.2),
                    axis,
                    stretch,
                    flat,
                    env,
                    rng,
                    &mut grid,
                );
            }
        }
        // Over the skin, where it is bare.
        let cover = match s.foliage {
            Foliage::Scales => 1.25,
            Foliage::Broom => 0.15,
            Foliage::Fine => 0.75,
            _ => 1.0,
        };
        let want = (env.area() / (PI * cr * cr * 0.8) * s.density * s.density * cover) as usize;
        let want = want.min(2400);
        let mut tries = 0;
        let mut placed = 0;
        while placed < want && tries < want * 4 + 50 {
            tries += 1;
            // Even over the skin: heights by the radius there.
            let t = rng.f();
            let az = rng.range(0.0, TAU);
            let q = env.skin(t, az);
            let rr = env.radius(q[2], az);
            if rng.f() * env.r > rr {
                continue;
            }
            let inward = cr * rng.range(0.25, 0.75);
            let toward = norm([env.center[0] - q[0], env.center[1] - q[1], 0.0]);
            let c = add(q, mul(toward, inward));
            if grid.near(c, cr * 1.05) {
                continue;
            }
            let axis = if matches!(s.foliage, Foliage::Needles | Foliage::Succulent) {
                norm([q[0] - env.center[0], q[1] - env.center[1], 0.6])
            } else {
                [0.0, 0.0, 1.0]
            };
            if self.clump(
                c,
                cr * rng.range(0.8, 1.2),
                axis,
                stretch,
                flat,
                env,
                rng,
                &mut grid,
            ) {
                placed += 1;
            }
        }
        // Some inside, so the crown is not hollow seen from below.
        let inner = (placed as f64 * 0.12 * s.density) as usize;
        for _ in 0..inner {
            let t = rng.range(0.15, 0.7);
            let az = rng.range(0.0, TAU);
            let q = env.skin(t, az);
            let k = rng.range(0.35, 0.7);
            let c = [
                env.center[0] + (q[0] - env.center[0]) * k,
                env.center[1] + (q[1] - env.center[1]) * k,
                q[2],
            ];
            self.clump(
                c,
                cr * rng.range(0.9, 1.3),
                [0.0, 0.0, 1.0],
                stretch,
                flat,
                env,
                rng,
                &mut grid,
            );
        }
        self.flowers_and_fruit(env, rng);
    }

    /// One clump of leaves, unless there is one there already.
    #[allow(clippy::too_many_arguments)]
    fn clump(
        &mut self,
        c: P3,
        r: f64,
        axis: P3,
        stretch: f64,
        flat: f64,
        env: &Envelope,
        rng: &mut Rng,
        grid: &mut Grid,
    ) -> bool {
        if c[2] < r * 0.3 && !matches!(self.s.habit, Habit::Cushion) {
            return false;
        }
        if self.clumps.len() >= 1200 {
            return false;
        }
        let depth = env.depth(c);
        let t = env.t(c[2]).clamp(0.0, 1.0);
        let light = (0.3 + 0.45 * (1.0 - depth) + 0.25 * t).clamp(0.15, 1.0);
        let tint_spread = match self.s.foliage {
            Foliage::Fine => 0.55,
            Foliage::Succulent => 0.6,
            Foliage::Scales => 0.2,
            _ => 0.35,
        };
        grid.put(c);
        self.clumps.push(Clump {
            center: [c[0], c[1], c[2].max(r * 0.5)],
            radius: r,
            axis: norm(axis),
            stretch,
            flat,
            kind: ClumpKind::Leaves,
            light,
            tint: rng.signed() * tint_spread,
            seed: rng.next(),
        });
        true
    }

    /// Flowers over the outer clumps and fruit on the skin.
    fn flowers_and_fruit(&mut self, env: &Envelope, rng: &mut Rng) {
        let s = self.s;
        if s.bloom > 0.0 {
            for c in self.clumps.iter_mut() {
                let outer = 1.0 - env.depth(c.center);
                if rng.f() < s.bloom * (0.35 + 0.9 * outer) {
                    c.kind = ClumpKind::Flowers;
                }
            }
        }
        if s.fruit > 0.0 {
            let fr = (s.clump * 0.16).clamp(0.03, 0.12);
            let n = (env.area() / (s.clump * s.clump * 4.0) * s.fruit).min(400.0) as usize;
            for _ in 0..n {
                let t = rng.range(0.1, 0.9);
                let az = rng.range(0.0, TAU);
                let q = env.skin(t, az);
                let toward = norm([env.center[0] - q[0], env.center[1] - q[1], 0.0]);
                let c = add(q, mul(toward, fr * 0.5));
                self.clumps.push(Clump {
                    center: c,
                    radius: fr,
                    axis: [0.0, 0.0, 1.0],
                    stretch: 1.0,
                    flat: 1.0,
                    kind: ClumpKind::Fruit,
                    light: 0.9,
                    tint: rng.signed() * 0.2,
                    seed: rng.next(),
                });
            }
        }
    }

    // --- Conifers -----------------------------------------------------

    fn conifer(&mut self, rng: &mut Rng) {
        let s = self.s;
        let h = s.height;
        let r = s.width / 2.0;
        let z0 = s.clear.min(h * 0.8);
        let lean_az = rng.range(0.0, TAU);
        let lean = s.lean * rng.range(0.4, 1.0) + self.var * 0.03 * rng.f();
        let drift = lean * h * 0.12;
        let top = [drift * lean_az.cos(), drift * lean_az.sin(), h * 0.985];
        self.center = [top[0] * 0.6, top[1] * 0.6];
        let mut env_rng = rng.fork(1);
        let env = Envelope::new(s.crown, self.center, z0, h, r, self.var, &mut env_rng);
        let n = s.stems.max(1);
        let mut leaders = Vec::new();
        for k in 0..n {
            let (base, top) = if n == 1 {
                ([0.0, 0.0, 0.0], top)
            } else {
                let az = k as f64 * TAU / n as f64;
                let b = s.trunk * 0.4;
                (
                    [b * az.cos(), b * az.sin(), 0.0],
                    [
                        top[0] + r * 0.3 * az.cos(),
                        top[1] + r * 0.3 * az.sin(),
                        top[2] * rng.range(0.85, 1.0),
                    ],
                )
            };
            let r0 = s.trunk * 0.5 / (n as f64).sqrt();
            let mut t = self.stem(base, top, r0, r0 * 0.06, rng, 0);
            // A leader is straight: much less wobble than a trunk.
            for (i, p) in t.points.iter_mut().enumerate() {
                let u = i as f64 / 10.0;
                let straight = lerp3(base, top, u);
                *p = lerp3(straight, *p, 0.35);
            }
            leaders.push(self.branches.len());
            self.branches.push(t);
        }
        // Whorls of branches up the leader.
        let mut tips = Vec::new();
        let span = h - z0;
        let whorls = ((span / (h * 0.065).max(0.18)).round() as usize).clamp(4, 40);
        let per = s.limbs.max(2);
        for &l in &leaders {
            for w in 0..whorls {
                let u = (w as f64 + 0.3 + 0.4 * rng.f()) / whorls as f64;
                let at = (z0 + span * u) / h;
                let (p, _, rr) = self.branches[l].at(at.min(0.97));
                let turn = w as f64 * GOLDEN;
                let up_angle = s.angle * (1.0 - 0.35 * u);
                let k = per + (rng.f() * 2.0) as u32;
                for i in 0..k {
                    let az = turn + i as f64 * TAU / k as f64 + rng.signed() * 0.3;
                    let a = (up_angle + rng.signed() * 8.0)
                        .to_radians()
                        .clamp(0.05, 1.9);
                    let d = dir(az, a);
                    let reach = env.ray(p, d);
                    let length = (reach * rng.range(0.85, 1.0)).max(s.clump * 0.4);
                    let b = self.limb(p, d, length, (rr * 0.4).max(0.004), 1, rng);
                    self.branches.push(b);
                    let idx = self.branches.len() - 1;
                    if s.levels >= 2 && length > s.clump * 2.0 {
                        self.children(idx, 2, &env, rng, &mut tips);
                    } else {
                        tips.push(idx);
                    }
                }
            }
        }
        self.leaves(&env, &tips, rng);
    }

    // --- Palms --------------------------------------------------------

    fn palm(&mut self, rng: &mut Rng) {
        let s = self.s;
        let h = s.height;
        let fl = s.frond_length;
        let n = s.stems.max(1);
        let lean_az = rng.range(0.0, TAU);
        for k in 0..n {
            let (az, splay) = if n == 1 {
                (lean_az, 0.0)
            } else {
                (
                    k as f64 * TAU / n as f64 + rng.signed() * 0.4,
                    rng.range(0.18, 0.4),
                )
            };
            let tall = if n == 1 { 1.0 } else { rng.range(0.55, 1.0) };
            let zt = ((h - fl * 0.3) * tall).max(fl * 0.2);
            let lean = s.lean * rng.range(0.5, 1.0) + splay + self.var * 0.05 * rng.f();
            let base = if n == 1 {
                [0.0, 0.0, 0.0]
            } else {
                let b = s.trunk * 0.5 * (n as f64).sqrt();
                [b * az.cos(), b * az.sin(), 0.0]
            };
            // A curved trunk: the lean grows towards the top.
            let r0 = s.trunk * 0.5;
            let mut points = Vec::new();
            let mut radii = Vec::new();
            let m = 12;
            for i in 0..=m {
                let t = i as f64 / m as f64;
                let off = lean * zt * 0.3 * t * t;
                points.push([base[0] + off * az.cos(), base[1] + off * az.sin(), zt * t]);
                // Palms swell at the foot and keep their girth.
                radii.push(r0 * (1.0 + 0.3 * (1.0 - t * 5.0).max(0.0)) * (1.0 - 0.12 * t));
            }
            let head = points[m];
            if k == 0 {
                self.center = [head[0], head[1]];
            }
            self.branches.push(Branch {
                points,
                radii,
                level: 0,
                green: false,
            });
            self.crown_of_fronds(head, r0, rng);
        }
    }

    fn crown_of_fronds(&mut self, head: P3, r0: f64, rng: &mut Rng) {
        let s = self.s;
        let fan = s.foliage == Foliage::Fan;
        let f = s.fronds.max(3);
        let turn = rng.range(0.0, TAU);
        for i in 0..f {
            // The youngest stand up in the middle, the oldest hang.
            let age = (i as f64 + rng.f() * 0.6) / f as f64;
            let az = turn + i as f64 * GOLDEN + rng.signed() * 0.25 * (0.5 + self.var);
            let rise = (70.0 - 105.0 * age + rng.signed() * 8.0).to_radians();
            let length = s.frond_length * rng.range(0.82, 1.08) * (1.0 - 0.25 * (age - 0.5).abs());
            let segs = 12;
            let (stalk, w) = if fan {
                (0.55, s.frond_length * 0.42)
            } else {
                (1.0, s.frond_length * 0.14)
            };
            let along = length * stalk;
            let mut d = [rise.cos() * az.cos(), rise.cos() * az.sin(), rise.sin()];
            let start = add(head, [r0 * 0.4 * az.cos(), r0 * 0.4 * az.sin(), -r0 * 0.3]);
            let mut p = start;
            let mut rachis = vec![p];
            let gravity =
                (0.35 + 0.45 * s.droop.max(-0.5) + 0.35 * age) * if fan { 0.5 } else { 1.0 };
            for j in 1..=segs {
                let k = j as f64 / segs as f64;
                d = norm(add(d, [0.0, 0.0, -gravity * 2.2 / segs as f64 * (0.4 + k)]));
                p = add(p, mul(d, along / segs as f64));
                rachis.push(p);
            }
            let side = norm([-az.sin(), az.cos(), 0.0]);
            self.fronds.push(Frond {
                rachis,
                width: w * rng.range(0.9, 1.1),
                kind: if fan {
                    FrondKind::Fan
                } else {
                    FrondKind::Pinnate
                },
                side,
                light: (0.45 + 0.55 * (1.0 - age)).clamp(0.2, 1.0),
                tint: rng.signed() * 0.3,
            });
        }
        // The head where the fronds start.
        let leaves_r = r0 * 1.35;
        self.clumps.push(Clump {
            center: add(head, [0.0, 0.0, leaves_r * 0.2]),
            radius: leaves_r,
            axis: [0.0, 0.0, 1.0],
            stretch: 1.3,
            flat: 1.0,
            kind: ClumpKind::Leaves,
            light: 0.4,
            tint: -0.4,
            seed: rng.next(),
        });
        if s.fruit > 0.0 {
            // Bunches of dates hanging under the fronds.
            let bunches = (s.fruit * 6.0).ceil() as usize;
            for b in 0..bunches {
                let az = b as f64 * TAU / bunches as f64 + rng.signed() * 0.3;
                let c = add(head, [r0 * 1.6 * az.cos(), r0 * 1.6 * az.sin(), -r0 * 2.0]);
                self.clumps.push(Clump {
                    center: c,
                    radius: (r0 * 0.8).max(0.08),
                    axis: [0.0, 0.0, 1.0],
                    stretch: 1.6,
                    flat: 1.0,
                    kind: ClumpKind::Fruit,
                    light: 0.7,
                    tint: rng.signed() * 0.2,
                    seed: rng.next(),
                });
            }
        }
    }

    // --- Grasses and rosettes -------------------------------------------

    fn tuft(&mut self, rng: &mut Rng) {
        let s = self.s;
        let rosette = s.habit == Habit::Rosette;
        let n = s.stems.max(1);
        for k in 0..n {
            // A rosette may stand on a short trunk (yucca, dragon tree).
            let (base_xy, z) = if n == 1 {
                ([0.0, 0.0], 0.0)
            } else {
                let az = k as f64 * GOLDEN + rng.signed() * 0.3;
                let rr = s.width * 0.3 * rng.f().sqrt();
                ([rr * az.cos(), rr * az.sin()], 0.0)
            };
            let z_head = if rosette && s.clear > 0.05 {
                let top = [base_xy[0], base_xy[1], s.clear * rng.range(0.7, 1.0)];
                let t = self.stem(
                    [base_xy[0], base_xy[1], z],
                    top,
                    s.trunk * 0.5,
                    s.trunk * 0.4,
                    rng,
                    0,
                );
                self.branches.push(t);
                top[2]
            } else {
                z
            };
            if k == 0 {
                self.center = base_xy;
            }
            let head = [base_xy[0], base_xy[1], z_head];
            let f = s.fronds.max(3);
            for i in 0..f {
                let u = (i as f64 + rng.f()) / f as f64;
                let az = i as f64 * GOLDEN + rng.signed() * 0.4;
                // The inner ones stand up, the outer ones spread.
                let from_vertical = if rosette {
                    (12.0 + 70.0 * u + rng.signed() * 6.0).to_radians()
                } else {
                    (4.0 + s.angle * u.sqrt() + rng.signed() * 6.0).to_radians()
                };
                let length = s.frond_length
                    * rng.range(0.65, 1.05)
                    * if rosette { 1.0 - 0.3 * (1.0 - u) } else { 1.0 };
                let segs = if rosette { 5 } else { 8 };
                let gravity = if rosette {
                    0.05 + 0.25 * s.droop.max(0.0)
                } else {
                    0.25 + 0.6 * s.droop.max(0.0) * u
                };
                let mut d = dir(az, from_vertical);
                let mut p = head;
                let mut rachis = vec![p];
                for j in 1..=segs {
                    let kk = j as f64 / segs as f64;
                    d = norm(add(
                        d,
                        [0.0, 0.0, -gravity * 2.0 / segs as f64 * (0.3 + kk)],
                    ));
                    p = add(p, mul(d, length / segs as f64));
                    rachis.push(p);
                }
                let side = norm([-az.sin(), az.cos(), 0.0]);
                self.fronds.push(Frond {
                    rachis,
                    width: if rosette {
                        s.frond_length * 0.07
                    } else {
                        (s.frond_length * 0.012).max(0.004)
                    },
                    kind: if rosette {
                        FrondKind::Sword
                    } else {
                        FrondKind::Blade
                    },
                    side,
                    light: (0.4 + 0.6 * (1.0 - u)).clamp(0.2, 1.0),
                    tint: rng.signed() * 0.35,
                });
                // Plumes on the tallest blades.
                if !rosette && s.bloom > 0.0 && rng.f() < s.bloom * 0.6 && u < 0.6 {
                    self.clumps.push(Clump {
                        center: p,
                        radius: (s.frond_length * 0.025).max(0.012),
                        axis: d,
                        stretch: 3.0,
                        flat: 1.0,
                        kind: ClumpKind::Flowers,
                        light: 0.9,
                        tint: rng.signed() * 0.2,
                        seed: rng.next(),
                    });
                }
            }
            // A flower spike (agave) when in bloom.
            if rosette && s.bloom > 0.0 && rng.f() < s.bloom {
                let top = [head[0], head[1], head[2] + s.height * 0.9];
                let spike = self.stem(head, top, s.frond_length * 0.03, 0.01, rng, 1);
                for j in 0..6 {
                    let (p, _, _) = spike.at(0.55 + 0.08 * j as f64);
                    self.clumps.push(Clump {
                        center: p,
                        radius: s.frond_length * 0.07,
                        axis: [0.0, 0.0, 1.0],
                        stretch: 1.0,
                        flat: 0.8,
                        kind: ClumpKind::Flowers,
                        light: 0.9,
                        tint: rng.signed() * 0.2,
                        seed: rng.next(),
                    });
                }
                self.branches.push(spike);
            }
        }
    }
}

/// A grid of the clumps placed, to keep them apart cheaply.
struct Grid {
    cell: f64,
    cells: std::collections::HashMap<(i32, i32, i32), Vec<P3>>,
}

impl Grid {
    fn new(cell: f64) -> Self {
        Grid {
            cell: cell.max(1e-3),
            cells: Default::default(),
        }
    }

    fn key(&self, p: P3) -> (i32, i32, i32) {
        (
            (p[0] / self.cell).floor() as i32,
            (p[1] / self.cell).floor() as i32,
            (p[2] / self.cell).floor() as i32,
        )
    }

    fn put(&mut self, p: P3) {
        let k = self.key(p);
        self.cells.entry(k).or_default().push(p);
    }

    fn near(&self, p: P3, d: f64) -> bool {
        let (i, j, k) = self.key(p);
        let reach = (d / self.cell).ceil() as i32;
        for a in -reach..=reach {
            for b in -reach..=reach {
                for c in -reach..=reach {
                    if let Some(v) = self.cells.get(&(i + a, j + b, k + c)) {
                        if v.iter().any(|q| dist(*q, p) < d) {
                            return true;
                        }
                    }
                }
            }
        }
        false
    }
}
